//! **La passerelle des patients** : un serveur sans mémoire entre la page
//! qu'un patient ouvre sur son téléphone et un poste de son officine
//! (`patientchat.rs`, `docs/CHAT_PATIENT.md`).
//!
//! Un navigateur ne parle pas TCP : il ne sait ouvrir qu'un WebSocket, et
//! depuis une page en HTTPS, un WebSocket chiffré (`wss://`), qui demande
//! un nom de domaine et un certificat. Les postes, eux, n'ont ni l'un ni
//! l'autre, et souvent pas d'adresse joignable. La passerelle se tient
//! donc sur un serveur, derrière un proxy qui porte le certificat
//! (Caddy, nginx), et fait trois choses :
//!
//! * elle **ne sert pas la page** : publiée à part, sur GitHub Pages
//!   (`patientchat::PAGE_SITE`), depuis le dépôt public — qui tient la
//!   passerelle ne peut pas changer le code qui chiffre, et les postes
//!   vérifient que la page publiée est celle qu'ils livrent
//!   (`patientpost::check_page`). Elle n'accepte que les WebSocket de
//!   cette origine (`Options::origins`) ;
//! * elle tient **les postes qui attendent** : un poste de l'officine
//!   l'appelle sur son port à lui et dit le secret de sa boîte ; il attend
//!   là qu'une page le demande (`BPMPATIENT1 attente <secret>`, puis
//!   `ENTRE`) ;
//! * elle **met en relation** une page qui ouvre `/ws/<boîte>` avec un
//!   poste qui attend à cette boîte, et passe les trames de l'une à
//!   l'autre — un message WebSocket d'un côté, quatre octets de longueur
//!   et la trame de l'autre — sans les lire : elles sont chiffrées de
//!   bout en bout sous la clé du patient, que la passerelle n'a pas.
//!
//! Aucun message n'est écrit sur son disque. Aucun poste n'attend : la
//! page l'apprend (fermeture 4004) et garde le message sur le téléphone.
//! Ce qu'elle sait : qu'une boîte échange, quand, combien, et depuis
//! quelle adresse — et c'est ce que son **journal** garde, si on le lui
//! demande (`--journal`), pour la sécurité (RGPD, art. 32) : une ligne
//! par connexion, jamais une trame, un fichier par jour, effacé au bout
//! de `--garde` jours (au plus `KEEP_MOST`).
//!
//! Le numéro de boîte est dans chaque code remis aux patients ; le
//! secret, que seuls les postes tiennent, est ce qui permet d'attendre.
//! Il passe en clair sur le port des postes : qui l'intercepterait
//! pourrait attendre à la place de l'officine et faire échouer les
//! conversations — pas en lire une, ni passer pour l'officine, faute des
//! clés des patients.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use base64::Engine;
use sha1::{Digest, Sha1};

use crate::patientchat::{self, Frames};

/// La préface d'un poste qui attend.
pub const PREFACE: &str = "BPMPATIENT1";
/// Ce que la passerelle écrit au poste qui attendait : une page est là.
pub const ENTER: &str = "ENTRE";
/// Où la passerelle sert la page et les WebSocket, quand on ne dit rien :
/// derrière le proxy, sur la machine.
pub const DEFAULT_WEB: &str = "127.0.0.1:8746";
/// Le port où les postes appellent, quand on ne dit rien — à côté de ceux
/// des postes (7743) et du relais (7746).
pub const DEFAULT_POSTS_PORT: u16 = 7747;
/// La fermeture WebSocket qui dit « aucun poste de l'officine n'attend ».
pub const CLOSE_ABSENT: u16 = 4004;

/// Combien de postes d'une boîte peuvent attendre à la fois — au-delà,
/// une nouvelle attente est refusée.
const WAITING_PER_BOX: usize = 4;
/// Combien de boîtes à la fois.
const MOST_BOXES: usize = 1024;
/// Une attente dure au plus ceci…
const WAIT_BOUND: Duration = Duration::from_secs(5 * 60);
/// …et le poste rappelle un peu avant.
pub const WAIT_RENEW: Duration = Duration::from_secs(4 * 60);
/// Combien de connexions à la fois, en tout.
const MOST_CONNECTIONS: usize = 512;
/// Une conversation dure au plus — quatre trames, quelques secondes
/// d'habitude.
const TALK_BOUND: Duration = Duration::from_secs(60);
/// Ce qu'une conversation porte au plus, dans chaque sens.
const TALK_BYTES: usize = 4 * patientchat::FRAME_MAX;
/// Le temps de dire sa requête (ou sa préface).
const HEAD_WAIT: Duration = Duration::from_secs(5);
/// La plus longue requête HTTP lue.
const HEAD_MAX: usize = 8 * 1024;
/// Combien de connexions une adresse ouvre par minute sur le port des
/// postes — un poste rappelle toutes les quatre minutes…
const POST_CALLS_PER_MINUTE: usize = 30;
/// …et sur celui des pages : une page en coûte quatre à son ouverture,
/// puis deux par minute tant qu'elle est ouverte, et le Wi-Fi d'une
/// officine ou un opérateur mobile met beaucoup de patients derrière une
/// seule adresse. Compté à part : les pages n'enlèvent rien aux postes.
const PAGE_CALLS_PER_MINUTE: usize = 240;
/// Combien d'attentes une même adresse tient à la fois, toutes boîtes
/// confondues : des secrets tirés au hasard ouvrent des boîtes, et sans
/// borne quelques adresses rempliraient la passerelle de toutes les
/// officines.
const WAITS_PER_IP: usize = 16;

/// Ce que la passerelle répond à qui l'ouvre dans un navigateur : rien à
/// exécuter, rien à encadrer.
const CSP: &str = "default-src 'none'; frame-ancestors 'none'";

/// Le plus long qu'un journal se garde, en jours : six mois — la
/// fourchette de la CNIL pour des journaux de sécurité est de trois à
/// six (`docs/CHAT_PATIENT.md`, et la page le dit au patient).
pub const KEEP_MOST: u32 = 183;
/// Quand on ne dit rien : trois mois.
pub const KEEP_DEFAULT: u32 = 92;
/// Au-delà, un jour ne s'écrit plus : une inondation ne remplit pas le
/// disque (une ligne de journal fait une centaine d'octets).
const JOURNAL_LINES_PER_DAY: usize = 1_000_000;

/// Comment la passerelle tient.
#[derive(Clone, Debug)]
pub struct Options {
    /// Les WebSocket des pages, derrière le proxy.
    pub web: SocketAddr,
    /// Les postes qui attendent.
    pub posts: SocketAddr,
    /// Les hôtes d'où une page peut appeler (l'en-tête `Origin`).
    pub origins: Vec<String>,
    /// Où écrire le journal des connexions ; `None`, rien.
    pub journal: Option<std::path::PathBuf>,
    /// Combien de jours il se garde (borné à `KEEP_MOST`).
    pub keep_days: u32,
}

impl Options {
    pub fn new(web: SocketAddr, posts: SocketAddr) -> Self {
        Options {
            web,
            posts,
            origins: vec![host_of(patientchat::PAGE_SITE)],
            journal: None,
            keep_days: KEEP_DEFAULT,
        }
    }
}

/// **Le journal des connexions** : `AAAA-MM-JJ.log` par jour (UTC), une
/// ligne par connexion — l'heure, page ou poste, l'adresse, la boîte, ce
/// qui s'est passé. Jamais une trame, jamais un secret. Les jours plus
/// vieux que `keep` sont effacés quand un jour commence.
pub struct Journal {
    dir: std::path::PathBuf,
    keep: u32,
    today: Mutex<Option<(i64, std::fs::File, usize)>>,
}

impl Journal {
    pub fn open(dir: &std::path::Path, keep: u32) -> std::io::Result<Self> {
        std::fs::create_dir_all(dir)?;
        Ok(Journal {
            dir: dir.to_owned(),
            keep: keep.clamp(1, KEEP_MOST),
            today: Mutex::new(None),
        })
    }

    /// Écrire une ligne maintenant.
    pub fn note(&self, what: &str) {
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        self.note_at(secs, what);
    }

    /// Écrire une ligne à l'heure `secs` (depuis 1970, UTC).
    pub fn note_at(&self, secs: u64, what: &str) {
        let Ok(mut today) = self.today.lock() else {
            return;
        };
        let day = i64::try_from(secs / 86_400).unwrap_or(0);
        if today.as_ref().map(|t| t.0) != Some(day) {
            *today = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(
                    self.dir
                        .join(format!("{}.log", crate::date::from_days(day))),
                )
                .ok()
                .map(|f| (day, f, 0));
            self.forget_before(day - i64::from(self.keep) + 1);
        }
        let Some((_, file, lines)) = today.as_mut() else {
            return;
        };
        if *lines >= JOURNAL_LINES_PER_DAY {
            return;
        }
        *lines += 1;
        let s = secs % 86_400;
        let line = if *lines == JOURNAL_LINES_PER_DAY {
            "journal plein pour ce jour".to_owned()
        } else {
            what.replace(['\n', '\r'], " ")
        };
        let _ = writeln!(
            file,
            "{}T{:02}:{:02}:{:02}Z {line}",
            crate::date::from_days(day),
            s / 3600,
            s / 60 % 60,
            s % 60
        );
    }

    /// Effacer les jours d'avant `first`.
    fn forget_before(&self, first: i64) {
        let Ok(entries) = std::fs::read_dir(&self.dir) else {
            return;
        };
        for e in entries.flatten() {
            let name = e.file_name();
            let Some(day) = name
                .to_str()
                .and_then(|n| n.strip_suffix(".log"))
                .and_then(crate::date::to_days)
            else {
                continue;
            };
            if day < first {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
}

/// Ce qu'on fait d'une ligne de journal : rien sans journal.
fn note(journal: &Option<Arc<Journal>>, what: impl FnOnce() -> String) {
    if let Some(j) = journal {
        j.note(&what());
    }
}

struct Waiting {
    stream: TcpStream,
    since: Instant,
    ip: IpAddr,
}

/// **Une attente encore tenue** : un poste qui a raccroché (son attente
/// renouvelée, un réglage changé, l'application fermée) a fermé sa
/// connexion, et le premier coup d'œil le voit — lu sans attendre, rien à
/// lire n'est pas une fin. Sans cela, une page était mise en relation
/// avec une attente morte, et les places de la boîte restaient prises.
fn alive(stream: &TcpStream) -> bool {
    if stream.set_nonblocking(true).is_err() {
        return false;
    }
    let mut b = [0u8; 1];
    let alive = matches!(stream.peek(&mut b), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock);
    stream.set_nonblocking(false).is_ok() && alive
}

type Boxes = Arc<Mutex<HashMap<String, Vec<Waiting>>>>;
type Calls = Arc<Mutex<HashMap<IpAddr, Vec<Instant>>>>;

/// **Tenir la passerelle ouverte** jusqu'à ce que `stop` passe à vrai :
/// les WebSocket des pages sur `options.web`, les postes sur
/// `options.posts`. Rend tout de suite ; elle vit sur ses propres fils.
pub fn serve(options: Options, stop: Arc<AtomicBool>) -> std::io::Result<()> {
    let web = TcpListener::bind(options.web)?;
    let posts = TcpListener::bind(options.posts)?;
    let journal = match &options.journal {
        Some(dir) => Some(Arc::new(Journal::open(dir, options.keep_days)?)),
        None => None,
    };
    let origins: Arc<Vec<String>> = Arc::new(
        options
            .origins
            .iter()
            .map(|o| host_of(o))
            .filter(|o| !o.is_empty())
            .collect(),
    );
    web.set_nonblocking(true)?;
    posts.set_nonblocking(true)?;
    let boxes: Boxes = Arc::default();
    let page_calls: Calls = Arc::default();
    let post_calls: Calls = Arc::default();
    let open = Arc::new(AtomicUsize::new(0));
    for (listener, is_web) in [(web, true), (posts, false)] {
        let calls = if is_web { &page_calls } else { &post_calls };
        let (boxes, calls, open, stop, journal, origins) = (
            Arc::clone(&boxes),
            Arc::clone(calls),
            Arc::clone(&open),
            Arc::clone(&stop),
            journal.clone(),
            Arc::clone(&origins),
        );
        std::thread::spawn(move || {
            while !stop.load(Ordering::SeqCst) {
                let (stream, from) = match listener.accept() {
                    Ok(s) => s,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(50));
                        continue;
                    }
                    Err(_) => {
                        std::thread::sleep(Duration::from_millis(500));
                        continue;
                    }
                };
                if open.load(Ordering::SeqCst) >= MOST_CONNECTIONS {
                    if let Ok(mut b) = boxes.lock() {
                        prune(&mut b, &open);
                    }
                    if open.load(Ordering::SeqCst) >= MOST_CONNECTIONS {
                        note(&journal, || format!("plein {}", from.ip()));
                        continue;
                    }
                }
                open.fetch_add(1, Ordering::SeqCst);
                let (boxes, calls, open, journal, origins) = (
                    Arc::clone(&boxes),
                    Arc::clone(&calls),
                    Arc::clone(&open),
                    journal.clone(),
                    Arc::clone(&origins),
                );
                std::thread::spawn(move || {
                    let _ = stream.set_nonblocking(false);
                    let _ = if is_web {
                        let place = Place {
                            boxes: &boxes,
                            calls: &calls,
                            open: &open,
                            journal: &journal,
                            origins: &origins,
                        };
                        take_page(stream, from, &place)
                    } else {
                        take_post(stream, from.ip(), &boxes, &calls, &open, &journal)
                    };
                    open.fetch_sub(1, Ordering::SeqCst);
                });
            }
        });
    }
    Ok(())
}

/// Lâcher les attentes échues ou raccrochées, et les décompter.
fn prune(b: &mut HashMap<String, Vec<Waiting>>, open: &AtomicUsize) {
    b.retain(|_, v| {
        let before = v.len();
        v.retain(|x| x.since.elapsed() < WAIT_BOUND && alive(&x.stream));
        open.fetch_sub(before - v.len(), Ordering::SeqCst);
        !v.is_empty()
    });
}

/// Une adresse qui appelle trop souvent est lâchée.
fn admit(calls: &Mutex<HashMap<IpAddr, Vec<Instant>>>, ip: IpAddr, most: usize) -> bool {
    let Ok(mut calls) = calls.lock() else {
        return false;
    };
    let now = Instant::now();
    calls.retain(|_, v| {
        v.retain(|t| now.duration_since(*t) < Duration::from_secs(60));
        !v.is_empty()
    });
    if calls.len() >= 4096 && !calls.contains_key(&ip) {
        return false;
    }
    let seen = calls.entry(ip).or_default();
    seen.push(now);
    seen.len() <= most
}

/// La préface d'un poste : le secret de sa boîte (64 chiffres
/// hexadécimaux). `None` pour tout le reste.
pub fn read_preface(line: &str) -> Option<String> {
    let mut parts = line.split_whitespace();
    if parts.next()? != PREFACE || parts.next()? != "attente" {
        return None;
    }
    let secret = parts.next()?.to_ascii_lowercase();
    (parts.next().is_none() && patientchat::is_box_secret(&secret)).then_some(secret)
}

/// La préface qu'un poste écrit.
pub fn preface(secret: &str) -> String {
    format!("{PREFACE} attente {secret}\n")
}

/// Un poste qui appelle : sa préface, puis il attend dans sa boîte.
fn take_post(
    stream: TcpStream,
    ip: IpAddr,
    boxes: &Boxes,
    calls: &Calls,
    open: &AtomicUsize,
    journal: &Option<Arc<Journal>>,
) -> std::io::Result<()> {
    if !admit(calls, ip, POST_CALLS_PER_MINUTE) {
        note(journal, || format!("poste {ip} - trop d'appels"));
        return Ok(());
    }
    let line = read_line(&stream)?;
    let Some(secret) = read_preface(&line) else {
        note(journal, || format!("poste {ip} - illisible"));
        return Ok(());
    };
    let Some(bytes) = unhex(&secret) else {
        return Ok(());
    };
    let id = patientchat::box_id(&bytes);
    let say = |why: &str| note(journal, || format!("poste {ip} {id} {why}"));
    let Ok(mut b) = boxes.lock() else {
        return Ok(());
    };
    prune(&mut b, open);
    if !b.contains_key(&id) && b.len() >= MOST_BOXES {
        say("trop de boîtes");
        return Ok(());
    }
    if b.values().flatten().filter(|w| w.ip == ip).count() >= WAITS_PER_IP {
        say("trop d'attentes pour l'adresse");
        return Ok(());
    }
    let list = b.entry(id.clone()).or_default();
    if list.len() >= WAITING_PER_BOX {
        say("trop d'attentes pour la boîte");
        return Ok(());
    }
    say("attend");
    // Held past this thread: counted again until it is put in touch or
    // let go.
    open.fetch_add(1, Ordering::SeqCst);
    list.push(Waiting {
        stream,
        since: Instant::now(),
        ip,
    });
    Ok(())
}

fn unhex(text: &str) -> Option<Vec<u8>> {
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(text.get(i..i + 2)?, 16).ok())
        .collect()
}

/// La préface, lue octet par octet : ce qui la suit est déjà la
/// conversation.
fn read_line(mut stream: &TcpStream) -> std::io::Result<String> {
    let until = Instant::now() + HEAD_WAIT;
    let mut out = Vec::with_capacity(96);
    let mut byte = [0u8; 1];
    while out.len() < 128 {
        let left = until.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(std::io::Error::from(std::io::ErrorKind::TimedOut));
        }
        stream.set_read_timeout(Some(left))?;
        stream.read_exact(&mut byte)?;
        if byte[0] == b'\n' {
            break;
        }
        out.push(byte[0]);
    }
    Ok(String::from_utf8_lossy(&out).into_owned())
}

/// Une requête HTTP : la méthode, le chemin (sans requête) et les
/// en-têtes, en minuscules.
#[derive(Debug)]
struct Head {
    method: String,
    path: String,
    headers: HashMap<String, String>,
}

impl Head {
    fn header(&self, name: &str) -> &str {
        self.headers.get(name).map(String::as_str).unwrap_or("")
    }
}

/// Lire la requête jusqu'à la ligne vide, dans le temps et la taille
/// permis. Le WebSocket qui suit ne commence qu'après la réponse : rien
/// n'est avalé.
fn read_head(mut stream: &TcpStream) -> std::io::Result<Head> {
    let until = Instant::now() + HEAD_WAIT;
    let mut buf = Vec::with_capacity(1024);
    let mut chunk = [0u8; 1024];
    while !buf.windows(4).any(|w| w == b"\r\n\r\n") {
        if buf.len() > HEAD_MAX {
            return Err(std::io::Error::from(std::io::ErrorKind::InvalidData));
        }
        let left = until.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(std::io::Error::from(std::io::ErrorKind::TimedOut));
        }
        stream.set_read_timeout(Some(left))?;
        let n = stream.read(&mut chunk)?;
        if n == 0 {
            return Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof));
        }
        buf.extend_from_slice(&chunk[..n]);
    }
    parse_head(&String::from_utf8_lossy(&buf))
        .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::InvalidData))
}

fn parse_head(text: &str) -> Option<Head> {
    let mut lines = text.split("\r\n");
    let mut first = lines.next()?.split(' ');
    let method = first.next()?.to_owned();
    let path = first.next()?.split(['?', '#']).next()?.to_owned();
    let mut headers = HashMap::new();
    for l in lines.take_while(|l| !l.is_empty()) {
        let (k, v) = l.split_once(':')?;
        headers.insert(k.trim().to_ascii_lowercase(), v.trim().to_owned());
    }
    Some(Head {
        method,
        path,
        headers,
    })
}

/// L'adresse du patient : celle que le proxy a vue (le dernier mot de
/// `X-Forwarded-For`) quand la connexion vient de la machine, sinon celle
/// de la connexion.
fn client_ip(peer: IpAddr, head: &Head) -> IpAddr {
    if peer.is_loopback() {
        if let Some(ip) = head
            .header("x-forwarded-for")
            .rsplit(',')
            .next()
            .and_then(|s| s.trim().parse().ok())
        {
            return ip;
        }
    }
    peer
}

fn respond(mut s: &TcpStream, status: &str, kind: &str, body: &[u8]) -> std::io::Result<()> {
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\n\
         Cache-Control: no-cache\r\nX-Content-Type-Options: nosniff\r\n\
         Referrer-Policy: no-referrer\r\nContent-Security-Policy: {CSP}\r\n\
         Connection: close\r\n\r\n",
        body.len()
    );
    s.write_all(head.as_bytes())?;
    s.write_all(body)?;
    s.flush()
}

/// La valeur de `Sec-WebSocket-Accept` (RFC 6455 § 4.2.2).
fn accept_key(key: &str) -> String {
    let mut h = Sha1::new();
    h.update(key.trim().as_bytes());
    h.update(b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
    base64::engine::general_purpose::STANDARD.encode(h.finalize())
}

/// Le nom d'hôte d'une origine (`https://hôte:port`), ou d'un en-tête
/// `Host`.
fn host_of(text: &str) -> String {
    let t = text.split_once("://").map(|(_, rest)| rest).unwrap_or(text);
    t.split('/').next().unwrap_or("").to_ascii_lowercase()
}

/// Ce qu'une page qui appelle trouve à la passerelle.
struct Place<'a> {
    boxes: &'a Boxes,
    calls: &'a Calls,
    open: &'a AtomicUsize,
    journal: &'a Option<Arc<Journal>>,
    origins: &'a [String],
}

/// Ce que la passerelle dit à qui l'ouvre dans un navigateur.
const HERE: &str = "Passerelle de la messagerie des patients de BPM-Caddy.\n\
                    La page des patients n'est pas servie ici : elle est publiée \
                    à part, et s'ouvre avec le code remis par la pharmacie.\n";

/// Une page qui appelle : un WebSocket vers une boîte.
fn take_page(stream: TcpStream, from: SocketAddr, place: &Place) -> std::io::Result<()> {
    let head = read_head(&stream)?;
    let ip = client_ip(from.ip(), &head);
    if !admit(place.calls, ip, PAGE_CALLS_PER_MINUTE) {
        note(place.journal, || format!("page {ip} - trop d'appels"));
        return respond(&stream, "429 Too Many Requests", "text/plain", b"");
    }
    if head.method != "GET" {
        return respond(&stream, "405 Method Not Allowed", "text/plain", b"");
    }
    match head.path.as_str() {
        "/" => respond(
            &stream,
            "200 OK",
            "text/plain; charset=utf-8",
            HERE.as_bytes(),
        ),
        p => match p.strip_prefix("/ws/") {
            Some(id) if patientchat::is_box_id(id) => {
                let id = id.to_ascii_lowercase();
                let outcome = upgrade(stream, &head, &id, place);
                note(place.journal, || {
                    format!(
                        "page {ip} {id} {}",
                        match &outcome {
                            Ok(o) => o,
                            Err(_) => "rompue",
                        }
                    )
                });
                outcome.map(|_| ())
            }
            _ => respond(&stream, "404 Not Found", "text/plain", b""),
        },
    }
}

/// Mettre une page en relation : ce qui s'est passé, pour le journal.
fn upgrade(
    mut stream: TcpStream,
    head: &Head,
    id: &str,
    place: &Place,
) -> std::io::Result<&'static str> {
    let (boxes, open) = (place.boxes, place.open);
    let key = head.header("sec-websocket-key");
    if !head
        .header("upgrade")
        .to_ascii_lowercase()
        .contains("websocket")
        || head.header("sec-websocket-version") != "13"
        || key.is_empty()
    {
        respond(&stream, "400 Bad Request", "text/plain", b"")?;
        return Ok("refusée");
    }
    // A page of another site does not reach the officine through here:
    // only the published page (a browser always says where it calls from).
    let origin = head.header("origin");
    if !origin.is_empty() && !place.origins.contains(&host_of(origin)) {
        respond(&stream, "403 Forbidden", "text/plain", b"")?;
        return Ok("autre origine");
    }
    stream.write_all(
        format!(
            "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\n\
             Connection: Upgrade\r\nSec-WebSocket-Accept: {}\r\n\r\n",
            accept_key(key)
        )
        .as_bytes(),
    )?;
    // **The page speaks first, before a post is taken**: a page that
    // opens and says nothing holds no post of the officine — it is let go
    // here, after the time to say a request.
    let hello = {
        let mut from = Until {
            stream: &stream,
            until: Instant::now() + HEAD_WAIT,
        };
        match read_frame(&mut from, true, patientchat::FRAME_MAX) {
            Ok(f) if f.op == OP_BINARY && f.fin => f.payload,
            _ => return Ok("muette"),
        }
    };
    // The post that came last is the likeliest alive; one gone away is
    // let go and the next tried. Each taken out of the box is no longer
    // held there: its count goes with it, whichever way.
    let post = loop {
        let next = boxes
            .lock()
            .ok()
            .and_then(|mut b| b.get_mut(id).and_then(Vec::pop));
        let Some(mut w) = next else {
            break None;
        };
        open.fetch_sub(1, Ordering::SeqCst);
        // A post that stops reading does not hold this thread either.
        let _ = w.stream.set_write_timeout(Some(HEAD_WAIT));
        if w.since.elapsed() < WAIT_BOUND
            && alive(&w.stream)
            && w.stream.write_all(format!("{ENTER}\n").as_bytes()).is_ok()
            && w.stream.send(&hello).is_ok()
        {
            break Some(w.stream);
        }
    };
    let Some(post) = post else {
        let mut reason = CLOSE_ABSENT.to_be_bytes().to_vec();
        reason.extend_from_slice(b"absente");
        write_frame(&mut stream, OP_CLOSE, &reason)?;
        return Ok("aucun poste");
    };
    bridge(stream, post);
    Ok("mise en relation")
}

const OP_CONTINUE: u8 = 0;
const OP_BINARY: u8 = 2;
const OP_CLOSE: u8 = 8;
const OP_PING: u8 = 9;
const OP_PONG: u8 = 10;

/// Écrire une trame WebSocket, du serveur : jamais masquée.
fn write_frame(w: &mut impl Write, op: u8, payload: &[u8]) -> std::io::Result<()> {
    let mut head = vec![0x80 | op];
    match payload.len() {
        n if n < 126 => head.push(n as u8),
        n if n <= u16::MAX as usize => {
            head.push(126);
            head.extend_from_slice(&(n as u16).to_be_bytes());
        }
        n => {
            head.push(127);
            head.extend_from_slice(&(n as u64).to_be_bytes());
        }
    }
    w.write_all(&head)?;
    w.write_all(payload)?;
    w.flush()
}

/// Une trame WebSocket lue.
struct WsFrame {
    fin: bool,
    op: u8,
    payload: Vec<u8>,
}

/// Lire une trame. `masked` : ce que doit être une trame de ce côté (une
/// page masque toujours, un serveur jamais).
fn read_frame(r: &mut impl Read, masked: bool, max: usize) -> std::io::Result<WsFrame> {
    let bad = || std::io::Error::from(std::io::ErrorKind::InvalidData);
    let mut h = [0u8; 2];
    r.read_exact(&mut h)?;
    let (fin, op) = (h[0] & 0x80 != 0, h[0] & 0x0f);
    if h[0] & 0x70 != 0 || (h[1] & 0x80 != 0) != masked {
        return Err(bad());
    }
    let len = match h[1] & 0x7f {
        126 => {
            let mut b = [0u8; 2];
            r.read_exact(&mut b)?;
            u16::from_be_bytes(b) as u64
        }
        127 => {
            let mut b = [0u8; 8];
            r.read_exact(&mut b)?;
            u64::from_be_bytes(b)
        }
        n => n as u64,
    };
    if len > max as u64 || (op >= OP_CLOSE && (len > 125 || !fin)) {
        return Err(bad());
    }
    let mut mask = [0u8; 4];
    if masked {
        r.read_exact(&mut mask)?;
    }
    let mut payload = vec![0u8; len as usize];
    r.read_exact(&mut payload)?;
    if masked {
        for (i, b) in payload.iter_mut().enumerate() {
            *b ^= mask[i % 4];
        }
    }
    Ok(WsFrame { fin, op, payload })
}

/// Un flux qui ne lit que jusqu'à une heure donnée : une page ou un poste
/// qui distille un octet de temps en temps ne tient pas la passerelle
/// au-delà de la conversation.
struct Until<'a> {
    stream: &'a TcpStream,
    until: Instant,
}

impl Read for Until<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let left = self.until.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(std::io::Error::from(std::io::ErrorKind::TimedOut));
        }
        self.stream.set_read_timeout(Some(left))?;
        (&mut &*self.stream).read(buf)
    }
}

impl Write for Until<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        (&mut &*self.stream).write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        (&mut &*self.stream).flush()
    }
}

/// Passer les trames de la page au poste et du poste à la page, jusqu'à
/// ce que l'un ferme ou que les bornes soient dépassées.
fn bridge(page: TcpStream, post: TcpStream) {
    let until = Instant::now() + TALK_BOUND;
    for s in [&page, &post] {
        let _ = s.set_write_timeout(Some(HEAD_WAIT));
    }
    let (Ok(page_w), Ok(post_r)) = (page.try_clone(), post.try_clone()) else {
        return;
    };
    // Both directions write to the page (a pong from one, the post's
    // frames from the other): one at a time.
    let page_w = Arc::new(Mutex::new(page_w));
    let back = {
        let page_w = Arc::clone(&page_w);
        std::thread::spawn(move || {
            let mut moved = 0usize;
            let mut from = Until {
                stream: &post_r,
                until,
            };
            while let Ok(frame) = from.recv() {
                moved += frame.len();
                let sent = page_w
                    .lock()
                    .map(|mut w| write_frame(&mut *w, OP_BINARY, &frame).is_ok())
                    .unwrap_or(false);
                if moved > TALK_BYTES || !sent {
                    break;
                }
            }
            if let Ok(mut w) = page_w.lock() {
                let _ = write_frame(&mut *w, OP_CLOSE, &1000u16.to_be_bytes());
            }
            let _ = post_r.shutdown(std::net::Shutdown::Both);
        })
    };
    let mut moved = 0usize;
    let mut from = Until {
        stream: &page,
        until,
    };
    let mut to = Until {
        stream: &post,
        until,
    };
    let mut message: Vec<u8> = Vec::new();
    let mut started = false;
    while let Ok(f) = read_frame(&mut from, true, patientchat::FRAME_MAX) {
        match f.op {
            OP_PING => {
                if let Ok(mut w) = page_w.lock() {
                    let _ = write_frame(&mut *w, OP_PONG, &f.payload);
                }
                continue;
            }
            OP_PONG => continue,
            OP_BINARY if !started => {
                started = true;
                moved += f.payload.len();
                message = f.payload;
            }
            OP_CONTINUE if started => {
                moved += f.payload.len();
                message.extend_from_slice(&f.payload);
            }
            // A close, a text frame, or a continuation of nothing.
            _ => break,
        }
        if message.len() > patientchat::FRAME_MAX || moved > TALK_BYTES {
            break;
        }
        if f.fin {
            started = false;
            if to.send(&std::mem::take(&mut message)).is_err() {
                break;
            }
        }
    }
    let _ = post.shutdown(std::net::Shutdown::Both);
    let _ = page.shutdown(std::net::Shutdown::Both);
    let _ = back.join();
}

fn connect(address: &str, patience: Duration) -> std::io::Result<TcpStream> {
    let mut last = Err(std::io::Error::from(std::io::ErrorKind::NotFound));
    for a in address.to_socket_addrs()? {
        match TcpStream::connect_timeout(&a, patience) {
            Ok(s) => return Ok(s),
            Err(e) => last = Err(e),
        }
    }
    last
}

/// **Attendre à la passerelle** qu'une page demande l'officine. Rend la
/// connexion mise en relation — la conversation commence, ce poste y
/// répond (`patientchat::answer`) — ou une erreur quand l'attente a fini
/// sans personne (le poste rappelle).
pub fn wait(address: &str, secret: &str, patience: Duration) -> std::io::Result<TcpStream> {
    wait_with(address, secret, patience, |_| {})
}

/// [`wait`], qui montre la connexion à `held` dès qu'elle est ouverte :
/// celui qui attend peut la couper pour s'arrêter sans attendre la fin
/// de l'attente.
pub fn wait_with(
    address: &str,
    secret: &str,
    patience: Duration,
    held: impl FnOnce(&TcpStream),
) -> std::io::Result<TcpStream> {
    let mut s = connect(address, patience)?;
    held(&s);
    s.write_all(preface(secret).as_bytes())?;
    s.set_read_timeout(Some(WAIT_RENEW))?;
    let mut line = [0u8; 6];
    s.read_exact(&mut line)?;
    if &line[..] != format!("{ENTER}\n").as_bytes() {
        return Err(std::io::Error::from(std::io::ErrorKind::InvalidData));
    }
    s.set_read_timeout(Some(TALK_BOUND))?;
    s.set_write_timeout(Some(TALK_BOUND))?;
    Ok(s)
}

/// L'adresse où un poste appelle une passerelle : son nom d'hôte, au port
/// des postes.
pub fn posts_address(gateway: &str) -> String {
    let host = patientchat::gateway_host(gateway);
    // A port written after the name is the web one (behind the proxy):
    // the posts' port is its own.
    let name = match host.rsplit_once(':') {
        Some((n, p)) if p.parse::<u16>().is_ok() && (n.ends_with(']') || !n.contains(':')) => {
            n.to_owned()
        }
        _ => host,
    };
    format!("{name}:{DEFAULT_POSTS_PORT}")
}

/// Une page ouverte sur la boîte `id` d'une passerelle, pour les tests du
/// poste qui attend.
#[cfg(test)]
pub(crate) fn tests_page(web: SocketAddr, id: &str) -> tests::Page {
    let (head, s) = tests::open_ws(web, id, &format!("http://{web}"));
    assert!(head.starts_with("HTTP/1.1 101"), "{head}");
    tests::Page(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::patientchat::memory::Memory;
    use crate::patientchat::{talk_as_patient, KeyRow, Request, ToPatient};

    #[test]
    fn the_handshake_answers_the_rfc_example() {
        assert_eq!(
            accept_key("dGhlIHNhbXBsZSBub25jZQ=="),
            "s3pPLMBiTxaQ9kYGzzhZRbK+xOo="
        );
    }

    #[test]
    fn a_preface_reads_back_and_nothing_else_does() {
        let secret = "ab".repeat(32);
        assert_eq!(read_preface(&preface(&secret)), Some(secret.clone()));
        for bad in [
            String::new(),
            PREFACE.to_owned(),
            format!("{PREFACE} attente"),
            format!("{PREFACE} attente {}", "ab".repeat(16)),
            format!("{PREFACE} appel {secret}"),
            format!("{PREFACE} attente {secret} de trop"),
            format!("{PREFACE} attente {}", "zz".repeat(32)),
        ] {
            assert!(read_preface(&bad).is_none(), "{bad}");
        }
    }

    #[test]
    fn where_a_post_calls_a_gateway() {
        assert_eq!(posts_address("chat.example.fr"), "chat.example.fr:7747");
        assert_eq!(
            posts_address("https://chat.example.fr:8443/"),
            "chat.example.fr:7747"
        );
        assert_eq!(posts_address("[2001:db8::1]"), "[2001:db8::1]:7747");
        assert_eq!(posts_address("[2001:db8::1]:8443"), "[2001:db8::1]:7747");
    }

    /// A page as a browser runs it: a WebSocket client, masked frames.
    pub(crate) struct Page(pub(crate) TcpStream);

    impl Frames for Page {
        fn send(&mut self, frame: &[u8]) -> std::io::Result<()> {
            let mask = [1u8, 2, 3, 4];
            let mut head = vec![0x80 | OP_BINARY];
            match frame.len() {
                n if n < 126 => head.push(0x80 | n as u8),
                n => {
                    head.push(0x80 | 126);
                    head.extend_from_slice(&(n as u16).to_be_bytes());
                }
            }
            head.extend_from_slice(&mask);
            head.extend(frame.iter().enumerate().map(|(i, b)| b ^ mask[i % 4]));
            self.0.write_all(&head)
        }
        fn recv(&mut self) -> std::io::Result<Vec<u8>> {
            let f = read_frame(&mut self.0, false, patientchat::FRAME_MAX)?;
            match f.op {
                OP_BINARY => Ok(f.payload),
                _ => Err(std::io::Error::other(format!(
                    "fermée {:?}",
                    f.payload.get(..2).map(|c| u16::from_be_bytes([c[0], c[1]]))
                ))),
            }
        }
    }

    fn gateway() -> (SocketAddr, String, Arc<AtomicBool>) {
        gateway_with(None)
    }

    /// A gateway on free ports, which takes pages from the published
    /// site and from itself (the tests' pages say so).
    fn gateway_with(journal: Option<std::path::PathBuf>) -> (SocketAddr, String, Arc<AtomicBool>) {
        let free = || {
            TcpListener::bind("127.0.0.1:0")
                .unwrap()
                .local_addr()
                .unwrap()
        };
        let (web, posts) = (free(), free());
        let stop = Arc::new(AtomicBool::new(false));
        let mut options = Options::new(web, posts);
        options.origins.push(format!("http://{web}"));
        options.journal = journal;
        serve(options, Arc::clone(&stop)).unwrap();
        (web, posts.to_string(), stop)
    }

    pub(super) fn get(web: SocketAddr, path: &str, extra: &str) -> (String, TcpStream) {
        let mut s = TcpStream::connect(web).unwrap();
        s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        write!(s, "GET {path} HTTP/1.1\r\nHost: {web}\r\n{extra}\r\n").unwrap();
        let mut head = Vec::new();
        let mut b = [0u8; 1];
        while !head.ends_with(b"\r\n\r\n") {
            s.read_exact(&mut b).unwrap();
            head.push(b[0]);
        }
        (String::from_utf8(head).unwrap(), s)
    }

    pub(super) fn open_ws(web: SocketAddr, id: &str, origin: &str) -> (String, TcpStream) {
        get(
            web,
            &format!("/ws/{id}"),
            &format!(
                "Upgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Version: 13\r\n\
                 Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nOrigin: {origin}\r\n"
            ),
        )
    }

    /// A page reaches the post waiting at its officine's box, and the
    /// conversation of `patientchat` goes through untouched.
    #[test]
    fn a_page_talks_to_its_officine_through_the_gateway() {
        let (web, posts, stop) = gateway();
        let secret = "5a".repeat(32);
        let id = patientchat::box_id(&unhex(&secret).unwrap());
        let key = [8u8; 32];
        let post = {
            let (posts, secret) = (posts.clone(), secret.clone());
            std::thread::spawn(move || {
                let mut s = wait(&posts, &secret, Duration::from_secs(5)).unwrap();
                let mut store = Memory {
                    keys: vec![KeyRow {
                        id: 4,
                        key,
                        open: true,
                    }],
                    replies: vec![(
                        4,
                        ToPatient {
                            id: 1,
                            at: "14/10/2026 09:00".into(),
                            text: "Bonjour".into(),
                        },
                    )],
                    ..Default::default()
                };
                let o = patientchat::answer(&mut s, &mut store, |n| n.fill(1)).unwrap();
                (o, store.received.len())
            })
        };
        std::thread::sleep(Duration::from_millis(300));
        let (head, s) = open_ws(web, &id, &format!("http://{web}"));
        assert!(head.starts_with("HTTP/1.1 101"), "{head}");
        assert!(head.contains("s3pPLMBiTxaQ9kYGzzhZRbK+xOo="));
        let mut page = Page(s);
        let r = talk_as_patient(
            &mut page,
            &key,
            [2; 16],
            &Request {
                out: vec![patientchat::FromPatient {
                    id: "x1".into(),
                    at: String::new(),
                    text: "Ma commande ?".into(),
                }],
                seen: vec![],
            },
        )
        .unwrap()
        .unwrap();
        assert_eq!(r.got, vec!["x1".to_string()]);
        assert_eq!(r.messages[0].text, "Bonjour");
        let (o, received) = post.join().unwrap();
        assert_eq!(
            o,
            patientchat::Outcome::Talked {
                key: 4,
                received: 1
            }
        );
        assert_eq!(received, 1);
        stop.store(true, Ordering::SeqCst);
    }

    /// No post waiting: the page hears it at once, with its own code.
    #[test]
    fn a_page_with_no_post_waiting_is_told_so() {
        let (web, _, stop) = gateway();
        let id = "0".repeat(32);
        let (head, s) = open_ws(web, &id, &format!("http://{web}"));
        assert!(head.starts_with("HTTP/1.1 101"));
        let mut page = Page(s);
        page.send(b"BPMP1 bonjour").unwrap();
        let err = page.recv().unwrap_err().to_string();
        assert!(err.contains(&CLOSE_ABSENT.to_string()), "{err}");
        stop.store(true, Ordering::SeqCst);
    }

    /// A page that opens and says nothing takes no post: it is let go,
    /// and the post still waits for the next one.
    #[test]
    fn a_silent_page_holds_no_post() {
        let (web, posts, stop) = gateway();
        let secret = "5b".repeat(32);
        let id = patientchat::box_id(&unhex(&secret).unwrap());
        let waiter = {
            let posts = posts.clone();
            std::thread::spawn(move || wait(&posts, &secret, Duration::from_secs(5)))
        };
        std::thread::sleep(Duration::from_millis(300));
        let (_, mut s) = open_ws(web, &id, &format!("http://{web}"));
        s.set_read_timeout(Some(HEAD_WAIT + Duration::from_secs(3)))
            .unwrap();
        let mut b = [0u8; 1];
        assert_eq!(s.read(&mut b).unwrap_or(0), 0, "let go");
        assert!(!waiter.is_finished(), "the post was not taken");
        stop.store(true, Ordering::SeqCst);
    }

    /// A post that hung up is seen gone: its place in the box is free
    /// for the next wait, and no page is put in touch with it.
    #[test]
    fn a_wait_that_hung_up_frees_its_place() {
        let (web, posts, stop) = gateway();
        let secret = "5c".repeat(32);
        let id = patientchat::box_id(&unhex(&secret).unwrap());
        for _ in 0..WAITING_PER_BOX {
            let mut s = TcpStream::connect(&posts).unwrap();
            s.write_all(preface(&secret).as_bytes()).unwrap();
        } // all dropped: hung up
        std::thread::sleep(Duration::from_millis(300));
        let waiter = {
            let (posts, secret) = (posts.clone(), secret.clone());
            std::thread::spawn(move || wait(&posts, &secret, Duration::from_secs(5)))
        };
        std::thread::sleep(Duration::from_millis(300));
        let (_, s) = open_ws(web, &id, &format!("http://{web}"));
        let mut page = Page(s);
        page.send(b"BPMP1 bonjour").unwrap();
        let mut post = waiter
            .join()
            .unwrap()
            .expect("the live wait is the one taken");
        assert_eq!(post.recv().unwrap(), b"BPMP1 bonjour");
        stop.store(true, Ordering::SeqCst);
    }

    /// The gateway serves no page — only says what it is; the published
    /// page's origin gets a WebSocket, another site's page does not; a
    /// wrong box or path gets nothing.
    #[test]
    fn no_page_here_and_what_is_refused() {
        let (web, _, stop) = gateway();
        let (head, mut s) = get(web, "/", "");
        assert!(head.starts_with("HTTP/1.1 200"), "{head}");
        assert!(head.contains("Content-Security-Policy: default-src 'none'"));
        assert!(head.contains("Referrer-Policy: no-referrer"));
        let mut body = String::new();
        let _ = s.read_to_string(&mut body);
        assert!(body.contains("publiée"), "{body}");
        for path in ["/index.html", "/app.js", "/style.css"] {
            let (head, _) = get(web, path, "");
            assert!(head.starts_with("HTTP/1.1 404"), "{path}: {head}");
        }
        let (head, _) = open_ws(web, &"0".repeat(32), "https://youssefsahli.github.io");
        assert!(head.starts_with("HTTP/1.1 101"), "{head}");
        let (head, _) = open_ws(web, &"0".repeat(32), "https://ailleurs.example");
        assert!(head.starts_with("HTTP/1.1 403"), "{head}");
        let (head, _) = open_ws(
            web,
            &"0".repeat(32),
            "https://youssefsahli.github.io.ailleurs.example",
        );
        assert!(head.starts_with("HTTP/1.1 403"), "{head}");
        let (head, _) = get(web, "/ws/court", "");
        assert!(head.starts_with("HTTP/1.1 404"), "{head}");
        let (head, _) = get(web, "/etc/passwd", "");
        assert!(head.starts_with("HTTP/1.1 404"), "{head}");
        stop.store(true, Ordering::SeqCst);
    }

    /// The journal: a file a day, a line a connection, never a frame;
    /// days older than what is kept go when a day begins; a flood stops
    /// at a line count; the keeping is held to six months.
    #[test]
    fn the_journal_keeps_connections_and_forgets_on_time() {
        let dir = std::env::temp_dir().join(format!(
            "bpm-passerelle-journal-days-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let j = Journal::open(&dir, 2).unwrap();
        let day = |iso: &str| crate::date::to_days(iso).unwrap() as u64 * 86_400;
        j.note_at(
            day("2026-10-01") + 3_725,
            "page 203.0.113.9 abc aucun poste",
        );
        j.note_at(
            day("2026-10-02") + 60,
            "poste 198.51.100.2 abc attend\ninjecté",
        );
        let first = std::fs::read_to_string(dir.join("2026-10-01.log")).unwrap();
        assert_eq!(
            first,
            "2026-10-01T01:02:05Z page 203.0.113.9 abc aucun poste\n"
        );
        let second = std::fs::read_to_string(dir.join("2026-10-02.log")).unwrap();
        assert_eq!(second.lines().count(), 1, "one line, whatever it holds");
        j.note_at(day("2026-10-03"), "page");
        assert!(!dir.join("2026-10-01.log").exists(), "two days kept");
        assert!(dir.join("2026-10-02.log").exists());
        std::fs::write(dir.join("notes.txt"), "à l'exploitant").unwrap();
        j.note_at(day("2026-12-01"), "page");
        assert!(dir.join("notes.txt").exists(), "only its own files");
        assert_eq!(Journal::open(&dir, 10_000).unwrap().keep, KEEP_MOST);
        let _ = std::fs::remove_dir_all(&dir);

        // A gateway with a journal writes who called and what came of it.
        let dir = std::env::temp_dir().join(format!(
            "bpm-passerelle-journal-gateway-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let (web, _, stop) = gateway_with(Some(dir.clone()));
        let id = "1".repeat(32);
        let (_, s) = open_ws(web, &id, &format!("http://{web}"));
        let mut page = Page(s);
        page.send(b"BPMP1 secret de la page").unwrap();
        let _ = page.recv();
        std::thread::sleep(Duration::from_millis(300));
        let text: String = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|e| std::fs::read_to_string(e.path()).unwrap())
            .collect();
        assert!(
            text.contains(&format!("page 127.0.0.1 {id} aucun poste")),
            "{text}"
        );
        assert!(!text.contains("secret de la page"), "never a frame");
        stop.store(true, Ordering::SeqCst);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Behind the proxy, the patient's own address is the one counted.
    #[test]
    fn the_proxy_says_who_calls() {
        let h =
            parse_head("GET / HTTP/1.1\r\nX-Forwarded-For: 10.0.0.1, 203.0.113.9\r\n\r\n").unwrap();
        let local: IpAddr = "127.0.0.1".parse().unwrap();
        assert_eq!(
            client_ip(local, &h),
            "203.0.113.9".parse::<IpAddr>().unwrap()
        );
        let far: IpAddr = "198.51.100.2".parse().unwrap();
        assert_eq!(client_ip(far, &h), far, "only the proxy is believed");
    }
}

#[cfg(test)]
mod browser {
    use super::*;
    use crate::patientchat::memory::Memory;
    use crate::patientchat::{KeyRow, Store, ToPatient};

    struct Logged(Memory);
    impl Store for Logged {
        fn keys(&mut self) -> Result<Vec<crate::patientchat::KeyRow>, String> {
            self.0.keys()
        }
        fn receive(
            &mut self,
            key: i64,
            m: &[crate::patientchat::FromPatient],
        ) -> Result<Vec<String>, String> {
            eprintln!("RECEIVE {key} {m:?}");
            self.0.receive(key, m)
        }
        fn replies(&mut self, key: i64, seen: &[i64]) -> Result<Vec<ToPatient>, String> {
            eprintln!("REPLIES {key} seen={seen:?}");
            self.0.replies(key, seen)
        }
        fn officine(&mut self) -> crate::patientchat::Officine {
            self.0.officine()
        }
    }

    /// The page as GitHub Pages serves it, on 18745: what a browser
    /// opens, apart from the gateway on 18746.
    fn serve_the_page() {
        let l = TcpListener::bind("127.0.0.1:18745").unwrap();
        std::thread::spawn(move || {
            for s in l.incoming().flatten() {
                // One thread each: a browser opens connections it does
                // not use at once.
                std::thread::spawn(move || {
                    let Ok(head) = read_head(&s) else { return };
                    let name = head.path.trim_start_matches('/');
                    let name = if name.is_empty() { "index.html" } else { name };
                    let kind = match name.rsplit('.').next() {
                        Some("js") => "text/javascript; charset=utf-8",
                        Some("css") => "text/css; charset=utf-8",
                        _ => "text/html; charset=utf-8",
                    };
                    // As GitHub Pages: no policy header, the page's own
                    // `<meta>` one holds.
                    let (status, body) =
                        match patientchat::PAGE_FILES.iter().find(|(n, _)| *n == name) {
                            Some((_, body)) => ("200 OK", body.as_bytes()),
                            None => ("404 Not Found", b"".as_slice()),
                        };
                    let mut s = s;
                    let _ = write!(
                        s,
                        "HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\n\
                         Connection: close\r\n\r\n",
                        body.len()
                    );
                    let _ = s.write_all(body);
                });
            }
        });
    }

    #[test]
    #[ignore]
    fn serve_for_a_browser() {
        let stop = Arc::new(AtomicBool::new(false));
        let mut options = Options::new(
            "127.0.0.1:18746".parse().unwrap(),
            "127.0.0.1:17747".parse().unwrap(),
        );
        options.origins.push("http://127.0.0.1:18745".into());
        serve(options, Arc::clone(&stop)).unwrap();
        serve_the_page();
        let secret = "5a".repeat(32);
        let id = patientchat::box_id(&unhex(&secret).unwrap());
        let key = [8u8; 32];
        eprintln!(
            "URL http://127.0.0.1:18745/#b={id}&k={}&p=127.0.0.1:18746",
            patientchat::key_text(&key)
        );
        let mut store = Logged(Memory {
            keys: vec![KeyRow {
                id: 4,
                key,
                open: std::env::var_os("BPM_TEST_CLOSED").is_none(),
            }],
            replies: vec![(
                4,
                ToPatient {
                    id: 7,
                    at: "14/10/2026 09:00".into(),
                    text: "Bonjour <b>Mme</b>".into(),
                },
            )],
            ..Default::default()
        });
        let until = Instant::now() + Duration::from_secs(80);
        while Instant::now() < until {
            if let Ok(mut s) = wait("127.0.0.1:17747", &secret, Duration::from_secs(5)) {
                let o = patientchat::answer(&mut s, &mut store, |n| n.fill(1));
                eprintln!("OUTCOME {o:?}");
            }
        }
    }
}
