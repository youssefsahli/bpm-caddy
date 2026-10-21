//! **Le relais sans mémoire** : une officine du réseau qui accepte de
//! mettre en relation un poste qu'on ne peut pas joindre et un téléphone
//! de son équipe — et qui ne garde rien.
//!
//! Le poste injoignable (derrière une traduction d'adresse d'opérateur,
//! sans IPv6) ne peut qu'**appeler** ; le téléphone aussi. Tous deux
//! appellent donc l'officine qui relaie, en disant le nom du groupe de
//! postes : le poste appelle et *attend*, le téléphone appelle et
//! *demande*. Le relais passe alors les octets de l'un à l'autre, dans
//! les deux sens, sans les lire : ce qui traverse est la conversation
//! chiffrée de bout en bout du § 5 de `docs/SYNC.md` — la poignée de main
//! prouve de part et d'autre qu'on parle au bon poste, et le relais n'a
//! aucune clé. Rien n'est écrit sur son disque : quand la conversation
//! finit, il n'en reste rien.
//!
//! Ce qu'il sait : qu'un groupe de postes échange, quand et combien.
//! Ce qu'il accepte : les seuls groupes que des officines de son réseau
//! lui ont demandé de relayer (`allowed`), des conversations bornées en
//! durée et en octets, et un nombre d'appels par adresse.
//!
//! Le protocole tient en une ligne de préface (`BPMRELAIS1 attente|appel
//! <groupe>`) et une réponse (`ENTRE`) au poste qui attendait, au moment
//! où on le met en relation.

use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream, ToSocketAddrs, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// La préface d'une connexion au relais.
pub const PREFACE: &str = "BPMRELAIS1";
/// Ce que le relais écrit au poste qui attendait : quelqu'un est là.
pub const ENTER: &str = "ENTRE";
/// Le port proposé pour relayer, quand l'officine n'en a pas écrit — à
/// côté de ceux des postes (7743), des invitations d'officines (7742) et
/// de la porte des officines (7745).
pub const DEFAULT_PORT: u16 = 7746;

/// Combien de postes d'un même groupe peuvent attendre à la fois — au-delà,
/// une nouvelle attente est refusée, jamais mise à la place d'une autre :
/// qui connaîtrait le nom du groupe ne chasse pas le poste qui attend.
const WAITING_PER_GROUP: usize = 4;
/// Combien de groupes à la fois.
const MOST_GROUPS: usize = 64;
/// Une attente dure au plus ceci chez le relais…
const WAIT_BOUND: Duration = Duration::from_secs(5 * 60);
/// …et le poste qui attend rappelle un peu avant : une connexion
/// silencieuse plus longtemps, une traduction d'adresse d'opérateur
/// l'oublie, et le téléphone serait mis en relation avec un mort.
pub const WAIT_RENEW: Duration = Duration::from_secs(4 * 60);
/// Combien de connexions le relais tient à la fois, en tout — attentes,
/// préfaces en cours, conversations (deux fils chacune).
const MOST_CONNECTIONS: usize = 256;
/// Une conversation relayée dure au plus…
const TALK_BOUND: Duration = Duration::from_secs(10 * 60);
/// …et porte au plus, dans chaque sens :
const TALK_BYTES: u64 = 256 * 1024 * 1024;
/// Le silence au-delà duquel une conversation relayée est lâchée.
const IDLE: Duration = Duration::from_secs(90);
/// Le temps de dire sa préface.
const PREFACE_WAIT: Duration = Duration::from_secs(5);
/// Combien d'appels une adresse peut passer par minute.
const CALLS_PER_MINUTE: usize = 30;

/// Qui appelle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// Un poste qu'on ne peut pas joindre : il attend qu'on le demande.
    Attente,
    /// Un téléphone (ou un poste) qui demande un poste du groupe.
    Appel,
}

/// La ligne de préface.
pub fn preface(role: Role, group: &str) -> String {
    let r = match role {
        Role::Attente => "attente",
        Role::Appel => "appel",
    };
    format!("{PREFACE} {r} {group}\n")
}

/// Relire une préface : le rôle et le groupe (vingt caractères
/// hexadécimaux, le nom du groupe de postes). `None` pour tout le reste.
pub fn read_preface(line: &str) -> Option<(Role, String)> {
    let mut parts = line.split_whitespace();
    if parts.next()? != PREFACE {
        return None;
    }
    let role = match parts.next()? {
        "attente" => Role::Attente,
        "appel" => Role::Appel,
        _ => return None,
    };
    let group = parts.next()?.to_ascii_lowercase();
    if parts.next().is_some() || group.len() != 20 || !group.chars().all(|c| c.is_ascii_hexdigit())
    {
        return None;
    }
    Some((role, group))
}

/// Les groupes que ce relais accepte, que le fil de l'officine tient à
/// jour depuis les demandes reçues du réseau.
pub type Allowed = Arc<Mutex<HashSet<String>>>;

struct Waiting {
    stream: TcpStream,
    since: Instant,
}

/// **Tenir le relais ouvert** sur `port` jusqu'à ce que `stop` passe à
/// vrai. Rend tout de suite ; le relais vit sur ses propres fils.
pub fn serve(port: u16, allowed: Allowed, stop: Arc<AtomicBool>) -> std::io::Result<()> {
    let listener = TcpListener::bind(("0.0.0.0", port))?;
    listener.set_nonblocking(true)?;
    let waiting: Arc<Mutex<HashMap<String, Vec<Waiting>>>> = Arc::default();
    let calls: Arc<Mutex<HashMap<IpAddr, Vec<Instant>>>> = Arc::default();
    let open = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    std::thread::spawn(move || {
        while !stop.load(Ordering::SeqCst) {
            let (stream, from) = match listener.accept() {
                Ok(s) => s,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(100));
                    continue;
                }
                // Out of descriptors, or anything else: breathe rather than
                // spin a core on a failing accept.
                Err(_) => {
                    std::thread::sleep(Duration::from_millis(500));
                    continue;
                }
            };
            if !admit(&calls, from.ip()) {
                continue;
            }
            // Plein : d'abord lâcher les attentes échues. Elles ne
            // s'élaguaient qu'à l'arrivée d'une nouvelle attente, que le
            // relais plein refusait — saturé, il le restait jusqu'au
            // redémarrage.
            if open.load(Ordering::SeqCst) >= MOST_CONNECTIONS {
                if let Ok(mut w) = waiting.lock() {
                    prune(&mut w, &open);
                }
                if open.load(Ordering::SeqCst) >= MOST_CONNECTIONS {
                    continue;
                }
            }
            open.fetch_add(1, Ordering::SeqCst);
            let (allowed, waiting, open) = (
                Arc::clone(&allowed),
                Arc::clone(&waiting),
                Arc::clone(&open),
            );
            std::thread::spawn(move || {
                let _ = take(stream, &allowed, &waiting, &open);
                open.fetch_sub(1, Ordering::SeqCst);
            });
        }
    });
    Ok(())
}

/// Lâcher les attentes échues, et les décompter.
fn prune(w: &mut HashMap<String, Vec<Waiting>>, open: &std::sync::atomic::AtomicUsize) {
    w.retain(|_, v| {
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
    // Bounded memory, without forgetting who has been calling: past a
    // thousand addresses in the minute, a new one is refused.
    if calls.len() >= 1024 && !calls.contains_key(&ip) {
        return false;
    }
    let seen = calls.entry(ip).or_default();
    seen.push(now);
    seen.len() <= CALLS_PER_MINUTE
}

/// Une connexion reçue : sa préface, puis attendre ou être mise en
/// relation.
fn take(
    stream: TcpStream,
    allowed: &Allowed,
    waiting: &Mutex<HashMap<String, Vec<Waiting>>>,
    open: &std::sync::atomic::AtomicUsize,
) -> std::io::Result<()> {
    stream.set_nonblocking(false)?;
    stream.set_read_timeout(Some(PREFACE_WAIT))?;
    let line = read_line(&stream)?;
    let Some((role, group)) = read_preface(&line) else {
        return Ok(());
    };
    if !allowed.lock().map(|a| a.contains(&group)).unwrap_or(false) {
        return Ok(());
    }
    match role {
        Role::Attente => {
            let Ok(mut w) = waiting.lock() else {
                return Ok(());
            };
            prune(&mut w, open);
            if !w.contains_key(&group) && w.len() >= MOST_GROUPS {
                return Ok(());
            }
            let list = w.entry(group).or_default();
            if list.len() >= WAITING_PER_GROUP {
                return Ok(());
            }
            // Held, not in a thread: the connection counts until it is
            // put in touch or dropped (`open` is released by the caller's
            // thread ending; the held one is counted again below).
            open.fetch_add(1, Ordering::SeqCst);
            list.push(Waiting {
                stream,
                since: Instant::now(),
            });
            Ok(())
        }
        Role::Appel => loop {
            // The one that has waited longest: first in, first out.
            let next = waiting.lock().ok().and_then(|mut w| {
                w.get_mut(&group)
                    .and_then(|v| (!v.is_empty()).then(|| v.remove(0)))
            });
            let Some(mut other) = next else {
                return Ok(());
            };
            // Out of the list: no longer counted as held.
            open.fetch_sub(1, Ordering::SeqCst);
            if other.since.elapsed() >= WAIT_BOUND {
                continue;
            }
            // A waiting post that went away fails here, and the next one
            // is tried.
            if other
                .stream
                .write_all(format!("{ENTER}\n").as_bytes())
                .is_err()
            {
                continue;
            }
            splice(stream, other.stream);
            return Ok(());
        },
    }
}

/// La ligne de préface, lue **octet par octet** : ce qui la suit est déjà
/// la conversation, et un tampon de lecture en aurait avalé le début.
fn read_line(mut stream: &TcpStream) -> std::io::Result<String> {
    // One deadline for the whole line, not one per byte: a byte every few
    // seconds does not hold a thread for ten minutes.
    let until = Instant::now() + PREFACE_WAIT;
    let mut out = Vec::with_capacity(64);
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

/// Passer les octets de l'un à l'autre, dans les deux sens, jusqu'à ce
/// que l'un ferme, que **les deux** se taisent, ou que les bornes soient
/// dépassées.
///
/// Le silence se compte pour la conversation, pas pour un sens : un
/// téléphone qui reçoit un long tour sur un lien faible n'envoie rien
/// pendant des minutes, et l'attente de quatre-vingt-dix secondes sur
/// son seul sens coupait le transfert au milieu.
fn splice(a: TcpStream, b: TcpStream) {
    let start = Instant::now();
    let until = start + TALK_BOUND;
    for s in [&a, &b] {
        let _ = s.set_read_timeout(Some(TICK));
        let _ = s.set_write_timeout(Some(IDLE));
    }
    let (Ok(a2), Ok(b2)) = (a.try_clone(), b.try_clone()) else {
        return;
    };
    let last = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
    let other = std::sync::Arc::clone(&last);
    let one = std::thread::spawn(move || pipe(a, b2, start, until, &other));
    pipe(b, a2, start, until, &last);
    let _ = one.join();
}

/// Le pas auquel chaque sens regarde si la conversation s'est tue.
const TICK: Duration = Duration::from_secs(5);

fn pipe(
    mut from: TcpStream,
    mut to: TcpStream,
    start: Instant,
    until: Instant,
    last: &std::sync::atomic::AtomicU64,
) {
    use std::sync::atomic::Ordering;
    let mut buf = [0u8; 16 * 1024];
    let mut moved: u64 = 0;
    let since_start = || u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
    loop {
        if Instant::now() >= until || moved >= TALK_BYTES {
            break;
        }
        match from.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                moved += n as u64;
                last.store(since_start(), Ordering::Relaxed);
                if to.write_all(&buf[..n]).is_err() {
                    break;
                }
            }
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                let quiet = since_start().saturating_sub(last.load(Ordering::Relaxed));
                if quiet >= u64::try_from(IDLE.as_millis()).unwrap_or(u64::MAX) {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    let _ = to.shutdown(std::net::Shutdown::Both);
    let _ = from.shutdown(std::net::Shutdown::Both);
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

/// **Attendre au relais** qu'un téléphone du groupe demande ce poste.
/// Rend la connexion mise en relation — la conversation commence, ce
/// poste y répond — ou une erreur quand l'attente a fini sans personne
/// (le poste rappelle).
pub fn wait(address: &str, group: &str, patience: Duration) -> std::io::Result<TcpStream> {
    let mut s = connect(address, patience)?;
    s.write_all(preface(Role::Attente, group).as_bytes())?;
    s.set_read_timeout(Some(WAIT_RENEW))?;
    let mut line = [0u8; 6];
    s.read_exact(&mut line)?;
    if &line[..] != format!("{ENTER}\n").as_bytes() {
        return Err(std::io::Error::from(std::io::ErrorKind::InvalidData));
    }
    s.set_read_timeout(None)?;
    Ok(s)
}

/// **Demander un poste du groupe au relais.** La connexion rendue mène à
/// un poste qui attendait — ou se ferme aussitôt s'il n'y en a aucun, et
/// la conversation échoue comme sur un poste éteint.
pub fn call(address: &str, group: &str, patience: Duration) -> std::io::Result<TcpStream> {
    let mut s = connect(address, patience)?;
    s.write_all(preface(Role::Appel, group).as_bytes())?;
    Ok(s)
}

/// **La voie directe des patients, relayée** — le même relais, en UDP,
/// sur le même numéro de port.
///
/// La page d'un patient ne parle que WebRTC (`direct.rs`) : elle envoie
/// des datagrammes à une adresse, et ne sait rien d'un relais. Le poste
/// qu'on ne peut pas joindre *attend* donc au relais en UDP — il y redit
/// toutes les [`UDP_RENEW`] le nom de son groupe et l'identifiant ICE de
/// sa boîte (`ufrag`), ce qui garde aussi ouverte la traduction d'adresse
/// de son opérateur — et publie l'adresse du relais parmi les siennes.
/// La page la compose comme une autre. Le relais lit seulement le nom
/// d'utilisateur de la première demande STUN de la page (`ufrag:page`),
/// la passe aux postes qui attendent sous ce `ufrag`, enveloppée de
/// l'adresse de la page ; le premier poste qui répond à cette page la
/// garde, et le relais passe dès lors les datagrammes de l'un à l'autre
/// sans les lire. DTLS (l'empreinte du code QR) et la clé du patient se
/// négocient de bout en bout : le relais n'a aucune clé, et n'écrit rien.
///
/// Il ne renvoie un datagramme d'un poste qu'à une page qui lui a écrit
/// d'abord : on ne fait pas du relais un canon à datagrammes. Et une
/// attente ne compte qu'une fois **revenue avec le témoin** que le relais
/// a donné à son adresse ([`UdpFrame::Held`]) : une attente au nom d'une
/// adresse usurpée n'est jamais tenue, et le relais ne copie rien vers
/// elle.
pub const UDP_MAGIC: [u8; 4] = *b"BPMU";
const UDP_WAIT: u8 = 1;
const UDP_HELD: u8 = 2;
const UDP_DATA: u8 = 3;
/// Le poste redit son attente tous les… — une traduction d'adresse
/// d'opérateur oublie un flux UDP muet en trente secondes au pire.
pub const UDP_RENEW: Duration = Duration::from_secs(15);
/// …et le relais l'oublie après ce silence.
pub const UDP_WAIT_BOUND: Duration = Duration::from_secs(60);
/// Une page dont la première demande est passée, et qu'aucun poste n'a
/// encore prise.
const UDP_PENDING_BOUND: Duration = Duration::from_secs(20);
const UDP_MOST_PENDING: usize = 256;
/// Combien de datagrammes d'une page pas encore prise le relais passe —
/// un navigateur redit sa demande quelques fois par paire d'adresses ;
/// au-delà, une même adresse ferait copier chacun de ses datagrammes à
/// chaque poste qui attend, sans fin.
const UDP_PENDING_FORWARDS: u32 = 32;
/// Combien de pages relayées à la fois, en tout et par abonné.
const UDP_MOST_SESSIONS: usize = 128;
const UDP_SESSIONS_PER_CALLER: usize = 8;

/// Une trame entre un poste et le relais.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UdpFrame<'a> {
    /// Du poste : « j'attends, pour ce groupe, les pages de ce `ufrag` »,
    /// avec le témoin que le relais a donné à son adresse (zéro avant).
    Wait {
        group: &'a str,
        ufrag: &'a str,
        cookie: [u8; 16],
    },
    /// Du relais : le témoin de l'adresse qui attend, et si l'attente est
    /// tenue — elle ne l'est qu'avec ce témoin.
    Held { cookie: [u8; 16], held: bool },
    /// Dans les deux sens : un datagramme de la page, ou pour elle.
    Data { peer: SocketAddr, payload: &'a [u8] },
}

/// Un `ufrag` qu'un relais tient : ce qu'ICE permet (RFC 8445 § 5.3),
/// de quatre à trente-deux caractères, sans le deux-points qui le sépare
/// de celui de la page.
fn plain_ufrag(u: &str) -> bool {
    (4..=32).contains(&u.len())
        && u.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'+' || b == b'/')
}

pub fn udp_encode(frame: &UdpFrame) -> Vec<u8> {
    let mut out = UDP_MAGIC.to_vec();
    match frame {
        UdpFrame::Wait {
            group,
            ufrag,
            cookie,
        } => {
            out.push(UDP_WAIT);
            out.extend_from_slice(group.as_bytes());
            out.extend_from_slice(cookie);
            out.push(ufrag.len() as u8);
            out.extend_from_slice(ufrag.as_bytes());
        }
        UdpFrame::Held { cookie, held } => {
            out.push(UDP_HELD);
            out.extend_from_slice(cookie);
            out.push(u8::from(*held));
        }
        UdpFrame::Data { peer, payload } => {
            out.push(UDP_DATA);
            match peer.ip() {
                IpAddr::V4(ip) => {
                    out.push(4);
                    out.extend_from_slice(&ip.octets());
                }
                IpAddr::V6(ip) => {
                    out.push(6);
                    out.extend_from_slice(&ip.octets());
                }
            }
            out.extend_from_slice(&peer.port().to_be_bytes());
            out.extend_from_slice(payload);
        }
    }
    out
}

pub fn udp_decode(b: &[u8]) -> Option<UdpFrame<'_>> {
    let rest = b.strip_prefix(&UDP_MAGIC)?;
    let (&kind, rest) = rest.split_first()?;
    match kind {
        UDP_WAIT => {
            let group = std::str::from_utf8(rest.get(..20)?).ok()?;
            let cookie: [u8; 16] = rest.get(20..36)?.try_into().ok()?;
            let (&n, ufrag) = rest[36..].split_first()?;
            let ufrag = std::str::from_utf8(ufrag).ok()?;
            let hex = group
                .bytes()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase());
            (hex && ufrag.len() == n as usize && plain_ufrag(ufrag)).then_some(UdpFrame::Wait {
                group,
                ufrag,
                cookie,
            })
        }
        UDP_HELD => {
            let cookie: [u8; 16] = rest.get(..16)?.try_into().ok()?;
            match rest.get(16..)? {
                [flag @ (0 | 1)] => Some(UdpFrame::Held {
                    cookie,
                    held: *flag == 1,
                }),
                _ => None,
            }
        }
        UDP_DATA => {
            let (&family, rest) = rest.split_first()?;
            let (ip, rest): (IpAddr, &[u8]) = match family {
                4 => (<[u8; 4]>::try_from(rest.get(..4)?).ok()?.into(), &rest[4..]),
                6 => (
                    <[u8; 16]>::try_from(rest.get(..16)?).ok()?.into(),
                    &rest[16..],
                ),
                _ => return None,
            };
            let port = u16::from_be_bytes([*rest.first()?, *rest.get(1)?]);
            let payload = &rest[2..];
            (port != 0 && !payload.is_empty()).then_some(UdpFrame::Data {
                peer: SocketAddr::new(ip, port),
                payload,
            })
        }
        _ => None,
    }
}

struct UdpSession {
    post: SocketAddr,
    born: Instant,
    last: Instant,
    moved: u64,
}

/// **Ce que le relais UDP tient en mémoire**, et rien d'autre : qui
/// attend, quelles pages viennent d'écrire, quelles pages parlent à quel
/// poste. Sans prise ni horloge — l'heure est passée — et donc testé tel
/// quel ; [`serve_udp`] le fait parler.
pub struct UdpRelay {
    /// La clé des témoins : tirée au lancement, jamais écrite.
    key: [u8; 32],
    waiters: HashMap<String, Vec<(SocketAddr, Instant)>>,
    pending: HashMap<SocketAddr, (String, Instant, u32)>,
    sessions: HashMap<SocketAddr, UdpSession>,
    /// Les pages arrivées au bout de leurs bornes (durée, octets) : elles
    /// ne repartent pas aussitôt pour un tour — sans cela, la demande
    /// suivante du navigateur rouvrait une conversation sans fin.
    spent: HashMap<SocketAddr, Instant>,
    calls: Mutex<HashMap<IpAddr, Vec<Instant>>>,
}

impl Default for UdpRelay {
    fn default() -> Self {
        let mut key = [0u8; 32];
        bpm_sync::Entropy::fill(&mut bpm_sync::OsEntropy, &mut key);
        Self::new(key)
    }
}

impl UdpRelay {
    pub fn new(key: [u8; 32]) -> Self {
        Self {
            key,
            waiters: HashMap::new(),
            pending: HashMap::new(),
            sessions: HashMap::new(),
            spent: HashMap::new(),
            calls: Mutex::default(),
        }
    }

    /// Le témoin d'une adresse : ce qu'une attente doit rapporter pour
    /// compter — seule l'adresse qui l'a reçu le connaît.
    pub fn cookie(&self, from: SocketAddr) -> [u8; 16] {
        let mut h = blake3::Hasher::new_keyed(&self.key);
        h.update(from.to_string().as_bytes());
        let mut out = [0u8; 16];
        out.copy_from_slice(&h.finalize().as_bytes()[..16]);
        out
    }

    /// Lâcher ce qui a passé ses bornes.
    pub fn prune(&mut self, now: Instant) {
        self.waiters.retain(|_, v| {
            v.retain(|(_, t)| now.duration_since(*t) < UDP_WAIT_BOUND);
            !v.is_empty()
        });
        self.pending
            .retain(|_, (_, t, _)| now.duration_since(*t) < UDP_PENDING_BOUND);
        self.spent
            .retain(|_, t| now.duration_since(*t) < TALK_BOUND);
        let spent = &mut self.spent;
        self.sessions.retain(|page, s| {
            let worn = now.duration_since(s.born) >= TALK_BOUND || s.moved >= 2 * TALK_BYTES;
            if worn && spent.len() < UDP_MOST_PENDING {
                spent.insert(*page, now);
            }
            !worn && now.duration_since(s.last) < IDLE
        });
    }

    /// Un datagramme reçu de `from` : ce qu'il faut envoyer, et à qui.
    pub fn handle(
        &mut self,
        now: Instant,
        from: SocketAddr,
        data: &[u8],
        allowed: &HashSet<String>,
    ) -> Vec<(SocketAddr, Vec<u8>)> {
        match udp_decode(data) {
            Some(UdpFrame::Wait {
                group,
                ufrag,
                cookie,
            }) => {
                if !allowed.contains(group) {
                    return Vec::new();
                }
                // Not yet back with its address's cookie: told it, not
                // held — a reply no larger than what was sent, to the
                // address it claims.
                let expected = self.cookie(from);
                if cookie != expected {
                    return vec![(
                        from,
                        udp_encode(&UdpFrame::Held {
                            cookie: expected,
                            held: false,
                        }),
                    )];
                }
                if !self.waiters.contains_key(ufrag) && self.waiters.len() >= MOST_GROUPS {
                    return Vec::new();
                }
                let list = self.waiters.entry(ufrag.to_owned()).or_default();
                if let Some(w) = list.iter_mut().find(|(a, _)| *a == from) {
                    w.1 = now;
                } else if list.len() < WAITING_PER_GROUP {
                    list.push((from, now));
                } else if let Some(oldest) = list.iter_mut().min_by_key(|(_, t)| *t) {
                    // Full: the wait heard least lately makes room — a
                    // post whose operator moved its port leaves its old
                    // address behind for a minute.
                    *oldest = (from, now);
                }
                vec![(
                    from,
                    udp_encode(&UdpFrame::Held {
                        cookie: expected,
                        held: true,
                    }),
                )]
            }
            Some(UdpFrame::Data { peer, payload }) => {
                if let Some(s) = self.sessions.get_mut(&peer) {
                    if s.post != from {
                        return Vec::new();
                    }
                    s.last = now;
                    s.moved += payload.len() as u64;
                    return vec![(peer, payload.to_vec())];
                }
                // The first answer to a page: the post that gives it keeps
                // it — if it waits under the ufrag the page asked for.
                let Some((ufrag, _, _)) = self.pending.get(&peer) else {
                    return Vec::new();
                };
                let waits = self
                    .waiters
                    .get(ufrag)
                    .is_some_and(|v| v.iter().any(|(a, _)| *a == from));
                let who = caller(peer.ip());
                let theirs = self
                    .sessions
                    .keys()
                    .filter(|a| caller(a.ip()) == who)
                    .count();
                if !waits
                    || self.sessions.len() >= UDP_MOST_SESSIONS
                    || theirs >= UDP_SESSIONS_PER_CALLER
                {
                    return Vec::new();
                }
                self.pending.remove(&peer);
                self.sessions.insert(
                    peer,
                    UdpSession {
                        post: from,
                        born: now,
                        last: now,
                        moved: payload.len() as u64,
                    },
                );
                vec![(peer, payload.to_vec())]
            }
            Some(UdpFrame::Held { .. }) => Vec::new(),
            None => self.page_said(now, from, data),
        }
    }

    fn page_said(
        &mut self,
        now: Instant,
        from: SocketAddr,
        data: &[u8],
    ) -> Vec<(SocketAddr, Vec<u8>)> {
        let wrap = |data: &[u8]| {
            udp_encode(&UdpFrame::Data {
                peer: from,
                payload: data,
            })
        };
        if let Some(s) = self.sessions.get_mut(&from) {
            s.last = now;
            s.moved += data.len() as u64;
            return vec![(s.post, wrap(data))];
        }
        // A page not yet taken: only a binding request, under a ufrag a
        // post waits for, and within the calls an address may place.
        let Some(ufrag) = crate::direct::stun_username(data)
            .and_then(|u| u.split_once(':'))
            .map(|(ours, _)| ours)
        else {
            return Vec::new();
        };
        let Some(posts) = self.waiters.get(ufrag).filter(|v| !v.is_empty()) else {
            return Vec::new();
        };
        if self.spent.contains_key(&from) {
            return Vec::new();
        }
        match self.pending.get_mut(&from) {
            Some((asked, _, forwarded)) => {
                if asked != ufrag || *forwarded >= UDP_PENDING_FORWARDS {
                    return Vec::new();
                }
                *forwarded += 1;
            }
            None => {
                if self.pending.len() >= UDP_MOST_PENDING || !admit(&self.calls, from.ip()) {
                    return Vec::new();
                }
                self.pending.insert(from, (ufrag.to_owned(), now, 1));
            }
        }
        posts.iter().map(|(p, _)| (*p, wrap(data))).collect()
    }
}

/// **Qui appelle**, pour les bornes : une IPv4, ou le `/64` d'une IPv6.
fn caller(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V4(_) => ip,
        IpAddr::V6(v6) => {
            let mut o = v6.octets();
            o[8..].fill(0);
            IpAddr::V6(o.into())
        }
    }
}

/// **Tenir le relais UDP** sur `port` (celui du relais TCP) jusqu'à ce
/// que `stop` passe à vrai. Rend tout de suite ; le relais vit sur son
/// propre fil.
pub fn serve_udp(
    port: u16,
    allowed: Allowed,
    stop: Arc<AtomicBool>,
) -> std::io::Result<SocketAddr> {
    let socket = UdpSocket::bind(("0.0.0.0", port))?;
    socket.set_read_timeout(Some(Duration::from_millis(250)))?;
    let at = socket.local_addr()?;
    std::thread::spawn(move || {
        let mut relay = UdpRelay::default();
        let mut buf = vec![0u8; 2048];
        let mut pruned = Instant::now();
        let mut groups: HashSet<String> = HashSet::new();
        let mut read: Option<Instant> = None;
        while !stop.load(Ordering::SeqCst) {
            let now = Instant::now();
            if read.is_none_or(|t| now.duration_since(t) >= Duration::from_secs(1)) {
                read = Some(now);
                if let Ok(a) = allowed.lock() {
                    groups.clone_from(&a);
                }
            }
            if now.duration_since(pruned) >= Duration::from_secs(1) {
                pruned = now;
                relay.prune(now);
            }
            let (n, from) = match socket.recv_from(&mut buf) {
                Ok(got) => got,
                Err(_) => continue,
            };
            for (to, bytes) in relay.handle(now, from, &buf[..n], &groups) {
                let _ = socket.send_to(&bytes, to);
            }
        }
    });
    Ok(at)
}

#[cfg(test)]
mod tests {
    use super::*;

    const G: &str = "0123456789abcdef0123";

    #[test]
    fn a_udp_frame_reads_back_and_nothing_else_does() {
        let page: SocketAddr = "203.0.113.7:51000".parse().unwrap();
        let page6: SocketAddr = "[2001:db8::7]:51000".parse().unwrap();
        for f in [
            UdpFrame::Wait {
                group: G,
                ufrag: "01234567",
                cookie: [3; 16],
            },
            UdpFrame::Held {
                cookie: [4; 16],
                held: true,
            },
            UdpFrame::Held {
                cookie: [0; 16],
                held: false,
            },
            UdpFrame::Data {
                peer: page,
                payload: b"stun",
            },
            UdpFrame::Data {
                peer: page6,
                payload: b"dtls",
            },
        ] {
            assert_eq!(udp_decode(&udp_encode(&f)), Some(f));
        }
        let wait = |group: &str, ufrag: &str| {
            udp_encode(&UdpFrame::Wait {
                group,
                ufrag,
                cookie: [0; 16],
            })
        };
        let mut held_flag = udp_encode(&UdpFrame::Held {
            cookie: [4; 16],
            held: true,
        });
        *held_flag.last_mut().unwrap() = 2;
        for bad in [
            wait("0123456789ABCDEF0123", "01234567"),
            wait("0123456789abcdef012z", "01234567"),
            wait(G, "012"),
            wait(G, "0123:567"),
            wait(G, &"a".repeat(33)),
            udp_encode(&UdpFrame::Data {
                peer: page,
                payload: b"",
            }),
            udp_encode(&UdpFrame::Data {
                peer: "203.0.113.7:0".parse().unwrap(),
                payload: b"x",
            }),
            held_flag,
            b"BPMU\x09".to_vec(),
            b"BPMU\x02x".to_vec(),
            b"\x00\x01\x00\x00".to_vec(),
        ] {
            assert_eq!(udp_decode(&bad), None, "{bad:?}");
        }
        let mut short = wait(G, "01234567");
        short.pop();
        assert_eq!(udp_decode(&short), None, "le ufrag tronqué");
    }

    /// Une demande de liaison STUN, comme la page l'envoie.
    fn binding(username: &str) -> Vec<u8> {
        let mut p = vec![0x00, 0x01, 0x00, 0x00, 0x21, 0x12, 0xa4, 0x42];
        p.extend_from_slice(&[9; 12]);
        p.extend_from_slice(&[0x00, 0x06]);
        p.extend_from_slice(&(username.len() as u16).to_be_bytes());
        p.extend_from_slice(username.as_bytes());
        while p.len() % 4 != 0 {
            p.push(0);
        }
        p
    }

    /// L'attente de `from` sous `ufrag`, avec le témoin que le relais lui
    /// a donné.
    fn wait_with(r: &UdpRelay, from: SocketAddr, group: &str, ufrag: &str) -> Vec<u8> {
        udp_encode(&UdpFrame::Wait {
            group,
            ufrag,
            cookie: r.cookie(from),
        })
    }

    /// **Une attente ne compte qu'avec le témoin de son adresse** : sans
    /// lui, le relais le dit à l'adresse — une réponse pas plus grande que
    /// la demande — et ne tient rien ; une attente au nom d'une autre
    /// adresse (usurpée) ne fait donc rien copier vers elle.
    #[test]
    fn a_wait_counts_only_back_with_its_address_s_cookie() {
        let allowed: HashSet<String> = [G.to_owned()].into();
        let post: SocketAddr = "198.51.100.1:4000".parse().unwrap();
        let victim: SocketAddr = "192.0.2.99:53".parse().unwrap();
        let t0 = Instant::now();
        let mut r = UdpRelay::new([7; 32]);
        assert_ne!(r.cookie(post), r.cookie(victim));
        assert_ne!(
            r.cookie(post),
            UdpRelay::new([8; 32]).cookie(post),
            "par lancement"
        );
        let bare = udp_encode(&UdpFrame::Wait {
            group: G,
            ufrag: "01234567",
            cookie: [0; 16],
        });
        let told = r.handle(t0, post, &bare, &allowed);
        assert_eq!(
            told,
            vec![(
                post,
                udp_encode(&UdpFrame::Held {
                    cookie: r.cookie(post),
                    held: false,
                })
            )]
        );
        assert!(told[0].1.len() <= bare.len());
        // The victim's cookie never reached the spoofer: its waits, under
        // any cookie but the victim's own, hold nothing.
        let spoofed = wait_with(&r, post, G, "01234567");
        r.handle(t0, victim, &spoofed, &allowed);
        let stun = binding("01234567:page");
        let page: SocketAddr = "203.0.113.7:51000".parse().unwrap();
        assert!(
            r.handle(t0, page, &stun, &allowed).is_empty(),
            "personne n'attend"
        );
        // Back with its own: held.
        let held = r.handle(t0, post, &wait_with(&r, post, G, "01234567"), &allowed);
        assert_eq!(
            udp_decode(&held[0].1),
            Some(UdpFrame::Held {
                cookie: r.cookie(post),
                held: true
            })
        );
        assert_eq!(r.handle(t0, page, &stun, &allowed).len(), 1);
    }

    /// **Le relais UDP, sans prise** : une attente d'un groupe accepté
    /// seulement ; la première demande d'une page passe à tous les postes
    /// qui attendent sous son `ufrag` ; le premier qui lui répond la garde ;
    /// un poste n'écrit qu'à une page qui a écrit d'abord ; tout s'oublie
    /// à ses bornes.
    #[test]
    fn the_udp_relay_puts_a_page_in_touch_with_one_waiting_post() {
        let allowed: HashSet<String> = [G.to_owned()].into();
        let (a, b, stranger): (SocketAddr, SocketAddr, SocketAddr) = (
            "198.51.100.1:4000".parse().unwrap(),
            "198.51.100.2:4000".parse().unwrap(),
            "198.51.100.9:4000".parse().unwrap(),
        );
        let page: SocketAddr = "203.0.113.7:51000".parse().unwrap();
        let other_page: SocketAddr = "203.0.113.8:51000".parse().unwrap();
        let t0 = Instant::now();
        let mut r = UdpRelay::default();
        let held = |r: &UdpRelay, at| {
            vec![(
                at,
                udp_encode(&UdpFrame::Held {
                    cookie: r.cookie(at),
                    held: true,
                }),
            )]
        };
        let other_group = wait_with(&r, a, "ffffffffffffffffffff", "01234567");
        assert!(r.handle(t0, a, &other_group, &allowed).is_empty());
        let wa = wait_with(&r, a, G, "01234567");
        assert_eq!(r.handle(t0, a, &wa, &allowed), held(&r, a));
        assert_eq!(r.handle(t0, a, &wa, &allowed), held(&r, a), "redite");
        let wb = wait_with(&r, b, G, "01234567");
        assert_eq!(r.handle(t0, b, &wb, &allowed), held(&r, b));

        // Nobody waits under another ufrag; a datagram that is not STUN
        // from an unknown page goes nowhere.
        assert!(r
            .handle(t0, page, &binding("ffffffff:page"), &allowed)
            .is_empty());
        assert!(r
            .handle(t0, page, b"\x16\xfe\xfd dtls", &allowed)
            .is_empty());
        let stun = binding("01234567:page");
        let out = r.handle(t0, page, &stun, &allowed);
        let wrapped = udp_encode(&UdpFrame::Data {
            peer: page,
            payload: &stun,
        });
        let mut to: Vec<SocketAddr> = out.iter().map(|(t, _)| *t).collect();
        to.sort();
        assert_eq!(to, vec![a, b], "à chacun des deux");
        assert!(out.iter().all(|(_, d)| *d == wrapped));

        // A post that does not wait, or a page that never wrote: dropped.
        let answer = |peer| {
            udp_encode(&UdpFrame::Data {
                peer,
                payload: b"answer",
            })
        };
        assert!(r.handle(t0, stranger, &answer(page), &allowed).is_empty());
        assert!(
            r.handle(t0, a, &answer(other_page), &allowed).is_empty(),
            "pas un canon"
        );
        // B answers first and keeps the page; A's answer goes nowhere.
        assert_eq!(
            r.handle(t0, b, &answer(page), &allowed),
            vec![(page, b"answer".to_vec())]
        );
        assert!(r.handle(t0, a, &answer(page), &allowed).is_empty());
        // From now on the page's datagrams, STUN or not, go to B alone.
        let dtls = b"\x16\xfe\xfd dtls".to_vec();
        assert_eq!(
            r.handle(t0, page, &dtls, &allowed),
            vec![(
                b,
                udp_encode(&UdpFrame::Data {
                    peer: page,
                    payload: &dtls,
                })
            )]
        );

        // Silence past the bounds: the page, then the waits, forgotten.
        r.prune(t0 + IDLE);
        assert!(r.handle(t0 + IDLE, page, &dtls, &allowed).is_empty());
        r.prune(t0 + UDP_WAIT_BOUND + IDLE);
        assert!(r
            .handle(t0 + UDP_WAIT_BOUND + IDLE, page, &stun, &allowed)
            .is_empty());
    }

    /// **Une conversation a une fin** : passée sa durée, la page est
    /// lâchée, et sa demande suivante — le navigateur en redit une toutes
    /// les quelques secondes — n'en rouvre pas une autre avant que la
    /// durée ait passé encore.
    #[test]
    fn a_relayed_page_past_its_bound_does_not_start_over() {
        let allowed: HashSet<String> = [G.to_owned()].into();
        let post: SocketAddr = "198.51.100.1:4000".parse().unwrap();
        let page: SocketAddr = "203.0.113.7:51000".parse().unwrap();
        let t0 = Instant::now();
        let mut r = UdpRelay::default();
        let stun = binding("01234567:page");
        let answer = udp_encode(&UdpFrame::Data {
            peer: page,
            payload: b"answer",
        });
        // Kept talking (never idle) until the bound.
        let mut t = t0;
        while t < t0 + TALK_BOUND {
            r.prune(t);
            let w = wait_with(&r, post, G, "01234567");
            r.handle(t, post, &w, &allowed);
            r.handle(t, page, &stun, &allowed);
            assert_eq!(
                r.handle(t, post, &answer, &allowed).len(),
                1,
                "{:?}",
                t - t0
            );
            t += Duration::from_secs(10);
        }
        r.prune(t);
        let w = wait_with(&r, post, G, "01234567");
        r.handle(t, post, &w, &allowed);
        assert!(
            r.handle(t, page, &stun, &allowed).is_empty(),
            "pas un autre tour"
        );
        assert!(r.handle(t, post, &answer, &allowed).is_empty());
        // Another page from the same box is not refused for it.
        let next: SocketAddr = "203.0.113.7:51001".parse().unwrap();
        assert_eq!(r.handle(t, next, &stun, &allowed).len(), 1);
        // Once the bound has passed again, the page is a page like any.
        let later = t + TALK_BOUND;
        r.prune(later);
        let w = wait_with(&r, post, G, "01234567");
        r.handle(later, post, &w, &allowed);
        assert_eq!(r.handle(later, page, &stun, &allowed).len(), 1);
    }

    /// **Une liste pleine fait de la place** : le poste dont l'opérateur a
    /// changé le port arrive sous une adresse neuve — l'attente entendue
    /// le moins récemment lui cède la sienne, au lieu de la refuser une
    /// minute.
    #[test]
    fn a_full_wait_list_makes_room_for_the_newest() {
        let allowed: HashSet<String> = [G.to_owned()].into();
        let t0 = Instant::now();
        let mut r = UdpRelay::default();
        let at = |i: u16| SocketAddr::from(([198, 51, 100, 1], 4000 + i));
        for i in 0..WAITING_PER_GROUP as u16 {
            let w = wait_with(&r, at(i), G, "01234567");
            r.handle(t0 + Duration::from_secs(u64::from(i)), at(i), &w, &allowed);
        }
        let late = t0 + Duration::from_secs(30);
        let w = wait_with(&r, at(99), G, "01234567");
        let held = r.handle(late, at(99), &w, &allowed);
        assert!(matches!(
            udp_decode(&held[0].1),
            Some(UdpFrame::Held { held: true, .. })
        ));
        let page: SocketAddr = "203.0.113.7:51000".parse().unwrap();
        let mut to: Vec<SocketAddr> = r
            .handle(late, page, &binding("01234567:page"), &allowed)
            .into_iter()
            .map(|(t, _)| t)
            .collect();
        to.sort();
        assert_eq!(
            to,
            vec![at(1), at(2), at(3), at(99)],
            "la plus ancienne a cédé"
        );
    }

    /// Une adresse qui frappe trop souvent ne crée plus de page en attente
    /// — redire la même demande ne compte qu'une fois, et ne passe qu'un
    /// nombre borné de fois.
    #[test]
    fn a_page_address_places_a_bounded_number_of_calls() {
        let allowed: HashSet<String> = [G.to_owned()].into();
        let post: SocketAddr = "198.51.100.1:4000".parse().unwrap();
        let t0 = Instant::now();
        let mut r = UdpRelay::default();
        let wait = wait_with(&r, post, G, "01234567");
        r.handle(t0, post, &wait, &allowed);
        let stun = binding("01234567:page");
        let same: SocketAddr = "203.0.113.7:51000".parse().unwrap();
        let forwarded = (0..100)
            .filter(|_| !r.handle(t0, same, &stun, &allowed).is_empty())
            .count();
        assert_eq!(forwarded, UDP_PENDING_FORWARDS as usize, "redite, bornée");
        let passed = (0..100u16)
            .filter(|i| {
                let from = SocketAddr::new("203.0.113.8".parse().unwrap(), 2000 + i);
                !r.handle(t0, from, &stun, &allowed).is_empty()
            })
            .count();
        assert_eq!(passed, CALLS_PER_MINUTE);
    }

    /// **De bout en bout, sur la boucle locale** : un poste qui n'écoute
    /// à aucune adresse, seulement au relais ; la page compose le relais
    /// et ouvre son canal, DTLS contre l'empreinte de l'officine — et
    /// les octets passent dans les deux sens.
    #[test]
    fn a_page_reaches_a_post_through_the_relay_alone() {
        let allowed: Allowed = Arc::new(Mutex::new([G.to_owned()].into()));
        let stop = Arc::new(AtomicBool::new(false));
        let at = serve_udp(0, allowed, Arc::clone(&stop)).unwrap();
        let relay = SocketAddr::from(([127, 0, 0, 1], at.port()));
        let id = crate::direct::Identity::draw().unwrap();
        let box_id = "0123456789abcdef0123456789abcdef";
        let (tx, links) = std::sync::mpsc::channel();
        let listening = crate::direct::serve_relayed(Vec::new(), &[relay], G, &id, box_id, tx);
        let until = Instant::now() + Duration::from_secs(5);
        while listening.relayed().is_empty() && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(20));
        }
        assert_eq!(listening.relayed(), vec![relay], "l'attente tenue");
        let mut page =
            crate::direct::tests::dial(&[relay], box_id, id.fingerprint()).expect("canal ouvert");
        let mut link = links
            .recv_timeout(Duration::from_secs(10))
            .expect("une page");
        assert_eq!(
            link.from.ip(),
            relay.ip(),
            "l'adresse de la page, pas du relais"
        );
        assert_ne!(link.from, relay);
        page.write_all(b"bonjour").unwrap();
        let mut got = [0u8; 7];
        link.read_exact(&mut got).unwrap();
        assert_eq!(&got, b"bonjour");
        let big: Vec<u8> = (0..60_000u32).map(|i| (i % 251) as u8).collect();
        link.write_all(&big).unwrap();
        let mut back = vec![0u8; big.len()];
        page.read_exact(&mut back).unwrap();
        assert_eq!(back, big);
        stop.store(true, Ordering::SeqCst);
    }

    /// Un relais qui ne tient pas le groupe ne tient pas l'attente : le
    /// poste ne le publie pas.
    #[test]
    fn a_relay_that_refuses_the_group_is_not_published() {
        let allowed: Allowed = Arc::default();
        let stop = Arc::new(AtomicBool::new(false));
        let at = serve_udp(0, allowed, Arc::clone(&stop)).unwrap();
        let relay = SocketAddr::from(([127, 0, 0, 1], at.port()));
        let id = crate::direct::Identity::draw().unwrap();
        let (tx, _links) = std::sync::mpsc::channel();
        let listening = crate::direct::serve_relayed(
            Vec::new(),
            &[relay],
            G,
            &id,
            "0123456789abcdef0123456789abcdef",
            tx,
        );
        std::thread::sleep(Duration::from_millis(500));
        assert!(listening.relayed().is_empty());
        stop.store(true, Ordering::SeqCst);
    }

    #[test]
    fn a_preface_reads_back_and_nothing_else_does() {
        assert_eq!(
            read_preface(&preface(Role::Attente, G)),
            Some((Role::Attente, G.to_owned()))
        );
        assert_eq!(
            read_preface(&preface(Role::Appel, &G.to_uppercase())),
            Some((Role::Appel, G.to_owned()))
        );
        for bad in [
            "",
            "BPMRELAIS1",
            "BPMRELAIS1 appel",
            "BPMRELAIS1 appel 0123",
            "BPMRELAIS1 ecoute 0123456789abcdef0123",
            "BPMRELAIS2 appel 0123456789abcdef0123",
            "BPMRELAIS1 appel 0123456789abcdef0123 de trop",
            "BPMRELAIS1 appel zz23456789abcdef0123",
        ] {
            assert!(read_preface(bad).is_none(), "{bad}");
        }
    }

    fn relay(groups: &[&str]) -> (u16, Arc<AtomicBool>) {
        let port = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let allowed: Allowed = Arc::new(Mutex::new(groups.iter().map(|g| g.to_string()).collect()));
        let stop = Arc::new(AtomicBool::new(false));
        serve(port, allowed, Arc::clone(&stop)).unwrap();
        (port, stop)
    }

    /// A waiting post and a caller of the same group are put in touch,
    /// and bytes go both ways untouched.
    #[test]
    fn a_waiting_post_and_a_caller_talk_through_the_relay() {
        let (port, stop) = relay(&[G]);
        let at = format!("127.0.0.1:{port}");
        let waiter = {
            let at = at.clone();
            std::thread::spawn(move || {
                let mut s = wait(&at, G, Duration::from_secs(5)).unwrap();
                let mut got = [0u8; 5];
                s.read_exact(&mut got).unwrap();
                s.write_all(b"pong!").unwrap();
                got
            })
        };
        // Let the waiting post register first.
        std::thread::sleep(Duration::from_millis(300));
        let mut c = call(&at, G, Duration::from_secs(5)).unwrap();
        c.write_all(b"ping!").unwrap();
        let mut back = [0u8; 5];
        c.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        c.read_exact(&mut back).unwrap();
        assert_eq!(&back, b"pong!");
        assert_eq!(&waiter.join().unwrap(), b"ping!");
        stop.store(true, Ordering::SeqCst);
    }

    /// A preface trickled one byte at a time is let go when its whole
    /// deadline passes, not a byte's: nobody holds a thread for minutes.
    #[test]
    fn a_trickled_preface_is_let_go_on_time() {
        let (port, stop) = relay(&[G]);
        let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
        let begun = Instant::now();
        let mut closed = false;
        for b in b"BPMRELAIS1 attente ".iter() {
            if s.write_all(&[*b]).is_err() {
                closed = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(700));
            if begun.elapsed() > PREFACE_WAIT + Duration::from_secs(2) {
                break;
            }
        }
        if !closed {
            s.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
            let mut buf = [0u8; 1];
            // Closed reads as 0 (or a reset); still open would time out.
            match s.read(&mut buf) {
                Ok(0) => {}
                Err(e) => assert!(
                    !matches!(
                        e.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ),
                    "toujours ouverte"
                ),
                Ok(_) => panic!("le relais a répondu"),
            }
        }
        assert!(begun.elapsed() < PREFACE_WAIT + Duration::from_secs(6));
        stop.store(true, Ordering::SeqCst);
    }

    /// A group nobody asked to relay gets nothing, and a caller with no
    /// post waiting is let go at once.
    #[test]
    fn a_stranger_group_or_an_empty_one_is_let_go() {
        let (port, stop) = relay(&[G]);
        let at = format!("127.0.0.1:{port}");
        let mut c = call(&at, "ffffffffffffffffffff", Duration::from_secs(5)).unwrap();
        c.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        let mut buf = [0u8; 1];
        assert_eq!(c.read(&mut buf).unwrap_or(0), 0, "fermé");
        let mut c = call(&at, G, Duration::from_secs(5)).unwrap();
        c.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        assert_eq!(c.read(&mut buf).unwrap_or(0), 0, "personne n'attend");
        stop.store(true, Ordering::SeqCst);
    }
}
