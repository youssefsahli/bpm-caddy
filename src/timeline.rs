//! Le fil du dossier : tout ce que la base sait de quelqu'un, dans
//! l'ordre des jours.
//!
//! Un dossier se lit aujourd'hui par onglets — les actes, les vaccins,
//! la biologie, les locations, les pièces —, et chacun répond bien à sa
//! question. Aucun ne répond à celle qu'on pose en ouvrant la fiche de
//! quelqu'un qu'on n'a pas vu depuis six mois : **qu'est-ce qui s'est
//! passé, et quand ?** La dernière délivrance de stupéfiant est derrière
//! un onglet, le dernier vaccin derrière un autre, le dernier résultat
//! de laboratoire derrière un troisième ; chacun est une ligne, et il
//! faut cliquer trois fois pour lire trois lignes.
//!
//! Ce module fond ces sources en une seule suite datée. Il ne connaît
//! aucune d'elles : l'appelant lui passe des [`Event`], et c'est
//! délibéré — la base sait où vivent les vaccinations, ce module sait ce
//! qu'est un fil, et le jour où l'on ajoutera une source il n'y aura
//! rien à changer ici.
//!
//! Pur et testé, comme `revue` et `ordonnancier`. Aucune horloge : le
//! jour est donné.
//!
//! Et **rien ici ne s'écrit** : le fil est une lecture, refaite à chaque
//! chargement de dossier à partir des tables qui, elles, sont la
//! vérité. C'est pourquoi [`Kind`] n'a pas de clé de base — les autres
//! modules en ont une parce que leur valeur part au disque et doit se
//! relire dans dix ans ; celle-ci ne sort jamais de la mémoire.

/// D'où vient une ligne du fil.
///
/// Une nature et non un simple libellé, parce que trois choses en
/// dépendent : la couleur du repère, l'ordre du résumé, et le filtre.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    /// Un entretien, un bilan, un TROD.
    Entretien,
    Vaccin,
    /// Un résultat de laboratoire.
    Biologie,
    /// Une délivrance inscrite au registre des stupéfiants.
    Stupefiant,
    /// Une location de matériel.
    Location,
    /// Une pièce numérisée versée au dossier.
    Piece,
    /// Une note écrite au dossier.
    Note,
}

impl Kind {
    /// L'ordre du résumé, et **rien d'autre**.
    ///
    /// Fixe, et c'est le point : « dernier acte, dernier vaccin,
    /// dernier résultat… » se lit d'un coup d'œil parce que la ligne est
    /// toujours la même. Rangées par récence, les mêmes six lignes
    /// changeraient de place à chaque écriture, et il faudrait les lire
    /// pour savoir laquelle est laquelle.
    pub const ALL: [Kind; 7] = [
        Kind::Entretien,
        Kind::Stupefiant,
        Kind::Vaccin,
        Kind::Biologie,
        Kind::Location,
        Kind::Piece,
        Kind::Note,
    ];

    pub fn label_key(self) -> &'static str {
        match self {
            Kind::Entretien => "fil_kind_entretien",
            Kind::Vaccin => "fil_kind_vaccin",
            Kind::Biologie => "fil_kind_biologie",
            Kind::Stupefiant => "fil_kind_stupefiant",
            Kind::Location => "fil_kind_location",
            Kind::Piece => "fil_kind_piece",
            Kind::Note => "fil_kind_note",
        }
    }

    /// Sa couleur dans la rampe de données, fixe par nature — comme au
    /// registre. Une couleur qui suivrait les données serait de la
    /// décoration, et le repère d'un vaccin doit être le même sur tous
    /// les dossiers.
    ///
    /// **Le rang 1 n'est donné à personne.** C'est le gris de la rampe,
    /// celui que tous les graphiques emploient pour « ce qui n'est pas
    /// fait » ; un vaccin repéré en gris se lit comme un vaccin en
    /// attente. Sept natures pour huit couleurs, et la huitième est
    /// celle qui veut dire quelque chose d'autre.
    pub fn series(self) -> usize {
        match self {
            Kind::Entretien => 0,
            Kind::Vaccin => 2,
            Kind::Stupefiant => 3,
            Kind::Location => 4,
            Kind::Biologie => 5,
            Kind::Piece => 6,
            Kind::Note => 7,
        }
    }
}

/// Une ligne du fil.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Event {
    /// Le jour, ISO. Une ligne sans jour n'entre pas dans un fil — voir
    /// [`merge`].
    pub day: String,
    pub kind: Kind,
    /// Ce que la ligne dit : « Bilan partagé de médication », « Skenan
    /// LP 30 mg », « DTP ».
    pub title: String,
    /// Le détail, s'il en faut un : l'état de l'acte, la quantité, le
    /// lot. Vide plutôt qu'une phrase inventée.
    pub detail: String,
    /// De quoi retourner à la chose : l'identifiant dans sa propre
    /// table. Zéro quand il n'y a nulle part où aller.
    pub id: i64,
}

/// Le fil, du plus récent au plus ancien.
///
/// # Deux décisions
///
/// **Une ligne sans jour n'entre pas.** Une pièce dont personne n'a noté
/// la date, une note d'une version qui n'en écrivait pas : les placer
/// « quelque part » les daterait d'un jour qui n'est pas le leur, et un
/// fil dont une ligne ment sur sa date ne se lit plus du tout. Elles
/// restent visibles là où elles vivent — c'est leur onglet qui les
/// montre —, elles ne sont simplement pas ici.
///
/// **Et l'ordre est total.** À jour égal, la nature départage, puis le
/// libellé : sans cela deux lignes du même jour changeraient de place
/// d'une image à l'autre selon ce que la base a rendu en premier, et un
/// fil qui bouge sous les yeux se lit deux fois.
pub fn merge(events: Vec<Event>) -> Vec<Event> {
    let mut out: Vec<Event> = events
        .into_iter()
        .filter(|e| crate::date::parse_iso(&e.day).is_some())
        .collect();
    out.sort_by(|a, b| {
        b.day
            .cmp(&a.day)
            .then(a.kind.series().cmp(&b.kind.series()))
            .then(a.title.cmp(&b.title))
    });
    out
}

/// Ce qui n'a pas encore eu lieu : le nombre de lignes en tête du fil
/// dont le jour est **après** aujourd'hui.
///
/// Un dossier porte des rendez-vous pris, un rappel de vaccin, une
/// location dont l'ordonnance court : ce sont des lignes datées comme
/// les autres, et les cacher ferait un fil qui s'arrête à hier. Elles
/// sont en tête, puisque le fil se lit du plus récent au plus ancien, et
/// **elles se nomment** : « TROD angine — 12/09/2026 » en tête d'une
/// liste sans titre se lit comme quelque chose qui a eu lieu.
///
/// `events` est supposé rendu par [`merge`], donc trié : les lignes à
/// venir sont exactement les premières.
pub fn upcoming(events: &[Event], today: &str) -> usize {
    if crate::date::parse_iso(today).is_none() {
        return 0;
    }
    events.iter().take_while(|e| e.day.as_str() > today).count()
}

/// La dernière ligne **déjà arrivée** de chaque nature, dans l'ordre de
/// [`Kind::ALL`].
///
/// C'est la réponse à « quand l'a-t-on vu pour la dernière fois, et pour
/// quoi » : quelques lignes qui tiennent en haut du fil, toujours les
/// mêmes, toujours dans le même ordre. Une nature dont le dossier ne
/// porte rien n'y figure pas — « aucun vaccin » sur cinq dossiers sur
/// six ferait des lignes vides qu'on apprend à sauter.
///
/// **Un rendez-vous de la semaine prochaine n'est pas le dernier acte.**
/// C'est la seule raison pour laquelle cette fonction connaît le jour :
/// sans lui, un dossier où l'on a programmé quelque chose annoncerait
/// comme dernière visite une visite qui n'a pas eu lieu, ce qui est
/// exactement le contraire de ce qu'on lit ici.
///
/// `events` est supposé rendu par [`merge`], donc déjà trié : la
/// première rencontrée est la plus récente.
pub fn latest<'a>(events: &'a [Event], today: &str) -> Vec<&'a Event> {
    Kind::ALL
        .into_iter()
        .filter_map(|k| {
            events
                .iter()
                .find(|e| e.kind == k && e.day.as_str() <= today)
        })
        .collect()
}

/// Combien de lignes par mois, sur les `months` derniers, du plus ancien
/// au plus récent.
///
/// Pour la bande de chaleur en tête du fil. Les mois **vides sont
/// rendus** : un trou de huit mois est précisément ce que la bande doit
/// montrer, et une bande qui ne porterait que les mois actifs les
/// mettrait côte à côte et raconterait le contraire.
///
/// `today` est donné et non lu à une horloge, comme partout ici.
pub fn per_month(events: &[Event], today: &str, months: usize) -> Vec<(String, usize)> {
    let Some((y, m, _)) = crate::date::parse_iso(today) else {
        return Vec::new();
    };
    let months = months.max(1);
    let mut out = Vec::with_capacity(months);
    // Le compte des mois depuis l'an zéro : soustraire des mois est
    // alors une soustraction, et non une suite de conditions sur
    // décembre.
    let end = y * 12 + (m - 1);
    #[allow(clippy::cast_possible_wrap)]
    let start = end - (months as i64 - 1);
    for n in start..=end {
        let key = format!("{:04}-{:02}", n.div_euclid(12), n.rem_euclid(12) + 1);
        let count = events.iter().filter(|e| e.day.starts_with(&key)).count();
        out.push((key, count));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(day: &str, kind: Kind, title: &str) -> Event {
        Event {
            day: day.to_owned(),
            kind,
            title: title.to_owned(),
            detail: String::new(),
            id: 0,
        }
    }

    /// Le fil se lit du plus récent au plus ancien, et **une ligne sans
    /// jour n'y est pas**.
    ///
    /// Une pièce dont personne n'a noté la date placée « quelque part »
    /// serait datée d'un jour qui n'est pas le sien ; un fil dont une
    /// ligne ment sur sa date ne se lit plus du tout.
    #[test]
    fn the_thread_is_read_newest_first_and_an_undated_line_is_not_in_it() {
        let fil = merge(vec![
            ev("2026-01-05", Kind::Entretien, "Bilan partagé de médication"),
            ev("", Kind::Piece, "Ordonnance sans date"),
            ev("2026-09-01", Kind::Vaccin, "Grippe"),
            ev("pas une date", Kind::Note, "note d'une vieille version"),
            ev("2026-03-12", Kind::Biologie, "INR"),
        ]);
        assert_eq!(
            fil.iter().map(|e| e.day.as_str()).collect::<Vec<_>>(),
            vec!["2026-09-01", "2026-03-12", "2026-01-05"]
        );
        // Et l'ordre est **total** : deux lignes du même jour ne
        // changent pas de place d'une image à l'autre. La nature
        // départage, puis le libellé.
        let same_day = merge(vec![
            ev("2026-05-04", Kind::Piece, "Ordonnance"),
            ev("2026-05-04", Kind::Entretien, "TROD angine"),
            ev("2026-05-04", Kind::Stupefiant, "Skenan LP 30 mg"),
            ev("2026-05-04", Kind::Entretien, "AVK — entretien 2"),
        ]);
        assert_eq!(
            same_day
                .iter()
                .map(|e| e.title.as_str())
                .collect::<Vec<_>>(),
            vec![
                "AVK — entretien 2",
                "TROD angine",
                "Skenan LP 30 mg",
                "Ordonnance",
            ]
        );
        assert!(merge(Vec::new()).is_empty());
    }

    /// Le résumé donne la dernière de chaque nature, **dans un ordre
    /// fixe** et non par récence.
    ///
    /// C'est ce qui le rend lisible d'un coup d'œil : la ligne est
    /// toujours la même. Rangé par récence, le même résumé changerait de
    /// place à chaque écriture et il faudrait le lire pour savoir ce
    /// qu'on lit.
    #[test]
    fn the_summary_gives_the_last_of_each_kind_in_a_fixed_order() {
        let fil = merge(vec![
            ev("2026-01-05", Kind::Stupefiant, "Skenan LP 30 mg"),
            ev("2026-07-20", Kind::Stupefiant, "Skenan LP 60 mg"),
            ev("2026-09-01", Kind::Vaccin, "Grippe"),
            ev("2024-02-02", Kind::Vaccin, "DTP"),
            ev("2026-03-12", Kind::Biologie, "INR"),
        ]);
        let last = latest(&fil, "2026-09-08");
        assert_eq!(
            last.iter()
                .map(|e| (e.kind, e.title.as_str()))
                .collect::<Vec<_>>(),
            vec![
                (Kind::Stupefiant, "Skenan LP 60 mg"),
                (Kind::Vaccin, "Grippe"),
                (Kind::Biologie, "INR"),
            ],
            "l'ordre de Kind::ALL, et pas celui des dates"
        );
        // Une nature dont le dossier ne porte rien n'y figure pas :
        // « aucun vaccin » sur cinq dossiers sur six ferait des lignes
        // vides qu'on apprend à sauter.
        assert!(latest(&[], "2026-09-08").is_empty());

        // **Un rendez-vous de la semaine prochaine n'est pas le dernier
        // acte.** Il est en tête du fil, où il a sa place, et il se
        // nomme : « à venir ». Sans cela le résumé annoncerait comme
        // dernière visite une visite qui n'a pas eu lieu.
        let with_rdv = merge(vec![
            ev("2026-09-20", Kind::Entretien, "TROD angine"),
            ev("2026-09-01", Kind::Entretien, "AOD — entretien 1"),
            ev("2026-03-12", Kind::Biologie, "INR"),
        ]);
        assert_eq!(upcoming(&with_rdv, "2026-09-08"), 1);
        assert_eq!(
            latest(&with_rdv, "2026-09-08")
                .iter()
                .map(|e| e.title.as_str())
                .collect::<Vec<_>>(),
            vec!["AOD — entretien 1", "INR"]
        );
        // Le jour même a eu lieu : on ne repousse pas au lendemain ce
        // qu'on vient de faire.
        assert_eq!(upcoming(&with_rdv, "2026-09-20"), 0);
        assert_eq!(upcoming(&with_rdv, ""), 0, "sans jour, rien n'est à venir");
    }

    /// Chaque nature a son libellé et **sa couleur à elle**.
    ///
    /// Deux natures qui partageraient une couleur feraient un fil où
    /// une pièce et une note se ressemblent : le repère est la seule
    /// chose qui distingue une ligne d'une autre sans la lire, et il ne
    /// vaut que s'il est unique. Le libellé, lui, doit exister —
    /// `strings.rs` refuse déjà une clé que personne n'affiche ; ce test
    /// tient l'autre sens, une nature dont personne n'aurait écrit la
    /// clé.
    #[test]
    fn every_kind_has_its_own_colour_and_its_own_label() {
        let mut seen: Vec<usize> = Vec::new();
        for k in Kind::ALL {
            assert!(!k.label_key().is_empty(), "{k:?}");
            assert!(
                k.label_key().starts_with("fil_kind_"),
                "{k:?} : la clé dit d'où elle vient"
            );
            assert!(!seen.contains(&k.series()), "{k:?} partage sa couleur");
            assert_ne!(
                k.series(),
                1,
                "{k:?} : le rang 1 est le gris de « pas fait », il ne \
                 nomme aucune nature"
            );
            seen.push(k.series());
        }
        assert_eq!(seen.len(), Kind::ALL.len());
    }

    /// La bande de chaleur rend **les mois vides aussi**.
    ///
    /// Un trou de huit mois est précisément ce qu'elle doit montrer.
    /// Une bande qui ne porterait que les mois actifs les mettrait côte
    /// à côte et raconterait le contraire de ce qui s'est passé.
    #[test]
    fn the_heat_strip_keeps_the_empty_months() {
        let fil = merge(vec![
            ev("2026-09-02", Kind::Entretien, "TROD"),
            ev("2026-09-08", Kind::Biologie, "INR"),
            ev("2026-07-01", Kind::Vaccin, "Grippe"),
            // Hors fenêtre : un an et demi en arrière.
            ev("2025-02-01", Kind::Note, "vieille note"),
        ]);
        let band = per_month(&fil, "2026-09-08", 6);
        assert_eq!(
            band,
            vec![
                ("2026-04".to_owned(), 0),
                ("2026-05".to_owned(), 0),
                ("2026-06".to_owned(), 0),
                ("2026-07".to_owned(), 1),
                ("2026-08".to_owned(), 0),
                ("2026-09".to_owned(), 2),
            ]
        );
        // Le passage d'année se fait par une soustraction et non par
        // une condition sur décembre.
        let across = per_month(&fil, "2026-02-15", 4);
        assert_eq!(
            across.iter().map(|(m, _)| m.as_str()).collect::<Vec<_>>(),
            vec!["2025-11", "2025-12", "2026-01", "2026-02"]
        );
        // Un jour illisible ne rend pas une bande fausse : il n'en rend
        // aucune.
        assert!(per_month(&fil, "", 6).is_empty());
        // Et zéro mois demandé rend quand même le mois courant : une
        // bande vide serait un rectangle qu'on ne peut pas expliquer.
        assert_eq!(per_month(&fil, "2026-09-08", 0).len(), 1);
    }
}
