//! Filling the official « bulletin d'adhésion et de désignation du
//! pharmacien ».
//!
//! The five forms in `assets/bulletins/` are the Assurance Maladie PDFs
//! exactly as ameli.fr serves them. The app **fills their AcroForm
//! fields**; it does not redraw the document. What comes out of the
//! printer is the official form with the identity blocks typed in,
//! which is the only version worth signing.
//!
//! Two things are deliberately left alone:
//!
//! * **Every checkbox.** OUI/NON on the adhésion, OUI/NON on informing
//!   the médecin traitant, and « à l'initiative du pharmacien » are the
//!   patient's decisions, taken in front of the form. Pre-ticking a
//!   consent box on someone's behalf is not a convenience.
//! * **The date and the signatures.** The form is signed in pen; the
//!   day it is signed is the day that belongs on it.
//!
//! A value the officine has not entered — a NIR, a régime, the
//! pharmacy's Assurance Maladie number — leaves its line untouched, so
//! the printed form still carries the dotted rule to fill by hand.
//!
//! ## Why the field names are a table rather than a convention
//!
//! The five PDFs were produced separately and their field names do not
//! agree: the pharmacy's Assurance Maladie number is `N AM` on two of
//! them, `Num identification` on a third and `fill_11` on the last two.
//! Worse, `Adresse 1`/`Adresse 2` is the **patient's** address on the
//! AOD, AVK and asthme forms and the **pharmacy's** on the BPM and
//! anticancéreux ones. The names were therefore read off the rendered
//! forms by position, not guessed, and a test checks every one of them
//! still exists in its PDF.

use lopdf::{Dictionary, Document, Object, Stream, StringFormat};

use crate::config::PharmacyConfig;
use crate::db::{format_french_date, InterviewKind, Patient};

/// Where each value goes on one form. `""` means the form has no field
/// for it (the AOD, AVK and asthme bulletins print the « Régime
/// d'affiliation » line without one).
struct Form {
    pdf: &'static [u8],
    nom: &'static str,
    naissance: &'static str,
    nir: &'static str,
    regime: &'static str,
    adresse: [&'static str; 2],
    pharmacie: &'static str,
    pharmacie_adresse: [&'static str; 2],
    pharmacie_am: &'static str,
    pharmacien: &'static str,
    medecin: &'static str,
}

const BPM: Form = Form {
    pdf: include_bytes!("../assets/bulletins/bpm.pdf"),
    nom: "Nom et Prénom",
    naissance: "Date de naissance",
    nir: "N dimmatriculation",
    regime: "fill_5",
    adresse: ["Adresse", "undefined_2"],
    pharmacie: "Nom de la pharmacie",
    pharmacie_adresse: ["Adresse 1", "Adresse 2"],
    pharmacie_am: "fill_11",
    pharmacien: "Nom du pharmacien désigné en charge de laccompagnement1",
    medecin: "Nom du médecin traitant",
};

const AOD: Form = Form {
    pdf: include_bytes!("../assets/bulletins/aod.pdf"),
    nom: "Nom et Prénom",
    naissance: "Date de naissance",
    nir: "N dimmatriculation",
    regime: "",
    adresse: ["Adresse 1", "Adresse 2"],
    pharmacie: "Nom de la pharmacie",
    pharmacie_adresse: ["Adresse 1_2", "Adresse 2_2"],
    pharmacie_am: "N AM",
    pharmacien: "Nom du pharmacien désigné en charge de laccompagnement1",
    medecin: "Nom du médecin traitant",
};

const AVK: Form = Form {
    pdf: include_bytes!("../assets/bulletins/avk.pdf"),
    nom: "Nom et Prénom",
    naissance: "Date de naissance",
    nir: "N dimmatriculation",
    regime: "",
    adresse: ["Adresse 1", "Adresse 2"],
    pharmacie: "Nom de la pharmacie",
    pharmacie_adresse: ["Adresse 1_2", "Adresse 2_2"],
    pharmacie_am: "Num identification",
    pharmacien: "Nom du pharmacien désigné en charge de laccompagnement1",
    medecin: "Nom du médecin traitant",
};

const ASTHME: Form = Form {
    pdf: include_bytes!("../assets/bulletins/asthme.pdf"),
    nom: "Nom et Prénom",
    naissance: "Date de naissance",
    nir: "N dimmatriculation",
    regime: "",
    adresse: ["Adresse 1", "Adresse 2"],
    pharmacie: "Nom de la pharmacie",
    pharmacie_adresse: ["Adresse 1_2", "Adresse 2_2"],
    pharmacie_am: "N AM",
    pharmacien: "Nom du pharmacien désigné en charge de laccompagnement1",
    medecin: "Nom du médecin traitant",
};

const ANTICANCEREUX: Form = Form {
    pdf: include_bytes!("../assets/bulletins/anticancereux.pdf"),
    nom: "Nom et Prénom",
    naissance: "Date de naissance",
    nir: "N dimmatriculation",
    regime: "fill_5",
    adresse: ["Adresse", "undefined_2"],
    pharmacie: "Nom de la pharmacie",
    pharmacie_adresse: ["Adresse 1", "Adresse 2"],
    pharmacie_am: "fill_11",
    pharmacien: "Nom du pharmacien désigné en charge de laccompagnement1",
    medecin: "Nom du médecin traitant",
};

/// The bulletin an act's theme adheres to, if it has one.
///
/// The two anticancéreux themes share a single form, and the acts
/// outside the accompaniment convention — TROD, vaccination, RDV
/// prévention — have no adhésion at all.
fn form_for(kind: InterviewKind) -> Option<&'static Form> {
    match kind {
        InterviewKind::Bpm => Some(&BPM),
        InterviewKind::Aod => Some(&AOD),
        InterviewKind::Avk => Some(&AVK),
        InterviewKind::Asthme => Some(&ASTHME),
        InterviewKind::AnticancereuxLc | InterviewKind::AnticancereuxAutres => Some(&ANTICANCEREUX),
        InterviewKind::TrodAngine
        | InterviewKind::TrodCystite
        | InterviewKind::Prevention
        | InterviewKind::Vaccination => None,
    }
}

/// Does this act kind have a bulletin d'adhésion to print?
pub fn has_bulletin(kind: InterviewKind) -> bool {
    form_for(kind).is_some()
}

/// Split a one-line address over the form's two lines, at the last
/// comma — "12 rue des Lilas, 34000 Montpellier" is street then town.
/// Without a comma the whole thing goes on the first line.
fn split_address(address: &str) -> [String; 2] {
    let address = address.trim();
    match address.rfind(',') {
        Some(i) => [
            address[..i].trim().to_owned(),
            address[i + 1..].trim().to_owned(),
        ],
        None => [address.to_owned(), String::new()],
    }
}

/// Fill the bulletin for `kind` and return the PDF bytes.
pub fn fill(
    kind: InterviewKind,
    patient: &Patient,
    pharmacy: &PharmacyConfig,
) -> Result<Vec<u8>, String> {
    let form = form_for(kind)
        .ok_or_else(|| format!("{} n'a pas de bulletin d'adhésion.", kind.label()))?;
    let mut doc = Document::load_mem(form.pdf).map_err(|e| format!("bulletin illisible : {e}"))?;

    let patient_address = split_address(&patient.address);
    let pharmacy_address = split_address(&pharmacy.address);
    let values: Vec<(&str, String)> = vec![
        (form.nom, patient.full_name()),
        (form.naissance, format_french_date(&patient.birth_date)),
        (form.nir, patient.nir.clone()),
        (form.regime, patient.regime.clone()),
        (form.adresse[0], patient_address[0].clone()),
        (form.adresse[1], patient_address[1].clone()),
        (form.pharmacie, pharmacy.name.clone()),
        (form.pharmacie_adresse[0], pharmacy_address[0].clone()),
        (form.pharmacie_adresse[1], pharmacy_address[1].clone()),
        (form.pharmacie_am, pharmacy.am_number.clone()),
        (form.pharmacien, pharmacy.pharmacist.clone()),
        (form.medecin, patient.physician.clone()),
    ];

    let helv = helvetica_ref(&doc);
    for (name, value) in values {
        // An empty name is a form with no such field; an empty value is
        // something the officine has not entered. Both leave the line
        // exactly as the Assurance Maladie printed it.
        if name.is_empty() || value.trim().is_empty() {
            continue;
        }
        set_text_field(&mut doc, name, value.trim(), helv)?;
    }
    // Belt and braces: a viewer that ignores our appearance streams is
    // told to build its own.
    set_need_appearances(&mut doc);

    let mut out = Vec::new();
    doc.save_to(&mut out)
        .map_err(|e| format!("écriture du bulletin impossible : {e}"))?;
    Ok(out)
}

/// The object id of the AcroForm's `/Helv` font.
///
/// Every one of the five forms carries Helvetica with a Latin-1
/// encoding in `/DR`, while the fields' own `/DA` points at Arial with
/// a *MacRoman* one — which is why a name with an « é » came out as
/// « È » when the viewer was left to draw it. Drawing through `/Helv`
/// is what makes French names print correctly.
fn helvetica_ref(doc: &Document) -> Option<(u32, u16)> {
    let acro = doc
        .catalog()
        .ok()?
        .get(b"AcroForm")
        .ok()?
        .as_reference()
        .ok()?;
    let dr = doc.get_object(acro).ok()?.as_dict().ok()?.get(b"DR").ok()?;
    let dr = match dr {
        Object::Reference(id) => doc.get_object(*id).ok()?.as_dict().ok()?,
        Object::Dictionary(d) => d,
        _ => return None,
    };
    let fonts = dr.get(b"Font").ok()?;
    let fonts = match fonts {
        Object::Reference(id) => doc.get_object(*id).ok()?.as_dict().ok()?,
        Object::Dictionary(d) => d,
        _ => return None,
    };
    fonts.get(b"Helv").ok()?.as_reference().ok()
}

fn set_need_appearances(doc: &mut Document) {
    let Ok(acro) = doc.catalog().and_then(|c| c.get(b"AcroForm")) else {
        return;
    };
    let Ok(id) = acro.as_reference() else { return };
    if let Ok(Object::Dictionary(d)) = doc.get_object_mut(id) {
        d.set("NeedAppearances", Object::Boolean(true));
    }
}

/// Encode to Latin-1 for `/Helv`. A character outside it is replaced
/// rather than dropped, so a name never silently loses a letter.
fn latin1(text: &str) -> Vec<u8> {
    text.chars()
        .map(|c| if (c as u32) < 256 { c as u8 } else { b'?' })
        .collect()
}

/// Escape a Latin-1 byte string as a PDF literal string.
fn pdf_literal(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len() + 8);
    out.push(b'(');
    for &b in bytes {
        if b == b'(' || b == b')' || b == b'\\' {
            out.push(b'\\');
        }
        out.push(b);
    }
    out.push(b')');
    out
}

/// Helvetica is about half an em wide on average; enough to decide
/// whether a long address has to be set smaller to fit its box.
const AVERAGE_GLYPH_EM: f32 = 0.5;

/// Write `value` into the text field named `name`: the value itself,
/// a `/DA` that draws it through Helvetica, and an appearance stream so
/// every viewer — and every printer — shows the same thing.
fn set_text_field(
    doc: &mut Document,
    name: &str,
    value: &str,
    helv: Option<(u32, u16)>,
) -> Result<(), String> {
    let Some(id) = find_field(doc, name) else {
        return Err(format!("champ « {name} » absent du bulletin"));
    };
    let rect = field_rect(doc, id).unwrap_or([0.0, 0.0, 200.0, 14.0]);
    let (w, h) = (rect[2] - rect[0], rect[3] - rect[1]);
    // Shrink to fit rather than overflow the ruled line.
    let mut size = (h * 0.62).clamp(6.0, 10.0);
    let needed = value.chars().count() as f32 * AVERAGE_GLYPH_EM;
    if needed > 0.0 {
        size = size.min(((w - 4.0).max(1.0)) / needed).max(5.0);
    }
    let baseline = ((h - size) / 2.0 + size * 0.22).max(1.0);

    let mut content = Vec::new();
    content.extend_from_slice(b"/Tx BMC q BT /Helv ");
    content.extend_from_slice(format!("{size:.2}").as_bytes());
    content.extend_from_slice(b" Tf 0 g 2 ");
    content.extend_from_slice(format!("{baseline:.2}").as_bytes());
    content.extend_from_slice(b" Td ");
    content.extend_from_slice(&pdf_literal(&latin1(value)));
    content.extend_from_slice(b" Tj ET Q EMC");

    let mut resources = Dictionary::new();
    if let Some(helv) = helv {
        let mut fonts = Dictionary::new();
        fonts.set("Helv", Object::Reference(helv));
        resources.set("Font", Object::Dictionary(fonts));
    }
    let mut stream_dict = Dictionary::new();
    stream_dict.set("Type", Object::Name(b"XObject".to_vec()));
    stream_dict.set("Subtype", Object::Name(b"Form".to_vec()));
    stream_dict.set("FormType", Object::Integer(1));
    stream_dict.set(
        "BBox",
        Object::Array(vec![
            Object::Real(0.0),
            Object::Real(0.0),
            Object::Real(w),
            Object::Real(h),
        ]),
    );
    stream_dict.set("Resources", Object::Dictionary(resources));
    let appearance = doc.add_object(Stream::new(stream_dict, content));

    let da = format!("/Helv {size:.2} Tf 0 g");
    let Ok(Object::Dictionary(field)) = doc.get_object_mut(id) else {
        return Err(format!("champ « {name} » illisible"));
    };
    field.set("V", Object::String(latin1(value), StringFormat::Literal));
    field.set("DA", Object::String(da.into_bytes(), StringFormat::Literal));
    let mut ap = Dictionary::new();
    ap.set("N", Object::Reference(appearance));
    field.set("AP", Object::Dictionary(ap));
    Ok(())
}

/// The object id of the text field called `name`.
fn find_field(doc: &Document, name: &str) -> Option<(u32, u16)> {
    doc.objects.iter().find_map(|(id, object)| {
        let dict = object.as_dict().ok()?;
        let title = dict.get(b"T").ok()?.as_str().ok()?;
        // Field names are stored as PDFDocEncoded bytes; the names we
        // match are ASCII-plus-Latin-1, so compare byte for byte.
        (title == latin1(name).as_slice() && dict.get(b"FT").ok()?.as_name().ok()? == b"Tx")
            .then_some(*id)
    })
}

fn field_rect(doc: &Document, id: (u32, u16)) -> Option<[f32; 4]> {
    let rect = doc.get_object(id).ok()?.as_dict().ok()?.get(b"Rect").ok()?;
    let rect = rect.as_array().ok()?;
    let mut out = [0.0_f32; 4];
    for (slot, value) in out.iter_mut().zip(rect.iter()) {
        *slot = match value {
            Object::Integer(i) => *i as f32,
            Object::Real(r) => *r,
            _ => return None,
        };
    }
    Some([
        out[0].min(out[2]),
        out[1].min(out[3]),
        out[0].max(out[2]),
        out[1].max(out[3]),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_patient() -> Patient {
        Patient {
            id: 1,
            last_name: "Lefèvre".to_owned(),
            first_name: "Hélène".to_owned(),
            birth_date: "1952-09-27".to_owned(),
            physician: "Dr Morel".to_owned(),
            address: "12 rue des Lilas, 34000 Montpellier".to_owned(),
            nir: "2 52 09 34 172 042 11".to_owned(),
            regime: "01".to_owned(),
            ..Default::default()
        }
    }

    fn sample_pharmacy() -> PharmacyConfig {
        PharmacyConfig {
            name: "Pharmacie du Centre".to_owned(),
            address: "1 place de la Mairie, 34000 Montpellier".to_owned(),
            phone: "04 67 00 00 00".to_owned(),
            pharmacist: "Dr Claire Leroy".to_owned(),
            am_number: "3400123".to_owned(),
            operators: Vec::new(),
        }
    }

    /// The mapping is the whole feature: a renamed or missing field
    /// would silently drop a line off a form that gets signed.
    #[test]
    fn every_mapped_field_exists_in_its_form() {
        for kind in InterviewKind::ALL {
            let Some(form) = form_for(kind) else { continue };
            let doc = Document::load_mem(form.pdf).expect("le bulletin doit se charger");
            let mut names = vec![
                form.nom,
                form.naissance,
                form.nir,
                form.adresse[0],
                form.adresse[1],
                form.pharmacie,
                form.pharmacie_adresse[0],
                form.pharmacie_adresse[1],
                form.pharmacie_am,
                form.pharmacien,
                form.medecin,
            ];
            if !form.regime.is_empty() {
                names.push(form.regime);
            }
            for name in names {
                assert!(
                    find_field(&doc, name).is_some(),
                    "{} : champ « {name} » introuvable",
                    kind.label()
                );
            }
        }
    }

    /// The patient's address and the pharmacy's must not land in the
    /// same pair of fields — on three of the five forms they are called
    /// `Adresse 1`/`Adresse 2` and `Adresse 1_2`/`Adresse 2_2`, which is
    /// exactly the confusion this guards.
    #[test]
    fn the_two_address_blocks_are_distinct_fields() {
        for kind in InterviewKind::ALL {
            let Some(form) = form_for(kind) else { continue };
            assert_ne!(form.adresse, form.pharmacie_adresse, "{}", kind.label());
        }
    }

    #[test]
    fn only_the_convention_themes_have_a_bulletin() {
        assert!(has_bulletin(InterviewKind::Bpm));
        assert!(has_bulletin(InterviewKind::Avk));
        assert!(has_bulletin(InterviewKind::AnticancereuxLc));
        assert!(has_bulletin(InterviewKind::AnticancereuxAutres));
        assert!(!has_bulletin(InterviewKind::TrodAngine));
        assert!(!has_bulletin(InterviewKind::Vaccination));
        assert!(!has_bulletin(InterviewKind::Prevention));
    }

    #[test]
    fn an_address_splits_at_its_last_comma() {
        assert_eq!(
            split_address("12 rue des Lilas, 34000 Montpellier"),
            [
                "12 rue des Lilas".to_owned(),
                "34000 Montpellier".to_owned()
            ]
        );
        // No comma: everything on the first line, second left blank.
        assert_eq!(
            split_address("12 rue des Lilas"),
            ["12 rue des Lilas".to_owned(), String::new()]
        );
        assert_eq!(split_address(""), [String::new(), String::new()]);
    }

    #[test]
    fn french_names_survive_the_latin1_round_trip() {
        assert_eq!(
            latin1("Hélène Lefèvre"),
            b"H\xe9l\xe8ne Lef\xe8vre".to_vec()
        );
        // Parentheses and backslashes cannot break out of the string.
        assert_eq!(pdf_literal(b"a(b)c\\d"), b"(a\\(b\\)c\\\\d)".to_vec());
    }

    #[test]
    fn filling_writes_the_values_and_leaves_the_blanks_alone() {
        let patient = sample_patient();
        let pharmacy = sample_pharmacy();
        let bytes =
            fill(InterviewKind::Bpm, &patient, &pharmacy).expect("le bulletin doit se remplir");
        assert!(bytes.starts_with(b"%PDF-"));
        let doc = Document::load_mem(&bytes).expect("le résultat doit se relire");

        let value = |name: &str| -> Option<String> {
            let id = find_field(&doc, name)?;
            let v = doc.get_object(id).ok()?.as_dict().ok()?.get(b"V").ok()?;
            let bytes = v.as_str().ok()?;
            Some(bytes.iter().map(|&b| b as char).collect())
        };
        assert_eq!(value(BPM.nom).as_deref(), Some("Hélène Lefèvre"));
        assert_eq!(value(BPM.naissance).as_deref(), Some("27/09/1952"));
        assert_eq!(value(BPM.nir).as_deref(), Some("2 52 09 34 172 042 11"));
        assert_eq!(value(BPM.regime).as_deref(), Some("01"));
        assert_eq!(value(BPM.adresse[0]).as_deref(), Some("12 rue des Lilas"));
        assert_eq!(value(BPM.adresse[1]).as_deref(), Some("34000 Montpellier"));
        assert_eq!(value(BPM.pharmacie_am).as_deref(), Some("3400123"));
        assert_eq!(value(BPM.medecin).as_deref(), Some("Dr Morel"));
        // Consent is never pre-ticked: no checkbox carries a value.
        for (_, object) in doc.objects.iter() {
            let Ok(dict) = object.as_dict() else { continue };
            if dict.get(b"FT").ok().and_then(|f| f.as_name().ok()) == Some(b"Btn") {
                assert!(
                    dict.get(b"V").is_err(),
                    "une case a été cochée à la place du patient"
                );
            }
        }
    }

    #[test]
    fn a_patient_without_a_nir_leaves_that_line_blank() {
        let mut patient = sample_patient();
        patient.nir.clear();
        patient.regime.clear();
        let mut pharmacy = sample_pharmacy();
        pharmacy.am_number.clear();
        let bytes =
            fill(InterviewKind::Aod, &patient, &pharmacy).expect("le bulletin doit se remplir");
        let doc = Document::load_mem(&bytes).expect("le résultat doit se relire");
        for name in [AOD.nir, AOD.pharmacie_am] {
            let id = find_field(&doc, name).expect("le champ existe");
            let dict = doc.get_object(id).unwrap().as_dict().unwrap();
            assert!(dict.get(b"V").is_err(), "« {name} » aurait dû rester vide");
        }
        // The name is still filled, so the form is not simply untouched.
        let id = find_field(&doc, AOD.nom).unwrap();
        assert!(doc
            .get_object(id)
            .unwrap()
            .as_dict()
            .unwrap()
            .get(b"V")
            .is_ok());
    }

    /// For manual inspection: BPM_CADDY_TEST_PDF_OUT=/some/dir cargo test
    /// bulletin_samples — writes one filled bulletin per theme.
    #[test]
    fn bulletin_samples_render_for_every_theme() {
        let (patient, pharmacy) = (sample_patient(), sample_pharmacy());
        for kind in InterviewKind::ALL {
            if !has_bulletin(kind) {
                continue;
            }
            let bytes = fill(kind, &patient, &pharmacy).expect("le bulletin doit se remplir");
            assert!(bytes.starts_with(b"%PDF-"));
            if let Ok(dir) = std::env::var("BPM_CADDY_TEST_PDF_OUT") {
                let name = format!("bulletin_{}.pdf", kind.as_str().to_lowercase());
                let _ = std::fs::write(std::path::Path::new(&dir).join(name), &bytes);
            }
        }
    }

    #[test]
    fn an_act_without_an_adhesion_is_refused_rather_than_guessed() {
        let err = fill(
            InterviewKind::TrodAngine,
            &sample_patient(),
            &sample_pharmacy(),
        )
        .unwrap_err();
        assert!(err.contains("bulletin"), "{err}");
    }
}
