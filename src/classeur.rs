//! **Les classeurs de StupControl, lus et écrits ici** — le même
//! registre légal, les mêmes gestes que StupControl lui-même.
//!
//! StupControl tient le registre des stupéfiants dans des classeurs
//! LibreOffice (`{données}/{année}/{fichier}.ods`, un onglet par mois,
//! une colonne Entrée, Sortie et Bilan par spécialité). Ce module est la
//! traduction, règle pour règle, de son moteur Java (`ods/`, `config/`,
//! `catalog/`, `bridge/Bridge.java`) : on ne lance plus le programme, on
//! fait **exactement** ce qu'il fait.
//!
//! * **Lire** ([`read_file`]) : le flux de `content.xml` comme
//!   `FastOdsReader` — la meilleure paire d'en-têtes dans les neuf
//!   premières lignes, les en-têtes retapés d'un mois à l'autre ramenés à
//!   un seul produit, la ligne TOTAUX jamais comptée, les lignes répétées
//!   comptées chacune, le Bilan pris sur une ligne de mouvement ou sur la
//!   première ligne du mois.
//! * **Le stock** ([`current_stock`]) : le Bilan de clôture de l'année
//!   précédente plus les entrées et sorties de l'année, comme
//!   `StockCalculator` — le report de janvier, lien externe que
//!   LibreOffice ne recalcule pas toujours, n'est pas cru.
//! * **Ajouter une ligne** ([`add_entry`]) : comme
//!   `StupRegister.addEntry` — refus tant que LibreOffice tient le
//!   fichier (`.~lock.…#`), la première ligne libre au-dessus de TOTAUX
//!   (après compactage des lignes vides si la feuille est pleine), le
//!   numéro d'ordonnancier en nombre s'il en est un, le Bilan en formule
//!   `IFERROR(INT(…))` qui part du dernier bilan de la colonne ; puis un
//!   fichier temporaire, relu, le fichier d'origine inchangé entre-temps,
//!   une copie dans `.backups/` (dix gardées), l'échange atomique et la
//!   ligne du journal d'audit de StupControl.
//!
//! Tout ce que le classeur contient d'autre — styles, autres onglets,
//! formules de report, images, réglages — est recopié tel quel, octet
//! pour octet hors `content.xml`.
//!
//! Le module touche le disque et lit l'heure (le nom d'une sauvegarde,
//! le journal) : ce n'est pas un module pur. Ce qui se décide — en-têtes,
//! correspondance d'une spécialité, ligne d'écriture, bilans — l'est, et
//! se teste sur des classeurs construits en mémoire.

use crate::strings::{tr, trf, trn};
use crate::stupcontrol::{Answer, Bridge, Entry, Line, Outcome, Product};
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};

// ------------------------------------------------------------------
// Java's text primitives
// ------------------------------------------------------------------

/// `String.trim()`: every char at or below U+0020, nothing else.
fn java_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c <= ' ')
}

/// `Character.isWhitespace`: the Unicode space separators except the
/// non-breaking ones, and the ASCII controls Java names.
fn java_whitespace(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{0B}' | '\u{0C}' | '\r' | '\u{1C}'..='\u{1F}'
    ) || (c.is_whitespace()
        && !matches!(c, '\u{A0}' | '\u{2007}' | '\u{202F}')
        && !matches!(c, '\u{85}'))
}

/// `String.isBlank()`.
fn java_blank(s: &str) -> bool {
    s.chars().all(java_whitespace)
}

/// NFD, then every combining mark removed (`\p{M}`).
fn strip_marks(s: &str) -> String {
    use unicode_normalization::char::is_combining_mark;
    use unicode_normalization::UnicodeNormalization;
    s.nfd().filter(|c| !is_combining_mark(*c)).collect()
}

/// `Double.parseDouble` on the forms a register cell can hold: an
/// optional sign, digits, a dot, an exponent, Java's `d`/`f` suffix,
/// `Infinity` and `NaN`. Leading and trailing blanks (≤ U+0020) are
/// ignored, as Java does.
fn java_parse_double(s: &str) -> Option<f64> {
    let t = java_trim(s);
    let body = t.strip_prefix(['+', '-']).unwrap_or(t);
    if body == "Infinity" || body == "NaN" {
        let v = if body == "NaN" {
            f64::NAN
        } else {
            f64::INFINITY
        };
        return Some(if t.starts_with('-') { -v } else { v });
    }
    let t = t.strip_suffix(['d', 'D', 'f', 'F']).unwrap_or(t);
    if t.is_empty()
        || !t
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, '+' | '-' | '.' | 'e' | 'E'))
        || !t.chars().any(|c| c.is_ascii_digit())
    {
        return None;
    }
    t.parse::<f64>().ok()
}

/// `Double.toString`: `28.0`, `0.5`, `1.0E7`, `1.5E-4`.
fn java_double(v: f64) -> String {
    if v.is_nan() {
        return "NaN".into();
    }
    if v.is_infinite() {
        return if v > 0.0 { "Infinity" } else { "-Infinity" }.into();
    }
    if v == 0.0 {
        return if v.is_sign_negative() { "-0.0" } else { "0.0" }.into();
    }
    let a = v.abs();
    if (1e-3..1e7).contains(&a) {
        let s = format!("{v}");
        if s.contains('.') {
            s
        } else {
            s + ".0"
        }
    } else {
        let s = format!("{v:e}");
        let (mant, exp) = s.split_once('e').unwrap_or((&s, "0"));
        let mant = if mant.contains('.') {
            mant.to_owned()
        } else {
            format!("{mant}.0")
        };
        format!("{mant}E{exp}")
    }
}

/// `Num.fmt`: a whole number without decimals, otherwise two decimals
/// rounded half up, trailing zeros dropped.
pub fn num_fmt(v: f64) -> String {
    if !v.is_finite() {
        return java_double(v);
    }
    if v == v.round_ties_even() && v.fract() == 0.0 {
        return format!("{}", v as i64);
    }
    // The shortest decimal that names v (Java's BigDecimal.valueOf),
    // rounded half up at the second decimal.
    let s = format!("{}", v.abs());
    let (int, frac) = s.split_once('.').unwrap_or((&s, ""));
    let mut digits: Vec<u8> = int.bytes().map(|b| b - b'0').collect();
    let mut fr: Vec<u8> = frac.bytes().map(|b| b - b'0').collect();
    let up = fr.len() > 2 && fr[2] >= 5;
    fr.truncate(2);
    while fr.len() < 2 {
        fr.push(0);
    }
    if up {
        let mut i = 1usize;
        loop {
            if fr[i] < 9 {
                fr[i] += 1;
                break;
            }
            fr[i] = 0;
            if i == 0 {
                // carry into the integer part
                let mut j = digits.len();
                loop {
                    if j == 0 {
                        digits.insert(0, 1);
                        break;
                    }
                    j -= 1;
                    if digits[j] < 9 {
                        digits[j] += 1;
                        break;
                    }
                    digits[j] = 0;
                }
                break;
            }
            i -= 1;
        }
    }
    while fr.last() == Some(&0) {
        fr.pop();
    }
    let int: String = digits.iter().map(|d| (b'0' + d) as char).collect();
    let int = if int.is_empty() { "0".to_owned() } else { int };
    let zero = int.bytes().all(|b| b == b'0') && fr.is_empty();
    let mut out = String::new();
    if v < 0.0 && !zero {
        out.push('-');
    }
    out.push_str(&int);
    if !fr.is_empty() {
        out.push('.');
        out.extend(fr.iter().map(|d| (b'0' + d) as char));
    }
    out
}

/// `Num.parse`: comma or dot decimal, spaces of every kind as grouping.
fn num_parse(s: &str) -> Option<f64> {
    let t: String = java_trim(s)
        .chars()
        .filter(|c| {
            !matches!(
                c,
                ' ' | '\t'
                    | '\n'
                    | '\u{0B}'
                    | '\u{0C}'
                    | '\r'
                    | '\u{A0}'
                    | '\u{202F}'
                    | '\u{2007}'
                    | '\u{2060}'
            )
        })
        .collect();
    if t.is_empty() {
        return None;
    }
    let t = if t.contains(',') {
        t.replace('.', "").replace(',', ".")
    } else {
        t
    };
    java_parse_double(&t).filter(|v| !v.is_nan())
}

// ------------------------------------------------------------------
// LayoutDetector's folds and matching
// ------------------------------------------------------------------

/// `LayoutDetector.norm`: trimmed, lowercase, accents stripped.
pub fn norm(s: &str) -> String {
    strip_marks(&java_trim(s).to_lowercase())
}

/// `Fuzzy.norm`: accents stripped, lowercase, only `a-z0-9` and single
/// spaces.
fn fuzzy_norm(s: &str) -> String {
    let n = strip_marks(s).to_lowercase();
    let n: String = n
        .chars()
        .map(|c| {
            if c.is_ascii_lowercase() || c.is_ascii_digit() || c == ' ' {
                c
            } else {
                ' '
            }
        })
        .collect();
    n.split([' ', '\t', '\n', '\u{0B}', '\u{0C}', '\r'])
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// `Fuzzy.score`: higher is closer, -1 is no match.
fn fuzzy_score(query: &str, candidate: &str) -> i32 {
    let q = fuzzy_norm(query);
    let c = fuzzy_norm(candidate);
    if q.is_empty() {
        return 0;
    }
    if c == q {
        return 1000;
    }
    let cl = c.len() as i32;
    if c.contains(&q) {
        return 800 - (cl - q.len() as i32);
    }
    let tokens: Vec<&str> = q.split(' ').collect();
    let matched = tokens
        .iter()
        .filter(|t| !t.is_empty() && c.contains(**t))
        .count() as i32;
    if matched == tokens.len() as i32 {
        return 600 - cl;
    }
    if matched > 0 {
        return 300 + matched * 20 - cl;
    }
    let qs: Vec<char> = q.replace(' ', "").chars().collect();
    let mut i = 0;
    for ch in c.replace(' ', "").chars() {
        if i < qs.len() && ch == qs[i] {
            i += 1;
        }
    }
    if i == qs.len() {
        return 100 - cl;
    }
    -1
}

/// The first word of an already folded header (`[^a-z0-9]+` splits).
fn first_word_is(s: &str, letter: &str, prefixes: &[&str]) -> bool {
    let first = s
        .split(|c: char| !(c.is_ascii_lowercase() || c.is_ascii_digit()))
        .find(|w| !w.is_empty())
        .unwrap_or("");
    first == letter || prefixes.iter().any(|p| first.starts_with(p))
}

fn is_entry(s: &str) -> bool {
    first_word_is(s, "e", &["entree", "recept"])
}

fn is_exit(s: &str) -> bool {
    first_word_is(s, "s", &["sortie", "deliv"])
}

fn is_balance(s: &str) -> bool {
    first_word_is(s, "b", &["bilan", "solde", "balance", "stock"])
}

/// The dotted abbreviation of `normSpecialite`: `(?:[a-z]\.)+[a-z]?\.?`.
fn dotted_abbreviation(t: &str) -> bool {
    let b = t.as_bytes();
    let mut i = 0;
    let mut pairs = 0;
    while i + 1 < b.len() && b[i].is_ascii_lowercase() && b[i + 1] == b'.' {
        i += 2;
        pairs += 1;
    }
    if pairs == 0 {
        return false;
    }
    let rest = &b[i..];
    match rest {
        [] | [b'.'] => true,
        [c] => c.is_ascii_lowercase(),
        [c, b'.'] => c.is_ascii_lowercase(),
        _ => false,
    }
}

/// `LayoutDetector.normSpecialite`: « SKENAN L.P. 10MG », « Skenan LP
/// 10 mg » and « SKENAN-LP 10 mg » are all `skenan lp 10 mg`.
pub fn norm_specialite(s: &str) -> String {
    let n: String = norm(s)
        .chars()
        .filter(|c| {
            !matches!(
                c,
                '\u{AD}' | '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{FEFF}'
            )
        })
        .collect();
    let n = n
        .replace(['µ', 'μ'], " u")
        .replace('%', " pct ")
        .replace('‰', " pmil ");
    let mut sb = String::new();
    for tok in n.split(|c: char| !(c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.')) {
        if tok.is_empty() {
            continue;
        }
        if !sb.is_empty() {
            sb.push(' ');
        }
        if dotted_abbreviation(tok) {
            sb.push_str(&tok.replace('.', ""));
        } else {
            sb.push_str(&tok.replace('.', " "));
        }
    }
    // A space at every digit/letter edge.
    let mut edged = String::with_capacity(sb.len() + 8);
    let mut prev: Option<char> = None;
    for c in sb.chars() {
        if let Some(p) = prev {
            if (p.is_ascii_digit() && c.is_ascii_lowercase())
                || (p.is_ascii_lowercase() && c.is_ascii_digit())
            {
                edged.push(' ');
            }
        }
        edged.push(c);
        prev = Some(c);
    }
    edged
        .split(' ')
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn digits_only(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

/// `subsetScore`: every typed word the start of a distinct header word
/// (a number only by equality); -1 when one is missing.
fn subset_score(query: &[&str], header: &[&str]) -> i32 {
    let mut used = vec![false; header.len()];
    let mut score = 0i32;
    for q in query {
        if q.is_empty() {
            continue;
        }
        let numeric = digits_only(q);
        let mut best_j: Option<usize> = None;
        let mut best_extra = i32::MAX;
        for (j, h) in header.iter().enumerate() {
            if used[j] {
                continue;
            }
            if if numeric { h != q } else { !h.starts_with(q) } {
                continue;
            }
            let extra = h.len() as i32 - q.len() as i32;
            if extra < best_extra {
                best_extra = extra;
                best_j = Some(j);
            }
        }
        let Some(j) = best_j else {
            return -1;
        };
        used[j] = true;
        score += 100 - best_extra.min(99);
    }
    score - (header.len() as i32 - query.len() as i32)
}

/// `LayoutDetector.matchSpecialite`: the one header a spécialité names
/// (exact fold first, then the closest unique token subset), or none
/// when two fit as well.
pub fn match_specialite(query: &str, candidates: &[String]) -> Option<usize> {
    let q = norm_specialite(query);
    if q.is_empty() {
        return None;
    }
    let norms: Vec<String> = candidates.iter().map(|c| norm_specialite(c)).collect();
    let exact: Vec<usize> = (0..norms.len()).filter(|&i| norms[i] == q).collect();
    match exact.len() {
        1 => return Some(exact[0]),
        n if n > 1 => return None,
        _ => {}
    }
    let qt: Vec<&str> = q.split(' ').collect();
    let mut best: Option<usize> = None;
    let mut best_score = i32::MIN;
    let mut tie = false;
    for (i, n) in norms.iter().enumerate() {
        let ht: Vec<&str> = n.split(' ').collect();
        let s = subset_score(&qt, &ht);
        if s < 0 {
            continue;
        }
        if s > best_score {
            best_score = s;
            best = Some(i);
            tie = false;
        } else if s == best_score {
            tie = true;
        }
    }
    if tie {
        None
    } else {
        best
    }
}

/// `isTotalsLabel`: « TOTAUX » or « TOTAL » as a whole word.
fn is_totals_label(text: &str) -> bool {
    if java_blank(text) {
        return false;
    }
    norm(text)
        .split(|c: char| !c.is_ascii_lowercase())
        .any(|t| t == "total" || t == "totaux")
}

// ------------------------------------------------------------------
// Layout
// ------------------------------------------------------------------

/// One spécialité's three columns, 0-based; `b` is absent when the sheet
/// has no Bilan for it.
#[derive(Clone, Debug, PartialEq)]
pub struct Col {
    pub name: String,
    pub e: usize,
    pub s: usize,
    pub b: Option<usize>,
}

/// `SheetLayout`: the patient columns (-1 when not found), the first
/// data row and the spécialités.
#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    pub nom: i64,
    pub prenom: i64,
    pub date: i64,
    pub ordo: i64,
    pub first_data_row: usize,
    pub products: Vec<Col>,
}

impl Layout {
    fn product(&self, specialite: &str) -> Option<&Col> {
        let names: Vec<String> = self.products.iter().map(|p| p.name.clone()).collect();
        let i = match_specialite(specialite, &names)?;
        let name = &names[i];
        self.products.iter().find(|p| &p.name == name)
    }

    fn max_col(&self) -> usize {
        let mut max = [self.nom, self.prenom, self.date, self.ordo]
            .into_iter()
            .max()
            .unwrap_or(0)
            .max(0) as usize;
        for p in &self.products {
            max = max.max(p.e).max(p.s).max(p.b.unwrap_or(0));
        }
        max
    }
}

/// A header row: text by column, absent where the cell is empty.
type RowTexts = HashMap<usize, String>;

fn at(row: &RowTexts, c: usize) -> &str {
    row.get(&c).map(String::as_str).unwrap_or("")
}

const MAX_HEADER_SCAN: usize = 8;
const PATIENT_THRESHOLD: i32 = 300;

fn detect_products(cols: usize, row0: &RowTexts, row1: &RowTexts) -> Option<Vec<Col>> {
    let mut keys: Vec<usize> = row0.keys().copied().filter(|&c| c < cols).collect();
    keys.sort_unstable();
    let mut out = Vec::new();
    for c in keys {
        let name = java_trim(at(row0, c));
        if name.is_empty() {
            continue;
        }
        let s0 = norm(at(row1, c));
        let s1 = if c + 1 < cols {
            norm(at(row1, c + 1))
        } else {
            String::new()
        };
        if is_entry(&s0) && is_exit(&s1) {
            let s2 = if c + 2 < cols {
                norm(at(row1, c + 2))
            } else {
                String::new()
            };
            out.push(Col {
                name: name.to_owned(),
                e: c,
                s: c + 1,
                b: is_balance(&s2).then_some(c + 2),
            });
        }
    }
    (!out.is_empty()).then_some(out)
}

fn assign_patient_columns(bound: usize, row1: &RowTexts) -> [i64; 4] {
    const KW: [&[&str]; 4] = [
        &["nom", "patient", "beneficiaire", "client", "titulaire"],
        &["prenom"],
        &["date", "jour"],
        &[
            "ordonnancier",
            "ordonnance",
            "ordo",
            "ordre",
            "n ordre",
            "numero d ordre",
            "prescription",
            "registre",
        ],
    ];
    let mut keys: Vec<usize> = row1.keys().copied().filter(|&c| c < bound).collect();
    keys.sort_unstable();
    let mut ranked: Vec<(usize, usize, i32)> = Vec::new();
    for (role, kws) in KW.iter().enumerate() {
        for &col in &keys {
            let best = kws
                .iter()
                .map(|kw| fuzzy_score(kw, at(row1, col)))
                .max()
                .unwrap_or(-1);
            if best >= PATIENT_THRESHOLD {
                ranked.push((role, col, best));
            }
        }
    }
    // Stable, as Java's List.sort.
    ranked.sort_by_key(|r| std::cmp::Reverse(r.2));
    let mut cols = [-1i64; 4];
    let mut used = HashSet::new();
    for (role, col, _) in ranked {
        if cols[role] == -1 && !used.contains(&col) {
            cols[role] = col as i64;
            used.insert(col);
        }
    }
    cols
}

/// `bestLayout`: of every adjacent pair of rows in the window, the one
/// that finds the most spécialités, then the most patient columns; the
/// topmost on a tie.
fn best_layout(cols: usize, window: &[RowTexts], row_index: &[usize]) -> Option<Layout> {
    let mut best: Option<Layout> = None;
    let mut best_score = -1i64;
    for k in 1..window.len() {
        let Some(products) = detect_products(cols, &window[k - 1], &window[k]) else {
            continue;
        };
        let bound = products[0].e;
        let pc = assign_patient_columns(bound, &window[k]);
        let roles = pc.iter().filter(|c| **c >= 0).count() as i64;
        let score = products.len() as i64 * 8 + roles;
        if score > best_score {
            best_score = score;
            best = Some(Layout {
                nom: pc[0],
                prenom: pc[1],
                date: pc[2],
                ordo: pc[3],
                first_data_row: row_index[k] + 1,
                products,
            });
        }
    }
    best
}

const MONTHS: [&str; 12] = [
    "Janvier",
    "Février",
    "Mars",
    "Avril",
    "Mai",
    "Juin",
    "Juillet",
    "Août",
    "Septembre",
    "Octobre",
    "Novembre",
    "Décembre",
];

/// The sheet of a month, 1-12.
pub fn sheet_name(month: u32) -> &'static str {
    MONTHS[(month.clamp(1, 12) - 1) as usize]
}

fn month_index_of(sheet: &str) -> usize {
    let n = norm(sheet);
    MONTHS
        .iter()
        .position(|m| norm(m) == n)
        .map(|i| i + 1)
        .unwrap_or(0)
}

// ------------------------------------------------------------------
// Register dates
// ------------------------------------------------------------------

fn leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn days_in(y: i64, m: i64) -> i64 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap(y) => 29,
        _ => 28,
    }
}

/// `RegisterDates.parse`: `d/M/yy` or `d/M/yyyy`, strict — « 31/02/25 »
/// is no date. Returns `(année, mois, jour)`.
pub fn register_date(s: &str) -> Option<(i64, i64, i64)> {
    let t = java_trim(s);
    let mut parts = t.split('/');
    let (d, m, y) = (parts.next()?, parts.next()?, parts.next()?);
    if parts.next().is_some() {
        return None;
    }
    let ok = |p: &str, lens: &[usize]| digits_only(p) && lens.contains(&p.len());
    if !ok(d, &[1, 2]) || !ok(m, &[1, 2]) || !ok(y, &[2, 4]) {
        return None;
    }
    let day: i64 = d.parse().ok()?;
    let month: i64 = m.parse().ok()?;
    let year: i64 = if y.len() == 2 {
        2000 + y.parse::<i64>().ok()?
    } else {
        y.parse().ok()?
    };
    if !(1..=12).contains(&month) || day < 1 || day > days_in(year, month) {
        return None;
    }
    Some((year, month, day))
}

/// An ISO day `AAAA-MM-JJ`, strict.
fn iso_date(s: &str) -> Option<(i64, i64, i64)> {
    let b = s.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return None;
    }
    let (y, m, d) = (&s[0..4], &s[5..7], &s[8..10]);
    if !digits_only(y) || !digits_only(m) || !digits_only(d) {
        return None;
    }
    let (y, m, d): (i64, i64, i64) = (y.parse().ok()?, m.parse().ok()?, d.parse().ok()?);
    if !(1..=12).contains(&m) || d < 1 || d > days_in(y, m) {
        return None;
    }
    Some((y, m, d))
}

// ------------------------------------------------------------------
// The zip around the workbook
// ------------------------------------------------------------------

struct ZipEntry {
    name: Vec<u8>,
    flags: u16,
    method: u16,
    time: u16,
    date: u16,
    crc: u32,
    csize: u32,
    usize_: u32,
    data: Vec<u8>,
}

fn le16(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(at..at + 2)?.try_into().ok()?))
}

fn le32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

fn zip_read(bytes: &[u8]) -> Option<Vec<ZipEntry>> {
    let min = bytes.len().saturating_sub(22 + 65535);
    let eocd = (min..=bytes.len().checked_sub(22)?)
        .rev()
        .find(|&i| le32(bytes, i) == Some(0x0605_4b50))?;
    let count = le16(bytes, eocd + 10)? as usize;
    let mut at = le32(bytes, eocd + 16)? as usize;
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        if le32(bytes, at)? != 0x0201_4b50 {
            return None;
        }
        let flags = le16(bytes, at + 8)?;
        let method = le16(bytes, at + 10)?;
        let time = le16(bytes, at + 12)?;
        let date = le16(bytes, at + 14)?;
        let crc = le32(bytes, at + 16)?;
        let csize = le32(bytes, at + 20)?;
        let usize_ = le32(bytes, at + 24)?;
        let nlen = le16(bytes, at + 28)? as usize;
        let xlen = le16(bytes, at + 30)? as usize;
        let clen = le16(bytes, at + 32)? as usize;
        let local = le32(bytes, at + 42)? as usize;
        if csize == u32::MAX || usize_ == u32::MAX || local == u32::MAX as usize {
            return None; // zip64: never a register
        }
        let name = bytes.get(at + 46..at + 46 + nlen)?.to_vec();
        at += 46 + nlen + xlen + clen;
        if le32(bytes, local)? != 0x0403_4b50 {
            return None;
        }
        let lnlen = le16(bytes, local + 26)? as usize;
        let lxlen = le16(bytes, local + 28)? as usize;
        let start = local + 30 + lnlen + lxlen;
        let data = bytes.get(start..start + csize as usize)?.to_vec();
        out.push(ZipEntry {
            name,
            flags,
            method,
            time,
            date,
            crc,
            csize,
            usize_,
            data,
        });
    }
    Some(out)
}

fn zip_inflate(e: &ZipEntry) -> Option<Vec<u8>> {
    match e.method {
        0 => Some(e.data.clone()),
        8 => {
            let mut out = Vec::with_capacity(e.usize_ as usize);
            flate2::read::DeflateDecoder::new(&e.data[..])
                .read_to_end(&mut out)
                .ok()?;
            Some(out)
        }
        _ => None,
    }
}

fn zip_entry<'a>(entries: &'a [ZipEntry], name: &str) -> Option<&'a ZipEntry> {
    entries.iter().find(|e| e.name == name.as_bytes())
}

fn zip_write(entries: &[ZipEntry]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut central = Vec::new();
    for e in entries {
        let offset = out.len() as u32;
        let flags = e.flags & !0x0008; // sizes in the header, no descriptor
        let mut head = Vec::with_capacity(30 + e.name.len());
        head.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        head.extend_from_slice(&20u16.to_le_bytes());
        head.extend_from_slice(&flags.to_le_bytes());
        head.extend_from_slice(&e.method.to_le_bytes());
        head.extend_from_slice(&e.time.to_le_bytes());
        head.extend_from_slice(&e.date.to_le_bytes());
        head.extend_from_slice(&e.crc.to_le_bytes());
        head.extend_from_slice(&e.csize.to_le_bytes());
        head.extend_from_slice(&e.usize_.to_le_bytes());
        head.extend_from_slice(&(e.name.len() as u16).to_le_bytes());
        head.extend_from_slice(&0u16.to_le_bytes());
        head.extend_from_slice(&e.name);
        out.extend_from_slice(&head);
        out.extend_from_slice(&e.data);
        central.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        central.extend_from_slice(&20u16.to_le_bytes());
        central.extend_from_slice(&20u16.to_le_bytes());
        central.extend_from_slice(&flags.to_le_bytes());
        central.extend_from_slice(&e.method.to_le_bytes());
        central.extend_from_slice(&e.time.to_le_bytes());
        central.extend_from_slice(&e.date.to_le_bytes());
        central.extend_from_slice(&e.crc.to_le_bytes());
        central.extend_from_slice(&e.csize.to_le_bytes());
        central.extend_from_slice(&e.usize_.to_le_bytes());
        central.extend_from_slice(&(e.name.len() as u16).to_le_bytes());
        central.extend_from_slice(&[0u8; 8]); // extra, comment, disk, internal attrs
        central.extend_from_slice(&0u32.to_le_bytes()); // external attrs
        central.extend_from_slice(&offset.to_le_bytes());
        central.extend_from_slice(&e.name);
    }
    let cd_offset = out.len() as u32;
    out.extend_from_slice(&central);
    out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    out.extend_from_slice(&[0u8; 4]);
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    out.extend_from_slice(&(central.len() as u32).to_le_bytes());
    out.extend_from_slice(&cd_offset.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}

fn deflated(name: &[u8], like: &ZipEntry, content: &[u8]) -> ZipEntry {
    use std::io::Write;
    let mut enc = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
    let _ = enc.write_all(content);
    let data = enc.finish().unwrap_or_default();
    let mut crc = crc32fast::Hasher::new();
    crc.update(content);
    ZipEntry {
        name: name.to_vec(),
        flags: like.flags,
        method: 8,
        time: like.time,
        date: like.date,
        crc: crc.finalize(),
        csize: data.len() as u32,
        usize_: content.len() as u32,
        data,
    }
}

fn content_xml(path: &Path) -> Result<(Vec<ZipEntry>, Vec<u8>), String> {
    let name = file_name(path);
    let bytes = std::fs::read(path).map_err(|_| trf("cls_err_read", &name))?;
    let entries = zip_read(&bytes).ok_or_else(|| trf("cls_err_read", &name))?;
    let content =
        zip_entry(&entries, "content.xml").ok_or_else(|| trf("cls_err_no_content", &name))?;
    let xml = zip_inflate(content).ok_or_else(|| trf("cls_err_read", &name))?;
    Ok((entries, xml))
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

// ------------------------------------------------------------------
// Reading: FastOdsReader
// ------------------------------------------------------------------

const MAX_REPEAT_FILL: usize = 256;
const MAX_DATA_ROWS_PER_SHEET: usize = 100_000;

/// One movement read from a sheet.
#[derive(Clone, Debug, PartialEq)]
pub struct Movement {
    pub month: usize,
    pub date: String,
    /// Entrée (réception) when true, sortie (délivrance) otherwise.
    pub entry: bool,
    pub units: f64,
    pub who: String,
    pub prenom: String,
    pub ordo: String,
}

#[derive(Clone, Debug)]
struct Acc {
    monthly: [[f64; 2]; 12],
    movements: Vec<Movement>,
    closing: [f64; 12],
    /// The first Bilan of each month less what its own row did: what the
    /// month opened at. Not StupControl's — the takeover's check.
    opening: [f64; 12],
    captured: [bool; 12],
    last_movement_month: usize,
}

impl Acc {
    fn new() -> Acc {
        Acc {
            monthly: [[0.0; 2]; 12],
            movements: Vec::new(),
            closing: [f64::NAN; 12],
            opening: [f64::NAN; 12],
            captured: [false; 12],
            last_movement_month: 0,
        }
    }
}

/// Every spécialité of one workbook: monthly totals, movements and the
/// Bilan of each month (`MovementReader.FileData`).
#[derive(Clone, Debug)]
pub struct FileData {
    names: Vec<String>,
    accs: Vec<Acc>,
}

impl FileData {
    fn slot(&mut self, name: &str) -> usize {
        match self.names.iter().position(|n| n == name) {
            Some(i) => i,
            None => {
                self.names.push(name.to_owned());
                self.accs.push(Acc::new());
                self.names.len() - 1
            }
        }
    }

    fn find(&self, specialite: &str) -> Option<&Acc> {
        match_specialite(specialite, &self.names).map(|i| &self.accs[i])
    }

    /// The headers the workbook holds, in the order first met.
    pub fn products(&self) -> &[String] {
        &self.names
    }

    /// The Bilan of the last month with a movement, else the latest
    /// carried-in balance.
    pub fn current_balance(&self, specialite: &str) -> Option<f64> {
        let a = self.find(specialite)?;
        if a.last_movement_month >= 1 && !a.closing[a.last_movement_month - 1].is_nan() {
            return Some(a.closing[a.last_movement_month - 1]);
        }
        a.closing.iter().rev().find(|v| !v.is_nan()).copied()
    }

    pub fn monthly(&self, specialite: &str) -> Option<[[f64; 2]; 12]> {
        self.find(specialite).map(|a| a.monthly)
    }

    pub fn movements(&self, specialite: &str) -> Option<&[Movement]> {
        self.find(specialite).map(|a| a.movements.as_slice())
    }

    /// What the year opened at: the first month's first Bilan, less what
    /// its own row did.
    pub fn opening(&self, specialite: &str) -> Option<f64> {
        let a = self.find(specialite)?;
        a.opening.iter().find(|v| !v.is_nan()).copied()
    }
}

/// **The header a misspelt name designates**, when StupControl's own
/// matching finds none: the same doses, and the words within one typing
/// slip per six letters, of one header only. « Oxycodonne 10 » reads
/// « Oxycodone 10 » ; « Oxycodone 20 » never does.
pub fn closest_header(query: &str, candidates: &[String]) -> Option<usize> {
    fn split(s: &str) -> (Vec<String>, String) {
        let n = norm_specialite(s);
        let mut doses: Vec<String> = n
            .split(' ')
            .filter(|t| t.chars().any(|c| c.is_ascii_digit()))
            .map(str::to_owned)
            .collect();
        doses.sort();
        let words = n
            .split(' ')
            .filter(|t| !t.chars().any(|c| c.is_ascii_digit()))
            .collect::<Vec<_>>()
            .join(" ");
        (doses, words)
    }
    fn distance(a: &str, b: &str) -> usize {
        let a: Vec<char> = a.chars().collect();
        let b: Vec<char> = b.chars().collect();
        let mut row: Vec<usize> = (0..=b.len()).collect();
        for i in 1..=a.len() {
            let mut prev = row[0];
            row[0] = i;
            for j in 1..=b.len() {
                let here = row[j];
                row[j] = if a[i - 1] == b[j - 1] {
                    prev
                } else {
                    1 + prev.min(row[j]).min(row[j - 1])
                };
                prev = here;
            }
        }
        row[b.len()]
    }
    let (doses, words) = split(query);
    if words.is_empty() {
        return None;
    }
    let allowed = (words.chars().count() / 6).max(1);
    let hits: Vec<usize> = candidates
        .iter()
        .enumerate()
        .filter(|(_, c)| {
            let (d, w) = split(c);
            d == doses && distance(&w, &words) <= allowed
        })
        .map(|(i, _)| i)
        .collect();
    match hits.as_slice() {
        [one] => Some(*one),
        _ => None,
    }
}

/// Reads a workbook as StupControl's streaming reader does.
pub fn read_file(path: &Path) -> Result<FileData, String> {
    let (_, xml) = content_xml(path)?;
    parse_content(&xml).map_err(|e| {
        if e.is_empty() {
            trf("cls_err_read", file_name(path))
        } else {
            e
        }
    })
}

struct Window {
    texts: Vec<RowTexts>,
    vals: Vec<HashMap<usize, f64>>,
    meta: Vec<(usize, usize)>,
}

impl Window {
    fn clear(&mut self) {
        self.texts.clear();
        self.vals.clear();
        self.meta.clear();
    }
}

fn local(name: &[u8]) -> &[u8] {
    match name.iter().rposition(|b| *b == b':') {
        Some(i) => &name[i + 1..],
        None => name,
    }
}

fn attr_local(e: &quick_xml::events::BytesStart, want: &[u8]) -> Option<String> {
    e.attributes()
        .flatten()
        .find(|a| local(a.key.as_ref()) == want)
        .and_then(|a| a.unescape_value().ok().map(|v| v.into_owned()))
}

fn int_attr(e: &quick_xml::events::BytesStart, want: &[u8], def: usize) -> usize {
    attr_local(e, want)
        .and_then(|v| java_trim(&v).parse::<i64>().ok())
        .map(|v| v.max(0) as usize)
        .unwrap_or(def)
}

/// The streaming pass over `content.xml`. An empty error is « unreadable »
/// without more to say.
fn parse_content(xml: &[u8]) -> Result<FileData, String> {
    use quick_xml::events::Event;
    let mut r = quick_xml::Reader::from_reader(xml);
    r.config_mut().trim_text(false);
    r.config_mut().expand_empty_elements = true;
    let mut data = FileData {
        names: Vec::new(),
        accs: Vec::new(),
    };
    let mut canon: HashMap<String, String> = HashMap::new();
    let mut month = 0usize;
    let mut rows_seen = 0usize;
    let mut layout: Option<Layout> = None;
    let mut failed = false;
    let mut win = Window {
        texts: Vec::new(),
        vals: Vec::new(),
        meta: Vec::new(),
    };
    let mut next_logical = 0usize;
    let mut row_vals: HashMap<usize, f64> = HashMap::new();
    let mut row_texts: RowTexts = HashMap::new();
    let mut col = 0usize;
    let mut cell_start = 0usize;
    let mut cell_repeat = 1usize;
    let mut row_repeat = 1usize;
    let mut cell_value: Option<f64> = None;
    let mut in_cell = false;
    let mut cell_text = String::new();
    loop {
        let ev = r.read_event().map_err(|_| String::new())?;
        match ev {
            Event::Start(e) => match local(e.name().as_ref()) {
                b"table" => {
                    month = month_index_of(&attr_local(&e, b"name").unwrap_or_default());
                    rows_seen = 0;
                    layout = None;
                    failed = false;
                    win.clear();
                    next_logical = 0;
                }
                b"table-row" if month != 0 => {
                    row_repeat = int_attr(&e, b"number-rows-repeated", 1);
                    col = 0;
                    row_vals.clear();
                    row_texts.clear();
                }
                b"table-cell" | b"covered-table-cell" if month != 0 => {
                    cell_start = col;
                    cell_repeat = int_attr(&e, b"number-columns-repeated", 1);
                    cell_value = attr_local(&e, b"value").and_then(|v| java_parse_double(&v));
                    cell_text.clear();
                    in_cell = true;
                }
                _ => {}
            },
            Event::Text(t) if in_cell => {
                cell_text.push_str(&t.unescape().map_err(|_| String::new())?);
            }
            Event::CData(t) if in_cell => {
                cell_text.push_str(&String::from_utf8_lossy(&t.into_inner()));
            }
            Event::End(e) => match local(e.name().as_ref()) {
                b"table-cell" | b"covered-table-cell" if month != 0 => {
                    let rep = cell_repeat.max(1);
                    if cell_value.is_some() || !cell_text.is_empty() {
                        let fill = rep.min(MAX_REPEAT_FILL);
                        for k in 0..fill {
                            if let Some(v) = cell_value {
                                row_vals.insert(cell_start + k, v);
                            }
                            if !cell_text.is_empty() {
                                row_texts.insert(cell_start + k, cell_text.clone());
                            }
                        }
                    }
                    col = cell_start + rep;
                    in_cell = false;
                }
                b"table-row" if month != 0 => {
                    if layout.is_none() {
                        if !failed {
                            win.texts.push(row_texts.clone());
                            win.vals.push(row_vals.clone());
                            win.meta.push((next_logical, row_repeat.max(1)));
                            next_logical += row_repeat.max(1);
                            if next_logical > MAX_HEADER_SCAN {
                                layout = decide_layout(&win, &mut data, &mut canon);
                                failed = layout.is_none();
                                if let Some(lay) = &layout {
                                    rows_seen +=
                                        replay_window(month, lay, &win, &mut data, rows_seen)?;
                                }
                                win.clear();
                            }
                        }
                    } else if !(row_vals.is_empty() && row_texts.is_empty()) {
                        if rows_seen >= MAX_DATA_ROWS_PER_SHEET {
                            return Err(trn(
                                "cls_err_huge",
                                &[&sheet_name(month as u32), &MAX_DATA_ROWS_PER_SHEET],
                            ));
                        }
                        let rep = row_repeat.clamp(1, MAX_REPEAT_FILL);
                        if let Some(lay) = &layout {
                            for _ in 0..rep {
                                record_row(month, lay, &row_vals, &row_texts, &mut data);
                            }
                        }
                        rows_seen += rep;
                    }
                }
                b"table" => {
                    if month != 0 && layout.is_none() && !failed && !win.texts.is_empty() {
                        layout = decide_layout(&win, &mut data, &mut canon);
                        if let Some(lay) = &layout {
                            replay_window(month, lay, &win, &mut data, rows_seen)?;
                        }
                    }
                    month = 0;
                    win.clear();
                }
                _ => {}
            },
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(data)
}

fn decide_layout(
    win: &Window,
    data: &mut FileData,
    canon: &mut HashMap<String, String>,
) -> Option<Layout> {
    let cols = win
        .texts
        .iter()
        .map(|m| m.keys().copied().max().unwrap_or(0))
        .max()
        .unwrap_or(0)
        + 3;
    let index: Vec<usize> = win.meta.iter().map(|m| m.0).collect();
    let mut lay = best_layout(cols, &win.texts, &index)?;
    for p in &mut lay.products {
        let display = canon
            .entry(norm_specialite(&p.name))
            .or_insert_with(|| p.name.clone())
            .clone();
        p.name = display;
        data.slot(&p.name);
    }
    Some(lay)
}

fn replay_window(
    month: usize,
    lay: &Layout,
    win: &Window,
    data: &mut FileData,
    seen: usize,
) -> Result<usize, String> {
    let sub = lay.first_data_row - 1;
    let mut added = 0usize;
    for i in 0..win.texts.len() {
        if win.meta[i].0 <= sub {
            continue;
        }
        if win.texts[i].is_empty() && win.vals[i].is_empty() {
            continue;
        }
        if seen + added >= MAX_DATA_ROWS_PER_SHEET {
            return Err(trn(
                "cls_err_huge",
                &[&sheet_name(month as u32), &MAX_DATA_ROWS_PER_SHEET],
            ));
        }
        let rep = win.meta[i].1.min(MAX_REPEAT_FILL);
        for _ in 0..rep {
            record_row(month, lay, &win.vals[i], &win.texts[i], data);
        }
        added += rep;
    }
    Ok(added)
}

fn label_cols(lay: &Layout) -> [i64; 6] {
    [0, 1, lay.nom, lay.prenom, lay.date, lay.ordo]
}

fn record_row(
    month: usize,
    lay: &Layout,
    vals: &HashMap<usize, f64>,
    texts: &RowTexts,
    data: &mut FileData,
) {
    if label_cols(lay)
        .iter()
        .any(|&c| c >= 0 && texts.get(&(c as usize)).is_some_and(|t| is_totals_label(t)))
    {
        return;
    }
    let text = |c: i64| {
        if c >= 0 {
            at(texts, c as usize).to_owned()
        } else {
            String::new()
        }
    };
    let date = text(lay.date);
    let who = text(lay.nom);
    let prenom = text(lay.prenom);
    let ordo = if lay.ordo < 0 {
        String::new()
    } else {
        match vals.get(&(lay.ordo as usize)) {
            Some(v) if *v == v.round_ties_even() => format!("{}", *v as i64),
            Some(v) => num_fmt(*v),
            None => text(lay.ordo),
        }
    };
    let num = |c: usize| {
        vals.get(&c)
            .copied()
            .or_else(|| texts.get(&c).and_then(|t| num_parse(t)))
    };
    for p in &lay.products {
        let slot = data.slot(&p.name);
        let acc = &mut data.accs[slot];
        let e = num(p.e);
        let s = num(p.s);
        let movement = e.is_some_and(|v| v > 0.0) || s.is_some_and(|v| v > 0.0);
        if let Some(bc) = p.b {
            if let Some(b) = vals.get(&bc) {
                if acc.opening[month - 1].is_nan() {
                    let e = e.filter(|v| *v > 0.0).unwrap_or(0.0);
                    let s = s.filter(|v| *v > 0.0).unwrap_or(0.0);
                    acc.opening[month - 1] = *b - e + s;
                }
                if movement || !acc.captured[month - 1] {
                    acc.closing[month - 1] = *b;
                    acc.captured[month - 1] = true;
                }
            }
        }
        if movement {
            acc.captured[month - 1] = true;
        }
        let mut push = |entry: bool, units: f64| {
            acc.movements.push(Movement {
                month,
                date: date.clone(),
                entry,
                units,
                who: who.clone(),
                prenom: prenom.clone(),
                ordo: ordo.clone(),
            })
        };
        if let Some(v) = e.filter(|v| *v > 0.0) {
            push(true, v);
            acc.monthly[month - 1][0] += v;
        }
        if let Some(v) = s.filter(|v| *v > 0.0) {
            push(false, v);
            acc.monthly[month - 1][1] += v;
        }
        if movement && month > acc.last_movement_month {
            acc.last_movement_month = month;
        }
    }
}

// ------------------------------------------------------------------
// StupControl's settings and catalogue
// ------------------------------------------------------------------

/// One product StupControl follows (`DrugConfig`).
#[derive(Clone, Debug, PartialEq)]
pub struct Drug {
    pub commercial_name: String,
    pub file_name: String,
    pub specialite: String,
    pub pills_per_box: i64,
    pub min_stock: i64,
}

impl Drug {
    /// `aliasKey`: the file and the header, trimmed and lowercase.
    pub fn alias_key(&self) -> String {
        format!(
            "{}|{}",
            java_trim(&self.file_name).to_lowercase(),
            java_trim(&self.specialite).to_lowercase()
        )
    }
}

/// What StupControl's `config.json` holds that the register needs.
#[derive(Clone, Debug, Default)]
pub struct Settings {
    pub data_dir: String,
    pub drugs: Vec<Drug>,
    pub patient_columns: HashMap<String, Vec<i64>>,
}

/// `Config.configDir()`: `%APPDATA%\StupControl`, else
/// `~/.config/StupControl`.
pub fn config_dir() -> PathBuf {
    match std::env::var("APPDATA") {
        Ok(a) if !java_blank(&a) => PathBuf::from(a).join("StupControl"),
        _ => dirs::home_dir()
            .unwrap_or_default()
            .join(".config")
            .join("StupControl"),
    }
}

/// The file StupControl keeps its settings in, unless one is named.
pub fn default_config() -> PathBuf {
    config_dir().join("config.json")
}

/// StupControl's settings on this post: the file named in the options,
/// else its own — when it is there.
pub fn settings_file(written: &str) -> Option<PathBuf> {
    let written = written.trim();
    let p = if written.is_empty() {
        default_config()
    } else {
        PathBuf::from(written)
    };
    p.is_file().then_some(p)
}

#[derive(serde::Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct RawDrug {
    commercial_name: Option<String>,
    file_name: Option<String>,
    specialite: Option<String>,
    min_stock: i64,
}

#[derive(serde::Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct RawConfig {
    data_dir: Option<String>,
    drugs: Option<Vec<Option<RawDrug>>>,
    patient_columns: Option<HashMap<String, Vec<i64>>>,
}

/// `Config.load(path)`, normalised as StupControl does, with the
/// units-per-box sidecar applied. A file that does not parse gives the
/// empty settings (StupControl then answers « répertoire introuvable »).
pub fn load_settings(path: &Path) -> Settings {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Settings::default();
    };
    let Ok(raw) = serde_json::from_str::<RawConfig>(&text) else {
        return Settings::default();
    };
    let t = |s: Option<String>| java_trim(&s.unwrap_or_default()).to_owned();
    let mut drugs: Vec<Drug> = raw
        .drugs
        .unwrap_or_default()
        .into_iter()
        .flatten()
        .map(|d| Drug {
            commercial_name: t(d.commercial_name),
            file_name: t(d.file_name),
            specialite: t(d.specialite),
            pills_per_box: 28,
            min_stock: d.min_stock.max(0),
        })
        .collect();
    let sidecar = path.with_file_name("unites-par-boite.json");
    if let Ok(text) = std::fs::read_to_string(&sidecar) {
        if let Ok(units) = serde_json::from_str::<HashMap<String, i64>>(&text) {
            for d in &mut drugs {
                let key = format!(
                    "{}|{}",
                    java_trim(&d.file_name).to_lowercase(),
                    java_trim(&d.specialite).to_lowercase()
                );
                if let Some(u) = units.get(&key).filter(|u| **u >= 1) {
                    d.pills_per_box = *u;
                }
            }
        }
    }
    Settings {
        data_dir: t(raw.data_dir),
        drugs,
        patient_columns: raw.patient_columns.unwrap_or_default(),
    }
}

struct CatalogEntry {
    name: String,
    commercial_name: String,
    file: String,
    units: i64,
    min_stock: i64,
}

fn catalog_key(file: &str, name: &str) -> String {
    format!(
        "{} {}",
        java_trim(file).to_lowercase(),
        norm_specialite(name)
    )
}

/// `ProductCatalog.load`: `{données}/products.toml`, one `[[produit]]`
/// per column header; two spellings of one header merged.
fn load_catalog(data_dir: &str) -> Vec<CatalogEntry> {
    if java_blank(data_dir) {
        return Vec::new();
    }
    let Ok(text) = std::fs::read_to_string(Path::new(data_dir).join("products.toml")) else {
        return Vec::new();
    };
    let Ok(parsed) = text.parse::<toml::Table>() else {
        return Vec::new();
    };
    let Some(tables) = parsed.get("produit").and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    let mut out: Vec<CatalogEntry> = Vec::new();
    let mut seen: HashMap<String, usize> = HashMap::new();
    for t in tables {
        let Some(t) = t.as_table() else {
            continue;
        };
        let string = |k: &str| -> Result<String, ()> {
            match t.get(k) {
                None => Ok(String::new()),
                Some(toml::Value::String(s)) => Ok(s.clone()),
                Some(_) => Err(()),
            }
        };
        let int = |k: &str| t.get(k).and_then(|v| v.as_integer()).unwrap_or(0);
        let (Some(toml::Value::String(name)), Ok(file), Ok(com)) =
            (t.get("nom"), string("fichier"), string("nom_commercial"))
        else {
            continue;
        };
        if java_blank(name) {
            continue;
        }
        let e = CatalogEntry {
            name: java_trim(name).to_owned(),
            commercial_name: com,
            file: file.clone(),
            units: int("unites_par_boite"),
            min_stock: int("seuil"),
        };
        // Case-insensitive, as Java's TreeMap(CASE_INSENSITIVE_ORDER).
        let key = catalog_key(&file, name).to_lowercase();
        match seen.get(&key) {
            None => {
                seen.insert(key, out.len());
                out.push(e);
            }
            Some(&i) => {
                let kept = &mut out[i];
                if java_blank(&kept.commercial_name) {
                    kept.commercial_name = e.commercial_name;
                }
                if kept.units <= 0 {
                    kept.units = e.units;
                }
                if kept.min_stock <= 0 {
                    kept.min_stock = e.min_stock;
                }
            }
        }
    }
    out
}

/// `ProductCatalog.activeDrugs`: the catalogue when it has entries, else
/// the settings' list. Two products shown under one name get their file
/// in brackets.
pub fn active_drugs(settings: &Settings) -> Vec<Drug> {
    let entries = load_catalog(&settings.data_dir);
    if entries.is_empty() {
        return settings.drugs.clone();
    }
    let label = |e: &CatalogEntry| {
        if java_blank(&e.commercial_name) {
            e.name.clone()
        } else {
            e.commercial_name.clone()
        }
    };
    let mut count: HashMap<String, usize> = HashMap::new();
    for e in &entries {
        *count.entry(label(e).to_lowercase()).or_default() += 1;
    }
    entries
        .iter()
        .map(|e| {
            let l = label(e);
            let shown = if count.get(&l.to_lowercase()).copied().unwrap_or(1) > 1 {
                let stem = if e.file.to_lowercase().ends_with(".ods") {
                    &e.file[..e.file.len() - 4]
                } else {
                    &e.file
                };
                format!("{l} ({stem})")
            } else {
                l
            };
            Drug {
                commercial_name: shown,
                file_name: e.file.clone(),
                specialite: e.name.clone(),
                pills_per_box: e.units.max(1),
                min_stock: e.min_stock.max(0),
            }
        })
        .collect()
}

/// `Config.years()`: the year folders under the data directory,
/// ascending.
pub fn years(data_dir: &str) -> Vec<i64> {
    let mut out: Vec<i64> = Vec::new();
    if java_blank(data_dir) {
        return out;
    }
    if let Ok(rd) = std::fs::read_dir(data_dir) {
        for e in rd.flatten() {
            if e.path().is_dir() {
                if let Ok(y) = e.file_name().to_string_lossy().parse::<i32>() {
                    out.push(y as i64);
                }
            }
        }
    }
    out.sort_unstable();
    out.dedup();
    out
}

fn norm_file_name(s: &str) -> String {
    java_trim(s)
        .to_lowercase()
        .split(|c: char| c.is_whitespace() || c == '\u{A0}' || c == '\u{202F}')
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// `Config.resolveFile`: `{données}/{année}/{fichier}`, or the file of
/// that folder whose name differs only by case and spacing.
pub fn resolve_file(data_dir: &str, drug: &Drug, year: i64) -> PathBuf {
    let exact = Path::new(data_dir)
        .join(year.to_string())
        .join(&drug.file_name);
    if exact.exists() {
        return exact;
    }
    let dir = Path::new(data_dir).join(year.to_string());
    if dir.is_dir() {
        let want = norm_file_name(&drug.file_name);
        if let Ok(rd) = std::fs::read_dir(&dir) {
            for e in rd.flatten() {
                if e.path().is_file() && norm_file_name(&e.file_name().to_string_lossy()) == want {
                    return e.path();
                }
            }
        }
    }
    exact
}

// ------------------------------------------------------------------
// Stock: StockCalculator
// ------------------------------------------------------------------

/// `ReadSession`: one read per file for one operation.
#[derive(Default)]
pub struct Session {
    states: HashMap<PathBuf, Option<Result<FileData, ()>>>,
}

impl Session {
    fn state(&mut self, path: &Path) -> &Option<Result<FileData, ()>> {
        self.states.entry(path.to_path_buf()).or_insert_with(|| {
            if !path.exists() {
                None
            } else {
                Some(read_file(path).map_err(|_| ()))
            }
        })
    }

    pub fn read(&mut self, path: &Path) -> Option<&FileData> {
        match self.state(path) {
            Some(Ok(d)) => Some(d),
            _ => None,
        }
    }

    pub fn unreadable(&mut self, path: &Path) -> bool {
        matches!(self.state(path), Some(Err(())))
    }
}

fn sum_net(m: &[[f64; 2]; 12]) -> f64 {
    m.iter().map(|x| x[0] - x[1]).sum()
}

fn latest_year_with(
    data_dir: &str,
    drug: &Drug,
    years: &[i64],
    from: isize,
    session: &mut Session,
) -> isize {
    let mut i = from;
    while i >= 0 {
        let f = resolve_file(data_dir, drug, years[i as usize]);
        if session
            .read(&f)
            .is_some_and(|fd| fd.monthly(&drug.specialite).is_some())
        {
            return i;
        }
        i -= 1;
    }
    -1
}

fn carry_in_before(
    data_dir: &str,
    drug: &Drug,
    year: i64,
    years: &[i64],
    session: &mut Session,
) -> f64 {
    let mut net_after = 0.0;
    for &y in years.iter().rev() {
        if y >= year {
            continue;
        }
        let f = resolve_file(data_dir, drug, y);
        let Some(fd) = session.read(&f) else {
            continue;
        };
        let Some(monthly) = fd.monthly(&drug.specialite) else {
            continue;
        };
        if let Some(b) = fd.current_balance(&drug.specialite) {
            return b + net_after;
        }
        net_after += sum_net(&monthly);
    }
    net_after
}

/// `StockCalculator.currentStock`.
pub fn current_stock(data_dir: &str, drug: &Drug, session: &mut Session) -> f64 {
    let years = years(data_dir);
    let latest = latest_year_with(data_dir, drug, &years, years.len() as isize - 1, session);
    if latest < 0 {
        return 0.0;
    }
    let year = years[latest as usize];
    let f = resolve_file(data_dir, drug, year);
    let (this_net, own) = match session.read(&f) {
        Some(fd) => (
            fd.monthly(&drug.specialite)
                .map(|m| sum_net(&m))
                .unwrap_or(0.0),
            fd.current_balance(&drug.specialite),
        ),
        None => (0.0, None),
    };
    let prev = latest_year_with(data_dir, drug, &years, latest - 1, session);
    if prev < 0 {
        return own.unwrap_or(this_net);
    }
    carry_in_before(data_dir, drug, year, &years, session) + this_net
}

// ------------------------------------------------------------------
// The document tree, for writing
// ------------------------------------------------------------------

#[derive(Clone, Debug)]
enum Node {
    El(El),
    Text(String),
    Raw(String),
}

#[derive(Clone, Debug)]
struct El {
    name: String,
    attrs: Vec<(String, String)>,
    kids: Vec<Node>,
}

impl El {
    fn new(name: String) -> El {
        El {
            name,
            attrs: Vec::new(),
            kids: Vec::new(),
        }
    }

    fn get(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }

    fn set(&mut self, name: &str, value: &str) {
        match self.attrs.iter_mut().find(|(k, _)| k == name) {
            Some(a) => a.1 = value.to_owned(),
            None => self.attrs.push((name.to_owned(), value.to_owned())),
        }
    }

    fn remove(&mut self, name: &str) {
        self.attrs.retain(|(k, _)| k != name);
    }
}

fn parse_dom(xml: &[u8]) -> Option<Vec<Node>> {
    use quick_xml::events::Event;
    let mut r = quick_xml::Reader::from_reader(xml);
    r.config_mut().trim_text(false);
    let mut stack: Vec<El> = Vec::new();
    let mut top: Vec<Node> = Vec::new();
    fn push(stack: &mut [El], top: &mut Vec<Node>, n: Node) {
        match stack.last_mut() {
            Some(p) => p.kids.push(n),
            None => top.push(n),
        }
    }
    let element = |e: &quick_xml::events::BytesStart| -> Option<El> {
        let mut el = El::new(String::from_utf8(e.name().as_ref().to_vec()).ok()?);
        for a in e.attributes() {
            let a = a.ok()?;
            el.attrs.push((
                String::from_utf8(a.key.as_ref().to_vec()).ok()?,
                a.unescape_value().ok()?.into_owned(),
            ));
        }
        Some(el)
    };
    loop {
        match r.read_event().ok()? {
            Event::Start(e) => stack.push(element(&e)?),
            Event::Empty(e) => {
                let el = element(&e)?;
                push(&mut stack, &mut top, Node::El(el));
            }
            Event::End(_) => {
                let el = stack.pop()?;
                push(&mut stack, &mut top, Node::El(el));
            }
            Event::Text(t) => {
                let s = t.unescape().ok()?.into_owned();
                push(&mut stack, &mut top, Node::Text(s));
            }
            Event::CData(t) => {
                let s = String::from_utf8(t.into_inner().into_owned()).ok()?;
                push(&mut stack, &mut top, Node::Text(s));
            }
            Event::Decl(d) => {
                let s = format!("<?{}?>", String::from_utf8_lossy(&d));
                push(&mut stack, &mut top, Node::Raw(s));
            }
            Event::PI(p) => {
                let s = format!("<?{}?>", String::from_utf8_lossy(&p));
                push(&mut stack, &mut top, Node::Raw(s));
            }
            Event::Comment(c) => {
                let s = format!("<!--{}-->", String::from_utf8_lossy(&c));
                push(&mut stack, &mut top, Node::Raw(s));
            }
            Event::DocType(d) => {
                let s = format!("<!DOCTYPE {}>", String::from_utf8_lossy(&d));
                push(&mut stack, &mut top, Node::Raw(s));
            }
            Event::Eof => break,
        }
    }
    stack.is_empty().then_some(top)
}

fn escape_into(out: &mut String, s: &str, attr: bool) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' if attr => out.push_str("&quot;"),
            '\n' if attr => out.push_str("&#10;"),
            '\t' if attr => out.push_str("&#9;"),
            '\r' => out.push_str("&#13;"),
            c => out.push(c),
        }
    }
}

fn serialize(nodes: &[Node], out: &mut String) {
    for n in nodes {
        match n {
            Node::Text(t) => escape_into(out, t, false),
            Node::Raw(r) => out.push_str(r),
            Node::El(e) => {
                out.push('<');
                out.push_str(&e.name);
                for (k, v) in &e.attrs {
                    out.push(' ');
                    out.push_str(k);
                    out.push_str("=\"");
                    escape_into(out, v, true);
                    out.push('"');
                }
                if e.kids.is_empty() {
                    out.push_str("/>");
                } else {
                    out.push('>');
                    serialize(&e.kids, out);
                    out.push_str("</");
                    out.push_str(&e.name);
                    out.push('>');
                }
            }
        }
    }
}

/// The prefixes the document gives the ODF namespaces.
#[derive(Clone)]
struct Ns {
    table: String,
    office: String,
    text: String,
    calcext: Option<String>,
}

impl Ns {
    fn of(root: &El) -> Option<Ns> {
        let prefix = |uri: &str| {
            root.attrs
                .iter()
                .find(|(k, v)| k.starts_with("xmlns:") && v == uri)
                .map(|(k, _)| k["xmlns:".len()..].to_owned())
        };
        Some(Ns {
            table: prefix("urn:oasis:names:tc:opendocument:xmlns:table:1.0")?,
            office: prefix("urn:oasis:names:tc:opendocument:xmlns:office:1.0")?,
            text: prefix("urn:oasis:names:tc:opendocument:xmlns:text:1.0")?,
            calcext: prefix("urn:org:documentfoundation:names:experimental:calc:xmlns:calcext:1.0"),
        })
    }

    fn t(&self, local: &str) -> String {
        format!("{}:{local}", self.table)
    }

    fn o(&self, local: &str) -> String {
        format!("{}:{local}", self.office)
    }

    fn x(&self, local: &str) -> String {
        format!("{}:{local}", self.text)
    }
}

/// simple-odf's `getDisplayText` (`TextExtractor`): each paragraph ends
/// with a line break, the last one dropped; `text:s`, `text:tab` and
/// `text:line-break` spelled out.
fn display_text(cell: &El, ns: &Ns) -> String {
    fn walk(el: &El, ns: &Ns, out: &mut String, line: &mut bool) {
        for k in &el.kids {
            match k {
                Node::Text(t) => out.push_str(t),
                Node::Raw(_) => {}
                Node::El(e) => {
                    let n = e.name.as_str();
                    if n == ns.x("p") || n == ns.x("h") {
                        walk(e, ns, out, line);
                        out.push('\n');
                        *line = true;
                    } else if n == ns.x("s") {
                        let c = e
                            .get(&ns.x("c"))
                            .and_then(|v| v.trim().parse::<usize>().ok())
                            .unwrap_or(1);
                        out.extend(std::iter::repeat_n(' ', c));
                    } else if n == ns.x("tab") {
                        out.push('\t');
                    } else if n == ns.x("line-break") {
                        out.push('\n');
                    } else {
                        walk(e, ns, out, line);
                    }
                }
            }
        }
    }
    let mut out = String::new();
    let mut line = false;
    walk(cell, ns, &mut out, &mut line);
    if line {
        out.pop();
    }
    out
}

/// A cell's number when it is a `float` cell with a value
/// (`getDoubleValue`, `SheetRows.numeric`).
fn float_value(cell: &El, ns: &Ns) -> Option<f64> {
    if cell.get(&ns.o("value-type")) != Some("float") {
        return None;
    }
    cell.get(&ns.o("value")).and_then(java_parse_double)
}

/// `StupRegister.hasNumber`: a numeric type, value or not.
fn numeric_type(cell: &El, ns: &Ns) -> bool {
    matches!(
        cell.get(&ns.o("value-type")),
        Some("float" | "currency" | "percentage")
    )
}

fn clear_value(ns: &Ns, cell: &mut El) {
    for a in [
        "value",
        "value-type",
        "date-value",
        "time-value",
        "boolean-value",
        "string-value",
        "currency",
    ] {
        cell.remove(&ns.o(a));
    }
    if let Some(c) = &ns.calcext {
        cell.remove(&format!("{c}:value-type"));
    }
}

/// The cell's paragraphs replaced by one holding `text`; a note stays.
fn set_paragraph(ns: &Ns, cell: &mut El, text: &str) {
    let annotation = ns.o("annotation");
    cell.kids
        .retain(|k| matches!(k, Node::El(e) if e.name == annotation));
    let mut p = El::new(ns.x("p"));
    if !text.is_empty() {
        p.kids.push(Node::Text(text.to_owned()));
    }
    cell.kids.push(Node::El(p));
}

struct RowRef {
    path: Vec<usize>,
    start: usize,
    rep: usize,
}

/// One month sheet, addressed by (column, row) as simple-odf does.
struct Sheet<'a> {
    el: &'a mut El,
    ns: Ns,
    rows: Vec<RowRef>,
}

fn rep_of(el: &El, attr: &str) -> usize {
    el.get(attr)
        .and_then(|v| java_trim(v).parse::<i64>().ok())
        .map(|v| v.max(1) as usize)
        .unwrap_or(1)
}

impl<'a> Sheet<'a> {
    fn new(el: &'a mut El, ns: Ns) -> Sheet<'a> {
        let mut s = Sheet {
            el,
            ns,
            rows: Vec::new(),
        };
        s.index();
        s
    }

    fn name(&self) -> String {
        self.el.get(&self.ns.t("name")).unwrap_or("").to_owned()
    }

    fn index(&mut self) {
        fn walk(el: &El, ns: &Ns, path: &mut Vec<usize>, rows: &mut Vec<RowRef>, next: &mut usize) {
            for (i, k) in el.kids.iter().enumerate() {
                let Node::El(e) = k else {
                    continue;
                };
                path.push(i);
                if e.name == ns.t("table-row") {
                    let rep = rep_of(e, &ns.t("number-rows-repeated"));
                    rows.push(RowRef {
                        path: path.clone(),
                        start: *next,
                        rep,
                    });
                    *next += rep;
                } else if e.name == ns.t("table-header-rows")
                    || e.name == ns.t("table-rows")
                    || e.name == ns.t("table-row-group")
                {
                    walk(e, ns, path, rows, next);
                }
                path.pop();
            }
        }
        let mut rows = Vec::new();
        let mut next = 0;
        walk(self.el, &self.ns, &mut Vec::new(), &mut rows, &mut next);
        self.rows = rows;
    }

    fn row_count(&self) -> usize {
        self.rows.last().map(|r| r.start + r.rep).unwrap_or(0)
    }

    fn column_count(&self) -> usize {
        fn walk(el: &El, ns: &Ns) -> usize {
            let mut n = 0;
            for k in &el.kids {
                let Node::El(e) = k else {
                    continue;
                };
                if e.name == ns.t("table-column") {
                    n += rep_of(e, &ns.t("number-columns-repeated"));
                } else if e.name == ns.t("table-columns")
                    || e.name == ns.t("table-header-columns")
                    || e.name == ns.t("table-column-group")
                {
                    n += walk(e, ns);
                }
            }
            n
        }
        walk(self.el, &self.ns)
    }

    fn at_path(&self, path: &[usize]) -> &El {
        let mut el: &El = self.el;
        for &i in path {
            if let Node::El(e) = &el.kids[i] {
                el = e;
            }
        }
        el
    }

    fn at_path_mut(&mut self, path: &[usize]) -> &mut El {
        let mut el: &mut El = self.el;
        for &i in path {
            el = match &mut el.kids[i] {
                Node::El(e) => e,
                _ => unreachable!("a row path names elements"),
            };
        }
        el
    }

    fn row_slot(&self, r: usize) -> Option<usize> {
        let i = self.rows.partition_point(|x| x.start + x.rep <= r);
        (i < self.rows.len() && self.rows[i].start <= r).then_some(i)
    }

    fn is_cell(&self, e: &El) -> bool {
        e.name == self.ns.t("table-cell") || e.name == self.ns.t("covered-table-cell")
    }

    /// The cell at (c, r), absent beyond the row's last cell.
    fn cell(&self, c: usize, r: usize) -> Option<&El> {
        let slot = self.row_slot(r)?;
        let row = self.at_path(&self.rows[slot].path);
        let mut next = 0;
        for k in &row.kids {
            let Node::El(e) = k else {
                continue;
            };
            if !self.is_cell(e) {
                continue;
            }
            let rep = rep_of(e, &self.ns.t("number-columns-repeated"));
            if c < next + rep {
                return Some(e);
            }
            next += rep;
        }
        None
    }

    fn text(&self, c: i64, r: usize) -> String {
        if c < 0 {
            return String::new();
        }
        self.cell(c as usize, r)
            .map(|e| display_text(e, &self.ns))
            .unwrap_or_default()
    }

    fn has_text(&self, c: i64, r: usize) -> bool {
        c >= 0 && !java_blank(&self.text(c, r))
    }

    fn float(&self, c: usize, r: usize) -> Option<f64> {
        self.cell(c, r).and_then(|e| float_value(e, &self.ns))
    }

    fn numeric_type(&self, c: i64, r: usize) -> bool {
        c >= 0
            && self
                .cell(c as usize, r)
                .is_some_and(|e| numeric_type(e, &self.ns))
    }

    /// The cell at (c, r) on its own: a repeated row or cell split around
    /// it, missing cells added (simple-odf's `splitRepeatedCells`).
    fn cell_mut(&mut self, c: usize, r: usize) -> &mut El {
        let rows_attr = self.ns.t("number-rows-repeated");
        let cols_attr = self.ns.t("number-columns-repeated");
        let slot = self.row_slot(r).unwrap_or(self.rows.len() - 1);
        let RowRef { path, start, rep } = &self.rows[slot];
        let (path, start, rep) = (path.clone(), *start, *rep);
        if rep > 1 {
            let (parent_path, idx) = path.split_at(path.len() - 1);
            let idx = idx[0];
            let parent = self.at_path_mut(parent_path);
            let Node::El(orig) = parent.kids[idx].clone() else {
                unreachable!()
            };
            let mut pieces = Vec::new();
            let before = r - start;
            let after = start + rep - r - 1;
            let piece = |n: usize| {
                let mut e = orig.clone();
                if n > 1 {
                    e.set(&rows_attr, &n.to_string());
                } else {
                    e.remove(&rows_attr);
                }
                Node::El(e)
            };
            if before > 0 {
                pieces.push(piece(before));
            }
            pieces.push(piece(1));
            if after > 0 {
                pieces.push(piece(after));
            }
            let target = idx + usize::from(before > 0);
            parent.kids.splice(idx..=idx, pieces);
            let mut p = parent_path.to_vec();
            p.push(target);
            self.index();
            return self.cell_in_row_mut(&p, c, &cols_attr);
        }
        self.cell_in_row_mut(&path, c, &cols_attr)
    }

    fn cell_in_row_mut(&mut self, row_path: &[usize], c: usize, cols_attr: &str) -> &mut El {
        let cell_name = self.ns.t("table-cell");
        let covered = self.ns.t("covered-table-cell");
        let row = self.at_path_mut(row_path);
        let mut next = 0;
        let mut found: Option<(usize, usize, usize)> = None;
        for (i, k) in row.kids.iter().enumerate() {
            let Node::El(e) = k else {
                continue;
            };
            if e.name != cell_name && e.name != covered {
                continue;
            }
            let rep = rep_of(e, cols_attr);
            if c < next + rep {
                found = Some((i, next, rep));
                break;
            }
            next += rep;
        }
        let (idx, start, rep) = match found {
            Some(f) => f,
            None => {
                while next <= c {
                    row.kids.push(Node::El(El::new(cell_name.clone())));
                    next += 1;
                }
                (row.kids.len() - 1, c, 1)
            }
        };
        if rep > 1 {
            let Node::El(orig) = row.kids[idx].clone() else {
                unreachable!()
            };
            let before = c - start;
            let after = start + rep - c - 1;
            let mut pieces = Vec::new();
            let piece = |n: usize| {
                let mut e = orig.clone();
                if n > 1 {
                    e.set(cols_attr, &n.to_string());
                } else {
                    e.remove(cols_attr);
                }
                Node::El(e)
            };
            if before > 0 {
                pieces.push(piece(before));
            }
            pieces.push(piece(1));
            if after > 0 {
                pieces.push(piece(after));
            }
            let target = idx + usize::from(before > 0);
            row.kids.splice(idx..=idx, pieces);
            let Node::El(e) = &mut row.kids[target] else {
                unreachable!()
            };
            return e;
        }
        let Node::El(e) = &mut row.kids[idx] else {
            unreachable!()
        };
        e
    }

    /// `setDoubleValue`: a float, its value and its text as Java writes
    /// them (`28.0`).
    fn set_double(&mut self, c: usize, r: usize, v: f64) {
        let ns = self.ns.clone();
        let shown = java_double(v);
        let cell = self.cell_mut(c, r);
        clear_value(&ns, cell);
        cell.set(&ns.o("value-type"), "float");
        cell.set(&ns.o("value"), &shown);
        if let Some(cx) = &ns.calcext {
            cell.set(&format!("{cx}:value-type"), "float");
        }
        set_paragraph(&ns, cell, &shown);
    }

    /// `setStringValue`.
    fn set_string(&mut self, c: usize, r: usize, v: &str) {
        let ns = self.ns.clone();
        let cell = self.cell_mut(c, r);
        clear_value(&ns, cell);
        cell.set(&ns.o("value-type"), "string");
        cell.set(&ns.o("string-value"), v);
        if let Some(cx) = &ns.calcext {
            cell.set(&format!("{cx}:value-type"), "string");
        }
        set_paragraph(&ns, cell, v);
    }

    fn set_formula(&mut self, c: usize, r: usize, f: &str) {
        let name = self.ns.t("formula");
        self.cell_mut(c, r).set(&name, f);
    }

    /// `SheetRows.clear`: no value, no type, no formula, no content.
    fn clear(&mut self, c: usize, r: usize) {
        let ns = self.ns.clone();
        let cell = self.cell_mut(c, r);
        clear_value(&ns, cell);
        cell.remove(&ns.t("formula"));
        cell.kids.clear();
    }

    /// `Table.appendRow`: a row after the last, styled like it, empty.
    /// Returns its index.
    fn append_row(&mut self) -> usize {
        let index = self.row_count();
        let ns = self.ns.clone();
        let named = ns.t("named-expressions");
        let at = match self.el.kids.iter().rposition(|k| matches!(k, Node::El(_))) {
            Some(i) if matches!(&self.el.kids[i], Node::El(e) if e.name == named) => i,
            _ => self.el.kids.len(),
        };
        let last_row = at.checked_sub(1).and_then(|i| match &self.el.kids[i] {
            Node::El(e) if e.name == ns.t("table-row") => Some(e.clone()),
            _ => None,
        });
        let row = match last_row {
            Some(mut row) => {
                row.remove(&ns.t("number-rows-repeated"));
                for k in &mut row.kids {
                    if let Node::El(cell) = k {
                        if cell.name == ns.t("covered-table-cell") {
                            cell.name = ns.t("table-cell");
                        }
                        cell.remove(&ns.t("number-columns-spanned"));
                        cell.remove(&ns.t("number-rows-spanned"));
                        clear_value(&ns, cell);
                        cell.remove(&ns.t("formula"));
                        cell.kids.clear();
                    }
                }
                row
            }
            None => {
                let mut row = El::new(ns.t("table-row"));
                let mut cell = El::new(ns.t("table-cell"));
                let n = self.column_count().max(1);
                if n > 1 {
                    cell.set(&ns.t("number-columns-repeated"), &n.to_string());
                }
                row.kids.push(Node::El(cell));
                row
            }
        };
        self.el.kids.insert(at, Node::El(row));
        self.index();
        index
    }

    /// `LayoutDetector.detect(Table)`: the header window read row by row.
    fn detect(&self) -> Option<Layout> {
        let cols = self.column_count();
        let rows = self.row_count().min(MAX_HEADER_SCAN + 1);
        let mut window = Vec::with_capacity(rows);
        for r in 0..rows {
            let mut texts = RowTexts::new();
            if let Some(slot) = self.row_slot(r) {
                let row = self.at_path(&self.rows[slot].path);
                let mut next = 0;
                for k in &row.kids {
                    let Node::El(e) = k else {
                        continue;
                    };
                    if !self.is_cell(e) {
                        continue;
                    }
                    let rep = rep_of(e, &self.ns.t("number-columns-repeated"));
                    if next < cols {
                        let t = display_text(e, &self.ns);
                        if !t.is_empty() {
                            for c in next..(next + rep).min(cols) {
                                texts.insert(c, t.clone());
                            }
                        }
                    }
                    next += rep;
                }
            }
            window.push(texts);
        }
        let index: Vec<usize> = (0..rows).collect();
        best_layout(cols, &window, &index)
    }
}

// ------------------------------------------------------------------
// Writing: StupRegister and SheetRows
// ------------------------------------------------------------------

/// Why a line did not go into the workbook; `code` is what the bridge
/// named it (`verrou`, `plein`, `colonnes`, `specialite`…).
#[derive(Debug, Clone, PartialEq)]
pub struct Fail {
    pub code: &'static str,
    pub msg: String,
}

fn fail(code: &'static str, msg: String) -> Fail {
    Fail { code, msg }
}

const TOTALS_SCAN_LIMIT: usize = 5000;
const MAX_SPAN: usize = 5000;

fn find_totals_row(t: &Sheet, lay: &Layout) -> Option<usize> {
    let rows = t.row_count().min(TOTALS_SCAN_LIMIT);
    (lay.first_data_row..rows).find(|&r| {
        label_cols(lay)
            .iter()
            .any(|&c| c >= 0 && is_totals_label(&t.text(c, r)))
    })
}

/// `StupRegister.rowHasMovement`: a name, a date, or a number typed in
/// an E or S column.
fn register_row_has_movement(t: &Sheet, lay: &Layout, r: usize) -> bool {
    t.has_text(lay.nom, r)
        || t.has_text(lay.date, r)
        || lay
            .products
            .iter()
            .any(|p| t.numeric_type(p.e as i64, r) || t.numeric_type(p.s as i64, r))
}

/// `SheetRows.rowHasMovement`: an E or S float.
fn rows_has_movement(t: &Sheet, lay: &Layout, r: usize) -> bool {
    lay.products
        .iter()
        .any(|p| t.float(p.e, r).is_some() || t.float(p.s, r).is_some())
}

fn first_data_row_skipping_header_copies(t: &Sheet, lay: &Layout) -> usize {
    let mut first = lay.first_data_row;
    let rows = t.row_count();
    for _ in 0..4 {
        if first + 1 >= rows || !is_sub_header_copy(t, lay, first) {
            break;
        }
        first += 1;
    }
    first
}

fn is_sub_header_copy(t: &Sheet, lay: &Layout, r: usize) -> bool {
    let Some(p) = lay.products.first() else {
        return false;
    };
    if rows_has_movement(t, lay, r) {
        return false;
    }
    is_entry(&norm(&t.text(p.e as i64, r))) && is_exit(&norm(&t.text(p.s as i64, r)))
}

fn choose_write_row(t: &mut Sheet, lay: &Layout, audit: &Audit) -> Result<usize, Fail> {
    let Some(totals) = find_totals_row(t, lay) else {
        if t.row_count() > TOTALS_SCAN_LIMIT {
            audit.log(
                "TOTAUX_HORS_SCAN",
                &format!(
                    "sheet={} | rowCount={} | TOTAUX introuvable dans les {} premières lignes — ajout en fin de feuille",
                    t.name(),
                    t.row_count(),
                    TOTALS_SCAN_LIMIT
                ),
            );
        }
        return Ok(t.append_row());
    };
    let first = first_data_row_skipping_header_copies(t, lay);
    let mut last: Option<usize> = None;
    for r in first..totals {
        if register_row_has_movement(t, lay, r) {
            last = Some(r);
        }
    }
    let write = match last {
        Some(l) => l + 1,
        None => {
            let opening = lay
                .products
                .iter()
                .any(|p| p.b.is_some_and(|b| t.numeric_type(b as i64, first)));
            if opening {
                first + 1
            } else {
                first
            }
        }
    };
    if write >= totals {
        return Err(fail("plein", trf("cls_err_full", t.name())));
    }
    Ok(write)
}

/// A captured cell: a number or a text.
#[derive(Clone)]
enum Val {
    Num(f64),
    Text(String),
}

struct Captured {
    has_movement: bool,
    blank_nom: bool,
    cells: Vec<(usize, Val)>,
}

/// `SheetRows.removeEmptyRowsInPlace`: the span between the opening row
/// and TOTAUX rewritten with its movements packed at the top, bilans
/// recomputed. True when a row was freed.
fn remove_empty_rows_in_place(t: &mut Sheet, lay: &Layout) -> bool {
    let first = first_data_row_skipping_header_copies(t, lay);
    let totals = find_totals_row(t, lay);
    let max_col = lay.max_col();
    let is_opening = |t: &Sheet, r: usize| {
        !(rows_has_movement(t, lay, r) || t.has_text(lay.nom, r) || t.has_text(lay.date, r))
            && lay
                .products
                .iter()
                .any(|p| p.b.is_some_and(|b| t.float(b, r).is_some()))
    };
    let opening = is_opening(t, first);
    let first_move = if opening { first + 1 } else { first };
    let end = match totals {
        Some(tr) => tr,
        None => {
            let mut last = first_move as isize - 1;
            let cap = (first_move + MAX_SPAN).min(t.row_count());
            for r in first_move..cap {
                if rows_has_movement(t, lay, r) || t.has_text(lay.nom, r) || t.has_text(lay.date, r)
                {
                    last = r as isize;
                }
            }
            (last + 1).max(0) as usize
        }
    }
    .max(first_move);
    let b_cols: HashSet<usize> = lay.products.iter().filter_map(|p| p.b).collect();
    let mut rows: Vec<Captured> = Vec::new();
    for r in first_move..end {
        let mut cells = Vec::new();
        for c in 0..=max_col {
            if let Some(v) = t.float(c, r) {
                cells.push((c, Val::Num(v)));
            } else {
                let s = t.text(c as i64, r);
                if !s.is_empty() {
                    cells.push((c, Val::Text(s)));
                }
            }
        }
        rows.push(Captured {
            has_movement: rows_has_movement(t, lay, r),
            blank_nom: !t.has_text(lay.nom, r) && !t.has_text(lay.date, r),
            cells,
        });
    }
    let before = rows.len();
    rows.retain(|c| c.has_movement || !c.blank_nom);
    let removed = rows.len() != before;
    for r in first_move..end {
        for c in 0..=max_col {
            t.clear(c, r);
        }
    }
    for (i, cap) in rows.iter().enumerate() {
        for (c, v) in &cap.cells {
            if b_cols.contains(c) {
                continue;
            }
            match v {
                Val::Num(n) => t.set_double(*c, first_move + i, *n),
                Val::Text(s) => t.set_string(*c, first_move + i, s),
            }
        }
    }
    for p in &lay.products {
        let Some(bc) = p.b else {
            continue;
        };
        let mut running = if opening { t.float(bc, first) } else { None };
        let mut prev_row1: isize = if opening { first as isize + 1 } else { -1 };
        for (i, cap) in rows.iter().enumerate() {
            let ri = first_move + i;
            let num = |c: usize| match cap.cells.iter().find(|(k, _)| *k == c) {
                Some((_, Val::Num(n))) => *n,
                _ => 0.0,
            };
            let (e, s) = (num(p.e), num(p.s));
            if e <= 0.0 && s <= 0.0 {
                continue;
            }
            let value = running.unwrap_or(0.0) - s + e;
            running = Some(value);
            let ods_row = ri + 1;
            let prev_ref = if prev_row1 >= 1 {
                prev_row1 as usize
            } else {
                ods_row - 1
            };
            t.set_double(bc, ri, value);
            t.set_formula(bc, ri, &balance_formula(bc, p.s, p.e, prev_ref, ods_row));
            prev_row1 = ods_row as isize;
        }
    }
    removed
}

/// `CellRef.colLetters`: 0 is `A`, 26 is `AA`.
fn col_letters(col: usize) -> String {
    let mut n = col + 1;
    let mut out = Vec::new();
    while n > 0 {
        out.push(b'A' + ((n - 1) % 26) as u8);
        n = (n - 1) / 26;
    }
    out.reverse();
    String::from_utf8(out).unwrap_or_default()
}

/// The running balance, as the template writes it:
/// `of:=IFERROR(INT([.G14]); 0)-[.F17]+[.E17]`.
fn balance_formula(b: usize, s: usize, e: usize, prev: usize, row: usize) -> String {
    format!(
        "of:=IFERROR(INT([.{}{prev}]); 0)-[.{}{row}]+[.{}{row}]",
        col_letters(b),
        col_letters(s),
        col_letters(e)
    )
}

/// One line for the workbook (`StupRegister.addEntry`'s arguments).
pub struct Write<'a> {
    pub sheet: &'a str,
    pub specialite: &'a str,
    pub entry: bool,
    pub nom: &'a str,
    pub prenom: &'a str,
    /// `JJ/MM/AA`, as the register writes it.
    pub date_text: &'a str,
    pub ordo: &'a str,
    pub quantity: f64,
    /// `[nom, prénom, date, ordonnancier]` chosen in StupControl, or
    /// none to detect them.
    pub columns: Option<&'a [i64]>,
}

/// `doAddEntry`'s edit of the sheet, in memory.
fn add_to_sheet(t: &mut Sheet, w: &Write, audit: &Audit, file: &str) -> Result<(), Fail> {
    let lay = t
        .detect()
        .ok_or_else(|| fail("ecriture", trf("cls_err_no_specialite", t.name())))?;
    let target = lay.product(w.specialite).cloned().ok_or_else(|| {
        fail(
            "specialite",
            trn("cls_err_specialite", &[&w.specialite, &w.sheet]),
        )
    })?;
    let Some(b_col) = target.b else {
        return Err(fail(
            "ecriture",
            trn("cls_err_no_bilan", &[&w.specialite, &w.sheet]),
        ));
    };
    let (mut nom, mut prenom, mut date, mut ordo) = (lay.nom, lay.prenom, lay.date, lay.ordo);
    if let Some(c) = w.columns.filter(|c| c.len() == 4) {
        (nom, prenom, date, ordo) = (c[0], c[1], c[2], c[3]);
    }
    if nom < 0 || date < 0 {
        return Err(fail("colonnes", trf("cls_err_columns", w.sheet)));
    }
    let row = match choose_write_row(t, &lay, audit) {
        Ok(r) => r,
        Err(full) => {
            if !remove_empty_rows_in_place(t, &lay) {
                return Err(full);
            }
            audit.log(
                "NETTOYAGE_AUTO",
                &format!(
                    "file={file} | sheet={} | lignes vides récupérées avant l'ajout",
                    w.sheet
                ),
            );
            choose_write_row(t, &lay, audit)?
        }
    };
    let ods_row = row + 1;
    for (c, v) in [(nom, w.nom), (prenom, w.prenom), (date, w.date_text)] {
        if c >= 0 && !v.is_empty() {
            t.set_string(c as usize, row, v);
        }
    }
    if !java_blank(w.ordo) && ordo >= 0 {
        let plain = digits_only(w.ordo)
            && w.ordo.len() <= 15
            && (w.ordo.len() == 1 || !w.ordo.starts_with('0'));
        match plain.then(|| w.ordo.parse::<f64>().ok()).flatten() {
            Some(n) => t.set_double(ordo as usize, row, n),
            None => t.set_string(ordo as usize, row, w.ordo),
        }
    }
    let value_col = if w.entry { target.e } else { target.s };
    t.set_double(value_col, row, w.quantity);
    let (e, s) = if w.entry {
        (w.quantity, 0.0)
    } else {
        (0.0, w.quantity)
    };
    let prev = (0..row).rev().find(|&r| t.float(b_col, r).is_some());
    let prev_bal = prev.and_then(|r| t.float(b_col, r)).unwrap_or(0.0);
    let prev_ods = prev.map(|r| r + 1).unwrap_or(ods_row - 1);
    t.set_double(b_col, row, prev_bal - s + e);
    t.set_formula(
        b_col,
        row,
        &balance_formula(b_col, target.s, target.e, prev_ods, ods_row),
    );
    Ok(())
}

fn lock_file(path: &Path) -> PathBuf {
    path.with_file_name(format!(".~lock.{}#", file_name(path)))
}

fn check_lock(path: &Path) -> Result<(), Fail> {
    if lock_file(path).exists() {
        return Err(fail("verrou", trf("cls_err_locked", file_name(path))));
    }
    Ok(())
}

fn stamp(path: &Path) -> Option<(std::time::SystemTime, u64)> {
    let m = std::fs::metadata(path).ok()?;
    Some((m.modified().ok()?, m.len()))
}

/// The local time, from SQLite's clock — the one clock of the
/// application: `(AAAA-MM-JJ HH:MM:SS, AAAAMMJJ-HHMMSS-mmm)`.
fn local_now() -> (String, String) {
    let q = || -> rusqlite::Result<(String, String)> {
        let c = rusqlite::Connection::open_in_memory()?;
        c.query_row(
            "SELECT strftime('%Y-%m-%d %H:%M:%S', t), \
                    strftime('%Y%m%d-%H%M%S-', t) || substr(strftime('%f', t), 4, 3) \
             FROM (SELECT strftime('%Y-%m-%d %H:%M:%f', 'now', 'localtime') AS t)",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
    };
    q().unwrap_or_default()
}

/// StupControl's audit log, `audit.log` beside its settings.
pub struct Audit {
    pub file: PathBuf,
}

impl Audit {
    fn log(&self, action: &str, detail: &str) {
        use std::io::Write;
        let user = std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_else(|_| "?".into());
        let eol = if cfg!(windows) { "\r\n" } else { "\n" };
        let line = format!(
            "{} | user={user} | {action}{}{eol}",
            local_now().0,
            if detail.is_empty() {
                String::new()
            } else {
                format!(" | {detail}")
            }
        );
        if let Some(dir) = self.file.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if std::fs::metadata(&self.file).is_ok_and(|m| m.len() > 5 * 1024 * 1024) {
            let mut old = self.file.clone().into_os_string();
            old.push(".1");
            let _ = std::fs::rename(&self.file, old);
        }
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.file)
        {
            let _ = f.write_all(line.as_bytes());
        }
    }
}

/// `Backup.backup`: a copy in `.backups/` beside the file, ten kept.
fn backup(path: &Path) -> bool {
    static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    if !path.exists() {
        return true;
    }
    let Some(parent) = path.parent() else {
        return false;
    };
    let dir = parent.join(".backups");
    if std::fs::create_dir_all(&dir).is_err() {
        return false;
    }
    let prefix = format!("{}.", file_name(path));
    let seq = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let target = dir.join(format!("{prefix}{}-{seq:06}.bak", local_now().1));
    if std::fs::copy(path, &target).is_err() {
        return false;
    }
    let mut kept: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map(|rd| {
            rd.flatten()
                .map(|e| e.path())
                .filter(|p| {
                    let n = file_name(p);
                    n.starts_with(&prefix) && n.ends_with(".bak")
                })
                .collect()
        })
        .unwrap_or_default();
    kept.sort();
    let extra = kept.len().saturating_sub(10);
    for p in &kept[..extra] {
        let _ = std::fs::remove_file(p);
    }
    true
}

/// `StupRegister.addEntry`: one line into one workbook, with every
/// safeguard StupControl has, and its line in the audit log.
pub fn add_entry(path: &Path, w: &Write, audit: &Audit) -> Result<(), Fail> {
    let loc = format!(
        "{}{}",
        path.parent()
            .and_then(|p| p.file_name())
            .map(|n| format!("{}/", n.to_string_lossy()))
            .unwrap_or_default(),
        file_name(path)
    );
    let kind = if w.entry { "RECEPTION" } else { "DELIVRANCE" };
    let summary = format!(
        "file={loc} | sheet={} | spec={} | qte={} | nom={} | ordo={}",
        w.sheet,
        w.specialite,
        num_fmt(w.quantity),
        w.nom,
        w.ordo
    );
    let result = add_entry_unlogged(path, w, audit);
    match &result {
        Ok(()) => audit.log(kind, &summary),
        Err(f) if f.code == "colonnes" => audit.log(
            &format!("{kind}_COLONNES"),
            &format!("{summary} | colonnes patient à confirmer"),
        ),
        Err(f) => audit.log(
            &format!("{kind}_ECHEC"),
            &format!("{summary} | erreur={}", f.msg),
        ),
    }
    result
}

fn add_entry_unlogged(path: &Path, w: &Write, audit: &Audit) -> Result<(), Fail> {
    if java_blank(w.sheet) {
        return Err(fail("ecriture", tr("cls_err_no_sheet").to_owned()));
    }
    if java_blank(w.specialite) {
        return Err(fail("ecriture", tr("cls_err_no_spec").to_owned()));
    }
    if !w.quantity.is_finite() || w.quantity <= 0.0 {
        return Err(fail(
            "ecriture",
            trf("cls_err_quantity", java_double(w.quantity)),
        ));
    }
    let name = file_name(path);
    // One writer per file at a time, in this process.
    static LOCKS: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = LOCKS.lock().unwrap_or_else(|e| e.into_inner());
    check_lock(path)?;
    let loaded = stamp(path);
    if !path.is_file() {
        return Err(fail("ecriture", trf("cls_err_read", &name)));
    }
    let (entries, xml) = content_xml(path).map_err(|e| fail("ecriture", e))?;
    let mut dom = parse_dom(&xml).ok_or_else(|| fail("ecriture", trf("cls_err_read", &name)))?;
    let root = dom
        .iter_mut()
        .find_map(|n| match n {
            Node::El(e) => Some(e),
            _ => None,
        })
        .ok_or_else(|| fail("ecriture", trf("cls_err_read", &name)))?;
    let ns = Ns::of(root).ok_or_else(|| fail("ecriture", trf("cls_err_read", &name)))?;
    let table = find_table(root, &ns, w.sheet)
        .ok_or_else(|| fail("ecriture", trn("cls_err_no_tab", &[&w.sheet, &name])))?;
    {
        let mut sheet = Sheet::new(table, ns);
        add_to_sheet(&mut sheet, w, audit, &name)?;
    }
    let mut out = String::with_capacity(xml.len() + 4096);
    serialize(&dom, &mut out);
    let content_entry = zip_entry(&entries, "content.xml")
        .ok_or_else(|| fail("ecriture", trf("cls_err_write", &name)))?;
    let fresh = deflated(b"content.xml", content_entry, out.as_bytes());
    let parts: Vec<ZipEntry> = entries
        .into_iter()
        .map(|e| {
            if e.name == b"content.xml" {
                ZipEntry {
                    name: fresh.name.clone(),
                    flags: fresh.flags,
                    method: fresh.method,
                    time: fresh.time,
                    date: fresh.date,
                    crc: fresh.crc,
                    csize: fresh.csize,
                    usize_: fresh.usize_,
                    data: fresh.data.clone(),
                }
            } else {
                e
            }
        })
        .collect();
    let bytes = zip_write(&parts);
    let dir = path.parent().unwrap_or(Path::new("."));
    let tmp = temp_in(dir).ok_or_else(|| fail("ecriture", trf("cls_err_write", &name)))?;
    if std::fs::write(&tmp, &bytes).is_err() {
        let _ = std::fs::remove_file(&tmp);
        return Err(fail("ecriture", trf("cls_err_write", &name)));
    }
    // Never a corrupt save over a good register.
    if let Err(e) = validate(&tmp) {
        let _ = std::fs::remove_file(&tmp);
        return Err(fail("ecriture", e));
    }
    if let Err(e) = check_lock(path) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    if stamp(path) != loaded {
        let _ = std::fs::remove_file(&tmp);
        return Err(fail("concurrent", trf("cls_err_changed", &name)));
    }
    if !backup(path) {
        audit.log(
            "SAUVEGARDE_ECHEC",
            &format!("file={name} | copie de sécurité impossible avant écriture"),
        );
    }
    if std::fs::rename(&tmp, path).is_err() && std::fs::copy(&tmp, path).is_err() {
        let _ = std::fs::remove_file(&tmp);
        return Err(fail("ecriture", trf("cls_err_save", &name)));
    }
    let _ = std::fs::remove_file(&tmp);
    Ok(())
}

fn find_table<'a>(el: &'a mut El, ns: &Ns, name: &str) -> Option<&'a mut El> {
    let table = ns.t("table");
    let attr = ns.t("name");
    if el.name == table && el.get(&attr) == Some(name) {
        return Some(el);
    }
    for k in &mut el.kids {
        if let Node::El(e) = k {
            if let Some(t) = find_table(e, ns, name) {
                return Some(t);
            }
        }
    }
    None
}

fn temp_in(dir: &Path) -> Option<PathBuf> {
    let base = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    for k in 0..100u128 {
        let p = dir.join(format!(
            "stupctl-{}{}.ods",
            std::process::id(),
            base.wrapping_add(k)
        ));
        if std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&p)
            .is_ok()
        {
            return Some(p);
        }
    }
    None
}

/// `validateSavedOds`: a content.xml, and one the reader reads.
fn validate(tmp: &Path) -> Result<(), String> {
    let bytes = std::fs::read(tmp).map_err(|_| tr("cls_err_saved_unreadable").to_owned())?;
    let entries = zip_read(&bytes).ok_or_else(|| tr("cls_err_saved_unreadable").to_owned())?;
    match zip_entry(&entries, "content.xml") {
        Some(e) if e.usize_ > 0 => {}
        _ => return Err(tr("cls_err_saved_invalid").to_owned()),
    }
    read_file(tmp)
        .map(|_| ())
        .map_err(|e| trf("cls_err_saved_bad", e))
}

// ------------------------------------------------------------------
// The bridge, without the program
// ------------------------------------------------------------------

/// **StupControl's registers, read and written from here.** `config` is
/// StupControl's settings file, empty for its own.
pub struct Native {
    pub config: String,
    /// Where the audit lines go; none for StupControl's own `audit.log`.
    pub audit: Option<PathBuf>,
}

struct Ready {
    settings: Settings,
}

impl Native {
    fn config_path(&self) -> PathBuf {
        let c = self.config.trim();
        if c.is_empty() {
            default_config()
        } else {
            PathBuf::from(c)
        }
    }

    fn ready(&self) -> Result<Ready, String> {
        let path = self.config_path();
        if self.config.trim().is_empty() && !path.exists() {
            return Err(trf("cls_err_unconfigured", path.display()));
        }
        let settings = load_settings(&path);
        if java_blank(&settings.data_dir) || !Path::new(&settings.data_dir).is_dir() {
            return Err(trf("cls_err_data_dir", &settings.data_dir));
        }
        Ok(Ready { settings })
    }

    fn audit(&self) -> Audit {
        Audit {
            file: self
                .audit
                .clone()
                .unwrap_or_else(|| config_dir().join("audit.log")),
        }
    }

    /// The registers are there: where.
    pub fn ping(&self) -> Result<Answer, String> {
        let ready = self.ready()?;
        Ok(Answer {
            version: String::new(),
            data_dir: ready.settings.data_dir,
            products: Vec::new(),
            unreadable: Vec::new(),
        })
    }
}

impl Bridge for Native {
    fn read(&self, since: &str) -> Result<Answer, String> {
        self.gather(since, false)
    }

    fn survey(&self, since: &str) -> Result<Answer, String> {
        self.gather(since, true)
    }

    fn add(&self, entries: &[Entry]) -> Result<Vec<Outcome>, String> {
        self.add_entries(entries)
    }
}

impl Native {
    /// `Bridge.read`, rule for rule; with `survey`, also what each year
    /// says ([`crate::stupcontrol::YearRead`]) and a misspelt header read
    /// under the name it designates ([`closest_header`]) — what a
    /// takeover needs and StupControl never did.
    fn gather(&self, since: &str, survey: bool) -> Result<Answer, String> {
        let ready = self.ready()?;
        let (sy, sm, sd) = iso_date(since).ok_or_else(|| trf("cls_err_since", since))?;
        let dir = &ready.settings.data_dir;
        let mut session = Session::default();
        let mut seen = HashSet::new();
        let mut unreadable = Vec::new();
        let mut products = Vec::new();
        let years = years(dir);
        for d in active_drugs(&ready.settings) {
            if java_blank(&d.file_name) || !seen.insert(d.alias_key()) {
                continue;
            }
            let mut lines = Vec::new();
            let mut surveyed = Vec::new();
            // From this product's first workbook to its last: a year
            // without one in between is a hole, one outside is not.
            let held: Vec<i64> = years
                .iter()
                .copied()
                .filter(|&y| y >= sy && resolve_file(dir, &d, y).exists())
                .collect();
            let span = held.first().copied().unwrap_or(1)..=held.last().copied().unwrap_or(0);
            for year in span {
                let file = resolve_file(dir, &d, year);
                let shown = format!("{year}/{}", d.file_name);
                let mut read = crate::stupcontrol::YearRead {
                    year,
                    file: shown.clone(),
                    ..Default::default()
                };
                if !years.contains(&year) || !file.exists() {
                    read.file.clear();
                    if survey {
                        surveyed.push(read);
                    }
                    continue;
                }
                if session.read(&file).is_none() {
                    if session.unreadable(&file) {
                        unreadable.push(shown);
                    }
                    continue;
                }
                let Some(fd) = session.read(&file) else {
                    continue;
                };
                let header = match fd.movements(&d.specialite) {
                    Some(_) => Some(d.specialite.clone()),
                    None if survey => closest_header(&d.specialite, fd.products()).map(|i| {
                        read.lenient = true;
                        fd.products()[i].clone()
                    }),
                    None => None,
                };
                let Some(header) = header else {
                    if survey {
                        read.headers = fd.products().to_vec();
                        surveyed.push(read);
                    }
                    continue;
                };
                let Some(moves) = fd.movements(&header) else {
                    continue;
                };
                if survey {
                    read.header = if read.lenient {
                        header.clone()
                    } else {
                        d.specialite.clone()
                    };
                    read.opening = fd.opening(&header);
                    read.closing = fd.current_balance(&header);
                    read.net = fd.monthly(&header).map(|m| sum_net(&m)).unwrap_or(0.0);
                    read.fractions = moves.iter().filter(|m| m.units != m.units.trunc()).count();
                    surveyed.push(read);
                }
                for m in moves {
                    let day = register_date(&m.date);
                    let in_window = match day {
                        Some(day) => day >= (sy, sm, sd),
                        None => year > sy || m.month as i64 >= sm,
                    };
                    if !in_window {
                        continue;
                    }
                    lines.push(Line {
                        year,
                        month: m.month as i64,
                        date: day.map(|(y, mo, da)| format!("{y:04}-{mo:02}-{da:02}")),
                        date_text: m.date.clone(),
                        kind: if m.entry { "E" } else { "S" }.to_owned(),
                        units: m.units,
                        nom: java_trim(&m.who).to_owned(),
                        prenom: java_trim(&m.prenom).to_owned(),
                        ordo: java_trim(&m.ordo).to_owned(),
                    });
                }
            }
            let stock = current_stock(dir, &d, &mut session);
            products.push(Product {
                key: d.alias_key(),
                label: d.commercial_name.clone(),
                file: d.file_name.clone(),
                specialite: d.specialite.clone(),
                units_per_box: d.pills_per_box,
                min_stock: d.min_stock,
                stock: Some(stock),
                movements: lines,
                years: surveyed,
            });
        }
        Ok(Answer {
            version: String::new(),
            data_dir: dir.clone(),
            products,
            unreadable,
        })
    }

    fn add_entries(&self, entries: &[Entry]) -> Result<Vec<Outcome>, String> {
        let ready = self.ready()?;
        let audit = self.audit();
        audit.log("PASSERELLE_BPM", &format!("ajouts={}", entries.len()));
        let mut by_key: HashMap<String, Drug> = HashMap::new();
        for d in active_drugs(&ready.settings) {
            by_key.entry(d.alias_key()).or_insert(d);
        }
        let dir = &ready.settings.data_dir;
        Ok(entries
            .iter()
            .map(
                |e| match add_one(dir, &ready.settings, &by_key, e, &audit) {
                    Ok(()) => Outcome {
                        ok: true,
                        code: None,
                        error: None,
                    },
                    Err(f) => Outcome {
                        ok: false,
                        code: Some(f.code.to_owned()),
                        error: Some(f.msg),
                    },
                },
            )
            .collect())
    }
}

fn add_one(
    dir: &str,
    settings: &Settings,
    by_key: &HashMap<String, Drug>,
    e: &Entry,
    audit: &Audit,
) -> Result<(), Fail> {
    let d = by_key
        .get(&e.key)
        .ok_or_else(|| fail("produit", trf("cls_err_product", &e.key)))?;
    let (y, m, day) =
        iso_date(&e.date).ok_or_else(|| fail("date", trf("cls_err_date", &e.date)))?;
    let entry = match e.kind.as_str() {
        "E" => true,
        "S" => false,
        other => return Err(fail("nature", trf("cls_err_kind", other))),
    };
    let file = resolve_file(dir, d, y);
    if !file.exists() {
        return Err(fail(
            "fichier",
            trf("cls_err_missing", format!("{y}/{}", d.file_name)),
        ));
    }
    let date_text = format!("{day:02}/{m:02}/{:02}", y.rem_euclid(100));
    let columns = settings.patient_columns.get(&d.file_name);
    add_entry(
        &file,
        &Write {
            sheet: sheet_name(m as u32),
            specialite: &d.specialite,
            entry,
            nom: java_trim(&e.nom),
            prenom: java_trim(&e.prenom),
            date_text: &date_text,
            ordo: java_trim(&e.ordo),
            quantity: e.units,
            columns: columns.map(|c| c.as_slice()),
        },
        audit,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Un en-tête mal recopié** se lit sous le nom qu'il désigne : même
    /// dosage, une faute de frappe ; jamais un autre dosage, jamais deux.
    #[test]
    fn a_misspelt_header_is_read_only_when_one_header_is_that_close() {
        let h = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(
            closest_header("Oxycodone 10", &h(&["Oxycodonne 10", "Skenan 10"])),
            Some(0)
        );
        assert_eq!(closest_header("Oxycodone 10", &h(&["Oxycodone 20"])), None);
        assert_eq!(
            closest_header("Méthadone 40 mg", &h(&["Metadone 40 mg"])),
            Some(0)
        );
        assert_eq!(
            closest_header("Oxycodone 10", &h(&["Oxycodonne 10", "Oxycodone 10."])),
            None
        );
        assert_eq!(closest_header("Oxycodone 10", &h(&["Fentanyl 10"])), None);
    }

    #[test]
    fn a_specialite_finds_its_one_column() {
        let heads: Vec<String> = [
            "Actiskénan CINQ ORO",
            "Actiskénan LP CINQ 10",
            "ACTISKÉNAN 20 mg",
            "Actiskénan 5 mg ORO",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        assert_eq!(match_specialite("actiskenan cinq oro", &heads), Some(0));
        assert_eq!(match_specialite("ACTISKENAN 20MG", &heads), Some(2));
        // Two headers fit as well: no column.
        assert_eq!(match_specialite("actiskenan", &heads), None);
        // A dose is a number: « 10 » never reaches « 100 ».
        let skenan = vec!["Skenan LP 100 mg".to_string()];
        assert_eq!(match_specialite("skenan 10", &skenan), None);
        assert_eq!(norm_specialite("SKENAN L.P. 10MG"), "skenan lp 10 mg");
        assert_eq!(norm_specialite("Skenan-LP 10 mg"), "skenan lp 10 mg");
        assert_eq!(norm_specialite("0.5 µg"), "0 5 ug");
    }

    #[test]
    fn numbers_print_as_java_prints_them() {
        assert_eq!(java_double(28.0), "28.0");
        assert_eq!(java_double(0.5), "0.5");
        assert_eq!(java_double(1e7), "1.0E7");
        assert_eq!(java_double(-3.0), "-3.0");
        assert_eq!(num_fmt(28.0), "28");
        assert_eq!(num_fmt(28.299999999999997), "28.3");
        assert_eq!(num_fmt(0.125), "0.13");
        assert_eq!(num_fmt(2.995), "3");
        assert_eq!(num_parse("1 000,5"), Some(1000.5));
        assert_eq!(num_parse("abc"), None);
    }

    #[test]
    fn a_register_date_is_strict() {
        assert_eq!(register_date("5/1/25"), Some((2025, 1, 5)));
        assert_eq!(register_date("15/01/2025"), Some((2025, 1, 15)));
        assert_eq!(register_date("31/02/25"), None);
        assert_eq!(register_date("15-01-25"), None);
        assert!(is_totals_label("Totaux mensuels"));
        assert!(!is_totals_label("Totaro"));
    }

    /// A small workbook: names row, E/S/B row, an opening row, and the
    /// rows given, then TOTAUX when asked.
    fn workbook(dir: &Path, name: &str, rows: &[&str], totals: bool, blanks: usize) -> PathBuf {
        let cell_s = |t: &str| {
            format!(
                "<table:table-cell office:value-type=\"string\" calcext:value-type=\"string\"><text:p>{t}</text:p></table:table-cell>"
            )
        };
        let mut sheet = String::from(
            "<table:table table:name=\"Mars\"><table:table-column table:number-columns-repeated=\"10\"/>",
        );
        sheet.push_str("<table:table-row><table:table-cell table:number-columns-repeated=\"4\"/>");
        sheet.push_str(&cell_s("Méthadone 10 mg"));
        sheet.push_str("<table:table-cell table:number-columns-repeated=\"2\"/>");
        sheet.push_str(&cell_s("Méthadone 40 mg"));
        sheet.push_str("</table:table-row><table:table-row>");
        for h in [
            "Nom",
            "Prénom",
            "Date",
            "Ordonnancier",
            "E",
            "S",
            "B",
            "E",
            "S",
            "B",
        ] {
            sheet.push_str(&cell_s(h));
        }
        sheet.push_str("</table:table-row>");
        sheet.push_str("<table:table-row><table:table-cell table:number-columns-repeated=\"6\"/><table:table-cell office:value-type=\"float\" office:value=\"40\" calcext:value-type=\"float\"><text:p>40</text:p></table:table-cell></table:table-row>");
        for r in rows {
            sheet.push_str(r);
        }
        if blanks > 0 {
            sheet.push_str(&format!(
                "<table:table-row table:number-rows-repeated=\"{blanks}\"><table:table-cell table:number-columns-repeated=\"10\"/></table:table-row>"
            ));
        }
        if totals {
            sheet.push_str("<table:table-row>");
            sheet.push_str(&cell_s("TOTAUX"));
            sheet.push_str("</table:table-row>");
        }
        sheet.push_str("</table:table>");
        let content = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?><office:document-content xmlns:office=\"urn:oasis:names:tc:opendocument:xmlns:office:1.0\" xmlns:table=\"urn:oasis:names:tc:opendocument:xmlns:table:1.0\" xmlns:text=\"urn:oasis:names:tc:opendocument:xmlns:text:1.0\" xmlns:calcext=\"urn:org:documentfoundation:names:experimental:calc:xmlns:calcext:1.0\"><office:body><office:spreadsheet>{sheet}</office:spreadsheet></office:body></office:document-content>"
        );
        let mime = b"application/vnd.oasis.opendocument.spreadsheet";
        let mut crc = crc32fast::Hasher::new();
        crc.update(mime);
        let stored = ZipEntry {
            name: b"mimetype".to_vec(),
            flags: 0,
            method: 0,
            time: 0,
            date: 0x21,
            crc: crc.finalize(),
            csize: mime.len() as u32,
            usize_: mime.len() as u32,
            data: mime.to_vec(),
        };
        let content = deflated(b"content.xml", &stored, content.as_bytes());
        let path = dir.join(name);
        std::fs::write(&path, zip_write(&[stored, content])).unwrap();
        path
    }

    fn temp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "bpm-classeur-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn write<'a>(entry: bool, q: f64, nom: &'a str, ordo: &'a str) -> Write<'a> {
        Write {
            sheet: "Mars",
            specialite: "Méthadone 10 mg",
            entry,
            nom,
            prenom: "",
            date_text: "03/03/25",
            ordo,
            quantity: q,
            columns: None,
        }
    }

    #[test]
    fn a_line_lands_under_the_last_one_and_the_bilan_follows() {
        let dir = temp("add");
        let path = workbook(&dir, "methadone.ods", &[], true, 5);
        let audit = Audit {
            file: dir.join("audit.log"),
        };
        add_entry(&path, &write(true, 28.0, "CERP", ""), &audit).unwrap();
        add_entry(&path, &write(false, 7.0, "Dupont", "18412"), &audit).unwrap();
        let fd = read_file(&path).unwrap();
        let moves = fd.movements("Méthadone 10 mg").unwrap();
        assert_eq!(moves.len(), 2);
        assert_eq!(
            (moves[1].who.as_str(), moves[1].ordo.as_str()),
            ("Dupont", "18412")
        );
        // 40 carried in, +28, -7.
        assert_eq!(fd.current_balance("Méthadone 10 mg"), Some(61.0));
        // The formula starts from the bilan above it.
        let (_, xml) = content_xml(&path).unwrap();
        let xml = String::from_utf8(xml).unwrap();
        assert!(
            xml.contains("of:=IFERROR(INT([.G3]); 0)-[.F4]+[.E4]"),
            "{xml}"
        );
        assert!(
            xml.contains("of:=IFERROR(INT([.G4]); 0)-[.F5]+[.E5]"),
            "{xml}"
        );
        // The other spécialité is untouched; TOTAUX is still there, below.
        assert!(fd.movements("Méthadone 40 mg").unwrap().is_empty());
        assert!(xml.find("TOTAUX").unwrap() > xml.find("Dupont").unwrap());
        // A backup per write, and the audit lines.
        assert_eq!(std::fs::read_dir(dir.join(".backups")).unwrap().count(), 2);
        let log = std::fs::read_to_string(dir.join("audit.log")).unwrap();
        assert!(log.contains("| RECEPTION | file="), "{log}");
        assert!(
            log.contains("| DELIVRANCE | file=") && log.contains("ordo=18412"),
            "{log}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_file_open_in_libreoffice_or_a_full_sheet_is_refused() {
        let dir = temp("refuse");
        let audit = Audit {
            file: dir.join("audit.log"),
        };
        let path = workbook(&dir, "methadone.ods", &[], true, 0);
        let full = add_entry(&path, &write(true, 1.0, "CERP", ""), &audit).unwrap_err();
        assert_eq!(full.code, "plein");
        std::fs::write(dir.join(".~lock.methadone.ods#"), "x").unwrap();
        let locked = add_entry(&path, &write(true, 1.0, "CERP", ""), &audit).unwrap_err();
        assert_eq!(locked.code, "verrou");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_full_sheet_with_empty_rows_among_its_lines_is_packed_first() {
        let dir = temp("pack");
        let audit = Audit {
            file: dir.join("audit.log"),
        };
        let line = "<table:table-row><table:table-cell office:value-type=\"string\"><text:p>Martin</text:p></table:table-cell><table:table-cell table:number-columns-repeated=\"4\"/><table:table-cell office:value-type=\"float\" office:value=\"5\"><text:p>5</text:p></table:table-cell></table:table-row>";
        let blank = "<table:table-row><table:table-cell table:number-columns-repeated=\"10\"/></table:table-row>";
        let path = workbook(&dir, "methadone.ods", &[blank, line], true, 0);
        add_entry(&path, &write(false, 2.0, "Durand", "7"), &audit).unwrap();
        let fd = read_file(&path).unwrap();
        let moves = fd.movements("Méthadone 10 mg").unwrap();
        let who: Vec<&str> = moves.iter().map(|m| m.who.as_str()).collect();
        assert_eq!(who, ["Martin", "Durand"]);
        assert_eq!(fd.current_balance("Méthadone 10 mg"), Some(33.0));
        assert!(std::fs::read_to_string(dir.join("audit.log"))
            .unwrap()
            .contains("NETTOYAGE_AUTO"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn without_totaux_a_line_is_appended() {
        let dir = temp("append");
        let audit = Audit {
            file: dir.join("audit.log"),
        };
        let path = workbook(&dir, "methadone.ods", &[], false, 0);
        add_entry(&path, &write(true, 10.0, "OCP", ""), &audit).unwrap();
        add_entry(&path, &write(false, 3.0, "Petit", "ORD-42"), &audit).unwrap();
        let fd = read_file(&path).unwrap();
        let moves = fd.movements("Méthadone 10 mg").unwrap();
        assert_eq!(moves.len(), 2);
        assert_eq!(moves[1].ordo, "ORD-42");
        assert_eq!(fd.current_balance("Méthadone 10 mg"), Some(47.0));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_settings_and_the_catalogue_name_the_products() {
        let dir = temp("settings");
        let data = dir.join("registres");
        std::fs::create_dir_all(data.join("2025")).unwrap();
        let config = dir.join("config.json");
        std::fs::write(
            &config,
            serde_json::json!({
                "dataDir": data, "language": "fr",
                "drugs": [{"commercialName": " Méthadone 10 mg ", "fileName": "methadone.ods",
                           "specialite": "Méthadone 10 mg", "minStock": 5}],
                "patientColumns": {"methadone.ods": [0, 1, 2, 3]}
            })
            .to_string(),
        )
        .unwrap();
        std::fs::write(
            dir.join("unites-par-boite.json"),
            r#"{"methadone.ods|méthadone 10 mg": 7}"#,
        )
        .unwrap();
        let s = load_settings(&config);
        assert_eq!(s.drugs[0].commercial_name, "Méthadone 10 mg");
        assert_eq!(s.drugs[0].pills_per_box, 7);
        assert_eq!(active_drugs(&s), s.drugs);
        std::fs::write(
            data.join("products.toml"),
            "[[produit]]\nnom = \"Méthadone 10 mg\"\nnom_commercial = \"\"\nfichier = \"a.ods\"\nunites_par_boite = 0\nseuil = 0\n\n[[produit]]\nnom = \"Méthadone 10 mg\"\nfichier = \"b.ods\"\n",
        )
        .unwrap();
        let drugs = active_drugs(&s);
        assert_eq!(drugs.len(), 2);
        assert_eq!(drugs[0].commercial_name, "Méthadone 10 mg (a)");
        assert_eq!(drugs[0].pills_per_box, 1);
        assert_eq!(years(&data.to_string_lossy()), vec![2025]);
        let _ = std::fs::remove_dir_all(&dir);
    }
    /// A cell's text as StupControl reads it: spaces, tabs and line
    /// breaks spelled out, one line per paragraph.
    #[test]
    fn a_cell_reads_its_spaces_tabs_and_paragraphs() {
        let xml = br#"<table:table-cell xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"><text:p>Du<text:s text:c="2"/>pont<text:tab/>A<text:line-break/>B</text:p><text:p>C<text:s/><text:span>D</text:span></text:p></table:table-cell>"#;
        let nodes = parse_dom(xml).unwrap();
        let Some(Node::El(cell)) = nodes.iter().find(|n| matches!(n, Node::El(_))) else {
            panic!("no element");
        };
        let ns = Ns::of(cell).unwrap();
        assert_eq!(display_text(cell, &ns), "Du  pont\tA\nB\nC D");
    }

    /// The bridge itself, from StupControl's settings file to the
    /// answer: lines added and refused, the stock carried over a year
    /// with no workbook, a broken workbook named, a missing column and
    /// a misspelt header said year by year.
    #[test]
    fn the_bridge_reads_and_writes_a_tree_of_registers() {
        use crate::stupcontrol::{Bridge, Entry};
        let dir = temp("bridge");
        let data = dir.join("registres");
        for y in ["2023", "2024", "2025"] {
            std::fs::create_dir_all(data.join(y)).unwrap();
        }
        workbook(&data.join("2023"), "methadone.ods", &[], true, 5);
        workbook(&data.join("2025"), "methadone.ods", &[], true, 5);
        std::fs::write(data.join("2025").join("broken.ods"), b"not a zip").unwrap();
        let config = dir.join("config.json");
        let drug = |spec: &str, file: &str| serde_json::json!({"commercialName": spec, "fileName": file, "specialite": spec});
        std::fs::write(
            &config,
            serde_json::json!({
                "dataDir": data,
                "drugs": [drug("Méthadone 10 mg", "methadone.ods"),
                          drug("Méthadone 5 mg", "methadone.ods"),
                          drug("Méthdone 40 mg", "methadone.ods"),
                          drug("Cassé 1 mg", "broken.ods")],
            })
            .to_string(),
        )
        .unwrap();
        let native = Native {
            config: config.to_string_lossy().into_owned(),
            audit: Some(dir.join("audit.log")),
        };
        assert_eq!(native.ping().unwrap().data_dir, data.to_string_lossy());
        assert!(native.read("hier").is_err());
        let nowhere = Native {
            config: dir.join("absent.json").to_string_lossy().into_owned(),
            audit: None,
        };
        assert!(nowhere.read("2025-01-01").is_err());

        let key = "methadone.ods|méthadone 10 mg".to_owned();
        let entry = |key: &str, kind: &str, date: &str, units: f64, ordo: &str| Entry {
            key: key.to_owned(),
            kind: kind.to_owned(),
            date: date.to_owned(),
            units,
            nom: "Fournisseur".to_owned(),
            prenom: String::new(),
            ordo: ordo.to_owned(),
        };
        let out = native
            .add(&[
                entry(&key, "E", "2025-03-02", 10.0, ""),
                entry(&key, "S", "2025-03-03", 3.0, "18001"),
                entry("inconnu|x", "E", "2025-03-02", 1.0, ""),
                entry(&key, "E", "2025-13-40", 1.0, ""),
                entry(&key, "X", "2025-03-02", 1.0, ""),
                entry(&key, "E", "2024-03-02", 1.0, ""),
            ])
            .unwrap();
        let codes: Vec<Option<&str>> = out.iter().map(|o| o.code.as_deref()).collect();
        assert_eq!(
            codes,
            vec![
                None,
                None,
                Some("produit"),
                Some("date"),
                Some("nature"),
                Some("fichier")
            ]
        );

        let read = native.read("2025-01-01").unwrap();
        assert_eq!(read.unreadable, vec!["2025/broken.ods".to_owned()]);
        let p = read.products.iter().find(|p| p.key == key).unwrap();
        let moved: Vec<(&str, f64, &str)> = p
            .movements
            .iter()
            .map(|l| (l.kind.as_str(), l.units, l.ordo.as_str()))
            .collect();
        assert_eq!(moved, vec![("E", 10.0, ""), ("S", 3.0, "18001")]);
        assert_eq!(p.movements[0].date.as_deref(), Some("2025-03-02"));
        assert_eq!(p.stock, Some(47.0));
        assert!(p.years.is_empty(), "a plain read surveys nothing");

        let survey = native.survey("2023-01-01").unwrap();
        let p = survey.products.iter().find(|p| p.key == key).unwrap();
        let years: Vec<(i64, bool)> = p
            .years
            .iter()
            .map(|y| (y.year, y.file.is_empty()))
            .collect();
        assert_eq!(years, vec![(2023, false), (2024, true), (2025, false)]);
        assert_eq!(p.years[2].closing, Some(47.0));
        assert_eq!(p.years[2].net, 7.0);
        let five = survey
            .products
            .iter()
            .find(|p| p.specialite == "Méthadone 5 mg")
            .unwrap();
        assert!(five.movements.is_empty());
        assert!(five.years[0]
            .headers
            .contains(&"Méthadone 10 mg".to_owned()));
        let typo = survey
            .products
            .iter()
            .find(|p| p.specialite == "Méthdone 40 mg")
            .unwrap();
        assert!(
            typo.years
                .iter()
                .filter(|y| !y.file.is_empty())
                .all(|y| y.lenient && y.header == "Méthadone 40 mg"),
            "{:?}",
            typo.years
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// **The same registers, the same answers as StupControl itself.**
    /// Run when `BPM_CADDY_STUPCONTROL` names the built program
    /// (`./gradlew installDist`, with `JAVA_HOME` set) and
    /// `BPM_CADDY_STUPCONTROL_SAMPLE` a tree of registers: the tree is
    /// copied twice, the same reads and the same lines go to the program
    /// on one copy and to this module on the other, and the answers, the
    /// cells of every sheet and the audit actions must agree.
    #[test]
    fn the_program_and_this_module_do_the_same_thing() {
        let (Ok(exe), Ok(sample)) = (
            std::env::var("BPM_CADDY_STUPCONTROL"),
            std::env::var("BPM_CADDY_STUPCONTROL_SAMPLE"),
        ) else {
            return;
        };
        let root = temp("parity");
        let drugs = serde_json::json!([
            {"commercialName": "Méthadone 10", "fileName": "methadone.ods", "specialite": "Méthadone 10 mg"},
            {"commercialName": "Méthadone 60", "fileName": "methadone.ods", "specialite": "METHADONE 60MG"},
            {"commercialName": "Actiskénan oro", "fileName": "actiskenan.ods", "specialite": "actiskenan cinq oro"},
            {"commercialName": "Actiskénan 5", "fileName": "actiskenan.ods", "specialite": "Actiskénan 5 mg ORO"},
            {"commercialName": "Actiskénan", "fileName": "actiskenan.ods", "specialite": "actiskenan"},
            {"commercialName": "Medikinet 10", "fileName": "medikinet.ods", "specialite": "Medikinet 10 mg"},
            {"commercialName": "Oxycontin", "fileName": "oxy.ods", "specialite": "OXYCONTIN dix LP"},
            {"commercialName": "Gabarit B", "fileName": "tpl.ods", "specialite": "Spécialité B"},
            {"commercialName": "Gabarit A", "fileName": "tpl.ods", "specialite": "Spécialité A"},
            {"commercialName": "Absent", "fileName": "absent.ods", "specialite": "X"},
            {"commercialName": "Skenan 10", "fileName": "skenan.ods", "specialite": "Skenan LP 10 mg"},
            {"commercialName": "Skenan 30", "fileName": "Skenan.ODS", "specialite": "skenan lp 30 mg"}
        ]);
        let side = |tag: &str| {
            let base = root.join(tag);
            let data = base.join("registres");
            copy_tree(Path::new(&sample), &data);
            let config = base.join("config.json");
            std::fs::write(
                &config,
                serde_json::json!({"dataDir": data, "drugs": drugs, "patientColumns": {}})
                    .to_string(),
            )
            .unwrap();
            (base, data, config)
        };
        let (java_base, java_data, java_config) = side("java");
        let (rust_base, rust_data, rust_config) = side("rust");
        let call = |request: serde_json::Value| -> serde_json::Value {
            let req = java_base.join("requete.json");
            let resp = java_base.join("reponse.json");
            std::fs::write(&req, request.to_string()).unwrap();
            let status = std::process::Command::new(&exe)
                .arg("--bridge")
                .arg(&req)
                .arg(&resp)
                .env("APPDATA", &java_base)
                .status()
                .unwrap();
            assert!(status.success());
            serde_json::from_str(&std::fs::read_to_string(&resp).unwrap()).unwrap()
        };
        let native = Native {
            config: rust_config.to_string_lossy().into_owned(),
            audit: Some(rust_base.join("StupControl").join("audit.log")),
        };
        let java_read = |since: &str| -> Answer {
            let v = call(serde_json::json!({"op": "read", "config": java_config, "since": since}));
            assert_eq!(v["ok"], true, "{v}");
            serde_json::from_value(v).unwrap()
        };
        let same = |since: &str| {
            let mut j = java_read(since);
            let mut r = native.read(since).unwrap();
            j.version.clear();
            j.data_dir.clear();
            r.data_dir.clear();
            j.unreadable.sort();
            r.unreadable.sort();
            assert_eq!(j.products.len(), r.products.len());
            for (a, b) in j.products.iter().zip(&r.products) {
                assert_eq!(a, b, "{}", a.key);
            }
            assert_eq!(j, r);
        };
        same("2024-01-01");
        same("2025-03-01");
        same("2026-02-15");

        let entry = |key: &str, kind: &str, date: &str, units: f64, nom: &str, ordo: &str| Entry {
            key: key.into(),
            kind: kind.into(),
            date: date.into(),
            units,
            nom: nom.into(),
            prenom: if nom.is_empty() { "" } else { "Albert" }.into(),
            ordo: ordo.into(),
        };
        let m10 = "methadone.ods|méthadone 10 mg";
        let entries = vec![
            entry(m10, "E", "2025-03-02", 28.0, "CERP", ""),
            entry(m10, "S", "2025-03-03", 7.0, "Dupont", "18412"),
            entry(
                "methadone.ods|methadone 60mg",
                "S",
                "2025-03-05",
                3.0,
                "Martin",
                "007",
            ),
            entry(
                "actiskenan.ods|actiskenan cinq oro",
                "S",
                "2026-02-10",
                2.0,
                "Durand",
                "ORD-2025/42",
            ),
            entry(
                "actiskenan.ods|actiskenan",
                "S",
                "2026-02-11",
                2.0,
                "Durand",
                "1",
            ),
            entry(
                "oxy.ods|oxycontin dix lp",
                "E",
                "2025-03-04",
                10.0,
                "OCP",
                "",
            ),
            entry(
                "oxy.ods|oxycontin dix lp",
                "S",
                "2025-03-06",
                4.0,
                "Petit",
                "99",
            ),
            entry(
                "tpl.ods|spécialité b",
                "S",
                "2026-02-20",
                5.0,
                "Leroy",
                "18420",
            ),
            entry("tpl.ods|spécialité a", "E", "2026-01-15", 3.0, "CERP", ""),
            entry("nope|x", "E", "2025-03-02", 1.0, "", ""),
            entry(m10, "E", "2025-13-01", 1.0, "", ""),
            entry(m10, "X", "2025-03-02", 1.0, "", ""),
            entry(m10, "E", "2030-03-02", 1.0, "", ""),
            entry(m10, "S", "2025-03-03", 0.0, "Dupont", "18413"),
            entry(
                "medikinet.ods|medikinet 10 mg",
                "S",
                "2024-05-05",
                1.5,
                "Roux",
                "5",
            ),
            entry("absent.ods|x", "S", "2025-05-05", 1.0, "Roux", "6"),
            entry(
                "skenan.ods|skenan lp 10 mg",
                "S",
                "2025-02-10",
                1.0,
                "Blanc",
                "18002",
            ),
            entry(
                "skenan.ods|skenan lp 30 mg",
                "E",
                "2025-02-11",
                2.0,
                "CERP",
                "",
            ),
            entry(
                "skenan.ods|skenan lp 10 mg",
                "S",
                "2025-03-10",
                1.0,
                "Noir",
                "18102",
            ),
            entry(
                "skenan.ods|skenan lp 10 mg",
                "S",
                "2025-03-11",
                1.0,
                "Vert",
                "18103",
            ),
            entry(
                "skenan.ods|skenan lp 10 mg",
                "S",
                "2025-04-02",
                1.0,
                "Gris",
                "18104",
            ),
        ];
        let v = call(serde_json::json!({"op": "add", "config": java_config, "entries": entries}));
        assert_eq!(v["ok"], true, "{v}");
        let java_out: Vec<Outcome> = serde_json::from_value(v["results"].clone()).unwrap();
        let rust_out = native.add(&entries).unwrap();
        for (i, (a, b)) in java_out.iter().zip(&rust_out).enumerate() {
            assert_eq!((a.ok, &a.code), (b.ok, &b.code), "line {i}: {a:?} / {b:?}");
            assert_eq!(a.error, b.error, "line {i}");
        }
        same("2024-01-01");

        // Cell by cell, every sheet of every workbook.
        for year in ["2024", "2025", "2026"] {
            for e in std::fs::read_dir(java_data.join(year)).unwrap().flatten() {
                let name = e.file_name();
                if !name.to_string_lossy().ends_with(".ods") {
                    continue;
                }
                let a = cells(&e.path());
                let b = cells(&rust_data.join(year).join(&name));
                assert_eq!(a.len(), b.len(), "{year}/{name:?}");
                for (x, y) in a.iter().zip(&b) {
                    assert_eq!(x, y, "{year}/{name:?}");
                }
            }
        }
        // The same audit actions, in the same order.
        let actions = |p: PathBuf| -> Vec<String> {
            std::fs::read_to_string(p)
                .unwrap_or_default()
                .lines()
                .map(|l| l.splitn(3, " | ").nth(2).unwrap_or("").to_owned())
                .collect()
        };
        assert_eq!(
            actions(java_base.join("StupControl").join("audit.log")),
            actions(rust_base.join("StupControl").join("audit.log"))
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Every non-empty cell of every sheet: sheet, row, column, type,
    /// value, text and formula.
    type Cell = (String, usize, usize, String, Option<f64>, String, String);

    fn cells(path: &Path) -> Vec<Cell> {
        let (_, xml) = content_xml(path).unwrap();
        let mut dom = parse_dom(&xml).unwrap();
        let root = dom
            .iter_mut()
            .find_map(|n| match n {
                Node::El(e) => Some(e),
                _ => None,
            })
            .unwrap();
        let ns = Ns::of(root).unwrap();
        let mut names = Vec::new();
        fn tables(el: &El, ns: &Ns, out: &mut Vec<String>) {
            if el.name == ns.t("table") {
                out.push(el.get(&ns.t("name")).unwrap_or("").to_owned());
            }
            for k in &el.kids {
                if let Node::El(e) = k {
                    tables(e, ns, out);
                }
            }
        }
        tables(root, &ns, &mut names);
        let mut out = Vec::new();
        for name in names {
            let t = find_table(root, &ns, &name).unwrap();
            let sheet = Sheet::new(t, ns.clone());
            let rows = sheet.row_count().min(400);
            for r in 0..rows {
                for c in 0..40 {
                    let Some(e) = sheet.cell(c, r) else {
                        continue;
                    };
                    let kind = e.get(&ns.o("value-type")).unwrap_or("").to_owned();
                    let value = e.get(&ns.o("value")).and_then(java_parse_double);
                    let text = display_text(e, &ns);
                    let formula = e.get(&ns.t("formula")).unwrap_or("").to_owned();
                    if kind.is_empty() && text.is_empty() && formula.is_empty() {
                        continue;
                    }
                    out.push((name.clone(), r, c, kind, value, text, formula));
                }
            }
        }
        out
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
}
