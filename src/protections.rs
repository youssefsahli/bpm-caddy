//! Les protections périodiques réutilisables : coupes et culottes
//! menstruelles prises en charge depuis le 1er octobre 2026.
//!
//! Ce que les textes disent, et que le module lit :
//!
//! - **Qui** (art. R. 162-145 CSS, décret n° 2026-288 du 17/04/2026) :
//!   la personne de moins de 26 ans, ou bénéficiaire de la
//!   complémentaire santé solidaire (C2S) quel que soit son âge.
//! - **Combien** : deux produits par période annuelle, coupe ou culotte
//!   dans n'importe quelle combinaison. La période débute à la première
//!   délivrance, puis court de date à date.
//! - **Comment** : sans ordonnance, en officine (jusqu'au 31/12/2028 au
//!   plus tard) ; facturation par le code individuel du fabricant, le
//!   pharmacien inscrit son propre numéro comme prescripteur ; le code
//!   générique est rejeté.
//! - **Hors LPP au sens de l'article L. 165-1** : ces produits sont sur
//!   la liste propre de l'article L. 162-59, même s'ils se facturent par
//!   le canal LPP.
//!
//! Les prix ne sont pas livrés : ils sont fixés par arrêté et se
//! vérifient au moment de la délivrance.
//!
//! Pur et sans horloge.

/// Produits pris en charge par période annuelle.
pub const PER_YEAR: u32 = 2;
/// Âge en deçà duquel la prise en charge est ouverte sans condition.
pub const AGE_LIMIT: u32 = 26;

/// Le type de produit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Culotte,
    Coupe,
}

impl Kind {
    pub fn key(self) -> &'static str {
        match self {
            Kind::Culotte => "culotte",
            Kind::Coupe => "coupe",
        }
    }

    pub fn from_key(key: &str) -> Option<Kind> {
        [Kind::Culotte, Kind::Coupe]
            .into_iter()
            .find(|k| k.key() == key)
    }

    pub fn label(self) -> &'static str {
        match self {
            Kind::Culotte => "Culotte menstruelle",
            Kind::Coupe => "Coupe menstruelle",
        }
    }
}

/// Un code individuel de la liste prévue à l'article L. 162-59.
pub struct Code {
    pub code: &'static str,
    pub kind: Kind,
    /// Le fabricant ou le distributeur, tel que le libellé le nomme.
    pub maker: &'static str,
}

/// Les codes individuels publiés dans la base de codage de l'Assurance
/// Maladie (version du 30/09/2026). La liste s'allonge à chaque
/// référencement : un produit absent ici peut avoir été référencé
/// depuis, et la base de codage fait foi.
pub const CODES: &[Code] = &[
    Code {
        code: "6706971",
        kind: Kind::Culotte,
        maker: "Elia Innovation",
    },
    Code {
        code: "6724578",
        kind: Kind::Culotte,
        maker: "Biocodex",
    },
    Code {
        code: "6790121",
        kind: Kind::Culotte,
        maker: "Corman SpA",
    },
    Code {
        code: "6775452",
        kind: Kind::Culotte,
        maker: "Lemahieu",
    },
    Code {
        code: "6784014",
        kind: Kind::Culotte,
        maker: "Sisters Republic",
    },
    Code {
        code: "6730544",
        kind: Kind::Coupe,
        maker: "Claripharm",
    },
    Code {
        code: "6759105",
        kind: Kind::Coupe,
        maker: "Elia Innovation",
    },
    Code {
        code: "6778261",
        kind: Kind::Coupe,
        maker: "Petites Choses",
    },
];

/// Les codes génériques : rejetés à la facturation, qui exige le code
/// individuel du fabricant.
pub const GENERIC: [(&str, Kind); 2] = [("1724852", Kind::Culotte), ("1719621", Kind::Coupe)];

/// Le code individuel, s'il est dans la liste livrée.
pub fn code(c: &str) -> Option<&'static Code> {
    let c = c.trim();
    CODES.iter().find(|x| x.code == c)
}

/// Pourquoi une personne est éligible, ou pourquoi elle ne l'est pas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Eligibility {
    /// Moins de 26 ans.
    Age,
    /// Bénéficiaire de la C2S.
    C2s,
    /// 26 ans ou plus, sans C2S.
    No,
    /// Âge inconnu et C2S non déclarée : à demander.
    Unknown,
}

pub fn eligibility(age: Option<u32>, c2s: bool) -> Eligibility {
    match (age, c2s) {
        (_, true) => Eligibility::C2s,
        (Some(a), false) if a < AGE_LIMIT => Eligibility::Age,
        (Some(_), false) => Eligibility::No,
        (None, false) => Eligibility::Unknown,
    }
}

/// Une délivrance passée : date ISO et nombre de produits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Delivery {
    pub on: String,
    pub quantity: u32,
}

/// La période annuelle en cours et ce qu'il y reste.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Period {
    /// ISO ; `None` avant toute délivrance : la période s'ouvrira avec
    /// la première.
    pub start: Option<String>,
    /// ISO, premier jour de la période suivante.
    pub next: Option<String>,
    pub used: u32,
    pub remaining: u32,
}

/// Lire les délivrances : la période débute à la première délivrance et
/// court de date à date, d'année en année.
pub fn period(history: &[Delivery], today: &str) -> Period {
    let mut past: Vec<&Delivery> = history
        .iter()
        .filter(|d| !d.on.is_empty() && d.on.as_str() <= today)
        .collect();
    past.sort_by(|a, b| a.on.cmp(&b.on));
    let Some(first) = past.first() else {
        return Period {
            start: None,
            next: None,
            used: 0,
            remaining: PER_YEAR,
        };
    };
    // L'anniversaire de la première délivrance qui précède ou est le
    // jour : on avance d'un an tant que l'année suivante a commencé.
    let mut start = first.on.clone();
    loop {
        match crate::date::add_months(&start, 12) {
            Some(n) if n.as_str() <= today => start = n,
            _ => break,
        }
    }
    let next = crate::date::add_months(&start, 12);
    let used: u32 = past
        .iter()
        .filter(|d| d.on.as_str() >= start.as_str())
        .map(|d| d.quantity)
        .sum();
    Period {
        remaining: PER_YEAR.saturating_sub(used),
        start: Some(start),
        next,
        used,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn under_twenty_six_or_c2s() {
        assert_eq!(eligibility(Some(25), false), Eligibility::Age);
        assert_eq!(eligibility(Some(26), false), Eligibility::No);
        assert_eq!(eligibility(Some(45), true), Eligibility::C2s);
        assert_eq!(eligibility(None, false), Eligibility::Unknown);
    }

    #[test]
    fn the_period_opens_with_the_first_delivery_and_runs_date_to_date() {
        let d = |on: &str, quantity| Delivery {
            on: on.to_owned(),
            quantity,
        };
        // Rien de délivré : deux produits, la période n'est pas ouverte.
        let p = period(&[], "2026-10-08");
        assert_eq!((p.start, p.remaining), (None, 2));
        // Une culotte le 02/10 : il en reste une jusqu'au 02/10/2027.
        let p = period(&[d("2026-10-02", 1)], "2026-10-08");
        assert_eq!(p.start.as_deref(), Some("2026-10-02"));
        assert_eq!(p.next.as_deref(), Some("2027-10-02"));
        assert_eq!(p.remaining, 1);
        // Deux produits : plus rien avant l'anniversaire.
        let h = [d("2026-10-02", 1), d("2027-03-15", 1)];
        assert_eq!(period(&h, "2027-10-01").remaining, 0);
        // Le jour anniversaire ouvre la période suivante.
        let p = period(&h, "2027-10-02");
        assert_eq!((p.start.as_deref(), p.remaining), (Some("2027-10-02"), 2));
        // Deux ans plus tard, sans délivrance entre-temps : la période
        // court toujours de date à date depuis la première.
        let p = period(&[d("2026-10-02", 2)], "2028-11-01");
        assert_eq!(p.start.as_deref(), Some("2028-10-02"));
        assert_eq!(p.remaining, 2);
    }

    #[test]
    fn every_code_is_seven_digits_and_unique() {
        let mut all: Vec<&str> = CODES.iter().map(|c| c.code).collect();
        all.extend(GENERIC.iter().map(|g| g.0));
        for c in &all {
            assert!(c.len() == 7 && c.chars().all(|x| x.is_ascii_digit()), "{c}");
        }
        let n = all.len();
        all.sort_unstable();
        all.dedup();
        assert_eq!(n, all.len());
        assert_eq!(code(" 6706971 ").map(|c| c.kind), Some(Kind::Culotte));
        assert!(
            code("1724852").is_none(),
            "le générique n'est pas un code individuel"
        );
    }
}
