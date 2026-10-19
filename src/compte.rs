//! Les comptes : **une personne**, pas un poste ni une officine.
//!
//! Un poste a son identité (`Device`), l'officine sa clé sur chaque
//! réseau ; rien ne disait qui, de l'équipe, écrit. Un compte est la clé
//! d'une personne : elle la garde en douze mots, l'ouvre sur n'importe
//! quel poste, et ce qu'elle y range la suit — son agenda, ses notes.
//!
//! **Trois visibilités, une seule forme d'enregistrement.**
//!
//! * *Public* : le contenu est en clair, signé par le compte. Le profil
//!   (nom, rôle, RPPS, officine, clé de boîte) l'est toujours — c'est ce
//!   qui permet à une autre personne de partager avec celle-ci.
//! * *Partagé* : scellé (`bpm_sync::boxed`) pour les comptes choisis et
//!   pour soi.
//! * *Privé* : scellé pour soi seul. Le même sceau, un destinataire.
//!
//! **Semé par le réseau.** Un enregistrement porte sa propre preuve (la
//! signature du compte sur ses octets) : n'importe quel poste peut le
//! garder et le passer, aucun ne peut le réécrire, et seuls ses
//! destinataires lisent ce qui est scellé. Les officines le transportent
//! dans le journal du réseau (`network.rs`, préfixe [`TAG`]) ; un poste
//! neuf qui reçoit la phrase retrouve ainsi le compte. **Jamais un
//! patient** : ce qui est scellé ici ne nomme personne d'autre que son
//! auteur, et l'écran le dit — une officine qui garderait, même
//! chiffrées, les données de santé d'une autre ferait de l'hébergement
//! (`docs/SYNC.md` § 8).
//!
//! **La dernière version gagne**, élément par élément : un numéro qui
//! monte (`n`), puis le nom de l'enregistrement pour départager deux
//! postes qui ont écrit le même numéro sans s'être vus. Un effacement est
//! une version vide.
//!
//! Pur et testé : aucune base, aucune horloge, l'aléa passé.

use bpm_sync::{boxed, Device, DeviceId, Entropy};
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

/// Ce qui ouvre un enregistrement de compte dans le journal du réseau —
/// les lecteurs JSON le laissent passer, comme les boîtes.
pub const TAG: &[u8] = b"ACC1";

const SIGN_DOMAIN: &str = "bpm-caddy/compte/v1";
const SEED_DOMAIN: &str = "bpm-caddy/compte-graine/v1";

/// Au plus ce que porte un élément, en clair : une note, un rendez-vous,
/// pas un fichier. Court **parce que le journal d'un réseau ne s'oublie
/// pas** : chaque version publiée y reste, et les autres officines ne
/// gardent d'une même officine qu'un mégaoctet (`OPEN_LIMITS`).
pub const MAX_DATA: usize = 4 * 1024;

/// Le plus long corps qu'une boîte de [`MOST_RECIPIENTS`] destinataires
/// et de [`MAX_DATA`] octets donne, en hexadécimal, avec sa marge : ce
/// que [`read`] accepte, et donc tout ce que [`write`] peut rendre.
const MOST_BODY: usize = 2 * (34 + MOST_RECIPIENTS * 72 + 24 + MAX_SETTINGS + 16) + 1024;

/// Au plus ce que portent les réglages d'un compte : des noms de compte
/// de soixante-quatre caractères, de quoi en écarter quelque deux cents
/// — une vague de comptes jetables comprise. Ils changent rarement, et
/// un seul vit à la fois.
pub const MAX_SETTINGS: usize = 16 * 1024;

/// Au plus ce que le journal d'un réseau garde d'une même officine
/// (`OPEN_LIMITS`) est un mégaoctet : un compte ne garde au plus que
/// ce nombre d'éléments vivants, pour en laisser à ses collègues.
pub const MOST_ITEMS: usize = 200;

/// Au plus ce nombre de destinataires, soi compris.
pub const MOST_RECIPIENTS: usize = boxed::MAX_RECIPIENTS;

/// Le plus court des codes qui gardent un compte sur un poste.
pub const PIN_FLOOR: usize = 8;

// ---------------------------------------------------------------------
// La phrase
// ---------------------------------------------------------------------

/// Les douze mots, et les seize octets qu'ils disent.
#[derive(Clone, PartialEq, Eq, Zeroize)]
#[zeroize(drop)]
pub struct Phrase {
    entropy: [u8; 16],
}

impl std::fmt::Debug for Phrase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Phrase(…)")
    }
}

/// Pourquoi une phrase tapée ne se lit pas.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PhraseError {
    /// Pas douze mots.
    Count(usize),
    /// Ce mot (rang à partir de 1, tel que tapé) n'est pas de la liste.
    Unknown(usize, String),
    /// Ce mot commence comme plusieurs mots de la liste.
    Ambiguous(usize, String),
    /// Douze mots de la liste, mais pas une phrase : une faute de frappe
    /// qui est tombée sur un autre mot, ou deux mots échangés.
    Checksum,
}

/// Un mot sans ses accents ni ses majuscules : « Élève », « eleve » et
/// la forme décomposée de la liste se lisent pareil.
pub fn fold(word: &str) -> String {
    let mut out = String::with_capacity(word.len());
    for c in word.chars() {
        if ('\u{300}'..='\u{36f}').contains(&c) {
            continue;
        }
        let c = match c {
            'à' | 'â' | 'ä' | 'À' | 'Â' | 'Ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' | 'Ê' | 'Ë' => 'e',
            'î' | 'ï' | 'Î' | 'Ï' => 'i',
            'ô' | 'ö' | 'Ô' | 'Ö' => 'o',
            'ù' | 'û' | 'ü' | 'Ù' | 'Û' | 'Ü' => 'u',
            'ç' | 'Ç' => 'c',
            other => other,
        };
        out.extend(c.to_lowercase());
    }
    out
}

fn list() -> &'static [&'static str; 2048] {
    bip39::Language::French.word_list()
}

/// Le rang d'un mot tapé dans la liste : le mot entier, sans accent, ou
/// un début d'au moins quatre lettres qui n'en désigne qu'un.
fn word_index(typed: &str) -> Result<usize, bool> {
    let typed = fold(typed);
    let folded = list().iter().map(|w| fold(w));
    let mut starts = Vec::new();
    for (i, w) in folded.enumerate() {
        if w == typed {
            return Ok(i);
        }
        if typed.chars().count() >= 4 && w.starts_with(&typed) {
            starts.push(i);
        }
    }
    match starts.as_slice() {
        [one] => Ok(*one),
        [] => Err(false),
        _ => Err(true),
    }
}

impl Phrase {
    pub fn generate(entropy: &mut dyn Entropy) -> Self {
        let mut bytes = [0u8; 16];
        entropy.fill(&mut bytes);
        Self { entropy: bytes }
    }

    pub fn from_entropy(entropy: [u8; 16]) -> Self {
        Self { entropy }
    }

    pub fn entropy(&self) -> [u8; 16] {
        self.entropy
    }

    /// Les douze mots, avec leurs accents, dans l'ordre.
    pub fn words(&self) -> Vec<String> {
        // Seize octets font toujours une phrase de douze mots.
        bip39::Mnemonic::from_entropy_in(bip39::Language::French, &self.entropy)
            .map(|m| m.words().map(compose).collect())
            .unwrap_or_default()
    }

    /// Une phrase tapée : espaces, tirets, virgules et retours à la
    /// ligne séparent les mots ; les accents et les majuscules ne
    /// comptent pas ; quatre lettres suffisent quand elles ne désignent
    /// qu'un mot.
    pub fn parse(text: &str) -> Result<Self, PhraseError> {
        let typed: Vec<&str> = text
            .split(|c: char| c.is_whitespace() || c == ',' || c == ';' || c == '-')
            .filter(|w| !w.is_empty())
            // « 1. abeille » : les numéros recopiés avec la phrase.
            .filter(|w| {
                !w.trim_end_matches(['.', ')'])
                    .chars()
                    .all(|c| c.is_ascii_digit())
            })
            .collect();
        if typed.len() != 12 {
            return Err(PhraseError::Count(typed.len()));
        }
        let mut words = Vec::with_capacity(12);
        for (n, w) in typed.iter().enumerate() {
            match word_index(w) {
                Ok(i) => words.push(list()[i]),
                Err(true) => return Err(PhraseError::Ambiguous(n + 1, (*w).to_owned())),
                Err(false) => return Err(PhraseError::Unknown(n + 1, (*w).to_owned())),
            }
        }
        let m = bip39::Mnemonic::parse_in_normalized(bip39::Language::French, &words.join(" "))
            .map_err(|_| PhraseError::Checksum)?;
        let mut entropy = [0u8; 16];
        let bytes = m.to_entropy();
        if bytes.len() != 16 {
            return Err(PhraseError::Checksum);
        }
        entropy.copy_from_slice(&bytes);
        Ok(Self { entropy })
    }
}

/// La liste est écrite décomposée (un « e » suivi de l'accent, aigu ou
/// grave : ses deux seuls accents) ; composée, elle se tape au clavier
/// et se compare à ce qu'on a recopié.
fn compose(word: &str) -> String {
    let mut out = String::with_capacity(word.len());
    for c in word.chars() {
        let composed = match (out.chars().last(), c) {
            (Some('e'), '\u{301}') => Some('é'),
            (Some('e'), '\u{300}') => Some('è'),
            _ => None,
        };
        match composed {
            Some(e) => {
                out.pop();
                out.push(e);
            }
            None => out.push(c),
        }
    }
    out
}

// ---------------------------------------------------------------------
// Le compte
// ---------------------------------------------------------------------

/// Un compte ouvert : la phrase, et les deux clés qu'on en tire.
pub struct Account {
    phrase: Phrase,
    keys: Device,
}

impl Account {
    pub fn from_phrase(phrase: Phrase) -> Self {
        let mut seed = blake3::derive_key(SEED_DOMAIN, &phrase.entropy);
        let keys = Device::from_seed(seed);
        seed.zeroize();
        Self { phrase, keys }
    }

    pub fn phrase(&self) -> &Phrase {
        &self.phrase
    }

    /// Le nom public du compte : sa clé de signature, en hexadécimal.
    pub fn id(&self) -> String {
        hex(&self.keys.id().0)
    }

    /// Ce qui se lit à voix haute pour reconnaître un compte.
    pub fn groups(&self) -> String {
        self.keys.id().fingerprint().groups()
    }

    pub fn box_public(&self) -> [u8; 32] {
        self.keys.box_public()
    }

    fn box_secret(&self) -> [u8; 32] {
        self.keys.box_secret()
    }
}

/// L'empreinte lisible d'un compte, d'après son nom.
pub fn groups_of(account: &str) -> String {
    unhex::<32>(account)
        .map(|k| DeviceId::from_bytes(k).fingerprint().groups())
        .unwrap_or_default()
}

// ---------------------------------------------------------------------
// Ce qu'un compte écrit
// ---------------------------------------------------------------------

/// Ce qu'un élément est.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Le profil : un par compte, toujours public.
    Profil,
    /// Une note libre.
    Note,
    /// Un rendez-vous d'agenda.
    Rdv,
    /// Un billet : ce qu'on dit aux autres, lu au fil des actualités.
    Billet,
    /// Les réglages du compte — qui il a masqué, qui il a bloqué : un
    /// par compte, toujours scellé pour lui seul. Ils le suivent d'un
    /// poste à l'autre comme le reste.
    Reglages,
}

impl Kind {
    pub fn key(self) -> &'static str {
        match self {
            Kind::Profil => "profil",
            Kind::Note => "note",
            Kind::Rdv => "rdv",
            Kind::Billet => "billet",
            Kind::Reglages => "reglages",
        }
    }

    pub fn from_key(key: &str) -> Option<Kind> {
        match key {
            "profil" => Some(Kind::Profil),
            "note" => Some(Kind::Note),
            "rdv" => Some(Kind::Rdv),
            "billet" => Some(Kind::Billet),
            "reglages" => Some(Kind::Reglages),
            _ => None,
        }
    }
}

/// L'élément « profil » : un seul par compte.
pub const PROFILE_ITEM: &str = "profil";

/// L'élément « réglages » : un seul par compte.
pub const SETTINGS_ITEM: &str = "reglages";

/// Ce qu'un compte a décidé des autres : ceux dont il ne lit plus les
/// billets (masqués), et ceux dont il ne lit plus rien (bloqués).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub masques: Vec<String>,
    #[serde(default)]
    pub bloques: Vec<String>,
}

/// Ce qu'un compte fait de ce qu'un autre écrit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hold {
    /// Lu comme tout.
    Read,
    /// Ses billets ne paraissent plus au fil ; ce qu'il partage reste.
    Muted,
    /// Plus rien de lui : ni billets, ni partages, ni publics, et il
    /// n'est plus proposé comme lecteur.
    Blocked,
}

impl Settings {
    pub fn hold(&self, account: &str) -> Hold {
        if self.bloques.iter().any(|a| a == account) {
            Hold::Blocked
        } else if self.masques.iter().any(|a| a == account) {
            Hold::Muted
        } else {
            Hold::Read
        }
    }

    /// Ranger un compte sous une décision — une seule à la fois.
    pub fn set(&mut self, account: &str, hold: Hold) {
        self.masques.retain(|a| a != account);
        self.bloques.retain(|a| a != account);
        match hold {
            Hold::Read => {}
            Hold::Muted => self.masques.push(account.to_owned()),
            Hold::Blocked => self.bloques.push(account.to_owned()),
        }
        self.masques.sort();
        self.bloques.sort();
    }
}

/// Le profil, public.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Profile {
    #[serde(default)]
    pub nom: String,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub rpps: String,
    #[serde(default)]
    pub officine: String,
    /// La clé de boîte, en hexadécimal : ce qui permet de partager avec
    /// ce compte.
    #[serde(default)]
    pub boite: String,
}

/// Une note ou un rendez-vous.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    #[serde(default)]
    pub titre: String,
    #[serde(default)]
    pub texte: String,
    /// ISO, pour un rendez-vous.
    #[serde(default)]
    pub jour: String,
    /// `HH:MM`, vide quand l'heure n'est pas fixée.
    #[serde(default)]
    pub debut: String,
    #[serde(default)]
    pub fin: String,
    /// Avec qui l'élément est partagé (noms de compte) — dans le sceau,
    /// pour que l'auteur le relise sans que le journal le dise.
    #[serde(default)]
    pub pour: Vec<String>,
}

/// Qui lit un élément.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Visibility {
    Private,
    /// Les comptes nommés, et soi.
    Shared(Vec<String>),
    Public,
}

#[derive(Serialize, Deserialize)]
struct Body {
    /// Le compte.
    a: String,
    /// L'élément.
    i: String,
    /// La version.
    n: u64,
    /// Sa sorte.
    k: String,
    /// `public`, `scelle` ou `efface`.
    v: String,
    /// Le contenu : en clair, ou la boîte en hexadécimal.
    #[serde(default)]
    d: String,
}

/// Ce qu'on écrit.
pub enum Content<'a> {
    Public(&'a str),
    /// Pour soi et ces clés de boîte.
    Sealed(&'a str, &'a [[u8; 32]]),
    Deleted,
}

/// Pourquoi un élément ne s'écrit pas.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WriteError {
    TooLong,
    TooManyRecipients,
    Seal,
}

/// Un enregistrement signé : `TAG ‖ compte (32) ‖ signature (64) ‖ corps`.
/// La signature porte sur le hachage du corps sous son domaine.
pub fn write(
    account: &Account,
    item: &str,
    seq: u64,
    kind: Kind,
    content: Content<'_>,
    entropy: &mut dyn Entropy,
) -> Result<Vec<u8>, WriteError> {
    let most = if kind == Kind::Reglages {
        MAX_SETTINGS
    } else {
        MAX_DATA
    };
    let (v, d) = match content {
        Content::Public(text) => {
            if text.len() > most {
                return Err(WriteError::TooLong);
            }
            ("public", text.to_owned())
        }
        Content::Sealed(text, others) => {
            if text.len() > most {
                return Err(WriteError::TooLong);
            }
            let mut to = vec![account.box_public()];
            for k in others {
                if !to.contains(k) {
                    to.push(*k);
                }
            }
            if to.len() > MOST_RECIPIENTS {
                return Err(WriteError::TooManyRecipients);
            }
            let sealed =
                boxed::seal(&to, text.as_bytes(), entropy).map_err(|_| WriteError::Seal)?;
            ("scelle", hex(&sealed))
        }
        Content::Deleted => ("efface", String::new()),
    };
    let body = Body {
        a: account.id(),
        i: item.to_owned(),
        n: seq,
        k: kind.key().to_owned(),
        v: v.to_owned(),
        d,
    };
    let body = serde_json::to_vec(&body).map_err(|_| WriteError::Seal)?;
    let signature = account.keys.sign(&signed(&body));
    let mut out = Vec::with_capacity(TAG.len() + 96 + body.len());
    out.extend_from_slice(TAG);
    out.extend_from_slice(&account.keys.id().0);
    out.extend_from_slice(&signature);
    out.extend_from_slice(&body);
    Ok(out)
}

fn signed(body: &[u8]) -> [u8; 32] {
    let mut h = blake3::Hasher::new_derive_key(SIGN_DOMAIN);
    h.update(body);
    *h.finalize().as_bytes()
}

/// Un enregistrement lu, sa signature vérifiée.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Read {
    /// Son nom : le hachage de ses octets.
    pub name: String,
    pub account: String,
    pub item: String,
    pub seq: u64,
    pub kind: Kind,
    pub state: State,
}

/// Ce qu'un enregistrement dit de son élément.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum State {
    Public(String),
    /// Scellé : `Some` quand la clé de ce compte-ci l'ouvre.
    Sealed(Option<String>),
    Deleted,
}

/// Le nom d'un enregistrement.
pub fn name_of(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

/// Lire un enregistrement — `None` s'il n'est pas d'un compte, s'il est
/// mal formé, trop long, ou si la signature ne tient pas. `reader` est
/// le compte ouvert ici, qui essaie d'ouvrir ce qui est scellé.
pub fn read(bytes: &[u8], reader: Option<&Account>) -> Option<Read> {
    let rest = bytes.strip_prefix(TAG)?;
    if rest.len() < 96 || rest.len() > 96 + MOST_BODY {
        return None;
    }
    let key: [u8; 32] = rest[..32].try_into().ok()?;
    let signature: [u8; 64] = rest[32..96].try_into().ok()?;
    let body = &rest[96..];
    DeviceId::from_bytes(key)
        .verify(&signed(body), &signature)
        .ok()?;
    let body: Body = serde_json::from_slice(body).ok()?;
    if body.a != hex(&key) || body.i.is_empty() || body.i.len() > 64 {
        return None;
    }
    let kind = Kind::from_key(&body.k)?;
    // Le profil est public et n'est que le profil ; les réglages sont
    // scellés et ne sont que les réglages.
    if (kind == Kind::Profil) != (body.i == PROFILE_ITEM)
        || (kind == Kind::Reglages) != (body.i == SETTINGS_ITEM)
    {
        return None;
    }
    let state = match body.v.as_str() {
        "public" if kind == Kind::Reglages => return None,
        "public" => State::Public(body.d),
        "efface" => State::Deleted,
        "scelle" if kind != Kind::Profil => {
            let opened = reader.and_then(|r| {
                let sealed = unhex_vec(&body.d)?;
                let mut secret = r.box_secret();
                let plain = boxed::open(&secret, &sealed).ok();
                secret.zeroize();
                String::from_utf8(plain?).ok()
            });
            State::Sealed(opened)
        }
        _ => return None,
    };
    Some(Read {
        name: name_of(bytes),
        account: body.a,
        item: body.i,
        seq: body.n,
        kind,
        state,
    })
}

/// Qui gagne entre deux versions d'un même élément : le plus grand
/// numéro, puis le plus grand nom — le même choix sur chaque poste.
pub fn newer(a: &Read, b: &Read) -> bool {
    (a.seq, &a.name) > (b.seq, &b.name)
}

/// La dernière version de chaque élément, dans l'ordre (compte,
/// élément).
pub fn latest(records: impl IntoIterator<Item = Read>) -> Vec<Read> {
    let mut best: std::collections::BTreeMap<(String, String), Read> = Default::default();
    for r in records {
        let key = (r.account.clone(), r.item.clone());
        match best.get(&key) {
            Some(b) if !newer(&r, b) => {}
            _ => {
                best.insert(key, r);
            }
        }
    }
    best.into_values().collect()
}

/// Ce qu'une officine passe au réseau : de chaque compte, le profil et
/// les réglages, ses [`MOST_ITEMS`] éléments vivants les plus récents,
/// autant de billets et autant d'effacements — les numéros de version suivent l'heure d'écriture
/// (`max(dernier + 1, maintenant)`), si bien que le plus grand est le plus
/// récent. Les effacements comptent à part : ils ne prennent pas la place
/// d'un élément neuf.
pub fn to_seed<B>(latest: Vec<(Read, B)>) -> Vec<(Read, B)> {
    let mut by_account: std::collections::BTreeMap<String, Vec<(Read, B)>> = Default::default();
    for (r, b) in latest {
        by_account
            .entry(r.account.clone())
            .or_default()
            .push((r, b));
    }
    let mut out = Vec::new();
    for (_, mut items) in by_account {
        items.sort_by(|(a, _), (b, _)| {
            (b.kind == Kind::Profil)
                .cmp(&(a.kind == Kind::Profil))
                .then(b.seq.cmp(&a.seq))
                .then(b.name.cmp(&a.name))
        });
        // Le profil et les réglages partent toujours ; les billets se
        // comptent à part des notes et des rendez-vous, pour qu'un fil
        // bavard ne pousse pas l'agenda hors du réseau.
        let (mut live, mut posts, mut gone) = (0, 0, 0);
        out.extend(items.into_iter().filter(|(r, _)| {
            if matches!(r.kind, Kind::Profil | Kind::Reglages) {
                return true;
            }
            let count = if r.state == State::Deleted {
                &mut gone
            } else if r.kind == Kind::Billet {
                &mut posts
            } else {
                &mut live
            };
            *count += 1;
            *count <= MOST_ITEMS
        }));
    }
    out
}

/// Le profil que dit un enregistrement, s'il en est un.
pub fn profile_of(r: &Read) -> Option<Profile> {
    match (&r.kind, &r.state) {
        (Kind::Profil, State::Public(text)) => serde_json::from_str(text).ok(),
        _ => None,
    }
}

/// Le contenu d'une note ou d'un rendez-vous, s'il se lit.
pub fn entry_of(r: &Read) -> Option<Entry> {
    match (&r.kind, &r.state) {
        (
            Kind::Note | Kind::Rdv | Kind::Billet,
            State::Public(text) | State::Sealed(Some(text)),
        ) => serde_json::from_str(text).ok(),
        _ => None,
    }
}

/// Qui lit cet élément, vu de son auteur.
pub fn visibility_of(r: &Read, entry: &Entry) -> Visibility {
    match r.state {
        State::Public(_) => Visibility::Public,
        _ if entry.pour.is_empty() => Visibility::Private,
        _ => Visibility::Shared(entry.pour.clone()),
    }
}

/// Écrire une note ou un rendez-vous sous sa visibilité. `boxes` donne
/// la clé de boîte d'un compte ; un destinataire sans clé connue est
/// refusé plutôt qu'oublié en silence. `at` : l'élément et sa version.
pub fn write_entry(
    account: &Account,
    (item, seq): (&str, u64),
    kind: Kind,
    entry: &Entry,
    visibility: &Visibility,
    boxes: &dyn Fn(&str) -> Option<[u8; 32]>,
    entropy: &mut dyn Entropy,
) -> Result<Vec<u8>, EntryError> {
    let mut entry = entry.clone();
    entry.pour = match visibility {
        Visibility::Shared(to) => {
            let mut to: Vec<String> = to.iter().filter(|a| **a != account.id()).cloned().collect();
            to.sort();
            to.dedup();
            to
        }
        _ => Vec::new(),
    };
    let text = serde_json::to_string(&entry).map_err(|_| EntryError::Write(WriteError::Seal))?;
    let content_keys: Vec<[u8; 32]>;
    let content = match visibility {
        Visibility::Public => Content::Public(&text),
        Visibility::Private => Content::Sealed(&text, &[]),
        Visibility::Shared(_) => {
            let mut keys = Vec::new();
            for a in &entry.pour {
                keys.push(boxes(a).ok_or_else(|| EntryError::NoKey(a.clone()))?);
            }
            content_keys = keys;
            Content::Sealed(&text, &content_keys)
        }
    };
    write(account, item, seq, kind, content, entropy).map_err(EntryError::Write)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EntryError {
    /// Ce compte n'a pas encore publié sa clé de boîte.
    NoKey(String),
    Write(WriteError),
}

/// Le profil, signé.
pub fn write_profile(
    account: &Account,
    seq: u64,
    profile: &Profile,
    entropy: &mut dyn Entropy,
) -> Result<Vec<u8>, WriteError> {
    let mut profile = profile.clone();
    profile.boite = hex(&account.box_public());
    let text = serde_json::to_string(&profile).map_err(|_| WriteError::Seal)?;
    write(
        account,
        PROFILE_ITEM,
        seq,
        Kind::Profil,
        Content::Public(&text),
        entropy,
    )
}

/// Les réglages que dit un enregistrement, ouverts par leur compte.
pub fn settings_of(r: &Read) -> Option<Settings> {
    match (&r.kind, &r.state) {
        (Kind::Reglages, State::Sealed(Some(text))) => serde_json::from_str(text).ok(),
        _ => None,
    }
}

/// Les réglages, scellés pour le compte seul.
pub fn write_settings(
    account: &Account,
    seq: u64,
    settings: &Settings,
    entropy: &mut dyn Entropy,
) -> Result<Vec<u8>, WriteError> {
    let text = serde_json::to_string(settings).map_err(|_| WriteError::Seal)?;
    write(
        account,
        SETTINGS_ITEM,
        seq,
        Kind::Reglages,
        Content::Sealed(&text, &[]),
        entropy,
    )
}

/// Un nom d'élément neuf : tiré au hasard, pour qu'il ne dise rien de
/// ce qu'il porte.
pub fn new_item(entropy: &mut dyn Entropy) -> String {
    let mut b = [0u8; 12];
    entropy.fill(&mut b);
    hex(&b)
}

// ---------------------------------------------------------------------
// Le compte gardé sur un poste
// ---------------------------------------------------------------------

/// L'étirement d'un code : Argon2id, 64 Mio, trois passes — au-dessus du
/// plancher de l'OWASP, une ouverture coûte une fraction de seconde à la
/// personne et chaque essai autant à qui devine. Les tests en passent un
/// plus léger.
#[derive(Clone, Copy, Debug)]
pub struct Stretch {
    pub memory_kib: u32,
    pub passes: u32,
}

pub const STRETCH: Stretch = Stretch {
    memory_kib: 64 * 1024,
    passes: 3,
};

fn stretch(pin: &str, salt: &[u8; 16], s: Stretch) -> Option<[u8; 32]> {
    let params = argon2::Params::new(s.memory_kib, s.passes, 1, Some(32)).ok()?;
    let a = argon2::Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);
    let mut out = [0u8; 32];
    a.hash_password_into(pin.as_bytes(), salt, &mut out).ok()?;
    Some(out)
}

/// Un code acceptable : huit caractères au moins, **pas seulement des
/// chiffres**, pas un seul caractère répété. Le scellé vit dans la base,
/// que toute l'équipe ouvre : un code de six chiffres s'y essaie en une
/// soirée, même étiré.
pub fn pin_ok(pin: &str) -> bool {
    let n = pin.chars().count();
    let mut chars = pin.chars();
    let first = chars.next();
    n >= PIN_FLOOR && !pin.chars().all(|c| c.is_ascii_digit()) && !chars.all(|c| Some(c) == first)
}

const LOCK_VERSION: u8 = 1;
/// Ce qu'un scellé peut demander au plus : un fichier abîmé ou forgé ne
/// fait pas tourner Argon2 sur un gigaoctet.
const MOST_MEMORY_KIB: u32 = 256 * 1024;
const MOST_PASSES: u32 = 16;

/// La phrase scellée sous un code : `version (1) ‖ mémoire (4) ‖ passes
/// (4) ‖ sel (16) ‖ nonce (12) ‖ scellé`. L'étirement est écrit avec le
/// scellé : le relever un jour ne ferme aucun compte déjà gardé.
pub fn lock(phrase: &Phrase, pin: &str, s: Stretch, entropy: &mut dyn Entropy) -> Option<Vec<u8>> {
    use aes_gcm::aead::{Aead, KeyInit};
    let mut salt = [0u8; 16];
    let mut nonce = [0u8; 12];
    entropy.fill(&mut salt);
    entropy.fill(&mut nonce);
    let mut key = stretch(pin, &salt, s)?;
    let cipher = aes_gcm::Aes256Gcm::new_from_slice(&key).ok()?;
    key.zeroize();
    let mut head = vec![LOCK_VERSION];
    head.extend_from_slice(&s.memory_kib.to_le_bytes());
    head.extend_from_slice(&s.passes.to_le_bytes());
    let sealed = cipher
        .encrypt(
            aes_gcm::Nonce::from_slice(&nonce),
            aes_gcm::aead::Payload {
                msg: phrase.entropy.as_slice(),
                aad: &head,
            },
        )
        .ok()?;
    let mut out = head;
    out.extend_from_slice(&salt);
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&sealed);
    Some(out)
}

/// La phrase rendue par son code, ou `None`.
pub fn unlock(locked: &[u8], pin: &str) -> Option<Phrase> {
    use aes_gcm::aead::{Aead, KeyInit};
    if locked.len() < 9 + 28 || locked[0] != LOCK_VERSION {
        return None;
    }
    let head = &locked[..9];
    let s = Stretch {
        memory_kib: u32::from_le_bytes(locked[1..5].try_into().ok()?),
        passes: u32::from_le_bytes(locked[5..9].try_into().ok()?),
    };
    if s.memory_kib > MOST_MEMORY_KIB || s.passes > MOST_PASSES {
        return None;
    }
    let salt: [u8; 16] = locked[9..25].try_into().ok()?;
    let mut key = stretch(pin, &salt, s)?;
    let cipher = aes_gcm::Aes256Gcm::new_from_slice(&key).ok()?;
    key.zeroize();
    let mut plain = cipher
        .decrypt(
            aes_gcm::Nonce::from_slice(&locked[25..37]),
            aes_gcm::aead::Payload {
                msg: &locked[37..],
                aad: head,
            },
        )
        .ok()?;
    let entropy: Option<[u8; 16]> = plain.as_slice().try_into().ok();
    plain.zeroize();
    entropy.map(Phrase::from_entropy)
}

// ---------------------------------------------------------------------

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn unhex_vec(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(text.get(i..i + 2)?, 16).ok())
        .collect()
}

pub fn unhex<const N: usize>(text: &str) -> Option<[u8; N]> {
    unhex_vec(text)?.try_into().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un aléa qui se rejoue : le même test donne les mêmes octets.
    struct Counter(u8);
    impl Entropy for Counter {
        fn fill(&mut self, out: &mut [u8]) {
            for b in out {
                self.0 = self.0.wrapping_mul(37).wrapping_add(11);
                *b = self.0;
            }
        }
    }

    const LIGHT: Stretch = Stretch {
        memory_kib: 64,
        passes: 1,
    };

    fn account(seed: u8) -> Account {
        Account::from_phrase(Phrase::generate(&mut Counter(seed)))
    }

    #[test]
    fn a_phrase_is_read_back_however_it_is_typed() {
        let p = Phrase::generate(&mut Counter(3));
        let words = p.words();
        assert_eq!(words.len(), 12);
        assert_eq!(Phrase::parse(&words.join(" ")).unwrap(), p);
        // Sans accents, en majuscules, numérotée, sur plusieurs lignes.
        let bare: Vec<String> = words.iter().map(|w| fold(w).to_uppercase()).collect();
        let numbered: String = bare
            .iter()
            .enumerate()
            .map(|(i, w)| format!("{}. {w}\n", i + 1))
            .collect();
        assert_eq!(Phrase::parse(&numbered).unwrap(), p);
        // Quatre lettres suffisent quand elles ne désignent qu'un mot.
        let short: Vec<String> = words
            .iter()
            .map(|w| {
                let f = fold(w);
                let n = (4..=f.len())
                    .find(|n| word_index(&f[..*n]) == word_index(&f))
                    .unwrap_or(f.len());
                f[..n].to_owned()
            })
            .collect();
        assert_eq!(Phrase::parse(&short.join(" ")).unwrap(), p);
        // Composée, la liste se dessine avec les faces livrées.
        assert!(words
            .iter()
            .all(|w| !w.chars().any(|c| ('\u{300}'..='\u{36f}').contains(&c))));
    }

    #[test]
    fn a_mistyped_phrase_says_where() {
        let p = Phrase::generate(&mut Counter(5));
        let mut words = p.words();
        assert_eq!(
            Phrase::parse(&words[..11].join(" ")),
            Err(PhraseError::Count(11))
        );
        words[3] = "zzzz".to_owned();
        assert_eq!(
            Phrase::parse(&words.join(" ")),
            Err(PhraseError::Unknown(4, "zzzz".to_owned()))
        );
        // Deux mots échangés : de la liste, mais pas une phrase.
        let mut swapped = p.words();
        let i = (0..11).find(|&i| swapped[i] != swapped[i + 1]).unwrap();
        swapped.swap(i, i + 1);
        assert_eq!(
            Phrase::parse(&swapped.join(" ")),
            Err(PhraseError::Checksum)
        );
    }

    #[test]
    fn the_folded_list_still_names_every_word_once() {
        let mut folded: Vec<String> = list().iter().map(|w| fold(w)).collect();
        folded.sort();
        folded.dedup();
        assert_eq!(folded.len(), 2048);
        // Et la forme composée est la forme pliée, accents remis : rien
        // qui ne se tape au clavier français.
        for w in list() {
            assert_eq!(fold(&compose(w)), fold(w), "{w}");
        }
    }

    #[test]
    fn the_same_phrase_is_the_same_account_everywhere() {
        let a = account(9);
        let b = Account::from_phrase(Phrase::parse(&a.phrase().words().join(" ")).unwrap());
        assert_eq!(a.id(), b.id());
        assert_eq!(a.box_public(), b.box_public());
        assert_ne!(a.id(), account(10).id());
        assert_eq!(groups_of(&a.id()), a.groups());
    }

    #[test]
    fn a_record_proves_its_account_and_nothing_rewrites_it() {
        let a = account(1);
        let bytes = write(
            &a,
            "x1",
            1,
            Kind::Note,
            Content::Public("{}"),
            &mut Counter(1),
        )
        .unwrap();
        let r = read(&bytes, None).unwrap();
        assert_eq!(r.account, a.id());
        assert_eq!(r.state, State::Public("{}".to_owned()));
        assert_eq!(r.name, name_of(&bytes));
        // Un octet changé n'importe où : plus un enregistrement.
        for i in [TAG.len() + 1, TAG.len() + 40, bytes.len() - 3] {
            let mut bad = bytes.clone();
            bad[i] ^= 1;
            assert!(read(&bad, None).is_none(), "{i}");
        }
        // Un corps qui nomme un autre compte que sa clé : refusé.
        let b = account(2);
        let theirs = write(
            &b,
            "x1",
            1,
            Kind::Note,
            Content::Public("{}"),
            &mut Counter(1),
        )
        .unwrap();
        let mut forged = bytes[..TAG.len() + 96].to_vec();
        forged.extend_from_slice(&theirs[TAG.len() + 96..]);
        assert!(read(&forged, None).is_none());
        // Ni une boîte, ni un message JSON du journal.
        assert!(read(b"BOX1....", None).is_none());
        assert!(read(br#"{"t":"boite"}"#, None).is_none());
    }

    #[test]
    fn what_is_sealed_opens_for_its_readers_only() {
        let me = account(1);
        let you = account(2);
        let them = account(3);
        let boxes = |id: &str| -> Option<[u8; 32]> {
            [&me, &you, &them]
                .into_iter()
                .find(|a| a.id() == id)
                .map(|a| a.box_public())
        };
        let entry = Entry {
            titre: "Formation AOD".to_owned(),
            jour: "2026-10-22".to_owned(),
            debut: "14:00".to_owned(),
            ..Entry::default()
        };
        let private = write_entry(
            &me,
            ("r1", 1),
            Kind::Rdv,
            &entry,
            &Visibility::Private,
            &boxes,
            &mut Counter(4),
        )
        .unwrap();
        let shared = write_entry(
            &me,
            ("r2", 1),
            Kind::Rdv,
            &entry,
            &Visibility::Shared(vec![you.id()]),
            &boxes,
            &mut Counter(5),
        )
        .unwrap();
        let read_by = |b: &[u8], who: &Account| entry_of(&read(b, Some(who)).unwrap());
        assert_eq!(read_by(&private, &me).unwrap().titre, "Formation AOD");
        assert!(read_by(&private, &you).is_none());
        let mine = read_by(&shared, &me).unwrap();
        assert_eq!(mine.pour, vec![you.id()]);
        assert_eq!(
            visibility_of(&read(&shared, Some(&me)).unwrap(), &mine),
            Visibility::Shared(vec![you.id()])
        );
        assert!(read_by(&shared, &you).is_some());
        assert!(read_by(&shared, &them).is_none());
        // Sans lecteur, l'élément se garde et se passe, scellé.
        assert_eq!(read(&shared, None).unwrap().state, State::Sealed(None));
        // Le contenu ne paraît pas dans les octets.
        assert!(!String::from_utf8_lossy(&shared).contains("AOD"));
        // Un destinataire dont la clé n'est pas connue : refusé, pas oublié.
        let stranger = account(7).id();
        assert_eq!(
            write_entry(
                &me,
                ("r3", 1),
                Kind::Rdv,
                &entry,
                &Visibility::Shared(vec![stranger.clone()]),
                &|_| None,
                &mut Counter(6),
            ),
            Err(EntryError::NoKey(stranger))
        );
    }

    /// **Ce qui part au réseau** : le profil toujours, les éléments vivants
    /// les plus récents, et les effacements à part — un compte qui a
    /// beaucoup effacé publie encore ce qu'il vient d'écrire.
    #[test]
    fn the_seed_keeps_the_newest_items_and_counts_deletions_apart() {
        let a = account(1);
        let mut all = vec![(
            read(
                &write_profile(&a, 1, &Profile::default(), &mut Counter(1)).unwrap(),
                None,
            )
            .unwrap(),
            (),
        )];
        for i in 0..MOST_ITEMS as u64 + 5 {
            let gone = write(
                &a,
                &format!("g{i}"),
                100 + i,
                Kind::Note,
                Content::Deleted,
                &mut Counter(2),
            )
            .unwrap();
            all.push((read(&gone, None).unwrap(), ()));
        }
        for i in 0..MOST_ITEMS as u64 + 5 {
            let live = write(
                &a,
                &format!("l{i}"),
                10 + i,
                Kind::Note,
                Content::Public("{}"),
                &mut Counter(3),
            )
            .unwrap();
            all.push((read(&live, None).unwrap(), ()));
        }
        // Des réglages anciens, et plus de billets que le plafond : les
        // réglages partent quand même, et les billets ne prennent pas la
        // place des notes.
        let settings = write_settings(&a, 1, &Settings::default(), &mut Counter(4)).unwrap();
        all.push((read(&settings, None).unwrap(), ()));
        for i in 0..MOST_ITEMS as u64 + 5 {
            let post = write(
                &a,
                &format!("b{i}"),
                1000 + i,
                Kind::Billet,
                Content::Public("{}"),
                &mut Counter(5),
            )
            .unwrap();
            all.push((read(&post, None).unwrap(), ()));
        }
        let seed = to_seed(all);
        assert!(seed.iter().any(|(r, _)| r.kind == Kind::Profil));
        assert!(seed.iter().any(|(r, _)| r.kind == Kind::Reglages));
        assert_eq!(
            seed.iter().filter(|(r, _)| r.kind == Kind::Billet).count(),
            MOST_ITEMS
        );
        let live: Vec<u64> = seed
            .iter()
            .filter(|(r, _)| r.kind == Kind::Note && r.state != State::Deleted)
            .map(|(r, _)| r.seq)
            .collect();
        assert_eq!(live.len(), MOST_ITEMS);
        assert!(!live.contains(&10), "le plus ancien reste");
        assert!(
            live.contains(&(10 + MOST_ITEMS as u64 + 4)),
            "le plus récent part"
        );
        assert_eq!(
            seed.iter()
                .filter(|(r, _)| r.state == State::Deleted)
                .count(),
            MOST_ITEMS
        );
    }

    #[test]
    fn the_newest_version_wins_the_same_way_on_every_post() {
        let a = account(1);
        let v = |n: u64, text: &str, seed: u8| {
            read(
                &write(
                    &a,
                    "n1",
                    n,
                    Kind::Note,
                    Content::Public(text),
                    &mut Counter(seed),
                )
                .unwrap(),
                None,
            )
            .unwrap()
        };
        let one = v(1, "un", 1);
        let two = v(2, "deux", 2);
        let gone = read(
            &write(&a, "n1", 3, Kind::Note, Content::Deleted, &mut Counter(3)).unwrap(),
            None,
        )
        .unwrap();
        assert_eq!(latest([one.clone(), two.clone()])[0], two);
        assert_eq!(latest([two.clone(), one.clone()])[0], two);
        assert_eq!(latest([one, gone.clone(), two])[0], gone);
        // Deux postes, le même numéro : le même gagnant des deux côtés.
        let x = v(5, "x", 4);
        let y = v(5, "y", 5);
        assert_eq!(latest([x.clone(), y.clone()]), latest([y, x]));
    }

    #[test]
    fn the_profile_is_public_and_carries_the_box_key() {
        let a = account(1);
        let p = Profile {
            nom: "Claire Martin".to_owned(),
            role: "Pharmacienne".to_owned(),
            ..Profile::default()
        };
        let r = read(&write_profile(&a, 1, &p, &mut Counter(1)).unwrap(), None).unwrap();
        let got = profile_of(&r).unwrap();
        assert_eq!(got.nom, "Claire Martin");
        assert_eq!(unhex::<32>(&got.boite), Some(a.box_public()));
        // Un profil scellé, ou un autre élément qui se dit profil : refusés.
        let sealed = write(
            &a,
            PROFILE_ITEM,
            2,
            Kind::Profil,
            Content::Sealed("{}", &[]),
            &mut Counter(2),
        )
        .unwrap();
        assert!(read(&sealed, Some(&a)).is_none());
        let wrong = write(
            &a,
            "autre",
            2,
            Kind::Profil,
            Content::Public("{}"),
            &mut Counter(2),
        )
        .unwrap();
        assert!(read(&wrong, None).is_none());
    }

    /// **Les réglages ne se lisent que de leur compte** : scellés pour lui,
    /// refusés publics ou sous un autre nom d'élément ; une décision
    /// remplace l'autre.
    #[test]
    fn the_settings_are_sealed_for_their_account_only() {
        let a = account(1);
        let b = account(2);
        let mut s = Settings::default();
        s.set(&b.id(), Hold::Muted);
        assert_eq!(s.hold(&b.id()), Hold::Muted);
        s.set(&b.id(), Hold::Blocked);
        assert_eq!(s.hold(&b.id()), Hold::Blocked);
        assert!(s.masques.is_empty(), "une décision à la fois");
        let bytes = write_settings(&a, 1, &s, &mut Counter(1)).unwrap();
        assert_eq!(
            settings_of(&read(&bytes, Some(&a)).unwrap()),
            Some(s.clone())
        );
        assert_eq!(settings_of(&read(&bytes, Some(&b)).unwrap()), None);
        assert!(!String::from_utf8_lossy(&bytes).contains(&b.id()));
        let public = write(
            &a,
            SETTINGS_ITEM,
            2,
            Kind::Reglages,
            Content::Public("{}"),
            &mut Counter(2),
        )
        .unwrap();
        assert!(read(&public, None).is_none());
        let elsewhere = write(
            &a,
            "x",
            2,
            Kind::Reglages,
            Content::Sealed("{}", &[]),
            &mut Counter(2),
        )
        .unwrap();
        assert!(read(&elsewhere, None).is_none());
        s.set(&b.id(), Hold::Read);
        assert_eq!(s, Settings::default());
        // Un billet se lit comme une note.
        let post = write(
            &a,
            "p1",
            1,
            Kind::Billet,
            Content::Public(r#"{"texte":"Bonjour"}"#),
            &mut Counter(3),
        )
        .unwrap();
        assert_eq!(
            entry_of(&read(&post, None).unwrap()).unwrap().texte,
            "Bonjour"
        );
    }

    #[test]
    fn a_long_entry_is_refused_before_it_is_signed() {
        let a = account(1);
        let long = "x".repeat(MAX_DATA + 1);
        assert_eq!(
            write(
                &a,
                "n",
                1,
                Kind::Note,
                Content::Public(&long),
                &mut Counter(1)
            ),
            Err(WriteError::TooLong)
        );
        // Le plus long élément scellé tient dans un enregistrement du
        // journal, et se relit.
        let most = "x".repeat(MAX_DATA);
        let b = write(
            &a,
            "n",
            1,
            Kind::Note,
            Content::Sealed(&most, &[]),
            &mut Counter(1),
        )
        .unwrap();
        assert!(b.len() <= bpm_sync::MAX_PAYLOAD);
        assert!(matches!(
            read(&b, Some(&a)).unwrap().state,
            State::Sealed(Some(_))
        ));
        // **Et pour le plus de destinataires** : ce que `write` rend, `read`
        // l'accepte — sinon l'élément serait dit enregistré, puis écarté.
        let keys: Vec<[u8; 32]> = (0..MOST_RECIPIENTS as u8 - 1)
            .map(|i| account(i.wrapping_add(40)).box_public())
            .collect();
        let b = write(
            &a,
            "n",
            1,
            Kind::Note,
            Content::Sealed(&most, &keys),
            &mut Counter(1),
        )
        .unwrap();
        assert!(b.len() <= bpm_sync::MAX_PAYLOAD);
        assert!(read(&b, None).is_some());
    }

    #[test]
    fn the_pin_opens_the_phrase_and_a_wrong_one_does_not() {
        let p = Phrase::generate(&mut Counter(8));
        let locked = lock(&p, "472915ab", LIGHT, &mut Counter(9)).unwrap();
        assert_eq!(unlock(&locked, "472915ab"), Some(p.clone()));
        assert_eq!(unlock(&locked, "472916ab"), None);
        assert_eq!(unlock(&locked[..20], "472915ab"), None);
        // L'étirement voyage avec le scellé, et il est lié : abaissé à la
        // main, le scellé ne s'ouvre plus.
        let mut cheaper = locked.clone();
        cheaper[5] = 0;
        assert_eq!(unlock(&cheaper, "472915ab"), None);
        let mut greedy = locked.clone();
        greedy[1..5].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(unlock(&greedy, "472915ab"), None);
        // La phrase n'est pas dans ce qui est gardé.
        assert!(!locked.windows(16).any(|w| w == p.entropy()));
        assert!(pin_ok("472915ab") && pin_ok("abcdefgh"));
        assert!(!pin_ok("1234567") && !pin_ok("aaaaaaaa"));
        assert!(!pin_ok("47291583"), "seulement des chiffres");
    }
}
