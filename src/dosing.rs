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
        if let Some((low, high)) = figures_before(hay, start) {
            out.push(PerKilo {
                low,
                high,
                cadence: cadence_after(hay, end),
                sentence: sentence_around(hay, start),
            });
        }
        from = end;
    }
    out
}

/// Le nombre — ou les deux bornes — écrits juste avant « mg/kg ».
///
/// À la virgule : les fiches écrivent « 0,35 mg/kg », et lire trente-cinq
/// là où il y a trente-cinq centièmes est un facteur cent sur un enfant.
fn figures_before(hay: &str, at: usize) -> Option<(f64, Option<f64>)> {
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
    match linked.and_then(number_at_end) {
        Some((first, _)) if first < second => Some((first, Some(second))),
        _ => Some((second, None)),
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

/// Ce que la phrase dit du rythme, juste après l'unité.
fn cadence_after(hay: &str, at: usize) -> Cadence {
    let tail = &hay[at..];
    // Collé à l'unité : « 50 mg/kg/jour ».
    let glued = tail.trim_start_matches('/');
    if glued.len() < tail.len() && (glued.starts_with("jour") || glued.starts_with('j')) {
        return Cadence::Jour;
    }
    // Sinon dans ce qui suit immédiatement. Une fenêtre courte, et
    // volontairement : « soit 60 mg/kg par 24 heures » se trouve à
    // trente caractères de la dose d'avant, et une fenêtre large ferait
    // porter à la première le rythme de la seconde.
    let window: String = tail.chars().take(40).collect::<String>().to_lowercase();
    // La prise d'abord : « par prise toutes les 6 heures » contient les
    // deux, et c'est bien une dose par prise.
    if window.contains("par prise") || window.contains("toutes les") {
        return Cadence::Prise;
    }
    if window.contains("par jour")
        || window.contains("par 24 heures")
        || window.contains("par vingt-quatre")
        || window.contains("quotidien")
    {
        return Cadence::Jour;
    }
    Cadence::NonDite
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
        // Et celle qui ne répond pas : la fiche du Bactrim n'écrit pas
        // de dose au poids. L'écran le dit au lieu de proposer un
        // chiffre qui n'est pas le sien — c'est la même règle que
        // « le silence n'est pas une autorisation », appliquée à un
        // nombre, et c'est aussi ce qui dit à l'officine quelle fiche
        // compléter.
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
}
