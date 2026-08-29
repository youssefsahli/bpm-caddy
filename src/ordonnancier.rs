//! Le registre des stupéfiants : ce qui est entré, ce qui est sorti, et
//! ce qui devrait rester.
//!
//! L'article R. 5132-36 du code de la santé publique demande que toute
//! entrée et toute sortie de stupéfiant soient inscrites, que la balance
//! soit tenue, et qu'un inventaire par pesée ou par comptage soit porté
//! au registre au moins une fois par an. Le registre est **inaltérable**
//! : une ligne écrite ne se corrige pas, elle se contre-passe. C'est la
//! contrainte qui décide de tout le reste — il n'y a ici ni modification
//! ni suppression, et la base non plus n'en propose pas.
//!
//! Ce module ne contient que l'arithmétique et les règles :
//!
//! * la **balance**, qui n'est pas une somme — un inventaire *fixe* le
//!   solde au lieu de s'y ajouter, sans quoi l'écart constaté serait
//!   compté deux fois ;
//! * le **numéro d'ordonnancier**, séquentiel dans l'année et jamais
//!   réattribué ;
//! * l'**écart** d'inventaire, et ce qu'il vaut ;
//! * la **liste de contrôle** : ce qu'il faut aller compter, parce que
//!   le stock est bas ou parce que personne ne l'a compté depuis
//!   longtemps.
//!
//! Pur et testé, comme `revue` et `conciliation`. Aucune base ici : la
//! base est passée en argument. Et aucune horloge : le jour est donné,
//! parce qu'un registre qui se lit différemment selon l'heure à laquelle
//! on l'ouvre n'est pas un registre.

/// Ce qu'une ligne du registre fait au stock.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    /// Réception d'une commande : le stock monte.
    Entree,
    /// Délivrance sur ordonnance : le stock descend, et la ligne porte
    /// un numéro d'ordonnancier et le dossier du patient.
    Sortie,
    /// Comptage physique : le stock **devient** ce qui a été compté.
    Inventaire,
    /// Casse, péremption, retour, vol : le stock descend, hors
    /// délivrance. La ligne porte ce qui s'est passé.
    Perte,
}

impl Kind {
    /// Sa clé dans la base, stable : le registre se relit dans dix ans.
    pub fn as_key(self) -> &'static str {
        match self {
            Kind::Entree => "ENTREE",
            Kind::Sortie => "SORTIE",
            Kind::Inventaire => "INVENTAIRE",
            Kind::Perte => "PERTE",
        }
    }

    /// Une clé que cette version ne connaît pas n'est pas une erreur de
    /// lecture : c'est une ligne écrite par une version plus récente, et
    /// elle est lue comme une perte — le choix qui sous-estime le stock
    /// plutôt que de le surestimer.
    pub fn from_key(key: &str) -> Kind {
        match key {
            "ENTREE" => Kind::Entree,
            "SORTIE" => Kind::Sortie,
            "INVENTAIRE" => Kind::Inventaire,
            _ => Kind::Perte,
        }
    }

    /// La clé de son libellé français.
    pub fn label_key(self) -> &'static str {
        match self {
            Kind::Entree => "stup_kind_entree",
            Kind::Sortie => "stup_kind_sortie",
            Kind::Inventaire => "stup_kind_inventaire",
            Kind::Perte => "stup_kind_perte",
        }
    }

    /// Sa couleur dans la palette de données, fixe par nature : une
    /// couleur qui suivrait les données serait de la décoration.
    pub fn series(self) -> usize {
        match self {
            Kind::Entree => 2,
            Kind::Sortie => 0,
            Kind::Inventaire => 1,
            Kind::Perte => 3,
        }
    }

    /// Porte-t-elle un numéro d'ordonnancier et un dossier ? Seule la
    /// délivrance en porte : une réception n'a pas de patient, et en
    /// inventer un serait écrire un nom dans un registre pour rien.
    pub fn is_dispensing(self) -> bool {
        self == Kind::Sortie
    }

    pub const ALL: [Kind; 4] = [Kind::Entree, Kind::Sortie, Kind::Inventaire, Kind::Perte];
}

/// Une ligne du registre, telle que l'arithmétique la lit.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Move<'a> {
    pub kind: Kind,
    /// Toujours positive : c'est [`Kind`] qui dit le sens. Une quantité
    /// signée serait une deuxième façon de dire la même chose, donc une
    /// deuxième façon de se tromper.
    pub quantity: f64,
    /// Le jour, ISO. L'ordre du registre est celui-ci et non celui des
    /// identifiants : une réception saisie le lendemain n'est pas une
    /// réception du lendemain.
    pub day: &'a str,
    /// L'ordre de saisie, qui départage deux lignes du même jour.
    pub seq: i64,
}

/// Le solde après toutes ces lignes.
///
/// Ce n'est **pas** une somme. Un inventaire fixe le solde à ce qui a
/// été compté : additionner l'écart *et* poser le compte reviendrait à
/// le compter deux fois, et le registre partirait à la dérive dès le
/// premier comptage qui ne tombe pas juste.
///
/// Les lignes sont triées ici, par jour puis par ordre de saisie : ce
/// que la base rend n'a pas à être dans l'ordre, et un inventaire lu
/// avant les sorties qui le précèdent donnerait un solde faux.
pub fn balance(moves: &[Move]) -> f64 {
    let mut ordered: Vec<&Move> = moves.iter().collect();
    ordered.sort_by(|a, b| a.day.cmp(b.day).then(a.seq.cmp(&b.seq)));
    let mut stock = 0.0;
    for m in ordered {
        match m.kind {
            Kind::Entree => stock += m.quantity,
            Kind::Sortie | Kind::Perte => stock -= m.quantity,
            Kind::Inventaire => stock = m.quantity,
        }
    }
    stock
}

/// Le solde jour après jour, pour la courbe : une valeur par ligne, dans
/// l'ordre du registre.
///
/// Le dessin lit ça et rien d'autre — refaire l'arithmétique dans la vue
/// serait deux versions de la même règle, et un jour elles diffèrent.
pub fn running(moves: &[Move]) -> Vec<f64> {
    let mut ordered: Vec<&Move> = moves.iter().collect();
    ordered.sort_by(|a, b| a.day.cmp(b.day).then(a.seq.cmp(&b.seq)));
    let mut stock = 0.0;
    ordered
        .into_iter()
        .map(|m| {
            match m.kind {
                Kind::Entree => stock += m.quantity,
                Kind::Sortie | Kind::Perte => stock -= m.quantity,
                Kind::Inventaire => stock = m.quantity,
            }
            stock
        })
        .collect()
}

/// Le prochain numéro d'ordonnancier de l'année.
///
/// Séquentiel dans l'année et **jamais réattribué** : `used` est ce que
/// le registre porte déjà pour cette année, et le prochain est un de
/// plus que le plus grand. Un trou dans la suite reste un trou — une
/// ligne annulée l'est par une contre-passation, et son numéro ne
/// revient pas servir une autre délivrance.
pub fn next_number(used: &[u32]) -> u32 {
    used.iter().copied().max().unwrap_or(0) + 1
}

/// Le numéro tel qu'il s'écrit et se lit : « 2026-0042 ».
///
/// L'année devant, parce que la suite repart à un chaque année et qu'un
/// « 42 » seul ne désigne rien dans un registre de dix ans.
pub fn number_label(year: u32, no: u32) -> String {
    format!("{year}-{no:04}")
}

/// Ce qu'un inventaire a trouvé.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Discrepancy {
    /// Ce que le registre disait avant le comptage.
    pub expected: f64,
    /// Ce qui a été compté.
    pub counted: f64,
}

impl Discrepancy {
    /// Compté moins attendu : négatif, il manque.
    pub fn gap(self) -> f64 {
        self.counted - self.expected
    }

    /// L'écart mérite-t-il d'être expliqué ?
    ///
    /// Tout écart non nul en mérite un — c'est la règle, et il n'y a pas
    /// de seuil de tolérance dans le code de la santé publique. La marge
    /// ici n'est qu'une marge de **calcul** : les quantités sont des
    /// flottants, et 0,1 + 0,2 n'est pas 0,3 sur une machine.
    pub fn matters(self) -> bool {
        self.gap().abs() > 1e-6
    }
}

/// Pourquoi un produit est sur la liste de contrôle.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Why {
    /// Le solde du registre est négatif : impossible, donc une ligne
    /// manque. C'est le seul motif qui est une erreur et pas un rappel.
    Negative,
    /// Le solde est à zéro ou en dessous du seuil que l'officine a posé.
    Low,
    /// Jamais compté, ou compté il y a trop longtemps.
    Uncounted,
}

impl Why {
    /// L'ordre d'affichage : l'impossible d'abord, le rappel en dernier.
    fn rank(self) -> u8 {
        match self {
            Why::Negative => 0,
            Why::Low => 1,
            Why::Uncounted => 2,
        }
    }

    pub fn label_key(self) -> &'static str {
        match self {
            Why::Negative => "stup_why_negative",
            Why::Low => "stup_why_low",
            Why::Uncounted => "stup_why_uncounted",
        }
    }
}

/// Un produit suivi, et ce que le registre en dit.
#[derive(Clone, PartialEq, Debug)]
pub struct Followed {
    pub id: i64,
    pub label: String,
    pub unit: String,
    /// Le solde du registre.
    pub stock: f64,
    /// Le plancher que l'officine a posé ; zéro veut dire « pas de
    /// plancher », et non « plancher à zéro ».
    pub threshold: f64,
    /// Le dernier inventaire, ISO ; vide si jamais compté.
    pub last_count: String,
}

/// Une ligne de la liste de contrôle.
#[derive(Clone, PartialEq, Debug)]
pub struct ToCheck {
    pub id: i64,
    pub label: String,
    pub unit: String,
    pub stock: f64,
    pub why: Why,
    /// Depuis combien de jours le produit n'a pas été compté ; `None`
    /// s'il ne l'a jamais été.
    pub days: Option<i64>,
}

/// Ce qu'il faut aller compter, et pourquoi.
///
/// `max_days` est le délai que l'officine se donne entre deux comptages
/// (la loi en demande un par an ; une officine sérieuse en fait un par
/// mois sur les produits qui bougent). `today` est le jour, donné et non
/// lu à une horloge, pour que la liste se teste.
///
/// Un produit n'apparaît qu'une fois, sous le motif le plus grave : un
/// stock négatif jamais compté est un stock négatif, et le dire deux
/// fois n'ajoute rien à la ligne qu'il faut aller chercher.
pub fn to_check(followed: &[Followed], today: &str, max_days: i64) -> Vec<ToCheck> {
    let mut out: Vec<ToCheck> = Vec::new();
    for f in followed {
        let days = days_between(&f.last_count, today);
        // Jamais compté et compté il y a trop longtemps sont le même
        // motif : dans les deux cas la ligne à aller chercher est la
        // même, et `days` dit lequel des deux c'est.
        let never = f.last_count.trim().is_empty();
        let why = if f.stock < -1e-6 {
            Some(Why::Negative)
        } else if f.threshold > 0.0 && f.stock <= f.threshold + 1e-6 {
            Some(Why::Low)
        } else if never || days.is_some_and(|d| d > max_days) {
            Some(Why::Uncounted)
        } else {
            None
        };
        if let Some(why) = why {
            out.push(ToCheck {
                id: f.id,
                label: f.label.clone(),
                unit: f.unit.clone(),
                stock: f.stock,
                why,
                days,
            });
        }
    }
    // Le plus grave en tête, puis le stock le plus bas, puis le nom —
    // un ordre stable, sans quoi la liste imprimée le lundi et celle du
    // mardi ne se comparent pas.
    out.sort_by(|a, b| {
        a.why
            .rank()
            .cmp(&b.why.rank())
            .then(a.stock.total_cmp(&b.stock))
            .then(a.label.cmp(&b.label))
    });
    out
}

/// Combien de jours séparent deux dates ISO, `None` si l'une des deux
/// n'en est pas une.
///
/// Le calendrier grégorien par le compte des jours depuis une origine
/// commune : pas de bibliothèque de dates, pas d'horloge, et un test qui
/// tient les années bissextiles.
pub fn days_between(from: &str, to: &str) -> Option<i64> {
    Some(day_number(to)? - day_number(from)?)
}

/// Le rang d'une date ISO dans le calendrier, en jours.
fn day_number(iso: &str) -> Option<i64> {
    let mut parts = iso.trim().split('-');
    let y: i64 = parts.next()?.parse().ok()?;
    let m: i64 = parts.next()?.parse().ok()?;
    let d: i64 = parts.next()?.parse().ok()?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    // Le calcul des jours juliens : mars devient le premier mois, ce qui
    // met le jour de février qui manque à la fin de l'année et fait
    // disparaître le cas particulier du bissextile.
    let a = (14 - m) / 12;
    let y2 = y + 4800 - a;
    let m2 = m + 12 * a - 3;
    Some(d + (153 * m2 + 2) / 5 + 365 * y2 + y2 / 4 - y2 / 100 + y2 / 400 - 32045)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mv(kind: Kind, quantity: f64, day: &str, seq: i64) -> Move<'_> {
        Move {
            kind,
            quantity,
            day,
            seq,
        }
    }

    /// La balance n'est pas une somme : un inventaire **pose** le solde.
    ///
    /// C'est la règle qui décide de tout. Écrite comme une somme
    /// signée, un comptage qui trouve deux comprimés de moins ajouterait
    /// son écart au solde *et* laisserait le solde d'avant, et le
    /// registre dériverait à partir du premier comptage qui ne tombe pas
    /// juste — c'est-à-dire à partir du premier.
    #[test]
    fn an_inventory_sets_the_balance_it_does_not_add_to_it() {
        let moves = [
            mv(Kind::Entree, 30.0, "2026-01-05", 1),
            mv(Kind::Sortie, 14.0, "2026-01-08", 2),
            // Le registre dit 16 ; le comptage en trouve 15.
            mv(Kind::Inventaire, 15.0, "2026-01-10", 3),
            mv(Kind::Sortie, 5.0, "2026-01-12", 4),
        ];
        assert!((balance(&moves) - 10.0).abs() < 1e-9, "{}", balance(&moves));

        // Et l'écart de ce comptage se lit pour ce qu'il est.
        let before = balance(&moves[..2]);
        let d = Discrepancy {
            expected: before,
            counted: 15.0,
        };
        assert!((d.gap() + 1.0).abs() < 1e-9, "il manque un comprimé");
        assert!(d.matters());
        // Un comptage qui tombe juste ne demande pas d'explication.
        assert!(!Discrepancy {
            expected: 16.0,
            counted: 16.0
        }
        .matters());
    }

    /// L'ordre du registre est celui des jours, pas celui de la saisie
    /// ni celui que la base rend.
    ///
    /// Une réception notée le lendemain n'est pas une réception du
    /// lendemain, et un inventaire lu avant les sorties qui le précèdent
    /// donnerait un solde faux — c'est-à-dire un manquant inventé.
    #[test]
    fn the_register_is_read_in_the_order_of_its_days() {
        let jumbled = [
            mv(Kind::Sortie, 5.0, "2026-01-12", 4),
            mv(Kind::Inventaire, 15.0, "2026-01-10", 3),
            mv(Kind::Entree, 30.0, "2026-01-05", 1),
            mv(Kind::Sortie, 14.0, "2026-01-08", 2),
        ];
        assert!((balance(&jumbled) - 10.0).abs() < 1e-9);
        // Deux lignes du même jour sont départagées par l'ordre de
        // saisie : l'inventaire du matin puis la sortie de l'après-midi
        // ne donnent pas le même solde que l'inverse.
        let same_day = [
            mv(Kind::Inventaire, 20.0, "2026-02-02", 1),
            mv(Kind::Sortie, 6.0, "2026-02-02", 2),
        ];
        assert!((balance(&same_day) - 14.0).abs() < 1e-9);
        let reversed = [
            mv(Kind::Sortie, 6.0, "2026-02-02", 1),
            mv(Kind::Inventaire, 20.0, "2026-02-02", 2),
        ];
        assert!((balance(&reversed) - 20.0).abs() < 1e-9);
    }

    /// La courbe et le solde disent la même chose, parce que c'est le
    /// même calcul : la dernière valeur de la courbe **est** la balance.
    #[test]
    fn the_curve_ends_where_the_balance_is() {
        let moves = [
            mv(Kind::Entree, 30.0, "2026-01-05", 1),
            mv(Kind::Sortie, 14.0, "2026-01-08", 2),
            mv(Kind::Inventaire, 15.0, "2026-01-10", 3),
            mv(Kind::Perte, 2.0, "2026-01-11", 4),
        ];
        let curve = running(&moves);
        assert_eq!(curve.len(), moves.len());
        assert_eq!(curve, vec![30.0, 16.0, 15.0, 13.0]);
        assert!((curve.last().copied().unwrap() - balance(&moves)).abs() < 1e-9);
        assert!(running(&[]).is_empty());
        assert_eq!(balance(&[]), 0.0);
    }

    /// Le numéro d'ordonnancier ne revient jamais en arrière et ne
    /// rebouche jamais un trou.
    ///
    /// Un numéro annulé l'est par une contre-passation, et le réattribuer
    /// ferait exister deux délivrances sous le même numéro — c'est-à-dire
    /// un registre qui ne prouve plus rien.
    #[test]
    fn a_dispensing_number_is_never_reused() {
        assert_eq!(next_number(&[]), 1);
        assert_eq!(next_number(&[1, 2, 3]), 4);
        // Le 3 a été annulé : le suivant est quand même le 5.
        assert_eq!(next_number(&[1, 2, 4]), 5);
        // L'ordre dans lequel la base les rend ne change rien.
        assert_eq!(next_number(&[4, 1, 2]), 5);
        assert_eq!(number_label(2026, 42), "2026-0042");
        assert_eq!(number_label(2026, 1), "2026-0001");
        // Au-delà de dix mille, le numéro s'écrit en entier plutôt que
        // d'être tronqué : une officine qui délivre beaucoup ne perd pas
        // ses quatre premiers chiffres.
        assert_eq!(number_label(2026, 12345), "2026-12345");
    }

    /// Une clé que cette version ne connaît pas est lue comme une perte.
    ///
    /// Le choix qui sous-estime le stock plutôt que de le surestimer :
    /// une ligne écrite par une version plus récente doit faire chercher
    /// une explication, jamais rassurer.
    #[test]
    fn an_unknown_line_lowers_the_stock_rather_than_raising_it() {
        for k in Kind::ALL {
            assert_eq!(Kind::from_key(k.as_key()), k);
        }
        assert_eq!(Kind::from_key("QUELQUE CHOSE"), Kind::Perte);
        assert_eq!(Kind::from_key(""), Kind::Perte);
        // Et seule la délivrance porte un numéro et un dossier.
        assert!(Kind::Sortie.is_dispensing());
        for k in [Kind::Entree, Kind::Inventaire, Kind::Perte] {
            assert!(!k.is_dispensing(), "{k:?}");
        }
    }

    /// La liste de contrôle : un produit, un motif, le plus grave.
    #[test]
    fn the_control_list_says_what_to_go_and_count_and_why() {
        let f = |id: i64, label: &str, stock: f64, threshold: f64, last: &str| Followed {
            id,
            label: label.to_owned(),
            unit: "comprimé".to_owned(),
            stock,
            threshold,
            last_count: last.to_owned(),
        };
        let base = [
            // Impossible : une ligne manque au registre.
            f(1, "Skenan LP 30 mg", -2.0, 10.0, "2026-08-01"),
            // Sous le plancher que l'officine s'est donné.
            f(2, "Oxycontin 10 mg", 4.0, 10.0, "2026-08-20"),
            // Compté il y a trop longtemps.
            f(3, "Durogesic 25", 40.0, 5.0, "2025-06-01"),
            // Jamais compté.
            f(4, "Méthadone 40 mg", 25.0, 5.0, ""),
            // Rien à signaler.
            f(5, "Subutex 8 mg", 60.0, 10.0, "2026-08-25"),
            // Pas de plancher posé : zéro veut dire « pas de plancher »
            // et non « plancher à zéro », donc un stock bas ne suffit
            // pas — c'est le comptage récent qui le tient hors liste.
            f(6, "Ritaline 10 mg", 1.0, 0.0, "2026-08-25"),
        ];
        let list = to_check(&base, "2026-08-29", 60);
        let ids: Vec<i64> = list.iter().map(|c| c.id).collect();
        assert_eq!(
            ids,
            vec![1, 2, 4, 3],
            "l'impossible d'abord, le rappel après"
        );
        assert_eq!(list[0].why, Why::Negative);
        assert_eq!(list[1].why, Why::Low);
        assert_eq!(list[2].why, Why::Uncounted);
        assert_eq!(
            list[2].days, None,
            "jamais compté n'est pas « il y a n jours »"
        );
        assert_eq!(list[3].why, Why::Uncounted);
        assert_eq!(list[3].days, Some(454));
        // Un produit n'est jamais sur la liste deux fois, sous deux
        // motifs : la ligne à aller chercher est une ligne.
        let mut seen: Vec<i64> = list.iter().map(|c| c.id).collect();
        let n = seen.len();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), n);
        // Un stock exactement au plancher est déjà bas : le plancher est
        // le moment de recommander, pas celui d'être à court.
        let at = [f(9, "Pile au seuil", 10.0, 10.0, "2026-08-25")];
        assert_eq!(to_check(&at, "2026-08-29", 60)[0].why, Why::Low);
    }

    /// Le compte des jours, y compris à travers un 29 février.
    #[test]
    fn the_days_are_counted_across_years_and_leap_days() {
        assert_eq!(days_between("2026-08-01", "2026-08-29"), Some(28));
        assert_eq!(days_between("2026-08-29", "2026-08-29"), Some(0));
        // 2024 est bissextile : février y a vingt-neuf jours.
        assert_eq!(days_between("2024-02-28", "2024-03-01"), Some(2));
        // 2023 ne l'est pas.
        assert_eq!(days_between("2023-02-28", "2023-03-01"), Some(1));
        // 2000 est bissextile, 1900 ne l'était pas : la règle des
        // siècles, que la plupart des calculs à la main oublient.
        assert_eq!(days_between("2000-02-28", "2000-03-01"), Some(2));
        assert_eq!(days_between("1900-02-28", "1900-03-01"), Some(1));
        // Une année entière.
        assert_eq!(days_between("2025-01-01", "2026-01-01"), Some(365));
        // Une date en arrière compte en négatif, et ce qui n'est pas une
        // date ne compte pas du tout.
        assert_eq!(days_between("2026-08-29", "2026-08-01"), Some(-28));
        for bad in ["", "hier", "2026-13-01", "2026-08-32", "2026-08", "x-y-z"] {
            assert_eq!(days_between(bad, "2026-08-29"), None, "{bad}");
            assert_eq!(days_between("2026-08-29", bad), None, "{bad}");
        }
    }
}
