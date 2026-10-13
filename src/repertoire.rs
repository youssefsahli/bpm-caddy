//! **Le répertoire des officines** : une officine qui le veut paraît sur
//! la carte, dit ce qu'elle partage, et n'importe quelle autre parcourt
//! son partage — sans appairage.
//!
//! # Ce que c'est
//!
//! Un **petit serveur sans disque** qu'une officine accepte de tenir sur
//! l'un de ses postes, comme elle tiendrait le relais (`relay`). Les
//! autres en écrivent l'adresse ; aucune n'est livrée. Chaque officine
//! qui a choisi d'y paraître lui envoie, tant que l'application est
//! ouverte, sa **carte de visite** ([`Fiche`] : son nom, sa commune, son
//! FINESS si elle le donne, ce qu'elle partage) et son **catalogue** — ce
//! qu'elle a écrit elle-même des sortes publiques (`enligne::own_offers`).
//! Le répertoire les garde en mémoire et les rend à qui demande. Un
//! redémarrage l'efface ; les officines le remplissent de nouveau à leur
//! annonce suivante.
//!
//! # Ce qui protège quoi
//!
//! * **Rien n'y entre sans être coché.** Jamais un patient. Des fiches,
//!   préparations, protocoles, lignes de TROD, vaccins, dispositifs,
//!   cascades, listes et phrases imprimées — chacune cochée « Publier sur
//!   la carte » par l'officine (`published_entries`) —, et son journal des
//!   ruptures si elle l'a choisi, sans initiales.
//! * **Signé.** La carte de visite est signée par la clé de l'officine
//!   sur le réseau (`network::device_hex`) et porte l'empreinte de son
//!   catalogue. Le répertoire vérifie avant de garder, **et chaque poste
//!   vérifie de nouveau** : l'officine qui héberge ne peut ni changer un
//!   mot d'un catalogue, ni faire parler une officine à sa place, ni
//!   remettre une carte d'hier par-dessus celle d'aujourd'hui (`seq`).
//! * **Qui est qui n'est pas prouvé.** N'importe qui peut écrire
//!   « Pharmacie du Port » : la carte montre l'empreinte de la clé, le
//!   FINESS se compare au registre public, et l'appairage reste la seule
//!   preuve. C'est pourquoi rien de ce qu'on lit ici n'entre de lui-même
//!   dans la base : une entrée se prend à la main, une à une.
//! * **Borné.** Comme le relais : un nombre de connexions, d'appels par
//!   adresse, d'officines par adresse et en tout, une taille de catalogue,
//!   un temps pour dire sa demande.
//!
//! Le protocole tient en une ligne (`BPMREPERTOIRE1 annonce|liste|
//! catalogue <clé>`) suivie de blocs préfixés de leur longueur.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{IpAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::versions::{self, Kind};

/// La préface d'une connexion au répertoire.
pub const PREFACE: &str = "BPMREPERTOIRE1";
/// Le port proposé pour tenir le répertoire — à côté de celui du relais
/// (7746).
pub const DEFAULT_PORT: u16 = 7747;
/// Ce qui signe une carte : jamais une autre chose signée par la même
/// clé (une poignée de main, une boîte scellée).
const CONTEXT: &[u8] = b"bpm-caddy/repertoire/v1\n";

/// Une officine qui ne s'est pas annoncée depuis ceci quitte le
/// répertoire…
pub const FORGET: Duration = Duration::from_secs(24 * 3600);
/// …et passe pour **ouverte** tant que sa dernière carte a moins de
/// ceci. Le poste en signe une nouvelle toutes les [`ANNOUNCE_EVERY`].
pub const ONLINE: Duration = Duration::from_secs(25 * 60);
pub const ANNOUNCE_EVERY: Duration = Duration::from_secs(10 * 60);
/// Combien de temps la base garde la même heure de réception
/// (`Presence::parue_le`) avant de la récrire : le réglage voyage vers
/// tous les postes, pas toutes les dix minutes.
pub const PARUE_REFRESH: u64 = 6 * 3600;
/// Une carte n'est reçue que signée depuis moins de ceci, et pas plus
/// loin dans l'avenir : rejouée plus tard, elle est refusée. Les deux
/// horloges doivent donc s'accorder à une demi-heure près.
pub const FRESH: u64 = 30 * 60;

/// Les bornes d'une carte de visite.
pub const NAME_MAX: usize = 80;
const CARD_MAX: usize = 4 * 1024;
/// Un catalogue porte au plus ceci, en octets…
pub const CATALOGUE_MAX: usize = 2 * 1024 * 1024;
/// …et autant d'entrées, d'au plus ceci par valeur.
pub const ARTICLES_MAX: usize = 5000;
const VALUE_MAX: usize = 64 * 1024;
/// Le nom ou la clé d'une entrée, en octets.
const ENTRY_NAME_MAX: usize = 400;
/// Combien de connexions une adresse tient à la fois.
const OPEN_PER_ADDRESS: usize = 4;
/// Le temps de dire sa préface — la demande entière en a plus.
const PREFACE_WAIT: Duration = Duration::from_secs(5);
/// Combien d'officines le répertoire tient, en tout et par adresse, et
/// ce qu'une adresse peut y occuper.
const MOST_LISTED: usize = 2000;
const LISTED_PER_ADDRESS: usize = 16;
/// Les mêmes bornes pour un **réseau** (`network`) — un `/48` en IPv6,
/// ce qu'un tunnel gratuit ou un serveur loué donne à une seule personne,
/// un `/24` en IPv4 : soixante-cinq mille `/64` ne remplissent pas le
/// répertoire, ni ne tiennent toutes ses connexions.
const LISTED_PER_NETWORK: usize = 64;
const BYTES_PER_NETWORK: usize = 32 * 1024 * 1024;
const CALLERS_PER_NETWORK: usize = 64;
const OPEN_PER_NETWORK: usize = 16;
const BYTES_PER_ADDRESS: usize = 8 * 1024 * 1024;
/// Combien d'officines parties il se rappelle.
const MOST_GONE: usize = 4096;
/// Ce que le répertoire garde en tout, catalogues compris.
const MOST_BYTES: usize = 256 * 1024 * 1024;
const MOST_CONNECTIONS: usize = 64;
const CALLS_PER_MINUTE: usize = 30;
/// Le temps d'une demande entière, réponse comprise.
const REQUEST_WAIT: Duration = Duration::from_secs(30);
/// Ce qu'un poste accepte de lire en réponse.
const ANSWER_MAX: usize = 16 * 1024 * 1024;

/// **La carte de visite** d'une officine au répertoire.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Fiche {
    /// Sa clé publique sur le réseau, en hexadécimal.
    pub device: String,
    /// Croissant d'une annonce à l'autre (l'heure Unix de la signature) :
    /// une carte plus ancienne ne remplace jamais une plus récente.
    pub seq: u64,
    pub nom: String,
    pub commune: String,
    pub code_postal: String,
    /// Le numéro FINESS de l'officine, ou vide.
    pub finess: String,
    /// Ce qu'elle partage, par sorte : (clé de la sorte, nombre).
    pub parts: Vec<(String, u32)>,
    /// L'empreinte BLAKE3 de son catalogue, en hexadécimal.
    pub catalogue: String,
    /// `false` : elle quitte le répertoire.
    pub present: bool,
    /// La signature Ed25519 de tout ce qui précède, en hexadécimal.
    pub signature: String,
}

/// **Ce que l'officine a décidé** pour la carte — un réglage de la base
/// (`settings`, clé [`SETTING`]), le même sur tous ses postes : où est le
/// répertoire, si elle y paraît, sa commune, son FINESS.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Presence {
    /// L'adresse du répertoire, `hôte:port` — celle qu'une officine qui
    /// l'héberge a donnée. Vide : pas de répertoire.
    pub adresse: String,
    pub paraitre: bool,
    pub code_postal: String,
    pub commune: String,
    pub finess: String,
    /// Publier aussi ses ruptures et substitutions (`ruptures::
    /// public_events`) dans son catalogue. Non par défaut : le journal dit
    /// ce que l'officine n'a pas en stock, et ce n'est pas à tout le monde.
    pub ruptures: bool,
    /// **Le répertoire qui a reçu la dernière carte de l'officine**, et
    /// quand (heure Unix) — retenu dans la base : quitter ce répertoire
    /// lui doit un départ, qu'un départ manqué, un redémarrage ou un autre
    /// poste rattrape. Passé [`FORGET`], plus rien n'est dû : le
    /// répertoire a oublié la carte, et [`FRESH`] l'empêche de revenir.
    pub parue: String,
    pub parue_le: u64,
}

impl Presence {
    /// Le départ encore dû, s'il l'est : à un répertoire qui a reçu une
    /// carte, que l'officine ne veut plus — ou plus là —, depuis moins
    /// d'un jour.
    pub fn owed(&self, now: u64) -> Option<String> {
        let wanted = self.paraitre.then(|| self.address()).flatten();
        (!self.parue.is_empty()
            && wanted.as_deref() != Some(self.parue.as_str())
            // Le répertoire garde la carte un jour depuis la dernière ;
            // l'heure retenue ici peut avoir jusqu'à `PARUE_REFRESH` de
            // retard sur elle.
            && now.saturating_sub(self.parue_le) < FORGET.as_secs() + PARUE_REFRESH)
            .then(|| self.parue.clone())
    }
}

/// La clé du réglage.
pub const SETTING: &str = "repertoire";

impl Presence {
    /// Relire le réglage ; illisible ou absent, rien n'est décidé.
    pub fn read(text: Option<&str>) -> Self {
        text.and_then(|t| serde_json::from_str(t).ok())
            .unwrap_or_default()
    }

    pub fn text(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// L'adresse, complétée du port proposé quand elle n'en dit pas.
    pub fn address(&self) -> Option<String> {
        let a = self.adresse.trim();
        if a.is_empty() || a.contains(char::is_whitespace) {
            return None;
        }
        // `hôte:port`, `[v6]:port` ; un hôte seul, ou une IPv6 nue, prend
        // le port du répertoire.
        let has_port = match a.rsplit_once(':') {
            Some((host, port)) => {
                port.parse::<u16>().is_ok() && (!host.contains(':') || host.ends_with(']'))
            }
            None => false,
        };
        Some(if has_port {
            a.to_owned()
        } else if a.contains(':') && !a.starts_with('[') {
            format!("[{a}]:{DEFAULT_PORT}")
        } else {
            format!("{a}:{DEFAULT_PORT}")
        })
    }
}

/// Les cinq groupes de l'empreinte d'une clé, tels que l'appairage les
/// fait lire : la même chose ici et à l'écran du réseau.
pub fn fingerprint(device: &str) -> Option<String> {
    let id = unhex::<32>(device)?;
    Some(bpm_sync::DeviceId::from_bytes(id).fingerprint().groups())
}

/// Une entrée du catalogue : ce qu'une offre de `enligne` est, sans ce
/// que seul le journal local sait.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Article {
    pub kind: String,
    pub card: String,
    pub name: String,
    pub day: String,
    pub fields: Vec<(String, String)>,
}

impl Fiche {
    /// Depuis combien de secondes elle n'a pas signé de carte, **à
    /// l'horloge de qui lit** : ce que le répertoire dirait de son âge,
    /// il pourrait le mentir.
    pub fn age(&self, now: u64) -> u64 {
        now.saturating_sub(self.seq)
    }

    /// Ouverte en ce moment, à peu de chose près.
    pub fn online(&self, now: u64) -> bool {
        self.age(now) < ONLINE.as_secs()
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex<const N: usize>(text: &str) -> Option<[u8; N]> {
    // Minuscules seulement : une clé n'a qu'une écriture.
    if text.len() != N * 2 || !text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
        return None;
    }
    let mut out = [0u8; N];
    for (i, o) in out.iter_mut().enumerate() {
        *o = u8::from_str_radix(&text[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
}

/// Ce que la signature couvre : la carte sans sa signature.
fn signed_bytes(fiche: &Fiche) -> Vec<u8> {
    let bare = Fiche {
        signature: String::new(),
        ..fiche.clone()
    };
    let mut out = CONTEXT.to_vec();
    out.extend(serde_json::to_vec(&bare).unwrap_or_default());
    out
}

/// L'empreinte d'un catalogue.
pub fn digest(catalogue: &[u8]) -> String {
    blake3::hash(catalogue).to_hex().to_string()
}

/// **Le catalogue d'une officine**, sérialisé : ce qu'elle a écrit des
/// sortes publiques, chaque entrée à sa dernière version — et le compte
/// par sorte que sa carte annonce.
pub fn catalogue_of(edits: &[versions::Edit]) -> (Vec<u8>, Vec<(String, u32)>) {
    catalogue_where(edits, |k, _| !k.is_private())
}

/// [`catalogue_of`], réduit aux entrées que `here` dit encore là (sorte,
/// clé) : le journal garde les versions d'une fiche supprimée, et une
/// fiche retirée parce qu'elle était fausse ne se publie plus.
pub fn catalogue_where(
    edits: &[versions::Edit],
    here: impl Fn(Kind, &str) -> bool,
) -> (Vec<u8>, Vec<(String, u32)>) {
    catalogue_with(edits, here, &[])
}

/// **Le nom sous lequel une sorte voyage au répertoire.** Une liste ou une
/// phrase n'y paraît que cochée, une à une, et sous un nom à elle : une
/// version d'avant refusait une carte entière qui annonçait `liste` ou
/// `texte` ; un nom qu'elle ne connaît pas, elle le laisse passer chez qui
/// héberge et l'écarte article par article chez qui lit.
pub fn wire_key(kind: Kind) -> &'static str {
    match kind {
        Kind::Liste => "listepublique",
        Kind::Texte => "textepublic",
        k => k.key(),
    }
}

/// L'inverse : jamais `liste` ni `texte` sous leur nom d'officine à
/// officine appairée.
pub fn kind_of_wire(key: &str) -> Option<Kind> {
    Kind::ALL.into_iter().find(|k| wire_key(*k) == key)
}

/// La sorte d'article qui porte un événement du journal des ruptures.
///
/// Pas une `versions::Kind` : un événement ne se réécrit pas et ne se
/// « prend » pas comme une fiche — il entre au journal de qui le lit, au
/// nom de l'officine qui l'a publié. Une version d'avant ne connaît pas
/// cette sorte et laisse l'article de côté, sans refuser le catalogue
/// (`read_catalogue` juge article par article).
pub const SUPPLY_KIND: &str = "rupture";
const SUPPLY_FIELD: &str = "evenement";

/// Les événements qu'une officine publie, en articles : l'événement entier,
/// encodé comme entre officines appairées, sans le nom de l'officine — c'est
/// la carte signée qui le dit.
pub fn supply_articles(events: &[crate::ruptures::Event]) -> Vec<Article> {
    events
        .iter()
        .take(crate::ruptures::PUBLIC_MOST)
        .map(|e| Article {
            kind: SUPPLY_KIND.to_owned(),
            card: e.uid.clone(),
            name: e.product.clone(),
            day: e.day.clone(),
            fields: vec![(
                SUPPLY_FIELD.to_owned(),
                String::from_utf8(crate::ruptures::encode(e, "")).unwrap_or_default(),
            )],
        })
        .collect()
}

/// L'événement qu'un article porte, au nom de l'officine dont la carte l'a
/// signé — jamais du nom écrit dans l'événement —, sans initiales ni note.
///
/// **Jamais un retrait** : une officine ne publie pas les siens
/// (`ruptures::public_events`), et un retrait lu sur la carte, sous le nom
/// qu'une carte se donne, retirerait ici les événements d'une officine
/// appairée qui porte ce nom.
pub fn supply_of(article: &Article, officine: &str) -> Option<crate::ruptures::Event> {
    if article.kind != SUPPLY_KIND || article.name.trim().is_empty() {
        return None;
    }
    let (_, text) = article.fields.iter().find(|(f, _)| f == SUPPLY_FIELD)?;
    let mut e = crate::ruptures::decode(text.as_bytes())?;
    if e.uid != article.card || e.kind == crate::ruptures::Kind::Retrait {
        return None;
    }
    e.source = officine.to_owned();
    e.operator.clear();
    e.note.clear();
    Some(e)
}

/// **L'identifiant sous lequel un événement lu sur la carte entre au
/// journal d'ici** : celui de l'événement, précédé de la clé qui a signé
/// la carte. Un identifiant se devine (`<base>:<numéro>`) : pris tel quel,
/// celui qu'une officine de la carte aurait copié prendrait la place de
/// l'événement véritable d'une officine appairée, qui serait ensuite
/// ignoré à son arrivée.
pub fn taken_uid(device: &str, uid: &str) -> String {
    format!("carte:{device}:{uid}")
}

/// [`catalogue_where`], et les événements du journal que l'officine a
/// choisi de publier — d'abord, pour qu'un grand catalogue ne les pousse
/// pas dehors.
pub fn catalogue_with(
    edits: &[versions::Edit],
    here: impl Fn(Kind, &str) -> bool,
    supply: &[Article],
) -> (Vec<u8>, Vec<(String, u32)>) {
    let mut articles: Vec<Article> = supply
        .iter()
        .take(crate::ruptures::PUBLIC_MOST)
        .cloned()
        .filter_map(admitted)
        .collect();
    let mut parts = Vec::new();
    if !articles.is_empty() {
        parts.push((SUPPLY_KIND.to_owned(), articles.len() as u32));
    }
    // Toutes les sortes : `here` ne laisse passer que ce que l'officine a
    // coché, et c'est la seule porte d'une liste ou d'une phrase.
    for kind in Kind::ALL {
        let own = crate::enligne::own_offers_any(edits, kind);
        let mut n = 0_u32;
        for o in own.into_iter().filter(|o| here(kind, &o.card)) {
            if articles.len() >= ARTICLES_MAX {
                break;
            }
            // La règle de qui lit, appliquée avant d'écrire : un article
            // qu'un poste écarterait ferait refuser tout le catalogue.
            let Some(article) = admitted(Article {
                kind: wire_key(kind).to_owned(),
                card: o.card,
                name: o.name,
                day: o.day,
                fields: o.fields,
            }) else {
                continue;
            };
            n += 1;
            articles.push(article);
        }
        if n > 0 {
            parts.push((wire_key(kind).to_owned(), n));
        }
    }
    // Trop gros : les dernières entrées tombent, jamais la carte entière.
    let mut bytes = serde_json::to_vec(&articles).unwrap_or_default();
    while bytes.len() > CATALOGUE_MAX && !articles.is_empty() {
        let drop = (articles.len() / 10).max(1);
        articles.truncate(articles.len() - drop);
        bytes = serde_json::to_vec(&articles).unwrap_or_default();
        parts = recount(&articles);
    }
    (bytes, parts)
}

fn recount(articles: &[Article]) -> Vec<(String, u32)> {
    let mut parts: Vec<(String, u32)> = Vec::new();
    for a in articles {
        match parts.iter_mut().find(|(k, _)| *k == a.kind) {
            Some((_, n)) => *n += 1,
            None => parts.push((a.kind.clone(), 1)),
        }
    }
    parts
}

/// **Signer une carte** avec la clé de l'officine.
pub fn sign(mut fiche: Fiche, device: &bpm_sync::Device) -> Fiche {
    fiche.device = hex(&device.id().0);
    fiche.signature = String::new();
    fiche.signature = hex(&device.sign(&signed_bytes(&fiche)));
    fiche
}

/// Une carte bien formée et signée par la clé qu'elle nomme.
pub fn verify(fiche: &Fiche) -> bool {
    let (Some(id), Some(sig)) = (unhex::<32>(&fiche.device), unhex::<64>(&fiche.signature)) else {
        return false;
    };
    well_formed(fiche)
        && bpm_sync::DeviceId::from_bytes(id)
            .verify(&signed_bytes(fiche), &sig)
            .is_ok()
}

/// Les bornes d'une carte, avant toute signature.
fn well_formed(f: &Fiche) -> bool {
    let printable = |s: &str| !s.chars().any(char::is_control);
    // Ce qu'elle partage : chaque sorte une fois, nommée court — une
    // sorte qu'une version d'après partagera passe, une privée jamais.
    let parts_ok = f.parts.len() <= 16
        && f.parts.iter().enumerate().all(|(i, (k, n))| {
            !k.is_empty()
                && k.len() <= 32
                && k.bytes().all(|b| b.is_ascii_lowercase())
                && versions::kind_of_key(k).is_none_or(|k| !k.is_private())
                && *n as usize <= ARTICLES_MAX
                && !f.parts[..i].iter().any(|(o, _)| o == k)
        });
    f.nom.chars().count() <= NAME_MAX
        && f.commune.chars().count() <= NAME_MAX
        && printable(&f.nom)
        && printable(&f.commune)
        && (f.code_postal.is_empty()
            || (f.code_postal.len() == 5 && f.code_postal.chars().all(|c| c.is_ascii_digit())))
        && (f.finess.is_empty()
            || (f.finess.len() == 9 && f.finess.chars().all(|c| c.is_ascii_alphanumeric())))
        && parts_ok
        && (!f.present || !f.nom.trim().is_empty())
        && (f.catalogue.is_empty() || unhex::<32>(&f.catalogue).is_some())
        && serde_json::to_vec(f).is_ok_and(|b| b.len() <= CARD_MAX)
}

/// **Lire un catalogue reçu**, et n'en garder que ce qui peut entrer :
/// une sorte publique connue, des champs qu'elle partage et qui se
/// lisent. `None` si les octets ne sont pas ceux que la carte signe.
pub fn read_catalogue(fiche: &Fiche, bytes: &[u8]) -> Option<Vec<Article>> {
    if bytes.len() > CATALOGUE_MAX || digest(bytes) != fiche.catalogue {
        return None;
    }
    let articles: Vec<Article> = serde_json::from_slice(bytes).ok()?;
    Some(
        articles
            .into_iter()
            .take(ARTICLES_MAX)
            .filter_map(admitted)
            .collect(),
    )
}

/// **Ce qu'un article doit être pour entrer chez qui lit** : une sorte
/// publique que cette version connaît, les champs qu'elle partage et qui
/// se lisent, un nom et une clé. La même règle pour qui publie et pour qui
/// lit.
fn admitted(mut a: Article) -> Option<Article> {
    if a.kind == SUPPLY_KIND {
        // Un seul champ, l'événement, qui se lit et nomme la même clé.
        a.fields
            .retain(|(f, v)| f == SUPPLY_FIELD && v.len() <= VALUE_MAX);
        let e = supply_of(&a, "")?;
        // Les bornes de qui héberge, redites ici : un nom trop long
        // ferait refuser la carte entière, pas seulement l'article.
        return (e.product == a.name
            && e.day == a.day
            && a.fields.len() == 1
            && a.card.len() <= ENTRY_NAME_MAX
            && a.name.len() <= ENTRY_NAME_MAX)
            .then_some(a);
    }
    let kind = kind_of_wire(&a.kind)?;
    a.fields
        .retain(|(f, v)| v.len() <= VALUE_MAX && versions::admissible(kind, f, v));
    (!a.fields.is_empty()
        && !a.card.trim().is_empty()
        && !a.name.trim().is_empty()
        && a.card.len() <= ENTRY_NAME_MAX
        && a.name.len() <= ENTRY_NAME_MAX)
        .then_some(a)
}

/// **Ce que le répertoire garde, quelle que soit la version** : des
/// bornes, sans juger ce que l'article dit — une sorte ou un champ
/// qu'une version d'après partagera ne fait pas disparaître de la carte
/// les officines qui l'ont. Juger est le travail de qui lit.
fn bounded(a: &Article) -> bool {
    a.kind.len() <= 32
        && a.card.len() <= ENTRY_NAME_MAX
        && a.name.len() <= ENTRY_NAME_MAX
        && a.day.len() <= 32
        && !a.fields.is_empty()
        && a.fields.len() <= 64
        && a.fields
            .iter()
            .all(|(f, v)| f.len() <= 64 && v.len() <= VALUE_MAX)
}

/// **Vérifier une annonce avant de prendre le verrou** : sa signature,
/// son catalogue — lié à la carte, borné, dans sa forme exacte, et compté
/// comme la carte le dit. Rend ce qu'elle occupera.
pub fn check(fiche: &Fiche, catalogue: &[u8]) -> Result<usize, &'static str> {
    if !verify(fiche) {
        return Err("signature");
    }
    if !fiche.present {
        return Ok(0);
    }
    if catalogue.len() > CATALOGUE_MAX || digest(catalogue) != fiche.catalogue {
        return Err("catalogue");
    }
    let raw: Vec<Article> = serde_json::from_slice(catalogue).map_err(|_| "catalogue")?;
    // Dans sa forme exacte : ce qui ne sert à rien — de l'espace, un
    // champ en double — n'occupe pas la mémoire de qui héberge.
    if raw.len() > ARTICLES_MAX
        || !raw.iter().all(bounded)
        || serde_json::to_vec(&raw).ok().as_deref() != Some(catalogue)
        || recount(&raw) != fiche.parts
    {
        return Err("catalogue");
    }
    Ok(catalogue.len() + serde_json::to_vec(fiche).map_or(CARD_MAX, |b| b.len()))
}

/// Un article en offre de la fenêtre « En ligne », au nom de l'officine
/// qui le publie.
pub fn offer_of(article: &Article, from: &str) -> Option<crate::enligne::Offer> {
    let kind = kind_of_wire(&article.kind)?;
    Some(crate::enligne::Offer {
        kind,
        card: article.card.clone(),
        name: article.name.clone(),
        from: vec![from.to_owned()],
        day: article.day.clone(),
        stamp: String::new(),
        stamp_at: 0,
        mine: false,
        fields: article.fields.clone(),
    })
}

/// **Combien de catalogues un balayage demande** : avec la liste, sous
/// les trente appels par adresse et par minute que le répertoire tient,
/// en laissant de quoi à l'annonce et aux autres postes derrière la même
/// adresse — un balayage ne se fait pas refuser la fin de sa propre
/// lecture. Deux balayages sont espacés de [`SWEEP_EVERY`].
pub const SWEEP_MOST: usize = 20;
const _: () = assert!(SWEEP_MOST + 1 + 4 <= CALLS_PER_MINUTE);

/// L'écart entre deux balayages : la minute que le répertoire compte.
pub const SWEEP_EVERY: Duration = Duration::from_secs(60);

/// Le temps qu'un balayage prend au plus, liste comprise.
pub const SWEEP_BUDGET: Duration = Duration::from_secs(90);

/// Ce que les catalogues gardés en mémoire pèsent au plus : un
/// répertoire qui en annonce de très gros ne fait pas grossir l'écran
/// sans fin.
pub const SWEEP_BYTES: usize = 32 << 20;

/// **Les officines dont un balayage lit le catalogue** : présentes, pas
/// celle-ci, avec un catalogue annoncé, et pas déjà lu sous la même
/// empreinte (`held`) ; les ouvertes d'abord, puis les plus récemment
/// vues — l'heure d'une carte bornée à `now`, qu'une carte datée de
/// demain ne passe pas devant —, et celles qui n'ont pas répondu
/// (`failed`) en dernier : elles ne prennent pas la place des autres à
/// chaque balayage. Au plus `most`.
pub fn to_sweep<'a>(
    list: &'a [Fiche],
    held: impl Fn(&Fiche) -> bool,
    failed: impl Fn(&Fiche) -> bool,
    me: &str,
    now: u64,
    most: usize,
) -> Vec<&'a Fiche> {
    let mut out: Vec<&Fiche> = list
        .iter()
        .filter(|f| {
            f.present
                && f.device != me
                && !f.catalogue.is_empty()
                && f.parts.iter().any(|(_, n)| *n > 0)
                && !held(f)
        })
        .collect();
    out.sort_by_key(|f| (failed(f), !f.online(now), std::cmp::Reverse(f.seq.min(now))));
    out.truncate(most);
    out
}

/// Ce qu'un catalogue lu pèse en mémoire, à peu près : ses textes et ce
/// que chaque chaîne coûte en plus d'elle-même — trois fois, gardé une
/// fois et copié deux tant que « En ligne » le montre.
pub fn weight(articles: &[Article]) -> usize {
    const STRING: usize = std::mem::size_of::<String>();
    articles
        .iter()
        .map(|a| {
            a.kind.len()
                + a.card.len()
                + a.name.len()
                + a.day.len()
                + a.fields
                    .iter()
                    .map(|(f, v)| f.len() + v.len() + 2 * STRING)
                    .sum::<usize>()
                + std::mem::size_of::<Article>()
        })
        .sum::<usize>()
        * 3
}

/// Le verbe d'une demande.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verbe {
    Annonce,
    Liste,
    Catalogue(String),
}

pub fn preface(verbe: &Verbe) -> String {
    match verbe {
        Verbe::Annonce => format!("{PREFACE} annonce\n"),
        Verbe::Liste => format!("{PREFACE} liste\n"),
        Verbe::Catalogue(device) => format!("{PREFACE} catalogue {device}\n"),
    }
}

pub fn read_preface(line: &str) -> Option<Verbe> {
    let mut parts = line.split_whitespace();
    if parts.next()? != PREFACE {
        return None;
    }
    let verbe = match parts.next()? {
        "annonce" => Verbe::Annonce,
        "liste" => Verbe::Liste,
        "catalogue" => {
            let device = parts.next()?.to_ascii_lowercase();
            unhex::<32>(&device)?;
            Verbe::Catalogue(device)
        }
        _ => return None,
    };
    parts.next().is_none().then_some(verbe)
}

/// Un bloc : sa longueur sur quatre octets, puis lui.
fn write_block(stream: &mut impl Write, bytes: &[u8]) -> std::io::Result<()> {
    let len = u32::try_from(bytes.len())
        .map_err(|_| std::io::Error::from(std::io::ErrorKind::InvalidInput))?;
    stream.write_all(&len.to_be_bytes())?;
    stream.write_all(bytes)
}

fn read_block(stream: &mut impl Read, most: usize) -> std::io::Result<Vec<u8>> {
    let mut len = [0u8; 4];
    stream.read_exact(&mut len)?;
    let len = u32::from_be_bytes(len) as usize;
    if len > most {
        return Err(std::io::Error::from(std::io::ErrorKind::InvalidData));
    }
    let mut out = vec![0u8; len];
    stream.read_exact(&mut out)?;
    Ok(out)
}

/// Ce que le répertoire tient d'une officine.
struct Listed {
    fiche: Fiche,
    catalogue: Vec<u8>,
    /// Ce que la carte et le catalogue occupent, comptés tels quels.
    weight: usize,
    from: IpAddr,
}

/// L'état du répertoire, partagé entre ses fils. Le temps lui est donné
/// (`now`, heure Unix en secondes) : il se teste sans horloge.
#[derive(Default)]
pub struct Store {
    listed: HashMap<String, Listed>,
    /// **Ceux qui sont partis**, et la dernière carte qu'ils ont signée :
    /// rejouer leur carte d'avant, encore fraîche, ne les fait pas
    /// revenir. Oubliés quand cette carte n'est plus fraîche — elle serait
    /// refusée de toute façon.
    gone: HashMap<String, u64>,
}

impl Store {
    fn bytes(&self) -> usize {
        self.listed.values().map(|l| l.weight).sum()
    }

    fn prune(&mut self, now: u64) {
        self.listed
            .retain(|_, l| now.saturating_sub(l.fiche.seq) < FORGET.as_secs());
        self.gone.retain(|_, seq| seq.saturating_add(FRESH) >= now);
    }

    /// **Garder une annonce** : vérifiée (`check`), puis admise.
    pub fn take(
        &mut self,
        fiche: Fiche,
        catalogue: Vec<u8>,
        from: IpAddr,
        now: u64,
    ) -> Result<(), &'static str> {
        let weight = check(&fiche, &catalogue)?;
        self.admit(fiche, catalogue, weight, from, now)
    }

    /// **Admettre une annonce déjà vérifiée**, sous le verrou : fraîche,
    /// plus récente que tout ce qu'on a vu de cette officine, et dans les
    /// bornes. Rend le refus en un mot.
    pub fn admit(
        &mut self,
        fiche: Fiche,
        catalogue: Vec<u8>,
        weight: usize,
        from: IpAddr,
        now: u64,
    ) -> Result<(), &'static str> {
        self.prune(now);
        // Fraîche, à l'heure de ce poste près : une carte d'hier, sauvée
        // par qui l'a lue, ne se rejoue pas.
        if fiche.seq.saturating_add(FRESH) < now || fiche.seq > now.saturating_add(FRESH) {
            return Err("heure");
        }
        let last = self
            .listed
            .get(&fiche.device)
            .map(|l| l.fiche.seq)
            .or_else(|| self.gone.get(&fiche.device).copied());
        if last.is_some_and(|seq| fiche.seq <= seq) {
            return Err("ancienne");
        }
        if !fiche.present {
            // Retenue seulement pour qui était là : c'est le seul départ
            // qu'un rejeu pourrait défaire, et les bornes des cartes
            // bornent ainsi celles des départs.
            if self.listed.remove(&fiche.device).is_some() && self.gone.len() < MOST_GONE {
                self.gone.insert(fiche.device, fiche.seq);
            }
            return Ok(());
        }
        // Les bornes par adresse, comptées **à chaque annonce** : une carte
        // qui change d'adresse entre dans le compte de la nouvelle.
        let (mut here, mut here_bytes) = (0_usize, weight);
        let (mut near, mut near_bytes) = (0_usize, weight);
        let net = network(from);
        for (device, l) in &self.listed {
            if *device == fiche.device {
                continue;
            }
            if l.from == from {
                here += 1;
                here_bytes += l.weight;
            }
            if network(l.from) == net {
                near += 1;
                near_bytes += l.weight;
            }
        }
        if here >= LISTED_PER_ADDRESS || here_bytes > BYTES_PER_ADDRESS {
            return Err("adresse");
        }
        if near >= LISTED_PER_NETWORK || near_bytes > BYTES_PER_NETWORK {
            return Err("adresse");
        }
        let held = self.listed.get(&fiche.device).map_or(0, |l| l.weight);
        if held == 0 && self.listed.len() >= MOST_LISTED {
            return Err("plein");
        }
        if self.bytes() - held + weight > MOST_BYTES {
            return Err("plein");
        }
        self.gone.remove(&fiche.device);
        self.listed.insert(
            fiche.device.clone(),
            Listed {
                fiche,
                catalogue,
                weight,
                from,
            },
        );
        Ok(())
    }

    /// Les cartes du répertoire, la plus récente d'abord.
    pub fn list(&mut self, now: u64) -> Vec<Fiche> {
        self.prune(now);
        let mut out: Vec<Fiche> = self.listed.values().map(|l| l.fiche.clone()).collect();
        out.sort_by_key(|f| std::cmp::Reverse(f.seq));
        out
    }

    pub fn catalogue(&mut self, device: &str, now: u64) -> Vec<u8> {
        self.prune(now);
        self.listed
            .get(device)
            .map(|l| l.catalogue.clone())
            .unwrap_or_default()
    }
}

/// L'heure Unix de ce poste, en secondes.
pub fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Qui appelle, et combien de fois dans la minute — borné, et élagué à
/// la seconde plutôt qu'à chaque appel : une adresse qui martèle ne fait
/// pas grossir la table, ni travailler le fil qui accepte.
#[derive(Default)]
struct Calls {
    seen: HashMap<IpAddr, Vec<Instant>>,
    pruned: Option<Instant>,
}

impl Calls {
    fn admit(&mut self, ip: IpAddr) -> bool {
        let now = Instant::now();
        let minute = Duration::from_secs(60);
        if self
            .pruned
            .is_none_or(|t| now.duration_since(t) >= Duration::from_secs(1))
        {
            self.seen.retain(|_, v| {
                v.retain(|t| now.duration_since(*t) < minute);
                !v.is_empty()
            });
            self.pruned = Some(now);
        }
        if !self.seen.contains_key(&ip) {
            if self.seen.len() >= 8192 {
                return false;
            }
            // Un réseau ne remplit pas la table de ses `/64`.
            let net = network(ip);
            if self.seen.keys().filter(|k| network(**k) == net).count() >= CALLERS_PER_NETWORK {
                return false;
            }
        }
        let v = self.seen.entry(ip).or_default();
        v.retain(|t| now.duration_since(*t) < minute);
        if v.len() >= CALLS_PER_MINUTE {
            return false;
        }
        v.push(now);
        true
    }
}

/// **Qui appelle, pour les bornes** : une adresse IPv4 telle quelle (et
/// celle qu'une prise double pile présente sous forme IPv6), une adresse
/// IPv6 réduite à son préfixe `/64` — le réseau d'un abonné, qui en tient
/// des milliards et changerait d'adresse à chaque appel pour passer sous
/// chaque borne.
pub fn caller(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V4(_) => ip,
        IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(v4) => IpAddr::V4(v4),
            None => {
                let s = v6.segments();
                IpAddr::V6(std::net::Ipv6Addr::new(s[0], s[1], s[2], s[3], 0, 0, 0, 0))
            }
        },
    }
}

/// **Le réseau d'un appelant**, pour les bornes d'un cran au-dessus : le
/// `/24` d'une IPv4, le `/48` d'une IPv6.
pub fn network(ip: IpAddr) -> IpAddr {
    match caller(ip) {
        IpAddr::V4(v4) => {
            let o = v4.octets();
            IpAddr::V4(std::net::Ipv4Addr::new(o[0], o[1], o[2], 0))
        }
        IpAddr::V6(v6) => {
            let s = v6.segments();
            IpAddr::V6(std::net::Ipv6Addr::new(s[0], s[1], s[2], 0, 0, 0, 0, 0))
        }
    }
}

/// La prise IPv6 du répertoire, **IPv6 seulement** sur tous les systèmes :
/// l'IPv4 a sa propre prise, et un port IPv4 tenu par un autre programme
/// se dit au lieu de passer inaperçu. `None` : ce poste n'a pas d'IPv6.
fn bind_v6(port: u16) -> std::io::Result<Option<TcpListener>> {
    use socket2::{Domain, Protocol, SockAddr, Socket, Type};
    let socket = match Socket::new(Domain::IPV6, Type::STREAM, Some(Protocol::TCP)) {
        Ok(s) => s,
        // Pas d'IPv6 sur ce poste (EAFNOSUPPORT : Linux, macOS, Windows) ;
        // toute autre erreur — plus de descripteurs — en est une.
        Err(e)
            if e.kind() == std::io::ErrorKind::Unsupported
                || matches!(e.raw_os_error(), Some(97 | 47 | 10047)) =>
        {
            return Ok(None)
        }
        Err(e) => return Err(e),
    };
    socket.set_only_v6(true)?;
    // Ce que la bibliothèque standard fait sous Unix : un répertoire
    // arrêté puis rouvert reprend son port sans attendre.
    #[cfg(unix)]
    socket.set_reuse_address(true)?;
    let at = std::net::SocketAddr::from((std::net::Ipv6Addr::UNSPECIFIED, port));
    match socket.bind(&SockAddr::from(at)) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::AddrNotAvailable => return Ok(None),
        Err(e) => return Err(e),
    }
    socket.listen(128)?;
    Ok(Some(socket.into()))
}

/// Des connexions ouvertes, par appelant ou par réseau.
type Counts = HashMap<IpAddr, usize>;

/// **Tenir le répertoire** sur `port` jusqu'à ce que `stop` passe à vrai,
/// en IPv6 et en IPv4 : en IPv6 la box n'a qu'à laisser passer, sans
/// ouverture de port. Rend tout de suite ; le répertoire vit sur ses
/// propres fils.
pub fn serve(port: u16, stop: Arc<AtomicBool>) -> std::io::Result<bool> {
    // Deux prises, chacune sa pile, et l'une ou l'autre prise ailleurs est
    // une erreur : la carte ne donne pas une adresse où répondrait un
    // autre programme.
    let v6 = bind_v6(port)?;
    let v4 = TcpListener::bind(("0.0.0.0", port))?;
    let on_v6 = v6.is_some();
    let listeners: Vec<TcpListener> = std::iter::once(v4).chain(v6).collect();
    for l in &listeners {
        l.set_nonblocking(true)?;
    }
    let store: Arc<Mutex<Store>> = Arc::default();
    let mut calls = Calls::default();
    let open = Arc::new(AtomicUsize::new(0));
    // Les connexions ouvertes par adresse : une adresse n'en tient que
    // quelques-unes, jamais toutes.
    // Les connexions ouvertes par adresse et par réseau : une adresse n'en
    // tient que quelques-unes, un réseau guère plus.
    let by_address: Arc<Mutex<(Counts, Counts)>> = Arc::default();
    std::thread::spawn(move || {
        let mut turn = 0_usize;
        while !stop.load(Ordering::SeqCst) {
            // Chaque prise à son tour ; on ne dort que quand aucune n'a
            // d'appel en attente.
            let mut accepted = None;
            let mut failed = false;
            for k in 0..listeners.len() {
                let l = &listeners[(turn + k) % listeners.len()];
                match l.accept() {
                    Ok(s) => {
                        accepted = Some(s);
                        break;
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                    Err(_) => failed = true,
                }
            }
            turn = turn.wrapping_add(1);
            let Some((stream, from)) = accepted else {
                std::thread::sleep(Duration::from_millis(if failed { 500 } else { 100 }));
                continue;
            };
            // Les bornes comptent un abonné, pas une adresse.
            let ip = caller(from.ip());
            if !calls.admit(ip) || open.load(Ordering::SeqCst) >= MOST_CONNECTIONS {
                continue;
            }
            let net = network(ip);
            let held = by_address.lock().map(|mut m| {
                let (here, near) = &mut *m;
                let (h, n) = (
                    here.get(&ip).copied().unwrap_or(0),
                    near.get(&net).copied().unwrap_or(0),
                );
                if h >= OPEN_PER_ADDRESS || n >= OPEN_PER_NETWORK {
                    false
                } else {
                    here.insert(ip, h + 1);
                    near.insert(net, n + 1);
                    true
                }
            });
            if !held.unwrap_or(false) {
                continue;
            }
            open.fetch_add(1, Ordering::SeqCst);
            let (store, open, by_address) = (
                Arc::clone(&store),
                Arc::clone(&open),
                Arc::clone(&by_address),
            );
            std::thread::spawn(move || {
                let _ = answer(stream, ip, &store);
                open.fetch_sub(1, Ordering::SeqCst);
                if let Ok(mut m) = by_address.lock() {
                    let (here, near) = &mut *m;
                    for (map, key) in [(here, ip), (near, net)] {
                        if let Some(n) = map.get_mut(&key) {
                            *n = n.saturating_sub(1);
                            if *n == 0 {
                                map.remove(&key);
                            }
                        }
                    }
                }
            });
        }
    });
    Ok(on_v6)
}

/// Une prise lue et écrite **contre une seule échéance**, celle de
/// l'échange entier : un octet toutes les quelques secondes, dans un sens
/// ou dans l'autre, ne tient pas un fil une heure.
struct Deadline<'a> {
    stream: &'a TcpStream,
    until: Instant,
}

impl Deadline<'_> {
    fn left(&self) -> std::io::Result<Duration> {
        let left = self.until.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(std::io::Error::from(std::io::ErrorKind::TimedOut));
        }
        Ok(left)
    }
}

impl Read for Deadline<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.stream.set_read_timeout(Some(self.left()?))?;
        let mut s = self.stream;
        s.read(buf)
    }
}

impl Write for Deadline<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.stream.set_write_timeout(Some(self.left()?))?;
        let mut s = self.stream;
        // Par morceaux : l'échéance est relue entre deux.
        s.write(&buf[..buf.len().min(16 * 1024)])
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let mut s = self.stream;
        s.flush()
    }
}

/// La ligne de préface, octet par octet : ce qui suit est un bloc.
fn read_line(r: &mut impl Read) -> std::io::Result<String> {
    let mut out = Vec::with_capacity(96);
    let mut byte = [0u8; 1];
    while out.len() < 128 {
        r.read_exact(&mut byte)?;
        if byte[0] == b'\n' {
            return Ok(String::from_utf8_lossy(&out).into_owned());
        }
        out.push(byte[0]);
    }
    Err(std::io::Error::from(std::io::ErrorKind::InvalidData))
}

/// Une demande reçue, et sa réponse — l'une et l'autre dans le temps
/// d'une demande.
fn answer(stream: TcpStream, from: IpAddr, store: &Mutex<Store>) -> std::io::Result<()> {
    stream.set_nonblocking(false)?;
    let start = Instant::now();
    let mut io = Deadline {
        stream: &stream,
        until: start + PREFACE_WAIT,
    };
    let Some(verbe) = read_preface(&read_line(&mut io)?) else {
        return Ok(());
    };
    io.until = start + REQUEST_WAIT;
    let now = unix_now();
    match verbe {
        Verbe::Annonce => {
            let fiche = read_block(&mut io, CARD_MAX)?;
            let catalogue = read_block(&mut io, CATALOGUE_MAX)?;
            // Vérifiée hors du verrou : une signature et deux mégaoctets
            // à relire ne font pas attendre ceux qui lisent la liste.
            let said = match serde_json::from_slice::<Fiche>(&fiche) {
                Ok(fiche) => match check(&fiche, &catalogue) {
                    Ok(weight) => match store.lock() {
                        Ok(mut s) => s.admit(fiche, catalogue, weight, from, now),
                        Err(_) => Err("plein"),
                    },
                    Err(why) => Err(why),
                },
                Err(_) => Err("illisible"),
            };
            let line = match said {
                Ok(()) => "OK\n".to_owned(),
                Err(why) => format!("NON {why}\n"),
            };
            io.write_all(line.as_bytes())
        }
        Verbe::Liste => {
            let list = store.lock().map(|mut s| s.list(now)).unwrap_or_default();
            write_block(&mut io, &serde_json::to_vec(&list).unwrap_or_default())
        }
        Verbe::Catalogue(device) => {
            let bytes = store
                .lock()
                .map(|mut s| s.catalogue(&device, now))
                .unwrap_or_default();
            write_block(&mut io, &bytes)
        }
    }
}

/// Composer le répertoire.
fn dial(address: &str, patience: Duration) -> Result<TcpStream, String> {
    let addrs: Vec<std::net::SocketAddr> = address
        .to_socket_addrs()
        .map_err(|e| e.to_string())?
        .collect();
    let mut last = String::from("adresse vide");
    for a in addrs {
        match TcpStream::connect_timeout(&a, patience) {
            Ok(s) => return Ok(s),
            Err(e) => last = e.to_string(),
        }
    }
    Err(last)
}

/// Un échange avec le répertoire, borné en tout par `patience` fois
/// quatre : un répertoire qui répond un octet à la fois ne tient pas le
/// poste.
fn exchange<T>(
    address: &str,
    patience: Duration,
    talk: impl FnOnce(&mut Deadline<'_>) -> std::io::Result<T>,
) -> Result<T, String> {
    let stream = dial(address, patience)?;
    let mut io = Deadline {
        stream: &stream,
        until: Instant::now() + patience * 4,
    };
    talk(&mut io).map_err(|e| e.to_string())
}

/// **S'annoncer** : la carte signée et son catalogue.
pub fn announce(
    address: &str,
    fiche: &Fiche,
    catalogue: &[u8],
    patience: Duration,
) -> Result<(), String> {
    let card = serde_json::to_vec(fiche).map_err(|e| e.to_string())?;
    let line = exchange(address, patience, |io| {
        io.write_all(preface(&Verbe::Annonce).as_bytes())?;
        write_block(io, &card)?;
        write_block(io, catalogue)?;
        read_line(io)
    })?;
    match line.trim() {
        "OK" => Ok(()),
        other => Err(other.trim_start_matches("NON ").to_owned()),
    }
}

/// **Lire le répertoire** — les seules cartes qui se vérifient, une par
/// officine, la plus récente.
pub fn list(address: &str, patience: Duration) -> Result<Vec<Fiche>, String> {
    let bytes = exchange(address, patience, |io| {
        io.write_all(preface(&Verbe::Liste).as_bytes())?;
        read_block(io, ANSWER_MAX)
    })?;
    let mut list: Vec<Fiche> = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    // Borné avant de vérifier : chaque vérification coûte.
    list.truncate(MOST_LISTED);
    let mut out: Vec<Fiche> = Vec::new();
    for f in list.into_iter().filter(|f| f.present && verify(f)) {
        match out.iter_mut().find(|o| o.device == f.device) {
            Some(o) if o.seq < f.seq => *o = f,
            Some(_) => {}
            None => out.push(f),
        }
    }
    Ok(out)
}

/// **Lire le catalogue d'une officine**, vérifié contre sa carte.
pub fn fetch_catalogue(
    address: &str,
    fiche: &Fiche,
    patience: Duration,
) -> Result<Vec<Article>, String> {
    let device = fiche.device.clone();
    let bytes = exchange(address, patience, |io| {
        io.write_all(preface(&Verbe::Catalogue(device)).as_bytes())?;
        read_block(io, CATALOGUE_MAX)
    })?;
    read_catalogue(fiche, &bytes).ok_or_else(|| "catalogue".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Une entrée supprimée ici ne se publie plus, même si le journal
    /// garde ses versions.
    #[test]
    fn a_deleted_entry_leaves_the_catalogue() {
        let edits = vec![
            edit(Kind::Codex, "Lugol", "form", "solution", ""),
            edit(Kind::Codex, "Dakin", "form", "solution", ""),
        ];
        let (all, _) = catalogue_of(&edits);
        let (kept, parts) = catalogue_where(&edits, |_, card| card == versions::key("Dakin"));
        let read = |b: &[u8]| serde_json::from_slice::<Vec<Article>>(b).unwrap().len();
        assert_eq!(read(&all), 2);
        assert_eq!(read(&kept), 1);
        assert_eq!(parts, vec![("codex".to_owned(), 1)]);
    }

    /// Un balayage lit les ouvertes d'abord, jamais soi-même, ni une
    /// officine absente, sans catalogue, ou déjà lue sous la même empreinte.
    /// Les bornes comptent un abonné : une adresse IPv4, présentée telle
    /// quelle ou par une prise double pile, est la même ; deux adresses
    /// d'un même `/64` sont un seul appelant.
    #[test]
    fn a_caller_is_a_subscriber_not_an_address() {
        let v4: IpAddr = "203.0.113.7".parse().unwrap();
        let mapped: IpAddr = "::ffff:203.0.113.7".parse().unwrap();
        assert_eq!(caller(mapped), v4);
        assert_eq!(caller(v4), v4);
        let a: IpAddr = "2a01:cb00:1:2:aaaa:bbbb:cccc:dddd".parse().unwrap();
        let b: IpAddr = "2a01:cb00:1:2::1".parse().unwrap();
        let other: IpAddr = "2a01:cb00:1:3::1".parse().unwrap();
        assert_eq!(caller(a), caller(b));
        assert_ne!(caller(a), caller(other));
    }

    /// Le répertoire répond en IPv6 comme en IPv4, chacune sur sa prise.
    #[test]
    fn the_directory_answers_on_both_stacks() {
        // Arrêté même si une assertion tombe.
        struct Stop(Arc<AtomicBool>);
        impl Drop for Stop {
            fn drop(&mut self) {
                self.0.store(true, Ordering::SeqCst);
            }
        }
        let stop = Stop(Arc::new(AtomicBool::new(false)));
        // Un port libre à l'instant peut être pris l'instant d'après :
        // quelques essais.
        let port = (0..20)
            .find_map(|_| {
                let port = TcpListener::bind(("127.0.0.1", 0))
                    .ok()?
                    .local_addr()
                    .ok()?
                    .port();
                serve(port, Arc::clone(&stop.0)).ok().map(|_| port)
            })
            .expect("un port libre");
        assert!(list(&format!("127.0.0.1:{port}"), Duration::from_secs(4)).is_ok());
        // Une machine sans IPv6 n'a pas de boucle `::1` : rien à dire.
        if TcpListener::bind(("::1", 0)).is_ok() {
            assert!(list(&format!("[::1]:{port}"), Duration::from_secs(4)).is_ok());
        }
    }

    /// Un réseau qui tient des milliers de `/64` ne remplit ni la table
    /// des appelants ni le répertoire.
    #[test]
    fn a_network_of_many_prefixes_is_one_neighbourhood() {
        let in_48 = |k: u16| -> IpAddr { format!("2001:470:1:{k:x}::1").parse().unwrap() };
        assert_eq!(network(in_48(1)), network(in_48(2000)));
        assert_ne!(caller(in_48(1)), caller(in_48(2)));
        let v4: IpAddr = "203.0.113.7".parse().unwrap();
        assert_eq!(network(v4), "203.0.113.0".parse::<IpAddr>().unwrap());
        let mut calls = Calls::default();
        let admitted = (0..2000).filter(|k| calls.admit(in_48(*k))).count();
        assert_eq!(admitted, CALLERS_PER_NETWORK);
        // Un autre réseau passe encore.
        assert!(calls.admit("2a01:cb00::1".parse().unwrap()));
        // Le répertoire : au plus LISTED_PER_NETWORK cartes d'un réseau.
        let mut store = Store::default();
        let now = 1_000_000;
        let mut listed = 0;
        for k in 0..200u16 {
            let d = device((k % 250) as u8 + 1);
            let fiche = sign(
                Fiche {
                    seq: now,
                    nom: format!("O{k}"),
                    present: true,
                    ..Fiche::default()
                },
                &d,
            );
            if store
                .admit(fiche, Vec::new(), 10, caller(in_48(k)), now)
                .is_ok()
            {
                listed += 1;
            }
        }
        assert_eq!(listed, LISTED_PER_NETWORK);
    }

    #[test]
    fn a_sweep_reads_the_open_ones_first_and_nothing_twice() {
        let now = 1_000_000;
        let fiche = |device: &str, age: u64, catalogue: &str, present: bool| Fiche {
            device: device.to_owned(),
            seq: now - age,
            catalogue: catalogue.to_owned(),
            parts: vec![("codex".to_owned(), 1)],
            present,
            ..Fiche::default()
        };
        let mut empty = fiche("e", 10, "d", true);
        empty.parts = vec![("codex".to_owned(), 0)];
        let list = vec![
            fiche("old", 7200, "d1", true),
            fiche("me", 10, "d2", true),
            fiche("gone", 10, "d3", false),
            fiche("bare", 10, "", true),
            fiche("open", 60, "d4", true),
            fiche("held", 30, "d5", true),
            fiche("older", 9000, "d6", true),
            empty,
        ];
        let held = |f: &Fiche| f.device == "held" && f.catalogue == "d5";
        let order = |list: &[Fiche], failed: &dyn Fn(&Fiche) -> bool, most: usize| -> Vec<String> {
            to_sweep(list, held, failed, "me", now, most)
                .iter()
                .map(|f| f.device.clone())
                .collect()
        };
        assert_eq!(order(&list, &|_| false, 10), ["open", "old", "older"]);
        assert_eq!(order(&list, &|_| false, 1).len(), 1);
        // Celle qui n'a pas répondu passe en dernier, ouverte ou non.
        assert_eq!(
            order(&list, &|f| f.device == "open", 10),
            ["old", "older", "open"]
        );
        // Une carte datée de demain ne passe pas devant une carte d'à
        // l'instant.
        let mut later = vec![fiche("now", 0, "d", true), fiche("future", 0, "d", true)];
        later[1].seq = now + 1000;
        assert_eq!(order(&later, &|_| false, 10), ["now", "future"]);
        let a = Article {
            kind: "codex".to_owned(),
            card: "x".to_owned(),
            name: "X".to_owned(),
            day: String::new(),
            fields: vec![("f".to_owned(), "v".repeat(100))],
        };
        assert!(weight(&[a]) > 100);
    }

    fn device(n: u8) -> bpm_sync::Device {
        bpm_sync::Device::from_seed([n; 32])
    }

    fn edit(kind: Kind, name: &str, field: &str, value: &str, source: &str) -> versions::Edit {
        versions::Edit {
            kind,
            uid: format!("{source}:{name}:{field}"),
            day: "2026-10-12".to_owned(),
            card: versions::key(name),
            card_name: name.to_owned(),
            field: field.to_owned(),
            value: value.to_owned(),
            previous: String::new(),
            corrects: String::new(),
            revert: false,
            operator: String::new(),
            source: source.to_owned(),
        }
    }

    fn card(n: u8, seq: u64, catalogue: &[u8], parts: Vec<(String, u32)>) -> Fiche {
        sign(
            Fiche {
                seq,
                nom: "Pharmacie de la Gare".to_owned(),
                commune: "Épinal".to_owned(),
                code_postal: "88000".to_owned(),
                parts,
                catalogue: digest(catalogue),
                present: true,
                ..Fiche::default()
            },
            &device(n),
        )
    }

    /// **Le catalogue est ce que l'officine a écrit, des sortes
    /// publiques** : ni ce qu'elle a seulement reçu, ni une liste, ni une
    /// phrase.
    #[test]
    fn the_catalogue_is_what_this_officine_wrote_and_nothing_private() {
        let edits = [
            edit(Kind::Codex, "Méthadone sirop", "form", "sirop", ""),
            edit(Kind::Codex, "Lugol", "form", "solution", "Pharmacie B"),
            edit(Kind::Liste, "Ouverture", "titre", "Ouverture", ""),
            edit(Kind::Texte, "ordo.pied", "value", "Pharmacie X, 01 02", ""),
            edit(Kind::Codex, "Sans champ", "notes", "à nous", ""),
        ];
        let (bytes, parts) = catalogue_of(&edits);
        let articles: Vec<Article> = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(articles.len(), 1, "{articles:?}");
        assert_eq!(articles[0].name, "Méthadone sirop");
        assert_eq!(parts, vec![("codex".to_owned(), 1)]);
    }

    /// **Une liste ou une phrase cochée voyage sous un nom à elle** : jamais
    /// `liste` ni `texte`, qu'une version d'avant refusait jusqu'à la carte
    /// entière ; un nom qu'elle ne connaît pas, fait de minuscules, passe
    /// chez qui héberge. Non cochée, elle ne part pas.
    #[test]
    fn a_ticked_list_travels_under_its_own_name() {
        let edits = [
            edit(Kind::Liste, "Ouverture", "titre", "Ouverture", ""),
            edit(Kind::Liste, "Fermeture", "titre", "Fermeture", ""),
        ];
        let (bytes, parts) = catalogue_with(
            &edits,
            |k, card| k == Kind::Liste && card == versions::key("Ouverture"),
            &[],
        );
        assert_eq!(parts, vec![("listepublique".to_owned(), 1)]);
        for k in [Kind::Liste, Kind::Texte] {
            let w = wire_key(k);
            assert!(w.bytes().all(|b| b.is_ascii_lowercase()));
            assert!(versions::kind_of_key(w).is_none());
            assert_eq!(kind_of_wire(w), Some(k));
            assert_eq!(kind_of_wire(k.key()), None);
        }
        let fiche = card(1, 10, &bytes, parts);
        assert!(check(&fiche, &bytes).is_ok());
        let read = read_catalogue(&fiche, &bytes).unwrap();
        assert_eq!(read.len(), 1);
        assert_eq!(offer_of(&read[0], "x").unwrap().kind, Kind::Liste);
    }

    /// **Les ruptures publiées voyagent dans le catalogue** : lues au nom de
    /// l'officine dont la carte est signée, jamais de celui qu'un
    /// événement écrirait ; un article dont la clé ne nomme pas son
    /// événement, ou qui ne se lit pas, est écarté seul.
    #[test]
    fn published_ruptures_travel_in_the_catalogue() {
        let e = crate::ruptures::Event {
            uid: "base:7".to_owned(),
            day: "2026-10-10".to_owned(),
            kind: crate::ruptures::Kind::Substitution,
            product: "Locoid crème".to_owned(),
            product_dci: String::new(),
            other: "Betneval crème".to_owned(),
            other_dci: String::new(),
            outcome: crate::ruptures::Outcome::Accepted,
            note: String::new(),
            operator: String::new(),
            source: String::new(),
            refers: String::new(),
        };
        let mut supply = supply_articles(&[e]);
        let mut forged = supply[0].clone();
        forged.card = "autre:1".to_owned();
        supply.push(forged);
        let mut broken = supply[0].clone();
        broken.card = "base:8".to_owned();
        broken.fields = vec![(SUPPLY_FIELD.to_owned(), "{pas un événement".to_owned())];
        supply.push(broken);
        let (bytes, parts) = catalogue_with(
            &[edit(Kind::Codex, "Lugol", "form", "solution", "")],
            |_, _| true,
            &supply,
        );
        assert_eq!(
            parts,
            vec![(SUPPLY_KIND.to_owned(), 1), ("codex".to_owned(), 1)]
        );
        let fiche = card(1, 10, &bytes, parts);
        let read = read_catalogue(&fiche, &bytes).unwrap();
        let events: Vec<_> = read
            .iter()
            .filter_map(|a| supply_of(a, "Pharmacie du Port"))
            .collect();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].source, "Pharmacie du Port");
        assert_eq!(events[0].other, "Betneval crème");
        // Un retrait ne se lit jamais sur la carte : sous le nom qu'une
        // carte se donne, il retirerait ici ce qu'une officine appairée
        // de ce nom a publié.
        let mut withdraw = supply_articles(&[crate::ruptures::Event {
            uid: "base:9".to_owned(),
            kind: crate::ruptures::Kind::Retrait,
            refers: "base:7".to_owned(),
            product: String::new(),
            ..events[0].clone()
        }]);
        assert!(supply_of(&withdraw[0], "x").is_none());
        withdraw[0].name = "Locoid crème".to_owned();
        assert!(supply_of(&withdraw[0], "x").is_none());
        assert_ne!(taken_uid("abc", "base:7"), "base:7");
        // Une rupture n'est pas une offre « En ligne ».
        assert!(read
            .iter()
            .filter(|a| a.kind == SUPPLY_KIND)
            .all(|a| offer_of(a, "x").is_none()));
    }

    /// **Une carte se vérifie, et ne se falsifie pas** : un mot changé,
    /// une autre clé, un catalogue échangé — refusés.
    #[test]
    fn a_card_is_signed_and_its_catalogue_bound_to_it() {
        let (bytes, parts) = catalogue_of(&[edit(Kind::Codex, "Lugol", "form", "solution", "")]);
        let fiche = card(1, 10, &bytes, parts);
        assert!(verify(&fiche));
        assert!(read_catalogue(&fiche, &bytes).is_some());
        let mut forged = fiche.clone();
        forged.nom = "Pharmacie du Port".to_owned();
        assert!(!verify(&forged), "un mot changé");
        let mut stolen = fiche.clone();
        stolen.device = hex(&device(2).id().0);
        assert!(!verify(&stolen), "une autre clé");
        let other = serde_json::to_vec(&vec![Article {
            kind: "codex".to_owned(),
            card: "lugol".to_owned(),
            name: "Lugol".to_owned(),
            day: String::new(),
            fields: vec![("form".to_owned(), "poison".to_owned())],
        }])
        .unwrap();
        assert!(
            read_catalogue(&fiche, &other).is_none(),
            "un catalogue échangé"
        );
        let mut long = fiche.clone();
        long.nom = "x".repeat(NAME_MAX + 1);
        let long = sign(long, &device(1));
        assert!(!verify(&long), "trop long, même signé");
    }

    /// **Ce qu'un catalogue reçu ne fait pas entrer** : une sorte privée,
    /// une sorte inconnue, un champ non partagé, un arbre illisible.
    #[test]
    fn a_received_catalogue_keeps_only_what_may_enter() {
        let articles = vec![
            Article {
                kind: "liste".to_owned(),
                card: "ouverture".to_owned(),
                name: "Ouverture".to_owned(),
                day: String::new(),
                fields: vec![("titre".to_owned(), "Ouverture".to_owned())],
            },
            Article {
                kind: "inconnue".to_owned(),
                card: "x".to_owned(),
                name: "X".to_owned(),
                day: String::new(),
                fields: vec![("form".to_owned(), "x".to_owned())],
            },
            Article {
                kind: "codex".to_owned(),
                card: "lugol".to_owned(),
                name: "Lugol".to_owned(),
                day: String::new(),
                fields: vec![
                    ("form".to_owned(), "solution".to_owned()),
                    ("notes".to_owned(), "à nous".to_owned()),
                ],
            },
            Article {
                kind: "protocole".to_owned(),
                card: "cystite".to_owned(),
                name: "Cystite".to_owned(),
                day: String::new(),
                fields: vec![("arbre".to_owned(), "pas un arbre".to_owned())],
            },
        ];
        let bytes = serde_json::to_vec(&articles).unwrap();
        let fiche = card(1, 1, &bytes, Vec::new());
        let kept = read_catalogue(&fiche, &bytes).unwrap();
        assert_eq!(kept.len(), 1, "{kept:?}");
        assert_eq!(
            kept[0].fields,
            vec![("form".to_owned(), "solution".to_owned())]
        );
        let offer = offer_of(&kept[0], "Pharmacie de la Gare").unwrap();
        assert_eq!(offer.kind, Kind::Codex);
        assert_eq!(offer.from, vec!["Pharmacie de la Gare"]);
    }

    const NOW: u64 = 1_800_000_000;

    fn lugol() -> (Vec<u8>, Vec<(String, u32)>) {
        catalogue_of(&[edit(Kind::Codex, "Lugol", "form", "solution", "")])
    }

    /// **Le répertoire garde la plus récente, et seulement elle** ; une
    /// officine qui part s'en va ; une adresse ne remplit pas tout, même
    /// en faisant passer ses cartes par une autre.
    #[test]
    fn the_store_keeps_the_newest_card_and_its_bounds() {
        let ip: IpAddr = "192.0.2.1".parse().unwrap();
        let other: IpAddr = "192.0.2.2".parse().unwrap();
        let mut store = Store::default();
        let (bytes, parts) = lugol();
        let take =
            |s: &mut Store, f: Fiche, b: &[u8], from: IpAddr| s.take(f, b.to_vec(), from, NOW);
        assert_eq!(
            take(&mut store, card(1, NOW, &bytes, parts.clone()), &bytes, ip),
            Ok(())
        );
        assert_eq!(
            take(
                &mut store,
                card(1, NOW - 1, &bytes, parts.clone()),
                &bytes,
                ip
            ),
            Err("ancienne")
        );
        assert_eq!(
            take(&mut store, card(1, NOW, &bytes, parts.clone()), &bytes, ip),
            Err("ancienne")
        );
        let mut forged = card(1, NOW + 1, &bytes, parts.clone());
        forged.commune = "Ailleurs".to_owned();
        assert_eq!(take(&mut store, forged, &bytes, ip), Err("signature"));
        assert_eq!(
            take(
                &mut store,
                card(1, NOW + 2, &bytes, parts.clone()),
                b"[]",
                ip
            ),
            Err("catalogue")
        );
        assert_eq!(store.list(NOW).len(), 1);
        assert_eq!(store.catalogue(&hex(&device(1).id().0), NOW), bytes);
        for n in 2..=LISTED_PER_ADDRESS as u8 {
            assert_eq!(
                take(&mut store, card(n, NOW, &bytes, parts.clone()), &bytes, ip),
                Ok(())
            );
        }
        assert_eq!(
            take(
                &mut store,
                card(200, NOW, &bytes, parts.clone()),
                &bytes,
                ip
            ),
            Err("adresse")
        );
        // Quinze cartes passées par une autre adresse, une seizième qui y
        // naît : la première carte ne peut plus y passer.
        for n in 2..=LISTED_PER_ADDRESS as u8 {
            assert_eq!(
                take(
                    &mut store,
                    card(n, NOW + 1, &bytes, parts.clone()),
                    &bytes,
                    other
                ),
                Ok(())
            );
        }
        assert_eq!(
            take(
                &mut store,
                card(100, NOW, &bytes, parts.clone()),
                &bytes,
                other
            ),
            Ok(())
        );
        assert_eq!(
            take(
                &mut store,
                card(1, NOW + 3, &bytes, parts.clone()),
                &bytes,
                other
            ),
            Err("adresse"),
            "une carte qui change d'adresse entre dans son compte"
        );
        let mut gone = card(1, NOW + 4, b"", Vec::new());
        gone.present = false;
        let gone = sign(gone, &device(1));
        assert_eq!(take(&mut store, gone, &[], ip), Ok(()));
        assert!(store.catalogue(&hex(&device(1).id().0), NOW).is_empty());
    }

    /// **Une carte ne se rejoue pas** : ni vieille d'une heure, ni après
    /// le départ de l'officine, ni signée dans un avenir lointain.
    #[test]
    fn a_saved_card_does_not_replay() {
        let ip: IpAddr = "192.0.2.1".parse().unwrap();
        let mut store = Store::default();
        let (bytes, parts) = lugol();
        let saved = card(1, NOW, &bytes, parts.clone());
        assert_eq!(store.take(saved.clone(), bytes.clone(), ip, NOW), Ok(()));
        let mut gone = card(1, NOW + 60, b"", Vec::new());
        gone.present = false;
        let gone = sign(gone, &device(1));
        assert_eq!(store.take(gone, Vec::new(), ip, NOW + 60), Ok(()));
        assert_eq!(
            store.take(saved.clone(), bytes.clone(), ip, NOW + 120),
            Err("ancienne"),
            "partie, elle ne revient pas par sa carte d'avant"
        );
        let mut fresh = Store::default();
        assert_eq!(
            fresh.take(saved, bytes.clone(), ip, NOW + FRESH + 1),
            Err("heure"),
            "rejouée trop tard"
        );
        assert_eq!(
            fresh.take(card(2, NOW + 2 * FRESH, &bytes, parts), bytes, ip, NOW),
            Err("heure"),
            "signée dans l'avenir"
        );
    }

    /// **Le catalogue dans sa forme exacte** : du remplissage, une sorte
    /// inconnue, un compte qui ment — refusés ; une carte qui annonce une
    /// sorte privée ou inconnue ne se vérifie même pas.
    #[test]
    fn a_padded_or_lying_catalogue_is_refused() {
        let ip: IpAddr = "192.0.2.1".parse().unwrap();
        let mut store = Store::default();
        let (bytes, parts) = lugol();
        let mut padded = bytes.clone();
        padded.splice(1..1, std::iter::repeat_n(b' ', 1000));
        assert_eq!(
            store.take(card(1, NOW, &padded, parts.clone()), padded, ip, NOW),
            Err("catalogue")
        );
        assert_eq!(
            store.take(
                card(1, NOW, &bytes, vec![("codex".to_owned(), 7)]),
                bytes.clone(),
                ip,
                NOW
            ),
            Err("catalogue")
        );
        assert!(!verify(&card(
            1,
            NOW,
            &bytes,
            vec![("liste".to_owned(), 1)]
        )));
        assert!(!verify(&card(1, NOW, &bytes, vec![("x".repeat(3000), 1)])));
        assert!(!verify(&card(
            1,
            NOW,
            &bytes,
            vec![("codex".to_owned(), 1), ("codex".to_owned(), 1)]
        )));
        let mut upper = card(1, NOW, &bytes, parts);
        upper.device = upper.device.to_uppercase();
        assert!(!verify(&upper), "une clé n'a qu'une écriture");
    }

    /// **Un catalogue que `catalogue_of` écrit passe toujours** : un nom
    /// trop long est écarté avant d'écrire, pas reproché au catalogue
    /// entier.
    #[test]
    fn what_catalogue_of_writes_is_always_accepted() {
        let ip: IpAddr = "192.0.2.1".parse().unwrap();
        let long = "Préparation ".repeat(60);
        let (bytes, parts) = catalogue_of(&[
            edit(Kind::Codex, &long, "form", "solution", ""),
            edit(Kind::Codex, "Lugol", "form", "solution", ""),
            edit(
                Kind::Codex,
                "Échappé \"guillemets\" \\ et\ttab",
                "form",
                "sirop",
                "",
            ),
        ]);
        assert_eq!(parts, vec![("codex".to_owned(), 2)]);
        let mut store = Store::default();
        assert_eq!(
            store.take(card(1, NOW, &bytes, parts), bytes, ip, NOW),
            Ok(())
        );
    }

    /// **Une version d'après reste sur la carte des versions d'avant** :
    /// une sorte qu'elles ne connaissent pas passe au répertoire, et seul
    /// qui lit l'écarte.
    #[test]
    fn a_newer_kind_does_not_hide_an_officine_from_older_versions() {
        let ip: IpAddr = "192.0.2.1".parse().unwrap();
        let articles = vec![
            Article {
                kind: "codex".to_owned(),
                card: "lugol".to_owned(),
                name: "Lugol".to_owned(),
                day: String::new(),
                fields: vec![("form".to_owned(), "solution".to_owned())],
            },
            Article {
                kind: "ordonnance".to_owned(),
                card: "x".to_owned(),
                name: "X".to_owned(),
                day: String::new(),
                fields: vec![("nouveau".to_owned(), "champ".to_owned())],
            },
        ];
        let bytes = serde_json::to_vec(&articles).unwrap();
        let parts = vec![("codex".to_owned(), 1), ("ordonnance".to_owned(), 1)];
        let fiche = card(1, NOW, &bytes, parts);
        assert!(verify(&fiche));
        let mut store = Store::default();
        assert_eq!(store.take(fiche.clone(), bytes.clone(), ip, NOW), Ok(()));
        assert_eq!(
            read_catalogue(&fiche, &bytes).unwrap().len(),
            1,
            "qui lit écarte"
        );
    }

    /// **Un départ ne se retient que pour qui était là** : des clés
    /// jetables qui « partent » sans avoir paru n'occupent rien.
    #[test]
    fn a_departure_of_someone_never_listed_leaves_nothing() {
        let ip: IpAddr = "192.0.2.1".parse().unwrap();
        let mut store = Store::default();
        for n in 1..=50 {
            let mut gone = card(n, NOW, b"", Vec::new());
            gone.present = false;
            let gone = sign(gone, &device(n));
            assert_eq!(store.take(gone, Vec::new(), ip, NOW), Ok(()));
        }
        assert!(store.gone.is_empty());
    }

    /// Le nom qu'une officine appairée a donné à une entrée ne sort pas
    /// non plus au nom de celle-ci.
    #[test]
    fn a_name_a_peer_wrote_is_not_published_as_ours() {
        let mut renamed = edit(
            Kind::Codex,
            "Lugol",
            "indication",
            "thyroïde",
            "Pharmacie B",
        );
        renamed.card_name = "Nom de B".to_owned();
        let edits = [edit(Kind::Codex, "Lugol", "form", "solution", ""), renamed];
        let (bytes, _) = catalogue_of(&edits);
        let articles: Vec<Article> = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(articles[0].name, "Lugol");
    }

    /// **Ce qu'une officine appairée a réécrit ne sort pas** au nom de
    /// celle-ci : seuls les champs dont la dernière version est d'ici.
    #[test]
    fn a_field_a_peer_rewrote_is_not_published_as_ours() {
        let edits = [
            edit(Kind::Codex, "Lugol", "form", "solution", ""),
            edit(Kind::Codex, "Lugol", "indication", "thyroïde", ""),
            edit(Kind::Codex, "Lugol", "form", "à 5 %", "Pharmacie B"),
        ];
        let (bytes, _) = catalogue_of(&edits);
        let articles: Vec<Article> = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            articles[0].fields,
            vec![("indication".to_owned(), "thyroïde".to_owned())]
        );
    }

    #[test]
    fn the_directory_address_takes_the_default_port_when_it_names_none() {
        let p = |a: &str| {
            Presence {
                adresse: a.to_owned(),
                ..Presence::default()
            }
            .address()
        };
        assert_eq!(
            p("annuaire.example.fr"),
            Some("annuaire.example.fr:7747".to_owned())
        );
        assert_eq!(p(" 192.0.2.4:9000 "), Some("192.0.2.4:9000".to_owned()));
        assert_eq!(p("2001:db8::1"), Some("[2001:db8::1]:7747".to_owned()));
        assert_eq!(
            p("[2001:db8::1]:9000"),
            Some("[2001:db8::1]:9000".to_owned())
        );
        assert_eq!(p(""), None);
        assert_eq!(p("deux mots"), None);
        let back = Presence::read(Some(
            &Presence {
                adresse: "x".to_owned(),
                paraitre: true,
                ..Presence::default()
            }
            .text(),
        ));
        assert!(back.paraitre);
        assert_eq!(Presence::read(Some("pas du json")), Presence::default());
        assert!(fingerprint(&hex(&device(1).id().0)).is_some());
        assert!(fingerprint("zz").is_none());
    }

    /// **Un départ n'est dû qu'à un répertoire qui a reçu une carte**,
    /// que l'officine quitte, et pas au-delà d'un jour.
    #[test]
    fn a_departure_is_owed_only_where_a_card_was_received_and_for_a_day() {
        let at = |paraitre: bool, adresse: &str, parue: &str, le: u64| Presence {
            adresse: adresse.to_owned(),
            paraitre,
            parue: parue.to_owned(),
            parue_le: le,
            ..Presence::default()
        };
        assert_eq!(
            at(true, "a", "a:7747", NOW).owed(NOW),
            None,
            "elle y paraît"
        );
        assert_eq!(
            at(true, "b", "a:7747", NOW).owed(NOW),
            Some("a:7747".to_owned())
        );
        assert_eq!(
            at(false, "a", "a:7747", NOW).owed(NOW),
            Some("a:7747".to_owned())
        );
        assert_eq!(at(false, "a", "", 0).owed(NOW), None, "aucune carte reçue");
        assert_eq!(
            at(false, "a", "a:7747", NOW).owed(NOW + FORGET.as_secs() + PARUE_REFRESH),
            None,
            "le répertoire a oublié"
        );
    }

    #[test]
    fn a_preface_reads_back_and_nothing_else_does() {
        for v in [
            Verbe::Annonce,
            Verbe::Liste,
            Verbe::Catalogue(hex(&device(1).id().0)),
        ] {
            assert_eq!(read_preface(preface(&v).trim_end()), Some(v));
        }
        assert_eq!(read_preface("BPMREPERTOIRE1 catalogue zz"), None);
        assert_eq!(read_preface("BPMREPERTOIRE1 liste de plus"), None);
        assert_eq!(read_preface("BPMRELAIS1 appel 00"), None);
    }

    /// **De bout en bout, sur une vraie prise** : une officine s'annonce,
    /// une autre lit le répertoire et le catalogue.
    #[test]
    fn an_officine_announces_and_another_reads_its_catalogue() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let stop = Arc::new(AtomicBool::new(false));
        serve(port, Arc::clone(&stop)).unwrap();
        let at = format!("127.0.0.1:{port}");
        let (bytes, parts) = catalogue_of(&[edit(Kind::Codex, "Lugol", "form", "solution", "")]);
        let fiche = card(1, unix_now(), &bytes, parts);
        let patience = Duration::from_secs(5);
        let mut said = announce(&at, &fiche, &bytes, patience);
        for _ in 0..20 {
            if said.is_ok() {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
            said = announce(&at, &fiche, &bytes, patience);
        }
        assert_eq!(said, Ok(()));
        assert_eq!(
            announce(&at, &fiche, &bytes, patience),
            Err("ancienne".to_owned())
        );
        let seen = list(&at, patience).unwrap();
        assert_eq!(seen.len(), 1);
        assert!(seen[0].online(unix_now()));
        assert_eq!(seen[0].nom, "Pharmacie de la Gare");
        let articles = fetch_catalogue(&at, &seen[0], patience).unwrap();
        assert_eq!(articles.len(), 1);
        assert_eq!(articles[0].name, "Lugol");
        stop.store(true, Ordering::SeqCst);
    }
}
