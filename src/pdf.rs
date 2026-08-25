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
                patient_id: 1,
                patient_name: "Jean #eval \"Dupont\" \\ *gras*".to_owned(),
                phone: "06 12 34 56 78".to_owned(),
                kind: InterviewKind::Bpm,
                date: "2026-09-01".to_owned(),
            },
            Appointment {
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
}
