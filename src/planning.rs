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
//! Six règles le tiennent, une par test, et ce sont elles le contenu :
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
//! * **« Les semaines paires » n'est pas « une semaine sur deux ».**
//!   [`Cadence`] porte les deux et refuse de les confondre : l'une se
//!   lit sur le calendrier, l'autre se compte depuis le jour où on l'a
//!   posée. Elles coïncident presque toujours, et divergent pour
//!   toujours au premier passage d'une année ISO de 53 semaines — le
//!   31 décembre 2026 est en semaine 53, le 4 janvier 2027 en semaine
//!   1, deux impaires de suite. Une officine qui travaille « les
//!   semaines paires » et à qui on aurait écrit quatorze jours se
//!   retrouverait à contretemps un lundi de janvier, sans que rien ne
//!   le dise.
//!
//! Et une sixième, qui n'est pas dans le code mais dans le nom des
//! choses : [`day_total`] compte une **présence**, pas une paie. Le
//! module ne connaît ni majoration, ni heure supplémentaire, ni
//! convention collective, ni horaire contractuel, et il n'en connaîtra
//! pas. Ce qu'on déduit d'un total d'heures est du droit du travail, il
//! change, et une application de pharmacie qui imprimerait « dont 2 h
//! majorées » — ou « −3 h 15 sur le contrat » — se tromperait un jour
//! sans que personne le voie. La même retenue que `vigilance.rs` : une
//! question, jamais un verdict.
//!
//! Pur, testé, sans horloge : le jour est passé.

use crate::agenda::{overlaps, Slot};

/// À quel rythme une trame revient.
///
/// Une officine ne travaille pas « tous les mercredis » et rien
/// d'autre : on est là **les semaines paires**, une semaine sur trois
/// au dépôt, un samedi sur deux. Écrire cela à la main, c'est poser
/// vingt-six lignes par an et par personne, et se tromper d'une.
///
/// Les huit rythmes se rangent en deux familles, et **elles ne se
/// confondent pas** :
///
/// * celles qui se lisent sur le **calendrier** — [`Cadence::Paires`],
///   [`Cadence::Impaires`] : le numéro de semaine ISO décide, et le
///   jour où la trame a été posée n'y change rien ;
/// * celles qui se comptent **depuis le jour posé** —
///   [`Cadence::UneSurDeux`] et ses sœurs : quinze jours après ce
///   mercredi-là, quoi que dise le calendrier. [`Cadence::Quotidien`]
///   en fait partie, et c'est par lui qu'une plage s'écrit : « congé du
///   12 au 26 » est une ligne rangée bornée par sa date de fin, et non
///   quinze lignes.
///
/// La différence a l'air d'un détail et n'en est pas un. Une année ISO
/// compte 52 ou 53 semaines ; au premier passage d'une année de 53, une
/// trame écrite tous les quatorze jours cesse pour toujours de tomber
/// sur les semaines paires. C'est la raison d'être de cette énumération
/// plutôt que d'un simple nombre de jours dans la base.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Cadence {
    /// Ce jour-là, et pas un autre.
    Unique,
    /// Tous les jours, jusqu'à une date écrite.
    ///
    /// C'est ainsi qu'une absence se pose : « congé du 12 au 26
    /// octobre » est **une** ligne rangée et non quinze. Et c'est le
    /// seul rythme qui exige une fin — sans elle, ce n'est pas une
    /// absence, c'est quelqu'un d'absent pour toujours.
    Quotidien,
    /// Toutes les semaines.
    Hebdomadaire,
    /// Les semaines dont le numéro ISO est pair.
    Paires,
    /// Les semaines dont le numéro ISO est impair.
    Impaires,
    /// Une semaine sur deux, à compter du jour posé.
    UneSurDeux,
    /// Une semaine sur trois, à compter du jour posé.
    UneSurTrois,
    /// Une semaine sur quatre, à compter du jour posé.
    UneSurQuatre,
}

impl Cadence {
    pub const ALL: [Cadence; 8] = [
        Self::Unique,
        Self::Quotidien,
        Self::Hebdomadaire,
        Self::Paires,
        Self::Impaires,
        Self::UneSurDeux,
        Self::UneSurTrois,
        Self::UneSurQuatre,
    ];

    /// La clé écrite en base — elle part au disque et se relit dans dix
    /// ans, comme celle de [`ShiftKind`].
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unique => "UNIQUE",
            Self::Quotidien => "QUOTIDIEN",
            Self::Hebdomadaire => "HEBDO",
            Self::Paires => "PAIRES",
            Self::Impaires => "IMPAIRES",
            Self::UneSurDeux => "SUR2",
            Self::UneSurTrois => "SUR3",
            Self::UneSurQuatre => "SUR4",
        }
    }

    /// Relire une clé — et **`None` sur ce qu'on ne connaît pas**, y
    /// compris sur la chaîne vide.
    ///
    /// La chaîne vide est le cas ordinaire et non un défaut : c'est ce
    /// que portent les lignes écrites avant que ce type n'existe, et
    /// leur rythme est dans `repeat_days`. Rendre `Unique` par défaut
    /// effacerait une trame hebdomadaire de l'an dernier ; rendre
    /// `None` laisse la base lire ce qu'elle a toujours lu.
    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.as_str() == s)
    }

    /// Le libellé du menu. Écrit ici, avec le type, comme celui de
    /// [`ShiftKind`] : c'est le vocabulaire du module.
    pub fn label(self) -> &'static str {
        match self {
            Self::Unique => "Ce jour-là",
            Self::Quotidien => "Tous les jours",
            Self::Hebdomadaire => "Chaque semaine",
            Self::Paires => "Semaines paires",
            Self::Impaires => "Semaines impaires",
            Self::UneSurDeux => "Une semaine sur deux",
            Self::UneSurTrois => "Une semaine sur trois",
            Self::UneSurQuatre => "Une semaine sur quatre",
        }
    }

    /// Ce que le rythme veut dire, en une phrase — ce qui se lit au
    /// survol, et qui dit surtout **d'où il compte**.
    pub fn hint(self) -> &'static str {
        match self {
            Self::Unique => "Un seul jour. Rien ne revient.",
            Self::Quotidien => {
                "Tous les jours jusqu'à la date de fin, qui devient obligatoire. \
                 C'est ainsi qu'on pose un congé : une ligne, et non quinze."
            }
            Self::Hebdomadaire => "Une ligne rangée, dépliée à la lecture — pas cinquante-deux.",
            Self::Paires => {
                "Les semaines dont le numéro ISO est pair. C'est le calendrier qui décide, \
                 et non un cycle de quinze jours : une année de 53 semaines retourne la parité."
            }
            Self::Impaires => {
                "Les semaines dont le numéro ISO est impair. C'est le calendrier qui décide, \
                 et non un cycle de quinze jours : une année de 53 semaines retourne la parité."
            }
            Self::UneSurDeux => {
                "Quinze jours après le jour posé, puis de quinze en quinze. \
                 Ce n'est pas « les semaines paires » : le calendrier n'y entre pas."
            }
            Self::UneSurTrois => "Trois semaines après le jour posé, puis de trois en trois.",
            Self::UneSurQuatre => "Quatre semaines après le jour posé, puis de quatre en quatre.",
        }
    }

    /// De combien de jours on avance d'une occurrence à la suivante.
    ///
    /// Les deux rythmes de parité avancent de **sept** jours et non de
    /// quatorze : c'est [`Cadence::accepte`] qui écarte une semaine sur
    /// deux, et c'est exactement ce qui les distingue de
    /// [`Cadence::UneSurDeux`]. Les écrire à quatorze jours donnerait le
    /// bon résultat pendant cinq ans, puis le mauvais pour toujours.
    pub fn pas(self) -> i64 {
        match self {
            Self::Unique => 0,
            Self::Quotidien => 1,
            Self::Hebdomadaire | Self::Paires | Self::Impaires => 7,
            Self::UneSurDeux => 14,
            Self::UneSurTrois => 21,
            Self::UneSurQuatre => 28,
        }
    }

    /// Ce jour-là tombe-t-il sous ce rythme ?
    ///
    /// Vrai partout sauf pour les deux parités, qui lisent le numéro de
    /// semaine ISO. Une date illisible n'est acceptée par aucune des
    /// deux : on ne fait pas tomber une occurrence sur un jour dont on
    /// ne sait pas dans quelle semaine il est.
    pub fn accepte(self, day: &str) -> bool {
        let parity = match self {
            Self::Paires => 0,
            Self::Impaires => 1,
            _ => return true,
        };
        crate::date::iso_week(day).is_some_and(|(_, w)| w % 2 == parity)
    }

    /// Le rythme exige-t-il une date de fin ?
    ///
    /// Un seul : le quotidien. Les autres tournent indéfiniment sans
    /// que ce soit une faute — un horaire de travail n'a pas de date
    /// d'expiration —, là où « tous les jours, sans fin » n'est pas un
    /// congé mais quelqu'un d'absent pour toujours, et un an de lignes
    /// dépliées à chaque lecture.
    pub fn needs_an_end(self) -> bool {
        self == Self::Quotidien
    }

    /// L'autre moitié d'une alternance : paires ↔ impaires, et rien
    /// pour les autres.
    ///
    /// C'est aussi la façon de demander « ce rythme se lit-il sur le
    /// calendrier ? » — il n'y a pas de seconde fonction pour cela :
    /// deux manières de poser une question finissent par y répondre
    /// différemment.
    pub fn other_half(self) -> Option<Self> {
        match self {
            Self::Paires => Some(Self::Impaires),
            Self::Impaires => Some(Self::Paires),
            _ => None,
        }
    }
}

/// Les `count` premiers jours où une trame posée le `anchor` tombe.
///
/// **Le jour posé n'est pas toujours le premier.** Régler « semaines
/// paires » un mercredi de semaine impaire ne pose rien ce mercredi-là :
/// la première occurrence est huit jours plus tard, et c'est précisément
/// ce que l'écran doit montrer avant qu'on valide. Une liste qui
/// commencerait par le jour choisi mentirait sur la moitié des cas.
///
/// Rien du tout quand la date ne se lit pas, et **jamais une boucle
/// sans fin** : la recherche s'arrête après un nombre d'essais borné,
/// ce qui suffit très largement — une parité écarte au plus une semaine
/// sur deux.
pub fn occurrences(cadence: Cadence, anchor: &str, count: usize) -> Vec<String> {
    let mut out = Vec::with_capacity(count);
    if crate::date::to_days(anchor).is_none() {
        return out;
    }
    let mut day = anchor.to_owned();
    let pas = cadence.pas();
    for _ in 0..(count.saturating_mul(4) + 8) {
        if out.len() == count {
            break;
        }
        if cadence.accepte(&day) {
            out.push(day.clone());
        }
        if pas == 0 {
            break;
        }
        match crate::date::add_days(&day, pas) {
            Some(next) => day = next,
            None => break,
        }
    }
    out
}

/// Le `weekday` (lundi = 1 … dimanche = 7) **de la semaine où tombe
/// `from`** — en arrière comme en avant.
///
/// C'est par là qu'une trame se pose, et le sens compte. « Le premier
/// mercredi à partir d'ici » aurait l'air d'être la même chose et ne
/// l'est pas : réglée depuis un mercredi, la trame poserait son mercredi
/// le jour même et son **lundi la semaine suivante**. Les sept journées
/// d'une semaine qu'on remplit d'un coup ne seraient plus une semaine —
/// et pour « une semaine sur deux », qui compte depuis le jour posé, les
/// deux moitiés partiraient à contretemps l'une de l'autre, pour
/// toujours.
///
/// La semaine est donc celle que `from` **nomme**, quel que soit le jour
/// de cette semaine-là qu'on lui donne.
pub fn in_week_of(from: &str, weekday: i64) -> Option<String> {
    let current = crate::date::weekday(from)?;
    crate::date::add_days(from, weekday - current)
}

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
///
/// **Une pause n'a pas d'heure**, et c'est la limite à connaître ici :
/// [`Shift::pause`] est une durée, pas un créneau, si bien qu'un poste
/// de 9 h à 19 h avec quatre-vingt-dix minutes de coupure compte au
/// comptoir *toute* la journée. Une officine dont tout le monde déjeune
/// de midi et demi à deux s'annoncerait donc tenue. La façon d'écrire
/// une journée coupée est **deux postes** — 9 h – 12 h 30 puis 14 h –
/// 19 h 30 —, que la grille sait montrer dans une case et que la
/// fenêtre de trame sait poser : eux disent où est le trou.
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

    /// **« Les semaines paires » n'est pas « une semaine sur deux ».**
    /// La règle du module, et la raison pour laquelle la base range un
    /// rythme et non un nombre de jours.
    ///
    /// Les deux marchent du même pas tant que l'année ISO compte 52
    /// semaines. 2026 en compte 53 : le 30 décembre 2026 est en semaine
    /// 53, le 6 janvier 2027 en semaine 1 — deux impaires de suite. La
    /// parité saute donc une semaine à cet endroit-là, et le cycle de
    /// quatorze jours, lui, continue tout droit et se retrouve à
    /// contretemps **pour toujours**.
    #[test]
    fn even_weeks_are_not_a_fortnight() {
        // Un mercredi de semaine paire : le 16 septembre 2026, semaine
        // 38.
        let depart = "2026-09-16";
        assert_eq!(crate::date::iso_week(depart), Some((2026, 38)));
        let paires = occurrences(Cadence::Paires, depart, 10);
        let quinzaine = occurrences(Cadence::UneSurDeux, depart, 10);
        // Tant qu'on reste dans l'année, les deux disent la même chose,
        // et c'est ce qui rend la confusion si facile à faire.
        assert_eq!(paires[..8], quinzaine[..8]);
        assert_eq!(paires[7], "2026-12-23");
        // Puis 2026 finit sur une semaine 53, et elles se séparent —
        // à la neuvième occurrence, six mois plus tard, quand plus
        // personne ne relit la trame qu'il a posée.
        assert_eq!(paires[8], "2027-01-13");
        assert_eq!(quinzaine[8], "2027-01-06");
        // Et pour toujours : la parité est sur le calendrier, le cycle
        // sur le jour posé.
        let loin = occurrences(Cadence::Paires, depart, 20);
        let cycle = occurrences(Cadence::UneSurDeux, depart, 20);
        assert_ne!(loin[19], cycle[19]);
        for d in &loin {
            assert_eq!(crate::date::iso_week(d).map(|(_, w)| w % 2), Some(0), "{d}");
        }
    }

    /// **Le jour posé n'est pas toujours la première occurrence.**
    /// Régler « semaines paires » pendant une semaine impaire ne pose
    /// rien cette semaine-là, et l'écran doit le dire avant qu'on
    /// valide plutôt que de laisser croire que cela commence ce jour-ci.
    #[test]
    fn a_rhythm_may_not_start_on_the_day_it_is_set() {
        // Le 9 septembre 2026 est en semaine 37, impaire.
        let impaire = "2026-09-09";
        assert_eq!(crate::date::iso_week(impaire), Some((2026, 37)));
        assert_eq!(
            occurrences(Cadence::Paires, impaire, 2),
            ["2026-09-16", "2026-09-30"]
        );
        // Le même jour sous « impaires » commence bien ce jour-là.
        assert_eq!(
            occurrences(Cadence::Impaires, impaire, 2),
            ["2026-09-09", "2026-09-23"]
        );
        // « Ce jour-là » ne rend qu'un jour, quoi qu'on lui demande.
        assert_eq!(occurrences(Cadence::Unique, impaire, 5), [impaire]);
        // Et une date qu'on ne lit pas ne rend rien — surtout pas une
        // boucle.
        assert!(occurrences(Cadence::Hebdomadaire, "la semaine prochaine", 5).is_empty());
        assert!(occurrences(Cadence::Paires, "", 5).is_empty());
    }

    /// Les deux parités avancent de **sept** jours et se filtrent ;
    /// c'est ce qui les sépare d'un cycle de quatorze, et rien d'autre
    /// dans le module n'en décide.
    #[test]
    fn every_rhythm_survives_a_trip_through_the_base() {
        for c in Cadence::ALL {
            assert_eq!(Cadence::parse(c.as_str()), Some(c));
            assert!(!c.label().is_empty());
            assert!(!c.hint().is_empty());
            assert!(c.pas() >= 0);
            // Un rythme qui revient avance ; celui qui ne revient pas
            // n'avance pas.
            assert_eq!(c.pas() == 0, c == Cadence::Unique);
        }
        assert_eq!(Cadence::Paires.pas(), 7);
        assert_eq!(Cadence::Impaires.pas(), 7);
        assert_eq!(Cadence::UneSurDeux.pas(), 14);
        assert_eq!(Cadence::Quotidien.pas(), 1);
        // **Un seul rythme exige une fin**, et c'est celui qui, sans
        // elle, poserait un congé perpétuel.
        assert!(Cadence::Quotidien.needs_an_end());
        assert!(Cadence::ALL
            .into_iter()
            .filter(|c| c.needs_an_end())
            .eq([Cadence::Quotidien]));
        // **La chaîne vide n'est pas « ce jour-là ».** C'est ce que
        // portent les lignes écrites avant ce type, dont le rythme est
        // dans `repeat_days` : leur donner `Unique` effacerait les
        // trames de l'an dernier.
        assert_eq!(Cadence::parse(""), None);
        assert_eq!(Cadence::parse("TOUS_LES_MARDIS"), None);
        // L'alternance a deux moitiés, et elles seules.
        assert_eq!(Cadence::Paires.other_half(), Some(Cadence::Impaires));
        assert_eq!(Cadence::Impaires.other_half(), Some(Cadence::Paires));
        assert_eq!(Cadence::Hebdomadaire.other_half(), None);
        // Seules les parités trient ; les autres acceptent tout, y
        // compris ce qu'elles ne savent pas lire, parce que ce n'est
        // pas leur question.
        assert!(Cadence::Hebdomadaire.accepte("n'importe quoi"));
        assert!(!Cadence::Paires.accepte("n'importe quoi"));
    }

    /// **Un congé se pose comme une plage, et la plage est une ligne.**
    /// « Du 12 au 26 octobre » est quinze jours dépliés depuis un seul
    /// enregistrement — et sans date de fin, ce n'est pas un congé mais
    /// quelqu'un d'absent pour toujours.
    #[test]
    fn a_daily_rhythm_is_how_a_date_range_is_written() {
        let jours = occurrences(Cadence::Quotidien, "2026-10-12", 15);
        assert_eq!(jours.len(), 15);
        assert_eq!(jours.first().map(String::as_str), Some("2026-10-12"));
        assert_eq!(jours.last().map(String::as_str), Some("2026-10-26"));
        // Aucun jour sauté : c'est ce que « plage » veut dire, et un
        // rythme qui filtrerait ici laisserait des trous dans un congé.
        for pair in jours.windows(2) {
            assert_eq!(
                crate::date::days_between(&pair[0], &pair[1]),
                Some(1),
                "{} puis {}",
                pair[0],
                pair[1]
            );
        }
    }

    /// **Une trame se pose sur la semaine que sa date nomme**, en
    /// arrière comme en avant.
    ///
    /// C'est la règle que « le premier mercredi à partir d'ici » rate,
    /// et elle ne se voit que sur une trame réglée en milieu de
    /// semaine : le mercredi tomberait le jour même et le lundi huit
    /// jours plus tard, si bien que les sept journées saisies d'un coup
    /// ne formeraient plus une semaine.
    #[test]
    fn a_frame_lands_on_the_week_its_date_names() {
        // Le 7 septembre 2026 est un lundi, le 9 un mercredi.
        assert_eq!(in_week_of("2026-09-07", 1).as_deref(), Some("2026-09-07"));
        assert_eq!(in_week_of("2026-09-07", 3).as_deref(), Some("2026-09-09"));
        assert_eq!(in_week_of("2026-09-07", 7).as_deref(), Some("2026-09-13"));
        // Depuis le mercredi, le lundi est celui de **cette**
        // semaine-là — deux jours en arrière, et non cinq en avant.
        assert_eq!(in_week_of("2026-09-09", 1).as_deref(), Some("2026-09-07"));
        assert_eq!(in_week_of("2026-09-09", 7).as_deref(), Some("2026-09-13"));
        // Quel que soit le jour de départ, les sept rendus forment une
        // semaine : sept jours consécutifs, lundi en tête.
        for start in ["2026-09-07", "2026-09-09", "2026-09-13"] {
            let week: Vec<String> = (1..=7).filter_map(|d| in_week_of(start, d)).collect();
            assert_eq!(week.len(), 7);
            assert_eq!(week[0], "2026-09-07", "depuis {start}");
            assert_eq!(week[6], "2026-09-13", "depuis {start}");
        }
        // Et le passage de mois se fait par le calendrier, pas par une
        // soustraction sur le numéro du jour.
        assert_eq!(in_week_of("2026-10-01", 1).as_deref(), Some("2026-09-28"));
        assert_eq!(in_week_of("pas une date", 1), None);
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
