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
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE IF NOT EXISTS interviews (
    id          INTEGER PRIMARY KEY,
    patient_id  INTEGER NOT NULL REFERENCES patients(id),
    kind        TEXT NOT NULL,
    state       TEXT NOT NULL DEFAULT 'IDENTIFIED',
    duration_minutes INTEGER NOT NULL DEFAULT 0,
    scheduled_date TEXT,
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE IF NOT EXISTS drugs (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL,
    dci         TEXT NOT NULL DEFAULT '',
    dosage      TEXT NOT NULL DEFAULT '',
    ddi         TEXT NOT NULL DEFAULT '',
    iup         TEXT NOT NULL DEFAULT '',
    antidote    TEXT NOT NULL DEFAULT '',
    notes       TEXT NOT NULL DEFAULT ''
);
";

/// Idempotent migrations for databases created by older versions.
const MIGRATIONS: &[&str] = &[
    "ALTER TABLE interviews ADD COLUMN duration_minutes INTEGER NOT NULL DEFAULT 0",
    "ALTER TABLE interviews ADD COLUMN scheduled_date TEXT",
    "ALTER TABLE patients ADD COLUMN phone TEXT NOT NULL DEFAULT ''",
    "ALTER TABLE patients ADD COLUMN notes TEXT NOT NULL DEFAULT ''",
    "ALTER TABLE drugs ADD COLUMN dci TEXT NOT NULL DEFAULT ''",
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
}

impl InterviewKind {
    pub const ALL: [InterviewKind; 6] = [
        Self::Bpm,
        Self::Aod,
        Self::Asthme,
        Self::TrodAngine,
        Self::TrodCystite,
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
        }
    }
}

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
    pub created_at: String,
}

/// One entry of the team's drug reference base (shared, encrypted with
/// the patient data): the facts wanted at the counter in one glance.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Drug {
    pub id: i64,
    pub name: String,
    /// Dénomination commune internationale (INN).
    pub dci: String,
    /// Usual dosage / posology.
    pub dosage: String,
    /// Drug-drug interactions to watch for.
    pub ddi: String,
    /// IUP.
    pub iup: String,
    pub antidote: String,
    /// The team's own notes.
    pub notes: String,
}

#[derive(Clone, Debug, PartialEq)]
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
}

impl Patient {
    pub fn full_name(&self) -> String {
        format!("{} {}", self.first_name, self.last_name)
    }
}

/// (brand name, DCI, textbook antidote or ""). See
/// [`Db::seed_drugs_if_empty`].
const STARTER_DRUGS: &[(&str, &str, &str)] = &[
    // Anticoagulants / antiagrégants
    ("Eliquis", "apixaban", "Andexanet alfa"),
    ("Xarelto", "rivaroxaban", "Andexanet alfa"),
    ("Pradaxa", "dabigatran", "Idarucizumab"),
    ("Lixiana", "edoxaban", ""),
    ("Coumadine", "warfarine", "Vitamine K"),
    ("Previscan", "fluindione", "Vitamine K"),
    ("Sintrom", "acénocoumarol", "Vitamine K"),
    ("Héparine", "héparine sodique", "Protamine"),
    ("Lovenox", "énoxaparine", ""),
    ("Kardégic", "acide acétylsalicylique", ""),
    ("Plavix", "clopidogrel", ""),
    ("Brilique", "ticagrélor", ""),
    // Douleur
    ("Doliprane", "paracétamol", "N-acétylcystéine"),
    ("Dafalgan", "paracétamol", "N-acétylcystéine"),
    ("Tramadol", "tramadol", "Naloxone"),
    ("Skenan", "morphine", "Naloxone"),
    ("Oxycontin", "oxycodone", "Naloxone"),
    ("Durogesic", "fentanyl", "Naloxone"),
    // Benzodiazépines / hypnotiques
    ("Xanax", "alprazolam", "Flumazénil"),
    ("Lexomil", "bromazépam", "Flumazénil"),
    ("Temesta", "lorazépam", "Flumazénil"),
    ("Valium", "diazépam", "Flumazénil"),
    ("Séresta", "oxazépam", "Flumazénil"),
    ("Stilnox", "zolpidem", "Flumazénil"),
    ("Imovane", "zopiclone", "Flumazénil"),
    // Cardiologie
    ("Tahor", "atorvastatine", ""),
    ("Crestor", "rosuvastatine", ""),
    ("Coversyl", "périndopril", ""),
    ("Triatec", "ramipril", ""),
    ("Cozaar", "losartan", ""),
    ("Aprovel", "irbésartan", ""),
    ("Amlor", "amlodipine", ""),
    ("Isoptine", "vérapamil", ""),
    ("Cordarone", "amiodarone", ""),
    ("Cardensiel", "bisoprolol", ""),
    ("Ténormine", "aténolol", ""),
    ("Lasilix", "furosémide", ""),
    ("Aldactone", "spironolactone", ""),
    ("Digoxine", "digoxine", "Fab antidigoxine"),
    // Diabète
    ("Glucophage", "metformine", ""),
    ("Diamicron", "gliclazide", ""),
    ("Ozempic", "sémaglutide", ""),
    ("Lantus", "insuline glargine", ""),
    // Respiratoire
    ("Ventoline", "salbutamol", ""),
    ("Symbicort", "budésonide + formotérol", ""),
    ("Seretide", "fluticasone + salmétérol", ""),
    ("Spiriva", "tiotropium", ""),
    ("Singulair", "montélukast", ""),
    // Divers courants
    ("Levothyrox", "lévothyroxine", ""),
    ("Inexium", "ésoméprazole", ""),
    ("Inipomp", "pantoprazole", ""),
    ("Mopral", "oméprazole", ""),
    ("Amoxicilline", "amoxicilline", ""),
    ("Augmentin", "amoxicilline + acide clavulanique", ""),
    ("Pyostacine", "pristinamycine", ""),
    ("Cortancyl", "prednisone", ""),
    ("Solupred", "prednisolone", ""),
    ("Méthotrexate", "méthotrexate", "Acide folinique"),
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
            .prepare("SELECT id, last_name, first_name, birth_date, phone, notes FROM patients")
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
    #[allow(clippy::too_many_arguments)]
    pub fn update_patient(
        &self,
        id: i64,
        last_name: &str,
        first_name: &str,
        birth_date: &str,
        phone: &str,
        notes: &str,
        expected: &Patient,
    ) -> Result<bool, String> {
        let changed = self
            .conn
            .execute(
                "UPDATE patients SET last_name = ?1, first_name = ?2, birth_date = ?3,
                        phone = ?4, notes = ?5
                 WHERE id = ?6 AND last_name = ?7 AND first_name = ?8
                   AND birth_date = ?9 AND phone = ?10 AND notes = ?11",
                (
                    last_name,
                    first_name,
                    birth_date,
                    phone,
                    notes,
                    id,
                    &expected.last_name,
                    &expected.first_name,
                    &expected.birth_date,
                    &expected.phone,
                    &expected.notes,
                ),
            )
            .map_err(|e| e.to_string())?;
        Ok(changed == 1)
    }

    /// Remove a patient and every interview attached to them, atomically.
    pub fn delete_patient(&self, id: i64) -> Result<(), String> {
        let tx = self
            .conn
            .unchecked_transaction()
            .map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM interviews WHERE patient_id = ?1", [id])
            .map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM patients WHERE id = ?1", [id])
            .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())
    }

    pub fn interviews_for(&self, patient_id: i64) -> Result<Vec<Interview>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, kind, state, duration_minutes, scheduled_date, created_at
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
                ))
            })
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for row in rows {
            let (id, kind, state, duration_minutes, scheduled_date, created_at) =
                row.map_err(|e| e.to_string())?;
            out.push(Interview {
                id,
                duration_minutes,
                scheduled_date,
                kind: InterviewKind::parse(&kind)
                    .ok_or_else(|| format!("type d'entretien inconnu : {kind}"))?,
                state: InterviewState::parse(&state)
                    .ok_or_else(|| format!("état d'entretien inconnu : {state}"))?,
                created_at,
            });
        }
        Ok(out)
    }

    pub fn add_interview(&self, patient_id: i64, kind: InterviewKind) -> Result<i64, String> {
        self.conn
            .execute(
                "INSERT INTO interviews (patient_id, kind) VALUES (?1, ?2)",
                (patient_id, kind.as_str()),
            )
            .map_err(|e| e.to_string())?;
        Ok(self.conn.last_insert_rowid())
    }

    /// The drug reference base, alphabetical.
    pub fn drugs(&self) -> Result<Vec<Drug>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, name, dci, dosage, ddi, iup, antidote, notes
                 FROM drugs ORDER BY name COLLATE NOCASE",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok(Drug {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    dci: r.get(2)?,
                    dosage: r.get(3)?,
                    ddi: r.get(4)?,
                    iup: r.get(5)?,
                    antidote: r.get(6)?,
                    notes: r.get(7)?,
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
        for (name, dci, antidote) in STARTER_DRUGS {
            inserted += tx
                .execute(
                    "INSERT INTO drugs (name, dci, antidote)
                     SELECT ?1, ?2, ?3
                     WHERE NOT EXISTS (SELECT 1 FROM drugs WHERE name = ?1)",
                    (name, dci, antidote),
                )
                .map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok(inserted)
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
                "UPDATE drugs SET name = ?1, dci = ?2, dosage = ?3, ddi = ?4, iup = ?5,
                        antidote = ?6, notes = ?7
                 WHERE id = ?8 AND name = ?9 AND dci = ?10 AND dosage = ?11 AND ddi = ?12
                   AND iup = ?13 AND antidote = ?14 AND notes = ?15",
                rusqlite::params![
                    new.name,
                    new.dci,
                    new.dosage,
                    new.ddi,
                    new.iup,
                    new.antidote,
                    new.notes,
                    expected.id,
                    expected.name,
                    expected.dci,
                    expected.dosage,
                    expected.ddi,
                    expected.iup,
                    expected.antidote,
                    expected.notes,
                ],
            )
            .map_err(|e| e.to_string())?;
        Ok(changed == 1)
    }

    /// Remove a drug card; refused (`false`) if it was renamed meanwhile.
    pub fn delete_drug(&self, id: i64, expected_name: &str) -> Result<bool, String> {
        let changed = self
            .conn
            .execute(
                "DELETE FROM drugs WHERE id = ?1 AND name = ?2",
                (id, expected_name),
            )
            .map_err(|e| e.to_string())?;
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
                        i.duration_minutes
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
                ))
            })
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
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
            ) = row.map_err(|e| e.to_string())?;
            out.push(ExportRow {
                patient_name,
                phone,
                birth_date,
                created_date,
                scheduled_date,
                duration_minutes: minutes,
                kind: InterviewKind::parse(&kind)
                    .ok_or_else(|| format!("type d'entretien inconnu : {kind}"))?,
                state: InterviewState::parse(&state)
                    .ok_or_else(|| format!("état d'entretien inconnu : {state}"))?,
            });
        }
        Ok(out)
    }

    /// Planned interviews not yet performed, soonest first — the
    /// dashboard's appointment list (overdue ones included, so a missed
    /// RDV is never silently forgotten).
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
                        duration_minutes
                 FROM interviews",
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
                ))
            })
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for row in rows {
            let (kind, state, created_month, updated_month, duration_minutes) =
                row.map_err(|e| e.to_string())?;
            out.push(InterviewSummary {
                duration_minutes,
                kind: InterviewKind::parse(&kind)
                    .ok_or_else(|| format!("type d'entretien inconnu : {kind}"))?,
                state: InterviewState::parse(&state)
                    .ok_or_else(|| format!("état d'entretien inconnu : {state}"))?,
                created_month,
                updated_month,
            });
        }
        Ok(out)
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
    fn starter_drugs_seed_once_and_never_resurrect() {
        let dir = std::env::temp_dir().join(format!("bpm-caddy-sdrug-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("starter.db");
        let _ = std::fs::remove_file(&path);

        let db = Db::open(&path, "secret").unwrap();
        let n = db.seed_drugs_if_empty().unwrap();
        assert_eq!(n, STARTER_DRUGS.len());
        let eliquis = db
            .drugs()
            .unwrap()
            .into_iter()
            .find(|d| d.name == "Eliquis")
            .unwrap();
        assert_eq!(eliquis.dci, "apixaban");
        assert_eq!(eliquis.antidote, "Andexanet alfa");
        assert!(eliquis.dosage.is_empty());

        // Second run: no-op. After a deliberate deletion: still no-op.
        assert_eq!(db.seed_drugs_if_empty().unwrap(), 0);
        assert!(db.delete_drug(eliquis.id, "Eliquis").unwrap());
        assert_eq!(db.seed_drugs_if_empty().unwrap(), 0);
        assert!(db.drugs().unwrap().iter().all(|d| d.name != "Eliquis"));

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
        assert!(db.update_drug(&card, &base).unwrap());
        // A stale edit (based on the pre-update card) is refused.
        let mut stale = base.clone();
        stale.dosage = "2,5 mg x2/j".to_owned();
        assert!(!db.update_drug(&stale, &base).unwrap());
        let fresh = db.drugs().unwrap()[0].clone();
        assert_eq!(fresh.dosage, "5 mg x2/j");
        assert_eq!(fresh.antidote, "Andexanet alfa");

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
        assert!(db
            .update_patient(
                pid,
                "Dupont",
                "Jean",
                "1958-07-03",
                "06 12 34 56 78",
                "préfère le matin",
                &seen,
            )
            .unwrap());
        let p = db.patients().unwrap();
        assert_eq!(p[0].full_name(), "Jean Dupont");
        assert_eq!(p[0].birth_date, "1958-07-03");
        assert_eq!(p[0].phone, "06 12 34 56 78");
        assert_eq!(p[0].notes, "préfère le matin");

        // An edit based on the pre-correction snapshot is rejected
        // instead of silently overwriting the newer values.
        assert!(!db
            .update_patient(pid, "X", "Y", "1958-07-03", "", "", &seen)
            .unwrap());
        assert_eq!(db.patients().unwrap()[0].full_name(), "Jean Dupont");

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

        // Deletion removes the patient and their interviews atomically.
        db.add_interview(pid, InterviewKind::Bpm).unwrap();
        db.delete_patient(pid).unwrap();
        assert!(db.patients().unwrap().is_empty());
        assert!(db.interviews_for(pid).unwrap().is_empty());

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
            ),
            (
                "Martin",
                "Claire",
                "1949-02-11",
                InterviewKind::Bpm,
                2,
                50,
                None,
            ),
            (
                "Lefèvre",
                "Hélène",
                "1952-09-27",
                InterviewKind::Aod,
                1,
                30,
                Some(("+2 days", "06 12 34 56 78")),
            ),
            (
                "Bernard",
                "Paul",
                "1946-12-05",
                InterviewKind::Asthme,
                0,
                0,
                Some(("-3 days", "07 98 76 54 32")),
            ),
            (
                "Moreau",
                "Lucie",
                "1961-03-18",
                InterviewKind::Aod,
                3,
                35,
                None,
            ),
        ];
        // The starter base, plus one detailed card for the screenshots.
        db.seed_drugs_if_empty().unwrap();
        if let Some(base) = db
            .drugs()
            .unwrap()
            .into_iter()
            .find(|d| d.name == "Eliquis")
        {
            let mut card = base.clone();
            card.dosage = "5 mg x2/j (2,5 mg x2/j si ≥ 2 critères)".to_owned();
            card.ddi = "Inhibiteurs puissants CYP3A4/P-gp".to_owned();
            db.update_drug(&card, &base).unwrap();
        }

        for (last, first, dob, kind, advances, minutes, rdv) in seed {
            let pid = db.add_patient(last, first, dob).unwrap();
            let iid = db.add_interview(pid, kind).unwrap();
            let mut state = InterviewState::Identified;
            for _ in 0..advances {
                db.advance_interview(iid, state).unwrap();
                state = state.next().unwrap();
            }
            if minutes > 0 {
                db.set_duration(iid, minutes, 0).unwrap();
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
                    phone: String::new(),
                    notes: String::new(),
                };
                db.update_patient(pid, last, first, dob, phone, "", &seen)
                    .unwrap();
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
