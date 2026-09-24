//! In-process PDF generation with the embedded Typst engine (spec 3.2):
//! no LaTeX, no HTML-to-PDF wrapper. The clinical template is compiled to
//! PDF bytes in memory and handed to the OS default viewer.

use std::path::PathBuf;

use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime};
use typst::layout::PagedDocument;
use typst::syntax::{FileId, Source};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, World};

use crate::config::PharmacyConfig;
use crate::db::{Appointment, Drug, InterviewKind, Patient};

/// Default A4 carnet page. `entry(head, operator, body)` draws one
/// transmission; the operator's initials carry a stable colour so a
/// page can be scanned by who wrote what.
const DEFAULT_TRANS_TEMPLATE: &str = r##"
#set page(paper: "a4", margin: 2cm)
#set text(size: 11pt)

#let palette = (
  rgb("#3a547e"), rgb("#2e6e4e"), rgb("#7e3a5e"),
  rgb("#8b5a1a"), rgb("#1a6e8b"), rgb("#5e3a7e"),
)
#let op-color(op) = {
  if op == "" { rgb("#5c5f6e") } else {
    let sum = 0
    for b in bytes(op) { sum += b }
    palette.at(calc.rem(sum, palette.len()))
  }
}
#let entry(head, op, body) = block(above: 3mm, below: 0mm)[
  #box(fill: op-color(op), inset: (x: 3pt, y: 1.5pt))[
    #text(size: 8.5pt, weight: "bold", fill: white)[#head]
  ]
  #v(1mm)
  #body
]

#align(center)[#text(15pt, weight: "bold")[Carnet de transmissions]]
#v(1mm)
#align(center)[{{DAY}}]
#v(4mm)
#line(length: 100%, stroke: 0.6pt)
{{ENTRIES}}
"##;

/// Default A4 interview sheet: letterhead, the patient and who holds
/// the entretien, what the officine knows, the points of the theme as
/// tick-boxes, and three note boxes that **share what is left of the
/// page** — a long checklist shrinks them instead of pushing the
/// signature onto a second sheet, a short one leaves room to write.
const DEFAULT_TEMPLATE: &str = r#"
#set page(paper: "a4", margin: (x: 1.5cm, y: 1.3cm))
#set text(size: 10.5pt, lang: "fr")
#set block(spacing: 2.5mm)

#let sec(title) = block(sticky: true, above: 4mm, below: 2mm)[
  #text(weight: "bold")[#title]
  #v(-1.5mm)
  #line(length: 100%, stroke: 0.4pt)
]
#let note-box(title) = [
  #sec(title)
  #block(width: 100%, height: 1fr, stroke: 0.7pt)
]

#grid(columns: (1fr, auto),
  [#text(weight: "bold")[{{PHARMACY_NAME}}] \ #text(size: 9pt)[{{PHARMACY_PHONE}}]],
  [#align(right)[Le {{DATE}}]],
)
#v(3mm)
#align(center)[
  #text(15pt, weight: "bold")[Entretien pharmaceutique — {{KIND}}]
]
#v(2mm)
#box(width: 100%, stroke: 0.7pt, inset: 7pt)[
  #grid(columns: (1fr, 1fr), row-gutter: 1.5mm,
    [*Patient :* {{PATIENT_NAME}}], [*Né(e) le :* {{BIRTH_DATE}} ({{AGE}})],
    [*Thème :* {{THEME}}], [*Pharmacien :* {{PHARMACIST}}],
  )
]

#sec[Traitements connus à l'officine]
{{TREATMENTS}}

#sec[À couvrir pendant l'entretien]
{{CHECKLIST}}

#note-box[Propos du patient]
#note-box[Points d'attention, interactions]
#note-box[Conclusion et plan d'action]

#v(3mm)
#grid(columns: (1fr, 1fr), column-gutter: 6mm,
  [#text(weight: "bold")[Prochain rendez-vous]
   #v(1mm)
   #box(width: 100%, height: 1.8cm, stroke: 0.7pt)],
  [#text(weight: "bold")[Signature du pharmacien]
   #v(1mm)
   #box(width: 100%, height: 1.8cm, stroke: 0.7pt)],
)
"#;

/// Default CR letter to the médecin traitant: pharmacy letterhead,
/// patient and act, known treatments, and boxes for the handwritten
/// synthesis and signature.
const DEFAULT_CR_TEMPLATE: &str = r#"
#set page(paper: "a4", margin: 2cm)
#set text(size: 11pt)

#grid(columns: (1fr, auto),
  [#text(weight: "bold", size: 13pt)[{{PHARMACY_NAME}}] \
   {{PHARMACY_ADDRESS}} \
   {{PHARMACY_PHONE}}],
  [#align(right)[À l'attention du \ #text(weight: "bold")[{{PHYSICIAN}}]]],
)
#v(8mm)
#align(right)[Le {{DATE}}]
#v(4mm)
#text(weight: "bold")[Objet : {{KIND}} — {{PATIENT_NAME}} (né(e) le {{BIRTH_DATE}})] \
#text(weight: "bold")[Thème de l'entretien :] {{THEME}}
#v(4mm)
Docteur,

Dans le cadre d'un accompagnement à l'officine ({{KIND}}), nous avons reçu
votre patient(e) {{PATIENT_NAME}}. Vous trouverez ci-dessous les éléments
issus de cet échange.

#v(2mm)
#text(weight: "bold")[Traitements connus à l'officine :]

{{TREATMENTS}}

#v(2mm)
#text(weight: "bold")[Synthèse et points d'attention :]
#v(1mm)
{{POINTS}}

#v(1fr)
Restant à votre disposition, nous vous prions d'agréer, Docteur,
l'expression de nos salutations confraternelles.

#align(right)[{{PHARMACIST}}
#v(2mm)
#box(width: 6.5cm, height: 2.2cm, stroke: 0.7pt)]
"#;

/// Default A4 ordonnance for a dispensation after a positive TROD.
/// `{{LINES}}` receives the prescribed lines and `{{ADVICE}}` the advice
/// paragraphs the toggles switch on; either may be empty.
const DEFAULT_ORDONNANCE_TEMPLATE: &str = r#"
#set page(paper: "a4", margin: 2cm)
#set text(size: 11pt, lang: "fr")

#grid(columns: (1fr, auto),
  [#text(weight: "bold", size: 13pt)[{{PHARMACY_NAME}}] \
   {{PHARMACY_ADDRESS}} \
   {{PHARMACY_PHONE}} \
   #text(size: 9pt)[N° AM : {{PHARMACY_AM}}]],
  [#align(right)[Le {{DATE}}]],
)
#v(6mm)
#align(center)[#text(15pt, weight: "bold")[Ordonnance]]
{{MENTION_HEADER}}
#v(4mm)
#line(length: 100%, stroke: 0.6pt)
#v(3mm)

#text(weight: "bold")[Patient :] {{PATIENT_NAME}} — né(e) le {{BIRTH_DATE}} \
#text(weight: "bold")[Indication :] {{INDICATION}}
#v(5mm)

{{LINES}}

{{ADVICE}}
#v(1fr)
#line(length: 100%, stroke: 0.6pt)
#v(2mm)
{{PHARMACIST}}
#v(2mm)
#box(width: 6.5cm, height: 2.2cm, stroke: 0.8pt)
{{MENTION_FOOTER}}
"#;

/// A self-contained Typst world: one in-memory source, embedded fonts.
struct PdfWorld {
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<Font>,
    source: Source,
}

/// Parsing the embedded fonts takes long enough to be felt at the
/// counter: do it once per process, not once per click.
fn fonts() -> &'static (Vec<Font>, FontBook) {
    static FONTS: std::sync::OnceLock<(Vec<Font>, FontBook)> = std::sync::OnceLock::new();
    FONTS.get_or_init(|| {
        let fonts: Vec<Font> = typst_assets::fonts()
            .flat_map(|data| Font::iter(Bytes::new(data)))
            .collect();
        let book = FontBook::from_fonts(&fonts);
        (fonts, book)
    })
}

impl PdfWorld {
    fn new(text: String) -> Self {
        let (fonts, book) = fonts();
        Self {
            library: LazyHash::new(Library::default()),
            book: LazyHash::new(book.clone()),
            fonts: fonts.clone(),
            source: Source::detached(text),
        }
    }
}

impl World for PdfWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> FileId {
        self.source.id()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.source.id() {
            Ok(self.source.clone())
        } else {
            Err(FileError::NotFound(id.vpath().as_rootless_path().into()))
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        Err(FileError::NotFound(id.vpath().as_rootless_path().into()))
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).cloned()
    }

    fn today(&self, _offset: Option<i64>) -> Option<Datetime> {
        None
    }
}

/// Ce que la fiche d'entretien reçoit.
pub struct InterviewPaper<'a> {
    pub patient: &'a Patient,
    pub kind: InterviewKind,
    /// La date imprimée, déjà au format `JJ/MM/AAAA`.
    pub date: &'a str,
    /// L'âge à cette date, quand la naissance est connue.
    pub age: Option<u32>,
    pub theme: &'a str,
    pub signature: &'a str,
    pub treats: &'a [Drug],
    pub checklist: &'a [&'a str],
}

/// Compile the interview sheet for a patient and hand it to the OS PDF
/// viewer. `template_path` is [`crate::config::Config::template_path`]:
/// when the file does not exist, the embedded template is used.
pub fn open_interview_sheet(
    paper: &InterviewPaper,
    pharmacy: &PharmacyConfig,
    template_path: &std::path::Path,
) -> Result<PathBuf, String> {
    let filled = interview_source(paper, pharmacy, template_path)?;
    let stem = format!(
        "fiche_{}_{}",
        paper.patient.id,
        paper.kind.as_str().to_lowercase()
    );
    compile_and_open(filled, &stem)
}

/// La fiche d'entretien, remplie mais pas encore compilée. Voir
/// [`bilan_source`].
pub fn interview_source(
    paper: &InterviewPaper,
    pharmacy: &PharmacyConfig,
    template_path: &std::path::Path,
) -> Result<String, String> {
    let template = if template_path.exists() {
        std::fs::read_to_string(template_path)
            .map_err(|e| format!("modèle {} illisible : {e}", template_path.display()))?
    } else {
        DEFAULT_TEMPLATE.to_owned()
    };
    Ok(fill_interview_template(&template, paper, pharmacy))
}

/// Plusieurs documents en un seul PDF, dans l'ordre reçu.
///
/// **C'est la fin d'un entretien qu'on imprime**, pas un document :
/// bilan, plan de prise et fiche font trois boutons dans trois écrans,
/// à la fin d'un rendez-vous où l'on est déjà en retard.
///
/// Les pages se suivent par un `#pagebreak()` et chaque partie garde son
/// `#set page` — c'est ainsi que Typst change de mise en page en cours
/// de document, et c'est ce qui permet à un A4 portrait de suivre un
/// paysage sans que l'un impose sa marge à l'autre.
///
/// Une partie vide est sautée plutôt que de faire une page blanche : un
/// dossier sans traitement n'a pas de plan de prise, et une feuille
/// vierge au milieu d'une liasse se lit comme une erreur d'impression.
pub fn open_bundle(parts: &[String], stem: &str) -> Result<PathBuf, String> {
    compile_and_open(bundle_source(parts)?, stem)
}

/// Les parties mises bout à bout, pas encore compilées.
///
/// Séparée d'[`open_bundle`] pour que le test assemble **la liasse
/// elle-même** et non une copie de son assemblage : une deuxième
/// jonction écrite dans un test finirait par prouver que le test
/// fonctionne.
pub fn bundle_source(parts: &[String]) -> Result<String, String> {
    let joined = parts
        .iter()
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        // **Chaque partie dans son bloc** : un `#set` de la fiche
        // d'entretien (l'espacement de ses blocs, ou ce qu'une officine
        // ajoute à son propre modèle) s'appliquait au bilan et au plan
        // qui la suivent, mis en page plus serrés dans la liasse que
        // seuls. Un bloc de contenu borne les règles à sa partie.
        .map(|p| format!("#[\n{p}\n]"))
        .collect::<Vec<_>>()
        .join("\n#pagebreak()\n");
    if joined.is_empty() {
        return Err("rien à imprimer".to_owned());
    }
    Ok(joined)
}

/// The markers a template of each kind may use, in the order they
/// appear on the page. The editor shows them: a marker nobody knows
/// about is a marker nobody uses, and a mistyped one silently prints
/// itself.
pub fn template_markers(target: &str) -> &'static [&'static str] {
    doc(target).map_or(MARKERS_ORDONNANCE, |d| d.markers)
}

/// The embedded interview-sheet template, as a starting point for the
/// in-app editor.
pub fn default_template() -> &'static str {
    DEFAULT_TEMPLATE
}

fn sample_patient() -> Patient {
    Patient {
        id: 0,
        last_name: "Dupont".to_owned(),
        first_name: "Jean".to_owned(),
        birth_date: "1958-07-03".to_owned(),
        ..Default::default()
    }
}

/// One markup list line per treatment, each value escaped.
fn treatments_markup(treats: &[Drug]) -> String {
    if treats.is_empty() {
        return format!("- #{}", typst_str("(aucun traitement enregistré)"));
    }
    treats
        .iter()
        .map(|d| {
            let mut s = d.name.clone();
            if !d.dci.is_empty() {
                s.push_str(&format!(" ({})", d.dci));
            }
            if !d.class.is_empty() {
                s.push_str(&format!(" — {}", d.class));
            }
            if !d.dosage.is_empty() {
                s.push_str(&format!(" — {}", d.dosage));
            }
            format!("- #{}", typst_str(&s))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Substitute the CR-letter placeholders, all values escaped.
#[allow(clippy::too_many_arguments)]
fn fill_cr_template(
    template: &str,
    patient: &Patient,
    kind: InterviewKind,
    date: &str,
    theme: &str,
    treats: &[Drug],
    pharmacy: &PharmacyConfig,
    signature: &str,
    points: &[&str],
) -> String {
    let physician = if patient.physician.trim().is_empty() {
        "Médecin traitant"
    } else {
        patient.physician.trim()
    };
    fill(
        template,
        &[
            (
                "{{PHARMACY_NAME}}",
                format!("#{}", typst_str(&pharmacy.name)),
            ),
            (
                "{{PHARMACY_ADDRESS}}",
                format!("#{}", typst_str(&pharmacy.address)),
            ),
            (
                "{{PHARMACY_PHONE}}",
                format!("#{}", typst_str(&pharmacy.phone)),
            ),
            // Signed by whoever held the entretien, when the team list
            // knows those initials; by the officine's own line otherwise.
            ("{{PHARMACIST}}", format!("#{}", typst_str(signature))),
            ("{{PHYSICIAN}}", format!("#{}", typst_str(physician))),
            (
                "{{PATIENT_NAME}}",
                format!("#{}", typst_str(&patient.full_name())),
            ),
            (
                "{{BIRTH_DATE}}",
                format!(
                    "#{}",
                    typst_str(&crate::db::format_french_date(&patient.birth_date))
                ),
            ),
            ("{{KIND}}", format!("#{}", typst_str(kind.label()))),
            ("{{DATE}}", format!("#{}", typst_str(date))),
            // Un acte qui ne porte pas de thème n'en imprime pas, même
            // si la base en garde un : jusqu'à la 0.145 le thème armé
            // par le choix rapide était écrit sur les actes qui n'en ont
            // pas, et il ressortait ici. La source est corrigée ; ceci
            // couvre les lignes déjà écrites, sans réécrire la base.
            (
                "{{THEME}}",
                format!(
                    "#{}",
                    typst_str(theme_or_dash(if kind.has_theme() { theme } else { "" }))
                ),
            ),
            ("{{TREATMENTS}}", treatments_markup(treats)),
            // Ce qui a été retenu à l'export, ou le cadre vide.
            //
            // Vide veut dire vide : un courrier dont personne n'a coché de
            // point garde l'encadré qu'on remplit à la main, qui est ce que
            // le modèle portait avant que ce marqueur existe. Imprimer une
            // liste de points qu'on n'a pas choisis serait faire dire au
            // pharmacien ce qu'il n'a pas dit.
            ("{{POINTS}}", cr_points_markup(points)),
        ],
    )
}

/// Les points retenus, ou l'encadré à remplir quand il n'y en a pas.
fn cr_points_markup(points: &[&str]) -> String {
    if points.is_empty() {
        return "#box(width: 100%, height: 7cm, stroke: 0.7pt)".to_owned();
    }
    let list = points
        .iter()
        .map(|p| format!("#block(below: 2mm)[— #{}]", typst_str(p)))
        .collect::<Vec<_>>()
        .join("\n");
    // L'encadré reste sous la liste, plus court : le médecin y répond,
    // et c'est la moitié de l'intérêt d'envoyer la feuille.
    format!("{list}\n#v(2mm)\n#box(width: 100%, height: 3.5cm, stroke: 0.7pt)")
}

/// Compile the CR letter for a patient and open it in the OS viewer.
/// `template_path` behaves like the interview sheet's: the file when it
/// exists, the embedded default otherwise.
#[allow(clippy::too_many_arguments)]
pub fn open_cr_letter(
    patient: &Patient,
    kind: InterviewKind,
    date: &str,
    theme: &str,
    treats: &[Drug],
    pharmacy: &PharmacyConfig,
    template_path: &std::path::Path,
    signature: &str,
    points: &[&str],
) -> Result<PathBuf, String> {
    let template = if template_path.exists() {
        std::fs::read_to_string(template_path)
            .map_err(|e| format!("modèle {} illisible : {e}", template_path.display()))?
    } else {
        DEFAULT_CR_TEMPLATE.to_owned()
    };
    let filled = fill_cr_template(
        &template, patient, kind, date, theme, treats, pharmacy, signature, points,
    );
    compile_and_open(filled, &format!("cr_{}", patient.id))
}

fn sample_pharmacy() -> PharmacyConfig {
    PharmacyConfig {
        name: "Pharmacie du Centre".to_owned(),
        address: "1 place de la Mairie, 34000 Montpellier".to_owned(),
        phone: "04 67 00 00 00".to_owned(),
        pharmacist: "Dr Claire Leroy, pharmacien titulaire".to_owned(),
        am_number: "3400123".to_owned(),
        operators: Vec::new(),
        horaires: Vec::new(),
    }
}

/// Trois soirs de caisse pour l'aperçu du modèle : un soir recompté et
/// un soir sans recette attendue.
///
/// Les valeurs d'exemple ne sont **jamais vides** et jamais toutes du
/// même cas : un modèle validé sur trois lignes qui tombent juste
/// compile, et casse le premier soir où quelque chose ne tombe pas
/// juste. Ces deux cas-là sont exactement ceux que la feuille doit
/// savoir écrire.
fn sample_counted() -> Vec<crate::caisse::Counted> {
    vec![
        crate::caisse::Counted {
            id: 1,
            day: "2026-09-07".to_owned(),
            cash: 20_250,
            other: 45_075,
            float_kept: 15_000,
            opening: Some(15_000),
            expected: Some(51_000),
        },
        crate::caisse::Counted {
            id: 2,
            day: "2026-09-07".to_owned(),
            cash: 20_450,
            other: 45_075,
            float_kept: 15_000,
            opening: Some(15_000),
            expected: Some(51_000),
        },
        crate::caisse::Counted {
            id: 3,
            day: "2026-09-08".to_owned(),
            cash: 31_200,
            other: 52_300,
            float_kept: 15_000,
            opening: Some(15_000),
            expected: None,
        },
    ]
}

fn sample_caisse_history() -> Vec<CaisseHistoryRow> {
    let counts = sample_counted();
    let superseded = crate::caisse::superseded(&counts);
    counts
        .iter()
        .map(|c| CaisseHistoryRow {
            day: crate::db::format_french_date(&c.day),
            cash: c.cash,
            opening: c.opening,
            other: c.other,
            takings: c.takings(),
            expected: c.expected,
            gap: c.gap(),
            operator: "CL".to_owned(),
            remark: if c.id == 2 {
                "Recompté : un billet de 20 € était resté sous le tiroir.".to_owned()
            } else {
                String::new()
            },
            superseded: superseded.contains(&c.id),
        })
        .collect()
}

fn sample_treatments() -> Vec<Drug> {
    vec![
        Drug {
            name: "Eliquis".to_owned(),
            dci: "apixaban".to_owned(),
            class: "AOD".to_owned(),
            dosage: "5 mg x2/j".to_owned(),
            ..Default::default()
        },
        Drug {
            name: "Tahor".to_owned(),
            dci: "atorvastatine".to_owned(),
            class: "statine".to_owned(),
            ..Default::default()
        },
    ]
}

/// Build the Typst source for the conversion tables (all of them, one
/// A4 document). Every cell goes through the string escaping.
type TableEdits = std::collections::HashMap<(String, usize, usize), String>;

const MARKERS_TABLES: &[&str] = &["{{TABLES}}"];

const DEFAULT_TABLES_TEMPLATE: &str = r##"
#set page(paper: "a4", margin: 1.5cm)
#set text(size: 10pt, lang: "fr", hyphenate: true)
#align(center)[#text(15pt, weight: "bold")[Tables de conversion]]
{{TABLES}}
"##;

fn conversion_tables_values(edits: &TableEdits) -> Vec<(&'static str, String)> {
    let mut out = String::new();
    for t in crate::tables::TABLES {
        out.push_str(&format!(
            "#v(4mm)\n#text(weight: \"bold\", size: 12pt)[#{}]\n#v(1mm)\n",
            typst_str(t.title)
        ));
        // Fractional columns, so a long word wraps inside its cell
        // instead of spilling into the next one. The first column (the
        // molecule) gets a little more room than the others.
        let widths = std::iter::once("1.3fr")
            .chain(std::iter::repeat_n(
                "1fr",
                t.columns.len().saturating_sub(1),
            ))
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!(
            "#table(\n  columns: ({widths}),\n  inset: 5pt,\n  stroke: 0.6pt,\n"
        ));
        for c in t.columns {
            out.push_str(&format!("  [*#{}*],\n", typst_str(c)));
        }
        for (ri, row) in t.rows.iter().enumerate() {
            for (ci, cell) in row.iter().enumerate() {
                // The team's correction prints instead of the shipped
                // value, so paper and screen never disagree.
                let text = edits
                    .get(&(t.short.to_owned(), ri, ci))
                    .map(String::as_str)
                    .unwrap_or(cell);
                out.push_str(&format!("  [#{}],\n", typst_str(text)));
            }
        }
        out.push_str(")\n");
        let sources = t
            .sources
            .iter()
            .enumerate()
            .map(|(i, s)| format!("{}. {}", i + 1, s))
            .collect::<Vec<_>>()
            .join("   ");
        out.push_str(&format!(
            "#text(size: 8pt)[Relu en : #{} — Sources : #{}]\n",
            typst_str(t.reviewed),
            typst_str(&sources)
        ));
    }
    vec![("{{TABLES}}", out)]
}

/// One drug card as a printable A4 monograph: identity, every filled
/// section in reading order, the pharmacokinetics as a definition list
/// and the numbered sources at the foot.
pub fn open_drug_monograph(
    d: &Drug,
    posologies: &[crate::db::Posologie],
    sourced: &[(String, String)],
    template_path: &std::path::Path,
) -> Result<PathBuf, String> {
    compile_and_open(
        fill(
            &template_source("monographie", template_path),
            &monograph_values(d, posologies, sourced),
        ),
        &format!("monographie_{}", d.id),
    )
}

/// `sourced` : les valeurs de pharmacocinétique que l'officine a lues dans
/// le RCP, libellé et « valeur (source) » — celles que l'écran montre sous
/// la même rubrique, pour que la feuille dise ce que dit l'écran.
fn monograph_values(
    d: &Drug,
    posologies: &[crate::db::Posologie],
    sourced: &[(String, String)],
) -> Vec<(&'static str, String)> {
    let mut src = String::new();
    let mut sub = d.dci.trim().to_owned();
    if !d.class.trim().is_empty() {
        if !sub.is_empty() {
            sub.push_str(" — ");
        }
        sub.push_str(d.class.trim());
    }
    src.push_str(&format!(
        "#align(center)[#text(16pt, weight: \"bold\")[#{}]]\n",
        typst_str(&d.name.trim().to_uppercase())
    ));
    if !sub.is_empty() {
        src.push_str(&format!(
            "#align(center)[#text(11pt, style: \"italic\")[#{}]]\n",
            typst_str(&sub)
        ));
    }
    if !d.antidote.trim().is_empty() {
        src.push_str(&format!(
            "#align(center)[#text(10pt, weight: \"bold\")[Antidote : #{}]]\n",
            typst_str(d.antidote.trim())
        ));
    }
    src.push_str("#v(2mm)\n#line(length: 100%, stroke: 1pt)\n");
    if !d.status.trim().is_empty() {
        src.push_str(&format!(
            "#align(center)[#text(9pt)[Statut : #{}]]\n",
            typst_str(d.status.trim())
        ));
    }
    if !d.tags.trim().is_empty() {
        src.push_str(&format!(
            "#align(center)[#text(8pt, style: \"italic\")[#{}]]\n",
            typst_str(d.tags.trim())
        ));
    }
    for (title, body) in [
        ("Indications", d.indications.as_str()),
        ("Mécanisme d'action", d.mechanism.as_str()),
        ("Posologie", d.dosage.as_str()),
        ("Contre-indications", d.contraindications.as_str()),
        ("Interactions", d.ddi.as_str()),
        ("Effets indésirables", d.adverse.as_str()),
        ("Toxicité / marge thérapeutique", d.toxicity.as_str()),
        ("Surveillance", d.monitoring.as_str()),
        ("Conseils au patient", d.iup.as_str()),
        ("En cas d'oubli", d.missed_dose.as_str()),
        ("Signes d'alerte", d.red_flags.as_str()),
        ("Évaluation SMR / ASMR", d.smr.as_str()),
    ] {
        if body.trim().is_empty() {
            continue;
        }
        // First argument is code position: the quoted literal goes in
        // as is, without the `#` that only belongs in content.
        src.push_str(&format!(
            "#sec({}, [#{}])\n",
            typst_str(title),
            typst_str(body.trim())
        ));
    }
    if !posologies.is_empty() {
        let mut rows = String::new();
        for p in posologies {
            let right = if p.remarque.trim().is_empty() {
                format!("[#{}]", typst_str(p.posologie.trim()))
            } else {
                format!(
                    "[#{} #linebreak() #text(size: 8.5pt, style: \"italic\")[#{}]]",
                    typst_str(p.posologie.trim()),
                    typst_str(p.remarque.trim())
                )
            };
            rows.push_str(&format!(
                "  [#text(weight: \"bold\")[#{}]], {},\n",
                typst_str(p.indication.trim()),
                right
            ));
        }
        src.push_str(&format!(
            "#sec(\"Posologies par indication\", table(columns: (5cm, 1fr), inset: 3pt, \
             stroke: none,\n{rows}))\n"
        ));
    }
    let pk = [
        ("Formes et dosages", d.forms.as_str()),
        ("Demi-vie", d.half_life.as_str()),
        ("AUC / exposition", d.auc.as_str()),
        ("Élimination", d.elimination.as_str()),
        ("Adaptation DFG", d.renal.as_str()),
        ("Grossesse / allaitement", d.pregnancy.as_str()),
    ];
    if pk.iter().any(|(_, v)| !v.trim().is_empty()) || !sourced.is_empty() {
        let mut rows = String::new();
        for (label, value) in pk
            .iter()
            .map(|(l, v)| (*l, *v))
            .chain(sourced.iter().map(|(l, v)| (l.as_str(), v.as_str())))
        {
            if value.trim().is_empty() {
                continue;
            }
            rows.push_str(&format!(
                "  [#text(weight: \"bold\")[#{}]], [#{}],\n",
                typst_str(label),
                typst_str(value.trim())
            ));
        }
        src.push_str(&format!(
            "#sec(\"Pharmacocinétique\", table(columns: (4.5cm, 1fr), inset: 3pt, \
             stroke: none,\n{rows}))\n"
        ));
    }
    if !d.notes.trim().is_empty() {
        src.push_str(&format!(
            "#sec(\"Notes de l'équipe\", [#{}])\n",
            typst_str(d.notes.trim())
        ));
    }
    let sources: Vec<&str> = d
        .sources
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    if !sources.is_empty() {
        let list = sources
            .iter()
            .enumerate()
            .map(|(i, s)| format!("{}. {}", i + 1, s))
            .collect::<Vec<_>>()
            .join("\n");
        src.push_str(&format!(
            "#v(4mm)\n#line(length: 100%, stroke: 0.4pt)\n#v(1.5mm)\n\
             #text(size: 8pt)[Sources\\ #{}]\n",
            typst_str(&list)
        ));
    }
    vec![("{{BODY}}", src)]
}

const MARKERS_MONOGRAPHIE: &[&str] = &["{{BODY}}"];

const DEFAULT_MONOGRAPHIE_TEMPLATE: &str = r##"
#set page(paper: "a4", margin: 2cm)
#set text(size: 10.5pt)
#set par(justify: true, leading: 0.6em, spacing: 0.7em)

// Le style d'un intertitre de monographie, appelé par le corps.
#let sec(title, body) = block(above: 4.5mm, below: 0mm)[
  #block(above: 0mm, below: 1.2mm)[#text(size: 9pt, weight: "bold")[#upper(title)]
  #v(-1.2mm)
  #line(length: 100%, stroke: 0.4pt)]
  #body
]

{{BODY}}
"##;

/// Everything the file knows about one patient, gathered for the bilan
/// partagé de médication. The caller assembles it; this module only
/// lays it out.
pub struct BilanData<'a> {
    pub patient: &'a Patient,
    /// French date of the day the bilan is printed.
    pub today: &'a str,
    /// (nom, DCI et classe, posologie) — the treatments the file holds.
    pub treatments: Vec<(String, String, String)>,
    /// (A ↔ B, the sentence of A's monograph that names B).
    pub interactions: Vec<(String, String)>,
    /// (niveau, titre, ce que ça veut dire, les médicaments en cause) —
    /// what the ordonnance says about itself.
    pub review: Vec<(String, String, String, String)>,
    /// (date, analyte, valeur, lecture).
    pub biology: Vec<(String, String, String, String)>,
    /// (niveau, ce que ça change).
    pub findings: Vec<(String, String)>,
    /// (traitement et niveau, le risque, ce qu'on met à la place —
    /// **préfixé par l'appelant**, « À la place : » ou « Conduite : »
    /// selon le niveau, comme à l'écran) — ce que l'âge du dossier
    /// change, ligne par ligne.
    ///
    /// Le bilan partagé de médication est fait pour le patient
    /// polymédiqué, c'est-à-dire presque toujours pour un sujet âgé :
    /// c'est la feuille où cette lecture sert le plus, et la seule qui
    /// parte avec lui chez le prescripteur.
    pub elderly: Vec<(String, String, String)>,
    /// (où en est le dossier, l'analyte, le rythme, depuis quand, ce qui
    /// le demande) — ce que l'ordonnance réclame de faire vérifier.
    /// C'est la seule section du bilan qui parle de ce qui *manque*.
    pub watch: Vec<(String, String, String, String, String)>,
    /// What the calendrier vaccinal still owes.
    pub vaccines: Vec<String>,
    /// (date, acte, thème, état) — the year's accompaniment.
    pub acts: Vec<(String, String, String, String)>,
    /// Who signs it.
    pub signature: &'a str,
}

/// The bilan partagé de médication on paper: what the file knows, laid
/// out so the entretien can be held with it in hand, and with the
/// blanks the pharmacist fills during it.
pub fn open_bilan(
    data: &BilanData,
    pharmacy: &PharmacyConfig,
    template_path: &std::path::Path,
) -> Result<PathBuf, String> {
    compile_and_open(
        bilan_source(data, pharmacy, template_path),
        &format!("bilan_{}", data.patient.id),
    )
}

/// Le bilan, rempli mais pas encore compilé.
///
/// Séparé de [`open_bilan`] pour que l'impression groupée puisse le
/// mettre bout à bout avec les autres. **Le remplissage passe par la
/// même fonction dans les deux cas** : deux constructions d'une même
/// page finissent toujours par diverger, et c'est la version qu'on
/// regarde le moins qui a tort.
pub fn bilan_source(
    data: &BilanData,
    pharmacy: &PharmacyConfig,
    template_path: &std::path::Path,
) -> String {
    fill(
        &template_source("bilan", template_path),
        &bilan_values(data, pharmacy),
    )
}

fn bilan_values(data: &BilanData, pharmacy: &PharmacyConfig) -> Vec<(&'static str, String)> {
    let mut src = String::new();
    src.push_str(&format!(
        "#grid(columns: (1fr, auto), [#text(weight: \"bold\")[#{}]], [#align(right)[#text(9pt)[Bilan partagé de médication — #{}]]])\n",
        typst_str(&pharmacy.name),
        typst_str(data.today)
    ));
    src.push_str("#v(2mm)#line(length: 100%, stroke: 0.8pt)#v(3mm)\n");
    src.push_str(&format!(
        "#text(14pt, weight: \"bold\")[#{}] #h(4mm) #text(10pt)[né(e) le #{}]\n",
        typst_str(&data.patient.full_name()),
        typst_str(&crate::db::format_french_date(&data.patient.birth_date))
    ));
    let mut header = Vec::new();
    if !data.patient.physician.trim().is_empty() {
        header.push(format!(
            "Médecin traitant : {}",
            data.patient.physician.trim()
        ));
    }
    if !data.patient.phone.trim().is_empty() {
        header.push(format!("Tél : {}", data.patient.phone.trim()));
    }
    if !header.is_empty() {
        src.push_str(&format!(
            "\\\n#text(9pt)[#{}]\n",
            typst_str(&header.join("   ·   "))
        ));
    }

    // --- Treatments -------------------------------------------------
    src.push_str("#sec[Traitements connus à l'officine]\n");
    if data.treatments.is_empty() {
        src.push_str("#text(9.5pt, style: \"italic\")[Aucun traitement rattaché à la fiche.]\n");
    } else {
        let mut rows = String::new();
        for (name, about, poso) in &data.treatments {
            rows.push_str(&format!(
                "{}, {}, {},\n",
                typst_str(name),
                typst_str(about),
                typst_str(poso)
            ));
        }
        src.push_str(&format!(
            "#table(columns: (auto, 1fr, 1fr), inset: 5pt, stroke: 0.5pt,\n  [*Médicament*], [*DCI et classe*], [*Posologie*],\n{rows})\n"
        ));
    }

    // --- What the file itself can see -------------------------------
    if !data.interactions.is_empty() {
        src.push_str("#sec[Interactions repérées entre ces traitements]\n");
        for (pair, sentence) in &data.interactions {
            src.push_str(&format!(
                "#block(below: 2mm)[#text(weight: \"bold\", size: 9.5pt)[#{}] #linebreak() #text(9.5pt)[#{}]]\n",
                typst_str(pair),
                typst_str(sentence)
            ));
        }
    }

    // --- What the ordonnance says about itself ----------------------
    if !data.review.is_empty() {
        src.push_str("#sec[Revue de l'ordonnance]\n");
        for (level, title, detail, drugs) in &data.review {
            src.push_str(&format!(
                "#block(below: 2.4mm)[#text(8.5pt, weight: \"bold\")[#{}] #text(9.5pt, weight: \"bold\")[ #{}] #linebreak() #text(9.5pt)[#{}] #linebreak() #text(8.5pt, style: \"italic\")[#{}]]\n",
                typst_str(level),
                typst_str(title),
                typst_str(detail),
                typst_str(drugs)
            ));
        }
    }

    // --- What the age changes ---------------------------------------
    if !data.elderly.is_empty() {
        src.push_str("#sec[Sujet âgé]\n");
        for (head, risk, instead) in &data.elderly {
            src.push_str(&format!(
                "#block(below: 2.4mm)[#text(10pt, weight: \"bold\")[#{}] \\\n#text(9pt)[#{}] \\\n#text(9pt, style: \"italic\")[#{}]]\n",
                typst_str(head),
                typst_str(risk),
                typst_str(instead)
            ));
        }
        src.push_str("#text(8.5pt, style: \"italic\")[Listes de Laroche, STOPP/START et Beers, rapprochées de la date de naissance du dossier, sans tenir compte de la dose, de la durée ni de l'indication. Pas d'arrêt brutal : tout remplacement se prépare avec le prescripteur.]\n");
    }

    // --- Biology ----------------------------------------------------
    if !data.biology.is_empty() {
        src.push_str("#sec[Biologie]\n");
        let mut rows = String::new();
        for (date, label, value, level) in &data.biology {
            rows.push_str(&format!(
                "{}, {}, {}, {},\n",
                typst_str(date),
                typst_str(label),
                typst_str(value),
                typst_str(level)
            ));
        }
        src.push_str(&format!(
            "#table(columns: (auto, 1fr, auto, auto), inset: 5pt, stroke: 0.5pt,\n  [*Prélevé le*], [*Analyte*], [*Valeur*], [*Lecture*],\n{rows})\n"
        ));
    }
    if !data.findings.is_empty() {
        src.push_str("#v(2mm)\n");
        for (level, text) in &data.findings {
            src.push_str(&format!(
                "#block(below: 1.8mm)[#text(8.5pt, weight: \"bold\")[#{}] #text(9.5pt)[ — #{}]]\n",
                typst_str(level),
                typst_str(text)
            ));
        }
    }

    // --- What has not been asked for --------------------------------
    if !data.watch.is_empty() {
        src.push_str("#sec[À faire vérifier]\n");
        let mut rows = String::new();
        for (level, label, rhythm, since, by) in &data.watch {
            rows.push_str(&format!(
                "[#text(8pt, weight: \"bold\")[#{}]], [*#{}*], {}, {}, {},\n",
                typst_str(level),
                typst_str(label),
                typst_str(rhythm),
                typst_str(since),
                typst_str(by)
            ));
        }
        src.push_str(&format!(
            "#table(columns: (auto, auto, auto, 1fr, 1fr), inset: 5pt, stroke: 0.5pt,\n  [*État*], [*Analyte*], [*Rythme*], [*Dernier résultat*], [*Demandé par*],\n{rows})\n"
        ));
        src.push_str("#text(8.5pt, style: \"italic\")[Rythmes usuels des RCP et des recommandations : l'espacement réel est décidé par le prescripteur.]\n");
    }

    // --- Vaccines and acts ------------------------------------------
    if !data.vaccines.is_empty() {
        src.push_str("#sec[Vaccinations à faire]\n");
        for line in &data.vaccines {
            src.push_str(&format!(
                "#block(below: 1.2mm)[#text(9.5pt)[— #{}]]\n",
                typst_str(line)
            ));
        }
    }
    if !data.acts.is_empty() {
        src.push_str("#sec[Accompagnement à l'officine]\n");
        let mut rows = String::new();
        for (date, kind, theme, state) in &data.acts {
            rows.push_str(&format!(
                "{}, {}, {}, {},\n",
                typst_str(date),
                typst_str(kind),
                typst_str(theme),
                typst_str(state)
            ));
        }
        src.push_str(&format!(
            "#table(columns: (auto, auto, 1fr, auto), inset: 5pt, stroke: 0.5pt,\n  [*Date*], [*Acte*], [*Thème*], [*État*],\n{rows})\n"
        ));
    }

    // --- What is written during the entretien ------------------------
    src.push_str("#sec[Analyse pharmaceutique et points d'attention]\n");
    src.push_str("#box(width: 100%, height: 4.2cm, stroke: 0.7pt)\n");
    src.push_str("#sec[Plan d'action convenu avec le patient]\n");
    src.push_str("#box(width: 100%, height: 3.4cm, stroke: 0.7pt)\n");
    src.push_str("#v(3mm)\n");
    src.push_str(&format!(
        "#grid(columns: (1fr, auto), [#text(9pt)[Pharmacien : #{}]], [#box(width: 6cm, height: 1.8cm, stroke: 0.7pt)])\n",
        typst_str(data.signature)
    ));
    vec![("{{BODY}}", src)]
}

const MARKERS_BILAN: &[&str] = &["{{BODY}}"];

const DEFAULT_BILAN_TEMPLATE: &str = r##"
#set page(paper: "a4", margin: 1.6cm)
#set text(size: 10pt, lang: "fr", hyphenate: true)

#let sec(t) = block(sticky: true, above: 3mm, below: 1.5mm)[#text(11pt, weight: "bold")[#t] #v(1mm) #line(length: 100%, stroke: 0.6pt)]

{{BODY}}
"##;

/// The team's handout: what the application is for, view by view, with
/// the shortcuts at the foot. Printed rather than shown — it lives
/// beside the counter PC, not behind a menu.
pub fn open_guide(
    pharmacy: &PharmacyConfig,
    template_path: &std::path::Path,
) -> Result<PathBuf, String> {
    compile_and_open(
        fill(
            &template_source("guide", template_path),
            &guide_values(pharmacy),
        ),
        "mode_emploi",
    )
}

const MARKERS_GUIDE: &[&str] = &["{{PHARMACY_NAME}}", "{{SECTIONS}}"];

const DEFAULT_GUIDE_TEMPLATE: &str = r##"
#set page(paper: "a4", margin: 1.5cm, columns: 2)
#set text(size: 9pt, lang: "fr", hyphenate: true)
#set par(justify: true)

#let sec(t) = [#v(2.4mm) #text(10pt, weight: "bold")[#t] #v(0.8mm) #line(length: 100%, stroke: 0.5pt) #v(1mm)]

#place(top + center, scope: "parent", float: true)[
  #text(15pt, weight: "bold")[BPM-Caddy — mode d'emploi]
  #v(1mm)
  #text(9pt)[{{PHARMACY_NAME}}]
  #v(2mm)
]

{{SECTIONS}}
"##;

/// Les raccourcis, sur le mode d'emploi imprimé.
///
/// **Une liste recopiée vieillit là où personne ne la relit.** Celle-ci
/// en oubliait quatre — `F2`, `F9`, `Ctrl+Shift+Tab` et les chiffres du
/// choix rapide — parce qu'elle avait été écrite à la main et que rien
/// ne la reliait à la fenêtre `F12`. Le manuel de l'écran dit déjà de
/// cette liste-là qu'elle est « tenue par l'application elle-même, et
/// non recopiée ici » ; `every_shortcut_the_app_answers_to_is_on_the_
/// printed_guide` le rend vrai du papier aussi.
const GUIDE_SHORTCUTS: &str = "Ctrl+K aller à… · Ctrl+F chercher un patient · Ctrl+N nouvel entretien · Ctrl+Tab et Ctrl+Shift+Tab onglet suivant et précédent · Ctrl+W fermer l'onglet · F1 panneau d'équipe · F2 tableau de bord · F3 médicaments · F4 agenda · F5 carnet · F6 liste de gauche · F7 carte vaccinale · F9 la fenêtre réduite en barre, où les flèches haut et bas parcourent les fiches trouvées, gauche et droite tournent ses pages, et Alt + 1 … 5 appellent ses cinq gestes · F12 cette liste · Échap ferme l'élément ouvert. Dans une liste — patients, protocoles, préparations, dispositifs — tapez dans son champ de recherche, puis les flèches parcourent et Entrée ouvre ; sur un dossier, Alt et les flèches changent d'onglet, et dans le choix rapide les chiffres 1 … 9, 0 posent l'acte. Dates : 230826 donne 23/08/2026, 2308 donne le 23/08, l'année étant déduite du champ.";

/// Attache la ponctuation double au mot qu'elle accompagne.
///
/// Typst justifie et coupe où il veut : dans le mode d'emploi imprimé,
/// sur deux colonnes et en neuf points, trois lignes commençaient par
/// « » » — un guillemet fermant en tête de ligne, ce que la typographie
/// française ne fait pas. `lang: "fr"` règle la coupure des mots, pas
/// l'espace avant une ponctuation double : celle-là s'écrit, et c'est
/// une espace **insécable**.
///
/// C'est la même règle que `app::help_bound` applique au volet d'aide,
/// et pour la même raison ; ici l'espace peut être la fine que la
/// typographie française demande, puisque le PDF n'est pas dessiné avec
/// les fontes d'egui — celles-là n'ont pas le glyphe, et c'est pourquoi
/// le volet se contente de l'insécable ordinaire.
fn bind_french(text: &str) -> String {
    // L'apostrophe typographique : la prose des modèles la reçoit de
    // Typst, qui courbe celle du balisage, mais une chaîne passée ici
    // est du texte brut et gardait la droite — une même page imprimait
    // « l’officine » dans son titre et « l'état » deux lignes plus bas.
    text.replace('\'', "\u{2019}")
        .replace(" »", "\u{202f}»")
        .replace("« ", "«\u{202f}")
        .replace(" :", "\u{202f}:")
        .replace(" ;", "\u{202f};")
        .replace(" ?", "\u{202f}?")
        .replace(" !", "\u{202f}!")
}

fn guide_values(pharmacy: &PharmacyConfig) -> Vec<(&'static str, String)> {
    let mut sections = String::new();
    for (title, body) in GUIDE_SECTIONS {
        sections.push_str(&format!(
            "#sec[#{}]\n#text(9pt)[#{}]\n",
            typst_str(title),
            typst_str(body)
        ));
    }
    vec![
        (
            "{{PHARMACY_NAME}}",
            format!(
                "#{}",
                typst_str(if pharmacy.name.trim().is_empty() {
                    "Aide-mémoire de l'officine"
                } else {
                    pharmacy.name.trim()
                })
            ),
        ),
        ("{{SECTIONS}}", sections),
    ]
}

/// The guide itself: one paragraph per thing the counter does. Written
/// for someone who has never opened the application, and short enough
/// to be read standing up.
/// Combien de sections le mode d'emploi porte — lu par le README, qui
/// affirme ce nombre.
/// Elle n'existe que pour le test de la documentation : le README
/// affirme ce nombre.
#[cfg(test)]
pub fn guide_section_count() -> usize {
    GUIDE_SECTIONS.len()
}

const GUIDE_SECTIONS: &[(&str, &str)] = &[
    (
        "Ouvrir la base",
        "L'application demande le mot de passe de la base au démarrage : la base est chiffrée. « Verrouiller » (en haut à droite) ferme l'écran sans quitter ; le verrouillage est aussi automatique après le délai d'inactivité réglé dans les Options. Une sauvegarde du jour est écrite à chaque déverrouillage, dans le dossier « backups » à côté de la base.",
    ),
    (
        "Trouver ou créer un patient",
        "L'application s'ouvre sur la recherche. Tapez les premières lettres : « jndp » trouve Jean Dupont, les accents et la casse n'ont pas d'importance. Sans résultat, le même champ devient le formulaire de création. Entrée ouvre le résultat choisi, Échap referme.",
    ),
    (
        "Le dossier patient",
        "Le bandeau du haut porte l'identité, les traitements rattachés au référentiel médicaments (une puce par médicament, cliquable), et les éléments relevés automatiquement : les interactions entre ces traitements, et la revue d'ordonnance. En dessous, sept onglets : les entretiens, le fil du dossier, le carnet de vaccination, la biologie, les locations de matériel, la conciliation de sortie et les pièces numérisées.",
    ),
    (
        "Créer et suivre un entretien",
        "Ctrl+N ouvre le choix rapide : un chiffre par acte, le thème en option. La ligne créée porte, de gauche à droite, le code de l'acte et son rang dans la séquence, le thème, la date de réalisation (modifiable) et les initiales de l'opérateur, l'état, puis « » » pour avancer d'un état. Un acte avance jusqu'à « Facturé » ; « « » revient à l'état précédent.",
    ),
    (
        "Documents imprimés par l'acte",
        "Sur chaque ligne : « PDF » sort la fiche d'entretien à remplir, « CR » le courrier au médecin traitant avec les traitements connus, « Adhésion » le bulletin officiel de l'Assurance Maladie pré-rempli — les cases, la date et les signatures restent à faire devant le patient. Sur un TROD, « PDF » sort la feuille du test — signes d'orientation, score, lecture, lot et conduite —, et un TROD positif ouvre en plus l'ordonnance protocolisée.",
    ),
    (
        "Le bilan et le plan de prise",
        "En haut du dossier, « Bilan… » imprime le bilan partagé de médication à partir du dossier : traitements, interactions, revue d'ordonnance, sujet âgé, biologie, examens à faire vérifier, vaccinations dues, actes de l'année, et les cadres à remplir pendant l'entretien. « Plan de prise… » imprime la feuille que le patient emporte : indication, posologie et conduite à tenir en cas d'oubli, médicament par médicament.",
    ),
    (
        "La biologie",
        "L'onglet « Biologie » enregistre les résultats : choisissez l'analyte, tapez la valeur, la date si ce n'est pas aujourd'hui. Chaque valeur est lue contre son intervalle usuel, et le panneau « Interprétation » la relit contre les traitements du dossier (une kaliémie à 5,4 ne s'interprète pas de la même façon sous IEC). À côté, cinq autres analyses de la même ordonnance : surveillance biologique en retard, adaptation à la fonction rénale, grossesse et allaitement, sujet âgé, interactions par les cytochromes. Cliquez le nom d'un analyte pour voir sa courbe.",
    ),
    (
        "Le carnet de vaccination",
        "Les doses reçues, avec le lot et le site. À côté, « À faire » compare le carnet au calendrier vaccinal et signale les doses manquantes ; « Compléter le carnet… » inscrit d'un coup les doses dues, sans date, à corriger ligne par ligne. « Voyage » coche les vaccins recommandés pour les destinations notées au dossier.",
    ),
    (
        "Le référentiel médicaments (F3)",
        "Plus de huit cents fiches, trouvées dès deux lettres. La fiche s'ouvre comme une monographie imprimée ; les noms des autres médicaments y sont cliquables. À droite, la fiche technique repliable : demi-vie, élimination, adaptation rénale, grossesse. « Modifier » passe au formulaire — tout est modifiable, et les textes de l'équipe ne sont jamais réécrits par une mise à jour. Options › Interface choisit comment elle se lit : « Feuille » la pose sur du papier, « Dense » retire la feuille et les marges pour afficher deux fois plus de texte, « Lecture » resserre la colonne et espace les lignes. Aucune des trois ne change la taille des lettres.",
    ),
    (
        "La barre au-dessus des autres fenêtres (F9)",
        "La fenêtre réduite à une barre sans bordure, posée par-dessus le logiciel de comptoir. Elle se déplace par sa barre de titre ; le menu voisin d'« Agrandir » la place dans un coin, en bandeau en bas, en colonne à droite, ou libre. Saisie d'un nom ou d'une molécule, au clavier ou à la douchette ; les flèches haut et bas parcourent les fiches trouvées, gauche et droite tournent cinq pages : tables, posologie, conseils, précautions, ordonnance du dossier ouvert. Les pastilles rapportent dix analyses de la fiche cherchée : monographies du dossier, revue d'ordonnance, doublon de molécule sous un autre nom, interprétation de la biologie sous ce traitement, examens à refaire, cytochromes, écrasement, grossesse, rein au DFG du dossier et âge. Chacune cite le terme de sa table sans conclure ; un clic ouvre l'écran complet. Quand aucun nom ne répond, la barre cherche dans le texte des fiches. « Copier » met la page lue dans le presse-papier.",
    ),
    (
        "Les tables, le codex, les protocoles",
        "Depuis les médicaments : « Tables de conversion » (les références de comptoir, datées et sourcées, interrogées par une seule recherche), « Codex… » (les préparations de l'officine, avec la formule mise à la quantité prescrite et la fiche de fabrication), « Protocoles… » (les arbres de décision, à dérouler question par question au comptoir).",
    ),
    (
        "Chercher partout : « Aller à… » et « Dans le texte… »",
        "Ctrl+K ouvre la recherche globale : trois lettres suffisent pour lister les patients, les fiches, les tables, les préparations et les protocoles qui répondent, avec les flèches pour parcourir et Entrée pour ouvrir. Sa dernière ligne cherche le même mot dans le *texte* des fiches. Le même bouton se trouve dans les médicaments sous « Dans le texte… » : « pamplemousse », « allaitement », « QT », chaque fiche concernée s'affiche avec la phrase correspondante, mot surligné, la posologie et sa remarque comprises. Lorsqu'une fiche patient est ouverte, un bouton limite la recherche à ses seuls traitements.",
    ),
    (
        "L'agenda et le carnet de transmissions",
        "F4 ouvre la semaine : un bloc par rendez-vous, une couleur par acte, un clic ouvre le dossier. Le panneau du jour détaille les rendez-vous, les entrées qui ne sont pas des actes (formation, réunion, livraison, congé) et les notes du jour. F5 ouvre le carnet de transmissions : une page par jour, imprimable pour le classeur.",
    ),
    (
        "Le tableau de bord",
        "Le chiffre d'affaires facturé et en attente, le taux horaire, la charge des 28 prochains jours. « À revoir » est la liste d'appel : dossiers à rappeler pour la biologie ou l'ordonnance. « Récapitulatif de facturation… » imprime les actes à facturer ; « Exporter CSV » écrit l'ensemble dans un fichier directement lisible par un tableur.",
    ),
    (
        "Régler l'application",
        "« Options… » : l'identité de l'officine et l'équipe (les initiales signent les notes, le nom signe les documents), les mentions imprimées (vides par défaut : aucun avertissement n'est ajouté automatiquement), les honoraires par acte et par rang, les règles de quota, la base et les sauvegardes. « Modèles… » ouvre les sources des documents imprimables — fiche d'entretien, courrier, carnet, ordonnance, registre… — modifiables avec aperçu.",
    ),
    (
        "Raccourcis",
        GUIDE_SHORTCUTS,
    ),
    (
        "En cas de doute",
        "L'application propose, rappelle et calcule ; elle ne décide pas. Les intervalles de biologie sont ceux de l'adulte et celui du laboratoire prime ; les tables portent leur date de relecture et leurs sources ; les préparations ne se font que sur ordonnance et selon les bonnes pratiques. La base est partagée entre les postes : si un message dit qu'une ligne a changé ailleurs, relisez-la avant de réécrire.",
    ),
];

/// The patient's own copy: what they take, when, and what to do when a
/// dose is missed. Written for the person, not for the file — the
/// bilan stays at the officine, this goes home.
pub struct PlanData<'a> {
    pub patient: &'a Patient,
    pub today: &'a str,
    /// (médicament, indication, posologie, remarques) — one line per
    /// treatment.
    pub lines: Vec<(String, String, String, String)>,
    /// The officine's own mention, empty unless it wrote one.
    pub mention: &'a str,
    pub signature: &'a str,
}

/// The plan de prise on one sheet, in a size that is read without
/// glasses.
pub fn open_plan(
    data: &PlanData,
    pharmacy: &PharmacyConfig,
    template_path: &std::path::Path,
) -> Result<PathBuf, String> {
    compile_and_open(
        plan_source(data, pharmacy, template_path),
        &format!("plan_{}", data.patient.id),
    )
}

/// Le plan de prise, rempli mais pas encore compilé. Voir
/// [`bilan_source`].
pub fn plan_source(
    data: &PlanData,
    pharmacy: &PharmacyConfig,
    template_path: &std::path::Path,
) -> String {
    fill(
        &template_source("plan", template_path),
        &plan_values(data, pharmacy),
    )
}

const MARKERS_PLAN: &[&str] = &[
    "{{PATIENT_NAME}}",
    "{{DATE}}",
    "{{ROWS}}",
    "{{PHARMACY_NAME}}",
    "{{PHARMACY_PHONE}}",
    "{{SIGNATURE}}",
    "{{MENTION}}",
];

const DEFAULT_PLAN_TEMPLATE: &str = r##"
#set page(paper: "a4", margin: 1.6cm)
#set text(size: 11.5pt, lang: "fr", hyphenate: true)

#align(center)[#text(17pt, weight: "bold")[Plan de prise]]
#v(1mm)
#align(center)[#text(10pt)[{{PATIENT_NAME}} — {{DATE}}]]
#v(4mm)

#table(columns: (auto, 1fr, 1fr, 1.2fr), inset: 7pt, stroke: 0.6pt,
  [*Médicament*], [*Indication*], [*Posologie*], [*Remarques*],
{{ROWS}})

#v(4mm)
#text(10.5pt, weight: "bold")[Questions à poser]
#v(1.5mm)
#box(width: 100%, height: 3cm, stroke: 0.7pt)
#v(4mm)
#text(10pt)[Votre pharmacie : {{PHARMACY_NAME}} — {{PHARMACY_PHONE}}]
{{SIGNATURE}}
{{MENTION}}
"##;

fn plan_values(data: &PlanData, pharmacy: &PharmacyConfig) -> Vec<(&'static str, String)> {
    let mut rows = String::new();
    for (name, what, when, know) in &data.lines {
        rows.push_str(&format!(
            "  [*#{}*], {}, {}, {},\n",
            typst_str(name),
            typst_str(what),
            typst_str(when),
            typst_str(know)
        ));
    }
    // Un tableau Typst sans cellule ne compile pas : une ligne vide
    // vaut mieux qu'une erreur devant le patient.
    if rows.is_empty() {
        rows.push_str("  [], [], [], [],\n");
    }
    let signature = if data.signature.trim().is_empty() {
        String::new()
    } else {
        format!(
            "\\\n#text(10pt)[Préparé par #{}]",
            typst_str(data.signature.trim())
        )
    };
    let mention = if data.mention.trim().is_empty() {
        String::new()
    } else {
        format!(
            "#v(3mm)\n#text(8.5pt, style: \"italic\")[#{}]",
            typst_str(data.mention.trim())
        )
    };
    vec![
        (
            "{{PATIENT_NAME}}",
            format!("#{}", typst_str(&data.patient.full_name())),
        ),
        ("{{DATE}}", format!("#{}", typst_str(data.today))),
        ("{{ROWS}}", rows),
        (
            "{{PHARMACY_NAME}}",
            format!("#{}", typst_str(&pharmacy.name)),
        ),
        (
            "{{PHARMACY_PHONE}}",
            format!("#{}", typst_str(&pharmacy.phone)),
        ),
        ("{{SIGNATURE}}", signature),
        ("{{MENTION}}", mention),
    ]
}

// ---------------------------------------------------------------------
// La fiche de nouveau traitement
// ---------------------------------------------------------------------

/// Un médicament de l'ordonnance, tel que la fiche le porte.
///
/// **La grille et la phrase voyagent ensemble.** `reading` est la
/// lecture de `posology` par `intake.rs`, et `posology` reste le texte
/// du dossier, mot pour mot : la grille est une aide à la lecture, et
/// une aide à la lecture qui chasse ce qu'elle aide à lire est une
/// perte sèche. Un test le tient.
pub struct FicheLine {
    /// Le nom avec son dosage — « Amlor 5 mg ». Sur la feuille qui part
    /// à la maison, c'est ce qui est écrit sur la boîte.
    pub name: String,
    /// À quoi ça sert, dans les mots du dossier.
    pub what: String,
    /// La posologie de *ce* patient, telle qu'elle est au dossier.
    pub posology: String,
    /// Ce que `intake.rs` en a lu.
    pub reading: crate::intake::Reading,
    /// Ce qu'il faut savoir — le paragraphe d'information patient de la
    /// fiche.
    pub know: String,
    /// Ce qu'on fait quand une prise a été oubliée.
    pub missed: String,
    /// Les signes qui doivent faire consulter sans attendre.
    pub watch: String,
}

/// Une ordonnance du dossier et ce qu'elle couvre.
pub struct FicheStand {
    pub stand: crate::renewal::Stand,
    pub prescription: crate::renewal::Prescription,
    /// Les traitements que cette ordonnance porte.
    pub treatments: Vec<String>,
}

/// La fiche remise au patient pour une nouvelle ordonnance.
///
/// Quatre questions, dans l'ordre où elles se posent en sortant de la
/// pharmacie : *quand est-ce que je prends quoi* (la grille), *j'en ai
/// pour combien de temps et quand est-ce que je retourne voir le
/// médecin* (le renouvellement), *à quoi sert chacun et qu'est-ce que
/// je dois surveiller* (les blocs), *qui j'appelle* (le pied).
pub struct FicheData<'a> {
    pub patient: &'a Patient,
    /// Déjà en français.
    pub today: &'a str,
    /// Le prescripteur, s'il est connu. Vide = la ligne ne s'imprime
    /// pas : un « Dr — » se lit comme un défaut d'impression.
    pub prescriber: &'a str,
    pub lines: Vec<FicheLine>,
    pub stands: Vec<FicheStand>,
    pub viz: crate::renewal::Viz,
    /// La mention de l'officine, vide tant qu'elle n'en a pas écrit.
    pub mention: &'a str,
    pub signature: &'a str,
}

const MARKERS_TRAITEMENT: &[&str] = &[
    "{{PATIENT_NAME}}",
    "{{BIRTH_DATE}}",
    "{{DATE}}",
    "{{PRESCRIBER}}",
    "{{GRID}}",
    "{{RENEWAL}}",
    "{{PRODUCTS}}",
    "{{PHARMACY_NAME}}",
    "{{PHARMACY_PHONE}}",
    "{{SIGNATURE}}",
    "{{MENTION}}",
];

/// Le cadre est ce que l'officine réécrit ; les trois corps sont
/// calculés à partir du dossier et ne se recolonnent pas depuis un
/// modèle. C'est le partage honnête, celui de la monographie et du
/// bilan.
const DEFAULT_TRAITEMENT_TEMPLATE: &str = r##"
#set page(paper: "a4", margin: 1.4cm)
#set text(size: 10.5pt, lang: "fr", hyphenate: true)

#align(center)[#text(18pt, weight: "bold")[Fiche de traitement]]
#v(1.5mm)
#align(center)[#text(10pt)[{{PATIENT_NAME}} — né(e) le {{BIRTH_DATE}} — {{DATE}}]]
{{PRESCRIBER}}
#v(5mm)

#text(12.5pt, weight: "bold")[Prises de la journée]
#v(2mm)
{{GRID}}

{{RENEWAL}}

#v(5mm)
#text(12.5pt, weight: "bold")[Détail par médicament]
#v(2.5mm)
{{PRODUCTS}}

#v(3mm)
#text(11pt, weight: "bold")[Vos questions]
#v(1.5mm)
#box(width: 100%, height: 2.4cm, stroke: 0.7pt)

#v(4mm)
#line(length: 100%, stroke: 0.5pt)
#v(2mm)
#text(10pt)[Votre pharmacie : {{PHARMACY_NAME}} — {{PHARMACY_PHONE}}]
{{SIGNATURE}}
{{MENTION}}
"##;

/// La fiche, remplie mais pas encore compilée.
pub fn traitement_source(
    data: &FicheData,
    pharmacy: &PharmacyConfig,
    template_path: &std::path::Path,
) -> String {
    fill(
        &template_source("traitement", template_path),
        &traitement_values(data, pharmacy),
    )
}

/// La fiche de nouveau traitement, compilée et ouverte.
pub fn open_traitement(
    data: &FicheData,
    pharmacy: &PharmacyConfig,
    template_path: &std::path::Path,
) -> Result<PathBuf, String> {
    compile_and_open(
        traitement_source(data, pharmacy, template_path),
        &format!("traitement_{}", data.patient.id),
    )
}

/// Ce qu'une case de la grille porte.
///
/// Une quantité quand la posologie la donne, une pastille quand elle
/// donne le moment sans le nombre, rien sinon. **Jamais un « 1 »
/// inventé** : la règle est celle d'`intake.rs`, et elle s'applique au
/// dessin autant qu'à la lecture.
fn grid_cell(take: Option<crate::intake::Take>) -> String {
    match take {
        None => "[]".to_owned(),
        Some(t) => match t.label() {
            Some(n) => format!("[#text(weight: \"bold\")[#{}]]", typst_str(&n)),
            None => "[#circle(radius: 2.4pt, fill: rgb(\"#222222\"))]".to_owned(),
        },
    }
}

/// La grille du jour : une ligne par traitement, quatre colonnes de
/// moments.
///
/// Les lignes que la grille ne sait pas placer — à la demande, un
/// rythme qui n'est pas quotidien, une posologie illisible — occupent
/// les quatre colonnes d'un bloc et disent leur phrase. Quatre cases
/// blanches se liraient « rien à prendre », et c'est exactement le
/// contraire de ce qu'elles veulent dire.
fn grid_body(lines: &[FicheLine]) -> String {
    use crate::intake::Moment;
    let mut rows = String::new();
    for l in lines {
        let mut name = format!("[*#{}*", typst_str(&l.name));
        if !l.reading.meal.trim().is_empty() {
            name.push_str(&format!(
                " \\\n   #text(8.5pt, fill: rgb(\"#555555\"))[#{}]",
                typst_str(l.reading.meal.trim())
            ));
        }
        name.push(']');
        let what = format!("[#text(9pt)[#{}]]", typst_str(l.what.trim()));
        if l.reading.has_grid() {
            let cells: Vec<String> = Moment::ALL
                .iter()
                .map(|m| grid_cell(l.reading.doses[m.index()]))
                .collect();
            rows.push_str(&format!("  {name}, {}, {what},\n", cells.join(", ")));
        } else {
            rows.push_str(&format!(
                "  {name}, table.cell(colspan: 4)[#text(9.5pt)[#{}]], {what},\n",
                typst_str(&off_grid(l))
            ));
        }
    }
    if rows.is_empty() {
        rows.push_str("  [], [], [], [], [], [],\n");
    }
    // **La légende n'existe que si la pastille est dessinée.**
    // Expliquer un signe que la page ne porte pas est du bruit, et une
    // feuille de comptoir en a déjà assez.
    let legend = if lines.iter().any(|l| {
        l.reading.has_grid()
            && l.reading
                .doses
                .iter()
                .any(|d| d.is_some_and(|t| t.quarters.is_none()))
    }) {
        format!(
            "\n#v(1.5mm)\n#text(8.5pt, fill: rgb(\"#555555\"))[#{}]",
            typst_str(crate::strings::tr("fiche_grid_legend"))
        )
    } else {
        String::new()
    };
    // **Les quatre colonnes de moments ont une largeur fixe.** En
    // `auto`, c'est la cellule à cheval sur quatre — celle d'un « si
    // besoin » — qui décide de leur taille : « Coucher » prenait la
    // moitié de la feuille et « Médicament » sortait coupé en deux
    // syllabes. Une colonne qui ne porte qu'un chiffre ou une pastille
    // n'a pas à se mesurer sur une phrase.
    format!(
        "#table(\n  columns: (1fr, 1.85cm, 1.85cm, 1.85cm, 1.85cm, 1fr),\n  \
         align: (left + horizon, center + horizon, center + horizon, center + horizon, \
         center + horizon, left + horizon),\n  inset: 6pt, stroke: 0.5pt,\n  \
         fill: (_, row) => if row == 0 {{ rgb(\"#e4e4e4\") }},\n  \
         table.header([*Médicament*], [*{}*], [*{}*], [*{}*], [*{}*], [*Indication*]),\n{rows}){legend}",
        Moment::Matin.label(),
        Moment::Midi.label(),
        Moment::Soir.label(),
        Moment::Coucher.label(),
    )
}

/// Ce qu'écrit une ligne que la grille ne place pas.
///
/// **La posologie du dossier passe en entier, et une seule fois.** Un
/// « à la demande » et un rythme hebdomadaire la portent déjà — la
/// phrase les préface, elle ne les résume pas : « à la demande : si
/// besoin » suivi de « 1 comprimé si besoin » écrivait deux fois la
/// même condition et perdait la quantité, qui n'était que dans la
/// seconde moitié. Les deux autres cas — une quantité par jour dont le
/// moment n'est pas dit, une phrase illisible — ont un intitulé qui
/// n'est pas dans la posologie, et celle-ci les suit.
fn off_grid(l: &FicheLine) -> String {
    use crate::intake::Kind;
    let poso = l.posology.trim();
    match l.reading.kind {
        Kind::OnDemand => crate::strings::trf("fiche_on_demand", poso),
        Kind::Cyclic => crate::strings::trf("fiche_cyclic", poso),
        Kind::Daily if !poso.is_empty() => {
            format!("{poso} — {}", crate::strings::tr("fiche_daily_hour"))
        }
        Kind::Daily => crate::strings::trf("fiche_daily", l.reading.times),
        // Illisible : la phrase du dossier, mot pour mot. Y ajouter
        // « voir la posologie sur l'ordonnance » juste après l'avoir
        // écrite renvoie le lecteur à ce qu'il vient de lire ; la
        // phrase n'a de sens que lorsqu'il n'y a rien à écrire.
        _ if !poso.is_empty() => poso.to_owned(),
        _ => crate::strings::tr("fiche_unread").to_owned(),
    }
}

/// Le renouvellement, dessiné de la façon que l'officine a choisie.
///
/// Les quatre lisent le même état : `Stand::pips` est calculé une fois
/// et chacune le met en image. Il n'y a pas deux calculs de
/// l'avancement dans cette application.
fn renewal_body(data: &FicheData) -> String {
    use crate::renewal::{State, Step, Viz};
    if data.stands.is_empty() {
        return String::new();
    }
    let mut out = String::from(
        "#v(5mm)\n#text(12.5pt, weight: \"bold\")[Renouvellement de l'ordonnance]\n#v(2.5mm)\n",
    );
    for block in &data.stands {
        let s = &block.stand;
        out.push_str(&format!(
            "#block(width: 100%, inset: 7pt, stroke: 0.5pt, radius: 2pt)[\n  \
             #text(10pt, weight: \"bold\")[#{}]\n  #v(1.5mm)\n",
            typst_str(&block.treatments.join(", "))
        ));
        if s.state == State::Unknown {
            out.push_str(&format!(
                "  #text(9.5pt)[#{}]\n]\n#v(2.5mm)\n",
                typst_str(&crate::strings::trf("fiche_renewal_missing", s.missing))
            ));
            continue;
        }
        let pips = s.pips(&block.prescription);
        // Le dessin, quand il y a un avancement à montrer. Une
        // ordonnance non renouvelable n'en a pas : elle a une date de
        // fin, et une jauge à cent pour cent fabriquerait une question
        // là où il n'y en a pas.
        if s.has_progress() {
            match data.viz {
                Viz::Pastilles => {
                    let marks: Vec<String> = pips
                        .iter()
                        // Sans dièse : les arguments de `#stack` sont
                        // lus en mode code, où un `#` n'est pas valide.
                        .map(|p| match p.step {
                            // Faite et courante ne se distinguaient
                            // que par un filet : sur le papier, deux
                            // disques noirs à six points d'écart se
                            // comptent mais ne se lisent pas. La faite
                            // s'efface d'un ton, la courante garde
                            // l'encre pleine et son anneau.
                            Step::Done => {
                                "circle(radius: 5pt, fill: rgb(\"#999999\"), stroke: none)".to_owned()
                            }
                            Step::Current => {
                                "circle(radius: 5pt, fill: rgb(\"#111111\"), stroke: 1.6pt + rgb(\"#111111\"))"
                                    .to_owned()
                            }
                            Step::Todo => {
                                "circle(radius: 5pt, stroke: 0.8pt + rgb(\"#777777\"))".to_owned()
                            }
                        })
                        .collect();
                    out.push_str(&format!(
                        "  #stack(dir: ltr, spacing: 6pt, {})\n  #v(1.5mm)\n",
                        marks.join(", ")
                    ));
                }
                Viz::Jauge => {
                    let share = f64::from(s.step) / f64::from(s.steps.max(1));
                    out.push_str(&format!(
                        "  #box(width: 100%, height: 9pt, stroke: 0.6pt)[#place(left + horizon)[#rect(width: {:.0}%, height: 9pt, fill: rgb(\"#555555\"), stroke: none)]]\n  #v(1.5mm)\n",
                        share * 100.0
                    ));
                }
                Viz::Dates => {
                    let cells: Vec<String> = pips
                        .iter()
                        .map(|p| {
                            let day = p
                                .due
                                .as_deref()
                                .map(crate::db::format_french_date)
                                .unwrap_or_default();
                            let mark = match p.step {
                                Step::Done => "✓",
                                Step::Current => "→",
                                Step::Todo => "·",
                            };
                            format!(
                                "[#text(9pt)[{mark} #{} — #{}]]",
                                typst_str(&crate::strings::trf("fiche_renewal_step", p.n)),
                                typst_str(&day)
                            )
                        })
                        .collect();
                    out.push_str(&format!(
                        "  #table(columns: {}, inset: 4pt, stroke: none, {})\n  #v(1mm)\n",
                        cells.len().max(1),
                        cells.join(", ")
                    ));
                }
                Viz::Phrase => {}
            }
        }
        out.push_str(&format!(
            "  #text(9.5pt)[#{}]\n]\n#v(2.5mm)\n",
            typst_str(&renewal_sentence(s))
        ));
    }
    out
}

/// Ce que le renouvellement dit en toutes lettres — et il le dit même
/// quand un dessin l'accompagne : une pastille sans phrase se compte
/// mais ne se lit pas, et c'est la phrase qui porte la date du
/// rendez-vous.
fn renewal_sentence(s: &crate::renewal::Stand) -> String {
    use crate::renewal::State;
    use crate::strings::{tr, trf, trn};
    let day = |iso: &Option<String>| {
        iso.as_deref()
            .map(crate::db::format_french_date)
            .unwrap_or_default()
    };
    let mut out = String::new();
    if s.has_progress() {
        out.push_str(&trn("fiche_renewal_of", &[&s.step, &s.steps]));
    } else if s.steps == 1 {
        out.push_str(tr("fiche_renewal_single"));
    } else {
        out.push_str(tr("fiche_renewal_unrecorded"));
    }
    if let Some(covered) = &s.covered_to {
        out.push(' ');
        out.push_str(&trf(
            "fiche_renewal_covered",
            crate::db::format_french_date(covered),
        ));
    }
    match s.state {
        State::Over => {
            out.push(' ');
            out.push_str(&trf("fiche_renewal_over", day(&s.ends_on)));
        }
        _ => {
            out.push(' ');
            out.push_str(&trf("fiche_renewal_valid", day(&s.ends_on)));
            if s.see_by.is_some() && s.see_by != s.ends_on {
                out.push(' ');
                out.push_str(&trf("fiche_renewal_see_by", day(&s.see_by)));
            }
        }
    }
    out
}

/// Un bloc par médicament : ce que c'est, comment le prendre, ce qu'il
/// faut savoir, l'oubli, et ce qui doit faire appeler.
///
/// Une section vide ne s'imprime pas. Un intitulé suivi de rien se lit
/// comme un défaut d'impression, et sur une feuille qui porte
/// « Ce qui doit vous faire appeler » c'est la pire des lectures.
fn products_body(lines: &[FicheLine]) -> String {
    let mut out = String::new();
    for l in lines {
        out.push_str("#block(width: 100%, breakable: false)[\n");
        out.push_str(&format!(
            "  #text(11.5pt, weight: \"bold\")[#{}]\n",
            typst_str(&l.name)
        ));
        if !l.what.trim().is_empty() {
            out.push_str(&format!(
                "  #h(5pt) #text(9.5pt, fill: rgb(\"#555555\"))[#{}]\n",
                typst_str(l.what.trim())
            ));
        }
        out.push_str("  #v(1.5mm)\n");
        let mut section = |key: &'static str, body: &str| {
            let body = body.trim();
            if body.is_empty() {
                return;
            }
            out.push_str(&format!(
                "  #text(9.5pt)[*#{}* #{}]\\\n",
                typst_str(crate::strings::tr(key)),
                typst_str(body)
            ));
        };
        let how = match (l.posology.trim(), l.reading.meal.trim()) {
            ("", meal) => meal.to_owned(),
            (poso, "") => poso.to_owned(),
            (poso, meal) => format!("{poso} — {meal}"),
        };
        section("fiche_sec_how", &how);
        section("fiche_sec_know", &l.know);
        section("fiche_sec_missed", &l.missed);
        out.push_str("]\n");
        // Ce qui doit faire appeler sort du bloc et prend un cadre : il
        // est lu en diagonale, et la diagonale saute ce qui ressemble à
        // ce qu'elle vient de lire.
        if !l.watch.trim().is_empty() {
            out.push_str(&format!(
                "#block(width: 100%, inset: 5pt, stroke: 0.6pt, radius: 2pt)[#text(9.5pt)[*#{}* #{}]]\n",
                typst_str(crate::strings::tr("fiche_sec_watch")),
                typst_str(l.watch.trim())
            ));
        }
        out.push_str("#v(3mm)\n");
    }
    if out.is_empty() {
        out.push_str(&format!(
            "#text(10pt, style: \"italic\")[#{}]\n",
            typst_str(crate::strings::tr("fiche_no_treatment"))
        ));
    }
    out
}

fn traitement_values(data: &FicheData, pharmacy: &PharmacyConfig) -> Vec<(&'static str, String)> {
    let prescriber = if data.prescriber.trim().is_empty() {
        String::new()
    } else {
        format!(
            "#align(center)[#text(10pt)[#{}]]",
            typst_str(&crate::strings::trf(
                "fiche_prescriber",
                data.prescriber.trim()
            ))
        )
    };
    let signature = if data.signature.trim().is_empty() {
        String::new()
    } else {
        format!(
            "\\\n#text(10pt)[Préparé par #{}]",
            typst_str(data.signature.trim())
        )
    };
    let mention = if data.mention.trim().is_empty() {
        String::new()
    } else {
        format!(
            "#v(3mm)\n#text(8.5pt, style: \"italic\")[#{}]",
            typst_str(data.mention.trim())
        )
    };
    vec![
        (
            "{{PATIENT_NAME}}",
            format!("#{}", typst_str(&data.patient.full_name())),
        ),
        (
            "{{BIRTH_DATE}}",
            format!(
                "#{}",
                typst_str(&crate::db::format_french_date(&data.patient.birth_date))
            ),
        ),
        ("{{DATE}}", format!("#{}", typst_str(data.today))),
        ("{{PRESCRIBER}}", prescriber),
        ("{{GRID}}", grid_body(&data.lines)),
        ("{{RENEWAL}}", renewal_body(data)),
        ("{{PRODUCTS}}", products_body(&data.lines)),
        (
            "{{PHARMACY_NAME}}",
            format!("#{}", typst_str(&pharmacy.name)),
        ),
        (
            "{{PHARMACY_PHONE}}",
            format!("#{}", typst_str(&pharmacy.phone)),
        ),
        ("{{SIGNATURE}}", signature),
        ("{{MENTION}}", mention),
    ]
}

/// Un carnet de suivi, tel que le patient l'emporte.
///
/// **Le protocole d'abord, la grille ensuite.** Une grille se photocopie
/// n'importe où ; ce qu'une officine ne donne jamais, faute de l'avoir
/// sous la main, c'est la façon de mesurer — et une tension prise après
/// le café, debout, sur le bras qui traîne ne veut rien dire. La page
/// est donc ordonnée comme la consigne se dit au comptoir : comment
/// faire, ce qu'on vise, où écrire, et ce qui ne s'attend pas.
///
/// Le nom du patient est imprimé quand un dossier est ouvert, et
/// remplacé par une ligne à remplir sinon : une feuille vierge se donne
/// aussi bien, et une feuille au nom de quelqu'un d'autre ne se donne
/// pas du tout.
pub fn open_selfcheck(
    sheet: &crate::selfcheck::Resolved,
    patient: Option<&str>,
    pharmacy: &PharmacyConfig,
    today_french: &str,
    template_path: &std::path::Path,
) -> Result<PathBuf, String> {
    compile_and_open(
        fill(
            &template_source("suivi", template_path),
            &selfcheck_values(sheet, patient, pharmacy, today_french),
        ),
        &format!("carnet_{}", sheet.key),
    )
}

fn selfcheck_values(
    sheet: &crate::selfcheck::Resolved,
    patient: Option<&str>,
    pharmacy: &PharmacyConfig,
    today_french: &str,
) -> Vec<(&'static str, String)> {
    let mut src = String::new();
    src.push_str(&format!(
        "#align(center)[#text(17pt, weight: \"bold\")[#{}]]\n#v(1mm)\n",
        typst_str(&sheet.title)
    ));
    // Le nom, ou la ligne où l'écrire. Jamais un nom vide entre deux
    // tirets : une feuille se donne aussi bien vierge.
    match patient {
        Some(name) if !name.trim().is_empty() => src.push_str(&format!(
            "#align(center)[#text(11pt)[#{} — #{}]]\n",
            typst_str(name.trim()),
            typst_str(today_french)
        )),
        _ => src.push_str(&format!(
            "#align(center)[#text(11pt)[Nom : #box(width: 7cm, stroke: (bottom: 0.5pt))   #{}]]\n",
            typst_str(today_french)
        )),
    }
    src.push_str("#v(4mm)\n");

    // --- Comment mesurer ------------------------------------------
    src.push_str("#text(11pt, weight: \"bold\")[Méthode de mesure]\n#v(1.5mm)\n");
    for (i, step) in sheet.protocol.iter().enumerate() {
        src.push_str(&format!(
            "#text(10pt)[*{}.* #{}]\\\n",
            i + 1,
            typst_str(step)
        ));
    }

    // --- L'objectif ------------------------------------------------
    src.push_str(&format!(
        "#v(3mm)\n#block(width: 100%, inset: 6pt, stroke: 0.6pt)[#text(10pt)[*Objectif.* #{} #box(width: 5cm, stroke: (bottom: 0.5pt))]]\n",
        typst_str(&sheet.target)
    ));

    // --- La grille -------------------------------------------------
    //
    // Une colonne de date, puis les colonnes de la feuille, toutes de
    // même largeur : ce sont des cases où l'on écrit à la main, et une
    // colonne deux fois plus large que sa voisine invite à y écrire deux
    // fois plus, ce que le tableau ne relira pas.
    src.push_str("#v(4mm)\n");
    let widths = std::iter::once("2.2cm".to_owned())
        .chain(sheet.columns.iter().map(|_| "1fr".to_owned()))
        .collect::<Vec<_>>()
        .join(", ");
    let mut head = String::from("[*Date*], ");
    for c in &sheet.columns {
        head.push_str(&format!("[*#{}*], ", typst_str(c)));
    }
    // Une case vide s'écrit `[]` et non rien : deux virgules qui se
    // suivent ne sont pas une case, c'est une erreur de syntaxe.
    let empty = "[], ".repeat(sheet.columns.len());
    let mut body = String::new();
    for _ in 0..sheet.rows {
        // Une rangée de cases vides, assez hautes pour qu'un chiffre
        // écrit à la main y tienne.
        body.push_str(&format!("[#v(6mm)], {empty}\n"));
    }
    src.push_str(&format!(
        "#table(columns: ({widths}), inset: 5pt, stroke: 0.5pt,\n  {head}\n{body})\n"
    ));

    // --- Ce qui se calcule au bas de la grille ---------------------
    //
    // L'automesure tensionnelle n'existe que pour cela : ce que le
    // médecin lit n'est aucune des dix-huit mesures, c'est leur
    // moyenne. Sans la case, elle s'additionne en consultation — ou
    // pas du tout.
    if !sheet.totals.is_empty() {
        src.push_str("#v(3mm)\n");
        for label in &sheet.totals {
            src.push_str(&format!(
                "#text(10pt)[#{} : #box(width: 3.5cm, stroke: (bottom: 0.5pt))]\\\n",
                typst_str(label)
            ));
        }
    }

    // --- Ce qui ne s'attend pas ------------------------------------
    src.push_str(&format!(
        "#v(4mm)\n#block(width: 100%, inset: 6pt, stroke: 0.8pt)[#text(10pt, weight: \"bold\")[À signaler sans attendre]\\\n#text(10pt)[#{}]]\n",
        typst_str(&sheet.alert)
    ));
    src.push_str(&format!(
        "#v(3mm)\n#text(10pt)[#{}]\n",
        typst_str(&sheet.bring_back)
    ));
    src.push_str(&format!(
        "#v(3mm)\n#text(9.5pt)[Votre pharmacie : #{} — #{}]\n",
        typst_str(&pharmacy.name),
        typst_str(&pharmacy.phone)
    ));
    vec![("{{BODY}}", src)]
}

/// La source Typst d'une feuille de suivi, pour le test qui vérifie
/// qu'une réécriture de l'officine atteint bien le papier.
///
/// Le chemin réel passe par un fichier ouvert dans le lecteur PDF ; ce
/// qu'il faut pouvoir relire est ce qui part à la compilation.
#[cfg(test)]
pub fn selfcheck_source_for_test(sheet: &crate::selfcheck::Resolved) -> String {
    fill(
        DEFAULT_SUIVI_TEMPLATE,
        &selfcheck_values(sheet, None, &sample_pharmacy(), "11/09/2026"),
    )
}

const MARKERS_SUIVI: &[&str] = &["{{BODY}}"];

const DEFAULT_SUIVI_TEMPLATE: &str = r##"
#set page(paper: "a4", margin: 1.4cm)
#set text(size: 10.5pt, lang: "fr", hyphenate: true)

{{BODY}}
"##;

pub struct CallRow<'a> {
    pub name: &'a str,
    pub phone: &'a str,
    /// Pourquoi ce dossier est sur la liste : « 2 alerte(s) »,
    /// « 1 à refaire »…
    pub tag: &'a str,
    pub reason: &'a str,
}

/// La liste d'appel sur papier.
///
/// Le tableau de bord dit qui rappeler ; il ne dit rien de ce qu'on a
/// fait de l'appel. Cette feuille se coche et s'annote au téléphone,
/// avec une case et une colonne vide pour ce qui a été dit — c'est ce
/// qui permet de reprendre la liste le lendemain sans rappeler deux
/// fois les mêmes.
pub fn open_call_list(
    rows: &[CallRow],
    today: &str,
    pharmacy: &PharmacyConfig,
    template_path: &std::path::Path,
) -> Result<PathBuf, String> {
    compile_and_open(
        fill(
            &template_source("appels", template_path),
            &call_list_values(rows, today, pharmacy),
        ),
        "liste_appel",
    )
}

const MARKERS_APPELS: &[&str] = &["{{PHARMACY_NAME}}", "{{DATE}}", "{{ROWS}}"];

const DEFAULT_APPELS_TEMPLATE: &str = r##"
#set page(paper: "a4", margin: 1.5cm)
#set text(size: 10pt, lang: "fr", hyphenate: true)

#align(center)[#text(15pt, weight: "bold")[Liste d'appel]]
#v(1mm)
#align(center)[#text(10pt)[{{PHARMACY_NAME}} — {{DATE}}]]
#v(4mm)

#table(columns: (auto, auto, auto, auto, 1.4fr, 1fr), inset: 5pt, stroke: 0.5pt,
  [], [*Patient*], [*Téléphone*], [*Motif*], [*Dossier*], [*Réponse*],
{{ROWS}})

#v(4mm)
#text(9pt, style: "italic")[Liste établie le {{DATE}} d'après l'état de la base ; à réimprimer à chaque usage.]
"##;

fn call_list_values(
    rows: &[CallRow],
    today: &str,
    pharmacy: &PharmacyConfig,
) -> Vec<(&'static str, String)> {
    let mut body = String::new();
    for r in rows {
        body.push_str(&format!(
            "  [#box(width: 4mm, height: 4mm, stroke: 0.6pt)], [*#{}*], {}, [#text(8pt, weight: \"bold\")[#{}]], {}, [],\n",
            typst_str(r.name),
            typst_str(r.phone),
            typst_str(r.tag),
            typst_str(r.reason)
        ));
    }
    if body.is_empty() {
        body.push_str("  [], [], [], [], [], [],\n");
    }
    vec![
        (
            "{{PHARMACY_NAME}}",
            format!("#{}", typst_str(&pharmacy.name)),
        ),
        ("{{DATE}}", format!("#{}", typst_str(today))),
        ("{{ROWS}}", body),
    ]
}

/// La liste de ce qu'il faut aller compter, sur papier.
///
/// Elle sort du placard avec la clé : on lit le libellé, on compte, on
/// écrit ce qu'on a trouvé dans la colonne vide, et on ressaisit ensuite.
/// C'est pour cela que le solde du registre est **imprimé** en face —
/// compter à l'aveugle est plus honnête, mais recompter tout un placard
/// sans savoir ce qu'on cherche est ce qui fait qu'on ne le fait pas.
///
/// Aucun nom de patient : c'est une feuille qui traîne sur une paillasse.
pub fn open_stock_check(
    rows: &[crate::ordonnancier::ToCheck],
    pharmacy: &PharmacyConfig,
    today: &str,
    template_path: &std::path::Path,
) -> Result<PathBuf, String> {
    compile_and_open(
        fill(
            &template_source("controle", template_path),
            &stock_check_values(rows, pharmacy, today),
        ),
        "controle_stock",
    )
}

/// Le procès-verbal de destruction : ce qu'on s'apprête à détruire, et
/// devant qui.
///
/// Un stupéfiant rapporté par un patient ne se jette pas et ne se
/// délivre plus : il se dénature en présence d'un confrère et la
/// destruction s'inscrit au registre. Ce qui rend cette ligne
/// vérifiable est la pièce qu'elle cite, et cette pièce n'existait
/// nulle part — c'est celle-ci. Elle sort **avant** la destruction, on
/// coche à mesure, et les deux signatures se posent au bas.
///
/// Aucun nom de patient : c'est une feuille qui sort du logiciel, se
/// pose sur une paillasse et se garde dix ans. Le numéro de dossier de
/// chaque retour est au registre, qui est l'endroit pour cela.
pub fn open_destruction_list(
    rows: &[crate::ordonnancier::Awaiting],
    pharmacy: &PharmacyConfig,
    today: &str,
    template_path: &std::path::Path,
) -> Result<PathBuf, String> {
    compile_and_open(
        fill(
            &template_source("destruction", template_path),
            &destruction_list_values(rows, pharmacy, today),
        ),
        "proces_verbal_destruction",
    )
}

const MARKERS_DESTRUCTION: &[&str] = &["{{PHARMACY_NAME}}", "{{DATE}}", "{{ROWS}}"];

const DEFAULT_DESTRUCTION_TEMPLATE: &str = r##"
#set page(paper: "a4", margin: 1.5cm)
#set text(size: 10pt, lang: "fr", hyphenate: true)

#align(center)[#text(15pt, weight: "bold")[Procès-verbal de destruction de stupéfiants]]
#v(1mm)
#align(center)[#text(10pt)[{{PHARMACY_NAME}} — {{DATE}}]]
#v(4mm)

#table(columns: (auto, 1.4fr, auto, auto, auto, 1fr), inset: 5pt, stroke: 0.5pt,
  [], [*Produit*], [*Au coffre*], [*En attente depuis*], [*Détruit*], [*Observation*],
{{ROWS}})

#v(6mm)
#text(9pt)[Dénaturation effectuée le : #box(width: 3cm, stroke: (bottom: 0.5pt))   Procédé : #box(width: 6cm, stroke: (bottom: 0.5pt))]
#v(4mm)
#text(9pt)[Le pharmacien : #box(width: 6cm, stroke: (bottom: 0.5pt))   Le témoin : #box(width: 6cm, stroke: (bottom: 0.5pt))]
#v(3mm)
#text(9pt, style: "italic")[Les quantités ci-dessus sont celles du compte « à détruire » du registre : retours de patients non réintégrés au stock délivrable. La destruction se porte au registre ligne par ligne, en citant le numéro du présent procès-verbal. Un stupéfiant rapporté ne se redélivre jamais.]
"##;

fn destruction_list_values(
    rows: &[crate::ordonnancier::Awaiting],
    pharmacy: &PharmacyConfig,
    today: &str,
) -> Vec<(&'static str, String)> {
    let mut body = String::new();
    for r in rows {
        let since = match (r.since.is_empty(), r.days) {
            (false, Some(d)) => format!("{} ({d} j)", crate::db::format_french_date(&r.since)),
            (false, None) => crate::db::format_french_date(&r.since),
            (true, _) => String::new(),
        };
        body.push_str(&format!(
            "  [#box(width: 4mm, height: 4mm, stroke: 0.6pt)], [*#{}*], [#{}], [#{}], [], [],\n",
            typst_str(&r.label),
            typst_str(&crate::ordonnancier::quantity_and_unit(r.quantity, &r.unit)),
            typst_str(&since),
        ));
    }
    if body.is_empty() {
        body.push_str("  [], [], [], [], [], [],\n");
    }
    vec![
        (
            "{{PHARMACY_NAME}}",
            format!("#{}", typst_str(&pharmacy.name)),
        ),
        (
            "{{DATE}}",
            format!("#{}", typst_str(&crate::db::format_french_date(today))),
        ),
        ("{{ROWS}}", body),
    ]
}

const MARKERS_CONTROLE: &[&str] = &["{{PHARMACY_NAME}}", "{{DATE}}", "{{ROWS}}"];

const DEFAULT_CONTROLE_TEMPLATE: &str = r##"
#set page(paper: "a4", margin: 1.5cm)
#set text(size: 10pt, lang: "fr", hyphenate: true)

#align(center)[#text(15pt, weight: "bold")[Contrôle des stupéfiants]]
#v(1mm)
#align(center)[#text(10pt)[{{PHARMACY_NAME}} — {{DATE}}]]
#v(4mm)

#table(columns: (auto, 1.4fr, auto, auto, auto, auto, 1fr), inset: 5pt, stroke: 0.5pt,
  [], [*Produit*], [*Au registre*], [*Motif*], [*Dernier comptage*], [*Compté*], [*Observation*],
{{ROWS}})

#v(6mm)
#text(9pt)[Compté par : #box(width: 5cm, stroke: (bottom: 0.5pt))   Le : #box(width: 3cm, stroke: (bottom: 0.5pt))   Signature : #box(width: 4cm, stroke: (bottom: 0.5pt))]
#v(3mm)
#text(9pt, style: "italic")[Tout écart entre le comptage et le registre est porté au registre par une ligne d'inventaire, avec son explication. Le registre ne se rature pas.]
"##;

fn stock_check_values(
    rows: &[crate::ordonnancier::ToCheck],
    pharmacy: &PharmacyConfig,
    today: &str,
) -> Vec<(&'static str, String)> {
    let mut body = String::new();
    for r in rows {
        let since = match r.days {
            Some(d) => format!("{d} j"),
            None => "jamais".to_owned(),
        };
        body.push_str(&format!(
            "  [#box(width: 4mm, height: 4mm, stroke: 0.6pt)], [*#{}*], [#{}], [#{}], [#{}], [], [],\n",
            typst_str(&r.label),
            typst_str(&crate::ordonnancier::quantity_and_unit(r.stock, &r.unit)),
            typst_str(crate::strings::tr(r.why.label_key())),
            typst_str(&since),
        ));
    }
    if body.is_empty() {
        body.push_str("  [], [], [], [], [], [], [],\n");
    }
    vec![
        (
            "{{PHARMACY_NAME}}",
            format!("#{}", typst_str(&pharmacy.name)),
        ),
        (
            "{{DATE}}",
            format!("#{}", typst_str(&crate::db::format_french_date(today))),
        ),
        ("{{ROWS}}", body),
    ]
}

/// L'ordonnancier d'une année : la suite des délivrances, tous produits
/// confondus, dans l'ordre de leurs numéros.
///
/// C'est ce qu'un contrôle demande, et c'est la seule vue où le manque
/// d'un numéro se voit. Le patient y est **un numéro de dossier** et
/// jamais un nom : une feuille imprimée sort du logiciel, se pose sur un
/// comptoir et se garde dix ans ; ce qu'elle doit permettre, c'est de
/// remonter au dossier, pas d'afficher qui prend de la morphine.
///
/// Une ligne annulée est imprimée **annulée**, avec son motif, jamais
/// retirée : une feuille d'où l'on aurait ôté les erreurs ne serait pas
/// une copie du registre.
/// Le registre d'un produit, tel qu'un contrôle le lit.
///
/// Deux documents s'imprimaient : la liste d'inventaire et
/// l'ordonnancier de l'année. **Celui-ci manquait**, et c'est pourtant
/// la page qu'une inspection demande : un produit, ses lignes dans
/// l'ordre des jours, ce qui entre, ce qui sort, et le solde en face de
/// chacune.
#[allow(clippy::too_many_arguments)]
pub fn open_stup_register(
    label: &str,
    unit: &str,
    rows: &[crate::db::StupMove],
    running: &[crate::ordonnancier::Balance],
    cancelled: &std::collections::HashSet<i64>,
    pharmacy: &PharmacyConfig,
    today: &str,
    template_path: &std::path::Path,
) -> Result<PathBuf, String> {
    compile_and_open(
        fill(
            &template_source("registre", template_path),
            &stup_register_values(label, unit, rows, running, cancelled, pharmacy, today),
        ),
        "registre_stupefiant",
    )
}

fn stup_register_values(
    label: &str,
    unit: &str,
    rows: &[crate::db::StupMove],
    running: &[crate::ordonnancier::Balance],
    cancelled: &std::collections::HashSet<i64>,
    pharmacy: &PharmacyConfig,
    today: &str,
) -> Vec<(&'static str, String)> {
    use crate::ordonnancier::Kind;
    let mut body = String::new();
    for (i, m) in rows.iter().enumerate() {
        let struck = cancelled.contains(&m.id);
        let cell = |s: &str| {
            if struck {
                format!("[#strike[#{}]]", typst_str(s))
            } else {
                format!("[#{}]", typst_str(s))
            }
        };
        let kind = Kind::from_key(&m.kind);
        let qty = crate::codex::format_quantity(m.quantity);
        // Deux colonnes, comme sur le papier : ce qui entre et ce qui
        // sort. Une annulation ne porte de quantité dans ni l'une ni
        // l'autre — ce qu'elle rend se lit sur la ligne qu'elle nomme.
        //
        // Un retour entre et une destruction sort : de l'autre compte,
        // ce que dit la colonne « À détruire » en face. Les colonnes de
        // quantité disent ce qui a bougé, les colonnes de solde disent
        // dans quel compte — c'est ainsi qu'un registre à deux comptes
        // se lit, et la note du bas le redit en toutes lettres.
        let (into, out) = match kind {
            Kind::Entree | Kind::Retour => (qty.clone(), String::new()),
            Kind::Sortie
            | Kind::Perte
            | Kind::Destruction
            | Kind::Peremption
            | Kind::DestructionPerimes => (String::new(), qty.clone()),
            Kind::Inventaire => (format!("= {qty}"), String::new()),
            Kind::Annulation => (String::new(), String::new()),
        };
        let after = running.get(i).copied().unwrap_or_default();
        // La colonne du coffre ne s'écrit que si quelque chose y est
        // passé : un registre sans aucun retour — l'immense majorité —
        // ne porte pas une colonne de zéros.
        // **Deux coffres, deux colonnes.** Les additionner annoncerait
        // un sac là où il y en a deux, qui ne se détruisent pas
        // ensemble et ne relèvent pas du même procès-verbal. Chacune ne
        // s'écrit que si quelque chose y est passé : un registre sans
        // aucun retour ni aucun périmé — l'immense majorité — ne porte
        // pas deux colonnes de zéros.
        let waiting = if after.to_destroy.abs() > 1e-6 || kind.is_destruction_side() {
            crate::codex::format_quantity(after.to_destroy)
        } else {
            String::new()
        };
        let expired = if after.expired.abs() > 1e-6 || kind.is_expiry_side() {
            crate::codex::format_quantity(after.expired)
        } else {
            String::new()
        };
        let no = if m.ordo_no > 0 {
            crate::ordonnancier::number_label(m.ordo_no as u32)
        } else if kind.is_dispensing() {
            crate::strings::tr("stup_no_pending").to_owned()
        } else {
            String::new()
        };
        let file = if m.patient_id > 0 {
            format!("dossier {}", m.patient_id)
        } else {
            String::new()
        };
        // Le lot est **nommé** sur le papier comme à l'écran : « L4821B »
        // seul entre un bon de livraison et une remarque ne se
        // distingue pas d'une référence de commande.
        let lot = if m.lot.trim().is_empty() {
            String::new()
        } else {
            format!("lot {}", m.lot.trim())
        };
        let side = [
            m.prescriber.as_str(),
            m.supplier.as_str(),
            m.reference.as_str(),
            lot.as_str(),
            m.remark.as_str(),
            m.operator.as_str(),
        ]
        .into_iter()
        .filter(|t| !t.is_empty())
        .collect::<Vec<_>>()
        .join(" · ");
        body.push_str(&format!(
            "{}, {}, {}, {}, {}, {}, {}, {}, {}, [#{}],\n",
            cell(&crate::db::format_french_date(&m.happened_on)),
            cell(&no),
            cell(crate::strings::tr(kind.label_key())),
            cell(&into),
            cell(&out),
            cell(&crate::codex::format_quantity(after.stock)),
            cell(&waiting),
            cell(&expired),
            cell(&file),
            typst_str(&if struck {
                format!("annulée · {side}")
            } else {
                side
            }),
        ));
    }
    if body.is_empty() {
        body.push_str("[], [], [], [], [], [], [], [], [], [],\n");
    }
    vec![
        ("{{PRODUCT}}", format!("#{}", typst_str(label))),
        (
            "{{PHARMACY_NAME}}",
            format!("#{}", typst_str(&pharmacy.name)),
        ),
        (
            "{{DATE}}",
            format!("#{}", typst_str(&crate::db::format_french_date(today))),
        ),
        ("{{ROWS}}", body),
        ("{{COUNT}}", rows.len().to_string()),
        // **Échappée comme tout texte venu de l'officine** : l'unité
        // s'écrit à la main au registre, et « ampoule [1 mL] » fermait le
        // bloc du modèle — le registre d'inspection ne s'imprimait plus.
        (
            "{{UNIT}}",
            format!(
                "#{}",
                typst_str(if unit.is_empty() { "unités" } else { unit })
            ),
        ),
    ]
}

const MARKERS_REGISTRE: &[&str] = &[
    "{{PRODUCT}}",
    "{{PHARMACY_NAME}}",
    "{{DATE}}",
    "{{ROWS}}",
    "{{COUNT}}",
    "{{UNIT}}",
];

const DEFAULT_REGISTRE_TEMPLATE: &str = r##"
#set page(paper: "a4", flipped: true, margin: 1.2cm)
#set text(size: 9pt, lang: "fr", hyphenate: true)

#align(center)[#text(15pt, weight: "bold")[Registre des stupéfiants — {{PRODUCT}}]]
#v(1mm)
#align(center)[#text(9pt)[{{PHARMACY_NAME}} — édité le {{DATE}}]]
#v(4mm)

#table(columns: (auto, auto, auto, auto, auto, auto, auto, auto, auto, 1fr), inset: 4pt, stroke: 0.5pt,
  [*Date*], [*N°*], [*Nature*], [*Entrée*], [*Sortie*], [*Solde*], [*À détruire*], [*Périmés*], [*Dossier*], [*Mention*],
{{ROWS}})

#v(4mm)
#text(8pt, style: "italic")[{{COUNT}} ligne(s) au registre, comptées en {{UNIT}}. Aucune ligne n'est raturée : une ligne annulée reste imprimée, barrée, et la ligne d'annulation qui la désigne en inverse l'effet sur le stock. Un inventaire *fixe* le solde au lieu de s'y ajouter : lorsqu'un comptage a trouvé un écart, les colonnes ne s'additionnent plus au solde final. Deux soldes : les retours de patients sont justifiés ici, ne sont jamais redélivrés et restent au compte « à détruire » jusqu'au procès-verbal. Le nom du patient figure dans le dossier dont le numéro est indiqué ci-dessus.]
"##;

pub fn open_ordonnancier(
    rows: &[crate::db::StupMove],
    labels: &std::collections::HashMap<i64, String>,
    cancelled: &std::collections::HashSet<i64>,
    year: i64,
    pharmacy: &PharmacyConfig,
    today: &str,
    template_path: &std::path::Path,
) -> Result<PathBuf, String> {
    compile_and_open(
        fill(
            &template_source("ordonnancier", template_path),
            &ordonnancier_values(rows, labels, cancelled, year, pharmacy, today),
        ),
        &format!("ordonnancier_{year}"),
    )
}

fn ordonnancier_values(
    rows: &[crate::db::StupMove],
    labels: &std::collections::HashMap<i64, String>,
    cancelled: &std::collections::HashSet<i64>,
    year: i64,
    pharmacy: &PharmacyConfig,
    today: &str,
) -> Vec<(&'static str, String)> {
    let mut body = String::new();
    for m in rows {
        let struck = cancelled.contains(&m.id);
        let cell = |s: &str| {
            if struck {
                format!("[#strike[#{}]]", typst_str(s))
            } else {
                format!("[#{}]", typst_str(s))
            }
        };
        body.push_str(&format!(
            "[*#{}*], {}, {}, {}, {}, {}, {}, [#{}],\n",
            typst_str(&crate::ordonnancier::number_label(m.ordo_no as u32)),
            cell(&crate::db::format_french_date(&m.happened_on)),
            cell(labels.get(&m.stup_id).map_or("—", String::as_str)),
            cell(&crate::codex::format_quantity(m.quantity)),
            cell(&format!("dossier {}", m.patient_id)),
            cell(&m.prescriber),
            cell(&m.operator),
            typst_str(if struck { "annulée" } else { "" }),
        ));
    }
    if body.is_empty() {
        body.push_str("[], [], [], [], [], [], [], [],\n");
    }
    vec![
        ("{{YEAR}}", format!("#{}", typst_str(&year.to_string()))),
        (
            "{{PHARMACY_NAME}}",
            format!("#{}", typst_str(&pharmacy.name)),
        ),
        (
            "{{DATE}}",
            format!("#{}", typst_str(&crate::db::format_french_date(today))),
        ),
        ("{{ROWS}}", body),
        ("{{COUNT}}", rows.len().to_string()),
    ]
}

const MARKERS_ORDONNANCIER: &[&str] = &[
    "{{YEAR}}",
    "{{PHARMACY_NAME}}",
    "{{DATE}}",
    "{{ROWS}}",
    "{{COUNT}}",
];

const DEFAULT_ORDONNANCIER_TEMPLATE: &str = r##"
#set page(paper: "a4", flipped: true, margin: 1.2cm)
#set text(size: 9pt, lang: "fr", hyphenate: true)

#align(center)[#text(15pt, weight: "bold")[Ordonnancier des stupéfiants — {{YEAR}}]]
#v(1mm)
#align(center)[#text(9pt)[{{PHARMACY_NAME}} — édité le {{DATE}}]]
#v(4mm)

#table(columns: (auto, auto, 1.4fr, auto, auto, 1fr, auto, auto), inset: 4pt, stroke: 0.5pt,
  [*N°*], [*Date*], [*Produit*], [*Quantité*], [*Dossier*], [*Prescripteur*], [*Par*], [*État*],
{{ROWS}})

#v(4mm)
#text(8pt, style: "italic")[{{COUNT}} délivrance(s) inscrite(s) pour l'année. Un numéro n'est jamais réattribué : une ligne annulée conserve le sien. Le nom du patient figure dans le dossier dont le numéro est indiqué ci-dessus.]
"##;

pub struct ConciliationData<'a> {
    pub patient: &'a Patient,
    pub today: &'a str,
    /// The prescriber the file names, so the sheet says who it is for.
    pub physician: &'a str,
    /// One row per treatment: (statut, produit, au dossier, à la sortie,
    /// remarque). Already ordered — loudest first, as on screen.
    pub rows: Vec<(String, String, String, String, String)>,
    /// The one-line count under the title.
    pub summary: &'a str,
    /// The officine's own mention, empty unless it wrote one.
    pub mention: &'a str,
    pub signature: &'a str,
}

/// The conciliation as a sheet for the prescriber.
///
/// It carries the reconductions too, and not only the divergences: a
/// sheet that lists five changes says nothing about the twelve lines it
/// did not look at, and the prescriber has no way of telling the two
/// apart. The blank box at the foot is the point of sending it — the
/// answer comes back on the same sheet.
pub fn open_conciliation(
    data: &ConciliationData,
    pharmacy: &PharmacyConfig,
    template_path: &std::path::Path,
) -> Result<PathBuf, String> {
    compile_and_open(
        fill(
            &template_source("conciliation", template_path),
            &conciliation_values(data, pharmacy),
        ),
        &format!("conciliation_{}", data.patient.id),
    )
}

const MARKERS_CONCILIATION: &[&str] = &[
    "{{PATIENT_NAME}}",
    "{{BIRTH_DATE}}",
    "{{DATE}}",
    "{{PHYSICIAN}}",
    "{{SUMMARY}}",
    "{{ROWS}}",
    "{{PHARMACY_NAME}}",
    "{{PHARMACY_PHONE}}",
    "{{SIGNATURE}}",
    "{{MENTION}}",
];

const DEFAULT_CONCILIATION_TEMPLATE: &str = r##"
#set page(paper: "a4", margin: 1.5cm)
#set text(size: 10pt, lang: "fr", hyphenate: true)

#align(center)[#text(15pt, weight: "bold")[Conciliation médicamenteuse]]
#v(1mm)
#align(center)[#text(10pt)[{{PATIENT_NAME}} — né(e) le {{BIRTH_DATE}} — le {{DATE}}]]
{{PHYSICIAN}}
#v(2mm)
#align(center)[#text(9.5pt, style: "italic")[{{SUMMARY}}]]
#v(3mm)

#table(columns: (auto, auto, 1fr, 1fr, 1.1fr), inset: 5pt, stroke: 0.5pt,
  [*Statut*], [*Traitement*], [*Au dossier*], [*Sur l'ordonnance de sortie*], [*Remarque*],
{{ROWS}})

#v(4mm)
#text(10pt, weight: "bold")[Avis du prescripteur]
#v(1.5mm)
#box(width: 100%, height: 3.5cm, stroke: 0.7pt)
#v(4mm)
#text(9.5pt)[{{PHARMACY_NAME}} — {{PHARMACY_PHONE}}]
{{SIGNATURE}}
{{MENTION}}
"##;

fn conciliation_values(
    data: &ConciliationData,
    pharmacy: &PharmacyConfig,
) -> Vec<(&'static str, String)> {
    let mut rows = String::new();
    for (status, name, before, after, note) in &data.rows {
        rows.push_str(&format!(
            "  [#text(8pt, weight: \"bold\")[#{}]], [*#{}*], {}, {}, [#text(8.5pt, style: \"italic\")[#{}]],\n",
            typst_str(status),
            typst_str(name),
            typst_str(before),
            typst_str(after),
            typst_str(note)
        ));
    }
    if rows.is_empty() {
        rows.push_str("  [], [], [], [], [],\n");
    }
    let physician = if data.physician.trim().is_empty() {
        String::new()
    } else {
        format!(
            "#align(center)[#text(10pt)[À l'attention du #{}]]",
            typst_str(data.physician.trim())
        )
    };
    let signature = if data.signature.trim().is_empty() {
        String::new()
    } else {
        format!(
            "\\\n#text(9.5pt)[Rapprochement établi par #{}]",
            typst_str(data.signature.trim())
        )
    };
    let mention = if data.mention.trim().is_empty() {
        String::new()
    } else {
        format!(
            "#v(3mm)\n#text(8pt, style: \"italic\")[#{}]",
            typst_str(data.mention.trim())
        )
    };
    vec![
        (
            "{{PATIENT_NAME}}",
            format!("#{}", typst_str(&data.patient.full_name())),
        ),
        (
            "{{BIRTH_DATE}}",
            format!(
                "#{}",
                typst_str(&crate::db::format_french_date(&data.patient.birth_date))
            ),
        ),
        ("{{DATE}}", format!("#{}", typst_str(data.today))),
        ("{{PHYSICIAN}}", physician),
        ("{{SUMMARY}}", format!("#{}", typst_str(data.summary))),
        ("{{ROWS}}", rows),
        (
            "{{PHARMACY_NAME}}",
            format!("#{}", typst_str(&pharmacy.name)),
        ),
        (
            "{{PHARMACY_PHONE}}",
            format!("#{}", typst_str(&pharmacy.phone)),
        ),
        ("{{SIGNATURE}}", signature),
        ("{{MENTION}}", mention),
    ]
}

/// The fiche de fabrication of a preparation: the formula at the
/// quantity actually being made, then the blanks the bonnes pratiques
/// ask to fill in — lot numbers, operator, date, control.
///
/// `lines` is (ingredient, what the formula says, what to weigh today).
pub fn open_preparation(
    prep: &crate::db::Preparation,
    target: &str,
    lines: &[(String, String, String)],
    pharmacy: &PharmacyConfig,
    operator: &str,
    template_path: &std::path::Path,
) -> Result<PathBuf, String> {
    compile_and_open(
        fill(
            &template_source("preparation", template_path),
            &preparation_values(prep, target, lines, pharmacy, operator),
        ),
        &format!("preparation_{}", prep.id),
    )
}

const MARKERS_PREPARATION: &[&str] = &[
    "{{PHARMACY_NAME}}",
    "{{NAME}}",
    "{{FORM}}",
    "{{TARGET}}",
    "{{OPERATOR}}",
    "{{ROWS}}",
    "{{SECTIONS}}",
    "{{SOURCES}}",
];

const DEFAULT_PREPARATION_TEMPLATE: &str = r##"
#set page(paper: "a4", margin: 1.8cm)
#set text(size: 10.5pt, lang: "fr", hyphenate: true)

#grid(columns: (1fr, auto), [#text(weight: "bold")[{{PHARMACY_NAME}}]], [#align(right)[#text(9pt)[Fiche de fabrication]]])
#v(2mm)
#line(length: 100%, stroke: 0.8pt)
#v(3mm)
#align(center)[#text(15pt, weight: "bold")[{{NAME}}]]
{{FORM}}
#v(4mm)
#text(weight: "bold")[Quantité préparée :] {{TARGET}} #h(1fr) #text(weight: "bold")[Date :] #box(width: 3cm, stroke: (bottom: 0.6pt))[] #h(6mm) #text(weight: "bold")[Par :] #box(width: 2.5cm, stroke: (bottom: 0.6pt))[{{OPERATOR}}]
#v(4mm)

#table(columns: (1fr, auto, auto, 3.4cm), inset: 6pt, stroke: 0.6pt,
  [*Matière première*], [*Formule*], [*À peser*], [*N° de lot*],
{{ROWS}})

{{SECTIONS}}

#v(5mm)
#line(length: 100%, stroke: 0.4pt)
#v(2mm)
#grid(columns: (1fr, 1fr), gutter: 8mm,
  [#text(9.5pt, weight: "bold")[Contrôle] #v(1mm) #box(width: 100%, height: 2cm, stroke: 0.6pt)],
  [#text(9.5pt, weight: "bold")[Étiquetage et remise] #v(1mm) #box(width: 100%, height: 2cm, stroke: 0.6pt)])
{{SOURCES}}
"##;

fn preparation_values(
    prep: &crate::db::Preparation,
    target: &str,
    lines: &[(String, String, String)],
    pharmacy: &PharmacyConfig,
    operator: &str,
) -> Vec<(&'static str, String)> {
    // La colonne du lot reste vide : c'est elle qui fait la fiche.
    let mut rows = String::new();
    for (name, written, weighed) in lines {
        rows.push_str(&format!(
            "  {}, {}, {}, [],\n",
            typst_str(name),
            typst_str(written),
            typst_str(weighed)
        ));
    }
    if rows.is_empty() {
        rows.push_str("  [], [], [], [],\n");
    }
    let mut sections = String::new();
    for (title, body) in [
        ("Mode opératoire", prep.method.as_str()),
        ("Conservation", prep.conservation.as_str()),
        ("Mise en garde", prep.caution.as_str()),
        ("Indication", prep.indication.as_str()),
    ] {
        if body.trim().is_empty() {
            continue;
        }
        sections.push_str(&format!(
            "#v(3mm)\n#text(weight: \"bold\", size: 10pt)[#{}]\n#v(1mm)\n#text(9.5pt)[#{}]\n",
            typst_str(title),
            typst_str(body.trim())
        ));
    }
    let sources: Vec<&str> = prep
        .sources
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    let sources = if sources.is_empty() {
        String::new()
    } else {
        format!(
            "#v(3mm)\n#text(size: 8pt)[Sources : #{}]",
            typst_str(&sources.join(" · "))
        )
    };
    let form = if prep.form.trim().is_empty() {
        String::new()
    } else {
        format!(
            "#align(center)[#text(10pt, style: \"italic\")[#{}]]",
            typst_str(prep.form.trim())
        )
    };
    vec![
        (
            "{{PHARMACY_NAME}}",
            format!("#{}", typst_str(&pharmacy.name)),
        ),
        ("{{NAME}}", format!("#{}", typst_str(&prep.name))),
        ("{{FORM}}", form),
        ("{{TARGET}}", format!("#{}", typst_str(target))),
        ("{{OPERATOR}}", format!("#{}", typst_str(operator))),
        ("{{ROWS}}", rows),
        ("{{SECTIONS}}", sections),
        ("{{SOURCES}}", sources),
    ]
}

/// One substitution protocol as a printable A4 page: the decision tree
/// as an indented list, questions in bold, branches labelled.
pub fn open_protocol(
    title: &str,
    subject: &str,
    nodes: &[crate::db::ProtocolNode],
    template_path: &std::path::Path,
) -> Result<PathBuf, String> {
    compile_and_open(
        fill(
            &template_source("protocole", template_path),
            &protocol_values(title, subject, nodes),
        ),
        "protocole",
    )
}

const MARKERS_PROTOCOLE: &[&str] = &["{{TITLE}}", "{{SUBJECT}}", "{{TREE}}"];

/// Une liste de contrôle sur une page, **avec ses cases**.
///
/// C'est la case qui fait la différence avec un protocole : on ne
/// descend pas une liste, on la coche, et une feuille sans case se
/// coche au stylo dans la marge. La date et la personne sont en tête,
/// vides : une liste cochée sans savoir quand ni par qui ne prouve
/// rien, et les pré-remplir serait remplir à la place de quelqu'un.
pub fn open_checklist(
    title: &str,
    subject: &str,
    items: &[crate::db::ChecklistItem],
    template_path: &std::path::Path,
) -> Result<PathBuf, String> {
    compile_and_open(
        fill(
            &template_source("liste", template_path),
            &checklist_values(title, subject, items),
        ),
        "liste",
    )
}

const MARKERS_LISTE: &[&str] = &["{{TITLE}}", "{{SUBJECT}}", "{{ITEMS}}"];

const DEFAULT_LISTE_TEMPLATE: &str = r##"
#set page(paper: "a4", margin: 2cm)
#set text(size: 11pt, lang: "fr", hyphenate: true)

#align(center)[#text(15pt, weight: "bold")[{{TITLE}}]]
#align(center)[{{SUBJECT}}]
#v(3mm)
#grid(columns: (1fr, 1fr), gutter: 6mm,
  [Date : #box(width: 1fr, repeat[.])],
  [Par : #box(width: 1fr, repeat[.])],
)
#v(2mm)
#line(length: 100%, stroke: 0.6pt)
#v(3mm)
{{ITEMS}}
"##;

fn checklist_values(
    title: &str,
    subject: &str,
    items: &[crate::db::ChecklistItem],
) -> Vec<(&'static str, String)> {
    let mut body = String::new();
    for item in items {
        // Une case dessinée, et non un caractère : la police d'une
        // feuille imprimée n'est pas celle de l'écran, et un carré
        // typographique manquant sort en blanc — une liste sans cases.
        body.push_str("#grid(columns: (6mm, 1fr), gutter: 0mm, align: (left + top, left + top),\n");
        body.push_str("  [#box(width: 3.6mm, height: 3.6mm, stroke: 0.6pt)],\n  [");
        body.push_str(&format!("#{}", typst_str(item.text.trim())));
        if !item.note.trim().is_empty() {
            body.push_str(&format!(
                "\\\n#text(size: 9pt, fill: rgb(90, 90, 90))[#{}]",
                typst_str(item.note.trim())
            ));
        }
        body.push_str("],\n)\n#v(2.4mm)\n");
    }
    if items.is_empty() {
        body.push_str("#text(fill: rgb(120, 120, 120))[Liste vide.]\n");
    }
    vec![
        ("{{TITLE}}", format!("#{}", typst_str(title.trim()))),
        (
            "{{SUBJECT}}",
            if subject.trim().is_empty() {
                String::new()
            } else {
                format!("#text(size: 10pt)[#{}]", typst_str(subject.trim()))
            },
        ),
        ("{{ITEMS}}", body),
    ]
}

const DEFAULT_PROTOCOLE_TEMPLATE: &str = r##"
#set page(paper: "a4", margin: 2cm)
#set text(size: 11pt, lang: "fr", hyphenate: true)

#align(center)[#text(15pt, weight: "bold")[{{TITLE}}]]
{{SUBJECT}}
#v(3mm)
#line(length: 100%, stroke: 0.6pt)
#v(2mm)
{{TREE}}
"##;

fn protocol_values(
    title: &str,
    subject: &str,
    nodes: &[crate::db::ProtocolNode],
) -> Vec<(&'static str, String)> {
    // Depth-first, "yes" branch before "no", same order as on screen.
    let mut tree = String::new();
    let mut stack: Vec<(&crate::db::ProtocolNode, usize)> = nodes
        .iter()
        .filter(|n| n.parent_id.is_none())
        .rev()
        .map(|n| (n, 0))
        .collect();
    while let Some((node, depth)) = stack.pop() {
        let tag = match node.branch {
            crate::db::Branch::Yes => "Oui — ",
            crate::db::Branch::No => "Non — ",
            crate::db::Branch::Root => "",
        };
        let body = if node.kind == crate::db::NodeKind::Question {
            format!(
                "#text(weight: \"bold\")[#{} ?]",
                typst_str(&format!("{tag}{}", node.text.trim_end_matches('?').trim()))
            )
        } else {
            format!("#{}", typst_str(&format!("{tag}{}", node.text.trim())))
        };
        tree.push_str(&format!(
            "#pad(left: {}mm)[{}]\n#v(1.2mm)\n",
            depth * 7,
            body
        ));
        let mut children: Vec<&crate::db::ProtocolNode> = nodes
            .iter()
            .filter(|n| n.parent_id == Some(node.id))
            .collect();
        children.sort_by_key(|n| (n.branch != crate::db::Branch::Yes, n.position));
        for child in children.into_iter().rev() {
            stack.push((child, depth + 1));
        }
    }
    let subject = if subject.trim().is_empty() {
        String::new()
    } else {
        format!(
            "#align(center)[#text(10pt, style: \"italic\")[#{}]]",
            typst_str(subject.trim())
        )
    };
    vec![
        ("{{TITLE}}", format!("#{}", typst_str(title))),
        ("{{SUBJECT}}", subject),
        ("{{TREE}}", tree),
    ]
}

/// The week on one landscape A4 page: a column per day, rendez-vous
/// and other entries in the order of the day, hours first.
pub fn open_week_plan(
    week: &[String],
    appointments: &[Appointment],
    events: &[crate::db::Event],
    today: &str,
    template_path: &std::path::Path,
) -> Result<PathBuf, String> {
    if week.is_empty() {
        return Err("semaine vide".to_owned());
    }
    compile_and_open(
        fill(
            &template_source("semaine", template_path),
            &week_plan_values(week, appointments, events, today),
        ),
        "semaine",
    )
}

const MARKERS_SEMAINE: &[&str] = &["{{MONDAY}}", "{{CELLS}}"];

const DEFAULT_SEMAINE_TEMPLATE: &str = r##"
#set page(paper: "a4", flipped: true, margin: 1.2cm)
#set text(size: 9pt, lang: "fr", hyphenate: true)

#align(center)[#text(14pt, weight: "bold")[Semaine du {{MONDAY}}]]
#v(3mm)

// Des colonnes pleine hauteur : la feuille se punaise et s'annote
// pendant la semaine, elle ne se lit pas seulement.
#table(columns: (1fr, 1fr, 1fr, 1fr, 1fr, 1fr, 1fr), rows: 16cm, inset: 4pt, stroke: 0.5pt, align: top,
{{CELLS}})
"##;

fn week_plan_values(
    week: &[String],
    appointments: &[Appointment],
    events: &[crate::db::Event],
    today: &str,
) -> Vec<(&'static str, String)> {
    let monday = week.first().map(String::as_str).unwrap_or("");
    let mut cells = String::new();
    for day in week {
        let name = crate::db::weekday_fr(day).unwrap_or("");
        let head = format!(
            "{}{} {}",
            name.chars()
                .next()
                .map(|c| c.to_uppercase().to_string())
                .unwrap_or_default(),
            name.chars().skip(1).collect::<String>(),
            crate::db::format_french_date(day)
        );
        let mark = if day == today { " (aujourd'hui)" } else { "" };
        let mut body = String::new();
        for rdv in appointments.iter().filter(|r| r.date == *day) {
            let hour = if rdv.time.is_empty() {
                String::new()
            } else {
                format!("{} ", rdv.time)
            };
            body.push_str(&format!(
                "#text(size: 8pt)[#{}]#linebreak()\n",
                typst_str(&format!(
                    "{hour}{} — {}",
                    rdv.patient_name,
                    rdv.kind.label()
                ))
            ));
        }
        for ev in events.iter().filter(|e| e.day == *day) {
            // An entry that runs to an hour prints both, so the plan on
            // the wall says how much of the day it takes.
            let hour = match (ev.time.as_str(), ev.end_time.as_str()) {
                ("", _) => String::new(),
                (t, "") => format!("{t} "),
                (t, e) => format!("{t}–{e} "),
            };
            body.push_str(&format!(
                "#text(size: 8pt, style: \"italic\")[#{}]#linebreak()\n",
                typst_str(&format!("{hour}{} ({})", ev.title, ev.category.label()))
            ));
        }
        if body.is_empty() {
            body.push_str("#text(size: 8pt, fill: gray)[—]\n");
        }
        cells.push_str(&format!(
            "  [#text(weight: \"bold\", size: 8.5pt)[#{}]#linebreak()#v(1mm){}],\n",
            typst_str(&format!("{head}{mark}")),
            body
        ));
    }
    vec![
        (
            "{{MONDAY}}",
            format!("#{}", typst_str(&crate::db::format_french_date(monday))),
        ),
        ("{{CELLS}}", cells),
    ]
}

/// Compile and open the conversion tables as a printable A4 reference.
pub fn open_conversion_tables(
    edits: &TableEdits,
    template_path: &std::path::Path,
) -> Result<PathBuf, String> {
    compile_and_open(
        fill(
            &template_source("tables", template_path),
            &conversion_tables_values(edits),
        ),
        "tables",
    )
}

/// The whole codex as a booklet: one block per preparation, in the
/// order the list shows them. What goes in the préparatoire's binder.
pub fn open_codex(
    preparations: &[crate::db::Preparation],
    template_path: &std::path::Path,
) -> Result<PathBuf, String> {
    compile_and_open(
        fill(
            &template_source("codex", template_path),
            &codex_values(preparations),
        ),
        "codex",
    )
}

fn codex_values(preparations: &[crate::db::Preparation]) -> Vec<(&'static str, String)> {
    let mut src = String::new();
    for prep in preparations {
        src.push_str(&format!(
            "#block(breakable: false, below: 5mm)[\n#text(11pt, weight: \"bold\")[#{}]",
            typst_str(&prep.name)
        ));
        if !prep.form.trim().is_empty() {
            src.push_str(&format!(
                " #text(9pt, style: \"italic\")[ — #{}]",
                typst_str(prep.form.trim())
            ));
        }
        src.push_str("\n#v(1mm)\n#line(length: 100%, stroke: 0.5pt)\n#v(1.5mm)\n");
        // The formula as written, then what it yields: the sheet is
        // read at the bench, where the quantity is recomputed anyway.
        let mut rows = String::new();
        for line in crate::codex::parse_formula(&prep.formula) {
            rows.push_str(&format!(
                "{}, {},\n",
                typst_str(&line.name),
                typst_str(&line.written)
            ));
        }
        if !rows.is_empty() {
            src.push_str(&format!(
                "#table(columns: (1fr, auto), inset: 4pt, stroke: 0.4pt,\n{rows})\n"
            ));
        }
        if !prep.yield_amount.trim().is_empty() {
            src.push_str(&format!(
                "#text(8.5pt)[Pour #{}]\n",
                typst_str(prep.yield_amount.trim())
            ));
        }
        for (title, body) in [
            ("Indication", prep.indication.as_str()),
            ("Mode opératoire", prep.method.as_str()),
            ("Conservation", prep.conservation.as_str()),
            ("Mise en garde", prep.caution.as_str()),
        ] {
            if body.trim().is_empty() {
                continue;
            }
            src.push_str(&format!(
                "#v(1.2mm)\n#text(8.5pt)[#text(weight: \"bold\")[#{} : ]#{}]\n",
                typst_str(title),
                typst_str(body.trim())
            ));
        }
        let sources: Vec<&str> = prep
            .sources
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .collect();
        if !sources.is_empty() {
            src.push_str(&format!(
                "#v(1.2mm)\n#text(8pt, style: \"italic\")[Sources : #{}]\n",
                typst_str(&sources.join(" · "))
            ));
        }
        src.push_str("]\n");
    }
    vec![("{{BODY}}", src)]
}

const MARKERS_CODEX: &[&str] = &["{{BODY}}"];

const DEFAULT_CODEX_TEMPLATE: &str = r##"
#set page(paper: "a4", margin: 1.6cm)
#set text(size: 9.5pt, lang: "fr", hyphenate: true)
#set par(justify: true)
#align(center)[#text(15pt, weight: "bold")[Codex des préparations]]
#v(1mm)
#align(center)[#text(9pt, style: "italic")[Préparation sur ordonnance uniquement, selon les bonnes pratiques de préparation.]]
#v(4mm)

{{BODY}}
"##;

/// One dispositif as a printable A4 sheet — the one that goes in the
/// drawer beside the box, or in the patient's hand at the counter.
pub fn open_dispositif(
    dispo: &crate::db::Dispositif,
    pharmacy: &PharmacyConfig,
    template_path: &std::path::Path,
) -> Result<PathBuf, String> {
    compile_and_open(
        fill(
            &template_source("dispositif", template_path),
            &dispositif_values(dispo, pharmacy),
        ),
        "dispositif",
    )
}

fn dispositif_values(
    dispo: &crate::db::Dispositif,
    pharmacy: &PharmacyConfig,
) -> Vec<(&'static str, String)> {
    let mut src = String::new();
    if !pharmacy.name.trim().is_empty() {
        src.push_str(&format!(
            "#align(right)[#text(8.5pt, style: \"italic\")[#{}]]\n",
            typst_str(pharmacy.name.trim())
        ));
    }
    src.push_str(&format!(
        "#text(16pt, weight: \"bold\")[#{}]\n",
        typst_str(&dispo.name)
    ));
    if !dispo.family.trim().is_empty() {
        src.push_str(&format!(
            "#v(1mm)\n#text(10pt, style: \"italic\")[#{}]\n",
            typst_str(dispo.family.trim())
        ));
    }
    for (title, body) in dispositif_sections(dispo) {
        src.push_str(&format!(
            "#sec[#{}]\n#text(10pt)[#{}]\n",
            typst_str(title),
            typst_str(body.trim())
        ));
    }
    let sources: Vec<&str> = dispo
        .sources
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    if !sources.is_empty() {
        src.push_str(&format!(
            "#v(3mm)\n#text(8.5pt, style: \"italic\")[Sources : #{}]\n",
            typst_str(&sources.join(" · "))
        ));
    }
    src.push_str(
        "#v(2mm)\n#text(8pt, style: \"italic\")[Ligne LPP et tarif à vérifier lors de la délivrance : cette fiche indique les conditions de prise en charge, non le prix.]\n",
    );
    vec![("{{BODY}}", src)]
}

const MARKERS_DISPOSITIF: &[&str] = &["{{BODY}}"];

const DEFAULT_DISPOSITIF_TEMPLATE: &str = r##"
#set page(paper: "a4", margin: 1.8cm)
#set text(size: 10pt, lang: "fr", hyphenate: true)
#set par(justify: true)

#let sec(t) = [#v(3mm) #text(10.5pt, weight: "bold")[#t] #v(1mm) #line(length: 100%, stroke: 0.5pt) #v(1.5mm)]

{{BODY}}
"##;

/// The sections of a dispositif fiche, in the order of the gesture —
/// shared by the single sheet and the whole booklet so the two can
/// never drift apart.
fn dispositif_sections(dispo: &crate::db::Dispositif) -> Vec<(&'static str, &str)> {
    [
        ("Indication", dispo.indication.as_str()),
        ("Formes et tailles", dispo.sizes.as_str()),
        ("Pose", dispo.application.as_str()),
        ("Renouvellement", dispo.renewal.as_str()),
        ("Prise en charge (LPP)", dispo.lpp.as_str()),
        ("Incidents fréquents", dispo.caution.as_str()),
    ]
    .into_iter()
    .filter(|(_, body)| !body.trim().is_empty())
    .collect()
}

/// The whole set of dispositifs as one booklet, in two columns, grouped
/// by family: what the team pins near the stock.
pub fn open_dispositifs(
    dispositifs: &[crate::db::Dispositif],
    pharmacy: &PharmacyConfig,
    template_path: &std::path::Path,
) -> Result<PathBuf, String> {
    compile_and_open(
        fill(
            &template_source("dispositifs", template_path),
            &dispositifs_values(dispositifs, pharmacy),
        ),
        "dispositifs",
    )
}

fn dispositifs_values(
    dispositifs: &[crate::db::Dispositif],
    pharmacy: &PharmacyConfig,
) -> Vec<(&'static str, String)> {
    let mut src = String::new();
    let mut family = String::new();
    for dispo in dispositifs {
        if dispo.family != family {
            family = dispo.family.clone();
            if !family.trim().is_empty() {
                src.push_str(&format!(
                    "#v(2mm)\n#text(11pt, weight: \"bold\")[#{}]\n#v(1mm)\n",
                    typst_str(family.trim().to_uppercase().as_str())
                ));
            }
        }
        src.push_str(&format!(
            "#block(breakable: false, below: 3.5mm)[\n#text(9.5pt, weight: \"bold\")[#{}]\n",
            typst_str(&dispo.name)
        ));
        for (title, body) in dispositif_sections(dispo) {
            src.push_str(&format!(
                "#v(0.8mm)\n#text(8pt)[#text(weight: \"bold\")[#{} : ]#{}]\n",
                typst_str(title),
                typst_str(body.trim())
            ));
        }
        src.push_str("]\n");
    }
    vec![
        (
            "{{PHARMACY_NAME}}",
            format!(
                "#{}",
                typst_str(if pharmacy.name.trim().is_empty() {
                    "La ligne LPP et son tarif se vérifient au moment de la délivrance."
                } else {
                    pharmacy.name.trim()
                })
            ),
        ),
        ("{{BODY}}", src),
    ]
}

const MARKERS_DISPOSITIFS: &[&str] = &["{{PHARMACY_NAME}}", "{{BODY}}"];

const DEFAULT_DISPOSITIFS_TEMPLATE: &str = r##"
#set page(paper: "a4", margin: 1.5cm, columns: 2)
#set text(size: 8.5pt, lang: "fr", hyphenate: true)
#set par(justify: true)

#place(top + center, scope: "parent", float: true)[
  #text(15pt, weight: "bold")[Dispositifs médicaux]
  #v(1mm)
  #text(8.5pt, style: "italic")[{{PHARMACY_NAME}}]
  #v(3mm)
]

{{BODY}}
"##;

/// One day of the transmission logbook as a printable A4 page.
pub fn open_transmission_day(
    day_title: &str,
    entries: &[crate::db::Note],
    template_path: &std::path::Path,
) -> Result<PathBuf, String> {
    let template = if template_path.exists() {
        std::fs::read_to_string(template_path)
            .map_err(|e| format!("modèle {} illisible : {e}", template_path.display()))?
    } else {
        DEFAULT_TRANS_TEMPLATE.to_owned()
    };
    compile_and_open(fill_trans_template(&template, day_title, entries), "carnet")
}

fn trans_entries_markup(entries: &[crate::db::Note]) -> String {
    let mut out = String::new();
    for n in entries {
        let head = if n.operator.is_empty() {
            n.stamp()
        } else {
            format!("{} · {}", n.stamp(), n.operator)
        };
        out.push_str(&format!(
            "#entry({}, {}, [#{}])\n",
            typst_str(&head),
            typst_str(n.operator.trim()),
            typst_str(&n.body)
        ));
    }
    if out.is_empty() {
        out.push_str("#text(style: \"italic\")[Aucune transmission ce jour.]\n");
    }
    out
}

fn fill_trans_template(template: &str, day_title: &str, entries: &[crate::db::Note]) -> String {
    template
        .replace("{{DAY}}", &format!("#{}", typst_str(day_title)))
        .replace("{{ENTRIES}}", &trans_entries_markup(entries))
}

fn sample_transmissions() -> Vec<crate::db::Note> {
    vec![
        crate::db::Note {
            id: 1,
            operator: "CL".to_owned(),
            body: "Rupture Eliquis 5 mg — dépannage possible pharmacie Centrale.".to_owned(),
            created_at: "2026-08-24 18:40:00".to_owned(),
        },
        crate::db::Note {
            id: 2,
            operator: "YS".to_owned(),
            body: "M. Dupont rappellera demain pour son BPM.".to_owned(),
            created_at: "2026-08-24 19:05:00".to_owned(),
        },
    ]
}

/// Compile Typst source to a PDF in the temp dir and open it in the OS
/// viewer. The file name is unique per generation: the previous PDF may
/// still be open in the viewer (Windows locks it, and reusing the name
/// would fail).
fn compile_and_open(source: String, stem: &str) -> Result<PathBuf, String> {
    let world = PdfWorld::new(source);
    let document: PagedDocument = typst::compile(&world)
        .output
        .map_err(|errs| format!("compilation Typst : {}", format_diagnostics(&errs)))?;
    let pdf = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
        .map_err(|errs| format!("export PDF : {}", format_diagnostics(&errs)))?;

    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let out = std::env::temp_dir().join(format!("bpm_caddy_{stem}_{stamp}.pdf"));
    std::fs::write(&out, pdf).map_err(|e| format!("écriture du PDF impossible : {e}"))?;
    open::that_detached(&out).map_err(|e| format!("ouverture du PDF impossible : {e}"))?;
    // Compté ici et nulle part ailleurs : les vingt et un `open_*`
    // passent tous par cette fonction, et compter chez chacun d'eux
    // serait vingt et une occasions d'en oublier un. Voir
    // `src/telemetry.rs` — le compteur ne sort pas du processus, et
    // c'est la session qui décide s'il est enregistré.
    crate::telemetry::tally(crate::telemetry::Signal::Printed);
    Ok(out)
}

/// Escape arbitrary text as a Typst string literal, so patient names
/// can never inject markup into the generated document.
fn typst_str(s: &str) -> String {
    // **Et la ponctuation double tient à son mot.** Tout ce qui va sur
    // le papier passe par ici : la prose des modèles comme les cellules
    // que l'officine écrit. Sans cela une ligne justifiée commence par
    // « » » ou par « : », ce que la typographie française ne fait pas —
    // trois documents le faisaient. Lier ici plutôt qu'à chaque appel,
    // parce qu'un seul appel oublié est une ligne qui recommence.
    let s = bind_french(s);
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

/// The points of a theme, as tick-boxes: the sheet in the pharmacist's
/// hand carries what the entretien is for. The box in its own column,
/// so a point that wraps continues under its text and not under the
/// box.
fn checklist_markup(points: &[&str]) -> String {
    points
        .iter()
        .map(|point| {
            format!(
                "#grid(columns: (auto, 1fr), column-gutter: 2mm, [#box(width: 3.4mm, height: 3.4mm, stroke: 0.7pt)], [#{}])",
                typst_str(point)
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Substitute the interview-sheet placeholders. Values are spliced as
/// Typst string literals (`#"…"`), so a patient name containing markup
/// ('#', '*', brackets…) can neither break compilation nor restyle the
/// sheet.
fn fill_interview_template(
    template: &str,
    paper: &InterviewPaper,
    pharmacy: &PharmacyConfig,
) -> String {
    fill(template, &interview_values(paper, pharmacy))
}

/// Les valeurs de la fiche d'entretien — celles de l'impression et
/// celles de l'aperçu, par la même fonction.
fn interview_values(
    paper: &InterviewPaper,
    pharmacy: &PharmacyConfig,
) -> Vec<(&'static str, String)> {
    let s = |v: &str| format!("#{}", typst_str(v));
    let kind = paper.kind;
    let birth = if paper.patient.birth_date.trim().is_empty() {
        "—".to_owned()
    } else {
        crate::db::format_french_date(&paper.patient.birth_date)
    };
    let age = paper.age.map_or_else(
        || crate::strings::tr("trod_pdf_no_age").to_owned(),
        |a| format!("{a} ans"),
    );
    vec![
        ("{{PHARMACY_NAME}}", s(&pharmacy.name)),
        ("{{PHARMACY_PHONE}}", s(&pharmacy.phone)),
        ("{{PATIENT_NAME}}", s(&paper.patient.full_name())),
        ("{{BIRTH_DATE}}", s(&birth)),
        ("{{AGE}}", s(&age)),
        ("{{KIND}}", s(kind.label())),
        ("{{DATE}}", s(paper.date)),
        (
            "{{THEME}}",
            // Un acte qui ne porte pas de thème n'en imprime pas, même
            // si la base en garde un : jusqu'à la 0.145 le thème armé
            // par le choix rapide était écrit sur les actes qui n'en ont
            // pas, et il ressortait ici. La source est corrigée ; ceci
            // couvre les lignes déjà écrites, sans réécrire la base.
            s(theme_or_dash(if kind.has_theme() {
                paper.theme
            } else {
                ""
            })),
        ),
        // Whoever held the entretien signs the sheet. A template
        // written before the team list simply has no such marker, and
        // loses nothing.
        ("{{PHARMACIST}}", s(paper.signature)),
        ("{{TREATMENTS}}", treatments_markup(paper.treats)),
        ("{{CHECKLIST}}", checklist_markup(paper.checklist)),
    ]
}

/// An empty thematic prints as a dash rather than a blank.
fn theme_or_dash(theme: &str) -> &str {
    if theme.trim().is_empty() {
        "—"
    } else {
        theme.trim()
    }
}

/// Build the printable list of upcoming appointments (date, patient,
/// kind, phone) — a paper companion for the counter.
const MARKERS_RDV: &[&str] = &["{{DATE}}", "{{ROWS}}"];

const DEFAULT_RDV_TEMPLATE: &str = r##"
#set page(paper: "a4", margin: 1.5cm)
#set text(size: 11pt)
#align(center)[#text(16pt, weight: "bold")[Rendez-vous à venir]]
#v(1mm)
#align(center)[Édité le {{DATE}}]
#v(5mm)
#table(
  columns: (auto, 1fr, auto, auto),
  inset: 7pt,
  stroke: 0.6pt,
  [*Date*], [*Patient*], [*Type*], [*Téléphone*],
{{ROWS}})
"##;

fn appointment_list_values(
    rdvs: &[Appointment],
    today_french: &str,
) -> Vec<(&'static str, String)> {
    let mut rows = String::new();
    for rdv in rdvs {
        rows.push_str(&format!(
            "  {}, {}, {}, {},\n",
            typst_str(&crate::db::format_french_date(&rdv.date)),
            typst_str(&rdv.patient_name),
            typst_str(rdv.kind.label()),
            typst_str(&rdv.phone),
        ));
    }
    if rows.is_empty() {
        rows.push_str("  [], [], [], [],\n");
    }
    vec![
        ("{{DATE}}", format!("#{}", typst_str(today_french))),
        ("{{ROWS}}", rows),
    ]
}

/// Render the prescribed lines as a numbered block.
fn ordonnance_lines_markup(lines: &[crate::ordonnance::Line]) -> String {
    if lines.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    for (i, line) in lines.iter().enumerate() {
        // A one-column grid with an explicit row gutter, not paragraph
        // linebreaks: `\` and `#linebreak()` both left the molecule and
        // its own posology a full paragraph apart, and `#pad` for the
        // indent did the same. A grid row is the only spacing here that
        // is stated rather than inherited.
        out.push_str("#block(above: 0mm, below: 3.5mm)[#grid(columns: (1fr), row-gutter: 1.4mm,\n");
        out.push_str(&format!(
            "  [#text(weight: \"bold\")[{}. #{}]],\n",
            i + 1,
            typst_str(&line.name)
        ));
        if !line.posology.is_empty() {
            out.push_str(&format!("  [#h(5mm)#{}],\n", typst_str(&line.posology)));
        }
        if !line.caution.is_empty() {
            out.push_str(&format!(
                "  [#h(5mm)#text(9pt, style: \"italic\")[#{}]],\n",
                typst_str(&line.caution)
            ));
        }
        out.push_str(")]\n");
    }
    out
}

/// Render the advice paragraphs, if any toggle is on.
fn ordonnance_advice_markup(advice: &[String]) -> String {
    if advice.is_empty() {
        return String::new();
    }
    let mut out = String::from(
        "#v(3mm)\n#line(length: 100%, stroke: 0.4pt)\n#v(2mm)\n         #text(weight: \"bold\", size: 10pt)[Conseils]\n#v(1.5mm)\n",
    );
    for item in advice {
        out.push_str(&format!(
            "#block(above: 0mm, below: 1.8mm)[#text(9.5pt)[— #{}]]\n",
            typst_str(item)
        ));
    }
    out
}

/// Les deux mentions de l'ordonnance, écrites **une fois** : par
/// l'impression et par l'aperçu de l'éditeur. L'aperçu les écrivait
/// nues et centrées nulle part, si bien que ce qu'on réglait dans
/// l'éditeur n'était pas ce qui s'imprimait. Une mention vide ne laisse
/// pas de ligne, pas une ligne italique vide.
fn mention_header(text: &str) -> String {
    if text.trim().is_empty() {
        String::new()
    } else {
        format!(
            "#align(center)[#text(9pt, style: \"italic\")[#{}]]",
            typst_str(text.trim())
        )
    }
}

/// Voir [`mention_header`].
fn mention_footer(text: &str) -> String {
    if text.trim().is_empty() {
        String::new()
    } else {
        format!(
            "#v(2mm)\n#text(8pt, style: \"italic\")[#{}]",
            typst_str(text.trim())
        )
    }
}

/// Substitute the ordonnance placeholders. Every value is spliced as a
/// Typst string literal, so a patient name or a hand-written posology
/// containing markup can neither break compilation nor restyle the page.
#[allow(clippy::too_many_arguments)]
fn fill_ordonnance_template(
    template: &str,
    patient: &Patient,
    pharmacy: &PharmacyConfig,
    indication: &str,
    today: &str,
    lines: &[crate::ordonnance::Line],
    advice: &[String],
    signature: &str,
    mentions: (&str, &str),
) -> String {
    fill(
        template,
        &[
            (
                "{{PHARMACY_NAME}}",
                format!("#{}", typst_str(&pharmacy.name)),
            ),
            (
                "{{PHARMACY_ADDRESS}}",
                format!("#{}", typst_str(&pharmacy.address)),
            ),
            (
                "{{PHARMACY_PHONE}}",
                format!("#{}", typst_str(&pharmacy.phone)),
            ),
            (
                "{{PHARMACY_AM}}",
                format!("#{}", typst_str(&pharmacy.am_number)),
            ),
            ("{{PHARMACIST}}", format!("#{}", typst_str(signature))),
            (
                "{{PATIENT_NAME}}",
                format!("#{}", typst_str(&patient.full_name())),
            ),
            (
                "{{BIRTH_DATE}}",
                format!(
                    "#{}",
                    typst_str(&crate::db::format_french_date(&patient.birth_date))
                ),
            ),
            ("{{INDICATION}}", format!("#{}", typst_str(indication))),
            ("{{DATE}}", format!("#{}", typst_str(today))),
            ("{{LINES}}", ordonnance_lines_markup(lines)),
            ("{{ADVICE}}", ordonnance_advice_markup(advice)),
            ("{{MENTION_HEADER}}", mention_header(mentions.0)),
            ("{{MENTION_FOOTER}}", mention_footer(mentions.1)),
        ],
    )
}

/// Typeset the ordonnance and hand it to the OS viewer.
#[allow(clippy::too_many_arguments)]
pub fn open_ordonnance(
    patient: &Patient,
    pharmacy: &PharmacyConfig,
    indication: &str,
    today: &str,
    lines: &[crate::ordonnance::Line],
    advice: &[String],
    template_path: &std::path::Path,
    signature: &str,
    mentions: (&str, &str),
) -> Result<PathBuf, String> {
    if lines.is_empty() {
        return Err("Rien à prescrire : choisissez au moins une ligne.".to_owned());
    }
    let template = if template_path.exists() {
        std::fs::read_to_string(template_path)
            .map_err(|e| format!("modèle {} illisible : {e}", template_path.display()))?
    } else {
        DEFAULT_ORDONNANCE_TEMPLATE.to_owned()
    };
    let filled = fill_ordonnance_template(
        &template, patient, pharmacy, indication, today, lines, advice, signature, mentions,
    );
    compile_and_open(filled, &format!("ordonnance_{}", patient.id))
}

/// Fill the official bulletin d'adhésion for this act's theme and hand
/// it to the OS viewer. The PDF is the Assurance Maladie's own; only
/// its form fields are written (see [`crate::bulletin`]).
pub fn open_bulletin(
    kind: InterviewKind,
    patient: &Patient,
    pharmacy: &PharmacyConfig,
) -> Result<PathBuf, String> {
    let bytes = crate::bulletin::fill(kind, patient, pharmacy)?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let out = std::env::temp_dir().join(format!(
        "bpm_caddy_bulletin_{}_{}_{stamp}.pdf",
        patient.id,
        kind.as_str().to_lowercase()
    ));
    std::fs::write(&out, bytes).map_err(|e| format!("écriture du bulletin impossible : {e}"))?;
    open::that_detached(&out).map_err(|e| format!("ouverture du PDF impossible : {e}"))?;
    crate::telemetry::tally(crate::telemetry::Signal::Printed);
    Ok(out)
}

/// The patient's carnet de vaccination on one sheet: the doses in the
/// order they were given, with the lot and the site, so it can be
/// filed, handed over or sent to the médecin traitant.
pub fn open_vaccination_carnet(
    patient: &Patient,
    lines: &[crate::db::Vaccination],
    mention: &str,
    template_path: &std::path::Path,
) -> Result<PathBuf, String> {
    compile_and_open(
        fill(
            &template_source("vaccination", template_path),
            &vaccination_carnet_values(patient, lines, mention),
        ),
        "carnet_vaccination",
    )
}

/// Le carnet de vaccination, rempli mais pas encore compilé — pour la
/// liasse d'une vaccination, qui le met après la fiche.
pub fn vaccination_carnet_source(
    patient: &Patient,
    lines: &[crate::db::Vaccination],
    mention: &str,
    template_path: &std::path::Path,
) -> String {
    fill(
        &template_source("vaccination", template_path),
        &vaccination_carnet_values(patient, lines, mention),
    )
}

fn vaccination_carnet_values(
    patient: &Patient,
    lines: &[crate::db::Vaccination],
    mention: &str,
) -> Vec<(&'static str, String)> {
    let mut rows = String::new();
    // Oldest first on paper: a carnet is read forwards, unlike the
    // screen's table, where the dose just given belongs on top.
    let mut ordered: Vec<&crate::db::Vaccination> = lines.iter().collect();
    ordered.sort_by(|a, b| a.given_on.cmp(&b.given_on));
    for line in ordered {
        let remark = if line.next_due.is_empty() {
            line.remark.clone()
        } else if line.remark.is_empty() {
            format!(
                "Prochaine : {}",
                crate::db::format_french_date(&line.next_due)
            )
        } else {
            format!(
                "Prochaine : {} — {}",
                crate::db::format_french_date(&line.next_due),
                line.remark
            )
        };
        rows.push_str(&format!(
            "{}, {}, {}, {}, {}, {}, {},\n",
            typst_str(&if line.given_on.is_empty() {
                "—".to_owned()
            } else {
                crate::db::format_french_date(&line.given_on)
            }),
            typst_str(&line.label),
            typst_str(&line.dose),
            typst_str(&line.lot),
            typst_str(&line.site),
            typst_str(&line.operator),
            typst_str(&remark),
        ));
    }
    if rows.is_empty() {
        rows.push_str("[], [], [], [], [], [], [],\n");
    }
    let head = typst_str(&patient.full_name());
    let born = typst_str(&crate::db::format_french_date(&patient.birth_date));
    // The foot of the page is the officine's own mention, and there is
    // no line at all until it writes one.
    let foot = if mention.trim().is_empty() {
        String::new()
    } else {
        format!(
            "#v(4mm)\n#text(8pt, style: \"italic\")[#{}]",
            typst_str(mention.trim())
        )
    };
    vec![
        ("{{PATIENT_NAME}}", format!("#{head}")),
        ("{{BIRTH_DATE}}", format!("#{born}")),
        ("{{ROWS}}", rows),
        ("{{MENTION}}", foot),
    ]
}

const MARKERS_VACCINATION: &[&str] = &[
    "{{PATIENT_NAME}}",
    "{{BIRTH_DATE}}",
    "{{ROWS}}",
    "{{MENTION}}",
];

const DEFAULT_VACCINATION_TEMPLATE: &str = r##"
#set page(paper: "a4", flipped: true, margin: 1.4cm)
#set text(size: 10pt, lang: "fr")
#align(center)[#text(16pt, weight: "bold")[Carnet de vaccination]]
#v(1mm)
#align(center)[#text(12pt)[{{PATIENT_NAME}}] — né(e) le {{BIRTH_DATE}}]
#v(5mm)
#table(
  columns: (auto, 1fr, auto, auto, auto, auto, 1fr),
  inset: 6pt,
  stroke: 0.6pt,
  [*Date*], [*Vaccin*], [*Dose*], [*Lot*], [*Site*], [*Par*], [*Remarque*],
{{ROWS}})
{{MENTION}}
"##;

/// One line of the printable billing recap: what the memo asks the
/// pharmacy to send — the act code, the step it pays, the situation to
/// declare and the amount, patient by patient.
pub struct BillingLine {
    pub date: String,
    pub patient: String,
    pub kind: String,
    pub code: String,
    pub step: String,
    pub situation: String,
    pub remote: bool,
    pub coverage: u32,
    pub fee: f64,
}

/// Build the billing recap: the acts to invoice, their codes and their
/// amounts, with the total at the foot.
/// One rental as the recap prints it: the patient, the material, what
/// it has run for and what that comes to.
pub struct BillingRental {
    pub patient: String,
    pub label: String,
    pub started: String,
    /// Empty while the material is still out.
    pub ended: String,
    pub periods: u32,
    pub period_word: String,
    pub amount: f64,
}

fn billing_recap_values(
    lines: &[BillingLine],
    rentals: &[BillingRental],
    period: &str,
    today_french: &str,
) -> Vec<(&'static str, String)> {
    let mut rows = String::new();
    let mut total = 0.0;
    for l in lines {
        total += l.fee;
        let code = if l.remote {
            format!("{} + {}", l.code, crate::db::REMOTE_CODE)
        } else {
            l.code.clone()
        };
        rows.push_str(&format!(
            "{}, {}, {}, {}, {}, {}, {}, {},\n",
            typst_str(&crate::db::format_french_date(&l.date)),
            typst_str(&l.patient),
            typst_str(&l.kind),
            typst_str(&code),
            typst_str(&l.step),
            typst_str(&l.situation),
            typst_str(&format!("{} %", l.coverage)),
            typst_str(&format!("{:.2} €", l.fee).replace('.', ",")),
        ));
    }
    // **Le symbole, comme partout ailleurs.** Ce récapitulatif écrivait
    // « 15,00 EUR » là où l'écran, les feuilles de caisse et le registre
    // écrivent « 15,00 € » — le même chiffre sous deux graphies, sur les
    // deux documents qu'on met côte à côte en fin de mois.
    let total = format!("{total:.2} €").replace('.', ",");
    let count = lines.len();
    // The rentals are a second table, not more rows of the first: they
    // are not acts, they have no code acte and no étape, and adding them
    // to the same grid would invite them into the acts' total.
    let mut rental_block = String::new();
    if !rentals.is_empty() {
        let mut rows = String::new();
        let mut sum = 0.0;
        for r in rentals {
            sum += r.amount;
            rows.push_str(&format!(
                "{}, {}, {}, {}, {}, {},\n",
                typst_str(&r.patient),
                typst_str(&r.label),
                typst_str(&crate::db::format_french_date(&r.started)),
                typst_str(&if r.ended.trim().is_empty() {
                    "en cours".to_owned()
                } else {
                    crate::db::format_french_date(&r.ended)
                }),
                typst_str(&format!("{} {}", r.periods, r.period_word)),
                typst_str(&format!("{:.2} €", r.amount).replace('.', ",")),
            ));
        }
        let sum = format!("{sum:.2} €").replace('.', ",");
        rental_block = format!(
            r#"
#v(6mm)
#text(13pt, weight: "bold")[Locations de matériel]
#v(2mm)
#table(
  columns: (1fr, auto, auto, auto, auto, auto),
  inset: 6pt,
  stroke: 0.6pt,
  [*Patient*], [*Matériel*], [*Posé le*], [*Repris le*], [*Périodes*], [*Montant*],
{rows})
#v(2mm)
#text(weight: "bold")[{n} location(s) — total {sum}]
#v(2mm)
#text(9pt)[Forfaits enregistrés à la pose. Ligne LPP et tarif à vérifier avant facturation.]
"#,
            n = rentals.len()
        );
    }
    vec![
        // Des libellés que l'officine peut réécrire dans `strings.toml` :
        // échappés comme le reste.
        ("{{PERIOD}}", format!("#{}", typst_str(period))),
        ("{{DATE}}", format!("#{}", typst_str(today_french))),
        ("{{ROWS}}", rows),
        ("{{COUNT}}", count.to_string()),
        ("{{TOTAL}}", total),
        ("{{RENTALS}}", rental_block),
    ]
}

const MARKERS_FACTURATION: &[&str] = &[
    "{{PERIOD}}",
    "{{DATE}}",
    "{{ROWS}}",
    "{{COUNT}}",
    "{{TOTAL}}",
    "{{RENTALS}}",
];

const DEFAULT_FACTURATION_TEMPLATE: &str = r##"
#set page(paper: "a4", margin: 1.5cm, flipped: true)
#set text(size: 10pt)
#align(center)[#text(16pt, weight: "bold")[Récapitulatif de facturation]]
#v(1mm)
#align(center)[{{PERIOD}} — édité le {{DATE}}]
#v(5mm)
#table(
  columns: (auto, 1fr, auto, auto, auto, auto, auto, auto),
  inset: 6pt,
  stroke: 0.6pt,
  [*Date*], [*Patient*], [*Thème*], [*Code acte*], [*Étape*], [*Situation*],
  [*Prise en charge*], [*Montant*],
{{ROWS}})
#v(4mm)
#text(weight: "bold")[{{COUNT}} acte(s) — total {{TOTAL}}]
#v(3mm)
#text(9pt)[Prestation facturée en tiers payant, indépendamment de tout code CIP, aux prix TTC. Une seule pharmacie accompagne un patient : celle qui a débuté la séquence annuelle perçoit la rémunération.]
{{RENTALS}}
"##;

/// Compile and open the billing recap for printing.
pub fn open_billing_recap(
    lines: &[BillingLine],
    rentals: &[BillingRental],
    period: &str,
    today_french: &str,
    template_path: &std::path::Path,
) -> Result<PathBuf, String> {
    compile_and_open(
        fill(
            &template_source("facturation", template_path),
            &billing_recap_values(lines, rentals, period, today_french),
        ),
        "facturation",
    )
}

/// Compile and open the RDV list for printing.
pub fn open_appointment_list(
    rdvs: &[Appointment],
    today_french: &str,
    template_path: &std::path::Path,
) -> Result<PathBuf, String> {
    compile_and_open(
        fill(
            &template_source("rdv", template_path),
            &appointment_list_values(rdvs, today_french),
        ),
        "rdv",
    )
}

fn format_diagnostics(errs: &[typst::diag::SourceDiagnostic]) -> String {
    errs.iter()
        .map(|d| d.message.to_string())
        .collect::<Vec<_>>()
        .join(" ; ")
}

// ===================================================================
// Le registre des documents imprimables
// ===================================================================
//
// Quatre documents avaient un modèle éditable — la fiche, le courrier,
// le carnet, l'ordonnance — et vingt-deux n'en avaient pas : leur
// Typst était écrit en Rust, avec la mise en page et les données
// mélangées dans le même `format!`. Une officine qui voulait sa
// marge, son en-tête ou sa police sur la liste d'appel n'avait rien à
// ouvrir.
//
// Le registre les met tous sur le même pied. Un document, c'est :
// une clé, un nom, un modèle Typst par défaut, et la liste des
// marqueurs `{{…}}` qu'il accepte. L'appelant construit les valeurs,
// `fill` les substitue, et le modèle vient du disque s'il existe.
//
// **Trois règles, chacune tenue par un test**, parce qu'un modèle est
// la seule chose ici que l'utilisateur peut casser :
//
// * Les marqueurs déclarés et ceux qui apparaissent dans le modèle par
//   défaut sont **exactement les mêmes**. Un marqueur oublié dans la
//   liste est un marqueur que l'éditeur ne montre pas, donc que
//   personne n'utilise ; un marqueur déclaré et absent du modèle est
//   une promesse que rien ne tient.
// * Un modèle rempli ne contient plus de `{{`. Un marqueur mal tapé
//   s'imprime tel quel, en toutes lettres, au milieu de la page.
// * Chaque modèle par défaut **compile** avec ses valeurs d'exemple.
//   C'est aussi ce qui donne l'aperçu de l'éditeur sans dossier ouvert.

/// Une ligne de registre d'exemple, pour vérifier et prévisualiser les
/// modèles du registre sans ouvrir la base.
fn sample_stup_move() -> crate::db::StupMove {
    crate::db::StupMove {
        id: 1,
        stup_id: 1,
        kind: "SORTIE".to_owned(),
        happened_on: "2026-08-29".to_owned(),
        quantity: 14.0,
        ordo_year: 2026,
        ordo_no: 37,
        patient_id: 12,
        prescriber: "Dr Martin".to_owned(),
        supplier: String::new(),
        reference: String::new(),
        lot: "L4821B".to_owned(),
        expiry: "2027-04-30".to_owned(),
        expected: 0.0,
        operator: "CL".to_owned(),
        remark: String::new(),
        cancels: 0,
    }
}

/// Un document imprimable et son modèle.
pub struct Doc {
    /// La clé : le nom du fichier `<clé>.typ` et l'identifiant partout.
    pub key: &'static str,
    /// La **clé de chaîne** du nom que l'éditeur affiche, résolue par
    /// `strings::tr` au moment du dessin — jamais le libellé lui-même :
    /// les textes de l'interface vivent tous dans
    /// `assets/strings.fr.toml`, où l'officine peut les remplacer.
    pub label: &'static str,
    /// Les marqueurs que ce modèle accepte, dans l'ordre où ils
    /// apparaissent sur la page.
    pub markers: &'static [&'static str],
    /// Le modèle embarqué, utilisé tant que l'officine n'en a pas
    /// écrit un.
    pub default: &'static str,
}

/// Tous les documents imprimables de l'application.
///
/// Le bulletin d'adhésion n'y est pas et ne doit pas y être : ce n'est
/// pas un Typst mais le PDF de l'Assurance Maladie, rempli champ par
/// champ (voir `bulletin.rs`). Le rendre « éditable » serait le
/// redessiner, ce que docs/ARCHITECTURE.md interdit explicitement.
pub const DOCS: &[Doc] = &[
    Doc {
        key: "fiche",
        label: "tpl_target_fiche",
        markers: MARKERS_FICHE,
        default: DEFAULT_TEMPLATE,
    },
    Doc {
        key: "cr",
        label: "tpl_target_cr",
        markers: MARKERS_CR,
        default: DEFAULT_CR_TEMPLATE,
    },
    Doc {
        key: "carnet",
        label: "tpl_target_carnet",
        markers: MARKERS_CARNET,
        default: DEFAULT_TRANS_TEMPLATE,
    },
    Doc {
        key: "ordonnance",
        label: "tpl_target_ordonnance",
        markers: MARKERS_ORDONNANCE,
        default: DEFAULT_ORDONNANCE_TEMPLATE,
    },
    Doc {
        key: "plan",
        label: "tpl_target_plan",
        markers: MARKERS_PLAN,
        default: DEFAULT_PLAN_TEMPLATE,
    },
    Doc {
        key: "traitement",
        label: "tpl_target_traitement",
        markers: MARKERS_TRAITEMENT,
        default: DEFAULT_TRAITEMENT_TEMPLATE,
    },
    Doc {
        key: "controle",
        label: "tpl_target_controle",
        markers: MARKERS_CONTROLE,
        default: DEFAULT_CONTROLE_TEMPLATE,
    },
    Doc {
        key: "destruction",
        label: "tpl_target_destruction",
        markers: MARKERS_DESTRUCTION,
        default: DEFAULT_DESTRUCTION_TEMPLATE,
    },
    Doc {
        key: "appels",
        label: "tpl_target_appels",
        markers: MARKERS_APPELS,
        default: DEFAULT_APPELS_TEMPLATE,
    },
    Doc {
        key: "rdv",
        label: "tpl_target_rdv",
        markers: MARKERS_RDV,
        default: DEFAULT_RDV_TEMPLATE,
    },
    Doc {
        key: "guide",
        label: "tpl_target_guide",
        markers: MARKERS_GUIDE,
        default: DEFAULT_GUIDE_TEMPLATE,
    },
    Doc {
        key: "conciliation",
        label: "tpl_target_conciliation",
        markers: MARKERS_CONCILIATION,
        default: DEFAULT_CONCILIATION_TEMPLATE,
    },
    Doc {
        key: "preparation",
        label: "tpl_target_preparation",
        markers: MARKERS_PREPARATION,
        default: DEFAULT_PREPARATION_TEMPLATE,
    },
    Doc {
        key: "protocole",
        label: "tpl_target_protocole",
        markers: MARKERS_PROTOCOLE,
        default: DEFAULT_PROTOCOLE_TEMPLATE,
    },
    Doc {
        key: "liste",
        label: "tpl_target_liste",
        markers: MARKERS_LISTE,
        default: DEFAULT_LISTE_TEMPLATE,
    },
    Doc {
        key: "semaine",
        label: "tpl_target_semaine",
        markers: MARKERS_SEMAINE,
        default: DEFAULT_SEMAINE_TEMPLATE,
    },
    Doc {
        key: "tables",
        label: "tpl_target_tables",
        markers: MARKERS_TABLES,
        default: DEFAULT_TABLES_TEMPLATE,
    },
    Doc {
        key: "ordonnancier",
        label: "tpl_target_ordonnancier",
        markers: MARKERS_ORDONNANCIER,
        default: DEFAULT_ORDONNANCIER_TEMPLATE,
    },
    Doc {
        key: "vaccination",
        label: "tpl_target_vaccination",
        markers: MARKERS_VACCINATION,
        default: DEFAULT_VACCINATION_TEMPLATE,
    },
    Doc {
        key: "facturation",
        label: "tpl_target_facturation",
        markers: MARKERS_FACTURATION,
        default: DEFAULT_FACTURATION_TEMPLATE,
    },
    Doc {
        key: "monographie",
        label: "tpl_target_monographie",
        markers: MARKERS_MONOGRAPHIE,
        default: DEFAULT_MONOGRAPHIE_TEMPLATE,
    },
    Doc {
        key: "bilan",
        label: "tpl_target_bilan",
        markers: MARKERS_BILAN,
        default: DEFAULT_BILAN_TEMPLATE,
    },
    Doc {
        key: "suivi",
        label: "tpl_target_suivi",
        markers: MARKERS_SUIVI,
        default: DEFAULT_SUIVI_TEMPLATE,
    },
    Doc {
        key: "codex",
        label: "tpl_target_codex",
        markers: MARKERS_CODEX,
        default: DEFAULT_CODEX_TEMPLATE,
    },
    Doc {
        key: "dispositif",
        label: "tpl_target_dispositif",
        markers: MARKERS_DISPOSITIF,
        default: DEFAULT_DISPOSITIF_TEMPLATE,
    },
    Doc {
        key: "dispositifs",
        label: "tpl_target_dispositifs",
        markers: MARKERS_DISPOSITIFS,
        default: DEFAULT_DISPOSITIFS_TEMPLATE,
    },
    Doc {
        key: "registre",
        label: "tpl_target_registre",
        markers: MARKERS_REGISTRE,
        default: DEFAULT_REGISTRE_TEMPLATE,
    },
    Doc {
        key: "caisse",
        label: "tpl_target_caisse",
        markers: MARKERS_CAISSE,
        default: DEFAULT_CAISSE_TEMPLATE,
    },
    Doc {
        key: "caisses",
        label: "tpl_target_caisses",
        markers: MARKERS_CAISSES,
        default: DEFAULT_CAISSES_TEMPLATE,
    },
    Doc {
        key: "ecraser",
        label: "tpl_target_ecraser",
        markers: MARKERS_ECRASER,
        default: DEFAULT_ECRASER_TEMPLATE,
    },
    Doc {
        key: "planning",
        label: "tpl_target_planning",
        markers: MARKERS_PLANNING,
        default: DEFAULT_PLANNING_TEMPLATE,
    },
    Doc {
        key: "etiquettes",
        label: "tpl_target_etiquettes",
        markers: MARKERS_ETIQUETTES,
        default: DEFAULT_ETIQUETTES_TEMPLATE,
    },
    Doc {
        key: "surveillance",
        label: "tpl_target_surveillance",
        markers: MARKERS_SURVEILLANCE,
        default: DEFAULT_SURVEILLANCE_TEMPLATE,
    },
    Doc {
        key: "trod",
        label: "tpl_target_trod",
        markers: MARKERS_TROD,
        default: DEFAULT_TROD_TEMPLATE,
    },
    Doc {
        key: "vaccin",
        label: "tpl_target_vaccin",
        markers: MARKERS_VACCIN,
        default: DEFAULT_VACCIN_TEMPLATE,
    },
    Doc {
        key: "trod_cr",
        label: "tpl_target_trod_cr",
        markers: MARKERS_TROD_CR,
        default: DEFAULT_TROD_CR_TEMPLATE,
    },
];

const MARKERS_FICHE: &[&str] = &[
    "{{PHARMACY_NAME}}",
    "{{PHARMACY_PHONE}}",
    "{{AGE}}",
    "{{PATIENT_NAME}}",
    "{{BIRTH_DATE}}",
    "{{DATE}}",
    "{{KIND}}",
    "{{THEME}}",
    "{{TREATMENTS}}",
    "{{CHECKLIST}}",
    "{{PHARMACIST}}",
];

const MARKERS_CR: &[&str] = &[
    "{{POINTS}}",
    "{{PHARMACY_NAME}}",
    "{{PHARMACY_ADDRESS}}",
    "{{PHARMACY_PHONE}}",
    "{{PHYSICIAN}}",
    "{{PATIENT_NAME}}",
    "{{BIRTH_DATE}}",
    "{{KIND}}",
    "{{DATE}}",
    "{{THEME}}",
    "{{TREATMENTS}}",
    "{{PHARMACIST}}",
];

const MARKERS_CARNET: &[&str] = &["{{DAY}}", "{{ENTRIES}}"];

const MARKERS_ORDONNANCE: &[&str] = &[
    "{{PHARMACY_NAME}}",
    "{{PHARMACY_ADDRESS}}",
    "{{PHARMACY_PHONE}}",
    "{{PHARMACY_AM}}",
    "{{PATIENT_NAME}}",
    "{{BIRTH_DATE}}",
    "{{INDICATION}}",
    "{{DATE}}",
    "{{LINES}}",
    "{{ADVICE}}",
    "{{MENTION_HEADER}}",
    "{{MENTION_FOOTER}}",
    "{{PHARMACIST}}",
];

const MARKERS_CAISSE: &[&str] = &[
    "{{PHARMACY_NAME}}",
    "{{DATE}}",
    "{{OPERATOR}}",
    "{{NOTES_ROWS}}",
    "{{COINS_ROWS}}",
    "{{NOTES_TOTAL}}",
    "{{COINS_TOTAL}}",
    "{{CASH}}",
    "{{OPENING}}",
    "{{OTHERS}}",
    "{{OTHER_TOTAL}}",
    "{{TAKINGS}}",
    "{{EXPECTED}}",
    "{{GAP}}",
    "{{FLOAT}}",
    "{{BANKED}}",
    "{{REMARK}}",
];

/// Le document de cette clé.
#[must_use]
pub fn doc(key: &str) -> Option<&'static Doc> {
    DOCS.iter().find(|d| d.key == key)
}

/// Substituer les marqueurs. Les valeurs sont déjà du Typst : un texte
/// venu d'un dossier passe par [`typst_str`] avant d'arriver ici, de
/// sorte qu'un nom contenant `#` ou `*` ne peut ni casser la
/// compilation ni redessiner la page.
#[must_use]
///
/// **En une seule passe, de gauche à droite.** Remplacés l'un après
/// l'autre, un marqueur tapé *dans* une valeur déjà posée était
/// substitué à son tour : une remarque de caisse « {{OPERATOR}} »
/// s'imprimait avec les initiales de l'opérateur. Ce qui a été posé ne
/// se relit plus.
pub fn fill(template: &str, values: &[(&str, String)]) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(at) = rest.find("{{") {
        out.push_str(&rest[..at]);
        let tail = &rest[at..];
        match values.iter().find(|(m, _)| tail.starts_with(m)) {
            Some((marker, value)) => {
                out.push_str(value);
                rest = &tail[marker.len()..];
            }
            None => {
                out.push_str("{{");
                rest = &tail[2..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// Le modèle de ce document : celui du disque s'il existe, sinon celui
/// qui est embarqué.
///
/// Une erreur de lecture n'est pas silencieuse — mais elle ne doit pas
/// empêcher d'imprimer : un modèle illisible rend le modèle par
/// défaut, et l'éditeur, lui, dit pourquoi.
#[must_use]
pub fn template_source(key: &str, path: &std::path::Path) -> String {
    let embedded = doc(key).map_or("", |d| d.default);
    if path.exists() {
        std::fs::read_to_string(path).unwrap_or_else(|_| embedded.to_owned())
    } else {
        embedded.to_owned()
    }
}

/// Compiler un modèle avec des valeurs d'exemple : la validation de
/// l'éditeur.
pub fn check_doc(key: &str, template: &str) -> Result<(), String> {
    let filled = fill(template, &sample_values(key));
    if let Some(rest) = filled.split_once("{{") {
        let stray: String = rest.1.chars().take_while(|c| *c != '}').collect();
        return Err(format!(
            "marqueur inconnu « {{{{{stray}}}}} » : il s'imprimerait tel quel."
        ));
    }
    let world = PdfWorld::new(filled);
    typst::compile::<PagedDocument>(&world)
        .output
        .map(|_| ())
        .map_err(|errs| format!("compilation Typst : {}", format_diagnostics(&errs)))
}

/// Compiler un modèle avec des valeurs d'exemple et l'ouvrir : le
/// bouton « Aperçu » de l'éditeur.
pub fn preview_doc(key: &str, template: &str) -> Result<PathBuf, String> {
    check_doc(key, template)?;
    compile_and_open(
        fill(template, &sample_values(key)),
        &format!("apercu_{key}"),
    )
}

/// Les valeurs d'exemple d'un document, pour vérifier et prévisualiser
/// un modèle sans dossier ouvert.
///
/// Elles ne sont pas vides : un modèle validé sur des chaînes vides
/// compile toujours, et se casse à la première vraie impression.
fn sample_values(key: &str) -> Vec<(&'static str, String)> {
    let patient = sample_patient();
    let pharmacy = sample_pharmacy();
    let s = |v: &str| format!("#{}", typst_str(v));
    match key {
        "fiche" => interview_values(
            &InterviewPaper {
                patient: &patient,
                kind: InterviewKind::Bpm,
                date: "24/08/2026",
                age: crate::db::age_on(&patient.birth_date, "2026-08-24"),
                theme: "Observance",
                signature: &pharmacy.pharmacist,
                treats: &sample_treatments(),
                checklist: crate::entretien::checklist("Observance"),
            },
            &pharmacy,
        ),
        "cr" => vec![
            (
                "{{POINTS}}",
                cr_points_markup(&["Observance satisfaisante sur les trois derniers mois."]),
            ),
            ("{{PHARMACY_NAME}}", s(&pharmacy.name)),
            ("{{PHARMACY_ADDRESS}}", s(&pharmacy.address)),
            ("{{PHARMACY_PHONE}}", s(&pharmacy.phone)),
            ("{{PHYSICIAN}}", s("Docteur Martin")),
            ("{{PATIENT_NAME}}", s(&patient.full_name())),
            (
                "{{BIRTH_DATE}}",
                s(&crate::db::format_french_date(&patient.birth_date)),
            ),
            ("{{KIND}}", s(InterviewKind::Bpm.label())),
            ("{{DATE}}", s("24/08/2026")),
            ("{{THEME}}", s("Observance")),
            ("{{TREATMENTS}}", treatments_markup(&sample_treatments())),
            ("{{PHARMACIST}}", s(&pharmacy.pharmacist)),
        ],
        // Le carnet : les mêmes transmissions d'exemple que le test du
        // carnet, et par la même fonction de remplissage — deux
        // constructions d'une même page finissent toujours par
        // diverger, et c'est l'aperçu qui ment.
        "carnet" => {
            let filled = fill_trans_template(
                "{{DAY}}\u{1}{{ENTRIES}}",
                "Lundi 24/08/2026",
                &sample_transmissions(),
            );
            let (day, entries) = filled.split_once('\u{1}').unwrap_or((&filled, ""));
            vec![
                ("{{DAY}}", day.to_owned()),
                ("{{ENTRIES}}", entries.to_owned()),
            ]
        }
        "plan" => plan_values(
            &PlanData {
                patient: &patient,
                today: "24/08/2026",
                // **Un plan de prise porte une ordonnance entière**, six
                // ou huit lignes, et c'est là-dessus qu'on règle le
                // partage des quatre colonnes. Sur une seule, ni une
                // posologie longue — celle qui décide de la largeur —,
                // ni une remarque qui enveloppe, ni l'alternance des
                // rangées ne se voient.
                lines: vec![
                    (
                        "Amlodipine 5 mg".to_owned(),
                        "Tension artérielle".to_owned(),
                        "1 comprimé le matin".to_owned(),
                        "Ne pas arrêter sans avis.".to_owned(),
                    ),
                    (
                        "Lévothyrox 75 µg".to_owned(),
                        "Thyroïde".to_owned(),
                        "1 comprimé le matin à jeun, 30 minutes avant le petit-déjeuner"
                            .to_owned(),
                        "À distance du calcium, du fer et du magnésium : quatre heures."
                            .to_owned(),
                    ),
                    (
                        "Eliquis 5 mg".to_owned(),
                        "Anticoagulant".to_owned(),
                        "1 comprimé matin et soir, à heure fixe".to_owned(),
                        "Ne jamais doubler la dose pour rattraper un oubli. Signaler tout \
                         saignement qui ne s'arrête pas."
                            .to_owned(),
                    ),
                    (
                        "Doliprane 1 g".to_owned(),
                        "Douleur ou fièvre".to_owned(),
                        "1 comprimé si besoin, 6 heures entre deux prises".to_owned(),
                        "Jamais plus de 3 g par jour, et une seule boîte de paracétamol à \
                         la fois."
                            .to_owned(),
                    ),
                ],
                mention: "Document remis à titre informatif.",
                signature: &pharmacy.pharmacist,
            },
            &pharmacy,
        ),
        // **La fiche de traitement montre ce qu'elle sait faire.** Les
        // cinq lignes ne sont pas cinq exemples de la même chose : une
        // prise placée avec sa quantité, une prise placée sans quantité
        // (« 20 mg le soir » — la pastille), une prise à la demande,
        // une prise hebdomadaire et une posologie que la grille ne sait
        // pas lire. Ce sont les cinq cas d'`intake.rs`, et un aperçu
        // qui n'en montre qu'un ne dit pas comment le tableau se
        // comporte quand il rencontre les autres.
        "traitement" => {
            let line = |name: &str, what: &str, poso: &str, know: &str, missed: &str, watch: &str| {
                FicheLine {
                    name: name.to_owned(),
                    what: what.to_owned(),
                    posology: poso.to_owned(),
                    reading: crate::intake::read(poso),
                    know: know.to_owned(),
                    missed: missed.to_owned(),
                    watch: watch.to_owned(),
                }
            };
            let ordonnance = crate::renewal::Prescription {
                prescribed_on: "2026-08-24".to_owned(),
                duration_days: 30,
                renewals: 2,
                dispensed: 2,
                dispensed_on: "2026-09-23".to_owned(),
            };
            traitement_values(
                &FicheData {
                    patient: &patient,
                    today: "24/08/2026",
                    prescriber: "Docteur Martin",
                    lines: vec![
                        line(
                            "Amlor 5 mg",
                            "Tension artérielle",
                            "1 comprimé le matin",
                            "Des chevilles qui gonflent en fin de journée sont l'effet le \
                             plus courant : surélever les jambes aide, et si la gêne \
                             persiste, la dose peut être revue. Évitez le jus de \
                             pamplemousse.",
                            "Prenez-le dès que vous y pensez dans la journée ; passez la \
                             dose si le lendemain est déjà là.",
                            "",
                        ),
                        line(
                            "Tahor 20 mg",
                            "Cholestérol",
                            "20 mg le soir",
                            "Les douleurs musculaires diffuses se signalent : elles ne sont \
                             pas une fatalité du traitement.",
                            "",
                            "Douleurs musculaires intenses, faiblesse, urines foncées.",
                        ),
                        line(
                            "Doliprane 1 g",
                            "Douleur ou fièvre",
                            "1 comprimé si besoin, 6 heures entre deux prises",
                            "Jamais plus de 3 g par jour, et une seule boîte de paracétamol \
                             à la fois.",
                            "",
                            "",
                        ),
                        line(
                            "Méthotrexate 10 mg",
                            "Polyarthrite",
                            "1 comprimé par semaine, le lundi matin",
                            "Une seule prise par semaine, le même jour. L'acide folique se \
                             prend le lendemain ou le surlendemain.",
                            "",
                            "Fièvre, angine, aphtes dans la bouche, toux ou essoufflement.",
                        ),
                        line(
                            "Coumadine",
                            "Anticoagulant",
                            "Dose adaptée à l'INR",
                            "",
                            "",
                            "",
                        ),
                    ],
                    stands: vec![FicheStand {
                        stand: crate::renewal::read(&ordonnance, "2026-09-23", 7),
                        prescription: ordonnance.clone(),
                        treatments: vec!["Amlor 5 mg".to_owned(), "Tahor 20 mg".to_owned()],
                    }],
                    viz: crate::renewal::Viz::default(),
                    mention: "Document remis à titre informatif.",
                    signature: &pharmacy.pharmacist,
                },
                &pharmacy,
            )
        }
        // **Une liste d'appels montre une liste.** Sur une seule ligne,
        // rien ne dit comment les colonnes se partagent, ni ce que
        // devient un dossier sans téléphone — qui est précisément le
        // cas qu'on cherche en imprimant cette feuille.
        "appels" => call_list_values(
            &[
                CallRow {
                    name: "Jean Dupont",
                    phone: "04 67 00 00 00",
                    tag: "2 alerte(s)",
                    reason: "Kaliémie à 5,4 sous IEC.",
                },
                CallRow {
                    name: "Claire Martin",
                    phone: "06 12 34 56 78",
                    tag: "Ordonnance",
                    reason: "AINS au long cours à revoir avec le prescripteur.",
                },
                CallRow {
                    name: "Hélène Lefèvre",
                    phone: "",
                    tag: "RDV en retard",
                    reason: "Entretien AOD non replanifié depuis le 12/07.",
                },
            ],
            "24/08/2026",
            &pharmacy,
        ),
        // **Une liste de rendez-vous montre une liste.** Sur un seul,
        // ni le groupement par jour, ni la marque du « à distance », ni
        // ce que devient un dossier sans téléphone ne se voient — et
        // c'est tout ce que ce modèle-là met en page.
        "rdv" => {
            let rdv = |id, name: &str, phone: &str, kind, date: &str, time: &str, remote| {
                Appointment {
                    id,
                    patient_id: id,
                    patient_name: name.to_owned(),
                    phone: phone.to_owned(),
                    kind,
                    date: date.to_owned(),
                    time: time.to_owned(),
                    remote,
                    duration_minutes: 30,
                    operator: "CL".to_owned(),
                }
            };
            appointment_list_values(
                &[
                    rdv(
                        1,
                        &patient.full_name(),
                        "04 67 00 00 00",
                        InterviewKind::Bpm,
                        "2026-09-14",
                        "09:30",
                        false,
                    ),
                    rdv(
                        2,
                        "Claire Martin",
                        "06 12 34 56 78",
                        InterviewKind::Aod,
                        "2026-09-14",
                        "11:00",
                        true,
                    ),
                    rdv(
                        3,
                        "Hélène Lefèvre",
                        "",
                        InterviewKind::Asthme,
                        "2026-09-15",
                        "14:15",
                        false,
                    ),
                ],
                "24/08/2026",
            )
        }
        "guide" => guide_values(&pharmacy),
        // **Une monographie d'aperçu montre des *sections*.** Elle n'en
        // portait qu'une — la posologie —, or c'est justement le style
        // des sections (`#let sec`) que ce modèle-ci laisse régler :
        // sur une seule, rien ne se juge.
        "monographie" => monograph_values(
            &Drug {
                name: "Eliquis".to_owned(),
                dci: "apixaban".to_owned(),
                class: "AOD".to_owned(),
                dosage: "Fibrillation atriale : 5 mg deux fois par jour ; 2,5 mg deux fois \
                         par jour si au moins deux critères parmi âge ≥ 80 ans, poids ≤ 60 kg, \
                         créatininémie ≥ 133 µmol/L."
                    .to_owned(),
                contraindications: "Saignement évolutif, valve mécanique, grossesse et \
                                    allaitement, clairance sous 15 mL/min."
                    .to_owned(),
                ddi: "Azolés et macrolides augmentent l'exposition ; la rifampicine et le \
                      millepertuis la diminuent. AINS : risque hémorragique additionnel."
                    .to_owned(),
                monitoring: "Clairance au moins une fois par an, hémogramme en cas de \
                             saignement ; pas de suivi d'activité en routine."
                    .to_owned(),
                missed_dose: "Prendre la dose oubliée dans les six heures ; au-delà, sauter \
                              la prise et reprendre le rythme habituel — jamais deux doses."
                    .to_owned(),
                antidote: "Andexanet alfa".to_owned(),
                ..Default::default()
            },
            &[],
            &[(
                "Liaison aux protéines".to_owned(),
                "87 % (RCP Eliquis, 5.2)".to_owned(),
            )],
        ),
        // **Une liste d'aperçu a plus d'une ligne.** Ce modèle-ci range
        // les fiches par famille : sur un seul dispositif, ni le
        // regroupement ni l'indication ne se voient.
        "dispositifs" => dispositifs_values(
            &[
                crate::db::Dispositif {
                    id: 1,
                    name: "Bas de compression classe 2".to_owned(),
                    family: "Compression".to_owned(),
                    indication: "Insuffisance veineuse chronique, prévention de la récidive \
                                 d'ulcère."
                        .to_owned(),
                    ..Default::default()
                },
                crate::db::Dispositif {
                    id: 2,
                    name: "Pansement hydrocellulaire".to_owned(),
                    family: "Pansements".to_owned(),
                    indication: "Plaie exsudative en phase de bourgeonnement ; se change \
                                 tous les deux à trois jours."
                        .to_owned(),
                    ..Default::default()
                },
                crate::db::Dispositif {
                    id: 3,
                    name: "Autopiqueur et lancettes".to_owned(),
                    family: "Diabète".to_owned(),
                    indication: "Autosurveillance glycémique : une lancette par prélèvement, \
                                 jamais réutilisée."
                        .to_owned(),
                    ..Default::default()
                },
                // **Ce modèle est en deux colonnes**, et trois fiches
                // courtes tiennent dans la première : l'officine réglait
                // donc une mise en page dont la chose la plus visible —
                // le passage d'une colonne à l'autre — ne s'y voyait
                // jamais. Ce qui suit remplit la page.
                crate::db::Dispositif {
                    id: 4,
                    name: "Lecteur de glycémie et bandelettes".to_owned(),
                    family: "Diabète".to_owned(),
                    indication: "Le lecteur et ses bandelettes vont ensemble : une \
                                 bandelette d'une autre marque ne se lit pas. Vérifier la \
                                 date d'ouverture du flacon."
                        .to_owned(),
                    ..Default::default()
                },
                crate::db::Dispositif {
                    id: 5,
                    name: "Alginate de calcium".to_owned(),
                    family: "Pansements".to_owned(),
                    indication: "Plaie très exsudative ou hémorragique ; se retire à \
                                 l'eau, jamais à sec."
                        .to_owned(),
                    ..Default::default()
                },
                crate::db::Dispositif {
                    id: 6,
                    name: "Hydrogel".to_owned(),
                    family: "Pansements".to_owned(),
                    indication: "Plaie sèche ou nécrotique à déterger ; protéger la peau \
                                 péri-lésionnelle, qu'il macère."
                        .to_owned(),
                    ..Default::default()
                },
                crate::db::Dispositif {
                    id: 7,
                    name: "Bande de contention à allongement court".to_owned(),
                    family: "Compression".to_owned(),
                    indication: "Ulcère veineux, lymphœdème : forte pression de travail, \
                                 faible pression de repos. Posée le matin, jambe non \
                                 œdématiée."
                        .to_owned(),
                    ..Default::default()
                },
                crate::db::Dispositif {
                    id: 8,
                    name: "Poche de colostomie une pièce".to_owned(),
                    family: "Stomie".to_owned(),
                    indication: "Découpe au diamètre de la stomie, ni plus ni moins : \
                                 un jour de peau à nu suffit à faire une dermite."
                        .to_owned(),
                    ..Default::default()
                },
                crate::db::Dispositif {
                    id: 9,
                    name: "Chambre d'inhalation".to_owned(),
                    family: "Respiratoire".to_owned(),
                    indication: "Aérosol-doseur chez l'enfant, le sujet âgé ou devant \
                                 toute coordination incertaine. Lavage hebdomadaire à \
                                 l'eau savonneuse, séchage à l'air libre."
                        .to_owned(),
                    ..Default::default()
                },
                crate::db::Dispositif {
                    id: 10,
                    name: "Collecteur d'aiguilles".to_owned(),
                    family: "Sécurité".to_owned(),
                    indication: "Remis avec toute délivrance d'aiguilles ou de lancettes ; \
                                 rapporté plein à l'officine."
                        .to_owned(),
                    ..Default::default()
                },
            ],
            &pharmacy,
        ),
        // **Une ligne annulée, puisque le pied en parle.** Le bas de
        // page explique longuement qu'une ligne ne se rature pas, qu'elle
        // reste barrée et qu'une autre la désigne — et l'exemple n'en
        // montrait aucune.
        "registre" => stup_register_values(
            "Skenan LP 30 mg",
            "gélule",
            &[sample_stup_move(), {
                let mut m = sample_stup_move();
                m.id = 2;
                m.ordo_no = 38;
                m.quantity = 28.0;
                // La mention porte déjà « annulée » : la remarque dit
                // ce que cela corrige, et ne le répète pas.
                m.remark = "quantité fautive".to_owned();
                m
            }],
            &[
                crate::ordonnancier::Balance {
                    stock: 24.0,
                    to_destroy: 0.0,
                    expired: 0.0,
                },
                // La ligne annulée n'a pas bougé le solde : c'est tout
                // ce que « annulée » veut dire.
                crate::ordonnancier::Balance {
                    stock: 24.0,
                    to_destroy: 0.0,
                    expired: 0.0,
                },
            ],
            &std::collections::HashSet::from([2_i64]),
            &pharmacy,
            "2026-08-29",
        ),
        // **Un aperçu ne se contredit pas.** La section « Ce que l'âge
        // change » écrit « dès 75 ans » ; au-dessus, l'en-tête donne la
        // date de naissance. Avec celle de l'échantillon commun — 1958,
        // soixante-huit ans — la page disait deux choses à la fois, et
        // c'est la page que l'officine lit pour comprendre sa propre
        // feuille. Le bilan prend donc un dossier de l'âge dont il
        // parle ; les douze autres aperçus gardent le leur.
        "bilan" => bilan_values(
            &BilanData {
                patient: &Patient {
                    birth_date: "1946-12-05".to_owned(),
                    ..sample_patient()
                },
                today: "24/08/2026",
                // Les quatre traitements que les lectures ci-dessous
                // citent : une feuille qui nomme un AINS, un IEC et une
                // statine absents de sa propre liste se contredit.
                treatments: vec![
                    (
                        "Eliquis 5 mg".to_owned(),
                        "apixaban — anticoagulant oral direct".to_owned(),
                        "1 comprimé matin et soir".to_owned(),
                    ),
                    (
                        "Advil 400 mg".to_owned(),
                        "ibuprofène — AINS".to_owned(),
                        "si douleur, sans dépasser trois jours".to_owned(),
                    ),
                    (
                        "Coversyl 5 mg".to_owned(),
                        "périndopril — IEC".to_owned(),
                        "1 comprimé le matin".to_owned(),
                    ),
                    (
                        "Tahor 20 mg".to_owned(),
                        "atorvastatine — statine".to_owned(),
                        "1 comprimé le soir".to_owned(),
                    ),
                ],
                // **Le bilan est fait de ses neuf sections**, et le
                // modèle ne montre que celles qui portent quelque
                // chose : un aperçu vide de sept d'entre elles ne dit
                // rien de la mise en page qu'on vient y régler. Une
                // ligne par section suffit.
                // Entre deux traitements **de la liste** : l'aperçu
                // montrait une interaction avec un macrolide que le
                // tableau au-dessus ne portait pas.
                interactions: vec![(
                    "Coversyl ↔ Advil".to_owned(),
                    "Un AINS diminue l'effet antihypertenseur de l'IEC et majore le risque d'insuffisance rénale aiguë.".to_owned(),
                )],
                review: vec![(
                    "ALERTE".to_owned(),
                    "Anticoagulant + AINS".to_owned(),
                    "Le risque hémorragique digestif est multiplié.".to_owned(),
                    "Eliquis · Advil".to_owned(),
                )],
                biology: vec![(
                    "20/08/2026".to_owned(),
                    "Kaliémie".to_owned(),
                    "5,4 mmol/L".to_owned(),
                    "élevé".to_owned(),
                )],
                findings: vec![(
                    "ALERTE".to_owned(),
                    "Kaliémie élevée sous IEC.".to_owned(),
                )],
                elderly: vec![(
                    "Advil — À éviter à cet âge (dès 75 ans)".to_owned(),
                    "Hémorragie digestive, insuffisance rénale aiguë, \
                     décompensation d'une insuffisance cardiaque."
                        .to_owned(),
                    "À la place : Le paracétamol en première intention ; si un AINS \
                     reste indispensable, la durée la plus courte avec un inhibiteur \
                     de la pompe à protons."
                        .to_owned(),
                )],
                watch: vec![(
                    "À REFAIRE".to_owned(),
                    "LDL-cholestérol".to_owned(),
                    "une fois par an".to_owned(),
                    "12/02/2024 (30 mois)".to_owned(),
                    "Tahor".to_owned(),
                )],
                vaccines: vec!["Grippe saisonnière".to_owned()],
                acts: vec![(
                    "20/08/2026".to_owned(),
                    "BPM".to_owned(),
                    "Observance".to_owned(),
                    "Réalisé".to_owned(),
                )],
                signature: &pharmacy.pharmacist,
            },
            &pharmacy,
        ),
        // L'aperçu montre le **modèle**, donc la feuille livrée : ce
        // qu'on règle ici est la mise en page, et une réécriture de
        // l'officine ferait douter de ce qu'on regarde.
        "suivi" => selfcheck_values(
            &crate::selfcheck::resolve(
                &crate::selfcheck::SHEETS[0],
                &crate::content::Overrides::default(),
            ),
            Some(&patient.full_name()),
            &pharmacy,
            "24/08/2026",
        ),
        // **Deux préparations, et toutes leurs sections.** Le modèle
        // pose six choses — le titre et sa forme, la formule, le
        // rendement, l'indication, le mode opératoire, la conservation
        // et la mise en garde — et l'aperçu n'en montrait que deux :
        // une fiche dont les quatre paragraphes étaient vides. On
        // réglait donc la mise en page d'un document sans en voir le
        // corps. La seconde préparation est là pour ce qu'une seule ne
        // peut pas montrer : comment deux fiches se séparent sur la
        // page.
        "codex" => codex_values(&[
            crate::db::Preparation {
                id: 1,
                name: "Pommade à l'oxyde de zinc".to_owned(),
                form: "pommade".to_owned(),
                formula: "Oxyde de zinc | 15 g\nVaseline | qsp 100 g".to_owned(),
                yield_amount: "100 g".to_owned(),
                indication: "Érythème fessier du nourrisson, irritation cutanée \
                             suintante, protection des berges d'un ulcère."
                    .to_owned(),
                method: "Tamiser l'oxyde de zinc. Incorporer par fractions à la \
                         vaseline préalablement ramollie au bain-marie tiède, en \
                         triturant au mortier jusqu'à homogénéité complète, sans \
                         grumeau visible à la spatule."
                    .to_owned(),
                conservation: "Pot en verre ou en polypropylène, à l'abri de la \
                               lumière, température ambiante. Un mois."
                    .to_owned(),
                caution: "Ne pas appliquer sur une plaie infectée ni sur une \
                          brûlure du deuxième degré sans avis."
                    .to_owned(),
                ..Default::default()
            },
            crate::db::Preparation {
                id: 2,
                name: "Solution de chlorhexidine aqueuse à 0,05 %".to_owned(),
                form: "solution pour application cutanée".to_owned(),
                formula: "Digluconate de chlorhexidine à 20 % | 0,25 g\n\
                          Eau purifiée | qsp 100 mL"
                    .to_owned(),
                yield_amount: "100 mL".to_owned(),
                indication: "Antisepsie des plaies superficielles et des \
                             écorchures."
                    .to_owned(),
                method: "Diluer la solution mère dans l'eau purifiée sous \
                         agitation douce. Répartir en flacon opaque."
                    .to_owned(),
                conservation: "Quinze jours à température ambiante, à l'abri de \
                               la lumière ; la solution diluée ne se conserve pas."
                    .to_owned(),
                caution: "Jamais en contact avec l'œil ni le conduit auditif \
                          externe si le tympan est perforé."
                    .to_owned(),
                ..Default::default()
            },
        ]),
        // **Un aperçu vide n'apprend rien du cadre.** Cette fiche
        // n'avait qu'un nom : l'officine ouvrait l'éditeur et voyait une
        // page blanche sous un titre, sans savoir ce que le modèle fait
        // des sections. Elles sont toutes remplies, brièvement.
        "dispositif" => dispositif_values(
            &crate::db::Dispositif {
                id: 1,
                name: "Bas de compression classe 2".to_owned(),
                family: "Compression".to_owned(),
                indication: "Insuffisance veineuse chronique, après un ulcère cicatrisé, \
                             prévention de la récidive."
                    .to_owned(),
                sizes: "Chaussette, bas-cuisse, collant — quatre tailles, mesurées le matin."
                    .to_owned(),
                application: "Enfilé au lever, jambe encore non œdématiée ; gant de préhension \
                              si la main est faible."
                    .to_owned(),
                renewal: "Deux paires pour six mois, portées en alternance ; renouvelables \
                          après un an."
                    .to_owned(),
                lpp: "Compression médicale, classe 2 — la ligne et son tarif se vérifient \
                      à la délivrance."
                    .to_owned(),
                caution: "Contre-indiqué en artériopathie sévère : mesurer l'IPS avant. \
                          Jamais sur une peau lésée sans avis."
                    .to_owned(),
                ..Default::default()
            },
            &pharmacy,
        ),
        // **Un carnet montre des lignes qui se suivent.** Sur une dose,
        // ni l'ordre chronologique, ni la colonne du lot vide — celle
        // d'une injection faite ailleurs — ne se voient.
        "vaccination" => vaccination_carnet_values(
            &patient,
            &[
                crate::db::Vaccination {
                    id: 1,
                    label: "dTPolio".to_owned(),
                    dose: "rappel".to_owned(),
                    given_on: "2026-03-14".to_owned(),
                    lot: "K2341".to_owned(),
                    site: "deltoïde gauche".to_owned(),
                    operator: "CL".to_owned(),
                    ..Default::default()
                },
                crate::db::Vaccination {
                    id: 2,
                    label: "Grippe saisonnière".to_owned(),
                    dose: "annuelle".to_owned(),
                    given_on: "2025-10-14".to_owned(),
                    lot: "G7812".to_owned(),
                    site: "deltoïde droit".to_owned(),
                    operator: "YS".to_owned(),
                    ..Default::default()
                },
                crate::db::Vaccination {
                    id: 3,
                    label: "Pneumocoque (VPC13)".to_owned(),
                    dose: "1re dose".to_owned(),
                    given_on: "2024-11-05".to_owned(),
                    lot: String::new(),
                    site: "deltoïde gauche".to_owned(),
                    operator: String::new(),
                    ..Default::default()
                },
            ],
            "Document remis à titre informatif.",
        ),
        // **Un récapitulatif vide n'est pas un récapitulatif.** Celui-ci
        // n'avait aucune ligne : l'officine ouvrait l'éditeur de son
        // modèle de facturation et voyait une page avec un titre, un
        // total à zéro et rien entre les deux — c'est-à-dire rien de ce
        // que ce modèle met en page. Quatre actes et une location, parce
        // que ce sont les deux tableaux qu'il porte, et parce que la
        // colonne « Code » ne montre ce qu'elle sait faire qu'avec un
        // acte à distance, qui s'y écrit « code + majoration ».
        "facturation" => billing_recap_values(
            &[
                BillingLine {
                    date: "2026-08-04".to_owned(),
                    patient: "Jean Dupont".to_owned(),
                    kind: "BPM".to_owned(),
                    code: "BPM1".to_owned(),
                    step: "Entretien initial".to_owned(),
                    situation: String::new(),
                    remote: false,
                    coverage: 70,
                    fee: 60.0,
                },
                BillingLine {
                    date: "2026-08-11".to_owned(),
                    patient: "Claire Martin".to_owned(),
                    kind: "AOD".to_owned(),
                    code: "AOD1".to_owned(),
                    step: "Entretien annuel".to_owned(),
                    situation: "ALD".to_owned(),
                    remote: true,
                    coverage: 100,
                    fee: 30.0,
                },
                BillingLine {
                    date: "2026-08-18".to_owned(),
                    patient: "Hélène Lefèvre".to_owned(),
                    kind: "Asthme".to_owned(),
                    code: "AST1".to_owned(),
                    step: "Entretien de suivi".to_owned(),
                    situation: String::new(),
                    remote: false,
                    coverage: 70,
                    fee: 20.0,
                },
                BillingLine {
                    date: "2026-08-25".to_owned(),
                    patient: "Lucie Moreau".to_owned(),
                    kind: "Vaccination".to_owned(),
                    code: "VAC".to_owned(),
                    step: "Injection".to_owned(),
                    situation: String::new(),
                    remote: false,
                    coverage: 100,
                    fee: 7.5,
                },
            ],
            &[BillingRental {
                patient: "Jean Dupont".to_owned(),
                label: "Nébuliseur".to_owned(),
                started: "2026-06-15".to_owned(),
                ended: String::new(),
                periods: 11,
                period_word: "semaines".to_owned(),
                amount: 132.0,
            }],
            "Août 2026",
            "24/08/2026",
        ),
        // **Deux lignes, dont une annulée.** Sur une seule délivrance
        // ordinaire, la colonne « État » reste vide et le barré ne se
        // voit nulle part : or c'est exactement ce que cet
        // ordonnancier-là doit savoir montrer, puisqu'une ligne ne se
        // rature jamais. Le numéro de la ligne annulée reste le sien, et
        // la suite continue après lui — la phrase du pied le dit, et
        // l'exemple le montre.
        "ordonnancier" => {
            let mut cancelled = sample_stup_move();
            cancelled.id = 2;
            cancelled.ordo_no = 38;
            cancelled.happened_on = "2026-08-29".to_owned();
            cancelled.quantity = 28.0;
            ordonnancier_values(
                &[sample_stup_move(), cancelled],
                &std::collections::HashMap::from([(1_i64, "Skenan LP 30 mg".to_owned())]),
                &std::collections::HashSet::from([2_i64]),
                2026,
                &pharmacy,
                "2026-08-29",
            )
        }
        "tables" => conversion_tables_values(&TableEdits::new()),
        "preparation" => preparation_values(
            &crate::db::Preparation {
                id: 1,
                name: "Pommade à l'oxyde de zinc".to_owned(),
                form: "pommade".to_owned(),
                indication: "Érythème fessier du nourrisson.".to_owned(),
                method: "Triturer l'oxyde de zinc dans une petite quantité d'excipient, puis compléter.".to_owned(),
                conservation: "À l'abri de la lumière, 3 mois.".to_owned(),
                caution: "Ne pas appliquer sur peau lésée suintante.".to_owned(),
                sources: "Formulaire national".to_owned(),
                ..Default::default()
            },
            "60 g",
            // **L'excipient est une matière première.** La feuille
            // n'avait que le principe actif, alors que son propre mode
            // opératoire dit « puis compléter » — compléter avec quoi,
            // la table ne le disait pas. Et le « qsp » est justement ce
            // que la colonne « À peser » ne recalcule pas.
            &[
                (
                    "Oxyde de zinc".to_owned(),
                    "15 g".to_owned(),
                    "9 g".to_owned(),
                ),
                (
                    "Vaseline".to_owned(),
                    "qsp 100 g".to_owned(),
                    "qsp 60 g".to_owned(),
                ),
            ],
            &pharmacy,
            "CL",
        ),
        // **Un protocole d'aperçu se descend.** Il n'avait qu'un nœud —
        // sa question racine —, et c'est justement l'arbre que ce
        // modèle-là met en page : deux branches, un retrait, une action
        // au bout. Sur un seul nœud, rien ne se juge.
        "protocole" => protocol_values(
            "Rupture d'AOD",
            "Anticoagulants oraux directs",
            &[
                crate::db::ProtocolNode {
                    id: 1,
                    parent_id: None,
                    branch: crate::db::Branch::Root,
                    kind: crate::db::NodeKind::Question,
                    text: "Le patient a-t-il une ordonnance en cours ?".to_owned(),
                    position: 0,
                },
                crate::db::ProtocolNode {
                    id: 2,
                    parent_id: Some(1),
                    branch: crate::db::Branch::Yes,
                    kind: crate::db::NodeKind::Question,
                    text: "La molécule prescrite est-elle disponible chez un confrère ?"
                        .to_owned(),
                    position: 0,
                },
                crate::db::ProtocolNode {
                    id: 3,
                    parent_id: Some(2),
                    branch: crate::db::Branch::Yes,
                    kind: crate::db::NodeKind::Action,
                    text: "Dépanner le nombre de jours nécessaires et noter la délivrance."
                        .to_owned(),
                    position: 0,
                },
                crate::db::ProtocolNode {
                    id: 4,
                    parent_id: Some(2),
                    branch: crate::db::Branch::No,
                    kind: crate::db::NodeKind::Action,
                    text: "Appeler le prescripteur : un AOD ne se substitue pas d'une \
                           molécule à l'autre sans son accord."
                        .to_owned(),
                    position: 1,
                },
                crate::db::ProtocolNode {
                    id: 5,
                    parent_id: Some(1),
                    branch: crate::db::Branch::No,
                    kind: crate::db::NodeKind::Action,
                    text: "Pas de délivrance sans ordonnance : orienter vers le médecin \
                           traitant ou la permanence de soins."
                        .to_owned(),
                    position: 1,
                },
            ],
        ),
        "liste" => checklist_values(
            "Ouverture de l'officine",
            "Vérifications avant ouverture",
            &[
                crate::db::ChecklistItem {
                    id: 1,
                    text: "Relever la température du réfrigérateur".to_owned(),
                    note: "Entre +2 et +8 °C, relevé signé".to_owned(),
                    position: 1,
                },
                crate::db::ChecklistItem {
                    id: 2,
                    text: "Compter le fonds de caisse".to_owned(),
                    note: String::new(),
                    position: 2,
                },
            ],
        ),
        "semaine" => week_plan_values(
            &[
                "2026-08-24".to_owned(),
                "2026-08-25".to_owned(),
                "2026-08-26".to_owned(),
                "2026-08-27".to_owned(),
                "2026-08-28".to_owned(),
                "2026-08-29".to_owned(),
                "2026-08-30".to_owned(),
            ],
            // **Une semaine montre une semaine.** Sur un seul
            // rendez-vous, six cases sur sept sont vides et rien ne dit
            // ce que le modèle fait de deux entrées le même jour, ni
            // d'une entrée d'agenda à côté d'un rendez-vous — les deux
            // colonnes que cette feuille porte.
            &[
                Appointment {
                    id: 1,
                    patient_id: 1,
                    patient_name: patient.full_name(),
                    phone: "04 67 00 00 00".to_owned(),
                    kind: InterviewKind::Bpm,
                    date: "2026-08-25".to_owned(),
                    time: "09:30".to_owned(),
                    remote: false,
                    duration_minutes: 30,
                    operator: "CL".to_owned(),
                },
                Appointment {
                    id: 2,
                    patient_id: 2,
                    patient_name: "Claire Martin".to_owned(),
                    phone: "06 12 34 56 78".to_owned(),
                    kind: InterviewKind::Aod,
                    date: "2026-08-25".to_owned(),
                    time: "14:00".to_owned(),
                    remote: true,
                    duration_minutes: 30,
                    operator: "YS".to_owned(),
                },
                Appointment {
                    id: 3,
                    patient_id: 3,
                    patient_name: "Hélène Lefèvre".to_owned(),
                    phone: String::new(),
                    kind: InterviewKind::Asthme,
                    date: "2026-08-27".to_owned(),
                    time: "10:15".to_owned(),
                    remote: false,
                    duration_minutes: 45,
                    operator: "CL".to_owned(),
                },
            ],
            &[crate::db::Event {
                id: 1,
                day: "2026-08-28".to_owned(),
                time: "08:00".to_owned(),
                end_time: "09:00".to_owned(),
                title: "Livraison grossiste".to_owned(),
                category: crate::db::EventCategory::Livraison,
                repeat_days: 0,
                cadence: String::new(),
                source_id: 1,
            }],
            "2026-08-24",
        ),
        "conciliation" => conciliation_values(
            &ConciliationData {
                patient: &patient,
                today: "24/08/2026",
                summary: "3 divergences sur 7 lignes.",
                // **Trois divergences annoncées, trois divergences
                // montrées.** La feuille en comptait trois et n'en
                // listait qu'une : un exemple qui se contredit est
                // celui-là même que l'officine lit pour comprendre le
                // sien. Et les trois statuts que le prescripteur doit
                // distinguer sont là — la ligne que personne n'a pu
                // rapprocher passe devant les autres.
                rows: vec![
                    (
                        "Non rapproché".to_owned(),
                        "Zorglub lyoc".to_owned(),
                        String::new(),
                        "1 le soir".to_owned(),
                        "Ligne non retrouvée dans la base : à vérifier à la main.".to_owned(),
                    ),
                    (
                        "Remplacé".to_owned(),
                        "Coversyl remplacé par Acuitel".to_owned(),
                        "5 mg le matin".to_owned(),
                        "5 mg le matin".to_owned(),
                        "Même classe (IEC).".to_owned(),
                    ),
                    (
                        "Arrêté".to_owned(),
                        "Furosémide 40 mg".to_owned(),
                        "1 le matin".to_owned(),
                        String::new(),
                        "Absent de l'ordonnance de sortie.".to_owned(),
                    ),
                ],
                physician: "Docteur Martin",
                mention: "Document remis à titre informatif.",
                signature: &pharmacy.pharmacist,
            },
            &pharmacy,
        ),
        // **Une liste de comptage montre une liste.** Le comptage sûr
        // est quarante produits ; sur un seul, ni l'empilement, ni les
        // trois motifs — dont le solde négatif, le seul qui soit une
        // erreur et non un rappel —, ni la colonne « Dernier comptage »
        // vide d'un produit jamais compté ne se voient. Et c'est cette
        // page-là que l'officine règle dans l'éditeur de modèles.
        "controle" => stock_check_values(
            &[
                crate::ordonnancier::ToCheck {
                    id: 1,
                    label: "Méthadone AP-HP gélule 40 mg".to_owned(),
                    unit: "gélule".to_owned(),
                    stock: -2.0,
                    days: Some(12),
                    why: crate::ordonnancier::Why::Negative,
                },
                crate::ordonnancier::ToCheck {
                    id: 2,
                    label: "Subutex 8 mg".to_owned(),
                    unit: "comprimé sublingual".to_owned(),
                    stock: 3.0,
                    days: None,
                    why: crate::ordonnancier::Why::Low,
                },
                crate::ordonnancier::ToCheck {
                    id: 3,
                    label: "Skenan LP 30 mg".to_owned(),
                    unit: "gélule".to_owned(),
                    stock: 24.0,
                    days: Some(63),
                    why: crate::ordonnancier::Why::Uncounted,
                },
            ],
            &pharmacy,
            "2026-08-29",
        ),
        // Deux lignes plutôt qu'une, pour la même raison : un bordereau
        // de destruction en porte plusieurs, et l'unité la plus longue
        // du catalogue est ce qui décide de la largeur des colonnes.
        "destruction" => destruction_list_values(
            &[
                crate::ordonnancier::Awaiting {
                    id: 1,
                    label: "Skenan LP 30 mg".to_owned(),
                    unit: "gélule".to_owned(),
                    quantity: 14.0,
                    since: "2026-07-02".to_owned(),
                    days: Some(68),
                },
                crate::ordonnancier::Awaiting {
                    id: 2,
                    label: "Subutex 8 mg".to_owned(),
                    unit: "comprimé sublingual".to_owned(),
                    quantity: 21.0,
                    since: "2026-08-19".to_owned(),
                    days: Some(20),
                },
            ],
            &pharmacy,
            "2026-09-08",
        ),
        "caisse" => {
            let mut q = [0_i64; crate::caisse::DENOMINATIONS.len()];
            q[3] = 4;
            q[4] = 7;
            q[7] = 9;
            q[9] = 5;
            // **Deux lignes hors tiroir, pas une.** Avec une seule, la
            // feuille écrivait « Carte 450,75 » puis « Autres
            // encaissements 450,75 » — la ligne et son total, au même
            // chiffre, ce qui se lit comme un doublon. Deux lignes
            // montrent ce que la somme additionne.
            let others = [
                crate::caisse::Other {
                    label: "Carte".to_owned(),
                    cents: 45_075,
                },
                crate::caisse::Other {
                    label: "Chèques".to_owned(),
                    cents: 12_000,
                },
            ];
            caisse_values(
                &pharmacy.name,
                "24/08/2026",
                "Claire Leroy",
                &q,
                &others,
                &crate::caisse::tally(&q, 15_000, 15_000, &others, Some(77_500)),
                "Un billet de 20 € retrouvé sous le tiroir en fin de comptage.",
            )
        }
        // L'historique : un mois qui porte un recomptage et un soir
        // sans attendu, parce que ce sont les deux cas que la feuille
        // doit savoir écrire.
        "caisses" => {
            let rows = sample_caisse_history();
            let counts: Vec<crate::caisse::Counted> = sample_counted();
            caisse_history_values(
                &rows,
                "septembre 2026",
                &crate::caisse::summarize(&counts),
                &pharmacy.name,
                true,
            )
        }
        // Le planning : une semaine où une personne fait une garde,
        // une autre une absence, et une troisième un poste dont
        // personne n'a noté la fin — les trois cas que la feuille doit
        // savoir écrire, et jamais une grille pleine d'horaires
        // identiques qui ne prouverait rien.
        // La feuille de l'EHPAD : les quatre réponses, parce que ce
        // sont les quatre que la feuille doit savoir écrire — et « à
        // vérifier » est celle qu'on oublie.
        "ecraser" => crush_values(
            &[
                crate::crush::Resolved {
                    treatment: "Moscontin 30 mg".to_owned(),
                    label: "Moscontin",
                    verdict: crate::crush::Verdict::No,
                    why: "Comprimé à libération prolongée : écrasé, il délivre en une fois la dose de douze heures.".to_owned(),
                    instead: "Skenan LP, dont la gélule s'ouvre, à dose recalculée par le prescripteur.".to_owned(),
                    source: "RCP Moscontin",
                },
                crate::crush::Resolved {
                    treatment: "Inexium 20 mg".to_owned(),
                    label: "IPP",
                    verdict: crate::crush::Verdict::Conditional,
                    why: "La gélule s'ouvre et les microgranules se versent dans une compote ; ils ne se croquent pas.".to_owned(),
                    instead: "".to_owned(),
                    source: "RCP ésoméprazole",
                },
                crate::crush::Resolved {
                    treatment: "Doliprane 1 g".to_owned(),
                    label: "Paracétamol",
                    verdict: crate::crush::Verdict::Yes,
                    why: "Le comprimé s'écrase ; la forme effervescente se dissout.".to_owned(),
                    instead: "".to_owned(),
                    source: "RCP paracétamol",
                },
                crate::crush::Resolved {
                    treatment: "Zoltruc 40 mg LP".to_owned(),
                    label: "",
                    verdict: crate::crush::Verdict::Unknown,
                    why: "La table ne connaît pas cette présentation. Lire le RCP avant d'écraser.".to_owned(),
                    instead: "".to_owned(),
                    source: "",
                },
            ],
            &patient.full_name(),
            "24/08/2026",
            &pharmacy.name,
        ),
        // **L'aperçu ne montre pas ce que l'application refuse de
        // faire.** Deux totaux y étaient écrits « 31 h 30 / 35 h 00 » :
        // une semaine contractuelle, c'est-à-dire précisément la notion
        // retirée en 0.185.0 avec `heures_semaine` et le « Relevé
        // d'heures ». Cette colonne compte une **présence**, et rien qui
        // s'en déduise ; un exemple qui promet l'autre chose est un
        // exemple qui la fera demander.
        "planning" => planning_values(
            &["2026-09-07".to_owned(), "2026-09-13".to_owned()],
            &[
                "Lun 07".to_owned(),
                "Mar 08".to_owned(),
                "Mer 09".to_owned(),
                "Jeu 10".to_owned(),
                "Ven 11".to_owned(),
                "Sam 12".to_owned(),
                "Dim 13".to_owned(),
            ],
            &[
                (
                    "CL".to_owned(),
                    vec![
                        "9 h–19 h".to_owned(),
                        "9 h–19 h".to_owned(),
                        "9 h–12 h 30".to_owned(),
                        "Formation".to_owned(),
                        "9 h–19 h".to_owned(),
                        String::new(),
                        String::new(),
                    ],
                    "31 h 30".to_owned(),
                ),
                (
                    "YS".to_owned(),
                    vec![
                        "14 h–19 h 30".to_owned(),
                        String::new(),
                        "20 h–26 h".to_owned(),
                        "14 h–19 h 30".to_owned(),
                        "14 h–19 h 30".to_owned(),
                        "9 h–12 h 30".to_owned(),
                        String::new(),
                    ],
                    "26 h 00".to_owned(),
                ),
                (
                    "MB".to_owned(),
                    vec![
                        String::new(),
                        "9 h–13 h".to_owned(),
                        String::new(),
                        "9 h–…".to_owned(),
                        String::new(),
                        String::new(),
                        String::new(),
                    ],
                    "4 h 00 +1".to_owned(),
                ),
            ],
            "61 h 30",
            &pharmacy.name,
        ),
        "etiquettes" => label_sheet_values(
            &LabelSheet {
                patient: &patient,
                today: "24/08/2026",
                lines: vec![
                    (
                        "Amlodipine 5 mg".to_owned(),
                        "1 comprimé le matin".to_owned(),
                        "Prendre le comprimé oublié dans la journée ; ne jamais doubler la dose le lendemain.".to_owned(),
                    ),
                    (
                        "Lévothyrox 75 µg".to_owned(),
                        "1 comprimé le matin à jeun".to_owned(),
                        String::new(),
                    ),
                ],
                mention: "Document remis à titre informatif.",
            },
            &pharmacy,
        ),
        "surveillance" => watch_sheet_values(
            &WatchSheet {
                patient: &patient,
                today: "24/08/2026",
                rows: vec![
                    (
                        "Créatinine et DFG".to_owned(),
                        "tous les 6 mois".to_owned(),
                        "jamais noté".to_owned(),
                        "Amlodipine, Périndopril".to_owned(),
                    ),
                    (
                        "Kaliémie".to_owned(),
                        "tous les 3 mois".to_owned(),
                        // **À jour, et l'exemple doit le rester.** La
                        // case ne se pose que sur ce qui est en retard
                        // ou jamais fait ; une kaliémie trimestrielle
                        // datée de six mois et sans case montrait
                        // l'inverse de ce que la feuille dit — et ces
                        // valeurs-là sont l'aperçu de l'éditeur de
                        // modèles, c'est-à-dire ce que l'officine lit
                        // pour comprendre sa propre feuille.
                        "12/07/2026".to_owned(),
                        "Périndopril".to_owned(),
                    ),
                ],
                flagged: vec!["Créatinine et DFG".to_owned()],
                mention: "Document remis à titre informatif.",
            },
            &pharmacy,
        ),
        // L'angine : le score se montre, l'âge connu le coche, et un
        // résultat enregistré arrive coché — tout ce que la feuille sait
        // faire, pour qu'on voie où ses propres phrases tomberaient.
        "trod" => trod_values(
            &TrodPaper {
                patient: &patient,
                kind: InterviewKind::TrodAngine,
                date: "24/08/2026",
                age: crate::db::age_on(&patient.birth_date, "2026-08-24"),
                result: crate::ordonnance::POSITIF,
                signature: &pharmacy.pharmacist,
                treats: &sample_treatments(),
                offers: &crate::ordonnance::starter("angine"),
                pregnant: false,
                content: &crate::content::Overrides::default(),
            },
            &pharmacy,
        ),
        // Une dose due et une question : les deux formes de ligne que
        // la section du calendrier sait écrire.
        "vaccin" => vaccination_values(
            &VaccinationPaper {
                patient: &patient,
                date: "24/08/2026",
                age: crate::db::age_on(&patient.birth_date, "2026-08-24"),
                signature: &pharmacy.pharmacist,
                treats: &sample_treatments(),
                due: &crate::vaccines::due_lines_with(&patient.birth_date, "2026-10-15", &[], ""),
                content: &crate::content::Overrides::default(),
            },
            &pharmacy,
        ),
        "trod_cr" => trod_letter_values(
            &TrodPaper {
                patient: &patient,
                kind: InterviewKind::TrodAngine,
                date: "24/08/2026",
                age: crate::db::age_on(&patient.birth_date, "2026-08-24"),
                result: crate::ordonnance::POSITIF,
                signature: &pharmacy.pharmacist,
                treats: &sample_treatments(),
                offers: &[],
                pregnant: false,
                content: &crate::content::Overrides::default(),
            },
            &pharmacy,
        ),
        // L'ordonnance : son aperçu montre les deux mentions remplies,
        // pour qu'on voie où les siennes tomberaient.
        _ => vec![
            ("{{PHARMACY_NAME}}", s(&pharmacy.name)),
            ("{{PHARMACY_ADDRESS}}", s(&pharmacy.address)),
            ("{{PHARMACY_PHONE}}", s(&pharmacy.phone)),
            ("{{PHARMACY_AM}}", s(&pharmacy.am_number)),
            ("{{PATIENT_NAME}}", s(&patient.full_name())),
            (
                "{{BIRTH_DATE}}",
                s(&crate::db::format_french_date(&patient.birth_date)),
            ),
            (
                "{{INDICATION}}",
                s("Angine à streptocoque du groupe A — TROD positif"),
            ),
            ("{{DATE}}", s("26/08/2026")),
            // **Une ordonnance sous protocole porte plus d'une ligne.**
            // L'aperçu n'en montrait qu'une et un seul conseil : ni la
            // numérotation, ni l'adjuvant que le protocole propose, ni
            // la façon dont une posologie longue enveloppe sous son
            // produit ne s'y voyaient — et c'est précisément la mise en
            // page qu'on vient régler ici.
            // Par les fonctions qui impriment, sur des données d'exemple :
            // deux écritures d'une même page divergent, et c'est l'aperçu
            // qui ment.
            (
                "{{LINES}}",
                ordonnance_lines_markup(&[
                    crate::ordonnance::Line {
                        name: "Amoxicilline 1 g".to_owned(),
                        posology: "1 g deux fois par jour pendant 6 jours".to_owned(),
                        caution: String::new(),
                    },
                    crate::ordonnance::Line {
                        name: "Ultra-levure 200 mg".to_owned(),
                        posology: "1 gélule par jour pendant la durée de l'antibiotique, à \
                                   distance d'au moins deux heures de la prise"
                            .to_owned(),
                        caution: String::new(),
                    },
                ]),
            ),
            (
                "{{ADVICE}}",
                ordonnance_advice_markup(&[
                    "Boire fréquemment, par petites quantités.".to_owned(),
                    "La fièvre tombe en deux à trois jours ; l'antibiotique se termine \
                     quand même."
                        .to_owned(),
                    "Consulter sans attendre devant une difficulté à avaler la salive, \
                     une voix étouffée ou un gonflement du cou."
                        .to_owned(),
                ]),
            ),
            (
                "{{MENTION_HEADER}}",
                mention_header("Mention d'en-tête (facultative, [disclaimers] du config.toml)"),
            ),
            (
                "{{MENTION_FOOTER}}",
                mention_footer("Mention de pied (facultative)"),
            ),
            ("{{PHARMACIST}}", s(&pharmacy.pharmacist)),
        ],
    }
}

// ===================================================================
// Le comptage de caisse
// ===================================================================

/// La feuille de comptage : ce qu'on a trouvé, ce qu'on laisse, et
/// l'écart — signé et **jamais résorbé**.
const DEFAULT_CAISSE_TEMPLATE: &str = r##"
#set page(paper: "a4", margin: 1.6cm)
#set text(size: 10.5pt, lang: "fr")

#align(center)[#text(16pt, weight: "bold")[Comptage de caisse]]
#v(1mm)
#align(center)[#text(10pt)[{{PHARMACY_NAME}} — {{DATE}} — {{OPERATOR}}]]
#v(5mm)

#grid(columns: (1fr, 1fr), gutter: 8mm,
  [
    #text(weight: "bold")[Billets]
    #v(1.5mm)
    #table(columns: (1fr, auto, auto), inset: 4pt, stroke: 0.4pt,
      [*Coupure*], [*Nombre*], [*Montant*],
      {{NOTES_ROWS}}
      [*Total billets*], [], [*{{NOTES_TOTAL}} €*],
    )
  ],
  [
    #text(weight: "bold")[Pièces]
    #v(1.5mm)
    #table(columns: (1fr, auto, auto), inset: 4pt, stroke: 0.4pt,
      [*Coupure*], [*Nombre*], [*Montant*],
      {{COINS_ROWS}}
      [*Total pièces*], [], [*{{COINS_TOTAL}} €*],
    )
  ],
)

#v(5mm)
#table(columns: (1fr, auto), inset: 5pt, stroke: 0.4pt,
  [Espèces comptées], [{{CASH}} €],
  [Fond à l'ouverture], [− {{OPENING}} €],
  {{OTHERS}}
  [Autres encaissements], [{{OTHER_TOTAL}} €],
  [*Recette encaissée*], [*{{TAKINGS}} €*],
  [Recette attendue], [{{EXPECTED}}],
  [*Écart*], [*{{GAP}}*],
)

#v(4mm)
#table(columns: (1fr, auto), inset: 5pt, stroke: 0.4pt,
  [Fond de caisse reporté], [{{FLOAT}} €],
  [*Sorti du tiroir*], [*{{BANKED}} €*],
)

{{REMARK}}

#v(10mm)
#grid(columns: (1fr, 1fr), gutter: 10mm,
  [Compté par : #v(9mm) #line(length: 100%, stroke: 0.5pt)],
  [Vérifié par : #v(9mm) #line(length: 100%, stroke: 0.5pt)],
)

#v(4mm)
#text(8.5pt, style: "italic")[Tout écart est noté et justifié, sans modification du comptage.]
"##;

/// Les valeurs de la feuille de caisse.
///
/// Extraite pour que l'aperçu de l'éditeur et l'impression réelle
/// passent par la **même** fonction : deux constructions d'une même
/// page finissent toujours par diverger, et c'est l'aperçu qui ment.
fn caisse_values(
    pharmacy: &str,
    date_french: &str,
    operator: &str,
    quantities: &crate::caisse::Quantities,
    others: &[crate::caisse::Other],
    tally: &crate::caisse::Tally,
    remark: &str,
) -> Vec<(&'static str, String)> {
    use crate::caisse::{euros, DENOMINATIONS};
    // Une coupure qu'on n'a pas trouvée reste sur la feuille, à zéro :
    // la ligne vide est ce qui prouve qu'on l'a regardée.
    let rows = |note: bool| -> String {
        DENOMINATIONS
            .iter()
            .zip(quantities.iter())
            .filter(|(d, _)| d.note == note)
            .map(|(d, q)| {
                let q = (*q).max(0);
                format!(
                    "[#{}], [{q}], [{} €],\n      ",
                    typst_str(d.label),
                    euros(d.cents * q)
                )
            })
            .collect()
    };
    let others_rows: String = others
        .iter()
        .map(|o| format!("[#{}], [{} €],\n  ", typst_str(&o.label), euros(o.cents)))
        .collect();
    // Sans attendu, la ligne reste vide et l'écart aussi : la règle du
    // module, portée jusqu'au papier.
    let expected = tally
        .expected
        .map_or_else(|| "—".to_owned(), |e| format!("{} €", euros(e)));
    let gap = tally.gap.map_or_else(
        || "—".to_owned(),
        |g| {
            let sign = if g > 0 { "+" } else { "" };
            format!("{sign}{} €", euros(g))
        },
    );
    let remark = if remark.trim().is_empty() {
        String::new()
    } else {
        format!(
            "\n#v(4mm)\n#block(width: 100%, stroke: 0.4pt, inset: 6pt)[#text(weight: \"bold\")[Remarque] \\\n#{}]\n",
            typst_str(remark.trim())
        )
    };
    vec![
        ("{{PHARMACY_NAME}}", format!("#{}", typst_str(pharmacy))),
        ("{{DATE}}", format!("#{}", typst_str(date_french))),
        ("{{OPERATOR}}", format!("#{}", typst_str(operator))),
        ("{{NOTES_ROWS}}", rows(true)),
        ("{{COINS_ROWS}}", rows(false)),
        (
            "{{NOTES_TOTAL}}",
            euros(crate::caisse::notes_total(quantities)),
        ),
        (
            "{{COINS_TOTAL}}",
            euros(crate::caisse::coins_total(quantities)),
        ),
        ("{{CASH}}", euros(tally.cash)),
        ("{{OPENING}}", euros(tally.opening)),
        ("{{OTHERS}}", others_rows),
        ("{{OTHER_TOTAL}}", euros(tally.other)),
        ("{{TAKINGS}}", euros(tally.takings)),
        ("{{EXPECTED}}", expected),
        ("{{GAP}}", gap),
        ("{{FLOAT}}", euros(tally.float_kept)),
        ("{{BANKED}}", euros(tally.banked)),
        ("{{REMARK}}", remark),
    ]
}

/// Imprimer le comptage de caisse.
#[allow(clippy::too_many_arguments)]
pub fn open_caisse(
    pharmacy: &PharmacyConfig,
    date_french: &str,
    operator: &str,
    quantities: &crate::caisse::Quantities,
    others: &[crate::caisse::Other],
    tally: &crate::caisse::Tally,
    remark: &str,
    template_path: &std::path::Path,
) -> Result<PathBuf, String> {
    let template = template_source("caisse", template_path);
    let values = caisse_values(
        &pharmacy.name,
        date_french,
        operator,
        quantities,
        others,
        tally,
        remark,
    );
    compile_and_open(fill(&template, &values), "caisse")
}

/// Une soirée de l'historique de caisse, telle que la feuille l'écrit.
///
/// Les montants sont en centimes, comme partout où il est question
/// d'argent ici ; c'est la feuille qui les met en euros, une fois.
pub struct CaisseHistoryRow {
    /// Le jour compté, déjà en français.
    pub day: String,
    pub cash: i64,
    /// Le fond d'ouverture retranché ; `None` quand il n'est pas connu.
    pub opening: Option<i64>,
    pub other: i64,
    pub takings: i64,
    pub expected: Option<i64>,
    pub gap: Option<i64>,
    pub operator: String,
    pub remark: String,
    /// Un comptage qu'un plus récent a remplacé. Il **reste sur la
    /// feuille**, marqué, et ne compte pas dans les totaux : une
    /// histoire de caisse dont on a retiré les comptages refaits ne
    /// prouve rien, et c'est justement le soir qu'on a recompté qui se
    /// relit six mois plus tard.
    pub superseded: bool,
}

const MARKERS_CAISSES: &[&str] = &[
    "{{PHARMACY_NAME}}",
    "{{PERIOD}}",
    "{{ROWS}}",
    "{{DAYS}}",
    "{{TAKINGS}}",
    "{{GAP}}",
    "{{WORST}}",
];

const DEFAULT_CAISSES_TEMPLATE: &str = r##"
#set page(paper: "a4", margin: 1.4cm)
#set text(size: 9pt, lang: "fr", hyphenate: true)

#align(center)[#text(15pt, weight: "bold")[Historique de caisse]]
#v(1mm)
#align(center)[#text(10pt)[{{PHARMACY_NAME}} — {{PERIOD}}]]
#v(4mm)

#table(columns: (auto, auto, auto, auto, auto, auto, auto, auto, 1fr), inset: 4pt, stroke: 0.4pt,
  align: (left, right, right, right, right, right, right, left, left),
  [*Jour*], [*Espèces*], [*Fond ouv.*], [*Autres*], [*Recette*], [*Attendu*], [*Écart*], [*Par*], [*Remarque*],
{{ROWS}})

#v(4mm)
#block(width: 100%, stroke: 0.4pt, inset: 6pt)[
  #text(weight: "bold")[Sur la période] \
  {{DAYS}} \
  Recette encaissée : {{TAKINGS}} \
  {{GAP}} \
  {{WORST}}
]

#v(3mm)
#text(8pt, style: "italic")[Un recomptage ajoute une seconde ligne : les deux figurent ici, seule la dernière entre dans les totaux. Tout écart est noté et justifié, sans modification du comptage.]
"##;

fn caisse_history_values(
    rows: &[CaisseHistoryRow],
    period: &str,
    summary: &crate::caisse::Summary,
    pharmacy: &str,
    want_expected: bool,
) -> Vec<(&'static str, String)> {
    use crate::caisse::euros;
    let money = |cents: Option<i64>| -> String {
        cents.map_or_else(|| "—".to_owned(), |c| format!("{} €", euros(c)))
    };
    let signed = |cents: Option<i64>| -> String {
        cents.map_or_else(
            || "—".to_owned(),
            |c| {
                let sign = if c > 0 { "+" } else { "" };
                format!("{sign}{} €", euros(c))
            },
        )
    };
    let mut body = String::new();
    for r in rows {
        // Le comptage remplacé se nomme plutôt qu'il ne disparaît : il
        // est là pour être lu, pas pour être compté. Le jour en gras
        // est celui qui fait foi.
        let day = if r.superseded {
            format!(
                "[#text(fill: luma(40%))[#{} (recompté)]]",
                typst_str(&r.day)
            )
        } else {
            format!("[*#{}*]", typst_str(&r.day))
        };
        // L'unité sur chaque montant, y compris les trois premiers : une
        // colonne de chiffres nus à côté d'une colonne en euros se lit
        // comme deux choses différentes.
        body.push_str(&format!(
            "  {day}, [{} €], [{}], [{} €], [{} €], [{}], [{}], [#{}], [#text(8pt)[#{}]],\n",
            euros(r.cash),
            r.opening
                .map_or_else(|| "—".to_owned(), |o| format!("- {} €", euros(o))),
            euros(r.other),
            euros(r.takings),
            money(r.expected),
            signed(r.gap),
            typst_str(&r.operator),
            typst_str(r.remark.trim()),
        ));
    }
    if body.is_empty() {
        body.push_str("  [], [], [], [], [], [], [], [], [],\n");
    }
    // Ce que la période dit, et ce qu'elle ne dit pas. Sans attendu
    // saisi, il n'y a pas d'écart : la ligne le dit en toutes lettres
    // plutôt que d'écrire « 0,00 € », qui se lirait « tout est tombé
    // juste ».
    // L'officine qui ne compte pas contre une recette attendue
    // (`[ui] caisse_expected = false`) n'a pas d'écart à lire : la
    // phrase disparaît de la feuille plutôt que d'annoncer qu'aucun
    // attendu n'a été saisi, ce qui parlerait d'un manque là où il y a
    // un choix. Les deux colonnes, elles, restent en tirets : leur
    // en-tête est dans le modèle, et c'est le modèle qu'on modifie pour
    // les retirer du papier — c'est à cela qu'il sert.
    let gap = match (summary.gap, summary.with_expected) {
        _ if !want_expected => String::new(),
        (Some(g), n) => {
            // **Le même moins que dans le tableau, au-dessus.** Les
            // montants des cellules sont posés en markup Typst, qui
            // rend le trait d'union d'un nombre négatif par le vrai
            // signe moins (U+2212) ; cette phrase-ci passe par
            // `typst_str`, donc littéralement, et gardait le trait
            // d'union. Une même page écrivait « −4,75 € » dans la
            // colonne et « -4,75 € » dans le récapitulatif juste
            // dessous.
            //
            // C'est le papier qui prend le signe typographique et
            // l'écran qui garde le tiret : les fontes d'egui n'ont pas
            // toutes U+2212, et `caisse::euros` sert aux deux.
            let sign = if g > 0 { "+" } else { "\u{2212}" };
            format!(
                "Écart cumulé : {sign}{} € — sur {n} {}, {} en moins, {} en plus, {} sans écart.",
                euros(g.abs()),
                if n > 1 { "soirs" } else { "soir" },
                summary.short,
                summary.over,
                summary.exact
            )
        }
        (None, _) => {
            "Écart : non calculable, aucune recette attendue saisie sur la période.".to_owned()
        }
    };
    let worst = match summary.worst.as_ref().filter(|_| want_expected) {
        Some((day, gap)) => {
            // Le même signe moins que le tableau, comme au-dessus.
            let sign = if *gap > 0 { "+" } else { "\u{2212}" };
            format!(
                "Écart maximal : {} ({sign}{} €).",
                crate::db::format_french_date(day),
                euros(gap.abs())
            )
        }
        None => String::new(),
    };
    // Des soirs comptés, jamais des jours de calendrier : un soir où
    // personne n'a compté n'est pas un soir à zéro euro, et une
    // moyenne par jour ouvré se calculerait sur des jours que cette
    // feuille ne connaît pas.
    let counted = format!(
        "{} {}",
        summary.days,
        if summary.days > 1 {
            "soirs comptés"
        } else {
            "soir compté"
        }
    );
    let sentence = |text: &str| -> String {
        if text.is_empty() {
            String::new()
        } else {
            format!("#{}", typst_str(text))
        }
    };
    let days = match summary.counts.saturating_sub(summary.days) {
        0 => format!("{counted}."),
        n => format!(
            "{counted}, sur {} lignes — {n} {}.",
            summary.counts,
            if n > 1 { "recomptages" } else { "recomptage" }
        ),
    };
    vec![
        ("{{PHARMACY_NAME}}", format!("#{}", typst_str(pharmacy))),
        ("{{PERIOD}}", format!("#{}", typst_str(period))),
        ("{{ROWS}}", body),
        ("{{DAYS}}", format!("#{}", typst_str(&days))),
        ("{{TAKINGS}}", format!("{} €", euros(summary.takings))),
        // Une phrase vide sort **vide**, et non `#""` : la ligne du
        // modèle ne doit rien laisser derrière elle, pas même une chaîne
        // qui ne s'imprime pas mais qui garde son saut de ligne.
        ("{{GAP}}", sentence(&gap)),
        ("{{WORST}}", sentence(&worst)),
    ]
}

/// L'historique de caisse d'une période, sur une feuille.
///
/// Ce n'est pas le comptage du soir (voir [`open_caisse`]) : c'est ce
/// que le mois a donné, soir par soir, avec ses écarts et le total de
/// ce qu'ils font. La feuille qu'on garde, ou qu'on donne au
/// comptable.
pub fn open_caisse_history(
    rows: &[CaisseHistoryRow],
    period: &str,
    summary: &crate::caisse::Summary,
    pharmacy: &PharmacyConfig,
    want_expected: bool,
    template_path: &std::path::Path,
) -> Result<PathBuf, String> {
    compile_and_open(
        fill(
            &template_source("caisses", template_path),
            &caisse_history_values(rows, period, summary, &pharmacy.name, want_expected),
        ),
        "historique_caisse",
    )
}

const MARKERS_ECRASER: &[&str] = &["{{PHARMACY_NAME}}", "{{PATIENT}}", "{{TODAY}}", "{{ROWS}}"];

/// La feuille qu'on donne à l'EHPAD ou à l'infirmière : ordonnance en
/// entier, une ligne par traitement, ce qu'on peut en faire.
///
/// **Toutes les lignes, y compris celles que la table ne connaît pas.**
/// Une feuille qui ne montrerait que les interdits se lirait comme une
/// autorisation pour tout le reste, et c'est ainsi qu'on écrase un
/// comprimé à libération prolongée.
const DEFAULT_ECRASER_TEMPLATE: &str = r##"
#set page(paper: "a4", margin: 1.4cm)
#set text(size: 9.5pt, lang: "fr", hyphenate: true)

#align(center)[#text(15pt, weight: "bold")[Écrasement des comprimés]]
#v(1mm)
#align(center)[#text(11pt, weight: "bold")[{{PATIENT}}]]
#align(center)[#text(9pt)[{{PHARMACY_NAME}} — {{TODAY}}]]
#v(4mm)

#table(columns: (auto, auto, 1fr), inset: 5pt, stroke: 0.4pt,
  align: (left, left, left),
  [*Traitement*], [*Réponse*], [*Motif et alternative*],
{{ROWS}})

#v(4mm)
#block(width: 100%, stroke: 0.4pt, inset: 6pt)[
  #text(weight: "bold")[Précautions.]   Administrer *aussitôt* après écrasement : broyé à l'avance, le principe actif s'oxyde.   Laver le mortier entre deux traitements pour éviter toute contamination croisée.   « À vérifier » n'est pas une autorisation : consulter le résumé des caractéristiques du produit.
]

#v(3mm)
#text(8pt, style: "italic")[Cette feuille reprend les résumés des caractéristiques des produits et la liste nationale des médicaments écrasables. Elle ne remplace pas l'avis du prescripteur : un traitement non écrasable se remplace, sur son accord.]
"##;

const MARKERS_PLANNING: &[&str] = &[
    "{{PHARMACY_NAME}}",
    "{{PERIOD}}",
    "{{HEADS}}",
    "{{ROWS}}",
    "{{TOTAL}}",
];

/// La semaine affichée, une ligne par personne : la feuille qu'on
/// punaise en arrière-boutique.
const DEFAULT_PLANNING_TEMPLATE: &str = r##"
#set page(paper: "a4", flipped: true, margin: 1.2cm)
#set text(size: 9pt, lang: "fr", hyphenate: true)

#align(center)[#text(15pt, weight: "bold")[Planning de l'équipe]]
#v(1mm)
#align(center)[#text(10pt)[{{PHARMACY_NAME}} — {{PERIOD}}]]
#v(4mm)

#table(columns: (auto, 1fr, 1fr, 1fr, 1fr, 1fr, 1fr, 1fr, auto), inset: 4pt, stroke: 0.4pt,
  align: (left, left, left, left, left, left, left, left, right),
  {{HEADS}}
{{ROWS}})

#v(3mm)
#text(9pt)[Total de la semaine : *{{TOTAL}}*]

#v(3mm)
#text(8pt, style: "italic")[Un poste dont la fin n'a pas été notée s'écrit « 9 h–… » et n'entre pas dans le total : « 34 h 15 +1 » signifie 34 h 15 et un poste sans heure de fin. Un total « — » signifie qu'aucune heure n'est connue, et non zéro heure. Une garde qui franchit minuit est comptée en entier au jour qui la commence.]
"##;

fn crush_values(
    rows: &[crate::crush::Resolved],
    patient: &str,
    today: &str,
    pharmacy: &str,
) -> Vec<(&'static str, String)> {
    let mut body = String::new();
    for r in rows {
        // Le verdict en gras, et la raison avec le remplaçant à la
        // suite : sur une feuille lue debout, une colonne « pourquoi »
        // vide est une consigne qu'on ne suivra pas.
        let mut why = r.why.to_owned();
        if !r.instead.trim().is_empty() {
            why.push_str(" — À la place : ");
            why.push_str(&r.instead);
        }
        body.push_str(&format!(
            "  [#{}], [*#{}*], [#{}],
",
            typst_str(&r.treatment),
            typst_str(r.verdict.label()),
            typst_str(&why),
        ));
    }
    vec![
        ("{{PHARMACY_NAME}}", format!("#{}", typst_str(pharmacy))),
        ("{{PATIENT}}", format!("#{}", typst_str(patient))),
        ("{{TODAY}}", format!("#{}", typst_str(today))),
        ("{{ROWS}}", body),
    ]
}

/// La feuille « peut-on écraser ? » pour un dossier.
pub fn open_crush(
    rows: &[crate::crush::Resolved],
    patient: &Patient,
    today: &str,
    pharmacy: &PharmacyConfig,
    template_path: &std::path::Path,
) -> Result<PathBuf, String> {
    compile_and_open(
        fill(
            &template_source("ecraser", template_path),
            &crush_values(rows, &patient.full_name(), today, &pharmacy.name),
        ),
        &format!("ecraser_{}", patient.id),
    )
}

fn planning_values(
    week: &[String],
    heads: &[String],
    rows: &[(String, Vec<String>, String)],
    total: &str,
    pharmacy: &str,
) -> Vec<(&'static str, String)> {
    // **Les dates s'écrivent à la française sur le papier.** Le
    // `week` arrive en ISO, comme tout ce que la base stocke, et le
    // sous-titre le recopiait tel quel : « semaine du 2026-09-07 au
    // 2026-09-13 » sur une feuille où toutes les autres dates sont en
    // jj/mm/aaaa.
    let period = match (week.first(), week.last()) {
        (Some(a), Some(b)) => format!(
            "semaine du {} au {}",
            crate::db::format_french_date(a),
            crate::db::format_french_date(b)
        ),
        _ => String::new(),
    };
    // Les valeurs sont posées en littéraux Typst (`#"…"`), comme
    // partout ici : une initiale contenant du balisage ne peut ni
    // casser la compilation ni restyler la feuille.
    let mut head = String::from("[*Personne*], ");
    for h in heads {
        head.push_str(&format!("[*#{}*], ", typst_str(h)));
    }
    head.push_str("[*Total*],");
    let mut body = String::new();
    for (who, days, total) in rows {
        body.push_str(&format!("  [*#{}*], ", typst_str(who)));
        for d in days {
            // Une case vide reste une case : la table doit garder ses
            // colonnes, sans quoi la ligne suivante glisse d'un jour.
            body.push_str(&format!("[#{}], ", typst_str(d)));
        }
        body.push_str(&format!("[#{}],\n", typst_str(total)));
    }
    vec![
        ("{{PHARMACY_NAME}}", format!("#{}", typst_str(pharmacy))),
        ("{{PERIOD}}", format!("#{}", typst_str(&period))),
        ("{{HEADS}}", head),
        ("{{ROWS}}", body),
        ("{{TOTAL}}", format!("#{}", typst_str(total))),
    ]
}

/// La semaine affichée, à punaiser en arrière-boutique.
pub fn open_planning(
    week: &[String],
    heads: &[String],
    rows: &[(String, Vec<String>, String)],
    total: &str,
    pharmacy: &PharmacyConfig,
    template_path: &std::path::Path,
) -> Result<PathBuf, String> {
    compile_and_open(
        fill(
            &template_source("planning", template_path),
            &planning_values(week, heads, rows, total, &pharmacy.name),
        ),
        "planning",
    )
}

/// Ce qui va sur les boîtes : une étiquette par traitement.
pub struct LabelSheet<'a> {
    pub patient: &'a Patient,
    /// Déjà en français.
    pub today: &'a str,
    /// (nom et dosage, la posologie du dossier, que faire en cas
    /// d'oubli) — une entrée par traitement.
    pub lines: Vec<(String, String, String)>,
    /// La mention de l'officine, vide tant qu'elle n'en a pas écrit.
    pub mention: &'a str,
}

const MARKERS_ETIQUETTES: &[&str] = &[
    "{{PATIENT_NAME}}",
    "{{DATE}}",
    "{{LABELS}}",
    "{{PHARMACY_NAME}}",
    "{{MENTION}}",
];

const DEFAULT_ETIQUETTES_TEMPLATE: &str = r##"
#set page(paper: "a4", margin: 1cm)
#set text(size: 10pt, lang: "fr", hyphenate: true)

#text(9pt)[{{PATIENT_NAME}} — {{DATE}} — {{PHARMACY_NAME}}]
#v(2mm)

#grid(columns: (1fr, 1fr), rows: 3.3cm, gutter: 3mm,
{{LABELS}})

#v(2mm)
{{MENTION}}
"##;

fn label_sheet_values(data: &LabelSheet, pharmacy: &PharmacyConfig) -> Vec<(&'static str, String)> {
    // Ce qu'une étiquette de 8 cm sur 3 peut porter sans devenir
    // illisible. On coupe à la lecture, jamais au sens : le nom et la
    // posologie passent en entier, c'est la phrase de l'oubli qui
    // s'arrête — et la feuille de plan de prise, elle, la porte entière.
    let short = |text: &str, max: usize| -> String {
        let text = text.trim();
        if text.chars().count() <= max {
            return text.to_owned();
        }
        let cut: String = text.chars().take(max).collect();
        let cut = cut
            .rsplit_once(' ')
            .map_or(cut.clone(), |(a, _)| a.to_owned());
        format!("{cut}…")
    };
    let mut body = String::new();
    for (name, dose, missed) in &data.lines {
        let missed = short(missed, 110);
        let missed = if missed.is_empty() {
            String::new()
        } else {
            format!(
                "\n    #v(1mm)\n    #text(7.5pt, style: \"italic\")[Oubli : #{}]",
                typst_str(&missed)
            )
        };
        // **La posologie passe en entier, quoi qu'il en coûte en
        // corps.** Elle était coupée à soixante-dix caractères avec
        // trois points, alors que le commentaire au-dessus de cette
        // fonction promet le contraire depuis toujours — et une
        // posologie coupée sur une étiquette collée à la boîte est la
        // moitié dangereuse : « 1 comprimé le matin et 1 le soir
        // pendant 7 jours, puis 1 comprimé le… ». On coupe à la
        // lecture, jamais au sens : c'est le corps qui cède, d'un cran,
        // et la phrase de l'oubli qui s'arrête.
        let dose = dose.trim();
        let dose_pt = if dose.chars().count() <= 70 {
            "9.5pt"
        } else {
            "8pt"
        };
        body.push_str(&format!(
            "  box(width: 100%, height: 100%, stroke: 0.5pt, inset: 5pt, clip: true)[\n    #text(11pt, weight: \"bold\")[#{}] \\\n    #text({dose_pt})[#{}]{}\n    #place(bottom + left)[#text(6.5pt)[#{} — #{}]]\n  ],\n",
            typst_str(name.trim()),
            typst_str(dose),
            missed,
            typst_str(&data.patient.full_name()),
            typst_str(data.today),
        ));
    }
    if body.is_empty() {
        body.push_str("  box(width: 100%, height: 100%, stroke: 0.5pt)[],\n");
    }
    let mention = if data.mention.trim().is_empty() {
        String::new()
    } else {
        format!(
            "#text(7.5pt, style: \"italic\")[#{}]",
            typst_str(data.mention.trim())
        )
    };
    vec![
        (
            "{{PATIENT_NAME}}",
            format!("#{}", typst_str(&data.patient.full_name())),
        ),
        ("{{DATE}}", format!("#{}", typst_str(data.today))),
        ("{{LABELS}}", body),
        (
            "{{PHARMACY_NAME}}",
            format!("#{}", typst_str(&pharmacy.name)),
        ),
        ("{{MENTION}}", mention),
    ]
}

/// Les étiquettes de posologie, à coller sur les boîtes.
///
/// C'est le plan de prise découpé : la même posologie, la même phrase
/// d'oubli, mais sur la boîte plutôt que sur une feuille qui reste dans
/// un tiroir. Ce que le pilulier de l'EHPAD et la table de nuit
/// demandent, et ce qu'aucune feuille A4 ne remplace.
///
/// La mention imprimée est celle du plan de prise : c'est le même
/// document, coupé autrement, et il n'y a pas lieu d'en écrire une
/// seconde.
pub fn open_labels(
    data: &LabelSheet,
    pharmacy: &PharmacyConfig,
    template_path: &std::path::Path,
) -> Result<PathBuf, String> {
    compile_and_open(
        fill(
            &template_source("etiquettes", template_path),
            &label_sheet_values(data, pharmacy),
        ),
        "etiquettes",
    )
}

/// Le plan de surveillance : ce que l'ordonnance demande de faire
/// doser, et quand.
pub struct WatchSheet<'a> {
    pub patient: &'a Patient,
    /// Déjà en français.
    pub today: &'a str,
    /// (analyse, rythme, dernier résultat noté, ce qui la demande) —
    /// dans l'ordre où la vue les montre : les retards d'abord.
    pub rows: Vec<(String, String, String, String)>,
    /// Les analyses en retard ou jamais faites, par leur libellé : la
    /// feuille les coche.
    pub flagged: Vec<String>,
    pub mention: &'a str,
}

const MARKERS_SURVEILLANCE: &[&str] = &[
    "{{PATIENT_NAME}}",
    "{{BIRTH_DATE}}",
    "{{DATE}}",
    "{{ROWS}}",
    "{{PHARMACY_NAME}}",
    "{{MENTION}}",
];

const DEFAULT_SURVEILLANCE_TEMPLATE: &str = r##"
#set page(paper: "a4", margin: 1.5cm)
#set text(size: 10pt, lang: "fr", hyphenate: true)

#align(center)[#text(15pt, weight: "bold")[Plan de surveillance]]
#v(1mm)
#align(center)[#text(10pt)[{{PATIENT_NAME}} — né(e) le {{BIRTH_DATE}}]]
#v(4mm)

#table(columns: (auto, 1.1fr, auto, auto, 1.4fr, auto), inset: 5pt, stroke: 0.5pt,
  align: (center, left, left, left, left, left),
  [], [*Analyse*], [*Rythme*], [*Dernier*], [*Motif*], [*Résultat*],
{{ROWS}})

#v(5mm)
#text(9pt)[Feuille établie le {{DATE}} par {{PHARMACY_NAME}} d'après les traitements portés au dossier. Elle ne remplace ni la prescription du médecin ni le compte rendu du laboratoire.]
#v(2mm)
{{MENTION}}
"##;

fn watch_sheet_values(data: &WatchSheet, pharmacy: &PharmacyConfig) -> Vec<(&'static str, String)> {
    let mut body = String::new();
    for (analyte, rhythm, last, why) in &data.rows {
        // La case à cocher n'est pas sur toutes les lignes : elle est
        // sur celles qui sont en retard ou jamais faites. Une feuille
        // où tout est à cocher ne dit plus ce qui presse.
        let mark = if data.flagged.contains(analyte) {
            "[#box(width: 4mm, height: 4mm, stroke: 0.6pt)]"
        } else {
            "[]"
        };
        body.push_str(&format!(
            "  {mark}, [*#{}*], [#{}], [#{}], [#text(8.5pt)[#{}]], [],\n",
            typst_str(analyte),
            typst_str(rhythm),
            typst_str(last),
            typst_str(why),
        ));
    }
    if body.is_empty() {
        body.push_str("  [], [], [], [], [], [],\n");
    }
    let mention = if data.mention.trim().is_empty() {
        String::new()
    } else {
        format!(
            "#text(8.5pt, style: \"italic\")[#{}]",
            typst_str(data.mention.trim())
        )
    };
    vec![
        (
            "{{PATIENT_NAME}}",
            format!("#{}", typst_str(&data.patient.full_name())),
        ),
        (
            "{{BIRTH_DATE}}",
            format!(
                "#{}",
                typst_str(&crate::db::format_french_date(&data.patient.birth_date))
            ),
        ),
        ("{{DATE}}", format!("#{}", typst_str(data.today))),
        ("{{ROWS}}", body),
        (
            "{{PHARMACY_NAME}}",
            format!("#{}", typst_str(&pharmacy.name)),
        ),
        ("{{MENTION}}", mention),
    ]
}

/// Le plan de surveillance du dossier, sur une feuille.
///
/// Ce que `surveillance.rs` sait — quelle analyse chaque traitement
/// réclame, à quel rythme, et ce qui n'a pas été fait — n'atteignait
/// jusqu'ici que l'écran et une section du bilan. C'est pourtant la
/// feuille qu'on emporte au laboratoire ou chez le médecin.
///
/// **Aucun chiffre n'y figure**, et c'est délibéré : la feuille dit
/// quoi faire doser et à quel rythme, jamais ce que le résultat devrait
/// valoir. Une norme imprimée sur une feuille qui part à la maison est
/// une invitation à s'interpréter soi-même.
pub fn open_watch_sheet(
    data: &WatchSheet,
    pharmacy: &PharmacyConfig,
    template_path: &std::path::Path,
) -> Result<PathBuf, String> {
    compile_and_open(
        fill(
            &template_source("surveillance", template_path),
            &watch_sheet_values(data, pharmacy),
        ),
        "surveillance",
    )
}

/// Ce que la feuille d'un TROD reçoit : la personne, l'acte tel qu'il
/// est enregistré, et les lignes du protocole que l'officine tient.
pub struct TrodPaper<'a> {
    pub patient: &'a Patient,
    pub kind: InterviewKind,
    /// La date imprimée, déjà au format `JJ/MM/AAAA`.
    pub date: &'a str,
    /// L'âge à la date du test, quand la naissance est connue.
    pub age: Option<u32>,
    /// Ce que l'acte a enregistré : `POSITIF`, `NEGATIF` ou vide.
    pub result: &'a str,
    pub signature: &'a str,
    pub treats: &'a [Drug],
    /// Les lignes du protocole, telles que la base les tient.
    pub offers: &'a [crate::ordonnance::Offer],
    pub pregnant: bool,
    pub content: &'a crate::content::Overrides,
}

const MARKERS_TROD: &[&str] = &[
    "{{PHARMACY_NAME}}",
    "{{PHARMACY_PHONE}}",
    "{{TITLE}}",
    "{{PATIENT_NAME}}",
    "{{BIRTH_DATE}}",
    "{{AGE}}",
    "{{SEX}}",
    "{{DATE}}",
    "{{SIGNS}}",
    "{{SCORE}}",
    "{{READINGS}}",
    "{{RESULT}}",
    "{{POSITIVE}}",
    "{{NEGATIVE}}",
    "{{LINES}}",
    "{{TREATMENTS}}",
    "{{PHARMACIST}}",
];

/// La feuille d'un TROD, A4 : ce qui oriente sans tester, le score,
/// la lecture et sa traçabilité, la conduite selon le résultat.
///
/// Les cases sont des cadres vides — ce qui se constate devant le
/// patient se coche à la main. Seuls l'âge connu et le résultat déjà
/// enregistré arrivent cochés, en plein.
const DEFAULT_TROD_TEMPLATE: &str = r##"
#set page(paper: "a4", margin: (x: 1.5cm, y: 1.3cm))
#set text(size: 10pt, lang: "fr")
#set block(spacing: 2mm)

#let sec(title) = block(sticky: true, above: 4mm, below: 2mm)[
  #text(weight: "bold", size: 10.5pt)[#title]
  #v(-1.5mm)
  #line(length: 100%, stroke: 0.4pt)
]
#let blank(w) = box(width: w, height: 0.9em, stroke: (bottom: 0.5pt))

#grid(columns: (1fr, auto),
  [#text(weight: "bold")[{{PHARMACY_NAME}}] \ #text(size: 9pt)[{{PHARMACY_PHONE}}]],
  [#align(right)[Le {{DATE}}]],
)
#v(3mm)
#align(center)[#text(14pt, weight: "bold")[{{TITLE}}]]
#v(2mm)
#box(width: 100%, stroke: 0.7pt, inset: 7pt)[
  #grid(columns: (1fr, 1fr), row-gutter: 1.5mm,
    [*Patient :* {{PATIENT_NAME}}], [*Né(e) le :* {{BIRTH_DATE}} ({{AGE}})],
    [*Sexe :* {{SEX}}], [*Pharmacien :* {{PHARMACIST}}],
  )
]

#sec[Signes qui orientent vers le médecin, sans tester]
{{SIGNS}}

{{SCORE}}

#sec[Test]
#grid(columns: (1fr, 1fr), row-gutter: 3mm, column-gutter: 6mm,
  [Test utilisé : #blank(1fr)], [N° de lot : #blank(1fr)],
  [Péremption : #blank(1fr)], [Heure de lecture : #blank(1fr)],
)
#v(2mm)
{{READINGS}}
#v(1mm)
{{RESULT}}

#sec[Conduite]
#grid(columns: (1fr, 1fr), column-gutter: 6mm,
  [#text(weight: "bold")[Test positif] #v(1mm) {{POSITIVE}}],
  [#text(weight: "bold")[Test négatif] #v(1mm) {{NEGATIVE}}],
)

#sec[Lignes du protocole pour ce patient]
{{LINES}}

#sec[Traitements connus à l'officine]
{{TREATMENTS}}

#v(1fr)
#grid(columns: (1fr, 1fr), column-gutter: 6mm,
  [#text(weight: "bold")[Observations]
   #v(1mm)
   #box(width: 100%, height: 2cm, stroke: 0.7pt)],
  [#text(weight: "bold")[Signature du pharmacien]
   #v(1mm)
   #box(width: 100%, height: 2cm, stroke: 0.7pt)],
)
"##;

/// Une case : vide, ou pleine quand le logiciel sait déjà.
fn tick_box(done: bool) -> String {
    if done {
        "#box(width: 3.2mm, height: 3.2mm, stroke: 0.7pt, inset: 0.7mm)[#box(width: 100%, height: 100%, fill: black)]".to_owned()
    } else {
        "#box(width: 3.2mm, height: 3.2mm, stroke: 0.7pt)".to_owned()
    }
}

/// Une liste de cases, sur deux colonnes quand elle est longue — une
/// feuille de test se tient sur une page.
fn tick_list(items: &[String], columns: usize) -> String {
    if items.is_empty() {
        return String::new();
    }
    let cells: Vec<String> = items
        .iter()
        // La case à part et le texte dans sa propre colonne : une ligne
        // qui passe à la suivante reprend sous le texte, pas sous la case.
        .map(|p| {
            format!(
                "grid(columns: (auto, 1fr), column-gutter: 1.5mm, [{}], [#{}])",
                tick_box(false),
                typst_str(p)
            )
        })
        .collect();
    format!(
        "#grid(columns: ({}), row-gutter: 1.8mm, column-gutter: 6mm, {})",
        vec!["1fr"; columns.max(1)].join(", "),
        cells.join(", ")
    )
}

/// Les valeurs de la feuille d'un TROD. Séparées de l'ouverture pour
/// que l'aperçu de l'éditeur remplisse **la même** feuille.
fn trod_values(p: &TrodPaper, pharmacy: &PharmacyConfig) -> Vec<(&'static str, String)> {
    let s = |v: &str| format!("#{}", typst_str(v));
    let Some(sheet) = crate::trod::sheet(p.kind) else {
        return Vec::new();
    };
    let filled = crate::trod::fill(sheet, p.content, p.age);
    let known = crate::trod::result(p.result);
    let score = if filled.score.is_empty() {
        String::new()
    } else {
        let rows: Vec<String> = filled
            .score
            .iter()
            .map(|(label, points, ticked)| {
                let points = if *points > 0 {
                    format!("+{points}")
                } else {
                    // Le moins typographique : c'est ce que le papier
                    // porte ailleurs (les cellules de la caisse).
                    format!("\u{2212}{}", points.unsigned_abs())
                };
                format!(
                    "{}, [#{}], [#{}]",
                    format_args!("[{}]", tick_box(*ticked)),
                    typst_str(label),
                    typst_str(&points)
                )
            })
            .collect();
        format!(
            "#block(above: 4mm, below: 2mm)[#text(weight: \"bold\", size: 10.5pt)[#{}] #v(-1.5mm) #line(length: 100%, stroke: 0.4pt)]\n\
             #grid(columns: (auto, 9cm, auto), row-gutter: 1.8mm, column-gutter: 2mm, {}, [], [#text(weight: \"bold\")[#{}]], [#box(width: 8mm, height: 0.9em, stroke: (bottom: 0.5pt))])\n\
             #v(1mm)\n#text(size: 9pt)[#{}]",
            typst_str(crate::strings::tr("trod_pdf_score")),
            rows.join(", "),
            typst_str(crate::strings::tr("trod_pdf_total")),
            typst_str(&filled.score_rule),
        )
    };
    let readings = filled
        .readings
        .iter()
        .map(|r| {
            format!(
                "#grid(columns: (1fr, auto, auto, auto), column-gutter: 5mm, [#{}], [{} #h(1mm) #{}], [{} #h(1mm) #{}], [{} #h(1mm) #{}])",
                typst_str(r),
                tick_box(false),
                typst_str(crate::strings::tr("trod_pdf_positive")),
                tick_box(false),
                typst_str(crate::strings::tr("trod_pdf_negative")),
                tick_box(false),
                typst_str(crate::strings::tr("trod_pdf_invalid")),
            )
        })
        .collect::<Vec<_>>()
        .join("\n#v(1.5mm)\n");
    let result = format!(
        "#text(weight: \"bold\")[#{}] #h(3mm) {} #h(1mm) #{} #h(5mm) {} #h(1mm) #{}",
        typst_str(crate::strings::tr("trod_pdf_result")),
        tick_box(known == Some(true)),
        typst_str(crate::strings::tr("trod_pdf_positive")),
        tick_box(known == Some(false)),
        typst_str(crate::strings::tr("trod_pdf_negative")),
    );
    let who = crate::ordonnance::Who {
        age: p.age,
        sex: p.patient.known_sex(),
        pregnant: p.pregnant,
    };
    let fits: Vec<&crate::ordonnance::Offer> = p
        .offers
        .iter()
        .filter(|o| o.protocol == sheet.protocol && o.barrier(&who).is_none())
        .collect();
    let lines = if fits.is_empty() {
        format!("#{}", typst_str(crate::strings::tr("trod_pdf_no_line")))
    } else {
        fits.iter()
            .map(|o| {
                let mut line = o.name.clone();
                if !o.situation.trim().is_empty() {
                    line.push_str(&format!(" — {}", o.situation.trim()));
                }
                if let Some(first) = o.posologies.first() {
                    line.push_str(&format!(" : {first}"));
                }
                format!("- #{}", typst_str(&line))
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let sex = match p.patient.known_sex() {
        Some(crate::ordonnance::Sex::F) => "F",
        Some(crate::ordonnance::Sex::M) => "M",
        None => "—",
    };
    let age = p.age.map_or_else(
        || crate::strings::tr("trod_pdf_no_age").to_owned(),
        |a| format!("{a} ans"),
    );
    let birth = if p.patient.birth_date.trim().is_empty() {
        "—".to_owned()
    } else {
        crate::db::format_french_date(&p.patient.birth_date)
    };
    vec![
        ("{{PHARMACY_NAME}}", s(&pharmacy.name)),
        ("{{PHARMACY_PHONE}}", s(&pharmacy.phone)),
        ("{{TITLE}}", s(&filled.title)),
        ("{{PATIENT_NAME}}", s(&p.patient.full_name())),
        ("{{BIRTH_DATE}}", s(&birth)),
        ("{{AGE}}", s(&age)),
        ("{{SEX}}", s(sex)),
        ("{{DATE}}", s(p.date)),
        ("{{SIGNS}}", tick_list(&filled.signs, 2)),
        ("{{SCORE}}", score),
        ("{{READINGS}}", readings),
        ("{{RESULT}}", result),
        ("{{POSITIVE}}", tick_list(&filled.positive, 1)),
        ("{{NEGATIVE}}", tick_list(&filled.negative, 1)),
        ("{{LINES}}", lines),
        ("{{TREATMENTS}}", treatments_markup(p.treats)),
        ("{{PHARMACIST}}", s(p.signature)),
    ]
}

const MARKERS_TROD_CR: &[&str] = &[
    "{{PHARMACY_NAME}}",
    "{{PHARMACY_ADDRESS}}",
    "{{PHARMACY_PHONE}}",
    "{{PHYSICIAN}}",
    "{{DATE}}",
    "{{KIND}}",
    "{{PATIENT_NAME}}",
    "{{BIRTH_DATE}}",
    "{{TEST}}",
    "{{RESULT}}",
    "{{CONCLUSION}}",
    "{{TREATMENTS}}",
    "{{PHARMACIST}}",
];

/// Le courrier au médecin traitant après un TROD : le test, son
/// résultat, ce qui a été fait. **Le protocole demande de l'informer** ;
/// le courrier d'accompagnement générique parlait d'« accompagnement à
/// l'officine », ce qu'un test rapide n'est pas.
///
/// Sans résultat enregistré, la conclusion reste à écrire à la main :
/// le courrier ne suppose pas un résultat qu'on n'a pas lu.
const DEFAULT_TROD_CR_TEMPLATE: &str = r#"
#set page(paper: "a4", margin: 2cm)
#set text(size: 11pt, lang: "fr")

#grid(columns: (1fr, auto),
  [#text(weight: "bold", size: 13pt)[{{PHARMACY_NAME}}] \
   {{PHARMACY_ADDRESS}} \
   {{PHARMACY_PHONE}}],
  [#align(right)[À l'attention du \ #text(weight: "bold")[{{PHYSICIAN}}]]],
)
#v(8mm)
#align(right)[Le {{DATE}}]
#v(4mm)
#text(weight: "bold")[Objet : {{KIND}} — {{PATIENT_NAME}} (né(e) le {{BIRTH_DATE}})]
#v(4mm)
Docteur,

{{TEST}}

#text(weight: "bold")[Résultat :] {{RESULT}}

{{CONCLUSION}}

#v(2mm)
#text(weight: "bold")[Traitements connus à l'officine :]

{{TREATMENTS}}

#v(2mm)
#text(weight: "bold")[Observations :]
#v(1mm)
#box(width: 100%, height: 3cm, stroke: 0.7pt)

#v(1fr)
Restant à votre disposition, nous vous prions d'agréer, Docteur,
l'expression de nos salutations confraternelles.

#align(right)[{{PHARMACIST}}
#v(2mm)
#box(width: 6.5cm, height: 2.2cm, stroke: 0.7pt)]
"#;

/// Les valeurs du courrier d'un TROD — celles de l'impression et celles
/// de l'aperçu, par la même fonction.
fn trod_letter_values(p: &TrodPaper, pharmacy: &PharmacyConfig) -> Vec<(&'static str, String)> {
    let s = |v: &str| format!("#{}", typst_str(v));
    let Some(sheet) = crate::trod::sheet(p.kind) else {
        return Vec::new();
    };
    let filled = crate::trod::fill(sheet, p.content, p.age);
    let known = crate::trod::result(p.result);
    let physician = if p.patient.physician.trim().is_empty() {
        crate::strings::tr("trod_letter_physician")
    } else {
        p.patient.physician.trim()
    };
    let result = match known {
        Some(true) => crate::strings::tr("trod_letter_positive"),
        Some(false) => crate::strings::tr("trod_letter_negative"),
        None => crate::strings::tr("trod_letter_unread"),
    };
    let conclusion = match known {
        Some(true) => s(&filled.letter_positive),
        Some(false) => s(&filled.letter_negative),
        // Rien de lu : des lignes à remplir, pas une conclusion écrite
        // d'avance.
        None => "#box(width: 100%, height: 2cm, stroke: (bottom: 0.5pt))".to_owned(),
    };
    let birth = if p.patient.birth_date.trim().is_empty() {
        "—".to_owned()
    } else {
        crate::db::format_french_date(&p.patient.birth_date)
    };
    vec![
        ("{{PHARMACY_NAME}}", s(&pharmacy.name)),
        ("{{PHARMACY_ADDRESS}}", s(&pharmacy.address)),
        ("{{PHARMACY_PHONE}}", s(&pharmacy.phone)),
        ("{{PHYSICIAN}}", s(physician)),
        ("{{DATE}}", s(p.date)),
        ("{{KIND}}", s(p.kind.label())),
        ("{{PATIENT_NAME}}", s(&p.patient.full_name())),
        ("{{BIRTH_DATE}}", s(&birth)),
        ("{{TEST}}", s(&filled.letter_test)),
        (
            "{{RESULT}}",
            format!("#text(weight: \"bold\")[{}]", s(result)),
        ),
        ("{{CONCLUSION}}", conclusion),
        ("{{TREATMENTS}}", treatments_markup(p.treats)),
        ("{{PHARMACIST}}", s(p.signature)),
    ]
}

/// Le courrier au médecin traitant après un TROD. Voir
/// [`DEFAULT_TROD_CR_TEMPLATE`].
pub fn open_trod_letter(
    paper: &TrodPaper,
    pharmacy: &PharmacyConfig,
    template_path: &std::path::Path,
) -> Result<PathBuf, String> {
    compile_and_open(
        trod_letter_source(paper, pharmacy, template_path),
        &format!("courrier_trod_{}", paper.patient.id),
    )
}

/// Le courrier d'un TROD, rempli mais pas encore compilé — pour la
/// liasse.
pub fn trod_letter_source(
    paper: &TrodPaper,
    pharmacy: &PharmacyConfig,
    template_path: &std::path::Path,
) -> String {
    fill(
        &template_source("trod_cr", template_path),
        &trod_letter_values(paper, pharmacy),
    )
}

/// Ce que la fiche de vaccination reçoit.
pub struct VaccinationPaper<'a> {
    pub patient: &'a Patient,
    /// La date imprimée, déjà au format `JJ/MM/AAAA`.
    pub date: &'a str,
    pub age: Option<u32>,
    pub signature: &'a str,
    pub treats: &'a [Drug],
    /// Ce que le calendrier doit encore, lu contre le carnet.
    pub due: &'a [crate::vaccines::DueLine],
    pub content: &'a crate::content::Overrides,
}

const MARKERS_VACCIN: &[&str] = &[
    "{{PHARMACY_NAME}}",
    "{{PHARMACY_PHONE}}",
    "{{DATE}}",
    "{{PATIENT_NAME}}",
    "{{BIRTH_DATE}}",
    "{{AGE}}",
    "{{SEX}}",
    "{{PHARMACIST}}",
    "{{QUESTIONS}}",
    "{{DUE}}",
    "{{AFTER}}",
    "{{TREATMENTS}}",
];

/// La fiche d'une vaccination à l'officine, A4 : les questions avant
/// l'injection, le vaccin tracé (nom, lot, péremption, voie, site,
/// heure), les suites, et ce que le calendrier doit encore d'après le
/// carnet.
const DEFAULT_VACCIN_TEMPLATE: &str = r##"
#set page(paper: "a4", margin: (x: 1.5cm, y: 1.3cm))
#set text(size: 10pt, lang: "fr")
#set block(spacing: 2mm)

#let sec(title) = block(sticky: true, above: 4mm, below: 2mm)[
  #text(weight: "bold", size: 10.5pt)[#title]
  #v(-1.5mm)
  #line(length: 100%, stroke: 0.4pt)
]
#let blank(w) = box(width: w, height: 0.9em, stroke: (bottom: 0.5pt))
#let tick = box(width: 3.2mm, height: 3.2mm, stroke: 0.7pt)

#grid(columns: (1fr, auto),
  [#text(weight: "bold")[{{PHARMACY_NAME}}] \ #text(size: 9pt)[{{PHARMACY_PHONE}}]],
  [#align(right)[Le {{DATE}}]],
)
#v(3mm)
#align(center)[#text(14pt, weight: "bold")[Vaccination à l'officine]]
#v(2mm)
#box(width: 100%, stroke: 0.7pt, inset: 7pt)[
  #grid(columns: (1fr, 1fr), row-gutter: 1.5mm,
    [*Patient :* {{PATIENT_NAME}}], [*Né(e) le :* {{BIRTH_DATE}} ({{AGE}})],
    [*Sexe :* {{SEX}}], [*Pharmacien :* {{PHARMACIST}}],
  )
]

#sec[Avant l'injection : une réponse « oui » est à instruire avant de vacciner]
{{QUESTIONS}}

#sec[Vaccin administré]
#grid(columns: (1fr, 1fr), row-gutter: 3mm, column-gutter: 6mm,
  [Vaccin : #blank(1fr)], [Dose : #blank(1fr)],
  [N° de lot : #blank(1fr)], [Péremption : #blank(1fr)],
  [Voie : #h(1mm) #tick #h(1mm) IM #h(4mm) #tick #h(1mm) SC], [Heure : #blank(1fr)],
  [Site : #h(1mm) #tick #h(1mm) deltoïde gauche #h(4mm) #tick #h(1mm) deltoïde droit], [Autre site : #blank(1fr)],
)

#sec[Après l'injection]
{{AFTER}}

#sec[Calendrier vaccinal d'après le carnet]
{{DUE}}

#sec[Traitements connus à l'officine]
{{TREATMENTS}}

#v(1fr)
#grid(columns: (1fr, 1fr), column-gutter: 6mm,
  [#text(weight: "bold")[Observations]
   #v(1mm)
   #box(width: 100%, height: 2cm, stroke: 0.7pt)],
  [#text(weight: "bold")[Signature du pharmacien]
   #v(1mm)
   #box(width: 100%, height: 2cm, stroke: 0.7pt)],
)
"##;

/// Les valeurs de la fiche de vaccination — l'impression et l'aperçu.
fn vaccination_values(
    p: &VaccinationPaper,
    pharmacy: &PharmacyConfig,
) -> Vec<(&'static str, String)> {
    let s = |v: &str| format!("#{}", typst_str(v));
    let filled = crate::vaccsheet::fill(p.content);
    let owed: Vec<String> = p
        .due
        .iter()
        .filter_map(|l| {
            let what = if l.detail.trim().is_empty() {
                l.label.to_owned()
            } else {
                format!("{} — {}", l.label, l.detail.trim())
            };
            match l.level {
                crate::vaccines::DueLevel::Due => Some(crate::strings::trf("vacc_pdf_due", what)),
                crate::vaccines::DueLevel::Ask => Some(crate::strings::trf("vacc_pdf_ask", what)),
                crate::vaccines::DueLevel::Ok => None,
            }
        })
        .collect();
    let due = if owed.is_empty() {
        s(crate::strings::tr("vacc_pdf_due_none"))
    } else {
        owed.iter()
            .map(|l| format!("- #{}", typst_str(l)))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let sex = match p.patient.known_sex() {
        Some(crate::ordonnance::Sex::F) => "F",
        Some(crate::ordonnance::Sex::M) => "M",
        None => "—",
    };
    let age = p.age.map_or_else(
        || crate::strings::tr("trod_pdf_no_age").to_owned(),
        |a| format!("{a} ans"),
    );
    let birth = if p.patient.birth_date.trim().is_empty() {
        "—".to_owned()
    } else {
        crate::db::format_french_date(&p.patient.birth_date)
    };
    vec![
        ("{{PHARMACY_NAME}}", s(&pharmacy.name)),
        ("{{PHARMACY_PHONE}}", s(&pharmacy.phone)),
        ("{{DATE}}", s(p.date)),
        ("{{PATIENT_NAME}}", s(&p.patient.full_name())),
        ("{{BIRTH_DATE}}", s(&birth)),
        ("{{AGE}}", s(&age)),
        ("{{SEX}}", s(sex)),
        ("{{PHARMACIST}}", s(p.signature)),
        ("{{QUESTIONS}}", tick_list(&filled.questions, 1)),
        ("{{AFTER}}", tick_list(&filled.after, 1)),
        ("{{DUE}}", due),
        ("{{TREATMENTS}}", treatments_markup(p.treats)),
    ]
}

/// La fiche de vaccination, remplie mais pas encore compilée — pour la
/// liasse.
pub fn vaccination_source(
    paper: &VaccinationPaper,
    pharmacy: &PharmacyConfig,
    template_path: &std::path::Path,
) -> String {
    fill(
        &template_source("vaccin", template_path),
        &vaccination_values(paper, pharmacy),
    )
}

/// La fiche d'une vaccination à l'officine. Voir `vaccsheet.rs`.
pub fn open_vaccination_sheet(
    paper: &VaccinationPaper,
    pharmacy: &PharmacyConfig,
    template_path: &std::path::Path,
) -> Result<PathBuf, String> {
    compile_and_open(
        vaccination_source(paper, pharmacy, template_path),
        &format!("vaccination_{}", paper.patient.id),
    )
}

/// La feuille d'un TROD, remplie mais pas encore compilée — pour la
/// liasse, qui la met en tête.
pub fn trod_source(
    paper: &TrodPaper,
    pharmacy: &PharmacyConfig,
    template_path: &std::path::Path,
) -> String {
    fill(
        &template_source("trod", template_path),
        &trod_values(paper, pharmacy),
    )
}

/// La feuille d'un TROD : ce qui se vérifie, se lit et se trace, dans
/// l'ordre où cela se fait au comptoir. Voir `trod.rs`.
pub fn open_trod_sheet(
    paper: &TrodPaper,
    pharmacy: &PharmacyConfig,
    template_path: &std::path::Path,
) -> Result<PathBuf, String> {
    let stem = format!(
        "trod_{}_{}",
        paper.patient.id,
        paper.kind.as_str().to_lowercase()
    );
    compile_and_open(trod_source(paper, pharmacy, template_path), &stem)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tous les marqueurs `{{…}}` écrits dans un texte, dans l'ordre.
    fn markers_in(text: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut rest = text;
        while let Some(open) = rest.find("{{") {
            rest = &rest[open..];
            let Some(close) = rest.find("}}") else { break };
            out.push(rest[..close + 2].to_owned());
            rest = &rest[close + 2..];
        }
        out.dedup();
        out
    }

    /// La règle du registre : ce qu'un document déclare accepter et ce
    /// que son modèle par défaut écrit sont **la même liste**.
    ///
    /// Un marqueur déclaré mais absent du modèle est une promesse que
    /// rien ne tient ; un marqueur du modèle qu'on a oublié de
    /// déclarer est un marqueur que l'éditeur ne montre pas, donc que
    /// personne n'utilisera — et que la première réécriture du modèle
    /// fera disparaître sans que rien ne le dise.
    #[test]
    fn every_document_declares_exactly_the_markers_its_template_writes() {
        for d in DOCS {
            let mut declared: Vec<String> = d.markers.iter().map(|m| (*m).to_owned()).collect();
            let mut written = markers_in(d.default);
            declared.sort();
            written.sort();
            written.dedup();
            assert_eq!(
                declared, written,
                "modèle « {} » : les marqueurs déclarés et ceux du modèle diffèrent",
                d.key
            );
            assert!(
                !d.markers.is_empty(),
                "un document sans marqueur n'est pas un modèle, c'est une page fixe ({})",
                d.key
            );
        }
    }

    /// Les clés : uniques, en ASCII minuscule (elles nomment un
    /// fichier), et retrouvables.
    #[test]
    fn the_document_keys_name_a_file_and_are_unique() {
        let mut keys: Vec<&str> = DOCS.iter().map(|d| d.key).collect();
        let n = keys.len();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), n, "deux documents partagent une clé");
        for d in DOCS {
            assert!(
                !d.key.is_empty() && d.key.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
                "« {} » ne peut pas nommer un fichier",
                d.key
            );
            // Le nom affiché est une **clé de chaîne**, et elle doit
            // exister : un libellé écrit en dur ici serait le seul
            // texte de l'interface que l'officine ne pourrait pas
            // remplacer.
            assert!(
                crate::strings::tr(d.label) != d.label,
                "« {} » : {} n'est pas dans assets/strings.fr.toml",
                d.key,
                d.label
            );
            assert_eq!(doc(d.key).map(|f| f.key), Some(d.key));
        }
        assert!(doc("inexistant").is_none());
    }

    /// **Ce qui va sur le papier porte la ponctuation française.**
    ///
    /// Trois documents commençaient une ligne par un guillemet fermant
    /// ou par un deux-points : sur deux colonnes justifiées, Typst coupe
    /// où il peut, et `lang: "fr"` ne règle que la coupure des mots.
    /// Le lien se fait dans `typst_str`, par où passe tout ce qui entre
    /// dans une page — un seul appel oublié est une ligne qui
    /// recommence.
    #[test]
    fn every_string_laid_into_a_page_binds_its_french_punctuation() {
        for (loose, bound) in [
            (
                "le panneau « Interprétation » la relit",
                "«\u{202f}Interprétation\u{202f}»",
            ),
            ("Prochaine : 18/11/2026", "Prochaine\u{202f}:"),
            ("une question ; une réponse", "question\u{202f};"),
            ("que dit l'ordonnance ?", "ordonnance\u{202f}?"),
        ] {
            let laid = typst_str(loose);
            assert!(
                laid.contains(bound),
                "« {loose} » entre dans la page sans lier sa ponctuation : {laid}"
            );
        }
        // Et rien d'autre ne bouge : une heure, un rapport, un chemin
        // n'ont pas d'espace devant leur deux-points.
        assert!(typst_str("14:30").contains("14:30"));
        assert!(typst_str("1:2").contains("1:2"));
    }

    /// Chaque modèle par défaut compile avec ses valeurs d'exemple, et
    /// il ne reste pas un seul `{{` dedans — un marqueur non substitué
    /// s'imprime en toutes lettres au milieu de la page.
    ///
    /// **Et sous `BPM_CADDY_TEST_PDF_OUT`, chaque aperçu est écrit.**
    /// C'est ce que l'officine lit dans l'éditeur de modèles pour
    /// comprendre le sien, et aucun des trente-deux n'avait jamais été
    /// *regardé* : les fichiers que cette variable produisait venaient
    /// des autres tests, dont plusieurs remplissent leurs pages de texte
    /// hostile pour prouver l'échappement — une lettre au médecin pleine
    /// de « #eval » est le gardien qui fonctionne, pas un aperçu.
    #[test]
    fn every_default_template_compiles_with_its_sample_values() {
        let out = std::env::var("BPM_CADDY_TEST_PDF_OUT").ok();
        for d in DOCS {
            let filled = fill(d.default, &sample_values(d.key));
            assert!(
                !filled.contains("{{"),
                "modèle « {} » : un marqueur n'a pas été remplacé",
                d.key
            );
            check_doc(d.key, d.default).unwrap_or_else(|e| panic!("modèle « {} » : {e}", d.key));
            let Some(dir) = out.as_deref() else { continue };
            let world = PdfWorld::new(filled);
            let document: PagedDocument = typst::compile(&world)
                .output
                .unwrap_or_else(|_| panic!("aperçu « {} » : la compilation a échoué", d.key));
            if let Ok(pdf) = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default()) {
                let _ = std::fs::write(
                    std::path::Path::new(dir).join(format!("apercu_{}.pdf", d.key)),
                    &pdf,
                );
            }
        }
    }

    /// **La liasse compile en un seul PDF.**
    ///
    /// C'est le point risqué de l'impression groupée : chaque partie
    /// porte son propre `#set page`, et trois mises en page dans un
    /// document sont trois `#set page` séparés par des sauts. Typst
    /// sait le faire — c'est ainsi qu'on change de format en cours de
    /// document — mais rien ne le prouvait, et une liasse qui ne
    /// compile pas se découvre à la fin d'un entretien.
    ///
    /// Les trois parties sont les modèles livrés, remplis de leurs
    /// valeurs d'exemple : les mêmes que l'aperçu de l'éditeur.
    #[test]
    fn a_bundle_of_three_documents_compiles_as_one() {
        let parts: Vec<String> = ["fiche", "bilan", "plan"]
            .into_iter()
            .map(|key| {
                let d = doc(key).expect("le document est au registre");
                fill(d.default, &sample_values(key))
            })
            .collect();
        assert_eq!(parts.len(), 3);
        for p in &parts {
            assert!(!p.contains("{{"), "un marqueur non remplacé");
        }
        // Assemblée par la fonction de la liasse, pas par une copie.
        let joined = bundle_source(&parts).expect("trois parties non vides");
        let world = PdfWorld::new(joined);
        let out = typst::compile::<PagedDocument>(&world).output;
        let doc = out.unwrap_or_else(|errs| {
            panic!("la liasse ne compile pas : {}", format_diagnostics(&errs))
        });
        // Et elle fait bien plusieurs pages : trois documents qui
        // rendraient une page seraient trois documents écrasés l'un sur
        // l'autre.
        assert!(
            doc.pages.len() >= 3,
            "{} page(s) pour trois documents",
            doc.pages.len()
        );
        // Et une partie vide est sautée plutôt que de faire une page
        // blanche : une feuille vierge au milieu d'une liasse se lit
        // comme une erreur d'impression.
        let with_hole = [parts[0].clone(), "  \n ".to_owned(), parts[1].clone()];
        let holed = bundle_source(&with_hole).expect("deux parties restent");
        assert_eq!(
            holed.matches("#pagebreak()").count(),
            1,
            "deux parties, un seul saut"
        );
        // Rien du tout n'est une erreur et non un PDF vide : une liasse
        // vide qui s'ouvrirait quand même ferait croire à une
        // impression réussie.
        assert!(bundle_source(&[]).is_err());
        assert!(bundle_source(&[String::new(), "   ".to_owned()]).is_err());
    }

    /// **La règle qui empêche le registre de retomber en arrière** :
    /// toute fonction `open_*` de ce module prend un chemin de modèle.
    ///
    /// Vingt-deux documents étaient écrits en Rust, mise en page et
    /// données mélangées dans le même `format!`, et une officine qui
    /// voulait sa marge ou son en-tête sur la liste d'appel n'avait
    /// rien à ouvrir. Le prochain document imprimable ajouté sans
    /// modèle recommencerait cette histoire en petit : ce test lit le
    /// texte du module et le refuse, comme
    /// `no_font_size_is_written_in_pixels` refuse le prochain pixel.
    ///
    /// **Deux exemptions, et chacune est nommée**, pour la même raison :
    /// le document n'est pas dessiné ici. Le bulletin d'adhésion est le
    /// PDF de l'Assurance Maladie, dont on ne remplit que les champs de
    /// formulaire (voir `bulletin.rs`) — lui donner un « modèle » serait
    /// le redessiner. Et la liasse met bout à bout des pages que leurs
    /// propres modèles ont déjà remplies.
    #[test]
    fn every_printable_document_takes_a_template() {
        // Deux exemptions, chacune nommée et chacune pour la même
        // raison : le document n'a pas de modèle **parce qu'il n'est
        // pas dessiné ici**.
        //
        // `open_bulletin` écrit les champs du PDF de l'Assurance
        // Maladie ; lui donner un modèle voudrait dire le redessiner.
        // `open_bundle` met bout à bout des pages déjà remplies par
        // *leurs* modèles ; lui en donner un serait un modèle de
        // modèles, et les quatre règles s'appliquent déjà à chacune des
        // parties.
        const EXEMPT: &[&str] = &["open_bulletin", "open_bundle"];
        let source = include_str!("pdf.rs");
        let mut rest = source;
        let mut checked = 0;
        while let Some(at) = rest.find("\npub fn open_") {
            rest = &rest[at + 1..];
            let name: String = rest["pub fn ".len()..]
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            let Some(end) = rest.find(") -> Result") else {
                break;
            };
            let signature = &rest[..end];
            if EXEMPT.contains(&name.as_str()) {
                continue;
            }
            checked += 1;
            assert!(
                signature.contains("template_path: &std::path::Path"),
                "{name} imprime sans modèle : ajoutez son entrée à DOCS \
                 et un `template_path` à sa signature"
            );
        }
        // Le compte est là pour que le test ne passe pas en n'ayant
        // rien lu — un analyseur qui ne trouve plus rien est vert.
        assert!(
            checked >= 25,
            "seulement {checked} fonctions lues : l'analyse a décroché"
        );
        // Et chaque clé citée par une de ces signatures est bien au
        // registre : `template_source(\"typo\", …)` rendrait
        // silencieusement une page vide.
        let mut rest = source;
        while let Some(at) = rest.find("template_source(\"") {
            rest = &rest[at + "template_source(\"".len()..];
            let key: String = rest.chars().take_while(|c| *c != '"').collect();
            assert!(
                doc(&key).is_some(),
                "« {key} » n'est pas au registre des documents"
            );
        }
    }

    /// La feuille de caisse : ce qui a été compté, et l'écart tel
    /// qu'il est — signé, jamais résorbé.
    ///
    /// Et la règle du module portée jusqu'au papier : **sans recette
    /// attendue, la ligne de l'écart est vide**, pas remplie de tout
    /// le contenu du tiroir.
    #[test]
    fn the_caisse_sheet_prints_the_gap_it_found() {
        use crate::caisse::{tally, Other, DENOMINATIONS};
        let mut q = [0_i64; DENOMINATIONS.len()];
        q[3] = 4; // 4 × 50 €
        q[4] = 7; // 7 × 20 €
        q[9] = 5; // 5 × 50 c
        let others = [Other {
            // Un libellé hostile est échappé comme partout ailleurs.
            label: "Carte #eval \"x\"".to_owned(),
            cents: 45_075,
        }];
        let t = tally(&q, 0, 15_000, &others, Some(80_000));
        let src = fill(
            DEFAULT_CAISSE_TEMPLATE,
            &caisse_values(
                "Pharmacie du Centre",
                "24/08/2026",
                "Claire Leroy",
                &q,
                &others,
                &t,
                "Un billet de 20 € retrouvé sous le tiroir.",
            ),
        );
        assert!(src.contains("Comptage de caisse"));
        // 340,00 + 2,50 = 342,50 en espèces ; 793,25 encaissés ; il
        // manque 6,75.
        assert!(src.contains("342,50"), "les espèces comptées");
        assert!(src.contains("793,25"), "la recette encaissée");
        assert!(src.contains("-6,75"), "l'écart, avec son signe : {src}");
        // Les coupures absentes restent sur la feuille, à zéro : la
        // ligne vide prouve qu'on les a regardées.
        assert!(src.contains("500 €") && src.contains("1 c"));
        assert!(src.contains("Un billet de 20 €"), "la remarque");
        assert!(!src.contains("#eval \"x\"]"), "rien n'est du code Typst");
        let world = PdfWorld::new(src);
        let document: PagedDocument = typst::compile(&world)
            .output
            .expect("la feuille de caisse doit compiler");
        let pdf = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
            .expect("l'export PDF doit réussir");
        assert!(pdf.starts_with(b"%PDF-"));
        if let Ok(dir) = std::env::var("BPM_CADDY_TEST_PDF_OUT") {
            let _ = std::fs::write(std::path::Path::new(&dir).join("caisse_exemple.pdf"), &pdf);
        }

        // Sans attendu : pas d'écart, et pas de cadre de remarque non
        // plus — un tiret plutôt qu'un chiffre inventé. On vérifie la
        // **valeur** du marqueur et non la page rendue : le total des
        // espèces y figure de toute façon, sous son propre libellé.
        let values = caisse_values(
            "Pharmacie du Centre",
            "24/08/2026",
            "Claire Leroy",
            &q,
            &[],
            &tally(&q, 0, 0, &[], None),
            "   ",
        );
        let of = |m: &str| {
            values
                .iter()
                .find(|(k, _)| *k == m)
                .map(|(_, v)| v.as_str())
                .unwrap_or("")
        };
        assert_eq!(of("{{GAP}}"), "—", "l'écart ne vaut pas tout le tiroir");
        assert_eq!(of("{{EXPECTED}}"), "—");
        assert_eq!(of("{{CASH}}"), "342,50");
        assert_eq!(
            of("{{REMARK}}"),
            "",
            "une remarque d'espaces n'en est pas une"
        );
        let world = PdfWorld::new(fill(DEFAULT_CAISSE_TEMPLATE, &values));
        assert!(typst::compile::<PagedDocument>(&world).output.is_ok());
    }

    /// L'historique porte les soirs recomptés **et** ne les compte
    /// qu'une fois.
    ///
    /// C'est la règle de `caisse::per_day` portée jusqu'au papier : les
    /// deux lignes du 7 sont sur la feuille, la première nommée
    /// « recompté », et le total ne retient que la seconde. Une
    /// histoire de caisse d'où l'on aurait retiré les comptages refaits
    /// ne prouverait rien ; une histoire qui les additionne est fausse.
    /// **Une liste de contrôle sort avec ses cases.**
    ///
    /// C'est la case qui la sépare d'un protocole : on ne descend pas
    /// une liste, on la coche, et une feuille sans case se coche au
    /// stylo dans la marge. La case est **dessinée** et non écrite : la
    /// police d'une feuille imprimée n'est pas celle de l'écran, et un
    /// carré typographique manquant sort en blanc — c'est-à-dire une
    /// liste sans cases, qui compile et ne sert à rien.
    ///
    /// La date et la personne sont laissées vides : une liste cochée
    /// sans savoir quand ni par qui ne prouve rien, et les pré-remplir
    /// serait remplir à la place de quelqu'un.
    #[test]
    fn a_checklist_prints_its_boxes() {
        let items = vec![
            crate::db::ChecklistItem {
                id: 1,
                text: "Relever la température du réfrigérateur".to_owned(),
                note: "Entre +2 et +8 °C".to_owned(),
                position: 1,
            },
            crate::db::ChecklistItem {
                id: 2,
                text: "Compter le fonds de caisse".to_owned(),
                note: String::new(),
                position: 2,
            },
        ];
        let values = checklist_values("Ouverture", "Avant d'ouvrir", &items);
        let src = fill(DEFAULT_LISTE_TEMPLATE, &values);
        // Une case par ligne, dessinée.
        assert_eq!(src.matches("stroke: 0.6pt)]").count(), items.len());
        // La date et la personne sont sur la feuille, et vides.
        assert!(src.contains("Date :") && src.contains("Par :"));
        assert!(!src.contains("{{"));
        let world = PdfWorld::new(src.clone());
        assert!(typst::compile::<PagedDocument>(&world).output.is_ok());
        // Une liste sans ligne le dit plutôt que de sortir une page
        // blanche qu'on croirait ratée.
        let empty = fill(DEFAULT_LISTE_TEMPLATE, &checklist_values("Vide", "", &[]));
        assert!(empty.contains("Liste vide."));
        assert!(typst::compile::<PagedDocument>(&PdfWorld::new(empty))
            .output
            .is_ok());
        // Écrite sur disque quand on demande à la regarder : une feuille
        // se juge à l'œil, pas à une assertion de sous-chaîne.
        if let Ok(dir) = std::env::var("BPM_CADDY_TEST_PDF_OUT") {
            let doc: PagedDocument = typst::compile(&PdfWorld::new(src))
                .output
                .expect("la feuille doit compiler");
            if let Ok(pdf) = typst_pdf::pdf(&doc, &typst_pdf::PdfOptions::default()) {
                let _ = std::fs::write(std::path::Path::new(&dir).join("liste_exemple.pdf"), &pdf);
            }
        }
    }

    #[test]
    fn the_till_history_shows_a_recount_and_counts_it_once() {
        let counts = sample_counted();
        let summary = crate::caisse::summarize(&counts);
        let rows = sample_caisse_history();
        assert_eq!(rows.len(), 3, "les trois lignes sont sur la feuille");
        assert_eq!(rows.iter().filter(|r| r.superseded).count(), 1);
        let values = caisse_history_values(
            &rows,
            "septembre 2026",
            &summary,
            "Pharmacie du Centre",
            true,
        );
        let of = |m: &str| {
            values
                .iter()
                .find(|(k, _)| *k == m)
                .map(|(_, v)| v.as_str())
                .unwrap_or("")
        };
        // 204,50 − 150,00 + 450,75 le 7 (le recomptage), 312,00 −
        // 150,00 + 523,00 le 8 : 1 190,25 €, fond d'ouverture retranché.
        // Avec les deux comptages du 7 additionnés, on lirait 1 392,75
        // — l'erreur que ce test existe pour tenir.
        assert_eq!(of("{{TAKINGS}}"), "1\u{a0}190,25 €");
        assert!(of("{{DAYS}}").contains("2 soirs comptés"));
        assert!(of("{{DAYS}}").contains("1 recomptage"));
        // L'écart porte sur le seul soir qui avait un attendu, et le
        // dit : sans ce nombre à côté, la somme se lirait comme si elle
        // couvrait le mois.
        assert!(of("{{GAP}}").contains("sur 1 soir"), "{}", of("{{GAP}}"));
        // **Le même signe moins que dans le tableau.** Les cellules
        // sont posées en markup Typst, qui rend le trait d'union d'un
        // nombre négatif par U+2212 ; ces phrases-ci passent par
        // `typst_str`, littéralement, et gardaient le trait d'union —
        // une même page écrivait « −4,75 € » dans la colonne et
        // « -4,75 € » dans le récapitulatif juste dessous.
        assert!(
            of("{{GAP}}").contains("\u{2212}4,75 €"),
            "le récapitulatif doit porter le signe moins du tableau : {}",
            of("{{GAP}}")
        );
        assert!(
            !of("{{GAP}}").contains("-4,75"),
            "et pas le trait d'union : {}",
            of("{{GAP}}")
        );
        assert!(
            of("{{WORST}}").contains("\u{2212}4,75 €"),
            "{}",
            of("{{WORST}}")
        );
        let src = fill(DEFAULT_CAISSES_TEMPLATE, &values);
        assert!(src.contains("recompté"));
        assert!(!src.contains("{{"));
        let world = PdfWorld::new(src.clone());
        assert!(typst::compile::<PagedDocument>(&world).output.is_ok());
        // Écrite sur disque quand on demande à la regarder : une feuille
        // se juge à l'œil, pas à une assertion de sous-chaîne.
        if let Ok(dir) = std::env::var("BPM_CADDY_TEST_PDF_OUT") {
            let doc: PagedDocument = typst::compile(&PdfWorld::new(src.clone()))
                .output
                .expect("la feuille doit compiler");
            if let Ok(pdf) = typst_pdf::pdf(&doc, &typst_pdf::PdfOptions::default()) {
                let _ = std::fs::write(
                    std::path::Path::new(&dir).join("historique_caisse_exemple.pdf"),
                    &pdf,
                );
            }
        }

        // Et un mois où personne n'a saisi de recette attendue le dit,
        // plutôt que d'imprimer « 0,00 € » — qui se lirait « tout est
        // tombé juste ».
        let blind: Vec<crate::caisse::Counted> = counts
            .iter()
            .cloned()
            .map(|c| crate::caisse::Counted {
                expected: None,
                ..c
            })
            .collect();
        let values = caisse_history_values(
            &sample_caisse_history()
                .into_iter()
                .map(|r| CaisseHistoryRow {
                    expected: None,
                    gap: None,
                    ..r
                })
                .collect::<Vec<_>>(),
            "septembre 2026",
            &crate::caisse::summarize(&blind),
            "Pharmacie du Centre",
            true,
        );
        let gap = values
            .iter()
            .find(|(k, _)| *k == "{{GAP}}")
            .map(|(_, v)| v.clone())
            .unwrap_or_default();
        assert!(gap.contains("aucune recette attendue"), "{gap}");
        assert!(!gap.contains("0,00"));
    }

    /// L'officine qui ne compte pas contre une recette attendue n'a pas
    /// d'écart **sur le papier non plus**.
    ///
    /// Et la feuille ne dit pas « aucune recette attendue n'a été
    /// saisie » : ce serait annoncer un manque là où il y a un choix.
    /// La phrase disparaît, le reste du mois est écrit comme d'habitude.
    #[test]
    fn a_till_history_without_expected_takings_prints_no_gap_at_all() {
        let rows = sample_caisse_history();
        let summary = crate::caisse::summarize(&sample_counted());
        let values = caisse_history_values(
            &rows,
            "septembre 2026",
            &summary,
            "Pharmacie du Centre",
            false,
        );
        let of = |m: &str| {
            values
                .iter()
                .find(|(k, _)| *k == m)
                .map(|(_, v)| v.as_str())
                .unwrap_or("")
        };
        assert_eq!(of("{{GAP}}"), "", "pas d'écart, pas de phrase d'écart");
        assert_eq!(of("{{WORST}}"), "");
        // Le mois, lui, est toujours là : c'est un historique de caisse
        // avant d'être un relevé d'écarts.
        assert!(of("{{DAYS}}").contains("2 soirs comptés"));
        assert!(!of("{{TAKINGS}}").is_empty());
        let src = fill(DEFAULT_CAISSES_TEMPLATE, &values);
        assert!(!src.contains("{{"));
        assert!(!src.contains("aucune recette attendue"));
        let world = PdfWorld::new(src);
        assert!(typst::compile::<PagedDocument>(&world).output.is_ok());
    }

    /// Les étiquettes : une par traitement, la posologie du dossier, et
    /// la phrase d'oubli coupée à la lecture plutôt qu'au sens.
    ///
    /// Ce qui doit tenir : le nom et la posologie passent entiers — ce
    /// sont eux qu'on lit sur la boîte —, c'est la phrase de l'oubli
    /// qui s'arrête, et un traitement sans phrase d'oubli donne une
    /// étiquette sans ligne vide plutôt qu'une étiquette avec « Oubli :
    /// » et rien derrière.
    /// **Tout raccourci auquel l'application répond est sur le mode
    /// d'emploi imprimé.**
    ///
    /// La feuille recopiait la liste à la main, et il en manquait
    /// quatre : `F2`, `F9`, `Ctrl+Shift+Tab` et les chiffres du choix
    /// rapide. Une liste recopiée vieillit là où personne ne la relit —
    /// et celle-ci s'imprime et se pose près du poste, c'est-à-dire à
    /// l'endroit exact où on la croit à jour.
    ///
    /// Le manuel de l'écran a résolu le problème en ne recopiant rien ;
    /// le papier ne peut pas faire pareil, alors il est confronté.
    #[test]
    fn every_shortcut_the_app_answers_to_is_on_the_printed_guide() {
        let mut missing: Vec<&str> = crate::app::key_rows()
            .into_iter()
            .map(|(key, _)| key)
            .filter(|key| !key.is_empty())
            // Les flèches nues sont décrites en toutes lettres sur la
            // feuille — « les flèches parcourent » —, pas par leur
            // glyphe : les chercher tels quels dirait faux.
            .filter(|key| !key.contains('\u{2191}') && !key.contains('\u{2190}'))
            // Une case qui porte deux formes — « 230826 · 2308 » — est
            // tenue si la feuille porte les deux, chacune dans sa
            // phrase ; la chercher d'un bloc exigerait que le papier
            // recopie jusqu'au point médian.
            .flat_map(|key| key.split(" \u{b7} ").collect::<Vec<_>>())
            .filter(|part| !GUIDE_SHORTCUTS.contains(part))
            .collect();
        missing.sort_unstable();
        assert!(
            missing.is_empty(),
            "raccourcis absents du mode d'emploi imprimé : {missing:?}"
        );
    }

    #[test]
    fn the_labels_carry_the_dose_and_shorten_only_what_they_must() {
        let patient = sample_patient();
        let long = "Prendre le comprimé oublié dès que l'on s'en aperçoit, sauf s'il est presque l'heure de la prise suivante, auquel cas on saute la prise oubliée et on reprend le rythme habituel sans jamais doubler la dose.";
        const LONG_DOSE: &str =
            "1 comprimé le matin et 1 le soir pendant 7 jours, puis 1 comprimé le matin seul";
        let data = LabelSheet {
            patient: &patient,
            today: "24/08/2026",
            lines: vec![
                (
                    "Lévothyrox 75 µg".to_owned(),
                    "1 comprimé le matin à jeun, 30 minutes avant le petit-déjeuner".to_owned(),
                    long.to_owned(),
                ),
                (
                    "Doliprane 1 g #eval \"x\"".to_owned(),
                    String::new(),
                    String::new(),
                ),
                // **Une posologie plus longue que ce qu'une étiquette
                // porte confortablement.** Sans elle, l'assertion « la
                // posologie passe entière » se vérifiait sur soixante-
                // deux caractères, c'est-à-dire sur un cas qui tenait
                // de toute façon — et la posologie était bel et bien
                // coupée à soixante-dix, avec trois points, sur la
                // boîte du patient.
                (
                    "Kardégic 75 mg".to_owned(),
                    LONG_DOSE.to_owned(),
                    String::new(),
                ),
            ],
            mention: "",
        };
        let values = label_sheet_values(&data, &sample_pharmacy());
        let labels = values
            .iter()
            .find(|(k, _)| *k == "{{LABELS}}")
            .map(|(_, v)| v.clone())
            .unwrap_or_default();
        assert!(
            labels.contains("30 minutes avant le petit-déjeuner"),
            "la posologie passe entière : c'est ce qu'on lit sur la boîte"
        );
        assert!(
            labels.contains(LONG_DOSE),
            "et une posologie longue aussi : c'est le corps qui cède, pas le sens"
        );
        assert!(
            labels.contains("8pt)"),
            "le corps descend d'un cran quand la posologie est longue"
        );
        assert!(labels.contains('…'), "la phrase d'oubli est coupée");
        assert!(
            !labels.contains("sans jamais doubler la dose"),
            "et coupée pour de bon"
        );
        // Le second traitement n'a rien à dire sur l'oubli : pas de
        // ligne du tout, plutôt qu'un « Oubli : » suivi de rien.
        assert_eq!(labels.matches("Oubli :").count(), 1);
        assert!(!labels.contains("#eval \"x\"]"), "rien n'est du code Typst");
        // Une mention vide n'imprime pas un cadre vide.
        assert_eq!(
            values
                .iter()
                .find(|(k, _)| *k == "{{MENTION}}")
                .map(|(_, v)| v.as_str()),
            Some("")
        );
        let world = PdfWorld::new(fill(DEFAULT_ETIQUETTES_TEMPLATE, &values));
        assert!(typst::compile::<PagedDocument>(&world).output.is_ok());
        // Écrite sur disque quand on demande à la regarder : une feuille
        // se juge à l'œil, pas à une assertion de sous-chaîne.
        if let Ok(dir) = std::env::var("BPM_CADDY_TEST_PDF_OUT") {
            let doc: PagedDocument =
                typst::compile(&PdfWorld::new(fill(DEFAULT_ETIQUETTES_TEMPLATE, &values)))
                    .output
                    .expect("la feuille doit compiler");
            if let Ok(pdf) = typst_pdf::pdf(&doc, &typst_pdf::PdfOptions::default()) {
                let _ = std::fs::write(
                    std::path::Path::new(&dir).join("etiquettes_exemple.pdf"),
                    &pdf,
                );
            }
        }
    }

    /// Le plan de surveillance coche ce qui est en retard, et **n'écrit
    /// aucun chiffre de résultat**.
    ///
    /// La feuille dit quoi faire doser et à quel rythme ; ce que le
    /// résultat devrait valoir n'y est pas, et la colonne « Résultat »
    /// est vide, pour le laboratoire. Une norme imprimée sur une
    /// feuille qui part à la maison est une invitation à s'interpréter
    /// seul.
    #[test]
    fn the_watch_sheet_ticks_what_is_late_and_prints_no_figure() {
        let patient = sample_patient();
        let data = WatchSheet {
            patient: &patient,
            today: "24/08/2026",
            rows: vec![
                (
                    "Créatinine et DFG".to_owned(),
                    "tous les 6 mois".to_owned(),
                    "jamais noté".to_owned(),
                    "Périndopril, Furosémide".to_owned(),
                ),
                (
                    "Kaliémie".to_owned(),
                    "tous les 3 mois".to_owned(),
                    // À jour : la case ne se pose que sur ce qui est en
                    // retard, et l'exemple doit montrer les deux cas.
                    "12/07/2026".to_owned(),
                    "Périndopril".to_owned(),
                ),
            ],
            flagged: vec!["Créatinine et DFG".to_owned()],
            mention: "Document remis à titre informatif.",
        };
        let values = watch_sheet_values(&data, &sample_pharmacy());
        let rows = values
            .iter()
            .find(|(k, _)| *k == "{{ROWS}}")
            .map(|(_, v)| v.clone())
            .unwrap_or_default();
        // Une case à cocher, pas deux : une feuille où tout est à
        // cocher ne dit plus ce qui presse.
        assert_eq!(rows.matches("stroke: 0.6pt").count(), 1);
        assert!(rows.contains("jamais noté"));
        assert!(rows.contains("Périndopril, Furosémide"));
        let src = fill(DEFAULT_SURVEILLANCE_TEMPLATE, &values);
        assert!(!src.contains("{{"));
        let world = PdfWorld::new(src.clone());
        assert!(typst::compile::<PagedDocument>(&world).output.is_ok());
        // Écrite sur disque quand on demande à la regarder : une feuille
        // se juge à l'œil, pas à une assertion de sous-chaîne.
        if let Ok(dir) = std::env::var("BPM_CADDY_TEST_PDF_OUT") {
            let doc: PagedDocument = typst::compile(&PdfWorld::new(src.clone()))
                .output
                .expect("la feuille doit compiler");
            if let Ok(pdf) = typst_pdf::pdf(&doc, &typst_pdf::PdfOptions::default()) {
                let _ = std::fs::write(
                    std::path::Path::new(&dir).join("surveillance_exemple.pdf"),
                    &pdf,
                );
            }
        }
    }

    /// Et un marqueur inventé est refusé par son nom, plutôt que
    /// silencieusement imprimé.
    #[test]
    fn an_unknown_marker_is_named_rather_than_printed() {
        let err = check_doc("carnet", "#set page(paper: \"a4\")\n{{JOURNEE}}\n")
            .expect_err("un marqueur inconnu doit être refusé");
        assert!(
            err.contains("JOURNEE"),
            "l'erreur doit nommer le marqueur : {err}"
        );
    }

    /// Un modèle absent du disque rend celui qui est embarqué ; un
    /// modèle écrit par l'officine le remplace.
    #[test]
    fn the_officine_template_wins_over_the_embedded_one() {
        let dir = std::env::temp_dir().join(format!("bpm_tpl_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("caisse.typ");
        let _ = std::fs::remove_file(&path);
        assert_eq!(template_source("caisse", &path), DEFAULT_CAISSE_TEMPLATE);
        std::fs::write(&path, "#set page(paper: \"a5\")\n{{DATE}}\n").unwrap();
        assert!(template_source("caisse", &path).contains("a5"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn billing_recap_compiles_and_totals_the_acts() {
        let line = |patient: &str, code: &str, fee: f64, remote| BillingLine {
            date: "2026-08-24".to_owned(),
            patient: patient.to_owned(),
            kind: "Bilan de médication".to_owned(),
            code: code.to_owned(),
            step: "Entretien initial".to_owned(),
            situation: "ALD".to_owned(),
            remote,
            coverage: 70,
            fee,
        };
        let lines = vec![
            line("Hélène Lefèvre", "BMI", 15.0, false),
            // A hostile name must not inject markup into the page.
            line("Paul #eval \"Bernard\"", "BMI", 20.5, true),
        ];
        // The rentals print as their own table, with their own total:
        // they are not acts and must never join the acts' figure.
        let rentals = vec![BillingRental {
            patient: "Hélène Lefèvre".to_owned(),
            label: "Nébuliseur".to_owned(),
            started: "2026-08-03".to_owned(),
            ended: String::new(),
            periods: 4,
            // Accordé avec le nombre : c'est `Period::agreed` qui le
            // donne, et l'exemple doit montrer ce que la feuille écrit.
            period_word: "semaines".to_owned(),
            amount: 48.0,
        }];
        let src = fill(
            DEFAULT_FACTURATION_TEMPLATE,
            &billing_recap_values(&lines, &rentals, "Août 2026", "24/08/2026"),
        );
        assert!(!src.contains("#eval \"Bernard\"]"));
        // The TPH code sits beside the act code, and the total adds up.
        assert!(src.contains("BMI + TPH"));
        assert!(src.contains("35,50 €"));
        assert!(src.contains("Locations de matériel"));
        assert!(src.contains("48,00 €"));
        assert!(src.contains("en cours"));
        // No rental, no second table: an empty heading reads as a bug.
        let bare = fill(
            DEFAULT_FACTURATION_TEMPLATE,
            &billing_recap_values(&lines, &[], "Août 2026", "24/08/2026"),
        );
        assert!(!bare.contains("Locations de matériel"));
        let world = PdfWorld::new(src);
        let document: PagedDocument = typst::compile(&world)
            .output
            .expect("le récapitulatif doit compiler");
        let pdf = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
            .expect("l'export PDF doit réussir");
        assert!(pdf.starts_with(b"%PDF-"));
        if let Ok(dir) = std::env::var("BPM_CADDY_TEST_PDF_OUT") {
            let _ = std::fs::write(
                std::path::Path::new(&dir).join("facturation_exemple.pdf"),
                &pdf,
            );
        }
    }

    /// L'ordonnancier imprimé : la suite des numéros, le dossier et
    /// jamais le nom, et les annulations imprimées **annulées**.
    ///
    /// Les trois choses qu'une feuille sortie pour un contrôle doit
    /// tenir. La troisième surtout : une copie du registre d'où l'on
    /// aurait ôté les erreurs ne serait pas une copie du registre, et
    /// c'est précisément ce qu'un logiciel « propre » ferait.
    #[test]
    fn the_ordonnancier_prints_the_numbers_the_files_and_the_cancellations() {
        let line = |id: i64, no: i64, product: i64, qty: f64, patient: i64| crate::db::StupMove {
            id,
            stup_id: product,
            kind: "SORTIE".to_owned(),
            happened_on: "2026-01-08".to_owned(),
            quantity: qty,
            ordo_year: 2026,
            ordo_no: no,
            patient_id: patient,
            // Un nom hostile ne doit pas injecter de balisage dans la
            // page — un prescripteur se saisit à la main.
            prescriber: "Dr #eval \"Martin\"".to_owned(),
            supplier: String::new(),
            reference: String::new(),
            expected: 0.0,
            operator: "YS".to_owned(),
            remark: String::new(),
            cancels: 0,
            lot: String::new(),
            expiry: String::new(),
        };
        let rows = [line(1, 1, 7, 14.0, 55), line(2, 2, 7, 14.0, 61)];
        let mut labels = std::collections::HashMap::new();
        labels.insert(7_i64, "Skenan LP 30 mg".to_owned());
        let mut cancelled = std::collections::HashSet::new();
        cancelled.insert(2_i64);
        let src = fill(
            DEFAULT_ORDONNANCIER_TEMPLATE,
            &ordonnancier_values(
                &rows,
                &labels,
                &cancelled,
                2026,
                &PharmacyConfig::default(),
                "2026-08-30",
            ),
        );
        assert!(!src.contains("#eval \"Martin\"]"));
        // Le numéro nu : la suite est continue, et l'année devant ne
        // servait qu'à désambiguïser un « 1 » qui revenait tous les
        // ans. C'est aussi ce qui est écrit sur l'ordonnance, où
        // personne ne recopie un millésime.
        assert!(
            src.contains(r#"[*#"1"*]"#) && src.contains(r#"[*#"2"*]"#),
            "{src}"
        );
        assert!(!src.contains("2026-0001"));
        assert!(src.contains("Skenan LP 30 mg"));
        // Le dossier, et **jamais** le nom : une feuille imprimée sort
        // du logiciel, se pose sur un comptoir et se garde dix ans.
        assert!(src.contains("dossier 55") && src.contains("dossier 61"));
        // La ligne annulée est imprimée barrée, et l'état de la colonne
        // le dit en toutes lettres.
        assert!(src.contains("#strike["));
        assert_eq!(src.matches("annulée").count(), 2, "la cellule, et le pied");
        // Un produit que la table des libellés ne connaît pas n'imprime
        // pas un identifiant nu.
        let orphan = fill(
            DEFAULT_ORDONNANCIER_TEMPLATE,
            &ordonnancier_values(
                &[line(3, 3, 99, 7.0, 55)],
                &labels,
                &std::collections::HashSet::new(),
                2026,
                &PharmacyConfig::default(),
                "2026-08-30",
            ),
        );
        assert!(!orphan.contains("#strike["), "rien n'y est annulé");
        assert!(orphan.contains("—"));
        // Une année sans délivrance imprime un tableau vide plutôt que
        // rien : c'est aussi une réponse.
        let empty = fill(
            DEFAULT_ORDONNANCIER_TEMPLATE,
            &ordonnancier_values(
                &[],
                &labels,
                &std::collections::HashSet::new(),
                2025,
                &PharmacyConfig::default(),
                "2026-08-30",
            ),
        );
        let world = PdfWorld::new(empty);
        assert!(typst::compile::<PagedDocument>(&world).output.is_ok());

        let world = PdfWorld::new(src);
        let document: PagedDocument = typst::compile(&world)
            .output
            .expect("l'ordonnancier doit compiler");
        let pdf = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
            .expect("l'export PDF doit réussir");
        assert!(pdf.starts_with(b"%PDF-"));
        if let Ok(dir) = std::env::var("BPM_CADDY_TEST_PDF_OUT") {
            let _ = std::fs::write(
                std::path::Path::new(&dir).join("ordonnancier_exemple.pdf"),
                &pdf,
            );
        }
    }

    /// Le registre d'un produit s'imprime avec son solde en face de
    /// chaque ligne, ses annulations barrées, et la phrase qui explique
    /// pourquoi les colonnes ne s'additionnent pas au solde final.
    ///
    /// Cette phrase n'est pas de la décoration : un inventaire **pose**
    /// le solde au lieu de s'y ajouter, donc dès qu'un comptage a trouvé
    /// un écart, entrées moins sorties ne tombe plus sur le solde. Sans
    /// elle, la page a l'air fausse.
    #[test]
    fn the_register_of_one_product_prints_its_balance_against_every_line() {
        let line = |id: i64, kind: &str, qty: f64, day: &str, expected: f64| crate::db::StupMove {
            id,
            stup_id: 1,
            kind: kind.to_owned(),
            happened_on: day.to_owned(),
            quantity: qty,
            ordo_year: if kind == "SORTIE" { 2026 } else { 0 },
            ordo_no: if kind == "SORTIE" { id } else { 0 },
            patient_id: if kind == "SORTIE" { 217 } else { 0 },
            prescriber: if kind == "SORTIE" {
                "Dr Martin".to_owned()
            } else {
                String::new()
            },
            supplier: String::new(),
            reference: String::new(),
            expected,
            operator: "YS".to_owned(),
            remark: String::new(),
            cancels: 0,
            lot: String::new(),
            expiry: String::new(),
        };
        let rows = vec![
            line(1, "ENTREE", 30.0, "2026-01-05", 0.0),
            line(2, "SORTIE", 14.0, "2026-01-08", 0.0),
            line(3, "INVENTAIRE", 15.0, "2026-02-01", 16.0),
        ];
        let moves: Vec<crate::ordonnancier::Move> = rows
            .iter()
            .map(|m| crate::ordonnancier::Move {
                kind: crate::ordonnancier::Kind::from_key(&m.kind),
                quantity: m.quantity,
                day: &m.happened_on,
                seq: m.id,
                cancels: m.cancels,
                expected: m.expected,
            })
            .collect();
        let running = crate::ordonnancier::running(&moves);
        // 30, puis 16, puis le comptage qui **pose** 15.
        assert_eq!(running.len(), 3);
        assert!((running[2].stock - 15.0).abs() < 1e-9, "{running:?}");

        let mut cancelled = std::collections::HashSet::new();
        cancelled.insert(2_i64);
        let src = fill(
            DEFAULT_REGISTRE_TEMPLATE,
            &stup_register_values(
                "Skenan LP 30 mg",
                "gélule",
                &rows,
                &running,
                &cancelled,
                &PharmacyConfig::default(),
                "2026-08-30",
            ),
        );
        assert!(src.contains("Registre des stupéfiants"));
        assert!(src.contains("Skenan LP 30 mg"));
        assert!(src.contains("[*Solde*]"), "la colonne du solde");
        assert!(src.contains("strike"), "la ligne annulée reste, barrée");
        assert!(
            src.contains("un comptage a trouvé un écart"),
            "la page doit expliquer pourquoi les colonnes ne tombent pas"
        );
        assert!(src.contains("gélule"), "l'unité de comptage est dite");

        // Un registre vide compile aussi : un produit suivi qu'on n'a
        // pas encore mouvementé est une page blanche, pas une erreur.
        let empty = fill(
            DEFAULT_REGISTRE_TEMPLATE,
            &stup_register_values(
                "Skenan LP 30 mg",
                "",
                &[],
                &[],
                &std::collections::HashSet::new(),
                &PharmacyConfig::default(),
                "2026-08-30",
            ),
        );
        let world = PdfWorld::new(empty);
        assert!(typst::compile::<PagedDocument>(&world).output.is_ok());

        let world = PdfWorld::new(src);
        let document: PagedDocument = typst::compile(&world)
            .output
            .expect("le registre doit compiler");
        let pdf = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
            .expect("l'export PDF doit réussir");
        assert!(pdf.starts_with(b"%PDF-"));
        if let Ok(dir) = std::env::var("BPM_CADDY_TEST_PDF_OUT") {
            let _ = std::fs::write(
                std::path::Path::new(&dir).join("registre_exemple.pdf"),
                &pdf,
            );
        }
    }

    #[test]
    fn week_plan_compiles_with_hours_and_entries() {
        use crate::db::{Event, EventCategory};
        let week: Vec<String> = (24..=30).map(|d| format!("2026-08-{d:02}")).collect();
        let rdvs = vec![
            Appointment {
                id: 1,
                time: "09:30".to_owned(),
                patient_id: 1,
                patient_name: "Hélène Lefèvre".to_owned(),
                phone: "06 12 34 56 78".to_owned(),
                kind: InterviewKind::Aod,
                date: "2026-08-27".to_owned(),
                remote: false,
                duration_minutes: 30,
                operator: "CL".to_owned(),
            },
            Appointment {
                id: 2,
                time: String::new(),
                patient_id: 2,
                patient_name: "Paul #eval \"Bernard\"".to_owned(),
                phone: String::new(),
                kind: InterviewKind::Asthme,
                date: "2026-08-27".to_owned(),
                remote: true,
                duration_minutes: 0,
                operator: String::new(),
            },
        ];
        let events = vec![Event {
            end_time: String::new(),
            id: 1,
            day: "2026-08-25".to_owned(),
            time: "14:00".to_owned(),
            title: "Formation AOD".to_owned(),
            category: EventCategory::Formation,
            repeat_days: 0,
            cadence: String::new(),
            source_id: 1,
        }];
        let src = fill(
            DEFAULT_SEMAINE_TEMPLATE,
            &week_plan_values(&week, &rdvs, &events, "2026-08-25"),
        );
        // The hostile name is escaped, and the timed rendez-vous leads.
        assert!(!src.contains("#eval \"Bernard\"]"));
        assert!(src.find("09:30").unwrap() < src.find("Paul").unwrap());
        let world = PdfWorld::new(src);
        let document: PagedDocument = typst::compile(&world)
            .output
            .expect("le plan de semaine doit compiler");
        let pdf = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
            .expect("l'export PDF doit réussir");
        assert!(pdf.starts_with(b"%PDF-"));
        if let Ok(dir) = std::env::var("BPM_CADDY_TEST_PDF_OUT") {
            let _ = std::fs::write(std::path::Path::new(&dir).join("semaine_exemple.pdf"), &pdf);
        }
    }

    #[test]
    fn protocol_page_compiles_and_keeps_the_tree_order() {
        use crate::db::{Branch, NodeKind, ProtocolNode};
        let node =
            |id: i64, parent: Option<i64>, branch, kind, text: &str, position| ProtocolNode {
                id,
                parent_id: parent,
                branch,
                kind,
                text: text.to_owned(),
                position,
            };
        let nodes = vec![
            node(
                1,
                None,
                Branch::Root,
                NodeKind::Question,
                "Clairance inférieure à 30 mL/min",
                0,
            ),
            node(
                2,
                Some(1),
                Branch::Yes,
                NodeKind::Action,
                "Appeler le prescripteur *pour un relais*",
                1,
            ),
            node(
                3,
                Some(1),
                Branch::No,
                NodeKind::Question,
                "Apixaban disponible",
                2,
            ),
            node(4, Some(3), Branch::Yes, NodeKind::Action, "Délivrer", 3),
        ];
        let source = fill(
            DEFAULT_PROTOCOLE_TEMPLATE,
            &protocol_values("AOD indisponible", "AOD", &nodes),
        );
        // The "yes" branch is written before the "no" one, and deeper
        // steps are indented further.
        let yes = source.find("Appeler le prescripteur").unwrap();
        let no = source.find("Apixaban disponible").unwrap();
        assert!(yes < no);
        assert!(source.contains("#pad(left: 7mm)"));
        let world = PdfWorld::new(source);
        let document: PagedDocument = typst::compile(&world)
            .output
            .expect("le protocole doit compiler");
        let pdf = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
            .expect("l'export PDF doit réussir");
        assert!(pdf.starts_with(b"%PDF-"));
        if let Ok(dir) = std::env::var("BPM_CADDY_TEST_PDF_OUT") {
            let _ = std::fs::write(
                std::path::Path::new(&dir).join("protocole_exemple.pdf"),
                &pdf,
            );
        }
    }

    #[test]
    fn carnet_template_compiles_with_operator_colours() {
        // The default template must compile, colour each operator, and
        // survive a page with no entry at all.
        check_doc("carnet", DEFAULT_TRANS_TEMPLATE).expect("le carnet par défaut doit compiler");
        let empty = fill_trans_template(DEFAULT_TRANS_TEMPLATE, "Lundi 24/08/2026", &[]);
        let world = PdfWorld::new(empty);
        let document: PagedDocument = typst::compile(&world)
            .output
            .expect("une page vide doit compiler");
        let pdf = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
            .expect("l'export PDF doit réussir");
        assert!(pdf.starts_with(b"%PDF-"));
        if let Ok(dir) = std::env::var("BPM_CADDY_TEST_PDF_OUT") {
            let filled = fill_trans_template(
                DEFAULT_TRANS_TEMPLATE,
                "Lundi 24/08/2026",
                &sample_transmissions(),
            );
            let world = PdfWorld::new(filled);
            if let Ok(doc) = typst::compile::<PagedDocument>(&world).output {
                if let Ok(pdf) = typst_pdf::pdf(&doc, &typst_pdf::PdfOptions::default()) {
                    let _ =
                        std::fs::write(std::path::Path::new(&dir).join("carnet_exemple.pdf"), &pdf);
                }
            }
        }
    }

    #[test]
    fn drug_monograph_compiles_and_escapes() {
        let mut d = crate::db::Drug {
            id: 1,
            name: "Eliquis #eval \"X\"".to_owned(),
            dci: "apixaban".to_owned(),
            class: "AOD".to_owned(),
            dosage: "5 mg x2/j".to_owned(),
            ddi: "Inhibiteurs du CYP3A4".to_owned(),
            iup: "Deux prises par jour.\n\nSignaler tout saignement.".to_owned(),
            antidote: "Andexanet alfa".to_owned(),
            notes: String::new(),
            half_life: "12 h".to_owned(),
            auc: String::new(),
            elimination: "Biliaire".to_owned(),
            renal: "DFG < 15 : non recommandé".to_owned(),
            pregnancy: "Contre-indiqué".to_owned(),
            indications: "Fibrillation atriale *non* valvulaire".to_owned(),
            mechanism: "Inhibiteur direct du facteur Xa".to_owned(),
            contraindications: "Saignement évolutif".to_owned(),
            adverse: "Saignements".to_owned(),
            monitoring: "Clairance annuelle".to_owned(),
            sources: "RCP Eliquis (ANSM)\nESC 2020".to_owned(),
            status: "Commercialisé".to_owned(),
            smr: String::new(),
            tags: "aod, surveillance biologique".to_owned(),
            toxicity: String::new(),
            forms: "Comprimé pelliculé 2,5 mg et 5 mg".to_owned(),
            missed_dose: "Dans les 6 heures, sinon sauter la prise.".to_owned(),
            red_flags: "Selles noires, traumatisme crânien.".to_owned(),
        };
        let source = fill(
            DEFAULT_MONOGRAPHIE_TEMPLATE,
            &monograph_values(&d, &[], &[]),
        );
        // Hostile text is escaped, never interpreted as Typst markup.
        assert!(!source.contains("#eval \"X\"]"));
        let world = PdfWorld::new(source);
        let document: PagedDocument = typst::compile(&world)
            .output
            .expect("la monographie doit compiler");
        let pdf = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
            .expect("l'export PDF doit réussir");
        assert!(pdf.starts_with(b"%PDF-"));
        if let Ok(dir) = std::env::var("BPM_CADDY_TEST_PDF_OUT") {
            let _ = std::fs::write(
                std::path::Path::new(&dir).join("monographie_exemple.pdf"),
                &pdf,
            );
        }
        // A card with only its identity still produces a valid sheet.
        d.indications.clear();
        d.mechanism.clear();
        d.dosage.clear();
        d.contraindications.clear();
        d.ddi.clear();
        d.adverse.clear();
        d.monitoring.clear();
        d.iup.clear();
        d.sources.clear();
        let world = PdfWorld::new(fill(
            DEFAULT_MONOGRAPHIE_TEMPLATE,
            &monograph_values(&d, &[], &[]),
        ));
        assert!(typst::compile::<PagedDocument>(&world).output.is_ok());
    }

    #[test]
    fn default_template_compiles_to_pdf() {
        // Hostile name: goes through the real escaping path, must
        // neither restyle the sheet nor break compilation.
        let patient = Patient {
            id: 1,
            last_name: "#eval \"Dupont\" \\ *gras*".to_owned(),
            first_name: "Jean".to_owned(),
            birth_date: "1958-07-03".to_owned(),
            ..Default::default()
        };
        let filled = fill_interview_template(
            DEFAULT_TEMPLATE,
            &InterviewPaper {
                patient: &patient,
                kind: InterviewKind::Bpm,
                date: "22/08/2026",
                age: Some(68),
                theme: "Initiation / bon usage",
                // The signature goes through the same escaping.
                signature: "Claire #strike[Leroy]",
                treats: &sample_treatments(),
                checklist: crate::entretien::checklist("Initiation / bon usage"),
            },
            &sample_pharmacy(),
        );
        let world = PdfWorld::new(filled);
        let document: PagedDocument = typst::compile(&world)
            .output
            .expect("le modèle par défaut doit compiler");
        // The fiche is handed over as one sheet: the boxes are sized so
        // that the treatments and the checklist fit above them.
        assert_eq!(
            document.pages.len(),
            1,
            "la fiche d'entretien tient sur une page"
        );
        let pdf = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
            .expect("l'export PDF doit réussir");
        assert!(pdf.starts_with(b"%PDF-"));
        assert!(pdf.len() > 1000);
        // For manual inspection: BPM_CADDY_TEST_PDF_OUT=/some/dir cargo test
        if let Ok(dir) = std::env::var("BPM_CADDY_TEST_PDF_OUT") {
            let _ = std::fs::write(std::path::Path::new(&dir).join("fiche_exemple.pdf"), &pdf);
        }
    }

    /// **La feuille d'un TROD lit la personne et l'acte**, et ne
    /// suppose rien : les lignes du protocole sont celles qui
    /// s'appliquent à ce dossier, le résultat enregistré arrive coché,
    /// et un homme au comptoir d'une cystite ne se voit proposer aucune
    /// ligne — la feuille dit d'orienter.
    #[test]
    fn the_trod_sheet_reads_the_person_and_the_recorded_result() {
        let offers = crate::ordonnance::starter("cystite");
        let none = crate::content::Overrides::default();
        let woman = Patient {
            id: 3,
            last_name: "Martin".to_owned(),
            first_name: "Léa".to_owned(),
            birth_date: "1990-02-11".to_owned(),
            sex: "F".to_owned(),
            ..Default::default()
        };
        let man = Patient {
            sex: "M".to_owned(),
            ..woman.clone()
        };
        let paper = |p: &'static Patient, result: &'static str| TrodPaper {
            patient: p,
            kind: InterviewKind::TrodCystite,
            date: "24/08/2026",
            age: crate::db::age_on(&p.birth_date, "2026-08-24"),
            result,
            signature: "Claire Leroy",
            treats: &[],
            offers: &offers,
            pregnant: false,
            content: &none,
        };
        let woman: &'static Patient = Box::leak(Box::new(woman));
        let man: &'static Patient = Box::leak(Box::new(man));
        let her = trod_source(
            &paper(woman, crate::ordonnance::NEGATIF),
            &sample_pharmacy(),
            std::path::Path::new(""),
        );
        assert!(her.contains(&bind_french("Fosfomycine trométamol 3 g (Monuril)")));
        assert!(her.contains(&typst_str("36 ans")));
        assert!(!her.contains("{{"), "un marqueur est resté");
        // Pas de score pour la cystite : la section n'est pas imprimée.
        assert!(!her.contains(&typst_str(crate::strings::tr("trod_pdf_score"))));
        // Le négatif enregistré est la seule case pleine du résultat.
        assert_eq!(her.matches("fill: black").count(), 1);
        let him = trod_source(
            &paper(man, ""),
            &sample_pharmacy(),
            std::path::Path::new(""),
        );
        assert!(
            !him.contains("Fosfomycine"),
            "une ligne réservée aux femmes"
        );
        assert!(him.contains(&typst_str(crate::strings::tr("trod_pdf_no_line"))));
        assert_eq!(
            him.matches("fill: black").count(),
            0,
            "rien d'enregistré, rien de coché"
        );
        // **Le courrier dit le résultat enregistré, et rien d'autre** :
        // négatif, sa conclusion ; rien de lu, des lignes à remplir.
        let letter = |p: &TrodPaper| {
            fill(
                DEFAULT_TROD_CR_TEMPLATE,
                &trod_letter_values(p, &sample_pharmacy()),
            )
        };
        let sheet = crate::trod::sheet(InterviewKind::TrodCystite).unwrap();
        let negative = letter(&paper(woman, crate::ordonnance::NEGATIF));
        assert!(negative.contains(&typst_str(sheet.letter_negative)));
        assert!(!negative.contains(&typst_str(sheet.letter_positive)));
        let unread = letter(&paper(woman, ""));
        assert!(unread.contains(&typst_str(crate::strings::tr("trod_letter_unread"))));
        assert!(!unread.contains(&typst_str(sheet.letter_negative)));
        assert!(!unread.contains(&typst_str(sheet.letter_positive)));
        // L'angine, la plus longue des deux (score, six lignes) : c'est
        // l'aperçu de l'éditeur, et il doit tenir sur une page lui aussi.
        let angine = fill(DEFAULT_TROD_TEMPLATE, &sample_values("trod"));
        for (n, source) in [her, him, angine, negative, unread].into_iter().enumerate() {
            let world = PdfWorld::new(source);
            let document: PagedDocument = typst::compile(&world)
                .output
                .expect("la feuille de TROD doit compiler");
            assert_eq!(
                document.pages.len(),
                1,
                "une feuille de TROD tient sur une page"
            );
            if let Ok(dir) = std::env::var("BPM_CADDY_TEST_PDF_OUT") {
                let pdf = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default()).unwrap();
                let _ = std::fs::write(
                    std::path::Path::new(&dir).join(format!("trod_exemple_{n}.pdf")),
                    &pdf,
                );
            }
        }
    }

    /// **La fiche de vaccination tient sur une page, et liste ce que le
    /// calendrier doit** — les doses dues comme les questions — sans
    /// qu'un nom hostile ne la casse.
    #[test]
    fn the_vaccination_sheet_lists_what_is_owed_on_one_page() {
        let patient = Patient {
            id: 4,
            last_name: "#eval \"Durand\"".to_owned(),
            first_name: "Anne".to_owned(),
            birth_date: "1950-03-02".to_owned(),
            ..Default::default()
        };
        let due = crate::vaccines::due_lines_with(&patient.birth_date, "2026-10-15", &[], "");
        let none = crate::content::Overrides::default();
        let paper = VaccinationPaper {
            patient: &patient,
            date: "15/10/2026",
            age: Some(76),
            signature: "Claire Leroy",
            treats: &[],
            due: &due,
            content: &none,
        };
        let source = vaccination_source(&paper, &sample_pharmacy(), std::path::Path::new(""));
        assert!(!source.contains("{{"));
        let owed = due
            .iter()
            .filter(|l| l.level != crate::vaccines::DueLevel::Ok)
            .count();
        assert!(owed > 0, "l'exemple doit devoir quelque chose");
        assert_eq!(
            source.matches("- #").count() - 1,
            owed,
            "une ligne par dose due ou question"
        );
        let world = PdfWorld::new(source);
        let document: PagedDocument = typst::compile(&world)
            .output
            .expect("la fiche de vaccination doit compiler");
        assert_eq!(document.pages.len(), 1);
        // À jour : la phrase qui le dit, pas une section vide.
        let paper = VaccinationPaper { due: &[], ..paper };
        let source = vaccination_source(&paper, &sample_pharmacy(), std::path::Path::new(""));
        assert!(source.contains(&typst_str(crate::strings::tr("vacc_pdf_due_none"))));
    }

    /// **La liasse d'un TROD : la feuille, puis le courrier** — deux
    /// pages, et pas le bilan de médication d'un entretien.
    #[test]
    fn the_trod_bundle_is_the_sheet_then_the_letter() {
        let patient = sample_patient();
        let none = crate::content::Overrides::default();
        let offers = crate::ordonnance::starter("angine");
        let paper = TrodPaper {
            patient: &patient,
            kind: InterviewKind::TrodAngine,
            date: "24/09/2026",
            age: Some(68),
            result: crate::ordonnance::POSITIF,
            signature: "Claire Leroy",
            treats: &[],
            offers: &offers,
            pregnant: false,
            content: &none,
        };
        let path = std::path::Path::new("");
        let parts = [
            trod_source(&paper, &sample_pharmacy(), path),
            trod_letter_source(&paper, &sample_pharmacy(), path),
        ];
        let world = PdfWorld::new(bundle_source(&parts).unwrap());
        let document: PagedDocument = typst::compile(&world)
            .output
            .expect("la liasse du TROD doit compiler");
        assert_eq!(document.pages.len(), 2, "une feuille, un courrier");
    }

    #[test]
    fn template_check_accepts_default_and_reports_errors() {
        assert!(check_doc("fiche", DEFAULT_TEMPLATE).is_ok());
        let err = check_doc("fiche", "#broken(").unwrap_err();
        assert!(err.contains("compilation Typst"));
    }

    #[test]
    fn conversion_tables_compile_to_pdf() {
        // A team edit prints in place of the shipped value.
        let mut edits: TableEdits = std::collections::HashMap::new();
        edits.insert(
            (crate::tables::TABLES[0].short.to_owned(), 0, 1),
            "20 mg (protocole interne)".to_owned(),
        );
        let source = fill(DEFAULT_TABLES_TEMPLATE, &conversion_tables_values(&edits));
        assert!(source.contains("protocole interne"));
        let world = PdfWorld::new(source);
        let document: PagedDocument = typst::compile(&world)
            .output
            .expect("les tables de conversion doivent compiler");
        let pdf = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
            .expect("l'export PDF doit réussir");
        assert!(pdf.starts_with(b"%PDF-"));
        if let Ok(dir) = std::env::var("BPM_CADDY_TEST_PDF_OUT") {
            let _ = std::fs::write(std::path::Path::new(&dir).join("tables_exemple.pdf"), &pdf);
        }
    }

    #[test]
    fn cr_letter_compiles_with_treatments_and_hostile_names() {
        assert!(check_doc("cr", DEFAULT_CR_TEMPLATE).is_ok());
        assert!(check_doc("cr", "#broken(").is_err());
        // Hostile patient name through the real fill path.
        let mut patient = sample_patient();
        patient.last_name = "#eval \"X\" *gras*".to_owned();
        patient.physician = "Dr #strike[Y]".to_owned();
        let filled = fill_cr_template(
            DEFAULT_CR_TEMPLATE,
            &patient,
            InterviewKind::Prevention,
            "24/08/2026",
            "Prévention — #eval \"Z\"",
            &sample_treatments(),
            &sample_pharmacy(),
            "Claire #strike[Leroy]",
            // Un point tapé à la main passe par le même échappement que
            // le reste : c'est du texte libre, donc c'est là que le
            // balisage entrerait s'il devait entrer quelque part.
            &["Sommeil — #eval \"W\"", "Vaccinations"],
        );
        assert!(!filled.contains("#eval \"W\"]"));
        // Les points cochés remplacent le cadre vide ; sans eux il reste.
        assert!(filled.contains("Vaccinations"));
        assert!(cr_points_markup(&[]).contains("7cm"));
        assert!(!cr_points_markup(&["Sommeil"]).contains("7cm"));
        let world = PdfWorld::new(filled);
        let document: PagedDocument = typst::compile(&world)
            .output
            .expect("le courrier CR doit compiler");
        let pdf = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
            .expect("l'export PDF doit réussir");
        assert!(pdf.starts_with(b"%PDF-"));
        if let Ok(dir) = std::env::var("BPM_CADDY_TEST_PDF_OUT") {
            let _ = std::fs::write(std::path::Path::new(&dir).join("cr_exemple.pdf"), &pdf);
        }
        // An empty treatments list renders the placeholder line.
        assert!(treatments_markup(&[]).contains("aucun traitement"));
    }

    #[test]
    fn appointment_list_compiles_even_with_hostile_names() {
        let rdvs = [
            Appointment {
                id: 0,
                time: String::new(),
                patient_id: 1,
                patient_name: "Jean #eval \"Dupont\" \\ *gras*".to_owned(),
                phone: "06 12 34 56 78".to_owned(),
                kind: InterviewKind::Bpm,
                date: "2026-09-01".to_owned(),
                remote: false,
                duration_minutes: 0,
                operator: String::new(),
            },
            Appointment {
                id: 2,
                time: "09:30".to_owned(),
                patient_id: 2,
                patient_name: "Hélène Lefèvre".to_owned(),
                phone: String::new(),
                kind: InterviewKind::Aod,
                date: "2026-09-03".to_owned(),
                remote: false,
                duration_minutes: 45,
                operator: "CL".to_owned(),
            },
        ];
        let source = fill(
            DEFAULT_RDV_TEMPLATE,
            &appointment_list_values(&rdvs, "23/08/2026"),
        );
        let world = PdfWorld::new(source);
        let document: PagedDocument = typst::compile(&world)
            .output
            .expect("la liste de RDV doit compiler");
        let pdf = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
            .expect("l'export PDF doit réussir");
        assert!(pdf.starts_with(b"%PDF-"));
        if let Ok(dir) = std::env::var("BPM_CADDY_TEST_PDF_OUT") {
            let _ = std::fs::write(std::path::Path::new(&dir).join("rdv_exemple.pdf"), &pdf);
        }
    }

    #[test]
    fn the_ordonnance_compiles_with_every_block_and_escapes_hostile_text() {
        let lines = [
            crate::ordonnance::Line {
                name: "Amoxicilline 1 g".to_owned(),
                // Markup typed into the free posology must not restyle
                // the page or break the compile.
                posology: "1 g x2/j #box[*6 jours*]".to_owned(),
                caution: "Vérifier l'absence d'allergie.".to_owned(),
            },
            crate::ordonnance::Line {
                name: "Saccharomyces boulardii 200 mg".to_owned(),
                posology: "1 gélule deux fois par jour".to_owned(),
                caution: String::new(),
            },
        ];
        let advice = [
            "Boire fréquemment.".to_owned(),
            "Aller au bout du traitement.".to_owned(),
        ];
        let source = fill_ordonnance_template(
            DEFAULT_ORDONNANCE_TEMPLATE,
            &sample_patient(),
            &sample_pharmacy(),
            "Angine à streptocoque du groupe A — TROD positif",
            "26/08/2026",
            &lines,
            &advice,
            "Claire Leroy, Pharmacien titulaire",
            ("Cadre de la dispensation", "Reconsulter si aggravation."),
        );
        assert!(source.contains("3400123"), "le N° AM doit figurer");
        assert!(
            source.contains("Claire Leroy, Pharmacien titulaire"),
            "l'ordonnance est signée par qui l'a faite"
        );
        assert!(source.contains("Reconsulter si aggravation."));
        assert!(!source.contains("{{"), "un marqueur n'a pas été remplacé");
        let world = PdfWorld::new(source);
        let document: PagedDocument = typst::compile(&world)
            .output
            .expect("l'ordonnance doit compiler");
        let pdf = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
            .expect("l'export PDF doit réussir");
        assert!(pdf.starts_with(b"%PDF-"));
        if let Ok(dir) = std::env::var("BPM_CADDY_TEST_PDF_OUT") {
            let _ = std::fs::write(
                std::path::Path::new(&dir).join("ordonnance_exemple.pdf"),
                &pdf,
            );
        }
    }

    #[test]
    fn the_guide_prints_on_one_sheet() {
        for (title, body) in GUIDE_SECTIONS {
            assert!(!title.trim().is_empty());
            assert!(
                body.trim().len() > 80,
                "section « {title} » trop courte pour dire quoi que ce soit"
            );
        }
        let source = fill(DEFAULT_GUIDE_TEMPLATE, &guide_values(&sample_pharmacy()));
        assert!(source.contains("mode d'emploi"));
        let world = PdfWorld::new(source);
        let document: PagedDocument = typst::compile(&world)
            .output
            .expect("le mode d'emploi doit compiler");
        // Two columns on A4: it is a handout, not a manual.
        assert!(
            document.pages.len() <= 2,
            "le mode d'emploi tient sur une feuille recto-verso au plus, ici {} pages",
            document.pages.len()
        );
        let pdf = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
            .expect("l'export PDF doit réussir");
        if let Ok(dir) = std::env::var("BPM_CADDY_TEST_PDF_OUT") {
            let _ = std::fs::write(std::path::Path::new(&dir).join("mode_emploi.pdf"), &pdf);
        }
    }

    /// La liste de contrôle se travaille dans le placard : elle porte le
    /// solde du registre, une colonne pour ce qu'on trouve, et une
    /// signature. Et aucun nom de patient — c'est une feuille qui reste
    /// sur une paillasse.
    #[test]
    fn the_stock_check_sheet_can_be_worked_through() {
        use crate::ordonnancier::{ToCheck, Why};
        let rows = vec![
            ToCheck {
                id: 1,
                label: "Skenan #eval \"x\" LP 30 mg".to_owned(),
                unit: "gélule".to_owned(),
                stock: -2.0,
                why: Why::Negative,
                days: Some(28),
            },
            ToCheck {
                id: 2,
                label: "Méthadone 40 mg".to_owned(),
                unit: "gélule".to_owned(),
                stock: 7.0,
                why: Why::Uncounted,
                days: None,
            },
        ];
        let source = fill(
            DEFAULT_CONTROLE_TEMPLATE,
            &stock_check_values(&rows, &sample_pharmacy(), "2026-08-29"),
        );
        assert!(source.contains("Contrôle des stupéfiants"));
        assert!(source.contains("29/08/2026"), "la date se lit en français");
        // Le solde du registre est imprimé en face : recompter tout un
        // placard sans savoir ce qu'on cherche est ce qui fait qu'on ne
        // le fait pas.
        assert!(source.contains("Au registre"));
        assert!(source.contains("Compté"));
        assert!(source.contains("Observation"));
        assert!(source.contains("stroke: 0.6pt"), "une case à cocher");
        assert!(source.contains("jamais"), "jamais compté se dit");
        assert!(source.contains("Signature"));
        // Rien de ce qui vient de la base n'est du code Typst.
        assert!(!source.contains("#eval \"x\"]"));
        let world = PdfWorld::new(source);
        let document: PagedDocument = typst::compile(&world)
            .output
            .expect("la liste de contrôle doit compiler");
        let pdf = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
            .expect("l'export PDF doit réussir");
        assert!(pdf.starts_with(b"%PDF-"));
        if let Ok(dir) = std::env::var("BPM_CADDY_TEST_PDF_OUT") {
            let _ = std::fs::write(
                std::path::Path::new(&dir).join("controle_stock_exemple.pdf"),
                &pdf,
            );
        }
        // Rien à compter compile aussi : le bouton n'apparaît que
        // lorsqu'il y a quelque chose, mais la fonction n'en dépend pas.
        let world = PdfWorld::new(fill(
            DEFAULT_CONTROLE_TEMPLATE,
            &stock_check_values(&[], &sample_pharmacy(), "2026-08-29"),
        ));
        assert!(typst::compile::<PagedDocument>(&world).output.is_ok());
    }

    /// **Chaque carnet de suivi compile, et porte son protocole.**
    ///
    /// La grille n'est pas ce qui compte : c'est le protocole, la cible
    /// et la ligne d'alerte, et une feuille qui les perdrait à
    /// l'impression serait le papier que l'officine donne déjà. Les six
    /// sont éprouvées, avec et sans nom de patient — une feuille vierge
    /// se donne aussi bien, et c'est le cas courant au comptoir.
    #[test]
    fn every_self_monitoring_sheet_carries_its_protocol_to_paper() {
        for shipped in crate::selfcheck::SHEETS {
            // Le test porte sur ce qui est livré : la résolution sans
            // réécriture rend la feuille mot pour mot — `selfcheck` le
            // tient de son côté.
            let sheet = &crate::selfcheck::resolve(shipped, &crate::content::Overrides::default());
            let source = fill(
                DEFAULT_SUIVI_TEMPLATE,
                &selfcheck_values(
                    sheet,
                    Some("Jean #eval \"x\" Dupont"),
                    &sample_pharmacy(),
                    "09/09/2026",
                ),
            );
            assert!(
                source.contains(&bind_french(&sheet.title)),
                "{} : sans titre",
                sheet.key
            );
            for step in &sheet.protocol {
                // La ponctuation française passe par `typst_str` ; on
                // vérifie le début de la consigne, qui suffit à dire
                // qu'elle est là.
                let head: String = step.chars().take(24).collect();
                assert!(
                    source.contains(&bind_french(&head)),
                    "{} : la consigne « {head}… » n'atteint pas le papier",
                    sheet.key
                );
            }
            assert!(source.contains("À signaler sans attendre"));
            assert!(source.contains("Objectif."));
            for label in &sheet.totals {
                let head: String = label.chars().take(20).collect();
                assert!(
                    source.contains(&head),
                    "{} : « {head}… » n'atteint pas le papier",
                    sheet.key
                );
            }
            assert!(source.contains("09/09/2026"));
            // Rien de ce qui vient de la base n'est du code Typst.
            assert!(!source.contains("#eval \"x\"]"));
            let world = PdfWorld::new(source);
            let document: PagedDocument = typst::compile(&world)
                .output
                .unwrap_or_else(|e| panic!("« {} » doit compiler : {e:?}", sheet.title));
            let pdf = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
                .expect("l'export PDF doit réussir");
            assert!(pdf.starts_with(b"%PDF-"));
            if let Ok(dir) = std::env::var("BPM_CADDY_TEST_PDF_OUT") {
                let _ = std::fs::write(
                    std::path::Path::new(&dir).join(format!("carnet_{}.pdf", sheet.key)),
                    &pdf,
                );
            }
        }
        // Sans dossier ouvert, la feuille porte une ligne à remplir et
        // non un nom vide entre deux tirets : c'est le cas courant, on
        // en donne une au comptoir sans ouvrir de dossier.
        let shipped0 = crate::selfcheck::resolve(
            &crate::selfcheck::SHEETS[0],
            &crate::content::Overrides::default(),
        );
        let blank = fill(
            DEFAULT_SUIVI_TEMPLATE,
            &selfcheck_values(&shipped0, None, &sample_pharmacy(), "09/09/2026"),
        );
        assert!(blank.contains("Nom :"), "{blank}");
        let world = PdfWorld::new(blank);
        assert!(typst::compile::<PagedDocument>(&world).output.is_ok());
        // Un nom d'espaces est un nom absent, pas un nom.
        let spaces = fill(
            DEFAULT_SUIVI_TEMPLATE,
            &selfcheck_values(&shipped0, Some("   "), &sample_pharmacy(), "09/09/2026"),
        );
        assert!(spaces.contains("Nom :"));
    }

    /// Le procès-verbal de destruction : ce qu'on s'apprête à détruire,
    /// et devant qui.
    ///
    /// C'est la pièce que la ligne du registre cite, et sans laquelle
    /// une destruction ne prouve rien : elle sort **avant** la
    /// destruction, on coche à mesure, et les deux signatures se posent
    /// au bas. Aucun nom de patient — une feuille qui sort du logiciel
    /// se pose sur une paillasse et se garde dix ans ; le numéro de
    /// dossier de chaque retour est au registre, qui est l'endroit pour
    /// cela.
    #[test]
    fn the_destruction_sheet_carries_what_makes_it_provable() {
        use crate::ordonnancier::Awaiting;
        let rows = vec![
            Awaiting {
                id: 1,
                label: "Skenan #eval \"x\" LP 30 mg".to_owned(),
                unit: "gélule".to_owned(),
                quantity: 14.0,
                since: "2025-11-02".to_owned(),
                // **Compté, pas recopié.** Il y avait 311 ici pour un
                // écart qui en fait 310 : du 02/11/2025 au 08/09/2026.
                // Un exemple faux sur un procès-verbal enseigne une
                // ancienneté fausse, et c'est la pièce qui prouve
                // depuis combien de temps un stupéfiant rapporté attend
                // sa destruction.
                days: crate::date::days_between("2025-11-02", "2026-09-08"),
            },
            // Ce qui attend sans qu'on sache depuis quand : la colonne
            // reste vide plutôt que d'inventer un jour.
            Awaiting {
                id: 2,
                label: "Oxycontin LP 10 mg".to_owned(),
                unit: "comprimé".to_owned(),
                quantity: 12.0,
                since: String::new(),
                days: None,
            },
        ];
        let source = fill(
            DEFAULT_DESTRUCTION_TEMPLATE,
            &destruction_list_values(&rows, &sample_pharmacy(), "2026-09-08"),
        );
        assert!(source.contains("Procès-verbal de destruction"));
        assert!(source.contains("08/09/2026"), "la date se lit en français");
        assert!(
            source.contains("02/11/2025 (310 j)"),
            "et l'ancienneté avec, comptée et non recopiée"
        );
        assert!(source.contains("Au coffre"));
        assert!(source.contains("Détruit"), "une colonne à cocher à mesure");
        assert!(source.contains("stroke: 0.6pt"), "une case à cocher");
        assert!(
            source.contains("Le témoin"),
            "une destruction se fait devant quelqu'un"
        );
        assert!(source.contains("Procédé"));
        assert!(
            !source.contains("dossier"),
            "aucun numéro de dossier : cette feuille traîne sur une paillasse"
        );
        // Rien de ce qui vient de la base n'est du code Typst.
        assert!(!source.contains("#eval \"x\"]"));
        let world = PdfWorld::new(source);
        let document: PagedDocument = typst::compile(&world)
            .output
            .expect("le procès-verbal doit compiler");
        let pdf = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
            .expect("l'export PDF doit réussir");
        assert!(pdf.starts_with(b"%PDF-"));
        if let Ok(dir) = std::env::var("BPM_CADDY_TEST_PDF_OUT") {
            let _ = std::fs::write(
                std::path::Path::new(&dir).join("proces_verbal_destruction_exemple.pdf"),
                &pdf,
            );
        }
        // Un coffre vide compile aussi : le bouton ne s'active que
        // lorsqu'il y a quelque chose, mais la fonction n'en dépend pas.
        let world = PdfWorld::new(fill(
            DEFAULT_DESTRUCTION_TEMPLATE,
            &destruction_list_values(&[], &sample_pharmacy(), "2026-09-08"),
        ));
        assert!(typst::compile::<PagedDocument>(&world).output.is_ok());
    }

    /// The call list is worked through at the telephone: it must carry
    /// the number, say why in three words, and leave somewhere to write.
    #[test]
    fn the_call_list_can_be_worked_through() {
        let rows = vec![
            CallRow {
                name: "Jean #eval \"x\" Dupont",
                phone: "06 01 02 03 04",
                tag: "2 alerte(s)",
                reason: "Kaliémie élevée sous IEC.",
            },
            CallRow {
                name: "Claire Martin",
                phone: "",
                tag: "1 à refaire",
                reason: "ALAT — dernier résultat il y a 30 mois, demandé par Tahor",
            },
        ];
        let source = fill(
            DEFAULT_APPELS_TEMPLATE,
            &call_list_values(&rows, "29/08/2026", &sample_pharmacy()),
        );
        assert!(source.contains("Liste d'appel"));
        assert!(source.contains("06 01 02 03 04"));
        // A tick box and a column to write in: without them it is a
        // list one reads, not a list one works through.
        assert!(source.contains("[*Réponse*]"));
        assert!(source.contains("stroke: 0.6pt"));
        assert!(!source.contains("#eval \"x\"]"));
        let world = PdfWorld::new(source);
        let document: PagedDocument = typst::compile(&world)
            .output
            .expect("la liste d'appel doit compiler");
        let pdf = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
            .expect("l'export PDF doit réussir");
        assert!(pdf.starts_with(b"%PDF-"));
        if let Ok(dir) = std::env::var("BPM_CADDY_TEST_PDF_OUT") {
            let _ = std::fs::write(
                std::path::Path::new(&dir).join("liste_appel_exemple.pdf"),
                &pdf,
            );
        }
        // Nothing to call about still compiles: the button only appears
        // when there is, but the function must not depend on that.
        let world = PdfWorld::new(fill(
            DEFAULT_APPELS_TEMPLATE,
            &call_list_values(&[], "29/08/2026", &sample_pharmacy()),
        ));
        assert!(typst::compile::<PagedDocument>(&world).output.is_ok());
    }

    /// The prescriber's copy of a conciliation: it carries what was
    /// reconducted as well as what changed, it names who it is for, it
    /// leaves a box for the answer, and it escapes what was pasted into
    /// the app from a hospital sheet nobody wrote by hand.
    #[test]
    fn the_conciliation_sheet_says_what_did_not_change_too() {
        let patient = Patient {
            physician: "Dr Morel".to_owned(),
            ..sample_patient()
        };
        let data = ConciliationData {
            patient: &patient,
            today: "29/08/2026",
            physician: &patient.physician,
            rows: vec![
                (
                    "NON RAPPROCHÉ".to_owned(),
                    "Zorglub #eval \"x\" lyoc".to_owned(),
                    String::new(),
                    String::new(),
                    "Ligne non retrouvée dans la base : à vérifier à la main.".to_owned(),
                ),
                (
                    "REMPLACÉ".to_owned(),
                    "Coversyl remplacé par Acuitel".to_owned(),
                    "5 mg le matin".to_owned(),
                    "5 mg le matin".to_owned(),
                    "Même classe (IEC).".to_owned(),
                ),
                (
                    "RECONDUIT".to_owned(),
                    "Lasilix".to_owned(),
                    "40 mg le matin".to_owned(),
                    "40 mg le matin".to_owned(),
                    String::new(),
                ),
            ],
            summary: "2 divergence(s) sur 3 ligne(s) comparée(s)",
            mention: "Il ne vaut pas avis médical.",
            signature: "Claire Leroy",
        };
        let source = fill(
            DEFAULT_CONCILIATION_TEMPLATE,
            &conciliation_values(&data, &sample_pharmacy()),
        );
        assert!(source.contains("Conciliation médicamenteuse"));
        assert!(source.contains("Dr Morel"));
        // The reconduction is on the sheet: a list of changes alone says
        // nothing about the lines nobody looked at.
        assert!(source.contains("RECONDUIT"));
        assert!(source.contains("Avis du prescripteur"));
        // Pasted text is escaped like everything else.
        assert!(!source.contains("#eval \"x\"]"));
        let world = PdfWorld::new(source);
        let document: PagedDocument = typst::compile(&world)
            .output
            .expect("la conciliation doit compiler");
        let pdf = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
            .expect("l'export PDF doit réussir");
        assert!(pdf.starts_with(b"%PDF-"));
        if let Ok(dir) = std::env::var("BPM_CADDY_TEST_PDF_OUT") {
            let _ = std::fs::write(
                std::path::Path::new(&dir).join("conciliation_exemple.pdf"),
                &pdf,
            );
        }
        // Nothing compared yet still prints a sheet: the officine
        // sometimes sends the file's own ordonnance to be confirmed.
        let empty = ConciliationData {
            patient: &patient,
            today: "29/08/2026",
            physician: "",
            rows: Vec::new(),
            summary: "",
            mention: "",
            signature: "",
        };
        let world = PdfWorld::new(fill(
            DEFAULT_CONCILIATION_TEMPLATE,
            &conciliation_values(&empty, &sample_pharmacy()),
        ));
        assert!(typst::compile::<PagedDocument>(&world).output.is_ok());
    }

    /// The patient's copy: it must carry the missed-dose line, since
    /// that is the reason it exists, and escape like everything else.
    #[test]
    fn the_plan_is_written_for_the_patient() {
        let patient = sample_patient();
        let data = PlanData {
            patient: &patient,
            today: "27/08/2026",
            lines: vec![
                (
                    "Eliquis #eval \"x\"".to_owned(),
                    "Fibrillation atriale".to_owned(),
                    "1 comprimé matin et soir".to_owned(),
                    "Dans les 6 heures, sinon sauter la prise.".to_owned(),
                ),
                (
                    "Levothyrox".to_owned(),
                    "Thyroïde".to_owned(),
                    "1 comprimé le matin à jeun".to_owned(),
                    String::new(),
                ),
            ],
            mention: "Ce plan ne remplace pas votre ordonnance.",
            signature: "Claire Leroy",
        };
        // Par `plan_source` : c'est ce que la liasse et le bouton
        // appellent tous les deux.
        let source = plan_source(&data, &sample_pharmacy(), std::path::Path::new(""));
        assert!(source.contains("Plan de prise"));
        assert!(source.contains("Dans les 6 heures"));
        assert!(source.contains("Questions à poser"));
        assert!(!source.contains("#eval \"x\"]"));
        let world = PdfWorld::new(source);
        let document: PagedDocument = typst::compile(&world)
            .output
            .expect("le plan doit compiler");
        let pdf = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
            .expect("l'export PDF doit réussir");
        assert!(pdf.starts_with(b"%PDF-"));
        if let Ok(dir) = std::env::var("BPM_CADDY_TEST_PDF_OUT") {
            let _ = std::fs::write(std::path::Path::new(&dir).join("plan_exemple.pdf"), &pdf);
        }
        // A file with no treatment still prints the sheet one fills in
        // by hand.
        let empty = PlanData {
            patient: &patient,
            today: "27/08/2026",
            lines: Vec::new(),
            mention: "",
            signature: "",
        };
        let world = PdfWorld::new(fill(
            DEFAULT_PLAN_TEMPLATE,
            &plan_values(&empty, &sample_pharmacy()),
        ));
        assert!(typst::compile::<PagedDocument>(&world).output.is_ok());
    }

    /// **La grille ne remplace jamais la phrase**, et une prise
    /// hebdomadaire n'entre pas dans la journée.
    ///
    /// Les deux règles d'`intake.rs` vérifiées là où elles comptent :
    /// sur la feuille. Le module peut être parfait et la mise en page
    /// jeter ce qu'il a lu — c'est ce qui est arrivé à la première
    /// version, où « à la demande : si besoin » chassait « 1 comprimé »
    /// de la ligne.
    #[test]
    fn the_treatment_sheet_keeps_the_posology_it_reads() {
        let patient = sample_patient();
        let line = |name: &str, poso: &str| FicheLine {
            name: name.to_owned(),
            what: "Pour l'exemple".to_owned(),
            posology: poso.to_owned(),
            reading: crate::intake::read(poso),
            know: String::new(),
            missed: String::new(),
            watch: String::new(),
        };
        let posologies = [
            "1 comprimé matin et soir",
            "1 comprimé si besoin, 6 heures entre deux prises",
            "1 comprimé par semaine, le lundi matin",
            "1 comprimé par jour",
            "Dose adaptée à l'INR",
        ];
        let ordonnance = crate::renewal::Prescription {
            prescribed_on: "2026-08-24".to_owned(),
            duration_days: 30,
            renewals: 2,
            dispensed: 2,
            dispensed_on: "2026-09-23".to_owned(),
        };
        // **Les quatre visualisations rendent la même lecture.** Le
        // dessin change, les chiffres et les dates ne bougent pas —
        // c'est la règle de `renewal.rs`, vérifiée ici sur le papier
        // plutôt que sur le type.
        for viz in crate::renewal::Viz::ALL {
            let data = FicheData {
                patient: &patient,
                today: "23/09/2026",
                prescriber: "Docteur Martin #eval \"x\"",
                lines: posologies
                    .iter()
                    .map(|p| line("Médicament #eval \"x\"", p))
                    .collect(),
                stands: vec![FicheStand {
                    stand: crate::renewal::read(&ordonnance, "2026-09-23", 7),
                    prescription: ordonnance.clone(),
                    treatments: vec!["Médicament".to_owned()],
                }],
                viz,
                mention: "Ce document ne remplace pas votre ordonnance.",
                signature: "Claire Leroy",
            };
            let source = traitement_source(&data, &sample_pharmacy(), std::path::Path::new(""));
            // Chaque posologie du dossier passe en entier, quelle que
            // soit la façon dont la grille l'a lue.
            for p in posologies {
                assert!(
                    source.contains(&bind_french(p)),
                    "« {p} » a disparu de la feuille ({viz:?})"
                );
            }
            // La prise hebdomadaire est hors grille : sa phrase occupe
            // les quatre colonnes de la journée.
            assert!(
                source.contains("ne se prend pas tous les jours"),
                "la prise hebdomadaire doit sortir de la grille ({viz:?})"
            );
            // Et le rendez-vous est annoncé avant l'épuisement.
            assert!(
                source.contains("21/11/2026") && source.contains("14/11/2026"),
                "la date de fin et celle du rendez-vous ({viz:?})"
            );
            // Le balisage d'un nom hostile n'est jamais interprété.
            assert!(!source.contains("#eval \"x\"]"));
            let world = PdfWorld::new(source);
            let document: PagedDocument = typst::compile(&world)
                .output
                .unwrap_or_else(|_| panic!("la fiche doit compiler en {viz:?}"));
            let pdf = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
                .expect("l'export PDF doit réussir");
            assert!(pdf.starts_with(b"%PDF-"));
            if let Ok(dir) = std::env::var("BPM_CADDY_TEST_PDF_OUT") {
                let _ = std::fs::write(
                    std::path::Path::new(&dir).join(format!("traitement_{}.pdf", viz.key())),
                    &pdf,
                );
            }
        }

        // Un dossier sans traitement imprime quand même la feuille : le
        // vide est écrit en toutes lettres, et jamais un tableau vide
        // qui se lirait « rien à prendre ».
        let empty = FicheData {
            patient: &patient,
            today: "23/09/2026",
            prescriber: "",
            lines: Vec::new(),
            stands: Vec::new(),
            viz: crate::renewal::Viz::default(),
            mention: "",
            signature: "",
        };
        let source = traitement_source(&empty, &sample_pharmacy(), std::path::Path::new(""));
        assert!(source.contains("Aucun traitement"));
        let world = PdfWorld::new(source);
        assert!(typst::compile::<PagedDocument>(&world).output.is_ok());
    }

    /// The bilan gathers the whole file on one sheet, and every value
    /// on it goes through the same escaping as anywhere else.
    #[test]
    fn the_bilan_gathers_the_file_and_escapes_it() {
        let patient = sample_patient();
        let data = BilanData {
            patient: &patient,
            today: "26/08/2026",
            treatments: vec![
                (
                    "Eliquis".to_owned(),
                    "apixaban — AOD".to_owned(),
                    "5 mg x2/j".to_owned(),
                ),
                (
                    "Zithromax #eval \"x\"".to_owned(),
                    "azithromycine — macrolide".to_owned(),
                    "500 mg/j".to_owned(),
                ),
                // **Et les quatre traitements que les lectures citent.**
                // L'échantillon n'en portait que deux, et ses propres
                // lectures nommaient un AINS, un IEC, une statine et un
                // diurétique qui ne figuraient nulle part : une feuille
                // qui se contredit, et c'est celle que l'officine lit
                // dans l'éditeur de modèles pour comprendre la sienne.
                (
                    "Advil".to_owned(),
                    "ibuprofène — AINS".to_owned(),
                    "400 mg si douleur".to_owned(),
                ),
                (
                    "Coversyl".to_owned(),
                    "périndopril — IEC".to_owned(),
                    "5 mg/j".to_owned(),
                ),
                (
                    "Tahor".to_owned(),
                    "atorvastatine — statine".to_owned(),
                    "20 mg le soir".to_owned(),
                ),
                (
                    "Lasilix".to_owned(),
                    "furosémide — diurétique de l'anse".to_owned(),
                    "40 mg le matin".to_owned(),
                ),
            ],
            interactions: vec![(
                "Eliquis ↔ Zithromax".to_owned(),
                "Les macrolides augmentent l'exposition à l'apixaban.".to_owned(),
            )],
            review: vec![(
                "ALERTE".to_owned(),
                "Anticoagulant + AINS".to_owned(),
                "Le risque hémorragique digestif est multiplié.".to_owned(),
                "Eliquis · Advil".to_owned(),
            )],
            biology: vec![(
                "20/08/2026".to_owned(),
                "Kaliémie".to_owned(),
                "5,4 mmol/L".to_owned(),
                "élevé".to_owned(),
            )],
            findings: vec![("ALERTE".to_owned(), "Kaliémie élevée sous IEC.".to_owned())],
            // Du texte hostile ici aussi : cette section passe par
            // `typst_str` comme les autres, et c'est ce test qui le
            // prouve — une section ajoutée sans son échantillon
            // hostile est une section dont l'échappement n'est vérifié
            // par personne.
            elderly: vec![(
                "Advil #box[*6 jours*] — À éviter à cet âge (dès 75 ans)".to_owned(),
                "Hémorragie digestive et insuffisance rénale aiguë.".to_owned(),
                "À la place : Le paracétamol en première intention.".to_owned(),
            )],
            vaccines: vec!["dTP — rappel décennal attendu".to_owned()],
            watch: vec![
                (
                    "À REFAIRE".to_owned(),
                    "LDL-cholestérol".to_owned(),
                    "une fois par an".to_owned(),
                    "12/02/2024 (30 mois)".to_owned(),
                    "Tahor".to_owned(),
                ),
                (
                    "JAMAIS NOTÉ".to_owned(),
                    "Natrémie".to_owned(),
                    "tous les six mois".to_owned(),
                    "Aucun résultat noté au dossier.".to_owned(),
                    "Lasilix".to_owned(),
                ),
            ],
            acts: vec![(
                "20/08/2026".to_owned(),
                "BPM".to_owned(),
                "Observance".to_owned(),
                "Réalisé".to_owned(),
            )],
            signature: "Claire Leroy, pharmacien titulaire",
        };
        // Par `bilan_source`, qui est ce que la liasse et le bouton
        // appellent tous les deux : un test qui remplirait le modèle
        // lui-même prouverait que le test sait remplir.
        let source = bilan_source(&data, &sample_pharmacy(), std::path::Path::new(""));
        assert!(source.contains("Interactions repérées"));
        assert!(source.contains("Revue de l'ordonnance"));
        assert!(source.contains("Plan d'action"));
        // The only section of the bilan that speaks about what is *not*
        // on the file.
        assert!(source.contains("À faire vérifier"));
        assert!(source.contains("LDL-cholestérol"));
        // Hostile text goes in as a string literal, never as markup.
        assert!(!source.contains("#eval \"x\"]"));
        let world = PdfWorld::new(source);
        let document: PagedDocument = typst::compile(&world)
            .output
            .expect("le bilan doit compiler");
        let pdf = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
            .expect("l'export PDF doit réussir");
        assert!(pdf.starts_with(b"%PDF-"));
        if let Ok(dir) = std::env::var("BPM_CADDY_TEST_PDF_OUT") {
            let _ = std::fs::write(std::path::Path::new(&dir).join("bilan_exemple.pdf"), &pdf);
        }
    }

    /// An empty file still prints a usable sheet: the bilan is also the
    /// form one fills when there is nothing recorded yet.
    #[test]
    fn an_empty_file_still_prints_a_bilan() {
        let patient = sample_patient();
        let data = BilanData {
            patient: &patient,
            today: "26/08/2026",
            treatments: Vec::new(),
            interactions: Vec::new(),
            review: Vec::new(),
            biology: Vec::new(),
            findings: Vec::new(),
            elderly: Vec::new(),
            vaccines: Vec::new(),
            watch: Vec::new(),
            acts: Vec::new(),
            signature: "",
        };
        let world = PdfWorld::new(fill(
            DEFAULT_BILAN_TEMPLATE,
            &bilan_values(&data, &sample_pharmacy()),
        ));
        let document: PagedDocument = typst::compile(&world)
            .output
            .expect("un bilan vide doit compiler");
        assert!(typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
            .expect("l'export PDF doit réussir")
            .starts_with(b"%PDF-"));
    }

    /// The whole codex as a booklet: it must compile with the shipped
    /// preparations, and carry each one's formula and mise en garde.
    #[test]
    fn the_codex_prints_as_a_booklet() {
        let preparations: Vec<crate::db::Preparation> = crate::db::STARTER_PREPARATIONS
            .iter()
            .enumerate()
            .map(|(i, p)| crate::db::Preparation {
                id: i as i64 + 1,
                name: p.name.to_owned(),
                form: p.form.to_owned(),
                indication: p.indication.to_owned(),
                formula: p.formula.to_owned(),
                yield_amount: p.yield_amount.to_owned(),
                method: p.method.to_owned(),
                conservation: p.conservation.to_owned(),
                caution: p.caution.to_owned(),
                tags: p.tags.to_owned(),
                sources: p.sources.to_owned(),
            })
            .collect();
        let source = fill(DEFAULT_CODEX_TEMPLATE, &codex_values(&preparations));
        assert!(source.contains("Vaseline salicylée à 5 %"));
        assert!(source.contains("Mise en garde"));
        let world = PdfWorld::new(source);
        let document: PagedDocument = typst::compile(&world)
            .output
            .expect("le codex doit compiler");
        let pdf = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
            .expect("l'export PDF doit réussir");
        assert!(pdf.starts_with(b"%PDF-"));
        if let Ok(dir) = std::env::var("BPM_CADDY_TEST_PDF_OUT") {
            let _ = std::fs::write(std::path::Path::new(&dir).join("codex_exemple.pdf"), &pdf);
        }
        // An empty codex still prints its cover line rather than
        // failing: a base whose team deleted everything is legitimate.
        let world = PdfWorld::new(fill(DEFAULT_CODEX_TEMPLATE, &codex_values(&[])));
        assert!(typst::compile::<PagedDocument>(&world).output.is_ok());
    }

    /// The dispositifs print twice: one fiche for the drawer, and the
    /// whole set as a booklet grouped by family.
    #[test]
    fn the_dispositifs_print_as_a_sheet_and_as_a_booklet() {
        let all: Vec<crate::db::Dispositif> = crate::db::STARTER_DISPOSITIFS
            .iter()
            .enumerate()
            .map(|(i, d)| crate::db::Dispositif {
                id: i as i64 + 1,
                name: d.name.to_owned(),
                family: d.family.to_owned(),
                indication: d.indication.to_owned(),
                sizes: d.sizes.to_owned(),
                application: d.application.to_owned(),
                renewal: d.renewal.to_owned(),
                lpp: d.lpp.to_owned(),
                caution: d.caution.to_owned(),
                tags: d.tags.to_owned(),
                sources: d.sources.to_owned(),
            })
            .collect();
        let booklet = fill(
            DEFAULT_DISPOSITIFS_TEMPLATE,
            &dispositifs_values(&all, &sample_pharmacy()),
        );
        assert!(booklet.contains("Hydrocolloïde"));
        assert!(booklet.contains("PANSEMENT"));
        assert!(booklet.contains("Renouvellement"));
        let world = PdfWorld::new(booklet);
        let document: PagedDocument = typst::compile(&world)
            .output
            .expect("le livret des dispositifs doit compiler");
        let pdf = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
            .expect("l'export PDF doit réussir");
        assert!(pdf.starts_with(b"%PDF-"));
        if let Ok(dir) = std::env::var("BPM_CADDY_TEST_PDF_OUT") {
            let _ = std::fs::write(
                std::path::Path::new(&dir).join("dispositifs_exemple.pdf"),
                &pdf,
            );
        }
        // One fiche, with hostile input: everything the team types goes
        // through the same escaping as the rest.
        let one = crate::db::Dispositif {
            id: 1,
            name: "Pansement #[test] \"maison\"".to_owned(),
            family: "Pansement".to_owned(),
            indication: "Plaie propre.".to_owned(),
            sizes: "10 x 10 cm".to_owned(),
            application: "Sur peau sèche.".to_owned(),
            renewal: "Tous les deux jours.".to_owned(),
            lpp: "Titre I.".to_owned(),
            caution: "Pas sur plaie infectée.".to_owned(),
            tags: "pansement".to_owned(),
            sources: "Fiche de l'officine".to_owned(),
        };
        let sheet = fill(
            DEFAULT_DISPOSITIF_TEMPLATE,
            &dispositif_values(&one, &sample_pharmacy()),
        );
        let world = PdfWorld::new(sheet);
        assert!(typst::compile::<PagedDocument>(&world).output.is_ok());
        // An empty list still prints its cover rather than failing.
        let world = PdfWorld::new(fill(
            DEFAULT_DISPOSITIFS_TEMPLATE,
            &dispositifs_values(&[], &sample_pharmacy()),
        ));
        assert!(typst::compile::<PagedDocument>(&world).output.is_ok());
    }

    /// The fiche de fabrication is the record the bonnes pratiques ask
    /// for: it must carry the quantities actually weighed, and the
    /// blanks that are filled in by hand.
    #[test]
    fn the_fabrication_sheet_carries_the_weighed_quantities() {
        let prep = crate::db::Preparation {
            id: 1,
            // Hostile input goes through the same escaping as everywhere
            // else: a formula is written by the team.
            name: "Vaseline salicylée #eval \"x\" à 5 %".to_owned(),
            form: "Pommade".to_owned(),
            indication: "Kératolytique".to_owned(),
            formula: "Acide salicylique | 5 g\nVaseline blanche | qsp 100 g".to_owned(),
            yield_amount: "100 g".to_owned(),
            method: "Triturations successives.".to_owned(),
            conservation: "Pot opaque, trois mois.".to_owned(),
            caution: "Pas chez le nourrisson.".to_owned(),
            tags: "dermatologie".to_owned(),
            sources: "Formulaire National".to_owned(),
        };
        let lines = vec![
            (
                "Acide salicylique".to_owned(),
                "5 g".to_owned(),
                "3 g".to_owned(),
            ),
            (
                "Vaseline blanche".to_owned(),
                "qsp 100 g".to_owned(),
                "qsp 60 g".to_owned(),
            ),
        ];
        let source = fill(
            DEFAULT_PREPARATION_TEMPLATE,
            &preparation_values(&prep, "60 g", &lines, &sample_pharmacy(), "CL"),
        );
        assert!(
            source.contains("qsp 60 g"),
            "la quantité pesée doit figurer"
        );
        assert!(
            source.contains("N° de lot"),
            "la colonne des lots est le point de la fiche"
        );
        assert!(source.contains("Pharmacie du Centre"));
        let world = PdfWorld::new(source);
        let document: PagedDocument = typst::compile(&world)
            .output
            .expect("la fiche de fabrication doit compiler");
        let pdf = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
            .expect("l'export PDF doit réussir");
        assert!(pdf.starts_with(b"%PDF-"));
        if let Ok(dir) = std::env::var("BPM_CADDY_TEST_PDF_OUT") {
            let _ = std::fs::write(
                std::path::Path::new(&dir).join("fiche_fabrication_exemple.pdf"),
                &pdf,
            );
        }
    }

    /// With both toggles off there is no advice block at all — the
    /// ordonnance is the lines and nothing else.
    #[test]
    fn the_ordonnance_omits_the_advice_block_when_both_toggles_are_off() {
        let lines = [crate::ordonnance::Line {
            name: "Fosfomycine trométamol 3 g".to_owned(),
            posology: "3 g en dose unique".to_owned(),
            caution: String::new(),
        }];
        let source = fill_ordonnance_template(
            DEFAULT_ORDONNANCE_TEMPLATE,
            &sample_patient(),
            &sample_pharmacy(),
            "Cystite aiguë simple — test positif",
            "26/08/2026",
            &lines,
            &[],
            "Claire Leroy",
            ("", ""),
        );
        // Nothing configured, nothing printed: no stray italic line
        // under the title or under the signature.
        assert!(!source.contains("style: \"italic\""));
        assert!(!source.contains("Conseils"));
        let world = PdfWorld::new(source);
        let _: PagedDocument = typst::compile(&world)
            .output
            .expect("l'ordonnance doit compiler sans conseils");
    }

    #[test]
    fn the_default_ordonnance_template_passes_its_own_validation() {
        check_doc("ordonnance", DEFAULT_ORDONNANCE_TEMPLATE)
            .expect("le modèle par défaut doit compiler");
    }

    #[test]
    fn the_vaccination_carnet_compiles_and_reads_oldest_first() {
        let patient = sample_patient();
        let lines = vec![
            crate::db::Vaccination {
                id: 1,
                code: "GRIPPE".to_owned(),
                label: "Grippe saisonnière".to_owned(),
                given_on: "2025-10-14".to_owned(),
                lot: "FLU25-208".to_owned(),
                site: "Deltoïde G".to_owned(),
                operator: "CL".to_owned(),
                ..Default::default()
            },
            crate::db::Vaccination {
                id: 2,
                code: "DTP".to_owned(),
                // Markup in a hand-typed label must not restyle the
                // sheet: every value goes in as a string literal.
                label: "dTP #box[*injection*]".to_owned(),
                dose: "Rappel 45 ans".to_owned(),
                given_on: "2003-11-18".to_owned(),
                next_due: "2026-11-18".to_owned(),
                remark: "Carnet papier".to_owned(),
                ..Default::default()
            },
        ];
        let source = fill(
            DEFAULT_VACCINATION_TEMPLATE,
            &vaccination_carnet_values(&patient, &lines, "Mention de l'officine"),
        );
        // Oldest first on paper, whatever order the screen showed.
        let dtp = source.find("Rappel 45 ans").expect("le dTP doit figurer");
        let flu = source.find("FLU25-208").expect("la grippe doit figurer");
        assert!(
            dtp < flu,
            "le carnet imprimé se lit du plus ancien au plus récent"
        );
        // La ponctuation double est liée au mot qui la précède avant
        // d'entrer dans la page : c'est `typst_str` qui le fait, pour
        // tout le monde, et l'attendu doit donc passer par la même
        // fonction.
        assert!(source.contains(&bind_french("Prochaine : 18/11/2026 — Carnet papier")));
        assert!(source.contains(&bind_french("Mention de l'officine")));
        let world = PdfWorld::new(source);
        let document: PagedDocument = typst::compile(&world)
            .output
            .expect("le carnet de vaccination doit compiler");
        let pdf = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
            .expect("l'export PDF doit réussir");
        assert!(pdf.starts_with(b"%PDF-"));
        if let Ok(dir) = std::env::var("BPM_CADDY_TEST_PDF_OUT") {
            let _ = std::fs::write(
                std::path::Path::new(&dir).join("carnet_vaccination_exemple.pdf"),
                &pdf,
            );
        }
    }

    #[test]
    fn an_empty_carnet_still_produces_a_sheet() {
        // No mention configured: the page carries none, and still
        // compiles.
        let source = fill(
            DEFAULT_VACCINATION_TEMPLATE,
            &vaccination_carnet_values(&sample_patient(), &[], ""),
        );
        assert!(!source.contains("style: \"italic\""));
        let world = PdfWorld::new(source);
        let document: PagedDocument = typst::compile(&world)
            .output
            .expect("un carnet vide doit compiler");
        assert!(typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
            .expect("l'export PDF doit réussir")
            .starts_with(b"%PDF-"));
    }
}
