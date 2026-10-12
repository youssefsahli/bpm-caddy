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
/// …et passe pour **ouverte** tant que sa dernière annonce a moins de
/// ceci. Le poste annonce toutes les [`ANNOUNCE_EVERY`].
pub const ONLINE: Duration = Duration::from_secs(25 * 60);
pub const ANNOUNCE_EVERY: Duration = Duration::from_secs(10 * 60);

/// Les bornes d'une carte de visite.
pub const NAME_MAX: usize = 80;
/// Un catalogue porte au plus ceci, en octets…
pub const CATALOGUE_MAX: usize = 2 * 1024 * 1024;
/// …et autant d'entrées, d'au plus ceci par valeur.
pub const ARTICLES_MAX: usize = 5000;
const VALUE_MAX: usize = 64 * 1024;
/// Combien d'officines le répertoire tient, en tout et par adresse.
const MOST_LISTED: usize = 2000;
const LISTED_PER_ADDRESS: usize = 16;
/// Ce que le répertoire garde en tout, catalogues compris.
const MOST_BYTES: usize = 256 * 1024 * 1024;
const MOST_CONNECTIONS: usize = 64;
const CALLS_PER_MINUTE: usize = 30;
/// Le temps de dire sa demande entière, catalogue compris.
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

/// Une officine telle que le répertoire la rend : sa carte, et depuis
/// combien de secondes elle ne s'est pas annoncée.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Vue {
    pub fiche: Fiche,
    pub age: u64,
}

impl Vue {
    /// Ouverte en ce moment, à peu de chose près.
    pub fn online(&self) -> bool {
        self.age < ONLINE.as_secs()
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex<const N: usize>(text: &str) -> Option<[u8; N]> {
    if text.len() != N * 2 || !text.is_ascii() {
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
            let fields: Vec<(String, String)> = o
                .fields
                .into_iter()
                .filter(|(f, v)| v.len() <= VALUE_MAX && versions::admissible(kind, f, v))
                .collect();
            if fields.is_empty() {
                continue;
            }
            n += 1;
            articles.push(Article {
                kind: kind.key().to_owned(),
                card: o.card,
                name: o.name,
                day: o.day,
                fields,
            });
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
    f.nom.chars().count() <= NAME_MAX
        && f.commune.chars().count() <= NAME_MAX
        && printable(&f.nom)
        && printable(&f.commune)
        && (f.code_postal.is_empty()
            || (f.code_postal.len() == 5 && f.code_postal.chars().all(|c| c.is_ascii_digit())))
        && (f.finess.is_empty()
            || (f.finess.len() == 9 && f.finess.chars().all(|c| c.is_ascii_alphanumeric())))
        && f.parts.len() <= Kind::ALL.len()
        && (!f.present || !f.nom.trim().is_empty())
        && (f.catalogue.is_empty() || unhex::<32>(&f.catalogue).is_some())
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
            .filter_map(|mut a| {
                let kind = versions::kind_of_key(&a.kind).filter(|k| !k.is_private())?;
                a.fields
                    .retain(|(f, v)| v.len() <= VALUE_MAX && versions::admissible(kind, f, v));
                (!a.fields.is_empty()
                    && !a.card.trim().is_empty()
                    && !a.name.trim().is_empty()
                    && a.name.len() <= 400)
                    .then_some(a)
            })
            .collect(),
    )
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
    seen: Instant,
    from: IpAddr,
}

/// L'état du répertoire, partagé entre ses fils.
#[derive(Default)]
pub struct Store {
    listed: HashMap<String, Listed>,
}

impl Store {
    fn bytes(&self) -> usize {
        self.listed.values().map(|l| l.catalogue.len() + 1024).sum()
    }

    fn prune(&mut self) {
        self.listed.retain(|_, l| l.seen.elapsed() < FORGET);
    }

    /// **Garder une annonce** si elle est signée, bien formée, plus
    /// récente que celle qu'on tient, et que les bornes le permettent.
    /// Rend le refus en un mot.
    pub fn take(
        &mut self,
        fiche: Fiche,
        catalogue: Vec<u8>,
        from: IpAddr,
    ) -> Result<(), &'static str> {
        self.prune();
        if !verify(&fiche) {
            return Err("signature");
        }
        if let Some(held) = self.listed.get(&fiche.device) {
            if fiche.seq <= held.fiche.seq {
                return Err("ancienne");
            }
        }
        if !fiche.present {
            self.listed.remove(&fiche.device);
            return Ok(());
        }
        if read_catalogue(&fiche, &catalogue).is_none() {
            return Err("catalogue");
        }
        let known = self.listed.contains_key(&fiche.device);
        if !known {
            if self.listed.len() >= MOST_LISTED {
                return Err("plein");
            }
            let here = self.listed.values().filter(|l| l.from == from).count();
            if here >= LISTED_PER_ADDRESS {
                return Err("adresse");
            }
        }
        let held = self
            .listed
            .get(&fiche.device)
            .map_or(0, |l| l.catalogue.len() + 1024);
        if self.bytes() - held + catalogue.len() + 1024 > MOST_BYTES {
            return Err("plein");
        }
        self.listed.insert(
            fiche.device.clone(),
            Listed {
                fiche,
                catalogue,
                seen: Instant::now(),
                from,
            },
        );
        Ok(())
    }

    /// Les officines du répertoire, la plus récemment annoncée d'abord.
    pub fn list(&mut self) -> Vec<Vue> {
        self.prune();
        let mut out: Vec<Vue> = self
            .listed
            .values()
            .map(|l| Vue {
                fiche: l.fiche.clone(),
                age: l.seen.elapsed().as_secs(),
            })
            .collect();
        out.sort_by_key(|v| v.age);
        out
    }

    pub fn catalogue(&mut self, device: &str) -> Vec<u8> {
        self.prune();
        self.listed
            .get(device)
            .map(|l| l.catalogue.clone())
            .unwrap_or_default()
    }
}

/// **Tenir le répertoire** sur `port` jusqu'à ce que `stop` passe à vrai.
/// Rend tout de suite ; le répertoire vit sur ses propres fils.
pub fn serve(port: u16, stop: Arc<AtomicBool>) -> std::io::Result<()> {
    let listener = TcpListener::bind(("0.0.0.0", port))?;
    listener.set_nonblocking(true)?;
    let store: Arc<Mutex<Store>> = Arc::default();
    let calls: Arc<Mutex<HashMap<IpAddr, Vec<Instant>>>> = Arc::default();
    let open = Arc::new(AtomicUsize::new(0));
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
            if !admit(&calls, from.ip()) || open.load(Ordering::SeqCst) >= MOST_CONNECTIONS {
                continue;
            }
            open.fetch_add(1, Ordering::SeqCst);
            let (store, open) = (Arc::clone(&store), Arc::clone(&open));
            std::thread::spawn(move || {
                let _ = answer(stream, from.ip(), &store);
                open.fetch_sub(1, Ordering::SeqCst);
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
    if calls.len() >= 1024 && !calls.contains_key(&ip) {
        return false;
    }
    let seen = calls.entry(ip).or_default();
    seen.push(now);
    seen.len() <= CALLS_PER_MINUTE
}

/// Une lecture qui ne dépasse jamais l'échéance de la demande entière :
/// un octet toutes les quelques secondes ne tient pas un fil une heure.
struct Deadline<'a> {
    stream: &'a TcpStream,
    until: Instant,
}

impl Read for Deadline<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let left = self.until.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(std::io::Error::from(std::io::ErrorKind::TimedOut));
        }
        self.stream.set_read_timeout(Some(left))?;
        let mut s = self.stream;
        s.read(buf)
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

/// Une demande reçue, et sa réponse.
fn answer(mut stream: TcpStream, from: IpAddr, store: &Mutex<Store>) -> std::io::Result<()> {
    stream.set_nonblocking(false)?;
    stream.set_write_timeout(Some(REQUEST_WAIT))?;
    let mut r = Deadline {
        stream: &stream,
        until: Instant::now() + REQUEST_WAIT,
    };
    let Some(verbe) = read_preface(&read_line(&mut r)?) else {
        return Ok(());
    };
    match verbe {
        Verbe::Annonce => {
            let fiche = read_block(&mut r, 64 * 1024)?;
            let catalogue = read_block(&mut r, CATALOGUE_MAX)?;
            let said = match serde_json::from_slice::<Fiche>(&fiche) {
                Ok(fiche) => match store.lock() {
                    Ok(mut s) => s.take(fiche, catalogue, from),
                    Err(_) => Err("plein"),
                },
                Err(_) => Err("illisible"),
            };
            let line = match said {
                Ok(()) => "OK\n".to_owned(),
                Err(why) => format!("NON {why}\n"),
            };
            stream.write_all(line.as_bytes())
        }
        Verbe::Liste => {
            let list = store.lock().map(|mut s| s.list()).unwrap_or_default();
            write_block(&mut stream, &serde_json::to_vec(&list).unwrap_or_default())
        }
        Verbe::Catalogue(device) => {
            let bytes = store
                .lock()
                .map(|mut s| s.catalogue(&device))
                .unwrap_or_default();
            write_block(&mut stream, &bytes)
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
            Ok(s) => {
                s.set_read_timeout(Some(patience))
                    .map_err(|e| e.to_string())?;
                s.set_write_timeout(Some(patience))
                    .map_err(|e| e.to_string())?;
                return Ok(s);
            }
            Err(e) => last = e.to_string(),
        }
    }
    Err(last)
}

/// **S'annoncer** : la carte signée et son catalogue.
pub fn announce(
    address: &str,
    fiche: &Fiche,
    catalogue: &[u8],
    patience: Duration,
) -> Result<(), String> {
    let mut s = dial(address, patience)?;
    s.write_all(preface(&Verbe::Annonce).as_bytes())
        .map_err(|e| e.to_string())?;
    write_block(
        &mut s,
        &serde_json::to_vec(fiche).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    write_block(&mut s, catalogue).map_err(|e| e.to_string())?;
    let line = read_line(&mut s).map_err(|e| e.to_string())?;
    match line.trim() {
        "OK" => Ok(()),
        other => Err(other.trim_start_matches("NON ").to_owned()),
    }
}

/// **Lire le répertoire** — les seules cartes qui se vérifient.
pub fn list(address: &str, patience: Duration) -> Result<Vec<Vue>, String> {
    let mut s = dial(address, patience)?;
    s.write_all(preface(&Verbe::Liste).as_bytes())
        .map_err(|e| e.to_string())?;
    let bytes = read_block(&mut s, ANSWER_MAX).map_err(|e| e.to_string())?;
    let list: Vec<Vue> = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    Ok(list
        .into_iter()
        .filter(|v| v.fiche.present && verify(&v.fiche))
        .take(MOST_LISTED)
        .collect())
}

/// **Lire le catalogue d'une officine**, vérifié contre sa carte.
pub fn fetch_catalogue(
    address: &str,
    fiche: &Fiche,
    patience: Duration,
) -> Result<Vec<Article>, String> {
    let mut s = dial(address, patience)?;
    s.write_all(preface(&Verbe::Catalogue(fiche.device.clone())).as_bytes())
        .map_err(|e| e.to_string())?;
    let bytes = read_block(&mut s, CATALOGUE_MAX).map_err(|e| e.to_string())?;
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

    /// **Le répertoire garde la plus récente, et seulement elle** ; une
    /// officine qui part s'en va ; une adresse ne remplit pas tout.
    #[test]
    fn the_store_keeps_the_newest_card_and_its_bounds() {
        let ip: IpAddr = "192.0.2.1".parse().unwrap();
        let mut store = Store::default();
        let (bytes, parts) = catalogue_of(&[edit(Kind::Codex, "Lugol", "form", "solution", "")]);
        assert_eq!(
            store.take(card(1, 10, &bytes, parts.clone()), bytes.clone(), ip),
            Ok(())
        );
        assert_eq!(
            store.take(card(1, 9, &bytes, parts.clone()), bytes.clone(), ip),
            Err("ancienne")
        );
        assert_eq!(
            store.take(card(1, 10, &bytes, parts.clone()), bytes.clone(), ip),
            Err("ancienne")
        );
        let mut forged = card(1, 11, &bytes, parts.clone());
        forged.commune = "Ailleurs".to_owned();
        assert_eq!(store.take(forged, bytes.clone(), ip), Err("signature"));
        assert_eq!(
            store.take(card(1, 12, &bytes, parts.clone()), b"[]".to_vec(), ip),
            Err("catalogue")
        );
        assert_eq!(store.list().len(), 1);
        assert_eq!(store.catalogue(&hex(&device(1).id().0)), bytes);
        for n in 2..=LISTED_PER_ADDRESS as u8 {
            assert_eq!(
                store.take(card(n, 1, &bytes, parts.clone()), bytes.clone(), ip),
                Ok(())
            );
        }
        assert_eq!(
            store.take(card(200, 1, &bytes, parts.clone()), bytes.clone(), ip),
            Err("adresse")
        );
        let other: IpAddr = "192.0.2.2".parse().unwrap();
        assert_eq!(
            store.take(card(200, 1, &bytes, parts.clone()), bytes.clone(), other),
            Ok(())
        );
        let mut gone = card(1, 20, b"", Vec::new());
        gone.present = false;
        let gone = sign(gone, &device(1));
        assert_eq!(store.take(gone, Vec::new(), ip), Ok(()));
        assert!(store.catalogue(&hex(&device(1).id().0)).is_empty());
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
        let fiche = card(1, 10, &bytes, parts);
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
        assert!(seen[0].online());
        assert_eq!(seen[0].fiche.nom, "Pharmacie de la Gare");
        let articles = fetch_catalogue(&at, &seen[0].fiche, patience).unwrap();
        assert_eq!(articles.len(), 1);
        assert_eq!(articles[0].name, "Lugol");
        stop.store(true, Ordering::SeqCst);
    }
}
