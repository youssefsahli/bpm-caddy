#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! BPM-Caddy launcher.
//!
//! On startup it queries the GitHub Releases API for the latest version,
//! downloads the platform binary if the installed copy is missing or
//! outdated, then starts BPM-Caddy and exits. If the update check fails but
//! a binary is already installed, it launches that copy so the pharmacy is
//! never blocked by a network outage.

use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use eframe::egui;

const REPO: &str = "youssefsahli/bpm-caddy";

/// **La clé publique des mises à jour** (minisign), dans le dépôt :
/// `launcher/update-key.pub`. Vide, rien n'est vérifié — c'est l'état tant
/// que l'éditeur n'a pas créé sa clé, hors ligne. Posée, chaque binaire
/// installé doit venir avec sa signature (`<binaire>.minisig`, déposée
/// par `scripts/sign-release.sh`), faite par cette clé, et dont le
/// commentaire signé nomme ce binaire **et cette version** : une
/// signature d'une autre version ne fait pas installer celle-ci.
const UPDATE_KEY: &str = include_str!("../update-key.pub");

/// La clé, lue dans son fichier (la ligne de base64 après le commentaire),
/// ou `None` quand le fichier n'en porte pas.
fn update_key() -> Option<&'static str> {
    UPDATE_KEY
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty() && !l.starts_with("untrusted comment:"))
}

/// Ce binaire est-il celui que la clé a signé, pour ce nom et cette
/// version ?
fn verify_update(
    key: &str,
    bytes: &[u8],
    signature: &str,
    asset: &str,
    tag: &str,
) -> Result<(), String> {
    let key = minisign_verify::PublicKey::from_base64(key)
        .map_err(|e| format!("clé des mises à jour illisible : {e}"))?;
    let sig = minisign_verify::Signature::decode(signature)
        .map_err(|e| format!("signature illisible : {e}"))?;
    key.verify(bytes, &sig, false).map_err(|_| {
        "signature refusée : ce binaire n'est pas celui que l'éditeur a signé".to_owned()
    })?;
    let expected = format!("{asset} {tag}");
    if sig.trusted_comment().trim() != expected {
        return Err(format!(
            "signature d'un autre binaire ou d'une autre version (« {} », attendu « {expected} »)",
            sig.trusted_comment().trim()
        ));
    }
    Ok(())
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
const APP_ASSET: &str = "bpm-caddy-linux-x86_64";
#[cfg(target_os = "windows")]
const APP_ASSET: &str = "bpm-caddy-windows-x86_64.exe";
#[cfg(target_os = "macos")]
const APP_ASSET: &str = "bpm-caddy-macos-arm64";

/// **Le chemin du lanceur, pris au démarrage**, avant tout remplacement :
/// sous Linux, `current_exe` suit le fichier renommé — après une mise à
/// jour de lui-même, il nommait `….old`, que l'application inscrivait
/// alors au lancement à l'ouverture de session, et que le lancement
/// suivant effaçait.
static ME: std::sync::OnceLock<Option<PathBuf>> = std::sync::OnceLock::new();

fn me() -> Option<PathBuf> {
    ME.get_or_init(|| std::env::current_exe().ok()).clone()
}

/// Les fils qui travaillent encore, attendus à la sortie : la fenêtre se
/// ferme au lancement de l'application, et un remplacement du lanceur
/// coupé entre ses deux renommages ne laisserait rien à sa place.
static WORKERS: Mutex<Vec<std::thread::JoinHandle<()>>> = Mutex::new(Vec::new());

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
const LAUNCHER_ASSET: &str = "bpm-caddy-launcher-linux-x86_64";
#[cfg(target_os = "windows")]
const LAUNCHER_ASSET: &str = "bpm-caddy-launcher-windows-x86_64.exe";
#[cfg(target_os = "macos")]
const LAUNCHER_ASSET: &str = "bpm-caddy-launcher-macos-arm64";

#[derive(Clone, PartialEq)]
enum Phase {
    Checking,
    Downloading(String),
    ReadyToLaunch(String),
    Error(String),
}

struct Shared {
    phase: Mutex<Phase>,
    downloaded: AtomicU64,
    total: AtomicU64,
}

/// Jamais le dossier courant : un raccourci lancé depuis un partage
/// téléchargeait, puis exécutait, ce que n'importe qui pouvait y
/// déposer. Sans dossier de données, le dossier personnel ; sans lui,
/// celui du lanceur lui-même.
fn app_dir() -> PathBuf {
    dirs::data_dir()
        .or_else(|| dirs::home_dir().map(|h| h.join(".local").join("share")))
        .or_else(|| {
            std::env::current_exe()
                .ok()
                .and_then(|e| e.parent().map(PathBuf::from))
        })
        .unwrap_or_else(std::env::temp_dir)
        .join("bpm-caddy")
}

/// The skin the application is set to, read straight out of its
/// `config.toml`.
///
/// The launcher is the first window of the evening, and it opened in the
/// blue-grey of `mwm` whatever the officine had chosen: an operator who
/// picked « Nuit » got a lit rectangle in the face before the dark one
/// arrived. Read by hand and not through the app's `Config`: the
/// launcher does not depend on the application crate, and a whole
/// configuration is not needed to answer one key. Anything unreadable —
/// no file, no key, a name this build does not know — leaves
/// `motif::set_theme` on the classic palette, which is what it does with
/// an unknown key anyway.
fn configured_theme() -> Option<String> {
    theme_in(&configuration()?)
}

/// The application's `config.toml`, as text.
fn configuration() -> Option<String> {
    let path = dirs::config_dir()?.join("bpm-caddy").join("config.toml");
    std::fs::read_to_string(path).ok()
}

/// The `[ui] theme` of a configuration file, or nothing.
fn theme_in(text: &str) -> Option<String> {
    value_in(text, "ui", "theme")
}

/// One `key = "value"` of one `[section]` of a configuration file.
fn value_in(text: &str, section: &str, key: &str) -> Option<String> {
    let mut inside = false;
    let header = format!("[{section}]");
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            inside = line == header;
            continue;
        }
        if !inside {
            continue;
        }
        if let Some(value) = line.strip_prefix(key) {
            let value = value.trim_start();
            if let Some(value) = value.strip_prefix('=') {
                return Some(value.trim().trim_matches('"').to_owned());
            }
        }
    }
    None
}

/// **Le proxy web** par lequel chercher la mise à jour, comme l'application
/// le cherche (`src/proxy.rs`, dont ceci est le court résumé : le lanceur
/// ne dépend pas d'elle) : `[reseau] proxy` de sa configuration (« aucun »
/// pour s'en passer), sinon les variables d'environnement (lues par
/// `ureq`), sinon le réglage du système — Windows : le registre d'Internet
/// Settings ; macOS : `scutil --proxy`. Une URL que `ureq` prend.
fn system_proxy() -> Option<String> {
    let run = |cmd: &str, args: &[&str]| {
        std::process::Command::new(cmd)
            .args(args)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
    };
    if cfg!(windows) {
        let text = run(
            "reg",
            &[
                "query",
                r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings",
            ],
        )?;
        proxy_in_registry(&text)
    } else if cfg!(target_os = "macos") {
        proxy_in_scutil(&run("scutil", &["--proxy"])?)
    } else {
        None
    }
}

/// `ProxyServer` quand `ProxyEnable` vaut 1 : `hôte:port`, ou celui de
/// `https=` (sinon `http=`) d'un réglage par protocole.
fn proxy_in_registry(text: &str) -> Option<String> {
    let value = |name: &str| {
        text.lines().find_map(|l| {
            let mut parts = l.split_whitespace();
            (parts.next()? == name).then(|| parts.nth(1).map(str::to_owned))?
        })
    };
    if value("ProxyEnable")? != "0x1" {
        return None;
    }
    let server = value("ProxyServer")?;
    if !server.contains('=') {
        return Some(server);
    }
    let by = |p: &str| {
        server.split(';').find_map(|part| {
            let (k, v) = part.split_once('=')?;
            k.trim()
                .eq_ignore_ascii_case(p)
                .then(|| v.trim().to_owned())
        })
    };
    by("https").or_else(|| by("http"))
}

/// Le proxy HTTPS (sinon HTTP) allumé que dit `scutil --proxy`.
fn proxy_in_scutil(text: &str) -> Option<String> {
    let value = |name: &str| {
        text.lines().find_map(|l| {
            let (k, v) = l.split_once(':')?;
            (k.trim() == name).then(|| v.trim().to_owned())
        })
    };
    [
        ("HTTPSEnable", "HTTPSProxy", "HTTPSPort"),
        ("HTTPEnable", "HTTPProxy", "HTTPPort"),
    ]
    .into_iter()
    .find_map(|(on, host, port)| {
        (value(on).as_deref() == Some("1"))
            .then(|| Some(format!("{}:{}", value(host)?, value(port)?)))?
    })
}

/// Le proxy à prendre, d'après la configuration écrite `written` : `None`
/// pour « aucun », ou pour laisser `ureq` lire l'environnement.
fn chosen_proxy(written: Option<&str>, system: impl FnOnce() -> Option<String>) -> Option<String> {
    match written.map(str::trim) {
        Some(w) if w.eq_ignore_ascii_case("aucun") => None,
        Some(w) if !w.is_empty() => Some(w.to_owned()),
        _ if [
            "HTTPS_PROXY",
            "https_proxy",
            "ALL_PROXY",
            "all_proxy",
            "HTTP_PROXY",
            "http_proxy",
        ]
        .iter()
        .any(|k| std::env::var_os(k).is_some()) =>
        {
            None
        }
        _ => system(),
    }
}

fn bin_path() -> PathBuf {
    app_dir().join(format!("bpm-caddy{}", std::env::consts::EXE_SUFFIX))
}

fn version_file() -> PathBuf {
    app_dir().join("version.txt")
}

/// La taille du binaire installé, écrite à côté de sa version : une
/// version juste sur un fichier tronqué — une coupure de courant après
/// le renommage — ne se retéléchargeait jamais.
fn size_file() -> PathBuf {
    app_dir().join("version.size")
}

/// Le binaire installé est-il là, et entier ? Sa taille est retenue à
/// l'installation ; un fichier qui n'a plus cette taille est abîmé.
fn installed_whole() -> bool {
    let bin = bin_path();
    match std::fs::read_to_string(size_file())
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
    {
        Some(size) => std::fs::metadata(&bin).is_ok_and(|m| m.len() == size),
        None => bin.exists(),
    }
}

/// Les nombres d'une étiquette `v0.257.0`, pour comparer des versions
/// comme des nombres : « 0.100 » est après « 0.99 ». **Et la version
/// finale après ses préversions** : `v0.405.0-rc1` se lit `[0, 405, 0]`
/// suivi de 0, `v0.405.0` suivi de 1. Lues pareilles, la préversion
/// installée tenait la finale pour « à jour », jamais installée ; et
/// `v0.405.0-1` passait devant elle.
fn version_parts(tag: &str) -> Vec<u64> {
    let tag = tag.trim().trim_start_matches('v');
    let (core, suffix) = match tag.find(['-', '+']) {
        Some(at) => (&tag[..at], true),
        None => (tag, false),
    };
    let mut parts: Vec<u64> = core.split('.').map_while(|p| p.parse().ok()).collect();
    if !parts.is_empty() {
        parts.resize(3, 0);
        parts.push(u64::from(!suffix));
    }
    parts
}

#[derive(Debug, PartialEq, Eq)]
enum Decision {
    UpToDate,
    /// Le binaire est abîmé, mais la publication est plus ancienne.
    Older,
    Install,
}

/// **Jamais en arrière** : si la dernière publication est retirée,
/// « la plus récente » redevient la précédente, et chaque officine aurait
/// remplacé une version par une plus ancienne — qui ouvrirait une base
/// qu'une version plus récente a déjà migrée. Un binaire abîmé ne se
/// répare pas non plus par une version plus ancienne.
fn decide(tag: &str, installed: &str, whole: bool) -> Decision {
    let (published, here) = (version_parts(tag), version_parts(installed));
    if whole && published <= here {
        Decision::UpToDate
    } else if !installed.trim().is_empty() && published < here {
        Decision::Older
    } else {
        Decision::Install
    }
}

fn worker(shared: Arc<Shared>, ctx: egui::Context) {
    let latest = latest_release().map_err(|e| e.to_string());
    let outcome = match &latest {
        Ok((agent, release, tag)) => {
            update_app(&shared, agent, release, tag).map_err(|e| e.to_string())
        }
        Err(e) => Err(e.clone()),
    };
    let mut phase = shared.phase.lock().unwrap();
    *phase = match outcome {
        Ok(note) => Phase::ReadyToLaunch(note),
        // Une copie **entière**, pas seulement présente : un binaire
        // tronqué (téléchargement interrompu d'une ancienne version)
        // échouait au lancement sans rien dire de ce qui manquait.
        Err(e) if installed_whole() => {
            Phase::ReadyToLaunch(format!("Mise à jour impossible ({e}) — version installée"))
        }
        Err(e) => Phase::Error(e),
    };
    drop(phase);
    ctx.request_repaint();
    // **Le lanceur ensuite**, l'application déjà lancée — il ne la retient
    // pas — et quoi qu'il soit arrivé à sa mise à jour : un lanceur qui
    // ne se remplaçait jamais ne recevait ni ses corrections ni la clé
    // des mises à jour. Le résultat est écrit à côté de l'application
    // (`lanceur.txt`) : la fenêtre est fermée.
    if let Ok((agent, release, tag)) = &latest {
        let said = match update_self(agent, release, tag) {
            Ok(true) => Some(format!("Lanceur mis à jour vers {tag}")),
            Ok(false) => None,
            Err(e) => Some(format!("Lanceur non mis à jour vers {tag} : {e}")),
        };
        if let Some(said) = said {
            let _ = std::fs::write(app_dir().join("lanceur.txt"), said);
        }
    }
}

/// La dernière publication, et l'agent qui l'a lue.
fn latest_release() -> Result<(ureq::Agent, serde_json::Value, String), Box<dyn std::error::Error>>
{
    // Timeouts matter here: without them a hung connection blocks the
    // pharmacy at startup even though an installed copy is ready to run.
    // The read timeout is per read() call, so slow-but-progressing
    // downloads are fine while genuine stalls fail fast.
    let written = configuration().and_then(|c| value_in(&c, "reseau", "proxy"));
    let mut builder = ureq::AgentBuilder::new();
    let off = written
        .as_deref()
        .is_some_and(|w| w.trim().eq_ignore_ascii_case("aucun"));
    if !off {
        builder = builder.try_proxy_from_env(true);
    }
    if let Some(p) =
        chosen_proxy(written.as_deref(), system_proxy).and_then(|p| ureq::Proxy::new(p).ok())
    {
        builder = builder.proxy(p);
    }
    let agent = builder
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(30))
        .user_agent("bpm-caddy-launcher")
        // Une redirection vers `http://` n'est pas suivie : ce qui
        // arrive ici devient l'exécutable du comptoir.
        .https_only(true)
        .build();
    let release: serde_json::Value = agent
        .get(&format!(
            "https://api.github.com/repos/{REPO}/releases/latest"
        ))
        .call()?
        .into_json()?;
    let tag = release["tag_name"]
        .as_str()
        .ok_or("réponse GitHub sans tag_name")?
        .to_string();
    Ok((agent, release, tag))
}

/// La signature d'un binaire de la publication, quand une clé est posée ;
/// absente alors, la publication n'est pas signée et rien ne s'installe.
fn signature_for(
    agent: &ureq::Agent,
    assets: &[serde_json::Value],
    asset: &str,
    tag: &str,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    if update_key().is_none() {
        return Ok(None);
    }
    let name = format!("{asset}.minisig");
    let sig_url = assets
        .iter()
        .find(|a| a["name"].as_str() == Some(name.as_str()))
        .and_then(|a| a["browser_download_url"].as_str())
        .ok_or_else(|| format!("la version {tag} n'est pas encore signée : rien n'est installé"))?;
    let mut text = String::new();
    agent
        .get(sig_url)
        .call()?
        .into_reader()
        .take(64 * 1024)
        .read_to_string(&mut text)?;
    Ok(Some(text))
}

/// **Remplacer le lanceur qui tourne** par celui de la publication, s'il
/// est plus récent : téléchargé à côté, vérifié comme l'application
/// (taille, octets d'un exécutable, signature quand une clé est posée),
/// puis mis en place par deux renommages — l'ancien de côté (`.old`,
/// retiré au lancement suivant), le nouveau à sa place. Un système qui
/// refuse le second renommage remet l'ancien. Rend `true` une fois fait.
///
/// Renommer un programme en cours d'exécution est permis sous Windows
/// comme ailleurs ; l'écraser ne l'est pas, d'où les deux temps.
fn update_self(
    agent: &ureq::Agent,
    release: &serde_json::Value,
    tag: &str,
) -> Result<bool, Box<dyn std::error::Error>> {
    // Une copie de développement ne se remplace pas par la publiée.
    if cfg!(debug_assertions) || version_parts(tag) <= version_parts(env!("CARGO_PKG_VERSION")) {
        return Ok(false);
    }
    let me = me().ok_or("chemin du lanceur inconnu")?;
    let assets = release["assets"].as_array().ok_or("release sans assets")?;
    let asset = assets
        .iter()
        .find(|a| a["name"].as_str() == Some(LAUNCHER_ASSET))
        .ok_or_else(|| format!("pas de lanceur {LAUNCHER_ASSET} dans la release {tag}"))?;
    let url = asset["browser_download_url"]
        .as_str()
        .ok_or("asset sans URL de téléchargement")?;
    let expected_size = asset["size"].as_u64().unwrap_or(0);
    let signature = signature_for(agent, assets, LAUNCHER_ASSET, tag)?;
    let mut bytes = Vec::new();
    agent
        .get(url)
        .call()?
        .into_reader()
        .take(512 << 20)
        .read_to_end(&mut bytes)?;
    if expected_size > 0 && bytes.len() as u64 != expected_size {
        return Err(format!(
            "lanceur incomplet ({} / {expected_size} octets)",
            bytes.len()
        )
        .into());
    }
    if let (Some(key), Some(sig)) = (update_key(), &signature) {
        verify_update(key, &bytes, sig, LAUNCHER_ASSET, tag)?;
    }
    install_over(&me, &bytes)?;
    Ok(true)
}

/// Poser `bytes` à la place du programme `me` : écrit à côté, vérifié
/// exécutable, puis deux renommages — `me` de côté en `.old`, le nouveau
/// à sa place ; le second refusé, l'ancien revient.
fn install_over(me: &std::path::Path, bytes: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let file_name = me
        .file_name()
        .ok_or("lanceur sans nom de fichier")?
        .to_string_lossy()
        .into_owned();
    let new = me.with_file_name(format!("{file_name}.{}.nouveau", std::process::id()));
    let old = me.with_file_name(format!("{file_name}.old"));
    let written = (|| -> Result<(), Box<dyn std::error::Error>> {
        let mut file = std::fs::File::create(&new)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        looks_executable(&new)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&new, std::fs::Permissions::from_mode(0o755))?;
        }
        Ok(())
    })();
    if let Err(e) = written {
        let _ = std::fs::remove_file(&new);
        return Err(e);
    }
    let _ = std::fs::remove_file(&old);
    if let Err(e) = std::fs::rename(me, &old) {
        let _ = std::fs::remove_file(&new);
        return Err(e.into());
    }
    if let Err(e) = std::fs::rename(&new, me) {
        // L'ancien revient — par un renommage, sinon par une copie : un
        // lanceur absent de son chemin, c'est le raccourci et le
        // lancement à l'ouverture de session qui ne mènent plus à rien.
        let back = std::fs::rename(&old, me).or_else(|_| std::fs::copy(&old, me).map(|_| ()));
        let _ = std::fs::remove_file(&new);
        return Err(match back {
            Ok(()) => e.into(),
            Err(b) => format!(
                "lanceur introuvable à sa place ({e}, puis {b}) : l'ancien est {}",
                old.display()
            )
            .into(),
        });
    }
    Ok(())
}

/// Ce que le remplacement précédent a laissé de côté : l'ancien lanceur,
/// qui ne tourne plus, et un téléchargement coupé.
fn sweep_old_launcher() {
    if let Some(me) = me() {
        sweep_beside(&me);
    }
}

fn sweep_beside(me: &std::path::Path) {
    let (Some(dir), Some(name)) = (me.parent(), me.file_name()) else {
        return;
    };
    let name = name.to_string_lossy().into_owned();
    let _ = std::fs::remove_file(dir.join(format!("{name}.old")));
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.flatten() {
            let n = e.file_name().to_string_lossy().into_owned();
            if n.starts_with(&format!("{name}.")) && n.ends_with(".nouveau") {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
}

fn update_app(
    shared: &Shared,
    agent: &ureq::Agent,
    release: &serde_json::Value,
    tag: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let tag = tag.to_owned();
    let installed = std::fs::read_to_string(version_file()).unwrap_or_default();
    let bin = bin_path();
    // Le binaire installé est-il entier ? Sa taille est retenue à
    // l'installation ; une taille qui ne correspond plus est un fichier
    // abîmé, et il se retélécharge.
    let whole = installed_whole();
    match decide(&tag, &installed, whole) {
        Decision::UpToDate => return Ok(format!("À jour ({})", installed.trim())),
        Decision::Older => {
            return Err(format!(
                "binaire installé abîmé, et la version publiée ({tag}) est plus ancienne que {} : \
                 rien n'est remplacé",
                installed.trim()
            )
            .into())
        }
        Decision::Install => {}
    }

    *shared.phase.lock().unwrap() = Phase::Downloading(tag.clone());

    let assets = release["assets"].as_array().ok_or("release sans assets")?;
    let asset = assets
        .iter()
        .find(|a| a["name"].as_str() == Some(APP_ASSET))
        .ok_or_else(|| format!("pas de binaire {APP_ASSET} dans la release {tag}"))?;
    let url = asset["browser_download_url"]
        .as_str()
        .ok_or("asset sans URL de téléchargement")?;
    let expected_size = asset["size"].as_u64().unwrap_or(0);
    // La signature d'abord, quand une clé est posée : sans elle, rien ne
    // se télécharge — la version installée reste et se lance.
    let signature = signature_for(agent, assets, APP_ASSET, &tag)?;

    let resp = agent.get(url).call()?;
    let total: u64 = resp
        .header("Content-Length")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    shared.total.store(total, Ordering::Relaxed);
    shared.downloaded.store(0, Ordering::Relaxed);

    std::fs::create_dir_all(app_dir())?;
    // Les temporaires qu'un lanceur fermé en route a laissés — celui
    // qu'on quitte par « Lancer la version installée » en laisse un.
    // Seulement ceux de plus d'une heure : un autre lanceur peut être en
    // train d'écrire le sien.
    if let Ok(dir) = std::fs::read_dir(app_dir()) {
        for entry in dir.flatten() {
            let path = entry.path();
            let stale =
                path.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("bpm-caddy") && n.ends_with(".part"))
                    && entry.metadata().and_then(|m| m.modified()).is_ok_and(|t| {
                        t.elapsed().is_ok_and(|age| age > Duration::from_secs(3600))
                    });
            if stale {
                let _ = std::fs::remove_file(path);
            }
        }
    }
    // **Un fichier temporaire par lanceur** : sous un nom fixe, un second
    // lanceur ouvert d'un double clic vidait le fichier que le premier
    // écrivait, et le premier installait un binaire troué — que plus
    // rien ne remplaçait, puisque sa version était la bonne.
    let tmp = bin.with_extension(format!("{}.part", std::process::id()));
    let download = || -> Result<u64, Box<dyn std::error::Error>> {
        let mut file = std::fs::File::create(&tmp)?;
        let mut reader = resp.into_reader();
        let mut buf = [0u8; 64 * 1024];
        let mut written: u64 = 0;
        loop {
            let n = reader.read(&mut buf)?;
            if n == 0 {
                break;
            }
            file.write_all(&buf[..n])?;
            written += n as u64;
            shared.downloaded.fetch_add(n as u64, Ordering::Relaxed);
        }
        // **Sur le disque avant le renommage** : `flush` ne fait rien sur
        // un fichier, et une coupure de courant juste après pouvait
        // laisser le nouveau nom sur un contenu vide.
        file.sync_all()?;
        Ok(written)
    };
    let written = match download() {
        Ok(n) => n,
        Err(e) => {
            // Rien ne traîne sur un disque peut-être déjà plein.
            let _ = std::fs::remove_file(&tmp);
            return Err(e);
        }
    };

    // Never install a truncated binary (a cleanly closed connection can
    // end a download early without any read error).
    if expected_size > 0 && written != expected_size {
        let _ = std::fs::remove_file(&tmp);
        return Err(
            format!("téléchargement incomplet ({written} / {expected_size} octets)").into(),
        );
    }

    // **Et ce qu'on installe est un exécutable, lu dans ses octets.**
    // C'est la règle que l'application applique déjà aux pièces
    // numérisées : accepter un fichier parce qu'il porte le bon nom,
    // c'est accepter de rendre plus tard quelque chose que personne n'a
    // regardé. Ici le nom vient de GitHub et la taille est vérifiée,
    // mais un portail captif ou un proxy d'entreprise répond 200 à tout
    // — et ce qui serait alors posé à la place du binaire se lancerait
    // au démarrage suivant.
    if let Err(e) = looks_executable(&tmp) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e.into());
    }
    // **Signé par l'éditeur**, quand une clé est posée : un accès en
    // écriture aux publications (un jeton fuité, une action compromise)
    // ne suffit plus à faire exécuter quelque chose à chaque officine.
    if let (Some(key), Some(sig)) = (update_key(), &signature) {
        let checked = std::fs::read(&tmp)
            .map_err(|e| e.to_string())
            .and_then(|bytes| verify_update(key, &bytes, sig, APP_ASSET, &tag));
        if let Err(e) = checked {
            let _ = std::fs::remove_file(&tmp);
            return Err(e.into());
        }
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Err(e) = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755)) {
            let _ = std::fs::remove_file(&tmp);
            return Err(e.into());
        }
    }
    // L'ancienne taille part **avant** le nouveau binaire : si sa
    // réécriture échoue ensuite (disque plein), une taille restée de la
    // version d'avant déclarait abîmé le binaire neuf, retéléchargé à
    // chaque lancement — et refusé hors ligne.
    let kept_size = std::fs::read_to_string(size_file()).ok();
    let _ = std::fs::remove_file(size_file());
    if let Err(e) = std::fs::rename(&tmp, &bin) {
        // Sous Windows, un binaire en cours d'exécution ne se remplace
        // pas : la version installée reste, et le temporaire part. **Sa
        // taille revient avec elle** : sans elle, un binaire tronqué
        // passait pour entier jusqu'à la prochaine installation.
        let _ = std::fs::remove_file(&tmp);
        if let Some(size) = kept_size {
            let _ = std::fs::write(size_file(), size);
        }
        return Err(e.into());
    }
    std::fs::write(size_file(), written.to_string())?;
    std::fs::write(version_file(), &tag)?;

    Ok(format!("Mise à jour vers {tag} effectuée"))
}

/// Les premiers octets d'un exécutable de cette plateforme — ELF, PE ou
/// Mach-O. Quatre octets suffisent : il ne s'agit pas de valider un
/// binaire, seulement de refuser ce qui n'en est visiblement pas un.
fn looks_executable(path: &std::path::Path) -> Result<(), String> {
    use std::io::Read as _;
    let mut head = [0u8; 4];
    let mut f = std::fs::File::open(path).map_err(|e| format!("fichier illisible : {e}"))?;
    let n = f
        .read(&mut head)
        .map_err(|e| format!("fichier illisible : {e}"))?;
    if n < 4 {
        return Err("fichier téléchargé trop court pour être un programme".to_owned());
    }
    let ok = if cfg!(target_os = "windows") {
        head.starts_with(b"MZ")
    } else if cfg!(target_os = "macos") {
        // Mach-O 64 bits (little et big endian) et les binaires
        // universels, que `cargo build` produit sur les runners Apple.
        matches!(
            head,
            [0xcf, 0xfa, 0xed, 0xfe]
                | [0xfe, 0xed, 0xfa, 0xcf]
                | [0xca, 0xfe, 0xba, 0xbe]
                | [0xbe, 0xba, 0xfe, 0xca]
        )
    } else {
        head == *b"\x7fELF"
    };
    if ok {
        Ok(())
    } else {
        Err(format!(
            "le fichier téléchargé n'est pas un programme ({head:02x?}) — \
             réseau filtré, ou release incomplète"
        ))
    }
}

struct Launcher {
    shared: Arc<Shared>,
    launched: bool,
}

impl Launcher {
    fn new(ctx: &egui::Context) -> Self {
        // The palette before the style: `motif::apply` writes the
        // palette into egui's own visuals, so a theme set after it would
        // only reach the widgets Motif paints by hand. Same order as the
        // application's.
        if let Some(theme) = configured_theme() {
            motif::set_theme(&theme);
        }
        motif::apply(ctx);
        let shared = Arc::new(Shared {
            phase: Mutex::new(Phase::Checking),
            downloaded: AtomicU64::new(0),
            total: AtomicU64::new(0),
        });
        Self::spawn_worker(&shared, ctx);
        Self {
            shared,
            launched: false,
        }
    }

    fn spawn_worker(shared: &Arc<Shared>, ctx: &egui::Context) {
        let shared = Arc::clone(shared);
        let ctx = ctx.clone();
        let handle = std::thread::spawn(move || worker(shared, ctx));
        if let Ok(mut w) = WORKERS.lock() {
            w.push(handle);
        }
    }

    fn launch_app(&mut self, ctx: &egui::Context) {
        // Ses arguments passent à l'application (`--arriere-plan`, du
        // lancement à l'ouverture de session), et elle apprend qui l'a
        // lancée : c'est le lanceur qu'elle inscrit au démarrage, pour que
        // la mise à jour passe avant elle.
        let mut command = Command::new(bin_path());
        command.args(std::env::args_os().skip(1));
        if let Some(me) = me() {
            command.env("BPM_CADDY_LAUNCHER", me);
        }
        match command.spawn() {
            Ok(_) => {
                self.launched = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            Err(e) => {
                *self.shared.phase.lock().unwrap() =
                    Phase::Error(format!("échec du démarrage : {e}"));
            }
        }
    }
}

impl eframe::App for Launcher {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let phase = self.shared.phase.lock().unwrap().clone();

        if matches!(phase, Phase::Checking | Phase::Downloading(_)) {
            ctx.request_repaint_after(Duration::from_millis(50));
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            motif::bevel(ui.painter(), ui.max_rect().shrink(3.0), true);
            ui.vertical_centered(|ui| {
                ui.add_space(30.0);
                ui.heading("BPM-Caddy");
                ui.label("Lanceur — mise à jour automatique");
                ui.add_space(20.0);

                match &phase {
                    Phase::Checking => {
                        ui.label("Recherche de mises à jour…");
                        let t = ui.input(|i| i.time);
                        motif::progress_marquee(ui, 300.0, t);
                    }
                    Phase::Downloading(tag) => {
                        ui.label(format!("Téléchargement de {tag}…"));
                        let done = self.shared.downloaded.load(Ordering::Relaxed);
                        let total = self.shared.total.load(Ordering::Relaxed);
                        if total > 0 {
                            motif::progress_bar(ui, done as f32 / total as f32, 300.0);
                            ui.label(format!(
                                "{:.1} / {:.1} Mo",
                                done as f64 / 1e6,
                                total as f64 / 1e6
                            ));
                        } else {
                            let t = ui.input(|i| i.time);
                            motif::progress_marquee(ui, 300.0, t);
                        }
                        // **Le comptoir n'attend pas la mise à jour** :
                        // sur une ligne lente, trente mégaoctets prennent
                        // des minutes, et fermer la fenêtre laissait sans
                        // application. La version installée se lance
                        // tout de suite ; la mise à jour sera reprise au
                        // prochain démarrage.
                        if installed_whole() {
                            ui.add_space(6.0);
                            if motif::button(ui, "Lancer la version installée").clicked() {
                                self.launch_app(ctx);
                            }
                        }
                    }
                    Phase::ReadyToLaunch(note) => {
                        ui.label(note);
                        motif::progress_bar(ui, 1.0, 300.0);
                        ui.label("Démarrage de BPM-Caddy…");
                    }
                    Phase::Error(msg) => {
                        ui.label("Impossible d'installer BPM-Caddy :");
                        ui.label(msg);
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            ui.add_space(ui.available_width() / 2.0 - 110.0);
                            if motif::button(ui, "Réessayer").clicked() {
                                *self.shared.phase.lock().unwrap() = Phase::Checking;
                                Self::spawn_worker(&self.shared, ctx);
                            }
                            if motif::button(ui, "Quitter").clicked() {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                            }
                        });
                    }
                }
            });
        });

        if !self.launched {
            if let Phase::ReadyToLaunch(_) = phase {
                self.launch_app(ctx);
            }
        }
    }
}

fn main() -> eframe::Result {
    // Le chemin d'abord, avant que quoi que ce soit ne le renomme.
    let _ = me();
    sweep_old_launcher();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([420.0, 240.0])
            .with_resizable(false)
            .with_icon(motif::icon())
            .with_title("BPM-Caddy — Lanceur"),
        ..Default::default()
    };
    let shown = eframe::run_native(
        "BPM-Caddy Launcher",
        options,
        Box::new(|cc| Ok(Box::new(Launcher::new(&cc.egui_ctx)))),
    );
    // La fenêtre fermée, les fils finissent ce qu'ils ont commencé — au
    // plus quelques minutes, sans fenêtre.
    let until = std::time::Instant::now() + Duration::from_secs(300);
    let workers = WORKERS
        .lock()
        .map(|mut w| std::mem::take(&mut *w))
        .unwrap_or_default();
    for w in workers {
        while !w.is_finished() && std::time::Instant::now() < until {
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    shown
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_proxy_is_found_where_the_application_finds_it() {
        use super::*;
        let reg = "    ProxyEnable    REG_DWORD    0x1\r\n    ProxyServer    REG_SZ    http=p:8080;https=s:8443\r\n";
        assert_eq!(proxy_in_registry(reg).as_deref(), Some("s:8443"));
        assert_eq!(proxy_in_registry(&reg.replace("0x1", "0x0")), None);
        let mac = "  HTTPEnable : 1\n  HTTPPort : 3128\n  HTTPProxy : p.chu.fr\n";
        assert_eq!(proxy_in_scutil(mac).as_deref(), Some("p.chu.fr:3128"));
        let conf = "[ui]\ntheme = \"nuit\"\n[reseau]\nproxy = \"http://w:1\"\n";
        assert_eq!(
            value_in(conf, "reseau", "proxy").as_deref(),
            Some("http://w:1")
        );
        assert_eq!(theme_in(conf).as_deref(), Some("nuit"));
        assert_eq!(chosen_proxy(Some("aucun"), || Some("s:1".into())), None);
        assert_eq!(
            chosen_proxy(Some("w:2"), || Some("s:1".into())).as_deref(),
            Some("w:2")
        );
    }

    /// Une paire de clés et une signature **au format de minisign**
    /// (pré-haché BLAKE2b-512, commentaire signé), faites ici comme la
    /// commande les fait : ce que `verify_update` lit en vrai.
    fn minisign(seed: u8, bytes: &[u8], comment: &str) -> (String, String) {
        use base64::Engine as _;
        use blake2::Digest as _;
        use ed25519_dalek::Signer as _;
        let b64 = base64::engine::general_purpose::STANDARD;
        let sk = ed25519_dalek::SigningKey::from_bytes(&[seed; 32]);
        let id = [seed; 8];
        let mut pk = b"Ed".to_vec();
        pk.extend_from_slice(&id);
        pk.extend_from_slice(sk.verifying_key().as_bytes());
        let hash = blake2::Blake2b512::digest(bytes);
        let sig = sk.sign(&hash).to_bytes();
        let mut line = b"ED".to_vec();
        line.extend_from_slice(&id);
        line.extend_from_slice(&sig);
        let mut global = sig.to_vec();
        global.extend_from_slice(comment.as_bytes());
        let global = sk.sign(&global).to_bytes();
        let text = format!(
            "untrusted comment: signature\n{}\ntrusted comment: {comment}\n{}\n",
            b64.encode(line),
            b64.encode(global)
        );
        (b64.encode(pk), text)
    }

    /// **Une mise à jour signée s'installe ; une autre, non.** Le binaire
    /// changé d'un octet, une autre clé, la signature d'une autre version
    /// ou d'un autre binaire : refusés.
    #[test]
    fn an_update_installs_only_with_the_publishers_signature() {
        let bin = b"\x7fELF un binaire".to_vec();
        let (key, sig) = minisign(7, &bin, "bpm-caddy-linux-x86_64 v0.436.0");
        assert_eq!(
            super::verify_update(&key, &bin, &sig, "bpm-caddy-linux-x86_64", "v0.436.0"),
            Ok(())
        );
        let mut changed = bin.clone();
        changed[5] ^= 1;
        assert!(
            super::verify_update(&key, &changed, &sig, "bpm-caddy-linux-x86_64", "v0.436.0")
                .is_err()
        );
        let (other_key, _) = minisign(8, &bin, "x");
        assert!(
            super::verify_update(&other_key, &bin, &sig, "bpm-caddy-linux-x86_64", "v0.436.0")
                .is_err()
        );
        assert!(
            super::verify_update(&key, &bin, &sig, "bpm-caddy-linux-x86_64", "v0.437.0").is_err()
        );
        assert!(
            super::verify_update(&key, &bin, &sig, "bpm-caddy-windows-x86_64.exe", "v0.436.0")
                .is_err()
        );
        // Le commentaire signé ne se réécrit pas.
        let forged = sig.replace("v0.436.0", "v0.437.0");
        assert!(
            super::verify_update(&key, &bin, &forged, "bpm-caddy-linux-x86_64", "v0.437.0")
                .is_err()
        );
    }

    /// **Le lanceur se remplace en place** : l'ancien de côté, le nouveau
    /// à son nom, et ce qui n'est pas un exécutable ne remplace rien ; le
    /// lancement suivant retire ce qui reste de côté.
    #[test]
    fn the_launcher_replaces_itself_and_sweeps_after() {
        let dir = std::env::temp_dir().join(format!("bpm-launcher-self-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let me = dir.join("bpm-caddy-launcher");
        let magic: &[u8] = if cfg!(windows) {
            b"MZ\x90\x00"
        } else if cfg!(target_os = "macos") {
            b"\xcf\xfa\xed\xfe"
        } else {
            b"\x7fELF"
        };
        let version = |tail: &[u8]| [magic, tail].concat();
        std::fs::write(&me, version(b" ancien")).unwrap();
        // Pas un exécutable : rien ne bouge, rien ne traîne.
        assert!(super::install_over(&me, b"<html>portail</html>").is_err());
        assert_eq!(std::fs::read(&me).unwrap(), version(b" ancien"));
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
        // Un exécutable : à sa place, l'ancien de côté.
        super::install_over(&me, &version(b" neuf")).unwrap();
        assert_eq!(std::fs::read(&me).unwrap(), version(b" neuf"));
        assert!(dir.join("bpm-caddy-launcher.old").exists());
        super::sweep_beside(&me);
        assert!(!dir.join("bpm-caddy-launcher.old").exists());
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Le fichier de clé du dépôt se lit : vide, aucune vérification ;
    /// posé, la ligne de clé, sans le commentaire.
    #[test]
    fn the_key_file_is_read_or_left_off() {
        match super::update_key() {
            None => assert!(super::UPDATE_KEY.trim().is_empty()),
            Some(k) => {
                assert!(minisign_verify::PublicKey::from_base64(k).is_ok(), "{k}");
            }
        }
    }

    /// The launcher opens in the skin the application is set to, and it
    /// reads that out of the file by hand — so the reading is held here.
    ///
    /// What matters is the two ways of getting it wrong: a `theme` under
    /// another section is not the interface's, and a key that merely
    /// starts with the word is not the key.
    /// Jamais en arrière, même pour réparer un binaire abîmé.
    #[test]
    fn an_update_never_goes_backwards() {
        use super::{decide, Decision};
        assert_eq!(decide("v0.397.0", "0.397.0", true), Decision::UpToDate);
        assert_eq!(decide("v0.398.0", "0.397.0", true), Decision::Install);
        assert_eq!(decide("v0.397.0", "0.397.0", false), Decision::Install);
        assert_eq!(decide("v0.397.0", "0.398.0", false), Decision::Older);
        assert_eq!(decide("v0.397.0", "", false), Decision::Install);
    }

    #[test]
    fn the_skin_is_read_out_of_the_ui_section_and_nowhere_else() {
        let t = |s: &str| super::theme_in(s);
        assert_eq!(t("[ui]\ntheme = \"nuit\"\n").as_deref(), Some("nuit"));
        assert_eq!(t("[ui]\ntheme=\"ambre\"").as_deref(), Some("ambre"));
        // Sans guillemets, et avec de l'air : un fichier écrit à la main.
        assert_eq!(t("[ui]\n  theme  =  cde  ").as_deref(), Some("cde"));
        // Une autre section, une autre clé : ni l'une ni l'autre.
        assert_eq!(t("[pharmacy]\ntheme = \"nuit\"\n"), None);
        assert_eq!(t("[ui]\ntheme_de_secours = \"nuit\"\n"), None);
        // La section se referme.
        assert_eq!(
            t("[ui]\ndensity = \"compact\"\n[base]\ntheme = \"x\""),
            None
        );
        assert_eq!(t(""), None);
    }

    /// The release workflow, read at compile time. It is the other half
    /// of this file's only external contract.
    const WORKFLOW: &str = include_str!("../../.github/workflows/release.yml");

    /// **Ce qui est installé est lu dans ses octets.**
    ///
    /// La même règle que pour une pièce numérisée : accepter un fichier
    /// parce qu'il porte le bon nom, c'est accepter de lancer plus tard
    /// quelque chose que personne n'a regardé. Un portail captif répond
    /// 200 à tout, et la page qu'il rend a une taille comme une autre.
    #[test]
    fn what_gets_installed_has_to_look_like_a_program() {
        let dir = std::env::temp_dir().join("bpm-caddy-launcher-magic-check");
        let _ = std::fs::create_dir_all(&dir);
        let page = dir.join("portail.bin");
        std::fs::write(&page, b"<!DOCTYPE html><html>Authentification requise").unwrap();
        assert!(super::looks_executable(&page).is_err());

        let court = dir.join("court.bin");
        std::fs::write(&court, b"MZ").unwrap();
        assert!(super::looks_executable(&court).is_err());

        let vrai = dir.join("vrai.bin");
        let magic: &[u8] = if cfg!(target_os = "windows") {
            b"MZ\x90\x00"
        } else if cfg!(target_os = "macos") {
            &[0xcf, 0xfa, 0xed, 0xfe]
        } else {
            b"\x7fELF"
        };
        let mut bytes = magic.to_vec();
        bytes.extend_from_slice(&[0u8; 64]);
        std::fs::write(&vrai, &bytes).unwrap();
        assert!(super::looks_executable(&vrai).is_ok());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The names this launcher downloads must be the names the workflow
    /// uploads.
    ///
    /// They live in two files that nothing else ties together, and a
    /// rename in either one is invisible: the build goes green, the
    /// release is published, and every launcher already installed at
    /// every officine looks for a file that is not there — silently,
    /// until somebody restarts and is told « pas de binaire … dans la
    /// release ». There is no way to fix that remotely, because the
    /// thing that would fetch the fix is the thing that is broken.
    #[test]
    fn the_launcher_asks_for_the_files_the_workflow_uploads() {
        // Each matrix row gives a `suffix` and an `ext`; the rename step
        // makes `bpm-caddy-{suffix}{ext}` out of them.
        let mut produced: Vec<String> = Vec::new();
        let mut suffix: Option<String> = None;
        for line in WORKFLOW.lines() {
            let t = line.trim();
            if let Some(v) = t.strip_prefix("suffix:") {
                suffix = Some(v.trim().trim_matches('"').to_owned());
            }
            if let Some(v) = t.strip_prefix("ext:") {
                let ext = v.trim().trim_matches('"').to_owned();
                if let Some(s) = suffix.take() {
                    produced.push(format!("bpm-caddy-{s}{ext}"));
                }
            }
        }
        assert_eq!(
            produced.len(),
            3,
            "trois plateformes attendues dans la matrice, lues : {produced:?}"
        );
        // Every name this file can be compiled with, whatever the
        // platform doing the compiling: a Linux build must still notice
        // that the Windows asset has been renamed.
        for name in [
            "bpm-caddy-linux-x86_64",
            "bpm-caddy-windows-x86_64.exe",
            "bpm-caddy-macos-arm64",
        ] {
            assert!(
                produced.iter().any(|p| p == name),
                "{name} n'est plus produit par le workflow : {produced:?}"
            );
        }
        // …and the one this build will actually ask GitHub for.
        assert!(
            produced.iter().any(|p| p == super::APP_ASSET),
            "{} n'est pas dans {produced:?}",
            super::APP_ASSET
        );
        // The rename step builds the names; the upload step must send
        // them. Two different lines, and only the second is what
        // reaches the release.
        assert!(
            WORKFLOW.contains("bpm-caddy-${{ matrix.suffix }}${{ matrix.ext }}"),
            "l'étape « Upload to release » n'envoie plus le binaire de l'application"
        );
        assert!(
            WORKFLOW.contains("bpm-caddy-launcher-${{ matrix.suffix }}${{ matrix.ext }}"),
            "l'étape « Upload to release » n'envoie plus le lanceur — \
             une officine qui installe pour la première fois n'a rien à télécharger"
        );
        // **The release appears whole or not at all**: the builds hand
        // their files to one final job, which refuses to publish without
        // every post binary. Each build used to publish alone, and the
        // release was « latest » for half an hour without this launcher's
        // file — which answered « pas de binaire » meanwhile.
        assert!(
            WORKFLOW.contains("needs: [build, android]")
                && WORKFLOW.contains("needs.build.result == 'success'")
                && WORKFLOW.contains("files: dist/*"),
            "la publication n'attend plus la fin de toutes les constructions"
        );
        assert_eq!(
            WORKFLOW.matches("softprops/action-gh-release@").count(),
            1,
            "une seule tâche publie"
        );
        assert!(
            WORKFLOW.contains(&format!("{} ", super::APP_ASSET))
                || WORKFLOW.contains(&format!("{} \\", super::APP_ASSET)),
            "{} n'est pas exigé avant la publication",
            super::APP_ASSET
        );
        // And the repository it asks: a fork left in there would send
        // every officine to somebody else's releases.
        assert!(
            WORKFLOW.contains("tags: [\"v*\"]"),
            "le workflow ne se déclenche plus sur un tag v*"
        );
        assert_eq!(super::REPO, "youssefsahli/bpm-caddy");
    }

    /// **Les versions se comparent comme des nombres, et jamais en
    /// arrière** : « 0.100 » vient après « 0.99 », et une publication
    /// retirée ne fait pas redescendre les postes.
    #[test]
    fn versions_compare_as_numbers() {
        assert!(super::version_parts("v0.100.0") > super::version_parts("v0.99.0"));
        assert!(super::version_parts("v0.257.0") > super::version_parts("0.256.3"));
        assert!(super::version_parts("v0.256.0") <= super::version_parts("v0.257.0"));
        assert!(super::version_parts("v0.1.0") > super::version_parts(""));
        // Une préversion avant sa finale, jamais après.
        assert!(super::version_parts("v0.405.0") > super::version_parts("v0.405.0-rc1"));
        assert!(super::version_parts("v0.405.0") > super::version_parts("v0.405.0-1"));
        assert!(super::version_parts("v0.405.0-rc1") > super::version_parts("v0.404.0"));
        assert_eq!(
            super::decide("v0.405.0", "v0.405.0-rc1", true),
            super::Decision::Install
        );
        assert_eq!(
            super::version_parts("0.405"),
            super::version_parts("v0.405.0")
        );
    }
}
