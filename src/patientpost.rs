//! **Le poste qui attend les patients** : un poste de bureau de
//! l'officine répond aux pages des patients avec la base
//! (`patientchat::answer`), par deux chemins.
//!
//! * **La voie directe** (`direct.rs`) : le poste écoute à ses adresses
//!   publiques — IPv6, et l'IPv4 de la box qu'il fait ouvrir (UPnP) — et
//!   la page le joint en WebRTC, sans rien entre les deux. Les adresses
//!   trouvées sont rangées dans les réglages de l'officine, un réglage par
//!   poste ([`ADDRESSES`]) : un code donné sur un poste mène à tous.
//! * **Une passerelle** (`passerelle.rs`), si l'officine en a réglé une :
//!   deux attentes à la fois, chacune sur son fil, pour que pendant une
//!   conversation la suivante trouve encore le poste.
//!
//! Les conversations passent une à une au fil qui tient la base. Chaque
//! poste ouvert attend : un patient trouve l'officine tant qu'un poste est
//! allumé. Le téléphone de l'équipe n'attend jamais — il ne tient pas les
//! dossiers.

use std::net::IpAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use crate::db::Db;
use crate::direct;
use crate::patientchat::{self, FromPatient, KeyRow, Officine, Store, ToPatient};

/// Le début des réglages où chaque poste range ses adresses sur la voie
/// directe : `pchat_adr.<numéro du poste>`, valeur `AAAA-MM-JJ adresse,…`.
pub const ADDRESSES: &str = "pchat_adr.";
/// Une adresse qu'aucun poste n'a redite depuis ce nombre de jours n'est
/// plus donnée : le poste qui la tenait a quitté l'officine, ou elle a
/// changé.
const ADDRESSES_KEPT_DAYS: i64 = 30;
/// Tous les combien un poste relit ses adresses.
const ADDRESSES_EVERY: Duration = Duration::from_secs(10 * 60);

/// Combien d'attentes à la fois, par poste.
const WAITERS: usize = 2;
/// Le temps de joindre la passerelle.
const PATIENCE: Duration = Duration::from_secs(10);
/// Le temps d'une trame de la page, une fois la conversation commencée.
const WORD_WAIT: Duration = Duration::from_secs(10);
/// Après un échec (passerelle éteinte, nom inconnu), avant de rappeler.
const BACKOFF: Duration = Duration::from_secs(30);
/// Tous les combien la page publiée est comparée à celle qu'on livre.
const PAGE_EVERY: Duration = Duration::from_secs(6 * 3600);

/// Ce que le fil dit à l'écran.
#[derive(Debug, PartialEq, Eq)]
pub enum Event {
    /// Des messages de patients sont arrivés.
    Received(usize),
    /// La passerelle ne répond pas (le texte de l'erreur) ; rien n'est
    /// perdu, la page du patient garde ses messages.
    Unreachable(String),
    /// Elle répond de nouveau.
    Reached,
    /// Ce que la page publiée est, comparée à la nôtre.
    Page(PageCheck),
    /// La voie directe de ce poste : ses adresses telles qu'une page les
    /// compose depuis Internet (vides : ni IPv6 publique, ni box qui ouvre
    /// le port), et celles où il écoute — dont son adresse sur le réseau
    /// de l'officine, pour un écran qui s'y trouve.
    Direct {
        public: Vec<String>,
        bound: Vec<String>,
    },
}

/// **La voie directe d'un poste** : l'identité de l'officine, le port, et
/// où écouter — `None` : les adresses publiques de la machine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Direct {
    pub identity: direct::Identity,
    pub port: u16,
    pub at: Option<Vec<IpAddr>>,
    /// **Attendre aussi aux relais** que les officines du réseau offrent
    /// (`[postes] relais`, `relay.rs`) : une page y joint ce poste quand
    /// aucune de ses adresses ne se compose.
    pub relays: bool,
}

/// Les relais où attendre, et le groupe de postes sous lequel : relus de
/// la base par son fil, toutes les minutes.
type RelaysWanted = Arc<Mutex<Option<(String, Vec<std::net::SocketAddr>)>>>;

/// Combien de relais une voie directe tient au plus.
const MOST_RELAYS: usize = 4;

/// **Les adresses de l'officine sur la voie directe**, de tous ses postes,
/// lues des réglages ([`ADDRESSES`]) : celles qu'un poste a redites dans
/// les trente derniers jours, la plus récente d'abord, sans doublon.
pub fn officine_addresses(rows: &[(String, String)], today: &str) -> Vec<String> {
    let mut dated: Vec<(&str, &str)> = rows
        .iter()
        .filter_map(|(_, v)| v.split_once(' '))
        .filter(|(day, _)| {
            crate::date::days_between(day, today).is_some_and(|d| d <= ADDRESSES_KEPT_DAYS)
        })
        .collect();
    dated.sort_by(|a, b| b.0.cmp(a.0));
    let mut out: Vec<String> = Vec::new();
    for (_, list) in dated {
        for a in list.split(',').map(str::trim).filter(|a| !a.is_empty()) {
            if !out.iter().any(|o| o == a) {
                out.push(a.to_owned());
            }
        }
    }
    out
}

/// Une conversation, d'où qu'elle vienne.
trait Talk: std::io::Read + std::io::Write + Send {}
impl<T: std::io::Read + std::io::Write + Send> Talk for T {}

/// Ce que le fil de la base reçoit.
enum Job {
    /// Une conversation, et son bonjour quand il est déjà lu.
    Talk(Box<dyn Talk>, Option<Vec<u8>>),
    /// Les adresses de ce poste, à ranger.
    Addresses(Vec<String>),
}

/// **Le tableau tel que le fil de la base l'a lu en dernier** : une page
/// qui ne demande que lui reçoit celui-ci, sans attendre derrière les
/// patients — les écrans le demandent chaque minute.
type BoardCache = Arc<Mutex<Option<Vec<u8>>>>;

/// Combien de temps le tableau gardé vaut avant d'être relu.
const BOARD_EVERY: Duration = Duration::from_secs(60);

/// La génération des voies directes de ce processus : le port de la box
/// ne se referme qu'au départ de la plus récente, jamais derrière elle.
static DIRECT_GENERATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// **La page publiée, vue d'un poste** (`check_page`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PageCheck {
    /// Octet pour octet celle que ce logiciel livre.
    Same,
    /// Autre chose : qui l'ouvre ne chiffre peut-être plus comme le
    /// poste. L'officine est prévenue ; elle ne cesse pas d'attendre —
    /// cesser ne protégerait pas un patient qui écrit déjà sur la page.
    Different,
    /// Pas lue (pas encore publiée, ou hors ligne) : rien n'est dit de
    /// plus qu'un avertissement discret.
    Unread(String),
}

/// **Comparer la page publiée à la nôtre** : chacun des trois fichiers de
/// `patientchat::PAGE_FILES`, lu sous `patientchat::page_url()` par
/// `fetch`. L'empreinte est dans l'adresse ; ce sont les octets servis
/// qu'on compare — un dossier réécrit sous le même nom se voit.
pub fn check_page(fetch: impl Fn(&str) -> Result<Vec<u8>, String>) -> PageCheck {
    let base = patientchat::page_url();
    for (name, ours) in patientchat::PAGE_FILES {
        match fetch(&format!("{base}{name}")) {
            Ok(theirs) if theirs == ours.as_bytes() => {}
            Ok(_) => return PageCheck::Different,
            Err(e) => return PageCheck::Unread(e),
        }
    }
    PageCheck::Same
}

/// Lire un fichier de la page publiée, en HTTPS seulement.
fn fetch_https(url: &str) -> Result<Vec<u8>, String> {
    let agent = ureq::AgentBuilder::new()
        .https_only(true)
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(15))
        .user_agent("bpm-caddy")
        .build();
    let mut body = Vec::new();
    std::io::Read::read_to_end(
        &mut std::io::Read::take(
            agent
                .get(url)
                .call()
                .map_err(|e| e.to_string())?
                .into_reader(),
            1 << 20,
        ),
        &mut body,
    )
    .map_err(|e| e.to_string())?;
    Ok(body)
}

/// **Les heures d'ouverture, en clair**, pour la page : les plages de
/// chaque jour à la suite, dans l'ordre où l'officine les a écrites —
/// `(jour, de, à)`.
pub fn opening_hours(rows: &[(String, String, String)]) -> String {
    let mut days: Vec<(String, Vec<String>)> = Vec::new();
    for (day, from, to) in rows {
        let (day, from, to) = (day.trim().to_lowercase(), from.trim(), to.trim());
        if day.is_empty() || from.is_empty() || to.is_empty() {
            continue;
        }
        let span = format!("{from}-{to}");
        match days.iter_mut().find(|(d, _)| *d == day) {
            Some((_, spans)) => spans.push(span),
            None => days.push((day, vec![span])),
        }
    }
    days.into_iter()
        .map(|(d, spans)| format!("{d} {}", spans.join(" et ")))
        .collect::<Vec<_>>()
        .join(" ; ")
}

/// La base, vue par une conversation.
pub struct DbStore<'a> {
    pub db: &'a Db,
    pub officine: Officine,
}

/// L'heure d'un message, pour la page : `JJ/MM/AAAA HH:MM`.
pub fn shown_at(at: &str) -> String {
    match at.split_once(' ') {
        Some((day, time)) => format!("{} {time}", crate::db::format_french_date(day)),
        None => crate::db::format_french_date(at),
    }
}

impl DbStore<'_> {
    fn today(&self) -> String {
        self.db.today_iso().unwrap_or_default()
    }
}

impl Store for DbStore<'_> {
    fn keys(&mut self) -> Result<Vec<KeyRow>, String> {
        Ok(self
            .db
            .patient_chat_keys()?
            .into_iter()
            .filter_map(|k| {
                Some(KeyRow {
                    id: k.id,
                    key: patientchat::key_from_text(&k.key)?,
                    open: k.closed_on.is_empty(),
                })
            })
            .collect())
    }

    fn receive(&mut self, key: i64, messages: &[FromPatient]) -> Result<Vec<String>, String> {
        let rows: Vec<(String, String, String)> = messages
            .iter()
            .map(|m| (m.id.clone(), m.at.clone(), m.text.trim().to_owned()))
            .collect();
        self.db.receive_patient_messages(key, &rows)
    }

    fn replies(&mut self, key: i64, seen: &[i64]) -> Result<Vec<ToPatient>, String> {
        Ok(self
            .db
            .patient_replies(key, seen)?
            .into_iter()
            .map(|m| ToPatient {
                id: m.id,
                at: shown_at(&m.at),
                text: m.body,
            })
            .collect())
    }

    fn officine(&mut self) -> Officine {
        let rows = self.db.settings_with_prefix(ADDRESSES).unwrap_or_default();
        Officine {
            adresses: officine_addresses(&rows, &self.today()),
            ..self.officine.clone()
        }
    }

    fn board(&mut self) -> crate::tableau::Public {
        let t = crate::tableau::Tableau::read(self.db.setting(crate::tableau::SETTING).as_deref());
        let today = self.today();
        let rows = self.db.settings_with_prefix(ADDRESSES).unwrap_or_default();
        let o = &self.officine;
        let mut p =
            crate::tableau::Public::of(&t, &today, &o.nom, &o.adresse, &o.telephone, &o.horaires);
        p.adresses = officine_addresses(&rows, &today);
        p
    }
}

/// Ranger les adresses de ce poste, si elles ont changé ou datent d'un
/// autre jour — contre ce qu'on a lu : un autre poste n'écrit pas sous
/// ce nom.
fn note_addresses(db: &Db, addresses: &[String]) {
    let key = format!("{ADDRESSES}{}", db.sync_post().unwrap_or(0));
    let today = db.today_iso().unwrap_or_default();
    let value = format!("{today} {}", addresses.join(","));
    let was = db.setting(&key);
    if addresses.is_empty() && was.is_none() {
        return;
    }
    if was.as_deref() != Some(value.as_str()) {
        let _ = db.set_setting(&key, &value, was.as_deref(), &today, "");
    }
}

fn fill(buf: &mut [u8]) {
    bpm_sync::Entropy::fill(&mut bpm_sync::OsEntropy, buf);
}

/// Le fil qui attend : l'arrêter, c'est aussi lâcher les attentes en
/// cours à la passerelle — fermées, elles y sont vues raccrochées
/// (`passerelle::alive`) et rendent leurs places à la boîte.
pub struct Waiting {
    stop: Arc<AtomicBool>,
    held: Arc<Mutex<Vec<(u64, std::net::TcpStream)>>>,
    officine: Arc<Mutex<Officine>>,
    relays: Arc<AtomicBool>,
    pub events: mpsc::Receiver<Event>,
}

impl Waiting {
    /// L'officine a changé (son nom, ses heures) : dite aux pages dès la
    /// conversation suivante, sans relancer l'écoute — relancée, la voie
    /// directe perdait son port le temps que l'ancienne le rende.
    pub fn set_officine(&self, officine: Officine) {
        if let Ok(mut o) = self.officine.lock() {
            *o = officine;
        }
    }

    /// Attendre aux relais, ou plus (`[postes] relais`) : relu à la
    /// minute suivante, sans relancer l'écoute.
    pub fn set_relays(&self, on: bool) {
        self.relays.store(on, Ordering::SeqCst);
    }

    pub fn stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Ok(mut h) = self.held.lock() {
            for (_, s) in h.drain(..) {
                let _ = s.shutdown(std::net::Shutdown::Both);
            }
        }
    }
}

impl Drop for Waiting {
    fn drop(&mut self) {
        self.stop();
    }
}

/// **Attendre les patients**, pour la boîte de `secret`, avec la base de
/// `path` : à la passerelle `gateway` (un nom d'hôte ; vide : aucune) et
/// sur la voie directe (`direct`).
pub fn spawn(
    path: std::path::PathBuf,
    password: String,
    gateway: &str,
    secret: String,
    officine: Officine,
    direct: Option<Direct>,
) -> Waiting {
    let stop = Arc::new(AtomicBool::new(false));
    let held: Arc<Mutex<Vec<(u64, std::net::TcpStream)>>> = Arc::default();
    let shared = Arc::new(Mutex::new(officine));
    let board: BoardCache = Arc::default();
    let relays: RelaysWanted = Arc::default();
    let relays_on = Arc::new(AtomicBool::new(
        direct.as_ref().is_some_and(|d| d.relays && d.at.is_none()),
    ));
    let read_relays = Arc::clone(&relays_on);
    let (events_tx, events) = mpsc::channel();
    let (talk_tx, talk_rx) = mpsc::sync_channel::<Job>(0);
    let gateway = patientchat::gateway_host(gateway);
    let address = crate::passerelle::posts_address(&gateway);
    for _ in 0..if gateway.is_empty() { 0 } else { WAITERS } {
        let (stop, held, talk_tx, events_tx, address, secret) = (
            Arc::clone(&stop),
            Arc::clone(&held),
            talk_tx.clone(),
            events_tx.clone(),
            address.clone(),
            secret.clone(),
        );
        std::thread::spawn(move || {
            let mut failing = false;
            while !stop.load(Ordering::SeqCst) {
                match wait_once(&address, &secret, &held) {
                    Ok(stream) => {
                        if failing {
                            failing = false;
                            let _ = events_tx.send(Event::Reached);
                        }
                        // Each word of the page within a few seconds: a
                        // page that has the code and goes quiet does not
                        // hold this post a minute.
                        let _ = stream.set_read_timeout(Some(WORD_WAIT));
                        let _ = stream.set_write_timeout(Some(WORD_WAIT));
                        if talk_tx.send(Job::Talk(Box::new(stream), None)).is_err() {
                            return;
                        }
                    }
                    // The wait ran out with nobody: wait again at once.
                    Err(e)
                        if matches!(
                            e.kind(),
                            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                        ) => {}
                    Err(e) => {
                        if !failing && !stop.load(Ordering::SeqCst) {
                            failing = true;
                            let _ = events_tx.send(Event::Unreachable(e.to_string()));
                        }
                        let until = std::time::Instant::now() + BACKOFF;
                        while std::time::Instant::now() < until && !stop.load(Ordering::SeqCst) {
                            std::thread::sleep(Duration::from_millis(250));
                        }
                    }
                }
            }
        });
    }
    if let (Some(d), Some(box_id)) = (direct, patientchat::box_of_secret(&secret)) {
        let (stop, talk_tx, events_tx, board, relays) = (
            Arc::clone(&stop),
            talk_tx.clone(),
            events_tx.clone(),
            Arc::clone(&board),
            Arc::clone(&relays),
        );
        std::thread::spawn(move || {
            listen_direct(d, box_id, talk_tx, events_tx, board, relays, stop)
        });
    }
    drop(talk_tx);
    // The page the patients open, compared now and then to ours.
    {
        let (stop, events_tx) = (Arc::clone(&stop), events_tx.clone());
        std::thread::spawn(move || {
            while !stop.load(Ordering::SeqCst) {
                let _ = events_tx.send(Event::Page(check_page(fetch_https)));
                let until = std::time::Instant::now() + PAGE_EVERY;
                while std::time::Instant::now() < until && !stop.load(Ordering::SeqCst) {
                    std::thread::sleep(Duration::from_millis(500));
                }
            }
        });
    }
    let worker_stop = Arc::clone(&stop);
    let worker_officine = Arc::clone(&shared);
    std::thread::spawn(move || {
        let Ok(db) = Db::open(&path, &password) else {
            let _ = events_tx.send(Event::Unreachable(String::new()));
            return;
        };
        let shared = worker_officine;
        let officine = shared.lock().map(|o| o.clone()).unwrap_or_default();
        let mut store = DbStore { db: &db, officine };
        let mut board_read: Option<std::time::Instant> = None;
        while !worker_stop.load(Ordering::SeqCst) {
            if let Ok(o) = shared.lock() {
                if store.officine != *o {
                    store.officine = o.clone();
                    board_read = None;
                }
            }
            if board_read.is_none_or(|t| t.elapsed() > BOARD_EVERY) {
                if let Ok(bytes) = serde_json::to_vec(&store.board()) {
                    if let Ok(mut b) = board.lock() {
                        *b = Some(bytes);
                    }
                }
                if read_relays.load(Ordering::SeqCst) || relays.lock().is_ok_and(|r| r.is_some()) {
                    let wanted = if read_relays.load(Ordering::SeqCst) {
                        relays_wanted(&db)
                    } else {
                        Ok(None)
                    };
                    // A base that could not be read keeps the relays as
                    // they were: changed, the listening starts again and
                    // every page talking to this post is cut.
                    if let (Ok(wanted), Ok(mut r)) = (wanted, relays.lock()) {
                        *r = wanted;
                    }
                }
                board_read = Some(std::time::Instant::now());
            }
            match talk_rx.recv_timeout(Duration::from_millis(500)) {
                Ok(Job::Talk(mut talk, hello)) => {
                    let said = match hello {
                        Some(h) => patientchat::answer_to(h, &mut talk, &mut store, fill),
                        None => patientchat::answer(&mut talk, &mut store, fill),
                    };
                    if let Ok(patientchat::Outcome::Talked { received, .. }) = said {
                        if received > 0 {
                            let _ = events_tx.send(Event::Received(received));
                        }
                    }
                }
                Ok(Job::Addresses(list)) => note_addresses(&db, &list),
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => return,
            }
        }
    });
    Waiting {
        stop,
        held,
        officine: shared,
        relays: relays_on,
        events,
    }
}

/// **La voie directe d'un poste**, tenue tant que rien ne l'arrête : les
/// adresses de la machine relues toutes les dix minutes (une minute tant
/// qu'une adresse n'a pas pu être tenue), l'écoute refaite quand elles
/// changent, le port de la box ouvert et renouvelé.
fn listen_direct(
    d: Direct,
    box_id: String,
    talk_tx: mpsc::SyncSender<Job>,
    events_tx: mpsc::Sender<Event>,
    board: BoardCache,
    relays: RelaysWanted,
    stop: Arc<AtomicBool>,
) {
    let generation = DIRECT_GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
    let (links_tx, links) = mpsc::channel::<direct::Link>();
    {
        let (talk_tx, stop) = (talk_tx.clone(), Arc::clone(&stop));
        std::thread::spawn(move || {
            while !stop.load(Ordering::SeqCst) {
                let Ok(link) = links.recv_timeout(Duration::from_millis(500)) else {
                    continue;
                };
                // Each page read on its own thread, up to its first frame
                // (the link is bounded in time): the board is answered
                // here, from what the base thread read last; a patient
                // goes on to the base thread.
                let (talk_tx, board) = (talk_tx.clone(), Arc::clone(&board));
                std::thread::spawn(move || route(link, &talk_tx, &board));
            }
        });
    }
    let mut listening: Option<(Vec<IpAddr>, direct::Listening)> = None;
    let mut waiting_at: Option<(String, Vec<std::net::SocketAddr>)> = None;
    while !stop.load(Ordering::SeqCst) {
        let wanted = relays.lock().ok().and_then(|r| r.clone());
        // Where to listen: this machine's address on the officine's
        // network (which the box translates, by UPnP or by hand) and its
        // public IPv6 addresses — or what the caller named.
        let ips: Vec<IpAddr> = match &d.at {
            Some(at) => at.clone(),
            None => {
                let mut ips: Vec<IpAddr> =
                    crate::reach::lan_v4().map(IpAddr::V4).into_iter().collect();
                ips.extend(
                    crate::reach::global_ipv6()
                        .into_iter()
                        .take(2)
                        .map(IpAddr::V6),
                );
                ips
            }
        };
        // Again when the addresses change, or when one could not be held
        // (the port still in use by the listening before this one).
        let renew = waiting_at != wanted
            || listening
                .as_ref()
                .is_none_or(|(had, l)| *had != ips || l.bound.len() < had.len());
        if renew {
            // The old sockets closed first: the port is the same.
            drop(listening.take());
            let sockets = direct::bind_all(&ips, d.port);
            let (group, at) = wanted.clone().unwrap_or_default();
            let l =
                direct::serve_relayed(sockets, &at, &group, &d.identity, &box_id, links_tx.clone());
            // A relay holds the wait within a round trip: a moment, so
            // that the addresses said below already carry it.
            let until = std::time::Instant::now() + Duration::from_secs(3);
            while !at.is_empty()
                && l.relayed().len() < at.len()
                && std::time::Instant::now() < until
            {
                std::thread::sleep(Duration::from_millis(50));
            }
            listening = Some((ips.clone(), l));
            waiting_at = wanted;
        }
        // The router's port, asked again before its lease runs out.
        let lan = ips
            .iter()
            .any(|ip| matches!(ip, IpAddr::V4(v4) if !crate::reach::is_public_v4(v4)));
        let public_v4 = (d.at.is_none() && lan)
            .then(|| crate::reach::open_udp_mapping(d.port))
            .flatten();
        let mut public: Vec<String> = Vec::new();
        let mut bound: Vec<String> = Vec::new();
        let complete = listening
            .as_ref()
            .is_some_and(|(had, l)| l.bound.len() >= had.len());
        if let Some((_, l)) = &listening {
            for a in &l.bound {
                bound.push(a.to_string());
                match a.ip() {
                    IpAddr::V4(v4) if crate::reach::is_public_v4(&v4) || d.at.is_some() => {
                        public.push(a.to_string())
                    }
                    IpAddr::V4(_) => {
                        // The port the router granted: PCP and NAT-PMP
                        // may grant another than the one asked for.
                        if let Some(p) = public_v4.map(|p| p.to_string()) {
                            if !public.contains(&p) {
                                public.push(p);
                            }
                        }
                    }
                    IpAddr::V6(_) => public.push(a.to_string()),
                }
            }
            // Last: a relay is the way when none of this post's own is.
            for a in l.relayed() {
                public.push(a.to_string());
            }
        }
        let _ = events_tx.send(Event::Direct {
            public: public.clone(),
            bound,
        });
        if talk_tx.send(Job::Addresses(public)).is_err() {
            break;
        }
        // A relay not holding the wait yet: looked at again within the
        // minute, to be said once it does.
        let unheld = waiting_at.as_ref().is_some_and(|(_, at)| {
            listening
                .as_ref()
                .is_some_and(|(_, l)| l.relayed().len() < at.len())
        });
        let every = if complete && !unheld {
            ADDRESSES_EVERY
        } else {
            Duration::from_secs(60)
        };
        let until = std::time::Instant::now() + every;
        while std::time::Instant::now() < until && !stop.load(Ordering::SeqCst) {
            // The relays to wait at changed (read from the base, or the
            // setting turned on or off): at once, not at the next pass.
            if relays.lock().is_ok_and(|r| *r != waiting_at) {
                break;
            }
            std::thread::sleep(Duration::from_millis(500));
        }
    }
    drop(listening);
    // Only the newest listening closes the router's port: one started
    // since has opened it again for itself.
    if d.at.is_none() && DIRECT_GENERATION.load(Ordering::SeqCst) == generation {
        crate::reach::close_udp_mapping(d.port);
    }
}

/// **Où attendre aux relais** : le groupe de postes de cette base (un
/// trousseau entier — un compagnon n'attend pas), et les relais que les
/// officines du réseau offrent, une adresse IP et un port chacun.
fn relays_wanted(db: &Db) -> Result<Option<(String, Vec<std::net::SocketAddr>)>, String> {
    let Some(group) = crate::postes::Posts::identity(db)?.group_hex() else {
        return Ok(None);
    };
    // IPv4: a relay listens there only (`relay::serve_udp`).
    let at: Vec<std::net::SocketAddr> = crate::network::offered_relays(db)
        .iter()
        .filter_map(|a| crate::network::usable_announced(a)?.parse().ok())
        .filter(std::net::SocketAddr::is_ipv4)
        .take(MOST_RELAYS)
        .collect();
    Ok((!at.is_empty()).then_some((group, at)))
}

/// Une page de la voie directe : son premier mot lu ici. Le tableau
/// demandé est rendu tel que la base l'a lu en dernier ; tout autre mot
/// part au fil de la base avec la conversation.
fn route(mut link: direct::Link, talk_tx: &mpsc::SyncSender<Job>, board: &BoardCache) {
    let Ok(hello) = patientchat::Frames::recv(&mut link) else {
        return;
    };
    if hello == patientchat::BOARD {
        // Not read yet (the base thread just started): it answers itself.
        if let Some(bytes) = board.lock().ok().and_then(|b| b.clone()) {
            let _ = patientchat::Frames::send(&mut link, &bytes);
            return;
        }
    }
    let _ = talk_tx.send(Job::Talk(Box::new(link), Some(hello)));
}

/// Une attente, tenue dans `held` le temps qu'elle dure — et **retirée
/// à sa fin, quelle qu'elle soit**, par son numéro : gardée, la copie
/// tenait la connexion ouverte après une attente échue (une toutes les
/// quatre minutes), la passerelle la croyait vivante et le poste perdait
/// un descripteur à chaque fois.
fn wait_once(
    address: &str,
    secret: &str,
    held: &Mutex<Vec<(u64, std::net::TcpStream)>>,
) -> std::io::Result<std::net::TcpStream> {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let me = NEXT.fetch_add(1, Ordering::SeqCst);
    let stream = crate::passerelle::wait_with(address, secret, PATIENCE, |s| {
        if let (Ok(mut h), Ok(c)) = (held.lock(), s.try_clone()) {
            h.push((me, c));
        }
    });
    if let Ok(mut h) = held.lock() {
        h.retain(|(id, _)| *id != me);
    }
    stream
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Swept(std::path::PathBuf);
    impl Drop for Swept {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// The published page is ours byte for byte, or the officine hears
    /// it; one that cannot be read says why.
    #[test]
    fn the_published_page_is_compared_to_ours() {
        let ours = |url: &str| -> Result<Vec<u8>, String> {
            assert!(url.starts_with(&patientchat::page_url()), "{url}");
            let name = url.rsplit('/').next().unwrap();
            patientchat::PAGE_FILES
                .iter()
                .find(|(n, _)| *n == name)
                .map(|(_, b)| b.as_bytes().to_vec())
                .ok_or_else(|| "404".to_owned())
        };
        assert_eq!(check_page(ours), PageCheck::Same);
        let altered = |url: &str| {
            let mut b = ours(url)?;
            if url.ends_with("app.js") {
                b.extend_from_slice(b"fetch('https://ailleurs.example', {body: texte});");
            }
            Ok(b)
        };
        assert_eq!(check_page(altered), PageCheck::Different);
        assert_eq!(
            check_page(|_| Err("404".to_owned())),
            PageCheck::Unread("404".into())
        );
    }

    #[test]
    fn opening_hours_read_as_a_patient_reads_them() {
        let row = |d: &str, f: &str, t: &str| (d.to_owned(), f.to_owned(), t.to_owned());
        assert_eq!(
            opening_hours(&[
                row("Lundi", "09:00", "12:30"),
                row("lundi", "14:00", "19:30"),
                row("samedi", "09:00", "12:00"),
                row("", "09:00", "12:00"),
                row("mardi", "09:00", ""),
            ]),
            "lundi 09:00-12:30 et 14:00-19:30 ; samedi 09:00-12:00"
        );
        assert_eq!(opening_hours(&[]), "");
    }

    #[test]
    fn the_officine_addresses_are_every_posts_recent_ones() {
        let row = |k: &str, v: &str| (format!("{ADDRESSES}{k}"), v.to_owned());
        let rows = [
            row("1", "2026-10-15 203.0.113.7:7748,[2001:db8::1]:7748"),
            row("2", "2026-10-16 198.51.100.2:7748,203.0.113.7:7748"),
            row("3", "2026-08-01 192.0.2.9:7748"),
            row("4", "illisible"),
            row("5", "2026-10-16 "),
        ];
        assert_eq!(
            officine_addresses(&rows, "2026-10-16"),
            [
                "198.51.100.2:7748",
                "203.0.113.7:7748",
                "[2001:db8::1]:7748"
            ],
            "most recent post first, no twice, none from a post silent a month"
        );
    }

    /// **Where the direct way waits**: nowhere before the base is a
    /// post of a group, then at the relays the network offers — an IPv4
    /// and a port each, never a name or the loopback a member could
    /// announce.
    #[test]
    fn the_direct_way_waits_at_the_relays_the_network_offers() {
        let dir = std::env::temp_dir().join(format!(
            "bpm-caddy-patientpost-relays-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let _swept = Swept(dir.clone());
        let db = Db::open(&dir.join("base.db"), "secret").unwrap();
        let offer = |device: &str, text: &str| {
            db.set_setting(&crate::network::relay_key(device), text, None, "", "")
                .unwrap()
        };
        offer("a", "82.64.1.2:7746");
        offer(
            "b",
            "127.0.0.1:7746,relais.example:7746,[2a01:cb00::1]:7746",
        );
        assert_eq!(relays_wanted(&db), Ok(None), "pas de groupe");
        let mut posts = crate::postes::Posts::load(&db).unwrap();
        posts.found(&db, "Comptoir", "2026-10-21").unwrap();
        let group = posts.group_hex().unwrap();
        let (g, at) = relays_wanted(&db).unwrap().unwrap();
        assert_eq!(g, group);
        let mut at: Vec<String> = at.iter().map(|a| a.to_string()).collect();
        at.sort();
        // IPv4 only: a relay listens there, not in IPv6.
        assert_eq!(at, vec!["82.64.1.2:7746"]);
    }

    /// The direct way, from a page to the base and back: the post listens,
    /// says and keeps its addresses, answers a patient and the board.
    #[test]
    fn a_patient_writes_to_the_base_by_the_direct_way() {
        let dir = std::env::temp_dir().join(format!(
            "bpm-caddy-patientpost-direct-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let _swept = Swept(dir.clone());
        let path = dir.join("base.db");
        let db = Db::open(&path, "secret").unwrap();
        let pid = db.add_patient("Durand", "Anne", "1950-01-02").unwrap();
        let key = patientchat::new_key(fill);
        db.open_patient_chat(pid, &patientchat::key_text(&key), None, "2026-10-14", "AB")
            .unwrap()
            .unwrap();
        let board = crate::tableau::Tableau {
            lignes: vec![crate::tableau::Ligne {
                canal: crate::tableau::Canal::Produit,
                titre: "Amoxicilline buvable".into(),
                ..Default::default()
            }],
        };
        assert!(db
            .set_setting(
                crate::tableau::SETTING,
                &board.text(),
                None,
                "2026-10-16",
                "AB"
            )
            .unwrap());
        let secret = "d4".repeat(32);
        let box_id = patientchat::box_of_secret(&secret).unwrap();
        let identity = direct::Identity::draw().unwrap();
        let waiting = spawn(
            path.clone(),
            "secret".into(),
            "",
            secret,
            Officine {
                nom: "Pharmacie du Port".into(),
                ..Default::default()
            },
            Some(Direct {
                identity: identity.clone(),
                port: 0,
                at: Some(vec!["127.0.0.1".parse().unwrap()]),
                relays: false,
            }),
        );
        let at = loop {
            match waiting
                .events
                .recv_timeout(Duration::from_secs(10))
                .unwrap()
            {
                Event::Direct { public, .. } => break public,
                _ => continue,
            }
        };
        assert_eq!(at.len(), 1);
        let to: Vec<std::net::SocketAddr> = at.iter().map(|a| a.parse().unwrap()).collect();
        // The board, for anyone.
        let mut page = direct::tests::dial(&to, &box_id, identity.fingerprint()).unwrap();
        patientchat::Frames::send(&mut page, patientchat::BOARD).unwrap();
        let shown: crate::tableau::Public =
            serde_json::from_slice(&patientchat::Frames::recv(&mut page).unwrap()).unwrap();
        assert_eq!(shown.officine, "Pharmacie du Port");
        assert_eq!(shown.lignes[0].titre, "Amoxicilline buvable");
        drop(page);
        // A patient: kept in the base, answered with the addresses.
        let mut page = direct::tests::dial(&to, &box_id, identity.fingerprint()).unwrap();
        let r = patientchat::talk_as_patient(
            &mut page,
            &key,
            [4; 16],
            &patientchat::Request {
                out: vec![patientchat::FromPatient {
                    id: "d1".into(),
                    at: "16/10/2026 08:00".into(),
                    text: "Bonjour".into(),
                }],
                seen: vec![],
            },
        )
        .unwrap()
        .unwrap();
        assert_eq!(r.got, vec!["d1".to_string()]);
        assert_eq!(r.adresses, at, "the post's addresses, kept for the page");
        assert_eq!(r.tableau.len(), 1);
        assert_eq!(db.unread_patient_messages().unwrap(), vec![(pid, 1)]);
        assert!(db
            .setting(&format!("{ADDRESSES}0"))
            .unwrap()
            .ends_with(&at[0]));
        waiting.stop();
    }

    /// **The real page in a real browser, by the direct way**: the page
    /// served on 18755 as GitHub Pages would, a post on 17748. Prints the
    /// two links — the patient's and the board's — and answers for eighty
    /// seconds (`--ignored --nocapture`, then open them; Firefox needs
    /// `media.peerconnection.ice.loopback` to reach 127.0.0.1).
    #[test]
    #[ignore]
    fn serve_directly_for_a_browser() {
        let dir = std::env::temp_dir().join(format!(
            "bpm-caddy-patientpost-browser-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let _swept = Swept(dir.clone());
        let path = dir.join("base.db");
        let db = Db::open(&path, "secret").unwrap();
        let pid = db.add_patient("Durand", "Anne", "1950-01-02").unwrap();
        let key = [8u8; 32];
        let kid = db
            .open_patient_chat(pid, &patientchat::key_text(&key), None, "2026-10-14", "AB")
            .unwrap()
            .unwrap();
        db.write_patient_message(kid, "Votre commande est prête.", "AB")
            .unwrap();
        let today = db.today_iso().unwrap();
        let board = crate::tableau::Tableau {
            lignes: vec![
                crate::tableau::Ligne {
                    canal: crate::tableau::Canal::Garde,
                    titre: "Pharmacie du Marché, 2 place du Marché".into(),
                    detail: "Ce soir de 19 h à 9 h : sonner à la grille.".into(),
                    du: today.clone(),
                    au: today.clone(),
                },
                crate::tableau::Ligne {
                    canal: crate::tableau::Canal::Produit,
                    titre: "Amoxicilline buvable".into(),
                    ..Default::default()
                },
                crate::tableau::Ligne {
                    canal: crate::tableau::Canal::Info,
                    titre: "Vaccination contre la grippe sans rendez-vous".into(),
                    ..Default::default()
                },
            ],
        };
        db.set_setting(crate::tableau::SETTING, &board.text(), None, &today, "AB")
            .unwrap();
        let page = std::net::TcpListener::bind("127.0.0.1:18755").unwrap();
        std::thread::spawn(move || {
            for s in page.incoming().flatten() {
                std::thread::spawn(move || {
                    let mut s = s;
                    let mut head = [0u8; 2048];
                    let n = std::io::Read::read(&mut s, &mut head).unwrap_or(0);
                    let line = String::from_utf8_lossy(&head[..n]).to_string();
                    let name = line
                        .split_whitespace()
                        .nth(1)
                        .unwrap_or("/")
                        .split('?')
                        .next()
                        .unwrap_or("/")
                        .trim_start_matches('/')
                        .to_owned();
                    let name = if name.is_empty() {
                        "index.html".into()
                    } else {
                        name
                    };
                    let (status, body) =
                        match patientchat::PAGE_FILES.iter().find(|(n, _)| *n == name) {
                            Some((_, b)) => ("200 OK", b.as_bytes()),
                            None => ("404 Not Found", b"".as_slice()),
                        };
                    let kind = match name.rsplit('.').next() {
                        Some("js") => "text/javascript; charset=utf-8",
                        Some("css") => "text/css; charset=utf-8",
                        _ => "text/html; charset=utf-8",
                    };
                    let _ = std::io::Write::write_all(
                        &mut s,
                        format!(
                            "HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\n\
                             Connection: close\r\n\r\n",
                            body.len()
                        )
                        .as_bytes(),
                    );
                    let _ = std::io::Write::write_all(&mut s, body);
                });
            }
        });
        // Firefox reaches no loopback address by default: BPM_TEST_IP=<an
        // address of this machine on its network>.
        let ip: IpAddr = std::env::var("BPM_TEST_IP")
            .ok()
            .and_then(|t| t.parse().ok())
            .unwrap_or_else(|| "127.0.0.1".parse().unwrap());
        let secret = "5b".repeat(32);
        let box_id = patientchat::box_of_secret(&secret).unwrap();
        let identity = direct::Identity::draw().unwrap();
        let waiting = spawn(
            path.clone(),
            "secret".into(),
            "",
            secret,
            Officine {
                nom: "Pharmacie du Port".into(),
                horaires: "lundi 09:00-19:00".into(),
                ..Default::default()
            },
            Some(Direct {
                identity: identity.clone(),
                port: 17748,
                at: Some(vec![ip]),
                relays: false,
            }),
        );
        let reach = patientchat::Reach {
            fingerprint: direct::fingerprint_text(&identity.fingerprint()),
            addresses: vec![std::net::SocketAddr::new(ip, 17748).to_string()],
            gateway: String::new(),
        };
        let local = |l: String| l.replace(&patientchat::page_url(), "http://127.0.0.1:18755/");
        eprintln!(
            "PATIENT {}",
            local(patientchat::link(&reach, &box_id, &key))
        );
        eprintln!(
            "BOARD {}",
            local(patientchat::board_link(&reach, &box_id, true))
        );
        let until = std::time::Instant::now() + Duration::from_secs(80);
        while std::time::Instant::now() < until {
            if let Ok(e) = waiting.events.recv_timeout(Duration::from_secs(1)) {
                eprintln!("EVENT {e:?}");
            }
        }
        for m in db.patient_messages(pid).unwrap() {
            eprintln!("MESSAGE {} {}", m.from_patient, m.body);
        }
    }

    #[test]
    fn a_time_for_the_page() {
        assert_eq!(shown_at("2026-10-14 09:05"), "14/10/2026 09:05");
        assert_eq!(shown_at("2026-10-14"), "14/10/2026");
    }

    /// From the gateway to the base and back: a page writes, the base
    /// keeps it once, the officine answers, the page reads the answer; a
    /// closed access is told so.
    #[test]
    fn a_patient_writes_to_the_base_through_the_gateway() {
        let dir =
            std::env::temp_dir().join(format!("bpm-caddy-patientpost-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let _swept = Swept(dir.clone());
        let path = dir.join("base.db");
        let db = Db::open(&path, "secret").unwrap();
        let pid = db.add_patient("Durand", "Anne", "1950-01-02").unwrap();
        let key = patientchat::new_key(fill);
        let kid = db
            .open_patient_chat(pid, &patientchat::key_text(&key), None, "2026-10-14", "AB")
            .unwrap()
            .unwrap();
        assert!(db
            .write_patient_message(kid, "Votre commande est prête.", "AB")
            .unwrap());

        let free = || {
            std::net::TcpListener::bind("127.0.0.1:0")
                .unwrap()
                .local_addr()
                .unwrap()
        };
        let (web, posts) = (free(), free());
        let stop = Arc::new(AtomicBool::new(false));
        let mut options = crate::passerelle::Options::new(web, posts);
        options.origins.push(format!("http://{web}"));
        crate::passerelle::serve(options, Arc::clone(&stop)).unwrap();
        let secret = "c3".repeat(32);
        let mut store = DbStore {
            db: &db,
            officine: Officine {
                nom: "Pharmacie du Port".into(),
                ..Default::default()
            },
        };
        let talk = |store: &mut DbStore, request: patientchat::Request| {
            let (posts_at, secret2) = (posts.to_string(), secret.clone());
            let waiter = std::thread::spawn(move || {
                crate::passerelle::wait(&posts_at, &secret2, Duration::from_secs(5)).unwrap()
            });
            std::thread::sleep(Duration::from_millis(200));
            let mut page = crate::passerelle::tests_page(web, &patientchat::box_id(&[0xc3; 32]));
            // The page speaks first: the gateway takes a post only then.
            let page_side = std::thread::spawn(move || {
                patientchat::talk_as_patient(&mut page, &key, [4; 16], &request).unwrap()
            });
            let mut post = waiter.join().unwrap();
            let o = patientchat::answer(&mut post, store, fill).unwrap();
            (o, page_side.join().unwrap())
        };
        let hello = |id: &str| patientchat::FromPatient {
            id: id.into(),
            at: "14/10/2026 08:00".into(),
            text: "Bonjour, ma commande ?".into(),
        };
        let (o, r) = talk(
            &mut store,
            patientchat::Request {
                out: vec![hello("p1")],
                seen: vec![],
            },
        );
        assert_eq!(
            o,
            patientchat::Outcome::Talked {
                key: kid,
                received: 1
            }
        );
        let r = r.unwrap();
        assert_eq!(r.got, vec!["p1".to_string()]);
        assert_eq!(r.messages.len(), 1);
        assert_eq!(r.messages[0].text, "Votre commande est prête.");
        // Sent again (the answer was lost): kept once; the reply seen is
        // not sent back.
        let seen = vec![r.messages[0].id];
        let (_, r) = talk(
            &mut store,
            patientchat::Request {
                out: vec![hello("p1")],
                seen,
            },
        );
        let r = r.unwrap();
        assert_eq!(r.got, vec!["p1".to_string()]);
        assert!(r.messages.is_empty());
        let all = db.patient_messages(pid).unwrap();
        assert_eq!(all.iter().filter(|m| m.from_patient).count(), 1);
        assert_eq!(db.unread_patient_messages().unwrap(), vec![(pid, 1)]);
        assert_eq!(db.mark_patient_messages_read(pid, "2026-10-14").unwrap(), 1);
        assert!(db.unread_patient_messages().unwrap().is_empty());
        // A new access closes the old one: the old phone is told.
        let shown = db.patient_chat_key(pid).unwrap().map(|k| k.id);
        assert_eq!(shown, Some(kid));
        assert_eq!(
            db.open_patient_chat(
                pid,
                &patientchat::key_text(&[1; 32]),
                None,
                "2026-10-14",
                "AB"
            )
            .unwrap(),
            None,
            "against what the screen showed"
        );
        db.open_patient_chat(
            pid,
            &patientchat::key_text(&[1; 32]),
            shown,
            "2026-10-14",
            "AB",
        )
        .unwrap()
        .unwrap();
        assert!(!db.write_patient_message(kid, "trop tard", "AB").unwrap());
        // Two posts each gave a new code before seeing each other: one
        // stays open, the highest.
        db.execute_for_tests(&format!(
            "INSERT INTO patient_chat_keys (id, patient_id, key) VALUES (9999, {pid}, 'x')"
        ));
        assert_eq!(db.close_older_patient_chats(pid, "2026-10-15").unwrap(), 1);
        assert_eq!(db.patient_chat_key(pid).unwrap().unwrap().id, 9999);
        assert_eq!(db.close_older_patient_chats(pid, "2026-10-15").unwrap(), 0);
        let (o, r) = talk(
            &mut store,
            patientchat::Request {
                out: vec![hello("p2")],
                seen: vec![],
            },
        );
        assert_eq!(o, patientchat::Outcome::Closed);
        assert_eq!(r.unwrap_err(), patientchat::CLOSED.to_vec());
        stop.store(true, Ordering::SeqCst);
    }
}
