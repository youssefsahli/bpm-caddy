//! **Le tableau de l'officine** : ce qu'elle affiche pour tous, comme un
//! tableau des départs — la pharmacie de garde, les produits qu'elle a
//! (quand d'autres en manquent), ses informations du moment. Des lignes,
//! chacune sur un canal, avec ses dates.
//!
//! Il se lit sur la page des patients (avec ou sans code : le code du
//! tableau, en vitrine ou sur le site de l'officine, n'ouvre que lui), et
//! sur tout écran qui ouvre cette page en mode écran ; un poste le sert
//! par la voie directe (`direct.rs`), comme la messagerie, sans serveur.
//! Il est rangé dans les réglages de l'officine (`SETTING`) et voyage
//! entre ses postes.
//!
//! Ce module ne touche ni base ni écran : il lit, borne et trie. Les
//! dates sont celles de la base (ISO) ; la page les écrit en clair.

use serde::{Deserialize, Serialize};

/// La clé du réglage de l'officine qui le tient.
pub const SETTING: &str = "tableau";
/// Combien de lignes, au plus.
pub const MOST_LINES: usize = 40;
/// La plus longue ligne : son titre…
pub const TITLE_MAX: usize = 120;
/// …et son détail.
pub const DETAIL_MAX: usize = 600;

/// Le canal d'une ligne : ce qu'elle annonce.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Canal {
    /// La pharmacie de garde : une officine, quand.
    Garde,
    /// Un produit disponible ici.
    Produit,
    /// Une information de l'officine (fermeture, campagne, service).
    #[default]
    Info,
}

impl Canal {
    pub const ALL: [Canal; 3] = [Canal::Garde, Canal::Produit, Canal::Info];

    /// La clé de son nom dans `strings.fr.toml`.
    pub fn label_key(self) -> &'static str {
        match self {
            Canal::Garde => "tableau_canal_garde",
            Canal::Produit => "tableau_canal_produit",
            Canal::Info => "tableau_canal_info",
        }
    }

    fn rank(self) -> u8 {
        match self {
            Canal::Garde => 0,
            Canal::Produit => 1,
            Canal::Info => 2,
        }
    }
}

/// Une ligne du tableau.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Ligne {
    #[serde(default)]
    pub canal: Canal,
    #[serde(default)]
    pub titre: String,
    #[serde(default)]
    pub detail: String,
    /// Le premier jour où elle s'affiche (ISO) ; vide : dès maintenant.
    #[serde(default)]
    pub du: String,
    /// Le dernier (ISO) ; vide : jusqu'à ce qu'on la retire.
    #[serde(default)]
    pub au: String,
}

impl Ligne {
    /// Affichée ce jour-là (`today`, ISO).
    pub fn shows_on(&self, today: &str) -> bool {
        (self.du.is_empty() || self.du.as_str() <= today)
            && (self.au.is_empty() || self.au.as_str() >= today)
    }

    /// Passée : son dernier jour est derrière.
    pub fn is_past(&self, today: &str) -> bool {
        !self.au.is_empty() && self.au.as_str() < today
    }

    /// Bornée : titre et détail coupés à leur longueur, sur un caractère.
    fn bounded(mut self) -> Self {
        self.titre = cut(self.titre.trim(), TITLE_MAX);
        self.detail = cut(self.detail.trim(), DETAIL_MAX);
        self.du = iso_or_empty(&self.du);
        self.au = iso_or_empty(&self.au);
        self
    }
}

fn cut(text: &str, most: usize) -> String {
    text.chars().take(most).collect()
}

/// Une date ISO (`AAAA-MM-JJ`), ou rien.
fn iso_or_empty(text: &str) -> String {
    let t = text.trim();
    let ok = t.len() == 10
        && t.char_indices().all(|(i, c)| {
            if i == 4 || i == 7 {
                c == '-'
            } else {
                c.is_ascii_digit()
            }
        });
    if ok {
        t.to_owned()
    } else {
        String::new()
    }
}

/// Le tableau entier, tel qu'il est rangé.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Tableau {
    #[serde(default)]
    pub lignes: Vec<Ligne>,
}

impl Tableau {
    /// Lu du réglage : un texte illisible, ou rien, est un tableau vide.
    /// Les lignes sans titre tombent, les autres sont bornées.
    pub fn read(text: Option<&str>) -> Self {
        // Line by line: one a later version wrote (a channel this one
        // does not know) falls alone, not the whole board.
        let raw: Vec<serde_json::Value> = text
            .and_then(|t| serde_json::from_str::<serde_json::Value>(t).ok())
            .and_then(|v| v.get("lignes").and_then(|l| l.as_array()).cloned())
            .unwrap_or_default();
        Self {
            lignes: raw
                .into_iter()
                .filter_map(|v| serde_json::from_value::<Ligne>(v).ok())
                .map(Ligne::bounded)
                .filter(|l| !l.titre.is_empty())
                .take(MOST_LINES)
                .collect(),
        }
    }

    /// Tel qu'on le range.
    pub fn text(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// **Ce qui s'affiche ce jour-là**, dans l'ordre d'un tableau : la
    /// garde d'abord, puis les produits, puis le reste ; dans un canal,
    /// ce qui commence le plus tôt d'abord, puis dans l'ordre écrit.
    pub fn today(&self, today: &str) -> Vec<&Ligne> {
        let mut shown: Vec<(usize, &Ligne)> = self
            .lignes
            .iter()
            .enumerate()
            .filter(|(_, l)| l.shows_on(today))
            .collect();
        shown.sort_by(|(ia, a), (ib, b)| {
            a.canal
                .rank()
                .cmp(&b.canal.rank())
                .then_with(|| a.du.cmp(&b.du))
                .then_with(|| ia.cmp(ib))
        });
        shown.into_iter().map(|(_, l)| l).collect()
    }

    /// Le tableau sans ses lignes passées : ce que « Retirer les lignes
    /// passées » laisse.
    pub fn without_past(&self, today: &str) -> Self {
        Self {
            lignes: self
                .lignes
                .iter()
                .filter(|l| !l.is_past(today))
                .cloned()
                .collect(),
        }
    }
}

/// **Ce que la page reçoit** : l'officine, et les lignes du jour. C'est
/// aussi la forme qu'un autre logiciel lit (`docs/CHAT_PATIENT.md`,
/// « Le tableau ») : un objet JSON, versionné par `v`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Public {
    pub v: u32,
    pub officine: String,
    #[serde(default)]
    pub adresse: String,
    #[serde(default)]
    pub telephone: String,
    #[serde(default)]
    pub horaires: String,
    /// Le jour où le poste l'a servi (ISO).
    pub le: String,
    pub lignes: Vec<Ligne>,
    /// Les adresses des postes sur la voie directe, à jour : un écran qui
    /// reste allumé des semaines suit l'officine quand elles changent.
    #[serde(default)]
    pub adresses: Vec<String>,
}

/// La version de la forme de [`Public`].
pub const PUBLIC_VERSION: u32 = 1;

impl Public {
    pub fn of(
        tableau: &Tableau,
        today: &str,
        officine: &str,
        adresse: &str,
        telephone: &str,
        horaires: &str,
    ) -> Self {
        Self {
            v: PUBLIC_VERSION,
            officine: officine.to_owned(),
            adresse: adresse.to_owned(),
            telephone: telephone.to_owned(),
            horaires: horaires.to_owned(),
            le: today.to_owned(),
            lignes: tableau.today(today).into_iter().cloned().collect(),
            adresses: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(canal: Canal, titre: &str, du: &str, au: &str) -> Ligne {
        Ligne {
            canal,
            titre: titre.into(),
            detail: String::new(),
            du: du.into(),
            au: au.into(),
        }
    }

    #[test]
    fn a_line_shows_between_its_dates_inclusive() {
        let l = line(Canal::Info, "Fermé", "2026-10-20", "2026-10-22");
        assert!(!l.shows_on("2026-10-19"));
        assert!(l.shows_on("2026-10-20"));
        assert!(l.shows_on("2026-10-22"));
        assert!(!l.shows_on("2026-10-23"));
        assert!(l.is_past("2026-10-23"));
        let open = line(Canal::Info, "Toujours", "", "");
        assert!(open.shows_on("1999-01-01") && !open.is_past("2999-01-01"));
    }

    #[test]
    fn the_board_reads_in_board_order() {
        let t = Tableau {
            lignes: vec![
                line(Canal::Info, "Vaccination grippe", "", ""),
                line(Canal::Produit, "Amoxicilline buvable", "", ""),
                line(
                    Canal::Garde,
                    "Pharmacie du Marché",
                    "2026-10-18",
                    "2026-10-18",
                ),
                line(
                    Canal::Garde,
                    "Pharmacie du Port",
                    "2026-10-17",
                    "2026-10-17",
                ),
                line(
                    Canal::Garde,
                    "Pharmacie de la Gare",
                    "2026-10-25",
                    "2026-10-25",
                ),
            ],
        };
        let titles =
            |day: &str| -> Vec<String> { t.today(day).iter().map(|l| l.titre.clone()).collect() };
        assert_eq!(
            titles("2026-10-17"),
            [
                "Pharmacie du Port",
                "Amoxicilline buvable",
                "Vaccination grippe"
            ]
        );
        assert_eq!(titles("2026-10-18")[0], "Pharmacie du Marché");
        assert_eq!(t.without_past("2026-10-19").lignes.len(), 3);
    }

    #[test]
    fn what_is_read_is_bounded() {
        let long = "x".repeat(TITLE_MAX + 50);
        let mut raw = Tableau {
            lignes: vec![
                line(Canal::Info, &long, "20/10/2026", "2026-10-22"),
                line(Canal::Info, "   ", "", ""),
            ],
        };
        raw.lignes
            .extend((0..MOST_LINES + 5).map(|i| line(Canal::Produit, &format!("p{i}"), "", "")));
        let t = Tableau::read(Some(&raw.text()));
        assert_eq!(t.lignes[0].titre.chars().count(), TITLE_MAX);
        assert_eq!(t.lignes[0].du, "", "not a date the base writes");
        assert_eq!(t.lignes[0].au, "2026-10-22");
        assert!(t.lignes.iter().all(|l| !l.titre.trim().is_empty()));
        assert_eq!(t.lignes.len(), MOST_LINES);
        assert_eq!(Tableau::read(Some("{")), Tableau::default());
        assert_eq!(Tableau::read(None), Tableau::default());
        // An unknown channel from a later version: that line falls, the
        // rest of the board stays.
        let t = Tableau::read(Some(
            r#"{"lignes":[{"canal":"meteo","titre":"x"},{"canal":"garde","titre":"y"}]}"#,
        ));
        assert_eq!(t.lignes.len(), 1);
        assert_eq!(t.lignes[0].titre, "y");
    }

    #[test]
    fn the_public_form_carries_the_lines_of_the_day() {
        let t = Tableau {
            lignes: vec![
                line(Canal::Info, "Hier", "", "2026-10-15"),
                line(Canal::Garde, "Ce soir", "2026-10-16", "2026-10-16"),
            ],
        };
        let p = Public::of(&t, "2026-10-16", "Pharmacie du Port", "", "", "");
        assert_eq!(p.v, PUBLIC_VERSION);
        assert_eq!(p.lignes.len(), 1);
        assert_eq!(p.lignes[0].titre, "Ce soir");
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains(r#""canal":"garde""#), "{json}");
    }
}
