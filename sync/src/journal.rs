//! The journal: every record the officine has, and what they say once
//! read together.
//!
//! It is a graph, not a list. Each record names the records its author
//! had already seen, so « what happened before what » is written down by
//! the posts themselves and never inferred from a clock. Two posts that
//! write while neither can see the other produce two branches; merging
//! is the union of the two — no record is chosen over another, because
//! **merging never loses anything**. There is no verb here that removes
//! a record, on purpose: a peer that could make this post forget would
//! be a peer that could edit the register from the other end of a wire.
//!
//! What a *reading* adds on top is the one judgement this module makes:
//! a record somebody corrected is no longer current, and its correction
//! is. If two posts corrected the same record without having seen each
//! other, both corrections are current and **each names the other**.
//! That is a divergence, it is in the data rather than beside it, and
//! nothing here settles it — the same refusal the compare-and-set
//! notices make at the counter, for the same reason: choosing silently
//! between two clinical statements is choosing wrong half the time,
//! with the confidence of an answer.

use std::collections::{BTreeMap, BTreeSet};

use crate::keys::{Device, DeviceId, Trousseau};
use crate::seal::{Hash, Record, Stream, MAX_PARENTS};
use crate::{Entropy, Result};

/// One current statement of a stream, read.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Fact {
    pub id: Hash,
    /// The post that wrote it.
    pub author: DeviceId,
    /// Its rank in the causal order. Not a date: the payload carries the
    /// date, because the date is something a human wrote down and this
    /// is something the journal counted.
    pub lamport: u64,
    /// The record this one corrects, if any. That record is still in the
    /// journal and still on paper — struck through, never gone.
    pub corrects: Option<Hash>,
    /// What was sealed.
    pub payload: Vec<u8>,
    /// Other current facts correcting the same record as this one.
    ///
    /// Non-empty is a divergence: two posts answered the same question
    /// without seeing each other. Both are here, each naming the other,
    /// so a caller reading the list cannot fail to notice — which is why
    /// this is a field and not a separate query nobody would call.
    pub rivals: Vec<Hash>,
}

/// What a stream says, and what it could not say.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Reading {
    /// The current facts, in causal order.
    pub facts: Vec<Fact>,
    /// Records of this stream the trousseau in hand would not open.
    ///
    /// Named rather than skipped. A record sealed under a trousseau this
    /// officine has since replaced, or a record somebody damaged, is a
    /// hole in what the screen is about to show — and a list quietly one
    /// line short reads exactly like a complete one. Silence is not
    /// permission here either.
    pub unopened: Vec<Hash>,
}

/// Every record this post holds.
#[derive(Default)]
pub struct Journal {
    records: BTreeMap<Hash, Record>,
    /// Records nothing else names as a parent: what a new record will
    /// be written after, and what a peer is told when asked.
    heads: BTreeSet<Hash>,
    /// Every hash named as a parent by something we hold — including
    /// hashes we do not have yet, which is how [`Journal::missing`]
    /// knows what to ask a peer for.
    named: BTreeSet<Hash>,
    high: u64,
    /// The authors whose ranks move this post's clock, once set — see
    /// [`Journal::trust`]. `None`: every author, as a journal always was.
    trusted: Option<BTreeSet<DeviceId>>,
}

impl Journal {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn has(&self, id: &Hash) -> bool {
        self.records.contains_key(id)
    }

    pub fn get(&self, id: &Hash) -> Option<&Record> {
        self.records.get(id)
    }

    /// Everything, in causal order. What a fresh post is given.
    pub fn records(&self) -> impl Iterator<Item = &Record> {
        self.ordered().into_iter()
    }

    /// What a peer is told this post has. A handful of hashes, whatever
    /// the journal's size — which is the reason the exchange scales.
    pub fn heads(&self) -> Vec<Hash> {
        self.heads.iter().copied().collect()
    }

    /// **What a peer can place this post by**: the heads, and a few of
    /// their ancestors at doubling distances — one back, two, four,
    /// eight…
    ///
    /// Heads alone are not enough the moment both posts have written
    /// since they last met: each post's heads are records the other has
    /// never seen, so the answering post cannot walk down from them, finds
    /// no common ground, and sends its whole journal — three thousand
    /// records to a phone that held all but one. An ancestor one step
    /// back is almost always one the other post holds, and everything
    /// under it is then known to be shared; the doubling distances find
    /// the common ground after longer separations too, at a cost of a
    /// few hashes per head rather than one per record.
    pub fn frontier(&self) -> Vec<Hash> {
        let mut out: Vec<Hash> = self.heads();
        for head in self.heads() {
            let mut at = head;
            let mut step = 0usize;
            let mut next_mark = 1usize;
            while let Some(parent) = self
                .records
                .get(&at)
                .and_then(|r| r.parents().first().copied())
            {
                step += 1;
                at = parent;
                if step == next_mark {
                    if !out.contains(&at) {
                        out.push(at);
                    }
                    next_mark *= 2;
                }
            }
        }
        out
    }

    /// Parents named by records we hold, that we do not hold.
    ///
    /// This is the want list: a post asks for these, gets them, and asks
    /// again, until nothing is named that is not held. It converges
    /// because the graph is acyclic — a record's name is a hash of its
    /// parents' names, so a cycle would need a hash to contain itself.
    pub fn missing(&self) -> Vec<Hash> {
        self.named
            .iter()
            .filter(|h| !self.records.contains_key(h))
            .copied()
            .collect()
    }

    /// Adds a record. Answers whether it was new.
    ///
    /// Takes a [`Record`], which cannot be built unchecked from outside
    /// its module: by the time one is here, its name matches its content
    /// and its author signed it. This function therefore has one job and
    /// does not need to be trusted with a second.
    pub fn insert(&mut self, record: Record) -> bool {
        let id = record.id();
        if self.records.contains_key(&id) {
            return false;
        }
        if self.trusts(&record.author()) {
            self.high = self.high.max(record.lamport());
        }
        for parent in record.parents() {
            self.named.insert(*parent);
            self.heads.remove(parent);
        }
        if !self.named.contains(&id) {
            self.heads.insert(id);
        }
        self.records.insert(id, record);
        true
    }

    /// **Only these authors set the clock.** A record's rank comes off the
    /// record, and the record comes off whoever wrote it: one written at
    /// `u64::MAX` by somebody this post never admitted — relayed by a
    /// peer that did admit them — would have pinned every later write of
    /// this post at the ceiling, and records at one rank fall back to an
    /// order by hash, that is to no order at all. Records from others
    /// are still held and relayed; they no longer decide what « after »
    /// means here.
    pub fn trust(&mut self, authors: impl IntoIterator<Item = DeviceId>) {
        self.trusted = Some(authors.into_iter().collect());
        self.high = self
            .records
            .values()
            .filter(|r| {
                self.trusted
                    .as_ref()
                    .is_none_or(|t| t.contains(&r.author()))
            })
            .map(Record::lamport)
            .max()
            .unwrap_or(0);
    }

    fn trusts(&self, author: &DeviceId) -> bool {
        self.trusted.as_ref().is_none_or(|t| t.contains(author))
    }

    /// Writes a fact this post has to say.
    ///
    /// `corrects` names the record being corrected, and nothing else:
    /// there is no way to write a record that *removes* one.
    pub fn write(
        &mut self,
        device: &Device,
        trousseau: &Trousseau,
        stream: Stream,
        payload: &[u8],
        corrects: Option<Hash>,
        entropy: &mut dyn Entropy,
    ) -> Result<Hash> {
        let record = Record::seal(
            device,
            trousseau,
            stream,
            payload,
            corrects,
            self.write_parents(),
            // Saturating, and not for tidiness: the rank comes off
            // records a *peer* sent, so a peer that writes one at
            // `u64::MAX` chooses what this post's next `write` does.
            // Overflowing there panics, and a panic is worse than a
            // wrong order — the application goes down at the counter.
            // Saturated, the order degrades and the register still
            // takes its line.
            self.high.saturating_add(1),
            entropy,
        )?;
        let id = record.id();
        self.insert(record);
        Ok(id)
    }

    /// What a new record names as its parents: the heads.
    ///
    /// Above [`MAX_PARENTS`] it keeps the most recent, and that is a
    /// deliberate direction. A parent link is what proves one record
    /// came after another; dropping some makes two records look
    /// concurrent that were not, and *concurrent* is what this module
    /// reports as a divergence. So the loss errs towards showing a
    /// conflict that a human will dismiss, never towards hiding one.
    fn write_parents(&self) -> Vec<Hash> {
        if self.heads.len() <= MAX_PARENTS {
            return self.heads();
        }
        let mut heads: Vec<Hash> = self.heads();
        heads.sort_by_key(|h| {
            let rank = self.records.get(h).map(|r| r.lamport()).unwrap_or(0);
            (std::cmp::Reverse(rank), *h)
        });
        heads.truncate(MAX_PARENTS);
        heads.sort();
        heads
    }

    /// Everything a peer asked for that it has not already got.
    ///
    /// `need` is what the peer named; `have` is what it holds. The
    /// answer is everything at or below `need` that is not at or below
    /// `have` — which is why a whole first sync takes **one** round
    /// instead of one per generation. A peer that asked for a head and
    /// holds nothing gets the branch; one that asked for the same head
    /// and holds its parent gets one record.
    ///
    /// Capped at `most`, in causal order, so that a large journal
    /// crosses in bounded pieces rather than in one frame nobody can
    /// buffer. What is left over is asked for again in the next round:
    /// the cap costs a round trip and never a record.
    pub fn since(&self, need: &[Hash], have: &[Hash], most: usize) -> Vec<&Record> {
        let mut theirs = BTreeSet::new();
        let mut walk: Vec<Hash> = have.to_vec();
        while let Some(h) = walk.pop() {
            if !theirs.insert(h) {
                continue;
            }
            if let Some(record) = self.records.get(&h) {
                walk.extend(record.parents().iter().copied());
            }
        }

        let mut found = Vec::new();
        let mut seen = BTreeSet::new();
        let mut walk: Vec<Hash> = need.to_vec();
        while let Some(h) = walk.pop() {
            if theirs.contains(&h) || !seen.insert(h) {
                continue;
            }
            if let Some(record) = self.records.get(&h) {
                found.push(record);
                walk.extend(record.parents().iter().copied());
            }
        }
        found.sort_by_key(|r| (r.lamport(), r.author(), r.id()));
        found.truncate(most);
        found
    }

    /// Everything in the one order every post agrees on.
    ///
    /// Lamport first, then the author's name, then the record's own.
    /// The last two are not tie-breakers for elegance: without a *total*
    /// order two posts holding the same records would list them
    /// differently, and two screens showing one register in two orders
    /// is the kind of disagreement nobody can act on.
    fn ordered(&self) -> Vec<&Record> {
        let mut all: Vec<&Record> = self.records.values().collect();
        all.sort_by_key(|r| (r.lamport(), r.author(), r.id()));
        all
    }

    /// What a stream currently says, under the officine's key.
    pub fn read(&self, trousseau: &Trousseau, stream: Stream) -> Reading {
        self.read_with(std::slice::from_ref(trousseau), stream)
    }

    /// The same, under an officine that has changed its key.
    ///
    /// Re-keying is the only answer to a trousseau that walked out on
    /// somebody's laptop, and it has to be an answer that keeps the
    /// history: an officine does not throw away four years of register
    /// because a machine was stolen. Re-sealing every record would
    /// **rename** every record — the name is a hash of the ciphertext —
    /// and a renamed journal is a new journal, so that is not the way.
    ///
    /// The way is this: the new trousseau seals what is written from now
    /// on, the old ones stay in the base, and a reading tries them in
    /// order. Nothing on the wire changes, no record moves, and there is
    /// no epoch marker in the header to get wrong — the seal itself
    /// answers, because an AEAD under the wrong key does not open.
    ///
    /// Newest first, so the ordinary record costs one attempt. A record
    /// no key in the ring opens is still **named** rather than skipped.
    pub fn read_with(&self, keyring: &[Trousseau], stream: Stream) -> Reading {
        let of_stream: Vec<&Record> = self
            .ordered()
            .into_iter()
            .filter(|r| r.stream() == stream)
            .collect();

        // Corrected by something we hold — and a record only counts as
        // correcting while it is itself current, which falls out of
        // walking the corrections rather than assuming a chain of one.
        let corrected: BTreeSet<Hash> = of_stream.iter().filter_map(|r| r.corrects()).collect();

        // How many current records correct each target, so a rival is
        // found without comparing every pair.
        let mut rivals_of: BTreeMap<Hash, Vec<Hash>> = BTreeMap::new();
        for r in of_stream.iter().filter(|r| !corrected.contains(&r.id())) {
            if let Some(target) = r.corrects() {
                rivals_of.entry(target).or_default().push(r.id());
            }
        }

        let mut reading = Reading::default();
        for record in of_stream {
            if corrected.contains(&record.id()) {
                continue;
            }
            let payload = match keyring.iter().find_map(|key| record.open(key).ok()) {
                Some(bytes) => bytes,
                None => {
                    reading.unopened.push(record.id());
                    continue;
                }
            };
            let rivals = record
                .corrects()
                .and_then(|target| rivals_of.get(&target))
                .map(|all| all.iter().copied().filter(|h| *h != record.id()).collect())
                .unwrap_or_default();
            reading.facts.push(Fact {
                id: record.id(),
                author: record.author(),
                lamport: record.lamport(),
                corrects: record.corrects(),
                payload,
                rivals,
            });
        }
        reading
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::Counted;

    struct Post {
        device: Device,
        journal: Journal,
    }

    impl Post {
        fn new(seed: u8) -> Self {
            let mut e = Counted(seed);
            Self {
                device: Device::generate(&mut e),
                journal: Journal::new(),
            }
        }

        fn write(&mut self, t: &Trousseau, payload: &[u8], corrects: Option<Hash>) -> Hash {
            let mut e = crate::OsEntropy;
            self.journal
                .write(&self.device, t, Stream::Registre, payload, corrects, &mut e)
                .unwrap()
        }
    }

    fn officine() -> Trousseau {
        let mut e = Counted(3);
        Trousseau::generate(&mut e)
    }

    /// Everything one post has, given to another, and the two then say
    /// the same thing — in the same order, which is the harder half.
    fn pour(from: &Journal, into: &mut Journal) {
        for record in from.records() {
            into.insert(record.clone());
        }
    }

    #[test]
    fn what_is_written_is_read_back_in_order() {
        let t = officine();
        let mut post = Post::new(1);
        for line in ["premiere", "deuxieme", "troisieme"] {
            post.write(&t, line.as_bytes(), None);
        }
        let reading = post.journal.read(&t, Stream::Registre);
        let text: Vec<&[u8]> = reading.facts.iter().map(|f| f.payload.as_slice()).collect();
        assert_eq!(text, vec![&b"premiere"[..], b"deuxieme", b"troisieme"]);
        assert!(reading.unopened.is_empty());
        // And a stream nothing was written to says nothing.
        assert!(post.journal.read(&t, Stream::Caisse).facts.is_empty());
    }

    /// The rule the register already lives by, generalised: a
    /// correction is another record, and what it corrects stays.
    #[test]
    fn a_correction_never_removes_what_it_corrects() {
        let t = officine();
        let mut post = Post::new(1);
        let wrong = post.write(&t, b"12 comprimes", None);
        let right = post.write(&t, b"annulation : 21 comprimes", Some(wrong));

        // Two records in the journal, one fact on the screen.
        assert_eq!(post.journal.len(), 2);
        assert!(post.journal.has(&wrong), "la ligne fausse reste écrite");
        let reading = post.journal.read(&t, Stream::Registre);
        assert_eq!(reading.facts.len(), 1);
        assert_eq!(reading.facts[0].id, right);
        assert_eq!(reading.facts[0].corrects, Some(wrong));
        assert!(reading.facts[0].rivals.is_empty());

        // And a correction of the correction leaves both behind it.
        let last = post.write(&t, b"annulation : 20 comprimes", Some(right));
        let reading = post.journal.read(&t, Stream::Registre);
        assert_eq!(reading.facts.len(), 1);
        assert_eq!(reading.facts[0].id, last);
        assert_eq!(post.journal.len(), 3);
    }

    /// Two posts correcting one record without seeing each other. Both
    /// corrections stand, each names the other, and nothing here picks.
    #[test]
    fn two_corrections_of_one_record_are_a_divergence_and_not_a_winner() {
        let t = officine();
        let (mut a, mut b) = (Post::new(1), Post::new(2));
        let line = a.write(&t, b"12 comprimes", None);
        pour(&a.journal, &mut b.journal);

        // Both correct it, neither has seen the other.
        let fix_a = a.write(&t, b"annulation : 21", Some(line));
        let fix_b = b.write(&t, b"annulation : 20", Some(line));

        pour(&a.journal, &mut b.journal);
        pour(&b.journal, &mut a.journal);

        for post in [&a, &b] {
            let reading = post.journal.read(&t, Stream::Registre);
            assert_eq!(reading.facts.len(), 2, "les deux corrections restent");
            let named: Vec<Hash> = reading.facts.iter().map(|f| f.id).collect();
            assert!(named.contains(&fix_a) && named.contains(&fix_b));
            for fact in &reading.facts {
                assert_eq!(fact.rivals.len(), 1, "chacune nomme l'autre");
                assert_ne!(fact.rivals[0], fact.id);
                assert!(named.contains(&fact.rivals[0]));
            }
        }
    }

    /// Merging is a union, and the union is the same both ways round.
    /// Two posts that have exchanged everything hold the same journal
    /// and read it in the same order — the property the whole design is
    /// for, and the one nobody notices is broken until two screens
    /// disagree at the counter.
    #[test]
    fn two_posts_that_have_exchanged_everything_say_the_same_thing() {
        let t = officine();
        let (mut a, mut b) = (Post::new(1), Post::new(2));
        a.write(&t, b"a-1", None);
        b.write(&t, b"b-1", None);
        pour(&a.journal, &mut b.journal);
        a.write(&t, b"a-2", None);
        b.write(&t, b"b-2", None);
        pour(&b.journal, &mut a.journal);
        pour(&a.journal, &mut b.journal);

        assert_eq!(a.journal.len(), 4);
        assert_eq!(b.journal.len(), 4);
        let read = |p: &Post| -> Vec<Vec<u8>> {
            p.journal
                .read(&t, Stream::Registre)
                .facts
                .into_iter()
                .map(|f| f.payload)
                .collect()
        };
        assert_eq!(read(&a), read(&b));
        assert_eq!(a.journal.heads(), b.journal.heads());
    }

    /// A rank a peer chose does not take this post down.
    ///
    /// The Lamport counter is read off records that arrive, so the
    /// largest one is the peer's to pick. At `u64::MAX` the next write
    /// used to overflow, which panics — at the counter, in the middle of
    /// a dispensing. It saturates: the order stops being informative and
    /// the line still gets written, which is the right way round.
    #[test]
    fn a_rank_a_peer_chose_does_not_take_this_post_down() {
        let t = officine();
        let mut far = Post::new(1);
        // A record at the top of the range, reached honestly: seal one
        // by hand at `u64::MAX` and hand it over the way a peer would.
        let mut e = crate::OsEntropy;
        let extreme = Record::seal(
            &far.device,
            &t,
            Stream::Registre,
            b"depuis un poste au bout du compte",
            None,
            vec![],
            u64::MAX,
            &mut e,
        )
        .unwrap();

        let mut post = Post::new(2);
        post.journal.insert(extreme);
        let written = post.write(&t, b"la ligne d'apres", None);
        assert_eq!(post.journal.get(&written).unwrap().lamport(), u64::MAX);
        assert_eq!(post.journal.read(&t, Stream::Registre).facts.len(), 2);
        let _ = far.write(&t, b"x", None);
    }

    /// A record given twice is one record. Peers re-offer what they
    /// hold at every meeting, so this is the ordinary case and not an
    /// edge one.
    #[test]
    fn a_record_given_twice_is_one_record() {
        let t = officine();
        let mut a = Post::new(1);
        a.write(&t, b"une ligne", None);
        let record = a.journal.records().next().unwrap().clone();
        let mut b = Journal::new();
        assert!(b.insert(record.clone()));
        assert!(!b.insert(record), "déjà là");
        assert_eq!(b.len(), 1);
    }

    /// The order is the journal's, never a clock's. Two records written
    /// at the same rank by two posts fall in the same order on both, and
    /// nothing about the machine's date enters into it.
    #[test]
    fn nobodys_clock_decides_the_order() {
        let t = officine();
        let (mut a, mut b) = (Post::new(1), Post::new(2));
        a.write(&t, b"depuis a", None);
        b.write(&t, b"depuis b", None);
        // Same rank, concurrent: the author's name settles it, and it
        // settles it the same way on both posts.
        pour(&a.journal, &mut b.journal);
        pour(&b.journal, &mut a.journal);
        let order = |p: &Post| -> Vec<u64> {
            p.journal
                .read(&t, Stream::Registre)
                .facts
                .iter()
                .map(|f| f.lamport)
                .collect()
        };
        assert_eq!(order(&a), vec![1, 1]);
        assert_eq!(order(&a), order(&b));
        // And a record written after seeing both outranks them.
        let after = a.write(&t, b"apres les deux", None);
        assert_eq!(a.journal.get(&after).unwrap().lamport(), 2);
    }

    /// What a peer has to be asked for, and the fact that asking ends.
    #[test]
    fn a_post_can_name_what_it_is_missing() {
        let t = officine();
        let mut a = Post::new(1);
        for line in ["un", "deux", "trois"] {
            a.write(&t, line.as_bytes(), None);
        }
        let all: Vec<Record> = a.journal.records().cloned().collect();

        // The last record arrives first: its parent is named and
        // missing, which is exactly what the want loop asks for.
        let mut b = Journal::new();
        b.insert(all[2].clone());
        assert_eq!(b.missing(), vec![all[1].id()]);
        b.insert(all[1].clone());
        assert_eq!(b.missing(), vec![all[0].id()]);
        b.insert(all[0].clone());
        assert!(b.missing().is_empty(), "l'échange se termine");
        assert_eq!(b.heads(), a.journal.heads());
    }

    /// What a peer is owed, computed from what it asked for and what it
    /// says it has — in **one** pass, which is what makes a first sync
    /// one round trip instead of one per generation of the graph.
    #[test]
    fn a_peer_is_owed_what_it_asked_for_minus_what_it_holds() {
        let t = officine();
        let mut post = Post::new(1);
        let mut chain = Vec::new();
        for line in ["un", "deux", "trois", "quatre"] {
            chain.push(post.write(&t, line.as_bytes(), None));
        }
        let head = *chain.last().unwrap();

        // A peer holding nothing, asking for the head: it is owed the
        // whole chain, oldest first, so its own parents arrive before
        // their children.
        let owed = post.journal.since(&[head], &[], 100);
        assert_eq!(owed.len(), 4);
        assert_eq!(owed[0].id(), chain[0]);
        assert_eq!(owed[3].id(), head);

        // The same peer, holding the second record: it is owed two.
        let owed = post.journal.since(&[head], &[chain[1]], 100);
        assert_eq!(owed.len(), 2);
        assert_eq!(owed[0].id(), chain[2]);

        // Up to date: it is owed nothing, which is the ordinary evening.
        assert!(post.journal.since(&[head], &[head], 100).is_empty());

        // The cap cuts the far end and never the near one: what is left
        // out is the newest, so what arrives is always a run of history
        // with no hole in it.
        let owed = post.journal.since(&[head], &[], 2);
        assert_eq!(owed.len(), 2);
        assert_eq!(owed[0].id(), chain[0]);

        // And a hash nobody here has is simply not owed — asking for
        // something this post does not hold is not an error.
        assert!(post.journal.since(&[Hash([0; 32])], &[], 100).is_empty());
    }

    /// A record the trousseau in hand will not open is named, not
    /// skipped: a list quietly one line short reads like a complete one.
    #[test]
    fn a_record_that_will_not_open_is_named_and_not_dropped() {
        let t = officine();
        let mut post = Post::new(1);
        post.write(&t, b"lisible", None);
        let mut e = Counted(90);
        let other_officine = Trousseau::generate(&mut e);

        let reading = post.journal.read(&other_officine, Stream::Registre);
        assert!(reading.facts.is_empty());
        assert_eq!(reading.unopened.len(), 1, "le trou est nommé");
        // ...and with the right key there is no hole at all.
        let reading = post.journal.read(&t, Stream::Registre);
        assert_eq!(reading.facts.len(), 1);
        assert!(reading.unopened.is_empty());
    }

    /// An officine that has changed its key still reads what it wrote
    /// before it changed it.
    ///
    /// The whole point of the keyring: re-sealing would rename every
    /// record, so the old key stays and a reading tries it. And the
    /// reading under the new key alone still **names** what it cannot
    /// open — an officine that has lost a key learns it, rather than
    /// finding its register four lines short.
    #[test]
    fn an_officine_that_changed_its_key_still_reads_its_own_past() {
        let before = officine();
        let mut e = Counted(150);
        let after = Trousseau::generate(&mut e);

        let mut post = Post::new(1);
        post.write(&before, b"ecrit avant la re-cle", None);
        // The laptop walked out; the officine re-keys and goes on
        // writing. Nothing already written moves.
        let mut real = crate::OsEntropy;
        post.journal
            .write(
                &post.device,
                &after,
                Stream::Registre,
                b"ecrit apres la re-cle",
                None,
                &mut real,
            )
            .unwrap();

        // The ring, newest first: the whole register reads.
        let reading = post
            .journal
            .read_with(&[after.clone(), before.clone()], Stream::Registre);
        assert_eq!(reading.facts.len(), 2);
        assert!(reading.unopened.is_empty());
        assert_eq!(reading.facts[0].payload, b"ecrit avant la re-cle");
        assert_eq!(reading.facts[1].payload, b"ecrit apres la re-cle");

        // The new key alone: half the register, and the other half
        // named rather than quietly missing.
        let reading = post.journal.read(&after, Stream::Registre);
        assert_eq!(reading.facts.len(), 1);
        assert_eq!(reading.unopened.len(), 1);
        // And the old key alone, symmetrically.
        let reading = post.journal.read(&before, Stream::Registre);
        assert_eq!(reading.facts.len(), 1);
        assert_eq!(reading.unopened.len(), 1);
        // An empty ring opens nothing and hides nothing.
        let reading = post.journal.read_with(&[], Stream::Registre);
        assert!(reading.facts.is_empty());
        assert_eq!(reading.unopened.len(), 2);
    }

    /// Streams do not bleed into one another: a register reading shows
    /// register records and nothing else, whatever else the journal
    /// holds.
    #[test]
    fn a_stream_is_read_alone() {
        let t = officine();
        let mut post = Post::new(1);
        let mut e = crate::OsEntropy;
        for stream in Stream::ALL {
            post.journal
                .write(&post.device, &t, stream, b"x", None, &mut e)
                .unwrap();
        }
        for stream in Stream::ALL {
            assert_eq!(post.journal.read(&t, stream).facts.len(), 1, "{stream:?}");
        }
        assert_eq!(post.journal.len(), Stream::ALL.len());
    }

    /// There is no way to make this post forget. The rule is held by the
    /// module's own text, the way the register's is in `db.rs`: a `fn`
    /// that removes a record could be written and every other test would
    /// still pass.
    #[test]
    fn a_journal_can_only_ever_be_added_to() {
        let text = include_str!("journal.rs");
        let code = text.split("mod tests").next().unwrap();
        for verb in [
            "fn remove",
            "fn delete",
            "fn clear",
            "fn prune",
            "self.records.remove",
            "self.records.retain",
        ] {
            assert!(
                !code.contains(verb),
                "un journal ne se vide pas : « {verb} »"
            );
        }
        // `heads` is a set that loses members — that is bookkeeping, not
        // forgetting — so the one place records live is checked apart:
        // the only call on it that changes anything is `insert`.
        for verb in [
            "remove",
            "retain",
            "clear",
            "pop",
            "take",
            "drain",
            "get_mut",
            "values_mut",
            "entry",
            "split_off",
            "append",
        ] {
            assert!(
                !code.contains(&format!("self.records.{verb}")),
                "les enregistrements ne se perdent pas : « {verb} »"
            );
        }
    }

    /// **A rank from somebody untrusted does not move this post's clock**:
    /// a record at `u64::MAX` from a stranger is held, and the next write
    /// still ranks just above what the trusted authors wrote.
    #[test]
    fn an_untrusted_rank_does_not_move_the_clock() {
        let mut e = Counted(61);
        let (me, stranger) = (Device::generate(&mut e), Device::generate(&mut e));
        let t = Trousseau::generate(&mut e);
        let mut theirs = Journal::new();
        theirs.high = u64::MAX - 1;
        theirs
            .write(&stranger, &t, Stream::Reseau, b"x", None, &mut e)
            .unwrap();
        let mut j = Journal::new();
        j.write(&me, &t, Stream::Reseau, b"a", None, &mut e)
            .unwrap();
        j.trust([me.id()]);
        for r in theirs.records() {
            j.insert(r.clone());
        }
        assert_eq!(j.len(), 2, "gardé");
        let id = j
            .write(&me, &t, Stream::Reseau, b"b", None, &mut e)
            .unwrap();
        assert_eq!(j.get(&id).unwrap().lamport(), 2);
        // Trusted, the same record would have pinned the clock.
        j.trust([me.id(), stranger.id()]);
        let id = j
            .write(&me, &t, Stream::Reseau, b"c", None, &mut e)
            .unwrap();
        assert_eq!(j.get(&id).unwrap().lamport(), u64::MAX);
    }
}
