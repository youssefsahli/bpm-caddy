//! Vaccination reference data: the French calendrier vaccinal and the
//! traveller's country groups.
//!
//! Two tables, both static and both *indicative*: the calendar rules
//! that say what an adult owes today, and a country list carrying the
//! travel recommendations of the BEH « Recommandations sanitaires pour
//! les voyageurs ». Neither replaces the source — every view that shows
//! them prints the year's BEH as the authority — but both turn a
//! question the counter asks twenty times a day into one glance.
//!
//! The country list also carries the tile the country occupies on the
//! schematic world map: countries are grouped into regions, and each
//! region is a block of tiles placed roughly where it belongs. It is a
//! cartogram, not a projection — every country gets the same square,
//! which is exactly what a reference table wants.

/// A travel vaccine (or an antipaludique) recommended for a country.
///
/// Stored per country as a bitmask, so a row is one `u16`.
pub mod reco {
    pub const HEP_A: u16 = 1 << 0;
    pub const HEP_B: u16 = 1 << 1;
    pub const TYPHOIDE: u16 = 1 << 2;
    pub const RAGE: u16 = 1 << 3;
    pub const MENINGO: u16 = 1 << 4;
    pub const ENCEPH_JAP: u16 = 1 << 5;
    pub const ENCEPH_TIQUES: u16 = 1 << 6;
    pub const CHOLERA: u16 = 1 << 7;
    pub const POLIO: u16 = 1 << 8;
}

use reco::*;

/// One recommendation flag, with the label and the carnet code it maps
/// to — that mapping is what lets the travel panel tick a line off
/// against the doses already in the patient's carnet.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Reco {
    pub bit: u16,
    /// Catalogue code of the vaccine that answers it.
    pub code: &'static str,
    pub label: &'static str,
    pub detail: &'static str,
}

/// Every flag, in the order the travel panel lists them.
pub const RECOS: &[Reco] = &[
    Reco {
        bit: HEP_A,
        code: "HEPA",
        label: "Hépatite A",
        detail: "Séjour en zone d'endémie, quelles que soient les conditions.",
    },
    Reco {
        bit: HEP_B,
        code: "HEPB",
        label: "Hépatite B",
        detail: "Séjour long ou répété, soins possibles sur place, conduites à risque.",
    },
    Reco {
        bit: TYPHOIDE,
        code: "TYPH",
        label: "Typhoïde",
        detail: "Séjour prolongé ou dans de mauvaises conditions d'hygiène.",
    },
    Reco {
        bit: RAGE,
        code: "RAGE",
        label: "Rage",
        detail: "Séjour prolongé ou isolé, jeunes enfants, contact animalier.",
    },
    Reco {
        bit: MENINGO,
        code: "MENACYW",
        label: "Méningocoque ACYW",
        detail: "Ceinture méningitique en saison sèche ; exigé pour le pèlerinage.",
    },
    Reco {
        bit: ENCEPH_JAP,
        code: "EJ",
        label: "Encéphalite japonaise",
        detail: "Séjour rural en saison de transmission.",
    },
    Reco {
        bit: ENCEPH_TIQUES,
        code: "ET",
        label: "Encéphalite à tiques",
        detail: "Randonnée ou camping en zone forestière, du printemps à l'automne.",
    },
    Reco {
        bit: CHOLERA,
        code: "CHOL",
        label: "Choléra",
        detail: "Réservé aux personnels intervenant en situation d'épidémie.",
    },
    Reco {
        bit: POLIO,
        code: "DTP",
        label: "Poliomyélite (dose supplémentaire)",
        detail: "Dose de rappel 4 semaines à 12 mois avant le départ (RSI).",
    },
];

/// What the country asks of a traveller for yellow fever.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Yf {
    /// No requirement and no recommendation.
    No,
    /// Endemic: recommended, not demanded at the border.
    Recommended,
    /// Demanded only of travellers coming from an endemic country.
    RequiredFromEndemic,
    /// Demanded of every traveller from 9 months (or 1 year) of age.
    Required,
}

impl Yf {
    pub fn label(self) -> &'static str {
        match self {
            Self::No => "Non concernée",
            Self::Recommended => "Recommandée (zone d'endémie)",
            Self::RequiredFromEndemic => "Exigée si provenance d'un pays endémique",
            Self::Required => "Exigée pour tout voyageur",
        }
    }

    /// Does the traveller need the dose in practice?
    pub fn needed(self) -> bool {
        matches!(self, Self::Recommended | Self::Required)
    }
}

/// Malaria risk, the level that decides the chemoprophylaxis question.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Palu {
    No,
    /// Foyers limités, saisonniers ou d'altitude basse seulement.
    Limited,
    Present,
    High,
}

impl Palu {
    pub fn label(self) -> &'static str {
        match self {
            Self::No => "Absent",
            Self::Limited => "Foyers limités",
            Self::Present => "Présent",
            Self::High => "Élevé toute l'année",
        }
    }
}

/// The country groups the map is drawn in. Each is a block of tiles at
/// a fixed place on the grid; the countries of the group fill it in
/// reading order.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Region {
    AmeriqueNord,
    AmeriqueCentrale,
    Caraibes,
    AmeriqueSud,
    EuropeOuest,
    EuropeEst,
    AfriqueNord,
    AfriqueOuest,
    AfriqueCentrale,
    AfriqueEst,
    AfriqueAustrale,
    MoyenOrient,
    AsieCentrale,
    AsieSud,
    AsieEst,
    AsieSudEst,
    Oceanie,
}

impl Region {
    pub const ALL: [Region; 17] = [
        Self::AmeriqueNord,
        Self::AmeriqueCentrale,
        Self::Caraibes,
        Self::AmeriqueSud,
        Self::EuropeOuest,
        Self::EuropeEst,
        Self::AfriqueNord,
        Self::AfriqueOuest,
        Self::AfriqueCentrale,
        Self::AfriqueEst,
        Self::AfriqueAustrale,
        Self::MoyenOrient,
        Self::AsieCentrale,
        Self::AsieSud,
        Self::AsieEst,
        Self::AsieSudEst,
        Self::Oceanie,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::AmeriqueNord => "Amérique du Nord",
            Self::AmeriqueCentrale => "Amérique centrale",
            Self::Caraibes => "Caraïbes",
            Self::AmeriqueSud => "Amérique du Sud",
            Self::EuropeOuest => "Europe de l'Ouest",
            Self::EuropeEst => "Europe de l'Est et Caucase",
            Self::AfriqueNord => "Afrique du Nord",
            Self::AfriqueOuest => "Afrique de l'Ouest",
            Self::AfriqueCentrale => "Afrique centrale",
            Self::AfriqueEst => "Afrique de l'Est",
            Self::AfriqueAustrale => "Afrique australe",
            Self::MoyenOrient => "Moyen-Orient",
            Self::AsieCentrale => "Asie centrale",
            Self::AsieSud => "Asie du Sud",
            Self::AsieEst => "Asie de l'Est",
            Self::AsieSudEst => "Asie du Sud-Est",
            Self::Oceanie => "Océanie",
        }
    }

    /// Where the group's block starts on the tile grid, and how many
    /// tiles wide it is: `(col, row, width)`.
    pub fn block(self) -> (i32, i32, i32) {
        match self {
            Self::AmeriqueNord => (0, 0, 3),
            Self::EuropeOuest => (7, 0, 6),
            Self::EuropeEst => (14, 0, 5),
            Self::AsieCentrale => (20, 0, 3),
            Self::AsieEst => (24, 0, 3),
            Self::AmeriqueCentrale => (0, 2, 4),
            Self::AsieSudEst => (24, 3, 3),
            Self::AsieSud => (20, 4, 3),
            Self::Caraibes => (0, 5, 5),
            Self::AfriqueNord => (7, 5, 3),
            Self::MoyenOrient => (11, 5, 4),
            Self::AfriqueOuest => (7, 8, 4),
            Self::AfriqueEst => (16, 8, 4),
            Self::Oceanie => (22, 8, 4),
            Self::AfriqueCentrale => (12, 9, 3),
            Self::AmeriqueSud => (1, 10, 4),
            Self::AfriqueAustrale => (14, 13, 3),
        }
    }
}

/// One country of the reference table.
pub struct Country {
    /// ISO 3166-1 alpha-2, the label drawn on the tile.
    pub code: &'static str,
    pub name: &'static str,
    pub region: Region,
    pub yf: Yf,
    pub palu: Palu,
    /// Bitmask of [`reco`] flags.
    pub reco: u16,
}

impl Country {
    /// The recommendation flags this country carries, as rows.
    pub fn recos(&self) -> impl Iterator<Item = &'static Reco> + '_ {
        RECOS.iter().filter(move |r| self.reco & r.bit != 0)
    }

    /// Its tile on the schematic map, `(col, row)`.
    pub fn tile(&self) -> (i32, i32) {
        let (col, row, width) = self.region.block();
        let i = COUNTRIES
            .iter()
            .filter(|c| c.region == self.region)
            .position(|c| c.code == self.code)
            .unwrap_or(0) as i32;
        (col + i % width, row + i / width)
    }
}

/// Look a country up by its ISO code.
pub fn country(code: &str) -> Option<&'static Country> {
    COUNTRIES.iter().find(|c| c.code == code)
}

/// Countries whose French name matches `query`, accent- and
/// case-insensitively; the ISO code matches too ("th" → Thaïlande).
pub fn search(query: &str) -> Vec<&'static Country> {
    let q = fold(query);
    if q.is_empty() {
        return Vec::new();
    }
    let mut out: Vec<(u8, &'static Country)> = COUNTRIES
        .iter()
        .filter_map(|c| {
            let name = fold(c.name);
            if name.starts_with(&q) {
                Some((0, c))
            } else if fold(c.code) == q {
                Some((1, c))
            } else if name.contains(&q) {
                Some((2, c))
            } else {
                None
            }
        })
        .collect();
    out.sort_by_key(|(rank, c)| (*rank, c.name));
    out.into_iter().map(|(_, c)| c).collect()
}

/// Lowercase and strip the accents a name may be typed without.
fn fold(s: &str) -> String {
    s.trim()
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'à' | 'â' | 'ä' | 'á' | 'ã' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' | 'í' => 'i',
            'ô' | 'ö' | 'ó' | 'õ' => 'o',
            'û' | 'ü' | 'ù' | 'ú' => 'u',
            'ç' => 'c',
            'ñ' => 'n',
            other => other,
        })
        .collect()
}

/// The vaccines the carnet offers by name, so a line typed at the
/// counter carries a code the calendar and the travel panel can match.
pub struct VaccineRef {
    pub code: &'static str,
    pub label: &'static str,
    /// What the line usually records, shown as the dose hint.
    pub schedule: &'static str,
}

pub const CATALOGUE: &[VaccineRef] = &[
    VaccineRef {
        code: "DTP",
        label: "dTP — diphtérie, tétanos, poliomyélite",
        schedule: "Rappels à 25, 45 et 65 ans, puis tous les 10 ans",
    },
    VaccineRef {
        code: "DTCAP",
        label: "dTcaP — avec coqueluche",
        schedule: "Rappel de 25 ans, cocooning, grossesse (20-36 SA)",
    },
    VaccineRef {
        code: "GRIPPE",
        label: "Grippe saisonnière",
        schedule: "Chaque automne",
    },
    VaccineRef {
        code: "COVID",
        label: "COVID-19",
        schedule: "Campagne annuelle",
    },
    VaccineRef {
        code: "PNEUMO",
        label: "Pneumocoque",
        schedule: "Selon les facteurs de risque et les doses déjà reçues",
    },
    VaccineRef {
        code: "ZONA",
        label: "Zona (Shingrix)",
        schedule: "2 doses ; à partir de 65 ans",
    },
    VaccineRef {
        code: "VRS",
        label: "VRS (virus respiratoire syncytial)",
        schedule: "Dose unique à partir de 75 ans",
    },
    VaccineRef {
        code: "ROR",
        label: "ROR — rougeole, oreillons, rubéole",
        schedule: "2 doses au total pour les personnes nées depuis 1980",
    },
    VaccineRef {
        code: "HPV",
        label: "Papillomavirus (HPV)",
        schedule: "11-14 ans, rattrapage jusqu'à 19 ans",
    },
    VaccineRef {
        code: "MENACYW",
        label: "Méningocoque ACYW",
        schedule: "Calendrier du nourrisson ; voyage, pèlerinage",
    },
    VaccineRef {
        code: "MENB",
        label: "Méningocoque B",
        schedule: "Calendrier du nourrisson ; rattrapage",
    },
    VaccineRef {
        code: "HEPB",
        label: "Hépatite B",
        schedule: "3 doses",
    },
    VaccineRef {
        code: "HEPA",
        label: "Hépatite A",
        schedule: "1 dose, rappel 6 à 12 mois plus tard",
    },
    VaccineRef {
        code: "FJ",
        label: "Fièvre jaune (amaril)",
        schedule: "1 dose, valable à vie ; centre agréé uniquement",
    },
    VaccineRef {
        code: "TYPH",
        label: "Typhoïde",
        schedule: "1 dose, 15 jours avant le départ, valable 3 ans",
    },
    VaccineRef {
        code: "RAGE",
        label: "Rage (préventive)",
        schedule: "3 doses avant le départ",
    },
    VaccineRef {
        code: "EJ",
        label: "Encéphalite japonaise",
        schedule: "2 doses à 28 jours d'intervalle",
    },
    VaccineRef {
        code: "ET",
        label: "Encéphalite à tiques",
        schedule: "3 doses puis rappels",
    },
    VaccineRef {
        code: "CHOL",
        label: "Choléra",
        schedule: "2 doses orales",
    },
    VaccineRef {
        code: "BCG",
        label: "BCG — tuberculose",
        schedule: "Nourrissons à risque",
    },
    VaccineRef {
        code: "VARICELLE",
        label: "Varicelle",
        schedule: "2 doses ; contre-indiqué pendant la grossesse",
    },
];

// ---------------------------------------------------------------------
// Le calendrier vaccinal
// ---------------------------------------------------------------------

/// How firmly the calendar asks for a line.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DueLevel {
    /// The carnet already carries it.
    Ok,
    /// Owed now, and the carnet has nothing.
    Due,
    /// Owed only on a ground the app cannot know (risk factors,
    /// pregnancy, immunosuppression): a question to ask, not a verdict.
    Ask,
}

/// One line of the "what does this patient owe" panel.
pub struct DueLine {
    pub code: &'static str,
    pub label: &'static str,
    pub level: DueLevel,
    pub detail: String,
}

/// A dose already recorded, reduced to what the calendar needs.
pub struct Dose<'a> {
    pub code: &'a str,
    /// ISO `YYYY-MM-DD`, possibly empty.
    pub date: &'a str,
}

/// Read the calendar against a patient's carnet.
///
/// `age` is in years, `birth_year` decides the ROR cohort, and `today`
/// is ISO so the whole thing stays pure and testable — no clock inside.
pub fn due_lines(
    age: Option<u32>,
    birth_year: Option<u32>,
    today: &str,
    doses: &[Dose],
) -> Vec<DueLine> {
    let year: u32 = today.get(..4).and_then(|y| y.parse().ok()).unwrap_or(0);
    let last = |code: &str| -> Option<&str> {
        doses
            .iter()
            .filter(|d| d.code == code && !d.date.is_empty())
            .map(|d| d.date)
            .max()
    };
    let count = |code: &str| doses.iter().filter(|d| d.code == code).count();
    // Years elapsed since a dose, by ISO year difference — good enough
    // for a ten-year booster, and it never needs a calendar library.
    let years_since = |iso: &str| -> Option<u32> {
        let y: u32 = iso.get(..4)?.parse().ok()?;
        Some(year.saturating_sub(y))
    };
    let mut out = Vec::new();

    // --- dTP: 25, 45, 65, then every ten years ---
    //
    // The clock is the *milestone*, not a flat ten years: someone
    // boosted at 25 owes nothing until 45, and reading their dose as
    // "eleven years old, therefore overdue" would send them back for a
    // needle the calendar does not ask for.
    if let Some(age) = age {
        // dTcaP counts as a dTP dose for the booster clock.
        let dtp = [last("DTP"), last("DTCAP")].into_iter().flatten().max();
        let year_of = |iso: &str| iso[..4.min(iso.len())].to_owned();
        let (level, detail) = match (dtp, dtp_milestone(age)) {
            // Under 25: the adult schedule has not started.
            (_, None) => (
                DueLevel::Ask,
                "Suivre le calendrier de l'enfant et de l'adolescent.".to_owned(),
            ),
            (None, Some(m)) => (
                DueLevel::Due,
                format!("Aucun rappel au carnet ; le rappel de {m} ans est attendu."),
            ),
            (Some(date), Some(m)) => {
                let age_at_dose = age.saturating_sub(years_since(date).unwrap_or(0));
                if age_at_dose < m {
                    (
                        DueLevel::Due,
                        format!(
                            "Dernier rappel en {} (à {age_at_dose} ans) ; le rappel de {m} ans est attendu.",
                            year_of(date)
                        ),
                    )
                } else {
                    (
                        DueLevel::Ok,
                        format!(
                            "Rappel de {} ; prochain vers {} ans.",
                            year_of(date),
                            dtp_next_after(m)
                        ),
                    )
                }
            }
        };
        out.push(DueLine {
            code: "DTP",
            label: "dTP — rappel décennal",
            level,
            detail,
        });
    }

    // --- Grippe: one dose per campaign, from 65 (or on risk) ---
    {
        let season = flu_season_start(today);
        let done = last("GRIPPE").is_some_and(|d| d >= season.as_str());
        let level = if done {
            DueLevel::Ok
        } else if age.is_some_and(|a| a >= 65) {
            DueLevel::Due
        } else {
            DueLevel::Ask
        };
        out.push(DueLine {
            code: "GRIPPE",
            label: "Grippe saisonnière",
            level,
            detail: if done {
                "Dose de la campagne en cours enregistrée.".to_owned()
            } else if age.is_some_and(|a| a >= 65) {
                "Campagne en cours, aucune dose enregistrée.".to_owned()
            } else {
                "Selon les facteurs de risque et l'entourage.".to_owned()
            },
        });
    }

    // --- COVID-19: annual campaign, same reading ---
    {
        let season = flu_season_start(today);
        let done = last("COVID").is_some_and(|d| d >= season.as_str());
        out.push(DueLine {
            code: "COVID",
            label: "COVID-19",
            level: if done {
                DueLevel::Ok
            } else if age.is_some_and(|a| a >= 65) {
                DueLevel::Due
            } else {
                DueLevel::Ask
            },
            detail: if done {
                "Dose de la campagne en cours enregistrée.".to_owned()
            } else {
                "Campagne annuelle : 65 ans et plus, immunodéprimés, entourage.".to_owned()
            },
        });
    }

    // --- Zona: two doses from 65 ---
    if age.is_some_and(|a| a >= 65) {
        let n = count("ZONA");
        out.push(DueLine {
            code: "ZONA",
            label: "Zona (Shingrix)",
            level: if n >= 2 { DueLevel::Ok } else { DueLevel::Due },
            detail: match n {
                0 => "2 doses attendues à partir de 65 ans.".to_owned(),
                1 => "1re dose faite ; 2e dose de 2 à 6 mois plus tard.".to_owned(),
                _ => "Schéma complet.".to_owned(),
            },
        });
    }

    // --- VRS: one dose from 75 ---
    if age.is_some_and(|a| a >= 75) {
        let done = count("VRS") >= 1;
        out.push(DueLine {
            code: "VRS",
            label: "VRS",
            level: if done { DueLevel::Ok } else { DueLevel::Due },
            detail: if done {
                "Dose enregistrée.".to_owned()
            } else {
                "Dose unique recommandée à partir de 75 ans.".to_owned()
            },
        });
    }

    // --- Pneumocoque: on risk, so always a question ---
    if age.is_some_and(|a| a >= 65) {
        let n = count("PNEUMO");
        out.push(DueLine {
            code: "PNEUMO",
            label: "Pneumocoque",
            level: if n >= 1 { DueLevel::Ok } else { DueLevel::Ask },
            detail: if n >= 1 {
                "Dose enregistrée ; vérifier le schéma selon les vaccins déjà reçus.".to_owned()
            } else {
                "Recommandé sur facteurs de risque ; schéma selon les doses antérieures.".to_owned()
            },
        });
    }

    // --- ROR: two doses for everyone born from 1980 ---
    if birth_year.is_some_and(|y| y >= 1980) && age.is_some_and(|a| a >= 2) {
        let n = count("ROR");
        out.push(DueLine {
            code: "ROR",
            label: "ROR",
            level: if n >= 2 { DueLevel::Ok } else { DueLevel::Due },
            detail: format!("{n} dose(s) au carnet ; 2 doses au total attendues."),
        });
    }

    // --- HPV: the 11-19 window ---
    if age.is_some_and(|a| (11..=19).contains(&a)) {
        let n = count("HPV");
        out.push(DueLine {
            code: "HPV",
            label: "Papillomavirus",
            level: if n >= 2 { DueLevel::Ok } else { DueLevel::Due },
            detail: format!("{n} dose(s) ; 2 doses avant 15 ans, 3 doses ensuite."),
        });
    }

    out
}

/// The dTP booster this age is standing on: the latest milestone the
/// patient has reached. `None` before 25, where the adult schedule has
/// not started.
fn dtp_milestone(age: u32) -> Option<u32> {
    match age {
        0..=24 => None,
        25..=44 => Some(25),
        45..=64 => Some(45),
        // From 65 on, every ten years: 65, 75, 85, …
        _ => Some(65 + (age - 65) / 10 * 10),
    }
}

/// The milestone that follows `m`.
fn dtp_next_after(m: u32) -> u32 {
    match m {
        0..=24 => 25,
        25..=44 => 45,
        45..=64 => 65,
        _ => m + 10,
    }
}

/// The first day of the vaccination campaign `today` falls in: doses
/// are counted from the 1st of September before it.
fn flu_season_start(today: &str) -> String {
    let year: u32 = today.get(..4).and_then(|y| y.parse().ok()).unwrap_or(0);
    let month: u32 = today.get(5..7).and_then(|m| m.parse().ok()).unwrap_or(1);
    let start = if month >= 9 {
        year
    } else {
        year.saturating_sub(1)
    };
    format!("{start:04}-09-01")
}

// ---------------------------------------------------------------------
// Les pays
// ---------------------------------------------------------------------

/// Shorthand so the table below stays a table.
const fn c(
    code: &'static str,
    name: &'static str,
    region: Region,
    yf: Yf,
    palu: Palu,
    reco: u16,
) -> Country {
    Country {
        code,
        name,
        region,
        yf,
        palu,
        reco,
    }
}

use Region::*;

/// The reference table. Order matters twice: it is the order the tiles
/// are laid out in inside a region's block, and the order the country
/// list is shown in.
pub const COUNTRIES: &[Country] = &[
    // --- Amérique du Nord ---
    c("GL", "Groenland", AmeriqueNord, Yf::No, Palu::No, 0),
    c("CA", "Canada", AmeriqueNord, Yf::No, Palu::No, 0),
    c("US", "États-Unis", AmeriqueNord, Yf::No, Palu::No, 0),
    // --- Europe de l'Ouest ---
    c("IS", "Islande", EuropeOuest, Yf::No, Palu::No, 0),
    c("NO", "Norvège", EuropeOuest, Yf::No, Palu::No, 0),
    c("SE", "Suède", EuropeOuest, Yf::No, Palu::No, ENCEPH_TIQUES),
    c(
        "FI",
        "Finlande",
        EuropeOuest,
        Yf::No,
        Palu::No,
        ENCEPH_TIQUES,
    ),
    c("DK", "Danemark", EuropeOuest, Yf::No, Palu::No, 0),
    c("IE", "Irlande", EuropeOuest, Yf::No, Palu::No, 0),
    c("GB", "Royaume-Uni", EuropeOuest, Yf::No, Palu::No, 0),
    c("NL", "Pays-Bas", EuropeOuest, Yf::No, Palu::No, 0),
    c("BE", "Belgique", EuropeOuest, Yf::No, Palu::No, 0),
    c("LU", "Luxembourg", EuropeOuest, Yf::No, Palu::No, 0),
    c(
        "DE",
        "Allemagne",
        EuropeOuest,
        Yf::No,
        Palu::No,
        ENCEPH_TIQUES,
    ),
    c("CH", "Suisse", EuropeOuest, Yf::No, Palu::No, ENCEPH_TIQUES),
    c("FR", "France", EuropeOuest, Yf::No, Palu::No, 0),
    c("AD", "Andorre", EuropeOuest, Yf::No, Palu::No, 0),
    c("MC", "Monaco", EuropeOuest, Yf::No, Palu::No, 0),
    c("PT", "Portugal", EuropeOuest, Yf::No, Palu::No, 0),
    c("ES", "Espagne", EuropeOuest, Yf::No, Palu::No, 0),
    c("IT", "Italie", EuropeOuest, Yf::No, Palu::No, 0),
    c(
        "AT",
        "Autriche",
        EuropeOuest,
        Yf::No,
        Palu::No,
        ENCEPH_TIQUES,
    ),
    c(
        "LI",
        "Liechtenstein",
        EuropeOuest,
        Yf::No,
        Palu::No,
        ENCEPH_TIQUES,
    ),
    c("SM", "Saint-Marin", EuropeOuest, Yf::No, Palu::No, 0),
    c("MT", "Malte", EuropeOuest, Yf::No, Palu::No, 0),
    c("GR", "Grèce", EuropeOuest, Yf::No, Palu::No, HEP_A),
    c("CY", "Chypre", EuropeOuest, Yf::No, Palu::No, HEP_A),
    // --- Europe de l'Est et Caucase ---
    c(
        "EE",
        "Estonie",
        EuropeEst,
        Yf::No,
        Palu::No,
        HEP_A | ENCEPH_TIQUES,
    ),
    c(
        "LV",
        "Lettonie",
        EuropeEst,
        Yf::No,
        Palu::No,
        HEP_A | ENCEPH_TIQUES,
    ),
    c(
        "LT",
        "Lituanie",
        EuropeEst,
        Yf::No,
        Palu::No,
        HEP_A | ENCEPH_TIQUES,
    ),
    c(
        "BY",
        "Biélorussie",
        EuropeEst,
        Yf::No,
        Palu::No,
        HEP_A | ENCEPH_TIQUES,
    ),
    c(
        "RU",
        "Russie",
        EuropeEst,
        Yf::No,
        Palu::No,
        HEP_A | HEP_B | ENCEPH_TIQUES | RAGE,
    ),
    c(
        "PL",
        "Pologne",
        EuropeEst,
        Yf::No,
        Palu::No,
        HEP_A | ENCEPH_TIQUES,
    ),
    c(
        "CZ",
        "Tchéquie",
        EuropeEst,
        Yf::No,
        Palu::No,
        HEP_A | ENCEPH_TIQUES,
    ),
    c(
        "SK",
        "Slovaquie",
        EuropeEst,
        Yf::No,
        Palu::No,
        HEP_A | ENCEPH_TIQUES,
    ),
    c(
        "HU",
        "Hongrie",
        EuropeEst,
        Yf::No,
        Palu::No,
        HEP_A | ENCEPH_TIQUES,
    ),
    c(
        "SI",
        "Slovénie",
        EuropeEst,
        Yf::No,
        Palu::No,
        HEP_A | ENCEPH_TIQUES,
    ),
    c(
        "HR",
        "Croatie",
        EuropeEst,
        Yf::No,
        Palu::No,
        HEP_A | ENCEPH_TIQUES,
    ),
    c(
        "BA",
        "Bosnie-Herzégovine",
        EuropeEst,
        Yf::No,
        Palu::No,
        HEP_A | ENCEPH_TIQUES,
    ),
    c(
        "RS",
        "Serbie",
        EuropeEst,
        Yf::No,
        Palu::No,
        HEP_A | ENCEPH_TIQUES,
    ),
    c("XK", "Kosovo", EuropeEst, Yf::No, Palu::No, HEP_A),
    c("ME", "Monténégro", EuropeEst, Yf::No, Palu::No, HEP_A),
    c(
        "MK",
        "Macédoine du Nord",
        EuropeEst,
        Yf::No,
        Palu::No,
        HEP_A,
    ),
    c("AL", "Albanie", EuropeEst, Yf::No, Palu::No, HEP_A | HEP_B),
    c(
        "BG",
        "Bulgarie",
        EuropeEst,
        Yf::No,
        Palu::No,
        HEP_A | ENCEPH_TIQUES,
    ),
    c(
        "RO",
        "Roumanie",
        EuropeEst,
        Yf::No,
        Palu::No,
        HEP_A | ENCEPH_TIQUES,
    ),
    c(
        "MD",
        "Moldavie",
        EuropeEst,
        Yf::No,
        Palu::No,
        HEP_A | HEP_B | ENCEPH_TIQUES,
    ),
    c(
        "UA",
        "Ukraine",
        EuropeEst,
        Yf::No,
        Palu::No,
        HEP_A | HEP_B | ENCEPH_TIQUES | RAGE,
    ),
    c(
        "TR",
        "Turquie",
        EuropeEst,
        Yf::No,
        Palu::No,
        HEP_A | HEP_B | TYPHOIDE | RAGE,
    ),
    c(
        "GE",
        "Géorgie",
        EuropeEst,
        Yf::No,
        Palu::No,
        HEP_A | HEP_B | TYPHOIDE | RAGE,
    ),
    c(
        "AM",
        "Arménie",
        EuropeEst,
        Yf::No,
        Palu::No,
        HEP_A | HEP_B | TYPHOIDE | RAGE,
    ),
    c(
        "AZ",
        "Azerbaïdjan",
        EuropeEst,
        Yf::No,
        Palu::No,
        HEP_A | HEP_B | TYPHOIDE | RAGE,
    ),
    // --- Afrique du Nord ---
    c(
        "MA",
        "Maroc",
        AfriqueNord,
        Yf::No,
        Palu::No,
        HEP_A | TYPHOIDE | RAGE,
    ),
    c(
        "DZ",
        "Algérie",
        AfriqueNord,
        Yf::No,
        Palu::No,
        HEP_A | TYPHOIDE | RAGE,
    ),
    c(
        "TN",
        "Tunisie",
        AfriqueNord,
        Yf::No,
        Palu::No,
        HEP_A | TYPHOIDE | RAGE,
    ),
    c(
        "LY",
        "Libye",
        AfriqueNord,
        Yf::RequiredFromEndemic,
        Palu::No,
        HEP_A | HEP_B | TYPHOIDE | RAGE,
    ),
    c(
        "EG",
        "Égypte",
        AfriqueNord,
        Yf::RequiredFromEndemic,
        Palu::No,
        HEP_A | HEP_B | TYPHOIDE | RAGE,
    ),
    // --- Afrique de l'Ouest ---
    c(
        "MR",
        "Mauritanie",
        AfriqueOuest,
        Yf::RequiredFromEndemic,
        Palu::Present,
        HEP_A | HEP_B | TYPHOIDE | RAGE | MENINGO,
    ),
    c(
        "SN",
        "Sénégal",
        AfriqueOuest,
        Yf::Required,
        Palu::Present,
        HEP_A | HEP_B | TYPHOIDE | RAGE | MENINGO,
    ),
    c(
        "GM",
        "Gambie",
        AfriqueOuest,
        Yf::RequiredFromEndemic,
        Palu::High,
        HEP_A | HEP_B | TYPHOIDE | RAGE | MENINGO,
    ),
    c(
        "GW",
        "Guinée-Bissau",
        AfriqueOuest,
        Yf::Required,
        Palu::High,
        HEP_A | HEP_B | TYPHOIDE | RAGE | MENINGO,
    ),
    c(
        "GN",
        "Guinée",
        AfriqueOuest,
        Yf::Required,
        Palu::High,
        HEP_A | HEP_B | TYPHOIDE | RAGE | MENINGO,
    ),
    c(
        "SL",
        "Sierra Leone",
        AfriqueOuest,
        Yf::Required,
        Palu::High,
        HEP_A | HEP_B | TYPHOIDE | RAGE | MENINGO,
    ),
    c(
        "LR",
        "Liberia",
        AfriqueOuest,
        Yf::Required,
        Palu::High,
        HEP_A | HEP_B | TYPHOIDE | RAGE | MENINGO,
    ),
    c(
        "CI",
        "Côte d'Ivoire",
        AfriqueOuest,
        Yf::Required,
        Palu::High,
        HEP_A | HEP_B | TYPHOIDE | RAGE | MENINGO,
    ),
    c(
        "GH",
        "Ghana",
        AfriqueOuest,
        Yf::Required,
        Palu::High,
        HEP_A | HEP_B | TYPHOIDE | RAGE | MENINGO,
    ),
    c(
        "TG",
        "Togo",
        AfriqueOuest,
        Yf::Required,
        Palu::High,
        HEP_A | HEP_B | TYPHOIDE | RAGE | MENINGO,
    ),
    c(
        "BJ",
        "Bénin",
        AfriqueOuest,
        Yf::Required,
        Palu::High,
        HEP_A | HEP_B | TYPHOIDE | RAGE | MENINGO,
    ),
    c(
        "NG",
        "Nigeria",
        AfriqueOuest,
        Yf::Required,
        Palu::High,
        HEP_A | HEP_B | TYPHOIDE | RAGE | MENINGO | POLIO,
    ),
    c(
        "NE",
        "Niger",
        AfriqueOuest,
        Yf::Required,
        Palu::High,
        HEP_A | HEP_B | TYPHOIDE | RAGE | MENINGO,
    ),
    c(
        "BF",
        "Burkina Faso",
        AfriqueOuest,
        Yf::Required,
        Palu::High,
        HEP_A | HEP_B | TYPHOIDE | RAGE | MENINGO,
    ),
    c(
        "ML",
        "Mali",
        AfriqueOuest,
        Yf::Required,
        Palu::High,
        HEP_A | HEP_B | TYPHOIDE | RAGE | MENINGO,
    ),
    c(
        "CV",
        "Cap-Vert",
        AfriqueOuest,
        Yf::RequiredFromEndemic,
        Palu::Limited,
        HEP_A | HEP_B | TYPHOIDE,
    ),
    // --- Afrique centrale ---
    c(
        "TD",
        "Tchad",
        AfriqueCentrale,
        Yf::Required,
        Palu::High,
        HEP_A | HEP_B | TYPHOIDE | RAGE | MENINGO,
    ),
    c(
        "CM",
        "Cameroun",
        AfriqueCentrale,
        Yf::Required,
        Palu::High,
        HEP_A | HEP_B | TYPHOIDE | RAGE | MENINGO,
    ),
    c(
        "CF",
        "Centrafrique",
        AfriqueCentrale,
        Yf::Required,
        Palu::High,
        HEP_A | HEP_B | TYPHOIDE | RAGE | MENINGO,
    ),
    c(
        "GQ",
        "Guinée équatoriale",
        AfriqueCentrale,
        Yf::Required,
        Palu::High,
        HEP_A | HEP_B | TYPHOIDE | RAGE,
    ),
    c(
        "GA",
        "Gabon",
        AfriqueCentrale,
        Yf::Required,
        Palu::High,
        HEP_A | HEP_B | TYPHOIDE | RAGE,
    ),
    c(
        "CG",
        "Congo",
        AfriqueCentrale,
        Yf::Required,
        Palu::High,
        HEP_A | HEP_B | TYPHOIDE | RAGE,
    ),
    c(
        "CD",
        "République démocratique du Congo",
        AfriqueCentrale,
        Yf::Required,
        Palu::High,
        HEP_A | HEP_B | TYPHOIDE | RAGE | MENINGO | POLIO,
    ),
    c(
        "AO",
        "Angola",
        AfriqueCentrale,
        Yf::Required,
        Palu::High,
        HEP_A | HEP_B | TYPHOIDE | RAGE | POLIO,
    ),
    c(
        "ST",
        "Sao Tomé-et-Principe",
        AfriqueCentrale,
        Yf::RequiredFromEndemic,
        Palu::Present,
        HEP_A | HEP_B | TYPHOIDE,
    ),
    // --- Afrique de l'Est ---
    c(
        "SD",
        "Soudan",
        AfriqueEst,
        Yf::Required,
        Palu::High,
        HEP_A | HEP_B | TYPHOIDE | RAGE | MENINGO | POLIO,
    ),
    c(
        "SS",
        "Soudan du Sud",
        AfriqueEst,
        Yf::Required,
        Palu::High,
        HEP_A | HEP_B | TYPHOIDE | RAGE | MENINGO | POLIO,
    ),
    c(
        "ER",
        "Érythrée",
        AfriqueEst,
        Yf::RequiredFromEndemic,
        Palu::Present,
        HEP_A | HEP_B | TYPHOIDE | RAGE | MENINGO,
    ),
    c(
        "ET",
        "Éthiopie",
        AfriqueEst,
        Yf::Recommended,
        Palu::Present,
        HEP_A | HEP_B | TYPHOIDE | RAGE | MENINGO,
    ),
    c(
        "DJ",
        "Djibouti",
        AfriqueEst,
        Yf::RequiredFromEndemic,
        Palu::Present,
        HEP_A | HEP_B | TYPHOIDE | RAGE | MENINGO,
    ),
    c(
        "SO",
        "Somalie",
        AfriqueEst,
        Yf::RequiredFromEndemic,
        Palu::Present,
        HEP_A | HEP_B | TYPHOIDE | RAGE | MENINGO | POLIO,
    ),
    c(
        "KE",
        "Kenya",
        AfriqueEst,
        Yf::Recommended,
        Palu::Present,
        HEP_A | HEP_B | TYPHOIDE | RAGE,
    ),
    c(
        "UG",
        "Ouganda",
        AfriqueEst,
        Yf::Required,
        Palu::High,
        HEP_A | HEP_B | TYPHOIDE | RAGE | MENINGO,
    ),
    c(
        "RW",
        "Rwanda",
        AfriqueEst,
        Yf::RequiredFromEndemic,
        Palu::Present,
        HEP_A | HEP_B | TYPHOIDE | RAGE,
    ),
    c(
        "BI",
        "Burundi",
        AfriqueEst,
        Yf::Recommended,
        Palu::High,
        HEP_A | HEP_B | TYPHOIDE | RAGE,
    ),
    c(
        "TZ",
        "Tanzanie",
        AfriqueEst,
        Yf::RequiredFromEndemic,
        Palu::High,
        HEP_A | HEP_B | TYPHOIDE | RAGE,
    ),
    c(
        "MG",
        "Madagascar",
        AfriqueEst,
        Yf::RequiredFromEndemic,
        Palu::Present,
        HEP_A | HEP_B | TYPHOIDE | RAGE | POLIO,
    ),
    c(
        "KM",
        "Comores",
        AfriqueEst,
        Yf::RequiredFromEndemic,
        Palu::Present,
        HEP_A | HEP_B | TYPHOIDE | RAGE,
    ),
    c(
        "MU",
        "Maurice",
        AfriqueEst,
        Yf::RequiredFromEndemic,
        Palu::No,
        HEP_A | TYPHOIDE,
    ),
    c(
        "SC",
        "Seychelles",
        AfriqueEst,
        Yf::RequiredFromEndemic,
        Palu::No,
        HEP_A | TYPHOIDE,
    ),
    // --- Afrique australe ---
    c(
        "ZM",
        "Zambie",
        AfriqueAustrale,
        Yf::RequiredFromEndemic,
        Palu::High,
        HEP_A | HEP_B | TYPHOIDE | RAGE,
    ),
    c(
        "ZW",
        "Zimbabwe",
        AfriqueAustrale,
        Yf::RequiredFromEndemic,
        Palu::Present,
        HEP_A | HEP_B | TYPHOIDE | RAGE,
    ),
    c(
        "MW",
        "Malawi",
        AfriqueAustrale,
        Yf::RequiredFromEndemic,
        Palu::High,
        HEP_A | HEP_B | TYPHOIDE | RAGE | POLIO,
    ),
    c(
        "MZ",
        "Mozambique",
        AfriqueAustrale,
        Yf::RequiredFromEndemic,
        Palu::High,
        HEP_A | HEP_B | TYPHOIDE | RAGE | POLIO,
    ),
    c(
        "BW",
        "Botswana",
        AfriqueAustrale,
        Yf::RequiredFromEndemic,
        Palu::Limited,
        HEP_A | HEP_B | TYPHOIDE | RAGE,
    ),
    c(
        "NA",
        "Namibie",
        AfriqueAustrale,
        Yf::RequiredFromEndemic,
        Palu::Limited,
        HEP_A | HEP_B | TYPHOIDE | RAGE,
    ),
    c(
        "ZA",
        "Afrique du Sud",
        AfriqueAustrale,
        Yf::RequiredFromEndemic,
        Palu::Limited,
        HEP_A | HEP_B | TYPHOIDE | RAGE,
    ),
    c(
        "LS",
        "Lesotho",
        AfriqueAustrale,
        Yf::RequiredFromEndemic,
        Palu::No,
        HEP_A | HEP_B | TYPHOIDE,
    ),
    c(
        "SZ",
        "Eswatini",
        AfriqueAustrale,
        Yf::RequiredFromEndemic,
        Palu::Limited,
        HEP_A | HEP_B | TYPHOIDE,
    ),
    // --- Moyen-Orient ---
    c(
        "SY",
        "Syrie",
        MoyenOrient,
        Yf::RequiredFromEndemic,
        Palu::No,
        HEP_A | HEP_B | TYPHOIDE | RAGE | POLIO,
    ),
    c(
        "LB",
        "Liban",
        MoyenOrient,
        Yf::RequiredFromEndemic,
        Palu::No,
        HEP_A | HEP_B | TYPHOIDE | RAGE,
    ),
    c(
        "PS",
        "Palestine",
        MoyenOrient,
        Yf::No,
        Palu::No,
        HEP_A | HEP_B | TYPHOIDE,
    ),
    c(
        "JO",
        "Jordanie",
        MoyenOrient,
        Yf::RequiredFromEndemic,
        Palu::No,
        HEP_A | HEP_B | TYPHOIDE | RAGE,
    ),
    c(
        "IQ",
        "Irak",
        MoyenOrient,
        Yf::RequiredFromEndemic,
        Palu::No,
        HEP_A | HEP_B | TYPHOIDE | RAGE | POLIO,
    ),
    c(
        "IR",
        "Iran",
        MoyenOrient,
        Yf::RequiredFromEndemic,
        Palu::Limited,
        HEP_A | HEP_B | TYPHOIDE | RAGE,
    ),
    c(
        "SA",
        "Arabie saoudite",
        MoyenOrient,
        Yf::RequiredFromEndemic,
        Palu::Limited,
        HEP_A | HEP_B | TYPHOIDE | RAGE | MENINGO,
    ),
    c(
        "YE",
        "Yémen",
        MoyenOrient,
        Yf::RequiredFromEndemic,
        Palu::Present,
        HEP_A | HEP_B | TYPHOIDE | RAGE | CHOLERA | POLIO,
    ),
    c(
        "OM",
        "Oman",
        MoyenOrient,
        Yf::RequiredFromEndemic,
        Palu::Limited,
        HEP_A | HEP_B | TYPHOIDE | RAGE,
    ),
    c(
        "AE",
        "Émirats arabes unis",
        MoyenOrient,
        Yf::RequiredFromEndemic,
        Palu::No,
        HEP_A | HEP_B | TYPHOIDE,
    ),
    c(
        "QA",
        "Qatar",
        MoyenOrient,
        Yf::RequiredFromEndemic,
        Palu::No,
        HEP_A | HEP_B | TYPHOIDE,
    ),
    c(
        "BH",
        "Bahreïn",
        MoyenOrient,
        Yf::RequiredFromEndemic,
        Palu::No,
        HEP_A | HEP_B | TYPHOIDE,
    ),
    c(
        "KW",
        "Koweït",
        MoyenOrient,
        Yf::RequiredFromEndemic,
        Palu::No,
        HEP_A | HEP_B | TYPHOIDE,
    ),
    // --- Asie centrale ---
    c(
        "KZ",
        "Kazakhstan",
        AsieCentrale,
        Yf::No,
        Palu::No,
        HEP_A | HEP_B | TYPHOIDE | RAGE | ENCEPH_TIQUES,
    ),
    c(
        "UZ",
        "Ouzbékistan",
        AsieCentrale,
        Yf::No,
        Palu::No,
        HEP_A | HEP_B | TYPHOIDE | RAGE,
    ),
    c(
        "TM",
        "Turkménistan",
        AsieCentrale,
        Yf::No,
        Palu::No,
        HEP_A | HEP_B | TYPHOIDE | RAGE,
    ),
    c(
        "KG",
        "Kirghizistan",
        AsieCentrale,
        Yf::No,
        Palu::No,
        HEP_A | HEP_B | TYPHOIDE | RAGE,
    ),
    c(
        "TJ",
        "Tadjikistan",
        AsieCentrale,
        Yf::No,
        Palu::Limited,
        HEP_A | HEP_B | TYPHOIDE | RAGE | POLIO,
    ),
    c(
        "AF",
        "Afghanistan",
        AsieCentrale,
        Yf::RequiredFromEndemic,
        Palu::Present,
        HEP_A | HEP_B | TYPHOIDE | RAGE | POLIO,
    ),
    c(
        "MN",
        "Mongolie",
        AsieCentrale,
        Yf::No,
        Palu::No,
        HEP_A | HEP_B | TYPHOIDE | RAGE | ENCEPH_TIQUES,
    ),
    // --- Asie du Sud ---
    c(
        "PK",
        "Pakistan",
        AsieSud,
        Yf::RequiredFromEndemic,
        Palu::Present,
        HEP_A | HEP_B | TYPHOIDE | RAGE | ENCEPH_JAP | POLIO,
    ),
    c(
        "IN",
        "Inde",
        AsieSud,
        Yf::RequiredFromEndemic,
        Palu::Present,
        HEP_A | HEP_B | TYPHOIDE | RAGE | ENCEPH_JAP,
    ),
    c(
        "NP",
        "Népal",
        AsieSud,
        Yf::RequiredFromEndemic,
        Palu::Limited,
        HEP_A | HEP_B | TYPHOIDE | RAGE | ENCEPH_JAP,
    ),
    c(
        "BT",
        "Bhoutan",
        AsieSud,
        Yf::RequiredFromEndemic,
        Palu::Limited,
        HEP_A | HEP_B | TYPHOIDE | RAGE | ENCEPH_JAP,
    ),
    c(
        "BD",
        "Bangladesh",
        AsieSud,
        Yf::RequiredFromEndemic,
        Palu::Limited,
        HEP_A | HEP_B | TYPHOIDE | RAGE | ENCEPH_JAP,
    ),
    c(
        "LK",
        "Sri Lanka",
        AsieSud,
        Yf::RequiredFromEndemic,
        Palu::No,
        HEP_A | HEP_B | TYPHOIDE | RAGE | ENCEPH_JAP,
    ),
    c(
        "MV",
        "Maldives",
        AsieSud,
        Yf::RequiredFromEndemic,
        Palu::No,
        HEP_A | HEP_B | TYPHOIDE,
    ),
    // --- Asie de l'Est ---
    c(
        "CN",
        "Chine",
        AsieEst,
        Yf::RequiredFromEndemic,
        Palu::Limited,
        HEP_A | HEP_B | TYPHOIDE | RAGE | ENCEPH_JAP | ENCEPH_TIQUES,
    ),
    c(
        "KP",
        "Corée du Nord",
        AsieEst,
        Yf::RequiredFromEndemic,
        Palu::Limited,
        HEP_A | HEP_B | TYPHOIDE | ENCEPH_JAP,
    ),
    c(
        "KR",
        "Corée du Sud",
        AsieEst,
        Yf::RequiredFromEndemic,
        Palu::No,
        HEP_A | ENCEPH_JAP,
    ),
    c(
        "JP",
        "Japon",
        AsieEst,
        Yf::No,
        Palu::No,
        ENCEPH_JAP | ENCEPH_TIQUES,
    ),
    c(
        "TW",
        "Taïwan",
        AsieEst,
        Yf::No,
        Palu::No,
        HEP_A | ENCEPH_JAP,
    ),
    c(
        "HK",
        "Hong Kong",
        AsieEst,
        Yf::RequiredFromEndemic,
        Palu::No,
        HEP_A | ENCEPH_JAP,
    ),
    // --- Asie du Sud-Est ---
    c(
        "MM",
        "Birmanie (Myanmar)",
        AsieSudEst,
        Yf::RequiredFromEndemic,
        Palu::Present,
        HEP_A | HEP_B | TYPHOIDE | RAGE | ENCEPH_JAP,
    ),
    c(
        "TH",
        "Thaïlande",
        AsieSudEst,
        Yf::RequiredFromEndemic,
        Palu::Limited,
        HEP_A | HEP_B | TYPHOIDE | RAGE | ENCEPH_JAP,
    ),
    c(
        "LA",
        "Laos",
        AsieSudEst,
        Yf::RequiredFromEndemic,
        Palu::Present,
        HEP_A | HEP_B | TYPHOIDE | RAGE | ENCEPH_JAP,
    ),
    c(
        "KH",
        "Cambodge",
        AsieSudEst,
        Yf::RequiredFromEndemic,
        Palu::Present,
        HEP_A | HEP_B | TYPHOIDE | RAGE | ENCEPH_JAP,
    ),
    c(
        "VN",
        "Viêt Nam",
        AsieSudEst,
        Yf::RequiredFromEndemic,
        Palu::Limited,
        HEP_A | HEP_B | TYPHOIDE | RAGE | ENCEPH_JAP,
    ),
    c(
        "MY",
        "Malaisie",
        AsieSudEst,
        Yf::RequiredFromEndemic,
        Palu::Limited,
        HEP_A | HEP_B | TYPHOIDE | RAGE | ENCEPH_JAP,
    ),
    c(
        "SG",
        "Singapour",
        AsieSudEst,
        Yf::RequiredFromEndemic,
        Palu::No,
        HEP_A | ENCEPH_JAP,
    ),
    c(
        "BN",
        "Brunei",
        AsieSudEst,
        Yf::RequiredFromEndemic,
        Palu::No,
        HEP_A | HEP_B | TYPHOIDE | ENCEPH_JAP,
    ),
    c(
        "ID",
        "Indonésie",
        AsieSudEst,
        Yf::RequiredFromEndemic,
        Palu::Present,
        HEP_A | HEP_B | TYPHOIDE | RAGE | ENCEPH_JAP,
    ),
    c(
        "TL",
        "Timor oriental",
        AsieSudEst,
        Yf::RequiredFromEndemic,
        Palu::Present,
        HEP_A | HEP_B | TYPHOIDE | RAGE | ENCEPH_JAP,
    ),
    c(
        "PH",
        "Philippines",
        AsieSudEst,
        Yf::RequiredFromEndemic,
        Palu::Limited,
        HEP_A | HEP_B | TYPHOIDE | RAGE | ENCEPH_JAP | POLIO,
    ),
    // --- Océanie ---
    c(
        "AU",
        "Australie",
        Oceanie,
        Yf::RequiredFromEndemic,
        Palu::No,
        ENCEPH_JAP,
    ),
    c("NZ", "Nouvelle-Zélande", Oceanie, Yf::No, Palu::No, 0),
    c(
        "PG",
        "Papouasie-Nouvelle-Guinée",
        Oceanie,
        Yf::RequiredFromEndemic,
        Palu::High,
        HEP_A | HEP_B | TYPHOIDE | RAGE | ENCEPH_JAP | POLIO,
    ),
    c(
        "SB",
        "Îles Salomon",
        Oceanie,
        Yf::RequiredFromEndemic,
        Palu::Present,
        HEP_A | HEP_B | TYPHOIDE,
    ),
    c(
        "VU",
        "Vanuatu",
        Oceanie,
        Yf::RequiredFromEndemic,
        Palu::Present,
        HEP_A | HEP_B | TYPHOIDE,
    ),
    c(
        "NC",
        "Nouvelle-Calédonie",
        Oceanie,
        Yf::RequiredFromEndemic,
        Palu::No,
        HEP_A,
    ),
    c(
        "FJ",
        "Fidji",
        Oceanie,
        Yf::RequiredFromEndemic,
        Palu::No,
        HEP_A | HEP_B | TYPHOIDE,
    ),
    c(
        "PF",
        "Polynésie française",
        Oceanie,
        Yf::RequiredFromEndemic,
        Palu::No,
        HEP_A,
    ),
    c(
        "WS",
        "Samoa",
        Oceanie,
        Yf::RequiredFromEndemic,
        Palu::No,
        HEP_A | HEP_B | TYPHOIDE,
    ),
    c(
        "TO",
        "Tonga",
        Oceanie,
        Yf::RequiredFromEndemic,
        Palu::No,
        HEP_A | HEP_B | TYPHOIDE,
    ),
    c("CK", "Îles Cook", Oceanie, Yf::No, Palu::No, HEP_A),
    c(
        "KI",
        "Kiribati",
        Oceanie,
        Yf::RequiredFromEndemic,
        Palu::No,
        HEP_A | HEP_B | TYPHOIDE,
    ),
    c(
        "TV",
        "Tuvalu",
        Oceanie,
        Yf::RequiredFromEndemic,
        Palu::No,
        HEP_A | HEP_B | TYPHOIDE,
    ),
    c(
        "NR",
        "Nauru",
        Oceanie,
        Yf::RequiredFromEndemic,
        Palu::No,
        HEP_A | HEP_B | TYPHOIDE,
    ),
    c(
        "FM",
        "Micronésie",
        Oceanie,
        Yf::No,
        Palu::No,
        HEP_A | HEP_B | TYPHOIDE,
    ),
    c(
        "MH",
        "Îles Marshall",
        Oceanie,
        Yf::No,
        Palu::No,
        HEP_A | HEP_B | TYPHOIDE,
    ),
    c(
        "PW",
        "Palaos",
        Oceanie,
        Yf::No,
        Palu::No,
        HEP_A | HEP_B | TYPHOIDE,
    ),
    // --- Amérique centrale ---
    c(
        "MX",
        "Mexique",
        AmeriqueCentrale,
        Yf::RequiredFromEndemic,
        Palu::Limited,
        HEP_A | HEP_B | TYPHOIDE | RAGE,
    ),
    c(
        "GT",
        "Guatemala",
        AmeriqueCentrale,
        Yf::RequiredFromEndemic,
        Palu::Limited,
        HEP_A | HEP_B | TYPHOIDE | RAGE,
    ),
    c(
        "BZ",
        "Belize",
        AmeriqueCentrale,
        Yf::RequiredFromEndemic,
        Palu::Limited,
        HEP_A | HEP_B | TYPHOIDE | RAGE,
    ),
    c(
        "SV",
        "Salvador",
        AmeriqueCentrale,
        Yf::RequiredFromEndemic,
        Palu::Limited,
        HEP_A | HEP_B | TYPHOIDE | RAGE,
    ),
    c(
        "HN",
        "Honduras",
        AmeriqueCentrale,
        Yf::RequiredFromEndemic,
        Palu::Present,
        HEP_A | HEP_B | TYPHOIDE | RAGE,
    ),
    c(
        "NI",
        "Nicaragua",
        AmeriqueCentrale,
        Yf::RequiredFromEndemic,
        Palu::Present,
        HEP_A | HEP_B | TYPHOIDE | RAGE,
    ),
    c(
        "CR",
        "Costa Rica",
        AmeriqueCentrale,
        Yf::RequiredFromEndemic,
        Palu::Limited,
        HEP_A | HEP_B | TYPHOIDE | RAGE,
    ),
    c(
        "PA",
        "Panama",
        AmeriqueCentrale,
        Yf::Recommended,
        Palu::Limited,
        HEP_A | HEP_B | TYPHOIDE | RAGE,
    ),
    // --- Caraïbes ---
    c(
        "CU",
        "Cuba",
        Caraibes,
        Yf::RequiredFromEndemic,
        Palu::No,
        HEP_A | HEP_B | TYPHOIDE,
    ),
    c(
        "BS",
        "Bahamas",
        Caraibes,
        Yf::RequiredFromEndemic,
        Palu::No,
        HEP_A | HEP_B | TYPHOIDE,
    ),
    c(
        "JM",
        "Jamaïque",
        Caraibes,
        Yf::RequiredFromEndemic,
        Palu::No,
        HEP_A | HEP_B | TYPHOIDE,
    ),
    c(
        "HT",
        "Haïti",
        Caraibes,
        Yf::RequiredFromEndemic,
        Palu::Present,
        HEP_A | HEP_B | TYPHOIDE | RAGE | CHOLERA,
    ),
    c(
        "DO",
        "République dominicaine",
        Caraibes,
        Yf::RequiredFromEndemic,
        Palu::Limited,
        HEP_A | HEP_B | TYPHOIDE | RAGE,
    ),
    c(
        "PR",
        "Porto Rico",
        Caraibes,
        Yf::No,
        Palu::No,
        HEP_A | HEP_B,
    ),
    c(
        "VI",
        "Îles Vierges américaines",
        Caraibes,
        Yf::No,
        Palu::No,
        HEP_A | HEP_B,
    ),
    c(
        "KN",
        "Saint-Christophe-et-Niévès",
        Caraibes,
        Yf::RequiredFromEndemic,
        Palu::No,
        HEP_A | HEP_B | TYPHOIDE,
    ),
    c(
        "AG",
        "Antigua-et-Barbuda",
        Caraibes,
        Yf::RequiredFromEndemic,
        Palu::No,
        HEP_A | HEP_B | TYPHOIDE,
    ),
    c("GP", "Guadeloupe", Caraibes, Yf::No, Palu::No, HEP_A),
    c(
        "DM",
        "Dominique",
        Caraibes,
        Yf::RequiredFromEndemic,
        Palu::No,
        HEP_A | HEP_B | TYPHOIDE,
    ),
    c("MQ", "Martinique", Caraibes, Yf::No, Palu::No, HEP_A),
    c(
        "LC",
        "Sainte-Lucie",
        Caraibes,
        Yf::RequiredFromEndemic,
        Palu::No,
        HEP_A | HEP_B | TYPHOIDE,
    ),
    c(
        "VC",
        "Saint-Vincent-et-les-Grenadines",
        Caraibes,
        Yf::RequiredFromEndemic,
        Palu::No,
        HEP_A | HEP_B | TYPHOIDE,
    ),
    c(
        "BB",
        "Barbade",
        Caraibes,
        Yf::RequiredFromEndemic,
        Palu::No,
        HEP_A | HEP_B | TYPHOIDE,
    ),
    c(
        "GD",
        "Grenade",
        Caraibes,
        Yf::RequiredFromEndemic,
        Palu::No,
        HEP_A | HEP_B | TYPHOIDE,
    ),
    c(
        "TT",
        "Trinité-et-Tobago",
        Caraibes,
        Yf::Recommended,
        Palu::No,
        HEP_A | HEP_B | TYPHOIDE,
    ),
    c(
        "AW",
        "Aruba",
        Caraibes,
        Yf::RequiredFromEndemic,
        Palu::No,
        HEP_A | HEP_B,
    ),
    c(
        "CW",
        "Curaçao",
        Caraibes,
        Yf::RequiredFromEndemic,
        Palu::No,
        HEP_A | HEP_B,
    ),
    // --- Amérique du Sud ---
    c(
        "CO",
        "Colombie",
        AmeriqueSud,
        Yf::Recommended,
        Palu::Present,
        HEP_A | HEP_B | TYPHOIDE | RAGE,
    ),
    c(
        "VE",
        "Venezuela",
        AmeriqueSud,
        Yf::Recommended,
        Palu::Present,
        HEP_A | HEP_B | TYPHOIDE | RAGE,
    ),
    c(
        "GY",
        "Guyana",
        AmeriqueSud,
        Yf::Required,
        Palu::Present,
        HEP_A | HEP_B | TYPHOIDE | RAGE,
    ),
    c(
        "SR",
        "Suriname",
        AmeriqueSud,
        Yf::Recommended,
        Palu::Limited,
        HEP_A | HEP_B | TYPHOIDE | RAGE,
    ),
    c(
        "GF",
        "Guyane française",
        AmeriqueSud,
        Yf::Required,
        Palu::Present,
        HEP_A | HEP_B | TYPHOIDE | RAGE,
    ),
    c(
        "EC",
        "Équateur",
        AmeriqueSud,
        Yf::Recommended,
        Palu::Limited,
        HEP_A | HEP_B | TYPHOIDE | RAGE,
    ),
    c(
        "PE",
        "Pérou",
        AmeriqueSud,
        Yf::Recommended,
        Palu::Present,
        HEP_A | HEP_B | TYPHOIDE | RAGE,
    ),
    c(
        "BR",
        "Brésil",
        AmeriqueSud,
        Yf::Recommended,
        Palu::Present,
        HEP_A | HEP_B | TYPHOIDE | RAGE,
    ),
    c(
        "BO",
        "Bolivie",
        AmeriqueSud,
        Yf::Recommended,
        Palu::Present,
        HEP_A | HEP_B | TYPHOIDE | RAGE,
    ),
    c(
        "PY",
        "Paraguay",
        AmeriqueSud,
        Yf::Recommended,
        Palu::No,
        HEP_A | HEP_B | TYPHOIDE | RAGE,
    ),
    c(
        "CL",
        "Chili",
        AmeriqueSud,
        Yf::No,
        Palu::No,
        HEP_A | TYPHOIDE,
    ),
    c(
        "AR",
        "Argentine",
        AmeriqueSud,
        Yf::Recommended,
        Palu::No,
        HEP_A | HEP_B | TYPHOIDE,
    ),
    c(
        "UY",
        "Uruguay",
        AmeriqueSud,
        Yf::RequiredFromEndemic,
        Palu::No,
        HEP_A | TYPHOIDE,
    ),
    c("FK", "Îles Malouines", AmeriqueSud, Yf::No, Palu::No, 0),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_country_has_its_own_tile() {
        let mut seen = std::collections::HashMap::new();
        for country in COUNTRIES {
            let tile = country.tile();
            if let Some(other) = seen.insert(tile, country.code) {
                panic!(
                    "{} et {} occupent la même case {tile:?}",
                    other, country.code
                );
            }
        }
    }

    #[test]
    fn region_blocks_do_not_overlap() {
        // Two regions whose blocks intersect would interleave their
        // tiles: the test above would catch it, but only for the cells
        // that actually collide. Check the rectangles themselves.
        let extents: Vec<(Region, i32, i32, i32, i32)> = Region::ALL
            .iter()
            .map(|r| {
                let (col, row, width) = r.block();
                let n = COUNTRIES.iter().filter(|c| c.region == *r).count() as i32;
                let width = width.max(1);
                let height = (n + width - 1) / width;
                (*r, col, row, width, height)
            })
            .collect();
        for (i, a) in extents.iter().enumerate() {
            for b in &extents[i + 1..] {
                let overlap_x = a.1 < b.1 + b.3 && b.1 < a.1 + a.3;
                let overlap_y = a.2 < b.2 + b.4 && b.2 < a.2 + a.4;
                assert!(
                    !(overlap_x && overlap_y),
                    "les blocs {:?} et {:?} se chevauchent",
                    a.0,
                    b.0
                );
            }
        }
    }

    #[test]
    fn codes_are_unique_and_well_formed() {
        let mut seen = std::collections::HashSet::new();
        for country in COUNTRIES {
            assert!(
                seen.insert(country.code),
                "code en double : {}",
                country.code
            );
            assert_eq!(country.code.len(), 2, "code ISO attendu : {}", country.code);
            assert!(!country.name.is_empty());
        }
    }

    #[test]
    fn every_reco_flag_maps_to_a_catalogue_entry() {
        for r in RECOS {
            assert!(
                CATALOGUE.iter().any(|v| v.code == r.code),
                "{} ne correspond à aucun vaccin du catalogue",
                r.code
            );
        }
    }

    #[test]
    fn search_finds_a_country_by_name_code_and_accentless_spelling() {
        assert_eq!(search("thai")[0].code, "TH");
        assert_eq!(search("TH")[0].code, "TH");
        assert_eq!(search("Bresil")[0].code, "BR");
        assert_eq!(search("egypte")[0].code, "EG");
        assert!(search("  ").is_empty());
    }

    #[test]
    fn dtp_milestones_follow_the_calendar() {
        // The milestone standing under an age.
        assert_eq!(dtp_milestone(20), None);
        assert_eq!(dtp_milestone(25), Some(25));
        assert_eq!(dtp_milestone(44), Some(25));
        assert_eq!(dtp_milestone(45), Some(45));
        assert_eq!(dtp_milestone(64), Some(45));
        assert_eq!(dtp_milestone(65), Some(65));
        assert_eq!(dtp_milestone(74), Some(65));
        assert_eq!(dtp_milestone(75), Some(75));
        assert_eq!(dtp_milestone(86), Some(85));
        // And the one after it.
        assert_eq!(dtp_next_after(25), 45);
        assert_eq!(dtp_next_after(45), 65);
        assert_eq!(dtp_next_after(65), 75);
        assert_eq!(dtp_next_after(85), 95);
    }

    #[test]
    fn a_booster_at_25_is_not_overdue_at_36() {
        // Eleven years old, but the next milestone is 45: nothing due.
        let doses = [Dose {
            code: "DTP",
            date: "2015-05-04",
        }];
        let lines = due_lines(Some(36), Some(1990), "2026-08-26", &doses);
        let dtp = lines.iter().find(|l| l.code == "DTP").unwrap();
        assert_eq!(dtp.level, DueLevel::Ok);
        assert!(dtp.detail.contains("45 ans"), "{}", dtp.detail);
    }

    #[test]
    fn a_dose_given_before_the_current_milestone_is_owed() {
        // Boosted at 45, now 66: the 65 booster was never given.
        let doses = [Dose {
            code: "DTP",
            date: "2005-03-01",
        }];
        let lines = due_lines(Some(66), Some(1960), "2026-08-26", &doses);
        let dtp = lines.iter().find(|l| l.code == "DTP").unwrap();
        assert_eq!(dtp.level, DueLevel::Due);
        assert!(dtp.detail.contains("65 ans"), "{}", dtp.detail);
    }

    #[test]
    fn the_flu_campaign_starts_in_september() {
        assert_eq!(flu_season_start("2026-08-26"), "2025-09-01");
        assert_eq!(flu_season_start("2026-09-01"), "2026-09-01");
        assert_eq!(flu_season_start("2026-12-31"), "2026-09-01");
    }

    #[test]
    fn an_empty_carnet_owes_the_milestone_already_reached() {
        let lines = due_lines(Some(52), Some(1974), "2026-08-26", &[]);
        let dtp = lines.iter().find(|l| l.code == "DTP").unwrap();
        assert_eq!(dtp.level, DueLevel::Due);
        // At 52 the booster owed is the one for 45, not a future one.
        assert!(dtp.detail.contains("45 ans"), "{}", dtp.detail);
        // Born before 1980: the ROR line is not raised.
        assert!(!lines.iter().any(|l| l.code == "ROR"));
    }

    #[test]
    fn a_recent_booster_and_this_years_flu_shot_read_as_up_to_date() {
        let doses = [
            Dose {
                code: "DTP",
                date: "2022-04-12",
            },
            Dose {
                code: "GRIPPE",
                date: "2025-10-03",
            },
        ];
        let lines = due_lines(Some(70), Some(1956), "2026-01-15", &doses);
        let dtp = lines.iter().find(|l| l.code == "DTP").unwrap();
        assert_eq!(dtp.level, DueLevel::Ok);
        let flu = lines.iter().find(|l| l.code == "GRIPPE").unwrap();
        assert_eq!(flu.level, DueLevel::Ok);
        // 70 ans : le zona est dû, le VRS ne l'est pas encore.
        assert_eq!(
            lines.iter().find(|l| l.code == "ZONA").unwrap().level,
            DueLevel::Due
        );
        assert!(!lines.iter().any(|l| l.code == "VRS"));
    }

    #[test]
    fn a_dtcap_dose_counts_for_the_dtp_clock() {
        // Given at 32, so the 45 booster is still owed at 48.
        let doses = [Dose {
            code: "DTCAP",
            date: "2010-06-01",
        }];
        let lines = due_lines(Some(48), Some(1978), "2026-08-26", &doses);
        let dtp = lines.iter().find(|l| l.code == "DTP").unwrap();
        assert_eq!(dtp.level, DueLevel::Due);
        assert!(dtp.detail.contains("32 ans"), "{}", dtp.detail);
    }

    #[test]
    fn travel_flags_resolve_to_named_vaccines() {
        let mali = country("ML").unwrap();
        let labels: Vec<&str> = mali.recos().map(|r| r.label).collect();
        assert!(labels.contains(&"Méningocoque ACYW"));
        assert!(mali.yf.needed());
        let france = country("FR").unwrap();
        assert_eq!(france.recos().count(), 0);
        assert!(!france.yf.needed());
    }
}
