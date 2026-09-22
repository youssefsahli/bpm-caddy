//! Ce qu'une posologie écrite dit d'une dose **au poids**.
//!
//! La question est celle du comptoir : « le Clamoxyl, pour un enfant de
//! seize kilos, ça fait combien ». Les calculs de la maison savent
//! multiplier un poids par des milligrammes ; ce qu'ils ne savaient pas,
//! c'est **d'où sort le nombre de milligrammes** — il fallait le
//! connaître, donc ne pas avoir besoin de l'outil. Or les fiches
//! l'écrivent : une cinquantaine de fois dans les posologies de la base
//! livrée, sur une trentaine de fiches.
//!
//! **La posologie et rien d'autre.** « mg/kg » se lit aussi dans les
//! conseils au patient et dans les indications, où il raconte plutôt
//! qu'il ne prescrit ; un outil qui calcule prend ses chiffres là où la
//! fiche écrit sa posologie.
//!
//! Ce module lit ces phrases-là et rien d'autre. Il ne calcule pas, il
//! ne choisit pas, et surtout **il n'invente pas** : une fiche qui
//! n'écrit pas de dose au poids rend une liste vide, et l'écran le dit.
//! C'est la règle de `crush.rs` — le silence n'est pas une autorisation
//! — appliquée à un chiffre : une dose absente proposée à zéro serait
//! pire qu'une absence, parce qu'elle se lirait comme une réponse.
//!
//! **La cadence n'est pas un détail, c'est la moitié de la dose.** Une
//! même fiche écrit « 15 mg/kg par prise toutes les 6 heures, soit
//! 60 mg/kg par 24 heures » : le même enfant, le même médicament, et un
//! facteur quatre entre les deux lectures. Le type porte donc la
//! cadence, et l'écran ne montre jamais un chiffre sans elle.
//!
//! Pur, testé, et confronté aux fiches livrées : une grammaire inventée
//! au bureau lit très bien les phrases qu'on a écrites pour elle.

/// Sous quel rythme la phrase donne la dose.
///
/// **Trois cas et non deux** : une phrase qui ne le dit pas ne dit pas
/// « par jour ». « 1 mg/kg » seul, sur la fiche d'une héparine, est une
/// dose par injection ; le lire comme une dose quotidienne serait la
/// diviser par deux, et l'inverse la doubler.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cadence {
    /// Par prise, ou « toutes les N heures », ce qui est la même chose
    /// dite autrement.
    Prise,
    /// Par jour, par 24 heures, ou écrit « /jour » contre l'unité.
    Jour,
    /// La phrase donne les milligrammes par kilo et pas le rythme.
    NonDite,
}

impl Cadence {
    /// La clé de son libellé français.
    pub fn label_key(self) -> &'static str {
        match self {
            Cadence::Prise => "dosing_par_prise",
            Cadence::Jour => "dosing_par_jour",
            Cadence::NonDite => "dosing_sans_rythme",
        }
    }
}

/// Une dose au poids, telle qu'une fiche l'écrit.
#[derive(Clone, PartialEq, Debug)]
pub struct PerKilo {
    /// La dose, en milligrammes par kilo. Quand la phrase donne une
    /// fourchette, c'est sa borne basse.
    pub low: f64,
    /// La borne haute, **quand la phrase en donne une**.
    ///
    /// Une fourchette reste une fourchette : « 80 à 90 mg/kg/jour » ne
    /// se moyenne pas en 85. Le prescripteur a écrit deux bornes, et
    /// laquelle vaut pour cet enfant-là dépend de l'indication, que
    /// cette lecture ne connaît pas.
    pub high: Option<f64>,
    pub cadence: Cadence,
    /// **Un plafond n'est pas une dose.** « sans dépasser 30 mg/kg par
    /// jour » donne la borne qu'on ne franchit pas, et l'écrire à
    /// l'écran comme la dose à donner ferait prendre le maximum pour
    /// l'habitude.
    pub ceiling: bool,
    /// La molécule que la phrase nomme derrière l'unité, quand elle en
    /// nomme une — « 30 mg/kg par jour **de sulfaméthoxazole** et
    /// 6 mg/kg par jour **de triméthoprime** ». Une association donne
    /// deux chiffres pour un même médicament, et sans ce nom le second
    /// se lit comme une seconde dose du premier.
    pub of: Option<String>,
    /// En combien de prises la dose du jour se répartit, quand la
    /// phrase le dit d'un seul chiffre (« en 2 prises »). « En 2 ou 3
    /// prises » laisse le choix au prescripteur et reste `None`.
    pub takes: Option<u32>,
    /// La phrase d'où elle vient, telle que la fiche l'écrit.
    ///
    /// **C'est elle qui décide, pas le chiffre.** Une même fiche donne
    /// souvent deux doses au poids pour deux indications — cinquante
    /// pour l'angine, quatre-vingts pour l'otite — et rien dans les
    /// nombres ne dit laquelle. La phrase le dit ; l'écran la montre, et
    /// c'est la personne au comptoir qui choisit.
    pub sentence: String,
}

/// Lire toutes les doses au poids d'une posologie, dans l'ordre où elle
/// les écrit.
pub fn read(dosage: &str) -> Vec<PerKilo> {
    let mut out = Vec::new();
    // On cherche « mg/kg » et rien d'autre : la base écrit aussi
    // « 100 UI/kg », qui est une dose au poids mais pas en
    // milligrammes, et la confondre reviendrait à lire cent là où il y
    // en a un.
    let hay = dosage;
    let mut from = 0usize;
    while let Some(at) = hay[from..].find("mg/kg") {
        let start = from + at;
        let end = start + "mg/kg".len();
        if let Some((low, high, begin)) = figures_before(hay, start) {
            // **Une dose cumulée n'est pas une dose.** « une dose
            // cumulée de 120 à 150 mg/kg sur l'ensemble de la cure »
            // (l'isotrétinoïne) est le total d'un traitement de
            // plusieurs mois ; lue comme une dose, elle donnait sept
            // grammes à un adulte de soixante kilos.
            if !cumulative_before(hay, start) {
                let tail = rhythm_window(hay, end);
                out.push(PerKilo {
                    low,
                    high,
                    cadence: cadence_of(hay, end, &tail),
                    ceiling: ceiling_around(hay, begin, &tail),
                    of: component_after(hay, end),
                    takes: takes_in(&tail),
                    sentence: sentence_around(hay, start),
                });
            }
        }
        from = end;
    }
    out
}

/// Une dose au poids et l'indication qui la porte, quand elle vient
/// d'une ligne par indication plutôt que de la posologie de la carte.
#[derive(Clone, PartialEq, Debug)]
pub struct Found {
    pub indication: Option<String>,
    pub dose: PerKilo,
}

/// Tout ce qu'une fiche écrit au poids : **ses lignes par indication
/// d'abord**, parce qu'elles disent pour quoi — c'est la question que
/// le chiffre seul ne départage pas —, puis ce que sa posologie écrit
/// et qu'aucune ligne n'a déjà donné.
///
/// La carte du Bactrim ne dit rien au poids ; sa ligne « Infections de
/// l'enfant » écrit trente milligrammes par kilo. Ne lire que la carte
/// répondait « rien » à la question même qui a fait naître l'outil.
pub fn gather(card: &str, lines: &[(&str, &str)]) -> Vec<Found> {
    let mut out: Vec<Found> = lines
        .iter()
        .flat_map(|(indication, text)| {
            read(text).into_iter().map(|dose| Found {
                indication: Some((*indication).to_owned()),
                dose,
            })
        })
        .collect();
    for dose in read(card) {
        let same = |f: &Found| {
            f.dose.low == dose.low
                && f.dose.high == dose.high
                && f.dose.cadence == dose.cadence
                && f.dose.of == dose.of
        };
        if !out.iter().any(same) {
            out.push(Found {
                indication: None,
                dose,
            });
        }
    }
    out
}

/// Des milligrammes calculés, écrits à la précision qu'ils ont.
///
/// **Jamais « 0 mg »** : 0,1 mg/kg de terbutaline pour un nourrisson de
/// quatre kilos fait 0,4 mg, et l'arrondi à l'entier l'écrivait zéro —
/// un chiffre calculé à zéro se lit comme une réponse, ce que ce module
/// refuse. Une décimale sous dix, deux sous un, l'entier au-dessus : un
/// « 480,0 » se lirait comme une précision qu'on n'a pas.
pub fn milligrams(v: f64) -> String {
    let places = if v >= 10.0 {
        0
    } else if v >= 1.0 {
        1
    } else {
        2
    };
    let text = crate::strings::decimal(v, places);
    // « 2,0 » s'écrit « 2 », « 0,40 » s'écrit « 0,4 ».
    let text = if text.contains(',') {
        text.trim_end_matches('0').trim_end_matches(',').to_owned()
    } else {
        text
    };
    if text == "0" {
        crate::strings::decimal(v, 3)
    } else {
        text
    }
}

/// Le nombre — ou les deux bornes — écrits juste avant « mg/kg ».
///
/// À la virgule : les fiches écrivent « 0,35 mg/kg », et lire trente-cinq
/// là où il y a trente-cinq centièmes est un facteur cent sur un enfant.
fn figures_before(hay: &str, at: usize) -> Option<(f64, Option<f64>, usize)> {
    let head = &hay[..at];
    let (second, rest) = number_at_end(head)?;
    // Une fourchette : « 80 à 90 mg/kg ». Le « à » peut aussi s'écrire
    // avec un tiret dans une fiche saisie à la main.
    let rest = rest.trim_end();
    let linked = rest
        .strip_suffix('à')
        .or_else(|| rest.strip_suffix('a'))
        .or_else(|| rest.strip_suffix('-'))
        .or_else(|| rest.strip_suffix('–'));
    // Le troisième terme est l'endroit où le chiffre commence : ce qui
    // précède est la phrase, et c'est là qu'on cherche « sans
    // dépasser » — pas dans « 0,75 », dont la virgule coupait la
    // phrase au milieu du nombre.
    match linked.and_then(number_at_end) {
        Some((first, before)) if first < second => Some((first, Some(second), before.len())),
        _ => Some((second, None, rest.len())),
    }
}

/// Le nombre qui termine ce texte, et ce qui le précède.
fn number_at_end(text: &str) -> Option<(f64, &str)> {
    let trimmed = text.trim_end();
    let digits: String = trimmed
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_digit() || *c == ',' || *c == '.')
        .collect();
    let digits: String = digits.chars().rev().collect();
    // Une virgule ou un point isolés ne sont pas un nombre, et un
    // nombre qui finit par la ponctuation de sa phrase non plus.
    let digits = digits.trim_matches(|c| c == ',' || c == '.');
    if digits.is_empty() || !digits.chars().any(|c| c.is_ascii_digit()) {
        return None;
    }
    let value: f64 = digits.replace(',', ".").parse().ok()?;
    let cut = trimmed.len() - digits.len();
    Some((value, &trimmed[..cut]))
}

/// Ce qui suit l'unité **et appartient encore à cette dose-là**.
///
/// Une fenêtre courte, et bornée à ce qui ouvre la dose suivante :
/// « 20 à 30 mg/kg et par jour, soit 7,5 à 10 mg/kg par prise » porte
/// deux rythmes, et une fenêtre qui déborde sur le second faisait lire
/// la dose du jour comme une dose par prise — un facteur trois ou
/// quatre sur un enfant.
fn rhythm_window(hay: &str, at: usize) -> String {
    let tail: String = hay[at..]
        .chars()
        .take(48)
        .collect::<String>()
        .to_lowercase();
    let mut cut = tail.len();
    for stop in ["soit ", ";", ". ", "mg/kg", "sans dépasser"] {
        // Pas au tout début : « mg/kg/jour » commence par l'unité
        // qu'on vient de lire, et la fenêtre s'arrêterait avant son
        // propre rythme.
        if let Some(i) = tail.get(1..).and_then(|t| t.find(stop)) {
            cut = cut.min(i + 1);
        }
    }
    tail[..cut].to_owned()
}

/// Ce que la phrase dit du rythme, juste après l'unité.
fn cadence_of(hay: &str, at: usize, window: &str) -> Cadence {
    let tail = &hay[at..];
    // Collé à l'unité : « 50 mg/kg/jour ».
    let glued = tail.trim_start_matches('/');
    if glued.len() < tail.len() && (glued.starts_with("jour") || glued.starts_with('j')) {
        return Cadence::Jour;
    }
    // La prise d'abord : « par prise toutes les 6 heures » contient les
    // deux, et c'est bien une dose par prise. **Et « deux fois par
    // jour » aussi** : « 10 mg/kg deux fois par jour » donne dix par
    // prise, vingt dans la journée — le lire par jour divisait la dose
    // par deux.
    if window.contains("par prise")
        || window.contains("toutes les")
        || window.contains("fois par jour")
        || window.contains("fois dans la journée")
        || window.contains("par injection")
        || window.contains("par nébulisation")
    {
        return Cadence::Prise;
    }
    if window.contains("par jour")
        || window.contains("par 24 heures")
        || window.contains("par vingt-quatre")
        || window.contains("quotidien")
    {
        return Cadence::Jour;
    }
    // « 40 mg/kg en une prise », « en dose unique » : toute la dose,
    // prise une fois — c'est une dose par prise, et la dire « sans
    // rythme » était la lire moins bien qu'elle n'est écrite.
    if window.contains("en une prise")
        || window.contains("prise unique")
        || window.contains("dose unique")
    {
        return Cadence::Prise;
    }
    Cadence::NonDite
}

/// « dose cumulée », juste avant le chiffre.
fn cumulative_before(hay: &str, at: usize) -> bool {
    let head: String = hay[..at]
        .chars()
        .rev()
        .take(40)
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>()
        .to_lowercase();
    head.contains("cumul")
}

/// Le chiffre est-il un plafond plutôt qu'une dose ? `at` est l'endroit
/// où le chiffre commence.
fn ceiling_around(hay: &str, at: usize, window: &str) -> bool {
    let head: String = hay[..at]
        .chars()
        .rev()
        .take(30)
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>()
        .to_lowercase();
    // Seulement la fin de ce qui précède : « sans dépasser 2 g par
    // jour. Enfant : 50 mg/kg » ne fait pas de la dose de l'enfant un
    // plafond.
    let head = head.rsplit(['.', ';', ',']).next().unwrap_or("");
    // « jusqu'à environ 1 mg/kg par jour **et davantage** » n'est pas
    // un plafond : la phrase dit elle-même qu'on le franchit.
    if window.contains("davantage") || window.contains("et plus") {
        return false;
    }
    ["dépasser", "maximum", "jusqu'à", "au plus", "inférieure à"]
        .iter()
        .any(|w| head.contains(w))
        // « 60 mg/kg par jour au maximum » — collé au rythme, et pas
        // « …, 5 jours au maximum », qui borne une durée.
        || window.split(',').next().is_some_and(|c| c.contains("au maximum"))
}

/// La molécule nommée juste derrière l'unité et son rythme.
fn component_after(hay: &str, at: usize) -> Option<String> {
    let tail = hay[at..].trim_start_matches('/');
    let mut rest = tail.trim_start();
    for lead in [
        "jour",
        "et par jour",
        "par jour",
        "par 24 heures",
        "par prise",
    ] {
        if let Some(r) = rest.strip_prefix(lead) {
            rest = r.trim_start();
            break;
        }
    }
    let name = rest
        .strip_prefix("exprimés en ")
        .or_else(|| rest.strip_prefix("de "))
        .or_else(|| rest.strip_prefix("d'"))
        .or_else(|| rest.strip_prefix("d’"))?;
    let word: String = name
        .chars()
        .take_while(|c| c.is_alphabetic() || *c == '-')
        .collect();
    // Un mot court, ou un mot qui ne nomme pas une molécule — « de
    // poids », « de charge », « de traitement » — ne nomme rien ici.
    const NOT_A_MOLECULE: &[&str] = &["poids", "charge", "traitement", "entretien", "base"];
    (word.chars().count() >= 5 && !NOT_A_MOLECULE.contains(&word.to_lowercase().as_str()))
        .then_some(word)
}

/// « en 2 prises », « en trois prises » — un seul chiffre.
fn takes_in(window: &str) -> Option<u32> {
    let (_, after) = window.split_once("en ")?;
    let mut words = after.split_whitespace();
    let n = match words.next()? {
        "1" | "une" => 1,
        "2" | "deux" => 2,
        "3" | "trois" => 3,
        "4" | "quatre" => 4,
        _ => return None,
    };
    words
        .next()
        .is_some_and(|w| w.starts_with("prise"))
        .then_some(n)
}

/// La phrase qui porte cette dose, bornée.
///
/// Bornée parce qu'une posologie écrit parfois trois lignes sans point,
/// et qu'une bulle de trois cents pixels ne les porte pas. On coupe au
/// dernier espace, jamais au milieu d'un mot.
fn sentence_around(hay: &str, at: usize) -> String {
    let start = hay[..at]
        .rfind(". ")
        .map(|i| i + 2)
        .or_else(|| hay[..at].rfind("; ").map(|i| i + 2))
        .unwrap_or(0);
    let rest = &hay[at..];
    let stop = rest.find(". ").map(|i| at + i + 1).unwrap_or(hay.len());
    let cut = hay[start..stop].trim();
    const MAX: usize = 180;
    if cut.chars().count() <= MAX {
        return cut.to_owned();
    }
    let head: String = cut.chars().take(MAX).collect();
    let head = head.rsplit_once(' ').map_or(head.as_str(), |(h, _)| h);
    format!("{head}…")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Une dose par prise n'est pas une dose par jour**, et c'est la
    /// règle de ce module.
    ///
    /// La fiche du Doliprane écrit les deux dans la même phrase :
    /// « 15 mg/kg par prise toutes les 6 heures, soit 60 mg/kg par
    /// 24 heures ». Le même enfant, le même médicament, un facteur
    /// quatre. Un outil qui rendrait « 15 » sans son rythme ferait
    /// prendre quinze milligrammes par kilo et par jour à qui en a
    /// besoin de soixante, ou l'inverse.
    #[test]
    fn a_dose_per_take_is_not_a_dose_per_day() {
        let got = read(
            "Enfant : 15 mg/kg par prise toutes les 6 heures, soit 60 mg/kg \
             par 24 heures, à répartir en quatre prises régulières.",
        );
        assert_eq!(got.len(), 2, "{got:?}");
        assert_eq!(got[0].low, 15.0);
        assert_eq!(got[0].cadence, Cadence::Prise);
        assert_eq!(got[1].low, 60.0);
        assert_eq!(got[1].cadence, Cadence::Jour);
        // « /jour » collé à l'unité, qui est l'autre écriture.
        let got = read("Enfant : 50 mg/kg/jour en deux prises pour l'angine.");
        assert_eq!(got[0].cadence, Cadence::Jour);
        // « toutes les N heures » est une dose par prise dite autrement.
        let got = read("Enfant : 15 mg/kg toutes les 6 heures.");
        assert_eq!(got[0].cadence, Cadence::Prise);
        // **Et ce que la phrase ne dit pas n'est pas complété.** Sur une
        // héparine, « 1 mg/kg » est une dose par injection ; la lire
        // par jour la diviserait par deux.
        let got = read("Traitement curatif : 100 UI/kg, soit 1 mg/kg, puis relais.");
        assert_eq!(got.len(), 1, "« UI/kg » n'est pas « mg/kg » : {got:?}");
        assert_eq!(got[0].low, 1.0);
        assert_eq!(got[0].cadence, Cadence::NonDite);
    }

    /// **Une fourchette reste une fourchette**, et une virgule reste une
    /// virgule.
    ///
    /// « 80 à 90 mg/kg/jour » ne se moyenne pas en 85 : le prescripteur
    /// a écrit deux bornes et laquelle vaut dépend de l'indication, que
    /// cette lecture ne connaît pas. Et « 0,35 mg/kg » lu trente-cinq
    /// est un facteur cent.
    #[test]
    fn a_range_stays_a_range_and_a_comma_stays_a_comma() {
        let got = read("et 80 à 90 mg/kg/jour en deux ou trois prises pour l'otite.");
        assert_eq!(got[0].low, 80.0);
        assert_eq!(got[0].high, Some(90.0));
        let got = read("de l'ordre de 0,35 mg/kg par jour dans les formes légères.");
        assert!((got[0].low - 0.35).abs() < 1e-9, "{got:?}");
        assert_eq!(got[0].high, None);
        let got = read("à la dose de 0,5 à 1 mg/kg de codéine par prise.");
        assert!((got[0].low - 0.5).abs() < 1e-9);
        assert_eq!(got[0].high, Some(1.0));
        assert_eq!(got[0].cadence, Cadence::Prise);
        // Un « à » qui n'ouvre pas de fourchette ne la ferme pas non
        // plus : la borne haute ne peut pas être sous la basse.
        let got = read("porter la dose à 20 mg/kg par jour.");
        assert_eq!((got[0].low, got[0].high), (20.0, None));
    }

    /// **Ce que la fiche n'écrit pas n'est pas inventé.**
    ///
    /// C'est la première règle de `crush.rs` appliquée à un chiffre :
    /// une dose absente proposée à zéro se lirait comme une réponse.
    /// L'écran doit dire « cette fiche n'écrit pas de dose au poids »,
    /// et pour cela la lecture doit rendre une liste vide.
    #[test]
    fn what_the_card_does_not_write_is_not_invented() {
        assert!(read("").is_empty());
        assert!(read("Un comprimé deux fois par jour. Cystite : 3 jours.").is_empty());
        // Une unité au kilo qui n'est pas en milligrammes non plus.
        assert!(read("Prophylaxie : 4 000 UI anti-Xa par jour.").is_empty());
        // Ni un « mg/kg » sans chiffre devant : une fiche saisie à la
        // main écrit parfois « la dose en mg/kg figure au RCP ».
        assert!(read("la dose en mg/kg figure au résumé des caractéristiques.").is_empty());
    }

    /// **La phrase voyage avec le chiffre.**
    ///
    /// Une même fiche donne deux doses au poids pour deux indications —
    /// cinquante pour l'angine, quatre-vingts pour l'otite — et rien
    /// dans les nombres ne dit laquelle. C'est la phrase qui le dit.
    #[test]
    fn the_sentence_travels_with_the_figure() {
        let got = read(
            "Adulte : 1 g trois fois par jour. Enfant : 50 mg/kg/jour en deux \
             prises pour l'angine pendant 6 jours, et 80 à 90 mg/kg/jour pour \
             l'otite moyenne aiguë.",
        );
        assert_eq!(got.len(), 2);
        for g in &got {
            assert!(
                g.sentence.contains("Enfant"),
                "la phrase de l'adulte n'est pas celle de l'enfant : {:?}",
                g.sentence
            );
            assert!(!g.sentence.contains("Adulte : 1 g"), "{:?}", g.sentence);
        }
        assert!(got[0].sentence.contains("angine"));
        // Bornée : une posologie écrit parfois trois lignes sans point.
        let long = format!(
            "Enfant : 30 mg/kg par jour {}",
            "et ainsi de suite ".repeat(30)
        );
        let got = read(&long);
        assert!(
            got[0].sentence.chars().count() <= 181,
            "{}",
            got[0].sentence
        );
        assert!(got[0].sentence.ends_with('…'));
    }

    /// **Confrontée aux fiches livrées**, parce qu'une grammaire
    /// inventée au bureau lit très bien les phrases écrites pour elle.
    ///
    /// Le compte n'est pas rond par hasard : il est celui de la base, et
    /// il bougera le jour où une fiche gagnera une dose au poids — ce
    /// qui est exactement le moment où il faut relire cette lecture.
    #[test]
    fn what_the_shipped_cards_write_is_read() {
        let mut total = 0usize;
        let mut cards = 0usize;
        let mut sans_rythme = 0usize;
        for (name, _, _, _) in crate::db::STARTER_DRUGS {
            let d = crate::db::STARTER_DETAILS
                .iter()
                .find(|d| d.name == *name)
                .map(|d| d.dosage)
                .unwrap_or("");
            let got = read(d);
            if !got.is_empty() {
                cards += 1;
            }
            total += got.len();
            for g in &got {
                // Aucune fiche n'écrit une dose au poids absurde : une
                // borne à zéro serait une lecture fautive, et au delà de
                // deux cents ce n'est plus une dose par kilo.
                assert!(
                    g.low > 0.0 && g.low <= 200.0,
                    "{name} : {} mg/kg — {}",
                    g.low,
                    g.sentence
                );
                if let Some(h) = g.high {
                    assert!(h > g.low, "{name} : fourchette {}–{h}", g.low);
                }
                assert!(
                    !g.sentence.trim().is_empty(),
                    "{name} : chiffre sans phrase"
                );
                if g.cadence == Cadence::NonDite {
                    sans_rythme += 1;
                }
            }
        }
        // Mesuré sur la base livrée : trente-deux fiches, cinquante et
        // une doses. Le plancher laisse la marge d'une fiche corrigée ;
        // s'il tombe, c'est la lecture qui a cassé, pas le contenu.
        assert!(
            cards >= 28 && total >= 46,
            "{cards} fiches, {total} doses au poids lues"
        );
        // **Les deux questions du comptoir, et leurs deux réponses.**
        let dosage = |name: &str| {
            crate::db::STARTER_DETAILS
                .iter()
                .find(|d| d.name == name)
                .map(|d| d.dosage)
                .unwrap_or_else(|| panic!("fiche absente : {name}"))
        };
        // Celle qui répond : l'amoxicilline écrit cinquante par jour pour
        // l'angine et quatre-vingts à quatre-vingt-dix pour l'otite —
        // deux doses, deux indications, et c'est la phrase qui départage.
        let clamoxyl = read(dosage("Amoxicilline"));
        assert_eq!(clamoxyl.len(), 2, "{clamoxyl:?}");
        assert_eq!((clamoxyl[0].low, clamoxyl[0].high), (50.0, None));
        assert_eq!(clamoxyl[0].cadence, Cadence::Jour);
        assert!(clamoxyl[0].sentence.contains("angine"));
        assert_eq!((clamoxyl[1].low, clamoxyl[1].high), (80.0, Some(90.0)));
        assert!(clamoxyl[1].sentence.contains("otite"));
        // Et celle dont la carte ne répond pas : la fiche du Bactrim
        // n'écrit pas de dose au poids **dans sa posologie**. Ses lignes
        // par indication, elles, l'écrivent — c'est
        // `the_indication_lines_answer_what_the_card_does_not` qui les
        // lit.
        assert!(
            read(dosage("Bactrim")).is_empty(),
            "la fiche du Bactrim écrit maintenant une dose au poids : \
             relire ce que l'écran en fait"
        );
        // **Et la plupart disent leur rythme.** Si cette part s'effondre,
        // c'est la lecture du rythme qui a cassé, pas les fiches.
        assert!(
            sans_rythme * 4 < total,
            "{sans_rythme} doses sur {total} sans rythme : la lecture du \
             rythme ne lit plus"
        );
    }

    /// **Un second rythme ne prête pas le sien au premier.**
    ///
    /// La fiche de l'Advil écrit « 20 à 30 mg/kg par jour, soit 7,5 à
    /// 10 mg/kg par prise » : la fenêtre qui suivait la première dose
    /// allait jusqu'au « par prise » de la seconde, et l'écran
    /// annonçait vingt à trente milligrammes **par prise** — trois à
    /// quatre fois la dose, sur un enfant.
    #[test]
    fn a_second_rhythm_does_not_lend_its_own_to_the_first() {
        let got = read(
            "Enfant à partir de 3 mois et 5 kg : 20 à 30 mg/kg par jour, soit 7,5 \
             à 10 mg/kg par prise toutes les 6 à 8 heures, sans dépasser 30 mg/kg \
             par 24 heures.",
        );
        assert_eq!(got.len(), 3, "{got:?}");
        assert_eq!(got[0].cadence, Cadence::Jour, "{:?}", got[0]);
        assert_eq!(got[1].cadence, Cadence::Prise);
        assert_eq!(got[2].cadence, Cadence::Jour);
        // Et « deux fois par jour » est une dose par prise : dix
        // milligrammes deux fois, vingt dans la journée.
        let got = read("10 mg/kg deux fois par jour, maximum 30 mg/kg deux fois par jour");
        assert_eq!(got[0].cadence, Cadence::Prise);
        let got = read("0,25 mg/kg jusqu'à trois fois par jour");
        assert_eq!(got[0].cadence, Cadence::Prise);
        let got = read("Bilharziose : 40 mg/kg en une prise.");
        assert_eq!(got[0].cadence, Cadence::Prise);
        let got = read("0,2 mg/kg par injection, renouvelable toutes les 4 heures");
        assert_eq!(got[0].cadence, Cadence::Prise);
    }

    /// **Un plafond n'est pas une dose, et une dose cumulée non plus.**
    ///
    /// « sans dépasser 30 mg/kg par 24 heures » est la borne qu'on ne
    /// franchit pas ; « une dose cumulée de 120 à 150 mg/kg » est le
    /// total d'une cure de plusieurs mois — lue comme une dose, elle
    /// donnait sept grammes à un adulte.
    #[test]
    fn a_ceiling_is_not_a_dose_and_a_course_total_is_not_read() {
        let got = read("20 mg/kg par jour, sans dépasser 30 mg/kg par jour.");
        assert!(!got[0].ceiling);
        assert!(got[1].ceiling);
        // La virgule du nombre ne coupe pas la phrase : « ni 0,5 » est
        // bien précédé de « sans dépasser ».
        let got = read("sans dépasser 30 mg par jour ni 0,5 mg/kg par jour.");
        assert!(got[0].ceiling, "{got:?}");
        let got = read("soit 60 mg/kg par jour au maximum ; ne pas associer.");
        assert!(got[0].ceiling);
        // « au maximum » qui borne une durée ne borne pas la dose.
        let got = read("0,1 mg/kg par prise, trois fois, 5 jours au maximum");
        assert!(!got[0].ceiling, "{got:?}");
        // « et davantage » dit lui-même qu'on franchit le chiffre.
        let got = read("jusqu'à environ 1 mg/kg par jour et davantage dans les formes graves");
        assert!(!got[0].ceiling);
        // Une phrase qui finit sur un plafond d'adulte ne fait pas de la
        // dose de l'enfant un plafond.
        let got = read("sans dépasser 2 g par jour. Enfant : 50 mg/kg par jour.");
        assert!(!got[0].ceiling);
        let got = read(
            "souvent porté à 0,5 à 1 mg/kg et par jour, pour une dose cumulée de \
             l'ordre de 120 à 150 mg/kg sur l'ensemble de la cure.",
        );
        assert_eq!(
            got.len(),
            1,
            "le total de la cure n'est pas une dose : {got:?}"
        );
    }

    /// **Une association donne un chiffre par molécule, et chacun porte
    /// son nom** ; et la dose du jour dit en combien de prises elle se
    /// répartit quand la phrase le dit d'un seul chiffre.
    #[test]
    fn a_combination_names_each_figure() {
        let got = read(
            "30 mg/kg par jour de sulfaméthoxazole et 6 mg/kg par jour de \
             triméthoprime, en 2 prises",
        );
        assert_eq!(got[0].of.as_deref(), Some("sulfaméthoxazole"));
        assert_eq!(got[1].of.as_deref(), Some("triméthoprime"));
        assert_eq!(got[1].takes, Some(2));
        let got = read("80 mg/kg par jour exprimés en amoxicilline, en 3 prises");
        assert_eq!(got[0].of.as_deref(), Some("amoxicilline"));
        // Ce qui n'est pas une molécule n'en devient pas une.
        let got = read("50 mg/kg de poids corporel par jour");
        assert_eq!(got[0].of, None);
        // Un choix laissé au prescripteur n'est pas un chiffre.
        let got = read("50 mg/kg par jour en 2 ou 3 prises");
        assert_eq!(got[0].takes, None);
        let got = read("50 mg/kg par jour en trois prises");
        assert_eq!(got[0].takes, Some(3));
    }

    /// **Les lignes par indication répondent là où la carte se tait.**
    ///
    /// La question qui a fait naître l'outil était « le Bactrim, pour un
    /// enfant » — et la posologie de la carte n'en dit rien, pendant que
    /// sa ligne « Infections de l'enfant » écrit trente milligrammes par
    /// kilo de sulfaméthoxazole. Confrontée à toutes les lignes livrées.
    #[test]
    fn the_indication_lines_answer_what_the_card_does_not() {
        let mut total = 0usize;
        for (name, indication, poso, _) in crate::db::STARTER_POSOLOGIES {
            for g in read(poso) {
                total += 1;
                assert!(
                    g.low > 0.0 && g.low <= 200.0,
                    "{name} / {indication} : {} mg/kg",
                    g.low
                );
                if let Some(h) = g.high {
                    assert!(h > g.low, "{name} / {indication}");
                }
            }
        }
        assert!(
            total >= 60,
            "{total} doses au poids dans les lignes livrées"
        );
        let bactrim: Vec<PerKilo> = crate::db::STARTER_POSOLOGIES
            .iter()
            .filter(|(n, i, _, _)| *n == "Bactrim" && i.contains("enfant"))
            .flat_map(|(_, _, p, _)| read(p))
            .collect();
        assert_eq!(bactrim.len(), 2, "{bactrim:?}");
        assert_eq!((bactrim[0].low, bactrim[0].cadence), (30.0, Cadence::Jour));
        assert_eq!(bactrim[0].of.as_deref(), Some("sulfaméthoxazole"));
        assert_eq!(bactrim[1].of.as_deref(), Some("triméthoprime"));
    }

    /// **Les lignes d'abord, et ce qu'elles donnent n'est pas répété.**
    #[test]
    fn the_lines_come_first_and_the_card_adds_only_what_they_lack() {
        let got = gather(
            "Enfant : 50 mg/kg par jour. Méningite : 100 mg/kg par jour.",
            &[("Angine de l'enfant", "50 mg/kg par jour en 2 prises")],
        );
        assert_eq!(got.len(), 2, "{got:?}");
        assert_eq!(got[0].indication.as_deref(), Some("Angine de l'enfant"));
        assert_eq!(got[1].indication, None);
        assert_eq!(got[1].dose.low, 100.0);
        assert!(gather("", &[]).is_empty());
    }

    /// **Jamais zéro milligramme**, et pas de fausse précision.
    #[test]
    fn a_computed_dose_is_never_written_zero() {
        assert_eq!(milligrams(480.0), "480");
        assert_eq!(milligrams(479.6), "480");
        assert_eq!(milligrams(3.2), "3,2");
        assert_eq!(milligrams(2.0), "2");
        assert_eq!(milligrams(0.4), "0,4");
        assert_ne!(milligrams(0.004), "0");
        assert!(milligrams(0.004).starts_with("0,00"));
    }
}
