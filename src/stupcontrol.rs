//! **StupControl**, le registre des stupéfiants tenu dans les classeurs
//! LibreOffice de l'officine (`{données}/{année}/{produit}.ods`, un
//! onglet par mois, une colonne Entrée, Sortie et Bilan par spécialité),
//! et le registre de BPM-Caddy : **les deux se tiennent à jour l'un
//! l'autre**.
//!
//! # Par où l'on passe
//!
//! Par StupControl lui-même, jamais par les classeurs directement. Un
//! classeur de registre est fragile : des en-têtes écrits à la main, une
//! ligne TOTAUX que le mois suivant lit par adresse, des bilans tirés
//! jusqu'en bas, des reports d'une année sur l'autre par lien externe.
//! StupControl sait les lire et y ajouter une ligne sans rien casser —
//! sauvegarde avant écriture, enregistrement atomique, refus tant que
//! LibreOffice tient le fichier, journal d'audit. Un deuxième moteur ici
//! serait deux façons d'écrire le même registre légal. Le programme
//! installé est donc lancé sans fenêtre (`StupControl --bridge requête
//! réponse`), une requête par appel : lire depuis une date, ajouter des
//! lignes.
//!
//! # Ce qui se rapproche
//!
//! Produit par produit (la correspondance est choisie par l'officine),
//! à partir d'une **date de départ** : ce qui précède n'est ni lu ni
//! écrit. Une ligne de chaque côté qui dit le même mouvement se **paire**
//! — une délivrance par son numéro d'ordonnancier, le reste par jour,
//! sens, quantité et nom, puis par jour, sens et quantité seuls quand il
//! n'y a qu'une façon d'apparier. Ce qui reste :
//!
//! * une ligne de BPM-Caddy absente du classeur y est **ajoutée** — le nom
//!   du patient vient du dossier, sur ce poste, et ne part nulle part
//!   ailleurs ;
//! * une ligne du classeur absente de BPM-Caddy y est **inscrite**, en
//!   ajout seul comme toute ligne du registre, rattachée au dossier
//!   seulement quand un seul porte ce nom. Le nom lui-même n'est jamais
//!   écrit dans le registre de BPM-Caddy ;
//! * ce que le registre de BPM-Caddy refuse — une délivrance sans
//!   numéro, un numéro qui n'est pas un nombre ou qui désigne déjà une
//!   délivrance — est **montré**, jamais forcé ;
//! * une ligne déjà rapprochée qui ne se retrouve plus dans le classeur
//!   (effacée à la main) est montrée : rien ne s'efface du registre.
//!
//! Le classeur ne connaît que l'entrée et la sortie. Une perte, une
//! péremption au coffre et l'écart d'un inventaire y deviennent une
//! sortie ou une entrée nommées ; une annulation, le mouvement contraire
//! de la ligne annulée ; un retour patient et une destruction ne touchent
//! pas le stock délivrable et n'y vont pas.
//!
//! Un écart de solde entre les deux registres est **dit**, produit par
//! produit, et jamais comblé d'office : un inventaire inventé pour
//! égaliser serait une fausse déclaration de comptage.
//!
//! Le rapprochement est pur ([`plan`], [`row_of`], [`import_of`]) ;
//! [`sync`] le mène contre la base et un [`Bridge`].

use crate::db::{Db, StupMove};
use crate::ordonnancier::Kind;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// La date de départ du rapprochement, ISO, dans `stup_settings`.
pub const SINCE_KEY: &str = "stupcontrol_since";
/// « 1 » quand le poste qui sert l'officine rapproche de lui-même.
pub const AUTO_KEY: &str = "stupcontrol_auto";
/// `stupcontrol_map:<id du produit>` = la clé du produit de StupControl.
pub const MAP_PREFIX: &str = "stupcontrol_map:";

/// Un produit tel que StupControl le décrit : sa clé est le fichier et
/// l'en-tête de colonne, en minuscules (`methadone.ods|méthadone 10 mg`).
#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Product {
    pub key: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub file: String,
    #[serde(default)]
    pub specialite: String,
    #[serde(default)]
    pub units_per_box: i64,
    #[serde(default)]
    pub min_stock: i64,
    /// Le Bilan du classeur, tel que StupControl le lit.
    pub stock: Option<f64>,
    #[serde(default)]
    pub movements: Vec<Line>,
}

/// Une ligne lue dans un onglet. `date` est ISO, ou absente quand la
/// cellule ne se lit pas comme une date.
#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Line {
    pub year: i64,
    pub month: i64,
    pub date: Option<String>,
    #[serde(default)]
    pub date_text: String,
    /// « E » (entrée) ou « S » (sortie).
    pub kind: String,
    pub units: f64,
    #[serde(default)]
    pub nom: String,
    #[serde(default)]
    pub prenom: String,
    #[serde(default)]
    pub ordo: String,
}

/// Ce que StupControl répond à une lecture.
#[derive(Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Answer {
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub data_dir: String,
    #[serde(default)]
    pub products: Vec<Product>,
    #[serde(default)]
    pub unreadable: Vec<String>,
}

/// Une ligne à ajouter à un classeur.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Entry {
    pub key: String,
    pub kind: String,
    pub date: String,
    pub units: f64,
    pub nom: String,
    pub prenom: String,
    pub ordo: String,
}

/// Ce que devient une ligne ajoutée. `code` nomme le refus : `verrou`
/// (ouvert dans LibreOffice), `colonnes`, `fichier`, `plein`…
#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct Outcome {
    pub ok: bool,
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
}

/// **Ce par quoi l'on parle à StupControl.** Le programme installé
/// ([`Program`]) ; un double en mémoire dans les tests.
pub trait Bridge {
    fn read(&self, since: &str) -> Result<Answer, String>;
    fn add(&self, entries: &[Entry]) -> Result<Vec<Outcome>, String>;
}

/// Le sens d'une ligne de classeur.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Side {
    E,
    S,
}

impl Side {
    fn of(kind: &str) -> Option<Side> {
        match kind {
            "E" => Some(Side::E),
            "S" => Some(Side::S),
            _ => None,
        }
    }

    fn key(self) -> &'static str {
        match self {
            Side::E => "E",
            Side::S => "S",
        }
    }

    fn flipped(self) -> Side {
        match self {
            Side::E => Side::S,
            Side::S => Side::E,
        }
    }
}

/// **La ligne de classeur qu'une ligne du registre de BPM-Caddy donne**,
/// ou rien quand elle ne touche pas le stock délivrable.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub side: Side,
    /// ISO.
    pub day: String,
    pub units: f64,
    pub who: String,
    pub prenom: String,
    pub ordo: String,
}

/// Minuscules, sans accents ni ponctuation, espaces resserrés : « DUPONT
/// » et « Dupont » sont un nom.
pub fn fold(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars().map(crate::fuzzy::fold) {
        if c.is_alphanumeric() {
            out.push(c);
        } else if !out.is_empty() && !out.ends_with(' ') {
            out.push(' ');
        }
    }
    out.trim_end().to_owned()
}

/// Le numéro d'ordonnancier d'une cellule, quand c'en est un : des
/// chiffres, éventuellement lus « 18412.0 » sur une cellule numérique.
/// « ORD-2025/42 » n'en est pas un pour BPM-Caddy, qui ne range que des
/// nombres.
pub fn ordo_number(text: &str) -> Option<i64> {
    let t = text.trim();
    let digits = t
        .strip_suffix(".0")
        .or_else(|| t.strip_suffix(",0"))
        .unwrap_or(t);
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) || digits.len() > 15 {
        return None;
    }
    digits.parse().ok().filter(|n| *n > 0)
}

/// Les mots d'une sortie qui n'est pas une délivrance : ce qu'une ligne
/// du classeur porte au lieu d'un nom de patient. Comparés repliés, au
/// début du nom.
const NOT_A_PATIENT: &[&str] = &[
    "perte",
    "casse",
    "vol",
    "perime",
    "peremption",
    "destruction",
    "inventaire",
    "ecart",
    "annulation",
    "regularisation",
];

fn names_no_patient(nom: &str) -> bool {
    let n = fold(nom);
    NOT_A_PATIENT
        .iter()
        .any(|w| n == *w || n.starts_with(&format!("{w} ")))
}

/// **La ligne de classeur d'une ligne du registre.**
///
/// `target` est la ligne qu'une annulation annule ; `patient` le nom et
/// le prénom du dossier d'une délivrance.
pub fn row_of(
    m: &StupMove,
    target: Option<&StupMove>,
    patient: Option<(&str, &str)>,
) -> Option<Row> {
    let row = |side, units: f64, who: &str, prenom: &str, ordo: String| Row {
        side,
        day: m.happened_on.clone(),
        units,
        who: who.trim().to_owned(),
        prenom: prenom.trim().to_owned(),
        ordo,
    };
    let tr = crate::strings::tr;
    match Kind::from_key(&m.kind) {
        Kind::Entree => {
            let who = if m.supplier.trim().is_empty() {
                tr("stc_word_reception")
            } else {
                m.supplier.as_str()
            };
            Some(row(Side::E, m.quantity, who, "", String::new()))
        }
        Kind::Sortie => {
            let (nom, prenom) = patient.unwrap_or(("", ""));
            let ordo = if m.ordo_no > 0 {
                m.ordo_no.to_string()
            } else {
                String::new()
            };
            Some(row(Side::S, m.quantity, nom, prenom, ordo))
        }
        Kind::Perte => Some(row(
            Side::S,
            m.quantity,
            tr("stc_word_perte"),
            "",
            String::new(),
        )),
        Kind::Peremption => Some(row(
            Side::S,
            m.quantity,
            tr("stc_word_perime"),
            "",
            String::new(),
        )),
        Kind::Inventaire => {
            let gap = m.quantity - m.expected;
            if gap.abs() < 1e-9 {
                None
            } else {
                let side = if gap > 0.0 { Side::E } else { Side::S };
                Some(row(
                    side,
                    gap.abs(),
                    tr("stc_word_inventaire"),
                    "",
                    String::new(),
                ))
            }
        }
        Kind::Annulation => {
            // Le contraire de ce que la ligne annulée avait fait au
            // classeur, et rien si elle n'y avait rien fait.
            let undone = row_of(target?, None, None)?;
            Some(row(
                undone.side.flipped(),
                undone.units,
                tr("stc_word_annulation"),
                "",
                undone.ordo,
            ))
        }
        Kind::Retour | Kind::Destruction | Kind::DestructionPerimes => None,
    }
}

/// Une ligne du registre de BPM-Caddy dans le rapprochement : ce qu'elle
/// donne au classeur, et si elle a déjà été rapprochée une fois.
#[derive(Clone, Debug, PartialEq)]
pub struct Ours {
    pub id: i64,
    pub row: Row,
    pub linked: bool,
}

/// **Ce que le rapprochement d'un produit décide.**
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Plan {
    /// (ligne de BPM-Caddy, indice de la ligne du classeur).
    pub pairs: Vec<(i64, usize)>,
    /// Les lignes de BPM-Caddy à ajouter au classeur.
    pub export: Vec<i64>,
    /// Les lignes du classeur à inscrire dans BPM-Caddy.
    pub import: Vec<usize>,
    /// Les lignes déjà rapprochées que le classeur ne porte plus.
    pub lost: Vec<i64>,
}

fn same_units(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

/// La clé stricte : le numéro d'une délivrance, le nom de tout le reste.
fn strict(side: Side, day: &str, units: f64, who: &str, ordo: &str) -> (Side, String, i64, String) {
    let tail = match (side, ordo_number(ordo)) {
        (Side::S, Some(n)) => format!("#{n}"),
        _ => fold(who),
    };
    (side, day.to_owned(), (units * 1000.0).round() as i64, tail)
}

/// **Apparier les deux registres d'un produit.**
///
/// Deux passes : la clé stricte, puis jour, sens et quantité — une ligne
/// tapée « DUPONT Albert » d'un côté et « Dupont » de l'autre est la
/// même délivrance, et la doubler serait sortir deux fois la boîte.
/// Chaque ligne ne se paire qu'une fois, dans l'ordre où elles viennent.
pub fn plan(ours: &[Ours], theirs: &[Line]) -> Plan {
    let mut out = Plan::default();
    let mut taken = vec![false; theirs.len()];
    let mut paired: HashSet<i64> = HashSet::new();
    let their_side: Vec<Option<Side>> = theirs.iter().map(|l| Side::of(&l.kind)).collect();
    // Première passe : la clé stricte.
    let mut by_key: HashMap<(Side, String, i64, String), Vec<usize>> = HashMap::new();
    for (i, l) in theirs.iter().enumerate() {
        if let (Some(side), Some(day)) = (their_side[i], l.date.as_deref()) {
            by_key
                .entry(strict(side, day, l.units, &l.nom, &l.ordo))
                .or_default()
                .push(i);
        }
    }
    for o in ours {
        let key = strict(o.row.side, &o.row.day, o.row.units, &o.row.who, &o.row.ordo);
        if let Some(i) = by_key
            .get(&key)
            .and_then(|v| v.iter().copied().find(|i| !taken[*i]))
        {
            taken[i] = true;
            paired.insert(o.id);
            out.pairs.push((o.id, i));
        }
    }
    // Deuxième passe : jour, sens et quantité. Une délivrance numérotée
    // des deux côtés sous deux numéros différents n'est pas la même.
    for o in ours {
        if paired.contains(&o.id) {
            continue;
        }
        let found = theirs.iter().enumerate().find(|(i, l)| {
            !taken[*i]
                && their_side[*i] == Some(o.row.side)
                && l.date.as_deref() == Some(o.row.day.as_str())
                && same_units(l.units, o.row.units)
                && match (ordo_number(&l.ordo), ordo_number(&o.row.ordo)) {
                    (Some(a), Some(b)) => a == b,
                    _ => true,
                }
        });
        if let Some((i, _)) = found {
            taken[i] = true;
            paired.insert(o.id);
            out.pairs.push((o.id, i));
        }
    }
    for o in ours.iter().filter(|o| !paired.contains(&o.id)) {
        if o.linked {
            out.lost.push(o.id);
        } else {
            out.export.push(o.id);
        }
    }
    out.import = (0..theirs.len()).filter(|i| !taken[*i]).collect();
    out
}

/// Ce qu'une ligne du classeur devient dans le registre de BPM-Caddy.
#[derive(Clone, Debug, PartialEq)]
pub enum Taken {
    Entree {
        supplier: String,
    },
    /// Le nom sert à retrouver le dossier, puis il est oublié.
    Sortie {
        ordo_no: i64,
        nom: String,
        prenom: String,
    },
    Perte {
        remark: String,
    },
}

/// Pourquoi une ligne du classeur n'entre pas dans le registre.
#[derive(Clone, Debug, PartialEq)]
pub enum Refusal {
    /// La cellule de la date ne se lit pas comme une date.
    NoDate,
    /// Une délivrance sans numéro d'ordonnancier.
    NoNumber,
    /// Un numéro qui n'est pas un nombre (« ORD-2025/42 »).
    OddNumber(String),
    Quantity,
    Kind,
}

impl Refusal {
    pub fn said(&self) -> String {
        use crate::strings::{tr, trf};
        match self {
            Refusal::NoDate => tr("stc_refused_date").to_owned(),
            Refusal::NoNumber => tr("stc_refused_number").to_owned(),
            Refusal::OddNumber(n) => trf("stc_refused_odd_number", n),
            Refusal::Quantity => tr("stc_refused_quantity").to_owned(),
            Refusal::Kind => tr("stc_refused_kind").to_owned(),
        }
    }
}

/// **La ligne de registre d'une ligne du classeur** : son jour, sa
/// quantité et sa nature.
pub fn import_of(l: &Line) -> Result<(String, f64, Taken), Refusal> {
    let day = l.date.clone().ok_or(Refusal::NoDate)?;
    if !l.units.is_finite() || l.units <= 0.0 {
        return Err(Refusal::Quantity);
    }
    let who = format!("{} {}", l.nom.trim(), l.prenom.trim())
        .trim()
        .to_owned();
    match Side::of(&l.kind).ok_or(Refusal::Kind)? {
        Side::E => Ok((day, l.units, Taken::Entree { supplier: who })),
        Side::S => {
            if let Some(n) = ordo_number(&l.ordo) {
                return Ok((
                    day,
                    l.units,
                    Taken::Sortie {
                        ordo_no: n,
                        nom: l.nom.trim().to_owned(),
                        prenom: l.prenom.trim().to_owned(),
                    },
                ));
            }
            if names_no_patient(&l.nom) {
                return Ok((day, l.units, Taken::Perte { remark: who }));
            }
            if l.ordo.trim().is_empty() {
                Err(Refusal::NoNumber)
            } else {
                Err(Refusal::OddNumber(l.ordo.trim().to_owned()))
            }
        }
    }
}

/// **Le dossier d'un nom lu dans le classeur**, quand un seul le porte.
///
/// Le classeur écrit le nom à sa façon : « DUPONT » et « Albert » dans
/// deux colonnes, ou « Dupont Albert » dans la première. Deux dossiers
/// du même nom, c'est aucun : rattacher une délivrance de morphine au
/// mauvais homonyme est pire que ne la rattacher à personne.
pub fn patient_for(nom: &str, prenom: &str, patients: &[(i64, String, String)]) -> i64 {
    let said = fold(&format!("{nom} {prenom}"));
    if said.is_empty() {
        return 0;
    }
    let hits: Vec<i64> = patients
        .iter()
        .filter(|(_, last, first)| {
            fold(&format!("{last} {first}")) == said || fold(&format!("{first} {last}")) == said
        })
        .map(|(id, _, _)| *id)
        .collect();
    if hits.len() == 1 {
        hits[0]
    } else {
        0
    }
}

/// **Le produit de StupControl qui ressemble le plus** à un produit suivi,
/// pour proposer la correspondance. Une proposition, jamais une décision :
/// « Skenan LP 10 mg » et « SKENAN 10 ORO » se ressemblent assez pour la
/// faire, et l'officine la valide.
pub fn suggest<'a>(label: &str, products: &'a [Product]) -> Option<&'a Product> {
    let want = fold(label);
    let words: Vec<&str> = want.split(' ').filter(|w| !w.is_empty()).collect();
    products
        .iter()
        .filter_map(|p| {
            let theirs = fold(&format!("{} {}", p.label, p.specialite));
            let shared = words
                .iter()
                .filter(|w| theirs.split(' ').any(|t| t == **w))
                .count();
            // Le premier mot (la spécialité) et le dosage doivent y être.
            let first = words.first().is_some_and(|w| theirs.contains(w));
            let dose = words
                .iter()
                .filter(|w| w.chars().all(|c| c.is_ascii_digit()))
                .all(|w| theirs.split(' ').any(|t| t == *w));
            (first && dose && shared > 0).then_some((shared, p))
        })
        .max_by_key(|(n, _)| *n)
        .map(|(_, p)| p)
}

/// Une ligne du classeur qui n'est pas entrée dans le registre.
#[derive(Clone, Debug, PartialEq)]
pub struct Refused {
    pub product: String,
    pub line: String,
    pub reason: String,
}

/// **Ce qu'un rapprochement a fait**, ou ferait (aperçu). Tenu en mémoire
/// seulement : il cite des noms lus dans les classeurs.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Report {
    pub preview: bool,
    pub version: String,
    pub data_dir: String,
    pub imported: usize,
    pub exported: usize,
    pub paired: usize,
    pub refused: Vec<Refused>,
    /// Les lignes que StupControl n'a pas pu ajouter, et pourquoi.
    pub failed: Vec<Refused>,
    /// Les lignes déjà rapprochées que le classeur ne porte plus.
    pub lost: Vec<Refused>,
    /// (produit, solde de BPM-Caddy, Bilan du classeur) quand ils diffèrent.
    pub gaps: Vec<(String, f64, f64)>,
    /// Les produits de StupControl qu'aucun produit suivi ne désigne.
    pub unmapped: Vec<Product>,
    /// Tous les produits de StupControl : (clé, libellé), pour choisir.
    pub products: Vec<(String, String)>,
    pub unreadable: Vec<String>,
}

/// Le nom d'un produit de StupControl à l'écran : son libellé, et son
/// fichier quand il n'en a pas.
pub fn shown_label(p: &Product) -> String {
    let label = if p.label.trim().is_empty() {
        p.specialite.trim()
    } else {
        p.label.trim()
    };
    format!("{label} ({})", p.file.trim())
}

fn line_said(l: &Line) -> String {
    let who = format!("{} {}", l.nom.trim(), l.prenom.trim())
        .trim()
        .to_owned();
    let mut out = format!(
        "{} — {} {}",
        if l.date_text.trim().is_empty() {
            l.date.clone().unwrap_or_default()
        } else {
            l.date_text.trim().to_owned()
        },
        if l.kind == "E" {
            crate::strings::tr("stc_side_e")
        } else {
            crate::strings::tr("stc_side_s")
        },
        crate::strings::decimal(l.units, 2)
    );
    if !who.is_empty() {
        out.push_str(&format!(" — {who}"));
    }
    if !l.ordo.trim().is_empty() {
        out.push_str(&format!(" — n° {}", l.ordo.trim()));
    }
    out
}

fn move_said(m: &StupMove) -> String {
    format!(
        "{} — {} {}",
        crate::db::format_french_date(&m.happened_on),
        crate::strings::tr(Kind::from_key(&m.kind).label_key()),
        crate::strings::decimal(m.quantity, 2)
    )
}

/// **Rapprocher les deux registres**, ou dire ce que cela ferait
/// (`preview`).
///
/// Une ligne inscrite et son lien s'écrivent dans une transaction : un
/// arrêt entre les deux laisserait une ligne que le rapprochement suivant
/// inscrirait une seconde fois. Une ligne ajoutée au classeur puis non
/// liée (le programme s'arrête entre les deux) se paire au passage
/// suivant : elle y est, sous la même clé.
pub fn sync(db: &Db, bridge: &dyn Bridge, today: &str, preview: bool) -> Result<Report, String> {
    let since = db.stup_setting(SINCE_KEY).unwrap_or_default();
    if since.trim().is_empty() {
        return Err(crate::strings::tr("stc_err_no_since").to_owned());
    }
    let answer = bridge.read(since.trim())?;
    let mut report = Report {
        preview,
        version: answer.version.clone(),
        data_dir: answer.data_dir.clone(),
        unreadable: answer.unreadable.clone(),
        products: answer
            .products
            .iter()
            .map(|p| (p.key.clone(), shown_label(p)))
            .collect(),
        ..Report::default()
    };
    let mapping = db.stupcontrol_mapping()?;
    let by_key: HashMap<&str, i64> = mapping.iter().map(|(id, k)| (k.as_str(), *id)).collect();
    report.unmapped = answer
        .products
        .iter()
        .filter(|p| !by_key.contains_key(p.key.as_str()))
        .cloned()
        .collect();
    let products: HashMap<i64, crate::db::Stupefiant> =
        db.stupefiants()?.into_iter().map(|p| (p.id, p)).collect();
    let links = db.stup_links()?;
    let patients: Vec<(i64, String, String)> = db
        .patients()?
        .into_iter()
        .map(|p| (p.id, p.last_name, p.first_name))
        .collect();
    let names: HashMap<i64, (String, String)> = patients
        .iter()
        .map(|(id, l, f)| (*id, (l.clone(), f.clone())))
        .collect();
    let mut entries: Vec<(i64, Entry, f64)> = Vec::new();
    let mut exported_effect: HashMap<i64, f64> = HashMap::new();
    for theirs in &answer.products {
        let Some(&stup_id) = by_key.get(theirs.key.as_str()) else {
            continue;
        };
        let Some(product) = products.get(&stup_id) else {
            continue;
        };
        let all = db.stup_moves(stup_id)?;
        let by_id: HashMap<i64, &StupMove> = all.iter().map(|m| (m.id, m)).collect();
        let ours: Vec<Ours> = all
            .iter()
            .filter(|m| m.happened_on.as_str() >= since.trim())
            .filter_map(|m| {
                let target = by_id.get(&m.cancels).copied();
                let patient = names
                    .get(&m.patient_id)
                    .map(|(l, f)| (l.as_str(), f.as_str()));
                row_of(m, target, patient).map(|row| Ours {
                    id: m.id,
                    row,
                    linked: links.contains(&m.id),
                })
            })
            .collect();
        let decided = plan(&ours, &theirs.movements);
        report.paired += decided.pairs.len();
        if !preview {
            for (id, _) in &decided.pairs {
                if !links.contains(id) {
                    db.add_stup_link(*id, "pair", today)?;
                }
            }
        }
        for id in &decided.lost {
            if let Some(m) = by_id.get(id) {
                report.lost.push(Refused {
                    product: product.label.clone(),
                    line: move_said(m),
                    reason: crate::strings::tr("stc_lost").to_owned(),
                });
            }
        }
        // Les lignes du classeur, dans l'ordre des jours.
        let mut incoming: Vec<&Line> = decided
            .import
            .iter()
            .map(|i| &theirs.movements[*i])
            .collect();
        incoming.sort_by(|a, b| a.date.cmp(&b.date));
        for l in incoming {
            let refuse = |reason: String| Refused {
                product: product.label.clone(),
                line: line_said(l),
                reason,
            };
            let (day, quantity, taken) = match import_of(l) {
                Ok(t) => t,
                Err(r) => {
                    report.refused.push(refuse(r.said()));
                    continue;
                }
            };
            let mut m = StupMove {
                id: 0,
                stup_id,
                kind: String::new(),
                happened_on: day,
                quantity,
                ordo_year: 0,
                ordo_no: 0,
                patient_id: 0,
                prescriber: String::new(),
                supplier: String::new(),
                reference: String::new(),
                lot: String::new(),
                expiry: String::new(),
                expected: 0.0,
                operator: crate::strings::tr("stc_operator").to_owned(),
                remark: crate::strings::tr("stc_remark").to_owned(),
                cancels: 0,
            };
            match taken {
                Taken::Entree { supplier } => {
                    m.kind = Kind::Entree.as_key().to_owned();
                    m.supplier = supplier;
                }
                Taken::Sortie {
                    ordo_no,
                    nom,
                    prenom,
                } => {
                    m.kind = Kind::Sortie.as_key().to_owned();
                    m.ordo_no = ordo_no;
                    m.patient_id = patient_for(&nom, &prenom, &patients);
                }
                Taken::Perte { remark } => {
                    m.kind = Kind::Perte.as_key().to_owned();
                    m.remark = crate::strings::trf("stc_remark_said", remark);
                }
            }
            if preview {
                report.imported += 1;
                continue;
            }
            match db.add_stup_move_linked(&m, "import", today) {
                Ok(_) => report.imported += 1,
                Err(e) => report.refused.push(refuse(e)),
            }
        }
        for id in &decided.export {
            let Some(o) = ours.iter().find(|o| o.id == *id) else {
                continue;
            };
            let effect = match o.row.side {
                Side::E => o.row.units,
                Side::S => -o.row.units,
            };
            entries.push((
                *id,
                Entry {
                    key: theirs.key.clone(),
                    kind: o.row.side.key().to_owned(),
                    date: o.row.day.clone(),
                    units: o.row.units,
                    nom: o.row.who.clone(),
                    prenom: o.row.prenom.clone(),
                    ordo: o.row.ordo.clone(),
                },
                effect,
            ));
        }
    }
    if preview {
        report.exported = entries.len();
    } else if !entries.is_empty() {
        let sent: Vec<Entry> = entries.iter().map(|(_, e, _)| e.clone()).collect();
        let outcomes = bridge.add(&sent)?;
        for ((id, entry, effect), outcome) in entries.iter().zip(outcomes.iter()) {
            let stup_id = by_key.get(entry.key.as_str()).copied().unwrap_or(0);
            if outcome.ok {
                db.add_stup_link(*id, "export", today)?;
                report.exported += 1;
                *exported_effect.entry(stup_id).or_insert(0.0) += effect;
            } else {
                let label = products
                    .get(&stup_id)
                    .map(|p| p.label.clone())
                    .unwrap_or_default();
                let line = db
                    .stup_moves(stup_id)?
                    .iter()
                    .find(|m| m.id == *id)
                    .map(move_said)
                    .unwrap_or_default();
                report.failed.push(Refused {
                    product: label,
                    line,
                    reason: outcome
                        .error
                        .clone()
                        .unwrap_or_else(|| outcome.code.clone().unwrap_or_default()),
                });
            }
        }
    }
    // Les soldes, après ce qui vient d'être fait. Un aperçu compare ce qui
    // est, sans projeter ce qu'il ferait.
    let standing: HashMap<i64, f64> = db
        .stup_summary()?
        .into_iter()
        .map(|s| (s.product.id, s.stock))
        .collect();
    for theirs in &answer.products {
        let Some(&stup_id) = by_key.get(theirs.key.as_str()) else {
            continue;
        };
        let (Some(ours), Some(stock)) = (standing.get(&stup_id), theirs.stock) else {
            continue;
        };
        let stock = stock + exported_effect.get(&stup_id).copied().unwrap_or(0.0);
        if !same_units(*ours, stock) {
            let label = products
                .get(&stup_id)
                .map(|p| p.label.clone())
                .unwrap_or_default();
            report.gaps.push((label, *ours, stock));
        }
    }
    Ok(report)
}

/// **Le programme StupControl installé**, lancé sans fenêtre pour une
/// requête.
pub struct Program {
    pub exe: PathBuf,
    /// Le fichier de réglages de StupControl, vide pour le sien.
    pub config: String,
    pub patience: Duration,
}

/// Où StupControl s'installe d'ordinaire, quand le chemin n'est pas
/// écrit dans les options.
pub fn find_program(written: &str) -> Option<PathBuf> {
    let written = written.trim();
    if !written.is_empty() {
        let p = PathBuf::from(written);
        return p.exists().then_some(p);
    }
    let mut candidates: Vec<PathBuf> = Vec::new();
    if cfg!(windows) {
        for var in ["LOCALAPPDATA", "ProgramFiles", "ProgramFiles(x86)"] {
            if let Ok(base) = std::env::var(var) {
                // L'installation (jpackage) ou l'archive zip, qui se lance
                // par un .bat.
                let dir = Path::new(&base).join("StupControl");
                candidates.push(dir.join("StupControl.exe"));
                candidates.push(dir.join("StupControl.bat"));
            }
        }
    } else if cfg!(target_os = "macos") {
        candidates.push(PathBuf::from(
            "/Applications/StupControl.app/Contents/MacOS/StupControl",
        ));
    } else {
        candidates.push(PathBuf::from("/opt/stupcontrol/bin/StupControl"));
        if let Ok(home) = std::env::var("HOME") {
            candidates.push(
                Path::new(&home)
                    .join("StupControl")
                    .join("bin")
                    .join("StupControl"),
            );
        }
    }
    candidates.into_iter().find(|p| p.exists())
}

impl Program {
    fn call(&self, request: serde_json::Value) -> Result<serde_json::Value, String> {
        static CALLS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "bpm-caddy-stupcontrol-{}-{}",
            std::process::id(),
            CALLS.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let answer = self.call_in(&dir, request);
        let _ = std::fs::remove_dir_all(&dir);
        answer
    }

    fn call_in(
        &self,
        dir: &Path,
        mut request: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        if !self.config.trim().is_empty() {
            request["config"] = serde_json::Value::String(self.config.trim().to_owned());
        }
        let req = dir.join("requete.json");
        let resp = dir.join("reponse.json");
        std::fs::write(&req, request.to_string()).map_err(|e| e.to_string())?;
        let mut child = std::process::Command::new(&self.exe)
            .arg("--bridge")
            .arg(&req)
            .arg(&resp)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| crate::strings::trf("stc_err_launch", e))?;
        let deadline = Instant::now() + self.patience;
        loop {
            match child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(100));
                }
                _ => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(crate::strings::tr("stc_err_timeout").to_owned());
                }
            }
        }
        let text = std::fs::read_to_string(&resp)
            .map_err(|_| crate::strings::tr("stc_err_no_answer").to_owned())?;
        let value: serde_json::Value =
            serde_json::from_str(&text).map_err(|e| crate::strings::trf("stc_err_answer", e))?;
        if value["ok"].as_bool() != Some(true) {
            return Err(crate::strings::trf(
                "stc_err_refused",
                value["error"].as_str().unwrap_or("?"),
            ));
        }
        Ok(value)
    }

    /// Le programme répond-il, et sur quel répertoire ?
    pub fn ping(&self) -> Result<Answer, String> {
        let value = self.call(serde_json::json!({"op": "ping"}))?;
        serde_json::from_value(value).map_err(|e| crate::strings::trf("stc_err_answer", e))
    }
}

impl Bridge for Program {
    fn read(&self, since: &str) -> Result<Answer, String> {
        let value = self.call(serde_json::json!({"op": "read", "since": since}))?;
        serde_json::from_value(value).map_err(|e| crate::strings::trf("stc_err_answer", e))
    }

    fn add(&self, entries: &[Entry]) -> Result<Vec<Outcome>, String> {
        let value = self.call(serde_json::json!({"op": "add", "entries": entries}))?;
        let outcomes: Vec<Outcome> = serde_json::from_value(value["results"].clone())
            .map_err(|e| crate::strings::trf("stc_err_answer", e))?;
        if outcomes.len() != entries.len() {
            return Err(crate::strings::tr("stc_err_short_answer").to_owned());
        }
        Ok(outcomes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    fn mv(id: i64, kind: Kind, day: &str, quantity: f64) -> StupMove {
        StupMove {
            id,
            stup_id: 1,
            kind: kind.as_key().to_owned(),
            happened_on: day.to_owned(),
            quantity,
            ordo_year: 0,
            ordo_no: 0,
            patient_id: 0,
            prescriber: String::new(),
            supplier: String::new(),
            reference: String::new(),
            lot: String::new(),
            expiry: String::new(),
            expected: 0.0,
            operator: String::new(),
            remark: String::new(),
            cancels: 0,
        }
    }

    fn line(kind: &str, day: &str, units: f64, nom: &str, prenom: &str, ordo: &str) -> Line {
        Line {
            year: day[..4].parse().unwrap(),
            month: day[5..7].parse().unwrap(),
            date: Some(day.to_owned()),
            date_text: format!("{}/{}/{}", &day[8..10], &day[5..7], &day[2..4]),
            kind: kind.to_owned(),
            units,
            nom: nom.to_owned(),
            prenom: prenom.to_owned(),
            ordo: ordo.to_owned(),
        }
    }

    #[test]
    fn an_ordonnancier_number_is_digits_and_nothing_else() {
        assert_eq!(ordo_number("18412"), Some(18412));
        assert_eq!(ordo_number(" 18412.0 "), Some(18412));
        assert_eq!(ordo_number("ORD-2025/42"), None);
        assert_eq!(ordo_number(""), None);
        assert_eq!(ordo_number("0"), None);
    }

    /// **Le classeur ne connaît que l'entrée et la sortie** : chaque nature
    /// du registre y devient l'une ou l'autre, ou rien.
    #[test]
    fn each_kind_of_register_line_gives_its_spreadsheet_row() {
        let mut e = mv(1, Kind::Entree, "2026-03-02", 28.0);
        e.supplier = "CERP".into();
        let r = row_of(&e, None, None).unwrap();
        assert_eq!((r.side, r.units, r.who.as_str()), (Side::E, 28.0, "CERP"));

        let mut s = mv(2, Kind::Sortie, "2026-03-03", 7.0);
        s.ordo_no = 18412;
        let r = row_of(&s, None, Some(("Dupont", "Albert"))).unwrap();
        assert_eq!(
            (r.side, r.who.as_str(), r.prenom.as_str(), r.ordo.as_str()),
            (Side::S, "Dupont", "Albert", "18412")
        );

        let r = row_of(&mv(3, Kind::Perte, "2026-03-04", 1.0), None, None).unwrap();
        assert_eq!((r.side, r.who.as_str()), (Side::S, "Perte"));
        let r = row_of(&mv(4, Kind::Peremption, "2026-03-04", 2.0), None, None).unwrap();
        assert_eq!(r.side, Side::S);

        let mut count = mv(5, Kind::Inventaire, "2026-03-05", 18.0);
        count.expected = 20.0;
        let r = row_of(&count, None, None).unwrap();
        assert_eq!((r.side, r.units), (Side::S, 2.0));
        count.expected = 18.0;
        assert_eq!(
            row_of(&count, None, None),
            None,
            "un comptage juste n'écrit rien"
        );

        let mut undo = mv(6, Kind::Annulation, "2026-03-06", 0.0);
        undo.cancels = 2;
        let r = row_of(&undo, Some(&s), None).unwrap();
        assert_eq!(
            (r.side, r.units, r.who.as_str(), r.ordo.as_str()),
            (Side::E, 7.0, "Annulation", "18412")
        );

        for kind in [Kind::Retour, Kind::Destruction, Kind::DestructionPerimes] {
            assert_eq!(
                row_of(&mv(7, kind, "2026-03-07", 3.0), None, None),
                None,
                "{kind:?}"
            );
        }
        let mut undo_return = mv(8, Kind::Annulation, "2026-03-08", 0.0);
        undo_return.cancels = 7;
        assert_eq!(
            row_of(
                &undo_return,
                Some(&mv(7, Kind::Retour, "2026-03-07", 3.0)),
                None
            ),
            None
        );
    }

    fn ours(
        id: i64,
        side: Side,
        day: &str,
        units: f64,
        who: &str,
        ordo: &str,
        linked: bool,
    ) -> Ours {
        Ours {
            id,
            row: Row {
                side,
                day: day.into(),
                units,
                who: who.into(),
                prenom: String::new(),
                ordo: ordo.into(),
            },
            linked,
        }
    }

    /// **Une délivrance se reconnaît à son numéro**, quel que soit le nom
    /// tapé ; le reste par le jour, le sens et la quantité ; ce qui ne se
    /// retrouve pas part dans l'autre registre — sauf ce qui avait déjà
    /// été rapproché, qui est dit perdu.
    #[test]
    fn two_registers_pair_line_by_line_and_the_rest_crosses() {
        let mine = [
            ours(1, Side::S, "2026-03-03", 7.0, "Dupont", "18412", false),
            ours(2, Side::E, "2026-03-02", 28.0, "CERP", "", false),
            ours(3, Side::S, "2026-03-04", 7.0, "Martin", "18413", false),
            ours(4, Side::S, "2026-03-05", 1.0, "Perte", "", true),
        ];
        let theirs = [
            line("S", "2026-03-03", 7.0, "DUPONT Albert", "", "18412"),
            line("E", "2026-03-02", 28.0, "Cerp Rouen", "", ""),
            // Même jour et même quantité, mais un autre numéro : une autre
            // délivrance.
            line("S", "2026-03-04", 7.0, "Martin", "", "18999"),
        ];
        let p = plan(&mine, &theirs);
        let mut pairs = p.pairs.clone();
        pairs.sort();
        assert_eq!(pairs, vec![(1, 0), (2, 1)]);
        assert_eq!(p.export, vec![3]);
        assert_eq!(p.lost, vec![4]);
        assert_eq!(p.import, vec![2]);
    }

    #[test]
    fn two_identical_lines_pair_one_each() {
        let mine = [ours(1, Side::E, "2026-03-02", 28.0, "CERP", "", false)];
        let theirs = [
            line("E", "2026-03-02", 28.0, "CERP", "", ""),
            line("E", "2026-03-02", 28.0, "CERP", "", ""),
        ];
        let p = plan(&mine, &theirs);
        assert_eq!(p.pairs.len(), 1);
        assert_eq!(
            p.import.len(),
            1,
            "la seconde boîte reçue est une seconde réception"
        );
    }

    #[test]
    fn a_spreadsheet_line_becomes_a_register_line_or_says_why_not() {
        let (day, q, t) = import_of(&line("E", "2026-03-02", 28.0, "OCP", "", "")).unwrap();
        assert_eq!((day.as_str(), q), ("2026-03-02", 28.0));
        assert_eq!(
            t,
            Taken::Entree {
                supplier: "OCP".into()
            }
        );
        let (_, _, t) =
            import_of(&line("S", "2026-03-03", 7.0, "Dupont", "Albert", "18412")).unwrap();
        assert!(matches!(t, Taken::Sortie { ordo_no: 18412, .. }));
        let (_, _, t) = import_of(&line("S", "2026-03-03", 1.0, "Casse", "", "")).unwrap();
        assert!(matches!(t, Taken::Perte { .. }));
        assert_eq!(
            import_of(&line("S", "2026-03-03", 7.0, "Dupont", "", "")),
            Err(Refusal::NoNumber)
        );
        assert_eq!(
            import_of(&line("S", "2026-03-03", 7.0, "Dupont", "", "ORD-2025/42")),
            Err(Refusal::OddNumber("ORD-2025/42".into()))
        );
        let mut undated = line("E", "2026-03-02", 28.0, "OCP", "", "");
        undated.date = None;
        assert_eq!(import_of(&undated), Err(Refusal::NoDate));
    }

    /// **Un homonyme, c'est personne** : une délivrance de morphine sur le
    /// mauvais dossier est pire qu'une délivrance sans dossier.
    #[test]
    fn a_name_finds_its_file_only_when_one_file_bears_it() {
        let patients = vec![
            (1, "Dupont".to_owned(), "Albert".to_owned()),
            (2, "Martin".to_owned(), "Paul".to_owned()),
            (3, "Martin".to_owned(), "Paul".to_owned()),
        ];
        assert_eq!(patient_for("DUPONT", "Albert", &patients), 1);
        assert_eq!(patient_for("Dupont Albert", "", &patients), 1);
        assert_eq!(patient_for("Albert", "Dupont", &patients), 1);
        assert_eq!(patient_for("Martin", "Paul", &patients), 0);
        assert_eq!(patient_for("", "", &patients), 0);
    }

    fn product(key: &str, label: &str, spec: &str) -> Product {
        Product {
            key: key.into(),
            label: label.into(),
            file: String::new(),
            specialite: spec.into(),
            units_per_box: 14,
            min_stock: 0,
            stock: Some(0.0),
            movements: Vec::new(),
        }
    }

    #[test]
    fn the_closest_product_is_proposed_dose_included() {
        let products = vec![
            product("skenan.ods|skenan 20 lp", "SKENAN 20 LP", "SKENAN 20 LP"),
            product("skenan.ods|skenan 10 oro", "SKENAN 10 ORO", "SKENAN 10 ORO"),
            product(
                "methadone.ods|methadone 10 mg",
                "Méthadone 10 mg",
                "Méthadone 10 mg",
            ),
        ];
        assert_eq!(
            suggest("Skenan LP 10 mg", &products).map(|p| p.key.as_str()),
            Some("skenan.ods|skenan 10 oro")
        );
        assert_eq!(
            suggest("Méthadone AP-HP 10 mg gélule", &products).map(|p| p.key.as_str()),
            Some("methadone.ods|methadone 10 mg")
        );
        assert_eq!(suggest("Actiskenan 5 mg", &products), None);
    }

    /// Un StupControl en mémoire : ce qu'il lit, et ce qu'on lui ajoute.
    struct Fake {
        products: RefCell<Vec<Product>>,
        refuse: RefCell<Option<String>>,
        added: RefCell<Vec<Entry>>,
    }

    impl Bridge for Fake {
        fn read(&self, since: &str) -> Result<Answer, String> {
            let mut products = self.products.borrow().clone();
            for p in &mut products {
                p.movements
                    .retain(|l| l.date.as_deref().is_none_or(|d| d >= since));
            }
            Ok(Answer {
                version: "test".into(),
                data_dir: "/registres".into(),
                products,
                unreadable: Vec::new(),
            })
        }

        fn add(&self, entries: &[Entry]) -> Result<Vec<Outcome>, String> {
            let mut out = Vec::new();
            for e in entries {
                if let Some(code) = self.refuse.borrow().clone() {
                    out.push(Outcome {
                        ok: false,
                        code: Some(code),
                        error: Some("Le fichier est ouvert dans LibreOffice.".into()),
                    });
                    continue;
                }
                let mut products = self.products.borrow_mut();
                let p = products.iter_mut().find(|p| p.key == e.key).unwrap();
                p.movements
                    .push(line(&e.kind, &e.date, e.units, &e.nom, &e.prenom, &e.ordo));
                *p.stock.as_mut().unwrap() += if e.kind == "E" { e.units } else { -e.units };
                self.added.borrow_mut().push(e.clone());
                out.push(Outcome {
                    ok: true,
                    code: None,
                    error: None,
                });
            }
            Ok(out)
        }
    }

    fn officine(tag: &str) -> (crate::db::Swept, Db) {
        let dir = std::env::temp_dir().join(format!("bpm-caddy-stc-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let swept = crate::db::Swept(dir.clone());
        let db = Db::open(&dir.join("stc.db"), "secret").unwrap();
        (swept, db)
    }

    const KEY: &str = "methadone.ods|méthadone 10 mg";

    /// **Les deux registres se tiennent à jour l'un l'autre**, une fois :
    /// ce qui est des deux côtés se paire, ce qui manque passe, ce que le
    /// registre refuse est dit, et un second passage ne fait plus rien.
    #[test]
    fn both_registers_end_up_holding_the_same_lines_and_a_second_pass_does_nothing() {
        let (_swept, db) = officine("both");
        let stup = db
            .add_stupefiant(0, "Méthadone 10 mg", "gélule", 0.0)
            .unwrap();
        db.set_stupcontrol_map(stup, KEY).unwrap();
        db.set_stup_setting(SINCE_KEY, "2026-03-01").unwrap();
        let dupont = db.add_patient("Dupont", "Albert", "1950-01-01").unwrap();
        let martin = db.add_patient("Martin", "Paul", "1960-01-01").unwrap();
        // Avant la date de départ : ni lu ni écrit.
        let mut old = mv(0, Kind::Entree, "2026-02-01", 14.0);
        old.stup_id = stup;
        db.add_stup_move(&old).unwrap();
        let mut entree = mv(0, Kind::Entree, "2026-03-02", 28.0);
        entree.stup_id = stup;
        entree.supplier = "CERP".into();
        db.add_stup_move(&entree).unwrap();
        let mut sortie = mv(0, Kind::Sortie, "2026-03-03", 7.0);
        sortie.stup_id = stup;
        sortie.ordo_no = 18412;
        sortie.patient_id = dupont;
        db.add_stup_move(&sortie).unwrap();

        let mut p = product(KEY, "Méthadone 10 mg", "Méthadone 10 mg");
        p.stock = Some(14.0 + 28.0);
        p.movements = vec![
            line("E", "2026-02-01", 14.0, "OCP", "", ""),
            line("E", "2026-03-02", 28.0, "Cerp", "", ""),
            line("S", "2026-03-04", 7.0, "MARTIN", "Paul", "18500"),
            line("S", "2026-03-05", 7.0, "Durand", "", ""),
        ];
        let fake = Fake {
            products: RefCell::new(vec![p]),
            refuse: RefCell::new(None),
            added: RefCell::new(Vec::new()),
        };

        let seen = sync(&db, &fake, "2026-03-10", true).unwrap();
        assert!(seen.preview);
        assert_eq!((seen.paired, seen.imported, seen.exported), (1, 1, 1));
        assert!(fake.added.borrow().is_empty(), "un aperçu n'écrit rien");
        assert!(db.stup_links().unwrap().is_empty());

        let done = sync(&db, &fake, "2026-03-10", false).unwrap();
        assert_eq!((done.paired, done.imported, done.exported), (1, 1, 1));
        assert_eq!(done.refused.len(), 1, "{:?}", done.refused);
        assert!(done.refused[0].line.contains("Durand"));
        // La délivrance de BPM-Caddy est partie avec le nom du dossier.
        let sent = fake.added.borrow().clone();
        assert_eq!(sent.len(), 1);
        assert_eq!(
            (
                sent[0].kind.as_str(),
                sent[0].nom.as_str(),
                sent[0].prenom.as_str(),
                sent[0].ordo.as_str()
            ),
            ("S", "Dupont", "Albert", "18412")
        );
        // Celle du classeur est inscrite, rattachée au seul dossier du nom,
        // sans que le nom soit écrit nulle part dans le registre.
        let moves = db.stup_moves(stup).unwrap();
        let taken = moves.iter().find(|m| m.ordo_no == 18500).unwrap();
        assert_eq!(taken.patient_id, martin);
        assert!(!taken.remark.contains("MARTIN") && !taken.remark.contains("Martin"));

        let again = sync(&db, &fake, "2026-03-11", false).unwrap();
        assert_eq!((again.imported, again.exported), (0, 0));
        assert_eq!(again.paired, 3);
        assert_eq!(
            again.refused.len(),
            1,
            "le refus reste dit tant qu'il n'est pas réglé"
        );
        assert_eq!(db.stup_moves(stup).unwrap().len(), moves.len());
    }

    /// **Un classeur ouvert dans LibreOffice** refuse la ligne : elle est
    /// dite, et repart au passage suivant.
    #[test]
    fn a_line_the_spreadsheet_refused_goes_again_next_time() {
        let (_swept, db) = officine("refused");
        let stup = db
            .add_stupefiant(0, "Méthadone 10 mg", "gélule", 0.0)
            .unwrap();
        db.set_stupcontrol_map(stup, KEY).unwrap();
        db.set_stup_setting(SINCE_KEY, "2026-03-01").unwrap();
        let mut entree = mv(0, Kind::Entree, "2026-03-02", 28.0);
        entree.stup_id = stup;
        db.add_stup_move(&entree).unwrap();
        let fake = Fake {
            products: RefCell::new(vec![product(KEY, "Méthadone 10 mg", "Méthadone 10 mg")]),
            refuse: RefCell::new(Some("verrou".into())),
            added: RefCell::new(Vec::new()),
        };
        let first = sync(&db, &fake, "2026-03-10", false).unwrap();
        assert_eq!(first.exported, 0);
        assert_eq!(first.failed.len(), 1);
        assert!(first.failed[0].reason.contains("LibreOffice"));
        assert!(db.stup_links().unwrap().is_empty());
        *fake.refuse.borrow_mut() = None;
        let second = sync(&db, &fake, "2026-03-11", false).unwrap();
        assert_eq!(second.exported, 1);
        assert!(second.gaps.is_empty(), "{:?}", second.gaps);
    }

    /// **Un écart de solde est dit, jamais comblé.**
    #[test]
    fn a_balance_gap_is_said_and_left_alone() {
        let (_swept, db) = officine("gap");
        let stup = db
            .add_stupefiant(0, "Méthadone 10 mg", "gélule", 0.0)
            .unwrap();
        db.set_stupcontrol_map(stup, KEY).unwrap();
        db.set_stup_setting(SINCE_KEY, "2026-03-01").unwrap();
        let mut p = product(KEY, "Méthadone 10 mg", "Méthadone 10 mg");
        p.stock = Some(21.0);
        let fake = Fake {
            products: RefCell::new(vec![p]),
            refuse: RefCell::new(None),
            added: RefCell::new(Vec::new()),
        };
        let r = sync(&db, &fake, "2026-03-10", false).unwrap();
        assert_eq!(r.gaps, vec![("Méthadone 10 mg".to_owned(), 0.0, 21.0)]);
        assert!(
            db.stup_moves(stup).unwrap().is_empty(),
            "aucun inventaire inventé"
        );
    }

    #[test]
    fn a_spreadsheet_product_is_designated_by_one_followed_product_only() {
        let (_swept, db) = officine("twice");
        let a = db
            .add_stupefiant(0, "Méthadone 10 mg", "gélule", 0.0)
            .unwrap();
        let b = db
            .add_stupefiant(0, "Méthadone 10 mg (EG)", "gélule", 0.0)
            .unwrap();
        db.set_stupcontrol_map(a, KEY).unwrap();
        assert!(db.set_stupcontrol_map(b, KEY).is_err());
        db.set_stupcontrol_map(a, "").unwrap();
        db.set_stupcontrol_map(b, KEY).unwrap();
        assert_eq!(db.stupcontrol_mapping().unwrap(), vec![(b, KEY.to_owned())]);
    }

    /// **Le vrai StupControl, de bout en bout** : une délivrance de
    /// BPM-Caddy écrite dans un vrai classeur, relue, et un second passage
    /// qui ne fait plus rien. Lancé quand `BPM_CADDY_STUPCONTROL` désigne
    /// le programme construit (`./gradlew installDist`) et
    /// `BPM_CADDY_STUPCONTROL_SAMPLE` l'arbre d'exemple
    /// (`scripts/scaffold_tree.py`) ; sinon, rien à dire.
    #[test]
    fn the_installed_program_writes_and_reads_a_real_register() {
        let (Ok(exe), Ok(sample)) = (
            std::env::var("BPM_CADDY_STUPCONTROL"),
            std::env::var("BPM_CADDY_STUPCONTROL_SAMPLE"),
        ) else {
            return;
        };
        let (swept, db) = officine("real");
        let data = swept.0.join("registres");
        copy_tree(Path::new(&sample), &data);
        let config = swept.0.join("stupcontrol.json");
        std::fs::write(
            &config,
            serde_json::json!({
                "dataDir": data,
                "drugs": [{"commercialName": "Méthadone 10 mg", "fileName": "methadone.ods",
                           "specialite": "Méthadone 10 mg", "minStock": 0}],
                "repartiteurs": [],
                "patientColumns": {}
            })
            .to_string(),
        )
        .unwrap();
        let program = Program {
            exe: PathBuf::from(exe),
            config: config.to_string_lossy().into_owned(),
            patience: Duration::from_secs(120),
        };
        assert_eq!(program.ping().unwrap().data_dir, data.to_string_lossy());
        let stup = db
            .add_stupefiant(0, "Méthadone 10 mg", "gélule", 0.0)
            .unwrap();
        db.set_stupcontrol_map(stup, KEY).unwrap();
        db.set_stup_setting(SINCE_KEY, "2025-03-01").unwrap();
        let dupont = db.add_patient("Dupont", "Albert", "1950-01-01").unwrap();
        let mut entree = mv(0, Kind::Entree, "2025-03-02", 28.0);
        entree.stup_id = stup;
        entree.supplier = "CERP".into();
        db.add_stup_move(&entree).unwrap();
        let mut sortie = mv(0, Kind::Sortie, "2025-03-03", 7.0);
        sortie.stup_id = stup;
        sortie.ordo_no = 18412;
        sortie.patient_id = dupont;
        db.add_stup_move(&sortie).unwrap();

        let first = sync(&db, &program, "2025-03-10", false).unwrap();
        assert_eq!(first.exported, 2, "{first:?}");
        assert!(first.failed.is_empty(), "{:?}", first.failed);
        let read = program.read("2025-03-01").unwrap();
        let p = read.products.iter().find(|p| p.key == KEY).unwrap();
        let written = p.movements.iter().find(|l| l.ordo == "18412").unwrap();
        assert_eq!(
            (
                written.date.as_deref(),
                written.kind.as_str(),
                written.nom.as_str()
            ),
            (Some("2025-03-03"), "S", "Dupont")
        );
        let again = sync(&db, &program, "2025-03-11", false).unwrap();
        assert_eq!(
            (again.exported, again.imported, again.paired),
            (0, 0, 2),
            "{again:?}"
        );
    }

    fn copy_tree(from: &Path, to: &Path) {
        std::fs::create_dir_all(to).unwrap();
        for entry in std::fs::read_dir(from).unwrap() {
            let entry = entry.unwrap();
            let target = to.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy_tree(&entry.path(), &target);
            } else {
                std::fs::copy(entry.path(), target).unwrap();
            }
        }
    }

    #[test]
    fn without_a_start_date_nothing_is_read() {
        let (_swept, db) = officine("nosince");
        let fake = Fake {
            products: RefCell::new(Vec::new()),
            refuse: RefCell::new(None),
            added: RefCell::new(Vec::new()),
        };
        assert!(sync(&db, &fake, "2026-03-10", false).is_err());
    }
}
