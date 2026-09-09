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

/// « 1 240,50 » — le montant sans son unité, en français : espace
/// insécable pour les milliers, virgule décimale. Le signe est celui du
/// nombre, et un écart positif est écrit avec son `+` par l'appelant :
/// ici, un zéro est un zéro et non un « +0,00 ».
///
/// **L'espace des milliers est U+00A0 et non l'espace fine U+202F**,
/// que la typographie française préférerait : la fonte que
/// l'application dessine avec n'a pas de glyphe pour la seconde, et
/// tout montant à quatre chiffres sortait « 1□240,50 » au comptoir.
/// C'est la même règle que les flèches qui restent dans les pastilles —
/// l'application ne livre aucune police, elle écrit avec ce que la
/// fonte sait dessiner. `every_symbol_the_code_draws_has_a_glyph…`
/// (`strings.rs`) passe la sortie de cette fonction dans la fonte qui
/// la peindra et refuse le prochain caractère sans glyphe.
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
        digits.insert(i, '\u{a0}');
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

/// Un comptage déjà rangé, tel que l'historique le relit.
///
/// Le module ne connaît pas la base : la vue lit ses lignes et les
/// donne sous cette forme, comme le plan de journée donne des
/// intervalles à qui calcule des voies. Tout est en centimes, ici comme
/// partout ailleurs dans ce fichier.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Counted {
    /// La ligne, telle que la base l'a numérotée. C'est elle qui dit
    /// quel comptage est le dernier d'un soir : deux comptages peuvent
    /// partager la seconde, jamais l'identifiant.
    pub id: i64,
    /// ISO `AAAA-MM-JJ` — le jour compté.
    pub day: String,
    pub cash: i64,
    /// Carte, chèques : la somme des lignes hors tiroir.
    pub other: i64,
    pub float_kept: i64,
    /// Ce que la journée devait faire, quand quelqu'un l'a saisi.
    pub expected: Option<i64>,
}

impl Counted {
    /// La recette encaissée : le tiroir et le reste.
    #[must_use]
    pub fn takings(&self) -> i64 {
        self.cash + self.other
    }

    /// L'écart de ce soir-là, ou rien — la règle du module, appliquée
    /// une ligne à la fois.
    #[must_use]
    pub fn gap(&self) -> Option<i64> {
        self.expected.map(|e| self.takings() - e)
    }
}

/// Le comptage qui fait foi pour chaque jour, du plus ancien au plus
/// récent.
///
/// **Un jour ne compte qu'une fois.** La table est en insertion seule :
/// une caisse recomptée le même soir est une deuxième ligne, et les
/// deux se lisent — mais les additionner ferait une journée à double
/// recette. C'est la dernière écrite qui fait foi, la première reste
/// affichée et marquée. On ne réécrit pas ce qui a été compté ; on lit
/// ce qui a été compté en dernier.
#[must_use]
pub fn per_day(counts: &[Counted]) -> Vec<&Counted> {
    let mut kept: Vec<&Counted> = Vec::new();
    for c in counts {
        match kept.iter().position(|k| k.day == c.day) {
            Some(at) if kept[at].id < c.id => kept[at] = c,
            Some(_) => {}
            None => kept.push(c),
        }
    }
    kept.sort_by(|a, b| a.day.cmp(&b.day).then(a.id.cmp(&b.id)));
    kept
}

/// Les comptages qu'un plus récent a remplacés, par identifiant : la
/// vue les garde à l'écran, en retrait, plutôt que de les cacher.
#[must_use]
pub fn superseded(counts: &[Counted]) -> Vec<i64> {
    let kept: Vec<i64> = per_day(counts).iter().map(|c| c.id).collect();
    let mut out: Vec<i64> = counts
        .iter()
        .map(|c| c.id)
        .filter(|id| !kept.contains(id))
        .collect();
    out.sort_unstable();
    out
}

/// Ce qu'une période de comptages dit d'elle-même.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Summary {
    /// Les lignes lues, recomptages compris.
    pub counts: usize,
    /// Les jours qu'elles couvrent — jamais les jours du calendrier :
    /// un soir où personne n'a compté n'est pas un soir à zéro euro.
    pub days: usize,
    pub cash: i64,
    pub other: i64,
    pub takings: i64,
    pub float_kept: i64,
    /// Sur combien de jours un attendu avait été saisi. Le total des
    /// écarts porte sur ceux-là et sur aucun autre : sans ce nombre à
    /// côté, une somme d'écarts se lit comme si elle portait sur la
    /// période entière.
    pub with_expected: usize,
    /// La somme des écarts des jours qui en ont un, `None` quand aucun
    /// jour n'a d'attendu — et non zéro, qui se lirait « tout tombe
    /// juste ».
    pub gap: Option<i64>,
    /// Les jours en moins, en plus, et ceux qui tombent juste.
    pub short: usize,
    pub over: usize,
    pub exact: usize,
    /// Le soir le plus loin de zéro, dans un sens ou dans l'autre.
    pub worst: Option<(String, i64)>,
}

/// La période lue d'un coup : les totaux, les écarts, et sur combien de
/// jours ils portent.
#[must_use]
pub fn summarize(counts: &[Counted]) -> Summary {
    let days = per_day(counts);
    let mut s = Summary {
        counts: counts.len(),
        days: days.len(),
        ..Summary::default()
    };
    let mut gap_total = 0_i64;
    for c in &days {
        s.cash += c.cash;
        s.other += c.other;
        s.takings += c.takings();
        s.float_kept += c.float_kept;
        let Some(gap) = c.gap() else { continue };
        s.with_expected += 1;
        gap_total += gap;
        match gap.cmp(&0) {
            std::cmp::Ordering::Less => s.short += 1,
            std::cmp::Ordering::Greater => s.over += 1,
            std::cmp::Ordering::Equal => s.exact += 1,
        }
        if s.worst.as_ref().is_none_or(|(_, w)| gap.abs() > w.abs()) {
            s.worst = Some((c.day.clone(), gap));
        }
    }
    if s.with_expected > 0 {
        s.gap = Some(gap_total);
    }
    s
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
        assert_eq!(euros(124_050), "1\u{a0}240,50");
        assert_eq!(euros(-925), "-9,25");
        assert_eq!(euros(123_456_789), "1\u{a0}234\u{a0}567,89");
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

    fn counted(id: i64, day: &str, cash: i64, expected: Option<i64>) -> Counted {
        Counted {
            id,
            day: day.to_owned(),
            cash,
            other: 0,
            float_kept: 15_000,
            expected,
        }
    }

    /// La règle de l'historique : la table est en insertion seule, donc
    /// un soir recompté est **deux lignes**, et les deux se lisent — mais
    /// le jour ne compte qu'une fois, sur la dernière écrite.
    ///
    /// Additionner les deux ferait une journée à double recette, et
    /// c'est le genre de faux qu'un total mensuel ne montre jamais.
    #[test]
    fn a_day_recounted_is_counted_once() {
        let counts = [
            counted(2, "2026-09-08", 21_000, Some(21_000)),
            counted(1, "2026-09-08", 20_000, Some(21_000)),
            counted(3, "2026-09-09", 30_000, None),
        ];
        let days = per_day(&counts);
        assert_eq!(days.len(), 2);
        // Du plus ancien au plus récent, quel que soit l'ordre reçu.
        assert_eq!(days[0].day, "2026-09-08");
        assert_eq!(days[0].id, 2, "c'est le dernier comptage qui fait foi");
        assert_eq!(days[1].id, 3);
        // Le premier reste lisible, nommé pour que la vue le marque.
        assert_eq!(superseded(&counts), vec![1]);

        let s = summarize(&counts);
        assert_eq!(s.counts, 3, "trois lignes ont bien été lues");
        assert_eq!(s.days, 2);
        assert_eq!(
            s.takings, 51_000,
            "et non 71 000 : le 8 ne compte qu'une fois"
        );
        assert_eq!(s.gap, Some(0));
        assert_eq!(s.with_expected, 1);
    }

    /// La règle du module, à l'échelle d'une période : sans attendu, pas
    /// d'écart — et surtout pas un écart de zéro, qui se lirait « tout
    /// est tombé juste ».
    ///
    /// Et quand une partie seulement des soirs porte un attendu, le
    /// total des écarts dit sur combien il porte : une somme d'écarts
    /// sans ce nombre à côté se lit comme si elle couvrait le mois.
    #[test]
    fn a_period_says_over_how_many_evenings_its_gap_is_computed() {
        let none = [
            counted(1, "2026-09-07", 20_000, None),
            counted(2, "2026-09-08", 30_000, None),
        ];
        let s = summarize(&none);
        assert_eq!(s.gap, None);
        assert_eq!(s.with_expected, 0);
        assert_eq!(s.takings, 50_000, "les recettes se comptent quand même");

        let mixed = [
            counted(1, "2026-09-07", 20_000, Some(20_500)),
            counted(2, "2026-09-08", 30_000, None),
            counted(3, "2026-09-09", 40_000, Some(39_600)),
        ];
        let s = summarize(&mixed);
        assert_eq!(s.gap, Some(-100));
        assert_eq!(s.with_expected, 2, "sur trois soirs comptés");
        assert_eq!(s.days, 3);
        assert_eq!(s.short, 1);
        assert_eq!(s.over, 1);
        assert_eq!(s.exact, 0);
        // Le pire soir est le plus loin de zéro, dans un sens ou dans
        // l'autre — ici le manque de 5,00 €, plus gros que l'excédent
        // de 4,00 € du 9, alors que le total des deux ne fait qu'un
        // euro : une somme d'écarts cache toujours les soirs.
        assert_eq!(s.worst, Some(("2026-09-07".to_owned(), -500)));
    }

    /// Une période sans comptage n'est pas une période à zéro euro.
    ///
    /// C'est la même retenue qu'ailleurs : un jour où personne n'a
    /// compté ne vaut rien, il ne vaut pas zéro, et une vue qui affiche
    /// « 0,00 € » sur un mois non saisi annonce une caisse vide.
    #[test]
    fn an_empty_period_is_not_a_period_at_zero() {
        let s = summarize(&[]);
        assert_eq!(s.days, 0);
        assert_eq!(s.counts, 0);
        assert_eq!(s.gap, None);
        assert_eq!(s.worst, None);
        assert!(per_day(&[]).is_empty());
        assert!(superseded(&[]).is_empty());
    }

    /// Les recettes d'une période se somment en centimes entiers, comme
    /// tout le reste : trente et un soirs à 12,10 € font 375,10 €.
    #[test]
    fn a_period_adds_up_in_whole_centimes() {
        let counts: Vec<Counted> = (1..=31)
            .map(|i| counted(i, &format!("2026-08-{i:02}"), 1_210, Some(1_200)))
            .collect();
        let s = summarize(&counts);
        assert_eq!(s.days, 31);
        assert_eq!(s.takings, 37_510);
        assert_eq!(euros(s.takings), "375,10");
        assert_eq!(s.gap, Some(310));
        assert_eq!(s.over, 31);
    }
}
