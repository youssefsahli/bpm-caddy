//! **Joignable depuis Internet** : ce qu'un poste fait pour qu'un
//! téléphone de l'équipe, hors de l'officine, puisse le composer.
//!
//! Éteint par défaut (`[postes] internet`) : c'est la seule porte que
//! l'application tient ouverte au-delà du réseau local, et l'officine la
//! décide poste par poste. La porte elle-même ne change pas — la poignée
//! de main chiffrée, et une réponse aux seuls postes du groupe (`docs/
//! SYNC.md` § 7.8). Ce module ne fait que trois choses, sans décider de
//! rien :
//!
//! * lire les adresses **IPv6 publiques** de la machine — en IPv6 il n'y
//!   a pas de traduction d'adresse : la box n'a qu'à laisser passer ;
//! * demander au **routeur** d'ouvrir le port des postes en IPv4 (UPnP
//!   IGD, sinon PCP ou NAT-PMP), et lire l'adresse publique qu'il
//!   annonce ;
//! * écrire et relire la liste d'adresses que le poste publie dans sa
//!   ligne de `sync_posts` — ce que le téléphone compose.
//!
//! Une adresse publique derrière une traduction d'opérateur (CGNAT,
//! `100.64.0.0/10`) ou privée n'est pas publiée : elle ne mènerait nulle
//! part, et le téléphone perdrait huit secondes à le découvrir.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, UdpSocket};
use std::time::Duration;

/// Combien d'adresses une ligne publie au plus.
pub const MOST: usize = 6;

/// La durée d'une ouverture de port demandée au routeur ; le poste la
/// renouvelle bien avant.
pub const LEASE: Duration = Duration::from_secs(3600);

/// Une adresse IPv6 qu'on peut composer depuis Internet : unicast
/// globale (`2000::/3`). Ni boucle locale, ni lien local, ni adresse
/// privée (`fc00::/7`), ni documentation (`2001:db8::/32`), ni les
/// tunnels de transition — Teredo (`2001::/32`) et 6to4 (`2002::/16`) —
/// qui ne mènent pas sûrement au poste.
pub fn is_global_v6(a: &Ipv6Addr) -> bool {
    let s = a.segments();
    (s[0] & 0xe000) == 0x2000
        && !(s[0] == 0x2001 && (s[1] == 0x0db8 || s[1] == 0))
        && s[0] != 0x2002
}

/// Une adresse IPv4 publique : ni privée, ni partagée par l'opérateur
/// (`100.64.0.0/10`), ni boucle, ni lien local, ni documentation.
pub fn is_public_v4(a: &Ipv4Addr) -> bool {
    let o = a.octets();
    !(a.is_private()
        || a.is_loopback()
        || a.is_link_local()
        || a.is_unspecified()
        || a.is_broadcast()
        || a.is_documentation()
        || a.is_multicast()
        || o[0] == 0
        || o[0] >= 240
        || (o[0] == 100 && (64..128).contains(&o[1]))
        || (o[0] == 192 && o[1] == 0 && o[2] == 0)
        || (o[0] == 198 && (o[1] == 18 || o[1] == 19)))
}

/// Les adresses IPv6 publiques de cette machine.
pub fn global_ipv6() -> Vec<Ipv6Addr> {
    let mut out: Vec<Ipv6Addr> = if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        // Ni un lien coupé, ni un tunnel (un VPN) : on ne les compose pas
        // depuis Internet, et un tunnel dirait à qui le lit par où sort
        // ce poste.
        .filter(|i| i.is_oper_up() && !i.is_p2p())
        .filter_map(|i| match i.ip() {
            IpAddr::V6(a) if is_global_v6(&a) => Some(a),
            _ => None,
        })
        .collect();
    out.sort();
    out.dedup();
    // Les adresses qui durent d'abord : une adresse temporaire (RFC 4941)
    // tourne chaque jour et ne vit qu'une semaine — une officine qui l'a
    // recopiée ne joindrait plus rien.
    let flags = temporary_or_deprecated();
    out.retain(|a| !flags.contains(a));
    out.sort_by_key(|a| !is_eui64(a));
    out
}

/// Une adresse formée sur l'adresse matérielle (`…ff:fe…`) : stable par
/// construction — et la seule que l'on reconnaisse sans demander au
/// système.
fn is_eui64(a: &Ipv6Addr) -> bool {
    let o = a.octets();
    o[11] == 0xff && o[12] == 0xfe
}

/// Les adresses que le système dit **temporaires** ou **dépréciées** :
/// sous Linux, `/proc/net/if_inet6` les marque (`IFA_F_TEMPORARY`,
/// `IFA_F_DEPRECATED`) ; sous macOS, `ifconfig` les dit en clair. Sous
/// Windows, rien de sûr à lire sans dépendance : les adresses formées sur
/// l'adresse matérielle passent d'abord.
fn temporary_or_deprecated() -> Vec<Ipv6Addr> {
    #[cfg(target_os = "linux")]
    {
        std::fs::read_to_string("/proc/net/if_inet6")
            .map(|t| flagged_in_if_inet6(&t))
            .unwrap_or_default()
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("/sbin/ifconfig")
            .output()
            .map(|o| flagged_in_ifconfig(&String::from_utf8_lossy(&o.stdout)))
            .unwrap_or_default()
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        Vec::new()
    }
}

/// Les adresses que `ifconfig` (macOS, BSD) marque `temporary` ou
/// `deprecated` : `inet6 <adresse>[%interface] prefixlen 64 autoconf
/// temporary`.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn flagged_in_ifconfig(text: &str) -> Vec<Ipv6Addr> {
    text.lines()
        .filter_map(|line| {
            let mut words = line.split_whitespace();
            if words.next()? != "inet6" {
                return None;
            }
            let address = words.next()?;
            let address = address.split('%').next()?.parse().ok()?;
            words
                .any(|w| w == "temporary" || w == "deprecated")
                .then_some(address)
        })
        .collect()
}

/// Les adresses marquées temporaires (0x01) ou dépréciées (0x20) dans le
/// texte de `/proc/net/if_inet6` : l'adresse en 32 chiffres hexadécimaux,
/// l'index, la longueur du préfixe, la portée, les marques, l'interface.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn flagged_in_if_inet6(text: &str) -> Vec<Ipv6Addr> {
    text.lines()
        .filter_map(|line| {
            let cols: Vec<&str> = line.split_whitespace().collect();
            let (hex, flags) = (cols.first()?, cols.get(4)?);
            let flags = u32::from_str_radix(flags, 16).ok()?;
            if flags & (0x01 | 0x20) == 0 || hex.len() != 32 {
                return None;
            }
            let mut o = [0_u8; 16];
            for (i, b) in o.iter_mut().enumerate() {
                *b = u8::from_str_radix(hex.get(i * 2..i * 2 + 2)?, 16).ok()?;
            }
            Some(Ipv6Addr::from(o))
        })
        .collect()
}

/// L'adresse IPv4 de cette machine du côté du routeur : celle par
/// laquelle un paquet vers lui partirait. Rien n'est envoyé.
fn local_v4_towards(gateway: SocketAddr) -> Option<Ipv4Addr> {
    let u = UdpSocket::bind("0.0.0.0:0").ok()?;
    u.connect(gateway).ok()?;
    match u.local_addr().ok()?.ip() {
        IpAddr::V4(a) => Some(a),
        IpAddr::V6(_) => None,
    }
}

/// **Demander au routeur d'ouvrir `port`** (TCP) vers cette machine, pour
/// [`LEASE`]. Rend l'adresse publique à composer, ou `None` : ni UPnP,
/// ni PCP, ni NAT-PMP chez le routeur, ou une adresse « publique » qui ne l'est
/// pas (CGNAT) — dans ce cas, l'IPv6 ou un transfert écrit à la main
/// dans la box.
pub fn open_mapping(port: u16) -> Option<SocketAddrV4> {
    open_mapping_of(igd_next::PortMappingProtocol::TCP, port, "BPM-Caddy postes")
}

/// [`open_mapping`] pour la porte des officines (le réseau ouvert, que
/// la DHT annonce) : le même geste, sous son propre nom dans la box.
pub fn open_officine_mapping(port: u16) -> Option<SocketAddrV4> {
    open_mapping_of(
        igd_next::PortMappingProtocol::TCP,
        port,
        "BPM-Caddy officines",
    )
}

/// **La porte web des officines** : le port 443 de la box renvoyé vers
/// la porte (`port`), par UPnP — pour l'officine qui appelle d'un réseau
/// d'hôpital ou d'entreprise, d'où seul le web sort
/// (`network::web_door`). Jamais pris à un autre : une box qui renvoie
/// déjà son 443 ailleurs refuse, et rien ne change. Rend l'adresse
/// publique, au port 443.
pub fn open_officine_web_mapping(port: u16) -> Option<SocketAddrV4> {
    let options = igd_next::SearchOptions {
        timeout: Some(Duration::from_secs(3)),
        ..Default::default()
    };
    let gateway = igd_next::search_gateway(options).ok()?;
    let local = local_v4_towards(gateway.addr)?;
    let web = crate::network::WEB_DOOR_PORT;
    // **Un 443 que la box renvoie déjà ailleurs reste à qui le tient** —
    // une autre machine, ou un serveur de celle-ci sur un autre port :
    // UPnP laisse une machine récrire sa propre ouverture, et la
    // refermer à la sortie aurait coupé ce serveur.
    let taken = (0..64)
        .map_while(|i| gateway.get_generic_port_mapping_entry(i).ok())
        .any(|e| {
            e.external_port == web
                && e.protocol == igd_next::PortMappingProtocol::TCP
                && !(e.internal_client == local.to_string() && e.internal_port == port)
        });
    if taken {
        return None;
    }
    gateway
        .add_port(
            igd_next::PortMappingProtocol::TCP,
            web,
            SocketAddr::V4(SocketAddrV4::new(local, port)),
            LEASE.as_secs() as u32,
            "BPM-Caddy officines (web)",
        )
        .ok()?;
    match gateway.get_external_ip() {
        Ok(IpAddr::V4(ip)) if is_public_v4(&ip) => Some(SocketAddrV4::new(ip, web)),
        // Sans adresse publique (CGNAT, double traduction), l'ouverture
        // ne mène nulle part : refermée aussitôt, sans attendre la sortie.
        _ => {
            let _ = gateway.remove_port(igd_next::PortMappingProtocol::TCP, web);
            None
        }
    }
}

/// Refermer ce que [`open_officine_web_mapping`] a ouvert — à n'appeler
/// que pour une ouverture accordée.
pub fn close_officine_web_mapping() {
    let options = igd_next::SearchOptions {
        timeout: Some(Duration::from_secs(2)),
        ..Default::default()
    };
    if let Ok(gateway) = igd_next::search_gateway(options) {
        let _ = gateway.remove_port(
            igd_next::PortMappingProtocol::TCP,
            crate::network::WEB_DOOR_PORT,
        );
    }
}

/// [`open_mapping`] en UDP : la voie directe des patients (`direct.rs`).
pub fn open_udp_mapping(port: u16) -> Option<SocketAddrV4> {
    open_mapping_of(
        igd_next::PortMappingProtocol::UDP,
        port,
        "BPM-Caddy patients",
    )
}

/// **L'adresse IPv4 de cette machine sur le réseau de l'officine** :
/// celle par laquelle partirait un paquet vers Internet (la route par
/// défaut). Rien n'est envoyé. Elle sert à écouter ce que la box traduit,
/// que le port y soit ouvert par UPnP ou à la main.
pub fn lan_v4() -> Option<Ipv4Addr> {
    let a = local_v4_towards(SocketAddr::from(([198, 51, 100, 1], 9)))?;
    (!a.is_loopback() && !a.is_unspecified() && !a.is_link_local()).then_some(a)
}

fn open_mapping_of(
    protocol: igd_next::PortMappingProtocol,
    port: u16,
    label: &str,
) -> Option<SocketAddrV4> {
    let options = igd_next::SearchOptions {
        timeout: Some(Duration::from_secs(3)),
        ..Default::default()
    };
    let upnp = || {
        let gateway = igd_next::search_gateway(options).ok()?;
        let local = local_v4_towards(gateway.addr)?;
        gateway
            .add_port(
                protocol,
                port,
                SocketAddr::V4(SocketAddrV4::new(local, port)),
                LEASE.as_secs() as u32,
                label,
            )
            .ok()?;
        match gateway.get_external_ip().ok()? {
            IpAddr::V4(ip) => Some(SocketAddrV4::new(ip, port)),
            IpAddr::V6(_) => None,
        }
    };
    // **Sans UPnP, PCP puis NAT-PMP** : les deux langages qu'un routeur
    // parle sur le port 5351 de sa propre adresse, plus simples, et que
    // certaines box parlent seuls.
    let tcp = protocol == igd_next::PortMappingProtocol::TCP;
    let found = upnp().or_else(|| pmp::open(tcp, port));
    found.filter(|a| is_public_v4(a.ip()))
}

/// Refermer ce que [`open_mapping`] a ouvert. Sans routeur, rien.
pub fn close_mapping(port: u16) {
    close_mapping_of(igd_next::PortMappingProtocol::TCP, port);
}

/// Refermer ce que [`open_udp_mapping`] a ouvert.
pub fn close_udp_mapping(port: u16) {
    close_mapping_of(igd_next::PortMappingProtocol::UDP, port);
}

fn close_mapping_of(protocol: igd_next::PortMappingProtocol, port: u16) {
    let options = igd_next::SearchOptions {
        timeout: Some(Duration::from_secs(2)),
        ..Default::default()
    };
    if let Ok(gateway) = igd_next::search_gateway(options) {
        let _ = gateway.remove_port(protocol, port);
    }
    pmp::close(protocol == igd_next::PortMappingProtocol::TCP, port);
}

/// **PCP (RFC 6887) et NAT-PMP (RFC 6886)** : demander au routeur, sur
/// son port 5351, d'ouvrir un port — ce qu'UPnP fait par HTTP et SSDP.
/// Les trames se lisent et s'écrivent ici sans réseau (et se testent
/// ainsi) ; seuls [`pmp::open`] et [`pmp::close`] parlent au routeur.
pub(crate) mod pmp {
    use std::collections::BTreeMap;
    use std::net::{Ipv4Addr, Ipv6Addr, SocketAddrV4, UdpSocket};
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    /// Le port où un routeur écoute PCP et NAT-PMP.
    pub const PORT: u16 = 5351;

    /// Une ouverture que le routeur a accordée : par quel langage, chez
    /// quel routeur — de quoi la refermer par le même chemin.
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub struct Granted {
        pub gateway: Ipv4Addr,
        pub pcp: bool,
        pub external: SocketAddrV4,
    }

    /// Ce que ce poste a ouvert par PCP ou NAT-PMP, par (TCP ?, port).
    static OPEN: Mutex<BTreeMap<(bool, u16), Granted>> = Mutex::new(BTreeMap::new());

    /// **Le nonce PCP d'une ouverture** : le même à chaque
    /// renouvellement, et d'un lancement à l'autre — un routeur refuse
    /// de renouveler ou de refermer une ouverture sous un autre nonce, et
    /// un nonce tiré au hasard à chaque lancement bloquerait le port
    /// jusqu'à la fin du bail précédent. Il ne protège que d'un tiers
    /// hors du réseau local, qui ne le voit pas passer.
    pub fn nonce(local: Ipv4Addr, tcp: bool, port: u16) -> [u8; 12] {
        let mut h = blake3::Hasher::new_derive_key("bpm-caddy/pcp-nonce/v1");
        h.update(&local.octets())
            .update(&[tcp as u8])
            .update(&port.to_be_bytes());
        let mut out = [0u8; 12];
        out.copy_from_slice(&h.finalize().as_bytes()[..12]);
        out
    }

    fn proto(tcp: bool) -> u8 {
        if tcp {
            6
        } else {
            17
        }
    }

    /// Une demande PCP `MAP` : `lifetime` 0 referme.
    pub fn pcp_request(
        local: Ipv4Addr,
        tcp: bool,
        port: u16,
        lifetime: u32,
        nonce: &[u8; 12],
    ) -> [u8; 60] {
        let mut b = [0u8; 60];
        b[0] = 2;
        b[1] = 1;
        b[4..8].copy_from_slice(&lifetime.to_be_bytes());
        b[8..24].copy_from_slice(&local.to_ipv6_mapped().octets());
        b[24..36].copy_from_slice(nonce);
        b[36] = proto(tcp);
        b[40..42].copy_from_slice(&port.to_be_bytes());
        b[42..44].copy_from_slice(&port.to_be_bytes());
        // Aucune adresse externe souhaitée : `::ffff:0.0.0.0`.
        b[44..60].copy_from_slice(&Ipv4Addr::UNSPECIFIED.to_ipv6_mapped().octets());
        b
    }

    /// La réponse PCP à [`pcp_request`] : l'adresse et le port accordés,
    /// ou `None` — un refus, une réponse à une autre demande, une adresse
    /// qui n'est pas IPv4.
    pub fn pcp_answer(b: &[u8], tcp: bool, port: u16, nonce: &[u8; 12]) -> Option<SocketAddrV4> {
        if b.len() < 60 || b[0] != 2 || b[1] != 0x81 || b[3] != 0 {
            return None;
        }
        if &b[24..36] != nonce || b[36] != proto(tcp) || b[40..42] != port.to_be_bytes() {
            return None;
        }
        let external = u16::from_be_bytes([b[42], b[43]]);
        let mut ip = [0u8; 16];
        ip.copy_from_slice(&b[44..60]);
        let ip = Ipv6Addr::from(ip).to_ipv4_mapped()?;
        Some(SocketAddrV4::new(ip, external))
    }

    /// Une demande NAT-PMP d'ouverture : `lifetime` 0 referme.
    pub fn pmp_request(tcp: bool, port: u16, lifetime: u32) -> [u8; 12] {
        let mut b = [0u8; 12];
        b[1] = if tcp { 2 } else { 1 };
        b[4..6].copy_from_slice(&port.to_be_bytes());
        // Refermer demande le port externe 0.
        let external = if lifetime == 0 { 0 } else { port };
        b[6..8].copy_from_slice(&external.to_be_bytes());
        b[8..12].copy_from_slice(&lifetime.to_be_bytes());
        b
    }

    /// Le port externe que la réponse NAT-PMP accorde.
    pub fn pmp_answer(b: &[u8], tcp: bool, port: u16) -> Option<u16> {
        let op = if tcp { 2 } else { 1 };
        if b.len() < 16 || b[0] != 0 || b[1] != 128 + op || b[2..4] != [0, 0] {
            return None;
        }
        (b[8..10] == port.to_be_bytes()).then(|| u16::from_be_bytes([b[10], b[11]]))
    }

    /// La demande NAT-PMP de l'adresse publique.
    pub const PMP_ADDRESS: [u8; 2] = [0, 0];

    /// L'adresse publique que la réponse NAT-PMP annonce.
    pub fn pmp_address(b: &[u8]) -> Option<Ipv4Addr> {
        if b.len() < 12 || b[0] != 0 || b[1] != 128 || b[2..4] != [0, 0] {
            return None;
        }
        Some(Ipv4Addr::new(b[8], b[9], b[10], b[11]))
    }

    /// **La passerelle par défaut, dans `/proc/net/route`** (Linux) : la
    /// ligne de destination `00000000`, son adresse en petit-boutiste.
    pub fn route_gateway(text: &str) -> Option<Ipv4Addr> {
        text.lines().skip(1).find_map(|line| {
            let f: Vec<&str> = line.split_whitespace().collect();
            if f.len() < 3 || f[1] != "00000000" {
                return None;
            }
            let g = u32::from_str_radix(f[2], 16).ok()?;
            (g != 0).then(|| Ipv4Addr::from(g.swap_bytes()))
        })
    }

    /// **Où chercher le routeur** : la passerelle que le système déclare
    /// quand il la dit (Linux), sinon les deux adresses où les box se
    /// tiennent d'habitude dans le réseau de l'officine — la première et
    /// la dernière de son /24 (`.1`, `.254`). Une demande à une machine
    /// qui n'est pas un routeur ne reçoit rien.
    pub fn candidates(lan: Ipv4Addr, declared: Option<Ipv4Addr>) -> Vec<Ipv4Addr> {
        let o = lan.octets();
        let mut out: Vec<Ipv4Addr> = declared.into_iter().collect();
        for last in [1, 254] {
            let a = Ipv4Addr::new(o[0], o[1], o[2], last);
            if a != lan && !out.contains(&a) {
                out.push(a);
            }
        }
        out
    }

    /// Envoie `request` à chaque candidat, à 250, 500 puis 1 000 ms
    /// (RFC 6886 § 3.1, écourté), et rend la première réponse qu'`accept`
    /// lit, avec le routeur qui l'a faite. Une réponse d'une autre
    /// adresse que celle demandée est ignorée.
    pub fn ask<T>(
        socket: &UdpSocket,
        to: &[Ipv4Addr],
        port: u16,
        request: &[u8],
        accept: impl Fn(&[u8]) -> Option<T>,
    ) -> Option<(Ipv4Addr, T)> {
        let mut buf = [0u8; 1100];
        for wait in [250u64, 500, 1000] {
            for g in to {
                let _ = socket.send_to(request, SocketAddrV4::new(*g, port));
            }
            let until = Instant::now() + Duration::from_millis(wait);
            while let Some(left) = until.checked_duration_since(Instant::now()) {
                if left.is_zero() || socket.set_read_timeout(Some(left)).is_err() {
                    break;
                }
                // Windows rend en erreur le « port fermé » qu'un
                // candidat qui n'est pas le routeur renvoie : le routeur,
                // lui, peut encore répondre.
                let (n, from) = match socket.recv_from(&mut buf) {
                    Ok(got) => got,
                    Err(e) if e.kind() == std::io::ErrorKind::ConnectionReset => continue,
                    Err(_) => break,
                };
                let std::net::SocketAddr::V4(from) = from else {
                    continue;
                };
                if from.port() != port || !to.contains(from.ip()) {
                    continue;
                }
                if let Some(t) = accept(&buf[..n]) {
                    return Some((*from.ip(), t));
                }
            }
        }
        None
    }

    fn socket() -> Option<(UdpSocket, Ipv4Addr, Vec<Ipv4Addr>)> {
        let lan = super::lan_v4()?;
        let declared = std::fs::read_to_string("/proc/net/route")
            .ok()
            .and_then(|t| route_gateway(&t));
        let socket = UdpSocket::bind(SocketAddrV4::new(lan, 0)).ok()?;
        Some((socket, lan, candidates(lan, declared)))
    }

    /// Ouvrir `port` vers ce poste, par PCP, sinon NAT-PMP.
    pub fn open(tcp: bool, port: u16) -> Option<SocketAddrV4> {
        let (socket, lan, to) = socket()?;
        let lifetime = super::LEASE.as_secs() as u32;
        let n = nonce(lan, tcp, port);
        let request = pcp_request(lan, tcp, port, lifetime, &n);
        let granted = match ask(&socket, &to, PORT, &request, |b| {
            pcp_answer(b, tcp, port, &n)
        }) {
            Some((gateway, external)) => Granted {
                gateway,
                pcp: true,
                external,
            },
            None => {
                let request = pmp_request(tcp, port, lifetime);
                let (gateway, external) =
                    ask(&socket, &to, PORT, &request, |b| pmp_answer(b, tcp, port))?;
                let (_, ip) = ask(&socket, &[gateway], PORT, &PMP_ADDRESS, pmp_address)?;
                Granted {
                    gateway,
                    pcp: false,
                    external: SocketAddrV4::new(ip, external),
                }
            }
        };
        if let Ok(mut open) = OPEN.lock() {
            open.insert((tcp, port), granted);
        }
        Some(granted.external)
    }

    /// Refermer ce que [`open`] a ouvert, chez le même routeur, par le
    /// même langage. Rien d'ouvert ainsi : rien n'est envoyé.
    pub fn close(tcp: bool, port: u16) {
        let Some(g) = OPEN.lock().ok().and_then(|mut o| o.remove(&(tcp, port))) else {
            return;
        };
        let Some(lan) = super::lan_v4() else {
            return;
        };
        let Ok(socket) = UdpSocket::bind(SocketAddrV4::new(lan, 0)) else {
            return;
        };
        if g.pcp {
            let request = pcp_request(lan, tcp, port, 0, &nonce(lan, tcp, port));
            let n = nonce(lan, tcp, port);
            let _ = ask(&socket, &[g.gateway], PORT, &request, |b| {
                pcp_answer(b, tcp, port, &n)
            });
        } else {
            let request = pmp_request(tcp, port, 0);
            let _ = ask(&socket, &[g.gateway], PORT, &request, |b| {
                pmp_answer(b, tcp, port)
            });
        }
    }
}

/// La ligne que le poste publie : ses adresses composables, IPv4 d'abord
/// (celle du routeur), puis IPv6, séparées par des virgules. Vide : rien
/// à composer depuis Internet.
pub fn reach_text(v4: Option<SocketAddrV4>, v6: &[Ipv6Addr], port: u16) -> String {
    let mut out: Vec<String> = Vec::new();
    if let Some(a) = v4 {
        out.push(a.to_string());
    }
    for a in v6.iter().filter(|a| is_global_v6(a)) {
        out.push(SocketAddr::from((*a, port)).to_string());
    }
    out.truncate(MOST);
    out.join(",")
}

/// Le préfixe d'une adresse de **relais** dans la ligne publiée : ce poste
/// attend là qu'on le demande (`relay.rs`).
pub const RELAY_TAG: &str = "relais:";

/// Ajouter à une ligne publiée les relais où ce poste attend.
pub fn with_relays(text: &str, relays: &[String]) -> String {
    let mut out: Vec<String> = text
        .split(',')
        .filter(|a| !a.trim().is_empty() && !a.starts_with(RELAY_TAG))
        .map(str::to_owned)
        .collect();
    out.extend(
        addresses(&relays.join(","))
            .into_iter()
            .map(|a| format!("{RELAY_TAG}{a}")),
    );
    out.truncate(MOST * 2);
    out.join(",")
}

/// La marque d'un poste qui **attend au rendez-vous** de son groupe
/// (`punch.rs`) : un téléphone qui ne le joint nulle part l'y appelle.
pub const MEET_TAG: &str = "rendez-vous";

/// Ajouter à une ligne publiée, ou en retirer, la marque du rendez-vous.
pub fn with_meeting(text: &str, meeting: bool) -> String {
    let mut out: Vec<&str> = text
        .split(',')
        .filter(|a| !a.trim().is_empty() && a.trim() != MEET_TAG)
        .collect();
    if meeting {
        out.push(MEET_TAG);
    }
    out.join(",")
}

/// Le préfixe de **l'identité au réseau d'officines** d'un poste : chaque
/// poste a sa base, donc la sienne ; dite ici, les autres postes du
/// groupe savent que c'est la leur (`network::own_posts`).
pub const NETWORK_TAG: &str = "reseau:";

/// Mettre dans une ligne publiée l'identité au réseau de ce poste (en
/// remplaçant celle d'avant), ou l'en retirer.
pub fn with_network(text: &str, device: Option<&str>) -> String {
    let mut out: Vec<String> = text
        .split(',')
        .map(str::trim)
        .filter(|a| !a.is_empty() && !a.starts_with(NETWORK_TAG))
        .map(str::to_owned)
        .collect();
    if let Some(d) = device.filter(|d| d.len() == 64 && d.bytes().all(|b| b.is_ascii_hexdigit())) {
        out.push(format!("{NETWORK_TAG}{}", d.to_ascii_lowercase()));
    }
    out.join(",")
}

/// L'identité au réseau que dit une ligne publiée.
pub fn network_of(text: &str) -> Option<String> {
    text.split(',')
        .filter_map(|a| a.trim().strip_prefix(NETWORK_TAG))
        .find(|d| d.len() == 64 && d.bytes().all(|b| b.is_ascii_hexdigit()))
        .map(str::to_ascii_lowercase)
}

/// Le poste de cette ligne attend-il au rendez-vous ?
pub fn meets(text: &str) -> bool {
    text.split(',').any(|a| a.trim() == MEET_TAG)
}

/// Les relais d'une ligne publiée — lus avec la même méfiance que les
/// adresses.
pub fn relays(text: &str) -> Vec<String> {
    let tagged: Vec<&str> = text
        .split(',')
        .filter_map(|a| a.trim().strip_prefix(RELAY_TAG))
        .collect();
    addresses(&tagged.join(","))
}

/// Relire une ligne publiée : les adresses qui en sont, au plus [`MOST`].
/// Ce qui ne se lit pas comme une adresse composable est laissé : la
/// ligne vient d'un autre poste, et un téléphone ne compose pas
/// n'importe quoi.
pub fn addresses(text: &str) -> Vec<String> {
    text.split(',')
        .filter(|a| !a.trim().starts_with(RELAY_TAG))
        .filter_map(|a| a.trim().parse::<SocketAddr>().ok())
        .filter(|a| match a.ip() {
            IpAddr::V4(v4) => is_public_v4(&v4),
            IpAddr::V6(v6) => is_global_v6(&v6),
        } && a.port() != 0)
        .take(MOST)
        .map(|a| a.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_says_the_posts_network_identity_once() {
        let d = "AB".repeat(32);
        let line = with_network("82.64.1.2:7743,rendez-vous", Some(&d));
        assert_eq!(network_of(&line), Some("ab".repeat(32)));
        assert_eq!(addresses(&line), vec!["82.64.1.2:7743"]);
        assert!(meets(&line));
        let again = with_network(&line, Some(&"cd".repeat(32)));
        assert_eq!(again.matches(NETWORK_TAG).count(), 1);
        assert_eq!(network_of(&again), Some("cd".repeat(32)));
        assert_eq!(network_of(&with_network(&line, None)), None);
        assert_eq!(network_of(&with_network("", Some("pas-hex"))), None);
        assert_eq!(network_of("reseau:zz"), None);
    }

    /// Ce que `ifconfig` dit sous macOS : une adresse temporaire, une
    /// dépréciée, une stable (`secured`), une de lien local.
    #[test]
    fn temporary_addresses_are_read_from_ifconfig() {
        let text = "en0: flags=8863<UP,BROADCAST,SMART,RUNNING,SIMPLEX,MULTICAST> mtu 1500\n\
            \tinet6 fe80::1c2a:3b4c:5d6e:7f80%en0 prefixlen 64 secured scopeid 0x6\n\
            \tinet6 2a01:cb00:1:2:1c2a:3b4c:5d6e:7f80 prefixlen 64 autoconf secured\n\
            \tinet6 2a01:cb00:1:2:9d8e:7f6a:5b4c:3d2e prefixlen 64 autoconf temporary\n\
            \tinet6 2a01:cb00:1:2:aaaa:bbbb:cccc:dddd prefixlen 64 deprecated autoconf temporary\n\
            \tinet6 2a01:cb00:9:9::5 prefixlen 64 deprecated\n\
            \tinet 192.168.1.20 netmask 0xffffff00 broadcast 192.168.1.255\n";
        let got = flagged_in_ifconfig(text);
        let want: Vec<Ipv6Addr> = [
            "2a01:cb00:1:2:9d8e:7f6a:5b4c:3d2e",
            "2a01:cb00:1:2:aaaa:bbbb:cccc:dddd",
            "2a01:cb00:9:9::5",
        ]
        .iter()
        .map(|a| a.parse().unwrap())
        .collect();
        assert_eq!(got, want);
    }

    /// **PCP, à l'octet près** (RFC 6887 § 7 et § 11) : la demande porte
    /// l'adresse du poste en IPv4 dans IPv6, le nonce, le protocole et le
    /// port ; la réponse ne compte que si elle répond à *cette* demande —
    /// même nonce, même protocole, même port interne, succès.
    #[test]
    fn a_pcp_mapping_is_asked_and_read_byte_for_byte() {
        let lan = Ipv4Addr::new(192, 168, 1, 20);
        let n = pmp::nonce(lan, true, 7790);
        assert_eq!(
            n,
            pmp::nonce(lan, true, 7790),
            "le même d'un lancement à l'autre"
        );
        assert_ne!(n, pmp::nonce(lan, false, 7790));
        assert_ne!(n, pmp::nonce(lan, true, 7791));
        let q = pmp::pcp_request(lan, true, 7790, 3600, &n);
        assert_eq!(&q[..4], &[2, 1, 0, 0], "version 2, MAP");
        assert_eq!(&q[4..8], &3600u32.to_be_bytes());
        assert_eq!(&q[8..24], &lan.to_ipv6_mapped().octets());
        assert_eq!(&q[24..36], &n);
        assert_eq!(q[36], 6, "TCP");
        assert_eq!(&q[40..44], &[0x1e, 0x6e, 0x1e, 0x6e]);
        assert_eq!(&q[44..60], &Ipv4Addr::UNSPECIFIED.to_ipv6_mapped().octets());

        // La réponse du routeur : la demande, retournée, port 40000 accordé.
        let public = Ipv4Addr::new(81, 2, 3, 4);
        let mut r = q;
        r[1] = 0x81;
        r[8..24].fill(0);
        r[42..44].copy_from_slice(&40000u16.to_be_bytes());
        r[44..60].copy_from_slice(&public.to_ipv6_mapped().octets());
        assert_eq!(
            pmp::pcp_answer(&r, true, 7790, &n),
            Some(SocketAddrV4::new(public, 40000)),
            "le port accordé, pas celui demandé"
        );
        let mut refused = r;
        refused[3] = 2;
        assert_eq!(
            pmp::pcp_answer(&refused, true, 7790, &n),
            None,
            "NOT_AUTHORIZED"
        );
        let other = pmp::nonce(lan, true, 9999);
        assert_eq!(
            pmp::pcp_answer(&r, true, 7790, &other),
            None,
            "un autre nonce"
        );
        assert_eq!(
            pmp::pcp_answer(&r, false, 7790, &n),
            None,
            "un autre protocole"
        );
        assert_eq!(pmp::pcp_answer(&r, true, 7791, &n), None, "un autre port");
        assert_eq!(pmp::pcp_answer(&q, true, 7790, &n), None, "une demande");
        assert_eq!(pmp::pcp_answer(&r[..59], true, 7790, &n), None, "tronquée");
        let mut v6 = r;
        v6[44..60].copy_from_slice(&"2001:db8::1".parse::<Ipv6Addr>().unwrap().octets());
        assert_eq!(pmp::pcp_answer(&v6, true, 7790, &n), None, "pas en IPv4");
    }

    /// **NAT-PMP** (RFC 6886 § 3) : l'ouverture, la fermeture (port
    /// externe 0, bail 0), l'adresse publique, et rien d'autre ne se lit
    /// comme une réponse.
    #[test]
    fn a_nat_pmp_mapping_is_asked_and_read_byte_for_byte() {
        let q = pmp::pmp_request(false, 7791, 3600);
        assert_eq!(q, [0, 1, 0, 0, 0x1e, 0x6f, 0x1e, 0x6f, 0, 0, 0x0e, 0x10]);
        let close = pmp::pmp_request(true, 7791, 0);
        assert_eq!(close, [0, 2, 0, 0, 0x1e, 0x6f, 0, 0, 0, 0, 0, 0]);
        let answer = [
            0, 129, 0, 0, 0, 0, 1, 0, 0x1e, 0x6f, 0x9c, 0x40, 0, 0, 0x0e, 0x10,
        ];
        assert_eq!(pmp::pmp_answer(&answer, false, 7791), Some(40000));
        assert_eq!(
            pmp::pmp_answer(&answer, true, 7791),
            None,
            "UDP répondu, TCP demandé"
        );
        assert_eq!(pmp::pmp_answer(&answer, false, 7790), None);
        let mut failed = answer;
        failed[3] = 3;
        assert_eq!(
            pmp::pmp_answer(&failed, false, 7791),
            None,
            "Network Failure"
        );
        assert_eq!(pmp::pmp_answer(&answer[..15], false, 7791), None);

        assert_eq!(pmp::PMP_ADDRESS, [0, 0]);
        let address = [0, 128, 0, 0, 0, 0, 1, 0, 81, 2, 3, 4];
        assert_eq!(pmp::pmp_address(&address), Some(Ipv4Addr::new(81, 2, 3, 4)));
        assert_eq!(
            pmp::pmp_address(&answer),
            None,
            "une ouverture n'est pas une adresse"
        );
    }

    /// **Une demande se répète jusqu'à la réponse**, et seule compte celle
    /// du routeur qu'on a interrogé, à cette demande : un faux routeur
    /// sur la boucle locale qui se tait une fois, puis répond d'abord
    /// faux, puis juste.
    #[test]
    fn a_router_is_asked_again_and_only_its_own_answer_counts() {
        let router = UdpSocket::bind("127.0.0.1:0").unwrap();
        let port = router.local_addr().unwrap().port();
        let lan = Ipv4Addr::LOCALHOST;
        let n = pmp::nonce(lan, true, 7790);
        let q = pmp::pcp_request(lan, true, 7790, 3600, &n);
        let served = std::thread::spawn(move || {
            let mut buf = [0u8; 100];
            let (_, _) = router.recv_from(&mut buf).unwrap();
            let (len, from) = router.recv_from(&mut buf).unwrap();
            let mut r = [0u8; 60];
            r.copy_from_slice(&buf[..len]);
            r[1] = 0x81;
            r[44..60].copy_from_slice(&Ipv4Addr::new(81, 2, 3, 4).to_ipv6_mapped().octets());
            let mut wrong = r;
            wrong[24] ^= 1;
            router.send_to(&wrong, from).unwrap();
            router.send_to(&r, from).unwrap();
            len
        });
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let got = pmp::ask(&socket, &[lan], port, &q, |b| {
            pmp::pcp_answer(b, true, 7790, &n)
        });
        assert_eq!(served.join().unwrap(), 60);
        assert_eq!(
            got,
            Some((lan, SocketAddrV4::new(Ipv4Addr::new(81, 2, 3, 4), 7790)))
        );

        // Personne ne répond : rien, en moins de deux secondes.
        let silent = UdpSocket::bind("127.0.0.1:0").unwrap();
        let port = silent.local_addr().unwrap().port();
        let started = std::time::Instant::now();
        let got = pmp::ask(&socket, &[lan], port, &q, |b| {
            pmp::pcp_answer(b, true, 7790, &n)
        });
        assert_eq!(got, None);
        assert!(started.elapsed() < Duration::from_secs(2));
    }

    /// Le routeur se cherche à la passerelle que Linux déclare, puis aux
    /// deux adresses habituelles des box — jamais à ce poste lui-même.
    #[test]
    fn the_router_is_looked_for_where_routers_stand() {
        let table = "Iface\tDestination\tGateway \tFlags\tRefCnt\tUse\tMetric\tMask\n\
                     eth0\t0001A8C0\t00000000\t0001\t0\t0\t100\t00FFFFFF\n\
                     eth0\t00000000\tFE01A8C0\t0003\t0\t0\t100\t00000000\n";
        let declared = pmp::route_gateway(table);
        assert_eq!(declared, Some(Ipv4Addr::new(192, 168, 1, 254)));
        assert_eq!(pmp::route_gateway("Iface Destination Gateway\n"), None);

        let lan = Ipv4Addr::new(192, 168, 1, 20);
        assert_eq!(
            pmp::candidates(lan, declared),
            vec![
                Ipv4Addr::new(192, 168, 1, 254),
                Ipv4Addr::new(192, 168, 1, 1)
            ]
        );
        assert_eq!(
            pmp::candidates(lan, None),
            vec![
                Ipv4Addr::new(192, 168, 1, 1),
                Ipv4Addr::new(192, 168, 1, 254)
            ]
        );
        let router_itself = Ipv4Addr::new(10, 0, 0, 1);
        assert_eq!(
            pmp::candidates(router_itself, None),
            vec![Ipv4Addr::new(10, 0, 0, 254)]
        );
    }

    /// Les marques de `/proc/net/if_inet6` se lisent : une adresse
    /// temporaire ou dépréciée est écartée, une stable reste.
    #[test]
    fn temporary_addresses_are_read_from_the_kernel_table() {
        let text = "2a01cb0000010002021122fffe334455 02 40 00 00 eth0\n\
                    2a01cb000001000212345678abcdef01 02 40 00 01 eth0\n\
                    2a01cb000001000287654321abcdef02 02 40 00 20 eth0\n\
                    fe80000000000000021122fffe334455 02 40 20 80 eth0\n";
        let flagged = flagged_in_if_inet6(text);
        assert_eq!(flagged.len(), 2);
        assert!(flagged.contains(&"2a01:cb00:1:2:1234:5678:abcd:ef01".parse().unwrap()));
        assert!(flagged.contains(&"2a01:cb00:1:2:8765:4321:abcd:ef02".parse().unwrap()));
        assert!(is_eui64(
            &"2a01:cb00:1:2:211:22ff:fe33:4455".parse().unwrap()
        ));
        assert!(!is_eui64(
            &"2a01:cb00:1:2:1234:5678:abcd:ef01".parse().unwrap()
        ));
    }

    #[test]
    fn only_addresses_the_internet_can_reach_are_published() {
        for a in ["2a01:e0a:1::5", "2001:41d0::1"] {
            assert!(is_global_v6(&a.parse().unwrap()), "{a}");
        }
        for a in [
            "::1",
            "fe80::1",
            "fd00::1",
            "2001:db8::1",
            "ff02::1",
            "2001:0:4136:e378::1",
            "2002:c000:204::1",
        ] {
            assert!(!is_global_v6(&a.parse().unwrap()), "{a}");
        }
        for a in ["82.64.1.2", "5.6.7.8"] {
            assert!(is_public_v4(&a.parse().unwrap()), "{a}");
        }
        for a in [
            "192.168.1.2",
            "10.0.0.1",
            "172.16.0.1",
            "100.64.1.1",
            "100.127.255.1",
            "127.0.0.1",
            "169.254.1.1",
            "203.0.113.5",
            "0.1.2.3",
            "192.0.0.8",
            "198.18.0.1",
            "240.0.0.1",
        ] {
            assert!(!is_public_v4(&a.parse().unwrap()), "{a}");
        }
    }

    #[test]
    fn a_published_line_reads_back_as_what_can_be_dialled() {
        let v6: Vec<Ipv6Addr> = vec!["2a01:e0a:1::5".parse().unwrap(), "fe80::1".parse().unwrap()];
        let text = reach_text(Some("82.64.1.2:7743".parse().unwrap()), &v6, 7743);
        assert_eq!(text, "82.64.1.2:7743,[2a01:e0a:1::5]:7743");
        assert_eq!(
            addresses(&text),
            vec![
                "82.64.1.2:7743".to_owned(),
                "[2a01:e0a:1::5]:7743".to_owned()
            ]
        );
        // What another post wrote is read with suspicion.
        assert!(addresses("192.168.1.2:7743,n'importe quoi,127.0.0.1:1,[::1]:7743").is_empty());
        assert_eq!(reach_text(None, &[], 7743), "");
        let many = (0..20)
            .map(|i| format!("82.64.1.{i}:7743"))
            .collect::<Vec<_>>()
            .join(",");
        assert_eq!(addresses(&many).len(), MOST);
    }

    /// Relays travel in the same line, tagged, read apart from the
    /// post's own addresses and with the same suspicion.
    #[test]
    fn relays_ride_in_the_published_line_and_read_apart() {
        let line = with_relays(
            "82.64.1.2:7743,relais:5.5.5.5:7745",
            &["5.6.7.8:7745".into(), "192.168.1.9:7745".into()],
        );
        assert_eq!(line, "82.64.1.2:7743,relais:5.6.7.8:7745");
        assert_eq!(addresses(&line), vec!["82.64.1.2:7743".to_owned()]);
        assert_eq!(relays(&line), vec!["5.6.7.8:7745".to_owned()]);
        assert_eq!(with_relays("", &[]), "");
    }
}
