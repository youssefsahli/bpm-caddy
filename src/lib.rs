//! BPM-Caddy as a library: every module the application is made of.
//!
//! **Two programmes, one reading of the base.** `bpm-caddy` (the counter
//! application) and `bpm-audit` (the officine's audit window) are two
//! binaries over this one crate: a second programme with its own copy of
//! the schema would be a second writing of it, and two writings of a
//! schema end up differing — the day they do, it is the one nobody runs
//! that looks right.

pub mod agenda;
pub mod annuaire;
pub mod app;
pub mod audit;
pub mod audit_window;
pub mod biology;
pub mod bulletin;
pub mod caisse;
pub mod classes;
pub mod codebar;
pub mod codex;
pub mod conciliation;
pub mod config;
pub mod content;
pub mod crush;
pub mod cyp;
pub mod date;
pub mod db;
pub mod dosing;
pub mod elderly;
pub mod entretien;
pub mod facets;
pub mod favorites;
pub mod fuzzy;
pub mod graph;
pub mod gravidity;
pub mod hepatic;
pub mod insulin;
pub mod intake;
pub mod location;
pub mod maintenance;
pub mod messages;
#[cfg(feature = "sync")]
pub mod network;
pub mod ordonnance;
pub mod ordonnancier;
pub mod pdf;
pub mod pk;
pub mod planning;
#[cfg(feature = "sync")]
pub mod postes;
pub mod prescribers;
pub mod release;
pub mod renal;
pub mod renewal;
pub mod replica;
pub mod revue;
pub mod ruptures;
pub mod scans;
pub mod script;
pub mod selfcheck;
pub mod strings;
pub mod surveillance;
pub mod tables;
pub mod telemetry;
pub mod timeline;
pub mod trod;
pub mod vaccines;
pub mod vaccsheet;
pub mod versions;
pub mod vigilance;
pub mod vitale;
pub mod winscard;
