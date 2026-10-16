//! **La messagerie des patients** : ce qu'un patient et l'officine se
//! disent, chiffré de bout en bout entre la page du patient et un poste
//! de l'officine, à travers une passerelle qui ne garde rien
//! (`passerelle.rs`, `docs/CHAT_PATIENT.md`).
//!
//! Le patient reçoit de l'officine, en code QR, une **clé à lui** : 32
//! octets tirés au hasard, rangés dans la base avec son dossier. Le lien
//! du code porte la clé dans son fragment (`#…`), que le navigateur
//! n'envoie à personne. La passerelle ne voit que des octets chiffrés et
//! le numéro de la boîte de l'officine.
//!
//! Une conversation, toujours la même, en quatre trames :
//!
//! 1. la page dit bonjour : `BPMP1`, un nombre tiré au hasard, et la
//!    preuve qu'elle tient une clé (HMAC du nombre) — sans dire laquelle :
//!    le poste essaie celles de l'officine ;
//! 2. le poste répond par son nombre et la preuve qu'il tient la même ;
//! 3. la page envoie, scellé, ce qui attendait sur le téléphone et les
//!    numéros des messages de l'officine qu'elle a déjà ;
//! 4. le poste range, et rend, scellé, ce qu'il a rangé et ce que
//!    l'officine a écrit qu'elle n'a pas — par numéros, pas par « depuis » :
//!    chaque poste tire les siens de son bloc, et un numéro plus petit
//!    n'est pas un message plus ancien.
//!
//! Chaque sens a sa clé, tirée de la clé du patient et des deux nombres
//! (HKDF) : une conversation enregistrée et rejouée ne s'ouvre pas, faute
//! du nombre du poste. Les primitives sont celles de WebCrypto
//! (AES-256-GCM, HKDF et HMAC sur SHA-256), que la page appelle sans
//! bibliothèque.
//!
//! Ce module ne touche ni base ni réseau : la base passe par [`Store`],
//! la connexion par [`Frames`].

use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, KeyInit};
use base64::Engine;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Les cinq octets qui ouvrent le bonjour d'une page.
pub const MAGIC: &[u8; 5] = b"BPMP1";
/// **La demande du tableau** (`tableau.rs`), seule dans sa trame : le
/// poste rend [`crate::tableau::Public`] en JSON, en clair — ce que
/// l'officine affiche pour tous. Pas de clé : le code du tableau n'en
/// porte pas. Ce qui le garantit venu de l'officine, c'est la voie
/// directe (l'empreinte du certificat, dans le lien) : la page ne le
/// demande jamais par une passerelle, qui pourrait changer la pharmacie
/// de garde. Le tableau qu'un patient reçoit dans sa conversation, lui,
/// est scellé avec elle.
pub const BOARD: &[u8; 5] = b"BPMT1";
/// La réponse d'un poste à une clé que l'officine a fermée — suivie de
/// [`closed_tag`] : la page ne s'en tient pour fermée que signée par qui
/// tient sa clé. Nue, une passerelle pouvait fermer un patient pour de
/// bon.
pub const CLOSED: &[u8] = b"FERME";
/// La réponse d'un poste à une clé qu'il ne connaît pas.
pub const UNKNOWN: &[u8] = b"INCONNU";
/// Le plus long message, en caractères — ce que la page laisse taper.
pub const TEXT_MAX: usize = 2000;
/// Combien de messages une page envoie en une conversation, au plus.
pub const OUT_MAX: usize = 20;
/// Combien de messages de l'officine un poste rend en une conversation —
/// les plus récents.
pub const IN_MAX: usize = 50;
/// Combien de numéros déjà reçus une page rappelle, au plus — les plus
/// récents ; un message plus ancien rendu de nouveau, elle l'a déjà.
pub const SEEN_MAX: usize = 500;
/// La plus grande trame. Une réponse pleine (cinquante messages de deux
/// mille caractères, en JSON) y tient.
pub const FRAME_MAX: usize = 256 * 1024;

const SALT: &[u8] = b"bpm-caddy-patient-1";
const NONCE: usize = 16;
const TAG: usize = 32;

/// La clé d'un patient.
pub type Key = [u8; 32];

type HmacSha256 = Hmac<Sha256>;

fn hkdf(salt: &[u8], key: &Key, info: &str) -> [u8; 32] {
    let mut out = [0u8; 32];
    // 32 bytes is always a valid HKDF-SHA256 output length.
    let _ = hkdf::Hkdf::<Sha256>::new(Some(salt), key).expand(info.as_bytes(), &mut out);
    out
}

fn mac(key: &[u8; 32], parts: &[&[u8]]) -> HmacSha256 {
    // HMAC takes a key of any length.
    let mut m = <HmacSha256 as Mac>::new_from_slice(key).expect("HMAC accepts any key length");
    for p in parts {
        m.update(p);
    }
    m
}

fn auth_key(key: &Key) -> [u8; 32] {
    hkdf(SALT, key, "auth")
}

/// La preuve de la page : elle tient `key`.
pub fn client_tag(key: &Key, nc: &[u8; NONCE]) -> [u8; TAG] {
    mac(&auth_key(key), &[b"client", nc])
        .finalize()
        .into_bytes()
        .into()
}

/// La preuve du poste : il tient la même.
pub fn server_tag(key: &Key, nc: &[u8; NONCE], ns: &[u8; NONCE]) -> [u8; TAG] {
    mac(&auth_key(key), &[b"server", nc, ns])
        .finalize()
        .into_bytes()
        .into()
}

/// La signature de « fermé » : le poste tient la clé fermée.
pub fn closed_tag(key: &Key, nc: &[u8; NONCE]) -> [u8; TAG] {
    mac(&auth_key(key), &[b"ferme", nc])
        .finalize()
        .into_bytes()
        .into()
}

fn verify(key: &[u8; 32], parts: &[&[u8]], tag: &[u8]) -> bool {
    mac(key, parts).verify_slice(tag).is_ok()
}

/// Les deux clés d'une conversation : de la page au poste, du poste à la
/// page.
pub fn session(key: &Key, nc: &[u8; NONCE], ns: &[u8; NONCE]) -> (Sealer, Sealer) {
    let mut salt = [0u8; 2 * NONCE];
    salt[..NONCE].copy_from_slice(nc);
    salt[NONCE..].copy_from_slice(ns);
    (
        Sealer::new(&hkdf(&salt, key, "patient>officine")),
        Sealer::new(&hkdf(&salt, key, "officine>patient")),
    )
}

/// Un sens de la conversation : une clé, et un compteur qui fait le
/// nombre de chaque trame (jamais deux fois le même sous une clé).
pub struct Sealer {
    cipher: Aes256Gcm,
    counter: u64,
}

impl Sealer {
    fn new(key: &[u8; 32]) -> Self {
        Sealer {
            cipher: Aes256Gcm::new(key.into()),
            counter: 0,
        }
    }

    fn nonce(&mut self) -> [u8; 12] {
        let mut n = [0u8; 12];
        n[4..].copy_from_slice(&self.counter.to_be_bytes());
        self.counter += 1;
        n
    }

    pub fn seal(&mut self, plain: &[u8]) -> Vec<u8> {
        let n = self.nonce();
        // AES-GCM encryption only fails past 64 GiB.
        self.cipher.encrypt((&n).into(), plain).unwrap_or_default()
    }

    pub fn open(&mut self, sealed: &[u8]) -> Option<Vec<u8>> {
        let n = self.nonce();
        self.cipher.decrypt((&n).into(), sealed).ok()
    }
}

/// Une nouvelle clé de patient.
pub fn new_key(fill: impl FnOnce(&mut [u8])) -> Key {
    let mut k = [0u8; 32];
    fill(&mut k);
    k
}

/// La clé écrite dans la base et dans le lien : base64 pour URL, sans
/// remplissage (43 caractères).
pub fn key_text(key: &Key) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(key)
}

/// Relire une clé écrite par [`key_text`].
pub fn key_from_text(text: &str) -> Option<Key> {
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(text.trim())
        .ok()?
        .try_into()
        .ok()
}

/// **Le numéro de la boîte** d'une officine, tiré de son secret. Le
/// numéro est dans chaque code remis ; le secret ne quitte pas les postes,
/// qui le disent à la passerelle pour y attendre — qui a photographié un
/// code ne peut donc pas attendre à la place de l'officine.
pub fn box_id(secret: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(b"bpm-caddy-boite");
    h.update(secret);
    hex(&h.finalize()[..16])
}

/// La boîte d'une officine depuis son secret tel que la base le range
/// (64 chiffres hexadécimaux) ; `None` pour autre chose.
pub fn box_of_secret(secret: &str) -> Option<String> {
    if !is_box_secret(secret) {
        return None;
    }
    let bytes: Vec<u8> = (0..secret.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(secret.get(i..i + 2)?, 16).ok())
        .collect::<Option<_>>()?;
    Some(box_id(&bytes))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Un numéro de boîte bien formé : trente-deux chiffres hexadécimaux.
pub fn is_box_id(text: &str) -> bool {
    text.len() == 32 && text.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Un secret de boîte bien formé : soixante-quatre chiffres
/// hexadécimaux.
pub fn is_box_secret(text: &str) -> bool {
    text.len() == 64 && text.bytes().all(|b| b.is_ascii_hexdigit())
}

/// **Où la page est publiée** : GitHub Pages, depuis le dépôt public
/// (`.github/workflows/page.yml`), et non par la passerelle. Qui tient la
/// passerelle ne touche pas au code qui chiffre : il ne voit passer que
/// des trames scellées. Chaque version de la page a son dossier, nommé
/// par son empreinte (`page_hash`), qui ne change plus une fois publié —
/// un code imprimé l'an passé ouvre la page de l'an passé.
pub const PAGE_SITE: &str = "https://bpmcaddy.github.io/";

/// La page, telle que ce logiciel la livre : ce qu'un poste compare à
/// ce que `PAGE_SITE` sert (`patientpost::check_page`).
pub const PAGE_FILES: [(&str, &str); 3] = [
    ("index.html", include_str!("../assets/patient/index.html")),
    ("app.js", include_str!("../assets/patient/app.js")),
    ("style.css", include_str!("../assets/patient/style.css")),
];

/// **L'empreinte de la page** : SHA-256 de ses trois fichiers mis bout à
/// bout dans l'ordre de `PAGE_FILES`, seize chiffres hexadécimaux — ce
/// que calcule le workflow qui la publie
/// (`cat index.html app.js style.css | sha256sum | cut -c1-16`).
pub fn page_hash() -> String {
    hash_of_page(PAGE_FILES.map(|(_, body)| body.as_bytes()))
}

/// L'empreinte de trois fichiers de page, dans l'ordre de `PAGE_FILES`.
pub fn hash_of_page(files: [&[u8]; 3]) -> String {
    let mut h = Sha256::new();
    for f in files {
        h.update(f);
    }
    hex(&h.finalize()[..8])
}

/// La ligne d'`app.js` qui dit une page publiée à part de la passerelle :
/// le workflow ne publie que les versions qui la portent — les plus
/// anciennes se servaient de la passerelle et ne la trouveraient pas.
pub const PAGE_MARK: &str = "// bpm-page: hors passerelle";

/// L'adresse de cette version de la page.
pub fn page_url() -> String {
    format!("{PAGE_SITE}{}/", page_hash())
}

/// **Comment la page joint l'officine** : la voie directe (l'empreinte
/// du certificat des postes et leurs adresses, `direct.rs`), une
/// passerelle (un nom d'hôte), ou les deux — la page essaie la voie
/// directe d'abord.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Reach {
    /// L'empreinte, en base64 (`direct::fingerprint_text`) ; vide : pas
    /// de voie directe.
    pub fingerprint: String,
    pub addresses: Vec<String>,
    /// Le nom d'hôte de la passerelle ; vide : aucune.
    pub gateway: String,
}

/// Combien d'adresses un lien porte, au plus : un code QR reste lisible.
pub const LINK_ADDRESSES: usize = 6;

impl Reach {
    /// Le fragment qui le dit : `f`, `a` (séparées par des virgules), `p`.
    fn fragment(&self) -> String {
        let mut out = String::new();
        if !self.fingerprint.is_empty() && !self.addresses.is_empty() {
            let a: Vec<&str> = self
                .addresses
                .iter()
                .map(String::as_str)
                .take(LINK_ADDRESSES)
                .collect();
            out.push_str(&format!("&f={}&a={}", self.fingerprint, a.join(",")));
        }
        let host = gateway_host(&self.gateway);
        if !host.is_empty() {
            out.push_str(&format!("&p={host}"));
        }
        out
    }

    /// Un chemin au moins.
    pub fn any(&self) -> bool {
        !self.fragment().is_empty()
    }
}

/// **L'adresse de la box écrite à la main**, telle que la page la
/// compose : une IPv4 (`203.0.113.7`), une IPv6 (`2001:db8::1` ou entre
/// crochets) ou l'une d'elles avec son port ; sans port, 7748. `None`
/// pour autre chose — un nom ne se compose pas en WebRTC sans serveur.
pub fn box_address(typed: &str) -> Option<String> {
    let t = typed.trim();
    if let Ok(a) = t.parse::<std::net::SocketAddr>() {
        return Some(a.to_string());
    }
    let ip: std::net::IpAddr = t
        .trim_start_matches('[')
        .trim_end_matches(']')
        .parse()
        .ok()?;
    Some(std::net::SocketAddr::new(ip, crate::direct::DEFAULT_PORT).to_string())
}

/// **Le lien du code QR** : la page publiée, puis dans le fragment la
/// boîte, la clé et le chemin ([`Reach`]) — un fragment n'est envoyé à
/// personne, pas même à GitHub.
pub fn link(reach: &Reach, box_id: &str, key: &Key) -> String {
    format!(
        "{}#b={box_id}&k={}{}",
        page_url(),
        key_text(key),
        reach.fragment()
    )
}

/// **Le lien du tableau** (`tableau.rs`) : la boîte et la voie directe,
/// sans clé — le code de la vitrine ou du site de l'officine. Jamais la
/// passerelle : rien n'y garantit le tableau venu de l'officine.
/// `screen` : la page en écran, en grand, qui se relit seule.
pub fn board_link(reach: &Reach, box_id: &str, screen: bool) -> String {
    let direct = Reach {
        gateway: String::new(),
        ..reach.clone()
    };
    format!(
        "{}#b={box_id}&t=1{}{}",
        page_url(),
        if screen { "&e=1" } else { "" },
        direct.fragment()
    )
}

/// Le nom d'hôte d'une passerelle telle qu'on l'a tapée : sans schéma,
/// sans chemin, sans espace. Vide quand il n'en reste rien.
pub fn gateway_host(typed: &str) -> String {
    let t = typed.trim();
    let t = t
        .strip_prefix("https://")
        .or_else(|| t.strip_prefix("http://"))
        .unwrap_or(t);
    let t = t.split(['/', '#', '?']).next().unwrap_or("");
    if t.chars()
        .all(|c| c.is_ascii_alphanumeric() || ".-:[]".contains(c))
    {
        t.to_ascii_lowercase()
    } else {
        String::new()
    }
}

/// **Les conditions d'une passerelle tenue pour l'officine** par un
/// prestataire : le contrat de sous-traitance (art. 28 RGPD), qu'une
/// acceptation électronique conclut si on en garde la preuve — le poste,
/// le jour et l'heure, la version du texte (`docs/CHAT_PATIENT.md`).
///
/// Elles ne valent que pour la passerelle livrée avec la version, et
/// seulement quand un texte est livré avec elle : une passerelle que
/// l'officine tient elle-même, ou qu'elle a choisie ailleurs, a son
/// propre contrat, hors du logiciel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Terms {
    /// Rien à accepter.
    #[default]
    None,
    /// À accepter avant que la passerelle serve l'officine.
    ToAccept,
    /// Acceptées, dans la version livrée.
    Accepted,
    /// Acceptées dans une version d'avant : la messagerie continue, et
    /// la nouvelle version se demande.
    Changed,
}

/// La version d'un texte de conditions : ses douze premiers chiffres
/// d'empreinte. Un mot changé change la version, et l'acceptation
/// enregistrée dit laquelle on a lue.
///
/// **Les fins de ligne ne comptent pas** : Git pour Windows rend le fichier
/// en CRLF, et la version acceptée sur un poste Windows se disait
/// « changée » sur un poste Linux.
pub fn terms_version(text: &str) -> String {
    let text = text.replace("\r\n", "\n");
    blake3::hash(text.trim().as_bytes()).to_hex()[..12].to_owned()
}

/// Le nom seul d'un hôte, pour comparer deux passerelles : sans port ni
/// point final. Réglée à la main sur `nom:443` ou `nom.`, la passerelle
/// livrée passait pour une autre, et ses conditions ne se demandaient
/// plus.
fn bare_host(host: &str) -> &str {
    let h = match host.strip_prefix('[') {
        Some(v6) => v6.split(']').next().unwrap_or(""),
        None => host.split(':').next().unwrap_or(""),
    };
    h.trim_end_matches('.')
}

/// Où en est l'officine. `gateway` et `built_in` sont des noms d'hôte
/// ([`gateway_host`]) : une passerelle réglée à la main sur le même nom
/// que la livrée est la livrée.
pub fn terms_state(gateway: &str, built_in: &str, terms: &str, accepted: Option<&str>) -> Terms {
    if terms.trim().is_empty() || built_in.is_empty() || bare_host(gateway) != bare_host(built_in) {
        return Terms::None;
    }
    match accepted {
        None => Terms::ToAccept,
        Some(v) if v == terms_version(terms) => Terms::Accepted,
        Some(_) => Terms::Changed,
    }
}

/// La preuve d'une acceptation, telle qu'elle est rangée dans les
/// réglages de l'officine (et voyage avec eux entre les postes).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Acceptance {
    pub version: String,
    /// ISO.
    pub jour: String,
    /// HH:MM, heure locale du poste.
    pub heure: String,
    pub poste: String,
    pub par: String,
}

impl Acceptance {
    pub fn read(text: &str) -> Option<Self> {
        serde_json::from_str(text).ok()
    }

    pub fn write(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

/// Un message du patient, tel que la page l'envoie.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FromPatient {
    /// Tiré par la page : un message renvoyé après une coupure n'est
    /// rangé qu'une fois.
    pub id: String,
    /// L'heure du téléphone, montrée pour ce qu'elle est.
    pub at: String,
    pub text: String,
}

impl FromPatient {
    /// Ce qu'un poste accepte de ranger : un numéro court, un texte non
    /// vide et pas trop long.
    pub fn admissible(&self) -> bool {
        (1..=64).contains(&self.id.len())
            && self
                .id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
            && self.at.chars().count() <= 32
            && !self.text.trim().is_empty()
            && self.text.chars().count() <= TEXT_MAX
    }
}

/// Un message de l'officine, tel que la page le reçoit.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToPatient {
    pub id: i64,
    pub at: String,
    pub text: String,
}

/// Ce que la page envoie, scellé.
#[derive(Debug, Serialize, Deserialize)]
pub struct Request {
    #[serde(default)]
    pub out: Vec<FromPatient>,
    /// Les messages de l'officine que la page a déjà.
    #[serde(default)]
    pub seen: Vec<i64>,
}

/// Ce que le poste rend, scellé.
#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    /// Les messages rangés (ou qui l'étaient déjà).
    pub got: Vec<String>,
    /// Les messages de l'officine que la page n'a pas.
    #[serde(rename = "in")]
    pub messages: Vec<ToPatient>,
    /// Le nom de l'officine, pour la page.
    pub officine: String,
    pub max: usize,
    /// Qui elle est, pour l'information que la page doit au patient
    /// (RGPD, art. 13) : le responsable du traitement, c'est elle.
    #[serde(default)]
    pub adresse: String,
    #[serde(default)]
    pub telephone: String,
    /// Ses heures d'ouverture, en clair : la messagerie n'est lue
    /// qu'alors.
    #[serde(default)]
    pub horaires: String,
    /// Les adresses de ses postes sur la voie directe, à jour : la page
    /// les garde à la place de celles du lien — une officine dont
    /// l'adresse change reste joignable de qui lui a écrit depuis.
    #[serde(default)]
    pub adresses: Vec<String>,
    /// Son tableau du jour (`tableau.rs`).
    #[serde(default)]
    pub tableau: Vec<crate::tableau::Ligne>,
}

/// L'officine, telle que la page la présente au patient.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Officine {
    pub nom: String,
    pub adresse: String,
    pub telephone: String,
    pub horaires: String,
    /// Les adresses de ses postes sur la voie directe.
    pub adresses: Vec<String>,
}

/// Une clé de patient de la base.
#[derive(Clone, Debug)]
pub struct KeyRow {
    pub id: i64,
    pub key: Key,
    /// Fermée par l'officine : la page l'apprend, rien ne passe.
    pub open: bool,
}

/// Ce dont une conversation a besoin de la base.
pub trait Store {
    /// Les clés de l'officine, ouvertes et fermées.
    fn keys(&mut self) -> Result<Vec<KeyRow>, String>;
    /// Ranger ce que le patient envoie ; rendre les numéros rangés (ou
    /// déjà là).
    fn receive(&mut self, key: i64, messages: &[FromPatient]) -> Result<Vec<String>, String>;
    /// Ce que l'officine a écrit sous cette clé, hors `seen`, dans l'ordre
    /// où elle l'a écrit.
    fn replies(&mut self, key: i64, seen: &[i64]) -> Result<Vec<ToPatient>, String>;
    /// Qui est l'officine.
    fn officine(&mut self) -> Officine;
    /// Son tableau du jour.
    fn board(&mut self) -> crate::tableau::Public;
}

/// Une connexion qui passe des trames.
pub trait Frames {
    fn send(&mut self, frame: &[u8]) -> std::io::Result<()>;
    fn recv(&mut self) -> std::io::Result<Vec<u8>>;
}

/// Des trames sur un flux : quatre octets de longueur, puis la trame.
impl<T: std::io::Read + std::io::Write> Frames for T {
    fn send(&mut self, frame: &[u8]) -> std::io::Result<()> {
        let len = u32::try_from(frame.len())
            .ok()
            .filter(|n| (*n as usize) <= FRAME_MAX)
            .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::InvalidInput))?;
        self.write_all(&len.to_be_bytes())?;
        self.write_all(frame)?;
        self.flush()
    }

    fn recv(&mut self) -> std::io::Result<Vec<u8>> {
        let mut len = [0u8; 4];
        self.read_exact(&mut len)?;
        let len = u32::from_be_bytes(len) as usize;
        if len > FRAME_MAX {
            return Err(std::io::Error::from(std::io::ErrorKind::InvalidData));
        }
        let mut out = vec![0u8; len];
        self.read_exact(&mut out)?;
        Ok(out)
    }
}

/// Comment une conversation a fini.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Le patient a parlé ; `received` messages nouveaux ou renvoyés.
    Talked { key: i64, received: usize },
    /// Une clé fermée.
    Closed,
    /// Une clé que l'officine ne connaît pas.
    Unknown,
    /// Le tableau, demandé et rendu.
    Board,
}

fn bad() -> std::io::Error {
    std::io::Error::from(std::io::ErrorKind::InvalidData)
}

/// **Répondre à une page**, du côté du poste.
pub fn answer(
    link: &mut impl Frames,
    store: &mut impl Store,
    fill: impl FnOnce(&mut [u8]),
) -> std::io::Result<Outcome> {
    let hello = link.recv()?;
    answer_to(hello, link, store, fill)
}

/// [`answer`], le premier mot de la page déjà lu (`hello`) — par qui a
/// dû le lire pour savoir où envoyer la page.
pub fn answer_to(
    hello: Vec<u8>,
    link: &mut impl Frames,
    store: &mut impl Store,
    fill: impl FnOnce(&mut [u8]),
) -> std::io::Result<Outcome> {
    if hello == BOARD {
        let board = serde_json::to_vec(&store.board()).map_err(|_| bad())?;
        link.send(&board)?;
        return Ok(Outcome::Board);
    }
    if hello.len() != MAGIC.len() + NONCE + TAG || !hello.starts_with(MAGIC) {
        return Err(bad());
    }
    let nc: [u8; NONCE] = hello[MAGIC.len()..MAGIC.len() + NONCE]
        .try_into()
        .map_err(|_| bad())?;
    let tag = &hello[MAGIC.len() + NONCE..];
    let keys = store.keys().map_err(std::io::Error::other)?;
    // Every key is tried, whatever matched: the time taken does not say
    // where in the list a patient sits.
    let mut found: Option<&KeyRow> = None;
    for k in &keys {
        if verify(&auth_key(&k.key), &[b"client", &nc], tag) && found.is_none() {
            found = Some(k);
        }
    }
    let Some(row) = found else {
        link.send(UNKNOWN)?;
        return Ok(Outcome::Unknown);
    };
    if !row.open {
        let mut word = CLOSED.to_vec();
        word.extend_from_slice(&closed_tag(&row.key, &nc));
        link.send(&word)?;
        return Ok(Outcome::Closed);
    }
    let mut ns = [0u8; NONCE];
    fill(&mut ns);
    let mut reply = Vec::with_capacity(NONCE + TAG);
    reply.extend_from_slice(&ns);
    reply.extend_from_slice(&server_tag(&row.key, &nc, &ns));
    link.send(&reply)?;
    let (mut inbound, mut outbound) = session(&row.key, &nc, &ns);
    let request = inbound.open(&link.recv()?).ok_or_else(bad)?;
    let request: Request = serde_json::from_slice(&request).map_err(|_| bad())?;
    let out: Vec<FromPatient> = request
        .out
        .into_iter()
        .take(OUT_MAX)
        .filter(FromPatient::admissible)
        .collect();
    let got = if out.is_empty() {
        Vec::new()
    } else {
        store.receive(row.id, &out).map_err(std::io::Error::other)?
    };
    let seen = &request.seen[request.seen.len().saturating_sub(SEEN_MAX)..];
    let mut messages = store.replies(row.id, seen).map_err(std::io::Error::other)?;
    if messages.len() > IN_MAX {
        messages.drain(..messages.len() - IN_MAX);
    }
    let who = store.officine();
    let tableau = store.board().lignes;
    let response = Response {
        got,
        messages,
        officine: who.nom,
        max: TEXT_MAX,
        adresse: who.adresse,
        telephone: who.telephone,
        horaires: who.horaires,
        adresses: who.adresses,
        tableau,
    };
    let plain = serde_json::to_vec(&response).map_err(|_| bad())?;
    link.send(&outbound.seal(&plain))?;
    Ok(Outcome::Talked {
        key: row.id,
        received: out.len(),
    })
}

/// **Le côté de la page**, en Rust : ce que `assets/patient/app.js` fait
/// avec WebCrypto. Pour les tests — de ce module, de la passerelle et du
/// poste qui attend.
#[cfg(test)]
pub(crate) fn talk_as_patient(
    link: &mut impl Frames,
    key: &Key,
    nc: [u8; NONCE],
    request: &Request,
) -> std::io::Result<Result<Response, Vec<u8>>> {
    let mut hello = MAGIC.to_vec();
    hello.extend_from_slice(&nc);
    hello.extend_from_slice(&client_tag(key, &nc));
    link.send(&hello)?;
    let back = link.recv()?;
    if back.len() == CLOSED.len() + TAG && back.starts_with(CLOSED) {
        return if verify(&auth_key(key), &[b"ferme", &nc], &back[CLOSED.len()..]) {
            Ok(Err(CLOSED.to_vec()))
        } else {
            Err(bad())
        };
    }
    if back.len() != NONCE + TAG {
        return Ok(Err(back));
    }
    let ns: [u8; NONCE] = back[..NONCE].try_into().map_err(|_| bad())?;
    if !verify(&auth_key(key), &[b"server", &nc, &ns], &back[NONCE..]) {
        return Err(bad());
    }
    let (mut outbound, mut inbound) = session(key, &nc, &ns);
    link.send(&outbound.seal(&serde_json::to_vec(request).map_err(|_| bad())?))?;
    let sealed = link.recv()?;
    let plain = inbound.open(&sealed).ok_or_else(bad)?;
    Ok(Ok(serde_json::from_slice(&plain).map_err(|_| bad())?))
}

/// Une officine en mémoire, pour les tests de ce module, de la
/// passerelle et du poste qui attend.
#[cfg(test)]
pub(crate) mod memory {
    use super::*;

    /// An in-memory officine.
    #[derive(Default)]
    pub(crate) struct Memory {
        pub keys: Vec<KeyRow>,
        pub received: Vec<(i64, FromPatient)>,
        pub replies: Vec<(i64, ToPatient)>,
    }

    impl Store for Memory {
        fn keys(&mut self) -> Result<Vec<KeyRow>, String> {
            Ok(self.keys.clone())
        }
        fn receive(&mut self, key: i64, messages: &[FromPatient]) -> Result<Vec<String>, String> {
            for m in messages {
                if !self.received.iter().any(|(k, r)| *k == key && r.id == m.id) {
                    self.received.push((key, m.clone()));
                }
            }
            Ok(messages.iter().map(|m| m.id.clone()).collect())
        }
        fn replies(&mut self, key: i64, seen: &[i64]) -> Result<Vec<ToPatient>, String> {
            Ok(self
                .replies
                .iter()
                .filter(|(k, r)| *k == key && !seen.contains(&r.id))
                .map(|(_, r)| r.clone())
                .collect())
        }
        fn officine(&mut self) -> Officine {
            Officine {
                nom: "Pharmacie du Port".into(),
                adresse: "1 quai des Pêcheurs, 29900 Concarneau".into(),
                telephone: "02 98 00 00 00".into(),
                horaires: "lundi 09:00-19:00".into(),
                adresses: vec!["203.0.113.7:7748".into()],
            }
        }
        fn board(&mut self) -> crate::tableau::Public {
            crate::tableau::Public {
                v: crate::tableau::PUBLIC_VERSION,
                officine: "Pharmacie du Port".into(),
                le: "2026-10-16".into(),
                lignes: vec![crate::tableau::Ligne {
                    canal: crate::tableau::Canal::Garde,
                    titre: "Pharmacie du Port, ce soir".into(),
                    ..Default::default()
                }],
                ..Default::default()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::memory::Memory;
    use super::*;

    /// Two ends of an in-memory link, each on its own thread.
    fn pair() -> (
        std::os::unix::net::UnixStream,
        std::os::unix::net::UnixStream,
    ) {
        std::os::unix::net::UnixStream::pair().unwrap()
    }

    fn msg(id: &str, text: &str) -> FromPatient {
        FromPatient {
            id: id.into(),
            at: "2026-10-14T10:00".into(),
            text: text.into(),
        }
    }

    fn officine_with(key: Key, open: bool) -> Memory {
        Memory {
            keys: vec![
                KeyRow {
                    id: 1,
                    key: [9; 32],
                    open: true,
                },
                KeyRow { id: 2, key, open },
            ],
            ..Default::default()
        }
    }

    fn run(
        mut store: Memory,
        key: Key,
        request: Request,
    ) -> (Result<Response, Vec<u8>>, Outcome, Memory) {
        let (mut a, mut b) = pair();
        let post = std::thread::spawn(move || {
            let o = answer(&mut b, &mut store, |n| n.fill(7)).unwrap();
            (o, store)
        });
        let r = talk_as_patient(&mut a, &key, [3; 16], &request).unwrap();
        let (o, store) = post.join().unwrap();
        (r, o, store)
    }

    #[test]
    fn a_patient_writes_and_reads_the_officine_back() {
        let key = [5u8; 32];
        let mut store = officine_with(key, true);
        store.replies = vec![
            (
                2,
                ToPatient {
                    id: 10,
                    at: "13/10/2026 18:00".into(),
                    text: "Votre commande est arrivée.".into(),
                },
            ),
            (
                1,
                ToPatient {
                    id: 11,
                    at: "x".into(),
                    text: "pas pour vous".into(),
                },
            ),
        ];
        let (r, o, store) = run(
            store,
            key,
            Request {
                out: vec![msg("a1", "Bonjour, est-ce prêt ?"), msg("a2", "  ")],
                seen: vec![],
            },
        );
        let r = r.unwrap();
        assert_eq!(r.got, vec!["a1".to_string()], "the blank one is not kept");
        assert_eq!(r.messages.len(), 1, "only this patient's");
        assert_eq!(r.messages[0].text, "Votre commande est arrivée.");
        assert_eq!(r.officine, "Pharmacie du Port");
        assert_eq!(r.telephone, "02 98 00 00 00");
        assert_eq!(r.horaires, "lundi 09:00-19:00");
        // The addresses kept up to date, and the board of the day.
        assert_eq!(r.adresses, vec!["203.0.113.7:7748".to_string()]);
        assert_eq!(r.tableau.len(), 1);
        assert_eq!(
            o,
            Outcome::Talked {
                key: 2,
                received: 1
            }
        );
        assert_eq!(
            store.received,
            vec![(2, msg("a1", "Bonjour, est-ce prêt ?"))]
        );
    }

    #[test]
    fn a_closed_key_or_a_stranger_gets_a_word_and_nothing_else() {
        let key = [5u8; 32];
        let (r, o, store) = run(
            officine_with(key, false),
            key,
            Request {
                out: vec![msg("a", "x")],
                seen: vec![],
            },
        );
        assert_eq!(r.unwrap_err(), CLOSED.to_vec());
        assert_eq!(o, Outcome::Closed);
        assert!(store.received.is_empty());
        let (r, o, _) = run(
            officine_with(key, true),
            [6; 32],
            Request {
                out: vec![],
                seen: vec![],
            },
        );
        assert_eq!(r.unwrap_err(), UNKNOWN.to_vec());
        assert_eq!(o, Outcome::Unknown);
    }

    /// The board needs no key: asked, it comes back in the clear, and
    /// nothing else happens.
    #[test]
    fn the_board_is_given_to_whoever_asks() {
        let (mut a, mut b) = pair();
        let mut store = officine_with([5; 32], true);
        let post = std::thread::spawn(move || answer(&mut b, &mut store, |n| n.fill(7)).unwrap());
        a.send(BOARD).unwrap();
        let board: crate::tableau::Public = serde_json::from_slice(&a.recv().unwrap()).unwrap();
        assert_eq!(post.join().unwrap(), Outcome::Board);
        assert_eq!(board.officine, "Pharmacie du Port");
        assert_eq!(board.lignes[0].canal, crate::tableau::Canal::Garde);
    }

    /// A post that does not hold the key cannot pass for the officine:
    /// the page checks the post's proof.
    #[test]
    fn the_page_refuses_a_post_without_the_key() {
        let (mut a, mut b) = pair();
        let fake = std::thread::spawn(move || {
            let _ = b.recv().unwrap();
            let mut back = vec![1u8; NONCE];
            back.extend_from_slice(&[0u8; TAG]);
            b.send(&back).unwrap();
        });
        let r = talk_as_patient(
            &mut a,
            &[5; 32],
            [3; 16],
            &Request {
                out: vec![],
                seen: vec![],
            },
        );
        assert!(r.is_err());
        fake.join().unwrap();
    }

    /// The same request sealed in one conversation does not open in
    /// another: each post's number makes its keys.
    #[test]
    fn a_recorded_request_does_not_replay() {
        let key = [5u8; 32];
        let (nc, ns1, ns2) = ([1u8; 16], [2u8; 16], [3u8; 16]);
        let (mut out1, _) = session(&key, &nc, &ns1);
        let sealed = out1.seal(b"{\"out\":[]}");
        let (mut in2, _) = session(&key, &nc, &ns2);
        assert!(in2.open(&sealed).is_none());
        let (mut in1, _) = session(&key, &nc, &ns1);
        assert_eq!(in1.open(&sealed).unwrap(), b"{\"out\":[]}");
    }

    #[test]
    fn what_a_post_accepts_to_keep() {
        assert!(msg("a-1", "ok").admissible());
        assert!(!msg("", "ok").admissible());
        assert!(!msg("a b", "ok").admissible());
        assert!(!msg(&"a".repeat(65), "ok").admissible());
        assert!(!msg("a", " \n").admissible());
        assert!(msg("a", &"é".repeat(TEXT_MAX)).admissible());
        assert!(!msg("a", &"é".repeat(TEXT_MAX + 1)).admissible());
    }

    /// The fixed answers the page's WebCrypto must give for the same
    /// inputs: `assets/patient/app.js` carries this vector and checks the
    /// browser against it before it says a word — a browser that seals
    /// otherwise is told so, instead of failing every conversation.
    #[test]
    fn the_known_answers_the_page_checks_against() {
        let key = [0x11u8; 32];
        let (nc, ns) = ([0x22u8; 16], [0x33u8; 16]);
        assert_eq!(hex(&client_tag(&key, &nc)), KAT_CLIENT);
        assert_eq!(hex(&server_tag(&key, &nc, &ns)), KAT_SERVER);
        assert_eq!(hex(&closed_tag(&key, &nc)), KAT_CLOSED);
        let (mut up, _) = session(&key, &nc, &ns);
        assert_eq!(hex(&up.seal(b"bonjour")), KAT_SEALED);
        let app = include_str!("../assets/patient/app.js");
        for kat in [KAT_CLIENT, KAT_SERVER, KAT_CLOSED, KAT_SEALED] {
            assert!(app.contains(kat), "app.js does not carry {kat}");
        }
    }

    const KAT_CLIENT: &str = "e2ae25f709a44e263610be14252b440e71ddcfbcfdfbea76aecaf8be5688d788";
    const KAT_SERVER: &str = "c4441c7f787363f4e263bee9b3043830e6328e4f032671d2a3e27e199e330167";
    const KAT_CLOSED: &str = "4620447ef9ec84d374c61162b26bf1ee13dff689608b6eed5ee639fa21cf006c";
    const KAT_SEALED: &str = "7569bf2885d9a3b1d898da2a820ca83b350bbdbf2c7d9f";

    #[test]
    fn keys_links_and_boxes_read_back() {
        let key = new_key(|k| k.fill(0xab));
        assert_eq!(key_text(&key).len(), 43);
        assert_eq!(key_from_text(&key_text(&key)), Some(key));
        assert_eq!(key_from_text("court"), None);
        let id = box_id(&[1; 32]);
        assert!(is_box_id(&id));
        assert_ne!(id, box_id(&[2; 32]));
        let gateway = Reach {
            gateway: "https://Chat.Example.fr/".into(),
            ..Default::default()
        };
        assert_eq!(
            link(&gateway, &id, &key),
            format!(
                "{PAGE_SITE}{}/#b={id}&k={}&p=chat.example.fr",
                page_hash(),
                key_text(&key)
            )
        );
        // The direct way first, then the gateway; addresses capped.
        let both = Reach {
            fingerprint: "F".repeat(43),
            addresses: (0..9).map(|i| format!("203.0.113.{i}:7748")).collect(),
            gateway: "chat.example.fr".into(),
        };
        let l = link(&both, &id, &key);
        assert!(
            l.contains(&format!("&f={}&a=203.0.113.0:7748,", "F".repeat(43))),
            "{l}"
        );
        assert!(l.ends_with("203.0.113.5:7748&p=chat.example.fr"), "{l}");
        // A fingerprint without an address is no way at all.
        let none = Reach {
            fingerprint: "F".repeat(43),
            ..Default::default()
        };
        assert!(!none.any());
        assert!(both.any() && gateway.any());
        // The board's code carries no key.
        let b = board_link(&both, &id, true);
        assert!(
            b.contains(&format!("#b={id}&t=1&e=1&f=")) && !b.contains("&k="),
            "{b}"
        );
        assert!(!b.contains("&p="), "never through a gateway: {b}");
        assert_eq!(
            gateway_host(" chat.example.fr:8443 "),
            "chat.example.fr:8443"
        );
        assert_eq!(gateway_host("chat example"), "");
        assert_eq!(gateway_host("<script>"), "");
    }

    #[test]
    fn a_box_address_typed_by_hand_reads_as_the_page_dials_it() {
        assert_eq!(
            box_address(" 203.0.113.7 ").as_deref(),
            Some("203.0.113.7:7748")
        );
        assert_eq!(
            box_address("203.0.113.7:9000").as_deref(),
            Some("203.0.113.7:9000")
        );
        assert_eq!(
            box_address("2001:db8::1").as_deref(),
            Some("[2001:db8::1]:7748")
        );
        assert_eq!(
            box_address("[2001:db8::1]:9000").as_deref(),
            Some("[2001:db8::1]:9000")
        );
        assert_eq!(box_address("box.exemple.fr"), None);
        assert_eq!(box_address(""), None);
    }

    /// The page is published by `.github/workflows/page.yml` under the
    /// hash this module computes: the two recipes are one, or every code
    /// handed out points at a folder that does not exist.
    #[test]
    fn the_page_is_published_where_the_link_points() {
        let flow = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/page.yml"),
        )
        .unwrap();
        let names: Vec<&str> = PAGE_FILES.iter().map(|(n, _)| *n).collect();
        assert!(
            flow.contains(&format!("cat {} | sha256sum | cut -c1-16", names.join(" "))),
            "page.yml hashes the page as page_hash does"
        );
        assert!(
            flow.contains(PAGE_MARK),
            "page.yml publishes only pages that call p="
        );
        assert!(PAGE_FILES[1].1.contains(PAGE_MARK));
        // Hashed as GitHub serves them: the bytes of the repository, never
        // a checkout's CRLF (`.gitattributes`).
        for (name, body) in PAGE_FILES {
            assert!(!body.contains('\r'), "{name} holds a carriage return");
        }
        let attributes = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".gitattributes"),
        )
        .unwrap();
        assert!(attributes
            .lines()
            .any(|l| l.trim() == "assets/patient/** -text"));
        assert!(PAGE_SITE.ends_with('/'));
        let h = page_hash();
        assert_eq!(h.len(), 16);
        assert_eq!(
            hash_of_page([b"a".as_slice(), b"b".as_slice(), b"c".as_slice()]),
            "ba7816bf8f01cfea" // sha256("abc")
        );
        assert!(page_url().ends_with(&format!("{h}/")));
    }
}

#[cfg(test)]
mod terms_tests {
    use super::*;

    /// **Les conditions ne visent que la passerelle livrée**, et seulement
    /// quand un texte est livré : une passerelle à soi n'a rien à accepter
    /// ici ; la livrée réglée à la main (même nom) reste la livrée.
    #[test]
    fn the_terms_bind_the_built_in_gateway_only() {
        let text = "# Conditions\nLe prestataire...";
        assert_eq!(terms_state("a.fr", "a.fr", "", None), Terms::None);
        assert_eq!(terms_state("a.fr", "", text, None), Terms::None);
        assert_eq!(terms_state("b.fr", "a.fr", text, None), Terms::None);
        assert_eq!(terms_state("a.fr", "a.fr", text, None), Terms::ToAccept);
        let v = terms_version(text);
        assert_eq!(terms_state("a.fr", "a.fr", text, Some(&v)), Terms::Accepted);
        // Un mot changé : une autre version, qui se redemande.
        assert_eq!(
            terms_state("a.fr", "a.fr", "# Conditions\nLe prestataire, v2", Some(&v)),
            Terms::Changed
        );
        // Les blancs autour ne font pas une version, ni les fins de ligne.
        assert_eq!(terms_version(text), terms_version(&format!("\n{text}\n\n")));
        assert_eq!(
            terms_version(text),
            terms_version(&text.replace('\n', "\r\n"))
        );
        // Le même hôte, avec un port ou un point final, est la livrée.
        for same in ["a.fr:443", "a.fr.", "a.fr.:8443"] {
            assert_eq!(
                terms_state(same, "a.fr", text, None),
                Terms::ToAccept,
                "{same}"
            );
        }
        assert_eq!(
            terms_state("[::1]:443", "[::1]", text, None),
            Terms::ToAccept
        );
    }

    /// La preuve se relit telle qu'elle a été écrite ; un réglage
    /// illisible n'est pas une acceptation.
    #[test]
    fn an_acceptance_reads_back_what_was_written() {
        let a = Acceptance {
            version: "0123456789ab".into(),
            jour: "2026-10-15".into(),
            heure: "14:02".into(),
            poste: "Comptoir 1".into(),
            par: "Claire Leroy".into(),
        };
        assert_eq!(Acceptance::read(&a.write()), Some(a));
        assert_eq!(Acceptance::read("oui"), None);
    }
}
