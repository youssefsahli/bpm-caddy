//! Où en est cette ordonnance — la question que le patient pose au
//! comptoir et à laquelle aucun écran ne répondait.
//!
//! « Il m'en reste combien ? », « c'est la dernière ? », « je dois
//! retourner voir le médecin quand ? ». Trois questions, une seule
//! réponse, et elle est écrite au dos de l'ordonnance en petit, quand
//! elle l'est. Le dossier porte déjà le jour de la prescription, la
//! durée qu'elle couvre et le nombre de renouvellements autorisés : ce
//! module en fait un **état**, et l'état se dessine de quatre façons au
//! choix de l'officine.
//!
//! # Les règles, une par test
//!
//! * **Pas de date, pas d'étape.** Sans jour de prescription ou sans
//!   durée, le module dit ce qui manque et ne conclut rien — la règle
//!   de `renal.rs`, qui refuse de juger une ordonnance sans DFG. Une
//!   ordonnance datée d'un « à peu près » est une date de
//!   renouvellement fausse sur une feuille que le patient croit.
//! * **La dernière boîte n'est pas la fin.** Ce qu'il faut annoncer
//!   n'est pas le jour où le traitement s'arrête, c'est le jour où il
//!   faut avoir vu le prescripteur — quelques jours avant, le délai de
//!   prévenance, parce qu'un rendez-vous ne se prend pas le matin pour
//!   le soir.
//! * **Une étape se compte, elle ne se déduit pas du calendrier.** Un
//!   patient qui revient trois semaines en retard n'en est pas à la
//!   quatrième délivrance : il en est à la deuxième, tard. Le rang
//!   vient de ce que l'officine a délivré, jamais d'une division du
//!   temps écoulé par la durée.
//! * **Zéro n'est pas un.** Une délivrance non notée ne se lit pas
//!   « première » : la feuille dit alors ce que l'ordonnance permet, et
//!   pas où l'on en est. C'est la règle de l'ordonnancier — une case
//!   vide n'est pas un zéro.
//! * **Une ordonnance non renouvelable n'a pas d'étapes.** Un
//!   antibiotique de sept jours n'a pas de jauge à cent pour cent : il
//!   a une date de fin. Dessiner « 1 sur 1 » fabrique une question là
//!   où il n'y en a pas.
//! * **Une visualisation change le dessin, jamais la lecture.** Les
//!   quatre façons de montrer l'avancement lisent le même état : c'est
//!   une affaire de goût et de place sur la feuille, et non un second
//!   calcul qui finirait par répondre autre chose.
//!
//! Pur et testé, sans horloge : le jour est passé.

use crate::date::{add_days, days_between};

/// Ce que l'ordonnance dit d'elle-même.
///
/// Quatre champs, et chacun répond à une question que les autres ne
/// posent pas : quand elle a été écrite, ce qu'une délivrance couvre,
/// combien de fois elle se renouvelle, et combien de fois elle l'a
/// déjà été.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Prescription {
    /// Le jour de la prescription, ISO. Vide = non noté.
    pub prescribed_on: String,
    /// Ce qu'une délivrance couvre, en jours. 0 = non dit.
    pub duration_days: u32,
    /// « À renouveler N fois ». 0 = non renouvelable.
    pub renewals: u32,
    /// Combien de délivrances ont été faites, celle du jour comprise.
    /// 0 = aucune n'a été notée, ce qui n'est pas « la première ».
    pub dispensed: u32,
}

impl Prescription {
    /// Combien de délivrances l'ordonnance porte en tout.
    pub fn steps(&self) -> u32 {
        self.renewals.saturating_add(1)
    }

    /// Y a-t-il de quoi calculer quoi que ce soit ?
    pub fn is_dated(&self) -> bool {
        !self.prescribed_on.trim().is_empty() && self.duration_days > 0
    }
}

/// Où en est l'ordonnance.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum State {
    /// Il manque la date ou la durée : rien n'est calculé.
    Unknown,
    /// En cours, et il reste des renouvellements.
    Running,
    /// La dernière délivrance : c'est maintenant qu'on prend
    /// rendez-vous.
    Last,
    /// L'ordonnance est épuisée ou périmée.
    Over,
}

/// L'état d'une étape, pour qui la dessine.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Step {
    Done,
    Current,
    Todo,
}

/// Une délivrance de l'ordonnance : son rang, le jour où elle est
/// attendue, et où elle en est.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Pip {
    pub n: u32,
    /// Le jour attendu, ISO — la prescription plus les durées
    /// précédentes. `None` quand l'ordonnance n'est pas datée.
    ///
    /// **Attendu, et non constaté** : c'est le calendrier de
    /// l'ordonnance, et une délivrance faite en retard ne le déplace
    /// pas. Le rang, lui, se compte.
    pub due: Option<String>,
    pub step: Step,
}

/// L'ordonnance lue.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Stand {
    pub state: State,
    /// La délivrance en cours, 0 quand aucune n'est notée.
    pub step: u32,
    /// Combien en tout.
    pub steps: u32,
    /// Jusqu'à quand la délivrance du jour couvre, ISO.
    pub covered_to: Option<String>,
    /// Le jour où l'ordonnance est épuisée, ISO.
    pub ends_on: Option<String>,
    /// Le jour avant lequel il faut avoir vu le prescripteur, ISO.
    /// **Ce n'est pas [`Self::ends_on`]** : c'est lui moins le délai de
    /// prévenance.
    pub see_by: Option<String>,
    /// Combien de jours restent avant l'épuisement. Négatif = dépassé.
    pub days_left: Option<i64>,
    /// Ce qui manque pour conclure, quand rien n'est calculé.
    pub missing: &'static str,
}

impl Stand {
    /// Les délivrances, dans l'ordre : ce que les quatre
    /// visualisations lisent — et elles lisent **celle-ci**, il n'y a
    /// pas deux calculs de l'avancement dans cette application.
    pub fn pips(&self, p: &Prescription) -> Vec<Pip> {
        (1..=self.steps)
            .map(|n| Pip {
                n,
                due: p
                    .is_dated()
                    .then(|| {
                        add_days(
                            p.prescribed_on.trim(),
                            i64::from(n - 1) * i64::from(p.duration_days),
                        )
                    })
                    .flatten(),
                step: match () {
                    () if self.step == 0 => Step::Todo,
                    () if n < self.step => Step::Done,
                    () if n == self.step => Step::Current,
                    () => Step::Todo,
                },
            })
            .collect()
    }

    /// L'ordonnance porte-t-elle un avancement à montrer ?
    ///
    /// Non renouvelable, il n'y a pas d'étapes : une jauge à cent pour
    /// cent sur un antibiotique de sept jours fabrique une question là
    /// où il n'y en a pas.
    pub fn has_progress(&self) -> bool {
        self.steps > 1 && self.step > 0
    }
}

/// Lire une ordonnance au jour dit.
///
/// `notice_days` est le délai de prévenance : de combien de jours le
/// rendez-vous doit précéder l'épuisement. Zéro le supprime, et c'est
/// un réglage et non un défaut — une officine qui ne veut pas de cette
/// phrase l'enlève.
pub fn read(p: &Prescription, today: &str, notice_days: u32) -> Stand {
    let steps = p.steps();
    if !p.is_dated() {
        return Stand {
            state: State::Unknown,
            step: p.dispensed.min(steps),
            steps,
            covered_to: None,
            ends_on: None,
            see_by: None,
            days_left: None,
            missing: if p.prescribed_on.trim().is_empty() {
                "le jour de la prescription"
            } else {
                "la durée couverte par une délivrance"
            },
        };
    }
    let from = p.prescribed_on.trim();
    let duration = i64::from(p.duration_days);
    let ends_on = add_days(from, duration * i64::from(steps));
    // **Un rendez-vous ne se prend pas dans le passé.** Quand le délai
    // de prévenance dépasse ce qui reste — sept jours de prévenance sur
    // un antibiotique de cinq —, la date tombait avant aujourd'hui, et
    // parfois avant la prescription elle-même : la feuille remise le
    // jour même disait « avant le 20/09 » d'une ordonnance du 22. Il
    // n'y a alors plus de date à annoncer, seulement la fin.
    let see_by = ends_on
        .as_deref()
        .and_then(|e| add_days(e, -i64::from(notice_days)))
        .filter(|d| d.as_str() >= today);
    // **La boîte du jour couvre à partir d'aujourd'hui**, et non à
    // partir d'un rang multiplié par une durée : c'est le jour de la
    // délivrance qui commande, et il est passé en argument. Un patient
    // venu avec huit jours de retard n'est pas couvert huit jours de
    // plus.
    let days_left = ends_on.as_deref().and_then(|e| days_between(today, e));
    let expired = days_left.is_some_and(|d| d < 0);
    let step = p.dispensed.min(steps);
    let state = match () {
        () if expired || p.dispensed > steps => State::Over,
        () if step == 0 => State::Running,
        () if step >= steps => State::Last,
        () => State::Running,
    };
    // Et « couvert jusqu'au » ne se dit que d'une délivrance notée, sur
    // une ordonnance qui court : la feuille écrivait « délivrance non
    // notée » puis la date qu'elle couvrait.
    let covered_to = (step > 0 && state != State::Over)
        .then(|| add_days(today, duration))
        .flatten();
    Stand {
        state,
        step,
        steps,
        covered_to,
        ends_on,
        see_by,
        days_left,
        missing: "",
    }
}

/// Rassembler les traitements qui viennent de la même ordonnance.
///
/// Les champs sont portés par chaque ligne du dossier, parce que c'est
/// ainsi que le dossier est fait ; mais une ordonnance de six lignes
/// est **une** ordonnance, et écrire six fois « délivrance 2 sur 3,
/// rendez-vous avant le 13 décembre » sur une feuille remise au patient
/// est le meilleur moyen qu'il n'en lise aucune.
///
/// L'ordre est celui de la première ligne qui la porte : la feuille
/// suit l'ordonnance, et une ordonnance triée par date de prescription
/// ne ressemble plus à celle que le patient tient.
pub fn group(lines: &[(String, Prescription)]) -> Vec<(Prescription, Vec<String>)> {
    let mut out: Vec<(Prescription, Vec<String>)> = Vec::new();
    for (name, p) in lines {
        match out.iter_mut().find(|(seen, _)| seen == p) {
            Some((_, names)) => names.push(name.clone()),
            None => out.push((p.clone(), vec![name.clone()])),
        }
    }
    out
}

/// Comment l'avancement se dessine.
///
/// Quatre, parce que les officines ne se ressemblent pas et que la
/// place sur une feuille n'est pas la même selon ce qu'on y met : des
/// pastilles se lisent d'un coup d'œil, une jauge dit la proportion,
/// les dates disent *quand*, et la phrase seule tient sur une ligne. Le
/// choix est un réglage ; il ne change pas ce qui est lu.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Viz {
    /// Une pastille par délivrance, pleine, cerclée ou vide.
    #[default]
    Pastilles,
    /// Une jauge remplie à la proportion des délivrances faites.
    Jauge,
    /// Le calendrier des délivrances attendues.
    Dates,
    /// Rien qu'une phrase.
    Phrase,
}

impl Viz {
    pub const ALL: [Viz; 4] = [Viz::Pastilles, Viz::Jauge, Viz::Dates, Viz::Phrase];

    /// La clé écrite dans `config.toml`.
    pub fn key(self) -> &'static str {
        match self {
            Viz::Pastilles => "pastilles",
            Viz::Jauge => "jauge",
            Viz::Dates => "dates",
            Viz::Phrase => "phrase",
        }
    }

    /// Ce que l'écran en dit.
    pub fn label(self) -> &'static str {
        match self {
            Viz::Pastilles => "Pastilles",
            Viz::Jauge => "Jauge",
            Viz::Dates => "Dates",
            Viz::Phrase => "Phrase seule",
        }
    }

    /// La clé lue. Une clé inconnue rend le défaut plutôt qu'une
    /// erreur : un réglage mal tapé ne doit pas empêcher d'imprimer.
    pub fn from_key(key: &str) -> Self {
        Self::ALL
            .into_iter()
            .find(|v| v.key() == key.trim())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(on: &str, days: u32, renewals: u32, dispensed: u32) -> Prescription {
        Prescription {
            prescribed_on: on.to_owned(),
            duration_days: days,
            renewals,
            dispensed,
        }
    }

    /// **Pas de date, pas d'étape.**
    ///
    /// Et ce qui manque est nommé : une feuille qui laisse un blanc
    /// sans dire pourquoi se lit comme un défaut d'impression.
    #[test]
    fn no_date_no_stage() {
        let s = read(&p("", 30, 2, 1), "2026-09-21", 7);
        assert_eq!(s.state, State::Unknown);
        assert!(s.ends_on.is_none() && s.see_by.is_none() && s.covered_to.is_none());
        assert_eq!(s.missing, "le jour de la prescription");

        // Une durée manquante est l'autre moitié, et elle se nomme
        // autrement : dire « il manque la date » d'une ordonnance datée
        // envoie chercher ce qui est déjà là.
        let s = read(&p("2026-09-21", 0, 2, 1), "2026-09-21", 7);
        assert_eq!(s.state, State::Unknown);
        assert_eq!(s.missing, "la durée couverte par une délivrance");

        // Et les étapes restent lisibles : l'ordonnance dit ce qu'elle
        // permet même quand on ne sait pas où l'on en est.
        assert_eq!(s.steps, 3);
    }

    /// **La dernière boîte n'est pas la fin.**
    ///
    /// Ce qu'on annonce est le jour où il faut avoir vu le médecin, et
    /// il est avant le jour où il n'y a plus rien à prendre.
    #[test]
    fn the_last_box_is_not_the_end() {
        let s = read(&p("2026-09-21", 30, 2, 3), "2026-11-20", 7);
        assert_eq!(s.state, State::Last);
        assert_eq!(s.ends_on.as_deref(), Some("2026-12-20"));
        assert_eq!(s.see_by.as_deref(), Some("2026-12-13"));
        assert!(
            s.see_by < s.ends_on,
            "le rendez-vous se prend avant l'épuisement"
        );

        // Un délai nul le supprime : c'est un réglage de l'officine, et
        // pas un défaut qu'il faudrait contourner.
        let s = read(&p("2026-09-21", 30, 2, 3), "2026-11-20", 0);
        assert_eq!(s.see_by, s.ends_on);
    }

    /// **Une étape se compte, elle ne se déduit pas du calendrier.**
    ///
    /// Un patient qui revient trois semaines en retard en est à sa
    /// deuxième délivrance, tard — pas à sa quatrième. Diviser le temps
    /// écoulé par la durée donnerait le contraire, et c'est la faute
    /// que ce test interdit.
    #[test]
    fn a_stage_is_counted_never_guessed_from_the_days() {
        let ordonnance = p("2026-01-05", 30, 5, 2);
        let early = read(&ordonnance, "2026-02-04", 7);
        let late = read(&ordonnance, "2026-04-28", 7);
        assert_eq!(early.step, 2);
        assert_eq!(late.step, 2, "le retard ne fait pas avancer le rang");
        assert_eq!(early.steps, late.steps);
        // Ce que le jour change, en revanche, c'est jusqu'à quand la
        // boîte du jour couvre : elle part du jour de la délivrance.
        assert_ne!(early.covered_to, late.covered_to);
    }

    /// **Zéro n'est pas un.**
    ///
    /// Une délivrance non notée ne se lit pas « première » : l'état
    /// dit ce que l'ordonnance permet, et se tait sur le rang.
    #[test]
    fn nothing_recorded_is_not_a_first_dispensing() {
        let s = read(&p("2026-09-21", 30, 2, 0), "2026-09-21", 7);
        assert_eq!(s.step, 0);
        assert!(!s.has_progress());
        assert!(
            s.pips(&p("2026-09-21", 30, 2, 0))
                .iter()
                .all(|pip| pip.step == Step::Todo),
            "aucune pastille n'est courante quand rien n'est noté"
        );
        // Mais les dates, elles, sont connues : l'ordonnance est datée.
        assert!(s.ends_on.is_some());
    }

    /// **Une ordonnance non renouvelable n'a pas d'étapes.**
    ///
    /// Sept jours d'antibiotique n'ont pas de jauge : ils ont une date
    /// de fin.
    #[test]
    fn a_prescription_that_is_not_renewable_has_no_progress() {
        let one = p("2026-09-21", 7, 0, 1);
        let s = read(&one, "2026-09-21", 7);
        assert_eq!(s.steps, 1);
        assert_eq!(s.state, State::Last);
        assert!(!s.has_progress(), "« 1 sur 1 » n'est pas un avancement");
        assert_eq!(s.pips(&one).len(), 1);
        assert_eq!(s.ends_on.as_deref(), Some("2026-09-28"));
    }

    /// **Une visualisation change le dessin, jamais la lecture.**
    ///
    /// Les quatre lisent le même état ; le choix est une affaire de
    /// goût et de place, et non un second calcul qui finirait par
    /// répondre autre chose. C'est la règle de la couverture et du
    /// planning — une question, un calcul.
    #[test]
    fn a_visualisation_changes_the_drawing_never_the_reading() {
        let o = p("2026-09-21", 30, 2, 2);
        let s = read(&o, "2026-10-21", 7);
        let reference = s.pips(&o);
        for v in Viz::ALL {
            // Le seul effet d'une visualisation est son nom : elle
            // n'entre dans aucun calcul, et c'est ce que ce test
            // vérifie en relisant le même état pour chacune.
            assert!(!v.label().is_empty());
            assert_eq!(s.pips(&o), reference, "{}", v.key());
        }
        assert_eq!(
            reference.iter().map(|p| p.step).collect::<Vec<_>>(),
            vec![Step::Done, Step::Current, Step::Todo]
        );
        // Et les jours attendus sont ceux de l'ordonnance, un mois
        // d'intervalle.
        assert_eq!(reference[0].due.as_deref(), Some("2026-09-21"));
        assert_eq!(reference[1].due.as_deref(), Some("2026-10-21"));
        assert_eq!(reference[2].due.as_deref(), Some("2026-11-20"));
    }

    /// Une ordonnance dont la validité est passée le dit, et ne
    /// continue pas à compter des délivrances qu'elle ne permet plus.
    #[test]
    fn an_expired_prescription_says_so() {
        let s = read(&p("2026-01-05", 30, 2, 3), "2026-06-01", 7);
        assert_eq!(s.state, State::Over);
        assert!(s.days_left.is_some_and(|d| d < 0));

        // Épuisée par le compte plutôt que par le calendrier : quatre
        // délivrances sur trois possibles.
        let s = read(&p("2026-09-21", 30, 2, 4), "2026-10-01", 7);
        assert_eq!(s.state, State::Over);
    }

    /// **Une ordonnance de six lignes est une ordonnance.**
    ///
    /// Et l'ordre est celui du dossier : une feuille qui reclasse les
    /// traitements par date de prescription ne ressemble plus à
    /// l'ordonnance que le patient tient dans l'autre main.
    #[test]
    fn treatments_from_one_prescription_make_one_block() {
        let chronic = p("2026-09-21", 30, 2, 2);
        let antibiotic = p("2026-09-21", 7, 0, 1);
        let blocks = group(&[
            ("Coversyl 5 mg".to_owned(), chronic.clone()),
            ("Zeclar 500 mg".to_owned(), antibiotic.clone()),
            ("Tahor 20 mg".to_owned(), chronic.clone()),
        ]);
        assert_eq!(blocks.len(), 2, "deux ordonnances, pas trois");
        assert_eq!(blocks[0].0, chronic);
        assert_eq!(blocks[0].1, vec!["Coversyl 5 mg", "Tahor 20 mg"]);
        assert_eq!(blocks[1].0, antibiotic);

        // Et une ligne dont rien n'est noté ne rejoint pas un bloc
        // daté : elle dit qu'elle n'est pas notée, ce qui est une
        // information et non un blanc.
        let blocks = group(&[
            ("Coversyl 5 mg".to_owned(), chronic),
            ("Kardégic 75".to_owned(), Prescription::default()),
        ]);
        assert_eq!(blocks.len(), 2);
        assert_eq!(read(&blocks[1].0, "2026-09-21", 7).state, State::Unknown);
    }

    /// La clé se relit, et une clé inconnue ne fait pas tomber
    /// l'impression.
    #[test]
    fn an_unknown_visualisation_key_falls_back_rather_than_fails() {
        for v in Viz::ALL {
            assert_eq!(Viz::from_key(v.key()), v);
        }
        assert_eq!(Viz::from_key("histogramme"), Viz::default());
        assert_eq!(Viz::from_key(""), Viz::default());
    }

    /// **Un rendez-vous ne se prend pas dans le passé**, et une
    /// délivrance non notée ne couvre rien. Cinq jours d'antibiotique
    /// prescrits et délivrés le 22 : la feuille disait « prenez
    /// rendez-vous avant le 20 », deux jours avant l'ordonnance.
    #[test]
    fn an_appointment_is_never_announced_in_the_past() {
        let s = read(&p("2026-09-22", 5, 0, 1), "2026-09-22", 7);
        assert_eq!(s.see_by, None, "{s:?}");
        assert_eq!(s.ends_on.as_deref(), Some("2026-09-27"));
        // En retard sur la dernière délivrance : plus de date à tenir.
        let s = read(&p("2026-06-01", 30, 2, 3), "2026-08-28", 7);
        assert_eq!(s.see_by, None, "{s:?}");
        // Rien de délivré : rien de couvert.
        let s = read(&p("2026-09-22", 30, 2, 0), "2026-09-22", 7);
        assert_eq!(s.covered_to, None, "{s:?}");
        // Terminée : rien de couvert non plus.
        let s = read(&p("2026-01-01", 30, 0, 1), "2026-09-22", 7);
        assert_eq!(s.state, State::Over);
        assert_eq!(s.covered_to, None);
    }
}
