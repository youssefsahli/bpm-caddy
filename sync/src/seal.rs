//! One record: a fact, sealed, named by its own content, signed.
//!
//! A record is the unit everything else is built from, and it carries
//! three claims that have to hold together or none of them is worth
//! anything.
//!
//! **It is named by what it contains.** The identifier is a hash of the
//! header and the ciphertext, so a byte changed anywhere renames it —
//! and since the next records name it as a parent, renaming one breaks
//! every record written after it. That is what makes a journal
//! reconciled over a network worth as much as a bound register: nobody
//! can go back.
//!
//! **Its ciphertext belongs to its header.** The header is the
//! additional data of the seal, so a ciphertext lifted out of one record
//! and dropped into another does not open. Without that, a relay could
//! keep the words of a dispensing and change which file, which post and
//! which day they were written under — everything that gives them
//! meaning — and the seal would still check.
//!
//! **Somebody wrote it.** The signature is over the name, by the post
//! that wrote it. A peer that stores ciphertext it cannot read also
//! cannot add to it.

use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};

use crate::enc::{Reader, Writer};
use crate::keys::{Device, DeviceId, Trousseau};
use crate::{Entropy, Error, Result};

/// The largest payload one record carries.
///
/// Forty-eight kilobytes, and the figure is not arbitrary: a Noise
/// transport message holds 65 535 bytes and there is **no reassembly
/// buffer in this crate**, on purpose. A protocol that cuts messages in
/// pieces and puts them back together is a protocol with a table of
/// half-arrived things in it, and that table is where a peer gets to
/// make this post hold memory it never asked for. So a record fits in a
/// message, and what does not fit in a record is not a record: a
/// scanned ordonnance is a file, it lives in `<base>_scans.db`, and it
/// gets its own arrangement the day somebody wants it synced.
pub const MAX_PAYLOAD: usize = 48_000;

/// The most parents a record names. The journal's heads, and a post
/// that has been offline through sixty-four divergent ones has a
/// different problem than this constant.
pub const MAX_PARENTS: usize = 64;

/// A record's name: the hash of everything it is.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Hash(pub [u8; 32]);

impl Hash {
    /// The first four bytes, which is what a screen shows when it has to
    /// name a record to a human. Never what anything *compares* on.
    pub fn short(&self) -> String {
        self.0[..4].iter().map(|b| format!("{b:02X}")).collect()
    }
}

/// Which body of data a record belongs to.
///
/// A record from a version that knows more streams than this one is
/// [`Stream::Autre`] and is **kept**: an old post relays what it cannot
/// read rather than dropping it, or upgrading one post in an officine
/// would quietly cost the others everything the new one wrote. Its key
/// is derived from the code and not from the name, so both posts derive
/// the same one whether or not they can say what it is for.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Stream {
    /// Files, acts, what was dispensed to whom.
    Dossiers,
    /// The register of stupéfiants. Append-only by law and by nature,
    /// which makes it the one stream that can never conflict.
    Registre,
    /// The team's shifts.
    Planning,
    /// Rendez-vous.
    Agenda,
    /// What the officine itself is: identity, labels, rewritten phrases.
    Officine,
    /// The drug cards, and the officine's own corrections to them.
    Fiches,
    /// The till.
    Caisse,
    /// What an officine shares with the **other officines of its
    /// network**: shortages, what was given in their place. Never a
    /// patient — which is the only reason a stream may leave the
    /// officine at all. Sealed under the network's own trousseau, never
    /// the officine's: a key that opens this stream opens no other.
    Reseau,
    /// A stream a later version named and this one does not know.
    Autre(u8),
}

impl Stream {
    /// Every stream this version knows. Not `Autre`, which is not one
    /// stream but every stream that does not exist yet.
    pub const ALL: [Stream; 8] = [
        Stream::Dossiers,
        Stream::Registre,
        Stream::Planning,
        Stream::Agenda,
        Stream::Officine,
        Stream::Fiches,
        Stream::Caisse,
        Stream::Reseau,
    ];

    pub fn code(self) -> u8 {
        match self {
            Stream::Dossiers => 1,
            Stream::Registre => 2,
            Stream::Planning => 3,
            Stream::Agenda => 4,
            Stream::Officine => 5,
            Stream::Fiches => 6,
            Stream::Caisse => 7,
            Stream::Reseau => 8,
            Stream::Autre(code) => code,
        }
    }

    /// Never fails. A code nobody here recognises is a stream, not an
    /// error: see the type's own note.
    pub fn from_code(code: u8) -> Self {
        match code {
            1 => Stream::Dossiers,
            2 => Stream::Registre,
            3 => Stream::Planning,
            4 => Stream::Agenda,
            5 => Stream::Officine,
            6 => Stream::Fiches,
            7 => Stream::Caisse,
            8 => Stream::Reseau,
            other => Stream::Autre(other),
        }
    }

    /// The domain string its key is derived under — from the **code**,
    /// so that two posts agree on the key for a stream only one of them
    /// has a name for.
    pub(crate) fn seal_context(self) -> String {
        format!("bpm-caddy/seal/v1/stream/{}", self.code())
    }

    /// Whether this version knows what the stream holds. A relayed
    /// stream is stored and passed on; it is not read.
    pub fn known(self) -> bool {
        !matches!(self, Stream::Autre(_))
    }
}

/// One sealed fact.
///
/// Everything but `sealed` is in the clear, and deliberately: a peer has
/// to be able to place a record in the graph — its parents, its author,
/// its rank — without being able to read it. What that costs is a
/// *traffic* picture: somebody holding the journal without the trousseau
/// learns that this post wrote eleven register records on a Tuesday. It
/// does not learn one word of them.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Record {
    id: Hash,
    stream: Stream,
    author: DeviceId,
    lamport: u64,
    parents: Vec<Hash>,
    corrects: Option<Hash>,
    nonce: [u8; 24],
    sealed: Vec<u8>,
    signature: [u8; 64],
}

impl Record {
    pub fn id(&self) -> Hash {
        self.id
    }
    pub fn stream(&self) -> Stream {
        self.stream
    }
    pub fn author(&self) -> DeviceId {
        self.author
    }
    pub fn lamport(&self) -> u64 {
        self.lamport
    }
    pub fn parents(&self) -> &[Hash] {
        &self.parents
    }
    /// The record this one corrects, if it corrects one. A correction
    /// never removes what it corrects — that stays in the journal, and
    /// on paper, struck through.
    pub fn corrects(&self) -> Option<Hash> {
        self.corrects
    }

    /// Everything but the signature, canonically. This is both what the
    /// name is computed over and what the seal is bound to, which is
    /// what stops a ciphertext being moved onto another header.
    fn header(
        stream: Stream,
        author: &DeviceId,
        lamport: u64,
        parents: &[Hash],
        corrects: Option<&Hash>,
        nonce: &[u8; 24],
    ) -> Vec<u8> {
        let mut w = Writer::new();
        w.raw(b"bpm-caddy/record/v1")
            .u8(stream.code())
            .raw(&author.0)
            .u64(lamport)
            .u32(parents.len() as u32);
        for parent in parents {
            w.raw(&parent.0);
        }
        match corrects {
            Some(h) => {
                w.u8(1).raw(&h.0);
            }
            None => {
                w.u8(0);
            }
        }
        w.raw(nonce);
        w.finish()
    }

    fn name(header: &[u8], sealed: &[u8]) -> Hash {
        let mut hasher = blake3::Hasher::new_derive_key("bpm-caddy/record-name/v1");
        hasher.update(&(header.len() as u64).to_be_bytes());
        hasher.update(header);
        hasher.update(sealed);
        Hash(*hasher.finalize().as_bytes())
    }

    /// Seals a payload into a record this post signs.
    ///
    /// The nonce is drawn at random every time. With XChaCha20's
    /// twenty-four bytes that is safe without any post telling the
    /// others what it has used — which is the only arrangement that
    /// survives three machines writing while one of them is unplugged.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn seal(
        device: &Device,
        trousseau: &Trousseau,
        stream: Stream,
        payload: &[u8],
        corrects: Option<Hash>,
        parents: Vec<Hash>,
        lamport: u64,
        entropy: &mut dyn Entropy,
    ) -> Result<Self> {
        if payload.len() > MAX_PAYLOAD {
            return Err(Error::TooLarge);
        }
        if parents.len() > MAX_PARENTS {
            return Err(Error::TooLarge);
        }
        let author = device.id();
        let mut nonce = [0u8; 24];
        entropy.fill(&mut nonce);
        let header = Self::header(
            stream,
            &author,
            lamport,
            &parents,
            corrects.as_ref(),
            &nonce,
        );

        let key = trousseau.stream_key(stream);
        let cipher = XChaCha20Poly1305::new(&Key::from(key));
        let sealed = cipher
            .encrypt(
                &XNonce::from(nonce),
                Payload {
                    msg: payload,
                    aad: &header,
                },
            )
            .map_err(|_| Error::Seal)?;

        let id = Self::name(&header, &sealed);
        Ok(Self {
            signature: device.sign(&id.0),
            id,
            stream,
            author,
            lamport,
            parents,
            corrects,
            nonce,
            sealed,
        })
    }

    /// Opens the payload. Needs the officine's trousseau and nothing
    /// else — no peer, no network, no clock.
    pub fn open(&self, trousseau: &Trousseau) -> Result<Vec<u8>> {
        let header = Self::header(
            self.stream,
            &self.author,
            self.lamport,
            &self.parents,
            self.corrects.as_ref(),
            &self.nonce,
        );
        let key = trousseau.stream_key(self.stream);
        let cipher = XChaCha20Poly1305::new(&Key::from(key));
        cipher
            .decrypt(
                &XNonce::from(self.nonce),
                Payload {
                    msg: &self.sealed,
                    aad: &header,
                },
            )
            .map_err(|_| Error::Seal)
    }

    /// The bytes that cross a link.
    pub fn encode(&self) -> Vec<u8> {
        let mut w = Writer::new();
        w.u8(self.stream.code())
            .raw(&self.author.0)
            .u64(self.lamport)
            .u32(self.parents.len() as u32);
        for parent in &self.parents {
            w.raw(&parent.0);
        }
        match &self.corrects {
            Some(h) => {
                w.u8(1).raw(&h.0);
            }
            None => {
                w.u8(0);
            }
        }
        w.raw(&self.nonce).bytes(&self.sealed).raw(&self.signature);
        w.finish()
    }

    /// Reads a record off a link — and **checks it before returning
    /// it**, so that a `Record` in hand is one whose name matches its
    /// content and whose signature its author made. There is no way to
    /// build an unchecked one from outside this module, which is the
    /// point: a caller cannot forget.
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let mut r = Reader::new(bytes);
        let stream = Stream::from_code(r.u8()?);
        let author = DeviceId(r.array::<32>()?);
        let lamport = r.u64()?;
        let count = r.count(MAX_PARENTS)?;
        let mut parents = Vec::with_capacity(count);
        for _ in 0..count {
            parents.push(Hash(r.array::<32>()?));
        }
        let corrects = match r.u8()? {
            0 => None,
            1 => Some(Hash(r.array::<32>()?)),
            _ => return Err(Error::Malformed),
        };
        let nonce = r.array::<24>()?;
        // The tag is sixteen bytes on top of the plain text.
        let sealed = r.bytes(MAX_PAYLOAD + 16)?.to_vec();
        let signature = r.array::<64>()?;
        r.end()?;

        let header = Self::header(
            stream,
            &author,
            lamport,
            &parents,
            corrects.as_ref(),
            &nonce,
        );
        let id = Self::name(&header, &sealed);
        author.verify(&id.0, &signature)?;
        Ok(Self {
            id,
            stream,
            author,
            lamport,
            parents,
            corrects,
            nonce,
            sealed,
            signature,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::Counted;

    fn one(stream: Stream, payload: &[u8]) -> (Device, Trousseau, Record) {
        let mut e = Counted(11);
        let device = Device::generate(&mut e);
        let trousseau = Trousseau::generate(&mut e);
        let record = Record::seal(
            &device,
            &trousseau,
            stream,
            payload,
            None,
            vec![],
            1,
            &mut e,
        )
        .unwrap();
        (device, trousseau, record)
    }

    #[test]
    fn what_is_sealed_comes_back_and_only_with_the_officines_key() {
        let (_, trousseau, record) = one(Stream::Registre, b"12 comprimes, dossier 4021");
        assert_eq!(
            record.open(&trousseau).unwrap(),
            b"12 comprimes, dossier 4021"
        );
        let mut e = Counted(200);
        let stranger = Trousseau::generate(&mut e);
        assert_eq!(record.open(&stranger), Err(Error::Seal));
    }

    /// The rule the whole journal rests on. A record is its content: a
    /// byte moved anywhere renames it, so there is nowhere to put a
    /// change that leaves the name alone.
    #[test]
    fn a_record_is_named_by_its_content() {
        let mut e = Counted(11);
        let device = Device::generate(&mut e);
        let trousseau = Trousseau::generate(&mut e);
        let mut names = std::collections::HashSet::new();
        // The same payload, one element of the header different each
        // time. Every one of them is a different record.
        let variants: Vec<Record> = [
            (Stream::Registre, 1u64, None, vec![]),
            (Stream::Dossiers, 1, None, vec![]),
            (Stream::Registre, 2, None, vec![]),
            (Stream::Registre, 1, Some(Hash([3; 32])), vec![]),
            (Stream::Registre, 1, None, vec![Hash([4; 32])]),
        ]
        .into_iter()
        .map(|(stream, lamport, corrects, parents)| {
            let mut fixed = Counted(77);
            Record::seal(
                &device,
                &trousseau,
                stream,
                b"un meme texte",
                corrects,
                parents,
                lamport,
                &mut fixed,
            )
            .unwrap()
        })
        .collect();
        for v in &variants {
            assert!(names.insert(v.id()), "deux en-têtes, un seul nom");
        }
        // And the payload renames it too.
        let mut fixed = Counted(77);
        let other = Record::seal(
            &device,
            &trousseau,
            Stream::Registre,
            b"un autre texte",
            None,
            vec![],
            1,
            &mut fixed,
        )
        .unwrap();
        assert!(names.insert(other.id()));
    }

    /// A ciphertext lifted from one record into another does not open.
    /// Without this, the words of a dispensing could be kept and the
    /// file, the post and the rank they were written under changed.
    #[test]
    fn a_ciphertext_moved_onto_another_header_does_not_open() {
        let mut e = Counted(11);
        let device = Device::generate(&mut e);
        let trousseau = Trousseau::generate(&mut e);
        let make = |lamport: u64| {
            let mut fixed = Counted(55);
            Record::seal(
                &device,
                &trousseau,
                Stream::Registre,
                b"12 comprimes",
                None,
                vec![],
                lamport,
                &mut fixed,
            )
            .unwrap()
        };
        let (first, second) = (make(1), make(2));
        // Same nonce, same payload, same key: only the header differs.
        let mut forged = second.clone();
        forged.sealed = first.sealed.clone();
        assert_eq!(forged.open(&trousseau), Err(Error::Seal));
    }

    /// Two records sealed at two moments never share a nonce, because
    /// the nonce is drawn and not counted. Three posts writing at once
    /// with nobody coordinating is the case this has to survive.
    #[test]
    fn two_records_never_share_a_nonce() {
        let mut e = Counted(11);
        let device = Device::generate(&mut e);
        let trousseau = Trousseau::generate(&mut e);
        let mut seen = std::collections::HashSet::new();
        let mut real = crate::OsEntropy;
        for _ in 0..64 {
            let r = Record::seal(
                &device,
                &trousseau,
                Stream::Registre,
                b"x",
                None,
                vec![],
                1,
                &mut real,
            )
            .unwrap();
            assert!(seen.insert(r.nonce), "un nonce réutilisé");
        }
    }

    /// A record in hand is a checked record: `decode` is the only door,
    /// and it refuses a name that does not match and a signature that
    /// does not check.
    #[test]
    fn a_record_that_does_not_check_out_never_becomes_a_record() {
        let (device, _, record) = one(Stream::Registre, b"une ligne");
        let bytes = record.encode();
        assert_eq!(Record::decode(&bytes).unwrap(), record);

        // The ciphertext changed: the name changes with it, and the
        // signature is then over a name nobody signed.
        let mut bent = bytes.clone();
        let last = bent.len() - 65;
        bent[last] ^= 1;
        assert_eq!(Record::decode(&bent), Err(Error::Signature));

        // The signature changed: refused.
        let mut bent = bytes.clone();
        let n = bent.len();
        bent[n - 1] ^= 1;
        assert_eq!(Record::decode(&bent), Err(Error::Signature));

        // Another post's signature over this record: refused, which is
        // what stops a relay writing in somebody else's name.
        let mut e = Counted(99);
        let stranger = Device::generate(&mut e);
        let mut forged = bytes.clone();
        let n = forged.len();
        forged[n - 64..].copy_from_slice(&stranger.sign(&record.id().0));
        assert_eq!(Record::decode(&forged), Err(Error::Signature));
        // ...and the true author's own signature still passes, so the
        // test above is measuring the signature and not the bending.
        assert!(Record::decode(&bytes).is_ok());
        let _ = device;
    }

    /// Every truncation of a valid record is refused, and none panics.
    #[test]
    fn a_truncated_record_is_refused_and_never_panics() {
        let (_, _, record) = one(Stream::Dossiers, b"une ligne de dossier");
        let bytes = record.encode();
        for cut in 0..bytes.len() {
            assert!(Record::decode(&bytes[..cut]).is_err(), "coupé à {cut}");
        }
        // And bytes after it are refused too.
        let mut longer = bytes.clone();
        longer.push(0);
        assert_eq!(Record::decode(&longer), Err(Error::Malformed));
    }

    /// A peer does not get to choose how much memory this post uses.
    #[test]
    fn a_record_larger_than_this_post_will_hold_is_refused() {
        let mut e = Counted(11);
        let device = Device::generate(&mut e);
        let trousseau = Trousseau::generate(&mut e);
        let big = vec![0u8; MAX_PAYLOAD + 1];
        assert_eq!(
            Record::seal(
                &device,
                &trousseau,
                Stream::Registre,
                &big,
                None,
                vec![],
                1,
                &mut e
            ),
            Err(Error::TooLarge)
        );
        let parents = vec![Hash([0; 32]); MAX_PARENTS + 1];
        assert_eq!(
            Record::seal(
                &device,
                &trousseau,
                Stream::Registre,
                b"x",
                None,
                parents,
                1,
                &mut e
            ),
            Err(Error::TooLarge)
        );
    }

    /// A stream a later version named is kept, relayed and sealed with
    /// the key both posts derive from its code. Dropping it would cost
    /// an officine everything its upgraded post wrote.
    #[test]
    fn a_stream_this_version_does_not_know_is_still_carried() {
        assert_eq!(Stream::from_code(200), Stream::Autre(200));
        assert!(!Stream::Autre(200).known());
        for s in Stream::ALL {
            assert!(s.known());
            assert_eq!(Stream::from_code(s.code()), s, "aller-retour du code");
        }
        let (_, trousseau, record) = one(Stream::Autre(200), b"ce qu'une version a ecrit");
        let carried = Record::decode(&record.encode()).unwrap();
        assert_eq!(carried.stream(), Stream::Autre(200));
        assert_eq!(
            carried.open(&trousseau).unwrap(),
            b"ce qu'une version a ecrit"
        );
    }

    #[test]
    fn a_record_is_shown_by_a_short_name_and_compared_on_the_whole_one() {
        let h = Hash([
            0xAB, 0xCD, 0xEF, 0x01, 0xFF, 0xFF, 0xFF, 0xFF, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ]);
        assert_eq!(h.short(), "ABCDEF01");
        let mut other = h;
        other.0[31] = 1;
        assert_eq!(h.short(), other.short());
        assert_ne!(h, other, "le nom court n'est pas ce qu'on compare");
    }
}
