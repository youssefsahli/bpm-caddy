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

/// **Le lien du code QR** : la page publiée, puis dans le fragment la
/// boîte, la clé et la passerelle (`p`) — un fragment n'est envoyé à
/// personne, pas même à GitHub. `gateway` est un nom d'hôte (et un
/// port), sans schéma : la page ne l'appelle qu'en `wss://`.
pub fn link(gateway: &str, box_id: &str, key: &Key) -> String {
    let host = gateway_host(gateway);
    format!("{}#b={box_id}&k={}&p={host}", page_url(), key_text(key))
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
}

/// L'officine, telle que la page la présente au patient.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Officine {
    pub nom: String,
    pub adresse: String,
    pub telephone: String,
    pub horaires: String,
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
    let response = Response {
        got,
        messages,
        officine: who.nom,
        max: TEXT_MAX,
        adresse: who.adresse,
        telephone: who.telephone,
        horaires: who.horaires,
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
        assert_eq!(
            link("https://Chat.Example.fr/", &id, &key),
            format!(
                "{PAGE_SITE}{}/#b={id}&k={}&p=chat.example.fr",
                page_hash(),
                key_text(&key)
            )
        );
        assert_eq!(
            gateway_host(" chat.example.fr:8443 "),
            "chat.example.fr:8443"
        );
        assert_eq!(gateway_host("chat example"), "");
        assert_eq!(gateway_host("<script>"), "");
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
