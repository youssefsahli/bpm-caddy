//! What v2 prints, through the first window's templates (`pdf::DOCS`):
//! the officine's own rewrites of a template apply here too, since the
//! template path comes from the same config.toml.

use crate::session::Session;
use bpm_caddy::config::Config;
use bpm_caddy::db::{Drug, Posologie};
use std::collections::HashMap;
use std::path::PathBuf;

/// The four columns of the plan de prise, one line per treatment:
/// (name with its strength, what it is for, when, a missed dose).
///
/// The same rules as the first window: « when » is this patient's
/// posology before the card's usual one — a usual schema printed on a
/// sheet that goes home is how a patient reads a schema not theirs —
/// and the strength rides with the name, because the person reading the
/// sheet has the box in hand.
pub fn plan_lines(
    drugs: &[Drug],
    dosage: &HashMap<i64, String>,
    posology: &HashMap<i64, String>,
    posologies: impl Fn(i64) -> Vec<Posologie>,
) -> Vec<(String, String, String, String)> {
    drugs
        .iter()
        .map(|d| {
            let poso = posologies(d.id);
            let first = poso.first();
            let what = first
                .map(|p| p.indication.trim().to_owned())
                .filter(|i| !i.is_empty())
                .unwrap_or_else(|| d.class.trim().to_owned());
            let own = posology.get(&d.id).map(|s| s.trim()).unwrap_or_default();
            let when = if own.is_empty() {
                first
                    .map(|p| p.posologie.trim().to_owned())
                    .filter(|p| !p.is_empty())
                    .unwrap_or_else(|| d.dosage.trim().to_owned())
            } else {
                own.to_owned()
            };
            let missed = if d.missed_dose.trim().is_empty() {
                first
                    .map(|p| p.remarque.trim().to_owned())
                    .unwrap_or_default()
            } else {
                d.missed_dose.trim().to_owned()
            };
            let strength = dosage.get(&d.id).map(|s| s.trim()).unwrap_or_default();
            let name = if strength.is_empty() {
                d.name.trim().to_owned()
            } else {
                format!("{} {strength}", d.name.trim())
            };
            (name, what, when, missed)
        })
        .collect()
}

type PlanParts = (
    bpm_caddy::db::Patient,
    Vec<(String, String, String, String)>,
    String,
    String,
);

/// What the plan needs from the base: the file, its lines, the day in
/// French and the signature.
fn plan_parts(s: &Session, patient_id: i64, config: &Config) -> Result<PlanParts, String> {
    let patient = s
        .patient(patient_id)
        .cloned()
        .ok_or_else(|| crate::t::t("print.no_patient").to_owned())?;
    let mut drugs = s.db.drugs_for_patient(patient_id)?;
    drugs.sort_by_cached_key(|d| bpm_caddy::fuzzy::sort_key(&d.name));
    let dosage: HashMap<i64, String> = s.db.patient_dosages(patient_id)?.into_iter().collect();
    let posology: HashMap<i64, String> = s.db.patient_posologies(patient_id)?.into_iter().collect();
    let lines = plan_lines(&drugs, &dosage, &posology, |id| {
        s.db.posologies(id).unwrap_or_default()
    });
    let today = s.db.today_french()?;
    let signature = config.pharmacy.signature_for(&s.operator);
    Ok((patient, lines, today, signature))
}

/// The plan de prise of a file, filled (not yet compiled).
pub fn plan_source(s: &Session, patient_id: i64, config: &Config) -> Result<String, String> {
    let (patient, lines, today, signature) = plan_parts(s, patient_id, config)?;
    let data = bpm_caddy::pdf::PlanData {
        patient: &patient,
        today: &today,
        lines,
        mention: &config.disclaimers.plan,
        signature: &signature,
    };
    bpm_caddy::pdf::plan_source(&data, &config.pharmacy, &config.doc_template_path("plan"))
}

/// Print the plan de prise: compile, open in the PDF reader.
pub fn plan(s: &Session, patient_id: i64) -> Result<PathBuf, String> {
    // A test (the random-use pass clicks everything) never opens a viewer.
    if cfg!(test) {
        return Err(crate::t::t("print.not_in_tests").to_owned());
    }
    let config = Config::load();
    let (patient, lines, today, signature) = plan_parts(s, patient_id, &config)?;
    let data = bpm_caddy::pdf::PlanData {
        patient: &patient,
        today: &today,
        lines,
        mention: &config.disclaimers.plan,
        signature: &signature,
    };
    bpm_caddy::pdf::open_plan(&data, &config.pharmacy, &config.doc_template_path("plan"))
}

/// Print a drug card as its monograph. The sourced pharmacokinetic
/// values and the CYP routes section stay with the first window's card
/// view, which computes them against the whole base.
pub fn monograph(s: &Session, drug_id: i64) -> Result<PathBuf, String> {
    if cfg!(test) {
        return Err(crate::t::t("print.not_in_tests").to_owned());
    }
    let config = Config::load();
    let d = s
        .drug(drug_id)
        .cloned()
        .ok_or_else(|| crate::t::t("print.no_drug").to_owned())?;
    let posologies = s.db.posologies(drug_id).unwrap_or_default();
    bpm_caddy::pdf::open_drug_monograph(
        &d,
        &posologies,
        &[],
        "",
        &config.doc_template_path("monographie"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(id: i64, name: &str) -> Drug {
        Drug {
            id,
            name: name.into(),
            class: "statine".into(),
            dosage: "1 cp le soir".into(),
            ..Default::default()
        }
    }

    #[test]
    fn the_files_posology_wins_and_the_strength_rides_with_the_name() {
        let drugs = [d(1, "Tahor"), d(2, "Kardegic")];
        let dosage = HashMap::from([(1, "20 mg".to_owned())]);
        let posology = HashMap::from([(1, "1 le soir au coucher".to_owned())]);
        let lines = plan_lines(&drugs, &dosage, &posology, |id| {
            if id == 2 {
                vec![Posologie {
                    id: 1,
                    indication: "Prévention cardiovasculaire".into(),
                    posologie: "1 sachet le midi".into(),
                    remarque: "Ne pas doubler la dose.".into(),
                }]
            } else {
                Vec::new()
            }
        });
        assert_eq!(lines[0].0, "Tahor 20 mg");
        assert_eq!(lines[0].1, "statine");
        assert_eq!(lines[0].2, "1 le soir au coucher");
        assert_eq!(lines[1].0, "Kardegic");
        assert_eq!(lines[1].1, "Prévention cardiovasculaire");
        assert_eq!(lines[1].2, "1 sachet le midi");
        assert_eq!(lines[1].3, "Ne pas doubler la dose.");
    }

    #[test]
    fn a_plan_fills_its_template_with_the_file() {
        let (mut s, dir) = crate::session::tests::scratch("print");
        let pid =
            s.db.add_patient_with_sex("Durand", "Anne", "1950-01-01", "")
                .unwrap();
        let did = s.drugs[0].id;
        s.db.add_patient_drug(pid, did).unwrap();
        s.changed();
        let source = plan_source(&s, pid, &Config::default()).unwrap();
        assert!(source.contains("Durand"), "le nom du patient");
        assert!(source.contains(s.drugs[0].name.trim()), "le traitement");
        let _ = std::fs::remove_dir_all(dir);
    }
}
