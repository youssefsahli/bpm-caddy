//! Les postes d'une officine : **chacun sa base, et toutes les données
//! de l'officine de l'une à l'autre** — dossiers, registre, caisse,
//! planning, agenda, fiches, réglages.
//!
//! Construit sur `bpm-sync` (voir `docs/SYNC.md`) et sur `replica.rs`,
//! qui dit ce qui voyage et comment une écriture reçue s'applique. Trois
//! choix en font l'outil des postes et non celui du réseau d'officines :
//!
//! * **La clé des postes n'est qu'à eux.** Elle scelle les sept flux de
//!   l'officine et elle est donnée à un poste le temps d'un appairage,
//!   code de cinq groupes lu des deux côtés. Le réseau d'officines a la
//!   sienne, qui n'ouvre que son flux : un patient ne passe jamais par
//!   là, parce qu'aucune de ses lignes n'est scellée sous cette clé-là.
//! * **Un poste qui rejoint reçoit tout** : la photographie que le poste
//!   fondateur a scellée, puis chaque écriture depuis. Ce qu'il contenait
//!   avant est remplacé — deux bases qui ont vécu chacune de leur côté ne
//!   se fusionnent pas, leurs numéros de dossier se chevauchent.
//! * **Trois chemins, au choix du poste** : sur le réseau local, tout
//!   seul, quelques secondes après chaque écriture (une annonce que les
//!   postes s'envoient, une porte que chacun tient ouverte) ; par un
//!   dossier d'échange ; ou sur un bouton et à la fermeture. Ce qui
//!   traverse est scellé et signé : qui le transporte ne lit rien.

use crate::db::{Applied, Db};
use crate::replica::Flux;
use bpm_sync::{Device, DeviceId, Intent, Journal, Meter, Record, Session, Stream, Trousseau};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Le port proposé aux postes, quand ce poste n'en a pas écrit.
pub const DEFAULT_PORT: u16 = 7743;

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

/// Le flux `bpm-sync` d'un flux de l'officine.
pub fn stream(flux: Flux) -> Stream {
    match flux {
        Flux::Dossiers => Stream::Dossiers,
        Flux::Registre => Stream::Registre,
        Flux::Caisse => Stream::Caisse,
        Flux::Planning => Stream::Planning,
        Flux::Agenda => Stream::Agenda,
        Flux::Officine => Stream::Officine,
        Flux::Fiches => Stream::Fiches,
    }
}

/// Les flux que les postes lisent : tous ceux de l'officine, jamais
/// celui du réseau d'officines.
const STREAMS: [Stream; 7] = [
    Stream::Dossiers,
    Stream::Registre,
    Stream::Caisse,
    Stream::Planning,
    Stream::Agenda,
    Stream::Officine,
    Stream::Fiches,
];

/// L'empreinte d'une identité en hexadécimal, en cinq groupes.
pub fn groups_of(device: &str) -> String {
    unhex::<32>(device)
        .map(|b| DeviceId::from_bytes(b).fingerprint().groups())
        .unwrap_or_default()
}

/// Ce qu'une synchronisation a fait, pour l'écran.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Report {
    pub sent: usize,
    pub received: Applied,
    /// Des enregistrements que la clé en main n'ouvre pas — nommés, pas
    /// sautés.
    pub unopened: usize,
}

/// Ce que ce poste sait des postes, lu dans la base.
pub struct Posts {
    device: Device,
    trousseau: Option<Trousseau>,
    journal: Journal,
}

impl Posts {
    /// Lire l'identité de ce poste, la clé des postes s'il en a une, et
    /// le journal. L'identité est tirée au hasard la première fois, **dans
    /// la base chiffrée**.
    pub fn load(db: &Db) -> Result<Self, String> {
        let seed = match db.sync_local("device") {
            Some(s) => s,
            None => {
                let s = hex(&random32());
                db.set_sync_local("device", &s)?;
                s
            }
        };
        let seed = unhex::<32>(&seed).ok_or("identité du poste illisible")?;
        let trousseau = db
            .sync_local("trousseau")
            .and_then(|t| unhex::<32>(&t))
            .map(Trousseau::from_secret);
        let mut journal = Journal::new();
        for bytes in db.sync_records()? {
            if let Ok(record) = Record::decode(&bytes) {
                journal.insert(record);
            }
        }
        Ok(Self {
            device: Device::from_seed(seed),
            trousseau,
            journal,
        })
    }

    pub fn in_group(&self) -> bool {
        self.trousseau.is_some()
    }

    /// L'identité de ce poste, en hexadécimal — ce que `sync_posts` porte.
    pub fn device_hex(&self) -> String {
        hex(&self.device.id().0)
    }

    /// L'empreinte de ce poste, en cinq groupes.
    pub fn groups(&self) -> String {
        self.device.id().fingerprint().groups()
    }

    /// Le nom du groupe : l'empreinte de sa clé, la même sur tous ses
    /// postes.
    pub fn group_groups(&self) -> Option<String> {
        self.trousseau.as_ref().map(|t| t.name().groups())
    }

    pub fn record_count(&self) -> usize {
        self.journal.len()
    }

    /// Les postes que ce poste écoute : ceux du groupe qui n'ont pas été
    /// retirés, et ceux qu'il vient d'admettre et qui ne se sont pas
    /// encore présentés.
    fn known(&self, db: &Db) -> Vec<DeviceId> {
        let me = self.device.id();
        let mut out: Vec<DeviceId> = db
            .sync_posts()
            .unwrap_or_default()
            .into_iter()
            .filter(|p| p.left_on.is_empty())
            .filter_map(|p| unhex::<32>(&p.device).map(DeviceId::from_bytes))
            .collect();
        for d in db.sync_local("admitted").unwrap_or_default().split(',') {
            if let Some(b) = unhex::<32>(d) {
                out.push(DeviceId::from_bytes(b));
            }
        }
        out.retain(|d| *d != me);
        out.sort_by_key(|d| d.0);
        out.dedup();
        out
    }

    /// Ceux que l'officine a retirés : ce qu'ils écrivent n'est plus
    /// rangé.
    fn retired(db: &Db) -> Vec<DeviceId> {
        db.sync_posts()
            .unwrap_or_default()
            .into_iter()
            .filter(|p| !p.left_on.is_empty())
            .filter_map(|p| unhex::<32>(&p.device).map(DeviceId::from_bytes))
            .collect()
    }

    fn admit(db: &Db, device: &DeviceId) -> Result<(), String> {
        let mut list: Vec<String> = db
            .sync_local("admitted")
            .unwrap_or_default()
            .split(',')
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect();
        let d = hex(&device.0);
        if !list.contains(&d) {
            list.push(d);
        }
        db.set_sync_local("admitted", &list.join(","))
    }

    /// Fonder le groupe sur cette base : une clé neuve, et la
    /// photographie de tout ce qu'elle contient scellée au journal — ce
    /// que le premier poste qui rejoindra recevra.
    pub fn found(&mut self, db: &Db, name: &str, day: &str) -> Result<usize, String> {
        if self.in_group() || db.sync_post().is_some() {
            return Err(crate::strings::tr("posts_err_already").to_owned());
        }
        let secret = random32();
        let snapshot = db.found_posts(&self.device_hex(), name, day)?;
        db.set_sync_local("trousseau", &hex(&secret))?;
        self.trousseau = Some(Trousseau::from_secret(secret));
        let sent = self.seal(db, &snapshot, day)?;
        // What the founding itself captured is inside the snapshot.
        let (_, upto) = db.pending_ops()?;
        db.clear_pending(upto)?;
        Ok(sent)
    }

    fn seal(&mut self, db: &Db, ops: &[crate::replica::Op], day: &str) -> Result<usize, String> {
        let Some(trousseau) = self.trousseau.clone() else {
            return Ok(0);
        };
        let mut written = 0;
        let mut start = 0;
        while start < ops.len() {
            let flux = crate::replica::table(ops[start].file, &ops[start].table)
                .map(|t| t.flux)
                .unwrap_or(Flux::Officine);
            let mut end = start;
            while end < ops.len()
                && crate::replica::table(ops[end].file, &ops[end].table)
                    .map(|t| t.flux)
                    .unwrap_or(Flux::Officine)
                    == flux
            {
                end += 1;
            }
            let (lots, refused) = crate::replica::batches(&ops[start..end], crate::replica::LIMIT);
            db.note_oversize(&refused, day)?;
            for lot in lots {
                let id = self
                    .journal
                    .write(
                        &self.device,
                        &trousseau,
                        stream(flux),
                        &lot,
                        None,
                        &mut bpm_sync::OsEntropy,
                    )
                    .map_err(|e| format!("{e:?}"))?;
                // Written here: nothing to range here.
                db.mark_sync_applied(&hex(&id.0))?;
                written += 1;
            }
            start = end;
        }
        self.keep(db)?;
        Ok(written)
    }

    /// Sceller au journal les écritures de ce poste pas encore parties.
    /// Rend combien d'enregistrements.
    pub fn publish(&mut self, db: &Db, day: &str) -> Result<usize, String> {
        if !self.in_group() {
            return Ok(0);
        }
        let (ops, upto) = db.pending_ops()?;
        let sent = self.seal(db, &ops, day)?;
        db.clear_pending(upto)?;
        Ok(sent)
    }

    fn keep(&self, db: &Db) -> Result<usize, String> {
        let all: Vec<(String, Vec<u8>)> = self
            .journal
            .records()
            .map(|r| (hex(&r.id().0), r.encode()))
            .collect();
        db.keep_sync_records(&all)
    }

    /// Ranger dans la base ce que les autres postes ont écrit, **dans
    /// l'ordre du journal** — le même sur tous les postes. Une fois par
    /// enregistrement.
    pub fn absorb(&self, db: &Db, day: &str) -> Result<Report, String> {
        let mut report = Report::default();
        let Some(trousseau) = &self.trousseau else {
            return Ok(report);
        };
        let me = self.device.id();
        let retired = Self::retired(db);
        let mut facts: Vec<bpm_sync::Fact> = Vec::new();
        for s in STREAMS {
            let reading = self.journal.read(trousseau, s);
            report.unopened += reading.unopened.len();
            facts.extend(reading.facts);
        }
        facts.sort_by_key(|f| (f.lamport, f.author.0, f.id.0));
        for f in facts {
            let id = hex(&f.id.0);
            if db.sync_applied(&id) {
                continue;
            }
            if f.author == me {
                db.mark_sync_applied(&id)?;
                continue;
            }
            if retired.contains(&f.author) {
                continue;
            }
            let Some(ops) = crate::replica::decode(&f.payload) else {
                db.mark_sync_applied(&id)?;
                continue;
            };
            let got = db.apply_record(&id, &f.author.fingerprint().groups(), day, &ops)?;
            report.received.written += got.written;
            report.received.conflicts += got.conflicts;
            report.received.refused += got.refused;
            report.received.duplicates += got.duplicates;
        }
        // The reference post numbers what arrived without a number; the
        // numbers leave with its next publication.
        db.number_pending(day)?;
        Ok(report)
    }

    /// Déposer ses enregistrements dans le dossier d'échange, et lire ceux
    /// des autres postes. Un fichier par poste, écrit à côté puis mis en
    /// place. N'entre que ce que la clé des postes ouvre.
    pub fn exchange_folder(&mut self, db: &Db, folder: &Path) -> Result<usize, String> {
        let Some(trousseau) = self.trousseau.clone() else {
            return Ok(0);
        };
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
        let target = folder.join(format!("{name}.bpmposte"));
        let part = folder.join(format!("{name}.bpmposte.part"));
        std::fs::write(&part, &mine).map_err(|e| e.to_string())?;
        std::fs::rename(&part, &target).map_err(|e| e.to_string())?;
        let mut added = 0;
        for entry in std::fs::read_dir(folder)
            .map_err(|e| e.to_string())?
            .flatten()
        {
            let path = entry.path();
            if path == target || path.extension().and_then(|e| e.to_str()) != Some("bpmposte") {
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
                if let Ok(record) = Record::decode(chunk) {
                    if !self.journal.has(&record.id())
                        && record.open(&trousseau).is_ok()
                        && self.journal.insert(record)
                    {
                        added += 1;
                    }
                }
            }
        }
        self.keep(db)?;
        Ok(added)
    }

    /// Converser avec un poste qui a une porte ouverte.
    pub fn sync_with(&mut self, db: &Db, address: &str, patience: Duration) -> Result<(), String> {
        let Some(trousseau) = self.trousseau.clone() else {
            return Ok(());
        };
        let mut link = bpm_sync::link::dial(address, patience).map_err(|e| format!("{e:?}"))?;
        self.talk(db, &trousseau, &mut link, true)
    }

    fn talk<L: bpm_sync::Link>(
        &mut self,
        db: &Db,
        trousseau: &Trousseau,
        link: &mut L,
        initiator: bool,
    ) -> Result<(), String> {
        let mut session = Session::new(
            &self.device,
            Some(trousseau),
            Intent::Sync,
            initiator,
            &self.known(db),
        )
        .map_err(|e| format!("{e:?}"))?;
        let mut meter = Meter::new();
        bpm_sync::drive(
            &mut session,
            link,
            &mut self.journal,
            &mut meter,
            &mut |_| true,
        )
        .map_err(|e| format!("{e:?}"))?;
        self.keep(db)?;
        Ok(())
    }
}

/// Ce que le fil des postes dit à l'écran.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Progress {
    /// Une porte est ouverte à cette adresse, en attente de l'autre poste.
    Waiting(String),
    /// Le code que les deux postes comparent.
    Code(String),
    /// Où en est la synchronisation automatique, en une phrase.
    Status(String),
    Done(String),
    Failed(String),
}

/// Ce qu'on demande au fil des postes.
#[derive(Clone, Debug)]
pub enum Job {
    /// Fonder le groupe sur cette base.
    Found { name: String },
    /// Ouvrir une porte le temps d'une invitation.
    Invite { port: u16 },
    /// Rejoindre le groupe en composant l'adresse du poste qui invite.
    /// **Ce que ce poste contenait est remplacé.**
    Join { address: String, name: String },
    /// Synchroniser une fois : le dossier d'échange, puis chaque adresse.
    Sync {
        folder: Option<PathBuf>,
        addresses: Vec<String>,
    },
}

const INVITE_PATIENCE: Duration = Duration::from_secs(300);
const TALK_PATIENCE: Duration = Duration::from_secs(8);

/// Lancer une tâche sur son propre fil, avec sa propre connexion à la
/// base. `answers` porte la réponse de l'écran au code affiché.
pub fn spawn(
    job: Job,
    path: PathBuf,
    password: String,
    today: String,
    answers: std::sync::mpsc::Receiver<bool>,
) -> std::sync::mpsc::Receiver<Progress> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let said = run(&job, &path, &password, &today, &tx, &answers);
        let _ = tx.send(match said {
            Ok(done) => Progress::Done(done),
            Err(e) => Progress::Failed(e),
        });
    });
    rx
}

fn said_synced(report: &Report) -> String {
    use crate::strings::{trf, trn};
    let mut said = trn(
        "posts_done_synced",
        &[&report.sent, &report.received.written],
    );
    if report.received.conflicts > 0 {
        said.push_str(&trf("posts_done_questions", report.received.conflicts));
    }
    if report.received.refused > 0 {
        said.push_str(&trf("posts_done_refused", report.received.refused));
    }
    if report.unopened > 0 {
        said.push_str(&trf("posts_done_unopened", report.unopened));
    }
    said
}

/// Ce que fait le fil. Public pour la synchronisation à la fermeture.
pub fn run(
    job: &Job,
    path: &Path,
    password: &str,
    today: &str,
    tx: &std::sync::mpsc::Sender<Progress>,
    answers: &std::sync::mpsc::Receiver<bool>,
) -> Result<String, String> {
    use crate::strings::{tr, trf, trn};
    let db = Db::open(path, password)?;
    let mut posts = Posts::load(&db)?;
    let mut confirm = |code: bpm_sync::Fingerprint| {
        let _ = tx.send(Progress::Code(code.groups()));
        answers.recv().unwrap_or(false)
    };
    match job {
        Job::Found { name } => {
            let n = posts.found(&db, name, today)?;
            Ok(trn("posts_done_founded", &[&n]))
        }
        Job::Invite { port } => {
            let trousseau = posts
                .trousseau
                .clone()
                .ok_or_else(|| tr("posts_err_no_group").to_owned())?;
            posts.publish(&db, today)?;
            let door = bpm_sync::link::Door::open(&format!("0.0.0.0:{port}"))
                .map_err(|_| tr("posts_err_port").to_owned())?;
            let _ = tx.send(Progress::Waiting(format!("{}:{port}", local_address())));
            let mut link = door
                .accept(INVITE_PATIENCE)
                .map_err(|_| tr("posts_err_nobody").to_owned())?;
            let mut session = Session::new(
                &posts.device,
                Some(&trousseau),
                Intent::Invite,
                false,
                &posts.known(&db),
            )
            .map_err(|e| format!("{e:?}"))?;
            let mut meter = Meter::new();
            bpm_sync::drive(
                &mut session,
                &mut link,
                &mut posts.journal,
                &mut meter,
                &mut confirm,
            )
            .map_err(|_| tr("posts_err_refused").to_owned())?;
            if let Some(peer) = session.peer() {
                Posts::admit(&db, &peer)?;
            }
            posts.keep(&db)?;
            Ok(tr("posts_done_invited").to_owned())
        }
        Job::Join { address, name } => {
            if posts.in_group() || db.sync_post().is_some() {
                return Err(tr("posts_err_already").to_owned());
            }
            // A fresh identity: a base copied from another post carries
            // that post's, and two posts under one name would each take
            // the other's records for their own.
            db.set_sync_local("device", &hex(&random32()))?;
            let mut posts = Posts::load(&db)?;
            let mut link = bpm_sync::link::dial(address, TALK_PATIENCE)
                .map_err(|_| tr("posts_err_unreachable").to_owned())?;
            let mut session = Session::new(&posts.device, None, Intent::Join, true, &[])
                .map_err(|e| format!("{e:?}"))?;
            let mut meter = Meter::new();
            bpm_sync::drive(
                &mut session,
                &mut link,
                &mut posts.journal,
                &mut meter,
                &mut confirm,
            )
            .map_err(|_| tr("posts_err_refused").to_owned())?;
            let joined = session
                .joined()
                .cloned()
                .ok_or_else(|| tr("posts_err_refused").to_owned())?;
            // Only now, with the key and the journal in hand, is what this
            // post held replaced.
            db.prepare_join()?;
            db.set_sync_local("trousseau", &hex(&joined.secret()))?;
            posts.trousseau = Some(joined);
            posts.keep(&db)?;
            let report = posts.absorb(&db, today)?;
            let post = db.finish_join(&posts.device_hex(), name, today)?;
            posts.publish(&db, today)?;
            // Tell the inviting post at once who joined, so it hears this
            // post's next records without waiting for anybody.
            let _ = posts.sync_with(&db, address, TALK_PATIENCE);
            Ok(trn(
                "posts_done_joined",
                &[&(post + 1), &report.received.written],
            ))
        }
        Job::Sync { folder, addresses } => {
            if !posts.in_group() {
                return Err(tr("posts_err_no_group").to_owned());
            }
            let sent = posts.publish(&db, today)?;
            let mut failed = Vec::new();
            if let Some(folder) = folder {
                if let Err(e) = posts.exchange_folder(&db, folder) {
                    failed.push(format!("{} : {e}", folder.display()));
                }
            }
            for a in addresses.iter().filter(|a| !a.trim().is_empty()) {
                if posts.sync_with(&db, a.trim(), TALK_PATIENCE).is_err() {
                    failed.push(a.clone());
                }
            }
            let mut report = posts.absorb(&db, today)?;
            report.sent = sent;
            let mut said = said_synced(&report);
            if !failed.is_empty() {
                said.push_str(&trf("posts_done_unreached", failed.join(", ")));
            }
            Ok(said)
        }
    }
}

/// Ce que l'écran demande au fil automatique.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Poke {
    /// Synchroniser maintenant : quelqu'un a appuyé sur le bouton.
    Now,
    Stop,
}

/// Ce que le fil automatique sait des postes qu'il a entendus.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Heard {
    pub device: String,
    pub address: String,
}

/// L'annonce qu'un poste envoie sur le réseau local : son identité et
/// son port. Rien d'autre — ni le nom de l'officine, ni une donnée.
pub fn beacon(device: &str, port: u16) -> String {
    format!("BPMPOSTE1 {device} {port}")
}

/// L'inverse, avec l'adresse d'où l'annonce est venue.
pub fn heard(text: &str, from: std::net::IpAddr) -> Option<Heard> {
    let mut parts = text.split_whitespace();
    if parts.next()? != "BPMPOSTE1" {
        return None;
    }
    let device = parts.next()?;
    unhex::<32>(device)?;
    let port: u16 = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some(Heard {
        device: device.to_owned(),
        address: std::net::SocketAddr::new(from, port).to_string(),
    })
}

/// Combien d'attente entre deux choses que le fil automatique fait
/// d'office.
pub struct Pace {
    pub beacon: Duration,
    pub talk: Duration,
    pub folder: Duration,
}

impl Default for Pace {
    fn default() -> Self {
        Self {
            beacon: Duration::from_secs(3),
            talk: Duration::from_secs(20),
            folder: Duration::from_secs(60),
        }
    }
}

/// La synchronisation automatique, **tant que l'application est
/// ouverte** : une porte tenue ouverte aux postes du groupe, une annonce
/// toutes les quelques secondes, et une conversation avec chaque poste
/// entendu dès que ce poste a écrit quelque chose — ou toutes les vingt
/// secondes sinon. Le dossier d'échange, s'il y en a un, chaque minute.
/// S'arrête sur [`Poke::Stop`] ou quand l'écran disparaît.
#[allow(clippy::too_many_arguments)]
pub fn spawn_auto(
    path: PathBuf,
    password: String,
    today: String,
    port: u16,
    folder: Option<PathBuf>,
    addresses: Vec<String>,
    pace: Pace,
) -> (
    std::sync::mpsc::Receiver<Progress>,
    std::sync::mpsc::Sender<Poke>,
) {
    let (tx, rx) = std::sync::mpsc::channel();
    let (poke_tx, poke_rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        if let Err(e) = auto(
            &path, &password, &today, port, folder, &addresses, &pace, &tx, &poke_rx,
        ) {
            let _ = tx.send(Progress::Failed(e));
        }
    });
    (rx, poke_tx)
}

#[allow(clippy::too_many_arguments)]
fn auto(
    path: &Path,
    password: &str,
    today: &str,
    port: u16,
    folder: Option<PathBuf>,
    addresses: &[String],
    pace: &Pace,
    tx: &std::sync::mpsc::Sender<Progress>,
    pokes: &std::sync::mpsc::Receiver<Poke>,
) -> Result<(), String> {
    use crate::strings::tr;
    let db = Db::open(path, password)?;
    let mut posts = Posts::load(&db)?;
    let Some(trousseau) = posts.trousseau.clone() else {
        return Err(tr("posts_err_no_group").to_owned());
    };
    let door = bpm_sync::link::Door::open(&format!("0.0.0.0:{port}")).ok();
    if door.is_none() {
        let _ = tx.send(Progress::Status(tr("posts_auto_no_door").to_owned()));
    }
    let udp = std::net::UdpSocket::bind(("0.0.0.0", port)).ok();
    if let Some(u) = &udp {
        let _ = u.set_broadcast(true);
        let _ = u.set_nonblocking(true);
    }
    let me = posts.device_hex();
    let mut seen: Vec<Heard> = Vec::new();
    let long_ago = Instant::now()
        .checked_sub(Duration::from_secs(3600))
        .unwrap_or_else(Instant::now);
    let (mut last_beacon, mut last_talk, mut last_folder) = (long_ago, long_ago, long_ago);
    loop {
        let mut now_asked = false;
        match pokes.try_recv() {
            Ok(Poke::Stop) | Err(std::sync::mpsc::TryRecvError::Disconnected) => break,
            Ok(Poke::Now) => now_asked = true,
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
        }
        let mut talked = false;
        // A post knocking at our door.
        if let Some(door) = &door {
            if let Ok(mut link) = door.accept(Duration::from_millis(400)) {
                if posts.talk(&db, &trousseau, &mut link, false).is_ok() {
                    talked = true;
                }
            }
        } else {
            std::thread::sleep(Duration::from_millis(400));
        }
        // Who announced themselves.
        if let Some(u) = &udp {
            let mut buf = [0u8; 256];
            while let Ok((n, from)) = u.recv_from(&mut buf) {
                if let Some(h) = std::str::from_utf8(&buf[..n])
                    .ok()
                    .and_then(|t| heard(t, from.ip()))
                {
                    if h.device != me {
                        seen.retain(|x| x.device != h.device);
                        seen.push(h);
                    }
                }
            }
            if last_beacon.elapsed() >= pace.beacon {
                let _ = u.send_to(beacon(&me, port).as_bytes(), ("255.255.255.255", port));
                last_beacon = Instant::now();
            }
        }
        let pending = db
            .pending_ops()
            .map(|(o, _)| !o.is_empty())
            .unwrap_or(false);
        if pending || now_asked || talked || last_talk.elapsed() >= pace.talk {
            let sent = posts.publish(&db, today)?;
            if sent > 0 || now_asked || last_talk.elapsed() >= pace.talk {
                let known: Vec<String> = posts.known(&db).iter().map(|d| hex(&d.0)).collect();
                let mut targets: Vec<String> = seen
                    .iter()
                    .filter(|h| known.contains(&h.device))
                    .map(|h| h.address.clone())
                    .collect();
                targets.extend(addresses.iter().filter(|a| !a.trim().is_empty()).cloned());
                targets.dedup();
                for t in targets {
                    if posts.sync_with(&db, &t, TALK_PATIENCE).is_ok() {
                        talked = true;
                    }
                }
                last_talk = Instant::now();
            }
        }
        if let Some(f) = &folder {
            if now_asked || last_folder.elapsed() >= pace.folder {
                let _ = posts.exchange_folder(&db, f);
                last_folder = Instant::now();
                talked = true;
            }
        }
        if talked || now_asked {
            let report = posts.absorb(&db, today)?;
            if report.received.written > 0 || report.received.conflicts > 0 || now_asked {
                let _ = tx.send(Progress::Status(said_synced(&report)));
            }
        }
    }
    Ok(())
}

/// L'adresse de ce poste sur le réseau local — un `connect` UDP ne
/// transmet rien, il demande seulement par quelle interface on enverrait.
pub fn local_address() -> String {
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

    fn post(tag: &str) -> (PathBuf, crate::db::Swept, Db) {
        let dir =
            std::env::temp_dir().join(format!("bpm-caddy-postes-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let swept = crate::db::Swept(dir.clone());
        let db = Db::open(&dir.join("poste.db"), "secret").unwrap();
        (dir, swept, db)
    }

    /// **Deux postes, une porte, le code des deux côtés** : B rejoint le
    /// groupe de A et reçoit ses dossiers ; ce que B avait est remplacé ;
    /// chacun écrit, et un dossier d'échange porte le reste. Rien de
    /// lisible dans le dossier.
    #[test]
    fn a_post_joins_through_a_door_receives_everything_and_both_keep_writing() {
        let (dir_a, _sa, a) = post("a");
        let (dir_b, _sb, b) = post("b");
        let file = a.add_patient("Dupont", "Jean", "1958-07-03").unwrap();
        b.add_patient("Ancien", "Local", "1990-01-01").unwrap();
        Posts::load(&a)
            .unwrap()
            .found(&a, "Comptoir 1", "2026-09-23")
            .unwrap();
        drop(a);
        drop(b);
        let port = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let (yes_a, answers_a) = std::sync::mpsc::channel();
        let (yes_b, answers_b) = std::sync::mpsc::channel();
        let inviting = spawn(
            Job::Invite { port },
            dir_a.join("poste.db"),
            "secret".to_owned(),
            "2026-09-23".to_owned(),
            answers_a,
        );
        match inviting.recv().unwrap() {
            Progress::Waiting(_) => {}
            other => panic!("{other:?}"),
        }
        let joining = spawn(
            Job::Join {
                address: format!("127.0.0.1:{port}"),
                name: "Comptoir 2".to_owned(),
            },
            dir_b.join("poste.db"),
            "secret".to_owned(),
            "2026-09-23".to_owned(),
            answers_b,
        );
        let mut codes = Vec::new();
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
        assert_eq!(codes[0], codes[1]);

        let a = Db::open(&dir_a.join("poste.db"), "secret").unwrap();
        let b = Db::open(&dir_b.join("poste.db"), "secret").unwrap();
        assert_eq!(b.sync_post(), Some(1));
        let names: Vec<String> = b
            .patients()
            .unwrap()
            .into_iter()
            .map(|p| p.last_name)
            .collect();
        assert_eq!(
            names,
            vec!["Dupont".to_owned()],
            "B a reçu, et son ancien contenu est parti"
        );

        // Both write; a folder carries it.
        let folder = dir_a.join("echange");
        let from_b = b.add_patient("Grand", "Eve", "1980-03-03").unwrap();
        a.conn_for_tests()
            .execute("UPDATE patients SET phone = '0611' WHERE id = ?1", [file])
            .unwrap();
        let mut pb = Posts::load(&b).unwrap();
        pb.publish(&b, "2026-09-23").unwrap();
        pb.exchange_folder(&b, &folder).unwrap();
        let mut pa = Posts::load(&a).unwrap();
        pa.publish(&a, "2026-09-23").unwrap();
        pa.exchange_folder(&a, &folder).unwrap();
        let got = pa.absorb(&a, "2026-09-23").unwrap();
        assert!(got.received.written >= 1, "{got:?}");
        assert!(a.patients().unwrap().iter().any(|p| p.id == from_b));
        assert!(
            a.sync_posts().unwrap().iter().any(|p| p.post == 1),
            "B s'est présenté"
        );
        let mut pb = Posts::load(&b).unwrap();
        pb.exchange_folder(&b, &folder).unwrap();
        pb.absorb(&b, "2026-09-23").unwrap();
        let phone: String = b
            .conn_for_tests()
            .query_row("SELECT phone FROM patients WHERE id = ?1", [file], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(phone, "0611");
        for entry in std::fs::read_dir(&folder).unwrap().flatten() {
            let bytes = std::fs::read(entry.path()).unwrap();
            let text = String::from_utf8_lossy(&bytes);
            assert!(!text.contains("Dupont") && !text.contains("Grand") && !text.contains("0611"));
        }

        // A retired post is no longer ranged.
        a.retire_post(1, "2026-09-24").unwrap();
        b.add_patient("Après", "Retrait", "1970-01-01").unwrap();
        let mut pb = Posts::load(&b).unwrap();
        pb.publish(&b, "2026-09-24").unwrap();
        pb.exchange_folder(&b, &folder).unwrap();
        let mut pa = Posts::load(&a).unwrap();
        pa.exchange_folder(&a, &folder).unwrap();
        pa.absorb(&a, "2026-09-24").unwrap();
        assert!(!a.patients().unwrap().iter().any(|p| p.last_name == "Après"));
    }

    /// **La clé des postes n'ouvre pas le réseau d'officines, et
    /// inversement** : un patient scellé pour les postes ne se lit pas
    /// avec la clé du réseau.
    #[test]
    fn the_posts_key_and_the_network_key_open_nothing_of_each_other() {
        let (_dir, _s, a) = post("keys");
        a.add_patient("Secret", "Patient", "1950-01-01").unwrap();
        let mut pa = Posts::load(&a).unwrap();
        pa.found(&a, "", "2026-09-23").unwrap();
        crate::network::Net::create(&a).unwrap();
        let net_key = a.setting("net_trousseau").unwrap();
        let net = Trousseau::from_secret(unhex::<32>(&net_key).unwrap());
        for s in STREAMS {
            let r = pa.journal.read(&net, s);
            assert!(r.facts.is_empty(), "{s:?}");
        }
        let mine = pa.trousseau.clone().unwrap();
        assert!(!pa.journal.read(&mine, Stream::Dossiers).facts.is_empty());
        assert!(pa.journal.read(&mine, Stream::Reseau).facts.is_empty());
        assert!(!mine.same(&net));
    }

    #[test]
    fn a_beacon_names_a_device_and_a_port_and_nothing_else() {
        let d = hex(&[7u8; 32]);
        let ip: std::net::IpAddr = "192.168.1.14".parse().unwrap();
        let h = heard(&beacon(&d, 7743), ip).unwrap();
        assert_eq!(h.device, d);
        assert_eq!(h.address, "192.168.1.14:7743");
        assert!(heard("BPMPOSTE1 zz 7743", ip).is_none());
        assert!(heard(&format!("BPMPOSTE1 {d} 7743 Pharmacie"), ip).is_none());
        assert!(heard("autre chose", ip).is_none());
        assert_eq!(stream(Flux::Registre), Stream::Registre);
        assert!(!STREAMS.contains(&Stream::Reseau));
        assert_eq!(groups_of("zz"), "");
    }
}
