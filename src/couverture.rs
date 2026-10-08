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

// ---------------------------------------------------------------------
// L'échelle : un médicament par rangée, toutes ses délivrances
// ---------------------------------------------------------------------

/// Ce qu'une période de la rangée représente.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SegKind {
    /// Couverte par une délivrance faite.
    Covered,
    /// Couverte par une délivrance faite **avant** la fin de la
    /// précédente : la boîte attend son tour, la période commence quand
    /// la précédente finit.
    Carried,
    /// Entre deux délivrances, sans traitement : rupture passée.
    Gap,
    /// Depuis la délivrance suivante attendue jusqu'à aujourd'hui : la
    /// boîte n'est pas venue, aujourd'hui compris.
    Overdue,
    /// Délivrances qui restent pour aller au bout de la durée prescrite.
    Planned,
    /// Ce qu'il faudrait délivrer en plus pour aligner les
    /// renouvellements sur un même jour.
    Bridge,
}

/// Une période de la rangée, bornes ISO incluses.
#[derive(Clone, Debug, PartialEq)]
pub struct Segment {
    pub from: String,
    pub to: String,
    pub kind: SegKind,
    /// Le jour de la délivrance qui couvre la période (vide hors
    /// `Covered`/`Carried`).
    pub delivered_on: String,
    /// Unités par jour pendant la période.
    pub per_day: f64,
}

impl Segment {
    pub fn days(&self) -> i64 {
        days_between(&self.from, &self.to).map_or(0, |d| d + 1)
    }
}

/// Une rangée de l'échelle : un médicament et toutes ses délivrances.
#[derive(Clone, Debug, PartialEq)]
pub struct Track {
    pub label: String,
    /// Unités par jour de la dernière délivrance.
    pub per_day: f64,
    /// Unités par boîte de la dernière délivrance.
    pub units_per_box: u32,
    /// Jours couverts par la dernière délivrance (le pas des
    /// délivrances prévues).
    pub step_days: u32,
    pub segments: Vec<Segment>,
    /// Les jours de délivrance, dans l'ordre.
    pub deliveries: Vec<String>,
    /// Dernier jour couvert par ce qui a été délivré.
    pub covered_until: String,
    /// Premier jour où une nouvelle délivrance est nécessaire.
    pub next_from: String,
    /// Fin de la durée prescrite la plus lointaine, ou vide.
    pub qsp_end: String,
    /// Délivrances qui restent pour aller au bout de la durée prescrite.
    pub deliveries_left: u32,
    /// Part des jours couverts depuis la première délivrance jusqu'à la
    /// veille de `today`, quand au moins un jour s'est écoulé.
    pub covered_share: Option<f64>,
    /// Jours de rupture passés, cumulés.
    pub gap_days: i64,
}

/// Les lignes d'un même médicament : la clé pliée, le libellé affiché,
/// et chaque ligne avec sa lecture.
type Group<'a> = (String, String, Vec<(&'a Line, Coverage)>);

/// Les rangées, une par médicament (libellés égaux au pliage près), et
/// les lignes qui n'ont pas pu se lire, avec ce qui leur manque.
pub fn tracks(lines: &[Line], today: &str) -> (Vec<Track>, Vec<(usize, Vec<Missing>)>) {
    let mut invalid = Vec::new();
    // (clé, libellé affiché, [(ligne, lecture)])
    let mut groups: Vec<Group> = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        match read(line, today) {
            Err(m) => invalid.push((i, m)),
            Ok(c) => {
                let key = crate::fuzzy::sort_key(line.label.trim());
                match groups.iter_mut().find(|(k, _, _)| *k == key) {
                    Some(g) => g.2.push((line, c)),
                    None => groups.push((key, line.label.trim().to_owned(), vec![(line, c)])),
                }
            }
        }
    }
    let tracks = groups
        .into_iter()
        .map(|(_, label, mut items)| {
            items.sort_by(|a, b| a.0.delivered_on.cmp(&b.0.delivered_on));
            track(label, &items, today)
        })
        .collect();
    (tracks, invalid)
}

fn track(label: String, items: &[(&Line, Coverage)], today: &str) -> Track {
    let mut segments = Vec::new();
    let mut deliveries = Vec::new();
    let mut end: Option<String> = None;
    let mut gap_days = 0;
    for (line, c) in items {
        let on = line.delivered_on.trim().to_owned();
        deliveries.push(on.clone());
        if c.days == 0 {
            continue;
        }
        let start = match &end {
            Some(e) if e.as_str() >= on.as_str() => add_days(e, 1).unwrap_or_default(),
            Some(e) => {
                let gap_from = add_days(e, 1).unwrap_or_default();
                let gap_to = add_days(&on, -1).unwrap_or_default();
                if gap_from <= gap_to {
                    gap_days += days_between(&gap_from, &gap_to).unwrap_or(0) + 1;
                    segments.push(Segment {
                        from: gap_from,
                        to: gap_to,
                        kind: SegKind::Gap,
                        delivered_on: String::new(),
                        per_day: 0.0,
                    });
                }
                on.clone()
            }
            None => on.clone(),
        };
        let to = add_days(&start, i64::from(c.days) - 1).unwrap_or_default();
        segments.push(Segment {
            kind: if start == on {
                SegKind::Covered
            } else {
                SegKind::Carried
            },
            from: start,
            to: to.clone(),
            delivered_on: on,
            per_day: c.per_day,
        });
        end = Some(to);
    }
    let (last_line, last) = items[items.len() - 1].clone();
    let first = items[0].0.delivered_on.trim().to_owned();
    let covered_until = end.clone().unwrap_or_default();
    let next_from = match &end {
        Some(e) => add_days(e, 1).unwrap_or_default(),
        None => first.clone(),
    };
    // La part couverte, sur les jours écoulés : de la première
    // délivrance à la veille d'aujourd'hui.
    let elapsed = days_between(&first, today).unwrap_or(0);
    let covered_share = (elapsed >= 1).then(|| {
        let yesterday = add_days(today, -1).unwrap_or_default();
        let covered: i64 = segments
            .iter()
            .filter(|s| matches!(s.kind, SegKind::Covered | SegKind::Carried))
            .map(|s| {
                let to = if s.to < yesterday {
                    s.to.as_str()
                } else {
                    yesterday.as_str()
                };
                days_between(&s.from, to).map_or(0, |d| (d + 1).max(0))
            })
            .sum();
        (covered as f64 / elapsed as f64).min(1.0)
    });
    let qsp_end = items
        .iter()
        .filter(|(l, _)| l.qsp_days > 0)
        .filter_map(|(l, _)| add_days(l.delivered_on.trim(), i64::from(l.qsp_days) - 1))
        .max()
        .unwrap_or_default();
    // La boîte qui n'est pas venue : de la délivrance attendue à hier —
    // sauf quand la durée prescrite était finie avant ce jour-là.
    let ended = !qsp_end.is_empty() && qsp_end < next_from;
    let mut planned_from = next_from.clone();
    if !ended && !next_from.is_empty() && next_from.as_str() < today {
        // Jusqu'à aujourd'hui compris : ce matin, la boîte manque encore.
        segments.push(Segment {
            from: next_from.clone(),
            to: today.to_owned(),
            kind: SegKind::Overdue,
            delivered_on: String::new(),
            per_day: 0.0,
        });
        planned_from = today.to_owned();
    }
    let mut deliveries_left = 0;
    if !qsp_end.is_empty() && last.days > 0 && planned_from <= qsp_end {
        let rest = days_between(&planned_from, &qsp_end).unwrap_or(0) + 1;
        deliveries_left = (rest as u32).div_ceil(last.days);
        let mut from = planned_from;
        let mut drawn = 0;
        while from <= qsp_end && drawn < MAX_SPANS {
            let to = add_days(&from, i64::from(last.days) - 1).unwrap_or_default();
            let to = if to > qsp_end { qsp_end.clone() } else { to };
            let next = add_days(&to, 1).unwrap_or_default();
            segments.push(Segment {
                from,
                to,
                kind: SegKind::Planned,
                delivered_on: String::new(),
                per_day: last.per_day,
            });
            from = next;
            drawn += 1;
        }
    }
    Track {
        label,
        per_day: last.per_day,
        units_per_box: last_line.units,
        step_days: last.days,
        segments,
        deliveries,
        covered_until,
        next_from,
        qsp_end,
        deliveries_left,
        covered_share,
        gap_days,
    }
}

/// Ce qu'une rangée dit d'un jour.
#[derive(Clone, Debug, PartialEq)]
pub enum DayState {
    /// Avant la première délivrance.
    Before,
    /// Couvert : le rang du jour dans sa délivrance, sur combien, et les
    /// unités que le patient a en main ce matin-là (délivrances déjà
    /// faites et pas encore entamées comprises).
    Covered { day: i64, of: i64, in_hand: f64 },
    /// Rupture passée, entre deux délivrances.
    Gap { since: String },
    /// La boîte attendue n'est pas venue.
    Overdue { since: String },
    /// À délivrer pour la durée prescrite.
    Planned,
    /// Au-delà de ce que l'échelle connaît.
    After,
}

/// L'état d'une rangée au matin de `day`.
pub fn at(track: &Track, day: &str) -> DayState {
    let Some(seg) = track
        .segments
        .iter()
        .find(|s| s.from.as_str() <= day && day <= s.to.as_str())
    else {
        return match track.deliveries.first() {
            Some(first) if day < first.as_str() => DayState::Before,
            _ => DayState::After,
        };
    };
    match seg.kind {
        SegKind::Covered | SegKind::Carried => {
            // En main : le reste de chaque délivrance déjà faite.
            let in_hand: f64 = track
                .segments
                .iter()
                .filter(|s| matches!(s.kind, SegKind::Covered | SegKind::Carried))
                .filter(|s| s.delivered_on.as_str() <= day && day <= s.to.as_str())
                .map(|s| {
                    let from = if s.from.as_str() > day {
                        s.from.as_str()
                    } else {
                        day
                    };
                    s.per_day * days_between(from, &s.to).map_or(0.0, |d| (d + 1) as f64)
                })
                .sum();
            DayState::Covered {
                day: days_between(&seg.from, day).unwrap_or(0) + 1,
                of: seg.days(),
                in_hand,
            }
        }
        SegKind::Gap => DayState::Gap {
            since: seg.from.clone(),
        },
        SegKind::Overdue => DayState::Overdue {
            since: seg.from.clone(),
        },
        SegKind::Planned | SegKind::Bridge => DayState::Planned,
    }
}

/// Ce qu'il faut délivrer en plus pour qu'une rangée tienne jusqu'à la
/// veille de `target` : la période, les unités (arrondies à l'unité
/// supérieure) et le nombre de boîtes de la dernière délivrance.
#[derive(Clone, Debug, PartialEq)]
pub struct Bridge {
    pub from: String,
    pub to: String,
    pub units: u32,
    pub boxes: u32,
}

pub fn bridge(track: &Track, target: &str) -> Option<Bridge> {
    let from = track.next_from.as_str();
    if from.is_empty() || target <= from {
        return None;
    }
    let days = days_between(from, target)?;
    let units = (track.per_day * days as f64 - 1e-9).ceil().max(1.0) as u32;
    Some(Bridge {
        from: from.to_owned(),
        to: add_days(target, -1)?,
        units,
        boxes: units.div_ceil(track.units_per_box.max(1)),
    })
}

/// Le jour proposé pour aligner les renouvellements : le plus tardif des
/// jours de délivrance suivante, pour que chaque rangée soit complétée
/// et qu'aucune ne soit délivrée en avance.
pub fn sync_target(tracks: &[Track]) -> Option<String> {
    tracks
        .iter()
        .map(|t| t.next_from.clone())
        .filter(|d| !d.is_empty())
        .max()
}

/// Le cadre commun de l'échelle et de la feuille imprimée : le jour
/// d'origine (la première délivrance, ou aujourd'hui s'il est avant) et
/// le décalage du dernier jour à montrer — ce que les rangées couvrent,
/// la durée prescrite, la délivrance suivante, le jour d'alignement.
pub fn frame(tracks: &[Track], today: &str, target: Option<&str>) -> (String, i64) {
    let first = tracks
        .iter()
        .filter_map(|t| t.deliveries.first())
        .min()
        .map_or(today, String::as_str);
    let origin = if first < today { first } else { today }.to_owned();
    let off = |d: &str| days_between(&origin, d).unwrap_or(0);
    let mut end = off(today);
    for t in tracks {
        for s in &t.segments {
            end = end.max(off(&s.to));
        }
        for d in [&t.qsp_end, &t.next_from] {
            if !d.is_empty() {
                end = end.max(off(d));
            }
        }
    }
    if let Some(tg) = target {
        end = end.max(off(tg));
    }
    (origin, end)
}

/// Les repères du calendrier entre deux jours (décalages en jours depuis
/// `origin`) : le premier de chaque mois, chaque lundi, chaque samedi.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Calendar {
    /// (décalage, « 2026-11 »)
    pub months: Vec<(i64, String)>,
    pub mondays: Vec<i64>,
    pub saturdays: Vec<i64>,
}

pub fn calendar(origin: &str, from: i64, to: i64) -> Calendar {
    let mut cal = Calendar::default();
    // Plafond de sécurité : dix ans de jours.
    let to = to.min(from + 3660);
    for off in from..=to {
        let Some(day) = add_days(origin, off) else {
            continue;
        };
        if day.get(8..10) == Some("01") || off == from {
            cal.months.push((off, day[..7].to_owned()));
        }
        match crate::date::weekday(&day) {
            Some(1) => cal.mondays.push(off),
            Some(6) => cal.saturdays.push(off),
            _ => {}
        }
    }
    cal
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

    fn on(label: &str, units: u32, day: &str, qsp: u32) -> Line {
        Line {
            label: label.to_owned(),
            units,
            boxes: 1,
            posology: "1-0-1".to_owned(),
            delivered_on: day.to_owned(),
            qsp_days: qsp,
        }
    }

    /// Deux délivrances d'un même médicament : en retard, une rupture ;
    /// en avance, la boîte attend la fin de la précédente.
    #[test]
    fn deliveries_of_one_drug_share_a_row_with_gaps_and_carry_over() {
        let late = [
            on("Mirtazapine 15", 28, "2026-09-01", 0),
            on("mirtazapine 15", 28, "2026-09-20", 0),
        ];
        let (t, bad) = tracks(&late, "2026-09-25");
        assert!(bad.is_empty());
        assert_eq!(t.len(), 1);
        let t = &t[0];
        assert_eq!(t.gap_days, 5); // du 15 au 19 septembre
        assert_eq!(t.segments[1].kind, SegKind::Gap);
        assert_eq!(t.covered_until, "2026-10-03");
        // 24 jours écoulés, 19 couverts.
        assert!((t.covered_share.unwrap() - 19.0 / 24.0).abs() < 1e-9);

        let early = [
            on("Mirtazapine", 28, "2026-09-01", 0),
            on("Mirtazapine", 28, "2026-09-10", 0),
        ];
        let (t, _) = tracks(&early, "2026-09-12");
        let t = &t[0];
        assert_eq!(t.segments[1].kind, SegKind::Carried);
        assert_eq!(t.segments[1].from, "2026-09-15");
        assert_eq!(t.next_from, "2026-09-29");
        // Le 12 au matin : 3 jours de la première boîte et la seconde
        // entière, soit 6 + 28 unités.
        assert_eq!(
            at(t, "2026-09-12"),
            DayState::Covered {
                day: 12,
                of: 14,
                in_hand: 34.0
            }
        );
    }

    /// La boîte qui n'est pas venue se voit jusqu'à hier, et les
    /// délivrances prévues partent d'aujourd'hui.
    #[test]
    fn a_box_that_did_not_come_is_overdue_until_today() {
        let (t, _) = tracks(&[on("Metformine", 28, "2026-09-01", 90)], "2026-09-20");
        let t = &t[0];
        let overdue = t
            .segments
            .iter()
            .find(|s| s.kind == SegKind::Overdue)
            .unwrap();
        assert_eq!(
            (overdue.from.as_str(), overdue.to.as_str()),
            ("2026-09-15", "2026-09-20")
        );
        assert_eq!(
            at(t, "2026-09-20"),
            DayState::Overdue {
                since: "2026-09-15".to_owned()
            }
        );
        assert_eq!(
            at(t, "2026-09-16"),
            DayState::Overdue {
                since: "2026-09-15".to_owned()
            }
        );
        let first_planned = t
            .segments
            .iter()
            .find(|s| s.kind == SegKind::Planned)
            .unwrap();
        assert_eq!(first_planned.from, "2026-09-20");
        // Du 20/09 au 29/11 : 71 jours, boîtes de 14 jours : 6.
        assert_eq!(t.deliveries_left, 6);
    }

    /// Un traitement fini n'est pas en retard.
    #[test]
    fn a_finished_treatment_is_not_overdue() {
        let (t, _) = tracks(&[on("Amoxicilline", 28, "2026-09-01", 14)], "2026-10-08");
        assert!(t[0].segments.iter().all(|s| s.kind != SegKind::Overdue));
        assert_eq!(t[0].deliveries_left, 0);
    }

    #[test]
    fn renewals_align_on_the_latest_next_delivery() {
        let (t, _) = tracks(
            &[
                on("Mirtazapine", 28, "2026-10-08", 0),
                on("Metformine", 90, "2026-10-08", 0),
            ],
            "2026-10-08",
        );
        let target = sync_target(&t).unwrap();
        assert_eq!(target, "2026-11-22");
        let b = bridge(&t[0], &target).unwrap();
        // Du 22/10 au 21/11 : 31 jours à 2 par jour.
        assert_eq!((b.units, b.boxes), (62, 3));
        assert!(bridge(&t[1], &target).is_none());
    }

    #[test]
    fn unreadable_lines_are_reported_by_index() {
        let (t, bad) = tracks(
            &[on("A", 0, "2026-10-08", 0), on("B", 28, "2026-10-08", 0)],
            "2026-10-08",
        );
        assert_eq!(t.len(), 1);
        assert_eq!(bad, vec![(0, vec![Missing::Units])]);
    }

    #[test]
    fn the_frame_runs_from_the_first_delivery_to_the_last_day_shown() {
        let (t, _) = tracks(
            &[
                on("Mirtazapine", 28, "2026-10-01", 0),
                on("Metformine", 90, "2026-10-08", 90),
            ],
            "2026-10-08",
        );
        let (origin, end) = frame(&t, "2026-10-08", None);
        assert_eq!(origin, "2026-10-01");
        // Fin de la durée prescrite : 05/01/2027, à 96 jours du 01/10.
        assert_eq!(end, 96);
        assert_eq!(frame(&t, "2026-10-08", Some("2027-03-01")).1, 151);
        assert_eq!(frame(&[], "2026-10-08", None), ("2026-10-08".to_owned(), 0));
    }

    #[test]
    fn the_calendar_marks_months_mondays_and_saturdays() {
        let c = calendar("2026-10-08", 0, 30);
        assert_eq!(
            c.months,
            vec![(0, "2026-10".to_owned()), (24, "2026-11".to_owned())]
        );
        assert_eq!(c.mondays.first(), Some(&4)); // lundi 12 octobre
        assert_eq!(c.saturdays.first(), Some(&2)); // samedi 10 octobre
    }
}
