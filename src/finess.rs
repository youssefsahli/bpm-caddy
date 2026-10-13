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
    const COMMON: [&str; 26] = [
        "pharmacie",
        "pharmacies",
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

/// **Un FINESS déclaré, lu au registre** : `None` quand rien n'est
/// déclaré. La commune déclarée colle quand son code postal est celui
/// du registre, ou son nom celui de la ville — une grande ville a
/// plusieurs codes.
pub fn check(finess: &str, code_postal: &str, commune: &str, nom: &str) -> Option<Check> {
    if finess.trim().is_empty() {
        return None;
    }
    let Some(entry) = lookup(finess) else {
        return Some(Check::Absent);
    };
    let same_place = entry.code_postal == code_postal.trim() || town(entry.ville) == town(commune);
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
/// de son code postal dont le nom est la ville, ou à défaut la première
/// de ce code.
pub fn commune_of(entry: &Officine) -> Option<&'static Commune> {
    let ville = town(entry.ville);
    let mut first = None;
    for c in crate::geo::communes()
        .iter()
        .filter(|c| c.postcode == entry.code_postal)
    {
        if town(c.nom) == ville {
            return Some(c);
        }
        first = first.or(Some(c));
    }
    first
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(placed * 100 >= all.len() * 99, "{placed} / {}", all.len());
        assert!(named * 100 >= all.len() * 90, "{named} / {}", all.len());
    }

    #[test]
    fn town_names_compare_across_the_two_spellings() {
        assert_eq!(town("ST PAUL DE VENCE"), town("Saint-Paul-de-Vence"));
        assert_eq!(town("STE VERGE"), town("Sainte-Verge"));
        assert_eq!(town("PARIS CEDEX 13"), town("Paris"));
        assert_ne!(town("SAINT DENIS"), town("Saint-Didier"));
    }

    #[test]
    fn names_agree_without_the_words_every_officine_carries() {
        assert!(same_name(
            "Pharmacie du Centre",
            "SELARL PHARMACIE DU CENTRE"
        ));
        assert!(same_name("Pharmacie Martin", "PHARMACIE MARTIN ET DUPONT"));
        assert!(same_name("Pharmacie", "PHARMACIE DE LA GARE"));
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
        assert_eq!(
            check(o.finess, "00000", o.ville, o.nom),
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
