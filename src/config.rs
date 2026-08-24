//! `config.toml` in the platform config directory (spec 4.3): database
//! location (e.g. a shared pharmacy network drive), auto-lock timeout,
//! UI defaults, and billing fees.

use std::path::PathBuf;

use serde::Deserialize;

/// Written on first launch; every option commented out at its default.
const CONFIG_TEMPLATE: &str = r#"# BPM-Caddy — configuration (fichier créé au premier lancement).
# Décommentez et adaptez les options utiles ; les valeurs indiquées
# sont les valeurs par défaut.

[database]
# Chemin de la base chiffrée. Pointez vers le lecteur réseau de la
# pharmacie pour la partager entre postes (les notes d'équipe et les
# sauvegardes automatiques suivent la base).
# path = "Z:/LGO_Shared/bpm_caddy.db"
# Verrouillage automatique après inactivité (minutes, 0 = jamais).
# auto_lock_timeout_minutes = 15
# Nombre de sauvegardes quotidiennes conservées (0 = désactivées).
# backups_keep = 14

[ui]
# Afficher le panneau de documentation d'équipe au démarrage.
# show_docs_on_start = true
# Masquer les montants du tableau de bord (mode discret au comptoir).
# discreet_finances = true
# Initiales de l'opérateur par défaut pour les entrées de notes.
# operator = "CL"

[billing]
# Honoraires en euros par acte.
# bpm_fee = 60.0
# aod_fee = 40.0
# asthme_fee = 40.0
# trod_angine_fee = 10.0
# trod_cystite_fee = 12.0
# prevention_fee = 30.0

[templates]
# Modèles Typst personnalisés (fiche d'entretien et courrier CR).
# bpm_template_path = "templates/bpm_layout.typ"
# cr_template_path = "templates/cr_layout.typ"

[pharmacy]
# Identité de l'officine, pour l'en-tête du courrier au médecin.
# name = "Pharmacie du Centre"
# address = "1 place de la Mairie, 34000 Montpellier"
# phone = "04 67 00 00 00"
# pharmacist = "Dr Claire Leroy, pharmacien titulaire"
"#;

#[derive(Deserialize, Default, Clone)]
#[serde(default)]
pub struct Config {
    pub database: DatabaseConfig,
    pub ui: UiConfig,
    pub billing: BillingConfig,
    pub templates: TemplatesConfig,
    pub pharmacy: PharmacyConfig,
}

/// The pharmacy's identity, used on the CR letter to the médecin
/// traitant.
#[derive(Deserialize, Default, Clone)]
#[serde(default)]
pub struct PharmacyConfig {
    pub name: String,
    pub address: String,
    pub phone: String,
    /// Signing pharmacist ("Dr Claire Leroy, pharmacien titulaire").
    pub pharmacist: String,
}

/// Custom Typst templates; the embedded default is used when unset.
#[derive(Deserialize, Clone, Default)]
#[serde(default)]
pub struct TemplatesConfig {
    pub bpm_template_path: Option<PathBuf>,
    pub cr_template_path: Option<PathBuf>,
}

#[derive(Deserialize, Clone)]
#[serde(default)]
pub struct DatabaseConfig {
    /// Where the encrypted database lives. Point this at the pharmacy
    /// network drive (e.g. `Z:/LGO_Shared/bpm_caddy.db`) to share it.
    pub path: Option<PathBuf>,
    pub auto_lock_timeout_minutes: u64,
    /// How many daily snapshots to keep in `backups/`; 0 disables the
    /// automatic backups entirely.
    pub backups_keep: usize,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            path: None,
            auto_lock_timeout_minutes: 15,
            backups_keep: 14,
        }
    }
}

#[derive(Deserialize, Clone)]
#[serde(default)]
pub struct UiConfig {
    pub show_docs_on_start: bool,
    /// Mask revenue amounts on the dashboard until explicitly revealed,
    /// so figures are not readable over a shoulder at the counter.
    pub discreet_finances: bool,
    /// Default operator initials for note stamps (editable in the app).
    pub operator: String,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            show_docs_on_start: true,
            discreet_finances: true,
            operator: String::new(),
        }
    }
}

/// Fees in euros per act. Defaults are placeholders — adjust them in
/// `config.toml` to the convention currently in force.
#[derive(Deserialize, Clone)]
#[serde(default)]
pub struct BillingConfig {
    pub bpm_fee: f64,
    pub aod_fee: f64,
    pub asthme_fee: f64,
    pub trod_angine_fee: f64,
    pub trod_cystite_fee: f64,
    pub prevention_fee: f64,
}

impl Default for BillingConfig {
    fn default() -> Self {
        Self {
            bpm_fee: 60.0,
            aod_fee: 40.0,
            asthme_fee: 40.0,
            trod_angine_fee: 10.0,
            trod_cystite_fee: 12.0,
            prevention_fee: 30.0,
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
    /// missing or malformed (the app must always start). On first run a
    /// fully commented template is written so the options are
    /// discoverable without reading the documentation.
    pub fn load() -> Self {
        let path = Self::path();
        match std::fs::read_to_string(&path) {
            Ok(s) => toml::from_str(&s).unwrap_or_default(),
            Err(_) => {
                Self::write_template(&path);
                Self::default()
            }
        }
    }

    fn write_template(path: &std::path::Path) {
        if path.exists() {
            return;
        }
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(path, CONFIG_TEMPLATE);
    }

    pub fn db_path(&self) -> PathBuf {
        // Test/demo hook: BPM_CADDY_DB overrides everything.
        if let Ok(p) = std::env::var("BPM_CADDY_DB") {
            return PathBuf::from(p);
        }
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

    /// The Typst template for the interview sheet: the configured path,
    /// or the editable default next to `config.toml`. The embedded
    /// template is used when the file does not exist.
    pub fn template_path(&self) -> PathBuf {
        self.templates
            .bpm_template_path
            .clone()
            .unwrap_or_else(|| Self::path().with_file_name("bpm_layout.typ"))
    }

    /// The Typst template for the CR letter to the médecin traitant.
    pub fn cr_template_path(&self) -> PathBuf {
        self.templates
            .cr_template_path
            .clone()
            .unwrap_or_else(|| Self::path().with_file_name("cr_layout.typ"))
    }

    pub fn fee(&self, kind: crate::db::InterviewKind) -> f64 {
        match kind {
            crate::db::InterviewKind::Bpm => self.billing.bpm_fee,
            crate::db::InterviewKind::Aod => self.billing.aod_fee,
            crate::db::InterviewKind::Asthme => self.billing.asthme_fee,
            crate::db::InterviewKind::TrodAngine => self.billing.trod_angine_fee,
            crate::db::InterviewKind::TrodCystite => self.billing.trod_cystite_fee,
            crate::db::InterviewKind::Prevention => self.billing.prevention_fee,
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
        assert!(cfg.ui.discreet_finances);
    }

    #[test]
    fn first_run_template_parses_to_defaults() {
        let cfg: Config = toml::from_str(CONFIG_TEMPLATE).unwrap();
        assert!(cfg.database.path.is_none());
        assert_eq!(cfg.database.auto_lock_timeout_minutes, 15);
        assert_eq!(cfg.billing.bpm_fee, 60.0);
        assert!(cfg.templates.bpm_template_path.is_none());
    }
}
