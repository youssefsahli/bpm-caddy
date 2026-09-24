//! Who a post is, and what the officine shares.
//!
//! Two secrets, and they answer two different questions. A [`Device`] is
//! **this machine**: it signs what it writes, so a record can be traced
//! to the post that wrote it and a relay cannot invent one. A
//! [`Trousseau`] is **the officine**: it opens the seals, so a post that
//! has it can read the journal and a post that has not cannot.
//!
//! What keeping them apart buys, exactly — and it is worth stating
//! exactly, because the comfortable version of this sentence is false.
//! Dropping a post out of a machine's list of known devices stops it
//! **connecting to that machine**, and every record stays attributable
//! for ever — it is signed, and [`crate::Fact`] carries the author, so
//! a screen can say which post wrote a line years later.
//!
//! Two things it does **not** do, and both matter. It is not officine-
//! wide on its own: a post the others have disowned can still hand a
//! new record to a post that has *not* disowned it, and from there it
//! reaches everybody, because a journal accepts what it is given —
//! refusing an author would make two posts' journals diverge for ever
//! over a list neither can see. Revoking therefore means revoking
//! everywhere. And it does not take back what the machine already
//! holds: a laptop that walked out with the trousseau reads the journal
//! on its own disk, and the only answer to that is to re-key the
//! officine — exactly as the only answer to a stolen base is to change
//! its password. **Revocation is about the future; re-keying is about
//! the past.** And re-keying keeps the history rather than costing it:
//! the new trousseau seals what comes next, the old ones stay in the
//! base, and [`crate::Journal::read_with`] tries the ring. Nothing is
//! re-sealed, because re-sealing would rename every record.
//!
//! Both are thirty-two bytes, both are wiped when they go out of scope,
//! and neither is ever written to a link in clear — the trousseau
//! crosses once, inside the handshake's encryption, under a code two
//! people read aloud to each other.

use ed25519_dalek::{Signer, SigningKey};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::seal::Stream;
use crate::{Entropy, Error, Result};

/// Five groups of four hexadecimal characters, which is what a human
/// reads out loud.
///
/// Sixteen bytes would be unreadable and eight are not enough to compare
/// by eye without losing the place, so it is **ten** — eighty bits, five
/// groups. It is one function because it renders two different things —
/// a post's fingerprint and a pairing's short code — and two spellings
/// of one idea drift: the day they disagree, two people on the telephone
/// decide the codes differ when they do not.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Fingerprint([u8; 10]);

impl Fingerprint {
    /// The fingerprint of a domain-separated hash of `material`.
    fn of(context: &str, material: &[u8]) -> Self {
        let full = blake3::derive_key(context, material);
        let mut short = [0u8; 10];
        short.copy_from_slice(&full[..10]);
        Self(short)
    }

    /// The ten bytes, for a caller that wants to store it.
    pub fn bytes(&self) -> [u8; 10] {
        self.0
    }

    /// A fingerprint read back — off a link, or out of the base. It is
    /// public data: nothing is proved by holding one.
    pub fn from_bytes(bytes: [u8; 10]) -> Self {
        Self(bytes)
    }

    /// « A1B2 C3D4 E5F6 7890 1234 » — what goes on a screen and down a
    /// telephone.
    pub fn groups(&self) -> String {
        let mut out = String::with_capacity(24);
        for (i, byte) in self.0.iter().enumerate() {
            if i % 2 == 0 && i != 0 {
                out.push(' ');
            }
            out.push_str(&format!("{byte:02X}"));
        }
        out
    }
}

/// A post's public name: the key its signatures are checked against.
///
/// Public, so it is `Copy` and travels in the clear — that is what it is
/// for. What it is *not* is a secret: seeing it tells you a post exists,
/// nothing more.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct DeviceId(pub [u8; 32]);

impl DeviceId {
    /// What two people compare when they pair two posts.
    pub fn fingerprint(&self) -> Fingerprint {
        Fingerprint::of("bpm-caddy/device-fingerprint/v1", &self.0)
    }

    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Checks a signature made by this post over `message`.
    ///
    /// `verify_strict` rather than `verify`: the permissive one accepts
    /// signatures under small-order keys, which is a door nobody here
    /// has a use for.
    pub fn verify(&self, message: &[u8], signature: &[u8; 64]) -> Result<()> {
        let key = ed25519_dalek::VerifyingKey::from_bytes(&self.0).map_err(|_| Error::Signature)?;
        let sig = ed25519_dalek::Signature::from_bytes(signature);
        key.verify_strict(message, &sig)
            .map_err(|_| Error::Signature)
    }
}

/// This machine, and the secret that proves it.
///
/// One seed, two keys derived from it by different domain strings: the
/// signing key that names what this post writes, and the static key the
/// handshake uses. One seed because it is one secret to keep, to back up
/// and to destroy; two derivations because a key used for two purposes
/// is a key whose two uses can be played against each other.
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct Device {
    seed: [u8; 32],
}

impl Device {
    /// A new post. The seed is the whole identity: whoever holds it is
    /// this post, so it belongs in the same place as the base's own key.
    pub fn generate(entropy: &mut dyn Entropy) -> Self {
        let mut seed = [0u8; 32];
        entropy.fill(&mut seed);
        Self { seed }
    }

    /// A post read back from wherever it was kept.
    pub fn from_seed(seed: [u8; 32]) -> Self {
        Self { seed }
    }

    /// The seed, to be written to the encrypted base and nowhere else.
    pub fn seed(&self) -> [u8; 32] {
        self.seed
    }

    fn signing(&self) -> SigningKey {
        SigningKey::from_bytes(&blake3::derive_key(
            "bpm-caddy/device-signing/v1",
            &self.seed,
        ))
    }

    /// The static key the Noise handshake speaks with. Derived, so that
    /// a captured handshake tells nothing about the signing key, and a
    /// compromised handshake key does not let anyone sign a record.
    pub(crate) fn static_secret(&self) -> [u8; 32] {
        blake3::derive_key("bpm-caddy/device-handshake/v1", &self.seed)
    }

    /// The secret of this post's *box key* — what opens a
    /// [`crate::boxed`] sealed for it. A third derivation of the same
    /// seed, for a third purpose.
    pub fn box_secret(&self) -> [u8; 32] {
        blake3::derive_key("bpm-caddy/device-box/v1", &self.seed)
    }

    /// The public half of the box key, which this post announces so that
    /// others can seal for it.
    pub fn box_public(&self) -> [u8; 32] {
        crate::boxed::public_of(&self.box_secret())
    }

    /// This post's public name.
    pub fn id(&self) -> DeviceId {
        DeviceId(self.signing().verifying_key().to_bytes())
    }

    /// Signs `message`. The only thing ever signed here is a
    /// thirty-two-byte name — a record's, or a handshake's — so nothing
    /// long, nothing attacker-chosen in shape.
    pub fn sign(&self, message: &[u8]) -> [u8; 64] {
        self.signing().sign(message).to_bytes()
    }
}

/// What the officine shares: the key that opens its records.
///
/// It is long-lived on purpose, and that is a trade written down rather
/// than hidden. A post that joins in March has to be able to read what
/// was written in January — a pharmacist opening a file wants the file,
/// not the part of it that postdates their laptop. So there is no
/// forward secrecy at rest: whoever ends up with the trousseau *and* the
/// sealed records can read them all. The defence against that is the one
/// the application already has — the records live inside SQLCipher, on a
/// disk somebody owns, behind the officine's password — and the defence
/// against it travelling is that it only ever crosses a link inside the
/// handshake's own encryption, once, at pairing.
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct Trousseau {
    secret: [u8; 32],
}

impl Trousseau {
    /// A new officine.
    pub fn generate(entropy: &mut dyn Entropy) -> Self {
        let mut secret = [0u8; 32];
        entropy.fill(&mut secret);
        Self { secret }
    }

    pub fn from_secret(secret: [u8; 32]) -> Self {
        Self { secret }
    }

    /// The secret, to be written to the encrypted base and nowhere else.
    pub fn secret(&self) -> [u8; 32] {
        self.secret
    }

    /// A name two posts can compare to know they belong to the same
    /// officine, that tells nobody anything about the key.
    ///
    /// It is a *derivation* and not the first bytes of the secret, which
    /// is the difference between a name and a leak.
    pub fn name(&self) -> Fingerprint {
        Fingerprint::of("bpm-caddy/trousseau-name/v1", &self.secret)
    }

    /// The key a given stream's records are sealed with.
    ///
    /// Per stream rather than one key for everything, so that handing a
    /// colleague the register alone stays possible without re-encrypting
    /// anything: the shape is already there the day it is wanted.
    pub(crate) fn stream_key(&self, stream: Stream) -> [u8; 32] {
        blake3::derive_key(&stream.seal_context(), &self.secret)
    }

    /// Constant time, because a trousseau is compared where someone can
    /// watch how long it takes.
    pub fn same(&self, other: &Trousseau) -> bool {
        use subtle::ConstantTimeEq;
        self.secret.ct_eq(&other.secret).into()
    }
}

/// The short code two people read to each other when they pair two
/// posts, derived from the handshake both sides computed.
///
/// This is the whole of the authentication. Noise XX gives two ends an
/// encrypted channel and proves each holds *a* static key; it cannot say
/// the key belongs to the officine's other post rather than to whoever
/// sits between them. Nothing but a human comparing this code can. If
/// the codes differ, there is someone in the middle — the pairing is
/// refused, and that is not a formality.
pub fn short_code(handshake_hash: &[u8]) -> Fingerprint {
    Fingerprint::of("bpm-caddy/pairing-code/v1", handshake_hash)
}

/// A one-time pairing ticket: the secret an invitation code carries.
///
/// The five-group comparison is what authenticates a pairing when the
/// two ends met on a channel nobody vouches for. An invitation code is
/// the other way to the same guarantee: the inviting officine draws ten
/// random bytes, hands them over by a channel it trusts — read down the
/// telephone, given in person — and each end then proves it holds them
/// **over this handshake's own hash**. Somebody in the middle holds two
/// handshakes with two hashes, and a proof for neither without the
/// ticket; a recording of one pairing proves nothing in the next.
///
/// Eighty bits, drawn fresh for every invitation and good for one
/// conversation: a wrong proof ends the invitation, so the ticket is
/// guessed once or not at all.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Ticket([u8; 10]);

/// The alphabet a ticket is written in: Crockford's base 32 — no I, L,
/// O or U, so nothing read down a telephone is two letters at once.
const TICKET_ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Which end a proof comes from. Two roles, so a proof sent back to its
/// author is not a proof from the other end.
pub(crate) const ROLE_JOIN: u8 = 1;
pub(crate) const ROLE_INVITE: u8 = 2;

impl Ticket {
    pub fn generate(e: &mut dyn Entropy) -> Self {
        let mut b = [0u8; 10];
        e.fill(&mut b);
        Self(b)
    }

    pub fn from_bytes(bytes: [u8; 10]) -> Self {
        Self(bytes)
    }

    pub fn bytes(&self) -> [u8; 10] {
        self.0
    }

    /// « K7QM-2XPA-9D3F-7H1Q » : sixteen characters of five bits, in four
    /// groups.
    pub fn text(&self) -> String {
        let mut bits: u128 = 0;
        for b in self.0 {
            bits = (bits << 8) | u128::from(b);
        }
        let mut out = String::with_capacity(19);
        for i in 0..16 {
            if i % 4 == 0 && i != 0 {
                out.push('-');
            }
            let v = ((bits >> (75 - 5 * i)) & 0x1f) as usize;
            out.push(TICKET_ALPHABET[v] as char);
        }
        out
    }

    /// The text read back, forgiving what a person does to it: case,
    /// dashes and spaces, and the letters Crockford reads as digits
    /// (O for 0, I and L for 1). Anything else is `None`.
    pub fn parse(text: &str) -> Option<Self> {
        let mut bits: u128 = 0;
        let mut n = 0;
        for c in text.chars() {
            let c = match c.to_ascii_uppercase() {
                '-' | ' ' => continue,
                'O' => '0',
                'I' | 'L' => '1',
                c => c,
            };
            let v = TICKET_ALPHABET.iter().position(|a| *a as char == c)?;
            bits = (bits << 5) | v as u128;
            n += 1;
            if n > 16 {
                return None;
            }
        }
        if n != 16 {
            return None;
        }
        let mut b = [0u8; 10];
        for (i, byte) in b.iter_mut().enumerate() {
            *byte = (bits >> (72 - 8 * i)) as u8;
        }
        Some(Self(b))
    }

    /// What an end sends to show it holds the ticket, for this handshake
    /// and this role.
    pub(crate) fn proof(&self, role: u8, handshake_hash: &[u8; 32]) -> blake3::Hash {
        let key = blake3::derive_key("bpm-caddy/pairing-ticket/v1", &self.0);
        let mut h = blake3::Hasher::new_keyed(&key);
        h.update(&[role]);
        h.update(handshake_hash);
        h.finalize()
    }
}

/// Never the secret in a log.
impl std::fmt::Debug for Ticket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Ticket(…)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::Counted;

    #[test]
    fn a_post_signs_what_it_writes_and_nobody_else_can() {
        let mut e = Counted(1);
        let (a, b) = (Device::generate(&mut e), Device::generate(&mut e));
        let name = [9u8; 32];
        let sig = a.sign(&name);
        a.id().verify(&name, &sig).unwrap();
        // Another post's key does not check it...
        assert_eq!(b.id().verify(&name, &sig), Err(Error::Signature));
        // ...and neither does another message.
        assert_eq!(a.id().verify(&[8u8; 32], &sig), Err(Error::Signature));
        // ...and neither does a signature somebody bent.
        let mut bent = sig;
        bent[0] ^= 1;
        assert_eq!(a.id().verify(&name, &bent), Err(Error::Signature));
    }

    /// One seed, two keys, and the handshake's is not the signing one.
    /// If they were the same bytes, a handshake recorded off the wire
    /// would be material against the key that signs the register.
    #[test]
    fn one_seed_gives_two_keys_that_are_not_each_other() {
        let mut e = Counted(1);
        let d = Device::generate(&mut e);
        assert_ne!(d.static_secret(), d.seed());
        assert_ne!(d.static_secret(), d.id().0);
        // And a post read back from its seed is the same post.
        assert_eq!(Device::from_seed(d.seed()).id(), d.id());
    }

    /// A name is a derivation, not a prefix. A `name()` that handed out
    /// ten bytes of the secret would be a trousseau published every time
    /// two posts said hello.
    #[test]
    fn a_trousseau_is_named_without_being_shown() {
        let mut e = Counted(40);
        let t = Trousseau::generate(&mut e);
        assert_ne!(&t.name().bytes()[..], &t.secret()[..10]);
        // Two officines have two names; one officine has one.
        let other = Trousseau::generate(&mut e);
        assert_ne!(t.name(), other.name());
        assert_eq!(Trousseau::from_secret(t.secret()).name(), t.name());
        assert!(t.same(&Trousseau::from_secret(t.secret())));
        assert!(!t.same(&other));
    }

    /// Each stream gets its own key, so « the register only » is a
    /// possible answer one day rather than a rewrite.
    #[test]
    fn two_streams_are_never_sealed_with_one_key() {
        let mut e = Counted(7);
        let t = Trousseau::generate(&mut e);
        let mut seen = std::collections::HashSet::new();
        for s in Stream::ALL {
            assert!(seen.insert(t.stream_key(s)), "{s:?} partage une clé");
            assert_ne!(t.stream_key(s), t.secret(), "la clé de flux est dérivée");
        }
    }

    /// The rendering is one function, and it is legible: five groups of
    /// four, separated, uppercase — a code read down a telephone.
    #[test]
    fn a_code_is_read_in_groups_of_four() {
        let f = Fingerprint([0x0A, 0x1B, 0x2C, 0x3D, 0x4E, 0x5F, 0x60, 0x71, 0x82, 0x93]);
        assert_eq!(f.groups(), "0A1B 2C3D 4E5F 6071 8293");
        assert_eq!(f.groups().len(), 24);
        // A device and a pairing are rendered by the same function.
        let mut e = Counted(3);
        let d = Device::generate(&mut e);
        assert_eq!(d.id().fingerprint().groups().len(), 24);
        assert_eq!(short_code(&[1, 2, 3]).groups().len(), 24);
    }

    /// Two handshakes are two codes. A code that did not move with the
    /// handshake would be a code that proves nothing about *this*
    /// conversation.
    #[test]
    fn a_pairing_code_follows_the_handshake_it_comes_from() {
        assert_ne!(short_code(b"une poignee"), short_code(b"une autre"));
    }

    /// A ticket is written in four groups and read back as itself —
    /// through the mistakes a telephone makes.
    #[test]
    fn a_ticket_is_written_and_read_back_as_itself() {
        let mut e = Counted(11);
        for _ in 0..64 {
            let t = Ticket::generate(&mut e);
            let text = t.text();
            assert_eq!(text.len(), 19, "{text}");
            assert_eq!(Ticket::parse(&text), Some(t));
            assert_eq!(Ticket::parse(&text.to_lowercase()), Some(t));
            assert_eq!(Ticket::parse(&text.replace('-', " ")), Some(t));
            assert_eq!(
                Ticket::parse(&text.replace('0', "O").replace('1', "l")),
                Some(t)
            );
        }
        let t = Ticket::from_bytes([0xff; 10]);
        assert_eq!(Ticket::parse(&t.text()), Some(t));
        assert_eq!(Ticket::parse("K7QM-2XPA-9D3F"), None, "trop court");
        assert_eq!(Ticket::parse("K7QM-2XPA-9D3F-7H1Q-A"), None, "trop long");
        assert_eq!(Ticket::parse("K7QM-2XPA-9D3F-7H1U"), None, "U n'en est pas");
        assert!(!format!("{t:?}").contains("FF"), "le secret ne s'écrit pas");
    }

    /// A proof names its ticket, its handshake and its end.
    #[test]
    fn a_ticket_proof_is_bound_to_ticket_handshake_and_role() {
        let a = Ticket::from_bytes([1; 10]);
        let b = Ticket::from_bytes([2; 10]);
        let (h1, h2) = ([7u8; 32], [8u8; 32]);
        assert_ne!(a.proof(ROLE_JOIN, &h1), b.proof(ROLE_JOIN, &h1));
        assert_ne!(a.proof(ROLE_JOIN, &h1), a.proof(ROLE_JOIN, &h2));
        assert_ne!(a.proof(ROLE_JOIN, &h1), a.proof(ROLE_INVITE, &h1));
    }
}
