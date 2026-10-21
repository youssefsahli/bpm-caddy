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

/// Le temps qu'une conversation par un relais peut durer en tout.
const RELAYED_BOUND: Duration = Duration::from_secs(120);

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum CaddyError {
    /// Une phrase en français, prête à montrer.
    #[error("{reason}")]
    Failed { reason: String },
}

/// Des initiales comme les postes de bureau les écrivent : sans espace,
/// en majuscules. Tapées « cl » au téléphone, elles signaient les messages
/// et rangeaient les marques de lecture sous « cl », que le bureau, qui lit
/// « CL », ne retrouvait pas.
fn initials(raw: &str) -> String {
    raw.trim().to_uppercase()
}

/// **Une conversation de l'équipe, ou rien.** Le téléphone ne liste que
/// celles-là, mais lire, enregistrer un fichier, marquer lu ou écrire
/// prenaient n'importe quel numéro : un message écrit dans une
/// conversation entre officines partait au réseau sous le nom de
/// l'officine à la synchronisation suivante du bureau.
fn team_only(db: &bpm_caddy::db::Db, conversation: i64) -> Result<()> {
    let team = db
        .conversations()?
        .into_iter()
        .any(|c| c.id == conversation && c.channel == bpm_caddy::messages::Channel::Equipe);
    if team {
        Ok(())
    } else {
        Err(CaddyError::Failed {
            reason: bpm_caddy::strings::tr("mobile_not_team").to_owned(),
        })
    }
}

impl From<String> for CaddyError {
    fn from(raw: String) -> Self {
        CaddyError::Failed {
            reason: bpm_caddy::strings::plain_error(&raw),
        }
    }
}

type Result<T> = std::result::Result<T, CaddyError>;

/// Une erreur dite par une clé de `strings.fr.toml`.
fn failed(key: &'static str) -> CaddyError {
    CaddyError::Failed {
        reason: bpm_caddy::strings::tr(key).to_owned(),
    }
}

/// Une entrée d'agenda tapée sur le téléphone : le jour à la française
/// (`260926`, `2609`, `26/09/2026`, l'année de `today` à défaut), les
/// heures à la volée (`9h30`), un titre non vide. Rend (jour ISO, début,
/// fin, titre).
fn typed_event(
    day: &str,
    time: &str,
    end_time: &str,
    title: &str,
    today: &str,
) -> Result<(String, String, String, String)> {
    use bpm_caddy::db::{parse_french_date, parse_hour, YearHint};
    use bpm_caddy::strings::tr;
    let year = today.get(..4).and_then(|y| y.parse().ok()).unwrap_or(2026);
    let day = parse_french_date(day, year, YearHint::Future)?;
    let hour = |text: &str| -> Result<String> {
        if text.trim().is_empty() {
            return Ok(String::new());
        }
        parse_hour(text).ok_or_else(|| CaddyError::Failed {
            reason: tr("mobile_agenda_bad_hour").to_owned(),
        })
    };
    let (time, end_time) = (hour(time)?, hour(end_time)?);
    let title = title.trim();
    if title.is_empty() {
        return Err(CaddyError::Failed {
            reason: tr("mobile_agenda_no_title").to_owned(),
        });
    }
    Ok((day, time, end_time, title.to_owned()))
}

/// Les heures d'un poste tapées au téléphone : le début requis, la fin
/// facultative (`db::shift_end`, le lendemain quand elle est plus tôt), la
/// pause en minutes.
fn typed_shift(start: &str, end: &str, pause: &str) -> Result<(String, String, i64)> {
    let start_time =
        bpm_caddy::db::parse_hour(start).ok_or_else(|| failed("planning_needs_an_hour"))?;
    if !end.trim().is_empty() && bpm_caddy::db::parse_hour(end).is_none() {
        return Err(failed("mobile_agenda_bad_hour"));
    }
    let pause = match pause.trim() {
        "" => 0,
        typed => typed
            .parse::<i64>()
            .ok()
            .filter(|p| *p >= 0)
            .ok_or_else(|| failed("planning_bad_pause"))?,
    };
    Ok((start_time, bpm_caddy::db::shift_end(start, end), pause))
}

/// Le nom d'un compte et le début de son empreinte, ou l'empreinte seule
/// quand il ne se nomme pas ici.
fn name_in(
    named: &std::collections::BTreeMap<String, bpm_caddy::compte::Profile>,
    account: &str,
) -> String {
    use bpm_caddy::compte;
    match named.get(account) {
        Some(p) => {
            bpm_caddy::strings::trn("compte_named", &[&p.nom, &compte::short_groups(account)])
        }
        None => bpm_caddy::strings::trf("compte_unnamed", compte::groups_of(account)),
    }
}

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

#[derive(uniffi::Record, Clone)]
pub struct ShiftItem {
    /// La ligne rangée qu'on voit, et celle de la trame : ce que
    /// [`Caddy::edit_shift_day`] renvoie pour nommer ce jour.
    pub id: i64,
    pub source_id: i64,
    pub day: String,
    pub operator: String,
    pub start_time: String,
    pub end_time: String,
    pub pause_minutes: i64,
    pub kind: String,
    /// La nature telle que la base la range ; `kind` est son libellé.
    pub kind_key: String,
    pub note: String,
    /// Un jour d'une trame : le changer ne change que ce jour.
    pub repeats: bool,
    /// La fin telle qu'on la tape (« 2h » pour une garde rangée « 26:00 ») :
    /// ce que le formulaire reprend, et que la lecture des heures relit.
    pub end_typed: String,
    /// Un congé, un arrêt, une formation : pas d'heures à changer au
    /// téléphone — « pas là ce jour » y rendrait la journée qu'il cache.
    pub absence: bool,
}

/// Un billet du fil des actualités.
#[derive(uniffi::Record)]
pub struct NewsPost {
    /// L'élément et sa version : ce qu'un effacement nomme.
    pub item: String,
    pub seq: u64,
    pub author_id: String,
    /// Le nom de son auteur et l'empreinte qui le distingue, ou
    /// l'empreinte seule quand son profil n'est pas arrivé ou est discret.
    pub author: String,
    /// `JJ/MM/AAAA HH:MM`, ou le jour seul.
    pub written: String,
    pub text: String,
    /// Du compte ouvert.
    pub mine: bool,
    /// Scellé pour des comptes choisis, pas public.
    pub shared: bool,
}

/// Le fil, et combien de billets il tient à l'écart.
#[derive(uniffi::Record)]
pub struct NewsFeed {
    pub posts: Vec<NewsPost>,
    /// Les billets d'auteurs masqués ou bloqués par le compte ouvert.
    pub hidden: u32,
}

/// Un rendez-vous du compte ouvert : le sien, ou partagé avec lui —
/// montré dans l'agenda à part des entrées de l'officine, comme au bureau.
#[derive(uniffi::Record)]
pub struct AccountRdv {
    pub item: String,
    /// ISO.
    pub day: String,
    pub time: String,
    pub end_time: String,
    pub title: String,
    /// Qui l'a écrit, nommé comme au fil ; vide : le compte ouvert, qui
    /// l'efface (`delete_post`) contre cette version.
    pub author: String,
    pub seq: u64,
}

/// Un compte : gardé sur le téléphone, ou connu pour partager avec lui.
#[derive(uniffi::Record)]
pub struct AccountInfo {
    pub id: String,
    /// Son nom et son empreinte.
    pub name: String,
    /// `lire`, `masquer` ou `bloquer` : ce que le compte ouvert en a
    /// décidé.
    pub hold: String,
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
    /// Les fichiers joints.
    pub files: Vec<Attachment>,
}

/// Un fichier joint à un message.
#[derive(uniffi::Record)]
pub struct Attachment {
    pub uid: String,
    pub name: String,
    pub size: u64,
    /// Tous ses morceaux sont arrivés : il s'ouvre.
    pub complete: bool,
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

/// Les natures d'une absence, dans l'ordre du bureau : congé, maladie,
/// formation, récupération.
#[uniffi::export]
pub fn absence_kinds() -> Vec<Choice> {
    bpm_caddy::planning::ShiftKind::ALL
        .iter()
        .filter(|k| k.is_absence())
        .map(|k| Choice {
            key: k.as_str().to_owned(),
            label: k.label().to_owned(),
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
    /// Le compte ouvert sur le téléphone : en mémoire seulement, fermé
    /// avec l'application.
    account: Mutex<Option<bpm_caddy::compte::Account>>,
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
            account: Mutex::new(None),
        }))
    }

    pub fn status(&self) -> Result<Status> {
        let db = self.db();
        let posts = Posts::identity(&db)?;
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
        let (post, report) = postes::join(
            &db,
            &address,
            &postes::JoinAs {
                name: &name,
                ticket,
                expect: None,
                companion: true,
            },
            &today,
            &mut |_| false,
        )?;
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
            // Its relays last: a relay puts the phone in touch with any
            // post of the group waiting there, which is as good.
            let relays: Vec<String> = bpm_caddy::reach::relays(&p.reach)
                .into_iter()
                .filter(|r| !tried.contains(&format!("relais:{r}")))
                .collect();
            if ways.is_empty() && relays.is_empty() {
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
            if let (false, Some(group)) = (ok, posts.group_hex()) {
                for relay in relays {
                    tried.push(format!("relais:{relay}"));
                    let said = bpm_caddy::relay::call(&relay, &group, TALK)
                        .map_err(|_| "Link".to_owned())
                        .and_then(|s| {
                            bpm_sync::link::TcpLink::new(s, TALK)
                                .map(|l| l.with_deadline(RELAYED_BOUND))
                                .map_err(|e| format!("{e:?}"))
                        })
                        .and_then(|mut link| posts.sync_over(&db, &mut link));
                    match said {
                        Ok(()) => {
                            ok = true;
                            break;
                        }
                        Err(e) => {
                            last = format!("{relay} ({})", bpm_caddy::strings::plain_error(&e))
                        }
                    }
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
        // Aucun poste joint : ce qui est écrit attend, et ce n'est pas dit
        // « envoyé ».
        let mut said = if targets == 0 {
            tr("mobile_sync_nobody").to_owned()
        } else if reached == 0 {
            trf("mobile_sync_unreached", sent)
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
        // Une copie locale que l'application tient : la créer ici est
        // juste, contrairement à un partage qu'un poste trouverait démonté.
        std::fs::create_dir_all(&dir).map_err(|e| CaddyError::Failed {
            reason: e.to_string(),
        })?;
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
        let operator = initials(&operator);
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
        use bpm_caddy::db::EventCategory;
        let (day, time, end_time, title) = typed_event(&day, &time, &end_time, &title, &today)?;
        let category = EventCategory::parse(&category).unwrap_or(EventCategory::Autre);
        Ok(self
            .db()
            .add_event_span(&day, &time, &end_time, &title, category, "", "")?)
    }

    /// **Retirer une entrée**, seulement si elle porte encore le titre
    /// que le téléphone montrait : une entrée qu'un collègue vient de
    /// changer n'est pas détruite. `false` : l'agenda a changé, à relire.
    pub fn delete_event(&self, id: i64, shown_title: String) -> Result<bool> {
        // **Pas une série depuis le téléphone.** L'agenda y montre chaque
        // jour d'une entrée répétée sous le même identifiant, et l'appui
        // long sur « ce mardi » supprimait toutes les réunions, passées
        // et à venir, chez tous les postes — sans l'avertissement que le
        // poste de bureau donne.
        let db = self.db();
        if db.event_repeats(id)? {
            return Err(CaddyError::Failed {
                reason: bpm_caddy::strings::tr("mobile_event_series").to_owned(),
            });
        }
        Ok(db.delete_event(id, &shown_title)?)
    }

    /// Le planning de `from` à `to` : qui travaille quand. En lecture.
    pub fn planning(&self, from: String, to: String) -> Result<Vec<ShiftItem>> {
        Ok(self
            .db()
            .shifts_between(&from, &to)?
            .into_iter()
            .map(|s| ShiftItem {
                end_typed: bpm_caddy::planning::parse_bound(&s.end_time)
                    .map(bpm_caddy::planning::typed_hour)
                    .unwrap_or_default(),
                id: s.id,
                source_id: s.source_id,
                repeats: s.id != s.source_id || s.repeat_days > 0,
                day: s.day,
                operator: s.operator,
                start_time: s.start_time,
                end_time: s.end_time,
                pause_minutes: s.pause_minutes,
                // The label the desktop shows, not the key it stores; a
                // kind a later version adds reads as written.
                kind: bpm_caddy::planning::ShiftKind::parse(&s.kind)
                    .map(|k| k.label().to_owned())
                    .unwrap_or_else(|| s.kind.clone()),
                absence: bpm_caddy::planning::ShiftKind::parse(&s.kind)
                    .is_some_and(bpm_caddy::planning::ShiftKind::is_absence),
                kind_key: s.kind,
                note: s.note,
            })
            .collect())
    }

    /// **Ce jour, et seulement ce jour** : d'autres heures pour un poste
    /// du planning — rester jusqu'à 12 h 30, arriver à 8 h 30. Le jour
    /// d'une trame prend une exception et la trame ne bouge pas, comme au
    /// bureau (`Db::set_occurrence`). Les heures se tapent à la volée
    /// (`9h30`) ; une fin plus tôt que le début est le lendemain matin ;
    /// la pause en minutes, vide : aucune. `false` : le jour a changé
    /// ailleurs depuis l'écran, à relire.
    pub fn edit_shift_day(
        &self,
        shown: ShiftItem,
        start_time: String,
        end_time: String,
        pause: String,
    ) -> Result<bool> {
        let (start, end, pause) = typed_shift(&start_time, &end_time, &pause)?;
        self.set_shift_day(&shown, Some((&start, &end, pause)))
    }

    /// **Pas là ce jour** : le jour d'une trame n'a pas lieu, la trame
    /// reste ; un poste d'un jour est ôté. `false` : à relire.
    pub fn cancel_shift_day(&self, shown: ShiftItem) -> Result<bool> {
        self.set_shift_day(&shown, None)
    }

    /// **Un jour de plus** : un poste d'un seul jour pour `operator`
    /// (ses initiales), une journée travaillée. Le jour tapé à la
    /// française, `today` donne l'année d'un jour tapé sans elle.
    pub fn add_shift_day(
        &self,
        operator: String,
        day: String,
        start_time: String,
        end_time: String,
        pause: String,
        today: String,
    ) -> Result<i64> {
        use bpm_caddy::db::{parse_french_date, YearHint};
        let operator = initials(&operator);
        if operator.trim().is_empty() {
            return Err(failed("mobile_planning_who"));
        }
        let year = today.get(..4).and_then(|y| y.parse().ok()).unwrap_or(2026);
        let day = parse_french_date(&day, year, YearHint::Future)?;
        let (start, end, pause) = typed_shift(&start_time, &end_time, &pause)?;
        Ok(self.db().add_shift(&bpm_caddy::db::NewShift {
            operator,
            day,
            start_time: start,
            end_time: end,
            pause_minutes: pause,
            kind: bpm_caddy::planning::ShiftKind::Journee.as_str().to_owned(),
            cadence: bpm_caddy::planning::Cadence::Unique.as_str().to_owned(),
            ..Default::default()
        })?)
    }

    /// **Une absence**, du premier au dernier jour (tapés à la française,
    /// `today` donne l'année d'un jour tapé sans elle), pour `operator`
    /// (ses initiales) : une ligne, comme le bureau la pose
    /// (`Db::add_absence`) — `kind`, la clé d'une absence.
    pub fn add_absence(
        &self,
        operator: String,
        kind: String,
        first: String,
        last: String,
        today: String,
    ) -> Result<i64> {
        use bpm_caddy::db::{parse_french_date, YearHint};
        let operator = initials(&operator);
        if operator.trim().is_empty() {
            return Err(failed("mobile_planning_who"));
        }
        let kind = bpm_caddy::planning::ShiftKind::parse(&kind)
            .filter(|k| k.is_absence())
            .ok_or_else(|| failed("planning_leave_invalid"))?;
        let year = today.get(..4).and_then(|y| y.parse().ok()).unwrap_or(2026);
        let first = parse_french_date(&first, year, YearHint::Future)?;
        let last = parse_french_date(&last, year, YearHint::Future)?;
        Ok(self.db().add_absence(&operator, kind, &first, &last)?)
    }

    /// **Le fil des actualités**, dans l'ordre du bureau (`compte::feed`),
    /// au plus `limit` : les billets publics des comptes, de cette officine
    /// et des autres, et — compte ouvert — ceux qu'on partage avec lui,
    /// moins ceux des auteurs qu'il a masqués ou bloqués. `today` (ISO)
    /// range un billet daté de demain.
    pub fn news(&self, today: String, limit: u32) -> Result<NewsFeed> {
        use bpm_caddy::compte::{self, State};
        // Ce que les postes passent garde chaque version qu'ils ont eue
        // un moment : le téléphone compacte aussi, sans rien relire tant
        // que la table n'a pas plus de lignes que d'éléments.
        self.db().compact_account_records()?;
        let account = self.account();
        let latest = self.reads(account.as_ref())?;
        let me = account.as_ref().map(compte::Account::id);
        let settings = me
            .as_deref()
            .map(|me| compte::settings_for(&latest, me).0)
            .unwrap_or_default();
        let named = compte::named(&latest);
        let feed = compte::feed(&latest, &settings, &today, limit as usize);
        Ok(NewsFeed {
            hidden: feed.hidden as u32,
            posts: feed
                .posts
                .into_iter()
                .map(|(r, e)| NewsPost {
                    author: name_in(&named, &r.account),
                    written: if e.debut.is_empty() {
                        bpm_caddy::db::format_french_date(&e.jour)
                    } else {
                        format!("{} {}", bpm_caddy::db::format_french_date(&e.jour), e.debut)
                    },
                    text: e.texte,
                    mine: Some(&r.account) == me.as_ref(),
                    shared: !matches!(r.state, State::Public(_)),
                    item: r.item,
                    seq: r.seq,
                    author_id: r.account,
                })
                .collect(),
        })
    }

    /// **Les rendez-vous du compte ouvert**, du jour `from` au jour `to`
    /// (ISO) : les siens et ceux qu'on partage avec lui, par la règle du
    /// bureau (`compte::appointments`). Sans compte ouvert, aucun.
    pub fn account_agenda(&self, from: String, to: String) -> Result<Vec<AccountRdv>> {
        use bpm_caddy::compte;
        let Some(account) = self.account() else {
            return Ok(Vec::new());
        };
        let me = account.id();
        let latest = self.reads(Some(&account))?;
        let (settings, _) = compte::settings_for(&latest, &me);
        let named = compte::named(&latest);
        Ok(compte::appointments(&latest, &me, &settings, (&from, &to))
            .into_iter()
            .map(|(r, e)| AccountRdv {
                author: if r.account == me {
                    String::new()
                } else {
                    name_in(&named, &r.account)
                },
                title: if e.titre.trim().is_empty() {
                    e.texte
                } else {
                    e.titre
                },
                day: e.jour,
                time: e.debut,
                end_time: e.fin,
                item: r.item,
                seq: r.seq,
            })
            .collect())
    }

    /// Les comptes gardés sur ce téléphone, chacun sous son code.
    pub fn kept_accounts(&self) -> Result<Vec<AccountInfo>> {
        let latest = self.reads(None)?;
        let named = bpm_caddy::compte::named(&latest);
        Ok(self
            .db()
            .kept_accounts()?
            .into_iter()
            .map(|(id, _, _)| AccountInfo {
                name: name_in(&named, &id),
                id,
                hold: "lire".to_owned(),
            })
            .collect())
    }

    /// Le compte ouvert, s'il y en a un.
    pub fn open_account(&self) -> Option<AccountInfo> {
        let account = self.account()?;
        let id = account.id();
        // Lu par lui : un compte discret se nomme à lui-même.
        let latest = self.reads(Some(&account)).unwrap_or_default();
        Some(AccountInfo {
            name: name_in(&bpm_caddy::compte::named(&latest), &id),
            id,
            hold: "lire".to_owned(),
        })
    }

    /// **Ouvrir un compte gardé ici**, par son code.
    pub fn account_unlock(&self, id: String, pin: String) -> Result<AccountInfo> {
        let locked = self
            .db()
            .kept_accounts()?
            .into_iter()
            .find(|(a, _, _)| *a == id)
            .map(|(_, _, l)| l);
        let opened = locked
            .and_then(|l| bpm_caddy::compte::unlock(&l, &pin))
            .map(bpm_caddy::compte::Account::from_phrase)
            .filter(|a| a.id() == id)
            .ok_or_else(|| failed("compte_bad_pin"))?;
        *self.account.lock().unwrap_or_else(|e| e.into_inner()) = Some(opened);
        self.open_account()
            .ok_or_else(|| failed("compte_write_failed"))
    }

    /// **Retrouver un compte par sa phrase**, sous un code pour ce
    /// téléphone — la phrase n'y est gardée que scellée sous lui, comme
    /// sur un poste. Le compte se crée sur un poste : douze mots s'y
    /// recopient sur papier, pas sur l'écran d'un téléphone.
    pub fn account_recover(&self, words: String, pin: String) -> Result<AccountInfo> {
        use bpm_caddy::compte;
        let phrase =
            compte::Phrase::parse(&words).map_err(|e| CaddyError::Failed { reason: e.text() })?;
        if !compte::pin_ok(&pin) {
            return Err(failed("compte_pin_short"));
        }
        let account = compte::Account::from_phrase(phrase.clone());
        let id = account.id();
        let latest = self.reads(None)?;
        let label = compte::named(&latest)
            .get(&id)
            .map(|p| p.nom.clone())
            .unwrap_or_else(|| bpm_caddy::strings::trf("compte_unnamed", account.groups()));
        let locked = compte::lock(&phrase, &pin, compte::STRETCH, &mut bpm_sync::OsEntropy)
            .ok_or_else(|| failed("compte_write_failed"))?;
        self.db().keep_account(&id, &label, &locked)?;
        *self.account.lock().unwrap_or_else(|e| e.into_inner()) = Some(account);
        self.open_account()
            .ok_or_else(|| failed("compte_write_failed"))
    }

    /// Fermer le compte ouvert.
    pub fn account_close(&self) {
        *self.account.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }

    /// Retirer un compte du téléphone — rien de ce qu'il a écrit n'est
    /// effacé, et sa phrase le rouvre.
    pub fn account_forget(&self, id: String) -> Result<()> {
        if self.account().is_some_and(|a| a.id() == id) {
            self.account_close();
        }
        Ok(self.db().forget_account(&id)?)
    }

    /// Ceux avec qui le compte ouvert peut partager, et ce qu'il en a
    /// décidé : les comptes qui se nomment ici, lui excepté. Un compte
    /// bloqué y reste, pour qu'il se rétablisse ; il n'est pas proposé
    /// comme lecteur (`publish_post` le retire).
    pub fn people(&self) -> Result<Vec<AccountInfo>> {
        use bpm_caddy::compte::{self, Hold};
        let Some(account) = self.account() else {
            return Ok(Vec::new());
        };
        let latest = self.reads(Some(&account))?;
        let (settings, _) = compte::settings_for(&latest, &account.id());
        let mut out: Vec<AccountInfo> = compte::named(&latest)
            .into_iter()
            .filter(|(a, _)| *a != account.id())
            .map(|(a, p)| AccountInfo {
                name: bpm_caddy::strings::trn("compte_named", &[&p.nom, &compte::short_groups(&a)]),
                hold: match settings.hold(&a) {
                    Hold::Read => "lire",
                    Hold::Muted => "masquer",
                    Hold::Blocked => "bloquer",
                }
                .to_owned(),
                id: a,
            })
            .collect();
        out.sort_by_key(|p| p.name.to_lowercase());
        Ok(out)
    }

    /// **Publier un billet** du compte ouvert : public quand `readers` est
    /// vide, sinon scellé pour eux et pour lui. `today` (ISO), `time`
    /// (`HH:MM`) et `now` (secondes) viennent de l'appelant, qui a
    /// l'horloge. Jamais un patient : l'écran le dit là où l'on écrit.
    pub fn publish_post(
        &self,
        text: String,
        readers: Vec<String>,
        today: String,
        time: String,
        now: u64,
    ) -> Result<()> {
        use bpm_caddy::compte::{self, Kind, Visibility};
        if text.trim().is_empty() {
            return Err(failed("compte_post_empty"));
        }
        let entry = compte::Entry {
            texte: text.trim().to_owned(),
            jour: today,
            debut: time,
            ..compte::Entry::default()
        };
        self.write_own(Kind::Billet, entry, readers, Visibility::Public, now)
    }

    /// **Un rendez-vous dans son compte** : privé, ou partagé avec les
    /// comptes choisis — jamais public d'ici, comme au bureau où un
    /// rendez-vous se partage. Le jour et les heures se tapent comme pour
    /// l'agenda de l'officine. Jamais un patient : l'écran le dit.
    #[allow(clippy::too_many_arguments)]
    pub fn add_account_rdv(
        &self,
        day: String,
        time: String,
        end_time: String,
        title: String,
        readers: Vec<String>,
        today: String,
        now: u64,
    ) -> Result<()> {
        use bpm_caddy::compte::{self, Kind, Visibility};
        let (day, time, end_time, title) = typed_event(&day, &time, &end_time, &title, &today)?;
        let entry = compte::Entry {
            titre: title,
            jour: day,
            debut: time,
            fin: end_time,
            ..compte::Entry::default()
        };
        self.write_own(Kind::Rdv, entry, readers, Visibility::Private, now)
    }

    /// **Modifier un de ses rendez-vous** — jour, heures, titre —, contre
    /// la version que l'écran montrait. Il garde ses lecteurs, moins un
    /// compte bloqué depuis : ce qui n'a plus de lecteur ne part pas.
    #[allow(clippy::too_many_arguments)]
    pub fn edit_account_rdv(
        &self,
        item: String,
        shown_seq: u64,
        day: String,
        time: String,
        end_time: String,
        title: String,
        today: String,
        now: u64,
    ) -> Result<()> {
        use bpm_caddy::compte::{self, Hold, Kind, State, Visibility};
        let account = self
            .account()
            .ok_or_else(|| failed("compte_write_failed"))?;
        let latest = self.reads(Some(&account))?;
        let current = latest
            .iter()
            .find(|r| r.account == account.id() && r.item == item && r.kind == Kind::Rdv)
            .filter(|r| r.state != State::Deleted)
            .ok_or_else(|| failed("compte_write_failed"))?;
        if current.seq != shown_seq {
            return Err(failed("compte_stale"));
        }
        let was = compte::entry_of(current).ok_or_else(|| failed("compte_write_failed"))?;
        let (day, time, end_time, title) = typed_event(&day, &time, &end_time, &title, &today)?;
        let entry = compte::Entry {
            titre: title,
            jour: day,
            debut: time,
            fin: end_time,
            ..was.clone()
        };
        let visibility = match compte::visibility_of(current, &was) {
            Visibility::Shared(readers) => {
                let (settings, _) = compte::settings_for(&latest, &account.id());
                let readers: Vec<String> = readers
                    .into_iter()
                    .filter(|a| settings.hold(a) != Hold::Blocked)
                    .collect();
                if readers.is_empty() {
                    return Err(failed("compte_need_reader"));
                }
                Visibility::Shared(readers)
            }
            other => other,
        };
        self.seal(
            &account,
            &latest,
            (&item, compte::next_seq(current.seq, now)),
            Kind::Rdv,
            &entry,
            &visibility,
        )
    }

    /// **Effacer un de ses billets ou de ses rendez-vous**, contre la
    /// version que l'écran montrait : changé ailleurs depuis, il ne
    /// s'efface pas sans être relu.
    pub fn delete_post(&self, item: String, shown_seq: u64, now: u64) -> Result<()> {
        use bpm_caddy::compte::{self, Kind};
        let account = self
            .account()
            .ok_or_else(|| failed("compte_write_failed"))?;
        let latest = self.reads(None)?;
        let current = latest
            .iter()
            .find(|r| {
                r.account == account.id()
                    && r.item == item
                    && matches!(r.kind, Kind::Billet | Kind::Rdv)
            })
            .ok_or_else(|| failed("compte_write_failed"))?;
        if current.seq != shown_seq {
            return Err(failed("compte_stale"));
        }
        let bytes = compte::write(
            &account,
            &item,
            compte::next_seq(current.seq, now),
            current.kind,
            compte::Content::Deleted,
            &mut bpm_sync::OsEntropy,
        )
        .map_err(|_| failed("compte_write_failed"))?;
        self.keep_one(bytes)
    }

    /// **Masquer, bloquer ou rétablir un auteur** (`lire`, `masquer`,
    /// `bloquer`) : les réglages du compte ouvert, scellés pour lui, d'une
    /// version de plus. Écrits sur la dernière version que la base tient,
    /// pas sur celle que l'écran montrait : ce geste ne change que cet
    /// auteur, et une décision prise ailleurs sur un autre reste.
    pub fn hold_author(&self, author: String, hold: String, now: u64) -> Result<String> {
        use bpm_caddy::compte::{self, Hold};
        let account = self
            .account()
            .ok_or_else(|| failed("compte_write_failed"))?;
        if author == account.id() {
            return Err(failed("compte_write_failed"));
        }
        let (hold, key) = match hold.as_str() {
            "masquer" => (Hold::Muted, "compte_hold_muted"),
            "bloquer" => (Hold::Blocked, "compte_hold_blocked"),
            _ => (Hold::Read, "compte_hold_read"),
        };
        let latest = self.reads(Some(&account))?;
        let (mut settings, seq) = compte::settings_for(&latest, &account.id());
        settings.set(&author, hold);
        let bytes = compte::write_settings(
            &account,
            compte::next_seq(seq, now),
            &settings,
            &mut bpm_sync::OsEntropy,
        )
        .map_err(|e| match e {
            compte::WriteError::TooLong => failed("compte_settings_full"),
            _ => failed("compte_write_failed"),
        })?;
        let mut records = vec![bytes];
        // Bloqué, il n'est plus lecteur de la fiche — rescellée sans lui,
        // avec les réglages, comme au bureau.
        let fiche = latest
            .iter()
            .find(|r| r.account == account.id() && r.item == compte::FICHE_ITEM);
        if let Some((r, fiche)) = fiche
            .and_then(|r| Some((r, compte::fiche_of(r)?)))
            .filter(|_| hold == Hold::Blocked)
            .and_then(|(r, f)| Some((r, compte::fiche_without(&f, &author)?)))
        {
            let boxes = |a: &str| {
                latest
                    .iter()
                    .filter(|r| r.account == a)
                    .find_map(compte::profile_of)
                    .and_then(|p| compte::unhex::<32>(&p.boite))
            };
            records.push(
                compte::write_fiche(
                    &account,
                    compte::next_seq(r.seq, now),
                    Some(&fiche),
                    &boxes,
                    &mut bpm_sync::OsEntropy,
                )
                .map_err(|e| match e {
                    compte::EntryError::NoKey(a) => bpm_caddy::strings::trf(
                        "compte_no_key",
                        name_in(&compte::named(&latest), &a),
                    )
                    .into(),
                    compte::EntryError::Write(_) => failed("compte_write_failed"),
                })?,
            );
        }
        if self.db().keep_account_records(&records)? != records.len() {
            return Err(failed("compte_write_failed"));
        }
        Ok(bpm_caddy::strings::trf(
            key,
            name_in(&compte::named(&latest), &author),
        ))
    }

    /// Les conversations de l'équipe, la plus récente d'abord. Celles
    /// avec d'autres officines passent par le réseau, que le téléphone
    /// n'a pas : elles n'y sont pas.
    pub fn conversations(&self, operator: String) -> Result<Vec<Talk>> {
        let operator = initials(&operator);
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
            // **La même règle que le poste de bureau** : une conversation
            // privée ne se montre qu'à ses membres. Le téléphone ne tient
            // pas les groupes — une conversation de groupe y reste à son
            // auteur. Sans initiales, seules celles de toute l'équipe.
            if !bpm_caddy::messages::visible_to(&c, &[], &operator) {
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
        let db = self.db();
        team_only(&db, conversation)?;
        let files = db.conversation_files(conversation)?;
        Ok(db
            .conversation_messages(conversation)?
            .into_iter()
            .map(|m| Said {
                files: files
                    .iter()
                    .filter(|(f, _)| f.message == m.uid)
                    .map(|(f, got)| Attachment {
                        uid: f.uid.clone(),
                        name: f.name.clone(),
                        size: u64::try_from(f.size).unwrap_or(0),
                        complete: *got >= f.chunks,
                    })
                    .collect(),
                id: m.id,
                author: m.author,
                body: m.body,
                sent_at: stamp(&m.sent_at),
                cites_patient: m.patient_id.is_some(),
            })
            .collect())
    }

    /// **Enregistrer un fichier joint** dans `dir`, recollé et vérifié
    /// contre son empreinte — rien s'il n'est pas entier ou pas conforme.
    /// Rend le chemin écrit.
    pub fn save_file(&self, conversation: i64, file_uid: String, dir: String) -> Result<String> {
        let db = self.db();
        let failed = || CaddyError::Failed {
            reason: bpm_caddy::strings::tr("mobile_file_incomplete").to_owned(),
        };
        team_only(&db, conversation)?;
        let (meta, _) = db
            .conversation_files(conversation)?
            .into_iter()
            .find(|(f, _)| f.uid == file_uid)
            .ok_or_else(failed)?;
        let bytes = db.file_bytes(&meta)?.ok_or_else(failed)?;
        // **Jamais par-dessus un fichier déjà là** : deux pièces du même
        // nom, « ordonnance.pdf », se remplaçaient sans un mot. La
        // suivante prend « ordonnance (2).pdf ».
        let name = bpm_caddy::messages::safe_name(&meta.name);
        let (stem, ext) = match name.rsplit_once('.') {
            Some((s, e)) if !s.is_empty() => (s.to_owned(), format!(".{e}")),
            _ => (name.clone(), String::new()),
        };
        for n in 1..1000 {
            let candidate = if n == 1 {
                name.clone()
            } else {
                format!("{stem} ({n}){ext}")
            };
            let path = PathBuf::from(&dir).join(&candidate);
            match std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
            {
                Ok(mut f) => {
                    std::io::Write::write_all(&mut f, &bytes).map_err(|e| e.to_string())?;
                    return Ok(path.display().to_string());
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e.to_string().into()),
            }
        }
        Err(failed())
    }

    /// Écrire un message avec un fichier joint (5 Mio au plus, comme sur
    /// les postes). Part à la synchronisation suivante.
    pub fn send_file(
        &self,
        conversation: i64,
        author: String,
        body: String,
        name: String,
        bytes: Vec<u8>,
    ) -> Result<()> {
        let author = initials(&author);
        let db = self.db();
        if bytes.len() > bpm_caddy::messages::MAX_FILE {
            return Err(CaddyError::Failed {
                reason: bpm_caddy::strings::trf(
                    "msg_file_too_big",
                    bpm_caddy::messages::MAX_FILE / (1024 * 1024),
                ),
            });
        }
        team_only(&db, conversation)?;
        // Le message et son fichier ensemble, comme sur un poste.
        db.post_message_with_file(
            conversation,
            author.trim(),
            body.trim(),
            None,
            Some((&name, &bytes)),
        )?;
        Ok(())
    }

    /// **Ouvrir une conversation d'équipe**, pour toute l'équipe (sans
    /// membres nommés, comme sur un poste). Rend son numéro.
    pub fn new_conversation(&self, title: String, author: String) -> Result<i64> {
        let author = initials(&author);
        Ok(self.db().create_conversation(
            "",
            bpm_caddy::messages::Channel::Equipe,
            title.trim(),
            &[],
            None,
            &[],
            author.trim(),
        )?)
    }

    /// Noter la conversation lue par `operator` jusqu'à son dernier
    /// message — la marque que les postes lisent aussi. Sans initiales,
    /// rien.
    pub fn mark_read(&self, conversation: i64, operator: String) -> Result<()> {
        let operator = initials(&operator);
        if operator.trim().is_empty() {
            return Ok(());
        }
        let db = self.db();
        team_only(&db, conversation)?;
        if let Some(last) = db.conversation_messages(conversation)?.last() {
            db.mark_read(conversation, &operator, &last.sent_at)?;
        }
        Ok(())
    }

    /// Écrire dans une conversation de l'équipe. `author` : les initiales
    /// de qui tient le téléphone. Part à la synchronisation suivante.
    pub fn send_message(&self, conversation: i64, author: String, body: String) -> Result<()> {
        let author = initials(&author);
        let body = body.trim();
        if body.is_empty() {
            return Ok(());
        }
        let db = self.db();
        team_only(&db, conversation)?;
        db.post_message(conversation, "", author.trim(), body, None, "", None)?;
        Ok(())
    }
}

impl Caddy {
    /// Ce jour tel que le téléphone l'a montré, retrouvé dans la base
    /// (le rythme de sa trame), puis écrit en compare-and-set.
    fn set_shift_day(&self, shown: &ShiftItem, now: Option<(&str, &str, i64)>) -> Result<bool> {
        if bpm_caddy::planning::ShiftKind::parse(&shown.kind_key)
            .is_some_and(bpm_caddy::planning::ShiftKind::is_absence)
        {
            return Err(failed("mobile_planning_absence"));
        }
        let db = self.db();
        let Some(stored) = db
            .shifts_between(&shown.day, &shown.day)?
            .into_iter()
            .find(|s| s.id == shown.id && s.source_id == shown.source_id)
        else {
            return Ok(false);
        };
        let shown = bpm_caddy::db::PlannedShift {
            operator: shown.operator.clone(),
            start_time: shown.start_time.clone(),
            end_time: shown.end_time.clone(),
            pause_minutes: shown.pause_minutes,
            kind: shown.kind_key.clone(),
            ..stored
        };
        Ok(db.set_occurrence(&shown, now)?)
    }

    /// Le compte ouvert, rouvert de sa phrase pour l'appel : la clé ne
    /// se copie pas, elle se refait.
    fn account(&self) -> Option<bpm_caddy::compte::Account> {
        self.account
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .map(|a| bpm_caddy::compte::Account::from_phrase(a.phrase().clone()))
    }

    /// La dernière version de chaque élément de compte, lue par `reader`.
    fn reads(
        &self,
        reader: Option<&bpm_caddy::compte::Account>,
    ) -> Result<Vec<bpm_caddy::compte::Read>> {
        Ok(self
            .db()
            .account_latest(reader)?
            .into_iter()
            .map(|(r, _)| r)
            .collect())
    }

    /// Écrire un élément neuf du compte ouvert, pour `readers`, ou
    /// `alone` sans lecteur choisi. **Partagé reste partagé** : des
    /// lecteurs tous bloqués ne le font pas partir public (ni privé).
    fn write_own(
        &self,
        kind: bpm_caddy::compte::Kind,
        entry: bpm_caddy::compte::Entry,
        readers: Vec<String>,
        alone: bpm_caddy::compte::Visibility,
        now: u64,
    ) -> Result<()> {
        use bpm_caddy::compte::{self, Hold, Visibility};
        let account = self
            .account()
            .ok_or_else(|| failed("compte_write_failed"))?;
        let latest = self.reads(Some(&account))?;
        if kind != compte::Kind::Billet && !compte::has_room(&latest, &account.id()) {
            return Err(CaddyError::Failed {
                reason: bpm_caddy::strings::trf("compte_too_many", compte::MOST_ITEMS),
            });
        }
        let (settings, _) = compte::settings_for(&latest, &account.id());
        let visibility = if readers.is_empty() {
            alone
        } else {
            let readers: Vec<String> = readers
                .into_iter()
                .filter(|a| settings.hold(a) != Hold::Blocked)
                .collect();
            if readers.is_empty() {
                return Err(failed("compte_need_reader"));
            }
            Visibility::Shared(readers)
        };
        let item = compte::new_item(&mut bpm_sync::OsEntropy);
        self.seal(
            &account,
            &latest,
            (&item, compte::next_seq(0, now)),
            kind,
            &entry,
            &visibility,
        )
    }

    /// Sceller un élément pour ses lecteurs et le ranger.
    fn seal(
        &self,
        account: &bpm_caddy::compte::Account,
        latest: &[bpm_caddy::compte::Read],
        at: (&str, u64),
        kind: bpm_caddy::compte::Kind,
        entry: &bpm_caddy::compte::Entry,
        visibility: &bpm_caddy::compte::Visibility,
    ) -> Result<()> {
        use bpm_caddy::compte::{self, EntryError};
        let named = compte::named(latest);
        // La clé de tout compte qui l'a publiée, nommé ici ou non.
        let boxes = |a: &str| {
            latest
                .iter()
                .filter(|r| r.account == a)
                .find_map(compte::profile_of)
                .and_then(|p| compte::unhex::<32>(&p.boite))
        };
        let bytes = compte::write_entry(
            account,
            at,
            kind,
            entry,
            visibility,
            &boxes,
            &mut bpm_sync::OsEntropy,
        )
        .map_err(|e| match e {
            EntryError::NoKey(a) => CaddyError::Failed {
                reason: bpm_caddy::strings::trf("compte_no_key", name_in(&named, &a)),
            },
            EntryError::Write(compte::WriteError::TooLong) => CaddyError::Failed {
                reason: bpm_caddy::strings::trf("compte_too_long", compte::MAX_DATA / 1024),
            },
            EntryError::Write(_) => failed("compte_write_failed"),
        })?;
        self.keep_one(bytes)
    }

    /// Ranger un enregistrement qu'on vient d'écrire — refusé à la
    /// relecture, il ne se dit pas enregistré.
    fn keep_one(&self, bytes: Vec<u8>) -> Result<()> {
        match self.db().keep_account_records(&[bytes])? {
            0 => Err(failed("compte_write_failed")),
            _ => Ok(()),
        }
    }

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
    /// Une conversation entre officines ne se lit ni ne s'écrit depuis
    /// le téléphone, même en en donnant le numéro.
    #[test]
    fn a_conversation_with_other_officines_is_out_of_reach() {
        let d = dir("not-team");
        let c = Caddy::open(d.display().to_string(), "k".into()).unwrap();
        let (team, other) = {
            let db = c.db();
            let team = db
                .create_conversation(
                    "t",
                    bpm_caddy::messages::Channel::Equipe,
                    "Équipe",
                    &[],
                    None,
                    &[],
                    "AB",
                )
                .unwrap();
            let other = db
                .create_conversation(
                    "o",
                    bpm_caddy::messages::Channel::Officines,
                    "Voisine",
                    &[],
                    None,
                    &["x".to_owned()],
                    "AB",
                )
                .unwrap();
            (team, other)
        };
        assert!(c.send_message(team, "ab".into(), "bonjour".into()).is_ok());
        assert!(c.messages(team).is_ok());
        assert!(c
            .send_message(other, "ab".into(), "bonjour".into())
            .is_err());
        assert!(c.messages(other).is_err());
        assert!(c.mark_read(other, "ab".into()).is_err());
        let _ = std::fs::remove_dir_all(&d);
    }

    /// **Le fil sur le téléphone** : les billets publics seulement, le
    /// plus récent d'abord, un billet daté de demain rangé à aujourd'hui,
    /// l'auteur nommé par son profil et son empreinte.
    #[test]
    fn the_phone_reads_the_public_feed_newest_first() {
        use bpm_caddy::compte::{self, Account, Kind, Phrase, Visibility};
        let d = dir("news");
        let c = Caddy::open(d.display().to_string(), "k".into()).unwrap();
        let claire = Account::from_phrase(Phrase::from_entropy([31; 16]));
        let marc = Account::from_phrase(Phrase::from_entropy([32; 16]));
        let profile = compte::Profile {
            nom: "Claire Martin".to_owned(),
            ..compte::Profile::default()
        };
        let mut records =
            vec![compte::write_profile(&claire, 1, &profile, &mut bpm_sync::OsEntropy).unwrap()];
        let boxes = |a: &str| (a == claire.id()).then(|| claire.box_public());
        for (who, item, jour, debut, vis) in [
            (&claire, "b1", "2026-10-18", "09:00", Visibility::Public),
            (&claire, "b2", "2026-10-19", "", Visibility::Public),
            (&marc, "b3", "2099-01-01", "08:00", Visibility::Public),
            (
                &marc,
                "b4",
                "2026-10-19",
                "10:00",
                Visibility::Shared(vec![claire.id()]),
            ),
        ] {
            let e = compte::Entry {
                texte: item.to_owned(),
                jour: jour.to_owned(),
                debut: debut.to_owned(),
                ..compte::Entry::default()
            };
            records.push(
                compte::write_entry(
                    who,
                    (item, 1),
                    Kind::Billet,
                    &e,
                    &vis,
                    &boxes,
                    &mut bpm_sync::OsEntropy,
                )
                .unwrap(),
            );
        }
        c.db().keep_account_records(&records).unwrap();
        let news = c.news("2026-10-19".into(), 10).unwrap().posts;
        let texts: Vec<&str> = news.iter().map(|n| n.text.as_str()).collect();
        assert_eq!(texts, ["b2", "b3", "b1"], "le partagé n'y est pas");
        assert!(
            news[0].author.starts_with("Claire Martin ("),
            "{}",
            news[0].author
        );
        assert!(!news[1].author.contains("Claire"));
        assert_eq!(news[2].written, "18/10/2026 09:00");
        assert_eq!(c.news("2026-10-19".into(), 1).unwrap().posts.len(), 1);
        let _ = std::fs::remove_dir_all(&d);
    }

    /// **Un compte sur le téléphone**, de bout en bout : retrouvé par sa
    /// phrase sous un code, il lit ce qu'on partage avec lui, publie et
    /// efface un billet (pas sur une version que l'écran ne montrait
    /// plus), masque puis bloque un auteur, se ferme et se rouvre par son
    /// code — un mauvais refusé.
    #[test]
    fn an_account_opens_on_the_phone_posts_and_sets_an_author_aside() {
        use bpm_caddy::compte::{self, Account, Kind, Phrase, Visibility};
        let d = dir("account");
        let c = Caddy::open(d.display().to_string(), "k".into()).unwrap();
        let phrase = Phrase::from_entropy([41; 16]);
        let me = Account::from_phrase(phrase.clone());
        let marc = Account::from_phrase(Phrase::from_entropy([42; 16]));
        let mut records = Vec::new();
        for (a, nom) in [(&me, "Claire"), (&marc, "Marc")] {
            let p = compte::Profile {
                nom: nom.to_owned(),
                ..compte::Profile::default()
            };
            records.push(compte::write_profile(a, 1, &p, &mut bpm_sync::OsEntropy).unwrap());
        }
        let to_me = |a: &str| (a == me.id()).then(|| me.box_public());
        let rdv = compte::Entry {
            titre: "Réunion".to_owned(),
            jour: "2026-10-21".to_owned(),
            debut: "14:00".to_owned(),
            ..compte::Entry::default()
        };
        records.push(
            compte::write_entry(
                &marc,
                ("r1", 1),
                Kind::Rdv,
                &rdv,
                &Visibility::Shared(vec![me.id()]),
                &to_me,
                &mut bpm_sync::OsEntropy,
            )
            .unwrap(),
        );
        let e = compte::Entry {
            texte: "pour Claire".to_owned(),
            jour: "2026-10-19".to_owned(),
            ..compte::Entry::default()
        };
        records.push(
            compte::write_entry(
                &marc,
                ("m1", 1),
                Kind::Billet,
                &e,
                &Visibility::Shared(vec![me.id()]),
                &to_me,
                &mut bpm_sync::OsEntropy,
            )
            .unwrap(),
        );
        c.db().keep_account_records(&records).unwrap();
        let today = || "2026-10-20".to_owned();
        assert!(c.news(today(), 10).unwrap().posts.is_empty(), "scellé");
        // Une phrase fautive dit où ; un code court est refusé.
        let mut words = phrase.words();
        assert!(c
            .account_recover("qwerty ".repeat(12), "Pharma24".into())
            .is_err());
        assert!(c.account_recover(words.join(" "), "123".into()).is_err());
        let opened = c
            .account_recover(words.join(" "), "Pharma24".into())
            .unwrap();
        assert_eq!(opened.id, me.id());
        assert!(opened.name.starts_with("Claire ("));
        // Le rendez-vous que Marc lui partage, à l'agenda de la semaine.
        let week = || ("2026-10-19".to_owned(), "2026-10-25".to_owned());
        let rdvs = c.account_agenda(week().0, week().1).unwrap();
        assert_eq!(rdvs.len(), 1);
        assert_eq!(
            (rdvs[0].title.as_str(), rdvs[0].time.as_str()),
            ("Réunion", "14:00")
        );
        assert!(rdvs[0].author.starts_with("Marc ("));
        // Discret, il se nomme encore à lui-même — et à Marc, son lecteur.
        let fiche = compte::Profile {
            nom: "Claire".to_owned(),
            pour: vec![marc.id()],
            ..compte::Profile::default()
        };
        let to_marc = |a: &str| (a == marc.id()).then(|| marc.box_public());
        let bare = compte::Profile::default();
        c.db()
            .keep_account_records(&[
                compte::write_profile(&me, 2, &bare, &mut bpm_sync::OsEntropy).unwrap(),
                compte::write_fiche(&me, 2, Some(&fiche), &to_marc, &mut bpm_sync::OsEntropy)
                    .unwrap(),
            ])
            .unwrap();
        assert!(c.open_account().unwrap().name.starts_with("Claire ("));
        let feed = c.news(today(), 10).unwrap();
        assert_eq!(feed.posts.len(), 1);
        assert!(feed.posts[0].shared && !feed.posts[0].mine);
        // Publier : public, puis pour Marc.
        c.publish_post("Bonjour".into(), vec![], today(), "09:30".into(), 100)
            .unwrap();
        c.publish_post(
            "Pour Marc".into(),
            vec![marc.id()],
            today(),
            "09:31".into(),
            101,
        )
        .unwrap();
        assert!(c
            .publish_post("  ".into(), vec![], today(), "09:32".into(), 102)
            .is_err());
        let feed = c.news(today(), 10).unwrap().posts;
        let texts: Vec<&str> = feed.iter().map(|p| p.text.as_str()).collect();
        assert_eq!(texts, ["Pour Marc", "Bonjour", "pour Claire"]);
        assert!(feed[0].mine && feed[0].shared && !feed[1].shared);
        // Marc lit celui qui lui est destiné.
        let marc_reads = c.db().account_latest(Some(&marc)).unwrap();
        assert!(marc_reads
            .iter()
            .any(|(r, _)| compte::entry_of(r).is_some_and(|e| e.texte == "Pour Marc")));
        // Effacer : pas sur une version que l'écran ne montrait plus.
        let bonjour = &feed[1];
        assert!(c
            .delete_post(bonjour.item.clone(), bonjour.seq + 1, 200)
            .is_err());
        c.delete_post(bonjour.item.clone(), bonjour.seq, 200)
            .unwrap();
        assert_eq!(c.news(today(), 10).unwrap().posts.len(), 2);
        // Masquer Marc : son billet se compte sans se montrer.
        let people = c.people().unwrap();
        assert_eq!(people.len(), 1);
        assert_eq!(people[0].hold, "lire");
        c.hold_author(marc.id(), "masquer".into(), 300).unwrap();
        let feed = c.news(today(), 10).unwrap();
        assert_eq!((feed.posts.len(), feed.hidden), (1, 1));
        let marc_names_me = |c: &Caddy| {
            let latest = c.db().account_latest(Some(&marc)).unwrap();
            latest.iter().any(|(r, _)| compte::fiche_of(r).is_some())
        };
        assert!(marc_names_me(&c));
        c.hold_author(marc.id(), "bloquer".into(), 301).unwrap();
        assert_eq!(c.people().unwrap()[0].hold, "bloquer");
        // Ni lecteur de la fiche : rescellée sans lui, au même geste.
        assert!(!marc_names_me(&c));
        assert!(c.account_agenda(week().0, week().1).unwrap().is_empty());
        assert!(c.open_account().unwrap().name.starts_with("Claire ("));
        // Bloqué, il n'est plus lecteur de ce qu'on écrit — et un billet
        // pour lui seul ne part pas public pour autant.
        assert!(c
            .publish_post(
                "Encore".into(),
                vec![marc.id()],
                today(),
                "10:00".into(),
                400
            )
            .is_err());
        assert!(!c
            .news(today(), 10)
            .unwrap()
            .posts
            .iter()
            .any(|p| p.text == "Encore"));
        c.hold_author(marc.id(), "lire".into(), 500).unwrap();
        assert_eq!(c.news(today(), 10).unwrap().hidden, 0);
        // Fermé, rouvert par son code ; un mauvais refusé.
        assert_eq!(c.account_agenda(week().0, week().1).unwrap().len(), 1);
        // Un rendez-vous à soi, puis un pour Marc ; le sien s'efface.
        c.add_account_rdv(
            "2210".into(),
            "9h30".into(),
            "10h".into(),
            "Pharmacovigilance".into(),
            vec![],
            today(),
            510,
        )
        .unwrap();
        assert!(c
            .add_account_rdv(
                "2210".into(),
                "25h".into(),
                "".into(),
                "x".into(),
                vec![],
                today(),
                511
            )
            .is_err());
        c.add_account_rdv(
            "23/10/2026".into(),
            "".into(),
            "".into(),
            "Inventaire".into(),
            vec![marc.id()],
            today(),
            512,
        )
        .unwrap();
        let rdvs = c.account_agenda(week().0, week().1).unwrap();
        let shown: Vec<(&str, &str, &str)> = rdvs
            .iter()
            .map(|r| (r.day.as_str(), r.time.as_str(), r.title.as_str()))
            .collect();
        assert_eq!(
            shown,
            [
                ("2026-10-21", "14:00", "Réunion"),
                ("2026-10-22", "09:30", "Pharmacovigilance"),
                ("2026-10-23", "", "Inventaire")
            ]
        );
        assert!(rdvs[1].author.is_empty() && rdvs[1].end_time == "10:00");
        let private = c.db().account_latest(Some(&marc)).unwrap();
        assert!(private
            .iter()
            .any(|(r, _)| compte::entry_of(r).is_some_and(|e| e.titre == "Inventaire")));
        assert!(!private
            .iter()
            .any(|(r, _)| compte::entry_of(r).is_some_and(|e| e.titre == "Pharmacovigilance")));
        // Modifié : Marc le lit toujours, à la nouvelle heure ; contre
        // une version dépassée, refusé.
        c.edit_account_rdv(
            rdvs[2].item.clone(),
            rdvs[2].seq,
            "23/10/2026".into(),
            "14h".into(),
            "".into(),
            "Inventaire annuel".into(),
            today(),
            513,
        )
        .unwrap();
        assert!(c
            .edit_account_rdv(
                rdvs[2].item.clone(),
                rdvs[2].seq,
                "2310".into(),
                "".into(),
                "".into(),
                "x".into(),
                today(),
                514
            )
            .is_err());
        let private = c.db().account_latest(Some(&marc)).unwrap();
        assert!(private.iter().any(|(r, _)| {
            compte::entry_of(r)
                .is_some_and(|e| e.titre == "Inventaire annuel" && e.debut == "14:00")
        }));
        assert_eq!(c.account_agenda(week().0, week().1).unwrap().len(), 3);
        c.delete_post(rdvs[1].item.clone(), rdvs[1].seq, 520)
            .unwrap();
        assert_eq!(c.account_agenda(week().0, week().1).unwrap().len(), 2);
        assert!(
            c.delete_post(rdvs[0].item.clone(), rdvs[0].seq, 521)
                .is_err(),
            "pas le sien"
        );
        // Au plafond, le téléphone refuse comme le bureau ; un billet,
        // compté à part, passe encore.
        let mut written = 0;
        let full = loop {
            match c.add_account_rdv(
                "2410".into(),
                "".into(),
                "".into(),
                format!("r{written}"),
                vec![],
                today(),
                530 + written,
            ) {
                Ok(()) => written += 1,
                Err(e) => break e,
            }
            assert!(written <= compte::MOST_ITEMS as u64);
        };
        assert!(written > 0 && full.to_string().contains("au plus"));
        c.publish_post("Toujours".into(), vec![], today(), "10:00".into(), 800)
            .unwrap();
        c.account_close();
        assert!(c.open_account().is_none());
        assert!(c.account_agenda(week().0, week().1).unwrap().is_empty());
        assert!(c
            .add_account_rdv(
                "2210".into(),
                "9h".into(),
                "".into(),
                "x".into(),
                vec![],
                today(),
                600
            )
            .is_err());
        assert_eq!(c.kept_accounts().unwrap().len(), 1);
        assert!(c.account_unlock(me.id(), "mauvais1".into()).is_err());
        assert!(c.account_unlock(me.id(), "Pharma24".into()).is_ok());
        c.account_forget(me.id()).unwrap();
        assert!(c.open_account().is_none() && c.kept_accounts().unwrap().is_empty());
        words.clear();
        let _ = std::fs::remove_dir_all(&d);
    }

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

    /// **One day from the phone**: a day more, typed the counter's way; a
    /// day of a pattern changed or emptied alone; nothing written over a
    /// day that moved since the phone showed it.
    /// **Une absence posée au téléphone** : une ligne du premier au
    /// dernier jour, qui cache ce que la trame dit ces jours-là et pas
    /// les autres ; une nature qui n'est pas une absence, des jours à
    /// l'envers ou pas d'initiales ne posent rien.
    #[test]
    fn the_phone_sets_an_absence_as_the_desktop_does() {
        let d = dir("absence");
        let c = Caddy::open(d.display().to_string(), "k".into()).unwrap();
        let keys: Vec<String> = absence_kinds().into_iter().map(|k| k.key).collect();
        assert_eq!(keys, ["CONGE", "MALADIE", "FORMATION", "RECUP"]);
        c.db()
            .add_shift(&bpm_caddy::db::NewShift {
                operator: "CL".into(),
                day: "2026-09-29".into(),
                start_time: "08:30".into(),
                end_time: "12:00".into(),
                kind: "JOURNEE".into(),
                repeat_days: 7,
                ..Default::default()
            })
            .unwrap();
        let at = |day: &str| c.planning(day.into(), day.into()).unwrap();
        let today = || "2026-10-01".to_owned();
        let set = |who: &str, kind: &str, first: &str, last: &str| {
            c.add_absence(who.into(), kind.into(), first.into(), last.into(), today())
        };
        assert!(set("cl", "JOURNEE", "12/10", "16/10").is_err());
        assert!(set("cl", "CONGE", "16/10", "12/10").is_err());
        assert!(set("", "CONGE", "12/10", "16/10").is_err());
        assert!(set("cl", "CONGE", "12/10", "16/10").is_ok());
        let tuesday = at("2026-10-13");
        assert_eq!(tuesday.len(), 1);
        assert!(tuesday[0].absence);
        assert_eq!(tuesday[0].operator, "CL");
        assert_eq!(tuesday[0].start_time, "08:30", "son heure la plus matinale");
        assert!(
            at("2026-10-20").iter().all(|s| !s.absence),
            "la semaine d'après"
        );
        assert_eq!(at("2026-10-20").len(), 1);
        // A single day.
        assert!(set("MD", "FORMATION", "21/10/2026", "21/10/2026").is_ok());
        let wed = at("2026-10-21");
        assert_eq!(wed.len(), 1);
        assert!(wed[0].absence && !wed[0].repeats);
        assert_eq!(wed[0].start_time, "09:00");
    }

    #[test]
    fn the_phone_changes_one_day_of_the_planning_and_only_that_day() {
        let d = dir("planning");
        let c = Caddy::open(d.display().to_string(), "k".into()).unwrap();
        c.db()
            .add_shift(&bpm_caddy::db::NewShift {
                operator: "CL".into(),
                day: "2026-09-29".into(),
                start_time: "09:00".into(),
                end_time: "12:00".into(),
                kind: "JOURNEE".into(),
                repeat_days: 7,
                ..Default::default()
            })
            .unwrap();
        let week = |from: &str, to: &str| c.planning(from.into(), to.into()).unwrap();
        let shown = week("2026-10-06", "2026-10-06").remove(0);
        assert!(shown.repeats);
        assert!(c
            .edit_shift_day(shown.clone(), "9".into(), "12h30".into(), "".into())
            .unwrap());
        assert_eq!(week("2026-10-06", "2026-10-06")[0].end_time, "12:30");
        assert_eq!(week("2026-10-13", "2026-10-13")[0].end_time, "12:00");
        // Ce que le téléphone montrait n'est plus : rien n'est écrit.
        assert!(!c
            .edit_shift_day(shown, "9".into(), "13".into(), "".into())
            .unwrap());
        assert!(c
            .edit_shift_day(
                week("2026-10-06", "2026-10-06").remove(0),
                "9".into(),
                "13".into(),
                "45 min".into()
            )
            .is_err());
        let shown = week("2026-10-13", "2026-10-13").remove(0);
        assert!(c.cancel_shift_day(shown).unwrap());
        assert!(week("2026-10-13", "2026-10-13").is_empty());
        assert_eq!(week("2026-10-20", "2026-10-20").len(), 1);
        // Un samedi de plus.
        assert!(c
            .add_shift_day(
                "".into(),
                "0310".into(),
                "9".into(),
                "12".into(),
                "".into(),
                "2026-09-26".into()
            )
            .is_err());
        c.add_shift_day(
            "cl".into(),
            "0310".into(),
            "9".into(),
            "12".into(),
            "".into(),
            "2026-09-26".into(),
        )
        .unwrap();
        let saturday = week("2026-10-03", "2026-10-03").remove(0);
        assert_eq!(
            (saturday.operator.as_str(), saturday.repeats),
            ("CL", false)
        );
        assert!(c.cancel_shift_day(saturday).unwrap());
        assert!(week("2026-10-03", "2026-10-03").is_empty());
        // Une garde finit le lendemain : sa fin se reprend comme on la tape.
        c.add_shift_day(
            "CL".into(),
            "0410".into(),
            "20".into(),
            "2".into(),
            "".into(),
            "2026-09-26".into(),
        )
        .unwrap();
        let garde = week("2026-10-04", "2026-10-04").remove(0);
        assert_eq!(
            (garde.end_time.as_str(), garde.end_typed.as_str()),
            ("26:00", "2h")
        );
        let end = garde.end_typed.clone();
        assert!(c
            .edit_shift_day(garde, "20".into(), end, "30".into())
            .unwrap());
        assert_eq!(week("2026-10-04", "2026-10-04")[0].end_time, "26:00");
        // Un congé ne se change pas au téléphone.
        c.db()
            .add_shift(&bpm_caddy::db::NewShift {
                operator: "AB".into(),
                day: "2026-10-05".into(),
                start_time: "09:00".into(),
                kind: bpm_caddy::planning::ShiftKind::Conge.as_str().into(),
                cadence: bpm_caddy::planning::Cadence::Unique.as_str().into(),
                ..Default::default()
            })
            .unwrap();
        let off = week("2026-10-05", "2026-10-05").remove(0);
        assert!(off.absence);
        assert!(c.cancel_shift_day(off).is_err());
        let _ = std::fs::remove_dir_all(&d);
    }

    /// **The phone's whole path, through its own API**: a desktop in this
    /// process founds a group and invites a phone; the bridge joins with
    /// the full code, reads a card and the team's message, writes back,
    /// and the desktop's door receives it.
    #[test]
    fn the_bridge_joins_reads_writes_and_syncs_with_a_desktop() {
        use bpm_caddy::postes::{spawn, spawn_auto, Job, OfficineSide, Pace, Poke, Progress};
        let desk_dir = dir("bridge-desk");
        let phone_dir = dir("bridge-phone");
        let desk_path = desk_dir.join("poste.db");
        {
            let db = Db::open(&desk_path, "secret").unwrap();
            db.add_drug("Amoxicilline").unwrap();
            let talk = db
                .create_conversation(
                    "u1",
                    bpm_caddy::messages::Channel::Equipe,
                    "Équipe",
                    &[],
                    None,
                    &[],
                    "YS",
                )
                .unwrap();
            db.post_message(talk, "m1", "YS", "Bonjour", None, "", None)
                .unwrap();
            Posts::load(&db)
                .unwrap()
                .found(&db, "Comptoir 1", "2026-09-26")
                .unwrap();
        }
        let port = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let (_yes, answers) = std::sync::mpsc::channel();
        let rx = spawn(
            Job::Invite {
                port,
                for_device: None,
                companion: true,
            },
            desk_path.clone(),
            "secret".into(),
            "2026-09-26".into(),
            answers,
        );
        let code = match rx.recv().unwrap() {
            Progress::Waiting(code) => {
                // The code names the machine's network address; here, the
                // loop-back.
                format!("{}@127.0.0.1:{port}", code.split('@').next().unwrap())
            }
            other => panic!("{other:?}"),
        };
        let phone = Caddy::open(phone_dir.display().to_string(), "k".into()).unwrap();
        let said = phone
            .join(code, "Téléphone".into(), "2026-09-26".into(), port)
            .unwrap();
        assert!(said.contains('2'), "{said}");
        loop {
            if let Progress::Done(_) | Progress::Failed(_) = rx.recv().unwrap() {
                break;
            }
        }
        let status = phone.status().unwrap();
        assert!(status.in_group && status.cards == 1);
        assert_eq!(phone.search_cards("amox".into(), 5).len(), 1);
        let talks = phone.conversations("AB".into()).unwrap();
        assert_eq!(talks.len(), 1);
        assert_eq!(talks[0].unread, 1);
        phone.mark_read(talks[0].id, "AB".into()).unwrap();
        assert_eq!(phone.conversations("AB".into()).unwrap()[0].unread, 0);
        phone
            .send_message(talks[0].id, "AB".into(), "Reçu".into())
            .unwrap();
        // And a file, which must reach the desktop whole and matching its
        // fingerprint; one over the posts' limit is refused on the phone.
        let photo: Vec<u8> = (0..40_000u32).map(|i| (i % 251) as u8).collect();
        phone
            .send_file(
                talks[0].id,
                "AB".into(),
                "Le frigo".into(),
                "frigo.jpg".into(),
                photo.clone(),
            )
            .unwrap();
        assert!(phone
            .send_file(
                talks[0].id,
                "AB".into(),
                String::new(),
                "gros.bin".into(),
                vec![0u8; bpm_caddy::messages::MAX_FILE + 1],
            )
            .is_err());
        // Saved twice, the same file takes a second name instead of
        // overwriting the first.
        let photo_uid = phone
            .messages(talks[0].id)
            .unwrap()
            .iter()
            .flat_map(|m| m.files.iter())
            .find(|f| f.name == "frigo.jpg")
            .unwrap()
            .uid
            .clone();
        let out = phone_dir.join("saved");
        std::fs::create_dir_all(&out).unwrap();
        let first = phone
            .save_file(talks[0].id, photo_uid.clone(), out.display().to_string())
            .unwrap();
        let second = phone
            .save_file(talks[0].id, photo_uid, out.display().to_string())
            .unwrap();
        assert_ne!(first, second);
        assert!(second.ends_with("frigo (2).jpg"), "{second}");

        // The desktop holds its door; the phone dials it by address.
        let (_auto, poke) = spawn_auto(
            desk_path.clone(),
            "secret".into(),
            "2026-09-26".into(),
            port,
            None,
            Vec::new(),
            OfficineSide::default(),
            Pace::default(),
        );
        let mut reached = 0;
        for _ in 0..20 {
            reached = phone
                .sync("2026-09-26".into(), 0, vec![format!("127.0.0.1:{port}")])
                .unwrap()
                .reached;
            if reached == 1 {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(300));
        }
        let _ = poke.send(Poke::Stop);
        assert_eq!(reached, 1);
        std::thread::sleep(std::time::Duration::from_millis(500));
        let db = Db::open(&desk_path, "secret").unwrap();
        let posts = Posts::load(&db).unwrap();
        posts.absorb(&db, "2026-09-26").unwrap();
        let conv = db.conversations().unwrap()[0].id;
        let back = db.conversation_messages(conv).unwrap();
        assert!(back.iter().any(|m| m.body == "Reçu" && m.author == "AB"));
        let files = db.conversation_files(conv).unwrap();
        let (meta, _) = files
            .iter()
            .find(|(f, _)| f.name == "frigo.jpg")
            .expect("le fichier est arrivé");
        assert_eq!(db.file_bytes(meta).unwrap(), Some(photo));
        let _ = std::fs::remove_dir_all(&desk_dir);
        let _ = std::fs::remove_dir_all(&phone_dir);
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
