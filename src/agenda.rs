//! Les intervalles de la journée : ce qui se chevauche, dans quelle
//! voie cela se dessine, et ce qu'un filtre a le droit d'éteindre.
//!
//! L'agenda ne savait pas ce qu'est un chevauchement. Le plan de
//! journée plaçait ses blocs par **seau d'heure** — `offset / row_h`,
//! puis `index % 2` — si bien que 9 h 00 et 9 h 45 étaient dessinées
//! côte à côte alors qu'elles ne se rencontrent jamais, et que la
//! troisième entrée de la même heure repeignait la première. Un
//! rendez-vous était toujours haut d'une ligne, quelle que soit sa
//! durée.
//!
//! Tant que cela reste vrai, un filtre « au comptoir / à distance » est
//! dangereux : cacher l'entretien à distance de 14 h 00 – 14 h 30 alors
//! qu'une vaccination est posée à 14 h 15 au comptoir **fabrique** le
//! conflit qu'il devait montrer. Ce n'est pas une personne dans une
//! liste, c'est une personne qui ne peut pas être aux deux endroits.
//!
//! Trois règles tiennent ce module :
//!
//! * **Les bornes qui se touchent ne se chevauchent pas.** Un bloc qui
//!   finit à 9 h 30 et un rendez-vous à 9 h 30 se suivent : la salle est
//!   libre. Un intervalle couvre donc `[début, fin[`, jamais la minute
//!   de sa fin.
//! * **Un rendez-vous sans durée n'est pas un rendez-vous de zéro
//!   minute** — c'est un point dans la journée. Il garde sa ligne à
//!   l'écran et ne chevauche que ce qui couvre effectivement sa minute
//!   (ou ce qui commence exactement avec lui). On n'invente pas une
//!   durée moyenne pour le dessiner plus grand : ce serait un conflit
//!   fabriqué par le logiciel, et c'est la même retenue qu'aux durées
//!   d'entretien et à l'écart de caisse sans recette attendue.
//! * **Un filtre n'efface pas, il éteint — et il n'éteint pas ce qui
//!   chevauche ce qu'il garde.** [`states`] est cette règle, et c'est
//!   elle la fonction : voir [`State`].
//!
//! Le module ne connaît **ni egui, ni la base, ni les natures d'acte**.
//! Il prend des intervalles et rend des voies, des paires et des états ;
//! savoir si un entretien pharmaceutique à distance passe le filtre est
//! l'affaire de la vue, qui pose le `kept` de chaque entrée. C'est la
//! même frontière que `timeline.rs` — celui-là ne sait pas où vivent les
//! vaccinations, celui-ci ne sait pas ce qu'est un TROD — et elle vaut
//! pour la même raison : le jour où les postes de l'équipe se
//! chevaucheront eux aussi, il n'y aura rien à changer ici.
//!
//! Pur, testé, sans horloge : les minutes sont passées.

/// Un intervalle de la journée, en minutes depuis minuit.
///
/// `end == start` est un **point** : une entrée dont personne n'a noté
/// la durée. Les minutes peuvent dépasser 1 440 — une garde qui
/// commence à 20 h et finit à 2 h se range `1200 → 1560` et reste un
/// seul intervalle, au jour qui la commence.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Slot {
    pub start: u16,
    pub end: u16,
}

impl Slot {
    /// Un intervalle sain : une fin antérieure au début serait un bloc
    /// dessiné à l'envers, et un chevauchement calculé sur des bornes
    /// croisées répond n'importe quoi sans le dire.
    pub fn new(start: u16, end: u16) -> Slot {
        Slot {
            start,
            end: end.max(start),
        }
    }

    /// Une entrée sans durée connue.
    pub fn point(start: u16) -> Slot {
        Slot { start, end: start }
    }

    /// Personne n'a noté la durée.
    pub fn is_point(&self) -> bool {
        self.end == self.start
    }

    /// La durée, ou `None` pour un point.
    ///
    /// `None` et non `0` : « — » se lit, « 0 min » se croit.
    pub fn minutes(&self) -> Option<u16> {
        (!self.is_point()).then(|| self.end - self.start)
    }

    /// La **dernière minute couverte**. Un bloc `[9 h 00, 9 h 30[`
    /// couvre jusqu'à 9 h 29 ; un point ne couvre que la sienne.
    ///
    /// C'est ce décalage d'une minute qui fait que deux blocs bord à
    /// bord ne se chevauchent pas et qu'un point posé sur le début d'un
    /// bloc, lui, s'y trouve.
    fn last(&self) -> u16 {
        if self.is_point() {
            self.start
        } else {
            self.end - 1
        }
    }
}

/// Deux intervalles partagent-ils une minute ?
///
/// La règle des bornes : `[9 h 00, 9 h 30[` et `[9 h 30, 10 h 00[` se
/// suivent, ils ne se chevauchent pas.
pub fn overlaps(a: &Slot, b: &Slot) -> bool {
    a.start.max(b.start) <= a.last().min(b.last())
}

/// Une entrée de la journée telle que le calcul la voit : une place
/// dans le temps, un identifiant pour la nommer, et le verdict du
/// filtre.
///
/// `kept` est **posé par la vue** : c'est elle qui sait si « à
/// distance » est coché et si la nature de l'acte est dans la liste. Ce
/// module ne fait qu'en tirer les conséquences.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Entry {
    pub id: i64,
    pub slot: Slot,
    /// L'entrée passe le filtre.
    pub kept: bool,
}

impl Entry {
    pub fn new(id: i64, slot: Slot, kept: bool) -> Entry {
        Entry { id, slot, kept }
    }
}

/// Ce qu'un filtre fait d'une entrée. Trois états, dans cet ordre de
/// priorité — et c'est la fonction elle-même :
///
/// Aucune entrée ne disparaît de la journée. C'est ce qui rend le
/// filtre utilisable au comptoir : on filtre pour **lire**, pas pour
/// décider, et la décision (« puis-je prendre quelqu'un à 14 h 15 ? »)
/// a précisément besoin de ce qui est éteint.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum State {
    /// Passe le filtre : couleur pleine.
    Retenu,
    /// Ne passe pas le filtre, mais chevauche au moins un retenu :
    /// dessiné à sa place, dans sa voie, en aplat éteint. On lit qu'il
    /// est là et qu'il n'est pas le sujet.
    Contexte,
    /// Ne passe pas et ne chevauche rien de retenu : pas de bloc, un
    /// trait à la minute où il commence. La journée garde sa densité
    /// vraie ; le survol le nomme.
    Ecarte,
}

impl State {
    /// L'entrée occupe une voie et se dessine comme un bloc.
    ///
    /// Un écarté n'en prend pas : il n'est plus qu'un trait contre la
    /// gouttière des heures, et lui laisser une colonne rétrécirait la
    /// journée pour rien.
    pub fn drawn(&self) -> bool {
        !matches!(self, State::Ecarte)
    }
}

/// Le sort de chaque entrée sous le filtre, dans l'ordre reçu.
///
/// L'entrée est écartée d'elle-même **par sa position et non par son
/// identifiant** : un identifiant se répète (une entrée du calendrier
/// qui se répète porte le même sur chacun de ses jours), et une entrée
/// qui se sauverait elle-même ne serait jamais écartée.
pub fn states(entries: &[Entry]) -> Vec<State> {
    entries
        .iter()
        .enumerate()
        .map(|(i, e)| {
            if e.kept {
                State::Retenu
            } else if entries
                .iter()
                .enumerate()
                .any(|(j, o)| o.kept && j != i && overlaps(&e.slot, &o.slot))
            {
                State::Contexte
            } else {
                State::Ecarte
            }
        })
        .collect()
}

/// La voie de chaque entrée, dans l'ordre reçu.
///
/// Un coloriage glouton du graphe d'intervalles : on trie par début, on
/// donne à chaque entrée la plus petite voie qu'aucune de ses voisines
/// déjà placées n'occupe. Sur un graphe d'intervalles ce glouton-là est
/// optimal : le nombre de voies d'une journée est sa clique maximale —
/// le plus grand nombre d'entrées qui partagent une même minute — et
/// non deux colonnes en dur.
///
/// L'ordre de parcours est **total** (début, puis fin, puis
/// identifiant) : deux entrées de même heure qui échangeraient leurs
/// voies d'une image à l'autre feraient sauter la journée sous les
/// yeux.
pub fn lanes(entries: &[Entry]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..entries.len()).collect();
    order.sort_by_key(|&i| {
        let e = &entries[i];
        (e.slot.start, e.slot.end, e.id)
    });
    let mut out = vec![0_usize; entries.len()];
    let mut placed: Vec<usize> = Vec::with_capacity(entries.len());
    for i in order {
        let mut lane = 0;
        // La plus petite voie libre : on remonte tant qu'une voisine
        // déjà posée l'occupe. Les voisines sont peu nombreuses (une
        // journée d'officine, pas un aéroport), donc la boucle carrée
        // est ici la bonne.
        loop {
            let taken = placed
                .iter()
                .any(|&j| out[j] == lane && overlaps(&entries[i].slot, &entries[j].slot));
            if !taken {
                break;
            }
            lane += 1;
        }
        out[i] = lane;
        placed.push(i);
    }
    out
}

/// Le nombre de voies qu'une journée demande — au moins une, même vide,
/// puisqu'un bloc se dessine dans quelque chose.
pub fn lane_count(lanes: &[usize]) -> usize {
    lanes.iter().copied().max().map_or(1, |m| m + 1)
}

/// Les paires d'entrées qui partagent une minute, par identifiants, le
/// plus petit d'abord et chaque paire une seule fois.
///
/// Un conflit **n'est pas une erreur** : une officine en prend, à deux
/// personnes. C'est pourquoi cela se rend en liste et se dessine en
/// liseré, jamais en refus ni en boîte de dialogue.
pub fn conflicts(entries: &[Entry]) -> Vec<(i64, i64)> {
    let mut out = Vec::new();
    for (i, a) in entries.iter().enumerate() {
        for b in entries.iter().skip(i + 1) {
            if overlaps(&a.slot, &b.slot) {
                out.push((a.id.min(b.id), a.id.max(b.id)));
            }
        }
    }
    out.sort_unstable();
    out.dedup();
    out
}

/// Les identifiants pris dans au moins un chevauchement.
pub fn in_conflict(entries: &[Entry]) -> std::collections::HashSet<i64> {
    conflicts(entries)
        .into_iter()
        .flat_map(|(a, b)| [a, b])
        .collect()
}

/// « 09:30 » en minutes depuis minuit.
///
/// Sur une heure vide ou illisible : `None`. Pas de repli à minuit —
/// un rendez-vous dont l'heure n'a pas été saisie se rangerait en tête
/// de journée et se lirait comme un rendez-vous de 0 h 00.
pub fn minutes_of(hhmm: &str) -> Option<u16> {
    let (h, m) = hhmm.split_once(':')?;
    let (h, m) = (h.parse::<u16>().ok()?, m.parse::<u16>().ok()?);
    (h < 24 && m < 60).then_some(h * 60 + m)
}

/// L'intervalle d'une entrée à partir de son heure et de ce qu'on sait
/// de sa durée : un bloc quand la durée est écrite, un point sinon.
///
/// `duration` vaut `0` sur tous les actes déjà pris — la colonne est
/// arrivée après eux —, et **zéro n'est pas une durée**.
pub fn slot_of(time: &str, duration: i64) -> Option<Slot> {
    let start = minutes_of(time)?;
    let span = u16::try_from(duration.max(0)).unwrap_or(u16::MAX);
    Some(Slot::new(start, start.saturating_add(span)))
}

/// Le même calcul depuis deux heures écrites — celui du calendrier, où
/// la fin est une heure et non une durée.
pub fn slot_between(time: &str, end_time: &str) -> Option<Slot> {
    let start = minutes_of(time)?;
    Some(match minutes_of(end_time) {
        Some(end) => Slot::new(start, end),
        None => Slot::point(start),
    })
}

/// La plage d'heures qu'un plan de journée doit couvrir : celle qu'on
/// lui demande, **élargie jusqu'aux entrées qui en sortent**.
///
/// Les heures d'ouverture décident de la plage ordinaire, et c'est bien
/// ainsi : un plan qui va de 8 à 20 se lit d'un coup d'œil. Mais une
/// entrée posée en dehors n'en est pas moins une entrée, et elle
/// tombait dans la liste « Sans heure » — c'est-à-dire qu'un
/// rendez-vous de 20 h 30, dont l'heure est écrite noir sur blanc,
/// s'affichait comme un rendez-vous dont personne n'avait noté l'heure.
/// On étend le plan jusqu'à lui plutôt que de le renommer.
///
/// Les bornes sont des **heures**, début inclus et fin exclue, comme la
/// fenêtre que le dessin teste. D'où les deux élargissements : l'heure
/// de début doit être *dans* la plage — une entrée à 20 h 00 demande
/// donc 21 —, et un bloc doit y tenir jusqu'à sa fin, à l'heure
/// entamée. Une garde qui franchit minuit s'arrête à 24 : ses minutes
/// dépassent 1 440, mais un plan de journée est d'un jour.
pub fn span(wanted: (u16, u16), entries: &[Entry]) -> (u16, u16) {
    let from0 = wanted.0.min(23);
    let (mut from, mut to) = (from0, wanted.1.clamp(from0 + 1, 24));
    for entry in entries {
        let starts = (entry.slot.start / 60).min(23);
        from = from.min(starts);
        to = to.max(starts + 1);
        if entry.slot.minutes().is_some() {
            to = to.max(entry.slot.end.div_ceil(60));
        }
        to = to.min(24);
    }
    (from, to.clamp(from + 1, 24))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(id: i64, start: u16, end: u16) -> Entry {
        Entry::new(id, Slot::new(start, end), true)
    }

    /// **Une entrée hors des heures d'ouverture a quand même une
    /// heure.** Le plan s'étend jusqu'à elle ; il ne la renomme pas
    /// « sans heure », ce qui était le cas et se lisait comme un
    /// rendez-vous que personne n'avait daté.
    #[test]
    fn the_plan_stretches_to_the_entries_that_fall_outside_it() {
        // Rien à placer : la plage demandée, telle quelle.
        assert_eq!(span((8, 20), &[]), (8, 20));
        // Tout est dedans : elle ne bouge pas non plus.
        assert_eq!(span((8, 20), &[at(1, 9 * 60, 10 * 60)]), (8, 20));
        // Un rendez-vous de 7 h 30 tire le début vers le bas.
        assert_eq!(span((8, 20), &[at(1, 7 * 60 + 30, 8 * 60)]), (7, 20));
        // **La borne haute est exclue** : une entrée à 20 h 00 pile
        // n'est pas dans une plage qui s'arrête à 20, donc il en faut
        // 21. C'est la même heure que le dessin teste.
        assert_eq!(
            span((8, 20), &[Entry::new(1, Slot::point(20 * 60), true)]),
            (8, 21)
        );
        // Un point à 20 h 30, et un bloc qui finit à 19 h 45 : l'heure
        // entamée, pas celle qui est finie.
        assert_eq!(
            span((8, 20), &[Entry::new(1, Slot::point(20 * 60 + 30), true)]),
            (8, 21)
        );
        assert_eq!(span((8, 19), &[at(1, 18 * 60, 19 * 60 + 45)]), (8, 20));
        // Une garde de 20 h à 2 h se range `1200 → 1560` : le plan
        // s'arrête à 24, parce qu'il est d'un jour.
        assert_eq!(span((8, 20), &[at(1, 1200, 1560)]), (8, 24));
        // Une plage demandée à l'envers ou vide reste lisible : au
        // moins une heure, jamais une hauteur nulle à diviser.
        assert_eq!(span((20, 8), &[]), (20, 21));
        assert_eq!(span((23, 23), &[]), (23, 24));
    }

    /// Le défaut d'aujourd'hui, et la raison d'être du module : 9 h 00
    /// et 9 h 45 étaient dessinées côte à côte parce qu'elles tombent
    /// dans le même seau d'heure. Elles ne se rencontrent pas : une
    /// voie chacune, la première.
    #[test]
    fn two_entries_in_the_same_hour_do_not_share_a_lane() {
        let day = [at(1, 9 * 60, 9 * 60 + 30), at(2, 9 * 60 + 45, 10 * 60 + 15)];
        assert!(!overlaps(&day[0].slot, &day[1].slot));
        assert_eq!(lanes(&day), vec![0, 0]);
        assert_eq!(lane_count(&lanes(&day)), 1);
        assert!(conflicts(&day).is_empty());
    }

    /// Trois à la même heure font trois voies, et aucune n'est
    /// repeinte. L'ancien `index % 2` en perdait une sur trois.
    #[test]
    fn three_at_the_same_hour_take_three_lanes() {
        let day = [
            at(1, 9 * 60, 10 * 60),
            at(2, 9 * 60, 10 * 60),
            at(3, 9 * 60, 10 * 60),
        ];
        let l = lanes(&day);
        assert_eq!(lane_count(&l), 3);
        let mut seen = l.clone();
        seen.sort_unstable();
        assert_eq!(seen, vec![0, 1, 2]);
        assert_eq!(conflicts(&day), vec![(1, 2), (1, 3), (2, 3)]);
    }

    /// Un point est dans le bloc qui commence avec lui : la personne
    /// est attendue à 14 h 15 et quelqu'un occupe déjà la place.
    #[test]
    fn a_point_inside_a_block_overlaps_it() {
        let block = Slot::new(9 * 60, 9 * 60 + 30);
        assert!(overlaps(&Slot::point(9 * 60), &block));
        assert!(overlaps(&Slot::point(9 * 60 + 15), &block));
        assert!(overlaps(&Slot::point(9 * 60 + 29), &block));
        // Et deux points de la même minute se chevauchent : c'est le
        // seul chevauchement qu'un point sans durée puisse avoir avec
        // un autre point.
        assert!(overlaps(&Slot::point(9 * 60), &Slot::point(9 * 60)));
        assert!(!overlaps(&Slot::point(9 * 60), &Slot::point(9 * 60 + 1)));
    }

    /// Les bornes se touchent, la salle est libre : un point à 9 h 30
    /// contre un bloc qui *finit* à 9 h 30 n'est pas un chevauchement,
    /// et deux blocs bord à bord non plus.
    #[test]
    fn touching_bounds_do_not_overlap() {
        let block = Slot::new(9 * 60, 9 * 60 + 30);
        assert!(!overlaps(&Slot::point(9 * 60 + 30), &block));
        assert!(!overlaps(&Slot::new(9 * 60 + 30, 10 * 60), &block));
        // Et une minute plus tôt, si.
        assert!(overlaps(&Slot::new(9 * 60 + 29, 10 * 60), &block));
    }

    /// Une journée vide rend un vecteur vide plutôt que de paniquer, et
    /// se dessine quand même dans une voie.
    #[test]
    fn an_empty_day_is_not_a_panic() {
        let day: [Entry; 0] = [];
        assert!(lanes(&day).is_empty());
        assert!(conflicts(&day).is_empty());
        assert!(states(&day).is_empty());
        assert_eq!(lane_count(&[]), 1);
    }

    /// La règle du filtre, et c'est elle la fonction : l'entretien à
    /// distance de 14 h 00 – 14 h 30 est éteint, la vaccination de
    /// 14 h 15 au comptoir est retenue — donc l'entretien reste
    /// **dessiné**, en contexte. Le filtre qui l'effacerait fabriquerait
    /// le conflit qu'il devait montrer.
    #[test]
    fn a_filter_does_not_hide_what_overlaps_what_it_keeps() {
        let day = [
            Entry::new(1, Slot::new(14 * 60, 14 * 60 + 30), false),
            Entry::new(2, Slot::point(14 * 60 + 15), true),
            // Et loin de là, un troisième que rien ne retient.
            Entry::new(3, Slot::new(17 * 60, 17 * 60 + 30), false),
        ];
        assert_eq!(
            states(&day),
            vec![State::Contexte, State::Retenu, State::Ecarte]
        );
        assert!(states(&day)[0].drawn());
        assert!(!states(&day)[2].drawn());
    }

    /// Deux éteints qui se chevauchent entre eux ne se sauvent pas
    /// l'un l'autre : le contexte est celui de ce qui est **retenu**.
    #[test]
    fn two_filtered_entries_do_not_keep_each_other() {
        let day = [
            Entry::new(1, Slot::new(14 * 60, 14 * 60 + 30), false),
            Entry::new(2, Slot::new(14 * 60 + 10, 14 * 60 + 40), false),
        ];
        assert_eq!(states(&day), vec![State::Ecarte, State::Ecarte]);
    }

    /// Un point ne prend une voie à personne : il n'écarte pas ce qui
    /// se passe ailleurs dans la journée.
    #[test]
    fn a_point_does_not_widen_the_day() {
        let day = [
            at(1, 9 * 60, 12 * 60),
            Entry::new(2, Slot::point(15 * 60), true),
        ];
        assert_eq!(lane_count(&lanes(&day)), 1);
    }

    /// Une durée écrite fait un bloc, une durée nulle un point — et
    /// zéro minute n'est pas une durée.
    #[test]
    fn a_zero_duration_is_a_point_and_not_a_block() {
        let point = slot_of("09:30", 0).unwrap();
        assert!(point.is_point());
        assert_eq!(point.minutes(), None);
        let block = slot_of("09:30", 45).unwrap();
        assert_eq!(block.minutes(), Some(45));
        assert_eq!(block.end, 10 * 60 + 15);
        // Sans heure, pas d'intervalle du tout : surtout pas minuit.
        assert!(slot_of("", 30).is_none());
        assert!(minutes_of("").is_none());
        assert!(minutes_of("24:00").is_none());
        assert_eq!(minutes_of("00:00"), Some(0));
        assert_eq!(minutes_of("23:59"), Some(23 * 60 + 59));
    }

    /// La fin lue comme une heure — celle du calendrier —, et l'absence
    /// de fin qui redonne un point.
    #[test]
    fn an_event_without_an_end_is_a_point() {
        assert_eq!(
            slot_between("09:00", "10:30"),
            Some(Slot::new(9 * 60, 10 * 60 + 30))
        );
        assert_eq!(slot_between("09:00", ""), Some(Slot::point(9 * 60)));
        // Une fin antérieure au début ne fait pas un bloc à l'envers.
        assert_eq!(slot_between("10:00", "09:00"), Some(Slot::point(10 * 60)));
    }

    /// L'ordre des voies est total : la même journée, donnée dans un
    /// autre ordre, rend les mêmes voies aux mêmes entrées.
    #[test]
    fn the_lane_order_is_total() {
        let a = at(7, 9 * 60, 10 * 60);
        let b = at(3, 9 * 60, 10 * 60);
        let one = lanes(&[a, b]);
        let two = lanes(&[b, a]);
        assert_eq!(one, vec![1, 0]);
        assert_eq!(two, vec![0, 1]);
    }
}
