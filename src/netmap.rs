//! La carte des connexions : ce poste au centre, les autres postes de
//! l'officine sur un premier cercle, les officines du réseau sur un
//! second, et chaque lien dans l'état où il est.
//!
//! **Un état se lit à la forme autant qu'à la couleur** : un lien qui
//! répond est plein, un lien qu'on n'entend que par le dossier d'échange
//! est fin, un lien qui échoue est tireté, un lien muet est pointillé. Une
//! couleur seule ne dit rien à qui ne la voit pas.
//!
//! Pur et testé : les nœuds, leur état, leur place. L'écran peint et
//! reçoit les clics (`App::conn_map`).

/// Ce qu'est un nœud.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeKind {
    /// Ce poste.
    Me,
    /// Un autre poste de l'officine.
    Post,
    /// Une officine du réseau.
    Officine,
    /// Une officine qui s'annonce sur le réseau local et n'est d'aucun
    /// réseau de celle-ci : **aucun lien** n'est dessiné vers elle, il
    /// n'y en a pas.
    Nearby,
    /// Une officine d'un réseau d'ici, qu'une autre y a fait entrer et
    /// qu'on n'a pas ajoutée : sur l'arc de son réseau, sans lien.
    Introduced,
}

/// L'état d'un lien, du meilleur au pire.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LinkState {
    /// La dernière conversation directe a réussi.
    Ok,
    /// Des nouvelles, mais pas de conversation directe réussie (dossier
    /// d'échange, poste entendu sur le réseau local).
    Heard,
    /// La dernière tentative a échoué.
    Failing,
    /// Rien entendu.
    Silent,
}

/// Un nœud de la carte.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    /// Unique sur la carte : `me`, `post:<n>`, `net:<empreinte>`.
    pub key: String,
    pub kind: NodeKind,
    pub label: String,
    /// L'état du lien avec ce poste-ci ; `Ok` pour ce poste lui-même.
    pub state: LinkState,
    /// L'empreinte de l'appareil (hex), pour agir sur lui.
    pub device: String,
    /// Le réseau où la carte range une officine (son rang dans la liste
    /// des réseaux) ; 0 pour ce poste et les postes.
    pub group: usize,
}

/// Ce qu'on sait d'un poste de l'officine.
pub struct PostIn<'a> {
    pub post: i64,
    pub device: &'a str,
    pub name: &'a str,
    /// Retiré du groupe : il ne figure plus sur la carte.
    pub left: bool,
}

/// Ce que le réseau local a entendu : un appareil, et la dernière
/// conversation avec lui.
pub struct HeardIn<'a> {
    pub device: &'a str,
    pub talked: Option<bool>,
}

/// Ce qu'on sait d'une officine du réseau.
pub struct OfficineIn<'a> {
    pub device: &'a str,
    pub name: &'a str,
    pub seen_as: &'a str,
    pub received: i64,
    pub last_ok: &'a str,
    pub last_try: &'a str,
    pub last_error: &'a str,
    /// Le réseau où la ranger (rang dans la liste des réseaux).
    pub group: usize,
}

/// Une officine présentée par son réseau.
pub struct IntroIn<'a> {
    pub device: &'a str,
    pub name: &'a str,
    /// Le rang de son réseau, comme [`OfficineIn::group`].
    pub group: usize,
}

/// Une officine entendue sur le réseau local.
pub struct NearIn<'a> {
    pub device: &'a str,
    pub name: &'a str,
    /// Elle a une invitation ouverte.
    pub inviting: bool,
}

/// L'état du lien avec une officine, d'après ce que `net_peers` garde.
pub fn officine_state(o: &OfficineIn<'_>) -> LinkState {
    let failed = !o.last_error.trim().is_empty() && o.last_try >= o.last_ok;
    if failed {
        LinkState::Failing
    } else if !o.last_ok.trim().is_empty() {
        LinkState::Ok
    } else if o.received > 0 {
        LinkState::Heard
    } else {
        LinkState::Silent
    }
}

/// L'état du lien avec un poste, d'après ce que le réseau local a entendu.
pub fn post_state(device: &str, heard: &[HeardIn<'_>]) -> LinkState {
    match heard.iter().find(|h| h.device == device) {
        Some(h) => match h.talked {
            Some(true) => LinkState::Ok,
            Some(false) => LinkState::Failing,
            None => LinkState::Heard,
        },
        None => LinkState::Silent,
    }
}

/// Les nœuds de la carte : ce poste, les autres postes du groupe (sans
/// ceux qui l'ont quitté), puis les officines — chacun avec son état —,
/// puis les officines voisines qui ne sont d'aucun réseau d'ici, sur un
/// arc à elles, le rang qui suit le dernier réseau.
#[allow(clippy::too_many_arguments)]
pub fn nodes(
    me_label: &str,
    my_post: Option<i64>,
    posts: &[PostIn<'_>],
    heard: &[HeardIn<'_>],
    officines: &[OfficineIn<'_>],
    introduced: &[IntroIn<'_>],
    nearby: &[NearIn<'_>],
    unnamed_post: &dyn Fn(i64) -> String,
    unnamed_officine: &dyn Fn(&str) -> String,
) -> Vec<Node> {
    let mut out = vec![Node {
        key: "me".to_owned(),
        kind: NodeKind::Me,
        label: me_label.to_owned(),
        state: LinkState::Ok,
        device: String::new(),
        group: 0,
    }];
    for p in posts.iter().filter(|p| !p.left && Some(p.post) != my_post) {
        out.push(Node {
            key: format!("post:{}", p.post),
            kind: NodeKind::Post,
            label: if p.name.trim().is_empty() {
                unnamed_post(p.post)
            } else {
                p.name.trim().to_owned()
            },
            state: post_state(p.device, heard),
            device: p.device.to_owned(),
            group: 0,
        });
    }
    // **Un réseau, un arc** : les officines rangées par réseau se suivent
    // sur le cercle extérieur — celles qu'on a ajoutées, puis celles que le
    // réseau présente.
    let mut sorted: Vec<&OfficineIn<'_>> = officines.iter().collect();
    sorted.sort_by_key(|o| o.group);
    let groups: Vec<usize> = {
        let mut g: Vec<usize> = officines
            .iter()
            .map(|o| o.group)
            .chain(introduced.iter().map(|i| i.group))
            .collect();
        g.sort_unstable();
        g.dedup();
        g
    };
    for g in groups {
        for o in sorted.iter().filter(|o| o.group == g) {
            let label = [o.name, o.seen_as]
                .into_iter()
                .map(str::trim)
                .find(|n| !n.is_empty())
                .map(str::to_owned)
                .unwrap_or_else(|| unnamed_officine(o.device));
            out.push(Node {
                key: format!("net:{}", o.device),
                kind: NodeKind::Officine,
                label,
                state: officine_state(o),
                device: o.device.to_owned(),
                group: o.group,
            });
        }
        for i in introduced.iter().filter(|i| i.group == g) {
            if out.iter().any(|x| x.device == i.device) {
                continue;
            }
            out.push(Node {
                key: format!("intro:{}:{}", i.group, i.device),
                kind: NodeKind::Introduced,
                label: if i.name.trim().is_empty() {
                    unnamed_officine(i.device)
                } else {
                    i.name.trim().to_owned()
                },
                state: LinkState::Heard,
                device: i.device.to_owned(),
                group: i.group,
            });
        }
    }
    let near_group = officines
        .iter()
        .map(|o| o.group + 1)
        .chain(introduced.iter().map(|i| i.group + 1))
        .max()
        .unwrap_or(0);
    for n in nearby {
        if out.iter().any(|x| x.device == n.device) {
            continue;
        }
        out.push(Node {
            key: format!("near:{}", n.device),
            kind: NodeKind::Nearby,
            label: if n.name.trim().is_empty() {
                unnamed_officine(n.device)
            } else {
                n.name.trim().to_owned()
            },
            state: if n.inviting {
                LinkState::Ok
            } else {
                LinkState::Heard
            },
            device: n.device.to_owned(),
            group: near_group,
        });
    }
    out
}

/// Le rang qu'occupent les officines voisines, quand il y en a.
pub fn nearby_group(nodes: &[Node]) -> Option<usize> {
    nodes
        .iter()
        .find(|n| n.kind == NodeKind::Nearby)
        .map(|n| n.group)
}

/// Le cercle extérieur : les officines des réseaux et les voisines.
fn outer(kind: NodeKind) -> bool {
    matches!(
        kind,
        NodeKind::Officine | NodeKind::Introduced | NodeKind::Nearby
    )
}

/// La place de chaque nœud, en fraction du carré qui porte la carte
/// (0 à 1 sur chaque axe) : ce poste au centre, les postes sur un cercle
/// de rayon 0,22, les officines sur un cercle de rayon 0,40. Chaque
/// cercle est réparti également, et le second décalé d'un demi-pas pour
/// que ses nœuds ne tombent pas derrière ceux du premier.
pub fn layout(nodes: &[Node]) -> Vec<(f32, f32)> {
    let ring = |kind: NodeKind| nodes.iter().filter(|n| n.kind == kind).count();
    let (n_posts, n_net) = (
        ring(NodeKind::Post),
        nodes.iter().filter(|n| outer(n.kind)).count(),
    );
    let (mut i_post, mut i_net) = (0usize, 0usize);
    let tau = std::f32::consts::TAU;
    nodes
        .iter()
        .map(|n| {
            let at = |i: usize, count: usize, radius: f32, shift: f32| {
                let a =
                    -std::f32::consts::FRAC_PI_2 + tau * (i as f32 + shift) / count.max(1) as f32;
                (0.5 + radius * a.cos(), 0.5 + radius * a.sin())
            };
            match n.kind {
                NodeKind::Me => (0.5, 0.5),
                NodeKind::Post => {
                    i_post += 1;
                    at(i_post - 1, n_posts, 0.22, 0.0)
                }
                NodeKind::Officine | NodeKind::Introduced | NodeKind::Nearby => {
                    i_net += 1;
                    at(i_net - 1, n_net, 0.40, 0.5)
                }
            }
        })
        .collect()
}

/// La direction de chaque réseau depuis le centre : l'angle moyen de ses
/// officines, en vecteur unité. Rend `(rang du réseau, ux, uy)` pour
/// chaque réseau qui a des officines sur la carte.
pub fn group_directions(nodes: &[Node], places: &[(f32, f32)]) -> Vec<(usize, f32, f32)> {
    let mut groups: Vec<usize> = nodes
        .iter()
        .filter(|n| outer(n.kind))
        .map(|n| n.group)
        .collect();
    groups.dedup();
    groups
        .into_iter()
        .map(|g| {
            let (sx, sy, k) = nodes
                .iter()
                .zip(places)
                .filter(|(n, _)| outer(n.kind) && n.group == g)
                .fold((0.0_f32, 0.0_f32, 0.0_f32), |(sx, sy, k), (_, (x, y))| {
                    (sx + (x - 0.5), sy + (y - 0.5), k + 1.0)
                });
            let (dx, dy) = (sx / k.max(1.0), sy / k.max(1.0));
            let len = dx.hypot(dy);
            // Un seul réseau qui fait tout le tour : son nom en haut.
            if len < 1e-3 {
                (g, 0.0, -1.0)
            } else {
                (g, dx / len, dy / len)
            }
        })
        .collect()
}

/// Les rayons où essayer le nom d'un réseau, dans l'ordre — au-delà du
/// cercle, sinon en dedans. **En bas, en dedans d'abord** : le nom d'un
/// nœud s'écrit sous lui, et celui du réseau posé au-delà tombait dessus.
/// L'écran prend le premier où le nom, mesuré, ne couvre rien.
pub fn label_radii(uy: f32) -> [f32; 5] {
    if uy > 0.3 {
        [0.31, 0.24, 0.47, 0.53, 0.58]
    } else {
        [0.47, 0.53, 0.58, 0.31, 0.24]
    }
}

/// Où écrire le nom de chaque réseau, sans rien mesurer : l'angle moyen
/// de ses officines, au premier rayon de [`label_radii`] qui s'écarte de
/// chaque nœud (mêmes unités que `layout`). Rend `(rang, x, y)`.
pub fn group_labels(nodes: &[Node], places: &[(f32, f32)]) -> Vec<(usize, f32, f32)> {
    group_directions(nodes, places)
        .into_iter()
        .map(|(g, ux, uy)| {
            let clear = |r: f32| {
                let (x, y) = (0.5 + ux * r, 0.5 + uy * r);
                places
                    .iter()
                    .all(|(px, py)| (px - x).hypot(py - y) > LABEL_CLEARANCE)
            };
            let tries = label_radii(uy);
            let radius = tries.into_iter().find(|r| clear(*r)).unwrap_or(tries[0]);
            (g, 0.5 + ux * radius, 0.5 + uy * radius)
        })
        .collect()
}

/// **Le nom court d'une officine**, quand le long ne tient pas : sans
/// « Pharmacie » ni l'article qui suit — « Pharmacie de la Gare » se lit
/// « Gare ». Couper à la fin donnait « Pharmacie de la… » partout, le
/// seul morceau que toutes les officines ont en commun. Un nom qui n'a
/// rien de tel reste tel quel.
pub fn short_name(label: &str) -> String {
    let t = label.trim();
    let lower = t.to_lowercase();
    let mut cut = 0;
    for head in ["grande pharmacie ", "pharmacie ", "officine "] {
        if lower.starts_with(head) {
            cut = head.len();
            break;
        }
    }
    if cut == 0 {
        return t.to_owned();
    }
    let rest = &t[cut..];
    let lower_rest = rest.to_lowercase();
    for article in ["de la ", "de l'", "de l’", "des ", "du ", "de ", "d'", "d’"] {
        if lower_rest.starts_with(article) && rest.len() > article.len() {
            return rest[article.len()..].trim().to_owned();
        }
    }
    let rest = rest.trim();
    if rest.is_empty() {
        t.to_owned()
    } else {
        rest.to_owned()
    }
}

/// Combien un nom de réseau se tient loin de tout nœud (unités de
/// `layout`) : la place d'un carré et du nom écrit sous lui.
const LABEL_CLEARANCE: f32 = 0.07;

/// Le nœud le plus proche d'un point (mêmes unités que `layout`), à moins
/// de `reach` — ce qu'un clic a touché.
pub fn hit(places: &[(f32, f32)], x: f32, y: f32, reach: f32) -> Option<usize> {
    places
        .iter()
        .enumerate()
        .map(|(i, (px, py))| (i, (px - x).hypot(py - y)))
        .filter(|(_, d)| *d <= reach)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(i, _)| i)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn officine<'a>(
        device: &'a str,
        ok: &'a str,
        try_: &'a str,
        err: &'a str,
        received: i64,
    ) -> OfficineIn<'a> {
        OfficineIn {
            device,
            name: "",
            seen_as: "",
            received,
            last_ok: ok,
            last_try: try_,
            last_error: err,
            group: 0,
        }
    }

    /// **L'état d'un lien** : l'échec le plus récent l'emporte ; une
    /// réussite postérieure l'efface ; des nouvelles sans conversation
    /// directe, c'est « entendu ».
    #[test]
    fn a_link_says_what_happened_last() {
        assert_eq!(
            officine_state(&officine("a", "", "", "", 0)),
            LinkState::Silent
        );
        assert_eq!(
            officine_state(&officine("a", "", "", "", 4)),
            LinkState::Heard
        );
        assert_eq!(
            officine_state(&officine(
                "a",
                "2026-09-24 10:00",
                "2026-09-24 10:00",
                "",
                4
            )),
            LinkState::Ok
        );
        assert_eq!(
            officine_state(&officine(
                "a",
                "2026-09-24 09:00",
                "2026-09-24 10:00",
                "pas de réponse",
                4
            )),
            LinkState::Failing
        );
        // Une erreur ancienne, une réussite depuis : le lien répond.
        assert_eq!(
            officine_state(&officine(
                "a",
                "2026-09-24 11:00",
                "2026-09-24 10:00",
                "pas de réponse",
                4
            )),
            LinkState::Ok
        );
        let heard = [
            HeardIn {
                device: "p1",
                talked: Some(true),
            },
            HeardIn {
                device: "p2",
                talked: None,
            },
            HeardIn {
                device: "p3",
                talked: Some(false),
            },
        ];
        assert_eq!(post_state("p1", &heard), LinkState::Ok);
        assert_eq!(post_state("p2", &heard), LinkState::Heard);
        assert_eq!(post_state("p3", &heard), LinkState::Failing);
        assert_eq!(post_state("p4", &heard), LinkState::Silent);
    }

    /// Ce poste au centre, pas deux fois ; les postes partis n'y sont
    /// plus ; un nœud sans nom en reçoit un.
    #[test]
    fn the_map_holds_each_node_once() {
        let posts = [
            PostIn {
                post: 1,
                device: "d1",
                name: "Comptoir 1",
                left: false,
            },
            PostIn {
                post: 2,
                device: "d2",
                name: "",
                left: false,
            },
            PostIn {
                post: 3,
                device: "d3",
                name: "Ancien",
                left: true,
            },
        ];
        let net = [
            officine("n1", "", "", "", 0),
            OfficineIn {
                name: "Pharmacie du Port",
                ..officine("n2", "", "", "", 1)
            },
            OfficineIn {
                seen_as: "Gare",
                ..officine("n3", "", "", "", 1)
            },
        ];
        let list = nodes(
            "Ce poste",
            Some(1),
            &posts,
            &[],
            &net,
            &[],
            &[],
            &|n| format!("Poste {n}"),
            &|d| format!("[{d}]"),
        );
        let keys: Vec<&str> = list.iter().map(|n| n.key.as_str()).collect();
        assert_eq!(keys, ["me", "post:2", "net:n1", "net:n2", "net:n3"]);
        let labels: Vec<&str> = list.iter().map(|n| n.label.as_str()).collect();
        assert_eq!(
            labels,
            ["Ce poste", "Poste 2", "[n1]", "Pharmacie du Port", "Gare"]
        );
    }

    /// **Un réseau, un arc** : les officines d'un même réseau se suivent
    /// sur le cercle, et le nom de chaque réseau se pose de son côté.
    #[test]
    fn each_network_takes_its_own_arc() {
        let net = [
            OfficineIn {
                group: 1,
                ..officine("b1", "", "", "", 0)
            },
            OfficineIn {
                group: 0,
                ..officine("a1", "", "", "", 0)
            },
            OfficineIn {
                group: 1,
                ..officine("b2", "", "", "", 0)
            },
            OfficineIn {
                group: 0,
                ..officine("a2", "", "", "", 0)
            },
        ];
        let list = nodes(
            "me",
            None,
            &[],
            &[],
            &net,
            &[],
            &[],
            &|n| n.to_string(),
            &|d| d.to_owned(),
        );
        let groups: Vec<usize> = list.iter().skip(1).map(|n| n.group).collect();
        assert_eq!(groups, [0, 0, 1, 1], "contigus");
        let places = layout(&list);
        let labels = group_labels(&list, &places);
        assert_eq!(labels.len(), 2);
        // Chaque nom est plus près des siens que des autres.
        for (g, x, y) in &labels {
            let near = |want: bool| {
                list.iter()
                    .zip(&places)
                    .filter(|(n, _)| n.kind == NodeKind::Officine && (n.group == *g) == want)
                    .map(|(_, (px, py))| (px - x).hypot(py - y))
                    .fold(f32::MAX, f32::min)
            };
            assert!(near(true) < near(false), "réseau {g}");
        }
    }

    /// Les places : au centre, sur deux cercles, dans le carré ; un clic
    /// touche le nœud le plus proche, et rien loin de tout.
    #[test]
    fn nodes_sit_on_two_rings_and_a_click_finds_the_nearest() {
        let posts = [
            PostIn {
                post: 2,
                device: "d2",
                name: "A",
                left: false,
            },
            PostIn {
                post: 3,
                device: "d3",
                name: "B",
                left: false,
            },
        ];
        let net = [
            officine("n1", "", "", "", 0),
            officine("n2", "", "", "", 0),
            officine("n3", "", "", "", 0),
        ];
        let list = nodes(
            "me",
            Some(1),
            &posts,
            &[],
            &net,
            &[],
            &[],
            &|n| n.to_string(),
            &|d| d.to_owned(),
        );
        let places = layout(&list);
        assert_eq!(places[0], (0.5, 0.5));
        for (n, (x, y)) in list.iter().zip(&places) {
            assert!((0.0..=1.0).contains(x) && (0.0..=1.0).contains(y), "{n:?}");
            let r = (x - 0.5).hypot(y - 0.5);
            match n.kind {
                NodeKind::Me => assert!(r < 1e-6),
                NodeKind::Post => assert!((r - 0.22).abs() < 1e-4),
                NodeKind::Officine | NodeKind::Introduced | NodeKind::Nearby => {
                    assert!((r - 0.40).abs() < 1e-4)
                }
            }
        }
        assert_eq!(hit(&places, 0.51, 0.5, 0.05), Some(0));
        assert_eq!(hit(&places, places[3].0 + 0.01, places[3].1, 0.05), Some(3));
        assert_eq!(hit(&places, 0.02, 0.98, 0.05), None);
        // Une carte vide de pairs : ce poste seul, au centre.
        assert_eq!(
            layout(&nodes(
                "me",
                None,
                &[],
                &[],
                &[],
                &[],
                &[],
                &|n| n.to_string(),
                &|d| d.to_owned()
            )),
            vec![(0.5, 0.5)]
        );
    }

    /// **Les officines voisines** : sur leur arc, après le dernier réseau ;
    /// une voisine déjà du réseau n'y figure qu'une fois, comme officine
    /// du réseau ; sans nom, son empreinte.
    #[test]
    fn nearby_officines_take_the_arc_after_the_networks() {
        let net = [
            OfficineIn {
                group: 1,
                ..officine("b1", "", "", "", 0)
            },
            officine("a1", "", "", "", 0),
        ];
        let near = [
            NearIn {
                device: "a1",
                name: "Déjà du réseau",
                inviting: false,
            },
            NearIn {
                device: "v1",
                name: "Pharmacie voisine",
                inviting: true,
            },
            NearIn {
                device: "v2",
                name: " ",
                inviting: false,
            },
        ];
        let list = nodes(
            "me",
            None,
            &[],
            &[],
            &net,
            &[],
            &near,
            &|n| n.to_string(),
            &|d| format!("[{d}]"),
        );
        let keys: Vec<&str> = list.iter().map(|n| n.key.as_str()).collect();
        assert_eq!(keys, ["me", "net:a1", "net:b1", "near:v1", "near:v2"]);
        assert_eq!(nearby_group(&list), Some(2));
        assert_eq!(list[3].state, LinkState::Ok, "invitation ouverte");
        assert_eq!(list[4].state, LinkState::Heard);
        assert_eq!(list[4].label, "[v2]");
        let places = layout(&list);
        let labels = group_labels(&list, &places);
        assert_eq!(labels.len(), 3, "deux réseaux et les voisines");
        // Toutes sur le cercle extérieur.
        for (n, (x, y)) in list.iter().zip(&places).skip(1) {
            assert!(((x - 0.5).hypot(y - 0.5) - 0.40).abs() < 1e-4, "{}", n.key);
        }
    }

    /// **Une officine présentée par son réseau** se range sur l'arc de ce
    /// réseau, après les officines ajoutées ; déjà ajoutée, elle n'y est
    /// qu'une fois ; les voisines viennent après le dernier réseau.
    #[test]
    fn introduced_officines_sit_on_their_network_arc() {
        let net = [
            officine("a1", "", "", "", 0),
            OfficineIn {
                group: 1,
                ..officine("b1", "", "", "", 0)
            },
        ];
        let intro = [
            IntroIn {
                device: "a2",
                name: "Pharmacie du Canal",
                group: 0,
            },
            IntroIn {
                device: "a1",
                name: "Déjà là",
                group: 0,
            },
            IntroIn {
                device: "c1",
                name: "",
                group: 2,
            },
        ];
        let near = [NearIn {
            device: "v1",
            name: "Voisine",
            inviting: false,
        }];
        let list = nodes(
            "me",
            None,
            &[],
            &[],
            &net,
            &intro,
            &near,
            &|n| n.to_string(),
            &|d| format!("[{d}]"),
        );
        let keys: Vec<&str> = list.iter().map(|n| n.key.as_str()).collect();
        assert_eq!(
            keys,
            [
                "me",
                "net:a1",
                "intro:0:a2",
                "net:b1",
                "intro:2:c1",
                "near:v1"
            ]
        );
        assert_eq!(list[2].kind, NodeKind::Introduced);
        assert_eq!(list[4].label, "[c1]");
        assert_eq!(nearby_group(&list), Some(3));
    }

    /// **Le nom d'un réseau ne se pose pas sur une officine** : trois
    /// officines dont celle du milieu tombe sur l'angle moyen — le nom va
    /// ailleurs, à distance de chaque nœud.
    #[test]
    fn a_network_name_does_not_sit_on_a_node() {
        let net = [
            officine("a1", "", "", "", 0),
            officine("a2", "", "", "", 0),
            officine("a3", "", "", "", 0),
            OfficineIn {
                group: 1,
                ..officine("b1", "", "", "", 0)
            },
            OfficineIn {
                group: 1,
                ..officine("b2", "", "", "", 0)
            },
        ];
        let list = nodes(
            "me",
            None,
            &[],
            &[],
            &net,
            &[],
            &[],
            &|n| n.to_string(),
            &|d| d.to_owned(),
        );
        let places = layout(&list);
        for (g, x, y) in group_labels(&list, &places) {
            for (n, (px, py)) in list.iter().zip(&places) {
                assert!(
                    (px - x).hypot(py - y) > LABEL_CLEARANCE,
                    "le nom du réseau {g} tombe sur {}",
                    n.key
                );
            }
        }
    }

    #[test]
    fn a_short_name_keeps_what_tells_officines_apart() {
        assert_eq!(short_name("Pharmacie de la Gare"), "Gare");
        assert_eq!(short_name("Pharmacie du Port"), "Port");
        assert_eq!(short_name("Pharmacie des Arceaux"), "Arceaux");
        assert_eq!(short_name("pharmacie de l'Église"), "Église");
        assert_eq!(short_name("Grande Pharmacie Centrale"), "Centrale");
        assert_eq!(short_name("Pharmacie Martin"), "Martin");
        assert_eq!(short_name("Officine du Marché"), "Marché");
        assert_eq!(short_name("Pharmacie"), "Pharmacie");
        assert_eq!(short_name("Comptoir 2"), "Comptoir 2");
        assert_eq!(short_name("  Pharmacie de  "), "de");
    }
}
