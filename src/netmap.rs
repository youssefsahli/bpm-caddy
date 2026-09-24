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
/// ceux qui l'ont quitté), puis les officines — chacun avec son état.
pub fn nodes(
    me_label: &str,
    my_post: Option<i64>,
    posts: &[PostIn<'_>],
    heard: &[HeardIn<'_>],
    officines: &[OfficineIn<'_>],
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
    // sur le cercle extérieur.
    let mut sorted: Vec<&OfficineIn<'_>> = officines.iter().collect();
    sorted.sort_by_key(|o| o.group);
    for o in sorted {
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
    out
}

/// La place de chaque nœud, en fraction du carré qui porte la carte
/// (0 à 1 sur chaque axe) : ce poste au centre, les postes sur un cercle
/// de rayon 0,22, les officines sur un cercle de rayon 0,40. Chaque
/// cercle est réparti également, et le second décalé d'un demi-pas pour
/// que ses nœuds ne tombent pas derrière ceux du premier.
pub fn layout(nodes: &[Node]) -> Vec<(f32, f32)> {
    let ring = |kind: NodeKind| nodes.iter().filter(|n| n.kind == kind).count();
    let (n_posts, n_net) = (ring(NodeKind::Post), ring(NodeKind::Officine));
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
                NodeKind::Officine => {
                    i_net += 1;
                    at(i_net - 1, n_net, 0.40, 0.5)
                }
            }
        })
        .collect()
}

/// Où écrire le nom de chaque réseau : l'angle moyen de ses officines,
/// un peu au-delà du cercle extérieur (mêmes unités que `layout`). Rend
/// `(rang du réseau, x, y)` pour chaque réseau qui a des officines.
pub fn group_labels(nodes: &[Node], places: &[(f32, f32)]) -> Vec<(usize, f32, f32)> {
    let mut groups: Vec<usize> = nodes
        .iter()
        .filter(|n| n.kind == NodeKind::Officine)
        .map(|n| n.group)
        .collect();
    groups.dedup();
    groups
        .into_iter()
        .map(|g| {
            let (sx, sy, k) = nodes
                .iter()
                .zip(places)
                .filter(|(n, _)| n.kind == NodeKind::Officine && n.group == g)
                .fold((0.0_f32, 0.0_f32, 0.0_f32), |(sx, sy, k), (_, (x, y))| {
                    (sx + (x - 0.5), sy + (y - 0.5), k + 1.0)
                });
            let (dx, dy) = (sx / k.max(1.0), sy / k.max(1.0));
            let len = dx.hypot(dy);
            // Un seul réseau qui fait tout le tour : son nom en haut.
            let (ux, uy) = if len < 1e-3 {
                (0.0, -1.0)
            } else {
                (dx / len, dy / len)
            };
            (g, 0.5 + ux * 0.47, 0.5 + uy * 0.47)
        })
        .collect()
}

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
        let list = nodes("me", None, &[], &[], &net, &|n| n.to_string(), &|d| {
            d.to_owned()
        });
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
        let list = nodes("me", Some(1), &posts, &[], &net, &|n| n.to_string(), &|d| {
            d.to_owned()
        });
        let places = layout(&list);
        assert_eq!(places[0], (0.5, 0.5));
        for (n, (x, y)) in list.iter().zip(&places) {
            assert!((0.0..=1.0).contains(x) && (0.0..=1.0).contains(y), "{n:?}");
            let r = (x - 0.5).hypot(y - 0.5);
            match n.kind {
                NodeKind::Me => assert!(r < 1e-6),
                NodeKind::Post => assert!((r - 0.22).abs() < 1e-4),
                NodeKind::Officine => assert!((r - 0.40).abs() < 1e-4),
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
                &|n| n.to_string(),
                &|d| d.to_owned()
            )),
            vec![(0.5, 0.5)]
        );
    }
}
