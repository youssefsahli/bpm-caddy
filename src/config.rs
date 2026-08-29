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
# Palette de l'interface : "motif", "cde", "decwindows", "indigo",
# "olive" ou "contraste". La forme ne change pas — angles droits, biseaux
# de deux pixels — seules les couleurs changent.
# theme = "motif"
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
# ordonnance_template_path = "templates/ordonnance_layout.typ"

[pharmacy]
# Identité de l'officine, pour l'en-tête du courrier au médecin.
# name = "Pharmacie du Centre"
# address = "1 place de la Mairie, 34000 Montpellier"
# phone = "04 67 00 00 00"
# pharmacist = "Dr Claire Leroy, pharmacien titulaire"
# N° d'identification Assurance Maladie, imprimé sur le bulletin
# d'adhésion. Laissé vide, la ligne du bulletin reste à remplir à la main.
# am_number = "3400123"
# L'équipe. Les initiales servent à signer les notes et le carnet ; le
# nom et la qualité sont ce qui s'imprime au bas des documents, à la
# place de `pharmacist`, quand l'acte porte ces initiales.
# operators = [
#   { initials = "CL", name = "Claire Leroy", role = "Pharmacien titulaire" },
#   { initials = "YS", name = "Yanis Saïd", role = "Pharmacien adjoint" },
#   { initials = "MB", name = "Maya Bertrand", role = "Préparatrice" },
# ]

[disclaimers]
# Les mentions imprimées ou affichées par l'application. Elles sont
# vides par défaut : l'application n'ajoute aucun avertissement de son
# propre chef. Écrivez ici celles que l'officine veut voir apparaître.
# Ligne d'en-tête de l'ordonnance, sous le titre.
# ordonnance_header = "Dispensation protocolisée par le pharmacien après test rapide d'orientation diagnostique"
# Pied de l'ordonnance, sous la signature.
# ordonnance_footer = "Reconsulter sans attendre en cas d'aggravation ou de signes nouveaux."
# Encadré affiché à l'écran au-dessus de l'ordonnance en préparation.
# ordonnance_screen = "Vérifiez la molécule, la posologie et les contre-indications avant d'imprimer."
# Pied du carnet de vaccination imprimé.
# carnet = "Document indicatif : il ne remplace pas le carnet de vaccination officiel ni le dossier médical partagé."
# Sous le carnet à l'écran et sous la carte vaccinale.
# vaccins = "Données indicatives — l'autorité reste le calendrier vaccinal en vigueur et le BEH « Recommandations sanitaires pour les voyageurs » de l'année en cours."
# Ligne sous les calculatrices (Cockcroft, dose/kg, décroissance).
# calculator = "Modèle à un compartiment : la clinique et le RCP priment."
# Pied du plan de prise remis au patient.
# plan = "Ce plan reprend ce que nous avons vu ensemble : il ne remplace pas votre ordonnance."
# Pied de la fiche de conciliation adressée au prescripteur.
# conciliation = "Rapprochement établi à l'officine à partir de l'ordonnance de sortie remise par le patient : il ne vaut pas avis médical."

[ordonnance]
# Les fiches du référentiel médicaments portant cette étiquette sont
# proposées comme adjuvant dans l'ordonnance après un TROD positif.
# Ajoutez une fiche, étiquetez-la, écrivez-y ses posologies : elle
# apparaît dans la liste.
# adjuvant_tag = "probiotique"

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

[vitale]
# Lecture de la carte Vitale par un lecteur PC/SC : elle remplit
# l'identité du patient — NIR, noms, date de naissance — au lieu de la
# faire taper. Elle ne facture rien : une feuille de soins électronique
# demande un progiciel SESAM-Vitale agréé et une carte CPS, et c'est le
# LGO qui la produit.
#
# La séquence de commandes APDU est écrite ici et non dans le programme :
# elle relève des spécifications SESAM-Vitale, qui sont sous licence et
# versionnées. Vide, l'application liste quand même les lecteurs et
# affiche l'ATR de la carte — de quoi voir si le lecteur et la carte sont
# vus.
# enabled = false
# reader = ""
# apdu = ["00A4040007A0000000180C000100", "00B0000000"]

[locations]
# Les locations de matériel : nébuliseur, neurostimulateur, lit
# médicalisé… Un forfait court tant que le matériel est sorti, et se
# compte en périodes entamées.
#
# Le catalogue est fourni — les six matériels et la ligne LPP de chacun,
# comme les six fiches de la famille « Location ». Ce qui est fourni à
# zéro, c'est l'euro : l'application ne connaît aucun tarif de sa propre
# autorité, et c'est l'officine qui écrit ce que sa LPP paie, vérifié sur
# ameli.fr. Un montant livré serait faux dans l'année.
#
# Écrire « forfaits = [] » vide le catalogue ; le laisser en commentaire
# le rétablit.
#
#   period       : "jour", "semaine" ou "mois"
#   fee          : le forfait d'une période, en euros
#   renewal_days : au bout de combien de jours l'ordonnance se renouvelle
#                  (0 = pas de renouvellement à rappeler)
#   max_periods  : nombre de périodes payées par la ligne LPP
#                  (0 = sans plafond)
# Le délai de prévenance des renouvellements, en jours.
# notice_days = 7
# forfaits = [
#   { label = "Nébuliseur", lpp = "Aérosolthérapie — titre I", period = "semaine", fee = 0.0, renewal_days = 0, max_periods = 0 },
#   { label = "Neurostimulateur (TENS)", lpp = "Neurostimulateur transcutané — titre I", period = "semaine", fee = 0.0, renewal_days = 0, max_periods = 0 },
#   { label = "Lit médicalisé", lpp = "Maintien à domicile — titre I", period = "semaine", fee = 0.0, renewal_days = 0, max_periods = 0 },
#   { label = "Matelas anti-escarre", lpp = "Support d\'aide à la prévention — titre I", period = "semaine", fee = 0.0, renewal_days = 0, max_periods = 0 },
#   { label = "Fauteuil roulant", lpp = "Véhicule pour handicapé physique — titre IV", period = "semaine", fee = 0.0, renewal_days = 0, max_periods = 0 },
#   { label = "Tire-lait", lpp = "Tire-lait — titre I", period = "semaine", fee = 0.0, renewal_days = 0, max_periods = 0 },
# ]
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
    pub ordonnance: OrdonnanceConfig,
    pub disclaimers: DisclaimersConfig,
    pub locations: LocationsConfig,
    pub vitale: VitaleConfig,
}

/// Reading a carte Vitale through a PC/SC reader.
///
/// **The command script lives here and not in the code**, and that is
/// the point of this section. The sequence of APDU commands that reaches
/// the beneficiary data belongs to the SESAM-Vitale specification, which
/// is licensed and versioned: a sequence compiled into the binary could
/// be corrected neither by the officine nor by whoever integrates the
/// reader, and would not follow a card generation it predates. Empty by
/// default — the application then still lists the readers and shows the
/// card's ATR, which is what says whether the reader and the card are
/// seen at all.
///
/// What this never does is bill: an FSE needs an agreed SESAM-Vitale
/// package and a carte CPS, and the LGO stays the tool that produces it.
#[derive(Deserialize, Serialize, Clone, Default)]
#[serde(default)]
pub struct VitaleConfig {
    /// Off until a reader is actually connected: a button that always
    /// fails is a button nobody presses twice.
    pub enabled: bool,
    /// Part of the reader's name, when the post has several. Empty takes
    /// the first one found.
    pub reader: String,
    /// The commands sent to the card, in hexadecimal, in order.
    pub apdu: Vec<String>,
}

impl VitaleConfig {
    /// The script as bytes. An unreadable command stops the whole
    /// script: half an APDU is a different command, and sending it to a
    /// card is not something to do on a guess.
    pub fn script(&self) -> Result<Vec<Vec<u8>>, String> {
        self.apdu
            .iter()
            .map(|c| crate::vitale::parse_hex(c))
            .collect()
    }
}

/// How a rental forfait is counted. The LPP pays by the week for most
/// of the material an officine lends out, by the month for some, by the
/// day for none of it — but the officine's own agreements vary, so all
/// three exist.
#[derive(Deserialize, Serialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "lowercase")]
pub enum Period {
    #[serde(rename = "jour")]
    Day,
    #[default]
    #[serde(rename = "semaine")]
    Week,
    #[serde(rename = "mois")]
    Month,
}

impl Period {
    /// The length of one period, in days.
    pub fn days(self) -> u32 {
        match self {
            Period::Day => 1,
            Period::Week => 7,
            Period::Month => 30,
        }
    }

    /// The French word, as it is written in `config.toml` and shown.
    pub fn label(self) -> &'static str {
        match self {
            Period::Day => "jour",
            Period::Week => "semaine",
            Period::Month => "mois",
        }
    }

    /// The same word agreed with a count — « 1 semaine », « 11 semaines »,
    /// and « mois » either way. A table that says « 11 semaine » is a
    /// table someone wrote in a hurry, and it shows.
    pub fn agreed(self, count: u32) -> &'static str {
        match (self, count) {
            (Period::Month, _) => "mois",
            (_, 0 | 1) => self.label(),
            (Period::Day, _) => "jours",
            (Period::Week, _) => "semaines",
        }
    }

    pub const ALL: [Period; 3] = [Period::Day, Period::Week, Period::Month];
}

/// The rentals the officine bills, and how long before a renewal it
/// wants to be told.
///
/// The *catalogue* ships filled in — the six machines the officine
/// actually lends out, each with the wording of its LPP line and the
/// period it is counted in, matching the six fiches of the « Location »
/// family. What ships at **zero** is every euro: the LPP's tarifs move,
/// and a fee shipped in the application would be a fee wrong within the
/// year, the same rule as the `lpp` field of a dispositif fiche. The
/// officine writes what its own line pays, in Options › Locations.
///
/// The tab used to open on an empty list and a paragraph explaining how
/// to fill it; now it opens on the six lines, and the only thing left to
/// type is the amount.
#[derive(Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct LocationsConfig {
    pub forfaits: Vec<LocationForfait>,
    /// How many days before a renewal it is announced. 0 = the day of.
    pub notice_days: u32,
}

impl LocationsConfig {
    /// The catalogue an officine starts from: what it lends, on which
    /// LPP line, counted in which period. Never a price.
    pub fn starter_forfaits() -> Vec<LocationForfait> {
        [
            (
                "Nébuliseur",
                "Titre I — forfait hebdomadaire d'aérosolthérapie ; consommables à part",
                Period::Week,
            ),
            (
                "Neurostimulateur (TENS)",
                "Titre I — location du neurostimulateur ; électrodes sur ligne propre",
                Period::Week,
            ),
            (
                "Lit médicalisé",
                "Titre I — forfait hebdomadaire lit, matelas et accessoires",
                Period::Week,
            ),
            (
                "Matelas anti-escarre",
                "Titre I — support dynamique, ligne selon le niveau de risque",
                Period::Week,
            ),
            (
                "Fauteuil roulant",
                "Titre IV — location du fauteuil roulant manuel ; coussin sur ligne propre",
                Period::Week,
            ),
            (
                "Tire-lait",
                "Titre I — location sur prescription ; set de recueil sur ligne propre",
                Period::Week,
            ),
        ]
        .into_iter()
        .map(|(label, lpp, period)| LocationForfait {
            label: label.to_owned(),
            lpp: lpp.to_owned(),
            period,
            fee: 0.0,
            renewal_days: 0,
            max_periods: 0,
        })
        .collect()
    }

    /// Append the starter lines this configuration is missing, matched
    /// by label, and answer how many were added.
    ///
    /// The same discipline as the drug base's « Compléter les
    /// médicaments de départ »: it never touches a line the officine has
    /// already written, fee included.
    pub fn complete_forfaits(&mut self) -> usize {
        let mut added = 0;
        for f in Self::starter_forfaits() {
            let known = self
                .forfaits
                .iter()
                .any(|k| k.label.trim().eq_ignore_ascii_case(f.label.trim()));
            if !known {
                self.forfaits.push(f);
                added += 1;
            }
        }
        added
    }
}

impl Default for LocationsConfig {
    fn default() -> Self {
        Self {
            forfaits: Self::starter_forfaits(),
            notice_days: 7,
        }
    }
}

/// One rental line as the officine bills it.
#[derive(Deserialize, Serialize, Default, Clone, PartialEq, Debug)]
#[serde(default)]
pub struct LocationForfait {
    /// "Nébuliseur", "Neurostimulateur (TENS)"…
    pub label: String,
    /// The LPP line, as the team wrote it down.
    pub lpp: String,
    pub period: Period,
    /// What one period is paid, in euros.
    pub fee: f64,
    /// Days before the prescription has to be renewed. 0 = no reminder.
    pub renewal_days: u32,
    /// Periods the LPP line pays. 0 = no cap.
    pub max_periods: u32,
}

impl LocationForfait {
    /// Whether this line is usable: a forfait without a name cannot be
    /// chosen, and one at zero euros bills nothing — but it is still
    /// legitimate, since the renewal reminder alone is worth having.
    pub fn is_named(&self) -> bool {
        !self.label.trim().is_empty()
    }
}

/// Every mention the application prints or shows, and nothing it says
/// on its own account: each field is empty by default, and an empty
/// field prints no line at all. The officine writes what it wants to
/// see — a legal frame on the ordonnance, an advice at the foot of the
/// carnet — and nothing else appears.
#[derive(Deserialize, Serialize, Default, Clone)]
#[serde(default)]
pub struct DisclaimersConfig {
    /// Under the ordonnance's title.
    pub ordonnance_header: String,
    /// At the foot of the ordonnance, under the signature box.
    pub ordonnance_footer: String,
    /// In the ordonnance box on screen, above the lines being prepared.
    pub ordonnance_screen: String,
    /// At the foot of the printed carnet de vaccination.
    pub carnet: String,
    /// Under the carnet on screen, and under the vaccine map.
    pub vaccins: String,
    /// Under the dose calculators.
    pub calculator: String,
    /// At the foot of the patient's plan de prise.
    pub plan: String,
    /// At the foot of the conciliation sheet sent to the prescriber.
    pub conciliation: String,
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
    /// N° d'identification Assurance Maladie of the officine, printed on
    /// the bulletin d'adhésion. Empty leaves that line blank.
    #[serde(default)]
    pub am_number: String,
    /// The team, by initials. What the counter stamps on a note is the
    /// initials; what a printed document signs is the name and the
    /// qualité behind them.
    #[serde(default)]
    pub operators: Vec<Operator>,
}

/// One member of the team: the initials that stamp a note, and the name
/// that signs a document.
#[derive(Deserialize, Serialize, Default, Clone, PartialEq, Debug)]
#[serde(default)]
pub struct Operator {
    /// Two or three letters, as typed at the counter.
    pub initials: String,
    /// "Claire Leroy".
    pub name: String,
    /// "Pharmacien titulaire", "Préparatrice"…
    pub role: String,
}

impl Operator {
    /// How this operator signs: the name, then the qualité when one is
    /// given. Falls back to the initials, so a half-filled row still
    /// signs something rather than nothing.
    pub fn signature(&self) -> String {
        let name = self.name.trim();
        let role = self.role.trim();
        match (name.is_empty(), role.is_empty()) {
            (false, false) => format!("{name}, {role}"),
            (false, true) => name.to_owned(),
            (true, false) => role.to_owned(),
            (true, true) => self.initials.trim().to_owned(),
        }
    }

    /// What the operator picker shows: "CL — Claire Leroy".
    pub fn label(&self) -> String {
        let initials = self.initials.trim();
        let signature = self.signature();
        if signature.is_empty() || signature == initials {
            initials.to_owned()
        } else {
            format!("{initials} — {signature}")
        }
    }
}

impl PharmacyConfig {
    /// The team member behind these initials, case-insensitively.
    pub fn operator(&self, initials: &str) -> Option<&Operator> {
        let wanted = initials.trim();
        if wanted.is_empty() {
            return None;
        }
        self.operators
            .iter()
            .find(|o| o.initials.trim().eq_ignore_ascii_case(wanted))
    }

    /// Who signs a document made by `initials`: that operator's name and
    /// qualité when the list knows them, the officine's default
    /// `pharmacist` line otherwise. A document is never left unsigned
    /// because the initials were unknown.
    pub fn signature_for(&self, initials: &str) -> String {
        match self.operator(initials) {
            Some(op) => op.signature(),
            None => self.pharmacist.clone(),
        }
    }
}

/// What the ordonnance box offers alongside the antibiotic.
#[derive(Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct OrdonnanceConfig {
    /// Drug cards carrying this tag are offered as adjuvants. Point it
    /// at whatever tag the officine actually uses; every card so tagged
    /// joins the list, with its own posology lines.
    pub adjuvant_tag: String,
}

impl Default for OrdonnanceConfig {
    fn default() -> Self {
        Self {
            adjuvant_tag: "probiotique".to_owned(),
        }
    }
}

/// Custom Typst templates; the embedded default is used when unset.
#[derive(Deserialize, Serialize, Clone, Default)]
#[serde(default)]
pub struct TemplatesConfig {
    pub bpm_template_path: Option<PathBuf>,
    pub cr_template_path: Option<PathBuf>,
    pub carnet_template_path: Option<PathBuf>,
    pub ordonnance_template_path: Option<PathBuf>,
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
    /// Which palette the Motif chrome is drawn from — one of
    /// `motif::THEMES` by its key. The shape never changes; only the
    /// colours. An unknown name falls back to the classic one.
    pub theme: String,
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
            theme: motif::THEMES[0].key.to_owned(),
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
    /// Put the chosen palette in force. Called once at start-up and
    /// again whenever the options are saved; an unknown name is not an
    /// error, it is the classic theme.
    pub fn apply_theme(&self) {
        motif::set_theme(&self.ui.theme);
    }

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

    /// Where the ordonnance template lives (dispensation after a TROD).
    pub fn ordonnance_template_path(&self) -> PathBuf {
        self.templates
            .ordonnance_template_path
            .clone()
            .unwrap_or_else(|| Self::path().with_file_name("ordonnance_layout.typ"))
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

/// Where the workspace was left: the window's size, how wide the two
/// docks were dragged, whether they were open at all, which view was on
/// screen, and which version wrote all that down.
///
/// Kept in its own `layout.toml` beside `config.toml` rather than in it:
/// the configuration is hand-editable and carries the operator's own
/// comments, and rewriting it on every quit to record a window size
/// would quietly throw those away.
///
/// Every field means « never recorded » at its default, and every one
/// of them is a *preference the operator expressed by working*, never a
/// setting: `[ui] show_nav_on_start` still decides the first launch,
/// and after that the shape the post was left in wins.
#[derive(Deserialize, Serialize, Clone, PartialEq, Debug)]
#[serde(default)]
pub struct Layout {
    /// Window size in logical pixels. Zero means "never recorded".
    pub window_width: f32,
    pub window_height: f32,
    /// Dock widths. Zero means "use the default share of the window".
    pub nav_width: f32,
    pub docs_width: f32,
    /// Whether each dock was open. `None` = never recorded, so the
    /// configuration's start-up flags decide.
    pub nav_open: Option<bool>,
    pub docs_open: Option<bool>,
    /// The right pane's content: "docs", "carnet" or "notes".
    pub side_pane: String,
    /// Which view was on screen: see `MainView::as_key`. An empty string
    /// — or a name this version does not know — lands on the search.
    pub view: String,
    /// The version of BPM-Caddy that wrote this file. Recorded so
    /// Options › À propos can say when the workspace was last laid out
    /// by another version, and so a future change of shape can tell a
    /// stale record from a current one.
    pub version: String,
}

impl Default for Layout {
    fn default() -> Self {
        Self {
            window_width: 0.0,
            window_height: 0.0,
            nav_width: 0.0,
            docs_width: 0.0,
            nav_open: None,
            docs_open: None,
            side_pane: String::new(),
            view: String::new(),
            version: String::new(),
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

    /// Was this record written by a different version of the
    /// application? `false` when there is no record at all — nothing to
    /// be surprised by on a first launch.
    pub fn written_by_another_version(&self) -> bool {
        !self.version.is_empty() && self.version != env!("CARGO_PKG_VERSION")
    }

    /// Write the record, stamping it with the version doing the writing.
    pub fn save(&mut self) {
        self.version = env!("CARGO_PKG_VERSION").to_owned();
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
    fn the_team_signs_by_its_initials() {
        let cfg: Config = toml::from_str(
            r#"
            [pharmacy]
            pharmacist = "Dr Claire Leroy, pharmacien titulaire"
            operators = [
              { initials = "CL", name = "Claire Leroy", role = "Pharmacien titulaire" },
              { initials = "MB", name = "Maya Bertrand" },
            ]
            "#,
        )
        .unwrap();
        // The initials are matched as typed at the counter, whatever
        // the case.
        assert_eq!(
            cfg.pharmacy.signature_for("cl"),
            "Claire Leroy, Pharmacien titulaire"
        );
        // A row without a qualité signs its name alone.
        assert_eq!(cfg.pharmacy.signature_for("MB"), "Maya Bertrand");
        // Unknown initials, or none at all, fall back to the officine's
        // own line: a document is never left unsigned.
        assert_eq!(
            cfg.pharmacy.signature_for("ZZ"),
            "Dr Claire Leroy, pharmacien titulaire"
        );
        assert_eq!(
            cfg.pharmacy.signature_for(""),
            "Dr Claire Leroy, pharmacien titulaire"
        );
        assert_eq!(
            cfg.pharmacy.operators[0].label(),
            "CL — Claire Leroy, Pharmacien titulaire"
        );
    }

    /// The Options dialog writes the whole configuration back with
    /// `toml::to_string_pretty`. An array of tables in the middle of a
    /// struct is exactly what that serializer refuses when the order is
    /// wrong — so the round trip is tested, not assumed.
    #[test]
    fn the_options_dialog_can_write_the_whole_configuration_back() {
        let mut cfg = Config::default();
        cfg.pharmacy.name = "Pharmacie du Centre".to_owned();
        cfg.pharmacy.am_number = "3400123".to_owned();
        cfg.pharmacy.operators = vec![
            Operator {
                initials: "CL".to_owned(),
                name: "Claire Leroy".to_owned(),
                role: "Pharmacien titulaire".to_owned(),
            },
            Operator {
                initials: "YS".to_owned(),
                name: "Yanis Saïd".to_owned(),
                role: String::new(),
            },
        ];
        cfg.disclaimers.ordonnance_footer = "Reconsulter si aggravation.".to_owned();
        cfg.disclaimers.plan = "Ce plan ne remplace pas votre ordonnance.".to_owned();
        cfg.disclaimers.conciliation = "Il ne vaut pas avis médical.".to_owned();
        cfg.ordonnance.adjuvant_tag = "probiotique".to_owned();
        let text = toml::to_string_pretty(&cfg).expect("la configuration doit se sérialiser");
        let back: Config = toml::from_str(&text).expect("et se relire");
        assert_eq!(back.pharmacy.operators, cfg.pharmacy.operators);
        assert_eq!(back.pharmacy.am_number, "3400123");
        assert_eq!(
            back.disclaimers.ordonnance_footer,
            "Reconsulter si aggravation."
        );
        assert_eq!(back.disclaimers.plan, cfg.disclaimers.plan);
        assert_eq!(back.disclaimers.conciliation, cfg.disclaimers.conciliation);
        assert_eq!(back.billing.bpm, cfg.billing.bpm);
        assert_eq!(back.rules.cycle_months, cfg.rules.cycle_months);
    }

    #[test]
    fn the_application_says_nothing_of_its_own_accord() {
        // Every mention is empty until the officine writes it: the
        // defaults print no disclaimer anywhere.
        let cfg = Config::default();
        assert_eq!(cfg.disclaimers.ordonnance_header, "");
        assert_eq!(cfg.disclaimers.ordonnance_footer, "");
        assert_eq!(cfg.disclaimers.ordonnance_screen, "");
        assert_eq!(cfg.disclaimers.carnet, "");
        assert_eq!(cfg.disclaimers.calculator, "");
        let written: Config = toml::from_str(
            r#"
            [disclaimers]
            ordonnance_footer = "Reconsulter en cas d'aggravation."
            "#,
        )
        .unwrap();
        assert_eq!(
            written.disclaimers.ordonnance_footer,
            "Reconsulter en cas d'aggravation."
        );
        assert_eq!(written.disclaimers.carnet, "");
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

    /// Completing the catalogue adds what is missing and rewrites
    /// nothing: a fee the officine typed, or a line it renamed, has to
    /// survive the button that fills the rest in.
    #[test]
    fn completing_the_forfaits_never_overwrites_what_is_there() {
        let mut cfg = LocationsConfig {
            forfaits: vec![LocationForfait {
                label: "nébuliseur".to_owned(),
                lpp: "notre ligne".to_owned(),
                period: Period::Month,
                fee: 12.5,
                renewal_days: 28,
                max_periods: 4,
            }],
            notice_days: 7,
        };
        let added = cfg.complete_forfaits();
        assert_eq!(added, 5, "les cinq autres, pas le nébuliseur");
        assert_eq!(cfg.forfaits.len(), 6);
        let mine = &cfg.forfaits[0];
        assert_eq!(mine.fee, 12.5);
        assert_eq!(mine.period, Period::Month);
        assert_eq!(mine.lpp, "notre ligne");
        // And it is idempotent: pressing it twice adds nothing.
        assert_eq!(cfg.complete_forfaits(), 0);
        assert_eq!(cfg.forfaits.len(), 6);
        // Every shipped line is usable and priced at nothing.
        assert!(cfg.forfaits.iter().all(|f| f.is_named()));
        assert!(cfg.forfaits[1..].iter().all(|f| f.fee == 0.0));
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

    /// The configuration's whole life on disk, in one test: a first
    /// launch that finds nothing and leaves a commented template, a
    /// save, a reload, and the paths every printed document is looked
    /// up through.
    ///
    /// One test and not five, deliberately: it moves `XDG_CONFIG_HOME`,
    /// which is process-wide, and two of them running side by side
    /// would each be reading the other's directory.
    #[test]
    fn the_configuration_survives_a_round_trip_through_the_disk() {
        let dir = std::env::temp_dir().join(format!("bpm-caddy-cfg-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let _swept = crate::db::Swept(dir.clone());
        std::env::set_var("XDG_CONFIG_HOME", &dir);

        let expected = dir.join("bpm-caddy").join("config.toml");
        assert_eq!(Config::path(), expected);
        assert!(!expected.exists());

        // First launch: nothing to read, defaults returned, and the
        // commented template left behind so the options are findable
        // without opening the documentation.
        let cfg = Config::load();
        assert!(expected.exists(), "le modèle commenté doit être écrit");
        let template = std::fs::read_to_string(&expected).unwrap();
        assert!(template.contains("[billing]"));
        assert!(template.contains("[locations]"));
        assert!(template.starts_with("# BPM-Caddy"));
        // Every option in the template is commented out: it documents
        // the defaults, it does not impose them.
        for line in template.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with('[') {
                continue;
            }
            panic!("option active dans le modèle : {line}");
        }
        // The catalogue ships filled in and every euro ships at zero:
        // the officine types the amount its own LPP line pays, and
        // nothing else.
        assert_eq!(cfg.locations.forfaits.len(), 6);
        assert!(cfg.locations.forfaits.iter().all(|f| f.is_named()));
        assert!(cfg.locations.forfaits.iter().all(|f| f.fee == 0.0));
        assert!(cfg.locations.forfaits.iter().all(|f| !f.lpp.is_empty()));
        assert_eq!(cfg.locations.notice_days, 7);

        // The template is written once: a second load does not stamp
        // over what the officine has since written.
        std::fs::write(
            &expected,
            "[ui]\ndensity = \"compact\"\noperator = \"CL\"\n",
        )
        .unwrap();
        let cfg = Config::load();
        assert_eq!(cfg.density(), motif::Density::Compact);
        assert_eq!(cfg.ui.operator, "CL");

        // Saving writes real TOML that loads back as itself.
        let mut cfg = Config::default();
        cfg.pharmacy.name = "Pharmacie du Centre".to_owned();
        cfg.rules.bpm_per_year = 4;
        cfg.locations.notice_days = 14;
        cfg.locations.forfaits = vec![LocationForfait {
            label: "Nébuliseur".to_owned(),
            lpp: "Titre I".to_owned(),
            period: Period::Week,
            fee: 12.0,
            renewal_days: 28,
            max_periods: 0,
        }];
        cfg.save().unwrap();
        let back = Config::load();
        assert_eq!(back.pharmacy.name, "Pharmacie du Centre");
        assert_eq!(back.per_year(crate::db::InterviewKind::Bpm), 4);
        assert_eq!(back.locations.notice_days, 14);
        // A list the officine has cut down stays cut down: what is
        // written wins over what is shipped.
        assert_eq!(back.locations.forfaits.len(), 1);
        assert_eq!(back.locations.forfaits[0].period, Period::Week);
        assert!(back.locations.forfaits[0].is_named());
        assert!(!LocationForfait::default().is_named());

        // The four templates sit beside config.toml unless the officine
        // pointed them somewhere else.
        let beside = dir.join("bpm-caddy");
        for path in [
            back.template_path(),
            back.cr_template_path(),
            back.carnet_template_path(),
            back.ordonnance_template_path(),
        ] {
            assert_eq!(path.parent().unwrap(), beside, "{path:?}");
        }
        let mut moved = back.clone();
        moved.templates.bpm_template_path = Some(PathBuf::from("/srv/modeles/bpm.typ"));
        assert_eq!(moved.template_path(), PathBuf::from("/srv/modeles/bpm.typ"));

        // The window geometry has its own little file, and refuses a
        // size a minimised window would have reported.
        assert_eq!(Layout::path(), beside.join("layout.toml"));
        let mut layout = Layout {
            window_width: 1600.0,
            window_height: 1000.0,
            nav_open: Some(true),
            docs_open: Some(false),
            side_pane: "carnet".to_owned(),
            view: "agenda".to_owned(),
            ..Layout::default()
        };
        // No record of a version until one is written.
        assert!(!layout.written_by_another_version());
        layout.save();
        let back = Layout::load();
        assert_eq!(back.window(), Some([1600.0, 1000.0]));
        // The whole shape of the workspace comes back, and the record
        // is stamped with the version that wrote it.
        assert_eq!(back.nav_open, Some(true));
        assert_eq!(back.docs_open, Some(false));
        assert_eq!(back.side_pane, "carnet");
        assert_eq!(back.view, "agenda");
        assert_eq!(back.version, env!("CARGO_PKG_VERSION"));
        assert!(!back.written_by_another_version());
        // A record left by an older release is recognised as one.
        let old = Layout {
            version: "0.1.0".to_owned(),
            ..Layout::default()
        };
        assert!(old.written_by_another_version());
        let tiny = Layout {
            window_width: 40.0,
            window_height: 20.0,
            ..Layout::default()
        };
        assert_eq!(
            tiny.window(),
            None,
            "une fenêtre réduite ne se mémorise pas"
        );
        // An unreadable file is « no record », never a panic.
        std::fs::write(Layout::path(), "ceci n'est pas du TOML {{{").unwrap();
        assert_eq!(Layout::load().window(), Layout::default().window());

        std::env::remove_var("XDG_CONFIG_HOME");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A period agrees with its count, and knows how long it is.
    #[test]
    fn a_period_knows_its_length_and_its_plural() {
        assert_eq!(Period::Day.days(), 1);
        assert_eq!(Period::Week.days(), 7);
        assert_eq!(Period::Month.days(), 30);
        assert_eq!(Period::default(), Period::Week);
        assert_eq!(Period::Week.agreed(1), "semaine");
        assert_eq!(Period::Week.agreed(11), "semaines");
        assert_eq!(Period::Day.agreed(0), "jour");
        assert_eq!(Period::Day.agreed(3), "jours");
        // « mois » does not take an s, at one or at twelve.
        assert_eq!(Period::Month.agreed(1), "mois");
        assert_eq!(Period::Month.agreed(12), "mois");
        // Every variant is in ALL, or the options list would hide one.
        assert_eq!(Period::ALL.len(), 3);
        for p in Period::ALL {
            assert!(!p.label().is_empty());
        }
    }

    /// The fee matrix is edited slot by slot in the options, and each
    /// slot must be the one the sequence actually bills.
    #[test]
    fn a_fee_slot_is_reachable_and_writable() {
        let mut fees = ActFees::staged([15.0, 15.0, 15.0, 20.0], [10.0, 20.0, 0.0, 0.0]);
        assert_eq!(*fees.slot_mut(0, 0), 15.0);
        assert_eq!(*fees.slot_mut(0, 3), 20.0);
        assert_eq!(*fees.slot_mut(1, 1), 20.0);
        *fees.slot_mut(1, 1) = 25.0;
        assert_eq!(*fees.slot_mut(1, 1), 25.0);
        // Out of range asks for a slot that does not exist: it must
        // clamp rather than panic in the middle of the options screen.
        let _ = *fees.slot_mut(9, 9);
    }
}
