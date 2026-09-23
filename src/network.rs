//! Le réseau d'officines : ce qu'une officine partage avec les autres
//! officines de son groupement — **les ruptures et ce qu'on a donné à la
//! place, et rien d'autre**.
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
//! source est son nom). Aucune fonction ici n'écrit dans une autre
//! table de la base que celles du réseau et le journal des ruptures.

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

/// Une officine appairée.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Peer {
    pub device: String,
    pub name: String,
    pub address: String,
    pub added: String,
}

impl Peer {
    /// Son empreinte, en cinq groupes — ce qu'on se lit au téléphone.
    pub fn groups(&self) -> String {
        unhex::<32>(&self.device)
            .map(|b| DeviceId::from_bytes(b).fingerprint().groups())
            .unwrap_or_default()
    }
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
        let peers = db
            .net_peers()?
            .into_iter()
            .map(|(device, name, address, added)| Peer {
                device,
                name,
                address,
                added,
            })
            .collect();
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

    /// Converser avec une officine appairée qui a une porte ouverte.
    pub fn sync_with(&mut self, db: &Db, address: &str, patience: Duration) -> Result<(), String> {
        let Some(trousseau) = self.trousseau.clone() else {
            return Ok(());
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
        Ok(())
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
                if net.sync_with(&db, &p.address, TALK_PATIENCE).is_err() {
                    failed.push(if p.name.trim().is_empty() {
                        p.address.clone()
                    } else {
                        p.name.clone()
                    });
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
    }
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

    /// **Deux officines, un dossier d'échange, et ce qui en sort.**
    /// L'une note une substitution ; l'autre la lit, sous le nom de la
    /// première. Le dossier ne contient que des octets scellés — ni le
    /// produit ni le nom de l'officine n'y sont lisibles —, et une
    /// officine qui n'est pas appairée n'y est pas lue.
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

        let mut na = Net::load(&a).unwrap();
        assert_eq!(na.publish(&a, "Pharmacie du Centre").unwrap(), 2);
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
        assert_eq!(nb.exchange_folder(&b, &folder).unwrap(), 2);
        assert_eq!(nb.absorb(&b).unwrap(), 1);
        let tried = crate::ruptures::tried(&b.supply_events().unwrap(), "Diprosone");
        assert_eq!(tried.len(), 1);
        assert_eq!(tried[0].other, "Locoid");
        let theirs = b.supply_events().unwrap();
        assert_eq!(theirs[0].source, "Pharmacie du Centre");
        let facts = b.net_facts_for("eliquis").unwrap();
        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].officine, "Pharmacie du Centre");
        assert_eq!(facts[0].fact.low, Some(87.0));
        // Relu : rien de neuf.
        assert_eq!(nb.absorb(&b).unwrap(), 0);

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
