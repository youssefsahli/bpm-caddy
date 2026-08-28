//! Reading a carte Vitale.
//!
//! What this does and what it deliberately does not do:
//!
//! - It **reads the beneficiary identity** off the card through a PC/SC
//!   reader — the NIR, the names, the date of birth — so that presenting
//!   the card opens the patient's file, or fills a new one, instead of
//!   the operator typing fifteen digits from a plastic card.
//! - It does **not** produce feuilles de soins électroniques. An FSE
//!   requires an agreed SESAM-Vitale package, a carte CPS and a
//!   concentrateur; nothing here pretends otherwise, and the officine's
//!   LGO stays the tool that bills.
//!
//! **How the identity is found.** The card's data files are read as
//! bytes and searched, rather than decoded at fixed offsets. That is a
//! deliberate choice: the file layout differs between a Vitale 1 and a
//! Vitale 2, and between versions of the same generation, and an offset
//! guessed wrong reads a plausible-looking wrong patient — which is the
//! one failure this module must never have. A NIR carries its own
//! two-digit control key, so a fifteen-digit run whose key checks out is
//! a NIR and not a coincidence: the odds of a random run passing are one
//! in ninety-seven, and the names around it confirm it. Everything in
//! this module up to the card transmission is pure and tested; what
//! cannot be tested without a card and a reader is the transmission
//! itself, and it is one function.

use std::fmt::Write as _;

/// One person the card knows about: the holder, or one of the ayants
/// droit it carries.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Beneficiary {
    /// Fifteen digits, control key verified.
    pub nir: String,
    pub surname: String,
    pub given: String,
    /// ISO `AAAA-MM-JJ`, or empty when the card gave only the month and
    /// the year — which the NIR alone always carries.
    pub birth_date: String,
}

impl Beneficiary {
    /// « DUPONT Jean », for a list row.
    pub fn label(&self) -> String {
        format!("{} {}", self.surname.trim(), self.given.trim())
            .trim()
            .to_owned()
    }
}

/// The two control digits of a NIR: `97 - (n mod 97)`, the thirteen-digit
/// body read as a number.
///
/// Corsica is the exception the formula needs: the département reads
/// `2A` or `2B` on the card and in the body, and those become 19 and 18
/// before the modulo. A NIR from Ajaccio checked without that
/// substitution fails, and the file would be refused.
pub fn nir_key(body: &str) -> Option<u8> {
    let body = body.trim();
    // ASCII first, and only then by byte: the card's text carries
    // accented names, and `body[5..7]` on an « È » panics on a char
    // boundary. A NIR holds digits and, for Corsica, an A or a B —
    // nothing that is not ASCII can be one.
    if !body.is_ascii() || body.len() != 13 {
        return None;
    }
    let body = body.to_ascii_uppercase();
    // The two Corsican départements are the only letters a NIR may hold.
    let digits: String = match &body[5..7] {
        "2A" => format!("{}19{}", &body[..5], &body[7..]),
        "2B" => format!("{}18{}", &body[..5], &body[7..]),
        _ => body.clone(),
    };
    if !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    // Thirteen digits overflow nothing in u64, and the modulo is taken
    // on the whole number and not digit by digit.
    let n: u64 = digits.parse().ok()?;
    Some((97 - (n % 97)) as u8)
}

/// Is this a whole NIR — thirteen digits of body and two of key, and the
/// key is the one the body implies?
pub fn nir_is_valid(nir: &str) -> bool {
    let nir: String = nir.chars().filter(|c| !c.is_whitespace()).collect();
    if !nir.is_ascii() || nir.len() != 15 {
        return false;
    }
    let (body, key) = nir.split_at(13);
    match (nir_key(body), key.parse::<u8>()) {
        (Some(expected), Ok(given)) => expected == given,
        _ => false,
    }
}

/// The month and the year of birth a NIR carries, as `(année, mois)`.
///
/// The year is on two digits and the century is not in the NIR: a `55`
/// is 1955 for anyone alive today, and the window slides with
/// `this_year`. A month above 12 is not an error — the codes 20, 30, 40
/// and 50 exist for people whose birth month is unknown or who were
/// naturalised — and the caller gets `None` for the month rather than a
/// date invented from it.
pub fn nir_birth(nir: &str, this_year: u32) -> Option<(u32, Option<u32>)> {
    if !nir_is_valid(nir) {
        return None;
    }
    let yy: u32 = nir.get(1..3)?.parse().ok()?;
    let mm: u32 = nir.get(3..5)?.parse().ok()?;
    // Two digits, one century: the year is the most recent one that is
    // not in the future.
    let base = this_year - this_year % 100;
    let year = if base + yy > this_year {
        base + yy - 100
    } else {
        base + yy
    };
    Some((year, (1..=12).contains(&mm).then_some(mm)))
}

/// Every beneficiary the raw card content names, in the order they are
/// found, without duplicates.
///
/// The NIR is the anchor — it is the only field that proves itself — and
/// the names are the printable text around it.
pub fn beneficiaries(raw: &[u8], this_year: u32) -> Vec<Beneficiary> {
    let text = decode(raw);
    let chars: Vec<char> = text.chars().collect();
    let mut out: Vec<Beneficiary> = Vec::new();
    let mut i = 0usize;
    while i + 15 <= chars.len() {
        let window: String = chars[i..i + 15].iter().collect();
        if !nir_is_valid(&window) {
            i += 1;
            continue;
        }
        // A run of digits longer than fifteen is not a NIR that happens
        // to start here: it is a number, and taking its first fifteen
        // characters would invent a person.
        let bounded = (i == 0 || !chars[i - 1].is_ascii_digit())
            && (i + 15 == chars.len() || !chars[i + 15].is_ascii_digit());
        if !bounded {
            i += 1;
            continue;
        }
        let (surname, given) = names_around(&chars, i);
        let birth_date = birth_around(&chars, i, &window, this_year);
        let found = Beneficiary {
            nir: window,
            surname,
            given,
            birth_date,
        };
        if !out.iter().any(|b| b.nir == found.nir) {
            out.push(found);
        }
        i += 15;
    }
    out
}

/// A byte that is not text at all, kept as one character so offsets are
/// preserved. It is deliberately **not** a space: a space belongs inside
/// a name — « JEAN PIERRE », « DE LA TOUR » — so mapping separators to
/// spaces glued two fields into one run and handed back « LEFÈVRE
/// HÉLÈNE » as a single surname.
const SEP: char = '\u{1}';

/// The card's bytes as text.
///
/// Latin-1 and not UTF-8: the card writes ISO 8859 and a `É` read as
/// UTF-8 is two replacement characters, which cuts a name in half. One
/// character in, one character out, so every offset found in the text is
/// the offset in the card.
fn decode(raw: &[u8]) -> String {
    raw.iter()
        .map(|&b| match b {
            0x20..=0x7e => b as char,
            0xc0..=0xff => b as char, // Latin-1 letters, accents included
            _ => SEP,
        })
        .collect()
}

/// Is this a character a French civil-status name may contain?
fn name_char(c: char) -> bool {
    c.is_alphabetic() || c == ' ' || c == '-' || c == '\'' || c == '.'
}

/// The two text runs nearest the NIR: the surname then the given name.
///
/// The card writes them next to the NIR, before it or after it depending
/// on the file. Both sides are read, and the two runs closest to the NIR
/// win — which is what makes this independent of the layout.
///
/// Which of the two is the surname is settled by **position and not by
/// distance**: a civil-status record writes the name before the given
/// name, on the card as on paper, so of the two runs the earlier one is
/// the surname. Ranking by distance instead reads « DUPONT⏎JEAN⏎NIR »
/// backwards and hands « JEAN DUPONT » to a counter, which is the sort
/// of wrong that gets typed onto a file and stays there.
fn names_around(chars: &[char], at: usize) -> (String, String) {
    let start = at.saturating_sub(96);
    let end = (at + 15 + 96).min(chars.len());
    let mut runs: Vec<(usize, String)> = Vec::new();
    let mut current = String::new();
    let mut current_at = start;
    for (k, &c) in chars.iter().enumerate().take(end).skip(start) {
        // Never read *through* the NIR: a run that swallowed it would
        // hand back fifteen digits as a surname.
        let inside = (at..at + 15).contains(&k);
        if name_char(c) && !inside {
            if current.is_empty() {
                current_at = k;
            }
            current.push(c);
        } else {
            push_run(&mut runs, current_at, std::mem::take(&mut current));
        }
    }
    push_run(&mut runs, current_at, current);
    // Nearest to the NIR first: the card's own fields sit against it,
    // and whatever else is in the file is further away.
    runs.sort_by_key(|(k, _)| at.abs_diff(*k));
    runs.truncate(2);
    // …then back into the order the card wrote them.
    runs.sort_by_key(|(k, _)| *k);
    let mut it = runs.into_iter().map(|(_, s)| s);
    (it.next().unwrap_or_default(), it.next().unwrap_or_default())
}

fn push_run(runs: &mut Vec<(usize, String)>, at: usize, run: String) {
    let run = run.trim().trim_matches(['-', '\'', '.']).trim().to_owned();
    // Two letters is a name — « Ly », « An » — one is a separator that
    // survived, and a run without a letter is not a name at all.
    if run.chars().filter(|c| c.is_alphabetic()).count() >= 2 {
        runs.push((at, run));
    }
}

/// A full date of birth found beside the NIR, as ISO — and only if it
/// agrees with the month and the year the NIR itself carries.
///
/// That agreement is the whole point: eight digits near a NIR could be
/// anything, and a birth date invented from an expiry date would put a
/// wrong age on a file for years.
fn birth_around(chars: &[char], at: usize, nir: &str, this_year: u32) -> String {
    let Some((year, month)) = nir_birth(nir, this_year) else {
        return String::new();
    };
    let Some(month) = month else {
        return String::new();
    };
    let start = at.saturating_sub(96);
    let end = (at + 15 + 96).min(chars.len());
    let text: String = chars[start..end].iter().collect();
    let digits: Vec<char> = text.chars().collect();
    for k in 0..digits.len().saturating_sub(7) {
        if !digits[k..k + 8].iter().all(|c| c.is_ascii_digit()) {
            continue;
        }
        // Not part of a longer number: an eight-digit slice of a
        // sixteen-digit field is not a date.
        if (k > 0 && digits[k - 1].is_ascii_digit())
            || (k + 8 < digits.len() && digits[k + 8].is_ascii_digit())
        {
            continue;
        }
        let run: String = digits[k..k + 8].iter().collect();
        // Both orders exist on the card; the NIR decides which reading
        // is the right one, and refuses both when neither agrees.
        for (y, m, d) in [
            (&run[0..4], &run[4..6], &run[6..8]),
            (&run[4..8], &run[2..4], &run[0..2]),
        ] {
            let (Ok(y), Ok(m), Ok(d)) = (y.parse::<u32>(), m.parse::<u32>(), d.parse::<u32>())
            else {
                continue;
            };
            if y == year && m == month && (1..=31).contains(&d) {
                let mut out = String::new();
                let _ = write!(out, "{y:04}-{m:02}-{d:02}");
                return out;
            }
        }
    }
    String::new()
}

/// What the reader answered, whether or not a card was found.
pub struct CardRead {
    /// Every reader the system knows, so a wrong one is visible.
    pub readers: Vec<String>,
    /// The card's ATR, hex, when a card answered.
    pub atr: String,
    /// The concatenated data of every command that returned some.
    pub data: Vec<u8>,
    /// One line per command: what was sent and what came back.
    pub log: Vec<String>,
}

/// Read a card through a PC/SC reader.
///
/// `script` is the sequence of APDU commands, in hex. It lives in
/// `config.toml` rather than in this file on purpose: the command
/// sequence that reaches the beneficiary data is part of the SESAM-Vitale
/// specification, which is licensed and versioned, and a sequence
/// compiled into the binary could neither be corrected by the officine
/// nor follow a card generation it predates. With no script the function
/// still connects and answers with the readers and the ATR, which is
/// what tells an operator whether the reader and the card are seen at
/// all.
///
/// A command that fails is logged and the next is sent: a script written
/// for another card generation must still say what it managed to read,
/// because that log is the only thing an officine can send to whoever
/// wrote the script.
pub fn read_card(reader_filter: &str, script: &[Vec<u8>]) -> Result<CardRead, String> {
    let ctx = crate::winscard::Context::open()?;
    let names = ctx.readers()?;
    let mut out = CardRead {
        readers: names.clone(),
        atr: String::new(),
        data: Vec::new(),
        log: Vec::new(),
    };
    let filter = reader_filter.trim().to_lowercase();
    let Some(name) = names
        .iter()
        .find(|n| filter.is_empty() || n.to_lowercase().contains(&filter))
    else {
        return Err(if names.is_empty() {
            "aucun lecteur de carte détecté".to_owned()
        } else {
            format!("aucun lecteur ne correspond à « {reader_filter} »")
        });
    };
    let card = ctx
        .connect(name)
        .map_err(|e| format!("carte illisible dans « {name} » : {e}"))?;
    if let Some(atr) = card.atr() {
        out.atr = hex(&atr);
    }
    for command in script {
        match card.transmit(command) {
            Ok(reply) => {
                let (body, sw) = reply.split_at(reply.len().saturating_sub(2));
                out.log
                    .push(format!("> {}\n< {} [{}]", hex(command), hex(body), hex(sw)));
                out.data.extend_from_slice(body);
            }
            Err(e) => out.log.push(format!("> {}\n! {e}", hex(command))),
        }
    }
    Ok(out)
}

/// Bytes as uppercase hex, spaced — the form an APDU is written in.
pub fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 3);
    for b in bytes {
        let _ = write!(out, "{b:02X} ");
    }
    out.trim_end().to_owned()
}

/// One APDU written as hex, with or without spaces, into bytes. An
/// unreadable command is refused rather than truncated: half an APDU is
/// a different command.
pub fn parse_hex(text: &str) -> Result<Vec<u8>, String> {
    let clean: String = text
        .chars()
        .filter(|c| !c.is_whitespace() && *c != ':')
        .collect();
    if clean.is_empty() || !clean.len().is_multiple_of(2) {
        return Err(format!("commande APDU illisible : « {text} »"));
    }
    (0..clean.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&clean[i..i + 2], 16).map_err(|_| format!("« {text} »")))
        .collect::<Result<Vec<u8>, String>>()
        .map_err(|e| format!("commande APDU illisible : {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A NIR proves itself: that is what makes the whole search safe.
    ///
    /// The key is `97 - (n mod 97)` on the thirteen-digit body, and the
    /// example below is checked by hand: 1 550 875 116 001 mod 97 = 72,
    /// so the key is 25.
    #[test]
    fn a_nir_carries_its_own_control_key() {
        assert_eq!(1_550_875_116_001_u64 % 97, 72);
        assert_eq!(nir_key("1550875116001"), Some(25));
        assert!(nir_is_valid("155087511600125"));
        // One digit changed anywhere and the key no longer matches.
        assert!(!nir_is_valid("155087511600126"));
        assert!(!nir_is_valid("255087511600125"));
        // Too short, too long, not digits.
        assert!(!nir_is_valid("15508751160012"));
        assert!(!nir_is_valid("1550875116001250"));
        assert!(!nir_is_valid("abcdefghijklmno"));
        assert_eq!(nir_key("155087511600"), None);
    }

    /// Corsica is the exception the formula needs: without the 2A → 19
    /// and 2B → 18 substitution, every NIR from the island is refused.
    /// The département sits on the sixth and seventh characters.
    #[test]
    fn corsica_is_read_before_the_modulo() {
        assert_eq!(nir_key("155082A116001"), nir_key("1550819116001"));
        assert_eq!(nir_key("155082B116001"), nir_key("1550818116001"));
        assert!(nir_is_valid("155082A11600182"));
        assert!(nir_is_valid("155082B11600112"));
        // And a letter anywhere else is still not a NIR.
        assert_eq!(nir_key("1A50875116001"), None);
        assert_eq!(nir_key("2A50875116001"), None);
    }

    #[test]
    fn the_nir_gives_the_month_and_the_year_but_not_the_day() {
        assert_eq!(nir_birth("155087511600125", 2026), Some((1955, Some(8))));
        // A month code that is not a month — 20, 30, 40 and 50 exist for
        // an unknown birth month — gives no month rather than an
        // invented one, and therefore no date.
        assert_eq!(nir_birth("155307511600186", 2026), Some((1955, None)));
        // Two digits, one century: the year is the most recent that is
        // not in the future, because nobody is born tomorrow.
        assert_eq!(nir_birth("226027511600139", 2026).map(|b| b.0), Some(2026));
        assert_eq!(nir_birth("226027511600139", 2025).map(|b| b.0), Some(1926));
        assert_eq!(nir_birth("pas un nir", 2026), None);
    }

    /// The card is bytes, and the identity is found in them rather than
    /// read at an offset nobody can guarantee.
    #[test]
    fn a_beneficiary_is_found_by_its_nir_and_named_by_what_surrounds_it() {
        let mut raw: Vec<u8> = Vec::new();
        raw.extend_from_slice(&[0x00, 0x01, 0x02]);
        raw.extend_from_slice(b"DUPONT");
        raw.push(0x00);
        raw.extend_from_slice(b"JEAN");
        raw.push(0x00);
        raw.extend_from_slice(b"155087511600125");
        raw.push(0x00);
        raw.extend_from_slice(b"03081955");
        let found = beneficiaries(&raw, 2026);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].nir, "155087511600125");
        // The card writes « DUPONT » then « JEAN », and so does the row:
        // the given name sits nearer the NIR, but it is not the surname
        // for that.
        assert_eq!(found[0].surname, "DUPONT");
        assert_eq!(found[0].given, "JEAN");
        assert_eq!(found[0].label(), "DUPONT JEAN");
        // And the same fields written after the NIR read the same way.
        let after = beneficiaries(b"\x00155087511600125\x00DUPONT\x00JEAN\x00", 2026);
        assert_eq!(after[0].label(), "DUPONT JEAN");
        assert_eq!(found[0].birth_date, "1955-08-03");
    }

    /// Two people on one card — the holder and an ayant droit — and each
    /// keeps the names that sit beside its own NIR.
    #[test]
    fn every_beneficiary_of_the_card_is_returned_once() {
        let mut raw = Vec::new();
        raw.extend_from_slice(b"\x00MARTIN\x00CLAIRE\x00155087511600125\x00");
        raw.extend_from_slice(b"MARTIN\x00LUCIE\x00226027511600139");
        let found = beneficiaries(&raw, 2026);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].nir, "155087511600125");
        assert_eq!(found[1].nir, "226027511600139");
        assert_eq!(found[1].label(), "MARTIN LUCIE");
        // The same card read twice does not double the list.
        let twice = [raw.clone(), raw].concat();
        assert_eq!(beneficiaries(&twice, 2026).len(), 2);
    }

    /// What must never happen: a number that is not a NIR read as one,
    /// or a date that is not a birth date written onto a file.
    #[test]
    fn nothing_is_invented_from_bytes_that_prove_nothing() {
        // Fifteen digits that fail the key: no beneficiary at all.
        assert!(beneficiaries(b"DUPONT\x00155087511600126", 2026).is_empty());
        // A valid NIR embedded in a longer number is not a NIR.
        assert!(beneficiaries(b"9155087511600125", 2026).is_empty());
        assert!(beneficiaries(b"1550875116001259", 2026).is_empty());
        // A NIR with no name around it is still a NIR, and the fields
        // that are not there stay empty rather than being guessed.
        let bare = beneficiaries(b"\x00155087511600125\x00", 2026);
        assert_eq!(bare.len(), 1);
        assert!(bare[0].surname.is_empty());
        assert!(bare[0].birth_date.is_empty());
        // A date beside the NIR that disagrees with it is not the birth
        // date: an expiry date would otherwise age the patient by years.
        let wrong = beneficiaries(b"DUPONT\x00155087511600125\x0012122030", 2026);
        assert_eq!(wrong.len(), 1);
        assert!(wrong[0].birth_date.is_empty());
        // Empty input is not an error, it is nobody.
        assert!(beneficiaries(b"", 2026).is_empty());
    }

    /// Accented names come off the card in Latin-1, and read as UTF-8
    /// they lose half their letters.
    #[test]
    fn an_accented_name_survives_the_card_encoding() {
        let mut raw: Vec<u8> = Vec::new();
        raw.push(0x00);
        // « LEFÈVRE » in ISO 8859-1: È is 0xC8, É is 0xC9.
        raw.extend_from_slice(&[b'L', b'E', b'F', 0xC8, b'V', b'R', b'E']);
        raw.extend_from_slice(&[0x00, b'H', 0xC9, b'L', 0xC8, b'N', b'E', 0x00]);
        raw.extend_from_slice(b"155087511600125");
        let found = beneficiaries(&raw, 2026);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].label(), "LEFÈVRE HÉLÈNE");
    }

    #[test]
    fn an_apdu_is_read_whole_or_refused() {
        let want = vec![0x00, 0xA4, 0x04, 0x00];
        assert_eq!(parse_hex("00 A4 04 00").unwrap(), want);
        assert_eq!(parse_hex("00a40400").unwrap(), want);
        assert_eq!(parse_hex("00:A4:04:00").unwrap(), want);
        // Half a byte is a different command, so it is refused.
        assert!(parse_hex("00 A4 0").is_err());
        assert!(parse_hex("").is_err());
        assert!(parse_hex("zz").is_err());
        assert_eq!(hex(&[0x00, 0xA4, 0xFF]), "00 A4 FF");
        assert_eq!(hex(&[]), "");
    }
}
