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

/// **Le sujet propre à une officine** : tiré de sa clé de réseau (`device`,
/// en hexadécimal). Elle s'y annonce aussi, si bien qu'une officine
/// choisie — sur la carte, par son empreinte — se cherche là, sans
/// parcourir toutes les autres. N'importe qui peut s'y annoncer aussi :
/// on n'y compose qu'en attendant **cette** officine, et une autre qui
/// répondrait échoue à la poignée de main.
pub fn own_topic(device: &str) -> mainline::Id {
    let key = blake3::derive_key(
        "bpm-caddy 2026-10-21 officine dht v1",
        device.trim().to_ascii_lowercase().as_bytes(),
    );
    let mut id = [0_u8; 20];
    id.copy_from_slice(&key[..20]);
    mainline::Id::from(id)
}

/// Combien d'adresses trouvées on compose au plus par tour.
pub const MOST_DIALS: usize = 6;
/// Une même adresse n'est composée de nouveau qu'après ce délai.
pub const REDIAL: Duration = Duration::from_secs(6 * 3600);
/// Entre deux tours : annoncer, chercher, composer. Une annonce vit une
/// demi-heure dans la DHT : un tour de dix minutes la tient toujours, et
/// une officine qui arrive est trouvée dans le quart d'heure.
pub const ROUND: Duration = Duration::from_secs(10 * 60);
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

    /// [`Finder::announce`], et sous le sujet propre à l'officine
    /// ([`own_topic`]) : qui la choisit la trouve là.
    #[allow(deprecated)]
    pub fn announce_as(&self, device: &str, port: u16) -> bool {
        let shared = self.announce(port);
        let own = self
            .dht
            .announce_peer(own_topic(device), Some(port))
            .is_ok();
        shared && own
    }

    /// **La DHT répond-elle d'ici ?** Une recherche de voisins aboutie :
    /// sans réponse, l'UDP sortant est bloqué (un réseau d'hôpital ou
    /// d'entreprise) ou la machine est hors ligne. Bloque le temps de la
    /// recherche.
    #[allow(deprecated)]
    pub fn reachable(&self) -> bool {
        self.dht.bootstrapped()
    }

    /// Les adresses qui s'annoncent sous le sujet d'une officine, au plus
    /// [`MOST_DIALS`].
    #[allow(deprecated)]
    pub fn find_officine(&self, device: &str) -> Vec<SocketAddrV4> {
        let mut out: Vec<SocketAddrV4> = Vec::new();
        for batch in self.dht.get_peers(own_topic(device)) {
            for a in batch {
                if !out.contains(&a) {
                    out.push(a);
                }
            }
            if out.len() >= MOST_DIALS {
                out.truncate(MOST_DIALS);
                break;
            }
        }
        out
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
    web_door: bool,
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
        let mut web = false;
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
                if std::mem::take(&mut web) {
                    crate::reach::close_officine_web_mapping();
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
            let opened = crate::reach::open_officine_mapping(port);
            mapped |= opened.is_some();
            // La porte web aussi, quand la box l'accorde : ceux qui
            // appellent d'un réseau qui ne laisse sortir que le web la
            // composent d'eux-mêmes (`network::web_door`).
            if opened.is_some() && web_door {
                web |= crate::reach::open_officine_web_mapping(port).is_some();
            }
            // Sans routeur qui ouvre : la porte ouverte à la main dans la
            // box, que l'officine a écrite (« Adresse publique »).
            let mine = opened.or_else(|| {
                written_door(
                    &db.setting(crate::network::PUBLIC_ADDRESS)
                        .unwrap_or_default(),
                )
            });
            if let Some(at) = mine {
                match crate::network::device_hex(&db) {
                    Ok(me) => f.announce_as(&me, at.port()),
                    Err(_) => f.announce(at.port()),
                };
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
        if web {
            crate::reach::close_officine_web_mapping();
        }
    });
}

/// **L'adresse publique écrite par l'officine**, quand c'est une IPv4
/// publique et un port : ce qu'elle a ouvert à la main dans sa box, et
/// qu'elle peut annoncer.
pub fn written_door(text: &str) -> Option<SocketAddrV4> {
    let at: SocketAddrV4 = text.trim().parse().ok()?;
    (crate::reach::is_public_v4(at.ip()) && at.port() != 0).then_some(at)
}

/// **Joindre une officine choisie** (`device`, sa clé de réseau) au réseau
/// ouvert, sans code : d'abord là où elle a déjà répondu, puis aux
/// adresses qui s'annoncent sous son sujet dans la DHT — en n'acceptant
/// qu'**elle** (`network::dial_open`) : une adresse qu'un autre y aurait
/// mise ne mène qu'à une poignée de main refusée. `bootstrap` : un réseau
/// d'essai.
pub fn reach_officine(
    db: &crate::db::Db,
    device: &str,
    officine: &str,
    bootstrap: Option<&[String]>,
) -> Result<String, String> {
    use crate::strings::tr;
    if !crate::network::in_open(db) {
        return Err(tr("net_err_no_network").to_owned());
    }
    let mut last = tr("carte_join_not_found").to_owned();
    if let Some(at) = db.setting(&crate::network::reached_key(device)) {
        match crate::network::dial_open(db, device, &at, officine) {
            Ok(said) => return Ok(said),
            Err(e) => last = e,
        }
    }
    let finder = Finder::start(bootstrap).map_err(|e| e.to_string())?;
    // Un réseau d'essai tient sur la boucle locale, qu'on ne compose
    // jamais autrement.
    let testing = bootstrap.is_some();
    for a in finder.find_officine(device) {
        // Public addresses only: a DHT node does not point this officine's
        // dials into its own network.
        if !testing && !crate::reach::is_public_v4(a.ip()) {
            continue;
        }
        match crate::network::dial_open(db, device, &a.to_string(), officine) {
            Ok(said) => return Ok(said),
            Err(e) => last = e,
        }
    }
    Err(last)
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

    /// **De bout en bout** : A tient sa porte des officines et s'annonce ;
    /// B, qui ne la connaît pas, la trouve dans la DHT, la compose au
    /// réseau ouvert, et chacune inscrit l'autre — sans adresse saisie,
    /// sans code, sans serveur.
    #[test]
    fn an_officine_found_in_the_dht_is_dialled_and_met() {
        let dir = |tag: &str| {
            let d =
                std::env::temp_dir().join(format!("bpm-caddy-dht-{tag}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&d);
            std::fs::create_dir_all(&d).unwrap();
            d
        };
        let (dir_a, dir_b) = (dir("a"), dir("b"));
        let (_sa, _sb) = (
            crate::db::Swept(dir_a.clone()),
            crate::db::Swept(dir_b.clone()),
        );
        let a = crate::db::Db::open(&dir_a.join("net.db"), "secret").unwrap();
        let b = crate::db::Db::open(&dir_b.join("net.db"), "secret").unwrap();
        for db in [&a, &b] {
            crate::network::join_open(db, "2026-10-16").unwrap();
        }
        let (a_dev, b_dev) = (
            crate::network::device_hex(&a).unwrap(),
            crate::network::device_hex(&b).unwrap(),
        );
        let door = bpm_sync::link::Door::open("127.0.0.1:0").unwrap();
        let port = door.address().unwrap().port();
        let path_a = dir_a.join("net.db");
        let answering = std::thread::spawn(move || {
            let db = crate::db::Db::open(&path_a, "secret").unwrap();
            let mut link = door.accept(Duration::from_secs(30)).unwrap();
            crate::network::answer(&db, &mut link)
        });
        let testnet = mainline::Testnet::new(8).unwrap();
        let finder_a = Finder::start(Some(&testnet.bootstrap)).unwrap();
        let finder_b = Finder::start(Some(&testnet.bootstrap)).unwrap();
        assert!(finder_a.announce(port));
        let found = finder_b.find();
        let at = found.iter().find(|a| a.port() == port).expect("trouvée");
        crate::network::dial_found(&b, &at.to_string(), "Pharmacie B").expect("jointe");
        assert_eq!(answering.join().unwrap(), Ok(b_dev.clone()));
        let open = crate::network::open_id();
        assert!(b
            .net_members(&open)
            .unwrap()
            .iter()
            .any(|p| p.device == a_dev));
        assert!(a
            .net_members(&open)
            .unwrap()
            .iter()
            .any(|p| p.device == b_dev));
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

    /// Ce qu'on annonce sans routeur qui ouvre : une IPv4 publique et un
    /// port écrits, rien d'autre.
    #[test]
    fn a_door_opened_by_hand_is_announced() {
        assert_eq!(
            written_door(" 82.64.1.2:7745 "),
            Some(SocketAddrV4::new([82, 64, 1, 2].into(), 7745))
        );
        for no in [
            "",
            "192.168.1.20:7745",
            "100.64.0.9:7745",
            "82.64.1.2",
            "nom:7745",
            "[2a01::1]:7745",
        ] {
            assert_eq!(written_door(no), None, "{no}");
        }
    }

    /// **Une officine choisie se joint sans code** : A s'annonce sous son
    /// sujet ; un leurre, une autre officine, s'annonce sous le même
    /// sujet que A. B, qui ne connaît ni l'une ni l'autre, cherche A par
    /// sa clé : elle la joint, et le leurre n'est pas inscrit.
    #[test]
    fn an_officine_picked_by_its_key_is_reached_and_not_a_decoy() {
        let dir = |tag: &str| {
            let d = std::env::temp_dir()
                .join(format!("bpm-caddy-dht-pick-{tag}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&d);
            std::fs::create_dir_all(&d).unwrap();
            d
        };
        let (dir_a, dir_b, dir_c) = (dir("a"), dir("b"), dir("c"));
        let _swept = (
            crate::db::Swept(dir_a.clone()),
            crate::db::Swept(dir_b.clone()),
            crate::db::Swept(dir_c.clone()),
        );
        let open = |d: &std::path::Path| {
            let db = crate::db::Db::open(&d.join("net.db"), "secret").unwrap();
            crate::network::join_open(&db, "2026-10-21").unwrap();
            db
        };
        let (a, b, c) = (open(&dir_a), open(&dir_b), open(&dir_c));
        let a_dev = crate::network::device_hex(&a).unwrap();
        let c_dev = crate::network::device_hex(&c).unwrap();
        let serve = |d: std::path::PathBuf| {
            let door = bpm_sync::link::Door::open("127.0.0.1:0").unwrap();
            let port = door.address().unwrap().port();
            std::thread::spawn(move || {
                let db = crate::db::Db::open(&d.join("net.db"), "secret").unwrap();
                // Answer whoever calls, a few times: the decoy may be
                // called first.
                for _ in 0..2 {
                    if let Ok(mut link) = door.accept(Duration::from_secs(20)) {
                        let _ = crate::network::answer(&db, &mut link);
                    }
                }
            });
            port
        };
        let (port_a, port_c) = (serve(dir_a.clone()), serve(dir_c.clone()));
        let testnet = mainline::Testnet::new(8).unwrap();
        let finder_a = Finder::start(Some(&testnet.bootstrap)).unwrap();
        let finder_c = Finder::start(Some(&testnet.bootstrap)).unwrap();
        assert!(finder_a.announce_as(&a_dev, port_a));
        // The decoy, under A's own topic.
        #[allow(deprecated)]
        let decoy = finder_c.dht.announce_peer(own_topic(&a_dev), Some(port_c));
        assert!(decoy.is_ok());
        let said = reach_officine(&b, &a_dev, "Pharmacie B", Some(&testnet.bootstrap));
        assert!(said.is_ok(), "{said:?}");
        let members: Vec<String> = b
            .net_members(&crate::network::open_id())
            .unwrap()
            .into_iter()
            .map(|p| p.device)
            .collect();
        assert!(members.contains(&a_dev));
        assert!(!members.contains(&c_dev), "le leurre n'est pas inscrit");
        // Unknown key: nothing found, said plainly.
        let nobody = "ab".repeat(32);
        assert!(reach_officine(&b, &nobody, "Pharmacie B", Some(&testnet.bootstrap)).is_err());
        assert_ne!(own_topic(&a_dev), own_topic(&c_dev));
        assert_eq!(own_topic(&a_dev), own_topic(&a_dev.to_uppercase()));
    }
}
