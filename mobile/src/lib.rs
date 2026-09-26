//! **Le compagnon Android** : ce que l'application Kotlin appelle.
//!
//! Le téléphone d'un membre de l'équipe est un poste du groupe, d'une
//! sorte à part — un *compagnon* (`postes::COMPANION`). Il tient une
//! **part** de la clé des postes : les fiches, l'agenda, le planning et
//! l'équipe (messagerie, liste des postes). Jamais les dossiers, le
//! registre, la caisse ni les réglages de l'officine : ces
//! enregistrements lui parviennent scellés, il les garde pour que le
//! journal reste entier et les relaie, il ne les ouvre pas — il n'en a
//! pas la clé.
//!
//! Tout ici passe par la bibliothèque de l'application (`bpm-caddy`,
//! sans `desktop`) : la même base SQLCipher, le même rangement
//! (`replica`), la même conversation (`bpm-sync`). Ce crate ne fait que
//! traduire en types que UniFFI sait donner à Kotlin — il ne décide
//! rien de clinique et n'a pas d'horloge : le jour vient de l'appelant.

use std::net::UdpSocket;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use bpm_caddy::db::{Db, Drug, MONO_FIELDS};
use bpm_caddy::postes::{self, Posts};

uniffi::setup_scaffolding!();

/// Le port où les postes s'annoncent et tiennent leur porte, quand
/// l'officine n'en a pas écrit d'autre.
pub const DEFAULT_PORT: u16 = postes::DEFAULT_PORT;

/// Combien de temps le téléphone écoute les annonces des postes avant de
/// composer : elles partent toutes les trois secondes.
const LISTEN: Duration = Duration::from_millis(3_500);

/// La patience d'une conversation avec un poste.
const TALK: Duration = Duration::from_secs(8);

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum CaddyError {
    /// Une phrase en français, prête à montrer.
    #[error("{reason}")]
    Failed { reason: String },
}

impl From<String> for CaddyError {
    fn from(raw: String) -> Self {
        CaddyError::Failed {
            reason: bpm_caddy::strings::plain_error(&raw),
        }
    }
}

type Result<T> = std::result::Result<T, CaddyError>;

/// Un texte de l'interface, lu dans `assets/strings.fr.toml` comme sur
/// le bureau : l'application Android n'écrit pas ses libellés elle-même.
/// Une clé inconnue revient telle quelle, pour se voir.
#[uniffi::export]
pub fn tr(key: String) -> String {
    bpm_caddy::strings::shipped()
        .get(&key)
        .cloned()
        .unwrap_or(key)
}

/// Une date ISO affichée `JJ/MM/AAAA`.
#[uniffi::export]
pub fn display_date(iso: String) -> String {
    bpm_caddy::db::format_french_date(&iso)
}

/// Un horodatage de la base (`AAAA-MM-JJ HH:MM:SS`) affiché
/// `JJ/MM/AAAA HH:MM`.
fn stamp(at: &str) -> String {
    match at.split_once(' ') {
        Some((day, time)) => format!(
            "{} {}",
            bpm_caddy::db::format_french_date(day),
            time.get(..5).unwrap_or(time)
        ),
        None => bpm_caddy::db::format_french_date(at),
    }
}

/// Où en est ce téléphone.
#[derive(uniffi::Record)]
pub struct Status {
    /// Il a rejoint un groupe.
    pub in_group: bool,
    /// L'empreinte du téléphone, en cinq groupes.
    pub device: String,
    /// L'empreinte du groupe, en cinq groupes ; vide hors groupe.
    pub group: String,
    /// Les postes du groupe, compagnons compris, retirés exclus.
    pub posts: Vec<PostInfo>,
    pub cards: u32,
}

#[derive(uniffi::Record)]
pub struct PostInfo {
    pub number: i64,
    pub name: String,
    pub companion: bool,
    pub me: bool,
}

/// Une fiche trouvée.
#[derive(uniffi::Record)]
pub struct CardHit {
    pub id: i64,
    pub name: String,
    pub dci: String,
    pub class: String,
}

/// Une fiche, telle que le bureau la montre : l'en-tête, puis chaque
/// section de prose qui porte quelque chose, dans l'ordre du bureau.
#[derive(uniffi::Record)]
pub struct Card {
    pub id: i64,
    pub name: String,
    pub dci: String,
    pub class: String,
    pub status: String,
    pub sections: Vec<Section>,
}

#[derive(uniffi::Record)]
pub struct Section {
    /// La clé de la section (`MONO_FIELDS`) : ce que l'écriture nomme.
    pub key: String,
    pub label: String,
    pub text: String,
}

#[derive(uniffi::Record)]
pub struct AgendaItem {
    pub id: i64,
    /// ISO.
    pub day: String,
    pub time: String,
    pub end_time: String,
    pub title: String,
    pub category: String,
}

#[derive(uniffi::Record)]
pub struct ShiftItem {
    pub day: String,
    pub operator: String,
    pub start_time: String,
    pub end_time: String,
    pub kind: String,
    pub note: String,
}

#[derive(uniffi::Record)]
pub struct Talk {
    pub id: i64,
    pub title: String,
    pub members: Vec<String>,
    /// Le dernier message, pour la liste.
    pub last: String,
    pub last_at: String,
    /// Ce qui reste à lire pour qui tient le téléphone — les mêmes marques
    /// que sur les postes, sous ses initiales.
    pub unread: u32,
}

#[derive(uniffi::Record)]
pub struct Said {
    pub id: i64,
    pub author: String,
    pub body: String,
    pub sent_at: String,
    /// Le message cite un dossier : le téléphone ne l'ouvre pas.
    pub cites_patient: bool,
}

/// Une nature d'entrée d'agenda : la clé rangée, le libellé montré.
#[derive(uniffi::Record)]
pub struct Choice {
    pub key: String,
    pub label: String,
}

/// Les natures d'une entrée d'agenda, dans l'ordre du bureau.
#[uniffi::export]
pub fn event_categories() -> Vec<Choice> {
    bpm_caddy::db::EventCategory::ALL
        .iter()
        .map(|c| Choice {
            key: c.as_str().to_owned(),
            label: c.label().to_owned(),
        })
        .collect()
}

/// Ce qu'un passage par le dossier d'échange a fait.
#[derive(uniffi::Record)]
pub struct FolderDone {
    /// Le fichier de ce téléphone, à recopier dans le dossier choisi.
    pub mine: String,
    pub said: String,
}

/// Ce qu'une synchronisation a fait.
#[derive(uniffi::Record)]
pub struct SyncDone {
    /// Combien de postes ont répondu.
    pub reached: u32,
    pub sent: u32,
    pub written: u32,
    pub conflicts: u32,
    /// La phrase à montrer.
    pub said: String,
}

/// La base du téléphone, et ce qui l'ouvre.
#[derive(uniffi::Object)]
pub struct Caddy {
    path: PathBuf,
    password: String,
    db: Mutex<Db>,
    /// Les fiches, lues une fois et relues après une synchronisation :
    /// la recherche les parcourt à chaque lettre tapée.
    cards: Mutex<Option<Arc<Vec<Drug>>>>,
}

#[uniffi::export]
impl Caddy {
    /// Ouvrir (ou créer) la base du téléphone dans `dir`, chiffrée sous
    /// `password` — une clé que l'application garde dans le Keystore
    /// Android, jamais tapée.
    #[uniffi::constructor]
    pub fn open(dir: String, password: String) -> Result<Arc<Self>> {
        let path = PathBuf::from(dir).join("bpm-caddy.db");
        let db = Db::open(&path, &password)?;
        Ok(Arc::new(Self {
            path,
            password,
            db: Mutex::new(db),
            cards: Mutex::new(None),
        }))
    }

    pub fn status(&self) -> Result<Status> {
        let db = self.db();
        let posts = Posts::load(&db)?;
        let me = posts.device_hex();
        let list = if posts.in_group() {
            db.sync_posts()?
                .into_iter()
                .filter(|p| p.left_on.is_empty())
                .map(|p| PostInfo {
                    number: p.post,
                    me: p.device == me,
                    name: p.name,
                    companion: p.companion,
                })
                .collect()
        } else {
            Vec::new()
        };
        // The cards take the base's lock themselves.
        drop(db);
        Ok(Status {
            in_group: posts.in_group(),
            device: posts.groups(),
            group: posts.group_groups().unwrap_or_default(),
            posts: list,
            cards: self.cards().len() as u32,
        })
    }

    /// **Rejoindre l'officine** avec le code qu'affiche « Inviter un
    /// téléphone… » sur un poste : le code seul (l'invitation s'annonce
    /// sur le Wi-Fi de l'officine, le téléphone l'y entend) ou le code
    /// complet `…@adresse:port`. `name` : le nom du téléphone dans la
    /// liste des postes. Ce que la base contenait est remplacé.
    pub fn join(&self, code: String, name: String, today: String, port: u16) -> Result<String> {
        let heard = if code.contains('@') {
            Vec::new()
        } else {
            listen(port, LISTEN)
        };
        let inviting: Vec<(&str, &str)> = heard
            .iter()
            .filter_map(|h| {
                h.invites
                    .as_ref()
                    .filter(|(target, _)| target == "*")
                    .map(|(_, tag)| (h.address.as_str(), tag.as_str()))
            })
            .collect();
        let (address, ticket) = postes::join_target(&code, &inviting).map_err(|e| {
            use bpm_caddy::strings::tr;
            CaddyError::Failed {
                reason: tr(match e {
                    postes::JoinText::Unreadable => "mobile_join_unreadable",
                    postes::JoinText::NoAddress => "mobile_join_no_invite",
                    postes::JoinText::NoMatch => "mobile_join_no_match",
                })
                .to_owned(),
            }
        })?;
        let db = self.db();
        // Un code d'invitation porte déjà la preuve : rien à comparer.
        let (post, report) =
            postes::join(&db, &address, &name, ticket, true, &today, &mut |_| false)?;
        drop(db);
        self.forget_cards();
        Ok(bpm_caddy::strings::trn(
            "posts_done_joined",
            &[&(post + 1), &report.received.written],
        ))
    }

    /// **Synchroniser** : écouter les postes du groupe sur le Wi-Fi, leur
    /// parler à chacun, plus les adresses écrites ; puis ranger ce qui est
    /// arrivé. Sur sa propre connexion à la base, pour que l'écran puisse
    /// lire pendant ce temps.
    pub fn sync(&self, today: String, port: u16, addresses: Vec<String>) -> Result<SyncDone> {
        let db = Db::open(&self.path, &self.password)?;
        let mut posts = Posts::load(&db)?;
        if !posts.in_group() {
            return Err(CaddyError::Failed {
                reason: bpm_caddy::strings::tr("posts_err_no_group").to_owned(),
            });
        }
        let sent = posts.publish(&db, &today)?;
        let me = posts.device_hex();
        let members: Vec<bpm_caddy::db::PostRow> = db
            .sync_posts()?
            .into_iter()
            .filter(|p| p.left_on.is_empty() && p.device != me && !p.companion)
            .collect();
        let heard = listen(port, LISTEN);
        // Each post once: on the Wi-Fi when it is heard there, otherwise
        // at the addresses it publishes when reachable from the internet
        // (`[postes] internet`) — the first that answers. Then the
        // addresses written in the phone's settings.
        let mut reached = 0u32;
        let mut failed: Vec<String> = Vec::new();
        let mut targets = 0usize;
        let mut tried: Vec<String> = Vec::new();
        for p in &members {
            let mut ways: Vec<String> = heard
                .iter()
                .filter(|h| h.device == p.device)
                .map(|h| h.address.clone())
                .collect();
            if ways.is_empty() {
                ways = bpm_caddy::reach::addresses(&p.reach);
            }
            if ways.is_empty() {
                continue;
            }
            targets += 1;
            let mut last = String::new();
            let mut ok = false;
            for address in ways {
                tried.push(address.clone());
                match posts.sync_with(&db, &address, TALK) {
                    Ok(()) => {
                        ok = true;
                        break;
                    }
                    Err(e) => last = format!("{address} ({})", bpm_caddy::strings::plain_error(&e)),
                }
            }
            if ok {
                reached += 1;
            } else {
                failed.push(last);
            }
        }
        for a in addresses {
            let a = a.trim().to_owned();
            if a.is_empty() || tried.contains(&a) {
                continue;
            }
            targets += 1;
            match posts.sync_with(&db, &a, TALK) {
                Ok(()) => reached += 1,
                Err(e) => failed.push(format!("{a} ({})", bpm_caddy::strings::plain_error(&e))),
            }
        }
        let report = posts.absorb(&db, &today)?;
        drop(db);
        self.forget_cards();
        use bpm_caddy::strings::{tr, trf, trn};
        let mut said = if targets == 0 {
            tr("mobile_sync_nobody").to_owned()
        } else {
            trn(
                "mobile_sync_done",
                &[&reached, &(sent as u32), &report.received.written],
            )
        };
        if !failed.is_empty() {
            said.push_str(&trf("posts_done_unreached", failed.join(", ")));
        }
        Ok(SyncDone {
            reached,
            sent: sent as u32,
            written: report.received.written as u32,
            conflicts: report.received.conflicts as u32,
            said,
        })
    }

    /// **Le dossier d'échange, sur le téléphone** : `dir` est une copie
    /// locale que l'application fait du dossier choisi (Android ne donne
    /// pas de chemin, seulement des documents) — les fichiers `.bpmposte`
    /// des postes. Ce téléphone y dépose le sien, lit les autres, range.
    /// Rend le nom du fichier à recopier dans le dossier choisi.
    pub fn exchange_folder(&self, dir: String, today: String) -> Result<FolderDone> {
        let db = Db::open(&self.path, &self.password)?;
        let mut posts = Posts::load(&db)?;
        if !posts.in_group() {
            return Err(CaddyError::Failed {
                reason: bpm_caddy::strings::tr("posts_err_no_group").to_owned(),
            });
        }
        let sent = posts.publish(&db, &today)?;
        let added = posts.exchange_folder(&db, std::path::Path::new(&dir))?;
        let report = posts.absorb(&db, &today)?;
        drop(db);
        self.forget_cards();
        Ok(FolderDone {
            mine: posts.folder_file(),
            said: bpm_caddy::strings::trn(
                "mobile_folder_done",
                &[&sent, &added, &report.received.written],
            ),
        })
    }

    /// Les fiches qui répondent à `query` — nom, DCI, classe, étiquettes
    /// — les meilleures d'abord. Vide : toutes, dans l'ordre alphabétique.
    pub fn search_cards(&self, query: String, limit: u32) -> Vec<CardHit> {
        let cards = self.cards();
        let q = query.trim();
        let mut found: Vec<(i32, &Drug)> = cards
            .iter()
            .filter_map(|d| {
                if q.is_empty() {
                    return Some((0, d));
                }
                [&d.name, &d.dci, &d.class, &d.tags]
                    .iter()
                    .filter_map(|f| bpm_caddy::fuzzy::score(q, f))
                    .max()
                    .map(|s| (s, d))
            })
            .collect();
        found.sort_by(|a, b| {
            b.0.cmp(&a.0).then_with(|| {
                bpm_caddy::fuzzy::sort_key(&a.1.name).cmp(&bpm_caddy::fuzzy::sort_key(&b.1.name))
            })
        });
        found
            .into_iter()
            .take(limit as usize)
            .map(|(_, d)| CardHit {
                id: d.id,
                name: d.name.clone(),
                dci: d.dci.clone(),
                class: d.class.clone(),
            })
            .collect()
    }

    pub fn card(&self, id: i64) -> Option<Card> {
        let cards = self.cards();
        let d = cards.iter().find(|d| d.id == id)?;
        Some(Card {
            id: d.id,
            name: d.name.clone(),
            dci: d.dci.clone(),
            class: d.class.clone(),
            status: d.status.clone(),
            sections: MONO_FIELDS
                .iter()
                .filter(|(_, get)| !get(d).trim().is_empty())
                .map(|(key, get)| Section {
                    key: (*key).to_owned(),
                    label: bpm_caddy::strings::tr(key).to_owned(),
                    text: get(d).trim().to_owned(),
                })
                .collect(),
        })
    }

    /// **Récrire une section d'une fiche**, par le même chemin que le
    /// bureau (`update_drug_by` : comparer-et-écrire sur la fiche entière,
    /// journal des modifications au nom de `operator`). Seulement si la
    /// section dit encore ce que le téléphone montrait : `false`, la fiche
    /// a changé ailleurs — elle est relue, rien n'est écrasé.
    pub fn edit_card_section(
        &self,
        id: i64,
        key: String,
        shown: String,
        text: String,
        operator: String,
    ) -> Result<bool> {
        let db = self.db();
        let Some(current) = db.drugs()?.into_iter().find(|d| d.id == id) else {
            return Ok(false);
        };
        let mut next = current.clone();
        let Some(slot) = bpm_caddy::db::mono_field_mut(&mut next, &key) else {
            return Ok(false);
        };
        if slot.trim() != shown.trim() {
            return Ok(false);
        }
        *slot = text.trim().to_owned();
        let written = db.update_drug_by(&next, &current, operator.trim(), false)?;
        drop(db);
        self.forget_cards();
        Ok(written)
    }

    /// L'agenda de `from` à `to` (ISO, bornes comprises), répétitions
    /// dépliées comme sur le bureau.
    pub fn agenda(&self, from: String, to: String) -> Result<Vec<AgendaItem>> {
        Ok(self
            .db()
            .events_between(&from, &to)?
            .into_iter()
            .map(|e| AgendaItem {
                id: e.id,
                day: e.day,
                time: e.time,
                end_time: e.end_time,
                title: e.title,
                category: e.category.label().to_owned(),
            })
            .collect())
    }

    /// **Ajouter une entrée à l'agenda**, comme sur le bureau : le jour
    /// tapé à la française (`260926`, `2609`, `26/09/2026`), les heures
    /// à la volée (`9h30`), un titre, une nature. `today` donne l'année
    /// d'un jour tapé sans elle. Part à la synchronisation suivante.
    pub fn add_event(
        &self,
        day: String,
        time: String,
        end_time: String,
        title: String,
        category: String,
        today: String,
    ) -> Result<i64> {
        use bpm_caddy::db::{parse_french_date, parse_hour, EventCategory, YearHint};
        use bpm_caddy::strings::tr;
        let year = today.get(..4).and_then(|y| y.parse().ok()).unwrap_or(2026);
        let day = parse_french_date(&day, year, YearHint::Future)?;
        let hour = |text: &str| -> Result<String> {
            if text.trim().is_empty() {
                return Ok(String::new());
            }
            parse_hour(text).ok_or_else(|| CaddyError::Failed {
                reason: tr("mobile_agenda_bad_hour").to_owned(),
            })
        };
        let (time, end_time) = (hour(&time)?, hour(&end_time)?);
        let title = title.trim();
        if title.is_empty() {
            return Err(CaddyError::Failed {
                reason: tr("mobile_agenda_no_title").to_owned(),
            });
        }
        let category = EventCategory::parse(&category).unwrap_or(EventCategory::Autre);
        Ok(self
            .db()
            .add_event_span(&day, &time, &end_time, title, category, "", "")?)
    }

    /// **Retirer une entrée**, seulement si elle porte encore le titre
    /// que le téléphone montrait : une entrée qu'un collègue vient de
    /// changer n'est pas détruite. `false` : l'agenda a changé, à relire.
    pub fn delete_event(&self, id: i64, shown_title: String) -> Result<bool> {
        Ok(self.db().delete_event(id, &shown_title)?)
    }

    /// Le planning de `from` à `to` : qui travaille quand. En lecture.
    pub fn planning(&self, from: String, to: String) -> Result<Vec<ShiftItem>> {
        Ok(self
            .db()
            .shifts_between(&from, &to)?
            .into_iter()
            .map(|s| ShiftItem {
                day: s.day,
                operator: s.operator,
                start_time: s.start_time,
                end_time: s.end_time,
                kind: s.kind,
                note: s.note,
            })
            .collect())
    }

    /// Les conversations de l'équipe, la plus récente d'abord. Celles
    /// avec d'autres officines passent par le réseau, que le téléphone
    /// n'a pas : elles n'y sont pas.
    pub fn conversations(&self, operator: String) -> Result<Vec<Talk>> {
        let db = self.db();
        let unread = if operator.trim().is_empty() {
            Default::default()
        } else {
            db.unread_counts(&operator)?
        };
        let mut out = Vec::new();
        for c in db.conversations()? {
            if c.channel != bpm_caddy::messages::Channel::Equipe {
                continue;
            }
            let said = db.conversation_messages(c.id)?;
            let (last, last_at) = said
                .last()
                .map(|m| (format!("{} : {}", m.author, m.body), stamp(&m.sent_at)))
                .unwrap_or_default();
            out.push(Talk {
                unread: unread.get(&c.id).copied().unwrap_or(0) as u32,
                id: c.id,
                title: c.title,
                members: c.members,
                last,
                last_at,
            });
        }
        Ok(out)
    }

    pub fn messages(&self, conversation: i64) -> Result<Vec<Said>> {
        Ok(self
            .db()
            .conversation_messages(conversation)?
            .into_iter()
            .map(|m| Said {
                id: m.id,
                author: m.author,
                body: m.body,
                sent_at: stamp(&m.sent_at),
                cites_patient: m.patient_id.is_some(),
            })
            .collect())
    }

    /// Noter la conversation lue par `operator` jusqu'à son dernier
    /// message — la marque que les postes lisent aussi. Sans initiales,
    /// rien.
    pub fn mark_read(&self, conversation: i64, operator: String) -> Result<()> {
        if operator.trim().is_empty() {
            return Ok(());
        }
        let db = self.db();
        if let Some(last) = db.conversation_messages(conversation)?.last() {
            db.mark_read(conversation, &operator, &last.sent_at)?;
        }
        Ok(())
    }

    /// Écrire dans une conversation de l'équipe. `author` : les initiales
    /// de qui tient le téléphone. Part à la synchronisation suivante.
    pub fn send_message(&self, conversation: i64, author: String, body: String) -> Result<()> {
        let body = body.trim();
        if body.is_empty() {
            return Ok(());
        }
        self.db()
            .post_message(conversation, "", author.trim(), body, None, "", None)?;
        Ok(())
    }
}

impl Caddy {
    fn db(&self) -> std::sync::MutexGuard<'_, Db> {
        // A panic while holding the base leaves it as it was: SQLite's
        // own transactions guard the data, not this lock.
        self.db.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn cards(&self) -> Arc<Vec<Drug>> {
        let mut slot = self.cards.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(c) = slot.as_ref() {
            return Arc::clone(c);
        }
        let read = Arc::new(self.db().drugs().unwrap_or_default());
        *slot = Some(Arc::clone(&read));
        read
    }

    fn forget_cards(&self) {
        *self.cards.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }
}

/// Les postes qui s'annoncent sur le réseau local pendant `patience`.
fn listen(port: u16, patience: Duration) -> Vec<postes::Heard> {
    let Ok(socket) = UdpSocket::bind(("0.0.0.0", port)) else {
        return Vec::new();
    };
    let until = Instant::now() + patience;
    let mut out: Vec<postes::Heard> = Vec::new();
    let mut buffer = [0u8; 512];
    loop {
        let left = until.saturating_duration_since(Instant::now());
        if left.is_zero() {
            break;
        }
        let _ = socket.set_read_timeout(Some(left.max(Duration::from_millis(10))));
        let Ok((n, from)) = socket.recv_from(&mut buffer) else {
            continue;
        };
        let Ok(text) = std::str::from_utf8(&buffer[..n]) else {
            continue;
        };
        if let Some(h) = postes::heard(text, from.ip()) {
            if let Some(same) = out.iter_mut().find(|o| o.device == h.device) {
                // The latest word from a post: an invitation opened since.
                *same = h;
            } else if out.len() < 64 {
                out.push(h);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("bpm-mobile-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// A phone that has joined nothing opens, says so, and refuses to
    /// sync rather than pretend.
    #[test]
    fn a_fresh_phone_opens_and_is_in_no_group() {
        let d = dir("fresh");
        let c = Caddy::open(d.display().to_string(), "k".into()).unwrap();
        let s = c.status().unwrap();
        assert!(!s.in_group);
        assert!(s.posts.is_empty());
        assert!(c.sync("2026-09-26".into(), 0, vec![]).is_err());
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Every `mobile_` key the bridge and the Kotlin screens name exists
    /// — a missing one would show as its key on the phone.
    #[test]
    fn every_string_the_phone_names_exists() {
        const SOURCES: &[&str] = &[
            include_str!("lib.rs"),
            include_str!(
                "../../android/app/src/main/java/io/github/youssefsahli/bpmcaddy/CompanionApp.kt"
            ),
            include_str!(
                "../../android/app/src/main/java/io/github/youssefsahli/bpmcaddy/MainActivity.kt"
            ),
            include_str!(
                "../../android/app/src/main/java/io/github/youssefsahli/bpmcaddy/AppState.kt"
            ),
            include_str!(
                "../../android/app/src/main/java/io/github/youssefsahli/bpmcaddy/Folder.kt"
            ),
        ];
        let mut missing = Vec::new();
        for src in SOURCES {
            let mut at = 0;
            while let Some(i) = src[at..].find("\"mobile_") {
                let start = at + i + 1;
                let end = start + src[start..].find('"').unwrap();
                let key = &src[start..end];
                // A prefix being assembled is not a key.
                if !key.ends_with('_') && tr(key.to_owned()) == key {
                    missing.push(key.to_owned());
                }
                at = end;
            }
        }
        assert!(missing.is_empty(), "clés absentes : {missing:?}");
    }

    /// An entry typed the counter's way lands on the agenda, and is
    /// removed only while it still says what the phone showed.
    #[test]
    fn an_agenda_entry_is_added_the_desktop_way_and_removed_by_what_was_shown() {
        let d = dir("agenda");
        let c = Caddy::open(d.display().to_string(), "k".into()).unwrap();
        let id = c
            .add_event(
                "2909".into(),
                "11h".into(),
                "12".into(),
                "Formation vaccination".into(),
                "FORMATION".into(),
                "2026-09-26".into(),
            )
            .unwrap();
        let week = c.agenda("2026-09-28".into(), "2026-10-04".into()).unwrap();
        assert_eq!(week.len(), 1);
        assert_eq!(
            (
                week[0].day.as_str(),
                week[0].time.as_str(),
                week[0].end_time.as_str()
            ),
            ("2026-09-29", "11:00", "12:00")
        );
        assert!(c
            .add_event(
                "32/13".into(),
                "".into(),
                "".into(),
                "x".into(),
                "".into(),
                "2026-09-26".into()
            )
            .is_err());
        assert!(c
            .add_event(
                "2909".into(),
                "".into(),
                "".into(),
                "  ".into(),
                "".into(),
                "2026-09-26".into()
            )
            .is_err());
        assert!(!c.delete_event(id, "Autre titre".into()).unwrap());
        assert!(c.delete_event(id, "Formation vaccination".into()).unwrap());
        assert_eq!(event_categories().len(), 5);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn a_stamp_reads_in_the_french_order() {
        assert_eq!(stamp("2026-09-26 08:45:47"), "26/09/2026 08:45");
        assert_eq!(stamp("2026-09-26"), "26/09/2026");
    }

    /// The card reads the prose the desktop reads, field by field.
    #[test]
    fn a_card_reads_its_prose_in_the_desktop_order() {
        let d = dir("card");
        let c = Caddy::open(d.display().to_string(), "k".into()).unwrap();
        let id = c.db().add_drug("Amoxicilline").unwrap();
        let was = c.cards().iter().find(|d| d.id == id).unwrap().clone();
        let mut now = was.clone();
        now.indications = "Angine".into();
        now.adverse = "Rash".into();
        assert!(c.db().update_drug(&now, &was).unwrap());
        c.forget_cards();
        let hits = c.search_cards("amox".into(), 10);
        assert_eq!(hits.first().map(|h| h.id), Some(id));
        let card = c.card(id).unwrap();
        let labels: Vec<&str> = card.sections.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(labels, vec!["Angine", "Rash"]);
        // A section rewritten from what the phone showed; one it no longer
        // shows is refused and nothing is lost.
        let key = card.sections[1].key.clone();
        assert!(c
            .edit_card_section(
                id,
                key.clone(),
                "Rash".into(),
                "Éruption".into(),
                "AB".into()
            )
            .unwrap());
        assert!(!c
            .edit_card_section(id, key, "Rash".into(), "Autre".into(), "AB".into())
            .unwrap());
        assert_eq!(c.card(id).unwrap().sections[1].text, "Éruption");
        let _ = std::fs::remove_dir_all(&d);
    }
}
