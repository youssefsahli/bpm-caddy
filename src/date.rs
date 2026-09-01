//! Le calendrier de l'application, écrit une fois.
//!
//! Il était écrit trois fois. `ordonnancier::day_number` comptait les
//! jours par la formule julienne, `location::julian` par la formule
//! civile — deux arithmétiques grégoriennes pour la même soustraction —,
//! et `surveillance::months_between` relisait une date ISO à sa façon
//! pour une troisième. Aucune n'était fausse ; c'est justement la
//! situation que la maison refuse ailleurs, parce que **deux mesures
//! d'une même chose finissent toujours par diverger**, et que celle qui
//! divergera est celle que personne ne relit.
//!
//! Le module garde la paire civile plutôt que la julienne : elle a un
//! inverse. `from_days` est ce que la julienne ne savait pas faire, et
//! c'est ce dont on a besoin pour décaler une date — et pour lire la
//! péremption d'une boîte, où le jour `00` veut dire « fin du mois ».
//!
//! Les deux formules ne donnent pas le même nombre absolu, et cela n'a
//! aucune importance : seules les **différences** sortent d'ici, et
//! elles sont identiques.
//!
//! Statique, pur, testé. Aucune horloge : le jour est donné.

/// Les trois nombres d'une date ISO `AAAA-MM-JJ`, si c'en est une.
///
/// Le jour est accepté de 1 à 31 et le mois de 1 à 12 : c'est un
/// contrôle de forme et non de calendrier, et le 31 février se rattrape
/// à la conversion, où il ne tombe simplement pas sur lui-même.
pub fn parse_iso(iso: &str) -> Option<(i64, i64, i64)> {
    let mut parts = iso.trim().split('-');
    let y: i64 = parts.next()?.parse().ok()?;
    let m: i64 = parts.next()?.parse().ok()?;
    let d: i64 = parts.next()?.parse().ok()?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    Some((y, m, d))
}

/// Le rang d'une date ISO en jours, depuis une origine arbitraire.
///
/// L'algorithme civil usuel : mars devient le premier mois, ce qui met
/// le jour manquant de février à la fin de l'année et fait disparaître
/// le cas particulier du bissextile — règle du siècle comprise.
pub fn to_days(iso: &str) -> Option<i64> {
    let (y, m, d) = parse_iso(iso)?;
    Some(days_from_civil(y, m, d))
}

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Le chemin inverse : un rang en jours, rendu en ISO `AAAA-MM-JJ`.
pub fn from_days(z: i64) -> String {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

/// Le nombre de jours de `from` à `to`, négatif quand `to` précède
/// `from`. `None` dès que l'une des deux ne se lit pas.
pub fn days_between(from: &str, to: &str) -> Option<i64> {
    Some(to_days(to)? - to_days(from)?)
}

/// `from` décalé de `days` jours, rendu en ISO.
pub fn add_days(from: &str, days: i64) -> Option<String> {
    Some(from_days(to_days(from)? + days))
}

/// Le dernier jour de ce mois-là — 28, 29, 30 ou 31.
///
/// La règle du siècle est dans l'arithmétique et non dans une condition
/// écrite à la main : le premier du mois suivant, moins un jour. 1900
/// n'est pas bissextile, 2000 l'est, et rien ici n'a eu à le savoir.
///
/// C'est ce dont la lecture d'une péremption a besoin : sur une boîte,
/// le jour `00` d'un AI 17 veut dire « fin du mois ».
pub fn end_of_month(y: i64, m: i64) -> Option<i64> {
    if !(1..=12).contains(&m) {
        return None;
    }
    let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
    parse_iso(&from_days(days_from_civil(ny, nm, 1) - 1)).map(|(_, _, d)| d)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// L'aller et le retour se répondent, sur deux siècles.
    ///
    /// C'est le test qui autorise à n'avoir qu'un calendrier : si
    /// `from_days` défait exactement `to_days` sur chaque jour de 1900 à
    /// 2100, alors tout ce qui s'appuie dessus — l'écart entre deux
    /// dates, le décalage d'une échéance, la fin d'un mois — s'appuie
    /// sur la même chose.
    #[test]
    fn every_day_of_two_centuries_survives_the_round_trip() {
        let first = to_days("1900-01-01").expect("le premier jour doit se lire");
        let last = to_days("2100-12-31").expect("le dernier jour doit se lire");
        assert!(last > first);
        let mut seen = 0;
        for z in first..=last {
            let iso = from_days(z);
            assert_eq!(
                to_days(&iso),
                Some(z),
                "{iso} ne revient pas sur son propre rang"
            );
            seen += 1;
        }
        // 201 ans, dont 49 bissextiles (1900 n'en est pas, 2000 en est).
        assert_eq!(seen, 201 * 365 + 49, "le compte des jours de deux siècles");
    }

    /// La règle du siècle, là où elle décide : le lendemain du 28
    /// février.
    ///
    /// Le compte du test précédent l'exige déjà globalement — 49
    /// bissextiles sur deux siècles n'est juste que si 1900 n'en est pas
    /// et 2000 en est —, mais un compte qui tombe faux ne dit pas
    /// *laquelle* des deux années a bougé. Ces quatre lignes le disent.
    #[test]
    fn the_century_rule_decides_the_length_of_february() {
        assert_eq!(add_days("2024-02-28", 1).as_deref(), Some("2024-02-29"));
        assert_eq!(add_days("2023-02-28", 1).as_deref(), Some("2023-03-01"));
        assert_eq!(
            add_days("2000-02-28", 1).as_deref(),
            Some("2000-02-29"),
            "2000 est bissextile : divisible par 400"
        );
        assert_eq!(
            add_days("1900-02-28", 1).as_deref(),
            Some("1900-03-01"),
            "1900 ne l'est pas : divisible par 100 et non par 400"
        );
    }

    /// Ce qu'on lit, et ce qu'on refuse de lire.
    #[test]
    fn a_day_that_is_not_a_day_is_read_as_nothing() {
        assert_eq!(days_between("2026-01-01", "2026-12-31"), Some(364));
        assert_eq!(days_between("2026-12-31", "2026-01-01"), Some(-364));
        assert_eq!(
            days_between("2024-02-28", "2024-03-01"),
            Some(2),
            "bissextile"
        );
        assert_eq!(days_between("2023-02-28", "2023-03-01"), Some(1));
        assert_eq!(add_days("2026-02-28", 1).as_deref(), Some("2026-03-01"));
        assert_eq!(add_days("2024-02-28", 1).as_deref(), Some("2024-02-29"));
        assert_eq!(add_days("2026-01-01", -1).as_deref(), Some("2025-12-31"));
        for bad in ["", "hier", "2026-13-01", "2026-08-32", "2026-08", "x-y-z"] {
            assert_eq!(to_days(bad), None, "« {bad} » ne doit pas se lire");
            assert_eq!(days_between(bad, "2026-01-01"), None);
            assert_eq!(days_between("2026-01-01", bad), None);
        }
    }
}
