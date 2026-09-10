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

/// Default A4 interview sheet: patient header plus rounded boxes sized for
/// handwritten notes during the interview.
const DEFAULT_TEMPLATE: &str = r#"
#set page(paper: "a4", margin: 1.5cm)
#set text(size: 11pt)
#set block(spacing: 2.5mm)

#let note-box(title, h) = [
  #v(3mm)
  #text(weight: "bold")[#title]
  #v(1.5mm)
  #box(width: 100%, height: h, stroke: 0.8pt, radius: 5pt)
]

#align(center)[
  #text(17pt, weight: "bold")[Entretien pharmaceutique — {{KIND}}]
]
#v(4mm)

#box(width: 100%, stroke: 0.8pt, radius: 5pt, inset: 9pt)[
  #text(weight: "bold")[Patient :] {{PATIENT_NAME}} \
  #text(weight: "bold")[Date de naissance :] {{BIRTH_DATE}} \
  #text(weight: "bold")[Date de l'entretien :] {{DATE}} \
  #text(weight: "bold")[Thème :] {{THEME}}
]

#v(3mm)
#text(weight: "bold")[Traitements connus à l'officine]
#v(1.5mm)
{{TREATMENTS}}

#v(3mm)
#text(weight: "bold")[À couvrir pendant l'entretien]
#v(1.5mm)
{{CHECKLIST}}

#note-box("Ce que le patient dit", 2.2cm)
#note-box("Points d'attention / interactions", 2cm)
#note-box("Conclusion et plan d'action", 2.2cm)

#v(3mm)
#grid(columns: (1fr, 1fr), gutter: 1cm,
  [#text(weight: "bold")[Signature du pharmacien] \
   #text(9pt)[{{PHARMACIST}}]
   #v(1.5mm)
   #box(width: 100%, height: 1.6cm, stroke: 0.8pt, radius: 5pt)],
  [#text(weight: "bold")[Prochain rendez-vous]
   #v(1.5mm)
   #box(width: 100%, height: 1.6cm, stroke: 0.8pt, radius: 5pt)],
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
#box(width: 6.5cm, height: 2.2cm, stroke: 0.8pt, radius: 5pt)]
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

/// Compile the interview sheet for a patient and hand it to the OS PDF
/// viewer. `template_path` is [`crate::config::Config::template_path`]:
/// when the file does not exist, the embedded template is used.
#[allow(clippy::too_many_arguments)]
pub fn open_interview_sheet(
    patient: &Patient,
    kind: InterviewKind,
    today: &str,
    theme: &str,
    template_path: &std::path::Path,
    signature: &str,
    treats: &[Drug],
    checklist: &[&str],
) -> Result<PathBuf, String> {
    let template = if template_path.exists() {
        std::fs::read_to_string(template_path)
            .map_err(|e| format!("modèle {} illisible : {e}", template_path.display()))?
    } else {
        DEFAULT_TEMPLATE.to_owned()
    };
    let filled = fill_interview_template(
        &template, patient, kind, today, theme, signature, treats, checklist,
    );

    let stem = format!("fiche_{}_{}", patient.id, kind.as_str().to_lowercase());
    compile_and_open(filled, &stem)
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
    template
        .replace(
            "{{PHARMACY_NAME}}",
            &format!("#{}", typst_str(&pharmacy.name)),
        )
        .replace(
            "{{PHARMACY_ADDRESS}}",
            &format!("#{}", typst_str(&pharmacy.address)),
        )
        .replace(
            "{{PHARMACY_PHONE}}",
            &format!("#{}", typst_str(&pharmacy.phone)),
        )
        // Signed by whoever held the entretien, when the team list
        // knows those initials; by the officine's own line otherwise.
        .replace("{{PHARMACIST}}", &format!("#{}", typst_str(signature)))
        .replace("{{PHYSICIAN}}", &format!("#{}", typst_str(physician)))
        .replace(
            "{{PATIENT_NAME}}",
            &format!("#{}", typst_str(&patient.full_name())),
        )
        .replace(
            "{{BIRTH_DATE}}",
            &format!(
                "#{}",
                typst_str(&crate::db::format_french_date(&patient.birth_date))
            ),
        )
        .replace("{{KIND}}", &format!("#{}", typst_str(kind.label())))
        .replace("{{DATE}}", &format!("#{}", typst_str(date)))
        .replace(
            "{{THEME}}",
            // Un acte qui ne porte pas de thème n'en imprime pas, même
            // si la base en garde un : jusqu'à la 0.145 le thème armé
            // par le choix rapide était écrit sur les actes qui n'en ont
            // pas, et il ressortait ici. La source est corrigée ; ceci
            // couvre les lignes déjà écrites, sans réécrire la base.
            &format!(
                "#{}",
                typst_str(theme_or_dash(if kind.has_theme() { theme } else { "" }))
            ),
        )
        .replace("{{TREATMENTS}}", &treatments_markup(treats))
        // Ce qui a été retenu à l'export, ou le cadre vide.
        //
        // Vide veut dire vide : un courrier dont personne n'a coché de
        // point garde l'encadré qu'on remplit à la main, qui est ce que
        // le modèle portait avant que ce marqueur existe. Imprimer une
        // liste de points qu'on n'a pas choisis serait faire dire au
        // pharmacien ce qu'il n'a pas dit.
        .replace("{{POINTS}}", &cr_points_markup(points))
}

/// Les points retenus, ou l'encadré à remplir quand il n'y en a pas.
fn cr_points_markup(points: &[&str]) -> String {
    if points.is_empty() {
        return "#box(width: 100%, height: 7cm, stroke: 0.8pt, radius: 5pt)".to_owned();
    }
    let list = points
        .iter()
        .map(|p| format!("#block(below: 2mm)[— #{}]", typst_str(p)))
        .collect::<Vec<_>>()
        .join("\n");
    // L'encadré reste sous la liste, plus court : le médecin y répond,
    // et c'est la moitié de l'intérêt d'envoyer la feuille.
    format!("{list}\n#v(2mm)\n#box(width: 100%, height: 3.5cm, stroke: 0.8pt, radius: 5pt)")
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
            expected: Some(66_000),
        },
        crate::caisse::Counted {
            id: 2,
            day: "2026-09-07".to_owned(),
            cash: 20_450,
            other: 45_075,
            float_kept: 15_000,
            expected: Some(66_000),
        },
        crate::caisse::Counted {
            id: 3,
            day: "2026-09-08".to_owned(),
            cash: 31_200,
            other: 52_300,
            float_kept: 15_000,
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
    template_path: &std::path::Path,
) -> Result<PathBuf, String> {
    compile_and_open(
        fill(
            &template_source("monographie", template_path),
            &monograph_values(d, posologies),
        ),
        &format!("monographie_{}", d.id),
    )
}

fn monograph_values(d: &Drug, posologies: &[crate::db::Posologie]) -> Vec<(&'static str, String)> {
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
    if pk.iter().any(|(_, v)| !v.trim().is_empty()) {
        let mut rows = String::new();
        for (label, value) in pk {
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
        fill(
            &template_source("bilan", template_path),
            &bilan_values(data, pharmacy),
        ),
        &format!("bilan_{}", data.patient.id),
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
        src.push_str("#sec[Vaccinations à jour ?]\n");
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

#let sec(t) = [#v(3mm) #text(11pt, weight: "bold")[#t] #v(1mm) #line(length: 100%, stroke: 0.6pt) #v(1.5mm)]

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
                    "Un exemplaire près du poste, un dans le classeur."
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
const GUIDE_SECTIONS: &[(&str, &str)] = &[
    (
        "Ouvrir la base",
        "L'application demande le mot de passe de la base au démarrage : la base est chiffrée, et rien n'en sort. « Verrouiller » (en haut à droite) ferme l'écran sans quitter, et l'inactivité le fait toute seule au bout du délai réglé dans les Options. Une sauvegarde du jour est écrite à chaque déverrouillage, dans le dossier « backups » à côté de la base.",
    ),
    (
        "Trouver ou créer un patient",
        "L'application s'ouvre sur la recherche. Tapez ce que vous avez : « jndp » trouve Jean Dupont, les accents et la casse n'ont pas d'importance. Aucun résultat ? Le même champ devient le formulaire de création. Entrée ouvre le résultat choisi, Échap referme.",
    ),
    (
        "Le dossier patient",
        "Le bandeau du haut porte l'identité, les traitements rattachés au référentiel médicaments (une puce par médicament, cliquable), et ce que le dossier voit tout seul : les interactions repérées entre ces traitements, et la revue d'ordonnance. En dessous, six onglets : les entretiens, le carnet de vaccination, la biologie, les locations de matériel, la conciliation de sortie et les pièces numérisées.",
    ),
    (
        "Créer et suivre un entretien",
        "Ctrl+N ouvre le choix rapide : un chiffre par acte, le thème si vous en voulez un. La ligne créée se lit de gauche à droite — le code de l'acte et son rang dans la séquence, le thème, le jour où il a été fait (modifiable) et les initiales de qui l'a fait, l'état, puis « » » pour avancer d'un état. Un acte avance jusqu'à « Facturé » ; « « » revient en arrière si vous avez cliqué trop vite.",
    ),
    (
        "Ce que l'acte imprime",
        "Sur chaque ligne : « PDF » sort la fiche d'entretien à remplir, « CR » le courrier au médecin traitant avec les traitements connus, « Adhésion » le bulletin officiel de l'Assurance Maladie pré-rempli — les cases, la date et les signatures restent à faire devant le patient. Un TROD positif ouvre en plus l'ordonnance protocolisée.",
    ),
    (
        "Le bilan et le plan de prise",
        "En haut du dossier, « Bilan… » imprime le bilan partagé de médication avec ce que le dossier sait : traitements, interactions, revue d'ordonnance, biologie, vaccinations dues, actes de l'année, et les cadres à remplir pendant l'entretien. « Plan de prise… » imprime la feuille que le patient emporte : indication, posologie et conduite à tenir en cas d'oubli, médicament par médicament.",
    ),
    (
        "La biologie",
        "L'onglet « Biologie » enregistre les résultats : choisissez l'analyte, tapez la valeur, la date si ce n'est pas aujourd'hui. Chaque valeur est lue contre son intervalle usuel, et le panneau « Interprétation » la relit contre les traitements du dossier — une kaliémie à 5,4 n'a pas le même sens sous IEC. Cliquez le nom d'un analyte pour voir sa courbe.",
    ),
    (
        "Le carnet de vaccination",
        "Les doses reçues, avec le lot et le site. À côté, « À faire » compare le carnet au calendrier vaccinal et dit ce qui manque ; « Compléter le carnet… » inscrit d'un coup les doses dues, sans date, à corriger ligne par ligne. « Voyage » coche les vaccins recommandés pour les destinations notées au dossier.",
    ),
    (
        "Le référentiel médicaments (F3)",
        "Plus de huit cents fiches, deux lettres suffisent à en trouver une. La fiche s'ouvre comme une monographie imprimée ; les noms des autres médicaments y sont cliquables. À droite, la fiche technique repliable : demi-vie, élimination, adaptation rénale, grossesse. « Modifier » passe au formulaire — tout est modifiable, et ce que l'équipe écrit n'est jamais réécrit par une mise à jour.",
    ),
    (
        "Les tables, le codex, les protocoles",
        "Depuis les médicaments : « Tables de conversion » (les références de comptoir, chacune datée et sourcée, qu'une seule recherche traverse toutes), « Codex… » (les préparations de l'officine, avec la formule mise à la quantité prescrite et la fiche de fabrication), « Protocoles… » (les arbres de décision, à dérouler question par question au comptoir).",
    ),
    (
        "Chercher partout : « Aller à… » et « Dans le texte… »",
        "Ctrl+K ouvre une boîte au-dessus de tout : tapez trois lettres et elle rend les patients, les fiches, les tables, les préparations et les protocoles qui répondent, avec les flèches pour parcourir et Entrée pour ouvrir. Sa dernière ligne cherche le même mot dans le *texte* des fiches, où se trouve souvent la réponse. Le même bouton se trouve dans les médicaments sous « Dans le texte… » : « pamplemousse », « allaitement », « QT », et chaque fiche qui le dit revient avec la phrase qui le porte, mot surligné, la posologie et sa remarque comprises. Une fiche patient ouverte ? Un bouton limite la recherche à ses seuls traitements.",
    ),
    (
        "L'agenda et le carnet de transmissions",
        "F4 ouvre la semaine : un bloc par rendez-vous, la couleur dit l'acte, un clic ouvre le dossier. Le panneau du jour détaille les rendez-vous, les entrées qui ne sont pas des actes (formation, réunion, livraison, congé) et les notes du jour. F5 ouvre le carnet de transmissions : une page par jour, imprimable pour le classeur.",
    ),
    (
        "Le tableau de bord",
        "Ce qui a été facturé, ce qui attend, le taux horaire, la charge des 28 jours. « À revoir » est la liste d'appel : les dossiers dont la biologie ou l'ordonnance a quelque chose à dire. « Récapitulatif de facturation… » imprime les actes à facturer ; « Exporter CSV » écrit tout dans un fichier que le tableur ouvre sans rien demander.",
    ),
    (
        "Régler l'application",
        "« Options… » : l'identité de l'officine et l'équipe (les initiales signent les notes, le nom signe les documents), les mentions imprimées — vides par défaut, l'application n'ajoute aucun avertissement de son propre chef —, les honoraires par acte et par rang, les règles de quota, la base et les sauvegardes. « Modèles… » ouvre les sources des quatre documents à modèle — fiche d'entretien, courrier, carnet, ordonnance — modifiables avec aperçu.",
    ),
    (
        "Raccourcis",
        "Ctrl+K aller à… · Ctrl+F chercher un patient · Ctrl+N nouvel entretien · Ctrl+Tab onglet suivant · Ctrl+W fermer l'onglet · F1 panneau d'équipe · F3 médicaments · F4 agenda · F5 carnet · F6 liste de gauche · F7 carte vaccinale · F12 cette liste · Échap ferme ce qui est ouvert. Dans une liste — patients, protocoles, préparations, dispositifs — tapez dans son champ de recherche, puis les flèches parcourent et Entrée ouvre. Dates : 230826 donne 23/08/2026, 2308 donne le 23/08 de l'année utile.",
    ),
    (
        "En cas de doute",
        "Rien n'est décidé par l'application : elle propose, elle rappelle, elle calcule. Les intervalles de biologie sont ceux de l'adulte et celui du laboratoire prime ; les tables portent leur date de relecture et leurs sources ; les préparations ne se font que sur ordonnance et selon les bonnes pratiques. La base est partagée entre les postes : si un message dit qu'une ligne a changé ailleurs, relisez-la avant de réécrire.",
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
        fill(
            &template_source("plan", template_path),
            &plan_values(data, pharmacy),
        ),
        &format!("plan_{}", data.patient.id),
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
    sheet: &crate::selfcheck::Sheet,
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
    sheet: &crate::selfcheck::Sheet,
    patient: Option<&str>,
    pharmacy: &PharmacyConfig,
    today_french: &str,
) -> Vec<(&'static str, String)> {
    let mut src = String::new();
    src.push_str(&format!(
        "#align(center)[#text(17pt, weight: \"bold\")[#{}]]\n#v(1mm)\n",
        typst_str(sheet.title)
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
    src.push_str("#text(11pt, weight: \"bold\")[Comment faire]\n#v(1.5mm)\n");
    for (i, step) in sheet.protocol.iter().enumerate() {
        src.push_str(&format!(
            "#text(10pt)[*{}.* #{}]\\\n",
            i + 1,
            typst_str(step)
        ));
    }

    // --- Ce qu'on vise --------------------------------------------
    src.push_str(&format!(
        "#v(3mm)\n#block(width: 100%, inset: 6pt, stroke: 0.6pt)[#text(10pt)[*Ce qu'on vise.* #{} #box(width: 5cm, stroke: (bottom: 0.5pt))]]\n",
        typst_str(sheet.target)
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
    for c in sheet.columns {
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
        for label in sheet.totals {
            src.push_str(&format!(
                "#text(10pt)[#{} : #box(width: 3.5cm, stroke: (bottom: 0.5pt))]\\\n",
                typst_str(label)
            ));
        }
    }

    // --- Ce qui ne s'attend pas ------------------------------------
    src.push_str(&format!(
        "#v(4mm)\n#block(width: 100%, inset: 6pt, stroke: 0.8pt)[#text(10pt, weight: \"bold\")[À signaler sans attendre]\\\n#text(10pt)[#{}]]\n",
        typst_str(sheet.alert)
    ));
    src.push_str(&format!(
        "#v(3mm)\n#text(10pt)[#{}]\n",
        typst_str(sheet.bring_back)
    ));
    src.push_str(&format!(
        "#v(3mm)\n#text(9.5pt)[Votre pharmacie : #{} — #{}]\n",
        typst_str(&pharmacy.name),
        typst_str(&pharmacy.phone)
    ));
    vec![("{{BODY}}", src)]
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
  [], [*Patient*], [*Téléphone*], [*Motif*], [*Ce que dit le dossier*], [*Ce qui a été dit*],
{{ROWS}})

#v(4mm)
#text(9pt, style: "italic")[Liste établie le {{DATE}} : elle vieillit avec la base, et se réimprime plutôt qu'elle ne se conserve.]
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
#text(9pt, style: "italic")[Les quantités portées ci-dessus sont celles que le registre tient au compte « à détruire » : ce que des patients ont rapporté et qui n'a pas été remis au stock délivrable. La destruction se porte au registre ligne par ligne, en citant le numéro du présent procès-verbal. Un stupéfiant rapporté ne se redélivre jamais.]
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
            typst_str(&format!(
                "{} {}",
                crate::codex::format_quantity(r.quantity),
                r.unit
            )),
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
            typst_str(&format!(
                "{} {}",
                crate::codex::format_quantity(r.stock),
                r.unit
            )),
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
            Kind::Sortie | Kind::Perte | Kind::Destruction => (String::new(), qty.clone()),
            Kind::Inventaire => (format!("= {qty}"), String::new()),
            Kind::Annulation => (String::new(), String::new()),
        };
        let after = running.get(i).copied().unwrap_or_default();
        // La colonne du coffre ne s'écrit que si quelque chose y est
        // passé : un registre sans aucun retour — l'immense majorité —
        // ne porte pas une colonne de zéros.
        let waiting = if after.to_destroy.abs() > 1e-6 || kind.is_destruction_side() {
            crate::codex::format_quantity(after.to_destroy)
        } else {
            String::new()
        };
        let no = if m.ordo_no > 0 {
            crate::ordonnancier::number_label(m.ordo_year as u32, m.ordo_no as u32)
        } else {
            String::new()
        };
        let file = if m.patient_id > 0 {
            format!("dossier {}", m.patient_id)
        } else {
            String::new()
        };
        let side = [
            m.prescriber.as_str(),
            m.supplier.as_str(),
            m.reference.as_str(),
            m.remark.as_str(),
            m.operator.as_str(),
        ]
        .into_iter()
        .filter(|t| !t.is_empty())
        .collect::<Vec<_>>()
        .join(" · ");
        body.push_str(&format!(
            "{}, {}, {}, {}, {}, {}, {}, {}, [#{}],\n",
            cell(&crate::db::format_french_date(&m.happened_on)),
            cell(&no),
            cell(crate::strings::tr(kind.label_key())),
            cell(&into),
            cell(&out),
            cell(&crate::codex::format_quantity(after.stock)),
            cell(&waiting),
            cell(&file),
            typst_str(&if struck {
                format!("annulée · {side}")
            } else {
                side
            }),
        ));
    }
    if body.is_empty() {
        body.push_str("[], [], [], [], [], [], [], [], [],\n");
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
        (
            "{{UNIT}}",
            if unit.is_empty() { "unités" } else { unit }.to_owned(),
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

#table(columns: (auto, auto, auto, auto, auto, auto, auto, auto, 1fr), inset: 4pt, stroke: 0.5pt,
  [*Date*], [*N°*], [*Nature*], [*Entrée*], [*Sortie*], [*Solde*], [*À détruire*], [*Dossier*], [*Mention*],
{{ROWS}})

#v(4mm)
#text(8pt, style: "italic")[{{COUNT}} ligne(s) au registre, comptées en {{UNIT}}. Une ligne écrite ne se rature pas : elle reste, barrée, et une ligne de plus la désigne et défait ce qu'elle avait fait au stock. Un inventaire *pose* le solde au lieu de s'y ajouter, si bien que les colonnes ne s'additionnent pas au solde final dès qu'un comptage a trouvé un écart — c'est le comptage qui l'explique. Deux soldes et non un : ce qu'un patient rapporte entre à l'officine et se justifie ici, mais ne se délivre plus, et reste au compte « à détruire » jusqu'au procès-verbal. Le nom du patient se lit en ouvrant le dossier dont le numéro figure ci-dessus.]
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
            typst_str(&crate::ordonnancier::number_label(
                m.ordo_year as u32,
                m.ordo_no as u32
            )),
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
#text(8pt, style: "italic")[{{COUNT}} délivrance(s) inscrite(s) pour l'année. Un numéro n'est jamais réattribué : une ligne annulée garde le sien, et la suite continue après lui. Le nom du patient se lit en ouvrant le dossier dont le numéro figure ci-dessus.]
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
#align(center)[#text(9pt, style: "italic")[Une préparation ne se fait que sur ordonnance et selon les bonnes pratiques de préparation.]]
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
        "#v(2mm)\n#text(8pt, style: \"italic\")[La ligne LPP et son tarif se vérifient au moment de la délivrance : cette fiche en donne la règle, pas le prix.]\n",
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
        ("Ce qui va de travers", dispo.caution.as_str()),
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
    Ok(out)
}

/// Escape arbitrary text as a Typst string literal, so patient names
/// can never inject markup into the generated document.
fn typst_str(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Substitute the interview-sheet placeholders. Values are spliced as
/// Typst string literals (`#"…"`), so a patient name containing markup
/// ('#', '*', brackets…) can neither break compilation nor restyle the
/// sheet.
#[allow(clippy::too_many_arguments)]
fn fill_interview_template(
    template: &str,
    patient: &Patient,
    kind: InterviewKind,
    today: &str,
    theme: &str,
    signature: &str,
    treats: &[Drug],
    checklist: &[&str],
) -> String {
    // The points of this theme, as tick-boxes: the sheet in the
    // pharmacist's hand carries what the entretien is for.
    let ticks = if checklist.is_empty() {
        String::new()
    } else {
        checklist
            .iter()
            .map(|point| {
                format!(
                    "#block(below: 2mm)[#box(width: 3.4mm, height: 3.4mm, stroke: 0.7pt) #h(2mm) #{}]",
                    typst_str(point)
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    template
        .replace(
            "{{PATIENT_NAME}}",
            &format!("#{}", typst_str(&patient.full_name())),
        )
        .replace(
            "{{BIRTH_DATE}}",
            &format!(
                "#{}",
                typst_str(&crate::db::format_french_date(&patient.birth_date))
            ),
        )
        .replace("{{KIND}}", &format!("#{}", typst_str(kind.label())))
        .replace("{{DATE}}", &format!("#{}", typst_str(today)))
        .replace(
            "{{THEME}}",
            // Un acte qui ne porte pas de thème n'en imprime pas, même
            // si la base en garde un : jusqu'à la 0.145 le thème armé
            // par le choix rapide était écrit sur les actes qui n'en ont
            // pas, et il ressortait ici. La source est corrigée ; ceci
            // couvre les lignes déjà écrites, sans réécrire la base.
            &format!(
                "#{}",
                typst_str(theme_or_dash(if kind.has_theme() { theme } else { "" }))
            ),
        )
        // Whoever held the entretien signs the sheet. A template
        // written before the team list simply has no such marker, and
        // loses nothing.
        .replace("{{PHARMACIST}}", &format!("#{}", typst_str(signature)))
        .replace("{{TREATMENTS}}", &treatments_markup(treats))
        .replace("{{CHECKLIST}}", &ticks)
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
fn ordonnance_advice_markup(advice: &[&str]) -> String {
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
    advice: &[&str],
    signature: &str,
    mentions: (&str, &str),
) -> String {
    // Both mentions are the officine's own: an empty one leaves no
    // line, not an empty italic line.
    let centered = |text: &str, size: &str| {
        if text.trim().is_empty() {
            String::new()
        } else {
            format!(
                "#align(center)[#text({size}, style: \"italic\")[#{}]]",
                typst_str(text.trim())
            )
        }
    };
    let footer = |text: &str| {
        if text.trim().is_empty() {
            String::new()
        } else {
            format!(
                "#v(2mm)\n#text(8pt, style: \"italic\")[#{}]",
                typst_str(text.trim())
            )
        }
    };
    template
        .replace(
            "{{PHARMACY_NAME}}",
            &format!("#{}", typst_str(&pharmacy.name)),
        )
        .replace(
            "{{PHARMACY_ADDRESS}}",
            &format!("#{}", typst_str(&pharmacy.address)),
        )
        .replace(
            "{{PHARMACY_PHONE}}",
            &format!("#{}", typst_str(&pharmacy.phone)),
        )
        .replace(
            "{{PHARMACY_AM}}",
            &format!("#{}", typst_str(&pharmacy.am_number)),
        )
        .replace("{{PHARMACIST}}", &format!("#{}", typst_str(signature)))
        .replace(
            "{{PATIENT_NAME}}",
            &format!("#{}", typst_str(&patient.full_name())),
        )
        .replace(
            "{{BIRTH_DATE}}",
            &format!(
                "#{}",
                typst_str(&crate::db::format_french_date(&patient.birth_date))
            ),
        )
        .replace("{{INDICATION}}", &format!("#{}", typst_str(indication)))
        .replace("{{DATE}}", &format!("#{}", typst_str(today)))
        .replace("{{LINES}}", &ordonnance_lines_markup(lines))
        .replace("{{ADVICE}}", &ordonnance_advice_markup(advice))
        .replace("{{MENTION_HEADER}}", &centered(mentions.0, "9pt"))
        .replace("{{MENTION_FOOTER}}", &footer(mentions.1))
}

/// Typeset the ordonnance and hand it to the OS viewer.
#[allow(clippy::too_many_arguments)]
pub fn open_ordonnance(
    patient: &Patient,
    pharmacy: &PharmacyConfig,
    indication: &str,
    today: &str,
    lines: &[crate::ordonnance::Line],
    advice: &[&str],
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
            typst_str(&format!("{:.2} EUR", l.fee).replace('.', ",")),
        ));
    }
    let total = format!("{total:.2} EUR").replace('.', ",");
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
                typst_str(&format!("{:.2} EUR", r.amount).replace('.', ",")),
            ));
        }
        let sum = format!("{sum:.2} EUR").replace('.', ",");
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
#text(9pt)[Forfaits tels qu'ils étaient enregistrés à la pose. La ligne LPP et son tarif se vérifient avant facturation.]
"#,
            n = rentals.len()
        );
    }
    vec![
        ("{{PERIOD}}", period.to_owned()),
        ("{{DATE}}", today_french.to_owned()),
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
/// redessiner, ce que le CLAUDE.md interdit explicitement.
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
];

const MARKERS_FICHE: &[&str] = &[
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
pub fn fill(template: &str, values: &[(&str, String)]) -> String {
    let mut out = template.to_owned();
    for (marker, value) in values {
        out = out.replace(marker, value);
    }
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
        "fiche" => vec![
            ("{{PATIENT_NAME}}", s(&patient.full_name())),
            (
                "{{BIRTH_DATE}}",
                s(&crate::db::format_french_date(&patient.birth_date)),
            ),
            ("{{DATE}}", s("24/08/2026")),
            ("{{KIND}}", s(InterviewKind::Bpm.label())),
            ("{{THEME}}", s("Observance")),
            (
                "{{TREATMENTS}}",
                sample_treatments()
                    .iter()
                    .map(|d| format!("- #{}", typst_str(&d.name)))
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
            (
                "{{CHECKLIST}}",
                crate::entretien::checklist("Observance")
                    .iter()
                    .map(|p| {
                        format!(
                            "#block(below: 2mm)[#box(width: 3.4mm, height: 3.4mm, stroke: 0.7pt) #h(2mm) #{}]",
                            typst_str(p)
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
            ("{{PHARMACIST}}", s(&pharmacy.pharmacist)),
        ],
        "cr" => vec![
            (
                "{{POINTS}}",
                "- #\"Observance satisfaisante sur les trois derniers mois.\"".to_owned(),
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
            (
                "{{TREATMENTS}}",
                sample_treatments()
                    .iter()
                    .map(|d| format!("- #{}", typst_str(&d.name)))
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
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
                lines: vec![(
                    "Amlodipine 5 mg".to_owned(),
                    "Tension artérielle".to_owned(),
                    "1 comprimé le matin".to_owned(),
                    "Ne pas arrêter sans avis.".to_owned(),
                )],
                mention: "Document remis à titre informatif.",
                signature: &pharmacy.pharmacist,
            },
            &pharmacy,
        ),
        "appels" => call_list_values(
            &[CallRow {
                name: "Jean Dupont",
                phone: "04 67 00 00 00",
                tag: "2 alerte(s)",
                reason: "Kaliémie à 5,4 sous IEC.",
            }],
            "24/08/2026",
            &pharmacy,
        ),
        "rdv" => appointment_list_values(
            &[Appointment {
                id: 1,
                patient_id: 1,
                patient_name: patient.full_name(),
                phone: "04 67 00 00 00".to_owned(),
                kind: InterviewKind::Bpm,
                date: "2026-09-14".to_owned(),
                time: "09:30".to_owned(),
            }],
            "24/08/2026",
        ),
        "guide" => guide_values(&pharmacy),
        "monographie" => monograph_values(&sample_treatments()[0], &[]),
        "dispositifs" => dispositifs_values(
            &[crate::db::Dispositif {
                id: 1,
                name: "Bas de compression classe 2".to_owned(),
                ..Default::default()
            }],
            &pharmacy,
        ),
        "registre" => stup_register_values(
            "Skenan LP 30 mg",
            "gélule",
            &[sample_stup_move()],
            &[crate::ordonnancier::Balance {
                stock: 24.0,
                to_destroy: 0.0,
            }],
            &std::collections::HashSet::new(),
            &pharmacy,
            "2026-08-29",
        ),
        "bilan" => bilan_values(
            &BilanData {
                patient: &patient,
                today: "24/08/2026",
                treatments: vec![(
                    "Eliquis 5 mg".to_owned(),
                    "apixaban — anticoagulant oral direct".to_owned(),
                    "1 comprimé matin et soir".to_owned(),
                )],
                interactions: Vec::new(),
                review: Vec::new(),
                biology: Vec::new(),
                findings: Vec::new(),
                watch: Vec::new(),
                vaccines: vec!["Grippe saisonnière".to_owned()],
                acts: Vec::new(),
                signature: &pharmacy.pharmacist,
            },
            &pharmacy,
        ),
        "suivi" => selfcheck_values(
            &crate::selfcheck::SHEETS[0],
            Some(&patient.full_name()),
            &pharmacy,
            "24/08/2026",
        ),
        "codex" => codex_values(&[crate::db::Preparation {
            id: 1,
            name: "Pommade à l'oxyde de zinc".to_owned(),
            form: "pommade".to_owned(),
            formula: "Oxyde de zinc | 15 g\nVaseline | qsp 100 g".to_owned(),
            yield_amount: "100 g".to_owned(),
            ..Default::default()
        }]),
        "dispositif" => dispositif_values(
            &crate::db::Dispositif {
                id: 1,
                name: "Bas de compression classe 2".to_owned(),
                ..Default::default()
            },
            &pharmacy,
        ),
        "vaccination" => vaccination_carnet_values(
            &patient,
            &[crate::db::Vaccination {
                id: 1,
                label: "dTPolio".to_owned(),
                dose: "rappel".to_owned(),
                given_on: "2026-03-14".to_owned(),
                lot: "K2341".to_owned(),
                site: "deltoïde gauche".to_owned(),
                operator: "CL".to_owned(),
                ..Default::default()
            }],
            "Document remis à titre informatif.",
        ),
        "facturation" => billing_recap_values(&[], &[], "Août 2026", "24/08/2026"),
        "ordonnancier" => ordonnancier_values(
            &[sample_stup_move()],
            &std::collections::HashMap::from([(1_i64, "Skenan LP 30 mg".to_owned())]),
            &std::collections::HashSet::new(),
            2026,
            &pharmacy,
            "2026-08-29",
        ),
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
            &[(
                "Oxyde de zinc".to_owned(),
                "15 g".to_owned(),
                "9 g".to_owned(),
            )],
            &pharmacy,
            "CL",
        ),
        "protocole" => protocol_values(
            "Rupture d'AOD",
            "Anticoagulants oraux directs",
            &[crate::db::ProtocolNode {
                id: 1,
                parent_id: None,
                branch: crate::db::Branch::Root,
                kind: crate::db::NodeKind::Question,
                text: "Le patient a-t-il une ordonnance en cours ?".to_owned(),
                position: 0,
            }],
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
            &[Appointment {
                id: 1,
                patient_id: 1,
                patient_name: patient.full_name(),
                phone: "04 67 00 00 00".to_owned(),
                kind: InterviewKind::Bpm,
                date: "2026-08-25".to_owned(),
                time: "09:30".to_owned(),
            }],
            &[],
            "2026-08-24",
        ),
        "conciliation" => conciliation_values(
            &ConciliationData {
                patient: &patient,
                today: "24/08/2026",
                summary: "3 divergences sur 7 lignes.",
                rows: vec![(
                    "Arrêté".to_owned(),
                    "Furosémide 40 mg".to_owned(),
                    "1 le matin".to_owned(),
                    String::new(),
                    "Absent de l'ordonnance de sortie.".to_owned(),
                )],
                physician: "Docteur Martin",
                mention: "Document remis à titre informatif.",
                signature: &pharmacy.pharmacist,
            },
            &pharmacy,
        ),
        "controle" => stock_check_values(
            &[crate::ordonnancier::ToCheck {
                id: 1,
                label: "Skenan LP 30 mg".to_owned(),
                unit: "gélule".to_owned(),
                stock: 24.0,
                days: Some(63),
                why: crate::ordonnancier::Why::Uncounted,
            }],
            &pharmacy,
            "2026-08-29",
        ),
        "destruction" => destruction_list_values(
            &[crate::ordonnancier::Awaiting {
                id: 1,
                label: "Skenan LP 30 mg".to_owned(),
                unit: "gélule".to_owned(),
                quantity: 14.0,
                since: "2026-07-02".to_owned(),
                days: Some(68),
            }],
            &pharmacy,
            "2026-09-08",
        ),
        "caisse" => {
            let mut q = [0_i64; crate::caisse::DENOMINATIONS.len()];
            q[3] = 4;
            q[4] = 7;
            q[7] = 9;
            q[9] = 5;
            let others = [crate::caisse::Other {
                label: "Carte".to_owned(),
                cents: 45_075,
            }];
            caisse_values(
                &pharmacy.name,
                "24/08/2026",
                "Claire Leroy",
                &q,
                &others,
                &crate::caisse::tally(&q, 15_000, &others, Some(66_000)),
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
                        "12/02/2026".to_owned(),
                        "Périndopril".to_owned(),
                    ),
                ],
                flagged: vec!["Créatinine et DFG".to_owned()],
                mention: "Document remis à titre informatif.",
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
            (
                "{{LINES}}",
                "+ #\"Amoxicilline 1 g\" \\\n  #\"1 g deux fois par jour pendant 6 jours\""
                    .to_owned(),
            ),
            (
                "{{ADVICE}}",
                "- #\"Boire fréquemment, par petites quantités.\"".to_owned(),
            ),
            (
                "{{MENTION_HEADER}}",
                s("Mention d'en-tête (facultative, [disclaimers] du config.toml)"),
            ),
            ("{{MENTION_FOOTER}}", s("Mention de pied (facultative)")),
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
  {{OTHERS}}
  [Autres encaissements], [{{OTHER_TOTAL}} €],
  [*Recette encaissée*], [*{{TAKINGS}} €*],
  [Recette attendue], [{{EXPECTED}}],
  [*Écart*], [*{{GAP}}*],
)

#v(4mm)
#table(columns: (1fr, auto), inset: 5pt, stroke: 0.4pt,
  [Fond de caisse laissé pour demain], [{{FLOAT}} €],
  [*Sorti du tiroir*], [*{{BANKED}} €*],
)

{{REMARK}}

#v(10mm)
#grid(columns: (1fr, 1fr), gutter: 10mm,
  [Compté par : #v(9mm) #line(length: 100%, stroke: 0.5pt)],
  [Vérifié par : #v(9mm) #line(length: 100%, stroke: 0.5pt)],
)

#v(4mm)
#text(8.5pt, style: "italic")[Un écart se note et s'explique ; il ne se corrige pas en changeant le comptage.]
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

#table(columns: (auto, auto, auto, auto, auto, auto, auto, 1fr), inset: 4pt, stroke: 0.4pt,
  align: (left, right, right, right, right, right, left, left),
  [*Jour*], [*Espèces*], [*Autres*], [*Recette*], [*Attendu*], [*Écart*], [*Par*], [*Remarque*],
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
#text(8pt, style: "italic")[Un soir recompté est une deuxième ligne : les deux figurent ici, et seule la dernière entre dans les totaux. Un écart se note et s'explique ; il ne se corrige pas en changeant le comptage.]
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
            "  {day}, [{} €], [{} €], [{} €], [{}], [{}], [#{}], [#text(8pt)[#{}]],\n",
            euros(r.cash),
            euros(r.other),
            euros(r.takings),
            money(r.expected),
            signed(r.gap),
            typst_str(&r.operator),
            typst_str(r.remark.trim()),
        ));
    }
    if body.is_empty() {
        body.push_str("  [], [], [], [], [], [], [], [],\n");
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
            let sign = if g > 0 { "+" } else { "" };
            format!(
                "Écart cumulé : {sign}{} € — sur {n} {}, {} en moins, {} en plus, {} juste.",
                euros(g),
                if n > 1 { "soirs" } else { "soir" },
                summary.short,
                summary.over,
                summary.exact
            )
        }
        (None, _) => {
            "Écart : aucune recette attendue n'a été saisie sur la période — il n'y a pas d'écart à établir.".to_owned()
        }
    };
    let worst = match summary.worst.as_ref().filter(|_| want_expected) {
        Some((day, gap)) => {
            let sign = if *gap > 0 { "+" } else { "" };
            format!(
                "Le soir le plus loin du compte : {} ({sign}{} €).",
                crate::db::format_french_date(day),
                euros(*gap)
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
        body.push_str(&format!(
            "  box(width: 100%, height: 100%, stroke: 0.5pt, inset: 5pt, clip: true)[\n    #text(11pt, weight: \"bold\")[#{}] \\\n    #text(9.5pt)[#{}]{}\n    #place(bottom + left)[#text(6.5pt)[#{} — #{}]]\n  ],\n",
            typst_str(name.trim()),
            typst_str(&short(dose, 70)),
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
  [], [*Analyse*], [*Rythme*], [*Dernier*], [*Ce qui la demande*], [*Résultat*],
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

    /// Chaque modèle par défaut compile avec ses valeurs d'exemple, et
    /// il ne reste pas un seul `{{` dedans — un marqueur non substitué
    /// s'imprime en toutes lettres au milieu de la page.
    #[test]
    fn every_default_template_compiles_with_its_sample_values() {
        for d in DOCS {
            let filled = fill(d.default, &sample_values(d.key));
            assert!(
                !filled.contains("{{"),
                "modèle « {} » : un marqueur n'a pas été remplacé",
                d.key
            );
            check_doc(d.key, d.default).unwrap_or_else(|e| panic!("modèle « {} » : {e}", d.key));
        }
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
    /// Une seule exemption, et elle est nommée : le bulletin
    /// d'adhésion n'est pas un Typst mais le PDF de l'Assurance
    /// Maladie, dont on ne remplit que les champs de formulaire (voir
    /// `bulletin.rs`). Lui donner un « modèle » serait le redessiner.
    #[test]
    fn every_printable_document_takes_a_template() {
        const EXEMPT: &[&str] = &["open_bulletin"];
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
        let t = tally(&q, 15_000, &others, Some(80_000));
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
            &tally(&q, 0, &[], None),
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
        // 204,50 + 450,75 le 7 (le recomptage), 312,00 + 523,00 le 8 :
        // 1 490,25 €. Avec les deux comptages du 7 additionnés, on
        // lirait 1 692,75 — l'erreur que ce test existe pour tenir.
        assert_eq!(of("{{TAKINGS}}"), "1\u{a0}490,25 €");
        assert!(of("{{DAYS}}").contains("2 soirs comptés"));
        assert!(of("{{DAYS}}").contains("1 recomptage"));
        // L'écart porte sur le seul soir qui avait un attendu, et le
        // dit : sans ce nombre à côté, la somme se lirait comme si elle
        // couvrait le mois.
        assert!(of("{{GAP}}").contains("sur 1 soir"), "{}", of("{{GAP}}"));
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
    #[test]
    fn the_labels_carry_the_dose_and_shorten_only_what_they_must() {
        let patient = sample_patient();
        let long = "Prendre le comprimé oublié dès que l'on s'en aperçoit, sauf s'il est presque l'heure de la prise suivante, auquel cas on saute la prise oubliée et on reprend le rythme habituel sans jamais doubler la dose.";
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
                    "12/02/2026".to_owned(),
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
            period_word: "semaine".to_owned(),
            amount: 48.0,
        }];
        let src = fill(
            DEFAULT_FACTURATION_TEMPLATE,
            &billing_recap_values(&lines, &rentals, "Août 2026", "24/08/2026"),
        );
        assert!(!src.contains("#eval \"Bernard\"]"));
        // The TPH code sits beside the act code, and the total adds up.
        assert!(src.contains("BMI + TPH"));
        assert!(src.contains("35,50 EUR"));
        assert!(src.contains("Locations de matériel"));
        assert!(src.contains("48,00 EUR"));
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
        assert!(src.contains("2026-0001") && src.contains("2026-0002"));
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
            },
            Appointment {
                id: 2,
                time: String::new(),
                patient_id: 2,
                patient_name: "Paul #eval \"Bernard\"".to_owned(),
                phone: String::new(),
                kind: InterviewKind::Asthme,
                date: "2026-08-27".to_owned(),
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
        let source = fill(DEFAULT_MONOGRAPHIE_TEMPLATE, &monograph_values(&d, &[]));
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
            &monograph_values(&d, &[]),
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
            &patient,
            InterviewKind::Bpm,
            "22/08/2026",
            "Initiation / bon usage",
            // The signature goes through the same escaping.
            "Claire #strike[Leroy]",
            &sample_treatments(),
            crate::entretien::checklist("Initiation / bon usage"),
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
            },
            Appointment {
                id: 2,
                time: "09:30".to_owned(),
                patient_id: 2,
                patient_name: "Hélène Lefèvre".to_owned(),
                phone: String::new(),
                kind: InterviewKind::Aod,
                date: "2026-09-03".to_owned(),
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
        let advice = ["Boire fréquemment.", "Aller au bout du traitement."];
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
        for sheet in crate::selfcheck::SHEETS {
            let source = fill(
                DEFAULT_SUIVI_TEMPLATE,
                &selfcheck_values(
                    sheet,
                    Some("Jean #eval \"x\" Dupont"),
                    &sample_pharmacy(),
                    "09/09/2026",
                ),
            );
            assert!(source.contains(sheet.title), "{} : sans titre", sheet.key);
            for step in sheet.protocol {
                // La ponctuation française passe par `typst_str` ; on
                // vérifie le début de la consigne, qui suffit à dire
                // qu'elle est là.
                let head: String = step.chars().take(24).collect();
                assert!(
                    source.contains(&head),
                    "{} : la consigne « {head}… » n'atteint pas le papier",
                    sheet.key
                );
            }
            assert!(source.contains("À signaler sans attendre"));
            assert!(source.contains("Ce qu'on vise"));
            for label in sheet.totals {
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
        let blank = fill(
            DEFAULT_SUIVI_TEMPLATE,
            &selfcheck_values(
                &crate::selfcheck::SHEETS[0],
                None,
                &sample_pharmacy(),
                "09/09/2026",
            ),
        );
        assert!(blank.contains("Nom :"), "{blank}");
        let world = PdfWorld::new(blank);
        assert!(typst::compile::<PagedDocument>(&world).output.is_ok());
        // Un nom d'espaces est un nom absent, pas un nom.
        let spaces = fill(
            DEFAULT_SUIVI_TEMPLATE,
            &selfcheck_values(
                &crate::selfcheck::SHEETS[0],
                Some("   "),
                &sample_pharmacy(),
                "09/09/2026",
            ),
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
                days: Some(311),
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
            source.contains("02/11/2025 (311 j)"),
            "et l'ancienneté avec"
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
        assert!(source.contains("Ce qui a été dit"));
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
        let source = fill(
            DEFAULT_PLAN_TEMPLATE,
            &plan_values(&data, &sample_pharmacy()),
        );
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
        let source = fill(
            DEFAULT_BILAN_TEMPLATE,
            &bilan_values(&data, &sample_pharmacy()),
        );
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
        assert!(source.contains("Prochaine : 18/11/2026 — Carnet papier"));
        assert!(source.contains("Mention de l'officine"));
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
