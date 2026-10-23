//! **Le registre qualité de l'officine** : les non-conformités, de leur
//! déclaration à leur clôture.
//!
//! Une non-conformité est ce qui s'est écarté de ce qui devait se passer
//! — une erreur de délivrance rattrapée ou non, une livraison abîmée, une
//! rupture de la chaîne du froid, une réclamation, un appareil en panne.
//! Elle suit toujours le même chemin, et c'est ce chemin que le registre
//! tient :
//!
//! 1. **Déclarée** : quoi, quand, par qui, et ce qui a été fait sur le
//!    moment (l'action immédiate).
//! 2. **Analysée** : sa gravité et sa fréquence (la criticité en est le
//!    produit), et sa cause.
//! 3. **En action** : les actions correctives (supprimer la cause) et
//!    préventives (qu'elle ne revienne pas), chacune avec qui la mène et
//!    pour quand.
//! 4. **À vérifier** : les actions sont faites ; reste à dire si elles
//!    ont marché.
//! 5. **Close**, une fois l'efficacité constatée.
//!
//! L'étape n'est **jamais stockée** : elle se lit dans ce qui est écrit
//! ([`stage`]). Une étape stockée finit toujours par contredire les
//! actions qu'elle résume.
//!
//! Pur et sans horloge : la date du jour est passée en paramètre.

/// Les types de non-conformité, dans l'ordre du choix. Chacun a son
/// libellé `qual_kind_<clé>`.
pub const KINDS: [&str; 8] = [
    "dispensation",
    "fournisseur",
    "froid",
    "reclamation",
    "materiel",
    "hygiene",
    "documentaire",
    "autre",
];

/// Les familles de cause (les « 5 M », et une sixième pour le reste) ;
/// libellés `qual_cause_<clé>`.
pub const CAUSES: [&str; 6] = [
    "methode",
    "main_oeuvre",
    "materiel",
    "matiere",
    "milieu",
    "autre",
];

/// La gravité et la fréquence se cotent de 1 à 4 ; 0 : pas encore cotée.
pub const SCALE: i64 = 4;

/// Une non-conformité, telle que la base la garde.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NonConformite {
    pub id: i64,
    /// ISO.
    pub declared_on: String,
    /// Les initiales de qui l'a déclarée.
    pub declared_by: String,
    /// Une clé de [`KINDS`].
    pub kind: String,
    pub title: String,
    pub description: String,
    /// Ce qui a été fait sur le moment.
    pub immediate: String,
    /// 1 à [`SCALE`] ; 0 : non cotée.
    pub severity: i64,
    /// 1 à [`SCALE`] ; 0 : non cotée.
    pub likelihood: i64,
    /// Une clé de [`CAUSES`], ou vide.
    pub cause_kind: String,
    pub cause: String,
    /// Une déclaration faite au-dehors (pharmacovigilance, matériovigilance,
    /// fournisseur, ARS…), telle qu'on l'écrit.
    pub declaration: String,
    /// 0 : pas encore vérifiée ; 1 : efficace ; 2 : pas efficace.
    pub effective: i64,
    /// Ce que la vérification a constaté.
    pub verification: String,
    /// ISO ; vide tant qu'elle est ouverte.
    pub closed_on: String,
    pub closed_by: String,
}

/// Une action décidée pour une non-conformité.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Action {
    pub id: i64,
    pub nc_id: i64,
    /// Préventive (qu'elle ne revienne pas) plutôt que corrective
    /// (supprimer la cause).
    pub preventive: bool,
    pub what: String,
    /// Les initiales de qui la mène.
    pub owner: String,
    /// ISO ; vide : sans échéance.
    pub due_on: String,
    /// ISO ; vide tant qu'elle n'est pas faite.
    pub done_on: String,
}

/// Où en est une non-conformité.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Stage {
    Declaree,
    Analysee,
    EnAction,
    AVerifier,
    Close,
}

impl Stage {
    pub const ALL: [Stage; 5] = [
        Stage::Declaree,
        Stage::Analysee,
        Stage::EnAction,
        Stage::AVerifier,
        Stage::Close,
    ];

    /// La clé de chaîne de son libellé.
    pub fn label_key(self) -> &'static str {
        match self {
            Stage::Declaree => "qual_stage_declaree",
            Stage::Analysee => "qual_stage_analysee",
            Stage::EnAction => "qual_stage_action",
            Stage::AVerifier => "qual_stage_verifier",
            Stage::Close => "qual_stage_close",
        }
    }
}

/// Les actions d'une non-conformité, dans l'ordre où elles ont été
/// décidées.
pub fn actions_of(nc: i64, actions: &[Action]) -> Vec<&Action> {
    actions.iter().filter(|a| a.nc_id == nc).collect()
}

/// L'analyse est faite : gravité, fréquence et cause écrites.
pub fn analysed(nc: &NonConformite) -> bool {
    criticity(nc).is_some() && !nc.cause.trim().is_empty()
}

/// L'étape, lue dans ce qui est écrit.
pub fn stage(nc: &NonConformite, actions: &[Action]) -> Stage {
    if !nc.closed_on.trim().is_empty() {
        return Stage::Close;
    }
    let mine = actions_of(nc.id, actions);
    if !mine.is_empty() && mine.iter().all(|a| done(a)) {
        Stage::AVerifier
    } else if !mine.is_empty() {
        Stage::EnAction
    } else if analysed(nc) {
        Stage::Analysee
    } else {
        Stage::Declaree
    }
}

/// Faite.
pub fn done(a: &Action) -> bool {
    !a.done_on.trim().is_empty()
}

/// Pas faite, et son échéance est passée.
pub fn overdue(a: &Action, today: &str) -> bool {
    !done(a) && !a.due_on.trim().is_empty() && a.due_on.as_str() < today
}

/// Gravité × fréquence, de 1 à 16 ; `None` tant que l'une des deux n'est
/// pas cotée.
pub fn criticity(nc: &NonConformite) -> Option<i64> {
    let ok = |v: i64| (1..=SCALE).contains(&v);
    (ok(nc.severity) && ok(nc.likelihood)).then_some(nc.severity * nc.likelihood)
}

/// Le niveau d'une criticité : la grille que la fiche imprime.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    /// 1 à 3.
    Faible,
    /// 4 à 8.
    Moderee,
    /// 9 à 16 : une action est attendue.
    Elevee,
}

impl Level {
    pub fn of(criticity: i64) -> Level {
        match criticity {
            ..=3 => Level::Faible,
            4..=8 => Level::Moderee,
            _ => Level::Elevee,
        }
    }

    pub fn label_key(self) -> &'static str {
        match self {
            Level::Faible => "qual_level_low",
            Level::Moderee => "qual_level_mid",
            Level::Elevee => "qual_level_high",
        }
    }
}

/// Pourquoi une non-conformité ne peut pas encore être close — la clé
/// de chaîne de la raison — ou `None` si elle le peut.
///
/// On ne clôt pas ce qu'on n'a pas compris (l'analyse), ni ce dont on
/// n'a pas vérifié l'effet : une action faite n'est pas une action qui a
/// marché. Une action jugée **inefficace** ne clôt rien — c'est une
/// nouvelle action qu'il faut.
pub fn cannot_close(nc: &NonConformite, actions: &[Action]) -> Option<&'static str> {
    if !nc.closed_on.trim().is_empty() {
        return Some("qual_close_already");
    }
    if !analysed(nc) {
        return Some("qual_close_need_analysis");
    }
    let mine = actions_of(nc.id, actions);
    if mine.iter().any(|a| !done(a)) {
        return Some("qual_close_need_actions");
    }
    match nc.effective {
        1 => None,
        2 => Some("qual_close_not_effective"),
        _ if mine.is_empty() && !nc.verification.trim().is_empty() => None,
        _ => Some("qual_close_need_check"),
    }
}

/// Ce que le tableau de bord et l'en-tête de l'onglet disent.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Board {
    /// Non-conformités ouvertes.
    pub open: usize,
    /// Ouvertes et de criticité élevée.
    pub high: usize,
    /// Actions en retard (sur des non-conformités ouvertes).
    pub late: usize,
    /// Faites et à vérifier.
    pub to_check: usize,
}

pub fn board(ncs: &[NonConformite], actions: &[Action], today: &str) -> Board {
    let mut b = Board::default();
    for nc in ncs {
        let st = stage(nc, actions);
        if st == Stage::Close {
            continue;
        }
        b.open += 1;
        if criticity(nc).map(Level::of) == Some(Level::Elevee) {
            b.high += 1;
        }
        if st == Stage::AVerifier {
            b.to_check += 1;
        }
        b.late += actions_of(nc.id, actions)
            .iter()
            .filter(|a| overdue(a, today))
            .count();
    }
    b
}

/// **La revue annuelle** : ce que l'année a déclaré, ce qui en a été
/// fait, et ce qui reste.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Review {
    pub year: i32,
    /// Déclarées dans l'année.
    pub total: usize,
    /// Par type, dans l'ordre de [`KINDS`], les types vides compris.
    pub by_kind: Vec<(&'static str, usize)>,
    /// Faible, modérée, élevée, non cotée.
    pub by_level: [usize; 4],
    /// Par famille de cause, dans l'ordre de [`CAUSES`], puis « non
    /// dite ».
    pub by_cause: Vec<(&'static str, usize)>,
    pub cause_unknown: usize,
    pub closed: usize,
    /// Jours entre la déclaration et la clôture, en moyenne, sur celles
    /// qui sont closes.
    pub mean_days_to_close: Option<i64>,
    pub actions: usize,
    pub actions_done: usize,
    /// Faites après leur échéance.
    pub actions_done_late: usize,
    /// Pas faites, échéance passée, à la date de la revue.
    pub actions_late: usize,
    pub preventive: usize,
    /// Déclarées au-dehors.
    pub declared_outside: usize,
}

/// L'année d'une date ISO.
fn year_of(iso: &str) -> Option<i32> {
    iso.get(..4)?.parse().ok()
}

pub fn review(year: i32, ncs: &[NonConformite], actions: &[Action], today: &str) -> Review {
    let mine: Vec<&NonConformite> = ncs
        .iter()
        .filter(|n| year_of(&n.declared_on) == Some(year))
        .collect();
    let mut r = Review {
        year,
        total: mine.len(),
        by_kind: KINDS.iter().map(|k| (*k, 0)).collect(),
        by_cause: CAUSES.iter().map(|k| (*k, 0)).collect(),
        ..Review::default()
    };
    let mut days = Vec::new();
    for nc in &mine {
        // Un type inconnu (écrit par une version plus récente) compte
        // dans « autre » : le total des types reste le total.
        let k = KINDS
            .iter()
            .position(|k| *k == nc.kind)
            .unwrap_or(KINDS.len() - 1);
        r.by_kind[k].1 += 1;
        match criticity(nc).map(Level::of) {
            Some(Level::Faible) => r.by_level[0] += 1,
            Some(Level::Moderee) => r.by_level[1] += 1,
            Some(Level::Elevee) => r.by_level[2] += 1,
            None => r.by_level[3] += 1,
        }
        match CAUSES.iter().position(|c| *c == nc.cause_kind) {
            Some(c) => r.by_cause[c].1 += 1,
            None => r.cause_unknown += 1,
        }
        if !nc.declaration.trim().is_empty() {
            r.declared_outside += 1;
        }
        if stage(nc, actions) == Stage::Close {
            r.closed += 1;
            if let (Some(a), Some(b)) = (
                crate::date::to_days(&nc.declared_on),
                crate::date::to_days(&nc.closed_on),
            ) {
                days.push((b - a).max(0));
            }
        }
        for a in actions_of(nc.id, actions) {
            r.actions += 1;
            if a.preventive {
                r.preventive += 1;
            }
            if done(a) {
                r.actions_done += 1;
                if !a.due_on.trim().is_empty() && a.done_on > a.due_on {
                    r.actions_done_late += 1;
                }
            } else if overdue(a, today) {
                r.actions_late += 1;
            }
        }
    }
    if !days.is_empty() {
        r.mean_days_to_close = Some(days.iter().sum::<i64>() / days.len() as i64);
    }
    r
}

/// Les années qui ont au moins une non-conformité, la plus récente
/// d'abord, et l'année en cours toujours.
pub fn years(ncs: &[NonConformite], today: &str) -> Vec<i32> {
    let mut out: Vec<i32> = ncs.iter().filter_map(|n| year_of(&n.declared_on)).collect();
    out.extend(year_of(today));
    out.sort_unstable_by(|a, b| b.cmp(a));
    out.dedup();
    out
}

/// **Un registre qualité de démonstration**, pour la base de
/// démonstration et la vue `qualite` des captures : une non-conformité
/// à chaque étape, datées autour de `today`, et une action en retard.
/// Les identifiants ne lient que les actions à leur non-conformité.
pub fn demo(today: &str) -> (Vec<NonConformite>, Vec<Action>) {
    let day = |back: i64| {
        crate::date::to_days(today)
            .map(|d| crate::date::from_days(d - back))
            .unwrap_or_else(|| today.to_owned())
    };
    let ncs = vec![
        NonConformite {
            id: 1,
            declared_on: day(1),
            declared_by: "MB".to_owned(),
            kind: "froid".to_owned(),
            title: "Réfrigérateur à 11 °C à l'ouverture".to_owned(),
            immediate: "Vaccins mis en quarantaine, laboratoire appelé.".to_owned(),
            ..NonConformite::default()
        },
        NonConformite {
            id: 2,
            declared_on: day(12),
            declared_by: "CL".to_owned(),
            kind: "dispensation".to_owned(),
            title: "Lévothyrox 100 µg délivré pour 75 µg".to_owned(),
            description: "Vu au contrôle de l'ordonnance, avant le départ du patient.".to_owned(),
            immediate: "Boîte reprise au comptoir, patient prévenu.".to_owned(),
            severity: 3,
            likelihood: 3,
            cause_kind: "materiel".to_owned(),
            cause: "Deux dosages rangés côte à côte dans le même tiroir.".to_owned(),
            ..NonConformite::default()
        },
        NonConformite {
            id: 3,
            declared_on: day(40),
            declared_by: "CL".to_owned(),
            kind: "fournisseur".to_owned(),
            title: "Colis du grossiste écrasé".to_owned(),
            severity: 1,
            likelihood: 2,
            cause_kind: "milieu".to_owned(),
            cause: "Transporteur.".to_owned(),
            effective: 1,
            verification: "Réclamation acceptée, avoir reçu.".to_owned(),
            closed_on: day(30),
            closed_by: "CL".to_owned(),
            ..NonConformite::default()
        },
    ];
    let actions = vec![
        Action {
            id: 1,
            nc_id: 2,
            what: "Séparer les dosages dans le tiroir".to_owned(),
            owner: "MB".to_owned(),
            due_on: day(5),
            done_on: day(6),
            ..Action::default()
        },
        Action {
            id: 2,
            nc_id: 2,
            preventive: true,
            what: "Étiquette de couleur par dosage".to_owned(),
            owner: "CL".to_owned(),
            due_on: day(2),
            ..Action::default()
        },
    ];
    (ncs, actions)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nc(id: i64) -> NonConformite {
        NonConformite {
            id,
            declared_on: "2026-03-02".to_owned(),
            kind: "dispensation".to_owned(),
            title: "Erreur de dosage".to_owned(),
            ..NonConformite::default()
        }
    }

    fn act(id: i64, nc_id: i64, due: &str, done: &str) -> Action {
        Action {
            id,
            nc_id,
            what: "Double contrôle".to_owned(),
            due_on: due.to_owned(),
            done_on: done.to_owned(),
            ..Action::default()
        }
    }

    #[test]
    fn the_stage_is_read_from_what_is_written() {
        let mut n = nc(1);
        assert_eq!(stage(&n, &[]), Stage::Declaree);
        n.severity = 3;
        n.likelihood = 2;
        assert_eq!(
            stage(&n, &[]),
            Stage::Declaree,
            "cotée, pas encore expliquée"
        );
        n.cause = "Deux dosages rangés côte à côte".to_owned();
        assert_eq!(stage(&n, &[]), Stage::Analysee);
        let mut acts = vec![act(1, 1, "2026-03-20", ""), act(2, 2, "", "")];
        assert_eq!(stage(&n, &acts), Stage::EnAction);
        acts[0].done_on = "2026-03-18".to_owned();
        assert_eq!(
            stage(&n, &acts),
            Stage::AVerifier,
            "l'action d'une autre ne compte pas"
        );
        n.closed_on = "2026-04-01".to_owned();
        assert_eq!(stage(&n, &acts), Stage::Close);
    }

    #[test]
    fn criticity_is_severity_by_likelihood_and_needs_both() {
        let mut n = nc(1);
        n.severity = 4;
        assert_eq!(criticity(&n), None);
        n.likelihood = 3;
        assert_eq!(criticity(&n), Some(12));
        assert_eq!(Level::of(12), Level::Elevee);
        assert_eq!(Level::of(3), Level::Faible);
        assert_eq!(Level::of(4), Level::Moderee);
        assert_eq!(Level::of(8), Level::Moderee);
        assert_eq!(Level::of(9), Level::Elevee);
        n.likelihood = 5;
        assert_eq!(criticity(&n), None, "hors de l'échelle : non cotée");
    }

    #[test]
    fn nothing_closes_before_it_is_understood_done_and_checked() {
        let mut n = nc(1);
        assert_eq!(cannot_close(&n, &[]), Some("qual_close_need_analysis"));
        n.severity = 1;
        n.likelihood = 1;
        n.cause = "Étiquette décollée".to_owned();
        // Sans action : la vérification écrite suffit.
        assert_eq!(cannot_close(&n, &[]), Some("qual_close_need_check"));
        n.verification = "Rien à corriger, isolé".to_owned();
        assert_eq!(cannot_close(&n, &[]), None);
        // Avec une action : faite, puis jugée efficace.
        let mut acts = vec![act(1, 1, "", "")];
        assert_eq!(cannot_close(&n, &acts), Some("qual_close_need_actions"));
        acts[0].done_on = "2026-03-10".to_owned();
        assert_eq!(cannot_close(&n, &acts), Some("qual_close_need_check"));
        n.effective = 2;
        assert_eq!(cannot_close(&n, &acts), Some("qual_close_not_effective"));
        n.effective = 1;
        assert_eq!(cannot_close(&n, &acts), None);
        n.closed_on = "2026-03-11".to_owned();
        assert_eq!(cannot_close(&n, &acts), Some("qual_close_already"));
    }

    #[test]
    fn the_board_counts_only_what_is_open() {
        let mut a = nc(1);
        a.severity = 4;
        a.likelihood = 4;
        let mut b = nc(2);
        b.closed_on = "2026-03-05".to_owned();
        let acts = vec![
            act(1, 1, "2026-03-01", ""),
            act(2, 1, "2026-12-01", ""),
            act(3, 2, "2026-01-01", ""),
        ];
        assert_eq!(
            board(&[a, b], &acts, "2026-03-10"),
            Board {
                open: 1,
                high: 1,
                late: 1,
                to_check: 0
            }
        );
    }

    #[test]
    fn the_yearly_review_adds_up() {
        let mut a = nc(1);
        a.severity = 2;
        a.likelihood = 2;
        a.cause_kind = "methode".to_owned();
        a.cause = "x".to_owned();
        a.closed_on = "2026-03-12".to_owned();
        a.declaration = "Pharmacovigilance, 03/03/2026".to_owned();
        let mut b = nc(2);
        b.kind = "inconnu_d_une_version_future".to_owned();
        let mut old = nc(3);
        old.declared_on = "2025-12-30".to_owned();
        let acts = vec![
            act(1, 1, "2026-03-05", "2026-03-08"),
            Action {
                preventive: true,
                ..act(2, 1, "2026-03-20", "2026-03-10")
            },
            act(3, 2, "2026-04-01", ""),
            act(4, 3, "2026-01-01", ""),
        ];
        let r = review(2026, &[a, b, old], &acts, "2026-05-01");
        assert_eq!(r.total, 2);
        assert_eq!(r.by_kind.iter().map(|(_, n)| n).sum::<usize>(), 2);
        assert_eq!(r.by_kind[0], ("dispensation", 1));
        assert_eq!(r.by_kind[KINDS.len() - 1], ("autre", 1));
        assert_eq!(r.by_level, [0, 1, 0, 1]);
        assert_eq!(r.by_cause[0], ("methode", 1));
        assert_eq!(r.cause_unknown, 1);
        assert_eq!(r.closed, 1);
        assert_eq!(r.mean_days_to_close, Some(10));
        assert_eq!(
            (
                r.actions,
                r.actions_done,
                r.actions_done_late,
                r.actions_late
            ),
            (3, 2, 1, 1)
        );
        assert_eq!(r.preventive, 1);
        assert_eq!(r.declared_outside, 1);
        assert_eq!(years(&[nc(1)], "2027-01-02"), vec![2027, 2026]);
    }
}
