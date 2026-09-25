//! BPM-Caddy as a library: every module the application is made of.
//!
//! **Two programmes, one reading of the base.** `bpm-caddy` (the counter
//! application) and `bpm-audit` (the officine's audit window) are two
//! binaries over this one crate: a second programme with its own copy of
//! the schema would be a second writing of it, and two writings of a
//! schema end up differing — the day they do, it is the one nobody runs
//! that looks right.

pub mod agenda;
pub mod annuaire;
pub mod app;
pub mod audit;
pub mod audit_window;
pub mod biology;
pub mod bulletin;
pub mod caisse;
pub mod classes;
pub mod codebar;
pub mod codex;
pub mod conciliation;
pub mod config;
pub mod content;
pub mod crush;
pub mod cyp;
pub mod date;
pub mod db;
pub mod dosing;
pub mod elderly;
pub mod entretien;
pub mod facets;
pub mod favorites;
pub mod fuzzy;
pub mod graph;
pub mod gravidity;
pub mod hepatic;
pub mod insulin;
pub mod intake;
pub mod location;
pub mod maintenance;
pub mod messages;
#[cfg(feature = "sync")]
pub mod netmap;
#[cfg(feature = "sync")]
pub mod network;
pub mod ordonnance;
pub mod ordonnancier;
pub mod pdf;
pub mod pk;
pub mod planning;
#[cfg(feature = "sync")]
pub mod postes;
pub mod prescribers;
pub mod release;
pub mod renal;
pub mod renewal;
pub mod replica;
pub mod revue;
pub mod ruptures;
pub mod scans;
pub mod script;
pub mod selfcheck;
pub mod strings;
pub mod surveillance;
pub mod tables;
pub mod telemetry;
pub mod timeline;
pub mod trod;
pub mod vaccines;
pub mod vaccsheet;
pub mod versions;
pub mod vigilance;
pub mod vitale;
pub mod winscard;

/// L'empreinte d'une officine, en cinq groupes — ce qu'on se lit au
/// téléphone. Sans la synchronisation, qui sait la calculer, le début de
/// son identité.
pub fn peer_groups(device: &str) -> String {
    #[cfg(feature = "sync")]
    {
        network::peer_groups(device)
    }
    #[cfg(not(feature = "sync"))]
    {
        let head: Vec<char> = device.chars().take(20).collect();
        head.chunks(4)
            .map(|c| c.iter().collect::<String>())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// **Un nom qu'une officine se donne, et qu'une autre porte déjà**, se lit
/// avec son empreinte : sans cela, une officine appairée hostile se nommait
/// comme une autre, et ses envois passaient pour les siens partout où
/// l'opérateur ne les avait pas nommées lui-même — c'est-à-dire par défaut.
/// `names` : (empreinte, nom) des autres officines, et de celle-ci (sous
/// une empreinte vide).
///
/// Comparés sur leur squelette — lettres et chiffres, en minuscules — pour
/// qu'une espace doublée, un tiret ou un caractère invisible ne suffise pas.
/// Un nom qui porte une parenthèse (une fausse empreinte) ou une lettre hors
/// de l'alphabet latin (un « Р » cyrillique) prend toujours la sienne.
pub fn disambiguated(name: &str, device: &str, names: &[(String, String)]) -> String {
    let skeleton = |s: &str| -> String {
        s.chars()
            .filter(|c| c.is_alphanumeric())
            .flat_map(char::to_lowercase)
            .collect()
    };
    let own = skeleton(name);
    let latin = |c: char| {
        c.is_ascii() || ('\u{c0}'..='\u{24f}').contains(&c) || matches!(c, '\u{2018}' | '\u{2019}')
    };
    let suspicious = name.contains('(') || !name.trim().chars().all(latin);
    let taken = names
        .iter()
        .any(|(d, n)| d != device && !own.is_empty() && skeleton(n) == own);
    if taken || suspicious {
        let groups = peer_groups(device);
        let short: String = groups.split(' ').take(2).collect::<Vec<_>>().join(" ");
        format!("{} ({short})", name.trim())
    } else {
        name.trim().to_owned()
    }
}

/// Les noms que portent les officines appairées — donnés ici, ou signés —
/// et celui de cette officine : une appairée ne signe pas en son nom.
pub fn peer_names(peers: &[db::NetPeerRow], own: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = peers
        .iter()
        .flat_map(|p| {
            [p.name.trim(), p.seen_as.trim()]
                .into_iter()
                .filter(|n| !n.is_empty())
                .map(|n| (p.device.clone(), n.to_owned()))
                .collect::<Vec<_>>()
        })
        .collect();
    if !own.trim().is_empty() {
        out.push((String::new(), own.trim().to_owned()));
    }
    out
}

/// Le nom qu'une officine appairée signe, tel qu'on le lit : avec son
/// empreinte s'il est déjà pris.
pub fn peer_claim(device: &str, peers: &[db::NetPeerRow], own: &str) -> Option<String> {
    peers
        .iter()
        .find(|p| p.device == device)
        .filter(|p| !p.seen_as.trim().is_empty())
        .map(|p| disambiguated(&p.seen_as, device, &peer_names(peers, own)))
}

/// Le nom sous lequel on lit une officine appairée : celui donné ici, sinon
/// celui qu'elle signe (`peer_claim`), sinon son empreinte. `own` : le nom
/// de cette officine.
pub fn peer_label(device: &str, peers: &[db::NetPeerRow], own: &str) -> String {
    match peers.iter().find(|p| p.device == device) {
        Some(p) if !p.name.trim().is_empty() => p.name.trim().to_owned(),
        _ => peer_claim(device, peers, own).unwrap_or_else(|| peer_groups(device)),
    }
}

#[cfg(test)]
mod disambiguated_tests {
    #[test]
    fn a_name_another_officine_bears_reads_with_the_fingerprint() {
        let names = vec![
            ("aa11".to_owned(), "Pharmacie B".to_owned()),
            ("bb22".to_owned(), "Pharmacie C".to_owned()),
        ];
        assert_eq!(
            super::disambiguated(" Pharmacie C ", "bb22", &names),
            "Pharmacie C"
        );
        let forged = super::disambiguated("pharmacie b", "cc33", &names);
        assert!(forged.starts_with("pharmacie b ("), "{forged}");
        assert_eq!(
            super::disambiguated("Pharmacie D", "cc33", &names),
            "Pharmacie D"
        );
        // Espaces, tirets, invisibles : le même squelette.
        for forged in ["Pharmacie  B", "Pharmacie-B", "Pharma\u{200b}cie B"] {
            assert_ne!(super::disambiguated(forged, "cc33", &names), forged.trim());
        }
        // Un « Р » cyrillique, une fausse empreinte.
        for forged in ["\u{420}harmacie E", "Pharmacie E (1A2B 3C4D)"] {
            assert_ne!(super::disambiguated(forged, "cc33", &names), forged);
        }
        // Accents et apostrophe typographique restent nets.
        let name = "Pharmacie de l\u{2019}\u{c9}glise";
        assert_eq!(super::disambiguated(name, "cc33", &names), name);
        // Le nom de cette officine-ci.
        let mine = vec![(String::new(), "Pharmacie Ici".to_owned())];
        assert_ne!(
            super::disambiguated("Pharmacie Ici", "cc33", &mine),
            "Pharmacie Ici"
        );
    }
}
