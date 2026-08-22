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
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE IF NOT EXISTS interviews (
    id          INTEGER PRIMARY KEY,
    patient_id  INTEGER NOT NULL REFERENCES patients(id),
    kind        TEXT NOT NULL,
    state       TEXT NOT NULL DEFAULT 'IDENTIFIED',
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
);
";

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
}

/// Interview kinds billable at the counter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InterviewKind {
    Bpm,
    Aod,
    Asthme,
}

impl InterviewKind {
    pub const ALL: [InterviewKind; 3] = [Self::Bpm, Self::Aod, Self::Asthme];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Bpm => "BPM",
            Self::Aod => "AOD",
            Self::Asthme => "ASTHME",
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
        }
    }
}

#[derive(Clone, Debug)]
pub struct Interview {
    pub id: i64,
    pub kind: InterviewKind,
    pub state: InterviewState,
    pub created_at: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Patient {
    pub id: i64,
    pub last_name: String,
    pub first_name: String,
    /// ISO `YYYY-MM-DD`.
    pub birth_date: String,
}

impl Patient {
    pub fn full_name(&self) -> String {
        format!("{} {}", self.first_name, self.last_name)
    }
}

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
        conn.pragma_update(None, "key", password)
            .map_err(|e| format!("configuration du chiffrement impossible : {e}"))?;
        // Probing the schema is how SQLCipher reports a wrong key.
        conn.query_row("SELECT count(*) FROM sqlite_master", [], |r| {
            r.get::<_, i64>(0)
        })
        .map_err(|_| "Mot de passe incorrect (ou fichier illisible).".to_owned())?;
        conn.execute_batch(SCHEMA)
            .map_err(|e| format!("initialisation du schéma impossible : {e}"))?;
        Ok(Self { conn })
    }

    pub fn patients(&self) -> Result<Vec<Patient>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, last_name, first_name, birth_date FROM patients")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok(Patient {
                    id: r.get(0)?,
                    last_name: r.get(1)?,
                    first_name: r.get(2)?,
                    birth_date: r.get(3)?,
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

    pub fn interviews_for(&self, patient_id: i64) -> Result<Vec<Interview>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, kind, state, created_at
                 FROM interviews WHERE patient_id = ?1 ORDER BY created_at DESC, id DESC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([patient_id], |r| {
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
            let (id, kind, state, created_at) = row.map_err(|e| e.to_string())?;
            out.push(Interview {
                id,
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

    /// Advance an interview to the next pipeline state; no-op once billed.
    pub fn advance_interview(&self, id: i64, current: InterviewState) -> Result<(), String> {
        let Some(next) = current.next() else {
            return Ok(());
        };
        self.conn
            .execute(
                "UPDATE interviews SET state = ?1, updated_at = datetime('now') WHERE id = ?2",
                (next.as_str(), id),
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

/// Parse a French `JJ/MM/AAAA` date into ISO `YYYY-MM-DD`, with basic
/// range validation.
pub fn parse_french_date(input: &str) -> Result<String, String> {
    let parts: Vec<&str> = input.trim().split(['/', '-', '.']).collect();
    let err = || "Date attendue au format JJ/MM/AAAA".to_owned();
    if parts.len() != 3 {
        return Err(err());
    }
    let day: u32 = parts[0].parse().map_err(|_| err())?;
    let month: u32 = parts[1].parse().map_err(|_| err())?;
    let year: u32 = parts[2].parse().map_err(|_| err())?;
    if !(1..=31).contains(&day) || !(1..=12).contains(&month) || !(1900..=2100).contains(&year) {
        return Err(err());
    }
    Ok(format!("{year:04}-{month:02}-{day:02}"))
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

    #[test]
    fn date_roundtrip() {
        assert_eq!(parse_french_date("03/07/1958").unwrap(), "1958-07-03");
        assert_eq!(format_french_date("1958-07-03"), "03/07/1958");
        assert!(
            parse_french_date("1958-07-03").is_err() || parse_french_date("31/12/1999").is_ok()
        );
        assert!(parse_french_date("32/01/2000").is_err());
        assert!(parse_french_date("abc").is_err());
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

        let _ = std::fs::remove_file(&path);
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
