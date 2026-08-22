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

use crate::db::{InterviewKind, Patient};

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

/// A self-contained Typst world: one in-memory source, embedded fonts.
struct PdfWorld {
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<Font>,
    source: Source,
}

impl PdfWorld {
    fn new(text: String) -> Self {
        let fonts: Vec<Font> = typst_assets::fonts()
            .flat_map(|data| Font::iter(Bytes::new(data)))
            .collect();
        let book = FontBook::from_fonts(&fonts);
        Self {
            library: LazyHash::new(Library::default()),
            book: LazyHash::new(book),
            fonts,
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
/// viewer. `template_override` comes from `config.toml`; `today` is the
/// interview date shown on the sheet (JJ/MM/AAAA).
pub fn open_interview_sheet(
    patient: &Patient,
    kind: InterviewKind,
    today: &str,
    template_override: Option<&std::path::Path>,
) -> Result<PathBuf, String> {
    let template = match template_override {
        Some(path) => std::fs::read_to_string(path)
            .map_err(|e| format!("modèle {} illisible : {e}", path.display()))?,
        None => DEFAULT_TEMPLATE.to_owned(),
    };
    let filled = template
        .replace("{{PATIENT_NAME}}", &patient.full_name())
        .replace(
            "{{BIRTH_DATE}}",
            &crate::db::format_french_date(&patient.birth_date),
        )
        .replace("{{KIND}}", kind.label())
        .replace("{{DATE}}", today);

    let world = PdfWorld::new(filled);
    let document: PagedDocument = typst::compile(&world)
        .output
        .map_err(|errs| format!("compilation Typst : {}", format_diagnostics(&errs)))?;
    let pdf = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
        .map_err(|errs| format!("export PDF : {}", format_diagnostics(&errs)))?;

    let out = std::env::temp_dir().join(format!(
        "bpm_caddy_fiche_{}_{}.pdf",
        patient.id,
        kind.as_str().to_lowercase()
    ));
    std::fs::write(&out, pdf).map_err(|e| format!("écriture du PDF impossible : {e}"))?;
    open::that_detached(&out).map_err(|e| format!("ouverture du PDF impossible : {e}"))?;
    Ok(out)
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
        let filled = DEFAULT_TEMPLATE
            .replace("{{PATIENT_NAME}}", "Jean Dupont")
            .replace("{{BIRTH_DATE}}", "03/07/1958")
            .replace("{{KIND}}", "BPM")
            .replace("{{DATE}}", "22/08/2026");
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
}
