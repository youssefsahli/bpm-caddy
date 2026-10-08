//! La compression médicale : les classes, la prise de mesures, les grilles
//! de tailles des fabricants et le renouvellement.
//!
//! Ce que le module sait, et d'où il le tient :
//!
//! - **Les classes** sont celles de la classification française, en mmHg
//!   (HAS, fiches de bon usage de 2010 ; ameli), et leur équivalent en
//!   hPa mesuré à la cheville selon la LPP (titre II, chapitre 1er,
//!   section D). Les classes allemandes (RAL) sont différentes et ne
//!   sont pas lues ici.
//! - **Les points de mesure** sont ceux des guides des fabricants
//!   (repères B, B1, C, D, E, F, G sur la face externe de la jambe) ;
//!   ce que chaque article demande suit le même guide.
//! - **Les grilles de tailles ne sont pas livrées** : chaque fabricant a
//!   la sienne, et une grille recopiée d'un catalogue devient fausse au
//!   catalogue suivant. L'officine saisit celles des modèles qu'elle
//!   délivre ; le module lit le texte et dit quelles tailles conviennent
//!   aux mesures prises.
//! - **Le renouvellement** suit le mémo de l'Assurance Maladie
//!   (24/07/2025) : garantie de 6 mois, et sans changement de classe ni
//!   de taille, 2 paires par période de 6 mois, soit 4 paires par an au
//!   plus.
//!
//! Pur et sans horloge.

// ---------------------------------------------------------------------
// Les classes
// ---------------------------------------------------------------------

/// Une classe de compression, classification française.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Class {
    pub key: &'static str,
    pub label: &'static str,
    /// La pression à la cheville en mmHg, telle que la HAS l'écrit.
    pub mmhg: &'static str,
    /// La même classe en hPa, selon la LPP.
    pub hpa: &'static str,
}

/// Les quatre classes françaises.
pub const CLASSES: [Class; 4] = [
    Class {
        key: "I",
        label: "Classe I",
        mmhg: "10 à 15 mmHg",
        hpa: "13 à 20 hPa",
    },
    Class {
        key: "II",
        label: "Classe II",
        mmhg: "15,1 à 20 mmHg",
        hpa: "20,1 à 27 hPa",
    },
    Class {
        key: "III",
        label: "Classe III",
        mmhg: "20,1 à 36 mmHg",
        hpa: "27,1 à 48 hPa",
    },
    Class {
        key: "IV",
        label: "Classe IV",
        mmhg: "plus de 36 mmHg",
        hpa: "plus de 48 hPa",
    },
];

pub fn class(key: &str) -> Option<&'static Class> {
    CLASSES.iter().find(|c| c.key == key)
}

// ---------------------------------------------------------------------
// Les mesures
// ---------------------------------------------------------------------

/// Un point de mesure, circonférence ou hauteur.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Point {
    /// La clé écrite en base et dans les grilles (`cB`, `lG`…).
    pub key: &'static str,
    pub label: &'static str,
    /// Où le ruban se pose.
    pub hint: &'static str,
}

/// Les points de mesure d'un membre inférieur, de la cheville à la
/// taille. Les repères se prennent sur la face externe de la jambe.
pub const POINTS: [Point; 11] = [
    Point {
        key: "cB",
        label: "Cheville (cB)",
        hint: "À l'endroit le plus fin de la cheville",
    },
    Point {
        key: "cB1",
        label: "Naissance du mollet (cB1)",
        hint: "À la naissance des muscles jumeaux",
    },
    Point {
        key: "cC",
        label: "Mollet (cC)",
        hint: "Au point le plus fort du mollet",
    },
    Point {
        key: "cD",
        label: "Sous le genou (cD)",
        hint: "Sur la tête du péroné, 3 à 4 cm sous la rotule",
    },
    Point {
        key: "cE",
        label: "Genou (cE)",
        hint: "Au milieu de la rotule",
    },
    Point {
        key: "cF",
        label: "Mi-cuisse (cF)",
        hint: "À mi-distance entre le genou et le pli fessier",
    },
    Point {
        key: "cG",
        label: "Haut de cuisse (cG)",
        hint: "Au pli fessier",
    },
    Point {
        key: "lD",
        label: "Hauteur sol-genou (lD)",
        hint: "Du sol au point D, sous le genou",
    },
    Point {
        key: "lG",
        label: "Hauteur sol-cuisse (lG)",
        hint: "Du sol au pli fessier",
    },
    Point {
        key: "hanches",
        label: "Tour de hanches",
        hint: "Au plus fort des hanches",
    },
    Point {
        key: "taille",
        label: "Tour de taille",
        hint: "À la taille",
    },
];

pub fn point(key: &str) -> Option<&'static Point> {
    POINTS.iter().find(|p| p.key == key)
}

/// Le type d'article : ce qu'il faut mesurer en dépend.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Article {
    /// Chaussette (bas jarret), jusque sous le genou.
    Chaussette,
    /// Bas-cuisse.
    BasCuisse,
    /// Collant.
    Collant,
}

impl Article {
    pub const ALL: [Article; 3] = [Article::Chaussette, Article::BasCuisse, Article::Collant];

    pub fn key(self) -> &'static str {
        match self {
            Article::Chaussette => "chaussette",
            Article::BasCuisse => "bas_cuisse",
            Article::Collant => "collant",
        }
    }

    pub fn from_key(key: &str) -> Option<Article> {
        Article::ALL.into_iter().find(|a| a.key() == key)
    }

    pub fn label(self) -> &'static str {
        match self {
            Article::Chaussette => "Chaussettes",
            Article::BasCuisse => "Bas-cuisse",
            Article::Collant => "Collant",
        }
    }

    /// Les points que l'article demande, dans l'ordre où on les prend.
    pub fn points(self) -> &'static [&'static str] {
        match self {
            Article::Chaussette => &["cB", "cB1", "cC", "cD", "lD"],
            Article::BasCuisse => &["cB", "cB1", "cC", "cD", "cE", "cF", "cG", "lG"],
            Article::Collant => &[
                "cB", "cB1", "cC", "cD", "cE", "cF", "cG", "lG", "hanches", "taille",
            ],
        }
    }

    /// Un collant habille les deux jambes et le bassin : les tours de
    /// hanches et de taille ne se prennent qu'une fois.
    pub fn per_leg(self, point: &str) -> bool {
        !matches!(point, "hanches" | "taille")
    }
}

/// Le côté d'une mesure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Droite,
    Gauche,
}

/// Une série de mesures en centimètres : `(point, côté, valeur)`. Les
/// points pris une seule fois (hanches, taille) sont rangés à droite.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Measures {
    pub values: Vec<(String, Side, f64)>,
}

impl Measures {
    pub fn get(&self, point: &str, side: Side) -> Option<f64> {
        self.values
            .iter()
            .find(|(p, s, _)| p == point && *s == side)
            .map(|(_, _, v)| *v)
    }

    pub fn set(&mut self, point: &str, side: Side, value: Option<f64>) {
        self.values.retain(|(p, s, _)| !(p == point && *s == side));
        if let Some(v) = value {
            self.values.push((point.to_owned(), side, v));
        }
    }

    /// La forme écrite en base : `cB.D=22.5;cC.G=35`.
    pub fn encode(&self) -> String {
        self.values
            .iter()
            .map(|(p, s, v)| {
                let side = match s {
                    Side::Droite => "D",
                    Side::Gauche => "G",
                };
                format!("{p}.{side}={}", fmt_cm(*v))
            })
            .collect::<Vec<_>>()
            .join(";")
    }

    pub fn decode(text: &str) -> Measures {
        let mut m = Measures::default();
        for part in text.split(';') {
            let Some((k, v)) = part.split_once('=') else {
                continue;
            };
            let Some((p, s)) = k.trim().rsplit_once('.') else {
                continue;
            };
            let side = match s {
                "D" => Side::Droite,
                "G" => Side::Gauche,
                _ => continue,
            };
            if let Some(v) = parse_cm(v) {
                m.set(p, side, Some(v));
            }
        }
        m
    }
}

/// Un nombre de centimètres tapé au comptoir : virgule ou point, et
/// rien d'autre. Une mesure hors de 5 à 200 cm n'est pas une mesure.
pub fn parse_cm(text: &str) -> Option<f64> {
    let t = text.trim().trim_end_matches("cm").trim().replace(',', ".");
    let v: f64 = t.parse().ok()?;
    (5.0..=200.0).contains(&v).then_some(v)
}

/// Un nombre de centimètres écrit à la française, au demi-centimètre.
pub fn fmt_cm(v: f64) -> String {
    let s = format!("{v:.1}");
    let s = s.trim_end_matches('0').trim_end_matches('.').to_owned();
    s.replace('.', ",")
}

/// Ce que la vérification d'une prise de mesures relève.
#[derive(Clone, Debug, PartialEq)]
pub enum Finding {
    /// Un point demandé par l'article n'a pas été pris.
    Missing(&'static str, Side),
    /// Une circonférence plus haute est plus petite qu'une plus basse
    /// sur la même jambe (cheville plus forte que le mollet) : erreur
    /// de saisie ou de repère, à reprendre avant de choisir une taille.
    Inverted {
        lower: &'static str,
        upper: &'static str,
        side: Side,
    },
    /// Les deux jambes diffèrent en un point : la taille peut différer
    /// à droite et à gauche, ce que la prescription doit alors dire.
    Asymmetry { point: &'static str, diff: f64 },
}

/// Vérifier une prise de mesures pour un article.
///
/// `sides` dit quelles jambes sont mesurées (une seule dans un
/// lymphœdème unilatéral).
pub fn check(article: Article, sides: &[Side], m: &Measures) -> Vec<Finding> {
    let mut out = Vec::new();
    for &p in article.points() {
        let key = point(p).map(|x| x.key).unwrap_or("");
        if article.per_leg(p) {
            for &s in sides {
                if m.get(p, s).is_none() {
                    out.push(Finding::Missing(key, s));
                }
            }
        } else if m.get(p, Side::Droite).is_none() {
            out.push(Finding::Missing(key, Side::Droite));
        }
    }
    // La jambe s'élargit de la cheville au mollet, et du genou à la
    // cuisse. Le genou et le dessous du genou ne sont pas comparés au
    // mollet : un mollet fort dépasse le genou chez beaucoup.
    const ORDER: [(&str, &str); 3] = [("cB", "cC"), ("cB", "cB1"), ("cE", "cG")];
    for &s in sides {
        for (lower, upper) in ORDER {
            if let (Some(a), Some(b)) = (m.get(lower, s), m.get(upper, s)) {
                if b < a {
                    out.push(Finding::Inverted {
                        lower: point(lower).map(|x| x.key).unwrap_or(""),
                        upper: point(upper).map(|x| x.key).unwrap_or(""),
                        side: s,
                    });
                }
            }
        }
    }
    if sides.contains(&Side::Droite) && sides.contains(&Side::Gauche) {
        for &p in article.points().iter().filter(|p| article.per_leg(p)) {
            if let (Some(d), Some(g)) = (m.get(p, Side::Droite), m.get(p, Side::Gauche)) {
                let diff = (d - g).abs();
                // Au-delà de la précision de la mesure (le
                // demi-centimètre) : un écart d'un centimètre au moins.
                if diff >= 1.0 {
                    out.push(Finding::Asymmetry {
                        point: point(p).map(|x| x.key).unwrap_or(""),
                        diff,
                    });
                }
            }
        }
    }
    out
}

// ---------------------------------------------------------------------
// Les grilles de tailles
// ---------------------------------------------------------------------

/// Une taille d'une grille : son nom, et pour chaque point l'intervalle
/// de mesures qu'elle accepte (bornes comprises).
#[derive(Clone, Debug, PartialEq)]
pub struct Size {
    pub name: String,
    pub ranges: Vec<(String, f64, f64)>,
}

/// Lire une grille saisie par l'équipe, une taille par ligne :
///
/// ```text
/// 1 : cB 18-20 ; cC 28-34 ; lD 36-40
/// 2 : cB 20-22 ; cC 30-37
/// ```
///
/// Le nom de la taille avant les deux-points, puis des points séparés
/// par des points-virgules. Une ligne illisible est rendue dans les
/// erreurs, avec son numéro, plutôt que d'être ignorée en silence.
pub fn parse_grid(text: &str) -> (Vec<Size>, Vec<(usize, String)>) {
    let mut sizes = Vec::new();
    let mut errors = Vec::new();
    for (i, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((name, rest)) = line.split_once(':') else {
            errors.push((i + 1, line.to_owned()));
            continue;
        };
        let mut ranges = Vec::new();
        let mut ok = !name.trim().is_empty();
        for part in rest.split(';').map(str::trim).filter(|p| !p.is_empty()) {
            let mut words = part.split_whitespace();
            let (Some(key), Some(span), None) = (words.next(), words.next(), words.next()) else {
                ok = false;
                break;
            };
            let Some((lo, hi)) = span.split_once('-') else {
                ok = false;
                break;
            };
            match (parse_cm(lo), parse_cm(hi), point(key)) {
                (Some(lo), Some(hi), Some(_)) if lo <= hi => {
                    ranges.push((key.to_owned(), lo, hi));
                }
                _ => {
                    ok = false;
                    break;
                }
            }
        }
        if ok && !ranges.is_empty() {
            sizes.push(Size {
                name: name.trim().to_owned(),
                ranges,
            });
        } else {
            errors.push((i + 1, line.to_owned()));
        }
    }
    (sizes, errors)
}

/// Ce qu'une taille répond aux mesures d'une jambe.
#[derive(Clone, Debug, PartialEq)]
pub struct Fit {
    pub size: String,
    /// Les points de la grille que la mesure ne satisfait pas, avec la
    /// mesure prise.
    pub outside: Vec<(String, f64)>,
    /// Les points de la grille qui n'ont pas été mesurés.
    pub unmeasured: Vec<String>,
}

impl Fit {
    pub fn fits(&self) -> bool {
        self.outside.is_empty() && self.unmeasured.is_empty()
    }
}

/// Lire chaque taille de la grille contre les mesures d'une jambe ; les
/// tailles qui conviennent d'abord, puis celles qui manquent d'un seul
/// point. Les points pris une fois (hanches, taille) sont lus à droite.
pub fn fit(sizes: &[Size], m: &Measures, side: Side) -> Vec<Fit> {
    let mut out: Vec<Fit> = sizes
        .iter()
        .map(|s| {
            let mut f = Fit {
                size: s.name.clone(),
                outside: Vec::new(),
                unmeasured: Vec::new(),
            };
            for (key, lo, hi) in &s.ranges {
                let leg = if matches!(key.as_str(), "hanches" | "taille") {
                    Side::Droite
                } else {
                    side
                };
                match m.get(key, leg) {
                    Some(v) if v >= *lo && v <= *hi => {}
                    Some(v) => f.outside.push((key.clone(), v)),
                    None => f.unmeasured.push(key.clone()),
                }
            }
            f
        })
        .collect();
    out.sort_by_key(|f| (!f.fits(), f.outside.len() + f.unmeasured.len()));
    out
}

// ---------------------------------------------------------------------
// Le renouvellement
// ---------------------------------------------------------------------

/// Une délivrance passée : date ISO, nombre de paires, classe, et
/// taille — un changement de classe ou de taille ouvre un nouveau droit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Delivery {
    pub on: String,
    pub pairs: u32,
    pub class: String,
    pub size: String,
}

/// Paires prises en charge par période de 6 mois, sans changement de
/// classe ni de taille.
pub const PAIRS_PER_HALF_YEAR: u32 = 2;
/// Et par an.
pub const PAIRS_PER_YEAR: u32 = 4;

/// Ce que le renouvellement permet aujourd'hui.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Renewal {
    /// Paires délivrées dans les 6 derniers mois, même classe et taille.
    pub half_year: u32,
    /// Paires délivrées dans les 12 derniers mois, même classe et taille.
    pub year: u32,
    /// Paires encore possibles aujourd'hui.
    pub allowed: u32,
    /// ISO : la date à partir de laquelle une paire de plus le sera, si
    /// aucune ne l'est aujourd'hui.
    pub next: Option<String>,
}

/// Les paires accordées le jour `day`, sans chercher la date suivante.
fn renewal_allowed(history: &[Delivery], class: &str, size: &str, day: &str) -> u32 {
    let six = crate::date::add_months(day, -6).unwrap_or_default();
    let twelve = crate::date::add_months(day, -12).unwrap_or_default();
    let within = |from: &str| -> u32 {
        history
            .iter()
            .filter(|d| d.class == class && d.size == size && d.on.as_str() <= day)
            .filter(|d| d.on.as_str() > from)
            .map(|d| d.pairs)
            .sum()
    };
    PAIRS_PER_HALF_YEAR
        .saturating_sub(within(&six))
        .min(PAIRS_PER_YEAR.saturating_sub(within(&twelve)))
}

/// Lire les délivrances contre la règle : 2 paires par 6 mois, 4 par an,
/// pour une même classe et une même taille.
pub fn renewal(history: &[Delivery], class: &str, size: &str, today: &str) -> Renewal {
    let six = crate::date::add_months(today, -6).unwrap_or_default();
    let twelve = crate::date::add_months(today, -12).unwrap_or_default();
    let same: Vec<&Delivery> = history
        .iter()
        .filter(|d| d.class == class && d.size == size && d.on.as_str() <= today)
        .collect();
    let half_year: u32 = same
        .iter()
        .filter(|d| d.on.as_str() > six.as_str())
        .map(|d| d.pairs)
        .sum();
    let year: u32 = same
        .iter()
        .filter(|d| d.on.as_str() > twelve.as_str())
        .map(|d| d.pairs)
        .sum();
    let allowed = PAIRS_PER_HALF_YEAR
        .saturating_sub(half_year)
        .min(PAIRS_PER_YEAR.saturating_sub(year));
    // Le jour où une paire se libère : le premier jour, après
    // aujourd'hui, où la règle en accorde une. Les candidats sont les
    // sorties de fenêtre de chaque délivrance — et les quelques jours qui
    // suivent, parce qu'un ajout de mois en fin de mois (31 août plus six
    // mois) ne retombe pas exactement là où la fenêtre se referme. Lire la
    // règle à chaque candidat traite aussi le cas où la plus ancienne
    // sortie ne libère pas assez de paires.
    let next = (allowed == 0)
        .then(|| {
            let mut candidates: Vec<String> = same
                .iter()
                .flat_map(|d| [6, 12].map(|m| crate::date::add_months(&d.on, m)))
                .flatten()
                .flat_map(|c| (0..4).filter_map(move |k| crate::date::add_days(&c, k)))
                .filter(|c| c.as_str() > today)
                .collect();
            candidates.sort();
            candidates.dedup();
            candidates
                .into_iter()
                .find(|c| renewal_allowed(history, class, size, c) > 0)
        })
        .flatten();
    Renewal {
        half_year,
        year,
        allowed,
        next,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_article_asks_for_points_that_exist() {
        for a in Article::ALL {
            for p in a.points() {
                assert!(point(p).is_some(), "{p}");
            }
            assert_eq!(Article::from_key(a.key()), Some(a));
        }
        // Une chaussette ne demande rien au-dessus du genou.
        assert!(!Article::Chaussette.points().contains(&"cG"));
        assert!(Article::Collant.points().contains(&"hanches"));
    }

    #[test]
    fn a_measure_reads_as_typed_at_the_counter() {
        assert_eq!(parse_cm("22,5"), Some(22.5));
        assert_eq!(parse_cm(" 35 cm"), Some(35.0));
        assert_eq!(parse_cm("2"), None, "pas une mesure de jambe");
        assert_eq!(parse_cm("abc"), None);
        assert_eq!(fmt_cm(22.5), "22,5");
        assert_eq!(fmt_cm(35.0), "35");
    }

    #[test]
    fn measures_travel_through_their_text() {
        let mut m = Measures::default();
        m.set("cB", Side::Droite, Some(22.5));
        m.set("cC", Side::Gauche, Some(35.0));
        m.set("hanches", Side::Droite, Some(102.0));
        let back = Measures::decode(&m.encode());
        assert_eq!(back.get("cB", Side::Droite), Some(22.5));
        assert_eq!(back.get("cC", Side::Gauche), Some(35.0));
        assert_eq!(back.get("hanches", Side::Droite), Some(102.0));
        // Effacer une mesure la retire.
        m.set("cB", Side::Droite, None);
        assert_eq!(m.get("cB", Side::Droite), None);
    }

    #[test]
    fn the_check_names_what_is_missing_inverted_or_uneven() {
        let mut m = Measures::default();
        for (p, d, g) in [
            ("cB", 22.0, 22.0),
            ("cB1", 27.0, 27.0),
            ("cC", 36.0, 38.5),
            ("cD", 34.0, 34.0),
        ] {
            m.set(p, Side::Droite, Some(d));
            m.set(p, Side::Gauche, Some(g));
        }
        let f = check(Article::Chaussette, &[Side::Droite, Side::Gauche], &m);
        // 0,5 cm est la précision de la mesure : pas un écart.
        m.set("cD", Side::Gauche, Some(34.5));
        assert!(
            !check(Article::Chaussette, &[Side::Droite, Side::Gauche], &m)
                .iter()
                .any(|x| matches!(x, Finding::Asymmetry { point: "cD", .. }))
        );
        assert!(f.contains(&Finding::Missing("lD", Side::Droite)));
        assert!(f.contains(&Finding::Missing("lD", Side::Gauche)));
        assert!(f.contains(&Finding::Asymmetry {
            point: "cC",
            diff: 2.5
        }));
        // Cheville plus forte que le mollet : une erreur de repère.
        m.set("cB", Side::Droite, Some(40.0));
        let f = check(Article::Chaussette, &[Side::Droite], &m);
        assert!(f.contains(&Finding::Inverted {
            lower: "cB",
            upper: "cC",
            side: Side::Droite
        }));
        // Une seule jambe mesurée : pas d'écart à lire.
        assert!(!f.iter().any(|x| matches!(x, Finding::Asymmetry { .. })));
    }

    #[test]
    fn a_grid_reads_line_by_line_and_names_what_it_cannot_read() {
        let (sizes, errors) = parse_grid(
            "# Modèle de démonstration\n1 : cB 18-20 ; cC 28-34 ; lD 36-40\n2: cB 20-22; cC 30-37\nXL cB 1-2\n3 : cZ 1-2\n",
        );
        assert_eq!(sizes.len(), 2);
        assert_eq!(sizes[0].name, "1");
        assert_eq!(sizes[0].ranges[2], ("lD".to_owned(), 36.0, 40.0));
        assert_eq!(errors.iter().map(|e| e.0).collect::<Vec<_>>(), [4, 5]);
    }

    #[test]
    fn the_sizes_that_fit_come_first() {
        let (sizes, _) =
            parse_grid("1 : cB 18-20 ; cC 28-34\n2 : cB 20-22 ; cC 30-37\n3 : cB 22-24 ; cC 33-40");
        let mut m = Measures::default();
        m.set("cB", Side::Droite, Some(21.0));
        m.set("cC", Side::Droite, Some(33.0));
        let f = fit(&sizes, &m, Side::Droite);
        assert_eq!(f[0].size, "2");
        assert!(f[0].fits());
        assert!(!f[1].fits());
        // Une jambe non mesurée en un point de la grille : rien ne va.
        let f = fit(&sizes, &Measures::default(), Side::Gauche);
        assert!(f.iter().all(|x| !x.fits()));
        assert_eq!(f[0].unmeasured, ["cB", "cC"]);
    }

    #[test]
    fn renewal_follows_two_pairs_a_half_year_and_four_a_year() {
        let d = |on: &str, pairs| Delivery {
            on: on.to_owned(),
            pairs,
            class: "II".to_owned(),
            size: "2".to_owned(),
        };
        let today = "2026-10-08";
        // Rien de délivré : deux paires.
        assert_eq!(renewal(&[], "II", "2", today).allowed, 2);
        // Deux paires en juin : rien avant décembre.
        let r = renewal(&[d("2026-06-03", 2)], "II", "2", today);
        assert_eq!(r.allowed, 0);
        assert_eq!(r.next.as_deref(), Some("2026-12-03"));
        // Une paire en juin : une encore.
        assert_eq!(renewal(&[d("2026-06-03", 1)], "II", "2", today).allowed, 1);
        // Quatre paires dans l'année : la limite annuelle tient même
        // quand le semestre est libre.
        let h = [d("2025-11-01", 2), d("2026-03-01", 2)];
        let r = renewal(&h, "II", "2", today);
        assert_eq!((r.half_year, r.year, r.allowed), (0, 4, 0));
        assert_eq!(r.next.as_deref(), Some("2026-11-01"));
        // Un changement de taille ouvre un nouveau droit.
        assert_eq!(renewal(&h, "II", "3", today).allowed, 2);
        // Fin de mois : deux paires le 31 août ne libèrent rien le
        // 28 février, et la date annoncée est celle où la règle accorde
        // vraiment une paire.
        let r = renewal(&[d("2026-08-31", 2)], "II", "2", "2026-10-08");
        let next = r.next.unwrap();
        assert!(next.as_str() > "2027-02-28", "{next}");
        assert!(renewal(&[d("2026-08-31", 2)], "II", "2", &next).allowed > 0);
        // Plus de paires que le plafond : la plus ancienne sortie ne
        // suffit pas.
        let over = [d("2026-06-01", 1), d("2026-07-01", 2)];
        let r = renewal(&over, "II", "2", "2026-10-08");
        assert_eq!(r.next.as_deref(), Some("2027-01-01"));
    }

    #[test]
    fn the_classes_are_the_french_ones() {
        assert_eq!(class("II").map(|c| c.mmhg), Some("15,1 à 20 mmHg"));
        assert_eq!(class("III").map(|c| c.hpa), Some("27,1 à 48 hPa"));
        assert!(
            class("2").is_none(),
            "les classes allemandes ne sont pas lues"
        );
    }
}
