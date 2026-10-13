//! **La feuille de l'infirmier(ère)** : ce que le patient prend à chaque
//! moment de la journée, remise avec les boîtes quand c'est une
//! infirmière qui vient les chercher et prépare le pilulier.
//!
//! La fiche de traitement se lit par médicament ; celle-ci se lit par
//! **moment** — c'est l'ordre dans lequel on remplit un pilulier. Elle lit
//! la posologie par la même fonction que le plan de prise et les
//! étiquettes (`intake::read`) : trois papiers, une lecture, et rien
//! qu'elle n'ajoute — une prise à la demande reste hors des moments, un
//! rythme non quotidien s'écrit en toutes lettres, une posologie que la
//! lecture ne comprend pas passe telle quelle.
//!
//! Ce que l'officine retient d'un patient — la case, le nom et le
//! téléphone de l'infirmier(ère), le style de la feuille — vit dans la
//! base (`settings`, une clé par dossier), écrit contre ce qui était lu.
//!
//! Pur, testé.

use crate::intake::{Kind, Moment, Reading};
use serde::{Deserialize, Serialize};

/// Les trois feuilles parmi lesquelles l'officine choisit.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Style {
    /// Une case à cocher par prise, sous chaque moment : le pilulier se
    /// remplit en cochant.
    #[default]
    Simple,
    /// Par moment, avec le repas et les remarques de la posologie, et une
    /// ligne « préparé le / par » pour chaque moment.
    Detaillee,
    /// La grille du plan de prise, un médicament par ligne et les quatre
    /// moments en colonnes.
    Grille,
}

impl Style {
    pub const ALL: [Style; 3] = [Style::Simple, Style::Detaillee, Style::Grille];

    pub fn key(self) -> &'static str {
        match self {
            Style::Simple => "simple",
            Style::Detaillee => "detaillee",
            Style::Grille => "grille",
        }
    }
}

/// Ce que l'officine retient d'un patient dont l'infirmier(ère) prend les
/// traitements.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Care {
    /// Les traitements sont remis à l'infirmier(ère) : la feuille se
    /// propose après chaque délivrance.
    pub remis: bool,
    pub nom: String,
    pub telephone: String,
    pub style: Style,
}

impl Care {
    /// La clé de réglage d'un dossier.
    pub fn key(patient_id: i64) -> String {
        format!("infirmier.{patient_id}")
    }

    /// Lu du réglage ; un texte illisible, ou rien, est « pas d'infirmier ».
    pub fn read(text: Option<&str>) -> Self {
        text.and_then(|t| serde_json::from_str(t).ok())
            .unwrap_or_default()
    }

    pub fn text(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

/// Une prise, sous un moment : le médicament et ce qu'on en prend. La
/// quantité est `None` quand la posologie donne le moment sans le nombre
/// — la feuille ne l'invente pas.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dose {
    pub name: String,
    pub quantity: Option<String>,
    /// Le repas et les remarques de la posologie.
    pub note: String,
}

/// Une ligne hors des moments : son nom et ce qui s'en dit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Apart {
    pub name: String,
    pub text: String,
}

/// La feuille, rangée par moment.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Sheet {
    /// Les quatre moments, dans l'ordre de [`Moment::ALL`].
    pub moments: [Vec<Dose>; 4],
    /// Une quantité par jour dont le moment n'est pas dit : l'heure est à
    /// convenir, et la feuille le dit plutôt que de choisir.
    pub daily: Vec<Apart>,
    /// À la demande, avec sa condition.
    pub on_demand: Vec<Apart>,
    /// Un rythme qui n'est pas quotidien, en toutes lettres.
    pub cyclic: Vec<Apart>,
    /// Ce que la lecture n'a pas compris : la posologie telle quelle.
    pub unread: Vec<Apart>,
}

impl Sheet {
    pub fn is_empty(&self) -> bool {
        self.moments.iter().all(Vec::is_empty)
            && self.daily.is_empty()
            && self.on_demand.is_empty()
            && self.cyclic.is_empty()
            && self.unread.is_empty()
    }
}

/// **Ranger les traitements d'un dossier par moment** : `(nom, posologie,
/// lecture)`, dans l'ordre du dossier.
pub fn sheet(lines: &[(String, String, Reading)]) -> Sheet {
    let mut out = Sheet::default();
    for (name, posology, reading) in lines {
        let note = [reading.meal.trim(), reading.note.trim()]
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" · ");
        let apart = |text: String| Apart {
            name: name.clone(),
            text,
        };
        match reading.kind {
            Kind::Placed if reading.has_grid() => {
                for m in Moment::ALL {
                    if let Some(take) = reading.doses[m.index()] {
                        out.moments[m.index()].push(Dose {
                            name: name.clone(),
                            quantity: take.label(),
                            note: note.clone(),
                        });
                    }
                }
            }
            Kind::Daily => out.daily.push(apart(posology.trim().to_owned())),
            Kind::OnDemand => out.on_demand.push(apart(posology.trim().to_owned())),
            Kind::Cyclic => out.cyclic.push(apart(posology.trim().to_owned())),
            _ => out.unread.push(apart(posology.trim().to_owned())),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(name: &str, poso: &str) -> (String, String, Reading) {
        (name.to_owned(), poso.to_owned(), crate::intake::read(poso))
    }

    /// Chaque prise sous son moment, avec sa quantité ; à la demande, le
    /// rythme non quotidien et l'illisible hors des moments, tels qu'écrits.
    #[test]
    fn a_file_is_sorted_by_moment() {
        let s = sheet(&[
            line("Metformine 1000 mg", "1 comprimé matin et soir"),
            line("Kardégic 75 mg", "1 sachet le matin"),
            line("Doliprane 1 g", "1 comprimé si besoin, 3 par jour maximum"),
            line(
                "Méthotrexate 10 mg",
                "1 comprimé le lundi, une fois par semaine",
            ),
            line("Crème X", "selon l'avis du dermatologue"),
        ]);
        let names = |m: Moment| -> Vec<&str> {
            s.moments[m.index()]
                .iter()
                .map(|d| d.name.as_str())
                .collect()
        };
        assert_eq!(
            names(Moment::Matin),
            ["Metformine 1000 mg", "Kardégic 75 mg"]
        );
        assert_eq!(names(Moment::Soir), ["Metformine 1000 mg"]);
        assert!(names(Moment::Midi).is_empty());
        assert_eq!(s.moments[0][0].quantity.as_deref(), Some("1"));
        assert_eq!(s.on_demand.len(), 1);
        assert_eq!(s.cyclic.len(), 1);
        assert_eq!(s.unread.len(), 1);
        assert_eq!(s.unread[0].text, "selon l'avis du dermatologue");
        assert!(!s.is_empty());
        assert!(sheet(&[]).is_empty());
    }

    /// Ce que l'officine retient se relit tel quel, et un réglage absent
    /// ou illisible ne dit pas qu'il y a une infirmière.
    #[test]
    fn care_reads_back() {
        let c = Care {
            remis: true,
            nom: "Mme Durand".to_owned(),
            telephone: "06 00 00 00 00".to_owned(),
            style: Style::Detaillee,
        };
        assert_eq!(Care::read(Some(&c.text())), c);
        assert_eq!(Care::read(None), Care::default());
        assert_eq!(Care::read(Some("{pas du json")), Care::default());
        assert!(!Care::default().remis);
        assert_eq!(Care::key(42), "infirmier.42");
    }
}
