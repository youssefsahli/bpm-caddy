//! Les campagnes de l'officine : la vaccination de l'hiver, et toute
//! autre campagne que l'équipe se donne — une autre année, un autre
//! vaccin, ou un sujet qui n'est pas un vaccin (les entretiens AVK, les
//! bilans partagés de médication, un dépistage).
//!
//! Une campagne est une **période**, une **clé** sous laquelle ses appels
//! sont rangés, et des **sujets**. Un sujet dit qui il vise (le calendrier
//! vaccinal, ou des critères : âge, sexe, grossesse, traitements), ce qui
//! le règle (une dose au carnet, un acte réalisé, ou seulement un appel)
//! et sur quelle durée. Trois questions que l'équipe se pose chaque jour
//! d'une campagne, et auxquelles le dossier répondait un dossier à la
//! fois :
//!
//! - **Combien reste-t-il de doses de ce lot ?** Le lot est saisi au
//!   carnet à chaque injection ; il suffit de compter les lignes qui le
//!   portent contre ce qui a été reçu. Rien n'est décrémenté à la main :
//!   une dose corrigée ou supprimée au carnet se retrouve dans le compte.
//! - **Combien depuis le début de la campagne ?** Par sujet, le jour, la
//!   semaine et la campagne, lus dans les carnets, les actes et les appels.
//! - **Qui n'est pas encore venu ?** Les dossiers que le sujet vise, sans
//!   rien qui le règle sur la période, moins ceux que l'équipe a déjà
//!   appelés pour cette campagne.
//!
//! Sans campagne enregistrée, la campagne de la saison
//! ([`default_campaign`]) est construite pour l'année en cours : rien
//! n'est semé, et chaque hiver a la sienne sans qu'on y touche.
//!
//! Pur et sans horloge : la date du jour est passée en paramètre.

use crate::vaccines::{self, DueLevel};

// ---------------------------------------------------------------------
// La saison
// ---------------------------------------------------------------------

/// Une saison de vaccination : du 1er septembre au 31 août suivant.
///
/// Le découpage est celui que le calendrier lit déjà
/// (`vaccines::flu_season_start`) : une dose de grippe faite en
/// septembre compte pour l'hiver qui vient, jamais pour le précédent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Season {
    /// ISO, premier jour (1er septembre).
    pub start: String,
    /// ISO, dernier jour (31 août).
    pub end: String,
    /// « 2026-2027 » : la clé sous laquelle les appels de la campagne de
    /// la saison sont rangés.
    pub label: String,
}

/// La saison dans laquelle tombe `today` (ISO).
pub fn season(today: &str) -> Season {
    let start = vaccines::flu_season_start(today);
    let year: u32 = start.get(..4).and_then(|y| y.parse().ok()).unwrap_or(0);
    Season {
        end: format!("{:04}-08-31", year + 1),
        label: format!("{:04}-{:04}", year, year + 1),
        start,
    }
}

/// Une clé de la forme d'une saison (« 2026-2027 ») : l'année qui suit.
fn next_season_key(key: &str) -> Option<String> {
    let (a, b) = key.split_once('-')?;
    let (a, b): (u32, u32) = (a.parse().ok()?, b.parse().ok()?);
    (a >= 1900 && b == a + 1 && key.len() == 9).then(|| format!("{:04}-{:04}", a + 1, b + 1))
}

// ---------------------------------------------------------------------
// La campagne et ses sujets
// ---------------------------------------------------------------------

/// Ce qui règle un sujet pour un dossier.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Means {
    /// Une dose de ce vaccin au carnet (`Subject::code` est le code du
    /// catalogue). Le rendez-vous est un acte de vaccination.
    Vaccine,
    /// Un acte de ce type réalisé (`db::InterviewKind::as_str`) : un
    /// entretien AVK, un bilan partagé de médication. Le rendez-vous est
    /// cet acte, planifié.
    Act(String),
    /// Rien que la base sache lire : seul un appel le règle (un dépistage
    /// fait ailleurs, une information donnée).
    Call,
}

/// Qui un sujet vise.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rule {
    /// Le calendrier vaccinal en décide (`vaccines::due_lines_with`) :
    /// seulement pour un vaccin que le calendrier lit.
    Calendar,
    /// Les critères du sujet : âge, sexe, grossesse, traitements.
    Criteria,
}

/// Sur quelle durée ce qui règle un sujet compte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Coverage {
    /// Depuis le premier jour de la campagne (la grippe, chaque hiver).
    Campaign,
    /// Une fois pour toutes (le pneumocoque, le VRS).
    Ever,
    /// Dans les `n` derniers mois (un bilan annuel : 12).
    Months(u32),
}

/// Un sujet de campagne : un vaccin, un acte, ou un appel à passer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Subject {
    /// Le code du catalogue pour un vaccin ; pour un autre sujet, une
    /// clé libre, stable, sous laquelle ses appels sont rangés.
    pub code: String,
    /// Le nom court, sur les boutons et les listes.
    pub label: String,
    pub means: Means,
    pub rule: Rule,
    /// Critères (`Rule::Criteria`) : âge à la date du jour, bornes
    /// comprises ; une borne exige une date de naissance.
    pub age_min: Option<u32>,
    pub age_max: Option<u32>,
    /// `F`, `M`, ou vide pour tous.
    pub sex: String,
    /// Une grossesse en cours inclut le dossier quel que soit son âge.
    pub pregnancy: bool,
    /// Un dossier hors des bornes d'âge, dont un traitement évoque un
    /// groupe à risque ([`evocations`]), est listé à part, « à confirmer ».
    pub evoked: bool,
    /// Fragments de DCI ou de nom (« apixaban », « fluindione ») : le
    /// dossier doit avoir un traitement qui en porte un.
    pub molecules: Vec<String>,
    /// Le dossier doit avoir au moins ce nombre de traitements (le bilan
    /// partagé de médication : 5).
    pub min_treatments: u32,
    pub coverage: Coverage,
    /// Le nombre de doses ou d'actes qui règlent le sujet (le zona : 2).
    pub doses: u32,
    /// Entre deux doses d'une série, en mois : la suivante n'est pas
    /// rappelée avant.
    pub interval_months: u32,
    /// Les flacons multidoses : un lot de ce vaccin dont la spécialité
    /// porte ce fragment se note ouvert, et s'utilise dans `vial_hours`.
    pub vial_product: String,
    pub vial_hours: u32,
}

impl Default for Subject {
    fn default() -> Self {
        Subject {
            code: String::new(),
            label: String::new(),
            means: Means::Vaccine,
            rule: Rule::Criteria,
            age_min: None,
            age_max: None,
            sex: String::new(),
            pregnancy: false,
            evoked: false,
            molecules: Vec::new(),
            min_treatments: 0,
            coverage: Coverage::Campaign,
            doses: 1,
            interval_months: 0,
            vial_product: String::new(),
            vial_hours: 0,
        }
    }
}

impl Subject {
    pub fn is_vaccine(&self) -> bool {
        self.means == Means::Vaccine
    }

    /// Le type d'acte que son rendez-vous planifie, s'il en a un.
    pub fn act_kind(&self) -> Option<&str> {
        match &self.means {
            Means::Vaccine => Some("VACCINATION"),
            Means::Act(kind) if !kind.trim().is_empty() => Some(kind.as_str()),
            _ => None,
        }
    }
}

/// Une campagne : une période, une clé, des sujets, et les seuils de son
/// stock et de son bilan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Campaign {
    /// 0 : la campagne de la saison, construite et non enregistrée.
    pub id: i64,
    /// La clé de ses appels : « 2026-2027 » pour la campagne d'une
    /// saison. Fixée à la création, jamais réécrite — des dates
    /// corrigées gardent les appels déjà notés.
    pub key: String,
    pub title: String,
    /// ISO, premier et dernier jour.
    pub starts_on: String,
    pub ends_on: String,
    pub subjects: Vec<Subject>,
    /// Sous ce nombre de doses, un lot est en stock bas.
    pub low_stock: i64,
    /// Une péremption à moins de ce nombre de jours est signalée.
    pub expiry_days: i64,
    /// Les âges qui découpent le bilan : `[65, 75]` donne moins de 65 ans,
    /// 65 à 74 ans, 75 ans et plus.
    pub bands: Vec<u32>,
    pub remark: String,
}

/// Le stock bas et la péremption proche par défaut.
pub const DEFAULT_LOW_STOCK: i64 = 5;
pub const DEFAULT_EXPIRY_DAYS: i64 = 30;
/// Les tranches d'âge du bilan par défaut.
pub const DEFAULT_BANDS: [u32; 2] = [65, 75];

/// Le délai d'utilisation d'un flacon multidose de vaccin contre le
/// COVID-19 après la première ponction (Comirnaty, campagne 2026-2027) :
/// celui de la campagne de la saison, que l'officine corrige dans la
/// sienne quand la spécialité change.
pub const VIAL_HOURS: u32 = 12;

/// Les vaccins de la campagne de la saison, dans l'ordre des boutons.
pub const SEASON_CODES: [&str; 5] = ["GRIPPE", "COVID", "VRS", "PNEUMO", "ZONA"];

/// Les sujets de la campagne de la saison : ce que le calendrier lit
/// chaque hiver, réglé comme il le règle.
pub fn season_subjects() -> Vec<Subject> {
    let vaccine = |code: &str| Subject {
        code: code.to_owned(),
        label: code_label(code).to_owned(),
        rule: Rule::Calendar,
        ..Subject::default()
    };
    vec![
        Subject {
            evoked: true,
            ..vaccine("GRIPPE")
        },
        Subject {
            evoked: true,
            vial_product: "comirnaty".to_owned(),
            vial_hours: VIAL_HOURS,
            ..vaccine("COVID")
        },
        Subject {
            coverage: Coverage::Ever,
            ..vaccine("VRS")
        },
        Subject {
            coverage: Coverage::Ever,
            ..vaccine("PNEUMO")
        },
        // Deux doses, la seconde à partir de deux mois après la première.
        Subject {
            coverage: Coverage::Ever,
            doses: 2,
            interval_months: 2,
            ..vaccine("ZONA")
        },
    ]
}

/// Le nom court d'un vaccin de la saison ; un autre code se montre tel
/// quel.
pub fn code_label(code: &str) -> &str {
    match code {
        "GRIPPE" => crate::strings::tr("camp_code_grippe"),
        "COVID" => crate::strings::tr("camp_code_covid"),
        "VRS" => crate::strings::tr("camp_code_vrs"),
        "PNEUMO" => crate::strings::tr("camp_code_pneumo"),
        "ZONA" => crate::strings::tr("camp_code_zona"),
        "DTCAP" => crate::strings::tr("camp_code_dtcap"),
        other => other,
    }
}

/// La campagne de la saison où tombe `today`, telle que livrée.
pub fn default_campaign(today: &str) -> Campaign {
    let s = season(today);
    Campaign {
        id: 0,
        title: crate::strings::trf("camp_title", &s.label),
        key: s.label,
        starts_on: s.start,
        ends_on: s.end,
        subjects: season_subjects(),
        low_stock: DEFAULT_LOW_STOCK,
        expiry_days: DEFAULT_EXPIRY_DAYS,
        bands: DEFAULT_BANDS.to_vec(),
        remark: String::new(),
    }
}

/// Où en est une campagne à une date.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Upcoming,
    Running,
    Ended,
}

impl Campaign {
    pub fn phase(&self, today: &str) -> Phase {
        if today < self.starts_on.as_str() {
            Phase::Upcoming
        } else if !self.ends_on.is_empty() && today > self.ends_on.as_str() {
            Phase::Ended
        } else {
            Phase::Running
        }
    }

    /// Le dernier jour lu : aujourd'hui, ou la fin d'une campagne passée.
    pub fn read_until<'a>(&'a self, today: &'a str) -> &'a str {
        if !self.ends_on.is_empty() && self.ends_on.as_str() < today {
            &self.ends_on
        } else {
            today
        }
    }

    pub fn subject(&self, code: &str) -> Option<&Subject> {
        self.subjects.iter().find(|s| s.code == code)
    }

    pub fn has_vaccines(&self) -> bool {
        self.subjects.iter().any(Subject::is_vaccine)
    }

    /// Les seuils de son stock.
    pub fn stock(&self) -> Stock {
        Stock {
            low: self.low_stock,
            expiry_days: self.expiry_days,
        }
    }

    /// Le délai d'utilisation d'un flacon ouvert de ce lot, en heures, si
    /// un sujet de la campagne le déclare multidose.
    pub fn vial_hours(&self, lot: &Lot) -> Option<u32> {
        let product = crate::fuzzy::sort_key(&lot.product);
        self.subjects
            .iter()
            .filter(|s| s.is_vaccine() && s.code == lot.code && s.vial_hours > 0)
            .find(|s| {
                let f = crate::fuzzy::sort_key(&s.vial_product);
                f.is_empty() || crate::fuzzy::contains_folded(&product, &f)
            })
            .map(|s| s.vial_hours)
    }

    /// La même campagne l'année suivante : les dates décalées d'un an,
    /// les années du titre aussi, les sujets et les seuils repris. La
    /// clé d'une saison devient celle de la saison suivante ; une autre
    /// est tirée de ses nouvelles dates.
    pub fn next_year(&self, taken: &[&str]) -> Campaign {
        // Un dernier jour de mois reste le dernier : une campagne qui finit
        // le 28 février 2027 finit le 29 février 2028.
        let shift = |d: &str| {
            let moved = crate::date::add_months(d, 12).unwrap_or_else(|| d.to_owned());
            match (crate::date::parse_iso(d), crate::date::parse_iso(&moved)) {
                (Some((y, m, day)), Some((ny, nm, _)))
                    if crate::date::end_of_month(y, m) == Some(day) =>
                {
                    crate::date::end_of_month(ny, nm)
                        .map(|last| format!("{ny:04}-{nm:02}-{last:02}"))
                        .unwrap_or(moved)
                }
                _ => moved,
            }
        };
        let starts_on = shift(&self.starts_on);
        let ends_on = shift(&self.ends_on);
        let key = next_season_key(&self.key)
            .filter(|k| !taken.contains(&k.as_str()))
            .unwrap_or_else(|| fresh_key(&starts_on, &ends_on, taken));
        Campaign {
            id: 0,
            key,
            title: shift_years(&self.title),
            starts_on,
            ends_on,
            ..self.clone()
        }
    }
}

/// Chaque année écrite dans un titre, avancée d'un an : « Hiver
/// 2026-2027 » devient « Hiver 2027-2028 ». Un nombre de quatre chiffres
/// hors de 1900 à 2999 n'est pas une année et reste.
pub fn shift_years(title: &str) -> String {
    let chars: Vec<char> = title.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_ascii_digit() {
            let mut j = i;
            while j < chars.len() && chars[j].is_ascii_digit() {
                j += 1;
            }
            let run: String = chars[i..j].iter().collect();
            match run.parse::<u32>() {
                Ok(y) if run.len() == 4 && (1900..3000).contains(&y) => {
                    out.push_str(&(y + 1).to_string())
                }
                _ => out.push_str(&run),
            }
            i = j;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

/// La clé d'une campagne nouvelle : ses dates, et un rang si une autre
/// campagne les porte déjà. Jamais le libellé d'une saison, même quand
/// ses dates en sont les bornes : cette clé est celle de la campagne
/// livrée de cet hiver-là (et de ce qui la personnalise ou la reprend),
/// et une campagne nouvelle qui la prendrait effacerait la campagne
/// livrée du menu et lirait ses appels — aujourd'hui, ou à l'automne où
/// cette saison arrive.
pub fn fresh_key(starts_on: &str, ends_on: &str, taken: &[&str]) -> String {
    let base = format!("{starts_on}~{ends_on}");
    if !taken.contains(&base.as_str()) {
        return base;
    }
    (2..)
        .map(|n| format!("{base}#{n}"))
        .find(|k| !taken.contains(&k.as_str()))
        .unwrap_or(base)
}

/// Les campagnes que la vue propose : celles de la base, et la campagne
/// de la saison et celle de la saison passée tant qu'aucune campagne
/// enregistrée ne porte leur clé — le bilan de l'hiver dernier reste
/// lisible. En cours d'abord (la plus récente en tête), puis les
/// terminées (la plus récente en tête), puis celles à venir.
pub fn listing(stored: &[Campaign], today: &str) -> Vec<Campaign> {
    let mut out: Vec<Campaign> = stored.to_vec();
    let last_year = crate::date::add_months(today, -12).unwrap_or_default();
    for day in [today, last_year.as_str()] {
        if day.is_empty() {
            continue;
        }
        let d = default_campaign(day);
        if !out.iter().any(|c| c.key == d.key) {
            out.push(d);
        }
    }
    let rank = |c: &Campaign| match c.phase(today) {
        Phase::Running => 0,
        Phase::Ended => 1,
        Phase::Upcoming => 2,
    };
    out.sort_by(|a, b| {
        rank(a).cmp(&rank(b)).then_with(|| match a.phase(today) {
            Phase::Upcoming => a.starts_on.cmp(&b.starts_on),
            Phase::Running => b.starts_on.cmp(&a.starts_on),
            Phase::Ended => b.ends_on.cmp(&a.ends_on),
        })
    });
    out
}

// ---------------------------------------------------------------------
// La campagne écrite en base
// ---------------------------------------------------------------------

/// Une valeur sans `;`, `=` ni fin de ligne : les séparateurs du texte
/// d'une campagne. `%` s'échappe aussi, pour que la lecture soit sûre.
fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '%' => out.push_str("%25"),
            ';' => out.push_str("%3B"),
            '=' => out.push_str("%3D"),
            '\n' => out.push_str("%0A"),
            '\r' => out.push_str("%0D"),
            _ => out.push(c),
        }
    }
    out
}

fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('%') {
        out.push_str(&rest[..i]);
        let code = rest.get(i + 1..i + 3);
        let c = match code {
            Some("25") => Some('%'),
            Some("3B") => Some(';'),
            Some("3D") => Some('='),
            Some("0A") => Some('\n'),
            Some("0D") => Some('\r'),
            _ => None,
        };
        match c {
            Some(c) => {
                out.push(c);
                rest = &rest[i + 3..];
            }
            None => {
                out.push('%');
                rest = &rest[i + 1..];
            }
        }
    }
    out.push_str(rest);
    out
}

fn encode_pairs(pairs: &[(&str, String)]) -> String {
    pairs
        .iter()
        .filter(|(_, v)| !v.is_empty())
        .map(|(k, v)| format!("{k}={}", escape(v)))
        .collect::<Vec<_>>()
        .join(";")
}

fn decode_pairs(line: &str) -> Vec<(String, String)> {
    line.split(';')
        .filter_map(|p| p.split_once('='))
        .map(|(k, v)| (k.trim().to_owned(), unescape(v)))
        .collect()
}

fn opt_num(v: Option<u32>) -> String {
    v.map(|n| n.to_string()).unwrap_or_default()
}

/// Un nombre, ou rien quand il vaut sa valeur par défaut.
fn unless_default(n: u32, default: u32) -> String {
    if n == default {
        String::new()
    } else {
        n.to_string()
    }
}

fn flag(b: bool) -> String {
    if b { "1" } else { "" }.to_owned()
}

/// Les sujets d'une campagne en texte, une ligne par sujet de
/// `clé=valeur` séparés par des points-virgules. Une valeur par défaut
/// n'est pas écrite ; une clé inconnue est ignorée à la lecture, si bien
/// qu'une version plus récente peut en ajouter sans casser une plus
/// ancienne.
pub fn encode_subjects(subjects: &[Subject]) -> String {
    subjects
        .iter()
        .map(|s| {
            let (means, act) = match &s.means {
                Means::Vaccine => ("vaccin", String::new()),
                Means::Act(k) => ("acte", k.clone()),
                Means::Call => ("appel", String::new()),
            };
            let (coverage, months) = match s.coverage {
                Coverage::Campaign => ("campagne", String::new()),
                Coverage::Ever => ("toujours", String::new()),
                Coverage::Months(n) => ("mois", n.to_string()),
            };
            encode_pairs(&[
                ("code", s.code.clone()),
                ("libelle", s.label.clone()),
                ("moyen", means.to_owned()),
                ("acte", act),
                (
                    "cible",
                    match s.rule {
                        Rule::Calendar => "calendrier",
                        Rule::Criteria => "criteres",
                    }
                    .to_owned(),
                ),
                ("age_min", opt_num(s.age_min)),
                ("age_max", opt_num(s.age_max)),
                ("sexe", s.sex.clone()),
                ("grossesse", flag(s.pregnancy)),
                ("evocation", flag(s.evoked)),
                ("molecules", s.molecules.join(", ")),
                ("traitements", unless_default(s.min_treatments, 0)),
                ("couverture", coverage.to_owned()),
                ("mois", months),
                ("doses", unless_default(s.doses, 1)),
                ("intervalle", unless_default(s.interval_months, 0)),
                ("flacon", s.vial_product.clone()),
                ("heures", unless_default(s.vial_hours, 0)),
            ])
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Relire [`encode_subjects`]. Une ligne sans code est ignorée.
pub fn decode_subjects(text: &str) -> Vec<Subject> {
    let mut out = Vec::new();
    for line in text.lines() {
        let mut s = Subject::default();
        let mut means = "vaccin".to_owned();
        let mut act = String::new();
        let mut coverage = "campagne".to_owned();
        let mut months = 0u32;
        for (k, v) in decode_pairs(line) {
            let n = || v.trim().parse::<u32>().ok();
            match k.as_str() {
                "code" => s.code = v.trim().to_owned(),
                "libelle" => s.label = v.trim().to_owned(),
                "moyen" => means = v.clone(),
                "acte" => act = v.trim().to_owned(),
                "cible" => {
                    s.rule = if v == "calendrier" {
                        Rule::Calendar
                    } else {
                        Rule::Criteria
                    }
                }
                "age_min" => s.age_min = n(),
                "age_max" => s.age_max = n(),
                "sexe" => s.sex = v.trim().to_owned(),
                "grossesse" => s.pregnancy = v == "1",
                "evocation" => s.evoked = v == "1",
                "molecules" => s.molecules = split_list(&v),
                "traitements" => s.min_treatments = n().unwrap_or(0),
                "couverture" => coverage = v.clone(),
                "mois" => months = n().unwrap_or(0),
                "doses" => s.doses = n().unwrap_or(1).max(1),
                "intervalle" => s.interval_months = n().unwrap_or(0),
                "flacon" => s.vial_product = v.trim().to_owned(),
                "heures" => s.vial_hours = n().unwrap_or(0),
                _ => {}
            }
        }
        s.means = match means.as_str() {
            "acte" => Means::Act(act),
            "appel" => Means::Call,
            _ => Means::Vaccine,
        };
        s.coverage = match coverage.as_str() {
            "toujours" => Coverage::Ever,
            "mois" if months > 0 => Coverage::Months(months),
            _ => Coverage::Campaign,
        };
        if s.label.is_empty() {
            s.label = code_label(&s.code).to_owned();
        }
        if !s.code.is_empty() {
            out.push(s);
        }
    }
    out
}

/// Une liste tapée à virgules (ou points-virgules), sans les vides.
pub fn split_list(text: &str) -> Vec<String> {
    text.split([',', ';'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Les seuils d'une campagne en texte : stock bas, péremption, tranches.
pub fn encode_settings(c: &Campaign) -> String {
    encode_pairs(&[
        ("stock", c.low_stock.to_string()),
        ("peremption", c.expiry_days.to_string()),
        (
            "tranches",
            if c.bands.is_empty() {
                "aucune".to_owned()
            } else {
                c.bands
                    .iter()
                    .map(u32::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            },
        ),
    ])
}

/// Relire [`encode_settings`] dans `c` ; ce qui manque garde sa valeur
/// par défaut.
pub fn decode_settings(c: &mut Campaign, text: &str) {
    c.low_stock = DEFAULT_LOW_STOCK;
    c.expiry_days = DEFAULT_EXPIRY_DAYS;
    c.bands = DEFAULT_BANDS.to_vec();
    for (k, v) in decode_pairs(text) {
        match k.as_str() {
            "stock" => {
                if let Ok(n) = v.trim().parse() {
                    c.low_stock = n;
                }
            }
            "peremption" => {
                if let Ok(n) = v.trim().parse() {
                    c.expiry_days = n;
                }
            }
            "tranches" if v == "aucune" => c.bands.clear(),
            "tranches" => {
                if let Some(b) = parse_bands(&v) {
                    c.bands = b;
                }
            }
            _ => {}
        }
    }
}

/// Des âges tapés (« 65, 75 ») : croissants, sans doublon, au plus
/// six ; `None` si l'un n'est pas un âge.
pub fn parse_bands(text: &str) -> Option<Vec<u32>> {
    let mut out: Vec<u32> = Vec::new();
    for part in split_list(text) {
        let n: u32 = part.parse().ok()?;
        if !(1..=120).contains(&n) {
            return None;
        }
        out.push(n);
    }
    out.sort_unstable();
    out.dedup();
    out.truncate(6);
    Some(out)
}

/// Ce qui ne va pas dans une campagne avant de l'enregistrer : la
/// première faute, en clair ; `None` si elle peut s'écrire.
pub fn check(c: &Campaign) -> Option<String> {
    let tr = crate::strings::tr;
    if c.title.trim().is_empty() {
        return Some(tr("camp_edit_err_title").to_owned());
    }
    if crate::date::to_days(&c.starts_on).is_none() || crate::date::to_days(&c.ends_on).is_none() {
        return Some(tr("camp_edit_err_dates").to_owned());
    }
    if c.ends_on < c.starts_on {
        return Some(tr("camp_edit_err_order").to_owned());
    }
    if c.subjects.is_empty() {
        return Some(tr("camp_edit_err_none").to_owned());
    }
    for (i, s) in c.subjects.iter().enumerate() {
        if s.code.trim().is_empty() {
            return Some(crate::strings::trf("camp_edit_err_code", i + 1));
        }
        if c.subjects[..i].iter().any(|o| o.code == s.code) {
            return Some(crate::strings::trf("camp_edit_err_twice", &s.code));
        }
        if let (Some(a), Some(b)) = (s.age_min, s.age_max) {
            if b < a {
                return Some(crate::strings::trf("camp_edit_err_ages", &s.label));
            }
        }
        if matches!(&s.means, Means::Act(k) if k.trim().is_empty()) {
            return Some(crate::strings::trf("camp_edit_err_act", &s.label));
        }
        if s.rule == Rule::Calendar && !s.is_vaccine() {
            return Some(crate::strings::trf("camp_edit_err_calendar", &s.label));
        }
    }
    None
}

/// Les vaccins dont le calendrier sait dire qui les doit : la règle
/// « calendrier » n'a de sens que pour eux. Lu en interrogeant le
/// calendrier à tous les âges, avec et sans grossesse, à `today`.
pub fn calendar_codes(today: &str) -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    let year: i64 = today.get(..4).and_then(|y| y.parse().ok()).unwrap_or(2026);
    let ddr = crate::date::add_days(today, -7 * 33).unwrap_or_default();
    for age in 0..=100 {
        let birth = format!("{:04}{}", year - age, today.get(4..).unwrap_or("-01-01"));
        for d in ["", ddr.as_str()] {
            for l in vaccines::due_lines_with(&birth, today, &[], d) {
                if l.level != DueLevel::Ok && !out.contains(&l.code) {
                    out.push(l.code);
                }
            }
        }
    }
    out
}

// ---------------------------------------------------------------------
// Les lots
// ---------------------------------------------------------------------

/// Un lot de vaccin reçu à l'officine.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Lot {
    pub id: i64,
    /// Le code du catalogue (`GRIPPE`, `COVID`…), pour que le carnet
    /// propose ce lot quand on note une dose de ce vaccin.
    pub code: String,
    /// La spécialité, telle qu'écrite sur la boîte.
    pub product: String,
    /// Le numéro de lot, tel qu'imprimé.
    pub lot: String,
    /// ISO, date de péremption ; vide si non saisie.
    pub expires_on: String,
    /// Doses reçues.
    pub received: i64,
    /// ISO, date de réception.
    pub received_on: String,
    pub remark: String,
}

/// Le numéro de lot ramené à une forme comparable : sans espaces ni
/// tirets, en majuscules. « fl25-208 » et « FL25 208 » sont le même
/// lot ; le carnet garde ce qui a été tapé, seul le compte compare.
pub fn norm_lot(lot: &str) -> String {
    lot.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_uppercase)
        .collect()
}

/// L'état d'un lot, du plus grave au plus anodin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LotState {
    /// Date de péremption dépassée : ne plus l'utiliser.
    Expired,
    /// Toutes les doses reçues sont au carnet.
    Exhausted,
    /// Périmé dans moins de [`Stock::expiry_days`] jours.
    ExpiresSoon(i64),
    /// Moins de [`Stock::low`] doses restantes.
    Low,
    Ok,
}

/// Les seuils qui disent un lot en stock bas ou proche de sa péremption.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stock {
    pub low: i64,
    pub expiry_days: i64,
}

impl Default for Stock {
    fn default() -> Self {
        Stock {
            low: DEFAULT_LOW_STOCK,
            expiry_days: DEFAULT_EXPIRY_DAYS,
        }
    }
}

/// Ce qu'un lot est devenu : doses utilisées, restantes, et son état.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LotUse {
    pub used: i64,
    /// Peut être négatif : plus de doses au carnet que de doses reçues
    /// signale une erreur de saisie (lot ou quantité), que la vue écrit.
    pub remaining: i64,
    pub state: LotState,
}

/// Lire un lot contre le nombre de doses du carnet qui le portent.
pub fn lot_use(lot: &Lot, used: i64, today: &str, stock: Stock) -> LotUse {
    let remaining = lot.received - used;
    let to_expiry = (!lot.expires_on.trim().is_empty())
        .then(|| crate::date::days_between(today, lot.expires_on.trim()))
        .flatten();
    let state = match to_expiry {
        Some(d) if d < 0 => LotState::Expired,
        _ if lot.received > 0 && remaining <= 0 => LotState::Exhausted,
        Some(d) if d <= stock.expiry_days => LotState::ExpiresSoon(d),
        _ if remaining < stock.low => LotState::Low,
        _ => LotState::Ok,
    };
    LotUse {
        used,
        remaining,
        state,
    }
}

/// Pour chaque lot, le nombre de doses du carnet qui portent son numéro.
///
/// Un même numéro reçu deux fois (deux livraisons du même lot) : les
/// doses sont comptées sur la première ligne, jusqu'à ce qu'elle soit
/// épuisée, puis sur la suivante — sinon chaque ligne compterait toutes
/// les doses et le stock paraîtrait vide deux fois.
pub fn usage(lots: &[Lot], dose_lots: &[&str]) -> Vec<i64> {
    let mut counts: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    for l in dose_lots {
        let k = norm_lot(l);
        if !k.is_empty() {
            *counts.entry(k).or_default() += 1;
        }
    }
    // Les lignes d'un même numéro, dans l'ordre de réception.
    let mut order: Vec<usize> = (0..lots.len()).collect();
    order.sort_by(|&a, &b| {
        lots[a]
            .received_on
            .cmp(&lots[b].received_on)
            .then(lots[a].id.cmp(&lots[b].id))
    });
    let mut out = vec![0; lots.len()];
    let mut last_of: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for &i in &order {
        let k = norm_lot(&lots[i].lot);
        if k.is_empty() {
            continue;
        }
        let left = counts.entry(k.clone()).or_default();
        let take = (*left).min(lots[i].received.max(0));
        out[i] = take;
        *left -= take;
        last_of.insert(k, i);
    }
    // Ce qui dépasse toutes les livraisons va sur la dernière : le reste
    // négatif y signale l'erreur au lieu de la cacher.
    for (k, i) in last_of {
        out[i] += counts.get(&k).copied().unwrap_or(0);
    }
    out
}

/// Lire une date de péremption telle qu'imprimée sur la boîte.
///
/// Les vaccins portent le plus souvent un mois et une année
/// (« 06/2027 », « 06/27 ») : la date retenue est alors le dernier jour
/// de ce mois. Une date complète passe par `db::parse_french_date`.
pub fn parse_expiry(input: &str, current_year: u32) -> Result<String, String> {
    let s = input.trim();
    let parts: Vec<&str> = s
        .split(['/', '-', '.', ' '])
        .filter(|p| !p.is_empty())
        .collect();
    if parts.len() == 2 && parts.iter().all(|p| p.chars().all(|c| c.is_ascii_digit())) {
        // « 2027-06 » : l'année d'abord.
        let parts = if parts[0].len() == 4 && parts[1].len() <= 2 {
            vec![parts[1], parts[0]]
        } else {
            parts
        };
        let m: i64 = parts[0].parse().map_err(|_| expiry_error())?;
        let y: i64 = match parts[1].len() {
            2 => 2000 + parts[1].parse::<i64>().map_err(|_| expiry_error())?,
            4 => parts[1].parse().map_err(|_| expiry_error())?,
            _ => return Err(expiry_error()),
        };
        if !(1..=12).contains(&m) {
            return Err(expiry_error());
        }
        let d = crate::date::end_of_month(y, m).ok_or_else(expiry_error)?;
        return Ok(format!("{y:04}-{m:02}-{d:02}"));
    }
    crate::db::parse_french_date(s, current_year, crate::db::YearHint::Future)
        .map_err(|_| expiry_error())
}

fn expiry_error() -> String {
    crate::strings::tr("camp_expiry_error").to_owned()
}

/// Les lots utilisables pour un vaccin, à proposer au carnet : ni
/// périmés ni épuisés, la péremption la plus proche d'abord (on écoule
/// le plus ancien).
pub fn usable_lots<'a>(lots: &'a [Lot], used: &[i64], code: &str, today: &str) -> Vec<&'a Lot> {
    let mut out: Vec<&Lot> = lots
        .iter()
        .zip(used)
        .filter(|(l, _)| code.is_empty() || l.code == code)
        .filter(|(l, &u)| {
            !matches!(
                lot_use(l, u, today, Stock::default()).state,
                LotState::Expired | LotState::Exhausted
            )
        })
        .map(|(l, _)| l)
        .collect();
    out.sort_by(|a, b| {
        let key = |l: &Lot| {
            if l.expires_on.trim().is_empty() {
                "9999".to_owned()
            } else {
                l.expires_on.clone()
            }
        };
        key(a).cmp(&key(b))
    });
    out
}

// ---------------------------------------------------------------------
// Le décompte
// ---------------------------------------------------------------------

/// Une dose lue dans un carnet, réduite à ce que le décompte demande.
#[derive(Clone, Copy, Debug)]
pub struct DoseRow<'a> {
    pub code: &'a str,
    pub label: &'a str,
    /// ISO.
    pub given_on: &'a str,
}

/// Ce qui s'est fait sur la période, tel que le décompte le lit : les
/// doses des carnets, les actes réalisés (type et jour), les appels de la
/// campagne.
#[derive(Clone, Copy, Debug, Default)]
pub struct Done<'a> {
    pub doses: &'a [DoseRow<'a>],
    pub acts: &'a [(&'a str, &'a str)],
    pub calls: &'a [Call],
}

/// Le compte d'un sujet sur la campagne.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tally {
    pub code: String,
    pub label: String,
    pub today: usize,
    /// Les sept derniers jours, aujourd'hui compris.
    pub week: usize,
    pub season: usize,
}

/// La clé sous laquelle une dose se compte : son code, ou son libellé
/// pour un vaccin saisi en texte libre.
fn row_key<'a>(code: &'a str, label: &'a str) -> &'a str {
    if code.trim().is_empty() {
        label.trim()
    } else {
        code.trim()
    }
}

/// Les jours où le sujet a été réglé, sur la période de la campagne :
/// les doses de son vaccin, ses actes réalisés, ou les dossiers dont le
/// dernier appel conclut (prévenu, fait ailleurs), au jour de cet appel.
pub fn subject_dates<'a>(
    s: &Subject,
    done: &Done<'a>,
    campaign: &Campaign,
    today: &str,
) -> Vec<&'a str> {
    let until = campaign.read_until(today);
    let inside = |d: &str| !d.is_empty() && d >= campaign.starts_on.as_str() && d <= until;
    let mut out: Vec<&'a str> = match &s.means {
        Means::Vaccine => done
            .doses
            .iter()
            .filter(|d| d.code == s.code)
            .map(|d| d.given_on.trim())
            .filter(|d| inside(d))
            .collect(),
        Means::Act(kind) => done
            .acts
            .iter()
            .filter(|(k, _)| k == kind)
            .map(|(_, d)| d.trim())
            .filter(|d| inside(d))
            .collect(),
        // Une personne, une fois : son **dernier** appel, comme le bilan
        // des appels (`call_counts`) — deux « prévenu » ne font pas deux
        // personnes, et un « prévenu » suivi d'un refus n'en fait aucune.
        Means::Call => last_calls(done.calls, &campaign.key, &s.code)
            .into_values()
            .filter(|c| matches!(c.outcome.as_str(), "prevenu" | "ailleurs"))
            .map(|c| c.called_on.trim())
            .filter(|d| inside(d))
            .collect(),
    };
    out.sort_unstable();
    out
}

fn count(code: &str, label: &str, dates: &[&str], until: &str) -> Tally {
    let week_start = crate::date::add_days(until, -6).unwrap_or_default();
    Tally {
        code: code.to_owned(),
        label: label.to_owned(),
        today: dates.iter().filter(|d| **d == until).count(),
        week: dates.iter().filter(|d| **d >= week_start.as_str()).count(),
        season: dates.len(),
    }
}

/// Le compte de la campagne : chaque sujet, dans l'ordre de la campagne
/// et même à zéro ; puis, si la campagne a des vaccins, les autres
/// vaccins faits sur la période, le plus fourni d'abord. « Aujourd'hui »
/// est le dernier jour d'une campagne passée.
///
/// Une dose sans code (vaccin saisi en texte libre) est comptée sous son
/// libellé.
pub fn tally(done: &Done, campaign: &Campaign, today: &str) -> Vec<Tally> {
    let until = campaign.read_until(today);
    let mut out: Vec<Tally> = campaign
        .subjects
        .iter()
        .map(|s| {
            count(
                &s.code,
                &s.label,
                &subject_dates(s, done, campaign, today),
                until,
            )
        })
        .collect();
    if !campaign.has_vaccines() {
        return out;
    }
    let mut others: Vec<(String, String, Vec<&str>)> = Vec::new();
    for d in done.doses {
        let date = d.given_on.trim();
        if date.is_empty() || date < campaign.starts_on.as_str() || date > until {
            continue;
        }
        let key = row_key(d.code, d.label);
        if key.is_empty()
            || campaign
                .subjects
                .iter()
                .any(|s| s.is_vaccine() && s.code == key)
        {
            continue;
        }
        match others.iter_mut().find(|o| o.0 == key) {
            Some(o) => o.2.push(date),
            None => others.push((key.to_owned(), d.label.trim().to_owned(), vec![date])),
        }
    }
    let mut extra: Vec<Tally> = others
        .iter()
        .map(|(code, label, dates)| count(code, label, dates, until))
        .collect();
    extra.sort_by(|a, b| b.season.cmp(&a.season).then(a.code.cmp(&b.code)));
    out.extend(extra);
    out
}

/// Une série semaine par semaine depuis le début de la campagne : le
/// lundi de chaque semaine et le nombre de `dates` qui y tombent. La
/// première semaine commence au lundi qui précède (ou est) le premier
/// jour ; la dernière est celle d'aujourd'hui, ou de la fin d'une
/// campagne passée.
pub fn weekly(dates: &[&str], campaign: &Campaign, today: &str) -> Vec<(String, usize)> {
    let until = campaign.read_until(today);
    let monday = |iso: &str| -> Option<String> {
        let wd = crate::date::weekday(iso)?;
        crate::date::add_days(iso, 1 - wd)
    };
    let (Some(first), Some(last)) = (monday(&campaign.starts_on), monday(until)) else {
        return Vec::new();
    };
    let mut out: Vec<(String, usize)> = Vec::new();
    let mut w = first;
    while w <= last {
        out.push((w.clone(), 0));
        match crate::date::add_days(&w, 7) {
            Some(n) => w = n,
            None => break,
        }
    }
    for date in dates {
        let date = date.trim();
        if date < campaign.starts_on.as_str() || date > until {
            continue;
        }
        if let Some(m) = monday(date) {
            if let Some(slot) = out.iter_mut().find(|(k, _)| *k == m) {
                slot.1 += 1;
            }
        }
    }
    out
}

// ---------------------------------------------------------------------
// Les rappels
// ---------------------------------------------------------------------

/// Ce que l'équipe a noté après un appel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Rendez-vous pris ou personne prévenue.
    Prevenu,
    /// Message laissé, sans réponse : reste dans la liste.
    Message,
    /// Fait ailleurs (médecin, infirmier, autre officine).
    Ailleurs,
    /// Ne souhaite pas y donner suite pour cette campagne.
    Refus,
}

impl Outcome {
    pub const ALL: [Outcome; 4] = [
        Outcome::Prevenu,
        Outcome::Message,
        Outcome::Ailleurs,
        Outcome::Refus,
    ];

    /// La clé écrite en base : stable, jamais dérivée du libellé.
    pub fn key(self) -> &'static str {
        match self {
            Outcome::Prevenu => "prevenu",
            Outcome::Message => "message",
            Outcome::Ailleurs => "ailleurs",
            Outcome::Refus => "refus",
        }
    }

    pub fn from_key(key: &str) -> Option<Outcome> {
        Outcome::ALL.into_iter().find(|o| o.key() == key)
    }

    /// Un message laissé n'a rien réglé : la personne reste à rappeler.
    pub fn closes(self) -> bool {
        !matches!(self, Outcome::Message)
    }
}

/// Un appel noté, pour une campagne (sa clé) et un sujet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Call {
    pub id: i64,
    pub patient_id: i64,
    /// La clé de la campagne (`Campaign::key`).
    pub season: String,
    pub code: String,
    /// ISO.
    pub called_on: String,
    pub outcome: String,
    pub operator: String,
}

/// Le dernier appel de chaque dossier pour un sujet d'une campagne.
fn last_calls<'a>(
    calls: &'a [Call],
    key: &str,
    code: &str,
) -> std::collections::HashMap<i64, &'a Call> {
    let mut out: std::collections::HashMap<i64, &Call> = std::collections::HashMap::new();
    for c in calls.iter().filter(|c| c.season == key && c.code == code) {
        match out.get(&c.patient_id) {
            Some(prev) if (prev.called_on.as_str(), prev.id) > (c.called_on.as_str(), c.id) => {}
            _ => {
                out.insert(c.patient_id, c);
            }
        }
    }
    out
}

/// Le bilan des appels d'un sujet : combien de dossiers ont pour dernier
/// appel chaque issue, dans l'ordre de [`Outcome::ALL`].
pub fn call_counts(calls: &[Call], key: &str, code: &str) -> [usize; 4] {
    let mut out = [0; 4];
    for c in last_calls(calls, key, code).values() {
        if let Some(i) = Outcome::ALL.iter().position(|o| o.key() == c.outcome) {
            out[i] += 1;
        }
    }
    out
}

/// Un dossier tel que la liste des rappels le lit.
#[derive(Default)]
pub struct Person<'a> {
    pub id: i64,
    pub birth: &'a str,
    pub ddr: &'a str,
    /// `F`, `M` ou vide.
    pub sex: &'a str,
    pub doses: Vec<vaccines::Dose<'a>>,
    /// Les actes réalisés : type (`db::InterviewKind::as_str`) et jour.
    pub acts: Vec<(&'a str, &'a str)>,
    /// Les traitements : nom, DCI, classe.
    pub treatments: Vec<(&'a str, &'a str, &'a str)>,
    /// Les groupes que ses traitements évoquent ([`evocations`]).
    pub evoked: Vec<(&'static str, String)>,
}

/// Une ligne de la liste des rappels.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recall {
    pub patient_id: i64,
    pub code: String,
    /// La raison : celle que le calendrier donne (« Campagne en cours,
    /// aucune dose enregistrée. »), ou les critères que le dossier
    /// remplit.
    pub detail: String,
    /// Le dernier appel de la campagne pour ce sujet, s'il y en a un :
    /// un message laissé reste dans la liste, avec sa date.
    pub last_call: Option<(String, String)>,
    /// La ligne vient d'un traitement évocateur et non des critères :
    /// l'indication est à confirmer avec le patient.
    pub evoked: bool,
}

/// Les jours de ce qui règle le sujet pour ce dossier, sur la durée que
/// le sujet compte, jusqu'à `today` : les doses de son vaccin ou ses
/// actes réalisés. Un appel ne laisse rien à compter.
fn settled_on<'a>(p: &'a Person, s: &Subject, campaign: &Campaign, today: &str) -> Vec<&'a str> {
    let from: String = match s.coverage {
        Coverage::Campaign => campaign.starts_on.clone(),
        Coverage::Ever => String::new(),
        Coverage::Months(n) => crate::date::add_months(today, -i64::from(n)).unwrap_or_default(),
    };
    // Une dose sans date compte pour « une fois pour toutes » : elle a
    // été faite, on ne sait quand.
    let keep = |d: &str| {
        if d.is_empty() {
            from.is_empty()
        } else {
            d >= from.as_str() && d <= today
        }
    };
    let mut out: Vec<&str> = match &s.means {
        Means::Vaccine => p
            .doses
            .iter()
            .filter(|d| d.code == s.code && keep(d.date))
            .map(|d| d.date)
            .collect(),
        Means::Act(kind) => p
            .acts
            .iter()
            .filter(|(k, d)| k == kind && keep(d))
            .map(|(_, d)| *d)
            .collect(),
        Means::Call => Vec::new(),
    };
    out.sort_unstable();
    out
}

/// Le premier traitement du dossier qui porte l'un des fragments, hors
/// formes locales.
fn treatment_with<'a>(p: &Person<'a>, molecules: &[String]) -> Option<&'a str> {
    let folded: Vec<String> = molecules
        .iter()
        .map(|m| crate::fuzzy::sort_key(m))
        .collect();
    p.treatments
        .iter()
        .filter(|(_, _, class)| !crate::classes::is_local_form(class))
        .find(|(name, dci, _)| {
            let (n, d) = (crate::fuzzy::sort_key(name), crate::fuzzy::sort_key(dci));
            folded.iter().filter(|f| !f.is_empty()).any(|f| {
                crate::fuzzy::contains_folded(&d, f) || crate::fuzzy::contains_folded(&n, f)
            })
        })
        .map(|(name, _, _)| *name)
}

/// Le nombre de traitements distincts du dossier (par DCI, sinon par
/// nom), hors formes locales.
fn treatment_count(p: &Person) -> usize {
    let mut seen: Vec<String> = p
        .treatments
        .iter()
        .filter(|(_, _, class)| !crate::classes::is_local_form(class))
        .map(|(name, dci, _)| {
            crate::fuzzy::sort_key(if dci.trim().is_empty() { name } else { dci })
        })
        .filter(|k| !k.is_empty())
        .collect();
    seen.sort_unstable();
    seen.dedup();
    seen.len()
}

/// Les critères d'un sujet lus sur un dossier : `Some(raisons)` s'il est
/// visé, avec ce qui le désigne ; `None` sinon. `age_ok` dit à part si
/// seul l'âge l'écarte, pour la liste des traitements évocateurs.
fn criteria(p: &Person, s: &Subject, today: &str) -> (Option<Vec<String>>, bool) {
    use crate::strings::{trf, trn};
    let age = crate::db::age_on(p.birth, today);
    let mut why: Vec<String> = Vec::new();
    // La grossesse inclut, quels que soient l'âge et le reste.
    if s.pregnancy {
        if let Some(sa) = vaccines::weeks_of_amenorrhea(p.ddr, today) {
            return (Some(vec![trf("camp_why_pregnancy", sa)]), true);
        }
    }
    let bounded = s.age_min.is_some() || s.age_max.is_some();
    let age_ok = match age {
        Some(a) => s.age_min.is_none_or(|m| a >= m) && s.age_max.is_none_or(|m| a <= m),
        None => !bounded,
    };
    if bounded {
        if let Some(a) = age {
            why.push(trf("camp_why_age", a));
        }
    }
    if !s.sex.is_empty() && !p.sex.eq_ignore_ascii_case(&s.sex) {
        return (None, false);
    }
    if !s.molecules.is_empty() {
        match treatment_with(p, &s.molecules) {
            Some(name) => why.push(trf("camp_why_treatment", name)),
            None => return (None, false),
        }
    }
    if s.min_treatments > 0 {
        let n = treatment_count(p);
        if n < s.min_treatments as usize {
            return (None, false);
        }
        why.push(trn("camp_why_treatments", &[&n]));
    }
    if !age_ok {
        return (None, true);
    }
    (Some(why), true)
}

/// Les dossiers à rappeler pour le sujet `code` de la campagne : visés
/// (par le calendrier ou par les critères), sans rien qui le règle sur
/// la durée qu'il compte, et sans appel qui ait clos la question pour
/// cette campagne. Les plus âgés d'abord — c'est l'ordre du risque, et
/// celui dans lequel on appelle.
pub fn recalls(
    people: &[Person],
    calls: &[Call],
    campaign: &Campaign,
    code: &str,
    today: &str,
) -> Vec<Recall> {
    let Some(s) = campaign.subject(code) else {
        return Vec::new();
    };
    let last = last_calls(calls, &campaign.key, code);
    // **Une campagne préparée d'avance se lit à son premier jour.** Lu
    // au 20 août, le calendrier comptait la grippe de l'hiver passé pour
    // celle de la saison (elle commence au 1er septembre d'après la date
    // donnée), et une personne vaccinée en octobre dernier n'était pas
    // listée pour la campagne d'octobre prochain. Les âges et les critères
    // se lisent au même jour ; ce qui règle le sujet, lui, se lit à
    // aujourd'hui — aucune dose n'est encore faite dans l'avenir.
    let at = if campaign.starts_on.as_str() > today {
        campaign.starts_on.as_str()
    } else {
        today
    };
    let mut out: Vec<(String, Recall)> = Vec::new();
    for p in people {
        let (listed, evoked, mut detail) = match s.rule {
            Rule::Calendar => {
                let lines = vaccines::due_lines_with(p.birth, at, &p.doses, p.ddr);
                // **La ligne la plus pressante du code**, pas la première :
                // pendant une grossesse, le calendrier garde la ligne du
                // rappel de l'adulte et ajoute « dTcaP — grossesse » sous
                // le même code. La première disait « à jour » ou « à
                // demander », et une femme enceinte de 31 semaines sans
                // dose depuis le début de sa grossesse n'était pas listée.
                let urgency = |l: &&vaccines::DueLine| match l.level {
                    DueLevel::Due => 0,
                    DueLevel::Ask => 1,
                    DueLevel::Ok => 2,
                };
                let Some(line) = lines.iter().filter(|l| l.code == code).min_by_key(urgency) else {
                    continue;
                };
                // Dû par l'âge ou la grossesse ; sinon, si le sujet lit les
                // traitements, évoqué par l'un d'eux — une question ouverte
                // que le calendrier laisse en « à demander ».
                let evoked = line.level == DueLevel::Ask && s.evoked && !p.evoked.is_empty();
                (
                    line.level == DueLevel::Due || evoked,
                    evoked,
                    line.detail.clone(),
                )
            }
            Rule::Criteria => match criteria(p, s, at) {
                (Some(why), _) => (true, false, why.join(" ; ")),
                (None, true) if s.evoked && !p.evoked.is_empty() => (true, true, String::new()),
                _ => (false, false, String::new()),
            },
        };
        if !listed {
            continue;
        }
        // Ce qui règle le sujet : assez de doses ou d'actes sur la durée,
        // ou, au milieu d'une série, la suivante pas encore due.
        let done = settled_on(p, s, campaign, today);
        if done.len() >= s.doses.max(1) as usize {
            continue;
        }
        if let Some(prev) = done.last().filter(|d| !d.is_empty()) {
            if s.interval_months > 0
                && crate::date::add_months(prev, i64::from(s.interval_months))
                    .is_some_and(|from| from.as_str() > today)
            {
                continue;
            }
        }
        let call = last.get(&p.id);
        if call.is_some_and(|c| Outcome::from_key(&c.outcome).is_some_and(Outcome::closes)) {
            continue;
        }
        if evoked {
            let named: Vec<String> = p
                .evoked
                .iter()
                .map(|(group, drug)| format!("{group} ({drug})"))
                .collect();
            detail = crate::strings::trf("camp_evoked", named.join(" ; "));
        } else if s.rule == Rule::Criteria {
            let mut parts: Vec<String> = Vec::new();
            if !detail.is_empty() {
                parts.push(detail);
            }
            let vaccine = s.is_vaccine();
            parts.push(match (&s.means, done.len()) {
                (Means::Call, _) => crate::strings::tr("camp_why_to_call").to_owned(),
                (_, 0) if vaccine => crate::strings::tr("camp_why_no_dose").to_owned(),
                (_, 0) => crate::strings::tr("camp_why_no_act").to_owned(),
                (_, n) => crate::strings::trn(
                    if vaccine {
                        "camp_why_series_dose"
                    } else {
                        "camp_why_series_act"
                    },
                    &[&n, &s.doses, &crate::db::format_french_date(done[n - 1])],
                ),
            });
            detail = capitalize(&parts.join(" ; "));
            if !detail.ends_with('.') {
                detail.push('.');
            }
        }
        out.push((
            p.birth.to_owned(),
            Recall {
                patient_id: p.id,
                code: s.code.clone(),
                detail,
                last_call: call.map(|c| (c.called_on.clone(), c.outcome.clone())),
                evoked,
            },
        ));
    }
    // Les dossiers visés d'abord, puis les traitements évocateurs ; dans
    // chaque groupe, la naissance la plus ancienne d'abord, sans date en
    // dernier.
    out.sort_by(|a, b| {
        (a.1.evoked, a.0.is_empty(), a.0.as_str(), a.1.patient_id).cmp(&(
            b.1.evoked,
            b.0.is_empty(),
            b.0.as_str(),
            b.1.patient_id,
        ))
    });
    out.into_iter().map(|(_, r)| r).collect()
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().chain(c).collect(),
        None => String::new(),
    }
}

// ---------------------------------------------------------------------
// Les flacons multidoses ouverts
// ---------------------------------------------------------------------

/// Un flacon ouvert : le lot, le jour et la minute de la première
/// ponction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Vial {
    pub id: i64,
    pub lot: String,
    /// ISO.
    pub opened_on: String,
    /// Minutes depuis minuit.
    pub opened_min: i64,
    pub operator: String,
}

/// Ce qu'il reste d'un flacon utilisable `hours` heures : l'heure limite
/// (jour ISO et minute) et les minutes restantes à `today`/`now_min`,
/// négatives une fois le délai dépassé.
pub fn vial_left(v: &Vial, hours: u32, today: &str, now_min: i64) -> Option<(String, i64, i64)> {
    let opened = crate::date::to_days(&v.opened_on)? * 1440 + v.opened_min;
    let now = crate::date::to_days(today)? * 1440 + now_min;
    let end = opened + i64::from(hours) * 60;
    let day = crate::date::from_days(end.div_euclid(1440));
    Some((day, end.rem_euclid(1440), end - now))
}

/// Une heure du jour écrite à la française : « 9 h 05 ».
pub fn hm(minutes: i64) -> String {
    let m = minutes.rem_euclid(1440);
    format!("{} h {:02}", m / 60, m % 60)
}

// ---------------------------------------------------------------------
// Le bilan de la campagne
// ---------------------------------------------------------------------

/// Les doses d'un vaccin sur la campagne, par tranche d'âge à la date de
/// l'injection (`Campaign::bands`), puis l'âge inconnu (pas de date de
/// naissance au dossier).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Summary {
    pub code: String,
    pub label: String,
    /// Une case de plus que de bornes : avant la première, entre deux,
    /// à partir de la dernière.
    pub by_band: Vec<usize>,
    pub unknown: usize,
}

impl Summary {
    pub fn total(&self) -> usize {
        self.by_band.iter().sum::<usize>() + self.unknown
    }
}

/// Les en-têtes des tranches d'âge : « Moins de 65 ans », « 65 à 74
/// ans », « 75 ans et plus ». Sans borne, une seule : « Tous âges ».
pub fn band_labels(bands: &[u32]) -> Vec<String> {
    use crate::strings::{trf, trn};
    if bands.is_empty() {
        return vec![crate::strings::tr("camp_band_all").to_owned()];
    }
    let mut out = vec![trf("camp_band_under", bands[0])];
    for w in bands.windows(2) {
        out.push(trn("camp_band_between", &[&w[0], &(w[1] - 1)]));
    }
    out.push(trf("camp_band_from", bands[bands.len() - 1]));
    out
}

/// Le bilan de la campagne. `rows` : naissance (ISO, vide si inconnue),
/// code, libellé, date de la dose. Une campagne sans vaccin (des
/// entretiens, un dépistage) n'a pas de doses à compter : comme son
/// compte (`tally`), son bilan ne reprend pas la grippe de la saison.
pub fn summary(
    rows: &[(&str, &str, &str, &str)],
    campaign: &Campaign,
    today: &str,
) -> Vec<Summary> {
    if !campaign.has_vaccines() {
        return Vec::new();
    }
    let until = campaign.read_until(today);
    let bands = &campaign.bands;
    let mut out: Vec<Summary> = Vec::new();
    for (birth, code, label, given) in rows {
        let date = given.trim();
        if date.is_empty() || date < campaign.starts_on.as_str() || date > until {
            continue;
        }
        let key = row_key(code, label);
        if key.is_empty() {
            continue;
        }
        let i = match out.iter().position(|x| x.code == key) {
            Some(i) => i,
            None => {
                let label = campaign
                    .subject(key)
                    .map(|s| s.label.clone())
                    .unwrap_or_else(|| label.trim().to_owned());
                out.push(Summary {
                    code: key.to_owned(),
                    label,
                    by_band: vec![0; bands.len() + 1],
                    unknown: 0,
                });
                out.len() - 1
            }
        };
        let row = &mut out[i];
        match crate::db::age_on(birth, date) {
            Some(a) => {
                let band = bands.iter().take_while(|b| a >= **b).count();
                row.by_band[band] += 1;
            }
            None => row.unknown += 1,
        }
    }
    out.sort_by(|a, b| b.total().cmp(&a.total()).then(a.code.cmp(&b.code)));
    out
}

// ---------------------------------------------------------------------
// Les traitements évocateurs d'une indication
// ---------------------------------------------------------------------

/// Une molécule dont la présence dans les traitements d'un dossier
/// évoque l'un des groupes que le calendrier vise pour la grippe et le
/// COVID-19 avant 65 ans. **Une évocation, jamais un diagnostic** : la
/// ligne de rappel la présente « à confirmer » avec le patient.
pub struct Evocation {
    /// Fragment de DCI, en minuscules sans accents.
    pub dci: &'static str,
    /// Le groupe du calendrier qu'elle évoque.
    pub group: &'static str,
    /// Si non vide : la classe de la fiche doit contenir ce fragment
    /// (un corticoïde *inhalé*, et non nasal ou cutané).
    pub class_needs: &'static str,
}

const DIAB: &str = "diabète";
const RESP: &str = "maladie respiratoire chronique (asthme, BPCO)";
const CORO: &str = "maladie coronaire ou antécédent d'AVC";
const RYTHME: &str = "trouble du rythme traité au long cours";
const IC: &str = "insuffisance cardiaque";
const IMMUNO: &str = "immunodépression (traitement immunosuppresseur)";
const CANCER: &str = "cancer ou hémopathie sous traitement";
const VIH: &str = "infection par le VIH";

const fn ev(dci: &'static str, group: &'static str) -> Evocation {
    Evocation {
        dci,
        group,
        class_needs: "",
    }
}

/// Les molécules lues, groupe par groupe. Le fragment est une
/// sous-chaîne de la DCI : la liste des fiches livrées que chacun
/// attrape est tenue par le test `every_evocation_catches_what_it_says`.
pub const EVOCATIONS: &[Evocation] = &[
    // Diabète. Les analogues du GLP-1 de l'obésité sont lus à part.
    ev("insuline", DIAB),
    ev("metformine", DIAB),
    ev("gliclazide", DIAB),
    ev("glimepiride", DIAB),
    ev("glibenclamide", DIAB),
    ev("repaglinide", DIAB),
    ev("gliptine", DIAB),
    ev("acarbose", DIAB),
    ev("dulaglutide", DIAB),
    ev("exenatide", DIAB),
    // Les gliflozines traitent aussi l'insuffisance cardiaque et la
    // maladie rénale chronique : toutes sont des groupes visés.
    ev(
        "gliflozine",
        "diabète, insuffisance cardiaque ou maladie rénale chronique",
    ),
    // Corticoïdes inhalés, bronchodilatateurs de longue durée, et le
    // reste du traitement de fond de l'asthme et de la BPCO.
    Evocation {
        dci: "budesonide",
        group: RESP,
        class_needs: "inhal",
    },
    Evocation {
        dci: "beclometasone",
        group: RESP,
        class_needs: "inhal",
    },
    Evocation {
        dci: "fluticasone",
        group: RESP,
        class_needs: "inhal",
    },
    Evocation {
        dci: "ciclesonide",
        group: RESP,
        class_needs: "inhal",
    },
    ev("formoterol", RESP),
    ev("salmeterol", RESP),
    ev("vilanterol", RESP),
    ev("indacaterol", RESP),
    ev("olodaterol", RESP),
    ev("tiotropium", RESP),
    ev("glycopyrronium", RESP),
    ev("umeclidinium", RESP),
    ev("aclidinium", RESP),
    ev("montelukast", RESP),
    ev("omalizumab", RESP),
    ev("mepolizumab", RESP),
    ev("benralizumab", RESP),
    ev("tezepelumab", RESP),
    ev("theophylline", RESP),
    ev("roflumilast", RESP),
    ev("salbutamol", RESP),
    ev("terbutaline", RESP),
    ev("ivacaftor", "mucoviscidose"),
    // Cœur.
    ev("sacubitril", IC),
    ev("ivabradine", IC),
    ev("vericiguat", IC),
    ev("digoxine", "insuffisance cardiaque ou trouble du rythme"),
    ev("clopidogrel", CORO),
    ev("ticagrelor", CORO),
    ev("prasugrel", CORO),
    ev("isosorbide", CORO),
    ev("trinitrine", CORO),
    ev("nicorandil", CORO),
    ev("ranolazine", CORO),
    ev("amiodarone", RYTHME),
    ev("flecainide", RYTHME),
    ev("dronedarone", RYTHME),
    // Un anticoagulant oral traite aussi une thrombose, sur quelques
    // mois : le trouble du rythme est à confirmer.
    ev("apixaban", RYTHME),
    ev("rivaroxaban", RYTHME),
    ev("dabigatran", RYTHME),
    ev("edoxaban", RYTHME),
    ev("warfarine", RYTHME),
    ev("fluindione", RYTHME),
    ev("acenocoumarol", RYTHME),
    // Immunodépression.
    ev("methotrexate", IMMUNO),
    ev("azathioprine", IMMUNO),
    ev("mycophenol", IMMUNO),
    ev("ciclosporine", IMMUNO),
    ev("tacrolimus", IMMUNO),
    ev("everolimus", IMMUNO),
    ev("sirolimus", IMMUNO),
    ev("cyclophosphamide", IMMUNO),
    ev("adalimumab", IMMUNO),
    ev("etanercept", IMMUNO),
    ev("infliximab", IMMUNO),
    ev("certolizumab", IMMUNO),
    ev("golimumab", IMMUNO),
    ev("tocilizumab", IMMUNO),
    ev("sarilumab", IMMUNO),
    ev("abatacept", IMMUNO),
    ev("rituximab", IMMUNO),
    ev("ocrelizumab", IMMUNO),
    ev("ofatumumab", IMMUNO),
    ev("ustekinumab", IMMUNO),
    ev("secukinumab", IMMUNO),
    ev("ixekizumab", IMMUNO),
    ev("guselkumab", IMMUNO),
    ev("risankizumab", IMMUNO),
    ev("tofacitinib", IMMUNO),
    ev("baricitinib", IMMUNO),
    ev("upadacitinib", IMMUNO),
    ev("filgotinib", IMMUNO),
    ev("fingolimod", IMMUNO),
    ev("siponimod", IMMUNO),
    ev("ozanimod", IMMUNO),
    ev("natalizumab", IMMUNO),
    ev("teriflunomide", IMMUNO),
    ev("leflunomide", IMMUNO),
    ev("capecitabine", CANCER),
    ev("imatinib", CANCER),
    ev("ibrutinib", CANCER),
    ev("ruxolitinib", CANCER),
    ev("lenalidomide", CANCER),
    ev("hydroxycarbamide", "drépanocytose ou hémopathie"),
    // VIH : les molécules qui ne servent qu'au traitement. L'association
    // emtricitabine-ténofovir seule est aussi la prophylaxie
    // préexposition, et le ténofovir seul traite l'hépatite B : ni l'une
    // ni l'autre n'est lue.
    ev("dolutegravir", VIH),
    ev("bictegravir", VIH),
    ev("raltegravir", VIH),
    ev("elvitegravir", VIH),
    ev("cabotegravir", VIH),
    ev("darunavir", VIH),
    ev("rilpivirine", VIH),
    ev("doravirine", VIH),
    ev("abacavir", VIH),
];

/// [`evocations`], avec la lecture de chaque médicament gardée d'un
/// dossier à l'autre : une officine a des milliers de dossiers et
/// quelques centaines de médicaments, et la liste des rappels se relit à
/// chaque appel noté.
pub fn evocations_memo(
    treatments: &[(&str, &str, &str)],
    memo: &mut std::collections::HashMap<(String, String), Vec<&'static str>>,
) -> Vec<(&'static str, String)> {
    let mut out: Vec<(&'static str, String)> = Vec::new();
    for t in treatments {
        // La lecture ne dépend que de la DCI et de la classe : deux fiches
        // du même nom peuvent différer, deux noms d'une même molécule non.
        let groups = memo
            .entry((t.1.to_owned(), t.2.to_owned()))
            .or_insert_with(|| evocations(&[*t]).into_iter().map(|(g, _)| g).collect());
        for g in groups.iter() {
            if !out.iter().any(|(x, _)| x == g) {
                out.push((g, t.0.to_owned()));
            }
        }
    }
    out
}

/// Les groupes évoqués par les traitements d'un dossier, chacun avec le
/// premier médicament qui l'évoque. `treatments` : nom, DCI, classe.
pub fn evocations(treatments: &[(&str, &str, &str)]) -> Vec<(&'static str, String)> {
    let mut out: Vec<(&'static str, String)> = Vec::new();
    for (name, dci, class) in treatments {
        if crate::classes::is_local_form(class) {
            continue;
        }
        let d = crate::fuzzy::sort_key(dci);
        let c = crate::fuzzy::sort_key(class);
        for e in EVOCATIONS {
            if !crate::fuzzy::contains_folded(&d, e.dci) {
                continue;
            }
            if !e.class_needs.is_empty() && !crate::fuzzy::contains_folded(&c, e.class_needs) {
                continue;
            }
            if !out.iter().any(|(g, _)| *g == e.group) {
                out.push((e.group, (*name).to_owned()));
            }
        }
    }
    out
}

/// Le jour d'un rendez-vous tapé sans année (« 05/01 ») et déjà passé
/// cette année tombe l'année suivante : le 20 décembre, « 05/01 » est le
/// 5 janvier qui vient. Une année tapée est gardée telle quelle.
pub fn rdv_day(parsed: &str, typed: &str, today: &str) -> String {
    let digits = typed.chars().filter(char::is_ascii_digit).count();
    if digits > 4 || parsed >= today {
        return parsed.to_owned();
    }
    match (
        parsed.get(..4).and_then(|y| y.parse::<u32>().ok()),
        parsed.get(4..),
    ) {
        (Some(y), Some(rest)) => format!("{:04}{rest}", y + 1),
        _ => parsed.to_owned(),
    }
}

/// L'export de la saison, en CSV pour un tableur français : point-virgule,
/// marque d'ordre des octets, dates en JJ/MM/AAAA, champs entre guillemets
/// quand ils portent un séparateur. Une ligne par dose, dans l'ordre de
/// [`crate::db::Db::vaccinations_season`].
pub fn season_csv(rows: &[[String; 10]]) -> String {
    fn field(s: &str) -> String {
        if s.contains([';', '"', '\n', '\r']) {
            format!("\"{}\"", s.replace('"', "\"\""))
        } else {
            s.to_owned()
        }
    }
    let mut out =
        String::from("\u{feff}Date;Nom;Prénom;Naissance;Vaccin;Code;Dose;Lot;Site;Par\r\n");
    for r in rows {
        let cells: Vec<String> = r
            .iter()
            .enumerate()
            .map(|(i, v)| match i {
                0 | 3 => crate::db::format_french_date(v),
                _ => field(v),
            })
            .collect();
        out.push_str(&cells.join(";"));
        out.push_str("\r\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dose(code: &'static str, date: &'static str) -> vaccines::Dose<'static> {
        vaccines::Dose { code, date }
    }

    fn person(
        id: i64,
        birth: &'static str,
        doses: Vec<vaccines::Dose<'static>>,
    ) -> Person<'static> {
        Person {
            id,
            birth,
            doses,
            ..Person::default()
        }
    }

    fn ids(r: &[Recall]) -> Vec<i64> {
        r.iter().map(|x| x.patient_id).collect()
    }

    /// **Une femme enceinte est rappelée pour le dTcaP de la grossesse**,
    /// même quand son rappel d'adulte est à jour : le calendrier porte les
    /// deux lignes sous le même code, et la première lue disait « à jour ».
    #[test]
    fn a_pregnancy_dtcap_is_recalled_even_with_the_adult_booster_done() {
        let today = "2026-10-10";
        let mut c = default_campaign(today);
        c.subjects.push(Subject {
            code: "DTCAP".into(),
            label: "dTcaP".into(),
            means: Means::Vaccine,
            rule: Rule::Calendar,
            ..Subject::default()
        });
        let pregnant = Person {
            id: 1,
            birth: "1996-01-01",
            sex: "F",
            ddr: "2026-03-01",
            doses: vec![dose("DTCAP", "2021-06-01")],
            ..Person::default()
        };
        let not = Person {
            id: 2,
            birth: "1996-01-01",
            sex: "F",
            doses: vec![dose("DTCAP", "2021-06-01")],
            ..Person::default()
        };
        assert_eq!(
            ids(&recalls(&[pregnant, not], &[], &c, "DTCAP", today)),
            vec![1]
        );
    }

    /// **Une campagne préparée d'avance se lit à son premier jour** : la
    /// grippe d'octobre dernier n'est pas celle de la saison qui vient.
    #[test]
    fn an_upcoming_campaign_reads_the_calendar_at_its_start() {
        let mut c = default_campaign("2026-10-20");
        c.starts_on = "2026-10-13".into();
        let people = vec![person(1, "1956-01-01", vec![dose("GRIPPE", "2025-10-20")])];
        assert_eq!(
            ids(&recalls(&people, &[], &c, "GRIPPE", "2026-08-20")),
            vec![1]
        );
        // Vaccinée cette saison-ci : plus rien à rappeler.
        let done = vec![person(1, "1956-01-01", vec![dose("GRIPPE", "2026-10-15")])];
        assert!(recalls(&done, &[], &c, "GRIPPE", "2026-10-20").is_empty());
    }

    /// Le pneumocoque : 65 ans et plus sans dose au carnet.
    #[test]
    fn the_pneumococcal_recall_lists_those_without_a_dose_from_65() {
        let today = "2026-10-20";
        let people = vec![
            person(1, "1955-01-01", vec![]),
            person(2, "1955-01-01", vec![dose("PNEUMO", "2026-01-10")]),
            person(3, "1975-01-01", vec![]),
        ];
        let c = default_campaign(today);
        assert_eq!(ids(&recalls(&people, &[], &c, "PNEUMO", today)), vec![1]);
    }

    /// Le zona : à 65 ans et plus sans dose, ou une première dose faite
    /// depuis deux mois au moins ; pas avant.
    #[test]
    fn a_second_shingrix_dose_is_recalled_from_two_months() {
        let today = "2026-10-20";
        let people = vec![
            person(1, "1950-01-01", vec![]),
            person(2, "1950-01-01", vec![dose("ZONA", "2026-09-30")]),
            person(3, "1950-01-01", vec![dose("ZONA", "2026-07-01")]),
            person(4, "1970-01-01", vec![]),
            person(
                5,
                "1950-01-01",
                vec![dose("ZONA", "2026-03-01"), dose("ZONA", "2026-05-01")],
            ),
        ];
        let c = default_campaign(today);
        assert_eq!(ids(&recalls(&people, &[], &c, "ZONA", today)), vec![1, 3]);
    }

    /// L'export de la saison se lit dans un tableur français : dates à la
    /// française, champ à point-virgule entre guillemets.
    #[test]
    fn the_season_export_reads_in_a_french_spreadsheet() {
        let row = |label: &str| {
            [
                "2026-10-08".to_owned(),
                "DUPONT".to_owned(),
                "Jean".to_owned(),
                "1950-02-11".to_owned(),
                label.to_owned(),
                "GRIPPE".to_owned(),
                "1".to_owned(),
                "VX26".to_owned(),
                "Deltoïde G".to_owned(),
                "MB".to_owned(),
            ]
        };
        let csv = super::season_csv(&[row("Efluelda"), row("Vaxigrip; tétra")]);
        assert!(csv.starts_with('\u{feff}'));
        assert!(csv.contains("08/10/2026;DUPONT;Jean;11/02/1950;Efluelda;GRIPPE"));
        assert!(csv.contains("\"Vaxigrip; tétra\""));
        assert_eq!(csv.matches("\r\n").count(), 3);
    }

    #[test]
    fn a_yearless_rdv_after_new_year_falls_next_year() {
        assert_eq!(rdv_day("2026-01-05", "05/01", "2026-12-20"), "2027-01-05");
        assert_eq!(rdv_day("2026-01-05", "0501", "2026-12-20"), "2027-01-05");
        // Une année tapée reste : le passé est alors refusé plus loin.
        assert_eq!(
            rdv_day("2026-01-05", "05/01/2026", "2026-12-20"),
            "2026-01-05"
        );
        assert_eq!(rdv_day("2026-12-22", "22/12", "2026-12-20"), "2026-12-22");
    }

    fn lot(id: i64, code: &str, n: &str, exp: &str, received: i64, on: &str) -> Lot {
        Lot {
            id,
            code: code.into(),
            product: "Vaccin".into(),
            lot: n.into(),
            expires_on: exp.into(),
            received,
            received_on: on.into(),
            remark: String::new(),
        }
    }

    #[test]
    fn the_season_runs_from_september_to_august() {
        let s = season("2026-10-08");
        assert_eq!(s.start, "2026-09-01");
        assert_eq!(s.end, "2027-08-31");
        assert_eq!(s.label, "2026-2027");
        // En février, c'est encore la saison commencée l'automne d'avant.
        assert_eq!(season("2027-02-14").label, "2026-2027");
        assert_eq!(season("2027-09-01").label, "2027-2028");
    }

    /// **Chaque hiver a sa campagne sans qu'on y touche** : rien n'est
    /// enregistré, et la campagne de la saison se construit pour l'année
    /// où tombe la date — 2026, 2031 ou 2040.
    #[test]
    fn every_winter_has_its_campaign_without_a_row() {
        for (today, key, start) in [
            ("2026-10-08", "2026-2027", "2026-09-01"),
            ("2032-01-15", "2031-2032", "2031-09-01"),
            ("2040-09-01", "2040-2041", "2040-09-01"),
        ] {
            let c = default_campaign(today);
            assert_eq!((c.key.as_str(), c.starts_on.as_str()), (key, start));
            assert!(c.title.contains(key), "{}", c.title);
            assert_eq!(c.phase(today), Phase::Running);
            let codes: Vec<&str> = c.subjects.iter().map(|s| s.code.as_str()).collect();
            assert_eq!(codes, SEASON_CODES);
        }
    }

    #[test]
    fn a_lot_number_is_compared_without_its_typing() {
        assert_eq!(norm_lot(" fl25-208 "), "FL25208");
        assert_eq!(norm_lot("FL25 208"), norm_lot("fl25208"));
        assert_eq!(norm_lot(" - "), "");
    }

    #[test]
    fn a_lot_reads_its_state_most_serious_first() {
        let today = "2026-10-08";
        let st = Stock::default();
        let l = lot(1, "GRIPPE", "A1", "2027-06-30", 20, "2026-10-01");
        assert_eq!(lot_use(&l, 3, today, st).state, LotState::Ok);
        assert_eq!(lot_use(&l, 3, today, st).remaining, 17);
        assert_eq!(lot_use(&l, 16, today, st).state, LotState::Low);
        assert_eq!(lot_use(&l, 20, today, st).state, LotState::Exhausted);
        // Plus de doses au carnet que reçues : épuisé, et le reste
        // négatif dit l'erreur.
        assert_eq!(lot_use(&l, 22, today, st).remaining, -2);
        let soon = lot(2, "GRIPPE", "A2", "2026-10-20", 20, "2026-10-01");
        assert_eq!(
            lot_use(&soon, 0, today, st).state,
            LotState::ExpiresSoon(12)
        );
        let gone = lot(3, "GRIPPE", "A3", "2026-10-07", 20, "2026-10-01");
        // Périmé l'emporte sur tout, même s'il reste des doses.
        assert_eq!(lot_use(&gone, 0, today, st).state, LotState::Expired);
        // Sans date de péremption : jamais dit périmé.
        let undated = lot(4, "GRIPPE", "A4", "", 10, "");
        assert_eq!(lot_use(&undated, 0, today, st).state, LotState::Ok);
        // Les seuils sont ceux de la campagne : 10 doses, 7 jours.
        let strict = Stock {
            low: 10,
            expiry_days: 7,
        };
        assert_eq!(lot_use(&l, 12, today, strict).state, LotState::Low);
        assert_eq!(lot_use(&soon, 0, today, strict).state, LotState::Ok);
    }

    #[test]
    fn doses_are_counted_against_the_lot_they_carry() {
        let lots = [
            lot(1, "GRIPPE", "FL25-208", "2027-06-30", 10, "2026-10-01"),
            lot(2, "COVID", "CV9", "2027-01-31", 6, "2026-10-01"),
        ];
        let doses = ["fl25 208", "FL25208", "cv9", "", "INCONNU"];
        assert_eq!(usage(&lots, &doses), vec![2, 1]);
    }

    #[test]
    fn a_lot_delivered_twice_is_drawn_down_in_order() {
        // Deux livraisons du même numéro : la première s'épuise avant
        // que la seconde ne serve, et le trop-plein va sur la dernière.
        let lots = [
            lot(2, "GRIPPE", "X", "2027-06-30", 5, "2026-10-15"),
            lot(1, "GRIPPE", "X", "2027-06-30", 3, "2026-10-01"),
        ];
        let doses = vec!["X"; 4];
        assert_eq!(usage(&lots, &doses), vec![1, 3]);
        let doses = vec!["X"; 10];
        // 3 sur la première, 5 sur la seconde, 2 de trop sur la dernière.
        assert_eq!(usage(&lots, &doses), vec![7, 3]);
    }

    #[test]
    fn the_counter_offers_usable_lots_oldest_expiry_first() {
        let today = "2026-10-08";
        let lots = [
            lot(1, "GRIPPE", "LATE", "2027-06-30", 10, "2026-10-01"),
            lot(2, "GRIPPE", "EARLY", "2027-01-31", 10, "2026-10-01"),
            lot(3, "GRIPPE", "GONE", "2026-09-30", 10, "2026-09-01"),
            lot(4, "GRIPPE", "EMPTY", "2027-06-30", 2, "2026-10-01"),
            lot(5, "COVID", "OTHER", "2027-01-01", 10, "2026-10-01"),
        ];
        let used = [0, 0, 0, 2, 0];
        let names: Vec<&str> = usable_lots(&lots, &used, "GRIPPE", today)
            .iter()
            .map(|l| l.lot.as_str())
            .collect();
        assert_eq!(names, ["EARLY", "LATE"]);
    }

    #[test]
    fn the_tally_counts_the_season_the_week_and_the_day() {
        let today = "2026-10-08";
        let d = |code, date| DoseRow {
            code,
            label: "",
            given_on: date,
        };
        let doses = [
            d("GRIPPE", "2026-10-08"),
            d("GRIPPE", "2026-10-02"),
            d("GRIPPE", "2026-09-15"),
            // Saison passée : ne compte pas.
            d("GRIPPE", "2026-02-01"),
            // Une date à venir (erreur de saisie) : ne compte pas.
            d("GRIPPE", "2026-10-09"),
            d("COVID", "2026-10-08"),
            d("DTCAP", "2026-10-07"),
            d("", "2026-10-08"),
        ];
        let c = default_campaign(today);
        let t = tally(
            &Done {
                doses: &doses,
                ..Done::default()
            },
            &c,
            today,
        );
        // Les cinq sujets dans leur ordre, même à zéro, puis le dTcaP.
        let codes: Vec<&str> = t.iter().map(|x| x.code.as_str()).collect();
        assert_eq!(codes, ["GRIPPE", "COVID", "VRS", "PNEUMO", "ZONA", "DTCAP"]);
        assert_eq!((t[0].today, t[0].week, t[0].season), (1, 2, 3));
        assert_eq!((t[1].today, t[1].week, t[1].season), (1, 1, 1));
        assert_eq!(t[2].season, 0);
        assert_eq!(t[0].label, code_label("GRIPPE"));
    }

    /// Une campagne sans vaccin compte ses actes et ses appels conclus, et
    /// rien des carnets.
    #[test]
    fn a_campaign_without_vaccines_counts_its_acts_and_calls() {
        let today = "2026-11-10";
        let mut c = custom("2026-10-01", "2026-12-31");
        c.subjects = vec![
            Subject {
                code: "AVK".into(),
                label: "Entretien AVK".into(),
                means: Means::Act("AVK".into()),
                ..Subject::default()
            },
            Subject {
                code: "COLON".into(),
                label: "Dépistage".into(),
                means: Means::Call,
                ..Subject::default()
            },
        ];
        let acts = [
            ("AVK", "2026-11-10"),
            ("AVK", "2026-09-30"),
            ("BPM", "2026-11-10"),
        ];
        let call = |id, outcome: &str, day: &str| Call {
            id,
            patient_id: id,
            season: c.key.clone(),
            code: "COLON".into(),
            called_on: day.into(),
            outcome: outcome.into(),
            operator: String::new(),
        };
        let calls = [
            call(1, "prevenu", "2026-11-09"),
            call(2, "message", "2026-11-09"),
            call(3, "ailleurs", "2026-10-02"),
        ];
        let doses = [DoseRow {
            code: "GRIPPE",
            label: "",
            given_on: "2026-11-10",
        }];
        let done = Done {
            doses: &doses,
            acts: &acts,
            calls: &calls,
        };
        let t = tally(&done, &c, today);
        assert_eq!(t.len(), 2, "la grippe n'est pas de cette campagne");
        assert_eq!((t[0].today, t[0].season), (1, 1));
        assert_eq!((t[1].week, t[1].season), (1, 2));
        let w = weekly(&subject_dates(&c.subjects[1], &done, &c, today), &c, today);
        assert_eq!(w.iter().map(|x| x.1).sum::<usize>(), 2);
    }

    #[test]
    fn the_weekly_series_starts_on_the_monday_of_september() {
        let c = default_campaign("2026-10-08");
        let w = weekly(&["2026-10-08", "2026-10-05"], &c, "2026-10-08");
        // Le 1er septembre 2026 est un mardi : la série part du lundi 31 août.
        assert_eq!(w.first().map(|x| x.0.as_str()), Some("2026-08-31"));
        assert_eq!(w.last(), Some(&("2026-10-05".to_owned(), 2)));
        assert_eq!(w.iter().map(|x| x.1).sum::<usize>(), 2);
    }

    /// Une campagne passée se lit jusqu'à son dernier jour, et pas au-delà.
    #[test]
    fn an_ended_campaign_reads_until_its_last_day() {
        let c = custom("2026-10-01", "2026-10-31");
        let today = "2027-03-01";
        assert_eq!(c.phase(today), Phase::Ended);
        assert_eq!(c.read_until(today), "2026-10-31");
        let w = weekly(&["2026-10-30", "2026-11-02"], &c, today);
        assert_eq!(w.last().map(|x| x.0.as_str()), Some("2026-10-26"));
        assert_eq!(w.iter().map(|x| x.1).sum::<usize>(), 1);
    }

    #[test]
    fn the_recall_list_keeps_those_owed_and_not_yet_settled() {
        let today = "2026-10-08";
        let c = default_campaign(today);
        let people = vec![
            // 80 ans, rien au carnet : à rappeler.
            person(1, "1946-03-01", vec![]),
            // 70 ans, déjà vacciné cette saison : non.
            person(2, "1956-01-01", vec![dose("GRIPPE", "2026-10-02")]),
            // 40 ans : la grippe est une question, pas un dû.
            person(3, "1986-01-01", vec![]),
            // 68 ans, appelé, a refusé : sorti de la liste.
            person(4, "1958-05-05", vec![]),
            // 90 ans, message laissé : reste, avec la date.
            person(5, "1936-05-05", vec![]),
        ];
        let call = |id, pid, outcome: &str, season: &str| Call {
            id,
            patient_id: pid,
            season: season.to_owned(),
            code: "GRIPPE".to_owned(),
            called_on: "2026-10-05".to_owned(),
            outcome: outcome.to_owned(),
            operator: String::new(),
        };
        let calls = [
            call(1, 4, "refus", &c.key),
            call(2, 5, "message", &c.key),
            // Un refus de la saison passée ne vaut pas pour celle-ci.
            call(3, 1, "refus", "2025-2026"),
        ];
        let r = recalls(&people, &calls, &c, "GRIPPE", today);
        assert_eq!(ids(&r), [5, 1], "les plus âgés d'abord");
        assert_eq!(
            r[0].last_call,
            Some(("2026-10-05".to_owned(), "message".to_owned()))
        );
        assert_eq!(r[1].last_call, None);
    }

    #[test]
    fn a_pregnancy_puts_a_young_patient_on_the_flu_list() {
        let today = "2026-11-02";
        let people = vec![Person {
            id: 9,
            birth: "1995-04-04",
            ddr: "2026-06-01",
            ..Person::default()
        }];
        let r = recalls(&people, &[], &default_campaign(today), "GRIPPE", today);
        assert_eq!(r.len(), 1);
    }

    #[test]
    fn an_expiry_reads_as_printed_on_the_box() {
        assert_eq!(parse_expiry("06/2027", 2026).as_deref(), Ok("2027-06-30"));
        assert_eq!(parse_expiry("02/28", 2026).as_deref(), Ok("2028-02-29"));
        assert_eq!(
            parse_expiry("15/03/2027", 2026).as_deref(),
            Ok("2027-03-15")
        );
        assert!(parse_expiry("13/2027", 2026).is_err());
        assert!(parse_expiry("demain", 2026).is_err());
    }

    /// **Chaque fragment attrape ce qu'il dit, et rien d'autre** : la
    /// liste des fiches livrées que chaque groupe évoque, relue une fois
    /// et tenue ici. Un fragment qui n'attrape plus rien, ou une fiche
    /// locale (spray nasal, pommade, collyre) qui se met à évoquer un
    /// groupe, fait échouer le test.
    #[test]
    fn every_evocation_catches_what_it_says() {
        let cards: Vec<(&str, &str, &str)> = crate::db::STARTER_DRUGS
            .iter()
            .map(|(n, d, c, _)| (*n, *d, *c))
            .collect();
        for e in EVOCATIONS {
            let caught = cards
                .iter()
                .filter(|(n, d, c)| {
                    !evocations(&[(n, d, c)]).is_empty()
                        && crate::fuzzy::contains_folded(&crate::fuzzy::sort_key(d), e.dci)
                })
                .count();
            assert!(caught > 0, "« {} » n'attrape aucune fiche livrée", e.dci);
        }
        let group_of = |name: &str| -> Vec<&'static str> {
            let c = cards.iter().find(|x| x.0 == name).expect(name);
            evocations(&[*c]).into_iter().map(|(g, _)| g).collect()
        };
        // Locales, ou hors des groupes visés : rien.
        for name in [
            "Avamys", "Nasonex", "Protopic", "Saxenda", "Wegovy", "Truvada", "Viread",
        ] {
            assert!(group_of(name).is_empty(), "{name} : {:?}", group_of(name));
        }
        assert_eq!(group_of("Lantus"), [DIAB]);
        assert_eq!(group_of("Pulmicort"), [RESP]);
        assert_eq!(group_of("Seretide"), [RESP]);
        assert_eq!(group_of("Humira"), [IMMUNO]);
        assert_eq!(group_of("Biktarvy"), [VIH]);
    }

    #[test]
    fn a_treatment_list_names_each_group_once_with_its_first_drug() {
        let t = [
            ("Glucophage", "metformine", "biguanide"),
            ("Januvia", "sitagliptine", "inhibiteur de la DPP-4"),
            (
                "Symbicort",
                "budésonide, formotérol",
                "corticoïde inhalé et bêta-2 de longue durée",
            ),
        ];
        let e = evocations(&t);
        assert_eq!(e[0], (DIAB, "Glucophage".to_owned()));
        assert_eq!(e[1].0, RESP);
        assert_eq!(e.len(), 2);
    }

    #[test]
    fn a_younger_patient_on_insulin_is_listed_after_those_owed() {
        let today = "2026-10-20";
        let people = vec![
            Person {
                id: 1,
                birth: "1980-01-01",
                evoked: vec![(DIAB, "Lantus".to_owned())],
                ..Person::default()
            },
            person(2, "1950-01-01", vec![]),
            // 46 ans sans traitement évocateur : pas dans la liste.
            person(3, "1980-01-01", vec![]),
        ];
        let c = default_campaign(today);
        let r = recalls(&people, &[], &c, "GRIPPE", today);
        assert_eq!(ids(&r), [2, 1]);
        assert!(r[1].evoked && r[1].detail.contains("Lantus"));
        // Le VRS ne se lit pas sur un traitement.
        assert!(recalls(&people, &[], &c, "VRS", today)
            .iter()
            .all(|x| x.patient_id != 1));
    }

    #[test]
    fn the_season_summary_counts_by_age_at_the_injection() {
        let rows = [
            ("1955-01-01", "GRIPPE", "Grippe", "2026-10-14"),
            ("1945-01-01", "GRIPPE", "Grippe", "2026-10-14"),
            ("1990-01-01", "GRIPPE", "Grippe", "2026-10-15"),
            ("", "GRIPPE", "Grippe", "2026-10-15"),
            ("1950-01-01", "COVID", "COVID-19", "2026-10-14"),
            // Saison passée : hors bilan.
            ("1950-01-01", "GRIPPE", "Grippe", "2026-01-10"),
        ];
        let mut c = default_campaign("2026-10-20");
        let s = summary(&rows, &c, "2026-10-20");
        assert_eq!(s[0].code, "GRIPPE");
        assert_eq!((s[0].by_band.as_slice(), s[0].unknown), (&[1, 1, 1][..], 1));
        assert_eq!(s[0].total(), 4);
        assert_eq!(s[1].total(), 1);
        // Une seule borne, 70 ans : 36 ans d'un côté, 71 et 81 ans de l'autre.
        c.bands = vec![70];
        let s = summary(&rows, &c, "2026-10-20");
        assert_eq!(s[0].by_band, [1, 2]);
        assert_eq!(
            band_labels(&c.bands),
            ["Moins de 70 ans", "70 ans et plus"],
            "une colonne de plus que de bornes"
        );
        assert_eq!(band_labels(&[65, 75])[1], "65 à 74 ans");
        c.bands.clear();
        assert_eq!(summary(&rows, &c, "2026-10-20")[0].by_band, [3]);
    }

    #[test]
    fn an_open_vial_is_used_within_its_hours_even_across_midnight() {
        let v = Vial {
            id: 1,
            lot: "CX".to_owned(),
            opened_on: "2026-10-20".to_owned(),
            opened_min: 9 * 60 + 12,
            operator: String::new(),
        };
        let (day, at, left) = vial_left(&v, 12, "2026-10-20", 18 * 60).unwrap();
        assert_eq!(
            (day.as_str(), hm(at).as_str(), left),
            ("2026-10-20", "21 h 12", 192)
        );
        // Ouvert le soir : la limite tombe le lendemain matin.
        let late = Vial {
            opened_min: 19 * 60,
            ..v.clone()
        };
        let (day, at, _) = vial_left(&late, 12, "2026-10-20", 19 * 60).unwrap();
        assert_eq!((day.as_str(), hm(at).as_str()), ("2026-10-21", "7 h 00"));
        // Délai dépassé : négatif.
        assert!(vial_left(&v, 12, "2026-10-20", 22 * 60).unwrap().2 < 0);
        // Un autre délai, celui d'une autre spécialité : 6 heures.
        assert!(vial_left(&v, 6, "2026-10-20", 16 * 60).unwrap().2 < 0);
    }

    /// Le flacon multidose se déclare au sujet : la spécialité de la
    /// campagne, et pas une autre, reçoit le bouton et son délai.
    #[test]
    fn a_vial_rule_belongs_to_its_subject_and_product() {
        let c = default_campaign("2026-10-20");
        let mut l = lot(1, "COVID", "CX", "2027-01-31", 10, "2026-10-01");
        l.product = "Comirnaty XFG 30 µg".into();
        assert_eq!(c.vial_hours(&l), Some(VIAL_HOURS));
        l.product = "Nuvaxovid".into();
        assert_eq!(c.vial_hours(&l), None);
        l.code = "GRIPPE".into();
        l.product = "Comirnaty".into();
        assert_eq!(c.vial_hours(&l), None);
    }

    #[test]
    fn every_outcome_key_reads_back() {
        for o in Outcome::ALL {
            assert_eq!(Outcome::from_key(o.key()), Some(o));
        }
        assert!(!Outcome::Message.closes());
        assert!(Outcome::Refus.closes());
    }

    fn custom(start: &str, end: &str) -> Campaign {
        Campaign {
            id: 7,
            key: fresh_key(start, end, &[]),
            title: "Campagne".into(),
            starts_on: start.into(),
            ends_on: end.into(),
            subjects: Vec::new(),
            low_stock: DEFAULT_LOW_STOCK,
            expiry_days: DEFAULT_EXPIRY_DAYS,
            bands: DEFAULT_BANDS.to_vec(),
            remark: String::new(),
        }
    }

    /// **Un sujet par critères** : un vaccin que le calendrier ne lit pas
    /// à cet âge (le HPV de rattrapage), visé de 15 à 26 ans.
    #[test]
    fn a_criteria_subject_lists_by_age_and_sex() {
        let today = "2026-10-20";
        let mut c = custom("2026-09-01", "2027-06-30");
        c.subjects = vec![Subject {
            code: "HPV".into(),
            label: "HPV".into(),
            age_min: Some(15),
            age_max: Some(26),
            ..Subject::default()
        }];
        let people = vec![
            person(1, "2005-01-01", vec![]),
            person(2, "2008-06-01", vec![dose("HPV", "2026-10-01")]),
            person(3, "1990-01-01", vec![]),
            // Sans date de naissance : une borne d'âge ne se lit pas.
            person(4, "", vec![]),
            person(5, "2012-01-01", vec![]),
        ];
        let r = recalls(&people, &[], &c, "HPV", today);
        assert_eq!(ids(&r), [1]);
        assert!(r[0].detail.starts_with("21 ans"), "{}", r[0].detail);
        // Réservé aux femmes : un dossier sans sexe connu n'y est pas.
        c.subjects[0].sex = "F".into();
        assert!(recalls(&people, &[], &c, "HPV", today).is_empty());
        let women = vec![Person {
            sex: "F",
            ..person(1, "2005-01-01", vec![])
        }];
        assert_eq!(ids(&recalls(&women, &[], &c, "HPV", today)), [1]);
    }

    /// **Un sujet qui n'est pas un vaccin** : les entretiens AVK, visés
    /// par les traitements, réglés par un entretien réalisé dans l'année.
    #[test]
    fn an_act_subject_targets_treatments_and_is_settled_by_the_act() {
        let today = "2026-11-10";
        let mut c = custom("2026-11-01", "2027-03-31");
        c.subjects = vec![Subject {
            code: "AVK".into(),
            label: "Entretien AVK".into(),
            means: Means::Act("AVK".into()),
            molecules: vec!["fluindione".into(), "warfarine".into()],
            coverage: Coverage::Months(12),
            ..Subject::default()
        }];
        let on = |id, name, dci, acts: Vec<(&'static str, &'static str)>| Person {
            id,
            birth: "1950-01-01",
            treatments: vec![(name, dci, "antivitamine K")],
            acts,
            ..Person::default()
        };
        let people = vec![
            on(1, "Préviscan", "fluindione", vec![]),
            // Un entretien il y a huit mois : réglé pour douze mois.
            on(2, "Coumadine", "warfarine", vec![("AVK", "2026-03-10")]),
            // Il y a plus d'un an : à refaire.
            on(3, "Coumadine", "warfarine", vec![("AVK", "2025-10-01")]),
            on(4, "Eliquis", "apixaban", vec![]),
            // Un autre acte ne règle rien.
            on(5, "Préviscan", "fluindione", vec![("BPM", "2026-11-01")]),
        ];
        let r = recalls(&people, &[], &c, "AVK", today);
        assert_eq!(ids(&r), [1, 3, 5]);
        assert!(r[0].detail.contains("Préviscan"), "{}", r[0].detail);
        assert_eq!(c.subjects[0].act_kind(), Some("AVK"));
    }

    /// **Le bilan partagé de médication** : 65 ans et plus, au moins cinq
    /// traitements, comptés par molécule et hors formes locales.
    #[test]
    fn a_minimum_number_of_treatments_counts_molecules() {
        let today = "2026-11-10";
        let mut c = custom("2026-11-01", "2027-10-31");
        c.subjects = vec![Subject {
            code: "BPM".into(),
            label: "Bilan de médication".into(),
            means: Means::Act("BPM".into()),
            age_min: Some(65),
            min_treatments: 5,
            ..Subject::default()
        }];
        let five = vec![
            ("Kardegic", "acide acetylsalicylique", "antiagrégant"),
            ("Bisoce", "bisoprolol", "bêtabloquant"),
            ("Tahor", "atorvastatine", "statine"),
            ("Glucophage", "metformine", "biguanide"),
            ("Inexium", "esomeprazole", "IPP"),
        ];
        let mut twice = five[..4].to_vec();
        twice.push(("Metformine Biogaran", "metformine", "biguanide"));
        let people = vec![
            Person {
                id: 1,
                birth: "1950-01-01",
                treatments: five.clone(),
                ..Person::default()
            },
            Person {
                id: 2,
                birth: "1950-01-01",
                treatments: twice,
                ..Person::default()
            },
            Person {
                id: 3,
                birth: "1980-01-01",
                treatments: five,
                ..Person::default()
            },
        ];
        let r = recalls(&people, &[], &c, "BPM", today);
        assert_eq!(ids(&r), [1]);
        assert!(r[0].detail.contains('5'), "{}", r[0].detail);
    }

    /// **Un sujet sans rien à lire en base** : seul l'appel le règle, et
    /// un message laissé ne le règle pas.
    #[test]
    fn a_call_subject_is_settled_only_by_a_closing_call() {
        let today = "2026-11-10";
        let mut c = custom("2026-11-01", "2026-11-30");
        c.subjects = vec![Subject {
            code: "COLON".into(),
            label: "Dépistage colorectal".into(),
            means: Means::Call,
            age_min: Some(50),
            age_max: Some(74),
            ..Subject::default()
        }];
        let people = vec![
            person(1, "1960-01-01", vec![]),
            person(2, "1960-01-01", vec![]),
            person(3, "1960-01-01", vec![]),
        ];
        let call = |pid, outcome: &str| Call {
            id: pid,
            patient_id: pid,
            season: c.key.clone(),
            code: "COLON".into(),
            called_on: "2026-11-05".into(),
            outcome: outcome.into(),
            operator: String::new(),
        };
        let calls = [call(2, "prevenu"), call(3, "message")];
        let r = recalls(&people, &calls, &c, "COLON", today);
        assert_eq!(ids(&r), [1, 3]);
        assert_eq!(c.subjects[0].act_kind(), None);
        assert_eq!(call_counts(&calls, &c.key, "COLON"), [1, 1, 0, 0]);
    }

    /// Une série par critères : la seconde dose n'est rappelée qu'après
    /// l'intervalle, et la série faite sort de la liste.
    #[test]
    fn a_series_waits_for_its_interval() {
        let today = "2026-11-10";
        let mut c = custom("2026-09-01", "2027-08-31");
        c.subjects = vec![Subject {
            code: "HEPB".into(),
            label: "Hépatite B".into(),
            coverage: Coverage::Ever,
            doses: 3,
            interval_months: 1,
            ..Subject::default()
        }];
        let people = vec![
            person(1, "1990-01-01", vec![dose("HEPB", "2026-10-30")]),
            person(2, "1990-01-01", vec![dose("HEPB", "2026-09-01")]),
            person(
                3,
                "1990-01-01",
                vec![
                    dose("HEPB", "2026-01-01"),
                    dose("HEPB", "2026-02-01"),
                    dose("HEPB", "2026-07-01"),
                ],
            ),
        ];
        let r = recalls(&people, &[], &c, "HEPB", today);
        assert_eq!(ids(&r), [2]);
        assert!(r[0].detail.contains("1 dose"), "{}", r[0].detail);
    }

    /// **L'an prochain** : les dates, les années du titre et la clé d'une
    /// saison avancent d'un an ; le reste est repris.
    #[test]
    fn a_campaign_carries_over_to_next_year() {
        let mut c = default_campaign("2026-10-20");
        c.id = 4;
        c.title = "Hiver 2026-2027 (équipe de 2026)".into();
        c.subjects[1].vial_hours = 6;
        let n = c.next_year(&[]);
        assert_eq!(n.id, 0);
        assert_eq!(n.key, "2027-2028");
        assert_eq!(
            (n.starts_on.as_str(), n.ends_on.as_str()),
            ("2027-09-01", "2028-08-31")
        );
        assert_eq!(n.title, "Hiver 2027-2028 (équipe de 2027)");
        assert_eq!(n.subjects, c.subjects);
        // La clé de la saison suivante prise : une autre, tirée des dates.
        let n = c.next_year(&["2027-2028"]);
        assert_eq!(n.key, "2027-09-01~2028-08-31");
        // Une campagne d'une autre forme garde sa forme.
        let m = custom("2026-11-01", "2027-02-28").next_year(&[]);
        assert_eq!(m.key, "2027-11-01~2028-02-29");
        assert_eq!(shift_years("Lot 12345 en 2026"), "Lot 12345 en 2027");
    }

    #[test]
    fn a_fresh_key_never_takes_one_already_used() {
        // Les bornes d'une saison ne donnent pas sa clé : elle reste à la
        // campagne livrée de cet hiver-là, dont les appels y sont rangés.
        assert_eq!(
            fresh_key("2027-09-01", "2028-08-31", &[]),
            "2027-09-01~2028-08-31"
        );
        assert_eq!(
            fresh_key("2027-09-01", "2028-08-31", &["2027-09-01~2028-08-31"]),
            "2027-09-01~2028-08-31#2"
        );
    }

    /// Une campagne nouvelle aux bornes de la saison ne prend pas la place
    /// de la campagne livrée : les deux restent au menu, chacune ses appels.
    #[test]
    fn a_new_campaign_on_season_dates_leaves_the_season_campaign() {
        let today = "2026-10-10";
        let season = default_campaign(today);
        let avk = custom("2026-09-01", "2027-08-31");
        assert_ne!(avk.key, season.key);
        let keys: Vec<String> = listing(std::slice::from_ref(&avk), today)
            .into_iter()
            .map(|c| c.key)
            .collect();
        assert!(keys.contains(&season.key) && keys.contains(&avk.key));
        // L'an prochain, la saison 2027-2028 arrive : sa clé est libre.
        let later = avk.next_year(&[]);
        assert_ne!(later.key, default_campaign("2027-10-10").key);
    }

    /// Ce que la base garde se relit à l'identique, séparateurs compris ;
    /// une clé inconnue (d'une version plus récente) est ignorée.
    #[test]
    fn a_campaign_reads_back_what_it_wrote() {
        let mut subjects = season_subjects();
        subjects.push(Subject {
            code: "AVK".into(),
            label: "Entretien ; AVK = 100 % ".trim().into(),
            means: Means::Act("AVK".into()),
            age_min: Some(18),
            age_max: Some(99),
            sex: "F".into(),
            pregnancy: true,
            molecules: vec!["fluindione".into(), "warfarine".into()],
            min_treatments: 3,
            coverage: Coverage::Months(12),
            doses: 2,
            interval_months: 6,
            ..Subject::default()
        });
        subjects.push(Subject {
            code: "COLON".into(),
            label: "Dépistage\nrévisé".into(),
            means: Means::Call,
            ..Subject::default()
        });
        let text = encode_subjects(&subjects);
        assert_eq!(decode_subjects(&text), subjects);
        assert_eq!(decode_subjects(&format!("{text};futur=oui")), subjects);
        assert!(decode_subjects("libelle=sans code").is_empty());
        let mut c = default_campaign("2026-10-20");
        c.low_stock = 12;
        c.bands = vec![50, 65, 80];
        let mut back = default_campaign("2026-10-20");
        decode_settings(&mut back, &encode_settings(&c));
        assert_eq!((back.low_stock, back.bands.clone()), (12, vec![50, 65, 80]));
        c.bands.clear();
        decode_settings(&mut back, &encode_settings(&c));
        assert!(back.bands.is_empty(), "aucune tranche reste aucune tranche");
    }

    /// Un sujet réglé par un appel se compte par personne, d'après son
    /// dernier appel — comme le bilan des appels.
    #[test]
    fn a_call_subject_counts_people_by_their_last_call() {
        let mut c = custom("2026-09-01", "2027-03-31");
        c.subjects = vec![Subject {
            code: "COLON".into(),
            label: "Dépistage".into(),
            means: Means::Call,
            ..Subject::default()
        }];
        let call = |id: i64, patient_id: i64, day: &str, outcome: &str| Call {
            id,
            patient_id,
            season: c.key.clone(),
            code: "COLON".into(),
            called_on: day.into(),
            outcome: outcome.into(),
            operator: String::new(),
        };
        let calls = vec![
            // Prévenue deux fois : une personne.
            call(1, 10, "2026-10-01", "prevenu"),
            call(2, 10, "2026-10-05", "prevenu"),
            // Prévenu, puis refus : personne.
            call(3, 11, "2026-10-02", "prevenu"),
            call(4, 11, "2026-10-06", "refus"),
            call(5, 12, "2026-10-03", "ailleurs"),
        ];
        let done = Done {
            doses: &[],
            acts: &[],
            calls: &calls,
        };
        let dates = subject_dates(&c.subjects[0], &done, &c, "2026-10-10");
        assert_eq!(dates, ["2026-10-03", "2026-10-05"]);
        let counts = call_counts(&calls, &c.key, "COLON");
        let settled = Outcome::ALL
            .iter()
            .zip(counts)
            .filter(|(o, _)| matches!(o.key(), "prevenu" | "ailleurs"))
            .map(|(_, n)| n)
            .sum::<usize>();
        assert_eq!(settled, dates.len(), "le compte et le bilan des appels");
    }

    #[test]
    fn a_campaign_without_vaccines_sums_no_doses() {
        let mut c = custom("2026-09-01", "2027-03-31");
        c.subjects = vec![Subject {
            code: "AVK".into(),
            label: "Entretien AVK".into(),
            means: Means::Act("AVK".into()),
            ..Subject::default()
        }];
        let rows = [("1950-01-01", "GRIPPE", "Grippe", "2026-10-02")];
        assert!(summary(&rows, &c, "2026-10-10").is_empty());
        let winter = default_campaign("2026-10-10");
        assert_eq!(summary(&rows, &winter, "2026-10-10").len(), 1);
    }

    #[test]
    fn typed_age_bands_are_sorted_or_refused() {
        assert_eq!(parse_bands("75, 65"), Some(vec![65, 75]));
        assert_eq!(parse_bands(""), Some(vec![]));
        assert_eq!(parse_bands("65;65"), Some(vec![65]));
        assert_eq!(parse_bands("soixante"), None);
        assert_eq!(parse_bands("0"), None);
    }

    /// La liste propose la campagne de la saison et celle de l'an passé
    /// tant que la base n'en porte pas la clé, en cours d'abord.
    #[test]
    fn the_listing_puts_the_running_campaign_first() {
        let today = "2026-11-10";
        let l = listing(&[], today);
        let keys: Vec<&str> = l.iter().map(|c| c.key.as_str()).collect();
        assert_eq!(keys, ["2026-2027", "2025-2026"]);
        let mut avk = custom("2026-11-01", "2027-03-31");
        avk.title = "AVK".into();
        let mut stored = default_campaign(today);
        stored.id = 3;
        stored.title = "Hiver de l'officine".into();
        let soon = custom("2027-01-01", "2027-02-28");
        let l = listing(&[stored, avk, soon], today);
        let titles: Vec<&str> = l.iter().map(|c| c.title.as_str()).collect();
        assert_eq!(titles[..2], ["AVK", "Hiver de l'officine"]);
        assert_eq!(l.len(), 4, "la campagne de la saison n'est pas doublée");
        assert_eq!(l[3].phase(today), Phase::Upcoming);
    }

    #[test]
    fn a_campaign_is_checked_before_it_is_written() {
        let ok = default_campaign("2026-10-20");
        assert_eq!(check(&ok), None);
        let mut c = ok.clone();
        c.ends_on = "2026-01-01".into();
        assert!(check(&c).is_some());
        let mut c = ok.clone();
        c.subjects[1].code = "GRIPPE".into();
        assert!(check(&c).is_some(), "deux sujets du même code");
        let mut c = ok.clone();
        c.subjects[0].means = Means::Call;
        assert!(check(&c).is_some(), "le calendrier ne lit que les vaccins");
        let mut c = ok;
        c.subjects[0].rule = Rule::Criteria;
        c.subjects[0].age_min = Some(70);
        c.subjects[0].age_max = Some(60);
        assert!(check(&c).is_some());
    }

    #[test]
    fn the_calendar_reads_the_winter_vaccines() {
        let codes = calendar_codes("2026-10-20");
        for c in SEASON_CODES {
            assert!(codes.contains(&c), "{c} : {codes:?}");
        }
    }
}

#[cfg(test)]
mod perf {
    /// Le coût des évocations et des rappels sur une grosse officine
    /// (5 000 dossiers, 5 traitements chacun), en version optimisée :
    /// `cargo test --release --lib campagne::perf -- --nocapture`.
    #[test]
    fn the_recall_list_stays_cheap_on_a_large_officine() {
        let cards: Vec<(&str, &str, &str)> = crate::db::STARTER_DRUGS
            .iter()
            .map(|(n, d, c, _)| (*n, *d, *c))
            .collect();
        let t0 = std::time::Instant::now();
        let mut evoked = Vec::new();
        let mut memo = std::collections::HashMap::new();
        for i in 0..5000 {
            let list: Vec<(&str, &str, &str)> = (0..5)
                .map(|k| cards[(i * 7 + k * 131) % cards.len()])
                .collect();
            let e = super::evocations_memo(&list, &mut memo);
            // La mémoire ne change pas la réponse.
            if i < 50 {
                assert_eq!(e, super::evocations(&list));
            }
            evoked.push(e);
        }
        let t_evoc = t0.elapsed();
        let births: Vec<String> = (0..5000)
            .map(|i| format!("{}-03-01", 1930 + i % 70))
            .collect();
        let people: Vec<super::Person> = births
            .iter()
            .zip(evoked)
            .enumerate()
            .map(|(i, (b, e))| super::Person {
                id: i as i64,
                birth: b,
                evoked: e,
                ..super::Person::default()
            })
            .collect();
        let t1 = std::time::Instant::now();
        let c = super::default_campaign("2026-10-20");
        let n: usize = super::SEASON_CODES
            .iter()
            .map(|code| super::recalls(&people, &[], &c, code, "2026-10-20").len())
            .sum();
        eprintln!(
            "évocations : {:?} ; rappels ({} vaccins) : {:?} ; {n} lignes",
            t_evoc,
            super::SEASON_CODES.len(),
            t1.elapsed()
        );
        assert!(n > 0);
    }
}
