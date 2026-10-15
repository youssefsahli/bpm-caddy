//! Sous Windows, l'icône et le nom du programme dans le `.exe` : ce que
//! l'Explorateur, le menu Démarrer, la barre des tâches et le Gestionnaire
//! des tâches montrent. Sans eux, l'icône générique d'un programme
//! inconnu, et le nom du fichier. Ailleurs, rien.
//!
//! L'icône est `assets/bpm-caddy.ico`, tirée de `motif::icon` (un test
//! de `motif` la tient égale).

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=assets/bpm-caddy.ico");
    // L'hôte construit pour Windows : la machine de la publication. Une
    // vérification croisée depuis Linux n'a pas de compilateur de
    // ressources, et n'en a pas besoin.
    #[cfg(windows)]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/bpm-caddy.ico")
            .set("ProductName", "BPM-Caddy")
            .set("FileDescription", "BPM-Caddy")
            .set("CompanyName", "BPM-Caddy");
        res.compile().expect("ressources Windows (icône)");
    }
}
