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

/// Une version d'un champ.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edit {
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
    let k = key(card);
    edits
        .iter()
        .filter(|e| e.card == k && e.field == field)
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
    let k = key(card);
    let mut out: Vec<&Edit> = Vec::new();
    for field in SHARED_FIELDS {
        // The field's last version, when it came from elsewhere and the
        // local card does not carry it. A local version written after it —
        // « garder la mienne », or an edit here — closes the question.
        if let Some(e) = edits
            .iter()
            .rfind(|e| e.card == k && e.field == field)
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
        "t": "version",
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
    if v.get("v")?.as_u64()? != 1 || v.get("t")?.as_str()? != "version" {
        return None;
    }
    let text = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_owned();
    let field = text("field");
    if !SHARED_FIELDS.contains(&field.as_str()) {
        return None;
    }
    let uid = text("uid");
    let card = text("card");
    if uid.is_empty() || card.is_empty() {
        return None;
    }
    let officine = text("officine");
    Some(Edit {
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
}
