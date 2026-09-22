#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod agenda;
mod annuaire;
mod app;
mod audit;
mod biology;
mod bulletin;
mod caisse;
mod classes;
mod codebar;
mod codex;
mod conciliation;
mod config;
mod content;
mod crush;
mod cyp;
mod date;
mod db;
mod dosing;
mod elderly;
mod entretien;
mod facets;
mod fuzzy;
mod graph;
mod gravidity;
mod hepatic;
mod insulin;
mod intake;
mod location;
mod maintenance;
mod ordonnance;
mod ordonnancier;
mod pdf;
mod planning;
mod prescribers;
mod release;
mod renal;
mod renewal;
mod revue;
mod scans;
mod script;
mod selfcheck;
mod strings;
mod surveillance;
mod tables;
mod telemetry;
mod timeline;
mod vaccines;
mod vigilance;
mod vitale;
mod winscard;

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
    let Some(password) = std::env::var("BPM_CADDY_PASSWORD")
        .ok()
        .or_else(|| app::keyring_entry().and_then(|e| e.get_password().ok()))
    else {
        eprintln!("{}", audit::locked());
        return 2;
    };
    let path = cfg.db_path();
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
    let access = audit::summarize(&db.accesses_since(&from).unwrap_or_default());
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

fn main() -> eframe::Result {
    // Le relevé passe avant tout le reste : une fenêtre ne s'ouvre pas
    // pour répondre à une question posée depuis un terminal.
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("audit") {
        std::process::exit(run_audit(&args[1..]));
    }

    let options = eframe::NativeOptions {
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
        ..Default::default()
    };
    eframe::run_native(
        "BPM-Caddy",
        options,
        Box::new(|cc| {
            motif::apply(&cc.egui_ctx);
            // A font chosen in the options replaces the embedded family
            // for the whole interface; anything unreadable is ignored.
            if let Some(path) = config::Config::load().ui.font_path {
                if let Err(e) = install_font(&cc.egui_ctx, &path) {
                    eprintln!("police {} ignorée : {e}", path.display());
                }
            }
            Ok(Box::new(app::App::new()))
        }),
    )
}
