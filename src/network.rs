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

/// L'empreinte d'une officine appairée, en cinq groupes — ce qu'on se
/// lit au téléphone.
pub fn peer_groups(device: &str) -> String {
    unhex::<32>(device)
        .map(|b| DeviceId::from_bytes(b).fingerprint().groups())
        .unwrap_or_default()
}

/// Ce que cette officine sait du réseau, lu dans la base.
pub struct Net {
    device: Device,
    trousseau: Option<Trousseau>,
    journal: Journal,
    pub peers: Vec<Peer>,
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
        let mut journal = Journal::new();
        for bytes in db.net_records()? {
            if let Ok(record) = Record::decode(&bytes) {
                journal.insert(record);
            }
        }
        let peers = db.net_peers()?;
        Ok(Self {
            device: Device::from_seed(seed),
            trousseau,
            journal,
            peers,
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
        let pending = db.unpublished_supply_events()?;
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
        let already_versions = db.published()?;
        for e in db
            .card_edits()?
            .into_iter()
            .filter(|e| e.source.is_empty() && !already_versions.contains(&e.uid))
        {
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
        let already = db.published()?;
        for (name, dci, fact) in db.all_drug_facts()? {
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
        for m in db.outgoing_net_messages()? {
            let key = format!("msg:{}", m.uid);
            if already.contains(&key) {
                continue;
            }
            let Some(mut recipients) = m
                .peers
                .iter()
                .map(|p| keys.get(p).copied())
                .collect::<Option<Vec<[u8; 32]>>>()
            else {
                continue;
            };
            recipients.push(mine);
            let mut all_peers = m.peers.clone();
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
        db.mark_published(&done)?;
        Ok(done.len())
    }

    /// Ranger dans la base les enregistrements que le journal tient.
    fn keep(&self, db: &Db) -> Result<usize, String> {
        let all: Vec<(String, Vec<u8>)> = self
            .journal
            .records()
            .map(|r| (hex(&r.id().0), r.encode()))
            .collect();
        db.keep_net_records(&all)
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
        let facts: Vec<bpm_sync::Fact> = self
            .journal
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
            db.note_net_heard(
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
        let name = hex(&me.0[..10]);
        let target = folder.join(format!("{name}.bpmnet"));
        let part = folder.join(format!("{name}.bpmnet.part"));
        std::fs::write(&part, &mine).map_err(|e| e.to_string())?;
        std::fs::rename(&part, &target).map_err(|e| e.to_string())?;
        let known = self.known();
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
                // Signé par son auteur, et d'une officine appairée : le
                // reste du dossier n'est lu par personne.
                if let Ok(record) = Record::decode(chunk) {
                    if known.contains(&record.author()) && self.journal.insert(record) {
                        added += 1;
                    }
                }
            }
        }
        self.keep(db)?;
        Ok(added)
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
    /// Fini : ce qui s'est passé, en une phrase.
    Done(String),
    Failed(String),
}

/// Ce qu'on demande au fil du réseau.
#[derive(Clone, Debug)]
pub enum Job {
    /// Ouvrir une porte le temps d'une invitation.
    Invite { port: u16 },
    /// Rejoindre un réseau en composant l'adresse de l'officine qui
    /// invite.
    Join { address: String },
    /// Synchroniser : le dossier d'échange s'il y en a un, puis chaque
    /// officine qui a une adresse.
    Sync { folder: Option<PathBuf> },
    /// Composer l'adresse d'**une** officine : savoir si elle répond,
    /// sans attendre toutes les autres ni le dossier d'échange.
    Dial { device: String },
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
    let mut net = Net::load(&db)?;
    let mut confirm = |code: bpm_sync::Fingerprint| {
        let _ = tx.send(Progress::Code(code.groups()));
        answers.recv().unwrap_or(false)
    };
    match job {
        Job::Invite { port } => {
            let trousseau = net
                .trousseau
                .clone()
                .ok_or_else(|| tr("net_err_no_network").to_owned())?;
            let door = bpm_sync::link::Door::open(&format!("0.0.0.0:{port}"))
                .map_err(|_| tr("net_err_port").to_owned())?;
            let _ = tx.send(Progress::Waiting(format!("{}:{port}", local_address())));
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
                db.add_net_peer(&hex(&peer.0), "", today)?;
            }
            net.keep(&db)?;
            Ok(tr("net_done_invited").to_owned())
        }
        Job::Join { address } => {
            if net.in_network() {
                return Err(tr("net_err_already").to_owned());
            }
            let mut link = bpm_sync::link::dial(address, TALK_PATIENCE)
                .map_err(|_| tr("net_err_unreachable").to_owned())?;
            let mut session = Session::new(&net.device, None, Intent::Join, true, &[])
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
            let joined = session
                .joined()
                .cloned()
                .ok_or_else(|| tr("net_err_refused").to_owned())?;
            db.net_key("net_trousseau", &hex(&joined.secret()))?;
            if let Some(peer) = session.peer() {
                db.add_net_peer(&hex(&peer.0), address, today)?;
            }
            net.keep(&db)?;
            let net = Net::load(&db)?;
            let received = net.absorb(&db)?;
            Ok(trn("net_done_joined", &[&received]))
        }
        Job::Sync { folder } => {
            if !net.in_network() {
                return Err(tr("net_err_no_network").to_owned());
            }
            let sent = net.publish(&db, officine)?;
            let mut failed = Vec::new();
            if let Some(folder) = folder {
                if let Err(e) = net.exchange_folder(&db, folder) {
                    failed.push(format!("{} : {e}", folder.display()));
                }
            }
            let peers = net.peers.clone();
            for p in peers.iter().filter(|p| !p.address.trim().is_empty()) {
                // Chaque conversation est notée, réussie ou non, avec sa
                // raison : « injoignable » sans date ni cause ne dit pas
                // si c'est la box d'en face ou celle d'ici.
                // Une note qui ne s'écrit pas n'arrête pas les autres
                // conversations : c'est un état affiché, pas une donnée.
                match net.sync_with(&db, &p.address, TALK_PATIENCE) {
                    Ok(answered) => {
                        let _ = db.note_net_dial(answered.as_deref().unwrap_or(&p.device), None);
                    }
                    Err(e) => {
                        let _ = db.note_net_dial(&p.device, Some(&dial_reason(&e)));
                        failed.push(if p.name.trim().is_empty() {
                            p.address.clone()
                        } else {
                            p.name.clone()
                        });
                    }
                }
            }
            let received = net.absorb(&db)?;
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
            if !net.in_network() {
                return Err(tr("net_err_no_network").to_owned());
            }
            let Some(p) = net
                .peers
                .iter()
                .find(|p| p.device == *device && !p.address.trim().is_empty())
                .cloned()
            else {
                return Err(tr("net_err_no_address").to_owned());
            };
            let name = if p.name.trim().is_empty() {
                p.address.clone()
            } else {
                p.name.clone()
            };
            let sent = net.publish(&db, officine)?;
            match net.sync_with(&db, &p.address, TALK_PATIENCE) {
                Ok(answered) => {
                    let answered = answered.unwrap_or_else(|| p.device.clone());
                    let _ = db.note_net_dial(&answered, None);
                    let received = net.absorb(&db)?;
                    if answered == p.device {
                        return Ok(trn("net_done_dialed", &[&name, &sent, &received]));
                    }
                    // Une autre officine du réseau répond à cette adresse
                    // (un bail DHCP qui a changé) : l'échange a eu lieu,
                    // mais celle qu'on composait n'a pas répondu.
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
            Job::Invite { port },
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
}
