//! Ce que couvre une délivrance : jusqu'à quel jour la boîte tient, et
//! à partir de quel jour la suivante se délivre.
//!
//! « Mirtazapine (28) 1-0-1 » tient quatorze jours, « metformine 500
//! (90) 1-0-1 » quarante-cinq : deux boîtes délivrées le même jour ne
//! finissent pas ensemble, et le patient revient pour l'une quand
//! l'autre n'est qu'à moitié. Le module rend, pour chaque ligne, la
//! période couverte, le jour de la délivrance suivante et, quand une
//! durée de traitement est donnée (QSP), les délivrances qui la
//! complètent. La vue les pose sur une même échelle de temps.
//!
//! # Les règles
//!
//! * **La quantité vient de la posologie écrite, jamais d'une
//!   supposition.** Une posologie qui ne dit pas combien par prise
//!   (« 2 fois par jour ») ne donne pas de durée : la ligne dit ce qui
//!   manque, comme `renewal.rs` sans date.
//! * **Une journée entamée n'est pas couverte.** 30 comprimés à 1-0-1
//!   font quinze jours ; 31 en font quinze aussi, et le trente et unième
//!   n'ouvre pas un seizième jour.
//! * **Le reste à ce jour est une estimation**, qui suppose la
//!   posologie suivie depuis la délivrance ; la vue le dit.
//!
//! Pur et testé, sans horloge : le jour est passé.

use crate::date::{add_days, days_between};

/// Une ligne de délivrance, telle qu'elle se tape au comptoir.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Line {
    pub label: String,
    /// Unités par boîte (comprimés, gélules, sachets…).
    pub units: u32,
    /// Boîtes délivrées. 0 se lit 1.
    pub boxes: u32,
    pub posology: String,
    /// ISO. Vide = non noté.
    pub delivered_on: String,
    /// Durée de traitement prescrite, en jours. 0 = non dite.
    pub qsp_days: u32,
}

/// Ce qui manque pour conclure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Missing {
    Units,
    Posology,
    Date,
}

/// Une période sur l'échelle : délivrée, ou à délivrer pour compléter
/// la durée prescrite. Bornes ISO, incluses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    pub from: String,
    pub to: String,
    pub delivered: bool,
}

/// La lecture d'une ligne.
#[derive(Clone, Debug, PartialEq)]
pub struct Coverage {
    /// Unités par jour, lues dans la posologie.
    pub per_day: f64,
    /// Jours couverts par la délivrance.
    pub days: u32,
    /// Dernier jour couvert. Vide quand la quantité ne couvre pas un jour.
    pub until: String,
    /// Premier jour où la délivrance suivante est nécessaire.
    pub next_from: String,
    /// Fin de la durée prescrite, quand elle est donnée.
    pub qsp_end: String,
    /// Délivrances qu'il reste à faire pour aller au bout de la durée
    /// prescrite (celle du jour non comprise).
    pub deliveries_left: u32,
    /// Unités restantes au matin de `today`, si la posologie est suivie.
    pub left_today: f64,
    pub spans: Vec<Span>,
}

/// Au-delà, l'échelle ne dessine plus chaque délivrance : un an de
/// boîtes d'une semaine resterait lisible, pas un an de boîtes d'un jour.
pub const MAX_SPANS: usize = 26;

/// Unités par jour, quand la posologie place chaque prise avec sa
/// quantité (« 1-0-1 », « 1 le matin, ½ le soir »).
pub fn per_day(posology: &str) -> Option<f64> {
    let r = crate::intake::read(posology);
    if r.kind != crate::intake::Kind::Placed {
        return None;
    }
    let mut quarters = 0;
    let mut any = false;
    for take in r.doses.iter().flatten() {
        quarters += take.quarters?;
        any = true;
    }
    (any && quarters > 0).then(|| f64::from(quarters) / 4.0)
}

/// Lire une ligne au jour `today`.
pub fn read(line: &Line, today: &str) -> Result<Coverage, Vec<Missing>> {
    let mut missing = Vec::new();
    if line.units == 0 {
        missing.push(Missing::Units);
    }
    let per_day = per_day(&line.posology);
    if per_day.is_none() {
        missing.push(Missing::Posology);
    }
    let start = line.delivered_on.trim();
    if days_between(start, start).is_none() {
        missing.push(Missing::Date);
    }
    let Some(per_day) = per_day.filter(|_| missing.is_empty()) else {
        return Err(missing);
    };
    let total = f64::from(line.units) * f64::from(line.boxes.max(1));
    let days = (total / per_day + 1e-9).floor() as u32;
    let until = if days > 0 {
        add_days(start, i64::from(days) - 1).unwrap_or_default()
    } else {
        String::new()
    };
    let next_from = add_days(start, i64::from(days)).unwrap_or_default();
    let elapsed = days_between(start, today).unwrap_or(0).max(0) as f64;
    let left_today = (total - per_day * elapsed).max(0.0);

    let mut spans = Vec::new();
    if days > 0 {
        spans.push(Span {
            from: start.to_owned(),
            to: until.clone(),
            delivered: true,
        });
    }
    let mut qsp_end = String::new();
    let mut deliveries_left = 0;
    if line.qsp_days > 0 {
        qsp_end = add_days(start, i64::from(line.qsp_days) - 1).unwrap_or_default();
        if days > 0 && line.qsp_days > days {
            let rest = line.qsp_days - days;
            deliveries_left = rest.div_ceil(days);
            let mut from = next_from.clone();
            while from <= qsp_end && spans.len() < MAX_SPANS {
                let to = add_days(&from, i64::from(days) - 1).unwrap_or_default();
                let to = if to > qsp_end { qsp_end.clone() } else { to };
                let next = add_days(&to, 1).unwrap_or_default();
                spans.push(Span {
                    from,
                    to,
                    delivered: false,
                });
                from = next;
            }
        }
    }
    Ok(Coverage {
        per_day,
        days,
        until,
        next_from,
        qsp_end,
        deliveries_left,
        left_today,
        spans,
    })
}

/// Lire une ligne tapée d'un trait : « mirtazapine 15 (28) 1-0-1 »,
/// « metformine 500 (2x90) 1-0-1 ». Le libellé avant la parenthèse, les
/// unités (et les boîtes) dedans, la posologie après. `None` sans
/// parenthèse lisible.
pub fn parse_quick(text: &str) -> Option<Line> {
    let open = text.find('(')?;
    let close = open + text[open..].find(')')?;
    let label = text[..open].trim();
    let inside: String = text[open + 1..close]
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_lowercase();
    let (boxes, units) = match inside.split_once(['x', '×', '*']) {
        Some((b, u)) => (b.parse().ok()?, u.parse().ok()?),
        None => (1, inside.parse().ok()?),
    };
    if label.is_empty() || units == 0 {
        return None;
    }
    Some(Line {
        label: label.to_owned(),
        units,
        boxes,
        posology: text[close + 1..].trim().to_owned(),
        ..Default::default()
    })
}

/// L'échelle commune : du premier jour délivré au dernier jour dessiné,
/// `today` compris.
pub fn window<'a>(readings: impl Iterator<Item = &'a Coverage>, today: &str) -> (String, String) {
    let mut from = today.to_owned();
    let mut to = today.to_owned();
    for c in readings {
        for s in &c.spans {
            if s.from < from {
                from = s.from.clone();
            }
            if s.to > to {
                to = s.to.clone();
            }
        }
        if !c.qsp_end.is_empty() && c.qsp_end > to {
            to = c.qsp_end.clone();
        }
        if !c.next_from.is_empty() && c.next_from > to {
            to = c.next_from.clone();
        }
    }
    (from, to)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(units: u32, posology: &str, qsp: u32) -> Line {
        Line {
            label: "x".to_owned(),
            units,
            boxes: 1,
            posology: posology.to_owned(),
            delivered_on: "2026-10-08".to_owned(),
            qsp_days: qsp,
        }
    }

    /// L'exemple du comptoir : deux boîtes du même jour, deux fins.
    #[test]
    fn two_boxes_of_the_same_day_end_on_different_days() {
        let mirta = read(&line(28, "1-0-1", 0), "2026-10-08").unwrap();
        assert_eq!((mirta.days, mirta.until.as_str()), (14, "2026-10-21"));
        assert_eq!(mirta.next_from, "2026-10-22");
        let metf = read(&line(90, "1-0-1", 0), "2026-10-08").unwrap();
        assert_eq!((metf.days, metf.next_from.as_str()), (45, "2026-11-22"));
    }

    #[test]
    fn a_started_day_is_not_covered() {
        assert_eq!(read(&line(31, "1-0-1", 0), "2026-10-08").unwrap().days, 15);
        assert_eq!(read(&line(30, "½-0-½", 0), "2026-10-08").unwrap().days, 30);
        let mut two = line(28, "1-0-0", 0);
        two.boxes = 2;
        assert_eq!(read(&two, "2026-10-08").unwrap().days, 56);
    }

    /// Une posologie sans quantité par prise ne donne pas de durée.
    #[test]
    fn silence_on_the_quantity_is_not_one() {
        let err = read(&line(30, "2 fois par jour", 0), "2026-10-08").unwrap_err();
        assert_eq!(err, vec![Missing::Posology]);
        let mut none = line(0, "1-0-1", 0);
        none.delivered_on.clear();
        assert_eq!(
            read(&none, "2026-10-08").unwrap_err(),
            vec![Missing::Units, Missing::Date]
        );
    }

    /// Trois mois prescrits, des boîtes de 45 jours : deux délivrances
    /// de plus, la dernière tronquée à la fin de la durée.
    #[test]
    fn the_prescribed_duration_is_completed_by_planned_deliveries() {
        let c = read(&line(90, "1-0-1", 91), "2026-10-08").unwrap();
        assert_eq!(c.deliveries_left, 2);
        assert_eq!(c.qsp_end, "2027-01-06");
        let planned: Vec<_> = c.spans.iter().filter(|s| !s.delivered).collect();
        assert_eq!(planned.len(), 2);
        assert_eq!(planned[0].from, "2026-11-22");
        assert_eq!(planned[1].to, "2027-01-06");
        // Une boîte d'un jour sur un an : l'échelle s'arrête au plafond.
        let tiny = read(&line(1, "1-0-0", 365), "2026-10-08").unwrap();
        assert_eq!(tiny.spans.len(), MAX_SPANS);
    }

    #[test]
    fn what_is_left_this_morning_follows_the_posology() {
        let c = read(&line(28, "1-0-1", 0), "2026-10-13").unwrap();
        assert_eq!(c.left_today, 18.0);
        assert_eq!(
            read(&line(28, "1-0-1", 0), "2026-12-01")
                .unwrap()
                .left_today,
            0.0
        );
    }

    #[test]
    fn a_line_is_typed_in_one_go() {
        let l = parse_quick("mirtazapine 15 (28) 1-0-1").unwrap();
        assert_eq!(
            (l.label.as_str(), l.units, l.boxes),
            ("mirtazapine 15", 28, 1)
        );
        assert_eq!(l.posology, "1-0-1");
        let l = parse_quick("metformine 500 (2 x 90) 1-0-1").unwrap();
        assert_eq!((l.units, l.boxes), (90, 2));
        assert!(parse_quick("metformine 1-0-1").is_none());
        assert!(parse_quick("(30) 1-0-1").is_none());
    }

    #[test]
    fn the_window_holds_every_line_and_today() {
        let a = read(&line(28, "1-0-1", 0), "2026-10-08").unwrap();
        let b = read(&line(90, "1-0-1", 0), "2026-10-08").unwrap();
        let (from, to) = window([&a, &b].into_iter(), "2026-10-08");
        assert_eq!((from.as_str(), to.as_str()), ("2026-10-08", "2026-11-22"));
    }
}
