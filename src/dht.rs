//! **Le réseau ouvert au-delà du réseau local** : les officines se
//! trouvent par la DHT publique de BitTorrent (« mainline »), sans
//! serveur à tenir — ni par l'éditeur, ni par une officine.
//!
//! Chacune annonce, sous un même sujet tiré d'un texte fixe
//! ([`topic`]), l'adresse où sa porte des officines répond — quand elle
//! en a une : son routeur a ouvert le port (UPnP), ou elle a une adresse
//! publique. Chacune demande ensuite qui d'autre s'y annonce, et les
//! compose au réseau ouvert (`network::dial_found`). Une officine sans
//! adresse joignable compose les autres quand même : une conversation
//! va dans les deux sens.
//!
//! **Ce que la DHT apprend** : qu'une adresse IPv4 parle sous ce sujet —
//! rien d'autre ; ni le nom de l'officine, ni ce qu'elle partage, qui ne
//! passe que dans les conversations chiffrées. **Ce qu'une adresse
//! trouvée vaut** : rien de plus qu'une adresse — n'importe qui peut s'y
//! annoncer. La conversation dit qui répond ; ce qu'elle apporte est lu
//! comme tout ce qui vient du réseau ouvert (proposé, borné, jamais
//! appliqué de soi-même), et l'on n'en compose que [`MOST_DIALS`] par
//! tour, chacune au plus une fois par [`REDIAL`].

use std::collections::HashMap;
use std::net::SocketAddrV4;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Le sujet sous lequel les officines s'annoncent : vingt octets tirés
/// d'un texte fixe, les mêmes partout.
pub fn topic() -> mainline::Id {
    let key = blake3::derive_key("bpm-caddy 2026-10-16 reseau ouvert dht v1", b"");
    let mut id = [0_u8; 20];
    id.copy_from_slice(&key[..20]);
    mainline::Id::from(id)
}

/// Combien d'adresses trouvées on compose au plus par tour.
pub const MOST_DIALS: usize = 6;
/// Une même adresse n'est composée de nouveau qu'après ce délai.
pub const REDIAL: Duration = Duration::from_secs(6 * 3600);
/// Entre deux tours : annoncer, chercher, composer.
pub const ROUND: Duration = Duration::from_secs(30 * 60);
/// Combien d'adresses trouvées on lit au plus par recherche.
const MOST_FOUND: usize = 256;

/// Un nœud de la DHT, client seulement : il ne garde rien pour les
/// autres.
pub struct Finder {
    dht: mainline::Dht,
}

impl Finder {
    /// Rejoindre la DHT publique, ou un réseau d'essai (`bootstrap`).
    pub fn start(bootstrap: Option<&[String]>) -> std::io::Result<Self> {
        let mut b = mainline::Dht::builder();
        if let Some(nodes) = bootstrap {
            b.bootstrap(nodes);
        }
        Ok(Self { dht: b.build()? })
    }

    /// Annoncer que la porte des officines répond sur `port`, à l'adresse
    /// d'où partent nos paquets — celle que la DHT voit.
    #[allow(deprecated)]
    pub fn announce(&self, port: u16) -> bool {
        self.dht.announce_peer(topic(), Some(port)).is_ok()
    }

    /// Les adresses qui s'annoncent sous le sujet, au plus
    /// [`MOST_FOUND`], sans doublon.
    #[allow(deprecated)]
    pub fn find(&self) -> Vec<SocketAddrV4> {
        let mut out: Vec<SocketAddrV4> = Vec::new();
        for batch in self.dht.get_peers(topic()) {
            for a in batch {
                if !out.contains(&a) {
                    out.push(a);
                }
            }
            if out.len() >= MOST_FOUND {
                out.truncate(MOST_FOUND);
                break;
            }
        }
        out
    }
}

/// **Celles à composer à ce tour** : ni la nôtre, ni une adresse qu'on
/// ne composerait pas (`network::usable_announced` : ni boucle locale,
/// ni réseau privé de lien, ni multidiffusion), ni une composée depuis
/// moins de [`REDIAL`] ; au plus [`MOST_DIALS`]. `dialed` oublie ce qui
/// a passé le délai.
pub fn to_dial(
    found: &[SocketAddrV4],
    mine: Option<SocketAddrV4>,
    dialed: &mut HashMap<SocketAddrV4, Instant>,
    now: Instant,
) -> Vec<SocketAddrV4> {
    dialed.retain(|_, at| now.duration_since(*at) < REDIAL);
    let mut out = Vec::new();
    for a in found {
        if out.len() >= MOST_DIALS {
            break;
        }
        if Some(*a) == mine
            || dialed.contains_key(a)
            || crate::network::usable_announced(&a.to_string()).is_none()
        {
            continue;
        }
        dialed.insert(*a, now);
        out.push(*a);
    }
    out
}

/// **Le fil de la DHT**, tant que `on` tient : à chaque tour, ouvrir la
/// porte des officines au routeur (UPnP), s'annoncer si elle s'ouvre,
/// chercher les autres et en composer quelques-unes au réseau ouvert.
/// Rien quand l'officine a quitté le réseau ouvert. Ce qu'il dit passe
/// par `said`.
pub fn spawn(
    path: std::path::PathBuf,
    password: String,
    port: u16,
    officine: String,
    on: Arc<AtomicBool>,
    said: impl Fn(String) + Send + 'static,
) {
    std::thread::spawn(move || {
        // **Un seul fil de la DHT par processus** : celui d'un fil des
        // postes relancé attend que l'ancien ait fini son tour — sans quoi
        // l'ancien, en partant, refermait au routeur la porte que le
        // nouveau venait d'ouvrir.
        static ONE: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _one = ONE
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut finder: Option<Finder> = None;
        let mut dialed: HashMap<SocketAddrV4, Instant> = HashMap::new();
        let mut mapped = false;
        let mut next = Instant::now();
        while on.load(Ordering::SeqCst) {
            if Instant::now() < next {
                std::thread::sleep(Duration::from_secs(1));
                continue;
            }
            next = Instant::now() + ROUND;
            let Ok(db) = crate::db::Db::open(&path, &password) else {
                continue;
            };
            // Le réseau ouvert quitté : la porte se referme au routeur.
            if !crate::network::in_open(&db) {
                if std::mem::take(&mut mapped) {
                    crate::reach::close_mapping(port);
                }
                finder = None;
                continue;
            }
            if finder.is_none() {
                finder = Finder::start(None).ok();
            }
            let Some(f) = &finder else {
                continue;
            };
            let mine = crate::reach::open_officine_mapping(port);
            mapped |= mine.is_some();
            if let Some(at) = mine {
                f.announce(at.port());
            }
            for a in to_dial(&f.find(), mine, &mut dialed, Instant::now()) {
                if !on.load(Ordering::SeqCst) {
                    break;
                }
                if let Ok(line) = crate::network::dial_found(&db, &a.to_string(), &officine) {
                    said(line);
                }
            }
        }
        if mapped {
            crate::reach::close_mapping(port);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Les officines se trouvent par la DHT** : sur un réseau d'essai,
    /// deux s'annoncent, une troisième les trouve toutes les deux.
    #[test]
    fn officines_find_each_other_through_the_dht() {
        let testnet = mainline::Testnet::new(8).unwrap();
        let a = Finder::start(Some(&testnet.bootstrap)).unwrap();
        let b = Finder::start(Some(&testnet.bootstrap)).unwrap();
        let c = Finder::start(Some(&testnet.bootstrap)).unwrap();
        assert!(a.announce(17745));
        assert!(b.announce(27745));
        let found = c.find();
        let ports: Vec<u16> = found.iter().map(SocketAddrV4::port).collect();
        assert!(
            ports.contains(&17745) && ports.contains(&27745),
            "{found:?}"
        );
    }

    /// **Ce qu'on compose d'un tour** : pas la nôtre, pas une adresse
    /// locale, au plus six, et pas deux fois avant le délai.
    #[test]
    fn a_round_dials_a_few_new_public_addresses_once() {
        let public = |i: u8| SocketAddrV4::new([203, 0, 113, i].into(), 7745);
        let mut found: Vec<SocketAddrV4> = (1..=9).map(public).collect();
        found.push(SocketAddrV4::new([127, 0, 0, 1].into(), 7745));
        let mut dialed = HashMap::new();
        let now = Instant::now();
        let first = to_dial(&found, Some(public(1)), &mut dialed, now);
        assert_eq!(first, (2..=7).map(public).collect::<Vec<_>>());
        let second = to_dial(&found, Some(public(1)), &mut dialed, now);
        assert_eq!(second, vec![public(8), public(9)]);
        assert!(to_dial(&found, None, &mut dialed, now)
            .iter()
            .all(|a| *a == public(1)));
        let later = now + REDIAL + Duration::from_secs(1);
        assert_eq!(to_dial(&found, None, &mut dialed, later).len(), MOST_DIALS);
    }
}
