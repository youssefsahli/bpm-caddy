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
use std::net::{IpAddr, TcpListener, TcpStream, ToSocketAddrs};
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

/// Combien de postes d'un même groupe peuvent attendre à la fois.
const WAITING_PER_GROUP: usize = 4;
/// Combien de groupes à la fois.
const MOST_GROUPS: usize = 64;
/// Une attente se renouvelle : au-delà, le relais la lâche.
const WAIT_BOUND: Duration = Duration::from_secs(10 * 60);
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
    std::thread::spawn(move || {
        while !stop.load(Ordering::SeqCst) {
            let (stream, from) = match listener.accept() {
                Ok(s) => s,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(100));
                    continue;
                }
                Err(_) => continue,
            };
            if !admit(&calls, from.ip()) {
                continue;
            }
            let (allowed, waiting) = (Arc::clone(&allowed), Arc::clone(&waiting));
            std::thread::spawn(move || {
                let _ = take(stream, &allowed, &waiting);
            });
        }
    });
    Ok(())
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
    if calls.len() > 1024 {
        calls.clear();
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
            w.retain(|_, v| {
                v.retain(|x| x.since.elapsed() < WAIT_BOUND);
                !v.is_empty()
            });
            if !w.contains_key(&group) && w.len() >= MOST_GROUPS {
                return Ok(());
            }
            let list = w.entry(group).or_default();
            if list.len() >= WAITING_PER_GROUP {
                list.remove(0);
            }
            list.push(Waiting {
                stream,
                since: Instant::now(),
            });
            Ok(())
        }
        Role::Appel => loop {
            let next = waiting
                .lock()
                .ok()
                .and_then(|mut w| w.get_mut(&group).and_then(|v| v.pop()));
            let Some(mut other) = next else {
                return Ok(());
            };
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
    let mut out = Vec::with_capacity(64);
    let mut byte = [0u8; 1];
    while out.len() < 128 {
        stream.read_exact(&mut byte)?;
        if byte[0] == b'\n' {
            break;
        }
        out.push(byte[0]);
    }
    Ok(String::from_utf8_lossy(&out).into_owned())
}

/// Passer les octets de l'un à l'autre, dans les deux sens, jusqu'à ce
/// que l'un se taise, ferme, ou dépasse les bornes.
fn splice(a: TcpStream, b: TcpStream) {
    let until = Instant::now() + TALK_BOUND;
    for s in [&a, &b] {
        let _ = s.set_read_timeout(Some(IDLE));
        let _ = s.set_write_timeout(Some(IDLE));
    }
    let (Ok(a2), Ok(b2)) = (a.try_clone(), b.try_clone()) else {
        return;
    };
    let one = std::thread::spawn(move || pipe(a, b2, until));
    pipe(b, a2, until);
    let _ = one.join();
}

fn pipe(mut from: TcpStream, mut to: TcpStream, until: Instant) {
    let mut buf = [0u8; 16 * 1024];
    let mut moved: u64 = 0;
    loop {
        if Instant::now() >= until || moved >= TALK_BYTES {
            break;
        }
        match from.read(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                moved += n as u64;
                if to.write_all(&buf[..n]).is_err() {
                    break;
                }
            }
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
    s.set_read_timeout(Some(WAIT_BOUND))?;
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

#[cfg(test)]
mod tests {
    use super::*;

    const G: &str = "0123456789abcdef0123";

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
