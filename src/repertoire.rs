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
//! * **Rien de privé n'y entre.** Ni patient, ni liste de contrôle, ni
//!   phrase imprimée, ni journal des ruptures : des fiches, préparations,
//!   protocoles, lignes de TROD, vaccins, dispositifs et cascades — ce
//!   que l'officine a de toute façon choisi d'écrire pour qu'on le lise.
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
            && now.saturating_sub(self.parue_le) < FORGET.as_secs())
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
    let mut articles = Vec::new();
    let mut parts = Vec::new();
    for kind in Kind::ALL.into_iter().filter(|k| !k.is_private()) {
        let own = crate::enligne::own_offers(edits, kind);
        let mut n = 0_u32;
        for o in own {
            if articles.len() >= ARTICLES_MAX {
                break;
            }
            // La règle de qui lit, appliquée avant d'écrire : un article
            // qu'un poste écarterait ferait refuser tout le catalogue.
            let Some(article) = admitted(Article {
                kind: kind.key().to_owned(),
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
            parts.push((kind.key().to_owned(), n));
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
    let kind = versions::kind_of_key(&a.kind).filter(|k| !k.is_private())?;
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
    let kind = versions::kind_of_key(&article.kind)?;
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
        for (device, l) in &self.listed {
            if *device != fiche.device && l.from == from {
                here += 1;
                here_bytes += l.weight;
            }
        }
        if here >= LISTED_PER_ADDRESS || here_bytes > BYTES_PER_ADDRESS {
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
        if self.seen.len() >= 8192 && !self.seen.contains_key(&ip) {
            return false;
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

/// **Tenir le répertoire** sur `port` jusqu'à ce que `stop` passe à vrai.
/// Rend tout de suite ; le répertoire vit sur ses propres fils.
pub fn serve(port: u16, stop: Arc<AtomicBool>) -> std::io::Result<()> {
    let listener = TcpListener::bind(("0.0.0.0", port))?;
    listener.set_nonblocking(true)?;
    let store: Arc<Mutex<Store>> = Arc::default();
    let mut calls = Calls::default();
    let open = Arc::new(AtomicUsize::new(0));
    // Les connexions ouvertes par adresse : une adresse n'en tient que
    // quelques-unes, jamais toutes.
    let by_address: Arc<Mutex<HashMap<IpAddr, usize>>> = Arc::default();
    std::thread::spawn(move || {
        while !stop.load(Ordering::SeqCst) {
            let (stream, from) = match listener.accept() {
                Ok(s) => s,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(100));
                    continue;
                }
                Err(_) => {
                    std::thread::sleep(Duration::from_millis(500));
                    continue;
                }
            };
            let ip = from.ip();
            if !calls.admit(ip) || open.load(Ordering::SeqCst) >= MOST_CONNECTIONS {
                continue;
            }
            let held = by_address.lock().map(|mut m| {
                let n = m.entry(ip).or_default();
                if *n >= OPEN_PER_ADDRESS {
                    false
                } else {
                    *n += 1;
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
                    if let Some(n) = m.get_mut(&ip) {
                        *n = n.saturating_sub(1);
                        if *n == 0 {
                            m.remove(&ip);
                        }
                    }
                }
            });
        }
    });
    Ok(())
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
            at(false, "a", "a:7747", NOW).owed(NOW + FORGET.as_secs()),
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
