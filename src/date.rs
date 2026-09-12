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

/// De combien de **semaines** `to` est après `from` : l'écart entre les
/// lundis de leurs semaines, négatif quand `to` précède.
///
/// Ce n'est pas `days_between / 7`, qui répondrait zéro pour un dimanche
/// et le lundi qui le suit — deux semaines différentes. C'est le
/// décalage qu'attend `Db::week_dates`, qui compte depuis la semaine
/// d'aujourd'hui : il permet de demander « la semaine de ce jour-là »
/// à une fonction qui ne sait dire que « dans n semaines ».
pub fn weeks_between(from: &str, to: &str) -> Option<i64> {
    let a = to_days(from)? - (weekday(from)? - 1);
    let b = to_days(to)? - (weekday(to)? - 1);
    // L'écart entre deux lundis est un multiple de sept : la division
    // est exacte, y compris négative.
    Some((b - a) / 7)
}

/// De combien de **mois** `to` est après `from` : l'écart entre les
/// premiers de leurs mois, et non un nombre de jours divisé.
///
/// Le décalage qu'attend `Db::month_grid`, pour la même raison que
/// [`weeks_between`].
pub fn months_between(from: &str, to: &str) -> Option<i64> {
    let (y1, m1, _) = parse_iso(from)?;
    let (y2, m2, _) = parse_iso(to)?;
    Some((y2 * 12 + m2) - (y1 * 12 + m1))
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

/// Le jour de la semaine, **lundi = 1 … dimanche = 7** — la numérotation
/// ISO, qui est celle dont le planning a besoin : une trame se pose sur
/// « le mercredi », et une semaine commence le lundi.
pub fn weekday(iso: &str) -> Option<i64> {
    // Le rang 0 de `to_days` est un jeudi. `+3` ramène ce jeudi sur
    // un lundi, le reste euclidien fait le tour, et `+1` donne le
    // lundi à 1 plutôt qu'à 0.
    Some((to_days(iso)? + 3).rem_euclid(7) + 1)
}

/// Le numéro de semaine ISO 8601, et **l'année à laquelle il
/// appartient**, qui n'est pas toujours celle de la date.
///
/// Les deux vont ensemble et ne se séparent pas : le 1er janvier 2027
/// est un vendredi, il est donc dans la semaine 53 de **2026**, et un
/// numéro rendu sans son année se lirait « semaine 53 de 2027 », qui
/// n'existe pas.
///
/// La règle ISO tient en une phrase : une semaine appartient à l'année
/// de son **jeudi**. C'est ce que le calcul fait, littéralement — on
/// remonte au jeudi de la semaine, on lit son année, on compte les
/// semaines depuis le premier janvier de cette année-là.
///
/// Le planning s'en sert pour les semaines paires et impaires, et c'est
/// la seule chose qui empêche de les confondre avec un cycle de quinze
/// jours : **une année ISO compte 52 ou 53 semaines**, et une année de
/// 53 retourne la parité pour toutes les suivantes.
pub fn iso_week(iso: &str) -> Option<(i64, i64)> {
    let z = to_days(iso)?;
    let thursday = z + (4 - weekday(iso)?);
    let (year, _, _) = parse_iso(&from_days(thursday))?;
    let jan1 = to_days(&format!("{year:04}-01-01"))?;
    Some((year, (thursday - jan1) / 7 + 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Deux dates de la même semaine sont à zéro semaine l'une de
    /// l'autre**, et un dimanche et le lundi suivant sont à une.
    ///
    /// C'est exactement ce qu'un écart en jours divisé par sept ne sait
    /// pas dire : il répond zéro pour ces deux-là, qui ne sont pas dans
    /// la même semaine, et l'agenda ramènerait alors la mauvaise.
    #[test]
    fn a_week_apart_is_counted_from_monday_to_monday() {
        // Lundi 21, dimanche 27 : la même semaine.
        assert_eq!(weeks_between("2026-09-21", "2026-09-27"), Some(0));
        // Dimanche 27 et lundi 28 : un jour, et une semaine.
        assert_eq!(weeks_between("2026-09-27", "2026-09-28"), Some(1));
        assert_eq!(weeks_between("2026-09-28", "2026-09-27"), Some(-1));
        // Et cela compte au-delà de l'année, sans passer par un
        // numéro de semaine : c'est un écart, pas une étiquette.
        assert_eq!(weeks_between("2026-12-28", "2027-01-04"), Some(1));
        assert_eq!(weeks_between("2026-09-07", "2026-09-25"), Some(2));
        assert_eq!(weeks_between("hier", "2026-09-25"), None);
    }

    /// Les mois se comptent de premier à premier, et le 31 d'un mois
    /// suivi du 1er du suivant sont à **un** mois — pas à zéro, ce que
    /// trente-et-un jours divisés donneraient.
    #[test]
    fn a_month_apart_is_counted_from_the_first_to_the_first() {
        assert_eq!(months_between("2026-09-01", "2026-09-30"), Some(0));
        assert_eq!(months_between("2026-09-30", "2026-10-01"), Some(1));
        assert_eq!(months_between("2026-10-01", "2026-09-30"), Some(-1));
        assert_eq!(months_between("2026-12-31", "2027-01-01"), Some(1));
        assert_eq!(months_between("2026-01-15", "2027-01-15"), Some(12));
        assert_eq!(months_between("2026-01-15", ""), None);
    }

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
            assert_eq!(weekday(bad), None);
            assert_eq!(iso_week(bad), None);
        }
    }

    /// Lundi vaut 1 et dimanche 7, et c'est la seule numérotation qui
    /// sorte d'ici : une trame posée « le mercredi » se cherche par ce
    /// nombre-là.
    #[test]
    fn the_week_begins_on_monday() {
        // Le 7 septembre 2026 est un lundi.
        assert_eq!(weekday("2026-09-07"), Some(1));
        assert_eq!(weekday("2026-09-13"), Some(7));
        // Et le tour se boucle sans trou, sur deux mois pris au hasard.
        let mut day = "2026-01-01".to_owned();
        for _ in 0..60 {
            let next = add_days(&day, 1).expect("le lendemain existe");
            let (a, b) = (
                weekday(&day).expect("un jour a un rang"),
                weekday(&next).expect("le lendemain aussi"),
            );
            assert_eq!(b, a % 7 + 1, "{day} → {next}");
            day = next;
        }
    }

    /// **Une semaine appartient à l'année de son jeudi**, et c'est toute
    /// la règle ISO. Les deux bords de l'an sont les seuls endroits où
    /// elle se voit, et ce sont exactement ceux où une parité se
    /// trompe : le 1er janvier 2027 est dans la semaine 53 de 2026.
    #[test]
    fn a_week_belongs_to_the_year_of_its_thursday() {
        // 2026 commence un jeudi : le 1er janvier est donc dans sa
        // propre semaine 1.
        assert_eq!(iso_week("2026-01-01"), Some((2026, 1)));
        // Fin décembre 2026, la semaine 53 court sur janvier 2027.
        assert_eq!(iso_week("2026-12-31"), Some((2026, 53)));
        assert_eq!(iso_week("2027-01-01"), Some((2026, 53)));
        assert_eq!(iso_week("2027-01-03"), Some((2026, 53)));
        assert_eq!(iso_week("2027-01-04"), Some((2027, 1)));
        // Et dans l'autre sens : le 1er janvier 2021 est un vendredi,
        // donc dans la semaine 53 de 2020.
        assert_eq!(iso_week("2021-01-01"), Some((2020, 53)));
        assert_eq!(iso_week("2020-01-01"), Some((2020, 1)));
    }

    /// La propriété qui tient tout le reste : sur deux siècles, le
    /// numéro est entre 1 et 53, il ne bouge pas d'un jour à l'autre à
    /// l'intérieur d'une semaine, et le 4 janvier est toujours en
    /// semaine 1 — la définition même de la norme.
    #[test]
    fn every_week_of_two_centuries_is_numbered_once() {
        for y in 1900..=2100 {
            assert_eq!(
                iso_week(&format!("{y}-01-04")).map(|(_, w)| w),
                Some(1),
                "le 4 janvier {y} est en semaine 1, par définition"
            );
        }
        let first = to_days("1900-01-01").expect("le premier jour");
        let last = to_days("2100-12-31").expect("le dernier");
        let mut weeks = std::collections::HashSet::new();
        for z in first..=last {
            let iso = from_days(z);
            let (year, week) = iso_week(&iso).expect("chaque jour a sa semaine");
            assert!((1..=53).contains(&week), "{iso} : semaine {week}");
            // Le lundi de la semaine porte le même numéro que son
            // dimanche : sans quoi une parité changerait en cours de
            // semaine.
            let wd = weekday(&iso).expect("un rang");
            let monday = from_days(z - (wd - 1));
            assert_eq!(iso_week(&monday), Some((year, week)), "{iso}");
            weeks.insert((year, week));
        }
        // Une semaine ISO fait sept jours, et rien n'en invente une.
        assert_eq!(
            weeks.len(),
            usize::try_from((last - first) / 7 + 1).unwrap()
        );
    }

    /// **La parité ISO n'est pas un cycle de quinze jours**, et une
    /// année de 53 semaines est la raison. C'est la règle sur laquelle
    /// repose « les semaines paires » du planning : deux semaines
    /// impaires se suivent au passage de 2026 à 2027, et une trame
    /// posée tous les quatorze jours se retrouverait à contretemps pour
    /// toujours.
    #[test]
    fn a_year_of_fifty_three_weeks_flips_the_parity() {
        let (_, before) = iso_week("2026-12-28").expect("un lundi de décembre");
        let (_, after) = iso_week("2027-01-04").expect("le lundi suivant l'an");
        assert_eq!((before, after), (53, 1));
        assert_eq!(
            before % 2,
            after % 2,
            "deux semaines impaires de suite : quatorze jours s'y trompent"
        );
    }
}
