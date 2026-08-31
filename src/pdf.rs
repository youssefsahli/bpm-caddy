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
    match target {
        "fiche" => &[
            "{{PATIENT_NAME}}",
            "{{BIRTH_DATE}}",
            "{{DATE}}",
            "{{KIND}}",
            "{{THEME}}",
            "{{TREATMENTS}}",
            "{{CHECKLIST}}",
            "{{PHARMACIST}}",
        ],
        "cr" => &[
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
        ],
        "carnet" => &["{{DAY}}", "{{ENTRIES}}"],
        _ => &[
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
        ],
    }
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
        "Claire Leroy, Pharmacien titulaire",
        &sample_treatments(),
        crate::entretien::checklist("Observance"),
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
        "Claire Leroy, Pharmacien titulaire",
        &sample_treatments(),
        crate::entretien::checklist("Observance"),
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
            &format!("#{}", typst_str(theme_or_dash(theme))),
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
        "Claire Leroy, Pharmacien titulaire",
        // L'aperçu montre le cas où l'on a coché : c'est celui qui peut
        // déborder la page, donc celui qu'un modèle doit être vérifié
        // sur. Le cadre vide, lui, n'a jamais fait déborder personne.
        &[
            "Observance sur la semaine écoulée",
            "Effets indésirables signalés",
        ],
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
        "Claire Leroy, Pharmacien titulaire",
        &[
            "Observance sur la semaine écoulée",
            "Effets indésirables signalés",
        ],
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
            "#text(size: 8pt)[Relu en : #{} — Sources : #{}]\n",
            typst_str(t.reviewed),
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
        ("En cas d'oubli", d.missed_dose.as_str()),
        ("Ce qui doit faire consulter", d.red_flags.as_str()),
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
pub fn open_bilan(data: &BilanData, pharmacy: &PharmacyConfig) -> Result<PathBuf, String> {
    compile_and_open(
        bilan_source(data, pharmacy),
        &format!("bilan_{}", data.patient.id),
    )
}

fn bilan_source(data: &BilanData, pharmacy: &PharmacyConfig) -> String {
    let mut src = String::from(
        "#set page(paper: \"a4\", margin: 1.6cm)\n\
         #set text(size: 10pt, lang: \"fr\", hyphenate: true)\n\
         #let sec(t) = [#v(3mm) #text(11pt, weight: \"bold\")[#t] #v(1mm) #line(length: 100%, stroke: 0.6pt) #v(1.5mm)]\n",
    );
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
    src
}

/// The team's handout: what the application is for, view by view, with
/// the shortcuts at the foot. Printed rather than shown — it lives
/// beside the counter PC, not behind a menu.
pub fn open_guide(pharmacy: &PharmacyConfig) -> Result<PathBuf, String> {
    compile_and_open(guide_source(pharmacy), "mode_emploi")
}

fn guide_source(pharmacy: &PharmacyConfig) -> String {
    let mut src = String::from(
        "#set page(paper: \"a4\", margin: 1.5cm, columns: 2)\n\
         #set text(size: 9pt, lang: \"fr\", hyphenate: true)\n\
         #set par(justify: true)\n\
         #let sec(t) = [#v(2.4mm) #text(10pt, weight: \"bold\")[#t] #v(0.8mm) #line(length: 100%, stroke: 0.5pt) #v(1mm)]\n\
         #place(top + center, scope: \"parent\", float: true)[\n\
           #text(15pt, weight: \"bold\")[BPM-Caddy — mode d'emploi]\n\
           #v(1mm)\n",
    );
    src.push_str(&format!(
        "  #text(9pt)[#{}]\n  #v(2mm)\n]\n",
        typst_str(if pharmacy.name.trim().is_empty() {
            "Un exemplaire près du poste, un dans le classeur."
        } else {
            pharmacy.name.trim()
        })
    ));
    for (title, body) in GUIDE_SECTIONS {
        src.push_str(&format!(
            "#sec[#{}]\n#text(9pt)[#{}]\n",
            typst_str(title),
            typst_str(body)
        ));
    }
    src
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
        "Le bandeau du haut porte l'identité, les traitements rattachés au référentiel médicaments (une puce par médicament, cliquable), et ce que le dossier voit tout seul : les interactions repérées entre ces traitements, et la revue d'ordonnance. En dessous, trois onglets : les entretiens, le carnet de vaccination, la biologie.",
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
        "En haut du dossier, « Bilan… » imprime le bilan partagé de médication avec ce que le dossier sait : traitements, interactions, revue d'ordonnance, biologie, vaccinations dues, actes de l'année, et les cadres à remplir pendant l'entretien. « Plan de prise… » imprime la feuille que le patient emporte : à quoi sert chaque médicament, quand le prendre, et quoi faire en cas d'oubli.",
    ),
    (
        "La biologie",
        "L'onglet « Biologie » enregistre les résultats : choisissez l'analyte, tapez la valeur, la date si ce n'est pas aujourd'hui. Chaque valeur est lue contre son intervalle usuel, et le panneau « Ce que ça change » la relit contre les traitements du dossier — une kaliémie à 5,4 n'a pas le même sens sous IEC. Cliquez le nom d'un analyte pour voir sa courbe.",
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
        "Depuis les médicaments : « Tables de conversion » (vingt-sept références datées, une recherche unique les traverse toutes), « Codex… » (les préparations de l'officine, avec la formule mise à la quantité prescrite et la fiche de fabrication), « Protocoles… » (les arbres de décision, à dérouler question par question au comptoir).",
    ),
    (
        "Chercher partout : « Aller à… » et « Dans le texte… »",
        "Ctrl+K ouvre une boîte au-dessus de tout : tapez trois lettres et elle rend les patients, les fiches, les tables, les préparations et les protocoles qui répondent, avec les flèches pour parcourir et Entrée pour ouvrir. Sa dernière ligne cherche le même mot dans le *texte* des fiches — c'est là que vivent les vraies questions du comptoir. Le même bouton se trouve dans les médicaments sous « Dans le texte… » : « pamplemousse », « allaitement », « QT », et chaque fiche qui le dit revient avec la phrase qui le porte, mot surligné, la posologie et sa remarque comprises. Une fiche patient ouverte ? Un bouton limite la recherche à ses seuls traitements.",
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
    /// (médicament, à quoi ça sert, quand le prendre, ce qu'il faut
    /// savoir) — one line per treatment.
    pub lines: Vec<(String, String, String, String)>,
    /// The officine's own mention, empty unless it wrote one.
    pub mention: &'a str,
    pub signature: &'a str,
}

/// The plan de prise on one sheet, in a size that is read without
/// glasses.
pub fn open_plan(data: &PlanData, pharmacy: &PharmacyConfig) -> Result<PathBuf, String> {
    compile_and_open(
        plan_source(data, pharmacy),
        &format!("plan_{}", data.patient.id),
    )
}

fn plan_source(data: &PlanData, pharmacy: &PharmacyConfig) -> String {
    let mut src = String::from(
        "#set page(paper: \"a4\", margin: 1.6cm)\n\
         #set text(size: 11.5pt, lang: \"fr\", hyphenate: true)\n",
    );
    src.push_str(&format!(
        "#align(center)[#text(17pt, weight: \"bold\")[Mon plan de traitement]]\n#v(1mm)\n#align(center)[#text(10pt)[#{} — #{}]]\n",
        typst_str(&data.patient.full_name()),
        typst_str(data.today)
    ));
    src.push_str("#v(4mm)\n");
    let mut rows = String::new();
    for (name, what, when, know) in &data.lines {
        rows.push_str(&format!(
            "[*#{}*], {}, {}, {},\n",
            typst_str(name),
            typst_str(what),
            typst_str(when),
            typst_str(know)
        ));
    }
    if rows.is_empty() {
        rows.push_str("[], [], [], [],\n");
    }
    src.push_str(&format!(
        "#table(columns: (auto, 1fr, 1fr, 1.2fr), inset: 7pt, stroke: 0.6pt,\n  [*Médicament*], [*À quoi ça sert*], [*Quand le prendre*], [*Ce qu'il faut savoir*],\n{rows})\n"
    ));
    src.push_str("#v(4mm)\n#text(10.5pt, weight: \"bold\")[Mes questions pour la prochaine fois]\n#v(1.5mm)\n#box(width: 100%, height: 3cm, stroke: 0.7pt)\n");
    src.push_str("#v(4mm)\n");
    src.push_str(&format!(
        "#text(10pt)[Votre pharmacie : #{} — #{}]\n",
        typst_str(&pharmacy.name),
        typst_str(&pharmacy.phone)
    ));
    if !data.signature.trim().is_empty() {
        src.push_str(&format!(
            "\\\n#text(10pt)[Préparé avec vous par #{}]\n",
            typst_str(data.signature.trim())
        ));
    }
    if !data.mention.trim().is_empty() {
        src.push_str(&format!(
            "#v(3mm)\n#text(8.5pt, style: \"italic\")[#{}]\n",
            typst_str(data.mention.trim())
        ));
    }
    src
}

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
) -> Result<PathBuf, String> {
    compile_and_open(call_list_source(rows, today, pharmacy), "liste_appel")
}

fn call_list_source(rows: &[CallRow], today: &str, pharmacy: &PharmacyConfig) -> String {
    let mut src = String::from(
        "#set page(paper: \"a4\", margin: 1.5cm)\n\
         #set text(size: 10pt, lang: \"fr\", hyphenate: true)\n",
    );
    src.push_str(&format!(
        "#align(center)[#text(15pt, weight: \"bold\")[Liste d'appel]]\n#v(1mm)\n#align(center)[#text(10pt)[#{} — #{}]]\n#v(4mm)\n",
        typst_str(&pharmacy.name),
        typst_str(today)
    ));
    let mut body = String::new();
    for r in rows {
        body.push_str(&format!(
            "[#box(width: 4mm, height: 4mm, stroke: 0.6pt)], [*#{}*], {}, [#text(8pt, weight: \"bold\")[#{}]], {}, [],\n",
            typst_str(r.name),
            typst_str(r.phone),
            typst_str(r.tag),
            typst_str(r.reason)
        ));
    }
    if body.is_empty() {
        body.push_str("[], [], [], [], [], [],\n");
    }
    src.push_str(&format!(
        "#table(columns: (auto, auto, auto, auto, 1.4fr, 1fr), inset: 5pt, stroke: 0.5pt,\n  [], [*Patient*], [*Téléphone*], [*Motif*], [*Ce que dit le dossier*], [*Ce qui a été dit*],\n{body})\n"
    ));
    src.push_str(&format!(
        "#v(4mm)\n#text(9pt, style: \"italic\")[Liste établie le #{} : elle vieillit avec la base, et se réimprime plutôt qu'elle ne se conserve.]\n",
        typst_str(today)
    ));
    src
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
) -> Result<PathBuf, String> {
    compile_and_open(stock_check_source(rows, pharmacy, today), "controle_stock")
}

fn stock_check_source(
    rows: &[crate::ordonnancier::ToCheck],
    pharmacy: &PharmacyConfig,
    today: &str,
) -> String {
    let mut src = String::from(
        "#set page(paper: \"a4\", margin: 1.5cm)\n\
         #set text(size: 10pt, lang: \"fr\", hyphenate: true)\n",
    );
    src.push_str(&format!(
        "#align(center)[#text(15pt, weight: \"bold\")[Contrôle des stupéfiants]]\n#v(1mm)\n#align(center)[#text(10pt)[#{} — #{}]]\n#v(4mm)\n",
        typst_str(&pharmacy.name),
        typst_str(&crate::db::format_french_date(today))
    ));
    let mut body = String::new();
    for r in rows {
        let since = match r.days {
            Some(d) => format!("{d} j"),
            None => "jamais".to_owned(),
        };
        body.push_str(&format!(
            "[#box(width: 4mm, height: 4mm, stroke: 0.6pt)], [*#{}*], [#{}], [#{}], [#{}], [], [],\n",
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
        body.push_str("[], [], [], [], [], [], [],\n");
    }
    src.push_str(&format!(
        "#table(columns: (auto, 1.4fr, auto, auto, auto, auto, 1fr), inset: 5pt, stroke: 0.5pt,\n  [], [*Produit*], [*Au registre*], [*Motif*], [*Dernier comptage*], [*Compté*], [*Observation*],\n{body})\n"
    ));
    src.push_str(
        "#v(6mm)\n#text(9pt)[Compté par : #box(width: 5cm, stroke: (bottom: 0.5pt))   Le : #box(width: 3cm, stroke: (bottom: 0.5pt))   Signature : #box(width: 4cm, stroke: (bottom: 0.5pt))]\n",
    );
    src.push_str(
        "#v(3mm)\n#text(9pt, style: \"italic\")[Tout écart entre le comptage et le registre est porté au registre par une ligne d'inventaire, avec son explication. Le registre ne se rature pas.]\n",
    );
    src
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
pub fn open_ordonnancier(
    rows: &[crate::db::StupMove],
    labels: &std::collections::HashMap<i64, String>,
    cancelled: &std::collections::HashSet<i64>,
    year: i64,
    pharmacy: &PharmacyConfig,
    today: &str,
) -> Result<PathBuf, String> {
    compile_and_open(
        ordonnancier_source(rows, labels, cancelled, year, pharmacy, today),
        &format!("ordonnancier_{year}"),
    )
}

fn ordonnancier_source(
    rows: &[crate::db::StupMove],
    labels: &std::collections::HashMap<i64, String>,
    cancelled: &std::collections::HashSet<i64>,
    year: i64,
    pharmacy: &PharmacyConfig,
    today: &str,
) -> String {
    let mut src = String::from(
        "#set page(paper: \"a4\", flipped: true, margin: 1.2cm)\n\
         #set text(size: 9pt, lang: \"fr\", hyphenate: true)\n",
    );
    src.push_str(&format!(
        "#align(center)[#text(15pt, weight: \"bold\")[Ordonnancier des stupéfiants — #{}]]\n#v(1mm)\n#align(center)[#text(9pt)[#{} — édité le #{}]]\n#v(4mm)\n",
        typst_str(&year.to_string()),
        typst_str(&pharmacy.name),
        typst_str(&crate::db::format_french_date(today))
    ));
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
    src.push_str(&format!(
        "#table(columns: (auto, auto, 1.4fr, auto, auto, 1fr, auto, auto), inset: 4pt, stroke: 0.5pt,\n  [*N°*], [*Date*], [*Produit*], [*Quantité*], [*Dossier*], [*Prescripteur*], [*Par*], [*État*],\n{body})\n"
    ));
    src.push_str(&format!(
        "#v(4mm)\n#text(8pt, style: \"italic\")[{} délivrance(s) inscrite(s) pour l'année. Un numéro n'est jamais réattribué : une ligne annulée garde le sien, et la suite continue après lui. Le nom du patient se lit en ouvrant le dossier dont le numéro figure ci-dessus.]\n",
        rows.len()
    ));
    src
}

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
) -> Result<PathBuf, String> {
    compile_and_open(
        conciliation_source(data, pharmacy),
        &format!("conciliation_{}", data.patient.id),
    )
}

fn conciliation_source(data: &ConciliationData, pharmacy: &PharmacyConfig) -> String {
    let mut src = String::from(
        "#set page(paper: \"a4\", margin: 1.5cm)\n\
         #set text(size: 10pt, lang: \"fr\", hyphenate: true)\n",
    );
    src.push_str(&format!(
        "#align(center)[#text(15pt, weight: \"bold\")[Conciliation médicamenteuse]]\n#v(1mm)\n#align(center)[#text(10pt)[#{} — né(e) le #{} — le #{}]]\n",
        typst_str(&data.patient.full_name()),
        typst_str(&crate::db::format_french_date(&data.patient.birth_date)),
        typst_str(data.today)
    ));
    if !data.physician.trim().is_empty() {
        src.push_str(&format!(
            "#align(center)[#text(10pt)[À l'attention du #{}]]\n",
            typst_str(data.physician.trim())
        ));
    }
    src.push_str(&format!(
        "#v(2mm)\n#align(center)[#text(9.5pt, style: \"italic\")[#{}]]\n#v(3mm)\n",
        typst_str(data.summary)
    ));
    let mut rows = String::new();
    for (status, name, before, after, note) in &data.rows {
        rows.push_str(&format!(
            "[#text(8pt, weight: \"bold\")[#{}]], [*#{}*], {}, {}, [#text(8.5pt, style: \"italic\")[#{}]],\n",
            typst_str(status),
            typst_str(name),
            typst_str(before),
            typst_str(after),
            typst_str(note)
        ));
    }
    if rows.is_empty() {
        rows.push_str("[], [], [], [], [],\n");
    }
    src.push_str(&format!(
        "#table(columns: (auto, auto, 1fr, 1fr, 1.1fr), inset: 5pt, stroke: 0.5pt,\n  [*Statut*], [*Traitement*], [*Au dossier*], [*Sur l'ordonnance de sortie*], [*Remarque*],\n{rows})\n"
    ));
    src.push_str("#v(4mm)\n#text(10pt, weight: \"bold\")[Avis du prescripteur]\n#v(1.5mm)\n#box(width: 100%, height: 3.5cm, stroke: 0.7pt)\n");
    src.push_str("#v(4mm)\n");
    src.push_str(&format!(
        "#text(9.5pt)[#{} — #{}]\n",
        typst_str(&pharmacy.name),
        typst_str(&pharmacy.phone)
    ));
    if !data.signature.trim().is_empty() {
        src.push_str(&format!(
            "\\\n#text(9.5pt)[Rapprochement établi par #{}]\n",
            typst_str(data.signature.trim())
        ));
    }
    if !data.mention.trim().is_empty() {
        src.push_str(&format!(
            "#v(3mm)\n#text(8pt, style: \"italic\")[#{}]\n",
            typst_str(data.mention.trim())
        ));
    }
    src
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
) -> Result<PathBuf, String> {
    compile_and_open(
        preparation_source(prep, target, lines, pharmacy, operator),
        &format!("preparation_{}", prep.id),
    )
}

fn preparation_source(
    prep: &crate::db::Preparation,
    target: &str,
    lines: &[(String, String, String)],
    pharmacy: &PharmacyConfig,
    operator: &str,
) -> String {
    let mut src = String::from(
        "#set page(paper: \"a4\", margin: 1.8cm)\n\
         #set text(size: 10.5pt, lang: \"fr\", hyphenate: true)\n",
    );
    src.push_str(&format!(
        "#grid(columns: (1fr, auto), [#text(weight: \"bold\")[#{}]], [#align(right)[#text(9pt)[Fiche de fabrication]]])\n",
        typst_str(&pharmacy.name)
    ));
    src.push_str("#v(2mm)\n#line(length: 100%, stroke: 0.8pt)\n#v(3mm)\n");
    src.push_str(&format!(
        "#align(center)[#text(15pt, weight: \"bold\")[#{}]]\n",
        typst_str(&prep.name)
    ));
    if !prep.form.trim().is_empty() {
        src.push_str(&format!(
            "#align(center)[#text(10pt, style: \"italic\")[#{}]]\n",
            typst_str(prep.form.trim())
        ));
    }
    src.push_str("#v(4mm)\n");
    src.push_str(&format!(
        "#text(weight: \"bold\")[Quantité préparée :] #{} #h(1fr) #text(weight: \"bold\")[Date :] #box(width: 3cm, stroke: (bottom: 0.6pt))[] #h(6mm) #text(weight: \"bold\")[Par :] #box(width: 2.5cm, stroke: (bottom: 0.6pt))[#{}]\n",
        typst_str(target),
        typst_str(operator)
    ));
    src.push_str("#v(4mm)\n");
    // The formula, with a blank column for the lot of every raw
    // material: that column is the point of the sheet.
    let mut rows = String::new();
    for (name, written, weighed) in lines {
        rows.push_str(&format!(
            "{}, {}, {}, [],\n",
            typst_str(name),
            typst_str(written),
            typst_str(weighed)
        ));
    }
    if rows.is_empty() {
        rows.push_str("[], [], [], [],\n");
    }
    src.push_str(&format!(
        "#table(columns: (1fr, auto, auto, 3.4cm), inset: 6pt, stroke: 0.6pt,\n  [*Matière première*], [*Formule*], [*À peser*], [*N° de lot*],\n{rows})\n"
    ));
    for (title, body) in [
        ("Mode opératoire", prep.method.as_str()),
        ("Conservation", prep.conservation.as_str()),
        ("Mise en garde", prep.caution.as_str()),
        ("Indication", prep.indication.as_str()),
    ] {
        if body.trim().is_empty() {
            continue;
        }
        src.push_str(&format!(
            "#v(3mm)\n#text(weight: \"bold\", size: 10pt)[#{}]\n#v(1mm)\n#text(9.5pt)[#{}]\n",
            typst_str(title),
            typst_str(body.trim())
        ));
    }
    src.push_str(
        "#v(5mm)\n#line(length: 100%, stroke: 0.4pt)\n#v(2mm)\n\
         #grid(columns: (1fr, 1fr), gutter: 8mm,\n\
           [#text(9.5pt, weight: \"bold\")[Contrôle] #v(1mm) #box(width: 100%, height: 2cm, stroke: 0.6pt)],\n\
           [#text(9.5pt, weight: \"bold\")[Étiquetage et remise] #v(1mm) #box(width: 100%, height: 2cm, stroke: 0.6pt)])\n",
    );
    let sources: Vec<&str> = prep
        .sources
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    if !sources.is_empty() {
        src.push_str(&format!(
            "#v(3mm)\n#text(size: 8pt)[Sources : #{}]\n",
            typst_str(&sources.join(" · "))
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

/// The whole codex as a booklet: one block per preparation, in the
/// order the list shows them. What goes in the préparatoire's binder.
pub fn open_codex(preparations: &[crate::db::Preparation]) -> Result<PathBuf, String> {
    compile_and_open(codex_source(preparations), "codex")
}

fn codex_source(preparations: &[crate::db::Preparation]) -> String {
    let mut src = String::from(
        "#set page(paper: \"a4\", margin: 1.6cm)\n\
         #set text(size: 9.5pt, lang: \"fr\", hyphenate: true)\n\
         #set par(justify: true)\n\
         #align(center)[#text(15pt, weight: \"bold\")[Codex des préparations]]\n\
         #v(1mm)\n\
         #align(center)[#text(9pt, style: \"italic\")[Une préparation ne se fait que sur ordonnance et selon les bonnes pratiques de préparation.]]\n\
         #v(4mm)\n",
    );
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
    src
}

/// One dispositif as a printable A4 sheet — the one that goes in the
/// drawer beside the box, or in the patient's hand at the counter.
pub fn open_dispositif(
    dispo: &crate::db::Dispositif,
    pharmacy: &PharmacyConfig,
) -> Result<PathBuf, String> {
    compile_and_open(dispositif_source(dispo, pharmacy), "dispositif")
}

fn dispositif_source(dispo: &crate::db::Dispositif, pharmacy: &PharmacyConfig) -> String {
    let mut src = String::from(
        "#set page(paper: \"a4\", margin: 1.8cm)\n\
         #set text(size: 10pt, lang: \"fr\", hyphenate: true)\n\
         #set par(justify: true)\n\
         #let sec(t) = [#v(3mm) #text(10.5pt, weight: \"bold\")[#t] #v(1mm) #line(length: 100%, stroke: 0.5pt) #v(1.5mm)]\n",
    );
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
    src
}

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
) -> Result<PathBuf, String> {
    compile_and_open(dispositifs_source(dispositifs, pharmacy), "dispositifs")
}

fn dispositifs_source(dispositifs: &[crate::db::Dispositif], pharmacy: &PharmacyConfig) -> String {
    let mut src = String::from(
        "#set page(paper: \"a4\", margin: 1.5cm, columns: 2)\n\
         #set text(size: 8.5pt, lang: \"fr\", hyphenate: true)\n\
         #set par(justify: true)\n\
         #place(top + center, scope: \"parent\", float: true)[\n\
           #text(15pt, weight: \"bold\")[Dispositifs médicaux]\n\
           #v(1mm)\n",
    );
    src.push_str(&format!(
        "  #text(8.5pt, style: \"italic\")[#{}]\n  #v(3mm)\n]\n",
        typst_str(if pharmacy.name.trim().is_empty() {
            "La ligne LPP et son tarif se vérifient au moment de la délivrance."
        } else {
            pharmacy.name.trim()
        })
    ));
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
    src
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
            &format!("#{}", typst_str(theme_or_dash(theme))),
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
    // The preview shows the template itself: both mentions are filled
    // with a sample, so an officine editing the layout can see where
    // its own would land.
    let filled = fill_ordonnance_template(
        template,
        &sample_patient(),
        &sample_pharmacy(),
        "Angine à streptocoque du groupe A — TROD positif",
        "26/08/2026",
        &lines,
        &advice,
        &sample_pharmacy().pharmacist,
        (
            "Mention d'en-tête (facultative, [disclaimers] du config.toml)",
            "Mention de pied (facultative)",
        ),
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
    mention: &str,
) -> Result<PathBuf, String> {
    compile_and_open(
        vaccination_carnet_source(patient, lines, mention),
        "carnet_vaccination",
    )
}

fn vaccination_carnet_source(
    patient: &Patient,
    lines: &[crate::db::Vaccination],
    mention: &str,
) -> String {
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
{foot}
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

fn billing_recap_source(
    lines: &[BillingLine],
    rentals: &[BillingRental],
    period: &str,
    today_french: &str,
) -> String {
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
{rental_block}"#
    )
}

/// Compile and open the billing recap for printing.
pub fn open_billing_recap(
    lines: &[BillingLine],
    rentals: &[BillingRental],
    period: &str,
    today_french: &str,
) -> Result<PathBuf, String> {
    compile_and_open(
        billing_recap_source(lines, rentals, period, today_french),
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
        let src = billing_recap_source(&lines, &rentals, "Août 2026", "24/08/2026");
        assert!(!src.contains("#eval \"Bernard\"]"));
        // The TPH code sits beside the act code, and the total adds up.
        assert!(src.contains("BMI + TPH"));
        assert!(src.contains("35,50 EUR"));
        assert!(src.contains("Locations de matériel"));
        assert!(src.contains("48,00 EUR"));
        assert!(src.contains("en cours"));
        // No rental, no second table: an empty heading reads as a bug.
        let bare = billing_recap_source(&lines, &[], "Août 2026", "24/08/2026");
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
        let src = ordonnancier_source(
            &rows,
            &labels,
            &cancelled,
            2026,
            &PharmacyConfig::default(),
            "2026-08-30",
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
        let orphan = ordonnancier_source(
            &[line(3, 3, 99, 7.0, 55)],
            &labels,
            &std::collections::HashSet::new(),
            2026,
            &PharmacyConfig::default(),
            "2026-08-30",
        );
        assert!(!orphan.contains("#strike["), "rien n'y est annulé");
        assert!(orphan.contains("—"));
        // Une année sans délivrance imprime un tableau vide plutôt que
        // rien : c'est aussi une réponse.
        let empty = ordonnancier_source(
            &[],
            &labels,
            &std::collections::HashSet::new(),
            2025,
            &PharmacyConfig::default(),
            "2026-08-30",
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
            missed_dose: "Dans les 6 heures, sinon sauter la prise.".to_owned(),
            red_flags: "Selles noires, traumatisme crânien.".to_owned(),
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

    /// Every marker the editor advertises must actually be filled, and
    /// every marker the default template uses must be advertised. A
    /// marker that is only half of that prints itself on the page.
    #[test]
    fn the_editor_advertises_the_markers_the_templates_fill() {
        let cases: [(&str, &str); 4] = [
            ("fiche", default_template()),
            ("cr", default_cr_template()),
            ("carnet", default_trans_template()),
            ("ordonnance", default_ordonnance_template()),
        ];
        for (key, template) in cases {
            let listed = template_markers(key);
            for marker in listed {
                assert!(
                    template.contains(marker),
                    "{key} : {marker} annoncé mais absent du modèle par défaut"
                );
            }
            // And the other way round: every {{MARKER}} of the default
            // template is listed.
            let mut rest = template;
            while let Some(at) = rest.find("{{") {
                rest = &rest[at..];
                let Some(end) = rest.find("}}") else { break };
                let marker = &rest[..end + 2];
                assert!(
                    listed.contains(&marker),
                    "{key} : {marker} utilisé mais non annoncé"
                );
                rest = &rest[end + 2..];
            }
        }
    }

    /// The handout must compile and say what it is about — an empty
    /// section would be a paragraph nobody wrote.
    #[test]
    fn the_guide_prints_on_one_sheet() {
        for (title, body) in GUIDE_SECTIONS {
            assert!(!title.trim().is_empty());
            assert!(
                body.trim().len() > 80,
                "section « {title} » trop courte pour dire quoi que ce soit"
            );
        }
        let source = guide_source(&sample_pharmacy());
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
        let source = stock_check_source(&rows, &sample_pharmacy(), "2026-08-29");
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
        let world = PdfWorld::new(stock_check_source(&[], &sample_pharmacy(), "2026-08-29"));
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
        let source = call_list_source(&rows, "29/08/2026", &sample_pharmacy());
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
        let world = PdfWorld::new(call_list_source(&[], "29/08/2026", &sample_pharmacy()));
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
        let source = conciliation_source(&data, &sample_pharmacy());
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
        let world = PdfWorld::new(conciliation_source(&empty, &sample_pharmacy()));
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
        let source = plan_source(&data, &sample_pharmacy());
        assert!(source.contains("Mon plan de traitement"));
        assert!(source.contains("Dans les 6 heures"));
        assert!(source.contains("Mes questions"));
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
        let world = PdfWorld::new(plan_source(&empty, &sample_pharmacy()));
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
        let source = bilan_source(&data, &sample_pharmacy());
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
        let world = PdfWorld::new(bilan_source(&data, &sample_pharmacy()));
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
        let source = codex_source(&preparations);
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
        let world = PdfWorld::new(codex_source(&[]));
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
        let booklet = dispositifs_source(&all, &sample_pharmacy());
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
        let sheet = dispositif_source(&one, &sample_pharmacy());
        let world = PdfWorld::new(sheet);
        assert!(typst::compile::<PagedDocument>(&world).output.is_ok());
        // An empty list still prints its cover rather than failing.
        let world = PdfWorld::new(dispositifs_source(&[], &sample_pharmacy()));
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
        let source = preparation_source(&prep, "60 g", &lines, &sample_pharmacy(), "CL");
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
        let source = vaccination_carnet_source(&patient, &lines, "Mention de l'officine");
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
        let source = vaccination_carnet_source(&sample_patient(), &[], "");
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
