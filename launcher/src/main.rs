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

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
const APP_ASSET: &str = "bpm-caddy-linux-x86_64";
#[cfg(target_os = "windows")]
const APP_ASSET: &str = "bpm-caddy-windows-x86_64.exe";
#[cfg(target_os = "macos")]
const APP_ASSET: &str = "bpm-caddy-macos-arm64";

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

fn app_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("bpm-caddy")
}

fn bin_path() -> PathBuf {
    app_dir().join(format!("bpm-caddy{}", std::env::consts::EXE_SUFFIX))
}

fn version_file() -> PathBuf {
    app_dir().join("version.txt")
}

fn worker(shared: Arc<Shared>, ctx: egui::Context) {
    let outcome = check_and_update(&shared);
    let mut phase = shared.phase.lock().unwrap();
    *phase = match outcome {
        Ok(note) => Phase::ReadyToLaunch(note),
        Err(e) if bin_path().exists() => {
            Phase::ReadyToLaunch(format!("Mise à jour impossible ({e}) — version installée"))
        }
        Err(e) => Phase::Error(e.to_string()),
    };
    drop(phase);
    ctx.request_repaint();
}

fn check_and_update(shared: &Shared) -> Result<String, Box<dyn std::error::Error>> {
    // Timeouts matter here: without them a hung connection blocks the
    // pharmacy at startup even though an installed copy is ready to run.
    // The read timeout is per read() call, so slow-but-progressing
    // downloads are fine while genuine stalls fail fast.
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(30))
        .user_agent("bpm-caddy-launcher")
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

    let installed = std::fs::read_to_string(version_file()).unwrap_or_default();
    let bin = bin_path();
    if installed.trim() == tag && bin.exists() {
        return Ok(format!("À jour ({tag})"));
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

    let resp = agent.get(url).call()?;
    let total: u64 = resp
        .header("Content-Length")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    shared.total.store(total, Ordering::Relaxed);
    shared.downloaded.store(0, Ordering::Relaxed);

    std::fs::create_dir_all(app_dir())?;
    let tmp = bin.with_extension("part");
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
    file.flush()?;
    drop(file);

    // Never install a truncated binary (a cleanly closed connection can
    // end a download early without any read error).
    if expected_size > 0 && written != expected_size {
        let _ = std::fs::remove_file(&tmp);
        return Err(
            format!("téléchargement incomplet ({written} / {expected_size} octets)").into(),
        );
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755))?;
    }
    std::fs::rename(&tmp, &bin)?;
    std::fs::write(version_file(), &tag)?;

    Ok(format!("Mise à jour vers {tag} effectuée"))
}

struct Launcher {
    shared: Arc<Shared>,
    launched: bool,
}

impl Launcher {
    fn new(ctx: &egui::Context) -> Self {
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
        std::thread::spawn(move || worker(shared, ctx));
    }

    fn launch_app(&mut self, ctx: &egui::Context) {
        match Command::new(bin_path()).spawn() {
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
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([420.0, 240.0])
            .with_resizable(false)
            .with_icon(motif::icon())
            .with_title("BPM-Caddy — Lanceur"),
        ..Default::default()
    };
    eframe::run_native(
        "BPM-Caddy Launcher",
        options,
        Box::new(|cc| Ok(Box::new(Launcher::new(&cc.egui_ctx)))),
    )
}

#[cfg(test)]
mod tests {
    /// The release workflow, read at compile time. It is the other half
    /// of this file's only external contract.
    const WORKFLOW: &str = include_str!("../../.github/workflows/release.yml");

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
        // And the repository it asks: a fork left in there would send
        // every officine to somebody else's releases.
        assert!(
            WORKFLOW.contains("tags: [\"v*\"]"),
            "le workflow ne se déclenche plus sur un tag v*"
        );
        assert_eq!(super::REPO, "youssefsahli/bpm-caddy");
    }
}
