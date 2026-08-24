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
  #text(weight: "bold")[Date de l'entretien :] {{DATE}}
]

#note-box("Traitements en cours", 3.4cm)
#note-box("Observance et difficultés rencontrées", 3.4cm)
#note-box("Points d'attention / interactions", 3.4cm)
#note-box("Conclusion et plan d'action", 3.6cm)

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
#text(weight: "bold")[Objet : {{KIND}} — {{PATIENT_NAME}} (né(e) le {{BIRTH_DATE}})]
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
    template_path: &std::path::Path,
) -> Result<PathBuf, String> {
    let template = if template_path.exists() {
        std::fs::read_to_string(template_path)
            .map_err(|e| format!("modèle {} illisible : {e}", template_path.display()))?
    } else {
        DEFAULT_TEMPLATE.to_owned()
    };
    let filled = fill_interview_template(&template, patient, kind, today);

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
fn fill_cr_template(
    template: &str,
    patient: &Patient,
    kind: InterviewKind,
    date: &str,
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
        .replace("{{TREATMENTS}}", &treatments_markup(treats))
}

/// Compile the CR letter for a patient and open it in the OS viewer.
/// `template_path` behaves like the interview sheet's: the file when it
/// exists, the embedded default otherwise.
pub fn open_cr_letter(
    patient: &Patient,
    kind: InterviewKind,
    date: &str,
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
    let filled = fill_cr_template(&template, patient, kind, date, treats, pharmacy);
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
        &sample_treatments(),
        &sample_pharmacy(),
    );
    compile_and_open(filled, "apercu_cr")
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
        let filled =
            fill_interview_template(DEFAULT_TEMPLATE, &patient, InterviewKind::Bpm, "22/08/2026");
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
