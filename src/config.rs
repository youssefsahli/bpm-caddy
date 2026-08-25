//! `config.toml` in the platform config directory (spec 4.3): database
//! location (e.g. a shared pharmacy network drive), auto-lock timeout,
//! UI defaults, and billing fees.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

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
# Échelle du texte (1.0 = taille de référence).
# text_scale = 1.0
# Densité de l'interface : "confortable" ou "compact".
# density = "confortable"
# Pictogrammes dans la barre d'outils.
# icons = false
# Police de l'interface (fichier .ttf ou .otf ; vide = police intégrée).
# font_path = "C:/Windows/Fonts/segoeui.ttf"
# Contenu du panneau de droite au démarrage : "docs", "carnet" ou "notes".
# side_pane = "docs"
# Masquer les montants du tableau de bord (mode discret au comptoir).
# discreet_finances = true
# Initiales de l'opérateur par défaut pour les entrées de notes.
# operator = "CL"

[billing]
# Honoraires en euros, par acte et par rang dans l'année
# d'accompagnement : entretien initial / 1er suivi / 2e suivi (et
# au-delà). La forme simple `bpm = 60.0` applique le même tarif aux
# trois rangs.
# bpm = { initial = 60.0, suivi_1 = 20.0, suivi_2 = 20.0 }
# aod = { initial = 40.0, suivi_1 = 20.0, suivi_2 = 20.0 }
# avk = { initial = 40.0, suivi_1 = 20.0, suivi_2 = 20.0 }
# asthme = { initial = 40.0, suivi_1 = 20.0, suivi_2 = 20.0 }
# anticancereux = { initial = 60.0, suivi_1 = 20.0, suivi_2 = 20.0 }
# trod_angine = 10.0
# trod_cystite = 12.0
# vaccination = 10.0
# prevention = 30.0

[templates]
# Modèles Typst personnalisés (fiche d'entretien et courrier CR).
# bpm_template_path = "templates/bpm_layout.typ"
# cr_template_path = "templates/cr_layout.typ"
# carnet_template_path = "templates/carnet_layout.typ"

[pharmacy]
# Identité de l'officine, pour l'en-tête du courrier au médecin.
# name = "Pharmacie du Centre"
# address = "1 place de la Mairie, 34000 Montpellier"
# phone = "04 67 00 00 00"
# pharmacist = "Dr Claire Leroy, pharmacien titulaire"

[rules]
# Nombre maximal d'actes par année d'accompagnement (cycle glissant à
# partir du premier acte ; 0 = sans limite).
# Durée du cycle en mois (12 = année d'accompagnement conventionnelle).
# cycle_months = 12
# Comportement quand le quota est atteint : "warn" (message + création
# forçable), "inform" (simple information) ou "block" (création refusée).
# enforcement = "warn"
# bpm_per_year = 3
# aod_per_year = 3
# asthme_per_year = 3
# trod_angine_per_year = 0
# trod_cystite_per_year = 0
# prevention_per_year = 1
# avk_per_year = 3
# anticancereux_per_year = 3
# vaccination_per_year = 0
"#;

#[derive(Deserialize, Serialize, Default, Clone)]
#[serde(default)]
pub struct Config {
    pub database: DatabaseConfig,
    pub ui: UiConfig,
    pub billing: BillingConfig,
    pub templates: TemplatesConfig,
    pub pharmacy: PharmacyConfig,
    pub rules: RulesConfig,
}

/// Convention rules: how many acts of each kind per "année
/// d'accompagnement" (12 months from the cycle's first act; the next
/// cycle starts at least 12 months later). 0 disables the rule.
/// What the app does when an act falls inside the running cycle's
/// quota window.
#[derive(Deserialize, Serialize, Clone, Copy, PartialEq, Debug, Default)]
#[serde(rename_all = "snake_case")]
pub enum RuleEnforcement {
    /// State the rule, and let the act be created after an explicit
    /// confirmation (the historical behaviour).
    #[default]
    Warn,
    /// State the rule as information only; creation is not interrupted.
    Inform,
    /// Refuse the creation outright — no override button.
    Block,
}

#[derive(Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct RulesConfig {
    /// Months between the first act of a cycle and the first act of the
    /// next one (the convention's "année d'accompagnement").
    pub cycle_months: u32,
    /// How a blocked creation is handled.
    pub enforcement: RuleEnforcement,
    pub bpm_per_year: u32,
    pub aod_per_year: u32,
    pub asthme_per_year: u32,
    pub trod_angine_per_year: u32,
    pub trod_cystite_per_year: u32,
    pub prevention_per_year: u32,
    pub avk_per_year: u32,
    pub anticancereux_per_year: u32,
    pub vaccination_per_year: u32,
}

impl Default for RulesConfig {
    fn default() -> Self {
        Self {
            cycle_months: 12,
            enforcement: RuleEnforcement::Warn,
            bpm_per_year: 3,
            aod_per_year: 3,
            asthme_per_year: 3,
            trod_angine_per_year: 0,
            trod_cystite_per_year: 0,
            prevention_per_year: 1,
            avk_per_year: 3,
            anticancereux_per_year: 3,
            vaccination_per_year: 0,
        }
    }
}

/// The pharmacy's identity, used on the CR letter to the médecin
/// traitant.
#[derive(Deserialize, Serialize, Default, Clone)]
#[serde(default)]
pub struct PharmacyConfig {
    pub name: String,
    pub address: String,
    pub phone: String,
    /// Signing pharmacist ("Dr Claire Leroy, pharmacien titulaire").
    pub pharmacist: String,
}

/// Custom Typst templates; the embedded default is used when unset.
#[derive(Deserialize, Serialize, Clone, Default)]
#[serde(default)]
pub struct TemplatesConfig {
    pub bpm_template_path: Option<PathBuf>,
    pub cr_template_path: Option<PathBuf>,
    pub carnet_template_path: Option<PathBuf>,
}

#[derive(Deserialize, Serialize, Clone)]
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

#[derive(Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct UiConfig {
    pub show_docs_on_start: bool,
    /// Text scale, 1.0 being the design size. The counter screen is
    /// often far from the eye; the tablet is often small.
    pub text_scale: f32,
    /// "confortable" (default) or "compact": how much the interface
    /// spends on padding.
    pub density: String,
    /// Draw the small pictograms next to the toolbar labels.
    pub icons: bool,
    /// A TrueType file to use for the whole interface. Empty keeps the
    /// embedded family; a bad path falls back to it too.
    pub font_path: Option<PathBuf>,
    /// Which content the right pane shows on start: "docs", "carnet"
    /// or "notes".
    pub side_pane: String,
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
            text_scale: 1.0,
            density: "confortable".to_owned(),
            icons: false,
            font_path: None,
            side_pane: "docs".to_owned(),
            discreet_finances: true,
            operator: String::new(),
        }
    }
}

/// The fee schedule of one act kind: the convention pays the entretien
/// initial, the 1er suivi and the 2e suivi (and beyond) of an année
/// d'accompagnement differently.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
pub struct ActFees {
    pub initial: f64,
    pub suivi_1: f64,
    pub suivi_2: f64,
}

impl ActFees {
    pub const fn flat(v: f64) -> Self {
        Self {
            initial: v,
            suivi_1: v,
            suivi_2: v,
        }
    }

    pub const fn staged(initial: f64, suivi: f64) -> Self {
        Self {
            initial,
            suivi_1: suivi,
            suivi_2: suivi,
        }
    }

    /// Fee for the act ranked `rank` (0-based) inside its yearly cycle.
    /// Ranks beyond the third are billed at the last rate.
    pub fn for_rank(&self, rank: usize) -> f64 {
        match rank {
            0 => self.initial,
            1 => self.suivi_1,
            _ => self.suivi_2,
        }
    }

    /// Mutable access by rank, for the Options grid — which shows one
    /// column per act allowed in the year.
    pub fn slot_mut(&mut self, rank: usize) -> &mut f64 {
        match rank {
            0 => &mut self.initial,
            1 => &mut self.suivi_1,
            _ => &mut self.suivi_2,
        }
    }

    /// How many distinct rates this schedule can express.
    pub const SLOTS: usize = 3;
}

/// One fee entry as written in `config.toml`: either the flat number
/// of the older format (`bpm_fee = 60.0`), or a table where each rank
/// is optional (`bpm = { initial = 65.0 }`). Slots left out — and keys
/// that are misspelt, which serde ignores — keep the default fee of
/// that act rather than silently becoming 0 €.
#[derive(Deserialize)]
#[serde(untagged)]
enum FeesRepr {
    Flat(f64),
    Table {
        #[serde(default)]
        initial: Option<f64>,
        #[serde(default)]
        suivi_1: Option<f64>,
        #[serde(default)]
        suivi_2: Option<f64>,
    },
}

impl FeesRepr {
    /// Apply what the file specified on top of the act's default fees.
    fn merge(this: Option<Self>, default: ActFees) -> ActFees {
        match this {
            None => default,
            Some(Self::Flat(v)) => ActFees::flat(v),
            Some(Self::Table {
                initial,
                suivi_1,
                suivi_2,
            }) => ActFees {
                initial: initial.unwrap_or(default.initial),
                suivi_1: suivi_1.unwrap_or(default.suivi_1),
                suivi_2: suivi_2.unwrap_or(default.suivi_2),
            },
        }
    }
}

/// Fees in euros per act and per rank within the année
/// d'accompagnement. Defaults are placeholders — adjust them in
/// `config.toml` to the convention currently in force.
#[derive(Serialize, Clone)]
pub struct BillingConfig {
    pub bpm: ActFees,
    pub aod: ActFees,
    pub avk: ActFees,
    pub asthme: ActFees,
    pub anticancereux: ActFees,
    pub trod_angine: ActFees,
    pub trod_cystite: ActFees,
    pub vaccination: ActFees,
    pub prevention: ActFees,
}

/// Every act is optional in the file and merged onto its default, so a
/// partial edit never zeroes the ranks it did not mention.
impl<'de> Deserialize<'de> for BillingConfig {
    fn deserialize<D: serde::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        #[derive(Deserialize, Default)]
        #[serde(default)]
        struct Partial {
            #[serde(alias = "bpm_fee")]
            bpm: Option<FeesRepr>,
            #[serde(alias = "aod_fee")]
            aod: Option<FeesRepr>,
            #[serde(alias = "avk_fee")]
            avk: Option<FeesRepr>,
            #[serde(alias = "asthme_fee")]
            asthme: Option<FeesRepr>,
            #[serde(alias = "anticancereux_fee")]
            anticancereux: Option<FeesRepr>,
            #[serde(alias = "trod_angine_fee")]
            trod_angine: Option<FeesRepr>,
            #[serde(alias = "trod_cystite_fee")]
            trod_cystite: Option<FeesRepr>,
            #[serde(alias = "vaccination_fee")]
            vaccination: Option<FeesRepr>,
            #[serde(alias = "prevention_fee")]
            prevention: Option<FeesRepr>,
        }
        let p = Partial::deserialize(de)?;
        let d = BillingConfig::default();
        Ok(BillingConfig {
            bpm: FeesRepr::merge(p.bpm, d.bpm),
            aod: FeesRepr::merge(p.aod, d.aod),
            avk: FeesRepr::merge(p.avk, d.avk),
            asthme: FeesRepr::merge(p.asthme, d.asthme),
            anticancereux: FeesRepr::merge(p.anticancereux, d.anticancereux),
            trod_angine: FeesRepr::merge(p.trod_angine, d.trod_angine),
            trod_cystite: FeesRepr::merge(p.trod_cystite, d.trod_cystite),
            vaccination: FeesRepr::merge(p.vaccination, d.vaccination),
            prevention: FeesRepr::merge(p.prevention, d.prevention),
        })
    }
}

impl Default for BillingConfig {
    fn default() -> Self {
        Self {
            bpm: ActFees::staged(60.0, 20.0),
            aod: ActFees::staged(40.0, 20.0),
            avk: ActFees::staged(40.0, 20.0),
            asthme: ActFees::staged(40.0, 20.0),
            anticancereux: ActFees::staged(60.0, 20.0),
            trod_angine: ActFees::flat(10.0),
            trod_cystite: ActFees::flat(12.0),
            vaccination: ActFees::flat(10.0),
            prevention: ActFees::flat(30.0),
        }
    }
}

impl Config {
    /// The density chosen in the options, as the `motif` enum.
    pub fn density(&self) -> motif::Density {
        if self.ui.density.trim().eq_ignore_ascii_case("compact") {
            motif::Density::Compact
        } else {
            motif::Density::Comfortable
        }
    }

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

    pub fn carnet_template_path(&self) -> PathBuf {
        self.templates
            .carnet_template_path
            .clone()
            .unwrap_or_else(|| Self::path().with_file_name("carnet_layout.typ"))
    }

    /// The yearly quota for an act kind (0 = no rule).
    pub fn per_year(&self, kind: crate::db::InterviewKind) -> u32 {
        match kind {
            crate::db::InterviewKind::Bpm => self.rules.bpm_per_year,
            crate::db::InterviewKind::Aod => self.rules.aod_per_year,
            crate::db::InterviewKind::Asthme => self.rules.asthme_per_year,
            crate::db::InterviewKind::TrodAngine => self.rules.trod_angine_per_year,
            crate::db::InterviewKind::TrodCystite => self.rules.trod_cystite_per_year,
            crate::db::InterviewKind::Prevention => self.rules.prevention_per_year,
            crate::db::InterviewKind::Avk => self.rules.avk_per_year,
            crate::db::InterviewKind::Anticancereux => self.rules.anticancereux_per_year,
            crate::db::InterviewKind::Vaccination => self.rules.vaccination_per_year,
        }
    }

    /// Serialize and persist the configuration (the options editor).
    /// Note: rewrites `config.toml`, replacing any hand-written comments.
    pub fn save(&self) -> Result<(), String> {
        let text = toml::to_string_pretty(self).map_err(|e| e.to_string())?;
        let path = Self::path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&path, text).map_err(|e| e.to_string())
    }

    pub fn act_fees(&self, kind: crate::db::InterviewKind) -> ActFees {
        match kind {
            crate::db::InterviewKind::Bpm => self.billing.bpm,
            crate::db::InterviewKind::Aod => self.billing.aod,
            crate::db::InterviewKind::Avk => self.billing.avk,
            crate::db::InterviewKind::Asthme => self.billing.asthme,
            crate::db::InterviewKind::Anticancereux => self.billing.anticancereux,
            crate::db::InterviewKind::TrodAngine => self.billing.trod_angine,
            crate::db::InterviewKind::TrodCystite => self.billing.trod_cystite,
            crate::db::InterviewKind::Vaccination => self.billing.vaccination,
            crate::db::InterviewKind::Prevention => self.billing.prevention,
        }
    }

    /// Fee of one act, given its 0-based rank inside its yearly cycle
    /// (0 = entretien initial).
    pub fn fee(&self, kind: crate::db::InterviewKind, rank: usize) -> f64 {
        self.act_fees(kind).for_rank(rank)
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
            aod = { initial = 44.0, suivi_1 = 22.0, suivi_2 = 11.0 }
            "#,
        )
        .unwrap();
        assert_eq!(cfg.database.auto_lock_timeout_minutes, 5);
        assert!(!cfg.ui.show_docs_on_start);
        // The legacy flat form fills all three rank slots…
        assert_eq!(cfg.billing.bpm, ActFees::flat(55.5));
        // …the nested form sets them individually.
        assert_eq!(cfg.billing.aod.for_rank(0), 44.0);
        assert_eq!(cfg.billing.aod.for_rank(1), 22.0);
        assert_eq!(cfg.billing.aod.for_rank(9), 11.0);
        // Unset fields keep their defaults.
        assert_eq!(cfg.billing.asthme, ActFees::staged(40.0, 20.0));
        assert!(cfg.team_doc_path().ends_with("notes_equipe.md"));
    }

    #[test]
    fn partial_fee_tables_keep_the_other_ranks() {
        // The natural minimal edit — raise the initial BPM fee only —
        // must not zero the suivi fees (they would bill 0 €).
        let cfg: Config = toml::from_str(
            r#"
            [billing]
            bpm = { initial = 65.0 }
            asthme = { }
            # A misspelt key is ignored, defaults kept.
            aod = { initail = 99.0 }
            trod_angine = 11.0
            "#,
        )
        .unwrap();
        assert_eq!(cfg.billing.bpm, ActFees::staged(65.0, 20.0));
        assert_eq!(cfg.billing.asthme, ActFees::staged(40.0, 20.0));
        assert_eq!(cfg.billing.aod, ActFees::staged(40.0, 20.0));
        assert_eq!(cfg.billing.trod_angine, ActFees::flat(11.0));
        // Acts never mentioned keep their defaults too.
        assert_eq!(cfg.billing.prevention, ActFees::flat(30.0));
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
        assert_eq!(cfg.billing.bpm, ActFees::staged(60.0, 20.0));
        assert!(cfg.templates.bpm_template_path.is_none());
        assert_eq!(cfg.rules.bpm_per_year, 3);
        assert_eq!(cfg.rules.trod_angine_per_year, 0);
    }

    #[test]
    fn config_roundtrips_through_toml() {
        let mut cfg = Config::default();
        cfg.pharmacy.name = "Pharmacie du Centre".to_owned();
        cfg.billing.bpm = ActFees::staged(55.5, 25.0);
        cfg.rules.prevention_per_year = 2;
        cfg.ui.operator = "CL".to_owned();
        let text = toml::to_string_pretty(&cfg).unwrap();
        let back: Config = toml::from_str(&text).unwrap();
        assert_eq!(back.pharmacy.name, "Pharmacie du Centre");
        assert_eq!(back.billing.bpm, ActFees::staged(55.5, 25.0));
        assert_eq!(back.rules.prevention_per_year, 2);
        assert_eq!(back.ui.operator, "CL");
    }
}
