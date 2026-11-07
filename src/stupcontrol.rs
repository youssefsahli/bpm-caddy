//! **StupControl**, le registre des stupéfiants tenu dans les classeurs
//! LibreOffice de l'officine (`{données}/{année}/{produit}.ods`, un
//! onglet par mois, une colonne Entrée, Sortie et Bilan par spécialité),
//! et le registre de BPM-Caddy : **les deux se tiennent à jour l'un
//! l'autre**.
//!
//! # Par où l'on passe
//!
//! Par les classeurs, comme StupControl lui-même : [`crate::classeur`]
//! est la traduction de son moteur, règle pour règle — les en-têtes
//! écrits à la main, la ligne TOTAUX que le mois suivant lit par adresse,
//! les bilans tirés jusqu'en bas, les reports d'une année sur l'autre par
//! lien externe ; sauvegarde avant écriture, enregistrement atomique,
//! refus tant que LibreOffice tient le fichier, journal d'audit de
//! StupControl. Un seul moteur, deux langues : un test les fait tourner
//! côte à côte sur les mêmes classeurs. Le programme n'est plus lancé.
//! Les réglages de StupControl (`config.json`) disent où sont les
//! registres et quels produits suivre.
//!
//! # Ce qui se rapproche
//!
//! Produit par produit (la correspondance est choisie par l'officine),
//! à partir d'une **date de départ** : ce qui précède n'est ni lu ni
//! écrit. Une ligne de chaque côté qui dit le même mouvement se **paire**
//! — une délivrance par son numéro d'ordonnancier, le reste par jour,
//! sens, quantité et nom, puis par jour, sens et quantité seuls quand il
//! n'y a qu'une façon d'apparier. Deux noms de patient qui diffèrent ne
//! se pairent jamais d'office : la ligne est **montrée** ([`Plan::doubts`]),
//! ni rapprochée ni écrite. Ce qui reste :
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

/// La date de départ du rapprochement, ISO, dans `stup_settings`.
pub const SINCE_KEY: &str = "stupcontrol_since";
/// « 1 » quand le poste qui sert l'officine rapproche de lui-même.
pub const AUTO_KEY: &str = "stupcontrol_auto";
/// `stupcontrol_map:<id du produit>` = la clé du produit de StupControl.
pub const MAP_PREFIX: &str = "stupcontrol_map:";
/// Le jour, ISO, où le registre a été **repris** des classeurs : depuis,
/// il est tenu ici seul et les classeurs ne sont plus rapprochés.
pub const TAKEN_KEY: &str = "stupcontrol_taken";
/// « 1 » quand, le registre repris, ce qui s'y écrit **se reporte aux
/// classeurs**, sur confirmation et réversiblement ([`push`]).
pub const PUSH_KEY: &str = "stupcontrol_push";
/// Ce que lit une reprise sans date de départ : tout l'historique.
const EVERYTHING: &str = "1900-01-01";

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
    /// Ce que dit chaque année, de la première lue à la dernière : lu
    /// seulement pour une reprise ([`Bridge::survey`]).
    #[serde(default)]
    pub years: Vec<YearRead>,
}

/// **Une année d'un produit**, telle qu'une reprise la contrôle. Les
/// en-têtes se recopient d'une année sur l'autre, avec leurs fautes :
/// ce qui ne se lit pas se dit ici, et ne s'absorbe pas dans le stock
/// reporté.
#[derive(Deserialize, Clone, Debug, PartialEq, Default)]
pub struct YearRead {
    pub year: i64,
    /// `2025/oxy.ods` ; vide quand l'année n'a pas de classeur.
    pub file: String,
    /// L'en-tête lu ; vide quand aucune colonne ne correspond.
    pub header: String,
    /// L'en-tête a été lu malgré une faute de frappe.
    pub lenient: bool,
    /// Les en-têtes du classeur, quand aucun ne correspond.
    pub headers: Vec<String>,
    /// Le stock à l'ouverture de l'année et son dernier Bilan.
    pub opening: Option<f64>,
    pub closing: Option<f64>,
    /// Entrées moins sorties de l'année.
    pub net: f64,
    /// Les lignes en quantité non entière.
    pub fractions: usize,
}

/// **Le bilan qu'une reprise retient** : la dernière cellule Bilan lue
/// dans les classeurs, celle que l'officine voit, et non le stock que
/// StupControl en recalcule — il ignore un en-tête mal recopié et
/// additionne d'une année sur l'autre ce qui ne s'y suit pas. Celui de
/// StupControl à défaut d'année lue.
pub fn bilan(p: &Product) -> Option<f64> {
    p.years.iter().rev().find_map(|y| y.closing).or(p.stock)
}

/// Le stock d'ouverture de la première année lue d'un produit.
fn first_opening(p: &Product) -> Option<f64> {
    p.years
        .iter()
        .find(|y| !y.header.is_empty())
        .and_then(|y| y.opening)
}

/// **Ce que le classeur dit d'un produit suivi**, quand plusieurs
/// produits du classeur y sont versés (des génériques, des marques
/// fondues) : leurs lignes ensemble, dans l'ordre des jours, et leurs
/// stocks additionnés. Ce qui part d'ici va au premier.
struct Merged {
    stup_id: i64,
    /// Le premier produit, les lignes et le stock de tous.
    product: Product,
    parts: Vec<Product>,
    opening: Option<f64>,
    bilan: Option<f64>,
}

fn sum_some(v: impl Iterator<Item = Option<f64>>) -> Option<f64> {
    v.fold(None, |acc, x| match (acc, x) {
        (None, x) => x,
        (Some(a), Some(b)) => Some(a + b),
        (a, None) => a,
    })
}

fn merged(stup_id: i64, parts: Vec<Product>) -> Option<Merged> {
    let first = parts.first()?.clone();
    if parts.len() == 1 {
        return Some(Merged {
            stup_id,
            opening: first_opening(&first),
            bilan: bilan(&first),
            product: first,
            parts,
        });
    }
    let mut product = first;
    product.movements = parts.iter().flat_map(|p| p.movements.clone()).collect();
    product
        .movements
        .sort_by(|a, b| (&a.date, a.year, a.month).cmp(&(&b.date, b.year, b.month)));
    product.stock = sum_some(parts.iter().map(|p| p.stock));
    product.years = Vec::new();
    let opening = parts
        .iter()
        .map(first_opening)
        .collect::<Option<Vec<f64>>>()
        .map(|v| v.iter().sum());
    Some(Merged {
        stup_id,
        opening,
        bilan: sum_some(parts.iter().map(bilan)),
        product,
        parts,
    })
}

/// Un nombre d'unités qui n'est pas entier.
fn fractional(v: f64) -> bool {
    (v - v.round()).abs() > 1e-6
}

/// **Ce qu'il faut vérifier dans les classeurs d'un produit** avant de
/// les reprendre : une année sans classeur ou sans colonne, un en-tête
/// lu malgré une faute, une année qui ne s'ouvre pas où la précédente
/// s'est fermée, une quantité ou un bilan non entiers.
pub fn traps(p: &Product) -> Vec<String> {
    use crate::strings::{decimal, trf, trn};
    let mut out = Vec::new();
    let mut last: Option<(i64, f64)> = None;
    for y in &p.years {
        if y.file.is_empty() {
            out.push(trf("stc_trap_no_file", y.year));
            continue;
        }
        if y.header.is_empty() {
            out.push(trn(
                "stc_trap_no_column",
                &[&y.file, &p.specialite, &y.headers.join(", ")],
            ));
            last = None;
            continue;
        }
        if y.lenient {
            out.push(trn("stc_trap_lenient", &[&y.file, &y.header]));
        }
        if y.fractions > 0 {
            out.push(trn("stc_trap_fractions", &[&y.file, &y.fractions]));
        }
        if let (Some((year, closed)), Some(opened)) = (last, y.opening) {
            if (closed - opened).abs() > 1e-6 {
                out.push(trn(
                    "stc_trap_break",
                    &[&y.file, &decimal(opened, 2), &year, &decimal(closed, 2)],
                ));
            }
        }
        last = y.closing.map(|c| (y.year, c));
    }
    if let Some(stock) = p.stock.filter(|v| fractional(*v)) {
        out.push(trf("stc_trap_stock", decimal(stock, 2)));
    }
    out
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
    /// Les classeurs lus qui ne portent pas l'en-tête du produit : ses
    /// lignes de l'année manquent à la lecture.
    #[serde(default)]
    pub headerless: Vec<String>,
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

/// **Ce par quoi l'on parle aux classeurs de StupControl.** Les
/// classeurs eux-mêmes ([`crate::classeur::Native`]) ; un double en
/// mémoire dans les tests.
pub trait Bridge {
    fn read(&self, since: &str) -> Result<Answer, String>;
    fn add(&self, entries: &[Entry]) -> Result<Vec<Outcome>, String>;
    /// Une lecture pour une reprise : la même, et ce que dit chaque année.
    fn survey(&self, since: &str) -> Result<Answer, String> {
        self.read(since)
    }
    /// Les classeurs qu'écrire ces lignes toucherait, pour en garder une
    /// copie avant ([`push`]). Aucun, pour un pont qui n'a pas de fichiers.
    fn files_of(&self, _entries: &[Entry]) -> Vec<std::path::PathBuf> {
        Vec::new()
    }
    /// Remettre un classeur tel que sa copie le garde, avec les mêmes
    /// précautions qu'une écriture : pas ouvert dans LibreOffice, une
    /// copie de sécurité avant, l'échange atomique.
    fn restore(&self, _file: &std::path::Path, _copy: &std::path::Path) -> Result<(), String> {
        Err(crate::strings::tr("stc_err_restore").to_owned())
    }
    /// Ce classeur peut-il être remis ? Demandé pour tous avant d'en
    /// remettre un seul.
    fn writable(&self, _file: &std::path::Path) -> Result<(), String> {
        Ok(())
    }
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
    /// (ligne de BPM-Caddy, indice de la ligne du classeur) : même jour,
    /// même sens, même quantité, mais rien ne dit que c'est le même
    /// patient. Ni rapprochées, ni écrites, ni inscrites : à trancher.
    pub doubts: Vec<(i64, usize)>,
}

/// Deux noms désignent-ils la même personne ? Les mots du plus court se
/// retrouvent tous dans le plus long (« Dupont », « DUPONT Albert »).
/// `None` quand l'un manque : rien ne le dit.
fn names_fit(a: &str, b: &str) -> Option<bool> {
    let (a, b) = (fold(a), fold(b));
    if a.is_empty() || b.is_empty() {
        return None;
    }
    let (short, long) = if a.len() <= b.len() { (a, b) } else { (b, a) };
    let long: HashSet<&str> = long.split(' ').collect();
    Some(short.split(' ').all(|w| long.contains(w)))
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
    let candidates = |o: &Ours, taken: &[bool]| -> Vec<usize> {
        theirs
            .iter()
            .enumerate()
            .filter(|(i, l)| {
                !taken[*i]
                    && their_side[*i] == Some(o.row.side)
                    && l.date.as_deref() == Some(o.row.day.as_str())
                    && same_units(l.units, o.row.units)
                    && match (ordo_number(&l.ordo), ordo_number(&o.row.ordo)) {
                        (Some(a), Some(b)) => a == b,
                        _ => true,
                    }
            })
            .map(|(i, _)| i)
            .collect()
    };
    // Le nom qui concorde d'abord, pour tout le monde : sinon la première
    // ligne prenait celle d'un autre patient, et la sienne restait.
    for o in ours {
        if paired.contains(&o.id) {
            continue;
        }
        let fits = candidates(o, &taken)
            .into_iter()
            .find(|i| names_fit(&theirs[*i].nom, &o.row.who) == Some(true));
        if let Some(i) = fits {
            taken[i] = true;
            paired.insert(o.id);
            out.pairs.push((o.id, i));
        }
    }
    // Puis ce qui ne désigne pas un patient (un fournisseur écrit
    // autrement), et ce qui était déjà rapproché : l'opérateur a retouché
    // le nom au classeur, c'est toujours la même ligne.
    for o in ours {
        if paired.contains(&o.id) || !(o.linked || o.row.side != Side::S) {
            continue;
        }
        if let Some(i) = candidates(o, &taken).first().copied() {
            taken[i] = true;
            paired.insert(o.id);
            out.pairs.push((o.id, i));
        }
    }
    // Une délivrance reste. Un nom qui manque n'apparie que s'il n'y a
    // qu'une façon de le faire ; deux noms qui diffèrent, jamais : deux
    // délivrances réelles du même jour et de la même dose se perdraient
    // l'une dans l'autre sans qu'aucun solde ne bouge.
    let open: Vec<&Ours> = ours.iter().filter(|o| !paired.contains(&o.id)).collect();
    for o in &open {
        let found = candidates(o, &taken);
        let Some(i) = found.first().copied() else {
            continue;
        };
        let rivals = open
            .iter()
            .filter(|r| {
                !r.linked
                    && r.row.side == o.row.side
                    && r.row.day == o.row.day
                    && same_units(r.row.units, o.row.units)
            })
            .count();
        taken[i] = true;
        paired.insert(o.id);
        if found.len() == 1 && rivals == 1 && names_fit(&theirs[i].nom, &o.row.who).is_none() {
            out.pairs.push((o.id, i));
        } else {
            out.doubts.push((o.id, i));
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
    /// Une reprise ([`take_over`]) plutôt qu'un rapprochement.
    pub takeover: bool,
    /// Un report des lignes d'ici vers les classeurs ([`push`]).
    pub push: bool,
    /// Ce qu'un report a trouvé au classeur sans ligne au registre : dit,
    /// jamais inscrit — le registre est tenu ici.
    pub unwritten: Vec<Refused>,
    /// Les stocks reportés des classeurs, un par produit qui en avait.
    pub openings: usize,
    /// Les lignes de BPM-Caddy absentes des classeurs, laissées telles
    /// quelles par une reprise.
    pub kept: usize,
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
    /// Les lignes laissées de côté : même jour, même quantité qu'une ligne
    /// du classeur, sans dire le même patient ([`Plan::doubts`]).
    pub doubts: Vec<Refused>,
    /// (produit, solde de BPM-Caddy, Bilan du classeur) quand ils diffèrent.
    pub gaps: Vec<(String, f64, f64)>,
    /// Les produits de StupControl qu'aucun produit suivi ne désigne.
    pub unmapped: Vec<Product>,
    /// Tous les produits de StupControl : (clé, libellé), pour choisir.
    pub products: Vec<(String, String)>,
    pub unreadable: Vec<String>,
    /// Ce qu'une reprise a relevé dans les classeurs ([`traps`]).
    pub warnings: Vec<Refused>,
    /// Les délivrances inscrites sans dossier : le nom du classeur n'est
    /// celui d'aucun dossier (une faute, un nom d'usage), ou celui de
    /// plusieurs. Inscrites quand même — la ligne est au registre —, et
    /// dites, pour que la vigilance les rattache de l'œil.
    pub unlinked: Vec<Refused>,
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
    if db.stup_setting(TAKEN_KEY).is_some() {
        return Err(crate::strings::tr("stc_err_taken").to_owned());
    }
    run(db, bridge, today, preview, false)
}

/// **Reprendre le registre des classeurs**, pour le tenir ici seul, ou
/// dire ce que cela ferait (`preview`).
///
/// Produit par produit déjà suivi et rapproché — les produits de
/// BPM-Caddy restent ceux de l'officine : un produit de StupControl sans
/// correspondance est montré, et repris une fois associé. Pour chacun :
///
/// * **le stock reporté** : le Bilan du classeur la veille du premier jour
///   repris, inscrit en inventaire nommé « report » quand le registre ne
///   le porte pas déjà — c'est ce que fait l'ouverture d'un registre ;
/// * **toutes les lignes** des classeurs depuis la date de départ (tout,
///   quand elle est vide), appariées à celles déjà au registre, avec les
///   mêmes règles et les mêmes refus qu'un rapprochement ;
/// * rien n'est écrit dans les classeurs : ce que le registre porte de
///   plus y reste.
///
/// Puis le registre se dit repris ([`TAKEN_KEY`]) et le rapprochement
/// s'arrête. Une reprise se relance sans rien doubler : ce qu'elle a
/// inscrit est lié, et le stock reporté est déjà au registre.
pub fn take_over(
    db: &Db,
    bridge: &dyn Bridge,
    today: &str,
    preview: bool,
) -> Result<Report, String> {
    let report = run(db, bridge, today, preview, true)?;
    if !preview {
        // **La première reprise reste le départ du report** : relancée,
        // elle avançait la date, et ce qui s'était écrit entre-temps sans
        // partir sortait pour toujours de ce qu'un report regarde.
        if db.stup_setting(TAKEN_KEY).is_none() {
            db.set_stup_setting(TAKEN_KEY, today)?;
        }
        db.set_stup_setting(AUTO_KEY, "")?;
    }
    Ok(report)
}

/// **Reporter aux classeurs ce qui s'est écrit ici depuis la reprise**,
/// ou dire ce que cela ferait (`preview`).
///
/// Les lignes du registre datées depuis la reprise et jamais parties
/// sont rapprochées des classeurs comme au rapprochement — une ligne
/// tapée aussi dans LibreOffice se paire, elle ne s'écrit pas deux fois
/// —, puis écrites. Rien ne vient des classeurs : ce qu'ils portent de
/// plus est dit ([`Report::unwritten`]), le registre étant tenu ici.
///
/// **Réversible** : avant d'écrire, chaque classeur touché est copié dans
/// `undo_root`, avec ce qui a été écrit ; [`undo_push`] le remet.
pub fn push(
    db: &Db,
    bridge: &dyn Bridge,
    today: &str,
    preview: bool,
    undo_root: &std::path::Path,
) -> Result<Report, String> {
    if db.stup_setting(TAKEN_KEY).is_none() {
        return Err(crate::strings::tr("stc_err_not_taken").to_owned());
    }
    let _lock = if preview {
        None
    } else {
        Some(WriteLock::take(undo_root)?)
    };
    run_with(db, bridge, today, preview, Mode::Push(undo_root))
}

/// **Un poste à la fois écrit aux classeurs.** Les verrous de `classeur`
/// ne valent que dans un programme, et chaque ligne relit son fichier :
/// deux postes lancés ensemble ajoutaient chacun toutes les lignes. Le
/// dossier des reports est à côté de la base, donc commun aux postes qui
/// la partagent. Un verrou laissé par un poste tombé s'efface à son âge.
struct WriteLock(std::path::PathBuf);

impl WriteLock {
    const STALE: std::time::Duration = std::time::Duration::from_secs(30 * 60);

    fn take(root: &std::path::Path) -> Result<WriteLock, String> {
        let failed = || crate::strings::trf("stc_err_undo_copy", root.display());
        std::fs::create_dir_all(root).map_err(|_| failed())?;
        let path = root.join("report.lock");
        let old = std::fs::metadata(&path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.elapsed().ok())
            .is_some_and(|age| age > Self::STALE);
        if old {
            let _ = std::fs::remove_file(&path);
        }
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(_) => Ok(WriteLock(path)),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                Err(crate::strings::tr("stc_err_busy").to_owned())
            }
            Err(_) => Err(failed()),
        }
    }
}

impl Drop for WriteLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// Un report écrit : quand, les lignes parties, et pour chaque classeur
/// la copie d'avant et l'empreinte d'après.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Pushed {
    pub at: String,
    pub moves: Vec<i64>,
    pub files: Vec<PushedFile>,
    /// Le jour où il a été défait ; vide tant qu'il tient.
    #[serde(default)]
    pub undone: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct PushedFile {
    pub file: String,
    pub copy: String,
    /// L'empreinte du classeur juste après le report : s'il a changé
    /// depuis, le remettre effacerait ce qu'on y a écrit après.
    pub after: String,
}

/// Une empreinte stable d'un fichier (FNV-1a, 64 bits, et sa taille) :
/// elle doit se relire pareil d'une version du programme à l'autre.
fn fingerprint(path: &std::path::Path) -> String {
    let Ok(bytes) = std::fs::read(path) else {
        return String::new();
    };
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in &bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{}-{h:016x}", bytes.len())
}

const MANIFEST: &str = "report.json";

/// Ce qu'un report garde pendant qu'il écrit.
struct Journal {
    dir: std::path::PathBuf,
    pushed: Pushed,
    files: Vec<(std::path::PathBuf, std::path::PathBuf)>,
}

impl Journal {
    fn open(
        root: &std::path::Path,
        today: &str,
        files: &[std::path::PathBuf],
    ) -> Result<Journal, String> {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default();
        let dir = root.join(format!("{stamp:024}"));
        let failed = || crate::strings::trf("stc_err_undo_copy", root.display());
        std::fs::create_dir_all(&dir).map_err(|_| failed())?;
        let mut kept = Vec::new();
        let mut seen = HashSet::new();
        for (n, f) in files.iter().enumerate() {
            if !seen.insert(f.clone()) || !f.exists() {
                continue;
            }
            let copy = dir.join(format!("{n}.ods"));
            if std::fs::copy(f, &copy).is_err() {
                let _ = std::fs::remove_dir_all(&dir);
                return Err(failed());
            }
            kept.push((f.clone(), copy));
        }
        Ok(Journal {
            dir,
            pushed: Pushed {
                at: today.to_owned(),
                ..Pushed::default()
            },
            files: kept,
        })
    }

    /// Le report a écrit ce qu'il a pu : son manifeste, ou rien s'il n'a
    /// rien écrit.
    fn close(mut self) -> Result<(), String> {
        if self.pushed.moves.is_empty() {
            let _ = std::fs::remove_dir_all(&self.dir);
            return Ok(());
        }
        // Seulement ceux qu'il a écrits : un classeur copié puis refusé (tenu
        // par LibreOffice) n'a rien à défaire, et ce qu'on y tape ensuite
        // ne doit pas bloquer l'annulation des autres.
        self.pushed.files = self
            .files
            .iter()
            .filter(|(f, c)| fingerprint(f) != fingerprint(c))
            .map(|(f, c)| PushedFile {
                file: f.to_string_lossy().into_owned(),
                copy: c.to_string_lossy().into_owned(),
                after: fingerprint(f),
            })
            .collect();
        let text = serde_json::to_string_pretty(&self.pushed).map_err(|e| e.to_string())?;
        std::fs::write(self.dir.join(MANIFEST), text)
            .map_err(|_| crate::strings::trf("stc_err_undo_copy", self.dir.display()))
    }
}

/// **Le dernier report qui tient**, le plus récent d'abord : c'est lui
/// qu'on défait, puis le précédent.
pub fn last_push(undo_root: &std::path::Path) -> Option<(std::path::PathBuf, Pushed)> {
    let mut dirs: Vec<std::path::PathBuf> = std::fs::read_dir(undo_root)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.join(MANIFEST).is_file())
        .collect();
    dirs.sort();
    dirs.into_iter().rev().find_map(|d| {
        let text = std::fs::read_to_string(d.join(MANIFEST)).ok()?;
        let p: Pushed = serde_json::from_str(&text).ok()?;
        p.undone.is_empty().then_some((d, p))
    })
}

/// **Défaire le dernier report** : chaque classeur remis tel qu'il était
/// avant, et les lignes reparties « à reporter ».
///
/// Refusé en entier si un classeur a changé depuis — une ligne tapée
/// dans LibreOffice, un autre report : le remettre l'effacerait. Rien
/// n'est alors touché, et le classeur en cause est nommé. Rend le nombre
/// de lignes reparties.
pub fn undo_push(
    db: &Db,
    bridge: &dyn Bridge,
    today: &str,
    undo_root: &std::path::Path,
) -> Result<usize, String> {
    let _lock = WriteLock::take(undo_root)?;
    let Some((dir, mut pushed)) = last_push(undo_root) else {
        return Err(crate::strings::tr("stc_err_nothing_to_undo").to_owned());
    };
    // **Une annulation interrompue se reprend** : un classeur déjà égal à
    // sa copie est remis. Sans cela le second essai le voyait « changé
    // depuis », et le report restait à moitié défait pour toujours.
    let mut left = Vec::new();
    for f in &pushed.files {
        let (file, copy) = (std::path::Path::new(&f.file), std::path::Path::new(&f.copy));
        if !copy.is_file() {
            return Err(crate::strings::trf("stc_err_undo_copy", &f.copy));
        }
        let now = fingerprint(file);
        if now == fingerprint(copy) {
            continue;
        }
        if now != f.after {
            return Err(crate::strings::trf("stc_err_changed_since", &f.file));
        }
        left.push((file, copy));
    }
    // Tous libres avant d'en remettre un seul.
    for (file, _) in &left {
        bridge.writable(file)?;
    }
    for (file, copy) in &left {
        bridge.restore(file, copy)?;
    }
    for id in &pushed.moves {
        db.drop_stup_link(*id, "export")?;
    }
    pushed.undone = today.to_owned();
    let text = serde_json::to_string_pretty(&pushed).map_err(|e| e.to_string())?;
    std::fs::write(dir.join(MANIFEST), text).map_err(|e| e.to_string())?;
    Ok(pushed.moves.len())
}

/// Le solde d'une suite de lignes du registre, dans l'ordre du registre.
fn balance_of(moves: &[&StupMove]) -> f64 {
    let mut sorted: Vec<&StupMove> = moves.to_vec();
    sorted.sort_by(|a, b| a.happened_on.cmp(&b.happened_on).then(a.id.cmp(&b.id)));
    let lines: Vec<crate::ordonnancier::Move> = sorted
        .iter()
        .map(|m| crate::ordonnancier::Move {
            kind: Kind::from_key(&m.kind),
            quantity: m.quantity,
            day: &m.happened_on,
            seq: m.id,
            cancels: m.cancels,
            expected: m.expected,
        })
        .collect();
    crate::ordonnancier::balance(&lines).stock
}

/// **Le stock d'un classeur la veille de `start`** : son Bilan, moins ce
/// que les lignes lues depuis y ont fait.
pub fn balance_before(stock: f64, lines: &[Line], start: &str) -> f64 {
    lines
        .iter()
        .filter(|l| l.date.as_deref().is_none_or(|d| d >= start))
        .fold(stock, |b, l| match Side::of(&l.kind) {
            Some(Side::E) => b - l.units,
            Some(Side::S) => b + l.units,
            None => b,
        })
}

/// Ce que fait un passage sur les classeurs.
#[derive(Clone, Copy)]
enum Mode<'a> {
    Sync,
    Takeover,
    /// Le report, et où garder de quoi le défaire.
    Push(&'a std::path::Path),
}

fn run(
    db: &Db,
    bridge: &dyn Bridge,
    today: &str,
    preview: bool,
    takeover: bool,
) -> Result<Report, String> {
    run_with(
        db,
        bridge,
        today,
        preview,
        if takeover { Mode::Takeover } else { Mode::Sync },
    )
}

fn run_with(
    db: &Db,
    bridge: &dyn Bridge,
    today: &str,
    preview: bool,
    mode: Mode,
) -> Result<Report, String> {
    let takeover = matches!(mode, Mode::Takeover);
    let undo_root = match mode {
        Mode::Push(root) => Some(root),
        _ => None,
    };
    let push = undo_root.is_some();
    // Un report part de la reprise : ce qui la précède est aux classeurs.
    let since = if push {
        db.stup_setting(TAKEN_KEY).unwrap_or_default()
    } else {
        db.stup_setting(SINCE_KEY).unwrap_or_default()
    };
    let since = since.trim().to_owned();
    if since.is_empty() && !takeover {
        return Err(crate::strings::tr("stc_err_no_since").to_owned());
    }
    let since = if since.is_empty() {
        EVERYTHING.to_owned()
    } else {
        since
    };
    let answer = if takeover {
        bridge.survey(&since)?
    } else {
        bridge.read(&since)?
    };
    // **Un classeur illisible n'est pas un classeur vide.** Ses lignes
    // paraissaient perdues, et celles d'ici repartaient s'y écrire. Une
    // reprise, elle, relit l'en-tête mal orthographié et dit le reste.
    if !takeover {
        if let Some(file) = answer.unreadable.iter().chain(&answer.headerless).next() {
            return Err(crate::strings::trf("stc_err_cannot_read", file));
        }
    }
    let mut report = Report {
        preview,
        takeover,
        push,
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
    // Chaque produit suivi, et ce que le classeur en dit : les produits
    // qu'il désigne, le premier d'abord.
    let groups: Vec<Merged> = {
        let mut ids: Vec<i64> = Vec::new();
        for p in &answer.products {
            if let Some(id) = by_key.get(p.key.as_str()) {
                if !ids.contains(id) {
                    ids.push(*id);
                }
            }
        }
        ids.into_iter()
            .filter_map(|id| {
                let parts: Vec<Product> = mapping
                    .iter()
                    .filter(|(i, _)| *i == id)
                    .filter_map(|(_, k)| answer.products.iter().find(|p| &p.key == k))
                    .cloned()
                    .collect();
                merged(id, parts)
            })
            .collect()
    };
    for group in &groups {
        let stup_id = group.stup_id;
        let theirs = &group.product;
        let Some(product) = products.get(&stup_id) else {
            continue;
        };
        for part in &group.parts {
            for said in traps(part) {
                report.warnings.push(Refused {
                    product: if group.parts.len() > 1 {
                        format!("{} ({})", product.label, shown_label(part))
                    } else {
                        product.label.clone()
                    },
                    line: said,
                    reason: String::new(),
                });
            }
        }
        let all = db.stup_moves(stup_id)?;
        let by_id: HashMap<i64, &StupMove> = all.iter().map(|m| (m.id, m)).collect();
        // Le premier jour repris : la date de départ, ou la première ligne
        // datée des classeurs quand une reprise prend tout. Ce qui précède
        // au registre est compté dans le stock reporté.
        let start = if since == EVERYTHING {
            theirs.movements.iter().filter_map(|l| l.date.clone()).min()
        } else {
            Some(since.clone())
        };
        let from = if takeover {
            start.clone().unwrap_or_else(|| since.clone())
        } else {
            since.clone()
        };
        let ours: Vec<Ours> = all
            .iter()
            .filter(|m| m.happened_on.as_str() >= from.as_str())
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
        for (id, i) in &decided.doubts {
            if let Some(m) = by_id.get(id) {
                report.doubts.push(Refused {
                    product: product.label.clone(),
                    line: move_said(m),
                    reason: crate::strings::trf("stc_doubt", theirs.movements[*i].nom.trim()),
                });
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
        // Le stock reporté : celui du classeur la veille du premier jour
        // repris, quand le registre ne le porte pas déjà.
        if takeover {
            if let (Some(start), Some(stock)) = (start.clone(), group.bilan) {
                let before: Vec<&StupMove> = all
                    .iter()
                    .filter(|m| m.happened_on.as_str() < start.as_str())
                    .collect();
                let ours_then = balance_of(&before);
                // Tout l'historique : le stock d'ouverture de la première
                // année, et la suite des lignes. Une année perdue ou un
                // bilan faux se lit alors en écart au bout, au lieu de se
                // fondre dans le report.
                let theirs_then = match group.opening {
                    Some(o) if since == EVERYTHING => o,
                    _ => balance_before(stock, &theirs.movements, &start),
                };
                let day = crate::date::add_days(&start, -1).unwrap_or_else(|| start.clone());
                if !same_units(ours_then, theirs_then) {
                    let said = || {
                        format!(
                            "{} — {}",
                            crate::db::format_french_date(&day),
                            crate::strings::decimal(theirs_then, 2)
                        )
                    };
                    if theirs_then < 0.0 {
                        report.refused.push(Refused {
                            product: product.label.clone(),
                            line: said(),
                            reason: crate::strings::tr("stc_refused_opening").to_owned(),
                        });
                    } else if preview {
                        report.openings += 1;
                    } else {
                        let m = StupMove {
                            id: 0,
                            stup_id,
                            kind: Kind::Inventaire.as_key().to_owned(),
                            happened_on: day.clone(),
                            quantity: theirs_then,
                            ordo_year: 0,
                            ordo_no: 0,
                            patient_id: 0,
                            prescriber: String::new(),
                            supplier: String::new(),
                            reference: String::new(),
                            lot: String::new(),
                            expiry: String::new(),
                            expected: ours_then,
                            operator: crate::strings::tr("stc_operator").to_owned(),
                            remark: crate::strings::trf(
                                "stc_opening_remark",
                                crate::db::format_french_date(&day),
                            ),
                            cancels: 0,
                        };
                        match db.add_stup_move_linked(&m, "report", today) {
                            Ok(_) => report.openings += 1,
                            Err(e) => report.refused.push(Refused {
                                product: product.label.clone(),
                                line: said(),
                                reason: e,
                            }),
                        }
                    }
                }
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
            if push {
                report.unwritten.push(refuse(String::new()));
                continue;
            }
            let (day, quantity, taken) = match import_of(l) {
                Ok(t) => t,
                Err(r) => {
                    report.refused.push(refuse(r.said()));
                    continue;
                }
            };
            let mut unlinked = false;
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
                    unlinked = m.patient_id == 0;
                }
                Taken::Perte { remark } => {
                    m.kind = Kind::Perte.as_key().to_owned();
                    m.remark = crate::strings::trf("stc_remark_said", remark);
                }
            }
            let said_unlinked = || refuse(String::new());
            if preview {
                report.imported += 1;
                if unlinked {
                    report.unlinked.push(said_unlinked());
                }
                continue;
            }
            match db.add_stup_move_linked(&m, "import", today) {
                Ok(_) => {
                    report.imported += 1;
                    if unlinked {
                        report.unlinked.push(said_unlinked());
                    }
                }
                Err(e) => report.refused.push(refuse(e)),
            }
        }
        if takeover {
            report.kept += decided.export.len();
            continue;
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
        // Un report garde d'abord de quoi se défaire.
        let mut journal = match undo_root {
            Some(root) => Some(Journal::open(root, today, &bridge.files_of(&sent))?),
            None => None,
        };
        let outcomes = match bridge.add(&sent) {
            Ok(o) => o,
            Err(e) => {
                if let Some(j) = journal {
                    let _ = j.close();
                }
                return Err(e);
            }
        };
        // Le classeur est écrit : quoi qu'il arrive au lien, le journal se
        // ferme, sinon ces lignes ne se déferaient plus.
        let mut link_failed = None;
        for ((id, entry, effect), outcome) in entries.iter().zip(outcomes.iter()) {
            let stup_id = by_key.get(entry.key.as_str()).copied().unwrap_or(0);
            if outcome.ok {
                if let Some(j) = &mut journal {
                    j.pushed.moves.push(*id);
                }
                if let Err(e) = db.add_stup_link(*id, "export", today) {
                    link_failed.get_or_insert(e);
                }
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
        if let Some(Err(said)) = journal.map(Journal::close) {
            report.warnings.push(Refused {
                product: String::new(),
                line: said,
                reason: String::new(),
            });
        }
        if let Some(e) = link_failed {
            return Err(e);
        }
    }
    // Les soldes, après ce qui vient d'être fait. Un aperçu compare ce qui
    // est, sans projeter ce qu'il ferait.
    let standing: HashMap<i64, f64> = db
        .stup_summary()?
        .into_iter()
        .map(|s| (s.product.id, s.stock))
        .collect();
    for group in &groups {
        let stup_id = group.stup_id;
        let stock = if takeover {
            group.bilan
        } else {
            group.product.stock
        };
        let (Some(ours), Some(stock)) = (standing.get(&stup_id), stock) else {
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

    /// **Le nom qui concorde passe d'abord** : Dupont, premier venu, prenait
    /// la ligne de Martin, et le classeur montrait Martin deux fois.
    #[test]
    fn a_same_dose_the_same_day_pairs_with_its_own_patient() {
        let mine = [
            ours(1, Side::S, "2026-03-03", 7.0, "Dupont", "5", false),
            ours(2, Side::S, "2026-03-03", 7.0, "Martin", "6", false),
        ];
        let theirs = [line("S", "2026-03-03", 7.0, "MARTIN Paul", "", "")];
        let p = plan(&mine, &theirs);
        assert_eq!(p.pairs, vec![(2, 0)]);
        assert_eq!(p.export, vec![1]);
        assert!(p.doubts.is_empty() && p.import.is_empty());
    }

    /// **Deux noms différents ne se rapprochent pas d'office** : ce sont
    /// peut-être deux délivrances, et aucun solde ne le dirait. Ni écrit,
    /// ni inscrit : montré.
    #[test]
    fn two_different_patients_the_same_day_are_shown_not_merged() {
        let mine = [ours(1, Side::S, "2026-03-03", 7.0, "Dupont", "5", false)];
        let theirs = [line("S", "2026-03-03", 7.0, "DURAND", "", "")];
        let p = plan(&mine, &theirs);
        assert_eq!(p.doubts, vec![(1, 0)]);
        assert!(p.pairs.is_empty() && p.export.is_empty() && p.import.is_empty());
        // « Martine » n'est pas « Martin ».
        assert_eq!(names_fit("Martin", "MARTINE"), Some(false));
        assert_eq!(names_fit("Dupont", "DUPONT Albert"), Some(true));
        assert_eq!(names_fit("", "DUPONT"), None);
        // Une ligne déjà rapprochée reste elle-même, nom retouché ou non.
        let kept = [ours(1, Side::S, "2026-03-03", 7.0, "Dupont", "5", true)];
        assert_eq!(plan(&kept, &theirs).pairs, vec![(1, 0)]);
    }

    /// Un nom manquant n'apparie que s'il n'y a qu'une façon de le faire.
    #[test]
    fn a_line_without_a_name_pairs_only_when_it_is_the_only_way() {
        let mine = [ours(1, Side::S, "2026-03-03", 7.0, "", "5", false)];
        let one = [line("S", "2026-03-03", 7.0, "DURAND", "", "")];
        assert_eq!(plan(&mine, &one).pairs, vec![(1, 0)]);
        let two = [
            line("S", "2026-03-03", 7.0, "DURAND", "", ""),
            line("S", "2026-03-03", 7.0, "LEROY", "", ""),
        ];
        let p = plan(&mine, &two);
        assert!(p.pairs.is_empty());
        assert_eq!(p.doubts, vec![(1, 0)]);
        assert_eq!(p.import, vec![1]);
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
            years: Vec::new(),
        }
    }

    fn year(y: i64, opening: f64, closing: f64) -> YearRead {
        YearRead {
            year: y,
            file: format!("{y}/oxy.ods"),
            header: "Oxycodone 10".into(),
            opening: Some(opening),
            closing: Some(closing),
            ..Default::default()
        }
    }

    /// **Les pièges des classeurs se disent** : une année sans classeur ou
    /// sans colonne, un en-tête lu malgré sa faute, une année qui ne
    /// s'ouvre pas où la précédente s'est fermée, un bilan non entier.
    #[test]
    fn what_the_workbooks_get_wrong_is_said_year_by_year() {
        let mut p = product("oxy.ods|oxycodone 10", "Oxycodone 10", "Oxycodone 10");
        assert!(traps(&p).is_empty());
        let mut lenient = year(2024, 41.0, 30.0);
        lenient.lenient = true;
        lenient.header = "Oxycodonne 10".into();
        lenient.fractions = 2;
        let missing = YearRead {
            year: 2025,
            file: "2025/oxy.ods".into(),
            headers: vec!["Oxycodone Biogaran 10".into()],
            ..Default::default()
        };
        let none = YearRead {
            year: 2026,
            ..Default::default()
        };
        p.years = vec![
            year(2022, 10.0, 20.0),
            year(2023, 20.0, 40.0),
            lenient,
            missing,
            none,
            year(2027, 47.6, 47.6),
        ];
        p.stock = Some(47.6);
        let said = traps(&p);
        assert_eq!(said.len(), 6, "{said:#?}");
        assert!(said[0].contains("Oxycodonne 10"));
        assert!(said[1].contains("2 ligne"));
        assert!(
            said[2].contains("41") && said[2].contains("40"),
            "{}",
            said[2]
        );
        assert!(said[3].contains("Oxycodone Biogaran 10"));
        assert!(said[4].contains("2026"));
        assert!(said[5].contains("47,6"), "{}", said[5]);
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
                headerless: Vec::new(),
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

    /// **Une reprise** inscrit tout l'historique des classeurs sous les
    /// produits suivis qui les désignent, avec le stock reporté la veille
    /// du premier jour, n'écrit rien aux classeurs, arrête le
    /// rapprochement, et se relance sans rien doubler.
    #[test]
    fn a_takeover_carries_the_stock_and_the_history_into_the_followed_products() {
        let (_swept, db) = officine("take");
        let stup = db
            .add_stupefiant(0, "Méthadone 10 mg", "gélule", 0.0)
            .unwrap();
        db.set_stupcontrol_map(stup, KEY).unwrap();
        let dupont = db.add_patient("Dupont", "Albert", "1950-01-01").unwrap();
        let mut sortie = mv(0, Kind::Sortie, "2026-03-03", 7.0);
        sortie.stup_id = stup;
        sortie.ordo_no = 18412;
        sortie.patient_id = dupont;
        db.add_stup_move(&sortie).unwrap();

        let mut p = product(KEY, "Méthadone 10 mg", "Méthadone 10 mg");
        p.stock = Some(30.0);
        p.movements = vec![
            line("E", "2026-03-02", 28.0, "Cerp", "", ""),
            line("S", "2026-03-03", 7.0, "DUPONT", "Albert", "18412"),
            line("S", "2026-03-04", 5.0, "MARTIN", "Paul", "18500"),
        ];
        let mut other = product("skenan.ods|skenan 10", "Skenan 10 mg", "Skenan 10");
        other.stock = Some(12.0);
        other.movements = vec![line("E", "2026-03-02", 12.0, "Cerp", "", "")];
        let fake = Fake {
            products: RefCell::new(vec![p, other]),
            refuse: RefCell::new(None),
            added: RefCell::new(Vec::new()),
        };

        let seen = take_over(&db, &fake, "2026-03-10", true).unwrap();
        assert!(seen.takeover && seen.preview);
        assert_eq!(
            (seen.openings, seen.imported, seen.paired, seen.kept),
            (1, 2, 1, 0)
        );
        assert!(
            db.stup_setting(TAKEN_KEY).is_none(),
            "un aperçu n'écrit rien"
        );
        assert_eq!(db.stup_moves(stup).unwrap().len(), 1);

        let done = take_over(&db, &fake, "2026-03-10", false).unwrap();
        assert_eq!((done.openings, done.imported, done.paired), (1, 2, 1));
        assert!(done.refused.is_empty(), "{:?}", done.refused);
        assert!(done.gaps.is_empty(), "{:?}", done.gaps);
        assert!(
            fake.added.borrow().is_empty(),
            "rien n'est écrit aux classeurs"
        );
        // Le produit sans correspondance est dit, et rien n'est créé pour lui.
        assert_eq!(done.unmapped.len(), 1);
        assert_eq!(db.stupefiants().unwrap().len(), 1);
        let moves = db.stup_moves(stup).unwrap();
        let opening = moves
            .iter()
            .find(|m| m.kind == Kind::Inventaire.as_key())
            .unwrap();
        assert_eq!(
            (opening.happened_on.as_str(), opening.quantity),
            ("2026-03-01", 14.0)
        );
        assert!(opening.remark.contains("01/03/2026"));
        let standing = db.stup_summary().unwrap();
        assert_eq!(standing[0].stock, 30.0);

        assert_eq!(db.stup_setting(TAKEN_KEY).as_deref(), Some("2026-03-10"));
        assert!(sync(&db, &fake, "2026-03-11", false).is_err());

        let again = take_over(&db, &fake, "2026-03-11", false).unwrap();
        assert!(again.warnings.is_empty());
        assert_eq!((again.openings, again.imported, again.paired), (0, 0, 3));
        assert_eq!(db.stup_moves(stup).unwrap().len(), moves.len());
    }

    /// **Tout l'historique** part du stock d'ouverture de la première
    /// année : un bilan faux plus loin se lit en écart, au lieu de se
    /// fondre dans le stock reporté.
    #[test]
    fn a_whole_takeover_starts_from_the_first_year_and_says_the_gap() {
        let (_swept, db) = officine("whole");
        let stup = db.add_stupefiant(0, "Oxycodone 10", "gélule", 0.0).unwrap();
        db.set_stupcontrol_map(stup, KEY).unwrap();
        let mut p = product(KEY, "Oxycodone 10", "Oxycodone 10");
        p.movements = vec![
            line("E", "2025-01-10", 28.0, "Cerp", "", ""),
            line("E", "2026-01-10", 28.0, "Cerp", "", ""),
        ];
        // 2025 s'ouvre à 20 et se ferme à 48 ; 2026 repart de 47,6.
        p.years = vec![year(2025, 20.0, 48.0), year(2026, 47.6, 75.6)];
        p.stock = Some(75.6);
        let fake = Fake {
            products: RefCell::new(vec![p]),
            refuse: RefCell::new(None),
            added: RefCell::new(Vec::new()),
        };
        let done = take_over(&db, &fake, "2026-03-10", false).unwrap();
        assert_eq!((done.openings, done.imported), (1, 2));
        let opening = db
            .stup_moves(stup)
            .unwrap()
            .into_iter()
            .find(|m| m.kind == Kind::Inventaire.as_key())
            .unwrap();
        assert_eq!(
            (opening.happened_on.as_str(), opening.quantity),
            ("2025-01-09", 20.0)
        );
        assert_eq!(done.gaps, vec![("Oxycodone 10".to_owned(), 76.0, 75.6)]);
        assert_eq!(done.warnings.len(), 2, "{:?}", done.warnings);
    }

    /// Un classeur qui s'ouvre en négatif ne donne pas de stock reporté :
    /// il est refusé et dit, l'aperçu comme la reprise ; une casse
    /// devient une perte.
    #[test]
    fn a_negative_opening_is_refused_and_a_breakage_is_a_loss() {
        let (_swept, db) = officine("negative");
        let stup = db.add_stupefiant(0, "Oxycodone 10", "gélule", 0.0).unwrap();
        db.set_stupcontrol_map(stup, KEY).unwrap();
        let mut p = product(KEY, "Oxycodone 10", "Oxycodone 10");
        p.movements = vec![
            line("E", "2025-01-10", 28.0, "Cerp", "", ""),
            line("S", "2025-01-12", 1.0, "Casse", "", ""),
        ];
        p.years = vec![year(2025, -5.0, 22.0)];
        p.stock = Some(22.0);
        let fake = Fake {
            products: RefCell::new(vec![p]),
            refuse: RefCell::new(None),
            added: RefCell::new(Vec::new()),
        };
        let seen = take_over(&db, &fake, "2026-03-10", true).unwrap();
        assert_eq!((seen.openings, seen.imported), (0, 2));
        assert_eq!(seen.refused.len(), 1, "{:?}", seen.refused);
        assert!(
            db.stup_moves(stup).unwrap().is_empty(),
            "an aperçu writes nothing"
        );
        let done = take_over(&db, &fake, "2026-03-10", false).unwrap();
        assert_eq!((done.openings, done.imported), (0, 2));
        assert_eq!(
            done.refused[0].reason,
            crate::strings::tr("stc_refused_opening")
        );
        let moves = db.stup_moves(stup).unwrap();
        assert!(moves.iter().all(|m| m.kind != Kind::Inventaire.as_key()));
        assert!(moves.iter().any(|m| m.kind == Kind::Perte.as_key()));
        assert!(
            fake.added.borrow().is_empty(),
            "a takeover writes no workbook"
        );
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

    /// Plusieurs produits du classeur versés dans un seul : chacun ne
    /// l'est qu'une fois, la première clé se change sans perdre les
    /// autres.
    #[test]
    fn several_spreadsheet_products_pour_into_one_followed_product() {
        let (_swept, db) = officine("pour");
        let a = db
            .add_stupefiant(0, "Méthadone 10 mg", "gélule", 0.0)
            .unwrap();
        let b = db.add_stupefiant(0, "Autre", "gélule", 0.0).unwrap();
        db.set_stupcontrol_map(a, KEY).unwrap();
        db.add_stupcontrol_key(a, EG).unwrap();
        db.add_stupcontrol_key(a, EG).unwrap();
        assert_eq!(db.stupcontrol_keys(a).unwrap(), vec![KEY, EG]);
        assert!(db.add_stupcontrol_key(b, EG).is_err());
        assert!(db.set_stupcontrol_map(b, EG).is_err());
        db.set_stupcontrol_map(a, "").unwrap();
        assert_eq!(db.stupcontrol_keys(a).unwrap(), vec![EG]);
        // La première change : EG l'était devenue.
        db.set_stupcontrol_map(a, KEY).unwrap();
        assert_eq!(db.stupcontrol_keys(a).unwrap(), vec![KEY]);
        db.add_stupcontrol_key(a, EG).unwrap();
        db.remove_stupcontrol_key(a, EG).unwrap();
        assert_eq!(db.stupcontrol_keys(a).unwrap(), vec![KEY]);
    }

    /// **Un nom mal écrit au classeur** : la délivrance est inscrite, sans
    /// dossier, et dite — jamais rattachée au dossier qui lui ressemble.
    #[test]
    fn a_misspelt_name_is_inscribed_without_a_file_and_said() {
        let (_swept, db) = officine("unlinked");
        let dupont = db.add_patient("Dupont", "Albert", "1950-01-01").unwrap();
        let stup = db
            .add_stupefiant(0, "Skenan LP 30 mg", "gélule", 0.0)
            .unwrap();
        db.set_stupcontrol_map(stup, KEY).unwrap();
        let mut p = product(KEY, "Skenan LP 30 mg", "Skenan LP 30 mg");
        p.movements = vec![
            line("E", "2025-01-02", 28.0, "Cerp", "", ""),
            line("S", "2025-01-10", 7.0, "DUPONT", "Albert", "18412"),
            line("S", "2025-01-20", 7.0, "Dupond", "Albert", "18413"),
        ];
        p.years = vec![year(2025, 0.0, 14.0)];
        p.stock = Some(14.0);
        let fake = Fake {
            products: RefCell::new(vec![p]),
            refuse: RefCell::new(None),
            added: RefCell::new(Vec::new()),
        };
        let seen = take_over(&db, &fake, "2026-03-10", true).unwrap();
        assert_eq!(seen.unlinked.len(), 1, "{:?}", seen.unlinked);
        let done = take_over(&db, &fake, "2026-03-10", false).unwrap();
        assert_eq!(done.imported, 3);
        assert_eq!(done.unlinked.len(), 1);
        assert!(
            done.unlinked[0].line.contains("Dupond"),
            "{:?}",
            done.unlinked
        );
        let files: Vec<i64> = db
            .stup_moves(stup)
            .unwrap()
            .iter()
            .filter(|m| m.kind == Kind::Sortie.as_key())
            .map(|m| m.patient_id)
            .collect();
        assert_eq!(files, vec![dupont, 0]);
    }

    const EG: &str = "methadone-eg.ods|méthadone eg 10 mg";

    /// **Deux marques, un produit suivi** : la reprise inscrit les lignes
    /// des deux, reporte la somme de leurs stocks d'ouverture et compare
    /// au bout la somme de leurs bilans ; chaque marque dit ses pièges.
    #[test]
    fn two_brands_taken_over_into_one_product_add_up() {
        let (_swept, db) = officine("brands");
        let stup = db
            .add_stupefiant(0, "Méthadone 10 mg", "gélule", 0.0)
            .unwrap();
        db.set_stupcontrol_map(stup, KEY).unwrap();
        db.add_stupcontrol_key(stup, EG).unwrap();
        let mut a = product(KEY, "Méthadone 10 mg", "Méthadone 10 mg");
        a.movements = vec![line("E", "2025-02-10", 28.0, "Cerp", "", "")];
        a.years = vec![year(2025, 10.0, 38.0)];
        a.stock = Some(38.0);
        let mut b = product(EG, "Méthadone EG 10 mg", "Méthadone EG 10 mg");
        b.movements = vec![
            line("E", "2025-01-05", 14.0, "OCP", "", ""),
            line("E", "2025-03-01", 1.5, "OCP", "", ""),
        ];
        b.years = vec![year(2025, 4.0, 19.5)];
        b.stock = Some(19.5);
        let fake = Fake {
            products: RefCell::new(vec![a, b]),
            refuse: RefCell::new(None),
            added: RefCell::new(Vec::new()),
        };
        let done = take_over(&db, &fake, "2026-03-10", false).unwrap();
        assert_eq!((done.openings, done.imported), (1, 3));
        assert!(done.unmapped.is_empty());
        let moves = db.stup_moves(stup).unwrap();
        let opening = moves
            .iter()
            .find(|m| m.kind == Kind::Inventaire.as_key())
            .unwrap();
        assert_eq!(
            (opening.happened_on.as_str(), opening.quantity),
            ("2025-01-04", 14.0)
        );
        assert!(done.gaps.is_empty(), "{:?}", done.gaps);
        assert!(
            done.warnings
                .iter()
                .any(|w| w.product.contains("Méthadone EG 10 mg")),
            "{:?}",
            done.warnings
        );
    }

    /// Ce qui part d'ici va au premier produit désigné, et le solde se
    /// compare à la somme des stocks.
    #[test]
    fn a_line_from_here_goes_to_the_first_brand() {
        let (_swept, db) = officine("first-brand");
        let stup = db
            .add_stupefiant(0, "Méthadone 10 mg", "gélule", 0.0)
            .unwrap();
        db.set_stupcontrol_map(stup, KEY).unwrap();
        db.add_stupcontrol_key(stup, EG).unwrap();
        db.set_stup_setting(SINCE_KEY, "2026-03-01").unwrap();
        let mut entree = mv(0, Kind::Entree, "2026-03-02", 28.0);
        entree.stup_id = stup;
        db.add_stup_move(&entree).unwrap();
        let mut a = product(KEY, "Méthadone 10 mg", "Méthadone 10 mg");
        a.stock = Some(0.0);
        let mut b = product(EG, "Méthadone EG 10 mg", "Méthadone EG 10 mg");
        b.stock = Some(0.0);
        let fake = Fake {
            products: RefCell::new(vec![b, a]),
            refuse: RefCell::new(None),
            added: RefCell::new(Vec::new()),
        };
        let done = sync(&db, &fake, "2026-03-10", false).unwrap();
        assert_eq!(done.exported, 1);
        assert_eq!(fake.added.borrow()[0].key, KEY);
        assert!(done.gaps.is_empty(), "{:?}", done.gaps);
    }

    /// **De bout en bout, sur de vrais classeurs** : une délivrance de
    /// BPM-Caddy écrite dans le classeur, relue, et un second passage qui
    /// ne fait plus rien. Lancé quand `BPM_CADDY_STUPCONTROL_SAMPLE`
    /// désigne l'arbre d'exemple de StupControl
    /// (`scripts/scaffold_tree.py`) ; sinon, rien à dire.
    /// **Une reprise sur de vrais classeurs**, tout l'historique : le
    /// stock reporté vient de la première année, le solde final tombe sur
    /// le bilan, et un en-tête configuré avec une faute se lit quand même,
    /// en le disant. Rien n'est écrit aux classeurs.
    #[cfg(feature = "desktop")]
    #[test]
    fn a_real_register_is_taken_over() {
        use std::path::Path;
        let Ok(sample) = std::env::var("BPM_CADDY_STUPCONTROL_SAMPLE") else {
            return;
        };
        let (swept, db) = officine("real-take");
        let data = swept.0.join("registres");
        copy_tree(Path::new(&sample), &data);
        let before = std::fs::read(data.join("2025/skenan.ods")).unwrap();
        let config = swept.0.join("stupcontrol.json");
        std::fs::write(
            &config,
            serde_json::json!({
                "dataDir": data,
                "drugs": [{"commercialName": "Skenan LP 10 mg", "fileName": "skenan.ods",
                           "specialite": "Skenen LP 10 mg", "minStock": 0}],
                "repartiteurs": [],
                "patientColumns": {}
            })
            .to_string(),
        )
        .unwrap();
        let program = crate::classeur::Native {
            config: config.to_string_lossy().into_owned(),
            audit: Some(swept.0.join("audit.log")),
        };
        let strict = program.read(EVERYTHING).unwrap();
        assert!(
            strict.products[0].movements.is_empty(),
            "StupControl ne lit pas l'en-tête mal écrit"
        );
        let stup = db
            .add_stupefiant(0, "Skenan LP 10 mg", "gélule", 0.0)
            .unwrap();
        let key = &strict.products[0].key;
        db.set_stupcontrol_map(stup, key).unwrap();
        let done = take_over(&db, &program, "2026-03-10", false).unwrap();
        assert!(done.imported > 0, "{done:?}");
        assert_eq!(done.openings, 1);
        assert!(
            done.warnings
                .iter()
                .any(|w| w.line.contains("Skenan LP 10 mg")),
            "{:?}",
            done.warnings
        );
        // Le classeur d'exemple porte une date impossible et un numéro
        // d'ordonnancier écrit deux fois : refusés, dits, et l'écart au
        // bout le montre au lieu d'être fondu dans le stock reporté.
        assert!(
            done.refused.iter().any(|r| r.line.starts_with("31/02/25")),
            "{:?}",
            done.refused
        );
        let read = program.survey(EVERYTHING).unwrap();
        let first = read.products[0].years[0].opening.unwrap();
        let moves = db.stup_moves(stup).unwrap();
        let opening = moves
            .iter()
            .find(|m| m.kind == Kind::Inventaire.as_key())
            .unwrap();
        assert_eq!(opening.quantity, first);
        let ours = db
            .stup_summary()
            .unwrap()
            .into_iter()
            .find(|s| s.product.id == stup)
            .unwrap()
            .stock;
        assert_eq!(done.gaps.len(), 1);
        assert!(same_units(done.gaps[0].1, ours));
        assert!(
            !done
                .warnings
                .iter()
                .any(|w| w.line.contains("pas de classeur")),
            "{:?}",
            done.warnings
        );
        assert_eq!(std::fs::read(data.join("2025/skenan.ods")).unwrap(), before);
        assert!(!swept.0.join("audit.log").exists());
    }

    #[cfg(feature = "desktop")]
    #[test]
    fn a_real_register_is_written_and_read_back() {
        use std::path::Path;
        let Ok(sample) = std::env::var("BPM_CADDY_STUPCONTROL_SAMPLE") else {
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
        let program = crate::classeur::Native {
            config: config.to_string_lossy().into_owned(),
            audit: Some(swept.0.join("audit.log")),
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

    #[cfg(feature = "desktop")]
    fn copy_tree(from: &std::path::Path, to: &std::path::Path) {
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

    /// Un StupControl en mémoire tenu dans un vrai fichier : chaque ligne
    /// écrite y ajoute un octet, et le remettre rend aussi ce qu'il lisait.
    struct OnDisk {
        inner: Fake,
        file: std::path::PathBuf,
        before: Vec<Product>,
    }

    impl Bridge for OnDisk {
        fn read(&self, since: &str) -> Result<Answer, String> {
            self.inner.read(since)
        }
        fn add(&self, entries: &[Entry]) -> Result<Vec<Outcome>, String> {
            let out = self.inner.add(entries)?;
            let mut bytes = std::fs::read(&self.file).unwrap();
            bytes.extend(out.iter().filter(|o| o.ok).map(|_| b'+'));
            std::fs::write(&self.file, bytes).unwrap();
            Ok(out)
        }
        fn files_of(&self, _entries: &[Entry]) -> Vec<std::path::PathBuf> {
            vec![self.file.clone(), self.file.clone()]
        }
        fn restore(&self, file: &std::path::Path, copy: &std::path::Path) -> Result<(), String> {
            std::fs::copy(copy, file).map_err(|e| e.to_string())?;
            *self.inner.products.borrow_mut() = self.before.clone();
            Ok(())
        }
    }

    /// **Le report aux classeurs se défait** : il suit une reprise, n'écrit
    /// rien en aperçu, garde une copie de chaque classeur avant d'écrire,
    /// et le remet — sauf si le classeur a changé depuis, où rien n'est
    /// touché.
    #[test]
    fn a_push_to_the_workbooks_is_written_on_confirmation_and_undone() {
        let (swept, db) = officine("push");
        let root = swept.0.join("reports");
        let stup = db
            .add_stupefiant(0, "Méthadone 10 mg", "gélule", 0.0)
            .unwrap();
        db.set_stupcontrol_map(stup, KEY).unwrap();
        let dupont = db.add_patient("Dupont", "Albert", "1950-01-01").unwrap();
        let mut p = product(KEY, "Méthadone 10 mg", "Méthadone 10 mg");
        p.stock = Some(30.0);
        p.movements = vec![line("E", "2026-03-02", 28.0, "Cerp", "", "")];
        let file = swept.0.join("methadone.ods");
        std::fs::write(&file, b"classeur").unwrap();
        let bridge = OnDisk {
            inner: Fake {
                products: RefCell::new(vec![p]),
                refuse: RefCell::new(None),
                added: RefCell::new(Vec::new()),
            },
            file: file.clone(),
            before: Vec::new(),
        };
        assert!(push(&db, &bridge, "2026-03-09", false, &root).is_err());
        take_over(&db, &bridge, "2026-03-10", false).unwrap();
        let before = bridge.inner.products.borrow().clone();
        let bridge = OnDisk { before, ..bridge };
        let mut sortie = mv(0, Kind::Sortie, "2026-03-12", 5.0);
        sortie.stup_id = stup;
        sortie.ordo_no = 18600;
        sortie.patient_id = dupont;
        db.add_stup_move(&sortie).unwrap();
        // Au classeur, absent du registre : dit, jamais inscrit.
        bridge.inner.products.borrow_mut()[0].movements.push(line(
            "S",
            "2026-03-11",
            2.0,
            "MARTIN",
            "Paul",
            "18599",
        ));

        let seen = push(&db, &bridge, "2026-03-12", true, &root).unwrap();
        assert!(seen.push && seen.preview);
        assert_eq!(seen.exported, 1);
        assert_eq!(seen.unwritten.len(), 1, "{:?}", seen.unwritten);
        assert_eq!(std::fs::read(&file).unwrap(), b"classeur");
        assert!(last_push(&root).is_none(), "un aperçu ne garde rien");

        let done = push(&db, &bridge, "2026-03-12", false, &root).unwrap();
        assert_eq!(done.exported, 1);
        assert_eq!(std::fs::read(&file).unwrap(), b"classeur+");
        let (_, kept) = last_push(&root).unwrap();
        assert_eq!(kept.at, "2026-03-12");
        assert_eq!(kept.files.len(), 1, "un classeur copié une fois");
        let again = push(&db, &bridge, "2026-03-13", false, &root).unwrap();
        assert_eq!(again.exported, 0);
        assert_eq!(
            std::fs::read_dir(&root).unwrap().count(),
            1,
            "un report sans ligne ne garde rien"
        );

        // Le classeur a changé depuis : rien n'est défait.
        std::fs::write(&file, b"classeur+!").unwrap();
        assert!(undo_push(&db, &bridge, "2026-03-13", &root).is_err());
        assert_eq!(std::fs::read(&file).unwrap(), b"classeur+!");
        std::fs::write(&file, b"classeur+").unwrap();

        assert_eq!(undo_push(&db, &bridge, "2026-03-13", &root), Ok(1));
        assert_eq!(std::fs::read(&file).unwrap(), b"classeur");
        assert!(last_push(&root).is_none());
        assert!(undo_push(&db, &bridge, "2026-03-13", &root).is_err());
        let back = push(&db, &bridge, "2026-03-13", true, &root).unwrap();
        assert_eq!(back.exported, 1, "la ligne est de nouveau à reporter");

        // Un pont sans fichiers ne sait rien remettre.
        assert!(bridge.inner.restore(&file, &file).is_err());
        assert!(bridge.inner.files_of(&[]).is_empty());
    }

    /// Un pont qui ne dit qu'une chose : ce qu'on lui fait répondre.
    #[derive(Default)]
    struct Stubborn {
        unreadable: Vec<String>,
        headerless: Vec<String>,
        held: Option<std::path::PathBuf>,
    }

    impl Bridge for Stubborn {
        fn read(&self, _since: &str) -> Result<Answer, String> {
            Ok(Answer {
                unreadable: self.unreadable.clone(),
                headerless: self.headerless.clone(),
                ..Answer::default()
            })
        }
        fn add(&self, _entries: &[Entry]) -> Result<Vec<Outcome>, String> {
            panic!("rien ne s'écrit ici");
        }
        fn writable(&self, file: &std::path::Path) -> Result<(), String> {
            match &self.held {
                Some(h) if h == file => Err("ouvert dans LibreOffice".to_owned()),
                _ => Ok(()),
            }
        }
        fn restore(&self, file: &std::path::Path, copy: &std::path::Path) -> Result<(), String> {
            std::fs::copy(copy, file)
                .map(|_| ())
                .map_err(|e| e.to_string())
        }
    }

    /// **Un classeur illisible n'est pas un classeur vide** : ses lignes
    /// se disaient perdues, et celles d'ici repartaient s'y écrire.
    #[test]
    fn a_workbook_that_cannot_be_read_stops_the_push() {
        let (swept, db) = officine("illisible");
        let root = swept.0.join("reports");
        db.set_stup_setting(TAKEN_KEY, "2026-03-10").unwrap();
        for bridge in [
            Stubborn {
                unreadable: vec!["2026/methadone.ods".to_owned()],
                ..Stubborn::default()
            },
            Stubborn {
                headerless: vec!["2026/methadone.ods".to_owned()],
                ..Stubborn::default()
            },
        ] {
            let said = push(&db, &bridge, "2026-03-12", false, &root).unwrap_err();
            assert!(said.contains("2026/methadone.ods"), "{said}");
        }
    }

    /// **Une reprise relancée ne déplace pas le départ du report** : ce qui
    /// s'est écrit entre-temps sans partir en sortait pour toujours.
    #[test]
    fn a_second_takeover_keeps_the_start_of_the_push_window() {
        let (_swept, db) = officine("reprise2");
        let bridge = Stubborn::default();
        take_over(&db, &bridge, "2026-11-01", false).unwrap();
        take_over(&db, &bridge, "2026-11-10", false).unwrap();
        assert_eq!(db.stup_setting(TAKEN_KEY).as_deref(), Some("2026-11-01"));
    }

    fn journal_of(root: &std::path::Path, files: &[(&std::path::Path, &[u8], &[u8])]) {
        let dir = root.join("0001");
        std::fs::create_dir_all(&dir).unwrap();
        let mut pushed = Pushed {
            at: "2026-03-12".to_owned(),
            ..Pushed::default()
        };
        for (n, (file, before, after)) in files.iter().enumerate() {
            let copy = dir.join(format!("{n}.ods"));
            std::fs::write(&copy, before).unwrap();
            std::fs::write(file, after).unwrap();
            pushed.files.push(PushedFile {
                file: file.to_string_lossy().into_owned(),
                copy: copy.to_string_lossy().into_owned(),
                after: fingerprint(file),
            });
        }
        std::fs::write(dir.join(MANIFEST), serde_json::to_string(&pushed).unwrap()).unwrap();
    }

    /// **Tous libres, ou rien de remis** ; et une annulation interrompue se
    /// reprend : le classeur déjà remis se voyait « changé depuis », et le
    /// report restait à moitié défait pour toujours.
    #[test]
    fn an_undo_touches_nothing_while_one_workbook_is_held_and_can_resume() {
        let (swept, db) = officine("undo2");
        let root = swept.0.join("reports");
        let (a, b) = (swept.0.join("a.ods"), swept.0.join("b.ods"));
        journal_of(&root, &[(&a, b"a0", b"a1"), (&b, b"b0", b"b1")]);
        let held = Stubborn {
            held: Some(b.clone()),
            ..Stubborn::default()
        };
        assert!(undo_push(&db, &held, "2026-03-13", &root).is_err());
        assert_eq!(std::fs::read(&a).unwrap(), b"a1", "rien n'est remis");
        assert!(last_push(&root).is_some());

        // Le poste est tombé entre les deux : `a` est déjà remis.
        std::fs::write(&a, b"a0").unwrap();
        assert_eq!(
            undo_push(&db, &Stubborn::default(), "2026-03-13", &root),
            Ok(0)
        );
        assert_eq!(std::fs::read(&b).unwrap(), b"b0");
        assert!(last_push(&root).is_none());
    }

    /// Un seul poste à la fois, et le verrou d'un poste tombé ne dure pas.
    #[test]
    fn two_posts_do_not_write_to_the_workbooks_together() {
        let (swept, _db) = officine("verrou");
        let root = swept.0.join("reports");
        let lock = WriteLock::take(&root).unwrap();
        assert!(WriteLock::take(&root).is_err());
        drop(lock);
        assert!(WriteLock::take(&root).is_ok());
    }
}
