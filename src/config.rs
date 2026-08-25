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
# Amplitude horaire de la journée dans l'agenda.
# day_start_hour = 8
# day_end_hour = 20
# Masquer les montants du tableau de bord (mode discret au comptoir).
# discreet_finances = true
# Initiales de l'opérateur par défaut pour les entrées de notes.
# operator = "CL"

[billing]
# Honoraires en euros, tels que le mémo « Aide à la facturation » de
# l'Assurance Maladie les fixe : un montant par entretien de la
# séquence, sur deux lignes — `annee_1` (1re année d'accompagnement)
# et `annees_suivantes`. Les cases inutilisées valent 0.
#   BMI 15 + 15 + 15 + 20 = 65 €   puis BMS 10 + 20 = 30 €
#   ASI 15 + 15 + 20 = 50 €        puis ASS 10 + 20 = 30 €
#   AC1 15 + 15 + 30 = 60 €        puis AC3 10 + 20 = 30 €
#   AC2 15 + 15 + 50 = 80 €        puis AC4 10 + 20 = 30 €
# bpm = { annee_1 = [15.0, 15.0, 15.0, 20.0], annees_suivantes = [10.0, 20.0, 0.0, 0.0] }
# aod = { annee_1 = [15.0, 15.0, 20.0, 0.0], annees_suivantes = [10.0, 20.0, 0.0, 0.0] }
# avk = { annee_1 = [15.0, 15.0, 20.0, 0.0], annees_suivantes = [10.0, 20.0, 0.0, 0.0] }
# asthme = { annee_1 = [15.0, 15.0, 20.0, 0.0], annees_suivantes = [10.0, 20.0, 0.0, 0.0] }
# anticancereux_lc = { annee_1 = [15.0, 15.0, 30.0, 0.0], annees_suivantes = [10.0, 20.0, 0.0, 0.0] }
# anticancereux_autres = { annee_1 = [15.0, 15.0, 50.0, 0.0], annees_suivantes = [10.0, 20.0, 0.0, 0.0] }
# Code traceur TAC, facturé à chaque adhésion à un nouveau thème.
# adhesion = 0.01
# Code TPH, ajouté pour un entretien réalisé à distance (le mémo ne
# donne pas de montant : mettez celui que votre convention applique).
# teleconsultation = 0.0
# Actes hors convention d'accompagnement : montant unique de l'officine.
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
    /// Open the left navigator dock (the patient / drug / month list)
    /// on start. Off on a small screen, on at a wide counter.
    pub show_nav_on_start: bool,
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
    /// The counter's opening hours, used by the agenda's day plan.
    pub day_start_hour: u32,
    pub day_end_hour: u32,
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
            show_nav_on_start: true,
            text_scale: 1.0,
            density: "confortable".to_owned(),
            icons: false,
            font_path: None,
            side_pane: "docs".to_owned(),
            day_start_hour: 8,
            day_end_hour: 20,
            discreet_finances: true,
            operator: String::new(),
        }
    }
}

/// The staged fees of one theme, as the convention pays them: what is
/// billed at each entretien of the first year's sequence, then of the
/// following years. A zero means the sequence has no such step.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
pub struct ActFees {
    /// Année 1, up to four entretiens (the bilan de médication has four).
    pub annee_1: [f64; ActFees::STEPS],
    /// Années suivantes.
    pub annees_suivantes: [f64; ActFees::STEPS],
}

impl ActFees {
    /// The longest sequence the convention defines is the bilan de
    /// médication's first year: recueil, analyse, suivi, observance.
    pub const STEPS: usize = 4;

    /// A theme billed the same amount every time, whatever the rank —
    /// the acts outside the accompaniment convention (TROD, vaccination,
    /// rendez-vous de prévention).
    pub const fn flat(v: f64) -> Self {
        Self {
            annee_1: [v, v, v, v],
            annees_suivantes: [v, v, v, v],
        }
    }

    /// The convention's staged schedule.
    pub const fn staged(annee_1: [f64; 4], annees_suivantes: [f64; 4]) -> Self {
        Self {
            annee_1,
            annees_suivantes,
        }
    }

    /// The amount for the `rank`-th entretien of the `year`-th year of
    /// accompaniment, both 0-based.
    pub fn amount(&self, year: usize, rank: usize) -> f64 {
        let row = if year == 0 {
            &self.annee_1
        } else {
            &self.annees_suivantes
        };
        row.get(rank).copied().unwrap_or(0.0)
    }

    /// What the whole year is worth once its sequence is complete.
    pub fn year_total(&self, year: usize) -> f64 {
        let row = if year == 0 {
            &self.annee_1
        } else {
            &self.annees_suivantes
        };
        row.iter().sum()
    }

    /// Mutable access to one amount, for the Options grid.
    pub fn slot_mut(&mut self, year: usize, rank: usize) -> &mut f64 {
        let row = if year == 0 {
            &mut self.annee_1
        } else {
            &mut self.annees_suivantes
        };
        &mut row[rank.min(ActFees::STEPS - 1)]
    }
}

/// Fees in euros per theme, per year of accompaniment and per
/// entretien, as the Assurance Maladie memo sets them. Adjust them in
/// `config.toml` when the convention changes.
#[derive(Serialize, Clone)]
pub struct BillingConfig {
    pub bpm: ActFees,
    pub aod: ActFees,
    pub avk: ActFees,
    pub asthme: ActFees,
    pub anticancereux_lc: ActFees,
    pub anticancereux_autres: ActFees,
    pub trod_angine: ActFees,
    pub trod_cystite: ActFees,
    pub vaccination: ActFees,
    pub prevention: ActFees,
    /// Code TAC, billed once per patient and per theme on joining.
    pub adhesion: f64,
    /// Code TPH, added for an entretien held remotely. The memo gives
    /// no amount: set it to what your convention pays.
    pub teleconsultation: f64,
}

impl Default for BillingConfig {
    fn default() -> Self {
        Self {
            // Bilan de médication : BMI 15 + 15 + 15 + 20 = 65 €,
            // puis BMS 10 + 20 = 30 €.
            bpm: ActFees::staged([15.0, 15.0, 15.0, 20.0], [10.0, 20.0, 0.0, 0.0]),
            // AOD, AVK, asthme : ASI 15 + 15 + 20 = 50 €, ASS 10 + 20.
            aod: ActFees::staged([15.0, 15.0, 20.0, 0.0], [10.0, 20.0, 0.0, 0.0]),
            avk: ActFees::staged([15.0, 15.0, 20.0, 0.0], [10.0, 20.0, 0.0, 0.0]),
            asthme: ActFees::staged([15.0, 15.0, 20.0, 0.0], [10.0, 20.0, 0.0, 0.0]),
            // Anticancéreux au long cours : AC1 15 + 15 + 30 = 60 €,
            // AC3 10 + 20 = 30 €.
            anticancereux_lc: ActFees::staged([15.0, 15.0, 30.0, 0.0], [10.0, 20.0, 0.0, 0.0]),
            // Autres anticancéreux : AC2 15 + 15 + 50 = 80 €, AC4 30 €.
            anticancereux_autres: ActFees::staged([15.0, 15.0, 50.0, 0.0], [10.0, 20.0, 0.0, 0.0]),
            // Hors convention d'accompagnement : montants de l'officine.
            trod_angine: ActFees::flat(10.0),
            trod_cystite: ActFees::flat(12.0),
            vaccination: ActFees::flat(10.0),
            prevention: ActFees::flat(30.0),
            adhesion: 0.01,
            teleconsultation: 0.0,
        }
    }
}

/// One fee entry as written in `config.toml`: the flat number of the
/// oldest format (`bpm_fee = 60.0`), the per-rank table of the 0.15
/// format (`bpm = { initial = 65.0 }`), or the convention's own two
/// rows (`bpm = { annee_1 = [15, 15, 15, 20] }`). Anything left out —
/// and keys that are misspelt, which serde ignores — keeps the default
/// of that theme rather than silently becoming 0 €.
#[derive(Deserialize)]
#[serde(untagged)]
enum FeesRepr {
    Flat(f64),
    Table {
        #[serde(default)]
        annee_1: Option<[f64; ActFees::STEPS]>,
        #[serde(default)]
        annees_suivantes: Option<[f64; ActFees::STEPS]>,
        #[serde(default)]
        initial: Option<f64>,
        #[serde(default)]
        suivi_1: Option<f64>,
        #[serde(default)]
        suivi_2: Option<f64>,
    },
}

impl FeesRepr {
    /// Apply what the file specified on top of the theme's default.
    fn merge(this: Option<Self>, default: ActFees) -> ActFees {
        match this {
            None => default,
            Some(Self::Flat(v)) => ActFees::flat(v),
            Some(Self::Table {
                annee_1,
                annees_suivantes,
                initial,
                suivi_1,
                suivi_2,
            }) => {
                let mut out = ActFees {
                    annee_1: annee_1.unwrap_or(default.annee_1),
                    annees_suivantes: annees_suivantes.unwrap_or(default.annees_suivantes),
                };
                // The 0.15 form had one rate per rank, whatever the
                // year: spread it over both rows so an old config.toml
                // keeps billing what it billed.
                if initial.is_some() || suivi_1.is_some() || suivi_2.is_some() {
                    let i = initial.unwrap_or(out.annee_1[0]);
                    let s1 = suivi_1.unwrap_or(out.annee_1[1]);
                    let s2 = suivi_2.unwrap_or(out.annee_1[2]);
                    out.annee_1 = [i, s1, s2, s2];
                    out.annees_suivantes = [i, s1, s2, s2];
                }
                out
            }
        }
    }
}

/// Every theme is optional in the file and merged onto its default, so
/// a partial edit never zeroes what it does not mention.
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
            #[serde(alias = "anticancereux", alias = "anticancereux_fee")]
            anticancereux_lc: Option<FeesRepr>,
            anticancereux_autres: Option<FeesRepr>,
            #[serde(alias = "trod_angine_fee")]
            trod_angine: Option<FeesRepr>,
            #[serde(alias = "trod_cystite_fee")]
            trod_cystite: Option<FeesRepr>,
            #[serde(alias = "vaccination_fee")]
            vaccination: Option<FeesRepr>,
            #[serde(alias = "prevention_fee")]
            prevention: Option<FeesRepr>,
            adhesion: Option<f64>,
            teleconsultation: Option<f64>,
        }
        let p = Partial::deserialize(de)?;
        let d = BillingConfig::default();
        Ok(BillingConfig {
            bpm: FeesRepr::merge(p.bpm, d.bpm),
            aod: FeesRepr::merge(p.aod, d.aod),
            avk: FeesRepr::merge(p.avk, d.avk),
            asthme: FeesRepr::merge(p.asthme, d.asthme),
            anticancereux_lc: FeesRepr::merge(p.anticancereux_lc, d.anticancereux_lc),
            anticancereux_autres: FeesRepr::merge(p.anticancereux_autres, d.anticancereux_autres),
            trod_angine: FeesRepr::merge(p.trod_angine, d.trod_angine),
            trod_cystite: FeesRepr::merge(p.trod_cystite, d.trod_cystite),
            vaccination: FeesRepr::merge(p.vaccination, d.vaccination),
            prevention: FeesRepr::merge(p.prevention, d.prevention),
            adhesion: p.adhesion.unwrap_or(d.adhesion),
            teleconsultation: p.teleconsultation.unwrap_or(d.teleconsultation),
        })
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
            crate::db::InterviewKind::AnticancereuxLc
            | crate::db::InterviewKind::AnticancereuxAutres => self.rules.anticancereux_per_year,
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
            crate::db::InterviewKind::AnticancereuxLc => self.billing.anticancereux_lc,
            crate::db::InterviewKind::AnticancereuxAutres => self.billing.anticancereux_autres,
            crate::db::InterviewKind::TrodAngine => self.billing.trod_angine,
            crate::db::InterviewKind::TrodCystite => self.billing.trod_cystite,
            crate::db::InterviewKind::Vaccination => self.billing.vaccination,
            crate::db::InterviewKind::Prevention => self.billing.prevention,
        }
    }

    /// What one entretien is worth: its theme, the year of
    /// accompaniment it belongs to and its rank in that year's
    /// sequence, all 0-based.
    pub fn fee(&self, kind: crate::db::InterviewKind, year: usize, rank: usize) -> f64 {
        self.act_fees(kind).amount(year, rank)
    }

    /// What one entretien actually bills: the act code's fee plus the
    /// TPH supplement when it was held remotely.
    pub fn act_total(
        &self,
        kind: crate::db::InterviewKind,
        year: usize,
        rank: usize,
        remote: bool,
    ) -> f64 {
        let base = self.fee(kind, year, rank);
        if remote && kind.is_accompaniment() {
            base + self.billing.teleconsultation
        } else {
            base
        }
    }

    /// The quota the convention sets for one theme and one year: the
    /// number of entretiens its sequence holds. Outside the
    /// accompaniment themes, the officine's own rule applies.
    pub fn sequence_len(&self, kind: crate::db::InterviewKind, year: usize) -> usize {
        if kind.is_accompaniment() {
            kind.sequence(year).len()
        } else {
            self.per_year(kind) as usize
        }
    }
}

/// Where the workspace was left: the window's size and how wide the
/// two docks were dragged.
///
/// Kept in its own `layout.toml` beside `config.toml` rather than in it:
/// the configuration is hand-editable and carries the operator's own
/// comments, and rewriting it on every quit to record a window size
/// would quietly throw those away.
#[derive(Deserialize, Serialize, Clone, Copy, PartialEq, Debug)]
#[serde(default)]
pub struct Layout {
    /// Window size in logical pixels. Zero means "never recorded".
    pub window_width: f32,
    pub window_height: f32,
    /// Dock widths. Zero means "use the default share of the window".
    pub nav_width: f32,
    pub docs_width: f32,
}

impl Default for Layout {
    fn default() -> Self {
        Self {
            window_width: 0.0,
            window_height: 0.0,
            nav_width: 0.0,
            docs_width: 0.0,
        }
    }
}

impl Layout {
    pub fn path() -> PathBuf {
        Config::path().with_file_name("layout.toml")
    }

    /// Never fails: a missing or unreadable file is simply "no record".
    pub fn load() -> Self {
        std::fs::read_to_string(Self::path())
            .ok()
            .and_then(|s| toml::from_str(&s).ok())
            .unwrap_or_default()
    }

    /// A size only counts once it is plausible — a window minimised or
    /// mid-restore reports a few pixels, and reopening at that size the
    /// next morning would be a bug the operator could not undo.
    pub fn window(&self) -> Option<[f32; 2]> {
        (self.window_width >= 640.0 && self.window_height >= 480.0)
            .then_some([self.window_width, self.window_height])
    }

    pub fn save(&self) {
        let Ok(text) = toml::to_string_pretty(self) else {
            return;
        };
        let path = Self::path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(path, text);
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
        assert_eq!(cfg.billing.aod.amount(0, 0), 44.0);
        assert_eq!(cfg.billing.aod.amount(0, 1), 22.0);
        assert_eq!(cfg.billing.aod.amount(1, 2), 11.0);
        // Unset fields keep the convention's defaults.
        assert_eq!(
            cfg.billing.asthme,
            ActFees::staged([15.0, 15.0, 20.0, 0.0], [10.0, 20.0, 0.0, 0.0])
        );
        assert!(cfg.team_doc_path().ends_with("notes_equipe.md"));
    }

    #[test]
    fn partial_fee_tables_keep_the_other_ranks() {
        // The natural minimal edit — raise the initial BPM fee only —
        // must not zero the suivi fees (they would bill 0 €).
        let cfg: Config = toml::from_str(
            r#"
            [billing]
            bpm = { annee_1 = [16.0, 16.0, 16.0, 21.0] }
            asthme = { }
            # A misspelt key is ignored, defaults kept.
            aod = { annee_2 = 99.0 }
            trod_angine = 11.0
            "#,
        )
        .unwrap();
        // The edited row is taken, the untouched one keeps its default.
        assert_eq!(
            cfg.billing.bpm,
            ActFees::staged([16.0, 16.0, 16.0, 21.0], [10.0, 20.0, 0.0, 0.0])
        );
        let convention = ActFees::staged([15.0, 15.0, 20.0, 0.0], [10.0, 20.0, 0.0, 0.0]);
        assert_eq!(cfg.billing.asthme, convention);
        assert_eq!(cfg.billing.aod, convention);
        assert_eq!(cfg.billing.trod_angine, ActFees::flat(11.0));
        // Acts never mentioned keep their defaults too.
        assert_eq!(cfg.billing.prevention, ActFees::flat(30.0));
    }

    /// The table of the Assurance Maladie memo, line by line: if a
    /// default drifts, the officine bills the wrong amount.
    #[test]
    fn defaults_match_the_official_fee_table() {
        use crate::db::InterviewKind::*;
        let cfg = Config::default();
        let year = |kind, y: usize, steps: &[f64], total: f64| {
            for (rank, want) in steps.iter().enumerate() {
                assert_eq!(
                    cfg.fee(kind, y, rank),
                    *want,
                    "{kind:?} année {y} rang {rank}"
                );
            }
            assert_eq!(
                cfg.act_fees(kind).year_total(y),
                total,
                "{kind:?} année {y}"
            );
        };
        // ASI 15 + 15 + 20 = 50 €, ASS 10 + 20 = 30 €.
        for kind in [Aod, Avk, Asthme] {
            year(kind, 0, &[15.0, 15.0, 20.0, 0.0], 50.0);
            year(kind, 1, &[10.0, 20.0, 0.0, 0.0], 30.0);
        }
        // BMI 15 + 15 + 15 + 20 = 65 €, BMS 10 + 20 = 30 €.
        year(Bpm, 0, &[15.0, 15.0, 15.0, 20.0], 65.0);
        year(Bpm, 1, &[10.0, 20.0, 0.0, 0.0], 30.0);
        // AC1 15 + 15 + 30 = 60 €, AC3 30 €.
        year(AnticancereuxLc, 0, &[15.0, 15.0, 30.0, 0.0], 60.0);
        year(AnticancereuxLc, 1, &[10.0, 20.0, 0.0, 0.0], 30.0);
        // AC2 15 + 15 + 50 = 80 €, AC4 30 €.
        year(AnticancereuxAutres, 0, &[15.0, 15.0, 50.0, 0.0], 80.0);
        year(AnticancereuxAutres, 1, &[10.0, 20.0, 0.0, 0.0], 30.0);
        // Adhésion : le code traceur TAC, 0,01 €.
        assert_eq!(cfg.billing.adhesion, 0.01);
        // Codes actes, années 1 et suivantes.
        assert_eq!(Aod.act_code(0), Some("ASI"));
        assert_eq!(Aod.act_code(1), Some("ASS"));
        assert_eq!(Bpm.act_code(0), Some("BMI"));
        assert_eq!(Bpm.act_code(1), Some("BMS"));
        assert_eq!(AnticancereuxLc.act_code(0), Some("AC1"));
        assert_eq!(AnticancereuxLc.act_code(1), Some("AC3"));
        assert_eq!(AnticancereuxAutres.act_code(0), Some("AC2"));
        assert_eq!(AnticancereuxAutres.act_code(1), Some("AC4"));
        // Prise en charge : 100 % pour l'anticancéreux, 70 % ailleurs.
        assert_eq!(AnticancereuxLc.coverage_rate(), 100);
        assert_eq!(AnticancereuxAutres.coverage_rate(), 100);
        assert_eq!(Bpm.coverage_rate(), 70);
        assert_eq!(Aod.coverage_rate(), 70);
        // Le quota d'une année est la longueur de sa séquence.
        assert_eq!(cfg.sequence_len(Bpm, 0), 4);
        assert_eq!(cfg.sequence_len(Bpm, 1), 2);
        assert_eq!(cfg.sequence_len(Aod, 0), 3);
        assert_eq!(cfg.sequence_len(Aod, 1), 2);
    }

    /// TPH is billed on top of the act code, and only for the themes
    /// the accompaniment convention covers.
    #[test]
    fn remote_entretiens_add_the_tph_code() {
        use crate::db::InterviewKind::*;
        let mut cfg = Config::default();
        cfg.billing.teleconsultation = 3.5;
        assert_eq!(cfg.act_total(Bpm, 0, 0, false), 15.0);
        assert_eq!(cfg.act_total(Bpm, 0, 0, true), 18.5);
        // Hors accompagnement : pas de supplément.
        let trod = cfg.fee(TrodAngine, 0, 0);
        assert_eq!(cfg.act_total(TrodAngine, 0, 0, true), trod);
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
        assert_eq!(
            cfg.billing.bpm,
            ActFees::staged([15.0, 15.0, 15.0, 20.0], [10.0, 20.0, 0.0, 0.0])
        );
        assert!(cfg.templates.bpm_template_path.is_none());
        assert_eq!(cfg.rules.bpm_per_year, 3);
        assert_eq!(cfg.rules.trod_angine_per_year, 0);
    }

    #[test]
    fn an_implausible_window_size_is_not_restored() {
        let mut l = Layout::default();
        assert!(l.window().is_none());
        // Minimised, or caught mid-restore: not a size to reopen at.
        l.window_width = 12.0;
        l.window_height = 8.0;
        assert!(l.window().is_none());
        l.window_width = 1280.0;
        l.window_height = 800.0;
        assert_eq!(l.window(), Some([1280.0, 800.0]));
    }

    #[test]
    fn config_roundtrips_through_toml() {
        let mut cfg = Config::default();
        cfg.pharmacy.name = "Pharmacie du Centre".to_owned();
        cfg.billing.bpm = ActFees::staged([55.5, 25.0, 0.0, 0.0], [25.0, 25.0, 0.0, 0.0]);
        cfg.rules.prevention_per_year = 2;
        cfg.ui.operator = "CL".to_owned();
        let text = toml::to_string_pretty(&cfg).unwrap();
        let back: Config = toml::from_str(&text).unwrap();
        assert_eq!(back.pharmacy.name, "Pharmacie du Centre");
        assert_eq!(
            back.billing.bpm,
            ActFees::staged([55.5, 25.0, 0.0, 0.0], [25.0, 25.0, 0.0, 0.0])
        );
        assert_eq!(back.rules.prevention_per_year, 2);
        assert_eq!(back.ui.operator, "CL");
    }
}
