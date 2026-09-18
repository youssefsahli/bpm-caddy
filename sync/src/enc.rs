//! The canonical encoding: how a header, a record and a frame are
//! written down as bytes.
//!
//! Written by hand rather than handed to a serialisation library, for
//! one reason: a record's **name** is a hash of these bytes, so the
//! encoding is part of the security argument. A library that reorders a
//! map, pads a field or changes its representation between two versions
//! renames every record in the officine's journal, and nothing would
//! say so — every post would simply stop recognising what the others
//! wrote. Here the encoding is thirty lines, it is fixed, and a
//! round-trip test holds it.
//!
//! Two rules it carries. **Every length is bounded by the reader, not
//! by the writer**: a peer announcing four gigabytes is a peer choosing
//! how much memory this post allocates, so every read takes the largest
//! it will accept. And **a truncated buffer is a refusal, never a
//! panic** — the bytes come off a socket, and `&bytes[..n]` on a short
//! slice takes the application down at the counter.

use crate::{Error, Result};

/// Builds the canonical bytes.
#[derive(Default)]
pub struct Writer(Vec<u8>);

impl Writer {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn u8(&mut self, v: u8) -> &mut Self {
        self.0.push(v);
        self
    }

    pub fn u32(&mut self, v: u32) -> &mut Self {
        self.0.extend_from_slice(&v.to_be_bytes());
        self
    }

    pub fn u64(&mut self, v: u64) -> &mut Self {
        self.0.extend_from_slice(&v.to_be_bytes());
        self
    }

    /// Bytes of a length both sides already agree on (a hash, a key, a
    /// nonce): written without a prefix, because a length that cannot
    /// vary is a length nobody has to be told.
    pub fn raw(&mut self, v: &[u8]) -> &mut Self {
        self.0.extend_from_slice(v);
        self
    }

    /// Bytes of a length only the writer knows, prefixed with it.
    pub fn bytes(&mut self, v: &[u8]) -> &mut Self {
        self.u32(v.len() as u32).raw(v)
    }

    pub fn finish(self) -> Vec<u8> {
        self.0
    }
}

/// Reads the canonical bytes back, refusing rather than panicking.
pub struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self.pos.checked_add(n).ok_or(Error::Malformed)?;
        let slice = self.bytes.get(self.pos..end).ok_or(Error::Malformed)?;
        self.pos = end;
        Ok(slice)
    }

    pub fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }

    pub fn u32(&mut self) -> Result<u32> {
        let b = self.take(4)?;
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    pub fn u64(&mut self) -> Result<u64> {
        let b: [u8; 8] = self.take(8)?.try_into().map_err(|_| Error::Malformed)?;
        Ok(u64::from_be_bytes(b))
    }

    /// A fixed-width field: a hash, a public key, a nonce.
    pub fn array<const N: usize>(&mut self) -> Result<[u8; N]> {
        self.take(N)?.try_into().map_err(|_| Error::Malformed)
    }

    /// A length-prefixed field, refused above `max`. The caller names
    /// the ceiling because the caller is the one who knows what the
    /// field is: a record's payload and a peer's list of hashes do not
    /// deserve the same room.
    pub fn bytes(&mut self, max: usize) -> Result<&'a [u8]> {
        let n = self.u32()? as usize;
        if n > max {
            return Err(Error::TooLarge);
        }
        self.take(n)
    }

    /// A count, refused above `max` — read *before* anything is
    /// allocated for it, which is the whole point.
    pub fn count(&mut self, max: usize) -> Result<usize> {
        let n = self.u32()? as usize;
        if n > max {
            return Err(Error::TooLarge);
        }
        Ok(n)
    }

    /// Refuses trailing bytes. A frame with something after it is a
    /// frame somebody added something to.
    pub fn end(&self) -> Result<()> {
        if self.pos == self.bytes.len() {
            Ok(())
        } else {
            Err(Error::Malformed)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_is_written_is_what_is_read() {
        let mut w = Writer::new();
        w.u8(7)
            .u32(4_000_000_000)
            .u64(u64::MAX)
            .raw(&[1, 2, 3, 4])
            .bytes(b"une ligne de registre");
        let bytes = w.finish();

        let mut r = Reader::new(&bytes);
        assert_eq!(r.u8().unwrap(), 7);
        assert_eq!(r.u32().unwrap(), 4_000_000_000);
        assert_eq!(r.u64().unwrap(), u64::MAX);
        assert_eq!(r.array::<4>().unwrap(), [1, 2, 3, 4]);
        assert_eq!(r.bytes(64).unwrap(), b"une ligne de registre");
        r.end().unwrap();
    }

    /// The bytes come off a socket. Every prefix of a valid encoding —
    /// every one, not a chosen few — must come back as a refusal, and
    /// the test is written as a sweep because it is the *one* the author
    /// did not think of that panics at the counter.
    #[test]
    fn a_truncated_buffer_is_refused_and_never_panics() {
        let mut w = Writer::new();
        w.u8(1).u64(2).raw(&[9; 32]).bytes(b"abc");
        let full = w.finish();
        for cut in 0..full.len() {
            let mut r = Reader::new(&full[..cut]);
            let read = (|| {
                r.u8()?;
                r.u64()?;
                r.array::<32>()?;
                r.bytes(16)?;
                r.end()
            })();
            assert!(read.is_err(), "coupé à {cut} et lu quand même");
        }
    }

    /// A peer announcing more than the reader will take is refused
    /// *before* the allocation, not after it.
    #[test]
    fn a_length_a_peer_chose_is_bounded_by_the_reader() {
        let mut w = Writer::new();
        w.u32(3_000_000_000).raw(b"rien");
        let bytes = w.finish();
        assert_eq!(Reader::new(&bytes).bytes(4096), Err(Error::TooLarge));
        assert_eq!(Reader::new(&bytes).count(4096), Err(Error::TooLarge));
        // And the same announcement, under the ceiling, is simply short.
        let mut w = Writer::new();
        w.bytes(b"quatre");
        assert_eq!(
            Reader::new(&w.finish()).bytes(3).unwrap_err(),
            Error::TooLarge
        );
    }

    #[test]
    fn a_frame_with_something_after_it_is_refused() {
        let mut w = Writer::new();
        w.u8(1).raw(b"de trop");
        let bytes = w.finish();
        let mut r = Reader::new(&bytes);
        r.u8().unwrap();
        assert_eq!(r.end(), Err(Error::Malformed));
    }
}
