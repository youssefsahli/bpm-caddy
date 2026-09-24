//! Les postes d'une officine, **chacun avec sa base**, et ce qui passe de
//! l'une à l'autre : toutes les données de l'officine, ligne par ligne.
//!
//! Ce module ne touche à aucune base. Il dit, pour chaque table, si elle
//! voyage et sur quel flux ; ce qu'une écriture capturée devient pour
//! voyager ; ce qu'une écriture reçue fait à la ligne locale ; et d'où un
//! poste tire ses numéros. `db.rs` capture et range, `postes.rs` fait
//! voyager.
//!
//! Cinq règles, un test chacune.
//!
//! * **Une écriture reçue ne s'applique que sur la valeur qu'elle
//!   remplaçait**, champ par champ. Deux postes qui modifient deux champs
//!   d'un même dossier ne se gênent pas ; deux qui modifient le même champ
//!   sans s'être vus laissent une question « à arbitrer », et rien n'est
//!   écrasé en silence. C'est la règle des fiches entre officines
//!   (`versions.rs`), appliquée à tout.
//! * **Ce qui s'écrit en ajout seul le reste en voyageant.** Le registre
//!   des stupéfiants (R. 5132-36), la caisse, les journaux des ruptures et
//!   des versions : une modification ou une suppression reçue pour l'une
//!   de ces tables est refusée, et le refus se voit.
//! * **Deux postes ne tirent jamais le même numéro.** Chaque poste a son
//!   bloc — pour les dossiers, des centaines de mille lisibles (le poste
//!   fondateur garde les numéros qu'il a déjà donnés) ; pour le reste, des
//!   blocs d'un million de millions. Un numéro de dossier désigne donc le
//!   même dossier sur tous les postes, et le registre qui le cite aussi.
//! * **Rien de ce qui est propre à un poste ne voyage** : son identité, sa
//!   clé, ce qu'il a déjà envoyé, sa télémétrie, et les pièces scannées
//!   (des fichiers, trop gros pour un enregistrement — `docs/SYNC.md`
//!   § 7.6).
//! * **Un enregistrement tient dans la limite du protocole** : une ligne
//!   trop longue se découpe par colonnes, et une valeur qui ne tient pas
//!   seule est nommée au lieu d'être tronquée.
//!
//! Les patients ne sortent jamais de l'officine : ces enregistrements sont
//! scellés sous la clé des postes, que le réseau d'officines n'a pas.

use serde_json::{Map, Value};

/// Le fichier d'une table : la base, ou celui du registre.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum File {
    Main,
    Stups,
}

impl File {
    pub fn key(self) -> &'static str {
        match self {
            File::Main => "m",
            File::Stups => "s",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        match key {
            "m" => Some(File::Main),
            "s" => Some(File::Stups),
            _ => None,
        }
    }
}

/// Le flux d'une table — ce qui choisit la clé qui la scelle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Flux {
    Dossiers,
    Registre,
    Caisse,
    Planning,
    Agenda,
    Officine,
    Fiches,
}

/// Une table qui voyage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Table {
    pub name: &'static str,
    pub file: File,
    pub flux: Flux,
    /// En ajout seul : une modification ou une suppression reçue est
    /// refusée.
    pub append_only: bool,
    /// Un identifiant entier que chaque poste tire de son bloc.
    pub blocked: bool,
}

const fn t(name: &'static str, file: File, flux: Flux, append_only: bool, blocked: bool) -> Table {
    Table {
        name,
        file,
        flux,
        append_only,
        blocked,
    }
}

use File::{Main, Stups};
use Flux::*;

/// Toutes les tables qui voyagent entre les postes.
pub const TABLES: &[Table] = &[
    t("patients", Main, Dossiers, false, true),
    t("interviews", Main, Dossiers, false, true),
    t("biology", Main, Dossiers, false, true),
    t("locations", Main, Dossiers, false, true),
    t("access_log", Main, Dossiers, false, true),
    t("patient_drugs", Main, Dossiers, false, false),
    t("notes", Main, Dossiers, false, true),
    // Un favori peut nommer un dossier : il voyage sous la clé des
    // dossiers.
    t("favorites", Main, Dossiers, false, true),
    t("vaccinations", Main, Dossiers, false, true),
    t("patient_travel", Main, Dossiers, false, false),
    t("drugs", Main, Fiches, false, true),
    t("drug_field_locks", Main, Fiches, false, false),
    t("protocols", Main, Fiches, false, true),
    t("checklists", Main, Fiches, false, true),
    t("checklist_items", Main, Fiches, false, true),
    t("protocol_nodes", Main, Fiches, false, true),
    t("posologies", Main, Fiches, false, true),
    t("drug_facts", Main, Fiches, false, false),
    t("net_facts", Main, Fiches, false, false),
    t("card_edits", Main, Fiches, true, true),
    t("supply_events", Main, Fiches, true, true),
    t("preparations", Main, Fiches, false, true),
    t("dispositifs", Main, Fiches, false, true),
    t("class_notes", Main, Fiches, false, false),
    t("table_cells", Main, Fiches, false, false),
    t("seed_state", Main, Officine, false, false),
    t("settings", Main, Officine, false, false),
    t("prescribers", Main, Officine, false, true),
    t("vaccine_catalogue", Main, Officine, false, true),
    t("trod_lines", Main, Officine, false, true),
    t("content_overrides", Main, Officine, false, false),
    t("sync_posts", Main, Officine, false, false),
    t("events", Main, Agenda, false, true),
    t("shifts", Main, Planning, false, true),
    t("caisse_counts", Main, Caisse, true, true),
    t("stupefiants", Stups, Registre, false, true),
    t("stup_moves", Stups, Registre, true, true),
    t("stup_settings", Stups, Registre, false, false),
    t("stup_labels", Stups, Registre, true, true),
    t("stup_codes", Stups, Registre, false, false),
    t("stup_numbers", Stups, Registre, true, false),
];

/// Les tables qui ne voyagent pas, et pourquoi. Un test tient les deux
/// listes contre le schéma : une table ajoutée sans être rangée dans
/// l'une ou l'autre le fait échouer.
pub const LOCAL: &[(File, &str, &str)] = &[
    (Main, "telemetry", "le logiciel sur ce poste"),
    (
        Main,
        "net_records",
        "le journal du réseau d'officines, par poste",
    ),
    (Main, "net_peers", "les officines que ce poste compose"),
    (
        Main,
        "net_published",
        "ce que ce poste a déjà envoyé au réseau",
    ),
    (
        Main,
        "scans",
        "les pièces : des fichiers, pas des enregistrements",
    ),
    (
        Main,
        "stupefiants",
        "l'ancienne place du registre, avant son fichier",
    ),
    (
        Main,
        "stup_moves",
        "l'ancienne place du registre, avant son fichier",
    ),
    (Main, "sync_local", "l'identité et la clé de ce poste"),
    (Main, "sync_log", "les écritures pas encore parties"),
    (Main, "sync_records", "le journal des postes"),
    (Main, "sync_applied", "ce que ce poste a déjà rangé"),
    (Main, "sync_conflicts", "les questions de ce poste"),
    (Main, "sync_blocks", "les numéros de ce poste"),
    (Main, "sync_mute", "l'interrupteur de la capture"),
    (Stups, "sync_log", "les écritures pas encore parties"),
    (Stups, "sync_blocks", "les numéros de ce poste"),
    (Stups, "sync_mute", "l'interrupteur de la capture"),
];

pub fn table(file: File, name: &str) -> Option<&'static Table> {
    TABLES.iter().find(|t| t.file == file && t.name == name)
}

/// La taille d'un bloc de numéros de dossier : au moins cent mille, et
/// assez pour que le poste fondateur garde tous ceux qu'il a déjà donnés.
pub fn patient_block(max_id: i64) -> i64 {
    const STEP: i64 = 100_000;
    let need = max_id.max(0) + 1;
    (((need + STEP - 1) / STEP) * STEP).max(STEP)
}

/// La taille d'un bloc pour toutes les autres tables.
pub const ROW_BLOCK: i64 = 1_000_000_000_000;

/// Les bornes du bloc d'un poste : le fondateur (0) commence à un.
pub fn bounds(post: i64, size: i64) -> (i64, i64) {
    let lo = post.saturating_mul(size);
    (lo.max(1), lo.saturating_add(size - 1))
}

/// Le numéro du prochain poste appairé.
pub fn next_post(taken: &[i64]) -> i64 {
    taken.iter().copied().max().map_or(0, |m| m + 1)
}

/// Ce que fait une écriture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Insert,
    Update,
    Delete,
    /// La suite d'une ligne trop longue pour un seul enregistrement : ces
    /// colonnes-là, sur la ligne que l'`Insert` qui précède a créée.
    Patch,
}

impl Kind {
    pub fn key(self) -> &'static str {
        match self {
            Kind::Insert => "I",
            Kind::Update => "U",
            Kind::Delete => "D",
            Kind::Patch => "P",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        match key {
            "I" => Some(Kind::Insert),
            "U" => Some(Kind::Update),
            "D" => Some(Kind::Delete),
            "P" => Some(Kind::Patch),
            _ => None,
        }
    }
}

/// Une écriture, telle qu'elle voyage.
#[derive(Clone, Debug, PartialEq)]
pub struct Op {
    pub file: File,
    pub table: String,
    pub kind: Kind,
    /// Les colonnes de la clé primaire et leur valeur.
    pub key: Map<String, Value>,
    /// Ce que les colonnes valaient : les modifiées pour un `Update`, la
    /// ligne entière pour un `Delete`, rien pour un `Insert`.
    pub old: Map<String, Value>,
    /// Ce qu'elles valent : la ligne entière pour un `Insert`, les
    /// modifiées pour un `Update`, rien pour un `Delete`.
    pub new: Map<String, Value>,
}

fn object(text: Option<&str>) -> Map<String, Value> {
    text.and_then(|t| serde_json::from_str::<Value>(t).ok())
        .and_then(|v| match v {
            Value::Object(m) => Some(m),
            _ => None,
        })
        .unwrap_or_default()
}

fn pick(row: &Map<String, Value>, cols: &[String]) -> Option<Map<String, Value>> {
    let mut out = Map::new();
    for c in cols {
        out.insert(c.clone(), row.get(c)?.clone());
    }
    Some(out)
}

/// Ce qu'une écriture capturée devient. `op` est `I`, `U` ou `D`, `old` et
/// `new` les lignes entières en JSON, `key` les colonnes de la clé.
///
/// Une modification qui ne change rien ne voyage pas ; une qui change la
/// clé est une suppression suivie d'une création.
pub fn from_capture(
    file: File,
    table: &str,
    op: &str,
    old: Option<&str>,
    new: Option<&str>,
    key: &[String],
) -> Vec<Op> {
    let (old, new) = (object(old), object(new));
    let make = |kind, k: Map<String, Value>, o, n| Op {
        file,
        table: table.to_owned(),
        kind,
        key: k,
        old: o,
        new: n,
    };
    match op {
        "I" => pick(&new, key)
            .map(|k| vec![make(Kind::Insert, k, Map::new(), new.clone())])
            .unwrap_or_default(),
        "D" => pick(&old, key)
            .map(|k| vec![make(Kind::Delete, k, old.clone(), Map::new())])
            .unwrap_or_default(),
        "U" => {
            let (Some(ko), Some(kn)) = (pick(&old, key), pick(&new, key)) else {
                return Vec::new();
            };
            if ko != kn {
                return vec![
                    make(Kind::Delete, ko, old.clone(), Map::new()),
                    make(Kind::Insert, kn, Map::new(), new.clone()),
                ];
            }
            let mut o = Map::new();
            let mut n = Map::new();
            for (c, v) in &new {
                if old.get(c) != Some(v) {
                    o.insert(c.clone(), old.get(c).cloned().unwrap_or(Value::Null));
                    n.insert(c.clone(), v.clone());
                }
            }
            if n.is_empty() {
                Vec::new()
            } else {
                vec![make(Kind::Update, ko, o, n)]
            }
        }
        _ => Vec::new(),
    }
}

/// Une question à arbitrer : ce champ dit une chose ici, l'autre poste en
/// a écrit une autre. Une colonne vide parle de la ligne entière —
/// supprimée d'un côté, modifiée de l'autre.
#[derive(Clone, Debug, PartialEq)]
pub struct Conflict {
    pub column: String,
    pub mine: Option<Value>,
    pub theirs: Option<Value>,
}

/// Ce qu'une écriture reçue fait à la ligne locale.
#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    /// La ligne n'existe pas ici : la créer.
    Insert,
    /// Écrire ces colonnes.
    Set(Map<String, Value>),
    Delete,
    /// Rien à faire : c'est déjà ainsi, ou tout est en question.
    Nothing,
    /// Une table en ajout seul a reçu autre chose qu'une création.
    Refused,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Outcome {
    pub action: Action,
    pub conflicts: Vec<Conflict>,
}

impl Outcome {
    fn clean(action: Action) -> Self {
        Self {
            action,
            conflicts: Vec::new(),
        }
    }
}

/// **Une écriture reçue ne s'applique que sur la valeur qu'elle
/// remplaçait.** `local` est la ligne telle qu'elle est ici, lue de la
/// même façon qu'elle a été capturée.
pub fn decide(op: &Op, append_only: bool, local: Option<&Map<String, Value>>) -> Outcome {
    if append_only && op.kind != Kind::Insert {
        return Outcome::clean(Action::Refused);
    }
    match (op.kind, local) {
        (Kind::Insert, None) => Outcome::clean(Action::Insert),
        (Kind::Insert, Some(row)) => {
            let conflicts: Vec<Conflict> = op
                .new
                .iter()
                .filter(|(c, v)| !op.key.contains_key(*c) && row.get(*c) != Some(*v))
                .map(|(c, v)| Conflict {
                    column: c.clone(),
                    mine: row.get(c).cloned(),
                    theirs: Some(v.clone()),
                })
                .collect();
            Outcome {
                action: Action::Nothing,
                conflicts,
            }
        }
        (Kind::Update | Kind::Patch, None) => Outcome {
            action: Action::Nothing,
            conflicts: vec![Conflict {
                column: String::new(),
                mine: None,
                theirs: Some(Value::Object(op.new.clone())),
            }],
        },
        (Kind::Patch, Some(_)) => Outcome::clean(Action::Set(op.new.clone())),
        (Kind::Update, Some(row)) => {
            let mut set = Map::new();
            let mut conflicts = Vec::new();
            for (c, v) in &op.new {
                let here = row.get(c);
                if here == Some(v) {
                    continue;
                }
                if here == op.old.get(c) {
                    set.insert(c.clone(), v.clone());
                } else {
                    conflicts.push(Conflict {
                        column: c.clone(),
                        mine: here.cloned(),
                        theirs: Some(v.clone()),
                    });
                }
            }
            Outcome {
                action: if set.is_empty() {
                    Action::Nothing
                } else {
                    Action::Set(set)
                },
                conflicts,
            }
        }
        (Kind::Delete, None) => Outcome::clean(Action::Nothing),
        (Kind::Delete, Some(row)) => {
            let changed = op.old.iter().any(|(c, v)| row.get(c) != Some(v));
            if changed {
                Outcome {
                    action: Action::Nothing,
                    conflicts: vec![Conflict {
                        column: String::new(),
                        mine: Some(Value::Object(row.clone())),
                        theirs: None,
                    }],
                }
            } else {
                Outcome::clean(Action::Delete)
            }
        }
    }
}

/// La taille qu'un lot ne dépasse pas : sous `bpm_sync::MAX_PAYLOAD`,
/// avec de quoi sceller.
pub const LIMIT: usize = 40_000;

fn op_json(op: &Op) -> Value {
    serde_json::json!({
        "f": op.file.key(),
        "t": op.table,
        "o": op.kind.key(),
        "k": op.key,
        "a": op.old,
        "b": op.new,
    })
}

fn size(op: &Op) -> usize {
    op_json(op).to_string().len()
}

/// Découper une écriture trop longue. Rend les morceaux, et la valeur
/// fautive quand une seule colonne ne tient pas.
fn split(op: &Op, limit: usize) -> Result<Vec<Op>, Op> {
    if size(op) <= limit {
        return Ok(vec![op.clone()]);
    }
    let bare = Op {
        old: Map::new(),
        new: Map::new(),
        ..op.clone()
    };
    // Greedy: columns into pieces while each piece stays under the limit.
    let pieces = |cols: &Map<String, Value>, other: &Map<String, Value>| {
        let mut out: Vec<(Map<String, Value>, Map<String, Value>)> = Vec::new();
        for (c, v) in cols {
            let mut n = Map::new();
            n.insert(c.clone(), v.clone());
            let mut o = Map::new();
            if let Some(x) = other.get(c) {
                o.insert(c.clone(), x.clone());
            }
            if let Some((po, pn)) = out.last_mut() {
                let mut tn = pn.clone();
                tn.insert(c.clone(), v.clone());
                let mut to = po.clone();
                to.extend(o.clone());
                let trial = Op {
                    old: to.clone(),
                    new: tn.clone(),
                    ..bare.clone()
                };
                if size(&trial) <= limit {
                    *po = to;
                    *pn = tn;
                    continue;
                }
            }
            out.push((o, n));
        }
        out
    };
    let too_big = |o: &Map<String, Value>, n: &Map<String, Value>| {
        size(&Op {
            old: o.clone(),
            new: n.clone(),
            ..bare.clone()
        }) > limit
    };
    match op.kind {
        Kind::Update | Kind::Patch => {
            let parts = pieces(&op.new, &op.old);
            if parts.iter().any(|(o, n)| too_big(o, n)) {
                return Err(op.clone());
            }
            Ok(parts
                .into_iter()
                .map(|(o, n)| Op {
                    old: o,
                    new: n,
                    ..bare.clone()
                })
                .collect())
        }
        Kind::Insert => {
            let parts = pieces(&op.new, &Map::new());
            if parts.iter().any(|(o, n)| too_big(o, n)) {
                return Err(op.clone());
            }
            Ok(parts
                .into_iter()
                .enumerate()
                .map(|(i, (_, mut n))| {
                    let kind = if i == 0 { Kind::Insert } else { Kind::Patch };
                    if i == 0 {
                        for (c, v) in &op.key {
                            n.insert(c.clone(), v.clone());
                        }
                    }
                    Op {
                        kind,
                        new: n,
                        ..bare.clone()
                    }
                })
                .collect())
        }
        Kind::Delete => {
            // Only what fits is compared: the key always, then the columns
            // in order. A partial check is still a check.
            let parts = pieces(&op.old, &Map::new());
            let first = parts.into_iter().next().map(|(_, n)| n).unwrap_or_default();
            let mut old = first;
            for (c, v) in &op.key {
                old.insert(c.clone(), v.clone());
            }
            let cut = Op {
                old,
                ..bare.clone()
            };
            if size(&cut) > limit {
                Err(op.clone())
            } else {
                Ok(vec![cut])
            }
        }
    }
}

/// Des écritures en lots qui tiennent chacun sous `limit`, dans l'ordre.
/// Rend aussi celles qui ne tiennent pas, même seules — nommées, jamais
/// tronquées.
pub fn batches(ops: &[Op], limit: usize) -> (Vec<Vec<u8>>, Vec<Op>) {
    let mut out = Vec::new();
    let mut refused = Vec::new();
    let mut current: Vec<Value> = Vec::new();
    let mut current_len = 0usize;
    const ENVELOPE: usize = 64;
    for op in ops {
        let pieces = match split(op, limit - ENVELOPE) {
            Ok(p) => p,
            Err(bad) => {
                refused.push(bad);
                continue;
            }
        };
        for p in pieces {
            let v = op_json(&p);
            let len = v.to_string().len() + 1;
            if !current.is_empty() && current_len + len + ENVELOPE > limit {
                out.push(envelope(std::mem::take(&mut current)));
                current_len = 0;
            }
            current_len += len;
            current.push(v);
        }
    }
    if !current.is_empty() {
        out.push(envelope(current));
    }
    (out, refused)
}

fn envelope(ops: Vec<Value>) -> Vec<u8> {
    serde_json::json!({ "v": 1, "t": "lignes", "ops": ops })
        .to_string()
        .into_bytes()
}

/// L'inverse. Une écriture pour une table qui ne voyage pas n'entre pas,
/// ni une dont un nom de colonne n'est pas un identifiant simple : ces
/// noms-là finissent dans du SQL.
pub fn decode(bytes: &[u8]) -> Option<Vec<Op>> {
    let v: Value = serde_json::from_slice(bytes).ok()?;
    if v.get("v")?.as_u64()? != 1 || v.get("t")?.as_str()? != "lignes" {
        return None;
    }
    let mut out = Vec::new();
    for o in v.get("ops")?.as_array()? {
        let file = File::from_key(o.get("f").and_then(|x| x.as_str()).unwrap_or(""));
        let kind = Kind::from_key(o.get("o").and_then(|x| x.as_str()).unwrap_or(""));
        let name = o.get("t").and_then(|x| x.as_str()).unwrap_or("");
        let (Some(file), Some(kind)) = (file, kind) else {
            continue;
        };
        if table(file, name).is_none() {
            continue;
        }
        let map = |k: &str| match o.get(k) {
            Some(Value::Object(m)) => m.clone(),
            _ => Map::new(),
        };
        let op = Op {
            file,
            table: name.to_owned(),
            kind,
            key: map("k"),
            old: map("a"),
            new: map("b"),
        };
        let names_ok = op
            .key
            .keys()
            .chain(op.old.keys())
            .chain(op.new.keys())
            .all(|c| plain_identifier(c));
        if names_ok && !op.key.is_empty() {
            out.push(op);
        }
    }
    Some(out)
}

/// Un nom de colonne tel que le schéma les écrit : lettres, chiffres,
/// soulignés.
pub fn plain_identifier(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        && !name.starts_with(|c: char| c.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn m(v: Value) -> Map<String, Value> {
        match v {
            Value::Object(m) => m,
            _ => Map::new(),
        }
    }

    fn keys(k: &[&str]) -> Vec<String> {
        k.iter().map(|s| s.to_string()).collect()
    }

    /// **Champ par champ** : une écriture reçue s'applique sur ce qu'elle
    /// remplaçait, laisse passer ce que l'autre poste a changé ailleurs,
    /// et pose une question là où les deux ont écrit.
    #[test]
    fn an_incoming_write_applies_field_by_field_and_asks_where_both_wrote() {
        let ops = from_capture(
            File::Main,
            "patients",
            "U",
            Some(r#"{"id":7,"phone":"0600","notes":"","email":""}"#),
            Some(r#"{"id":7,"phone":"0611","notes":"allergie","email":""}"#),
            &keys(&["id"]),
        );
        assert_eq!(ops.len(), 1);
        let op = &ops[0];
        assert_eq!(op.new.len(), 2, "seuls les champs modifiés voyagent");
        // Here, someone changed the e-mail: no clash, both land.
        let here = m(json!({"id":7,"phone":"0600","notes":"","email":"a@b.fr"}));
        let o = decide(op, false, Some(&here));
        assert!(o.conflicts.is_empty());
        assert_eq!(
            o.action,
            Action::Set(m(json!({"phone":"0611","notes":"allergie"})))
        );
        // Here, someone changed the same phone: one question, the notes land.
        let here = m(json!({"id":7,"phone":"0622","notes":"","email":""}));
        let o = decide(op, false, Some(&here));
        assert_eq!(o.action, Action::Set(m(json!({"notes":"allergie"}))));
        assert_eq!(o.conflicts.len(), 1);
        assert_eq!(o.conflicts[0].column, "phone");
        assert_eq!(o.conflicts[0].mine, Some(json!("0622")));
        assert_eq!(o.conflicts[0].theirs, Some(json!("0611")));
        // Already there: nothing.
        let here = m(json!({"id":7,"phone":"0611","notes":"allergie","email":""}));
        assert_eq!(
            decide(op, false, Some(&here)),
            Outcome::clean(Action::Nothing)
        );
        // A write that changes nothing does not travel.
        assert!(from_capture(
            File::Main,
            "patients",
            "U",
            Some(r#"{"id":7,"phone":"0600"}"#),
            Some(r#"{"id":7,"phone":"0600"}"#),
            &keys(&["id"]),
        )
        .is_empty());
    }

    /// Une suppression ne passe que sur la ligne qu'elle a vue ; une ligne
    /// modifiée ici entre-temps reste, avec sa question.
    #[test]
    fn a_delete_passes_only_on_the_row_it_saw() {
        let op = &from_capture(
            File::Main,
            "notes",
            "D",
            Some(r#"{"id":3,"body":"rappeler"}"#),
            None,
            &keys(&["id"]),
        )[0];
        let same = m(json!({"id":3,"body":"rappeler"}));
        assert_eq!(decide(op, false, Some(&same)).action, Action::Delete);
        assert_eq!(decide(op, false, None).action, Action::Nothing);
        let changed = m(json!({"id":3,"body":"rappeler lundi"}));
        let o = decide(op, false, Some(&changed));
        assert_eq!(o.action, Action::Nothing);
        assert_eq!(o.conflicts.len(), 1);
        assert_eq!(o.conflicts[0].column, "");
        // And a key change is a delete then an insert.
        let moved = from_capture(
            File::Main,
            "patient_travel",
            "U",
            Some(r#"{"patient_id":1,"country":"Mali","depart_on":""}"#),
            Some(r#"{"patient_id":1,"country":"Niger","depart_on":""}"#),
            &keys(&["patient_id", "country"]),
        );
        assert_eq!(
            moved.iter().map(|o| o.kind).collect::<Vec<_>>(),
            vec![Kind::Delete, Kind::Insert]
        );
    }

    /// **Ce qui s'écrit en ajout seul le reste en voyageant.**
    #[test]
    fn an_append_only_table_refuses_anything_but_a_creation() {
        for name in [
            "stup_moves",
            "stup_labels",
            "stup_numbers",
            "caisse_counts",
            "supply_events",
            "card_edits",
        ] {
            let t = TABLES.iter().find(|t| t.name == name).unwrap();
            assert!(t.append_only, "{name}");
        }
        let row = m(json!({"id":1,"quantity":2.0}));
        for kind in [Kind::Update, Kind::Delete, Kind::Patch] {
            let op = Op {
                file: File::Stups,
                table: "stup_moves".to_owned(),
                kind,
                key: m(json!({"id":1})),
                old: row.clone(),
                new: m(json!({"quantity":20.0})),
            };
            assert_eq!(decide(&op, true, Some(&row)).action, Action::Refused);
        }
        let insert = Op {
            file: File::Stups,
            table: "stup_moves".to_owned(),
            kind: Kind::Insert,
            key: m(json!({"id":1})),
            old: Map::new(),
            new: row.clone(),
        };
        assert_eq!(decide(&insert, true, None).action, Action::Insert);
        assert_eq!(decide(&insert, true, Some(&row)).action, Action::Nothing);
    }

    /// **Deux postes ne tirent jamais le même numéro**, et le fondateur
    /// garde ceux qu'il a donnés.
    #[test]
    fn two_posts_never_draw_the_same_number_and_the_founder_keeps_his() {
        assert_eq!(patient_block(0), 100_000);
        assert_eq!(patient_block(99_999), 100_000);
        assert_eq!(patient_block(100_000), 200_000);
        assert_eq!(patient_block(123_456), 200_000);
        let size = patient_block(1_234);
        let (lo0, hi0) = bounds(0, size);
        let (lo1, hi1) = bounds(1, size);
        let (lo2, _) = bounds(2, size);
        assert_eq!((lo0, hi0), (1, 99_999));
        assert_eq!((lo1, hi1), (100_000, 199_999));
        assert_eq!(lo2, 200_000);
        assert!(hi0 < lo1 && hi1 < lo2);
        let (_, big) = bounds(9_000, ROW_BLOCK);
        assert!(big > 0, "pas de débordement");
        assert_eq!(next_post(&[]), 0);
        assert_eq!(next_post(&[0, 1, 3]), 4);
    }

    /// **Un enregistrement tient dans la limite** : une ligne trop longue
    /// se découpe, se relit dans l'ordre, et une valeur qui ne tient pas
    /// seule est nommée.
    #[test]
    fn a_long_row_is_cut_by_columns_and_an_oversize_value_is_named() {
        let long = "x".repeat(900);
        let mut row = Map::new();
        row.insert("id".to_owned(), json!(5));
        for i in 0..30 {
            row.insert(format!("c{i}"), json!(long));
        }
        let op = Op {
            file: File::Main,
            table: "drugs".to_owned(),
            kind: Kind::Insert,
            key: m(json!({"id":5})),
            old: Map::new(),
            new: row.clone(),
        };
        let (lots, refused) = batches(std::slice::from_ref(&op), 4_000);
        assert!(refused.is_empty());
        assert!(lots.len() > 1);
        assert!(lots.iter().all(|l| l.len() <= 4_000), "chaque lot tient");
        let back: Vec<Op> = lots.iter().flat_map(|l| decode(l).unwrap()).collect();
        assert_eq!(back[0].kind, Kind::Insert);
        assert!(back[1..].iter().all(|o| o.kind == Kind::Patch));
        let mut whole = Map::new();
        for o in &back {
            whole.extend(o.new.clone());
        }
        assert_eq!(whole, row, "rien de perdu, rien de changé");

        let mut huge = Map::new();
        huge.insert("id".to_owned(), json!(6));
        huge.insert("body".to_owned(), json!("y".repeat(5_000)));
        let op = Op {
            new: huge,
            key: m(json!({"id":6})),
            ..op
        };
        let (lots, refused) = batches(&[op], 4_000);
        assert!(lots.is_empty());
        assert_eq!(refused.len(), 1);
    }

    /// Ce qui revient du réseau des postes ne peut nommer qu'une table qui
    /// voyage et des colonnes au nom simple : ces noms finissent en SQL.
    #[test]
    fn only_travelling_tables_and_plain_column_names_come_in() {
        let ok = Op {
            file: File::Main,
            table: "patients".to_owned(),
            kind: Kind::Update,
            key: m(json!({"id":1})),
            old: m(json!({"phone":"1"})),
            new: m(json!({"phone":"2"})),
        };
        let (lots, _) = batches(std::slice::from_ref(&ok), LIMIT);
        assert_eq!(decode(&lots[0]).unwrap(), vec![ok.clone()]);
        for (table, col) in [
            ("telemetry", "n"),
            ("sync_local", "value"),
            ("patients", "phone = '', notes"),
            ("patients", "Phone"),
        ] {
            let bad = Op {
                table: table.to_owned(),
                new: m(json!({ col: "2" })),
                ..ok.clone()
            };
            let (lots, _) = batches(&[bad], LIMIT);
            assert!(decode(&lots[0]).unwrap().is_empty(), "{table}.{col}");
        }
        assert!(decode(b"{}").is_none());
    }

    #[test]
    fn a_table_is_either_travelling_or_local_never_both() {
        for (file, name, why) in LOCAL {
            assert!(table(*file, name).is_none(), "{name}");
            assert!(!why.is_empty());
        }
        for t in TABLES {
            assert!(plain_identifier(t.name));
            assert_eq!(
                TABLES
                    .iter()
                    .filter(|u| u.file == t.file && u.name == t.name)
                    .count(),
                1,
                "{}",
                t.name
            );
        }
    }
}
