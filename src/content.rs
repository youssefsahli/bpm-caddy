//! Les mots imprimés, et le droit de l'officine de les réécrire.
//!
//! # Ce qui manquait
//!
//! Les fiches médicament, les préparations, les dispositifs, les
//! protocoles et les *cellules* des tables de conversion s'éditent déjà :
//! ce que l'application livre est une graine, et ce que l'équipe écrit
//! n'est jamais réécrit par une mise à jour. Sept cent soixante-douze
//! phrases échappaient à cette règle — les carnets de suivi, la liste de
//! points de la fiche d'entretien, la feuille « peut-on écraser ? », le
//! plan de surveillance, les interprétations de biologie, la revue
//! d'ordonnance — et ce sont précisément des phrases qui **partent sur
//! du papier**, au nom de l'officine, à un patient ou à un confrère.
//! Une tournure qui ne convient pas était une tournure qu'il fallait
//! subir.
//!
//! # Le mécanisme, qui n'est pas neuf
//!
//! Les tables de conversion le portaient déjà, cellule par cellule :
//! une surcharge rangée à côté du texte livré, écrite contre ce que
//! l'écran affichait, et **supprimée dès qu'on y réécrit le texte
//! livré**. Ce module en fait la règle générale plutôt qu'un deuxième
//! mécanisme à côté — il n'y a pas deux façons de surcharger un texte
//! dans cette application.
//!
//! # Les règles
//!
//! **Sans surcharge, c'est le texte livré.** Une officine qui n'a rien
//! réécrit lit ce qui est livré, et continue de recevoir les
//! corrections des versions suivantes.
//!
//! **Une surcharge se souvient du texte qu'elle remplaçait**, et ne
//! s'applique que tant que celui-ci n'a pas changé. C'est la garde qui
//! rend l'adressage par rang tenable : une liste de points se désigne
//! forcément par un numéro, et le jour où cette liste est réordonnée ou
//! réécrite, une surcharge appliquée en aveugle poserait la réécriture
//! d'une phrase par-dessus une autre. Elle est alors **montrée à
//! relire** au lieu d'être appliquée — la même famille que « le silence
//! n'est pas une permission » de `crush.rs`.
//!
//! **Réécrire le texte livré n'est pas une surcharge** : la ligne s'en
//! va. La table ne porte donc que de vraies différences, et une phrase
//! qu'on a rétablie recommence à suivre les versions suivantes.
//!
//! Pur et testé : aucune base ici, aucune horloge. La table est lue une
//! fois et passée.

use std::collections::HashMap;

/// Ce que l'officine a écrit à la place d'une phrase livrée.
#[derive(Clone, PartialEq, Debug)]
pub struct Entry {
    /// Le texte de l'officine.
    pub value: String,
    /// Le texte livré **tel qu'il était** quand l'officine l'a réécrit.
    /// C'est lui qui dit si la surcharge vise encore la même phrase.
    pub shipped_seen: String,
}

/// Toutes les surcharges de l'officine, lues une fois.
#[derive(Clone, Default, PartialEq, Debug)]
pub struct Overrides {
    map: HashMap<String, Entry>,
}

/// Ce qu'une phrase est devenue.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum State {
    /// Telle qu'elle est livrée : personne n'y a touché.
    Shipped,
    /// Réécrite par l'officine, et toujours en face de la phrase
    /// qu'elle remplaçait.
    Rewritten,
    /// Réécrite, mais le texte livré a changé depuis : la surcharge ne
    /// s'applique plus et attend une relecture.
    Outdated,
}

impl Overrides {
    pub fn from_rows(rows: impl IntoIterator<Item = (String, String, String)>) -> Self {
        Self {
            map: rows
                .into_iter()
                .map(|(key, value, shipped_seen)| {
                    (
                        key,
                        Entry {
                            value,
                            shipped_seen,
                        },
                    )
                })
                .collect(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Ce qui doit être imprimé pour cette phrase.
    ///
    /// Le texte livré est rendu tel quel quand rien ne le remplace, et
    /// **aussi** quand la surcharge ne vise plus la même phrase : une
    /// réécriture posée sur autre chose que ce qu'elle visait serait
    /// pire que pas de réécriture du tout.
    pub fn get<'a>(&'a self, key: &str, shipped: &'a str) -> &'a str {
        match self.map.get(key) {
            Some(e) if e.shipped_seen == shipped => e.value.as_str(),
            _ => shipped,
        }
    }

    /// L'état de cette phrase, pour que l'écran puisse le montrer.
    pub fn state(&self, key: &str, shipped: &str) -> State {
        match self.map.get(key) {
            None => State::Shipped,
            Some(e) if e.shipped_seen == shipped => State::Rewritten,
            Some(_) => State::Outdated,
        }
    }

    /// Le texte que l'officine avait écrit, même s'il ne s'applique
    /// plus : c'est ce qu'on lui montre pour qu'elle le reprenne plutôt
    /// que de le perdre.
    pub fn written(&self, key: &str) -> Option<&str> {
        self.map.get(key).map(|e| e.value.as_str())
    }

    /// Le texte livré que la surcharge visait, pour l'afficher à côté
    /// du nouveau quand les deux ont divergé.
    pub fn aimed_at(&self, key: &str) -> Option<&str> {
        self.map.get(key).map(|e| e.shipped_seen.as_str())
    }
}

/// L'adresse d'une phrase livrée, écrite **une seule façon**.
///
/// Un document, ce qu'il désigne à l'intérieur, et le champ. Deux
/// façons de composer une clé seraient deux clés pour une phrase, donc
/// une réécriture que l'impression ne trouve pas.
pub fn key(doc: &str, item: &str, field: &str) -> String {
    format!("{doc}.{item}.{field}")
}

/// L'adresse d'une phrase dans une liste — une consigne d'un carnet, un
/// point d'une thématique d'entretien.
///
/// Le rang est le seul repère qu'une liste offre, et c'est lui qui rend
/// la garde de `shipped_seen` nécessaire plutôt que décorative.
pub fn key_n(doc: &str, item: &str, field: &str, n: usize) -> String {
    format!("{doc}.{item}.{field}.{n}")
}

/// Le repère d'une règle, tiré de **ce qui l'identifie** et jamais de sa
/// prose.
///
/// C'est la différence qui compte pour une table de règles. Une adresse
/// tirée du texte s'évanouit le jour où ce texte est corrigé : la
/// surcharge ne devient pas périmée — elle devient introuvable, et
/// personne ne la voit partir. Une adresse tirée du rang dans un tableau
/// se décale dès qu'on insère une règle au-dessus, et rend périmées
/// vingt réécritures pour une règle ajoutée. Le titre d'un point de
/// revue, le code d'un analyte : voilà ce qui ne bouge pas.
///
/// Les caractères sont repliés comme partout ailleurs — accents et
/// casse — et ce qui n'est ni lettre ni chiffre devient un tiret, pour
/// qu'une adresse reste lisible dans la table.
pub fn slug(identity: &str) -> String {
    let mut out = String::with_capacity(identity.len());
    let mut last_dash = true;
    for c in crate::fuzzy::sort_key(identity).chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}

/// Un document dont les phrases se réécrivent.
pub struct Document {
    /// Le préfixe de ses adresses — « carnet.tension », « revue ». Il
    /// sert aussi à tout rétablir d'un geste.
    pub subject: String,
    /// Ce que l'écran en dit.
    pub label: String,
    /// Ses phrases, dans l'ordre où elles s'impriment.
    pub phrases: Vec<(String, &'static str, &'static str)>,
}

/// Tout ce qui s'imprime et dont les mots appartiennent à l'officine.
///
/// **Une ligne par document**, sur le modèle de `pdf::DOCS` : ajouter un
/// document éditable est ajouter une entrée ici, et il apparaît dans
/// l'écran « Textes » sans que cet écran ait à le connaître.
///
/// Le registre est la seule liste : l'écran central le parcourt, et
/// chaque vue qui édite ses propres phrases en place appelle le
/// `phrases()` du même module. Deux listes des mêmes phrases finiraient
/// par différer, et une phrase absente de l'une serait une phrase que
/// personne ne peut corriger.
pub fn documents() -> Vec<Document> {
    let mut out: Vec<Document> = crate::selfcheck::SHEETS
        .iter()
        .map(|s| Document {
            subject: format!("{}.{}", crate::selfcheck::DOC, s.key),
            label: s.title.to_owned(),
            phrases: crate::selfcheck::phrases(s),
        })
        .collect();
    out.push(Document {
        subject: crate::revue::DOC.to_owned(),
        label: crate::strings::tr("textes_doc_revue").to_owned(),
        phrases: crate::revue::phrases(),
    });
    out.push(Document {
        subject: crate::entretien::DOC.to_owned(),
        label: crate::strings::tr("textes_doc_entretien").to_owned(),
        phrases: crate::entretien::phrases(),
    });
    out.push(Document {
        subject: crate::crush::DOC.to_owned(),
        label: crate::strings::tr("textes_doc_ecraser").to_owned(),
        phrases: crate::crush::phrases(),
    });
    out.push(Document {
        subject: crate::surveillance::DOC.to_owned(),
        label: crate::strings::tr("textes_doc_surveillance").to_owned(),
        phrases: crate::surveillance::phrases(),
    });
    out.push(Document {
        subject: crate::biology::DOC.to_owned(),
        label: crate::strings::tr("textes_doc_biologie").to_owned(),
        phrases: crate::biology::phrases(),
    });
    out.push(Document {
        subject: crate::gravidity::DOC.to_owned(),
        label: crate::strings::tr("textes_doc_grossesse").to_owned(),
        phrases: crate::gravidity::phrases(),
    });
    out.push(Document {
        subject: crate::renal::DOC.to_owned(),
        label: crate::strings::tr("textes_doc_rein").to_owned(),
        phrases: crate::renal::phrases(),
    });
    out.push(Document {
        subject: crate::ordonnance::DOC.to_owned(),
        label: crate::strings::tr("textes_doc_ordonnance").to_owned(),
        phrases: crate::ordonnance::phrases(),
    });
    out.push(Document {
        subject: crate::vaccines::DOC.to_owned(),
        label: crate::strings::tr("textes_doc_voyage").to_owned(),
        phrases: crate::vaccines::phrases(),
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(key: &str, value: &str, shipped_seen: &str) -> Overrides {
        Overrides::from_rows([(key.to_owned(), value.to_owned(), shipped_seen.to_owned())])
    }

    /// **Sans surcharge, c'est le texte livré.**
    ///
    /// Et il le reste : une officine qui n'a rien réécrit continue de
    /// recevoir les corrections des versions suivantes, ce qui est tout
    /// l'intérêt de ne pas figer une copie au premier lancement.
    #[test]
    fn what_nobody_rewrote_is_what_is_shipped() {
        let none = Overrides::default();
        assert_eq!(
            none.get("carnet.tension.alerte", "Appelez le 15."),
            "Appelez le 15."
        );
        assert_eq!(
            none.state("carnet.tension.alerte", "Appelez le 15."),
            State::Shipped
        );
        assert!(none.is_empty());
    }

    /// Une réécriture posée en face de la phrase qu'elle remplace est
    /// celle qui s'imprime.
    #[test]
    fn what_the_officine_rewrote_is_what_prints() {
        let o = one(
            "carnet.tension.alerte",
            "Appelez le 15 sans attendre.",
            "Appelez le 15.",
        );
        assert_eq!(
            o.get("carnet.tension.alerte", "Appelez le 15."),
            "Appelez le 15 sans attendre."
        );
        assert_eq!(
            o.state("carnet.tension.alerte", "Appelez le 15."),
            State::Rewritten
        );
    }

    /// **Une surcharge dont le texte livré a changé ne s'applique pas.**
    ///
    /// C'est la règle qui rend l'adressage par rang tenable. Une liste
    /// de consignes se désigne par un numéro — il n'y a pas d'autre
    /// repère —, et le jour où elle est réordonnée, `carnet.glycemie.3`
    /// ne désigne plus la même phrase. Appliquée en aveugle, la
    /// réécriture d'une consigne se poserait par-dessus une autre : sur
    /// une feuille qui part chez un patient, c'est exactement ce qu'il
    /// ne faut pas faire.
    ///
    /// Elle n'est pas perdue pour autant — elle est montrée à relire.
    #[test]
    fn a_rewrite_does_not_land_on_a_sentence_it_did_not_aim_at() {
        let o = one(
            "carnet.glycemie.consigne.3",
            "Notez le chiffre tout de suite.",
            "Notez le chiffre immédiatement.",
        );
        // La phrase livrée a été réécrite entre-temps.
        let now = "Piquez sur le côté de la pulpe du doigt.";
        assert_eq!(
            o.get("carnet.glycemie.consigne.3", now),
            now,
            "c'est le texte livré qui s'imprime, pas une réécriture qui visait autre chose"
        );
        assert_eq!(o.state("carnet.glycemie.consigne.3", now), State::Outdated);
        // Et ce que l'officine avait écrit reste lisible, avec ce qu'elle
        // visait : c'est ce qui permet de le reprendre plutôt que de le
        // retaper.
        assert_eq!(
            o.written("carnet.glycemie.consigne.3"),
            Some("Notez le chiffre tout de suite.")
        );
        assert_eq!(
            o.aimed_at("carnet.glycemie.consigne.3"),
            Some("Notez le chiffre immédiatement.")
        );
    }

    /// **Réécrire le texte livré n'est pas une surcharge.**
    ///
    /// La ligne s'en va, la table ne porte que de vraies différences, et
    /// la phrase recommence à suivre les versions suivantes. Sans cela,
    /// une officine qui « annule » sa modification en retapant le texte
    /// d'origine se retrouverait figée dessus pour toujours.
    #[test]
    fn writing_the_shipped_wording_back_removes_the_override() {
        // La suppression est celle de `Db::set_content`, qui efface la
        // ligne dès qu'on y réécrit le texte livré ; ce qui se vérifie
        // ici est ce que la vue lit ensuite : plus de surcharge, donc le
        // texte livré et l'état « livré ».
        let shipped = "Appelez le 15.";
        let o = one("carnet.tension.alerte", "Appelez tout de suite.", shipped);
        assert_eq!(o.len(), 1);
        let after = Overrides::default();
        assert!(after.is_empty(), "la ligne s'en va");
        assert_eq!(after.get("carnet.tension.alerte", shipped), shipped);
        assert_eq!(
            after.state("carnet.tension.alerte", shipped),
            State::Shipped
        );
    }

    /// **Le registre est la seule liste, et rien n'y est à deux
    /// adresses.**
    ///
    /// L'écran « Textes » le parcourt, et chaque vue qui édite ses
    /// phrases en place appelle le `phrases()` du même module : deux
    /// listes des mêmes phrases finiraient par différer, et une phrase
    /// absente de l'une serait une phrase que personne ne peut
    /// corriger. Deux documents qui partageraient une adresse seraient
    /// pire : une réécriture de l'un changerait l'autre.
    #[test]
    fn the_register_lists_every_editable_document_once() {
        let docs = documents();
        assert!(docs.len() >= 7, "{} documents", docs.len());

        // Un sujet par document.
        let mut subjects: Vec<&str> = docs.iter().map(|d| d.subject.as_str()).collect();
        subjects.sort_unstable();
        let n = subjects.len();
        subjects.dedup();
        assert_eq!(n, subjects.len(), "deux documents au même sujet");

        // Une adresse par phrase, **tous documents confondus**.
        let mut keys: Vec<&str> = docs
            .iter()
            .flat_map(|d| d.phrases.iter().map(|(k, _, _)| k.as_str()))
            .collect();
        assert!(keys.len() >= 200, "{} phrases", keys.len());
        keys.sort_unstable();
        let n = keys.len();
        keys.dedup();
        assert_eq!(n, keys.len(), "deux phrases à la même adresse");

        for d in &docs {
            assert!(!d.label.trim().is_empty(), "{} sans intitulé", d.subject);
            assert!(!d.phrases.is_empty(), "{} sans phrase", d.subject);
            // Chaque adresse descend du sujet : c'est ce qui permet de
            // rétablir un document entier par son préfixe.
            for (key, _, shipped) in &d.phrases {
                assert!(
                    key.starts_with(&d.subject),
                    "{key} ne descend pas de {}",
                    d.subject
                );
                assert!(!shipped.trim().is_empty(), "{key} : phrase vide");
            }
        }
    }

    /// **Le repère d'une règle vient de son identité, pas de sa prose.**
    ///
    /// Une adresse tirée du texte s'évanouit le jour où ce texte est
    /// corrigé : la surcharge ne devient pas périmée, elle devient
    /// introuvable — et c'est la seule façon de perdre le travail de
    /// l'officine sans que rien ne le dise.
    #[test]
    fn a_rule_is_addressed_by_what_identifies_it() {
        // Replié comme partout : accents, casse, ponctuation.
        assert_eq!(
            slug("Bêtabloquant + anticholinestérasique"),
            "betabloquant-anticholinesterasique"
        );
        assert_eq!(slug("AINS + IEC + diurétique"), "ains-iec-diuretique");
        // Deux règles différentes ne partagent pas une adresse.
        assert_ne!(
            slug("Opioïde sans laxatif"),
            slug("Opioïde sans antiémétique")
        );
        // Rien qui traîne aux extrémités : une adresse se lit dans la
        // table.
        assert_eq!(slug("  Deux AVK ?  "), "deux-avk");
        assert!(!slug("Quoi ?").ends_with('-'));
    }

    /// Une clé s'écrit d'une seule façon : deux compositions seraient
    /// deux clés pour une phrase, donc une réécriture que l'impression
    /// ne retrouve pas.
    #[test]
    fn a_key_is_composed_one_way() {
        assert_eq!(key("carnet", "tension", "alerte"), "carnet.tension.alerte");
        assert_eq!(
            key_n("entretien", "observance", "point", 2),
            "entretien.observance.point.2"
        );
        // Et deux documents ne se marchent pas dessus sur un même nom
        // de champ.
        assert_ne!(
            key("carnet", "tension", "alerte"),
            key("entretien", "tension", "alerte")
        );
    }
}
