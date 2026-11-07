//! Quand se prend un traitement — la posologie lue en **grille**.
//!
//! Une ordonnance se lit en prose : « 1 comprimé matin et soir »,
//! « 5 mg le soir », « 1 comprimé par semaine, le lundi ». Le patient,
//! lui, ne lit pas de la prose à huit heures du matin avec sa boîte à
//! la main : il regarde une case. Ce module lit la phrase et rend les
//! quatre moments de la journée, de sorte que la feuille remise au
//! comptoir porte un tableau plutôt qu'un paragraphe.
//!
//! **Il ne remplace jamais la phrase.** La ligne imprimée porte les
//! deux : la grille pour l'œil, la posologie telle qu'elle est écrite
//! au dossier pour la lettre. Une grille est une aide à la lecture, et
//! une aide à la lecture qui chasse ce qu'elle aide à lire est une
//! perte sèche.
//!
//! # Les règles, une par test
//!
//! * **Le silence n'est pas une case vide.** Une posologie que la table
//!   ne sait pas lire n'a pas de grille — elle a une ligne qui dit le
//!   texte, et jamais quatre cases blanches. Quatre cases blanches se
//!   lisent « rien à prendre », ce qui est le contraire de « je n'ai
//!   pas su lire ». C'est la règle de `crush.rs`, au même endroit.
//! * **« Si besoin » n'entre jamais dans la grille.** C'est écrit dans
//!   les tables de cette application, à propos du pilulier : « un
//!   pilulier dit quand prendre ; y déposer un antalgique à la demande
//!   le transforme en prise systématique ». Un traitement à la demande
//!   sort de la grille et emporte sa condition avec lui.
//! * **Une prise hebdomadaire n'est pas une prise quotidienne.** C'est
//!   la règle qui tue : le méthotrexate est hebdomadaire, et une croix
//!   dans la colonne « matin » d'une grille quotidienne se lit « tous
//!   les matins ». Un rythme qui n'est pas quotidien ne coche rien et
//!   s'écrit en toutes lettres.
//! * **Une prise dont le moment n'est pas dit ne se place pas.** « 1
//!   comprimé par jour » ne dit pas le matin, et le logiciel n'a pas à
//!   le décider : la feuille laisse l'heure à convenir, avec un blanc
//!   pour l'écrire. La fiche du médicament, elle, dit souvent quand
//!   c'est le mieux — et c'est elle qui le dit, pas la grille.
//! * **Un milligramme n'est pas un comprimé.** « 5 mg matin et soir »
//!   porte deux prises et pas cinq : un nombre suivi d'une unité de
//!   masse est un dosage, jamais une quantité. C'était la lecture la
//!   plus fréquente du dossier de démonstration, et la faute aurait
//!   imprimé « 5 » dans une case.
//! * **Une prise sans quantité reste une prise.** Quand la phrase donne
//!   le moment sans donner le nombre, la case porte une marque et non
//!   un « 1 » : inventer l'unité, c'est écrire une posologie que
//!   personne n'a prescrite.
//!
//! Statique, pur et testé : ni base, ni horloge, ni egui.

/// Les quatre moments d'un pilulier français.
///
/// Quatre et non cinq : c'est le découpage des piluliers vendus, de la
/// préparation des doses à administrer et de la façon dont une
/// ordonnance s'écrit. Une cinquième colonne « dans la journée » a été
/// essayée puis retirée — elle recueillait ce que la table n'avait pas
/// su placer, c'est-à-dire qu'elle donnait une case à une incertitude.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Moment {
    Matin,
    Midi,
    Soir,
    Coucher,
}

impl Moment {
    /// Dans l'ordre de la journée — l'ordre des colonnes.
    pub const ALL: [Moment; 4] = [Moment::Matin, Moment::Midi, Moment::Soir, Moment::Coucher];

    /// Ce que la colonne porte en tête.
    pub fn label(self) -> &'static str {
        match self {
            Moment::Matin => "Matin",
            Moment::Midi => "Midi",
            Moment::Soir => "Soir",
            Moment::Coucher => "Coucher",
        }
    }

    /// L'index de sa colonne.
    pub fn index(self) -> usize {
        match self {
            Moment::Matin => 0,
            Moment::Midi => 1,
            Moment::Soir => 2,
            Moment::Coucher => 3,
        }
    }
}

/// Ce qu'une case porte.
///
/// La quantité est comptée en **quarts d'unité** : un comprimé
/// quadrisécable se prend par quart — l'hémigoxine n'existe qu'en
/// 0,125 mg quadrisécable, et une officine doit pouvoir écrire le quart
/// qu'un patient prend réellement. Compter en entiers dirait que ce
/// quart n'existe pas ; il est dans la boîte à pilules.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Take {
    /// En quarts : 2 = ½, 4 = 1, 6 = 1 ½. `None` quand la phrase donne
    /// le moment sans donner le nombre — la case porte alors une marque.
    pub quarters: Option<u32>,
}

impl Take {
    /// Ce qui s'écrit dans la case. `None` quand la quantité n'est pas
    /// dite : c'est au dessin de poser sa marque, et non à ce module
    /// d'écrire un « 1 » que personne n'a prescrit.
    pub fn label(self) -> Option<String> {
        let q = self.quarters?;
        let whole = q / 4;
        let rest = q % 4;
        let frac = match rest {
            0 => "",
            1 => "¼",
            2 => "½",
            _ => "¾",
        };
        Some(match (whole, frac) {
            (0, "") => "0".to_owned(),
            (0, f) => f.to_owned(),
            (w, "") => w.to_string(),
            (w, f) => format!("{w} {f}"),
        })
    }
}

/// Ce que la lecture a trouvé.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    /// Au moins une prise est placée dans la journée : la grille se
    /// dessine.
    Placed,
    /// Une quantité par jour dont le moment n'est pas dit. La grille ne
    /// coche rien et la feuille laisse l'heure à convenir.
    Daily,
    /// À la demande : hors grille, avec sa condition.
    OnDemand,
    /// Un rythme qui n'est pas quotidien — hebdomadaire, mensuel, un
    /// jour sur deux. Hors grille, en toutes lettres.
    Cyclic,
    /// La table n'a pas su lire. Le texte passe tel quel.
    Unread,
}

/// La posologie, lue.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Reading {
    pub kind: Kind,
    /// Les quatre moments, dans l'ordre de [`Moment::ALL`].
    pub doses: [Option<Take>; 4],
    /// Combien de prises par jour quand le moment n'est pas dit —
    /// seulement pour [`Kind::Daily`].
    pub times: u32,
    /// Ce qui se dit à côté de la grille : le rythme, la condition, le
    /// repas. Vide quand la phrase n'en porte pas.
    pub note: String,
    /// Le rapport au repas, tel qu'il est écrit. Vide quand la phrase
    /// n'en dit rien — et un silence sur le repas n'est pas « pendant ».
    pub meal: String,
}

impl Reading {
    /// Une lecture qui ne place rien.
    fn empty(kind: Kind) -> Self {
        Self {
            kind,
            doses: [None; 4],
            times: 0,
            note: String::new(),
            meal: String::new(),
        }
    }

    /// La grille se dessine-t-elle ? Seule [`Kind::Placed`] répond oui :
    /// tout le reste laisserait des cases vides qui se lisent « rien à
    /// prendre ».
    pub fn has_grid(&self) -> bool {
        self.kind == Kind::Placed && self.doses.iter().any(Option::is_some)
    }

    /// Combien de prises la journée porte, grille comprise.
    pub fn takes_a_day(&self) -> u32 {
        match self.kind {
            Kind::Placed => self.doses.iter().filter(|d| d.is_some()).count() as u32,
            Kind::Daily => self.times,
            _ => 0,
        }
    }
}

/// Les mots qui désignent chaque moment, après pliage.
///
/// « dejeuner » est le midi et « petit dejeuner » le matin : le second
/// contient le premier, et c'est pour cela que la normalisation
/// remplace le petit-déjeuner avant que ces mots ne soient cherchés.
const MATIN: &[&str] = &["matin", "reveil", "lever"];
const MIDI: &[&str] = &["midi", "dejeuner"];
const SOIR: &[&str] = &["soir", "diner", "souper"];
const COUCHER: &[&str] = &["coucher", "au lit", "dormir", "nuit"];

/// Ce qui fait d'une prise une prise à la demande.
///
/// **Le bare « en cas de » n'y est pas, et c'est mesuré** : sur les
/// fiches livrées il attrape trente-deux lignes, dont presque aucune
/// n'est une posologie — « arrêt immédiat et avis médical en cas de
/// douleur tendineuse », « ne pas délivrer en cas de varicelle »,
/// « consulter en cas de fièvre ». C'est la locution de
/// l'avertissement, pas celle de la demande, et nommer les conditions
/// plutôt que la locution est la règle que ce dépôt applique déjà aux
/// abréviations de classe.
const ON_DEMAND: &[&str] = &[
    "si besoin",
    "au besoin",
    "a la demande",
    "si necessaire",
    "en cas de besoin",
    "si douleur",
    "si fievre",
    "si crise",
    "si insomnie",
    "si angoisse",
    "si anxiete",
    "si nausee",
    "si vomissement",
    "si constipation",
    "si diarrhee",
    "si gene",
    "si reveil",
    "si difficulte",
    "si mal ",
    "si migraine",
    "si glycemie",
    "si ta ",
    "si inr",
    "si poids",
    "en cas d'insomnie",
    "en cas de reveil",
    "ponctuel",
    // Le début d'une crise est une demande : « 50 mg dès le début de
    // la céphalée » est un triptan, et la fiche lui demandait une heure
    // fixe (« horaire à noter »). Pas « renouvelable » seul : « à
    // renouveler 15 à 21 jours plus tard » (le Vermox) est une seconde
    // prise datée, pas une demande.
    "des le debut",
    "des les premier",
    "au moment des",
    "interdose",
];

/// **Une demande gouverne une prise, jamais une dose.**
///
/// « Si besoin » se dit de deux choses dans une ordonnance, et une
/// seule sort de la grille. « 1 comprimé à renouveler si besoin toutes
/// les 6 heures » est une prise à la demande ; « 5 mg une fois par
/// jour, portés **si besoin** à 10 mg » est une titration, et le
/// traitement se prend tous les jours. Quinze fiches livrées portent
/// la seconde forme — Coversyl, Amlor, Cozaar, Aprovel, Cardensiel,
/// Jardiance, Micardis… — et elles sortaient toutes de la grille du
/// jour avec « à la demande » écrit devant, sur la feuille d'un
/// antihypertenseur.
///
/// Ce qui départage est ce qui vient **avant** le mot : un rythme
/// quotidien déjà énoncé, ou un verbe de titration. Après lui, un
/// « maximum 1 200 mg par jour » est un plafond et ne dit rien du
/// rythme — c'est pourquoi la position compte et non la simple
/// présence.
const NOT_ON_DEMAND_BEFORE: &[&str] = &["par jour", "chaque jour", "tous les jours", "quotidien"];

/// Les verbes de titration : « portés à », « augmentation par paliers »,
/// « jusqu'à … si nécessaire ». Eux seuls valent où qu'ils soient avant.
const TITRATION: &[&str] = &["porte", "augment", "jusqu'a", "palier"];

/// Les seuls mots qui, collés à un rythme quotidien, le rendent
/// conditionnel. Pas « au moment des repas », ni « dès les premières
/// heures », ni « plutôt qu'à la demande » : ceux-là suivent un rythme sans
/// le suspendre.
const CONDITION: &[&str] = &[
    "si besoin",
    "au besoin",
    "si necessaire",
    "en cas de besoin",
    "si douleur",
    "si fievre",
    "si crise",
];

/// **La demande gouverne-t-elle une dose ?** Un rythme quotidien le dit,
/// sauf si la condition le suit de près sans autre dose entre eux, et qu'il
/// est le seul de la phrase : « 1 cp 3 fois par jour si besoin » est une prise
/// à la demande, qui se dessinait trois fois par jour ; « 10 mg par jour,
/// puis 20 mg par jour ; 40 mg par jour si nécessaire » est une titration.
fn governs_a_dose(before: &str, marker: &str) -> bool {
    // « jusqu'à 3 fois par jour » caps a rhythm, it titrates nothing.
    let capped = |w: &str| {
        w == "jusqu'a"
            && before.match_indices(w).all(|(at, _)| {
                let mut after = before[at + w.len()..].split_whitespace();
                after.next().is_some_and(|n| n.ends_with("/j"))
                    || after.next().is_some_and(|n| n.starts_with("fois"))
            })
    };
    if TITRATION.iter().any(|w| before.contains(w) && !capped(w)) {
        return true;
    }
    let said: Vec<(usize, &str)> = NOT_ON_DEMAND_BEFORE
        .iter()
        .flat_map(|w| before.match_indices(*w))
        .collect();
    match said.as_slice() {
        [] => false,
        [(at, w)] if CONDITION.contains(&marker) => {
            let rest = &before[at + w.len()..];
            rest.contains(|c: char| c.is_ascii_digit())
                || (rest.contains([',', ';']) && rest.contains(char::is_alphabetic))
        }
        _ => true,
    }
}

/// **Un intervalle plus long qu'un jour.** La liste [`CYCLIC`] énumérait :
/// « toutes les deux semaines » y était, « toutes les 2 semaines » non, et la
/// ligne se dessinait chaque jour. Le nombre se lit ici, écrit ou en
/// chiffres, suivi aussitôt de son unité : « tous les 10 à 15 jours » est le
/// pas d'une titration, comme tout ce qui suit un verbe de titration.
fn spaced(folded: &str) -> bool {
    if abbreviated_rhythm(folded) {
        return true;
    }
    const SLOWER: [&str; 6] = [
        "palier",
        "augment",
        "doubl",
        "diminu",
        "titrat",
        "progressi",
    ];
    const WATCHED: [&str; 4] = ["reevalu", "control", "surveill", "bilan"];
    let tokens: Vec<&str> = folded
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    let count = |t: &str| -> Option<u32> {
        const WORDS: [&str; 11] = [
            "deux", "trois", "quatre", "cinq", "six", "sept", "huit", "neuf", "dix", "onze",
            "douze",
        ];
        t.parse()
            .ok()
            .or_else(|| WORDS.iter().position(|w| *w == t).map(|i| i as u32 + 2))
    };
    let days = |n: u32, unit: &str| -> bool {
        (unit.starts_with("jour") && n > 1)
            || unit.starts_with("semaine")
            || unit == "mois"
            || (unit.starts_with("heure") && n > 24)
    };
    tokens.iter().enumerate().any(|(i, t)| {
        let after = tokens.get(i + 1).copied();
        let every = matches!(*t, "tous" | "toutes") && after == Some("les");
        let (first, second) = (tokens.get(i + 2).copied(), tokens.get(i + 3).copied());
        let every = every
            && match (first.and_then(count), first, second) {
                (Some(n), _, Some(unit)) => days(n, unit),
                (None, Some("mois"), _) => true,
                _ => false,
            };
        // « un soir sur deux », « 1 jour sur 3 ».
        let moment = matches!(*t, "jour" | "soir" | "matin" | "nuit");
        let one = i > 0 && matches!(tokens[i - 1], "un" | "une" | "1");
        let ratio = moment && one && after == Some("sur") && first.and_then(count).is_some();
        let said = tokens[..i].iter();
        (every || ratio)
            && !said
                .clone()
                .any(|w| SLOWER.iter().any(|s| w.starts_with(s)))
            && !tokens[..i]
                .iter()
                .rev()
                .take(2)
                .any(|w| WATCHED.iter().any(|s| w.starts_with(s)))
    })
}

/// the written-short rhythms. « 1j/2 », « 5 j sur 7 », « 21j/28 »,
/// « 3 sem sur 4 », « 2 soirs sur 3 », « tous les 2j », « toutes les 72 h »,
/// « tous les 3 soirs ».
fn abbreviated_rhythm(folded: &str) -> bool {
    // Digits and letters apart: « 5j/7 » is 5, j, /, 7.
    let mut t: Vec<String> = Vec::new();
    let mut cur = String::new();
    for c in folded.chars() {
        let same = cur
            .chars()
            .last()
            .is_none_or(|l| l.is_ascii_digit() == c.is_ascii_digit());
        if c.is_alphanumeric() && same {
            cur.push(c);
            continue;
        }
        if !cur.is_empty() {
            t.push(std::mem::take(&mut cur));
        }
        if c.is_alphanumeric() {
            cur.push(c);
        } else if c == '/' {
            t.push("/".to_owned());
        }
    }
    if !cur.is_empty() {
        t.push(cur);
    }
    let num = |s: Option<&String>| s.and_then(|s| s.parse::<u32>().ok());
    let span = |u: &str| {
        matches!(
            u,
            "j" | "jour"
                | "jours"
                | "sem"
                | "soir"
                | "soirs"
                | "nuit"
                | "nuits"
                | "matin"
                | "matins"
        )
    };
    (0..t.len()).any(|i| {
        let unit = t.get(i + 1).map_or("", |s| s.as_str());
        let n = num(t.get(i));
        // N unit / M, N unit sur M.
        let ratio = n.is_some()
            && span(unit)
            && matches!(t.get(i + 2).map(|s| s.as_str()), Some("/" | "sur"))
            && num(t.get(i + 3)).is_some();
        // tous les N unit.
        let every = i >= 2
            && matches!(t[i - 2].as_str(), "tous" | "toutes")
            && t[i - 1] == "les"
            && n.is_some_and(|n| match unit {
                "j" => n > 1,
                "h" => n > 24,
                "soirs" | "matins" | "nuits" | "sem" => true,
                _ => false,
            });
        ratio || every
    })
}

/// « matin ou soir », « le matin ou le soir » : un choix, pas deux prises.
/// Une grille ne dit pas un choix.
fn moment_choice(folded: &str) -> bool {
    let tokens: Vec<&str> = folded.split_whitespace().collect();
    let is_moment = |i: usize| tokens.get(i).is_some_and(|t| moment_of(t).is_some());
    tokens.iter().enumerate().any(|(i, t)| {
        t.trim_matches(|c: char| !c.is_alphanumeric()) == "ou"
            && i > 0
            && is_moment(i - 1)
            && (is_moment(i + 1)
                || (tokens
                    .get(i + 1)
                    .is_some_and(|a| matches!(*a, "le" | "la" | "au" | "a"))
                    && is_moment(i + 2))
                // « matin ou 1 cp soir », « le matin ou 1 cp le soir ».
                || (1..=4).any(|k| is_moment(i + k)
                    && tokens.get(i + 1).is_some_and(|a| a.starts_with(|c: char| c.is_ascii_digit()))))
    }) || (tokens.iter().filter(|t| **t == "soit").count() >= 2)
}

/// Ce qui fait d'un rythme un rythme non quotidien.
///
/// **Une durée n'est pas un rythme**, et c'est le seul piège de cette
/// liste. « pendant 7 jours » n'en est pas un : elle ne doit pas faire
/// sortir de la grille un traitement qui se prend bien tous les jours.
/// C'est la raison pour laquelle chaque motif porte son « tous les »,
/// son « par » ou son « sur » — le mot « jours » seul désigne une durée
/// neuf fois sur dix. La liste a porté « cure de », qui est exactement
/// la faute qu'elle prétendait éviter : sur les fiches livrées, il ne
/// nomme que deux traitements **quotidiens** décrits par la durée de
/// leur cure — « 10 mg par jour, en cure de quelques jours à quelques
/// semaines » —, et il les sortait tous deux de la grille du jour.
/// `every_rhythm_word_names_a_repetition_and_not_a_duration` refuse le
/// prochain.
const CYCLIC: &[&str] = &[
    "par semaine",
    "/semaine",
    "une fois par semaine",
    "hebdomadaire",
    "par mois",
    "mensuel",
    "tous les lundis",
    "tous les mardis",
    "tous les mercredis",
    "tous les jeudis",
    "tous les vendredis",
    "tous les samedis",
    "tous les dimanches",
    "un jour sur deux",
    // L'Imovane « en espaçant les prises une nuit sur deux » se
    // dessinait chaque soir au coucher.
    "nuit sur deux",
    "1 jour sur 2",
    "jour sur deux",
    "tous les deux jours",
    "toutes les semaines",
    "toutes les deux semaines",
    "tous les trois jours",
    "tous les 2 jours",
    "tous les 3 jours",
    "tous les 7 jours",
    "toutes les 4 semaines",
];

/// written short, or no rhythm word at all, and still not a daily take.
const NOT_DAILY: &[&str] = &[
    "/sem",
    "par sem",
    "hebdo",
    "/mois",
    "week-end",
    "week end",
    "weekend",
    "jour oui",
    "jour sur l'autre",
    "en alternance",
    "lendemain matin",
    "lendemain soir",
    "veille au soir",
    // a one-off before an examination: « 1 sachet à 20h la veille de la
    // coloscopie » is no evening of every day.
    "la veille de la ",
    "la veille de l'",
    "avant l'intervention",
    "jours, arret",
    "jours d'arret",
    " a j1",
    "cycle",
    "chaque cure",
    "chimio",
    "avant l'examen",
];

/// Le rapport au repas, tel qu'il s'imprime.
///
/// La forme rendue est la forme livrée, et non celle qu'a tapée
/// l'officine : la fiche porte une phrase courte et régulière d'une
/// ligne à l'autre, et « au cours du repas » ne mérite pas une colonne
/// différente de « pendant le repas ».
const MEALS: &[(&str, &str)] = &[
    ("a jeun", "À jeun"),
    ("a distance des repas", "À distance des repas"),
    ("avant le repas", "Avant le repas"),
    ("avant les repas", "Avant le repas"),
    ("avant le petit dejeuner", "Avant le repas"),
    ("avant le petit-dejeuner", "Avant le repas"),
    ("au debut du repas", "Au début du repas"),
    ("au cours du repas", "Pendant le repas"),
    ("pendant le repas", "Pendant le repas"),
    ("pendant les repas", "Pendant le repas"),
    ("au milieu du repas", "Pendant le repas"),
    ("apres le repas", "Après le repas"),
    ("apres les repas", "Après le repas"),
    ("en dehors des repas", "En dehors des repas"),
];

/// Les unités qui font d'un nombre un **dosage** et non une quantité.
const STRENGTHS: &[&str] = &["mg", "g", "µg", "mcg", "ug", "ml", "ui", "%", "mmol", "kg"];

/// **L'unité d'insuline est ce qu'on injecte** : « 20 UI le soir » porte
/// 20 dans la colonne du soir, comme « 10-0-10 UI » et « 14 unités ». Sauf
/// dans une phrase qui l'adapte ([`ADAPTED`]) : la case garde sa marque.
/// Au-delà de 40 ([`quarters`]), le nombre n'est pas lu.
const INSULIN: &[&str] = &["ui", "u"];

/// Ce qui fait d'une dose d'insuline écrite un point de départ et non une
/// dose : « 6 UI le matin selon glycémie ». Imprimer 6 dirait une dose fixe.
const ADAPTED: &[&str] = &[
    "selon",
    "adapt",
    "ajust",
    "protocole",
    "schema",
    "correction",
    "glycemie",
    "dextro",
    "+/-",
    "titr",
];

fn insulin_counts(folded: &str) -> bool {
    !ADAPTED.iter().any(|w| folded.contains(w))
}

/// Les unités qui font d'un nombre une **quantité**.
const UNITS: &[&str] = &[
    "comprime",
    "comprimes",
    "cp",
    "cps",
    "cpr",
    "cprs",
    "gelule",
    "gelules",
    "gel",
    "gels",
    "sachet",
    "sachets",
    "goutte",
    "gouttes",
    "bouffee",
    "bouffees",
    "dose",
    "doses",
    "ampoule",
    "ampoules",
    "cuillere",
    "cuilleres",
    "pulverisation",
    "pulverisations",
    "suppositoire",
    "suppositoires",
    "patch",
    "patchs",
    "application",
    "applications",
    "unite",
    "unites",
    "inhalation",
    "inhalations",
];

/// Lire une posologie.
///
/// Le texte est celui du dossier — celui de *ce* patient, tel qu'il est
/// sur son ordonnance — et non la posologie usuelle de la molécule.
pub fn read(posology: &str) -> Reading {
    let raw = posology.trim();
    if raw.is_empty() {
        return Reading::empty(Kind::Unread);
    }
    // Deux états du même texte. Le pliage seul sert à chercher le
    // repas — la normalisation remplace « petit-déjeuner » par
    // « matin », ce qui est juste pour le moment de la journée et
    // efface le repas ; la normalisation sert à tout le reste.
    let plain = crate::fuzzy::sort_key(raw);
    let folded = normalize(&plain);

    // **« Si besoin » d'abord.** Une phrase peut porter les deux — « 1
    // comprimé le soir, et 1 de plus si besoin » — et c'est alors la
    // condition qui commande : ce qui n'est pas systématique n'entre
    // pas dans un pilulier.
    //
    // Sauf quand la demande gouverne une **dose** et non une prise :
    // voir [`NOT_ON_DEMAND_BEFORE`], qui regarde ce qui précède le mot.
    if let Some((at, m)) = ON_DEMAND
        .iter()
        .filter_map(|m| folded.find(*m).map(|at| (at, m)))
        .min_by_key(|(at, _)| *at)
    {
        if !governs_a_dose(&folded[..at], m) {
            let mut r = Reading::empty(Kind::OnDemand);
            r.note = condition(raw, m);
            r.meal = meal_of(&plain);
            return r;
        }
    }

    // **Puis le rythme.** Avant de chercher les moments, parce que « le
    // lundi matin » porte « matin » et n'est pas une prise du matin.
    // **Un schéma en phases n'est pas une journée.** « 1 cp le soir
    // pendant 7 jours puis 1 cp matin et soir » se dessinait matin et soir
    // dès le premier jour — la première semaine doublée sur le plan
    // imprimé —, et « 2 cp le matin pendant 3 jours puis 1 » perdait sa
    // dose de charge. Une grille ne dit pas une suite : la ligne en sort,
    // avec la posologie écrite en note, comme un rythme.
    // Sur le texte tel qu'écrit : le repli retire les parenthèses.
    let phased = phased(&raw.to_lowercase());
    if let Some(m) = CYCLIC
        .iter()
        .copied()
        .find(|m| folded.contains(*m))
        .or_else(|| NOT_DAILY.iter().copied().find(|m| folded.contains(*m)))
        // « 2 fois par an », and not « par année d'âge ».
        .or_else(|| {
            folded
                .split_whitespace()
                .collect::<Vec<_>>()
                .windows(2)
                .any(|w| w[0] == "par" && w[1].trim_end_matches([',', '.']) == "an")
                .then_some("par an")
        })
        .or_else(|| {
            let at = folded.find("chaque semaine")?;
            let slower = ["augment", "palier", "diminu", "doubl", "progressi"];
            (!slower.iter().any(|w| folded[..at].contains(w))).then_some("chaque semaine")
        })
        .or_else(|| {
            // A one-off, unless the phrase counts days.
            folded
                .contains("dose unique")
                .then_some("unique")
                .filter(|_| !folded.contains("par jour") && !folded.contains("/j"))
        })
        .or_else(|| weekly_by_name(&folded))
        .or_else(|| {
            folded
                .split(|c: char| !c.is_alphanumeric())
                .any(|w| {
                    w.strip_prefix('j')
                        .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
                })
                .then_some("j")
        })
        .or(spaced(&folded).then_some("tous les"))
        .or(phased.then_some("puis"))
    {
        let mut r = Reading::empty(Kind::Cyclic);
        r.note = rhythm(raw, m);
        r.meal = meal_of(&plain);
        return r;
    }

    let meal = meal_of(&plain);

    // La notation chiffrée, quand elle est là : « 1-0-1 » se lit sans
    // ambiguïté et court-circuite tout le reste.
    if let Some(doses) = notation(&folded) {
        if doses.iter().all(Option::is_none) {
            let mut r = Reading::empty(Kind::Unread);
            r.meal = meal;
            return r;
        }
        return Reading {
            kind: Kind::Placed,
            doses,
            times: 0,
            note: String::new(),
            meal,
        };
    }

    // Un choix de moment : autant de prises que la phrase en dit, une
    // sinon. « 1 cp matin ou soir » comptait deux prises, et la boîte la
    // moitié de sa durée.
    if moment_choice(&folded) {
        return Reading {
            kind: Kind::Daily,
            doses: [None; 4],
            times: per_day(&folded).unwrap_or(1),
            note: String::new(),
            meal,
        };
    }

    let (doses, lost) = placed(&folded);
    // the takes the columns cannot hold.
    let lost = lost
        || folded.contains("trois repas")
        || folded.contains("3 repas")
        || folded.contains("chaque repas")
        || folded.contains("repas et ")
        || unnamed_condition(&folded)
        || short_interval(&folded);
    // **Une quantité sans moment connu ne disparaît pas.** « 1 cp à 8h, 1
    // cp à 16h, 1 le soir » se lisait une prise le soir : la phrase n'est
    // pas lue, et le dire vaut mieux qu'une dose sur trois.
    if lost && doses.iter().any(Option::is_some) {
        let mut r = Reading::empty(Kind::Unread);
        r.meal = meal;
        return r;
    }
    // « 1 cp le soir en cas de douleur »: with a dose placed, this is
    // no warning. Alone it stays one: « arrêt immédiat en cas de douleur ».
    if doses.iter().any(Option::is_some)
        && ["en cas de douleur", "en cas de fievre"]
            .iter()
            .any(|w| folded.contains(w))
    {
        let mut r = Reading::empty(Kind::OnDemand);
        r.note = condition(raw, "en cas de");
        r.meal = meal;
        return r;
    }
    if doses.iter().any(Option::is_some) {
        return Reading {
            kind: Kind::Placed,
            doses,
            times: 0,
            note: String::new(),
            meal,
        };
    }

    // Aucun moment nommé : reste-t-il une quantité par jour ?
    if let Some(times) = per_day(&folded) {
        return Reading {
            kind: Kind::Daily,
            doses: [None; 4],
            times,
            note: String::new(),
            meal,
        };
    }

    let mut r = Reading::empty(Kind::Unread);
    r.meal = meal;
    r
}

/// A condition [`ON_DEMAND`] does not name: « si pas de selles », « en cas
/// de diarrhée ». The grid cannot say it, so the phrase is not read as one.
fn unnamed_condition(folded: &str) -> bool {
    let w: Vec<&str> = folded.split_whitespace().collect();
    folded.contains("en cas d")
        || w.windows(2).any(|p| {
            p[0].trim_start_matches(|c: char| !c.is_alphanumeric()) == "si"
                && !p[1].starts_with("possible")
                && !p[1].starts_with("oubli")
                && !p[1].starts_with("besoin")
                && !p[1].starts_with("necessaire")
        })
}

/// Aplanir ce qui piège la recherche de mots.
///
/// Trois substitutions, et chacune répare une lecture fausse :
/// « petit dejeuner » contient « dejeuner », qui est le midi ; « soir
/// au coucher » est **une** prise et non deux ; la ponctuation colle
/// les mots aux nombres.
fn normalize(folded: &str) -> String {
    let mut s = folded.to_owned();
    for (from, to) in [
        ("petit-dejeuner", "matin"),
        ("petit dejeuner", "matin"),
        ("petit dej", "matin"),
        // « en début d'après-midi » n'est pas le midi : c'est le mot
        // qui contient l'autre, la famille du « petit déjeuner » juste
        // au-dessus. Le Lasilix l'écrit, et une deuxième prise placée
        // au déjeuner n'est pas celle qui a été prescrite.
        ("apres midi", "apresmidi"),
        ("soir au coucher", "coucher"),
        ("soir avant le coucher", "coucher"),
        ("soir, au coucher", "coucher"),
        ("soir avant de dormir", "coucher"),
        ("soir avant de se coucher", "coucher"),
        ("soir au moment du coucher", "coucher"),
        ("soir a l'heure du coucher", "coucher"),
        ("soir au lit", "coucher"),
        ("soir pour bien dormir", "soir"),
        ("soir pour dormir", "soir"),
        ("soir pour la nuit", "soir"),
        ("avant le coucher et apres le repas du soir", "soir"),
        ("du matin au soir", "dans la journee"),
        ("du lever au coucher", "dans la journee"),
        ("pt dejeuner", "matin"),
        ("p'tit dejeuner", "matin"),
        ("petit-dej", "matin"),
    ] {
        while let Some(i) = s.find(from) {
            s.replace_range(i..i + from.len(), to);
        }
    }
    // an empty cell, « Midi : ; Soir : 1 ».
    let s = s
        .replace(": ;", " - ")
        .replace(": ,", " - ")
        .replace(": /", " - ");
    let s = s.replace(['(', ')', ';', ':'], " ");
    // « matin/soir », « matin-midi-soir » : des moments joints par un
    // trait ou une barre sont autant de moments. Le jeton entier se
    // lisait sur son premier mot — « matin » — et la grille imprimée
    // perdait le midi et le soir. Seulement quand **chaque** morceau
    // est un moment : « 1/2 », « 1-0-1 » restent la notation.
    let s = s
        .split(' ')
        .map(|tok| {
            let parts: Vec<&str> = tok.split(['-', '/', '+', ',', '&']).collect();
            if parts.len() > 1
                && parts
                    .iter()
                    .all(|p| !p.is_empty() && moment_of(p).is_some())
            {
                parts.join(" ")
            } else {
                tok.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    clock_hours(&s)
}

/// The column a clock hour falls in, in minutes of the day. The gaps are
/// on purpose: 16h is a goûter, 21h either evening or bedtime, and a
/// take with no column of its own leaves the phrase unread.
fn column_of_hour(minutes: u32) -> Option<&'static str> {
    match minutes {
        300..=630 => Some("matin"),
        660..=840 => Some("midi"),
        1080..=1230 => Some("soir"),
        1290..=1439 => Some("coucher"),
        _ => None,
    }
}

/// « 8h », « 08h30 », « 20h00 », in minutes of the day.
fn clock(token: &str) -> Option<u32> {
    let t = token.trim_end_matches([',', '.']);
    let (h, m) = t.split_once('h')?;
    if h.is_empty() || h.len() > 2 || !h.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    if !(m.is_empty() || (m.len() == 2 && m.chars().all(|c| c.is_ascii_digit()))) {
        return None;
    }
    let (h, m): (u32, u32) = (
        h.parse().ok()?,
        if m.is_empty() { 0 } else { m.parse().ok()? },
    );
    (h < 24 && m < 60).then_some(h * 60 + m)
}

/// **A clock hour is a moment** when the phrase names none: « 1 cp à 8h
/// et 20h » is a morning and an evening take. Only an hour said as one
/// (« à », « vers », after the dose's unit, or in a list of hours), never
/// a delay (« 2h avant », « toutes les 12h »). An hour with no column is
/// left as written, and the phrase stays unread.
fn clock_hours(s: &str) -> String {
    let mut tokens: Vec<String> = s.split_whitespace().map(str::to_owned).collect();
    if tokens.iter().any(|t| moment_of(t).is_some()) {
        return s.to_owned();
    }
    // « 8 h » is « 8h ».
    let mut i = 0;
    while i + 1 < tokens.len() {
        let next = tokens[i + 1].trim_end_matches([',', '.']);
        if next == "h" && tokens[i].chars().all(|c| c.is_ascii_digit()) && !tokens[i].is_empty() {
            let glued = format!("{}{}", tokens[i], tokens[i + 1]);
            tokens[i] = glued;
            tokens.remove(i + 1);
        }
        i += 1;
    }
    let mut previous_hour = false;
    for i in 0..tokens.len() {
        let parts: Vec<&str> = tokens[i].split(['-', '/']).collect();
        let minutes: Option<Vec<u32>> = parts.iter().map(|p| clock(p)).collect();
        let Some(minutes) = minutes.filter(|m| !m.is_empty()) else {
            previous_hour = previous_hour && tokens[i] == "et";
            continue;
        };
        let before = i
            .checked_sub(1)
            .map(|b| {
                tokens[b]
                    .trim_matches(|c: char| !c.is_alphanumeric())
                    .to_owned()
            })
            .unwrap_or_default();
        let after = tokens
            .get(i + 1)
            .map(|a| a.trim_matches(|c: char| !c.is_alphanumeric()).to_owned())
            .unwrap_or_default();
        let said = matches!(before.as_str(), "a" | "vers")
            || UNITS.contains(&before.as_str())
            || previous_hour
            || parts.len() > 1;
        let delay = matches!(after.as_str(), "avant" | "apres" | "plus" | "de" | "d");
        let columns: Option<Vec<&str>> = minutes.iter().map(|m| column_of_hour(*m)).collect();
        previous_hour = true;
        if !said || delay {
            continue;
        }
        if let Some(columns) = columns {
            let comma = if tokens[i].ends_with(',') { "," } else { "" };
            tokens[i] = format!("{}{comma}", columns.join(" "));
        }
    }
    tokens.join(" ")
}

/// Ce que porte la ligne d'un traitement à la demande : sa condition,
/// dans les mots de l'ordonnance.
///
/// La phrase d'origine est reprise à partir du repère, accents compris :
/// c'est elle qui dit *quand* — « en cas de douleur », « si la fièvre
/// dépasse 38,5 » — et une condition résumée est une condition que le
/// patient applique de travers.
fn condition(raw: &str, marker: &str) -> String {
    // Le repère se cherche dans le texte **seulement plié**, pas
    // normalisé : la normalisation raccourcit (« petit-déjeuner » devient
    // « matin »), et la condition imprimée commençait neuf caractères
    // trop loin — « déjeuner si besoin ».
    let folded = crate::fuzzy::sort_key(raw);
    let Some(byte) = folded.find(marker) else {
        return raw.to_owned();
    };
    // Le pliage conserve le nombre de caractères (il ne fait que
    // minuscule et accent), donc l'index en caractères vaut des deux
    // côtés. Il ne conserve pas le nombre d'**octets** : « é » en fait
    // deux et « e » un seul.
    let chars = folded[..byte].chars().count();
    raw.chars()
        .skip(chars)
        .collect::<String>()
        .trim()
        .to_owned()
}

/// Un « puis » qui **ouvre une seconde phase** — presque tous : dose de
/// charge (« 180 mg en une prise, puis 90 mg »), titration (« pendant 2
/// semaines, puis doublement »), « 2 cp à J1 puis 1 cp/j ». La ligne
/// sort alors de la grille du jour, et c'est le sens qui ne coûte
/// qu'une question.
///
/// Deux exceptions seulement, celles que le mot seul attrapait à tort :
/// un « puis » **entre parenthèses** (la Coversyl livrée : « soit 4 mg
/// puis 8 mg pour la forme tert-butylamine », une équivalence de sel),
/// et un « puis » suivi d'**un geste** et non d'une dose (« matin et
/// soir puis se rincer la bouche », la consigne de tout corticoïde
/// inhalé). Une première version exigeait une durée avant et une dose
/// après : la dose de charge du Brilique, la titration du Kredex et le
/// J1 de l'azithromycine s'y perdaient.
fn phased(folded: &str) -> bool {
    const GESTURES: [&str; 14] = [
        "se",
        "rincer",
        "recracher",
        "cracher",
        "attendre",
        "laver",
        "essuyer",
        "agiter",
        "secouer",
        "refermer",
        "jeter",
        "nettoyer",
        "masser",
        "boire",
    ];
    let mut depth = 0_i32;
    let mut words: Vec<(&str, bool)> = Vec::new();
    let mut start = None;
    for (i, c) in folded.char_indices() {
        if c.is_alphanumeric() {
            start.get_or_insert(i);
            continue;
        }
        if let Some(st) = start.take() {
            words.push((&folded[st..i], depth > 0));
        }
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = (depth - 1).max(0),
            _ => {}
        }
    }
    if let Some(st) = start {
        words.push((&folded[st..], depth > 0));
    }
    words.iter().enumerate().any(|(at, (w, inside))| {
        let next = words.get(at + 1).map(|(next, _)| *next);
        !inside
            && match *w {
                "puis" => !next.is_some_and(|n| GESTURES.contains(&n)),
                // « ensuite 1 cp », and not « adaptée ensuite ».
                "ensuite" => next.is_some_and(|n| n.starts_with(|c: char| c.is_ascii_digit())),
                _ => false,
            }
    })
}

/// Ce que porte la ligne d'un traitement à rythme non quotidien.
///
/// Toute la phrase, et non le seul repère : « 1 comprimé par semaine,
/// le lundi matin » ne se résume pas à « par semaine », et c'est le
/// jour qui manque le plus au patient.
/// Un rythme écrit **par le nom d'un jour ou par un rapport**, que
/// [`CYCLIC`] ne porte pas : « 3 cp le lundi matin » (le méthotrexate,
/// la règle qui tue) se dessinait trois comprimés chaque matin, et
/// « 5 jours sur 7 », « 1 semaine sur 2 », « 21 jours sur 28 », « les
/// jours impairs », « sauf le dimanche » faisaient de même.
///
/// Un jour nommé comme **début ou fin** (« à partir du lundi », « dès
/// lundi », « jusqu'à lundi ») n'est pas un rythme. Dans le doute, la
/// ligne sort de la grille du jour : c'est le sens qui ne coûte qu'une
/// question, l'autre multiplie une dose hebdomadaire par sept.
fn weekly_by_name(folded: &str) -> Option<&'static str> {
    const RATIOS: [&str; 6] = [
        "jours sur ",
        "semaine sur ",
        "semaines sur ",
        "jours pairs",
        "jours impairs",
        "sauf le ",
    ];
    // « N semaines sur M » est un rapport ; « 2 à 4 semaines sur la
    // peau glabre » (la Lomexin livrée) est une durée et un lieu. Le
    // rapport porte un nombre juste après « sur ».
    let counted = |rest: &str| {
        let next = rest.split_whitespace().next().unwrap_or("");
        next.starts_with(|c: char| c.is_ascii_digit())
            || matches!(next, "deux" | "trois" | "quatre" | "sept")
    };
    if let Some(r) = RATIOS.iter().find(|r| {
        folded
            .match_indices(**r)
            .any(|(at, _)| !r.ends_with("sur ") || counted(&folded[at + r.len()..]))
    }) {
        return Some(r);
    }
    const DAYS: [&str; 7] = [
        "lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi", "dimanche",
    ];
    for day in DAYS {
        for (at, _) in folded.match_indices(day) {
            // « les lundis ».
            let rest = &folded[at + day.len()..];
            let rest = rest.strip_prefix('s').unwrap_or(rest);
            let whole = !folded[..at].ends_with(char::is_alphanumeric)
                && !rest.starts_with(char::is_alphanumeric);
            // Les trois mots d'avant : « a partir du », « des le »,
            // « jusqu'a », « a compter du ».
            // articles only between the bound word and the day.
            let bound = folded[..at]
                .split_whitespace()
                .rev()
                .take(3)
                .find(|w| !matches!(*w, "le" | "du" | "de" | "au"))
                .is_some_and(|w| {
                    matches!(w, "partir" | "des" | "compter" | "apres") || w.starts_with("jusqu")
                });
            if whole && !bound {
                return Some(day);
            }
        }
    }
    // « lun mer ven ».
    const SHORT: [&str; 7] = ["lun", "mar", "mer", "jeu", "ven", "sam", "dim"];
    folded
        .split(|c: char| !c.is_alphanumeric())
        .find_map(|w| SHORT.iter().find(|s| **s == w).copied())
}

fn rhythm(raw: &str, _marker: &str) -> String {
    raw.trim().to_owned()
}

/// Le rapport au repas, ou rien.
fn meal_of(folded: &str) -> String {
    // « au cours ou en dehors des repas » dit que le repas n'importe
    // pas, et la fiche imprimait « En dehors des repas » sous le nom.
    if [
        "ou en dehors",
        "avec ou sans",
        "ou pendant",
        "independamment des repas",
    ]
    .iter()
    .any(|w| folded.contains(w))
    {
        return String::new();
    }
    // Le plus précis d'abord : « avant le petit dejeuner » est devenu
    // « avant le matin » à la normalisation, et « a distance des
    // repas » contient « repas » comme les autres.
    MEALS
        .iter()
        .find(|(needle, _)| folded.contains(needle))
        .map_or_else(String::new, |(_, label)| (*label).to_owned())
}

/// La notation « 1-0-1 » ou « 1/0/1/0 », si c'en est une.
///
/// Trois nombres pour matin, midi, soir ; quatre en ajoutant le
/// coucher. Exigée **entière** : deux nombres seulement, c'est « 1/2 »,
/// c'est-à-dire un demi-comprimé, et lire une fraction comme une
/// notation serait la faute la plus bête de ce module.
fn notation(folded: &str) -> Option<[Option<Take>; 4]> {
    let words: Vec<&str> = folded.split_whitespace().collect();
    let at = words
        .iter()
        .position(|t| t.contains('-') || t.contains('/'))?;
    let token = words[at];
    let sep = if token.contains('-') { '-' } else { '/' };
    let parts: Vec<&str> = token.split(sep).collect();
    if parts.len() != 3 && parts.len() != 4 {
        return None;
    }
    // the grid alone is the posology. A quantity ahead of it
    // (« 2 cp 1-0-1 »), a dose after it (« 1-0-1 + 2 au coucher »), a second
    // grid (a phase) or a padded date (« 05-11-26 ») are not this grid.
    let clean = |w: &&str| {
        w.trim_matches(|c: char| !c.is_alphanumeric() && c != '½' && c != '¼' && c != '¾')
            .to_owned()
    };
    let counted = |ws: &[&str]| {
        ws.windows(2).any(|w| {
            quarters(&clean(&w[0])).is_some_and(|q| q != 4 || w[0] != "1")
                && UNITS.contains(&clean(&w[1]).as_str())
        })
    };
    let after = &words[at + 1..];
    let after_dose = after.iter().any(|w| moment_of(w).is_some())
        && after
            .iter()
            .any(|w| quarters(&clean(w)).is_some_and(|q| q > 0));
    let grid_again = after.iter().any(|w| w.split(sep).count() == parts.len());
    let padded = parts
        .iter()
        .any(|p| p.len() > 1 && p.starts_with('0') && !p.contains([',', '.']));
    if after_dose || grid_again {
        return Some([None; 4]);
    }
    if counted(&words[..at]) || counted(after) || padded {
        return None;
    }
    // **Une date n'est pas une grille** : « dès le 1/3/26 » se lisait
    // 1 le matin, 3 à midi, 26 le soir. Une date a un jour, **un mois
    // de 1 à 12** et une année de deux ou quatre chiffres ; « 20/0/20
    // gouttes » ou « 14/0/10 » d'une insuline n'en ont pas — refuser
    // toute case au-delà de 12 les perdait.
    //
    // Et seulement **une date qui en a l'air** : une année à quatre
    // chiffres, ou une date introduite (« le », « du », « dès »,
    // « depuis »…). « 10/10/10 gouttes » ou « 6/8/10 » d'une insuline
    // ont un jour, un mois et une année plausibles, et sont des grilles.
    if sep == '/' && parts.len() == 3 {
        let int = |p: &str| p.parse::<u32>().ok();
        let plausible = matches!(int(parts[0]), Some(1..=31))
            && matches!(int(parts[1]), Some(1..=12))
            && (parts[2].len() == 2 || parts[2].len() == 4);
        let introduced = at
            .checked_sub(1)
            .and_then(|i| words.get(i))
            .is_some_and(|w| {
                matches!(
                    *w,
                    "le" | "du" | "au" | "des" | "depuis" | "avant" | "apres"
                ) || w.starts_with("jusqu")
            });
        if plausible && (parts[2].len() == 4 || introduced) {
            return None;
        }
    }
    if parts.iter().any(|p| quarters(p).is_none()) {
        return None;
    }
    // **Un milligramme n'est pas un comprimé**, ici comme dans `placed` :
    // « 5-0-5 mg » comptait cinq unités par prise. L'unité d'insuline,
    // elle, est ce qu'on injecte : « 10-0-10 UI » reste une grille.
    if words
        .get(at + 1)
        .is_some_and(|u| matches!(*u, "mg" | "g" | "µg" | "mcg" | "ug" | "ml"))
    {
        return None;
    }
    let mut doses = [None; 4];
    for (i, p) in parts.iter().enumerate() {
        let q = quarters(p)?;
        if q > 0 {
            doses[i] = Some(Take { quarters: Some(q) });
        }
    }
    doses.iter().any(Option::is_some).then_some(doses)
}

/// Un nombre en quarts d'unité : « 1 » → 4, « 0,5 » → 2, « ½ » → 2.
fn quarters(text: &str) -> Option<u32> {
    let t = text.trim();
    match t {
        "½" | "1/2" => return Some(2),
        "¼" | "1/4" => return Some(1),
        "¾" | "3/4" => return Some(3),
        _ => {}
    }
    let t = t.replace(',', ".");
    let v: f64 = t.parse().ok()?;
    if !(0.0..=40.0).contains(&v) {
        return None;
    }
    let q = (v * 4.0).round();
    ((q - v * 4.0).abs() < 0.01).then_some(q as u32)
}

/// « toutes les 8 heures, y compris la nuit »: a rhythm under twelve
/// hours is no list of moments.
fn short_interval(folded: &str) -> bool {
    let t: Vec<&str> = folded.split_whitespace().collect();
    t.windows(4).any(|w| {
        w[0] == "toutes"
            && w[1] == "les"
            && w[2]
                .trim_end_matches(|c: char| !c.is_ascii_digit())
                .parse::<u32>()
                .is_ok_and(|n| n < 12)
            && (w[2].contains('h') || w.get(3).is_some_and(|u| u.starts_with('h')))
    })
}

/// Placer les prises nommées.
///
/// Un passage de gauche à droite : un nombre s'accroche au moment qui
/// le suit, et un moment sans nombre à lui reprend le dernier nombre
/// énoncé — c'est ainsi que « 1 comprimé matin, midi et soir » se lit,
/// et c'est la seule lecture que la langue autorise.
fn placed(folded: &str) -> ([Option<Take>; 4], bool) {
    let insulin = insulin_counts(folded);
    let counts = |w: &str| UNITS.contains(&w) || (insulin && INSULIN.contains(&w));
    if let Some(doses) = moments_first(folded, insulin) {
        return (doses, false);
    }
    let mut doses: [Option<Take>; 4] = [None; 4];
    // Une quantité comptée qui n'a pas trouvé de moment, quand les
    // quantités viennent avant les moments (« matin : 1 cp, soir : 1 cp »
    // finit sur une quantité sans rien perdre).
    let mut lost = false;
    let mut seen_moment = false;
    let tokens: Vec<&str> = folded.split_whitespace().collect();
    // Le dernier nombre énoncé, et celui qui n'a pas encore trouvé son
    // moment. Deux variables et non une : « 2 le matin et le soir » doit
    // donner 2 aux deux, tandis que « 1 comprimé le matin et 2 le soir »
    // ne doit pas donner 1 au soir.
    //
    // `Some(None)` est une prise dont la quantité est une fourchette —
    // « 1 à 2 comprimés le soir » : la case porte une marque, jamais un
    // des deux bouts.
    let mut pending: Option<Option<u32>> = None;
    let mut last: Option<Option<u32>> = None;
    let word = |i: usize| -> Option<String> {
        tokens.get(i).map(|n| {
            n.trim_matches(|c: char| !c.is_alphanumeric() && c != '%')
                .to_owned()
        })
    };
    let is = |i: usize, set: &[&str]| word(i).as_deref().is_some_and(|n| set.contains(&n));
    let unit = |i: usize| word(i).as_deref().is_some_and(counts);
    let strength = |i: usize| is(i, STRENGTHS) && !unit(i);
    let number = |i: usize| {
        tokens
            .get(i)
            .and_then(|t| quarters(t.trim_matches(|c: char| c == ',' || c == '.')))
    };
    let moment_at = |i: usize| tokens.get(i).and_then(|t| moment_of(t)).is_some();
    let hour = |t: &str| {
        let r = t.trim_start_matches(|c: char| c.is_ascii_digit());
        r.len() < t.len()
            && (r == "h" || (r.starts_with('h') && r[1..].chars().all(|c| c.is_ascii_digit())))
    };
    let mut i = 0;
    while i < tokens.len() {
        let t = tokens[i].trim_matches(|c: char| c == ',' || c == '.');
        // an hour or a goûter between takes has no column.
        let before = i >= 1 && moment_at(i - 1);
        let after = moment_at(i + 1);
        let attached = (after && !before)
            || (is(i + 1, &["le", "la", "au"]) && moment_at(i + 2))
            || (i >= 2 && is(i - 1, &["a", "vers"]) && moment_at(i - 2));
        if (hour(t) && !attached)
            || matches!(
                t,
                "gouter" | "apres-midi" | "d'apres-midi" | "l'apres-midi" | "apresmidi"
            )
        {
            lost = true;
        }
        // a dose carries only across links.
        if seen_moment
            && moment_of(t).is_none()
            && !matches!(
                word(i).as_deref().unwrap_or(""),
                "" | "et"
                    | "le"
                    | "la"
                    | "l"
                    | "les"
                    | "au"
                    | "aux"
                    | "a"
                    | "puis"
                    | "en"
                    | "tous"
                    | "toutes"
                    | "chaque"
            )
            && !hour(t)
        {
            last = None;
        }
        // **Un nombre n'est une quantité que si quelque chose le dit** :
        // une unité de comptage juste après, ou le moment qu'il
        // précède (« 2 le matin »). Tout autre nombre est autre chose —
        // « toutes les **12** heures », « **30** minutes avant »,
        // « **20** à 40 mg », « en **3** prises » —, et la grille du
        // Skenan imprimait « 12 » au matin et au soir d'une morphine.
        if let Some(q) = quarters(t) {
            // **Un nombre mixte** : « 1 1/2 cp le soir » est un et demi.
            // Lu nombre par nombre, le « 1 », sans unité ni moment juste
            // après, tombait, et il restait un demi — la boîte durait
            // trois fois plus que la vraie.
            let mut j = i;
            let mut q = q;
            // « 1, 5 cp »: a decimal cut by a typing space.
            if tokens[i].ends_with(',') && q % 4 == 0 && number(i + 1).is_some() && unit(i + 2) {
                pending = Some(None);
                last = Some(None);
                i += 3;
                continue;
            }
            if q % 4 == 0 {
                if let Some(frac) = tokens
                    .get(i + 1)
                    .and_then(|t| quarters(t.trim_matches(|c: char| c == ',' || c == '.')))
                    .filter(|f| *f > 0 && *f < 4 && tokens[i + 1].contains(['/', '½', '¼', '¾']))
                {
                    q += frac;
                    j = i + 1;
                }
            }
            // Une fourchette : « 1 à 2 comprimés », « 20 à 40 mg ». Ce
            // qui suit le second nombre décide des deux.
            //
            // **Written with a sign, it is never read as a figure**:
            // « 1 / 2 comprimé » is a half or a slip, « 1 - 2 cp » a range,
            // « 2 x 1 cp » a product. Number by number, the last one won.
            let sign = |k: usize| {
                tokens.get(k).is_some_and(|t| {
                    matches!(
                        t.trim_matches(|c: char| c == ',' || c == '.'),
                        "/" | "-" | "–" | "x" | "×" | "+" | "voire"
                    )
                })
            };
            if (is(j + 1, &["a", "ou"]) || sign(j + 1)) && number(j + 2).is_some() {
                let counted = unit(j + 3);
                if counted || sign(j + 1) {
                    pending = Some(None);
                    last = Some(None);
                }
                i = j + 3 + usize::from(counted);
                continue;
            }
            if strength(j + 1) {
                i = j + 2;
                continue;
            }
            // « 2 fois 1 cp le matin », « 1 cp 2 fois le matin ».
            if q > 0
                && is(j + 1, &["fois"])
                && !is(j + 2, &["par"])
                && !word(j + 2).is_some_and(|w| w.starts_with('j'))
            {
                pending = Some(None);
                last = Some(None);
                i = j + 2;
                while i < tokens.len() && (number(i).is_some() || unit(i)) {
                    i += 1;
                }
                continue;
            }
            if q > 0 && unit(j + 1) {
                // « 1 comprimé et demi », « 1 cp 1/2 », « 1 cp x 2 »,
                // « 1 cp + 1 cp »: the unit does not end the quantity.
                let fraction = tokens
                    .get(j + 2)
                    .is_some_and(|t| t.contains(['/', '½', '¼', '¾']) && number(j + 2).is_some())
                    && !is(j + 3, &["h", "heure", "heures", "min", "minutes"]);
                let half = is(j + 2, &["et"]) && is(j + 3, &["demi"]);
                let more = sign(j + 2) && number(j + 3).is_some();
                // « 1 cp x2 ».
                let times = word(j + 2).is_some_and(|w| {
                    w.strip_prefix('x')
                        .is_some_and(|n| n.parse::<u32>().is_ok())
                });
                if fraction || half || more || times {
                    pending = Some(None);
                    last = Some(None);
                    i = j + if fraction || times { 3 } else { 4 };
                    continue;
                }
                lost |= matches!(pending, Some(Some(_)));
                pending = Some(Some(q));
                last = Some(Some(q));
                // L'unité confirme, et n'est pas un moment : on la saute.
                i = j + 2;
                continue;
            }
            let article = is(j + 1, &["le", "la", "l", "au", "aux", "a"]);
            let at = if moment_at(j + 1) {
                Some(j + 1)
            } else if article && moment_at(j + 2) {
                Some(j + 2)
            } else {
                None
            };
            if let Some(at) = at {
                // « 0 midi » is nothing at that moment.
                if q == 0 {
                    last = None;
                    i = at + 1;
                    continue;
                }
                // « 3 soirs » counts evenings; « 1 cp et 1/2 le soir »,
                // « 1 cp de 40 le matin »: a bare number after a dose still
                // waiting is no replacement; after a name it is no dose.
                let plural = matches!(
                    word(at).as_deref(),
                    Some("soirs" | "matins" | "nuits" | "midis")
                );
                let opened = i == 0
                    || tokens[i - 1].ends_with(',')
                    || word(i - 1).is_some_and(|w| {
                        moment_of(&w).is_some()
                            || matches!(w.as_str(), "" | "et" | "puis" | "prendre" | "plus")
                    });
                if plural {
                    i = at;
                    continue;
                }
                lost |= matches!(pending, Some(Some(_)));
                pending = Some(opened.then_some(q));
                last = pending;
            }
            i = j + 1;
            continue;
        }
        // « 2cp x 2 »: glued, the first count is no number and the
        // second would stand alone.
        if t.starts_with(|c: char| c.is_ascii_digit())
            && matches!(tokens.get(i + 1), Some(&"x" | &"×"))
        {
            pending = Some(None);
            last = Some(None);
            i += 3;
            continue;
        }
        if let Some(m) = moment_of(t) {
            // « 1 cp le matin, pas le soir » — back to the clause's
            // start or the last number, whatever the verb.
            let denied = tokens[..i]
                .iter()
                .rev()
                .take(4)
                .take_while(|w| !w.ends_with(',') && !w.starts_with(|c: char| c.is_ascii_digit()))
                .any(|w| {
                    let w = w.trim_matches(|c: char| !c.is_alphanumeric());
                    matches!(
                        w,
                        "pas"
                            | "zero"
                            | "jamais"
                            | "aucun"
                            | "aucune"
                            | "sans"
                            | "rien"
                            | "ni"
                            | "eviter"
                            | "supprimer"
                            | "retirer"
                            | "enlever"
                            | "oter"
                            | "rincer"
                            | "sauf"
                    ) || (w == "laisser" && m == Moment::Coucher)
                        || (w == "toute" && m == Moment::Coucher)
                });
            if !denied {
                // a new quantity for a column already filled is a
                // phase, an alternation, or a sum.
                lost |= matches!(pending, Some(Some(_))) && doses[m.index()].is_some();
                let q = pending.take().or(last).flatten();
                doses[m.index()] = Some(Take { quarters: q });
                seen_moment = true;
            }
        }
        i += 1;
    }
    let first_quantity_before_moment = {
        let first_moment = tokens.iter().position(|t| moment_of(t).is_some());
        let first_number = tokens
            .iter()
            .position(|t| quarters(t.trim_matches(|c: char| c == ',' || c == '.')).is_some());
        matches!((first_number, first_moment), (Some(n), Some(m)) if n < m)
    };
    lost |= seen_moment && first_quantity_before_moment && matches!(pending, Some(Some(_)));
    // Les locutions en deux mots — « au lit » — que le passage par
    // jeton ne voit pas.
    if doses[3].is_none() && (folded.contains("au lit") || folded.contains("avant de dormir")) {
        doses[3] = Some(Take {
            quarters: last.flatten(),
        });
    }
    (doses, lost)
}

/// **The moment first, its dose after**: « matin : 1 comprimé, soir : 1/2
/// comprimé », « Matin 2, midi 1, soir 1 ». Read number-then-moment, each
/// dose slid onto the next moment, and the evening printed the morning's.
///
/// `None` unless a moment opens the phrase and a counted dose follows one.
/// Moments in a row share the dose after them (« matin et soir : 1 cp »).
fn moments_first(folded: &str, insulin: bool) -> Option<[Option<Take>; 4]> {
    let counts = |w: &str| UNITS.contains(&w) || (insulin && INSULIN.contains(&w));
    let tokens: Vec<&str> = folded.split_whitespace().collect();
    let clean = |t: &str| {
        t.trim_matches(|c: char| !c.is_alphanumeric() && c != '/' && c != '½')
            .to_owned()
    };
    let first_moment = tokens.iter().position(|t| moment_of(t).is_some())?;
    let opens = tokens[..first_moment]
        .iter()
        .all(|t| matches!(clean(t).as_str(), "" | "le" | "la" | "l" | "au" | "et"));
    if !opens {
        return None;
    }
    let denied = |i: usize| (1..=3).any(|back| i >= back && clean(tokens[i - back]) == "pas");
    let mut doses: [Option<Take>; 4] = [None; 4];
    // Every moment the phrase keeps, with no quantity: what is printed when
    // a quantity cannot be told. A blank is read; a wrong figure is taken.
    let mut marks = doses;
    for (i, t) in tokens.iter().enumerate().skip(first_moment) {
        if let Some(m) = moment_of(t).filter(|_| !denied(i)) {
            marks[m.index()] = Some(Take { quarters: None });
        }
    }
    // **Strict: figures only if every word is accounted for.** Reading what
    // it could, this printed 1-1-1 for « Matin 1, midi 0, soir 1 » and the
    // first dose for « 1 cp + 1 cp ». Whatever does not fit leaves `marks`.
    let previous = |i: usize| {
        tokens[..i]
            .iter()
            .rev()
            .map(|p| clean(p))
            .find(|p| !p.is_empty())
    };
    let part = |w: &str| {
        w == "demi"
            || w.contains(['½', '¼', '¾'])
            || (w.contains('/') && w.chars().any(|c| c.is_ascii_digit()))
    };
    let mut waiting: Vec<usize> = Vec::new();
    // A word between a moment and the next that is no dose: « matin un
    // demi comprimé, soir 1 comprimé ». The moment does not share the next
    // one's dose then.
    let mut stray = false;
    let mut bound = false;
    for (i, t) in tokens.iter().enumerate().skip(first_moment) {
        if let Some(m) = moment_of(t) {
            if denied(i) {
                if i == first_moment {
                    return None;
                }
                continue;
            }
            if stray && !waiting.is_empty() {
                return Some(marks);
            }
            stray = false;
            // the same moment bound twice.
            if doses[m.index()].is_some_and(|d| d.quarters.is_some()) {
                return Some(marks);
            }
            doses[m.index()] = Some(Take { quarters: None });
            waiting.push(m.index());
            continue;
        }
        // a dash in a moment's place is an empty cell, or a joiner:
        // either way no figure crosses it.
        if matches!(t.trim_end_matches(','), "-" | "–") && !waiting.is_empty() {
            return Some(marks);
        }
        let word = clean(t);
        let Some(q) = quarters(&word) else {
            stray |= !matches!(
                word.as_str(),
                "" | "le" | "la" | "l" | "au" | "aux" | "a" | "et"
            );
            continue;
        };
        let next = tokens.get(i + 1).map(|n| clean(n));
        let after = tokens.get(i + 2).map(|n| clean(n));
        let third = tokens.get(i + 3).map(|n| clean(n));
        let next = next.as_deref();
        let after = after.as_deref();
        // A strength, an hour, a delay, a duration: said so by what follows
        // it, or by what stands before it.
        let measure = next.is_some_and(|n| {
            (STRENGTHS.contains(&n) && !counts(n))
                || matches!(
                    n,
                    "h" | "heure" | "heures" | "min" | "minutes" | "jour" | "jours"
                )
        });
        let aside = previous(i).is_some_and(|p| matches!(p.as_str(), "a" | "vers" | "pendant"));
        // Past its dose, a phrase says nothing more that counts: another
        // quantity is a sum, a phase, a second moment.
        if waiting.is_empty() {
            if (measure || aside) && !next.is_some_and(counts) {
                continue;
            }
            return Some(marks);
        }
        // A range, a mixed number: the phrase says more than one figure.
        let range = matches!(next, Some("a" | "ou")) && after.and_then(quarters).is_some();
        // « 1 1/2 », « 1 et demi », and past the unit « 1 comprimé et demi ».
        // « / » alone separates two moments, « 1/2 heure avant » is a delay.
        let (rest, beyond) = if next.is_some_and(counts) {
            (after, third.as_deref())
        } else {
            (next, after)
        };
        let delay = matches!(beyond, Some("h" | "heure" | "heures" | "min" | "minutes"));
        let mixed =
            (rest.is_some_and(part) && !delay) || (rest == Some("et") && beyond.is_some_and(part));
        if range || mixed {
            return Some(marks);
        }
        // What says this number is a dose: its unit, the moment it runs
        // into, or the end of the phrase right after its own moment.
        let beside = previous(i).is_some_and(|p| moment_of(&p).is_some());
        let closes = match next {
            None => beside,
            Some(n) => {
                counts(n)
                    || moment_of(n).is_some()
                    || (matches!(n, "le" | "la" | "l" | "au" | "et" | "puis")
                        && after.is_some_and(|a| moment_of(a).is_some()))
            }
        };
        if !closes {
            // « le soir 20 mg 1 comprimé »: the dose is still to come,
            // with its unit, before the next moment.
            let dose_follows = tokens[i + 1..]
                .iter()
                .map(|t| clean(t))
                .take_while(|t| moment_of(t).is_none())
                .collect::<Vec<_>>()
                .windows(2)
                .any(|w| quarters(&w[0]).is_some_and(|q| q > 0) && counts(&w[1]));
            if (measure || aside) && dose_follows {
                continue;
            }
            return Some(marks);
        }
        // « midi 0 »: nothing at that moment.
        for m in waiting.drain(..) {
            doses[m] = (q > 0).then_some(Take { quarters: Some(q) });
        }
        stray = false;
        bound = true;
    }
    bound.then_some(doses)
}

/// Le moment que ce mot désigne, s'il en désigne un.
fn moment_of(token: &str) -> Option<Moment> {
    let t = token.trim_matches(|c: char| !c.is_alphanumeric());
    for (words, m) in [
        (MATIN, Moment::Matin),
        (MIDI, Moment::Midi),
        (SOIR, Moment::Soir),
        (COUCHER, Moment::Coucher),
    ] {
        if words.iter().any(|w| t == *w || t.starts_with(w)) {
            return Some(m);
        }
    }
    None
}

/// a count the table of needles does not hold. `Some(None)` is a count
/// said as a choice (« 1 à 2 fois par jour »): no figure.
fn told_count(head: &str) -> Option<Option<u32>> {
    let (mut seen, mut rest) = (None, head.to_owned());
    while let Some((n, end)) = told_once(&rest) {
        match (seen, n) {
            (None, n) => seen = Some(n),
            (Some(a), b) if a == b => {}
            _ => return Some(None),
        }
        rest = rest
            .split_whitespace()
            .skip(end + 1)
            .collect::<Vec<_>>()
            .join(" ");
    }
    seen
}

fn told_once(head: &str) -> Option<(Option<u32>, usize)> {
    const WORDS: [&str; 12] = [
        "zero", "une", "deux", "trois", "quatre", "cinq", "six", "sept", "huit", "neuf", "dix",
        "onze",
    ];
    let t: Vec<String> = head
        .split_whitespace()
        .map(|w| {
            w.trim_matches(|c: char| matches!(c, ',' | '.' | ';'))
                .to_owned()
        })
        .collect();
    let num = |w: &str| -> Option<u32> {
        w.parse()
            .ok()
            .or_else(|| WORDS.iter().position(|x| *x == w).map(|i| i as u32))
    };
    let daily = |i: usize| {
        let at = |k: usize| t.get(k).map_or("", |s| s.as_str());
        (at(i) == "par" && at(i + 1) == "jour") || at(i).starts_with("/j")
    };
    for (i, w) in t.iter().enumerate() {
        let at = |k: usize| t.get(k).map_or("", |s| s.as_str());
        // « en 3 prises », « 3 prises par jour ».
        let kinds = |s: &str| {
            s.starts_with("prise")
                || s.starts_with("injection")
                || s.starts_with("application")
                || s.starts_with("instillation")
        };
        if let Some(n) = num(w) {
            let choice = i >= 2 && matches!(at(i - 1), "a" | "ou") && num(at(i - 2)).is_some();
            let fois = at(i + 1) == "fois" && (daily(i + 2) || at(i + 2).starts_with("/j"));
            if fois || at(i + 1).starts_with("fois/j") {
                return Some(((!choice).then_some(n), i));
            }
            if kinds(at(i + 1)) && (daily(i + 2) || (i > 0 && at(i - 1) == "en")) {
                return Some(((!choice).then_some(n), i));
            }
            // « 3 x par jour », « 3x par jour », « 3 x 1 cp/j », « 3 fois 2 cp par jour ».
            if matches!(at(i + 1), "x" | "×" | "fois")
                && (daily(i + 2)
                    || daily(i + 3)
                    || daily(i + 4)
                    || at(i + 3).ends_with("/j")
                    || at(i + 4).contains("/j"))
            {
                return Some((Some(n), i));
            }
        }
        if let Some(n) = w
            .strip_suffix("x/j")
            .or_else(|| w.strip_suffix("x/jour"))
            .and_then(num)
        {
            let choice = i >= 2 && matches!(at(i - 1), "a" | "ou") && num(at(i - 2)).is_some();
            return Some(((!choice).then_some(n), i));
        }
        if let Some(n) = w.strip_suffix('x').and_then(num) {
            if daily(i + 1) {
                return Some((Some(n), i));
            }
        }
        // « 3x1 cp/j ».
        if let Some((n, _)) = w.split_once('x') {
            if let Some(n) = num(n) {
                if !w.ends_with('x')
                    && (at(i + 1).ends_with("/j")
                        || at(i + 2).ends_with("/j")
                        || daily(i + 2)
                        || daily(i + 1))
                {
                    return Some((Some(n), i));
                }
            }
        }
        // « x 3 par jour », « x3 par jour ».
        if *w == "x" && num(at(i + 1)).is_some() && daily(i + 2) {
            return Some((num(at(i + 1)), i));
        }
        if let Some(n) = w.strip_prefix('x').and_then(num) {
            if daily(i + 1) {
                return Some((Some(n), i));
            }
        }
        // « 3/j », « 3/jour », and « 2-3 fois par jour ».
        if let Some(n) = w
            .strip_suffix("/j")
            .or_else(|| w.strip_suffix("/jour"))
            .and_then(num)
        {
            return Some((Some(n), i));
        }
        if at(i + 1) == "fois" && daily(i + 2) && w.contains('-') {
            return Some((None, i));
        }
    }
    None
}

/// Combien de prises par jour, quand la phrase le dit sans dire quand.
fn per_day(folded: &str) -> Option<u32> {
    // **Un plafond ne dit pas un rythme.** « sans dépasser 300 mg par
    // jour » borne ce qu'on prend, il ne dit pas qu'on le prend une fois
    // par jour — et la grille lisait « 1 prise par jour » pour un
    // triptan. Ce qui suit le premier plafond ne compte pas.
    let head = [
        "sans depasser",
        "maximum",
        "dans la limite",
        "ne pas depasser",
        "au plus",
        " max ",
        "max ",
        "maxi",
        "pas plus de",
        "ne pas exceder",
        "jusqu'a 2 ",
        "jusqu'a 3 ",
        "jusqu'a 4 ",
        "jusqu'a 5 ",
        "jusqu'a 6 ",
        "jusqu'a 8 ",
    ]
    .iter()
    .filter_map(|w| folded.find(w))
    .min()
    .map_or(folded, |at| &folded[..at]);
    // Une dose **exprimée** par jour n'est pas un rythme non plus :
    // « un dixième de la dose quotidienne » est une interdose.
    let head = head
        .replace("dose quotidienne", "")
        .replace("dose journaliere", "")
        .replace("morphinique quotidien", "");
    // Et un intervalle dit un nombre de prises que la phrase ne donne
    // pas : « toutes les 6 heures », « au bout de 4 heures » — une prise
    // par jour serait faux, et rien d'autre n'est écrit.
    //
    // Seulement un intervalle **en heures** : « doublés toutes les
    // 2 semaines » est le pas d'une titration, et le traitement se
    // prend bien chaque jour. Et seulement quand la phrase ne dit pas
    // elle-même combien de fois : « 200 mg cinq fois par jour, toutes
    // les 4 heures » le dit.
    let hourly = |w: &str| {
        head.match_indices(w)
            .find(|(at, _)| {
                head[*at..]
                    .chars()
                    .take(w.chars().count() + 24)
                    .collect::<String>()
                    .contains("heure")
            })
            .map(|(at, _)| at)
    };
    // Le premier des deux dit le rythme : une fiche entière écrit
    // « 30 mg toutes les 12 heures » et, trois phrases plus loin, le
    // « quatre fois par jour » d'une autre forme.
    let interval = [hourly("toutes les"), hourly("au bout de")]
        .into_iter()
        .flatten()
        .min();
    // « toutes les 6h », hours glued; two hours named, a count not said.
    let glued_interval = head
        .split_whitespace()
        .collect::<Vec<_>>()
        .windows(3)
        .any(|w| {
            let n = w[2].trim_end_matches(',');
            w[0] == "toutes" && w[1] == "les" && n.ends_with('h') && n.len() > 1
        });
    let hours = head
        .split_whitespace()
        .filter(|w| {
            let r = w.trim_start_matches(|c: char| c.is_ascii_digit());
            r.len() < w.len() && r.trim_end_matches(',') == "h"
        })
        .count();
    if glued_interval || hours > 1 {
        return None;
    }
    let counted = head.find("fois par jour");
    if let Some(at) = interval {
        if counted.is_none_or(|c| at < c) {
            return None;
        }
    }
    // A ceiling is no rhythm (« 10 prises par jour au maximum »), and a
    // titration is not at its count yet.
    if folded.contains("par jour au maximum") || folded.contains("par jour maximum") {
        return None;
    }
    if (head.contains("par jour") || head.contains("/j")) && !head.contains("augment") {
        if let Some(n) = told_count(&head) {
            return n;
        }
    }
    // « en 2 ou 3 prises », « en 2 a 3 prises » : le prescripteur a
    // laissé le choix, et « 1 » n'est aucun des deux.
    let words: Vec<&str> = head.split_whitespace().collect();
    if words.windows(5).any(|w| {
        w[0] == "en"
            && quarters(w[1]).is_some()
            && (w[2] == "ou" || w[2] == "a")
            && quarters(w[3]).is_some()
            && w[4].starts_with("prise")
    }) {
        return None;
    }
    // **Les abréviations** : « 2x/j », « 2 x/j », « 2 fois/j »,
    // « 1 cp x 2/j ». Lues seulement par « par jour », elles faisaient
    // une prise par jour, imprimée sur la fiche. « 2 x 3/j », qui peut
    // dire l'un ou l'autre, n'est pas tranché ici.
    let per = |w: &str| {
        matches!(
            w,
            "/j" | "/jour" | "x/j" | "x/jour" | "fois/j" | "fois/jour"
        )
    };
    let count = |w: &str| w.parse::<u32>().ok().filter(|n| (1..=6).contains(n));
    for (i, w) in words.iter().enumerate() {
        let next = words.get(i + 1).copied().unwrap_or("");
        let found = if let Some(n) = w.strip_suffix("x/j").or_else(|| w.strip_suffix("x/jour")) {
            count(n)
        } else if let Some(n) = w.strip_prefix('x').and_then(|r| r.strip_suffix("/j")) {
            count(n)
        } else if *w == "x" {
            next.strip_suffix("/j")
                .or_else(|| next.strip_suffix("/jour"))
                .and_then(count)
        } else if per(next) && next != "/j" && next != "/jour" {
            count(w)
        } else {
            None
        };
        if found.is_some() {
            return found;
        }
    }
    let digits = |w: &str| !w.is_empty() && w.chars().all(|c| c.is_ascii_digit());
    let counted_a_day = |i: usize| {
        let w = words.get(i).copied().unwrap_or("");
        let glued = w
            .split_once('/')
            .is_some_and(|(u, r)| UNITS.contains(&u) && r.starts_with('j'));
        glued
            || (UNITS.contains(&w)
                && words.get(i + 1) == Some(&"par")
                && words.get(i + 2) == Some(&"jour"))
    };
    let range = (0..words.len()).any(|i| {
        (digits(words[i])
            && matches!(words.get(i + 1).copied(), Some("a" | "ou"))
            && words.get(i + 2).is_some_and(|w| digits(w))
            && counted_a_day(i + 3))
            || (words[i]
                .split_once('-')
                .is_some_and(|(a, b)| digits(a) && digits(b))
                && counted_a_day(i + 1))
    });
    if range {
        return None;
    }
    let folded = head.as_str();
    for (needle, times) in [
        ("6 fois par jour", 6),
        ("six fois par jour", 6),
        ("5 fois par jour", 5),
        ("cinq fois par jour", 5),
        ("4 fois par jour", 4),
        ("quatre fois par jour", 4),
        ("3 fois par jour", 3),
        ("trois fois par jour", 3),
        ("2 fois par jour", 2),
        ("deux fois par jour", 2),
        ("1 fois par jour", 1),
        ("une fois par jour", 1),
        ("par jour", 1),
        ("/jour", 1),
        ("/j", 1),
        ("quotidien", 1),
        ("chaque jour", 1),
        ("tous les jours", 1),
    ] {
        if folded.contains(needle) {
            return Some(times);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dose_written_after_its_moment_stays_on_it() {
        let grid = |text: &str| read(text).doses.map(|d| d.and_then(|t| t.quarters));
        assert_eq!(
            grid("le matin 1 comprimé et le soir 2 comprimés"),
            [Some(4), None, Some(8), None]
        );
        assert_eq!(
            grid("matin: 1 comprimé, soir: 1/2 comprimé"),
            [Some(4), None, Some(2), None]
        );
        assert_eq!(
            grid("Matin 2, midi 1, soir 1"),
            [Some(8), Some(4), Some(4), None]
        );
        assert_eq!(
            grid("matin et soir : 1 comprimé"),
            [Some(4), None, Some(4), None]
        );
        // The dose before its moment reads as it always did.
        assert_eq!(
            grid("1 comprimé le matin et 2 le soir"),
            [Some(4), None, Some(8), None]
        );
        assert_eq!(grid("matin 1, le soir 2"), [Some(4), None, Some(8), None]);
        assert_eq!(
            grid("matin : 1, le soir : 1/2"),
            [Some(4), None, Some(2), None]
        );
        assert_eq!(
            grid("matin 2, au coucher 1"),
            [Some(8), None, None, Some(4)]
        );
        assert_eq!(
            grid("matin et soir 2 pulvérisations pendant 10 jours"),
            [Some(8), None, Some(8), None]
        );
        assert_eq!(
            grid("le soir 20 mg 1 comprimé"),
            [None, None, Some(4), None]
        );
        assert_eq!(
            grid("le matin à 8 h 1 comprimé"),
            [Some(4), None, None, None]
        );
        assert_eq!(
            grid("matin et soir pendant 10 jours 2 pulvérisations"),
            [Some(8), None, Some(8), None]
        );
        assert_eq!(
            grid("matin 1 cp / soir 1 cp"),
            [Some(4), None, Some(4), None]
        );
        assert_eq!(
            grid("le matin 1 cp 1/2 heure avant le repas"),
            [Some(4), None, None, None]
        );
        // A dose carries across links only.
        for (phrase, expected) in [
            (
                "1 cp matin, 0 midi, 1 cp soir",
                [Some(4), None, Some(4), None],
            ),
            ("1 matin 0 soir", [Some(4), None, None, None]),
            ("2 matin, 1 midi, 0 soir", [Some(8), Some(4), None, None]),
            ("1 cp le matin, rien le soir", [Some(4), None, None, None]),
            (
                "1 cp le matin, pas de prise le soir",
                [Some(4), None, None, None],
            ),
            ("1 cp le matin, éviter le soir", [Some(4), None, None, None]),
            (
                "1 patch le matin, à retirer le soir",
                [Some(4), None, None, None],
            ),
            ("1 cp matin midi et soir", [Some(4), Some(4), Some(4), None]),
            (
                "1 comprimé matin, midi et soir",
                [Some(4), Some(4), Some(4), None],
            ),
        ] {
            assert_eq!(grid(phrase), expected, "{phrase}");
        }
        // What is no daily take does not reach the grid: the methotrexate rule.
        for phrase in [
            "3 cp les lundis matin",
            "3 cp le matin 1 fois/sem",
            "1 cp par sem le matin",
            "1 cp hebdo le matin",
            "1 cp le matin lun mer ven",
            "1 cp le soir 1j/2",
            "1 cp le soir 1 j sur 2",
            "1 cp le matin 5j/7",
            "1 cp le soir 21j/28",
            "1 cp le soir un jour oui un jour non",
            "1 cp le matin tous les 2j",
            "1 patch le matin toutes les 72h",
            "1 cp 5 soirs sur 7",
            "1 cp le matin 1 fois/mois",
            "1 cp le soir le week-end",
            "1 cp le matin pendant 21 jours, arrêt 7 jours",
            "1 cp le matin 2 fois par an",
            "1 cp le matin à J1 et J8",
            "1 cp le soir si insomnie",
            "1 cp matin et soir si nausées",
            "1 sachet le matin si constipation",
            "1 cp le soir en cas de douleur",
            "8 unités le soir si glycémie > 2 g/l",
        ] {
            assert_ne!(read(phrase).kind, Kind::Placed, "{phrase}");
        }
        // Two counts are no count, a ceiling no rhythm.
        for phrase in [
            "1 à 2 fois par jour",
            "2 à 3 fois par jour",
            "10 prises par jour au maximum",
            "1 cp, pas plus de 3 par jour",
        ] {
            assert_ne!(read(phrase).kind, Kind::Daily, "{phrase}");
        }
        assert_eq!(read("1,5 g par jour en 3 prises").times, 3);
        assert_eq!(read("1 cp 3/j").times, 3);
        // Cycle days, a course tied to a treatment, an unnamed condition,
        // meals beside a moment: no grid says these.
        for phrase in [
            "Xeloda 1500 mg matin et soir J1-J14 reprise J22",
            "Solupred 20 mg 3 cp le matin J1 à J3 de chaque cure",
            "Utrogestan 100 mg 1 caps au coucher du J1 au J25",
            "Zophren 8 mg 1 cp matin et soir pendant 3 jours après la chimio",
            "1 cp matin et soir si pas de selles depuis 3 jours",
            "1 cp matin et soir en cas de diarrhée",
            "Furosémide 40 mg : 1 cp le matin, +1 cp à midi si prise de poids > 2 kg",
            "Gaviscon 1 sachet après chaque repas et au coucher",
        ] {
            assert_ne!(read(phrase).kind, Kind::Placed, "{phrase}");
        }
        assert_eq!(
            read("Doliprane 1 g jusqu'à 3/j si douleurs").kind,
            Kind::OnDemand
        );
        for phrase in [
            "Macrogol 1 à 2 sachets/j",
            "1 à 3 cp/j",
            "2-3 cp/j",
            "1-2 cp/j",
        ] {
            assert_ne!(read(phrase).kind, Kind::Daily, "{phrase}");
        }
        // A phase is not one day's grid.
        assert_ne!(
            read("2 cp le matin pendant 7 jours, ensuite 1 cp le matin").kind,
            Kind::Placed
        );
        // Zero is a figure: nothing at that moment.
        assert_eq!(
            grid("Matin 1, midi 0, soir 1"),
            [Some(4), None, Some(4), None]
        );
        assert_eq!(grid("matin 0, midi 0, soir 1"), [None, None, Some(4), None]);
        assert_eq!(
            grid("matin : 1 cp, midi : 0, soir : 1 cp"),
            [Some(4), None, Some(4), None]
        );
        // Not a dose, or not one figure: the box stays a mark.
        for unsure in [
            "le matin un demi comprimé, le soir 1 comprimé",
            "le matin un comprimé et le soir 2 comprimés",
            "matin : rien, soir : 1 cp",
            "matin 45 midi 30 soir 20",
            "le matin 1 cp + 1 cp",
            "matin 1 cp + 1/2 cp, soir 1 cp",
            "le soir 1 cp ou 2 cp",
            "le soir 1 cp à 2 cp",
            "le matin 1 cp x 2",
            "le soir 1 cp en alternance avec 1/2 cp",
            "matin 1 cp, 2 cp à midi, 1 cp le soir",
            "matin 1 / 2 cp, soir 1 cp",
            "le matin 1 x 2 cp, le soir 1 cp",
            "matin 1 - 2 cp, soir 1 cp",
            "matin 1 voire 2 cp, soir 1 cp",
            "1 / 2 comprimé le soir",
            "1 - 2 comprimés le soir",
            "1 voire 2 cp le soir",
            "1 comprimé et demi le soir",
            "1 cp 1/2 le soir",
            "1 + 1/2 cp le soir",
            "1 cp x 3 matin midi soir",
            "1 cp x 2 matin et soir",
        ] {
            assert!(
                grid(unsure).iter().all(Option::is_none),
                "{unsure}: {:?}",
                grid(unsure)
            );
        }
        for blank in [
            "matin et soir pendant 7 jours",
            "le soir pendant 7",
            "le matin à 8",
            "le soir vers 20",
            "le soir 1 à 2 comprimés",
            "matin 1 ou 2 cp",
            "matin et soir 1/2 à 1 cp",
            "le soir 1 1/2 comprimé",
            "matin 1 1/2 cp, soir 1 cp",
            "le soir 1 comprimé et demi",
            "le soir 1 et demi",
            "le soir 1 cp 1/2",
            "le soir 1 comprimé et 1/2",
            "le soir 1 / 2 comprimé",
            "le soir 1 - 2 comprimés",
            "le soir 1 voire 2 cp",
            "le matin 2 x 1 cp",
            "matin 5 mg, soir 10",
        ] {
            assert!(
                grid(blank).iter().all(Option::is_none),
                "{blank}: {:?}",
                grid(blank)
            );
        }
        // Denied after the dose.
        let denied_late = read("le matin 1 comprimé, pas le soir");
        assert!(denied_late.doses[2].is_none());
        let denied = read("ne pas prendre le matin, 1 le soir");
        assert!(denied.doses[0].is_none());
    }

    fn q(r: &Reading, m: Moment) -> Option<Option<u32>> {
        r.doses[m.index()].map(|t| t.quarters)
    }

    /// **Le silence n'est pas une case vide.**
    ///
    /// Une posologie illisible n'a pas de grille du tout : quatre cases
    /// blanches se lisent « rien à prendre », ce qui est exactement le
    /// contraire de « je n'ai pas su lire ».
    /// **La règle qui tue, écrite en chiffres** : « toutes les 2 semaines »
    /// n'était pas dans la liste, et une injection bimensuelle se cochait
    /// chaque jour. Un pas de titration, lui, reste quotidien.
    #[test]
    fn an_interval_longer_than_a_day_is_never_a_daily_grid() {
        for p in [
            "1 cp le matin toutes les 2 semaines",
            "1 cp le matin toutes les 3 semaines",
            "1 cp le matin tous les 10 jours",
            "1 cp le matin tous les 14 jours",
            "1 cp le matin tous les mois",
            "1 cp le matin tous les 3 mois",
            "1 cp le matin, 1 jour sur 3",
            "1 cp le soir, un soir sur deux",
            "1 patch le matin toutes les 72 heures",
        ] {
            assert_eq!(read(p).kind, Kind::Cyclic, "{p}");
        }
        for p in [
            "1 cp le matin, augmentation par paliers toutes les 2 semaines",
            "1 cp le matin, réévalué tous les 6 mois",
            "1 cp le matin pendant 3 semaines",
        ] {
            assert_eq!(read(p).kind, Kind::Placed, "{p}");
        }
    }

    /// « Si besoin » collé au rythme suspend le rythme ; une dose en plus
    /// entre les deux, ou un second rythme, disent une titration.
    #[test]
    fn a_condition_right_after_a_rhythm_is_on_demand() {
        for p in [
            "1 cp 3 fois par jour si besoin",
            "1 sachet 2 fois par jour si douleur",
            "1 cp par jour le soir si douleur",
        ] {
            assert_eq!(read(p).kind, Kind::OnDemand, "{p}");
        }
        for (p, kind) in [
            (
                "5 mg une fois par jour, portés si besoin à 10 mg",
                Kind::Daily,
            ),
            ("5 mg par jour, 10 mg si nécessaire", Kind::Daily),
            (
                "10 mg par jour, puis 20 mg par jour ; 40 mg par jour si nécessaire",
                Kind::Cyclic,
            ),
            (
                "1 comprimé trois fois par jour au moment des repas",
                Kind::Daily,
            ),
            (
                "1 g trois fois par jour à heure fixe plutôt qu'à la demande",
                Kind::Daily,
            ),
        ] {
            assert_eq!(read(p).kind, kind, "{p}");
        }
    }

    /// Un choix de moment est une prise ; un moment nié n'en est pas une.
    #[test]
    fn a_choice_of_moment_is_one_taking() {
        for p in [
            "50 mg par jour en une prise, matin ou soir",
            "Une prise par jour, le matin ou le soir",
            "1 comprimé matin ou soir",
        ] {
            let r = read(p);
            assert_eq!((r.kind, r.takes_a_day()), (Kind::Daily, 1), "{p}");
        }
        let r = read("1 cp le matin, pas le soir");
        assert_eq!(r.takes_a_day(), 1);
        assert!(q(&r, Moment::Soir).is_none());
    }

    /// Une quantité qui ne trouve pas son moment rend la phrase illisible,
    /// plutôt qu'une dose sur trois.
    #[test]
    fn a_counted_dose_without_a_known_moment_is_not_dropped() {
        for p in [
            "1 cp à 8h, 1 cp à 16h, 1 le soir",
            "1 cp le matin et 1 cp en début d'après-midi",
        ] {
            assert_eq!(read(p).kind, Kind::Unread, "{p}");
        }
        // Le moment d'abord : rien ne se perd.
        let r = read("matin : 1 cp, soir : 1 cp");
        assert_eq!(r.kind, Kind::Placed);
    }

    #[test]
    fn a_posology_that_cannot_be_read_draws_no_grid() {
        for text in [
            "selon le schéma du service",
            "voir protocole",
            "",
            "adapter à l'INR",
        ] {
            let r = read(text);
            assert_eq!(r.kind, Kind::Unread, "« {text} » devrait rester illisible");
            assert!(!r.has_grid(), "« {text} » ne doit pas dessiner de grille");
        }
    }

    /// **« Si besoin » n'entre jamais dans la grille.**
    ///
    /// La règle est écrite dans les tables de cette application, à
    /// propos du pilulier : y déposer un antalgique à la demande le
    /// transforme en prise systématique. Et la condition part avec la
    /// ligne — un « si besoin » sans son « quand » est une autorisation
    /// permanente.
    #[test]
    fn a_treatment_on_demand_never_enters_the_grid() {
        let r = read("1 comprimé si besoin, en cas de douleur, maximum 3 par jour");
        assert_eq!(r.kind, Kind::OnDemand);
        assert!(!r.has_grid());
        assert!(r.doses.iter().all(Option::is_none));
        assert!(
            r.note.contains("douleur"),
            "la condition doit être reprise : {}",
            r.note
        );

        // Et la condition l'emporte même quand un moment est nommé :
        // « 1 le soir si besoin » n'est pas une prise du soir.
        let r = read("1 comprimé le soir si besoin");
        assert_eq!(r.kind, Kind::OnDemand);
        assert!(q(&r, Moment::Soir).is_none());
    }

    /// **Une prise hebdomadaire n'est pas une prise quotidienne.**
    ///
    /// C'est la règle qui tue. Le méthotrexate est hebdomadaire, et sa
    /// prise quotidienne est mortelle ; une croix dans la colonne
    /// « matin » d'une grille dont l'en-tête est une journée se lit
    /// « tous les matins ».
    #[test]
    fn a_weekly_dose_is_never_drawn_as_a_daily_one() {
        for text in [
            "1 comprimé par semaine, le lundi matin",
            "7,5 mg une fois par semaine le lundi",
            "1 comprimé tous les lundis",
            "1 comprimé un jour sur deux",
        ] {
            let r = read(text);
            assert_eq!(r.kind, Kind::Cyclic, "« {text} »");
            assert!(!r.has_grid(), "« {text} » ne doit rien cocher");
            assert!(r.doses.iter().all(Option::is_none), "« {text} »");
            assert!(r.note.contains(text.split(',').next().unwrap_or(text)));
        }

        // Et la confrontation aux fiches livrées, puisque c'est sur
        // elles que la faute se paierait : pas une seule posologie
        // hebdomadaire de la base ne doit se retrouver dans la grille
        // du jour. Le méthotrexate en écrit quatre, dont une qui dit
        // en toutes lettres pourquoi — « l'erreur de prise quotidienne
        // est la cause d'accidents mortels ».
        let mut weekly = 0;
        for (brand, _indication, posologie, _remarque) in crate::db::STARTER_POSOLOGIES {
            let folded = crate::fuzzy::sort_key(posologie);
            if !folded.contains("par semaine") && !folded.contains("hebdomadaire") {
                continue;
            }
            weekly += 1;
            assert_ne!(
                read(posologie).kind,
                Kind::Placed,
                "{brand} : « {posologie} » entrerait dans la grille du jour"
            );
        }
        assert!(weekly > 20, "seulement {weekly} posologies hebdomadaires");

        // Et une **durée** n'est pas un rythme : « 7 jours » derrière
        // une posologie quotidienne ne doit pas la faire sortir de la
        // grille. C'est la posologie du Zeclar du dossier de
        // démonstration.
        let r = read("500 mg matin et soir, 7 jours");
        assert_eq!(r.kind, Kind::Placed);
        assert!(r.has_grid());
    }

    /// **Une prise dont le moment n'est pas dit ne se place pas.**
    ///
    /// « 1 comprimé par jour » est la posologie la plus fréquente qui
    /// soit, et elle ne dit pas le matin. Placer d'office au matin
    /// serait écrire une consigne que le prescripteur n'a pas donnée —
    /// sur la feuille qui part à la maison, où personne ne viendra la
    /// contredire.
    #[test]
    fn a_dose_with_no_moment_is_left_to_be_agreed() {
        let r = read("1 comprimé par jour");
        assert_eq!(r.kind, Kind::Daily);
        assert_eq!(r.times, 1);
        assert!(!r.has_grid());
        assert!(r.doses.iter().all(Option::is_none));

        let r = read("1 gélule 3 fois par jour");
        assert_eq!(r.kind, Kind::Daily);
        assert_eq!(r.times, 3);
        assert!(!r.has_grid());
    }

    /// **Un milligramme n'est pas un comprimé.**
    ///
    /// « 5 mg matin et soir » porte deux prises, pas cinq. C'est la
    /// forme que prennent toutes les posologies du dossier de
    /// démonstration, et la faute aurait imprimé « 5 » dans une case de
    /// pilulier.
    #[test]
    fn a_milligram_is_not_a_tablet() {
        let r = read("5 mg matin et soir");
        assert_eq!(r.kind, Kind::Placed);
        assert_eq!(q(&r, Moment::Matin), Some(None));
        assert_eq!(q(&r, Moment::Soir), Some(None));
        assert_eq!(q(&r, Moment::Midi), None);

        let r = read("1000 mg matin et soir");
        assert_eq!(q(&r, Moment::Matin), Some(None));
        assert_eq!(q(&r, Moment::Soir), Some(None));
    }

    /// **Une prise sans quantité reste une prise.**
    ///
    /// La case porte une marque, jamais un « 1 » : `label()` rend
    /// `None`, et c'est au dessin de poser son signe.
    #[test]
    fn a_taking_with_no_quantity_is_still_a_taking() {
        let r = read("20 mg le soir");
        assert!(r.has_grid());
        let take = r.doses[Moment::Soir.index()].expect("une prise le soir");
        assert_eq!(take.quarters, None);
        assert_eq!(take.label(), None);
    }

    /// Les quarts s'écrivent comme on les dit au comptoir, et
    /// « avant de dormir » est le coucher.
    ///
    /// Le quart n'est pas une coquetterie : l'hémigoxine n'existe qu'en
    /// 0,125 mg quadrisécable, et c'est le quart que le patient a dans
    /// sa boîte à pilules. Une locution en deux mots ne se voit pas
    /// jeton par jeton, d'où le rattrapage en fin de lecture.
    #[test]
    fn a_quarter_is_written_as_it_is_said() {
        for (quarters, written) in [(1, "¼"), (2, "½"), (3, "¾"), (4, "1"), (7, "1 ¾")] {
            assert_eq!(
                Take {
                    quarters: Some(quarters)
                }
                .label()
                .as_deref(),
                Some(written)
            );
        }
        assert_eq!(quarters("¼"), Some(1));
        assert_eq!(quarters("¾"), Some(3));
        // Ni une lettre, ni un nombre de comprimés qu'aucune boîte ne
        // porte : une posologie qu'on ne sait pas lire se dit, elle ne
        // se devine pas.
        assert_eq!(quarters("deux"), None);
        assert_eq!(quarters("400"), None);

        let r = read("1 comprimé avant de dormir");
        assert_eq!(q(&r, Moment::Coucher), Some(Some(4)));
        let r = read("1 comprimé au lit");
        assert_eq!(q(&r, Moment::Coucher), Some(Some(4)));
    }

    /// Les tournures qui disent « par jour » sans dire quand se lisent
    /// toutes, et elles rendent le nombre de prises.
    #[test]
    fn every_way_of_saying_a_day_is_read() {
        for (text, times) in [
            ("1 comprimé 4 fois par jour", 4),
            ("1 comprimé trois fois par jour", 3),
            ("1 comprimé deux fois par jour", 2),
            ("1 comprimé une fois par jour", 1),
            ("1 comprimé/jour", 1),
            ("1 comprimé quotidiennement", 1),
            ("1 comprimé chaque jour", 1),
            ("1 comprimé tous les jours", 1),
        ] {
            let r = read(text);
            assert_eq!(r.kind, Kind::Daily, "« {text} »");
            assert_eq!(r.times, times, "« {text} »");
            assert_eq!(r.takes_a_day(), times, "« {text} »");
        }
        // Et une prise placée compte ses moments, tandis qu'une prise
        // hors grille ne compte rien : « à la demande » n'a pas de
        // nombre de prises par jour, et en inventer un est exactement
        // ce que le pilulier fait de travers.
        assert_eq!(read("1 comprimé matin et soir").takes_a_day(), 2);
        assert_eq!(read("1 comprimé si besoin").takes_a_day(), 0);
    }

    /// Une quantité énoncée une fois gouverne les moments qu'elle
    /// introduit, et une quantité propre reprend la main.
    #[test]
    fn a_quantity_said_once_governs_the_moments_it_introduces() {
        let r = read("1 comprimé matin, midi et soir");
        for m in [Moment::Matin, Moment::Midi, Moment::Soir] {
            assert_eq!(q(&r, m), Some(Some(4)), "{}", m.label());
        }
        assert_eq!(q(&r, Moment::Coucher), None);

        let r = read("2 comprimés le matin et 1 le soir");
        assert_eq!(q(&r, Moment::Matin), Some(Some(8)));
        assert_eq!(q(&r, Moment::Soir), Some(Some(4)));
    }

    /// Un demi-comprimé n'est pas un comprimé, et « 1/2 » n'est pas une
    /// notation à trois nombres.
    #[test]
    fn a_half_is_a_half() {
        assert_eq!(quarters("½"), Some(2));
        assert_eq!(quarters("0,5"), Some(2));
        assert_eq!(quarters("1/2"), Some(2));
        let r = read("½ comprimé le matin");
        assert_eq!(q(&r, Moment::Matin), Some(Some(2)));
        assert_eq!(Take { quarters: Some(2) }.label().as_deref(), Some("½"));
        assert_eq!(Take { quarters: Some(6) }.label().as_deref(), Some("1 ½"));
    }

    /// La notation chiffrée se lit, et ne se confond pas avec une
    /// fraction.
    #[test]
    fn the_numeric_notation_reads_three_or_four_figures() {
        let r = read("1-0-1");
        assert_eq!(q(&r, Moment::Matin), Some(Some(4)));
        assert_eq!(q(&r, Moment::Midi), None);
        assert_eq!(q(&r, Moment::Soir), Some(Some(4)));

        let r = read("1-1-1-1");
        assert!(Moment::ALL.iter().all(|m| q(&r, *m).is_some()));

        // Deux nombres ne sont pas une notation : c'est une fraction.
        assert!(notation("1/2 comprime le matin").is_none());
    }

    /// « le soir au coucher » est **une** prise.
    ///
    /// Deux cases cochées pour une prise, c'est un comprimé de plus par
    /// jour sur la feuille que le patient suit.
    #[test]
    fn the_evening_at_bedtime_is_one_taking() {
        let r = read("1 comprimé le soir au coucher");
        assert_eq!(q(&r, Moment::Coucher), Some(Some(4)));
        assert_eq!(q(&r, Moment::Soir), None);
        assert_eq!(r.takes_a_day(), 1);
    }

    /// Le petit-déjeuner est le matin, bien qu'il contienne
    /// « déjeuner ».
    #[test]
    fn breakfast_is_the_morning_and_not_the_noon() {
        let r = read("1 comprimé avant le petit-déjeuner");
        assert_eq!(q(&r, Moment::Matin), Some(Some(4)));
        assert_eq!(q(&r, Moment::Midi), None);
        assert_eq!(r.meal, "Avant le repas");
    }

    /// Le rapport au repas se lit, et son silence n'est pas
    /// « pendant ».
    #[test]
    fn a_silence_about_the_meal_is_not_during_the_meal() {
        assert_eq!(read("1 comprimé le matin à jeun").meal, "À jeun");
        assert_eq!(
            read("1 comprimé au cours du repas le midi").meal,
            "Pendant le repas"
        );
        assert_eq!(read("1 comprimé le matin").meal, "");
    }

    /// **Aucun moment coché n'est un moment que la phrase ne dit pas.**
    ///
    /// La confrontation aux mille sept cent trente-six posologies
    /// livrées, et non à six phrases écrites ici : une table peut être
    /// parfaitement cohérente avec elle-même et fausse sur une fiche,
    /// et c'est la rencontre qui le montre. Elle a servi le jour où
    /// elle a été écrite — « en début d'après-midi » cochait le midi,
    /// parce que le mot contient l'autre, exactement comme le petit
    /// déjeuner contient le déjeuner.
    #[test]
    fn no_moment_is_ticked_that_the_sentence_does_not_name() {
        let vocabulary = [
            (Moment::Matin, MATIN),
            (Moment::Midi, MIDI),
            (Moment::Soir, SOIR),
            (Moment::Coucher, COUCHER),
        ];
        let mut placed = 0;
        for (brand, _indication, posologie, _remarque) in crate::db::STARTER_POSOLOGIES {
            let r = read(posologie);
            if r.kind != Kind::Placed {
                continue;
            }
            placed += 1;
            let folded = normalize(&crate::fuzzy::sort_key(posologie));
            for (m, words) in vocabulary {
                if r.doses[m.index()].is_none() {
                    continue;
                }
                assert!(
                    words.iter().any(|w| folded.contains(w))
                        || folded.contains('-')
                        || folded.contains('/'),
                    "{brand} : « {posologie} » coche {} sans le dire",
                    m.label()
                );
            }
        }
        // Et la lecture sert à quelque chose : une table qui ne
        // placerait rien passerait la boucle ci-dessus sans rien
        // prouver.
        assert!(
            placed > 100,
            "seulement {placed} posologies livrées placées dans la journée"
        );
    }

    /// **Une demande gouverne une prise, jamais une dose.**
    ///
    /// « Si besoin » se dit de deux choses dans une ordonnance, et une
    /// seule sort de la grille : « à renouveler si besoin toutes les 6
    /// heures » est une prise à la demande, « 5 mg une fois par jour,
    /// portés si besoin à 10 mg » est une titration. Quinze fiches
    /// livrées portaient la seconde forme et sortaient toutes de la
    /// grille du jour, « à la demande » écrit devant — sur la feuille
    /// d'un antihypertenseur, c'est-à-dire du traitement qu'il faut
    /// prendre justement sans le sentir.
    ///
    /// La confrontation se fait dans **les deux sens**, et c'est ce qui
    /// la rend utile : un veto qui prendrait tout ferait disparaître le
    /// « si besoin » de l'ibuprofène, qui est la prise à la demande la
    /// plus délivrée du comptoir.
    #[test]
    fn a_demand_governs_a_taking_and_never_a_dose() {
        // Des titrations : la prise est quotidienne, la demande porte
        // sur la dose.
        for text in [
            "5 mg une fois par jour le matin, portés si besoin à 10 mg",
            "50 mg une fois par jour, portés si besoin à 100 mg par jour après 3 à 6 semaines",
            "20 mg par jour le matin, augmentation par paliers de 10 mg si besoin",
            "40 mg par jour en une prise, portés à 80 mg si nécessaire",
            "100 à 300 mg par jour, associés à une hydratation abondante et à une \
             alcalinisation des urines si nécessaire",
        ] {
            assert_ne!(
                read(text).kind,
                Kind::OnDemand,
                "« {text} » est une titration, pas une prise à la demande"
            );
        }
        // Et des prises à la demande, que le veto ne doit pas manger —
        // un plafond énoncé **après** le mot n'est pas un rythme.
        for text in [
            "200 à 400 mg par prise, à renouveler si besoin toutes les 6 heures ; \
             maximum 1200 mg par jour en automédication",
            "1 à 2 comprimés par prise, à renouveler au bout de 6 heures si besoin",
            "1 inhalation à la demande lors des épisodes de dyspnée",
            "1 comprimé si besoin, 6 heures entre deux prises",
        ] {
            let r = read(text);
            assert_eq!(r.kind, Kind::OnDemand, "« {text} »");
            assert!(!r.has_grid(), "« {text} »");
        }
    }

    /// **Une durée n'est pas un rythme.**
    ///
    /// Le seul piège de la table des rythmes, et elle y est tombée : la
    /// liste a porté « cure de », qui ne nomme aucune répétition. Sur
    /// les fiches livrées il ne désignait que deux traitements
    /// quotidiens décrits par la durée de leur cure, et il les sortait
    /// tous deux de la grille du jour — « à ne pas prendre tous les
    /// jours » sur une feuille remise au patient, pour un antihistaminique.
    ///
    /// Le filet porte sur la **table** et non sur les données : chaque
    /// motif doit nommer une répétition, ce qu'un mot de durée ne fait
    /// pas. Un test qui se contenterait de relire les fiches livrées
    /// repasserait au vert le jour où la faute revient sur un mot que
    /// la base ne dit pas encore.
    /// **Un jour nommé est un rythme** : « 3 cp le lundi matin » (le
    /// méthotrexate) se dessinait trois comprimés chaque matin. Un jour de
    /// début ou de fin, lui, n'en est pas un.
    #[test]
    fn a_named_day_or_a_ratio_is_not_a_daily_dose() {
        for text in [
            "3 cp le lundi matin",
            "1 cp le mercredi soir",
            "1 cp le soir 5 jours sur 7",
            "1 cp le matin 1 semaine sur 2",
            "1 cp par jour 21 jours sur 28",
            "1 cp le matin les jours impairs",
            "1 cp le matin sauf le dimanche",
        ] {
            assert_eq!(read(text).kind, Kind::Cyclic, "{text}");
        }
        for text in [
            "1 cp le matin à partir du lundi",
            "1 cp le matin dès lundi",
            "1 cp le soir jusqu'à lundi",
        ] {
            assert_eq!(read(text).kind, Kind::Placed, "{text}");
        }
    }

    /// **Ni une date ni une dose en milligrammes ne sont une grille.**
    #[test]
    fn a_date_or_a_strength_is_not_a_grid() {
        let r = read("1 cp le matin dès le 1/3/26");
        assert_eq!(r.kind, Kind::Placed);
        assert_eq!(r.doses[0], Some(Take { quarters: Some(4) }));
        assert!(r.doses[1..].iter().all(Option::is_none), "{:?}", r.doses);
        assert_ne!(read("5-0-5 mg").kind, Kind::Placed);
        // La grille, elle, se lit toujours.
        assert_eq!(read("1-0-1").kind, Kind::Placed);
        assert_eq!(read("1/0/1").kind, Kind::Placed);
        assert_eq!(read("20-0-20 gouttes").kind, Kind::Placed);
    }

    /// « 1 1/2 cp le soir » est un et demi, pas un demi.
    #[test]
    fn a_mixed_number_is_read_whole() {
        let r = read("1 1/2 cp le soir");
        assert_eq!(
            r.doses[2],
            Some(Take { quarters: Some(6) }),
            "{:?}",
            r.doses
        );
        assert_eq!(
            read("1 ½ comprimé le matin").doses[0],
            Some(Take { quarters: Some(6) })
        );
        assert_eq!(
            read("1 cp le matin et 1/2 le soir").doses[2],
            Some(Take { quarters: Some(2) })
        );
    }

    /// Les abréviations du nombre de prises : « 2x/j » n'est pas une prise
    /// par jour.
    #[test]
    fn the_abbreviated_number_of_takes_is_read() {
        for (text, n) in [
            ("1 cp 2x/j", 2),
            ("1 cp 2 x/j", 2),
            ("1 cp 2 fois/j", 2),
            ("1 cp 3 fois/jour", 3),
            ("1 cp x 2/j", 2),
            ("1 cp x3/j", 3),
            ("1 cp/j", 1),
        ] {
            assert_eq!(per_day(text), Some(n), "{text}");
        }
    }

    /// Un schéma en phases sort de la grille du jour.
    #[test]
    fn a_phased_schedule_is_not_drawn_as_one_day() {
        for text in [
            "1 cp le soir pendant 7 jours puis 1 cp matin et soir",
            "2 cp le matin pendant 3 jours, puis 1 cp le matin",
        ] {
            assert_eq!(read(text).kind, Kind::Cyclic, "{text}");
        }
        assert_eq!(read("1 cp matin et soir").kind, Kind::Placed);
        // Une dose de charge, une titration, un J1 : des phases.
        for text in [
            "Dose de charge de 180 mg en une prise, puis 90 mg deux fois par jour",
            "1,25 mg deux fois par jour pendant 2 semaines, puis doublement toutes les 2 semaines",
            "2 cp à J1 puis 1 cp/j de J2 à J5",
        ] {
            assert_eq!(read(text).kind, Kind::Cyclic, "{text}");
        }
        // Un « puis » qui n'ouvre pas de phase laisse la ligne en grille.
        for text in [
            "2 inhalations matin et soir puis se rincer la bouche",
            "5 mg une fois par jour le matin (soit 4 mg puis 8 mg pour la forme tert-butylamine)",
        ] {
            assert_eq!(read(text).kind, Kind::Placed, "{text}");
        }
    }

    /// The common blanks of a real file: an insulin's units, « gél »,
    /// « cpr », a clock hour. Each figure is the one written; an adapted
    /// insulin, an hour with no column and a delay stay without one.
    #[test]
    fn units_abbreviations_and_clock_hours_are_read() {
        let grid = |p: &str| {
            let r = read(p);
            assert_eq!(r.kind, Kind::Placed, "{p}");
            Moment::ALL.map(|m| q(&r, m).flatten().map(|n| n / 4))
        };
        assert_eq!(grid("Lantus 20 UI le soir"), [None, None, Some(20), None]);
        assert_eq!(
            grid("Levemir 12 U matin et soir"),
            [Some(12), None, Some(12), None]
        );
        assert_eq!(
            grid("Novorapid 8 UI matin, 10 UI midi, 6 UI soir"),
            [Some(8), Some(10), Some(6), None]
        );
        assert_eq!(
            grid("Insulatard 10 UI au coucher"),
            [None, None, None, Some(10)]
        );
        assert_eq!(grid("1 gél. matin et soir"), [Some(1), None, Some(1), None]);
        assert_eq!(grid("1 cpr le matin"), [Some(1), None, None, None]);
        assert_eq!(grid("1 cp à 8h et 20h"), [Some(1), None, Some(1), None]);
        assert_eq!(
            grid("1 cp à 8h, 12h et 18h"),
            [Some(1), Some(1), Some(1), None]
        );
        assert_eq!(
            grid("1 comprimé à 8 h et à 20 h"),
            [Some(1), None, Some(1), None]
        );
        assert_eq!(grid("1 cp 8h-20h"), [Some(1), None, Some(1), None]);
        assert_eq!(
            grid("2 gél à 8h et 1 à 20h"),
            [Some(2), None, Some(1), None]
        );
        assert_eq!(grid("Abasaglar 22 UI à 22 h"), [None, None, None, Some(22)]);
        assert_eq!(grid("1 cp le soir à 22h"), [None, None, Some(1), None]);
        // Adapted, past what a figure reads, or a strength: a mark, no figure.
        for p in [
            "6 UI le matin selon glycémie",
            "Humalog 4 UI matin midi soir à adapter",
            "Lovenox 4000 UI le soir",
            "Vitamine D 1000 UI le matin",
        ] {
            let r = read(p);
            assert!(r.has_grid(), "{p}");
            assert!(
                r.doses.iter().flatten().all(|t| t.quarters.is_none()),
                "{p}"
            );
        }
        // No column, a delay, a rhythm, a one-off: no grid.
        for p in [
            "1 cp à 21h",
            "1 cp à 8h, 12h, 16h et 20h",
            "1 cp 2h avant le repas",
            "1 cp toutes les 12h",
            "1 sachet à 20h la veille de la coloscopie",
            "1 sachet le soir la veille de la coloscopie",
        ] {
            assert!(!read(p).has_grid(), "{p}");
        }
    }

    /// Une grille de gouttes ou d'insuline au-delà de 12 reste une
    /// grille ; une date, non.
    #[test]
    fn a_large_slash_grid_is_read_and_a_date_is_not() {
        assert_eq!(read("20/0/20 gouttes").kind, Kind::Placed);
        assert_eq!(read("15/15/15 gouttes").kind, Kind::Placed);
        assert_eq!(read("14/0/10").kind, Kind::Placed);
        assert_eq!(read("10/10/10 gouttes").kind, Kind::Placed);
        assert_eq!(read("6/8/10").kind, Kind::Placed);
        assert_ne!(read("1 cp le 1/3/2026").kind, Kind::Placed);
        assert_eq!(read("10-0-10 UI").kind, Kind::Placed);
        assert_ne!(read("1 cp dès le 1/3/26").kind, Kind::Placed);
        assert_ne!(read("5-0-5 mg").kind, Kind::Placed);
        // Une durée sur un lieu n'est pas un rapport.
        assert_ne!(
            read("1 application par jour, 3 à 6 semaines entre les orteils, 2 à 4 semaines sur la peau glabre").kind,
            Kind::Cyclic
        );
        assert_eq!(read("1 cp par jour, 3 semaines sur 4").kind, Kind::Cyclic);
    }

    #[test]
    fn every_rhythm_word_names_a_repetition_and_not_a_duration() {
        for m in CYCLIC {
            assert!(
                m.contains("semaine")
                    || m.contains("hebdomadaire")
                    || m.contains("mois")
                    || m.contains("mensuel")
                    || m.contains("jour sur")
                    || m.contains("nuit sur")
                    || m.starts_with("tous les")
                    || m.starts_with("toutes les"),
                "« {m} » ne nomme pas une répétition : une durée n'est pas un rythme"
            );
        }
        // Et la confrontation, sur les deux fiches livrées que le mot
        // retiré attrapait.
        for text in [
            "10 mg par jour chez l'adulte, en cure de quelques jours à quelques semaines",
            "Un à deux comprimés par jour, en cure de quelques semaines renouvelable",
        ] {
            assert_ne!(read(text).kind, Kind::Cyclic, "« {text} »");
        }
    }

    /// Les posologies du dossier de démonstration se lisent toutes :
    /// c'est le seul jeu de phrases que toutes les captures montrent, et
    /// une grille vide y serait vue par tout le monde.
    #[test]
    fn every_posology_of_the_demo_file_reads() {
        for text in [
            "5 mg matin et soir",
            "20 mg le soir",
            "5 mg le matin",
            "40 mg le matin",
            "1000 mg matin et soir",
            "500 mg matin et soir, 7 jours",
        ] {
            let r = read(text);
            assert_eq!(r.kind, Kind::Placed, "« {text} »");
            assert!(r.has_grid(), "« {text} »");
        }
    }

    /// **Un nombre n'est une quantité que si quelque chose le dit.**
    ///
    /// Confronté aux 1 719 lignes livrées : la grille du Skenan
    /// imprimait « 12 » au matin et au soir — « toutes les **12**
    /// heures » —, celle des statines « 20 » au soir — « **20** à 40 mg
    /// par jour » —, celle du Mopral « 30 » au matin — « **30** minutes
    /// avant ». Trente-huit fiches l'avaient en première ligne, donc sur
    /// la feuille par défaut, et en gras.
    #[test]
    fn a_number_is_a_quantity_only_when_something_says_so() {
        let r =
            read("Débuter à 30 mg toutes les 12 heures, dose initiale usuelle 30 mg matin et soir");
        assert_eq!(q(&r, Moment::Matin), Some(None), "{r:?}");
        assert_eq!(q(&r, Moment::Soir), Some(None));
        let r = read("20 à 40 mg par jour en une prise le soir");
        assert_eq!(q(&r, Moment::Soir), Some(None), "{r:?}");
        let r = read("20 mg par jour, 30 minutes avant le petit-déjeuner");
        assert_eq!(q(&r, Moment::Matin), Some(None), "{r:?}");
        let r = read("50 mg par jour en 3 prises réparties (12,5 mg matin et midi, 25 mg le soir)");
        assert_eq!(q(&r, Moment::Matin), Some(None), "{r:?}");
        // Une fourchette de comptage est une marque, jamais un des bouts.
        let r = read("1 à 2 comprimés le soir");
        assert_eq!(q(&r, Moment::Soir), Some(None));
        // Et ce qui se lisait déjà se lit toujours.
        let r = read("2 le matin et le soir");
        assert_eq!(q(&r, Moment::Matin), Some(Some(8)));
        assert_eq!(q(&r, Moment::Soir), Some(Some(8)));
        let r = read("1 comprimé le matin et 2 le soir");
        assert_eq!(q(&r, Moment::Matin), Some(Some(4)));
        assert_eq!(q(&r, Moment::Soir), Some(Some(8)));
        let r = read("½ comprimé le soir");
        assert_eq!(q(&r, Moment::Soir), Some(Some(2)));
        // Sur toute la base livrée, aucune case ne porte plus de quatre
        // unités : au-delà, c'est un dosage ou un intervalle qu'on a
        // pris pour un compte.
        for (name, indication, poso, _) in crate::db::STARTER_POSOLOGIES {
            for take in read(poso).doses.iter().flatten() {
                assert!(
                    take.quarters.is_none_or(|q| q <= 16),
                    "{name} / {indication} : {:?} — {poso}",
                    take.quarters
                );
            }
        }
    }

    /// **Un plafond ne dit pas un rythme, et le début d'une crise est
    /// une demande.** « 50 mg dès le début de la céphalée … sans
    /// dépasser 300 mg par jour » se lisait « 1 prise par jour », et la
    /// fiche demandait une heure fixe pour un triptan.
    #[test]
    fn a_ceiling_is_not_a_rhythm_and_an_attack_is_a_demand() {
        let r = read("50 mg dès le début de la céphalée, renouvelable une fois, sans dépasser 300 mg par jour");
        assert_eq!(r.kind, Kind::OnDemand, "{r:?}");
        let r = read("Interdose de morphine égale à un dixième de la dose quotidienne totale");
        assert_eq!(r.kind, Kind::OnDemand, "{r:?}");
        let r = read("500 mg à 1 g par prise, à renouveler au bout de 4 à 6 heures, sans dépasser 3 g par jour");
        assert_eq!(r.kind, Kind::Unread, "{r:?}");
        let r = read("Ne pas dépasser 10 mg par jour");
        assert_eq!(r.kind, Kind::Unread, "{r:?}");
        let r = read("150 mg par jour en 2 ou 3 prises");
        assert_eq!(r.kind, Kind::Unread, "{r:?}");
        // Un pas de titration n'est pas un intervalle de prise.
        let r = read("1,25 mg deux fois par jour, doublés toutes les 1 à 2 semaines");
        assert_eq!((r.kind, r.times), (Kind::Daily, 2), "{r:?}");
        let r = read("25 mg par jour, augmentation toutes les 2 semaines");
        assert_eq!((r.kind, r.times), (Kind::Daily, 1), "{r:?}");
        let r = read("200 mg cinq fois par jour, toutes les 4 heures");
        assert_eq!((r.kind, r.times), (Kind::Daily, 5), "{r:?}");
        // Le premier des deux décide.
        let r = read("30 mg toutes les 12 heures. Autre forme : quatre fois par jour.");
        assert_eq!(r.kind, Kind::Unread, "{r:?}");
        // Une consigne d'arrêt n'est pas une demande.
        let r = read("Arrêt immédiat du traitement dès l'apparition d'une diarrhée");
        assert_ne!(r.kind, Kind::OnDemand);
    }

    /// **« Au cours ou en dehors des repas » dit que le repas
    /// n'importe pas**, et « une nuit sur deux » n'est pas chaque nuit.
    #[test]
    fn an_alternative_says_no_meal_and_every_other_night_is_a_rhythm() {
        assert_eq!(read("Prise au cours ou en dehors des repas").meal, "");
        assert_eq!(read("1 comprimé pendant ou en dehors des repas").meal, "");
        assert_eq!(
            read("1 comprimé en dehors des repas").meal,
            "En dehors des repas"
        );
        let r = read("Diminution progressive, en espaçant les prises une nuit sur deux");
        assert_eq!(r.kind, Kind::Cyclic, "{r:?}");
    }

    /// Des moments joints par un trait ou une barre sont autant de
    /// prises : la grille ne gardait que le matin.
    #[test]
    fn moments_joined_by_a_slash_or_a_dash_are_all_taken() {
        for text in ["1 cp matin/soir", "1 cp matin-soir"] {
            let r = read(text);
            assert!(r.doses[0].is_some() && r.doses[2].is_some(), "{text}");
            assert!(r.doses[1].is_none(), "{text}");
        }
        let r = read("1 cp matin-midi-soir");
        assert!(r.doses[..3].iter().all(Option::is_some));
        // La notation chiffrée reste la notation.
        let r = read("1-0-1");
        assert!(r.doses[0].is_some() && r.doses[1].is_none() && r.doses[2].is_some());
    }

    /// La condition d'une prise à la demande commence au repère, même
    /// quand la phrase porte un « petit-déjeuner » ou un « soir au
    /// coucher » avant lui.
    #[test]
    fn the_condition_starts_at_its_marker() {
        let r = read("1 sachet au petit-déjeuner si besoin");
        assert_eq!(r.kind, Kind::OnDemand);
        assert_eq!(r.note, "si besoin");
        let r = read("1 cp le soir au coucher si douleur");
        assert!(r.note.starts_with("si "), "{}", r.note);
    }
}
