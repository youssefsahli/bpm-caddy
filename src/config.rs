//! `config.toml` in the platform config directory (spec 4.3): database
//! location (e.g. a shared pharmacy network drive), auto-lock timeout,
//! UI defaults, and billing fees.

use std::path::PathBuf;

use serde::Deserialize;

#[derive(Deserialize, Default, Clone)]
#[serde(default)]
pub struct Config {
    pub database: DatabaseConfig,
    pub ui: UiConfig,
    pub billing: BillingConfig,
    pub templates: TemplatesConfig,
}

/// Custom Typst templates; the embedded default is used when unset.
#[derive(Deserialize, Clone, Default)]
#[serde(default)]
pub struct TemplatesConfig {
    pub bpm_template_path: Option<PathBuf>,
}

#[derive(Deserialize, Clone)]
#[serde(default)]
pub struct DatabaseConfig {
    /// Where the encrypted database lives. Point this at the pharmacy
    /// network drive (e.g. `Z:/LGO_Shared/bpm_caddy.db`) to share it.
    pub path: Option<PathBuf>,
    pub auto_lock_timeout_minutes: u64,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            path: None,
            auto_lock_timeout_minutes: 15,
        }
    }
}

#[derive(Deserialize, Clone)]
#[serde(default)]
pub struct UiConfig {
    pub show_docs_on_start: bool,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            show_docs_on_start: true,
        }
    }
}

/// Fees in euros per interview cycle. Defaults are placeholders — adjust
/// them in `config.toml` to the convention currently in force.
#[derive(Deserialize, Clone)]
#[serde(default)]
pub struct BillingConfig {
    pub bpm_fee: f64,
    pub aod_fee: f64,
    pub asthme_fee: f64,
}

impl Default for BillingConfig {
    fn default() -> Self {
        Self {
            bpm_fee: 60.0,
            aod_fee: 40.0,
            asthme_fee: 40.0,
        }
    }
}

impl Config {
    pub fn path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("bpm-caddy")
            .join("config.toml")
    }

    /// Load the configuration, falling back to defaults if the file is
    /// missing or malformed (the app must always start).
    pub fn load() -> Self {
        std::fs::read_to_string(Self::path())
            .ok()
            .and_then(|s| toml::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn db_path(&self) -> PathBuf {
        self.database
            .path
            .clone()
            .unwrap_or_else(crate::db::default_path)
    }

    /// The shared team documentation sits next to the database, so pointing
    /// the database at the network drive shares the notes too.
    pub fn team_doc_path(&self) -> PathBuf {
        self.db_path()
            .parent()
            .map(|p| p.join("notes_equipe.md"))
            .unwrap_or_else(|| PathBuf::from("notes_equipe.md"))
    }

    pub fn fee(&self, kind: crate::db::InterviewKind) -> f64 {
        match kind {
            crate::db::InterviewKind::Bpm => self.billing.bpm_fee,
            crate::db::InterviewKind::Aod => self.billing.aod_fee,
            crate::db::InterviewKind::Asthme => self.billing.asthme_fee,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_spec_example() {
        let cfg: Config = toml::from_str(
            r#"
            [database]
            path = "Z:/LGO_Shared/bpm_caddy.db"
            auto_lock_timeout_minutes = 5

            [ui]
            show_docs_on_start = false

            [billing]
            bpm_fee = 55.5
            "#,
        )
        .unwrap();
        assert_eq!(cfg.database.auto_lock_timeout_minutes, 5);
        assert!(!cfg.ui.show_docs_on_start);
        assert_eq!(cfg.billing.bpm_fee, 55.5);
        // Unset fields keep their defaults.
        assert_eq!(cfg.billing.aod_fee, 40.0);
        assert!(cfg.team_doc_path().ends_with("notes_equipe.md"));
    }

    #[test]
    fn empty_config_is_all_defaults() {
        let cfg: Config = toml::from_str("").unwrap();
        assert_eq!(cfg.database.auto_lock_timeout_minutes, 15);
        assert!(cfg.ui.show_docs_on_start);
    }
}
