//! Les postes de l'équipe : qui est là, quand, combien d'heures cela
//! fait, et à quelles tranches le comptoir est vide.
//!
//! « Poste » veut dire ici l'horaire d'une personne, pas le poste
//! informatique. La **garde** en est une nature et non un écran de
//! plus : c'est un poste qui commence le soir et qui compte à part. Les
//! **absences** aussi en sont une — congé, maladie, formation,
//! récupération : une absence est un poste qui ne porte pas d'heures, et
//! c'est précisément ce qui *explique* un creux au comptoir. La ranger
//! ailleurs serait séparer le trou de sa raison.
//!
//! Il ne voit ni egui ni la base : il prend des postes et rend des
//! minutes, des creux et des paires. Le chevauchement, lui, n'est pas
//! recalculé ici — [`crate::agenda`] le sait déjà, et **il n'y a pas
//! deux calculs de chevauchement dans cette application** : celui des
//! rendez-vous et celui des postes sont le même.
//!
//! Cinq règles le tiennent, une par test, et ce sont elles le contenu :
//!
//! * **Les heures se comptent en minutes entières**, jamais en heures
//!   décimales. Sept heures trente-cinq est `455`, jamais `7.583333` :
//!   la même discipline qu'aux centimes de la caisse, et pour la même
//!   raison — c'est en repassant par un flottant qu'une somme juste
//!   redevient fausse, et cinq minutes par jour font deux heures dans
//!   le mois. La conversion en « 7 h 35 » se fait à l'affichage.
//! * **Un poste sans fin n'est pas un poste de zéro heure.**
//!   [`Shift::minutes`] rend `None`, [`day_total`] rend `None`, et la
//!   ligne affiche « — ». C'est la règle des durées d'entretien, celle
//!   de l'écart de caisse sans recette attendue, et celle du
//!   rendez-vous sans durée : on ne fabrique pas un chiffre pour
//!   remplir une case.
//! * **Une nuit est comptée au jour qui la commence.** Une garde de
//!   20 h à 2 h se range `1200 → 1560` — les minutes dépassent 1 440 —
//!   et ses six heures vont **en entier** au jour de début. Partagée
//!   entre deux jours, elle serait comptée deux fois par qui additionne
//!   les colonnes.
//! * **Deux postes de la même personne qui se touchent ne sont pas un
//!   conflit.** 9 h – 12 h 30 et 14 h – 19 h 30 sont une journée coupée
//!   à midi, pas une erreur ; ils ne le deviennent qu'en partageant une
//!   minute. La même borne que les rendez-vous, puisque c'est le même
//!   `overlaps`.
//! * **Une pause plus longue que le poste est refusée, pas
//!   soustraite.** Une durée négative dessinée est un bloc à l'envers,
//!   et un total négatif se propage dans la semaine sans se voir.
//!
//! Et une sixième, qui n'est pas dans le code mais dans le nom des
//! choses : [`day_total`] compte une **présence**, pas une paie. Le
//! module ne connaît ni majoration, ni heure supplémentaire, ni
//! convention collective, et il n'en connaîtra pas. Ce qu'on déduit
//! d'un total d'heures est du droit du travail, il change, et une
//! application de pharmacie qui imprimerait « dont 2 h majorées » se
//! tromperait un jour sans que personne le voie. La même retenue que
//! `vigilance.rs` : une question, jamais un verdict.
//!
//! Pur, testé, sans horloge : le jour est passé.

use crate::agenda::{overlaps, Slot};

/// Ce qu'un poste est. La nature décide de deux choses et de rien
/// d'autre : si le poste porte des heures, et s'il met quelqu'un
/// **au comptoir**.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ShiftKind {
    /// Ouverture : le premier du matin.
    Ouverture,
    /// La journée ordinaire.
    Journee,
    /// Fermeture : le dernier du soir.
    Fermeture,
    /// La garde, qui commence le soir et déborde sur la nuit.
    Garde,
    /// L'astreinte : joignable, pas présent.
    Astreinte,
    Conge,
    Maladie,
    Formation,
    Recup,
}

impl ShiftKind {
    pub const ALL: [ShiftKind; 9] = [
        Self::Ouverture,
        Self::Journee,
        Self::Fermeture,
        Self::Garde,
        Self::Astreinte,
        Self::Conge,
        Self::Maladie,
        Self::Formation,
        Self::Recup,
    ];

    /// La clé écrite en base. Elle part au disque et se relit dans dix
    /// ans, donc elle existe — au contraire de `timeline::Kind`, qui
    /// n'est qu'une lecture et n'en a pas.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ouverture => "OUVERTURE",
            Self::Journee => "JOURNEE",
            Self::Fermeture => "FERMETURE",
            Self::Garde => "GARDE",
            Self::Astreinte => "ASTREINTE",
            Self::Conge => "CONGE",
            Self::Maladie => "MALADIE",
            Self::Formation => "FORMATION",
            Self::Recup => "RECUP",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.as_str() == s)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Ouverture => "Ouverture",
            Self::Journee => "Journée",
            Self::Fermeture => "Fermeture",
            Self::Garde => "Garde",
            Self::Astreinte => "Astreinte",
            Self::Conge => "Congé",
            Self::Maladie => "Maladie",
            Self::Formation => "Formation",
            Self::Recup => "Récupération",
        }
    }

    /// Le poste porte des heures de présence.
    ///
    /// Une absence n'en porte pas, **même munie d'un horaire** : « congé
    /// de 9 h à 19 h » est une manière d'écrire une journée entière
    /// d'absence, pas dix heures de présence. La formation est rangée
    /// avec les absences parce que c'est une absence *du comptoir* ;
    /// savoir si elle se paie est une question de convention
    /// collective, et ce module n'en connaît aucune.
    pub fn worked(self) -> bool {
        matches!(
            self,
            Self::Ouverture | Self::Journee | Self::Fermeture | Self::Garde | Self::Astreinte
        )
    }

    /// Le poste met quelqu'un **au comptoir**.
    ///
    /// L'astreinte porte des heures et ne compte pas ici : joignable au
    /// téléphone n'est pas devant le patient, et une bande de couverture
    /// qui compterait les astreintes annoncerait un comptoir tenu là où
    /// il ne l'est pas.
    pub fn at_counter(self) -> bool {
        matches!(
            self,
            Self::Ouverture | Self::Journee | Self::Fermeture | Self::Garde
        )
    }

    /// Le poste est une absence : il explique un creux au lieu de le
    /// combler.
    pub fn is_absence(self) -> bool {
        !self.worked()
    }
}

/// L'horaire d'une personne, un jour donné.
///
/// `end` vaut `None` quand personne ne l'a écrit — et c'est un fait, pas
/// une case à remplir d'office. Les minutes peuvent dépasser 1 440 : une
/// garde de 20 h à 2 h se range `1200 → 1560`, en un seul intervalle, au
/// jour qui la commence.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Shift {
    pub id: i64,
    /// Les initiales, telles que `[pharmacy] operators` les écrit.
    pub operator: String,
    /// Minutes depuis minuit.
    pub start: u16,
    pub end: Option<u16>,
    /// La coupure du midi, en minutes, déduite du total.
    pub pause: u16,
    pub kind: ShiftKind,
}

impl Shift {
    /// Les minutes de présence, pause déduite — ou `None`.
    ///
    /// `None` dans trois cas, et aucun ne vaut zéro : le poste n'a pas
    /// de fin écrite, le poste est une absence, ou la pause est plus
    /// longue que le poste. Ce dernier est **refusé et non soustrait** :
    /// une durée négative se propage dans le total de la semaine sans
    /// se voir.
    pub fn minutes(&self) -> Option<u16> {
        if !self.kind.worked() {
            return None;
        }
        let end = self.end?;
        let span = end.checked_sub(self.start)?;
        (span > self.pause).then(|| span - self.pause)
    }

    /// L'intervalle occupé, pause comprise — ce qui se dessine, et ce
    /// sur quoi les chevauchements se lisent.
    ///
    /// Une pause ne libère pas le créneau : quelqu'un dont la coupure
    /// est de midi à quatorze heures n'est pas disponible à midi et
    /// demi pour un autre poste. Un poste sans fin est un point, comme
    /// un rendez-vous sans durée.
    pub fn slot(&self) -> Slot {
        match self.end {
            Some(end) => Slot::new(self.start, end),
            None => Slot::point(self.start),
        }
    }
}

/// Le total d'un jour pour une personne, ou `None`.
///
/// `None` et non `0` dans tous les cas où le chiffre n'est pas connu :
/// la personne n'a pas de poste ce jour-là (**un jour sans trame ne vaut
/// pas zéro heure, il ne vaut rien**), elle n'a que des absences, ou
/// l'un de ses postes n'a pas de fin. Le graphique saute la barre au
/// lieu de la poser au sol, et la ligne affiche « — ».
pub fn day_total(shifts: &[Shift], who: &str) -> Option<u16> {
    let mine: Vec<&Shift> = shifts.iter().filter(|s| s.operator == who).collect();
    if mine.is_empty() {
        return None;
    }
    let worked: Vec<&&Shift> = mine.iter().filter(|s| s.kind.worked()).collect();
    if worked.is_empty() {
        return None;
    }
    // Un seul poste sans total rend le total du jour inconnu : la somme
    // des autres se lirait comme la journée entière.
    worked
        .iter()
        .try_fold(0_u16, |acc, s| Some(acc + s.minutes()?))
}

/// Le total d'une semaine par personne.
///
/// `unknown` est la moitié qui manque partout ailleurs : le nombre de
/// postes dont la durée n'est pas connue. « 34 h 15 » sans lui se lit
/// comme la semaine entière, alors qu'il en manque deux jours — la même
/// règle que l'écart cumulé de la caisse, qui dit sur combien de soirs
/// il porte.
///
/// Une nuit n'est comptée qu'une fois parce qu'elle n'est écrite qu'une
/// fois : elle appartient au jour qui la commence, et c'est l'appelant
/// qui l'y range.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct WeekTotal {
    pub who: String,
    /// Les minutes connues, et elles seules.
    pub minutes: u16,
    /// Les postes travaillés dont la durée n'est pas connue.
    pub unknown: usize,
}

/// Les totaux de la semaine, une ligne par personne, dans l'ordre des
/// initiales — un ordre stable, sans quoi les lignes du tableau
/// changeraient de place d'une image à l'autre.
pub fn week_totals(days: &[(String, Vec<Shift>)]) -> Vec<WeekTotal> {
    let mut out: Vec<WeekTotal> = Vec::new();
    for (_day, shifts) in days {
        for s in shifts.iter().filter(|s| s.kind.worked()) {
            let row = match out.iter_mut().find(|r| r.who == s.operator) {
                Some(r) => r,
                None => {
                    out.push(WeekTotal {
                        who: s.operator.clone(),
                        minutes: 0,
                        unknown: 0,
                    });
                    out.last_mut().expect("just pushed")
                }
            };
            match s.minutes() {
                Some(m) => row.minutes = row.minutes.saturating_add(m),
                None => row.unknown += 1,
            }
        }
    }
    out.sort_by(|a, b| a.who.cmp(&b.who));
    out
}

/// Combien de personnes sont au comptoir, tranche par tranche, de
/// `from` à `to` par pas de `step`.
///
/// Compte des têtes, pas des manques : la bande se dessine même sans
/// horaires d'ouverture déclarés. Ce qui a besoin des horaires, c'est
/// le rouge — voir [`gaps`].
pub fn coverage(shifts: &[Shift], from: u16, to: u16, step: u16) -> Vec<u8> {
    if step == 0 || to <= from {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(usize::from((to - from) / step) + 1);
    let mut at = from;
    while at < to {
        let slice = Slot::new(at, (at + step).min(to));
        let n = shifts
            .iter()
            .filter(|s| s.kind.at_counter() && overlaps(&s.slot(), &slice))
            .count();
        out.push(u8::try_from(n).unwrap_or(u8::MAX));
        at += step;
    }
    out
}

/// Les tranches où l'officine est ouverte et où personne n'est inscrit.
///
/// **Sans horaires écrits, il n'y a pas de creux** : `opening` vide rend
/// une liste vide, et non la journée entière. Une officine qui n'a rien
/// déclaré ne se fait pas dire tous les matins qu'elle n'ouvre pas —
/// c'est la même règle qu'à la caisse, où sans recette attendue il n'y a
/// pas d'écart.
///
/// Les tranches contiguës sont fondues : trois quarts d'heure de suite
/// sans personne sont **un** creux de quarante-cinq minutes, et non
/// trois lignes qu'il faut recoller de l'œil.
pub fn gaps(shifts: &[Shift], opening: &[Slot]) -> Vec<Slot> {
    let mut out: Vec<Slot> = Vec::new();
    for open in opening {
        if open.is_point() {
            continue;
        }
        let mut run: Option<(u16, u16)> = None;
        for minute in open.start..open.end {
            let covered = shifts
                .iter()
                .any(|s| s.kind.at_counter() && overlaps(&s.slot(), &Slot::point(minute)));
            match (&mut run, covered) {
                (Some(r), false) => r.1 = minute + 1,
                (Some(_), true) => {
                    if let Some((a, b)) = run.take() {
                        out.push(Slot::new(a, b));
                    }
                }
                (None, false) => run = Some((minute, minute + 1)),
                (None, true) => {}
            }
        }
        if let Some((a, b)) = run {
            out.push(Slot::new(a, b));
        }
    }
    out
}

/// Les paires de postes d'une **même personne** qui partagent une
/// minute, par identifiants, le plus petit d'abord.
///
/// Deux personnes au même créneau ne sont pas un conflit — c'est une
/// officine qui tourne. Deux postes de la même personne à la même
/// minute en sont un : elle ne peut pas les tenir tous les deux. Et
/// deux postes qui se *touchent* n'en sont pas un : 9 h – 12 h 30 puis
/// 14 h – 19 h 30 est une journée coupée à midi.
pub fn double_booked(shifts: &[Shift]) -> Vec<(i64, i64)> {
    let mut out = Vec::new();
    for (i, a) in shifts.iter().enumerate() {
        for b in shifts.iter().skip(i + 1) {
            if a.operator == b.operator && overlaps(&a.slot(), &b.slot()) {
                out.push((a.id.min(b.id), a.id.max(b.id)));
            }
        }
    }
    out.sort_unstable();
    out.dedup();
    out
}

/// L'heure d'une borne de poste, `HH:MM`, **jusqu'à 47:59**.
///
/// [`crate::agenda::minutes_of`] s'arrête à 23:59, et c'est juste pour
/// elle : c'est l'horloge d'une journée, et un rendez-vous à 26 h n'a
/// pas de sens. Une borne de poste, si — une garde de 20 h à 2 h se
/// range `1200 → 1560`, en un seul intervalle, au jour qui la commence,
/// et c'est la règle même de ce module. Lue avec l'horloge du jour,
/// cette fin-là devenait illisible : la garde perdait sa durée et le
/// total du mercredi valait trois heures et demie au lieu de neuf et
/// demie, sans que rien ne le dise.
pub fn parse_bound(hhmm: &str) -> Option<u16> {
    let (h, m) = hhmm.split_once(':')?;
    let (h, m) = (h.trim().parse::<u16>().ok()?, m.trim().parse::<u16>().ok()?);
    (h < 48 && m < 60).then_some(h * 60 + m)
}

/// La borne inverse : `1560` → « 26:00 », ce qui est ce que la base
/// range. Écrite en regard de [`parse_bound`] pour que l'aller et le
/// retour restent l'un sous les yeux de l'autre.
pub fn format_bound(minutes: u16) -> String {
    format!("{:02}:{:02}", minutes / 60, minutes % 60)
}

/// Une durée écrite à la main : « 35h00 », « 35 h », « 7h35 », « 455 ».
///
/// Le contrat d'une personne se tape dans `config.toml`, à la main,
/// par quelqu'un qui écrit ce qui lui vient. Un analyseur qui n'accepte
/// qu'une forme est un analyseur qui rend `None` sur la seule forme
/// qu'on ait envie d'écrire.
///
/// Un nombre nu est un **nombre de minutes** et non d'heures : c'est
/// l'unité de tout le module, et « 35 » voulant dire trente-cinq heures
/// ferait de « 455 » sept cent cinquante-huit heures.
pub fn parse_duration(text: &str) -> Option<u16> {
    let t = text.trim().to_lowercase().replace(',', ".");
    if t.is_empty() {
        return None;
    }
    // « 35h00 », « 35 h », « 7 h 35 ».
    if let Some((h, m)) = t.split_once('h') {
        let hours: u16 = h.trim().parse().ok()?;
        let m = m.trim();
        let mins: u16 = if m.is_empty() { 0 } else { m.parse().ok()? };
        if mins > 59 {
            return None;
        }
        return hours.checked_mul(60)?.checked_add(mins);
    }
    t.parse().ok()
}

/// Les plages d'ouverture d'un jour, lues sur ce que l'officine a
/// écrit — et **rien du tout quand elle n'a rien écrit**.
///
/// `weekday` est le nom français du jour, tel que `db::weekday_fr` le
/// rend ; la comparaison ignore la casse et les espaces, parce que la
/// ligne est tapée à la main.
///
/// Les plages illisibles ou à l'envers sont laissées de côté plutôt que
/// redressées : une plage « de 19 h à 9 h » corrigée en silence
/// annoncerait dix heures d'ouverture que personne n'a demandées.
pub fn opening_slots<'a>(
    horaires: impl IntoIterator<Item = (&'a str, &'a str, &'a str)>,
    weekday: &str,
) -> Vec<Slot> {
    let want = weekday.trim().to_lowercase();
    let mut out: Vec<Slot> = horaires
        .into_iter()
        .filter(|(jour, _, _)| jour.trim().to_lowercase() == want)
        .filter_map(|(_, de, a)| {
            let (from, to) = (
                crate::agenda::minutes_of(de)?,
                crate::agenda::minutes_of(a)?,
            );
            (to > from).then(|| Slot::new(from, to))
        })
        .collect();
    out.sort_by_key(|s| (s.start, s.end));
    out
}

/// « 7 h 35 » : des minutes entières rendues lisibles, à l'affichage et
/// nulle part ailleurs.
///
/// Une nuit qui déborde reste lisible : `1560` s'écrit « 26 h 00 » au
/// total d'un jour, ce qui est la vérité de ce qui a été fait ce
/// jour-là.
pub fn hhmm(minutes: u16) -> String {
    format!("{} h {:02}", minutes / 60, minutes % 60)
}

/// « 7 h 35 » ou « — ». Le tiret est la seule chose qu'on écrive d'une
/// durée qu'on ne connaît pas.
pub fn hhmm_or_dash(minutes: Option<u16>) -> String {
    minutes.map_or_else(|| "—".to_owned(), hhmm)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shift(id: i64, who: &str, start: u16, end: Option<u16>, kind: ShiftKind) -> Shift {
        Shift {
            id,
            operator: who.to_owned(),
            start,
            end,
            pause: 0,
            kind,
        }
    }

    /// La raison d'être du module, et la même qu'aux centimes de la
    /// caisse : sept heures trente-cinq est quatre cent cinquante-cinq
    /// minutes, exactement. En heures décimales c'est `7.583333…`, et
    /// cinq jours de cela ne font pas trente-sept heures cinquante-cinq.
    #[test]
    fn hours_are_counted_in_whole_minutes() {
        // 9 h 00 – 18 h 05, une heure et demie de coupure : 7 h 35.
        let s = shift(1, "CL", 9 * 60, Some(18 * 60 + 5), ShiftKind::Journee);
        let s = Shift { pause: 90, ..s };
        assert_eq!(s.minutes(), Some(455));
        assert_eq!(hhmm(455), "7 h 35");
        // Cinq jours pareils tombent juste, ce qu'une somme de
        // flottants ne garantit pas.
        let week: Vec<(String, Vec<Shift>)> = (0..5)
            .map(|d| (format!("2026-09-{:02}", 7 + d), vec![s.clone()]))
            .collect();
        assert_eq!(week_totals(&week)[0].minutes, 455 * 5);
        assert_eq!(hhmm(455 * 5), "37 h 55");
    }

    /// **Un poste sans fin n'est pas un poste de zéro heure.** Ni la
    /// ligne, ni le total du jour, ni la semaine ne le comptent pour
    /// rien : ils disent qu'ils ne savent pas.
    #[test]
    fn a_shift_without_an_end_has_no_total() {
        let s = shift(1, "CL", 9 * 60, None, ShiftKind::Journee);
        assert_eq!(s.minutes(), None);
        assert_eq!(day_total(std::slice::from_ref(&s), "CL"), None);
        assert_eq!(hhmm_or_dash(None), "—");
        // Et il rend le total du jour inconnu même à côté d'un poste
        // qui, lui, est complet : additionner l'autre seul se lirait
        // comme la journée entière.
        let full = shift(2, "CL", 14 * 60, Some(19 * 60), ShiftKind::Journee);
        assert_eq!(day_total(&[s.clone(), full.clone()], "CL"), None);
        assert_eq!(day_total(std::slice::from_ref(&full), "CL"), Some(5 * 60));
        // Dans la semaine, il est compté comme inconnu et nommé.
        let w = week_totals(&[("2026-09-07".to_owned(), vec![s, full])]);
        assert_eq!(w[0].minutes, 5 * 60);
        assert_eq!(w[0].unknown, 1);
    }

    /// **Un jour sans trame ne vaut pas zéro heure, il ne vaut rien** —
    /// et une journée d'absence non plus. Le graphique saute la barre
    /// au lieu de la poser au sol.
    #[test]
    fn a_day_with_no_shift_is_not_a_day_of_zero_hours() {
        assert_eq!(day_total(&[], "CL"), None);
        // Quelqu'un d'autre est là, pas elle.
        let other = shift(1, "YS", 9 * 60, Some(19 * 60), ShiftKind::Journee);
        assert_eq!(day_total(&[other], "CL"), None);
        // Et un congé **muni d'un horaire** ne porte pas dix heures :
        // c'est une manière d'écrire une journée d'absence.
        let conge = shift(2, "CL", 9 * 60, Some(19 * 60), ShiftKind::Conge);
        assert_eq!(conge.minutes(), None);
        assert_eq!(day_total(&[conge], "CL"), None);
    }

    /// **Une nuit est comptée au jour qui la commence.** Une garde de
    /// 20 h à 2 h se range `1200 → 1560`, ses six heures vont en entier
    /// au jour de début, et la semaine ne les voit qu'une fois.
    #[test]
    fn a_night_shift_is_counted_once() {
        let garde = shift(1, "CL", 20 * 60, Some(26 * 60), ShiftKind::Garde);
        assert_eq!(garde.minutes(), Some(6 * 60));
        assert_eq!(day_total(std::slice::from_ref(&garde), "CL"), Some(360));
        // Le lendemain porte sa propre journée et **pas** la nuit :
        // partagée entre les deux, elle serait comptée deux fois par
        // qui additionne les colonnes.
        let lendemain = shift(2, "CL", 14 * 60, Some(19 * 60), ShiftKind::Journee);
        let week = [
            ("2026-09-07".to_owned(), vec![garde]),
            ("2026-09-08".to_owned(), vec![lendemain]),
        ];
        assert_eq!(week_totals(&week)[0].minutes, 6 * 60 + 5 * 60);
    }

    /// **Deux postes de la même personne qui se touchent ne sont pas un
    /// conflit** : 9 h – 12 h 30 et 14 h – 19 h 30 sont une journée
    /// coupée à midi. Ils ne le deviennent qu'en partageant une minute.
    #[test]
    fn two_shifts_that_touch_are_not_a_conflict() {
        let matin = shift(1, "CL", 9 * 60, Some(12 * 60 + 30), ShiftKind::Ouverture);
        let aprem = shift(2, "CL", 14 * 60, Some(19 * 60 + 30), ShiftKind::Fermeture);
        assert!(double_booked(&[matin.clone(), aprem.clone()]).is_empty());
        // Bord à bord non plus.
        let colle = shift(3, "CL", 12 * 60 + 30, Some(14 * 60), ShiftKind::Journee);
        assert!(double_booked(&[matin.clone(), colle]).is_empty());
        // Une minute partagée, si.
        let chevauche = shift(4, "CL", 12 * 60, Some(14 * 60), ShiftKind::Journee);
        assert_eq!(double_booked(&[matin.clone(), chevauche.clone()]), [(1, 4)]);
        // Et deux **personnes** au même créneau ne sont pas un conflit :
        // c'est une officine qui tourne.
        let autre = Shift {
            operator: "YS".to_owned(),
            ..chevauche
        };
        assert!(double_booked(&[matin, autre]).is_empty());
    }

    /// **Une pause plus longue que le poste est refusée, pas
    /// soustraite.** Une durée négative dessinée est un bloc à
    /// l'envers, et un total négatif se propage dans la semaine sans se
    /// voir.
    #[test]
    fn a_pause_longer_than_the_shift_is_refused() {
        let s = Shift {
            pause: 3 * 60,
            ..shift(1, "CL", 9 * 60, Some(11 * 60), ShiftKind::Journee)
        };
        assert_eq!(s.minutes(), None);
        assert_eq!(day_total(&[s], "CL"), None);
        // Égale au poste, elle ne laisse rien non plus — et zéro minute
        // de présence n'est pas une présence.
        let pile = Shift {
            pause: 2 * 60,
            ..shift(2, "CL", 9 * 60, Some(11 * 60), ShiftKind::Journee)
        };
        assert_eq!(pile.minutes(), None);
        // Une minute de moins, et le poste existe.
        let juste = Shift {
            pause: 2 * 60 - 1,
            ..shift(3, "CL", 9 * 60, Some(11 * 60), ShiftKind::Journee)
        };
        assert_eq!(juste.minutes(), Some(1));
    }

    /// **Sans horaires d'ouverture écrits, il n'y a pas de creux** — et
    /// surtout pas la journée entière. Une officine qui n'a rien déclaré
    /// ne se fait pas dire tous les matins qu'elle n'ouvre pas.
    #[test]
    fn without_opening_hours_there_is_no_gap() {
        let s = shift(1, "CL", 9 * 60, Some(12 * 60), ShiftKind::Journee);
        assert!(gaps(std::slice::from_ref(&s), &[]).is_empty());
        // Déclarés, le trou de l'après-midi se voit — **d'un seul
        // tenant**, et non en quarts d'heure à recoller de l'œil.
        let ouverture = [Slot::new(9 * 60, 12 * 60 + 30)];
        assert_eq!(gaps(&[s], &ouverture), [Slot::new(12 * 60, 12 * 60 + 30)]);
    }

    /// La bande de couverture compte des têtes, et l'astreinte n'en est
    /// pas une : joignable au téléphone n'est pas devant le patient.
    #[test]
    fn coverage_counts_who_is_at_the_counter() {
        let cl = shift(1, "CL", 9 * 60, Some(11 * 60), ShiftKind::Ouverture);
        let ys = shift(2, "YS", 10 * 60, Some(12 * 60), ShiftKind::Journee);
        let astreinte = shift(3, "AB", 9 * 60, Some(12 * 60), ShiftKind::Astreinte);
        let conge = shift(4, "MD", 9 * 60, Some(12 * 60), ShiftKind::Conge);
        let band = coverage(&[cl, ys, astreinte.clone(), conge], 9 * 60, 12 * 60, 30);
        // 9 h – 10 h : CL seule. 10 h – 11 h : les deux. 11 h – 12 h :
        // YS seule. Ni l'astreinte ni le congé n'y sont.
        assert_eq!(band, vec![1, 1, 2, 2, 1, 1]);
        // L'astreinte porte quand même ses heures : elle ne compte pas
        // au comptoir, elle n'est pas pour autant une absence.
        assert_eq!(astreinte.minutes(), Some(3 * 60));
        assert!(!astreinte.kind.is_absence());
        // Une bande demandée à l'envers, ou d'un pas nul, ne panique pas.
        assert!(coverage(&[], 12 * 60, 9 * 60, 30).is_empty());
        assert!(coverage(&[], 9 * 60, 12 * 60, 0).is_empty());
    }

    /// **Une garde franchit minuit, et sa fin doit rester lisible.**
    /// L'horloge du jour s'arrête à 23:59 ; une borne de poste va
    /// jusqu'à 47:59, sans quoi 20 h → 2 h perd sa durée et le total du
    /// jour se trompe de six heures sans le dire.
    #[test]
    fn a_shift_bound_may_run_past_midnight() {
        assert_eq!(parse_bound("26:00"), Some(26 * 60));
        assert_eq!(parse_bound("02:00"), Some(2 * 60));
        assert_eq!(parse_bound("47:59"), Some(47 * 60 + 59));
        assert_eq!(parse_bound("48:00"), None);
        assert_eq!(parse_bound("26:60"), None);
        assert_eq!(parse_bound(""), None);
        // Aller et retour, sur la forme que la base range.
        assert_eq!(format_bound(26 * 60), "26:00");
        assert_eq!(format_bound(9 * 60 + 30), "09:30");
        for m in [0_u16, 9 * 60 + 5, 23 * 60 + 59, 26 * 60, 47 * 60 + 59] {
            assert_eq!(parse_bound(&format_bound(m)), Some(m));
        }
        // Et la garde qu'on lit ainsi porte bien ses six heures.
        let garde = Shift {
            id: 1,
            operator: "YS".to_owned(),
            start: parse_bound("20:00").unwrap(),
            end: parse_bound("26:00"),
            pause: 0,
            kind: ShiftKind::Garde,
        };
        assert_eq!(garde.minutes(), Some(6 * 60));
    }

    /// Le contrat se tape à la main dans `config.toml` : l'analyseur
    /// accepte ce qu'on a envie d'écrire, et **rien** quand la case est
    /// vide — sans contrat écrit, pas d'écart au contrat.
    #[test]
    fn a_contract_is_read_in_the_shapes_people_write_it() {
        assert_eq!(parse_duration("35h00"), Some(35 * 60));
        assert_eq!(parse_duration("35 h"), Some(35 * 60));
        assert_eq!(parse_duration(" 7 h 35 "), Some(455));
        assert_eq!(parse_duration("35H30"), Some(35 * 60 + 30));
        // Un nombre nu est un nombre de **minutes**, l'unité du module.
        assert_eq!(parse_duration("455"), Some(455));
        // Rien écrit, rien à en tirer : surtout pas zéro, qui se
        // lirait « contrat de zéro heure ».
        assert_eq!(parse_duration(""), None);
        assert_eq!(parse_duration("   "), None);
        assert_eq!(parse_duration("plein temps"), None);
        assert_eq!(parse_duration("7h75"), None);
    }

    /// Les plages d'ouverture se lisent sur ce que l'officine a écrit,
    /// et une plage à l'envers est laissée de côté plutôt que
    /// redressée : « de 19 h à 9 h » corrigée en silence annoncerait
    /// dix heures d'ouverture que personne n'a demandées.
    #[test]
    fn opening_hours_are_read_as_written() {
        let horaires = [
            ("lundi", "09:00", "12:30"),
            ("Lundi ", "14:00", "19:30"),
            ("mardi", "09:00", "19:30"),
            ("lundi", "19:00", "09:00"),
            ("lundi", "neuf heures", "midi"),
        ];
        assert_eq!(
            opening_slots(horaires, "lundi"),
            [
                Slot::new(9 * 60, 12 * 60 + 30),
                Slot::new(14 * 60, 19 * 60 + 30)
            ]
        );
        // Rien d'écrit pour le dimanche : pas de plage, donc pas de
        // creux — et non « fermé toute la journée » en rouge.
        assert!(opening_slots(horaires, "dimanche").is_empty());
        assert!(gaps(&[], &opening_slots(horaires, "dimanche")).is_empty());
    }

    /// Les bords des deux fonctions qui lisent une journée : une plage
    /// d'ouverture réduite à un point n'est pas une ouverture, et une
    /// personne présente toute la journée ne laisse aucun creux.
    #[test]
    fn the_edges_of_a_day_do_not_invent_anything() {
        let plein = shift(1, "CL", 9 * 60, Some(19 * 60), ShiftKind::Journee);
        // Une plage sans durée ne borne rien : on ne va pas déclarer un
        // creux d'une minute parce qu'une ligne d'horaires est fautive.
        assert!(gaps(std::slice::from_ref(&plein), &[Slot::point(9 * 60)]).is_empty());
        // Couverte de bout en bout, la journée n'a pas de creux.
        assert!(gaps(std::slice::from_ref(&plein), &[Slot::new(9 * 60, 19 * 60)]).is_empty());
        // Un creux au début **et** un à la fin font deux entrées, pas
        // une qui les recolle par-dessus la présence.
        assert_eq!(
            gaps(std::slice::from_ref(&plein), &[Slot::new(8 * 60, 20 * 60)]),
            [Slot::new(8 * 60, 9 * 60), Slot::new(19 * 60, 20 * 60)]
        );
        // Une absence ne couvre rien : c'est tout le creux.
        let conge = shift(2, "CL", 9 * 60, Some(19 * 60), ShiftKind::Conge);
        assert_eq!(
            gaps(std::slice::from_ref(&conge), &[Slot::new(9 * 60, 10 * 60)]),
            [Slot::new(9 * 60, 10 * 60)]
        );
        // Et la semaine d'une équipe vide est une liste vide, pas une
        // ligne à zéro heure.
        assert!(week_totals(&[]).is_empty());
        assert!(week_totals(&[("2026-09-07".to_owned(), Vec::new())]).is_empty());
    }

    /// Toute nature écrite en base se relit, et le libellé n'est jamais
    /// la clé : renommer « Récupération » ne doit pas rendre illisibles
    /// les postes de l'an dernier.
    #[test]
    fn every_kind_survives_a_trip_through_the_base() {
        for k in ShiftKind::ALL {
            assert_eq!(ShiftKind::parse(k.as_str()), Some(k));
            assert!(!k.label().is_empty());
            assert!(k.worked() || k.is_absence());
            // Seul un poste travaillé peut mettre quelqu'un au comptoir.
            assert!(!k.at_counter() || k.worked());
        }
        assert_eq!(ShiftKind::parse("PLAGE"), None);
    }
}
