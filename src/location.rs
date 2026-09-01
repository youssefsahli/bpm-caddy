//! Ce qu'une location de matériel doit à l'officine, et quand elle se
//! renouvelle.
//!
//! Une location au forfait ne se facture pas comme un acte : elle court.
//! Un nébuliseur posé le 3 mars et repris le 28 avril, c'est un nombre
//! de périodes entamées multiplié par un forfait, et une date à laquelle
//! l'ordonnance devait être renouvelée. Ces deux calculs sont ici,
//! purs et testés, sans horloge interne — le jour est passé en
//! paramètre, comme dans [`crate::vaccines`].
//!
//! Les forfaits eux-mêmes ne sont pas dans le code : ils vivent dans
//! `[locations]` de `config.toml`, éditables dans Options › Locations.
//! L'application ne connaît aucun tarif de sa propre autorité — la LPP
//! bouge, et un montant livré serait faux dans l'année.

use crate::config::Period;

/// One period of a running rental, as the counter counts them: a period
/// *entamée* is due, because the material was out of the officine for
/// it. Rounding the other way would mean lending the last week.
///
/// `start` and `end` are ISO days, `end` inclusive. Returns `None` when
/// either date is unreadable or when the end precedes the start — a
/// backwards rental is a typing mistake, not a credit note.
pub fn periods_between(start: &str, end: &str, period: Period) -> Option<u32> {
    let days = days_between(start, end)?;
    if days < 0 {
        return None;
    }
    // Inclusive: a rental that starts and ends the same day is one day.
    let days = days as u32 + 1;
    let len = period.days();
    Some(days.div_ceil(len))
}

/// What a running rental has earned by `today`, forfait included.
///
/// `until` is the day the material came back; empty means it is still
/// out, and the count runs to `today`. A rental capped at `max_periods`
/// stops there — the LPP line often pays a fixed number of weeks and
/// nothing beyond, and billing past it is what gets an indu.
pub fn amount_due(
    start: &str,
    until: &str,
    today: &str,
    period: Period,
    fee: f64,
    max_periods: u32,
) -> Option<(u32, f64)> {
    let end = if until.trim().is_empty() {
        today
    } else {
        until.trim()
    };
    let mut periods = periods_between(start, end, period)?;
    if max_periods > 0 {
        periods = periods.min(max_periods);
    }
    Some((periods, periods as f64 * fee))
}

/// The day the prescription has to be renewed: `renewal_days` after the
/// start, or after the last renewal the team recorded. Zero disables the
/// rule and returns `None` — not every rental is time-limited.
pub fn next_renewal(from: &str, renewal_days: u32) -> Option<String> {
    if renewal_days == 0 {
        return None;
    }
    add_days(from, renewal_days as i64)
}

/// How a renewal date reads against today: overdue, due within the
/// notice, or still far off. The dashboard sorts on this.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Standing {
    Overdue,
    Soon,
    Later,
}

/// Where a renewal date stands, given the notice the officine wants.
pub fn standing(renewal: &str, today: &str, notice_days: u32) -> Option<Standing> {
    let left = days_between(today, renewal)?;
    Some(if left < 0 {
        Standing::Overdue
    } else if left <= notice_days as i64 {
        Standing::Soon
    } else {
        Standing::Later
    })
}

// Le calendrier vit dans `crate::date`. Ce module en portait une copie
// entière — `julian`, `from_julian`, `parse_iso` — pendant que
// `ordonnancier` en portait une autre, écrite avec la formule julienne.
// Aucune des deux n'était fausse, et c'est précisément la situation que
// la maison refuse ailleurs : deux mesures d'une même chose finissent
// par diverger, et celle qui divergera est celle que personne ne relit.
use crate::date::{add_days, days_between};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_period_entamee_is_due() {
        // One day out is one week due: the material left the officine.
        assert_eq!(
            periods_between("2026-03-03", "2026-03-03", Period::Week),
            Some(1)
        );
        // Seven days inclusive is still one week; the eighth starts the
        // second.
        assert_eq!(
            periods_between("2026-03-03", "2026-03-09", Period::Week),
            Some(1)
        );
        assert_eq!(
            periods_between("2026-03-03", "2026-03-10", Period::Week),
            Some(2)
        );
        // A rental that ends before it starts is a typing mistake.
        assert_eq!(
            periods_between("2026-03-10", "2026-03-03", Period::Week),
            None
        );
        assert_eq!(
            periods_between("pas une date", "2026-03-03", Period::Week),
            None
        );
    }

    #[test]
    fn the_month_and_the_day_count_the_same_way() {
        assert_eq!(
            periods_between("2026-01-01", "2026-01-30", Period::Month),
            Some(1)
        );
        assert_eq!(
            periods_between("2026-01-01", "2026-01-31", Period::Month),
            Some(2)
        );
        assert_eq!(
            periods_between("2026-01-01", "2026-01-03", Period::Day),
            Some(3)
        );
    }

    #[test]
    fn a_running_rental_counts_to_today_and_a_returned_one_to_its_end() {
        // Still out: counted to today.
        let (periods, due) =
            amount_due("2026-03-03", "", "2026-03-24", Period::Week, 12.0, 0).unwrap();
        assert_eq!(periods, 4);
        assert_eq!(due, 48.0);
        // Returned: counted to the day it came back, whatever today is.
        let (periods, due) = amount_due(
            "2026-03-03",
            "2026-03-16",
            "2026-06-01",
            Period::Week,
            12.0,
            0,
        )
        .unwrap();
        assert_eq!(periods, 2);
        assert_eq!(due, 24.0);
    }

    /// The LPP often pays a fixed number of periods and nothing after.
    /// Billing past the cap is what gets an indu, so the cap is in the
    /// arithmetic and not in a remark nobody reads.
    #[test]
    fn the_cap_stops_the_count() {
        let (periods, due) =
            amount_due("2026-01-01", "", "2026-12-31", Period::Week, 10.0, 6).unwrap();
        assert_eq!(periods, 6);
        assert_eq!(due, 60.0);
        // Zero means no cap.
        let (periods, _) = amount_due(
            "2026-01-01",
            "2026-02-05",
            "2026-02-05",
            Period::Week,
            10.0,
            0,
        )
        .unwrap();
        assert_eq!(periods, 6);
    }

    #[test]
    fn the_renewal_lands_where_the_calendar_says() {
        assert_eq!(
            next_renewal("2026-03-03", 28).as_deref(),
            Some("2026-03-31")
        );
        // Across a month end, and across a leap February.
        assert_eq!(
            next_renewal("2026-01-20", 30).as_deref(),
            Some("2026-02-19")
        );
        assert_eq!(next_renewal("2024-02-27", 3).as_deref(), Some("2024-03-01"));
        // Zero disables the rule rather than answering today.
        assert_eq!(next_renewal("2026-03-03", 0), None);
    }

    #[test]
    fn a_renewal_reads_overdue_soon_or_later() {
        assert_eq!(
            standing("2026-03-01", "2026-03-03", 7),
            Some(Standing::Overdue)
        );
        assert_eq!(
            standing("2026-03-08", "2026-03-03", 7),
            Some(Standing::Soon)
        );
        assert_eq!(
            standing("2026-03-03", "2026-03-03", 7),
            Some(Standing::Soon)
        );
        assert_eq!(
            standing("2026-04-08", "2026-03-03", 7),
            Some(Standing::Later)
        );
        assert_eq!(standing("", "2026-03-03", 7), None);
    }

    /// The date arithmetic is the part that silently goes wrong, so it
    /// is checked against the awkward days rather than the easy ones.
    #[test]
    fn the_calendar_survives_the_awkward_days() {
        for (from, days, want) in [
            ("2026-12-31", 1, "2027-01-01"),
            ("2024-02-28", 1, "2024-02-29"),
            ("2025-02-28", 1, "2025-03-01"),
            ("2000-02-28", 2, "2000-03-01"),
            ("1900-02-28", 1, "1900-03-01"),
            ("2026-08-27", 365, "2027-08-27"),
        ] {
            assert_eq!(
                add_days(from, days).as_deref(),
                Some(want),
                "{from} + {days} j"
            );
        }
        assert_eq!(add_days("2026-13-01", 1), None);
        assert_eq!(add_days("", 1), None);
    }
}
