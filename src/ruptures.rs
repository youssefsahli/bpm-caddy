//! Les ruptures, et ce que les pharmaciens ont mis à la place.
//!
//! « Diprosone est en rupture — qu'est-ce que les autres ont donné ? » est
//! une question qu'un pharmacien arrivé la semaine dernière ne peut pas
//! poser au logiciel : la réponse est dans la tête de ceux qui étaient là
//! le mois dernier, et dans celle des collègues de l'officine d'à côté.
//! Ce module la rend lisible. Il tient un **journal** : une rupture
//! signalée, une rupture levée, une substitution faite — qui, quand, par
//! quoi, et comment ça s'est passé — et il en tire deux lectures : ce
//! produit est-il en rupture, et qu'a-t-on essayé à sa place.
//!
//! Trois règles, un test chacune.
//!
//! * **Ce qui a été tenté n'est pas une équivalence.** Le journal dit ce
//!   que des collègues ont fait, et combien de fois cela a tenu ; il ne
//!   dit pas que deux produits se valent. Une substitution se décide
//!   devant l'ordonnance — dosage, forme, puissance, patient —, et la
//!   liste n'est jamais présentée autrement que comme un historique.
//! * **Un événement ne se réécrit pas.** Le journal est fait pour voyager
//!   d'officine en officine (voir `sync/`) : un enregistrement réécrit
//!   chez l'une et pas chez l'autre, ce sont deux vérités. Une erreur se
//!   retire par un **retrait** qui la nomme, comme au registre.
//! * **Une rupture vieillit.** Signalée il y a quatre mois et jamais
//!   levée, elle ne dit plus rien : quelqu'un a oublié de la lever. Au-delà
//!   de l'horizon elle n'est plus « en cours » — elle reste au journal.
//!
//! Aucun patient n'y figure, et c'est ce qui permet de le partager entre
//! officines : un produit, un autre produit, une date, des initiales, le
//! nom de l'officine. Pur, testé, sans horloge : le jour est passé.

/// Ce que dit un événement du journal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Le produit manque — rupture ou tension d'approvisionnement.
    Rupture,
    /// Le produit est revenu.
    Levee,
    /// Délivré à la place du produit.
    Substitution,
    /// Retire l'événement qu'il nomme : une erreur ne s'efface pas.
    Retrait,
}

impl Kind {
    pub const ALL: [Kind; 4] = [
        Kind::Rupture,
        Kind::Levee,
        Kind::Substitution,
        Kind::Retrait,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Kind::Rupture => "RUPTURE",
            Kind::Levee => "LEVEE",
            Kind::Substitution => "SUBSTITUTION",
            Kind::Retrait => "RETRAIT",
        }
    }

    pub fn from_key(key: &str) -> Option<Kind> {
        Kind::ALL.into_iter().find(|k| k.key() == key)
    }
}

/// Comment une substitution s'est passée — **ce qu'on sait**, et « non
/// dit » en est une valeur : un résultat qu'on ne connaît pas ne se
/// compte ni comme un succès ni comme un échec.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Unknown,
    /// Délivré, et ça a tenu.
    Accepted,
    /// Le patient n'en a pas voulu.
    RefusedByPatient,
    /// Le prescripteur, joint, a refusé ou changé autrement.
    RefusedByPrescriber,
    /// Délivré, et c'est revenu : mal toléré, inefficace, mal compris.
    Failed,
}

impl Outcome {
    pub const ALL: [Outcome; 5] = [
        Outcome::Unknown,
        Outcome::Accepted,
        Outcome::RefusedByPatient,
        Outcome::RefusedByPrescriber,
        Outcome::Failed,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Outcome::Unknown => "",
            Outcome::Accepted => "ACCEPTE",
            Outcome::RefusedByPatient => "REFUS_PATIENT",
            Outcome::RefusedByPrescriber => "REFUS_PRESCRIPTEUR",
            Outcome::Failed => "ECHEC",
        }
    }

    pub fn from_key(key: &str) -> Outcome {
        Outcome::ALL
            .into_iter()
            .find(|o| o.key() == key.trim())
            .unwrap_or(Outcome::Unknown)
    }

    pub fn label(self) -> &'static str {
        use crate::strings::tr;
        match self {
            Outcome::Unknown => tr("rupt_outcome_unknown"),
            Outcome::Accepted => tr("rupt_outcome_accepted"),
            Outcome::RefusedByPatient => tr("rupt_outcome_patient"),
            Outcome::RefusedByPrescriber => tr("rupt_outcome_prescriber"),
            Outcome::Failed => tr("rupt_outcome_failed"),
        }
    }
}

/// Un événement du journal, tel que la base le garde.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Event {
    /// Identifiant **global** : `<base>:<numéro>`, où `<base>` est tiré au
    /// hasard une fois par base. C'est ce qui permet à deux officines de
    /// recevoir le même événement deux fois sans le compter deux fois.
    pub uid: String,
    /// ISO `AAAA-MM-JJ`.
    pub day: String,
    pub kind: Kind,
    /// Le produit en cause — tel que la fiche le nomme, et sa DCI.
    pub product: String,
    pub product_dci: String,
    /// Pour une substitution : ce qui a été donné à la place.
    pub other: String,
    pub other_dci: String,
    pub outcome: Outcome,
    pub note: String,
    /// Les initiales de qui l'a noté — un collègue, jamais un patient.
    pub operator: String,
    /// L'officine d'où vient l'événement ; vide : celle-ci.
    pub source: String,
    /// Pour un retrait : l'`uid` de l'événement retiré.
    pub refers: String,
}

/// La clé sous laquelle un produit se reconnaît d'une officine à l'autre :
/// son nom replié. Deux officines écrivent « Diprosone » et « DIPROSONE ».
pub fn key(name: &str) -> String {
    crate::fuzzy::sort_key(name.trim())
}

/// Combien de jours une rupture signalée reste « en cours » sans avoir été
/// levée.
pub const HORIZON_DAYS: i64 = 90;

/// Les événements qui tiennent : ceux qu'aucun retrait ne nomme, et pas
/// les retraits eux-mêmes.
pub fn standing(events: &[Event]) -> Vec<&Event> {
    let withdrawn: std::collections::HashSet<&str> = events
        .iter()
        .filter(|e| e.kind == Kind::Retrait)
        .map(|e| e.refers.as_str())
        .collect();
    events
        .iter()
        .filter(|e| e.kind != Kind::Retrait && !withdrawn.contains(e.uid.as_str()))
        .collect()
}

/// Où en est un produit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shortage {
    pub product: String,
    /// Le premier signalement de l'épisode en cours.
    pub since: String,
    /// Le dernier signalement.
    pub last: String,
    /// Combien d'officines l'ont signalé dans l'épisode (celle-ci
    /// comprise).
    pub sources: usize,
}

/// Le produit est-il en rupture à `today` ?
///
/// **Signalée et ni levée depuis, ni plus vieille que l'horizon.** Une
/// levée par n'importe quelle officine clôt l'épisode ; un signalement
/// postérieur en ouvre un autre. Au-delà de [`HORIZON_DAYS`] sans nouveau
/// signalement, la rupture n'est plus en cours : quelqu'un a oublié de la
/// lever, et le dire encore serait un mensonge qui dure.
pub fn shortage(events: &[Event], product: &str, today: &str) -> Option<Shortage> {
    let k = key(product);
    let mut mine: Vec<&Event> = standing(events)
        .into_iter()
        .filter(|e| matches!(e.kind, Kind::Rupture | Kind::Levee) && key(&e.product) == k)
        .collect();
    mine.sort_by(|a, b| a.day.cmp(&b.day).then(a.uid.cmp(&b.uid)));
    // L'épisode en cours : ce qui suit la dernière levée.
    let start = mine
        .iter()
        .rposition(|e| e.kind == Kind::Levee)
        .map_or(0, |i| i + 1);
    let open: Vec<&&Event> = mine[start..].iter().collect();
    let first = open.first()?;
    let last = open.last()?;
    let age = crate::date::days_between(&last.day, today)?;
    if !(0..=HORIZON_DAYS).contains(&age) {
        return None;
    }
    let sources: std::collections::HashSet<&str> = open.iter().map(|e| e.source.as_str()).collect();
    Some(Shortage {
        product: last.product.clone(),
        since: first.day.clone(),
        last: last.day.clone(),
        sources: sources.len(),
    })
}

/// Tous les produits en rupture à `today`, le plus signalé d'abord.
pub fn shortages(events: &[Event], today: &str) -> Vec<Shortage> {
    let mut seen = std::collections::HashSet::new();
    let mut out: Vec<Shortage> = standing(events)
        .into_iter()
        .filter(|e| e.kind == Kind::Rupture)
        .filter(|e| seen.insert(key(&e.product)))
        .filter_map(|e| shortage(events, &e.product, today))
        .collect();
    out.sort_by(|a, b| {
        b.sources
            .cmp(&a.sources)
            .then(b.last.cmp(&a.last))
            .then(a.product.cmp(&b.product))
    });
    out
}

/// Les produits en rupture **à présent** qui ne l'étaient pas avant —
/// ce qu'une synchronisation vient d'apprendre. Dans l'ordre de
/// [`shortages`].
///
/// Une rupture déjà connue qui reçoit un signalement de plus n'est pas
/// une nouvelle : elle était au tableau de bord avant.
pub fn newly_short(before: &[Event], after: &[Event], today: &str) -> Vec<String> {
    let known: std::collections::HashSet<String> = shortages(before, today)
        .into_iter()
        .map(|s| key(&s.product))
        .collect();
    shortages(after, today)
        .into_iter()
        .filter(|s| !known.contains(&key(&s.product)))
        .map(|s| s.product)
        .collect()
}

/// Ce qui a été donné à la place d'un produit, une ligne par substitut.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tried {
    pub other: String,
    pub other_dci: String,
    pub times: usize,
    /// Combien d'officines distinctes l'ont fait.
    pub sources: usize,
    pub last: String,
    /// Par issue, dans l'ordre de [`Outcome::ALL`] — « non dit » compris.
    pub outcomes: [usize; 5],
}

impl Tried {
    pub fn count(&self, outcome: Outcome) -> usize {
        Outcome::ALL
            .iter()
            .position(|o| *o == outcome)
            .map_or(0, |i| self.outcomes[i])
    }
}

/// Ce que les pharmaciens ont mis à la place de `product`, le plus
/// souvent fait d'abord, puis le plus récent. **Un historique, pas une
/// équivalence** — voir l'en-tête.
pub fn tried(events: &[Event], product: &str) -> Vec<Tried> {
    let k = key(product);
    let mut out: Vec<Tried> = Vec::new();
    let mut sources: Vec<std::collections::HashSet<String>> = Vec::new();
    for e in standing(events)
        .into_iter()
        .filter(|e| e.kind == Kind::Substitution && key(&e.product) == k)
    {
        let other = key(&e.other);
        if other.is_empty() {
            continue;
        }
        let at = match out.iter().position(|t| key(&t.other) == other) {
            Some(i) => i,
            None => {
                out.push(Tried {
                    other: e.other.trim().to_owned(),
                    other_dci: e.other_dci.trim().to_owned(),
                    times: 0,
                    sources: 0,
                    last: String::new(),
                    outcomes: [0; 5],
                });
                sources.push(std::collections::HashSet::new());
                out.len() - 1
            }
        };
        let t = &mut out[at];
        t.times += 1;
        if e.day > t.last {
            t.last = e.day.clone();
        }
        if let Some(i) = Outcome::ALL.iter().position(|o| *o == e.outcome) {
            t.outcomes[i] += 1;
        }
        sources[at].insert(e.source.clone());
        t.sources = sources[at].len();
    }
    out.sort_by(|a, b| {
        b.times
            .cmp(&a.times)
            .then(b.last.cmp(&a.last))
            .then(a.other.cmp(&b.other))
    });
    out
}

/// Ce qu'un événement devient pour voyager vers les autres officines du
/// réseau : les champs du journal et le nom de l'officine qui l'a noté.
/// **Rien d'autre n'existe dans ce format** — pas de patient, pas de
/// dossier, pas de poste : c'est ce qui permet à ce flux-là de sortir.
pub fn encode(e: &Event, officine: &str) -> Vec<u8> {
    serde_json::json!({
        "v": 1,
        "uid": e.uid,
        "day": e.day,
        "kind": e.kind.key(),
        "product": e.product,
        "product_dci": e.product_dci,
        "other": e.other,
        "other_dci": e.other_dci,
        "outcome": e.outcome.key(),
        "note": e.note,
        "operator": e.operator,
        "officine": officine,
        "refers": e.refers,
    })
    .to_string()
    .into_bytes()
}

/// L'inverse, pour ce qu'une autre officine a envoyé : l'événement, avec
/// pour source le nom de l'officine qu'il porte. `None` pour ce qui ne se
/// lit pas — une version postérieure, un enregistrement abîmé : il reste
/// au journal, il n'entre pas dans la base.
pub fn decode(bytes: &[u8]) -> Option<Event> {
    let v: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    if v.get("v")?.as_u64()? != 1 {
        return None;
    }
    let text = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_owned();
    let uid = text("uid");
    let day = text("day");
    if uid.is_empty() || crate::date::parse_iso(&day).is_none() {
        return None;
    }
    let officine = text("officine");
    Some(Event {
        uid,
        day,
        kind: Kind::from_key(&text("kind"))?,
        product: text("product"),
        product_dci: text("product_dci"),
        other: text("other"),
        other_dci: text("other_dci"),
        outcome: Outcome::from_key(&text("outcome")),
        note: text("note"),
        operator: text("operator"),
        // Une officine qui ne s'est pas nommée reste une autre officine :
        // vide voudrait dire « celle-ci ».
        source: if officine.trim().is_empty() {
            crate::strings::tr("rupt_unnamed_officine").to_owned()
        } else {
            officine
        },
        refers: text("refers"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Une nouvelle, c'est une rupture qu'on ne savait pas** : ni une
    /// déjà connue qui reçoit un signalement de plus, ni une levée.
    #[test]
    fn only_a_shortage_not_known_before_is_news() {
        let ev = |uid: &str, product: &str, kind, day: &str| Event {
            uid: uid.to_owned(),
            day: day.to_owned(),
            kind,
            product: product.to_owned(),
            product_dci: String::new(),
            other: String::new(),
            other_dci: String::new(),
            outcome: Outcome::Accepted,
            note: String::new(),
            operator: String::new(),
            source: "Pharmacie du Port".to_owned(),
            refers: String::new(),
        };
        let before = vec![ev("a:1", "Diprosone", Kind::Rupture, "2026-09-20")];
        let mut after = before.clone();
        after.push(ev("a:2", "Diprosone", Kind::Rupture, "2026-09-23"));
        after.push(ev("a:3", "Locoid", Kind::Rupture, "2026-09-23"));
        assert_eq!(
            newly_short(&before, &after, "2026-09-24"),
            vec!["Locoid".to_owned()]
        );
        // Levée : rien de neuf à annoncer.
        after.push(ev("a:4", "Locoid", Kind::Levee, "2026-09-24"));
        assert!(newly_short(&before, &after, "2026-09-24").is_empty());
    }

    fn ev(uid: &str, day: &str, kind: Kind, product: &str, other: &str, source: &str) -> Event {
        Event {
            uid: uid.to_owned(),
            day: day.to_owned(),
            kind,
            product: product.to_owned(),
            product_dci: String::new(),
            other: other.to_owned(),
            other_dci: String::new(),
            outcome: Outcome::Unknown,
            note: String::new(),
            operator: "CL".to_owned(),
            source: source.to_owned(),
            refers: String::new(),
        }
    }

    /// **Ce qui a été tenté, et combien de fois cela a tenu** — compté
    /// par substitut, par officine, et par issue, le plus fréquent
    /// d'abord. La casse d'une officine à l'autre ne sépare rien.
    #[test]
    fn what_was_tried_is_counted_by_substitute_source_and_outcome() {
        let mut a = ev(
            "a:1",
            "2026-09-01",
            Kind::Substitution,
            "Diprosone",
            "Locoid",
            "",
        );
        a.outcome = Outcome::Accepted;
        let mut b = ev(
            "b:1",
            "2026-09-10",
            Kind::Substitution,
            "DIPROSONE",
            "locoid",
            "Pharmacie du Port",
        );
        b.outcome = Outcome::Failed;
        let c = ev(
            "a:2",
            "2026-09-12",
            Kind::Substitution,
            "Diprosone",
            "Nérisone",
            "",
        );
        let d = ev(
            "a:3",
            "2026-09-12",
            Kind::Substitution,
            "Dermoval",
            "Locoid",
            "",
        );
        let list = tried(&[a, b, c, d], "diprosone");
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].other, "Locoid");
        assert_eq!(list[0].times, 2);
        assert_eq!(list[0].sources, 2);
        assert_eq!(list[0].last, "2026-09-10");
        assert_eq!(list[0].count(Outcome::Accepted), 1);
        assert_eq!(list[0].count(Outcome::Failed), 1);
        assert_eq!(list[1].other, "Nérisone");
        assert_eq!(list[1].count(Outcome::Unknown), 1);
    }

    /// **Un événement ne se réécrit pas, il se retire** — et le retrait
    /// l'ôte de toutes les lectures sans l'effacer du journal.
    #[test]
    fn a_withdrawn_event_is_read_by_nobody() {
        let a = ev(
            "a:1",
            "2026-09-01",
            Kind::Substitution,
            "Diprosone",
            "Locoid",
            "",
        );
        let r = Event {
            refers: "a:1".to_owned(),
            ..ev("a:2", "2026-09-02", Kind::Retrait, "Diprosone", "", "")
        };
        assert!(tried(&[a.clone(), r.clone()], "Diprosone").is_empty());
        let rupture = ev("a:3", "2026-09-01", Kind::Rupture, "Diprosone", "", "");
        let undo = Event {
            refers: "a:3".to_owned(),
            ..ev("a:4", "2026-09-02", Kind::Retrait, "Diprosone", "", "")
        };
        assert!(shortage(&[rupture, undo], "Diprosone", "2026-09-05").is_none());
        assert_eq!(standing(&[a, r]).len(), 0, "ni l'événement, ni son retrait");
    }

    /// **Une rupture vieillit, une levée la clôt, un nouveau signalement
    /// en ouvre une autre.**
    #[test]
    fn a_shortage_is_open_until_lifted_or_forgotten() {
        let events = [
            ev("a:1", "2026-06-01", Kind::Rupture, "Diprosone", "", ""),
            ev(
                "b:1",
                "2026-06-03",
                Kind::Rupture,
                "Diprosone",
                "",
                "Pharmacie du Port",
            ),
        ];
        let s = shortage(&events, "diprosone", "2026-06-10").expect("en cours");
        assert_eq!(s.since, "2026-06-01");
        assert_eq!(s.sources, 2);
        // Quatre mois sans nouvelle : plus « en cours ».
        assert!(shortage(&events, "Diprosone", "2026-10-15").is_none());
        // Levée : close.
        let mut lifted = events.to_vec();
        lifted.push(ev("a:2", "2026-06-20", Kind::Levee, "Diprosone", "", ""));
        assert!(shortage(&lifted, "Diprosone", "2026-06-21").is_none());
        // Signalée de nouveau : un nouvel épisode, qui part de là.
        lifted.push(ev("a:3", "2026-08-01", Kind::Rupture, "Diprosone", "", ""));
        let s = shortage(&lifted, "Diprosone", "2026-08-05").expect("nouvel épisode");
        assert_eq!(s.since, "2026-08-01");
        assert_eq!(s.sources, 1);
        assert_eq!(shortages(&lifted, "2026-08-05").len(), 1);
    }

    /// **Ce qui voyage revient tel quel**, avec pour source l'officine qui
    /// l'a noté ; ce qui ne se lit pas n'entre pas.
    #[test]
    fn an_event_crosses_to_another_officine_and_comes_back_whole() {
        let mut e = ev(
            "ab12:7",
            "2026-09-20",
            Kind::Substitution,
            "Diprosone",
            "Locoid",
            "",
        );
        e.outcome = Outcome::Failed;
        e.note = "mal toléré « visage »".to_owned();
        let back = decode(&encode(&e, "Pharmacie du Centre")).expect("lu");
        assert_eq!(back.source, "Pharmacie du Centre");
        assert_eq!(
            Event {
                source: String::new(),
                ..back
            },
            e
        );
        assert!(decode(b"pas du json").is_none());
        assert!(decode(br#"{"v":2,"uid":"x","day":"2026-09-20","kind":"RUPTURE"}"#).is_none());
        assert!(decode(br#"{"v":1,"uid":"x","day":"hier","kind":"RUPTURE"}"#).is_none());
    }

    #[test]
    fn every_key_reads_back() {
        for k in Kind::ALL {
            assert_eq!(Kind::from_key(k.key()), Some(k));
        }
        for o in Outcome::ALL {
            assert_eq!(Outcome::from_key(o.key()), o);
        }
        assert_eq!(Outcome::from_key("inconnu"), Outcome::Unknown);
    }
}
