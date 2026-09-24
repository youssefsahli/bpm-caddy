//! What two posts actually say to each other.
//!
//! Seven frames, and that is the whole vocabulary. Everything but the
//! first two is about one question — *which records do you not have* —
//! and the design that keeps it to one question is [`Frame::Want`]
//! carrying both halves: what I need, and what I already hold. The peer
//! can then work out the whole set to send in one pass, instead of the
//! two of us walking the graph one generation per round trip.
//!
//! Every frame crosses **inside** the handshake's encryption. Nothing
//! here is ever written to a socket as it stands, which is why
//! [`Frame::Welcome`] — the officine's key — can be a frame at all.

use crate::enc::{Reader, Writer};
use crate::keys::{DeviceId, Fingerprint};
use crate::seal::{Hash, MAX_PAYLOAD};
use crate::{Error, Result};

/// The version of this conversation. Two posts that do not agree on it
/// stop rather than guess: a frame read under the wrong rules is a frame
/// read wrong, and the data at stake is not the kind to improvise with.
pub const PROTOCOL: u8 = 1;

/// The most hashes one frame names. A peer does not choose how much
/// memory this post allocates — the constant is here and not there.
///
/// Five hundred and twelve, because [`Frame::Want`] carries **two**
/// lists and the whole frame has to stay inside one Noise message. A
/// post with more than that to ask for asks again next round: the cap
/// costs a round trip and never a record.
pub const MAX_HASHES: usize = 512;

/// The largest frame, once decrypted: a record plus its wrapping, and
/// comfortably under the 65 535 bytes a Noise transport message holds.
pub const MAX_FRAME: usize = MAX_PAYLOAD + 8_000;

/// The largest thing a [`crate::Link`] carries: a frame, sealed, plus
/// the handshake's own overhead. A link knows this and not `MAX_FRAME`,
/// because a link never sees a frame — only the bytes around one.
pub const MAX_WIRE: usize = MAX_FRAME + 1_024;

/// One thing a post says.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Frame {
    /// « I am this post, of this officine, and this handshake is
    /// mine. » The signature is over the handshake's own hash, which is
    /// what ties an identity to *this* conversation rather than to a
    /// recording of an old one.
    Hello {
        protocol: u8,
        device: DeviceId,
        /// The officine this post belongs to, if it belongs to one yet.
        /// A post that is joining has none, and that is the whole
        /// difference between a pairing and an ordinary sync.
        officine: Option<Fingerprint>,
        signature: [u8; 64],
    },
    /// The officine's key, handed to a post that is joining it. Sent
    /// once, after two humans compared the pairing code, and never
    /// otherwise.
    Welcome([u8; 32]),
    /// What I hold: the records nothing of mine names as a parent.
    Heads(Vec<Hash>),
    /// What I need, and what I have. Both, in one frame, so the peer
    /// can compute the whole answer instead of us taking turns.
    Want { need: Vec<Hash>, have: Vec<Hash> },
    /// One record, encoded.
    Give(Vec<u8>),
    /// « I have answered what you asked. » The round's full stop, and
    /// whether there was **more** than one round holds.
    ///
    /// That flag is the difference between « you have everything I
    /// have » and « I stopped at the cap ». Without it the asker cannot
    /// tell a record the peer does not hold from one it simply has not
    /// got round to sending, and it must then either give up early —
    /// losing records, silently — or ask for ever. It is told instead.
    EndRound { more: bool },
    /// « I hold the invitation's ticket », proved over this handshake —
    /// or `None`, « I have none, show the operators the code ». A
    /// joining post always sends one, first thing once it knows who it
    /// is talking to; an inviting post answers with its own only once
    /// the joiner's has checked. See [`crate::Ticket`].
    Proof(Option<[u8; 32]>),
}

impl Frame {
    fn tag(&self) -> u8 {
        match self {
            Frame::Hello { .. } => 1,
            Frame::Welcome(_) => 2,
            Frame::Heads(_) => 3,
            Frame::Want { .. } => 4,
            Frame::Give(_) => 5,
            Frame::EndRound { .. } => 6,
            Frame::Proof(_) => 7,
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut w = Writer::new();
        w.u8(self.tag());
        match self {
            Frame::Hello {
                protocol,
                device,
                officine,
                signature,
            } => {
                w.u8(*protocol).raw(&device.0);
                match officine {
                    Some(f) => {
                        w.u8(1).raw(&f.bytes());
                    }
                    None => {
                        w.u8(0).raw(&[0u8; 10]);
                    }
                }
                w.raw(signature);
            }
            Frame::Welcome(secret) => {
                w.raw(secret);
            }
            Frame::Heads(hashes) => put_hashes(&mut w, hashes),
            Frame::Want { need, have } => {
                put_hashes(&mut w, need);
                put_hashes(&mut w, have);
            }
            Frame::Give(record) => {
                w.bytes(record);
            }
            Frame::EndRound { more } => {
                w.u8(u8::from(*more));
            }
            Frame::Proof(proof) => match proof {
                Some(p) => {
                    w.u8(1).raw(p);
                }
                None => {
                    w.u8(0).raw(&[0u8; 32]);
                }
            },
        }
        w.finish()
    }

    pub fn decode(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > MAX_FRAME {
            return Err(Error::TooLarge);
        }
        let mut r = Reader::new(bytes);
        let frame = match r.u8()? {
            1 => {
                let protocol = r.u8()?;
                let device = DeviceId(r.array::<32>()?);
                let flag = r.u8()?;
                let name = r.array::<10>()?;
                let officine = match flag {
                    0 => None,
                    1 => Some(Fingerprint::from_bytes(name)),
                    _ => return Err(Error::Malformed),
                };
                Frame::Hello {
                    protocol,
                    device,
                    officine,
                    signature: r.array::<64>()?,
                }
            }
            2 => Frame::Welcome(r.array::<32>()?),
            3 => Frame::Heads(take_hashes(&mut r)?),
            4 => Frame::Want {
                need: take_hashes(&mut r)?,
                have: take_hashes(&mut r)?,
            },
            5 => Frame::Give(r.bytes(MAX_FRAME)?.to_vec()),
            6 => Frame::EndRound {
                more: match r.u8()? {
                    0 => false,
                    1 => true,
                    _ => return Err(Error::Malformed),
                },
            },
            7 => {
                let flag = r.u8()?;
                let proof = r.array::<32>()?;
                Frame::Proof(match flag {
                    0 => None,
                    1 => Some(proof),
                    _ => return Err(Error::Malformed),
                })
            }
            // Not `Malformed`: a tag this version does not know is a
            // peer speaking a protocol this one did not agree to, and
            // that is what `Hello` exists to settle before anything is
            // said. Saying so precisely is what lets the pane tell an
            // officine « mettez les deux postes à jour » rather than
            // « données illisibles ».
            _ => return Err(Error::Protocol),
        };
        r.end()?;
        Ok(frame)
    }
}

fn put_hashes(w: &mut Writer, hashes: &[Hash]) {
    w.u32(hashes.len() as u32);
    for h in hashes {
        w.raw(&h.0);
    }
}

fn take_hashes(r: &mut Reader) -> Result<Vec<Hash>> {
    let n = r.count(MAX_HASHES)?;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        out.push(Hash(r.array::<32>()?));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn samples() -> Vec<Frame> {
        vec![
            Frame::Hello {
                protocol: PROTOCOL,
                device: DeviceId([7; 32]),
                officine: Some(Fingerprint::from_bytes([3; 10])),
                signature: [9; 64],
            },
            Frame::Hello {
                protocol: PROTOCOL,
                device: DeviceId([7; 32]),
                officine: None,
                signature: [9; 64],
            },
            Frame::Welcome([4; 32]),
            Frame::Heads(vec![]),
            Frame::Heads(vec![Hash([1; 32]), Hash([2; 32])]),
            Frame::Want {
                need: vec![Hash([5; 32])],
                have: vec![],
            },
            Frame::Give(vec![0, 1, 2, 3]),
            Frame::EndRound { more: false },
            Frame::EndRound { more: true },
            Frame::Proof(None),
            Frame::Proof(Some([6; 32])),
        ]
    }

    #[test]
    fn every_frame_goes_out_and_comes_back_as_itself() {
        for frame in samples() {
            assert_eq!(Frame::decode(&frame.encode()).unwrap(), frame, "{frame:?}");
        }
        // And no two frames encode to the same bytes — a tag that
        // collided would be a frame read as another.
        let mut seen = std::collections::HashSet::new();
        for frame in samples() {
            assert!(seen.insert(frame.encode()), "{frame:?}");
        }
    }

    /// The bytes come off a link. Every truncation, and every trailing
    /// byte, is refused — and none of them panics.
    #[test]
    fn a_frame_that_is_not_one_is_refused_and_never_panics() {
        for frame in samples() {
            let bytes = frame.encode();
            for cut in 0..bytes.len() {
                assert!(Frame::decode(&bytes[..cut]).is_err(), "{frame:?} à {cut}");
            }
            let mut longer = bytes.clone();
            longer.push(0);
            assert_eq!(Frame::decode(&longer), Err(Error::Malformed), "{frame:?}");
        }
        assert_eq!(Frame::decode(&[]), Err(Error::Malformed));
    }

    /// A tag from a version this one does not know is named as such,
    /// so that the pane can say « mettez les deux postes à jour ».
    #[test]
    fn a_frame_this_version_does_not_know_is_a_protocol_refusal() {
        for tag in [0u8, 8, 200, 255] {
            assert_eq!(Frame::decode(&[tag]), Err(Error::Protocol), "tag {tag}");
        }
    }

    /// A peer announcing four thousand and one hashes does not get four
    /// thousand and one hashes' worth of this post's memory.
    #[test]
    fn a_peer_does_not_choose_how_much_memory_this_post_uses() {
        let mut w = Writer::new();
        w.u8(3).u32(MAX_HASHES as u32 + 1);
        assert_eq!(Frame::decode(&w.finish()), Err(Error::TooLarge));

        let mut w = Writer::new();
        w.u8(3).u32(u32::MAX);
        assert_eq!(Frame::decode(&w.finish()), Err(Error::TooLarge));

        // And a frame longer than this post will read is refused before
        // it is looked at at all.
        assert_eq!(
            Frame::decode(&vec![5u8; MAX_FRAME + 1]),
            Err(Error::TooLarge)
        );
    }
}
