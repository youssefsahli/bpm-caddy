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
    "ponctuel",
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
const NOT_ON_DEMAND_BEFORE: &[&str] = &[
    "par jour",
    "chaque jour",
    "tous les jours",
    "quotidien",
    // Les verbes de titration : « portés à », « augmentation par
    // paliers », « jusqu'à … si nécessaire ».
    "porte",
    "augment",
    "jusqu'a",
    "palier",
];

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

/// Les unités qui font d'un nombre une **quantité**.
const UNITS: &[&str] = &[
    "comprime",
    "comprimes",
    "cp",
    "cps",
    "gelule",
    "gelules",
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
        let before = &folded[..at];
        if !NOT_ON_DEMAND_BEFORE.iter().any(|w| before.contains(w)) {
            let mut r = Reading::empty(Kind::OnDemand);
            r.note = condition(raw, m);
            r.meal = meal_of(&plain);
            return r;
        }
    }

    // **Puis le rythme.** Avant de chercher les moments, parce que « le
    // lundi matin » porte « matin » et n'est pas une prise du matin.
    if let Some(m) = CYCLIC.iter().find(|m| folded.contains(**m)) {
        let mut r = Reading::empty(Kind::Cyclic);
        r.note = rhythm(raw, m);
        r.meal = meal_of(&plain);
        return r;
    }

    let meal = meal_of(&plain);

    // La notation chiffrée, quand elle est là : « 1-0-1 » se lit sans
    // ambiguïté et court-circuite tout le reste.
    if let Some(doses) = notation(&folded) {
        return Reading {
            kind: Kind::Placed,
            doses,
            times: 0,
            note: String::new(),
            meal,
        };
    }

    let doses = placed(&folded);
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
    ] {
        while let Some(i) = s.find(from) {
            s.replace_range(i..i + from.len(), to);
        }
    }
    s.replace(['(', ')', ';', ':'], " ")
}

/// Ce que porte la ligne d'un traitement à la demande : sa condition,
/// dans les mots de l'ordonnance.
///
/// La phrase d'origine est reprise à partir du repère, accents compris :
/// c'est elle qui dit *quand* — « en cas de douleur », « si la fièvre
/// dépasse 38,5 » — et une condition résumée est une condition que le
/// patient applique de travers.
fn condition(raw: &str, marker: &str) -> String {
    let folded = normalize(&crate::fuzzy::sort_key(raw));
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

/// Ce que porte la ligne d'un traitement à rythme non quotidien.
///
/// Toute la phrase, et non le seul repère : « 1 comprimé par semaine,
/// le lundi matin » ne se résume pas à « par semaine », et c'est le
/// jour qui manque le plus au patient.
fn rhythm(raw: &str, _marker: &str) -> String {
    raw.trim().to_owned()
}

/// Le rapport au repas, ou rien.
fn meal_of(folded: &str) -> String {
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
    let token = folded
        .split_whitespace()
        .find(|t| t.contains('-') || t.contains('/'))?;
    let sep = if token.contains('-') { '-' } else { '/' };
    let parts: Vec<&str> = token.split(sep).collect();
    if parts.len() != 3 && parts.len() != 4 {
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

/// Placer les prises nommées.
///
/// Un passage de gauche à droite : un nombre s'accroche au moment qui
/// le suit, et un moment sans nombre à lui reprend le dernier nombre
/// énoncé — c'est ainsi que « 1 comprimé matin, midi et soir » se lit,
/// et c'est la seule lecture que la langue autorise.
fn placed(folded: &str) -> [Option<Take>; 4] {
    let mut doses: [Option<Take>; 4] = [None; 4];
    let tokens: Vec<&str> = folded.split_whitespace().collect();
    // Le dernier nombre énoncé, et celui qui n'a pas encore trouvé son
    // moment. Deux variables et non une : « 2 le matin et le soir » doit
    // donner 2 aux deux, tandis que « 1 comprimé le matin et 2 le soir »
    // ne doit pas donner 1 au soir.
    let mut pending: Option<u32> = None;
    let mut last: Option<u32> = None;
    let mut i = 0;
    while i < tokens.len() {
        let t = tokens[i].trim_matches(|c: char| c == ',' || c == '.');
        // Un nombre : quantité, ou dosage si une unité de masse suit.
        if let Some(q) = quarters(t) {
            let next = tokens.get(i + 1).map(|n| {
                n.trim_matches(|c: char| !c.is_alphanumeric() && c != '%')
                    .to_owned()
            });
            let is_strength = next.as_deref().is_some_and(|n| STRENGTHS.contains(&n));
            if !is_strength && q > 0 {
                pending = Some(q);
                last = Some(q);
                // Une unité de comptage juste après confirme, et n'est
                // pas un moment : on la saute.
                if next.as_deref().is_some_and(|n| UNITS.contains(&n)) {
                    i += 2;
                    continue;
                }
            }
            i += 1;
            continue;
        }
        if let Some(m) = moment_of(t) {
            let q = pending.take().or(last);
            doses[m.index()] = Some(Take { quarters: q });
        }
        i += 1;
    }
    // Les locutions en deux mots — « au lit » — que le passage par
    // jeton ne voit pas.
    if doses[3].is_none() && (folded.contains("au lit") || folded.contains("avant de dormir")) {
        doses[3] = Some(Take { quarters: last });
    }
    doses
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

/// Combien de prises par jour, quand la phrase le dit sans dire quand.
fn per_day(folded: &str) -> Option<u32> {
    for (needle, times) in [
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

    fn q(r: &Reading, m: Moment) -> Option<Option<u32>> {
        r.doses[m.index()].map(|t| t.quarters)
    }

    /// **Le silence n'est pas une case vide.**
    ///
    /// Une posologie illisible n'a pas de grille du tout : quatre cases
    /// blanches se lisent « rien à prendre », ce qui est exactement le
    /// contraire de « je n'ai pas su lire ».
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
    #[test]
    fn every_rhythm_word_names_a_repetition_and_not_a_duration() {
        for m in CYCLIC {
            assert!(
                m.contains("semaine")
                    || m.contains("hebdomadaire")
                    || m.contains("mois")
                    || m.contains("mensuel")
                    || m.contains("jour sur")
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
}
