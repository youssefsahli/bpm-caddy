//! The conversation, as a state machine that touches nothing.
//!
//! [`Session`] never opens a socket, never reads a clock and never
//! decides to talk to anybody. It is handed bytes and it answers with
//! bytes; a caller carries them across whatever it likes. That is what
//! lets two whole posts — pairing, key exchange, divergence and all —
//! be run against each other inside one `#[test]`, which is the only
//! way a protocol ever gets looked at closely enough.
//!
//! # The handshake is not invented here
//!
//! It is Noise XX (`Noise_XX_25519_ChaChaPoly_BLAKE2s`), through
//! [`snow`]. A protocol written for one application is a protocol nobody
//! has attacked, and this one carries health data. What XX gives is an
//! encrypted channel where each end has proved it holds *some* static
//! key; what it cannot give is any reason to believe that key belongs to
//! the officine's other post rather than to whatever sat in the middle.
//!
//! Two things close that gap, and neither is optional.
//!
//! **The identity is bound to the channel.** Straight after the
//! handshake each post sends a [`Frame::Hello`] carrying its
//! [`DeviceId`] and an Ed25519 signature *over the handshake's own
//! hash*. A recording of yesterday's conversation replays into nothing:
//! the hash is different, so the signature does not check.
//!
//! **A stranger is admitted by a human, once.** The first meeting shows
//! both operators the same five groups of four characters — derived from
//! that same handshake hash — and they read them to each other. Equal
//! means there is nobody in between. Different means there is, and the
//! pairing is refused. There is no way to skip it: a post not already
//! known is not talked to until [`Session::accept`] has been called, and
//! the only thing that calls it is a person.
//!
//! # The exchange
//!
//! Heads, then rounds. Each round both posts send one [`Frame::Want`] —
//! *what I need and what I hold* — answer the other's with the records
//! it lacks, and close with [`Frame::EndRound`]. A round where both
//! wants were empty is the end: the two journals agree. The want
//! carries both halves so that a first sync is one round and not one per
//! generation of the graph, and the round is closed explicitly so that
//! « I have finished answering » is something the other end is *told*
//! rather than something it infers from a silence.

use std::collections::{BTreeSet, VecDeque};

use crate::journal::Journal;
use crate::keys::{
    short_code, Device, DeviceId, Fingerprint, Ticket, Trousseau, ROLE_INVITE, ROLE_JOIN,
};
use crate::meter::{Meter, Signal};
use crate::seal::{Hash, Record};
use crate::wire::{Frame, MAX_HASHES, MAX_WIRE, PROTOCOL};
use crate::{Error, Result};

/// The Noise pattern. XX because neither post knows the other's static
/// key in advance — which is the true situation the first time two
/// machines in an officine meet, and pretending otherwise would mean
/// shipping a key somewhere.
const PATTERN: &str = "Noise_XX_25519_ChaChaPoly_BLAKE2s";

/// The most records handed over in one round. A large journal crosses in
/// pieces; the piece that does not fit is asked for again next round.
const MOST_PER_ROUND: usize = 256;

/// A conversation that has not ended after this many rounds is not a
/// conversation, it is a peer keeping this post busy.
const MOST_ROUNDS: u32 = 64;

/// What this post is here to do.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Intent {
    /// Ordinary sync with a post this one has already paired with. A
    /// stranger met under this intent is refused, not offered a code:
    /// a pairing is a thing somebody *decides* to do.
    Sync,
    /// Let a new post into this officine. This side holds the trousseau.
    Invite,
    /// Join an officine. This side has no trousseau yet and receives one.
    Join,
}

/// What the caller should do next.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Step {
    /// Hand these bytes to the peer.
    Send(Vec<u8>),
    /// Nothing to say; read from the peer and [`Session::deliver`] it.
    Await,
    /// Show these five groups to the operator, and to the operator at
    /// the other end. Same code both sides means nobody is in between.
    /// Then either [`Session::accept`], or drop the session.
    Confirm(Fingerprint),
    /// Both journals agree. Nothing further.
    Done,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Phase {
    Handshake,
    Greet,
    Confirm,
    Exchange,
    Finished,
}

/// One conversation with one peer.
pub struct Session {
    device: Device,
    trousseau: Option<Trousseau>,
    intent: Intent,
    known: Vec<DeviceId>,

    handshake: Option<snow::HandshakeState>,
    transport: Option<snow::TransportState>,
    hash: Option<[u8; 32]>,
    code: Option<Fingerprint>,

    peer: Option<DeviceId>,
    accepted: bool,
    /// The invitation's ticket, when the pairing came with a code. See
    /// [`Ticket`].
    ticket: Option<Ticket>,
    /// Answering for any of several officines (networks): each with the
    /// posts it knows. The one used is the one the caller names in its
    /// `Hello` — see [`Session::answer_any`].
    choices: Vec<(Trousseau, Vec<DeviceId>)>,
    chosen: Option<usize>,
    /// The one post this conversation is for, when the caller knows it —
    /// a pairing started from a specific neighbour.
    expected: Option<DeviceId>,
    /// Chosen, and the exchange not yet opened on that network's journal.
    to_open: bool,
    /// The inviting side has not yet read the joiner's [`Frame::Proof`]:
    /// it neither shows a code nor hands over a key until it has.
    proof_due: bool,
    welcome: Option<[u8; 32]>,
    adopted: bool,

    phase: Phase,
    out: VecDeque<Frame>,

    peer_heads: Vec<Hash>,
    /// What this post asked for in the round now open.
    last_want: Vec<Hash>,
    /// Records the peer was asked for, did not send, and said it had
    /// nothing more to send. Not asked for again: a peer that does not
    /// hold a record is not going to start.
    unavailable: BTreeSet<Hash>,
    my_want_empty: bool,
    peer_want_empty: Option<bool>,
    peer_more: bool,
    my_end_sent: bool,
    peer_end: bool,
    rounds: u32,
}

impl Session {
    /// Opens a conversation.
    ///
    /// `initiator` is which end speaks first — the caller that dialled,
    /// not the one that answered. `known` is every post this officine
    /// has already paired with; a peer outside it is refused under
    /// [`Intent::Sync`] and offered a code under the other two.
    pub fn new(
        device: &Device,
        trousseau: Option<&Trousseau>,
        intent: Intent,
        initiator: bool,
        known: &[DeviceId],
    ) -> Result<Self> {
        // The two intents that need a trousseau, and the one that must
        // not have one: a post that already belongs to an officine does
        // not « join » a second, and one that belongs to none cannot
        // invite anybody into it.
        match (intent, trousseau.is_some()) {
            (Intent::Sync, false) | (Intent::Invite, false) | (Intent::Join, true) => {
                return Err(Error::Protocol)
            }
            _ => {}
        }
        let secret = device.static_secret();
        let builder = snow::Builder::new(PATTERN.parse().map_err(|_| Error::Handshake)?)
            .local_private_key(&secret)
            .map_err(|_| Error::Handshake)?;
        let handshake = if initiator {
            builder.build_initiator()
        } else {
            builder.build_responder()
        }
        .map_err(|_| Error::Handshake)?;

        Ok(Self {
            device: Device::from_seed(device.seed()),
            trousseau: trousseau.cloned(),
            intent,
            known: known.to_vec(),
            handshake: Some(handshake),
            transport: None,
            hash: None,
            code: None,
            peer: None,
            accepted: false,
            ticket: None,
            proof_due: false,
            choices: Vec::new(),
            chosen: None,
            expected: None,
            to_open: false,
            welcome: None,
            adopted: false,
            phase: Phase::Handshake,
            out: VecDeque::new(),
            peer_heads: Vec::new(),
            last_want: Vec::new(),
            unavailable: BTreeSet::new(),
            my_want_empty: false,
            peer_want_empty: None,
            peer_more: false,
            my_end_sent: false,
            peer_end: false,
            rounds: 0,
        })
    }

    /// A pairing that came with an invitation code: each end proves it
    /// holds the ticket over this handshake, and neither operator is
    /// shown a code to compare — the ticket, handed over by a channel
    /// the inviter trusts, is what the comparison would have proved.
    ///
    /// **Strict**: an inviter holding a ticket refuses a joiner that has
    /// none, rather than falling back to the comparison.
    ///
    /// Only a pairing takes one. An ordinary sync already knows its peer.
    pub fn with_ticket(mut self, ticket: Ticket) -> Result<Self> {
        if self.intent == Intent::Sync {
            return Err(Error::Protocol);
        }
        self.ticket = Some(ticket);
        Ok(self)
    }

    /// **Answer an ordinary sync for whichever of several officines the
    /// caller belongs to** — a post that is in more than one network holds
    /// one door, and cannot know which network a knock is for until the
    /// caller has said who it is.
    ///
    /// This end's own `Hello` therefore names no officine; the caller's
    /// names one, and it must be one of `networks` *and* the caller must
    /// be a post that network knows. Then [`Session::chosen`] says which,
    /// and [`drive_any`] opens the exchange on that network's journal.
    pub fn answer_any(device: &Device, networks: Vec<(Trousseau, Vec<DeviceId>)>) -> Result<Self> {
        let first = networks.first().ok_or(Error::Protocol)?.0.clone();
        let mut s = Self::new(device, Some(&first), Intent::Sync, false, &[])?;
        s.choices = networks;
        Ok(s)
    }

    /// **Only this post, or nobody**: a pairing started from one
    /// neighbour refuses any other device the moment it has proved who it
    /// is — before a code is ever shown, so a third party quicker to the
    /// door does not get the comparison screen.
    pub fn expecting(mut self, peer: DeviceId) -> Self {
        self.expected = Some(peer);
        self
    }

    /// Which of [`Session::answer_any`]'s networks the caller named.
    pub fn chosen(&self) -> Option<usize> {
        self.chosen
    }

    /// Open the exchange on the chosen network's journal.
    fn open_chosen(&mut self, journal: &Journal) {
        if self.to_open {
            self.to_open = false;
            self.open_exchange(journal);
        }
    }

    /// Which post is at the other end, once it has proved it.
    pub fn peer(&self) -> Option<DeviceId> {
        self.peer
    }

    /// The code the two operators compare. `None` before the handshake
    /// has finished — there is nothing to compare until then.
    pub fn code(&self) -> Option<Fingerprint> {
        self.code
    }

    /// The officine's key, for a post that has just joined one. The
    /// caller writes it where the base's own key lives and nowhere else.
    pub fn joined(&self) -> Option<&Trousseau> {
        self.adopted.then_some(self.trousseau.as_ref()).flatten()
    }

    /// What to do next.
    pub fn step(&mut self, meter: &mut Meter) -> Result<Step> {
        match self.phase {
            Phase::Handshake => {
                let hs = self.handshake.as_mut().ok_or(Error::Handshake)?;
                if hs.is_my_turn() {
                    let mut buffer = vec![0u8; MAX_WIRE];
                    let n = hs
                        .write_message(&[], &mut buffer)
                        .map_err(|_| Error::Handshake)?;
                    buffer.truncate(n);
                    if hs.is_handshake_finished() {
                        self.finish_handshake(meter)?;
                    }
                    meter.wrote_bytes(buffer.len());
                    Ok(Step::Send(buffer))
                } else {
                    Ok(Step::Await)
                }
            }
            Phase::Greet | Phase::Exchange => self.emit(meter),
            // Once the operator has said yes, there is nothing left to
            // ask them: this side is waiting for the key to arrive.
            // Returning `Confirm` again here is what would stop the
            // caller ever reading from the link — the conversation then
            // spins on a question already answered.
            Phase::Confirm if !self.out.is_empty() => self.emit(meter),
            Phase::Confirm if self.accepted => Ok(Step::Await),
            // Waiting on the joiner's proof, or — joining with a ticket —
            // on the inviter's: no code is shown in either case.
            Phase::Confirm if self.proof_due => Ok(Step::Await),
            Phase::Confirm if self.intent == Intent::Join && self.ticket.is_some() => {
                Ok(Step::Await)
            }
            Phase::Confirm => match self.code {
                Some(code) => Ok(Step::Confirm(code)),
                None => Err(Error::Handshake),
            },
            Phase::Finished => Ok(Step::Done),
        }
    }

    /// The operator compared the code with the operator at the other
    /// end, they matched, and this post is willing to talk to that one.
    ///
    /// This is the whole of the authorisation. Nothing else calls it.
    pub fn accept(&mut self, journal: &Journal, meter: &mut Meter) -> Result<()> {
        if self.phase != Phase::Confirm || self.accepted || self.proof_due {
            return Err(Error::Protocol);
        }
        self.accepted = true;
        match self.intent {
            Intent::Invite => {
                let secret = self.trousseau.as_ref().ok_or(Error::Protocol)?.secret();
                self.out.push_back(Frame::Welcome(secret));
                meter.note(Signal::Pairing);
                self.open_exchange(journal);
            }
            Intent::Join => {
                if let Some(secret) = self.welcome.take() {
                    self.adopt(secret, journal, meter);
                }
                // Otherwise the welcome has not arrived yet: `deliver`
                // will adopt it when it does, now that consent is given.
            }
            Intent::Sync => self.open_exchange(journal),
        }
        Ok(())
    }

    /// Bytes from the peer.
    pub fn deliver(
        &mut self,
        bytes: &[u8],
        journal: &mut Journal,
        meter: &mut Meter,
    ) -> Result<()> {
        meter.read_bytes(bytes.len());
        if bytes.len() > MAX_WIRE {
            meter.refuse(Error::TooLarge);
            return Err(Error::TooLarge);
        }
        if self.phase == Phase::Handshake {
            let hs = self.handshake.as_mut().ok_or(Error::Handshake)?;
            let mut buffer = vec![0u8; MAX_WIRE];
            hs.read_message(bytes, &mut buffer)
                .map_err(|_| Error::Handshake)?;
            if hs.is_handshake_finished() {
                self.finish_handshake(meter)?;
            }
            return Ok(());
        }

        let transport = self.transport.as_mut().ok_or(Error::Protocol)?;
        let mut buffer = vec![0u8; MAX_WIRE];
        let n = transport
            .read_message(bytes, &mut buffer)
            .map_err(|_| Error::Seal)?;
        buffer.truncate(n);
        let frame = Frame::decode(&buffer).inspect_err(|e| meter.refuse(*e))?;
        self.receive(frame, journal, meter)
    }

    /// Everything that happens the moment the handshake closes: the
    /// code, the transport keys, and this post's own `Hello`.
    fn finish_handshake(&mut self, meter: &mut Meter) -> Result<()> {
        let hs = self.handshake.take().ok_or(Error::Handshake)?;
        let mut hash = [0u8; 32];
        let got = hs.get_handshake_hash();
        if got.len() < hash.len() {
            return Err(Error::Handshake);
        }
        hash.copy_from_slice(&got[..32]);
        self.hash = Some(hash);
        self.code = Some(short_code(&hash));
        self.transport = Some(hs.into_transport_mode().map_err(|_| Error::Handshake)?);
        self.phase = Phase::Greet;
        // Answering for several networks, this end does not know which yet.
        let officine = if self.choices.is_empty() {
            self.trousseau.as_ref().map(Trousseau::name)
        } else {
            None
        };
        self.out.push_back(Frame::Hello {
            protocol: PROTOCOL,
            device: self.device.id(),
            officine,
            signature: self.device.sign(&hash),
        });
        meter.note(Signal::Session);
        Ok(())
    }

    /// Takes the next frame off the queue and seals it.
    fn emit(&mut self, meter: &mut Meter) -> Result<Step> {
        let Some(frame) = self.out.pop_front() else {
            return Ok(Step::Await);
        };
        let plain = frame.encode();
        let transport = self.transport.as_mut().ok_or(Error::Protocol)?;
        let mut buffer = vec![0u8; MAX_WIRE];
        let n = transport
            .write_message(&plain, &mut buffer)
            .map_err(|_| Error::Seal)?;
        buffer.truncate(n);
        meter.wrote_bytes(buffer.len());
        Ok(Step::Send(buffer))
    }

    fn receive(&mut self, frame: Frame, journal: &mut Journal, meter: &mut Meter) -> Result<()> {
        match (self.phase, frame) {
            (
                Phase::Greet,
                Frame::Hello {
                    protocol,
                    device,
                    officine,
                    signature,
                },
            ) => {
                if protocol != PROTOCOL {
                    meter.refuse(Error::Protocol);
                    return Err(Error::Protocol);
                }
                let hash = self.hash.ok_or(Error::Handshake)?;
                // The one line that ties this identity to this
                // conversation. Without it, XX proves a key was held and
                // nothing about whose.
                device
                    .verify(&hash, &signature)
                    .inspect_err(|e| meter.refuse(*e))?;
                if self.expected.is_some_and(|e| e != device) {
                    meter.refuse(Error::Unknown);
                    return Err(Error::Unknown);
                }
                self.peer = Some(device);
                meter.note(Signal::Handshake);
                meter.met(device.fingerprint());
                self.greeted(device, officine, journal, meter)
            }
            (Phase::Confirm, Frame::Welcome(secret)) => {
                if self.intent != Intent::Join {
                    meter.refuse(Error::Protocol);
                    return Err(Error::Protocol);
                }
                if self.accepted {
                    self.adopt(secret, journal, meter);
                } else {
                    // Held until a human has compared the code. A key
                    // adopted before that is a key from whoever was in
                    // the middle.
                    self.welcome = Some(secret);
                }
                Ok(())
            }
            (Phase::Confirm, Frame::Proof(proof)) => self.proved(proof, journal, meter),
            (Phase::Exchange, Frame::Heads(heads)) => {
                self.peer_heads = heads;
                self.begin_round(journal);
                Ok(())
            }
            (Phase::Exchange, Frame::Want { need, have }) => {
                if need.len() > MAX_HASHES || have.len() > MAX_HASHES {
                    meter.refuse(Error::TooLarge);
                    return Err(Error::TooLarge);
                }
                self.peer_want_empty = Some(need.is_empty());
                // One more than fits, so that « there was more » is
                // something this post *knows* rather than infers.
                let mut answer = journal.since(&need, &have, MOST_PER_ROUND + 1);
                let more = answer.len() > MOST_PER_ROUND;
                answer.truncate(MOST_PER_ROUND);
                for record in answer {
                    self.out.push_back(Frame::Give(record.encode()));
                    meter.note(Signal::Sent);
                }
                self.out.push_back(Frame::EndRound { more });
                self.my_end_sent = true;
                self.settle(journal, meter)
            }
            (Phase::Exchange, Frame::Give(bytes)) => {
                let record = Record::decode(&bytes).inspect_err(|e| meter.refuse(*e))?;
                if journal.insert(record) {
                    meter.note(Signal::Received);
                } else {
                    meter.note(Signal::Known);
                }
                Ok(())
            }
            (Phase::Exchange, Frame::EndRound { more }) => {
                self.peer_end = true;
                self.peer_more = more;
                self.settle(journal, meter)
            }
            // Anything else is a frame out of turn: a `Welcome` during
            // an ordinary sync, a `Heads` before anybody said who they
            // are. Refused rather than tolerated — the state machine is
            // the only thing keeping the key from crossing to a post
            // nobody confirmed.
            _ => {
                meter.refuse(Error::Protocol);
                Err(Error::Protocol)
            }
        }
    }

    /// What this post does once it knows who it is talking to.
    fn greeted(
        &mut self,
        device: DeviceId,
        officine: Option<Fingerprint>,
        journal: &Journal,
        meter: &mut Meter,
    ) -> Result<()> {
        let mine = self.trousseau.as_ref().map(Trousseau::name);
        match self.intent {
            Intent::Sync if !self.choices.is_empty() => {
                // The network the caller names, if it is one of ours and
                // knows this caller.
                let found = self
                    .choices
                    .iter()
                    .position(|(t, known)| Some(t.name()) == officine && known.contains(&device));
                let Some(i) = found else {
                    meter.refuse(Error::Unknown);
                    return Err(Error::Unknown);
                };
                self.trousseau = Some(self.choices[i].0.clone());
                self.known = self.choices[i].1.clone();
                self.chosen = Some(i);
                self.to_open = true;
                self.phase = Phase::Exchange;
                Ok(())
            }
            Intent::Sync => {
                // A stranger under this intent is refused. Pairing is a
                // decision, not something that happens because two
                // machines found each other.
                //
                // A post answering for several networks names none in its
                // `Hello` (see `answer_any`): it is the caller's officine
                // that chooses, and the answering end checks it.
                if !self.known.contains(&device) || (officine.is_some() && officine != mine) {
                    meter.refuse(Error::Unknown);
                    return Err(Error::Unknown);
                }
                self.open_exchange(journal);
                Ok(())
            }
            Intent::Invite => {
                // The peer must be joining: it has no officine yet.
                if officine.is_some() {
                    meter.refuse(Error::Protocol);
                    return Err(Error::Protocol);
                }
                self.phase = Phase::Confirm;
                // The joiner speaks first: its proof, or its « none ».
                self.proof_due = true;
                Ok(())
            }
            Intent::Join => {
                // The peer must have one to give.
                if officine.is_none() {
                    meter.refuse(Error::Protocol);
                    return Err(Error::Protocol);
                }
                self.phase = Phase::Confirm;
                let hash = self.hash.ok_or(Error::Handshake)?;
                let proof = self.ticket.map(|t| *t.proof(ROLE_JOIN, &hash).as_bytes());
                self.out.push_back(Frame::Proof(proof));
                Ok(())
            }
        }
    }

    /// The other end's [`Frame::Proof`].
    ///
    /// The inviter reads the joiner's: a proof that checks is the
    /// operator's « same code », and the inviter answers with its own
    /// before the key; `None` falls back to the comparison; a proof that
    /// does not check — or one for a ticket this post never issued — ends
    /// the conversation. The joiner reads the inviter's, and only when it
    /// sent one itself.
    fn proved(
        &mut self,
        proof: Option<[u8; 32]>,
        journal: &Journal,
        meter: &mut Meter,
    ) -> Result<()> {
        let hash = self.hash.ok_or(Error::Handshake)?;
        match self.intent {
            Intent::Invite if self.proof_due => {
                self.proof_due = false;
                match (proof, self.ticket) {
                    // Without a ticket of its own, the comparison.
                    (None, None) => Ok(()),
                    // **An invitation with a code takes no « none ».**
                    // Falling back would hand the inviter's operator a
                    // comparison while the post holding the code sees a
                    // connection that never answers — somebody quicker to
                    // the open door than the invited post is then one hurried
                    // « same code » away from the key.
                    (Some(p), Some(t)) if blake3::Hash::from(p) == t.proof(ROLE_JOIN, &hash) => {
                        let mine = *t.proof(ROLE_INVITE, &hash).as_bytes();
                        self.out.push_back(Frame::Proof(Some(mine)));
                        self.accept(journal, meter)
                    }
                    _ => {
                        meter.refuse(Error::Unknown);
                        Err(Error::Unknown)
                    }
                }
            }
            Intent::Join if !self.accepted => match (proof, self.ticket) {
                (Some(p), Some(t)) if blake3::Hash::from(p) == t.proof(ROLE_INVITE, &hash) => {
                    self.accept(journal, meter)
                }
                _ => {
                    meter.refuse(Error::Unknown);
                    Err(Error::Unknown)
                }
            },
            _ => {
                meter.refuse(Error::Protocol);
                Err(Error::Protocol)
            }
        }
    }

    fn adopt(&mut self, secret: [u8; 32], journal: &Journal, meter: &mut Meter) {
        self.trousseau = Some(Trousseau::from_secret(secret));
        self.adopted = true;
        meter.note(Signal::Pairing);
        self.open_exchange(journal);
    }

    fn open_exchange(&mut self, journal: &Journal) {
        self.phase = Phase::Exchange;
        self.out.push_back(Frame::Heads(capped(journal.heads())));
    }

    /// Opens a round: what this post needs, and what it holds.
    fn begin_round(&mut self, journal: &Journal) {
        let mut need: Vec<Hash> = self
            .peer_heads
            .iter()
            .chain(journal.missing().iter())
            .copied()
            .filter(|h| !journal.has(h) && !self.unavailable.contains(h))
            .collect();
        need.sort();
        need.dedup();
        need.truncate(MAX_HASHES);
        self.last_want = need.clone();
        self.my_want_empty = need.is_empty();
        self.my_end_sent = false;
        self.peer_end = false;
        self.peer_more = false;
        self.peer_want_empty = None;
        self.out.push_back(Frame::Want {
            need,
            have: capped(journal.heads()),
        });
    }

    /// Has the round closed, and does the conversation go on?
    fn settle(&mut self, journal: &Journal, meter: &mut Meter) -> Result<()> {
        let peer_want_empty = match self.peer_want_empty {
            Some(empty) => empty,
            None => return Ok(()),
        };
        if !(self.my_end_sent && self.peer_end) {
            return Ok(());
        }
        // What was asked for, not sent, and not held back by the cap:
        // the peer has not got it. Asking again would be asking for
        // ever — this is the only thing that comes off a want list, and
        // it comes off because the peer said it had nothing more.
        if !self.peer_more {
            for hash in std::mem::take(&mut self.last_want) {
                if !journal.has(&hash) {
                    self.unavailable.insert(hash);
                }
            }
        }
        // Both asked for nothing and neither stopped at the cap: the
        // two journals agree, and that is something both ends know
        // rather than something one assumes.
        if self.my_want_empty && peer_want_empty && !self.peer_more {
            self.phase = Phase::Finished;
            meter.note(Signal::Agreed);
            return Ok(());
        }
        self.rounds += 1;
        if self.rounds > MOST_ROUNDS {
            meter.refuse(Error::Protocol);
            return Err(Error::Protocol);
        }
        self.begin_round(journal);
        Ok(())
    }
}

/// Runs a whole conversation over a [`Link`], to its end.
///
/// The loop is four lines and there is exactly one way to write it
/// correctly, so it is written **here** rather than in each caller: the
/// one thing a caller could get wrong is calling [`Session::accept`]
/// without having shown anybody the code, and the shape of this
/// function makes that hard — `confirm` is handed the code and has to
/// answer, and answering `false` ends the conversation rather than
/// carrying on quietly.
///
/// It blocks, so it belongs on a thread of its own — the same
/// arrangement as `maintenance.rs` and the update check. Nothing here
/// retries, reconnects or schedules: the caller decides when two posts
/// talk, which is the rule [`crate::link`] states and this function
/// does not get to soften.
pub fn drive<L: crate::Link>(
    session: &mut Session,
    link: &mut L,
    journal: &mut Journal,
    meter: &mut Meter,
    confirm: &mut dyn FnMut(Fingerprint) -> bool,
) -> Result<()> {
    loop {
        match session.step(meter)? {
            Step::Send(bytes) => link.send(&bytes)?,
            Step::Await => {
                let bytes = link.recv()?;
                session.deliver(&bytes, journal, meter)?;
            }
            Step::Confirm(code) => {
                if !confirm(code) {
                    // « Ce n'est pas le même code » is not a cancelled
                    // pairing, it is somebody in the middle. It ends
                    // here, and it is counted.
                    meter.refuse(Error::Unknown);
                    return Err(Error::Unknown);
                }
                session.accept(journal, meter)?;
            }
            Step::Done => return Ok(()),
        }
    }
}

/// [`drive`] for a session made by [`Session::answer_any`]: the journal is
/// the chosen network's, **loaded only once the caller has named it** —
/// through `load`, given the network's index. Before that nothing but the
/// keys and member lists is needed, so a knock that never authenticates
/// costs the answering post no journal read. Returns the index and the
/// journal, which the caller keeps.
pub fn drive_any<L: crate::Link>(
    session: &mut Session,
    link: &mut L,
    load: &mut dyn FnMut(usize) -> Journal,
    meter: &mut Meter,
) -> Result<(usize, Journal)> {
    if session.choices.is_empty() {
        return Err(Error::Protocol);
    }
    // Greeting frames touch no journal; this one stands in until then.
    let mut journal = Journal::new();
    loop {
        match session.step(meter)? {
            Step::Send(bytes) => link.send(&bytes)?,
            Step::Await => {
                let bytes = link.recv()?;
                session.deliver(&bytes, &mut journal, meter)?;
                if let (Some(i), true) = (session.chosen, session.to_open) {
                    journal = load(i);
                    session.open_chosen(&journal);
                }
            }
            // An ordinary sync never shows a code.
            Step::Confirm(_) => return Err(Error::Protocol),
            Step::Done => {
                let i = session.chosen.ok_or(Error::Protocol)?;
                return Ok((i, journal));
            }
        }
    }
}

/// A list of hashes cut to what one frame carries. Truncating heads
/// costs a round, never a record: whatever is left out is named again
/// next time, because it is still a head.
fn capped(mut hashes: Vec<Hash>) -> Vec<Hash> {
    hashes.truncate(MAX_HASHES);
    hashes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::link::{loopback, Link};
    use crate::seal::Stream;
    use crate::tests::Counted;
    use crate::OsEntropy;

    struct Post {
        device: Device,
        trousseau: Option<Trousseau>,
        journal: Journal,
        meter: Meter,
        known: Vec<DeviceId>,
    }

    impl Post {
        fn new(seed: u8, trousseau: Option<Trousseau>) -> Self {
            let mut e = Counted(seed);
            Self {
                device: Device::generate(&mut e),
                trousseau,
                journal: Journal::new(),
                meter: Meter::new(),
                known: Vec::new(),
            }
        }

        fn write(&mut self, line: &[u8]) -> Hash {
            let t = self.trousseau.clone().expect("une officine");
            let mut e = OsEntropy;
            self.journal
                .write(&self.device, &t, Stream::Registre, line, None, &mut e)
                .unwrap()
        }

        fn lines(&self) -> Vec<Vec<u8>> {
            let t = self.trousseau.clone().expect("une officine");
            self.journal
                .read(&t, Stream::Registre)
                .facts
                .into_iter()
                .map(|f| f.payload)
                .collect()
        }
    }

    /// Runs two sessions against each other over a loopback link,
    /// answering every `Confirm` the way two operators who compared
    /// their codes would. Returns what each end was shown.
    fn converse(
        a: &mut Post,
        b: &mut Post,
        mut sa: Session,
        mut sb: Session,
        agree: bool,
    ) -> Result<Talked> {
        let (mut la, mut lb) = loopback::pair();
        let (mut code_a, mut code_b) = (None, None);
        let mut wire: Vec<Vec<u8>> = Vec::new();
        for _ in 0..4096 {
            let mut moved = false;
            // One end, then the other, until neither has anything to do.
            for first in [true, false] {
                let (post, session, link, seen) = if first {
                    (&mut *a, &mut sa, &mut la, &mut code_a)
                } else {
                    (&mut *b, &mut sb, &mut lb, &mut code_b)
                };
                match session.step(&mut post.meter)? {
                    Step::Send(bytes) => {
                        wire.push(bytes.clone());
                        link.send(&bytes).unwrap();
                        moved = true;
                    }
                    Step::Await => {
                        if let Ok(bytes) = link.recv() {
                            session.deliver(&bytes, &mut post.journal, &mut post.meter)?;
                            moved = true;
                        }
                    }
                    Step::Confirm(code) => {
                        *seen = Some(code);
                        if !agree {
                            return Ok(Talked {
                                code_a,
                                code_b,
                                sa,
                                sb,
                                wire,
                            });
                        }
                        session.accept(&post.journal, &mut post.meter)?;
                        moved = true;
                    }
                    Step::Done => {}
                }
            }
            if sa.phase == Phase::Finished && sb.phase == Phase::Finished {
                return Ok(Talked {
                    code_a,
                    code_b,
                    sa,
                    sb,
                    wire,
                });
            }
            if !moved {
                panic!("la conversation n'avance plus");
            }
        }
        panic!("la conversation ne se termine pas");
    }

    /// What a conversation leaves behind: what each operator was shown,
    /// and the two sessions — a joining post's new trousseau is on its
    /// session until the caller takes it, which is the point.
    struct Talked {
        code_a: Option<Fingerprint>,
        code_b: Option<Fingerprint>,
        sa: Session,
        sb: Session,
        /// Every byte that crossed, in order. What a relay would hold.
        wire: Vec<Vec<u8>>,
    }

    fn officine(seed: u8) -> Trousseau {
        let mut e = Counted(seed);
        Trousseau::generate(&mut e)
    }

    fn pair_posts(a: &mut Post, b: &mut Post) -> (Option<Fingerprint>, Option<Fingerprint>) {
        let sa = Session::new(&a.device, a.trousseau.as_ref(), Intent::Invite, true, &[]).unwrap();
        let sb = Session::new(&b.device, None, Intent::Join, false, &[]).unwrap();
        let (b_id, a_id) = (b.device.id(), a.device.id());
        let talked = converse(a, b, sa, sb, true).unwrap();
        // Each end knows which post it spoke to, and it knows it
        // because that post signed this handshake — not because it
        // said so.
        assert_eq!(talked.sa.peer(), Some(b_id));
        assert_eq!(talked.sb.peer(), Some(a_id));
        b.trousseau = talked.sb.joined().cloned();
        a.known.push(b_id);
        b.known.push(a_id);
        (talked.code_a, talked.code_b)
    }

    fn sync_posts(a: &mut Post, b: &mut Post) {
        let sa = Session::new(
            &a.device,
            a.trousseau.as_ref(),
            Intent::Sync,
            true,
            &a.known.clone(),
        )
        .unwrap();
        let sb = Session::new(
            &b.device,
            b.trousseau.as_ref(),
            Intent::Sync,
            false,
            &b.known.clone(),
        )
        .unwrap();
        converse(a, b, sa, sb, true).unwrap();
    }

    /// The whole thing, once: a post with a register, a post with
    /// nothing, a pairing, and two journals that agree afterwards.
    #[test]
    fn a_new_post_joins_an_officine_and_gets_what_it_wrote() {
        let t = officine(5);
        let mut a = Post::new(1, Some(t.clone()));
        let mut b = Post::new(2, None);
        for line in ["12 comprimes", "8 comprimes", "inventaire : 40"] {
            a.write(line.as_bytes());
        }

        let (code_a, code_b) = pair_posts(&mut a, &mut b);
        // Both operators were shown the same code. That is the whole
        // authentication: equal means nobody is in between.
        assert!(code_a.is_some());
        assert_eq!(code_a, code_b);

        // The joining post now belongs to the officine...
        assert!(b.trousseau.as_ref().unwrap().same(&t));
        // ...and holds what the other one had written.
        assert_eq!(b.journal.len(), 3);
        assert_eq!(a.lines(), b.lines());
        assert_eq!(a.journal.heads(), b.journal.heads());
        assert_eq!(b.meter.report().pairings, 1);
        assert_eq!(b.meter.report().received, 3);
        assert_eq!(a.meter.report().agreed, 1);
    }

    /// Two paired posts, each writing while the other cannot see it,
    /// end up holding one journal in one order.
    #[test]
    fn two_posts_that_wrote_apart_agree_afterwards() {
        let t = officine(5);
        let mut a = Post::new(1, Some(t.clone()));
        let mut b = Post::new(2, None);
        pair_posts(&mut a, &mut b);

        a.write(b"depuis le comptoir");
        b.write(b"depuis le bureau");
        a.write(b"encore le comptoir");
        sync_posts(&mut a, &mut b);

        assert_eq!(a.journal.len(), 3);
        assert_eq!(a.lines(), b.lines());
        assert_eq!(a.lines().len(), 3);

        // And a second sync with nothing new is one round that moves no
        // record — the ordinary evening.
        let before = a.meter.report().sent;
        sync_posts(&mut a, &mut b);
        assert_eq!(a.meter.report().sent, before);
        assert_eq!(a.lines(), b.lines());
    }

    /// A divergence survives the wire. Two posts correcting the same
    /// register line without seeing each other: both corrections arrive,
    /// both stay, and each names the other on **both** posts.
    #[test]
    fn a_divergence_crosses_the_wire_and_is_still_a_divergence() {
        let t = officine(5);
        let mut a = Post::new(1, Some(t.clone()));
        let mut b = Post::new(2, None);
        let line = a.write(b"12 comprimes");
        pair_posts(&mut a, &mut b);

        let write_correction = |post: &mut Post, text: &[u8]| {
            let t = post.trousseau.clone().unwrap();
            let mut e = OsEntropy;
            post.journal
                .write(&post.device, &t, Stream::Registre, text, Some(line), &mut e)
                .unwrap()
        };
        let fix_a = write_correction(&mut a, b"annulation : 21");
        let fix_b = write_correction(&mut b, b"annulation : 20");
        sync_posts(&mut a, &mut b);

        for post in [&a, &b] {
            let t = post.trousseau.clone().unwrap();
            let reading = post.journal.read(&t, Stream::Registre);
            assert_eq!(reading.facts.len(), 2, "les deux corrections restent");
            for fact in &reading.facts {
                assert_eq!(fact.rivals.len(), 1);
            }
            assert!(post.journal.has(&fix_a) && post.journal.has(&fix_b));
            assert!(post.journal.has(&line), "la ligne corrigée reste écrite");
        }
    }

    /// The whole thing again, this time over two real sockets on two
    /// real threads, driven by the loop callers are meant to use.
    ///
    /// Everything above talks over a queue in memory, which is what
    /// makes the protocol examinable — and which is also exactly the
    /// arrangement that hides a deadlock, because nothing ever blocks.
    /// Here both ends block on `recv`, and a conversation where both
    /// sides wait for the other would hang instead of passing. It is
    /// the same reason `link.rs` tests its framing against a socket
    /// rather than against a model of one.
    #[cfg(feature = "tcp")]
    #[test]
    fn two_posts_pair_and_sync_over_two_real_sockets() {
        use crate::link::{dial, Door};
        use std::time::Duration;

        let t = officine(5);
        let mut a = Post::new(1, Some(t.clone()));
        let mut b = Post::new(2, None);
        for line in ["12 comprimes", "8 comprimes", "inventaire : 40"] {
            a.write(line.as_bytes());
        }

        let patience = Duration::from_secs(20);
        let door = Door::open("127.0.0.1:0").unwrap();
        let address = door.address().unwrap().to_string();

        // The post that holds the officine waits and lets the other in.
        let waiting = std::thread::spawn(move || {
            let mut link = door.accept(patience).unwrap();
            let mut session =
                Session::new(&a.device, a.trousseau.as_ref(), Intent::Invite, false, &[]).unwrap();
            let mut shown = None;
            drive(
                &mut session,
                &mut link,
                &mut a.journal,
                &mut a.meter,
                &mut |code| {
                    shown = Some(code);
                    true
                },
            )
            .unwrap();
            (a, shown)
        });

        let mut link = dial(&address, patience).unwrap();
        let mut session = Session::new(&b.device, None, Intent::Join, true, &[]).unwrap();
        let mut shown = None;
        drive(
            &mut session,
            &mut link,
            &mut b.journal,
            &mut b.meter,
            &mut |code| {
                shown = Some(code);
                true
            },
        )
        .unwrap();
        b.trousseau = session.joined().cloned();

        let (a, other) = waiting.join().unwrap();
        assert!(shown.is_some(), "un code a bien été montré");
        assert_eq!(shown, other, "les deux opérateurs voient le même code");
        assert!(b.trousseau.as_ref().unwrap().same(&t));
        assert_eq!(b.journal.len(), 3);
        assert_eq!(a.lines(), b.lines());
        assert_eq!(a.lines().len(), 3);
        assert_eq!(b.meter.report().agreed, 1);
        assert!(b.meter.report().in_bytes > 0 && b.meter.report().out_bytes > 0);
    }

    /// A recording of yesterday's conversation is worth nothing, and
    /// the reason is visible: the code is different every time.
    ///
    /// The same two posts, twice. If the code repeated, the signature
    /// each end makes over the handshake hash would repeat with it, and
    /// a `Hello` captured once would authenticate for ever. It does not
    /// repeat, because the ephemeral keys are fresh — which is exactly
    /// why the handshake's randomness comes from the system and not
    /// from anything a test could supply.
    #[test]
    fn no_two_conversations_share_a_code() {
        let t = officine(5);
        let mut seen = std::collections::HashSet::new();
        for _ in 0..8 {
            let mut a = Post::new(1, Some(t.clone()));
            let mut b = Post::new(2, None);
            let (code, other) = pair_posts(&mut a, &mut b);
            let code = code.expect("un code");
            assert_eq!(Some(code), other, "les deux bouts voient le même");
            assert!(seen.insert(code.bytes()), "un code s'est répété");
        }
    }

    /// The claim the whole crate is for, measured on the bytes rather
    /// than argued from the design: **whoever carries this conversation
    /// cannot read it**.
    ///
    /// Everything that crossed is collected and searched for the words
    /// that were written, for the officine's key, and for the joining
    /// post's own new key. A relay — the officine's switch, a router, a
    /// server one day — holds exactly these bytes. It also checks the
    /// obvious inverse, that the payload really was in the journal: a
    /// test that passes because nothing was written proves nothing.
    #[test]
    fn a_relay_cannot_read_what_it_carries() {
        let t = officine(5);
        let mut a = Post::new(1, Some(t.clone()));
        let mut b = Post::new(2, None);
        // Something unmistakable, and something a register really says.
        let secrets: [&[u8]; 3] = [
            b"MADAME YVONNE CLEMENT 14 RUE DES LILAS",
            b"methadone 60 mg, dossier 4021",
            b"annulation : erreur de report",
        ];
        for line in secrets {
            a.write(line);
        }

        let sa = Session::new(&a.device, a.trousseau.as_ref(), Intent::Invite, true, &[]).unwrap();
        let sb = Session::new(&b.device, None, Intent::Join, false, &[]).unwrap();
        let talked = converse(&mut a, &mut b, sa, sb, true).unwrap();
        b.trousseau = talked.sb.joined().cloned();

        // The exchange really happened, or the search below is a search
        // through nothing.
        assert_eq!(b.journal.len(), 3);
        assert_eq!(a.lines(), b.lines());
        assert!(!talked.wire.is_empty());

        let carried: Vec<u8> = talked.wire.concat();
        for secret in secrets {
            assert!(
                !contains(&carried, secret),
                "« {} » a traversé en clair",
                String::from_utf8_lossy(secret)
            );
        }
        // And neither did the key that opens them, although it is the
        // one thing this conversation exists to hand over.
        assert!(
            !contains(&carried, &t.secret()),
            "le trousseau a traversé en clair"
        );
        assert!(
            !contains(&carried, &a.device.seed()),
            "la graine du poste a traversé"
        );
        assert!(
            !contains(&carried, &b.device.seed()),
            "la graine du poste a traversé"
        );
    }

    fn contains(haystack: &[u8], needle: &[u8]) -> bool {
        haystack.windows(needle.len()).any(|w| w == needle)
    }

    /// A post nobody paired with is refused, and it is refused *before*
    /// a single record moves. An officine's journal does not go to
    /// whoever manages to open a socket.
    #[test]
    fn a_post_nobody_paired_with_gets_nothing() {
        let t = officine(5);
        let mut a = Post::new(1, Some(t.clone()));
        let mut stranger = Post::new(9, Some(t.clone()));
        a.write(b"12 comprimes");

        let sa = Session::new(&a.device, a.trousseau.as_ref(), Intent::Sync, true, &[]).unwrap();
        let sb = Session::new(
            &stranger.device,
            stranger.trousseau.as_ref(),
            Intent::Sync,
            false,
            &[],
        )
        .unwrap();
        assert_eq!(
            converse(&mut a, &mut stranger, sa, sb, true).err(),
            Some(Error::Unknown)
        );
        assert_eq!(stranger.journal.len(), 0);
        assert!(a.meter.report().refused + stranger.meter.report().refused > 0);
    }

    /// The code is what a human checks, so refusing it must stop
    /// everything. An operator who says « ce n'est pas le même code »
    /// has just seen somebody in the middle.
    #[test]
    fn a_pairing_nobody_confirmed_hands_over_no_key() {
        let t = officine(5);
        let mut a = Post::new(1, Some(t.clone()));
        let mut b = Post::new(2, None);
        a.write(b"12 comprimes");

        let sa = Session::new(&a.device, a.trousseau.as_ref(), Intent::Invite, true, &[]).unwrap();
        let sb = Session::new(&b.device, None, Intent::Join, false, &[]).unwrap();
        let talked = converse(&mut a, &mut b, sa, sb, false).unwrap();

        assert!(talked.sb.joined().is_none(), "aucune clé n'est passée");
        assert!(b.trousseau.is_none());
        assert_eq!(b.journal.len(), 0);
    }

    /// Two officines that meet are two officines. The key is not the
    /// only thing checked — the *name* of the officine is compared
    /// before anything is exchanged, so a post paired long ago that has
    /// since been re-keyed is refused rather than silently sending
    /// records nobody will be able to open.
    #[test]
    fn two_different_officines_do_not_exchange_anything() {
        let mut a = Post::new(1, Some(officine(5)));
        let mut b = Post::new(2, Some(officine(60)));
        a.write(b"12 comprimes");
        let (a_known, b_known) = (vec![b.device.id()], vec![a.device.id()]);

        let sa = Session::new(
            &a.device,
            a.trousseau.as_ref(),
            Intent::Sync,
            true,
            &a_known,
        )
        .unwrap();
        let sb = Session::new(
            &b.device,
            b.trousseau.as_ref(),
            Intent::Sync,
            false,
            &b_known,
        )
        .unwrap();
        assert_eq!(
            converse(&mut a, &mut b, sa, sb, true).err(),
            Some(Error::Unknown)
        );
        assert_eq!(b.journal.len(), 0);
    }

    /// An intent that contradicts what this post holds is refused at the
    /// door: a post with an officine does not join a second, and one
    /// with none cannot let anybody into it.
    #[test]
    fn an_intent_that_contradicts_what_this_post_holds_is_refused() {
        let t = officine(5);
        let with = Post::new(1, Some(t));
        let without = Post::new(2, None);
        for (device, trousseau, intent) in [
            (&with.device, with.trousseau.as_ref(), Intent::Join),
            (&without.device, None, Intent::Invite),
            (&without.device, None, Intent::Sync),
        ] {
            assert_eq!(
                Session::new(device, trousseau, intent, true, &[]).err(),
                Some(Error::Protocol)
            );
        }
        // And the three that agree with themselves are allowed.
        assert!(Session::new(
            &with.device,
            with.trousseau.as_ref(),
            Intent::Sync,
            true,
            &[]
        )
        .is_ok());
        assert!(Session::new(
            &with.device,
            with.trousseau.as_ref(),
            Intent::Invite,
            true,
            &[]
        )
        .is_ok());
        assert!(Session::new(&without.device, None, Intent::Join, true, &[]).is_ok());
    }

    /// A journal larger than one round carries in several, and the cap
    /// costs round trips and never a record.
    #[test]
    fn a_journal_larger_than_one_round_still_crosses_whole() {
        let t = officine(5);
        let mut a = Post::new(1, Some(t.clone()));
        let mut b = Post::new(2, None);
        let many = MOST_PER_ROUND + 40;
        for i in 0..many {
            a.write(format!("ligne {i}").as_bytes());
        }
        pair_posts(&mut a, &mut b);
        assert_eq!(b.journal.len(), many);
        assert_eq!(a.lines(), b.lines());
    }

    /// **With an invitation code, nobody compares anything** — and the
    /// key still crosses only to the post that holds the ticket. Neither
    /// operator is shown a code; the ticket is not on the wire.
    #[test]
    fn a_ticket_pairs_two_posts_without_a_code_to_compare() {
        let t = officine(5);
        let mut a = Post::new(1, Some(t.clone()));
        let mut b = Post::new(2, None);
        a.write(b"12 comprimes");
        let ticket = Ticket::generate(&mut Counted(21));
        let sa = Session::new(&a.device, a.trousseau.as_ref(), Intent::Invite, false, &[])
            .unwrap()
            .with_ticket(ticket)
            .unwrap();
        let sb = Session::new(&b.device, None, Intent::Join, true, &[])
            .unwrap()
            .with_ticket(ticket)
            .unwrap();
        // `agree: false` would stop at the first code shown: there is none.
        let talked = converse(&mut a, &mut b, sa, sb, false).unwrap();
        assert!(talked.code_a.is_none() && talked.code_b.is_none());
        assert!(talked.sb.joined().expect("la clé").same(&t));
        assert_eq!(b.journal.len(), 1);
        for frame in &talked.wire {
            assert!(!contains(frame, &ticket.bytes()), "le ticket a traversé");
        }
    }

    /// A wrong ticket ends the pairing, from either side's point of view,
    /// and no key crosses.
    #[test]
    fn a_wrong_ticket_hands_over_no_key() {
        let t = officine(5);
        let mut a = Post::new(1, Some(t.clone()));
        let mut b = Post::new(2, None);
        let sa = Session::new(&a.device, a.trousseau.as_ref(), Intent::Invite, false, &[])
            .unwrap()
            .with_ticket(Ticket::from_bytes([1; 10]))
            .unwrap();
        let sb = Session::new(&b.device, None, Intent::Join, true, &[])
            .unwrap()
            .with_ticket(Ticket::from_bytes([2; 10]))
            .unwrap();
        assert!(converse(&mut a, &mut b, sa, sb, true).is_err());
        assert_eq!(a.meter.report().pairings, 0);
        assert_eq!(b.meter.report().pairings, 0);
        assert!(b.trousseau.is_none());
    }

    /// An inviter that issued a ticket does **not** fall back to the
    /// comparison for a post that came without it: whoever reached the
    /// open door first is refused, and no key crosses.
    #[test]
    fn an_invitation_with_a_code_refuses_a_joiner_without_it() {
        let t = officine(5);
        let mut a = Post::new(1, Some(t.clone()));
        let mut b = Post::new(2, None);
        let sa = Session::new(&a.device, a.trousseau.as_ref(), Intent::Invite, false, &[])
            .unwrap()
            .with_ticket(Ticket::from_bytes([1; 10]))
            .unwrap();
        let sb = Session::new(&b.device, None, Intent::Join, true, &[]).unwrap();
        // The joiner is shown its code, the inviter never is: the
        // conversation stops at the joiner's comparison or at the refusal.
        match converse(&mut a, &mut b, sa, sb, true) {
            Err(_) => {}
            Ok(talked) => {
                assert!(talked.code_a.is_none(), "l'invitant ne compare rien");
                assert!(talked.sb.joined().is_none(), "aucune clé n'est passée");
            }
        }
        assert_eq!(a.meter.report().pairings, 0);
        assert!(b.trousseau.is_none());
    }

    /// A joiner holding a ticket the inviter never issued gets nothing:
    /// a ticket is not something a joiner can bring on its own.
    #[test]
    fn a_ticket_the_inviter_never_issued_is_refused() {
        let t = officine(5);
        let mut a = Post::new(1, Some(t.clone()));
        let mut b = Post::new(2, None);
        let sa = Session::new(&a.device, a.trousseau.as_ref(), Intent::Invite, false, &[]).unwrap();
        let sb = Session::new(&b.device, None, Intent::Join, true, &[])
            .unwrap()
            .with_ticket(Ticket::from_bytes([2; 10]))
            .unwrap();
        assert!(converse(&mut a, &mut b, sa, sb, true).is_err());
        assert_eq!(b.meter.report().pairings, 0);
        // And an ordinary sync takes no ticket at all.
        let sync = Session::new(&a.device, a.trousseau.as_ref(), Intent::Sync, true, &[]).unwrap();
        assert!(sync.with_ticket(Ticket::from_bytes([1; 10])).is_err());
    }

    /// **One door for several networks**: a post in two officines answers
    /// a caller of the second on that network's journal, and a caller the
    /// named network does not know is refused — even one the other
    /// network knows.
    #[test]
    fn one_door_answers_each_network_on_its_own_journal() {
        let (ta, tb) = (officine(5), officine(6));
        let mut e = Counted(71);
        let door = Device::generate(&mut e);
        let caller = Device::generate(&mut e);
        let other = Device::generate(&mut e);
        // The caller writes in network B.
        let mut theirs = Journal::new();
        theirs
            .write(&caller, &tb, Stream::Reseau, b"de B", None, &mut e)
            .unwrap();
        let mut journals = [Journal::new(), Journal::new()];
        let run =
            |caller: &Device, t: &Trousseau, theirs: &mut Journal, journals: &mut [Journal]| {
                use crate::link::{dial, Door};
                let patience = std::time::Duration::from_secs(20);
                let door_link = Door::open("127.0.0.1:0").unwrap();
                let address = door_link.address().unwrap().to_string();
                let mut answering = Session::answer_any(
                    &door,
                    vec![
                        (ta.clone(), vec![other.id()]),
                        (tb.clone(), vec![caller.id()]),
                    ],
                )
                .unwrap();
                let mut calling =
                    Session::new(caller, Some(t), Intent::Sync, true, &[door.id()]).unwrap();
                std::thread::scope(|sc| {
                    let a = sc.spawn(|| {
                        let mut link = door_link.accept(patience).unwrap();
                        let mut load = |i: usize| std::mem::take(&mut journals[i]);
                        let got =
                            drive_any(&mut answering, &mut link, &mut load, &mut Meter::new());
                        got.map(|(i, j)| {
                            journals[i] = j;
                            i
                        })
                    });
                    let mut link = dial(&address, patience).unwrap();
                    let b = drive(
                        &mut calling,
                        &mut link,
                        theirs,
                        &mut Meter::new(),
                        &mut |_| false,
                    );
                    (a.join().unwrap(), b)
                })
            };
        let (a, b) = run(&caller, &tb, &mut theirs, &mut journals);
        assert_eq!(a, Ok(1), "le second réseau");
        assert!(b.is_ok());
        assert_eq!(journals[1].len(), 1, "reçu dans le journal de B");
        assert_eq!(journals[0].len(), 0, "rien dans celui de A");
        // A caller network A does not know, naming A: refused.
        let mut nobody = Journal::new();
        let (a, _) = run(&caller, &ta, &mut nobody, &mut journals);
        assert!(a.is_err(), "inconnu du réseau qu'il nomme");
    }

    /// **A pairing meant for one post refuses another**, before any code.
    #[test]
    fn a_pairing_meant_for_one_post_refuses_another() {
        let t = officine(5);
        let mut a = Post::new(1, Some(t.clone()));
        let mut b = Post::new(2, None);
        let someone = Post::new(3, None).device.id();
        let sa = Session::new(&a.device, a.trousseau.as_ref(), Intent::Invite, false, &[])
            .unwrap()
            .expecting(someone);
        let sb = Session::new(&b.device, None, Intent::Join, true, &[]).unwrap();
        match converse(&mut a, &mut b, sa, sb, true) {
            Err(_) => {}
            Ok(talked) => assert!(talked.code_a.is_none() && talked.sb.joined().is_none()),
        }
        assert!(b.trousseau.is_none());
        // Expecting the right one, the pairing goes as ever.
        let b_id = b.device.id();
        let sa = Session::new(&a.device, a.trousseau.as_ref(), Intent::Invite, false, &[])
            .unwrap()
            .expecting(b_id);
        let sb = Session::new(&b.device, None, Intent::Join, true, &[]).unwrap();
        let talked = converse(&mut a, &mut b, sa, sb, true).unwrap();
        assert!(talked.sb.joined().is_some());
    }
}
