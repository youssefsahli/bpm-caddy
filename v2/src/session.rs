//! An open base, and what every screen needs to know about it.
//!
//! The v2 window reads the very base the first window writes: same file,
//! same schema, same compare-and-set writes. It is a second face on one
//! store, never a second store.

use bpm_caddy::content::Overrides;
use bpm_caddy::db::{Db, Drug, Patient};
use std::path::{Path, PathBuf};

pub struct Session {
    pub db: Db,
    pub path: PathBuf,
    /// ISO `YYYY-MM-DD`, read from the base's clock once per minute.
    pub today: String,
    /// Initials of whoever works at this post (config `[ui] operator`).
    pub operator: String,
    /// The base's screen revision as last seen: when another post (or
    /// the first window) writes, it moves, and screens reload.
    rev: i64,
    /// Bumped by [`Session::changed`]: what screens memoise against.
    pub generation: u64,
    clock: Option<std::time::Instant>,
    /// One line for the status bar: what just happened.
    pub notice: Option<Notice>,
    /// Every file and every card, reloaded when `generation` moves.
    /// Public fields, so a screen can read them beside `db`.
    pub patients: Vec<Patient>,
    pub drugs: Vec<Drug>,
    /// The officine's rewrites of shipped sentences.
    pub overrides: Overrides,
    loaded: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Notice {
    pub text: String,
    pub bad: bool,
}

impl Session {
    /// Open the base and seed what a fresh one is missing, as the first
    /// window does — but only on the post that seeds (`seeds_here`): in
    /// a group of posts, two seeders would create every card twice.
    pub fn open(path: &Path, password: &str, operator: &str) -> Result<Self, String> {
        let db = Db::open(path, password)?;
        if db.seeds_here() {
            let _ = db.seed_drugs_if_empty();
            let _ = db.seed_preparations();
            let _ = db.seed_conduite();
            let _ = db.seed_protocols();
            let _ = db.seed_cascades();
            let _ = db.seed_dispositifs();
            let _ = db.seed_cno();
            let _ = db.refresh_reworded_fiches_once();
            let _ = db.seed_trod_lines();
            let _ = db.seed_vaccine_catalogue();
            let _ = db.refresh_vaccine_schedules();
            let _ = db.refresh_toxicity();
        }
        let today = db.today_iso().unwrap_or_default();
        let rev = db.screen_rev();
        let mut s = Session {
            db,
            path: path.to_path_buf(),
            today,
            operator: operator.trim().to_owned(),
            rev,
            generation: 1,
            clock: Some(std::time::Instant::now()),
            notice: None,
            patients: Vec::new(),
            drugs: Vec::new(),
            overrides: Overrides::default(),
            loaded: 0,
        };
        s.reload();
        Ok(s)
    }

    fn reload(&mut self) {
        if self.loaded == self.generation {
            return;
        }
        self.loaded = self.generation;
        match self.db.patients() {
            Ok(mut p) => {
                p.sort_by_cached_key(|p| {
                    bpm_caddy::fuzzy::sort_key(&format!("{} {}", p.last_name, p.first_name))
                });
                self.patients = p;
            }
            Err(e) => self.fail(e),
        }
        match self.db.drugs() {
            Ok(mut d) => {
                d.sort_by_cached_key(|d| bpm_caddy::fuzzy::sort_key(&d.name));
                self.drugs = d;
            }
            Err(e) => self.fail(e),
        }
        self.overrides = self.db.content_overrides().unwrap_or_default();
    }

    pub fn patient(&self, id: i64) -> Option<&Patient> {
        self.patients.iter().find(|p| p.id == id)
    }

    pub fn drug(&self, id: i64) -> Option<&Drug> {
        self.drugs.iter().find(|d| d.id == id)
    }

    /// Once per frame: notice another writer and a new day. Cheap — one
    /// `max()` over a one-row table, and the clock once a minute.
    pub fn tick(&mut self) {
        let rev = self.db.screen_rev();
        if rev != self.rev {
            self.rev = rev;
            self.generation += 1;
        }
        if self
            .clock
            .is_none_or(|t| t.elapsed() > std::time::Duration::from_secs(60))
        {
            self.clock = Some(std::time::Instant::now());
            if let Ok(day) = self.db.today_iso() {
                if day != self.today {
                    self.today = day;
                    self.generation += 1;
                }
            }
        }
        self.reload();
    }

    /// This window wrote: every screen reloads on the next frame.
    pub fn changed(&mut self) {
        self.rev = self.db.screen_rev();
        self.generation += 1;
        self.reload();
    }

    pub fn say(&mut self, text: impl Into<String>) {
        self.notice = Some(Notice {
            text: text.into(),
            bad: false,
        });
    }

    pub fn fail(&mut self, text: impl Into<String>) {
        self.notice = Some(Notice {
            text: text.into(),
            bad: true,
        });
    }

    /// A shared row changed under the screen between reading and
    /// writing: say so, reload everything.
    pub fn stale(&mut self) {
        self.fail(crate::t::t("app.stale"));
        self.changed();
    }

    /// Age in whole years on `self.today`, from an ISO birth date.
    pub fn age(&self, birth: &str) -> Option<u32> {
        age_on(birth, &self.today)
    }
}

/// Age in whole years on `today`, from an ISO birth date; `None` when
/// either date does not parse or the birth is in the future.
pub fn age_on(birth: &str, today: &str) -> Option<u32> {
    let (by, bm, bd) = bpm_caddy::date::parse_iso(birth)?;
    let (ty, tm, td) = bpm_caddy::date::parse_iso(today)?;
    let years = ty - by - i64::from((tm, td) < (bm, bd));
    u32::try_from(years).ok()
}

/// The password the screenshot / e2e hook passes, read once and taken
/// out of the environment so no child process inherits it.
pub fn env_password() -> Option<String> {
    static ONCE: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    ONCE.get_or_init(|| {
        let pw = std::env::var("BPM_CADDY_PASSWORD").ok();
        std::env::remove_var("BPM_CADDY_PASSWORD");
        pw
    })
    .clone()
}

/// Where the base is: `BPM_CADDY_DB` first, then config.toml, then the
/// default the first window uses.
pub fn base_path() -> PathBuf {
    if let Some(p) = std::env::var_os("BPM_CADDY_DB") {
        return PathBuf::from(p);
    }
    bpm_caddy::config::Config::load().db_path()
}

#[cfg(test)]
pub mod tests {
    use super::*;

    /// A fresh base in a directory of its own.
    pub fn scratch(name: &str) -> (Session, PathBuf) {
        let dir = std::env::temp_dir().join(format!(
            "bpm-v2-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let s = Session::open(&dir.join("base.db"), "test", "AB").unwrap();
        (s, dir)
    }

    #[test]
    fn age_counts_whole_years() {
        assert_eq!(age_on("1958-07-03", "2026-07-02"), Some(67));
        assert_eq!(age_on("1958-07-03", "2026-07-03"), Some(68));
        assert_eq!(age_on("2030-01-01", "2026-07-03"), None);
        assert_eq!(age_on("", "2026-07-03"), None);
    }

    #[test]
    fn a_fresh_base_opens_seeded_and_notices_its_own_writes() {
        let (mut s, dir) = scratch("open");
        assert!(!s.db.drugs().unwrap().is_empty(), "fiches de départ");
        assert_eq!(s.today.len(), 10);
        let g = s.generation;
        s.db.add_patient_with_sex("Durand", "Anne", "1950-01-01", "")
            .unwrap();
        s.tick();
        assert!(s.generation > g, "une écriture fait relire les écrans");
        let _ = std::fs::remove_dir_all(dir);
    }
}
