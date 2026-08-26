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

#note-box("Traitements en cours", 3.2cm)
#note-box("Observance et difficultés rencontrées", 3.2cm)
#note-box("Points d'attention / interactions", 3.2cm)
#note-box("Conclusion et plan d'action", 3.4cm)

#v(1fr)
#grid(columns: (1fr, 1fr), gutter: 1cm,
  [#text(weight: "bold")[Signature du pharmacien]
   #v(2mm)
   #box(width: 100%, height: 2cm, stroke: 0.8pt, radius: 5pt)],
  [#text(weight: "bold")[Prochain rendez-vous]
   #v(2mm)
   #box(width: 100%, height: 2cm, stroke: 0.8pt, radius: 5pt)],
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
#box(width: 100%, height: 7cm, stroke: 0.8pt, radius: 5pt)

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
#align(center)[#text(9pt, style: "italic")[Dispensation protocolisée par le pharmacien après test rapide d'orientation diagnostique]]
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
#v(2mm)
#text(8pt, style: "italic")[Reconsulter sans attendre en cas d'aggravation ou de signes nouveaux.]
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
pub fn open_interview_sheet(
    patient: &Patient,
    kind: InterviewKind,
    today: &str,
    theme: &str,
    template_path: &std::path::Path,
) -> Result<PathBuf, String> {
    let template = if template_path.exists() {
        std::fs::read_to_string(template_path)
            .map_err(|e| format!("modèle {} illisible : {e}", template_path.display()))?
    } else {
        DEFAULT_TEMPLATE.to_owned()
    };
    let filled = fill_interview_template(&template, patient, kind, today, theme);

    let stem = format!("fiche_{}_{}", patient.id, kind.as_str().to_lowercase());
    compile_and_open(filled, &stem)
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

/// Compile `template` with sample data, reporting Typst errors —
/// validation for the in-app template editor.
pub fn check_template(template: &str) -> Result<(), String> {
    let filled = fill_interview_template(
        template,
        &sample_patient(),
        InterviewKind::Bpm,
        "24/08/2026",
        "Observance",
    );
    let world = PdfWorld::new(filled);
    typst::compile::<PagedDocument>(&world)
        .output
        .map(|_| ())
        .map_err(|errs| format!("compilation Typst : {}", format_diagnostics(&errs)))
}

/// Compile `template` with sample data and open the result — the
/// editor's preview button.
pub fn preview_template(template: &str) -> Result<PathBuf, String> {
    let filled = fill_interview_template(
        template,
        &sample_patient(),
        InterviewKind::Bpm,
        "24/08/2026",
        "Observance",
    );
    compile_and_open(filled, "apercu")
}

/// The embedded CR-letter template, for the in-app editor.
pub fn default_cr_template() -> &'static str {
    DEFAULT_CR_TEMPLATE
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
        .replace(
            "{{PHARMACIST}}",
            &format!("#{}", typst_str(&pharmacy.pharmacist)),
        )
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
            &format!("#{}", typst_str(theme_or_dash(theme))),
        )
        .replace("{{TREATMENTS}}", &treatments_markup(treats))
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
) -> Result<PathBuf, String> {
    let template = if template_path.exists() {
        std::fs::read_to_string(template_path)
            .map_err(|e| format!("modèle {} illisible : {e}", template_path.display()))?
    } else {
        DEFAULT_CR_TEMPLATE.to_owned()
    };
    let filled = fill_cr_template(&template, patient, kind, date, theme, treats, pharmacy);
    compile_and_open(filled, &format!("cr_{}", patient.id))
}

fn sample_pharmacy() -> PharmacyConfig {
    PharmacyConfig {
        name: "Pharmacie du Centre".to_owned(),
        address: "1 place de la Mairie, 34000 Montpellier".to_owned(),
        phone: "04 67 00 00 00".to_owned(),
        pharmacist: "Dr Claire Leroy, pharmacien titulaire".to_owned(),
        am_number: "3400123".to_owned(),
    }
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

/// Validation for the CR template editor.
pub fn check_cr_template(template: &str) -> Result<(), String> {
    let filled = fill_cr_template(
        template,
        &sample_patient(),
        InterviewKind::Bpm,
        "24/08/2026",
        "Observance",
        &sample_treatments(),
        &sample_pharmacy(),
    );
    let world = PdfWorld::new(filled);
    typst::compile::<PagedDocument>(&world)
        .output
        .map(|_| ())
        .map_err(|errs| format!("compilation Typst : {}", format_diagnostics(&errs)))
}

/// Sample-data preview for the CR template editor.
pub fn preview_cr_template(template: &str) -> Result<PathBuf, String> {
    let filled = fill_cr_template(
        template,
        &sample_patient(),
        InterviewKind::Bpm,
        "24/08/2026",
        "Observance",
        &sample_treatments(),
        &sample_pharmacy(),
    );
    compile_and_open(filled, "apercu_cr")
}

/// Build the Typst source for the conversion tables (all of them, one
/// A4 document). Every cell goes through the string escaping.
type TableEdits = std::collections::HashMap<(String, usize, usize), String>;

fn conversion_tables_source(edits: &TableEdits) -> String {
    let mut src = String::from(
        "#set page(paper: \"a4\", margin: 1.5cm)\n\
         #set text(size: 10pt, lang: \"fr\", hyphenate: true)\n\
         #align(center)[#text(15pt, weight: \"bold\")[Tables de conversion]]\n",
    );
    for t in crate::tables::TABLES {
        src.push_str(&format!(
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
        src.push_str(&format!(
            "#table(\n  columns: ({widths}),\n  inset: 5pt,\n  stroke: 0.6pt,\n"
        ));
        for c in t.columns {
            src.push_str(&format!("  [*#{}*],\n", typst_str(c)));
        }
        for (ri, row) in t.rows.iter().enumerate() {
            for (ci, cell) in row.iter().enumerate() {
                // The team's correction prints instead of the shipped
                // value, so paper and screen never disagree.
                let text = edits
                    .get(&(t.short.to_owned(), ri, ci))
                    .map(String::as_str)
                    .unwrap_or(cell);
                src.push_str(&format!("  [#{}],\n", typst_str(text)));
            }
        }
        src.push_str(")\n");
        let sources = t
            .sources
            .iter()
            .enumerate()
            .map(|(i, s)| format!("{}. {}", i + 1, s))
            .collect::<Vec<_>>()
            .join("   ");
        src.push_str(&format!(
            "#text(size: 8pt)[Sources : #{}]\n",
            typst_str(&sources)
        ));
    }
    src
}

/// One drug card as a printable A4 monograph: identity, every filled
/// section in reading order, the pharmacokinetics as a definition list
/// and the numbered sources at the foot.
pub fn open_drug_monograph(
    d: &Drug,
    posologies: &[crate::db::Posologie],
) -> Result<PathBuf, String> {
    compile_and_open(
        monograph_source(d, posologies),
        &format!("monographie_{}", d.id),
    )
}

fn monograph_source(d: &Drug, posologies: &[crate::db::Posologie]) -> String {
    let mut src = String::from(
        "#set page(paper: \"a4\", margin: 2cm)\n#set text(size: 10.5pt)\n\
         #set par(justify: true, leading: 0.6em, spacing: 0.7em)\n\
         #let sec(title, body) = block(above: 4.5mm, below: 0mm)[\n  \
         #block(above: 0mm, below: 1.2mm)[\
         #text(size: 9pt, weight: \"bold\")[#upper(title)]\n  \
         #v(-1.2mm)\n  #line(length: 100%, stroke: 0.4pt)]\n  #body\n]\n",
    );
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
    src
}

/// One substitution protocol as a printable A4 page: the decision tree
/// as an indented list, questions in bold, branches labelled.
pub fn open_protocol(
    title: &str,
    subject: &str,
    nodes: &[crate::db::ProtocolNode],
) -> Result<PathBuf, String> {
    compile_and_open(protocol_source(title, subject, nodes), "protocole")
}

fn protocol_source(title: &str, subject: &str, nodes: &[crate::db::ProtocolNode]) -> String {
    let mut src = String::from(
        "#set page(paper: \"a4\", margin: 2cm)\n\
         #set text(size: 11pt, lang: \"fr\", hyphenate: true)\n",
    );
    src.push_str(&format!(
        "#align(center)[#text(15pt, weight: \"bold\")[#{}]]\n",
        typst_str(title)
    ));
    if !subject.trim().is_empty() {
        src.push_str(&format!(
            "#align(center)[#text(10pt, style: \"italic\")[#{}]]\n",
            typst_str(subject.trim())
        ));
    }
    src.push_str("#v(3mm)\n#line(length: 100%, stroke: 0.6pt)\n#v(2mm)\n");
    // Depth-first, "yes" branch before "no", same order as on screen.
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
        src.push_str(&format!(
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
    src
}

/// The week on one landscape A4 page: a column per day, rendez-vous
/// and other entries in the order of the day, hours first.
pub fn open_week_plan(
    week: &[String],
    appointments: &[Appointment],
    events: &[crate::db::Event],
    today: &str,
) -> Result<PathBuf, String> {
    if week.is_empty() {
        return Err("semaine vide".to_owned());
    }
    compile_and_open(
        week_plan_source(week, appointments, events, today),
        "semaine",
    )
}

fn week_plan_source(
    week: &[String],
    appointments: &[Appointment],
    events: &[crate::db::Event],
    today: &str,
) -> String {
    let monday = week.first().map(String::as_str).unwrap_or("");
    let mut src = String::from(
        "#set page(paper: \"a4\", flipped: true, margin: 1.2cm)\n\
         #set text(size: 9pt, lang: \"fr\", hyphenate: true)\n",
    );
    src.push_str(&format!(
        "#align(center)[#text(14pt, weight: \"bold\")[Semaine du #{}]]\n#v(3mm)\n",
        typst_str(&crate::db::format_french_date(monday))
    ));
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
            let hour = if ev.time.is_empty() {
                String::new()
            } else {
                format!("{} ", ev.time)
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
    // Full-height columns: the sheet is meant to be written on during
    // the week, not just read.
    src.push_str(&format!(
        "#table(columns: (1fr, 1fr, 1fr, 1fr, 1fr, 1fr, 1fr), rows: 16cm, inset: 4pt, \
         stroke: 0.5pt, align: top,\n{cells})\n"
    ));
    src
}

/// Compile and open the conversion tables as a printable A4 reference.
pub fn open_conversion_tables(edits: &TableEdits) -> Result<PathBuf, String> {
    compile_and_open(conversion_tables_source(edits), "tables")
}

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

/// The embedded carnet template, for the in-app editor.
pub fn default_trans_template() -> &'static str {
    DEFAULT_TRANS_TEMPLATE
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

/// Validation for the carnet template editor.
pub fn check_trans_template(template: &str) -> Result<(), String> {
    let filled = fill_trans_template(template, "Lundi 24/08/2026", &sample_transmissions());
    let world = PdfWorld::new(filled);
    typst::compile::<PagedDocument>(&world)
        .output
        .map(|_| ())
        .map_err(|errs| format!("compilation Typst : {}", format_diagnostics(&errs)))
}

/// Sample-data preview for the carnet template editor.
pub fn preview_trans_template(template: &str) -> Result<PathBuf, String> {
    let filled = fill_trans_template(template, "Lundi 24/08/2026", &sample_transmissions());
    compile_and_open(filled, "apercu_carnet")
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
fn fill_interview_template(
    template: &str,
    patient: &Patient,
    kind: InterviewKind,
    today: &str,
    theme: &str,
) -> String {
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
            &format!("#{}", typst_str(theme_or_dash(theme))),
        )
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
fn appointment_list_source(rdvs: &[Appointment], today_french: &str) -> String {
    let mut rows = String::new();
    for rdv in rdvs {
        rows.push_str(&format!(
            "{}, {}, {}, {},\n",
            typst_str(&crate::db::format_french_date(&rdv.date)),
            typst_str(&rdv.patient_name),
            typst_str(rdv.kind.label()),
            typst_str(&rdv.phone),
        ));
    }
    format!(
        r#"
#set page(paper: "a4", margin: 1.5cm)
#set text(size: 11pt)
#align(center)[#text(16pt, weight: "bold")[Rendez-vous à venir]]
#v(1mm)
#align(center)[Édité le {today_french}]
#v(5mm)
#table(
  columns: (auto, 1fr, auto, auto),
  inset: 7pt,
  stroke: 0.6pt,
  [*Date*], [*Patient*], [*Type*], [*Téléphone*],
{rows})
"#
    )
}

/// The embedded ordonnance template, for the in-app editor.
pub fn default_ordonnance_template() -> &'static str {
    DEFAULT_ORDONNANCE_TEMPLATE
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
fn fill_ordonnance_template(
    template: &str,
    patient: &Patient,
    pharmacy: &PharmacyConfig,
    indication: &str,
    today: &str,
    lines: &[crate::ordonnance::Line],
    advice: &[&str],
) -> String {
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
        .replace(
            "{{PHARMACIST}}",
            &format!("#{}", typst_str(&pharmacy.pharmacist)),
        )
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
}

/// Typeset the ordonnance and hand it to the OS viewer.
pub fn open_ordonnance(
    patient: &Patient,
    pharmacy: &PharmacyConfig,
    indication: &str,
    today: &str,
    lines: &[crate::ordonnance::Line],
    advice: &[&str],
    template_path: &std::path::Path,
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
        &template, patient, pharmacy, indication, today, lines, advice,
    );
    compile_and_open(filled, &format!("ordonnance_{}", patient.id))
}

/// Validation and preview for the ordonnance template editor.
pub fn check_ordonnance_template(template: &str) -> Result<(), String> {
    let _ = ordonnance_preview_source(template)?;
    Ok(())
}

pub fn preview_ordonnance_template(template: &str) -> Result<PathBuf, String> {
    let filled = ordonnance_preview_source(template)?;
    compile_and_open(filled, "apercu_ordonnance")
}

fn ordonnance_preview_source(template: &str) -> Result<String, String> {
    let lines = [crate::ordonnance::Line {
        name: "Amoxicilline 1 g".to_owned(),
        posology: "1 g deux fois par jour pendant 6 jours".to_owned(),
        caution: String::new(),
    }];
    let advice = ["Boire fréquemment, par petites quantités."];
    let filled = fill_ordonnance_template(
        template,
        &sample_patient(),
        &sample_pharmacy(),
        "Angine à streptocoque du groupe A — TROD positif",
        "26/08/2026",
        &lines,
        &advice,
    );
    let world = PdfWorld::new(filled.clone());
    let _: PagedDocument = typst::compile(&world)
        .output
        .map_err(|errs| format!("compilation Typst : {}", format_diagnostics(&errs)))?;
    Ok(filled)
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
) -> Result<PathBuf, String> {
    compile_and_open(
        vaccination_carnet_source(patient, lines),
        "carnet_vaccination",
    )
}

fn vaccination_carnet_source(patient: &Patient, lines: &[crate::db::Vaccination]) -> String {
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
    format!(
        r#"
#set page(paper: "a4", flipped: true, margin: 1.4cm)
#set text(size: 10pt, lang: "fr")
#align(center)[#text(16pt, weight: "bold")[Carnet de vaccination]]
#v(1mm)
#align(center)[#text(12pt)[#{head}] — né(e) le #{born}]
#v(5mm)
#table(
  columns: (auto, 1fr, auto, auto, auto, auto, 1fr),
  inset: 6pt,
  stroke: 0.6pt,
  [*Date*], [*Vaccin*], [*Dose*], [*Lot*], [*Site*], [*Par*], [*Remarque*],
{rows})
#v(4mm)
#text(8pt, style: "italic")[Document indicatif édité par BPM-Caddy : il ne remplace pas le carnet de vaccination officiel ni le dossier médical partagé.]
"#
    )
}

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
fn billing_recap_source(lines: &[BillingLine], period: &str, today_french: &str) -> String {
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
    format!(
        r#"
#set page(paper: "a4", margin: 1.5cm, flipped: true)
#set text(size: 10pt)
#align(center)[#text(16pt, weight: "bold")[Récapitulatif de facturation]]
#v(1mm)
#align(center)[{period} — édité le {today_french}]
#v(5mm)
#table(
  columns: (auto, 1fr, auto, auto, auto, auto, auto, auto),
  inset: 6pt,
  stroke: 0.6pt,
  [*Date*], [*Patient*], [*Thème*], [*Code acte*], [*Étape*], [*Situation*],
  [*Prise en charge*], [*Montant*],
{rows})
#v(4mm)
#text(weight: "bold")[{count} acte(s) — total {total}]
#v(3mm)
#text(9pt)[Prestation facturée en tiers payant, indépendamment de tout code CIP, aux prix TTC. Une seule pharmacie accompagne un patient : celle qui a débuté la séquence annuelle perçoit la rémunération.]
"#
    )
}

/// Compile and open the billing recap for printing.
pub fn open_billing_recap(
    lines: &[BillingLine],
    period: &str,
    today_french: &str,
) -> Result<PathBuf, String> {
    compile_and_open(
        billing_recap_source(lines, period, today_french),
        "facturation",
    )
}

/// Compile and open the RDV list for printing.
pub fn open_appointment_list(rdvs: &[Appointment], today_french: &str) -> Result<PathBuf, String> {
    compile_and_open(appointment_list_source(rdvs, today_french), "rdv")
}

fn format_diagnostics(errs: &[typst::diag::SourceDiagnostic]) -> String {
    errs.iter()
        .map(|d| d.message.to_string())
        .collect::<Vec<_>>()
        .join(" ; ")
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let src = billing_recap_source(&lines, "Août 2026", "24/08/2026");
        assert!(!src.contains("#eval \"Bernard\"]"));
        // The TPH code sits beside the act code, and the total adds up.
        assert!(src.contains("BMI + TPH"));
        assert!(src.contains("35,50 EUR"));
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
            id: 1,
            day: "2026-08-25".to_owned(),
            time: "14:00".to_owned(),
            title: "Formation AOD".to_owned(),
            category: EventCategory::Formation,
            repeat_days: 0,
            source_id: 1,
        }];
        let src = week_plan_source(&week, &rdvs, &events, "2026-08-25");
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
        let source = protocol_source("AOD indisponible", "AOD", &nodes);
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
        check_trans_template(DEFAULT_TRANS_TEMPLATE).expect("le carnet par défaut doit compiler");
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
        };
        let source = monograph_source(&d, &[]);
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
        let world = PdfWorld::new(monograph_source(&d, &[]));
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
        );
        let world = PdfWorld::new(filled);
        let document: PagedDocument = typst::compile(&world)
            .output
            .expect("le modèle par défaut doit compiler");
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
        assert!(check_template(DEFAULT_TEMPLATE).is_ok());
        let err = check_template("#broken(").unwrap_err();
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
        let source = conversion_tables_source(&edits);
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
        assert!(check_cr_template(DEFAULT_CR_TEMPLATE).is_ok());
        assert!(check_cr_template("#broken(").is_err());
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
        );
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
        let source = appointment_list_source(&rdvs, "23/08/2026");
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
        );
        assert!(source.contains("3400123"), "le N° AM doit figurer");
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
        );
        assert!(!source.contains("Conseils"));
        let world = PdfWorld::new(source);
        let _: PagedDocument = typst::compile(&world)
            .output
            .expect("l'ordonnance doit compiler sans conseils");
    }

    #[test]
    fn the_default_ordonnance_template_passes_its_own_validation() {
        check_ordonnance_template(DEFAULT_ORDONNANCE_TEMPLATE)
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
        let source = vaccination_carnet_source(&patient, &lines);
        // Oldest first on paper, whatever order the screen showed.
        let dtp = source.find("Rappel 45 ans").expect("le dTP doit figurer");
        let flu = source.find("FLU25-208").expect("la grippe doit figurer");
        assert!(
            dtp < flu,
            "le carnet imprimé se lit du plus ancien au plus récent"
        );
        assert!(source.contains("Prochaine : 18/11/2026 — Carnet papier"));
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
        let source = vaccination_carnet_source(&sample_patient(), &[]);
        let world = PdfWorld::new(source);
        let document: PagedDocument = typst::compile(&world)
            .output
            .expect("un carnet vide doit compiler");
        assert!(typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
            .expect("l'export PDF doit réussir")
            .starts_with(b"%PDF-"));
    }
}
