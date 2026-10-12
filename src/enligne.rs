//! **« En ligne » : ce que les autres officines ont, à chercher et à
//! prendre.**
//!
//! Le journal des versions (`versions`) porte déjà tout ce que les
//! officines du réseau ont écrit — fiches, préparations, protocoles,
//! lignes de TROD, vaccins. Mais il ne se lisait que fiche par fiche,
//! dans l'historique d'une entrée qu'on avait déjà : rien ne permettait
//! de demander « qui a une préparation de méthadone ? », ni de reprendre
//! une entrée qu'on avait supprimée ici et qu'un confrère tient à jour.
//!
//! Ce module range le journal en **offres** : une par entrée qu'au moins
//! une autre officine a écrite, avec son nom, qui la tient, quand, et ce
//! qu'elle dit champ par champ. L'écran les cherche et les compare à la
//! base locale ; la base, elle, prend une offre d'un geste
//! (`Db::adopt_offer`).
//!
//! Pur, testé, sans base ni horloge.

use crate::versions::{key, Edit, Kind};

/// Une entrée telle que le réseau la tient.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Offer {
    pub kind: Kind,
    /// Sa clé, le nom replié sous lequel elle voyage.
    pub card: String,
    /// Son nom tel qu'il s'écrit, le dernier connu.
    pub name: String,
    /// Les officines qui l'ont écrite, dans l'ordre où elles sont
    /// apparues au journal. Jamais celle-ci.
    pub from: Vec<String>,
    /// Le jour de sa dernière version, d'où qu'elle vienne.
    pub day: String,
    /// Chaque champ, à sa dernière version — ce que l'entrée est pour le
    /// journal, et ce qu'une adoption écrit.
    pub fields: Vec<(String, String)>,
}

/// Les offres d'une sorte d'entrée, **par nom**.
///
/// Une entrée n'est une offre que si une autre officine y a écrit : celles
/// qu'on est seul à avoir versionnées sont déjà les nôtres. Ses champs
/// sont pris à leur dernière version, d'où qu'elle vienne — c'est la même
/// règle que l'historique d'une entrée, et ce qu'on verrait en
/// l'ouvrant.
pub fn offers(edits: &[Edit], kind: Kind) -> Vec<Offer> {
    let mut out: Vec<Offer> = Vec::new();
    for e in edits.iter().filter(|e| e.kind == kind) {
        if !kind.fields().contains(&e.field.as_str()) || e.card.is_empty() {
            continue;
        }
        let i = match out.iter().position(|o| o.card == e.card) {
            Some(i) => i,
            None => {
                out.push(Offer {
                    kind,
                    card: e.card.clone(),
                    name: String::new(),
                    from: Vec::new(),
                    day: String::new(),
                    fields: Vec::new(),
                });
                out.len() - 1
            }
        };
        let o = &mut out[i];
        if !e.card_name.trim().is_empty() {
            o.name = e.card_name.trim().to_owned();
        }
        let source = e.source.trim();
        if !source.is_empty() && !o.from.iter().any(|f| f == source) {
            o.from.push(source.to_owned());
        }
        if e.day > o.day {
            o.day = e.day.clone();
        }
        match o.fields.iter_mut().find(|(f, _)| *f == e.field) {
            Some((_, v)) => v.clone_from(&e.value),
            None => o.fields.push((e.field.clone(), e.value.clone())),
        }
    }
    // **Une fiche renommée se suit jusqu'à son nouveau nom.** Ses
    // versions d'avant portent l'ancienne clé ; laissées là, elles
    // faisaient une offre « absente ici » sous un nom que plus personne
    // ne porte, et la prendre doublait la fiche. Fondue dans l'offre du
    // nouveau nom, celle-ci garde ses champs les plus récents.
    if kind == Kind::Fiche {
        let mut i = 0;
        while i < out.len() {
            let target = out[i]
                .field("name")
                .map(str::trim)
                .filter(|n| !n.is_empty() && key(n) != out[i].card)
                .map(str::to_owned);
            let Some(name) = target else {
                i += 1;
                continue;
            };
            let old = out.remove(i);
            let k = key(&name);
            match out.iter().position(|o| o.card == k) {
                Some(j) => {
                    let o = &mut out[j];
                    for f in old.from {
                        if !o.from.contains(&f) {
                            o.from.push(f);
                        }
                    }
                    if old.day > o.day {
                        o.day = old.day;
                    }
                    for (f, v) in old.fields {
                        if f != "name" && o.field(&f).is_none() {
                            o.fields.push((f, v));
                        }
                    }
                    o.name.clone_from(&name);
                    // A merged offer may now be renamed again: read it anew.
                    i = 0;
                }
                None => {
                    out.push(Offer {
                        card: k,
                        name,
                        fields: old
                            .fields
                            .into_iter()
                            .filter(|(f, _)| f != "name")
                            .collect(),
                        ..old
                    });
                    i = 0;
                }
            }
        }
    }
    out.retain(|o| !o.from.is_empty() && !o.name.is_empty());
    // Les champs dans l'ordre de la sorte, pas dans celui du journal : une
    // adoption les écrit dans cet ordre, et l'écran les lit ainsi.
    for o in &mut out {
        let order = |f: &str| kind.fields().iter().position(|k| *k == f);
        o.fields.sort_by_key(|(f, _)| order(f));
    }
    out.sort_by_key(|o| key(&o.name));
    out
}

impl Offer {
    /// La valeur d'un champ à sa dernière version.
    pub fn field(&self, name: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|(f, _)| f == name)
            .map(|(_, v)| v.as_str())
    }

    /// L'offre répond-elle à ce qui est tapé ? Le nom, puis ce qui la
    /// décrit : une préparation se cherche à son indication, une fiche à
    /// sa DCI ou sa classe, un protocole à son sujet.
    pub fn matches(&self, query: &str) -> bool {
        let q = query.trim();
        if q.is_empty() {
            return true;
        }
        if crate::fuzzy::contains_loose(&self.name, q) {
            return true;
        }
        let described = match self.kind {
            Kind::Fiche => &["dci", "class", "indications"][..],
            Kind::Codex => &["indication", "form", "tags"][..],
            Kind::Protocole => &["subject"][..],
            Kind::Trod => &["situation"][..],
            Kind::Vaccin => &["label", "code"][..],
            // Le nom d'aujourd'hui aussi : l'offre s'affiche sous lui.
            Kind::Dispositif => &["nom", "family", "indication", "etiquettes"][..],
            Kind::Liste => &["titre", "subject"][..],
            Kind::Cascade => &["texte"][..],
            Kind::Texte => &["value"][..],
        };
        described
            .iter()
            .filter_map(|f| self.field(f))
            .any(|v| crate::fuzzy::contains_loose(v, q))
    }
}

/// Ce qu'une offre est **ici**.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Here {
    /// L'officine ne l'a pas : elle peut la prendre.
    Absent,
    /// Elle l'a, et elle dit ce que dit le journal.
    Same,
    /// Elle l'a, et un champ au moins dit autre chose — une version
    /// reçue qui attend l'arbitrage, ou une modification d'ici.
    Differs,
}

/// Comparer une offre à l'entrée locale de même clé, rendue champ par
/// champ (`None` : l'officine ne l'a pas). Un champ que la base locale ne
/// connaît pas ne compte pas : il ne dit ni oui ni non.
pub fn here(offer: &Offer, local: Option<&[(&str, String)]>) -> Here {
    let Some(local) = local else {
        return Here::Absent;
    };
    let differs = offer.fields.iter().any(|(f, v)| {
        local
            .iter()
            .find(|(l, _)| l == f)
            .is_some_and(|(_, mine)| mine != v)
    });
    if differs {
        Here::Differs
    } else {
        Here::Same
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edit(kind: Kind, name: &str, field: &str, value: &str, source: &str, day: &str) -> Edit {
        Edit {
            kind,
            uid: format!("{source}:{name}:{field}:{value}"),
            day: day.to_owned(),
            card: key(name),
            card_name: name.to_owned(),
            field: field.to_owned(),
            value: value.to_owned(),
            previous: String::new(),
            corrects: String::new(),
            revert: false,
            operator: String::new(),
            source: source.to_owned(),
        }
    }

    /// **Une offre est ce qu'une autre officine a écrit**, à sa dernière
    /// version, champ par champ ; ce qu'on est seul à avoir écrit n'en est
    /// pas une, et une autre sorte d'entrée n'y entre pas.
    #[test]
    fn the_journal_reads_as_offers_from_elsewhere() {
        let edits = [
            edit(
                Kind::Codex,
                "Méthadone sirop",
                "form",
                "sirop",
                "Pharmacie A",
                "2026-09-01",
            ),
            edit(
                Kind::Codex,
                "Méthadone sirop",
                "indication",
                "TSO",
                "Pharmacie A",
                "2026-09-01",
            ),
            edit(
                Kind::Codex,
                "Méthadone sirop",
                "form",
                "solution",
                "Pharmacie B",
                "2026-09-03",
            ),
            edit(Kind::Codex, "Lugol", "form", "solution", "", "2026-09-02"),
            edit(
                Kind::Fiche,
                "Méthadone sirop",
                "dci",
                "méthadone",
                "Pharmacie C",
                "2026-09-04",
            ),
        ];
        let o = offers(&edits, Kind::Codex);
        assert_eq!(o.len(), 1, "le Lugol n'est écrit qu'ici : {o:?}");
        assert_eq!(o[0].name, "Méthadone sirop");
        assert_eq!(o[0].from, vec!["Pharmacie A", "Pharmacie B"]);
        assert_eq!(o[0].day, "2026-09-03");
        assert_eq!(o[0].field("form"), Some("solution"), "la dernière version");
        assert_eq!(o[0].field("indication"), Some("TSO"));
        // Dans l'ordre des champs de la sorte.
        assert_eq!(o[0].fields[0].0, "form");
        assert_eq!(o[0].fields[1].0, "indication");
    }

    /// Un champ que la sorte ne partage pas ne passe pas, même s'il est au
    /// journal : l'adoption écrirait une colonne nommée par un confrère.
    #[test]
    fn a_field_the_kind_does_not_share_is_left_out() {
        let edits = [
            edit(Kind::Codex, "Lugol", "form", "solution", "A", "2026-09-01"),
            edit(Kind::Codex, "Lugol", "notes", "à nous", "A", "2026-09-01"),
        ];
        let o = offers(&edits, Kind::Codex);
        assert_eq!(o[0].fields.len(), 1);
    }

    /// Une fiche renommée ne laisse pas d'offre sous son ancien nom : elle
    /// se lit sous le nouveau, avec ses champs d'avant.
    #[test]
    fn a_renamed_card_is_offered_under_its_new_name() {
        let edits = [
            edit(
                Kind::Fiche,
                "Molécule A",
                "dci",
                "a-ine",
                "Port",
                "2026-09-01",
            ),
            edit(
                Kind::Fiche,
                "Molécule A",
                "name",
                "Molécule B",
                "Port",
                "2026-09-02",
            ),
            edit(
                Kind::Fiche,
                "Molécule B",
                "class",
                "classe",
                "Lac",
                "2026-09-03",
            ),
        ];
        let o = offers(&edits, Kind::Fiche);
        assert_eq!(o.len(), 1, "{o:?}");
        assert_eq!(o[0].name, "Molécule B");
        assert_eq!(o[0].card, key("Molécule B"));
        assert_eq!(o[0].field("dci"), Some("a-ine"));
        assert_eq!(o[0].field("class"), Some("classe"));
        assert_eq!(o[0].from, vec!["Lac", "Port"]);
        assert_eq!(o[0].day, "2026-09-03");
        // Renommée sans version après : l'offre porte le nouveau nom.
        let alone = offers(&edits[..2], Kind::Fiche);
        assert_eq!(alone.len(), 1);
        assert_eq!(alone[0].name, "Molécule B");
    }

    #[test]
    fn an_offer_is_found_by_its_name_or_what_describes_it() {
        let edits = [edit(
            Kind::Codex,
            "Sirop BPM",
            "indication",
            "Toux sèche",
            "A",
            "2026-09-01",
        )];
        let o = &offers(&edits, Kind::Codex)[0];
        assert!(o.matches(""));
        assert!(o.matches("sirop"));
        assert!(o.matches("toux"));
        assert!(!o.matches("méthadone"));
        // Un dispositif se cherche à ses étiquettes, sous leur nom de
        // voyage.
        let dispo = [edit(
            Kind::Dispositif,
            "Hydrocolloïde",
            "etiquettes",
            "escarre",
            "A",
            "2026-09-01",
        )];
        assert!(offers(&dispo, Kind::Dispositif)[0].matches("escarre"));
    }

    /// Absente, pareille, ou différente : comparée champ par champ, et un
    /// champ que l'entrée locale ne porte pas ne fait pas une différence.
    #[test]
    fn an_offer_is_compared_with_the_local_entry() {
        let edits = [
            edit(Kind::Codex, "Lugol", "form", "solution", "A", "2026-09-01"),
            edit(Kind::Codex, "Lugol", "caution", "Iode", "A", "2026-09-01"),
        ];
        let o = &offers(&edits, Kind::Codex)[0];
        assert_eq!(here(o, None), Here::Absent);
        let same = [
            ("form", "solution".to_owned()),
            ("caution", "Iode".to_owned()),
        ];
        assert_eq!(here(o, Some(&same)), Here::Same);
        let partial = [("form", "solution".to_owned())];
        assert_eq!(here(o, Some(&partial)), Here::Same);
        let other = [("form", "gélule".to_owned())];
        assert_eq!(here(o, Some(&other)), Here::Differs);
    }
}
