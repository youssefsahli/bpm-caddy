//! Where a file's ordonnance stands, in one sentence, and which files
//! come due soon. The arithmetic is `bpm_caddy::renewal`'s; this module
//! only words it and gathers it across the base.

use crate::t::{t, tf};
use bpm_caddy::db::Db;
use bpm_caddy::renewal::{self, Prescription, Stand, State};

/// One sentence for a stand, and whether it asks for action now.
pub fn sentence(stand: &Stand) -> (String, bool) {
    let date = |d: &Option<String>| d.as_deref().map(crate::ui::date).unwrap_or_default();
    match stand.state {
        State::Unknown => (tf("renouv.unknown", &[&stand.missing]), false),
        State::Over => (tf("renouv.over", &[&date(&stand.ends_on)]), true),
        State::Last => (tf("renouv.last", &[&date(&stand.ends_on)]), true),
        State::Running => {
            let soon = stand.see_by.is_none() && stand.days_left.is_some_and(|d| d >= 0);
            let mut s = tf(
                "renouv.running",
                &[&stand.step, &stand.steps, &date(&stand.ends_on)],
            );
            if stand.covered_to.is_some() {
                s.push_str(&tf("renouv.covered", &[&date(&stand.covered_to)]));
            }
            (s, soon)
        }
    }
}

/// A file whose ordonnance ends within the notice, or has ended less
/// than a month ago — the ones to call.
#[derive(Clone, Debug, PartialEq)]
pub struct Due {
    pub patient_id: i64,
    pub name: String,
    pub ends_on: String,
    pub days_left: i64,
}

/// Every file whose ordonnance comes due: `days_left` within
/// [-30, notice]. Soonest first. One query per file — read once per base
/// generation by the caller.
pub fn due(db: &Db, patients: &[bpm_caddy::db::Patient], today: &str, notice: u32) -> Vec<Due> {
    let mut out = Vec::new();
    for p in patients {
        let Ok(lines) = db.patient_prescriptions(p.id) else {
            continue;
        };
        let worst = lines
            .iter()
            .map(|(_, rx)| renewal::read(rx, today, notice))
            .filter_map(|s| Some((s.days_left?, s.ends_on?)))
            .min_by_key(|(d, _)| *d);
        if let Some((days_left, ends_on)) = worst {
            if (-30..=i64::from(notice)).contains(&days_left) {
                out.push(Due {
                    patient_id: p.id,
                    name: format!("{} {}", p.last_name.to_uppercase(), p.first_name),
                    ends_on,
                    days_left,
                });
            }
        }
    }
    out.sort_by_key(|d| d.days_left);
    out
}

/// The words for a due line: « dans 3 jours », « aujourd'hui », « depuis 4 jours ».
pub fn when(days_left: i64) -> String {
    match days_left {
        0 => t("renouv.today").to_owned(),
        d if d > 0 => tf("renouv.in_days", &[&d]),
        d => tf("renouv.since_days", &[&-d]),
    }
}

/// The prescription the patient's lines share most, to fill the form.
pub fn common(lines: &[(i64, Prescription)]) -> Prescription {
    let named: Vec<(String, Prescription)> = lines
        .iter()
        .map(|(id, p)| (id.to_string(), p.clone()))
        .collect();
    renewal::group(&named)
        .into_iter()
        .max_by_key(|(_, names)| names.len())
        .map(|(p, _)| p)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rx(on: &str, days: u32, renewals: u32, dispensed: u32) -> Prescription {
        Prescription {
            prescribed_on: on.into(),
            duration_days: days,
            renewals,
            dispensed,
            dispensed_on: on.into(),
        }
    }

    #[test]
    fn a_sentence_for_every_state() {
        let (s, act) = sentence(&renewal::read(&Prescription::default(), "2026-10-01", 7));
        assert!(!s.is_empty() && !act);
        // Three months from 1 July: ends 28 September.
        let (s, act) = sentence(&renewal::read(&rx("2026-07-01", 30, 2, 1), "2026-10-15", 7));
        assert!(act, "{s}");
        let (s, _) = sentence(&renewal::read(&rx("2026-10-01", 30, 2, 1), "2026-10-05", 7));
        assert!(s.contains("1/3"), "{s}");
    }

    #[test]
    fn due_lists_what_ends_soon_and_not_what_is_far() {
        let (s, dir) = crate::session::tests::scratch("renouv");
        let a = s.db.add_patient_with_sex("Soon", "A", "", "").unwrap();
        let b = s.db.add_patient_with_sex("Far", "B", "", "").unwrap();
        let drug = s.drugs[0].id;
        for pid in [a, b] {
            s.db.add_patient_drug(pid, drug).unwrap();
        }
        let today = s.today.clone();
        // A: one month started 25 days ago, ends in 4 days.
        let start_a = bpm_caddy::date::add_days(&today, -25).unwrap();
        let start_b = bpm_caddy::date::add_days(&today, -2).unwrap();
        for (pid, start, renewals) in [(a, start_a, 0), (b, start_b, 2)] {
            let new = rx(&start, 30, renewals, 1);
            assert!(s
                .db
                .set_patient_prescription(pid, drug, &new, &Prescription::default())
                .unwrap());
        }
        let patients = s.db.patients().unwrap();
        let list = due(&s.db, &patients, &today, 7);
        assert_eq!(list.len(), 1, "{list:?}");
        assert_eq!(list[0].patient_id, a);
        assert_eq!(list[0].days_left, 4);
        assert_eq!(when(4), tf("renouv.in_days", &[&4]));
        let _ = std::fs::remove_dir_all(dir);
    }
}
