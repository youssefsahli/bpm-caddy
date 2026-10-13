//! **Le registre FINESS des officines**, livré : ce qu'une officine
//! déclare sur la carte (`repertoire`) se compare à ce que le registre
//! public écrit sous le même numéro.
//!
//! Un FINESS n'est jamais une preuve — n'importe qui peut écrire celui
//! d'une autre —, mais il dit ce qui ne colle pas : un numéro que le
//! registre ignore, une officine qui dit être à Lille quand le registre
//! la met à Nice, un nom qui n'est pas celui du registre. La carte le
//! montre ; elle ne refuse rien.
//!
//! Les données sont publiques (« FINESS Extraction du Fichier des
//! établissements », data.gouv.fr, Licence Ouverte), réduites aux
//! pharmacies d'officine et décrites dans `assets/geo/SOURCES.md`, avec
//! la façon de les refaire.
//!
//! Pur, testé : le fichier est lu une fois, à la première question.

use crate::geo::{folded, Commune};
use std::sync::OnceLock;

/// Le jour de l'extraction livrée, ISO — ce que « absent du registre »
/// veut dire : absent *à cette date*.
pub const AS_OF: &str = "2026-05-04";

/// Une officine au registre.
#[derive(Debug, PartialEq, Eq)]
pub struct Officine {
    pub finess: &'static str,
    pub nom: &'static str,
    pub code_postal: &'static str,
    pub ville: &'static str,
    /// La rue, telle que le registre l'écrit (le type de voie en entier).
    pub adresse: &'static str,
    /// Le téléphone, chiffres seuls.
    pub telephone: &'static str,
}

/// Les officines du registre, dans l'ordre de leur numéro.
pub fn register() -> &'static [Officine] {
    static REGISTER: OnceLock<Vec<Officine>> = OnceLock::new();
    REGISTER.get_or_init(|| {
        let mut all: Vec<Officine> = include_str!("../assets/geo/officines.tsv")
            .lines()
            .filter_map(|line| {
                let mut c = line.split('\t');
                Some(Officine {
                    finess: c.next()?,
                    nom: c.next()?,
                    code_postal: c.next()?,
                    ville: c.next()?,
                    adresse: c.next().unwrap_or(""),
                    telephone: c.next().unwrap_or(""),
                })
            })
            .collect();
        // Le fichier est trié ; un fichier refait à la main pourrait ne
        // plus l'être, et la recherche dichotomique se tairait alors.
        all.sort_by(|a, b| a.finess.cmp(b.finess));
        all
    })
}

/// L'officine que le registre écrit sous ce numéro.
pub fn lookup(finess: &str) -> Option<&'static Officine> {
    let finess = finess.trim().to_ascii_uppercase();
    let all = register();
    all.binary_search_by(|o| o.finess.cmp(finess.as_str()))
        .ok()
        .map(|i| &all[i])
}

/// **Retrouver une officine au registre** par ce qu'on en sait : un
/// numéro FINESS par son début, ou des mots — du nom, de la ville, du
/// code postal — qui y sont tous. Au plus `most`, dans l'ordre du
/// registre.
pub fn find(query: &str, most: usize) -> Vec<&'static Officine> {
    let q = folded(query);
    if q.chars().count() < 3 {
        return Vec::new();
    }
    let upper = query.trim().to_ascii_uppercase();
    let by_number = || -> Vec<&'static Officine> {
        if upper.len() > 9 || !upper.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Vec::new();
        }
        register()
            .iter()
            .filter(|o| o.finess.starts_with(upper.as_str()))
            .take(most)
            .collect()
    };
    let words: Vec<String> = town(&q).split(' ').map(str::to_owned).collect();
    let by_words = || -> Vec<&'static Officine> {
        register()
            .iter()
            .zip(haystack())
            .filter(|(_, text)| {
                words
                    .iter()
                    .all(|w| text.split(' ').any(|t| t.starts_with(w.as_str())))
            })
            .map(|(o, _)| o)
            .take(most)
            .collect()
    };
    // Cinq chiffres au plus : un code postal (ou son début) d'abord —
    // « 06000 » est Nice, pas les numéros FINESS qui commencent ainsi.
    let postcode = upper.len() <= 5 && upper.chars().all(|c| c.is_ascii_digit());
    let (first, then): (
        &dyn Fn() -> Vec<&'static Officine>,
        &dyn Fn() -> Vec<&'static Officine>,
    ) = if postcode {
        (&by_words, &by_number)
    } else {
        (&by_number, &by_words)
    };
    let hits = first();
    if hits.is_empty() {
        then()
    } else {
        hits
    }
}

/// Ce que [`find`] lit de chaque officine — son nom, sa ville et son
/// code postal, repliés —, une fois pour toutes, dans l'ordre du registre.
fn haystack() -> &'static [String] {
    static TEXT: OnceLock<Vec<String>> = OnceLock::new();
    TEXT.get_or_init(|| {
        register()
            .iter()
            .map(|o| town(&format!("{} {} {}", o.nom, o.ville, o.code_postal)))
            .collect()
    })
}

/// **Un texte du registre, écrit comme on l'écrit** : le registre est
/// en capitales ; « PHARMACIE DU CENTRE » devient « Pharmacie du
/// Centre », « 9 rue ALEXANDRE BERARD » « 9 rue Alexandre Berard ». Les
/// petits mots et les types de voie restent en minuscules (sauf en
/// tête), les sigles de société en capitales. Les accents que le
/// registre n'a pas écrits ne se devinent pas.
pub fn pretty(text: &str) -> String {
    const SMALL: [&str; 47] = [
        "de",
        "du",
        "des",
        "la",
        "le",
        "les",
        "et",
        "en",
        "sur",
        "sous",
        "aux",
        "au",
        "a",
        "l",
        "d",
        "rue",
        "avenue",
        "place",
        "boulevard",
        "route",
        "allée",
        "cours",
        "chemin",
        "quai",
        "impasse",
        "square",
        "lotissement",
        "résidence",
        "esplanade",
        "faubourg",
        "passage",
        "promenade",
        "bis",
        "ter",
        "quater",
        "lieu",
        "dit",
        "commercial",
        "rond",
        "point",
        "voie",
        "chaussée",
        "galerie",
        "cité",
        "montée",
        "hameau",
        "traverse",
    ];
    const UPPER: [&str; 22] = [
        "SELARL", "SELAS", "SELAFA", "SEL", "SAS", "SARL", "EURL", "SNC", "SPFPL", "SELEURL",
        "SELURL", "SELASU", "SASU", "ZAC", "ZI", "ZA", "ZUP", "RN", "RD", "CD", "CC", "RER",
    ];
    // Henri IV, Napoléon III, le XVe corps.
    const ROMAN: [&str; 20] = [
        "II", "III", "IV", "VI", "VII", "VIII", "IX", "XI", "XII", "XIII", "XIV", "XV", "XVI",
        "XVII", "XVIII", "XIX", "XX", "XXI", "XXII", "XXIII",
    ];
    let mut out = String::with_capacity(text.len());
    let mut first = true;
    let mut word = String::new();
    // `dotted` : le mot est suivi d'un point — une initiale, « A. Martin »,
    // jamais la préposition « à ».
    let flush = |word: &mut String, out: &mut String, first: &mut bool, dotted: bool| {
        if word.is_empty() {
            return;
        }
        let lower = word.to_lowercase();
        let upper = word.to_uppercase();
        let shouting = word.chars().any(char::is_uppercase);
        if shouting && (UPPER.contains(&upper.as_str()) || ROMAN.contains(&upper.as_str())) {
            out.push_str(&upper);
        } else if !*first && !dotted && SMALL.contains(&lower.as_str()) {
            out.push_str(&lower);
        } else {
            let mut c = lower.chars();
            if let Some(h) = c.next() {
                out.extend(h.to_uppercase());
                out.push_str(c.as_str());
            }
        }
        *first = false;
        word.clear();
    };
    for c in text.chars() {
        if c.is_alphanumeric() {
            word.push(c);
        } else {
            flush(&mut word, &mut out, &mut first, c == '.');
            out.push(c);
        }
    }
    flush(&mut word, &mut out, &mut first, false);
    out
}

/// Un téléphone de dix chiffres écrit par paires ; tout autre reste tel
/// quel.
pub fn phone(digits: &str) -> String {
    if digits.len() == 10 && digits.chars().all(|c| c.is_ascii_digit()) {
        let pairs: Vec<&str> = (0..5)
            .filter_map(|i| digits.get(i * 2..i * 2 + 2))
            .collect();
        pairs.join(" ")
    } else {
        digits.to_owned()
    }
}

/// L'adresse d'une officine du registre, en une ligne : la rue, puis le
/// code postal et la ville — la ville en dernier, que l'annonce sur le
/// réseau local lit (`network::town_of`).
pub fn address_line(o: &Officine) -> String {
    // « BORDEAUX CEDEX » : l'acheminement d'un courrier, pas la ville
    // qu'on annonce.
    let ville = match o.ville.find(" CEDEX") {
        Some(i) => &o.ville[..i],
        None => o.ville,
    };
    let town = format!("{} {}", o.code_postal, pretty(ville));
    if o.adresse.is_empty() {
        town
    } else {
        format!("{}, {town}", pretty(o.adresse))
    }
}

/// Une ville comme le registre et la liste des communes la comparent :
/// repliée, « ST » et « STE » écrits en entier, sans le « CEDEX » d'un
/// acheminement.
fn town(text: &str) -> String {
    let f = folded(text);
    let mut out: Vec<&str> = Vec::new();
    for w in f.split(' ') {
        if w == "cedex" {
            break;
        }
        out.push(match w {
            "st" => "saint",
            "ste" => "sainte",
            _ => w,
        });
    }
    out.join(" ")
}

/// Les mots qui disent *qui* est l'officine, sans ceux que toutes
/// portent : « SELARL Pharmacie du Centre » et « Pharmacie du centre »
/// disent la même chose.
fn name_words(text: &str) -> Vec<String> {
    const COMMON: [&str; 32] = [
        "pharmacie",
        "pharmacies",
        "phie",
        "pharma",
        "seleurl",
        "selurl",
        "selasu",
        "sasu",
        "officine",
        "selarl",
        "selas",
        "selafa",
        "sel",
        "sas",
        "sarl",
        "eurl",
        "snc",
        "spfpl",
        "de",
        "du",
        "des",
        "la",
        "le",
        "les",
        "l",
        "d",
        "et",
        "a",
        "au",
        "aux",
        "en",
        "sur",
    ];
    town(text)
        .split(' ')
        .filter(|w| !w.is_empty() && !COMMON.contains(w))
        .map(str::to_owned)
        .collect()
}

/// **Deux noms pour la même officine** : les mots de l'un sont tous
/// dans l'autre. Un nom fait des seuls mots communs (« Pharmacie ») ne
/// dit rien, et ne contredit donc rien.
pub fn same_name(declared: &str, registered: &str) -> bool {
    let a = name_words(declared);
    let b = name_words(registered);
    a.iter().all(|w| b.contains(w)) || b.iter().all(|w| a.contains(w))
}

/// Ce que le registre dit d'un numéro déclaré.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Check {
    /// Le registre ignore ce numéro.
    Absent,
    /// Le registre met cette officine ailleurs que là où elle dit être.
    Elsewhere(&'static Officine),
    /// Au registre, à la commune déclarée ; `renamed` quand le nom n'est
    /// pas celui du registre.
    Here {
        entry: &'static Officine,
        renamed: bool,
    },
}

/// Le registre coupe la ville de l'acheminement à vingt-six caractères :
/// « SORGES ET LIGUEUX EN PERIG ».
const TOWN_CUT: usize = 26;

/// Le département d'un code postal, tel que deux codes se comparent :
/// trois chiffres outre-mer, deux ailleurs (la Corse est « 20 » des deux
/// côtés).
fn dep_of(code_postal: &str) -> &str {
    let n = if code_postal.starts_with("97") { 3 } else { 2 };
    code_postal.get(..n).unwrap_or("")
}

/// La ville du registre et le nom d'une commune disent-ils la même —
/// la ville coupée se lisant comme un début.
fn same_town(ville: &str, commune: &str) -> bool {
    let (v, c) = (town(ville), town(commune));
    if v.is_empty() {
        return false;
    }
    v == c || (ville.trim().chars().count() >= TOWN_CUT && c.starts_with(&v))
}

/// **Un FINESS déclaré, lu au registre** : `None` quand rien n'est
/// déclaré. La commune déclarée colle quand son code postal est celui
/// du registre, ou son nom celui de la ville **dans le même
/// département** — une grande ville a plusieurs codes, mais Thoiry est
/// dans l'Ain, la Savoie et les Yvelines. Aucune commune déclarée ne
/// contredit rien.
pub fn check(finess: &str, code_postal: &str, commune: &str, nom: &str) -> Option<Check> {
    if finess.trim().is_empty() {
        return None;
    }
    let Some(entry) = lookup(finess) else {
        return Some(Check::Absent);
    };
    let code_postal = code_postal.trim();
    let same_place = (code_postal.is_empty() && commune.trim().is_empty())
        || entry.code_postal == code_postal
        || (dep_of(entry.code_postal) == dep_of(code_postal) && same_town(entry.ville, commune));
    Some(if same_place {
        Check::Here {
            entry,
            renamed: !same_name(nom, entry.nom),
        }
    } else {
        Check::Elsewhere(entry)
    })
}

/// La commune de la liste livrée où le registre met une officine : celle
/// de son code postal dont le nom est la ville, ou la seule de ce code.
/// Un code de plusieurs communes dont aucune n'a le nom de la ville — un
/// lieu-dit de La Poste — n'en donne aucune : mieux vaut rien qu'un
/// village voisin.
pub fn commune_of(entry: &Officine) -> Option<&'static Commune> {
    let at: Vec<&'static Commune> = crate::geo::communes()
        .iter()
        .filter(|c| c.postcode == entry.code_postal)
        .collect();
    at.iter()
        .find(|c| same_town(entry.ville, c.nom))
        .copied()
        .or(if at.len() == 1 { Some(at[0]) } else { None })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Rien dans le registre que les faces livrées ne sachent dessiner :
    /// le nom et l'adresse en sont recopiés et imprimés.
    #[test]
    fn the_register_holds_only_drawable_text() {
        for o in register() {
            for v in [o.nom, o.ville, o.adresse, o.telephone] {
                assert!(
                    v.chars().all(|c| (c.is_ascii() && !c.is_ascii_control())
                        || c == '°'
                        || (c.is_alphabetic() && (c as u32) < 0x180)),
                    "{} {v:?}",
                    o.finess
                );
                assert!(!v.contains("\"\""), "{} {v:?}", o.finess);
            }
        }
    }

    /// Le registre livré se lit entier, trié, un numéro par officine.
    #[test]
    fn the_shipped_register_reads_whole() {
        let all = register();
        assert!(all.len() > 19_000, "{}", all.len());
        for w in all.windows(2) {
            assert!(w[0].finess < w[1].finess, "{}", w[0].finess);
        }
        for o in all {
            assert_eq!(o.finess.len(), 9, "{}", o.finess);
            assert_eq!(o.code_postal.len(), 5, "{}", o.finess);
            assert!(!o.nom.is_empty() && !o.ville.is_empty(), "{}", o.finess);
        }
        let first = &all[0];
        assert_eq!(lookup(first.finess), Some(first));
        assert_eq!(lookup(&first.finess.to_lowercase()), Some(first));
        assert_eq!(lookup("000000000"), None);
    }

    /// Presque toutes les officines se posent à une commune de la liste
    /// livrée, et à la bonne : la ville du registre et le nom de la
    /// commune se lisent l'un l'autre.
    #[test]
    fn almost_every_registered_officine_lands_on_its_town() {
        let all = register();
        let mut placed = 0;
        let mut named = 0;
        for o in all {
            if let Some(c) = commune_of(o) {
                placed += 1;
                if town(c.nom) == town(o.ville) {
                    named += 1;
                }
            }
        }
        assert!(placed * 100 >= all.len() * 98, "{placed} / {}", all.len());
        assert!(named * 100 >= all.len() * 90, "{named} / {}", all.len());
    }

    #[test]
    fn register_text_reads_as_written() {
        assert_eq!(pretty("PHARMACIE DU CENTRE"), "Pharmacie du Centre");
        assert_eq!(
            pretty("SELARL PHARMACIE D'AMBERIEU"),
            "SELARL Pharmacie d'Amberieu"
        );
        assert_eq!(
            pretty("9 bis rue ALEXANDRE BERARD"),
            "9 bis rue Alexandre Berard"
        );
        assert_eq!(pretty("LE MERLERAULT"), "Le Merlerault");
        assert_eq!(pretty("SAINT-ETIENNE"), "Saint-Etienne");
        assert_eq!(pretty("8 place HENRI IV"), "8 place Henri IV");
        assert_eq!(pretty("11 place A. LEMARIGNIER"), "11 place A. Lemarignier");
        assert_eq!(pretty("D.BOUDOURESQUE"), "D.Boudouresque");
        assert_eq!(pretty("ZI LES PALUDS, RN 113"), "ZI les Paluds, RN 113");
        assert_eq!(pretty("lieu-dit LES BOURBES"), "Lieu-dit les Bourbes");
        assert_eq!(pretty("1 lieu-dit LES BOURBES"), "1 lieu-dit les Bourbes");
        assert_eq!(phone("0474380226"), "04 74 38 02 26");
        assert_eq!(phone("123"), "123");
        let o = Officine {
            finess: "010000000",
            nom: "PHARMACIE X",
            code_postal: "01500",
            ville: "AMBERIEU EN BUGEY",
            adresse: "9 rue ALEXANDRE BERARD",
            telephone: "",
        };
        assert_eq!(
            address_line(&o),
            "9 rue Alexandre Berard, 01500 Amberieu en Bugey"
        );
        // La ville en dernier : l'annonce sur le réseau local la lit.
        #[cfg(feature = "sync")]
        assert_eq!(
            crate::network::town_of(&address_line(&o)),
            "Amberieu en Bugey"
        );
    }

    /// Une officine se retrouve par son numéro, ou par des débuts de mots
    /// de son nom et de sa ville.
    #[test]
    fn an_officine_is_found_by_number_or_words() {
        let o = register()
            .iter()
            .find(|o| o.ville == "THOIRY" && o.code_postal.starts_with("01"))
            .unwrap();
        assert_eq!(find(o.finess, 5), [o]);
        assert!(find(&o.finess[..7], 50).contains(&o));
        let words = format!("{} thoir", o.nom.split(' ').next_back().unwrap());
        assert!(find(&words, 50).contains(&o), "{words}");
        assert!(find("zz", 5).is_empty());
        // Un code postal se lit comme un code postal, même quand des
        // numéros FINESS commencent par les mêmes chiffres.
        let nice = find("06000", 6);
        assert!(!nice.is_empty());
        assert!(nice.iter().all(|o| o.code_postal == "06000"), "{nice:?}");
        assert!(find("pharmacie", 7).len() == 7);
    }

    #[test]
    fn town_names_compare_across_the_two_spellings() {
        assert_eq!(town("ST PAUL DE VENCE"), town("Saint-Paul-de-Vence"));
        assert_eq!(town("STE VERGE"), town("Sainte-Verge"));
        assert_eq!(town("PARIS CEDEX 13"), town("Paris"));
        assert_ne!(town("SAINT DENIS"), town("Saint-Didier"));
        // Coupée à vingt-six caractères par le registre.
        assert!(same_town(
            "SORGES ET LIGUEUX EN PERIG",
            "Sorges et Ligueux en Périgord"
        ));
        assert!(!same_town("PARIS", "Paris-l'Hôpital"));
    }

    /// Un homonyme dans un autre département n'est pas la ville du
    /// registre ; une commune pas encore choisie ne contredit rien.
    #[test]
    fn a_homonym_elsewhere_is_elsewhere() {
        let o = register()
            .iter()
            .find(|o| o.ville == "THOIRY" && o.code_postal.starts_with("01"))
            .expect("une officine à Thoiry (Ain)");
        assert_eq!(
            check(o.finess, "78770", "Thoiry", o.nom),
            Some(Check::Elsewhere(o))
        );
        assert!(matches!(
            check(o.finess, "01710", "Thoiry", o.nom),
            Some(Check::Here { .. })
        ));
        assert!(matches!(
            check(o.finess, "", "", o.nom),
            Some(Check::Here { .. })
        ));
    }

    /// Un code de plusieurs communes, dont aucune n'a le nom de la ville
    /// du registre, ne propose pas un village voisin.
    #[test]
    fn a_locality_that_is_no_commune_offers_none() {
        let mut per_code = std::collections::HashMap::new();
        for c in crate::geo::communes() {
            *per_code.entry(c.postcode).or_insert(0) += 1;
        }
        for o in register() {
            if let Some(c) = commune_of(o) {
                let alone = per_code.get(o.code_postal) == Some(&1);
                assert!(
                    alone || same_town(o.ville, c.nom),
                    "{} {}",
                    o.finess,
                    o.ville
                );
            }
        }
    }

    #[test]
    fn names_agree_without_the_words_every_officine_carries() {
        assert!(same_name(
            "Pharmacie du Centre",
            "SELARL PHARMACIE DU CENTRE"
        ));
        assert!(same_name("Pharmacie Martin", "PHARMACIE MARTIN ET DUPONT"));
        assert!(same_name("Pharmacie", "PHARMACIE DE LA GARE"));
        assert!(same_name(
            "Grande pharmacie du Centre",
            "SELEURL GRANDE PHARMACIE DU CENTRE"
        ));
        assert!(same_name("Pharmacie du Centre", "SELEURL PHIE DU CENTRE"));
        assert!(!same_name("Pharmacie Martin", "PHARMACIE DU MARCHE"));
        assert!(!same_name("Grande pharmacie de Nice", "PHARMACIE DE LILLE"));
    }

    /// Les trois réponses, sur une officine réelle du registre.
    #[test]
    fn a_declared_number_reads_against_the_register() {
        let o = register()
            .iter()
            .find(|o| !name_words(o.nom).is_empty())
            .expect("une officine nommée");
        assert_eq!(check("", o.code_postal, o.ville, o.nom), None);
        assert_eq!(
            check("ZZZZZZZZZ", "75001", "Paris", "X"),
            Some(Check::Absent)
        );
        assert_eq!(
            check(o.finess, o.code_postal, o.ville, o.nom),
            Some(Check::Here {
                entry: o,
                renamed: false
            })
        );
        // Une autre commune, mais la même ville : un autre code postal
        // d'une grande ville ne la déplace pas.
        let other = format!("{}999", &o.code_postal[..2]);
        assert_eq!(
            check(o.finess, &other, o.ville, o.nom),
            Some(Check::Here {
                entry: o,
                renamed: false
            })
        );
        assert_eq!(
            check(o.finess, o.code_postal, o.ville, "Officine Zyxwvut"),
            Some(Check::Here {
                entry: o,
                renamed: true
            })
        );
        assert_eq!(
            check(o.finess, "00000", "Nulle-Part", o.nom),
            Some(Check::Elsewhere(o))
        );
    }
}
