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
//!   IGD), et lire l'adresse publique qu'il annonce ;
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
        .filter_map(|i| match i.ip() {
            IpAddr::V6(a) if is_global_v6(&a) => Some(a),
            _ => None,
        })
        .collect();
    out.sort();
    out.dedup();
    out
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
/// [`LEASE`]. Rend l'adresse publique à composer, ou `None` : pas de
/// routeur UPnP, UPnP refusé, ou une adresse « publique » qui ne l'est
/// pas (CGNAT) — dans ce cas, l'IPv6 ou un transfert écrit à la main
/// dans la box.
pub fn open_mapping(port: u16) -> Option<SocketAddrV4> {
    let options = igd_next::SearchOptions {
        timeout: Some(Duration::from_secs(3)),
        ..Default::default()
    };
    let gateway = igd_next::search_gateway(options).ok()?;
    let local = local_v4_towards(gateway.addr)?;
    gateway
        .add_port(
            igd_next::PortMappingProtocol::TCP,
            port,
            SocketAddr::V4(SocketAddrV4::new(local, port)),
            LEASE.as_secs() as u32,
            "BPM-Caddy postes",
        )
        .ok()?;
    match gateway.get_external_ip().ok()? {
        IpAddr::V4(ip) if is_public_v4(&ip) => Some(SocketAddrV4::new(ip, port)),
        _ => None,
    }
}

/// Refermer ce que [`open_mapping`] a ouvert. Sans routeur, rien.
pub fn close_mapping(port: u16) {
    let options = igd_next::SearchOptions {
        timeout: Some(Duration::from_secs(2)),
        ..Default::default()
    };
    if let Ok(gateway) = igd_next::search_gateway(options) {
        let _ = gateway.remove_port(igd_next::PortMappingProtocol::TCP, port);
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
