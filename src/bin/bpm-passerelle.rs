//! **La passerelle des patients** (`src/passerelle.rs`) : à lancer sur un
//! serveur, derrière un proxy qui porte le certificat du nom de domaine.
//!
//! ```text
//! bpm-passerelle [--web 127.0.0.1:8746] [--postes 0.0.0.0:7747]
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
//! Elle ne garde rien, ne lit rien et n'écrit rien sur le disque. Le port
//! des postes doit être ouvert vers Internet ; celui de la page, non.

use std::net::SocketAddr;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

fn main() {
    let mut web = bpm_caddy::passerelle::DEFAULT_WEB.to_owned();
    let mut posts = format!("0.0.0.0:{}", bpm_caddy::passerelle::DEFAULT_POSTS_PORT);
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match (a.as_str(), args.next()) {
            ("--web", Some(v)) => web = v,
            ("--postes", Some(v)) => posts = v,
            _ => {
                eprintln!("usage : bpm-passerelle [--web 127.0.0.1:8746] [--postes 0.0.0.0:7747]");
                std::process::exit(2);
            }
        }
    }
    let parse = |s: &str| -> SocketAddr {
        s.parse().unwrap_or_else(|_| {
            eprintln!("adresse illisible : {s}");
            std::process::exit(2);
        })
    };
    let (web, posts) = (parse(&web), parse(&posts));
    if let Err(e) = bpm_caddy::passerelle::serve(web, posts, Arc::new(AtomicBool::new(false))) {
        eprintln!("la passerelle ne s'ouvre pas : {e}");
        std::process::exit(1);
    }
    eprintln!("passerelle : la page sur {web}, les postes sur {posts}");
    loop {
        std::thread::park();
    }
}
