//! **Le rendez-vous** : la page du patient et un poste de l'officine se
//! retrouvent sans que l'un puisse joindre l'autre — ni box ouverte, ni
//! IPv6 publique, ni serveur de l'éditeur.
//!
//! Chacun sort vers Internet, ce que toute box laisse faire. La page
//! demande à un serveur STUN public l'adresse que sa box lui donne, et
//! le poste aussi (`direct.rs`, depuis la prise du port 7748) ; il ne
//! reste qu'à se dire ces adresses. Ce que d'habitude un serveur de
//! signalisation fait passer passe ici par les **trackers WebTorrent
//! publics** — des relais de signalisation WebRTC tenus par d'autres,
//! qui ne gardent rien : la page annonce dans un « essaim » nommé d'après
//! la boîte de l'officine (`swarm`) une offre WebRTC ; le tracker la
//! remet aux postes qui attendent dans cet essaim ; le poste répond, le
//! tracker rend la réponse. Puis les deux s'envoient leurs paquets en même
//! temps, chacun ouvrant sa box à l'autre (« hole punching »), et le canal
//! est direct.
//!
//! Le tracker voit des adresses IP et l'heure, jamais un message : une
//! offre ne porte que des adresses, des identifiants ICE et l'empreinte
//! DTLS. La page vérifie que la réponse porte l'empreinte du code QR
//! (`f`) — un tracker qui répondrait à la place du poste ne passe pas
//! DTLS — et les messages restent chiffrés de bout en bout par
//! `patientchat`. Deux box qui changent de port à chaque destinataire
//! (NAT symétrique, certains réseaux d'entreprise) ne se joignent pas
//! ainsi : la voie directe par adresse, un relais ou une passerelle
//! restent.
//!
//! Ce module tient le client des trackers (WebSocket sur TLS, sans
//! bibliothèque WebSocket) ; `direct.rs` tient les conversations.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

use base64::Engine;
use sha2::{Digest, Sha256};

/// Les trackers WebTorrent publics où l'on se donne rendez-vous ; la
/// page essaie les mêmes (`assets/patient/app.js`, `TRACKERS`).
pub const TRACKERS: [&str; 2] = [
    "wss://tracker.openwebtorrent.com",
    "wss://tracker.webtorrent.dev",
];

/// Les serveurs STUN publics qui disent à chacun l'adresse que sa box
/// lui donne ; la page a les mêmes (`STUN`).
pub const STUN: [&str; 2] = ["stun.l.google.com:19302", "stun.cloudflare.com:3478"];

/// Combien d'attente pour la réponse d'un poste à une offre.
const ANSWER_BOUND: Duration = Duration::from_secs(5);
/// L'annonce redite au moins aussi souvent : le tracker oublie un pair
/// qui se tait.
const ANNOUNCE_EVERY: Duration = Duration::from_secs(60);
/// Après une coupure, l'attente avant de rappeler un tracker, doublée à
/// chaque échec jusqu'à ce plafond.
const RETRY_MOST: Duration = Duration::from_secs(300);
/// Un message plus grand qu'une offre ne le sera jamais : refusé.
const MESSAGE_MOST: usize = 64 * 1024;

/// **L'essaim de l'officine** : vingt chiffres hexadécimaux tirés de sa
/// boîte (`patientchat::box_id`) — la page le calcule de la même façon.
/// Il ne cache rien : qui a le numéro de la boîte l'a.
pub fn swarm(box_id: &str) -> String {
    let mut h = Sha256::new();
    h.update(b"bpm-caddy rendez-vous 1|");
    h.update(box_id.as_bytes());
    h.finalize()[..10]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// **Une offre arrivée par un tracker** : la description de session de
/// la page, et où rendre celle du poste.
pub struct Offered {
    pub sdp: String,
    pub answer: mpsc::Sender<String>,
}

/// Où remettre les offres : la voie directe en cours, remplacée quand
/// elle se relance (`patientpost::listen_direct`).
pub type OfferSlot = Arc<Mutex<Option<mpsc::Sender<Offered>>>>;

/// Vingt caractères tirés au hasard : un identifiant de pair ou d'offre.
pub(crate) fn draw_id() -> String {
    use std::hash::{BuildHasher, Hasher};
    let mut out = String::new();
    while out.len() < 20 {
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u128(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos()),
        );
        out.push_str(&format!("{:016x}", h.finish()));
    }
    out.truncate(20);
    out
}

/// L'annonce d'un poste qui attend dans l'essaim : aucune offre à lui,
/// celles des pages lui seront remises.
pub fn announce(swarm: &str, peer: &str, first: bool) -> String {
    let mut m = serde_json::json!({
        "action": "announce",
        "info_hash": swarm,
        "peer_id": peer,
        "numwant": 0,
        "uploaded": 0,
        "downloaded": 0,
        "left": 0,
        "offers": [],
    });
    if first {
        m["event"] = "started".into();
    }
    m.to_string()
}

/// La réponse du poste à l'offre `offer_id` de la page `to`.
pub fn answer(swarm: &str, peer: &str, to: &str, offer_id: &str, sdp: &str) -> String {
    serde_json::json!({
        "action": "announce",
        "info_hash": swarm,
        "peer_id": peer,
        "to_peer_id": to,
        "offer_id": offer_id,
        "answer": { "type": "answer", "sdp": sdp },
    })
    .to_string()
}

/// **Ce que dit un tracker**, quand c'est une offre d'une page pour cet
/// essaim : qui, quelle offre, et sa description.
#[derive(Debug, PartialEq, Eq)]
pub struct Heard {
    pub from: String,
    pub offer_id: String,
    pub sdp: String,
}

/// Une offre, ou rien (une réponse à notre annonce, un avertissement, un
/// autre essaim, une offre qui n'est pas une offre).
pub fn heard(text: &str, swarm: &str) -> Option<Heard> {
    let m: serde_json::Value = serde_json::from_str(text).ok()?;
    if m.get("info_hash")?.as_str()? != swarm {
        return None;
    }
    let offer = m.get("offer")?;
    if offer.get("type")?.as_str()? != "offer" {
        return None;
    }
    let field = |v: &serde_json::Value, k: &str| v.get(k)?.as_str().map(str::to_owned);
    Some(Heard {
        from: field(&m, "peer_id")?,
        offer_id: field(&m, "offer_id")?,
        sdp: field(offer, "sdp")?,
    })
}

/// L'intervalle que le tracker demande entre deux annonces, s'il en dit
/// un — borné : ni plus souvent que toutes les vingt secondes, ni moins
/// que [`ANNOUNCE_EVERY`].
fn interval_of(text: &str) -> Option<Duration> {
    let m: serde_json::Value = serde_json::from_str(text).ok()?;
    let s = m.get("interval")?.as_u64()?;
    Some(Duration::from_secs(s.clamp(20, ANNOUNCE_EVERY.as_secs())))
}

// --- WebSocket (RFC 6455), côté client, sur TLS ---------------------

/// Une trame à envoyer : texte (`0x1`), pong (`0xA`) ou fermeture
/// (`0x8`), masquée comme tout ce qu'un client envoie.
pub fn frame(opcode: u8, payload: &[u8], mask: [u8; 4]) -> Vec<u8> {
    let mut out = vec![0x80 | opcode];
    match payload.len() {
        n if n < 126 => out.push(0x80 | n as u8),
        n if n <= 0xffff => {
            out.push(0x80 | 126);
            out.extend_from_slice(&(n as u16).to_be_bytes());
        }
        n => {
            out.push(0x80 | 127);
            out.extend_from_slice(&(n as u64).to_be_bytes());
        }
    }
    out.extend_from_slice(&mask);
    out.extend(payload.iter().enumerate().map(|(i, b)| b ^ mask[i % 4]));
    out
}

/// **Une trame reçue**, prise en tête de `buf` quand elle y est entière :
/// son code, son contenu, et combien d'octets elle occupait. `Err` : un
/// flux qui n'est pas du WebSocket, ou trop grand.
fn unframe(buf: &[u8]) -> Result<Option<(u8, Vec<u8>, usize)>, ()> {
    if buf.len() < 2 {
        return Ok(None);
    }
    let opcode = buf[0] & 0x0f;
    let masked = buf[1] & 0x80 != 0;
    let (len, mut at) = match buf[1] & 0x7f {
        126 if buf.len() >= 4 => (u16::from_be_bytes([buf[2], buf[3]]) as usize, 4),
        127 if buf.len() >= 10 => {
            let mut b = [0u8; 8];
            b.copy_from_slice(&buf[2..10]);
            (usize::try_from(u64::from_be_bytes(b)).map_err(|_| ())?, 10)
        }
        126 | 127 => return Ok(None),
        n => (n as usize, 2),
    };
    if len > MESSAGE_MOST {
        return Err(());
    }
    let mask = if masked {
        let Some(m) = buf.get(at..at + 4) else {
            return Ok(None);
        };
        at += 4;
        Some([m[0], m[1], m[2], m[3]])
    } else {
        None
    };
    let Some(body) = buf.get(at..at + len) else {
        return Ok(None);
    };
    let payload = match mask {
        Some(m) => body.iter().enumerate().map(|(i, b)| b ^ m[i % 4]).collect(),
        None => body.to_vec(),
    };
    Ok(Some((opcode, payload, at + len)))
}

type Tls = rustls::StreamOwned<rustls::ClientConnection, TcpStream>;

struct Ws {
    tls: Tls,
    buf: Vec<u8>,
    /// Un message texte en plusieurs trames, en cours.
    partial: Vec<u8>,
}

fn tls_config() -> Option<Arc<rustls::ClientConfig>> {
    static CONFIG: std::sync::OnceLock<Option<Arc<rustls::ClientConfig>>> =
        std::sync::OnceLock::new();
    CONFIG
        .get_or_init(|| {
            let roots = rustls::RootCertStore {
                roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
            };
            let config = rustls::ClientConfig::builder_with_provider(Arc::new(
                rustls::crypto::ring::default_provider(),
            ))
            .with_safe_default_protocol_versions()
            .ok()?
            .with_root_certificates(roots)
            .with_no_client_auth();
            Some(Arc::new(config))
        })
        .clone()
}

fn mask_now() -> [u8; 4] {
    let id = draw_id();
    let b = id.as_bytes();
    [b[0], b[3], b[7], b[11]]
}

impl Ws {
    /// Ouvrir `url` (`wss://hôte[:port][/chemin]`).
    fn connect(url: &str) -> Result<Self, String> {
        let rest = url.strip_prefix("wss://").ok_or("wss seulement")?;
        let (host_port, path) = rest.split_once('/').unwrap_or((rest, ""));
        let (host, port) = match host_port.rsplit_once(':') {
            Some((h, p)) => (h, p.parse::<u16>().map_err(|e| e.to_string())?),
            None => (host_port, 443),
        };
        let config = tls_config().ok_or("TLS indisponible")?;
        // Through the web proxy when there is one (`proxy.rs`): on a
        // network that only lets the web out, the trackers are still met.
        let tcp =
            crate::proxy::dial(host, port, Duration::from_secs(10)).map_err(|e| e.to_string())?;
        tcp.set_read_timeout(Some(Duration::from_secs(10)))
            .map_err(|e| e.to_string())?;
        let name =
            rustls::pki_types::ServerName::try_from(host.to_owned()).map_err(|e| e.to_string())?;
        let conn = rustls::ClientConnection::new(config, name).map_err(|e| e.to_string())?;
        let mut tls = rustls::StreamOwned::new(conn, tcp);
        let key = base64::engine::general_purpose::STANDARD.encode(&draw_id().as_bytes()[..16]);
        let request = format!(
            "GET /{path} HTTP/1.1\r\nHost: {host}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\
             Sec-WebSocket-Key: {key}\r\nSec-WebSocket-Version: 13\r\n\r\n"
        );
        tls.write_all(request.as_bytes())
            .map_err(|e| e.to_string())?;
        // The answer's head, up to the blank line; what follows it is
        // already WebSocket.
        let mut head = Vec::new();
        let mut byte = [0u8; 1];
        while !head.ends_with(b"\r\n\r\n") {
            if head.len() > 8192 {
                return Err("réponse trop longue".into());
            }
            tls.read_exact(&mut byte).map_err(|e| e.to_string())?;
            head.push(byte[0]);
        }
        let status = String::from_utf8_lossy(&head);
        if !status.starts_with("HTTP/1.1 101") {
            return Err(status.lines().next().unwrap_or("").to_owned());
        }
        tls.sock
            .set_read_timeout(Some(Duration::from_millis(500)))
            .map_err(|e| e.to_string())?;
        Ok(Self {
            tls,
            buf: Vec::new(),
            partial: Vec::new(),
        })
    }

    fn send(&mut self, opcode: u8, payload: &[u8]) -> Result<(), String> {
        self.tls
            .write_all(&frame(opcode, payload, mask_now()))
            .and_then(|()| self.tls.flush())
            .map_err(|e| e.to_string())
    }

    /// Le prochain message texte, s'il en arrive un dans la demi-seconde ;
    /// `Err` quand la connexion est tombée.
    fn next_text(&mut self) -> Result<Option<String>, String> {
        loop {
            match unframe(&self.buf) {
                Err(()) => return Err("trame".into()),
                Ok(Some((opcode, payload, used))) => {
                    let fin = self.buf[0] & 0x80 != 0;
                    self.buf.drain(..used);
                    match opcode {
                        0x1 | 0x0 => {
                            self.partial.extend_from_slice(&payload);
                            if self.partial.len() > MESSAGE_MOST {
                                return Err("message".into());
                            }
                            if fin {
                                let text = String::from_utf8_lossy(&self.partial).into_owned();
                                self.partial.clear();
                                return Ok(Some(text));
                            }
                        }
                        0x9 => self.send(0xA, &payload)?,
                        0x8 => return Err("fermée".into()),
                        _ => {}
                    }
                    continue;
                }
                Ok(None) => {}
            }
            let mut chunk = [0u8; 4096];
            match self.tls.read(&mut chunk) {
                Ok(0) => return Err("fermée".into()),
                Ok(n) => self.buf.extend_from_slice(&chunk[..n]),
                Err(e)
                    if matches!(
                        e.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) =>
                {
                    return Ok(None)
                }
                Err(e) => return Err(e.to_string()),
            }
        }
    }
}

/// **Attendre dans l'essaim de la boîte**, à chaque tracker, chacun sur
/// son fil, tant que `stop` ne le dit pas : chaque offre part vers la voie
/// directe en cours (`slot`), sa réponse revient au tracker. `up` compte
/// les trackers où le poste attend en ce moment.
pub fn spawn(box_id: &str, slot: OfferSlot, stop: Arc<AtomicBool>, up: Arc<AtomicUsize>) {
    spawn_swarm(swarm(box_id), slot, stop, up);
}

/// **Attendre dans un essaim** donné : celui d'une boîte ([`spawn`]), ou
/// celui d'un groupe de postes (`punch.rs`).
pub fn spawn_swarm(swarm: String, slot: OfferSlot, stop: Arc<AtomicBool>, up: Arc<AtomicUsize>) {
    for url in TRACKERS {
        let (swarm, slot, stop, up) = (
            swarm.clone(),
            Arc::clone(&slot),
            Arc::clone(&stop),
            Arc::clone(&up),
        );
        std::thread::spawn(move || {
            let mut retry = Duration::from_secs(5);
            while !stop.load(Ordering::SeqCst) {
                if let Ok(ws) = Ws::connect(url) {
                    up.fetch_add(1, Ordering::SeqCst);
                    let lasted = Instant::now();
                    wait_in(ws, &swarm, &slot, &stop);
                    up.fetch_sub(1, Ordering::SeqCst);
                    if lasted.elapsed() > Duration::from_secs(60) {
                        retry = Duration::from_secs(5);
                    }
                }
                let until = Instant::now() + retry;
                while Instant::now() < until && !stop.load(Ordering::SeqCst) {
                    std::thread::sleep(Duration::from_millis(500));
                }
                retry = (retry * 2).min(RETRY_MOST);
            }
        });
    }
}

/// Une connexion à un tracker, jusqu'à ce qu'elle tombe.
fn wait_in(mut ws: Ws, swarm: &str, slot: &OfferSlot, stop: &AtomicBool) {
    let me = draw_id();
    if ws.send(0x1, announce(swarm, &me, true).as_bytes()).is_err() {
        return;
    }
    let mut every = ANNOUNCE_EVERY;
    let mut said = Instant::now();
    while !stop.load(Ordering::SeqCst) {
        if said.elapsed() >= every {
            if ws
                .send(0x1, announce(swarm, &me, false).as_bytes())
                .is_err()
            {
                return;
            }
            said = Instant::now();
        }
        let text = match ws.next_text() {
            Ok(Some(t)) => t,
            Ok(None) => continue,
            Err(_) => return,
        };
        if let Some(i) = interval_of(&text) {
            every = i;
        }
        let Some(h) = heard(&text, swarm) else {
            continue;
        };
        let Some(to_direct) = slot.lock().ok().and_then(|s| s.clone()) else {
            continue;
        };
        let (tx, rx) = mpsc::channel();
        if to_direct
            .send(Offered {
                sdp: h.sdp,
                answer: tx,
            })
            .is_err()
        {
            continue;
        }
        if let Ok(sdp) = rx.recv_timeout(ANSWER_BOUND) {
            let reply = answer(swarm, &me, &h.from, &h.offer_id, &sdp);
            if ws.send(0x1, reply.as_bytes()).is_err() {
                return;
            }
        }
    }
    let _ = ws.send(0x8, &[]);
}

/// L'annonce d'une offre dans l'essaim : ce que la page publie, et un
/// téléphone qui cherche son poste (`punch::dial`).
pub fn offer_announce(swarm: &str, peer: &str, offer_id: &str, sdp: &str) -> String {
    serde_json::json!({
        "action": "announce", "info_hash": swarm, "peer_id": peer, "numwant": 1,
        "uploaded": 0, "downloaded": 0, "left": 1, "event": "started",
        "offers": [{ "offer_id": offer_id, "offer": { "type": "offer", "sdp": sdp } }],
    })
    .to_string()
}

/// La réponse à l'offre `offer_id`, quand c'est elle que dit le tracker.
pub fn answered(text: &str, swarm: &str, offer_id: &str) -> Option<String> {
    let m: serde_json::Value = serde_json::from_str(text).ok()?;
    if m.get("info_hash")?.as_str()? != swarm || m.get("offer_id")?.as_str()? != offer_id {
        return None;
    }
    let a = m.get("answer")?;
    if a.get("type")?.as_str()? != "answer" {
        return None;
    }
    a.get("sdp")?.as_str().map(str::to_owned)
}

/// Combien de postes attendent dans l'essaim, d'après la réponse du
/// tracker à une annonce (`complete` : ceux qui n'attendent rien d'autre).
fn waiting_in(text: &str, swarm: &str) -> Option<u64> {
    let m: serde_json::Value = serde_json::from_str(text).ok()?;
    if m.get("info_hash")?.as_str()? != swarm || m.get("offer_id").is_some() {
        return None;
    }
    m.get("complete")?.as_u64()
}

/// **Publier une offre et attendre sa réponse**, à tous les trackers à la
/// fois : la première qui revient, dans `bound`. `None` : aucun tracker,
/// ou personne n'attend dans l'essaim — dit aussitôt quand chaque tracker
/// répond que l'essaim est vide, sans attendre `bound`.
pub fn offer(swarm: &str, sdp: &str, bound: Duration) -> Option<String> {
    // Each tracker says once: its answer, or `None` (nobody there, gone,
    // or out of time).
    let (tx, rx) = mpsc::channel::<Option<String>>();
    let done = Arc::new(AtomicBool::new(false));
    let until = Instant::now() + bound;
    for url in TRACKERS {
        let (tx, done, swarm, sdp) = (
            tx.clone(),
            Arc::clone(&done),
            swarm.to_owned(),
            sdp.to_owned(),
        );
        std::thread::spawn(move || {
            let said = (|| {
                let mut ws = Ws::connect(url).ok()?;
                let (me, offer_id) = (draw_id(), draw_id());
                ws.send(0x1, offer_announce(&swarm, &me, &offer_id, &sdp).as_bytes())
                    .ok()?;
                let mut got = None;
                while Instant::now() < until && !done.load(Ordering::SeqCst) {
                    match ws.next_text() {
                        Ok(Some(t)) => {
                            if waiting_in(&t, &swarm) == Some(0) {
                                break;
                            }
                            if let Some(a) = answered(&t, &swarm, &offer_id) {
                                got = Some(a);
                                break;
                            }
                        }
                        Ok(None) => {}
                        Err(_) => break,
                    }
                }
                let _ = ws.send(0x8, &[]);
                got
            })();
            let _ = tx.send(said);
        });
    }
    drop(tx);
    let mut got = None;
    for _ in TRACKERS {
        match rx.recv_timeout(until.saturating_duration_since(Instant::now())) {
            Ok(Some(a)) => {
                got = Some(a);
                break;
            }
            Ok(None) => {}
            Err(_) => break,
        }
    }
    done.store(true, Ordering::SeqCst);
    got
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **L'essaim, des deux côtés** : la page le tire de la boîte comme
    /// le poste — la valeur ici est celle que `app.js` calcule.
    #[test]
    fn the_swarm_is_drawn_from_the_box_as_the_page_draws_it() {
        let s = swarm("0123456789abcdef0123456789abcdef");
        assert_eq!(s.len(), 20);
        assert!(s.bytes().all(|b| b.is_ascii_hexdigit()));
        assert_ne!(s, swarm("0123456789abcdef0123456789abcdee"));
        let page = include_str!("../assets/patient/app.js");
        assert!(page.contains("bpm-caddy rendez-vous 1|"));
        for t in TRACKERS {
            assert!(page.contains(t), "{t} manque à la page");
        }
        for s in STUN {
            assert!(page.contains(&format!("stun:{s}")), "{s} manque à la page");
        }
    }

    /// **Une trame aller et retour**, aux trois tailles de longueur ; une
    /// trame coupée attend la suite ; une trame démesurée est refusée.
    #[test]
    fn a_websocket_frame_goes_and_comes_back() {
        for n in [5usize, 300, 70_000.min(MESSAGE_MOST)] {
            let body: Vec<u8> = (0..n).map(|i| (i % 251) as u8).collect();
            let f = frame(0x1, &body, [1, 2, 3, 4]);
            assert_eq!(unframe(&f[..f.len() - 1]), Ok(None));
            let (op, got, used) = unframe(&f).unwrap().unwrap();
            assert_eq!((op, used), (0x1, f.len()));
            assert_eq!(got, body);
        }
        // Unmasked, as a server sends.
        assert_eq!(
            unframe(&[0x81, 2, b'o', b'k']),
            Ok(Some((0x1, b"ok".to_vec(), 4)))
        );
        let mut huge = vec![0x81, 127];
        huge.extend_from_slice(&(u64::MAX).to_be_bytes());
        assert_eq!(unframe(&huge), Err(()));
    }

    /// **Ce que dit un tracker** : l'offre d'une page dans notre essaim,
    /// et rien d'autre.
    #[test]
    fn only_an_offer_in_our_swarm_is_heard() {
        let s = swarm("0123456789abcdef0123456789abcdef");
        let offer = serde_json::json!({
            "action": "announce", "info_hash": s, "peer_id": "pagepagepagepagepage",
            "offer_id": "offeroffer0000000001",
            "offer": { "type": "offer", "sdp": "v=0\r\n" },
        })
        .to_string();
        assert_eq!(
            heard(&offer, &s),
            Some(Heard {
                from: "pagepagepagepagepage".into(),
                offer_id: "offeroffer0000000001".into(),
                sdp: "v=0\r\n".into(),
            })
        );
        assert_eq!(heard(&offer, &swarm("autre")), None);
        assert_eq!(heard(&offer.replace("\"offer\"", "\"answer\""), &s), None);
        assert_eq!(heard(r#"{"action":"announce","interval":120}"#, &s), None);
        assert_eq!(heard("pas du json", &s), None);
        assert_eq!(
            interval_of(r#"{"action":"announce","interval":120}"#),
            Some(ANNOUNCE_EVERY)
        );
        assert_eq!(
            interval_of(r#"{"interval":2}"#),
            Some(Duration::from_secs(20))
        );
        let a: serde_json::Value =
            serde_json::from_str(&answer(&s, "me", "them", "o1", "v=0")).unwrap();
        assert_eq!(a["to_peer_id"], "them");
        assert_eq!(a["answer"]["type"], "answer");
        assert_eq!(
            waiting_in(
                r#"{"action":"announce","interval":120,"info_hash":"S","complete":0,"incomplete":1}"#,
                "S"
            ),
            Some(0)
        );
        assert_eq!(waiting_in(r#"{"info_hash":"S","complete":2}"#, "T"), None);
        assert_eq!(waiting_in(&offer, &s), None);
        let n: serde_json::Value = serde_json::from_str(&announce(&s, "me", true)).unwrap();
        assert_eq!(n["event"], "started");
        assert_eq!(n["offers"].as_array().map(Vec::len), Some(0));
    }

    /// **Une page rejoint un poste par un vrai tracker** : le poste
    /// attend dans l'essaim, la page y publie son offre, la réponse revient
    /// et le canal s'ouvre (sur la boucle locale : c'est le tracker qu'on
    /// essaie ici, pas la box). Demande le réseau : `--ignored`.
    #[test]
    #[ignore]
    fn a_page_meets_a_post_through_a_public_tracker() {
        use str0m::net::{Protocol, Receive};
        use str0m::{Candidate, Event, Input, Output, RtcConfig};
        let box_id = "0123456789abcdef0123456789abcdef";
        let id = crate::direct::Identity::draw().unwrap();
        let (tx, links) = mpsc::channel();
        let listening = crate::direct::serve(
            crate::direct::bind_all(&["127.0.0.1".parse().unwrap()], 0),
            &id,
            box_id,
            tx,
        );
        let slot: OfferSlot = Arc::new(Mutex::new(Some(listening.offers.clone())));
        let (stop, up) = (
            Arc::new(AtomicBool::new(false)),
            Arc::new(AtomicUsize::new(0)),
        );
        spawn(box_id, slot, Arc::clone(&stop), Arc::clone(&up));
        let until = Instant::now() + Duration::from_secs(15);
        while up.load(Ordering::SeqCst) == 0 && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(100));
        }
        assert!(up.load(Ordering::SeqCst) > 0, "aucun tracker");
        std::thread::sleep(Duration::from_secs(1));

        let socket = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
        socket.set_nonblocking(true).unwrap();
        let local = socket.local_addr().unwrap();
        let mut rtc = RtcConfig::new().build(Instant::now());
        rtc.add_local_candidate(Candidate::host(local, "udp").unwrap());
        let mut api = rtc.sdp_api();
        let cid = api.add_channel("bpm".into());
        let (offer, pending) = api.apply().unwrap();
        let mut ws = Ws::connect(TRACKERS[0]).unwrap();
        let (me, offer_id) = (draw_id(), draw_id());
        let sw = swarm(box_id);
        ws.send(
            0x1,
            serde_json::json!({
                "action": "announce", "info_hash": sw, "peer_id": me, "numwant": 1,
                "uploaded": 0, "downloaded": 0, "left": 1, "event": "started",
                "offers": [{ "offer_id": offer_id,
                             "offer": { "type": "offer", "sdp": offer.to_sdp_string() } }],
            })
            .to_string()
            .as_bytes(),
        )
        .unwrap();
        let until = Instant::now() + Duration::from_secs(15);
        let answer = loop {
            assert!(Instant::now() < until, "aucune réponse par le tracker");
            let Some(t) = ws.next_text().unwrap() else {
                continue;
            };
            let m: serde_json::Value = serde_json::from_str(&t).unwrap();
            if m["offer_id"] == offer_id.as_str() && m["answer"]["type"] == "answer" {
                break m["answer"]["sdp"].as_str().unwrap().to_owned();
            }
        };
        rtc.sdp_api()
            .accept_answer(
                pending,
                str0m::change::SdpAnswer::from_sdp_string(&answer).unwrap(),
            )
            .unwrap();
        let mut buf = vec![0u8; 2048];
        let start = Instant::now();
        let mut open = false;
        while start.elapsed() < Duration::from_secs(10) && !open {
            let now = Instant::now();
            while let Ok((n, source)) = socket.recv_from(&mut buf) {
                let _ = rtc.handle_input(Input::Receive(
                    now,
                    Receive {
                        proto: Protocol::Udp,
                        source,
                        destination: local,
                        contents: buf[..n].try_into().unwrap(),
                    },
                ));
            }
            let _ = rtc.handle_input(Input::Timeout(now));
            loop {
                match rtc.poll_output().unwrap() {
                    Output::Transmit(t) => {
                        let _ = socket.send_to(&t.contents, t.destination);
                    }
                    Output::Event(Event::ChannelOpen(c, _)) if c == cid => open = true,
                    Output::Event(_) => {}
                    Output::Timeout(_) => break,
                }
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        stop.store(true, Ordering::SeqCst);
        assert!(open, "le canal ne s'ouvre pas");
        assert!(links.recv_timeout(Duration::from_secs(5)).is_ok());
    }

    /// **Un vrai tracker** : on s'y annonce et il répond. Demande le
    /// réseau : `--ignored`.
    #[test]
    #[ignore]
    fn a_public_tracker_takes_the_announce() {
        let mut ws = Ws::connect(TRACKERS[0]).expect("tracker");
        let s = swarm("0123456789abcdef0123456789abcdef");
        ws.send(0x1, announce(&s, &draw_id(), true).as_bytes())
            .unwrap();
        let until = Instant::now() + Duration::from_secs(10);
        while Instant::now() < until {
            if let Some(t) = ws.next_text().unwrap() {
                assert!(t.contains("interval") || t.contains("info_hash"), "{t}");
                return;
            }
        }
        panic!("aucune réponse du tracker");
    }
}
