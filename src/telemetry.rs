//! What this software does, counted where it happens and nowhere else.
//!
//! # A pane, not a beacon
//!
//! Nothing here is sent anywhere. There is no endpoint, no installation
//! identifier, no « usage statistics », and there will not be: this is
//! an application that holds health data, and the honest reading of the
//! word telemetry in that setting is *what the officine can see about
//! its own software*. The counters live in the officine's own encrypted
//! base, they are drawn in Options › À propos, and a guard below reads
//! this module's own text and refuses the day somebody adds a socket to
//! it.
//!
//! It is **opt-out**, and that is a deliberate direction rather than a
//! default nobody chose: a counter switched off by default is a counter
//! nobody ever turns on, so the pane would be empty on every post and
//! the switch would be decoration. It is off in one click, it is in the
//! same page as the version and the base, and turning it off stops the
//! counting on that post immediately.
//!
//! # It counts the software, never the person
//!
//! There is no operator here, no initials, no machine name — and that
//! is the rule the module is built around rather than an omission.
//! « Combien de dossiers Claire a-t-elle ouverts mardi » is a question
//! about a person, and a pharmacy's software is not the place it gets
//! answered by accident. What the officine gets is « combien de
//! dossiers ont été ouverts », which is what a decision is actually
//! made on. The register, the acts and the planning carry the operator
//! because a dispensing has to be attributable; a counter does not.
//!
//! # It counts, it never quotes
//!
//! No field is a string. A signal is an enum with nothing inside it,
//! and the French sentence is written by the screen that draws the
//! number. A message carries a fragment of what it is about — a name, a
//! path, a line — and the one place nobody looks for a leak is a log
//! somebody turned on to debug something else.
//!
//! # The day is passed in
//!
//! Pure, tested, no clock, like every other module here. And the day
//! matters more than it looks: an officine on garde works through
//! midnight, and **a night shift does not put Tuesday's work on
//! Monday's line** — [`Counters::turn`] is what stops it, by handing
//! back what is owed to the old day before the new one is counted into.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::strings::tr;

/// One thing the application does, worth counting.
///
/// The `key` is written into the base, so it is **stable for ever**: a
/// renamed key is a column of history that silently stops adding up.
/// A test holds that, and holds that no two share one.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Signal {
    /// The application was opened and its base unlocked.
    Opened,
    /// A patient file was opened.
    File,
    /// A drug card was read.
    Card,
    /// A document was printed.
    Printed,
    /// An act was recorded.
    Act,
    /// A line was written to the register of stupéfiants.
    Register,
    /// A till was counted.
    Till,
    /// A backup was written.
    Backup,
    /// One of the long passes over the base was run.
    Pass,
}

// Deliberately **not** here, and it is worth saying why rather than
// leaving a gap: « combien de fois un autre poste avait écrit le
// premier » is the most useful number this pane could carry — it is how
// an officine discovers that two people work on one thing at once, and
// no other screen says it. It is absent because every compare-and-set
// answers `false` in its own place, and there is no single one to count
// at; counting at twenty of them would eventually miss the
// twenty-first, and a counter that is quietly short is worse than one
// that is not there. It arrives the day those answers go through one
// function. The same goes for a sync conversation: `bpm-sync` is an
// optional crate, so in a shipped binary that counter would read zero
// for ever — which is not a figure, it is a lie with a number on it.

impl Signal {
    /// Every signal, so a pane and a test cannot miss one.
    pub const ALL: [Signal; 9] = [
        Signal::Opened,
        Signal::File,
        Signal::Card,
        Signal::Printed,
        Signal::Act,
        Signal::Register,
        Signal::Till,
        Signal::Backup,
        Signal::Pass,
    ];

    /// What is written in the base. Never changes.
    pub fn key(self) -> &'static str {
        match self {
            Signal::Opened => "ouverture",
            Signal::File => "dossier",
            Signal::Card => "fiche",
            Signal::Printed => "impression",
            Signal::Act => "acte",
            Signal::Register => "registre",
            Signal::Till => "caisse",
            Signal::Backup => "sauvegarde",
            Signal::Pass => "passe",
        }
    }

    /// A key read back out of the base.
    ///
    /// `None` for one this version does not know — a base written by a
    /// later version, whose row is **kept** and simply not drawn, the
    /// same answer `Stream::Autre` gives in `bpm-sync`. Dropping it
    /// would lose an officine's own figures on the day one post is
    /// upgraded before the others.
    pub fn from_key(key: &str) -> Option<Signal> {
        Signal::ALL.into_iter().find(|s| s.key() == key)
    }

    /// The French label the pane draws. A literal per arm, not a
    /// `format!` — the strings guard reads the source for its keys, and
    /// a key it cannot see is a key it reports as unused.
    pub fn label(self) -> &'static str {
        match self {
            Signal::Opened => tr("telem_ouverture"),
            Signal::File => tr("telem_dossier"),
            Signal::Card => tr("telem_fiche"),
            Signal::Printed => tr("telem_impression"),
            Signal::Act => tr("telem_acte"),
            Signal::Register => tr("telem_registre"),
            Signal::Till => tr("telem_caisse"),
            Signal::Backup => tr("telem_sauvegarde"),
            Signal::Pass => tr("telem_passe"),
        }
    }
}

/// What the modules have counted at home, since this process started.
///
/// Five of the nine signals are written in **one** place each — every
/// printable document goes through `pdf::compile_and_open`, every act
/// through `Db::add_interview_by`, every register line through
/// `Db::add_stup_moves` — and counting there is one line instead of
/// twenty-one chances to forget one. A process counter rather than a
/// field, because those functions have no session to hand it to.
///
/// Nothing leaves this array. The session reads its **difference** at
/// each flush and carries it into [`Counters`], which is where the
/// switch is checked: a post that does not count records nothing, and
/// what this tally held is dropped on the floor.
static TALLY: [AtomicU64; Signal::ALL.len()] = [const { AtomicU64::new(0) }; Signal::ALL.len()];

/// Counted where the thing happens. Saturating, like everything here.
pub fn tally(signal: Signal) {
    let slot = &TALLY[index(signal)];
    let _ = slot.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| {
        Some(n.saturating_add(1))
    });
}

fn tallied(signal: Signal) -> u64 {
    TALLY[index(signal)].load(Ordering::Relaxed)
}

/// What one day at one post has counted so far.
///
/// Held in memory and written to the base in one pass — a counter that
/// wrote a row per event would be a write per keystroke on a base that
/// sits on the officine's network share.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Counters {
    day: String,
    on: bool,
    counts: [u64; Signal::ALL.len()],
    /// What [`TALLY`] read last time, so only the difference is taken.
    /// Started at the tally rather than at zero: a session opened after
    /// something was already counted must not claim it.
    seen: [u64; Signal::ALL.len()],
}

/// What is owed to the base: a day, and the signals that moved.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Flush {
    pub day: String,
    /// Only what actually happened. A row of zeros is a row that says
    /// « rien » where nothing asked the question.
    pub counts: Vec<(&'static str, u64)>,
}

impl Counters {
    /// Fresh counters for `day`. `on` is `[telemetry] enabled`.
    pub fn new(day: &str, on: bool) -> Self {
        Self {
            day: day.to_owned(),
            on,
            counts: [0; Signal::ALL.len()],
            seen: Signal::ALL.map(tallied),
        }
    }

    pub fn on(&self) -> bool {
        self.on
    }

    /// Turns the counting off, or back on.
    ///
    /// Off is checked **here**, in the one place counting happens,
    /// rather than at each of the twenty call sites — one of which
    /// would eventually be written without the check, and nothing
    /// would say so.
    pub fn set_on(&mut self, on: bool) {
        self.on = on;
    }

    /// Counts one. Saturating: a counter that wraps round to zero
    /// reads as « rien ne s'est passé », which is the one answer it
    /// must never give.
    pub fn note(&mut self, signal: Signal) {
        self.add(signal, 1);
    }

    /// Counts `n` at once — what [`Counters::gather`] carries over.
    pub fn add(&mut self, signal: Signal, n: u64) {
        if !self.on || n == 0 {
            return;
        }
        let slot = &mut self.counts[index(signal)];
        *slot = slot.saturating_add(n);
    }

    /// Takes over what the modules counted at home since last time.
    ///
    /// Called before anything is written, so a document printed thirty
    /// seconds ago is on the right day. The difference is read whether
    /// the post counts or not — `add` is what drops it when it does
    /// not, so turning the switch back on never back-dates what
    /// happened while it was off.
    pub fn gather(&mut self) {
        for (i, signal) in Signal::ALL.into_iter().enumerate() {
            let now = tallied(signal);
            let since = now.saturating_sub(self.seen[i]);
            self.seen[i] = now;
            self.add(signal, since);
        }
    }

    pub fn count(&self, signal: Signal) -> u64 {
        self.counts[index(signal)]
    }

    /// Is there anything worth writing?
    pub fn pending(&self) -> bool {
        self.counts.iter().any(|n| *n > 0)
    }

    /// Takes what is owed and starts the same day again.
    pub fn take(&mut self) -> Flush {
        let counts = Signal::ALL
            .into_iter()
            .filter(|s| self.count(*s) > 0)
            .map(|s| (s.key(), self.count(s)))
            .collect();
        self.counts = [0; Signal::ALL.len()];
        Flush {
            day: self.day.clone(),
            counts,
        }
    }

    /// The day has moved on: hands back what the old one is owed and
    /// starts counting into the new one.
    ///
    /// An officine on garde works through midnight, and a night shift
    /// does not put Tuesday's work on Monday's line. `None` while the
    /// day is the same, so the caller may call it as often as it likes.
    pub fn turn(&mut self, day: &str) -> Option<Flush> {
        if day == self.day {
            return None;
        }
        let owed = self.take();
        self.day = day.to_owned();
        (!owed.counts.is_empty()).then_some(owed)
    }
}

fn index(signal: Signal) -> usize {
    Signal::ALL
        .iter()
        .position(|s| *s == signal)
        .unwrap_or_default()
}

/// The totals a pane draws, and **over how many days** they run.
///
/// The second half is not decoration. A cumulative figure with no
/// period beside it reads as though it covered everything there has
/// ever been — the rule the caisse's monthly summary already follows,
/// and the reason it says over how many evenings its gap is computed.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Span {
    pub days: usize,
    pub first: String,
    pub last: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_is_counted_is_read_back() {
        let mut c = Counters::new("2026-09-19", true);
        c.note(Signal::File);
        c.note(Signal::File);
        c.note(Signal::Printed);
        assert_eq!(c.count(Signal::File), 2);
        assert_eq!(c.count(Signal::Printed), 1);
        assert_eq!(c.count(Signal::Act), 0);
        assert!(c.pending());

        let owed = c.take();
        assert_eq!(owed.day, "2026-09-19");
        // Only what moved: a row of zeros says « rien » where nothing
        // asked the question.
        assert_eq!(owed.counts, vec![("dossier", 2), ("impression", 1)]);
        assert!(!c.pending(), "ce qui est rendu n'est pas compté deux fois");
        assert_eq!(
            c.take().day,
            "2026-09-19",
            "la journée ne bouge pas toute seule"
        );
    }

    /// The switch is checked in the one place counting happens. Twenty
    /// call sites each checking for themselves is nineteen chances to
    /// forget and one silent leak of a figure nobody consented to.
    #[test]
    fn off_means_nothing_is_counted() {
        let mut c = Counters::new("2026-09-19", false);
        for signal in Signal::ALL {
            c.note(signal);
            assert_eq!(c.count(signal), 0, "{signal:?}");
        }
        assert!(!c.pending());
        assert!(c.take().counts.is_empty());

        // Turned back on, it counts again — and does not invent what
        // happened while it was off.
        c.set_on(true);
        c.note(Signal::Act);
        assert_eq!(c.count(Signal::Act), 1);
        assert_eq!(c.count(Signal::File), 0);
    }

    /// An officine on garde works through midnight.
    #[test]
    fn a_night_shift_does_not_put_tuesdays_work_on_mondays_line() {
        let mut c = Counters::new("2026-09-19", true);
        c.note(Signal::Register);
        // Same day, nothing owed: the caller may ask at every frame.
        assert_eq!(c.turn("2026-09-19"), None);
        assert_eq!(c.count(Signal::Register), 1);

        let owed = c.turn("2026-09-20").expect("la veille est due");
        assert_eq!(owed.day, "2026-09-19");
        assert_eq!(owed.counts, vec![("registre", 1)]);
        assert_eq!(c.take().day, "2026-09-20");
        assert_eq!(c.count(Signal::Register), 0, "le jour neuf part de zéro");

        // A day that turns with nothing counted owes nothing, rather
        // than writing a line of zeros for every quiet Sunday.
        assert_eq!(c.turn("2026-09-21"), None);
        assert_eq!(c.take().day, "2026-09-21");
    }

    /// A counter that wraps reads as « rien ne s'est passé ».
    #[test]
    fn a_counter_stops_rather_than_turning_over() {
        let mut c = Counters::new("2026-09-19", true);
        c.counts[index(Signal::Card)] = u64::MAX;
        c.note(Signal::Card);
        assert_eq!(c.count(Signal::Card), u64::MAX);
    }

    /// The keys are written into the base, so they are stable for ever
    /// and no two share one. A renamed key is a column of an officine's
    /// history that silently stops adding up.
    #[test]
    fn a_signals_key_is_stable_and_its_own() {
        let mut seen = std::collections::HashSet::new();
        for signal in Signal::ALL {
            assert!(seen.insert(signal.key()), "{signal:?} partage sa clé");
            assert_eq!(Signal::from_key(signal.key()), Some(signal));
            assert!(!signal.key().is_empty());
            // Written into a base and read back by another post: no
            // accent, no space, nothing a later version would spell
            // differently.
            assert!(
                signal.key().chars().all(|c| c.is_ascii_lowercase()),
                "{signal:?} : « {} »",
                signal.key()
            );
            assert!(!signal.label().is_empty(), "{signal:?} n'a pas d'intitulé");
        }
        // The exact keys, spelled out: renaming one has to fail here
        // rather than in an officine's figures six months later.
        assert_eq!(
            Signal::ALL.map(Signal::key),
            [
                "ouverture",
                "dossier",
                "fiche",
                "impression",
                "acte",
                "registre",
                "caisse",
                "sauvegarde",
                "passe",
            ]
        );
    }

    /// A key a later version wrote is not an error: the row stays in the
    /// base and is simply not drawn. Dropping it would lose an
    /// officine's own figures the day one post is upgraded first.
    #[test]
    fn a_signal_this_version_does_not_know_is_not_a_fault() {
        assert_eq!(Signal::from_key("ce-que-fera-la-suite"), None);
        assert_eq!(Signal::from_key(""), None);
        assert_eq!(Signal::from_key("Dossier"), None, "la casse compte");
    }

    /// The rule the module exists under, held by its own text: nothing
    /// here reaches the world, nothing here holds a sentence, and
    /// nothing here knows who anybody is.
    #[test]
    fn telemetry_counts_the_software_and_never_the_person_and_never_leaves() {
        let text = include_str!("telemetry.rs");
        let code = text.split("mod tests").next().unwrap();
        let body: String = code
            .lines()
            .filter(|l| {
                let t = l.trim_start();
                !t.starts_with("//")
            })
            .collect::<Vec<_>>()
            .join("\n");

        for outward in ["ureq", "std::net", "std::fs", "reqwest", "http", "Command"] {
            assert!(
                !body.contains(outward),
                "la télémétrie ne sort pas : « {outward} »"
            );
        }
        for who in ["operator", "operateur", "initials", "user", "patient"] {
            assert!(
                !body.contains(who),
                "la télémétrie ne compte pas les gens : « {who} »"
            );
        }
        // The only text this module holds is a **date**: the day a
        // count belongs to, and the two ends of the period a pane
        // draws. Every label comes from the strings file. A count of
        // occurrences would be a magic number nobody could read, so
        // what is checked is the line each one sits on.
        for line in body.lines().filter(|l| l.contains("String")) {
            assert!(
                ["day", "first", "last", "let body"]
                    .iter()
                    .any(|allowed| line.contains(allowed)),
                "la télémétrie ne tient que des dates : « {} »",
                line.trim()
            );
        }
    }
}
