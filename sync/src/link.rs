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
pub use tcp::TcpLink;

#[cfg(feature = "tcp")]
mod tcp {
    use super::*;
    use crate::wire::MAX_WIRE;
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::time::Duration;

    /// A [`Link`] over TCP. The only socket in this crate, and it uses
    /// nothing but `std::net` — no runtime, no TLS stack, no
    /// certificate. The encryption is the handshake's, end to end: a
    /// transport layer that also encrypted would be a second thing to
    /// get right, and the one the records rely on would be the one
    /// nobody checked.
    pub struct TcpLink {
        stream: TcpStream,
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
            Ok(Self { stream })
        }
    }

    impl Link for TcpLink {
        fn send(&mut self, message: &[u8]) -> Result<()> {
            if message.len() > MAX_WIRE {
                return Err(Error::TooLarge);
            }
            let header = (message.len() as u32).to_be_bytes();
            self.stream.write_all(&header).map_err(|_| Error::Link)?;
            self.stream.write_all(message).map_err(|_| Error::Link)?;
            self.stream.flush().map_err(|_| Error::Link)
        }

        fn recv(&mut self) -> Result<Vec<u8>> {
            let mut header = [0u8; 4];
            self.stream
                .read_exact(&mut header)
                .map_err(|_| Error::Link)?;
            let len = u32::from_be_bytes(header) as usize;
            // Read **before** allocating: this is the whole reason the
            // length is checked here and not after the buffer exists.
            if len > MAX_WIRE {
                return Err(Error::TooLarge);
            }
            let mut message = vec![0u8; len];
            self.stream
                .read_exact(&mut message)
                .map_err(|_| Error::Link)?;
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
    /// one.
    #[cfg(feature = "tcp")]
    #[test]
    fn a_socket_gives_back_the_messages_that_were_put_in() {
        use std::net::{TcpListener, TcpStream};
        use std::time::Duration;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let waiter = std::thread::spawn(move || {
            let (socket, _) = listener.accept().unwrap();
            TcpLink::new(socket, Duration::from_secs(5)).unwrap()
        });
        let mut caller =
            TcpLink::new(TcpStream::connect(address).unwrap(), Duration::from_secs(5)).unwrap();
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
        use std::net::{TcpListener, TcpStream};
        use std::time::Duration;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let waiter = std::thread::spawn(move || {
            let (socket, _) = listener.accept().unwrap();
            let mut link = TcpLink::new(socket, Duration::from_secs(5)).unwrap();
            link.recv()
        });
        let mut rude = TcpStream::connect(address).unwrap();
        rude.write_all(&u32::MAX.to_be_bytes()).unwrap();
        rude.flush().unwrap();
        assert_eq!(waiter.join().unwrap(), Err(Error::TooLarge));

        // ...and this post does not send one either.
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let waiter = std::thread::spawn(move || listener.accept().unwrap());
        let mut caller =
            TcpLink::new(TcpStream::connect(address).unwrap(), Duration::from_secs(5)).unwrap();
        let _held = waiter.join().unwrap();
        assert_eq!(caller.send(&vec![0u8; MAX_WIRE + 1]), Err(Error::TooLarge));
    }
}
