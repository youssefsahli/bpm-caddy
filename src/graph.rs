//! The drug base as a map: one card in the middle, its neighbours
//! around it, and one click to move the middle.
//!
//! The list and the search answer « où est telle fiche ». They do not
//! answer « qu'est-ce qu'il y a autour », which is the question of a
//! rupture de stock, of a contre-indication found at the counter, and of
//! anyone learning a class. The technical sheet gives that answer for
//! one card in two lists; this gives it as a picture, and the picture
//! walks — clicking a neighbour makes it the centre and redraws its own
//! neighbourhood, so a class is explored by moving through it rather
//! than by typing a name, reading it, going back and typing another.
//!
//! Three kinds of tie, and they are not the same thing:
//!
//! * **la molécule** — another brand of the same DCI. The substitution
//!   question, and the only tie where the two boxes hold the same drug.
//! * **la classe** — another molecule of the same class. What a rupture
//!   or an intolerance asks.
//! * **l'interaction** — a card this one's own monograph names. The tie
//!   that does not follow from the classification, and the only one that
//!   can cross the whole base — an AVK to an antibiotic, a statine to a
//!   pamplemousse-metabolised azole.
//!
//! Pure and tested, like `revue` and `conciliation`: no database here,
//! no egui here. What comes out is a set of points on the unit circle
//! and what each one is; the view scales them into whatever rectangle it
//! was given. That split is what makes a map of eight hundred and fifty
//! cards cost nothing per frame — the layout is computed when the centre
//! moves, and painted from then on.

use crate::fuzzy;

/// One card, as the map needs to read it.
pub struct Known<'a> {
    pub id: i64,
    pub name: &'a str,
    pub dci: &'a str,
    pub class: &'a str,
    /// The card's own interactions section, where the tie of the third
    /// kind is found: the map looks for other cards' names in it.
    pub ddi: &'a str,
    /// Whether the card carries a « toxicité / marge thérapeutique »
    /// section. Worth seeing on a map — it is the one property that
    /// changes what you do with a neighbour you were about to suggest.
    pub narrow: bool,
}

/// How a neighbour is tied to the centre.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tie {
    Molecule,
    Class,
    Interaction,
}

impl Tie {
    /// The key of its French label, for the legend and the tooltips.
    pub fn label_key(self) -> &'static str {
        match self {
            Tie::Molecule => "graph_tie_molecule",
            Tie::Class => "graph_tie_class",
            Tie::Interaction => "graph_tie_interaction",
        }
    }

    /// Its index in `motif::chart`'s categorical ramp. Fixed per tie,
    /// so the colour means the same thing on every card's map — a
    /// colour that moved with the data would be decoration.
    ///
    /// Deliberately **not** the red of series 3, which was the first
    /// choice for the interaction: `motif::alert()` is the red that
    /// rings a narrow-margin card, and a red ring round a red node is
    /// no ring at all. The interaction gets the ochre.
    pub fn series(self) -> usize {
        match self {
            Tie::Molecule => 0,
            Tie::Class => 1,
            Tie::Interaction => 4,
        }
    }

    /// How far out its ring sits, as a fraction of the drawn radius.
    /// Closest first: same molecule is nearer than same class, which is
    /// nearer than a card merely named in the prose.
    fn radius(self) -> f32 {
        match self {
            Tie::Molecule => 0.38,
            Tie::Class => 0.70,
            Tie::Interaction => 1.0,
        }
    }

    /// The three, in the order they are laid out and drawn.
    pub const ALL: [Tie; 3] = [Tie::Molecule, Tie::Class, Tie::Interaction];
}

/// One card on the map, placed.
#[derive(Clone, PartialEq, Debug)]
pub struct Node {
    pub id: i64,
    pub name: String,
    pub dci: String,
    pub tie: Tie,
    pub narrow: bool,
    /// Position on the unit circle: the centre is `(0, 0)` and no node
    /// is further than 1 from it. The view multiplies by whatever half
    /// -width it has and adds its own middle.
    pub x: f32,
    pub y: f32,
}

/// A card's neighbourhood, laid out.
#[derive(Clone, PartialEq, Debug)]
pub struct Map {
    /// The card in the middle: id, name and whether it is narrow.
    pub centre: (i64, String, bool),
    pub nodes: Vec<Node>,
    /// Per tie, how many neighbours the ring could not take.
    ///
    /// Never silent. A class of forty drawn as twelve and nothing said
    /// would read as « il y en a douze », which is worse than a crowded
    /// ring: it is a wrong answer that looks like a complete one.
    pub omitted: Vec<(Tie, usize)>,
}

impl Map {
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// How many of one tie are drawn.
    pub fn count(&self, tie: Tie) -> usize {
        self.nodes.iter().filter(|n| n.tie == tie).count()
    }

    /// How many of one tie were left out.
    pub fn omitted_for(&self, tie: Tie) -> usize {
        self.omitted
            .iter()
            .find(|(t, _)| *t == tie)
            .map_or(0, |(_, n)| *n)
    }
}

/// How many neighbours each ring will take.
///
/// A ring is read at a glance or not at all: twelve names around a
/// circle can be read, forty cannot, and « les autres AINS » drawn as
/// forty overlapping labels tells you less than the number twelve and a
/// note saying twenty-eight more. The class ring is the one that
/// overflows — a DCI rarely has more than a handful of brands.
#[derive(Clone, Copy, Debug)]
pub struct Caps {
    pub molecule: usize,
    pub class: usize,
    pub interaction: usize,
}

impl Default for Caps {
    fn default() -> Self {
        Self {
            molecule: 8,
            class: 12,
            interaction: 8,
        }
    }
}

impl Caps {
    fn of(&self, tie: Tie) -> usize {
        match tie {
            Tie::Molecule => self.molecule,
            Tie::Class => self.class,
            Tie::Interaction => self.interaction,
        }
    }
}

/// Read `centre`'s neighbourhood out of `base` and place it.
///
/// One pass over the base per tie, and the ties are decided in order —
/// a card that is the same molecule is not also listed as the same
/// class, and neither is listed again as an interaction. A card belongs
/// to the *closest* ring it qualifies for, and to one ring only: the
/// same name twice on one map is two answers to one question.
pub fn around(centre: &Known, base: &[Known], caps: Caps) -> Map {
    let mut taken: Vec<i64> = vec![centre.id];
    let mut nodes: Vec<Node> = Vec::new();
    let mut omitted: Vec<(Tie, usize)> = Vec::new();
    // Folded once, here, and not once per candidate: the third ring
    // asks its question of every card in the base.
    let hay = interaction_haystack(centre.ddi);

    for tie in Tie::ALL {
        let mut ring: Vec<&Known> = base
            .iter()
            .filter(|k| !taken.contains(&k.id) && ties(centre, k, &hay, tie))
            .collect();
        // A stable order, so the same card always draws the same map:
        // a ring whose members swapped places between two openings
        // would be a picture nobody could learn.
        ring.sort_by_key(|k| fuzzy::sort_key(k.name));
        let cap = caps.of(tie);
        if ring.len() > cap {
            omitted.push((tie, ring.len() - cap));
            ring.truncate(cap);
        }
        for k in &ring {
            taken.push(k.id);
        }
        place(&ring, tie, &mut nodes);
    }

    Map {
        centre: (centre.id, centre.name.trim().to_owned(), centre.narrow),
        nodes,
        omitted,
    }
}

/// Is `other` tied to `centre` in this particular way? `folded_ddi` is
/// the centre's interactions section, already folded.
fn ties(centre: &Known, other: &Known, folded_ddi: &str, tie: Tie) -> bool {
    match tie {
        // An empty DCI or class is not a molecule and not a class: it
        // is a card the team has not finished. Matching on it would
        // make every unfilled fiche everybody's neighbour.
        Tie::Molecule => !centre.dci.trim().is_empty() && fuzzy::eq_folded(centre.dci, other.dci),
        Tie::Class => {
            !centre.class.trim().is_empty() && fuzzy::eq_folded(centre.class, other.class)
        }
        // Named in the centre's own interactions, by brand or by DCI.
        Tie::Interaction => named_in(folded_ddi, other.name) || named_in(folded_ddi, other.dci),
    }
}

/// The centre's interactions section, folded once for searching.
///
/// Split out because [`around`] would otherwise fold the same paragraph
/// once per card in the base — eight hundred and fifty times for one
/// map, and the map is redrawn every time the centre moves.
fn interaction_haystack(ddi: &str) -> String {
    fuzzy::sort_key(ddi)
}

/// Is this name written in that folded prose, as a **whole word**?
///
/// Whole-word and not merely contained: « fer » matches « conférer »,
/// and a map that ties every card to the iron because both share three
/// letters is a map of nothing. A name shorter than four letters is
/// refused outright — the base holds « IEC » and « AVK » in DCI fields,
/// and three letters inside a paragraph of French are an accident.
fn named_in(folded_hay: &str, name: &str) -> bool {
    let name = name.trim();
    if name.chars().count() < 4 {
        return false;
    }
    let needle = fuzzy::sort_key(name);
    let bytes = folded_hay.as_bytes();
    let mut from = 0;
    while let Some(at) = folded_hay[from..].find(&needle) {
        let start = from + at;
        let end = start + needle.len();
        let before_ok = start == 0 || !bytes[start - 1].is_ascii_alphanumeric();
        let after_ok = end >= bytes.len() || !bytes[end].is_ascii_alphanumeric();
        if before_ok && after_ok {
            return true;
        }
        // Advance past this occurrence's first character, not past the
        // whole match: overlapping names exist and `find` on an empty
        // remainder would loop.
        from = start + needle.chars().next().map_or(1, char::len_utf8);
        if from >= folded_hay.len() {
            break;
        }
    }
    false
}

/// Spread a ring evenly round its circle, and append it.
///
/// Evenly and deterministically: no jitter, no force simulation, no
/// random seed. A map that settles differently each time it is opened
/// cannot be learned, and a force simulation is a per-frame cost for a
/// picture whose answer does not change.
///
/// Each ring starts a little further round than a plain twelve o'clock
/// would put it, so a card with one neighbour in each ring does not draw
/// three nodes stacked straight above the centre.
fn place(ring: &[&Known], tie: Tie, out: &mut Vec<Node>) {
    let n = ring.len();
    if n == 0 {
        return;
    }
    let r = tie.radius();
    let offset = match tie {
        Tie::Molecule => 0.0,
        Tie::Class => std::f32::consts::PI / 7.0,
        Tie::Interaction => std::f32::consts::PI / 3.5,
    };
    for (i, k) in ring.iter().enumerate() {
        // Straight up is zero, going clockwise, which is how anyone
        // reads a dial.
        let a = offset + std::f32::consts::TAU * i as f32 / n as f32;
        out.push(Node {
            id: k.id,
            name: k.name.trim().to_owned(),
            dci: k.dci.trim().to_owned(),
            tie,
            narrow: k.narrow,
            x: r * a.sin(),
            y: -r * a.cos(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card<'a>(id: i64, name: &'a str, dci: &'a str, class: &'a str) -> Known<'a> {
        Known {
            id,
            name,
            dci,
            class,
            ddi: "",
            narrow: false,
        }
    }

    fn base<'a>() -> Vec<Known<'a>> {
        vec![
            Known {
                id: 1,
                name: "Eliquis",
                dci: "apixaban",
                class: "AOD",
                ddi: "Association déconseillée avec le kétoconazole et la rifampicine.",
                narrow: true,
            },
            card(2, "Apixaban Viatris", "Apixaban", "aod"),
            card(3, "Xarelto", "rivaroxaban", "AOD"),
            card(4, "Pradaxa", "dabigatran", "AOD"),
            card(5, "Nizoral", "kétoconazole", "antifongique azolé"),
            card(6, "Rifadine", "rifampicine", "antituberculeux"),
            card(7, "Previscan", "fluindione", "AVK"),
        ]
    }

    /// Each ring holds what it says it holds, and a card appears once.
    #[test]
    fn a_card_belongs_to_the_closest_ring_and_to_one_only() {
        let b = base();
        let map = around(&b[0], &b, Caps::default());
        let names = |tie: Tie| {
            let mut v: Vec<&str> = map
                .nodes
                .iter()
                .filter(|n| n.tie == tie)
                .map(|n| n.name.as_str())
                .collect();
            v.sort_unstable();
            v
        };
        assert_eq!(names(Tie::Molecule), vec!["Apixaban Viatris"]);
        assert_eq!(names(Tie::Class), vec!["Pradaxa", "Xarelto"]);
        // Named in Eliquis's own interactions, by DCI, and reached
        // through the brand card that carries it.
        assert_eq!(names(Tie::Interaction), vec!["Nizoral", "Rifadine"]);
        // The AVK shares nothing with an AOD but the indication, which
        // the base does not record: it is not on the map.
        assert!(!map.nodes.iter().any(|n| n.name == "Previscan"));
        // The centre is never its own neighbour, and nothing repeats.
        assert!(!map.nodes.iter().any(|n| n.id == 1));
        let mut ids: Vec<i64> = map.nodes.iter().map(|n| n.id).collect();
        ids.sort_unstable();
        let unique = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), unique, "une fiche deux fois sur la carte");
        // What the centre is, carried through for the drawing.
        assert_eq!(map.centre, (1, "Eliquis".to_owned(), true));
    }

    /// Every node lands inside the unit circle, on its ring's radius,
    /// and the rings are ordered closest tie first.
    #[test]
    fn the_rings_are_ordered_and_nothing_leaves_the_circle() {
        let b = base();
        let map = around(&b[0], &b, Caps::default());
        let radius = |n: &Node| (n.x * n.x + n.y * n.y).sqrt();
        for n in &map.nodes {
            let r = radius(n);
            assert!(r <= 1.0001, "{} sort du cercle : {r}", n.name);
            assert!((r - n.tie.radius()).abs() < 0.001, "{} hors anneau", n.name);
        }
        let ring = |t: Tie| map.nodes.iter().find(|n| n.tie == t).map(radius).unwrap();
        assert!(ring(Tie::Molecule) < ring(Tie::Class));
        assert!(ring(Tie::Class) < ring(Tie::Interaction));
    }

    /// A ring bigger than its cap is cut — and says by how much.
    ///
    /// The silence is what would be wrong: twelve of forty drawn with
    /// nothing said reads as « il y en a douze », a wrong answer that
    /// looks complete.
    #[test]
    fn a_ring_too_full_is_cut_and_never_silently() {
        let names: Vec<String> = (0..30).map(|i| format!("AINS {i:02}")).collect();
        let dcis: Vec<String> = (0..30).map(|i| format!("molécule {i:02}")).collect();
        let mut b: Vec<Known> = (0..30)
            .map(|i| card(i as i64 + 1, &names[i], &dcis[i], "AINS"))
            .collect();
        b[0].dci = "ibuprofène";
        let centre = Known {
            id: 99,
            name: "Advil",
            dci: "ibuprofène",
            class: "AINS",
            ddi: "",
            narrow: false,
        };
        let caps = Caps {
            molecule: 8,
            class: 12,
            interaction: 8,
        };
        let map = around(&centre, &b, caps);
        assert_eq!(map.count(Tie::Class), 12);
        // Thirty class-mates. One of them is also the same molecule and
        // is taken by the inner ring first, leaving twenty-nine for the
        // class ring: twelve drawn, seventeen said.
        assert_eq!(map.count(Tie::Molecule), 1);
        assert_eq!(map.omitted_for(Tie::Class), 30 - 1 - 12);
        assert_eq!(map.omitted_for(Tie::Molecule), 0);
        assert_eq!(map.omitted_for(Tie::Interaction), 0);
    }

    /// The map is the same map every time it is drawn.
    ///
    /// No jitter and no simulation: a picture that settles differently
    /// on each opening cannot be learned, and this one is meant to be
    /// walked through.
    #[test]
    fn the_same_card_always_draws_the_same_map() {
        let b = base();
        let once = around(&b[0], &b, Caps::default());
        let twice = around(&b[0], &b, Caps::default());
        assert_eq!(once, twice);
    }

    /// A name is found in the prose as a whole word, or not at all.
    ///
    /// « fer » is inside « conférer », « aine » inside « migraine ». A
    /// map that ties two cards because three letters of one are inside a
    /// sentence of the other is a map of coincidences.
    #[test]
    fn a_name_in_the_prose_is_a_whole_word_or_nothing() {
        let hay = interaction_haystack(
            "Le millepertuis diminue l'exposition. Conférer avec le prescripteur \
             avant d'associer la rifampicine ; migraine possible.",
        );
        assert!(named_in(&hay, "rifampicine"));
        assert!(named_in(&hay, "Millepertuis"), "casse et accents pliés");
        // Inside another word, and never a match.
        assert!(!named_in(&hay, "fer"), "« conférer » n'est pas le fer");
        assert!(!named_in(&hay, "aine"));
        // Too short to be anything but an accident in French prose.
        assert!(!named_in(&hay, "AVK"));
        assert!(!named_in(&hay, ""));
        // Punctuation and end of string still bound a word.
        assert!(named_in(
            &interaction_haystack("… la warfarine."),
            "warfarine"
        ));
        assert!(named_in(&interaction_haystack("warfarine"), "warfarine"));
    }

    /// A card with nothing written on it is nobody's neighbour, and has
    /// no neighbourhood of its own.
    #[test]
    fn an_empty_card_is_not_everybodys_neighbour() {
        let b = vec![
            card(1, "Sans rien", "", ""),
            card(2, "Sans rien non plus", "", ""),
            card(3, "Eliquis", "apixaban", "AOD"),
        ];
        let map = around(&b[0], &b, Caps::default());
        assert!(map.is_empty(), "{map:?}");
        // …and from the other side: a filled card does not gain an
        // empty one as a class-mate.
        let map = around(&b[2], &b, Caps::default());
        assert!(map.is_empty(), "{map:?}");
    }
}
