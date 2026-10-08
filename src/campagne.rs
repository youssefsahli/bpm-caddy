//! La campagne de vaccination de l'hiver : les lots reçus, les doses de
//! la saison, et les patients à rappeler.
//!
//! Trois questions que l'équipe se pose chaque jour d'octobre à février,
//! et auxquelles le carnet de chaque dossier répondait un dossier à la
//! fois :
//!
//! - **Combien reste-t-il de doses de ce lot ?** Le lot est saisi au
//!   carnet à chaque injection ; il suffit de compter les lignes qui le
//!   portent contre ce qui a été reçu. Rien n'est décrémenté à la main :
//!   une dose corrigée ou supprimée au carnet se retrouve dans le compte.
//! - **Combien de doses depuis le début de la saison ?** Par vaccin, le
//!   jour, la semaine et la saison, lus dans les carnets.
//! - **Qui n'est pas encore venu ?** Les dossiers que le calendrier
//!   (`vaccines::due_lines_with`) déclare *dus* — 65 ans et plus, ou
//!   grossesse — sans dose de la saison, moins ceux que l'équipe a déjà
//!   appelés pour cette saison.
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
    /// « 2026-2027 » : la clé sous laquelle un appel est rangé.
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
    /// Périmé dans moins de [`EXPIRY_WARNING_DAYS`] jours.
    ExpiresSoon(i64),
    /// Moins de [`LOW_STOCK`] doses restantes.
    Low,
    Ok,
}

/// Délai sous lequel une péremption proche est signalée.
pub const EXPIRY_WARNING_DAYS: i64 = 30;
/// Seuil sous lequel il reste peu de doses.
pub const LOW_STOCK: i64 = 5;

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
pub fn lot_use(lot: &Lot, used: i64, today: &str) -> LotUse {
    let remaining = lot.received - used;
    let to_expiry = (!lot.expires_on.trim().is_empty())
        .then(|| crate::date::days_between(today, lot.expires_on.trim()))
        .flatten();
    let state = match to_expiry {
        Some(d) if d < 0 => LotState::Expired,
        _ if lot.received > 0 && remaining <= 0 => LotState::Exhausted,
        Some(d) if d <= EXPIRY_WARNING_DAYS => LotState::ExpiresSoon(d),
        _ if remaining < LOW_STOCK => LotState::Low,
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
                lot_use(l, u, today).state,
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

/// Les doses d'un vaccin sur la saison.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tally {
    pub code: String,
    pub label: String,
    pub today: usize,
    /// Les sept derniers jours, aujourd'hui compris.
    pub week: usize,
    pub season: usize,
}

/// Les doses de la saison par vaccin, la plus fournie d'abord.
///
/// Une ligne sans code (vaccin saisi en texte libre) est comptée sous
/// son libellé.
pub fn tally(doses: &[DoseRow], today: &str) -> Vec<Tally> {
    let s = season(today);
    let week_start = crate::date::add_days(today, -6).unwrap_or_default();
    let mut out: Vec<Tally> = Vec::new();
    for d in doses {
        let date = d.given_on.trim();
        if date.is_empty() || date < s.start.as_str() || date > today {
            continue;
        }
        let key = if d.code.trim().is_empty() {
            d.label.trim()
        } else {
            d.code.trim()
        };
        if key.is_empty() {
            continue;
        }
        let i = match out.iter().position(|t| t.code == key) {
            Some(i) => i,
            None => {
                out.push(Tally {
                    code: key.to_owned(),
                    label: d.label.trim().to_owned(),
                    today: 0,
                    week: 0,
                    season: 0,
                });
                out.len() - 1
            }
        };
        let t = &mut out[i];
        t.season += 1;
        if date >= week_start.as_str() {
            t.week += 1;
        }
        if date == today {
            t.today += 1;
        }
    }
    out.sort_by(|a, b| b.season.cmp(&a.season).then(a.code.cmp(&b.code)));
    out
}

/// Les doses d'un vaccin semaine par semaine depuis le début de la
/// saison, le lundi de chaque semaine et son compte. La première
/// semaine commence au lundi qui précède (ou est) le 1er septembre.
pub fn weekly(doses: &[DoseRow], code: &str, today: &str) -> Vec<(String, usize)> {
    let s = season(today);
    let monday = |iso: &str| -> Option<String> {
        let wd = crate::date::weekday(iso)?;
        crate::date::add_days(iso, 1 - wd)
    };
    let (Some(first), Some(last)) = (monday(&s.start), monday(today)) else {
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
    for d in doses.iter().filter(|d| d.code == code) {
        let date = d.given_on.trim();
        if date < s.start.as_str() || date > today {
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

/// Les vaccins de la campagne que la liste des rappels sait lire. Le
/// calendrier a d'autres lignes, mais celles-ci sont celles de l'hiver.
pub const CAMPAIGN_CODES: [&str; 3] = ["GRIPPE", "COVID", "VRS"];

/// Ce que l'équipe a noté après un appel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Rendez-vous pris ou personne prévenue.
    Prevenu,
    /// Message laissé, sans réponse : reste dans la liste.
    Message,
    /// Vacciné ailleurs (médecin, infirmier, autre officine).
    Ailleurs,
    /// Ne souhaite pas être vacciné cette saison.
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

/// Un appel noté, pour une saison et un vaccin.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Call {
    pub id: i64,
    pub patient_id: i64,
    pub season: String,
    pub code: String,
    /// ISO.
    pub called_on: String,
    pub outcome: String,
    pub operator: String,
}

/// Un dossier tel que la liste des rappels le lit.
pub struct Person<'a> {
    pub id: i64,
    pub birth: &'a str,
    pub ddr: &'a str,
    pub doses: Vec<vaccines::Dose<'a>>,
    /// Les groupes que ses traitements évoquent ([`evocations`]).
    pub evoked: Vec<(&'static str, String)>,
}

/// Une ligne de la liste des rappels.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recall {
    pub patient_id: i64,
    pub code: &'static str,
    /// La raison que le calendrier donne (« Campagne en cours, aucune
    /// dose enregistrée. »).
    pub detail: String,
    /// Le dernier appel de la saison pour ce vaccin, s'il y en a un :
    /// un message laissé reste dans la liste, avec sa date.
    pub last_call: Option<(String, String)>,
    /// La ligne vient d'un traitement évocateur et non de l'âge ou d'une
    /// grossesse : l'indication est à confirmer avec le patient.
    pub evoked: bool,
}

/// Les dossiers à rappeler pour `code` : dus selon le calendrier, sans
/// dose de la saison, et sans appel qui ait clos la question cette
/// saison. Les plus âgés d'abord — c'est l'ordre du risque, et celui
/// dans lequel on appelle.
pub fn recalls(people: &[Person], calls: &[Call], code: &str, today: &str) -> Vec<Recall> {
    let s = season(today);
    let mut out: Vec<(String, Recall)> = Vec::new();
    for p in people {
        let lines = vaccines::due_lines_with(p.birth, today, &p.doses, p.ddr);
        let Some(line) = lines.iter().find(|l| l.code == code) else {
            continue;
        };
        // Dû par l'âge ou la grossesse ; sinon, pour la grippe et le
        // COVID-19, évoqué par un traitement — une question ouverte que
        // le calendrier laisse en « à demander ».
        let evoked = line.level == DueLevel::Ask
            && matches!(code, "GRIPPE" | "COVID")
            && !p.evoked.is_empty();
        if line.level != DueLevel::Due && !evoked {
            continue;
        }
        let mut mine: Vec<&Call> = calls
            .iter()
            .filter(|c| c.patient_id == p.id && c.code == code && c.season == s.label)
            .collect();
        mine.sort_by(|a, b| a.called_on.cmp(&b.called_on).then(a.id.cmp(&b.id)));
        let last = mine.last();
        if last.is_some_and(|c| Outcome::from_key(&c.outcome).is_some_and(Outcome::closes)) {
            continue;
        }
        let detail = if evoked {
            let named: Vec<String> = p
                .evoked
                .iter()
                .map(|(group, drug)| format!("{group} ({drug})"))
                .collect();
            crate::strings::trf("camp_evoked", named.join(" ; "))
        } else {
            line.detail.clone()
        };
        out.push((
            p.birth.to_owned(),
            Recall {
                patient_id: p.id,
                code: line.code,
                detail,
                last_call: last.map(|c| (c.called_on.clone(), c.outcome.clone())),
                evoked,
            },
        ));
    }
    // Les dossiers dus d'abord, puis les traitements évocateurs ; dans
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

// ---------------------------------------------------------------------
// Les flacons multidoses ouverts
// ---------------------------------------------------------------------

/// Le délai d'utilisation d'un flacon multidose de vaccin contre le
/// COVID-19 après la première ponction (Comirnaty XFG, campagne
/// 2026-2027).
pub const VIAL_HOURS: i64 = 12;

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

/// Ce qu'il reste d'un flacon : l'heure limite (jour ISO et minute) et
/// les minutes restantes à `today`/`now_min`, négatives une fois le délai
/// dépassé.
pub fn vial_left(v: &Vial, today: &str, now_min: i64) -> Option<(String, i64, i64)> {
    let opened = crate::date::to_days(&v.opened_on)? * 1440 + v.opened_min;
    let now = crate::date::to_days(today)? * 1440 + now_min;
    let end = opened + VIAL_HOURS * 60;
    let day = crate::date::from_days(end.div_euclid(1440));
    Some((day, end.rem_euclid(1440), end - now))
}

/// Une heure du jour écrite à la française : « 9 h 05 ».
pub fn hm(minutes: i64) -> String {
    let m = minutes.rem_euclid(1440);
    format!("{} h {:02}", m / 60, m % 60)
}

// ---------------------------------------------------------------------
// Le bilan de la saison
// ---------------------------------------------------------------------

/// Les doses d'un vaccin sur la saison, par tranche d'âge à la date de
/// l'injection : moins de 65 ans, 65 à 74 ans, 75 ans et plus, âge
/// inconnu (pas de date de naissance au dossier).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Summary {
    pub code: String,
    pub label: String,
    pub under_65: usize,
    pub from_65: usize,
    pub from_75: usize,
    pub unknown: usize,
}

impl Summary {
    pub fn total(&self) -> usize {
        self.under_65 + self.from_65 + self.from_75 + self.unknown
    }
}

/// Le bilan de la saison en cours. `rows` : naissance (ISO, vide si
/// inconnue), code, libellé, date de la dose.
pub fn summary(rows: &[(&str, &str, &str, &str)], today: &str) -> Vec<Summary> {
    let s = season(today);
    let mut out: Vec<Summary> = Vec::new();
    for (birth, code, label, given) in rows {
        let date = given.trim();
        if date.is_empty() || date < s.start.as_str() || date > today {
            continue;
        }
        let key = if code.trim().is_empty() {
            label.trim()
        } else {
            code.trim()
        };
        if key.is_empty() {
            continue;
        }
        let i = match out.iter().position(|x| x.code == key) {
            Some(i) => i,
            None => {
                out.push(Summary {
                    code: key.to_owned(),
                    label: label.trim().to_owned(),
                    under_65: 0,
                    from_65: 0,
                    from_75: 0,
                    unknown: 0,
                });
                out.len() - 1
            }
        };
        let row = &mut out[i];
        match crate::db::age_on(birth, date) {
            Some(a) if a >= 75 => row.from_75 += 1,
            Some(a) if a >= 65 => row.from_65 += 1,
            Some(_) => row.under_65 += 1,
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
    memo: &mut std::collections::HashMap<String, Vec<&'static str>>,
) -> Vec<(&'static str, String)> {
    let mut out: Vec<(&'static str, String)> = Vec::new();
    for t in treatments {
        let groups = memo
            .entry(t.0.to_owned())
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

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn a_lot_number_is_compared_without_its_typing() {
        assert_eq!(norm_lot(" fl25-208 "), "FL25208");
        assert_eq!(norm_lot("FL25 208"), norm_lot("fl25208"));
        assert_eq!(norm_lot(" - "), "");
    }

    #[test]
    fn a_lot_reads_its_state_most_serious_first() {
        let today = "2026-10-08";
        let l = lot(1, "GRIPPE", "A1", "2027-06-30", 20, "2026-10-01");
        assert_eq!(lot_use(&l, 3, today).state, LotState::Ok);
        assert_eq!(lot_use(&l, 3, today).remaining, 17);
        assert_eq!(lot_use(&l, 16, today).state, LotState::Low);
        assert_eq!(lot_use(&l, 20, today).state, LotState::Exhausted);
        // Plus de doses au carnet que reçues : épuisé, et le reste
        // négatif dit l'erreur.
        assert_eq!(lot_use(&l, 22, today).remaining, -2);
        let soon = lot(2, "GRIPPE", "A2", "2026-10-20", 20, "2026-10-01");
        assert_eq!(lot_use(&soon, 0, today).state, LotState::ExpiresSoon(12));
        let gone = lot(3, "GRIPPE", "A3", "2026-10-07", 20, "2026-10-01");
        // Périmé l'emporte sur tout, même s'il reste des doses.
        assert_eq!(lot_use(&gone, 0, today).state, LotState::Expired);
        // Sans date de péremption : jamais dit périmé.
        let undated = lot(4, "GRIPPE", "A4", "", 10, "");
        assert_eq!(lot_use(&undated, 0, today).state, LotState::Ok);
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
            d("", "2026-10-08"),
        ];
        let t = tally(&doses, today);
        assert_eq!(t[0].code, "GRIPPE");
        assert_eq!((t[0].today, t[0].week, t[0].season), (1, 2, 3));
        assert_eq!((t[1].today, t[1].week, t[1].season), (1, 1, 1));
        // Ni code ni libellé : rien à quoi la rattacher.
        assert_eq!(t.len(), 2);
    }

    #[test]
    fn the_weekly_series_starts_on_the_monday_of_september() {
        let doses = [
            DoseRow {
                code: "GRIPPE",
                label: "",
                given_on: "2026-10-08",
            },
            DoseRow {
                code: "GRIPPE",
                label: "",
                given_on: "2026-10-05",
            },
        ];
        let w = weekly(&doses, "GRIPPE", "2026-10-08");
        // Le 1er septembre 2026 est un mardi : la série part du lundi 31 août.
        assert_eq!(w.first().map(|x| x.0.as_str()), Some("2026-08-31"));
        assert_eq!(w.last(), Some(&("2026-10-05".to_owned(), 2)));
        assert_eq!(w.iter().map(|x| x.1).sum::<usize>(), 2);
    }

    #[test]
    fn the_recall_list_keeps_those_owed_and_not_yet_settled() {
        let today = "2026-10-08";
        let s = season(today).label;
        let people = vec![
            // 80 ans, rien au carnet : à rappeler.
            Person {
                id: 1,
                birth: "1946-03-01",
                ddr: "",
                doses: vec![],
                evoked: vec![],
            },
            // 70 ans, déjà vacciné cette saison : non.
            Person {
                id: 2,
                birth: "1956-01-01",
                ddr: "",
                doses: vec![vaccines::Dose {
                    code: "GRIPPE",
                    date: "2026-10-02",
                }],
                evoked: vec![],
            },
            // 40 ans : la grippe est une question, pas un dû.
            Person {
                id: 3,
                birth: "1986-01-01",
                ddr: "",
                doses: vec![],
                evoked: vec![],
            },
            // 68 ans, appelé, a refusé : sorti de la liste.
            Person {
                id: 4,
                birth: "1958-05-05",
                ddr: "",
                doses: vec![],
                evoked: vec![],
            },
            // 90 ans, message laissé : reste, avec la date.
            Person {
                id: 5,
                birth: "1936-05-05",
                ddr: "",
                doses: vec![],
                evoked: vec![],
            },
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
            call(1, 4, "refus", &s),
            call(2, 5, "message", &s),
            // Un refus de la saison passée ne vaut pas pour celle-ci.
            call(3, 1, "refus", "2025-2026"),
        ];
        let r = recalls(&people, &calls, "GRIPPE", today);
        let ids: Vec<i64> = r.iter().map(|x| x.patient_id).collect();
        assert_eq!(ids, [5, 1], "les plus âgés d'abord");
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
            doses: vec![],
            evoked: vec![],
        }];
        let r = recalls(&people, &[], "GRIPPE", today);
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
                ddr: "",
                doses: vec![],
                evoked: vec![(DIAB, "Lantus".to_owned())],
            },
            Person {
                id: 2,
                birth: "1950-01-01",
                ddr: "",
                doses: vec![],
                evoked: vec![],
            },
            // 46 ans sans traitement évocateur : pas dans la liste.
            Person {
                id: 3,
                birth: "1980-01-01",
                ddr: "",
                doses: vec![],
                evoked: vec![],
            },
        ];
        let r = recalls(&people, &[], "GRIPPE", today);
        assert_eq!(r.iter().map(|x| x.patient_id).collect::<Vec<_>>(), [2, 1]);
        assert!(r[1].evoked && r[1].detail.contains("Lantus"));
        // Le VRS ne se lit pas sur un traitement.
        assert!(recalls(&people, &[], "VRS", today)
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
        let s = summary(&rows, "2026-10-20");
        assert_eq!(s[0].code, "GRIPPE");
        assert_eq!(
            (s[0].under_65, s[0].from_65, s[0].from_75, s[0].unknown),
            (1, 1, 1, 1)
        );
        assert_eq!(s[0].total(), 4);
        assert_eq!(s[1].total(), 1);
    }

    #[test]
    fn an_open_vial_is_used_within_twelve_hours_even_across_midnight() {
        let v = Vial {
            id: 1,
            lot: "CX".to_owned(),
            opened_on: "2026-10-20".to_owned(),
            opened_min: 9 * 60 + 12,
            operator: String::new(),
        };
        let (day, at, left) = vial_left(&v, "2026-10-20", 18 * 60).unwrap();
        assert_eq!(
            (day.as_str(), hm(at).as_str(), left),
            ("2026-10-20", "21 h 12", 192)
        );
        // Ouvert le soir : la limite tombe le lendemain matin.
        let late = Vial {
            opened_min: 19 * 60,
            ..v.clone()
        };
        let (day, at, _) = vial_left(&late, "2026-10-20", 19 * 60).unwrap();
        assert_eq!((day.as_str(), hm(at).as_str()), ("2026-10-21", "7 h 00"));
        // Délai dépassé : négatif.
        assert!(vial_left(&v, "2026-10-20", 22 * 60).unwrap().2 < 0);
    }

    #[test]
    fn every_outcome_key_reads_back() {
        for o in Outcome::ALL {
            assert_eq!(Outcome::from_key(o.key()), Some(o));
        }
        assert!(!Outcome::Message.closes());
        assert!(Outcome::Refus.closes());
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
                ddr: "",
                doses: vec![],
                evoked: e,
            })
            .collect();
        let t1 = std::time::Instant::now();
        let n: usize = super::CAMPAIGN_CODES
            .iter()
            .map(|c| super::recalls(&people, &[], c, "2026-10-20").len())
            .sum();
        eprintln!(
            "évocations : {:?} ; rappels (3 vaccins) : {:?} ; {n} lignes",
            t_evoc,
            t1.elapsed()
        );
        assert!(n > 0);
    }
}
