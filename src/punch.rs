//! **La voie percée** : un téléphone de l'équipe et un poste de l'officine
//! se parlent quand ni l'un ni l'autre n'est joignable — ni box ouverte,
//! ni IPv6 publique, ni relais offert par une autre officine.
//!
//! Ce que la page du patient fait avec un poste (`rendezvous.rs`), entre
//! deux programmes à nous : chacun demande à un serveur STUN public
//! l'adresse que sa box donne à sa prise, l'appelant publie une offre
//! WebRTC dans l'essaim du groupe sur les trackers WebTorrent publics, le
//! poste qui y attend répond, puis les deux s'envoient leurs paquets en
//! même temps — chaque box s'ouvre à l'autre — et un canal de données
//! s'ouvre entre eux. La conversation du groupe (`bpm_sync`) y passe
//! telle quelle : la poignée de main prouve de chaque côté qu'on parle au
//! bon poste ; le tracker ne voit que des adresses, DTLS et la poignée de
//! main chiffrent de bout en bout. Rien n'est ouvert sur la box, rien ne
//! passe par un serveur de l'éditeur ni ne s'y garde.
//!
//! Deux box qui changent de port à chaque destinataire (NAT symétrique)
//! ne se joignent pas ainsi ; [`gather`] le reconnaît (deux serveurs STUN
//! qui ne voient pas le même port) et le diagnostic le dit.

use std::collections::VecDeque;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};
use str0m::net::{Protocol, Receive};
use str0m::{Candidate, Event, Input, Output, Rtc, RtcConfig};

/// Combien d'attente pour qu'un poste réponde, par un tracker, à l'offre
/// d'un téléphone.
pub const ANSWER_WAIT: Duration = Duration::from_secs(15);
/// Combien d'attente, la réponse arrivée, pour que le canal s'ouvre :
/// les vérifications ICE des deux côtés, DTLS, SCTP.
const OPEN_WAIT: Duration = Duration::from_secs(12);
/// Combien d'attente pour les serveurs STUN.
const STUN_WAIT: Duration = Duration::from_millis(1500);
/// Une conversation percée, au plus.
const TALK_MOST: Duration = Duration::from_secs(10 * 60);
/// Le silence au bout duquel l'autre bout est tenu pour parti.
const SILENCE_MOST: Duration = Duration::from_secs(90);
/// Ce qu'un message de la conversation pèse au plus : celui du fil.
const MESSAGE_MOST: usize = bpm_sync::MAX_WIRE;
/// Ce qui part en une écriture sur le canal.
const CHUNK: usize = 16 * 1024;
/// Des offres en cours d'ouverture à la fois, côté poste ; et par minute.
const OPENING_MOST: usize = 4;
const OFFERS_PER_MINUTE: usize = 12;

/// **L'essaim d'un groupe de postes** : vingt chiffres hexadécimaux tirés
/// de son nom (celui que connaissent ses postes et les relais). Il ne
/// protège rien : un inconnu qui y entre n'obtient qu'une poignée de main
/// manquée, et le poste n'en ouvre que quelques-unes par minute.
pub fn swarm(group: &str) -> String {
    let mut h = Sha256::new();
    h.update(b"bpm-caddy postes 1|");
    h.update(group.as_bytes());
    h.finalize()[..10]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// **Ce que la box fait d'une prise** : l'adresse publique qu'elle lui
/// donne, et si elle en donne une autre à chaque destinataire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Mapping {
    pub public: Option<SocketAddr>,
    /// Deux serveurs ont vu deux ports différents : NAT symétrique, que
    /// le perçage ne traverse pas.
    pub symmetric: bool,
}

/// Les adresses STUN, résolues ; vide sans DNS.
fn stun_servers() -> Vec<SocketAddr> {
    crate::rendezvous::STUN
        .iter()
        .filter_map(|s| std::net::ToSocketAddrs::to_socket_addrs(s).ok())
        .flat_map(|a| a.filter(SocketAddr::is_ipv4).take(1))
        .collect()
}

/// **Demander aux serveurs STUN** ce que la box donne à `socket`
/// (non bloquante), dans [`STUN_WAIT`].
pub fn gather(socket: &UdpSocket, servers: &[SocketAddr]) -> Mapping {
    let id = crate::direct::txid();
    for s in servers {
        let _ = socket.send_to(&crate::direct::stun_request(id), s);
    }
    let mut seen: Vec<(SocketAddr, SocketAddr)> = Vec::new();
    let until = Instant::now() + STUN_WAIT;
    let mut buf = [0u8; 1024];
    while Instant::now() < until && seen.len() < servers.len() {
        match socket.recv_from(&mut buf) {
            Ok((n, from)) if servers.contains(&from) => {
                if let Some(at) = crate::direct::stun_mapped(&buf[..n], &id) {
                    if !seen.iter().any(|(f, _)| *f == from) {
                        seen.push((from, at));
                    }
                }
            }
            Ok(_) => {}
            Err(_) => std::thread::sleep(Duration::from_millis(10)),
        }
    }
    Mapping {
        public: seen.first().map(|(_, at)| *at),
        symmetric: seen.windows(2).any(|w| w[0].1 != w[1].1),
    }
}

/// **Ce que la box de ce poste fait d'une prise**, demandé sur une prise
/// neuve : le test de connexion le dit (`network::diagnose`).
pub fn probe() -> Mapping {
    match UdpSocket::bind("0.0.0.0:0") {
        Ok(s) if s.set_nonblocking(true).is_ok() => gather(&s, &stun_servers()),
        _ => Mapping::default(),
    }
}

/// Les adresses IPv4 de cette machine où ouvrir une prise : toutes, hors
/// boucle locale.
pub fn local_v4() -> Vec<Ipv4Addr> {
    if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|i| match i.ip() {
            IpAddr::V4(v4) if !v4.is_loopback() && !v4.is_link_local() => Some(v4),
            _ => None,
        })
        .take(4)
        .collect()
}

/// Une prise par adresse, et ses candidats : l'hôte, puis ce que la box
/// lui donne.
struct Ports {
    sockets: Vec<(UdpSocket, SocketAddr)>,
    candidates: Vec<Candidate>,
    mapping: Mapping,
}

fn open_ports(addresses: &[Ipv4Addr], stun: &[SocketAddr]) -> Ports {
    let mut sockets = Vec::new();
    let mut candidates = Vec::new();
    let mut mapping = Mapping::default();
    for ip in addresses {
        let Ok(s) = UdpSocket::bind((*ip, 0)) else {
            continue;
        };
        let (Ok(at), Ok(())) = (s.local_addr(), s.set_nonblocking(true)) else {
            continue;
        };
        if let Ok(c) = Candidate::host(at, "udp") {
            candidates.push(c);
        }
        if !stun.is_empty() {
            let m = gather(&s, stun);
            if let Some(public) = m.public {
                if public != at {
                    if let Ok(c) = Candidate::server_reflexive(public, at, "udp") {
                        candidates.push(c);
                    }
                }
                mapping.public.get_or_insert(public);
            }
            mapping.symmetric |= m.symmetric;
        }
        sockets.push((s, at));
    }
    Ports {
        sockets,
        candidates,
        mapping,
    }
}

/// **Une conversation percée**, vue comme un lien de la synchronisation :
/// des messages entiers, dans l'ordre. Le canal porte des morceaux ; un
/// message y passe précédé de sa longueur.
pub struct PunchedLink {
    out: mpsc::Sender<Vec<u8>>,
    inbox: mpsc::Receiver<Vec<u8>>,
    patience: Duration,
    deadline: Instant,
}

impl PunchedLink {
    /// Une conversation qui ne dure pas plus de `total` d'ici.
    pub fn with_deadline(mut self, total: Duration) -> Self {
        self.deadline = self.deadline.min(Instant::now() + total);
        self
    }
}

impl bpm_sync::Link for PunchedLink {
    fn send(&mut self, message: &[u8]) -> bpm_sync::Result<()> {
        if message.len() > MESSAGE_MOST || Instant::now() >= self.deadline {
            return Err(bpm_sync::Error::Link);
        }
        self.out
            .send(message.to_vec())
            .map_err(|_| bpm_sync::Error::Link)
    }

    fn recv(&mut self) -> bpm_sync::Result<Vec<u8>> {
        let left = self.deadline.saturating_duration_since(Instant::now());
        self.inbox
            .recv_timeout(self.patience.min(left))
            .map_err(|_| bpm_sync::Error::Link)
    }
}

/// Ce qui arrive du canal, remis en messages : longueur sur quatre
/// octets, puis le message.
#[derive(Default)]
struct Reassembly {
    buf: Vec<u8>,
}

impl Reassembly {
    /// Les messages complets ; `Err` quand une longueur annoncée dépasse
    /// ce qu'un message pèse.
    fn push(&mut self, data: &[u8]) -> Result<Vec<Vec<u8>>, ()> {
        self.buf.extend_from_slice(data);
        let mut out = Vec::new();
        while let Some(head) = self.buf.get(..4) {
            let len = u32::from_be_bytes([head[0], head[1], head[2], head[3]]) as usize;
            if len > MESSAGE_MOST {
                return Err(());
            }
            if self.buf.len() < 4 + len {
                break;
            }
            out.push(self.buf[4..4 + len].to_vec());
            self.buf.drain(..4 + len);
        }
        Ok(out)
    }
}

/// Un message en morceaux pour le canal, sa longueur en tête.
fn chunks(message: &[u8]) -> impl Iterator<Item = Vec<u8>> + '_ {
    let mut whole = (message.len() as u32).to_be_bytes().to_vec();
    whole.extend_from_slice(message);
    let n = whole.len().div_ceil(CHUNK);
    (0..n).map(move |i| whole[i * CHUNK..((i + 1) * CHUNK).min(whole.len())].to_vec())
}

/// **Tenir une connexion** jusqu'à sa fin, sur son fil : les prises, les
/// vérifications ICE, DTLS, le canal ; ce que le lien envoie part, ce qui
/// arrive va au lien. `opened` dit quand le canal s'ouvre.
fn drive(
    mut rtc: Rtc,
    ports: Ports,
    from_link: mpsc::Receiver<Vec<u8>>,
    to_link: mpsc::Sender<Vec<u8>>,
    opened: mpsc::Sender<()>,
) {
    let born = Instant::now();
    let mut channel = None;
    let mut outbox: VecDeque<Vec<u8>> = VecDeque::new();
    let mut whole = Reassembly::default();
    let mut heard_at = Instant::now();
    let mut link_gone = false;
    let mut buf = vec![0u8; 2048];
    loop {
        let now = Instant::now();
        if now.duration_since(born) > TALK_MOST
            || now.duration_since(heard_at) > SILENCE_MOST
            || (channel.is_none() && now.duration_since(born) > OPEN_WAIT)
            || !rtc.is_alive()
        {
            break;
        }
        let mut busy = false;
        for (socket, local) in &ports.sockets {
            for _ in 0..64 {
                let Ok((n, source)) = socket.recv_from(&mut buf) else {
                    break;
                };
                busy = true;
                let Ok(contents) = buf[..n].try_into() else {
                    continue;
                };
                let input = Input::Receive(
                    now,
                    Receive {
                        proto: Protocol::Udp,
                        source,
                        destination: *local,
                        contents,
                    },
                );
                if rtc.accepts(&input) {
                    heard_at = now;
                    let _ = rtc.handle_input(input);
                }
            }
        }
        loop {
            match from_link.try_recv() {
                Ok(m) => outbox.extend(chunks(&m)),
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    link_gone = true;
                    break;
                }
            }
        }
        if let Some(cid) = channel {
            while let Some(chunk) = outbox.front() {
                let Some(mut ch) = rtc.channel(cid) else {
                    break;
                };
                match ch.write(true, chunk) {
                    Ok(true) => {
                        outbox.pop_front();
                        busy = true;
                    }
                    _ => break,
                }
            }
        }
        let _ = rtc.handle_input(Input::Timeout(now));
        loop {
            match rtc.poll_output() {
                Ok(Output::Transmit(t)) => {
                    let s = ports
                        .sockets
                        .iter()
                        .find(|(_, l)| *l == t.source)
                        .or(ports.sockets.first());
                    if let Some((s, _)) = s {
                        let _ = s.send_to(&t.contents, t.destination);
                    }
                }
                Ok(Output::Timeout(_)) => break,
                Ok(Output::Event(e)) => match e {
                    Event::ChannelOpen(cid, _) if channel.is_none() => {
                        channel = Some(cid);
                        let _ = opened.send(());
                    }
                    Event::ChannelData(d) if Some(d.id) == channel => match whole.push(&d.data) {
                        Ok(messages) => {
                            for m in messages {
                                let _ = to_link.send(m);
                            }
                        }
                        Err(()) => {
                            link_gone = true;
                            outbox.clear();
                        }
                    },
                    Event::ChannelClose(cid) if Some(cid) == channel => link_gone = true,
                    _ => {}
                },
                Err(_) => {
                    link_gone = true;
                    break;
                }
            }
        }
        let drained = outbox.is_empty()
            && channel
                .and_then(|cid| rtc.channel(cid).map(|mut c| c.buffered_amount()))
                .unwrap_or(0)
                == 0;
        if link_gone && drained {
            break;
        }
        if !busy {
            std::thread::sleep(Duration::from_millis(2));
        }
    }
    rtc.disconnect();
    while let Ok(Output::Transmit(t)) = rtc.poll_output() {
        if let Some((s, _)) = ports.sockets.iter().find(|(_, l)| *l == t.source) {
            let _ = s.send_to(&t.contents, t.destination);
        }
    }
}

/// Le lien, et le fil qui tient sa connexion.
fn start(rtc: Rtc, ports: Ports, patience: Duration) -> (PunchedLink, mpsc::Receiver<()>) {
    let (out, from_link) = mpsc::channel();
    let (to_link, inbox) = mpsc::channel();
    let (opened, is_open) = mpsc::channel();
    std::thread::spawn(move || drive(rtc, ports, from_link, to_link, opened));
    (
        PunchedLink {
            out,
            inbox,
            patience,
            deadline: Instant::now() + TALK_MOST,
        },
        is_open,
    )
}

/// **Appeler un poste du groupe** par le rendez-vous : une offre publiée
/// dans l'essaim `swarm`, sa réponse, puis le perçage. `addresses` : où
/// ouvrir les prises ([`local_v4`]). `Err` : la raison, en français.
pub fn dial(
    swarm: &str,
    addresses: &[Ipv4Addr],
    patience: Duration,
) -> Result<PunchedLink, String> {
    dial_with(swarm, addresses, &stun_servers(), patience, |s, sdp| {
        crate::rendezvous::offer(s, sdp, ANSWER_WAIT)
    })
}

/// [`dial`], par un passeur d'offres donné (un vrai tracker, ou un test).
fn dial_with(
    swarm: &str,
    addresses: &[Ipv4Addr],
    stun: &[SocketAddr],
    patience: Duration,
    pass: impl FnOnce(&str, &str) -> Option<String>,
) -> Result<PunchedLink, String> {
    let ports = open_ports(addresses, stun);
    if ports.sockets.is_empty() {
        return Err(crate::strings::tr("punch_no_socket").to_owned());
    }
    let mut rtc = RtcConfig::new().build(Instant::now());
    for c in &ports.candidates {
        rtc.add_local_candidate(c.clone());
    }
    let mut api = rtc.sdp_api();
    api.add_channel("bpm-postes".into());
    let Some((offer, pending)) = api.apply() else {
        return Err(crate::strings::tr("punch_no_offer").to_owned());
    };
    let answer = pass(swarm, &offer.to_sdp_string())
        .ok_or_else(|| crate::strings::tr("punch_no_answer").to_owned())?;
    let answer = str0m::change::SdpAnswer::from_sdp_string(&answer)
        .map_err(|_| crate::strings::tr("punch_no_answer").to_owned())?;
    rtc.sdp_api()
        .accept_answer(pending, answer)
        .map_err(|_| crate::strings::tr("punch_no_answer").to_owned())?;
    let symmetric = ports.mapping.symmetric;
    let (link, is_open) = start(rtc, ports, patience);
    match is_open.recv_timeout(OPEN_WAIT) {
        Ok(()) => Ok(link),
        Err(_) if symmetric => Err(crate::strings::tr("punch_symmetric").to_owned()),
        Err(_) => Err(crate::strings::tr("punch_unopened").to_owned()),
    }
}

/// **Répondre à une offre** : les prises, ce que la box leur donne, la
/// réponse ; le lien quand le canal s'ouvre. `None` : une offre illisible,
/// ou un canal qui ne s'ouvre pas.
fn answer(
    sdp: &str,
    addresses: &[Ipv4Addr],
    stun: &[SocketAddr],
    reply: mpsc::Sender<String>,
    patience: Duration,
) -> Option<PunchedLink> {
    let offer = str0m::change::SdpOffer::from_sdp_string(sdp).ok()?;
    let ports = open_ports(addresses, stun);
    if ports.sockets.is_empty() {
        return None;
    }
    let mut rtc = RtcConfig::new().build(Instant::now());
    for c in &ports.candidates {
        rtc.add_local_candidate(c.clone());
    }
    let answer = rtc.sdp_api().accept_offer(offer).ok()?;
    reply.send(answer.to_sdp_string()).ok()?;
    let (link, is_open) = start(rtc, ports, patience);
    is_open.recv_timeout(OPEN_WAIT).ok().map(|()| link)
}

/// **Un poste qui attend dans l'essaim de son groupe** : chaque offre qui
/// arrive par un tracker est percée sur son fil, et le lien ouvert remis à
/// `give` — lâché quand le fil des postes en tient déjà un. Au plus
/// quelques ouvertures à la fois et par minute. Rend le nombre de trackers
/// où le poste attend.
pub fn serve(
    swarm: String,
    stop: Arc<AtomicBool>,
    give: mpsc::SyncSender<PunchedLink>,
    patience: Duration,
) -> Arc<AtomicUsize> {
    let (offers, offered) = mpsc::channel::<crate::rendezvous::Offered>();
    let slot: crate::rendezvous::OfferSlot = Arc::new(Mutex::new(Some(offers)));
    let up = Arc::new(AtomicUsize::new(0));
    crate::rendezvous::spawn_swarm(swarm, slot, Arc::clone(&stop), Arc::clone(&up));
    std::thread::spawn(move || {
        let opening = Arc::new(AtomicUsize::new(0));
        let mut recent: Vec<Instant> = Vec::new();
        let mut stun: Vec<SocketAddr> = Vec::new();
        let mut stun_read: Option<Instant> = None;
        while !stop.load(Ordering::SeqCst) {
            let Ok(o) = offered.recv_timeout(Duration::from_millis(500)) else {
                continue;
            };
            recent.retain(|t| t.elapsed() < Duration::from_secs(60));
            if recent.len() >= OFFERS_PER_MINUTE || opening.load(Ordering::SeqCst) >= OPENING_MOST {
                continue;
            }
            recent.push(Instant::now());
            if stun.is_empty() || stun_read.is_none_or(|t| t.elapsed() > Duration::from_secs(600)) {
                stun = stun_servers();
                stun_read = Some(Instant::now());
            }
            let (give, stun, opening) = (give.clone(), stun.clone(), Arc::clone(&opening));
            opening.fetch_add(1, Ordering::SeqCst);
            std::thread::spawn(move || {
                let link = answer(&o.sdp, &local_v4(), &stun, o.answer, patience);
                opening.fetch_sub(1, Ordering::SeqCst);
                if let Some(link) = link {
                    let _ = give.try_send(link);
                }
            });
        }
    });
    up
}

/// **Deux bouts percés sur la boucle locale**, le passeur d'offres tenant
/// lieu du tracker : (l'appelant, celui qui a répondu).
#[cfg(test)]
pub(crate) fn loopback_pair(patience: Duration) -> (PunchedLink, PunchedLink) {
    let here = [Ipv4Addr::LOCALHOST];
    let (links_tx, links) = mpsc::channel();
    let caller = dial_with("s", &here, &[], patience, |_, sdp| {
        let (tx, rx) = mpsc::channel();
        let sdp = sdp.to_owned();
        std::thread::spawn(move || {
            let _ = links_tx.send(answer(&sdp, &here, &[], tx, patience));
        });
        rx.recv_timeout(Duration::from_secs(5)).ok()
    })
    .expect("le canal s'ouvre");
    let answered = links
        .recv_timeout(Duration::from_secs(15))
        .ok()
        .flatten()
        .expect("celui qui répond a son lien");
    (caller, answered)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bpm_sync::Link;

    #[test]
    fn a_group_swarm_is_not_a_box_swarm() {
        let g = "0123456789abcdef0123";
        assert_eq!(swarm(g).len(), 20);
        assert_ne!(swarm(g), crate::rendezvous::swarm(g));
        assert_ne!(swarm(g), swarm("0123456789abcdef0124"));
    }

    /// **Un message en morceaux, et rendu entier** : à la taille d'un
    /// morceau près, et coupé n'importe où ; une longueur démesurée est
    /// refusée.
    #[test]
    fn a_message_crosses_the_channel_in_pieces() {
        for n in [0usize, 1, CHUNK - 4, CHUNK - 3, 3 * CHUNK + 17] {
            let m: Vec<u8> = (0..n).map(|i| (i % 253) as u8).collect();
            let mut r = Reassembly::default();
            let mut got = Vec::new();
            for c in chunks(&m) {
                assert!(c.len() <= CHUNK);
                for half in c.chunks(c.len().div_ceil(2).max(1)) {
                    got.extend(r.push(half).unwrap());
                }
            }
            assert_eq!(got, vec![m]);
        }
        let mut r = Reassembly::default();
        assert!(r.push(&u32::MAX.to_be_bytes()).is_err());
    }

    /// **Un téléphone et son poste, par une offre et sa réponse**, sur la
    /// boucle locale : le passeur d'offres tient lieu du tracker, et des
    /// messages plus grands qu'un morceau vont et viennent.
    #[test]
    fn a_phone_and_its_post_talk_over_a_punched_channel() {
        let (mut phone, mut post) = loopback_pair(Duration::from_secs(5));
        let big: Vec<u8> = (0..MESSAGE_MOST).map(|i| (i % 241) as u8).collect();
        phone.send(b"bonjour").unwrap();
        phone.send(&big).unwrap();
        assert!(phone.send(&vec![0; MESSAGE_MOST + 1]).is_err());
        assert_eq!(post.recv().unwrap(), b"bonjour");
        assert_eq!(post.recv().unwrap(), big);
        post.send(b"au revoir").unwrap();
        assert_eq!(phone.recv().unwrap(), b"au revoir");
        drop(post);
        // The other end gone: the phone hears the silence, not a hang.
        let mut phone = phone.with_deadline(Duration::from_secs(3));
        assert!(phone.recv().is_err());
    }

    /// **Une offre de nulle part** : sans passeur qui réponde, l'appel
    /// échoue avec sa raison, sans attendre l'ouverture.
    #[test]
    fn no_answer_is_said_as_such() {
        let t = Instant::now();
        let e = dial_with(
            "s",
            &[Ipv4Addr::LOCALHOST],
            &[],
            Duration::from_secs(1),
            |_, _| None,
        )
        .err()
        .unwrap();
        assert_eq!(e, crate::strings::tr("punch_no_answer"));
        assert!(t.elapsed() < Duration::from_secs(2));
    }

    /// **Un téléphone rejoint son poste par un vrai tracker**, sur la
    /// boucle locale. Demande le réseau : `--ignored`.
    #[test]
    #[ignore]
    fn a_phone_meets_its_post_through_a_public_tracker() {
        let s = swarm(&format!("test-{}", crate::rendezvous::draw_id()));
        let stop = Arc::new(AtomicBool::new(false));
        let (give, links) = mpsc::sync_channel(1);
        let up = serve(s.clone(), Arc::clone(&stop), give, Duration::from_secs(5));
        let until = Instant::now() + Duration::from_secs(15);
        while up.load(Ordering::SeqCst) == 0 && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(100));
        }
        std::thread::sleep(Duration::from_secs(1));
        let mut phone = dial(&s, &local_v4(), Duration::from_secs(5)).expect("percé");
        let mut post = links.recv_timeout(Duration::from_secs(10)).unwrap();
        phone.send(b"ping").unwrap();
        assert_eq!(post.recv().unwrap(), b"ping");
        stop.store(true, Ordering::SeqCst);
        // An empty swarm is said at once by the trackers, not waited for.
        let t = Instant::now();
        let empty = swarm(&crate::rendezvous::draw_id());
        assert!(dial(&empty, &local_v4(), Duration::from_secs(5)).is_err());
        assert!(t.elapsed() < Duration::from_secs(8), "{:?}", t.elapsed());
    }

    /// **Ce que la box fait d'une prise**, demandé à de vrais serveurs
    /// STUN. Demande le réseau : `--ignored`.
    #[test]
    #[ignore]
    fn the_box_mapping_is_read_from_two_stun_servers() {
        let s = UdpSocket::bind("0.0.0.0:0").unwrap();
        s.set_nonblocking(true).unwrap();
        let m = gather(&s, &stun_servers());
        eprintln!("{m:?}");
        assert!(m.public.is_some());
    }
}
