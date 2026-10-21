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
use bpm_sync::{
    Device, DeviceId, Hash, Intent, Journal, Meter, Record, Session, Stream, Trousseau,
};
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
    /// Ce que la dernière publication a laissé de côté (trop gros pour un
    /// enregistrement) : dit à la synchronisation, sans arrêter le reste.
    pub refused: Vec<String>,
    /// **Le réseau ouvert** (voir [`open_trousseau`]) : toute officine y
    /// parle, et rien de ce qui y arrive ne s'applique de soi-même.
    pub open: bool,
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
        let journal = stored_journal(
            &device,
            &peers,
            false,
            db.net_records_of("")?,
            db.net_forgotten_of("")?,
        );
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
            refused: Vec::new(),
            open: false,
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
        let open = is_open(id);
        let journal = stored_journal(
            &device,
            &peers,
            open,
            db.net_records_of(id)?,
            db.net_forgotten_of(id)?,
        );
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
            refused: Vec::new(),
            open,
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

    /// Créer un réseau de plus (la démonstration et les essais : plus aucun geste n'en crée) — pour un lien avec une seule officine, ou
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

    /// Créer un réseau (la démonstration et les essais : plus aucun geste n'en crée) : une clé neuve, que cette officine donnera à
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
        let Some(trousseau) = self.trousseau.clone() else {
            return Ok(0);
        };
        // **Un seul à la fois dans ce processus** : la synchronisation et
        // l'appel né d'une annonce publient chacun sur leur fil ; ensemble,
        // chacun scellait ce que l'autre n'avait pas encore marqué parti.
        static PUBLISHING: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _one = PUBLISHING
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        // Ce que l'autre vient d'écrire, sous le verrou : sans cela ce
        // journal-ci, lu avant d'attendre, écrivait après des parents
        // périmés et au même rang que le dernier enregistrement.
        for bytes in db.net_records_of(&self.id)? {
            if let Ok(record) = Record::decode(&bytes) {
                self.journal.insert(record);
            }
        }
        // Ce que **ce** réseau a déjà reçu d'ici, et ce que l'officine y
        // partage : un événement part une fois par réseau, et pas du tout
        // dans un réseau où son genre n'est pas partagé.
        // **Ce qui est scellé est marqué parti, même quand la suite échoue** :
        // sans cela, une erreur au milieu laissait au journal des
        // enregistrements jamais marqués, rescellés à chaque synchronisation.
        self.refused.clear();
        let mut done = Vec::new();
        let outcome = self.seal_pending(db, officine, &trousseau, &mut done);
        self.keep(db)?;
        let keys: Vec<String> = done.iter().map(|k| self.done_key(k)).collect();
        db.mark_published(&keys)?;
        self.forget_superseded(db, &trousseau)?;
        outcome.map(|()| done.len())
    }

    /// **Oublier les versions remplacées des comptes** : de chaque élément
    /// de compte qu'un même auteur du journal a passé plusieurs fois, ne
    /// garder que la dernière version (`compte::newer`) — un effacement
    /// compris, qui doit voyager. Les autres ne gardent que leur nom et
    /// leurs parents (`Journal::forget`) : elles ne comptent plus au budget
    /// de leur auteur, ne sont plus redemandées, et la base ne les tient
    /// plus. Sans cela le journal grandissait de chaque version, et au
    /// réseau ouvert une officine se taisait, pour tous, au premier
    /// mégaoctet de sa vie. Ce que le réseau dit d'autre garde son histoire,
    /// tant que son auteur a de la place (`make_room`). Rend combien sont
    /// partis.
    fn forget_superseded(&mut self, db: &Db, trousseau: &Trousseau) -> Result<usize, String> {
        let mut latest: std::collections::HashMap<
            (DeviceId, String, String),
            (Hash, crate::compte::Read),
        > = Default::default();
        let mut superseded = Vec::new();
        for r in self.journal.records() {
            if r.stream() != Stream::Reseau || r.corrects().is_some() {
                continue;
            }
            let Some(read) = r
                .open(trousseau)
                .ok()
                .filter(|p| p.starts_with(crate::compte::TAG))
                .and_then(|p| crate::compte::read(&p, None))
            else {
                continue;
            };
            let key = (r.author(), read.account.clone(), read.item.clone());
            match latest.get(&key) {
                Some((_, kept)) if !crate::compte::newer(&read, kept) => superseded.push(r.id()),
                Some((old, _)) => {
                    superseded.push(*old);
                    latest.insert(key, (r.id(), read));
                }
                None => {
                    latest.insert(key, (r.id(), read));
                }
            }
        }
        let mut gone: Vec<Hash> = superseded
            .into_iter()
            .filter(|h| self.journal.forget(h))
            .collect();
        gone.extend(self.make_room(db, trousseau)?);
        if gone.is_empty() {
            return Ok(0);
        }
        let stubs: Vec<[String; 3]> = self
            .journal
            .stubs()
            .filter(|(h, _, _)| gone.contains(h))
            .map(|(h, author, parents)| {
                let named: String = parents.iter().map(|p| hex(&p.0)).collect();
                [hex(&h.0), hex(&author.0), named]
            })
            .collect();
        db.forget_net_records_of(&self.id, &stubs)?;
        Ok(gone.len())
    }

    /// **Une officine reste entendue** : au réseau ouvert, un membre que
    /// cette officine lit et qui tient plus des trois quarts de son budget
    /// (`OPEN_LIMITS`) y voit oublier ses enregistrements les plus anciens,
    /// jusqu'à la moitié — ce qui peut être encore vrai : une version de
    /// fiche, une rupture en cours, la dernière valeur d'une propriété.
    /// Jamais un élément de compte vivant (les versions remplacées sont
    /// déjà parties), jamais sa clé de boîte, qui ne se dit qu'une fois,
    /// jamais un message ou un morceau scellés, que cette officine ne peut
    /// pas lire et passe pour d'autres ; et toujours ses plus récents, pour
    /// un quart du budget, même quand ses noms oubliés et ce qui est gardé
    /// en tiennent déjà la moitié. Sans cela, ses ruptures, ses versions et
    /// ses valeurs remplissaient le budget d'une vie, et tout ce qu'elle
    /// disait ensuite était refusé. Ce qui est oublié d'un membre lu a été
    /// lu : la base le garde (`supply_events`, `card_edits`, `net_facts`) ;
    /// seuls les pairs qui ne l'ont jamais tenu ne le recevront plus d'ici.
    /// Une officine qu'on ne lit pas (écartée, ou sans place de membre)
    /// n'y perd rien, et bute comme avant.
    fn make_room(&mut self, db: &Db, trousseau: &Trousseau) -> Result<Vec<Hash>, String> {
        if !self.open {
            return Ok(Vec::new());
        }
        let read: std::collections::HashSet<DeviceId> =
            self.read_members(db)?.into_iter().collect();
        let full: std::collections::BTreeSet<DeviceId> = self
            .journal
            .records()
            .map(Record::author)
            .filter(|a| read.contains(a))
            .filter(|a| {
                self.journal
                    .budget_of(a)
                    .is_some_and(|(held, most)| held > most / 4 * 3)
            })
            .collect();
        let kept = |p: &[u8]| {
            p.starts_with(crate::compte::TAG)
                || p.starts_with(crate::messages::BOX_TAG)
                || serde_json::from_slice::<serde_json::Value>(p)
                    .is_ok_and(|v| v.get("t").and_then(|t| t.as_str()) == Some("boite"))
        };
        let mut gone = Vec::new();
        for author in full {
            let mut theirs: Vec<(u64, Hash, usize)> = self
                .journal
                .records()
                .filter(|r| r.author() == author && r.stream() == Stream::Reseau)
                .filter(|r| r.open(trousseau).is_ok_and(|p| !kept(&p)))
                .map(|r| (r.lamport(), r.id(), r.encode().len()))
                .collect();
            theirs.sort();
            let mut left: usize = theirs.iter().map(|(_, _, n)| n).sum();
            for (_, id, size) in theirs {
                let Some((held, most)) = self.journal.budget_of(&author) else {
                    break;
                };
                if held <= most / 2 || left <= most / 4 {
                    break;
                }
                left -= size;
                if self.journal.forget(&id) {
                    gone.push(id);
                }
            }
        }
        Ok(gone)
    }

    /// Sceller un enregistrement du réseau ; un refus (trop gros) est
    /// gardé dans `refused` et n'arrête pas les suivants.
    fn seal_one(&mut self, trousseau: &Trousseau, payload: &[u8]) -> bool {
        match self.journal.write(
            &self.device,
            trousseau,
            Stream::Reseau,
            payload,
            None,
            &mut bpm_sync::OsEntropy,
        ) {
            Ok(_) => true,
            Err(e) => {
                self.refused.push(format!("{e:?}"));
                false
            }
        }
    }

    /// Ce qui n'est pas encore parti, scellé ; `done` reçoit chaque clé
    /// scellée au fur et à mesure.
    fn seal_pending(
        &mut self,
        db: &Db,
        officine: &str,
        trousseau: &Trousseau,
        done: &mut Vec<String>,
    ) -> Result<(), String> {
        let already = self.already(db)?;
        let pending: Vec<crate::ruptures::Event> = if self.shares.ruptures {
            db.supply_events()?
                .into_iter()
                .filter(|e| e.source.is_empty() && !already.contains(&e.uid))
                .collect()
        } else {
            Vec::new()
        };
        for e in &pending {
            // **Au réseau ouvert, ni note ni initiales** : une note libre
            // peut nommer un patient, et la clé y est publique.
            let stripped;
            let e = if self.open {
                stripped = crate::ruptures::Event {
                    note: String::new(),
                    operator: String::new(),
                    ..e.clone()
                };
                &stripped
            } else {
                e
            };
            let payload = crate::ruptures::encode(e, officine);
            if self.seal_one(trousseau, &payload) {
                done.push(e.uid.clone());
            }
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
                // Une liste de contrôle ne part que si l'officine le dit ;
                // gardée non partie, elle partira le jour où elle le dira.
                .filter(|e| !e.kind.is_private() || (self.shares.listes && !self.open))
                .map(|e| {
                    if self.open {
                        crate::versions::Edit {
                            operator: String::new(),
                            ..e
                        }
                    } else {
                        e
                    }
                })
                .collect()
        } else {
            Vec::new()
        };
        for e in versions {
            let payload = crate::versions::encode(&e, officine);
            if self.seal_one(trousseau, &payload) {
                done.push(e.uid);
            }
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
            if self.seal_one(trousseau, &payload) {
                done.push(key);
            }
        }
        // **L'adresse où l'officine se joint de l'extérieur**, quand elle
        // en déclare une : annoncée une fois par valeur, aux seuls membres
        // du réseau — c'est ce qui permet la connexion au lancement entre
        // officines de deux sites.
        let public = db.setting(PUBLIC_ADDRESS).unwrap_or_default();
        let public = public.trim();
        // Une fois par adresse **enregistrée** : revenir à une adresse
        // d'avant l'annonce de nouveau.
        let seq = db.setting(PUBLIC_ADDRESS_SEQ).unwrap_or_default();
        let key = format!("adresse:{seq}:{public}");
        if !public.is_empty() && !already.contains(&key) && !self.open {
            let payload = serde_json::json!({
                "t": "adresse",
                "a": public,
                "officine": officine,
            })
            .to_string()
            .into_bytes();
            if self.seal_one(trousseau, &payload) {
                done.push(key);
            }
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
            if self.seal_one(trousseau, &payload) {
                done.push(key);
            }
        }
        // **Les comptes** : la dernière version de chaque élément, telle
        // que son compte l'a signée — l'officine la passe sans la lire, et
        // ce qui est scellé ne s'ouvre que chez ses lecteurs. **Seulement
        // ceux de l'officine** — ouverts sur un de ses postes, téléphone
        // compris (`account_homes`) : ce qui arrive d'ailleurs circule déjà
        // par le journal qui l'a apporté, et le resceller ici sous le nom
        // de l'officine userait le mégaoctet que les autres lui accordent.
        // Au plus `compte::MOST_ITEMS` éléments par compte.
        let here = db.officine_accounts()?;
        let latest: Vec<_> = db
            .account_latest(None)?
            .into_iter()
            .filter(|(r, _)| here.contains(&r.account))
            .collect();
        for (r, bytes) in crate::compte::to_seed(latest) {
            let key = format!("acc:{}", r.name);
            if already.contains(&key) {
                continue;
            }
            if self.seal_one(trousseau, &bytes) {
                done.push(key);
            }
        }
        // **Le relais** (`relay.rs`) : l'officine qui relaie dit où, une
        // fois par valeur — une valeur vide aussi, qui retire l'offre ;
        // celle qui demande à être relayée dit le nom de son groupe de
        // postes, qu'un relais n'accepte que d'une officine du réseau.
        for (setting, t, field) in [
            (RELAY_OFFER, "relais", "a"),
            (RELAY_REQUEST, "relais_demande", "g"),
        ] {
            let value = db.setting(setting).unwrap_or_default();
            let value = value.trim();
            let seq = db.setting(&format!("{setting}_seq")).unwrap_or_default();
            let key = format!("{t}:{seq}:{value}");
            let said_before = already.iter().any(|k| k.starts_with(&format!("{t}:")));
            if self.open || already.contains(&key) || (value.is_empty() && !said_before) {
                continue;
            }
            let mut payload = serde_json::json!({ "t": t, "officine": officine });
            payload[field] = serde_json::Value::String(value.to_owned());
            if self.seal_one(trousseau, &payload.to_string().into_bytes()) {
                done.push(key);
            }
        }
        // **Les propositions de relier** (`propose_link`), au réseau ouvert :
        // la clé d'un réseau à deux, scellée pour la seule officine à qui
        // on la propose. Celle qui n'a pas encore annoncé sa clé de boîte
        // l'attend à la synchronisation suivante.
        if self.open {
            let boxes: std::collections::HashMap<String, String> =
                db.net_box_keys()?.into_iter().collect();
            for (setting, value) in db.settings_with_prefix(LINK_OFFER)? {
                let device = &setting[LINK_OFFER.len()..];
                let Ok(offer) = serde_json::from_str::<LinkOffer>(&value) else {
                    continue;
                };
                let key = format!("relier:{device}:{}", offer.network);
                let Some(theirs) = boxes.get(device).and_then(|k| unhex::<32>(k)) else {
                    continue;
                };
                if already.contains(&key) {
                    continue;
                }
                let inner = serde_json::to_vec(&LinkBox {
                    relier: LinkOffer {
                        officine: officine.to_owned(),
                        ..offer
                    },
                })
                .map_err(|e| e.to_string())?;
                let sealed = bpm_sync::boxed::seal(&[theirs], &inner, &mut bpm_sync::OsEntropy)
                    .map_err(|e| format!("{e:?}"))?;
                let mut payload = crate::messages::BOX_TAG.to_vec();
                payload.extend(sealed);
                if self.seal_one(trousseau, &payload) {
                    done.push(key);
                }
            }
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
        let members: std::collections::HashSet<String> =
            self.peers.iter().map(|p| p.device.clone()).collect();
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
            // Un morceau refusé retient son message : il n'arrive jamais
            // avant ses morceaux.
            let mut chunks_ok = true;
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
                    if self.seal_one(trousseau, &payload) {
                        done.push(key);
                    } else {
                        chunks_ok = false;
                    }
                }
            }
            if !chunks_ok {
                continue;
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
                self.refused
                    .push(crate::strings::tr("msg_too_long").to_owned());
                continue;
            }
            if self.seal_one(trousseau, &payload) {
                done.push(key);
            }
        }
        Ok(())
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
        // **Une lecture à la fois dans ce processus** : au lancement, deux
        // conversations (l'une composée, l'autre reçue) absorbaient le même
        // message, et chacune créait sa conversation.
        static ABSORBING: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _one = ABSORBING
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let me = self.device.id();
        let known = if self.open {
            self.open_authors(db)?
        } else {
            self.known()
        };
        // **Les comptes** (`compte.rs`), des auteurs que ce réseau lit : un
        // enregistrement de compte porte la signature de son compte, et le
        // poste qui l'a passé n'y ajoute rien — c'est ainsi que le réseau
        // les sème. Lus enregistrement par enregistrement, pas par la
        // lecture du flux : une « correction » d'un autre auteur n'en cache
        // aucun. Une officine écartée n'en apporte plus. Ce que ce réseau
        // porte déjà est marqué parti, pour n'y être pas rescellé d'ici.
        let readers: std::collections::HashSet<DeviceId> = known.iter().copied().collect();
        let accounts: Vec<Vec<u8>> = self
            .journal
            .records()
            .filter(|r| r.stream() == Stream::Reseau && r.corrects().is_none())
            .filter(|r| r.author() != me && readers.contains(&r.author()))
            .filter_map(|r| r.open(trousseau).ok())
            .filter(|p| p.starts_with(crate::compte::TAG))
            .collect();
        if !accounts.is_empty() {
            db.keep_account_records(&accounts)?;
            let keys: Vec<String> = accounts
                .iter()
                .map(|b| self.done_key(&format!("acc:{}", crate::compte::name_of(b))))
                .collect();
            db.mark_published(&keys)?;
        }
        // **Lu à travers la vue de confiance** : ce que les conversations
        // relaient d'auteurs qu'on n'a pas ajoutés reste au journal — pour
        // qu'on ne le redemande pas à chaque échange —, mais ne corrige
        // rien ici. Voir `trusted_view`.
        let facts: Vec<bpm_sync::Fact> = self
            .trusted_view(&known)
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
        let mut claimed: std::collections::HashMap<DeviceId, String> =
            std::collections::HashMap::new();
        let mut by_author: std::collections::HashMap<DeviceId, Vec<&bpm_sync::Fact>> =
            std::collections::HashMap::new();
        for f in &facts {
            by_author.entry(f.author).or_default().push(f);
        }
        for author in &known {
            let theirs: &[&bpm_sync::Fact] = by_author.get(author).map_or(&[], Vec::as_slice);
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
            claimed.insert(*author, name.clone());
            db.note_net_heard_in(
                &self.id,
                &hex(&author.0),
                &name,
                i64::try_from(theirs.len()).unwrap_or(i64::MAX),
            )?;
        }
        // **Qui a envoyé, c'est qui a signé** : le nom qu'une officine
        // appairée écrit dans son envoi est à elle de choisir, et une
        // hostile signait ses ruptures du nom d'une autre. Le nom que
        // l'officine lui a donné ici, sinon celui qu'elle signe elle-même.
        let peers = &self.peers;
        // Les noms que portent les autres officines : ceux donnés ici, ceux
        // qu'elles signent, et ceux qu'elles écrivent dans ce passage.
        let own = db.officine().map(|o| o.name).unwrap_or_default();
        // Les noms de **toutes** les officines connues, de tous les
        // réseaux : au réseau ouvert, une inconnue qui signe du nom d'une
        // officine appairée ailleurs est ainsi démasquée.
        let mut names = crate::peer_names(&db.net_peers()?, &own);
        names.extend(
            claimed
                .iter()
                .filter(|(_, n)| !n.trim().is_empty())
                .map(|(a, n)| (hex(&a.0), n.trim().to_owned())),
        );
        let signer = |author: &DeviceId| -> String {
            let device = hex(&author.0);
            let peer = peers.iter().find(|p| p.device == device);
            // Le nom que l'officine lui a donné ici fait foi.
            if let Some(n) = peer.map(|p| p.name.trim()).filter(|n| !n.is_empty()) {
                return n.to_owned();
            }
            // Sinon le nom qu'elle se donne — `seen_as`, ou celui qu'elle
            // écrit dans ce passage même.
            let claimed_name = peer
                .map(|p| p.seen_as.trim().to_owned())
                .filter(|n| !n.is_empty())
                .or_else(|| {
                    claimed
                        .get(author)
                        .map(|n| n.trim().to_owned())
                        .filter(|n| !n.is_empty())
                });
            match claimed_name {
                // **Au réseau ouvert, le nom porte toujours l'empreinte** :
                // c'est la source sous laquelle ses ruptures, ses retraits et
                // ses valeurs sont rangés, et une source qu'une autre
                // pourrait prendre lui laisserait retirer ce qu'elle n'a pas
                // écrit.
                Some(n) if self.open => {
                    let short: String = peer_groups(&device)
                        .split(' ')
                        .take(2)
                        .collect::<Vec<_>>()
                        .join(" ");
                    format!("{} ({short})", clean_name(&n))
                }
                Some(n) => crate::disambiguated(&n, &device, &names),
                None => peer_groups(&device),
            }
        };
        // **Au réseau ouvert, un identifiant est à son auteur** : préfixé
        // de son empreinte, il ne peut ni prendre la place d'un numéro d'une
        // autre officine — ni de celle-ci — ni retirer ce qu'elle a écrit.
        let owned = |author: &DeviceId, uid: &str| -> String {
            if self.open && !uid.is_empty() {
                format!("o{}:{uid}", &hex(&author.0)[..16])
            } else {
                uid.to_owned()
            }
        };
        let events: Vec<crate::ruptures::Event> = facts
            .iter()
            .filter_map(|f| crate::ruptures::decode(&f.payload).map(|e| (f, e)))
            .map(|(f, mut e)| {
                e.source = signer(&f.author);
                e.uid = owned(&f.author, &e.uid);
                e.refers = owned(&f.author, &e.refers);
                e
            })
            .collect();
        // In causal order, so the last value an officine sent for a
        // property is the one kept.
        let shared: Vec<crate::pk::Shared> = facts
            .iter()
            .filter_map(|f| {
                crate::pk::decode_shared(&f.payload).map(|mut s| {
                    s.officine = signer(&f.author);
                    s
                })
            })
            .collect();
        db.receive_net_facts(&shared)?;
        let versions: Vec<crate::versions::Edit> = facts
            .iter()
            .filter_map(|f| crate::versions::decode(&f.payload).map(|e| (f, e)))
            .map(|(f, mut e)| {
                e.source = signer(&f.author);
                e.uid = owned(&f.author, &e.uid);
                e.corrects = owned(&f.author, &e.corrects);
                e
            })
            .collect();
        // What the codex and the protocols changed here is versioned
        // **before** what arrives is ranged: an edit made before this
        // synchronisation replaced what this officine had, not what just
        // came in — written after, it would silently answer it.
        db.version_shared_entries("")?;
        // Du réseau ouvert, **proposées, jamais appliquées** : une
        // officine que personne ici n'a choisie ne récrit pas une fiche ;
        // « En ligne » la propose, prise d'un geste.
        if self.open {
            db.receive_card_edits_offered(&versions)?;
        } else {
            db.receive_card_edits(&versions)?;
        }
        // **Les adresses annoncées** par les officines appairées : la
        // dernière de chacune, gardée à part — jamais à la place de
        // l'adresse saisie ; la synchronisation l'essaie quand celle-ci
        // ne répond pas, et ne la retient qu'une fois l'officine jointe.
        // Pas du réseau ouvert : une adresse annoncée par n'importe qui
        // dirigerait les appels de toutes les officines.
        for f in facts.iter().filter(|_| !self.open) {
            if let Some(a) = serde_json::from_slice::<serde_json::Value>(&f.payload)
                .ok()
                .filter(|v| v.get("t").and_then(|t| t.as_str()) == Some("adresse"))
                .and_then(|v| v.get("a").and_then(|a| a.as_str()).map(str::to_owned))
            {
                if let Some(a) = usable_announced(&a) {
                    let key = announced_key(&hex(&f.author.0));
                    let was = db.setting(&key);
                    if was.as_deref() != Some(a.as_str()) {
                        let _ = db.set_setting(&key, &a, was.as_deref(), "", "");
                    }
                }
            }
        }
        // **Les relais offerts** par les officines du réseau, et **les
        // groupes** qu'elles demandent de relayer — ce que le relais de
        // cette officine accepte, si elle relaie.
        // The **last** word of each officine only — its offer, its request
        // — so a history of offers and withdrawals is not written again at
        // every reading, and a withdrawn request is forgotten.
        let mut offers: std::collections::BTreeMap<String, String> = Default::default();
        let mut requests: std::collections::BTreeMap<String, String> = Default::default();
        // Pas du réseau ouvert : un relais se choisit parmi les siennes.
        for f in facts.iter().filter(|_| !self.open) {
            let Some(v) = serde_json::from_slice::<serde_json::Value>(&f.payload).ok() else {
                continue;
            };
            let author = hex(&f.author.0);
            match v.get("t").and_then(|t| t.as_str()) {
                Some("relais") => {
                    let a = v.get("a").and_then(|a| a.as_str()).unwrap_or_default();
                    offers.insert(author, crate::reach::addresses(a).join(","));
                }
                Some("relais_demande") => {
                    let g = v
                        .get("g")
                        .and_then(|g| g.as_str())
                        .unwrap_or_default()
                        .trim()
                        .to_ascii_lowercase();
                    let g = if g.len() == 20 && g.chars().all(|c| c.is_ascii_hexdigit()) {
                        g
                    } else {
                        String::new()
                    };
                    requests.insert(author, g);
                }
                _ => {}
            }
        }
        for (author, a) in offers {
            let key = relay_key(&author);
            let was = db.setting(&key);
            if was.as_deref().unwrap_or_default() != a {
                let _ = db.set_setting(&key, &a, was.as_deref(), "", "");
            }
        }
        if !requests.is_empty() {
            let mut groups: Vec<String> =
                requests.into_values().filter(|g| !g.is_empty()).collect();
            groups.sort();
            groups.dedup();
            let now = groups.join(",");
            let was = db.setting(RELAY_GROUPS);
            if was.as_deref().unwrap_or_default() != now {
                let _ = db.set_setting(RELAY_GROUPS, &now, was.as_deref(), "", "");
            }
        }
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
            } else if let Ok(b) = serde_json::from_slice::<LinkBox>(&inner) {
                // Only over the open network, from another officine.
                if self.open && author != me_hex {
                    receive_link_offer(db, &author, b.relier)?;
                }
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
        // Le réseau ouvert ne passe que par les conversations : un dossier
        // partagé est l'affaire d'officines qui se connaissent.
        if self.trousseau.is_none() || self.open {
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
    fn trusted_view(&self, known: &[DeviceId]) -> Journal {
        let mut trusted: std::collections::HashSet<DeviceId> = known.iter().copied().collect();
        trusted.insert(self.device.id());
        let mut view = Journal::new();
        for r in self.journal.records() {
            // **Une correction ne vaut que de l'auteur de ce qu'elle
            // corrige.** Le journal lit toute correction comme telle ; une
            // officine appairée mais hostile effaçait ainsi, chez tous, une
            // rupture ou une valeur d'une autre. Le réseau d'officines n'en
            // écrit d'ailleurs aucune.
            let foreign_fix = r.corrects().is_some_and(|t| {
                self.journal
                    .get(&t)
                    .is_none_or(|o| o.author() != r.author())
            });
            if trusted.contains(&r.author()) && !foreign_fix {
                view.insert(r.clone());
            }
        }
        view
    }

    /// **Qui le réseau ouvert lit : ses membres, et eux seuls.** Les
    /// auteurs de son journal y sont inscrits tant qu'il reste de la place
    /// ([`MOST_OPEN_MEMBERS`]) — sauf cette officine, celles qu'elle a
    /// écartées, et celles d'un autre de ses réseaux, lues là sous le nom
    /// qu'on leur a donné. Au-delà, un auteur reste au journal (borné) sans
    /// rien écrire ici : mille identités jetables ne coûtent ni mille
    /// lignes en base, ni mille lectures.
    fn open_authors(&self, db: &Db) -> Result<Vec<DeviceId>, String> {
        let me = self.device.id();
        let today = db.today_iso().unwrap_or_default();
        forget_quiet_open_members(db, &today)?;
        let members: std::collections::HashSet<String> = db
            .net_members(&self.id)?
            .into_iter()
            .map(|p| p.device)
            .collect();
        let mut room = (MOST_OPEN_MEMBERS / 2).saturating_sub(members.len());
        if room > 0 {
            let day = db.today_iso().unwrap_or_default();
            let mut looked: std::collections::HashSet<DeviceId> = Default::default();
            for r in self.journal.records() {
                let a = r.author();
                if room == 0 {
                    break;
                }
                if a == me || !looked.insert(a) || members.contains(&hex(&a.0)) {
                    continue;
                }
                if add_open_member(db, &hex(&a.0), &day, false)? {
                    room -= 1;
                }
            }
        }
        self.read_members(db)
    }

    /// Les membres que cette officine lit : ceux du réseau, moins ceux
    /// qu'elle a écartés.
    fn read_members(&self, db: &Db) -> Result<Vec<DeviceId>, String> {
        let set_aside = ignored(db);
        Ok(db
            .net_members(&self.id)?
            .into_iter()
            .filter(|p| !set_aside.contains(&ignore_key(&self.id, &p.device)))
            .filter_map(|p| unhex::<32>(&p.device).map(DeviceId::from_bytes))
            .collect())
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
        // **Toute la conversation bornée**, comme du côté qui répond : une
        // machine qui se fait passer pour une officine appairée et écrit un
        // octet de temps en temps ne tient pas l'appel.
        let link = bpm_sync::link::dial(address, patience)
            .map_err(|e| format!("{e:?}"))?
            .with_deadline(DIAL_BOUND);
        self.sync_over(db, link, &trousseau)
    }

    /// **Joindre une officine du réseau par un relais** (`relay.rs`) : à
    /// chacun de ceux que les officines du réseau offrent, demander celle
    /// qui y attend sous son nom (`relay::officine_group`) — pour deux
    /// officines qu'aucun chemin direct ne relie. Le relais ne lit rien :
    /// la poignée de main prouve qui répond. Rend qui a répondu.
    pub fn sync_relayed(
        &mut self,
        db: &Db,
        device: &str,
        patience: Duration,
    ) -> Result<Option<String>, String> {
        // **Bounded, and not at every sync**: a few relays, and an officine
        // not found through them is not asked for again before half an
        // hour — an officine simply closed for the night does not cost
        // each sync a round of every relay.
        static TRIED: std::sync::Mutex<
            Option<std::collections::HashMap<String, std::time::Instant>>,
        > = std::sync::Mutex::new(None);
        let key = format!("{}|{device}", self.id);
        let recently = TRIED.lock().ok().is_some_and(|t| {
            t.as_ref()
                .and_then(|m| m.get(&key))
                .is_some_and(|at| at.elapsed() < RELAY_BACKOFF)
        });
        if recently {
            return Ok(None);
        }
        let relays: Vec<String> = offered_relays(db)
            .iter()
            .filter_map(|a| usable_announced(a))
            .take(RELAY_TRIES)
            .collect();
        if relays.is_empty() {
            return Ok(None);
        }
        let said = self.sync_relayed_at(db, device, &relays, patience);
        if let Ok(mut t) = TRIED.lock() {
            let m = t.get_or_insert_with(Default::default);
            m.retain(|_, at| at.elapsed() < RELAY_BACKOFF);
            if matches!(said, Ok(Some(_))) {
                m.remove(&key);
            } else {
                m.insert(key, std::time::Instant::now());
            }
        }
        said
    }

    /// [`Net::sync_relayed`], aux relais `relays`.
    fn sync_relayed_at(
        &mut self,
        db: &Db,
        device: &str,
        relays: &[String],
        patience: Duration,
    ) -> Result<Option<String>, String> {
        let Some(trousseau) = self.trousseau.clone() else {
            return Ok(None);
        };
        if self.open {
            return Ok(None);
        }
        let group = crate::relay::officine_group(&hex(&trousseau.name().bytes()), device);
        let mut last = crate::strings::tr("net_err_no_address").to_owned();
        for at in relays {
            let stream = match crate::relay::call(at, &group, patience) {
                Ok(s) => s,
                Err(e) => {
                    last = e.to_string();
                    continue;
                }
            };
            let link = match bpm_sync::link::TcpLink::new(stream, patience) {
                Ok(l) => l.with_deadline(DIAL_BOUND),
                Err(e) => {
                    last = format!("{e:?}");
                    continue;
                }
            };
            match self.sync_over(db, link, &trousseau) {
                Ok(Some(who)) if who == device => return Ok(Some(who)),
                Ok(_) => last = dial_reason("Handshake"),
                Err(e) => last = e,
            }
        }
        Err(last)
    }

    fn sync_over<L: bpm_sync::Link>(
        &mut self,
        db: &Db,
        mut link: L,
        trousseau: &Trousseau,
    ) -> Result<Option<String>, String> {
        let mut session = Session::new(
            &self.device,
            Some(trousseau),
            Intent::Sync,
            true,
            &self.known(),
        )
        .and_then(|s| if self.open { s.open_to_all() } else { Ok(s) })
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
    /// Synchroniser : le dossier d'échange s'il y en a un, puis chaque
    /// officine qui a une adresse. `announced` : essayer aussi l'adresse
    /// qu'une officine a annoncée — jamais à la fermeture, qui attend la
    /// fin de la tâche.
    Sync {
        folder: Option<PathBuf>,
        announced: bool,
    },
    /// Composer l'adresse d'**une** officine : savoir si elle répond,
    /// sans attendre toutes les autres ni le dossier d'échange.
    Dial { device: String },
}

/// **Relier deux officines sans code** : la proposition qu'une officine
/// fait à une autre, rencontrée au réseau ouvert — la clé d'un réseau à
/// elles deux, son nom, et le nom de qui propose. Scellée pour la seule
/// destinataire ; c'est elle qui accepte, ou non.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LinkOffer {
    /// L'empreinte du réseau proposé (vingt chiffres hexadécimaux).
    pub network: String,
    /// Sa clé, en hexadécimal.
    pub secret: String,
    pub label: String,
    /// Le nom sous lequel signe l'officine qui propose.
    #[serde(default)]
    pub officine: String,
}

/// Ce que porte la boîte : un champ que ni un message ni un morceau n'ont,
/// pour qu'aucun ne se lise pour l'autre.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct LinkBox {
    relier: LinkOffer,
}

/// Combien de relais une synchronisation essaie pour une officine…
const RELAY_TRIES: usize = 4;
/// …et combien de temps elle ne la redemande pas après un échec.
const RELAY_BACKOFF: Duration = Duration::from_secs(30 * 60);

/// Où l'officine range une proposition qu'elle a faite, par destinataire.
pub const LINK_OFFER: &str = "net_link_offer:";
/// Où elle range une proposition reçue, par officine qui propose — vide
/// une fois acceptée, `-` refusée : **refuser écarte l'officine**, pas la
/// seule proposition — sans quoi une proposition neuve à chaque
/// synchronisation revenait d'elle-même.
pub const LINK_ASKED: &str = "net_link_asked:";

/// **Proposer à une officine du réseau ouvert de relier les deux**, sans
/// code : un réseau neuf, nommé `label`, où elle est inscrite, et sa clé
/// qui partira scellée pour elle seule à la prochaine synchronisation.
/// Ce qu'elle reçoit, elle l'accepte ou non ([`accept_link`]) ; d'ici là,
/// ce réseau ne lui répond que si elle a la clé. Rend l'`id` du réseau.
pub fn propose_link(db: &Db, device: &str, label: &str, day: &str) -> Result<String, String> {
    use crate::strings::tr;
    if !db
        .net_members(&open_id())?
        .iter()
        .any(|p| p.device == device)
    {
        return Err(tr("net_link_not_open").to_owned());
    }
    let key = format!("{LINK_OFFER}{device}");
    let was = db.setting(&key);
    if was.as_deref().is_some_and(|v| !v.trim().is_empty()) {
        return Err(tr("net_link_already").to_owned());
    }
    let secret = random32();
    let id = hex(&Trousseau::from_secret(secret).name().bytes());
    let offer = LinkOffer {
        network: id.clone(),
        secret: hex(&secret),
        label: label.trim().to_owned(),
        officine: String::new(),
    };
    let text = serde_json::to_string(&offer).map_err(|e| e.to_string())?;
    // The proposal first, written against what was read: a post that
    // proposed at the same moment wins, and no network is left behind.
    if !db.set_setting(&key, &text, was.as_deref(), day, "")? {
        return Err(tr("net_link_already").to_owned());
    }
    db.add_net_network(&id, &hex(&secret), label, day)?;
    db.add_net_peer_member(&id, device, "", day, false)?;
    Ok(id)
}

/// Une proposition reçue : rangée pour que l'opérateur la voie, sauf un
/// réseau déjà tenu ou refusé, ou une clé qui ne fait pas ce réseau.
fn receive_link_offer(db: &Db, author: &str, offer: LinkOffer) -> Result<(), String> {
    let Some(secret) = unhex::<32>(&offer.secret) else {
        return Ok(());
    };
    if hex(&Trousseau::from_secret(secret).name().bytes()) != offer.network {
        return Ok(());
    }
    if db.net_networks()?.iter().any(|n| n.id == offer.network) {
        return Ok(());
    }
    let key = format!("{LINK_ASKED}{author}");
    let was = db.setting(&key);
    if let Some(w) = &was {
        let declined = w.starts_with('-');
        let same = serde_json::from_str::<LinkOffer>(w).is_ok_and(|o| o.network == offer.network);
        if declined || same {
            return Ok(());
        }
    }
    let text = serde_json::to_string(&offer).map_err(|e| e.to_string())?;
    db.set_setting(&key, &text, was.as_deref(), "", "")
        .map(|_| ())
}

/// Les propositions reçues, à montrer : `(officine qui propose, ce
/// qu'elle propose)`.
pub fn link_offers(db: &Db) -> Vec<(String, LinkOffer)> {
    let held: std::collections::HashSet<String> = db
        .net_networks()
        .unwrap_or_default()
        .into_iter()
        .map(|n| n.id)
        .collect();
    db.settings_with_prefix(LINK_ASKED)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|(k, v)| {
            let offer = serde_json::from_str::<LinkOffer>(&v).ok()?;
            (!held.contains(&offer.network)).then(|| (k[LINK_ASKED.len()..].to_owned(), offer))
        })
        .collect()
}

/// **Accepter** la proposition d'une officine : le réseau à deux rangé,
/// elle inscrite — la synchronisation suivante les relie. Rend l'`id`.
pub fn accept_link(db: &Db, device: &str, day: &str) -> Result<String, String> {
    let key = format!("{LINK_ASKED}{device}");
    let was = db.setting(&key);
    let offer = was
        .as_deref()
        .and_then(|v| serde_json::from_str::<LinkOffer>(v).ok())
        .ok_or_else(|| crate::strings::tr("net_peer_stale").to_owned())?;
    // Against what the screen showed: another post that answered first
    // wins, and this one reloads.
    if !db.set_setting(&key, "", was.as_deref(), day, "")? {
        return Err(crate::strings::tr("net_peer_stale").to_owned());
    }
    db.add_net_network(&offer.network, &offer.secret, &offer.label, day)?;
    db.add_net_peer_member(&offer.network, device, "", day, false)?;
    Ok(offer.network)
}

/// **Refuser** : la proposition n'est plus montrée, et cette officine
/// n'en fait plus d'autre qui se montre.
pub fn decline_link(db: &Db, device: &str) -> Result<(), String> {
    let key = format!("{LINK_ASKED}{device}");
    let was = db.setting(&key);
    let Some(offer) = was
        .as_deref()
        .and_then(|v| serde_json::from_str::<LinkOffer>(v).ok())
    else {
        return Ok(());
    };
    let _ = offer;
    db.set_setting(&key, "-", was.as_deref(), "", "")
        .map(|_| ())
}

/// Le réglage de l'officine : l'adresse où elle se joint de l'extérieur
/// (routeur, VPN), annoncée aux membres de ses réseaux. Vide : aucune.
pub const PUBLIC_ADDRESS: &str = "net_public_address";

/// Où ce poste relaie, quand l'officine relaie (`[reseau] relais`) : ses
/// adresses composables pour le port du relais, écrites par le fil des
/// postes, annoncées au réseau.
pub const RELAY_OFFER: &str = "net_relay_offer";
/// Le groupe de postes que cette officine demande de relayer
/// (`[postes] relais`), en hexadécimal ; vide : aucun.
pub const RELAY_REQUEST: &str = "net_relay_request";
/// Les groupes que les officines du réseau ont demandé de relayer.
pub const RELAY_GROUPS: &str = "net_relay_groups";

/// **Écrire un réglage du relais**, et son numéro quand la valeur change :
/// l'annonce se fait une fois par numéro, pas par valeur — revenir à une
/// offre d'avant (un redémarrage, une adresse retrouvée) l'annonce de
/// nouveau. Rien n'est écrit quand rien ne change.
pub fn set_relay_value(db: &Db, key: &str, value: &str) {
    let was = db.setting(key);
    if was.as_deref().unwrap_or_default() == value {
        return;
    }
    let _ = db.set_setting(key, value, was.as_deref(), "", "");
    let seq_key = format!("{key}_seq");
    let seq = db.setting(&seq_key);
    let next = seq
        .as_deref()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0)
        + 1;
    let _ = db.set_setting(&seq_key, &next.to_string(), seq.as_deref(), "", "");
}

/// Où ranger les adresses de relais qu'une officine du réseau offre.
pub fn relay_key(device: &str) -> String {
    format!("net_relay:{device}")
}

/// **Les officines qu'un relais de cette officine accepte d'elles-mêmes**,
/// sous leur nom de relais (`relay::officine_group`) : celles de ses
/// réseaux fermés — appairées, ou reliées — jamais le réseau ouvert.
pub fn relayed_officines(db: &Db) -> Vec<String> {
    let mut out: Vec<String> = closed_networks(db)
        .iter()
        .flat_map(|(id, name)| {
            db.net_members(id)
                .unwrap_or_default()
                .into_iter()
                .map(|p| crate::relay::officine_group(name, &p.device))
        })
        .collect();
    out.sort();
    out.dedup();
    out
}

/// Les réseaux fermés de l'officine : `(id, nom)` — l'id tel que la base
/// le range (vide pour le principal), le nom tel que la poignée de main
/// le dit.
fn closed_networks(db: &Db) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    if let Some(t) = db
        .setting("net_trousseau")
        .and_then(|t| unhex::<32>(&t))
        .map(Trousseau::from_secret)
    {
        out.push((String::new(), hex(&t.name().bytes())));
    }
    for n in db.net_networks().unwrap_or_default() {
        if !is_open(&n.id) {
            out.push((n.id.clone(), n.id));
        }
    }
    out
}

/// **Où cette officine attend aux relais**, et sous quel nom : chaque
/// adresse qu'une officine offre, sous le nom de chaque réseau fermé que
/// celle-ci partage avec elle — le seul où le relais la connaît. Au plus
/// `most`.
pub fn relay_waits(db: &Db, most: usize) -> Vec<(String, String)> {
    let Ok(me) = device_hex(db) else {
        return Vec::new();
    };
    let networks = closed_networks(db);
    let mut out: Vec<(String, String)> = Vec::new();
    for (key, value) in db.settings_with_prefix("net_relay:").unwrap_or_default() {
        let offerer = &key["net_relay:".len()..];
        for (id, name) in &networks {
            let shares = db
                .net_members(id)
                .unwrap_or_default()
                .iter()
                .any(|p| p.device == offerer);
            if !shares {
                continue;
            }
            for at in crate::reach::addresses(&value)
                .iter()
                .filter_map(|a| usable_announced(a))
            {
                let wait = (at, crate::relay::officine_group(name, &me));
                if !out.contains(&wait) {
                    out.push(wait);
                }
            }
        }
    }
    out.truncate(most);
    out
}

/// Les adresses de relais offertes par les officines du réseau.
pub fn offered_relays(db: &Db) -> Vec<String> {
    let mut out: Vec<String> = db
        .settings_with_prefix("net_relay:")
        .unwrap_or_default()
        .into_iter()
        .flat_map(|(_, v)| crate::reach::addresses(&v))
        .collect();
    out.sort();
    out.dedup();
    out
}

/// Combien de fois l'adresse externe a été enregistrée : ce qui fait
/// repartir l'annonce, même vers une adresse d'avant.
pub const PUBLIC_ADDRESS_SEQ: &str = "net_public_address_seq";

/// **Une adresse annoncée qu'on accepte de composer** : une adresse IP et
/// un port, rien d'autre — pas de nom d'hôte, dont la résolution n'a pas
/// de délai et peut en rendre des dizaines ; ni boucle locale, ni lien
/// local, ni multidiffusion, ni port nul. Un membre du réseau ne choisit
/// pas vers où ce poste frappe.
pub fn usable_announced(text: &str) -> Option<String> {
    let at: std::net::SocketAddr = text.trim().parse().ok()?;
    // Une IPv4 écrite en IPv6 (`::ffff:127.0.0.1`) est jugée comme l'IPv4
    // qu'elle est : sans cela, la boucle locale passait déguisée.
    let ip = at.ip().to_canonical();
    let bad = at.port() == 0
        || ip.is_loopback()
        || ip.is_unspecified()
        || ip.is_multicast()
        || match ip {
            std::net::IpAddr::V4(v4) => v4.is_link_local() || v4.is_broadcast(),
            std::net::IpAddr::V6(v6) => (v6.segments()[0] & 0xffc0) == 0xfe80,
        };
    (!bad).then(|| at.to_string())
}

/// Où ranger l'adresse qu'une officine appairée a annoncée.
pub fn announced_key(device: &str) -> String {
    format!("net_announced:{device}")
}

/// Où ranger la dernière adresse où une officine appairée a répondu,
/// apprise d'une annonce ou de ce qu'elle a déclaré — à côté de l'adresse
/// saisie, jamais à sa place.
pub fn reached_key(device: &str) -> String {
    format!("net_reached:{device}")
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

/// **Le réseau ouvert** : celui que toute officine rejoint d'elle-même,
/// sans invitation, sous une clé que tout le monde tire de ce texte. La
/// clé ne cache donc rien à personne, et ce n'est pas son rôle : ce qui y
/// circule est ce qu'une officine dit à toutes — ses ruptures, ses
/// versions, ses valeurs sourcées, où la joindre, sa clé de boîte. Les
/// messages y restent scellés pour leurs seules destinataires
/// (`bpm_sync::boxed`). **Jamais un patient** : le réseau des officines
/// ne porte que `Stream::Reseau`, comme les autres.
///
/// Ce qui y arrive est **proposé, jamais appliqué** (une version attend
/// « En ligne ») ; seule cette officine règle son horloge (`Journal::trust`) ;
/// une correction ne vaut que de son auteur (`trusted_view`) ; une inconnue
/// tient au plus [`OPEN_LIMITS`] du journal ; une officine écartée n'est
/// plus lue. L'officine le quitte d'un geste ([`leave_open`]).
const OPEN_CONTEXT: &str = "bpm-caddy 2026-10-16 reseau ouvert v1";

/// La clé du réseau ouvert — la même partout.
pub fn open_trousseau() -> Trousseau {
    Trousseau::from_secret(blake3::derive_key(OPEN_CONTEXT, b""))
}

/// L'identifiant du réseau ouvert (l'empreinte de sa clé, en hex).
pub fn open_id() -> String {
    static ID: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    ID.get_or_init(|| hex(&open_trousseau().name().bytes()))
        .clone()
}

/// Ce réseau est-il le réseau ouvert ?
pub fn is_open(id: &str) -> bool {
    !id.is_empty() && id == open_id()
}

/// Le réglage de l'officine qui a quitté le réseau ouvert : « 1 », et il
/// ne se rejoint plus de lui-même.
pub const OPEN_LEFT: &str = "net_ouvert_quitte";

/// Ce que les officines inconnues tiennent au plus au journal du réseau
/// ouvert : 1 Mo chacune, 64 Mo toutes ensemble — mille identités
/// jetables ne remplissent pas le disque d'une officine.
pub const OPEN_LIMITS: bpm_sync::Limits = bpm_sync::Limits {
    per_author: 1 << 20,
    total: 64 << 20,
};

/// Combien d'officines le réseau ouvert inscrit d'elles-mêmes au plus.
pub const MOST_OPEN_MEMBERS: usize = 256;

/// Combien d'officines du réseau ouvert une synchronisation compose au
/// plus : les plus récemment jointes d'abord.
pub const OPEN_DIALS: usize = 6;

/// **Être du réseau ouvert**, sauf quand l'officine l'a quitté. Rend vrai
/// quand on vient d'y entrer. Le poste de référence seulement : la ligne
/// du réseau voyage entre les postes.
pub fn join_open(db: &Db, day: &str) -> Result<bool, String> {
    if db.setting(OPEN_LEFT).is_some_and(|v| v == "1") {
        return Ok(false);
    }
    let secret = open_trousseau().secret().ok_or("clé du réseau ouvert")?;
    db.add_net_network(
        &open_id(),
        &hex(&secret),
        crate::strings::tr("net_open_label"),
        day,
    )
}

/// Quitter le réseau ouvert (`false`) ou le rejoindre (`true`).
pub fn set_open(db: &Db, on: bool, day: &str) -> Result<(), String> {
    // Contre la valeur lue, relue si un autre poste écrit entre-temps :
    // un « quitté » perdu, et le réseau ouvert revenait au déverrouillage.
    let mut written = false;
    for _ in 0..3 {
        let was = db.setting(OPEN_LEFT);
        if db.set_setting(
            OPEN_LEFT,
            if on { "0" } else { "1" },
            was.as_deref(),
            day,
            "",
        )? {
            written = true;
            break;
        }
    }
    if !written {
        return Err(crate::strings::tr("net_network_stale").to_owned());
    }
    if on {
        join_open(db, day).map(|_| ())
    } else {
        db.leave_net_network(&open_id())
    }
}

/// L'officine est-elle du réseau ouvert ?
pub fn in_open(db: &Db) -> bool {
    db.net_networks()
        .is_ok_and(|n| n.iter().any(|n| is_open(&n.id)))
}

/// **Inscrire au réseau ouvert une officine qui y a parlé** — sauf
/// écartée, déjà d'un autre réseau de celle-ci (lue là, sous le nom qu'on
/// lui a donné), ou la liste pleine. `direct` : jointe, ou qui a frappé
/// ici ; sinon seulement l'auteur d'un enregistrement relayé, qui n'a
/// droit qu'à la moitié de la liste — des identités jetables relayées ne
/// prennent pas la place des officines qu'on rencontre. Rend vrai quand
/// elle est nouvelle.
pub fn add_open_member(db: &Db, device: &str, day: &str, direct: bool) -> Result<bool, String> {
    let id = open_id();
    if unhex::<32>(device).is_none() || ignored(db).contains(&ignore_key(&id, device)) {
        return Ok(false);
    }
    let members = db.net_members(&id)?;
    let most = if direct {
        MOST_OPEN_MEMBERS
    } else {
        MOST_OPEN_MEMBERS / 2
    };
    if members.iter().any(|p| p.device == device) || members.len() >= most {
        return Ok(false);
    }
    if db.net_peers()?.iter().any(|p| p.device == device) {
        return Ok(false);
    }
    let mut forgotten = open_forgotten(db);
    if forgotten.iter().any(|d| d == device) {
        if !direct {
            return Ok(false);
        }
        forgotten.retain(|d| d != device);
        set_open_forgotten(db, &forgotten)?;
    }
    db.add_net_peer_member(&id, device, "", day, false)?;
    Ok(true)
}

/// Combien de jours une officine du réseau ouvert reste inscrite sans
/// nouvelles : au-delà, elle sort de la liste (et s'y réinscrit en
/// parlant de nouveau) — la liste ne se remplit pas pour de bon.
pub const OPEN_FORGET_DAYS: i64 = 90;

/// **Oublier les officines du réseau ouvert qui se taisent** depuis
/// [`OPEN_FORGET_DAYS`] : ni ajoutées, ni entendues, ni jointes depuis.
/// Rend combien.
pub fn forget_quiet_open_members(db: &Db, today: &str) -> Result<usize, String> {
    // Sans date du jour lisible, rien : une lecture manquée ne vide pas
    // la liste.
    if crate::date::to_days(today).is_none() {
        return Ok(0);
    }
    let id = open_id();
    let mut forgotten = open_forgotten(db);
    let mut gone = 0;
    for p in db.net_members(&id)? {
        let latest = [&p.added, &p.last_heard, &p.last_ok]
            .into_iter()
            .filter_map(|d| d.get(..10))
            .filter(|d| crate::date::to_days(d).is_some())
            .max()
            .map(str::to_owned);
        // Aucune date lisible : on ne sait pas — gardée.
        let quiet = latest.is_some_and(|l| {
            crate::date::days_between(&l, today).is_some_and(|n| n > OPEN_FORGET_DAYS)
        });
        if quiet {
            db.remove_net_membership(&id, &p.device)?;
            forgotten.retain(|d| *d != p.device);
            forgotten.push(p.device);
            gone += 1;
        }
    }
    if gone > 0 {
        set_open_forgotten(db, &forgotten)?;
    }
    Ok(gone)
}

/// Combien d'officines oubliées ce poste retient : les plus récentes.
const MOST_FORGOTTEN: usize = 1024;

/// **Les officines oubliées** pour s'être tues ([`forget_quiet_open_members`]) :
/// leurs enregistrements, encore au journal, ne les réinscrivent pas —
/// sans cela, oubliées et réinscrites au même passage, elles ne
/// sortaient jamais de la liste. Une conversation directe les reprend.
/// Propre à ce poste, comme la liste elle-même.
fn open_forgotten(db: &Db) -> Vec<String> {
    db.sync_local("open_forgotten")
        .and_then(|v| serde_json::from_str::<Vec<String>>(&v).ok())
        .unwrap_or_default()
}

fn set_open_forgotten(db: &Db, list: &[String]) -> Result<(), String> {
    let keep = &list[list.len().saturating_sub(MOST_FORGOTTEN)..];
    let text = serde_json::to_string(keep).map_err(|e| e.to_string())?;
    db.set_sync_local("open_forgotten", &text)
}

/// **Les officines rencontrées** du réseau ouvert : jointes ou qui ont
/// frappé ici (une conversation directe réussie, `last_ok`), pas
/// seulement relayées. Elles ont leur budget au journal, et le total ne
/// les lie pas (`Journal::favour`) — 1 Mo chacune, au plus
/// [`MOST_OPEN_MEMBERS`].
fn met(peers: &[Peer]) -> Vec<DeviceId> {
    peers
        .iter()
        .filter(|p| !p.last_ok.trim().is_empty())
        .filter_map(|p| unhex::<32>(&p.device).map(DeviceId::from_bytes))
        .collect()
}

/// Le journal tel que la base le garde, avec ce qui règle son horloge —
/// et, pour le réseau ouvert, la borne des inconnues. Ce qu'il a oublié
/// (`forgotten` : nom, auteur et parents, en hex) est relu d'abord, pour
/// qu'une ligne restée en base ne le fasse pas revenir, et compté au
/// budget de son auteur.
fn stored_journal(
    me: &Device,
    peers: &[Peer],
    open: bool,
    records: Vec<Vec<u8>>,
    forgotten: Vec<[String; 3]>,
) -> Journal {
    let mut journal = Journal::new();
    journal.allow_forgetting();
    for [id, author, parents] in forgotten {
        let (Some(id), Some(author)) = (unhex::<32>(&id), unhex::<32>(&author)) else {
            continue;
        };
        let parents: Vec<Hash> = parents
            .as_bytes()
            .chunks(64)
            .filter_map(|c| unhex::<32>(std::str::from_utf8(c).ok()?))
            .map(Hash)
            .collect();
        journal.keep_stub(Hash(id), DeviceId::from_bytes(author), parents);
    }
    if open {
        journal.trust([me.id()]);
        journal.limit_strangers(OPEN_LIMITS);
        // Les officines rencontrées : leur budget, pas le total.
        journal.favour(met(peers));
    } else {
        journal.trust(trusted_ids(me, peers));
    }
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
    db.add_net_peer_member(network, device, "", day, was)?;
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

/// La clé de l'officine sur le réseau — ce qui signe sa carte au
/// répertoire (`repertoire::sign`), la même que l'appairage montre.
pub fn device(db: &Db) -> Result<Device, String> {
    let seed = db.net_key("net_seed", &hex(&random32()))?;
    let seed = unhex::<32>(&seed).ok_or("identité du réseau illisible")?;
    Ok(Device::from_seed(seed))
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
    // **Rien de lourd avant de savoir qui frappe** : les clés et les
    // membres de chaque réseau, sans leurs journaux — une connexion qui
    // ne s'authentifie pas ne coûte pas une lecture de tout le journal.
    let keys = network_keys(db)?;
    if keys.is_empty() {
        return Err(crate::strings::tr("net_err_no_network").to_owned());
    }
    let seed = db.net_key("net_seed", &hex(&random32()))?;
    let seed = unhex::<32>(&seed).ok_or("identité du réseau illisible")?;
    let device = Device::from_seed(seed);
    let choices = keys
        .iter()
        .map(|(id, t, k)| (t.clone(), k.clone(), is_open(id)))
        .collect();
    let mut session = Session::answer_any_open(&device, choices).map_err(|e| format!("{e:?}"))?;
    let mut loaded: Option<Net> = None;
    let mut load = |i: usize| match Net::load_network(db, &keys[i].0) {
        Ok(mut n) => {
            let j = std::mem::take(&mut n.journal);
            loaded = Some(n);
            j
        }
        Err(_) => Journal::new(),
    };
    let (_, journal) = bpm_sync::drive_any(&mut session, link, &mut load, &mut Meter::new())
        .map_err(|e| format!("{e:?}"))?;
    let mut net = loaded.ok_or_else(|| crate::strings::tr("net_err_no_network").to_owned())?;
    net.journal = journal;
    net.keep(db)?;
    let peer = session.peer().map(|p| hex(&p.0)).unwrap_or_default();
    // Qui frappe au réseau ouvert y est inscrite : elle paraît, et on la
    // composera.
    if net.open && add_open_member(db, &peer, &db.today_iso().unwrap_or_default(), true)? {
        net = Net::load_network(db, &net.id)?;
    }
    let _ = db.note_net_dial(&peer, None);
    net.absorb(db)?;
    Ok(peer)
}

/// Les réseaux de l'officine **sans leurs journaux** : identifiant, clé,
/// officines membres — ce qu'il faut pour répondre à une poignée de main.
fn network_keys(db: &Db) -> Result<Vec<(String, Trousseau, Vec<DeviceId>)>, String> {
    let members = |id: &str| -> Result<Vec<DeviceId>, String> {
        Ok(db
            .net_members(id)?
            .iter()
            .filter_map(|p| unhex::<32>(&p.device).map(DeviceId::from_bytes))
            .collect())
    };
    let mut out = Vec::new();
    if let Some(t) = db
        .setting("net_trousseau")
        .and_then(|t| unhex::<32>(&t))
        .map(Trousseau::from_secret)
    {
        out.push((String::new(), t, members("")?));
    }
    for n in db.net_networks()? {
        if let Some(t) = unhex::<32>(&n.secret).map(Trousseau::from_secret) {
            out.push((n.id.clone(), t, members(&n.id)?));
        }
    }
    Ok(out)
}

/// **Composer une officine appairée à l'adresse que son annonce donne**,
/// et ne garder cette adresse qu'**une fois qu'elle y a répondu** : une
/// annonce ne prouve rien — n'importe qui sur le réseau local peut en
/// écrire une sous l'identité d'une officine appairée. Une adresse
/// fausse mène à une poignée de main refusée, et l'adresse saisie ou
/// apprise auparavant reste. Rien pour une officine qu'on n'a pas
/// ajoutée.
pub fn dial_heard(db: &Db, device: &str, address: &str, officine: &str) -> Result<String, String> {
    dial_heard_in(db, None, device, address, officine)
}

/// [`dial_heard`] dans un réseau donné — celui que la synchronisation
/// parcourt ; sans réseau, le premier où l'officine est.
pub fn dial_heard_in(
    db: &Db,
    network: Option<&str>,
    device: &str,
    address: &str,
    officine: &str,
) -> Result<String, String> {
    use crate::strings::{tr, trn};
    let Some(mut net) = Net::load_all(db)?
        .into_iter()
        .filter(|n| network.is_none_or(|id| n.id == id))
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
    // **Rangée à part, jamais à la place de l'adresse saisie** : une
    // machine du réseau local qui relaie vers la vraie officine l'aurait
    // remplacée, et tenu ensuite chaque synchronisation à sa merci.
    let key = reached_key(device);
    let was = db.setting(&key);
    if was.as_deref() != Some(address.trim()) {
        let _ = db.set_setting(&key, address.trim(), was.as_deref(), "", "");
    }
    let _ = db.note_net_dial(device, None);
    let received = net.absorb(db)?;
    let own = db.officine().map(|o| o.name).unwrap_or_default();
    let name = crate::peer_label(&p.device, &net.peers, &own);
    Ok(trn("net_done_dialed", &[&name, &sent, &received]))
}

/// **Composer une officine entendue sur le réseau local, au réseau
/// ouvert** — connue ou non : rien à saisir, rien à comparer. Elle n'est
/// inscrite (et l'adresse gardée à part) qu'**une fois qu'elle a répondu
/// sous l'identité annoncée**.
pub fn dial_open(db: &Db, device: &str, address: &str, officine: &str) -> Result<String, String> {
    dial_open_at(db, Some(device), address, officine)
}

/// **Composer une adresse trouvée dans la DHT** (`dht.rs`), au réseau
/// ouvert : qui que ce soit qui réponde — sauf cette officine elle-même,
/// dont l'adresse revient aussi. Elle n'est inscrite qu'une fois la
/// conversation faite.
pub fn dial_found(db: &Db, address: &str, officine: &str) -> Result<String, String> {
    dial_open_at(db, None, address, officine)
}

fn dial_open_at(
    db: &Db,
    expected: Option<&str>,
    address: &str,
    officine: &str,
) -> Result<String, String> {
    use crate::strings::{tr, trn};
    if !in_open(db) {
        return Err(tr("net_err_no_network").to_owned());
    }
    let mut net = Net::load_network(db, &open_id())?;
    let me = hex(&net.device.id().0);
    let sent = net.publish(db, officine)?;
    let answered = net
        .sync_with(db, address, TALK_PATIENCE)
        .map_err(|e| dial_reason(&e))?;
    let device = match (answered, expected) {
        (Some(a), Some(e)) if a == e => a,
        (Some(a), None) if a != me => a,
        _ => return Err(dial_reason("Handshake")),
    };
    let device = device.as_str();
    add_open_member(db, device, &db.today_iso().unwrap_or_default(), true)?;
    let key = reached_key(device);
    let was = db.setting(&key);
    if was.as_deref() != Some(address.trim()) {
        let _ = db.set_setting(&key, address.trim(), was.as_deref(), "", "");
    }
    let was_met = unhex::<32>(device)
        .map(DeviceId::from_bytes)
        .is_some_and(|d| met(&net.peers).contains(&d));
    let _ = db.note_net_dial(device, None);
    let mut fresh = Net::load_network(db, &open_id())?;
    if was_met {
        fresh.journal = net.journal;
    } else {
        // **Rencontrée à l'instant** : son budget vaut maintenant, et le
        // total ne la lie plus — une seconde conversation prend ce que la
        // première, le total plein, avait refusé.
        let _ = fresh.sync_with(db, address, TALK_PATIENCE);
    }
    let received = fresh.absorb(db)?;
    let own = db.officine().map(|o| o.name).unwrap_or_default();
    let name = crate::peer_label(device, &fresh.peers, &own);
    Ok(trn("net_done_dialed", &[&name, &sent, &received]))
}

/// Les officines du réseau ouvert qu'une synchronisation compose : celles
/// qu'on sait où joindre, les plus récemment jointes d'abord, au plus
/// [`OPEN_DIALS`].
fn open_dial_list(db: &Db, peers: &[Peer]) -> Vec<Peer> {
    let mut out: Vec<Peer> = peers
        .iter()
        // Où elle a répondu, ou ce qu'on a saisi — jamais une adresse
        // qu'elle annonce : n'importe qui en annonce.
        .filter(|p| {
            !p.address.trim().is_empty()
                || db
                    .setting(&reached_key(&p.device))
                    .is_some_and(|a| !a.trim().is_empty())
        })
        .cloned()
        .collect();
    out.sort_by(|a, b| b.last_ok.cmp(&a.last_ok));
    out.truncate(OPEN_DIALS);
    out
}

/// Combien d'attente pour une conversation.
const TALK_PATIENCE: Duration = Duration::from_secs(8);
/// Le temps qu'un appel à une officine peut durer en tout.
const DIAL_BOUND: Duration = Duration::from_secs(120);

/// Lancer une tâche sur son propre fil, avec sa propre connexion à la
/// base — la règle de `maintenance.rs`.
pub fn spawn(
    job: Job,
    path: PathBuf,
    password: String,
    officine: String,
) -> std::sync::mpsc::Receiver<Progress> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let said = run(&job, &path, &password, &officine, &tx);
        let _ = tx.send(match said {
            Ok(done) => Progress::Done(done),
            Err(e) => Progress::Failed(e),
        });
    });
    rx
}

/// Ce que fait le fil. Public pour la synchronisation à la fermeture.
pub fn run(
    job: &Job,
    path: &Path,
    password: &str,
    officine: &str,
    tx: &std::sync::mpsc::Sender<Progress>,
) -> Result<String, String> {
    use crate::strings::{tr, trn};
    let db = Db::open(path, password)?;
    match job {
        Job::Sync { folder, announced } => {
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
                failed.append(&mut net.refused);
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
                let peers = if net.open {
                    open_dial_list(&db, &net.peers)
                } else {
                    net.peers.clone()
                };
                // `dial_heard` lit et écrit par son propre `Net` : celui de
                // cette boucle, relu avant d'absorber, ne récrit pas par-dessus
                // des valeurs plus anciennes.
                let mut reread = false;
                // Les adresses à essayer après celle qu'on a saisie : où
                // l'officine a répondu la dernière fois, puis celle qu'elle
                // annonce (pas à la fermeture).
                let fallbacks = |device: &str, typed: &str| -> Vec<String> {
                    let mut out: Vec<String> = Vec::new();
                    let reached = db.setting(&reached_key(device));
                    let announced = if *announced {
                        db.setting(&announced_key(device))
                            .and_then(|a| usable_announced(&a))
                    } else {
                        None
                    };
                    for a in [reached, announced].into_iter().flatten() {
                        let a = a.trim().to_owned();
                        if !a.is_empty() && a != typed.trim() && !out.contains(&a) {
                            out.push(a);
                        }
                    }
                    out
                };
                // Nowhere to dial it directly: a relay of the network, where
                // an officine nobody reaches waits (`Net::sync_relayed`).
                let by_relay = |net: &mut Net, device: &str| -> bool {
                    !net.open
                        && net
                            .sync_relayed(&db, device, TALK_PATIENCE)
                            .is_ok_and(|who| {
                                who.is_some_and(|w| {
                                    let _ = db.note_net_dial(&w, None);
                                    true
                                })
                            })
                };
                for p in peers.iter().filter(|p| p.address.trim().is_empty()) {
                    let mut reached = false;
                    for a in fallbacks(&p.device, "") {
                        if dial_heard_in(&db, Some(&net.id), &p.device, &a, officine).is_ok() {
                            reread = true;
                            reached = true;
                            break;
                        }
                    }
                    if !reached && by_relay(&mut net, &p.device) {
                        reread = true;
                    }
                }
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
                            let reached = fallbacks(&p.device, &p.address).into_iter().any(|a| {
                                dial_heard_in(&db, Some(&net.id), &p.device, &a, officine).is_ok()
                            });
                            if reached || by_relay(&mut net, &p.device) {
                                reread = true;
                                continue;
                            }
                            let _ = db.note_net_dial(&p.device, Some(&dial_reason(&e)));
                            // Une inconnue du réseau ouvert qui ne répond
                            // pas n'est pas une officine à laquelle on tenait.
                            if net.open {
                                continue;
                            }
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
                if reread {
                    if let Ok(fresh) = Net::load_network(&db, &net.id.clone()) {
                        net = fresh;
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
        Job::Dial { device } => dial_device(&db, device, officine),
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
        let go = |device: String| {
            run(
                &Job::Dial { device },
                &dir.join("net.db"),
                "secret",
                "Pharmacie du Centre",
                &tx,
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
        let said = run(
            &Job::Sync {
                folder: None,
                announced: true,
            },
            &dir.join("net.db"),
            "secret",
            "Pharmacie du Centre",
            &tx,
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

    /// **Un message trop long pour le réseau ne retient rien d'autre** :
    /// il est dit, les suivants partent, et ce qui est scellé est marqué
    /// parti — rien n'est rescellé au tour suivant.
    #[test]
    fn a_message_too_long_holds_back_nothing_else() {
        use crate::messages::Channel;
        let (dir_a, _sa, a) = officine("long-a");
        let (_db, _sb, b) = officine("long-b");
        let folder = dir_a.join("echange");
        Net::create(&a).unwrap();
        let key = a.setting("net_trousseau").unwrap();
        b.net_key("net_trousseau", &key).unwrap();
        let ids: Vec<String> = [&a, &b]
            .iter()
            .map(|d| hex(&Net::load(d).unwrap().device.id().0))
            .collect();
        a.add_net_peer(&ids[1], "", "2026-10-11").unwrap();
        b.add_net_peer(&ids[0], "", "2026-10-11").unwrap();
        for (d, name) in [(&a, "Centre"), (&b, "Port")] {
            let mut n = Net::load(d).unwrap();
            n.publish(d, name).unwrap();
            n.exchange_folder(d, &folder).unwrap();
        }
        let mut na = Net::load(&a).unwrap();
        na.exchange_folder(&a, &folder).unwrap();
        na.absorb(&a).unwrap();
        let conv = a
            .create_conversation(
                "",
                Channel::Officines,
                "Trop long",
                &[],
                None,
                &[ids[1].clone()],
                "CL",
            )
            .unwrap();
        let long = "x".repeat(bpm_sync::MAX_PAYLOAD + 1);
        for (body, at) in [
            (long.as_str(), "2026-10-11 10:00:00"),
            ("Le suivant.", "2026-10-11 10:01:00"),
        ] {
            a.post_message(conv, "", "CL", body, None, "", Some(at))
                .unwrap()
                .unwrap();
        }
        let mut na = Net::load(&a).unwrap();
        assert_eq!(na.publish(&a, "Centre").unwrap(), 1, "le suivant part");
        assert_eq!(na.refused.len(), 1, "le trop long est dit");
        let records = a.net_records_of("").unwrap().len();
        let mut na = Net::load(&a).unwrap();
        assert_eq!(na.publish(&a, "Centre").unwrap(), 0, "rien n'est rescellé");
        assert_eq!(na.refused.len(), 1, "et il est redit");
        assert_eq!(a.net_records_of("").unwrap().len(), records);
        // Ce que l'écran laisse passer part : `NET_BODY_MAX` octets.
        let most = "é".repeat(crate::messages::NET_BODY_MAX / 2);
        a.post_message(conv, "", "CL", &most, None, "", Some("2026-10-11 10:02:00"))
            .unwrap()
            .unwrap();
        let mut na = Net::load(&a).unwrap();
        assert_eq!(
            na.publish(&a, "Centre").unwrap(),
            1,
            "la limite de l'écran tient"
        );
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

    /// **Le relais passe par le réseau** : l'officine qui relaie annonce
    /// où ; celle qui veut être relayée dit le nom de son groupe de
    /// postes, que le relais accepte alors — et une offre retirée l'est
    /// aussi chez les autres.
    #[test]
    fn a_relay_offer_and_a_relay_request_cross_the_network() {
        let (dir_a, _sa, a) = officine("relais-a");
        let (_dir_b, _sb, b) = officine("relais-b");
        let folder = dir_a.join("echange");
        Net::create(&a).unwrap();
        let key = a.setting("net_trousseau").unwrap();
        b.net_key("net_trousseau", &key).unwrap();
        let na = Net::load(&a).unwrap();
        let nb = Net::load(&b).unwrap();
        a.add_net_peer(&hex(&nb.device.id().0), "", "2026-09-26")
            .unwrap();
        b.add_net_peer(&hex(&na.device.id().0), "", "2026-09-26")
            .unwrap();
        set_relay_value(&a, RELAY_OFFER, "82.64.1.2:7746");
        set_relay_value(&b, RELAY_REQUEST, "0123456789abcdef0123");
        let cross = || {
            let mut na = Net::load(&a).unwrap();
            let mut nb = Net::load(&b).unwrap();
            na.publish(&a, "A").unwrap();
            nb.publish(&b, "B").unwrap();
            na.exchange_folder(&a, &folder).unwrap();
            nb.exchange_folder(&b, &folder).unwrap();
            na.exchange_folder(&a, &folder).unwrap();
            na.absorb(&a).unwrap();
            nb.absorb(&b).unwrap();
        };
        cross();
        assert_eq!(offered_relays(&b), vec!["82.64.1.2:7746".to_owned()]);
        assert_eq!(
            a.setting(RELAY_GROUPS).as_deref(),
            Some("0123456789abcdef0123")
        );
        // Withdrawn: the other officine forgets it — and offered again
        // with the same address (a restart), it is announced again.
        set_relay_value(&a, RELAY_OFFER, "");
        set_relay_value(&b, RELAY_REQUEST, "");
        cross();
        assert!(offered_relays(&b).is_empty());
        assert_eq!(a.setting(RELAY_GROUPS).as_deref(), Some(""));
        set_relay_value(&a, RELAY_OFFER, "82.64.1.2:7746");
        cross();
        assert_eq!(offered_relays(&b), vec!["82.64.1.2:7746".to_owned()]);
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

    /// **A et B dans un même réseau appairé**, comme d'avant le réseau
    /// ouvert : la clé du réseau de A (créée au besoin) chez B, chacune
    /// inscrite chez l'autre. Rend ce que chacune « a dit », pour les
    /// tests écrits du temps de l'invitation.
    fn pair(
        dir_a: &std::path::Path,
        dir_b: &std::path::Path,
        network: &str,
    ) -> (Option<Progress>, Option<Progress>) {
        assert!(network.is_empty(), "le principal seulement");
        let a = Db::open(&dir_a.join("net.db"), "secret").unwrap();
        let b = Db::open(&dir_b.join("net.db"), "secret").unwrap();
        if a.setting("net_trousseau").is_none() {
            Net::create(&a).unwrap();
        }
        let key = a.setting("net_trousseau").unwrap();
        b.net_key("net_trousseau", &key).unwrap();
        let (da, db_) = (device_hex(&a).unwrap(), device_hex(&b).unwrap());
        a.add_net_peer(&db_, "", "2026-09-24").unwrap();
        b.add_net_peer(&da, "", "2026-09-24").unwrap();
        let done = || Some(Progress::Done(String::new()));
        (done(), done())
    }

    /// **Deux officines qu'on ne joint pas se parlent par un relais** :
    /// A n'écoute nulle part, elle attend au relais sous son nom ; B, qui
    /// ne la joint à aucune adresse, la demande au relais — et la
    /// poignée de main dit que c'est bien A. Un relais qui ne connaît pas
    /// A ne la met en relation avec personne.
    #[test]
    fn two_unreachable_officines_meet_through_a_relay() {
        let (dir_a, _sa, a) = officine("relay-a");
        let (dir_b, _sb, b) = officine("relay-b");
        pair(&dir_a, &dir_b, "");
        let (a_dev, _) = (device_hex(&a).unwrap(), device_hex(&b).unwrap());
        // A relay knowing A, on loopback.
        let port = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let principal = hex(&Trousseau::from_secret(
            unhex::<32>(&a.setting("net_trousseau").unwrap()).unwrap(),
        )
        .name()
        .bytes());
        let allowed: crate::relay::Allowed = std::sync::Arc::new(std::sync::Mutex::new(
            [crate::relay::officine_group(&principal, &a_dev)].into(),
        ));
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        crate::relay::serve(port, allowed, std::sync::Arc::clone(&stop)).unwrap();
        let relay = format!("127.0.0.1:{port}");
        let (path_a, at) = (dir_a.join("net.db"), relay.clone());
        let group = crate::relay::officine_group(&principal, &a_dev);
        let waiting = std::thread::spawn(move || {
            let db = Db::open(&path_a, "secret").unwrap();
            let stream = crate::relay::wait(&at, &group, TALK_PATIENCE).unwrap();
            let mut link = bpm_sync::link::TcpLink::new(stream, TALK_PATIENCE).unwrap();
            answer(&db, &mut link)
        });
        // The wait reaches the relay before the call.
        std::thread::sleep(Duration::from_millis(300));
        let mut nb = Net::load(&b).unwrap();
        assert_eq!(
            nb.sync_relayed_at(&b, &a_dev, std::slice::from_ref(&relay), TALK_PATIENCE),
            Ok(Some(a_dev.clone()))
        );
        assert!(waiting.join().unwrap().is_ok());
        // Nobody waits under another officine's name: refused, said.
        let other = "ef".repeat(32);
        assert!(nb
            .sync_relayed_at(&b, &other, &[relay], TALK_PATIENCE)
            .is_err());
        // Never through the open network.
        join_open(&b, "2026-10-21").unwrap();
        let mut open = Net::load_network(&b, &open_id()).unwrap();
        assert_eq!(
            open.sync_relayed_at(&b, &a_dev, &[], TALK_PATIENCE),
            Ok(None)
        );
        stop.store(true, std::sync::atomic::Ordering::SeqCst);
        // What a relay of this officine lets through by itself: its closed
        // networks' officines, not the open network's.
        let b_dev = device_hex(&b).unwrap();
        assert_eq!(
            relayed_officines(&a),
            vec![crate::relay::officine_group(&principal, &b_dev)]
        );
        // The name needs the network's: the key alone, public, is not it.
        assert_ne!(
            crate::relay::officine_group(&principal, &a_dev),
            crate::relay::officine_group(&"00".repeat(10), &a_dev)
        );
        assert_ne!(
            crate::relay::officine_group(&principal, &a_dev),
            crate::relay::officine_group(&principal, &other)
        );
        assert!(
            crate::relay::read_preface(&crate::relay::preface(
                crate::relay::Role::Appel,
                &crate::relay::officine_group(&principal, &a_dev)
            ))
            .is_some(),
            "la préface d'avant"
        );
        // Where A waits: at the relay B offers, under their network's name.
        b.set_setting(&relay_key(&a_dev), "", None, "", "").ok();
        a.set_setting(&relay_key(&b_dev), "82.64.1.2:7746", None, "", "")
            .unwrap();
        assert_eq!(
            relay_waits(&a, 8),
            vec![(
                "82.64.1.2:7746".to_owned(),
                crate::relay::officine_group(&principal, &a_dev)
            )]
        );
        // An offer from an officine A shares no closed network with: not there.
        a.set_setting(&relay_key(&other), "82.64.9.9:7746", None, "", "")
            .unwrap();
        assert_eq!(relay_waits(&a, 8).len(), 1);
    }

    #[test]
    fn a_key_reads_back_and_a_bad_one_does_not() {
        let k = random32();
        assert_eq!(unhex::<32>(&hex(&k)), Some(k));
        assert_eq!(unhex::<32>("zz"), None);
        assert_eq!(unhex::<32>(&"0".repeat(63)), None);
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
        let read = again.trusted_view(&again.known()).read(&t, Stream::Reseau);
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
        assert_eq!(stored(&b), Some(before.clone()), "l'adresse d'avant reste");
        // C n'a pas A parmi ses officines : rien à composer.
        assert!(dial_heard(&c, &a_device, &at, "Pharmacie C").is_err());
        // L'adresse annoncée, où A répond : jointe, et gardée.
        let said = dial_heard(&b, &a_device, &at, "Pharmacie B").expect("jointe");
        assert!(!said.is_empty());
        assert_eq!(
            b.setting(&reached_key(&a_device)),
            Some(at.clone()),
            "gardée une fois qu'A y a répondu"
        );
        assert_eq!(
            stored(&b),
            Some(before.clone()),
            "à part : l'adresse saisie ne bouge pas"
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

    /// **Le fil automatique tient la porte** : lancé comme au démarrage de
    /// l'application sur la base de A, il répond à B, appairée, qui compose
    /// son port d'écoute — et le dit à l'écran.
    #[test]
    fn the_auto_thread_answers_a_paired_officine_at_launch() {
        let (dir_a, _sa, a) = officine("auto-door-a");
        let (dir_b, _sb, b) = officine("auto-door-b");
        Net::create(&a).unwrap();
        drop((a, b));
        let (da, db_) = pair(&dir_a, &dir_b, "");
        assert!(matches!(da, Some(Progress::Done(_))), "{da:?}");
        assert!(matches!(db_, Some(Progress::Done(_))), "{db_:?}");
        let free = |_: ()| {
            std::net::TcpListener::bind("127.0.0.1:0")
                .unwrap()
                .local_addr()
                .unwrap()
                .port()
        };
        let (posts_port, listen) = (free(()), free(()));
        let a = Db::open(&dir_a.join("net.db"), "secret").unwrap();
        let a_device = device_hex(&a).unwrap();
        drop(a);
        let (rx, poke) = crate::postes::spawn_auto(
            dir_a.join("net.db"),
            "secret".to_owned(),
            "2026-09-25".to_owned(),
            posts_port,
            None,
            Vec::new(),
            crate::postes::OfficineSide {
                announce: None,
                listen,
                name: "Pharmacie A".to_owned(),
                posts_paused: false,
                posts_internet: false,
                relay_port: 0,
                posts_relay: false,
                relay_patients: false,
            },
            crate::postes::Pace::default(),
        );
        let b = Db::open(&dir_b.join("net.db"), "secret").unwrap();
        let at = format!("127.0.0.1:{listen}");
        let mut joined = Err(String::new());
        for _ in 0..40 {
            joined = dial_heard(&b, &a_device, &at, "Pharmacie B");
            if joined.is_ok() {
                break;
            }
            std::thread::sleep(Duration::from_millis(250));
        }
        assert!(joined.is_ok(), "{joined:?}");
        let mut said = false;
        for _ in 0..40 {
            if let Ok(crate::postes::Progress::Status(line)) =
                rx.recv_timeout(Duration::from_millis(250))
            {
                if line.contains("connectée") {
                    said = true;
                    break;
                }
            }
        }
        poke.send(crate::postes::Poke::Stop).unwrap();
        assert!(said, "la connexion est dite à l'écran");
    }

    /// **L'adresse d'un autre site** : A déclare où elle se joint de
    /// l'extérieur ; B, appairée, l'apprend par le réseau et sa
    /// synchronisation la joint là, puis retient l'adresse. Une officine
    /// non appairée qui annoncerait une adresse n'est pas écoutée.
    #[test]
    fn an_officine_announces_where_it_can_be_reached_and_is_reached_there() {
        let (dir_a, _sa, a) = officine("public-a");
        let (dir_b, _sb, b) = officine("public-b");
        Net::create(&a).unwrap();
        drop((a, b));
        let (da, db_) = pair(&dir_a, &dir_b, "");
        assert!(matches!(da, Some(Progress::Done(_))), "{da:?}");
        assert!(matches!(db_, Some(Progress::Done(_))), "{db_:?}");
        let a = Db::open(&dir_a.join("net.db"), "secret").unwrap();
        let b = Db::open(&dir_b.join("net.db"), "secret").unwrap();
        let a_device = device_hex(&a).unwrap();
        // B ne garde aucune adresse de A : celle de l'invitation est fermée.
        b.set_net_peer(&a_device, "", "", ("", "")).ok();
        let door = bpm_sync::link::Door::open("127.0.0.1:0").unwrap();
        let at = door.address().unwrap().to_string();
        // Annoncée : une adresse publique. La boucle locale du test, elle,
        // n'est jamais acceptée d'une annonce — on la compose à la main.
        let public = "203.0.113.9:7745";
        a.set_setting(PUBLIC_ADDRESS, public, None, "2026-09-25", "")
            .unwrap();
        let folder = dir_a.join("echange");
        let mut net_a = Net::load(&a).unwrap();
        net_a.publish(&a, "Pharmacie A").unwrap();
        net_a.exchange_folder(&a, &folder).unwrap();
        let mut net_b = Net::load(&b).unwrap();
        net_b.exchange_folder(&b, &folder).unwrap();
        net_b.absorb(&b).unwrap();
        assert_eq!(
            b.setting(&announced_key(&a_device)),
            Some(public.to_owned())
        );
        // A tient sa porte ; B synchronise : jointe à l'adresse annoncée.
        let path_a = dir_a.join("net.db");
        let answering = std::thread::spawn(move || {
            let db = Db::open(&path_a, "secret").unwrap();
            let mut link = door.accept(Duration::from_secs(20)).unwrap();
            answer(&db, &mut link)
        });
        dial_heard(&b, &a_device, &at, "Pharmacie B").expect("jointe");
        assert!(answering.join().unwrap().is_ok());
        assert_eq!(
            b.setting(&reached_key(&a_device)),
            Some(at),
            "retenue une fois jointe"
        );
    }

    /// **Relier deux officines sans code** : rencontrées au réseau ouvert,
    /// A propose à B un réseau à elles deux ; la clé part scellée pour B
    /// seule, B voit la proposition (de qui, sous quel nom), l'accepte, et
    /// les deux se parlent sur ce réseau. Une seule proposition à la fois,
    /// et seulement à une officine du réseau ouvert.
    #[test]
    fn two_officines_met_openly_link_without_a_code() {
        let (dir_a, _sa, a) = officine("link-a");
        let (dir_b, _sb, b) = officine("link-b");
        for (db, name) in [(&a, "Pharmacie A"), (&b, "Pharmacie B")] {
            join_open(db, "2026-10-21").unwrap();
            let mut n = Net::load_network(db, &open_id()).unwrap();
            n.publish(db, name).unwrap();
            n.keep(db).unwrap();
        }
        let _ = dir_a;
        let (a_dev, b_dev) = (device_hex(&a).unwrap(), device_hex(&b).unwrap());
        let door = bpm_sync::link::Door::open("127.0.0.1:0").unwrap();
        let at = door.address().unwrap().to_string();
        let path_b = dir_b.join("net.db");
        let answering = std::thread::spawn(move || {
            let db = Db::open(&path_b, "secret").unwrap();
            let mut said = Vec::new();
            while let Ok(mut link) = door.accept(Duration::from_secs(5)) {
                said.push(answer(&db, &mut link));
            }
            said
        });
        dial_open(&a, &b_dev, &at, "Pharmacie A").expect("rencontrée");
        assert!(propose_link(&a, &"cd".repeat(32), "X", "2026-10-21").is_err());
        let id = propose_link(&a, &b_dev, "A et B", "2026-10-21").unwrap();
        assert!(
            propose_link(&a, &b_dev, "Encore", "2026-10-21").is_err(),
            "une fois"
        );
        assert!(a
            .net_members(&id)
            .unwrap()
            .iter()
            .any(|p| p.device == b_dev));
        dial_open(&a, &b_dev, &at, "Pharmacie A").expect("la proposition part");
        let until = std::time::Instant::now() + Duration::from_secs(5);
        let offers = loop {
            let o = link_offers(&b);
            if !o.is_empty() || std::time::Instant::now() > until {
                break o;
            }
            std::thread::sleep(Duration::from_millis(50));
        };
        assert_eq!(offers.len(), 1, "{offers:?}");
        let (from, offer) = &offers[0];
        assert_eq!(from, &a_dev);
        assert_eq!(offer.network, id);
        assert_eq!(offer.label, "A et B");
        assert_eq!(offer.officine, "Pharmacie A");
        assert!(
            b.net_networks().unwrap().iter().all(|n| n.id != id),
            "pas avant"
        );
        assert_eq!(accept_link(&b, &a_dev, "2026-10-21").unwrap(), id);
        assert!(link_offers(&b).is_empty());
        assert!(b
            .net_members(&id)
            .unwrap()
            .iter()
            .any(|p| p.device == a_dev));
        // Their own network now: A dials B on it, and B answers there.
        let mut n = Net::load_network(&a, &id).unwrap();
        assert!(!n.open);
        assert_eq!(
            n.sync_with(&a, &at, TALK_PATIENCE).unwrap(),
            Some(b_dev.clone())
        );
        let said = answering.join().unwrap();
        assert!(said.iter().all(Result::is_ok), "{said:?}");
        assert!(said.iter().all(|s| s.as_deref() == Ok(a_dev.as_str())));
    }

    /// Une proposition reçue dont la clé ne fait pas le réseau qu'elle
    /// nomme n'est pas montrée ; refusée, elle ne revient pas — la boîte
    /// reste au journal, relue à chaque synchronisation.
    #[test]
    fn a_link_offer_that_lies_is_dropped_and_a_declined_one_stays_declined() {
        let (_dir, _s, b) = officine("link-decline");
        let c_dev = "ce".repeat(32);
        let secret = random32();
        let good = LinkOffer {
            network: hex(&Trousseau::from_secret(secret).name().bytes()),
            secret: hex(&secret),
            label: "C et B".to_owned(),
            officine: "Pharmacie C".to_owned(),
        };
        let lie = LinkOffer {
            network: "00".repeat(10),
            ..good.clone()
        };
        receive_link_offer(&b, &c_dev, lie).unwrap();
        assert!(
            link_offers(&b).is_empty(),
            "une clé qui ne fait pas ce réseau"
        );
        receive_link_offer(&b, &c_dev, good.clone()).unwrap();
        assert_eq!(link_offers(&b), vec![(c_dev.clone(), good.clone())]);
        receive_link_offer(&b, &c_dev, good.clone()).unwrap();
        assert_eq!(link_offers(&b).len(), 1, "pas deux fois");
        decline_link(&b, &c_dev).unwrap();
        assert!(link_offers(&b).is_empty());
        receive_link_offer(&b, &c_dev, good).unwrap();
        assert!(link_offers(&b).is_empty(), "refusée, elle le reste");
        // A fresh proposal from the same officine: not shown either.
        let other = random32();
        let fresh = LinkOffer {
            network: hex(&Trousseau::from_secret(other).name().bytes()),
            secret: hex(&other),
            label: "Encore".to_owned(),
            officine: "Pharmacie C".to_owned(),
        };
        receive_link_offer(&b, &c_dev, fresh).unwrap();
        assert!(link_offers(&b).is_empty(), "l'officine écartée");
        assert!(accept_link(&b, &c_dev, "2026-10-21").is_err());
    }

    /// **Deux officines qui ne se sont jamais appairées se parlent au
    /// réseau ouvert** : B compose A, rien à saisir ni à comparer. A inscrit
    /// B, reçoit sa substitution — signée de son nom et de son empreinte,
    /// sans note ni initiales —, et sa version d'une
    /// fiche reste **proposée** : aucune fiche n'est créée ni récrite. Le
    /// quitter le referme, et le rejoindre le rouvre.
    #[test]
    fn two_strangers_meet_on_the_open_network_and_nothing_applies_itself() {
        let (_dir_a, _sa, a) = officine("open-a");
        let (_dir_b, _sb, b) = officine("open-b");
        for db in [&a, &b] {
            assert!(join_open(db, "2026-10-16").unwrap());
            assert!(!join_open(db, "2026-10-16").unwrap(), "une fois");
            assert!(in_open(db));
        }
        let (a_device, b_device) = (device_hex(&a).unwrap(), device_hex(&b).unwrap());
        b.add_supply_event(&subst("Diprosone", "Locoid")).unwrap();
        let mut nb = Net::load_network(&b, &open_id()).unwrap();
        assert!(nb.open && nb.peers.is_empty());
        // Ce que B publie au réseau ouvert : ni note, ni initiales.
        nb.publish(&b, "Pharmacie B").unwrap();
        let t = open_trousseau();
        for r in nb.journal.records() {
            let text = String::from_utf8(r.open(&t).unwrap()).unwrap();
            assert!(!text.contains("\"CL\""), "{text}");
        }
        let edit = crate::versions::Edit {
            kind: crate::versions::Kind::Fiche,
            uid: "bbbb:1".to_owned(),
            day: "2026-10-16".to_owned(),
            card: crate::versions::key("Zolmitriptan"),
            card_name: "Zolmitriptan".to_owned(),
            field: "dci".to_owned(),
            value: "zolmitriptan".to_owned(),
            previous: String::new(),
            corrects: String::new(),
            revert: false,
            operator: "CL".to_owned(),
            source: String::new(),
        };
        assert!(nb.seal_one(
            &open_trousseau(),
            &crate::versions::encode(&edit, "Pharmacie B")
        ));
        nb.keep(&b).unwrap();
        let door = bpm_sync::link::Door::open("127.0.0.1:0").unwrap();
        let at = door.address().unwrap().to_string();
        let path_a = _dir_a.join("net.db");
        let answering = std::thread::spawn(move || {
            let db = Db::open(&path_a, "secret").unwrap();
            let mut link = door.accept(Duration::from_secs(20)).unwrap();
            answer(&db, &mut link)
        });
        dial_open(&b, &a_device, &at, "Pharmacie B").expect("jointe");
        assert_eq!(answering.join().unwrap(), Ok(b_device.clone()));
        // Chacune a inscrit l'autre.
        assert!(a
            .net_members(&open_id())
            .unwrap()
            .iter()
            .any(|p| p.device == b_device));
        assert!(b
            .net_members(&open_id())
            .unwrap()
            .iter()
            .any(|p| p.device == a_device));
        // Et pas au principal : aucune n'en a.
        assert!(Net::load(&a).unwrap().peers.is_empty() || a.setting("net_trousseau").is_none());
        let got = a.supply_events().unwrap();
        let e = got
            .iter()
            .find(|e| e.product == "Diprosone")
            .expect("reçue");
        // Signée de son nom **et de son empreinte**, sous un identifiant
        // à elle, sans note ni initiales.
        let short: String = peer_groups(&b_device)
            .split(' ')
            .take(2)
            .collect::<Vec<_>>()
            .join(" ");
        assert_eq!(e.source, format!("Pharmacie B ({short})"));
        assert!(
            e.uid.starts_with(&format!("o{}:", &b_device[..16])),
            "{}",
            e.uid
        );
        assert!(e.operator.is_empty() && e.note.is_empty());
        let offered = a.card_edits().unwrap();
        let mine = format!("o{}:bbbb:1", &b_device[..16]);
        assert!(offered.iter().any(|e| e.uid == mine), "rangée");
        assert!(
            a.drugs()
                .unwrap()
                .iter()
                .all(|d| crate::versions::key(&d.name) != edit.card),
            "aucune fiche créée"
        );
        // Quitter, puis rejoindre.
        set_open(&a, false, "2026-10-16").unwrap();
        assert!(!in_open(&a));
        assert!(!join_open(&a, "2026-10-17").unwrap(), "quitté, il le reste");
        assert!(a.net_members(&open_id()).unwrap().is_empty());
        set_open(&a, true, "2026-10-17").unwrap();
        assert!(in_open(&a));
    }

    /// **Un compte se sème par le réseau ouvert** : écrit chez A, passé
    /// par B, retrouvé chez C avec sa seule phrase — sans que B ni C lisent
    /// ce qui est scellé, ni que A et C se soient jamais parlé.
    #[test]
    fn an_account_is_seeded_by_the_open_network_and_found_again_by_its_phrase() {
        use crate::compte;
        let (dir_a, _sa, a) = officine("acc-a");
        let (dir_b, _sb, b) = officine("acc-b");
        let (_dir_c, _sc, c) = officine("acc-c");
        for db in [&a, &b, &c] {
            join_open(db, "2026-10-19").unwrap();
        }
        let claire =
            compte::Account::from_phrase(compte::Phrase::generate(&mut bpm_sync::OsEntropy));
        let profile = compte::Profile {
            nom: "Claire Martin".to_owned(),
            ..compte::Profile::default()
        };
        let note = compte::Entry {
            titre: "Codes du coffre".to_owned(),
            ..compte::Entry::default()
        };
        let records = vec![
            compte::write_profile(&claire, 1, &profile, &mut bpm_sync::OsEntropy).unwrap(),
            compte::write_entry(
                &claire,
                ("n1", 1),
                compte::Kind::Note,
                &note,
                &compte::Visibility::Private,
                &|_| None,
                &mut bpm_sync::OsEntropy,
            )
            .unwrap(),
        ];
        assert_eq!(a.keep_account_records(&records).unwrap(), 2);
        assert_eq!(a.keep_account_records(&records).unwrap(), 0, "une fois");
        // Ouvert sur un poste de A : c'est A qui le publie.
        a.keep_account(&claire.id(), "Claire Martin", b"scelle")
            .unwrap();
        // A passe à B, B passe à C.
        for (from, from_dir, to) in [(&a, &dir_a, &b), (&b, &dir_b, &c)] {
            Net::load_network(from, &open_id())
                .unwrap()
                .publish(from, "P")
                .unwrap();
            let door = bpm_sync::link::Door::open("127.0.0.1:0").unwrap();
            let at = door.address().unwrap().to_string();
            let path = from_dir.join("net.db");
            let answering = std::thread::spawn(move || {
                let db = Db::open(&path, "secret").unwrap();
                let mut link = door.accept(Duration::from_secs(20)).unwrap();
                answer(&db, &mut link)
            });
            dial_open(to, &device_hex(from).unwrap(), &at, "P").expect("jointe");
            answering.join().unwrap().unwrap();
            Net::load_network(to, &open_id())
                .unwrap()
                .absorb(to)
                .unwrap();
        }
        // B et C gardent le compte, scellé.
        for db in [&b, &c] {
            let held = db.account_latest(None).unwrap();
            assert_eq!(held.len(), 2);
            assert!(held
                .iter()
                .any(|(r, _)| r.state == compte::State::Sealed(None)));
        }
        // C, avec la phrase : le profil, et la note ouverte.
        let again = compte::Account::from_phrase(
            compte::Phrase::parse(&claire.phrase().words().join(" ")).unwrap(),
        );
        let got = c.account_latest(Some(&again)).unwrap();
        let profile = got.iter().find_map(|(r, _)| compte::profile_of(r)).unwrap();
        assert_eq!(profile.nom, "Claire Martin");
        let opened = got.iter().find_map(|(r, _)| compte::entry_of(r)).unwrap();
        assert_eq!(opened.titre, "Codes du coffre");
        // Ce que C a reçu ne repart pas de C sous un autre sceau : B le
        // porte déjà, C l'a marqué porté, et le compte n'est pas ouvert
        // chez C.
        let mut nc = Net::load_network(&c, &open_id()).unwrap();
        let before = nc.journal.len();
        nc.publish(&c, "P").unwrap();
        assert_eq!(nc.journal.len(), before);
    }

    /// **Une version remplacée s'oublie, partout, sans couper le chemin** :
    /// A publie trois versions d'un profil ; A, puis B qui les a reçues, ne
    /// gardent que la dernière — en mémoire, en base, et relues — et C,
    /// qui n'a jamais vu les anciennes, reçoit la dernière de B et ne
    /// redemande rien qui reste dû.
    #[test]
    fn a_superseded_account_version_is_forgotten_everywhere_without_cutting_a_path() {
        use crate::compte;
        let (dir_a, _sa, a) = officine("forget-a");
        let (dir_b, _sb, b) = officine("forget-b");
        let (_dir_c, _sc, c) = officine("forget-c");
        for db in [&a, &b, &c] {
            join_open(db, "2026-10-20").unwrap();
        }
        let claire =
            compte::Account::from_phrase(compte::Phrase::generate(&mut bpm_sync::OsEntropy));
        a.keep_account(&claire.id(), "Claire", b"scelle").unwrap();
        let version = |seq: u64| {
            let p = compte::Profile {
                nom: format!("Claire v{seq}"),
                ..compte::Profile::default()
            };
            compte::write_profile(&claire, seq, &p, &mut bpm_sync::OsEntropy).unwrap()
        };
        let accounts = |db: &Db| {
            let net = Net::load_network(db, &open_id()).unwrap();
            let t = net.trousseau.clone().unwrap();
            let held = net
                .journal
                .records()
                .filter_map(|r| r.open(&t).ok())
                .filter(|p| p.starts_with(compte::TAG))
                .count();
            (held, net.journal.stubs().count())
        };
        let pass = |from: &Db, from_dir: &std::path::Path, to: &Db| {
            Net::load_network(from, &open_id())
                .unwrap()
                .publish(from, "P")
                .unwrap();
            let door = bpm_sync::link::Door::open("127.0.0.1:0").unwrap();
            let at = door.address().unwrap().to_string();
            let path = from_dir.join("net.db");
            let answering = std::thread::spawn(move || {
                let db = Db::open(&path, "secret").unwrap();
                let mut link = door.accept(Duration::from_secs(20)).unwrap();
                answer(&db, &mut link)
            });
            dial_open(to, &device_hex(from).unwrap(), &at, "P").expect("jointe");
            answering.join().unwrap().unwrap();
            let mut net = Net::load_network(to, &open_id()).unwrap();
            net.absorb(to).unwrap();
            net.publish(to, "P").unwrap();
        };
        for seq in 1..=3 {
            a.keep_account_records(&[version(seq)]).unwrap();
            pass(&a, &dir_a, &b);
        }
        // A et B, relus de leur base : la dernière version, deux oubliées.
        assert_eq!(accounts(&a), (1, 2));
        assert_eq!(accounts(&b), (1, 2));
        let rows = |db: &Db| db.net_records_of(&open_id()).unwrap().len();
        let before = rows(&b);
        // Reçues encore d'un journal qui les tenait : pas reprises.
        pass(&a, &dir_a, &b);
        assert_eq!(rows(&b), before);
        // C ne les a jamais vues : il reçoit de B la dernière, et ce qui
        // la précédait au journal.
        pass(&b, &dir_b, &c);
        let name = |db: &Db| {
            db.account_latest(None)
                .unwrap()
                .iter()
                .find_map(|(r, _)| compte::profile_of(r))
                .map(|p| p.nom)
        };
        assert_eq!(name(&b).as_deref(), Some("Claire v3"));
        assert_eq!(name(&c).as_deref(), Some("Claire v3"));
        assert_eq!(accounts(&c).0, 1);
        let held_b: std::collections::BTreeSet<_> = Net::load_network(&b, &open_id())
            .unwrap()
            .journal
            .records()
            .filter(|r| r.author() != Net::load_network(&c, &open_id()).unwrap().device.id())
            .map(|r| r.id())
            .collect();
        let net_c = Net::load_network(&c, &open_id()).unwrap();
        assert!(
            held_b.iter().all(|h| net_c.journal.has(h)),
            "tout ce que B garde est passé, malgré les trous"
        );
    }

    /// **Au réseau ouvert, une officine reste entendue** : arrivée au bout
    /// du budget qu'une autre lui accorde, elle y perd ses enregistrements
    /// les plus anciens — pas son compte, pas sa clé de boîte —, et ce
    /// qu'elle dit ensuite est reçu au lieu d'être refusé pour la vie.
    #[test]
    fn a_stranger_at_the_end_of_its_budget_loses_its_oldest_records_not_its_voice() {
        use crate::compte;
        let (_dir_a, _sa, a) = officine("room-a");
        let (_dir_b, _sb, b) = officine("room-b");
        for db in [&a, &b] {
            join_open(db, "2026-10-20").unwrap();
        }
        let claire =
            compte::Account::from_phrase(compte::Phrase::generate(&mut bpm_sync::OsEntropy));
        a.keep_account(&claire.id(), "Claire", b"scelle").unwrap();
        let profile = compte::Profile {
            nom: "Claire".to_owned(),
            ..compte::Profile::default()
        };
        a.keep_account_records(&[compte::write_profile(
            &claire,
            1,
            &profile,
            &mut bpm_sync::OsEntropy,
        )
        .unwrap()])
            .unwrap();
        Net::load_network(&a, &open_id())
            .unwrap()
            .publish(&a, "A")
            .unwrap();
        for n in 0..40 {
            a.add_supply_event(&subst(&format!("Produit {n}"), "Autre"))
                .unwrap();
        }
        let mut net_a = Net::load_network(&a, &open_id()).unwrap();
        net_a.publish(&a, "A").unwrap();
        let t = net_a.trousseau.clone().unwrap();
        let mut theirs: Vec<Record> = net_a.journal.records().cloned().collect();
        theirs.sort_by_key(|r| (r.lamport(), r.id()));
        let is = |r: &Record, what: &dyn Fn(&[u8]) -> bool| r.open(&t).is_ok_and(|p| what(&p));
        let rupture = |p: &[u8]| crate::ruptures::decode(p).is_some();
        let account = |p: &[u8]| p.starts_with(compte::TAG);
        let boite = |p: &[u8]| {
            serde_json::from_slice::<serde_json::Value>(p).is_ok_and(|v| v["t"] == "boite")
        };
        assert!(theirs.iter().filter(|r| is(r, &rupture)).count() >= 40);
        let size: usize = theirs.iter().map(|r| r.encode().len()).sum();
        let mut net_b = Net::load_network(&b, &open_id()).unwrap();
        net_b.journal.limit_strangers(bpm_sync::Limits {
            per_author: size / 4 * 3,
            total: usize::MAX,
        });
        for r in &theirs {
            net_b.journal.insert(r.clone());
        }
        let newest = theirs.last().unwrap().id();
        assert!(!net_b.journal.has(&newest), "le budget est plein");
        // Une officine que B ne lit pas n'y perd rien.
        assert_eq!(net_b.forget_superseded(&b, &t).unwrap(), 0);
        add_open_member(&b, &hex(&net_a.device.id().0), "2026-10-20", false).unwrap();
        let gone = net_b.forget_superseded(&b, &t).unwrap();
        assert!(gone > 0);
        assert_eq!(db_forgotten(&b), gone, "la base les oublie aussi");
        for r in &theirs {
            net_b.journal.insert(r.clone());
        }
        assert!(
            net_b.journal.has(&newest),
            "ce qu'elle dit ensuite est reçu"
        );
        let oldest = theirs.iter().find(|r| is(r, &rupture)).unwrap().id();
        assert!(!net_b.journal.has(&oldest) && net_b.journal.knows(&oldest));
        for what in [&account as &dyn Fn(&[u8]) -> bool, &boite] {
            let r = theirs.iter().find(|r| is(r, what)).expect("publié");
            assert!(net_b.journal.has(&r.id()), "gardé");
        }
        let (held, most) = net_b.journal.budget_of(&net_a.device.id()).unwrap();
        assert!(held <= most);
    }

    fn db_forgotten(db: &Db) -> usize {
        db.net_forgotten_of(&open_id()).unwrap().len()
    }

    /// **Un compte ouvert sur un autre poste de l'officine se sème aussi** —
    /// un téléphone, ou un poste qui ne tient pas le réseau : sa trace
    /// (`account_homes`) voyage sans la phrase, et le poste du réseau le
    /// publie. Un compte venu d'ailleurs, que nul poste ne garde, ne part pas.
    #[test]
    fn an_account_kept_on_another_post_of_the_officine_is_seeded_too() {
        use crate::compte;
        let (_dir, _s, a) = officine("acc-home");
        join_open(&a, "2026-10-20").unwrap();
        let person =
            || compte::Account::from_phrase(compte::Phrase::generate(&mut bpm_sync::OsEntropy));
        let (phone, stranger) = (person(), person());
        let profile = |nom: &str| compte::Profile {
            nom: nom.to_owned(),
            ..compte::Profile::default()
        };
        let records = vec![
            compte::write_profile(&phone, 1, &profile("Julien"), &mut bpm_sync::OsEntropy).unwrap(),
            compte::write_profile(&stranger, 1, &profile("Inconnu"), &mut bpm_sync::OsEntropy)
                .unwrap(),
        ];
        a.keep_account_records(&records).unwrap();
        let mut net = Net::load_network(&a, &open_id()).unwrap();
        net.publish(&a, "P").unwrap();
        let before = net.journal.len();
        let published = |net: &Net| {
            net.journal
                .records()
                .filter_map(|r| r.open(net.trousseau.as_ref().unwrap()).ok())
                .filter(|p| p.starts_with(compte::TAG))
                .count()
        };
        assert_eq!(published(&net), 0, "personne ne garde ces comptes ici");
        // Ce que le téléphone écrit en gardant le compte, arrivé par les postes.
        a.conn_exec_for_tests(&format!(
            "INSERT INTO account_homes (id, account) VALUES ({}, '{}')",
            crate::db::next_id("account_homes"),
            phone.id()
        ))
        .unwrap();
        assert!(a.kept_accounts().unwrap().is_empty());
        net.publish(&a, "P").unwrap();
        assert_eq!(net.journal.len(), before + 1);
        assert_eq!(published(&net), 1, "le compte du téléphone, seul");
        // Retiré du téléphone : sa version suivante ne part plus d'ici.
        a.forget_account(&phone.id()).unwrap();
        assert!(a.officine_accounts().unwrap().is_empty());
        // Un poste retiré — un téléphone perdu — ne garde plus rien.
        a.conn_exec_for_tests(
            "INSERT INTO sync_posts (post, device, left_on) VALUES (5, 'perdu', '2026-10-20')",
        )
        .unwrap();
        a.conn_exec_for_tests(&format!(
            "INSERT INTO account_homes (id, account, post) VALUES ({}, '{}', 5)",
            crate::db::next_id("account_homes"),
            phone.id()
        ))
        .unwrap();
        assert!(a.officine_accounts().unwrap().is_empty());
        a.keep_account_records(&[compte::write_profile(
            &phone,
            2,
            &profile("Julien M."),
            &mut bpm_sync::OsEntropy,
        )
        .unwrap()])
            .unwrap();
        net.publish(&a, "P").unwrap();
        assert_eq!(published(&net), 1);
    }

    /// **Une officine écartée n'est plus lue au réseau ouvert**, et ne s'y
    /// inscrit plus.
    #[test]
    fn a_set_aside_stranger_is_not_read_on_the_open_network() {
        let (_dir_a, _sa, a) = officine("open-aside");
        join_open(&a, "2026-10-16").unwrap();
        let stranger = "ab".repeat(32);
        set_ignored(&a, &open_id(), &stranger, true, "2026-10-16").unwrap();
        assert!(!add_open_member(&a, &stranger, "2026-10-16", true).unwrap());
        assert!(add_open_member(&a, &"cd".repeat(32), "2026-10-16", true).unwrap());
        assert!(!add_open_member(&a, "pas une clé", "2026-10-16", true).unwrap());
    }

    /// **La liste du réseau ouvert ne se remplit pas pour de bon** : les
    /// auteurs relayés n'en prennent que la moitié, l'autre reste aux
    /// officines rencontrées ; une officine qui se tait trois mois en sort.
    #[test]
    fn the_open_list_keeps_room_for_officines_met_and_forgets_the_quiet() {
        let (_dir_a, _sa, a) = officine("open-room");
        join_open(&a, "2026-10-16").unwrap();
        let key = |i: usize| format!("{:064x}", i + 1);
        for i in 0..MOST_OPEN_MEMBERS / 2 {
            assert!(add_open_member(&a, &key(i), "2026-10-16", false).unwrap());
        }
        assert!(
            !add_open_member(&a, &key(9999), "2026-10-16", false).unwrap(),
            "relayée"
        );
        assert!(
            add_open_member(&a, &key(9999), "2026-10-16", true).unwrap(),
            "rencontrée"
        );
        assert_eq!(forget_quiet_open_members(&a, "2026-12-01").unwrap(), 0);
        let n = MOST_OPEN_MEMBERS / 2 + 1;
        assert_eq!(forget_quiet_open_members(&a, "2027-02-01").unwrap(), n);
        assert!(a.net_members(&open_id()).unwrap().is_empty());
        assert!(a.net_peers().unwrap().is_empty(), "et du carnet");
        // Oubliées, leurs enregistrements relayés ne les réinscrivent pas ;
        // une conversation directe, si.
        assert!(!add_open_member(&a, &key(0), "2027-02-01", false).unwrap());
        assert!(add_open_member(&a, &key(0), "2027-02-01", true).unwrap());
        // Sans date du jour lisible, rien n'est oublié.
        assert_eq!(forget_quiet_open_members(&a, "").unwrap(), 0);
        assert_eq!(a.net_members(&open_id()).unwrap().len(), 1);
    }

    #[test]
    fn an_announced_address_is_an_ip_and_a_port_nowhere_private_to_this_post() {
        assert_eq!(
            usable_announced(" 203.0.113.5:7745 "),
            Some("203.0.113.5:7745".to_owned())
        );
        assert_eq!(
            usable_announced("10.8.0.2:7745"),
            Some("10.8.0.2:7745".to_owned())
        );
        assert_eq!(
            usable_announced("[2001:db8::1]:7745"),
            Some("[2001:db8::1]:7745".to_owned())
        );
        for bad in [
            "exemple.fr:7745",
            "127.0.0.1:7745",
            "0.0.0.0:7745",
            "169.254.169.254:80",
            "224.0.0.1:7745",
            "255.255.255.255:7745",
            "203.0.113.5:0",
            "[::1]:7745",
            "[fe80::1]:7745",
            "[::ffff:127.0.0.1]:7745",
            "[::ffff:169.254.169.254]:80",
            "203.0.113.5",
            "",
        ] {
            assert_eq!(usable_announced(bad), None, "{bad}");
        }
    }

    /// **Une officine appairée ne parle que pour elle** : B, hostile,
    /// « corrige » la rupture de C et signe une rupture du nom de C. Chez A,
    /// qui a ajouté les deux, la rupture de C reste, et celle de B est
    /// de B.
    #[test]
    fn a_paired_officine_speaks_only_for_itself() {
        let (dir_a, _sa, a) = officine("speak-a");
        let (dir_b, _sb, b) = officine("speak-b");
        let (dir_c, _sc, c) = officine("speak-c");
        Net::create(&a).unwrap();
        drop((a, b, c));
        for dir in [&dir_b, &dir_c] {
            let (da, db_) = pair(&dir_a, dir, "");
            assert!(matches!(da, Some(Progress::Done(_))), "{da:?}");
            assert!(matches!(db_, Some(Progress::Done(_))), "{db_:?}");
        }
        let a = Db::open(&dir_a.join("net.db"), "secret").unwrap();
        let b = Db::open(&dir_b.join("net.db"), "secret").unwrap();
        let c = Db::open(&dir_c.join("net.db"), "secret").unwrap();
        let folder = dir_a.join("echange");
        c.add_supply_event(&subst("Diprosone", "Locoid")).unwrap();
        let mut net_c = Net::load(&c).unwrap();
        net_c.publish(&c, "Pharmacie C").unwrap();
        net_c.exchange_folder(&c, &folder).unwrap();
        let target = net_c
            .journal
            .records()
            .find(|r| r.author() == net_c.device.id())
            .map(|r| r.id())
            .unwrap();
        // B efface, et signe du nom de C.
        let mut net_b = Net::load(&b).unwrap();
        let t = net_b.trousseau.clone().unwrap();
        let mut fake = subst("Dafalgan", "Doliprane");
        fake.uid = "b-base:1".to_owned();
        let forged = crate::ruptures::encode(&fake, "Pharmacie C");
        net_b
            .journal
            .write(
                &net_b.device,
                &t,
                Stream::Reseau,
                b"{}",
                Some(target),
                &mut bpm_sync::OsEntropy,
            )
            .unwrap();
        net_b
            .journal
            .write(
                &net_b.device,
                &t,
                Stream::Reseau,
                &forged,
                None,
                &mut bpm_sync::OsEntropy,
            )
            .unwrap();
        net_b.exchange_folder(&b, &folder).unwrap();
        let mut net_a = Net::load(&a).unwrap();
        net_a.exchange_folder(&a, &folder).unwrap();
        net_a.absorb(&a).unwrap();
        let events = a.supply_events().unwrap();
        let dipro = crate::ruptures::tried(&events, "Diprosone");
        assert_eq!(dipro.len(), 1, "la rupture de C n'est pas effacée");
        let dafa = events
            .iter()
            .find(|e| e.product == "Dafalgan")
            .expect("reçue");
        assert_ne!(dafa.source, "Pharmacie C", "signée par B, pas par C");
    }
}
