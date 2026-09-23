//! Aller chercher l'annuaire des prescripteurs, sur un bouton.
//!
//! # La deuxième requête réseau de l'application, et la dernière
//!
//! Il n'y en avait qu'une : la recherche de mise à jour, dans Options ›
//! À propos, sur un bouton, jamais au démarrage. Celle-ci est la
//! seconde, et elle obéit aux mêmes règles parce que ce sont elles qui
//! font la posture — pas leur nombre.
//!
//! **Sur un bouton.** Rien ne part tout seul : ni au lancement, ni à
//! heure fixe, ni parce qu'un annuaire a trois mois. Une officine qui
//! n'appuie jamais ne verra jamais cette application ouvrir une
//! connexion.
//!
//! **Vers une adresse que l'officine a écrite.** Aucune n'est livrée,
//! et le bouton n'existe pas tant que `[prescribers] source_url` est
//! vide. Embarquer une adresse, c'est décider à la place de l'officine
//! à qui elle parle — et c'est une adresse qui sera morte dans deux
//! ans, avec un bouton qui échouera sans que personne sache pourquoi.
//!
//! **Rien ne monte.** C'est un `GET` : on demande un fichier, on ne
//! raconte rien. Il n'y a pas de dossier, pas de nom d'officine, pas
//! d'identifiant d'installation dans cette requête — et rien à en
//! mettre, puisque la fonction ne reçoit qu'une adresse.
//!
//! **Sur un fil à part.** Une requête qui gèle la fenêtre au comptoir
//! est pire que pas de requête, et les délais sont courts pour la même
//! raison : un serveur muet ne doit pas coûter la matinée.
//!
//! # Ce qui est lu, et ce qui est refusé
//!
//! Ce qui revient est un fichier texte, lu par `prescribers::import`.
//! Deux choses l'arrêtent avant, et les deux se disent :
//!
//! **Une archive n'est pas un fichier texte.** Les jeux publics sont
//! souvent livrés zippés ; importer les octets d'un zip donnerait un
//! annuaire de caractères illisibles. Reconnu à sa signature, refusé, et
//! dit en toutes lettres — « dézippez-le et importez-le » est une
//! consigne, « 0 prescripteur importé » n'en est pas une.
//!
//! **Un fichier plus gros que ce poste ne lira est refusé avant d'être
//! gardé.** Un serveur ne choisit pas la mémoire de la machine qui
//! l'interroge. C'est la règle que `bpm-sync` applique à chaque trame,
//! et elle vaut ici pour la même raison.
//!
//! Pur et testé, sauf la requête elle-même : tout ce qui lit la réponse
//! est une fonction à côté, comme dans `release.rs`.

use std::sync::mpsc::{Receiver, Sender};

/// Le plus gros annuaire que ce poste chargera : quarante mégaoctets.
///
/// Un annuaire national fait quelques dizaines de mégaoctets en texte ;
/// au-delà, ce n'est pas un annuaire, c'est autre chose. Refusé **avant
/// d'être gardé**, et non après.
pub const MAX_BYTES: usize = 40 * 1024 * 1024;

/// Ce qu'une recherche a rendu.
pub enum Fetched {
    /// Le texte de l'annuaire, prêt pour `prescribers::import`.
    Got(String),
    /// Le réseau, le serveur, ou la forme de ce qu'il a rendu.
    Failed(String),
}

/// Une archive se reconnaît à sa signature, jamais à son nom.
///
/// La même règle que les pièces scannées : un fichier est ce que ses
/// octets disent, et un serveur qui rend un zip sous une adresse en
/// `.csv` est une chose qui arrive tous les jours.
pub fn looks_like_archive(bytes: &[u8]) -> bool {
    // PK.. (zip), \x1f\x8b (gzip), 7z, xz, bzip2, rar.
    const SIGNATURES: &[&[u8]] = &[
        b"PK\x03\x04",
        b"PK\x05\x06",
        b"\x1f\x8b",
        b"7z\xbc\xaf",
        b"\xfd7zXZ",
        b"BZh",
        b"Rar!",
    ];
    SIGNATURES.iter().any(|sig| bytes.starts_with(sig))
}

/// Lit le corps de la réponse comme du texte.
///
/// **L'UTF-8 d'abord, puis le latin-1.** Les fichiers publics français
/// sont encore souvent en Windows-1252, et un « é » lu comme de l'UTF-8
/// invalide donnerait « Lefèvre » en « Lef?vre » — dans un annuaire de
/// noms propres, c'est-à-dire un nom faux. Le repli est fait octet par
/// octet, ce qui est exactement la table latin-1, et jamais avec
/// remplacement : un caractère de remplacement dans un nom est une
/// faute qu'on garde.
pub fn read_body(bytes: Vec<u8>) -> Result<String, String> {
    if bytes.is_empty() {
        return Err(crate::strings::tr("annuaire_empty").to_owned());
    }
    if looks_like_archive(&bytes) {
        return Err(crate::strings::tr("annuaire_archive").to_owned());
    }
    match String::from_utf8(bytes) {
        Ok(text) => Ok(text),
        // **Ligne par ligne** : un fichier fait de deux exports collés
        // porte des lignes UTF-8 et des lignes Windows, et relire le tout
        // en Windows pour un seul octet écrivait « LefÃ¨vre » sur chaque
        // nom juste. Seule la ligne qui n'est pas de l'UTF-8 se replie.
        Err(e) => Ok(e
            .into_bytes()
            .split_inclusive(|b| *b == b'\n')
            .map(|line| match std::str::from_utf8(line) {
                Ok(text) => text.to_owned(),
                Err(_) => line.iter().copied().map(cp1252).collect(),
            })
            .collect()),
    }
}

/// Un octet de Windows-1252. Le latin-1 et lui s'accordent partout sauf
/// de 0x80 à 0x9F, où le latin-1 met des caractères de contrôle et
/// Windows-1252 l'apostrophe typographique de « D’ALMEIDA », le « œ »,
/// les tirets, l'euro : lus en latin-1, ils entraient dans l'annuaire
/// comme des caractères invisibles.
fn cp1252(b: u8) -> char {
    const HIGH: [char; 32] = [
        '€', '\u{81}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{8d}', 'Ž',
        '\u{8f}', '\u{90}', '‘', '’', '“', '”', '•', '–', '—', '˜', '™', 'š', '›', 'œ', '\u{9d}',
        'ž', 'Ÿ',
    ];
    match b {
        0x80..=0x9F => HIGH[usize::from(b - 0x80)],
        _ => char::from(b),
    }
}

/// Va chercher l'annuaire, sur un fil à part.
///
/// Rend tout de suite le bout récepteur : l'interface continue de
/// peindre et lit la réponse quand elle arrive — le même agencement que
/// la recherche de mise à jour, et pour la même raison.
pub fn fetch_async(url: String) -> Receiver<Fetched> {
    let (tx, rx): (Sender<Fetched>, Receiver<Fetched>) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(fetch(&url));
    });
    rx
}

/// La seule requête. Tout ce qui lit la réponse est à côté, et testé.
fn fetch(url: &str) -> Fetched {
    // Une adresse qui n'est pas du `https` est refusée ici et non par
    // le serveur : un annuaire de noms propres qui traverse en clair
    // est un annuaire que le réseau de l'officine peut lire.
    if !url.trim().starts_with("https://") {
        return Fetched::Failed(crate::strings::tr("annuaire_not_https").to_owned());
    }
    let agent = ureq::AgentBuilder::new()
        .https_only(true)
        .timeout_connect(std::time::Duration::from_secs(10))
        .timeout_read(std::time::Duration::from_secs(120))
        .user_agent("bpm-caddy")
        .build();
    let body = match agent.get(url.trim()).call() {
        Ok(r) => r.into_reader(),
        Err(e) => return Fetched::Failed(e.to_string()),
    };
    // **Borné avant d'être gardé** : `take` lit au plus ce que ce poste
    // accepte, plus un octet — de quoi savoir qu'il y en avait plus,
    // sans avoir chargé le reste.
    let mut bytes = Vec::new();
    if let Err(e) = std::io::Read::read_to_end(
        &mut std::io::Read::take(body, MAX_BYTES as u64 + 1),
        &mut bytes,
    ) {
        return Fetched::Failed(e.to_string());
    }
    if bytes.len() > MAX_BYTES {
        return Fetched::Failed(crate::strings::tr("annuaire_too_big").to_owned());
    }
    match read_body(bytes) {
        Ok(text) => Fetched::Got(text),
        Err(e) => Fetched::Failed(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Une archive se reconnaît à ses octets, jamais à son nom.
    ///
    /// Les jeux publics sont souvent livrés zippés, et un serveur qui
    /// rend un zip sous une adresse en `.csv` arrive tous les jours.
    /// Importer ses octets donnerait un annuaire de caractères
    /// illisibles — et « 0 prescripteur importé » n'est pas une
    /// consigne.
    #[test]
    fn an_archive_is_known_by_its_bytes_and_refused() {
        assert!(looks_like_archive(b"PK\x03\x04rest"));
        assert!(looks_like_archive(b"\x1f\x8b\x08gzip"));
        assert!(looks_like_archive(b"BZh9"));
        assert!(!looks_like_archive(b"Nom;Prenom\nMorel;Jean\n"));
        assert!(!looks_like_archive(b""));

        let err = read_body(b"PK\x03\x04....".to_vec()).expect_err("refuse");
        assert!(!err.is_empty());
        // Et un corps vide se dit aussi : un fichier de zéro octet
        // n'est pas un annuaire vide, c'est une requête qui a échoué
        // sans le dire.
        assert!(read_body(Vec::new()).is_err());
    }

    /// **L'UTF-8 d'abord, puis le latin-1.**
    ///
    /// Les fichiers publics français sont encore souvent en
    /// Windows-1252. Un « è » lu comme de l'UTF-8 invalide et remplacé
    /// donnerait « Lef?vre » — dans un annuaire de noms propres,
    /// c'est-à-dire un nom faux, et un nom faux ne se rattrape pas.
    #[test]
    fn a_name_is_read_in_the_encoding_the_file_actually_uses() {
        // De l'UTF-8 : lu tel quel.
        let utf8 = "Nom;Ville\nLefèvre;Sète\n";
        assert_eq!(read_body(utf8.as_bytes().to_vec()).unwrap(), utf8);

        // Du latin-1 : les mêmes noms, un octet par accent.
        let latin1: Vec<u8> = {
            let mut v = b"Nom;Ville\nLef".to_vec();
            v.push(0xE8); // è
            v.extend_from_slice(b"vre;S");
            v.push(0xE8);
            v.extend_from_slice(b"te\n");
            v
        };
        let read = read_body(latin1).unwrap();
        assert!(read.contains("Lefèvre"), "« {read} »");
        assert!(read.contains("Sète"), "« {read} »");
        // Et jamais de caractère de remplacement : une faute gardée
        // dans un nom est une faute qu'on imprimera.
        assert!(!read.contains('\u{fffd}'));
    }

    /// Ce que le module **ne fait pas**, tenu par son propre texte.
    ///
    /// Rien ne part tout seul, rien ne monte, et aucune adresse n'est
    /// livrée : trois règles qui ne se voient pas dans un type et qui
    /// sont toute la différence entre un bouton et une balise.
    #[test]
    fn nothing_leaves_on_its_own_and_no_address_ships() {
        let text = include_str!("annuaire.rs");
        let code = text.split("mod tests").next().unwrap();
        let body: String = code
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        // Une seule requête, et c'est un GET : rien ne monte.
        assert_eq!(body.matches(".call()").count(), 1);
        // `.send(` tout court n'est pas dans la liste : c'est aussi le
        // verbe du canal qui rend la réponse au fil de l'interface, et
        // un garde qui se déclenche sur celui-là refuserait la seule
        // façon correcte d'écrire cette fonction.
        for sending in [
            ".post(",
            ".put(",
            ".send_json",
            ".send_string",
            ".send_bytes",
        ] {
            assert!(!body.contains(sending), "« {sending} » : rien ne monte");
        }
        // Aucune adresse livrée : le seul « https:// » du corps est
        // celui du contrôle qui refuse tout le reste.
        assert_eq!(body.matches("https://").count(), 1);
        // Et rien qui parte tout seul : pas d'horloge, pas de minuterie.
        for auto in ["Instant", "interval", "every", "schedule"] {
            assert!(!body.contains(auto), "« {auto} »");
        }
    }

    /// Une adresse qui n'est pas du `https` est refusée **ici**.
    ///
    /// Un annuaire de noms propres qui traverse en clair est un
    /// annuaire que le réseau de l'officine peut lire, et attendre que
    /// le serveur refuse serait avoir déjà parlé.
    #[test]
    fn an_address_that_is_not_https_never_leaves_this_machine() {
        for refused in [
            "http://annuaire.exemple.fr/ps.csv",
            "ftp://ailleurs/ps.csv",
            "  ",
            "annuaire.exemple.fr",
        ] {
            match fetch(refused) {
                Fetched::Failed(e) => assert!(!e.is_empty(), "{refused}"),
                Fetched::Got(_) => panic!("{refused} est parti"),
            }
        }
    }

    /// **Windows-1252, et non latin-1** : l'apostrophe de « D’ALMEIDA »
    /// et le « œ » vivent entre 0x80 et 0x9F.
    #[test]
    fn a_windows_file_keeps_its_apostrophes() {
        let text = read_body(b"D\x92ALMEIDA;C\x9cur;Lef\xe8vre".to_vec()).unwrap();
        assert_eq!(text, "D’ALMEIDA;Cœur;Lefèvre");
        // Deux exports collés : la ligne UTF-8 reste juste, seule la
        // ligne Windows se replie.
        let mut mixed = "Lefèvre;Sète\n".as_bytes().to_vec();
        mixed.extend_from_slice(b"D\x92ALMEIDA\n");
        assert_eq!(read_body(mixed).unwrap(), "Lefèvre;Sète\nD’ALMEIDA\n");
    }
}
