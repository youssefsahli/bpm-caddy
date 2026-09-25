//! The one place bytes move.
//!
//! Everything else in this crate is a function of its arguments. Here is
//! the socket, and it is deliberately the thinnest thing that can be
//! called one: a [`Link`] carries opaque blobs, in order, and says when
//! it breaks. It does not know what a frame means, it never looks
//! inside, and it holds no key — so the day the officine wants to sync
//! over something else (a serial cable between two counters, a file on a
//! memory stick carried across the road), that is one implementation of
//! two methods and not a change to the protocol.
//!
//! Three rules hold here.
//!
//! **A message keeps its edges.** TCP is a stream and knows nothing of
//! messages: read it naively and two frames arrive glued together, or
//! one arrives in halves. The length prefix is what stops that, and
//! every read is bounded by [`crate::wire::MAX_WIRE`] — a peer that
//! announces a gigabyte is a peer choosing how much memory this post
//! allocates.
//!
//! **A link that hangs is not a link that waits.** Timeouts are set on
//! the socket itself, so a post that goes silent mid-conversation costs
//! a few seconds and not the evening.
//!
//! **Nothing here decides anything.** No retry policy, no reconnection,
//! no schedule: the caller drives, the caller stops. A module that
//! reconnected on its own would be a module that reaches the network
//! when nobody asked it to, which is exactly what this application does
//! not do.

use crate::{Error, Result};

/// An ordered, reliable channel of opaque messages.
pub trait Link {
    /// Hands one message to the peer, whole.
    fn send(&mut self, message: &[u8]) -> Result<()>;
    /// Waits for the next message. `Err(Error::Link)` when the other end
    /// has gone.
    fn recv(&mut self) -> Result<Vec<u8>>;
}

/// Two ends of a link, in memory. What the tests talk over, and what an
/// officine syncing two processes on one machine could talk over.
///
/// It is in the ordinary build and not behind `#[cfg(test)]` on purpose:
/// a caller wanting to try the whole protocol — in a test of its own, in
/// a demo, on a bench — should not have to open a socket to do it.
pub mod loopback {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    type Queue = Arc<Mutex<VecDeque<Vec<u8>>>>;

    /// One end.
    pub struct End {
        mine: Queue,
        theirs: Queue,
    }

    /// Both ends of one link.
    pub fn pair() -> (End, End) {
        let (a, b): (Queue, Queue) = Default::default();
        (
            End {
                mine: a.clone(),
                theirs: b.clone(),
            },
            End { mine: b, theirs: a },
        )
    }

    impl Link for End {
        fn send(&mut self, message: &[u8]) -> Result<()> {
            self.theirs
                .lock()
                .map_err(|_| Error::Link)?
                .push_back(message.to_vec());
            Ok(())
        }

        fn recv(&mut self) -> Result<Vec<u8>> {
            self.mine
                .lock()
                .map_err(|_| Error::Link)?
                .pop_front()
                .ok_or(Error::Link)
        }
    }
}

#[cfg(feature = "tcp")]
pub use tcp::{dial, Door, TcpLink};

#[cfg(feature = "tcp")]
mod tcp {
    use super::*;
    use crate::wire::MAX_WIRE;
    use std::io::{Read, Write};
    use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
    use std::time::{Duration, Instant};

    /// A [`Link`] over TCP. The only socket in this crate, and it uses
    /// nothing but `std::net` — no runtime, no TLS stack, no
    /// certificate. The encryption is the handshake's, end to end: a
    /// transport layer that also encrypted would be a second thing to
    /// get right, and the one the records rely on would be the one
    /// nobody checked.
    pub struct TcpLink {
        stream: TcpStream,
        /// When the whole conversation must be over, if it has a bound.
        deadline: Option<Instant>,
    }

    impl TcpLink {
        /// Wraps a connected socket, with deadlines on both directions.
        pub fn new(stream: TcpStream, patience: Duration) -> Result<Self> {
            stream
                .set_read_timeout(Some(patience))
                .map_err(|_| Error::Link)?;
            stream
                .set_write_timeout(Some(patience))
                .map_err(|_| Error::Link)?;
            // Frames are small and answers matter more than throughput:
            // waiting for a full segment adds a round trip to every
            // round of the exchange.
            stream.set_nodelay(true).map_err(|_| Error::Link)?;

            Ok(Self {
                stream,
                deadline: None,
            })
        }

        /// **A bound on the whole conversation**, not only on each read.
        /// The per-read deadline is a socket option, and a caller that
        /// trickles one byte every few seconds satisfies it for ever: a
        /// door answering whoever knocks needs the whole exchange to end
        /// by a time it chose.
        pub fn with_deadline(mut self, total: Duration) -> Self {
            self.deadline = Some(Instant::now() + total);
            self
        }

        /// Past the conversation's bound?
        fn late(&self) -> bool {
            self.deadline.is_some_and(|d| Instant::now() >= d)
        }

        /// `read_exact`, looking at the bound between reads.
        fn read_whole(&mut self, buf: &mut [u8]) -> Result<()> {
            let mut at = 0;
            while at < buf.len() {
                if self.late() {
                    return Err(Error::Link);
                }
                match self.stream.read(&mut buf[at..]) {
                    Ok(0) => return Err(Error::Link),
                    Ok(n) => at += n,
                    Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                    Err(_) => return Err(Error::Link),
                }
            }
            Ok(())
        }
    }

    /// Knocks on another post's door.
    ///
    /// `address` is what that post's [`Door`] answered — « 192.168.1.14:7742 ».
    /// A name is resolved the way the operating system resolves one; an
    /// officine that types an address types an address.
    pub fn dial(address: &str, patience: Duration) -> Result<TcpLink> {
        // `connect_timeout` rather than `connect`, and that needs a
        // resolved address: a machine that is off answers nothing at
        // all, and the system's own connect gives up after a minute
        // and a half. Nobody at a counter waits ninety seconds to find
        // out the other post is unplugged.
        let mut last = Err(Error::Link);
        for resolved in address.to_socket_addrs().map_err(|_| Error::Link)? {
            match TcpStream::connect_timeout(&resolved, patience) {
                Ok(stream) => return TcpLink::new(stream, patience),
                Err(_) => last = Err(Error::Link),
            }
        }
        last
    }

    /// A door this post leaves open for one other post.
    ///
    /// It is a bound listener and nothing more: it does not loop, does
    /// not spawn, does not remember who called. One [`Door::accept`],
    /// one conversation — the caller decides whether there is another,
    /// which is the rule this module states and does not get to soften.
    pub struct Door(TcpListener);

    impl Door {
        /// Opens on `address`. « 0.0.0.0:0 » takes any free port and
        /// [`Door::address`] then says which, which is what a screen
        /// shows the other post's operator.
        pub fn open(address: &str) -> Result<Self> {
            TcpListener::bind(address)
                .map(Door)
                .map_err(|_| Error::Link)
        }

        /// Where this door is, to be read out or copied across.
        pub fn address(&self) -> Result<SocketAddr> {
            self.0.local_addr().map_err(|_| Error::Link)
        }

        /// Waits for one post, and gives up rather than waiting for
        /// ever.
        ///
        /// `std` has no accept with a deadline — `set_read_timeout` is a
        /// stream's, not a listener's — so this polls. That is the ugly
        /// part, and it is here rather than in the caller for the reason
        /// the rest of this module exists: a door that blocks with no
        /// way out is a thread an officine cannot get back, and the
        /// application would grow its own poll loop instead. Twenty-five
        /// milliseconds between looks is imperceptible to whoever is
        /// dialling and costs nothing to the machine waiting.
        pub fn accept(&self, patience: Duration) -> Result<TcpLink> {
            self.accept_with(patience, patience)
        }

        /// [`Door::accept`], waiting `wait` for a knock but giving the
        /// conversation `talk` for each read and write: a door looked at
        /// briefly, between other work, must not hand over a link that
        /// gives up on a peer a few hundred milliseconds away.
        pub fn accept_with(&self, wait: Duration, talk: Duration) -> Result<TcpLink> {
            let patience = talk;
            self.0.set_nonblocking(true).map_err(|_| Error::Link)?;
            let deadline = Instant::now() + wait;
            loop {
                match self.0.accept() {
                    Ok((stream, _)) => {
                        // Back to blocking: everything downstream reads
                        // and writes as though nothing here happened.
                        stream.set_nonblocking(false).map_err(|_| Error::Link)?;
                        let _ = self.0.set_nonblocking(false);
                        return TcpLink::new(stream, patience);
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        if Instant::now() >= deadline {
                            let _ = self.0.set_nonblocking(false);
                            return Err(Error::Link);
                        }
                        std::thread::sleep(Duration::from_millis(25));
                    }
                    Err(_) => {
                        let _ = self.0.set_nonblocking(false);
                        return Err(Error::Link);
                    }
                }
            }
        }
    }

    impl Link for TcpLink {
        fn send(&mut self, message: &[u8]) -> Result<()> {
            if message.len() > MAX_WIRE {
                return Err(Error::TooLarge);
            }
            if self.late() {
                return Err(Error::Link);
            }
            let header = (message.len() as u32).to_be_bytes();
            self.stream.write_all(&header).map_err(|_| Error::Link)?;
            // In pieces, the bound looked at between them: a peer that
            // reads slowly holds each write for a timeout, and the bound
            // is what ends the conversation all the same.
            for piece in message.chunks(4096) {
                if self.late() {
                    return Err(Error::Link);
                }
                self.stream.write_all(piece).map_err(|_| Error::Link)?;
            }
            self.stream.flush().map_err(|_| Error::Link)
        }

        fn recv(&mut self) -> Result<Vec<u8>> {
            let mut header = [0u8; 4];
            self.read_whole(&mut header)?;
            let len = u32::from_be_bytes(header) as usize;
            // Read **before** allocating: this is the whole reason the
            // length is checked here and not after the buffer exists.
            if len > MAX_WIRE {
                return Err(Error::TooLarge);
            }
            let mut message = vec![0u8; len];
            self.read_whole(&mut message)?;
            Ok(message)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_message_crosses_whole_and_in_order() {
        let (mut a, mut b) = loopback::pair();
        a.send(b"premier").unwrap();
        a.send(b"second").unwrap();
        assert_eq!(b.recv().unwrap(), b"premier");
        assert_eq!(b.recv().unwrap(), b"second");
        // Nothing left: an empty link says so rather than blocking.
        assert_eq!(b.recv(), Err(Error::Link));
        // And it is two directions, not one.
        b.send(b"retour").unwrap();
        assert_eq!(a.recv().unwrap(), b"retour");
    }

    /// The trap TCP sets: a stream has no messages in it. Two frames
    /// sent back to back must not arrive as one, and a frame must not
    /// arrive in halves — which is what the length prefix is for, and
    /// what this checks against a real socket rather than a model of
    /// one. It goes through the door and the dial, so the two halves an
    /// officine actually calls are the two halves under test.
    #[cfg(feature = "tcp")]
    #[test]
    fn a_socket_gives_back_the_messages_that_were_put_in() {
        use std::time::Duration;

        let patience = Duration::from_secs(5);
        let door = Door::open("127.0.0.1:0").unwrap();
        let address = door.address().unwrap().to_string();
        let waiter = std::thread::spawn(move || door.accept(patience).unwrap());
        let mut caller = dial(&address, patience).unwrap();
        let mut answerer = waiter.join().unwrap();

        // Small, back to back: the case that glues frames together.
        caller.send(b"un").unwrap();
        caller.send(b"deux").unwrap();
        // And one far past a segment: the case that splits them.
        let long = vec![0xABu8; crate::wire::MAX_WIRE];
        caller.send(&long).unwrap();

        assert_eq!(answerer.recv().unwrap(), b"un");
        assert_eq!(answerer.recv().unwrap(), b"deux");
        assert_eq!(answerer.recv().unwrap(), long);

        answerer.send(b"recu").unwrap();
        assert_eq!(caller.recv().unwrap(), b"recu");

        // The other end goes: the link says so, it does not hang.
        drop(answerer);
        assert_eq!(caller.recv(), Err(Error::Link));
    }

    /// A peer announcing more than this post will read gets a refusal,
    /// and the refusal happens before anything is allocated for it.
    #[cfg(feature = "tcp")]
    #[test]
    fn a_socket_refuses_a_frame_larger_than_this_post_will_read() {
        use crate::wire::MAX_WIRE;
        use std::io::Write;
        use std::net::TcpStream;
        use std::time::Duration;

        let patience = Duration::from_secs(5);
        let door = Door::open("127.0.0.1:0").unwrap();
        let address = door.address().unwrap();
        let waiter = std::thread::spawn(move || door.accept(patience).unwrap().recv());
        let mut rude = TcpStream::connect(address).unwrap();
        rude.write_all(&u32::MAX.to_be_bytes()).unwrap();
        rude.flush().unwrap();
        assert_eq!(waiter.join().unwrap(), Err(Error::TooLarge));

        // ...and this post does not send one either.
        let door = Door::open("127.0.0.1:0").unwrap();
        let address = door.address().unwrap().to_string();
        let waiter = std::thread::spawn(move || door.accept(patience).unwrap());
        let mut caller = dial(&address, patience).unwrap();
        let _held = waiter.join().unwrap();
        assert_eq!(caller.send(&vec![0u8; MAX_WIRE + 1]), Err(Error::TooLarge));
    }

    /// A door says where it is, because that is what somebody reads out
    /// to the other post — and « port zero » is how it gets a free one
    /// without an officine having to pick a number.
    #[cfg(feature = "tcp")]
    #[test]
    fn a_door_says_where_it_is() {
        let door = Door::open("127.0.0.1:0").unwrap();
        let address = door.address().unwrap();
        assert_ne!(address.port(), 0, "le port choisi est dit, pas « zéro »");
        assert!(address.to_string().starts_with("127.0.0.1:"));
        // A door where none can be opened is a refusal, not a panic.
        assert_eq!(Door::open("pas une adresse").err(), Some(Error::Link));
    }

    /// Nobody comes, and the thread comes back anyway.
    ///
    /// `std` has no accept with a deadline, so this is the one place in
    /// the crate that polls. The test measures the thing that matters:
    /// it **returns**, and it returns near the patience it was given
    /// rather than at the system's own idea of for ever.
    #[cfg(feature = "tcp")]
    #[test]
    fn a_door_nobody_comes_to_gives_up() {
        use std::time::{Duration, Instant};

        let door = Door::open("127.0.0.1:0").unwrap();
        let begun = Instant::now();
        assert_eq!(
            door.accept(Duration::from_millis(250)).err(),
            Some(Error::Link)
        );
        let waited = begun.elapsed();
        assert!(
            waited >= Duration::from_millis(200),
            "{waited:?} : rendu trop tôt"
        );
        assert!(
            waited < Duration::from_secs(5),
            "{waited:?} : n'a pas rendu la main"
        );
        // And the door still works afterwards: giving up is not closing.
        let address = door.address().unwrap().to_string();
        let patience = Duration::from_secs(5);
        let waiter = std::thread::spawn(move || door.accept(patience).unwrap());
        let mut caller = dial(&address, patience).unwrap();
        caller.send(b"enfin").unwrap();
        assert_eq!(waiter.join().unwrap().recv().unwrap(), b"enfin");
    }

    /// Knocking where there is nobody says so, and says so quickly.
    ///
    /// The system's own `connect` gives up after a minute and a half.
    /// Nobody at a counter waits ninety seconds to be told the other
    /// post is unplugged, which is the whole reason `dial` resolves the
    /// address itself and uses `connect_timeout`.
    #[cfg(feature = "tcp")]
    #[test]
    fn knocking_where_there_is_nobody_says_so() {
        use std::time::{Duration, Instant};

        // A port that was open a moment ago and is not any more: the
        // ordinary case of a post that has been switched off.
        let address = {
            let door = Door::open("127.0.0.1:0").unwrap();
            door.address().unwrap().to_string()
        };
        let begun = Instant::now();
        assert_eq!(
            dial(&address, Duration::from_secs(2)).err(),
            Some(Error::Link)
        );
        assert!(begun.elapsed() < Duration::from_secs(5));
        // And a name that is not an address at all.
        assert_eq!(
            dial("pas une adresse", Duration::from_secs(2)).err(),
            Some(Error::Link)
        );
    }

    /// **A caller that trickles is cut at the conversation's bound**: one
    /// byte a little faster than the per-read timeout would keep a door
    /// busy for ever; the whole-conversation deadline ends it.
    #[cfg(feature = "tcp")]
    #[test]
    fn a_trickling_caller_is_cut_at_the_conversation_bound() {
        use std::io::Write;
        use std::time::{Duration, Instant};
        let door = Door::open("127.0.0.1:0").unwrap();
        let address = door.address().unwrap().to_string();
        let trickle = std::thread::spawn(move || {
            let mut s = std::net::TcpStream::connect(address).unwrap();
            let _ = s.write_all(&[0, 0, 1, 0]);
            for _ in 0..40 {
                std::thread::sleep(Duration::from_millis(100));
                if s.write_all(&[1]).is_err() {
                    break;
                }
            }
        });
        let mut link = door
            .accept_with(Duration::from_secs(5), Duration::from_secs(2))
            .unwrap()
            .with_deadline(Duration::from_millis(700));
        let t = Instant::now();
        assert_eq!(link.recv(), Err(Error::Link));
        assert!(t.elapsed() < Duration::from_secs(2), "{:?}", t.elapsed());
        drop(link);
        trickle.join().unwrap();
    }
}
