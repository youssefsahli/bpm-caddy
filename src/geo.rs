//! **La France sur une carte** : les contours des départements, les
//! communes et leur centre, et la façon de les poser dans un cadre.
//!
//! Ce que la carte des officines (`repertoire`) dessine : une officine se
//! place par sa commune — jamais par sa rue —, et la commune se trouve
//! dans la liste livrée, sans rien demander à personne. Les données sont
//! publiques (IGN Admin Express, INSEE, geo.api.gouv.fr, Licence
//! Ouverte) et décrites dans `assets/geo/SOURCES.md`, avec la façon de
//! les refaire.
//!
//! La métropole est dessinée en projection équirectangulaire corrigée à
//! la latitude moyenne — à l'échelle d'un pays, l'écart avec une vraie
//! projection ne se voit pas ; chaque département d'outre-mer a son
//! encart, à sa propre échelle.
//!
//! Pur, testé : les fichiers sont lus une fois, à la première question.

use std::sync::OnceLock;

/// Un département : son code (`01`…`95`, `2A`, `2B`, `971`…`976`), son
/// nom, et ses contours en (longitude, latitude).
#[derive(Debug)]
pub struct Departement {
    pub code: &'static str,
    pub nom: &'static str,
    pub rings: Vec<Vec<[f32; 2]>>,
}

/// Une commune sous l'un de ses codes postaux.
#[derive(Debug, PartialEq)]
pub struct Commune {
    pub postcode: &'static str,
    pub nom: &'static str,
    pub lon: f32,
    pub lat: f32,
    pub dep: &'static str,
    pub population: u32,
}

/// Les départements, dans l'ordre de leur code.
pub fn departements() -> &'static [Departement] {
    static DEPS: OnceLock<Vec<Departement>> = OnceLock::new();
    DEPS.get_or_init(|| {
        include_str!("../assets/geo/departements.tsv")
            .lines()
            .filter_map(|line| {
                let mut cols = line.split('\t');
                let (code, nom, rings) = (cols.next()?, cols.next()?, cols.next()?);
                let rings = rings
                    .split('|')
                    .map(|ring| {
                        ring.split(';')
                            .filter_map(|p| {
                                let (x, y) = p.split_once(',')?;
                                Some([x.parse().ok()?, y.parse().ok()?])
                            })
                            .collect::<Vec<[f32; 2]>>()
                    })
                    .filter(|r| r.len() >= 3)
                    .collect();
                Some(Departement { code, nom, rings })
            })
            .collect()
    })
}

/// Les communes, par code postal puis par nom.
pub fn communes() -> &'static [Commune] {
    static COMMUNES: OnceLock<Vec<Commune>> = OnceLock::new();
    COMMUNES.get_or_init(|| {
        include_str!("../assets/geo/communes.tsv")
            .lines()
            .filter_map(|line| {
                let mut c = line.split('\t');
                Some(Commune {
                    postcode: c.next()?,
                    nom: c.next()?,
                    lon: c.next()?.parse().ok()?,
                    lat: c.next()?.parse().ok()?,
                    dep: c.next()?,
                    population: c.next()?.parse().ok()?,
                })
            })
            .collect()
    })
}

/// Un nom replié pour la recherche : sans accent ni casse, les traits
/// d'union et les apostrophes comme des espaces — « saint etienne »
/// trouve « Saint-Étienne ».
pub(crate) fn folded(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars().map(crate::fuzzy::fold) {
        let c = if c.is_alphanumeric() { c } else { ' ' };
        if c == ' ' && (out.is_empty() || out.ends_with(' ')) {
            continue;
        }
        out.push(c);
    }
    out.trim_end().to_owned()
}

/// **Les communes qui répondent à ce qu'on tape** : un code postal par
/// son début, un nom par son début puis n'importe où. Les plus peuplées
/// d'abord à rang égal : « Saint-Denis » est d'abord celle de la Réunion
/// et de la Seine-Saint-Denis, pas l'un des vingt hameaux du même nom.
pub fn find_communes(query: &str, most: usize) -> Vec<&'static Commune> {
    let q = folded(query);
    if q.is_empty() {
        return Vec::new();
    }
    let digits = q.chars().all(|c| c.is_ascii_digit());
    let mut hits: Vec<(u8, &'static Commune)> = communes()
        .iter()
        .filter_map(|c| {
            if digits {
                return c.postcode.starts_with(&q).then_some((0, c));
            }
            let name = folded(c.nom);
            if name == q {
                Some((0, c))
            } else if name.starts_with(&q) {
                Some((1, c))
            } else if name.contains(&q) {
                Some((2, c))
            } else {
                None
            }
        })
        .collect();
    hits.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then(b.1.population.cmp(&a.1.population))
            .then(a.1.nom.cmp(b.1.nom))
    });
    hits.into_iter().take(most).map(|(_, c)| c).collect()
}

/// La commune exacte d'un code postal et d'un nom, telle qu'une
/// officine l'a choisie.
pub fn commune(postcode: &str, nom: &str) -> Option<&'static Commune> {
    let postcode = postcode.trim();
    let nom = folded(nom);
    communes()
        .iter()
        .find(|c| c.postcode == postcode && folded(c.nom) == nom)
}

/// Où se dessine un point : la métropole, ou l'encart d'un département
/// d'outre-mer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Zone {
    Metropole,
    OutreMer(&'static str),
}

/// Les départements d'outre-mer, dans l'ordre des encarts.
pub const OUTRE_MER: [&str; 5] = ["971", "972", "973", "974", "976"];

/// La zone d'un département.
pub fn zone_of(dep: &str) -> Zone {
    match OUTRE_MER.iter().find(|d| **d == dep) {
        Some(d) => Zone::OutreMer(d),
        None => Zone::Metropole,
    }
}

/// La latitude où la projection est juste.
fn reference_lat(zone: Zone) -> f32 {
    match zone {
        Zone::Metropole => 46.5,
        Zone::OutreMer(d) => match d {
            "971" => 16.2,
            "972" => 14.6,
            "973" => 4.0,
            "974" => -21.1,
            _ => -12.8,
        },
    }
}

/// Un point projeté dans le plan de sa zone : x vers l'est, y vers le sud.
pub fn project(zone: Zone, lon: f32, lat: f32) -> [f32; 2] {
    let k = reference_lat(zone).to_radians().cos();
    [lon * k, -lat]
}

/// Le rectangle projeté d'une zone, `[x0, y0, x1, y1]` — lu une fois.
pub fn zone_bounds(zone: Zone) -> [f32; 4] {
    static BOUNDS: OnceLock<Vec<(Zone, [f32; 4])>> = OnceLock::new();
    let all = BOUNDS.get_or_init(|| {
        let mut out: Vec<(Zone, [f32; 4])> = Vec::new();
        for d in departements() {
            let zone = zone_of(d.code);
            for p in d.rings.iter().flatten() {
                let [x, y] = project(zone, p[0], p[1]);
                match out.iter_mut().find(|(z, _)| *z == zone) {
                    Some((_, b)) => {
                        b[0] = b[0].min(x);
                        b[1] = b[1].min(y);
                        b[2] = b[2].max(x);
                        b[3] = b[3].max(y);
                    }
                    None => out.push((zone, [x, y, x, y])),
                }
            }
        }
        out
    });
    all.iter()
        .find(|(z, _)| *z == zone)
        .map(|(_, b)| *b)
        .unwrap_or([0.0, 0.0, 1.0, 1.0])
}

/// **Les cadres de la carte** dans un rectangle `w` × `h` (origine en
/// haut à gauche) : la métropole à droite, les cinq encarts d'outre-mer
/// en colonne à sa gauche. Chaque cadre est `[x0, y0, x1, y1]`.
pub fn frames(w: f32, h: f32, gap: f32) -> Vec<(Zone, [f32; 4])> {
    let w = w.max(1.0);
    let h = h.max(1.0);
    // Un écart qui ne mange jamais les cadres d'une carte minuscule.
    let gap = gap.max(0.0).min(w * 0.02).min(h * 0.02);
    let column = (w * 0.18).min(h / 5.0 * 1.4);
    let mut out = vec![(Zone::Metropole, [column + gap, 0.0, w, h])];
    let cell = (h - gap * 4.0) / 5.0;
    for (i, d) in OUTRE_MER.iter().enumerate() {
        let y0 = i as f32 * (cell + gap);
        out.push((Zone::OutreMer(d), [0.0, y0, column, y0 + cell]));
    }
    out
}

/// Un point projeté de `zone`, posé dans `frame` : l'échelle qui fait
/// tenir toute la zone, centrée, sans la déformer.
pub fn place(zone: Zone, frame: [f32; 4], projected: [f32; 2]) -> [f32; 2] {
    let b = zone_bounds(zone);
    let (bw, bh) = ((b[2] - b[0]).max(1e-3), (b[3] - b[1]).max(1e-3));
    let (fw, fh) = (
        (frame[2] - frame[0]).max(1.0),
        (frame[3] - frame[1]).max(1.0),
    );
    let scale = (fw / bw).min(fh / bh);
    let ox = frame[0] + (fw - bw * scale) / 2.0;
    let oy = frame[1] + (fh - bh * scale) / 2.0;
    [
        ox + (projected[0] - b[0]) * scale,
        oy + (projected[1] - b[1]) * scale,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Les cent un départements, chacun avec un contour, et chaque
    /// commune dans un département qui existe.
    #[test]
    fn the_shipped_geography_reads_whole() {
        let deps = departements();
        assert_eq!(deps.len(), 101);
        for d in deps {
            assert!(!d.rings.is_empty(), "{}", d.code);
        }
        let codes: std::collections::HashSet<&str> = deps.iter().map(|d| d.code).collect();
        let communes = communes();
        assert!(communes.len() > 34_000, "{}", communes.len());
        for c in communes {
            assert!(codes.contains(c.dep), "{} {}", c.nom, c.dep);
            assert_eq!(c.postcode.len(), 5, "{}", c.nom);
        }
    }

    /// **Une commune tombe dans sa zone** : son centre projeté est dans le
    /// rectangle de la métropole, ou de son encart.
    #[test]
    fn every_commune_lands_inside_its_zone() {
        for c in communes() {
            let zone = zone_of(c.dep);
            let [x, y] = project(zone, c.lon, c.lat);
            let b = zone_bounds(zone);
            let slack = 0.05;
            assert!(
                x >= b[0] - slack && x <= b[2] + slack && y >= b[1] - slack && y <= b[3] + slack,
                "{} ({}) hors de sa zone",
                c.nom,
                c.dep
            );
        }
    }

    #[test]
    fn a_town_is_found_by_its_name_or_its_postcode() {
        let hit = find_communes("epinal", 5);
        assert_eq!(hit[0].nom, "Épinal");
        assert_eq!(hit[0].postcode, "88000");
        let hit = find_communes("saint etienne", 3);
        assert_eq!(hit[0].nom, "Saint-Étienne", "la plus peuplée d'abord");
        let hit = find_communes("750", 40);
        assert!(hit.iter().all(|c| c.postcode.starts_with("750")));
        assert!(hit.iter().any(|c| c.nom == "Paris"));
        assert!(find_communes("  ", 5).is_empty());
        assert_eq!(commune("88000", "epinal").map(|c| c.dep), Some("88"));
        assert_eq!(commune("97400", "Saint-Denis").map(|c| c.dep), Some("974"));
        assert!(commune("88000", "Paris").is_none());
    }

    /// **Les cadres tiennent dans la carte et ne se chevauchent pas**, et
    /// un point d'une zone se pose dans son cadre.
    #[test]
    fn the_frames_tile_the_map_and_points_land_in_theirs() {
        for (w, h) in [(800.0, 600.0), (300.0, 900.0), (1600.0, 300.0), (1.0, 1.0)] {
            let frames = frames(w, h, 4.0);
            assert_eq!(frames.len(), 6);
            for (i, (_, a)) in frames.iter().enumerate() {
                assert!(
                    a[0] >= 0.0 && a[1] >= 0.0 && a[2] <= w.max(1.0) + 0.01,
                    "{a:?}"
                );
                assert!(a[3] <= h.max(1.0) + 0.01, "{w}x{h} {a:?}");
                for (_, b) in &frames[i + 1..] {
                    let apart = a[2] <= b[0] || b[2] <= a[0] || a[3] <= b[1] || b[3] <= a[1];
                    assert!(apart, "{a:?} {b:?}");
                }
            }
            for c in [commune("88000", "Épinal"), commune("97400", "Saint-Denis")]
                .into_iter()
                .flatten()
            {
                let zone = zone_of(c.dep);
                let frame = frames.iter().find(|(z, _)| *z == zone).unwrap().1;
                let [x, y] = place(zone, frame, project(zone, c.lon, c.lat));
                assert!(x >= frame[0] - 0.5 && x <= frame[2] + 0.5, "{} {x}", c.nom);
                assert!(y >= frame[1] - 0.5 && y <= frame[3] + 0.5, "{} {y}", c.nom);
            }
        }
    }
}
