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
//! * elle **sert la page** (`assets/patient/`), en clair derrière le proxy
//!   — du code, aucune donnée ;
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
//! Rien n'est écrit sur son disque. Aucun poste n'attend : la page
//! l'apprend (fermeture 4004) et garde le message sur le téléphone. Ce
//! qu'elle sait : qu'une boîte échange, quand, combien, et depuis quelle
//! adresse.
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
/// Combien de connexions une adresse ouvre par minute.
const CALLS_PER_MINUTE: usize = 30;

const PAGE: &str = include_str!("../assets/patient/index.html");
const APP: &str = include_str!("../assets/patient/app.js");
const STYLE: &str = include_str!("../assets/patient/style.css");

/// La politique de la page : son propre code, ses propres styles, un
/// WebSocket vers elle-même, rien d'autre — ni script d'ailleurs, ni
/// cadre, ni formulaire.
const CSP: &str = "default-src 'none'; script-src 'self'; style-src 'self'; \
                   connect-src 'self'; img-src 'self' data:; base-uri 'none'; \
                   form-action 'none'; frame-ancestors 'none'";

struct Waiting {
    stream: TcpStream,
    since: Instant,
}

type Boxes = Arc<Mutex<HashMap<String, Vec<Waiting>>>>;
type Calls = Arc<Mutex<HashMap<IpAddr, Vec<Instant>>>>;

/// **Tenir la passerelle ouverte** jusqu'à ce que `stop` passe à vrai :
/// la page et les WebSocket sur `web`, les postes sur `posts`. Rend tout
/// de suite ; elle vit sur ses propres fils.
pub fn serve(web: SocketAddr, posts: SocketAddr, stop: Arc<AtomicBool>) -> std::io::Result<()> {
    let web = TcpListener::bind(web)?;
    let posts = TcpListener::bind(posts)?;
    web.set_nonblocking(true)?;
    posts.set_nonblocking(true)?;
    let boxes: Boxes = Arc::default();
    let calls: Calls = Arc::default();
    let open = Arc::new(AtomicUsize::new(0));
    for (listener, is_web) in [(web, true), (posts, false)] {
        let (boxes, calls, open, stop) = (
            Arc::clone(&boxes),
            Arc::clone(&calls),
            Arc::clone(&open),
            Arc::clone(&stop),
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
                        continue;
                    }
                }
                open.fetch_add(1, Ordering::SeqCst);
                let (boxes, calls, open) =
                    (Arc::clone(&boxes), Arc::clone(&calls), Arc::clone(&open));
                std::thread::spawn(move || {
                    let _ = stream.set_nonblocking(false);
                    let _ = if is_web {
                        take_page(stream, from, &boxes, &calls)
                    } else {
                        take_post(stream, from.ip(), &boxes, &calls, &open)
                    };
                    open.fetch_sub(1, Ordering::SeqCst);
                });
            }
        });
    }
    Ok(())
}

/// Lâcher les attentes échues, et les décompter.
fn prune(b: &mut HashMap<String, Vec<Waiting>>, open: &AtomicUsize) {
    b.retain(|_, v| {
        let before = v.len();
        v.retain(|x| x.since.elapsed() < WAIT_BOUND);
        open.fetch_sub(before - v.len(), Ordering::SeqCst);
        !v.is_empty()
    });
}

/// Une adresse qui appelle trop souvent est lâchée.
fn admit(calls: &Mutex<HashMap<IpAddr, Vec<Instant>>>, ip: IpAddr) -> bool {
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
    seen.len() <= CALLS_PER_MINUTE
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
) -> std::io::Result<()> {
    if !admit(calls, ip) {
        return Ok(());
    }
    let line = read_line(&stream)?;
    let Some(secret) = read_preface(&line) else {
        return Ok(());
    };
    let Some(bytes) = unhex(&secret) else {
        return Ok(());
    };
    let id = patientchat::box_id(&bytes);
    let Ok(mut b) = boxes.lock() else {
        return Ok(());
    };
    prune(&mut b, open);
    if !b.contains_key(&id) && b.len() >= MOST_BOXES {
        return Ok(());
    }
    let list = b.entry(id).or_default();
    if list.len() >= WAITING_PER_BOX {
        return Ok(());
    }
    // Held past this thread: counted again until it is put in touch or
    // let go.
    open.fetch_add(1, Ordering::SeqCst);
    list.push(Waiting {
        stream,
        since: Instant::now(),
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

/// Une page qui appelle : la page elle-même, ou un WebSocket vers une
/// boîte.
fn take_page(
    stream: TcpStream,
    from: SocketAddr,
    boxes: &Boxes,
    calls: &Calls,
) -> std::io::Result<()> {
    let head = read_head(&stream)?;
    if !admit(calls, client_ip(from.ip(), &head)) {
        return respond(&stream, "429 Too Many Requests", "text/plain", b"");
    }
    if head.method != "GET" {
        return respond(&stream, "405 Method Not Allowed", "text/plain", b"");
    }
    let page = |kind: &str, body: &str| respond(&stream, "200 OK", kind, body.as_bytes());
    match head.path.as_str() {
        "/" | "/index.html" => page("text/html; charset=utf-8", PAGE),
        "/app.js" => page("text/javascript; charset=utf-8", APP),
        "/style.css" => page("text/css; charset=utf-8", STYLE),
        p => match p.strip_prefix("/ws/") {
            Some(id) if patientchat::is_box_id(id) => {
                upgrade(stream, &head, &id.to_ascii_lowercase(), boxes)
            }
            _ => respond(&stream, "404 Not Found", "text/plain", b""),
        },
    }
}

fn upgrade(mut stream: TcpStream, head: &Head, id: &str, boxes: &Boxes) -> std::io::Result<()> {
    let key = head.header("sec-websocket-key");
    if !head
        .header("upgrade")
        .to_ascii_lowercase()
        .contains("websocket")
        || head.header("sec-websocket-version") != "13"
        || key.is_empty()
    {
        return respond(&stream, "400 Bad Request", "text/plain", b"");
    }
    // A page of another site does not reach the officine through here.
    let origin = head.header("origin");
    if !origin.is_empty() && host_of(origin) != host_of(head.header("host")) {
        return respond(&stream, "403 Forbidden", "text/plain", b"");
    }
    stream.write_all(
        format!(
            "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\n\
             Connection: Upgrade\r\nSec-WebSocket-Accept: {}\r\n\r\n",
            accept_key(key)
        )
        .as_bytes(),
    )?;
    // The post that has waited longest; one gone away is let go and the
    // next tried.
    let post = loop {
        let next = boxes.lock().ok().and_then(|mut b| {
            b.get_mut(id)
                .and_then(|v| (!v.is_empty()).then(|| v.remove(0)))
        });
        let Some(mut w) = next else {
            break None;
        };
        if w.since.elapsed() < WAIT_BOUND
            && w.stream.write_all(format!("{ENTER}\n").as_bytes()).is_ok()
        {
            break Some(w.stream);
        }
    };
    let Some(post) = post else {
        let mut reason = CLOSE_ABSENT.to_be_bytes().to_vec();
        reason.extend_from_slice(b"absente");
        return write_frame(&mut stream, OP_CLOSE, &reason);
    };
    bridge(stream, post);
    Ok(())
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
        let free = || {
            TcpListener::bind("127.0.0.1:0")
                .unwrap()
                .local_addr()
                .unwrap()
        };
        let (web, posts) = (free(), free());
        let stop = Arc::new(AtomicBool::new(false));
        serve(web, posts, Arc::clone(&stop)).unwrap();
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
        let err = Page(s).recv().unwrap_err().to_string();
        assert!(err.contains(&CLOSE_ABSENT.to_string()), "{err}");
        stop.store(true, Ordering::SeqCst);
    }

    /// The page is served with its policy; another site's page does not
    /// get a WebSocket; a wrong box or path gets nothing.
    #[test]
    fn the_page_its_policy_and_what_is_refused() {
        let (web, _, stop) = gateway();
        let (head, mut s) = get(web, "/", "");
        assert!(head.starts_with("HTTP/1.1 200"), "{head}");
        assert!(head.contains("script-src 'self'"));
        assert!(head.contains("Referrer-Policy: no-referrer"));
        let mut body = String::new();
        let _ = s.read_to_string(&mut body);
        assert!(body.contains("app.js"));
        let (head, _) = open_ws(web, &"0".repeat(32), "https://ailleurs.example");
        assert!(head.starts_with("HTTP/1.1 403"), "{head}");
        let (head, _) = get(web, "/ws/court", "");
        assert!(head.starts_with("HTTP/1.1 404"), "{head}");
        let (head, _) = get(web, "/etc/passwd", "");
        assert!(head.starts_with("HTTP/1.1 404"), "{head}");
        stop.store(true, Ordering::SeqCst);
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
        fn officine(&mut self) -> String {
            self.0.officine()
        }
    }

    #[test]
    #[ignore]
    fn serve_for_a_browser() {
        let stop = Arc::new(AtomicBool::new(false));
        serve(
            "127.0.0.1:18746".parse().unwrap(),
            "127.0.0.1:17747".parse().unwrap(),
            Arc::clone(&stop),
        )
        .unwrap();
        let secret = "5a".repeat(32);
        let id = patientchat::box_id(&unhex(&secret).unwrap());
        let key = [8u8; 32];
        eprintln!(
            "URL http://127.0.0.1:18746/#b={id}&k={}",
            patientchat::key_text(&key)
        );
        let mut store = Logged(Memory {
            keys: vec![KeyRow {
                id: 4,
                key,
                open: true,
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
