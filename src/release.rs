//! What version this is, and whether GitHub has a newer one.
//!
//! The application is offline by design: every « … » button hands a URL
//! to the browser and stops there. The one exception lives here, and it
//! is asked for — Options › À propos, « Vérifier la dernière version »,
//! never a check at startup. The launcher already does that at every
//! start; this is for the copy someone runs directly, and for the
//! question « est-ce qu'on est à jour ? » asked at the counter.
//!
//! Version comparison is by number, not by string: `v0.9.0` and
//! `v0.10.0` sort the wrong way round as text, and an officine told it
//! was ahead of a release it is behind would never look again.

use std::sync::mpsc::{Receiver, Sender};

const REPO: &str = "youssefsahli/bpm-caddy";

/// The releases page, for the button that opens the notes in a browser.
pub const RELEASES_URL: &str = "https://github.com/youssefsahli/bpm-caddy/releases";

/// What this binary is.
pub fn current() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// The platform this binary was built for, as the release assets name it.
pub fn target() -> &'static str {
    if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        "linux-x86_64"
    } else if cfg!(target_os = "windows") {
        "windows-x86_64"
    } else if cfg!(target_os = "macos") {
        "macos-arm64"
    } else {
        "—"
    }
}

/// The version the launcher last installed, if this copy was installed
/// by it. The launcher writes the tag it downloaded next to the binary;
/// a copy built or carried by hand has no such file.
pub fn launcher_version() -> Option<String> {
    let path = dirs::data_dir()?.join("bpm-caddy").join("version.txt");
    let text = std::fs::read_to_string(path).ok()?;
    let tag = text.trim();
    (!tag.is_empty()).then(|| tag.to_owned())
}

/// A dotted version as numbers, so `0.10.0` compares above `0.9.0`.
///
/// A leading `v` is ignored — the tags carry one, `CARGO_PKG_VERSION`
/// does not. Each component is read as far as its digits go and no
/// further: `0.107.0-rc1` is release `0.107.0`, and a tag nobody can
/// read at all is zero. Both fall the safe way — a tag the application
/// does not understand never claims to be ahead of what is installed,
/// so nobody is told to update to something that does not exist.
fn parts(tag: &str) -> Vec<u64> {
    tag.trim()
        .trim_start_matches(['v', 'V'])
        .split('.')
        .map(|p| {
            let digits: String = p.chars().take_while(char::is_ascii_digit).collect();
            digits.parse::<u64>().unwrap_or(0)
        })
        .collect()
}

/// Is `latest` strictly newer than `current`? Missing components count
/// as zero, so `1.2` and `1.2.0` are the same release.
pub fn is_newer(latest: &str, current: &str) -> bool {
    let (a, b) = (parts(latest), parts(current));
    let n = a.len().max(b.len());
    for i in 0..n {
        let (x, y) = (
            a.get(i).copied().unwrap_or(0),
            b.get(i).copied().unwrap_or(0),
        );
        if x != y {
            return x > y;
        }
    }
    false
}

/// What a finished check found.
pub enum Checked {
    /// The tag of the newest release, and whether it is ahead of us.
    Latest { tag: String, newer: bool },
    /// The network, GitHub, or the answer's shape.
    Failed(String),
}

/// Ask GitHub for the newest release tag, on a thread of its own.
///
/// Returns the receiving end straight away: the interface goes on
/// painting, and reads the answer whenever it arrives. Timeouts are
/// short and deliberate — a hung connection must never be something the
/// operator has to wait out.
pub fn check_async() -> Receiver<Checked> {
    let (tx, rx): (Sender<Checked>, Receiver<Checked>) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(check());
    });
    rx
}

fn check() -> Checked {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(std::time::Duration::from_secs(10))
        .timeout_read(std::time::Duration::from_secs(15))
        .user_agent("bpm-caddy")
        .build();
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let value: serde_json::Value = match agent.get(&url).call() {
        Ok(r) => match r.into_json() {
            Ok(v) => v,
            Err(e) => return Checked::Failed(e.to_string()),
        },
        Err(e) => return Checked::Failed(e.to_string()),
    };
    match value["tag_name"].as_str() {
        Some(tag) => Checked::Latest {
            newer: is_newer(tag, current()),
            tag: tag.to_owned(),
        },
        None => Checked::Failed("réponse GitHub sans tag_name".to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_compare_as_numbers_not_as_text() {
        // The whole reason this is not a string comparison.
        assert!(is_newer("v0.10.0", "0.9.0"));
        assert!(!is_newer("v0.9.0", "0.10.0"));
        assert!(is_newer("v0.107.1", "0.107.0"));
        assert!(is_newer("v1.0.0", "0.107.0"));
    }

    #[test]
    fn the_same_release_is_never_newer_than_itself() {
        assert!(!is_newer("v0.107.0", "0.107.0"));
        assert!(!is_newer("0.107.0", "0.107.0"));
        // Missing components are zero: 1.2 and 1.2.0 are one release.
        assert!(!is_newer("v1.2", "1.2.0"));
        assert!(!is_newer("v1.2.0", "1.2"));
    }

    #[test]
    fn a_tag_nobody_can_read_is_not_a_newer_release() {
        assert!(!is_newer("nightly", "0.107.0"));
        assert!(!is_newer("", "0.107.0"));
        // A pre-release suffix is read off: v0.107.0-rc1 is 0.107.0,
        // and nobody is told to update to a tag that is not a release.
        assert!(!is_newer("v0.107.0-rc1", "0.107.0"));
        assert!(is_newer("v0.108.0-rc1", "0.107.0"));
    }

    #[test]
    fn this_binary_knows_what_it_is() {
        assert_eq!(current(), env!("CARGO_PKG_VERSION"));
        assert!(!current().is_empty());
        assert!(RELEASES_URL.starts_with("https://github.com/"));
    }
}
