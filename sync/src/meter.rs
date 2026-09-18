//! The telemetry, and what the word is allowed to mean here.
//!
//! This module holds numbers about the module itself: how many times a
//! link opened, how many records crossed, how many bytes, how many
//! refusals and of what kind. It exists because a sync that fails
//! quietly is worse than one that fails loudly — somebody has to be able
//! to open a pane and see that this post has not spoken to the other one
//! since Tuesday.
//!
//! **It is a pane, not a beacon.** Nothing here sends anything anywhere.
//! There is no endpoint, no identifier of an installation, no « usage
//! statistics », and there will not be: this is an application that
//! holds health data, and the honest reading of the word telemetry in
//! that setting is *what the officine can see about its own machines*.
//! A guard below reads this module's own text and refuses the day
//! somebody adds a socket to it.
//!
//! **It counts, it never quotes.** No field here is a string. A message
//! carries a fragment of what it is about — a name, a path, a line — and
//! the one place nobody looks for a leak is a log somebody turned on to
//! debug something else. So a failure is an [`Error`], which is an enum
//! with nothing inside it, and the French sentence is written by the
//! screen that shows the number.

use crate::keys::Fingerprint;
use crate::Error;

/// What can be counted.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Signal {
    /// A conversation with a peer began.
    Session,
    /// The handshake completed and the peer proved which post it is.
    Handshake,
    /// A pairing was carried out: a new post joined the officine.
    Pairing,
    /// A record was offered to the peer.
    Sent,
    /// A record arrived and was new.
    Received,
    /// A record arrived that this post already had. The ordinary case,
    /// counted because a sync that is *only* this is a sync with
    /// nothing to do, which is a useful thing to see.
    Known,
    /// A conversation ended with both ends holding the same journal.
    Agreed,
}

impl Signal {
    pub const ALL: [Signal; 7] = [
        Signal::Session,
        Signal::Handshake,
        Signal::Pairing,
        Signal::Sent,
        Signal::Received,
        Signal::Known,
        Signal::Agreed,
    ];

    fn index(self) -> usize {
        Self::ALL.iter().position(|s| *s == self).unwrap_or(0)
    }
}

/// Why something was refused.
///
/// The same vocabulary the crate refuses in, deliberately: a second list
/// of reasons beside the first is two writings of one thing, and they
/// drift — the day they disagree, the pane counts a refusal the code no
/// longer makes.
pub type Refusal = Error;

/// The counters of one post.
#[derive(Clone, Default)]
pub struct Meter {
    signals: [u64; Signal::ALL.len()],
    refusals: [u64; Error::ALL.len()],
    in_bytes: u64,
    out_bytes: u64,
    peer: Option<Fingerprint>,
}

impl Meter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Counts one. **Saturating**: a counter that wraps round to zero
    /// after a long run reads as « nothing happened », which is the one
    /// answer it must never give.
    pub fn note(&mut self, signal: Signal) {
        let slot = &mut self.signals[signal.index()];
        *slot = slot.saturating_add(1);
    }

    pub fn refuse(&mut self, refusal: Refusal) {
        let slot = &mut self.refusals[refusal.index()];
        *slot = slot.saturating_add(1);
    }

    pub fn read_bytes(&mut self, n: usize) {
        self.in_bytes = self.in_bytes.saturating_add(n as u64);
    }

    pub fn wrote_bytes(&mut self, n: usize) {
        self.out_bytes = self.out_bytes.saturating_add(n as u64);
    }

    /// Which post was last spoken to. A fingerprint is public by
    /// construction — it is what two people read to each other — so
    /// this is the one identifier the pane may show.
    pub fn met(&mut self, peer: Fingerprint) {
        self.peer = Some(peer);
    }

    pub fn count(&self, signal: Signal) -> u64 {
        self.signals[signal.index()]
    }

    pub fn refusals(&self, refusal: Refusal) -> u64 {
        self.refusals[refusal.index()]
    }

    /// A snapshot for a screen. Reading it changes nothing — a counter
    /// that resets when it is looked at is a counter two panes disagree
    /// about.
    pub fn report(&self) -> Report {
        Report {
            sessions: self.count(Signal::Session),
            handshakes: self.count(Signal::Handshake),
            pairings: self.count(Signal::Pairing),
            sent: self.count(Signal::Sent),
            received: self.count(Signal::Received),
            already_held: self.count(Signal::Known),
            agreed: self.count(Signal::Agreed),
            refused: self
                .refusals
                .iter()
                .copied()
                .fold(0u64, u64::saturating_add),
            in_bytes: self.in_bytes,
            out_bytes: self.out_bytes,
            peer: self.peer,
        }
    }
}

/// What a pane draws. Numbers, and one public fingerprint.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Report {
    pub sessions: u64,
    pub handshakes: u64,
    pub pairings: u64,
    pub sent: u64,
    pub received: u64,
    pub already_held: u64,
    pub agreed: u64,
    /// Every refusal, of every kind. The kinds are read one by one with
    /// [`Meter::refusals`] — the total is here because a pane that shows
    /// nine kinds and no total makes the reader add up.
    pub refused: u64,
    pub in_bytes: u64,
    pub out_bytes: u64,
    pub peer: Option<Fingerprint>,
}

impl Error {
    /// Every refusal, so a counter cannot miss one.
    pub const ALL: [Error; 9] = [
        Error::Malformed,
        Error::TooLarge,
        Error::Signature,
        Error::Name,
        Error::Seal,
        Error::Protocol,
        Error::Handshake,
        Error::Unknown,
        Error::Link,
    ];

    fn index(self) -> usize {
        Self::ALL.iter().position(|e| *e == self).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_is_counted_is_read_back() {
        let mut m = Meter::new();
        m.note(Signal::Session);
        m.note(Signal::Received);
        m.note(Signal::Received);
        m.refuse(Error::Signature);
        m.read_bytes(400);
        m.wrote_bytes(90);
        let r = m.report();
        assert_eq!((r.sessions, r.received, r.refused), (1, 2, 1));
        assert_eq!((r.in_bytes, r.out_bytes), (400, 90));
        assert_eq!(m.refusals(Error::Signature), 1);
        assert_eq!(m.refusals(Error::Seal), 0);
        // Reading it does not empty it.
        assert_eq!(m.report(), r);
    }

    /// Every signal and every refusal has its own slot. Two sharing one
    /// would make a pane show a number that is two things added up, and
    /// nobody would ever know which.
    #[test]
    fn no_two_counters_share_a_slot() {
        for signal in Signal::ALL {
            let mut m = Meter::new();
            m.note(signal);
            assert_eq!(m.count(signal), 1, "{signal:?}");
            let others: u64 = Signal::ALL
                .iter()
                .filter(|s| **s != signal)
                .map(|s| m.count(*s))
                .sum();
            assert_eq!(others, 0, "{signal:?} a compté pour un autre");
        }
        for refusal in Error::ALL {
            let mut m = Meter::new();
            m.refuse(refusal);
            assert_eq!(m.refusals(refusal), 1, "{refusal:?}");
            assert_eq!(m.report().refused, 1);
        }
        // And the list is complete: a variant added to `Error` with no
        // line in `ALL` falls back to slot zero and is counted as a
        // `Malformed`, silently. The enumeration in `lib.rs` is what
        // this compares against, verbatim.
        let declared = include_str!("lib.rs")
            .split("pub enum Error {")
            .nth(1)
            .and_then(|t| t.split("\n}").next())
            .unwrap()
            .lines()
            .filter(|l| {
                let l = l.trim();
                !l.is_empty() && !l.starts_with("//")
            })
            .count();
        assert_eq!(declared, Error::ALL.len(), "un refus sans compteur");
    }

    /// A counter that wraps reads as « rien ne s'est passé ».
    #[test]
    fn a_counter_stops_rather_than_turning_over() {
        let mut m = Meter::new();
        m.signals[Signal::Sent.index()] = u64::MAX;
        m.note(Signal::Sent);
        assert_eq!(m.count(Signal::Sent), u64::MAX);
        m.in_bytes = u64::MAX;
        m.read_bytes(4096);
        assert_eq!(m.report().in_bytes, u64::MAX);
    }

    /// The rule the module exists under, held by its own text: nothing
    /// here reaches the world, and nothing here holds a sentence.
    #[test]
    fn telemetry_counts_and_never_quotes_and_never_leaves() {
        let text = include_str!("meter.rs");
        let code = text.split("mod tests").next().unwrap();
        let body: String = code
            .lines()
            .filter(|l| !l.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        for quoted in ["String", "&str", "format!", "Vec<"] {
            assert!(
                !body.contains(quoted),
                "la télémétrie ne cite pas : « {quoted} »"
            );
        }
        for outward in ["ureq", "std::net", "std::fs", "reqwest", "http", "send("] {
            assert!(
                !body.contains(outward),
                "la télémétrie ne sort pas : « {outward} »"
            );
        }
        // The one identifier it may hold is the public fingerprint, and
        // it is there on purpose — a pane has to be able to say which
        // post it last spoke to.
        assert!(body.contains("Fingerprint"));
    }
}
