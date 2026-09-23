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
    /// section. Worth seeing on a map: it is what makes you reopen the
    /// card of a neighbour you were about to suggest.
    ///
    /// **Named for the data, not for a reading of it.** It was `narrow`,
    /// and that name is what the view's comment and then its legend
    /// ended up asserting — « marge thérapeutique étroite », which this
    /// is not: the field is a prose section, filled on 484 of the 862
    /// shipped cards, Zeclar and Sporanox among them.
    pub toxicity_noted: bool,
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
    /// rings a card with a toxicity section, and a red ring round a red
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
    ///
    /// **Publique, et lue par la vue.** Les trois nombres étaient écrits
    /// deux fois — ici, et dans le `match` qui trace les anneaux : deux
    /// écritures d'une distance divergent, et celle qu'on corrigerait
    /// n'est pas celle qui place les nœuds.
    pub fn ring_radius(self) -> f32 {
        match self {
            Tie::Molecule => 0.38,
            Tie::Class => 0.70,
            Tie::Interaction => 1.0,
        }
    }

    /// The three, in the order they are laid out and drawn.
    pub const ALL: [Tie; 3] = [Tie::Molecule, Tie::Class, Tie::Interaction];
}

/// Ce que l'œil regarde : un grossissement et un décalage.
///
/// La figure était posée dans son rectangle et on la regardait d'un seul
/// endroit. Un anneau serré ne se lisait alors qu'en agrandissant la
/// fenêtre, et les noms que la place refusait n'avaient d'autre recours
/// que l'infobulle.
///
/// **Grossir ne relit pas la base**, et c'est délibéré. Les plafonds
/// restent ceux du volet : aucun nœud n'apparaît ni ne disparaît sous
/// les doigts, et surtout aucun ne change de place — un anneau se
/// répartit régulièrement, si bien qu'un membre de plus les ferait tous
/// tourner, et une image qui tourne pendant qu'on la grossit est une
/// image que personne ne peut suivre. Ce qu'on gagne en grossissant, ce
/// sont les **noms** que la place refusait, c'est-à-dire exactement ce
/// qui manquait.
///
/// Pur, et c'est ce qui permet de tenir l'arithmétique sans écran : un
/// grossissement qui ne garde pas sous le pointeur ce qui y était fait
/// fuir la figure au premier cran de molette, et cela ne se voit que
/// sur une capture.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Look {
    /// 1,0 est la figure telle que le volet la pose.
    pub zoom: f32,
    /// Le décalage, en pixels de l'écran, du milieu de la figure.
    pub pan: (f32, f32),
}

impl Default for Look {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan: (0.0, 0.0),
        }
    }
}

impl Look {
    /// En deçà, la figure est un pâté ; au delà, un anneau ne tient plus
    /// dans le creux et il n'y a plus de carte, seulement un nœud.
    pub const MIN: f32 = 0.5;
    pub const MAX: f32 = 4.0;
    /// Ce que vaut un cran de bouton. Multiplicatif et non additif :
    /// c'est la seule façon qu'un cran en avant et un cran en arrière se
    /// rendent au même endroit.
    pub const STEP: f32 = 1.25;

    /// Est-ce la vue telle que le volet la pose ?
    pub fn is_plain(self) -> bool {
        (self.zoom - 1.0).abs() < 1e-3 && self.pan.0.abs() < 0.5 && self.pan.1.abs() < 0.5
    }

    /// Grossir de `factor` **autour de `at`** — un décalage en pixels
    /// depuis le milieu du creux, celui du pointeur.
    ///
    /// Ce que le point sous le pointeur désigne y reste : c'est la seule
    /// façon dont une molette se lit. Grossir autour du milieu fait
    /// glisser sous les doigts ce qu'on regardait, et on rattrape la
    /// figure au lieu de la lire.
    pub fn zoom_about(self, factor: f32, at: (f32, f32)) -> Look {
        if !factor.is_finite() || factor <= 0.0 || !self.zoom.is_finite() || self.zoom <= 0.0 {
            return self;
        }
        let zoom = (self.zoom * factor).clamp(Self::MIN, Self::MAX);
        // Le facteur **réellement** appliqué, après bornage : sans cela,
        // molettant contre la borne, le décalage continuerait de courir
        // alors que la taille ne bouge plus.
        let k = zoom / self.zoom;
        Look {
            zoom,
            pan: (
                at.0 - (at.0 - self.pan.0) * k,
                at.1 - (at.1 - self.pan.1) * k,
            ),
        }
    }

    /// Ramener le décalage dans ce qu'on peut atteindre.
    ///
    /// `half` est le demi-rectangle du creux, `radius` les deux
    /// demi-axes de la figure **avant** grossissement. La règle tient en
    /// une phrase : **on peut amener n'importe quel nœud au milieu, et
    /// on ne peut pas perdre la figure.** D'où la borne, le plus grand
    /// des deux — sans le premier terme, une figure plus petite que son
    /// creux ne se déplacerait pas du tout ; sans le second, un anneau
    /// grossi quatre fois aurait des nœuds qu'aucun décalage ne ramène.
    pub fn clamp_pan(self, half: (f32, f32), radius: (f32, f32)) -> Look {
        let lim = |h: f32, r: f32| {
            let v = h.max(r * self.zoom);
            if v.is_finite() {
                v.max(0.0)
            } else {
                0.0
            }
        };
        let (lx, ly) = (lim(half.0, radius.0), lim(half.1, radius.1));
        Look {
            zoom: self.zoom,
            pan: (self.pan.0.clamp(-lx, lx), self.pan.1.clamp(-ly, ly)),
        }
    }
}

/// **Pourquoi deux fiches se rencontrent** — ce que le trait veut dire.
///
/// Le troisième anneau ne savait qu'une chose : « la monographie du
/// centre écrit ce nom ». Trois tables de la maison en savent davantage,
/// et chacune le dit à sa façon ; aucune ne conclut à la place du
/// prescripteur, et ce type n'a pas de champ où conclure.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Why {
    /// L'une des deux fiches nomme l'autre dans ses interactions : la
    /// phrase, citée, et le nom de la fiche qui l'écrit.
    Cited { by: String, sentence: String },
    /// La table des cytochromes : qui agit, sur quelle enzyme, dans quel
    /// sens, et si elle le range « pour mémoire ».
    Enzyme {
        actor: String,
        enzyme: String,
        shift: String,
        minor: bool,
    },
    /// La revue d'ordonnance : un effet qui s'additionne sans passer par
    /// une enzyme — deux allongeurs du QT, deux sédatifs, le saignement.
    Effect {
        title: String,
        detail: String,
        alert: bool,
    },
}

impl Why {
    /// Son poids de lecture : 3 ce qu'on regarde d'abord, 2 ce qu'on
    /// regarde, 1 ce qui est noté.
    ///
    /// Pour une phrase citée, **le mot du thésaurus que la fiche écrit
    /// elle-même** — « contre-indiqué » — et rien d'autre ne la fait
    /// monter au premier rang. Ce n'est pas une gravité clinique ; c'est
    /// l'ordre dans lequel la carte montre, comme `cyp::Weight`.
    pub fn weight(&self) -> u8 {
        match self {
            Why::Cited { sentence, .. } => {
                let s = fuzzy::sort_key(sentence);
                // Une phrase que la fiche écrit nommément vaut au moins
                // « à regarder » : quelqu'un l'a écrite pour cette paire,
                // quand une règle de la revue parle d'une classe entière.
                if ["contre-indiqu", "contre indiqu", "contreindiqu"]
                    .iter()
                    .any(|w| s.contains(w))
                {
                    3
                } else {
                    2
                }
            }
            // **Jamais 3.** La table des cytochromes le dit elle-même : son
            // « à regarder d'abord » est un ordre de lecture et non une
            // gravité — elle ne sait ni la dose, ni la durée, ni le
            // terrain. Un trait rouge sur sa seule foi dirait ce qu'elle
            // refuse de dire.
            Why::Enzyme { minor, .. } => {
                if *minor {
                    1
                } else {
                    2
                }
            }
            Why::Effect { alert, .. } => {
                if *alert {
                    3
                } else {
                    2
                }
            }
        }
    }
}

impl Why {
    /// À poids égal, qui passe d'abord : **ce qu'une fiche écrit** de
    /// cette paire, puis la revue, puis les cytochromes.
    fn source_rank(&self) -> u8 {
        match self {
            Why::Cited { .. } => 0,
            Why::Effect { .. } => 1,
            Why::Enzyme { .. } => 2,
        }
    }

    /// Ce que la raison dit, sans la paire : deux fiches qui rencontrent
    /// le centre par la même règle ont la même clé.
    fn key(&self) -> String {
        match self {
            Why::Cited { by, sentence } => format!("c:{by}:{sentence}"),
            Why::Effect { title, .. } => format!("e:{title}"),
            Why::Enzyme { enzyme, shift, .. } => format!("z:{enzyme}:{shift}"),
        }
    }
}

/// La raison qui décide de la place d'un trait : la plus lourde, puis
/// celle de la source qui passe d'abord.
fn lead(why: &[Why]) -> Option<&Why> {
    why.iter()
        .min_by_key(|w| (std::cmp::Reverse(w.weight()), w.source_rank()))
}

/// Le poids d'un trait : celui de sa raison la plus lourde, 1 quand la
/// carte le tient d'un nom cité sans phrase retrouvée.
pub fn weight_of(why: &[Why]) -> u8 {
    why.iter().map(Why::weight).max().unwrap_or(1)
}

/// One card on the map, placed.
#[derive(Clone, PartialEq, Debug)]
pub struct Node {
    pub id: i64,
    pub name: String,
    pub dci: String,
    pub tie: Tie,
    pub toxicity_noted: bool,
    /// Ce que le trait veut dire, quand une table le sait ; vide pour un
    /// voisin de molécule ou de classe, qui ne se prend pas avec le
    /// centre mais à sa place.
    pub why: Vec<Why>,
    /// Son poids de lecture (voir [`Why::weight`]) ; 0 hors de l'anneau
    /// des interactions.
    pub weight: u8,
    /// Position on the unit circle: the centre is `(0, 0)` and no node
    /// is further than 1 from it. The view multiplies by whatever half
    /// -width it has and adds its own middle.
    pub x: f32,
    pub y: f32,
}

/// A card's neighbourhood, laid out.
#[derive(Clone, PartialEq, Debug)]
pub struct Map {
    /// The card in the middle: id, name and whether its card documents
    /// a toxicity or a therapeutic margin.
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Caps {
    pub molecule: usize,
    pub class: usize,
    pub interaction: usize,
    /// Ce que les trois anneaux peuvent écrire **ensemble**.
    ///
    /// Les trois plafonds ne se voient pas les uns les autres, et les
    /// noms des trois anneaux s'écrivent dans les mêmes rangées : tenus
    /// séparément, ils laissaient la carte du Durogesic poser vingt-deux
    /// noms dans la place de douze. Voir [`share`].
    pub total: usize,
}

impl Default for Caps {
    fn default() -> Self {
        Self {
            molecule: 8,
            class: 12,
            interaction: 8,
            // La somme des trois : par défaut le partage ne décide rien,
            // et ce sont les plafonds de lecture qui parlent seuls.
            total: 28,
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

    /// Ce que la **place** permet d'écrire, anneau par anneau.
    ///
    /// Douze était un nombre, et un nombre ne connaît pas le volet où il
    /// se dessine. Mesuré sur la vue livrée à 1024x700 et
    /// `text_scale = 1,6`, l'anneau de la classe avait quatre-vingts
    /// pixels de demi-hauteur pour douze noms : six d'entre eux
    /// n'étaient **pas peints du tout**, chacun refusé par le voisin qui
    /// avait pris sa place. Douze carrés dont la moitié sont muets en
    /// disent moins que huit nommés et « quatre de plus » écrit dessous —
    /// c'est la règle de ce dépôt, un graphique compte ce qui ne tient
    /// pas plutôt que de le peindre par-dessus.
    ///
    /// L'arithmétique : sur un anneau de demi-hauteur `r`, les noms
    /// s'écrivent vers l'extérieur, donc en rangées. Pour `n` nœuds
    /// répartis régulièrement, l'écart vertical le plus serré est celui
    /// du haut et du bas, et il vaut environ `r · (2π/n)²`. Le demander
    /// plus grand qu'une ligne donne `n ≤ 2π·√(r / hauteur de ligne)`.
    /// Ce n'est pas une estimation prudente d'un cas moyen : c'est la
    /// paire la plus serrée, celle qui décide.
    ///
    /// Jamais moins de trois par anneau : un anneau réduit à un point
    /// n'est plus un anneau, et ce qu'il ne montre pas se dit dessous.
    /// Jamais plus que le plafond de lecture — un cercle de quarante
    /// noms ne se lit pas, quelle que soit la place.
    pub fn for_room(self, ry: f32, line_h: f32) -> Caps {
        let fit = |tie: Tie| -> usize {
            let r = ry * tie.ring_radius();
            // Une place ou une ligne qui n'en est pas une — zéro,
            // négative, ou pas un nombre — ne fait pas plier le
            // plafond : la vue n'a alors rien mesuré, et rogner sur une
            // mesure absente serait décider à sa place.
            if !r.is_finite() || !line_h.is_finite() || r <= 0.0 || line_h <= 0.0 {
                return self.of(tie);
            }
            // Deux bornes, et c'est la plus serrée qui vaut.
            //
            // **L'écartement** : l'écart vertical le plus serré de
            // l'anneau, celui du haut et du bas.
            let crowd = std::f32::consts::TAU * (r / line_h).sqrt();
            // **Les rangées** : un anneau de demi-hauteur `r` couvre
            // `2r` de haut, donc `2r / ligne` rangées de chaque côté.
            // C'est cette borne-là qui décide sur l'anneau du dedans, où
            // le rayon est petit — l'écartement seul y autorisait sept
            // noms dans la hauteur de quatre, et ils s'écrivaient sur le
            // moyeu.
            let rows = 4.0 * r / line_h;
            // `as usize` tronque, et tronquer est le bon sens ici : un
            // nom de plus que la place est un nom qui n'est pas peint.
            (crowd.min(rows) as usize).clamp(3, self.of(tie))
        };
        // Et la place **commune** : les noms s'écrivent vers l'extérieur,
        // donc en rangées, et une rangée sert les trois anneaux. La
        // figure a `2·ry / hauteur de ligne` rangées de chaque côté.
        //
        // Moins deux : le haut et le bas de la figure n'ont pas de côté.
        // Un nœud posé là écrit son nom au-dessus ou au-dessous de
        // lui-même, et cette rangée-là, les deux côtés se la partagent
        // au lieu de l'avoir chacun.
        let rows = if ry.is_finite() && line_h.is_finite() && ry > 0.0 && line_h > 0.0 {
            ((4.0 * ry / line_h) as usize).saturating_sub(2).max(6)
        } else {
            self.total
        };
        Caps {
            molecule: fit(Tie::Molecule),
            class: fit(Tie::Class),
            interaction: fit(Tie::Interaction),
            total: rows.min(self.total),
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
    around_with(centre, base, caps, &std::collections::HashMap::new())
}

/// La même chose, avec **ce que les tables savent** de chaque paire
/// (centre, fiche) : `reasons` donne, par fiche, pourquoi elle rencontre
/// le centre.
///
/// Une fiche qui a une raison entre dans l'anneau des interactions même
/// quand aucune monographie ne nomme l'autre — deux allongeurs du QT ne
/// se citent pas —, sauf si elle appartient déjà à un anneau plus
/// proche : un voisin de classe se prend **à la place** du centre, pas
/// avec lui. Et quand l'anneau est trop plein, **ce qu'on garde est ce
/// qui pèse le plus** : couper une contre-indication pour garder une
/// précaution d'emploi serait l'anneau qui ment.
pub fn around_with(
    centre: &Known,
    base: &[Known],
    caps: Caps,
    reasons: &std::collections::HashMap<i64, Vec<Why>>,
) -> Map {
    let mut taken: Vec<i64> = vec![centre.id];
    let mut nodes: Vec<Node> = Vec::new();
    let mut omitted: Vec<(Tie, usize)> = Vec::new();
    // Folded once, here, and not once per candidate: the third ring
    // asks its question of every card in the base.
    let hay = interaction_haystack(centre.ddi);

    // **On rassemble les trois anneaux d'abord, on taille ensuite.**
    // Deux raisons, et la première est une correction.
    //
    // Une fiche appartient à l'anneau le plus proche pour lequel elle
    // se qualifie — **que cet anneau ait de la place pour elle ou
    // non**. Ne marquer que celles qu'on garde laissait un voisin de
    // classe que le plafond avait coupé ressortir sur l'anneau des
    // interactions, peint dans l'ocre d'un lien qu'il n'a pas, pendant
    // que le pied le comptait toujours parmi les voisins de classe non
    // dessinés. Il n'apparaissait qu'une fois, donc rien ne le montrait
    // — sinon une réponse fausse à la question « à quel titre ».
    //
    // Et la place se partage : voir [`Caps::total`].
    let rings: Vec<Vec<&Known>> = Tie::ALL
        .iter()
        .map(|tie| {
            let mut ring: Vec<&Known> = base
                .iter()
                .filter(|k| {
                    !taken.contains(&k.id)
                        && (ties(centre, k, &hay, *tie)
                            || (*tie == Tie::Interaction
                                && reasons.get(&k.id).is_some_and(|r| !r.is_empty())
                                && local_may_meet(centre, k)))
                })
                .collect();
            // A stable order, so the same card always draws the same
            // map: a ring whose members swapped places between two
            // openings would be a picture nobody could learn.
            ring.sort_by_key(|k| fuzzy::sort_key(k.name));
            for k in &ring {
                taken.push(k.id);
            }
            ring
        })
        .collect();

    let want = [
        rings[0].len().min(caps.of(Tie::Molecule)),
        rings[1].len().min(caps.of(Tie::Class)),
        rings[2].len().min(caps.of(Tie::Interaction)),
    ];
    let allow = share(want, caps.total);
    for (i, tie) in Tie::ALL.into_iter().enumerate() {
        let ring = &rings[i];
        let keep = allow[i].min(ring.len());
        if ring.len() > keep {
            omitted.push((tie, ring.len() - keep));
        }
        // **Lesquels** : un ordre dont tout préfixe est réparti et
        // contenu dans le suivant (voir [`spread_order`]), puis remis
        // dans l'ordre des places pour que le dessin les parcoure comme
        // on lit un cadran.
        let order = spread_order(ring.len());
        let mut slots: Vec<usize> = if tie == Tie::Interaction {
            // The heaviest first, and among equals the spread order —
            // so a ring that is not cut keeps its dial, and a ring that
            // is cut keeps what weighs.
            //
            // **Et une place par raison avant une seconde.** Dix
            // anticoagulants rencontrent un AINS par la même règle : les
            // garder tous coupait le lithium, qui la rencontre par une
            // autre. Chaque raison distincte a sa place d'abord, puis les
            // suivantes dans le même ordre — le pied compte le reste.
            let mut by: Vec<(usize, u8, u8, String, usize)> = order
                .iter()
                .enumerate()
                .map(|(rank, &j)| {
                    let why = reasons
                        .get(&ring[j].id)
                        .map(|v| v.as_slice())
                        .unwrap_or(&[]);
                    let (src, key) = lead(why)
                        .map(|w| (w.source_rank(), w.key()))
                        .unwrap_or((0, format!("n:{}", ring[j].id)));
                    (j, weight_of(why), src, key, rank)
                })
                .collect();
            by.sort_by(|a, b| {
                (std::cmp::Reverse(a.1), a.2, a.4).cmp(&(std::cmp::Reverse(b.1), b.2, b.4))
            });
            let mut seen: std::collections::HashMap<String, usize> = Default::default();
            let mut ranked: Vec<(usize, u8, usize, u8, usize)> = by
                .into_iter()
                .map(|(j, w, src, key, rank)| {
                    let n = seen.entry(key).or_insert(0);
                    let occ = *n;
                    *n += 1;
                    (j, w, occ, src, rank)
                })
                .collect();
            ranked.sort_by_key(|&(_, w, occ, src, rank)| (std::cmp::Reverse(w), occ, src, rank));
            ranked.into_iter().take(keep).map(|(j, ..)| j).collect()
        } else {
            order.into_iter().take(keep).collect()
        };
        slots.sort_unstable();
        let first = nodes.len();
        place(ring, &slots, ring.len(), tie, &mut nodes);
        if tie == Tie::Interaction {
            for n in &mut nodes[first..] {
                n.why = reasons.get(&n.id).cloned().unwrap_or_default();
                n.weight = weight_of(&n.why);
            }
        }
    }

    Map {
        centre: (
            centre.id,
            centre.name.trim().to_owned(),
            centre.toxicity_noted,
        ),
        nodes,
        omitted,
    }
}

/// Partager `total` places entre les trois anneaux qui en demandent
/// `want`.
///
/// Les plafonds par anneau ne suffisent pas, parce qu'ils ne se voient
/// pas les uns les autres : les noms des trois anneaux s'écrivent dans
/// **les mêmes rangées**, et trois plafonds tenus séparément laissaient
/// la carte du Durogesic poser vingt-deux noms dans la place de douze —
/// cinq fentanyls, les opioïdes forts et les interactions, tous écrits
/// les uns par-dessus les autres et à travers les rayons. C'est ce que
/// le plafond de lecture refuse depuis toujours, une rangée plus haut :
/// un cercle de quarante noms ne se lit pas.
///
/// **Le plus fourni cède le premier.** Un nom de moins sur l'anneau qui
/// en a douze se remarque moins qu'un nom de moins sur celui qui en a
/// deux, et deux anneaux finissent à égalité plutôt que l'un plein et
/// l'autre vide.
///
/// **Et aucun anneau non vide ne tombe à zéro.** Un anneau qui
/// disparaît de l'image laisse sous elle une note qui parle de ce qu'on
/// ne voit pas — « 9 de plus en même classe » sans une seule classe
/// dessinée ne dit rien à personne.
fn share(want: [usize; 3], total: usize) -> [usize; 3] {
    let mut allow = want;
    let mut sum: usize = allow.iter().sum();
    while sum > total {
        let Some(i) = (0..3).filter(|&i| allow[i] > 1).max_by_key(|&i| allow[i]) else {
            break;
        };
        allow[i] -= 1;
        sum -= 1;
    }
    allow
}

/// Is `other` tied to `centre` in this particular way? `folded_ddi` is
/// the centre's interactions section, already folded.
/// Une forme locale ne rencontre pas un médicament général — la règle
/// que l'anneau des interactions suit déjà pour les noms cités (voir
/// [`ties`]), et que les raisons venues des tables suivent aussi.
fn local_may_meet(centre: &Known, other: &Known) -> bool {
    crate::classes::is_local_form(centre.class)
        || !crate::classes::stays_local(other.dci, other.class)
}

fn ties(centre: &Known, other: &Known, folded_ddi: &str, tie: Tie) -> bool {
    match tie {
        // An empty DCI or class is not a molecule and not a class: it
        // is a card the team has not finished. Matching on it would
        // make every unfilled fiche everybody's neighbour.
        Tie::Molecule => !centre.dci.trim().is_empty() && fuzzy::eq_folded(centre.dci, other.dci),
        // **La classe canonique, jamais la chaîne.** C'est la règle de
        // la maison — « `same()` est ce sur quoi le voisinage et la
        // pastille comparent » — et cette carte-ci, dont c'est le sujet
        // même, comparait les libellés bruts. Le champ `class` d'une
        // fiche est du texte libre et il a dérivé : 495 libellés pour
        // 862 fiches. Mesuré sur la base livrée, comparer les chaînes
        // coûtait **706 paires de voisins sur 331 fiches** — Fosamax
        // n'était pas du même anneau qu'Actonel pour une lettre
        // (« bisphosphonate » / « biphosphonate »), Cimzia pas du même
        // qu'Amgevita pour un mot (« anti-TNF » / « anti-TNF alpha »),
        // et rien n'avait l'air cassé : l'anneau était dessiné, simplement
        // plus court. Une carte qui répond « voici la classe » en en
        // montrant la moitié est pire qu'une carte vide.
        Tie::Class => {
            !centre.class.trim().is_empty() && crate::classes::same(centre.class, other.class)
        }
        // Named in the centre's own interactions, by brand or by DCI.
        //
        // **Sauf une forme locale**, et pour la raison que `cyp.rs`
        // écrit déjà : un kétoconazole local n'est pas un kétoconazole.
        // La section « interactions » de l'Eliquis nomme les azolés —
        // elle parle des azolés **généraux** —, et la carte mettait donc
        // le Kétoderm, qui est un shampooing, face à un anticoagulant.
        // C'est le genre de lien qui apprend à ignorer les liens.
        //
        // La parenté de **molécule** n'est pas concernée : un
        // kétoconazole topique et un kétoconazole oral sont bien la même
        // molécule, et le voir vaut la peine. Et si le centre est
        // lui-même une forme locale, deux topiques se citent
        // légitimement.
        Tie::Interaction => {
            // Le miconazole buccal croise quand même : voir
            // `classes::local_but_absorbed`.
            (crate::classes::is_local_form(centre.class)
                || !crate::classes::stays_local(other.dci, other.class))
                && (named_in(folded_ddi, other.name) || named_in(folded_ddi, other.dci))
        }
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
///
/// **La place d'un membre se lit sur la liste entière, pas sur ce qu'on
/// en dessine.** L'angle valait `i / dessinés` : en ouvrir un de plus
/// les faisait donc tous tourner, et une figure qui tourne pendant
/// qu'on la règle est une figure qu'on ne peut pas suivre. Il vaut
/// `slot / candidats`, si bien qu'un membre gardé garde sa place et
/// qu'un membre de plus vient **se poser dans un trou**. C'est ce qui
/// permet de régler la carte entre « tout voir » et « tout lire » sans
/// la perdre de vue.
fn place(ring: &[&Known], slots: &[usize], total: usize, tie: Tie, out: &mut Vec<Node>) {
    if total == 0 {
        return;
    }
    let r = tie.ring_radius();
    let offset = match tie {
        Tie::Molecule => 0.0,
        Tie::Class => std::f32::consts::PI / 7.0,
        Tie::Interaction => std::f32::consts::PI / 3.5,
    };
    for &j in slots {
        let Some(k) = ring.get(j) else { continue };
        // Straight up is zero, going clockwise, which is how anyone
        // reads a dial.
        let a = offset + std::f32::consts::TAU * j as f32 / total as f32;
        out.push(Node {
            id: k.id,
            name: k.name.trim().to_owned(),
            dci: k.dci.trim().to_owned(),
            tie,
            toxicity_noted: k.toxicity_noted,
            why: Vec::new(),
            weight: 0,
            x: r * a.sin(),
            y: -r * a.cos(),
        });
    }
}

/// L'ordre dans lequel on ouvre les places d'un anneau : **tout préfixe
/// est réparti, et tout préfixe est contenu dans le suivant.**
///
/// C'est ce qui rend le réglage utilisable. Garder « les douze premiers
/// par ordre alphabétique » d'une classe de trente en dessinerait douze
/// côte à côte sur un arc et rien ailleurs ; garder douze *répartis*
/// montre la classe. Et comme on ne fait qu'allonger la liste, en
/// ouvrir un de plus ne retire jamais celui d'à côté : ce qu'on voyait,
/// on le voit encore.
///
/// La construction est un demi-pas répété — d'abord midi, puis six
/// heures, puis trois et neuf, puis les quarts — c'est-à-dire l'ordre
/// dont on remplit un cadran quand on veut pouvoir s'arrêter n'importe
/// quand.
fn spread_order(n: usize) -> Vec<usize> {
    let mut out: Vec<usize> = Vec::with_capacity(n);
    if n == 0 {
        return out;
    }
    out.push(0);
    let mut step = n;
    while out.len() < n {
        step = step.div_ceil(2);
        for i in (0..n).step_by(step.max(1)) {
            if !out.contains(&i) {
                out.push(i);
            }
        }
        if step <= 1 {
            // Le dernier passage prend tout ce qui reste : sans lui,
            // un compte impair laisserait des places jamais ouvertes.
            for i in 0..n {
                if !out.contains(&i) {
                    out.push(i);
                }
            }
            break;
        }
    }
    out
}

/// Une ordonnance disposée en cercle : chaque ligne à sa place, toutes
/// au même rayon.
///
/// L'autre disposition de ce module a un **centre** — une fiche, et ce
/// qu'il y a autour. Celle-ci n'en a pas : sur une ordonnance, aucune
/// ligne n'est le milieu, et en choisir une donnerait à lire une
/// hiérarchie qui n'existe pas. Ce qu'on veut voir, ce sont les cordes :
/// qui rencontre qui.
///
/// Comme l'autre, elle rend des points du cercle unité et la vue les met
/// à l'échelle de son rectangle. C'est ce partage qui permet de la
/// tester sans écran — et une disposition qu'on ne teste pas est une
/// disposition dont on découvre les chevauchements sur une capture.
///
/// Le premier point est **en haut**, et on tourne dans le sens des
/// aiguilles : c'est le sens de lecture d'un cadran, et le même que
/// [`place`]. L'ordre reçu est l'ordre gardé, si bien qu'une même
/// ordonnance dessine toujours le même cercle — un cercle dont les
/// membres changeraient de place entre deux ouvertures serait une image
/// que personne ne peut apprendre.
pub fn circle(n: usize) -> Vec<(f32, f32)> {
    if n == 0 {
        return Vec::new();
    }
    // **Un point seul est au centre, pas en haut.** Une ordonnance d'une
    // ligne n'a pas de cercle à dessiner, et poser son unique point sur
    // le bord laisserait un grand rond vide à côté d'un nom.
    if n == 1 {
        return vec![(0.0, 0.0)];
    }
    (0..n)
        .map(|i| {
            let a = std::f32::consts::TAU * i as f32 / n as f32;
            (a.sin(), -a.cos())
        })
        .collect()
}

/// Une corde de la carte d'une ordonnance : deux lignes qui se
/// rencontrent, et pourquoi.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Chord {
    /// Les deux lignes, par leur rang dans l'ordonnance, la plus petite
    /// d'abord.
    pub a: usize,
    pub b: usize,
    pub why: Vec<Why>,
    pub weight: u8,
}

/// **La carte d'une ordonnance** : ce que les tables trouvent entre ses
/// lignes, rangé par paire.
///
/// `found` est ce que les trois lectures rendent — chaque raison avec
/// les rangs des lignes qu'elle nomme. Une règle qui en nomme trois (la
/// « triade ») relie chacune aux deux autres : c'est ce qu'elle dit, et
/// la découper en une seule corde choisirait une paire au hasard. Une
/// raison répétée sur une paire ne compte qu'une fois.
///
/// Rend les cordes, **les plus lourdes d'abord** — c'est l'ordre dans
/// lequel on les peint par-dessus les autres et celui dans lequel on les
/// lit —, et les lignes qui ne rencontrent rien. Ces dernières ne sont
/// pas « sans interaction » : elles sont sans rencontre **dans ces
/// tables**, et la vue le dit en ces mots.
pub fn chords(lines: usize, found: &[(Vec<usize>, Why)]) -> (Vec<Chord>, Vec<usize>) {
    let mut out: Vec<Chord> = Vec::new();
    for (named, why) in found {
        let mut named: Vec<usize> = named.iter().copied().filter(|i| *i < lines).collect();
        named.sort_unstable();
        named.dedup();
        for (x, &a) in named.iter().enumerate() {
            for &b in &named[x + 1..] {
                match out.iter_mut().find(|c| c.a == a && c.b == b) {
                    Some(c) => {
                        if !c.why.contains(why) {
                            c.why.push(why.clone());
                        }
                    }
                    None => out.push(Chord {
                        a,
                        b,
                        why: vec![why.clone()],
                        weight: 0,
                    }),
                }
            }
        }
    }
    for c in &mut out {
        c.why
            .sort_by_key(|w| (std::cmp::Reverse(w.weight()), w.source_rank()));
        c.weight = weight_of(&c.why);
    }
    out.sort_by_key(|c| (std::cmp::Reverse(c.weight), c.a, c.b));
    let alone: Vec<usize> = (0..lines)
        .filter(|i| !out.iter().any(|c| c.a == *i || c.b == *i))
        .collect();
    (out, alone)
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
            toxicity_noted: false,
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
                toxicity_noted: true,
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
            assert!(
                (r - n.tie.ring_radius()).abs() < 0.001,
                "{} hors anneau",
                n.name
            );
        }
        let ring = |t: Tie| map.nodes.iter().find(|n| n.tie == t).map(radius).unwrap();
        assert!(ring(Tie::Molecule) < ring(Tie::Class));
        assert!(ring(Tie::Class) < ring(Tie::Interaction));
    }

    /// **Le cercle d'une ordonnance** : chaque ligne à sa place, toutes
    /// au même rayon, la première en haut, dans le sens des aiguilles.
    ///
    /// L'ordre reçu est l'ordre gardé : un cercle dont les membres
    /// changeraient de place entre deux ouvertures serait une image que
    /// personne ne peut apprendre.
    #[test]
    fn an_ordonnance_is_laid_out_as_one_ring() {
        assert!(super::circle(0).is_empty());
        // Une seule ligne : au centre, pas sur le bord.
        assert_eq!(super::circle(1), vec![(0.0, 0.0)]);
        let four = super::circle(4);
        assert_eq!(four.len(), 4);
        // La première en haut, puis à droite : le sens des aiguilles.
        assert!(four[0].0.abs() < 1e-5 && four[0].1 < -0.99);
        assert!(four[1].0 > 0.99 && four[1].1.abs() < 1e-5);
        assert!(four[2].1 > 0.99);
        assert!(four[3].0 < -0.99);
        // Toutes sur le cercle unité, et aucune sur une autre.
        for n in 2..40_usize {
            let ring = super::circle(n);
            assert_eq!(ring.len(), n);
            for (i, (x, y)) in ring.iter().enumerate() {
                assert!(
                    (x.hypot(*y) - 1.0).abs() < 1e-5,
                    "n={n} i={i} hors du cercle"
                );
                for (j, (u, v)) in ring.iter().enumerate() {
                    if i != j {
                        assert!((x - u).hypot(y - v) > 1e-3, "n={n} : {i} et {j} confondus");
                    }
                }
            }
        }
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
            toxicity_noted: false,
        };
        let caps = Caps {
            molecule: 8,
            class: 12,
            interaction: 8,
            total: 28,
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
    /// **En ouvrir un de plus ne déplace pas les autres.**
    ///
    /// C'est la condition pour que le grossissement serve de réglage
    /// entre « tout voir » et « tout lire » : l'angle valait
    /// `i / dessinés`, si bien qu'un membre de plus les faisait tous
    /// tourner — une figure qui tourne pendant qu'on la règle est une
    /// figure qu'on ne peut pas suivre. Et ce qu'on voyait, on le voit
    /// encore : les membres gardés à un plafond le sont à tous les
    /// plafonds plus larges.
    #[test]
    fn opening_one_more_place_moves_none_of_the_others() {
        let names: Vec<String> = (0..30).map(|i| format!("AINS {i:02}")).collect();
        let dcis: Vec<String> = (0..30).map(|i| format!("molécule {i:02}")).collect();
        let b: Vec<Known> = (0..30)
            .map(|i| card(i as i64 + 1, &names[i], &dcis[i], "AINS"))
            .collect();
        let centre = Known {
            id: 99,
            name: "Advil",
            dci: "ibuprofène",
            class: "AINS",
            ddi: "",
            toxicity_noted: false,
        };
        let at = |total: usize| {
            around(
                &centre,
                &b,
                Caps {
                    molecule: 40,
                    class: 40,
                    interaction: 40,
                    total,
                },
            )
        };
        for total in 3..28 {
            let small = at(total);
            let big = at(total + 1);
            assert!(
                big.nodes.len() >= small.nodes.len(),
                "{total} : élargir le plafond a retiré du monde"
            );
            for n in &small.nodes {
                let same =
                    big.nodes.iter().find(|m| m.id == n.id).unwrap_or_else(|| {
                        panic!("{total} : « {} » a disparu en élargissant", n.name)
                    });
                assert!(
                    (same.x - n.x).abs() < 1e-5 && (same.y - n.y).abs() < 1e-5,
                    "{total} : « {} » s'est déplacé de ({}, {}) à ({}, {})",
                    n.name,
                    n.x,
                    n.y,
                    same.x,
                    same.y
                );
            }
        }
    }

    /// **Les places s'ouvrent réparties, et jamais l'une au détriment
    /// de l'autre.**
    ///
    /// Garder « les douze premiers par ordre alphabétique » d'une classe
    /// de trente en dessinerait douze côte à côte sur un arc et rien
    /// ailleurs : on lirait douze noms et on croirait avoir vu la
    /// classe.
    #[test]
    fn the_places_of_a_ring_open_spread_out() {
        for n in 1..40_usize {
            let order = super::spread_order(n);
            // Une permutation : chaque place une fois et une seule.
            let mut seen = order.clone();
            seen.sort_unstable();
            assert_eq!(seen, (0..n).collect::<Vec<_>>(), "n={n}");
            // Tout préfixe est réparti : le plus grand trou du cadran
            // reste de l'ordre de ce qu'on peut en attendre. Deux fois
            // l'écart régulier, plus une place, laisse de la marge au
            // demi-pas sans laisser passer un préfixe groupé — qui
            // ferait, lui, un trou de presque tout le tour.
            for k in 1..=n {
                let mut taken: Vec<usize> = order[..k].to_vec();
                taken.sort_unstable();
                let gap = taken
                    .windows(2)
                    .map(|w| w[1] - w[0])
                    .chain(std::iter::once(n - taken[k - 1] + taken[0]))
                    .max()
                    .unwrap_or(n);
                assert!(
                    gap <= 2 * n / k + 2,
                    "n={n}, k={k} : un trou de {gap} places sur {n}"
                );
            }
        }
        assert!(super::spread_order(0).is_empty());
    }

    /// **Grossir garde sous le pointeur ce qui y était.**
    ///
    /// C'est la seule façon dont une molette se lit : grossir autour du
    /// milieu fait glisser sous les doigts ce qu'on regardait, et on
    /// passe son temps à rattraper la figure. Le test le dit comme on le
    /// voit — un point du monde, là où il se dessine avant et après.
    #[test]
    fn zooming_keeps_what_is_under_the_pointer_where_it_was() {
        // Où un point du cercle unité se dessine, pour un regard donné.
        let draw = |l: Look, w: (f32, f32)| {
            (
                l.pan.0 + w.0 * 100.0 * l.zoom,
                l.pan.1 + w.1 * 60.0 * l.zoom,
            )
        };
        let start = Look::default();
        // Le point sous le pointeur, désigné par où il se dessine.
        let pointer = (40.0_f32, -18.0_f32);
        // Le point du monde qui s'y trouve, au regard de départ.
        let world = (
            (pointer.0 - start.pan.0) / (100.0 * start.zoom),
            (pointer.1 - start.pan.1) / (60.0 * start.zoom),
        );
        for factor in [1.25_f32, 0.8, 2.0, 1.0 / 3.0] {
            let after = start.zoom_about(factor, pointer);
            let (x, y) = draw(after, world);
            assert!(
                (x - pointer.0).abs() < 0.01 && (y - pointer.1).abs() < 0.01,
                "×{factor} : le point est parti de ({}, {}) à ({x}, {y})",
                pointer.0,
                pointer.1
            );
        }
        // Et en enchaînant les crans, ce qui est le vrai usage.
        let mut l = start;
        for _ in 0..5 {
            l = l.zoom_about(Look::STEP, pointer);
        }
        let (x, y) = draw(l, world);
        assert!((x - pointer.0).abs() < 0.05 && (y - pointer.1).abs() < 0.05);
        // Un cran en avant et un cran en arrière rendent au départ :
        // c'est pour cela que le pas est multiplicatif.
        let there_and_back = start
            .zoom_about(Look::STEP, pointer)
            .zoom_about(1.0 / Look::STEP, pointer);
        assert!((there_and_back.zoom - 1.0).abs() < 1e-4);
        assert!(there_and_back.pan.0.abs() < 0.01 && there_and_back.pan.1.abs() < 0.01);
    }

    /// **Le grossissement est borné, et contre la borne le décalage
    /// s'arrête aussi.**
    ///
    /// Sans quoi, molettant contre la butée, la figure continuerait de
    /// filer alors que sa taille ne bouge plus — on la perdrait sans
    /// avoir rien grossi.
    #[test]
    fn zoom_stops_at_its_bounds_and_so_does_the_pan() {
        let mut l = Look::default();
        for _ in 0..40 {
            l = l.zoom_about(Look::STEP, (50.0, 0.0));
        }
        assert!((l.zoom - Look::MAX).abs() < 1e-4, "{l:?}");
        let against = l.zoom_about(Look::STEP, (50.0, 0.0));
        assert_eq!(against, l, "contre la borne, rien ne bouge");
        for _ in 0..40 {
            l = l.zoom_about(1.0 / Look::STEP, (50.0, 0.0));
        }
        assert!((l.zoom - Look::MIN).abs() < 1e-4, "{l:?}");
        // Un facteur qui n'en est pas un ne fait rien plutôt que
        // n'importe quoi : la vue lit une molette, et une molette
        // rend parfois zéro.
        let l = Look::default();
        assert_eq!(l.zoom_about(0.0, (1.0, 1.0)), l);
        assert_eq!(l.zoom_about(f32::NAN, (1.0, 1.0)), l);
        assert_eq!(l.zoom_about(f32::INFINITY, (1.0, 1.0)), l);
    }

    /// **On peut amener n'importe quel nœud au milieu, et on ne peut pas
    /// perdre la figure.**
    #[test]
    fn the_pan_reaches_every_node_and_never_loses_the_figure() {
        let half = (300.0_f32, 115.0_f32);
        let radius = (184.0_f32, 78.0_f32);
        // À taille normale la figure est plus petite que son creux : la
        // borne est celle du creux, sans quoi on ne la déplacerait pas.
        let l = Look {
            zoom: 1.0,
            pan: (10_000.0, -10_000.0),
        }
        .clamp_pan(half, radius);
        assert_eq!(l.pan, (half.0, -half.1));
        // Grossie quatre fois, le nœud le plus à droite est à
        // `4 × 184` du milieu : il faut pouvoir le ramener.
        let l = Look {
            zoom: 4.0,
            pan: (-10_000.0, 0.0),
        }
        .clamp_pan(half, radius);
        assert!(
            l.pan.0 <= -radius.0 * 4.0,
            "on n'atteint pas le bord de la figure : {l:?}"
        );
        // Et jamais au delà : la figure ne sort pas de vue.
        assert!(l.pan.0 >= -radius.0 * 4.0 - 0.01);
        // Un creux ou un rayon qui n'en sont pas ne font pas partir le
        // décalage à l'infini.
        let l = Look {
            zoom: 1.0,
            pan: (50.0, 50.0),
        }
        .clamp_pan((f32::NAN, 0.0), (f32::INFINITY, 0.0));
        assert_eq!(l.pan, (0.0, 0.0));
        // La vue « telle que le volet la pose » se reconnaît.
        assert!(Look::default().is_plain());
        assert!(!Look {
            zoom: 1.6,
            pan: (0.0, 0.0)
        }
        .is_plain());
        assert!(!Look {
            zoom: 1.0,
            pan: (40.0, 0.0)
        }
        .is_plain());
    }

    /// **Une fiche coupée par un plafond ne réapparaît pas sur l'anneau
    /// d'à côté.**
    ///
    /// Une fiche appartient à l'anneau le plus proche pour lequel elle
    /// se qualifie, que cet anneau ait de la place pour elle ou non.
    /// `taken` n'était rempli qu'avec les gardées : un voisin de classe
    /// que le plafond avait coupé ressortait donc sur l'anneau des
    /// interactions, peint dans l'ocre d'un lien qu'il n'a pas, pendant
    /// que le pied le comptait toujours parmi les voisins de classe non
    /// dessinés. Il n'apparaissait qu'une fois, si bien que rien dans
    /// l'image ne le montrait — sinon une réponse fausse à la question
    /// « à quel titre ».
    #[test]
    fn a_card_the_cap_cut_does_not_come_back_on_the_next_ring() {
        // Trois de la même classe, tous les trois nommés dans les
        // interactions du centre, et un plafond de classe de un.
        let names: Vec<String> = (0..3).map(|i| format!("Voisine {i}")).collect();
        let dcis: Vec<String> = (0..3).map(|i| format!("molécule {i}")).collect();
        let b: Vec<Known> = (0..3)
            .map(|i| card(i as i64 + 1, &names[i], &dcis[i], "AINS"))
            .collect();
        let centre = Known {
            id: 99,
            name: "Advil",
            dci: "ibuprofène",
            class: "AINS",
            ddi: "Éviter Voisine 0, Voisine 1 et Voisine 2.",
            toxicity_noted: false,
        };
        let map = around(
            &centre,
            &b,
            Caps {
                molecule: 8,
                class: 1,
                interaction: 8,
                total: 28,
            },
        );
        assert_eq!(map.count(Tie::Class), 1);
        // Les deux coupées ne sont **pas** redessinées en interaction :
        // elles sont de la même classe, et c'est ce que le pied dit.
        assert_eq!(map.count(Tie::Interaction), 0, "{:?}", map.nodes);
        assert_eq!(map.omitted_for(Tie::Class), 2);
        assert_eq!(map.omitted_for(Tie::Interaction), 0);
    }

    /// **Les trois anneaux se partagent la place, ils ne l'ont pas
    /// chacun.**
    ///
    /// Trois plafonds tenus séparément ne se voient pas les uns les
    /// autres, et les noms des trois anneaux s'écrivent dans les mêmes
    /// rangées : la carte du Durogesic posait vingt-deux noms dans la
    /// place de douze — cinq fentanyls, les opioïdes forts et les
    /// interactions, écrits les uns par-dessus les autres et à travers
    /// les rayons.
    #[test]
    fn the_three_rings_share_the_room_rather_than_each_having_it() {
        // Le partage seul, d'abord : c'est lui qui décide.
        assert_eq!(super::share([2, 3, 4], 28), [2, 3, 4], "rien à tailler");
        // Le plus fourni cède le premier, et on finit à égalité plutôt
        // qu'avec un anneau plein et un vide.
        assert_eq!(super::share([5, 9, 8], 12), [4, 4, 4]);
        assert_eq!(super::share([1, 11, 1], 6), [1, 4, 1]);
        // Aucun anneau non vide ne tombe à zéro, même quand la place
        // manque : une note qui parle d'un anneau qu'on ne voit pas ne
        // dit rien à personne.
        assert_eq!(super::share([4, 4, 4], 2), [1, 1, 1]);
        // Un anneau vide reste vide et ne se voit rien attribuer.
        assert_eq!(super::share([0, 9, 0], 4), [0, 4, 0]);

        // Et de bout en bout : trente voisins de classe, cinq de
        // molécule, une place de douze.
        let names: Vec<String> = (0..30).map(|i| format!("AINS {i:02}")).collect();
        let dcis: Vec<String> = (0..30).map(|i| format!("molécule {i:02}")).collect();
        let mut b: Vec<Known> = (0..30)
            .map(|i| card(i as i64 + 1, &names[i], &dcis[i], "AINS"))
            .collect();
        for k in b.iter_mut().take(5) {
            k.dci = "ibuprofène";
        }
        let centre = Known {
            id: 99,
            name: "Advil",
            dci: "ibuprofène",
            class: "AINS",
            ddi: "",
            toxicity_noted: false,
        };
        let caps = Caps {
            total: 12,
            ..Caps::default()
        };
        let map = around(&centre, &b, caps);
        assert_eq!(map.nodes.len(), 12, "{:?}", map.nodes.len());
        // Et ce qui n'est pas dessiné est dit, anneau par anneau : la
        // somme des deux comptes est ce qui manque.
        assert_eq!(map.count(Tie::Molecule) + map.omitted_for(Tie::Molecule), 5);
        assert_eq!(map.count(Tie::Class) + map.omitted_for(Tie::Class), 25);
    }

    /// **Un anneau ne prend que ce que la place permet d'écrire.**
    ///
    /// Douze est un nombre, et un nombre ne connaît pas le volet où il
    /// se dessine : à 1024x700 et `text_scale = 1,6`, six des douze noms
    /// de l'anneau de classe n'étaient pas peints du tout. Le plafond
    /// suit donc la demi-hauteur de la figure et la hauteur d'une ligne.
    #[test]
    fn a_ring_takes_only_what_there_is_room_to_name() {
        let full = Caps::default();
        // Un grand volet : rien n'est rogné, le plafond de lecture tient.
        let roomy = full.for_room(300.0, 16.0);
        assert_eq!(roomy.class, full.class);
        assert_eq!(roomy.interaction, full.interaction);
        assert_eq!(roomy.molecule, full.molecule);
        // Le volet mesuré, celui qui a montré le défaut.
        let tight = full.for_room(85.0, 27.0);
        assert!(
            tight.class < full.class,
            "douze noms sur un anneau de soixante pixels : {tight:?}"
        );
        // Jamais rien en dessous de trois : un anneau réduit à un point
        // n'est plus un anneau, et ce qu'il ne prend pas se dit dessous.
        let crushed = full.for_room(4.0, 40.0);
        assert_eq!((crushed.molecule, crushed.class), (3, 3));
        // À plafond de lecture égal, l'anneau le plus large en prend le
        // plus : c'est lui qui a le plus de hauteur à répartir. Dit avec
        // des plafonds égaux, sinon ce sont eux qu'on mesure — celui de
        // l'interaction vaut huit et celui de la classe douze, si bien
        // que le plus large peut légitimement en porter moins.
        let even = Caps {
            molecule: 40,
            class: 40,
            interaction: 40,
            total: 120,
        }
        .for_room(85.0, 27.0);
        assert!(
            even.interaction > even.class && even.class > even.molecule,
            "{even:?}"
        );
        assert!(tight.class >= tight.molecule);
        // Et cela ne recule jamais quand la place grandit.
        let mut last = 0;
        for h in 1..40 {
            let n = full.for_room(h as f32 * 10.0, 20.0).class;
            assert!(n >= last, "h={h} : {n} après {last}");
            last = n;
        }
        // Une place ou une ligne absurde ne fait pas plier le plafond.
        assert_eq!(full.for_room(0.0, 20.0).class, full.class);
        assert_eq!(full.for_room(200.0, 0.0).class, full.class);
    }

    /// **L'anneau de la classe lit le référentiel, jamais le libellé.**
    ///
    /// Le champ `class` d'une fiche est du texte libre, et il a dérivé :
    /// c'est toute la raison d'être de `classes.rs`. Cette carte-ci
    /// comparait les chaînes, si bien que Fosamax et Actonel n'étaient
    /// pas du même anneau — une lettre, « bisphosphonate » contre
    /// « biphosphonate » — et Cimzia n'était pas de celui d'Amgevita —
    /// un mot, « anti-TNF » contre « anti-TNF alpha ».
    ///
    /// Le confronter à la base livrée est ce qui donne le chiffre :
    /// 706 paires de voisins sur 331 fiches. Le test tient les trois
    /// dérives que `classes.rs` mesure déjà, et en plus le **sens
    /// inverse** — deux classes que le référentiel sépare ne se
    /// rejoignent pas sur la carte.
    #[test]
    fn the_class_ring_reads_the_referential_and_not_the_label() {
        let known: Vec<Known> = crate::db::STARTER_DRUGS
            .iter()
            .enumerate()
            .map(|(i, (name, dci, class, _antidote))| Known {
                id: i as i64 + 1,
                name,
                dci,
                class,
                ddi: "",
                toxicity_noted: false,
            })
            .collect();
        let ring_of = |name: &str| -> Vec<String> {
            let centre = known
                .iter()
                .find(|k| k.name == name)
                .unwrap_or_else(|| panic!("fiche absente de la base livrée : {name}"));
            around(centre, &known, Caps::default())
                .nodes
                .into_iter()
                .filter(|n| n.tie == Tie::Class)
                .map(|n| n.name)
                .collect()
        };
        // Une lettre, un mot, un trait d'union : les trois dérives que
        // `classes.rs` a mesurées, vues d'ici.
        assert!(ring_of("Fosamax").contains(&"Actonel".to_owned()));
        assert!(ring_of("Cimzia").contains(&"Amgevita".to_owned()));
        assert!(ring_of("Cardensiel").contains(&"Avlocardyl".to_owned()));
        // Et l'inverse : une classe voisine n'est pas la même classe.
        // Sans lui, ce test passerait aussi avec un anneau qui prend
        // tout le monde.
        assert!(!ring_of("Fosamax").contains(&"Coversyl".to_owned()));
        // Enfin, la règle elle-même sur toute la base : deux fiches sont
        // du même anneau exactement quand le référentiel les dit de la
        // même classe.
        for centre in &known {
            if centre.class.trim().is_empty() {
                continue;
            }
            let map = around(centre, &known, Caps::default());
            for n in map.nodes.iter().filter(|n| n.tie == Tie::Class) {
                let other = known.iter().find(|k| k.id == n.id).unwrap();
                assert!(
                    crate::classes::same(centre.class, other.class),
                    "{} [{}] n'est pas de la classe de {} [{}]",
                    other.name,
                    other.class,
                    centre.name,
                    centre.class
                );
            }
        }
    }

    /// **Un kétoconazole local n'est pas un kétoconazole**, et la carte
    /// ne le met pas face à un anticoagulant.
    ///
    /// La section « interactions » d'un AOD nomme les azolés — elle
    /// parle des azolés **généraux**. Lue au mot près, elle faisait du
    /// Kétoderm, qui est un shampooing, un voisin cité de l'Eliquis :
    /// le genre de lien qui apprend à ignorer les liens, et c'est
    /// exactement ce que `cyp.rs` refuse depuis toujours.
    ///
    /// La parenté de **molécule** n'est pas concernée : un kétoconazole
    /// topique et un kétoconazole oral sont la même molécule, et le voir
    /// vaut la peine.
    #[test]
    fn a_topical_is_not_a_cited_interaction() {
        let mut b = base();
        // La classe **que la fiche livrée porte vraiment** : elle dit
        // « antifongique local » et non « topique », et c'est ce seul
        // mot qui faisait échapper dix-neuf boîtes au filtre.
        b.push(card(8, "Kétoderm", "kétoconazole", "antifongique local"));
        let map = around(&b[0], &b, Caps::default());
        let cited: Vec<&str> = map
            .nodes
            .iter()
            .filter(|n| n.tie == Tie::Interaction)
            .map(|n| n.name.as_str())
            .collect();
        assert!(
            !cited.contains(&"Kétoderm"),
            "un shampooing n'est pas une interaction citée : {cited:?}"
        );
        // Le Nizoral, lui, est bien cité : c'est le même mot, et c'est
        // la voie générale.
        assert!(cited.contains(&"Nizoral"), "{cited:?}");

        // Et si le centre est lui-même topique, deux topiques se citent.
        // Avec une **autre** molécule que la sienne : le même
        // kétoconazole tomberait dans l'anneau de la molécule, qui est
        // plus proche, et une fiche n'appartient qu'à un anneau.
        let ketoderm = Known {
            id: 8,
            name: "Kétoderm",
            dci: "kétoconazole",
            class: "antifongique local",
            ddi: "À ne pas appliquer en même temps que le Diprosone.",
            toxicity_noted: false,
        };
        let mut b2 = base();
        b2.insert(0, ketoderm);
        b2.push(card(
            9,
            "Diprosone",
            "bétaméthasone",
            "dermocorticoïde fort",
        ));
        let map = around(&b2[0], &b2, Caps::default());
        assert!(
            map.nodes
                .iter()
                .any(|n| n.tie == Tie::Interaction && n.name == "Diprosone"),
            "un centre topique cite encore le topique qu'il nomme"
        );
    }

    /// **Ce que les tables savent entre dans la carte.** Une fiche que
    /// rien ne cite mais qu'une règle relie au centre rejoint l'anneau des
    /// interactions, avec ses raisons ; un voisin de classe qu'une règle
    /// relie aussi reste dans son anneau, sans raison — il se prend à la
    /// place du centre, pas avec lui.
    #[test]
    fn a_card_the_tables_tie_to_the_centre_joins_the_interaction_ring() {
        let b = base();
        let effect = Why::Effect {
            title: "Deux anticoagulants".to_owned(),
            detail: "saignement".to_owned(),
            alert: true,
        };
        let mut reasons = std::collections::HashMap::new();
        reasons.insert(7, vec![effect.clone()]);
        reasons.insert(3, vec![effect.clone()]);
        let map = around_with(&b[0], &b, Caps::default(), &reasons);
        let previscan = map.nodes.iter().find(|n| n.id == 7).expect("sur la carte");
        assert_eq!(previscan.tie, Tie::Interaction);
        assert_eq!(previscan.weight, 3);
        assert_eq!(previscan.why, vec![effect]);
        let xarelto = map.nodes.iter().find(|n| n.id == 3).unwrap();
        assert_eq!(xarelto.tie, Tie::Class);
        assert!(xarelto.why.is_empty() && xarelto.weight == 0);
        // Without the reasons, the Previscan is nobody's neighbour.
        assert!(!around(&b[0], &b, Caps::default())
            .nodes
            .iter()
            .any(|n| n.id == 7));
    }

    /// **Un anneau coupé garde ce qui pèse.** Couper une
    /// contre-indication pour garder une précaution d'emploi serait
    /// l'anneau qui ment.
    #[test]
    fn a_cut_ring_keeps_the_heaviest() {
        let mut b = vec![card(1, "Centre", "centre", "classe a")];
        let names: Vec<String> = (0..10).map(|i| format!("Fiche {i:02}")).collect();
        for (i, n) in names.iter().enumerate() {
            b.push(card(10 + i as i64, n, "", "autre"));
        }
        let mut reasons = std::collections::HashMap::new();
        for i in 0..10 {
            let why = Why::Cited {
                by: "Centre".to_owned(),
                sentence: if i == 7 {
                    "Association contre-indiquée.".to_owned()
                } else {
                    "À prendre en compte.".to_owned()
                },
            };
            reasons.insert(10 + i, vec![why]);
        }
        let caps = Caps {
            interaction: 3,
            total: 3,
            ..Caps::default()
        };
        let map = around_with(&b[0], &b, caps, &reasons);
        assert_eq!(map.count(Tie::Interaction), 3);
        assert!(
            map.nodes.iter().any(|n| n.id == 17),
            "la contre-indication reste"
        );
        assert_eq!(map.omitted_for(Tie::Interaction), 7);
    }

    /// Le poids d'une raison : le mot du thésaurus que la fiche écrit,
    /// la règle d'alerte de la revue — **jamais une enzyme seule**.
    #[test]
    fn a_reason_weighs_by_its_own_words_and_an_enzyme_never_reaches_the_top() {
        let cited = |t: &str| Why::Cited {
            by: "X".to_owned(),
            sentence: t.to_owned(),
        };
        assert_eq!(cited("Association contre-indiquée.").weight(), 3);
        assert_eq!(cited("Association DÉCONSEILLÉE avec l'AVK.").weight(), 2);
        assert_eq!(cited("Précaution d'emploi.").weight(), 2);
        assert_eq!(cited("Surveiller l'INR.").weight(), 2);
        let enzyme = |minor| Why::Enzyme {
            actor: "Clarithromycine".to_owned(),
            enzyme: "CYP3A4".to_owned(),
            shift: "exposition augmentée".to_owned(),
            minor,
        };
        assert_eq!(enzyme(false).weight(), 2);
        assert_eq!(enzyme(true).weight(), 1);
        assert_eq!(weight_of(&[]), 1);
        assert_eq!(weight_of(&[enzyme(false), cited("contre-indiqué")]), 3);
    }

    /// **Une place par raison avant une seconde.** Cinq fiches
    /// rencontrent le centre par la même règle, une sixième par une
    /// autre, et l'anneau n'a que trois places : la sixième en a une.
    #[test]
    fn a_cut_ring_gives_each_reason_a_place_before_a_second() {
        let mut b = vec![card(1, "Centre", "centre", "classe a")];
        let names: Vec<String> = (0..6).map(|i| format!("Fiche {i}")).collect();
        for (i, n) in names.iter().enumerate() {
            b.push(card(10 + i as i64, n, "", "autre"));
        }
        let rule = |title: &str| Why::Effect {
            title: title.to_owned(),
            detail: String::new(),
            alert: true,
        };
        let mut reasons = std::collections::HashMap::new();
        for i in 0..5 {
            reasons.insert(10 + i, vec![rule("Anticoagulant + AINS")]);
        }
        reasons.insert(15, vec![rule("Lithium exposé")]);
        let caps = Caps {
            interaction: 3,
            total: 3,
            ..Caps::default()
        };
        let map = around_with(&b[0], &b, caps, &reasons);
        assert!(
            map.nodes.iter().any(|n| n.id == 15),
            "le lithium a sa place"
        );
        assert_eq!(map.count(Tie::Interaction), 3);
    }

    /// **Une corde par paire, et une règle de trois lignes en relie
    /// chacune aux deux autres.** Les plus lourdes d'abord ; une ligne
    /// qui ne rencontre rien est nommée comme telle.
    #[test]
    fn an_ordonnance_map_draws_one_chord_per_pair_heaviest_first() {
        let triade = Why::Effect {
            title: "Triade néfaste".to_owned(),
            detail: String::new(),
            alert: true,
        };
        let cited = Why::Cited {
            by: "A".to_owned(),
            sentence: "Surveiller.".to_owned(),
        };
        let found = vec![
            (vec![0, 1, 2], triade.clone()),
            (vec![3, 0], cited.clone()),
            // The same reason twice on one pair counts once.
            (vec![0, 3], cited.clone()),
            // A rank outside the list is ignored.
            (vec![1, 9], cited.clone()),
        ];
        let (c, alone) = chords(5, &found);
        let pairs: Vec<(usize, usize, u8)> = c.iter().map(|c| (c.a, c.b, c.weight)).collect();
        assert_eq!(pairs, vec![(0, 1, 3), (0, 2, 3), (1, 2, 3), (0, 3, 2)]);
        assert_eq!(c[3].why, vec![cited]);
        assert_eq!(alone, vec![4]);
        let (none, all) = chords(2, &[]);
        assert!(none.is_empty());
        assert_eq!(all, vec![0, 1]);
    }
}
