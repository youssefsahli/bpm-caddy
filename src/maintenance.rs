//! The long passes over the base, off the painting thread.
//!
//! « Synchroniser le contenu de référence » pours into the base
//! everything this version ships that it has not got: the fiches, their
//! details, the posologies, the conduites, the toxicities, the
//! préparations, the dispositifs, the protocoles. Eight passes over
//! eight hundred and fifty cards. It used to run where it was pressed —
//! inside `update`, between two frames — so the application stopped
//! answering the mouse until it was over, with no window, no cursor and
//! no way to tell a slow pass from a dead one. On a base sitting on the
//! officine's network share that is minutes.
//!
//! So it runs on a thread of its own, which opens **its own
//! connection** to the same file rather than borrowing the session's:
//! `rusqlite::Connection` is not `Sync`, and the base is shared between
//! PCs anyway — a second reader from the same process is the case it was
//! already built for, `busy_timeout` and all.
//!
//! What crosses back is a channel of [`Progress`]: one message as each
//! step starts, one at the end. The interface reads it where it reads
//! the update check, repaints while a job is running, and reloads its
//! caches when the last message says the work is done.
//!
//! The shape of the work — which passes, in what order, under what
//! label — is [`steps`], a pure function over a [`Job`]. That is the
//! half worth testing, and it is tested: every step is named in the
//! French strings, no pass is run twice inside one job, and the whole
//! catalogue actually runs on a real base.

use std::path::PathBuf;
use std::sync::mpsc::Receiver;

use crate::db::Db;

/// One named pass over the base.
pub struct Step {
    /// The key of its French label in `assets/strings.fr.toml`.
    pub key: &'static str,
    /// What it does. Returns what it filled, added or replaced.
    pub run: fn(&Db) -> Result<usize, String>,
}

/// What the operator asked for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Job {
    /// Options › À propos › « Synchroniser le contenu de référence ».
    Sync,
    /// Options › Base › « Compléter les médicaments de départ ».
    SeedMissing,
    /// Options › Base › « Compléter les fiches de référence ».
    FillDetails,
    /// Options › Base › « Réinitialiser la base… ». Two clicks, red.
    Reset,
    /// Options › Base › « Compacter la base ». Rend au disque la place
    /// que les suppressions ont laissée — c'est la seule passe qui ne
    /// sème rien et ne fait que reprendre.
    Compact,
}

impl Job {
    /// Every job, so a test cannot forget one when a job is added.
    ///
    /// `#[cfg(test)]` like [`run_all`] below: nothing in the application
    /// enumerates the jobs — each button names the one it starts — but
    /// the tests must, or a fifth job could ship with no French label
    /// and nobody would hear about it.
    #[cfg(test)]
    pub const ALL: [Job; 5] = [
        Job::Sync,
        Job::SeedMissing,
        Job::FillDetails,
        Job::Compact,
        Job::Reset,
    ];
}

/// The eight passes that carry this version's reference content into a
/// base, in the order the About page has always run them.
///
/// `insert_missing_drugs` rather than `seed_missing_drugs`: the latter
/// is a composite that calls six of the seven below it, so listing it
/// here would run each of them twice — once inside it, once after it.
const CONTENT: &[Step] = &[
    Step {
        key: "maint_step_drugs",
        run: Db::insert_missing_drugs,
    },
    Step {
        key: "maint_step_details",
        run: Db::fill_starter_details,
    },
    Step {
        key: "maint_step_posologies",
        run: Db::seed_posologies,
    },
    Step {
        key: "maint_step_conduite",
        run: Db::seed_conduite,
    },
    Step {
        key: "maint_step_toxicity",
        run: Db::refresh_toxicity,
    },
    Step {
        key: "maint_step_preparations",
        run: Db::seed_preparations,
    },
    Step {
        key: "maint_step_dispositifs",
        run: Db::seed_dispositifs,
    },
    Step {
        key: "maint_step_protocols",
        run: Db::seed_protocols,
    },
];

/// Only the details pass, for the button that offers just that.
const DETAILS: &[Step] = &[Step {
    key: "maint_step_details",
    run: Db::fill_starter_details,
}];

/// Emptying every table, then putting the shipped content back. The
/// wipe is its own step because it is the one the operator is anxious
/// about: it should be seen to start and to finish.
const RESET: &[Step] = &[Step {
    key: "maint_step_wipe",
    run: Db::wipe_all_data,
}];

/// Reprendre la place : déplacer ce qui restait de pièces dans la base
/// principale, balayer les octets orphelins, puis réécrire les deux
/// fichiers sans leurs pages libres.
///
/// Dans cet ordre, et l'ordre est la moitié du travail : compacter avant
/// d'avoir déplacé réécrirait la base principale avec les pièces encore
/// dedans, c'est-à-dire tout le contraire de ce qu'on demande.
const COMPACT: &[Step] = &[
    Step {
        key: "maint_step_move_scans",
        run: Db::move_scans_out,
    },
    Step {
        key: "maint_step_sweep_scans",
        run: Db::sweep_orphan_scans,
    },
    Step {
        key: "maint_step_compact",
        run: Db::compact,
    },
    Step {
        key: "maint_step_compact_scans",
        run: Db::compact_scans,
    },
];

/// The passes a job runs, in order.
///
/// « Compléter les médicaments de départ » and « Synchroniser » have
/// grown into the same list — the first one's own composite already
/// called six of the eight — and they differ only in what they report:
/// the one says how many *cards* it added (the first step's count), the
/// other how many *elements* in all.
pub fn steps(job: Job) -> Vec<&'static Step> {
    match job {
        Job::Sync | Job::SeedMissing => CONTENT.iter().collect(),
        Job::FillDetails => DETAILS.iter().collect(),
        Job::Compact => COMPACT.iter().collect(),
        Job::Reset => RESET.iter().chain(CONTENT.iter()).collect(),
    }
}

/// What the worker says while it works.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Progress {
    /// Step `index` of `total` is starting; `key` names it in French.
    Started {
        index: usize,
        total: usize,
        key: &'static str,
    },
    /// Every step has run, or one of them refused to.
    Done(Outcome),
}

/// The end of a job: what each step did, and the first thing that went
/// wrong if anything did.
///
/// One failing step must not hide the seven that worked, so the counts
/// are reported either way and the error travels beside them — the same
/// rule the synchronous version already followed.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Outcome {
    pub job: Job,
    /// One count per step of [`steps`], in the same order.
    pub counts: Vec<usize>,
    pub error: Option<String>,
}

impl Outcome {
    /// Everything the job filled, added or replaced.
    pub fn total(&self) -> usize {
        self.counts.iter().sum()
    }

    /// What the first step did — for « Compléter les médicaments de
    /// départ », which reports cards added and not fields filled.
    pub fn first(&self) -> usize {
        self.counts.first().copied().unwrap_or(0)
    }
}

/// Run every step of `job` here and now, reporting to nobody.
///
/// The synchronous form: [`spawn`] is the same work with a channel and
/// a thread around it. Only the tests want this — the application never
/// runs a pass where it would be waited on, which is the whole point of
/// this module — but they want it badly, because the alternative is
/// each of them listing the eight passes by hand and drifting from the
/// button the day a ninth is added.
#[cfg(test)]
pub fn run_all(db: &Db, job: Job) -> Outcome {
    run(db, job, &mut |_| true)
}

/// Run `job` on a thread of its own against the base at `path`.
///
/// Returns the receiving end straight away, like
/// [`crate::release::check_async`]: the interface goes on painting and
/// reads the messages as they arrive.
pub fn spawn(path: PathBuf, password: String, job: Job) -> Receiver<Progress> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let db = match Db::open(&path, &password) {
            Ok(db) => db,
            Err(e) => {
                let _ = tx.send(Progress::Done(Outcome {
                    job,
                    counts: Vec::new(),
                    error: Some(e),
                }));
                return;
            }
        };
        let _ = run(&db, job, &mut |m| {
            // The interface may have been closed under us; there is
            // nothing useful to do about it but stop shouting.
            tx.send(m).is_ok()
        });
    });
    rx
}

/// The job itself, with nowhere to hide: a `Db` and somewhere to report.
///
/// `report` answers whether anyone is still listening, so a job whose
/// window has gone stops at the next step instead of finishing a wipe
/// and a reseed into a base nobody will read.
fn run(db: &Db, job: Job, report: &mut dyn FnMut(Progress) -> bool) -> Outcome {
    let steps = steps(job);
    let total = steps.len();
    let mut counts = Vec::with_capacity(total);
    let mut error: Option<String> = None;
    for (index, step) in steps.iter().enumerate() {
        if !report(Progress::Started {
            index,
            total,
            key: step.key,
        }) {
            break;
        }
        match (step.run)(db) {
            Ok(n) => counts.push(n),
            Err(e) => {
                counts.push(0);
                error = error.or(Some(e));
            }
        }
    }
    let outcome = Outcome { job, counts, error };
    report(Progress::Done(outcome.clone()));
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every step's label exists in the French strings. A step whose key
    /// is not there shows the key itself on screen, which is how a
    /// pharmacist ends up looking at `maint_step_dispositifs`.
    #[test]
    fn every_step_is_named_in_french() {
        for job in Job::ALL {
            for step in steps(job) {
                let label = crate::strings::tr(step.key);
                assert_ne!(label, step.key, "clé sans libellé : {}", step.key);
                assert!(!label.is_empty(), "libellé vide : {}", step.key);
            }
        }
    }

    /// No job runs the same pass twice. They are all idempotent, so a
    /// repeat would be harmless — and it would also be the whole reason
    /// « Synchroniser » took twice as long as it needed to, which is not
    /// harmless at four minutes.
    #[test]
    fn no_job_runs_the_same_pass_twice() {
        for job in Job::ALL {
            let mut seen = std::collections::HashSet::new();
            for step in steps(job) {
                assert!(seen.insert(step.key), "{:?} répète {}", job, step.key);
            }
        }
    }

    /// The reset wipes first and reseeds after — in that order, or it
    /// deletes what it has just put back.
    #[test]
    fn the_reset_empties_before_it_refills() {
        let steps = steps(Job::Reset);
        assert_eq!(steps[0].key, "maint_step_wipe");
        assert!(steps.len() > 1, "un effacement sans remise en place");
        assert_eq!(steps[1].key, "maint_step_drugs");
    }

    /// The job actually runs, reports one message per step plus the
    /// end, and is idempotent: a second pass over the same base fills
    /// nothing, because the first one filled it all.
    #[test]
    fn a_job_runs_every_step_and_says_so() {
        let dir = std::env::temp_dir().join(format!("bpm-caddy-maint-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let _swept = crate::db::Swept(dir.clone());
        let db = Db::open(&dir.join("maint.db"), "secret").unwrap();

        let mut seen: Vec<Progress> = Vec::new();
        let first = run(&db, Job::Sync, &mut |m| {
            seen.push(m);
            true
        });
        let n = steps(Job::Sync).len();
        assert_eq!(seen.len(), n + 1, "un message par étape, plus la fin");
        assert_eq!(first.counts.len(), n);
        assert!(first.error.is_none(), "{:?}", first.error);
        assert!(first.total() > 0, "une base neuve doit se remplir");
        // The first step is the cards themselves.
        assert_eq!(first.first(), crate::db::STARTER_DRUG_COUNT);

        // Pressed again on a base already at this version: nothing left.
        let again = run(&db, Job::Sync, &mut |_| true);
        assert_eq!(again.total(), 0, "la synchronisation doit être idempotente");
    }

    /// A job whose window has closed stops at the next step rather than
    /// carrying on writing into a base nobody is looking at.
    #[test]
    fn a_job_nobody_is_listening_to_stops() {
        let dir = std::env::temp_dir().join(format!("bpm-caddy-maint-x-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let _swept = crate::db::Swept(dir.clone());
        let db = Db::open(&dir.join("maint.db"), "secret").unwrap();

        let mut ran = 0;
        let out = run(&db, Job::Sync, &mut |m| {
            if matches!(m, Progress::Started { .. }) {
                ran += 1;
            }
            // The receiver is gone after the first step.
            false
        });
        assert_eq!(ran, 1, "une étape commencée, et on s'arrête");
        assert_eq!(out.counts.len(), 0, "l'étape annoncée n'a pas tourné");
    }

    /// The end of a job reports both numbers the two buttons need.
    #[test]
    fn an_outcome_reports_the_first_count_and_the_whole() {
        let out = Outcome {
            job: Job::SeedMissing,
            counts: vec![3, 40, 5],
            error: None,
        };
        assert_eq!(out.first(), 3, "les fiches ajoutées");
        assert_eq!(out.total(), 48, "tout ce qui a été versé");
        let empty = Outcome {
            job: Job::Reset,
            counts: Vec::new(),
            error: Some("plus de place".to_owned()),
        };
        assert_eq!(empty.first(), 0);
        assert_eq!(empty.total(), 0);
    }

    /// A base that cannot be opened is reported as such, and does not
    /// take the thread down with it.
    #[test]
    fn a_base_that_will_not_open_comes_back_as_an_error() {
        let dir = std::env::temp_dir().join(format!("bpm-caddy-maint-e-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let _swept = crate::db::Swept(dir.clone());
        let path = dir.join("real.db");
        drop(Db::open(&path, "secret").unwrap());

        let rx = spawn(path, "le mauvais mot de passe".to_owned(), Job::Sync);
        let msg = rx.recv().expect("le fil doit répondre");
        match msg {
            Progress::Done(out) => {
                assert!(out.error.is_some(), "un mot de passe faux est une erreur");
                assert!(out.counts.is_empty());
            }
            other => panic!("{other:?}"),
        }
    }
}
