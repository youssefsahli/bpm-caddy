//! Encrypted patient storage: SQLCipher (256-bit AES) via rusqlite.
//!
//! The whole database file is encrypted at rest; the key is the master
//! password entered at startup (SQLCipher runs its own KDF over it).

use std::path::{Path, PathBuf};

use rusqlite::Connection;

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS patients (
    id          INTEGER PRIMARY KEY,
    last_name   TEXT NOT NULL,
    first_name  TEXT NOT NULL,
    birth_date  TEXT NOT NULL,
    phone       TEXT NOT NULL DEFAULT '',
    notes       TEXT NOT NULL DEFAULT '',
    physician   TEXT NOT NULL DEFAULT '',
    email       TEXT NOT NULL DEFAULT '',
    address     TEXT NOT NULL DEFAULT '',
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE IF NOT EXISTS interviews (
    id          INTEGER PRIMARY KEY,
    patient_id  INTEGER NOT NULL REFERENCES patients(id),
    kind        TEXT NOT NULL,
    state       TEXT NOT NULL DEFAULT 'IDENTIFIED',
    duration_minutes INTEGER NOT NULL DEFAULT 0,
    scheduled_date TEXT,
    theme       TEXT NOT NULL DEFAULT '',
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE IF NOT EXISTS drugs (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL,
    dci         TEXT NOT NULL DEFAULT '',
    class       TEXT NOT NULL DEFAULT '',
    dosage      TEXT NOT NULL DEFAULT '',
    ddi         TEXT NOT NULL DEFAULT '',
    iup         TEXT NOT NULL DEFAULT '',
    indications TEXT NOT NULL DEFAULT '',
    mechanism   TEXT NOT NULL DEFAULT '',
    contraindications TEXT NOT NULL DEFAULT '',
    adverse     TEXT NOT NULL DEFAULT '',
    monitoring  TEXT NOT NULL DEFAULT '',
    sources     TEXT NOT NULL DEFAULT '',
    status      TEXT NOT NULL DEFAULT '',
    smr         TEXT NOT NULL DEFAULT '',
    tags        TEXT NOT NULL DEFAULT '',
    toxicity    TEXT NOT NULL DEFAULT '',
    antidote    TEXT NOT NULL DEFAULT '',
    notes       TEXT NOT NULL DEFAULT '',
    half_life   TEXT NOT NULL DEFAULT '',
    auc         TEXT NOT NULL DEFAULT '',
    elimination TEXT NOT NULL DEFAULT '',
    renal       TEXT NOT NULL DEFAULT '',
    pregnancy   TEXT NOT NULL DEFAULT ''
);
CREATE TABLE IF NOT EXISTS events (
    id          INTEGER PRIMARY KEY,
    day         TEXT NOT NULL,
    title       TEXT NOT NULL,
    category    TEXT NOT NULL DEFAULT 'AUTRE',
    created_at  TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))
);
CREATE TABLE IF NOT EXISTS patient_drugs (
    patient_id  INTEGER NOT NULL REFERENCES patients(id),
    drug_id     INTEGER NOT NULL REFERENCES drugs(id),
    PRIMARY KEY (patient_id, drug_id)
);
CREATE TABLE IF NOT EXISTS notes (
    id           INTEGER PRIMARY KEY,
    subject_kind TEXT NOT NULL,
    subject_id   INTEGER NOT NULL DEFAULT 0,
    operator     TEXT NOT NULL DEFAULT '',
    body         TEXT NOT NULL,
    created_at   TEXT NOT NULL DEFAULT (datetime('now', 'localtime'))
);
";

/// Idempotent migrations for databases created by older versions.
const MIGRATIONS: &[&str] = &[
    "ALTER TABLE interviews ADD COLUMN duration_minutes INTEGER NOT NULL DEFAULT 0",
    "ALTER TABLE interviews ADD COLUMN scheduled_date TEXT",
    "ALTER TABLE patients ADD COLUMN phone TEXT NOT NULL DEFAULT ''",
    "ALTER TABLE patients ADD COLUMN notes TEXT NOT NULL DEFAULT ''",
    "ALTER TABLE drugs ADD COLUMN dci TEXT NOT NULL DEFAULT ''",
    "ALTER TABLE drugs ADD COLUMN class TEXT NOT NULL DEFAULT ''",
    "ALTER TABLE drugs ADD COLUMN half_life TEXT NOT NULL DEFAULT ''",
    "ALTER TABLE drugs ADD COLUMN auc TEXT NOT NULL DEFAULT ''",
    "ALTER TABLE drugs ADD COLUMN elimination TEXT NOT NULL DEFAULT ''",
    "ALTER TABLE drugs ADD COLUMN renal TEXT NOT NULL DEFAULT ''",
    "ALTER TABLE drugs ADD COLUMN pregnancy TEXT NOT NULL DEFAULT ''",
    "ALTER TABLE patients ADD COLUMN physician TEXT NOT NULL DEFAULT ''",
    "ALTER TABLE patients ADD COLUMN email TEXT NOT NULL DEFAULT ''",
    "ALTER TABLE patients ADD COLUMN address TEXT NOT NULL DEFAULT ''",
    "ALTER TABLE interviews ADD COLUMN theme TEXT NOT NULL DEFAULT ''",
    "ALTER TABLE drugs ADD COLUMN indications TEXT NOT NULL DEFAULT ''",
    "ALTER TABLE drugs ADD COLUMN mechanism TEXT NOT NULL DEFAULT ''",
    "ALTER TABLE drugs ADD COLUMN contraindications TEXT NOT NULL DEFAULT ''",
    "ALTER TABLE drugs ADD COLUMN adverse TEXT NOT NULL DEFAULT ''",
    "ALTER TABLE drugs ADD COLUMN monitoring TEXT NOT NULL DEFAULT ''",
    "ALTER TABLE drugs ADD COLUMN sources TEXT NOT NULL DEFAULT ''",
    "ALTER TABLE drugs ADD COLUMN status TEXT NOT NULL DEFAULT ''",
    "ALTER TABLE drugs ADD COLUMN smr TEXT NOT NULL DEFAULT ''",
    "ALTER TABLE drugs ADD COLUMN tags TEXT NOT NULL DEFAULT ''",
    "ALTER TABLE drugs ADD COLUMN toxicity TEXT NOT NULL DEFAULT ''",
];

/// Interview lifecycle (spec section 5): a strict pipeline so no billable
/// act is ever lost.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum InterviewState {
    Identified,
    Scheduled,
    Performed,
    ReportSent,
    Billed,
}

impl InterviewState {
    pub const ALL: [InterviewState; 5] = [
        Self::Identified,
        Self::Scheduled,
        Self::Performed,
        Self::ReportSent,
        Self::Billed,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Identified => "IDENTIFIED",
            Self::Scheduled => "SCHEDULED",
            Self::Performed => "PERFORMED",
            Self::ReportSent => "REPORT_SENT",
            Self::Billed => "BILLED",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|st| st.as_str() == s)
    }

    /// French label shown in the UI.
    pub fn label(self) -> &'static str {
        match self {
            Self::Identified => "Identifié",
            Self::Scheduled => "Planifié",
            Self::Performed => "Réalisé",
            Self::ReportSent => "CR envoyé",
            Self::Billed => "Facturé",
        }
    }

    /// The next pipeline step, or `None` once billed.
    pub fn next(self) -> Option<Self> {
        let i = Self::ALL.iter().position(|s| *s == self)?;
        Self::ALL.get(i + 1).copied()
    }

    /// The previous pipeline step, so a misclicked advance can be undone.
    pub fn prev(self) -> Option<Self> {
        let i = Self::ALL.iter().position(|s| *s == self)?;
        i.checked_sub(1).and_then(|j| Self::ALL.get(j).copied())
    }
}

/// Act kinds billable at the counter.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InterviewKind {
    Bpm,
    Aod,
    Asthme,
    /// Test rapide d'orientation diagnostique — angine.
    TrodAngine,
    /// Test rapide d'orientation diagnostique — cystite.
    TrodCystite,
    /// Rendez-vous de prévention.
    Prevention,
    /// Accompagnement patient sous AVK.
    Avk,
    /// Accompagnement patient sous anticancéreux oraux.
    Anticancereux,
    /// Vaccination à l'officine.
    Vaccination,
}

impl InterviewKind {
    pub const ALL: [InterviewKind; 9] = [
        Self::Bpm,
        Self::Aod,
        Self::Avk,
        Self::Asthme,
        Self::Anticancereux,
        Self::TrodAngine,
        Self::TrodCystite,
        Self::Vaccination,
        Self::Prevention,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Bpm => "BPM",
            Self::Aod => "AOD",
            Self::Asthme => "ASTHME",
            Self::TrodAngine => "TROD_ANGINE",
            Self::TrodCystite => "TROD_CYSTITE",
            Self::Prevention => "PREVENTION",
            Self::Avk => "AVK",
            Self::Anticancereux => "ANTICANCEREUX",
            Self::Vaccination => "VACCINATION",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.as_str() == s)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Bpm => "BPM",
            Self::Aod => "AOD",
            Self::Asthme => "Asthme",
            Self::TrodAngine => "TROD angine",
            Self::TrodCystite => "TROD cystite",
            Self::Prevention => "Prévention",
            Self::Avk => "AVK",
            Self::Anticancereux => "Anticancéreux",
            Self::Vaccination => "Vaccination",
        }
    }
}

/// The classic entretien thematics, offered as a quick pick on each
/// interview row (free choice — any kind can use any theme).
pub const THEMES: &[&str] = &[
    "Initiation / bon usage",
    "Observance",
    "Biologie / INR",
    "Effets indésirables",
    "Interactions",
    "Technique d'inhalation",
    "Vie quotidienne / diététique",
    "Automédication",
];

/// Dashboard aggregate: one row per interview, month-granular.
#[derive(Clone, Debug)]
pub struct InterviewSummary {
    pub kind: InterviewKind,
    pub state: InterviewState,
    /// `YYYY-MM` of creation — used to place pending revenue.
    pub created_month: String,
    /// `YYYY-MM` of the last state change — used to place billed revenue.
    pub updated_month: String,
    pub duration_minutes: i64,
    /// 0-based rank of this act inside its yearly cycle (0 = entretien
    /// initial) — selects the fee slot.
    pub fee_rank: usize,
}

/// One interview joined with its patient, for the CSV export.
#[derive(Clone, Debug)]
pub struct ExportRow {
    pub patient_name: String,
    /// May be empty.
    pub phone: String,
    /// ISO `YYYY-MM-DD`.
    pub birth_date: String,
    pub kind: InterviewKind,
    pub state: InterviewState,
    /// ISO `YYYY-MM-DD`.
    pub created_date: String,
    /// ISO `YYYY-MM-DD`, when planned.
    pub scheduled_date: Option<String>,
    pub duration_minutes: i64,
    pub theme: String,
    /// See [`InterviewSummary::fee_rank`].
    pub fee_rank: usize,
}

/// What an agenda entry that is not a billable act represents.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EventCategory {
    Formation,
    Reunion,
    Livraison,
    Conge,
    Autre,
}

impl EventCategory {
    pub const ALL: [EventCategory; 5] = [
        Self::Formation,
        Self::Reunion,
        Self::Livraison,
        Self::Conge,
        Self::Autre,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Formation => "FORMATION",
            Self::Reunion => "REUNION",
            Self::Livraison => "LIVRAISON",
            Self::Conge => "CONGE",
            Self::Autre => "AUTRE",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.as_str() == s)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Formation => "Formation",
            Self::Reunion => "Réunion",
            Self::Livraison => "Livraison",
            Self::Conge => "Congé",
            Self::Autre => "Autre",
        }
    }
}

/// One agenda entry that is not tied to a patient.
#[derive(Clone, Debug)]
pub struct Event {
    pub id: i64,
    /// ISO `YYYY-MM-DD`.
    pub day: String,
    pub title: String,
    pub category: EventCategory,
}

/// A planned interview with the patient it belongs to, for the
/// dashboard's upcoming-appointments list.
#[derive(Clone, Debug)]
pub struct Appointment {
    pub patient_id: i64,
    pub patient_name: String,
    /// May be empty — shown so the patient can be called about the RDV.
    pub phone: String,
    pub kind: InterviewKind,
    /// ISO `YYYY-MM-DD`.
    pub date: String,
}

#[derive(Clone, Debug)]
pub struct Interview {
    pub id: i64,
    pub kind: InterviewKind,
    pub state: InterviewState,
    pub duration_minutes: i64,
    /// ISO `YYYY-MM-DD`, set when the interview is scheduled.
    pub scheduled_date: Option<String>,
    pub theme: String,
    pub created_at: String,
}

/// The administrative statuses the base recognises. Anything else is
/// kept as free text and shown as written.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DrugStatus {
    /// Commercialisé, rien à signaler.
    Marketed,
    /// Rupture ou tension d'approvisionnement.
    Shortage,
    /// Retiré du marché ou arrêt de commercialisation.
    Withdrawn,
    /// Prescription hors AMM, ATU/accès précoce, ou non remboursé.
    OffLabel,
}

impl DrugStatus {
    /// Recognise a status from the text on the card, tolerant of case
    /// and of the usual French wordings.
    pub fn parse(text: &str) -> Option<Self> {
        let t = crate::fuzzy::sort_key(text);
        if t.is_empty() {
            return None;
        }
        if t.contains("rupture") || t.contains("tension") || t.contains("contingent") {
            Some(Self::Shortage)
        } else if t.contains("retir") || t.contains("arret") || t.contains("suspendu") {
            Some(Self::Withdrawn)
        } else if t.contains("hors amm")
            || t.contains("sans amm")
            || t.contains("acces precoce")
            || t.contains("atu")
            || t.contains("non rembours")
        {
            Some(Self::OffLabel)
        } else if t.contains("commercialis") || t.contains("disponible") {
            Some(Self::Marketed)
        } else {
            None
        }
    }
}

/// One entry of the team's drug reference base (shared, encrypted with
/// the patient data): the facts wanted at the counter in one glance.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Drug {
    pub id: i64,
    pub name: String,
    /// Dénomination commune internationale (INN).
    pub dci: String,
    /// Therapeutic class ("statine", "AOD", …).
    pub class: String,
    /// Usual dosage / posology.
    pub dosage: String,
    /// Drug-drug interactions to watch for.
    pub ddi: String,
    /// IUP.
    pub iup: String,
    /// Indications retenues à l'officine.
    pub indications: String,
    /// Mécanisme d'action, en une ou deux phrases.
    pub mechanism: String,
    /// Contre-indications.
    pub contraindications: String,
    /// Effets indésirables à connaître.
    pub adverse: String,
    /// Surveillance biologique et clinique.
    pub monitoring: String,
    /// Références, une par ligne : numérotées à l'affichage.
    pub sources: String,
    /// Statut administratif : commercialisé, retiré, rupture, hors
    /// AMM… ([`DrugStatus`] reconnaît les valeurs usuelles).
    pub status: String,
    /// Dernière évaluation SMR / ASMR de la commission de la
    /// transparence, telle que notée par l'équipe.
    pub smr: String,
    /// Étiquettes libres, séparées par des virgules : elles filtrent la
    /// recherche et se lisent sur la fiche.
    pub tags: String,
    /// Niveau de toxicité / marge thérapeutique, avec les DI et CI
    /// retenues dans la littérature.
    pub toxicity: String,
    pub antidote: String,
    /// The team's own notes.
    pub notes: String,
    /// Demi-vie d'élimination.
    pub half_life: String,
    /// AUC / exposition (base des interactions).
    pub auc: String,
    /// Voie d'élimination (rénale, hépatique, mixte…).
    pub elimination: String,
    /// Adaptation posologique selon le DFG.
    pub renal: String,
    /// Grossesse / allaitement.
    pub pregnancy: String,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Patient {
    pub id: i64,
    pub last_name: String,
    pub first_name: String,
    /// ISO `YYYY-MM-DD`.
    pub birth_date: String,
    /// Free-form, may be empty ("06 12 34 56 78").
    pub phone: String,
    /// Free-form counter note (allergies, préférences…), may be empty.
    pub notes: String,
    /// Médecin traitant (the interview report goes to them).
    pub physician: String,
    pub email: String,
    pub address: String,
}

impl Patient {
    pub fn full_name(&self) -> String {
        format!("{} {}", self.first_name, self.last_name)
    }
}

/// What a standalone note is attached to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoteSubject {
    Patient,
    Drug,
    /// Personal notes, keyed by the operator initials.
    Operator,
    /// The team's end-of-day handover logbook, organized by day.
    Transmission,
    /// A note pinned to one day of the agenda, keyed by `YYYYMMDD`.
    Day,
}

/// The subject id of a day note: `2026-08-25` becomes `20260825`.
pub fn day_subject_id(iso: &str) -> i64 {
    iso.chars()
        .filter(char::is_ascii_digit)
        .collect::<String>()
        .parse()
        .unwrap_or(0)
}

impl NoteSubject {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Patient => "PATIENT",
            Self::Drug => "DRUG",
            Self::Operator => "OPERATOR",
            Self::Day => "DAY",
            Self::Transmission => "TRANSMISSION",
        }
    }
}

/// One dated, author-stamped note (append-only journal).
#[derive(Clone, Debug, PartialEq)]
pub struct Note {
    pub id: i64,
    pub operator: String,
    pub body: String,
    /// `YYYY-MM-DD HH:MM:SS` local time.
    pub created_at: String,
}

impl Note {
    /// "24/08 14:32" for display.
    pub fn stamp(&self) -> String {
        let date = self.created_at.get(..10).unwrap_or("");
        let time = self.created_at.get(11..16).unwrap_or("");
        match (date.get(8..10), date.get(5..7)) {
            (Some(d), Some(m)) => format!("{d}/{m} {time}"),
            _ => self.created_at.clone(),
        }
    }
}

/// Clinical detail shipped for the drugs the supported entretiens turn
/// around (AOD, AVK, HBPM, asthme, marges thérapeutiques étroites,
/// anticancéreux oraux). These are the standard reference facts a
/// pharmacist would state at the counter; every field stays editable
/// and the team's own text is never overwritten. Cards outside this
/// list keep their clinical fields empty on purpose.
pub struct StarterDetail {
    pub name: &'static str,
    pub indications: &'static str,
    pub mechanism: &'static str,
    pub dosage: &'static str,
    pub contraindications: &'static str,
    pub ddi: &'static str,
    pub adverse: &'static str,
    pub monitoring: &'static str,
    /// Informations utiles au patient (IUP) : plan de prise, technique,
    /// signaux d'alerte.
    pub iup: &'static str,
    pub half_life: &'static str,
    pub elimination: &'static str,
    pub renal: &'static str,
    pub pregnancy: &'static str,
    /// One reference per line, numbered when displayed.
    pub sources: &'static str,
    pub status: &'static str,
    pub smr: &'static str,
    pub tags: &'static str,
    pub toxicity: &'static str,
}

pub const STARTER_DETAILS: &[StarterDetail] = &[
    StarterDetail {
        name: "Eliquis",
        indications: "Prévention de l'accident vasculaire cérébral et de l'embolie systémique dans la fibrillation atriale non valvulaire avec au moins un facteur de risque ; traitement et prévention des récidives de thrombose veineuse profonde et d'embolie pulmonaire ; prévention de la MTEV après chirurgie programmée de hanche ou de genou.",
        mechanism: "Inhibiteur direct, sélectif et réversible du facteur Xa, libre et lié au caillot, sans passer par l'antithrombine. L'inhibition du Xa interrompt la conversion de la prothrombine en thrombine et donc l'amplification de la coagulation.",
        dosage: "Fibrillation atriale : 5 mg deux fois par jour ; 2,5 mg deux fois par jour si au moins deux critères parmi âge ≥ 80 ans, poids ≤ 60 kg, créatininémie ≥ 133 µmol/L. Maladie thromboembolique veineuse : 10 mg deux fois par jour pendant 7 jours, puis 5 mg deux fois par jour ; prévention des récidives au-delà de 6 mois : 2,5 mg deux fois par jour. Chirurgie orthopédique : 2,5 mg deux fois par jour.",
        contraindications: "Saignement évolutif cliniquement significatif, lésion à risque hémorragique majeur, hépatopathie avec coagulopathie, association à un autre anticoagulant hors relais, prothèse valvulaire mécanique, syndrome des antiphospholipides triple positif, grossesse et allaitement, clairance inférieure à 15 mL/min ou dialyse.",
        ddi: "Inhibiteurs puissants du CYP3A4 et de la P-gp (kétoconazole, itraconazole, ritonavir, clarithromycine) : exposition augmentée, association déconseillée. Inducteurs puissants (rifampicine, carbamazépine, phénytoïne, millepertuis) : exposition diminuée, efficacité compromise. AINS, aspirine, antiagrégants, ISRS et IRSNA : risque hémorragique additif.",
        adverse: "Saignements de toutes localisations, le plus souvent mineurs (épistaxis, gingivorragies, ecchymoses, ménorragies) ; anémie ; hématomes. Plus rarement hémorragie digestive ou intracrânienne. Nausées et élévation modérée des transaminases.",
        monitoring: "Aucune surveillance de routine de la coagulation : les tests usuels ne reflètent pas l'effet. Créatininémie et clairance de Cockcroft au moins une fois par an, plus souvent au-delà de 75 ans, en cas de poids faible ou de clairance inférieure à 60 mL/min. Hémogramme si saignement. Réévaluer l'indication et la dose à chaque renouvellement.",
        iup: "Deux prises par jour à heure fixe, avec ou sans aliments. Oubli : prendre le comprimé dès que possible le jour même, puis reprendre le rythme habituel, jamais deux comprimés à la fois. Ne jamais interrompre le traitement sans avis médical, même quelques jours. Prévenir tout médecin, dentiste ou chirurgien avant un geste. Signaler saignement prolongé, selles noires, urines rouges, hématomes inhabituels, maux de tête violents ou chute avec choc à la tête.",
        half_life: "≈ 12 heures",
        elimination: "Élimination majoritairement biliaire et fécale ; environ 25 % rénale sous forme inchangée. Métabolisme partiel par le CYP3A4, substrat de la P-gp.",
        renal: "Clairance 30 à 50 mL/min : dose habituelle, réduite si les autres critères sont réunis. 15 à 29 mL/min : 2,5 mg deux fois par jour, prudence. Inférieure à 15 mL/min ou dialyse : non recommandé.",
        pregnancy: "Contre-indiqué pendant la grossesse et l'allaitement ; relais par une héparine de bas poids moléculaire si une anticoagulation est nécessaire.",
        sources: "RCP Eliquis — base de données publique des médicaments (ANSM)\nESC 2020 — prise en charge de la fibrillation atriale\nHAS — bon usage des anticoagulants oraux directs",
        status: "",
        smr: "",
        tags: "aod, surveillance biologique, contre-indiqué grossesse",
        toxicity: "",
    },
    StarterDetail {
        name: "Xarelto",
        indications: "Fibrillation atriale non valvulaire avec facteur de risque ; traitement et prévention des récidives de thrombose veineuse profonde et d'embolie pulmonaire ; prévention de la MTEV après chirurgie de hanche ou de genou ; à faible dose, en association à l'aspirine, prévention des événements athérothrombotiques dans la maladie coronarienne ou artérielle périphérique.",
        mechanism: "Inhibiteur direct et sélectif du facteur Xa. Le blocage du Xa au sein du complexe prothrombinase arrête la génération de thrombine, sans action sur la thrombine déjà formée.",
        dosage: "Fibrillation atriale : 20 mg une fois par jour au cours du repas ; 15 mg si clairance 15 à 49 mL/min. Maladie thromboembolique veineuse : 15 mg deux fois par jour pendant 21 jours puis 20 mg une fois par jour ; prévention prolongée : 10 mg par jour. Chirurgie orthopédique : 10 mg par jour.",
        contraindications: "Saignement évolutif, lésion à haut risque hémorragique, coagulopathie hépatique (Child B et C), association à un autre anticoagulant, prothèse valvulaire mécanique, syndrome des antiphospholipides triple positif, grossesse et allaitement, clairance inférieure à 15 mL/min.",
        ddi: "Inhibiteurs puissants du CYP3A4 et de la P-gp (azolés systémiques, inhibiteurs de protéase) : association déconseillée. Inducteurs puissants : efficacité compromise. AINS, aspirine, antiagrégants, ISRS : risque hémorragique additif. Le millepertuis est à proscrire.",
        adverse: "Saignements de toutes localisations, anémie, hématomes ; hémorragies digestives un peu plus fréquentes qu'avec les AVK ; élévation des transaminases, nausées, prurit.",
        monitoring: "Pas de surveillance biologique de routine. Clairance de la créatinine au moins annuelle, plus souvent chez le sujet âgé ou fragile. Hémoglobine en cas de saignement ou de fatigue inexpliquée.",
        iup: "Une prise par jour au cours d'un repas : l'absorption du dosage à 15 et 20 mg en dépend directement, un comprimé pris à jeun est sous-dosé. Oubli : prendre dès que possible le jour même. Ne jamais interrompre sans avis. Prévenir tout praticien avant un geste invasif. Signaler tout saignement anormal, selles noires, maux de tête inhabituels.",
        half_life: "5 à 9 heures chez l'adulte jeune, 11 à 13 heures chez le sujet âgé",
        elimination: "Environ un tiers éliminé par voie rénale sous forme inchangée, le reste métabolisé (CYP3A4, CYP2J2) puis éliminé par voies rénale et fécale.",
        renal: "Clairance 15 à 49 mL/min : 15 mg par jour en fibrillation atriale. Inférieure à 15 mL/min : non recommandé.",
        pregnancy: "Contre-indiqué pendant la grossesse et l'allaitement.",
        sources: "RCP Xarelto — base de données publique des médicaments (ANSM)\nESC 2020 — fibrillation atriale\nHAS — bon usage des anticoagulants oraux directs",
        status: "",
        smr: "",
        tags: "aod, contre-indiqué grossesse",
        toxicity: "",
    },
    StarterDetail {
        name: "Pradaxa",
        indications: "Prévention de l'AVC et de l'embolie systémique dans la fibrillation atriale non valvulaire ; traitement et prévention des récidives de MTEV après une héparine initiale ; prévention de la MTEV après chirurgie de hanche ou de genou.",
        mechanism: "Prodrogue du dabigatran, inhibiteur direct, compétitif et réversible de la thrombine, libre comme liée au caillot. En bloquant la thrombine, il empêche la conversion du fibrinogène en fibrine.",
        dosage: "Fibrillation atriale et MTEV : 150 mg deux fois par jour ; 110 mg deux fois par jour à partir de 80 ans, en cas d'association au vérapamil, et à considérer entre 75 et 80 ans, en cas de clairance 30 à 50 mL/min, de gastrite ou de risque hémorragique élevé.",
        contraindications: "Clairance inférieure à 30 mL/min, saignement évolutif, lésion à risque hémorragique majeur, insuffisance hépatique sévère, prothèse valvulaire mécanique, association au kétoconazole systémique, à la ciclosporine, à l'itraconazole ou à la dronédarone, grossesse et allaitement.",
        ddi: "Substrat de la P-gp sans métabolisme par les cytochromes : dronédarone, kétoconazole, itraconazole et ciclosporine sont contre-indiqués ; vérapamil et amiodarone augmentent l'exposition (réduction de dose) ; rifampicine et millepertuis la diminuent. AINS et antiagrégants majorent le risque hémorragique.",
        adverse: "Saignements, dyspepsie et douleurs épigastriques nettement plus fréquentes qu'avec les autres AOD (excipient acide tartrique), hémorragies digestives, anémie.",
        monitoring: "Clairance de la créatinine avant l'instauration puis au moins une fois par an, et à chaque épisode susceptible d'altérer la fonction rénale (déshydratation, infection, produit de contraste). Hémogramme en cas de saignement.",
        iup: "Gélules à avaler entières : ne jamais les ouvrir ni les croquer, la biodisponibilité doublerait. Les conserver dans le blister ou le flacon d'origine, jamais dans un pilulier : elles craignent l'humidité ; un flacon entamé se garde quatre mois. Deux prises par jour à heure fixe, avec un grand verre d'eau, de préférence au cours d'un repas pour limiter les brûlures d'estomac. Signaler tout saignement anormal.",
        half_life: "12 à 17 heures, allongée en cas d'insuffisance rénale",
        elimination: "Environ 80 % éliminé par voie rénale sous forme inchangée : la fonction rénale conditionne l'exposition.",
        renal: "Clairance 30 à 50 mL/min : envisager 110 mg deux fois par jour. Inférieure à 30 mL/min : contre-indiqué. Dialysable, contrairement aux anti-Xa.",
        pregnancy: "Contre-indiqué pendant la grossesse et l'allaitement.",
        sources: "RCP Pradaxa — base de données publique des médicaments (ANSM)\nESC 2020 — fibrillation atriale\nHAS — bon usage des anticoagulants oraux directs",
        status: "",
        smr: "",
        tags: "aod, contre-indiqué grossesse",
        toxicity: "",
    },
    StarterDetail {
        name: "Lixiana",
        indications: "Fibrillation atriale non valvulaire avec facteur de risque ; traitement et prévention des récidives de thrombose veineuse profonde et d'embolie pulmonaire, après au moins cinq jours d'héparine.",
        mechanism: "Inhibiteur direct et sélectif du facteur Xa, d'action rapide, avec une prise quotidienne unique.",
        dosage: "60 mg une fois par jour. 30 mg une fois par jour si clairance 15 à 50 mL/min, poids inférieur ou égal à 60 kg, ou association à un inhibiteur puissant de la P-gp (ciclosporine, dronédarone, érythromycine, kétoconazole).",
        contraindications: "Saignement évolutif, lésion à risque, hypertension artérielle sévère non contrôlée, coagulopathie hépatique, association à un autre anticoagulant, prothèse valvulaire mécanique, syndrome des antiphospholipides triple positif, grossesse et allaitement, clairance inférieure à 15 mL/min.",
        ddi: "Inhibiteurs de la P-gp : réduction de dose à 30 mg. Inducteurs puissants (rifampicine, millepertuis, carbamazépine) : efficacité diminuée. AINS, aspirine, antiagrégants et ISRS : risque hémorragique additif.",
        adverse: "Saignements de toutes localisations, anémie, épistaxis, hématurie ; éruptions cutanées et élévation des transaminases plus rarement.",
        monitoring: "Clairance de la créatinine et poids à l'instauration puis au moins une fois par an : les deux commandent la dose. Efficacité moindre décrite lorsque la clairance dépasse 95 mL/min en fibrillation atriale.",
        iup: "Une prise par jour à heure fixe, avec ou sans aliments. Oubli : prendre dès que possible le jour même, jamais deux doses. Signaler toute variation importante de poids, qui peut faire changer la dose. Prévenir avant tout geste invasif ; ne jamais arrêter de soi-même.",
        half_life: "10 à 14 heures",
        elimination: "Environ 50 % éliminé par voie rénale sous forme inchangée, le reste par voie biliaire et intestinale.",
        renal: "Clairance 15 à 50 mL/min : 30 mg par jour. Inférieure à 15 mL/min : non recommandé.",
        pregnancy: "Contre-indiqué pendant la grossesse et l'allaitement.",
        sources: "RCP Lixiana — base de données publique des médicaments (ANSM)\nESC 2020 — fibrillation atriale",
        status: "",
        smr: "",
        tags: "aod, surveillance biologique, contre-indiqué grossesse",
        toxicity: "",
    },
    StarterDetail {
        name: "Coumadine",
        indications: "Prévention des complications thromboemboliques des cardiopathies emboligènes, en particulier de la fibrillation atriale, et des prothèses valvulaires mécaniques, indication dans laquelle les anticoagulants oraux directs n'ont pas leur place. Traitement des thromboses veineuses profondes et des embolies pulmonaires et prévention de leurs récidives. Prévention des complications thromboemboliques des infarctus du myocarde compliqués.",
        mechanism: "Antivitamine K de la famille des coumariniques : la warfarine inhibe la vitamine K époxyde réductase (VKORC1) et bloque la régénération de la vitamine K réduite. La gamma-carboxylation hépatique des facteurs II, VII, IX et X, ainsi que des protéines C et S, ne se fait plus, et il se forme des facteurs inactifs. L'effet n'apparaît qu'après épuisement des facteurs déjà circulants, soit deux à quatre jours.",
        dosage: "Comprimés sécables à 2 mg et 5 mg, en une prise quotidienne, de préférence le soir. Dose initiale usuelle de 5 mg par jour chez l'adulte, réduite à 4 mg chez le sujet âgé, de faible poids ou insuffisant hépatique, sans dose de charge. La posologie est ensuite ajustée uniquement sur l'INR, par paliers d'environ 1 mg et jamais plus d'une fois tous les deux à trois jours. La cible est un INR entre 2 et 3 dans la majorité des indications, une zone plus élevée pouvant être retenue pour certaines prothèses valvulaires mécaniques selon leur type et leur position.",
        contraindications: "Hypersensibilité à la warfarine, insuffisance hépatique sévère, saignement évolutif ou lésion organique à risque hémorragique, hypertension artérielle maligne, accident vasculaire cérébral hémorragique récent, intervention neurochirurgicale ou ophtalmologique récente, association au miconazole y compris en gel buccal, au millepertuis, à l'aspirine à dose antalgique ou anti-inflammatoire et aux AINS pyrazolés, grossesse en dehors de situations exceptionnelles, impossibilité de réaliser la surveillance de l'INR.",
        ddi: "Le miconazole, même en gel buccal ou en ovule, et le millepertuis sont contre-indiqués, l'un majorant massivement l'INR, l'autre l'effondrant. Majorent l'effet anticoagulant : amiodarone, cotrimoxazole, métronidazole, fluconazole et azolés, macrolides, fluoroquinolones, allopurinol, tamoxifène, fibrates, statines pour certaines. Le diminuent : rifampicine, carbamazépine, phénytoïne, phénobarbital, cholestyramine, aprépitant. Aspirine, AINS, antiagrégants, corticoïdes et ISRS augmentent le risque hémorragique sans forcément modifier l'INR. Tout ajout ou arrêt de médicament justifie un INR de contrôle trois à cinq jours plus tard.",
        adverse: "Hémorragies de toutes localisations, des gingivorragies et ecchymoses banales aux hémorragies digestives, urinaires ou intracrâniennes graves, favorisées par un surdosage. Plus rarement, nécrose cutanée en début de traitement chez les déficitaires en protéine C ou S, alopécie, embolies de cristaux de cholestérol, calciphylaxie, réactions cutanées allergiques.",
        monitoring: "INR au deuxième ou troisième jour de traitement, puis tous les deux à trois jours jusqu'à obtention de deux INR successifs dans la zone cible, puis espacement progressif sans jamais dépasser un mois entre deux contrôles. Contrôle supplémentaire systématique après toute modification de dose, tout ajout ou arrêt de médicament, tout épisode intercurrent, fièvre, diarrhée ou dénutrition. Hémogramme en cas de saignement ou d'anémie.",
        iup: "Une seule prise par jour, toujours à la même heure et de préférence le soir, ce qui permet d'ajuster la dose dès le lendemain matin si l'INR revient anormal. En cas d'oubli constaté dans les huit heures, prendre le comprimé ; au-delà, sauter la prise et ne jamais doubler la dose le lendemain, puis noter l'oubli dans le carnet de suivi. Le carnet AVK doit accompagner chaque prise de sang et être montré à tout médecin, dentiste, infirmier ou pharmacien. Aucune automédication, en particulier aspirine, ibuprofène et gel buccal contre les aphtes ; le paracétamol est l'antalgique de référence, à signaler tout de même s'il est pris plusieurs jours de suite. Il n'est pas nécessaire de supprimer les choux, brocolis, épinards ou salades, mais il faut en manger de façon régulière d'une semaine à l'autre, sans excès brutal. Consulter sans délai devant un saignement qui ne s'arrête pas, des selles noires, des urines rouges, des hématomes spontanés, une fatigue ou un essoufflement inhabituels, et après tout choc à la tête même sans plaie.",
        half_life: "35 à 45 heures",
        elimination: "Métabolisme hépatique presque complet, l'énantiomère S le plus actif par le CYP2C9, l'énantiomère R par les CYP1A2 et CYP3A4 ; les métabolites inactifs sont éliminés dans les urines et la bile.",
        renal: "Pas d'adaptation posologique liée à la clairance, la posologie étant guidée par l'INR ; prudence et contrôles rapprochés en cas d'insuffisance rénale sévère, qui majore le risque hémorragique.",
        pregnancy: "Contre-indiqué pendant la grossesse en raison du risque d'embryopathie au premier trimestre et d'hémorragie fœtale et néonatale en fin de grossesse, sauf situation exceptionnelle comme certaines prothèses mécaniques ; l'allaitement est possible, la warfarine passant très peu dans le lait.",
        sources: "RCP Coumadine — base de données publique des médicaments (ANSM)\nHAS — bon usage des antivitamines K et carnet de suivi AVK\nESC 2020 — prise en charge de la fibrillation atriale",
        status: "",
        smr: "",
        tags: "avk, marge thérapeutique étroite, surveillance biologique, contre-indiqué grossesse",
        toxicity: "Marge thérapeutique étroite : un écart de dose ou une interaction suffit à faire basculer vers le sous-dosage ou la toxicité. Voir les sections Interactions et Surveillance.",
    },
    StarterDetail {
        name: "Previscan",
        indications: "Prévention des complications thromboemboliques des cardiopathies emboligènes et des prothèses valvulaires mécaniques, traitement des thromboses veineuses profondes et des embolies pulmonaires et prévention de leurs récidives, prévention des complications thromboemboliques des infarctus du myocarde compliqués. Depuis l'alerte de l'ANSM sur le risque immuno-allergique, la fluindione ne doit plus être instaurée chez un nouveau patient : elle n'est poursuivie que chez les patients déjà traités et équilibrés depuis plus de six mois.",
        mechanism: "Antivitamine K de la famille des indanediones. La fluindione inhibe la vitamine K époxyde réductase et empêche la gamma-carboxylation hépatique des facteurs II, VII, IX et X et des protéines C et S, qui sont alors produits sous une forme inactive. L'effet anticoagulant complet demande deux à quatre jours, le temps que les facteurs déjà synthétisés disparaissent.",
        dosage: "Comprimés quadrisécables à 20 mg, en une prise quotidienne à heure fixe, de préférence le soir. La dose initiale habituelle est d'un comprimé à 20 mg par jour chez l'adulte, réduite chez le sujet âgé, puis ajustée exclusivement sur l'INR par paliers d'un quart de comprimé, soit 5 mg, en respectant deux à trois jours entre deux modifications. Cible d'INR entre 2 et 3 dans la plupart des indications, plus élevée pour certaines prothèses valvulaires mécaniques.",
        contraindications: "Hypersensibilité à la fluindione ou antécédent d'accident immuno-allergique sous indanedione, insuffisance hépatique sévère, insuffisance rénale sévère, saignement évolutif ou lésion à risque hémorragique, accident vasculaire cérébral hémorragique récent, association au miconazole, au millepertuis, à l'aspirine anti-inflammatoire et aux AINS pyrazolés, grossesse et allaitement, impossibilité de surveiller l'INR.",
        ddi: "Miconazole sous toutes ses formes et millepertuis contre-indiqués. Augmentent l'INR : amiodarone, cotrimoxazole, métronidazole, azolés antifongiques, macrolides, fluoroquinolones, allopurinol, tamoxifène. Le diminuent : rifampicine, carbamazépine, phénytoïne, phénobarbital, cholestyramine. Aspirine, AINS, antiagrégants plaquettaires, corticoïdes et ISRS majorent le risque hémorragique. Contrôler l'INR trois à cinq jours après toute introduction ou tout arrêt de traitement.",
        adverse: "Hémorragies de toutes localisations, principal effet indésirable et principal motif d'hospitalisation. Effets immuno-allergiques propres aux indanediones, survenant surtout dans les six premiers mois : néphropathie tubulo-interstitielle pouvant conduire à une insuffisance rénale définitive, hépatite, cytopénies, éruptions cutanées graves de type DRESS, vascularite. Plus rarement alopécie, calciphylaxie.",
        monitoring: "INR au deuxième ou troisième jour, puis tous les deux à trois jours jusqu'à deux valeurs consécutives dans la cible, puis espacement jusqu'à un contrôle mensuel au minimum. Surveillance de la créatininémie, des transaminases et de l'hémogramme, en particulier pendant les six premiers mois, à la recherche d'un accident immuno-allergique. Toute fièvre, éruption cutanée, œdème ou baisse de la diurèse impose un arrêt et un avis médical immédiat.",
        iup: "Une prise par jour à heure fixe, le soir de préférence, le comprimé se coupant en quarts pour ajuster la dose ; le pilulier doit reproduire exactement le schéma du carnet, qui peut varier d'un jour à l'autre. En cas d'oubli constaté dans les huit heures, prendre la dose ; au-delà, la sauter et ne jamais doubler le lendemain. Le carnet AVK accompagne chaque prise de sang et se montre à tout professionnel de santé, y compris avant un détartrage ou une extraction. Pas d'automédication par aspirine, ibuprofène ou gel buccal antifongique ; le paracétamol reste l'antalgique de référence. Une alimentation régulière en légumes verts est préférable à leur suppression. Consulter en urgence devant un saignement qui persiste, des selles noires, des urines rouges, un hématome spontané, mais aussi devant une fièvre, une éruption ou des urines rares, qui peuvent annoncer une réaction immuno-allergique.",
        half_life: "environ 30 heures",
        elimination: "Métabolisme hépatique et élimination essentiellement urinaire sous forme de métabolites, avec une part de forme inchangée.",
        renal: "Contre-indiqué en cas d'insuffisance rénale sévère ; en cas d'atteinte rénale modérée, surveillance rapprochée de l'INR et de la créatininémie, la fluindione pouvant elle-même être en cause dans une dégradation rénale.",
        pregnancy: "Contre-indiqué pendant la grossesse en raison du risque d'embryofœtopathie et d'hémorragie néonatale, et contre-indiqué pendant l'allaitement, contrairement à la warfarine.",
        sources: "RCP Previscan — base de données publique des médicaments (ANSM)\nANSM — point d'information sur le risque immuno-allergique de la fluindione et restriction de primo-prescription\nHAS — bon usage des antivitamines K",
        status: "Poursuite seulement — plus d'initiation chez un nouveau patient",
        smr: "",
        tags: "avk, marge thérapeutique étroite, surveillance biologique, contre-indiqué grossesse",
        toxicity: "Marge thérapeutique étroite : un écart de dose ou une interaction suffit à faire basculer vers le sous-dosage ou la toxicité. Voir les sections Interactions et Surveillance.",
    },
    StarterDetail {
        name: "Sintrom",
        indications: "Prévention des complications thromboemboliques des cardiopathies emboligènes, notamment la fibrillation atriale, et des prothèses valvulaires mécaniques. Traitement des thromboses veineuses profondes et des embolies pulmonaires et prévention de leurs récidives. Prévention des complications thromboemboliques des infarctus du myocarde compliqués.",
        mechanism: "Antivitamine K coumarinique. L'acénocoumarol inhibe la vitamine K époxyde réductase, empêche la gamma-carboxylation des facteurs II, VII, IX et X et des protéines C et S, et aboutit à la production de facteurs de coagulation inactifs. Sa demi-vie courte rend l'anticoagulation plus rapidement modifiable mais aussi plus sensible aux oublis qu'avec la warfarine.",
        dosage: "Comprimés quadrisécables à 4 mg, et Minisintrom à 1 mg pour les ajustements fins, en une prise quotidienne à heure fixe, de préférence le soir. Dose initiale habituelle de 4 mg par jour chez l'adulte, réduite chez le sujet âgé, de faible poids ou insuffisant hépatique, sans dose de charge. Adaptation ensuite uniquement sur l'INR, par paliers de 1 mg ou d'un quart de comprimé, en espaçant les modifications de deux à trois jours. Cible d'INR entre 2 et 3 dans la majorité des indications, plus élevée pour certaines prothèses valvulaires mécaniques.",
        contraindications: "Hypersensibilité à l'acénocoumarol, insuffisance hépatique sévère, saignement évolutif ou lésion organique à risque hémorragique, hypertension artérielle maligne, accident vasculaire cérébral hémorragique récent, chirurgie neurologique ou ophtalmologique récente, association au miconazole y compris en gel buccal, au millepertuis, à l'aspirine anti-inflammatoire et aux AINS pyrazolés, grossesse hors situations exceptionnelles, impossibilité de surveiller l'INR.",
        ddi: "Miconazole, sous toutes ses formes, et millepertuis contre-indiqués. Augmentent l'effet : amiodarone, cotrimoxazole, métronidazole, azolés, macrolides, fluoroquinolones, allopurinol, tamoxifène, fibrates. Le diminuent : rifampicine, carbamazépine, phénytoïne, phénobarbital, cholestyramine. Aspirine, AINS, antiagrégants, corticoïdes et ISRS majorent le risque hémorragique. Un INR de contrôle s'impose trois à cinq jours après toute introduction ou tout arrêt de médicament, y compris un antibiotique de courte durée.",
        adverse: "Hémorragies de toutes localisations, des gingivorragies et épistaxis aux hémorragies digestives et intracrâniennes. Plus rarement nécrose cutanée en début de traitement chez les déficitaires en protéine C ou S, alopécie, réactions allergiques cutanées, hépatite cholestatique, embolies de cristaux de cholestérol.",
        monitoring: "INR au deuxième ou troisième jour, puis tous les deux à trois jours jusqu'à deux valeurs successives dans la cible, puis espacement progressif sans dépasser un mois entre deux contrôles. Contrôle systématique après toute modification thérapeutique ou tout épisode intercurrent, fièvre, diarrhée, vomissements ou dénutrition. Hémogramme en cas de saignement.",
        iup: "Une prise unique par jour, toujours à la même heure et de préférence le soir, avec un comprimé qui se coupe en quarts pour suivre exactement le schéma du carnet. La demi-vie courte de ce médicament rend l'oubli plus lourd de conséquences : si l'oubli est constaté dans les huit heures, prendre la dose, sinon la sauter, sans jamais doubler le lendemain, et le signaler au médecin. Le carnet AVK accompagne chaque prise de sang et se présente à tout médecin, dentiste ou infirmier avant un soin. Aucune automédication, en particulier aspirine, ibuprofène et gel buccal antifongique ; le paracétamol est l'antalgique à privilégier. Garder une consommation régulière de légumes verts plutôt que de les supprimer. Consulter sans attendre devant un saignement qui ne s'arrête pas, des selles noires, des urines rouges, des hématomes spontanés, un essoufflement inhabituel ou un choc à la tête.",
        half_life: "8 à 11 heures",
        elimination: "Métabolisme hépatique important, notamment par le CYP2C9, avec élimination des métabolites par voie urinaire et fécale.",
        renal: "Pas d'adaptation posologique fondée sur la clairance, la dose étant guidée par l'INR ; surveillance rapprochée en cas d'insuffisance rénale, qui augmente le risque hémorragique.",
        pregnancy: "Contre-indiqué pendant la grossesse en raison du risque d'embryofœtopathie et d'hémorragie néonatale, sauf situation exceptionnelle telle qu'une prothèse valvulaire mécanique ; l'allaitement est possible sous acénocoumarol.",
        sources: "RCP Sintrom — base de données publique des médicaments (ANSM)\nHAS — bon usage des antivitamines K et carnet de suivi AVK\nESC 2020 — prise en charge de la fibrillation atriale",
        status: "",
        smr: "",
        tags: "avk, marge thérapeutique étroite, surveillance biologique, contre-indiqué grossesse",
        toxicity: "Marge thérapeutique étroite : un écart de dose ou une interaction suffit à faire basculer vers le sous-dosage ou la toxicité. Voir les sections Interactions et Surveillance.",
    },
    StarterDetail {
        name: "Lovenox",
        indications: "Prophylaxie de la maladie thromboembolique veineuse en chirurgie, en particulier orthopédique et carcinologique, et chez le patient médical alité présentant un facteur de risque. Traitement curatif des thromboses veineuses profondes constituées et des embolies pulmonaires sans signe de gravité. Traitement des syndromes coronariens aigus. Relais des anticoagulants oraux lors des situations où ceux-ci doivent être interrompus, et prévention de la coagulation du circuit d'épuration extrarénale.",
        mechanism: "Héparine de bas poids moléculaire obtenue par dépolymérisation de l'héparine standard. L'énoxaparine se lie à l'antithrombine et potentialise très fortement son action inhibitrice sur le facteur Xa, avec une activité anti-IIa nettement moindre que celle de l'héparine non fractionnée. Il en résulte une anticoagulation prévisible sur une base pondérale, sans nécessité d'ajustement biologique dans la plupart des situations.",
        dosage: "Voie sous-cutanée. Prophylaxie en situation à risque modéré : 2 000 UI anti-Xa, soit 20 mg, une fois par jour ; en situation à risque élevé, notamment en chirurgie orthopédique majeure et chez le patient médical alité : 4 000 UI, soit 40 mg, une fois par jour. Traitement curatif de la maladie thromboembolique veineuse : 100 UI/kg, soit 1 mg/kg, toutes les douze heures, ou 150 UI/kg, soit 1,5 mg/kg, en une injection quotidienne selon le schéma retenu, la dose étant calculée sur le poids réel. Syndrome coronarien aigu : 100 UI/kg toutes les douze heures, avec un schéma spécifique et une dose réduite après 75 ans dans l'infarctus avec sus-décalage. Les seringues préremplies sont graduées, ce qui permet d'ajuster au poids.",
        contraindications: "Hypersensibilité à l'énoxaparine ou à l'héparine, antécédent de thrombopénie induite par l'héparine de type II, saignement évolutif ou trouble de l'hémostase constitutionnel, lésion organique à risque de saignement dont l'ulcère gastroduodénal évolutif, accident vasculaire cérébral hémorragique, endocardite infectieuse aiguë, anesthésie péridurale ou rachianesthésie en cours de traitement curatif, et insuffisance rénale sévère avec clairance inférieure à 30 mL/min pour les doses curatives.",
        ddi: "Aspirine à dose antalgique ou anti-inflammatoire, AINS, antiagrégants plaquettaires, anticoagulants oraux et thrombolytiques majorent le risque hémorragique et ne doivent pas être associés sans indication précise. Les corticoïdes à forte dose et le dextran augmentent également ce risque. Les médicaments hyperkaliémiants, inhibiteurs de l'enzyme de conversion, sartans, diurétiques épargneurs de potassium et sels de potassium, s'ajoutent à l'hypoaldostéronisme induit par l'héparine.",
        adverse: "Douleur, ecchymose et petit hématome au point d'injection, très fréquents et bénins. Saignements de toutes localisations, favorisés par un surdosage, l'âge et l'insuffisance rénale. Élévation asymptomatique des transaminases. Plus rarement thrombopénie induite par l'héparine de type II, redoutable car thrombosante, survenant surtout entre le cinquième et le vingt et unième jour, hyperkaliémie par hypoaldostéronisme, ostéoporose lors des traitements prolongés, nécrose cutanée et réactions allergiques.",
        monitoring: "Numération plaquettaire avant ou au début du traitement, puis deux fois par semaine pendant le premier mois dans les situations à risque de thrombopénie induite par l'héparine, notamment en contexte chirurgical ou traumatique et en cas d'exposition antérieure à l'héparine. Créatininémie et calcul de la clairance avant l'instauration, surtout après 75 ans et en cas de faible poids. Kaliémie en cas de traitement prolongé ou d'association hyperkaliémiante. L'activité anti-Xa n'est pas mesurée en routine et se réserve aux situations particulières comme l'insuffisance rénale, les poids extrêmes ou la grossesse.",
        iup: "L'injection se fait dans le tissu sous-cutané de l'abdomen, sur la ceinture antérolatérale, à distance du nombril, en alternant systématiquement le côté droit et le côté gauche d'une fois sur l'autre. La bulle d'air de la seringue préremplie ne doit jamais être purgée : elle chasse la dose résiduelle et évite la fuite du produit. Pincer un pli cutané franc entre le pouce et l'index, piquer perpendiculairement à la base du pli, injecter lentement, relâcher le pli après avoir retiré l'aiguille et ne surtout pas masser le point d'injection. Respecter l'heure fixe, en particulier le rythme des douze heures pour les schémas en deux injections ; en cas d'oubli, faire l'injection dès que possible et ne jamais doubler la dose suivante. Les bleus au point de piqûre sont normaux, mais il faut signaler un saignement de nez ou de gencives prolongé, des selles noires, des urines rouges, un gros hématome douloureux, et consulter en urgence devant un mollet gonflé et douloureux ou un essoufflement brutal, qui peuvent traduire une thrombose sous traitement. La seringue usagée se jette dans un collecteur DASTRI à rapporter à la pharmacie.",
        half_life: "environ 4 heures après une dose unique, jusqu'à 7 heures en administration répétée, mesurée sur l'activité anti-Xa",
        elimination: "Dépolymérisation et désulfatation hépatiques partielles, avec une élimination principalement rénale ; l'exposition augmente donc nettement lorsque la clairance diminue.",
        renal: "Clairance de 30 à 50 mL/min : surveillance clinique renforcée, réduction de dose à envisager selon l'indication. Clairance de 15 à 30 mL/min : doses curatives contre-indiquées, prophylaxie possible à posologie réduite. Clairance inférieure à 15 mL/min : non recommandé en dehors de la dialyse.",
        pregnancy: "Utilisable pendant la grossesse quel qu'en soit le terme, l'énoxaparine ne franchissant pas la barrière placentaire, et compatible avec l'allaitement ; c'est l'anticoagulant de référence chez la femme enceinte.",
        sources: "RCP Lovenox — base de données publique des médicaments (ANSM)\nANSM — bon usage des héparines de bas poids moléculaire et surveillance plaquettaire\nHAS — prévention et traitement de la maladie thromboembolique veineuse",
        status: "",
        smr: "",
        tags: "hbpm, surveillance biologique",
        toxicity: "",
    },
    StarterDetail {
        name: "Kardégic",
        indications: "Prévention secondaire des accidents ischémiques après infarctus du myocarde, angor stable ou instable, syndrome coronarien aigu, accident vasculaire cérébral ischémique ou accident ischémique transitoire, et dans l'artériopathie oblitérante des membres inférieurs. Également indiqué après angioplastie coronaire avec ou sans stent et après pontage aorto-coronarien, le plus souvent au long cours et sans interruption.",
        mechanism: "L'acétylsalicylate de lysine est une forme soluble de l'aspirine qui acétyle de façon irréversible la cyclo-oxygénase 1 plaquettaire. La plaquette étant anucléée et incapable de resynthétiser l'enzyme, la production de thromboxane A2 est abolie pour toute sa durée de vie, d'où une inhibition durable de l'agrégation malgré une demi-vie plasmatique très courte. Aux faibles doses antiagrégantes, l'effet plaquettaire domine et la synthèse endothéliale de prostacycline est relativement épargnée.",
        dosage: "Habituellement 75 mg par jour, la posologie antiagrégante usuelle en France se situant entre 75 et 160 mg par jour en une prise unique ; le dosage à 300 mg est réservé à certaines situations et à la phase aiguë. Le sachet se dissout dans un verre d'eau et se prend de préférence à la fin d'un repas. Il n'y a pas d'adaptation systématique au poids ni à l'âge, mais le risque hémorragique du sujet âgé conduit à préférer les doses les plus basses ; augmenter la dose n'augmente pas l'effet antiagrégant et augmente le risque digestif.",
        contraindications: "Ulcère gastroduodénal évolutif, maladie hémorragique constitutionnelle ou acquise, antécédent d'asthme ou de réaction d'hypersensibilité déclenchée par l'aspirine ou les AINS, insuffisance hépatique sévère, insuffisance rénale sévère, insuffisance cardiaque sévère non contrôlée, grossesse à partir du début du sixième mois. L'association au méthotrexate est déconseillée aux doses antiagrégantes et contre-indiquée aux doses supérieures de méthotrexate.",
        ddi: "Les AINS majorent le risque ulcéreux et hémorragique, et l'ibuprofène peut en outre entrer en compétition sur la cyclo-oxygénase plaquettaire et diminuer l'effet cardioprotecteur de l'aspirine : décaler les prises ou choisir un autre antalgique. L'association aux anticoagulants, aux autres antiagrégants, aux corticoïdes, aux ISRS et aux IRSNA augmente le risque hémorragique, en particulier digestif. L'aspirine réduit l'élimination rénale du méthotrexate et majore sa toxicité. Elle diminue l'effet des uricosuriques et peut altérer la fonction rénale en association aux diurétiques et aux bloqueurs du système rénine-angiotensine.",
        adverse: "Fréquemment gastralgies, nausées, saignements occultes digestifs pouvant entraîner une anémie ferriprive, épistaxis, gingivorragies et ecchymoses. Plus rarement mais gravement : hémorragie digestive haute, ulcère perforé, hémorragie intracrânienne, bronchospasme chez le sujet intolérant à l'aspirine, urticaire et angio-œdème. Des acouphènes et une baisse d'audition traduisent un surdosage, situation rare aux doses antiagrégantes.",
        monitoring: "Aucune surveillance biologique systématique n'est requise aux doses antiagrégantes. Hémogramme et ferritine en cas d'asthénie, de pâleur ou de suspicion de saignement digestif chronique, surtout chez le sujet âgé. Créatininémie en cas d'association aux diurétiques et aux bloqueurs du système rénine-angiotensine, notamment lors d'un épisode de déshydratation. Rechercher à l'interrogatoire les signes de saignement digestif et discuter une protection gastrique chez les patients à risque.",
        iup: "Videz le sachet dans un demi-verre d'eau, remuez et buvez immédiatement, de préférence à la fin d'un repas pour ménager l'estomac. Ce traitement se prend tous les jours, sans interruption : il ne soulage rien, il protège du risque d'infarctus et d'accident vasculaire cérébral, et l'arrêter, même quelques jours, augmente ce risque, surtout après la pose d'un stent. N'arrêtez jamais de vous-même avant une extraction dentaire ou une intervention : signalez le traitement et attendez la consigne du médecin ou du chirurgien, qui le maintient le plus souvent. Ne prenez pas d'anti-inflammatoire en automédication, ibuprofène compris : il augmente le risque d'ulcère et peut annuler l'effet protecteur de l'aspirine ; l'antalgique à privilégier est le paracétamol. Consultez rapidement en cas de selles noires ou goudronneuses, de vomissements sanglants, de sang dans les urines, de saignement de nez qui ne s'arrête pas, ou de fatigue et de pâleur inhabituelles. Prévenez tout médecin ou pharmacien de ce traitement avant l'achat de tout médicament, plusieurs spécialités contenant déjà de l'aspirine.",
        half_life: "Demi-vie plasmatique de l'acide acétylsalicylique de l'ordre de 15 à 20 minutes, mais l'inhibition plaquettaire irréversible persiste 7 à 10 jours",
        elimination: "Hydrolyse rapide en acide salicylique, métabolisation hépatique par glycoconjugaison saturable, puis élimination urinaire, accélérée en milieu alcalin.",
        renal: "Contre-indiqué en cas d'insuffisance rénale sévère. Prudence et surveillance de la fonction rénale en cas d'insuffisance rénale modérée, en particulier en association aux diurétiques, aux inhibiteurs de l'enzyme de conversion et aux sartans.",
        pregnancy: "Contre-indiqué à partir du début du sixième mois de grossesse en raison du risque de fermeture prématurée du canal artériel et de toxicité rénale fœtale ; utilisable avant ce terme uniquement sur indication médicale précise, et allaitement à éviter au long cours, un passage dans le lait étant décrit.",
        sources: "RCP Kardégic — base de données publique des médicaments (ANSM)\nESC — syndromes coronariens aigus et traitement antiagrégant plaquettaire\nHAS — bon usage des antiagrégants plaquettaires",
        status: "",
        smr: "",
        tags: "antiagrégant, contre-indiqué grossesse",
        toxicity: "",
    },
    StarterDetail {
        name: "Plavix",
        indications: "Prévention des événements athérothrombotiques après infarctus du myocarde, accident vasculaire cérébral ischémique ou en cas d'artériopathie oblitérante des membres inférieurs établie. En association à l'aspirine, traitement du syndrome coronarien aigu, avec ou sans sus-décalage du segment ST, et prévention de la thrombose de stent après angioplastie coronaire. Également utilisé, associé à l'aspirine, dans la fibrillation atriale lorsque les anticoagulants oraux sont contre-indiqués.",
        mechanism: "Prodrogue transformée par les cytochromes hépatiques, principalement le CYP2C19, en un métabolite actif qui se lie de façon irréversible au récepteur plaquettaire P2Y12 de l'ADP. Le blocage de ce récepteur empêche l'activation du complexe glycoprotéique IIb/IIIa et donc l'agrégation plaquettaire. L'inhibition étant irréversible, elle persiste toute la durée de vie de la plaquette et l'effet ne disparaît qu'avec le renouvellement du pool plaquettaire, en sept à dix jours.",
        dosage: "Entretien : 75 mg par jour en une prise, avec ou sans aliments. Dans le syndrome coronarien aigu, le traitement est débuté à l'hôpital par une dose de charge, suivie de 75 mg par jour, en association à l'aspirine à faible dose. La durée de la bithérapie antiagrégante après un stent est fixée par le cardiologue selon le type de stent et le risque hémorragique, et ne doit jamais être écourtée sans son avis. Aucune adaptation liée à l'âge ou au poids n'est prévue, mais le risque hémorragique du sujet âgé impose une vigilance accrue.",
        contraindications: "Hémorragie évolutive, notamment ulcéreuse ou intracrânienne, insuffisance hépatique sévère, hypersensibilité, allaitement.",
        ddi: "L'oméprazole et l'ésoméprazole inhibent le CYP2C19 et réduisent la formation du métabolite actif, donc l'efficacité antiagrégante : l'association est déconseillée, et le pantoprazole ou le rabéprazole doivent être préférés lorsqu'un inhibiteur de la pompe à protons est nécessaire. Les AINS, les anticoagulants, les autres antiagrégants, les ISRS et les IRSNA majorent le risque hémorragique. Le clopidogrel augmente l'exposition au répaglinide par inhibition du CYP2C8. La prise concomitante de corticoïdes accroît le risque digestif.",
        adverse: "Fréquemment ecchymoses, épistaxis, hématomes au moindre choc, saignements gingivaux, allongement des saignements après coupure, diarrhée, douleurs abdominales, dyspepsie et éruptions cutanées prurigineuses. Plus rarement hémorragie digestive ou intracrânienne, et exceptionnellement purpura thrombotique thrombocytopénique, neutropénie sévère ou agranulocytose, réactions cutanées graves.",
        monitoring: "Aucune surveillance biologique de routine de l'agrégation n'est nécessaire en pratique courante. Hémogramme en cas de saignement, de fièvre, d'angine ou d'ecchymoses inhabituelles, pour rechercher une neutropénie ou une thrombopénie. Surveiller la tolérance digestive et l'hémoglobine en cas d'asthénie inexpliquée chez le sujet âgé. Réévaluer à chaque renouvellement la pertinence et la durée de l'association à l'aspirine.",
        iup: "Prenez un comprimé par jour, à heure fixe, avec ou sans aliments, et n'interrompez jamais le traitement de votre propre initiative : après la pose d'un stent, un arrêt prématuré expose à une thrombose du stent, c'est-à-dire à un infarctus. En cas d'oubli, prenez le comprimé dans la journée si vous y pensez, sinon reprenez au jour suivant sans doubler la dose. Vous saignerez plus facilement et plus longtemps : appuyez plus longtemps sur une coupure, utilisez une brosse à dents souple et un rasoir électrique, et évitez les sports de contact. Signalez systématiquement ce traitement avant tout soin dentaire, toute infiltration, toute chirurgie ou coloscopie, et attendez la consigne du cardiologue avant tout arrêt, même de quelques jours. Ne prenez aucun anti-inflammatoire en automédication, ibuprofène et aspirine compris, et parlez-en avant tout achat de médicament contre la douleur ; le paracétamol reste l'antalgique de choix. Consultez sans attendre en cas de selles noires, de sang dans les urines ou les vomissements, de saignement de nez qui ne s'arrête pas, ou de maux de tête violents après un choc.",
        half_life: "Environ 6 heures pour le clopidogrel et une demi-heure pour son métabolite actif, mais l'effet antiplaquettaire irréversible persiste 7 à 10 jours",
        elimination: "Activation hépatique par le CYP2C19, avec inactivation majoritaire par les estérases plasmatiques ; élimination des métabolites pour moitié par voie urinaire et pour moitié par voie fécale.",
        renal: "Pas d'adaptation posologique ; l'expérience est limitée en cas d'insuffisance rénale sévère, où le risque hémorragique justifie une prudence particulière.",
        pregnancy: "Par prudence, à éviter pendant la grossesse en l'absence de données suffisantes, sauf nécessité ; allaitement contre-indiqué.",
        sources: "RCP Plavix — base de données publique des médicaments (ANSM)\nESC — syndromes coronariens aigus et traitement antiagrégant plaquettaire\nHAS — bon usage des antiagrégants plaquettaires",
        status: "",
        smr: "",
        tags: "antiagrégant, contre-indiqué grossesse",
        toxicity: "",
    },
    StarterDetail {
        name: "Xanax",
        indications: "Traitement symptomatique des manifestations anxieuses sévères ou invalidantes ; prévention et traitement du delirium tremens et des autres manifestations du sevrage alcoolique. La prescription doit rester limitée dans le temps et réévaluée régulièrement.",
        mechanism: "Alprazolam, benzodiazépine à demi-vie intermédiaire, modulateur allostérique positif du récepteur GABA-A : sa fixation sur le site benzodiazépinique augmente la fréquence d'ouverture du canal chlore en présence de GABA, ce qui hyperpolarise le neurone et renforce l'inhibition centrale. Il en résulte des effets anxiolytique, sédatif, myorelaxant, anticonvulsivant et amnésiant. L'utilisation prolongée entraîne une tolérance et une dépendance physique par adaptation du récepteur.",
        dosage: "0,25 à 0,5 mg trois fois par jour à l'instauration, avec adaptation progressive selon la réponse ; la posologie usuelle se situe entre 0,5 et 2 mg par jour et la dose maximale est de 4 mg par jour en plusieurs prises. Chez le sujet âgé, l'insuffisant hépatique ou respiratoire, débuter à 0,25 mg une à deux fois par jour et ne pas dépasser la moitié de la dose adulte. La durée totale de prescription, période de diminution comprise, ne doit pas excéder douze semaines, dont quatre semaines maximum d'anxiolyse continue avant réévaluation. L'arrêt se fait toujours par diminution progressive, de l'ordre de 10 à 25 % de la dose toutes une à deux semaines, plus lentement après un usage prolongé.",
        contraindications: "Hypersensibilité aux benzodiazépines, insuffisance respiratoire sévère, syndrome d'apnées du sommeil non appareillé, insuffisance hépatique sévère avec risque d'encéphalopathie, myasthénie, enfant de moins de six ans.",
        ddi: "Potentialisation de la dépression respiratoire et de la sédation par les opioïdes : l'association à la méthadone, à la buprénorphine, à la morphine ou au tramadol est un facteur majeur de décès par surdose, elle exige une justification, la dose minimale et une information explicite du patient. Sédation additive avec l'alcool, à proscrire, avec les autres hypnotiques, les antihistaminiques sédatifs, les neuroleptiques et les antidépresseurs sédatifs. Les inhibiteurs puissants du CYP3A4 (kétoconazole, itraconazole, ritonavir, clarithromycine, jus de pamplemousse) augmentent nettement les concentrations et la sédation. Les inducteurs (rifampicine, carbamazépine, millepertuis) diminuent l'efficacité. La clozapine associée expose à un risque de collapsus et d'arrêt respiratoire.",
        adverse: "Fréquents et dose-dépendants : somnolence diurne, sensations vertigineuses, asthénie, difficultés de concentration, hypotonie musculaire, ataxie et chutes chez le sujet âgé. Amnésie antérograde, parfois dès les doses usuelles. Dépendance physique et psychique avec syndrome de sevrage à l'arrêt (anxiété rebond, insomnie, tremblements, sueurs, plus rarement convulsions et état confusionnel). Réactions paradoxales avec agitation, irritabilité, désinhibition et agressivité, surtout chez le sujet âgé et l'enfant. Dépression respiratoire chez l'insuffisant respiratoire ou en association aux opioïdes.",
        monitoring: "Réévaluation de l'indication à chaque renouvellement, avec vérification de la durée cumulée de traitement et du respect du plafond de douze semaines. Surveillance du risque de chute, de la vigilance diurne et de la fonction cognitive chez le sujet âgé, chez qui les benzodiazépines à demi-vie intermédiaire figurent parmi les médicaments à éviter. Repérage d'un mésusage : demandes de renouvellement anticipé, pluralité de prescripteurs, escalade de doses. Recherche d'une consommation associée d'alcool ou d'opioïdes.",
        iup: "Prenez les comprimés aux heures indiquées, sans en ajouter de votre propre initiative même si l'anxiété remonte : l'effet est immédiat, en quelques dizaines de minutes, mais la dose ne doit pas augmenter d'elle-même. Ce traitement est prévu pour quelques semaines seulement, douze semaines au maximum en comptant la période de diminution, parce que le corps s'y habitue et qu'il devient ensuite difficile de s'en passer. N'arrêtez jamais brutalement après plusieurs semaines : l'arrêt doit être progressif, par paliers décidés avec le médecin, sinon apparaissent anxiété rebond, insomnie, tremblements et, rarement, des convulsions. Pas d'alcool du tout, et prudence extrême à la conduite : la vigilance et les réflexes sont diminués, et vous pouvez ne pas vous en rendre compte. Si vous prenez aussi un médicament de la douleur de la famille de la morphine ou un traitement de substitution, signalez-le, car l'association peut provoquer un arrêt respiratoire. Revenez ou appelez le jour même en cas d'agitation ou d'agressivité inhabituelles, de trous de mémoire, de chutes répétées, ou de somnolence importante avec ralentissement de la respiration.",
        half_life: "≈ 12 à 15 heures",
        elimination: "Métabolisme hépatique par le CYP3A4 en alpha-hydroxyalprazolam faiblement actif, puis glucuroconjugaison et élimination urinaire.",
        renal: "Pas d'adaptation particulière en insuffisance rénale légère à modérée ; en insuffisance rénale sévère, prudence et posologie minimale efficace du fait de l'accumulation des métabolites conjugués.",
        pregnancy: "Éviter pendant la grossesse, en particulier en fin de grossesse où une exposition expose le nouveau-né à une hypotonie, des difficultés de succion et un syndrome de sevrage ; l'allaitement est déconseillé en cas de prise régulière du fait de la somnolence du nourrisson.",
        sources: "RCP Xanax — base de données publique des médicaments (ANSM)\nHAS — quelle place pour les benzodiazépines dans l'anxiété et arrêt des benzodiazépines chez le sujet âgé\nANSM — état des lieux de la consommation des benzodiazépines",
        status: "",
        smr: "",
        tags: "benzodiazépine, vigilance conduite",
        toxicity: "",
    },
    StarterDetail {
        name: "Stilnox",
        indications: "Traitement de courte durée de l'insomnie occasionnelle ou transitoire de l'adulte, lorsqu'elle est sévère, invalidante ou responsable d'une fatigue diurne importante, et après échec des mesures d'hygiène du sommeil.",
        mechanism: "Zolpidem, hypnotique de la famille des imidazopyridines, apparenté aux benzodiazépines par son site d'action : il se fixe sur le récepteur GABA-A avec une sélectivité relative pour les sous-unités alpha-1, qui portent surtout l'effet sédatif et hypnotique. Il en résulte un effet hypnotique marqué avec des effets myorelaxant et anxiolytique moindres que ceux des benzodiazépines. Son délai d'action très court et sa demi-vie brève l'orientent vers les insomnies d'endormissement.",
        dosage: "10 mg en une prise unique au moment du coucher, immédiatement avant de se mettre au lit, jamais après une prise alimentaire tardive qui retarderait l'effet, et à condition de pouvoir consacrer au moins sept à huit heures au sommeil. Chez le sujet de plus de 65 ans, l'insuffisant hépatique ou le patient fragile, la posologie est de 5 mg par jour et ne doit pas être dépassée. La durée de traitement, période de diminution comprise, est limitée à quatre semaines : quelques jours pour une insomnie occasionnelle, deux à trois semaines pour une insomnie transitoire. La prescription doit être rédigée sur ordonnance sécurisée, en toutes lettres, pour une durée maximale de vingt-huit jours, sans chevauchement possible avec une ordonnance précédente sauf mention expresse du prescripteur.",
        contraindications: "Hypersensibilité au zolpidem, insuffisance respiratoire sévère, syndrome d'apnées du sommeil, insuffisance hépatique sévère, myasthénie, enfant et adolescent de moins de dix-huit ans.",
        ddi: "Potentialisation majeure de la sédation et de la dépression respiratoire par les opioïdes, y compris la méthadone et la buprénorphine : association à limiter, dose minimale et information du patient sur le risque vital. Sédation additive avec l'alcool, à proscrire formellement, avec les benzodiazépines, les antihistaminiques sédatifs, les neuroleptiques, les antidépresseurs sédatifs et les antitussifs opiacés. Les inhibiteurs puissants du CYP3A4 (kétoconazole, itraconazole, ritonavir, clarithromycine) augmentent l'exposition ; la rifampicine et le millepertuis la diminuent. Les ISRS, en particulier la sertraline et la fluvoxamine, peuvent majorer la sédation.",
        adverse: "Fréquents : somnolence résiduelle au réveil, céphalées, sensations vertigineuses, ataxie et chutes surtout chez le sujet âgé, troubles digestifs, goût amer. Amnésie antérograde fréquente, parfois avec comportements automatiques : conduite automobile, préparation ou prise d'aliments, appels téléphoniques sans souvenir le lendemain, y compris chez des patients n'ayant pas consommé d'alcool. Hallucinations hypnagogiques, cauchemars, somnambulisme. Réactions paradoxales avec agitation, irritabilité, désinhibition. Dépendance et syndrome de sevrage avec insomnie rebond à l'arrêt. Usage détourné et mésusage documentés, à l'origine de l'encadrement réglementaire.",
        monitoring: "Réévaluation de la nécessité du traitement à chaque renouvellement, avec vérification de la durée cumulée et du respect du plafond de quatre semaines. Contrôle de la conformité de l'ordonnance sécurisée à la délivrance : mention en toutes lettres, durée n'excédant pas vingt-huit jours, absence de chevauchement. Recherche d'un mésusage (renouvellement anticipé, pluralité de prescripteurs, doses supra-thérapeutiques). Chez le sujet âgé, surveillance du risque de chute nocturne et de la confusion matinale.",
        iup: "Prenez le comprimé au tout dernier moment, une fois couché, et pas avant : l'effet survient en quinze à trente minutes et vous ne devez plus vous relever ensuite. Ne le prenez que si vous pouvez rester au lit sept à huit heures d'affilée, sinon la somnolence se prolongera le lendemain matin. Ce médicament est prévu pour quelques jours à quelques semaines, quatre semaines au maximum : au-delà, il perd son efficacité et l'organisme s'y habitue, ce qui rend l'arrêt difficile. N'arrêtez pas d'un coup après plusieurs semaines de prise, la diminution doit être progressive avec votre médecin, et attendez-vous à quelques nuits moins bonnes au moment de l'arrêt. Pas une goutte d'alcool le soir de la prise, et pas de conduite ni de machine dangereuse ensuite ni le lendemain matin si vous vous sentez encore ralenti. Revenez le jour même si votre entourage vous rapporte des comportements nocturnes dont vous n'avez aucun souvenir, en cas d'hallucinations, de chute, ou d'agitation inhabituelle.",
        half_life: "≈ 2 à 3 heures",
        elimination: "Métabolisme hépatique par les CYP3A4, CYP1A2 et CYP2C9 en métabolites inactifs, éliminés par voies urinaire et fécale ; aucun métabolite actif.",
        renal: "Pas d'adaptation formelle, la molécule n'étant pas éliminée sous forme active par le rein ; prudence et posologie réduite chez l'insuffisant rénal fragile ou âgé.",
        pregnancy: "Éviter pendant la grossesse, en particulier au troisième trimestre où l'exposition expose le nouveau-né à une hypotonie et à un syndrome de sevrage ; l'allaitement est déconseillé en cas de prise répétée, le passage lacté étant faible mais la somnolence du nourrisson possible.",
        sources: "RCP Stilnox — base de données publique des médicaments (ANSM)\nANSM — conditions de prescription et de délivrance du zolpidem sur ordonnance sécurisée\nHAS — prise en charge de l'insomnie chez l'adulte en premier recours",
        status: "Ordonnance sécurisée, 28 jours, sans chevauchement",
        smr: "",
        tags: "hypnotique, vigilance conduite",
        toxicity: "",
    },
    StarterDetail {
        name: "Imovane",
        indications: "Traitement de courte durée de l'insomnie occasionnelle ou transitoire de l'adulte, lorsqu'elle est sévère ou invalidante, après échec des mesures d'hygiène du sommeil.",
        mechanism: "Zopiclone, hypnotique de la famille des cyclopyrrolones, agoniste du récepteur GABA-A sur un site proche mais distinct du site benzodiazépinique classique. Elle renforce la transmission inhibitrice GABAergique et raccourcit le délai d'endormissement tout en augmentant la durée totale du sommeil. Sa demi-vie un peu plus longue que celle du zolpidem lui donne un effet légèrement plus prolongé sur le maintien du sommeil, au prix d'une somnolence résiduelle plus fréquente.",
        dosage: "7,5 mg en une prise unique juste avant le coucher, sans prise fractionnée ni seconde prise dans la nuit, et à condition de pouvoir dormir au moins sept à huit heures. Chez le sujet de plus de 65 ans, l'insuffisant hépatique, l'insuffisant rénal ou l'insuffisant respiratoire chronique, la posologie est de 3,75 mg, soit un demi-comprimé, et peut être maintenue à cette dose. La durée totale de traitement, décroissance comprise, ne doit pas dépasser quatre semaines : quelques jours pour une insomnie occasionnelle, deux à trois semaines pour une insomnie transitoire. L'arrêt après un usage prolongé se fait par diminution progressive de la dose ou espacement des prises.",
        contraindications: "Hypersensibilité à la zopiclone, insuffisance respiratoire sévère, syndrome d'apnées du sommeil, insuffisance hépatique sévère, myasthénie, enfant et adolescent de moins de dix-huit ans.",
        ddi: "Potentialisation de la sédation et de la dépression respiratoire par les opioïdes, y compris les traitements de substitution : association à limiter et à expliquer au patient. Sédation additive avec l'alcool, à proscrire, ainsi qu'avec les benzodiazépines, les antihistaminiques sédatifs, les neuroleptiques, les antidépresseurs sédatifs et les antitussifs opiacés. Les inhibiteurs puissants du CYP3A4 (kétoconazole, itraconazole, ritonavir, clarithromycine, érythromycine) augmentent l'exposition et la somnolence résiduelle ; la rifampicine et le millepertuis la diminuent.",
        adverse: "Très fréquent et caractéristique : goût amer ou métallique persistant, souvent au réveil, motif fréquent d'arrêt. Fréquents : somnolence résiduelle et sensation de bouche sèche le matin, céphalées, sensations vertigineuses, ataxie et chutes chez le sujet âgé, troubles digestifs. Amnésie antérograde et comportements automatiques nocturnes sans souvenir. Cauchemars, hallucinations, somnambulisme. Réactions paradoxales avec agitation, irritabilité et agressivité. Dépendance et syndrome de sevrage avec insomnie rebond. Rarement, angio-œdème et réactions anaphylactiques.",
        monitoring: "Réévaluation systématique du bien-fondé du traitement à chaque renouvellement, avec contrôle de la durée cumulée et du plafond de quatre semaines. Repérage d'un mésusage ou d'une consommation prolongée qui se banalise. Chez le sujet âgé, surveillance des chutes nocturnes, de la confusion matinale et de la vigilance diurne ; la zopiclone figure parmi les médicaments à éviter dans cette population. Recherche d'une consommation d'alcool ou d'opioïdes associée.",
        iup: "Prenez le comprimé au moment de vous coucher, pas avant : il agit en une vingtaine de minutes et vous ne devez plus vous relever ensuite. Ne le prenez que si vous disposez de sept à huit heures de sommeil devant vous, faute de quoi vous serez encore ralenti le matin. Un goût amer ou métallique dans la bouche au réveil est très fréquent avec ce médicament ; il est sans gravité, un verre d'eau et un brossage des dents l'atténuent. Le traitement est prévu pour quelques jours à quelques semaines, quatre au maximum : au-delà il perd son efficacité, et l'arrêt devra alors être progressif, jamais brutal, avec quelques nuits moins bonnes à prévoir. Pas d'alcool le soir de la prise, et pas de conduite ni de machine dangereuse après la prise ni le lendemain matin si vous vous sentez encore endormi. Revenez le jour même si votre entourage vous décrit des comportements nocturnes dont vous ne vous souvenez pas, en cas d'hallucinations, de chute, ou de gonflement du visage ou de la gorge.",
        half_life: "≈ 5 heures, allongée chez le sujet âgé et l'insuffisant hépatique",
        elimination: "Métabolisme hépatique par le CYP3A4 et le CYP2C8 en un N-oxyde faiblement actif et un dérivé N-déméthylé inactif ; élimination essentiellement urinaire sous forme de métabolites.",
        renal: "Pas d'adaptation obligatoire, mais la posologie réduite de 3,75 mg est recommandée en insuffisance rénale, la marge de sécurité étant faible chez ces patients souvent âgés.",
        pregnancy: "Éviter pendant la grossesse, en particulier en fin de grossesse où l'exposition expose le nouveau-né à une hypotonie, des difficultés de succion et un syndrome de sevrage ; l'allaitement est déconseillé, la zopiclone passant dans le lait.",
        sources: "RCP Imovane — base de données publique des médicaments (ANSM)\nHAS — prise en charge de l'insomnie chez l'adulte en premier recours\nHAS — prescription médicamenteuse chez le sujet âgé, médicaments à éviter",
        status: "",
        smr: "",
        tags: "hypnotique, vigilance conduite",
        toxicity: "",
    },
    StarterDetail {
        name: "Tahor",
        indications: "Traitement des hypercholestérolémies pures et des dyslipidémies mixtes, en complément du régime, lorsque celui-ci est insuffisant, ainsi que de l'hypercholestérolémie familiale hétérozygote et homozygote. Prévention des événements cardiovasculaires en prévention secondaire après infarctus, accident vasculaire cérébral ischémique ou revascularisation, et en prévention primaire chez les patients à risque cardiovasculaire élevé, notamment le diabétique.",
        mechanism: "Inhibiteur compétitif de l'HMG-CoA réductase, enzyme limitante de la synthèse hépatique du cholestérol. La baisse du cholestérol intracellulaire hépatique induit une surexpression des récepteurs aux LDL à la surface des hépatocytes, ce qui augmente la captation et l'épuration des LDL circulantes. S'y ajoutent des effets pléiotropes de stabilisation de la plaque d'athérome et de réduction de l'inflammation vasculaire.",
        dosage: "La dose usuelle va de 10 à 80 mg par jour en une prise unique, à n'importe quel moment de la journée, l'atorvastatine ayant une durée d'action suffisante pour ne pas imposer la prise du soir. L'instauration se fait habituellement à 10 ou 20 mg, avec adaptation toutes les quatre semaines au minimum selon l'objectif de LDL-cholestérol fixé par le niveau de risque cardiovasculaire. Les doses élevées sont réservées aux hauts risques et aux hypercholestérolémies familiales, et sont réduites en cas d'association à un inhibiteur du CYP3A4 ou de facteurs de risque musculaire tels que l'âge avancé, l'hypothyroïdie non traitée ou l'insuffisance rénale.",
        contraindications: "Affection hépatique évolutive, élévation persistante et inexpliquée des transaminases au-delà de trois fois la limite supérieure de la normale, grossesse, allaitement, femme en âge de procréer sans contraception efficace, hypersensibilité ; association contre-indiquée à l'acide fusidique par voie générale.",
        ddi: "Les inhibiteurs puissants du CYP3A4, clarithromycine, érythromycine, itraconazole, kétoconazole, inhibiteurs de protéase et ritonavir, augmentent fortement l'exposition et le risque de rhabdomyolyse : réduction de dose, suspension temporaire ou changement d'antibiotique. La ciclosporine est contre-indiquée ou impose une dose minimale. L'association aux fibrates, notamment au gemfibrozil, à la colchicine et à l'acide fusidique majore la toxicité musculaire. Le jus de pamplemousse en grande quantité augmente l'exposition. Sous antivitamine K, l'INR peut s'élever lors de l'instauration ou de l'arrêt.",
        adverse: "Fréquemment myalgies et crampes sans élévation des CPK, troubles digestifs à type de nausées, constipation, diarrhée et flatulences, céphalées, élévation modérée des transaminases. Plus rarement myosite avec élévation des CPK et, exceptionnellement, rhabdomyolyse avec insuffisance rénale aiguë, hépatite, troubles du sommeil, et une légère augmentation du risque de diabète de novo qui ne remet pas en cause le bénéfice cardiovasculaire.",
        monitoring: "Bilan lipidique et transaminases avant l'instauration, puis bilan lipidique quatre à douze semaines après l'instauration ou toute modification de dose, ensuite une fois par an lorsque l'objectif est atteint. Transaminases en cas de symptômes hépatiques ou de facteur de risque, sans dosage systématique répété. CPK à l'instauration chez les sujets à risque musculaire et en cas de myalgies survenant sous traitement : un taux supérieur à cinq fois la normale impose l'arrêt. Devant des myalgies, penser à rechercher une hypothyroïdie non traitée.",
        iup: "Prenez un comprimé par jour, à l'heure qui vous convient le mieux, avec ou sans aliments : contrairement à d'autres statines, l'atorvastatine n'oblige pas à la prise du soir. En cas d'oubli, prenez le comprimé si vous y pensez dans la journée, sinon passez à la prise suivante sans jamais doubler la dose. Le traitement ne se ressent pas : il agit sur le risque d'infarctus et d'accident vasculaire cérébral, et il doit être poursuivi au long cours, le cholestérol remontant en quelques semaines à l'arrêt. Signalez des douleurs musculaires inhabituelles, une faiblesse ou une sensibilité des muscles, surtout si elles s'accompagnent de fièvre, d'urines foncées ou d'une fatigue importante : c'est le seul effet qui doit faire consulter rapidement. Évitez de consommer du pamplemousse en grande quantité et prévenez avant tout nouvel antibiotique ou antifongique, certains imposant une suspension temporaire de la statine. Le régime, l'activité physique et l'arrêt du tabac restent indispensables : le comprimé ne les remplace pas.",
        half_life: "Environ 14 heures pour l'atorvastatine, avec une inhibition de l'HMG-CoA réductase persistant 20 à 30 heures du fait des métabolites actifs",
        elimination: "Métabolisme hépatique intense par le CYP3A4 en métabolites actifs, élimination essentiellement biliaire et fécale ; excrétion urinaire inférieure à 2 %.",
        renal: "Pas d'adaptation de la posologie à la fonction rénale, l'élimination étant biliaire ; l'insuffisance rénale reste toutefois un facteur de risque de toxicité musculaire qui incite à la prudence sur les fortes doses.",
        pregnancy: "Contre-indiquée pendant la grossesse, avec contraception efficace requise chez la femme en âge de procréer et arrêt du traitement en cas de désir de grossesse ; allaitement contre-indiqué.",
        sources: "RCP Tahor — base de données publique des médicaments (ANSM)\nESC/EAS — prise en charge des dyslipidémies\nHAS — principales dyslipidémies, stratégies de prise en charge",
        status: "",
        smr: "",
        tags: "statine, surveillance biologique, contre-indiqué grossesse",
        toxicity: "",
    },
    StarterDetail {
        name: "Cordarone",
        indications: "Traitement et prévention des troubles du rythme sévères documentés, notamment les tachycardies ventriculaires, les tachycardies supraventriculaires et la prévention des récidives de fibrillation atriale, en particulier lorsqu'il existe une cardiopathie sous-jacente ou une insuffisance cardiaque qui contre-indique les autres antiarythmiques. L'amiodarone est également utilisée pour maintenir le rythme sinusal après cardioversion.",
        mechanism: "Antiarythmique de classe III qui bloque les canaux potassiques et allonge la durée du potentiel d'action et la période réfractaire de toutes les fibres myocardiques. Elle possède en outre des propriétés de classe I, II et IV, avec blocage sodique et calcique et antagonisme adrénergique non compétitif, ce qui explique la bradycardie et le ralentissement de la conduction. Sa très forte lipophilie entraîne une accumulation tissulaire massive, à l'origine de la longueur de sa demi-vie et de la persistance des effets après l'arrêt.",
        dosage: "Traitement d'attaque habituel de 3 comprimés à 200 mg par jour pendant huit à dix jours, en une ou plusieurs prises, puis recherche de la dose minimale efficace d'entretien, le plus souvent un demi-comprimé à un comprimé par jour. Beaucoup de schémas d'entretien retiennent une prise cinq jours sur sept afin de limiter l'accumulation. Les doses sont diminuées chez le sujet âgé, et la surveillance renforcée en cas d'insuffisance cardiaque.",
        contraindications: "Bradycardie sinusale et blocs sino-auriculaires ou auriculo-ventriculaires de haut degré non appareillés, dysfonction sinusale, dysthyroïdie non contrôlée, hypersensibilité à l'iode, association aux médicaments torsadogènes, allongement congénital du QT, grossesse sauf situation exceptionnelle, allaitement.",
        ddi: "L'association aux médicaments allongeant le QT expose aux torsades de pointes : antiarythmiques de classe Ia et III, sotalol, certains neuroleptiques, moxifloxacine, érythromycine intraveineuse, dompéridone, hydroxyzine, et toute situation d'hypokaliémie. L'amiodarone inhibe la P-glycoprotéine et plusieurs cytochromes : elle augmente fortement la digoxinémie, ce qui impose de réduire la digoxine de moitié, et elle augmente l'INR sous antivitamine K, imposant un contrôle rapproché et une baisse de dose. Avec les bêtabloquants, le vérapamil et le diltiazem, risque de bradycardie et de bloc. Avec la simvastatine et les autres statines métabolisées par le CYP3A4, risque accru de myopathie, la dose de simvastatine devant être limitée.",
        adverse: "Très fréquemment micro-dépôts cornéens, quasi constants et généralement asymptomatiques, et photosensibilisation cutanée. Fréquemment dysthyroïdies à l'iode, hypo comme hyperthyroïdie, élévation des transaminases, nausées, bradycardie, insomnie et cauchemars. Plus rarement mais gravement : pneumopathie interstitielle diffuse, hépatite aiguë, neuropathie périphérique, myopathie, troubles conductifs sévères, torsades de pointes, pigmentation cutanée ardoisée des zones exposées et névrite optique.",
        monitoring: "Avant l'instauration : TSH, transaminases, ionogramme avec kaliémie, ECG, radiographie thoracique. Sous traitement : TSH tous les six mois, y compris jusqu'à un an après l'arrêt du fait de la rémanence, transaminases régulièrement, ECG au moins annuel avec mesure du QT et de la fréquence. Radiographie thoracique et exploration fonctionnelle respiratoire au moindre signe respiratoire, examen ophtalmologique en cas de baisse d'acuité visuelle. Rechercher à l'interrogatoire toux sèche, dyspnée d'effort d'installation progressive, amaigrissement ou prise de poids, palpitations.",
        iup: "Ce médicament s'accumule dans l'organisme et son effet persiste plusieurs semaines à plusieurs mois après l'arrêt : respectez scrupuleusement le schéma de prise, y compris les schémas cinq jours sur sept, et n'arrêtez jamais de vous-même. Protégez-vous du soleil de façon stricte pendant toute la durée du traitement et plusieurs mois après : vêtements couvrants, chapeau, écran solaire indice très élevé sur le visage et les mains, y compris par temps couvert, car l'exposition peut provoquer des brûlures et, à la longue, une coloration gris-bleu définitive de la peau. Votre thyroïde doit être contrôlée par prise de sang tous les six mois : signalez un amaigrissement, des palpitations, une nervosité, ou au contraire une fatigue, une frilosité, une prise de poids et une constipation. Consultez sans attendre en cas de toux sèche qui s'installe, d'essoufflement à l'effort inhabituel ou de fièvre, car le poumon peut être touché. Signalez ce traitement à tout médecin ou dentiste avant toute nouvelle prescription, beaucoup de médicaments étant incompatibles, et évitez le pamplemousse. Si vous prenez un anticoagulant antivitamine K ou de la digoxine, leur dose devra être diminuée et contrôlée.",
        half_life: "Très longue, de l'ordre de 20 à 100 jours, avec une moyenne proche de 50 jours",
        elimination: "Métabolisme hépatique important, notamment par le CYP3A4, en déséthylamiodarone active ; élimination biliaire et fécale, excrétion rénale négligeable.",
        renal: "Pas d'adaptation de la posologie à la fonction rénale, l'élimination n'étant pas rénale. La kaliémie doit en revanche être surveillée chez l'insuffisant rénal en raison du risque de torsades de pointes.",
        pregnancy: "Contre-indiquée pendant la grossesse sauf situation exceptionnelle, du fait de la charge iodée et du risque de dysthyroïdie et de goitre fœtal ; allaitement contre-indiqué.",
        sources: "RCP Cordarone — base de données publique des médicaments (ANSM)\nESC 2020 — prise en charge de la fibrillation atriale\nHAS — bon usage des antiarythmiques et surveillance sous amiodarone",
        status: "",
        smr: "",
        tags: "antiarythmique, marge thérapeutique étroite, surveillance biologique, contre-indiqué grossesse",
        toxicity: "Marge thérapeutique étroite : un écart de dose ou une interaction suffit à faire basculer vers le sous-dosage ou la toxicité. Voir les sections Interactions et Surveillance.",
    },
    StarterDetail {
        name: "Digoxine",
        indications: "Ralentissement de la cadence ventriculaire dans la fibrillation atriale et le flutter auriculaire, en particulier chez le patient peu mobile ou lorsque les bêtabloquants sont insuffisants ou mal tolérés. Traitement adjuvant de l'insuffisance cardiaque à fraction d'éjection altérée restant symptomatique sous traitement optimal, où elle réduit les hospitalisations sans effet démontré sur la mortalité.",
        mechanism: "Inhibition de la pompe sodium-potassium ATPase membranaire du myocyte cardiaque, ce qui augmente le sodium intracellulaire puis, via l'échangeur sodium-calcium, le calcium disponible pour la contraction : effet inotrope positif. Parallèlement, la digoxine augmente le tonus vagal et ralentit la conduction dans le nœud auriculo-ventriculaire, d'où l'effet chronotrope et dromotrope négatif recherché dans la fibrillation atriale. Le potassium entre en compétition avec la digoxine sur la pompe : toute hypokaliémie majore la toxicité à concentration égale.",
        dosage: "Chez l'adulte à fonction rénale normale, l'entretien est habituellement de 0,125 à 0,25 mg par jour en une prise, la dose étant ajustée sur la digoxinémie et la fréquence cardiaque. Chez le sujet âgé, en cas de faible poids ou d'insuffisance rénale, on retient les doses les plus basses, éventuellement un jour sur deux, la forme à 0,125 mg étant alors adaptée. La digoxinémie cible recommandée dans l'insuffisance cardiaque est basse, de l'ordre de 0,5 à 0,9 ng/mL, et le prélèvement doit être réalisé au moins six heures après la prise.",
        contraindications: "Bloc auriculo-ventriculaire du deuxième ou du troisième degré non appareillé, dysfonction sinusale non appareillée, troubles du rythme ventriculaire, fibrillation atriale associée à un syndrome de Wolff-Parkinson-White, cardiomyopathie hypertrophique obstructive, hypokaliémie non corrigée, hypercalcémie, intoxication digitalique.",
        ddi: "L'amiodarone, la dronédarone, le vérapamil, la quinidine, l'itraconazole, la ciclosporine et les macrolides inhibent la P-glycoprotéine et augmentent nettement la digoxinémie : une réduction de dose, souvent de moitié avec l'amiodarone, et un contrôle du taux sont nécessaires. Les diurétiques hypokaliémiants, les corticoïdes et les laxatifs stimulants favorisent le surdosage en abaissant la kaliémie sans modifier la digoxinémie. Les bêtabloquants, le diltiazem et le vérapamil majorent la bradycardie et les troubles conductifs. Le millepertuis et la rifampicine diminuent l'exposition.",
        adverse: "Les signes les plus fréquents sont ceux du surdosage : anorexie, nausées, vomissements, diarrhée, asthénie, céphalées, confusion chez le sujet âgé, et troubles visuels caractéristiques avec vision floue, halos colorés et dyschromatopsie jaune-vert. Sur le plan cardiaque, bradycardie, blocs auriculo-ventriculaires, extrasystoles ventriculaires et tachycardies atriales avec bloc, pouvant aller jusqu'à des arythmies ventriculaires graves. Plus rarement gynécomastie et éruptions cutanées.",
        monitoring: "Créatininémie et clairance, kaliémie, magnésémie et calcémie avant l'instauration puis régulièrement, et systématiquement à chaque introduction ou modification d'un diurétique. Digoxinémie en cas de doute sur l'observance, de suspicion de surdosage, d'altération de la fonction rénale ou d'introduction d'un interactant, prélevée au moins six heures après la prise. Fréquence cardiaque et ECG à intervalles réguliers. Toute anorexie, nausée ou trouble visuel chez un patient digitalisé doit être considérée comme un surdosage jusqu'à preuve du contraire.",
        iup: "Prenez un comprimé par jour, toujours à la même heure, sans jamais doubler la dose en cas d'oubli : si vous avez oublié la prise de la veille, reprenez simplement le comprimé du jour. Surveillez votre pouls comme on vous l'a appris et signalez un pouls durablement inférieur à cinquante battements par minute. Consultez rapidement si vous perdez l'appétit, si vous avez des nausées, des vomissements, une fatigue inhabituelle, une confusion, ou si vous voyez trouble ou avec des halos jaunes ou verts autour des lumières : ce sont les signes d'un excès de digoxine dans le sang. Les diurétiques peuvent faire baisser votre potassium et rendre la digoxine toxique : ne les modifiez pas seul, faites vos prises de sang comme prévu et signalez toute diarrhée ou vomissements prolongés. Prévenez tout médecin qui vous prescrit un antibiotique, un antiarythmique ou un traitement du cœur, car beaucoup augmentent le taux de digoxine. Ne prenez ni laxatifs stimulants ni compléments de calcium sans avis.",
        half_life: "36 à 48 heures chez le sujet à fonction rénale normale, considérablement allongée en cas d'insuffisance rénale",
        elimination: "Élimination principalement rénale sous forme inchangée par filtration glomérulaire et sécrétion tubulaire ; substrat de la P-glycoprotéine, métabolisme hépatique très faible.",
        renal: "L'adaptation à la clairance de la créatinine est indispensable : espacement des prises ou réduction de dose dès l'insuffisance rénale modérée, doses minimales et digoxinémies rapprochées en cas d'insuffisance sévère. La digoxine n'est pas efficacement épurée par l'hémodialyse du fait de son grand volume de distribution.",
        pregnancy: "Utilisable pendant la grossesse si l'indication maternelle le justifie, avec surveillance rapprochée de la digoxinémie dont les besoins peuvent varier ; passage faible dans le lait, allaitement possible.",
        sources: "RCP Digoxine — base de données publique des médicaments (ANSM)\nESC 2021 — insuffisance cardiaque aiguë et chronique\nESC 2020 — prise en charge de la fibrillation atriale",
        status: "",
        smr: "",
        tags: "digitalique, marge thérapeutique étroite, surveillance biologique",
        toxicity: "Marge thérapeutique étroite : un écart de dose ou une interaction suffit à faire basculer vers le sous-dosage ou la toxicité. Voir les sections Interactions et Surveillance.",
    },
    StarterDetail {
        name: "Glucophage",
        indications: "Traitement du diabète de type 2, en particulier chez le patient en surpoids, en complément des mesures hygiéno-diététiques, en monothérapie de première intention ou en association aux autres antidiabétiques oraux et à l'insuline. La metformine est également utilisée chez l'adolescent à partir de dix ans, et hors AMM dans le syndrome des ovaires polykystiques.",
        mechanism: "La metformine réduit la production hépatique de glucose en inhibant la néoglucogenèse, par un mécanisme impliquant l'inhibition du complexe I mitochondrial et l'activation de l'AMP-kinase. Elle augmente parallèlement la captation musculaire du glucose et améliore la sensibilité périphérique à l'insuline. Elle ne stimule pas la sécrétion d'insuline, ce qui explique l'absence d'hypoglycémie en monothérapie et l'absence de prise de poids.",
        dosage: "Instauration progressive pour limiter les troubles digestifs : 500 ou 850 mg une fois par jour pendant les repas, puis augmentation par paliers d'une à deux semaines jusqu'à deux ou trois prises quotidiennes. La dose d'entretien usuelle se situe entre 1500 et 2000 mg par jour répartis en deux ou trois prises, la dose maximale étant de 3000 mg par jour. La dose est plafonnée en fonction du débit de filtration glomérulaire et réduite chez le sujet âgé fragile ; les formes à libération prolongée en une prise le soir sont une alternative en cas d'intolérance digestive.",
        contraindications: "Débit de filtration glomérulaire inférieur à 30 mL/min, acidocétose diabétique et pré-coma diabétique, toute acidose métabolique aiguë, affection aiguë susceptible d'altérer la fonction rénale telle que déshydratation, infection sévère ou choc, insuffisance cardiaque ou respiratoire décompensée, infarctus récent, insuffisance hépatique sévère, intoxication alcoolique aiguë et alcoolisme chronique.",
        ddi: "Les produits de contraste iodés imposent l'arrêt de la metformine au moment de l'examen et sa reprise seulement après contrôle de la fonction rénale, quarante-huit heures plus tard : c'est l'interaction à repérer systématiquement avant un scanner ou une coronarographie. Les diurétiques, les inhibiteurs de l'enzyme de conversion, les sartans et les AINS peuvent provoquer une insuffisance rénale fonctionnelle et faire basculer vers l'accumulation. L'alcool majore le risque d'acidose lactique. Les corticoïdes, les diurétiques et les bêta-2 mimétiques élèvent la glycémie et peuvent nécessiter un ajustement.",
        adverse: "Très fréquemment troubles digestifs en début de traitement ou lors d'une augmentation de dose : diarrhée, nausées, douleurs abdominales, ballonnements, perte d'appétit et goût métallique, habituellement transitoires et atténués par la prise au cours des repas. Au long cours, diminution de l'absorption de la vitamine B12 pouvant conduire à une carence avec anémie macrocytaire ou neuropathie. L'acidose lactique est exceptionnelle mais grave, survenant en situation d'accumulation : crampes musculaires, douleurs abdominales, hyperventilation, asthénie majeure.",
        monitoring: "Débit de filtration glomérulaire avant l'instauration puis au moins une fois par an, deux fois par an lorsqu'il est compris entre 45 et 60 mL/min, et tous les trois à six mois entre 30 et 45 mL/min ou chez le sujet âgé. HbA1c tous les trois mois jusqu'à l'objectif puis tous les six mois. Dosage de la vitamine B12 en cas de traitement prolongé, d'anémie ou de signes neurologiques. Réévaluer le traitement à chaque épisode aigu susceptible d'altérer la fonction rénale.",
        iup: "Prenez les comprimés pendant ou juste à la fin des repas, jamais à jeun : cela réduit nettement les diarrhées et les douleurs au ventre, qui sont fréquentes au début et s'atténuent en général en une à deux semaines. Ce médicament ne provoque pas d'hypoglycémie lorsqu'il est pris seul, mais il peut en provoquer s'il est associé à un sulfamide ou à de l'insuline. Suspendez le traitement et prévenez votre médecin en cas de diarrhée ou de vomissements importants, de fièvre élevée, d'infection sévère ou de forte chaleur avec impossibilité de boire, car la déshydratation est la principale situation à risque. Si un examen radiologique avec produit de contraste, scanner ou coronarographie, est programmé, signalez ce traitement : il devra être arrêté le jour de l'examen et repris seulement quarante-huit heures après, avec contrôle de la fonction rénale. Limitez fortement l'alcool. Consultez en urgence en cas de crampes musculaires diffuses, de douleurs abdominales, de respiration rapide et de fatigue intense inexpliquée. Ne rattrapez jamais un oubli en doublant la prise suivante.",
        half_life: "Environ 6,5 heures pour la phase plasmatique, avec une élimination érythrocytaire plus lente",
        elimination: "Non métabolisée : élimination rénale sous forme inchangée par filtration glomérulaire et sécrétion tubulaire active ; la fonction rénale conditionne entièrement l'exposition.",
        renal: "Débit de filtration glomérulaire supérieur ou égal à 60 mL/min : posologie usuelle. Entre 45 et 59 mL/min : dose maximale réduite, surveillance rapprochée. Entre 30 et 44 mL/min : dose fortement réduite, poursuite possible mais instauration déconseillée. Inférieur à 30 mL/min : contre-indiqué.",
        pregnancy: "La metformine peut être utilisée pendant la grossesse lorsque cela est jugé nécessaire, l'insuline restant le traitement de référence du diabète gestationnel et pré-gestationnel ; le passage dans le lait est faible et l'allaitement est possible avec surveillance du nourrisson.",
        sources: "RCP Glucophage — base de données publique des médicaments (ANSM)\nHAS — stratégie médicamenteuse du contrôle glycémique du diabète de type 2\nSFD — prise de position sur la prise en charge médicamenteuse du diabète de type 2",
        status: "",
        smr: "",
        tags: "biguanide",
        toxicity: "",
    },
    StarterDetail {
        name: "Ozempic",
        indications: "Traitement du diabète de type 2 insuffisamment contrôlé, en complément du régime et de l'exercice physique, en monothérapie lorsque la metformine est mal tolérée ou contre-indiquée, ou en association aux autres antidiabétiques. Le sémaglutide injectable hebdomadaire réduit également les événements cardiovasculaires majeurs chez le diabétique de type 2 à haut risque cardiovasculaire. Il n'a pas d'indication dans l'obésité sans diabète, indication qui relève d'une autre spécialité et d'un autre schéma de doses.",
        mechanism: "Analogue du GLP-1 résistant à la dégradation par la DPP-4, qui stimule la sécrétion d'insuline de façon glucose-dépendante et freine la sécrétion de glucagon. Il ralentit la vidange gastrique et agit sur les centres hypothalamiques de la satiété, d'où la réduction des prises alimentaires et la perte de poids. Le caractère glucose-dépendant de la stimulation insulinique explique le faible risque d'hypoglycémie en dehors d'une association aux sulfamides ou à l'insuline.",
        dosage: "Une injection sous-cutanée par semaine, le même jour chaque semaine, à n'importe quel moment de la journée, avec ou sans repas. Instauration à 0,25 mg par semaine pendant quatre semaines, dose d'escalade non thérapeutique destinée à la tolérance digestive, puis 0,5 mg par semaine ; après au moins quatre semaines, la dose peut être portée à 1 mg par semaine si le contrôle glycémique est insuffisant, puis à 2 mg par semaine. Aucune adaptation n'est prévue selon l'âge ou le poids, mais la dose d'un sulfamide ou de l'insuline associés doit souvent être réduite lors de l'instauration.",
        contraindications: "Hypersensibilité au sémaglutide. Le traitement n'est pas indiqué dans le diabète de type 1 ni dans l'acidocétose diabétique. Prudence particulière en cas d'antécédent de pancréatite, de gastroparésie ou de maladie inflammatoire digestive sévère, et en cas de rétinopathie diabétique proliférante traitée par insuline.",
        ddi: "Le ralentissement de la vidange gastrique peut modifier l'absorption des médicaments pris par voie orale : prudence avec les molécules à marge thérapeutique étroite, notamment la lévothyroxine, dont la TSH doit être recontrôlée. L'association aux sulfamides hypoglycémiants et à l'insuline expose à l'hypoglycémie et impose souvent d'en réduire la dose. Aucune interaction cliniquement significative n'est décrite avec les anticoagulants oraux directs ni avec les antivitamines K, mais l'INR sera contrôlé lors de l'instauration.",
        adverse: "Très fréquemment nausées, vomissements, diarrhée, constipation, douleurs abdominales, éructations, surtout en début de traitement et lors des augmentations de dose, généralement transitoires. Fréquemment réactions au site d'injection, asthénie, céphalées, lithiase biliaire favorisée par la perte de poids rapide. Plus rarement pancréatite aiguë, aggravation transitoire d'une rétinopathie diabétique lors d'une baisse glycémique rapide, et déshydratation avec insuffisance rénale fonctionnelle en cas de vomissements prolongés.",
        monitoring: "HbA1c tous les trois mois jusqu'à l'objectif puis tous les six mois, avec autosurveillance glycémique surtout en cas d'association à un sulfamide ou à l'insuline. Poids et tolérance digestive à chaque renouvellement. Créatininémie en cas de troubles digestifs marqués ou prolongés. Examen ophtalmologique selon le suivi habituel du diabète, renforcé en cas de rétinopathie connue. Rechercher une douleur abdominale intense et persistante irradiant dans le dos, qui doit faire évoquer une pancréatite et arrêter le traitement.",
        iup: "L'injection se fait une fois par semaine, le même jour, sous la peau du ventre, de la cuisse ou du haut du bras, en changeant de zone à chaque fois, et jamais dans le muscle ou la veine. Vous pouvez changer de jour de la semaine si nécessaire, à condition de respecter au moins trois jours entre deux injections. En cas d'oubli, faites l'injection dès que possible si l'oubli date de moins de cinq jours ; passé ce délai, sautez la dose et reprenez au jour habituel, sans jamais doubler. Les nausées et l'inconfort digestif sont fréquents au début et lors des augmentations de dose : mangez plus lentement, en portions réduites, évitez les repas gras et copieux, et sachez que cela s'atténue en général en quelques semaines. Consultez rapidement en cas de douleur violente au creux de l'estomac irradiant dans le dos avec vomissements, ou de vomissements et diarrhées prolongés qui vous empêchent de boire. Le stylo non entamé se conserve au réfrigérateur entre 2 et 8 degrés, sans jamais congeler ; une fois entamé il se garde plusieurs semaines selon la notice, à température ambiante ou au réfrigérateur, capuchon en place.",
        half_life: "Environ 1 semaine, ce qui autorise l'administration hebdomadaire et explique la persistance de l'effet un mois après l'arrêt",
        elimination: "Dégradation protéolytique du peptide et bêta-oxydation de la chaîne d'acide gras ; les métabolites sont éliminés par voies urinaire et fécale, sans élimination rénale de la molécule intacte.",
        renal: "Pas d'adaptation posologique en cas d'insuffisance rénale légère, modérée ou sévère. L'expérience est limitée en insuffisance rénale terminale et l'utilisation n'y est pas recommandée. Une déshydratation liée aux troubles digestifs peut aggraver une insuffisance rénale préexistante.",
        pregnancy: "À éviter pendant la grossesse et à interrompre au moins deux mois avant une conception programmée en raison de la longue demi-vie ; allaitement déconseillé faute de données, un relais par insuline étant proposé.",
        sources: "RCP Ozempic — base de données publique des médicaments (ANSM)\nHAS — stratégie médicamenteuse du contrôle glycémique du diabète de type 2\nSFD — prise de position sur la prise en charge médicamenteuse du diabète de type 2",
        status: "",
        smr: "",
        tags: "analogue glp-1",
        toxicity: "",
    },
    StarterDetail {
        name: "Lantus",
        indications: "Traitement du diabète sucré de l'adulte, de l'adolescent et de l'enfant à partir de deux ans nécessitant un traitement par insuline. L'insuline glargine constitue l'insuline basale du schéma basal-bolus dans le diabète de type 1, et l'insuline de première intention lors du passage à l'insuline dans le diabète de type 2 insuffisamment contrôlé par les antidiabétiques.",
        mechanism: "Analogue de l'insuline humaine modifié pour être soluble en milieu acide et précipiter en microcristaux au pH neutre du tissu sous-cutané, d'où une libération lente et régulière à partir du site d'injection. Il en résulte un profil d'action prolongé, sans pic marqué, couvrant environ vingt-quatre heures. L'insuline se lie au récepteur de l'insuline et favorise la captation périphérique du glucose et l'inhibition de la production hépatique de glucose.",
        dosage: "Une injection sous-cutanée par jour, à un horaire fixe choisi par le patient, matin ou soir, la régularité comptant plus que le moment. Dans le diabète de type 2, l'instauration se fait à faible dose, de l'ordre de 0,2 unité par kilogramme et par jour ou une dizaine d'unités, puis la dose est titrée par petits paliers de quelques unités tous les deux à trois jours selon la glycémie à jeun, jusqu'à l'objectif fixé. Dans le diabète de type 1, l'insuline basale représente environ la moitié des besoins quotidiens, complétée par les bolus prandiaux. La dose est réduite en cas d'insuffisance rénale ou hépatique, d'amaigrissement, d'activité physique accrue ou de réduction d'une corticothérapie.",
        contraindications: "Hypersensibilité à l'insuline glargine ou à l'un des excipients. Il n'existe pas d'autre contre-indication absolue, l'insuline étant indispensable ; l'hypoglycémie en cours est bien entendu une contre-indication à l'injection au moment considéré.",
        ddi: "Les corticoïdes, les diurétiques thiazidiques, les neuroleptiques atypiques, les bêta-2 mimétiques, les hormones thyroïdiennes et les œstroprogestatifs élèvent la glycémie et augmentent les besoins en insuline. À l'inverse, les sulfamides hypoglycémiants, les inhibiteurs de l'enzyme de conversion, les fibrates, la pentamidine et l'alcool majorent le risque d'hypoglycémie. Les bêtabloquants, en particulier non cardiosélectifs, masquent les signes adrénergiques de l'hypoglycémie et retardent la resucration.",
        adverse: "L'hypoglycémie est l'effet indésirable le plus fréquent et le plus important, avec sueurs, tremblements, faim, palpitations, troubles de la concentration, pouvant aller jusqu'à la perte de connaissance. Fréquemment réactions au site d'injection à type de rougeur ou de prurit, et lipodystrophies en cas de défaut de rotation des sites, responsables d'une absorption erratique. Plus rarement prise de poids, œdèmes en début de traitement, allergies et troubles transitoires de la réfraction lors d'une normalisation glycémique rapide.",
        monitoring: "Autosurveillance glycémique capillaire ou mesure continue selon le schéma, avec attention particulière à la glycémie à jeun qui guide la titration de la basale. HbA1c tous les trois mois. Inspection régulière des sites d'injection à la recherche de lipodystrophies, et vérification de la technique d'injection à chaque renouvellement. Surveillance du poids, de la pression artérielle, du bilan rénal et ophtalmologique dans le cadre du suivi du diabète, et vérification que le patient et l'entourage savent reconnaître et traiter une hypoglycémie.",
        iup: "Faites une injection par jour, toujours à la même heure, en sous-cutané dans le ventre, la cuisse ou le haut du bras, en changeant de point d'injection à chaque fois à l'intérieur d'une même zone : injecter toujours au même endroit crée des boules sous la peau qui rendent l'insuline imprévisible. La solution est limpide : ne l'agitez pas, ne la mélangez jamais dans la même seringue avec une autre insuline et n'injectez pas une solution trouble ou colorée. Gardez les stylos non entamés au réfrigérateur entre 2 et 8 degrés, sans jamais congeler, et le stylo en cours à température ambiante, à l'abri de la lumière, pendant quatre semaines au maximum, en notant la date d'ouverture. Ayez toujours sur vous de quoi vous resucrer, trois morceaux de sucre ou un jus de fruit, et reconnaissez les signes d'hypoglycémie : sueurs, tremblements, fringale, palpitations, vue trouble, difficulté à se concentrer. N'arrêtez jamais l'insuline, même si vous ne mangez pas : en cas de maladie, de fièvre ou de vomissements, contrôlez plus souvent la glycémie, cherchez les corps cétoniques si votre médecin vous l'a appris et prenez contact. Une activité physique inhabituelle, un repas sauté ou une consommation d'alcool augmentent le risque d'hypoglycémie, y compris plusieurs heures après.",
        half_life: "Notion peu pertinente pour cette insuline : la durée d'action est d'environ 24 heures, sans pic prononcé",
        elimination: "Dégradation partielle au site d'injection en métabolites actifs ; dégradation enzymatique hépatique, rénale et musculaire, comme pour l'insuline endogène.",
        renal: "Pas de règle d'adaptation chiffrée, mais les besoins en insuline diminuent en cas d'insuffisance rénale du fait d'une clairance réduite de l'insuline : réduction prudente des doses et surveillance glycémique rapprochée. Même précaution en cas d'insuffisance hépatique.",
        pregnancy: "Utilisable pendant la grossesse, l'insuline étant le traitement de référence du diabète chez la femme enceinte, avec adaptation fréquente des doses au fil des trimestres ; allaitement possible, sans restriction, avec surveillance des besoins qui peuvent diminuer.",
        sources: "RCP Lantus — base de données publique des médicaments (ANSM)\nHAS — stratégie médicamenteuse du contrôle glycémique du diabète de type 2\nSFD — référentiel de bonnes pratiques, insulinothérapie et autosurveillance glycémique",
        status: "",
        smr: "",
        tags: "insuline, surveillance biologique",
        toxicity: "",
    },
    StarterDetail {
        name: "Ventoline",
        indications: "Traitement symptomatique de la crise d'asthme et des exacerbations, quel que soit le traitement de fond. Prévention de l'asthme d'effort. Traitement symptomatique des exacerbations de bronchopneumopathie chronique obstructive. Test de réversibilité bronchique lors des explorations fonctionnelles respiratoires. La solution pour inhalation par nébuliseur est réservée aux crises sévères, à l'hôpital ou en ville sous surveillance.",
        mechanism: "Le salbutamol est un agoniste sélectif des récepteurs bêta-2 adrénergiques du muscle lisse bronchique, d'action rapide et de courte durée. La stimulation de ces récepteurs augmente l'AMP cyclique intracellulaire et relâche la fibre musculaire lisse, ce qui lève la bronchoconstriction en quelques minutes. Il n'a aucun effet sur l'inflammation bronchique, qui reste du ressort du corticoïde inhalé.",
        dosage: "Aérosol doseur à 100 microgrammes par bouffée. Crise : 1 à 2 bouffées, à renouveler quelques minutes plus tard si le soulagement est incomplet ; en cas de crise sévère, la répétition des bouffées se fait sous contrôle médical et impose un appel au 15 si l'amélioration n'est pas franche. Prévention de l'asthme d'effort : 1 à 2 bouffées quinze à trente minutes avant l'exercice. L'enfant reçoit les mêmes doses unitaires, systématiquement au travers d'une chambre d'inhalation, avec masque avant l'âge de trois à quatre ans. Solution pour nébulisation : 2,5 mg à 5 mg par nébulisation chez l'adulte, posologie adaptée au poids chez l'enfant. Il n'y a pas de posologie régulière : le recours répété, au-delà de deux fois par semaine hors effort, signe un asthme non contrôlé et impose une consultation.",
        contraindications: "Hypersensibilité au salbutamol ou aux excipients. Il n'existe pas d'autre contre-indication absolue à la forme inhalée, y compris chez le coronarien, où le rapport bénéfice-risque reste favorable ; prudence en cas de cardiopathie ischémique, de troubles du rythme, d'hyperthyroïdie, de diabète déséquilibré et d'hypokaliémie.",
        ddi: "Les bêtabloquants, en particulier non cardiosélectifs, antagonisent l'effet bronchodilatateur et peuvent déclencher un bronchospasme sévère chez l'asthmatique, y compris sous forme de collyre. Les hypokaliémiants, diurétiques de l'anse et thiazidiques, corticoïdes systémiques, dérivés xanthiques et digitaliques, majorent le risque d'hypokaliémie et de troubles du rythme lors de l'usage de fortes doses. Association prudente avec les médicaments allongeant l'intervalle QT et les autres sympathomimétiques.",
        adverse: "Tremblement fin des extrémités, palpitations, tachycardie, céphalées, nervosité et crampes musculaires, dose-dépendants et régressant spontanément. Irritation de la gorge et toux à l'inhalation. Aux fortes doses, hypokaliémie, hyperglycémie et acidose lactique. Rarement bronchospasme paradoxal imposant l'arrêt immédiat et un avis, agitation ou hyperactivité chez le jeune enfant, réactions d'hypersensibilité.",
        monitoring: "Le principal indicateur de suivi est la consommation elle-même : plus de deux utilisations par semaine en dehors de l'effort, ou plus d'un flacon par mois, traduisent un asthme non contrôlé et doivent conduire à revoir le traitement de fond. Vérifier la technique d'inhalation et l'état du dispositif à chaque délivrance. Kaliémie en cas de crise sévère ou d'usage répété de fortes doses, notamment sous diurétique. Débit expiratoire de pointe chez les patients éduqués à son usage.",
        iup: "Retirer le capuchon, agiter le flacon, expirer à fond en dehors de l'embout, placer celui-ci entre les lèvres bien serrées, puis déclencher la bouffée au tout début d'une inspiration lente et profonde, et retenir sa respiration une dizaine de secondes avant d'expirer par le nez. Attendre environ trente secondes avant une seconde bouffée, en agitant à nouveau le flacon. Une chambre d'inhalation est très utile chez l'enfant, la personne âgée et toute personne qui coordonne mal le geste ; en cas de crise, elle améliore nettement l'efficacité, à raison d'une bouffée à la fois suivie de cinq à dix respirations calmes. Ce médicament soulage mais ne traite pas la maladie : il ne remplace jamais le traitement de fond, qui doit être poursuivi même lorsque tout va bien. Toujours garder l'aérosol sur soi, vérifier régulièrement le compteur de doses et anticiper le renouvellement, un flacon vide donnant l'illusion de bouffées sans produit. Consulter rapidement si le soulagement devient incomplet, plus court ou nécessite des bouffées de plus en plus fréquentes, et appeler le 15 devant une gêne respiratoire qui ne cède pas après plusieurs bouffées, une difficulté à parler ou des lèvres bleutées.",
        half_life: "environ 4 à 6 heures",
        elimination: "Après inhalation, la fraction déglutie est métabolisée par sulfoconjugaison hépatique et intestinale ; l'élimination est essentiellement urinaire, sous forme de métabolite inactif et de produit inchangé.",
        renal: "Pas d'adaptation posologique pour la forme inhalée ; prudence en cas d'insuffisance rénale sévère lors de l'utilisation de fortes doses répétées.",
        pregnancy: "Utilisable pendant toute la grossesse et compatible avec l'allaitement : le contrôle de l'asthme maternel prime, une crise non traitée étant plus dangereuse pour le fœtus que le traitement.",
        sources: "RCP Ventoline — base de données publique des médicaments (ANSM)\nGINA — recommandations pour la prise en charge de l'asthme\nHAS — parcours de soins de l'asthme de l'adulte et de l'enfant",
        status: "",
        smr: "",
        tags: "bêta-2 mimétique",
        toxicity: "",
    },
    StarterDetail {
        name: "Symbicort",
        indications: "Traitement de fond continu de l'asthme persistant, lorsqu'un corticoïde inhalé associé à un bêta-2 mimétique de longue durée d'action est justifié, chez l'adulte, l'adolescent et l'enfant à partir de six ans selon le dosage. Certains dosages permettent le schéma dit d'ajustement, dans lequel le même inhalateur assure le traitement de fond et le traitement de secours. Traitement symptomatique de la bronchopneumopathie chronique obstructive sévère chez les patients présentant des exacerbations répétées malgré un bronchodilatateur de longue durée d'action.",
        mechanism: "Association d'un corticoïde inhalé, le budésonide, et d'un bêta-2 mimétique de longue durée d'action à délai d'action rapide, le formotérol. Le budésonide se fixe au récepteur intracellulaire des glucocorticoïdes et réduit la transcription des gènes pro-inflammatoires, ce qui diminue l'inflammation et l'hyperréactivité bronchiques. Le formotérol relâche le muscle lisse bronchique par stimulation des récepteurs bêta-2 avec un effet perceptible en une à trois minutes et maintenu environ douze heures, particularité qui autorise son emploi en secours.",
        dosage: "Turbuhaler dosé à 100/6, 200/6 et 400/12 microgrammes par inhalation. Asthme, traitement de fond classique : 1 à 2 inhalations deux fois par jour selon le dosage et la sévérité, avec recherche de la dose minimale efficace lors des réévaluations. Schéma d'ajustement avec les dosages 100/6 et 200/6 : 1 à 2 inhalations par jour en traitement de fond, complétées par une inhalation supplémentaire en cas de symptôme, sans dépasser huit inhalations par jour au total, et douze de façon exceptionnelle et transitoire, tout dépassement devant conduire à consulter. Bronchopneumopathie chronique obstructive : 2 inhalations deux fois par jour du dosage adapté, sans schéma de secours. Le dosage 400/12 n'est pas utilisé en traitement de secours.",
        contraindications: "Hypersensibilité au budésonide, au formotérol ou au lactose contenu dans le dispositif, qui contient des traces de protéines de lait. Prudence, sans contre-indication absolue, en cas de tuberculose pulmonaire active ou latente, d'infection respiratoire fongique ou virale, de cardiomyopathie obstructive, de troubles du rythme, d'allongement du QT, d'hyperthyroïdie, de phéochromocytome, de diabète mal équilibré et d'hypokaliémie non corrigée.",
        ddi: "Les inhibiteurs puissants du CYP3A4, kétoconazole, itraconazole, ritonavir, cobicistat, clarithromycine, augmentent l'exposition systémique au budésonide et le risque d'effets corticoïdes généraux : association à éviter ou à espacer. Les bêtabloquants, y compris en collyre, antagonisent le formotérol et peuvent provoquer un bronchospasme. Diurétiques hypokaliémiants, corticoïdes systémiques, dérivés xanthiques et digitaliques majorent le risque d'hypokaliémie et de trouble du rythme. Prudence avec les médicaments allongeant le QT et les IMAO.",
        adverse: "Candidose oropharyngée, dysphonie, irritation de la gorge et toux, directement liées au dépôt buccal et prévenues par le rinçage. Tremblement, palpitations et céphalées, surtout en début de traitement puis régressifs. Plus rarement hypokaliémie, hyperglycémie, troubles du sommeil, agitation, ecchymoses cutanées, et lors de traitements prolongés à forte dose les effets systémiques du corticoïde, freination surrénalienne, ostéoporose, cataracte, glaucome, ralentissement de la croissance chez l'enfant. Bronchospasme paradoxal rare imposant l'arrêt immédiat, et augmentation du risque de pneumonie chez le patient atteint de bronchopneumopathie chronique obstructive.",
        monitoring: "Évaluation du contrôle de l'asthme à chaque consultation, fondée sur les symptômes diurnes et nocturnes, la gêne à l'effort et la consommation de traitement de secours, avec réévaluation de la dose tous les trois mois environ en vue d'une décroissance. Vérification de la technique d'inhalation et du compteur de doses à chaque délivrance. Examen de la bouche en cas de gêne pharyngée. Mesure de la taille chez l'enfant traité au long cours, et surveillance ophtalmologique en cas de fortes doses prolongées. Signes évoquant une pneumonie chez le patient bronchopathe.",
        iup: "Dévisser et retirer le capuchon, tenir l'inhalateur bien vertical, molette colorée vers le bas, puis tourner la molette à fond dans un sens et la ramener en arrière jusqu'à entendre un clic : la dose est prête et ne doit pas être rechargée deux fois. Expirer à fond en dehors de l'embout, jamais dedans car l'humidité de l'expiration colle la poudre, puis serrer les lèvres sur l'embout et inspirer profondément et énergiquement, plus fort que pour un aérosol classique, avant de retenir sa respiration environ dix secondes. La poudre est très fine et souvent imperceptible : ne pas conclure à un dispositif vide, c'est le compteur de doses qui fait foi. Se rincer la bouche à l'eau et cracher après chaque prise, ce qui prévient la candidose et l'enrouement ; ne jamais laver l'inhalateur à l'eau, un chiffon sec sur l'embout suffit. En cas d'oubli, prendre la dose dès que possible sans doubler la suivante, et surtout ne pas interrompre le traitement parce que la respiration est bonne : c'est précisément le signe qu'il fonctionne. Consulter si le besoin d'inhalations supplémentaires augmente, si l'essoufflement réveille la nuit, ou si les symptômes ne cèdent pas malgré les prises de secours.",
        half_life: "environ 3 heures pour le budésonide et 10 heures pour le formotérol",
        elimination: "Le budésonide subit un important effet de premier passage hépatique et est métabolisé par le CYP3A4 en métabolites peu actifs éliminés dans les urines ; le formotérol est glucuroconjugué puis éliminé par voie urinaire.",
        renal: "Pas d'adaptation posologique.",
        pregnancy: "Peut être poursuivi pendant la grossesse et l'allaitement, le budésonide étant le corticoïde inhalé le mieux documenté dans cette situation ; l'objectif reste le contrôle de l'asthme maternel.",
        sources: "RCP Symbicort — base de données publique des médicaments (ANSM)\nGINA — recommandations pour la prise en charge de l'asthme\nHAS — bon usage des associations corticoïde inhalé et bêta-2 de longue durée d'action",
        status: "",
        smr: "",
        tags: "csi + bdla",
        toxicity: "",
    },
    StarterDetail {
        name: "Seretide",
        indications: "Traitement de fond continu de l'asthme persistant insuffisamment contrôlé par un corticoïde inhalé associé à un bêta-2 mimétique de courte durée d'action à la demande, ou déjà contrôlé par l'association des deux principes actifs pris séparément. Traitement symptomatique de la bronchopneumopathie chronique obstructive chez les patients présentant des exacerbations répétées malgré un traitement bronchodilatateur continu, avec le dosage le plus fort. L'association ne doit jamais être utilisée pour traiter une crise.",
        mechanism: "Association d'un corticoïde inhalé, le propionate de fluticasone, et d'un bêta-2 mimétique de longue durée d'action, le salmétérol. La fluticasone agit sur le récepteur intracellulaire des glucocorticoïdes et réduit l'inflammation et l'hyperréactivité bronchiques. Le salmétérol se fixe durablement au récepteur bêta-2 et procure une bronchodilatation d'environ douze heures, mais avec un délai d'action lent qui interdit son emploi en traitement de secours.",
        dosage: "Diskus à 100/50, 250/50 et 500/50 microgrammes par dose, et suspension pour inhalation à 50/25, 125/25 et 250/25 microgrammes par bouffée. Asthme chez l'adulte et l'adolescent : 1 inhalation de Diskus deux fois par jour, ou 2 bouffées de l'aérosol deux fois par jour, le dosage étant choisi selon la sévérité puis diminué lors des réévaluations vers la dose minimale efficace. Chez l'enfant, seuls les dosages les plus faibles sont utilisés, à partir de quatre ans pour l'aérosol avec chambre d'inhalation et de quatre ans pour le Diskus. Bronchopneumopathie chronique obstructive : 1 inhalation de Diskus 500/50 deux fois par jour. Les prises sont espacées d'environ douze heures, matin et soir.",
        contraindications: "Hypersensibilité à la fluticasone, au salmétérol ou au lactose du Diskus, qui contient des traces de protéines de lait. Prudence, sans contre-indication absolue, en cas de tuberculose pulmonaire, d'infection respiratoire non traitée, de troubles du rythme ou de la conduction, d'allongement du QT, d'hyperthyroïdie, de diabète déséquilibré et d'hypokaliémie.",
        ddi: "Le ritonavir et le cobicistat, ainsi que les autres inhibiteurs puissants du CYP3A4 comme le kétoconazole et l'itraconazole, augmentent fortement l'exposition systémique à la fluticasone, avec des cas rapportés de syndrome de Cushing et de freination surrénalienne : association déconseillée, à signaler au prescripteur. Les bêtabloquants, y compris en collyre, antagonisent le salmétérol et exposent au bronchospasme. Diurétiques hypokaliémiants, corticoïdes systémiques, xanthines et digitaliques majorent le risque d'hypokaliémie et de trouble du rythme.",
        adverse: "Candidose oropharyngée, enrouement et irritation de la gorge, très fréquents et prévenus par le rinçage buccal. Céphalées, tremblements et palpitations. Crampes musculaires. Plus rarement hypokaliémie, hyperglycémie, anxiété, troubles du sommeil, ecchymoses, et effets systémiques du corticoïde lors des fortes doses prolongées, freination surrénalienne, ostéoporose, cataracte, glaucome, ralentissement de la croissance chez l'enfant. Bronchospasme paradoxal rare imposant l'arrêt, et augmentation du risque de pneumonie chez le patient atteint de bronchopneumopathie chronique obstructive.",
        monitoring: "Évaluation régulière du contrôle de l'asthme sur les symptômes, les réveils nocturnes, la gêne à l'effort et la consommation de bronchodilatateur de secours, avec tentative de décroissance après trois mois de contrôle. Vérification de la technique d'inhalation et du compteur de doses à chaque délivrance. Surveillance de la taille chez l'enfant, avis ophtalmologique en cas de traitement prolongé à forte dose. Recherche de signes de pneumonie, fièvre et majoration de l'expectoration, chez le bronchopathe.",
        iup: "Avec le Diskus, tenir le dispositif à plat dans une main, ouvrir le capot en poussant la molette jusqu'à la butée, puis pousser le levier à fond jusqu'au clic sans incliner ni secouer l'appareil, la dose étant alors déposée et pouvant tomber si on le retourne. Expirer à fond en dehors de l'embout, serrer les lèvres autour, inspirer profondément et rapidement par la bouche, retenir sa respiration une dizaine de secondes, puis refermer le capot, ce qui prépare le dispositif pour la fois suivante. Avec l'aérosol doseur, agiter le flacon, déclencher au début d'une inspiration lente et profonde, et utiliser une chambre d'inhalation chez l'enfant ou en cas de mauvaise coordination. Se rincer la bouche et cracher après chaque prise pour éviter les mycoses et l'enrouement, et ne jamais laver le Diskus à l'eau. Ce traitement se prend matin et soir tous les jours, y compris lorsque la respiration est bonne, et ne soulage pas une crise : le bronchodilatateur de secours doit rester disponible en permanence. Consulter rapidement si le besoin de traitement de secours augmente, si l'essoufflement réveille la nuit, ou en cas de plaques blanches dans la bouche.",
        half_life: "salmétérol environ 5 heures, avec une durée d'action bronchodilatatrice d'environ 12 heures ; la fluticasone inhalée a une demi-vie terminale plus longue",
        elimination: "La fluticasone subit un effet de premier passage hépatique quasi complet et est métabolisée par le CYP3A4 en un métabolite inactif à élimination principalement fécale ; le salmétérol est également métabolisé par le CYP3A4 et éliminé surtout par voie fécale.",
        renal: "Pas d'adaptation posologique.",
        pregnancy: "Peut être poursuivi pendant la grossesse et l'allaitement si le contrôle de l'asthme l'exige, en privilégiant la dose minimale efficace de corticoïde inhalé.",
        sources: "RCP Seretide — base de données publique des médicaments (ANSM)\nGINA — recommandations pour la prise en charge de l'asthme\nANSM — mise en garde sur l'association fluticasone inhalée et ritonavir ou cobicistat",
        status: "",
        smr: "",
        tags: "csi + bdla",
        toxicity: "",
    },
    StarterDetail {
        name: "Spiriva",
        indications: "Traitement bronchodilatateur continu de la bronchopneumopathie chronique obstructive, destiné à soulager la dyspnée et à réduire la fréquence des exacerbations. La forme Respimat dispose également d'une indication en traitement additionnel de l'asthme chez les patients restant symptomatiques malgré l'association d'un corticoïde inhalé et d'un bêta-2 mimétique de longue durée d'action. Il ne s'agit en aucun cas d'un traitement de la crise ou de l'exacerbation aiguë.",
        mechanism: "Le tiotropium est un antagoniste des récepteurs muscariniques de longue durée d'action. Il bloque les récepteurs M1, M2 et M3 mais se dissocie très lentement du récepteur M3 du muscle lisse bronchique, ce qui prolonge la bronchodilatation au-delà de vingt-quatre heures et autorise une prise unique quotidienne. En levant le tonus cholinergique bronchoconstricteur, il réduit la distension thoracique et améliore la tolérance à l'effort.",
        dosage: "Handihaler : une gélule à 18 microgrammes inhalée une fois par jour à heure fixe, la gélule ne devant jamais être avalée. Respimat : 2 bouffées de 2,5 microgrammes, soit 5 microgrammes au total, en une seule prise quotidienne à heure fixe. La posologie est la même quels que soient l'âge et le poids et ne doit jamais être augmentée en cas d'aggravation des symptômes, situation qui relève d'un avis médical. Réservé à l'adulte dans la bronchopneumopathie chronique obstructive, et à partir de six ans pour l'indication asthme avec le Respimat.",
        contraindications: "Hypersensibilité au tiotropium, à l'atropine ou à ses dérivés, et au lactose des gélules du Handihaler, qui contient des traces de protéines de lait. Prudence, sans contre-indication formelle, en cas de glaucome à angle fermé, d'hypertrophie bénigne de la prostate, d'obstruction du col vésical, de troubles du rythme récents ou d'infarctus récent, et d'insuffisance rénale avec clairance inférieure ou égale à 50 mL/min.",
        ddi: "L'association à un autre anticholinergique inhalé, ipratropium notamment, n'est pas recommandée : elle n'apporte rien et cumule les effets atropiniques. Les autres médicaments à propriétés anticholinergiques, antidépresseurs imipraminiques, antihistaminiques H1 de première génération, antispasmodiques urinaires comme l'oxybutynine, neuroleptiques phénothiaziniques et antiparkinsoniens atropiniques, additionnent sécheresse buccale, constipation, rétention urinaire et confusion chez le sujet âgé. Aucune interaction cliniquement significative avec les bêta-2 mimétiques, les corticoïdes inhalés ou la théophylline.",
        adverse: "Sécheresse buccale, très fréquente, généralement modérée et s'atténuant avec le temps. Pharyngite, dysphonie, toux, goût métallique. Constipation, dysgueusie, candidose buccale. Plus rarement rétention urinaire, surtout chez l'homme prostatique, tachycardie, palpitations, fibrillation atriale, vision floue et glaucome aigu par angle fermé en cas de projection du produit dans l'œil, troubles de la déglutition, occlusion intestinale, réactions d'hypersensibilité.",
        monitoring: "Évaluation de la dyspnée, de la tolérance à l'effort et du nombre d'exacerbations lors du suivi, avec vérification systématique de la technique d'inhalation, souvent défaillante chez le sujet âgé ou peu vaillant. Estimation de la fonction rénale avant l'instauration et lors du suivi chez le sujet âgé. Interrogatoire ciblé sur les troubles urinaires chez l'homme et sur les douleurs oculaires ou la vision de halos colorés. Contrôle de la persistance du sevrage tabagique, seule mesure qui modifie l'évolution de la maladie.",
        iup: "Avec le Handihaler, sortir une gélule de son blister juste avant l'emploi, la déposer dans la chambre centrale, refermer l'embout, puis appuyer une seule fois à fond sur le bouton latéral pour la perforer ; expirer à fond en dehors de l'appareil, puis inspirer lentement et profondément jusqu'à entendre vibrer la gélule, retenir sa respiration une dizaine de secondes, et répéter l'inspiration une seconde fois pour vider complètement la gélule. La gélule ne s'avale jamais et ne se conserve pas hors de son blister, car elle craint l'humidité. Avec le Respimat, il faut amorcer le dispositif à la première utilisation, puis tourner la base d'un demi-tour jusqu'au clic, expirer à fond, refermer les lèvres sur l'embout et appuyer sur le bouton au début d'une inspiration lente et profonde, en répétant l'opération pour la seconde bouffée. Éviter soigneusement toute projection dans les yeux, et consulter en cas de douleur oculaire, de vision trouble ou de halos colorés autour des lumières. La bouche se rince après la prise pour limiter la sécheresse, que l'on soulage par de petites gorgées d'eau, des chewing-gums sans sucre et une hygiène dentaire attentive. Ce traitement se prend tous les jours à la même heure, même en l'absence de gêne, et ne soulage pas un essoufflement aigu : le bronchodilatateur de courte durée d'action doit rester à portée de main, et une gêne qui augmente ou des crachats devenus purulents imposent une consultation.",
        half_life: "longue, supérieure à 24 heures, ce qui justifie la prise unique quotidienne",
        elimination: "Faiblement métabolisé, éliminé principalement par voie rénale sous forme inchangée après inhalation, avec une part fécale pour la fraction déglutie.",
        renal: "Pas d'adaptation de la dose, la posologie étant unique ; en cas de clairance inférieure ou égale à 50 mL/min, utiliser uniquement si le bénéfice attendu justifie le risque, avec surveillance clinique renforcée.",
        pregnancy: "Données limitées : à éviter pendant la grossesse et l'allaitement sauf nécessité, après réévaluation du rapport bénéfice-risque par le prescripteur.",
        sources: "RCP Spiriva — base de données publique des médicaments (ANSM)\nGOLD — rapport annuel sur la prise en charge de la bronchopneumopathie chronique obstructive\nHAS — parcours de soins de la bronchopneumopathie chronique obstructive",
        status: "",
        smr: "",
        tags: "anticholinergique",
        toxicity: "",
    },
    StarterDetail {
        name: "Singulair",
        indications: "Traitement additif de l'asthme persistant léger à modéré insuffisamment contrôlé par un corticoïde inhalé et le recours à un bêta-2 mimétique de courte durée d'action à la demande. Alternative au corticoïde inhalé à faible dose chez certains enfants présentant un asthme persistant léger sans antécédent récent de crise sévère ayant nécessité une corticothérapie orale, et chez qui la technique d'inhalation n'est pas maîtrisée. Prévention de l'asthme d'effort lorsqu'il constitue la composante prédominante. Il soulage également les symptômes de rhinite allergique saisonnière chez les patients asthmatiques concernés par les deux affections.",
        mechanism: "Le montélukast est un antagoniste puissant et sélectif du récepteur CysLT1 des leucotriènes cystéinylés, médiateurs libérés par les mastocytes et les éosinophiles. Le blocage de ce récepteur s'oppose à la bronchoconstriction, à l'œdème de la muqueuse et au recrutement des éosinophiles induits par les leucotriènes. Son action est complémentaire de celle des corticoïdes inhalés, dont il ne constitue pas un équivalent en termes d'efficacité anti-inflammatoire.",
        dosage: "Une prise quotidienne unique le soir. Adulte et adolescent à partir de quinze ans : un comprimé pelliculé à 10 mg. Enfant de six à quatorze ans : un comprimé à croquer à 5 mg. Enfant de deux à cinq ans : un comprimé à croquer à 4 mg ou un sachet de granulés à 4 mg. Dans la prévention de l'asthme d'effort chez l'adulte et l'adolescent, la prise se fait au moins deux heures avant l'exercice, sans prise supplémentaire dans les vingt-quatre heures suivantes chez un patient déjà traité quotidiennement.",
        contraindications: "Hypersensibilité au montélukast ou à l'un des excipients. Les comprimés à croquer contiennent de l'aspartam, source de phénylalanine, à prendre en compte en cas de phénylcétonurie. Le montélukast ne doit jamais être utilisé pour traiter une crise d'asthme, ni se substituer brutalement à une corticothérapie inhalée ou orale.",
        ddi: "Peu d'interactions cliniquement significatives. Les inducteurs enzymatiques puissants, rifampicine, phénobarbital, phénytoïne et carbamazépine, diminuent l'exposition au montélukast et peuvent réduire son efficacité, sans que le RCP impose d'ajustement systématique. Le gemfibrozil augmente l'exposition par inhibition du CYP2C8, sans adaptation recommandée. Aucune interaction gênante avec les corticoïdes inhalés, les bêta-2 mimétiques ou les antihistaminiques.",
        adverse: "Céphalées, douleurs abdominales, diarrhée, nausées, soif, fièvre et infections des voies aériennes supérieures, fréquents et généralement bénins. Troubles neuropsychiatriques, qui font l'objet d'une mise en garde renforcée et concernent enfants comme adultes : cauchemars, troubles du sommeil, irritabilité, agitation, anxiété, dépression, troubles de l'attention, et plus rarement idées et comportements suicidaires. Plus rarement encore, réactions d'hypersensibilité, éruptions cutanées, œdème de Quincke, élévation des transaminases, saignements et, exceptionnellement, granulomatose éosinophilique avec polyangéite révélée lors de la décroissance d'une corticothérapie.",
        monitoring: "Évaluation du contrôle de l'asthme après quelques semaines de traitement : en l'absence de bénéfice net, le montélukast doit être arrêté plutôt que poursuivi par habitude. Interrogatoire systématique, à chaque renouvellement, sur le sommeil, l'humeur, le comportement et la scolarité chez l'enfant, à la recherche d'un effet neuropsychiatrique. Vérification que le traitement de fond inhalé et le bronchodilatateur de secours sont bien poursuivis. Transaminases en cas de signe d'atteinte hépatique.",
        iup: "Une seule prise par jour, le soir, à heure régulière et indépendamment des repas pour le comprimé pelliculé, le comprimé à croquer devant être mâché et non avalé entier, et les granulés versés directement dans la bouche ou mélangés à une cuillerée de compote ou de yaourt froid, à consommer dans les quinze minutes. En cas d'oubli, prendre la dose dès que possible mais ne jamais prendre deux doses le même jour ; si l'oubli n'est constaté que le lendemain, reprendre simplement le rythme habituel. Ce médicament est un traitement de fond qui met plusieurs jours à agir : il ne soulage jamais une crise, ne remplace pas le corticoïde inhalé et ne doit pas conduire à abandonner le bronchodilatateur de secours, qui reste indispensable. Il faut signaler sans attendre au médecin ou au pharmacien tout cauchemar répété, insomnie, irritabilité inhabituelle, nervosité, tristesse ou changement de comportement, en particulier chez l'enfant et l'adolescent, ces effets régressant à l'arrêt. Consulter également si l'essoufflement réveille la nuit, si le recours au traitement de secours augmente, ou si des symptômes inhabituels comme une éruption, des fourmillements ou une aggravation respiratoire apparaissent lors d'une diminution de la cortisone.",
        half_life: "2,7 à 5,5 heures chez l'adulte jeune",
        elimination: "Métabolisme hépatique important par les CYP2C8, CYP3A4 et CYP2C9, avec une élimination biliaire et fécale quasi exclusive des métabolites ; l'élimination urinaire est négligeable.",
        renal: "Pas d'adaptation.",
        pregnancy: "Peut être poursuivi pendant la grossesse si le contrôle de l'asthme l'impose, l'expérience clinique étant rassurante, et l'allaitement est possible sous surveillance du nourrisson.",
        sources: "RCP Singulair — base de données publique des médicaments (ANSM)\nANSM — point d'information sur les troubles neuropsychiatriques associés au montélukast\nGINA — recommandations pour la prise en charge de l'asthme",
        status: "",
        smr: "",
        tags: "antileucotriène",
        toxicity: "",
    },
    StarterDetail {
        name: "Levothyrox",
        indications: "Traitement substitutif des hypothyroïdies de toutes causes, qu'elles soient auto-immunes, post-chirurgicales, post-radiques ou médicamenteuses, et des hypothyroïdies infracliniques lorsqu'elles sont symptomatiques ou biologiquement franches. Également utilisé en traitement frénateur de la TSH dans les goitres euthyroïdiens et après thyroïdectomie pour cancer différencié de la thyroïde.",
        mechanism: "La lévothyroxine est la forme synthétique de la thyroxine, convertie en triiodothyronine active dans les tissus périphériques par les désiodases. La T3 se lie aux récepteurs nucléaires thyroïdiens et module la transcription de nombreux gènes, ce qui augmente le métabolisme de base, la thermogenèse, le turnover osseux et la sensibilité du myocarde aux catécholamines. L'effet est lent à s'installer et l'équilibre hormonal n'est atteint qu'après plusieurs semaines.",
        dosage: "Chez l'adulte jeune sans comorbidité, la dose substitutive complète est de l'ordre de 1,6 à 1,8 microgramme par kilogramme et par jour, en une prise quotidienne. Chez le sujet âgé, le coronarien ou en cas d'hypothyroïdie ancienne et profonde, l'instauration est prudente, à faible dose, avec augmentation progressive par paliers en fonction de la TSH contrôlée après six à huit semaines. Les besoins augmentent pendant la grossesse, souvent de 25 à 50 %, et peuvent varier lors d'un changement de poids important, d'un changement de spécialité ou de forme pharmaceutique.",
        contraindications: "Hyperthyroïdie non traitée, insuffisance surrénale non substituée, infarctus du myocarde récent, myocardite aiguë, angor instable et troubles du rythme non contrôlés ; en pratique, toute cardiopathie ischémique impose une instauration très progressive plutôt qu'une abstention.",
        ddi: "Le fer, le calcium, le magnésium, les topiques gastro-intestinaux, la cholestyramine et le sévélamer diminuent fortement l'absorption : il faut respecter au moins deux heures d'écart, davantage pour la cholestyramine. Les inhibiteurs de la pompe à protons peuvent réduire l'absorption en modifiant le pH gastrique. Les inducteurs enzymatiques comme la rifampicine, la carbamazépine et la phénytoïne augmentent la clairance de la lévothyroxine. L'amiodarone perturbe la conversion périphérique et l'équilibre thyroïdien. Sous antivitamine K, l'équilibre de l'INR est modifié lors des changements de dose, et les besoins en insuline ou en antidiabétiques peuvent augmenter.",
        adverse: "Les effets indésirables sont essentiellement les signes de surdosage, c'est-à-dire d'hyperthyroïdie iatrogène : palpitations, tachycardie, tremblements, nervosité, insomnie, sueurs, amaigrissement, diarrhée, intolérance à la chaleur. Chez le sujet âgé, le surdosage expose à la fibrillation atriale, à l'angor et, au long cours, à une déminéralisation osseuse. Un sous-dosage se traduit par la persistance de la fatigue, de la frilosité, de la constipation et de la prise de poids.",
        monitoring: "TSH six à huit semaines après l'instauration, après chaque changement de dose, de spécialité ou de forme, puis une fois par an lorsque l'équilibre est stable. La T4 libre n'est utile qu'en cas de discordance ou dans les hypothyroïdies d'origine hypophysaire, où l'on se guide sur la T4 libre et non sur la TSH. Chez la femme enceinte, TSH à chaque trimestre et adaptation rapide. Surveillance de la fréquence cardiaque et de la tolérance clinique chez le coronarien et le sujet âgé.",
        iup: "Prenez votre comprimé le matin à jeun, environ trente minutes avant le petit-déjeuner, avec un verre d'eau, toujours de la même façon : la régularité compte autant que la dose, car la façon de prendre le médicament influence son absorption. Ne prenez jamais en même temps du fer, du calcium, du magnésium, un pansement gastrique ou un traitement contre le cholestérol de type cholestyramine : espacez d'au moins deux heures. En cas d'oubli, prenez le comprimé dans la journée si vous y pensez, sinon sautez la prise et ne doublez jamais la dose le lendemain ; un oubli isolé est sans conséquence, le médicament restant plusieurs jours dans l'organisme. L'effet met plusieurs semaines à s'installer : ne modifiez pas la dose de vous-même parce que vous ne vous sentez pas mieux, et n'utilisez pas ce médicament pour maigrir. Signalez palpitations, tremblements, nervosité, insomnie ou amaigrissement, qui évoquent un excès, comme la persistance d'une grande fatigue, d'une frilosité et d'une constipation, qui évoquent un manque. Si vous changez de marque, de dosage ou de forme, un contrôle de la TSH est nécessaire six à huit semaines plus tard, et prévenez immédiatement en cas de grossesse, car la dose doit être augmentée.",
        half_life: "Environ 7 jours chez le sujet euthyroïdien",
        elimination: "Désiodation périphérique, principalement hépatique, rénale et musculaire, avec conjugaison hépatique et élimination biliaire, fécale et urinaire des métabolites ; cycle entéro-hépatique.",
        renal: "Pas d'adaptation de la posologie à la fonction rénale.",
        pregnancy: "Le traitement doit impérativement être poursuivi et généralement augmenté pendant la grossesse, avec contrôle de la TSH à chaque trimestre ; allaitement possible, le passage dans le lait étant négligeable aux doses substitutives.",
        sources: "RCP Levothyrox — base de données publique des médicaments (ANSM)\nHAS — hypothyroïdie de l'adulte, pertinence des dosages et prise en charge\nANSM — recommandations de suivi lors d'un changement de spécialité de lévothyroxine",
        status: "",
        smr: "",
        tags: "hormone thyroïdienne, marge thérapeutique étroite, surveillance biologique",
        toxicity: "Marge thérapeutique étroite : un écart de dose ou une interaction suffit à faire basculer vers le sous-dosage ou la toxicité. Voir les sections Interactions et Surveillance.",
    },
    StarterDetail {
        name: "Inexium",
        indications: "Traitement du reflux gastro-œsophagien avec œsophagite érosive et traitement d'entretien après cicatrisation, traitement symptomatique du reflux, éradication d'Helicobacter pylori en association aux antibiotiques, traitement et prévention des lésions gastroduodénales induites par les AINS chez les patients à risque, syndrome de Zollinger-Ellison, et relais oral après traitement intraveineux d'une hémorragie ulcéreuse.",
        mechanism: "L'ésoméprazole est l'énantiomère S de l'oméprazole, prodrogue qui s'accumule dans le canalicule acide de la cellule pariétale gastrique où elle est convertie en sulfénamide actif. Celui-ci se lie de façon covalente et irréversible à la pompe à protons H+/K+-ATPase, ce qui bloque la sécrétion acide, basale comme stimulée. La restauration de la sécrétion nécessite la synthèse de nouvelles pompes, d'où une durée d'action nettement supérieure à la demi-vie plasmatique.",
        dosage: "Œsophagite érosive : 40 mg par jour pendant quatre semaines, prolongé quatre semaines supplémentaires en l'absence de cicatrisation, puis entretien à 20 mg par jour. Reflux symptomatique sans œsophagite : 20 mg par jour pendant quatre semaines, puis à la demande. Prévention des lésions sous AINS chez le patient à risque : 20 mg par jour. Éradication d'Helicobacter pylori : 20 mg deux fois par jour associés aux antibiotiques. En cas d'insuffisance hépatique sévère, la dose ne doit pas dépasser 20 mg par jour.",
        contraindications: "Hypersensibilité à l'ésoméprazole, aux benzimidazoles substitués ou à l'un des excipients ; association contre-indiquée au nelfinavir.",
        ddi: "L'association au clopidogrel est déconseillée, l'inhibition du CYP2C19 réduisant l'activation du clopidogrel et son efficacité antiagrégante : privilégier le pantoprazole. L'élévation du pH gastrique diminue l'absorption des molécules dont la solubilité en dépend, atazanavir et nelfinavir contre-indiqués, itraconazole, kétoconazole et erlotinib fortement diminués. L'ésoméprazole augmente l'exposition au diazépam, au citalopram, à la phénytoïne et à la digoxine, et majore la toxicité du méthotrexate à forte dose. Sous antivitamine K, l'INR doit être surveillé.",
        adverse: "Fréquemment céphalées, douleurs abdominales, diarrhée, nausées, flatulences et constipation. Au long cours : hypomagnésémie parfois symptomatique avec crampes, tétanie et arythmies, carence en vitamine B12, hyposidérémie, augmentation modérée du risque de fracture ostéoporotique, colite microscopique, néphrite interstitielle aiguë, et augmentation du risque d'infections digestives, notamment à Clostridioides difficile, et de pneumopathies. Plus rarement hépatite, cytopénies et réactions cutanées graves, dont un lupus cutané subaigu.",
        monitoring: "La surveillance biologique de routine n'est pas nécessaire pour les traitements courts. Pour les traitements prolongés : magnésémie avant l'instauration puis périodiquement, surtout en cas d'association aux diurétiques ou à la digoxine, dosage de la vitamine B12 et bilan martial en cas de traitement de plusieurs années, et créatininémie devant une dégradation inexpliquée de la fonction rénale. Le point essentiel de la surveillance est la réévaluation régulière de l'indication : la prescription au long cours doit être justifiée et une déprescription progressive envisagée lorsqu'elle ne l'est plus.",
        iup: "Prenez le comprimé le matin, environ trente à soixante minutes avant le petit-déjeuner : la pompe à protons n'est bloquée efficacement que si le médicament est présent au moment où l'estomac se met à sécréter, une prise après le repas est nettement moins efficace. Avalez le comprimé entier avec un verre d'eau ; s'il est difficile à avaler, il peut être dispersé dans un demi-verre d'eau non gazeuse, les granulés ne devant être ni croqués ni écrasés, et la suspension doit être bue dans les trente minutes. Le soulagement peut demander deux à trois jours : ne multipliez pas les prises pour aller plus vite. Ce traitement n'est pas destiné à être pris indéfiniment sans réévaluation : parlez-en à votre médecin si vous le prenez depuis des mois ou des années, l'arrêt se faisant de préférence progressivement pour éviter un rebond d'acidité. Signalez des crampes, des fourmillements, des palpitations ou une fatigue inhabituelle, qui peuvent traduire un manque de magnésium, ainsi qu'une diarrhée abondante et persistante. Si vous prenez du clopidogrel, signalez-le, car un autre médicament de la même famille devra être choisi.",
        half_life: "Environ 1,3 heure, mais l'action antisécrétoire persiste plus de 24 heures du fait de la liaison irréversible à la pompe",
        elimination: "Métabolisme hépatique complet par les CYP2C19 et CYP3A4 ; élimination des métabolites inactifs à environ 80 % par voie urinaire et le reste par voie fécale.",
        renal: "Pas d'adaptation de la posologie en cas d'insuffisance rénale, y compris sévère, l'élimination étant sous forme de métabolites inactifs.",
        pregnancy: "Utilisable pendant la grossesse si nécessaire, les données disponibles étant rassurantes ; allaitement déconseillé faute de données sur le passage dans le lait, sauf nécessité clairement établie.",
        sources: "RCP Inexium — base de données publique des médicaments (ANSM)\nHAS — bon usage des inhibiteurs de la pompe à protons chez l'adulte et déprescription",
        status: "",
        smr: "",
        tags: "ipp",
        toxicity: "",
    },
    StarterDetail {
        name: "Amoxicilline",
        indications: "Angine documentée à streptocoque bêta-hémolytique du groupe A, otite moyenne aiguë purulente de l'enfant, sinusite maxillaire aiguë bactérienne, pneumonie aiguë communautaire présumée à pneumocoque en première intention, exacerbation de bronchopneumopathie chronique obstructive, infections stomatologiques et abcès dentaires, érythème migrant de la maladie de Lyme chez l'enfant et la femme enceinte, éradication d'Helicobacter pylori en association, et prophylaxie de l'endocardite infectieuse avant certains gestes dentaires.",
        mechanism: "Aminopénicilline du groupe des bêta-lactamines, qui se lie de façon covalente aux protéines de liaison à la pénicilline, les transpeptidases pariétales. Le blocage de la transpeptidation interrompt la réticulation du peptidoglycane et active les autolysines, ce qui aboutit à la lyse de la bactérie. L'activité est bactéricide et temps-dépendante, d'où l'importance de répartir les prises sur le nycthémère. Elle est détruite par les pénicillinases, ce qui limite le spectre.",
        dosage: "Adulte : 1 g deux à trois fois par jour selon l'indication, soit 2 à 3 g par jour ; angine à streptocoque A 2 g par jour en deux prises pendant 6 jours ; pneumonie communautaire 3 g par jour en trois prises pendant 7 jours en règle générale. Enfant : 50 mg/kg/jour en deux prises pour l'angine pendant 6 jours, et 80 à 90 mg/kg/jour en deux ou trois prises pour l'otite moyenne aiguë, pendant 8 à 10 jours avant 2 ans et 5 jours après 2 ans, sans dépasser la dose adulte.",
        contraindications: "Allergie aux pénicillines ou à une autre bêta-lactamine, antécédent de réaction d'hypersensibilité immédiate ou de toxidermie sévère (anaphylaxie, syndrome de Stevens-Johnson, DRESS) à une pénicilline ou à une céphalosporine.",
        ddi: "Méthotrexate : diminution de l'excrétion rénale et majoration de la toxicité hématologique et muqueuse, association à surveiller étroitement voire à éviter à forte dose. Allopurinol : majoration du risque d'éruption cutanée. Antivitamines K : déséquilibre possible de l'INR, contrôle recommandé pendant et après l'antibiothérapie. La prise concomitante d'une mononucléose infectieuse expose à une éruption quasi constante, sans qu'il s'agisse d'une allergie vraie.",
        adverse: "Diarrhée, nausées, douleurs abdominales, candidoses buccales ou vaginales, éruption maculopapuleuse retardée fréquente et souvent bénigne. Plus rarement urticaire et anaphylaxie, colite à Clostridioides difficile, néphrite interstitielle aiguë, cristallurie à fortes doses, cytopénies, DRESS et toxidermies bulleuses.",
        monitoring: "Réévaluation clinique à 48-72 heures : l'absence d'apyrexie ou d'amélioration doit faire reconsidérer le diagnostic, le germe ou l'observance. Hémogramme et fonction rénale en cas de traitement prolongé ou de fortes doses. Surveillance de la survenue d'une éruption, en distinguant l'exanthème retardé banal de l'urticaire et de l'angio-œdème.",
        iup: "Les prises se répartissent régulièrement dans la journée, toutes les 8 ou 12 heures selon la prescription, car l'efficacité de cet antibiotique dépend du temps passé au-dessus du seuil actif : mieux vaut matin, midi et soir à heures fixes que trois prises rapprochées. Elles peuvent se faire pendant ou en dehors des repas, avec un grand verre d'eau. Pour la suspension buvable de l'enfant, il faut reconstituer avec de l'eau jusqu'au trait, bien agiter avant chaque prise, utiliser la pipette graduée en kilogrammes de poids de l'enfant et conserver le flacon au réfrigérateur pendant 14 jours au maximum. Le traitement se poursuit jusqu'au bout même si la fièvre tombe en deux jours, sous peine de rechute et de sélection de résistances. Une diarrhée modérée est fréquente et cède à l'arrêt, mais une diarrhée abondante, glaireuse ou sanglante, avec fièvre, impose un avis médical. Toute éruption avec urticaire, gonflement du visage ou des lèvres, ou gêne respiratoire, impose l'arrêt immédiat et un appel au 15.",
        half_life: "Environ 1 heure",
        elimination: "Métabolisme faible ; élimination essentiellement rénale sous forme inchangée, par filtration glomérulaire et sécrétion tubulaire, avec de fortes concentrations urinaires.",
        renal: "Clairance 10 à 30 mL/min : espacer les prises à toutes les 12 heures. Inférieure à 10 mL/min : une seule prise par 24 heures, avec une dose unitaire réduite.",
        pregnancy: "Utilisable pendant toute la grossesse et pendant l'allaitement ; c'est l'un des antibiotiques de premier choix dans ces situations.",
        sources: "RCP Amoxicilline — base de données publique des médicaments (ANSM)\nHAS et SPILF — antibiothérapie des infections respiratoires hautes et basses de l'adulte et de l'enfant\nANSM — bon usage des antibiotiques",
        status: "",
        smr: "",
        tags: "pénicilline",
        toxicity: "",
    },
    StarterDetail {
        name: "Augmentin",
        indications: "Otite moyenne aiguë de l'enfant en échec de l'amoxicilline ou avec conjonctivite associée évoquant Haemophilus influenzae, sinusite aiguë bactérienne, exacerbation de bronchopneumopathie chronique obstructive, pneumonie communautaire notamment chez le sujet âgé ou avec comorbidités, infections cutanées et des tissus mous dont les plaies et les morsures, infections stomatologiques sévères, infections gynécologiques hautes et certaines infections urinaires selon la documentation bactériologique.",
        mechanism: "Association d'une aminopénicilline bactéricide, qui bloque les protéines de liaison à la pénicilline et la synthèse du peptidoglycane, et d'un inhibiteur suicide des bêta-lactamases de classe A, l'acide clavulanique, dépourvu d'activité antibactérienne propre. L'inhibiteur protège l'amoxicilline de l'hydrolyse et restaure son activité sur les staphylocoques producteurs de pénicillinase, Haemophilus influenzae, Moraxella, de nombreuses entérobactéries et les anaérobies.",
        dosage: "Adulte : 1 g d'amoxicilline associé à 125 mg d'acide clavulanique, trois fois par jour, sans dépasser 3 g d'amoxicilline et 375 mg d'acide clavulanique par jour. Enfant : 80 mg/kg/jour d'amoxicilline en trois prises avec la formulation pédiatrique adaptée, sans dépasser la posologie adulte. La durée dépend de l'indication, de l'ordre de 5 jours pour une morsure récente non compliquée à 10 jours ou davantage pour une infection profonde ; elle est fixée par la prescription et suivie jusqu'à son terme.",
        contraindications: "Allergie aux pénicillines ou à une autre bêta-lactamine, antécédent de toxidermie sévère ou d'anaphylaxie à une bêta-lactamine, antécédent d'ictère ou d'atteinte hépatique survenu sous amoxicilline-acide clavulanique.",
        ddi: "Méthotrexate : majoration de la toxicité, association à surveiller ou à éviter. Allopurinol : risque accru d'éruption. Antivitamines K : déséquilibre fréquent de l'INR, contrôle pendant et après le traitement. Comme pour toute antibiothérapie, les topiques gastro-intestinaux pris au même moment peuvent réduire l'absorption.",
        adverse: "Diarrhée nettement plus fréquente qu'avec l'amoxicilline seule, imputable à l'acide clavulanique, nausées, vomissements, candidoses, éruptions maculopapuleuses. Plus rarement hépatite cholestatique parfois retardée de plusieurs semaines et plus fréquente chez l'homme âgé ou après un traitement prolongé, colite à Clostridioides difficile, néphrite interstitielle, anaphylaxie, DRESS et toxidermies bulleuses.",
        monitoring: "Réévaluation clinique à 48-72 heures. Transaminases et bilirubine en cas de traitement prolongé, d'hépatopathie préexistante ou d'apparition d'un ictère, y compris dans les semaines qui suivent l'arrêt. Surveillance de la tolérance digestive et de l'apparition d'une éruption.",
        iup: "Les comprimés se prennent au début des trois repas : c'est la condition principale d'une bonne tolérance digestive, l'acide clavulanique étant responsable de la diarrhée quand la prise se fait à jeun. Les prises sont réparties toutes les 8 heures, avec un grand verre d'eau. Chez l'enfant, la suspension se reconstitue jusqu'au trait, s'agite avant chaque prise, se dose avec la pipette graduée en kilogrammes et se conserve au réfrigérateur pendant la durée indiquée sur la boîte. Le traitement se termine complètement, même si tout va mieux au bout de deux jours. Une diarrhée modérée est banale, mais une diarrhée abondante, glaireuse ou sanglante avec fièvre impose un avis. Il faut également consulter devant un jaunissement de la peau ou des yeux, des urines foncées ou des selles décolorées, même plusieurs semaines après la fin du traitement.",
        half_life: "Environ 1 heure pour les deux composants",
        elimination: "Élimination majoritairement rénale sous forme inchangée pour l'amoxicilline ; l'acide clavulanique est plus largement métabolisé et éliminé par voies rénale, fécale et respiratoire.",
        renal: "Clairance 10 à 30 mL/min : deux prises par jour avec une dose unitaire réduite. Inférieure à 10 mL/min : une prise par 24 heures. Le dosage à 1 g/125 mg n'est pas adapté en dessous de 30 mL/min.",
        pregnancy: "Utilisable pendant la grossesse et l'allaitement lorsqu'une association est nécessaire, l'amoxicilline seule restant préférée quand elle suffit.",
        sources: "RCP Augmentin — base de données publique des médicaments (ANSM)\nHAS et SPILF — antibiothérapie des infections respiratoires et des infections cutanées bactériennes\nANSM — bon usage des antibiotiques",
        status: "",
        smr: "",
        tags: "pénicilline + inhibiteur",
        toxicity: "",
    },
    StarterDetail {
        name: "Pyostacine",
        indications: "Infections cutanées bactériennes à staphylocoque ou à streptocoque, notamment impétigo, furoncle, abcès et érysipèle, en particulier en cas d'allergie aux bêta-lactamines. Infections respiratoires hautes et basses, infections stomatologiques, et relais oral de certaines infections ostéo-articulaires en milieu spécialisé.",
        mechanism: "Antibiotique de la famille des streptogramines, associant deux composants synergistiques, la pristinamycine I et la pristinamycine II, qui se fixent sur des sites voisins de la sous-unité ribosomale 50S. La pristinamycine II modifie la conformation du ribosome et augmente l'affinité de la pristinamycine I, ce qui bloque l'élongation de la chaîne peptidique. Chaque composant seul est bactériostatique, mais leur association est bactéricide sur les staphylocoques, y compris certaines souches résistantes à la méticilline, et sur les streptocoques.",
        dosage: "Adulte : 2 à 3 g par jour répartis en deux ou trois prises, au cours des repas, la dose la plus élevée étant réservée aux infections sévères. Enfant : de l'ordre de 50 mg/kg/jour en deux ou trois prises. La durée est habituellement de 7 jours pour un érysipèle et de 7 à 10 jours dans les autres infections cutanées ou respiratoires courantes, mais elle dépend de l'indication et de la réponse clinique.",
        contraindications: "Hypersensibilité aux streptogramines, association aux alcaloïdes vasoconstricteurs de l'ergot de seigle comme l'ergotamine et la dihydroergotamine, en raison du risque d'ergotisme. Antécédent de toxidermie sévère sous pristinamycine.",
        ddi: "Inhibiteur du CYP3A4 : l'association à l'ergotamine et à la dihydroergotamine est contre-indiquée ; celle à la colchicine est déconseillée du fait du risque de toxicité colchicinique parfois mortelle. Prudence et surveillance des concentrations avec la ciclosporine et le tacrolimus, dont l'exposition augmente. Surveillance de l'INR sous antivitamine K.",
        adverse: "Nausées, vomissements, douleurs épigastriques et diarrhée, fréquents et souvent liés à une prise en dehors des repas. Éruptions cutanées et prurit. Plus rarement pustulose exanthématique aiguë généralisée, DRESS, toxidermies bulleuses, œsophagite en cas de prise sans eau ou en position allongée, colite à Clostridioides difficile.",
        monitoring: "Réévaluation clinique à 48-72 heures, en particulier dans l'érysipèle où l'apyrexie et la régression du placard sont attendues. Arrêt immédiat et avis en cas d'éruption, surtout si elle s'accompagne de fièvre, de pustules ou d'une atteinte des muqueuses. Pas de bilan biologique systématique dans les traitements courts.",
        iup: "Les comprimés se prennent au milieu du repas, avec un grand verre d'eau, sans être croqués ni sucés, et sans s'allonger dans la demi-heure qui suit : cela évite à la fois les nausées et l'irritation de l'œsophage. Les deux ou trois prises se répartissent régulièrement, matin et soir ou matin, midi et soir. Le traitement se poursuit jusqu'à la fin de la boîte prescrite, même si le placard cutané blanchit rapidement. En cas d'érysipèle de jambe, il faut surélever le membre au repos et consulter si la fièvre persiste au-delà de 48 à 72 heures ou si la rougeur progresse. Toute éruption cutanée, en particulier avec de la fièvre ou de petits boutons pustuleux, impose l'arrêt et un avis médical rapide.",
        half_life: "Environ 4 à 5 heures",
        elimination: "Métabolisme hépatique important, avec une élimination essentiellement biliaire et fécale et une part urinaire faible.",
        renal: "Pas d'adaptation de la posologie en cas d'insuffisance rénale, l'élimination étant principalement biliaire.",
        pregnancy: "Utilisable pendant la grossesse quel qu'en soit le terme lorsqu'elle est indiquée ; pendant l'allaitement, un antibiotique mieux évalué chez le nourrisson est préféré quand une alternative existe.",
        sources: "RCP Pyostacine — base de données publique des médicaments (ANSM)\nSPILF et SFD — prise en charge des infections cutanées bactériennes courantes\nANSM — bon usage des antibiotiques",
        status: "",
        smr: "",
        tags: "streptogramine",
        toxicity: "",
    },
    StarterDetail {
        name: "Cortancyl",
        indications: "Corticothérapie orale par la prednisone dans les affections inflammatoires, allergiques et auto-immunes : poussées de rhumatismes inflammatoires, pseudopolyarthrite rhizomélique et maladie de Horton, asthme et exacerbations de bronchopneumopathie, allergies sévères et œdème de Quincke, maladies systémiques, hémopathies malignes, néphropathies glomérulaires, maladies inflammatoires chroniques de l'intestin et prévention du rejet de greffe. Les durées et les doses varient considérablement selon l'indication, de la cure courte de quelques jours à la corticothérapie prolongée de plusieurs années.",
        mechanism: "La prednisone est une prodrogue transformée dans le foie en prednisolone, glucocorticoïde de synthèse qui se lie au récepteur cytosolique des glucocorticoïdes. Le complexe migre au noyau et réprime la transcription des gènes des cytokines pro-inflammatoires tout en induisant celle de protéines anti-inflammatoires, d'où un effet anti-inflammatoire et immunosuppresseur puissant. L'activité minéralocorticoïde résiduelle explique la rétention hydrosodée et la fuite de potassium.",
        dosage: "La posologie est fonction de l'indication : traitement d'attaque de l'ordre de 0,5 à 1 mg par kilogramme et par jour dans les maladies inflammatoires sévères, doses plus faibles dans la pseudopolyarthrite rhizomélique, cures courtes de quelques jours à doses moyennes dans l'asthme ou l'allergie. La prise se fait en une fois le matin pour respecter le rythme circadien du cortisol et limiter l'insomnie et le freinage surrénalien. La décroissance est progressive dès que la maladie est contrôlée, jusqu'à la dose minimale efficace ; au-delà de deux à trois semaines de traitement, l'arrêt ne peut être brutal.",
        contraindications: "Infection non contrôlée, notamment bactérienne, mycosique ou parasitaire, viroses en évolution telles qu'hépatite, herpès, varicelle et zona, états psychotiques non contrôlés, vaccination par vaccin vivant atténué. Les autres situations, ulcère, diabète, hypertension, ostéoporose, ne sont pas des contre-indications absolues mais imposent des précautions et une surveillance renforcée.",
        ddi: "Les AINS et l'aspirine majorent le risque d'ulcère et d'hémorragie digestive. Les inducteurs enzymatiques, rifampicine, carbamazépine, phénytoïne et millepertuis, diminuent l'efficacité de la corticothérapie et peuvent nécessiter une augmentation de dose. Les diurétiques hypokaliémiants, les laxatifs stimulants et l'amphotéricine B majorent l'hypokaliémie, ce qui accroît la toxicité de la digoxine et le risque de torsades de pointes. Sous antivitamine K, l'INR est modifié et doit être contrôlé. Les vaccins vivants atténués sont contre-indiqués en cas de corticothérapie immunosuppressive, et les besoins en antidiabétiques et en antihypertenseurs augmentent.",
        adverse: "Fréquemment rétention hydrosodée avec œdèmes et élévation de la pression artérielle, hyperglycémie voire diabète cortico-induit, hypokaliémie, prise de poids et redistribution des graisses, augmentation de l'appétit, insomnie, excitation et labilité de l'humeur, fragilité cutanée avec ecchymoses et vergetures, gastralgies. Au long cours : ostéoporose et fractures vertébrales, ostéonécrose aseptique de la hanche, myopathie cortisonique des ceintures, cataracte sous-capsulaire et glaucome, retard de cicatrisation, infections favorisées, syndrome de Cushing iatrogène, et insuffisance surrénale aiguë en cas d'arrêt brutal.",
        monitoring: "Pression artérielle, poids et glycémie à chaque consultation, kaliémie régulièrement, surtout en cas d'association aux diurétiques ou à la digoxine. Pour toute corticothérapie prolongée : bilan lipidique, densitométrie osseuse initiale puis répétée, apports en calcium et vitamine D et discussion d'un bisphosphonate, examen ophtalmologique annuel, dépistage et traitement d'une anguillulose avant l'instauration chez les patients ayant séjourné en zone d'endémie. Surveiller la survenue de troubles de l'humeur et de signes infectieux, souvent atténués et donc trompeurs sous corticoïdes.",
        iup: "Prenez tous les comprimés en une seule fois le matin, au cours du petit-déjeuner : c'est le moment qui respecte le rythme naturel de l'organisme et qui limite les insomnies. N'arrêtez jamais brutalement une corticothérapie qui dure depuis plus de deux à trois semaines : la diminution doit être progressive et décidée par le médecin, sous peine de fatigue intense, de malaise et de chute de tension. Réduisez le sel pendant toute la durée du traitement, limitez les sucres rapides et privilégiez les protéines et les aliments riches en potassium comme les fruits secs et les bananes. Signalez toute fièvre, tout mal de gorge ou toute infection même banale, car la corticothérapie masque les signes et favorise les infections, et évitez le contact avec la varicelle et le zona si vous ne les avez pas eus. Attendez-vous à une possible prise de poids, à un gonflement du visage, à une nervosité ou à des difficultés d'endormissement, qui régressent à l'arrêt ; signalez en revanche une soif et des urines abondantes, des crampes, une faiblesse des cuisses ou des troubles de la vue. Si le traitement est prolongé, portez sur vous une carte mentionnant la corticothérapie et signalez-la avant toute chirurgie ou hospitalisation.",
        half_life: "Demi-vie plasmatique de la prednisolone de 2 à 4 heures, mais demi-vie biologique de 12 à 36 heures, ce qui autorise la prise unique quotidienne",
        elimination: "Conversion hépatique de la prednisone en prednisolone active, puis métabolisation hépatique et élimination urinaire des métabolites conjugués.",
        renal: "Pas d'adaptation systématique de la posologie à la fonction rénale ; la surveillance de la kaliémie, de la volémie et de la pression artérielle doit toutefois être renforcée chez l'insuffisant rénal.",
        pregnancy: "Utilisable pendant la grossesse si l'indication maternelle le justifie, la prednisone passant peu la barrière placentaire, avec surveillance de la pression artérielle et de la glycémie maternelles ; allaitement possible aux doses usuelles, en espaçant si possible la tétée de la prise.",
        sources: "RCP Cortancyl — base de données publique des médicaments (ANSM)\nHAS — corticothérapie systémique prolongée, mesures associées et prévention de l'ostéoporose cortisonique",
        status: "",
        smr: "",
        tags: "corticoïde, surveillance biologique",
        toxicity: "",
    },
    StarterDetail {
        name: "Méthotrexate",
        indications: "À faible dose hebdomadaire, traitement de fond de la polyarthrite rhumatoïde et des rhumatismes inflammatoires chroniques, du rhumatisme psoriasique, du psoriasis sévère résistant et, dans certaines situations, de la maladie de Crohn corticodépendante. À forte dose, en milieu hospitalier uniquement, il s'agit d'un cytotoxique utilisé en oncohématologie : les deux usages n'ont ni les mêmes doses ni les mêmes précautions.",
        mechanism: "Antifolique qui inhibe la dihydrofolate réductase et bloque la régénération du tétrahydrofolate, indispensable à la synthèse des bases puriques et pyrimidiques. À faible dose hebdomadaire, l'effet dominant n'est pas cytotoxique mais anti-inflammatoire, par accumulation d'adénosine et inhibition de plusieurs enzymes folate-dépendantes des lymphocytes activés.",
        dosage: "Rhumatologie et dermatologie : une prise unique hebdomadaire, généralement de 7,5 à 25 mg par semaine, débutée bas puis augmentée par paliers selon la réponse et la tolérance. La voie sous-cutanée est préférée en cas d'intolérance digestive ou de réponse insuffisante par voie orale. Une supplémentation en acide folique, le plus souvent 5 mg en une prise, est associée à distance de la prise de méthotrexate, classiquement 24 à 48 heures après. La dose est réduite chez le sujet âgé, en cas d'insuffisance rénale même modérée et en cas de faible masse maigre.",
        contraindications: "Insuffisance rénale sévère, insuffisance hépatique et hépatopathie chronique alcoolique, alcoolisme, cytopénies préexistantes ou insuffisance médullaire, infection sévère ou évolutive, ulcère gastroduodénal évolutif, immunodépression, syndrome d'immunodéficience acquise, association aux vaccins vivants atténués, grossesse et allaitement.",
        ddi: "Le triméthoprime et le cotrimoxazole majorent la toxicité hématologique par addition d'effet antifolique : l'association est à proscrire, c'est la première interaction à vérifier au comptoir. Les AINS et les salicylés réduisent la sécrétion tubulaire du méthotrexate et augmentent son exposition, de même que le probénécide, les pénicillines et les inhibiteurs de la pompe à protons. La phénytoïne, la ciclosporine et le léflunomide majorent la toxicité. Les vaccins vivants atténués sont contre-indiqués pendant le traitement.",
        adverse: "Fréquemment nausées, anorexie, stomatite et aphtes, asthénie des 24 à 48 heures suivant la prise, élévation des transaminases, alopécie modérée. Plus rarement mais gravement : cytopénies pouvant aller à l'aplastie, pneumopathie d'hypersensibilité avec toux sèche et dyspnée fébrile, fibrose hépatique au long cours, infections opportunistes, et toxicité massive en cas de prise quotidienne par erreur.",
        monitoring: "Avant l'instauration : hémogramme complet, transaminases, gamma-GT, bilirubine, créatininémie avec clairance, radiographie thoracique, sérologies virales. Puis hémogramme et bilan hépatique tous les quinze jours pendant les trois premiers mois, à chaque augmentation de dose, ensuite tous les un à trois mois selon la stabilité. Surveiller la fonction rénale, qui commande l'élimination, et rechercher à l'interrogatoire toux sèche, dyspnée, fièvre, aphtose et signes hémorragiques.",
        iup: "Le méthotrexate se prend une fois par semaine, et une seule fois : fixez ensemble le jour de la semaine, notez-le sur la boîte et sur un calendrier, car la prise quotidienne par confusion est l'erreur classique et elle peut être mortelle. Ne rattrapez jamais un oubli en doublant la dose la semaine suivante : si l'oubli est constaté dans les deux jours, la prise peut être décalée, sinon on saute la semaine et on prévient le médecin. L'acide folique se prend un autre jour, généralement le lendemain ou le surlendemain, et ne doit jamais être pris le même jour que le méthotrexate. Consultez sans attendre en cas de fièvre, d'angine, d'aphtes de la bouche, de toux sèche persistante, d'essoufflement, de saignements ou d'ecchymoses inhabituelles. Évitez l'alcool, ne prenez aucun anti-inflammatoire sans avis, y compris en automédication, et signalez systématiquement ce traitement à tout médecin ou dentiste. Une contraception efficace est indispensable chez la femme comme chez l'homme, pendant le traitement et plusieurs mois après son arrêt.",
        half_life: "3 à 10 heures aux doses faibles, allongée en cas d'insuffisance rénale ou d'épanchement",
        elimination: "Élimination essentiellement rénale sous forme inchangée par filtration glomérulaire et sécrétion tubulaire active ; métabolisation hépatique et intracellulaire mineure en polyglutamates.",
        renal: "La clairance de la créatinine conditionne directement la toxicité : réduction de dose et espacement en cas d'insuffisance rénale légère à modérée, avec surveillance hématologique rapprochée ; contre-indiqué en cas d'insuffisance rénale sévère. Toute déshydratation, tout épisode fébrile ou toute introduction d'un néphrotoxique impose de réévaluer la fonction rénale.",
        pregnancy: "Contre-indiqué pendant la grossesse en raison d'un risque tératogène et abortif majeur, avec contraception efficace exigée chez les deux sexes pendant le traitement et après son arrêt ; allaitement contre-indiqué.",
        sources: "RCP Méthotrexate — base de données publique des médicaments (ANSM)\nANSM — mesures de réduction du risque d'erreur de rythme d'administration du méthotrexate par voie orale\nHAS — polyarthrite rhumatoïde, prise en charge et traitements de fond",
        status: "",
        smr: "",
        tags: "immunosuppresseur, marge thérapeutique étroite, surveillance biologique, contre-indiqué grossesse",
        toxicity: "Marge thérapeutique étroite : un écart de dose ou une interaction suffit à faire basculer vers le sous-dosage ou la toxicité. Voir les sections Interactions et Surveillance.",
    },
    StarterDetail {
        name: "Zithromax",
        indications: "Angine documentée à streptocoque A en cas d'allergie vraie aux bêta-lactamines, sinusite aiguë, surinfection de bronchite aiguë et exacerbation de bronchopneumopathie chronique obstructive, pneumonie communautaire notamment lorsqu'un germe atypique est suspecté, urétrites et cervicites non gonococciques à Chlamydia trachomatis, et certaines infections cutanées.",
        mechanism: "Macrolide de la sous-classe des azalides, qui se fixe de façon réversible à l'ARN ribosomal 23S de la sous-unité 50S et bloque la translocation de la chaîne peptidique. L'action est essentiellement bactériostatique, bactéricide sur certains germes aux concentrations obtenues dans les tissus. Sa concentration intracellulaire et tissulaire très élevée et prolongée explique son activité sur les germes intracellulaires, Chlamydia, Mycoplasma et Legionella, et la brièveté des schémas de traitement.",
        dosage: "Adulte : 500 mg une fois par jour pendant 3 jours dans les infections respiratoires et dans l'angine à streptocoque A ; 1 g en prise unique dans les urétrites et cervicites à Chlamydia trachomatis. Enfant : 20 mg/kg/jour en une prise pendant 3 jours dans l'angine, sans dépasser la dose adulte. Le schéma de 3 jours est complet : la concentration tissulaire reste active plusieurs jours après la dernière prise.",
        contraindications: "Hypersensibilité aux macrolides, association aux alcaloïdes vasoconstricteurs de l'ergot de seigle, insuffisance hépatique sévère, antécédent d'ictère cholestatique sous azithromycine.",
        ddi: "Allongement de l'intervalle QT : effet additif avec les antiarythmiques de classe I et III, l'hydroxyzine, la dompéridone, le citalopram et l'escitalopram, certains neuroleptiques, avec un risque de torsades de pointes majoré par l'hypokaliémie. Alcaloïdes de l'ergot de seigle contre-indiqués. Colchicine déconseillée. Antivitamines K : surveillance de l'INR. Les antiacides à base d'aluminium ou de magnésium abaissent le pic plasmatique et doivent être espacés de deux heures. Statines : risque musculaire, moindre qu'avec la clarithromycine.",
        adverse: "Diarrhée, nausées, douleurs abdominales, flatulences, céphalées, altération du goût. Plus rarement allongement du QT et torsades de pointes, hépatite cholestatique, hypoacousie et acouphènes lors de traitements prolongés ou à fortes doses, colite à Clostridioides difficile, toxidermies graves dont DRESS et syndrome de Stevens-Johnson.",
        monitoring: "Réévaluation clinique à 48-72 heures. ECG et kaliémie chez les patients à risque de torsades de pointes : sujet âgé, cardiopathie, bradycardie, hypokaliémie, association à d'autres médicaments allongeant le QT. Transaminases en cas de signes d'atteinte hépatique.",
        iup: "Une seule prise par jour, à heure fixe, pendant trois jours seulement : c'est un schéma complet, l'antibiotique restant actif dans les tissus environ une semaine après la dernière prise, il n'y a donc pas lieu de le prolonger. Les comprimés se prennent avec un grand verre d'eau, avec ou sans aliments ; il faut en revanche espacer d'au moins deux heures les pansements gastriques et les antiacides. Dans le traitement d'une infection à Chlamydia par 1 g en prise unique, les rapports doivent être protégés et le ou les partenaires traités en même temps, avec un contrôle et un dépistage des autres infections sexuellement transmissibles. Il faut signaler la prise d'un traitement pour le cœur, d'un antidépresseur ou d'un anticoagulant, qui peut nécessiter une précaution particulière. Consulter en cas de palpitations, de malaise, de jaunisse, ou de diarrhée abondante persistant après la fin du traitement.",
        half_life: "Environ 68 heures, avec une élimination tissulaire très lente",
        elimination: "Faible métabolisme hépatique par déméthylation ; élimination majoritairement biliaire sous forme inchangée, moins de 10 % par voie rénale.",
        renal: "Pas d'adaptation pour une clairance supérieure à 10 mL/min ; prudence en dessous, les données étant limitées.",
        pregnancy: "Utilisable pendant la grossesse et pendant l'allaitement lorsqu'un macrolide est indiqué.",
        sources: "RCP Zithromax — base de données publique des médicaments (ANSM)\nHAS et SPILF — antibiothérapie des infections respiratoires hautes et basses\nHAS — dépistage et prise en charge des infections à Chlamydia trachomatis",
        status: "",
        smr: "",
        tags: "macrolide, surveillance biologique",
        toxicity: "",
    },
    StarterDetail {
        name: "Ciflox",
        indications: "Pyélonéphrite aiguë et infection urinaire masculine dont la prostatite aiguë, cystite à risque de complication et infection urinaire documentée à germe sensible, infections digestives bactériennes dont la diarrhée du voyageur sévère et les salmonelloses, infections ostéo-articulaires et infections à Pseudomonas aeruginosa, otite externe maligne, et prophylaxie des sujets contacts d'une infection invasive à méningocoque.",
        mechanism: "Fluoroquinolone qui inhibe deux enzymes essentielles à la réplication de l'ADN bactérien, l'ADN gyrase, cible principale chez les bacilles à Gram négatif, et la topo-isomérase IV. Le blocage du surenroulement et de la séparation des chromosomes filles génère des coupures double brin létales. L'action est bactéricide et concentration-dépendante, avec un effet post-antibiotique prolongé, et le spectre couvre largement les entérobactéries et Pseudomonas aeruginosa.",
        dosage: "Adulte : 500 mg deux fois par jour, jusqu'à 750 mg deux fois par jour dans les infections sévères ou à Pseudomonas, selon l'indication et le germe. Pyélonéphrite aiguë simple : 7 jours. Infection urinaire masculine et prostatite : 14 jours en règle générale. Prophylaxie du méningocoque chez le sujet contact : 500 mg en dose unique. La durée n'est jamais raccourcie de sa propre initiative, en particulier dans la prostatite.",
        contraindications: "Hypersensibilité aux quinolones, antécédent de tendinopathie ou de rupture tendineuse sous fluoroquinolone, association à la tizanidine, enfant et adolescent en période de croissance sauf indication spécialisée, grossesse et allaitement. Prudence en cas d'épilepsie, de myasthénie, de déficit en G6PD ou d'antécédent d'anévrisme aortique.",
        ddi: "Chélation majeure de la molécule par le fer, le calcium, le magnésium, le zinc, l'aluminium, les antiacides, les topiques gastro-intestinaux, le sucralfate et les produits laitiers, avec une perte d'absorption pouvant dépasser la moitié de la dose : il faut prendre la quinolone au moins 2 heures avant ou 4 à 6 heures après ces produits. Inhibiteur puissant du CYP1A2 : tizanidine contre-indiquée, théophylline déconseillée avec risque convulsif, clozapine et caféine à surveiller. Antivitamines K : élévation marquée de l'INR. Corticoïdes : risque tendineux fortement majoré. Médicaments allongeant le QT et antidiabétiques oraux, avec risque de dysglycémie. Méthotrexate : toxicité augmentée.",
        adverse: "Nausées, diarrhée, douleurs abdominales, céphalées, insomnie, vertiges, éruptions. Plus rarement tendinopathie et rupture du tendon d'Achille, pouvant survenir dès les premiers jours et jusqu'à plusieurs mois après l'arrêt, photosensibilité, troubles neuropsychiques à type de confusion, anxiété, hallucinations ou convulsions, neuropathie périphérique parfois durable, allongement du QT, dysglycémie, colite à Clostridioides difficile, et de rares cas d'anévrisme ou de dissection aortique.",
        monitoring: "Réévaluation clinique à 48-72 heures, avec ECBU et antibiogramme dans les infections urinaires afin de réduire le spectre dès que possible. Glycémie chez le diabétique, en particulier sous sulfamide hypoglycémiant. Fonction rénale chez le sujet âgé. Recherche active de douleurs tendineuses, de paresthésies et de troubles neuropsychiques à chaque contact. ECG en cas de facteurs de risque de torsades de pointes.",
        iup: "Les deux prises se font à 12 heures d'intervalle, avec un grand verre d'eau, et il faut boire abondamment tout au long de la journée. Le point capital est l'espacement : tout ce qui contient du fer, du calcium, du magnésium, du zinc ou de l'aluminium, y compris les compléments alimentaires, les pansements gastriques et les produits laitiers, doit être pris au moins deux heures après l'antibiotique, ou quatre à six heures avant, faute de quoi le médicament n'est tout simplement plus absorbé. Il faut éviter le soleil et les cabines à UV pendant le traitement et se protéger si l'exposition est inévitable. Un effort sportif intense est à éviter, et toute douleur au talon, à la cheville ou à l'épaule impose l'arrêt du traitement et un avis médical le jour même. Il faut également signaler des fourmillements des mains ou des pieds, une confusion, une angoisse inhabituelle, des palpitations, ou une douleur abdominale ou dorsale brutale. Enfin, le traitement se poursuit jusqu'au terme prescrit, en particulier dans une prostatite où la durée est longue.",
        half_life: "3 à 5 heures",
        elimination: "Métabolisme hépatique partiel ; élimination majoritairement rénale sous forme inchangée par filtration et sécrétion tubulaire, avec une part biliaire et fécale non négligeable.",
        renal: "Clairance 30 à 60 mL/min : ne pas dépasser 1000 mg par jour. Inférieure à 30 mL/min : ne pas dépasser 500 mg par jour.",
        pregnancy: "Non recommandée pendant la grossesse, où une alternative est privilégiée ; le passage dans le lait fait déconseiller l'allaitement pendant le traitement.",
        sources: "RCP Ciprofloxacine — base de données publique des médicaments (ANSM)\nANSM — restriction d'utilisation des fluoroquinolones et effets indésirables invalidants et durables\nSPILF — prise en charge des infections urinaires bactériennes communautaires de l'adulte",
        status: "",
        smr: "",
        tags: "fluoroquinolone",
        toxicity: "",
    },
    StarterDetail {
        name: "Oflocet",
        indications: "Infections urinaires hautes et basses à risque de complication, infection urinaire masculine et prostatite, urétrites et cervicites non gonococciques, infections génitales hautes de la femme en association, ainsi que certaines infections respiratoires, cutanées et ORL documentées lorsqu'une alternative de spectre plus étroit n'est pas utilisable.",
        mechanism: "Fluoroquinolone qui inhibe l'ADN gyrase et la topo-isomérase IV bactériennes, bloquant le surenroulement et la ségrégation de l'ADN et provoquant des cassures létales du chromosome. L'action est bactéricide et concentration-dépendante. Le spectre couvre les entérobactéries, Haemophilus, les germes intracellulaires dont Chlamydia trachomatis, avec une activité sur Pseudomonas aeruginosa moindre que celle de la ciprofloxacine.",
        dosage: "Adulte : 200 mg deux fois par jour, jusqu'à 400 mg deux fois par jour dans les infections sévères, selon l'indication et le germe. Infection urinaire masculine et prostatite : 14 jours en règle générale. Infection génitale haute : 14 jours, toujours en association. Pyélonéphrite : 7 jours lorsque la fluoroquinolone est adaptée à l'antibiogramme.",
        contraindications: "Hypersensibilité aux quinolones, antécédent de tendinopathie sous fluoroquinolone, enfant et adolescent en période de croissance sauf indication spécialisée, grossesse et allaitement. Prudence en cas d'épilepsie, de myasthénie, de déficit en G6PD ou d'antécédent d'anévrisme aortique.",
        ddi: "Chélation par le fer, le calcium, le magnésium, le zinc, l'aluminium, les antiacides, le sucralfate, les topiques gastro-intestinaux et les produits laitiers : absorption fortement réduite, prise à espacer d'au moins 2 heures avant ou 4 à 6 heures après. Antivitamines K : élévation de l'INR, contrôle rapproché. Corticoïdes : majoration nette du risque tendineux. AINS : abaissement du seuil épileptogène. Médicaments allongeant le QT : effet additif. Antidiabétiques : risque de dysglycémie.",
        adverse: "Nausées, diarrhée, céphalées, vertiges, insomnie, éruptions. Plus rarement tendinopathie et rupture tendineuse, notamment achilléenne, photosensibilité, troubles neuropsychiques dont confusion et convulsions, neuropathie périphérique parfois persistante, allongement du QT, dysglycémie, colite à Clostridioides difficile, toxidermies graves, et de rares atteintes de l'aorte.",
        monitoring: "Réévaluation clinique à 48-72 heures et adaptation à l'antibiogramme dès qu'il est disponible, avec réduction du spectre chaque fois que possible. Glycémie chez le diabétique. Fonction rénale chez le sujet âgé, la dose devant être adaptée. Recherche systématique de douleurs tendineuses et de signes neurologiques pendant et après le traitement.",
        iup: "Les deux prises se font à environ 12 heures d'intervalle, avec un grand verre d'eau, en buvant abondamment sur la journée. Tout produit contenant du fer, du calcium, du magnésium, du zinc ou de l'aluminium, ainsi que les pansements gastriques et les laitages, doit être espacé d'au moins deux heures, sans quoi l'antibiotique est en grande partie neutralisé dans l'estomac. Il faut se protéger du soleil et éviter les UV artificiels pendant toute la durée du traitement. Toute douleur au tendon d'Achille, à la cheville ou à l'épaule impose d'arrêter les comprimés et de consulter le jour même, en évitant tout effort sur le membre concerné. Il faut aussi signaler des fourmillements, une confusion, une anxiété inhabituelle ou des palpitations. Le traitement se poursuit jusqu'au bout, en particulier dans les infections de la prostate où la durée prescrite est longue et où un arrêt prématuré expose à la rechute.",
        half_life: "6 à 8 heures",
        elimination: "Métabolisme hépatique très faible ; plus de 80 % de la dose est éliminée par voie rénale sous forme inchangée.",
        renal: "Clairance 20 à 50 mL/min : moitié de la dose quotidienne habituelle. Inférieure à 20 mL/min : 100 mg toutes les 24 heures.",
        pregnancy: "Non recommandée pendant la grossesse, où une alternative est privilégiée ; allaitement déconseillé pendant le traitement.",
        sources: "RCP Ofloxacine — base de données publique des médicaments (ANSM)\nANSM — restriction d'utilisation des fluoroquinolones et effets indésirables invalidants et durables\nSPILF — prise en charge des infections urinaires bactériennes communautaires de l'adulte",
        status: "",
        smr: "",
        tags: "fluoroquinolone",
        toxicity: "",
    },
    StarterDetail {
        name: "Monuril",
        indications: "Traitement de première intention de la cystite aiguë simple de la femme, en dose unique. Utilisable dans la cystite de la femme enceinte et dans certaines cystites à risque de complication lorsque la documentation bactériologique le permet.",
        mechanism: "Dérivé de l'acide phosphonique, sans parenté avec les autres familles d'antibiotiques. Il inhibe de façon irréversible la MurA, énolpyruvyl transférase qui catalyse la toute première étape cytoplasmique de la synthèse du peptidoglycane, ce qui bloque la construction de la paroi et entraîne la lyse bactérienne. L'action est bactéricide, sans résistance croisée avec les bêta-lactamines ni les quinolones, et les concentrations urinaires restent actives pendant 24 à 48 heures après une prise unique.",
        dosage: "3 g de fosfomycine trométamol en une prise unique, dissous dans un demi-verre d'eau, à distance des repas, de préférence le soir au coucher après avoir vidé la vessie. Une seule prise constitue le traitement complet de la cystite aiguë simple ; aucune prise supplémentaire n'est nécessaire. Dans la cystite à risque de complication, un schéma répété peut être prescrit par le médecin selon l'antibiogramme.",
        contraindications: "Hypersensibilité à la fosfomycine, insuffisance rénale sévère avec clairance inférieure à 10 mL/min, les concentrations urinaires devenant alors insuffisantes.",
        ddi: "Métoclopramide et plus largement les accélérateurs du transit intestinal : diminution des concentrations sériques et urinaires de fosfomycine, association déconseillée. En dehors de cela, le potentiel d'interaction est faible, la molécule n'étant pas métabolisée et ne se liant pas aux protéines plasmatiques.",
        adverse: "Diarrhée, nausées, douleurs abdominales et dyspepsie, céphalées, vertiges, vulvovaginite. Plus rarement réactions d'hypersensibilité avec urticaire, et exceptionnellement angio-œdème ou anaphylaxie.",
        monitoring: "Aucun examen n'est nécessaire dans la cystite aiguë simple, ni ECBU de contrôle en cas d'évolution favorable. Réévaluation si les signes urinaires persistent au-delà de trois jours ou récidivent dans les deux semaines, avec alors ECBU et antibiogramme. Toute fièvre ou douleur lombaire fait sortir du cadre de la cystite simple.",
        iup: "Le contenu du sachet se verse dans un demi-verre d'eau froide, se remue et se boit immédiatement, sans le préparer à l'avance. La prise se fait à distance des repas, au moins deux heures après le dîner, idéalement le soir juste avant de se coucher et après être allée uriner, afin que l'antibiotique reste concentré dans la vessie toute la nuit. Une seule prise suffit : il n'y a pas de traitement à poursuivre les jours suivants, ce qui surprend souvent et mérite d'être expliqué. Il faut boire abondamment dans les jours qui suivent et ne pas se retenir d'uriner. Les brûlures s'atténuent en général en 24 à 48 heures ; si elles persistent au-delà de trois jours, il faut reconsulter. L'apparition d'une fièvre, de frissons ou d'une douleur du bas du dos impose une consultation rapide, car il peut s'agir d'une atteinte du rein.",
        half_life: "Environ 4 heures dans le plasma, avec des concentrations urinaires actives pendant 24 à 48 heures",
        elimination: "Non métabolisée ; élimination sous forme inchangée par filtration glomérulaire, avec des concentrations urinaires très élevées et prolongées.",
        renal: "Pas d'adaptation en cas d'atteinte légère à modérée. Non recommandée en dessous d'une clairance de 10 mL/min, les concentrations urinaires devenant insuffisantes.",
        pregnancy: "Utilisable pendant la grossesse, où elle fait partie des traitements de première intention de la cystite ; utilisable également pendant l'allaitement.",
        sources: "RCP Monuril — base de données publique des médicaments (ANSM)\nSPILF — prise en charge des infections urinaires bactériennes communautaires de l'adulte\nHAS — cystite aiguë simple de la femme",
        status: "",
        smr: "",
        tags: "antibiotique urinaire",
        toxicity: "",
    },
    StarterDetail {
        name: "Furadantine",
        indications: "Traitement curatif de courte durée de la cystite documentée de la femme, en deuxième intention lorsque la fosfomycine et le pivmécillinam ne sont pas utilisables ou lorsque l'antibiogramme l'impose. L'utilisation en traitement prophylactique prolongé ou en cures répétées n'est plus recommandée en raison de la toxicité pulmonaire et hépatique.",
        mechanism: "Nitrofurane concentré dans l'urine, activé par les nitroréductases bactériennes en intermédiaires électrophiles très réactifs. Ces métabolites altèrent simultanément l'ADN, les protéines ribosomales et plusieurs enzymes du métabolisme bactérien. Cette action multi-cible explique un effet bactéricide et une sélection de résistances restée très faible malgré des décennies d'usage, mais aussi l'absence d'activité en dehors de l'arbre urinaire, les concentrations plasmatiques étant négligeables.",
        dosage: "Adulte : 300 mg par jour répartis en trois prises, au cours des repas, pendant 5 à 7 jours selon la recommandation suivie et la présentation. Le traitement est strictement curatif et de courte durée ; il ne doit pas être renouvelé ni prolongé, et toute demande de cure répétée doit conduire à réévaluer la prise en charge avec le prescripteur.",
        contraindications: "Clairance de la créatinine inférieure à 45 mL/min, déficit en G6PD, nourrisson de moins d'un mois, fin de grossesse et accouchement imminent, antécédent d'atteinte hépatique ou pulmonaire attribuée à la nitrofurantoïne, hypersensibilité aux nitrofuranes.",
        ddi: "Antiacides contenant du magnésium et topiques gastro-intestinaux : diminution de l'absorption, prises à espacer. Probénécide et sulfinpyrazone : diminution de la sécrétion tubulaire de nitrofurantoïne, avec une baisse des concentrations urinaires et donc de l'efficacité, et une augmentation des concentrations sériques et de la toxicité. Quinolones : antagonisme in vitro.",
        adverse: "Nausées, anorexie, douleurs abdominales, céphalées, coloration brune des urines, banale. Effets graves qui justifient la restriction d'usage : pneumopathie d'hypersensibilité aiguë avec fièvre, toux et dyspnée, fibrose pulmonaire lors des expositions prolongées, hépatite cytolytique ou cholestatique parfois sévère, neuropathie périphérique favorisée par l'insuffisance rénale, le diabète et l'âge, et anémie hémolytique en cas de déficit en G6PD.",
        monitoring: "Réévaluation clinique à 48-72 heures. Vérification de la fonction rénale avant l'instauration, le seuil de 45 mL/min conditionnant l'usage. En dehors des cures courtes, une surveillance hépatique et pulmonaire serait nécessaire, ce qui est précisément l'argument pour ne pas prolonger ni répéter le traitement. Interrogatoire à la recherche de traitements antérieurs par nitrofurantoïne.",
        iup: "Les gélules se prennent au cours des trois repas, avec un grand verre d'eau : la prise alimentaire améliore à la fois l'absorption et la tolérance digestive. Les urines prennent une teinte brun-jaune, c'est normal et sans gravité. Ce traitement est réservé aux cures courtes : il ne doit jamais être repris de sa propre initiative lors d'un nouvel épisode, ni utilisé en traitement d'entretien, en raison d'atteintes graves des poumons et du foie lors des expositions répétées. Il faut consulter sans délai en cas de toux, d'essoufflement, de douleur thoracique ou de fièvre survenant pendant ou après le traitement, ainsi que devant un jaunissement de la peau ou des yeux, des urines très foncées, ou des fourmillements des mains et des pieds. Le traitement se poursuit jusqu'au terme prescrit, en buvant abondamment.",
        half_life: "Environ 1 heure, la molécule étant très rapidement éliminée dans les urines",
        elimination: "Métabolisme partiel dans de nombreux tissus ; élimination rénale rapide par filtration glomérulaire et sécrétion tubulaire, avec de fortes concentrations urinaires et des concentrations sériques faibles.",
        renal: "Contre-indiquée en dessous d'une clairance de 45 mL/min : l'efficacité urinaire devient insuffisante et la toxicité systémique augmente. Pas d'adaptation au-dessus de ce seuil.",
        pregnancy: "Utilisable pendant la grossesse en dehors de la période proche de l'accouchement, où elle est contre-indiquée du fait du risque d'hémolyse néonatale ; allaitement possible sauf déficit en G6PD chez le nourrisson.",
        sources: "RCP Furadantine — base de données publique des médicaments (ANSM)\nANSM — restriction d'utilisation de la nitrofurantoïne et risques hépatiques et pulmonaires\nSPILF — prise en charge des infections urinaires bactériennes communautaires de l'adulte",
        status: "",
        smr: "",
        tags: "antibiotique urinaire, contre-indiqué grossesse",
        toxicity: "",
    },
    StarterDetail {
        name: "Selexid",
        indications: "Traitement de la cystite aiguë simple de la femme, en première intention au même titre que la fosfomycine, et de la cystite à risque de complication documentée. Utilisable dans la cystite de la femme enceinte.",
        mechanism: "Prodrogue estérifiée du mécillinam, bêta-lactamine du groupe des amidinopénicillines. Après hydrolyse en mécillinam, la molécule se lie sélectivement à la protéine de liaison à la pénicilline de type 2 des entérobactéries, cible différente de celle des autres pénicillines. Il en résulte une altération de la forme bactérienne, avec transformation en formes sphériques puis lyse ; l'action est bactéricide, sur un spectre étroit limité aux bacilles à Gram négatif, ce qui préserve la flore commensale.",
        dosage: "Cystite aiguë simple : 400 mg deux fois par jour pendant 5 jours. Cystite à risque de complication : 400 mg deux à trois fois par jour, avec une durée portée à 7 jours en règle générale. Les comprimés se prennent au cours d'un repas, avec un grand verre d'eau, en position assise ou debout.",
        contraindications: "Allergie aux pénicillines ou à une autre bêta-lactamine, anomalie œsophagienne ou sténose gastro-intestinale gênant le transit, troubles héréditaires du cycle de l'urée et déficit en carnitine, en raison de la libération d'acide pivalique. Les traitements répétés ou prolongés sont à éviter pour la même raison.",
        ddi: "Méthotrexate : diminution de l'excrétion rénale et majoration de la toxicité, association à surveiller. Antivitamines K : contrôle de l'INR pendant l'antibiothérapie. Valproate et autres médicaments libérant du pivalate : addition de la déplétion en carnitine lors d'expositions prolongées. Peu d'autres interactions cliniquement significatives.",
        adverse: "Nausées, diarrhée, douleurs abdominales, candidose vaginale, éruptions cutanées. Œsophagite ou ulcération œsophagienne en cas de prise sans eau ou en position allongée. Plus rarement réactions d'hypersensibilité, colite à Clostridioides difficile, et déplétion en carnitine lors de traitements répétés ou prolongés.",
        monitoring: "Réévaluation clinique à 48-72 heures : l'amélioration des signes urinaires est attendue en 48 heures environ. ECBU et antibiogramme en cas d'échec, de récidive précoce ou de doute diagnostique. Pas de bilan biologique systématique dans les cures courtes.",
        iup: "Les comprimés s'avalent entiers, sans être croqués ni écrasés, avec un grand verre d'eau, au cours d'un repas, en restant assise ou debout et sans s'allonger immédiatement après : c'est ce qui évite l'irritation de l'œsophage. Les prises se font matin et soir, à environ 12 heures d'intervalle, pendant les cinq jours prescrits, et le traitement se termine même si les brûlures ont disparu au bout de deux jours. Il faut boire abondamment et ne pas se retenir d'uriner pendant l'épisode. L'amélioration est attendue en 48 heures ; si les signes persistent au-delà de 72 heures ou récidivent rapidement, il faut reconsulter. L'apparition d'une fièvre, de frissons ou d'une douleur du bas du dos impose une consultation rapide, car elle évoque une atteinte du rein.",
        half_life: "Environ 1 heure pour le mécillinam",
        elimination: "Hydrolyse rapide et complète du pivmécillinam en mécillinam et en acide pivalique ; le mécillinam est éliminé essentiellement par voie rénale sous forme active, avec de fortes concentrations urinaires.",
        renal: "Pas d'adaptation en cas d'atteinte légère à modérée. En cas d'insuffisance rénale sévère, l'usage est déconseillé, les concentrations urinaires devenant insuffisantes.",
        pregnancy: "Utilisable à tous les termes de la grossesse, où il fait partie des traitements de première intention de la cystite ; utilisable également pendant l'allaitement.",
        sources: "RCP Selexid — base de données publique des médicaments (ANSM)\nSPILF — prise en charge des infections urinaires bactériennes communautaires de l'adulte\nHAS — cystite aiguë simple de la femme",
        status: "",
        smr: "",
        tags: "pénicilline",
        toxicity: "",
    },
    StarterDetail {
        name: "Bactrim",
        indications: "Infections urinaires documentées, cystite, infection urinaire masculine et pyélonéphrite lorsque le germe est sensible, prostatite. Traitement curatif et prophylaxie de la pneumocystose, toxoplasmose en association, nocardiose, et certaines infections cutanées à staphylocoque doré résistant à la méticilline d'origine communautaire.",
        mechanism: "Association de deux antifoliques agissant sur deux étapes successives de la synthèse bactérienne des folates : le sulfaméthoxazole inhibe la dihydroptéroate synthase, le triméthoprime inhibe la dihydrofolate réductase bactérienne, dont l'affinité est très supérieure à celle de l'enzyme humaine. Ce blocage séquentiel prive la bactérie de tétrahydrofolate et donc de bases puriques et de thymidine ; l'association est bactéricide alors que chaque composant pris isolément n'est que bactériostatique.",
        dosage: "Adulte, forme forte à 800 mg de sulfaméthoxazole et 160 mg de triméthoprime : un comprimé deux fois par jour. Cystite documentée : 3 jours. Infection urinaire masculine : 14 jours en règle générale. Prophylaxie de la pneumocystose : un comprimé fort trois fois par semaine ou un comprimé simple par jour, au long cours. Le traitement curatif de la pneumocystose utilise de fortes doses adaptées au poids, en milieu spécialisé.",
        contraindications: "Allergie aux sulfamides, déficit sévère en G6PD, insuffisance hépatique sévère, insuffisance rénale sévère, anémie mégaloblastique par carence en folates, nouveau-né de moins de six semaines, dernier trimestre de la grossesse, association au méthotrexate.",
        ddi: "Méthotrexate : addition d'effet antifolique et toxicité hématologique grave, association contre-indiquée ou à proscrire en pratique. Antivitamines K : potentialisation majeure avec élévation rapide de l'INR et risque hémorragique. Inhibiteurs de l'enzyme de conversion, sartans, spironolactone, amiloride et suppléments potassiques : hyperkaliémie parfois sévère, en particulier chez le sujet âgé. Sulfamides hypoglycémiants et répaglinide : hypoglycémies. Phénytoïne, ciclosporine, digoxine : concentrations ou toxicité augmentées.",
        adverse: "Nausées, vomissements, douleurs abdominales, éruptions cutanées et prurit, candidoses. Hyperkaliémie, hyponatrémie, élévation de la créatininémie par compétition tubulaire sans altération réelle du débit de filtration. Plus rarement neutropénie, thrombopénie, anémie mégaloblastique, hépatite, néphrite interstitielle, photosensibilité, et toxidermies graves — syndrome de Stevens-Johnson, syndrome de Lyell, DRESS — qui font la gravité de cette classe.",
        monitoring: "Réévaluation clinique à 48-72 heures. Hémogramme, kaliémie, natrémie et créatininémie en cas de traitement prolongé, chez le sujet âgé, l'insuffisant rénal ou en association à un bloqueur du système rénine-angiotensine. INR renforcé sous antivitamine K, dès les premiers jours. Arrêt immédiat devant toute éruption cutanée, sans attendre.",
        iup: "Les deux prises se font à environ 12 heures d'intervalle, au cours des repas, avec un grand verre d'eau, et il faut boire abondamment pendant toute la durée du traitement pour protéger les reins. Il ne faut jamais prendre ce médicament en cas d'antécédent d'allergie aux sulfamides, information à rappeler au médecin et au pharmacien. Se protéger du soleil, la peau devenant plus sensible. Toute éruption cutanée, même discrète, impose d'arrêter les comprimés et de consulter sans délai, en particulier si apparaissent des bulles, des lésions dans la bouche ou sur les yeux, ou de la fièvre. Il faut également signaler une fièvre avec mal de gorge ou des bleus spontanés, qui peuvent traduire une atteinte des cellules du sang. Enfin, prévenir si l'on prend un anticoagulant, un traitement pour la tension ou pour le diabète, car des adaptations sont souvent nécessaires.",
        half_life: "Environ 10 heures pour chacun des deux composants, allongée en cas d'insuffisance rénale",
        elimination: "Métabolisme hépatique du sulfaméthoxazole, principalement par acétylation ; élimination rénale prédominante des deux composants et de leurs métabolites.",
        renal: "Clairance 15 à 30 mL/min : moitié de la dose habituelle. Inférieure à 15 mL/min : contre-indiqué en dehors d'un cadre spécialisé avec surveillance.",
        pregnancy: "À éviter au premier trimestre en raison de l'effet antifolique et contre-indiqué en fin de grossesse du fait du risque d'ictère nucléaire chez le nouveau-né ; allaitement déconseillé chez le nouveau-né prématuré, ictérique ou déficitaire en G6PD.",
        sources: "RCP Bactrim — base de données publique des médicaments (ANSM)\nSPILF — prise en charge des infections urinaires bactériennes communautaires de l'adulte\nANSM — bon usage des antibiotiques",
        status: "",
        smr: "",
        tags: "sulfamide antibactérien, surveillance biologique, contre-indiqué grossesse",
        toxicity: "",
    },
    StarterDetail {
        name: "Doxycycline",
        indications: "Acné inflammatoire moyenne à sévère et rosacée, urétrites et cervicites à Chlamydia trachomatis, érythème migrant et formes précoces de la maladie de Lyme, pneumonies à germes atypiques, exacerbation de bronchopneumopathie chronique obstructive, rickettsioses, leptospirose, syphilis en cas d'allergie à la pénicilline, et chimioprophylaxie du paludisme dans les zones de résistance.",
        mechanism: "Cycline de deuxième génération qui se fixe de façon réversible à la sous-unité ribosomale 30S et empêche la fixation de l'ARN de transfert sur le site accepteur, ce qui interrompt l'élongation de la chaîne peptidique. L'action est bactériostatique. Sa forte liposolubilité lui confère une excellente diffusion intracellulaire et tissulaire, d'où son intérêt sur Chlamydia, Mycoplasma, les rickettsies et Borrelia.",
        dosage: "Adulte : 200 mg le premier jour puis 100 mg par jour, ou 100 mg deux fois par jour selon l'indication et la sévérité. Infection à Chlamydia trachomatis : 100 mg deux fois par jour pendant 7 jours. Érythème migrant de la maladie de Lyme : 200 mg par jour pendant 14 jours. Acné : 100 mg par jour, avec une évaluation de l'efficacité à trois mois et une durée limitée. Chimioprophylaxie du paludisme : 100 mg par jour, de la veille du départ jusqu'à quatre semaines après le retour.",
        contraindications: "Hypersensibilité aux cyclines, enfant de moins de 8 ans, deuxième et troisième trimestres de la grossesse, allaitement, association aux rétinoïdes systémiques comme l'isotrétinoïne en raison du risque d'hypertension intracrânienne.",
        ddi: "Chélation par le fer, le calcium, le magnésium, l'aluminium, le zinc, les antiacides, les topiques gastro-intestinaux et les produits laitiers, avec une perte d'absorption importante : espacer d'au moins 2 heures. Rétinoïdes systémiques contre-indiqués. Inducteurs enzymatiques comme la rifampicine, la carbamazépine, la phénytoïne et le phénobarbital : demi-vie raccourcie et efficacité diminuée. Antivitamines K : élévation de l'INR. Pénicillines : antagonisme théorique entre un bactériostatique et un bactéricide.",
        adverse: "Nausées, douleurs épigastriques, diarrhée, candidoses. Œsophagite et ulcération œsophagienne en cas de prise sans eau ou en position allongée, effet fréquent et évitable. Photosensibilité marquée, avec coups de soleil pour une exposition minime. Coloration dentaire définitive chez l'enfant de moins de 8 ans et chez le fœtus. Plus rarement hypertension intracrânienne bénigne, toxidermies, DRESS, hépatite.",
        monitoring: "Suivi essentiellement clinique. Recherche de céphalées inhabituelles avec troubles visuels, qui doivent faire évoquer une hypertension intracrânienne et arrêter le traitement. Surveillance de la tolérance digestive et cutanée. Dans l'acné, réévaluation à trois mois avec une durée de traitement limitée pour ne pas entretenir la sélection de résistances.",
        iup: "Le comprimé ou la gélule se prend au cours d'un repas, avec un grand verre d'eau, en position assise ou debout, et il faut éviter de s'allonger pendant l'heure qui suit : c'est ce qui empêche la brûlure de l'œsophage, complication classique de cette famille. Il faut espacer d'au moins deux heures le fer, le calcium, le magnésium, les pansements gastriques et les laitages, qui empêchent l'absorption. La protection solaire doit être stricte pendant tout le traitement, avec vêtements couvrants, chapeau et écran total, et sans cabine à UV, même par temps couvert. En prophylaxie du paludisme, la prise se poursuit quatre semaines après le retour, la protection contre les piqûres restant indispensable. En cas de traitement pour une infection sexuellement transmissible, les rapports doivent être protégés et le partenaire traité. Il faut consulter en cas de mal de gorge à la déglutition, de douleur derrière le sternum, ou de maux de tête inhabituels avec vision trouble.",
        half_life: "16 à 22 heures",
        elimination: "Élimination principalement digestive, par voie biliaire et par excrétion transmuqueuse intestinale sous forme de complexes inactifs, avec une faible part urinaire.",
        renal: "Pas d'adaptation : c'est la cycline utilisable chez l'insuffisant rénal, y compris en dialyse.",
        pregnancy: "Contre-indiquée à partir du deuxième trimestre en raison de l'atteinte des bourgeons dentaires et de l'os fœtal ; l'allaitement est déconseillé et une alternative est privilégiée.",
        sources: "RCP Doxycycline — base de données publique des médicaments (ANSM)\nHAS — borréliose de Lyme et autres maladies vectorielles à tiques\nHCSP — recommandations sanitaires pour les voyageurs, chimioprophylaxie du paludisme",
        status: "",
        smr: "",
        tags: "cycline, contre-indiqué grossesse",
        toxicity: "",
    },
    StarterDetail {
        name: "Flagyl",
        indications: "Vaginose bactérienne, trichomonose urogénitale avec traitement simultané du ou des partenaires, amœbose intestinale et hépatique, giardiase, infections à bactéries anaérobies notamment stomatologiques, digestives et gynécologiques, éradication d'Helicobacter pylori en association, et traitement de l'infection à Clostridioides difficile dans les formes non sévères lorsque les alternatives ne sont pas disponibles.",
        mechanism: "Dérivé 5-nitro-imidazolé, prodrogue qui pénètre passivement dans la bactérie ou le protozoaire. En milieu anaérobie, les systèmes de transport d'électrons à bas potentiel rédox réduisent le groupement nitro en dérivés radicalaires cytotoxiques qui fragmentent l'ADN et inhibent sa synthèse. L'action est bactéricide sur les anaérobies et sur les protozoaires, et nulle sur les bactéries aérobies, qui ne réalisent pas cette réduction.",
        dosage: "Vaginose bactérienne : 500 mg deux fois par jour pendant 7 jours, ou 2 g en prise unique. Trichomonose : 2 g en prise unique, ou 500 mg deux fois par jour pendant 7 jours, avec traitement simultané du partenaire. Amœbose intestinale : 1,5 g par jour en trois prises pendant 7 jours. Infections à anaérobies : 1 à 1,5 g par jour en deux ou trois prises, pour une durée fixée par l'indication. Enfant : de l'ordre de 30 mg/kg/jour.",
        contraindications: "Hypersensibilité aux imidazolés, association au disulfirame et à l'alcool. Prudence en cas d'antécédent de neuropathie périphérique, de trouble hématologique ou d'atteinte hépatique sévère.",
        ddi: "Alcool : effet antabuse avec bouffées vasomotrices, nausées, vomissements, céphalées et tachycardie ; l'abstention est requise pendant le traitement et dans les 48 heures qui suivent. Disulfirame : bouffées délirantes et confusion, association contre-indiquée. Antivitamines K : potentialisation nette de l'effet anticoagulant, contrôle rapproché de l'INR. Lithium : concentrations et toxicité augmentées. Busulfan et 5-fluorouracile : toxicité majorée.",
        adverse: "Goût métallique persistant, nausées, langue chargée, glossite, anorexie, douleurs abdominales, coloration brun-rouge des urines, candidoses. Plus rarement neuropathie périphérique sensitive lors des traitements prolongés ou à forte dose, vertiges, ataxie et encéphalopathie régressive, leucopénie transitoire, éruptions cutanées, hépatite.",
        monitoring: "Hémogramme en cas de traitement à forte dose ou prolongé au-delà de dix jours. Recherche de paresthésies des extrémités et de troubles de l'équilibre lors des cures longues, qui imposent l'arrêt. Réévaluation clinique en cas de persistance des symptômes gynécologiques, avec recherche d'une réinfection si le partenaire n'a pas été traité.",
        iup: "Les comprimés se prennent au cours des repas, avec un grand verre d'eau, en respectant l'intervalle entre les prises. Aucune boisson alcoolisée n'est autorisée pendant le traitement ni dans les 48 heures qui suivent la dernière prise, y compris le vin de cuisine et certains sirops ou solutions buvables alcoolisés, en raison de bouffées de chaleur, de nausées et de malaise. Un goût métallique dans la bouche et des urines de couleur foncée sont banals et sans gravité. En cas de trichomonose, le ou les partenaires doivent être traités en même temps et les rapports protégés jusqu'à la fin du traitement, faute de quoi la réinfection est certaine. Les ovules, quand ils sont prescrits, s'insèrent le soir au coucher, y compris pendant les règles sauf indication contraire, et le traitement complet est mené à son terme. Il faut consulter en cas de fourmillements des mains ou des pieds, de troubles de l'équilibre ou d'éruption.",
        half_life: "8 à 10 heures",
        elimination: "Métabolisme hépatique important par hydroxylation et glucuronoconjugaison, suivi d'une élimination urinaire des métabolites et d'une part fécale.",
        renal: "Pas d'adaptation en cas d'insuffisance rénale ; en revanche, la posologie doit être réduite en cas d'insuffisance hépatique sévère.",
        pregnancy: "Utilisable pendant la grossesse quel qu'en soit le terme lorsqu'il est indiqué ; pendant l'allaitement, un traitement de courte durée est possible, mais l'allaitement est suspendu 12 à 24 heures après une prise unique de 2 g.",
        sources: "RCP Flagyl — base de données publique des médicaments (ANSM)\nCNGOF et HAS — prise en charge des infections génitales basses de la femme\nANSM — bon usage des antibiotiques",
        status: "",
        smr: "",
        tags: "nitro-imidazolé",
        toxicity: "",
    },
    StarterDetail {
        name: "Triflucan",
        indications: "Candidose vulvovaginale aiguë en traitement par voie orale, et traitement d'entretien des candidoses vulvovaginales récidivantes. Candidoses oropharyngées et œsophagiennes, candidoses systémiques et candidémies, cryptococcose dont la méningite à cryptocoque, prophylaxie des infections fongiques chez le patient immunodéprimé, et certaines dermatophyties ou pityriasis versicolor en deuxième intention.",
        mechanism: "Antifongique de la classe des triazolés, qui inhibe la 14-alpha-déméthylase fongique, enzyme du cytochrome P450 responsable de la conversion du lanostérol en ergostérol. La déplétion en ergostérol et l'accumulation de stérols méthylés désorganisent la membrane du champignon et arrêtent sa croissance. L'action est fongistatique sur Candida, avec une excellente diffusion tissulaire, notamment dans le liquide céphalorachidien et les urines.",
        dosage: "Candidose vulvovaginale aiguë : 150 mg en prise unique. Candidoses récidivantes : 150 mg à intervalle régulier en traitement d'entretien, selon le schéma prescrit. Candidose oropharyngée : 50 à 100 mg par jour pendant 7 à 14 jours. Candidose œsophagienne : 14 à 30 jours de traitement. Les infections systémiques et la cryptococcose relèvent de doses plus élevées avec dose de charge, en milieu hospitalier.",
        contraindications: "Hypersensibilité aux azolés, association au cisapride, au pimozide, à la quinidine, à la terfénadine et à l'astémizole en raison du risque de torsades de pointes. Grossesse pour les traitements à forte dose ou prolongés.",
        ddi: "Inhibiteur du CYP2C9 et du CYP2C19, et du CYP3A4 aux doses élevées. Antivitamines K : élévation nette de l'INR, surveillance rapprochée. Sulfamides hypoglycémiants : risque d'hypoglycémie. Phénytoïne, ciclosporine, tacrolimus : concentrations augmentées. Simvastatine et atorvastatine : risque de rhabdomyolyse. Médicaments allongeant le QT : effet additif. La rifampicine diminue l'exposition au fluconazole et peut faire échouer le traitement.",
        adverse: "Nausées, douleurs abdominales, diarrhée, céphalées, éruptions cutanées, altération du goût. Élévation des transaminases, plus rarement hépatite parfois sévère. Allongement de l'intervalle QT et torsades de pointes. Alopécie lors des traitements prolongés à forte dose. Rares toxidermies graves, en particulier chez le patient immunodéprimé.",
        monitoring: "Transaminases avant et pendant les traitements prolongés ou en cas d'hépatopathie préexistante, avec arrêt en cas d'élévation significative ou de signes cliniques. INR renforcé sous antivitamine K. Kaliémie et ECG chez les patients à risque de torsades de pointes. Dans les candidoses vaginales récidivantes, réévaluation diagnostique avec prélèvement et recherche de facteurs favorisants comme un diabète.",
        iup: "Pour une mycose vaginale simple, une seule gélule de 150 mg suffit : elle se prend à n'importe quel moment de la journée, avec ou sans aliments, avec un verre d'eau. L'amélioration des démangeaisons demande un à trois jours et il est normal qu'une gêne persiste un peu après la prise ; il n'y a pas lieu de reprendre une seconde gélule sans avis. Le partenaire n'est traité que s'il présente lui-même des symptômes. En cas d'épisodes répétés, plus de trois ou quatre fois par an, il faut consulter pour rechercher une cause favorisante plutôt que de renouveler seule le traitement. Il est important de signaler la prise d'un anticoagulant, d'une statine ou d'un traitement pour le diabète, qui peuvent nécessiter une surveillance. Enfin, ce médicament est à éviter en cas de grossesse ou de projet de grossesse sans avis médical, un traitement local étant alors préféré, et il faut consulter devant un jaunissement de la peau, des urines foncées ou une éruption étendue.",
        half_life: "Environ 30 heures, ce qui autorise une prise unique ou une prise quotidienne",
        elimination: "Métabolisme hépatique faible ; plus de 80 % de la dose est éliminée dans les urines sous forme inchangée, avec de fortes concentrations urinaires.",
        renal: "Prise unique : pas d'adaptation. Traitements répétés avec une clairance inférieure à 50 mL/min : dose de charge habituelle puis moitié de la dose d'entretien.",
        pregnancy: "La prise unique de 150 mg est possible si nécessaire, mais un traitement local est préféré pendant la grossesse ; les fortes doses et les traitements prolongés sont contre-indiqués en raison d'un risque malformatif. L'allaitement est possible après une prise unique.",
        sources: "RCP Triflucan — base de données publique des médicaments (ANSM)\nANSM — fluconazole et risque malformatif lors d'une exposition pendant la grossesse\nHAS — prise en charge des candidoses vulvovaginales",
        status: "",
        smr: "",
        tags: "antifongique azolé, surveillance biologique, contre-indiqué grossesse",
        toxicity: "",
    },
    StarterDetail {
        name: "Zelitrex",
        indications: "Traitement du zona chez l'adulte immunocompétent, à débuter dans les 72 heures suivant l'éruption, et prévention des douleurs post-zostériennes chez le sujet de plus de 50 ans. Traitement de la primo-infection et des récurrences d'herpès génital, prévention des récurrences génitales fréquentes, traitement de l'herpès labial. Prévention de l'infection à cytomégalovirus après transplantation d'organe.",
        mechanism: "Prodrogue L-valyl de l'aciclovir, hydrolysée dès le premier passage intestinal et hépatique, ce qui multiplie par trois à cinq la biodisponibilité de l'aciclovir par rapport à sa forme orale. L'aciclovir est ensuite phosphorylé par la thymidine kinase virale, présente uniquement dans les cellules infectées, puis par des kinases cellulaires en aciclovir triphosphate, qui inhibe l'ADN polymérase virale et termine prématurément la chaîne d'ADN. Cette activation sélective explique une action virustatique ciblée et une bonne tolérance.",
        dosage: "Zona : 1 g trois fois par jour pendant 7 jours, à débuter impérativement dans les 72 heures suivant l'apparition de l'éruption. Herpès génital, primo-infection : 500 mg deux fois par jour pendant 10 jours. Récurrence génitale : 500 mg deux fois par jour pendant 5 jours, à débuter dès les prodromes. Prévention des récurrences : 500 mg par jour, avec réévaluation périodique. Herpès labial : 2 g deux fois, à 12 heures d'intervalle, sur une seule journée.",
        contraindications: "Hypersensibilité au valaciclovir, à l'aciclovir ou à l'un des excipients. Prudence chez le sujet âgé et l'insuffisant rénal, chez qui la posologie doit être adaptée et l'hydratation assurée.",
        ddi: "Peu d'interactions pharmacocinétiques. Probénécide et cimétidine réduisent la clairance rénale de l'aciclovir et augmentent son exposition. Prudence en association aux médicaments néphrotoxiques : ciclosporine, tacrolimus, aminosides, AINS, produits de contraste iodés, avec risque additif d'insuffisance rénale aiguë. Le mycophénolate voit ses concentrations modifiées en cas d'insuffisance rénale.",
        adverse: "Céphalées, nausées, douleurs abdominales, vertiges, diarrhée, éruptions cutanées. Plus rarement confusion, hallucinations, somnolence, agitation ou tremblements, essentiellement chez le sujet âgé, l'insuffisant rénal ou sous fortes doses, et régressifs à l'arrêt. Insuffisance rénale aiguë par précipitation intratubulaire d'aciclovir en cas de déshydratation. Exceptionnellement cytopénies et microangiopathie thrombotique chez l'immunodéprimé sévère recevant de très fortes doses.",
        monitoring: "Créatininémie et estimation de la clairance avant l'instauration chez le sujet âgé, l'insuffisant rénal et pour les fortes doses du zona, puis en cours de traitement si la situation évolue. Surveillance de l'état d'hydratation et des apports en boissons. Recherche de signes neuropsychiques, notamment confusion et somnolence, chez la personne âgée. Évaluation de la douleur du zona, avec avis si les douleurs persistent après la cicatrisation.",
        iup: "Le traitement du zona doit commencer le plus tôt possible, dans les 72 heures qui suivent l'apparition des vésicules : c'est cette précocité qui conditionne l'effet sur la douleur, y compris sur les douleurs qui peuvent persister des mois. Les trois prises quotidiennes se répartissent toutes les 8 heures environ, avec ou sans aliments, et il faut boire au moins un litre et demi d'eau par jour pour protéger les reins. Le traitement se poursuit pendant les sept jours complets, même si les croûtes se forment plus tôt. Tant que les vésicules ne sont pas sèches, il faut éviter le contact avec les femmes enceintes non immunisées, les nourrissons et les personnes immunodéprimées. En cas d'herpès génital, les rapports sont à protéger dès les premiers picotements et jusqu'à la cicatrisation complète. Il faut signaler rapidement une confusion, une somnolence inhabituelle, des hallucinations ou une diminution des urines, en particulier chez une personne âgée.",
        half_life: "Environ 3 heures pour l'aciclovir, nettement allongée en cas d'insuffisance rénale",
        elimination: "Hydrolyse quasi complète en aciclovir ; élimination rénale de l'aciclovir sous forme inchangée, par filtration glomérulaire et sécrétion tubulaire.",
        renal: "Adaptation indispensable. Dans le zona : 1 g deux fois par jour si la clairance est de 30 à 49 mL/min, 1 g par jour si elle est de 10 à 29 mL/min, et 500 mg par jour en dessous de 10 mL/min. Les posologies de l'herpès sont réduites selon le même principe.",
        pregnancy: "Utilisable pendant la grossesse et pendant l'allaitement lorsqu'un traitement est nécessaire, l'aciclovir et le valaciclovir étant les antiviraux les mieux évalués dans cette situation.",
        sources: "RCP Zelitrex — base de données publique des médicaments (ANSM)\nHAS — prise en charge du zona et prévention des douleurs post-zostériennes\nHAS — dépistage et prise en charge de l'herpès génital",
        status: "",
        smr: "",
        tags: "antiviral, surveillance biologique",
        toxicity: "",
    },
    StarterDetail {
        name: "Atarax",
        indications: "Traitement symptomatique des manifestations mineures de l'anxiété chez l'adulte ; prémédication avant une anesthésie générale ; traitement symptomatique de l'urticaire et des prurits, notamment lorsque leur composante nocturne gêne le sommeil.",
        mechanism: "Hydroxyzine, antihistaminique H1 de première génération de la famille des pipérazines, antagoniste compétitif des récepteurs histaminiques H1 périphériques et centraux. Le franchissement de la barrière hémato-encéphalique explique l'effet sédatif et anxiolytique, sans action sur le récepteur GABA-A, donc sans dépendance ni syndrome de sevrage, ce qui la distingue des benzodiazépines. Elle possède une activité anticholinergique et bloque le canal potassique hERG, ce qui fonde le risque d'allongement du QT et l'encadrement de ses posologies.",
        dosage: "Anxiété chez l'adulte : 50 mg par jour répartis en trois prises, soit 12,5 mg le matin et à midi et 25 mg le soir ; en cas d'anxiété importante, la posologie peut être portée à 100 mg par jour, dose maximale à ne pas dépasser. Prurit et urticaire : 25 mg le soir, éventuellement portés à 100 mg par jour en plusieurs prises. Prémédication : 1 à 2 mg/kg par voie orale environ une heure avant l'intervention. Chez le sujet de plus de 65 ans, la dose initiale doit être réduite de moitié et la posologie ne doit pas dépasser 50 mg par jour. Chez l'enfant de plus de trente mois, la posologie ne doit pas dépasser 2 mg/kg par jour. Le traitement doit être aussi court que possible et la posologie efficace la plus faible recherchée.",
        contraindications: "Hypersensibilité à l'hydroxyzine, à la cétirizine ou aux dérivés de la pipérazine, allongement congénital ou acquis de l'intervalle QT, facteurs de risque de torsades de pointes tels que bradycardie, hypokaliémie, hypomagnésémie, cardiopathie sévère ou antécédent familial de mort subite, risque de glaucome par fermeture de l'angle, risque de rétention urinaire par obstacle urétro-prostatique, porphyrie, grossesse et allaitement, enfant de moins de trente mois.",
        ddi: "Association contre-indiquée ou déconseillée avec les médicaments allongeant l'intervalle QT et les torsadogènes : antiarythmiques de classe IA et III, dompéridone, citalopram et escitalopram, halopéridol, macrolides, fluoroquinolones, antifongiques azolés, méthadone. Association déconseillée aux médicaments hypokaliémiants, notamment les diurétiques hypokaliémiants et les laxatifs stimulants, qui majorent le risque de torsades. Sédation additive avec l'alcool, les benzodiazépines, les hypnotiques, les opioïdes, les antitussifs opiacés et les autres antihistaminiques sédatifs. Effets anticholinergiques additifs avec les antidépresseurs tricycliques, les antiparkinsoniens anticholinergiques, l'oxybutynine et le néfopam : confusion, rétention urinaire, constipation. Les inhibiteurs du CYP3A4 augmentent l'exposition.",
        adverse: "Très fréquents : somnolence, souvent marquée et prolongée le lendemain, sédation, fatigue, céphalées. Fréquents : sécheresse buccale, vision floue, constipation, nausées, sensations vertigineuses, difficultés de concentration. Chez le sujet âgé : confusion, chutes, rétention urinaire, aggravation d'un glaucome. Plus rarement mais graves : allongement de l'intervalle QT et torsades de pointes, à l'origine des restrictions de posologie ; réactions d'hypersensibilité dont angio-œdème et choc anaphylactique ; réactions cutanées sévères à type de pustulose exanthématique aiguë généralisée, d'érythème pigmenté fixe, de syndrome de Stevens-Johnson et de DRESS ; convulsions ; hépatite.",
        monitoring: "Recherche systématique avant la première délivrance des facteurs de risque de torsades de pointes : cardiopathie, bradycardie, antécédent familial de mort subite, traitement diurétique, association à un autre médicament allongeant le QT. Kaliémie et magnésémie chez le patient sous diurétique ou en cas de troubles digestifs prolongés, ECG en cas de facteur de risque. Contrôle du respect des plafonds de posologie : 100 mg par jour chez l'adulte, 50 mg par jour au-delà de 65 ans, 2 mg/kg par jour chez l'enfant. Chez le sujet âgé, surveillance de la vigilance, de la confusion, du transit et de la miction ; l'hydroxyzine figure parmi les médicaments à éviter dans cette population. Réévaluation régulière de la nécessité du traitement.",
        iup: "Ce médicament est très sédatif : prenez la dose la plus importante le soir, et sachez que la somnolence peut persister le lendemain matin. L'effet est rapide, en une heure environ, contrairement aux traitements de fond de l'anxiété ; il est prévu pour des périodes courtes et ne crée pas de dépendance, mais il ne doit pas devenir un réflexe quotidien. Ne dépassez jamais la dose prescrite, quatre comprimés de 25 mg par jour au maximum chez l'adulte et deux après 65 ans : au-delà, le médicament peut perturber le rythme du cœur. Pas d'alcool, et pas de conduite ni de machine dangereuse tant que vous vous sentez ralenti, y compris le lendemain matin. Attendez-vous à une bouche sèche et parfois à une vue trouble ou à de la constipation, surtout au début. Signalez à votre médecin ou revenez le jour même en cas de palpitations, de malaise ou de perte de connaissance, de difficulté à uriner, d'œil rouge et douloureux avec vision floue, de confusion chez une personne âgée, ou d'éruption cutanée étendue avec fièvre.",
        half_life: "≈ 14 heures chez l'adulte, ≈ 20 heures chez le sujet âgé",
        elimination: "Métabolisme hépatique important, notamment par l'alcool déshydrogénase et le CYP3A4, avec formation de cétirizine, métabolite actif antihistaminique non sédatif ; élimination urinaire sous forme de métabolites.",
        renal: "Insuffisance rénale modérée à sévère : réduire la posologie quotidienne, en pratique de moitié, du fait de l'accumulation de la cétirizine, éliminée par voie rénale. Insuffisance rénale légère : pas d'adaptation nécessaire.",
        pregnancy: "Contre-indiqué pendant la grossesse selon le résumé des caractéristiques du produit, notamment en fin de grossesse en raison des effets atropiniques et sédatifs chez le nouveau-né ; l'allaitement est contre-indiqué, l'hydroxyzine et son métabolite passant dans le lait et exposant le nourrisson à une sédation.",
        sources: "RCP Atarax — base de données publique des médicaments (ANSM)\nANSM — hydroxyzine, restriction des conditions d'utilisation et risque d'allongement de l'intervalle QT\nHAS — prescription médicamenteuse chez le sujet âgé, médicaments à éviter",
        status: "",
        smr: "",
        tags: "antihistaminique h1 sédatif, contre-indiqué grossesse, vigilance conduite",
        toxicity: "",
    },
    StarterDetail {
        name: "Seroplex",
        indications: "Épisodes dépressifs majeurs ; trouble panique avec ou sans agoraphobie ; trouble d'anxiété sociale ; trouble anxieux généralisé ; troubles obsessionnels compulsifs.",
        mechanism: "Escitalopram, énantiomère S du citalopram, inhibiteur sélectif de la recapture de la sérotonine : le blocage du transporteur présynaptique SERT augmente la concentration de sérotonine dans la fente synaptique. La désensibilisation progressive des autorécepteurs 5-HT1A explique le délai de deux à quatre semaines avant l'effet thymique. C'est le plus sélectif des ISRS, avec une affinité négligeable pour les récepteurs adrénergiques, histaminiques et cholinergiques.",
        dosage: "10 mg par jour en une prise, unique et à heure fixe, augmentable à 20 mg par jour après au moins une semaine selon la réponse. Dans le trouble panique, débuter à 5 mg par jour la première semaine pour limiter l'aggravation anxieuse initiale, puis 10 mg, exceptionnellement 20 mg. Chez le sujet de plus de 65 ans, la moitié de la dose adulte est recommandée et la posologie ne doit pas dépasser 10 mg par jour, en raison du risque d'allongement du QT et d'hyponatrémie. Le traitement d'un épisode dépressif se poursuit au moins six mois après la rémission ; l'arrêt se fait par diminution progressive sur une à deux semaines au minimum.",
        contraindications: "Hypersensibilité à l'escitalopram, association aux IMAO non sélectifs ou sélectifs (sélégiline, linézolide, bleu de méthylène) et pendant les quatorze jours qui suivent leur arrêt, association au pimozide, allongement congénital ou acquis de l'intervalle QT, association à d'autres médicaments allongeant le QT.",
        ddi: "Association contre-indiquée aux IMAO : risque de syndrome sérotoninergique potentiellement mortel, un délai de quatorze jours est nécessaire à l'arrêt de l'IMAO et de sept jours à l'arrêt de l'escitalopram. Risque sérotoninergique également avec les triptans, le tramadol, la péthidine, le linézolide, le millepertuis et les autres antidépresseurs sérotoninergiques. Association déconseillée aux médicaments allongeant le QT : antiarythmiques de classe IA et III, dompéridone, hydroxyzine, citalopram, halopéridol, macrolides, fluoroquinolones, antifongiques azolés. Les inhibiteurs puissants du CYP2C19 (oméprazole, fluconazole, fluvoxamine) augmentent l'exposition. Majoration du risque hémorragique avec les AINS, l'aspirine, les antiagrégants et les anticoagulants oraux.",
        adverse: "Fréquents en début de traitement et souvent transitoires : nausées, céphalées, diarrhée, sécheresse buccale, sueurs, insomnie ou somnolence, asthénie, nervosité. Dysfonction sexuelle très fréquente et persistante (baisse de la libido, retard à l'éjaculation, anorgasmie). Prise de poids modérée. Plus rarement mais graves : hyponatrémie par SIADH surtout chez le sujet âgé, allongement du QT et torsades de pointes, syndrome sérotoninergique (agitation, confusion, myoclonies, tremblements, hyperthermie, diarrhée), saignements cutanéo-muqueux et digestifs, majoration des idées suicidaires dans les premières semaines chez le sujet jeune.",
        monitoring: "Surveillance rapprochée du risque suicidaire et d'un virage maniaque durant les quatre à six premières semaines et à chaque changement de dose, particulièrement avant 25 ans : consultation à une puis deux semaines. Natrémie avant traitement puis à deux et quatre semaines chez le sujet âgé, sous diurétique ou en cas de somnolence, confusion ou chutes. ECG avant l'instauration en cas de cardiopathie, de bradycardie ou d'association à un autre médicament allongeant le QT, avec kaliémie et magnésémie. Réévaluer l'indication et la tolérance à chaque renouvellement.",
        iup: "Un comprimé par jour, toujours au même moment, matin ou soir selon que le médicament vous réveille ou vous endort, avec ou sans aliments. L'effet sur l'humeur n'apparaît qu'au bout de deux à quatre semaines : les premiers jours peuvent même être marqués par un peu plus d'anxiété, de nausées ou d'insomnie, ce qui est habituel et transitoire, mais ne doit pas vous faire arrêter seul. N'interrompez jamais brutalement, même si vous vous sentez mieux : un arrêt sec provoque vertiges, sensations de décharges électriques, nausées et irritabilité, et la diminution doit toujours être progressive et décidée avec le médecin. Prudence à la conduite les premiers jours, le temps de savoir comment vous réagissez, et évitez l'alcool qui majore la somnolence. Revenez ou appelez le jour même en cas d'idées noires ou d'agitation inhabituelle, de fièvre avec tremblements, agitation et diarrhée, de confusion ou de malaise chez une personne âgée, ou de saignements anormaux. Signalez ce traitement à tout médecin, notamment avant la prescription d'un anti-inflammatoire ou d'un antibiotique.",
        half_life: "≈ 30 heures",
        elimination: "Métabolisme hépatique principalement par le CYP2C19, accessoirement CYP3A4 et CYP2D6 ; élimination des métabolites par voie rénale. Inhibiteur faible du CYP2D6.",
        renal: "Pas d'adaptation si la clairance est supérieure à 30 mL/min. En dessous de 30 mL/min, prudence et titration lente, les données étant limitées.",
        pregnancy: "Utilisation possible pendant la grossesse si elle est nécessaire, en prévenant l'équipe obstétricale d'un syndrome d'adaptation néonatale et d'un risque faible d'hypertension artérielle pulmonaire persistante du nouveau-né en cas d'exposition tardive ; l'allaitement est possible sous surveillance du nourrisson, le passage dans le lait étant faible.",
        sources: "RCP Seroplex — base de données publique des médicaments (ANSM)\nHAS — épisode dépressif caractérisé de l'adulte, prise en charge en premier recours\nANSM — bon usage des médicaments allongeant l'intervalle QT",
        status: "",
        smr: "",
        tags: "isrs, surveillance biologique, vigilance conduite",
        toxicity: "",
    },
    StarterDetail {
        name: "Zoloft",
        indications: "Épisodes dépressifs majeurs et prévention de leurs récidives ; troubles obsessionnels compulsifs de l'adulte et de l'enfant à partir de six ans ; trouble panique avec ou sans agoraphobie ; état de stress post-traumatique ; phobie sociale.",
        mechanism: "Sertraline, inhibiteur sélectif et puissant de la recapture de la sérotonine par blocage du transporteur SERT, avec une faible action inhibitrice sur le transporteur de la dopamine et pratiquement aucune affinité pour les récepteurs muscariniques, histaminiques ou alpha-adrénergiques. L'élévation de la sérotonine synaptique, puis la désensibilisation des autorécepteurs, sous-tendent l'effet antidépresseur et anti-obsessionnel. C'est l'ISRS le mieux documenté chez le patient coronarien et pendant l'allaitement.",
        dosage: "Dépression et TOC : 50 mg par jour en une prise, matin ou soir. Trouble panique, état de stress post-traumatique et phobie sociale : débuter à 25 mg par jour pendant une semaine pour limiter l'aggravation anxieuse, puis 50 mg. En cas de réponse insuffisante, augmentation par paliers de 50 mg à intervalle d'au moins une semaine, sans dépasser 200 mg par jour. Chez le sujet âgé, la posologie usuelle de l'adulte s'applique mais la titration doit être plus lente et la surveillance de la natrémie renforcée. Poursuivre au moins six mois après la rémission d'un épisode dépressif, plus longtemps dans le TOC, et arrêter par décroissance progressive.",
        contraindications: "Hypersensibilité à la sertraline, association aux IMAO non sélectifs ou sélectifs et dans les quatorze jours suivant leur arrêt, association au pimozide, insuffisance hépatique sévère.",
        ddi: "Association contre-indiquée aux IMAO, y compris le linézolide et le bleu de méthylène : syndrome sérotoninergique. Risque sérotoninergique avec les triptans, le tramadol, la péthidine, le fentanyl, le millepertuis, le lithium et les autres antidépresseurs. Majoration du risque hémorragique avec les AINS, l'aspirine, les antiagrégants et les anticoagulants oraux, y compris les AOD. Augmentation de l'INR sous AVK, contrôle rapproché à l'instauration et à l'arrêt. Prudence en association aux médicaments allongeant le QT et au pimozide, contre-indiqué. Inhibiteur modéré du CYP2D6 aux fortes doses, ce qui peut majorer l'exposition aux antidépresseurs tricycliques et à certains bêtabloquants.",
        adverse: "Fréquents et souvent régressifs : diarrhée et nausées, sécheresse buccale, céphalées, insomnie, somnolence, tremblements, sueurs, anorexie. Dysfonction sexuelle fréquente. Plus rarement mais graves : syndrome sérotoninergique, hyponatrémie par SIADH, saignements digestifs ou cutanéo-muqueux, virage maniaque, convulsions, majoration des idées suicidaires en début de traitement chez le sujet jeune, allongement du QT à forte dose.",
        monitoring: "Consultation de réévaluation à une puis deux semaines pour dépister l'aggravation des idées suicidaires, l'akathisie et le virage maniaque, en particulier avant 25 ans. Natrémie à deux et quatre semaines chez le sujet âgé, sous diurétique thiazidique ou en cas de confusion, chutes ou somnolence. Poids et efficacité réévalués à chaque renouvellement, INR rapproché en cas d'association à un AVK. Bilan hépatique en cas de signes d'appel.",
        iup: "Un comprimé par jour à heure fixe, de préférence pendant un repas pour limiter les nausées, le matin si le traitement vous rend insomniaque. Comptez deux à quatre semaines avant de ressentir l'effet sur l'humeur, davantage dans les troubles obsessionnels où l'amélioration peut demander deux à trois mois : ce délai est normal et n'est pas un signe d'inefficacité. Les premiers jours peuvent apporter des selles molles, des nausées ou un peu plus d'anxiété, qui passent en une à deux semaines. N'arrêtez jamais brutalement : la diminution doit être étalée sur plusieurs semaines avec votre médecin, sans quoi apparaissent vertiges, nausées et sensations de décharges électriques. Évitez l'alcool et soyez prudent au volant tant que vous ne connaissez pas votre tolérance. Contactez le médecin ou revenez le jour même en cas d'idées noires, d'agitation intense, de fièvre avec tremblements et diarrhée, de saignement digestif ou de confusion inhabituelle.",
        half_life: "≈ 26 heures ; métabolite déméthylé actif à demi-vie plus longue",
        elimination: "Métabolisme hépatique important avec effet de premier passage, impliquant surtout le CYP2B6 et à un moindre degré les CYP2C19, CYP2C9, CYP2D6 et CYP3A4 ; élimination des métabolites par voies urinaire et fécale à parts sensiblement égales.",
        renal: "Pas d'adaptation, quel que soit le degré d'insuffisance rénale : l'élimination du produit inchangé par le rein est négligeable. La dialyse ne modifie pas la posologie.",
        pregnancy: "Utilisable pendant la grossesse si nécessaire, en informant l'équipe obstétricale du risque de syndrome d'adaptation néonatale et d'hypertension artérielle pulmonaire persistante du nouveau-né en cas d'exposition au troisième trimestre ; c'est l'un des ISRS de choix pendant l'allaitement, le passage dans le lait étant très faible.",
        sources: "RCP Zoloft — base de données publique des médicaments (ANSM)\nHAS — épisode dépressif caractérisé de l'adulte, prise en charge en premier recours\nCRAT — antidépresseurs et grossesse",
        status: "",
        smr: "",
        tags: "isrs, surveillance biologique",
        toxicity: "",
    },
    StarterDetail {
        name: "Deroxat",
        indications: "Épisodes dépressifs majeurs ; troubles obsessionnels compulsifs ; trouble panique avec ou sans agoraphobie ; phobie sociale ; trouble anxieux généralisé ; état de stress post-traumatique.",
        mechanism: "Paroxétine, inhibiteur sélectif et le plus puissant des ISRS sur le transporteur de la sérotonine SERT. Elle possède en outre une affinité muscarinique non négligeable, à l'origine d'effets anticholinergiques absents des autres ISRS, et une demi-vie relativement courte sans métabolite actif, ce qui rend le syndrome d'arrêt particulièrement marqué. C'est également un inhibiteur puissant du CYP2D6, y compris de son propre métabolisme.",
        dosage: "Dépression, phobie sociale, trouble anxieux généralisé et état de stress post-traumatique : 20 mg par jour en une prise le matin, augmentation possible par paliers de 10 mg à intervalle d'au moins une semaine, sans dépasser 50 mg par jour. Trouble obsessionnel compulsif : débuter à 20 mg, dose recommandée 40 mg par jour, maximum 60 mg. Trouble panique : débuter à 10 mg par jour pour éviter l'aggravation initiale des attaques, dose recommandée 40 mg, maximum 60 mg. Chez le sujet âgé, débuter à 10 mg et ne pas dépasser 40 mg par jour. L'arrêt impose une décroissance très progressive, par paliers de 10 mg toutes une à deux semaines, parfois davantage.",
        contraindications: "Hypersensibilité à la paroxétine, association aux IMAO non sélectifs ou sélectifs et dans les deux semaines suivant leur arrêt, association au pimozide, association à la thioridazine.",
        ddi: "Association contre-indiquée aux IMAO, au linézolide et au bleu de méthylène : syndrome sérotoninergique. Inhibiteur puissant du CYP2D6 : l'association au tamoxifène est à éviter car la conversion en endoxifène actif est bloquée et l'efficacité antitumorale compromise ; majoration de l'exposition aux antidépresseurs tricycliques, à la flécaïnide, au métoprolol, au propafénone, à la risperidone et à l'halopéridol. Risque sérotoninergique avec les triptans, le tramadol, le fentanyl, le millepertuis, le lithium. Majoration du risque hémorragique avec les AINS, l'aspirine, les antiagrégants et les anticoagulants. Association déconseillée au pimozide et à la thioridazine du fait de l'allongement du QT.",
        adverse: "Fréquents : nausées, somnolence, sécheresse buccale, constipation, sueurs, prise de poids souvent supérieure à celle des autres ISRS, tremblements. Dysfonction sexuelle très fréquente. Effets anticholinergiques : constipation, rétention urinaire, troubles de l'accommodation. Syndrome d'arrêt fréquent et intense en cas d'interruption brutale, avec vertiges, sensations de décharges électriques, troubles du sommeil, anxiété et nausées. Plus rarement mais graves : syndrome sérotoninergique, hyponatrémie, hémorragies digestives, virage maniaque, akathisie, majoration des idées suicidaires en début de traitement chez le sujet jeune.",
        monitoring: "Surveillance rapprochée du risque suicidaire, de l'akathisie et du virage maniaque durant les premières semaines et à chaque changement de dose, en particulier avant 25 ans. Natrémie chez le sujet âgé ou sous diurétique, à l'instauration puis à quelques semaines. Poids et tolérance digestive à chaque renouvellement. Chez la femme traitée par tamoxifène, vérifier systématiquement l'antécédent oncologique avant délivrance.",
        iup: "Un comprimé par jour le matin, pendant le petit-déjeuner pour limiter les nausées, sans le croquer. L'effet sur l'humeur ou sur les obsessions demande deux à quatre semaines, parfois plus, alors que les effets indésirables apparaissent d'emblée : c'est le moment de tenir bon en gardant le contact avec le médecin. Ce médicament est celui dont l'arrêt brutal se fait le plus sentir, avec vertiges et sensations de décharges électriques dans la tête : ne sautez pas de prise et n'arrêtez jamais seul, la diminution se fait toujours par paliers étalés sur plusieurs semaines. Attention à la conduite les premiers jours en raison de la somnolence, et évitez l'alcool. Signalez ce traitement avant toute prescription d'anti-inflammatoire, et impérativement si vous prenez du tamoxifène. Revenez le jour même en cas d'idées noires, d'agitation, de fièvre avec tremblements et diarrhée, de confusion, ou de selles noires.",
        half_life: "≈ 24 heures, avec une cinétique non linéaire",
        elimination: "Métabolisme hépatique extensif par le CYP2D6, qu'elle sature et inhibe, avec relais partiel par le CYP3A4 ; métabolites inactifs éliminés par voies urinaire et fécale.",
        renal: "Clairance inférieure à 30 mL/min : débuter à 10 mg par jour et ne pas dépasser la fourchette basse des posologies. Au-dessus de 30 mL/min, posologie habituelle.",
        pregnancy: "Éviter en première intention au premier trimestre du fait d'un signal de malformations cardiaques discuté, préférer un autre ISRS chez une femme en projet de grossesse ; en cas d'exposition tardive, prévenir de la possibilité d'un syndrome d'adaptation néonatale ; l'allaitement est possible avec surveillance du nourrisson.",
        sources: "RCP Deroxat — base de données publique des médicaments (ANSM)\nHAS — épisode dépressif caractérisé de l'adulte, prise en charge en premier recours\nCRAT — antidépresseurs et grossesse",
        status: "",
        smr: "",
        tags: "isrs, vigilance conduite",
        toxicity: "",
    },
    StarterDetail {
        name: "Effexor",
        indications: "Épisodes dépressifs majeurs et prévention de leurs récidives ; trouble anxieux généralisé ; trouble panique avec ou sans agoraphobie ; phobie sociale, pour les formes à libération prolongée.",
        mechanism: "Venlafaxine, inhibiteur de la recapture de la sérotonine et de la noradrénaline : l'inhibition du transporteur SERT prédomine aux faibles doses, celle du transporteur noradrénergique NET n'apparaissant qu'à partir de 150 mg par jour, ce qui rend l'effet dose-dépendant. Son métabolite actif, la O-desméthylvenlafaxine, contribue majoritairement à l'activité. L'absence d'affinité pour les récepteurs muscariniques et histaminiques la distingue des tricycliques, mais la composante noradrénergique explique l'élévation tensionnelle aux fortes posologies.",
        dosage: "Forme à libération prolongée : 75 mg par jour en une prise au cours d'un repas ; en cas de réponse insuffisante, augmentation par paliers de 75 mg à intervalle d'au moins deux semaines, sans dépasser 225 mg par jour en ambulatoire. Les posologies supérieures, jusqu'à 375 mg par jour avec les formes à libération immédiate en plusieurs prises, sont réservées aux dépressions sévères en milieu hospitalier. Chez le sujet âgé, pas de dose spécifique mais prudence, titration lente et posologie efficace la plus faible du fait du risque d'hyponatrémie et d'hypotension orthostatique. L'arrêt exige une décroissance sur au moins deux semaines, souvent davantage après un traitement prolongé.",
        contraindications: "Hypersensibilité à la venlafaxine, association aux IMAO non sélectifs ou sélectifs et dans les quatorze jours suivant leur arrêt, hypertension artérielle non contrôlée, situation à risque élevé de trouble du rythme ventriculaire.",
        ddi: "Association contre-indiquée aux IMAO, au linézolide et au bleu de méthylène : syndrome sérotoninergique parfois mortel. Risque sérotoninergique avec les triptans, le tramadol, la péthidine, le fentanyl, le millepertuis, le lithium et les autres antidépresseurs. Majoration du risque hémorragique avec les AINS, l'aspirine, les antiagrégants et les anticoagulants oraux. Prudence avec les médicaments allongeant le QT et avec les sympathomimétiques ou autres médicaments hypertenseurs. Les inhibiteurs puissants du CYP2D6 et du CYP3A4 augmentent l'exposition à la venlafaxine.",
        adverse: "Très fréquents : nausées surtout à l'instauration, sécheresse buccale, céphalées, sueurs, insomnie, sensations vertigineuses, constipation. Dysfonction sexuelle fréquente. Élévation dose-dépendante de la pression artérielle et de la fréquence cardiaque au-delà de 150 mg par jour, hypotension orthostatique chez le sujet âgé. Syndrome d'arrêt marqué. Plus rarement mais graves : syndrome sérotoninergique, hyponatrémie par SIADH, hémorragies, virage maniaque, convulsions, allongement du QT, majoration des idées suicidaires en début de traitement chez le sujet jeune.",
        monitoring: "Pression artérielle avant l'instauration puis à chaque augmentation de dose et régulièrement ensuite : une hypertension confirmée impose de réduire la dose ou d'arrêter. Surveillance du risque suicidaire, de l'akathisie et du virage maniaque durant les premières semaines, surtout avant 25 ans. Natrémie chez le sujet âgé, sous diurétique, ou devant une somnolence, une confusion ou des chutes. Fréquence cardiaque, poids et cholestérolémie sous traitement prolongé à forte dose.",
        iup: "Une gélule à libération prolongée par jour, à avaler entière sans l'ouvrir ni la croquer, au cours d'un repas et toujours à la même heure. L'effet sur l'humeur ou sur l'anxiété demande deux à quatre semaines, alors que les nausées des premiers jours s'estompent en une semaine environ. Ce traitement peut faire monter la tension : faites contrôler votre pression artérielle après chaque augmentation de dose, et signalez maux de tête inhabituels ou bourdonnements d'oreille. N'arrêtez jamais brutalement ni ne sautez de prise : le syndrome d'arrêt de la venlafaxine est particulièrement pénible, avec vertiges, sensations de décharges électriques, nausées et irritabilité, et la diminution doit être lente et encadrée. Prudence au volant tant que vous ne connaissez pas votre tolérance, et évitez l'alcool. Revenez ou appelez le jour même en cas d'idées noires, d'agitation majeure, de fièvre avec tremblements et diarrhée, de confusion, ou de saignement anormal.",
        half_life: "≈ 5 heures pour la venlafaxine, ≈ 11 heures pour son métabolite actif",
        elimination: "Métabolisme hépatique par le CYP2D6 en O-desméthylvenlafaxine active, accessoirement par le CYP3A4 ; élimination essentiellement rénale, sous forme de métabolites et pour une faible part sous forme inchangée.",
        renal: "Clairance 30 à 50 mL/min : prudence, réduction de la dose à envisager. Clairance inférieure à 30 mL/min ou hémodialyse : réduire la posologie quotidienne d'environ la moitié et administrer la dose après la séance de dialyse.",
        pregnancy: "Utilisation possible pendant la grossesse si elle est nécessaire, en prévenant l'équipe obstétricale d'un syndrome d'adaptation néonatale en cas d'exposition tardive ; l'allaitement est envisageable sous surveillance du nourrisson, le passage lacté étant faible mais non négligeable.",
        sources: "RCP Effexor LP — base de données publique des médicaments (ANSM)\nHAS — épisode dépressif caractérisé de l'adulte, prise en charge en premier recours\nCRAT — antidépresseurs et grossesse",
        status: "",
        smr: "",
        tags: "irsna",
        toxicity: "",
    },
    StarterDetail {
        name: "Cymbalta",
        indications: "Épisodes dépressifs majeurs ; douleur neuropathique périphérique diabétique de l'adulte ; trouble anxieux généralisé.",
        mechanism: "Duloxétine, inhibiteur équilibré de la recapture de la sérotonine et de la noradrénaline, sans affinité significative pour les récepteurs muscariniques, histaminiques, dopaminergiques ou adrénergiques. Le renforcement simultané des voies sérotoninergiques et noradrénergiques descendantes de la corne postérieure de la moelle explique l'effet antalgique propre dans la douleur neuropathique, indépendant de l'effet thymique. La composante noradrénergique rend compte de la tachycardie, de la sécheresse buccale et de l'élévation tensionnelle.",
        dosage: "Dépression et douleur neuropathique diabétique : 60 mg par jour en une prise, avec ou sans aliments. Trouble anxieux généralisé : débuter à 30 mg par jour pendant une à deux semaines, puis 60 mg. En cas de réponse insuffisante, la posologie peut être portée à 90 puis 120 mg par jour, sans bénéfice démontré au-delà de 60 mg dans la plupart des situations. Chez le sujet âgé, pas d'adaptation systématique mais titration prudente à partir de 30 mg et surveillance de la natrémie. Dans la douleur neuropathique, réévaluer l'efficacité après deux mois ; l'arrêt se fait par diminution progressive sur au moins deux semaines.",
        contraindications: "Hypersensibilité à la duloxétine, association aux IMAO non sélectifs ou sélectifs et dans les quatorze jours suivant leur arrêt, insuffisance hépatique ou hépatopathie évolutive, insuffisance rénale sévère avec clairance inférieure à 30 mL/min, hypertension artérielle non contrôlée, association aux inhibiteurs puissants du CYP1A2 (fluvoxamine, ciprofloxacine, énoxacine).",
        ddi: "Association contre-indiquée aux IMAO, au linézolide et au bleu de méthylène : syndrome sérotoninergique. Association contre-indiquée aux inhibiteurs puissants du CYP1A2, qui multiplient l'exposition. Le tabac induit le CYP1A2 et abaisse les concentrations, ce dont il faut tenir compte à l'arrêt du tabac. Risque sérotoninergique avec les triptans, le tramadol, le millepertuis, le lithium et les autres antidépresseurs. Majoration du risque hémorragique avec les AINS, l'aspirine, les antiagrégants et les anticoagulants. Inhibiteur modéré du CYP2D6 : prudence avec les tricycliques, la flécaïnide et le métoprolol.",
        adverse: "Très fréquents : nausées à l'instauration, sécheresse buccale, céphalées, somnolence ou insomnie, constipation, diminution de l'appétit, sueurs, sensations vertigineuses. Dysfonction sexuelle fréquente. Élévation modérée de la pression artérielle et de la fréquence cardiaque. Plus rarement mais graves : atteinte hépatique cytolytique parfois sévère, hyponatrémie par SIADH, syndrome sérotoninergique, hémorragies, virage maniaque, convulsions, rétention urinaire, majoration des idées suicidaires en début de traitement chez le sujet jeune.",
        monitoring: "Transaminases avant l'instauration en cas de facteur de risque hépatique, puis devant tout signe d'appel (ictère, urines foncées, douleur de l'hypochondre droit, nausées persistantes). Pression artérielle et fréquence cardiaque à l'instauration puis régulièrement. Surveillance du risque suicidaire et du virage maniaque les premières semaines, surtout avant 25 ans. Natrémie chez le sujet âgé ou sous diurétique. Glycémie chez le patient diabétique traité pour une neuropathie, un déséquilibre modéré ayant été décrit.",
        iup: "Une gélule par jour à heure fixe, à avaler entière sans l'ouvrir ni la mâcher, avec ou sans aliments. Sur la douleur des pieds ou des jambes, l'effet met en général deux à quatre semaines à s'installer, et il faut deux mois pour juger vraiment du bénéfice ; sur l'humeur ou l'anxiété, le délai est du même ordre. Les nausées et la bouche sèche des premiers jours s'atténuent le plus souvent en une à deux semaines. N'arrêtez jamais brutalement, la diminution doit être progressive sur au moins deux semaines sous contrôle médical, sinon apparaissent vertiges, nausées et sensations de décharges électriques. Évitez l'alcool, qui majore à la fois la somnolence et le risque pour le foie, et soyez prudent au volant au début. Revenez le jour même si vos yeux ou votre peau jaunissent, si vos urines deviennent très foncées, en cas d'idées noires, de fièvre avec tremblements et diarrhée, ou de confusion.",
        half_life: "≈ 12 heures",
        elimination: "Métabolisme hépatique extensif par les CYP1A2 et CYP2D6 ; les métabolites inactifs sont éliminés majoritairement par voie urinaire, moins de 1 % étant excrété sous forme inchangée.",
        renal: "Clairance supérieure à 30 mL/min : pas d'adaptation. Clairance inférieure à 30 mL/min : contre-indiqué, les métabolites s'accumulant.",
        pregnancy: "Éviter pendant la grossesse en l'absence d'alternative, les données restant plus limitées que pour les ISRS, et prévenir l'équipe obstétricale d'un possible syndrome d'adaptation néonatale en cas d'exposition tardive ; l'allaitement est déconseillé, d'autres molécules mieux documentées lui étant préférées.",
        sources: "RCP Cymbalta — base de données publique des médicaments (ANSM)\nHAS — épisode dépressif caractérisé de l'adulte, prise en charge en premier recours\nCRAT — antidépresseurs et grossesse",
        status: "",
        smr: "",
        tags: "irsna",
        toxicity: "",
    },
    StarterDetail {
        name: "Laroxyl",
        indications: "Épisodes dépressifs majeurs, en particulier les formes sévères ou avec insomnie ; douleurs neuropathiques de l'adulte ; algies rebelles ; énurésie nocturne de l'enfant après exclusion d'une cause organique. La solution buvable en gouttes permet une titration fine, très utilisée en antalgie.",
        mechanism: "Amitriptyline, antidépresseur tricyclique inhibant de façon non sélective la recapture de la sérotonine et de la noradrénaline. Elle bloque en outre les récepteurs muscariniques, histaminiques H1 et alpha-1 adrénergiques, ce qui explique la sécheresse buccale, la sédation et l'hypotension orthostatique, et exerce un effet stabilisant de membrane par blocage des canaux sodiques, responsable de la cardiotoxicité en surdosage. L'effet antalgique, obtenu à doses bien inférieures aux doses antidépressives, passe par le renforcement des contrôles inhibiteurs descendants.",
        dosage: "Douleur neuropathique : débuter à 5 à 10 mg le soir deux heures avant le coucher, en gouttes, avec augmentation par paliers de 5 à 10 mg tous les trois à sept jours selon la tolérance ; doses efficaces habituelles de 25 à 75 mg par jour, rarement au-delà. Dépression : débuter à 25 mg par jour et augmenter progressivement jusqu'à 75 à 150 mg par jour, la posologie la plus élevée relevant du milieu spécialisé. Chez le sujet âgé, commencer à la moitié de la dose adulte, soit 5 mg le soir en antalgie, et augmenter très lentement en raison du risque de confusion, de rétention urinaire et de chute. L'arrêt se fait toujours de façon progressive sur plusieurs semaines.",
        contraindications: "Hypersensibilité, infarctus du myocarde récent, troubles du rythme et de la conduction, insuffisance cardiaque décompensée, glaucome à angle fermé, risque de rétention urinaire par obstacle urétro-prostatique, association aux IMAO non sélectifs et dans les deux semaines suivant leur arrêt, insuffisance hépatique sévère, enfant de moins de six ans.",
        ddi: "Association contre-indiquée aux IMAO non sélectifs : syndrome sérotoninergique et poussée hypertensive. Association déconseillée aux médicaments allongeant le QT (antiarythmiques de classe IA et III, hydroxyzine, dompéridone, macrolides, halopéridol) : risque de torsades de pointes. Effets anticholinergiques additifs avec les autres atropiniques (antihistaminiques sédatifs, antiparkinsoniens anticholinergiques, oxybutynine, néfopam), avec risque de confusion, de rétention urinaire et de constipation sévère chez le sujet âgé. Sédation additive avec l'alcool, les benzodiazépines et les opioïdes. Les inhibiteurs du CYP2D6 (paroxétine, fluoxétine, bupropion) augmentent fortement les concentrations. Risque sérotoninergique avec le tramadol, qui abaisse aussi le seuil épileptogène.",
        adverse: "Fréquents et souvent limitants : sécheresse buccale, constipation, somnolence, prise de poids, hypotension orthostatique, troubles de l'accommodation, sueurs, tremblements, tachycardie. Rétention urinaire et confusion, surtout chez le sujet âgé. Plus rarement mais graves : troubles de la conduction et du rythme cardiaque, allongement du QT, convulsions, syndrome sérotoninergique, hyponatrémie, glaucome aigu par fermeture de l'angle, hépatite, agranulocytose, majoration des idées suicidaires en début de traitement chez le sujet jeune. Cardiotoxicité majeure en cas d'intoxication volontaire, ce qui limite les quantités délivrées chez le patient à risque suicidaire.",
        monitoring: "ECG avant l'instauration au-delà de 50 ans, en cas de cardiopathie ou de posologie antidépressive, puis en cas d'augmentation de dose. Pression artérielle couchée et debout à la recherche d'une hypotension orthostatique, surtout chez le sujet âgé. Surveillance de la constipation, du résidu post-mictionnel chez l'homme prostatique, et de l'état cognitif. Risque suicidaire et virage maniaque les premières semaines. Natrémie chez le sujet âgé ou sous diurétique. Ionogramme et NFS en cas de signes d'appel.",
        iup: "En gouttes, comptez exactement le nombre prescrit dans un peu d'eau, le soir deux heures avant le coucher : pris trop tard, le médicament laisse une somnolence au réveil. Contre la douleur, la dose est très inférieure à la dose antidépressive et l'effet met deux à quatre semaines à s'installer, souvent après plusieurs augmentations : ce médicament n'est pas un antalgique à prendre à la demande, il se prend tous les jours. Attendez-vous à une bouche sèche, à de la constipation et à une somnolence au début, qui s'atténuent en partie ; buvez suffisamment, prévoyez des fibres, et levez-vous en deux temps pour éviter le vertige en vous mettant debout. N'arrêtez jamais d'un coup, la diminution se fait par paliers sur plusieurs semaines avec le médecin. La conduite est déconseillée tant que la somnolence persiste, et l'alcool est à éviter complètement. Revenez le jour même en cas d'impossibilité d'uriner, d'œil rouge et douloureux avec vision floue, de palpitations ou de malaise, de confusion, ou d'idées noires.",
        half_life: "≈ 10 à 28 heures, plus longue chez le sujet âgé",
        elimination: "Métabolisme hépatique par le CYP2D6, avec production de nortriptyline active, et par les CYP2C19, CYP3A4 et CYP1A2 ; élimination principalement urinaire sous forme de métabolites.",
        renal: "Pas d'adaptation formelle en insuffisance rénale légère à modérée ; en insuffisance rénale sévère, prudence, titration lente et posologie minimale efficace, les métabolites pouvant s'accumuler.",
        pregnancy: "Utilisable pendant la grossesse si nécessaire, l'amitriptyline étant l'un des tricycliques les mieux documentés, en signalant à l'équipe obstétricale une possible symptomatologie atropinique ou de sevrage chez le nouveau-né en cas d'exposition en fin de grossesse ; l'allaitement est possible sous surveillance de la somnolence du nourrisson.",
        sources: "RCP Laroxyl — base de données publique des médicaments (ANSM)\nHAS — prise en charge des douleurs neuropathiques chroniques\nCRAT — antidépresseurs et grossesse",
        status: "",
        smr: "",
        tags: "antidépresseur tricyclique, surveillance biologique, vigilance conduite",
        toxicity: "",
    },
    StarterDetail {
        name: "Téralithe",
        indications: "Prophylaxie des rechutes des troubles bipolaires et des troubles schizo-affectifs à composante thymique prédominante, traitement curatif des états d'excitation maniaque ou hypomaniaque, et prévention des récidives dépressives dans certaines formes de trouble bipolaire. Le lithium reste le thymorégulateur de référence, notamment pour son effet démontré sur le risque suicidaire.",
        mechanism: "Le lithium modifie la transduction du signal intracellulaire des neurones, en inhibant l'inositol monophosphatase et la glycogène synthase kinase 3, ce qui atténue les cascades du phosphatidylinositol et de l'AMP cyclique. Il en résulte une stabilisation de la neurotransmission monoaminergique et glutamatergique et un effet neuroprotecteur, sans effet sédatif propre. Sa marge thérapeutique est très étroite : la dose utile et la dose toxique sont voisines.",
        dosage: "La posologie est strictement individuelle et se règle sur la lithiémie, jamais sur le nombre de comprimés. Téralithe 250 mg, forme à libération immédiate, se prend en deux à trois prises quotidiennes, avec une lithiémie cible mesurée douze heures après la dernière prise habituellement comprise entre 0,5 et 0,8 mmol/L. Téralithe 400 mg LP se prend en une à deux prises, le plus souvent le soir, avec une lithiémie cible de douze heures habituellement comprise entre 0,8 et 1,2 mmol/L. Les doses sont diminuées chez le sujet âgé et en cas d'insuffisance rénale, et tout changement de forme galénique impose un nouveau dosage.",
        contraindications: "Insuffisance rénale, déplétion sodée et régime désodé, déshydratation, insuffisance cardiaque instable, coronaropathie sévère et troubles du rythme, hyponatrémie, allaitement ; association aux diurétiques, notamment thiazidiques, formellement déconseillée. La grossesse impose une réévaluation spécialisée du rapport bénéfice-risque.",
        ddi: "Les AINS, y compris en automédication, réduisent la clairance rénale du lithium et peuvent faire basculer en surdosage en quelques jours : association déconseillée, à refuser au comptoir. Les inhibiteurs de l'enzyme de conversion et les sartans, les diurétiques thiazidiques et les diurétiques de l'anse augmentent également la lithiémie. Le métronidazole et certains antibiotiques réduisent son élimination. L'association aux neuroleptiques, aux triptans, aux ISRS et aux IMAO expose à un syndrome sérotoninergique ou à une neurotoxicité même à lithiémie normale.",
        adverse: "Fréquemment tremblements fins des extrémités, soif intense, polyurie, prise de poids, nausées, diarrhée, acné et psoriasis, goût métallique. Au long cours : hypothyroïdie, goitre, hyperparathyroïdie avec hypercalcémie, diabète insipide néphrogénique et néphropathie tubulo-interstitielle. Le surdosage se manifeste par des tremblements amples, une ataxie, une dysarthrie, une somnolence, des vomissements et des diarrhées, et peut évoluer vers un coma convulsif : c'est une urgence.",
        monitoring: "Avant l'instauration : créatininémie et clairance, ionogramme, calcémie, TSH, hémogramme, ECG chez le sujet à risque, test de grossesse. Lithiémie cinq à sept jours après l'instauration ou après tout changement de dose, puis mensuelle les trois premiers mois, ensuite tous les trois à six mois si le traitement est stable, toujours douze heures après la dernière prise. Créatininémie, ionogramme, calcémie et TSH au moins deux fois par an. Toute fièvre, tout épisode de vomissements, de diarrhée, de canicule ou de régime sans sel justifie un dosage rapproché.",
        iup: "Le lithium doit être pris tous les jours, aux mêmes heures, sans jamais modifier la dose de vous-même : l'écart entre la dose efficace et la dose toxique est très faible. Buvez régulièrement, au moins un litre et demi à deux litres d'eau par jour, et ne supprimez jamais le sel de votre alimentation : un régime sans sel, une forte transpiration, une canicule, une diarrhée ou des vomissements font monter le lithium dans le sang et peuvent provoquer un surdosage. Ne prenez jamais d'anti-inflammatoire, ibuprofène compris, sans en parler d'abord : c'est l'une des causes les plus fréquentes d'intoxication. La prise de sang de contrôle se fait toujours douze heures après la dernière prise, donc le matin sans avoir pris le comprimé du matin, que vous prendrez juste après le prélèvement. Consultez en urgence si apparaissent des tremblements marqués, une démarche instable, des troubles de la parole, une somnolence inhabituelle, des vomissements ou des diarrhées importantes. Signalez tout projet de grossesse avant de l'engager, car le traitement doit être réévalué avec le psychiatre.",
        half_life: "18 à 36 heures, allongée chez le sujet âgé et l'insuffisant rénal",
        elimination: "Élimination exclusivement rénale sous forme inchangée ; le lithium est réabsorbé au tube proximal en compétition avec le sodium, ce qui explique que toute déplétion sodée augmente la lithiémie.",
        renal: "Contre-indiqué en cas d'insuffisance rénale. En cas d'altération modérée de la fonction rénale, l'usage relève d'une décision spécialisée avec réduction de dose et lithiémies rapprochées. Une baisse de la clairance sous traitement impose de réévaluer la poursuite.",
        pregnancy: "Grossesse déconseillée avec risque malformatif cardiaque au premier trimestre : la poursuite ne se discute qu'en milieu spécialisé, avec lithiémies rapprochées et échographie cardiaque fœtale ; allaitement contre-indiqué du fait du passage dans le lait.",
        sources: "RCP Téralithe — base de données publique des médicaments (ANSM)\nHAS — troubles bipolaires, repérage et prise en charge\nANSM — bon usage du lithium et surveillance de la lithiémie",
        status: "",
        smr: "",
        tags: "thymorégulateur, marge thérapeutique étroite, surveillance biologique, contre-indiqué grossesse",
        toxicity: "Marge thérapeutique étroite : un écart de dose ou une interaction suffit à faire basculer vers le sous-dosage ou la toxicité. Voir les sections Interactions et Surveillance.",
    },
    StarterDetail {
        name: "Dépakote",
        indications: "Traitement des épisodes maniaques du trouble bipolaire en cas de contre-indication ou d'intolérance au lithium ; la poursuite du traitement après l'épisode aigu peut être envisagée chez les patients ayant répondu au divalproate lors de cet épisode.",
        mechanism: "Divalproate de sodium, complexe stable de valproate de sodium et d'acide valproïque, libérant du valproate. Celui-ci augmente la transmission GABAergique en inhibant la GABA-transaminase et la succinate-semialdéhyde déshydrogénase, bloque les canaux sodiques voltage-dépendants et modifie l'expression génique par inhibition des histone-désacétylases. La combinaison de ces effets sous-tend l'activité antimaniaque et antiépileptique, ainsi que la tératogénicité.",
        dosage: "Posologie initiale recommandée de 750 mg par jour répartis en deux ou trois prises au cours des repas, avec augmentation rapide en quelques jours jusqu'à la dose minimale efficace ; la posologie usuelle se situe entre 1 000 et 2 000 mg par jour, la dose ne devant pas dépasser 60 mg/kg par jour. Les comprimés gastro-résistants doivent être avalés entiers, sans être écrasés ni mâchés. Chez le sujet âgé, débuter plus bas et augmenter plus lentement, la dose efficace étant en règle plus faible et la surveillance clinique déterminante. L'ajustement se fait sur la réponse clinique, la valprotémie servant de repère de zone thérapeutique et de contrôle d'observance plutôt que de cible.",
        contraindications: "Hypersensibilité, hépatite aiguë ou chronique, antécédent personnel ou familial d'hépatite sévère, porphyrie hépatique, troubles connus du cycle de l'urée, mutations de l'ADN polymérase gamma (POLG, syndrome d'Alpers-Huttenlocher) et suspicion chez l'enfant de moins de deux ans, grossesse, et femme en âge de procréer en l'absence des conditions du programme de prévention des grossesses.",
        ddi: "Inhibiteur enzymatique : il double environ les concentrations de lamotrigine, ce qui impose une titration de la lamotrigine deux fois plus lente et à demi-dose sous peine de toxidermie grave. Il augmente les concentrations de phénobarbital, de primidone, de zidovudine et la fraction libre de la phénytoïne. Les carbapénèmes (méropénème, imipénème, ertapénème) effondrent la valprotémie en quelques jours, avec risque de récidive ou de crise : association déconseillée. Les inducteurs enzymatiques (carbamazépine, phénytoïne, phénobarbital, rifampicine) abaissent les concentrations de valproate. Association aux autres hépatotoxiques et à l'alcool déconseillée ; majoration du risque hémorragique avec l'aspirine, les AINS et les anticoagulants du fait de la thrombopénie et de l'atteinte fonctionnelle plaquettaire.",
        adverse: "Fréquents : nausées et douleurs épigastriques à l'instauration, prise de poids parfois importante, tremblement fin des extrémités, somnolence, alopécie souvent réversible avec repousse modifiée, troubles menstruels et syndrome des ovaires polykystiques. Élévation isolée et fréquente des transaminases et de l'ammoniémie. Thrombopénie dose-dépendante. Plus rarement mais graves : hépatite fulminante surtout dans les six premiers mois et chez l'enfant de moins de trois ans, pancréatite aiguë hémorragique, encéphalopathie hyperammoniémique avec confusion et troubles de la vigilance, atteinte hématologique avec pancytopénie, syndrome de Lyell et DRESS. Chez l'enfant exposé in utero, malformations dans environ 10 % des cas et troubles neurodéveloppementaux chez 30 à 40 % des enfants.",
        monitoring: "Bilan hépatique complet et NFS avec plaquettes et bilan d'hémostase avant l'instauration, puis contrôle du bilan hépatique de façon rapprochée pendant les six premiers mois, période où le risque d'hépatite est maximal. Amylasémie et lipasémie devant toute douleur abdominale intense. Ammoniémie en cas de somnolence, de confusion ou d'aggravation inexpliquée. Valprotémie utile pour vérifier l'observance, ajuster en cas d'inefficacité ou de signes de surdosage, et lors de l'introduction d'un inducteur ou d'un carbapénème. Poids, périmètre abdominal et régularité des cycles à chaque consultation. Chez toute femme en âge de procréer, vérification annuelle de l'accord de soins et du recours à une contraception efficace.",
        iup: "Avalez les comprimés entiers au cours des repas, sans les écraser ni les couper, ce qui protège l'estomac et permet la libération prévue. L'effet sur l'épisode maniaque s'installe en quelques jours à une à deux semaines, plus vite que celui d'un antidépresseur, mais le traitement se prend tous les jours sans exception, y compris quand tout va bien. N'arrêtez jamais ce médicament brutalement, y compris si vous découvrez une grossesse : appelez immédiatement votre médecin, car l'arrêt sec expose à une rechute sévère et l'adaptation doit être décidée avec lui. Si vous êtes une femme en âge d'avoir des enfants, ce traitement impose une contraception efficace et un accord de soins signé chaque année, parce qu'il provoque des malformations et des troubles du développement chez l'enfant à naître dans une proportion importante des grossesses exposées. Évitez l'alcool, prudence au volant tant que la somnolence ou le tremblement persistent, et attendez-vous à un possible appétit augmenté qu'il vaut mieux anticiper. Revenez ou appelez le jour même en cas de fatigue intense avec nausées et perte d'appétit, de jaunisse, de douleur abdominale violente, de bleus ou de saignements inhabituels, de confusion ou de somnolence anormale, ou d'éruption cutanée étendue avec fièvre.",
        half_life: "≈ 15 à 17 heures chez l'adulte, raccourcie en association à un inducteur enzymatique",
        elimination: "Métabolisme hépatique prédominant par glucuroconjugaison et bêta-oxydation mitochondriale, avec une part mineure d'oxydation par les CYP2C9, CYP2C19 et CYP2A6 ; élimination urinaire sous forme de métabolites, moins de 5 % sous forme inchangée.",
        renal: "Pas d'adaptation formelle en insuffisance rénale, mais l'hypoalbuminémie et la baisse de liaison protéique augmentent la fraction libre : interpréter la valprotémie totale avec prudence et ajuster sur la clinique, en réduisant si nécessaire la posologie.",
        pregnancy: "Contre-indiqué pendant la grossesse dans le trouble bipolaire, et contre-indiqué chez la femme en âge de procréer sauf respect strict du programme de prévention des grossesses : contraception efficace continue, information et accord de soins annuel cosignés, test de grossesse avant l'instauration et pendant le suivi, prescription initiale annuelle réservée au spécialiste ; l'allaitement est possible mais doit être discuté au cas par cas, le passage lacté étant faible.",
        sources: "RCP Dépakote — base de données publique des médicaments (ANSM)\nANSM — programme de prévention des grossesses sous valproate et dérivés, conditions de prescription et de délivrance\nHAS — trouble bipolaire, repérage et prise en charge",
        status: "Prescription encadrée chez la femme en âge de procréer (accord de soins)",
        smr: "",
        tags: "thymorégulateur, marge thérapeutique étroite, surveillance biologique, contre-indiqué grossesse",
        toxicity: "Marge thérapeutique étroite : un écart de dose ou une interaction suffit à faire basculer vers le sous-dosage ou la toxicité. Voir les sections Interactions et Surveillance.",
    },
    StarterDetail {
        name: "Lamictal",
        indications: "Épilepsie de l'adulte et de l'enfant : en monothérapie ou en association dans les crises partielles et les crises généralisées tonicocloniques, ainsi que dans les crises associées au syndrome de Lennox-Gastaut. Prévention des épisodes dépressifs chez l'adulte présentant un trouble bipolaire de type I ayant une prédominance d'épisodes dépressifs.",
        mechanism: "Lamotrigine, antiépileptique bloquant de façon voltage-dépendante et usage-dépendante les canaux sodiques neuronaux, ce qui stabilise la membrane présynaptique. Il en résulte une réduction de la libération des acides aminés excitateurs, en particulier du glutamate. Cette action, associée à un effet sur certains canaux calciques, explique son profil antiépileptique large et son efficacité préférentielle sur le versant dépressif du trouble bipolaire.",
        dosage: "La titration est lente et non négociable, car la vitesse d'augmentation conditionne le risque de toxidermie grave. En monothérapie : 25 mg par jour pendant deux semaines, puis 50 mg par jour pendant deux semaines, puis augmentation par paliers de 50 à 100 mg toutes les une à deux semaines, dose d'entretien usuelle de 100 à 200 mg par jour. En association au valproate, qui double les concentrations : 25 mg un jour sur deux pendant deux semaines, puis 25 mg par jour pendant deux semaines, puis augmentation par paliers de 25 à 50 mg, entretien usuel de 100 à 200 mg par jour. En association à un inducteur enzymatique sans valproate : 50 mg par jour pendant deux semaines, puis 100 mg par jour en deux prises pendant deux semaines, puis augmentation jusqu'à 200 à 400 mg par jour. Toute interruption de plus de cinq jours impose de reprendre la titration depuis le début. Chez le sujet âgé, pas d'adaptation spécifique mais titration prudente.",
        contraindications: "Hypersensibilité à la lamotrigine. Il n'existe pas d'autre contre-indication absolue, mais un antécédent de toxidermie sous lamotrigine interdit toute réintroduction.",
        ddi: "Le valproate inhibe la glucuroconjugaison de la lamotrigine et double environ ses concentrations en allongeant sa demi-vie : la titration doit alors être deux fois plus lente et les doses de départ réduites de moitié, sous peine de syndrome de Stevens-Johnson ou de syndrome de Lyell. Les inducteurs enzymatiques (carbamazépine, phénytoïne, phénobarbital, primidone, rifampicine) réduisent de moitié environ les concentrations et imposent des doses plus élevées ; l'association à la carbamazépine majore par ailleurs les vertiges, la diplopie et l'ataxie. Les contraceptifs œstroprogestatifs abaissent nettement les concentrations de lamotrigine par induction de la glucuroconjugaison, avec risque de perte d'efficacité pendant les semaines de prise et de surdosage pendant la semaine d'arrêt : toute instauration ou tout arrêt de contraception impose un réajustement. L'atazanavir, le lopinavir et le ritonavir diminuent également les concentrations.",
        adverse: "Fréquents et le plus souvent transitoires : céphalées, éruption cutanée maculopapuleuse bénigne dans les huit premières semaines, somnolence, sensations vertigineuses, diplopie et vision floue surtout en association à la carbamazépine, ataxie, nausées, irritabilité, insomnie. Plus rarement mais graves : syndrome de Stevens-Johnson et syndrome de Lyell, dont le risque est maximal dans les huit premières semaines et majoré par une titration trop rapide, l'association au valproate et l'âge pédiatrique ; syndrome d'hypersensibilité médicamenteuse DRESS avec fièvre, adénopathies, atteinte hépatique et hyperéosinophilie ; lymphohistiocytose hémophagocytaire ; anomalies hématologiques ; aggravation paradoxale des crises ; méningite aseptique ; idées suicidaires.",
        monitoring: "Surveillance cutanée étroite pendant les huit premières semaines : tout exanthème impose un avis médical immédiat et, en cas d'atteinte muqueuse, de fièvre ou d'altération de l'état général, l'arrêt immédiat et définitif. NFS, bilan hépatique et créatininémie devant toute suspicion de DRESS ou de réaction systémique. Vérification à chaque délivrance que le plan de titration est respecté et qu'aucune interruption supérieure à cinq jours n'a eu lieu. Réévaluation des concentrations plasmatiques lors de l'ajout ou du retrait de valproate, d'un inducteur ou d'une contraception œstroprogestative, et pendant la grossesse. Repérage des idées suicidaires, décrit avec l'ensemble des antiépileptiques.",
        iup: "Respectez très exactement le plan de montée des doses, semaine par semaine : ce n'est pas une précaution formelle, c'est ce qui protège votre peau d'une réaction grave, et augmenter plus vite parce que l'effet tarde est dangereux. L'effet préventif sur l'humeur ou sur les crises ne s'installe qu'après plusieurs semaines, le temps d'atteindre la dose d'entretien. Surveillez votre peau pendant les deux premiers mois : toute éruption, même discrète, doit être montrée le jour même, et il faut consulter en urgence si elle s'accompagne de fièvre, de bulles, de lésions dans la bouche ou les yeux, ou d'un gonflement du visage. Si vous oubliez le traitement plusieurs jours de suite, ne reprenez pas à la dose habituelle : appelez le médecin, car la montée devra être recommencée depuis le début. Prévenez systématiquement si vous commencez ou arrêtez une pilule œstroprogestative, car elle modifie fortement les concentrations et la dose devra être revue. N'arrêtez jamais brutalement, l'arrêt se fait sur au moins deux semaines, et évitez l'alcool ; prudence au volant en cas de vision double ou de vertiges.",
        half_life: "≈ 25 à 35 heures en monothérapie ; environ 70 heures en association au valproate et 14 heures en association à un inducteur",
        elimination: "Métabolisme hépatique par glucuroconjugaison via les UDP-glucuronosyltransférases, sans passage significatif par les cytochromes ; élimination urinaire sous forme de conjugués inactifs.",
        renal: "Pas d'adaptation en insuffisance rénale légère à modérée. En insuffisance rénale sévère ou en dialyse, prudence et doses d'entretien réduites, le métabolite glucuroconjugué s'accumulant ; l'hémodialyse épure une part de la lamotrigine.",
        pregnancy: "C'est l'un des antiépileptiques les mieux tolérés pendant la grossesse et il peut être poursuivi si nécessaire, avec supplémentation en acide folique ; les concentrations chutent fortement au cours de la grossesse et remontent après l'accouchement, ce qui impose des dosages réguliers et un réajustement des doses ; l'allaitement est possible sous surveillance de la somnolence et de l'éruption cutanée du nourrisson.",
        sources: "RCP Lamictal — base de données publique des médicaments (ANSM)\nANSM — antiépileptiques et grossesse, information des patientes\nHAS — trouble bipolaire, repérage et prise en charge",
        status: "",
        smr: "",
        tags: "antiépileptique, marge thérapeutique étroite, surveillance biologique",
        toxicity: "Marge thérapeutique étroite : un écart de dose ou une interaction suffit à faire basculer vers le sous-dosage ou la toxicité. Voir les sections Interactions et Surveillance.",
    },
    StarterDetail {
        name: "Keppra",
        indications: "Monothérapie des crises partielles avec ou sans généralisation secondaire chez l'adulte et l'adolescent à partir de seize ans présentant une épilepsie nouvellement diagnostiquée ; en association, traitement des crises partielles de l'adulte et de l'enfant, des crises myocloniques de l'épilepsie myoclonique juvénile à partir de douze ans, et des crises généralisées tonicocloniques primaires de l'épilepsie généralisée idiopathique à partir de douze ans.",
        mechanism: "Lévétiracétam, dont le mécanisme est distinct de celui des antiépileptiques classiques : il se lie à la protéine SV2A de la vésicule synaptique, modulant l'exocytose et donc la libération des neurotransmetteurs lors des décharges à haute fréquence. Il n'agit ni sur les canaux sodiques, ni sur les récepteurs GABA-A, ce qui explique l'absence quasi complète d'interactions pharmacocinétiques. Son délai d'efficacité est court, la dose de départ étant déjà thérapeutique.",
        dosage: "En association et en monothérapie chez l'adulte, la dose de départ usuelle est de 500 mg deux fois par jour ; en monothérapie dans l'épilepsie nouvellement diagnostiquée, l'instauration peut se faire à 250 mg deux fois par jour pendant deux semaines avant de passer à 500 mg deux fois par jour. Augmentation ensuite par paliers de 500 mg deux fois par jour toutes les deux à quatre semaines selon la réponse, jusqu'à un maximum de 1 500 mg deux fois par jour, soit 3 000 mg par jour. Chez le sujet âgé, la posologie doit être adaptée à la clairance de la créatinine, qui baisse avec l'âge. L'arrêt se fait par diminution progressive, par paliers de 500 mg deux fois par jour toutes les deux à quatre semaines.",
        contraindications: "Hypersensibilité au lévétiracétam ou aux autres dérivés de la pyrrolidone. Il n'existe pas d'autre contre-indication absolue.",
        ddi: "Profil d'interactions remarquablement pauvre : le lévétiracétam n'est pas métabolisé par les cytochromes et n'induit ni n'inhibe les enzymes hépatiques, ce qui en fait une option de choix chez le patient polymédiqué, sous anticoagulant oral, sous chimiothérapie ou sous contraception hormonale, dont l'efficacité n'est pas modifiée. Les inducteurs enzymatiques puissants (carbamazépine, phénytoïne, rifampicine) peuvent abaisser modérément ses concentrations. L'alcool et les autres dépresseurs du système nerveux central majorent la sédation. Le méthotrexate à forte dose voit son élimination possiblement ralentie, avec surveillance nécessaire.",
        adverse: "Très fréquents : somnolence, asthénie, céphalées, sensations vertigineuses. Fréquents et caractéristiques de la molécule : troubles du comportement avec irritabilité, nervosité, agressivité, labilité émotionnelle, agitation, parfois hostilité franche, plus marqués chez l'enfant, l'adolescent et le patient ayant des antécédents psychiatriques. Dépression, anxiété, insomnie, idées suicidaires. Fréquents également : rhinopharyngite, anorexie, nausées, toux. Plus rarement mais graves : réactions cutanées sévères à type de syndrome de Stevens-Johnson, de syndrome de Lyell et de DRESS ; cytopénies, dont neutropénie et thrombopénie ; pancréatite ; atteinte hépatique.",
        monitoring: "Recherche systématique et répétée des troubles du comportement, de l'irritabilité et de l'humeur dépressive à l'instauration et à chaque augmentation, en interrogeant aussi l'entourage : c'est le motif d'arrêt le plus fréquent. Créatininémie et estimation de la clairance avant l'instauration puis régulièrement, la posologie en dépendant directement, en particulier chez le sujet âgé. NFS en cas d'infection à répétition, d'ecchymoses ou de fatigue inexpliquée. Surveillance cutanée les premiers mois. Il n'y a pas d'intérêt à un dosage plasmatique en routine.",
        iup: "Deux prises par jour à douze heures d'intervalle, avec ou sans aliments ; les comprimés s'avalent entiers avec un verre d'eau et il existe une solution buvable si la déglutition est difficile. À la différence de beaucoup d'autres traitements, celui-ci agit rapidement, dès les premiers jours à la dose de départ. La somnolence et la fatigue du début s'atténuent en général en une à deux semaines. Ce médicament peut rendre irritable, à fleur de peau, parfois franchement agressif ou triste : si vous ou vos proches remarquez un tel changement, signalez-le rapidement, car il est fréquent et il existe des solutions. N'arrêtez jamais brutalement, même une seule journée d'oubli répétée peut favoriser une crise, et la diminution éventuelle se fait toujours par paliers avec le neurologue. Prudence au volant tant que la somnolence persiste, limitez l'alcool, et revenez le jour même en cas d'idées noires, d'éruption cutanée étendue avec fièvre, ou de fièvre inexpliquée avec mal de gorge.",
        half_life: "≈ 7 heures chez l'adulte, allongée chez le sujet âgé et l'insuffisant rénal",
        elimination: "Faiblement métabolisé, par hydrolyse enzymatique du groupement acétamide dans le sang, sans intervention des cytochromes ; environ deux tiers de la dose sont éliminés par voie rénale sous forme inchangée par filtration glomérulaire.",
        renal: "L'adaptation est indispensable et se fait sur la clairance. Clairance 50 à 79 mL/min : 500 à 1 000 mg deux fois par jour. Clairance 30 à 49 mL/min : 250 à 750 mg deux fois par jour. Clairance inférieure à 30 mL/min : 250 à 500 mg deux fois par jour. Hémodialyse : dose quotidienne réduite avec une dose supplémentaire après chaque séance, le lévétiracétam étant largement épuré.",
        pregnancy: "C'est, avec la lamotrigine, l'antiépileptique le mieux documenté et le plus utilisable pendant la grossesse, à poursuivre en monothérapie à la dose minimale efficace avec supplémentation en acide folique ; les concentrations baissent nettement au cours de la grossesse, ce qui peut imposer une augmentation de dose puis un retour à la posologie antérieure après l'accouchement ; l'allaitement est possible sous surveillance de la somnolence du nourrisson.",
        sources: "RCP Keppra — base de données publique des médicaments (ANSM)\nANSM — antiépileptiques et grossesse, information des patientes\nHAS — épilepsies, parcours de soins",
        status: "",
        smr: "",
        tags: "antiépileptique, surveillance biologique",
        toxicity: "",
    },
    StarterDetail {
        name: "Tégrétol",
        indications: "Épilepsies partielles avec ou sans généralisation secondaire et épilepsies généralisées avec crises tonicocloniques ; névralgie du trijumeau et névralgie du glossopharyngien ; prévention des récidives des troubles bipolaires, notamment en cas de résistance ou d'intolérance au lithium ; états d'excitation maniaque et hypomaniaque ; syndrome de sevrage alcoolique.",
        mechanism: "Carbamazépine, dérivé iminostilbène bloquant les canaux sodiques voltage-dépendants sous leur forme inactivée, ce qui limite les décharges neuronales répétitives à haute fréquence sans gêner la transmission normale. Cette stabilisation membranaire explique aussi bien l'effet antiépileptique que l'effet remarquable sur la douleur paroxystique du trijumeau. C'est par ailleurs un inducteur enzymatique puissant, y compris de son propre métabolisme, ce qui domine son profil d'interactions.",
        dosage: "Épilepsie : débuter à 100 à 200 mg une à deux fois par jour, avec augmentation lente par paliers de 200 mg tous les cinq à sept jours ; posologie d'entretien usuelle de 800 à 1 200 mg par jour en deux à trois prises, ou en deux prises avec les formes à libération prolongée. Névralgie du trijumeau : débuter à 200 à 400 mg par jour et augmenter progressivement jusqu'à disparition de la douleur, en général entre 600 et 800 mg par jour, puis rechercher la dose minimale efficace. Chez le sujet âgé, débuter à 100 mg par jour et augmenter très lentement, du fait du risque d'hyponatrémie, de sédation et d'ataxie. Les doses doivent être réévaluées deux à quatre semaines après l'instauration, l'auto-induction enzymatique faisant baisser les concentrations.",
        contraindications: "Hypersensibilité à la carbamazépine et allergie croisée possible avec les antidépresseurs tricycliques, bloc auriculoventriculaire, antécédent d'hypoplasie médullaire ou de porphyrie hépatique aiguë, association aux IMAO non sélectifs et dans les deux semaines suivant leur arrêt, association au voriconazole et aux inhibiteurs de protéase. Chez les patients d'origine han, thaïlandaise ou d'Asie du Sud-Est, la présence de l'allèle HLA-B*1502 contre-indique le traitement en l'absence d'alternative.",
        ddi: "Inducteur enzymatique puissant des CYP3A4, CYP2C9, CYP2B6 et de la glucuroconjugaison : il diminue fortement l'efficacité des contraceptifs œstroprogestatifs, des progestatifs seuls et de l'implant, imposant une contraception non hormonale ou un dispositif intra-utérin au cuivre, avec information explicite de la patiente. Il abaisse également les concentrations des anticoagulants oraux directs et des AVK, de la lamotrigine, du valproate, des corticoïdes, de la ciclosporine, des inhibiteurs de protéase, du praziquantel, de la lévothyroxine, des statines métabolisées par le CYP3A4 et de la méthadone, avec risque de sevrage. Les inhibiteurs du CYP3A4 (macrolides sauf spiramycine, azolés, vérapamil, diltiazem, jus de pamplemousse) augmentent la carbamazépinémie avec risque de surdosage. Le valproate augmente la fraction active du métabolite époxyde. Association déconseillée aux médicaments hyponatrémiants (diurétiques thiazidiques, ISRS) et risque sérotoninergique avec les IMAO, contre-indiqués.",
        adverse: "Fréquents en début de traitement et souvent dose-dépendants : somnolence, sensations vertigineuses, ataxie, diplopie, vision floue, nausées, céphalées, éruption cutanée bénigne. Hyponatrémie par SIADH fréquente, surtout chez le sujet âgé et en association à un diurétique. Leucopénie modérée souvent transitoire, élévation isolée des gamma-GT liée à l'induction. Plus rarement mais graves : syndrome de Stevens-Johnson et syndrome de Lyell, dont le risque est fortement lié à l'allèle HLA-B*1502 dans les populations asiatiques ; syndrome DRESS avec fièvre, éruption, adénopathies, hyperéosinophilie et atteinte hépatique ; agranulocytose et aplasie médullaire ; hépatite ; troubles de la conduction cardiaque ; idées suicidaires.",
        monitoring: "NFS avec plaquettes, bilan hépatique, ionogramme et natrémie avant l'instauration, puis à quelques semaines et régulièrement ensuite, plus souvent chez le sujet âgé. Génotypage HLA-B*1502 avant instauration chez les patients originaires d'Asie du Sud-Est ou d'ascendance han. Carbamazépinémie utile en cas d'inefficacité, de signes de surdosage (diplopie, ataxie, somnolence), d'association modifiant le métabolisme et deux à quatre semaines après l'instauration en raison de l'auto-induction. Surveillance cutanée étroite les deux premiers mois, tout exanthème fébrile imposant un arrêt et un avis immédiat. Vérification à chaque délivrance de la contraception chez la femme en âge de procréer.",
        iup: "Prenez les comprimés au cours des repas, avec un grand verre d'eau, et respectez la montée progressive des doses : les vertiges et la vision double du début s'atténuent quand l'organisme s'habitue, c'est précisément pourquoi on augmente lentement. Si vous êtes une femme et que vous prenez une pilule, un implant ou un patch contraceptif, sachez que ce médicament les rend inefficaces : il faut mettre en place une autre contraception, un stérilet au cuivre ou un préservatif, et en parler à votre médecin sans attendre. N'arrêtez jamais brutalement, un arrêt sec peut déclencher des crises même si vous n'en aviez plus, et la diminution doit toujours être décidée et étalée avec le médecin. Surveillez votre peau pendant les deux premiers mois et consultez le jour même devant toute éruption, surtout si elle s'accompagne de fièvre, de gonflement du visage, de lésions dans la bouche ou d'un mauvais état général. Signalez également fièvre inexpliquée, mal de gorge, aphtes ou bleus faciles, qui peuvent traduire une atteinte des globules blancs, ainsi qu'une fatigue avec confusion ou des chutes, qui peuvent traduire une baisse du sodium sanguin. Évitez l'alcool et le jus de pamplemousse, et signalez ce traitement pour toute nouvelle prescription, car il diminue l'effet de très nombreux médicaments.",
        half_life: "≈ 36 heures à la première prise, ramenée à 16 à 24 heures après auto-induction enzymatique",
        elimination: "Métabolisme hépatique principalement par le CYP3A4 en carbamazépine-10,11-époxyde, métabolite actif et neurotoxique, puis hydrolyse par l'époxyde hydrolase et glucuroconjugaison ; élimination essentiellement urinaire sous forme de métabolites. Puissant inducteur enzymatique et auto-inducteur.",
        renal: "Pas d'adaptation systématique en insuffisance rénale légère à modérée. En insuffisance rénale sévère, prudence, surveillance clinique et dosages plasmatiques, l'accumulation des métabolites étant possible.",
        pregnancy: "Éviter pendant la grossesse en raison d'un risque malformatif accru, notamment de spina bifida ; si la poursuite est indispensable, maintenir la dose minimale efficace en monothérapie, avec supplémentation en acide folique et suivi échographique spécialisé, et ne jamais interrompre sans avis ; l'allaitement est possible sous surveillance de la somnolence et de la prise de poids du nourrisson.",
        sources: "RCP Tégrétol — base de données publique des médicaments (ANSM)\nANSM — antiépileptiques et grossesse, information des patientes\nANSM — carbamazépine et interaction avec les contraceptifs hormonaux",
        status: "",
        smr: "",
        tags: "antiépileptique, marge thérapeutique étroite, surveillance biologique",
        toxicity: "Marge thérapeutique étroite : un écart de dose ou une interaction suffit à faire basculer vers le sous-dosage ou la toxicité. Voir les sections Interactions et Surveillance.",
    },
    StarterDetail {
        name: "Fosamax",
        indications: "Traitement de l'ostéoporose post-ménopausique chez la femme à risque élevé de fracture, notamment après une fracture par fragilité. Traitement de l'ostéoporose masculine. Prévention et traitement de l'ostéoporose cortico-induite selon la spécialité et le dosage.",
        mechanism: "Bisphosphonate azoté qui se fixe avec une forte affinité à l'hydroxyapatite des surfaces osseuses en résorption. Internalisé par l'ostéoclaste, il inhibe la farnésyl-pyrophosphate synthase de la voie du mévalonate, ce qui empêche la prénylation des petites protéines G nécessaires à la bordure en brosse et conduit à l'inactivation puis à l'apoptose de l'ostéoclaste. Le résultat est une réduction durable de la résorption osseuse et une augmentation de la densité minérale.",
        dosage: "70 mg une fois par semaine, le même jour de la semaine, ou 10 mg par jour selon la présentation. Le comprimé se prend le matin à jeun, au lever, au moins 30 minutes avant toute autre prise alimentaire ou médicamenteuse, avec un grand verre d'eau plate faiblement minéralisée, en restant en position assise ou debout pendant les 30 minutes suivantes. La durée habituelle est de trois à cinq ans, avec réévaluation du rapport bénéfice-risque et discussion d'une fenêtre thérapeutique au-delà.",
        contraindications: "Anomalie de l'œsophage retardant le transit comme une sténose ou une achalasie, incapacité à rester en position assise ou debout pendant au moins 30 minutes, hypocalcémie non corrigée, clairance de la créatinine inférieure à 35 mL/min, hypersensibilité, grossesse et allaitement.",
        ddi: "Sels de calcium, sels de fer, magnésium, aluminium, antiacides, eaux minérales fortement minéralisées, lait, café et jus de fruits diminuent nettement l'absorption déjà très faible du produit : rien d'autre que de l'eau plate dans les 30 minutes qui suivent la prise. Les AINS majorent le risque d'irritation digestive haute. Les corticoïdes au long cours majorent le risque d'ostéonécrose de la mâchoire.",
        adverse: "Douleurs abdominales, dyspepsie, reflux, nausées, ballonnements, myalgies et arthralgies parfois intenses en début de traitement, céphalées. Œsophagite, ulcération ou sténose œsophagienne en cas de prise incorrecte. Plus rarement ostéonécrose de la mâchoire, fractures fémorales atypiques après plusieurs années, uvéite ou sclérite, réactions d'hypersensibilité.",
        monitoring: "Calcémie et statut en vitamine D corrigés avant l'instauration, fonction rénale avant traitement. Bilan bucco-dentaire préalable et remise en état avant le début. Surveillance des symptômes œsophagiens, des douleurs de mâchoire et de toute douleur de cuisse ou d'aine persistante. Ostéodensitométrie de contrôle à deux ou trois ans, et réévaluation formelle de la poursuite au-delà de cinq ans.",
        iup: "Le comprimé se prend le matin au réveil, à jeun, avant le petit-déjeuner, avec un grand verre d'eau du robinet ou d'eau plate peu minéralisée, jamais avec du café, du lait ou une eau riche en calcium. Il faut ensuite rester debout ou assis, sans se recoucher, pendant au moins 30 minutes, et n'avaler ni aliment ni autre médicament pendant ce délai. Il est utile de choisir un jour fixe de la semaine et de le noter, par exemple le dimanche matin. En cas d'oubli, prendre le comprimé le lendemain matin dans les mêmes conditions, puis revenir au jour habituel, sans jamais prendre deux comprimés le même jour. Le calcium et la vitamine D éventuellement prescrits se prennent à un autre moment de la journée, au moins deux heures plus tard. Il faut consulter en cas de brûlure derrière le sternum, de douleur à la déglutition, de douleur dentaire ou de la mâchoire, ou de douleur de cuisse persistante.",
        half_life: "Demi-vie plasmatique très courte, mais demi-vie osseuse supérieure à dix ans du fait de la fixation sur l'hydroxyapatite",
        elimination: "Non métabolisé ; la fraction non fixée à l'os est éliminée inchangée par voie rénale, la fraction fixée étant relarguée très lentement.",
        renal: "Contre-indiqué en dessous d'une clairance de 35 mL/min. Pas d'adaptation de dose au-dessus de ce seuil.",
        pregnancy: "Contre-indiqué pendant la grossesse et l'allaitement.",
        sources: "RCP Fosamax — base de données publique des médicaments (ANSM)\nHAS — prise en charge médicamenteuse de l'ostéoporose post-ménopausique\nANSM — ostéonécrose de la mâchoire sous bisphosphonates",
        status: "",
        smr: "",
        tags: "bisphosphonate, contre-indiqué grossesse",
        toxicity: "",
    },
    StarterDetail {
        name: "Xeloda",
        indications: "Traitement adjuvant du cancer du côlon de stade III après résection ; cancer colorectal métastatique, en monothérapie ou en association ; cancer gastrique avancé en association à un sel de platine ; cancer du sein localement avancé ou métastatique, en association au docétaxel ou en monothérapie après échec des anthracyclines et des taxanes.",
        mechanism: "Précurseur oral du 5-fluorouracile, activé en trois étapes successives : carboxylestérase hépatique, cytidine désaminase, puis thymidine phosphorylase dont l'activité est plus élevée dans le tissu tumoral, ce qui concentre le 5-FU dans la tumeur. Le 5-FU inhibe la thymidylate synthase et prive la cellule de thymidine, tandis que ses métabolites s'incorporent à l'ARN et à l'ADN, avec un effet cytotoxique portant sur les cellules en division.",
        dosage: "Monothérapie : 1250 mg/m² deux fois par jour pendant 14 jours, suivis de 7 jours d'arrêt, en cycles de 21 jours ; le traitement adjuvant du côlon se poursuit habituellement pendant huit cycles, soit environ six mois. En association, la dose est le plus souvent réduite à 1000 mg/m² deux fois par jour selon le protocole. Chaque prise se fait dans les 30 minutes qui suivent la fin d'un repas ; toute réduction de dose après toxicité est décidée par l'oncologue et n'est jamais rattrapée.",
        contraindications: "Déficit complet en dihydropyrimidine déshydrogénase, antécédent de réaction sévère à une fluoropyrimidine, leucopénie ou thrombopénie sévère, insuffisance hépatique sévère, clairance de la créatinine inférieure à 30 mL/min, association à la brivudine ou à la sorivudine et dans les quatre semaines qui suivent leur arrêt, grossesse et allaitement.",
        ddi: "Brivudine et sorivudine : inhibition irréversible de la DPD et toxicité potentiellement mortelle du 5-FU, association contre-indiquée avec un délai de quatre semaines. Antivitamines K : élévation marquée et parfois retardée de l'INR avec risque hémorragique, surveillance rapprochée indispensable. Phénytoïne : concentrations augmentées. Allopurinol : perte d'efficacité, association déconseillée. L'acide folinique majore la toxicité digestive et hématologique.",
        adverse: "Syndrome main-pied (érythème, sécheresse, fissures, douleur palmoplantaire) très fréquent et dose-limitant, diarrhée parfois sévère, nausées, vomissements, stomatite, anorexie, fatigue, hyperbilirubinémie. Plus rarement neutropénie fébrile, déshydratation sévère par diarrhée, cardiotoxicité à type de spasme coronarien avec angor ou troubles du rythme, et toxidermies graves.",
        monitoring: "Recherche du déficit en dihydropyrimidine déshydrogénase par dosage de l'uracilémie avant la première cure, exigée avant toute fluoropyrimidine. Hémogramme, ionogramme, créatininémie et bilan hépatique avant chaque cycle. Évaluation à chaque cycle de la diarrhée, de l'état buccal et des mains et des pieds, avec réduction de dose ou suspension dès le grade 2. INR renforcé en cas d'AVK associé.",
        iup: "Les comprimés se prennent matin et soir, dans les 30 minutes qui suivent la fin du repas, avec un verre d'eau, sans être écrasés ni coupés. Le rythme est strict : 14 jours de prise, puis 7 jours sans rien, et il faut tenir un calendrier pour ne pas se tromper de semaine. Appliquer une crème émolliente sur les mains et les pieds, éviter les frottements, la chaleur et les travaux manuels appuyés. Il faut arrêter le traitement et appeler l'oncologue sans attendre la fin du cycle en cas de diarrhée dépassant quatre selles par jour ou survenant la nuit, de bouche trop douloureuse pour s'alimenter, de vomissements répétés, de fièvre, ou de mains et de pieds rouges et douloureux. Toute douleur thoracique doit conduire aux urgences. Une contraception efficace est nécessaire pendant le traitement et plusieurs mois après.",
        half_life: "Environ 45 minutes pour la capécitabine et ses métabolites, le 5-FU circulant ayant une demi-vie de l'ordre de 10 à 20 minutes",
        elimination: "Métabolisme séquentiel hépatique et tumoral, puis catabolisme du 5-FU par la DPD ; plus de 70 % de la dose est éliminée dans les urines, essentiellement sous forme de fluoro-bêta-alanine.",
        renal: "Clairance 30 à 50 mL/min : réduire la dose à 75 % de la dose initiale. Inférieure à 30 mL/min : contre-indiqué.",
        pregnancy: "Contre-indiqué pendant la grossesse et l'allaitement, avec contraception efficace chez la femme comme chez l'homme pendant le traitement et après son arrêt.",
        sources: "RCP Xeloda — base de données publique des médicaments (ANSM)\nANSM — recherche du déficit en dihydropyrimidine déshydrogénase avant tout traitement par fluoropyrimidine\nINCa — fiches de bon usage des anticancéreux oraux",
        status: "",
        smr: "",
        tags: "anticancéreux oral, fluoropyrimidine, marge thérapeutique étroite, surveillance biologique, contre-indiqué grossesse",
        toxicity: "Marge thérapeutique étroite : un écart de dose ou une interaction suffit à faire basculer vers le sous-dosage ou la toxicité. Voir les sections Interactions et Surveillance.",
    },
    StarterDetail {
        name: "Abilify",
        indications: "Traitement de la schizophrénie chez l'adulte et l'adolescent à partir de quinze ans ; traitement des épisodes maniaques modérés à sévères des troubles bipolaires de type I et prévention de récidive d'un épisode maniaque chez les patients ayant présenté des épisodes maniaques répondant au traitement.",
        mechanism: "Aripiprazole, antipsychotique de troisième génération agissant comme agoniste partiel des récepteurs dopaminergiques D2 et sérotoninergiques 5-HT1A, et antagoniste des récepteurs 5-HT2A. Son agonisme partiel D2 stabilise la transmission dopaminergique : il réduit l'hyperactivité dopaminergique mésolimbique tout en préservant une activité minimale dans les voies nigrostriée et tubéro-infundibulaire. Il en résulte un profil peu sédatif, peu métabolique et sans hyperprolactinémie, mais volontiers pourvoyeur d'akathisie.",
        dosage: "Schizophrénie : dose initiale de 10 ou 15 mg par jour en une prise, indépendamment des repas, dose d'entretien recommandée de 15 mg par jour. Épisode maniaque : 15 mg par jour en une prise, en monothérapie ou en association au lithium ou au valproate ; prévention des récidives, poursuite à la dose ayant permis la stabilisation. La posologie maximale est de 30 mg par jour, sans bénéfice démontré au-delà de 15 mg dans la plupart des situations. Chez le sujet âgé, aucune adaptation systématique n'est prévue mais une dose initiale plus faible est prudente. En cas d'association à un inhibiteur puissant du CYP2D6 ou du CYP3A4, réduire la dose de moitié ; en cas d'association à un inducteur puissant comme la carbamazépine, la doubler puis réajuster à l'arrêt de l'inducteur.",
        contraindications: "Hypersensibilité à l'aripiprazole. Comme les autres antipsychotiques, il n'est pas indiqué dans les troubles psychotiques et comportementaux liés à la démence, où les antipsychotiques augmentent la mortalité et le risque d'accident vasculaire cérébral.",
        ddi: "Les inhibiteurs puissants du CYP2D6 (paroxétine, fluoxétine, quinidine, bupropion) et du CYP3A4 (kétoconazole, itraconazole, clarithromycine, ritonavir, jus de pamplemousse) augmentent l'exposition et imposent de réduire la dose de moitié. Les inducteurs puissants, au premier rang desquels la carbamazépine, mais aussi la rifampicine, la phénytoïne et le millepertuis, diminuent fortement les concentrations et imposent de doubler la dose, avec réajustement à l'arrêt de l'inducteur. Sédation additive avec l'alcool et les dépresseurs centraux. Prudence en association aux médicaments allongeant le QT et aux antihypertenseurs, du fait de l'hypotension orthostatique.",
        adverse: "Fréquents : akathisie, souvent précoce et principale cause d'arrêt, se traduisant par une impatience motrice difficile à distinguer d'une aggravation anxieuse ; insomnie, anxiété, agitation, céphalées, nausées, vomissements, constipation, sensations vertigineuses, somnolence, tremblements, vision floue. Prise de poids et retentissement métabolique nettement moindres qu'avec l'olanzapine, et pas d'hyperprolactinémie, la prolactinémie tendant même à baisser. Troubles du contrôle des impulsions décrits et parfois sévères : jeu pathologique, achats compulsifs, hyperphagie, hypersexualité, réversibles à l'arrêt et rarement rapportés spontanément par le patient. Plus rarement mais graves : syndrome malin des neuroleptiques, dyskinésies tardives, convulsions, allongement du QT, hypotension orthostatique avec syncope, thromboembolie veineuse, hyperglycémie et acidocétose.",
        monitoring: "Recherche active de l'akathisie dans les premières semaines, en la distinguant d'une recrudescence anxieuse, car la conduite à tenir est opposée. Interrogatoire explicite et répété sur les troubles du contrôle des impulsions, en questionnant aussi l'entourage : jeux d'argent, dépenses inhabituelles, comportement sexuel, prises alimentaires compulsives. Bilan métabolique avant l'instauration puis à trois mois et annuellement : poids, tour de taille, pression artérielle, glycémie à jeun, bilan lipidique, même si le risque est plus faible qu'avec les autres antipsychotiques atypiques. ECG en cas de cardiopathie ou d'association allongeant le QT. Recherche de dyskinésies tardives sous traitement prolongé, et dosage des CPK devant une hyperthermie avec rigidité.",
        iup: "Un comprimé par jour, à heure fixe, le matin de préférence car il est plutôt stimulant et peut gêner l'endormissement, avec ou sans aliments. L'effet sur l'agitation ou la manie se voit en quelques jours, mais l'effet complet sur les symptômes psychotiques demande plusieurs semaines ; le traitement se poursuit ensuite pour éviter la rechute, même quand tout va bien. Il arrive fréquemment dans les premières semaines de ressentir une impossibilité de tenir en place, un besoin permanent de bouger les jambes ou de marcher : ce n'est pas de l'anxiété, c'est un effet du médicament, il faut le signaler rapidement car il se traite. Ce médicament peut aussi, chez certaines personnes, déclencher des comportements incontrôlables de jeu, d'achats, d'alimentation ou de sexualité : c'est réversible à l'arrêt, alors parlez-en sans gêne, à vous ou à un proche qui le remarquerait. N'arrêtez jamais brutalement, la diminution se décide avec le psychiatre, et évitez l'alcool ainsi que le jus de pamplemousse. Consultez le jour même en cas de fièvre avec raideur musculaire et sueurs, de mouvements involontaires du visage ou de la langue, de malaise en vous levant, ou de soif intense avec urines abondantes.",
        half_life: "≈ 75 heures pour l'aripiprazole et ≈ 94 heures pour le déhydro-aripiprazole actif",
        elimination: "Métabolisme hépatique par les CYP2D6 et CYP3A4, avec formation du déhydro-aripiprazole actif ; élimination essentiellement fécale et pour une moindre part urinaire, moins de 1 % sous forme inchangée dans les urines. Les métaboliseurs lents du CYP2D6 présentent une exposition nettement plus élevée.",
        renal: "Pas d'adaptation, l'élimination rénale du produit inchangé étant négligeable, y compris en insuffisance rénale sévère.",
        pregnancy: "Utilisation possible pendant la grossesse si elle est nécessaire, en prévenant l'équipe obstétricale d'un risque de syndrome extrapyramidal ou de sevrage néonatal en cas d'exposition en fin de grossesse ; l'allaitement est déconseillé, les données étant limitées et la demi-vie très longue.",
        sources: "RCP Abilify — base de données publique des médicaments (ANSM)\nHAS — schizophrénies, parcours de soins\nANSM — aripiprazole et troubles du contrôle des impulsions",
        status: "",
        smr: "",
        tags: "antipsychotique atypique, surveillance biologique",
        toxicity: "",
    },
    StarterDetail {
        name: "Zyprexa",
        indications: "Traitement de la schizophrénie, en phase aiguë comme en traitement d'entretien ; traitement des épisodes maniaques modérés à sévères ; prévention des récidives chez les patients présentant un trouble bipolaire ayant répondu à l'olanzapine lors d'un épisode maniaque.",
        mechanism: "Olanzapine, antipsychotique atypique antagoniste des récepteurs dopaminergiques D1 à D4 et sérotoninergiques 5-HT2A, 5-HT2C, 5-HT3 et 5-HT6, avec une affinité marquée pour les récepteurs histaminiques H1, muscariniques M1 à M5 et alpha-1 adrénergiques. Le rapport d'affinité en faveur des récepteurs 5-HT2A par rapport aux D2 explique la moindre incidence des effets extrapyramidaux. Le blocage histaminique H1 et sérotoninergique 5-HT2C rend compte de la sédation et de l'augmentation majeure de l'appétit.",
        dosage: "Schizophrénie : 10 mg par jour en une prise, indépendamment des repas. Épisode maniaque : 15 mg par jour en monothérapie, ou 10 mg par jour en association à un thymorégulateur. Prévention des récidives : 10 mg par jour. L'ajustement se fait ensuite entre 5 et 20 mg par jour, par paliers de 5 mg à intervalle d'au moins vingt-quatre heures, la dose de 20 mg n'étant retenue qu'après réévaluation clinique. Chez le sujet de plus de 65 ans, débuter à 5 mg par jour, de même qu'en cas d'insuffisance rénale ou hépatique, de sexe féminin non fumeur ou de facteurs de risque d'hypotension. L'arrêt se fait par diminution progressive, un arrêt brutal exposant à des nausées, une insomnie et des sueurs.",
        contraindications: "Hypersensibilité à l'olanzapine, glaucome à angle fermé ou risque de glaucome par fermeture de l'angle. L'olanzapine n'est pas indiquée dans les troubles psychotiques et comportementaux liés à la démence, situation dans laquelle les antipsychotiques augmentent la mortalité et le risque d'accident vasculaire cérébral.",
        ddi: "Le tabac induit le CYP1A2 et abaisse les concentrations d'environ un tiers : un arrêt du tabac peut entraîner un surdosage avec sédation et chutes, et impose de réévaluer la dose. Les inhibiteurs puissants du CYP1A2 (fluvoxamine, ciprofloxacine) augmentent nettement l'exposition et justifient une réduction de dose. La carbamazépine, inducteur enzymatique, diminue les concentrations d'environ moitié. Sédation additive et dépression respiratoire avec l'alcool, les benzodiazépines et les opioïdes ; l'association de l'olanzapine injectable à une benzodiazépine parentérale est déconseillée du fait du risque de dépression cardiorespiratoire. Effets anticholinergiques additifs et risque d'occlusion. Prudence en association aux médicaments allongeant le QT et aux antihypertenseurs, l'olanzapine majorant l'hypotension orthostatique.",
        adverse: "Très fréquents et dominants : prise de poids souvent importante et rapide, augmentation de l'appétit, somnolence, élévation de la prolactinémie généralement modérée et transitoire, hypotension orthostatique, sécheresse buccale, constipation, œdèmes, élévation des transaminases. Fréquents : hyperglycémie, dyslipidémie avec hypertriglycéridémie, vertiges, akathisie et effets extrapyramidaux modérés, éosinophilie. Plus rarement mais graves : diabète parfois révélé par une acidocétose ou un coma hyperosmolaire, syndrome malin des neuroleptiques avec hyperthermie, rigidité, dysautonomie et élévation des CPK, convulsions, neutropénie, pancréatite, thromboembolie veineuse, allongement du QT, dyskinésies tardives sous traitement prolongé.",
        monitoring: "Bilan métabolique complet avant l'instauration puis à trois mois et au moins une fois par an : poids et indice de masse corporelle, tour de taille, pression artérielle, glycémie à jeun, bilan lipidique. Le poids doit être suivi mensuellement pendant les trois premiers mois, la prise pondérale étant précoce et prédictive. Bilan hépatique et NFS périodiques. ECG en cas de cardiopathie, d'association allongeant le QT ou de troubles électrolytiques. Recherche clinique d'effets extrapyramidaux, d'akathisie et de dyskinésies tardives, et dosage de la prolactine en cas d'aménorrhée, de galactorrhée ou de troubles sexuels. Devant toute hyperthermie avec rigidité, évoquer un syndrome malin et doser les CPK en urgence.",
        iup: "Un comprimé par jour, à heure fixe, de préférence le soir puisque le médicament est sédatif, avec ou sans aliments ; il existe une forme orodispersible qui fond sur la langue si la prise du comprimé est difficile. L'effet sur l'agitation et le sommeil apparaît en quelques jours, mais l'effet complet sur les idées délirantes ou les hallucinations demande plusieurs semaines : le traitement se poursuit même quand tout va bien, car c'est ce qui empêche la rechute. Ce médicament ouvre l'appétit et fait souvent prendre du poids rapidement dans les premiers mois : anticipez dès maintenant sur l'alimentation et l'activité physique, et faites-vous peser régulièrement, c'est plus facile d'éviter la prise que de la reprendre ensuite. Levez-vous en deux temps, surtout au début et la nuit, car la tension peut baisser en position debout et faire tomber. N'arrêtez jamais brutalement de vous-même, la diminution se décide avec le psychiatre. Évitez l'alcool, prudence au volant tant que la somnolence persiste, et consultez le jour même en cas de fièvre avec raideur musculaire et sueurs, de soif intense avec urines abondantes et amaigrissement, de fièvre avec mal de gorge, ou de mouvements anormaux involontaires du visage ou de la langue.",
        half_life: "≈ 33 heures, plus longue chez la femme et le sujet âgé",
        elimination: "Métabolisme hépatique par glucuroconjugaison directe et oxydation par le CYP1A2, accessoirement par le CYP2D6 ; élimination majoritairement urinaire sous forme de métabolites inactifs, environ 7 % sous forme inchangée.",
        renal: "Pas d'adaptation obligatoire, l'élimination rénale du produit inchangé étant faible ; il est toutefois recommandé de débuter à 5 mg par jour en cas d'insuffisance rénale, comme en cas d'insuffisance hépatique modérée.",
        pregnancy: "Utilisation possible pendant la grossesse si elle est nécessaire, l'olanzapine faisant partie des antipsychotiques les mieux documentés, avec surveillance de la glycémie et de la prise de poids maternelles et information de l'équipe obstétricale d'un risque de syndrome extrapyramidal ou de sevrage néonatal en cas d'exposition tardive ; l'allaitement est déconseillé, le passage lacté étant non négligeable.",
        sources: "RCP Zyprexa — base de données publique des médicaments (ANSM)\nHAS — schizophrénies, parcours de soins\nANSM — antipsychotiques et troubles métaboliques, suivi recommandé",
        status: "",
        smr: "",
        tags: "antipsychotique atypique, surveillance biologique",
        toxicity: "",
    },
    StarterDetail {
        name: "Leponex",
        indications: "Schizophrénie résistante, définie par l'absence de réponse ou l'intolérance à au moins deux antipsychotiques différents, dont un antipsychotique atypique, prescrits à posologie adéquate et pendant une durée suffisante ; troubles psychotiques survenant au cours de l'évolution de la maladie de Parkinson, après échec des stratégies habituelles.",
        mechanism: "Clozapine, antipsychotique atypique de référence, antagoniste faible et rapidement dissociable des récepteurs dopaminergiques D2, avec une affinité marquée pour les récepteurs D4, sérotoninergiques 5-HT2A, alpha-1 et alpha-2 adrénergiques, muscariniques et histaminiques H1. La faible occupation striatale des récepteurs D2 explique l'absence quasi complète de syndrome extrapyramidal et de dyskinésies tardives. Ce profil multireceptoriel rend compte de son efficacité unique dans la schizophrénie résistante comme de sa toxicité hématologique, métabolique et cardiaque.",
        dosage: "La titration est très lente et impérative. Premier jour : 12,5 mg une ou deux fois ; deuxième jour : 25 à 50 mg ; puis augmentation par paliers de 25 à 50 mg par jour, sur deux à trois semaines, jusqu'à 300 mg par jour en deux ou trois prises, la dose du soir étant la plus forte. La posologie d'entretien usuelle se situe entre 200 et 450 mg par jour ; en cas de réponse insuffisante, augmentation par paliers de 50 à 100 mg à intervalle d'au moins une semaine, sans dépasser 900 mg par jour. Chez le sujet âgé et dans la psychose parkinsonienne, débuter à 12,5 mg le soir et augmenter très lentement, les doses efficaces restant très faibles, souvent de 25 à 50 mg par jour. Toute interruption de plus de deux jours impose de reprendre la titration à 12,5 mg, sous peine de collapsus.",
        contraindications: "Antécédent d'agranulocytose ou de granulopénie d'origine médicamenteuse, hémopathie, insuffisance médullaire, impossibilité de réaliser la surveillance hématologique régulière, épilepsie non contrôlée, psychose alcoolique ou toxique, intoxication médicamenteuse aiguë, état comateux, collapsus circulatoire, dépression du système nerveux central, insuffisance hépatique sévère ou hépatite évolutive, insuffisance rénale sévère, myocardite ou cardiopathie sévère, iléus paralytique, association aux médicaments dépresseurs de la moelle osseuse et aux antipsychotiques retard.",
        ddi: "Association contre-indiquée aux médicaments myélotoxiques et à ceux susceptibles de provoquer une agranulocytose : carbamazépine, thiamazole, sulfamides, métamizole, chimiothérapies cytotoxiques, ainsi qu'aux antipsychotiques à action prolongée dont l'effet ne peut être interrompu en cas d'atteinte hématologique. L'association aux benzodiazépines expose à un risque de collapsus, de dépression respiratoire et d'arrêt cardiorespiratoire, surtout à l'instauration. Les inhibiteurs du CYP1A2 (fluvoxamine, ciprofloxacine) augmentent fortement la clozapinémie ; le tabac induit le CYP1A2, et un arrêt brutal du tabac peut doubler les concentrations et provoquer un surdosage, ce qui doit être anticipé lors d'une hospitalisation. Association déconseillée aux médicaments allongeant le QT et à ceux abaissant le seuil épileptogène ; effets anticholinergiques additifs avec risque d'occlusion intestinale.",
        adverse: "Très fréquents : sédation importante, sialorrhée nocturne paradoxale, tachycardie, hypotension orthostatique, prise de poids majeure, constipation. Fréquents : hyperthermie bénigne transitoire à l'instauration, vertiges, énurésie nocturne, élévation des transaminases, hyperglycémie et dyslipidémie. Plus rarement mais graves et déterminants : agranulocytose, dont le risque est maximal pendant les dix-huit premières semaines et qui justifie l'ensemble du dispositif de surveillance ; myocardite et cardiomyopathie, essentiellement dans les deux premiers mois ; convulsions dose-dépendantes au-delà de 600 mg par jour ; constipation sévère pouvant aller jusqu'à l'iléus paralytique et à la perforation, première cause de décès sous clozapine ; diabète parfois inaugural par acidocétose ; thromboembolie veineuse ; syndrome malin des neuroleptiques.",
        monitoring: "Hémogramme obligatoire avant l'instauration, avec leucocytes au moins égaux à 3 500/mm³ et polynucléaires neutrophiles au moins égaux à 2 000/mm³, puis NFS hebdomadaire pendant les dix-huit premières semaines, puis mensuelle pendant toute la durée du traitement et durant les quatre semaines suivant l'arrêt. La délivrance est subordonnée à la présentation du carnet de surveillance renseigné et à un résultat hématologique conforme datant de moins de sept jours en phase hebdomadaire ; toute baisse impose l'avis immédiat du prescripteur et, en cas d'agranulocytose, l'arrêt définitif sans réintroduction possible. Surveillance cardiaque à l'instauration : fréquence cardiaque, pression artérielle, troponine, CRP et ECG en cas de signe d'appel dans les deux premiers mois. Bilan métabolique avec poids, tour de taille, glycémie à jeun et bilan lipidique avant traitement, à trois mois puis au moins annuellement. Surveillance active du transit à chaque consultation.",
        iup: "Ce traitement est le plus efficace lorsque les autres ont échoué, mais il ne se conçoit qu'avec la prise de sang : hebdomadaire pendant dix-huit semaines, puis mensuelle tant que dure le traitement, et je ne peux vous délivrer les comprimés que sur présentation du carnet à jour. La montée des doses est très lente et doit être suivie à la lettre ; si vous interrompez le traitement plus de deux jours, ne reprenez surtout pas à la dose habituelle, appelez le médecin, car il faudra tout recommencer à la plus petite dose. Attendez-vous à être très somnolent au début, à baver la nuit sur l'oreiller et à avoir des vertiges en vous levant : levez-vous en deux temps, et ces effets s'atténuent en partie avec le temps. La constipation est le danger le plus sous-estimé de ce médicament : buvez, mangez des fibres, bougez, et signalez tout arrêt du transit de plusieurs jours ou tout ventre douloureux et ballonné, qui doit conduire aux urgences. Si vous fumez, ne modifiez pas brutalement votre consommation sans le dire, car arrêter le tabac fait fortement monter le taux du médicament dans le sang. Appelez ou consultez le jour même en cas de fièvre, de mal de gorge, d'aphtes ou de grippe inhabituelle, de palpitations, d'essoufflement ou de douleur thoracique dans les deux premiers mois, de soif intense avec urines abondantes, ou de perte de connaissance.",
        half_life: "≈ 12 heures à l'état d'équilibre",
        elimination: "Métabolisme hépatique presque complet par les CYP1A2 et CYP3A4, avec contribution du CYP2D6 ; principal métabolite, la norclozapine, faiblement active ; élimination urinaire et fécale sous forme de métabolites, moins de 5 % sous forme inchangée. Le tabac est un inducteur puissant du CYP1A2.",
        renal: "Insuffisance rénale légère à modérée : réduction de la posologie et titration très prudente. Insuffisance rénale sévère : contre-indiqué.",
        pregnancy: "À n'utiliser pendant la grossesse que si le bénéfice est clairement supérieur au risque, en surveillant la glycémie maternelle et en prévenant l'équipe obstétricale d'un risque de syndrome extrapyramidal ou de sevrage néonatal ainsi que d'agranulocytose néonatale, avec NFS du nouveau-né ; l'allaitement est contre-indiqué.",
        sources: "RCP Leponex — base de données publique des médicaments (ANSM)\nANSM — clozapine, conditions de prescription et de délivrance et surveillance hématologique\nHAS — schizophrénies, parcours de soins",
        status: "Délivrance conditionnée à la NFS (carnet de surveillance)",
        smr: "",
        tags: "antipsychotique, nfs obligatoire, marge thérapeutique étroite, surveillance biologique, contre-indiqué grossesse",
        toxicity: "Marge thérapeutique étroite : un écart de dose ou une interaction suffit à faire basculer vers le sous-dosage ou la toxicité. Voir les sections Interactions et Surveillance.",
    },
    StarterDetail {
        name: "Tavanic",
        indications: "Pneumonie aiguë communautaire, y compris la légionellose, sinusite aiguë bactérienne documentée, exacerbation de bronchopneumopathie chronique obstructive, pyélonéphrite aiguë et infection urinaire à risque de complication, infection urinaire masculine et prostatite, et infections compliquées de la peau et des tissus mous, en tenant compte des restrictions d'usage de la classe.",
        mechanism: "Fluoroquinolone correspondant à l'énantiomère lévogyre actif de l'ofloxacine, environ deux fois plus puissant que le racémique. Elle inhibe l'ADN gyrase et la topo-isomérase IV, bloquant la réplication de l'ADN bactérien, avec une action bactéricide concentration-dépendante. Son activité renforcée sur Streptococcus pneumoniae, y compris de sensibilité diminuée à la pénicilline, et sur les germes atypiques lui vaut le nom de fluoroquinolone anti-pneumococcique.",
        dosage: "Adulte : 500 mg une fois par jour dans la plupart des indications, la légionellose ou les formes sévères pouvant justifier 500 mg deux fois par jour. Pneumonie communautaire : 7 jours en règle générale. Pyélonéphrite : 7 jours. Infection urinaire masculine et prostatite : 14 jours en règle générale. La durée dépend de l'indication et de l'évolution clinique et n'est jamais raccourcie sans avis.",
        contraindications: "Hypersensibilité aux quinolones, antécédent de tendinopathie sous fluoroquinolone, épilepsie, enfant et adolescent en période de croissance, grossesse et allaitement. Prudence en cas de myasthénie, de déficit en G6PD ou d'antécédent d'anévrisme ou de dissection aortique.",
        ddi: "Chélation par le fer, le calcium, le magnésium, le zinc, l'aluminium, les antiacides, le sucralfate et les topiques gastro-intestinaux : absorption effondrée, prise à espacer d'au moins 2 heures avant ou 4 à 6 heures après. Antivitamines K : élévation de l'INR. Corticoïdes : risque tendineux majoré, surtout après 60 ans. AINS : abaissement du seuil épileptogène. Médicaments allongeant le QT : effet additif. Antidiabétiques : hypoglycémies et hyperglycémies décrites.",
        adverse: "Nausées, diarrhée, céphalées, insomnie, vertiges, élévation des transaminases. Plus rarement tendinopathie et rupture du tendon d'Achille, troubles neuropsychiques dont confusion, hallucinations et convulsions, neuropathie périphérique parfois durable, photosensibilité, allongement du QT, dysglycémie sévère notamment chez le sujet âgé diabétique, colite à Clostridioides difficile, hépatite, toxidermies graves et atteintes de l'aorte.",
        monitoring: "Réévaluation clinique à 48-72 heures, avec désescalade vers un antibiotique de spectre plus étroit dès que la documentation le permet. Glycémie chez le diabétique et chez le sujet âgé. Fonction rénale avant l'instauration, la posologie d'entretien en dépendant directement. Recherche de douleurs tendineuses, de paresthésies et de troubles du comportement à chaque contact, y compris après l'arrêt.",
        iup: "Une seule prise par jour, à heure fixe, avec un grand verre d'eau, pendant ou en dehors des repas. Il faut impérativement espacer d'au moins deux heures le fer, le calcium, le magnésium, le zinc, les pansements gastriques et les compléments minéraux, sinon le comprimé perd l'essentiel de son efficacité. Se protéger du soleil et éviter les UV pendant le traitement. Éviter les efforts sportifs intenses et arrêter le traitement en consultant le jour même devant toute douleur du tendon d'Achille, de la cheville ou de l'épaule. Signaler également une confusion, des idées noires, une anxiété inhabituelle, des fourmillements persistants, un malaise ou des palpitations, ainsi que toute douleur brutale du ventre ou du dos. Le traitement doit être mené à son terme, notamment dans les infections de la prostate.",
        half_life: "6 à 8 heures",
        elimination: "Métabolisme négligeable ; plus de 85 % de la dose est éliminée dans les urines sous forme inchangée.",
        renal: "Adaptation nécessaire dès une clairance inférieure à 50 mL/min : dose de charge habituelle, puis dose d'entretien réduite de moitié entre 20 et 50 mL/min et au quart en dessous de 20 mL/min.",
        pregnancy: "Contre-indiquée pendant la grossesse et l'allaitement ; une alternative est utilisée dans ces situations.",
        sources: "RCP Tavanic — base de données publique des médicaments (ANSM)\nANSM — restriction d'utilisation des fluoroquinolones et effets indésirables invalidants et durables\nSPILF — antibiothérapie des pneumonies aiguës communautaires et des infections urinaires de l'adulte",
        status: "",
        smr: "",
        tags: "fluoroquinolone, contre-indiqué grossesse",
        toxicity: "",
    },
    StarterDetail {
        name: "Prolia",
        indications: "Traitement de l'ostéoporose post-ménopausique chez la femme à risque élevé de fracture, et de l'ostéoporose masculine à risque élevé de fracture. Traitement de la perte osseuse associée à un traitement hormono-ablatif dans le cancer de la prostate, et de la perte osseuse associée à une corticothérapie systémique prolongée chez l'adulte à risque élevé de fracture.",
        mechanism: "Anticorps monoclonal humain dirigé contre le RANK-ligand, cytokine indispensable à la formation, à la fonction et à la survie des ostéoclastes. En empêchant la liaison du RANKL à son récepteur RANK, le dénosumab supprime la résorption osseuse ostéoclastique, de façon rapide et profonde mais entièrement réversible à l'arrêt, ce qui explique le rebond de résorption après interruption.",
        dosage: "60 mg en une injection sous-cutanée tous les 6 mois, dans la cuisse, l'abdomen ou la face postérieure du bras. Le traitement est un traitement au long cours, réévalué périodiquement ; il ne doit jamais être interrompu sans relais par un autre antirésorbeur. Un apport quotidien suffisant en calcium et en vitamine D est indispensable pendant toute la durée du traitement.",
        contraindications: "Hypocalcémie non corrigée, hypersensibilité au dénosumab, grossesse. Une hypocalcémie doit être corrigée avant toute injection ; un foyer infectieux dentaire ou une extraction dentaire prévue doivent être traités avant l'instauration.",
        ddi: "Pas d'interaction pharmacocinétique cliniquement significative, le dénosumab n'étant pas métabolisé par les cytochromes. L'association à un autre traitement antirésorbeur n'est pas recommandée. Les corticoïdes au long cours et les immunosuppresseurs majorent le risque infectieux et le risque d'ostéonécrose de la mâchoire.",
        adverse: "Douleurs musculo-squelettiques et douleurs des extrémités, eczéma et dermatites, infections cutanées dont cellulites, cystites, sciatalgies, hypocalcémie souvent asymptomatique. Plus rarement ostéonécrose de la mâchoire, fractures fémorales atypiques sous-trochantériennes, hypersensibilité. À l'arrêt sans relais, rebond de résorption osseuse avec risque de fractures vertébrales multiples dans l'année qui suit.",
        monitoring: "Calcémie avant chaque injection, et de façon rapprochée dans les deux semaines suivantes chez l'insuffisant rénal sévère ou dialysé. Statut en vitamine D corrigé avant l'instauration. Examen bucco-dentaire avant le début du traitement puis surveillance des douleurs dentaires ou mandibulaires. Surveillance des douleurs de cuisse ou d'aine, qui peuvent précéder une fracture atypique. Ostéodensitométrie de suivi selon l'avis du prescripteur.",
        iup: "L'injection se fait tous les six mois, à date fixe : il faut noter la date de la prochaine injection, car un retard important expose au rebond de perte osseuse. La seringue se conserve au réfrigérateur entre 2 et 8 °C, sans être congelée, et se sort 15 à 30 minutes avant l'injection pour revenir à température ambiante, sans la réchauffer autrement. Le calcium et la vitamine D prescrits en même temps font partie du traitement et se prennent tous les jours. Il faut consulter un dentiste avant de commencer et prévenir tout dentiste ou chirurgien-dentiste que l'on reçoit ce traitement avant une extraction. Signaler des fourmillements autour de la bouche, des crampes ou des spasmes musculaires, qui évoquent une baisse du calcium, ainsi qu'une douleur de la mâchoire, une dent qui bouge, ou une douleur persistante de la cuisse ou de l'aine. Enfin, ce traitement ne s'arrête jamais de sa propre initiative : un relais est toujours nécessaire.",
        half_life: "Environ 26 jours",
        elimination: "Catabolisme protéique par le système réticulo-endothélial, comme les immunoglobulines ; pas d'élimination rénale ni hépatique de la molécule.",
        renal: "Pas d'adaptation de dose quelle que soit la clairance, mais le risque d'hypocalcémie augmente fortement en dessous de 30 mL/min et chez le dialysé, ce qui impose une supplémentation et une surveillance calcique rapprochées.",
        pregnancy: "Contre-indiqué pendant la grossesse, avec contraception efficace pendant le traitement et plusieurs mois après ; l'allaitement est déconseillé.",
        sources: "RCP Prolia — base de données publique des médicaments (ANSM)\nHAS — prise en charge médicamenteuse de l'ostéoporose post-ménopausique\nANSM — ostéonécrose de la mâchoire et fractures atypiques sous antirésorbeurs osseux",
        status: "",
        smr: "",
        tags: "anti-rankl, semestriel, contre-indiqué grossesse",
        toxicity: "",
    },
];

/// How many drugs a fresh base starts with.
pub const STARTER_DRUG_COUNT: usize = STARTER_DRUGS.len();

/// (brand name, DCI, therapeutic class, textbook antidote or ""). See
/// [`Db::seed_drugs_if_empty`].
const STARTER_DRUGS: &[(&str, &str, &str, &str)] = &[
    // Anticoagulants / antiagrégants
    ("Eliquis", "apixaban", "AOD", "Andexanet alfa"),
    ("Xarelto", "rivaroxaban", "AOD", "Andexanet alfa"),
    ("Pradaxa", "dabigatran", "AOD", "Idarucizumab"),
    ("Lixiana", "edoxaban", "AOD", ""),
    ("Coumadine", "warfarine", "AVK", "Vitamine K"),
    ("Previscan", "fluindione", "AVK", "Vitamine K"),
    ("Sintrom", "acénocoumarol", "AVK", "Vitamine K"),
    ("Héparine", "héparine sodique", "héparine", "Protamine"),
    ("Lovenox", "énoxaparine", "HBPM", ""),
    ("Kardégic", "acide acétylsalicylique", "antiagrégant", ""),
    ("Plavix", "clopidogrel", "antiagrégant", ""),
    ("Brilique", "ticagrélor", "antiagrégant", ""),
    // Douleur
    ("Doliprane", "paracétamol", "antalgique", "N-acétylcystéine"),
    ("Dafalgan", "paracétamol", "antalgique", "N-acétylcystéine"),
    ("Tramadol", "tramadol", "opioïde faible", "Naloxone"),
    ("Skenan", "morphine", "opioïde", "Naloxone"),
    ("Oxycontin", "oxycodone", "opioïde", "Naloxone"),
    ("Durogesic", "fentanyl", "opioïde", "Naloxone"),
    // Benzodiazépines / hypnotiques
    ("Xanax", "alprazolam", "benzodiazépine", "Flumazénil"),
    ("Lexomil", "bromazépam", "benzodiazépine", "Flumazénil"),
    ("Temesta", "lorazépam", "benzodiazépine", "Flumazénil"),
    ("Valium", "diazépam", "benzodiazépine", "Flumazénil"),
    ("Séresta", "oxazépam", "benzodiazépine", "Flumazénil"),
    ("Stilnox", "zolpidem", "hypnotique", "Flumazénil"),
    ("Imovane", "zopiclone", "hypnotique", "Flumazénil"),
    // Cardiologie
    ("Tahor", "atorvastatine", "statine", ""),
    ("Crestor", "rosuvastatine", "statine", ""),
    ("Coversyl", "périndopril", "IEC", ""),
    ("Triatec", "ramipril", "IEC", ""),
    ("Cozaar", "losartan", "ARA II", ""),
    ("Aprovel", "irbésartan", "ARA II", ""),
    ("Amlor", "amlodipine", "inhibiteur calcique", ""),
    ("Isoptine", "vérapamil", "inhibiteur calcique", ""),
    ("Cordarone", "amiodarone", "antiarythmique", ""),
    ("Cardensiel", "bisoprolol", "bêtabloquant", ""),
    ("Ténormine", "aténolol", "bêtabloquant", ""),
    ("Lasilix", "furosémide", "diurétique de l'anse", ""),
    ("Aldactone", "spironolactone", "diurétique épargneur K+", ""),
    ("Digoxine", "digoxine", "digitalique", "Fab antidigoxine"),
    // Diabète
    ("Glucophage", "metformine", "biguanide", ""),
    ("Diamicron", "gliclazide", "sulfamide hypoglycémiant", ""),
    ("Ozempic", "sémaglutide", "analogue GLP-1", ""),
    ("Lantus", "insuline glargine", "insuline", ""),
    // Respiratoire
    ("Ventoline", "salbutamol", "bêta-2 mimétique", ""),
    ("Symbicort", "budésonide + formotérol", "CSI + BDLA", ""),
    ("Seretide", "fluticasone + salmétérol", "CSI + BDLA", ""),
    ("Spiriva", "tiotropium", "anticholinergique", ""),
    ("Singulair", "montélukast", "antileucotriène", ""),
    // Divers courants
    ("Levothyrox", "lévothyroxine", "hormone thyroïdienne", ""),
    ("Inexium", "ésoméprazole", "IPP", ""),
    ("Inipomp", "pantoprazole", "IPP", ""),
    ("Mopral", "oméprazole", "IPP", ""),
    ("Amoxicilline", "amoxicilline", "pénicilline", ""),
    (
        "Augmentin",
        "amoxicilline + acide clavulanique",
        "pénicilline + inhibiteur",
        "",
    ),
    ("Pyostacine", "pristinamycine", "streptogramine", ""),
    ("Cortancyl", "prednisone", "corticoïde", ""),
    ("Solupred", "prednisolone", "corticoïde", ""),
    (
        "Méthotrexate",
        "méthotrexate",
        "immunosuppresseur",
        "Acide folinique",
    ),
    // Anti-infectieux
    ("Zithromax", "azithromycine", "macrolide", ""),
    ("Rovamycine", "spiramycine", "macrolide", ""),
    ("Josacine", "josamycine", "macrolide", ""),
    ("Orelox", "cefpodoxime", "céphalosporine C3G", ""),
    ("Oroken", "céfixime", "céphalosporine C3G", ""),
    ("Ciflox", "ciprofloxacine", "fluoroquinolone", ""),
    ("Oflocet", "ofloxacine", "fluoroquinolone", ""),
    (
        "Monuril",
        "fosfomycine-trométamol",
        "antibiotique urinaire",
        "",
    ),
    (
        "Furadantine",
        "nitrofurantoïne",
        "antibiotique urinaire",
        "",
    ),
    ("Selexid", "pivmécillinam", "pénicilline", ""),
    ("Bactrim", "cotrimoxazole", "sulfamide antibactérien", ""),
    ("Doxycycline", "doxycycline", "cycline", ""),
    ("Flagyl", "métronidazole", "nitro-imidazolé", ""),
    ("Triflucan", "fluconazole", "antifongique azolé", ""),
    ("Zelitrex", "valaciclovir", "antiviral", ""),
    ("Zovirax", "aciclovir", "antiviral", ""),
    // Douleur / inflammation
    (
        "Codoliprane",
        "paracétamol + codéine",
        "antalgique opioïde faible",
        "Naloxone",
    ),
    (
        "Ixprim",
        "paracétamol + tramadol",
        "antalgique opioïde faible",
        "Naloxone",
    ),
    ("Advil", "ibuprofène", "AINS", ""),
    ("Nurofen", "ibuprofène", "AINS", ""),
    ("Voltarène", "diclofénac", "AINS", ""),
    ("Bi-Profénid", "kétoprofène", "AINS", ""),
    ("Celebrex", "célécoxib", "AINS coxib", ""),
    ("Acupan", "néfopam", "antalgique non opioïde", ""),
    // Gastro-entérologie
    ("Gaviscon", "alginate + bicarbonate", "antiacide", ""),
    ("Spasfon", "phloroglucinol", "antispasmodique", ""),
    ("Vogalène", "métopimazine", "antiémétique", ""),
    ("Primpéran", "métoclopramide", "antiémétique", ""),
    ("Zophren", "ondansétron", "sétron antiémétique", ""),
    ("Motilium", "dompéridone", "antiémétique", ""),
    ("Imodium", "lopéramide", "antidiarrhéique", ""),
    ("Smecta", "diosmectite", "antidiarrhéique", ""),
    ("Forlax", "macrogol", "laxatif osmotique", ""),
    ("Duphalac", "lactulose", "laxatif osmotique", ""),
    // Allergie / ORL
    ("Aerius", "desloratadine", "antihistaminique H1", ""),
    ("Zyrtec", "cétirizine", "antihistaminique H1", ""),
    ("Clarityne", "loratadine", "antihistaminique H1", ""),
    ("Atarax", "hydroxyzine", "antihistaminique H1 sédatif", ""),
    // Psychiatrie
    ("Seroplex", "escitalopram", "ISRS", ""),
    ("Prozac", "fluoxétine", "ISRS", ""),
    ("Zoloft", "sertraline", "ISRS", ""),
    ("Deroxat", "paroxétine", "ISRS", ""),
    ("Effexor", "venlafaxine", "IRSNa", ""),
    ("Cymbalta", "duloxétine", "IRSNa", ""),
    ("Laroxyl", "amitriptyline", "antidépresseur tricyclique", ""),
    ("Norset", "mirtazapine", "antidépresseur", ""),
    ("Téralithe", "lithium", "thymorégulateur", ""),
    // Neurologie
    ("Dépakote", "divalproate de sodium", "thymorégulateur", ""),
    ("Lamictal", "lamotrigine", "antiépileptique", ""),
    ("Keppra", "lévétiracétam", "antiépileptique", ""),
    ("Tégrétol", "carbamazépine", "antiépileptique", ""),
    (
        "Neurontin",
        "gabapentine",
        "antiépileptique / douleur neuropathique",
        "",
    ),
    (
        "Lyrica",
        "prégabaline",
        "antiépileptique / douleur neuropathique",
        "",
    ),
    ("Imigrane", "sumatriptan", "triptan", ""),
    // Cardio / métabolisme
    ("Ezetrol", "ézétimibe", "hypolipémiant", ""),
    (
        "Esidrex",
        "hydrochlorothiazide",
        "diurétique thiazidique",
        "",
    ),
    (
        "Fludex",
        "indapamide",
        "diurétique apparenté thiazidique",
        "",
    ),
    ("Avlocardyl", "propranolol", "bêtabloquant", ""),
    ("Flécaïne", "flécaïnide", "antiarythmique", ""),
    (
        "Kaléorid",
        "chlorure de potassium",
        "supplément potassique",
        "",
    ),
    ("Januvia", "sitagliptine", "iDPP-4", ""),
    ("Forxiga", "dapagliflozine", "iSGLT2", ""),
    ("Jardiance", "empagliflozine", "iSGLT2", ""),
    ("Trulicity", "dulaglutide", "analogue GLP-1", ""),
    ("Victoza", "liraglutide", "analogue GLP-1", ""),
    ("NovoRapid", "insuline asparte", "insuline rapide", ""),
    // Divers courants
    ("Zyloric", "allopurinol", "hypo-uricémiant", ""),
    ("Adenuric", "fébuxostat", "hypo-uricémiant", ""),
    ("Colchicine", "colchicine", "anti-goutteux", ""),
    ("Fosamax", "alendronate", "bisphosphonate", ""),
    ("Uvedose", "cholécalciférol", "vitamine D", ""),
    ("Tardyferon", "sulfate ferreux", "fer", ""),
    ("Spéciafoldine", "acide folique", "vitamine B9", ""),
    ("Bricanyl", "terbutaline", "bêta-2 mimétique", ""),
    ("Pulmicort", "budésonide", "corticoïde inhalé", ""),
    ("Tanganil", "acétylleucine", "anti-vertigineux", ""),
    ("Circadin", "mélatonine", "mélatonine", ""),
    // Anticancéreux oraux (accompagnement officinal)
    (
        "Xeloda",
        "capécitabine",
        "anticancéreux oral — fluoropyrimidine",
        "",
    ),
    ("Glivec", "imatinib", "anticancéreux oral — ITK", ""),
    (
        "Ibrance",
        "palbociclib",
        "anticancéreux oral — inhibiteur CDK4/6",
        "",
    ),
    (
        "Verzenios",
        "abémaciclib",
        "anticancéreux oral — inhibiteur CDK4/6",
        "",
    ),
    (
        "Zytiga",
        "abiratérone",
        "anticancéreux oral — hormonothérapie",
        "",
    ),
    (
        "Xtandi",
        "enzalutamide",
        "anticancéreux oral — hormonothérapie",
        "",
    ),
    ("Tamoxifène", "tamoxifène", "hormonothérapie — SERM", ""),
    (
        "Fémara",
        "létrozole",
        "hormonothérapie — anti-aromatase",
        "",
    ),
    (
        "Arimidex",
        "anastrozole",
        "hormonothérapie — anti-aromatase",
        "",
    ),
    (
        "Aromasine",
        "exémestane",
        "hormonothérapie — anti-aromatase",
        "",
    ),
    (
        "Imnovid",
        "pomalidomide",
        "anticancéreux oral — immunomodulateur",
        "",
    ),
    (
        "Revlimid",
        "lénalidomide",
        "anticancéreux oral — immunomodulateur",
        "",
    ),
    // HBPM / anticoagulants complémentaires
    ("Innohep", "tinzaparine", "HBPM", "Protamine (partiel)"),
    ("Fraxiparine", "nadroparine", "HBPM", "Protamine (partiel)"),
    ("Fragmine", "daltéparine", "HBPM", "Protamine (partiel)"),
    ("Arixtra", "fondaparinux", "anti-Xa injectable", ""),
    // Cardio complémentaires
    (
        "Entresto",
        "sacubitril + valsartan",
        "IEC/ARA2 — insuffisance cardiaque",
        "",
    ),
    ("Procoralan", "ivabradine", "inhibiteur du courant If", ""),
    ("Loxen", "nicardipine", "inhibiteur calcique", ""),
    (
        "Eupressyl",
        "urapidil",
        "alpha-bloquant antihypertenseur",
        "",
    ),
    ("Zocor", "simvastatine", "statine", ""),
    ("Elisor", "pravastatine", "statine", ""),
    ("Praluent", "alirocumab", "anti-PCSK9", ""),
    ("Trinitrine", "trinitrine", "dérivé nitré", ""),
    // Diabète complémentaires
    ("Toujeo", "insuline glargine 300", "insuline lente", ""),
    ("Abasaglar", "insuline glargine", "insuline lente", ""),
    ("Humalog", "insuline lispro", "insuline rapide", ""),
    ("Amarel", "glimépiride", "sulfamide hypoglycémiant", ""),
    // Parkinson
    (
        "Modopar",
        "lévodopa + bensérazide",
        "antiparkinsonien — L-dopa",
        "",
    ),
    (
        "Sinemet",
        "lévodopa + carbidopa",
        "antiparkinsonien — L-dopa",
        "",
    ),
    ("Sifrol", "pramipexole", "agoniste dopaminergique", ""),
    ("Requip", "ropinirole", "agoniste dopaminergique", ""),
    ("Azilect", "rasagiline", "IMAO-B", ""),
    // Psychiatrie complémentaires
    ("Abilify", "aripiprazole", "antipsychotique atypique", ""),
    ("Zyprexa", "olanzapine", "antipsychotique atypique", ""),
    ("Risperdal", "rispéridone", "antipsychotique atypique", ""),
    (
        "Leponex",
        "clozapine",
        "antipsychotique — NFS obligatoire",
        "",
    ),
    ("Haldol", "halopéridol", "antipsychotique typique", ""),
    ("Lysanxia", "prazépam", "benzodiazépine", "Flumazénil"),
    ("Urbanyl", "clobazam", "benzodiazépine", "Flumazénil"),
    (
        "Mianserine",
        "miansérine",
        "antidépresseur tétracyclique",
        "",
    ),
    // Antibiotiques / anti-infectieux complémentaires
    ("Tavanic", "lévofloxacine", "fluoroquinolone", ""),
    ("Fucidine", "acide fusidique", "antibiotique local", ""),
    // Uro / gynéco
    ("Josir", "tamsulosine", "alpha-bloquant — HBP", ""),
    ("Xatral", "alfuzosine", "alpha-bloquant — HBP", ""),
    ("Avodart", "dutastéride", "inhibiteur 5-alpha-réductase", ""),
    ("Vesicare", "solifénacine", "anticholinergique vésical", ""),
    (
        "Optimizette",
        "désogestrel",
        "contraception microprogestative",
        "",
    ),
    (
        "Leeloo",
        "lévonorgestrel + éthinylestradiol",
        "contraception estroprogestative",
        "",
    ),
    ("Norlevo", "lévonorgestrel", "contraception d'urgence", ""),
    // Os / rhumato
    ("Prolia", "dénosumab", "anti-RANKL (semestriel)", ""),
    ("Actonel", "risédronate", "biphosphonate", ""),
    ("Cacit", "carbonate de calcium", "calcium", ""),
    ("Arava", "léflunomide", "immunomodulateur — DMARD", ""),
    ("Imurel", "azathioprine", "immunosuppresseur", ""),
    (
        "Plaquenil",
        "hydroxychloroquine",
        "antipaludéen de synthèse",
        "",
    ),
    // Divers comptoir
    ("Roaccutane", "isotrétinoïne", "rétinoïde — tératogène", ""),
    (
        "Néo-Mercazole",
        "carbimazole",
        "antithyroïdien de synthèse",
        "",
    ),
    ("Monoprost", "latanoprost", "collyre — prostaglandine", ""),
    (
        "Lamaline",
        "paracétamol + opium + caféine",
        "antalgique opiacé",
        "",
    ),
    ("Célestène", "bétaméthasone", "corticoïde", ""),
    ("Médrol", "méthylprednisolone", "corticoïde", ""),
    ("Ercéfuryl", "nifuroxazide", "antiseptique intestinal", ""),
    ("Débridat", "trimébutine", "antispasmodique", ""),
    ("Magné B6", "magnésium + vitamine B6", "magnésium", ""),
    // Anticancéreux oraux (suite)
    (
        "Tagrisso",
        "osimertinib",
        "anticancéreux oral — ITK EGFR",
        "",
    ),
    ("Tarceva", "erlotinib", "anticancéreux oral — ITK EGFR", ""),
    (
        "Sutent",
        "sunitinib",
        "anticancéreux oral — ITK multicible",
        "",
    ),
    (
        "Nexavar",
        "sorafénib",
        "anticancéreux oral — ITK multicible",
        "",
    ),
    (
        "Sprycel",
        "dasatinib",
        "anticancéreux oral — ITK BCR-ABL",
        "",
    ),
    (
        "Tasigna",
        "nilotinib",
        "anticancéreux oral — ITK BCR-ABL",
        "",
    ),
    (
        "Lynparza",
        "olaparib",
        "anticancéreux oral — inhibiteur PARP",
        "",
    ),
    (
        "Temodal",
        "témozolomide",
        "anticancéreux oral — alkylant",
        "",
    ),
    (
        "Hydréa",
        "hydroxycarbamide",
        "anticancéreux oral — antimétabolite",
        "",
    ),
    // Cardiologie (suite)
    ("Efient", "prasugrel", "antiagrégant", ""),
    ("Inspra", "éplérénone", "antialdostérone", ""),
    ("Sotalex", "sotalol", "bêta-bloquant antiarythmique", ""),
    ("Sectral", "acébutolol", "bêta-bloquant", ""),
    ("Multaq", "dronédarone", "antiarythmique", ""),
    (
        "Adancor",
        "nicorandil",
        "activateur des canaux potassiques",
        "",
    ),
    ("Vastarel", "trimétazidine", "antiangineux métabolique", ""),
    ("Kaleorid", "chlorure de potassium", "potassium", ""),
    // Pneumologie (suite)
    (
        "Foster",
        "béclométasone + formotérol",
        "corticoïde inhalé + BALA",
        "",
    ),
    (
        "Trelegy Ellipta",
        "fluticasone + uméclidinium + vilantérol",
        "trithérapie inhalée",
        "",
    ),
    ("Atrovent", "ipratropium", "anticholinergique inhalé", ""),
    ("Serevent", "salmétérol", "bêta-2 de longue durée", ""),
    (
        "Flixotide",
        "fluticasone propionate",
        "corticoïde inhalé",
        "",
    ),
    // Diabète / endocrinologie (suite)
    ("Mounjaro", "tirzépatide", "agoniste GIP/GLP-1", ""),
    ("Tresiba", "insuline dégludec", "insuline ultralente", ""),
    ("Levemir", "insuline détémir", "insuline lente", ""),
    ("Apidra", "insuline glulisine", "insuline rapide", ""),
    ("Insulatard", "insuline NPH", "insuline intermédiaire", ""),
    (
        "Hydrocortisone",
        "hydrocortisone",
        "corticoïde substitutif",
        "",
    ),
    // Neurologie / psychiatrie (suite)
    ("Epitomax", "topiramate", "antiépileptique", ""),
    ("Vimpat", "lacosamide", "antiépileptique", ""),
    (
        "Rivotril",
        "clonazépam",
        "benzodiazépine antiépileptique",
        "Flumazénil",
    ),
    ("Xeroquel", "quétiapine", "antipsychotique atypique", ""),
    ("Solian", "amisulpride", "antipsychotique", ""),
    (
        "Anafranil",
        "clomipramine",
        "antidépresseur tricyclique",
        "",
    ),
    ("Aricept", "donépézil", "anticholinestérasique", ""),
    ("Ebixa", "mémantine", "antiglutamate — Alzheimer", ""),
    (
        "Stresam",
        "étifoxine",
        "anxiolytique non benzodiazépinique",
        "",
    ),
    // Gastro-entérologie (suite)
    ("Créon", "pancréatine", "enzymes pancréatiques", ""),
    ("Pentasa", "mésalazine", "aminosalicylé — MICI", ""),
    (
        "Entocort",
        "budésonide oral",
        "corticoïde à action locale",
        "",
    ),
    ("Ursolvan", "acide ursodésoxycholique", "acide biliaire", ""),
    (
        "Questran",
        "colestyramine",
        "chélateur des acides biliaires",
        "",
    ),
    (
        "Movicol",
        "macrogol + électrolytes",
        "laxatif osmotique",
        "",
    ),
    ("Buscopan", "butylscopolamine", "antispasmodique", ""),
    // Urologie / gynécologie (suite)
    ("Cialis", "tadalafil", "inhibiteur PDE5", ""),
    ("Viagra", "sildénafil", "inhibiteur PDE5", ""),
    ("Ditropan", "oxybutynine", "anticholinergique vésical", ""),
    ("Décapeptyl", "triptoréline", "analogue de la GnRH", ""),
    // Dermatologie
    (
        "Diprosone",
        "bétaméthasone dipropionate",
        "dermocorticoïde fort",
        "",
    ),
    ("Dermoval", "clobétasol", "dermocorticoïde très fort", ""),
    (
        "Locoid",
        "hydrocortisone butyrate",
        "dermocorticoïde modéré",
        "",
    ),
    ("Locapred", "désonide", "dermocorticoïde modéré", ""),
    (
        "Daivobet",
        "calcipotriol + bétaméthasone",
        "antipsoriasique topique",
        "",
    ),
    ("Lamisil", "terbinafine", "antifongique", ""),
    ("Pevaryl", "éconazole", "antifongique topique", ""),
    ("Mycoster", "ciclopirox", "antifongique topique", ""),
    (
        "Skinoren",
        "acide azélaïque",
        "topique — acné / rosacée",
        "",
    ),
    // ORL / voies respiratoires hautes
    ("Fluimucil", "acétylcystéine", "mucolytique", ""),
    ("Rhinathiol", "carbocistéine", "mucolytique", ""),
    ("Toplexil", "oxomémazine", "antitussif antihistaminique", ""),
    (
        "Rhinofluimucil",
        "acétylcystéine + tuaminoheptane",
        "décongestionnant local",
        "",
    ),
    ("Pivalone", "tixocortol", "corticoïde nasal", ""),
    ("Nasonex", "mométasone", "corticoïde nasal", ""),
    // Ophtalmologie
    (
        "Cosopt",
        "dorzolamide + timolol",
        "collyre antiglaucomateux",
        "",
    ),
    (
        "Azopt",
        "brinzolamide",
        "collyre — inhibiteur anhydrase carbonique",
        "",
    ),
    ("Timoptol", "timolol", "collyre bêta-bloquant", ""),
    // Anti-infectieux (suite)
    ("Tamiflu", "oseltamivir", "antiviral — grippe", ""),
    (
        "Truvada",
        "emtricitabine + ténofovir",
        "antirétroviral / PrEP",
        "",
    ),
    (
        "Biktarvy",
        "bictégravir + emtricitabine + ténofovir",
        "antirétroviral",
        "",
    ),
    ("Baraclude", "entécavir", "antiviral — hépatite B", ""),
    ("Vermox", "mébendazole", "antiparasitaire", ""),
    ("Fluvermal", "flubendazole", "antiparasitaire", ""),
    ("Stromectol", "ivermectine", "antiparasitaire", ""),
    // Rhumatologie / immunologie (suite)
    ("Humira", "adalimumab", "anti-TNF alpha", ""),
    ("Enbrel", "étanercept", "anti-TNF alpha", ""),
    ("Salazopyrine", "sulfasalazine", "DMARD", ""),
];

pub struct Db {
    conn: Connection,
}

pub fn default_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("bpm-caddy")
        .join("bpm_caddy.db")
}

impl Db {
    /// Open (or create) the encrypted database. Fails with a French message
    /// if the password does not match an existing file.
    pub fn open(path: &Path, password: &str) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("dossier de données inaccessible : {e}"))?;
        }
        let conn = Connection::open(path).map_err(|e| format!("ouverture impossible : {e}"))?;
        // The database may sit on a shared network drive with several
        // PCs writing: wait for a colleague's transaction instead of
        // failing immediately with "database is locked".
        conn.busy_timeout(std::time::Duration::from_secs(5))
            .map_err(|e| format!("configuration impossible : {e}"))?;
        conn.pragma_update(None, "key", password)
            .map_err(|e| format!("configuration du chiffrement impossible : {e}"))?;
        // Probing the schema is how SQLCipher reports a wrong key.
        conn.query_row("SELECT count(*) FROM sqlite_master", [], |r| {
            r.get::<_, i64>(0)
        })
        .map_err(|_| "Mot de passe incorrect (ou fichier illisible).".to_owned())?;
        conn.execute_batch(SCHEMA)
            .map_err(|e| format!("initialisation du schéma impossible : {e}"))?;
        for migration in MIGRATIONS {
            // Fails harmlessly when the column already exists.
            let _ = conn.execute(migration, []);
        }
        Ok(Self { conn })
    }

    pub fn patients(&self) -> Result<Vec<Patient>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, last_name, first_name, birth_date, phone, notes,
                        physician, email, address
                 FROM patients",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok(Patient {
                    id: r.get(0)?,
                    last_name: r.get(1)?,
                    first_name: r.get(2)?,
                    birth_date: r.get(3)?,
                    phone: r.get(4)?,
                    notes: r.get(5)?,
                    physician: r.get(6)?,
                    email: r.get(7)?,
                    address: r.get(8)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn add_patient(
        &self,
        last_name: &str,
        first_name: &str,
        birth_date: &str,
    ) -> Result<i64, String> {
        self.conn
            .execute(
                "INSERT INTO patients (last_name, first_name, birth_date) VALUES (?1, ?2, ?3)",
                (last_name, first_name, birth_date),
            )
            .map_err(|e| e.to_string())?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Re-encrypt the database under a new master password (SQLCipher
    /// `PRAGMA rekey`). Do this while no other PC has the file open.
    pub fn change_password(&self, new_password: &str) -> Result<(), String> {
        if new_password.is_empty() {
            return Err("Le mot de passe ne peut pas être vide.".to_owned());
        }
        self.conn
            .pragma_update(None, "rekey", new_password)
            .map_err(|e| format!("changement du mot de passe impossible : {e}"))
    }

    /// Write a consistent snapshot of the database to `path` (encrypted
    /// with the same key). `VACUUM INTO` takes the proper locks, so the
    /// copy is safe even while other PCs are writing.
    pub fn backup_to(&self, path: &Path) -> Result<(), String> {
        let path_str = path
            .to_str()
            .ok_or_else(|| "chemin de sauvegarde invalide".to_owned())?;
        self.conn
            .execute("VACUUM INTO ?1", [path_str])
            .map_err(|e| format!("sauvegarde impossible : {e}"))?;
        Ok(())
    }

    /// Correct a patient's identity and contact details. Compare-and-set
    /// against the values this PC last saw (`expected`): a colleague's
    /// concurrent edit is never silently overwritten. Returns `false`
    /// when the row changed under us (the caller should reload).
    pub fn update_patient(&self, new: &Patient, expected: &Patient) -> Result<bool, String> {
        let changed = self
            .conn
            .execute(
                "UPDATE patients SET last_name = ?1, first_name = ?2, birth_date = ?3,
                        phone = ?4, notes = ?5, physician = ?6, email = ?7, address = ?8
                 WHERE id = ?9 AND last_name = ?10 AND first_name = ?11
                   AND birth_date = ?12 AND phone = ?13 AND notes = ?14
                   AND physician = ?15 AND email = ?16 AND address = ?17",
                rusqlite::params![
                    new.last_name,
                    new.first_name,
                    new.birth_date,
                    new.phone,
                    new.notes,
                    new.physician,
                    new.email,
                    new.address,
                    expected.id,
                    expected.last_name,
                    expected.first_name,
                    expected.birth_date,
                    expected.phone,
                    expected.notes,
                    expected.physician,
                    expected.email,
                    expected.address,
                ],
            )
            .map_err(|e| e.to_string())?;
        Ok(changed == 1)
    }

    /// The patient's current treatments, joined from the drug base.
    pub fn drugs_for_patient(&self, patient_id: i64) -> Result<Vec<Drug>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT d.id, d.name, d.dci, d.class, d.dosage, d.ddi, d.iup, d.antidote,
                        d.notes, d.half_life, d.auc, d.elimination, d.renal, d.pregnancy,
                        d.indications, d.mechanism, d.contraindications, d.adverse,
                        d.monitoring, d.sources, d.status, d.smr, d.tags, d.toxicity
                 FROM patient_drugs pd JOIN drugs d ON d.id = pd.drug_id
                 WHERE pd.patient_id = ?1 ORDER BY d.name COLLATE NOCASE",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([patient_id], |r| {
                Ok(Drug {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    dci: r.get(2)?,
                    class: r.get(3)?,
                    dosage: r.get(4)?,
                    ddi: r.get(5)?,
                    iup: r.get(6)?,
                    antidote: r.get(7)?,
                    notes: r.get(8)?,
                    half_life: r.get(9)?,
                    auc: r.get(10)?,
                    elimination: r.get(11)?,
                    renal: r.get(12)?,
                    pregnancy: r.get(13)?,
                    indications: r.get(14)?,
                    mechanism: r.get(15)?,
                    contraindications: r.get(16)?,
                    adverse: r.get(17)?,
                    monitoring: r.get(18)?,
                    sources: r.get(19)?,
                    status: r.get(20)?,
                    smr: r.get(21)?,
                    tags: r.get(22)?,
                    toxicity: r.get(23)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
    }

    /// Patients currently on a given drug — the recall / alert question
    /// ("qui est sous Eliquis ?").
    pub fn patients_for_drug(&self, drug_id: i64) -> Result<Vec<Patient>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT p.id, p.last_name, p.first_name, p.birth_date, p.phone, p.notes,
                        p.physician, p.email, p.address
                 FROM patient_drugs pd JOIN patients p ON p.id = pd.patient_id
                 WHERE pd.drug_id = ?1
                 ORDER BY p.last_name COLLATE NOCASE, p.first_name COLLATE NOCASE",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([drug_id], |r| {
                Ok(Patient {
                    id: r.get(0)?,
                    last_name: r.get(1)?,
                    first_name: r.get(2)?,
                    birth_date: r.get(3)?,
                    phone: r.get(4)?,
                    notes: r.get(5)?,
                    physician: r.get(6)?,
                    email: r.get(7)?,
                    address: r.get(8)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
    }

    /// Append a note. For [`NoteSubject::Operator`], `subject_id` is 0
    /// and the operator string itself is the key.
    pub fn add_note(
        &self,
        subject: NoteSubject,
        subject_id: i64,
        operator: &str,
        body: &str,
    ) -> Result<i64, String> {
        self.conn
            .execute(
                "INSERT INTO notes (subject_kind, subject_id, operator, body)
                 VALUES (?1, ?2, ?3, ?4)",
                (subject.as_str(), subject_id, operator, body),
            )
            .map_err(|e| e.to_string())?;
        Ok(self.conn.last_insert_rowid())
    }

    /// The notes of a patient or drug, newest first.
    pub fn notes_for(&self, subject: NoteSubject, subject_id: i64) -> Result<Vec<Note>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, operator, body, created_at FROM notes
                 WHERE subject_kind = ?1 AND subject_id = ?2
                 ORDER BY created_at DESC, id DESC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map((subject.as_str(), subject_id), |r| {
                Ok(Note {
                    id: r.get(0)?,
                    operator: r.get(1)?,
                    body: r.get(2)?,
                    created_at: r.get(3)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
    }

    /// The personal notes of one operator, newest first.
    pub fn notes_for_operator(&self, operator: &str) -> Result<Vec<Note>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, operator, body, created_at FROM notes
                 WHERE subject_kind = 'OPERATOR' AND operator = ?1
                 ORDER BY created_at DESC, id DESC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([operator], |r| {
                Ok(Note {
                    id: r.get(0)?,
                    operator: r.get(1)?,
                    body: r.get(2)?,
                    created_at: r.get(3)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
    }

    /// One day of the transmission logbook, chronological.
    pub fn transmissions_for_day(&self, day_iso: &str) -> Result<Vec<Note>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, operator, body, created_at FROM notes
                 WHERE subject_kind = 'TRANSMISSION' AND substr(created_at, 1, 10) = ?1
                 ORDER BY created_at, id",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([day_iso], |r| {
                Ok(Note {
                    id: r.get(0)?,
                    operator: r.get(1)?,
                    body: r.get(2)?,
                    created_at: r.get(3)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
    }

    /// The days that have transmission entries, newest first.
    pub fn transmission_days(&self) -> Result<Vec<String>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT DISTINCT substr(created_at, 1, 10) FROM notes
                 WHERE subject_kind = 'TRANSMISSION'
                 ORDER BY 1 DESC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
    }

    /// An ISO date shifted by whole days (SQLite's calendar).
    pub fn date_offset(&self, iso: &str, days: i64) -> Result<String, String> {
        self.conn
            .query_row(
                "SELECT date(?1, printf('%+d days', ?2))",
                (iso, days),
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())
    }

    pub fn delete_note(&self, id: i64) -> Result<(), String> {
        self.conn
            .execute("DELETE FROM notes WHERE id = ?1", [id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Link a drug to a patient's current treatments (idempotent).
    pub fn add_patient_drug(&self, patient_id: i64, drug_id: i64) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT OR IGNORE INTO patient_drugs (patient_id, drug_id) VALUES (?1, ?2)",
                (patient_id, drug_id),
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn remove_patient_drug(&self, patient_id: i64, drug_id: i64) -> Result<(), String> {
        self.conn
            .execute(
                "DELETE FROM patient_drugs WHERE patient_id = ?1 AND drug_id = ?2",
                (patient_id, drug_id),
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Remove a patient and everything attached to them, atomically.
    pub fn delete_patient(&self, id: i64) -> Result<(), String> {
        let tx = self
            .conn
            .unchecked_transaction()
            .map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM interviews WHERE patient_id = ?1", [id])
            .map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM patient_drugs WHERE patient_id = ?1", [id])
            .map_err(|e| e.to_string())?;
        tx.execute(
            "DELETE FROM notes WHERE subject_kind = 'PATIENT' AND subject_id = ?1",
            [id],
        )
        .map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM patients WHERE id = ?1", [id])
            .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())
    }

    pub fn interviews_for(&self, patient_id: i64) -> Result<Vec<Interview>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, kind, state, duration_minutes, scheduled_date, theme, created_at
                 FROM interviews WHERE patient_id = ?1 ORDER BY created_at DESC, id DESC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([patient_id], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, i64>(3)?,
                    r.get::<_, Option<String>>(4)?,
                    r.get::<_, String>(5)?,
                    r.get::<_, String>(6)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for row in rows {
            let (id, kind, state, duration_minutes, scheduled_date, theme, created_at) =
                row.map_err(|e| e.to_string())?;
            out.push(Interview {
                id,
                duration_minutes,
                scheduled_date,
                theme,
                kind: InterviewKind::parse(&kind)
                    .ok_or_else(|| format!("type d'entretien inconnu : {kind}"))?,
                state: InterviewState::parse(&state)
                    .ok_or_else(|| format!("état d'entretien inconnu : {state}"))?,
                created_at,
            });
        }
        Ok(out)
    }

    #[cfg(test)]
    pub fn add_interview(&self, patient_id: i64, kind: InterviewKind) -> Result<i64, String> {
        self.add_interview_themed(patient_id, kind, "")
    }

    /// Create an interview with its thematic already chosen (the quick
    /// picker sets both at once).
    pub fn add_interview_themed(
        &self,
        patient_id: i64,
        kind: InterviewKind,
        theme: &str,
    ) -> Result<i64, String> {
        self.conn
            .execute(
                "INSERT INTO interviews (patient_id, kind, theme) VALUES (?1, ?2, ?3)",
                (patient_id, kind.as_str(), theme),
            )
            .map_err(|e| e.to_string())?;
        Ok(self.conn.last_insert_rowid())
    }

    /// The drug reference base, alphabetical.
    pub fn drugs(&self) -> Result<Vec<Drug>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, name, dci, class, dosage, ddi, iup, antidote, notes,
                        half_life, auc, elimination, renal, pregnancy,
                        indications, mechanism, contraindications, adverse,
                        monitoring, sources, status, smr, tags, toxicity
                 FROM drugs ORDER BY name COLLATE NOCASE",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok(Drug {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    dci: r.get(2)?,
                    class: r.get(3)?,
                    dosage: r.get(4)?,
                    ddi: r.get(5)?,
                    iup: r.get(6)?,
                    antidote: r.get(7)?,
                    notes: r.get(8)?,
                    half_life: r.get(9)?,
                    auc: r.get(10)?,
                    elimination: r.get(11)?,
                    renal: r.get(12)?,
                    pregnancy: r.get(13)?,
                    indications: r.get(14)?,
                    mechanism: r.get(15)?,
                    contraindications: r.get(16)?,
                    adverse: r.get(17)?,
                    monitoring: r.get(18)?,
                    sources: r.get(19)?,
                    status: r.get(20)?,
                    smr: r.get(21)?,
                    tags: r.get(22)?,
                    toxicity: r.get(23)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
    }

    /// Populate a brand-new drug base with common French brand names and
    /// their DCI, plus the textbook antidotes. Dosage, interactions and
    /// IUP are deliberately left empty for the team to fill from the
    /// references they trust. No-op once the base has any content, and
    /// never resurrects a deleted card.
    pub fn seed_drugs_if_empty(&self) -> Result<usize, String> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM drugs", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        if count > 0 {
            return Ok(0);
        }
        let tx = self
            .conn
            .unchecked_transaction()
            .map_err(|e| e.to_string())?;
        let mut inserted = 0;
        for (name, dci, class, antidote) in STARTER_DRUGS {
            inserted += tx
                .execute(
                    "INSERT INTO drugs (name, dci, class, antidote)
                     SELECT ?1, ?2, ?3, ?4
                     WHERE NOT EXISTS (SELECT 1 FROM drugs WHERE name = ?1)",
                    (name, dci, class, antidote),
                )
                .map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())?;
        self.fill_starter_details()?;
        Ok(inserted)
    }

    /// Fill the clinical fields of the detailed starter cards, without
    /// ever touching a field the team has already written: each column
    /// is only set where it is still empty. Returns how many fields
    /// were filled, so the maintenance button can report it.
    pub fn fill_starter_details(&self) -> Result<usize, String> {
        let tx = self
            .conn
            .unchecked_transaction()
            .map_err(|e| e.to_string())?;
        let mut filled = 0;
        for d in STARTER_DETAILS {
            for (column, value) in [
                ("indications", d.indications),
                ("mechanism", d.mechanism),
                ("dosage", d.dosage),
                ("contraindications", d.contraindications),
                ("ddi", d.ddi),
                ("adverse", d.adverse),
                ("monitoring", d.monitoring),
                ("iup", d.iup),
                ("half_life", d.half_life),
                ("elimination", d.elimination),
                ("renal", d.renal),
                ("pregnancy", d.pregnancy),
                ("sources", d.sources),
                ("status", d.status),
                ("smr", d.smr),
                ("tags", d.tags),
                ("toxicity", d.toxicity),
            ] {
                if value.is_empty() {
                    continue;
                }
                filled += tx
                    .execute(
                        // The column name comes from the literal list
                        // above, never from user input.
                        &format!(
                            "UPDATE drugs SET {column} = ?1 WHERE name = ?2 AND {column} = ''"
                        ),
                        (value, d.name),
                    )
                    .map_err(|e| e.to_string())?;
            }
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok(filled)
    }

    /// Insert every starter drug the base does not already have (by
    /// brand name) — completes a base created before the starter list
    /// existed or grew. Returns how many were added.
    pub fn seed_missing_drugs(&self) -> Result<usize, String> {
        let tx = self
            .conn
            .unchecked_transaction()
            .map_err(|e| e.to_string())?;
        let mut inserted = 0;
        for (name, dci, class, antidote) in STARTER_DRUGS {
            inserted += tx
                .execute(
                    "INSERT INTO drugs (name, dci, class, antidote)
                     SELECT ?1, ?2, ?3, ?4
                     WHERE NOT EXISTS (SELECT 1 FROM drugs WHERE name = ?1)",
                    (name, dci, class, antidote),
                )
                .map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())?;
        self.fill_starter_details()?;
        Ok(inserted)
    }

    /// Debug/demo helper: erase every row of every table, then reseed
    /// the starter drugs. The schema, password and file stay unchanged.
    pub fn reset_all_data(&self) -> Result<(), String> {
        let tx = self
            .conn
            .unchecked_transaction()
            .map_err(|e| e.to_string())?;
        for table in ["notes", "patient_drugs", "interviews", "patients", "drugs"] {
            tx.execute(&format!("DELETE FROM {table}"), [])
                .map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())?;
        self.seed_drugs_if_empty()?;
        Ok(())
    }

    pub fn add_drug(&self, name: &str) -> Result<i64, String> {
        self.conn
            .execute("INSERT INTO drugs (name) VALUES (?1)", [name])
            .map_err(|e| e.to_string())?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Update a drug card. Compare-and-set against the card as loaded,
    /// like every other shared-row write. Returns `false` when stale.
    pub fn update_drug(&self, new: &Drug, expected: &Drug) -> Result<bool, String> {
        let changed = self
            .conn
            .execute(
                "UPDATE drugs SET name = ?1, dci = ?2, class = ?3, dosage = ?4, ddi = ?5,
                        iup = ?6, antidote = ?7, notes = ?8, half_life = ?9, auc = ?10,
                        elimination = ?11, renal = ?12, pregnancy = ?13, indications = ?14,
                        mechanism = ?15, contraindications = ?16, adverse = ?17,
                        monitoring = ?18, sources = ?19, status = ?20, smr = ?21,
                        tags = ?22, toxicity = ?23
                 WHERE id = ?24 AND name = ?25 AND dci = ?26 AND class = ?27 AND dosage = ?28
                   AND ddi = ?29 AND iup = ?30 AND antidote = ?31 AND notes = ?32
                   AND half_life = ?33 AND auc = ?34 AND elimination = ?35 AND renal = ?36
                   AND pregnancy = ?37 AND indications = ?38 AND mechanism = ?39
                   AND contraindications = ?40 AND adverse = ?41 AND monitoring = ?42
                   AND sources = ?43 AND status = ?44 AND smr = ?45 AND tags = ?46
                   AND toxicity = ?47",
                rusqlite::params![
                    new.name,
                    new.dci,
                    new.class,
                    new.dosage,
                    new.ddi,
                    new.iup,
                    new.antidote,
                    new.notes,
                    new.half_life,
                    new.auc,
                    new.elimination,
                    new.renal,
                    new.pregnancy,
                    new.indications,
                    new.mechanism,
                    new.contraindications,
                    new.adverse,
                    new.monitoring,
                    new.sources,
                    new.status,
                    new.smr,
                    new.tags,
                    new.toxicity,
                    expected.id,
                    expected.name,
                    expected.dci,
                    expected.class,
                    expected.dosage,
                    expected.ddi,
                    expected.iup,
                    expected.antidote,
                    expected.notes,
                    expected.half_life,
                    expected.auc,
                    expected.elimination,
                    expected.renal,
                    expected.pregnancy,
                    expected.indications,
                    expected.mechanism,
                    expected.contraindications,
                    expected.adverse,
                    expected.monitoring,
                    expected.sources,
                    expected.status,
                    expected.smr,
                    expected.tags,
                    expected.toxicity,
                ],
            )
            .map_err(|e| e.to_string())?;
        Ok(changed == 1)
    }

    /// Remove a drug card; refused (`false`) if it was renamed meanwhile.
    /// Its dated notes go with it, atomically.
    pub fn delete_drug(&self, id: i64, expected_name: &str) -> Result<bool, String> {
        let tx = self
            .conn
            .unchecked_transaction()
            .map_err(|e| e.to_string())?;
        let changed = tx
            .execute(
                "DELETE FROM drugs WHERE id = ?1 AND name = ?2",
                (id, expected_name),
            )
            .map_err(|e| e.to_string())?;
        if changed == 1 {
            tx.execute(
                "DELETE FROM notes WHERE subject_kind = 'DRUG' AND subject_id = ?1",
                [id],
            )
            .map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok(changed == 1)
    }

    /// Today's date as `JJ/MM/AAAA`, from SQLite's clock (local time).
    pub fn today_french(&self) -> Result<String, String> {
        self.conn
            .query_row("SELECT strftime('%d/%m/%Y', 'now', 'localtime')", [], |r| {
                r.get(0)
            })
            .map_err(|e| e.to_string())
    }

    /// Compact local timestamp for note stamps: `JJ/MM HH:MM`.
    pub fn now_stamp(&self) -> Result<String, String> {
        self.conn
            .query_row(
                "SELECT strftime('%d/%m %H:%M', 'now', 'localtime')",
                [],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())
    }

    /// Today's date as ISO `YYYY-MM-DD` (local time), comparable to
    /// `scheduled_date` with plain string ordering.
    pub fn today_iso(&self) -> Result<String, String> {
        self.conn
            .query_row("SELECT date('now', 'localtime')", [], |r| r.get(0))
            .map_err(|e| e.to_string())
    }

    /// The 7 ISO dates (Monday..Sunday) of the current week shifted by
    /// `offset_weeks` — the agenda's week grid.
    pub fn week_dates(&self, offset_weeks: i64) -> Result<Vec<String>, String> {
        // '-6 days' then 'weekday 1' lands on this week's Monday.
        let shift = format!("{} days", offset_weeks * 7);
        let mut out = Vec::with_capacity(7);
        for day in 0..7 {
            let day_shift = format!("{day} days");
            let date: String = self
                .conn
                .query_row(
                    "SELECT date('now', 'localtime', '-6 days', 'weekday 1', ?1, ?2)",
                    (&shift, &day_shift),
                    |r| r.get(0),
                )
                .map_err(|e| e.to_string())?;
            out.push(date);
        }
        Ok(out)
    }

    /// Tomorrow as ISO `YYYY-MM-DD` (local time), for agenda labels.
    pub fn tomorrow_iso(&self) -> Result<String, String> {
        self.conn
            .query_row("SELECT date('now', 'localtime', '+1 day')", [], |r| {
                r.get(0)
            })
            .map_err(|e| e.to_string())
    }

    /// The current year (local time), for expanding shorthand dates.
    /// Falls back to 0 — which fails date validation — rather than erring.
    pub fn current_year(&self) -> u32 {
        self.conn
            .query_row("SELECT strftime('%Y', 'now', 'localtime')", [], |r| {
                r.get::<_, String>(0)
            })
            .ok()
            .and_then(|y| y.parse().ok())
            .unwrap_or(0)
    }

    /// How many not-yet-billed interviews each patient has, keyed by
    /// patient id — the "n en cours" badge in the search results.
    pub fn pending_counts(&self) -> Result<std::collections::HashMap<i64, i64>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT patient_id, COUNT(*) FROM interviews
                 WHERE state != 'BILLED' GROUP BY patient_id",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)))
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
    }

    /// Every interview with its patient, oldest first — the CSV export
    /// for billing reconciliation.
    pub fn export_rows(&self) -> Result<Vec<ExportRow>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT p.first_name || ' ' || p.last_name, p.phone, p.birth_date, i.kind,
                        i.state, substr(i.created_at, 1, 10), i.scheduled_date,
                        i.duration_minutes, i.theme, i.patient_id
                 FROM interviews i JOIN patients p ON p.id = i.patient_id
                 ORDER BY i.created_at, i.id",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, String>(5)?,
                    r.get::<_, Option<String>>(6)?,
                    r.get::<_, i64>(7)?,
                    r.get::<_, String>(8)?,
                    r.get::<_, i64>(9)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        let mut keys = Vec::new();
        for row in rows {
            let (
                patient_name,
                phone,
                birth_date,
                kind,
                state,
                created_date,
                scheduled_date,
                minutes,
                theme,
                patient_id,
            ) = row.map_err(|e| e.to_string())?;
            let kind = InterviewKind::parse(&kind)
                .ok_or_else(|| format!("type d'entretien inconnu : {kind}"))?;
            keys.push((patient_id, kind, created_date.clone()));
            out.push(ExportRow {
                patient_name,
                phone,
                birth_date,
                created_date,
                scheduled_date,
                duration_minutes: minutes,
                theme,
                kind,
                state: InterviewState::parse(&state)
                    .ok_or_else(|| format!("état d'entretien inconnu : {state}"))?,
                fee_rank: 0,
            });
        }
        for (i, rank) in fee_ranks(&keys) {
            out[i].fee_rank = rank;
        }
        Ok(out)
    }

    /// Planned interviews not yet performed, soonest first — the
    /// dashboard's appointment list (overdue ones included, so a missed
    /// RDV is never silently forgotten).
    /// Add an agenda entry that is not a billable act — a formation, a
    /// réunion, a livraison, a congé.
    pub fn add_event(
        &self,
        day: &str,
        title: &str,
        category: EventCategory,
    ) -> Result<i64, String> {
        self.conn
            .execute(
                "INSERT INTO events (day, title, category) VALUES (?1, ?2, ?3)",
                (day, title, category.as_str()),
            )
            .map_err(|e| e.to_string())?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Every agenda entry between two ISO dates, inclusive.
    pub fn events_between(&self, from: &str, to: &str) -> Result<Vec<Event>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, day, title, category FROM events
                 WHERE day >= ?1 AND day <= ?2 ORDER BY day, id",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map((from, to), |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for row in rows {
            let (id, day, title, category) = row.map_err(|e| e.to_string())?;
            out.push(Event {
                id,
                day,
                title,
                category: EventCategory::parse(&category).unwrap_or(EventCategory::Autre),
            });
        }
        Ok(out)
    }

    /// Remove an agenda entry. Compare-and-set on the title this PC
    /// displayed, so a colleague's edited entry is never destroyed.
    pub fn delete_event(&self, id: i64, expected_title: &str) -> Result<bool, String> {
        let changed = self
            .conn
            .execute(
                "DELETE FROM events WHERE id = ?1 AND title = ?2",
                (id, expected_title),
            )
            .map_err(|e| e.to_string())?;
        Ok(changed == 1)
    }

    /// The days of one month (ISO), Monday-aligned grid included: the
    /// returned vector always starts on a Monday and ends on a Sunday.
    pub fn month_grid(&self, offset_months: i64) -> Result<Vec<String>, String> {
        let shift = format!("{offset_months} months");
        let first: String = self
            .conn
            .query_row(
                "SELECT date('now', 'localtime', 'start of month', ?1)",
                [&shift],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        let start: String = self
            .conn
            .query_row("SELECT date(?1, '-6 days', 'weekday 1')", [&first], |r| {
                r.get(0)
            })
            .map_err(|e| e.to_string())?;
        let last: String = self
            .conn
            .query_row(
                "SELECT date(?1, 'start of month', '+1 month', '-1 day')",
                [&first],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        let mut day = start;
        // Six weeks cover every month layout.
        for _ in 0..42 {
            out.push(day.clone());
            if day >= last && weekday_fr(&day) == Some("dimanche") {
                break;
            }
            day = self.date_offset(&day, 1)?;
        }
        Ok(out)
    }

    /// The month a grid offset lands on, as `YYYY-MM`.
    pub fn month_of(&self, offset_months: i64) -> Result<String, String> {
        let shift = format!("{offset_months} months");
        self.conn
            .query_row(
                "SELECT strftime('%Y-%m', date('now', 'localtime', 'start of month', ?1))",
                [&shift],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())
    }

    pub fn upcoming_appointments(&self) -> Result<Vec<Appointment>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT i.patient_id, p.first_name || ' ' || p.last_name, p.phone, i.kind,
                        i.scheduled_date
                 FROM interviews i JOIN patients p ON p.id = i.patient_id
                 WHERE i.scheduled_date IS NOT NULL
                   AND i.state IN ('IDENTIFIED', 'SCHEDULED')
                 ORDER BY i.scheduled_date, i.id",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for row in rows {
            let (patient_id, patient_name, phone, kind, date) = row.map_err(|e| e.to_string())?;
            out.push(Appointment {
                patient_id,
                patient_name,
                phone,
                kind: InterviewKind::parse(&kind)
                    .ok_or_else(|| format!("type d'entretien inconnu : {kind}"))?,
                date,
            });
        }
        Ok(out)
    }

    /// Every interview reduced to what the dashboard needs.
    pub fn interview_summaries(&self) -> Result<Vec<InterviewSummary>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT kind, state, substr(created_at, 1, 7), substr(updated_at, 1, 7),
                        duration_minutes, patient_id, substr(created_at, 1, 10)
                 FROM interviews ORDER BY created_at, id",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, i64>(4)?,
                    r.get::<_, i64>(5)?,
                    r.get::<_, String>(6)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        let mut keys = Vec::new();
        for row in rows {
            let (kind, state, created_month, updated_month, duration_minutes, patient, date) =
                row.map_err(|e| e.to_string())?;
            let kind = InterviewKind::parse(&kind)
                .ok_or_else(|| format!("type d'entretien inconnu : {kind}"))?;
            keys.push((patient, kind, date));
            out.push(InterviewSummary {
                duration_minutes,
                kind,
                state: InterviewState::parse(&state)
                    .ok_or_else(|| format!("état d'entretien inconnu : {state}"))?,
                created_month,
                updated_month,
                fee_rank: 0,
            });
        }
        for (i, rank) in fee_ranks(&keys) {
            out[i].fee_rank = rank;
        }
        Ok(out)
    }

    /// Set the thematic of an interview. Compare-and-set on the theme
    /// this PC saw. Returns `false` when stale.
    pub fn set_theme(&self, id: i64, theme: &str, expected: &str) -> Result<bool, String> {
        let changed = self
            .conn
            .execute(
                "UPDATE interviews SET theme = ?1 WHERE id = ?2 AND theme = ?3",
                (theme, id, expected),
            )
            .map_err(|e| e.to_string())?;
        Ok(changed == 1)
    }

    /// Remove a single interview (added by mistake). Compare-and-set on
    /// the state this PC saw: a row a colleague meanwhile advanced (and
    /// possibly billed) is never destroyed. Returns `false` when stale.
    pub fn delete_interview(&self, id: i64, expected: InterviewState) -> Result<bool, String> {
        let changed = self
            .conn
            .execute(
                "DELETE FROM interviews WHERE id = ?1 AND state = ?2",
                (id, expected.as_str()),
            )
            .map_err(|e| e.to_string())?;
        Ok(changed == 1)
    }

    /// The creation dates (ISO, ascending) of a patient's acts of one
    /// kind — input for the yearly-quota rule.
    pub fn interview_dates_for(
        &self,
        patient_id: i64,
        kind: InterviewKind,
    ) -> Result<Vec<String>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT substr(created_at, 1, 10) FROM interviews
                 WHERE patient_id = ?1 AND kind = ?2
                 ORDER BY created_at",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map((patient_id, kind.as_str()), |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<_, _>>().map_err(|e| e.to_string())
    }

    /// Set (or clear) the planned date of an interview (ISO `YYYY-MM-DD`).
    /// Compare-and-set on the date this PC saw (`IS` also matches NULL),
    /// so a stale field never reverts a colleague's newer date. Returns
    /// `false` when stale.
    pub fn set_scheduled_date(
        &self,
        id: i64,
        date: Option<&str>,
        expected: Option<&str>,
    ) -> Result<bool, String> {
        let changed = self
            .conn
            .execute(
                "UPDATE interviews SET scheduled_date = ?1
                 WHERE id = ?2 AND scheduled_date IS ?3",
                (date, id, expected),
            )
            .map_err(|e| e.to_string())?;
        Ok(changed == 1)
    }

    /// Record the time spent on an interview, for the hourly ROI metric.
    /// Compare-and-set like every other shared-row write (a stale 0 must
    /// not overwrite a colleague's entry). Returns `false` when stale.
    pub fn set_duration(&self, id: i64, minutes: i64, expected: i64) -> Result<bool, String> {
        let changed = self
            .conn
            .execute(
                "UPDATE interviews SET duration_minutes = ?1
                 WHERE id = ?2 AND duration_minutes = ?3",
                (minutes, id, expected),
            )
            .map_err(|e| e.to_string())?;
        Ok(changed == 1)
    }

    /// Advance an interview to the next pipeline state; no-op once billed.
    ///
    /// Compare-and-set: the row is only touched if it is still in the
    /// state this PC saw, so a colleague's concurrent change is never
    /// silently overwritten. Returns `false` when the state was stale
    /// (the caller should reload).
    pub fn advance_interview(&self, id: i64, current: InterviewState) -> Result<bool, String> {
        let Some(next) = current.next() else {
            return Ok(true);
        };
        self.set_state_cas(id, current, next)
    }

    /// Step an interview back to the previous pipeline state (undo of a
    /// misclicked advance); no-op at the first state. Compare-and-set,
    /// like [`Self::advance_interview`].
    pub fn regress_interview(&self, id: i64, current: InterviewState) -> Result<bool, String> {
        let Some(prev) = current.prev() else {
            return Ok(true);
        };
        self.set_state_cas(id, current, prev)
    }

    fn set_state_cas(
        &self,
        id: i64,
        expected: InterviewState,
        new: InterviewState,
    ) -> Result<bool, String> {
        let changed = self
            .conn
            .execute(
                "UPDATE interviews SET state = ?1, updated_at = datetime('now')
                 WHERE id = ?2 AND state = ?3",
                (new.as_str(), id, expected.as_str()),
            )
            .map_err(|e| e.to_string())?;
        Ok(changed == 1)
    }
}

/// Two-digit years are ambiguous ("26": 1926 or 2026?); the caller says
/// which reading makes sense for the field being typed.
#[derive(Clone, Copy)]
pub enum YearHint {
    /// Birth dates: never in the future ("49" is 1949, not 2049).
    Past,
    /// Appointments: always 20xx.
    Future,
}

/// Parse a date typed at the counter into ISO `YYYY-MM-DD`.
///
/// Accepts `JJ/MM/AAAA` (separators `/`, `-` or `.`) and the compact
/// keyboard forms: `JJMMAAAA`, `JJMMAA` ("230826" — two-digit year
/// expanded per `hint`) and `JJMM` / `JJ/MM` (current year). Pass the
/// current year in so the function stays pure and testable.
pub fn parse_french_date(input: &str, current_year: u32, hint: YearHint) -> Result<String, String> {
    let s = input.trim();
    let err = || "Date attendue : JJ/MM/AAAA, JJMMAA ou JJMM".to_owned();
    let expand = |yy: u32| match hint {
        YearHint::Future => 2000 + yy,
        YearHint::Past if 2000 + yy > current_year => 1900 + yy,
        YearHint::Past => 2000 + yy,
    };
    // A yearless form ("2308") means "this year" — sensible for an RDV,
    // but for a birth date it is far more likely a truncated entry
    // ("030746" missing the year) than a newborn: require the year.
    let current_year_ok = || match hint {
        YearHint::Future => Ok(current_year),
        YearHint::Past => Err("Année de naissance requise (JJMMAA ou JJ/MM/AAAA).".to_owned()),
    };
    let day: u32;
    let month: u32;
    let year: u32;
    if !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()) {
        // Digit-only shorthand: byte slicing is safe, everything is ASCII.
        let num = |r: &str| r.parse::<u32>().map_err(|_| err());
        match s.len() {
            4 => {
                day = num(&s[..2])?;
                month = num(&s[2..])?;
                year = current_year_ok()?;
            }
            6 => {
                day = num(&s[..2])?;
                month = num(&s[2..4])?;
                year = expand(num(&s[4..])?);
            }
            8 => {
                day = num(&s[..2])?;
                month = num(&s[2..4])?;
                year = num(&s[4..])?;
            }
            _ => return Err(err()),
        }
    } else {
        let parts: Vec<&str> = s.split(['/', '-', '.']).collect();
        match parts.len() {
            2 => {
                day = parts[0].trim().parse().map_err(|_| err())?;
                month = parts[1].trim().parse().map_err(|_| err())?;
                year = current_year_ok()?;
            }
            3 => {
                day = parts[0].trim().parse().map_err(|_| err())?;
                month = parts[1].trim().parse().map_err(|_| err())?;
                let y = parts[2].trim();
                let parsed: u32 = y.parse().map_err(|_| err())?;
                year = if y.len() == 2 { expand(parsed) } else { parsed };
            }
            _ => return Err(err()),
        }
    }
    if !(1..=12).contains(&month) || !(1900..=2100).contains(&year) {
        return Err(err());
    }
    let leap = year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let days_in_month = match month {
        2 => {
            if leap {
                29
            } else {
                28
            }
        }
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    if !(1..=days_in_month).contains(&day) {
        return Err(err());
    }
    Ok(format!("{year:04}-{month:02}-{day:02}"))
}

/// The same ISO date one year later ("2026-02-29" clamps to the 28th).
/// The same ISO date `months` later, clamping 29 February and the
/// short months to the last valid day.
pub fn add_months(iso: &str, months: u32) -> String {
    if months == 12 {
        return add_one_year(iso);
    }
    let mut parts = iso.split('-');
    let (y, m, d) = match (parts.next(), parts.next(), parts.next()) {
        (Some(y), Some(m), Some(d)) => (y, m, d),
        _ => return iso.to_owned(),
    };
    let (year, month): (i32, u32) = match (y.parse(), m.parse()) {
        (Ok(y), Ok(m)) => (y, m),
        _ => return iso.to_owned(),
    };
    let total = month as i64 - 1 + months as i64;
    let year = year + (total / 12) as i32;
    let month = (total % 12) as u32 + 1;
    let day: u32 = d.parse().unwrap_or(1);
    let last = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ => {
            if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 {
                29
            } else {
                28
            }
        }
    };
    format!("{:04}-{:02}-{:02}", year, month, day.min(last))
}

pub fn add_one_year(iso: &str) -> String {
    let mut parts = iso.split('-');
    let (Some(y), Some(m), Some(d)) = (parts.next(), parts.next(), parts.next()) else {
        return iso.to_owned();
    };
    let Ok(year) = y.parse::<u32>() else {
        return iso.to_owned();
    };
    let day = if m == "02" && d == "29" { "28" } else { d };
    format!("{:04}-{m}-{day}", year + 1)
}

/// Rank each act of a (patient, kind) group inside its yearly cycle:
/// 0 = entretien initial of the cycle, 1 = premier suivi, … Cycles
/// follow the same segmentation as [`yearly_rule_next_allowed`]: a new
/// cycle starts at the first act at least 12 months after the previous
/// cycle's start. Input rows must be in ascending date order per group;
/// returns `(row_index, rank)` pairs.
fn fee_ranks(keys: &[(i64, InterviewKind, String)]) -> Vec<(usize, usize)> {
    let mut groups: std::collections::HashMap<(i64, InterviewKind), Vec<(usize, String)>> =
        std::collections::HashMap::new();
    for (i, (patient, kind, date)) in keys.iter().enumerate() {
        groups
            .entry((*patient, *kind))
            .or_default()
            .push((i, date.clone()));
    }
    let mut out = Vec::with_capacity(keys.len());
    for rows in groups.into_values() {
        let dates: Vec<String> = rows.iter().map(|(_, d)| d.clone()).collect();
        for ((i, _), rank) in rows.iter().zip(cycle_ranks(&dates)) {
            out.push((*i, rank));
        }
    }
    out
}

/// The in-cycle rank (0-based) of each date of one group, ascending.
pub fn cycle_ranks_months(dates: &[String], months: u32) -> Vec<usize> {
    let mut out = Vec::with_capacity(dates.len());
    let mut cycle_start: Option<String> = None;
    let mut rank = 0usize;
    for d in dates {
        match &cycle_start {
            Some(start) if *d < add_months(start, months) => rank += 1,
            _ => {
                cycle_start = Some(d.clone());
                rank = 0;
            }
        }
        out.push(rank);
    }
    out
}

/// The in-cycle rank of each date, using the conventional 12 months.
pub fn cycle_ranks(dates: &[String]) -> Vec<usize> {
    cycle_ranks_months(dates, 12)
}

/// Convention rule: at most `per_year` acts per "année d'accompagnement",
/// where each yearly cycle starts at its first act and the next cycle
/// cannot start before 12 months after the previous cycle's first act.
///
/// `dates` are the patient's existing act dates (ISO, ascending) for one
/// kind. Returns `None` when a new act today is allowed, or
/// `Some(next_allowed_iso)` when the quota is reached. `per_year == 0`
/// disables the rule.
pub fn yearly_rule_next_allowed(dates: &[String], today: &str, per_year: u32) -> Option<String> {
    rule_next_allowed(dates, today, per_year, 12)
}

/// As above, with the cycle length in months taken from the config.
pub fn rule_next_allowed(
    dates: &[String],
    today: &str,
    per_year: u32,
    months: u32,
) -> Option<String> {
    if per_year == 0 || dates.is_empty() {
        return None;
    }
    // Walk to the current cycle: each cycle starts at the first act at
    // least `months` months after the previous cycle's start.
    let mut cycle_start = dates[0].clone();
    loop {
        let next_cycle_from = add_months(&cycle_start, months);
        match dates.iter().find(|d| **d >= next_cycle_from) {
            Some(d) => cycle_start = d.clone(),
            None => break,
        }
    }
    let cycle_end = add_months(&cycle_start, months);
    if today >= cycle_end.as_str() {
        // A new yearly cycle may start today.
        return None;
    }
    let in_cycle = dates
        .iter()
        .filter(|d| **d >= cycle_start && **d < cycle_end)
        .count();
    if (in_cycle as u32) < per_year {
        None
    } else {
        Some(cycle_end)
    }
}

/// "2026-08" → "août 2026", for the agenda's month header.
pub fn month_name_fr(ym: &str) -> String {
    const MONTHS: [&str; 12] = [
        "janvier",
        "février",
        "mars",
        "avril",
        "mai",
        "juin",
        "juillet",
        "août",
        "septembre",
        "octobre",
        "novembre",
        "décembre",
    ];
    let mut parts = ym.split('-');
    match (parts.next(), parts.next()) {
        (Some(y), Some(m)) => match m.parse::<usize>() {
            Ok(m) if (1..=12).contains(&m) => format!("{} {}", MONTHS[m - 1], y),
            _ => ym.to_owned(),
        },
        _ => ym.to_owned(),
    }
}

/// French weekday name for an ISO date ("2026-08-24" → "lundi").
/// Sakamoto's algorithm — no calendar crate needed.
pub fn weekday_fr(iso: &str) -> Option<&'static str> {
    let mut parts = iso.split('-');
    let y: i32 = parts.next()?.parse().ok()?;
    let m: u32 = parts.next()?.parse().ok()?;
    let d: u32 = parts.next()?.parse().ok()?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    const T: [i32; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let y = if m < 3 { y - 1 } else { y };
    let w = (y + y / 4 - y / 100 + y / 400 + T[(m - 1) as usize] + d as i32).rem_euclid(7);
    Some(
        [
            "dimanche", "lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi",
        ][w as usize],
    )
}

/// Format an ISO date back to `JJ/MM/AAAA` for display.
pub fn format_french_date(iso: &str) -> String {
    let parts: Vec<&str> = iso.split('-').collect();
    if parts.len() == 3 {
        format!("{}/{}/{}", parts[2], parts[1], parts[0])
    } else {
        iso.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Shorthand for the full-form tests: the hint only matters for
    /// two-digit years.
    fn parse(input: &str) -> Result<String, String> {
        parse_french_date(input, 2026, YearHint::Past)
    }

    #[test]
    fn date_roundtrip() {
        assert_eq!(parse("03/07/1958").unwrap(), "1958-07-03");
        assert_eq!(format_french_date("1958-07-03"), "03/07/1958");
        assert!(parse("1958-07-03").is_err() || parse("31/12/1999").is_ok());
        assert!(parse("32/01/2000").is_err());
        assert!(parse("abc").is_err());
    }

    #[test]
    fn yearly_quota_rule_cycles_every_twelve_months() {
        let d = |s: &str| s.to_owned();
        // No rule / no history: always allowed.
        assert_eq!(yearly_rule_next_allowed(&[], "2026-08-24", 3), None);
        assert_eq!(
            yearly_rule_next_allowed(&[d("2026-01-01")], "2026-08-24", 0),
            None
        );
        // Two of three used this cycle: allowed.
        let two = [d("2026-01-10"), d("2026-03-01")];
        assert_eq!(yearly_rule_next_allowed(&two, "2026-08-24", 3), None);
        // Quota full: blocked until 12 months after the cycle's first act.
        let three = [d("2026-01-10"), d("2026-03-01"), d("2026-06-15")];
        assert_eq!(
            yearly_rule_next_allowed(&three, "2026-08-24", 3),
            Some(d("2027-01-10"))
        );
        // On/after that date, the "entretien nouvelle année" is allowed.
        assert_eq!(yearly_rule_next_allowed(&three, "2027-01-10", 3), None);
        // A second cycle fills up relative to ITS first act, not the
        // original one.
        let cycle2 = [
            d("2026-01-10"),
            d("2026-03-01"),
            d("2026-06-15"),
            d("2027-02-01"), // first act of cycle 2
            d("2027-04-01"),
            d("2027-05-01"),
        ];
        assert_eq!(
            yearly_rule_next_allowed(&cycle2, "2027-06-01", 3),
            Some(d("2028-02-01"))
        );
        assert_eq!(yearly_rule_next_allowed(&cycle2, "2028-02-01", 3), None);
        // Leap-day cycle start clamps sanely.
        assert_eq!(add_one_year("2028-02-29"), "2029-02-28");
        assert_eq!(add_one_year("2026-08-24"), "2027-08-24");
    }

    #[test]
    fn weekdays_are_correct() {
        assert_eq!(weekday_fr("2026-01-01"), Some("jeudi"));
        assert_eq!(weekday_fr("2026-08-24"), Some("lundi"));
        assert_eq!(weekday_fr("2000-01-01"), Some("samedi"));
        assert_eq!(weekday_fr("1958-07-03"), Some("jeudi"));
        assert_eq!(weekday_fr("pas-une-date"), None);
    }

    #[test]
    fn week_dates_run_monday_to_sunday_around_today() {
        let dir = std::env::temp_dir().join(format!("bpm-caddy-week-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("week.db");
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path, "secret").unwrap();

        let week = db.week_dates(0).unwrap();
        assert_eq!(week.len(), 7);
        assert_eq!(weekday_fr(&week[0]), Some("lundi"));
        assert_eq!(weekday_fr(&week[6]), Some("dimanche"));
        let today = db.today_iso().unwrap();
        assert!(week.contains(&today));
        // Next week starts right after this week's Sunday.
        let next = db.week_dates(1).unwrap();
        assert!(next[0] > week[6]);
        assert_eq!(weekday_fr(&next[0]), Some("lundi"));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn impossible_dates_are_rejected() {
        assert!(parse("31/02/2000").is_err());
        assert!(parse("31/04/2020").is_err());
        assert!(parse("29/02/2001").is_err());
        // 2000 is a leap year (divisible by 400), 1900 is not.
        assert!(parse("29/02/2000").is_ok());
        assert!(parse("29/02/1900").is_err());
        assert!(parse("30/04/2020").is_ok());
        assert!(parse("31/12/1999").is_ok());
    }

    #[test]
    fn compact_date_shorthand() {
        use YearHint::{Future, Past};
        // Today typed at the counter: 230826 → 23/08/2026.
        assert_eq!(
            parse_french_date("230826", 2026, Future).unwrap(),
            "2026-08-23"
        );
        assert_eq!(
            parse_french_date("1908", 2026, Future).unwrap(),
            "2026-08-19"
        );
        assert_eq!(
            parse_french_date("19082026", 2026, Future).unwrap(),
            "2026-08-19"
        );
        // Birth dates never land in the future ("49" → 1949, newborn "26" → 2026)…
        assert_eq!(
            parse_french_date("110249", 2026, Past).unwrap(),
            "1949-02-11"
        );
        assert_eq!(
            parse_french_date("030726", 2026, Past).unwrap(),
            "2026-07-03"
        );
        // …while appointments always expand to 20xx.
        assert_eq!(
            parse_french_date("110249", 2026, Future).unwrap(),
            "2049-02-11"
        );
        // Yearless forms are for RDVs only: a birth date without a year
        // is more likely truncated than a newborn — rejected.
        assert!(parse_french_date("1908", 2026, Past).is_err());
        assert!(parse_french_date("19/08", 2026, Past).is_err());
        // Separator forms take the same two-digit and day/month shorthand.
        assert_eq!(
            parse_french_date("3/7/58", 2026, Past).unwrap(),
            "1958-07-03"
        );
        assert_eq!(
            parse_french_date("19/08", 2026, Future).unwrap(),
            "2026-08-19"
        );
        // Shorthand is still validated.
        assert!(parse_french_date("300226", 2026, Future).is_err()); // 30/02
        assert!(parse_french_date("12345", 2026, Future).is_err()); // wrong length
        assert!(parse_french_date("1913", 2026, Future).is_err()); // month 13
    }

    #[test]
    fn interview_pipeline_is_strictly_ordered() {
        let mut state = InterviewState::Identified;
        let mut seen = vec![state];
        while let Some(next) = state.next() {
            state = next;
            seen.push(state);
        }
        assert_eq!(seen, InterviewState::ALL);
        assert_eq!(state, InterviewState::Billed);
        assert_eq!(state.next(), None);
        // Round-trip through the storage representation.
        for s in InterviewState::ALL {
            assert_eq!(InterviewState::parse(s.as_str()), Some(s));
        }
        for k in InterviewKind::ALL {
            assert_eq!(InterviewKind::parse(k.as_str()), Some(k));
        }
    }

    #[test]
    fn interviews_advance_and_persist() {
        let dir = std::env::temp_dir().join(format!("bpm-caddy-itest-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("interviews.db");
        let _ = std::fs::remove_file(&path);

        let db = Db::open(&path, "secret").unwrap();
        let pid = db.add_patient("Martin", "Claire", "1949-02-11").unwrap();
        let iid = db.add_interview(pid, InterviewKind::Bpm).unwrap();

        let loaded = db.interviews_for(pid).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].state, InterviewState::Identified);
        assert_eq!(
            db.interview_dates_for(pid, InterviewKind::Bpm)
                .unwrap()
                .len(),
            1
        );
        assert!(db
            .interview_dates_for(pid, InterviewKind::Aod)
            .unwrap()
            .is_empty());

        db.advance_interview(iid, InterviewState::Identified)
            .unwrap();
        let loaded = db.interviews_for(pid).unwrap();
        assert_eq!(loaded[0].state, InterviewState::Scheduled);

        // A misclicked advance can be stepped back.
        db.regress_interview(iid, InterviewState::Scheduled)
            .unwrap();
        let loaded = db.interviews_for(pid).unwrap();
        assert_eq!(loaded[0].state, InterviewState::Identified);
        // Regressing the first state is a harmless no-op.
        db.regress_interview(iid, InterviewState::Identified)
            .unwrap();
        assert_eq!(
            db.interviews_for(pid).unwrap()[0].state,
            InterviewState::Identified
        );

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn cycles_follow_the_configured_length() {
        // Six-month cycles: the third act opens a new cycle.
        let dates: Vec<String> = ["2026-01-10", "2026-03-10", "2026-08-10", "2026-10-01"]
            .iter()
            .map(|s| (*s).to_owned())
            .collect();
        assert_eq!(cycle_ranks_months(&dates, 6), vec![0, 1, 0, 1]);
        // The same dates over twelve months are one cycle then another.
        assert_eq!(cycle_ranks_months(&dates, 12), vec![0, 1, 2, 3]);
        // The quota rule uses the same segmentation.
        assert_eq!(
            rule_next_allowed(&dates[..2], "2026-04-01", 2, 6),
            Some("2026-07-10".to_owned())
        );
        assert_eq!(rule_next_allowed(&dates[..2], "2026-08-01", 2, 6), None);
        // add_months clamps the short months and February.
        assert_eq!(add_months("2026-01-31", 1), "2026-02-28");
        assert_eq!(add_months("2024-01-31", 1), "2024-02-29");
        assert_eq!(add_months("2026-08-25", 6), "2027-02-25");
        assert_eq!(add_months("2026-08-25", 12), add_one_year("2026-08-25"));
    }

    #[test]
    fn cycle_ranks_number_acts_within_each_yearly_cycle() {
        // Three acts in one année d'accompagnement, then a new cycle
        // (≥ 12 months after the first act of the previous one).
        let dates: Vec<String> = [
            "2026-01-10",
            "2026-04-10",
            "2026-09-10",
            "2027-02-01",
            "2027-06-01",
        ]
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
        assert_eq!(cycle_ranks(&dates), vec![0, 1, 2, 0, 1]);
        // The very first act of a base is always the entretien initial.
        assert_eq!(cycle_ranks(&["2026-08-24".to_owned()]), vec![0]);
        assert!(cycle_ranks(&[]).is_empty());
    }

    #[test]
    fn themes_are_compare_and_set_and_reach_the_export() {
        let dir = std::env::temp_dir().join(format!("bpm-caddy-theme-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("theme.db");
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path, "secret").unwrap();
        let pid = db.add_patient("Dupont", "Jean", "1958-07-03").unwrap();
        let id = db
            .add_interview_themed(pid, InterviewKind::Bpm, "Observance")
            .unwrap();
        assert_eq!(db.interviews_for(pid).unwrap()[0].theme, "Observance");
        // A write from a stale view is rejected, not applied.
        assert!(!db.set_theme(id, "Interactions", "Biologie / INR").unwrap());
        assert!(db.set_theme(id, "Interactions", "Observance").unwrap());
        let rows = db.export_rows().unwrap();
        assert_eq!(rows[0].theme, "Interactions");
        assert_eq!(rows[0].fee_rank, 0);
        // A second act of the same cycle is a suivi, and the summaries
        // agree with the export.
        db.add_interview(pid, InterviewKind::Bpm).unwrap();
        let mut ranks: Vec<usize> = db
            .export_rows()
            .unwrap()
            .iter()
            .map(|r| r.fee_rank)
            .collect();
        ranks.sort_unstable();
        assert_eq!(ranks, vec![0, 1]);
        let mut ranks: Vec<usize> = db
            .interview_summaries()
            .unwrap()
            .iter()
            .map(|s| s.fee_rank)
            .collect();
        ranks.sort_unstable();
        assert_eq!(ranks, vec![0, 1]);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn starter_details_fill_only_empty_fields() {
        let dir = std::env::temp_dir().join(format!("bpm-caddy-detail-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("detail.db");
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path, "secret").unwrap();
        db.seed_drugs_if_empty().unwrap();

        // Every detailed card names a drug of the starter list, and the
        // seed applied its clinical fields.
        let drugs = db.drugs().unwrap();
        for d in STARTER_DETAILS {
            let card = drugs
                .iter()
                .find(|x| x.name == d.name)
                .unwrap_or_else(|| panic!("fiche « {} » absente de la base", d.name));
            assert_eq!(card.dosage, d.dosage);
            assert_eq!(card.iup, d.iup);
        }

        // The team's own text is never overwritten by a later top-up,
        // and a field they cleared stays theirs to refill.
        let base = drugs.iter().find(|d| d.name == "Eliquis").unwrap().clone();
        let mut edited = base.clone();
        edited.dosage = "Protocole interne : 5 mg x2/j, revoir à 3 mois".to_owned();
        edited.ddi = String::new();
        assert!(db.update_drug(&edited, &base).unwrap());
        db.fill_starter_details().unwrap();
        let after = db
            .drugs()
            .unwrap()
            .into_iter()
            .find(|d| d.name == "Eliquis")
            .unwrap();
        assert_eq!(
            after.dosage,
            "Protocole interne : 5 mg x2/j, revoir à 3 mois"
        );
        // The emptied field is refilled from the reference data.
        assert!(after.ddi.contains("CYP3A4"));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn reset_clears_everything_and_reseeds_the_drugs() {
        let dir = std::env::temp_dir().join(format!("bpm-caddy-reset-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("reset.db");
        let _ = std::fs::remove_file(&path);
        let db = Db::open(&path, "secret").unwrap();
        let pid = db.add_patient("Dupont", "Jean", "1958-07-03").unwrap();
        db.add_interview(pid, InterviewKind::Bpm).unwrap();
        db.add_note(NoteSubject::Patient, pid, "YS", "note")
            .unwrap();
        db.seed_drugs_if_empty().unwrap();
        // A base with only a couple of drugs left is completed in place.
        db.delete_drug(
            db.drugs().unwrap()[0].id,
            &db.drugs().unwrap()[0].name.clone(),
        )
        .unwrap();
        assert_eq!(db.seed_missing_drugs().unwrap(), 1);

        db.reset_all_data().unwrap();
        assert!(db.patients().unwrap().is_empty());
        assert!(db.interviews_for(pid).unwrap().is_empty());
        assert!(db.notes_for(NoteSubject::Patient, pid).unwrap().is_empty());
        assert_eq!(db.drugs().unwrap().len(), STARTER_DRUG_COUNT);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn starter_drugs_seed_once_and_never_resurrect() {
        let dir = std::env::temp_dir().join(format!("bpm-caddy-sdrug-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("starter.db");
        let _ = std::fs::remove_file(&path);

        let db = Db::open(&path, "secret").unwrap();
        // No duplicate brand names in the starter list (the per-name
        // NOT EXISTS guard would silently drop them).
        let mut names: Vec<&str> = STARTER_DRUGS.iter().map(|(n, _, _, _)| *n).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), STARTER_DRUGS.len());

        let n = db.seed_drugs_if_empty().unwrap();
        assert_eq!(n, STARTER_DRUGS.len());
        let eliquis = db
            .drugs()
            .unwrap()
            .into_iter()
            .find(|d| d.name == "Eliquis")
            .unwrap();
        assert_eq!(eliquis.dci, "apixaban");
        assert_eq!(eliquis.class, "AOD");
        assert_eq!(eliquis.antidote, "Andexanet alfa");
        // Eliquis is one of the detailed cards; a drug outside that
        // list ships identity only, its clinical fields left to the team.
        assert!(!eliquis.dosage.is_empty());
        let plain = db
            .drugs()
            .unwrap()
            .into_iter()
            .find(|d| d.name == "Doliprane")
            .unwrap();
        assert!(plain.dosage.is_empty() && plain.ddi.is_empty() && plain.iup.is_empty());

        // Second run: no-op. After a deliberate deletion: still no-op.
        assert_eq!(db.seed_drugs_if_empty().unwrap(), 0);
        assert!(db.delete_drug(eliquis.id, "Eliquis").unwrap());
        assert_eq!(db.seed_drugs_if_empty().unwrap(), 0);
        assert!(db.drugs().unwrap().iter().all(|d| d.name != "Eliquis"));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn transmission_logbook_groups_by_day() {
        let dir = std::env::temp_dir().join(format!("bpm-caddy-trans-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("trans.db");
        let _ = std::fs::remove_file(&path);

        let db = Db::open(&path, "secret").unwrap();
        let today = db.today_iso().unwrap();
        db.add_note(NoteSubject::Transmission, 0, "CL", "Commande X en retard.")
            .unwrap();
        db.add_note(NoteSubject::Transmission, 0, "YS", "Voir Mme M demain.")
            .unwrap();

        let entries = db.transmissions_for_day(&today).unwrap();
        assert_eq!(entries.len(), 2);
        // Chronological within the day.
        assert_eq!(entries[0].operator, "CL");
        assert_eq!(entries[1].operator, "YS");
        assert!(db
            .transmissions_for_day(&db.date_offset(&today, -1).unwrap())
            .unwrap()
            .is_empty());
        assert_eq!(db.transmission_days().unwrap(), vec![today.clone()]);
        assert_eq!(
            db.date_offset("2026-03-01", -1).unwrap(),
            "2026-02-28".to_owned()
        );
        assert_eq!(
            db.date_offset("2026-08-24", 1).unwrap(),
            "2026-08-25".to_owned()
        );

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn standalone_notes_journal() {
        let dir = std::env::temp_dir().join(format!("bpm-caddy-note-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("notes.db");
        let _ = std::fs::remove_file(&path);

        let db = Db::open(&path, "secret").unwrap();
        let pid = db.add_patient("Dupont", "Jean", "1958-07-03").unwrap();
        let did = db.add_drug("Eliquis").unwrap();

        db.add_note(NoteSubject::Patient, pid, "CL", "Préfère le matin.")
            .unwrap();
        let n2 = db
            .add_note(NoteSubject::Patient, pid, "YS", "Allergie pénicilline ?")
            .unwrap();
        db.add_note(NoteSubject::Drug, did, "CL", "Rupture fournisseur.")
            .unwrap();
        db.add_note(NoteSubject::Operator, 0, "CL", "Rappeler le grossiste.")
            .unwrap();

        let pnotes = db.notes_for(NoteSubject::Patient, pid).unwrap();
        assert_eq!(pnotes.len(), 2);
        // Newest first, author kept.
        assert_eq!(pnotes[0].id, n2);
        assert_eq!(pnotes[0].operator, "YS");
        assert!(!pnotes[0].stamp().is_empty());
        assert_eq!(db.notes_for(NoteSubject::Drug, did).unwrap().len(), 1);
        assert_eq!(db.notes_for_operator("CL").unwrap().len(), 1);
        assert!(db.notes_for_operator("YS").unwrap().is_empty());

        db.delete_note(n2).unwrap();
        assert_eq!(db.notes_for(NoteSubject::Patient, pid).unwrap().len(), 1);

        // Deleting the patient / drug removes their journals.
        db.delete_patient(pid).unwrap();
        assert!(db.notes_for(NoteSubject::Patient, pid).unwrap().is_empty());
        assert!(db.delete_drug(did, "Eliquis").unwrap());
        assert!(db.notes_for(NoteSubject::Drug, did).unwrap().is_empty());
        // Operator notes are personal and survive.
        assert_eq!(db.notes_for_operator("CL").unwrap().len(), 1);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn drug_base_crud_is_cas() {
        let dir = std::env::temp_dir().join(format!("bpm-caddy-drug-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("drugs.db");
        let _ = std::fs::remove_file(&path);

        let db = Db::open(&path, "secret").unwrap();
        let id = db.add_drug("Apixaban").unwrap();
        let base = db.drugs().unwrap()[0].clone();
        assert_eq!(base.name, "Apixaban");

        let mut card = base.clone();
        card.dosage = "5 mg x2/j".to_owned();
        card.antidote = "Andexanet alfa".to_owned();
        card.half_life = "≈ 12 h".to_owned();
        card.renal = "2,5 mg x2/j si DFG 15-29".to_owned();
        assert!(db.update_drug(&card, &base).unwrap());
        // A stale edit (based on the pre-update card) is refused.
        let mut stale = base.clone();
        stale.dosage = "2,5 mg x2/j".to_owned();
        assert!(!db.update_drug(&stale, &base).unwrap());
        let fresh = db.drugs().unwrap()[0].clone();
        assert_eq!(fresh.dosage, "5 mg x2/j");
        assert_eq!(fresh.antidote, "Andexanet alfa");
        assert_eq!(fresh.half_life, "≈ 12 h");
        assert_eq!(fresh.renal, "2,5 mg x2/j si DFG 15-29");

        // Alphabetical, case-insensitive.
        db.add_drug("amoxicilline").unwrap();
        let names: Vec<String> = db.drugs().unwrap().into_iter().map(|d| d.name).collect();
        assert_eq!(names, vec!["amoxicilline", "Apixaban"]);

        // Deleting a renamed card is refused; with the right name it works.
        assert!(!db.delete_drug(id, "Apixabon").unwrap());
        assert!(db.delete_drug(id, "Apixaban").unwrap());
        assert_eq!(db.drugs().unwrap().len(), 1);

        assert!(!db.now_stamp().unwrap().is_empty());

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn rekey_changes_the_master_password() {
        let dir = std::env::temp_dir().join(format!("bpm-caddy-rekey-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("rekey.db");
        let _ = std::fs::remove_file(&path);

        {
            let db = Db::open(&path, "ancien").unwrap();
            db.add_patient("Dupont", "Jean", "1958-07-03").unwrap();
            db.change_password("nouveau").unwrap();
            assert!(db.change_password("").is_err());
        }
        assert!(Db::open(&path, "ancien").is_err());
        let db = Db::open(&path, "nouveau").unwrap();
        assert_eq!(db.patients().unwrap()[0].full_name(), "Jean Dupont");

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn backup_snapshot_opens_with_the_same_password() {
        let dir = std::env::temp_dir().join(format!("bpm-caddy-bak-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("live.db");
        let bak = dir.join("bak.db");
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(&bak);

        let db = Db::open(&path, "secret").unwrap();
        db.add_patient("Dupont", "Jean", "1958-07-03").unwrap();
        db.backup_to(&bak).unwrap();

        // The snapshot is a full encrypted database with the same key.
        assert!(Db::open(&bak, "mauvais").is_err());
        let restored = Db::open(&bak, "secret").unwrap();
        assert_eq!(restored.patients().unwrap()[0].full_name(), "Jean Dupont");

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(&bak);
    }

    #[test]
    fn patients_can_be_corrected_and_deleted() {
        let dir = std::env::temp_dir().join(format!("bpm-caddy-edit-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("edit.db");
        let _ = std::fs::remove_file(&path);

        let db = Db::open(&path, "secret").unwrap();
        let pid = db.add_patient("Dupond", "Jaen", "1958-07-04").unwrap();
        let seen = db.patients().unwrap()[0].clone();
        let corrected = Patient {
            id: pid,
            last_name: "Dupont".to_owned(),
            first_name: "Jean".to_owned(),
            birth_date: "1958-07-03".to_owned(),
            phone: "06 12 34 56 78".to_owned(),
            notes: "préfère le matin".to_owned(),
            physician: "Dr Morel".to_owned(),
            email: "jean.dupont@example.org".to_owned(),
            address: "12 rue des Lilas".to_owned(),
        };
        assert!(db.update_patient(&corrected, &seen).unwrap());
        let p = db.patients().unwrap();
        assert_eq!(p[0].full_name(), "Jean Dupont");
        assert_eq!(p[0].birth_date, "1958-07-03");
        assert_eq!(p[0].phone, "06 12 34 56 78");
        assert_eq!(p[0].notes, "préfère le matin");
        assert_eq!(p[0].physician, "Dr Morel");
        assert_eq!(p[0].email, "jean.dupont@example.org");
        assert_eq!(p[0].address, "12 rue des Lilas");

        // An edit based on the pre-correction snapshot is rejected
        // instead of silently overwriting the newer values.
        let stale = Patient {
            last_name: "X".to_owned(),
            ..corrected.clone()
        };
        assert!(!db.update_patient(&stale, &seen).unwrap());
        assert_eq!(db.patients().unwrap()[0].full_name(), "Jean Dupont");

        // Treatments: link drugs to the patient, idempotently.
        let did = db.add_drug("Eliquis").unwrap();
        db.add_patient_drug(pid, did).unwrap();
        db.add_patient_drug(pid, did).unwrap();
        let treats = db.drugs_for_patient(pid).unwrap();
        assert_eq!(treats.len(), 1);
        assert_eq!(treats[0].name, "Eliquis");
        // …and the reverse lookup finds the patient from the drug.
        let on_drug = db.patients_for_drug(did).unwrap();
        assert_eq!(on_drug.len(), 1);
        assert_eq!(on_drug[0].full_name(), "Jean Dupont");
        db.remove_patient_drug(pid, did).unwrap();
        assert!(db.drugs_for_patient(pid).unwrap().is_empty());
        db.add_patient_drug(pid, did).unwrap();

        // Interview deletion is CAS on the state this PC saw.
        let iid = db.add_interview(pid, InterviewKind::Bpm).unwrap();
        db.advance_interview(iid, InterviewState::Identified)
            .unwrap();
        assert!(!db
            .delete_interview(iid, InterviewState::Identified)
            .unwrap());
        assert_eq!(db.interviews_for(pid).unwrap().len(), 1);
        assert!(db.delete_interview(iid, InterviewState::Scheduled).unwrap());
        assert!(db.interviews_for(pid).unwrap().is_empty());

        // Deletion removes the patient, their interviews and their
        // treatment links atomically (the drug card itself stays).
        db.add_interview(pid, InterviewKind::Bpm).unwrap();
        db.delete_patient(pid).unwrap();
        assert!(db.patients().unwrap().is_empty());
        assert!(db.interviews_for(pid).unwrap().is_empty());
        assert!(db.drugs_for_patient(pid).unwrap().is_empty());
        assert_eq!(db.drugs().unwrap().len(), 1);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn stale_state_changes_are_rejected() {
        let dir = std::env::temp_dir().join(format!("bpm-caddy-cas-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("cas.db");
        let _ = std::fs::remove_file(&path);

        let db = Db::open(&path, "secret").unwrap();
        let pid = db.add_patient("Dupont", "Jean", "1958-07-03").unwrap();
        let iid = db.add_interview(pid, InterviewKind::Bpm).unwrap();

        // Another PC advances the interview twice…
        assert!(db
            .advance_interview(iid, InterviewState::Identified)
            .unwrap());
        assert!(db
            .advance_interview(iid, InterviewState::Scheduled)
            .unwrap());
        // …so this PC's click, based on a stale "Identifié", must neither
        // apply nor regress the row.
        assert!(!db
            .advance_interview(iid, InterviewState::Identified)
            .unwrap());
        assert!(!db
            .regress_interview(iid, InterviewState::Scheduled)
            .unwrap());
        assert_eq!(
            db.interviews_for(pid).unwrap()[0].state,
            InterviewState::Performed
        );

        // Durations are CAS too: a stale 0 must not erase 45.
        assert!(db.set_duration(iid, 45, 0).unwrap());
        assert!(!db.set_duration(iid, 10, 0).unwrap());
        assert_eq!(db.interviews_for(pid).unwrap()[0].duration_minutes, 45);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn upcoming_appointments_lists_planned_interviews() {
        let dir = std::env::temp_dir().join(format!("bpm-caddy-rdv-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("rdv.db");
        let _ = std::fs::remove_file(&path);

        let db = Db::open(&path, "secret").unwrap();
        let pid = db.add_patient("Dupont", "Jean", "1958-07-03").unwrap();
        let planned = db.add_interview(pid, InterviewKind::Bpm).unwrap();
        assert!(db
            .set_scheduled_date(planned, Some("2030-05-12"), None)
            .unwrap());
        // A stale expectation must not overwrite the newer date.
        assert!(!db
            .set_scheduled_date(planned, Some("2030-06-01"), Some("2030-04-01"))
            .unwrap());
        let earlier = db.add_interview(pid, InterviewKind::Aod).unwrap();
        db.set_scheduled_date(earlier, Some("2030-01-02"), None)
            .unwrap();
        // No date, or already performed: not an upcoming appointment.
        db.add_interview(pid, InterviewKind::Asthme).unwrap();
        let done = db.add_interview(pid, InterviewKind::Bpm).unwrap();
        db.set_scheduled_date(done, Some("2030-03-03"), None)
            .unwrap();
        db.advance_interview(done, InterviewState::Identified)
            .unwrap();
        db.advance_interview(done, InterviewState::Scheduled)
            .unwrap();

        let rdv = db.upcoming_appointments().unwrap();
        assert_eq!(rdv.len(), 2);
        // Soonest first.
        assert_eq!(rdv[0].date, "2030-01-02");
        assert_eq!(rdv[0].kind, InterviewKind::Aod);
        assert_eq!(rdv[1].date, "2030-05-12");
        assert_eq!(rdv[0].patient_name, "Jean Dupont");
        assert_eq!(rdv[0].patient_id, pid);

        assert_eq!(db.today_iso().unwrap().len(), 10);

        // The CSV export sees every interview, oldest first, with the
        // patient joined in.
        let export = db.export_rows().unwrap();
        assert_eq!(export.len(), 4);
        assert_eq!(export[0].patient_name, "Jean Dupont");
        assert_eq!(export[0].kind, InterviewKind::Bpm);
        assert_eq!(export[0].scheduled_date.as_deref(), Some("2030-05-12"));

        let _ = std::fs::remove_file(&path);
    }

    /// Not a test of behavior: seeds a demo database when the env asks for
    /// one (screenshots, manual demos). `cargo test` without the env is a
    /// no-op.
    #[test]
    fn seed_demo_db_if_requested() {
        let Some(path) = std::env::var_os("BPM_CADDY_SEED_DB") else {
            return;
        };
        let pw = std::env::var("BPM_CADDY_SEED_PW").unwrap_or_else(|_| "demo".to_owned());
        let db = Db::open(Path::new(&path), &pw).unwrap();
        let seed = [
            (
                "Dupont",
                "Jean",
                "1958-07-03",
                InterviewKind::Bpm,
                4,
                45,
                None,
                "Initiation / bon usage",
            ),
            (
                "Martin",
                "Claire",
                "1949-02-11",
                InterviewKind::Bpm,
                2,
                50,
                None,
                "Observance",
            ),
            (
                "Lefèvre",
                "Hélène",
                "1952-09-27",
                InterviewKind::Aod,
                1,
                30,
                Some(("+2 days", "06 12 34 56 78")),
                "Biologie / INR",
            ),
            (
                "Bernard",
                "Paul",
                "1946-12-05",
                InterviewKind::Asthme,
                0,
                0,
                Some(("-3 days", "07 98 76 54 32")),
                "Technique d'inhalation",
            ),
            (
                "Moreau",
                "Lucie",
                "1961-03-18",
                InterviewKind::Aod,
                3,
                35,
                None,
                "Effets indésirables",
            ),
        ];
        // The starter base already carries the detailed reference cards
        // (Eliquis among them), so the demo needs no extra drug text.
        db.seed_drugs_if_empty().unwrap();

        for (last, first, dob, kind, advances, minutes, rdv, theme) in seed {
            let pid = db.add_patient(last, first, dob).unwrap();
            let iid = db.add_interview_themed(pid, kind, theme).unwrap();
            let mut state = InterviewState::Identified;
            for _ in 0..advances {
                db.advance_interview(iid, state).unwrap();
                state = state.next().unwrap();
            }
            if minutes > 0 {
                db.set_duration(iid, minutes, 0).unwrap();
            }
            // Full record and current treatments for the demo's first
            // patient, so the patient view shows everything.
            if last == "Dupont" {
                let seen = Patient {
                    id: pid,
                    last_name: last.to_owned(),
                    first_name: first.to_owned(),
                    birth_date: dob.to_owned(),
                    ..Default::default()
                };
                let full = Patient {
                    phone: "06 01 02 03 04".to_owned(),
                    physician: "Dr Morel".to_owned(),
                    email: "jean.dupont@exemple.fr".to_owned(),
                    address: "12 rue des Lilas, 34000 Montpellier".to_owned(),
                    ..seen.clone()
                };
                db.update_patient(&full, &seen).unwrap();
                for name in ["Eliquis", "Tahor"] {
                    if let Some(d) = db.drugs().unwrap().into_iter().find(|d| d.name == name) {
                        db.add_patient_drug(pid, d.id).unwrap();
                        if name == "Eliquis" {
                            db.add_note(
                                NoteSubject::Drug,
                                d.id,
                                "CL",
                                "Rupture fournisseur — retour annoncé sous 10 j.",
                            )
                            .unwrap();
                        }
                    }
                }
                // A second BPM in the same année d'accompagnement, so the
                // demo shows a suivi act (and its lower fee).
                let suivi = db
                    .add_interview_themed(pid, InterviewKind::Bpm, "Observance")
                    .unwrap();
                db.advance_interview(suivi, InterviewState::Identified)
                    .unwrap();
                db.set_duration(suivi, 20, 0).unwrap();
                db.add_note(
                    NoteSubject::Patient,
                    pid,
                    "CL",
                    "Préfère les RDV le matin ; fille à prévenir (06 …).",
                )
                .unwrap();
                db.add_note(
                    NoteSubject::Patient,
                    pid,
                    "YS",
                    "Confusion doses AOD à revoir.",
                )
                .unwrap();
                db.add_note(
                    NoteSubject::Operator,
                    0,
                    "CL",
                    "Rappeler le grossiste lundi.",
                )
                .unwrap();
                // Agenda entries that are not acts, for the demo.
                let today = db.today_iso().unwrap();
                db.add_event(&today, "Formation AOD — 14 h", EventCategory::Formation)
                    .unwrap();
                let plus2 = db.date_offset(&today, 2).unwrap();
                db.add_event(&plus2, "Livraison grossiste", EventCategory::Livraison)
                    .unwrap();
                db.add_note(
                    NoteSubject::Day,
                    day_subject_id(&today),
                    "CL",
                    "Vérifier le stock d'Eliquis avant la formation.",
                )
                .unwrap();

                // Transmissions: one entry yesterday, two today.
                let t1 = db
                    .add_note(
                        NoteSubject::Transmission,
                        0,
                        "CL",
                        "Rupture Eliquis 5 mg — dépannage possible pharmacie Centrale.",
                    )
                    .unwrap();
                db.conn
                    .execute(
                        "UPDATE notes SET created_at = datetime('now', 'localtime', '-1 day')
                         WHERE id = ?1",
                        [t1],
                    )
                    .unwrap();
                db.add_note(
                    NoteSubject::Transmission,
                    0,
                    "CL",
                    "M. Dupont passe demain matin récupérer son courrier CR.",
                )
                .unwrap();
                db.add_note(
                    NoteSubject::Transmission,
                    0,
                    "YS",
                    "Penser à facturer les 2 entretiens « Réalisés » de la semaine.",
                )
                .unwrap();
            }
            // Extra acts with RDVs so the agenda's week view shows
            // several colors.
            if last == "Martin" {
                let extra = db.add_interview(pid, InterviewKind::Prevention).unwrap();
                let d: String = db
                    .conn
                    .query_row("SELECT date('now','localtime','+1 day')", [], |r| r.get(0))
                    .unwrap();
                db.set_scheduled_date(extra, Some(&d), None).unwrap();
            }
            if last == "Moreau" {
                let extra = db.add_interview(pid, InterviewKind::TrodAngine).unwrap();
                let d: String = db
                    .conn
                    .query_row("SELECT date('now','localtime','+3 day')", [], |r| r.get(0))
                    .unwrap();
                db.set_scheduled_date(extra, Some(&d), None).unwrap();
            }
            // Planned dates relative to today, so the demo dashboard shows
            // both an upcoming and an overdue appointment (with a phone
            // to call for the reminder).
            if let Some((offset, phone)) = rdv {
                let seen = Patient {
                    id: pid,
                    last_name: last.to_owned(),
                    first_name: first.to_owned(),
                    birth_date: dob.to_owned(),
                    ..Default::default()
                };
                let with_phone = Patient {
                    phone: phone.to_owned(),
                    physician: "Dr Morel".to_owned(),
                    ..seen.clone()
                };
                db.update_patient(&with_phone, &seen).unwrap();
                let date: String = db
                    .conn
                    .query_row("SELECT date('now', 'localtime', ?1)", [offset], |r| {
                        r.get(0)
                    })
                    .unwrap();
                db.set_scheduled_date(iid, Some(&date), None).unwrap();
            }
        }
    }

    #[test]
    fn encrypted_db_rejects_wrong_password() {
        let dir = std::env::temp_dir().join(format!("bpm-caddy-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.db");
        let _ = std::fs::remove_file(&path);

        {
            let db = Db::open(&path, "bon-mot-de-passe").unwrap();
            db.add_patient("Dupont", "Jean", "1958-07-03").unwrap();
            assert_eq!(db.patients().unwrap().len(), 1);
        }
        assert!(Db::open(&path, "mauvais").is_err());
        let db = Db::open(&path, "bon-mot-de-passe").unwrap();
        assert_eq!(db.patients().unwrap()[0].full_name(), "Jean Dupont");

        let _ = std::fs::remove_file(&path);
    }
}
