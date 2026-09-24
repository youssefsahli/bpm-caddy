//! A sealed box: a payload only its recipients can open.
//!
//! The journal seals every record under the trousseau, which every member
//! of the network holds. That is right for what the network shares with
//! everyone, and wrong for a message meant for two officines out of ten:
//! the eight others carry the record and must not read it. This module is
//! the second seal, inside the first.
//!
//! **Construction** — nothing invented:
//!
//! * each device has a *box key*, an X25519 key pair derived from its
//!   seed under its own domain string (`Device::box_secret`) — never the
//!   signing key, for the reason `keys.rs` gives: a key with two uses is a
//!   key whose uses can be played against each other;
//! * the payload is sealed once, under a random content key, with
//!   XChaCha20-Poly1305;
//! * the content key is wrapped for each recipient under a key derived
//!   (BLAKE3 `derive_key`) from an X25519 exchange between a one-time key
//!   and the recipient's box key, bound to both public keys;
//! * the box does **not** list its recipients: a recipient finds its
//!   entry by trying each one. Who writes to whom stays between them.
//!
//! Who *sent* the box is not this module's business: the record that
//! carries it is signed by its author, and the journal checks that.
//!
//! Layout: `version (1) ‖ one-time public (32) ‖ count (1) ‖ count ×
//! (nonce (24) ‖ wrapped key (48)) ‖ nonce (24) ‖ sealed payload`.

use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use curve25519_dalek::montgomery::MontgomeryPoint;
use zeroize::Zeroize;

use crate::{Entropy, Error, Result};

const VERSION: u8 = 1;
const WRAP_DOMAIN: &str = "bpm-caddy/box-wrap/v1";
const ENTRY: usize = 24 + 48;

/// At most this many recipients in one box — a count that fits a byte,
/// and far more than a conversation between officines needs.
pub const MAX_RECIPIENTS: usize = 64;

/// The public half of a box key, from its secret.
pub fn public_of(secret: &[u8; 32]) -> [u8; 32] {
    MontgomeryPoint::mul_base_clamped(*secret).to_bytes()
}

fn wrap_key(shared: &MontgomeryPoint, ephemeral: &[u8; 32], recipient: &[u8; 32]) -> [u8; 32] {
    let mut material = Vec::with_capacity(96);
    material.extend_from_slice(shared.as_bytes());
    material.extend_from_slice(ephemeral);
    material.extend_from_slice(recipient);
    let key = blake3::derive_key(WRAP_DOMAIN, &material);
    material.zeroize();
    key
}

/// Seal `payload` for these box keys (public halves). Refuses no
/// recipient, too many, or a recipient key that is the identity point.
pub fn seal(recipients: &[[u8; 32]], payload: &[u8], entropy: &mut dyn Entropy) -> Result<Vec<u8>> {
    if recipients.is_empty() || recipients.len() > MAX_RECIPIENTS {
        return Err(Error::Seal);
    }
    let mut content_key = [0u8; 32];
    entropy.fill(&mut content_key);
    let mut one_time = [0u8; 32];
    entropy.fill(&mut one_time);
    let ephemeral = public_of(&one_time);
    let mut out = Vec::with_capacity(58 + recipients.len() * ENTRY + payload.len() + 16);
    out.push(VERSION);
    out.extend_from_slice(&ephemeral);
    out.push(u8::try_from(recipients.len()).map_err(|_| Error::Seal)?);
    for r in recipients {
        let shared = MontgomeryPoint(*r).mul_clamped(one_time);
        // A small-order key would make the shared secret predictable.
        if shared.as_bytes().iter().all(|b| *b == 0) {
            content_key.zeroize();
            one_time.zeroize();
            return Err(Error::Seal);
        }
        let mut k = wrap_key(&shared, &ephemeral, r);
        let mut nonce = [0u8; 24];
        entropy.fill(&mut nonce);
        let wrapped = XChaCha20Poly1305::new(&Key::from(k))
            .encrypt(&XNonce::from(nonce), content_key.as_slice())
            .map_err(|_| Error::Seal)?;
        k.zeroize();
        out.extend_from_slice(&nonce);
        out.extend_from_slice(&wrapped);
    }
    one_time.zeroize();
    let mut nonce = [0u8; 24];
    entropy.fill(&mut nonce);
    // The header is bound to the payload: an entry cannot be moved onto
    // another box.
    let header = out.clone();
    let sealed = XChaCha20Poly1305::new(&Key::from(content_key))
        .encrypt(
            &XNonce::from(nonce),
            Payload {
                msg: payload,
                aad: &header,
            },
        )
        .map_err(|_| Error::Seal)?;
    content_key.zeroize();
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&sealed);
    Ok(out)
}

/// Open a box with this box secret. `Err(Error::Seal)` when the box is
/// not for this key, or has been touched.
pub fn open(secret: &[u8; 32], sealed: &[u8]) -> Result<Vec<u8>> {
    if sealed.len() < 34 || sealed[0] != VERSION {
        return Err(Error::Seal);
    }
    let ephemeral: [u8; 32] = sealed[1..33].try_into().map_err(|_| Error::Seal)?;
    let count = usize::from(sealed[33]);
    let header_len = 34 + count * ENTRY;
    if sealed.len() < header_len + 24 + 16 {
        return Err(Error::Seal);
    }
    let me = public_of(secret);
    let shared = MontgomeryPoint(ephemeral).mul_clamped(*secret);
    let mut k = wrap_key(&shared, &ephemeral, &me);
    let cipher = XChaCha20Poly1305::new(&Key::from(k));
    let mut content_key = None;
    for i in 0..count {
        let at = 34 + i * ENTRY;
        let nonce: [u8; 24] = sealed[at..at + 24].try_into().map_err(|_| Error::Seal)?;
        let wrapped = &sealed[at + 24..at + ENTRY];
        if let Ok(key) = cipher.decrypt(&XNonce::from(nonce), wrapped) {
            content_key = Some(key);
            break;
        }
    }
    k.zeroize();
    let Some(mut key) = content_key else {
        return Err(Error::Seal);
    };
    let Ok(mut key32) = <[u8; 32]>::try_from(key.as_slice()) else {
        key.zeroize();
        return Err(Error::Seal);
    };
    key.zeroize();
    let header = &sealed[..header_len];
    let nonce: [u8; 24] = sealed[header_len..header_len + 24]
        .try_into()
        .map_err(|_| Error::Seal)?;
    let body = &sealed[header_len + 24..];
    let opened = XChaCha20Poly1305::new(&Key::from(key32))
        .decrypt(
            &XNonce::from(nonce),
            Payload {
                msg: body,
                aad: header,
            },
        )
        .map_err(|_| Error::Seal);
    key32.zeroize();
    opened
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Counter(u8);
    impl Entropy for Counter {
        fn fill(&mut self, out: &mut [u8]) {
            for b in out.iter_mut() {
                self.0 = self.0.wrapping_mul(31).wrapping_add(17);
                *b = self.0;
            }
        }
    }

    fn secret(n: u8) -> [u8; 32] {
        blake3::derive_key("test", &[n])
    }

    /// **Chaque destinataire ouvre, et personne d'autre.**
    #[test]
    fn recipients_open_and_nobody_else_does() {
        let (a, b, c) = (secret(1), secret(2), secret(3));
        let boxed = seal(
            &[public_of(&a), public_of(&b)],
            b"Rupture Diprosone",
            &mut Counter(5),
        )
        .unwrap();
        assert_eq!(open(&a, &boxed).unwrap(), b"Rupture Diprosone");
        assert_eq!(open(&b, &boxed).unwrap(), b"Rupture Diprosone");
        assert!(open(&c, &boxed).is_err(), "une tierce officine ne lit rien");
    }

    /// **Rien ne se touche** : un octet changé où que ce soit, et la
    /// boîte ne s'ouvre plus.
    #[test]
    fn a_touched_box_does_not_open() {
        let a = secret(1);
        let boxed = seal(&[public_of(&a)], b"message", &mut Counter(9)).unwrap();
        for i in 0..boxed.len() {
            let mut bad = boxed.clone();
            bad[i] ^= 0x01;
            assert!(open(&a, &bad).is_err(), "octet {i}");
        }
        assert!(open(&a, &boxed[..boxed.len() - 1]).is_err());
        assert!(open(&a, &[]).is_err());
    }

    /// Les destinataires ne se lisent pas dans la boîte : ni leur clé,
    /// ni leur empreinte.
    #[test]
    fn the_box_does_not_name_its_recipients() {
        let a = secret(1);
        let pa = public_of(&a);
        let boxed = seal(&[pa], b"x", &mut Counter(3)).unwrap();
        assert!(!boxed.windows(32).any(|w| w == pa));
    }

    #[test]
    fn a_box_needs_recipients_and_refuses_a_null_key() {
        assert!(seal(&[], b"x", &mut Counter(1)).is_err());
        assert!(seal(&[[0u8; 32]], b"x", &mut Counter(1)).is_err());
        let many = vec![public_of(&secret(1)); MAX_RECIPIENTS + 1];
        assert!(seal(&many, b"x", &mut Counter(1)).is_err());
    }
}
