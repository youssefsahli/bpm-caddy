//! **La passerelle des patients** (`src/passerelle.rs`) : à lancer sur un
//! serveur, derrière un proxy qui porte le certificat du nom de domaine.
//!
//! ```text
//! bpm-passerelle [--web 127.0.0.1:8746] [--postes 0.0.0.0:7747]
//!                [--journal /var/log/bpm-passerelle] [--garde 92]
//!                [--origine hôte]…
//! ```
//!
//! Avec Caddy, le proxy tient en deux lignes :
//!
//! ```text
//! messagerie.example.fr {
//!     reverse_proxy 127.0.0.1:8746
//! }
//! ```
//!
//! Elle ne garde aucun message et n'en lit aucun. Avec `--journal`, elle
//! écrit une ligne par connexion (l'heure, l'adresse, la boîte, l'issue),
//! un fichier par jour, effacé au bout de `--garde` jours (183 au plus).
//! Elle ne sert pas la page des patients, publiée à part : seules les
//! pages de cette origine l'appellent — `--origine` en ajoute une. Le
//! port des postes doit être ouvert vers Internet ; celui des pages, non.

use std::net::SocketAddr;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use bpm_caddy::passerelle::{self, Options};

const USAGE: &str = "usage : bpm-passerelle [--web 127.0.0.1:8746] [--postes 0.0.0.0:7747] \
                     [--journal dossier] [--garde jours] [--origine hôte]…";

fn main() {
    let mut web = passerelle::DEFAULT_WEB.to_owned();
    let mut posts = format!("0.0.0.0:{}", passerelle::DEFAULT_POSTS_PORT);
    let (mut journal, mut keep, mut origins) = (None, passerelle::KEEP_DEFAULT, Vec::new());
    let mut args = std::env::args().skip(1);
    let usage = || -> ! {
        eprintln!("{USAGE}");
        std::process::exit(2);
    };
    while let Some(a) = args.next() {
        match (a.as_str(), args.next()) {
            ("--web", Some(v)) => web = v,
            ("--postes", Some(v)) => posts = v,
            ("--journal", Some(v)) => journal = Some(std::path::PathBuf::from(v)),
            ("--garde", Some(v)) => keep = v.parse().unwrap_or_else(|_| usage()),
            ("--origine", Some(v)) => origins.push(v),
            _ => usage(),
        }
    }
    let parse = |s: &str| -> SocketAddr {
        s.parse().unwrap_or_else(|_| {
            eprintln!("adresse illisible : {s}");
            std::process::exit(2);
        })
    };
    let mut options = Options::new(parse(&web), parse(&posts));
    options.origins.extend(origins);
    options.journal = journal;
    options.keep_days = keep.min(passerelle::KEEP_MOST);
    let said = format!(
        "passerelle : les pages sur {}, les postes sur {} ; origines {} ; journal {}",
        options.web,
        options.posts,
        options.origins.join(", "),
        match &options.journal {
            Some(d) => format!("dans {} ({} jours)", d.display(), options.keep_days),
            None => "aucun".to_owned(),
        }
    );
    if let Err(e) = passerelle::serve(options, Arc::new(AtomicBool::new(false))) {
        eprintln!("la passerelle ne s'ouvre pas : {e}");
        std::process::exit(1);
    }
    eprintln!("{said}");
    loop {
        std::thread::park();
    }
}
