//! Le réseau d'officines : ce qu'une officine partage avec les autres
//! officines de son groupement — **les ruptures et ce qu'on a donné à la
//! place**, les valeurs de pharmacocinétique sourcées, et les versions du
//! contenu partageable (fiches, préparations, protocoles, lignes de TROD,
//! vaccins du catalogue, `versions.rs`) — **jamais un patient**.
//!
//! Construit sur `bpm-sync` (voir `docs/SYNC.md`), avec deux choix qui
//! en font un outil entre officines et non entre postes :
//!
//! * **Une clé à part.** Le réseau a son propre trousseau, qui ne scelle
//!   qu'un flux — [`Stream::Reseau`] — et n'en ouvre aucun autre. Les
//!   dossiers, le registre, la caisse ne passent jamais par ici : ils
//!   n'ont pas de projection vers ce flux, et une officine du réseau qui
//!   tiendrait la clé n'ouvrirait rien d'autre.
//! * **Rien n'écoute en permanence.** On appaire en se parlant au
//!   téléphone : une officine ouvre une porte le temps d'une invitation,
//!   l'autre compose son adresse, et toutes deux comparent un code de
//!   cinq groupes. Ensuite on synchronise **sur un bouton ou à la
//!   fermeture** : en composant l'adresse d'une officine qui a une porte
//!   ouverte, ou — ce qui traverse les box et les pare-feux sans rien
//!   ouvrir — par un **dossier d'échange** (un partage réseau, un dossier
//!   synchronisé) où chacune dépose ses enregistrements scellés et lit
//!   ceux des autres. Ce qui s'y trouve est chiffré et signé ; qui tient
//!   le dossier ne lit rien et ne peut rien fabriquer.
//!
//! Le journal des ruptures reste la seule vérité locale : ce module ne
//! fait que le **projeter** vers le réseau (un événement local, un
//! enregistrement scellé, une seule fois) et **ranger** ce qui en vient
//! (un enregistrement d'une officine appairée, un événement dont la
//! source est son nom). Une version reçue ne s'applique qu'à un contenu
//! qui dit encore ce qu'elle remplaçait ; sinon elle attend l'arbitrage.
//! Les valeurs de pharmacocinétique se rangent à part, sous le nom de
//! l'officine qui les a sourcées.

use crate::db::Db;
use bpm_sync::{Device, DeviceId, Intent, Journal, Meter, Record, Session, Stream, Trousseau};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Le port proposé pour une invitation, quand l'officine n'en a pas écrit.
pub const DEFAULT_PORT: u16 = 7742;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex<const N: usize>(text: &str) -> Option<[u8; N]> {
    let text = text.trim();
    if text.len() != N * 2 {
        return None;
    }
    let mut out = [0u8; N];
    for (i, slot) in out.iter_mut().enumerate() {
        *slot = u8::from_str_radix(text.get(i * 2..i * 2 + 2)?, 16).ok()?;
    }
    Some(out)
}

fn random32() -> [u8; 32] {
    let mut out = [0u8; 32];
    bpm_sync::Entropy::fill(&mut bpm_sync::OsEntropy, &mut out);
    out
}

/// Une officine appairée, et ce que ce poste sait de ses échanges avec
/// elle — voir [`crate::db::NetPeerRow`].
pub type Peer = crate::db::NetPeerRow;

/// Le nom d'un réseau par son `id` (l'empreinte de sa clé, en hex), en
/// cinq groupes.
pub fn network_groups_of(id: &str) -> String {
    unhex::<10>(id)
        .map(|b| bpm_sync::Fingerprint::from_bytes(b).groups())
        .unwrap_or_else(|| id.chars().take(10).collect())
}

/// L'empreinte d'une officine appairée, en cinq groupes — ce qu'on se
/// lit au téléphone.
pub fn peer_groups(device: &str) -> String {
    unhex::<32>(device)
        .map(|b| DeviceId::from_bytes(b).fingerprint().groups())
        .unwrap_or_default()
}

/// Ce que cette officine sait du réseau, lu dans la base.
pub struct Net {
    /// Vide pour le réseau principal ; l'empreinte de sa clé pour les
    /// autres (`net_networks`).
    pub id: String,
    pub label: String,
    device: Device,
    trousseau: Option<Trousseau>,
    journal: Journal,
    /// Les officines de **ce** réseau.
    pub peers: Vec<Peer>,
    /// Ce que l'officine partage dans ce réseau.
    pub shares: crate::db::NetShares,
    /// Le dossier d'échange propre à ce réseau (les autres que le
    /// principal) ; celui du principal vient de la configuration.
    pub folder: String,
    /// Ce que le dossier d'échange a montré d'officines qu'on n'a pas
    /// ajoutées — **compté, jamais gardé** : voir [`Net::introduced`].
    strangers: Vec<Introduced>,
}

impl Net {
    /// Lire l'identité de l'officine, sa clé de réseau si elle en a une,
    /// ses officines appairées et le journal. L'identité est tirée au
    /// hasard la première fois, **dans la base chiffrée** — jamais dans
    /// `config.toml`, qui est en clair.
    pub fn load(db: &Db) -> Result<Self, String> {
        let seed = db.net_key("net_seed", &hex(&random32()))?;
        let seed = unhex::<32>(&seed).ok_or("identité du réseau illisible")?;
        let trousseau = db
            .setting("net_trousseau")
            .and_then(|t| unhex::<32>(&t))
            .map(Trousseau::from_secret);
        let peers = db.net_members("")?;
        let device = Device::from_seed(seed);
        let mut journal = stored_journal(db.net_records_of("")?);
        journal.trust(trusted_ids(&device, &peers));
        Ok(Self {
            id: String::new(),
            label: String::new(),
            device,
            trousseau,
            journal,
            peers,
            shares: db.principal_shares(),
            folder: String::new(),
            strangers: Vec::new(),
        })
    }

    /// Un réseau de plus, par son `id`.
    pub fn load_network(db: &Db, id: &str) -> Result<Self, String> {
        if id.is_empty() {
            return Self::load(db);
        }
        let n = db
            .net_networks()?
            .into_iter()
            .find(|n| n.id == id)
            .ok_or_else(|| crate::strings::tr("net_err_no_network").to_owned())?;
        let seed = db.net_key("net_seed", &hex(&random32()))?;
        let seed = unhex::<32>(&seed).ok_or("identité du réseau illisible")?;
        let device = Device::from_seed(seed);
        let peers = db.net_members(id)?;
        let mut journal = stored_journal(db.net_records_of(id)?);
        journal.trust(trusted_ids(&device, &peers));
        Ok(Self {
            id: n.id.clone(),
            label: n.label.clone(),
            device,
            trousseau: unhex::<32>(&n.secret).map(Trousseau::from_secret),
            journal,
            peers,
            shares: n.shares,
            folder: n.folder.clone(),
            strangers: Vec::new(),
        })
    }

    /// Tous les réseaux dont l'officine est : le principal s'il existe,
    /// puis les autres.
    pub fn load_all(db: &Db) -> Result<Vec<Self>, String> {
        let mut out = Vec::new();
        let principal = Self::load(db)?;
        if principal.in_network() {
            out.push(principal);
        }
        for n in db.net_networks()? {
            out.push(Self::load_network(db, &n.id)?);
        }
        Ok(out)
    }

    /// Créer un réseau de plus — pour un lien avec une seule officine, ou
    /// un second groupement. Rend son `id`.
    pub fn create_network(db: &Db, label: &str, day: &str) -> Result<String, String> {
        let secret = random32();
        let id = hex(&Trousseau::from_secret(secret).name().bytes());
        db.add_net_network(&id, &hex(&secret), label, day)?;
        Ok(id)
    }

    /// La clé sous laquelle ce réseau note ce qui est parti : telle quelle
    /// pour le principal (ce qu'elle a toujours été), préfixée pour les
    /// autres — un même événement part une fois **par réseau**.
    fn done_key(&self, key: &str) -> String {
        if self.id.is_empty() {
            key.to_owned()
        } else {
            format!("{}|{key}", self.id)
        }
    }

    /// Ce que ce réseau a déjà publié, sans son préfixe.
    fn already(&self, db: &Db) -> Result<std::collections::HashSet<String>, String> {
        let all = db.published()?;
        // Une clé préfixée commence par l'empreinte d'un réseau (vingt
        // chiffres hexadécimaux) et `|` ; une valeur sourcée peut contenir
        // `|` dans son texte sans appartenir à un autre réseau.
        let prefixed = |k: &str| {
            k.len() > 21
                && k.as_bytes()[20] == b'|'
                && k[..20].bytes().all(|b| b.is_ascii_hexdigit())
        };
        Ok(if self.id.is_empty() {
            all.into_iter().filter(|k| !prefixed(k)).collect()
        } else {
            let prefix = format!("{}|", self.id);
            all.into_iter()
                .filter_map(|k| k.strip_prefix(&prefix).map(str::to_owned))
                .collect()
        })
    }

    pub fn in_network(&self) -> bool {
        self.trousseau.is_some()
    }

    /// L'empreinte de cette officine, en cinq groupes.
    pub fn groups(&self) -> String {
        self.device.id().fingerprint().groups()
    }

    /// Le nom du réseau — l'empreinte de sa clé, la même chez toutes les
    /// officines qui en font partie.
    pub fn network_groups(&self) -> Option<String> {
        self.trousseau.as_ref().map(|t| t.name().groups())
    }

    pub fn record_count(&self) -> usize {
        self.journal.len()
    }

    fn known(&self) -> Vec<DeviceId> {
        self.peers
            .iter()
            .filter_map(|p| unhex::<32>(&p.device).map(DeviceId::from_bytes))
            .collect()
    }

    /// **Les officines du réseau qu'on n'a pas ajoutées**, telles que le
    /// dernier passage au dossier d'échange les a vues : elles déposent des
    /// enregistrements que la clé du réseau ouvre — elles la détiennent.
    ///
    /// **Comptées, jamais gardées.** Leurs enregistrements n'entrent pas
    /// au journal tant qu'on ne les a pas ajoutées : gardés, ils voyageaient
    /// ensuite d'officine en officine, et un enregistrement d'un inconnu
    /// pouvait en corriger — donc effacer — un d'une officine appairée, ou
    /// porter un rang de Lamport au plafond et brouiller pour de bon l'ordre
    /// du réseau (revue de sécurité de 0.318.0). Au plus
    /// [`MOST_STRANGERS`] identités, pour qu'un dossier inondé
    /// d'identités jetables ne fige rien.
    pub fn introduced(&self, ignored: &[String]) -> Vec<Introduced> {
        self.strangers
            .iter()
            .filter(|o| !ignored.contains(&ignore_key(&self.id, &o.device)))
            .cloned()
            .collect()
    }

    /// Créer un réseau : une clé neuve, que cette officine donnera à
    /// celles qu'elle invite.
    pub fn create(db: &Db) -> Result<(), String> {
        if db.setting("net_trousseau").is_some() {
            return Err(crate::strings::tr("net_err_already").to_owned());
        }
        db.net_key("net_trousseau", &hex(&random32()))?;
        Ok(())
    }

    /// Sceller vers le réseau les événements locaux qui ne sont pas encore
    /// partis. Rend combien.
    pub fn publish(&mut self, db: &Db, officine: &str) -> Result<usize, String> {
        let Some(trousseau) = &self.trousseau else {
            return Ok(0);
        };
        // Ce que **ce** réseau a déjà reçu d'ici, et ce que l'officine y
        // partage : un événement part une fois par réseau, et pas du tout
        // dans un réseau où son genre n'est pas partagé.
        let already = self.already(db)?;
        let pending: Vec<crate::ruptures::Event> = if self.shares.ruptures {
            db.supply_events()?
                .into_iter()
                .filter(|e| e.source.is_empty() && !already.contains(&e.uid))
                .collect()
        } else {
            Vec::new()
        };
        let mut done = Vec::new();
        for e in &pending {
            let payload = crate::ruptures::encode(e, officine);
            self.journal
                .write(
                    &self.device,
                    trousseau,
                    Stream::Reseau,
                    &payload,
                    None,
                    &mut bpm_sync::OsEntropy,
                )
                .map_err(|e| format!("{e:?}"))?;
            done.push(e.uid.clone());
        }
        // Les versions des fiches, des préparations et des protocoles :
        // ce que le codex et les protocoles ont changé ici est d'abord
        // versionné, puis chaque version locale part une fois, et c'est
        // chez les autres qu'elle s'applique — ou attend.
        db.version_shared_entries("")?;
        let versions: Vec<crate::versions::Edit> = if self.shares.versions {
            db.card_edits()?
                .into_iter()
                .filter(|e| e.source.is_empty() && !already.contains(&e.uid))
                .collect()
        } else {
            Vec::new()
        };
        for e in versions {
            let payload = crate::versions::encode(&e, officine);
            self.journal
                .write(
                    &self.device,
                    trousseau,
                    Stream::Reseau,
                    &payload,
                    None,
                    &mut bpm_sync::OsEntropy,
                )
                .map_err(|e| format!("{e:?}"))?;
            done.push(e.uid);
        }
        // Les valeurs de pharmacocinétique sourcées : chacune part une
        // fois par valeur — la clé porte le texte et la source, donc une
        // valeur corrigée repart, et c'est la dernière reçue qui compte.
        let facts = if self.shares.pk {
            db.all_drug_facts()?
        } else {
            Vec::new()
        };
        for (name, dci, fact) in facts {
            let key = format!(
                "fait:{}:{}:{}:{}",
                crate::ruptures::key(&name),
                fact.property.key(),
                fact.text,
                fact.source
            );
            if already.contains(&key) {
                continue;
            }
            let payload = crate::pk::encode_shared(&name, &dci, &fact, officine);
            self.journal
                .write(
                    &self.device,
                    trousseau,
                    Stream::Reseau,
                    &payload,
                    None,
                    &mut bpm_sync::OsEntropy,
                )
                .map_err(|e| format!("{e:?}"))?;
            done.push(key);
        }
        // **La clé de boîte**, annoncée une fois : ce qui permet aux
        // autres officines de sceller un message pour celle-ci seule.
        let box_public = hex(&self.device.box_public());
        let key = format!("boite:{box_public}");
        if !already.contains(&key) {
            let payload = serde_json::json!({
                "t": "boite",
                "k": box_public,
                "officine": officine,
            })
            .to_string()
            .into_bytes();
            self.journal
                .write(
                    &self.device,
                    trousseau,
                    Stream::Reseau,
                    &payload,
                    None,
                    &mut bpm_sync::OsEntropy,
                )
                .map_err(|e| format!("{e:?}"))?;
            done.push(key);
        }
        // **Les messages**, scellés pour leurs seules officines — et pour
        // celle-ci, qui les relit comme les autres. Un message dont une
        // destinataire n'a pas encore annoncé sa clé attend la prochaine
        // synchronisation.
        let files = db.outgoing_net_files()?;
        let keys: std::collections::HashMap<String, [u8; 32]> = db
            .net_box_keys()?
            .into_iter()
            .filter_map(|(device, k)| unhex::<32>(&k).map(|b| (device, b)))
            .collect();
        let mine = self.device.box_public();
        let members: std::collections::HashSet<&str> =
            self.peers.iter().map(|p| p.device.as_str()).collect();
        for m in db.outgoing_net_messages()? {
            let key = format!("msg:{}", m.uid);
            if already.contains(&key) {
                continue;
            }
            // **Par ce réseau, à ses seuls membres** : une conversation qui
            // réunit des officines de deux réseaux part par chacun, chacun
            // portant la boîte de ses membres.
            let here: Vec<&String> = m
                .peers
                .iter()
                .filter(|p| members.contains(p.as_str()))
                .collect();
            if here.is_empty() {
                continue;
            }
            let Some(mut recipients) = here
                .iter()
                .map(|p| keys.get(*p).copied())
                .collect::<Option<Vec<[u8; 32]>>>()
            else {
                continue;
            };
            recipients.push(mine);
            // Les participants que **ce** réseau connaît : les membres d'un
            // autre réseau n'y sont pas nommés.
            let mut all_peers: Vec<String> = here.iter().map(|p| (*p).clone()).collect();
            all_peers.push(hex(&self.device.id().0));
            let attached: Vec<crate::messages::FileMeta> = files
                .iter()
                .filter(|f| f.message == m.uid)
                .cloned()
                .collect();
            // Les morceaux de ses fichiers d'abord, scellés pour les mêmes
            // officines : le message qui les annonce arrive après eux, ou
            // avec eux.
            for f in &attached {
                for (n, d) in db.file_chunks(&f.uid)? {
                    let key = format!("morceau:{}:{n}", f.uid);
                    if already.contains(&key) {
                        continue;
                    }
                    let chunk = crate::messages::Chunk {
                        file: f.uid.clone(),
                        n,
                        d,
                    };
                    let bytes = serde_json::to_vec(&chunk).map_err(|e| e.to_string())?;
                    let sealed =
                        bpm_sync::boxed::seal(&recipients, &bytes, &mut bpm_sync::OsEntropy)
                            .map_err(|e| format!("{e:?}"))?;
                    let mut payload = crate::messages::BOX_TAG.to_vec();
                    payload.extend(sealed);
                    self.journal
                        .write(
                            &self.device,
                            trousseau,
                            Stream::Reseau,
                            &payload,
                            None,
                            &mut bpm_sync::OsEntropy,
                        )
                        .map_err(|e| format!("{e:?}"))?;
                    done.push(key);
                }
            }
            let inner = crate::messages::Incoming {
                uid: m.uid.clone(),
                conversation: m.conversation.clone(),
                title: m.title.clone(),
                peers: all_peers,
                officine: officine.to_owned(),
                author: m.author.clone(),
                body: m.body.clone(),
                sent_at: m.sent_at.clone(),
                files: attached,
            };
            let bytes = serde_json::to_vec(&inner).map_err(|e| e.to_string())?;
            let sealed = bpm_sync::boxed::seal(&recipients, &bytes, &mut bpm_sync::OsEntropy)
                .map_err(|e| format!("{e:?}"))?;
            let mut payload = crate::messages::BOX_TAG.to_vec();
            payload.extend(sealed);
            if payload.len() > bpm_sync::MAX_PAYLOAD {
                return Err(crate::strings::tr("msg_too_long").to_owned());
            }
            self.journal
                .write(
                    &self.device,
                    trousseau,
                    Stream::Reseau,
                    &payload,
                    None,
                    &mut bpm_sync::OsEntropy,
                )
                .map_err(|e| format!("{e:?}"))?;
            done.push(key);
        }
        self.keep(db)?;
        let keys: Vec<String> = done.iter().map(|k| self.done_key(k)).collect();
        db.mark_published(&keys)?;
        Ok(done.len())
    }

    /// Ranger dans la base les enregistrements que le journal tient.
    fn keep(&self, db: &Db) -> Result<usize, String> {
        let all: Vec<(String, Vec<u8>)> = self
            .journal
            .records()
            .map(|r| (hex(&r.id().0), r.encode()))
            .collect();
        db.keep_net_records_of(&self.id, &all)
    }

    /// Ce que les officines appairées ont écrit, rangé dans le journal des
    /// ruptures. **Seules les officines appairées sont lues** : un
    /// enregistrement d'une officine qu'on a retirée reste au journal du
    /// réseau, il n'entre plus dans la base. Rend combien étaient nouveaux.
    pub fn absorb(&self, db: &Db) -> Result<usize, String> {
        let Some(trousseau) = &self.trousseau else {
            return Ok(0);
        };
        let me = self.device.id();
        let known = self.known();
        // **Lu à travers la vue de confiance** : ce que les conversations
        // relaient d'auteurs qu'on n'a pas ajoutés reste au journal — pour
        // qu'on ne le redemande pas à chaque échange —, mais ne corrige
        // rien ici. Voir `trusted_view`.
        let facts: Vec<bpm_sync::Fact> = self
            .trusted_view()
            .read(trousseau, Stream::Reseau)
            .facts
            .into_iter()
            .filter(|f| f.author != me && known.contains(&f.author))
            .collect();
        // **Ce que chaque officine a envoyé, et sous quel nom.** Le
        // compte de ses enregistrements au journal, et le nom qu'elle
        // écrit dans chacun — le dernier lu. Un compte qui monte, ce sont
        // des nouvelles : c'est ce qui dit, dans la liste, qu'une
        // officine est vivante même quand on ne la joint jamais
        // directement.
        for author in &known {
            let theirs: Vec<&bpm_sync::Fact> =
                facts.iter().filter(|f| f.author == *author).collect();
            let name = theirs
                .iter()
                .rev()
                .find_map(|f| {
                    serde_json::from_slice::<serde_json::Value>(&f.payload)
                        .ok()?
                        .get("officine")?
                        .as_str()
                        .map(str::to_owned)
                })
                .unwrap_or_default();
            db.note_net_heard_in(
                &self.id,
                &hex(&author.0),
                &name,
                i64::try_from(theirs.len()).unwrap_or(i64::MAX),
            )?;
        }
        let events: Vec<crate::ruptures::Event> = facts
            .iter()
            .filter_map(|f| crate::ruptures::decode(&f.payload))
            .collect();
        // In causal order, so the last value an officine sent for a
        // property is the one kept.
        let shared: Vec<crate::pk::Shared> = facts
            .iter()
            .filter_map(|f| crate::pk::decode_shared(&f.payload))
            .collect();
        db.receive_net_facts(&shared)?;
        let versions: Vec<crate::versions::Edit> = facts
            .iter()
            .filter_map(|f| crate::versions::decode(&f.payload))
            .collect();
        // What the codex and the protocols changed here is versioned
        // **before** what arrives is ranged: an edit made before this
        // synchronisation replaced what this officine had, not what just
        // came in — written after, it would silently answer it.
        db.version_shared_entries("")?;
        db.receive_card_edits(&versions)?;
        // Les clés de boîte annoncées, puis les messages scellés pour
        // celle-ci — ceux qui ne s'ouvrent pas sont pour d'autres.
        for f in &facts {
            if let Some(k) = serde_json::from_slice::<serde_json::Value>(&f.payload)
                .ok()
                .filter(|v| v.get("t").and_then(|t| t.as_str()) == Some("boite"))
                .and_then(|v| v.get("k").and_then(|k| k.as_str()).map(str::to_owned))
            {
                db.set_net_box_key(&hex(&f.author.0), &k)?;
            }
        }
        let secret = self.device.box_secret();
        let me_hex = hex(&me.0);
        // Les messages d'abord — ils décrivent les fichiers —, les
        // morceaux ensuite : un morceau n'est accepté que pour un fichier
        // décrit, et de l'officine qui l'a décrit.
        let mut chunks: Vec<(crate::messages::Chunk, String)> = Vec::new();
        for f in &facts {
            let Some(sealed) = f.payload.strip_prefix(crate::messages::BOX_TAG) else {
                continue;
            };
            let Ok(inner) = bpm_sync::boxed::open(&secret, sealed) else {
                continue;
            };
            let author = hex(&f.author.0);
            if let Ok(m) = serde_json::from_slice::<crate::messages::Incoming>(&inner) {
                db.receive_net_message(&m, &author, &me_hex)?;
            } else if let Ok(c) = serde_json::from_slice::<crate::messages::Chunk>(&inner) {
                chunks.push((c, author));
            }
        }
        for (c, author) in &chunks {
            db.receive_chunk(c, author)?;
        }
        db.receive_supply_events(&events)
    }

    /// Déposer ses enregistrements dans le dossier d'échange, et lire ceux
    /// des autres. Rend combien d'enregistrements étaient nouveaux.
    ///
    /// Un fichier par officine, nommé d'après son empreinte : chacune
    /// n'écrit que le sien, et personne n'écrase celui d'une autre. Le
    /// fichier est écrit à côté puis mis en place, pour qu'une officine
    /// qui lit au même moment ne tombe jamais sur un fichier à moitié.
    pub fn exchange_folder(&mut self, db: &Db, folder: &Path) -> Result<usize, String> {
        if self.trousseau.is_none() {
            return Ok(0);
        }
        std::fs::create_dir_all(folder).map_err(|e| e.to_string())?;
        let me = self.device.id();
        let mine: Vec<u8> = self
            .journal
            .records()
            .filter(|r| r.author() == me)
            .flat_map(|r| {
                let bytes = r.encode();
                let mut framed = (bytes.len() as u32).to_be_bytes().to_vec();
                framed.extend(bytes);
                framed
            })
            .collect();
        // Un fichier par officine **et par réseau** : deux réseaux peuvent
        // partager un dossier sans que l'un lise l'autre.
        let name = if self.id.is_empty() {
            hex(&me.0[..10])
        } else {
            format!("{}-{}", hex(&me.0[..10]), self.id)
        };
        let target = folder.join(format!("{name}.bpmnet"));
        let part = folder.join(format!("{name}.bpmnet.part"));
        std::fs::write(&part, &mine).map_err(|e| e.to_string())?;
        std::fs::rename(&part, &target).map_err(|e| e.to_string())?;
        let known = self.known();
        // Les officines écartées ne prennent pas de place parmi les
        // trente-deux : sans cela, trente-deux identités jetables, même
        // écartées, cachaient pour de bon une vraie nouvelle venue.
        let set_aside: Vec<DeviceId> = ignored(db)
            .iter()
            .filter_map(|k| k.strip_prefix(&format!("{}|", self.id)))
            .filter_map(|d| unhex::<32>(d).map(DeviceId::from_bytes))
            .collect();
        self.strangers.clear();
        let mut added = 0;
        for entry in std::fs::read_dir(folder)
            .map_err(|e| e.to_string())?
            .flatten()
        {
            let path = entry.path();
            if path == target || path.extension().and_then(|e| e.to_str()) != Some("bpmnet") {
                continue;
            }

            let Ok(bytes) = std::fs::read(&path) else {
                continue;
            };
            let mut at = 0;
            while at + 4 <= bytes.len() {
                let len =
                    u32::from_be_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
                        as usize;
                at += 4;
                let Some(chunk) = bytes.get(at..at + len) else {
                    break;
                };
                at += len;
                // Signé par son auteur, d'une officine appairée, et **scellé
                // sous la clé de ce réseau** : un dossier partagé par deux
                // réseaux — ou le principal d'une officine qui est le second
                // d'une autre — ne mêle pas leurs enregistrements.
                //
                // D'une officine qu'on n'a pas ajoutée : **comptée, pas
                // gardée** — voir `introduced`.
                if let Ok(record) = Record::decode(chunk) {
                    let opened = self.trousseau.as_ref().and_then(|t| record.open(t).ok());
                    let Some(payload) = opened else {
                        continue;
                    };
                    if known.contains(&record.author()) {
                        if self.journal.insert(record) {
                            added += 1;
                        }
                    } else if record.author() != me && !set_aside.contains(&record.author()) {
                        self.saw_stranger(&record.author(), &payload);
                    }
                }
            }
        }
        self.keep(db)?;
        Ok(added)
    }

    /// **La vue de confiance du journal** : ce que cette officine et les
    /// siennes ont écrit, rien d'autre. Une « correction » ne dit pas qui
    /// a le droit de corriger : lue sur le journal entier, celle d'un
    /// auteur qu'on n'a pas ajouté — relayée par une officine qui, elle,
    /// l'a ajouté — effaçait ici l'enregistrement d'une officine appairée.
    fn trusted_view(&self) -> Journal {
        let mut trusted = self.known();
        trusted.push(self.device.id());
        let mut view = Journal::new();
        for r in self.journal.records() {
            if trusted.contains(&r.author()) {
                view.insert(r.clone());
            }
        }
        view
    }

    /// Compter un enregistrement d'une officine qu'on n'a pas ajoutée : son
    /// nom (le dernier lu, nettoyé comme celui d'une annonce) et combien.
    fn saw_stranger(&mut self, author: &DeviceId, payload: &[u8]) {
        let device = hex(&author.0);
        let name = serde_json::from_slice::<serde_json::Value>(payload)
            .ok()
            .and_then(|v| v.get("officine")?.as_str().map(clean_name))
            .filter(|n| !n.is_empty());
        let full = self.strangers.len() >= MOST_STRANGERS;
        match self.strangers.iter_mut().find(|o| o.device == device) {
            Some(o) => {
                o.records += 1;
                if let Some(n) = name {
                    o.name = n;
                }
            }
            None if !full => {
                self.strangers.push(Introduced {
                    device,
                    name: name.unwrap_or_default(),
                    records: 1,
                });
            }
            None => {}
        }
    }

    /// Converser avec une officine appairée qui a une porte ouverte. Rend
    /// l'empreinte de **celle qui a répondu** — pas forcément celle qu'on
    /// croyait trouver à cette adresse : deux adresses échangées par la
    /// box, et c'est l'autre qui répond.
    pub fn sync_with(
        &mut self,
        db: &Db,
        address: &str,
        patience: Duration,
    ) -> Result<Option<String>, String> {
        let Some(trousseau) = self.trousseau.clone() else {
            return Ok(None);
        };
        let mut link = bpm_sync::link::dial(address, patience).map_err(|e| format!("{e:?}"))?;
        let mut session = Session::new(
            &self.device,
            Some(&trousseau),
            Intent::Sync,
            true,
            &self.known(),
        )
        .map_err(|e| format!("{e:?}"))?;
        let mut meter = Meter::new();
        bpm_sync::drive(
            &mut session,
            &mut link,
            &mut self.journal,
            &mut meter,
            &mut |_| true,
        )
        .map_err(|e| format!("{e:?}"))?;
        self.keep(db)?;
        Ok(session.peer().map(|p| hex(&p.0)))
    }
}

/// Ce que le fil du réseau dit à l'écran.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Progress {
    /// Une porte est ouverte à cette adresse, en attente de l'autre
    /// officine.
    Waiting(String),
    /// Le code que les deux officines comparent. L'écran le montre, et
    /// répond par le canal qu'il a reçu avec la tâche.
    Code(String),
    /// Ce que les dossiers d'échange ont montré d'officines non ajoutées,
    /// réseau par réseau : `(id du réseau, officines)`.
    Introduced(Vec<(String, Vec<Introduced>)>),
    /// Fini : ce qui s'est passé, en une phrase.
    Done(String),
    Failed(String),
}

/// Ce qu'on demande au fil du réseau.
#[derive(Clone, Debug)]
pub enum Job {
    /// Ouvrir une porte le temps d'une invitation — dans ce réseau (vide :
    /// le principal). D'ordinaire avec un code d'invitation, **strict** :
    /// qui vient sans lui est refusé. `nearby` : l'invitation faite à une
    /// officine voisine depuis la carte, annoncée sur le réseau local,
    /// **sans code** — on compare alors les cinq groupes au téléphone.
    Invite {
        port: u16,
        network: String,
        nearby: bool,
    },
    /// Rejoindre un réseau en composant l'adresse de l'officine qui
    /// invite — avec le ticket de son code d'invitation quand on l'a :
    /// aucun code à comparer alors. Sans lui, la comparaison.
    Join {
        address: String,
        ticket: Option<bpm_sync::Ticket>,
    },
    /// Synchroniser : le dossier d'échange s'il y en a un, puis chaque
    /// officine qui a une adresse.
    Sync { folder: Option<PathBuf> },
    /// Composer l'adresse d'**une** officine : savoir si elle répond,
    /// sans attendre toutes les autres ni le dossier d'échange.
    Dial { device: String },
}

/// Combien d'officines non ajoutées un passage au dossier compte au plus.
pub const MOST_STRANGERS: usize = 32;

/// Cette officine et les siennes : les auteurs dont le rang règle
/// l'horloge du journal — voir `Journal::trust`.
fn trusted_ids(me: &Device, peers: &[Peer]) -> Vec<DeviceId> {
    peers
        .iter()
        .filter_map(|p| unhex::<32>(&p.device).map(DeviceId::from_bytes))
        .chain(std::iter::once(me.id()))
        .collect()
}

/// Le journal tel que la base le garde.
fn stored_journal(records: Vec<Vec<u8>>) -> Journal {
    let mut journal = Journal::new();
    for bytes in records {
        if let Ok(record) = Record::decode(&bytes) {
            journal.insert(record);
        }
    }
    journal
}

/// Une officine du réseau présentée par une autre : voir
/// [`Net::introduced`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Introduced {
    pub device: String,
    /// Le nom qu'elle écrit dans ses envois — le dernier lu.
    pub name: String,
    /// Combien d'enregistrements d'elle le journal tient.
    pub records: usize,
}

/// La clé d'une officine écartée d'un réseau, dans le réglage
/// `net_ignored`.
pub fn ignore_key(network: &str, device: &str) -> String {
    format!("{network}|{device}")
}

/// Les officines écartées : ce que la carte ne propose plus d'ajouter.
pub fn ignored(db: &Db) -> Vec<String> {
    db.setting("net_ignored")
        .and_then(|v| serde_json::from_str::<Vec<String>>(&v).ok())
        .unwrap_or_default()
}

/// Écarter une officine d'un réseau (`on`), ou la reprendre.
pub fn set_ignored(
    db: &Db,
    network: &str,
    device: &str,
    on: bool,
    day: &str,
) -> Result<(), String> {
    // Contre la valeur lue ; un autre poste qui écrit entre-temps, et on
    // relit pour réessayer — perdu, le geste ne se tait pas.
    for _ in 0..3 {
        let was = db.setting("net_ignored");
        let mut list = ignored(db);
        let key = ignore_key(network, device);
        list.retain(|k| *k != key);
        if on {
            list.push(key);
        }
        let value = serde_json::to_string(&list).map_err(|e| e.to_string())?;
        if db.set_setting("net_ignored", &value, was.as_deref(), day, "")? {
            return Ok(());
        }
    }
    Err(crate::strings::tr("net_peer_stale").to_owned())
}

/// **Ajouter une officine présentée par le réseau** : l'inscrire comme
/// si on l'avait appairée — elle tient déjà la clé du réseau —, puis
/// relire le dossier d'échange, qui garde désormais ce qu'elle dépose, et
/// lire ce qu'elle a envoyé. **Sans lui donner de nom** : celui qu'elle
/// écrit n'est pas un nom que l'officine a choisi — il s'affiche déjà
/// comme celui qu'elle signe ; l'opérateur la nomme s'il le veut.
pub fn adopt(
    db: &Db,
    network: &str,
    device: &str,
    folder: Option<&Path>,
    day: &str,
) -> Result<usize, String> {
    unhex::<32>(device).ok_or("identité illisible")?;
    let was = db.net_members("")?.iter().any(|p| p.device == device);
    db.add_net_peer(device, "", day)?;
    db.add_net_membership(network, device, was)?;
    let _ = set_ignored(db, network, device, false, day);
    let mut net = Net::load_network(db, network)?;
    if let Some(dir) = folder {
        net.exchange_folder(db, dir)?;
    }
    net.absorb(db)
}

/// Le code d'invitation : le ticket, puis où composer —
/// « K7QM-2XPA-9D3F-7H1Q@192.168.1.20:7742 ». Une seule chaîne à lire
/// ou à coller, et rien à comparer ensuite.
pub fn invitation_code(ticket: &bpm_sync::Ticket, address: &str) -> String {
    format!("{}@{}", ticket.text(), address.trim())
}

/// Ce qu'on a tapé dans « Rejoindre » : un code d'invitation, ou une
/// adresse seule. `None` quand cela ressemble à un code sans en être un
/// — un ticket mal recopié ne doit pas devenir une comparaison de code
/// à l'insu de l'opérateur.
pub fn read_join(text: &str) -> Option<(String, Option<bpm_sync::Ticket>)> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    match text.rsplit_once('@') {
        Some((code, address)) => {
            let ticket = bpm_sync::Ticket::parse(code)?;
            let address = address.trim();
            (!address.is_empty() && !address.contains(char::is_whitespace))
                .then(|| (address.to_owned(), Some(ticket)))
        }
        None => (!text.contains(char::is_whitespace)).then(|| (text.to_owned(), None)),
    }
}

/// L'identité de l'officine sur le réseau, en hexadécimal — sans lire
/// son journal : ce qu'une annonce sur le réseau local a besoin de dire.
pub fn device_hex(db: &Db) -> Result<String, String> {
    let seed = db.net_key("net_seed", &hex(&random32()))?;
    let seed = unhex::<32>(&seed).ok_or("identité du réseau illisible")?;
    Ok(hex(&Device::from_seed(seed).id().0))
}

/// Une officine entendue sur le réseau local.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Nearby {
    /// Son identité sur le réseau (hex).
    pub device: String,
    /// Le nom **qu'elle se donne** — rien ne le prouve : c'est la
    /// comparaison du code, à l'appairage, qui dit à qui l'on parle.
    pub name: String,
    /// L'adresse de son invitation, quand elle en a une ouverte.
    pub invite: Option<String>,
    /// L'adresse où elle écoute les officines appairées, quand elle le
    /// fait — ce qu'une officine appairée apprend pour la joindre.
    pub listen: Option<String>,
    /// La ville **qu'elle se donne**, pour la reconnaître d'un coup d'œil.
    pub place: String,
}

/// Le plus long nom qu'une annonce porte, en caractères.
const NEARBY_NAME: usize = 48;

/// L'annonce d'une officine sur le réseau local : son identité, le port
/// d'une invitation ouverte (0 : aucune) et son nom, en hexadécimal pour
/// que l'annonce reste une ligne de mots. **Rien d'autre** : ni réseau,
/// ni membre, ni donnée — une officine est un commerce qui a pignon sur
/// rue, et son nom sur le réseau local n'apprend rien de plus.
pub fn officine_beacon(
    device: &str,
    name: &str,
    place: &str,
    listen_port: u16,
    invite_port: u16,
) -> String {
    format!(
        "BPMOFFICINE2 {device} {listen_port} {invite_port} {} {}",
        hex(clean_name(name).as_bytes()),
        hex(clean_name(place).as_bytes())
    )
}

/// **La ville d'une adresse**, pour l'annonce : son dernier morceau —
/// « 12 rue des Lilas, 75011 Paris » donne « Paris ». Le code postal
/// n'apprend rien de plus à qui voit la carte, et prend la place.
pub fn town_of(address: &str) -> String {
    let last = address
        .split(['\n', ','])
        .map(str::trim)
        .rfind(|l| !l.is_empty())
        .unwrap_or("");
    let words: Vec<&str> = last
        .split_whitespace()
        .skip_while(|w| w.chars().all(|c| c.is_ascii_digit()))
        .collect();
    clean_name(&words.join(" ")).chars().take(32).collect()
}

/// Un nom tel qu'une annonce peut le porter : sans caractère de
/// commande, sans blanc aux bords, au plus [`NEARBY_NAME`] caractères.
fn clean_name(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect::<String>()
        .trim()
        .chars()
        .take(NEARBY_NAME)
        .collect()
}

/// L'inverse, avec l'adresse d'où l'annonce est venue. Tout ce qui n'est
/// pas exactement une annonce est `None` — l'annonce vient de n'importe
/// qui sur le réseau local.
pub fn heard_officine(text: &str, from: std::net::IpAddr) -> Option<Nearby> {
    let parts: Vec<&str> = text.split(' ').collect();
    // Deux versions : la première ne disait ni où écouter ni la ville.
    let (device, listen, invite, name_hex, place_hex) = match parts.as_slice() {
        ["BPMOFFICINE1", d, i, n] => (*d, "0", *i, *n, ""),
        ["BPMOFFICINE2", d, l, i, n, p] => (*d, *l, *i, *n, *p),
        _ => return None,
    };
    // **Réécrite, jamais reprise telle quelle** : `unhex` lit aussi les
    // majuscules, et une même clé écrite de deux façons passerait pour
    // deux officines — une voisine inconnue portant l'empreinte d'un
    // membre.
    let device = hex(&unhex::<32>(device)?);
    let listen: u16 = listen.parse().ok()?;
    let invite: u16 = invite.parse().ok()?;
    let text_of = |h: &str| -> Option<String> {
        if h.len() > NEARBY_NAME * 8 || !h.len().is_multiple_of(2) {
            return None;
        }
        let bytes: Option<Vec<u8>> = (0..h.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(h.get(i..i + 2)?, 16).ok())
            .collect();
        Some(clean_name(&String::from_utf8(bytes?).ok()?))
    };
    let at = |port: u16| (port != 0).then(|| std::net::SocketAddr::new(from, port).to_string());
    Some(Nearby {
        device,
        name: text_of(name_hex)?,
        invite: at(invite),
        listen: at(listen),
        place: text_of(place_hex)?,
    })
}

/// **Répondre à une officine appairée qui frappe** — de n'importe lequel
/// des réseaux de celle-ci : c'est elle qui dit lequel, et il faut
/// qu'elle en soit. Ce qu'elle apporte est gardé et lu comme après une
/// synchronisation. Rend l'empreinte de qui a frappé.
pub fn answer<L: bpm_sync::Link>(db: &Db, link: &mut L) -> Result<String, String> {
    let mut nets = Net::load_all(db)?;
    let choices: Vec<(Trousseau, Vec<DeviceId>)> = nets
        .iter()
        .filter_map(|n| n.trousseau.clone().map(|t| (t, n.known())))
        .collect();
    if choices.len() != nets.len() || nets.is_empty() {
        return Err(crate::strings::tr("net_err_no_network").to_owned());
    }
    let device = Device::from_seed(nets[0].device.seed());
    let mut journals: Vec<Journal> = nets
        .iter_mut()
        .map(|n| std::mem::take(&mut n.journal))
        .collect();
    let mut session = Session::answer_any(&device, choices).map_err(|e| format!("{e:?}"))?;
    let chosen = bpm_sync::drive_any(&mut session, link, &mut journals, &mut Meter::new())
        .map_err(|e| format!("{e:?}"));
    for (n, j) in nets.iter_mut().zip(journals) {
        n.journal = j;
    }
    let i = chosen?;
    let net = &nets[i];
    net.keep(db)?;
    let peer = session.peer().map(|p| hex(&p.0)).unwrap_or_default();
    let _ = db.note_net_dial(&peer, None);
    net.absorb(db)?;
    Ok(peer)
}

/// **Composer une officine appairée à l'adresse que son annonce donne**,
/// et ne garder cette adresse qu'**une fois qu'elle y a répondu** : une
/// annonce ne prouve rien — n'importe qui sur le réseau local peut en
/// écrire une sous l'identité d'une officine appairée. Une adresse
/// fausse mène à une poignée de main refusée, et l'adresse saisie ou
/// apprise auparavant reste. Rien pour une officine qu'on n'a pas
/// ajoutée.
pub fn dial_heard(db: &Db, device: &str, address: &str, officine: &str) -> Result<String, String> {
    use crate::strings::{tr, trn};
    let Some(mut net) = Net::load_all(db)?
        .into_iter()
        .find(|n| n.peers.iter().any(|p| p.device == device))
    else {
        return Err(tr("net_err_no_network").to_owned());
    };
    let Some(p) = net.peers.iter().find(|p| p.device == device).cloned() else {
        return Err(tr("net_err_no_network").to_owned());
    };
    let sent = net.publish(db, officine)?;
    let answered = net
        .sync_with(db, address, TALK_PATIENCE)
        .map_err(|e| dial_reason(&e))?;
    if answered.as_deref() != Some(device) {
        return Err(dial_reason("Handshake"));
    }
    if p.address.trim() != address.trim() {
        db.add_net_peer(device, address, "")?;
    }
    let _ = db.note_net_dial(device, None);
    let received = net.absorb(db)?;
    let name = [p.name, p.seen_as]
        .into_iter()
        .find(|n| !n.trim().is_empty())
        .unwrap_or_else(|| peer_groups(device));
    Ok(trn("net_done_dialed", &[&name, &sent, &received]))
}

/// Combien d'attente pour une invitation, et pour une conversation.
const INVITE_PATIENCE: Duration = Duration::from_secs(300);
const TALK_PATIENCE: Duration = Duration::from_secs(8);

/// Lancer une tâche sur son propre fil, avec sa propre connexion à la
/// base — la règle de `maintenance.rs`. `answers` porte la réponse de
/// l'écran au code affiché.
pub fn spawn(
    job: Job,
    path: PathBuf,
    password: String,
    officine: String,
    today: String,
    answers: std::sync::mpsc::Receiver<bool>,
) -> std::sync::mpsc::Receiver<Progress> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let said = run(&job, &path, &password, &officine, &today, &tx, &answers);
        let _ = tx.send(match said {
            Ok(done) => Progress::Done(done),
            Err(e) => Progress::Failed(e),
        });
    });
    rx
}

/// Ce que fait le fil. Public pour la synchronisation à la fermeture, qui
/// n'a plus d'écran où montrer un code et ne demande donc jamais que
/// [`Job::Sync`].
pub fn run(
    job: &Job,
    path: &Path,
    password: &str,
    officine: &str,
    today: &str,
    tx: &std::sync::mpsc::Sender<Progress>,
    answers: &std::sync::mpsc::Receiver<bool>,
) -> Result<String, String> {
    use crate::strings::{tr, trn};
    let db = Db::open(path, password)?;
    let mut net = match job {
        Job::Invite { network, .. } => Net::load_network(&db, network)?,
        _ => Net::load(&db)?,
    };
    let mut confirm = |code: bpm_sync::Fingerprint| {
        let _ = tx.send(Progress::Code(code.groups()));
        answers.recv().unwrap_or(false)
    };
    match job {
        Job::Invite { port, nearby, .. } => {
            let trousseau = net
                .trousseau
                .clone()
                .ok_or_else(|| tr("net_err_no_network").to_owned())?;
            let door = bpm_sync::link::Door::open(&format!("0.0.0.0:{port}"))
                .map_err(|_| tr("net_err_port").to_owned())?;
            // **Un ticket par invitation**, tiré ici et jamais écrit : il
            // vit le temps de cette porte, et une preuve fausse la ferme.
            let ticket = (!nearby).then(|| bpm_sync::Ticket::generate(&mut bpm_sync::OsEntropy));
            let at = format!("{}:{port}", local_address());
            let _ = tx.send(Progress::Waiting(match &ticket {
                Some(t) => invitation_code(t, &at),
                None => at,
            }));
            let mut link = door
                .accept(INVITE_PATIENCE)
                .map_err(|_| tr("net_err_nobody").to_owned())?;
            let mut session = Session::new(
                &net.device,
                Some(&trousseau),
                Intent::Invite,
                false,
                &net.known(),
            )
            .and_then(|s| match ticket {
                Some(t) => s.with_ticket(t),
                None => Ok(s),
            })
            .map_err(|e| format!("{e:?}"))?;
            let mut meter = Meter::new();
            bpm_sync::drive(
                &mut session,
                &mut link,
                &mut net.journal,
                &mut meter,
                &mut confirm,
            )
            .map_err(|_| tr("net_err_refused").to_owned())?;
            if let Some(peer) = session.peer() {
                let device = hex(&peer.0);
                let was = db.net_members("")?.iter().any(|p| p.device == device);
                db.add_net_peer(&device, "", today)?;
                // Au principal aussi, l'appartenance s'écrit : une officine
                // déjà d'un autre réseau n'y serait pas comptée sinon.
                db.add_net_membership(&net.id, &device, was)?;
            }
            net.keep(&db)?;
            Ok(tr("net_done_invited").to_owned())
        }
        Job::Join { address, ticket } => {
            // **Déjà d'un réseau, on en rejoint un de plus** : la clé reçue
            // devient celle d'un réseau à part, le principal ne bouge pas.
            let second = net.in_network();
            let mut link = bpm_sync::link::dial(address, TALK_PATIENCE)
                .map_err(|_| tr("net_err_unreachable").to_owned())?;
            let mut session = Session::new(&net.device, None, Intent::Join, true, &[])
                .and_then(|s| match ticket {
                    Some(t) => s.with_ticket(*t),
                    None => Ok(s),
                })
                .map_err(|e| format!("{e:?}"))?;
            let mut meter = Meter::new();
            // **Pour un réseau de plus, un journal vide** : l'appairage
            // échange des enregistrements, et ceux du principal n'ont rien
            // à faire chez l'officine qui invite dans un autre réseau.
            let mut fresh = Journal::new();
            let journal = if second { &mut fresh } else { &mut net.journal };
            bpm_sync::drive(&mut session, &mut link, journal, &mut meter, &mut confirm)
                .map_err(|_| tr("net_err_refused").to_owned())?;
            let joined = session
                .joined()
                .cloned()
                .ok_or_else(|| tr("net_err_refused").to_owned())?;
            // Un réseau dont l'officine est déjà — principal ou autre — ne
            // se range pas une seconde fois sous un autre nom.
            let fingerprint = hex(&joined.name().bytes());
            let principal = net.trousseau.as_ref().map(|t| hex(&t.name().bytes()));
            if principal.as_deref() == Some(fingerprint.as_str())
                || db.net_networks()?.iter().any(|n| n.id == fingerprint)
            {
                return Ok(tr("net_done_already_member").to_owned());
            }
            let id = if second {
                let id = fingerprint;
                db.add_net_network(&id, &hex(&joined.secret()), "", today)?;
                id
            } else {
                db.net_key("net_trousseau", &hex(&joined.secret()))?;
                String::new()
            };
            if let Some(peer) = session.peer() {
                let device = hex(&peer.0);
                let was = db.net_members("")?.iter().any(|p| p.device == device);
                db.add_net_peer(&device, address, today)?;
                db.add_net_membership(&id, &device, was)?;
            }
            // Les enregistrements reçus pendant l'appairage — et seulement
            // ceux que la clé du réseau rejoint ouvre.
            let source = if second { &fresh } else { &net.journal };
            let all: Vec<(String, Vec<u8>)> = source
                .records()
                .filter(|r| r.open(&joined).is_ok())
                .map(|r| (hex(&r.id().0), r.encode()))
                .collect();
            db.keep_net_records_of(&id, &all)?;
            let net = Net::load_network(&db, &id)?;
            let received = net.absorb(&db)?;
            Ok(trn("net_done_joined", &[&received]))
        }
        Job::Sync { folder } => {
            let nets = Net::load_all(&db)?;
            if nets.is_empty() {
                return Err(tr("net_err_no_network").to_owned());
            }
            let (mut sent, mut received) = (0, 0);
            let mut failed = Vec::new();
            let mut introduced = Vec::new();
            for mut net in nets {
                // Un réseau qui échoue n'arrête pas les autres.
                match net.publish(&db, officine) {
                    Ok(n) => sent += n,
                    Err(e) => failed.push(e),
                }
                // Le principal prend le dossier de la configuration ; les
                // autres, le leur.
                let dir = if net.id.is_empty() {
                    folder.clone()
                } else {
                    (!net.folder.trim().is_empty()).then(|| PathBuf::from(net.folder.trim()))
                };
                // Ce que le dossier a montré — seulement d'un dossier lu :
                // sans lui, rien de neuf n'a été vu, et l'écran garde ce
                // qu'il savait.
                if let Some(dir) = &dir {
                    match net.exchange_folder(&db, dir) {
                        Ok(_) => introduced.push((net.id.clone(), net.introduced(&[]))),
                        Err(e) => failed.push(format!("{} : {e}", dir.display())),
                    }
                }
                let peers = net.peers.clone();
                for p in peers.iter().filter(|p| !p.address.trim().is_empty()) {
                    // Chaque conversation est notée, réussie ou non, avec sa
                    // raison ; une note qui ne s'écrit pas n'arrête pas les
                    // autres conversations.
                    match net.sync_with(&db, &p.address, TALK_PATIENCE) {
                        Ok(answered) => {
                            let _ =
                                db.note_net_dial(answered.as_deref().unwrap_or(&p.device), None);
                        }
                        Err(e) => {
                            let _ = db.note_net_dial(&p.device, Some(&dial_reason(&e)));
                            let who = if p.name.trim().is_empty() {
                                p.address.clone()
                            } else {
                                p.name.clone()
                            };
                            if !failed.contains(&who) {
                                failed.push(who);
                            }
                        }
                    }
                }
                match net.absorb(&db) {
                    Ok(n) => received += n,
                    Err(e) => failed.push(e),
                }
            }
            let _ = tx.send(Progress::Introduced(introduced));
            let mut said = trn("net_done_synced", &[&sent, &received]);
            if !failed.is_empty() {
                said.push_str(&crate::strings::trf(
                    "net_done_unreached",
                    failed.join(", "),
                ));
            }
            Ok(said)
        }
        Job::Dial { device } => {
            let _ = net;
            dial_device(&db, device, officine)
        }
    }
}

/// Composer l'adresse d'**une** officine appairée et échanger avec elle :
/// ce que « Essayer » demande, et ce que le fil automatique fait quand il
/// entend une officine appairée sur le réseau local.
pub fn dial_device(db: &Db, device: &str, officine: &str) -> Result<String, String> {
    use crate::strings::{tr, trn};
    // Le réseau où cette officine est : le premier qui la compte.
    let Some(mut net) = Net::load_all(db)?
        .into_iter()
        .find(|n| n.peers.iter().any(|p| p.device == device))
    else {
        return Err(tr("net_err_no_network").to_owned());
    };
    let Some(p) = net
        .peers
        .iter()
        .find(|p| p.device == device && !p.address.trim().is_empty())
        .cloned()
    else {
        return Err(tr("net_err_no_address").to_owned());
    };
    let name = if p.name.trim().is_empty() {
        p.address.clone()
    } else {
        p.name.clone()
    };
    let sent = net.publish(db, officine)?;
    match net.sync_with(db, &p.address, TALK_PATIENCE) {
        Ok(answered) => {
            let answered = answered.unwrap_or_else(|| p.device.clone());
            let _ = db.note_net_dial(&answered, None);
            let received = net.absorb(db)?;
            if answered == p.device {
                return Ok(trn("net_done_dialed", &[&name, &sent, &received]));
            }
            // Une autre officine du réseau répond à cette adresse (un bail
            // DHCP qui a changé) : l'échange a eu lieu, mais celle qu'on
            // composait n'a pas répondu.
            let reason = dial_reason("Handshake");
            let _ = db.note_net_dial(&p.device, Some(&reason));
            let who = net
                .peers
                .iter()
                .find(|q| q.device == answered)
                .map(|q| q.name.trim().to_owned())
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| crate::network::peer_groups(&answered));
            Ok(trn(
                "net_done_dialed_other",
                &[&who, &name, &sent, &received],
            ))
        }
        Err(e) => {
            let reason = dial_reason(&e);
            let _ = db.note_net_dial(&p.device, Some(&reason));
            Err(format!("{name} : {reason}"))
        }
    }
}

/// Pourquoi une conversation directe a échoué, en une phrase qu'on peut
/// lire au comptoir.
///
/// `bpm-sync` rend ses refus sans détail, et c'est voulu (voir
/// `sync/src/lib.rs`) : un lien qui casse est `Link`, qu'il s'agisse d'un
/// poste éteint, d'un port fermé ou d'une adresse hors d'atteinte — ce
/// que la phrase dit tel quel plutôt que de deviner lequel. Les autres
/// refus disent quelque chose de plus précis : l'officine qui répond
/// n'est pas celle qu'on attendait, ou ne tient plus la même clé.
fn dial_reason(raw: &str) -> String {
    use crate::strings::tr;
    tr(match raw.trim() {
        "Link" => "net_dial_link",
        "Handshake" | "Unknown" => "net_dial_identity",
        "Seal" => "net_dial_key",
        _ => "net_dial_other",
    })
    .to_owned()
}

/// L'adresse de ce poste sur le réseau local, à lire à l'autre officine.
///
/// Un `connect` UDP ne transmet **rien** : il demande seulement au système
/// par quelle interface il enverrait, ce qui est l'adresse qu'on cherche.
/// Faute de mieux, « ce poste ».
fn local_address() -> String {
    std::net::UdpSocket::bind("0.0.0.0:0")
        .and_then(|s| {
            s.connect("192.0.2.1:9")?;
            s.local_addr()
        })
        .map(|a| a.ip().to_string())
        .unwrap_or_else(|_| crate::strings::tr("net_this_post").to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn officine(tag: &str) -> (std::path::PathBuf, crate::db::Swept, Db) {
        let dir = std::env::temp_dir().join(format!("bpm-caddy-net-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let swept = crate::db::Swept(dir.clone());
        let db = Db::open(&dir.join("net.db"), "secret").unwrap();
        (dir, swept, db)
    }

    fn subst(product: &str, other: &str) -> crate::ruptures::Event {
        crate::ruptures::Event {
            uid: String::new(),
            day: "2026-09-20".to_owned(),
            kind: crate::ruptures::Kind::Substitution,
            product: product.to_owned(),
            product_dci: String::new(),
            other: other.to_owned(),
            other_dci: String::new(),
            outcome: crate::ruptures::Outcome::Accepted,
            note: String::new(),
            operator: "CL".to_owned(),
            source: String::new(),
            refers: String::new(),
        }
    }

    /// **Les lignes du TROD voyagent** comme le codex : une posologie
    /// corrigée dans une officine s'applique chez l'autre, qui avait la
    /// même ligne livrée, et une ligne ajoutée y est créée.
    #[test]
    fn a_trod_line_edited_in_one_officine_applies_in_the_other() {
        let (dir_a, _sa, a) = officine("trod-a");
        let (_dir_b, _sb, b) = officine("trod-b");
        let folder = dir_a.join("echange");
        Net::create(&a).unwrap();
        b.net_key("net_trousseau", &a.setting("net_trousseau").unwrap())
            .unwrap();
        let (na, nb) = (Net::load(&a).unwrap(), Net::load(&b).unwrap());
        a.add_net_peer(&hex(&nb.device.id().0), "", "2026-09-24")
            .unwrap();
        b.add_net_peer(&hex(&na.device.id().0), "", "2026-09-24")
            .unwrap();
        a.seed_trod_lines().unwrap();
        b.seed_trod_lines().unwrap();

        // A corrige la posologie de l'amoxicilline adulte, et ajoute une
        // ligne au protocole de l'angine.
        let amox = a.trod_lines("angine").unwrap()[0].clone();
        let changed = crate::ordonnance::Offer {
            posologies: vec!["1 g matin et soir pendant 6 jours".to_owned()],
            ..amox.clone()
        };
        assert!(a.update_trod_line(&changed, &amox).unwrap());
        let pristi = a.add_trod_line("angine", "Pristinamycine 500 mg").unwrap();
        let blank = a
            .trod_lines("angine")
            .unwrap()
            .into_iter()
            .find(|o| o.id == pristi)
            .unwrap();
        assert!(a
            .update_trod_line(
                &crate::ordonnance::Offer {
                    posologies: vec!["1 g deux fois par jour pendant 4 jours".to_owned()],
                    ..blank.clone()
                },
                &blank,
            )
            .unwrap());

        let mut na = Net::load(&a).unwrap();
        assert!(na.publish(&a, "Pharmacie du Centre").unwrap() >= 2);
        na.exchange_folder(&a, &folder).unwrap();
        let mut nb = Net::load(&b).unwrap();
        nb.exchange_folder(&b, &folder).unwrap();
        nb.absorb(&b).unwrap();

        let theirs = b.trod_lines("angine").unwrap();
        let same = theirs.iter().find(|o| o.name == amox.name).unwrap();
        assert_eq!(
            same.posologies,
            vec!["1 g matin et soir pendant 6 jours".to_owned()],
            "la correction s'est appliquée"
        );
        assert!(
            theirs.iter().any(|o| o.name == "Pristinamycine 500 mg"),
            "la ligne ajoutée existe chez B"
        );
        // Et rien d'une autre indication n'a bougé.
        let names = |db: &Db| -> Vec<(String, Vec<String>)> {
            db.trod_lines("cystite")
                .unwrap()
                .into_iter()
                .map(|o| (o.name, o.posologies))
                .collect()
        };
        assert_eq!(names(&b), names(&a));
    }

    /// **Une ligne renommée reste une ligne** chez les autres : son nom
    /// voyage comme un champ, sous son identité d'origine. Et une ligne
    /// livrée que l'autre officine a retirée ne revient pas, vide, sur
    /// son ordonnance.
    #[test]
    fn a_renamed_trod_line_stays_one_line_and_a_removed_one_stays_removed() {
        let (dir_a, _sa, a) = officine("trod-id-a");
        let (_dir_b, _sb, b) = officine("trod-id-b");
        let folder = dir_a.join("echange");
        Net::create(&a).unwrap();
        b.net_key("net_trousseau", &a.setting("net_trousseau").unwrap())
            .unwrap();
        let (na, nb) = (Net::load(&a).unwrap(), Net::load(&b).unwrap());
        a.add_net_peer(&hex(&nb.device.id().0), "", "2026-09-24")
            .unwrap();
        b.add_net_peer(&hex(&na.device.id().0), "", "2026-09-24")
            .unwrap();
        a.seed_trod_lines().unwrap();
        b.seed_trod_lines().unwrap();
        let before = b.trod_lines("angine").unwrap().len();

        // A renomme l'amoxicilline adulte et corrige la céfuroxime.
        let lines = a.trod_lines("angine").unwrap();
        let amox = lines[0].clone();
        assert!(a
            .update_trod_line(
                &crate::ordonnance::Offer {
                    name: "Amoxicilline 1 g (Clamoxyl)".to_owned(),
                    ..amox.clone()
                },
                &amox
            )
            .unwrap());
        let cefu = lines
            .iter()
            .find(|o| o.name.starts_with("Céfuroxime"))
            .unwrap()
            .clone();
        assert!(a
            .update_trod_line(
                &crate::ordonnance::Offer {
                    caution: "Précaution revue par le groupement.".to_owned(),
                    ..cefu.clone()
                },
                &cefu
            )
            .unwrap());
        // B, de son côté, a retiré la céfuroxime.
        let b_cefu = b
            .trod_lines("angine")
            .unwrap()
            .into_iter()
            .find(|o| o.name == cefu.name)
            .unwrap();
        assert!(b.delete_trod_line(b_cefu.id, &b_cefu.name).unwrap());

        let mut na = Net::load(&a).unwrap();
        na.publish(&a, "Pharmacie du Centre").unwrap();
        na.exchange_folder(&a, &folder).unwrap();
        let mut nb = Net::load(&b).unwrap();
        nb.exchange_folder(&b, &folder).unwrap();
        nb.absorb(&b).unwrap();

        let theirs = b.trod_lines("angine").unwrap();
        assert_eq!(
            theirs.len(),
            before - 1,
            "le renommage n'ajoute pas de ligne, la ligne retirée ne revient pas"
        );
        assert!(theirs
            .iter()
            .any(|o| o.name == "Amoxicilline 1 g (Clamoxyl)"));
        assert!(
            !theirs.iter().any(|o| o.name == amox.name),
            "renommée, pas doublée"
        );
        assert!(!theirs.iter().any(|o| o.name == cefu.name));
    }

    /// **Le catalogue des vaccins voyage** : un schéma corrigé dans une
    /// officine s'applique chez l'autre, un vaccin ajouté y est créé.
    #[test]
    fn a_vaccine_edited_in_one_officine_applies_in_the_other() {
        let (dir_a, _sa, a) = officine("vacc-a");
        let (_dir_b, _sb, b) = officine("vacc-b");
        let folder = dir_a.join("echange");
        Net::create(&a).unwrap();
        b.net_key("net_trousseau", &a.setting("net_trousseau").unwrap())
            .unwrap();
        let (na, nb) = (Net::load(&a).unwrap(), Net::load(&b).unwrap());
        a.add_net_peer(&hex(&nb.device.id().0), "", "2026-09-24")
            .unwrap();
        b.add_net_peer(&hex(&na.device.id().0), "", "2026-09-24")
            .unwrap();
        a.seed_vaccine_catalogue().unwrap();
        b.seed_vaccine_catalogue().unwrap();

        let first = a.vaccine_catalogue().unwrap()[0].clone();
        let changed = crate::vaccines::Vaccine {
            schedule: "Schéma revu par le groupement".to_owned(),
            ..first.clone()
        };
        assert!(a.update_vaccine(&changed, &first).unwrap());
        let added = a.add_vaccine("Vaccin du groupement").unwrap();
        let blank = a
            .vaccine_catalogue()
            .unwrap()
            .into_iter()
            .find(|v| v.id == added)
            .unwrap();
        assert!(a
            .update_vaccine(
                &crate::vaccines::Vaccine {
                    schedule: "3 doses".to_owned(),
                    ..blank.clone()
                },
                &blank
            )
            .unwrap());

        let mut na = Net::load(&a).unwrap();
        na.publish(&a, "Pharmacie du Centre").unwrap();
        na.exchange_folder(&a, &folder).unwrap();
        let mut nb = Net::load(&b).unwrap();
        nb.exchange_folder(&b, &folder).unwrap();
        nb.absorb(&b).unwrap();

        let theirs = b.vaccine_catalogue().unwrap();
        assert_eq!(
            theirs
                .iter()
                .find(|v| v.label == first.label)
                .unwrap()
                .schedule,
            "Schéma revu par le groupement"
        );
        assert_eq!(
            theirs
                .iter()
                .find(|v| v.label == "Vaccin du groupement")
                .map(|v| v.schedule.as_str()),
            Some("3 doses"),
            "le vaccin ajouté existe chez B, avec son schéma"
        );
    }

    /// **Composer une seule officine** : elle seule est tentée, l'échec
    /// dit son nom et sa raison, et la tentative est notée comme pour
    /// une synchronisation. Une officine sans adresse ne se compose pas.
    #[test]
    fn dialling_one_officine_tries_it_alone() {
        let (dir, _s, a) = officine("dial_one");
        Net::create(&a).unwrap();
        let one = Device::from_seed([7; 32]);
        let other = Device::from_seed([8; 32]);
        a.add_net_peer(&hex(&one.id().0), "127.0.0.1:1", "2026-09-24")
            .unwrap();
        a.add_net_peer(&hex(&other.id().0), "127.0.0.1:2", "2026-09-24")
            .unwrap();
        a.add_net_peer(&hex(&[9; 32]), "", "2026-09-24").unwrap();
        let (tx, _rx) = std::sync::mpsc::channel();
        let (_atx, arx) = std::sync::mpsc::channel();
        let go = |device: String| {
            run(
                &Job::Dial { device },
                &dir.join("net.db"),
                "secret",
                "Pharmacie du Centre",
                "2026-09-24",
                &tx,
                &arx,
            )
        };
        let err = go(hex(&one.id().0)).unwrap_err();
        assert!(err.starts_with("127.0.0.1:1 : "), "{err}");
        assert!(err.contains(crate::strings::tr("net_dial_link")), "{err}");
        let peers = a.net_peers().unwrap();
        let tried = |d: &Device| {
            !peers
                .iter()
                .find(|p| p.device == hex(&d.id().0))
                .unwrap()
                .last_try
                .is_empty()
        };
        assert!(tried(&one), "la tentative est notée");
        assert!(!tried(&other), "les autres ne sont pas composées");
        assert_eq!(
            go(hex(&[9; 32])).unwrap_err(),
            crate::strings::tr("net_err_no_address")
        );
    }

    /// **Une officine qu'on ne joint pas le dit, avec l'heure et la
    /// raison** — et une réussite ultérieure effacerait la raison.
    #[test]
    fn a_refused_dial_is_noted_with_its_reason() {
        let (dir, _s, a) = officine("dial");
        Net::create(&a).unwrap();
        let other = Device::from_seed([7; 32]);
        // Le port 1 de la boucle locale : personne n'y écoute, la
        // connexion est refusée tout de suite.
        a.add_net_peer(&hex(&other.id().0), "127.0.0.1:1", "2026-09-24")
            .unwrap();
        let (tx, _rx) = std::sync::mpsc::channel();
        let (_atx, arx) = std::sync::mpsc::channel();
        let said = run(
            &Job::Sync { folder: None },
            &dir.join("net.db"),
            "secret",
            "Pharmacie du Centre",
            "2026-09-24",
            &tx,
            &arx,
        )
        .unwrap();
        assert!(
            said.contains("127.0.0.1:1"),
            "l'officine injoignable est nommée : {said}"
        );
        let peer = &a.net_peers().unwrap()[0];
        assert!(!peer.last_try.is_empty(), "la tentative est datée");
        assert!(peer.last_ok.is_empty());
        assert!(!peer.last_error.is_empty(), "et sa raison écrite");
        assert_eq!(
            peer.last_error,
            crate::strings::tr("net_dial_link"),
            "une raison lisible, pas l'erreur brute"
        );
        // Une réussite efface l'erreur d'avant.
        a.note_net_dial(&peer.device, None).unwrap();
        let peer = &a.net_peers().unwrap()[0];
        assert!(peer.last_error.is_empty() && !peer.last_ok.is_empty());
    }

    /// **Deux officines, un dossier d'échange, et ce qui en sort.**
    /// L'une note une substitution ; l'autre la lit, sous le nom de la
    /// première. Le dossier ne contient que des octets scellés — ni le
    /// produit ni le nom de l'officine n'y sont lisibles —, et une
    /// officine qui n'est pas appairée n'y est pas lue.
    /// **Un message entre deux officines, que la troisième transporte
    /// sans le lire.** Chacune annonce sa clé de boîte ; le message est
    /// scellé pour B seule (et A, qui l'a écrit) ; C, du même réseau,
    /// reçoit l'enregistrement et n'en tire rien.
    #[test]
    fn a_message_reaches_its_officine_and_no_other() {
        use crate::messages::Channel;
        let (dir_a, _sa, a) = officine("msg-a");
        let (_db, _sb, b) = officine("msg-b");
        let (_dc, _sc, c) = officine("msg-c");
        let folder = dir_a.join("echange");
        Net::create(&a).unwrap();
        let key = a.setting("net_trousseau").unwrap();
        b.net_key("net_trousseau", &key).unwrap();
        c.net_key("net_trousseau", &key).unwrap();
        let ids: Vec<String> = [&a, &b, &c]
            .iter()
            .map(|d| hex(&Net::load(d).unwrap().device.id().0))
            .collect();
        for (i, d) in [&a, &b, &c].into_iter().enumerate() {
            for (k, id) in ids.iter().enumerate() {
                if k != i {
                    d.add_net_peer(id, "", "2026-09-24").unwrap();
                }
            }
        }
        // Un premier tour : chacune annonce sa clé de boîte.
        for (d, name) in [(&a, "Centre"), (&b, "Port"), (&c, "Gare")] {
            let mut n = Net::load(d).unwrap();
            n.publish(d, name).unwrap();
            n.exchange_folder(d, &folder).unwrap();
        }
        for d in [&a, &b, &c] {
            let mut n = Net::load(d).unwrap();
            n.exchange_folder(d, &folder).unwrap();
            n.absorb(d).unwrap();
        }
        assert_eq!(
            a.net_box_keys().unwrap().len(),
            2,
            "A connaît les clés de B et C"
        );
        // A écrit à B.
        let conv = a
            .create_conversation(
                "",
                Channel::Officines,
                "Rupture Diprosone",
                &[],
                None,
                &[ids[1].clone()],
                "CL",
            )
            .unwrap();
        let sent = a
            .post_message(
                conv,
                "",
                "CL",
                "Il vous en reste ? Patient : Jean Dupont",
                None,
                "",
                Some("2026-09-24 10:00:00"),
            )
            .unwrap()
            .unwrap();
        // Et un fichier joint, en trois morceaux.
        let doc: Vec<u8> = (0..30_000).map(|i| (i % 253) as u8).collect();
        let muid = a.message_uid(sent).unwrap();
        a.attach_file(&muid, "../ordonnance.pdf", &doc).unwrap();
        let mut na = Net::load(&a).unwrap();
        assert_eq!(
            na.publish(&a, "Centre").unwrap(),
            4,
            "trois morceaux et le message"
        );
        na.exchange_folder(&a, &folder).unwrap();
        for entry in std::fs::read_dir(&folder).unwrap().flatten() {
            let text = String::from_utf8_lossy(&std::fs::read(entry.path()).unwrap()).to_string();
            assert!(!text.contains("Dupont"), "rien de lisible dans le dossier");
        }
        let mut nb = Net::load(&b).unwrap();
        nb.exchange_folder(&b, &folder).unwrap();
        nb.absorb(&b).unwrap();
        let theirs = b.conversations().unwrap();
        assert_eq!(theirs.len(), 1);
        assert_eq!(theirs[0].title, "Rupture Diprosone");
        assert_eq!(theirs[0].peers, vec![ids[0].clone()], "B répond à A");
        let got = b.conversation_messages(theirs[0].id).unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].source, "Centre");
        assert_eq!(got[0].author, "CL");
        let files = b.conversation_files(theirs[0].id).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].0.name, "ordonnance.pdf", "sans chemin");
        assert_eq!(files[0].1, 3, "les trois morceaux");
        assert_eq!(b.file_bytes(&files[0].0).unwrap(), Some(doc.clone()));
        // Absorber deux fois ne double rien.
        nb.absorb(&b).unwrap();
        assert_eq!(b.conversation_messages(theirs[0].id).unwrap().len(), 1);
        // C a l'enregistrement, pas le message.
        let mut nc = Net::load(&c).unwrap();
        nc.exchange_folder(&c, &folder).unwrap();
        nc.absorb(&c).unwrap();
        assert!(c.conversations().unwrap().is_empty(), "C ne lit rien");
        // Et la réponse de B revient dans la conversation de A.
        b.post_message(
            theirs[0].id,
            "",
            "YS",
            "Oui, deux boîtes.",
            None,
            "",
            Some("2026-09-24 10:05:00"),
        )
        .unwrap();
        nb.publish(&b, "Port").unwrap();
        nb.exchange_folder(&b, &folder).unwrap();
        let mut na = Net::load(&a).unwrap();
        na.exchange_folder(&a, &folder).unwrap();
        na.absorb(&a).unwrap();
        assert_eq!(a.conversations().unwrap().len(), 1, "la même conversation");
        let thread = a.conversation_messages(conv).unwrap();
        assert_eq!(thread.len(), 2);
        assert_eq!(thread[1].source, "Port");
    }

    /// **Une officine du réseau ne se fait pas passer pour une autre**, ne
    /// glisse rien dans une conversation dont elle s'absente, et ce qu'elle
    /// envoie n'est jamais republié sous la signature de qui le reçoit ;
    /// un morceau pour le fichier d'une autre est ignoré.
    #[test]
    fn a_hostile_member_cannot_forge_inject_or_launder() {
        let (dir_m, _sm, m) = officine("hostile-m");
        let (_dv, _sv, v) = officine("hostile-v");
        let folder = dir_m.join("echange");
        Net::create(&m).unwrap();
        let key = m.setting("net_trousseau").unwrap();
        v.net_key("net_trousseau", &key).unwrap();
        let (im, iv) = (
            hex(&Net::load(&m).unwrap().device.id().0),
            hex(&Net::load(&v).unwrap().device.id().0),
        );
        m.add_net_peer(&iv, "", "2026-09-24").unwrap();
        v.add_net_peer(&im, "", "2026-09-24").unwrap();
        v.set_net_peer(&im, "Officine M", "", ("", "")).unwrap();
        let v_box = Net::load(&v).unwrap().device.box_public();
        // M écrit à la main dans son journal : ce qu'un poste modifié
        // pourrait faire.
        let send = |inner: &[u8]| {
            let mut nm = Net::load(&m).unwrap();
            let sealed = bpm_sync::boxed::seal(&[v_box], inner, &mut bpm_sync::OsEntropy).unwrap();
            let mut payload = crate::messages::BOX_TAG.to_vec();
            payload.extend(sealed);
            let trousseau = nm.trousseau.clone().unwrap();
            let device = Device::from_seed(nm.device.seed());
            nm.journal
                .write(
                    &device,
                    &trousseau,
                    Stream::Reseau,
                    &payload,
                    None,
                    &mut bpm_sync::OsEntropy,
                )
                .unwrap();
            nm.keep(&m).unwrap();
            nm.exchange_folder(&m, &folder).unwrap();
            let mut nv = Net::load(&v).unwrap();
            nv.exchange_folder(&v, &folder).unwrap();
            nv.absorb(&v).unwrap();
        };
        let msg = |conv: &str,
                   peers: Vec<String>,
                   officine: &str,
                   files: Vec<crate::messages::FileMeta>| {
            serde_json::to_vec(&crate::messages::Incoming {
                uid: format!("u-{conv}"),
                conversation: conv.to_owned(),
                title: "t".to_owned(),
                peers,
                officine: officine.to_owned(),
                author: "XX".to_owned(),
                body: "texte".to_owned(),
                sent_at: "2026-09-24 12:00:00".to_owned(),
                files,
            })
            .unwrap()
        };
        // 1 — Une conversation dont M s'absente : refusée.
        send(&msg(
            "sans-m",
            vec![iv.clone(), "cafe".repeat(16)],
            "Centre",
            vec![],
        ));
        assert!(v.conversations().unwrap().is_empty(), "M s'absente : rien");
        // 2 — Un nom d'officine usurpé, vide qui plus est : le message est
        // rangé sous le nom que V connaît à M, et n'est pas à republier.
        let big = crate::messages::FileMeta {
            uid: "gros".to_owned(),
            message: "u-avec-m".to_owned(),
            name: "x".to_owned(),
            size: i64::MAX,
            hash: String::new(),
            chunks: 1,
        };
        send(&msg("avec-m", vec![iv.clone(), im.clone()], "", vec![big]));
        let convs = v.conversations().unwrap();
        assert_eq!(convs.len(), 1);
        let got = v.conversation_messages(convs[0].id).unwrap();
        assert_eq!(got[0].source, "Officine M", "le nom que V lui connaît");
        assert!(
            v.outgoing_net_messages().unwrap().is_empty(),
            "jamais republié par V"
        );
        assert!(
            v.conversation_files(convs[0].id).unwrap().is_empty(),
            "une taille démesurée n'est pas rangée"
        );
        // 3 — Un morceau pour un fichier inconnu : ignoré.
        send(
            &serde_json::to_vec(&crate::messages::Chunk {
                file: "gros".to_owned(),
                n: 0,
                d: "00".to_owned(),
            })
            .unwrap(),
        );
        assert!(v.file_chunks("gros").unwrap().is_empty());
    }

    /// **Une officine dans deux réseaux.** A est du principal avec B, et
    /// d'un second réseau avec C (où les ruptures ne sont pas partagées) —
    /// réseau qui est le principal de C. Un dossier d'échange commun aux
    /// trois. La rupture de A va à B, pas à C ; un message pour C passe par
    /// le second réseau ; un message pour B et C passe par les deux.
    #[test]
    fn an_officine_in_two_networks_shares_what_each_allows() {
        use crate::messages::Channel;
        let (dir_a, _sa, a) = officine("multi-a");
        let (_db, _sb, b) = officine("multi-b");
        let (_dc, _sc, c) = officine("multi-c");
        let folder = dir_a.join("echange");
        // Le principal de A, partagé avec B.
        Net::create(&a).unwrap();
        b.net_key("net_trousseau", &a.setting("net_trousseau").unwrap())
            .unwrap();
        let id = |d: &Db| hex(&Net::load(d).unwrap().device.id().0);
        let (ia, ib, ic) = (id(&a), id(&b), id(&c));
        a.add_net_peer(&ib, "", "2026-09-24").unwrap();
        b.add_net_peer(&ia, "", "2026-09-24").unwrap();
        // Le second réseau de A, principal de C, sans les ruptures.
        let n2 = Net::create_network(&a, "Garde du secteur", "2026-09-24").unwrap();
        let secret = a.net_networks().unwrap()[0].secret.clone();
        c.net_key("net_trousseau", &secret).unwrap();
        a.add_net_peer(&ic, "", "2026-09-24").unwrap();
        a.add_net_membership(&n2, &ic, false).unwrap();
        c.add_net_peer(&ia, "", "2026-09-24").unwrap();
        a.set_net_network(
            &n2,
            "Garde du secteur",
            "",
            crate::db::NetShares {
                ruptures: false,
                ..Default::default()
            },
            ("Garde du secteur", crate::db::NetShares::default()),
        )
        .unwrap();
        // Contre ce que l'écran montrait : une valeur périmée n'écrit rien.
        assert!(!a
            .set_net_network(
                &n2,
                "Autre nom",
                "",
                crate::db::NetShares::default(),
                ("Garde du secteur", crate::db::NetShares::default()),
            )
            .unwrap());
        // C est au second réseau de A, pas au principal ; B l'inverse.
        let members = |n: &str| -> Vec<String> {
            a.net_members(n)
                .unwrap()
                .into_iter()
                .map(|p| p.device)
                .collect()
        };
        assert_eq!(members(""), vec![ib.clone()]);
        assert_eq!(members(&n2), vec![ic.clone()]);
        let round = |d: &Db, name: &str| {
            for mut n in Net::load_all(d).unwrap() {
                n.publish(d, name).unwrap();
                n.exchange_folder(d, &folder).unwrap();
            }
        };
        let take = |d: &Db| {
            for mut n in Net::load_all(d).unwrap() {
                n.exchange_folder(d, &folder).unwrap();
                n.absorb(d).unwrap();
            }
        };
        // Un tour pour les clés de boîte.
        for (d, name) in [(&a, "A"), (&b, "B"), (&c, "C")] {
            round(d, name);
        }
        for d in [&a, &b, &c] {
            take(d);
        }
        a.add_supply_event(&subst("Diprosone", "Locoid")).unwrap();
        let to_c = a
            .create_conversation(
                "",
                Channel::Officines,
                "",
                &[],
                None,
                std::slice::from_ref(&ic),
                "CL",
            )
            .unwrap();
        a.post_message(
            to_c,
            "",
            "CL",
            "Pour C seule",
            None,
            "",
            Some("2026-09-24 10:00:00"),
        )
        .unwrap();
        let to_both = a
            .create_conversation(
                "",
                Channel::Officines,
                "Tous",
                &[],
                None,
                &[ib.clone(), ic.clone()],
                "CL",
            )
            .unwrap();
        a.post_message(
            to_both,
            "",
            "CL",
            "Pour B et C",
            None,
            "",
            Some("2026-09-24 10:01:00"),
        )
        .unwrap();
        round(&a, "A");
        take(&b);
        take(&c);
        assert_eq!(b.supply_events().unwrap().len(), 1, "B reçoit la rupture");
        assert!(
            c.supply_events().unwrap().is_empty(),
            "le second réseau ne la partage pas"
        );
        let bodies = |d: &Db| -> Vec<String> {
            d.conversations()
                .unwrap()
                .iter()
                .flat_map(|cv| d.conversation_messages(cv.id).unwrap())
                .map(|m| m.body)
                .collect()
        };
        let mut got_c = bodies(&c);
        got_c.sort();
        assert_eq!(got_c, ["Pour B et C", "Pour C seule"]);
        assert_eq!(
            bodies(&b),
            ["Pour B et C"],
            "B ne lit pas ce qui est pour C"
        );
        // B, déjà du principal, rejoint aussi le second réseau : il reste
        // du principal. Quitter le second réseau n'y touche pas non plus.
        a.add_net_membership(&n2, &ib, true).unwrap();
        assert!(members("").contains(&ib) && members(&n2).contains(&ib));
        // Retirée du principal seul, B reste du second réseau.
        a.remove_net_membership("", &ib).unwrap();
        assert!(!members("").contains(&ib) && members(&n2).contains(&ib));
        a.add_net_membership("", &ib, false).unwrap();
        // **Un compte par réseau** : une officine de deux réseaux ne semble
        // pas écrire à chaque synchronisation.
        a.note_net_heard_in(&n2, &ib, "B", 5).unwrap();
        a.note_net_heard(&ib, "B", 50).unwrap();
        let heard = |d: &Db| {
            d.net_peers()
                .unwrap()
                .into_iter()
                .find(|p| p.device == ib)
                .unwrap()
                .last_heard
        };
        a.conn_exec_for_tests("UPDATE net_peers SET last_heard = 'avant'")
            .unwrap();
        a.note_net_heard_in(&n2, &ib, "B", 5).unwrap();
        a.note_net_heard(&ib, "B", 50).unwrap();
        assert_eq!(heard(&a), "avant", "rien de neuf dans aucun des deux");
        // Retirée du second réseau seul, B reste du principal.
        a.remove_net_membership(&n2, &ib).unwrap();
        assert!(members("").contains(&ib) && !members(&n2).contains(&ib));
        a.add_net_membership(&n2, &ib, true).unwrap();
        a.leave_net_network(&n2).unwrap();
        assert_eq!(members(""), vec![ib.clone()]);
        assert!(a.net_networks().unwrap().is_empty());
        assert!(
            a.leave_net_network("").is_err(),
            "le principal ne se quitte pas ainsi"
        );
    }

    #[test]
    fn two_officines_share_a_substitution_through_a_folder_and_nothing_readable_crosses() {
        let (dir_a, _sa, a) = officine("a");
        let (_dir_b, _sb, b) = officine("b");
        let folder = dir_a.join("echange");
        // A crée le réseau ; B le rejoint — l'appairage réel passe par un
        // lien et un code, ici on pose la clé et les empreintes à la main.
        Net::create(&a).unwrap();
        let key = a.setting("net_trousseau").unwrap();
        b.net_key("net_trousseau", &key).unwrap();
        let na = Net::load(&a).unwrap();
        let nb = Net::load(&b).unwrap();
        a.add_net_peer(&hex(&nb.device.id().0), "", "2026-09-20")
            .unwrap();
        b.add_net_peer(&hex(&na.device.id().0), "", "2026-09-20")
            .unwrap();
        a.add_supply_event(&subst("Diprosone", "Locoid")).unwrap();
        // Et une valeur sourcée : elle voyage aussi.
        let eliquis = a.add_drug("Eliquis").unwrap();
        let bound = crate::pk::parse(
            crate::pk::Property::ProteinBinding,
            "87",
            "RCP Eliquis, 5.2",
        )
        .unwrap();
        a.set_drug_fact(eliquis, &bound, None).unwrap();

        // Et une fiche modifiée : sa version voyage, et s'applique chez B,
        // qui a la même fiche.
        let b_eliquis = b.add_drug("Eliquis").unwrap();
        let before = a
            .drugs()
            .unwrap()
            .into_iter()
            .find(|d| d.id == eliquis)
            .unwrap();
        let after = crate::db::Drug {
            dosage: "5 mg deux fois par jour".to_owned(),
            ..before.clone()
        };
        a.update_drug_by(&after, &before, "CL", false).unwrap();

        let mut na = Net::load(&a).unwrap();
        // Trois enregistrements, et l'annonce de la clé de boîte.
        assert_eq!(na.publish(&a, "Pharmacie du Centre").unwrap(), 4);
        assert_eq!(
            na.publish(&a, "Pharmacie du Centre").unwrap(),
            0,
            "une seule fois"
        );
        na.exchange_folder(&a, &folder).unwrap();

        // Rien de lisible dans le dossier.
        for entry in std::fs::read_dir(&folder).unwrap().flatten() {
            let bytes = std::fs::read(entry.path()).unwrap();
            let text = String::from_utf8_lossy(&bytes);
            assert!(!text.contains("Diprosone") && !text.contains("Centre"));
        }

        let mut nb = Net::load(&b).unwrap();
        assert_eq!(nb.exchange_folder(&b, &folder).unwrap(), 4);
        assert_eq!(nb.absorb(&b).unwrap(), 1);
        let tried = crate::ruptures::tried(&b.supply_events().unwrap(), "Diprosone");
        assert_eq!(tried.len(), 1);
        assert_eq!(tried[0].other, "Locoid");
        let theirs = b.supply_events().unwrap();
        assert_eq!(theirs[0].source, "Pharmacie du Centre");
        let card = b
            .drugs()
            .unwrap()
            .into_iter()
            .find(|d| d.id == b_eliquis)
            .unwrap();
        assert_eq!(
            card.dosage, "5 mg deux fois par jour",
            "la version s'est appliquée"
        );
        let facts = b.net_facts_for("eliquis").unwrap();
        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].officine, "Pharmacie du Centre");
        assert_eq!(facts[0].fact.low, Some(87.0));
        // Relu : rien de neuf.
        assert_eq!(nb.absorb(&b).unwrap(), 0);
        // **Et B sait qui lui a écrit, combien, et depuis quand** : le nom
        // sous lequel A signe, ses quatre enregistrements (la clé de
        // boîte comprise), des nouvelles datées — arrivées par le dossier,
        // sans conversation directe.
        let peer = &b.net_peers().unwrap()[0];
        assert_eq!(peer.seen_as, "Pharmacie du Centre");
        assert_eq!(peer.received, 4);
        assert!(!peer.last_heard.is_empty(), "des nouvelles datées");
        assert!(
            peer.last_ok.is_empty() && peer.last_try.is_empty(),
            "jamais jointe"
        );

        // Retirée, A n'est plus lue — ce qu'elle a déjà envoyé reste.
        b.remove_net_peer(&hex(&Net::load(&a).unwrap().device.id().0))
            .unwrap();
        a.add_supply_event(&subst("Diprosone", "Nérisone")).unwrap();
        let mut na = Net::load(&a).unwrap();
        na.publish(&a, "Pharmacie du Centre").unwrap();
        na.exchange_folder(&a, &folder).unwrap();
        let mut nb = Net::load(&b).unwrap();
        nb.exchange_folder(&b, &folder).unwrap();
        nb.absorb(&b).unwrap();
        assert_eq!(
            crate::ruptures::tried(&b.supply_events().unwrap(), "Diprosone").len(),
            1,
            "une officine retirée n'est plus lue"
        );
    }

    /// **Le codex et les protocoles voyagent aussi**, par le même chemin
    /// et sous les mêmes règles que les fiches : une préparation créée
    /// chez A se crée chez B avec sa formule, un protocole avec son arbre
    /// entier ; une formule corrigée des deux côtés attend chez chacune.
    #[test]
    fn codex_entries_and_protocols_travel_with_their_versions() {
        let (dir_a, _sa, a) = officine("codex-a");
        let (_dir_b, _sb, b) = officine("codex-b");
        let folder = dir_a.join("echange");
        Net::create(&a).unwrap();
        let key = a.setting("net_trousseau").unwrap();
        b.net_key("net_trousseau", &key).unwrap();
        let (na, nb) = (Net::load(&a).unwrap(), Net::load(&b).unwrap());
        a.add_net_peer(&hex(&nb.device.id().0), "", "2026-09-23")
            .unwrap();
        b.add_net_peer(&hex(&na.device.id().0), "", "2026-09-23")
            .unwrap();

        let id = a.add_preparation("Pommade maison").unwrap();
        let before = a
            .preparations()
            .unwrap()
            .into_iter()
            .find(|p| p.id == id)
            .unwrap();
        let after = crate::db::Preparation {
            formula: "vaseline | qsp 100 g".to_owned(),
            form: "pommade".to_owned(),
            ..before.clone()
        };
        assert!(a.update_preparation(&after, &before).unwrap());
        let proto = a.add_protocol("Toux de l'enfant", "toux").unwrap();
        let q = a
            .add_protocol_node(
                proto,
                None,
                crate::db::Branch::Root,
                crate::db::NodeKind::Question,
                "Fièvre ?",
            )
            .unwrap();
        a.add_protocol_node(
            proto,
            Some(q),
            crate::db::Branch::Yes,
            crate::db::NodeKind::Action,
            "Orienter",
        )
        .unwrap();

        let ship = |from: &Db, to: &Db| {
            let mut n = Net::load(from).unwrap();
            n.publish(from, "Pharmacie").unwrap();
            n.exchange_folder(from, &folder).unwrap();
            let mut m = Net::load(to).unwrap();
            m.exchange_folder(to, &folder).unwrap();
            m.absorb(to).unwrap();
        };
        ship(&a, &b);
        let got = b
            .preparations()
            .unwrap()
            .into_iter()
            .find(|p| p.name == "Pommade maison")
            .expect("créée chez B");
        assert_eq!(got.formula, "vaseline | qsp 100 g");
        assert_eq!(got.form, "pommade");
        let bp = b
            .protocols()
            .unwrap()
            .into_iter()
            .find(|p| p.title == "Toux de l'enfant")
            .expect("protocole créé chez B");
        assert_eq!(bp.subject, "toux");
        assert_eq!(
            b.protocol_tree(bp.id).unwrap(),
            a.protocol_tree(proto).unwrap()
        );

        // Both correct the formula without seeing each other.
        let set = |db: &Db, formula: &str| {
            let p = db
                .preparations()
                .unwrap()
                .into_iter()
                .find(|p| p.name == "Pommade maison")
                .unwrap();
            let new = crate::db::Preparation {
                formula: formula.to_owned(),
                ..p.clone()
            };
            assert!(db.update_preparation(&new, &p).unwrap());
        };
        set(&a, "vaseline | qsp 50 g");
        set(&b, "vaseline | qsp 200 g");
        ship(&a, &b);
        ship(&b, &a);
        let pending_on = |db: &Db, mine: &str| {
            let edits = db.card_edits().unwrap();
            let local = |f: &str| (f == "formula").then(|| mine.to_owned());
            crate::versions::pending_of(
                &edits,
                crate::versions::Kind::Codex,
                "Pommade maison",
                &local,
            )
            .len()
        };
        assert_eq!(
            pending_on(&a, "vaseline | qsp 50 g"),
            1,
            "rien d'écrasé chez A"
        );
        assert_eq!(
            pending_on(&b, "vaseline | qsp 200 g"),
            1,
            "rien d'écrasé chez B"
        );
        let formula = |db: &Db| {
            db.preparations()
                .unwrap()
                .into_iter()
                .find(|p| p.name == "Pommade maison")
                .unwrap()
                .formula
        };
        assert_eq!(formula(&a), "vaseline | qsp 50 g");
        // A adopts B's: written here, and a version of it travels.
        let theirs = a
            .card_edits()
            .unwrap()
            .into_iter()
            .rfind(|e| e.kind == crate::versions::Kind::Codex && !e.source.is_empty())
            .unwrap();
        assert!(a
            .set_entry_field(
                crate::versions::Kind::Codex,
                "Pommade maison",
                "formula",
                &theirs.value,
                "vaseline | qsp 50 g",
                "CL",
                false,
            )
            .unwrap());
        assert_eq!(formula(&a), "vaseline | qsp 200 g");
        assert_eq!(pending_on(&a, "vaseline | qsp 200 g"), 0);
        // Reverting the protocol tree on B goes back to A.
        let bp_tree = b.protocol_tree(bp.id).unwrap();
        assert!(b
            .set_entry_field(
                crate::versions::Kind::Protocole,
                "Toux de l'enfant",
                "arbre",
                "[]",
                &bp_tree,
                "CL",
                true,
            )
            .unwrap());
        ship(&b, &a);
        assert_eq!(a.protocol_tree(proto).unwrap(), "[]");
    }

    /// Une invitation de A dans `network`, que B rejoint : les deux fils,
    /// les codes acceptés des deux côtés. Rend ce que chacun a dit.
    fn pair(
        dir_a: &std::path::Path,
        dir_b: &std::path::Path,
        network: &str,
    ) -> (Option<Progress>, Option<Progress>) {
        let port = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let (yes_a, answers_a) = std::sync::mpsc::channel();
        let (yes_b, answers_b) = std::sync::mpsc::channel();
        let inviting = spawn(
            Job::Invite {
                port,
                network: network.to_owned(),
                nearby: true,
            },
            dir_a.join("net.db"),
            "secret".to_owned(),
            "A".to_owned(),
            "2026-09-24".to_owned(),
            answers_a,
        );
        match inviting.recv().unwrap() {
            Progress::Waiting(_) => {}
            other => return (Some(other), None),
        }
        let joining = spawn(
            Job::Join {
                address: format!("127.0.0.1:{port}"),
                ticket: None,
            },
            dir_b.join("net.db"),
            "secret".to_owned(),
            "B".to_owned(),
            "2026-09-24".to_owned(),
            answers_b,
        );
        let (mut done_a, mut done_b) = (None, None);
        while done_a.is_none() || done_b.is_none() {
            if done_a.is_none() {
                if let Ok(p) = inviting.recv_timeout(Duration::from_millis(50)) {
                    match p {
                        Progress::Code(_) => yes_a.send(true).unwrap(),
                        other => done_a = Some(other),
                    }
                }
            }
            if done_b.is_none() {
                if let Ok(p) = joining.recv_timeout(Duration::from_millis(50)) {
                    match p {
                        Progress::Code(_) => yes_b.send(true).unwrap(),
                        other => done_b = Some(other),
                    }
                }
            }
        }
        (done_a, done_b)
    }

    /// **Rejoindre un second réseau pour de vrai** : B a déjà son principal
    /// (avec ses enregistrements) ; il rejoint celui de A comme réseau de
    /// plus. Son principal ne bouge pas, rien de son principal ne passe à
    /// A, et rejoindre une seconde fois ne double rien.
    #[test]
    fn joining_a_second_network_keeps_the_first_to_itself() {
        let (dir_a, _sa, a) = officine("second-a");
        let (dir_b, _sb, b) = officine("second-b");
        Net::create(&a).unwrap();
        Net::create(&b).unwrap();
        let b_key = b.setting("net_trousseau").unwrap();
        b.add_supply_event(&subst("Diprosone", "Locoid")).unwrap();
        Net::load(&b).unwrap().publish(&b, "B").unwrap();
        let b_records = b.net_records().unwrap().len();
        assert!(b_records > 0);
        drop(a);
        drop(b);
        let (done_a, done_b) = pair(&dir_a, &dir_b, "");
        assert!(matches!(done_a, Some(Progress::Done(_))), "{done_a:?}");
        assert!(matches!(done_b, Some(Progress::Done(_))), "{done_b:?}");
        let a = Db::open(&dir_a.join("net.db"), "secret").unwrap();
        let b = Db::open(&dir_b.join("net.db"), "secret").unwrap();
        assert_eq!(
            b.setting("net_trousseau").unwrap(),
            b_key,
            "le principal de B ne bouge pas"
        );
        let second = b.net_networks().unwrap();
        assert_eq!(second.len(), 1, "le réseau de A est un réseau de plus");
        assert_eq!(second[0].secret, a.setting("net_trousseau").unwrap());
        assert_eq!(
            b.net_records().unwrap().len(),
            b_records,
            "principal intact"
        );
        // Rien du principal de B n'est chez A.
        let b_device = Net::load(&b).unwrap().device.id();
        let from_b = a
            .net_records()
            .unwrap()
            .iter()
            .filter_map(|bytes| Record::decode(bytes).ok())
            .filter(|r| r.author() == b_device)
            .count();
        assert_eq!(from_b, 0, "aucun enregistrement du principal de B chez A");
        // A voit B membre de son principal.
        assert_eq!(a.net_members("").unwrap().len(), 1);
        drop(a);
        drop(b);
        let (_, again) = pair(&dir_a, &dir_b, "");
        let b = Db::open(&dir_b.join("net.db"), "secret").unwrap();
        assert!(matches!(again, Some(Progress::Done(_))), "{again:?}");
        assert_eq!(b.net_networks().unwrap().len(), 1, "pas de doublon");
    }

    /// **L'appairage réel** : une porte, un lien, le code des deux côtés —
    /// et la clé du réseau passe, avec ce qui était déjà au journal.
    #[test]
    fn an_officine_joins_through_a_door_and_receives_what_the_network_knew() {
        let (dir_a, _sa, a) = officine("door-a");
        let (dir_b, _sb, b) = officine("door-b");
        Net::create(&a).unwrap();
        a.add_supply_event(&subst("Diprosone", "Locoid")).unwrap();
        Net::load(&a)
            .unwrap()
            .publish(&a, "Pharmacie du Centre")
            .unwrap();
        drop(a);
        drop(b);
        // A invite sur un port libre ; B compose.
        let port = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let (yes_a, answers_a) = std::sync::mpsc::channel();
        let (yes_b, answers_b) = std::sync::mpsc::channel();
        let inviting = spawn(
            Job::Invite {
                port,
                network: String::new(),
                nearby: true,
            },
            dir_a.join("net.db"),
            "secret".to_owned(),
            "Pharmacie du Centre".to_owned(),
            "2026-09-20".to_owned(),
            answers_a,
        );
        // Attendre que la porte soit ouverte.
        let mut codes = Vec::new();
        match inviting.recv().unwrap() {
            Progress::Waiting(_) => {}
            other => panic!("{other:?}"),
        }
        let joining = spawn(
            Job::Join {
                address: format!("127.0.0.1:{port}"),
                ticket: None,
            },
            dir_b.join("net.db"),
            "secret".to_owned(),
            "Pharmacie du Port".to_owned(),
            "2026-09-20".to_owned(),
            answers_b,
        );
        let (mut done_a, mut done_b) = (None, None);
        while done_a.is_none() || done_b.is_none() {
            if done_a.is_none() {
                if let Ok(p) = inviting.recv_timeout(Duration::from_millis(50)) {
                    match p {
                        Progress::Code(c) => {
                            codes.push(c);
                            yes_a.send(true).unwrap();
                        }
                        other => done_a = Some(other),
                    }
                }
            }
            if done_b.is_none() {
                if let Ok(p) = joining.recv_timeout(Duration::from_millis(50)) {
                    match p {
                        Progress::Code(c) => {
                            codes.push(c);
                            yes_b.send(true).unwrap();
                        }
                        other => done_b = Some(other),
                    }
                }
            }
        }
        assert!(matches!(done_a, Some(Progress::Done(_))), "{done_a:?}");
        assert!(matches!(done_b, Some(Progress::Done(_))), "{done_b:?}");
        assert_eq!(codes.len(), 2);
        assert_eq!(codes[0], codes[1], "le même code des deux côtés");
        let b = Db::open(&dir_b.join("net.db"), "secret").unwrap();
        assert!(Net::load(&b).unwrap().in_network());
        assert_eq!(b.net_peers().unwrap().len(), 1);
        let tried = crate::ruptures::tried(&b.supply_events().unwrap(), "Diprosone");
        assert_eq!(tried.len(), 1, "ce que le réseau savait est arrivé");
    }

    #[test]
    fn a_key_reads_back_and_a_bad_one_does_not() {
        let k = random32();
        assert_eq!(unhex::<32>(&hex(&k)), Some(k));
        assert_eq!(unhex::<32>("zz"), None);
        assert_eq!(unhex::<32>(&"0".repeat(63)), None);
    }

    /// **Avec le code d'invitation, rien à comparer** : B colle le code
    /// que A affiche, aucun des deux fils ne montre de code, et la clé du
    /// réseau passe. Le même code recopié avec une faute ne rejoint rien.
    #[test]
    fn an_officine_joins_with_the_invitation_code_and_compares_nothing() {
        for spoil in [false, true] {
            let tag = if spoil { "ticket-bad" } else { "ticket" };
            let (dir_a, _sa, a) = officine(&format!("{tag}-a"));
            let (dir_b, _sb, b) = officine(&format!("{tag}-b"));
            Net::create(&a).unwrap();
            drop(a);
            drop(b);
            let port = std::net::TcpListener::bind("127.0.0.1:0")
                .unwrap()
                .local_addr()
                .unwrap()
                .port();
            let (_yes_a, answers_a) = std::sync::mpsc::channel();
            let (_yes_b, answers_b) = std::sync::mpsc::channel();
            let inviting = spawn(
                Job::Invite {
                    port,
                    network: String::new(),
                    nearby: false,
                },
                dir_a.join("net.db"),
                "secret".to_owned(),
                "A".to_owned(),
                "2026-09-24".to_owned(),
                answers_a,
            );
            let Progress::Waiting(code) = inviting.recv().unwrap() else {
                panic!("porte fermée");
            };
            let (_, ticket) = read_join(&code).expect("un code");
            let mut ticket = ticket.expect("un ticket");
            if spoil {
                let mut b = ticket.bytes();
                b[0] ^= 1;
                ticket = bpm_sync::Ticket::from_bytes(b);
            }
            let joining = spawn(
                Job::Join {
                    address: format!("127.0.0.1:{port}"),
                    ticket: Some(ticket),
                },
                dir_b.join("net.db"),
                "secret".to_owned(),
                "B".to_owned(),
                "2026-09-24".to_owned(),
                answers_b,
            );
            let wait = |rx: &std::sync::mpsc::Receiver<Progress>| loop {
                match rx.recv_timeout(Duration::from_secs(20)).expect("une fin") {
                    Progress::Code(_) => panic!("aucun code à comparer"),
                    Progress::Waiting(_) => {}
                    done => break done,
                }
            };
            let (done_a, done_b) = (wait(&inviting), wait(&joining));
            let b = Db::open(&dir_b.join("net.db"), "secret").unwrap();
            if spoil {
                assert!(matches!(done_a, Progress::Failed(_)), "{done_a:?}");
                assert!(matches!(done_b, Progress::Failed(_)), "{done_b:?}");
                assert!(!Net::load(&b).unwrap().in_network(), "aucune clé");
            } else {
                assert!(matches!(done_a, Progress::Done(_)), "{done_a:?}");
                assert!(matches!(done_b, Progress::Done(_)), "{done_b:?}");
                assert!(Net::load(&b).unwrap().in_network());
                assert_eq!(b.net_peers().unwrap().len(), 1);
            }
        }
    }

    /// Ce que « Rejoindre » accepte : un code entier, ou une adresse seule.
    /// Un code abîmé n'est pas une adresse.
    #[test]
    fn the_join_field_reads_a_code_or_an_address() {
        let t = bpm_sync::Ticket::from_bytes([9; 10]);
        let code = invitation_code(&t, "192.168.1.20:7742");
        assert_eq!(
            read_join(&code),
            Some(("192.168.1.20:7742".to_owned(), Some(t)))
        );
        assert_eq!(
            read_join(&format!("  {}  ", code.to_lowercase())),
            Some(("192.168.1.20:7742".to_owned(), Some(t)))
        );
        assert_eq!(
            read_join("192.168.1.20:7742"),
            Some(("192.168.1.20:7742".to_owned(), None))
        );
        assert_eq!(read_join("K7QM-2XPA@192.168.1.20:7742"), None);
        assert_eq!(read_join(&format!("{}@", t.text())), None);
        assert_eq!(read_join("192.168.1.20 7742"), None);
        assert_eq!(read_join(""), None);
    }

    /// **Une annonce dit qui, où inviter, et comment elle s'appelle —
    /// rien de plus**, et ce qui n'en est pas une exactement est refusé :
    /// l'annonce vient de n'importe qui sur le réseau local.
    #[test]
    fn an_officine_beacon_names_itself_and_nothing_else() {
        let d = hex(&[7u8; 32]);
        let ip: std::net::IpAddr = "192.168.1.30".parse().unwrap();
        let b = officine_beacon(&d, "  Pharmacie de la Gare\n", "Épinal", 7742, 0);
        assert!(!b.contains("Gare"), "le nom voyage en hexadécimal : {b}");
        let n = heard_officine(&b, ip).unwrap();
        assert_eq!(n.device, d);
        assert_eq!(n.name, "Pharmacie de la Gare");
        assert_eq!(n.place, "Épinal");
        assert_eq!(n.invite, None);
        assert_eq!(n.listen.as_deref(), Some("192.168.1.30:7742"));
        let n = heard_officine(&officine_beacon(&d, "Épinal — Centre", "", 0, 7742), ip).unwrap();
        assert_eq!(n.name, "Épinal — Centre");
        assert_eq!(n.invite.as_deref(), Some("192.168.1.30:7742"));
        assert_eq!(n.listen, None);
        // La première version s'entend encore : ni ville ni écoute.
        let v1 = format!("BPMOFFICINE1 {d} 0 {}", hex(b"Pharmacie du Port"));
        let n = heard_officine(&v1, ip).unwrap();
        assert_eq!(
            (n.name.as_str(), n.place.as_str(), n.listen),
            ("Pharmacie du Port", "", None)
        );
        // Une identité s'écrit d'une seule façon : en majuscules, elle
        // revient en minuscules — la même que celle du réseau.
        let upper = format!("BPMOFFICINE1 {} 0 41", d.to_uppercase());
        assert_eq!(heard_officine(&upper, ip).unwrap().device, d);
        // Un nom trop long est coupé, pas refusé.
        let long = "x".repeat(200);
        let n = heard_officine(&officine_beacon(&d, &long, "", 0, 0), ip).unwrap();
        assert_eq!(n.name.chars().count(), NEARBY_NAME);
        for bad in [
            String::new(),
            "BPMOFFICINE1".to_owned(),
            format!("BPMOFFICINE1 {d} 0"),
            "BPMOFFICINE1 zz 0 41".to_owned(),
            format!("BPMOFFICINE1 {d} 70000 41"),
            format!("BPMOFFICINE1 {d} 0 4"),
            format!("BPMOFFICINE1 {d} 0 zz"),
            format!("BPMOFFICINE1 {d} 0 ff"),
            format!("BPMOFFICINE1 {d} 0 41 de-trop"),
            format!("BPMPOSTE1 {d} 7743"),
            format!("BPMOFFICINE1 {d} 0 {}", "41".repeat(NEARBY_NAME * 4 + 1)),
            format!("BPMOFFICINE2 {d} 0 0 41"),
            format!("BPMOFFICINE2 {d} x 0 41 41"),
            format!("BPMOFFICINE2 {d} 0 0 41 zz"),
            format!("BPMOFFICINE2 {d} 0 0 41 41 de-trop"),
        ] {
            assert_eq!(heard_officine(&bad, ip), None, "{bad}");
        }
    }

    /// **Une officine présentée par le réseau** : A a fait entrer B, B a
    /// fait entrer C. A ne connaît pas C, mais ce que C écrit lui arrive
    /// par le journal : la carte le propose sous le nom que C écrit, A ne
    /// lit rien de C tant qu'il ne l'a pas ajoutée, et une officine
    /// écartée n'est plus proposée.
    #[test]
    fn an_officine_let_in_by_another_member_is_introduced_not_trusted() {
        let (dir_a, _sa, a) = officine("intro-a");
        let (dir_b, _sb, b) = officine("intro-b");
        let (dir_c, _sc, c) = officine("intro-c");
        Net::create(&a).unwrap();
        drop((a, b, c));
        let (da, db_) = pair(&dir_a, &dir_b, "");
        assert!(matches!(da, Some(Progress::Done(_))), "{da:?}");
        assert!(matches!(db_, Some(Progress::Done(_))), "{db_:?}");
        let (db2, dc) = pair(&dir_b, &dir_c, "");
        assert!(matches!(db2, Some(Progress::Done(_))), "{db2:?}");
        assert!(matches!(dc, Some(Progress::Done(_))), "{dc:?}");
        let a = Db::open(&dir_a.join("net.db"), "secret").unwrap();
        let c = Db::open(&dir_c.join("net.db"), "secret").unwrap();
        c.add_supply_event(&subst("Diprosone", "Locoid")).unwrap();
        let folder = dir_c.join("echange");
        std::fs::create_dir_all(&folder).unwrap();
        let mut net_c = Net::load(&c).unwrap();
        net_c.publish(&c, "Pharmacie du Canal").unwrap();
        net_c.exchange_folder(&c, &folder).unwrap();
        let mut net_a = Net::load(&a).unwrap();
        net_a.exchange_folder(&a, &folder).unwrap();
        net_a.absorb(&a).unwrap();
        let c_device = hex(&net_c.device.id().0);
        let intro = net_a.introduced(&ignored(&a));
        assert_eq!(intro.len(), 1, "{intro:?}");
        assert_eq!(intro[0].device, c_device);
        assert_eq!(intro[0].name, "Pharmacie du Canal");
        assert!(
            crate::ruptures::tried(&a.supply_events().unwrap(), "Diprosone").is_empty(),
            "rien de C n'entre avant qu'on l'ajoute"
        );
        // Écartée, elle n'est plus proposée ; reprise, elle l'est.
        set_ignored(&a, "", &c_device, true, "2026-09-24").unwrap();
        assert!(net_a.introduced(&ignored(&a)).is_empty());
        set_ignored(&a, "", &c_device, false, "2026-09-24").unwrap();
        assert_eq!(net_a.introduced(&ignored(&a)).len(), 1);
        // Ajoutée : ce qu'elle a envoyé entre, sous son nom.
        // Rien de C au journal ni dans la base : compté, pas gardé.
        assert!(!net_a
            .journal
            .records()
            .any(|r| hex(&r.author().0) == c_device));
        assert!(!a
            .net_records()
            .unwrap()
            .iter()
            .filter_map(|b| Record::decode(b).ok())
            .any(|r| hex(&r.author().0) == c_device));
        adopt(&a, "", &c_device, Some(&folder), "2026-09-24").unwrap();
        assert_eq!(
            crate::ruptures::tried(&a.supply_events().unwrap(), "Diprosone").len(),
            1
        );
        assert!(
            a.net_members("")
                .unwrap()
                .iter()
                .any(|p| p.device == c_device
                    && p.seen_as == "Pharmacie du Canal"
                    && p.name.is_empty()),
            "ajoutée sans prendre pour nom celui qu'elle écrit"
        );
        assert!(Net::load(&a).unwrap().introduced(&ignored(&a)).is_empty());
        assert!(adopt(&a, "", "pas une clé", None, "2026-09-24").is_err());
    }

    /// **Un inconnu qui détient la clé ne touche pas au journal** (revue de
    /// sécurité de 0.318.0) : un enregistrement qui en « corrige » un d'une
    /// officine appairée ne le fait pas disparaître, et un dossier inondé
    /// d'identités jetables n'en propose que [`MOST_STRANGERS`].
    #[test]
    fn a_key_holder_nobody_added_neither_hides_records_nor_floods_the_map() {
        let (dir_a, _sa, a) = officine("quarantine-a");
        let (dir_b, _sb, b) = officine("quarantine-b");
        Net::create(&a).unwrap();
        drop((a, b));
        let (da, db_) = pair(&dir_a, &dir_b, "");
        assert!(matches!(da, Some(Progress::Done(_))), "{da:?}");
        assert!(matches!(db_, Some(Progress::Done(_))), "{db_:?}");
        let a = Db::open(&dir_a.join("net.db"), "secret").unwrap();
        let b = Db::open(&dir_b.join("net.db"), "secret").unwrap();
        let folder = dir_a.join("echange");
        std::fs::create_dir_all(&folder).unwrap();
        // B signale une substitution et la dépose.
        b.add_supply_event(&subst("Diprosone", "Locoid")).unwrap();
        let mut net_b = Net::load(&b).unwrap();
        net_b.publish(&b, "Pharmacie B").unwrap();
        net_b.exchange_folder(&b, &folder).unwrap();
        let target = net_b
            .journal
            .records()
            .find(|r| r.author() == net_b.device.id())
            .map(|r| r.id())
            .expect("un enregistrement de B");
        // Quelqu'un qui détient la clé — une officine retirée — « corrige »
        // cet enregistrement, et inonde le dossier d'identités jetables.
        let trousseau = net_b.trousseau.clone().unwrap();
        let mut rogue = Journal::new();
        let mut framed = Vec::new();
        for n in 0..40u8 {
            let d = Device::from_seed([n.wrapping_add(100); 32]);
            let corrects = (n == 0).then_some(target);
            rogue
                .write(
                    &d,
                    &trousseau,
                    Stream::Reseau,
                    br#"{"t":"boite","officine":"Pharmacie B"}"#,
                    corrects,
                    &mut bpm_sync::OsEntropy,
                )
                .unwrap();
        }
        for r in rogue.records() {
            let bytes = r.encode();
            framed.extend((bytes.len() as u32).to_be_bytes());
            framed.extend(bytes);
        }
        std::fs::write(folder.join("rogue.bpmnet"), framed).unwrap();
        let mut net_a = Net::load(&a).unwrap();
        net_a.exchange_folder(&a, &folder).unwrap();
        net_a.absorb(&a).unwrap();
        assert_eq!(
            crate::ruptures::tried(&a.supply_events().unwrap(), "Diprosone").len(),
            1,
            "ce que B a écrit est lu, malgré la « correction »"
        );
        let intro = net_a.introduced(&[]);
        assert_eq!(intro.len(), MOST_STRANGERS, "plafonné");
        assert!(net_a
            .journal
            .records()
            .all(|r| r.author() == net_a.device.id() || r.author() == net_b.device.id()));
        // Et gardés dans la base — relayés par une conversation, ou par
        // 0.318.0 —, ils ne corrigent rien à la lecture.
        a.keep_net_records_of(
            "",
            &rogue
                .records()
                .map(|r| (hex(&r.id().0), r.encode()))
                .collect::<Vec<_>>(),
        )
        .unwrap();
        let again = Net::load(&a).unwrap();
        let t = again.trousseau.clone().unwrap();
        let read = again.trusted_view().read(&t, Stream::Reseau);
        assert!(
            read.facts.iter().any(|f| f.id == target),
            "gardée dans la base, la « correction » n'efface rien à la lecture"
        );
        assert!(read
            .facts
            .iter()
            .all(|f| f.author == again.device.id() || f.author == net_b.device.id()));
    }

    #[test]
    fn a_town_is_the_last_part_of_an_address_without_its_postcode() {
        assert_eq!(town_of("12 rue des Lilas, 75011 Paris"), "Paris");
        assert_eq!(town_of("Place de la Mairie\n88000 Épinal\n"), "Épinal");
        assert_eq!(town_of("Saint-Dié-des-Vosges"), "Saint-Dié-des-Vosges");
        assert_eq!(
            town_of("3 place du Marché, 13001 Marseille Cedex 01"),
            "Marseille Cedex 01"
        );
        assert_eq!(town_of(""), "");
        assert_eq!(town_of("75011"), "");
    }

    /// **La connexion au lancement, de bout en bout** : A tient sa porte,
    /// B apprend où A écoute (comme par son annonce), compose, et ce que B
    /// signale arrive chez A — sans rien saisir. Une officine que A n'a
    /// pas appairée n'obtient rien de la même porte.
    #[test]
    fn a_paired_officine_heard_on_the_network_connects_by_itself() {
        let (dir_a, _sa, a) = officine("listen-a");
        let (dir_b, _sb, b) = officine("listen-b");
        let (dir_c, _sc, c) = officine("listen-c");
        Net::create(&a).unwrap();
        Net::create(&c).unwrap();
        drop((a, b, c));
        let (da, db_) = pair(&dir_a, &dir_b, "");
        assert!(matches!(da, Some(Progress::Done(_))), "{da:?}");
        assert!(matches!(db_, Some(Progress::Done(_))), "{db_:?}");
        let a = Db::open(&dir_a.join("net.db"), "secret").unwrap();
        let b = Db::open(&dir_b.join("net.db"), "secret").unwrap();
        let c = Db::open(&dir_c.join("net.db"), "secret").unwrap();
        b.add_supply_event(&subst("Diprosone", "Locoid")).unwrap();
        let a_device = device_hex(&a).unwrap();
        let door = bpm_sync::link::Door::open("127.0.0.1:0").unwrap();
        let at = door.address().unwrap().to_string();
        let path_a = dir_a.join("net.db");
        let answering = std::thread::spawn(move || {
            let db = Db::open(&path_a, "secret").unwrap();
            let mut answered = Vec::new();
            for _ in 0..2 {
                if let Ok(mut link) = door.accept(Duration::from_secs(20)) {
                    answered.push(answer(&db, &mut link));
                }
            }
            answered
        });
        // Une adresse où personne ne répond : rien n'est gardé.
        let before = b
            .net_peers()
            .unwrap()
            .into_iter()
            .find(|p| p.device == a_device)
            .unwrap()
            .address;
        assert!(dial_heard(&b, &a_device, "127.0.0.1:9", "Pharmacie B").is_err());
        let stored = |db: &Db| {
            db.net_peers()
                .unwrap()
                .into_iter()
                .find(|p| p.device == a_device)
                .map(|p| p.address)
        };
        assert_eq!(stored(&b), Some(before), "l'adresse d'avant reste");
        // C n'a pas A parmi ses officines : rien à composer.
        assert!(dial_heard(&c, &a_device, &at, "Pharmacie C").is_err());
        // L'adresse annoncée, où A répond : jointe, et gardée.
        let said = dial_heard(&b, &a_device, &at, "Pharmacie B").expect("jointe");
        assert!(!said.is_empty());
        assert_eq!(
            stored(&b),
            Some(at.clone()),
            "gardée une fois qu'A y a répondu"
        );
        // C, d'un autre réseau, compose la même porte : refusée.
        let mut net_c = Net::load(&c).unwrap();
        assert!(net_c.sync_with(&c, &at, Duration::from_secs(8)).is_err());
        let answered = answering.join().unwrap();
        assert!(answered[0].is_ok(), "{answered:?}");
        assert!(answered.get(1).is_some_and(|r| r.is_err()), "{answered:?}");
        assert_eq!(
            crate::ruptures::tried(&a.supply_events().unwrap(), "Diprosone").len(),
            1,
            "ce que B signale est arrivé chez A"
        );
    }
}
