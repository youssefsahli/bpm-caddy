#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use bpm_caddy::{app, audit, config, date, db, strings};
use eframe::egui;

/// The size to open at: the screenshot/e2e hook
/// (`BPM_CADDY_WINDOW=1280x1200`) first, then the size the workspace
/// was left at, then a default that fits a counter screen.
fn window_size() -> [f32; 2] {
    let default = config::Layout::load().window().unwrap_or([1024.0, 700.0]);
    let Ok(spec) = std::env::var("BPM_CADDY_WINDOW") else {
        return default;
    };
    let Some((w, h)) = spec.split_once(['x', 'X']) else {
        return default;
    };
    match (w.trim().parse::<f32>(), h.trim().parse::<f32>()) {
        (Ok(w), Ok(h)) if w >= 640.0 && h >= 480.0 => [w, h],
        _ => default,
    }
}

/// Load a TrueType/OpenType file and make it the default family.
fn install_font(ctx: &egui::Context, path: &std::path::Path) -> Result<(), String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    // egui panics on a file it cannot parse, and the path lives in
    // config.toml — a mistyped one would make the app uncloseable.
    // Parse it here first and simply keep the embedded family instead.
    ab_glyph::FontRef::try_from_slice(&bytes)
        .map_err(|e| format!("fichier de police illisible : {e}"))?;
    let mut fonts = egui::FontDefinitions::default();
    fonts
        .font_data
        .insert("custom".to_owned(), egui::FontData::from_owned(bytes));
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        fonts
            .families
            .entry(family)
            .or_default()
            .insert(0, "custom".to_owned());
    }
    ctx.set_fonts(fonts);
    Ok(())
}

/// `bpm-caddy audit` — le relevé de décision, sur la sortie standard.
///
/// **Un mode de ce binaire et non un second programme**, et c'est la
/// seule forme défendable : un outil séparé devrait connaître le schéma
/// de la base, et deux écritures d'un schéma finissent par différer —
/// le jour où elles diffèrent, c'est celle que personne ne fait tourner
/// qui a l'air juste.
///
/// Il ne demande rien : le mot de passe vient de `BPM_CADDY_PASSWORD`
/// ou du trousseau du système, parce qu'il est fait pour tourner depuis
/// une tâche, la nuit, sur le poste de l'arrière-boutique.
///
/// Ailleurs que sous Linux, la commande existe et dit qu'elle ne tourne
/// pas ici : une commande qui répond « pas sur cette machine » se
/// diagnostique, une commande qui n'existe pas et répond « usage » en
/// listant autre chose ne se diagnostique pas.
///
/// Rend le code de sortie : 0 s'il a écrit un rapport, 2 sinon.
fn run_audit(args: &[String]) -> i32 {
    // Linux seulement, et vérifié **ici** plutôt que par deux corps de
    // fonction sous `cfg` : la seconde moitié serait du code mort sur
    // la plateforme où l'on compile, et le reste serait du code mort
    // sur les deux autres. `cfg!` est une constante, le compilateur
    // enlève ce qu'il faut, et il n'y a qu'une fonction à lire.
    if !cfg!(target_os = "linux") {
        eprintln!("{}", audit::linux_only());
        return 2;
    }
    // Un seul drapeau, et une valeur qu'on ne devine pas : « --jours »
    // sans nombre est une question mal posée, pas trente jours.
    let mut days: i64 = 30;
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            // La convention du terminal : demander l'usage n'est pas se
            // tromper, donc il sort sur la sortie standard et le
            // programme rend zéro.
            "--aide" | "--help" | "-h" => {
                println!("{}", audit::usage());
                return 0;
            }
            "--jours" | "--days" => match rest.next().and_then(|n| n.parse::<i64>().ok()) {
                Some(n) if n >= 1 => days = n,
                _ => {
                    eprintln!("{}", audit::usage());
                    return 2;
                }
            },
            _ => {
                eprintln!("{}", audit::usage());
                return 2;
            }
        }
    }

    let cfg = config::Config::load();
    let Some(password) =
        app::env_password().or_else(|| app::keyring_entry().and_then(|e| e.get_password().ok()))
    else {
        eprintln!("{}", audit::locked());
        return 2;
    };
    let path = cfg.db_path();
    if let Err(why) = audit::may_open(&path, &password, cfg.unreadable.as_deref()) {
        eprintln!("{why}");
        return 2;
    }
    let db = match db::Db::open(&path, &password) {
        Ok(db) => db,
        Err(e) => {
            eprintln!("{}", audit::unreadable(&e));
            return 2;
        }
    };

    let today = db.today_iso().unwrap_or_default();
    let from = date::add_days(&today, -(days - 1)).unwrap_or_else(|| today.clone());
    let head = audit::Head {
        // Le nom de l'officine vit **dans la base** depuis qu'il vaut
        // pour tous les postes ; `config.toml` n'en est plus que la
        // graine. Le lire là d'abord, sans quoi un rapport tiré depuis
        // l'arrière-boutique s'intitulerait au nom que ce poste-là
        // avait tapé, ou à rien.
        officine: db
            .officine()
            .map(|o| o.name)
            .filter(|n| !n.trim().is_empty())
            .unwrap_or_else(|| cfg.pharmacy.name.clone()),
        base: path.display().to_string(),
        made_on: db::format_french_date(&today),
        from: db::format_french_date(&from),
        to: db::format_french_date(&today),
        days: days as usize,
    };
    let activity = db.audit_activity(&from, &today).unwrap_or_default();
    // Une lecture qui échoue n'est pas un journal vide : elle se dit.
    let access = match db.accesses_since(&from, &today) {
        Ok(lines) => audit::summarize(&lines),
        Err(e) => {
            eprintln!("{}", audit::unreadable(&e));
            return 2;
        }
    };
    let conformity = db
        .audit_conformity(
            &today,
            i64::from(cfg.stock.count_days),
            cfg.locations.notice_days,
        )
        .unwrap_or_default();
    print!("{}", audit::render(&head, &activity, &access, &conformity));
    0
}

/// Le drapeau du lancement à l'ouverture de session : la fenêtre ne
/// s'ouvre pas, l'application attend dans la zone de notification — si
/// la base s'est ouverte seule (mot de passe mémorisé) et qu'il y a une
/// zone de notification. Sinon, la fenêtre, comme d'habitude.
const BACKGROUND: &str = "--arriere-plan";

/// Les crochets des passes automatiques (`smoke.sh`, `eyeball.sh`,
/// `fuzz.sh`) : sous eux, ni icône, ni seconde copie détournée, ni
/// lancement à l'ouverture de session écrit — une capture d'écran ne
/// touche pas au bureau de celui qui la lance.
fn hooked() -> bool {
    ["BPM_CADDY_START_VIEW", "BPM_CADDY_FUZZ", "BPM_CADDY_DB"]
        .iter()
        .any(|k| std::env::var_os(k).is_some())
}

/// **Une seule copie par session**, quand elle est en service : une
/// seconde copie lancée (un double clic sur le raccourci, le lancement à
/// l'ouverture de session d'une copie déjà ouverte) demande à la
/// première de se montrer et s'arrête. Par une porte sur `127.0.0.1`,
/// dont le numéro et un jeton tiré au lancement sont écrits à côté de
/// `config.toml` : sans le jeton, la porte ne fait rien.
mod instance {
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
    use std::path::{Path, PathBuf};
    use std::sync::mpsc::Sender;
    use std::sync::Arc;
    use std::time::Duration;

    /// Le mot qui suit le jeton quand on ne demande pas la fenêtre.
    const QUIET: &str = "present";

    pub fn file() -> PathBuf {
        bpm_caddy::config::Config::path().with_file_name("instance")
    }

    /// 128 bits tirés des clés de hachage du système (`RandomState` les
    /// tire du générateur de l'OS) : sans dépendance, et le binaire sans
    /// réseau (`--features desktop`) l'a aussi.
    fn token() -> String {
        use std::hash::{BuildHasher, Hasher};
        let half = || {
            let mut h = std::collections::hash_map::RandomState::new().build_hasher();
            h.write_u64(u64::from(std::process::id()));
            h.finish()
        };
        format!("{:016x}{:016x}", half(), half())
    }

    /// Le fichier : « port jeton ».
    fn read(file: &Path) -> Option<(u16, String)> {
        let text = std::fs::read_to_string(file).ok()?;
        let mut it = text.split_whitespace();
        let port = it.next()?.parse().ok()?;
        let token = it.next()?.to_owned();
        Some((port, token))
    }

    /// Demander à la copie qui tourne de se montrer — ou, `quiet`, seulement
    /// si elle est là (un second lancement à l'ouverture de session ne la
    /// montre pas). Vrai si elle a répondu.
    pub fn call(file: &Path, quiet: bool) -> bool {
        let Some((port, token)) = read(file) else {
            return false;
        };
        let at = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
        let Ok(mut s) = TcpStream::connect_timeout(&at, Duration::from_secs(1)) else {
            return false;
        };
        let _ = s.set_read_timeout(Some(Duration::from_secs(3)));
        let line = if quiet {
            format!("{token} {QUIET}")
        } else {
            token
        };
        if writeln!(s, "{line}").is_err() {
            return false;
        }
        let mut answer = String::new();
        let _ = BufReader::new(s).take(64).read_line(&mut answer);
        answer.trim() == "ouvert"
    }

    /// La porte de cette copie, tant qu'elle vit.
    pub struct Instance {
        file: PathBuf,
        token: String,
    }

    pub fn serve(
        file: PathBuf,
        tx: Sender<bpm_tray::Event>,
        wake: Arc<dyn Fn() + Send + Sync>,
    ) -> Option<Instance> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).ok()?;
        let port = listener.local_addr().ok()?.port();
        let token = token();
        write_private(&file, &format!("{port} {token}\n")).ok()?;
        let expected = token.clone();
        std::thread::Builder::new()
            .name("bpm-instance".into())
            .spawn(move || {
                for stream in listener.incoming().flatten() {
                    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
                    let mut line = String::new();
                    let Ok(reader) = stream.try_clone() else {
                        continue;
                    };
                    let _ = BufReader::new(reader).take(128).read_line(&mut line);
                    let mut words = line.split_whitespace();
                    if words.next() == Some(expected.as_str()) {
                        if words.next() != Some(QUIET) {
                            let _ = tx.send(bpm_tray::Event::Open);
                            wake();
                        }
                        let _ = (&stream).write_all(b"ouvert\n");
                    }
                }
            })
            .ok()?;
        Some(Instance { file, token })
    }

    #[cfg(unix)]
    fn write_private(file: &Path, text: &str) -> std::io::Result<()> {
        use std::os::unix::fs::OpenOptionsExt;
        if let Some(dir) = file.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(file)?;
        f.write_all(text.as_bytes())
    }

    #[cfg(not(unix))]
    fn write_private(file: &Path, text: &str) -> std::io::Result<()> {
        if let Some(dir) = file.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(file, text)
    }

    impl Drop for Instance {
        /// Le fichier n'est retiré que s'il est encore le sien : une
        /// autre copie a pu le réécrire depuis.
        fn drop(&mut self) {
            if read(&self.file).is_some_and(|(_, t)| t == self.token) {
                let _ = std::fs::remove_file(&self.file);
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn a_second_copy_reaches_the_first_with_its_token_only() {
            let dir = std::env::temp_dir().join(format!("bpm-instance-{}", std::process::id()));
            let file = dir.join("instance");
            let (tx, rx) = std::sync::mpsc::channel();
            let held = serve(file.clone(), tx, Arc::new(|| {})).expect("serve");
            assert!(call(&file, false));
            assert_eq!(
                rx.recv_timeout(Duration::from_secs(2)),
                Ok(bpm_tray::Event::Open)
            );
            // Discret : elle répond, et ne se montre pas.
            assert!(call(&file, true));
            assert!(rx.recv_timeout(Duration::from_millis(300)).is_err());
            // Le bon port, un autre jeton : rien.
            let (port, _) = read(&file).unwrap();
            let forged = dir.join("forged");
            std::fs::write(&forged, format!("{port} 0000\n")).unwrap();
            assert!(!call(&forged, false));
            assert!(rx.recv_timeout(Duration::from_millis(300)).is_err());
            drop(held);
            assert!(!file.exists());
            assert!(!call(&file, false));
            let _ = std::fs::remove_dir_all(&dir);
        }
    }
}

/// Ce qui mène l'application de l'extérieur de sa fenêtre : l'icône de
/// la zone de notification, une seconde copie lancée, le lancement à
/// l'ouverture de session.
struct Desk {
    events: std::sync::mpsc::Receiver<bpm_tray::Event>,
    tx: std::sync::mpsc::Sender<bpm_tray::Event>,
    /// Le contexte de la fenêtre ouverte, pour la réveiller.
    ctx: std::sync::Arc<std::sync::Mutex<Option<egui::Context>>>,
    tray: Option<(bpm_tray::Tray, Option<instance::Instance>)>,
    /// La dernière fois qu'une icône n'a pas pu se poser.
    tray_failed: Option<std::time::Instant>,
    status: String,
    /// La marque de l'icône telle qu'elle est posée.
    attention: bool,
    autostart: Option<bool>,
    hooked: bool,
}

impl Desk {
    fn new() -> Self {
        let (tx, events) = std::sync::mpsc::channel();
        Desk {
            events,
            tx,
            ctx: Default::default(),
            tray: None,
            tray_failed: None,
            status: String::new(),
            attention: false,
            autostart: None,
            hooked: hooked(),
        }
    }

    fn wake(&self) -> std::sync::Arc<dyn Fn() + Send + Sync> {
        let slot = std::sync::Arc::clone(&self.ctx);
        std::sync::Arc::new(move || {
            if let Some(ctx) = slot.lock().ok().and_then(|g| g.clone()) {
                ctx.request_repaint();
            }
        })
    }

    /// Une icône posée et une application en service : fermer la fenêtre
    /// la range, au lieu de quitter.
    fn can_hide(&self, app: &app::App) -> bool {
        self.tray.is_some() && app.in_service()
    }

    /// Suivre ce que veut le poste : l'icône, son infobulle, le lancement
    /// à l'ouverture de session. Appelé à chaque image et à chaque
    /// seconde fenêtre fermée ; ne touche au système que sur un changement.
    fn follow(&mut self, app: &app::App) {
        if self.hooked {
            return;
        }
        let wanted = app.keeps_service();
        if !wanted {
            self.tray = None;
        } else if self.tray.is_none()
            && self
                .tray_failed
                .is_none_or(|t| t.elapsed() > std::time::Duration::from_secs(60))
        {
            let picture = |icon: egui::IconData| bpm_tray::Image {
                width: icon.width,
                height: icon.height,
                rgba: icon.rgba,
            };
            let labels = bpm_tray::Labels {
                title: "BPM-Caddy".to_owned(),
                open: strings::tr("tray_open").to_owned(),
                quit: strings::tr("tray_quit").to_owned(),
            };
            match bpm_tray::Tray::spawn(
                picture(motif::icon()),
                picture(motif::icon_attention()),
                labels,
                self.tx.clone(),
                self.wake(),
            ) {
                Some(tray) => {
                    let door = instance::serve(instance::file(), self.tx.clone(), self.wake());
                    self.tray = Some((tray, door));
                    self.status.clear();
                    self.attention = false;
                    self.tray_failed = None;
                }
                None => self.tray_failed = Some(std::time::Instant::now()),
            }
        }
        if let Some((tray, _)) = &self.tray {
            let status = app.tray_status();
            if status != self.status {
                tray.set_status(&status);
                self.status = status;
            }
            let attention = app.tray_attention();
            if attention != self.attention {
                tray.set_attention(attention);
                self.attention = attention;
            }
        }
        let auto = app.wants_autostart();
        if self.autostart != Some(auto) {
            self.autostart = Some(auto);
            // Une copie de développement ne s'inscrit pas au démarrage.
            if !cfg!(debug_assertions) {
                if let Err(e) = apply_autostart(auto) {
                    eprintln!("lancement à l'ouverture de session : {e}");
                }
            }
        }
    }
}

/// Poser ou retirer le lancement à l'ouverture de session. **Le lanceur
/// d'abord** quand c'est lui qui a lancé cette copie (il le dit par
/// `BPM_CADDY_LAUNCHER`) : il met à jour avant d'ouvrir, ce que le poste
/// resté allumé des semaines ne ferait jamais autrement. Une entrée qui
/// nomme le lanceur n'est pas remplacée par une copie lancée sans lui.
fn apply_autostart(on: bool) -> Result<(), String> {
    use bpm_tray::autostart;
    if !on {
        return autostart::disable();
    }
    // Décoché dans les réglages de démarrage du bureau : son choix tient.
    if autostart::disabled_by_desktop() {
        return Ok(());
    }
    let launcher = std::env::var_os("BPM_CADDY_LAUNCHER").map(std::path::PathBuf::from);
    let program = match &launcher {
        Some(l) => l.clone(),
        None => std::env::current_exe().map_err(|e| e.to_string())?,
    };
    let command = autostart::Command {
        program,
        args: vec![BACKGROUND.to_owned()],
    };
    match autostart::current() {
        Some(now) if now == autostart::render(&command) => Ok(()),
        Some(now) if launcher.is_none() && now.contains("launcher") => Ok(()),
        _ => autostart::enable(&command),
    }
}

/// Comment s'est fermée la fenêtre.
#[derive(Clone, Copy, PartialEq, Eq)]
enum End {
    Quit,
    Tray,
}

/// La fenêtre, prêtée à eframe le temps qu'elle est ouverte : l'
/// application lui survit, dans la zone de notification.
struct Window<'a> {
    app: &'a mut app::App,
    desk: &'a mut Desk,
    end: &'a mut End,
    quitting: bool,
}

impl eframe::App for Window<'_> {
    fn raw_input_hook(&mut self, ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        self.app.raw_input_hook(ctx, raw_input);
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        while let Ok(event) = self.desk.events.try_recv() {
            match event {
                bpm_tray::Event::Open => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                }
                bpm_tray::Event::Quit => {
                    self.quitting = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        }
        // Décidé au moment de la demande : l'icône peut avoir disparu
        // depuis l'ouverture, et « Quitter » au menu de l'icône quitte.
        if ctx.input(|i| i.viewport().close_requested()) {
            *self.end = if !self.quitting && self.desk.can_hide(self.app) {
                End::Tray
            } else {
                End::Quit
            };
        }
        self.app.update(ctx, frame);
        self.desk.follow(self.app);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if let Ok(mut slot) = self.desk.ctx.lock() {
            *slot = None;
        }
        if *self.end == End::Tray {
            self.app.hidden_to_tray();
        }
    }
}

fn native_options() -> eframe::NativeOptions {
    eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(window_size())
            .with_min_inner_size([960.0, 640.0])
            // **La fenêtre maximisée est une forme, et aucune passe ne
            // la produisait.** `smoke.sh` et `eyeball.sh` tournent sous
            // un Xvfb sans gestionnaire de fenêtres : rien n'y est
            // jamais maximisé, et une demande de taille y est toujours
            // honorée. Sur un poste réel, c'est l'inverse — le
            // gestionnaire tient la taille d'une fenêtre maximisée et
            // jette la demande —, et c'est ainsi que F9 laissait la
            // fenêtre pleine au lieu d'en faire une barre. Le crochet
            // rend cette forme atteignable, comme `BPM_CADDY_WINDOW`
            // rend les autres.
            .with_maximized(std::env::var("BPM_CADDY_MAXIMIZED").is_ok())
            .with_icon(motif::icon())
            .with_title("BPM-Caddy"),
        // La fenêtre se ferme et rend la main : l'application peut lui
        // survivre (c'est le défaut d'eframe, écrit pour qu'on le lise).
        run_and_return: true,
        ..Default::default()
    }
}

/// Ouvrir la fenêtre sur `app`, jusqu'à ce qu'elle se ferme.
fn run_window(app: &mut app::App, desk: &mut Desk) -> Result<End, eframe::Error> {
    let mut end = End::Quit;
    eframe::run_native(
        "BPM-Caddy",
        native_options(),
        Box::new(|cc| {
            motif::apply(&cc.egui_ctx);
            // A font chosen in the options replaces the embedded family
            // for the whole interface; anything unreadable is ignored.
            if let Some(path) = config::Config::load().ui.font_path {
                if let Err(e) = install_font(&cc.egui_ctx, &path) {
                    eprintln!("police {} ignorée : {e}", path.display());
                }
            }
            if let Ok(mut slot) = desk.ctx.lock() {
                *slot = Some(cc.egui_ctx.clone());
            }
            app.window_opened();
            Ok(Box::new(Window {
                app,
                desk,
                end: &mut end,
                quitting: false,
            }))
        }),
    )?;
    Ok(end)
}

/// La fenêtre fermée, l'application en service : la tenir, toutes les
/// secondes, jusqu'à ce qu'on la rouvre ou la quitte.
fn wait_in_tray(app: &mut app::App, desk: &mut Desk) -> End {
    use std::sync::mpsc::RecvTimeoutError;
    // Un contexte sans fenêtre : ce qui demande une image n'en obtient
    // pas, et rien d'autre n'en dépend.
    let quiet = egui::Context::default();
    let mut next_tick = std::time::Instant::now();
    loop {
        // **Les messages de ce fil, sous Windows** : la boucle d'eframe
        // garde sur lui une fenêtre cachée, à qui le système demande
        // « puis-je fermer la session ? ». Sans réponse, la déconnexion
        // et l'arrêt du poste attendent BPM-Caddy.
        bpm_tray::pump_messages();
        let wait = next_tick.saturating_duration_since(std::time::Instant::now());
        match desk
            .events
            .recv_timeout(wait.min(std::time::Duration::from_millis(100)))
        {
            Ok(bpm_tray::Event::Open) | Err(RecvTimeoutError::Disconnected) => return End::Tray,
            Ok(bpm_tray::Event::Quit) => return End::Quit,
            Err(RecvTimeoutError::Timeout) if std::time::Instant::now() < next_tick => {}
            Err(RecvTimeoutError::Timeout) => {
                next_tick = std::time::Instant::now() + std::time::Duration::from_secs(1);
                app.background_tick(&quiet);
                desk.follow(app);
                // Plus d'icône (son hôte est parti pour de bon) ou plus
                // rien en service : la fenêtre revient plutôt que de
                // laisser tourner un programme qu'on ne peut plus atteindre.
                if !desk.can_hide(app) {
                    return End::Tray;
                }
            }
        }
    }
}

fn main() -> eframe::Result {
    // Lu et retiré de l'environnement avant tout fil et tout programme
    // lancé : voir `app::env_password`.
    let _ = app::env_password();
    // Le relevé passe avant tout le reste : une fenêtre ne s'ouvre pas
    // pour répondre à une question posée depuis un terminal.
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("audit") {
        std::process::exit(run_audit(&args[1..]));
    }
    let background = args.iter().any(|a| a == BACKGROUND);
    // Une copie déjà en service : elle se montre, celle-ci s'arrête.
    if !hooked() && instance::call(&instance::file(), background) {
        return Ok(());
    }

    let mut app = app::App::new();
    let mut desk = Desk::new();
    desk.follow(&app);
    let mut show = !(background && desk.can_hide(&app));
    loop {
        if show {
            match run_window(&mut app, &mut desk) {
                Ok(End::Quit) => break,
                Ok(End::Tray) => {}
                // La fenêtre ne s'ouvre pas (le pilote graphique) : quitter
                // proprement plutôt que tourner sans pouvoir se montrer.
                Err(e) => {
                    desk.tray = None;
                    app.shutdown();
                    return Err(e);
                }
            }
        }
        show = true;
        if wait_in_tray(&mut app, &mut desk) == End::Quit {
            break;
        }
    }
    desk.tray = None;
    // Un paquet en cours, attendu en répondant au système : sous Windows,
    // la fermeture de session attendrait sinon BPM-Caddy, puis le tuerait.
    app.finish_export(bpm_tray::pump_messages);
    app.shutdown();
    Ok(())
}
