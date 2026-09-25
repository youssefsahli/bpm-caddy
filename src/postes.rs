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
    /// Les postes entendus sur le réseau local à cet instant — ceux du
    /// groupe et les autres.
    Peers(Vec<PeerSeen>),
    /// Les officines qui s'annoncent sur le réseau local à cet instant.
    Officines(Vec<crate::network::Nearby>),
    Done(String),
    Failed(String),
}

/// Un poste entendu sur le réseau local.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PeerSeen {
    pub device: String,
    pub address: String,
    /// Du groupe de ce poste.
    pub member: bool,
    /// La dernière conversation avec lui : réussie, échouée, ou aucune.
    pub talked: Option<bool>,
    /// D'aucun groupe : à relier.
    pub alone: bool,
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
    /// Avec le ticket du code d'invitation quand on l'a : rien à
    /// comparer alors.
    Join {
        address: String,
        name: String,
        ticket: Option<bpm_sync::Ticket>,
    },
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
            // Le fil automatique tenait ce port : arrêté par l'écran, il le
            // rend dans la demi-seconde. On le lui laisse, plutôt que de
            // dire « port pris » à qui vient de cliquer.
            let mut door = None;
            for _ in 0..15 {
                door = bpm_sync::link::Door::open(&format!("0.0.0.0:{port}")).ok();
                if door.is_some() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(200));
            }
            let door = door.ok_or_else(|| tr("posts_err_port").to_owned())?;
            // Un ticket par invitation, comme entre officines.
            let ticket = bpm_sync::Ticket::generate(&mut bpm_sync::OsEntropy);
            let _ = tx.send(Progress::Waiting(crate::network::invitation_code(
                &ticket,
                &format!("{}:{port}", local_address()),
            )));
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
            .and_then(|s| s.with_ticket(ticket))
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
        Job::Join {
            address,
            name,
            ticket,
        } => {
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
                .and_then(|s| match ticket {
                    Some(t) => s.with_ticket(*t),
                    None => Ok(s),
                })
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
    /// Une invitation d'officine est ouverte sur ce port (0 : plus
    /// aucune) — l'annonce de l'officine le dit, pour qu'une voisine la
    /// rejoigne d'un clic au lieu de recopier une adresse.
    Inviting(u16),
    Stop,
}

/// Ce que le fil automatique sait des postes qu'il a entendus.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Heard {
    pub device: String,
    pub address: String,
    /// Un poste d'aucun groupe : un poste à relier, peut-être de cette
    /// officine. Faux pour la première version de l'annonce, qui ne
    /// venait que d'un poste d'un groupe.
    pub alone: bool,
}

/// L'annonce qu'un poste envoie sur le réseau local : son identité et
/// son port. Rien d'autre — ni le nom de l'officine, ni une donnée.
pub fn beacon(device: &str, port: u16) -> String {
    format!("BPMPOSTE1 {device} {port}")
}

/// L'inverse, avec l'adresse d'où l'annonce est venue.
pub fn heard(text: &str, from: std::net::IpAddr) -> Option<Heard> {
    let parts: Vec<&str> = text.split_whitespace().collect();
    let (device, port, alone) = match parts.as_slice() {
        ["BPMPOSTE1", d, p] => (*d, *p, false),
        // La seconde version dit le groupe — son empreinte, que la
        // poignée de main montre de toute façon — ou « - » pour un poste
        // seul.
        ["BPMPOSTE2", d, p, "-"] => (*d, *p, true),
        ["BPMPOSTE2", d, p, g] => {
            unhex::<10>(g)?;
            (*d, *p, false)
        }
        _ => return None,
    };
    // Réécrite : une clé, une écriture — voir `network::heard_officine`.
    let device = hex(&unhex::<32>(device)?);
    let port: u16 = port.parse().ok()?;
    Some(Heard {
        device,
        address: std::net::SocketAddr::new(from, port).to_string(),
        alone,
    })
}

/// La seconde version de l'annonce : envoyée **aussi par un poste seul**,
/// pour qu'un poste de l'officine qu'on n'a pas encore relié se voie sur
/// la carte ; `group` est l'empreinte du groupe (hex), ou rien pour un
/// poste seul. Un poste d'un groupe envoie aussi la première version,
/// pour ceux qui n'ont pas encore la mise à jour.
pub fn beacon2(device: &str, port: u16, group: Option<&str>) -> String {
    format!("BPMPOSTE2 {device} {port} {}", group.unwrap_or("-"))
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

/// La synchronisation automatique, **dès le lancement et tant que
/// l'application est ouverte**. Sur un poste d'un groupe : une porte
/// tenue ouverte aux postes du groupe, une annonce toutes les quelques
/// secondes, et une conversation avec chaque poste entendu dès que ce
/// poste a écrit quelque chose — ou toutes les vingt secondes sinon ; le
/// dossier d'échange, s'il y en a un, chaque minute. Sur un poste seul :
/// **l'écoute seulement** — il sait quels postes s'annoncent, ce qui
/// donne l'adresse à composer pour rejoindre, et n'ouvre rien. Dans les
/// deux cas, la liste des postes entendus remonte à l'écran. S'arrête sur
/// [`Poke::Stop`] ou quand l'écran disparaît.
#[allow(clippy::too_many_arguments)]
pub fn spawn_auto(
    path: PathBuf,
    password: String,
    today: String,
    port: u16,
    folder: Option<PathBuf>,
    addresses: Vec<String>,
    officine: OfficineSide,
    pace: Pace,
) -> (
    std::sync::mpsc::Receiver<Progress>,
    std::sync::mpsc::Sender<Poke>,
) {
    let (tx, rx) = std::sync::mpsc::channel();
    let (poke_tx, poke_rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        if let Err(e) = auto(
            &path, &password, &today, port, folder, &addresses, &officine, &pace, &tx, &poke_rx,
        ) {
            let _ = tx.send(Progress::Failed(e));
        }
    });
    (rx, poke_tx)
}

/// **Ce que le fil automatique fait pour l'officine** — le réseau
/// d'officines, à côté des postes.
#[derive(Clone, Debug, Default)]
pub struct OfficineSide {
    /// S'annoncer sur le réseau local : le nom, la ville.
    pub announce: Option<(String, String)>,
    /// Répondre aux officines appairées sur ce port (0 : non).
    pub listen: u16,
    /// Le nom sous lequel l'officine signe ce qu'elle publie.
    pub name: String,
    /// La synchronisation des postes est coupée (`[postes] automatique`),
    /// mais le fil tourne pour l'officine : il n'ouvre pas la porte des
    /// postes et ne les compose pas.
    pub posts_paused: bool,
}

/// Combien de conversations d'officines à la fois, au plus : chacune a
/// son fil, et une officine lente — ou quelqu'un qui frappe sans rien
/// dire — n'en retient aucune autre, ni les postes.
const MOST_ANSWERS: usize = 2;

/// Le temps qu'une conversation d'officine peut durer en tout.
const ANSWER_BOUND: Duration = Duration::from_secs(60);

/// Une officine appairée entendue n'est composée de nouveau qu'après ce
/// délai — une annonce toutes les trois secondes n'est pas un appel toutes
/// les trois secondes, et une annonce contrefaite non plus.
const REDIAL: Duration = Duration::from_secs(600);

/// Les répondeurs de la porte des officines : [`MOST_ANSWERS`] fils, et le
/// canal où leur remettre une conversation — sans file : un lien qu'aucun
/// n'attend est lâché.
fn answerers(
    path: &Path,
    password: &str,
    tx: &std::sync::mpsc::Sender<Progress>,
) -> std::sync::mpsc::SyncSender<bpm_sync::link::TcpLink> {
    let (give, take) = std::sync::mpsc::sync_channel::<bpm_sync::link::TcpLink>(0);
    let take = std::sync::Arc::new(std::sync::Mutex::new(take));
    for _ in 0..MOST_ANSWERS {
        let take = std::sync::Arc::clone(&take);
        let (path, password, tx) = (path.to_path_buf(), password.to_owned(), tx.clone());
        std::thread::spawn(move || {
            let mut db: Option<Db> = None;
            loop {
                let next = match take.lock() {
                    Ok(t) => t.recv(),
                    Err(_) => return,
                };
                let Ok(mut link) = next else {
                    return;
                };
                if db.is_none() {
                    db = Db::open(&path, &password).ok();
                }
                let Some(db) = &db else {
                    continue;
                };
                if let Ok(device) = crate::network::answer(db, &mut link) {
                    let who = db
                        .net_peers()
                        .ok()
                        .and_then(|p| p.into_iter().find(|p| p.device == device))
                        .map(|p| {
                            [p.name, p.seen_as]
                                .into_iter()
                                .find(|n| !n.trim().is_empty())
                                .unwrap_or_default()
                        })
                        .filter(|n| !n.is_empty())
                        .unwrap_or_else(|| crate::network::peer_groups(&device));
                    let _ = tx.send(Progress::Status(crate::strings::trf("net_answered", who)));
                }
            }
        });
    }
    give
}

/// Ce que le fil sait d'un poste entendu.
struct Seen {
    heard: Heard,
    at: Instant,
    talked: Option<bool>,
}

/// Combien de temps un poste reste « en ligne » sans s'annoncer.
const SEEN_FOR: Duration = Duration::from_secs(15);

/// Combien d'officines voisines on retient au plus : les annonces
/// viennent de n'importe qui sur le réseau local, et n'importe qui ne
/// choisit pas la mémoire de ce poste.
const MOST_NEARBY: usize = 32;

#[allow(clippy::too_many_arguments)]
fn auto(
    path: &Path,
    password: &str,
    today: &str,
    port: u16,
    folder: Option<PathBuf>,
    addresses: &[String],
    side: &OfficineSide,
    pace: &Pace,
    tx: &std::sync::mpsc::Sender<Progress>,
    pokes: &std::sync::mpsc::Receiver<Poke>,
) -> Result<(), String> {
    use crate::strings::tr;
    let db = Db::open(path, password)?;
    // **L'officine s'annonce aussi**, quand elle le veut : son identité
    // de réseau, son nom et sa ville, pour que les officines voisines la
    // voient sur leur carte. Tirée de la base, comme l'identité elle-même.
    let my_officine = crate::network::device_hex(&db).ok();
    let officine = side.announce.as_ref().and_then(|(name, place)| {
        my_officine
            .clone()
            .map(|d| (d, name.clone(), place.clone()))
    });
    // **La porte des officines appairées**, ouverte dès que l'officine est
    // d'un réseau — regardé chaque minute, pour un réseau rejoint en
    // cours de journée.
    let mut net_door: Option<bpm_sync::link::Door> = None;
    let mut net_door_looked: Option<Instant> = None;
    let mut net_door_said = false;
    // **Deux répondeurs, ouverts une fois** : chacun a sa connexion à la
    // base, ouverte à sa première conversation et gardée — frapper à la
    // porte ne coûte plus une dérivation de clé à chaque fois. Une porte
    // où les deux sont pris lâche la conversation aussitôt.
    let answers = (side.listen != 0).then(|| answerers(path, password, tx));
    // Les officines appairées, relues de temps en temps : ce qu'une
    // annonce entendue fait apprendre, et qui on a déjà composé.
    let mut paired: Vec<String> = Vec::new();
    let mut paired_read: Option<Instant> = None;
    // Par officine **et par adresse** : une annonce contrefaite entendue
    // la première ne retarde pas l'appel à la vraie adresse.
    let mut dialed: std::collections::HashMap<(String, String), Instant> =
        std::collections::HashMap::new();
    let dialing = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut inviting: u16 = 0;
    // Chaque voisine, l'adresse d'où elle s'est annoncée, et quand.
    let mut near: Vec<(crate::network::Nearby, std::net::IpAddr, Instant)> = Vec::new();
    let mut told_near: Vec<crate::network::Nearby> = Vec::new();
    let mut posts = Posts::load(&db)?;
    // Le groupe dont ce poste est, pour l'annonce ; celui qu'on synchronise,
    // seulement si la synchronisation des postes est voulue.
    let group_trousseau = posts.trousseau.clone();
    let trousseau = if side.posts_paused {
        None
    } else {
        group_trousseau.clone()
    };
    // A post on its own listens and opens nothing.
    let door = trousseau
        .as_ref()
        .and_then(|_| bpm_sync::link::Door::open(&format!("0.0.0.0:{port}")).ok());
    if trousseau.is_some() && door.is_none() {
        let _ = tx.send(Progress::Status(tr("posts_auto_no_door").to_owned()));
    }
    let udp = std::net::UdpSocket::bind(("0.0.0.0", port)).ok();
    if let Some(u) = &udp {
        let _ = u.set_broadcast(true);
        let _ = u.set_nonblocking(true);
    }
    let me = posts.device_hex();
    let mut seen: Vec<Seen> = Vec::new();
    let mut told: Vec<PeerSeen> = Vec::new();
    let long_ago = Instant::now()
        .checked_sub(Duration::from_secs(3600))
        .unwrap_or_else(Instant::now);
    let (mut last_beacon, mut last_talk, mut last_folder) = (long_ago, long_ago, long_ago);
    loop {
        let mut now_asked = false;
        match pokes.try_recv() {
            Ok(Poke::Stop) | Err(std::sync::mpsc::TryRecvError::Disconnected) => break,
            Ok(Poke::Now) => now_asked = true,
            Ok(Poke::Inviting(port)) => {
                inviting = port;
                // Dit tout de suite, pas au prochain tour d'annonce.
                last_beacon = long_ago;
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
        }
        let mut talked = false;
        // A post knocking at our door.
        match (&door, &trousseau) {
            (Some(door), Some(t)) => {
                if let Ok(mut link) = door.accept(Duration::from_millis(400)) {
                    if posts.talk(&db, t, &mut link, false).is_ok() {
                        talked = true;
                    }
                }
            }
            _ => std::thread::sleep(Duration::from_millis(400)),
        }
        // **Une officine appairée qui frappe** : sa conversation sur son fil.
        if side.listen != 0
            && net_door.is_none()
            && net_door_looked.is_none_or(|t| t.elapsed() >= Duration::from_secs(60))
        {
            net_door_looked = Some(Instant::now());
            if crate::network::Net::load_all(&db).is_ok_and(|n| !n.is_empty()) {
                net_door = bpm_sync::link::Door::open(&format!("0.0.0.0:{}", side.listen)).ok();
                if net_door.is_none() && !net_door_said {
                    net_door_said = true;
                    let _ = tx.send(Progress::Status(crate::strings::trf(
                        "net_listen_no_door",
                        side.listen,
                    )));
                }
            }
        }
        if let (Some(d), Some(give)) = (&net_door, &answers) {
            if let Ok(link) = d.accept_with(Duration::from_millis(100), TALK_PATIENCE) {
                // Toute la conversation en une minute au plus : qui écrit
                // un octet de temps en temps ne tient pas la porte. Remise
                // à un répondeur libre, ou lâchée sur-le-champ.
                let _ = give.try_send(link.with_deadline(ANSWER_BOUND));
            }
        }
        if paired_read.is_none_or(|t| t.elapsed() >= Duration::from_secs(30)) {
            paired_read = Some(Instant::now());
            paired = db
                .net_peers()
                .map(|p| p.into_iter().map(|p| p.device).collect())
                .unwrap_or_default();
        }
        // Who announced themselves.
        if let Some(u) = &udp {
            let mut buf = [0u8; 1024];
            while let Ok((n, from)) = u.recv_from(&mut buf) {
                let text = std::str::from_utf8(&buf[..n]).ok();
                if let Some(o) = text.and_then(|t| crate::network::heard_officine(t, from.ip())) {
                    let mine = my_officine.as_deref() == Some(o.device.as_str());
                    // **La première adresse tient** jusqu'à ce qu'elle se
                    // taise : une annonce venue d'ailleurs sous la même
                    // identité ne détourne pas l'invitation d'une voisine.
                    let held = near.iter().find(|(x, _, _)| x.device == o.device);
                    let elsewhere = held.is_some_and(|(_, ip, _)| *ip != from.ip());
                    if mine || elsewhere {
                        continue;
                    }
                    // **Une officine appairée qui s'annonce** : on la compose
                    // à l'adresse annoncée — c'est la connexion au
                    // lancement, sans rien à saisir. Une fois toutes les
                    // dix minutes au plus, et l'adresse n'est gardée qu'une
                    // fois qu'elle y a répondu (`dial_heard`).
                    if paired.contains(&o.device) {
                        use std::sync::atomic::Ordering;
                        let key = (o.device.clone(), o.listen.clone().unwrap_or_default());
                        let due = dialed
                            .get(&key)
                            .is_none_or(|t: &Instant| t.elapsed() >= REDIAL);
                        if let (Some(at), true) = (&o.listen, due) {
                            if dialing.load(Ordering::SeqCst) < MOST_ANSWERS {
                                dialed.insert(key, Instant::now());
                                dialing.fetch_add(1, Ordering::SeqCst);
                                let (busy, tx) = (std::sync::Arc::clone(&dialing), tx.clone());
                                let (path, password) = (path.to_path_buf(), password.to_owned());
                                let (device, name, at) =
                                    (o.device.clone(), side.name.clone(), at.clone());
                                std::thread::spawn(move || {
                                    if let Ok(db) = Db::open(&path, &password) {
                                        if let Ok(said) =
                                            crate::network::dial_heard(&db, &device, &at, &name)
                                        {
                                            let _ = tx.send(Progress::Status(said));
                                        }
                                    }
                                    busy.fetch_sub(1, Ordering::SeqCst);
                                });
                            }
                        }
                    }
                    if held.is_some() || near.len() < MOST_NEARBY {
                        near.retain(|(x, _, _)| x.device != o.device);
                        near.push((o, from.ip(), Instant::now()));
                    }
                    continue;
                }
                if let Some(h) = text.and_then(|t| heard(t, from.ip())) {
                    if h.device != me {
                        let talked = seen
                            .iter()
                            .find(|x| x.heard.device == h.device)
                            .and_then(|x| x.talked);
                        seen.retain(|x| x.heard.device != h.device);
                        seen.push(Seen {
                            heard: h,
                            at: Instant::now(),
                            talked,
                        });
                    }
                }
            }
            if last_beacon.elapsed() >= pace.beacon {
                let group = group_trousseau.as_ref().map(|t| hex(&t.name().bytes()));
                if trousseau.is_some() {
                    let _ = u.send_to(beacon(&me, port).as_bytes(), ("255.255.255.255", port));
                }
                let _ = u.send_to(
                    beacon2(&me, port, group.as_deref()).as_bytes(),
                    ("255.255.255.255", port),
                );
                if let Some((device, name, place)) = &officine {
                    let listening = if net_door.is_some() { side.listen } else { 0 };
                    let b =
                        crate::network::officine_beacon(device, name, place, listening, inviting);
                    let _ = u.send_to(b.as_bytes(), ("255.255.255.255", port));
                }
                last_beacon = Instant::now();
            }
        }
        seen.retain(|x| x.at.elapsed() < SEEN_FOR);
        near.retain(|(_, _, at)| at.elapsed() < SEEN_FOR);
        let near_now: Vec<crate::network::Nearby> =
            near.iter().map(|(n, _, _)| n.clone()).collect();
        if near_now != told_near {
            let _ = tx.send(Progress::Officines(near_now.clone()));
            told_near = near_now;
        }
        let known: Vec<String> = posts.known(&db).iter().map(|d| hex(&d.0)).collect();
        if trousseau.is_some() {
            let pending = db
                .pending_ops()
                .map(|(o, _)| !o.is_empty())
                .unwrap_or(false);
            if pending || now_asked || talked || last_talk.elapsed() >= pace.talk {
                let sent = posts.publish(&db, today)?;
                if sent > 0 || now_asked || last_talk.elapsed() >= pace.talk {
                    for x in seen.iter_mut().filter(|x| known.contains(&x.heard.device)) {
                        let ok = posts
                            .sync_with(&db, &x.heard.address, TALK_PATIENCE)
                            .is_ok();
                        x.talked = Some(ok);
                        talked |= ok;
                    }
                    for a in addresses.iter().filter(|a| !a.trim().is_empty()) {
                        if !seen.iter().any(|x| x.heard.address == *a)
                            && posts.sync_with(&db, a.trim(), TALK_PATIENCE).is_ok()
                        {
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
        let now: Vec<PeerSeen> = seen
            .iter()
            .map(|x| PeerSeen {
                device: x.heard.device.clone(),
                address: x.heard.address.clone(),
                member: known.contains(&x.heard.device),
                talked: x.talked,
                alone: x.heard.alone,
            })
            .collect();
        if now != told {
            let _ = tx.send(Progress::Peers(now.clone()));
            told = now;
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

    /// **Deux postes, une porte, le code d'invitation** : B rejoint le
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
        // Le code d'invitation, collé tel qu'affiché — avec l'adresse de
        // la boucle locale à la place de celle du réseau.
        let ticket = match inviting.recv().unwrap() {
            Progress::Waiting(code) => crate::network::read_join(&code).and_then(|(_, t)| t),
            other => panic!("{other:?}"),
        };
        assert!(ticket.is_some(), "une invitation porte son code");
        let joining = spawn(
            Job::Join {
                address: format!("127.0.0.1:{port}"),
                name: "Comptoir 2".to_owned(),
                ticket,
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
        assert!(codes.is_empty(), "rien à comparer avec le code : {codes:?}");

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

    /// **Un poste seul écoute** : lancé sur une base qui n'est d'aucun
    /// groupe, le fil n'ouvre pas de porte mais entend les postes qui
    /// s'annoncent, et le dit à l'écran — c'est l'adresse à composer.
    #[test]
    fn a_post_on_its_own_listens_and_reports_who_announces() {
        let (dir, _s, db) = post("listen");
        drop(db);
        let port = std::net::UdpSocket::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let (rx, poke) = spawn_auto(
            dir.join("poste.db"),
            "secret".to_owned(),
            "2026-09-23".to_owned(),
            port,
            None,
            Vec::new(),
            OfficineSide::default(),
            Pace::default(),
        );
        let other = hex(&[9u8; 32]);
        let sender = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
        let mut heard_it = None;
        for _ in 0..40 {
            let _ = sender.send_to(beacon(&other, 7743).as_bytes(), ("127.0.0.1", port));
            if let Ok(Progress::Peers(p)) = rx.recv_timeout(Duration::from_millis(250)) {
                heard_it = p.into_iter().find(|x| x.device == other);
                if heard_it.is_some() {
                    break;
                }
            }
        }
        poke.send(Poke::Stop).unwrap();
        let h = heard_it.expect("entendu");
        assert!(!h.member, "pas du groupe");
        assert_eq!(h.address, "127.0.0.1:7743");
        assert_eq!(h.talked, None);
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

    /// **Les officines voisines** : le fil entend une officine qui
    /// s'annonce, avec son invitation ouverte ; il ne se compte pas
    /// lui-même — sa propre annonce lui revient par la diffusion.
    #[test]
    fn the_auto_thread_reports_the_officines_that_announce() {
        let (dir, _s, db) = post("near");
        let me = crate::network::device_hex(&db).unwrap();
        drop(db);
        let port = std::net::UdpSocket::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let (rx, poke) = spawn_auto(
            dir.join("poste.db"),
            "secret".to_owned(),
            "2026-09-24".to_owned(),
            port,
            None,
            Vec::new(),
            OfficineSide {
                announce: Some(("Pharmacie du Centre".to_owned(), "Épinal".to_owned())),
                listen: 0,
                name: "Pharmacie du Centre".to_owned(),
                posts_paused: false,
            },
            Pace::default(),
        );
        let other = hex(&[9u8; 32]);
        let sender = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
        let mut found = None;
        for _ in 0..40 {
            for b in [
                crate::network::officine_beacon(&other, "Pharmacie du Port", "Sète", 0, 7742),
                crate::network::officine_beacon(&me, "Moi-même", "", 0, 0),
            ] {
                let _ = sender.send_to(b.as_bytes(), ("127.0.0.1", port));
            }
            if let Ok(Progress::Officines(list)) = rx.recv_timeout(Duration::from_millis(250)) {
                assert!(list.iter().all(|n| n.device != me), "pas soi-même");
                found = list.into_iter().find(|n| n.device == other);
                if found.is_some() {
                    break;
                }
            }
        }
        poke.send(Poke::Stop).unwrap();
        let n = found.expect("entendue");
        assert_eq!(n.name, "Pharmacie du Port");
        assert_eq!(n.invite.as_deref(), Some("127.0.0.1:7742"));
    }

    /// **La seconde annonce dit si le poste est seul** : un poste d'aucun
    /// groupe s'annonce aussi, avec « - », et la première version s'entend
    /// toujours.
    #[test]
    fn a_post_on_its_own_announces_itself_as_such() {
        let d = hex(&[7u8; 32]);
        let ip: std::net::IpAddr = "192.168.1.14".parse().unwrap();
        let alone = heard(&beacon2(&d, 7743, None), ip).unwrap();
        assert!(alone.alone);
        assert_eq!(alone.address, "192.168.1.14:7743");
        let g = hex(&[3u8; 10]);
        let grouped = heard(&beacon2(&d, 7743, Some(&g)), ip).unwrap();
        assert!(!grouped.alone);
        assert!(!heard(&beacon(&d, 7743), ip).unwrap().alone);
        for bad in [
            format!("BPMPOSTE2 {d} 7743"),
            format!("BPMPOSTE2 {d} 7743 zz"),
            format!("BPMPOSTE2 {d} 7743 - de-trop"),
            format!("BPMPOSTE2 {d} x -"),
        ] {
            assert!(heard(&bad, ip).is_none(), "{bad}");
        }
    }

    /// **Une invitation attend que le port se libère** : le fil automatique,
    /// arrêté par l'écran, rend son port dans la demi-seconde ; l'invitation
    /// ne dit pas « port pris » pour autant.
    #[test]
    fn an_invitation_waits_for_the_port_the_auto_thread_is_releasing() {
        let (dir, _s, db) = post("port-release");
        Posts::load(&db)
            .unwrap()
            .found(&db, "Comptoir 1", "2026-09-25")
            .unwrap();
        drop(db);
        let held = bpm_sync::link::Door::open("0.0.0.0:0").unwrap();
        let port = held.address().unwrap().port();
        let release = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(500));
            drop(held);
        });
        let (_yes, answers) = std::sync::mpsc::channel();
        let rx = spawn(
            Job::Invite { port },
            dir.join("poste.db"),
            "secret".to_owned(),
            "2026-09-25".to_owned(),
            answers,
        );
        let first = rx
            .recv_timeout(Duration::from_secs(10))
            .expect("une réponse");
        release.join().unwrap();
        assert!(matches!(first, Progress::Waiting(_)), "{first:?}");
    }
}
