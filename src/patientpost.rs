//! **Le poste qui attend les patients** : un poste de bureau de
//! l'officine attend à la passerelle (`passerelle.rs`) qu'une page de
//! patient le demande, et lui répond avec la base (`patientchat::answer`).
//!
//! Deux attentes à la fois, chacune sur son fil : pendant qu'une
//! conversation se tient, la suivante trouve encore le poste. Les
//! conversations passent une à une au fil qui tient la base. Chaque poste
//! ouvert attend : un patient trouve l'officine tant qu'un poste est
//! allumé. Le téléphone de l'équipe n'attend jamais — il ne tient pas les
//! dossiers.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use crate::db::Db;
use crate::patientchat::{self, FromPatient, KeyRow, Store, ToPatient};

/// Combien d'attentes à la fois, par poste.
const WAITERS: usize = 2;
/// Le temps de joindre la passerelle.
const PATIENCE: Duration = Duration::from_secs(10);
/// Après un échec (passerelle éteinte, nom inconnu), avant de rappeler.
const BACKOFF: Duration = Duration::from_secs(30);

/// Ce que le fil dit à l'écran.
#[derive(Debug, PartialEq, Eq)]
pub enum Event {
    /// Des messages de patients sont arrivés.
    Received(usize),
    /// La passerelle ne répond pas (le texte de l'erreur) ; rien n'est
    /// perdu, la page du patient garde ses messages.
    Unreachable(String),
    /// Elle répond de nouveau.
    Reached,
}

/// La base, vue par une conversation.
pub struct DbStore<'a> {
    pub db: &'a Db,
    pub officine: String,
}

/// L'heure d'un message, pour la page : `JJ/MM/AAAA HH:MM`.
pub fn shown_at(at: &str) -> String {
    match at.split_once(' ') {
        Some((day, time)) => format!("{} {time}", crate::db::format_french_date(day)),
        None => crate::db::format_french_date(at),
    }
}

impl Store for DbStore<'_> {
    fn keys(&mut self) -> Result<Vec<KeyRow>, String> {
        Ok(self
            .db
            .patient_chat_keys()?
            .into_iter()
            .filter_map(|k| {
                Some(KeyRow {
                    id: k.id,
                    key: patientchat::key_from_text(&k.key)?,
                    open: k.closed_on.is_empty(),
                })
            })
            .collect())
    }

    fn receive(&mut self, key: i64, messages: &[FromPatient]) -> Result<Vec<String>, String> {
        let rows: Vec<(String, String, String)> = messages
            .iter()
            .map(|m| (m.id.clone(), m.at.clone(), m.text.trim().to_owned()))
            .collect();
        self.db.receive_patient_messages(key, &rows)
    }

    fn replies(&mut self, key: i64, seen: &[i64]) -> Result<Vec<ToPatient>, String> {
        Ok(self
            .db
            .patient_replies(key, seen)?
            .into_iter()
            .map(|m| ToPatient {
                id: m.id,
                at: shown_at(&m.at),
                text: m.body,
            })
            .collect())
    }

    fn officine(&mut self) -> String {
        self.officine.clone()
    }
}

fn fill(buf: &mut [u8]) {
    bpm_sync::Entropy::fill(&mut bpm_sync::OsEntropy, buf);
}

/// Le fil qui attend : l'arrêter, c'est aussi lâcher les attentes en
/// cours à la passerelle, pour qu'un poste relancé (réglage changé) ne
/// trouve pas les places de sa boîte prises par lui-même.
pub struct Waiting {
    stop: Arc<AtomicBool>,
    held: Arc<Mutex<Vec<std::net::TcpStream>>>,
    pub events: mpsc::Receiver<Event>,
}

impl Waiting {
    pub fn stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Ok(mut h) = self.held.lock() {
            for s in h.drain(..) {
                let _ = s.shutdown(std::net::Shutdown::Both);
            }
        }
    }
}

impl Drop for Waiting {
    fn drop(&mut self) {
        self.stop();
    }
}

/// **Attendre les patients** à la passerelle `gateway` (un nom d'hôte),
/// pour la boîte de `secret`, avec la base de `path`.
pub fn spawn(
    path: std::path::PathBuf,
    password: String,
    gateway: &str,
    secret: String,
    officine: String,
) -> Waiting {
    let stop = Arc::new(AtomicBool::new(false));
    let held: Arc<Mutex<Vec<std::net::TcpStream>>> = Arc::default();
    let (events_tx, events) = mpsc::channel();
    let (talk_tx, talk_rx) = mpsc::sync_channel::<std::net::TcpStream>(0);
    let address = crate::passerelle::posts_address(gateway);
    for _ in 0..WAITERS {
        let (stop, held, talk_tx, events_tx, address, secret) = (
            Arc::clone(&stop),
            Arc::clone(&held),
            talk_tx.clone(),
            events_tx.clone(),
            address.clone(),
            secret.clone(),
        );
        std::thread::spawn(move || {
            let mut failing = false;
            while !stop.load(Ordering::SeqCst) {
                match wait_once(&address, &secret, &held) {
                    Ok(stream) => {
                        if failing {
                            failing = false;
                            let _ = events_tx.send(Event::Reached);
                        }
                        if talk_tx.send(stream).is_err() {
                            return;
                        }
                    }
                    // The wait ran out with nobody: wait again at once.
                    Err(e)
                        if matches!(
                            e.kind(),
                            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                        ) => {}
                    Err(e) => {
                        if !failing && !stop.load(Ordering::SeqCst) {
                            failing = true;
                            let _ = events_tx.send(Event::Unreachable(e.to_string()));
                        }
                        let until = std::time::Instant::now() + BACKOFF;
                        while std::time::Instant::now() < until && !stop.load(Ordering::SeqCst) {
                            std::thread::sleep(Duration::from_millis(250));
                        }
                    }
                }
            }
        });
    }
    let worker_stop = Arc::clone(&stop);
    std::thread::spawn(move || {
        let Ok(db) = Db::open(&path, &password) else {
            let _ = events_tx.send(Event::Unreachable(String::new()));
            return;
        };
        let mut store = DbStore { db: &db, officine };
        while !worker_stop.load(Ordering::SeqCst) {
            let Ok(mut stream) = talk_rx.recv_timeout(Duration::from_millis(500)) else {
                continue;
            };
            if let Ok(patientchat::Outcome::Talked { received, .. }) =
                patientchat::answer(&mut stream, &mut store, fill)
            {
                if received > 0 {
                    let _ = events_tx.send(Event::Received(received));
                }
            }
        }
    });
    Waiting { stop, held, events }
}

/// Une attente, tenue dans `held` le temps qu'elle dure.
fn wait_once(
    address: &str,
    secret: &str,
    held: &Mutex<Vec<std::net::TcpStream>>,
) -> std::io::Result<std::net::TcpStream> {
    let stream = crate::passerelle::wait_with(address, secret, PATIENCE, |s| {
        if let (Ok(mut h), Ok(c)) = (held.lock(), s.try_clone()) {
            h.push(c);
        }
    });
    // Out of the list once the wait is over, whichever way.
    if let Ok(mut h) = held.lock() {
        h.retain(|s| {
            s.peer_addr().is_ok()
                && stream
                    .as_ref()
                    .map(|t| t.local_addr().ok() != s.local_addr().ok())
                    .unwrap_or(true)
        });
    }
    stream
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Swept(std::path::PathBuf);
    impl Drop for Swept {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn a_time_for_the_page() {
        assert_eq!(shown_at("2026-10-14 09:05"), "14/10/2026 09:05");
        assert_eq!(shown_at("2026-10-14"), "14/10/2026");
    }

    /// From the gateway to the base and back: a page writes, the base
    /// keeps it once, the officine answers, the page reads the answer; a
    /// closed access is told so.
    #[test]
    fn a_patient_writes_to_the_base_through_the_gateway() {
        let dir =
            std::env::temp_dir().join(format!("bpm-caddy-patientpost-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let _swept = Swept(dir.clone());
        let path = dir.join("base.db");
        let db = Db::open(&path, "secret").unwrap();
        let pid = db.add_patient("Durand", "Anne", "1950-01-02").unwrap();
        let key = patientchat::new_key(fill);
        let kid = db
            .open_patient_chat(pid, &patientchat::key_text(&key), None, "2026-10-14", "AB")
            .unwrap()
            .unwrap();
        assert!(db
            .write_patient_message(kid, "Votre commande est prête.", "AB")
            .unwrap());

        let free = || {
            std::net::TcpListener::bind("127.0.0.1:0")
                .unwrap()
                .local_addr()
                .unwrap()
        };
        let (web, posts) = (free(), free());
        let stop = Arc::new(AtomicBool::new(false));
        crate::passerelle::serve(web, posts, Arc::clone(&stop)).unwrap();
        let secret = "c3".repeat(32);
        let mut store = DbStore {
            db: &db,
            officine: "Pharmacie du Port".into(),
        };
        let talk = |store: &mut DbStore, request: patientchat::Request| {
            let (posts_at, secret2) = (posts.to_string(), secret.clone());
            let waiter = std::thread::spawn(move || {
                crate::passerelle::wait(&posts_at, &secret2, Duration::from_secs(5)).unwrap()
            });
            std::thread::sleep(Duration::from_millis(200));
            let mut page = crate::passerelle::tests_page(web, &patientchat::box_id(&[0xc3; 32]));
            let mut post = waiter.join().unwrap();
            let page_side = std::thread::spawn(move || {
                patientchat::talk_as_patient(&mut page, &key, [4; 16], &request).unwrap()
            });
            let o = patientchat::answer(&mut post, store, fill).unwrap();
            (o, page_side.join().unwrap())
        };
        let hello = |id: &str| patientchat::FromPatient {
            id: id.into(),
            at: "14/10/2026 08:00".into(),
            text: "Bonjour, ma commande ?".into(),
        };
        let (o, r) = talk(
            &mut store,
            patientchat::Request {
                out: vec![hello("p1")],
                seen: vec![],
            },
        );
        assert_eq!(
            o,
            patientchat::Outcome::Talked {
                key: kid,
                received: 1
            }
        );
        let r = r.unwrap();
        assert_eq!(r.got, vec!["p1".to_string()]);
        assert_eq!(r.messages.len(), 1);
        assert_eq!(r.messages[0].text, "Votre commande est prête.");
        // Sent again (the answer was lost): kept once; the reply seen is
        // not sent back.
        let seen = vec![r.messages[0].id];
        let (_, r) = talk(
            &mut store,
            patientchat::Request {
                out: vec![hello("p1")],
                seen,
            },
        );
        let r = r.unwrap();
        assert_eq!(r.got, vec!["p1".to_string()]);
        assert!(r.messages.is_empty());
        let all = db.patient_messages(pid).unwrap();
        assert_eq!(all.iter().filter(|m| m.from_patient).count(), 1);
        assert_eq!(db.unread_patient_messages().unwrap(), vec![(pid, 1)]);
        assert_eq!(db.mark_patient_messages_read(pid, "2026-10-14").unwrap(), 1);
        assert!(db.unread_patient_messages().unwrap().is_empty());
        // A new access closes the old one: the old phone is told.
        let shown = db.patient_chat_key(pid).unwrap().map(|k| k.id);
        assert_eq!(shown, Some(kid));
        assert_eq!(
            db.open_patient_chat(
                pid,
                &patientchat::key_text(&[1; 32]),
                None,
                "2026-10-14",
                "AB"
            )
            .unwrap(),
            None,
            "against what the screen showed"
        );
        db.open_patient_chat(
            pid,
            &patientchat::key_text(&[1; 32]),
            shown,
            "2026-10-14",
            "AB",
        )
        .unwrap()
        .unwrap();
        assert!(!db.write_patient_message(kid, "trop tard", "AB").unwrap());
        let (o, r) = talk(
            &mut store,
            patientchat::Request {
                out: vec![hello("p2")],
                seen: vec![],
            },
        );
        assert_eq!(o, patientchat::Outcome::Closed);
        assert_eq!(r.unwrap_err(), patientchat::CLOSED.to_vec());
        stop.store(true, Ordering::SeqCst);
    }
}
