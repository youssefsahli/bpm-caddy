//! A versioned, end-to-end encrypted journal that two posts can
//! reconcile between themselves, with no server in the middle.
//!
//! # Why this is a crate of its own, and optional
//!
//! BPM-Caddy is offline by design. The base is encrypted, it lives on a
//! disk somebody owns, and the only request the application makes is the
//! update check, on a button press. That posture is the product, not a
//! limitation of it — so the module that adds a network path is built so
//! that **a binary that does not want it does not contain it**: a
//! separate crate, behind a cargo feature that is off, depending on
//! nothing from the application. Nothing here reads a database, draws a
//! pixel, or knows what a patient is.
//!
//! # What it guarantees, and what it does not
//!
//! *A peer never sees clear text.* Records are sealed before they are
//! named, and what crosses a link is the ciphertext. Whoever relays it —
//! an officine's own second post, a server one day, the network in
//! between — carries bytes it cannot read. The key lives in the
//! [`Trousseau`] and the trousseau is handed across once, by hand, under
//! a code two humans read to each other.
//!
//! *A record is never rewritten.* It is named by a hash of its own
//! content, so changing a byte renames it, and a correction is a further
//! record that says which one it corrects. This is the register of
//! stupéfiants' rule (R. 5132-36) applied to everything: a journal you
//! can edit proves nothing, and one you can edit *remotely* proves less
//! than nothing.
//!
//! *Nobody's clock decides.* Order comes from a Lamport counter and the
//! author's name, never from a wall clock. A post whose date is wrong
//! writes a record in the wrong place on a *calendar*; it cannot reorder
//! the journal, and it cannot make an older line disappear under a newer
//! one.
//!
//! *A conflict is shown, never settled.* Two posts correcting one record
//! without having seen each other is a divergence. Both corrections stay,
//! each naming the other, and a human decides. Last-writer-wins is
//! forbidden here for the reason the whole application refuses it:
//! silently choosing between two clinical statements is choosing wrong
//! half the time, with the confidence of an answer.
//!
//! What it does **not** give: forward secrecy for records at rest (a post
//! that joins the officine must be able to read what came before, so the
//! trousseau is long-lived), nor any protection against a post whose disk
//! is in someone else's hands and whose password they know. It is a
//! transport and a history, not a second lock on the door.
//!
//! # The shape
//!
//! Everything but [`link`] is pure. [`Session`] is the protocol as a
//! state machine over frames — two of them talk to each other in a test
//! with no socket at all — and a [`Link`] is the one place bytes move.
//! Randomness is passed in ([`Entropy`]) for the same reason the clock is
//! passed in everywhere else in this application: a function that reaches
//! for the world cannot be pinned down in a test.
//!
//! ```no_run
//! use bpm_sync::{Device, Journal, OsEntropy, Stream, Trousseau};
//!
//! let mut e = OsEntropy;
//! let device = Device::generate(&mut e);
//! let trousseau = Trousseau::generate(&mut e);
//! let mut journal = Journal::new();
//!
//! // Write a fact. What goes in is bytes: this crate does not know
//! // what a register line is, and must not.
//! journal
//!     .write(&device, &trousseau, Stream::Registre, b"...", None, &mut e)
//!     .unwrap();
//!
//! // Read the stream back. Each fact says whether a rival correction
//! // exists, so the caller cannot fail to notice a divergence.
//! let reading = journal.read(&trousseau, Stream::Registre);
//! for fact in &reading.facts {
//!     assert!(fact.rivals.is_empty());
//! }
//! ```

#![forbid(unsafe_code)]

pub mod boxed;
mod enc;
mod journal;
mod keys;
mod meter;
mod seal;
mod session;
mod wire;

pub mod link;

pub use journal::{Fact, Journal, Reading};
pub use keys::{Device, DeviceId, Fingerprint, Ticket, Trousseau};
pub use link::Link;
pub use meter::{Meter, Refusal, Report, Signal};
pub use seal::{Hash, Record, Stream, MAX_PAYLOAD};
pub use session::{drive, drive_any, Intent, Session, Step};
pub use wire::Frame;

/// Everything this crate can refuse to do, as a reason and never as a
/// sentence.
///
/// A message carries what went wrong *and* a fragment of what it went
/// wrong on — a path, a name, a number of bytes — and this crate handles
/// clinical data: the one place a leak is never looked for is an error
/// string that got logged. So a failure is one of these, the caller
/// writes the French, and [`Meter`] counts them without quoting
/// anything.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Error {
    /// The bytes are not a frame, a record, or whatever was expected.
    Malformed,
    /// A frame, a record or a list longer than this crate will allocate
    /// for. A peer does not get to choose how much memory we use.
    TooLarge,
    /// The signature over a record's name does not check out.
    Signature,
    /// A record whose content does not hash to the name it carries.
    Name,
    /// The seal would not open: wrong trousseau, or a ciphertext moved
    /// onto another header.
    Seal,
    /// The other end did something out of turn.
    Protocol,
    /// The handshake failed, or the peer would not prove who it is.
    Handshake,
    /// The peer is not one this post has paired with.
    Unknown,
    /// The link broke.
    Link,
}

/// The result of anything that can be refused.
pub type Result<T> = core::result::Result<T, Error>;

/// Where random bytes come from.
///
/// Passed in, like the day is passed into every dated module here. The
/// one thing that does *not* go through it is the handshake's ephemeral
/// key, which [`snow`] draws from the operating system itself: a
/// handshake whose randomness a test could supply is a handshake nobody
/// should trust, and pretending otherwise for the sake of symmetry would
/// be the wrong trade.
pub trait Entropy {
    /// Fill `out` with unpredictable bytes.
    fn fill(&mut self, out: &mut [u8]);
}

/// The operating system's randomness. The edge, and the only
/// implementation the application ever uses.
pub struct OsEntropy;

impl Entropy for OsEntropy {
    fn fill(&mut self, out: &mut [u8]) {
        getrandom::fill(out).expect("le système n'a pas fourni d'aléa");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A counter standing in for randomness, so a test can say exactly
    /// which bytes a record was sealed with. Never used outside tests —
    /// and it is deliberately not `pub`, so it cannot be.
    pub(crate) struct Counted(pub u8);

    impl Entropy for Counted {
        fn fill(&mut self, out: &mut [u8]) {
            for b in out.iter_mut() {
                *b = self.0;
                self.0 = self.0.wrapping_add(1);
            }
        }
    }

    #[test]
    fn the_systems_randomness_answers_and_does_not_repeat_itself() {
        let mut e = OsEntropy;
        let (mut a, mut b) = ([0u8; 32], [0u8; 32]);
        e.fill(&mut a);
        e.fill(&mut b);
        assert_ne!(a, b);
        assert_ne!(a, [0u8; 32], "un tampon laissé à zéro n'est pas de l'aléa");
    }

    /// The reasons are the whole vocabulary of failure here, and none of
    /// them carries a payload — that is the point. A variant with a
    /// `String` in it is the leak this enum exists to refuse.
    #[test]
    fn a_failure_is_a_reason_and_never_a_sentence() {
        let text = include_str!("lib.rs");
        let body = text
            .split("pub enum Error {")
            .nth(1)
            .and_then(|t| t.split("\n}").next())
            .expect("l'énumération des refus");
        for forbidden in ["String", "&str", "(", "{"] {
            assert!(
                !body.contains(forbidden),
                "un refus ne porte pas de donnée : « {forbidden} »"
            );
        }
        // And it is `Copy`, which a variant carrying anything owned
        // could not be — a second guard on the same rule, from the
        // type system rather than from the text.
        fn assert_copy<T: Copy>() {}
        assert_copy::<Error>();
    }
}
