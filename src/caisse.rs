//! Le comptage de la caisse : ce qu'il y a dans le tiroir le soir, ce
//! qu'on y laisse pour demain, et l'écart avec ce que la journée devait
//! rentrer.
//!
//! Trois règles tiennent ce module, et ce sont elles qui justifient
//! qu'il existe plutôt qu'une addition dans la vue :
//!
//! * **L'argent se compte en centimes, jamais en flottants.** Douze
//!   pièces de dix centimes font 1,20 € et non 1,1999999999999997 ;
//!   additionner deux cents lignes de caisse en `f64` fabrique un écart
//!   d'un centime que personne ne retrouvera, et un écart d'un centime
//!   est exactement ce qu'un comptage de caisse existe pour voir. Tout
//!   est `i64` de centimes, et la conversion se fait à l'affichage.
//! * **Un écart n'est pas une correction.** Le module dit l'écart ; il
//!   ne le résorbe pas, il ne propose pas d'ajuster le comptage, et le
//!   comptage n'est jamais recalculé depuis l'attendu. C'est la même
//!   discipline que le registre des stupéfiants : ce qu'on a trouvé
//!   reste écrit, et l'explication vient à côté.
//! * **Sans recette attendue, il n'y a pas d'écart** — et surtout pas
//!   un écart égal à tout ce qu'il y a dans le tiroir. Une caisse
//!   comptée sans que personne n'ait saisi ce que la journée devait
//!   faire est un comptage valide et un écart inconnu : `gap` vaut
//!   `None`, la feuille laisse la ligne vide. Le contraire annoncerait
//!   « + 1 240,50 € d'excédent » tous les soirs.
//!
//! Pur, testé, sans horloge et sans base : le jour et l'attendu sont
//! passés.

/// Une coupure de l'euro : sa valeur en centimes et comment on la nomme
/// sur la feuille.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Denomination {
    /// Valeur en centimes — la clé, et ce sur quoi tout est calculé.
    pub cents: i64,
    /// « 50 € », « 20 c » : ce qui est écrit en tête de colonne.
    pub label: &'static str,
    /// Billet ou pièce. Les deux se comptent séparément au comptoir :
    /// on compte les billets à la main et les pièces au plateau.
    pub note: bool,
}

/// Les quinze coupures de l'euro, de la plus grosse à la plus petite.
///
/// Le billet de 500 € n'est plus émis depuis 2019 mais reste ayant
/// cours légal : il est dans la liste parce qu'une caisse qui en reçoit
/// un doit pouvoir l'écrire, et une ligne à zéro ne coûte rien.
pub const DENOMINATIONS: [Denomination; 15] = [
    Denomination {
        cents: 50_000,
        label: "500 €",
        note: true,
    },
    Denomination {
        cents: 20_000,
        label: "200 €",
        note: true,
    },
    Denomination {
        cents: 10_000,
        label: "100 €",
        note: true,
    },
    Denomination {
        cents: 5_000,
        label: "50 €",
        note: true,
    },
    Denomination {
        cents: 2_000,
        label: "20 €",
        note: true,
    },
    Denomination {
        cents: 1_000,
        label: "10 €",
        note: true,
    },
    Denomination {
        cents: 500,
        label: "5 €",
        note: true,
    },
    Denomination {
        cents: 200,
        label: "2 €",
        note: false,
    },
    Denomination {
        cents: 100,
        label: "1 €",
        note: false,
    },
    Denomination {
        cents: 50,
        label: "50 c",
        note: false,
    },
    Denomination {
        cents: 20,
        label: "20 c",
        note: false,
    },
    Denomination {
        cents: 10,
        label: "10 c",
        note: false,
    },
    Denomination {
        cents: 5,
        label: "5 c",
        note: false,
    },
    Denomination {
        cents: 2,
        label: "2 c",
        note: false,
    },
    Denomination {
        cents: 1,
        label: "1 c",
        note: false,
    },
];

/// Ce qu'on a compté, coupure par coupure, dans l'ordre de
/// [`DENOMINATIONS`].
pub type Quantities = [i64; DENOMINATIONS.len()];

/// Un moyen de paiement autre que les espèces, saisi en une ligne.
///
/// La carte et les chèques ne se comptent pas : ils se lisent sur le
/// ticket Z du terminal et sur le bordereau de remise. Ils entrent donc
/// comme un montant et non comme un nombre de coupures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Other {
    pub label: String,
    pub cents: i64,
}

/// Ce que le comptage dit, une fois fait.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tally {
    /// Les espèces trouvées dans le tiroir.
    pub cash: i64,
    /// Ce qu'on laisse pour ouvrir demain.
    pub float_kept: i64,
    /// Ce qui sort du tiroir : les espèces moins le fond. Négatif
    /// lorsque le fond demandé dépasse ce qu'il y a — c'est-à-dire
    /// qu'il faudra en remettre, et le dire ainsi vaut mieux que de
    /// s'arrêter à zéro.
    pub banked: i64,
    /// Carte, chèques, tout ce qui n'est pas dans le tiroir.
    pub other: i64,
    /// Espèces plus le reste : la recette encaissée du jour.
    pub takings: i64,
    /// Ce que la journée devait faire, quand quelqu'un l'a saisi.
    pub expected: Option<i64>,
    /// `takings - expected`, positif pour un excédent. `None` tant que
    /// l'attendu n'est pas connu : voir l'en-tête du module.
    pub gap: Option<i64>,
}

/// Le total des espèces comptées.
///
/// Une quantité négative n'est pas un retrait : c'est une faute de
/// frappe. Elle compte pour zéro plutôt que de faire baisser un total
/// que personne ne relira.
#[must_use]
pub fn cash_total(quantities: &Quantities) -> i64 {
    DENOMINATIONS
        .iter()
        .zip(quantities.iter())
        .map(|(d, q)| d.cents * (*q).max(0))
        .sum()
}

/// Le total des billets seuls — ce qu'on met sous enveloppe.
#[must_use]
pub fn notes_total(quantities: &Quantities) -> i64 {
    DENOMINATIONS
        .iter()
        .zip(quantities.iter())
        .filter(|(d, _)| d.note)
        .map(|(d, q)| d.cents * (*q).max(0))
        .sum()
}

/// Le total des pièces seules.
#[must_use]
pub fn coins_total(quantities: &Quantities) -> i64 {
    cash_total(quantities) - notes_total(quantities)
}

/// Combien de coupures ont été comptées, toutes valeurs confondues.
/// Zéro veut dire « personne n'a rien saisi », ce qui n'est pas la même
/// chose qu'un tiroir vide — et la feuille le dit.
#[must_use]
pub fn pieces(quantities: &Quantities) -> i64 {
    quantities.iter().map(|q| (*q).max(0)).sum()
}

/// Le comptage complet.
#[must_use]
pub fn tally(
    quantities: &Quantities,
    float_kept: i64,
    others: &[Other],
    expected: Option<i64>,
) -> Tally {
    let cash = cash_total(quantities);
    let float_kept = float_kept.max(0);
    let other: i64 = others.iter().map(|o| o.cents).sum();
    let takings = cash + other;
    Tally {
        cash,
        float_kept,
        banked: cash - float_kept,
        other,
        takings,
        expected,
        gap: expected.map(|e| takings - e),
    }
}

/// « 1 240,50 » — le montant sans son unité, en français : espace fine
/// pour les milliers, virgule décimale. Le signe est celui du nombre,
/// et un écart positif est écrit avec son `+` par l'appelant : ici, un
/// zéro est un zéro et non un « +0,00 ».
#[must_use]
pub fn euros(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let cents = cents.unsigned_abs();
    let whole = cents / 100;
    let frac = cents % 100;
    let mut digits = whole.to_string();
    // Les milliers, groupés par la fin. `insert` sur une chaîne d'ASCII
    // seulement : les chiffres le sont.
    let mut i = digits.len();
    while i > 3 {
        i -= 3;
        digits.insert(i, '\u{202f}');
    }
    format!("{sign}{digits},{frac:02}")
}

/// L'inverse : lire « 1 240,50 », « 1240.5 », « 1 240 » ou « 12,5 € ».
///
/// Ce que le comptoir tape n'est pas ce qu'un analyseur strict attend :
/// le point et la virgule sont le même séparateur, l'euro peut traîner
/// derrière, et l'espace des milliers peut être une espace ordinaire,
/// une espace fine ou rien. Une saisie vide n'est pas zéro, c'est
/// `None` — sans quoi un champ qu'on n'a pas rempli deviendrait un
/// montant nul saisi exprès.
#[must_use]
pub fn parse_euros(text: &str) -> Option<i64> {
    let cleaned: String = text
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '\u{202f}' && *c != '\u{a0}' && *c != '€')
        .collect();
    if cleaned.is_empty() {
        return None;
    }
    let (sign, body) = match cleaned.strip_prefix('-') {
        Some(rest) => (-1_i64, rest),
        None => (1_i64, cleaned.strip_prefix('+').unwrap_or(&cleaned)),
    };
    let (whole, frac) = match body.split_once([',', '.']) {
        Some((w, f)) => (w, f),
        None => (body, ""),
    };
    if !whole.chars().all(|c| c.is_ascii_digit()) || !frac.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    if whole.is_empty() && frac.is_empty() {
        return None;
    }
    let whole: i64 = if whole.is_empty() {
        0
    } else {
        whole.parse().ok()?
    };
    // Deux décimales, et pas davantage : « 12,567 € » n'existe pas dans
    // un tiroir-caisse. On tronque plutôt que d'arrondir — arrondir un
    // écart le fait disparaître.
    let frac: i64 = match frac.len() {
        0 => 0,
        1 => frac.parse::<i64>().ok()? * 10,
        _ => frac.get(..2)?.parse().ok()?,
    };
    Some(sign * (whole.checked_mul(100)?.checked_add(frac)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn none() -> Quantities {
        [0; DENOMINATIONS.len()]
    }

    /// La raison d'être du module : douze pièces de dix centimes font
    /// 1,20 € exactement. En `f64`, `0.1 * 12.0` ne vaut pas `1.2`, et
    /// un comptage de caisse existe précisément pour voir un centime.
    #[test]
    fn money_is_counted_in_whole_centimes() {
        let mut q = none();
        q[11] = 12; // 10 c
        assert_eq!(cash_total(&q), 120);
        // Et la somme de toutes les petites coupures, une de chaque,
        // tombe juste : 0,50 + 0,20 + 0,10 + 0,05 + 0,02 + 0,01.
        let mut q = none();
        for slot in &mut q[9..15] {
            *slot = 1;
        }
        assert_eq!(cash_total(&q), 88);
        assert_eq!(euros(88), "0,88");
    }

    #[test]
    fn a_full_drawer_adds_up() {
        let mut q = none();
        q[3] = 4; // 4 × 50 €  = 200,00
        q[4] = 7; // 7 × 20 €  = 140,00
        q[6] = 3; // 3 × 5 €   =  15,00
        q[7] = 9; // 9 × 2 €   =  18,00
        q[9] = 5; // 5 × 50 c  =   2,50
        assert_eq!(cash_total(&q), 37_550);
        assert_eq!(notes_total(&q), 35_500);
        assert_eq!(coins_total(&q), 2_050);
        assert_eq!(notes_total(&q) + coins_total(&q), cash_total(&q));
        assert_eq!(pieces(&q), 28);
        assert_eq!(euros(cash_total(&q)), "375,50");
    }

    /// Une quantité négative est une faute de frappe, pas un retrait :
    /// elle ne fait pas baisser le total.
    #[test]
    fn a_negative_quantity_does_not_lower_the_total() {
        let mut q = none();
        q[3] = 2; // 100,00
        q[4] = -5; // une faute de frappe
        assert_eq!(cash_total(&q), 10_000);
        assert_eq!(pieces(&q), 2);
    }

    /// La règle du module : sans attendu, pas d'écart — surtout pas un
    /// écart égal à tout le tiroir.
    #[test]
    fn without_an_expected_figure_there_is_no_gap() {
        let mut q = none();
        q[3] = 4;
        let t = tally(&q, 15_000, &[], None);
        assert_eq!(t.cash, 20_000);
        assert_eq!(t.takings, 20_000);
        assert_eq!(t.gap, None);
        assert_eq!(t.expected, None);
    }

    #[test]
    fn the_gap_is_signed_and_never_resorbed() {
        let mut q = none();
        q[3] = 4; // 200,00 en espèces
        let others = [Other {
            label: "Carte".to_owned(),
            cents: 45_075,
        }];
        // Attendu 660,00 : il manque 9,25.
        let t = tally(&q, 15_000, &others, Some(66_000));
        assert_eq!(t.other, 45_075);
        assert_eq!(t.takings, 65_075);
        assert_eq!(t.gap, Some(-925));
        // Le comptage n'a pas bougé d'un centime pour se rapprocher de
        // l'attendu : c'est toute la discipline.
        assert_eq!(t.cash, 20_000);
        // Et dans l'autre sens, un excédent.
        let t = tally(&q, 15_000, &others, Some(64_000));
        assert_eq!(t.gap, Some(1_075));
    }

    /// Le fond de caisse sort du tiroir ; il n'est pas de la recette.
    #[test]
    fn the_float_leaves_the_takings_alone() {
        let mut q = none();
        q[3] = 4; // 200,00
        let t = tally(&q, 15_000, &[], Some(20_000));
        assert_eq!(t.banked, 5_000);
        assert_eq!(
            t.takings, 20_000,
            "le fond ne se retranche pas de la recette"
        );
        assert_eq!(t.gap, Some(0));
        // Un fond plus grand que le tiroir se dit en négatif : il faudra
        // en remettre demain matin.
        let t = tally(&q, 25_000, &[], None);
        assert_eq!(t.banked, -5_000);
        // Un fond négatif saisi par erreur vaut zéro.
        let t = tally(&q, -100, &[], None);
        assert_eq!(t.float_kept, 0);
        assert_eq!(t.banked, 20_000);
    }

    #[test]
    fn amounts_are_written_and_read_back_the_french_way() {
        assert_eq!(euros(0), "0,00");
        assert_eq!(euros(5), "0,05");
        assert_eq!(euros(124_050), "1\u{202f}240,50");
        assert_eq!(euros(-925), "-9,25");
        assert_eq!(euros(123_456_789), "1\u{202f}234\u{202f}567,89");
        // Ce que le comptoir tape, sous toutes ses formes.
        assert_eq!(parse_euros("1 240,50"), Some(124_050));
        assert_eq!(parse_euros("1240.5"), Some(124_050));
        assert_eq!(parse_euros("12,5 €"), Some(1_250));
        assert_eq!(parse_euros("40"), Some(4_000));
        assert_eq!(parse_euros(",05"), Some(5));
        assert_eq!(parse_euros("-9,25"), Some(-925));
        assert_eq!(parse_euros(&euros(124_050)), Some(124_050));
        // Trois décimales : on tronque, on n'arrondit pas — arrondir un
        // écart le fait disparaître.
        assert_eq!(parse_euros("12,567"), Some(1_256));
        // Et ce qui n'est pas un montant n'en devient pas un.
        assert_eq!(parse_euros(""), None);
        assert_eq!(parse_euros("   "), None);
        assert_eq!(parse_euros("douze"), None);
        assert_eq!(parse_euros("12,x"), None);
        assert_eq!(parse_euros("€"), None);
    }

    /// Les coupures : toutes les valeurs de l'euro, une seule fois
    /// chacune, de la plus grosse à la plus petite, et le compte des
    /// billets est celui de la Banque centrale.
    #[test]
    fn the_denominations_are_the_euro_and_nothing_else() {
        let values: Vec<i64> = DENOMINATIONS.iter().map(|d| d.cents).collect();
        let mut sorted = values.clone();
        sorted.sort_unstable_by(|a, b| b.cmp(a));
        assert_eq!(values, sorted, "de la plus grosse à la plus petite");
        let mut unique = values.clone();
        unique.dedup();
        assert_eq!(
            unique.len(),
            values.len(),
            "une valeur n'apparaît qu'une fois"
        );
        assert_eq!(DENOMINATIONS.iter().filter(|d| d.note).count(), 7);
        assert_eq!(DENOMINATIONS.iter().filter(|d| !d.note).count(), 8);
        // Un billet vaut au moins 5 €, une pièce au plus 2 € : c'est ce
        // qui permet de les compter séparément sans se demander où
        // passe la frontière.
        assert!(DENOMINATIONS.iter().all(|d| d.note == (d.cents >= 500)));
        // Et une de chaque fait 888,88 €, ce qui vérifie la table
        // entière en un nombre.
        let all = [1_i64; DENOMINATIONS.len()];
        assert_eq!(euros(cash_total(&all)), "888,88");
    }
}
