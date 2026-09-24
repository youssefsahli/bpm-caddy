//! Les favoris : ce qu'une personne de l'équipe épingle pour le
//! retrouver d'un geste — une fiche médicament, un dossier patient, un
//! outil ou une vue, un contact de la messagerie.
//!
//! **À la personne, pas au poste.** Un favori est rangé sous les
//! initiales de l'opérateur et voyage entre les postes de l'officine :
//! Claire retrouve ses fiches au comptoir 2 comme au comptoir 1. Sans
//! opérateur renseigné, les favoris sont ceux du poste (initiales
//! vides), partagés par qui s'y assoit.
//!
//! Pur et testé : ce module dit ce qu'est un favori et comment on le
//! nomme ; la base le range (`Db::favorites`, `Db::set_favorite`).

/// La sorte d'un favori.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Kind {
    // L'ordre des variantes est celui de l'affichage (`ordered`).
    Patient,
    Drug,
    /// Un outil du registre `Tool`, par sa clé.
    Tool,
    /// Une vue (onglet de travail), par sa clé.
    View,
    /// Un contact de la messagerie : un collègue (initiales) ou une
    /// officine (empreinte).
    Contact,
}

impl Kind {
    pub const ALL: [Kind; 5] = [
        Kind::Patient,
        Kind::Drug,
        Kind::Tool,
        Kind::View,
        Kind::Contact,
    ];

    /// Le mot rangé en base.
    pub fn key(self) -> &'static str {
        match self {
            Kind::Drug => "drug",
            Kind::Patient => "patient",
            Kind::Tool => "tool",
            Kind::View => "view",
            Kind::Contact => "contact",
        }
    }

    pub fn from_key(key: &str) -> Option<Kind> {
        Kind::ALL.into_iter().find(|k| k.key() == key)
    }
}

/// Un favori tel que la base le rend.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Favorite {
    pub id: i64,
    pub kind: Kind,
    /// Ce qui est épinglé : un identifiant de fiche ou de dossier, une
    /// clé d'outil ou de vue, des initiales ou une empreinte.
    pub target: String,
    /// Le nom au moment de l'épinglage — ce qui s'affiche si la cible
    /// ne se retrouve plus (dossier supprimé, outil renommé).
    pub label: String,
}

/// Les favoris dans l'ordre où on les montre : par sorte (dossiers,
/// fiches, outils, vues, contacts), puis par nom sans accents ni casse.
pub fn ordered(mut list: Vec<Favorite>) -> Vec<Favorite> {
    list.sort_by(|a, b| {
        a.kind
            .cmp(&b.kind)
            .then_with(|| crate::fuzzy::sort_key(&a.label).cmp(&crate::fuzzy::sort_key(&b.label)))
    });
    list
}

/// Est-ce que `target` de cette sorte est en favori ?
pub fn has(list: &[Favorite], kind: Kind, target: &str) -> bool {
    list.iter().any(|f| f.kind == kind && f.target == target)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fav(id: i64, kind: Kind, target: &str, label: &str) -> Favorite {
        Favorite {
            id,
            kind,
            target: target.to_owned(),
            label: label.to_owned(),
        }
    }

    #[test]
    fn a_kind_goes_to_the_base_and_comes_back() {
        for k in Kind::ALL {
            assert_eq!(Kind::from_key(k.key()), Some(k));
        }
        assert_eq!(Kind::from_key("inconnu"), None);
    }

    /// Dossiers d'abord, puis fiches, outils, vues, contacts ; à
    /// l'intérieur, l'ordre alphabétique sans accents.
    #[test]
    fn favorites_are_shown_by_kind_then_by_name() {
        let list = ordered(vec![
            fav(1, Kind::Drug, "7", "Tahor"),
            fav(2, Kind::Patient, "3", "Émile Zola"),
            fav(3, Kind::Drug, "2", "Eliquis"),
            fav(4, Kind::Patient, "9", "Dupont Jean"),
            fav(5, Kind::Tool, "trame", "Trame de la semaine"),
        ]);
        let labels: Vec<&str> = list.iter().map(|f| f.label.as_str()).collect();
        assert_eq!(
            labels,
            [
                "Dupont Jean",
                "Émile Zola",
                "Eliquis",
                "Tahor",
                "Trame de la semaine"
            ]
        );
        assert!(has(&list, Kind::Drug, "2"));
        assert!(!has(&list, Kind::Patient, "2"));
    }
}
