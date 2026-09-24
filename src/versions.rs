//! L'historique des fiches médicament, **partagé entre les officines** :
//! chaque modification d'un champ est une version, chaque officine du
//! réseau peut modifier, et chacune peut revenir à n'importe quelle
//! version de n'importe quel champ.
//!
//! Quatre règles, un test chacune.
//!
//! * **Une version ne se réécrit pas.** Le journal ne fait que grandir :
//!   revenir en arrière, c'est écrire une version de plus dont la valeur
//!   est l'ancienne — elle voyage comme les autres, et l'historique garde
//!   tout, y compris le retour.
//! * **Une version reçue ne s'applique que sur la valeur qu'elle
//!   remplaçait.** Si la fiche locale dit autre chose, c'est que quelqu'un
//!   ici l'a modifiée entre-temps : la version reçue attend, « à
//!   arbitrer », et rien n'est écrasé en silence. C'est la règle de
//!   `sync/` — un conflit se montre, il ne se tranche pas.
//! * **Deux versions qui remplacent la même sont une divergence** : deux
//!   officines ont corrigé le même champ sans s'être vues. Les deux
//!   restent, chacune nommée, et c'est l'officine qui choisit.
//! * **Les notes de l'équipe ne voyagent pas** : elles appartiennent à
//!   l'officine. Et aucun patient n'est dans une fiche — c'est ce qui
//!   permet à ce journal-là de sortir.
//!
//! Une fiche se reconnaît d'une officine à l'autre à son nom replié : les
//! fiches livrées ont les mêmes partout. Pur, testé, sans base.
//!
//! **Cinq sortes d'entrées** suivent les mêmes règles ([`Kind`]) : les
//! fiches médicament, les préparations du codex, les protocoles, les
//! lignes proposées après un TROD — sous leur protocole et leur nom
//! ([`trod_entry`]) — et les vaccins du catalogue, sous leur libellé ;
//! le rang des deux dernières reste propre à chaque officine. Une
//! préparation voyage champ par champ comme une fiche ; un protocole
//! voyage en deux champs — son sujet et son **arbre entier**, écrit sous
//! une forme canonique ([`tree_json`]) : deux officines qui ont le même
//! arbre écrivent les mêmes octets, et une version d'arbre ne s'applique
//! que sur l'arbre qu'elle remplaçait.

/// Les champs d'une fiche que le réseau partage — tous sauf les notes
/// de l'équipe.
pub const SHARED_FIELDS: [&str; 25] = [
    "name",
    "dci",
    "class",
    "dosage",
    "ddi",
    "iup",
    "antidote",
    "half_life",
    "auc",
    "elimination",
    "renal",
    "pregnancy",
    "indications",
    "mechanism",
    "contraindications",
    "adverse",
    "monitoring",
    "sources",
    "status",
    "smr",
    "tags",
    "toxicity",
    "forms",
    "missed_dose",
    "red_flags",
];

/// Les champs d'une préparation du codex que le réseau partage. Le nom
/// est ce qui la reconnaît d'une officine à l'autre.
pub const CODEX_FIELDS: [&str; 9] = [
    "form",
    "indication",
    "formula",
    "yield_amount",
    "method",
    "conservation",
    "caution",
    "tags",
    "sources",
];

/// Les champs d'un protocole : son sujet, et son arbre entier.
pub const PROTOCOL_FIELDS: [&str; 2] = ["subject", "arbre"];

/// Les champs d'une ligne de TROD que le réseau partage : tout ce que
/// l'ordonnance propose, sauf son rang — l'ordre des lignes est celui de
/// chaque officine. **Le nom en est un** : la ligne voyage sous son nom
/// d'origine (`trod_lines.origin`), si bien qu'un renommage est une
/// version comme une autre et non une ligne de plus chez les autres.
pub const TROD_FIELDS: [&str; 8] = [
    "name",
    "situation",
    "posologies",
    "caution",
    "min_age",
    "max_age",
    "sex",
    "pregnancy",
];

/// Les champs d'un vaccin du catalogue que le réseau partage : le code
/// que le calendrier lit et le schéma. Son libellé est ce qui le
/// reconnaît d'une officine à l'autre ; son rang reste celui de chacune.
pub const VACCIN_FIELDS: [&str; 3] = ["label", "code", "schedule"];

/// Le nom sous lequel une ligne de TROD voyage : son protocole et son
/// nom — « Amoxicilline 1 g » n'est pas la même ligne dans l'angine et
/// dans une autre indication.
pub fn trod_entry(protocol: &str, name: &str) -> String {
    format!("{} · {}", protocol.trim(), name.trim())
}

/// Le protocole et le nom d'une ligne de TROD, depuis son nom de voyage.
pub fn split_trod_entry(entry: &str) -> Option<(&str, &str)> {
    let (protocol, name) = entry.split_once(" · ")?;
    let (protocol, name) = (protocol.trim(), name.trim());
    (!protocol.is_empty() && !name.is_empty()).then_some((protocol, name))
}

/// Ce qu'une version modifie.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Kind {
    /// Une fiche médicament.
    #[default]
    Fiche,
    /// Une préparation du codex.
    Codex,
    /// Un protocole, son arbre compris.
    Protocole,
    /// Une ligne proposée après un TROD positif.
    Trod,
    /// Un vaccin du catalogue de l'officine.
    Vaccin,
}

impl Kind {
    pub fn key(self) -> &'static str {
        match self {
            Kind::Fiche => "fiche",
            Kind::Codex => "codex",
            Kind::Protocole => "protocole",
            Kind::Trod => "trod",
            Kind::Vaccin => "vaccin",
        }
    }

    pub fn from_key(key: &str) -> Kind {
        match key {
            "codex" => Kind::Codex,
            "protocole" => Kind::Protocole,
            "trod" => Kind::Trod,
            "vaccin" => Kind::Vaccin,
            _ => Kind::Fiche,
        }
    }

    /// Le type de charge qui voyage. Une fiche garde `version`, que les
    /// versions d'avant connaissent ; les deux autres ont le leur, qu'une
    /// version d'avant ignore au lieu de le prendre pour une fiche.
    fn payload(self) -> &'static str {
        match self {
            Kind::Fiche => "version",
            Kind::Codex => "codex",
            Kind::Protocole => "protocole",
            Kind::Trod => "trod",
            Kind::Vaccin => "vaccin",
        }
    }

    fn from_payload(tag: &str) -> Option<Kind> {
        match tag {
            "version" => Some(Kind::Fiche),
            "codex" => Some(Kind::Codex),
            "protocole" => Some(Kind::Protocole),
            "trod" => Some(Kind::Trod),
            "vaccin" => Some(Kind::Vaccin),
            _ => None,
        }
    }

    /// Les champs que le réseau partage pour cette sorte d'entrée.
    pub fn fields(self) -> &'static [&'static str] {
        match self {
            Kind::Fiche => &SHARED_FIELDS,
            Kind::Codex => &CODEX_FIELDS,
            Kind::Protocole => &PROTOCOL_FIELDS,
            Kind::Trod => &TROD_FIELDS,
            Kind::Vaccin => &VACCIN_FIELDS,
        }
    }
}

/// Une version d'un champ.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edit {
    /// Fiche, préparation ou protocole.
    pub kind: Kind,
    /// `<base>:<numéro>` — global, comme au journal des ruptures.
    pub uid: String,
    pub day: String,
    /// La fiche, par son nom replié **au moment de la modification**.
    pub card: String,
    /// Le nom de la fiche tel qu'il s'écrit, pour en créer une chez une
    /// officine qui ne l'a pas.
    pub card_name: String,
    pub field: String,
    pub value: String,
    /// La valeur qu'elle remplaçait.
    pub previous: String,
    /// La version qu'elle remplace, par son `uid` ; vide pour la première
    /// version connue d'un champ.
    pub corrects: String,
    /// Un retour à une version antérieure.
    pub revert: bool,
    pub operator: String,
    /// L'officine d'où elle vient ; vide : celle-ci.
    pub source: String,
}

/// La clé d'une fiche.
pub fn key(name: &str) -> String {
    crate::fuzzy::sort_key(name.trim())
}

/// Les versions d'un champ d'une fiche, **de la plus ancienne à la plus
/// récente** dans l'ordre où elles ont été reçues ou écrites.
pub fn history<'a>(edits: &'a [Edit], card: &str, field: &str) -> Vec<&'a Edit> {
    history_of(edits, Kind::Fiche, card, field)
}

/// La même chose pour n'importe quelle sorte d'entrée.
pub fn history_of<'a>(edits: &'a [Edit], kind: Kind, card: &str, field: &str) -> Vec<&'a Edit> {
    let k = key(card);
    edits
        .iter()
        .filter(|e| e.kind == kind && e.card == k && e.field == field)
        .collect()
}

/// La dernière version d'un champ, celle que la prochaine remplacera.
pub fn head<'a>(edits: &'a [Edit], card: &str, field: &str) -> Option<&'a Edit> {
    history(edits, card, field).into_iter().next_back()
}

/// Une version reçue s'applique-t-elle ? **Seulement si la fiche locale
/// dit encore ce qu'elle remplaçait** — ou dit déjà ce qu'elle apporte,
/// auquel cas il n'y a rien à faire.
pub fn applies(edit: &Edit, local: &str) -> Applies {
    if local == edit.value {
        Applies::Already
    } else if local == edit.previous {
        Applies::Clean
    } else {
        Applies::Conflict
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Applies {
    /// La fiche dit déjà cette valeur.
    Already,
    /// Elle dit ce que la version remplaçait : on l'applique.
    Clean,
    /// Elle dit autre chose : la version attend l'arbitrage.
    Conflict,
}

/// Les divergences d'un champ : les versions qui remplacent la même
/// version qu'une autre. Chaque paire est rendue une fois.
pub fn divergences<'a>(edits: &'a [Edit], card: &str, field: &str) -> Vec<(&'a Edit, &'a Edit)> {
    let h = history(edits, card, field);
    let mut out = Vec::new();
    for (i, a) in h.iter().enumerate() {
        for b in &h[i + 1..] {
            if !a.corrects.is_empty() && a.corrects == b.corrects {
                out.push((*a, *b));
            }
        }
    }
    out
}

/// Les versions reçues qui n'ont pas pu s'appliquer à la fiche locale,
/// par champ : ce que l'écran propose d'arbitrer. `local` rend la valeur
/// locale d'un champ.
pub fn pending<'a>(
    edits: &'a [Edit],
    card: &str,
    local: &dyn Fn(&str) -> Option<String>,
) -> Vec<&'a Edit> {
    pending_of(edits, Kind::Fiche, card, local)
}

/// La même chose pour n'importe quelle sorte d'entrée.
pub fn pending_of<'a>(
    edits: &'a [Edit],
    kind: Kind,
    card: &str,
    local: &dyn Fn(&str) -> Option<String>,
) -> Vec<&'a Edit> {
    let k = key(card);
    let mut out: Vec<&Edit> = Vec::new();
    for &field in kind.fields() {
        // The field's last version, when it came from elsewhere and the
        // local card does not carry it. A local version written after it —
        // « garder la mienne », or an edit here — closes the question.
        if let Some(e) = edits
            .iter()
            .rfind(|e| e.kind == kind && e.card == k && e.field == field)
            .filter(|e| !e.source.is_empty())
        {
            if let Some(value) = local(field) {
                if applies(e, &value) == Applies::Conflict {
                    out.push(e);
                }
            }
        }
    }
    out
}

/// Ce qu'une version devient pour voyager vers le réseau.
pub fn encode(e: &Edit, officine: &str) -> Vec<u8> {
    serde_json::json!({
        "v": 1,
        "t": e.kind.payload(),
        "uid": e.uid,
        "day": e.day,
        "card": e.card,
        "card_name": e.card_name,
        "field": e.field,
        "value": e.value,
        "previous": e.previous,
        "corrects": e.corrects,
        "revert": e.revert,
        "operator": e.operator,
        "officine": officine,
    })
    .to_string()
    .into_bytes()
}

/// L'inverse. Un champ que ce module ne partage pas n'entre pas — une
/// version postérieure qui partagerait les notes de l'équipe ne les ferait
/// pas entrer ici.
pub fn decode(bytes: &[u8]) -> Option<Edit> {
    let v: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    if v.get("v")?.as_u64()? != 1 {
        return None;
    }
    let kind = Kind::from_payload(v.get("t")?.as_str()?)?;
    let text = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_owned();
    let field = text("field");
    if !kind.fields().contains(&field.as_str()) {
        return None;
    }
    // An arbre that does not read as one never enters.
    if field == "arbre" && !text("value").is_empty() && tree_nodes(&text("value")).is_none() {
        return None;
    }
    let uid = text("uid");
    let card = text("card");
    if uid.is_empty() || card.is_empty() {
        return None;
    }
    let officine = text("officine");
    Some(Edit {
        kind,
        uid,
        day: text("day"),
        card,
        card_name: text("card_name"),
        field,
        value: text("value"),
        previous: text("previous"),
        corrects: text("corrects"),
        revert: v.get("revert").and_then(|x| x.as_bool()).unwrap_or(false),
        operator: text("operator"),
        source: if officine.trim().is_empty() {
            crate::strings::tr("rupt_unnamed_officine").to_owned()
        } else {
            officine
        },
    })
}

/// Ce qu'une entrée a changé ici depuis ce que son historique connaît :
/// les versions à écrire. `local` donne chaque champ tel qu'il est,
/// `origin` ce qu'il valait avant toute version — le contenu livré pour
/// une entrée livrée, vide pour une entrée de l'officine.
///
/// **Une valeur déjà connue n'est pas une modification** : ni l'origine,
/// ni la valeur d'une version de l'historique. C'est ce qui empêche une
/// version reçue en attente d'arbitrage d'être « répondue » par la valeur
/// locale qu'elle n'a pas remplacée. Une nouvelle version remplace la
/// dernière de l'historique — celle que les autres officines ont, le plus
/// probablement.
pub fn changes(
    kind: Kind,
    card_name: &str,
    local: &[(&str, String)],
    origin: &dyn Fn(&str) -> String,
    edits: &[Edit],
) -> Vec<Edit> {
    let mut out = Vec::new();
    for (field, value) in local {
        if !kind.fields().contains(field) {
            continue;
        }
        let h = history_of(edits, kind, card_name, field);
        let start = origin(field);
        let known = *value == start || h.iter().any(|e| e.value == *value);
        if known {
            continue;
        }
        let (previous, corrects) = match h.last() {
            Some(last) => (last.value.clone(), last.uid.clone()),
            None => (start, String::new()),
        };
        out.push(Edit {
            kind,
            uid: String::new(),
            day: String::new(),
            card: key(card_name),
            card_name: card_name.to_owned(),
            field: (*field).to_owned(),
            value: value.clone(),
            previous,
            corrects,
            revert: false,
            operator: String::new(),
            source: String::new(),
        });
    }
    out
}

/// Un nœud d'arbre de protocole, sans identifiant : ce qui voyage.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeNode {
    /// `QUESTION` ou `ACTION`, comme la base les écrit.
    pub kind: String,
    pub text: String,
    pub yes: Vec<TreeNode>,
    pub no: Vec<TreeNode>,
}

fn node_json(n: &TreeNode) -> serde_json::Value {
    serde_json::json!({
        "k": n.kind,
        "t": n.text,
        "y": n.yes.iter().map(node_json).collect::<Vec<_>>(),
        "n": n.no.iter().map(node_json).collect::<Vec<_>>(),
    })
}

/// L'arbre sous sa forme canonique : le même arbre donne les mêmes
/// octets, quels que soient les identifiants et l'ordre d'écriture.
pub fn tree_json(roots: &[TreeNode]) -> String {
    serde_json::Value::Array(roots.iter().map(node_json).collect()).to_string()
}

/// Un nœud tel que la base le rend : identifiant, parent, branche
/// (`ROOT`, `YES`, `NO`), sorte, texte, position.
pub type NodeRow = (i64, Option<i64>, String, String, String, i64);

/// L'arbre d'une liste de nœuds tels que la base les rend.
pub fn tree_from_rows(rows: &[NodeRow]) -> Vec<TreeNode> {
    fn under(rows: &[NodeRow], parent: Option<i64>, branch: &str, depth: usize) -> Vec<TreeNode> {
        if depth > 64 {
            return Vec::new();
        }
        let mut here: Vec<&NodeRow> = rows
            .iter()
            .filter(|r| r.1 == parent && (parent.is_none() || r.2 == branch))
            .collect();
        here.sort_by_key(|r| (r.5, r.0));
        here.into_iter()
            .map(|r| TreeNode {
                kind: r.3.clone(),
                text: r.4.clone(),
                yes: under(rows, Some(r.0), "YES", depth + 1),
                no: under(rows, Some(r.0), "NO", depth + 1),
            })
            .collect()
    }
    under(rows, None, "ROOT", 0)
}

/// L'inverse de [`tree_json`] ; rien pour un texte qui n'est pas un arbre.
pub fn tree_nodes(json: &str) -> Option<Vec<TreeNode>> {
    fn read(v: &serde_json::Value, depth: usize) -> Option<TreeNode> {
        if depth > 64 {
            return None;
        }
        let list = |k: &str| -> Option<Vec<TreeNode>> {
            v.get(k)?
                .as_array()?
                .iter()
                .map(|c| read(c, depth + 1))
                .collect()
        };
        let kind = v.get("k")?.as_str()?;
        if kind != "QUESTION" && kind != "ACTION" {
            return None;
        }
        Some(TreeNode {
            kind: kind.to_owned(),
            text: v.get("t")?.as_str()?.to_owned(),
            yes: list("y")?,
            no: list("n")?,
        })
    }
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    v.as_array()?.iter().map(|n| read(n, 0)).collect()
}

/// L'arbre en lignes lisibles, pour l'historique : une question suivie de
/// ses deux branches, en retrait.
pub fn tree_text(json: &str) -> String {
    fn walk(out: &mut Vec<String>, nodes: &[TreeNode], depth: usize, tag: &str) {
        for (i, n) in nodes.iter().enumerate() {
            let lead = if i == 0 { tag } else { "" };
            let mark = if n.kind == "QUESTION" { "? " } else { "- " };
            out.push(format!("{}{lead}{mark}{}", "    ".repeat(depth), n.text));
            walk(out, &n.yes, depth + 1, crate::strings::tr("proto_yes_tag"));
            walk(out, &n.no, depth + 1, crate::strings::tr("proto_no_tag"));
        }
    }
    match tree_nodes(json) {
        Some(nodes) => {
            let mut out = Vec::new();
            walk(&mut out, &nodes, 0, "");
            out.join("\n")
        }
        None => json.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ed(
        uid: &str,
        field: &str,
        value: &str,
        previous: &str,
        corrects: &str,
        source: &str,
    ) -> Edit {
        Edit {
            kind: Kind::Fiche,
            uid: uid.to_owned(),
            day: "2026-09-23".to_owned(),
            card: key("Eliquis"),
            card_name: "Eliquis".to_owned(),
            field: field.to_owned(),
            value: value.to_owned(),
            previous: previous.to_owned(),
            corrects: corrects.to_owned(),
            revert: false,
            operator: "CL".to_owned(),
            source: source.to_owned(),
        }
    }

    /// **Une version ne se réécrit pas** : revenir en arrière ajoute une
    /// version, et l'historique garde les trois.
    #[test]
    fn a_revert_is_one_more_version_and_the_history_keeps_all() {
        let a = ed("a:1", "dosage", "5 mg x2", "", "", "");
        let b = ed("a:2", "dosage", "2,5 mg x2", "5 mg x2", "a:1", "");
        let back = Edit {
            revert: true,
            ..ed("a:3", "dosage", "5 mg x2", "2,5 mg x2", "a:2", "")
        };
        let all = [a, b, back];
        let h = history(&all, "ELIQUIS", "dosage");
        assert_eq!(h.len(), 3);
        assert_eq!(
            head(&all, "Eliquis", "dosage").map(|e| e.value.as_str()),
            Some("5 mg x2")
        );
        assert!(h[2].revert);
    }

    /// **Une version reçue ne s'applique que sur la valeur qu'elle
    /// remplaçait** ; sinon elle attend l'arbitrage.
    #[test]
    fn an_incoming_version_applies_only_on_what_it_replaced() {
        let e = ed(
            "b:1",
            "dosage",
            "2,5 mg x2",
            "5 mg x2",
            "",
            "Pharmacie du Port",
        );
        assert_eq!(applies(&e, "5 mg x2"), Applies::Clean);
        assert_eq!(applies(&e, "2,5 mg x2"), Applies::Already);
        assert_eq!(applies(&e, "10 mg"), Applies::Conflict);
        let all = [e];
        let local = |f: &str| (f == "dosage").then(|| "10 mg".to_owned());
        assert_eq!(pending(&all, "Eliquis", &local).len(), 1);
        let local_same = |f: &str| (f == "dosage").then(|| "5 mg x2".to_owned());
        assert!(pending(&all, "Eliquis", &local_same).is_empty());
    }

    /// **Deux versions qui remplacent la même sont une divergence**, et
    /// les deux restent.
    #[test]
    fn two_versions_replacing_the_same_one_are_a_divergence() {
        let a = ed("a:1", "renal", "adapter < 30", "", "", "");
        let b = ed(
            "b:1",
            "renal",
            "adapter < 25",
            "adapter < 30",
            "a:1",
            "Pharmacie du Port",
        );
        let c = ed(
            "c:1",
            "renal",
            "éviter < 15",
            "adapter < 30",
            "a:1",
            "Pharmacie du Lac",
        );
        let all = [a, b, c];
        let d = divergences(&all, "Eliquis", "renal");
        assert_eq!(d.len(), 1);
        assert_eq!((d[0].0.uid.as_str(), d[0].1.uid.as_str()), ("b:1", "c:1"));
        assert_eq!(history(&all, "Eliquis", "renal").len(), 3);
    }

    /// **Les notes de l'équipe ne voyagent pas**, et ce qui voyage revient
    /// tel quel, avec l'officine pour source.
    #[test]
    fn the_team_notes_never_travel_and_a_version_comes_back_whole() {
        assert!(!SHARED_FIELDS.contains(&"notes"));
        let e = ed("a:9", "dosage", "5 mg x2", "", "", "");
        let back = decode(&encode(&e, "Pharmacie du Centre")).expect("lu");
        assert_eq!(back.source, "Pharmacie du Centre");
        assert_eq!(
            Edit {
                source: String::new(),
                ..back
            },
            e
        );
        let notes = ed("a:10", "notes", "ne pas commander", "", "", "");
        assert!(decode(&encode(&notes, "X")).is_none());
        assert!(decode(b"{}").is_none());
    }

    /// **Une valeur déjà connue n'est pas une modification** — l'origine,
    /// ou la valeur d'une version ; une valeur neuve remplace la dernière
    /// version, ou l'origine s'il n'y en a pas.
    #[test]
    fn only_an_unknown_value_is_a_change_and_it_replaces_the_last_version() {
        let origin = |f: &str| {
            if f == "formula" {
                "acide | 2 g".to_owned()
            } else {
                String::new()
            }
        };
        let local = |v: &str| vec![("formula", v.to_owned()), ("caution", String::new())];
        assert!(changes(Kind::Codex, "Vaseline", &local("acide | 2 g"), &origin, &[]).is_empty());
        let first = changes(Kind::Codex, "Vaseline", &local("acide | 3 g"), &origin, &[]);
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].previous, "acide | 2 g");
        assert_eq!(first[0].kind, Kind::Codex);
        // A received version waits: the local value, the origin, is known —
        // no version answers it behind the officine's back.
        let theirs = Edit {
            kind: Kind::Codex,
            card: key("Vaseline"),
            field: "formula".to_owned(),
            value: "acide | 5 g".to_owned(),
            previous: "acide | 2 g".to_owned(),
            source: "Pharmacie du Port".to_owned(),
            ..ed("b:1", "formula", "", "", "", "")
        };
        let all = [theirs];
        assert!(changes(
            Kind::Codex,
            "Vaseline",
            &local("acide | 2 g"),
            &origin,
            &all
        )
        .is_empty());
        let next = changes(
            Kind::Codex,
            "Vaseline",
            &local("acide | 4 g"),
            &origin,
            &all,
        );
        assert_eq!(
            (next[0].previous.as_str(), next[0].corrects.as_str()),
            ("acide | 5 g", "b:1")
        );
        // A drug card's history is not a preparation's.
        assert!(history_of(&all, Kind::Fiche, "Vaseline", "formula").is_empty());
        assert!(!Kind::Codex.fields().contains(&"name"));
    }

    /// **Le même arbre donne les mêmes octets**, quels que soient les
    /// identifiants et l'ordre d'écriture ; il se relit tel quel, et un
    /// texte qui n'est pas un arbre ne passe pas.
    #[test]
    fn a_tree_is_written_canonically_and_reads_back() {
        let rows = |ids: [i64; 3]| {
            vec![
                (
                    ids[0],
                    None,
                    "ROOT".to_owned(),
                    "QUESTION".to_owned(),
                    "Fièvre ?".to_owned(),
                    0,
                ),
                (
                    ids[1],
                    Some(ids[0]),
                    "YES".to_owned(),
                    "ACTION".to_owned(),
                    "Orienter".to_owned(),
                    0,
                ),
                (
                    ids[2],
                    Some(ids[0]),
                    "NO".to_owned(),
                    "ACTION".to_owned(),
                    "Conseil".to_owned(),
                    0,
                ),
            ]
        };
        let a = tree_json(&tree_from_rows(&rows([1, 2, 3])));
        let mut shuffled = rows([40, 7, 12]);
        shuffled.reverse();
        let b = tree_json(&tree_from_rows(&shuffled));
        assert_eq!(a, b);
        let back = tree_nodes(&a).unwrap();
        assert_eq!(back.len(), 1);
        assert_eq!(back[0].yes[0].text, "Orienter");
        assert_eq!(tree_json(&back), a);
        assert!(tree_text(&a).contains("Fièvre"));
        assert!(tree_nodes("pas un arbre").is_none());
        assert!(tree_nodes(r#"[{"k":"AUTRE","t":"x","y":[],"n":[]}]"#).is_none());
        // A protocol version travels under its own tag, and a malformed
        // tree never enters.
        let e = Edit {
            kind: Kind::Protocole,
            field: "arbre".to_owned(),
            value: a.clone(),
            ..ed("a:1", "arbre", "", "", "", "")
        };
        let bytes = encode(&e, "X");
        assert!(String::from_utf8_lossy(&bytes).contains("\"t\":\"protocole\""));
        assert_eq!(decode(&bytes).unwrap().kind, Kind::Protocole);
        let bad = Edit {
            value: "{".to_owned(),
            ..e
        };
        assert!(decode(&encode(&bad, "X")).is_none());
    }
}
