//! **La voie directe** : la page du patient joint un poste de l'officine
//! sans passerelle — un canal de données WebRTC, de son navigateur à ce
//! poste, et rien entre les deux qu'Internet.
//!
//! Un navigateur ne parle pas TCP, et depuis une page en HTTPS il
//! n'ouvre de WebSocket que vers un nom qui a un certificat : d'où la
//! passerelle (`passerelle.rs`). WebRTC, lui, joint une simple adresse,
//! sans nom ni autorité de certification, à une condition : que chaque
//! bout ait la description de l'autre. D'ordinaire, un serveur de
//! signalisation les fait passer. Ici, **le code QR est la
//! signalisation** : il porte l'empreinte du certificat DTLS de
//! l'officine et les adresses de ses postes ; les identifiants ICE se
//! tirent du numéro de sa boîte. La page écrit elle-même la réponse du
//! poste ; le poste, en ICE-lite, ne fait que répondre au premier paquet
//! STUN d'une page qui connaît ces identifiants, et apprend d'elle tout
//! ce qu'il lui faut.
//!
//! Ce que le canal porte est ce que portait la passerelle : les trames de
//! `patientchat` (une longueur sur quatre octets, puis la trame), comme un
//! flux d'octets découpé en messages de seize kilo-octets au plus — la
//! taille qu'un navigateur accepte toujours. Le chiffrement de bout en
//! bout est celui de `patientchat` : DTLS en dessous protège en plus
//! l'officine d'une page qui parlerait à un autre poste que le sien
//! (l'empreinte du code QR), sans être ce qui garde les messages.
//!
//! Tous les postes de l'officine ont le même certificat (un réglage de
//! l'officine, dans la base chiffrée) : une page essaie toutes leurs
//! adresses dans une seule connexion, et garde la première qui répond.
//!
//! Ce module ne touche ni base ni écran : les conversations sortent
//! d'ici comme des [`Link`], qu'un autre fil fait parler.

use std::collections::{HashMap, VecDeque};
use std::io::{Read, Write};
use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

use base64::Engine;
use sha2::{Digest, Sha256};
use str0m::change::DirectApi;
use str0m::channel::ChannelId;
use str0m::crypto::Fingerprint;
use str0m::net::{Protocol, Receive};
use str0m::{Candidate, Event, IceCreds, Input, Output, Rtc, RtcConfig};

/// Le port proposé : à côté de ceux des postes (7743), du relais (7746)
/// et de la passerelle (7747).
pub const DEFAULT_PORT: u16 = 7748;
/// Le plus grand message du canal : ce que tout navigateur reçoit.
pub const CHUNK: usize = 16 * 1024;
/// Combien de pages à la fois, en tout…
const MOST_PEERS: usize = 64;
/// …et par abonné (une IPv4, ou le `/64` d'une IPv6 — un abonné en tient
/// des milliards) : un téléphone, ou quelques-uns derrière une même box.
const PEERS_PER_CALLER: usize = 8;
/// Les pages dont le canal n'est pas encore ouvert, en tout et par
/// abonné : des paquets STUN fabriqués ne prennent pas toutes les places.
const MOST_OPENING: usize = 16;
const OPENING_PER_CALLER: usize = 4;
/// Ce qu'une page peut avoir envoyé que le poste n'a pas encore lu : au
/// delà, elle est lâchée — un flot ne remplit pas la mémoire du poste.
const HELD_MOST: usize = 2 * crate::patientchat::FRAME_MAX + CHUNK;
/// **Une conversation entière**, d'une page, de son bonjour à sa dernière
/// trame : une page qui se tait ou parle goutte à goutte ne tient pas le
/// fil de la base plus longtemps.
pub const CONVERSATION_BOUND: Duration = Duration::from_secs(20);
/// Une page qui n'a pas ouvert son canal dans ce délai est lâchée.
const OPEN_BOUND: Duration = Duration::from_secs(20);
/// Une conversation dure au plus…
const TALK_BOUND: Duration = Duration::from_secs(90);
/// …et ce qu'il reste à envoyer après sa fin, au plus ceci.
const DRAIN_BOUND: Duration = Duration::from_secs(10);

/// **L'identité de l'officine** sur la voie directe : un certificat DTLS
/// et sa clé privée, tirés une fois par le poste de référence, rangés
/// dans les réglages de l'officine — et donc les mêmes sur tous ses
/// postes.
#[derive(Clone, PartialEq, Eq)]
pub struct Identity {
    cert: Vec<u8>,
    key: Vec<u8>,
}

impl std::fmt::Debug for Identity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Jamais la clé privée, pas même dans un journal.
        write!(f, "Identity({})", fingerprint_text(&self.fingerprint()))
    }
}

fn b64() -> base64::engine::GeneralPurpose {
    base64::engine::general_purpose::URL_SAFE_NO_PAD
}

impl Identity {
    /// Tirer une identité neuve.
    pub fn draw() -> Option<Self> {
        let provider = str0m::crypto::from_feature_flags();
        let cert = provider.dtls_provider.generate_certificate()?;
        Some(Self {
            cert: cert.certificate,
            key: cert.private_key,
        })
    }

    /// Telle qu'on la range : le certificat et la clé, en base64, séparés
    /// d'un point.
    pub fn text(&self) -> String {
        format!("{}.{}", b64().encode(&self.cert), b64().encode(&self.key))
    }

    pub fn read(text: &str) -> Option<Self> {
        let (c, k) = text.trim().split_once('.')?;
        let (cert, key) = (b64().decode(c).ok()?, b64().decode(k).ok()?);
        (!cert.is_empty() && !key.is_empty()).then_some(Self { cert, key })
    }

    /// L'empreinte du certificat (SHA-256 de sa forme DER) : ce que la
    /// page exige du poste, et ce que le code QR porte.
    pub fn fingerprint(&self) -> [u8; 32] {
        Sha256::digest(&self.cert).into()
    }

    fn dtls(&self) -> str0m::config::DtlsCert {
        str0m::config::DtlsCert {
            certificate: self.cert.clone(),
            private_key: self.key.clone(),
        }
    }
}

/// L'empreinte comme le lien la porte : base64 sans remplissage.
pub fn fingerprint_text(fp: &[u8; 32]) -> String {
    b64().encode(fp)
}

/// **Les identifiants ICE de l'officine**, tirés du numéro de sa boîte
/// (`patientchat::box_id`, 32 chiffres hexadécimaux) : la page les
/// connaît par le lien, sans qu'il les porte. Ils ne gardent rien — qui a
/// le numéro les a — : ils écartent seulement les paquets d'autres
/// conversations WebRTC. Ce qui protège, c'est l'empreinte et la clé du
/// patient.
pub fn ice_creds(box_id: &str) -> IceCreds {
    IceCreds {
        ufrag: box_id.chars().take(8).collect(),
        pass: box_id.to_owned(),
    }
}

/// Les adresses où écouter : celles de cette machine qu'on peut joindre
/// depuis Internet, au port donné — ses adresses IPv6 publiques, et son
/// adresse IPv4 côté routeur, que le routeur traduit (UPnP).
pub fn bind_all(addresses: &[IpAddr], port: u16) -> Vec<UdpSocket> {
    addresses
        .iter()
        .filter_map(|ip| UdpSocket::bind(SocketAddr::new(*ip, port)).ok())
        .filter(|s| s.set_nonblocking(true).is_ok())
        .collect()
}

/// **Une conversation ouverte** par une page, du côté du poste : un flux
/// d'octets, lu et écrit comme une connexion — `patientchat::Frames` le
/// prend tel quel.
pub struct Link {
    peer: u64,
    /// L'adresse de la page.
    pub from: SocketAddr,
    inbox: mpsc::Receiver<Vec<u8>>,
    pending: VecDeque<u8>,
    commands: mpsc::Sender<Command>,
    /// Ce que la page a envoyé et qui n'est pas encore lu.
    held: Arc<AtomicUsize>,
    /// Passé ceci, plus rien ne se lit : [`CONVERSATION_BOUND`] depuis
    /// l'ouverture du canal.
    deadline: Instant,
}

impl Read for Link {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.pending.is_empty() {
            let left = self.deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Err(std::io::ErrorKind::TimedOut.into());
            }
            match self.inbox.recv_timeout(left) {
                Ok(chunk) => {
                    self.held.fetch_sub(
                        chunk.len().min(self.held.load(Ordering::SeqCst)),
                        Ordering::SeqCst,
                    );
                    self.pending.extend(chunk)
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    return Err(std::io::ErrorKind::TimedOut.into())
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(0),
            }
        }
        let n = buf.len().min(self.pending.len());
        for (b, v) in buf.iter_mut().zip(self.pending.drain(..n)) {
            *b = v;
        }
        Ok(n)
    }
}

impl Write for Link {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.commands
            .send(Command::Send(self.peer, buf.to_vec()))
            .map_err(|_| std::io::Error::from(std::io::ErrorKind::BrokenPipe))?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Drop for Link {
    fn drop(&mut self) {
        let _ = self.commands.send(Command::Close(self.peer));
    }
}

enum Command {
    Send(u64, Vec<u8>),
    Close(u64),
}

/// Une page, du côté du poste.
struct Peer {
    rtc: Rtc,
    /// La prise par laquelle elle est arrivée : celle des réponses dont la
    /// source n'est aucune prise.
    socket: usize,
    from: SocketAddr,
    held: Arc<AtomicUsize>,
    born: Instant,
    channel: Option<ChannelId>,
    to_link: Option<mpsc::Sender<Vec<u8>>>,
    outbox: VecDeque<Vec<u8>>,
    closing: Option<Instant>,
}

/// Le nom de l'utilisateur d'un paquet STUN (RFC 5389) : `ours:theirs`
/// dans une demande de liaison ICE. `None` pour tout autre paquet.
pub(crate) fn stun_username(buf: &[u8]) -> Option<&str> {
    // Binding request (0x0001), magic cookie.
    if buf.len() < 20 || buf[0..2] != [0x00, 0x01] || buf[4..8] != [0x21, 0x12, 0xa4, 0x42] {
        return None;
    }
    let mut i = 20;
    while i + 4 <= buf.len() {
        let kind = u16::from_be_bytes([buf[i], buf[i + 1]]);
        let len = u16::from_be_bytes([buf[i + 2], buf[i + 3]]) as usize;
        let body = buf.get(i + 4..i + 4 + len)?;
        if kind == 0x0006 {
            return std::str::from_utf8(body).ok();
        }
        i += 4 + ((len + 3) & !3);
    }
    None
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

/// Ce qui écoute : l'arrêter ferme les prises et lâche les pages.
/// Arrêtée, les prises sont refermées au retour : on peut écouter de
/// nouveau au même port aussitôt.
pub struct Listening {
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
    /// Les adresses tenues.
    pub bound: Vec<SocketAddr>,
    /// Les relais où ce poste attend, et quand chacun a tenu l'attente
    /// pour la dernière fois.
    held: Arc<std::sync::Mutex<HashMap<SocketAddr, Instant>>>,
    /// Où remettre une offre arrivée par un tracker (`rendezvous.rs`) :
    /// le poste y répond depuis ses prises, en ICE complet.
    pub offers: mpsc::Sender<crate::rendezvous::Offered>,
}

impl Listening {
    /// **Les relais qui tiennent l'attente de ce poste** : ce qu'il peut
    /// publier, une page les composant comme ses propres adresses.
    pub fn relayed(&self) -> Vec<SocketAddr> {
        let mut out: Vec<SocketAddr> = self
            .held
            .lock()
            .map(|h| {
                h.iter()
                    .filter(|(_, t)| t.elapsed() < crate::relay::UDP_WAIT_BOUND)
                    .map(|(a, _)| *a)
                    .collect()
            })
            .unwrap_or_default();
        out.sort();
        out
    }
}

impl Drop for Listening {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

/// **Écouter les pages** sur `sockets` (de [`bind_all`]), sous l'identité
/// de l'officine et pour sa boîte ; chaque canal ouvert part par `links`.
pub fn serve(
    sockets: Vec<UdpSocket>,
    identity: &Identity,
    box_id: &str,
    links: mpsc::Sender<Link>,
) -> Listening {
    serve_relayed(sockets, &[], "", identity, box_id, links)
}

/// **Une prise de ce poste** : une adresse à lui, ou celle d'un relais
/// où il attend (`relay.rs`) — la même pour la page, qui y compose ce
/// poste ; seul ce qui passe par la prise change d'enveloppe.
struct Port {
    socket: UdpSocket,
    /// Ce que la page compose : l'adresse de la prise, ou du relais.
    at: SocketAddr,
    relayed: bool,
}

impl Port {
    fn send(&self, contents: &[u8], to: SocketAddr) {
        if self.relayed {
            let frame = crate::relay::UdpFrame::Data {
                peer: to,
                payload: contents,
            };
            let _ = self
                .socket
                .send_to(&crate::relay::udp_encode(&frame), self.at);
        } else {
            let _ = self.socket.send_to(contents, to);
        }
    }
}

/// [`serve`], et en plus **attendre aux relais** `relays` sous le nom du
/// groupe de postes `group` : une page qui compose l'adresse d'un relais
/// arrive ici comme à une adresse de ce poste.
pub fn serve_relayed(
    sockets: Vec<UdpSocket>,
    relays: &[SocketAddr],
    group: &str,
    identity: &Identity,
    box_id: &str,
    links: mpsc::Sender<Link>,
) -> Listening {
    let stop = Arc::new(AtomicBool::new(false));
    let bound = sockets.iter().filter_map(|s| s.local_addr().ok()).collect();
    let mut ports: Vec<Port> = sockets
        .into_iter()
        .filter_map(|socket| {
            let at = socket.local_addr().ok()?;
            Some(Port {
                socket,
                at,
                relayed: false,
            })
        })
        .collect();
    for at in relays {
        let any: SocketAddr = if at.is_ipv4() {
            (std::net::Ipv4Addr::UNSPECIFIED, 0).into()
        } else {
            (std::net::Ipv6Addr::UNSPECIFIED, 0).into()
        };
        let Ok(socket) = UdpSocket::bind(any) else {
            continue;
        };
        if socket.set_nonblocking(true).is_ok() {
            ports.push(Port {
                socket,
                at: *at,
                relayed: true,
            });
        }
    }
    let held: Arc<std::sync::Mutex<HashMap<SocketAddr, Instant>>> = Arc::default();
    let (cert, creds, flag) = (identity.dtls(), ice_creds(box_id), Arc::clone(&stop));
    let held_by = Arc::clone(&held);
    let group = group.to_owned();
    let (offers, offered) = mpsc::channel();
    let thread =
        std::thread::spawn(move || drive(ports, group, held_by, cert, creds, links, offered, flag));
    Listening {
        stop,
        thread: Some(thread),
        bound,
        held,
        offers,
    }
}

/// Combien de temps l'adresse que la box donne à une prise vaut avant
/// d'être redemandée au serveur STUN — ce qui garde aussi la traduction
/// ouverte.
const STUN_EVERY: Duration = Duration::from_secs(25);

/// **Une demande de liaison STUN** (RFC 5389) : vingt octets, sans
/// attribut, sous l'identifiant `txid`.
pub(crate) fn stun_request(txid: [u8; 12]) -> [u8; 20] {
    let mut out = [0u8; 20];
    out[0..2].copy_from_slice(&[0x00, 0x01]);
    out[4..8].copy_from_slice(&[0x21, 0x12, 0xa4, 0x42]);
    out[8..20].copy_from_slice(&txid);
    out
}

/// **L'adresse que la box a donnée**, lue dans la réponse d'un serveur
/// STUN à la demande `txid` (XOR-MAPPED-ADDRESS, ou MAPPED-ADDRESS d'un
/// vieux serveur) ; IPv4 seulement — une IPv6 publique n'a pas de box à
/// traverser.
pub(crate) fn stun_mapped(buf: &[u8], txid: &[u8; 12]) -> Option<SocketAddr> {
    if buf.len() < 20
        || buf[0..2] != [0x01, 0x01]
        || buf[4..8] != [0x21, 0x12, 0xa4, 0x42]
        || &buf[8..20] != txid
    {
        return None;
    }
    let mut i = 20;
    let mut plain = None;
    while i + 4 <= buf.len() {
        let kind = u16::from_be_bytes([buf[i], buf[i + 1]]);
        let len = u16::from_be_bytes([buf[i + 2], buf[i + 3]]) as usize;
        let body = buf.get(i + 4..i + 4 + len)?;
        if (kind == 0x0020 || kind == 0x0001) && len >= 8 && body[1] == 0x01 {
            let (port, ip) = if kind == 0x0020 {
                (
                    u16::from_be_bytes([body[2], body[3]]) ^ 0x2112,
                    [
                        body[4] ^ 0x21,
                        body[5] ^ 0x12,
                        body[6] ^ 0xa4,
                        body[7] ^ 0x42,
                    ],
                )
            } else {
                (
                    u16::from_be_bytes([body[2], body[3]]),
                    [body[4], body[5], body[6], body[7]],
                )
            };
            let at = SocketAddr::from((ip, port));
            if kind == 0x0020 {
                return Some(at);
            }
            plain = Some(at);
        }
        i += 4 + ((len + 3) & !3);
    }
    plain
}

/// Douze octets tirés au hasard : l'identifiant d'une demande STUN.
fn txid() -> [u8; 12] {
    use std::hash::{BuildHasher, Hasher};
    let mut out = [0u8; 12];
    for part in out.chunks_mut(8) {
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u128(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos()),
        );
        let v = h.finish().to_be_bytes();
        part.copy_from_slice(&v[..part.len()]);
    }
    out
}

/// L'offre d'une page sans ses candidats `.local` : un navigateur cache
/// ses adresses locales sous un nom mDNS, qui ne se résout pas ici et
/// qu'aucun analyseur SDP n'a à lire.
fn without_mdns(sdp: &str) -> String {
    sdp.split("\r\n")
        .flat_map(|l| l.split('\n'))
        .filter(|l| !(l.starts_with("a=candidate:") && l.contains(".local ")))
        .filter(|l| !l.trim().is_empty())
        .map(|l| format!("{}\r\n", l.trim_end_matches('\r')))
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn drive(
    ports: Vec<Port>,
    group: String,
    held: Arc<std::sync::Mutex<HashMap<SocketAddr, Instant>>>,
    cert: str0m::config::DtlsCert,
    creds: IceCreds,
    links: mpsc::Sender<Link>,
    offered: mpsc::Receiver<crate::rendezvous::Offered>,
    stop: Arc<AtomicBool>,
) {
    let locals: Vec<SocketAddr> = ports.iter().map(|p| p.at).collect();
    // **What the box gives each IPv4 socket**, asked of the public STUN
    // servers: the address a page reached through a tracker sends to.
    let mut stun_at: Vec<SocketAddr> = Vec::new();
    let mut stun_next = Instant::now();
    // Their names looked up on a thread of their own, again every ten
    // minutes: a slow resolver never holds a conversation up.
    let (resolved_tx, resolved) = mpsc::channel::<Vec<SocketAddr>>();
    std::thread::spawn(move || loop {
        let found: Vec<SocketAddr> = crate::rendezvous::STUN
            .iter()
            .filter_map(|h| std::net::ToSocketAddrs::to_socket_addrs(h).ok())
            .flatten()
            .filter(SocketAddr::is_ipv4)
            .collect();
        if resolved_tx.send(found).is_err() {
            return;
        }
        std::thread::sleep(Duration::from_secs(600));
    });
    let mut asked: Vec<Option<[u8; 12]>> = vec![None; ports.len()];
    let mut mapped: Vec<Option<SocketAddr>> = vec![None; ports.len()];
    let mut waited: Option<Instant> = None;
    // The cookie each relay gave this post's address: brought back with
    // every wait, without which none is held.
    let mut cookies: Vec<[u8; 16]> = vec![[0; 16]; ports.len()];
    let wait = |cookie: [u8; 16]| {
        crate::relay::udp_encode(&crate::relay::UdpFrame::Wait {
            group: &group,
            ufrag: &creds.ufrag,
            cookie,
        })
    };
    let (commands, orders) = mpsc::channel::<Command>();
    let mut peers: HashMap<u64, Peer> = HashMap::new();
    let mut next: u64 = 0;
    let mut buf = vec![0u8; 2048];
    while !stop.load(Ordering::SeqCst) {
        let now = Instant::now();
        let mut heard = false;
        // The wait said again at every relay: what keeps it held there,
        // and the operator's translation of this post's address open.
        if waited.is_none_or(|t| now.duration_since(t) >= crate::relay::UDP_RENEW) {
            waited = Some(now);
            for (p, cookie) in ports.iter().zip(&cookies).filter(|(p, _)| p.relayed) {
                let _ = p.socket.send_to(&wait(*cookie), p.at);
            }
        }
        while let Ok(found) = resolved.try_recv() {
            if !found.is_empty() {
                if stun_at.is_empty() {
                    stun_next = now;
                }
                stun_at = found;
            }
        }
        if now >= stun_next && !stun_at.is_empty() {
            stun_next = now + STUN_EVERY;
            for (i, p) in ports.iter().enumerate() {
                if p.relayed || !p.at.is_ipv4() {
                    continue;
                }
                let id = txid();
                asked[i] = Some(id);
                for server in &stun_at {
                    let _ = p.socket.send_to(&stun_request(id), server);
                }
            }
        }
        for (index, port) in ports.iter().enumerate() {
            let local = port.at;
            // A burst at most per socket and round: one page that floods
            // does not starve the others.
            for _ in 0..64 {
                let Ok((n, from)) = port.socket.recv_from(&mut buf) else {
                    break;
                };
                heard = true;
                // Through a relay: only what the relay says, the page's
                // datagram unwrapped with the page's address.
                let (source, data) = if port.relayed {
                    if from != port.at {
                        continue;
                    }
                    match crate::relay::udp_decode(&buf[..n]) {
                        Some(crate::relay::UdpFrame::Held { cookie, held: true })
                            if cookie == cookies[index] =>
                        {
                            if let Ok(mut h) = held.lock() {
                                h.insert(port.at, now);
                            }
                            continue;
                        }
                        // A cookie for this address: the wait said again
                        // at once, with it.
                        Some(crate::relay::UdpFrame::Held { cookie, .. }) => {
                            if cookie != cookies[index] {
                                cookies[index] = cookie;
                                let _ = port.socket.send_to(&wait(cookie), port.at);
                            }
                            continue;
                        }
                        Some(crate::relay::UdpFrame::Data { peer, payload }) => (peer, payload),
                        _ => continue,
                    }
                } else {
                    if stun_at.contains(&from) {
                        if let Some(at) = asked[index].and_then(|id| stun_mapped(&buf[..n], &id)) {
                            mapped[index] = Some(at);
                        }
                        continue;
                    }
                    (from, &buf[..n])
                };
                let Ok(contents) = data.try_into() else {
                    continue;
                };
                let input = Input::Receive(
                    now,
                    Receive {
                        proto: Protocol::Udp,
                        source,
                        destination: local,
                        contents,
                    },
                );
                if let Some(p) = peers.values_mut().find(|p| p.rtc.accepts(&input)) {
                    let _ = p.rtc.handle_input(input);
                    continue;
                }
                // A new page: only a binding request with our ufrag, and
                // within the bounds.
                let Some((ours, theirs)) = stun_username(data).and_then(|u| u.split_once(':'))
                else {
                    continue;
                };
                let who = caller(source.ip());
                let theirs_ = || peers.values().filter(|p| caller(p.from.ip()) == who);
                let opening = peers.values().filter(|p| p.channel.is_none()).count();
                if ours != creds.ufrag
                    || theirs.is_empty()
                    || peers.len() >= MOST_PEERS
                    || opening >= MOST_OPENING
                    || theirs_().count() >= PEERS_PER_CALLER
                    || theirs_().filter(|p| p.channel.is_none()).count() >= OPENING_PER_CALLER
                {
                    continue;
                }
                let mut rtc = RtcConfig::new()
                    .set_ice_lite(true)
                    .set_dtls_cert(cert.clone())
                    // The page's certificate is its own, drawn by its
                    // browser: nothing to hold it against. The patient's
                    // key does the proving (`patientchat`).
                    .set_fingerprint_verification(false)
                    .build(now);
                {
                    let mut api: DirectApi = rtc.direct_api();
                    api.set_ice_controlling(false);
                    api.set_local_ice_credentials(creds.clone());
                    api.set_remote_ice_credentials(IceCreds {
                        ufrag: theirs.to_owned(),
                        // Never used: an ICE-lite side sends no check.
                        pass: creds.pass.clone(),
                    });
                    api.set_remote_fingerprint(Fingerprint {
                        hash_func: "sha-256".into(),
                        bytes: vec![0; 32],
                    });
                    if api.start_dtls(false).is_err() {
                        continue;
                    }
                    api.start_sctp(false);
                }
                // Every socket a candidate: a page that tries another of
                // this post's addresses later reaches the same peer.
                for l in &locals {
                    if let Ok(candidate) = Candidate::host(*l, "udp") {
                        rtc.add_local_candidate(candidate);
                    }
                }
                let Ok(contents) = data.try_into() else {
                    continue;
                };
                let input = Input::Receive(
                    now,
                    Receive {
                        proto: Protocol::Udp,
                        source,
                        destination: local,
                        contents,
                    },
                );
                // Only a request signed with our password takes a place:
                // the ufrag alone, anyone reading a board link has it.
                if !rtc.accepts(&input) || rtc.handle_input(input).is_err() {
                    continue;
                }
                peers.insert(
                    next,
                    Peer {
                        rtc,
                        socket: index,
                        from: source,
                        held: Arc::new(AtomicUsize::new(0)),
                        born: now,
                        channel: None,
                        to_link: None,
                        outbox: VecDeque::new(),
                        closing: None,
                    },
                );
                next += 1;
            }
        }
        // **An offer through a tracker** (`rendezvous.rs`): answered from
        // this post's sockets, with what the box gives them, in full ICE —
        // this side sends its checks too, which is what opens its box to
        // the page.
        while let Ok(o) = offered.try_recv() {
            let opening = peers.values().filter(|p| p.channel.is_none()).count();
            if peers.len() >= MOST_PEERS || opening >= MOST_OPENING {
                continue;
            }
            let Ok(offer) = str0m::change::SdpOffer::from_sdp_string(&without_mdns(&o.sdp)) else {
                continue;
            };
            let mut rtc = RtcConfig::new().set_dtls_cert(cert.clone()).build(now);
            let mut first = None;
            for (i, p) in ports.iter().enumerate().filter(|(_, p)| !p.relayed) {
                if let Ok(c) = Candidate::host(p.at, "udp") {
                    rtc.add_local_candidate(c);
                    first.get_or_insert(i);
                }
                if let Some(at) = mapped[i] {
                    if let Ok(c) = Candidate::server_reflexive(at, p.at, "udp") {
                        rtc.add_local_candidate(c);
                    }
                }
            }
            let Some(socket) = first else {
                continue;
            };
            let Ok(answer) = rtc.sdp_api().accept_offer(offer) else {
                continue;
            };
            if o.answer.send(answer.to_sdp_string()).is_err() {
                continue;
            }
            let unspecified: SocketAddr = (std::net::Ipv4Addr::UNSPECIFIED, 0).into();
            peers.insert(
                next,
                Peer {
                    rtc,
                    socket,
                    from: unspecified,
                    held: Arc::new(AtomicUsize::new(0)),
                    born: now,
                    channel: None,
                    to_link: None,
                    outbox: VecDeque::new(),
                    closing: None,
                },
            );
            next += 1;
        }
        while let Ok(order) = orders.try_recv() {
            match order {
                Command::Send(id, bytes) => {
                    if let Some(p) = peers.get_mut(&id) {
                        p.outbox.extend(bytes.chunks(CHUNK).map(<[u8]>::to_vec));
                    }
                }
                Command::Close(id) => {
                    if let Some(p) = peers.get_mut(&id) {
                        p.closing.get_or_insert(now);
                        p.to_link = None;
                    }
                }
            }
        }
        let now = Instant::now();
        let mut gone: Vec<u64> = Vec::new();
        for (id, p) in peers.iter_mut() {
            if let Some(cid) = p.channel {
                while let Some(chunk) = p.outbox.front() {
                    let Some(mut ch) = p.rtc.channel(cid) else {
                        break;
                    };
                    match ch.write(true, chunk) {
                        Ok(true) => {
                            p.outbox.pop_front();
                        }
                        _ => break,
                    }
                }
            }
            let _ = p.rtc.handle_input(Input::Timeout(now));
            loop {
                match p.rtc.poll_output() {
                    Ok(Output::Transmit(t)) => {
                        // From the socket the answer names as its source.
                        let i = locals
                            .iter()
                            .position(|l| *l == t.source)
                            .unwrap_or(p.socket);
                        ports[i].send(&t.contents, t.destination);
                    }
                    Ok(Output::Timeout(_)) => break,
                    Ok(Output::Event(e)) => match e {
                        Event::ChannelOpen(cid, _) if p.channel.is_none() => {
                            p.channel = Some(cid);
                            let (tx, rx) = mpsc::channel();
                            p.to_link = Some(tx);
                            let link = Link {
                                peer: *id,
                                from: p.from,
                                inbox: rx,
                                pending: VecDeque::new(),
                                commands: commands.clone(),
                                held: Arc::clone(&p.held),
                                deadline: now + CONVERSATION_BOUND,
                            };
                            if links.send(link).is_err() {
                                return;
                            }
                        }
                        Event::ChannelData(d) if Some(d.id) == p.channel => {
                            let held = p.held.fetch_add(d.data.len(), Ordering::SeqCst);
                            if held + d.data.len() > HELD_MOST {
                                gone.push(*id);
                                break;
                            }
                            if let Some(tx) = &p.to_link {
                                let _ = tx.send(d.data);
                            }
                        }
                        Event::ChannelClose(cid) if Some(cid) == p.channel => {
                            p.to_link = None;
                            p.closing.get_or_insert(now);
                        }
                        _ => {}
                    },
                    Err(_) => {
                        gone.push(*id);
                        break;
                    }
                }
            }
            let drained = p.outbox.is_empty()
                && p.channel
                    .and_then(|cid| p.rtc.channel(cid).map(|mut c| c.buffered_amount()))
                    .unwrap_or(0)
                    == 0;
            let age = now.duration_since(p.born);
            if !p.rtc.is_alive()
                || (p.channel.is_none() && age > OPEN_BOUND)
                || age > TALK_BOUND
                || p.closing
                    .is_some_and(|t| drained || now.duration_since(t) > DRAIN_BOUND)
            {
                gone.push(*id);
            }
        }
        for id in gone {
            if let Some(mut p) = peers.remove(&id) {
                p.rtc.disconnect();
                // The last words (an SCTP abort, a DTLS close) out.
                while let Ok(Output::Transmit(t)) = p.rtc.poll_output() {
                    ports[p.socket].send(&t.contents, t.destination);
                }
            }
        }
        if !heard {
            // Idle: a page's first packet waits a few milliseconds — its
            // browser sends it again anyway. Talking: answer at once.
            std::thread::sleep(if peers.is_empty() {
                Duration::from_millis(25)
            } else {
                Duration::from_millis(2)
            });
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// **Le côté de la page**, en Rust : ce que le navigateur fait avec
    /// le lien — une connexion ICE contrôlante vers chaque adresse, DTLS
    /// actif contre l'empreinte, un canal. Rend un flux d'octets.
    pub(crate) struct Dialed {
        inbox: mpsc::Receiver<Vec<u8>>,
        pending: VecDeque<u8>,
        out: mpsc::Sender<Option<Vec<u8>>>,
    }

    impl Read for Dialed {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            if self.pending.is_empty() {
                match self.inbox.recv_timeout(Duration::from_secs(10)) {
                    Ok(c) => self.pending.extend(c),
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        return Err(std::io::ErrorKind::TimedOut.into())
                    }
                    Err(_) => return Ok(0),
                }
            }
            let n = buf.len().min(self.pending.len());
            for (b, v) in buf.iter_mut().zip(self.pending.drain(..n)) {
                *b = v;
            }
            Ok(n)
        }
    }

    impl Write for Dialed {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.out
                .send(Some(buf.to_vec()))
                .map_err(|_| std::io::Error::from(std::io::ErrorKind::BrokenPipe))?;
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl Drop for Dialed {
        fn drop(&mut self) {
            let _ = self.out.send(None);
        }
    }

    pub(crate) fn dial(to: &[SocketAddr], box_id: &str, fp: [u8; 32]) -> Option<Dialed> {
        let socket = UdpSocket::bind("127.0.0.1:0").ok()?;
        socket.set_nonblocking(true).ok()?;
        let local = socket.local_addr().ok()?;
        let mut rtc = RtcConfig::new().build(Instant::now());
        let cid = {
            let mut api = rtc.direct_api();
            api.set_ice_controlling(true);
            // Drawn per page, as a browser does: two pages under one
            // ufrag would be one peer to the post.
            static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let n = NEXT.fetch_add(1, Ordering::SeqCst);
            api.set_local_ice_credentials(IceCreds {
                ufrag: format!("page{n:04}{}", std::process::id()),
                pass: "pagepagepagepagepagepagepage".into(),
            });
            api.set_remote_ice_credentials(ice_creds(box_id));
            api.set_remote_fingerprint(Fingerprint {
                hash_func: "sha-256".into(),
                bytes: fp.to_vec(),
            });
            api.start_dtls(true).ok()?;
            api.start_sctp(true);
            api.create_data_channel(str0m::channel::ChannelConfig {
                label: "bpm".into(),
                ..Default::default()
            })
        };
        rtc.add_local_candidate(Candidate::host(local, "udp").ok()?);
        for a in to {
            rtc.add_remote_candidate(Candidate::host(*a, "udp").ok()?);
        }
        let (in_tx, inbox) = mpsc::channel();
        let (out, out_rx) = mpsc::channel::<Option<Vec<u8>>>();
        let (ready_tx, ready) = mpsc::channel();
        std::thread::spawn(move || {
            let mut buf = vec![0u8; 2048];
            let mut open = false;
            let mut queue: VecDeque<Vec<u8>> = VecDeque::new();
            let start = Instant::now();
            while start.elapsed() < Duration::from_secs(60) {
                let now = Instant::now();
                while let Ok((n, source)) = socket.recv_from(&mut buf) {
                    if let Ok(contents) = buf[..n].try_into() {
                        let _ = rtc.handle_input(Input::Receive(
                            now,
                            Receive {
                                proto: Protocol::Udp,
                                source,
                                destination: local,
                                contents,
                            },
                        ));
                    }
                }
                loop {
                    match out_rx.try_recv() {
                        Ok(Some(b)) => queue.extend(b.chunks(CHUNK).map(<[u8]>::to_vec)),
                        Ok(None) | Err(mpsc::TryRecvError::Disconnected) => return,
                        Err(mpsc::TryRecvError::Empty) => break,
                    }
                }
                if open {
                    while let Some(c) = queue.front() {
                        match rtc.channel(cid).map(|mut ch| ch.write(true, c)) {
                            Some(Ok(true)) => {
                                queue.pop_front();
                            }
                            _ => break,
                        }
                    }
                }
                let _ = rtc.handle_input(Input::Timeout(Instant::now()));
                loop {
                    match rtc.poll_output() {
                        Ok(Output::Transmit(t)) => {
                            let _ = socket.send_to(&t.contents, t.destination);
                        }
                        Ok(Output::Timeout(_)) | Err(_) => break,
                        Ok(Output::Event(Event::ChannelOpen(..))) => {
                            open = true;
                            let _ = ready_tx.send(());
                        }
                        Ok(Output::Event(Event::ChannelData(d))) => {
                            let _ = in_tx.send(d.data);
                        }
                        Ok(Output::Event(_)) => {}
                    }
                }
                std::thread::sleep(Duration::from_millis(2));
            }
        });
        ready.recv_timeout(Duration::from_secs(15)).ok()?;
        Some(Dialed {
            inbox,
            pending: VecDeque::new(),
            out,
        })
    }

    fn loopback() -> Vec<UdpSocket> {
        bind_all(&["127.0.0.1".parse().unwrap()], 0)
    }

    const BOX: &str = "0123456789abcdef0123456789abcdef";

    #[test]
    fn an_identity_reads_back_and_is_fingerprinted_as_the_browser_does() {
        let id = Identity::draw().expect("certificate");
        assert_eq!(Identity::read(&id.text()), Some(id.clone()));
        assert_eq!(Identity::read("pas une identité"), None);
        // What DTLS presents is what the link names.
        let mut rtc = RtcConfig::new()
            .set_dtls_cert(id.dtls())
            .build(Instant::now());
        let shown = rtc.direct_api().local_dtls_fingerprint().clone();
        assert_eq!(shown.hash_func, "sha-256");
        assert_eq!(shown.bytes, id.fingerprint().to_vec());
        assert_eq!(fingerprint_text(&id.fingerprint()).len(), 43);
        // Never the private key in a log line.
        assert!(!format!("{id:?}").contains(&b64().encode(&id.key)));
    }

    #[test]
    fn a_caller_is_an_ipv4_or_an_ipv6_slash_64() {
        let a: IpAddr = "2001:db8:1:2:aaaa::1".parse().unwrap();
        let b: IpAddr = "2001:db8:1:2:bbbb::9".parse().unwrap();
        let c: IpAddr = "2001:db8:1:3::1".parse().unwrap();
        assert_eq!(caller(a), caller(b));
        assert_ne!(caller(a), caller(c));
        let v4: IpAddr = "203.0.113.7".parse().unwrap();
        assert_eq!(caller(v4), v4);
    }

    #[test]
    fn the_ice_credentials_come_from_the_box() {
        let c = ice_creds(BOX);
        assert_eq!(c.ufrag, "01234567");
        assert_eq!(c.pass, BOX);
        // ICE wants four characters of ufrag and twenty-two of password.
        assert!(c.ufrag.len() >= 4 && c.pass.len() >= 22);
    }

    #[test]
    fn a_stun_username_is_read_only_from_a_binding_request() {
        let mut p = vec![0x00, 0x01, 0x00, 0x00, 0x21, 0x12, 0xa4, 0x42];
        p.extend_from_slice(&[0; 12]);
        p.extend_from_slice(&[0x00, 0x06, 0x00, 0x0d]);
        p.extend_from_slice(b"01234567:abcd");
        p.extend_from_slice(&[0, 0, 0]);
        assert_eq!(stun_username(&p), Some("01234567:abcd"));
        // A response, or a truncated attribute: nothing.
        let mut r = p.clone();
        r[1] = 0x01;
        r[0] = 0x01;
        assert_eq!(stun_username(&r), None);
        assert_eq!(stun_username(&p[..30]), None);
        assert_eq!(stun_username(b"short"), None);
    }

    #[test]
    fn a_page_reaches_the_post_and_bytes_go_both_ways() {
        let id = Identity::draw().unwrap();
        let (tx, links) = mpsc::channel();
        let listening = serve(loopback(), &id, BOX, tx);
        let at = listening.bound.clone();
        let mut page = dial(&at, BOX, id.fingerprint()).expect("channel open");
        let mut link = links.recv_timeout(Duration::from_secs(10)).expect("a link");
        page.write_all(b"bonjour").unwrap();
        let mut got = [0u8; 7];
        link.read_exact(&mut got).unwrap();
        assert_eq!(&got, b"bonjour");
        // Larger than one chunk, both ways, in order.
        let big: Vec<u8> = (0..100_000u32).map(|i| (i % 251) as u8).collect();
        link.write_all(&big).unwrap();
        let mut back = vec![0u8; big.len()];
        page.read_exact(&mut back).unwrap();
        assert_eq!(back, big);
    }

    #[test]
    fn a_page_holding_another_fingerprint_never_opens_a_channel() {
        let id = Identity::draw().unwrap();
        let (tx, links) = mpsc::channel();
        let listening = serve(loopback(), &id, BOX, tx);
        assert!(dial(&listening.bound, BOX, [7; 32]).is_none());
        assert!(links.try_recv().is_err());
    }

    #[test]
    fn a_page_with_another_box_is_not_answered() {
        let id = Identity::draw().unwrap();
        let (tx, links) = mpsc::channel();
        let listening = serve(loopback(), &id, BOX, tx);
        assert!(dial(
            &listening.bound,
            "fedcba9876543210fedcba9876543210",
            id.fingerprint()
        )
        .is_none());
        assert!(links.try_recv().is_err());
    }

    /// **Une page venue par un tracker** (`rendezvous.rs`) : son offre,
    /// la réponse du poste — qui porte l'empreinte de l'officine, celle
    /// que la page vérifie — puis le canal, en ICE complet des deux côtés.
    #[test]
    fn a_page_met_through_a_tracker_opens_a_channel() {
        let id = Identity::draw().unwrap();
        let (tx, links) = mpsc::channel();
        let listening = serve(loopback(), &id, BOX, tx);
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        socket.set_nonblocking(true).unwrap();
        let local = socket.local_addr().unwrap();
        let mut rtc = RtcConfig::new().build(Instant::now());
        rtc.add_local_candidate(Candidate::host(local, "udp").unwrap());
        let mut api = rtc.sdp_api();
        let cid = api.add_channel("bpm".into());
        let (offer, pending) = api.apply().expect("an offer");
        // A browser's hidden local address rides along: skipped, not fatal.
        let sdp = offer.to_sdp_string().replace(
            "a=end-of-candidates",
            "a=candidate:9 1 udp 2122260223 0b1c2d3e-aaaa.local 51000 typ host\r\na=end-of-candidates",
        );
        let (answer_tx, answer_rx) = mpsc::channel();
        listening
            .offers
            .send(crate::rendezvous::Offered {
                sdp,
                answer: answer_tx,
            })
            .unwrap();
        let answer = answer_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("an answer");
        let expected = id
            .fingerprint()
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(":");
        assert!(answer.contains(&expected), "{answer}");
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
                let input = Input::Receive(
                    now,
                    Receive {
                        proto: Protocol::Udp,
                        source,
                        destination: local,
                        contents: buf[..n].try_into().unwrap(),
                    },
                );
                let _ = rtc.handle_input(input);
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
        assert!(open, "le canal ne s'ouvre pas");
        assert!(links.recv_timeout(Duration::from_secs(5)).is_ok());
    }

    #[test]
    fn what_the_box_gives_is_read_from_a_stun_answer() {
        let id = [7u8; 12];
        assert_eq!(&stun_request(id)[8..], &id);
        // 203.0.113.7:51000, XOR-mapped as a server writes it.
        let mut r = vec![0x01, 0x01, 0x00, 0x0c, 0x21, 0x12, 0xa4, 0x42];
        r.extend_from_slice(&id);
        let port = 51000u16 ^ 0x2112;
        r.extend_from_slice(&[0x00, 0x20, 0x00, 0x08, 0x00, 0x01]);
        r.extend_from_slice(&port.to_be_bytes());
        r.extend_from_slice(&[203 ^ 0x21, 0x12, 113 ^ 0xa4, 7 ^ 0x42]);
        assert_eq!(
            stun_mapped(&r, &id),
            Some("203.0.113.7:51000".parse().unwrap())
        );
        // Another request's answer, or a request: nothing.
        assert_eq!(stun_mapped(&r, &[8; 12]), None);
        r[0] = 0x00;
        r[1] = 0x01;
        assert_eq!(stun_mapped(&r, &id), None);
    }

    /// **Un vrai serveur STUN** dit l'adresse que la box donne. Demande
    /// le réseau : `--ignored`.
    #[test]
    #[ignore]
    fn a_public_stun_server_says_what_the_box_gives() {
        let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let server = std::net::ToSocketAddrs::to_socket_addrs(crate::rendezvous::STUN[0])
            .unwrap()
            .find(SocketAddr::is_ipv4)
            .unwrap();
        let id = txid();
        socket.send_to(&stun_request(id), server).unwrap();
        let mut buf = [0u8; 512];
        let (n, _) = socket.recv_from(&mut buf).unwrap();
        let at = stun_mapped(&buf[..n], &id).expect("an address");
        assert!(at.port() != 0);
    }

    #[test]
    fn the_first_answering_address_wins() {
        // One address that answers nothing, then the post: the page's one
        // connection still finds it.
        let id = Identity::draw().unwrap();
        let (tx, links) = mpsc::channel();
        let listening = serve(loopback(), &id, BOX, tx);
        let dead = UdpSocket::bind("127.0.0.1:0").unwrap();
        let mut to = vec![dead.local_addr().unwrap()];
        to.extend(listening.bound.iter().copied());
        let mut page = dial(&to, BOX, id.fingerprint()).expect("channel open");
        let mut link = links.recv_timeout(Duration::from_secs(10)).unwrap();
        link.write_all(b"ok").unwrap();
        let mut got = [0u8; 2];
        page.read_exact(&mut got).unwrap();
        assert_eq!(&got, b"ok");
    }
}
