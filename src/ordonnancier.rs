//! Le registre des stupéfiants : ce qui est entré, ce qui est sorti, et
//! ce qui devrait rester.
//!
//! L'article R. 5132-36 du code de la santé publique demande que toute
//! entrée et toute sortie de stupéfiant soient inscrites, que la balance
//! soit tenue, et qu'un inventaire par pesée ou par comptage soit porté
//! au registre au moins une fois par an. Le registre est **inaltérable**
//! : une ligne écrite ne se corrige pas, elle se contre-passe. C'est la
//! contrainte qui décide de tout le reste — il n'y a ici ni modification
//! ni suppression, et la base non plus n'en propose pas.
//!
//! Ce module ne contient que l'arithmétique et les règles :
//!
//! * la **balance**, qui n'est pas une somme — un inventaire *fixe* le
//!   solde au lieu de s'y ajouter, sans quoi l'écart constaté serait
//!   compté deux fois ;
//! * l'**annulation**, qui est la seule façon de corriger : une ligne
//!   fautive reste, et une ligne de plus la désigne et défait ce qu'elle
//!   avait fait au stock ;
//! * le **numéro d'ordonnancier**, séquentiel dans l'année et jamais
//!   réattribué ;
//! * l'**écart** d'inventaire, et ce qu'il vaut ;
//! * la **liste de contrôle** : ce qu'il faut aller compter, parce que
//!   le stock est bas ou parce que personne ne l'a compté depuis
//!   longtemps.
//!
//! Et le **catalogue** : les cent six présentations du marché français
//! qu'une officine peut avoir à inscrire, avec leur dosage, leur unité
//! de comptage, la durée maximale de prescription et la règle propre à
//! leur famille. C'est une table de règles et non un contenu de base :
//! elle ne se sème pas, on y **choisit**. Une base livrée avec cent six
//! produits suivis serait cent six soldes à zéro, cent six « jamais
//! compté » sur la liste de contrôle, et un écran qu'on n'ouvre plus.
//!
//! Pur et testé, comme `revue` et `conciliation`. Aucune base ici : la
//! base est passée en argument. Et aucune horloge : le jour est donné,
//! parce qu'un registre qui se lit différemment selon l'heure à laquelle
//! on l'ouvre n'est pas un registre.

/// Une famille du catalogue : ce qui partage la même règle.
///
/// La règle est **de la famille** et le dosage est de la présentation,
/// et c'est pour cela que la table a deux étages. Le fractionnement par
/// sept jours ne concerne pas le sirop de méthadone à 20 mg plutôt que
/// celui à 40 : il concerne le sirop. Écrire la règle sur chaque ligne
/// serait la recopier dix fois, donc la corriger neuf fois sur dix.
pub struct Family {
    pub name: &'static str,
    /// La clé de [`Status`].
    pub status: &'static str,
    /// La durée maximale de prescription, en jours.
    pub max_days: i64,
    /// La règle que le comptoir doit connaître, en une ou deux phrases.
    pub note: &'static str,
    /// Les présentations : le libellé tel qu'il s'écrira sur la ligne du
    /// registre — **avec son dosage**, puisque c'est la présentation
    /// qu'on compte et non la molécule — et l'unité de comptage.
    pub items: &'static [(&'static str, &'static str)],
}

/// Le catalogue des stupéfiants et assimilés du marché français de
/// ville, par famille.
///
/// Ce qui **n'y est pas**, et volontairement : les produits que seul un
/// hôpital détient (kétamine, sufentanil, péthidine, remifentanil), qui
/// ne passeront jamais par le registre d'une officine ; et les
/// benzodiazépines à ordonnance sécurisée — clonazépam, midazolam,
/// zolpidem —, qui relèvent de la liste I et d'une ordonnance
/// particulière, sans aucune obligation de registre. Les faire figurer
/// ici les rangerait à côté de la morphine, et un catalogue qui range
/// mal enseigne mal.
pub const CATALOGUE: &[Family] = &[
    Family {
        name: "Morphine LP",
        status: "STUPEFIANT",
        max_days: 28,
        note: "Ordonnance sécurisée, 28 jours. Le relais d'une forme LP à une autre se fait à dose égale. Une gélule LP ouverte garde sa libération prolongée si les microgranules ne sont pas écrasés ; un comprimé LP écrasé délivre la dose entière d'un coup.",
        items: &[
            ("Skenan LP 10 mg", "gélule"),
            ("Skenan LP 30 mg", "gélule"),
            ("Skenan LP 60 mg", "gélule"),
            ("Skenan LP 100 mg", "gélule"),
            ("Skenan LP 200 mg", "gélule"),
            ("Moscontin LP 10 mg", "comprimé"),
            ("Moscontin LP 30 mg", "comprimé"),
            ("Moscontin LP 60 mg", "comprimé"),
            ("Moscontin LP 100 mg", "comprimé"),
            ("Moscontin LP 200 mg", "comprimé"),
        ],
    },
    Family {
        name: "Morphine à libération immédiate",
        status: "STUPEFIANT",
        max_days: 28,
        note: "L'interdose de l'accès douloureux vaut le dixième au sixième de la dose quotidienne de fond. C'est le nombre d'interdoses prises par jour qui dit qu'il faut réévaluer le fond, et c'est une question à poser au comptoir.",
        items: &[
            ("Actiskenan 5 mg", "gélule"),
            ("Actiskenan 10 mg", "gélule"),
            ("Actiskenan 20 mg", "gélule"),
            ("Actiskenan 30 mg", "gélule"),
            ("Sevredol 10 mg", "comprimé"),
            ("Sevredol 20 mg", "comprimé"),
            ("Oramorph 10 mg/5 mL", "récipient unidose"),
            ("Oramorph 30 mg/5 mL", "récipient unidose"),
            ("Oramorph 100 mg/5 mL", "récipient unidose"),
            ("Oramorph 20 mg/mL solution buvable", "flacon"),
        ],
    },
    Family {
        name: "Opioïdes injectables",
        status: "STUPEFIANT",
        max_days: 7,
        note: "Voie parentérale : prescription limitée à 7 jours, portée à 28 jours lorsque l'administration se fait par un système actif de perfusion. C'est la seule famille où la durée n'est pas de 28 jours, et l'oublier fait délivrer une ordonnance périmée.",
        items: &[
            ("Chlorhydrate de morphine 10 mg/mL", "ampoule"),
            ("Chlorhydrate de morphine 20 mg/mL", "ampoule"),
            ("Chlorhydrate de morphine 50 mg/5 mL", "ampoule"),
            ("Oxycodone 10 mg/mL injectable", "ampoule"),
        ],
    },
    Family {
        name: "Oxycodone",
        status: "STUPEFIANT",
        max_days: 28,
        note: "Équianalgésie orale : 1 mg d'oxycodone vaut environ 2 mg de morphine. Un relais fait à dose égale double la dose, et c'est l'erreur classique de la sortie d'hospitalisation.",
        items: &[
            ("Oxycontin LP 5 mg", "comprimé"),
            ("Oxycontin LP 10 mg", "comprimé"),
            ("Oxycontin LP 20 mg", "comprimé"),
            ("Oxycontin LP 40 mg", "comprimé"),
            ("Oxycontin LP 80 mg", "comprimé"),
            ("Oxynorm 5 mg", "gélule"),
            ("Oxynorm 10 mg", "gélule"),
            ("Oxynorm 20 mg", "gélule"),
            ("Oxynormoro 5 mg", "comprimé orodispersible"),
            ("Oxynormoro 10 mg", "comprimé orodispersible"),
            ("Oxynormoro 20 mg", "comprimé orodispersible"),
            ("Oxynorm 10 mg/mL solution buvable", "flacon"),
        ],
    },
    Family {
        name: "Hydromorphone",
        status: "STUPEFIANT",
        max_days: 28,
        note: "Réservée aux douleurs intenses d'origine cancéreuse, en cas de résistance ou d'intolérance à la morphine. Équianalgésie : 4 mg d'hydromorphone valent 30 mg de morphine orale, soit un rapport de 7,5 — le plus facile à se tromper de tous.",
        items: &[
            ("Sophidone LP 4 mg", "gélule"),
            ("Sophidone LP 8 mg", "gélule"),
            ("Sophidone LP 16 mg", "gélule"),
            ("Sophidone LP 24 mg", "gélule"),
        ],
    },
    Family {
        name: "Fentanyl transdermique",
        status: "STUPEFIANT",
        max_days: 28,
        note: "Un patch usagé garde de quoi tuer un enfant : il se replie sur lui-même, adhésif contre adhésif, et se rapporte à l'officine. La chaleur — fièvre, couverture chauffante, bain chaud, soleil — augmente le passage et peut provoquer un surdosage sous un patch qui convenait la veille.",
        items: &[
            ("Durogesic 12 µg/h", "dispositif transdermique"),
            ("Durogesic 25 µg/h", "dispositif transdermique"),
            ("Durogesic 50 µg/h", "dispositif transdermique"),
            ("Durogesic 75 µg/h", "dispositif transdermique"),
            ("Durogesic 100 µg/h", "dispositif transdermique"),
        ],
    },
    Family {
        name: "Fentanyl transmuqueux",
        status: "STUPEFIANT",
        max_days: 28,
        note: "Réservé aux accès douloureux paroxystiques d'un patient déjà sous morphinique de fond équilibré. La dose efficace se titre à partir du dosage le plus faible et ne se déduit jamais de la dose de fond — c'est la mise en garde qui revient sur toutes les alertes de cette classe.",
        items: &[
            ("Abstral 100 µg", "comprimé sublingual"),
            ("Abstral 200 µg", "comprimé sublingual"),
            ("Abstral 300 µg", "comprimé sublingual"),
            ("Abstral 400 µg", "comprimé sublingual"),
            ("Abstral 600 µg", "comprimé sublingual"),
            ("Abstral 800 µg", "comprimé sublingual"),
            ("Effentora 100 µg", "comprimé gingival"),
            ("Effentora 200 µg", "comprimé gingival"),
            ("Effentora 400 µg", "comprimé gingival"),
            ("Effentora 600 µg", "comprimé gingival"),
            ("Effentora 800 µg", "comprimé gingival"),
            ("Actiq 200 µg", "comprimé avec applicateur buccal"),
            ("Actiq 400 µg", "comprimé avec applicateur buccal"),
            ("Actiq 600 µg", "comprimé avec applicateur buccal"),
            ("Actiq 800 µg", "comprimé avec applicateur buccal"),
            ("Actiq 1200 µg", "comprimé avec applicateur buccal"),
            ("Actiq 1600 µg", "comprimé avec applicateur buccal"),
            ("Instanyl 50 µg/dose", "flacon pulvérisateur"),
            ("Instanyl 100 µg/dose", "flacon pulvérisateur"),
            ("Instanyl 200 µg/dose", "flacon pulvérisateur"),
            ("Pecfent 100 µg/dose", "flacon pulvérisateur"),
            ("Pecfent 400 µg/dose", "flacon pulvérisateur"),
        ],
    },
    Family {
        name: "Méthadone gélule",
        status: "STUPEFIANT",
        max_days: 28,
        note: "Relais du sirop seulement, chez un patient stabilisé depuis au moins un an et suivi. Délivrance fractionnée par 14 jours sauf mention expresse du prescripteur. La gélule n'est jamais une initiation.",
        items: &[
            ("Méthadone AP-HP gélule 1 mg", "gélule"),
            ("Méthadone AP-HP gélule 5 mg", "gélule"),
            ("Méthadone AP-HP gélule 10 mg", "gélule"),
            ("Méthadone AP-HP gélule 20 mg", "gélule"),
            ("Méthadone AP-HP gélule 40 mg", "gélule"),
        ],
    },
    Family {
        name: "Méthadone sirop",
        status: "STUPEFIANT",
        max_days: 14,
        note: "Prescription limitée à 14 jours et délivrance fractionnée par 7 jours sauf mention expresse. Le nom du pharmacien qui délivre est porté sur l'ordonnance, et le chevauchement est interdit.",
        items: &[
            ("Méthadone AP-HP sirop 5 mg", "récipient unidose"),
            ("Méthadone AP-HP sirop 10 mg", "récipient unidose"),
            ("Méthadone AP-HP sirop 20 mg", "récipient unidose"),
            ("Méthadone AP-HP sirop 40 mg", "récipient unidose"),
            ("Méthadone AP-HP sirop 60 mg", "récipient unidose"),
        ],
    },
    Family {
        name: "Méthylphénidate",
        status: "STUPEFIANT",
        max_days: 28,
        note: "Prescription initiale annuelle réservée aux spécialistes (psychiatrie, neurologie, pédiatrie) ; les renouvellements de l'année se font par tout médecin. Ordonnance sécurisée, 28 jours. Les formes LP ne sont pas interchangeables entre elles : la part immédiate diffère d'une marque à l'autre.",
        items: &[
            ("Ritaline 10 mg", "comprimé"),
            ("Ritaline LP 10 mg", "gélule"),
            ("Ritaline LP 20 mg", "gélule"),
            ("Ritaline LP 30 mg", "gélule"),
            ("Ritaline LP 40 mg", "gélule"),
            ("Concerta LP 18 mg", "comprimé"),
            ("Concerta LP 36 mg", "comprimé"),
            ("Concerta LP 54 mg", "comprimé"),
            ("Quasym LP 10 mg", "gélule"),
            ("Quasym LP 20 mg", "gélule"),
            ("Quasym LP 30 mg", "gélule"),
            ("Medikinet 5 mg", "gélule"),
            ("Medikinet 10 mg", "gélule"),
            ("Medikinet 20 mg", "gélule"),
            ("Medikinet LM 10 mg", "gélule"),
            ("Medikinet LM 20 mg", "gélule"),
            ("Medikinet LM 30 mg", "gélule"),
            ("Medikinet LM 40 mg", "gélule"),
        ],
    },
    Family {
        name: "Oxybate de sodium",
        status: "STUPEFIANT",
        max_days: 28,
        note: "Les deux prises se font déjà couché, la seconde deux heures et demie à quatre heures après la première, et au moins deux heures après le dîner. L'alcool et tout autre dépresseur respiratoire sont formellement contre-indiqués le soir de la prise.",
        items: &[("Xyrem 500 mg/mL solution buvable", "flacon")],
    },
    Family {
        name: "Buprénorphine haut dosage",
        status: "ASSIMILE",
        max_days: 28,
        note: "Assimilé stupéfiant : ordonnance sécurisée, 28 jours, chevauchement interdit, délivrance fractionnée par 7 jours sauf mention expresse. L'inscription au registre n'est **pas** exigée pour cette classe — l'officine la tient si elle le choisit, et beaucoup le font.",
        items: &[
            ("Subutex 0,4 mg", "comprimé sublingual"),
            ("Subutex 2 mg", "comprimé sublingual"),
            ("Subutex 8 mg", "comprimé sublingual"),
            ("Buprénorphine 0,4 mg", "comprimé sublingual"),
            ("Buprénorphine 2 mg", "comprimé sublingual"),
            ("Buprénorphine 8 mg", "comprimé sublingual"),
            ("Orobupré 2 mg", "comprimé orodispersible"),
            ("Orobupré 8 mg", "comprimé orodispersible"),
            ("Suboxone 2 mg/0,5 mg", "comprimé sublingual"),
            ("Suboxone 8 mg/2 mg", "comprimé sublingual"),
        ],
    },
];

/// Combien de présentations le catalogue porte.
pub fn catalogue_size() -> usize {
    CATALOGUE.iter().map(|f| f.items.len()).sum()
}

/// Ce qu'une ligne du registre fait au stock.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    /// Réception d'une commande : le stock monte.
    Entree,
    /// Délivrance sur ordonnance : le stock descend, et la ligne porte
    /// un numéro d'ordonnancier et le dossier du patient.
    Sortie,
    /// Comptage physique : le stock **devient** ce qui a été compté.
    Inventaire,
    /// Casse, péremption, retour, vol : le stock descend, hors
    /// délivrance. La ligne porte ce qui s'est passé.
    Perte,
    /// L'annulation d'une ligne fautive : elle **désigne** la ligne
    /// qu'elle annule et défait exactement ce que celle-ci avait fait au
    /// stock. C'est la seule correction que le registre connaisse — la
    /// ligne fautive reste écrite, et c'est ce qui fait qu'un registre
    /// prouve quelque chose.
    ///
    /// Sa quantité n'est jamais lue : ce qu'elle défait se lit sur la
    /// ligne annulée, ce qui rend impossible une annulation qui rendrait
    /// autre chose que ce qui avait été pris.
    Annulation,
}

impl Kind {
    /// Sa clé dans la base, stable : le registre se relit dans dix ans.
    pub fn as_key(self) -> &'static str {
        match self {
            Kind::Entree => "ENTREE",
            Kind::Sortie => "SORTIE",
            Kind::Inventaire => "INVENTAIRE",
            Kind::Perte => "PERTE",
            Kind::Annulation => "ANNULATION",
        }
    }

    /// Une clé que cette version ne connaît pas n'est pas une erreur de
    /// lecture : c'est une ligne écrite par une version plus récente, et
    /// elle est lue comme une perte — le choix qui sous-estime le stock
    /// plutôt que de le surestimer.
    pub fn from_key(key: &str) -> Kind {
        match key {
            "ENTREE" => Kind::Entree,
            "SORTIE" => Kind::Sortie,
            "INVENTAIRE" => Kind::Inventaire,
            "ANNULATION" => Kind::Annulation,
            _ => Kind::Perte,
        }
    }

    /// La clé de son libellé français.
    pub fn label_key(self) -> &'static str {
        match self {
            Kind::Entree => "stup_kind_entree",
            Kind::Sortie => "stup_kind_sortie",
            Kind::Inventaire => "stup_kind_inventaire",
            Kind::Perte => "stup_kind_perte",
            Kind::Annulation => "stup_kind_annulation",
        }
    }

    /// Sa couleur dans la palette de données, fixe par nature : une
    /// couleur qui suivrait les données serait de la décoration.
    pub fn series(self) -> usize {
        match self {
            Kind::Entree => 2,
            Kind::Sortie => 0,
            Kind::Inventaire => 1,
            Kind::Perte => 3,
            Kind::Annulation => 5,
        }
    }

    /// Porte-t-elle un numéro d'ordonnancier et un dossier ? Seule la
    /// délivrance en porte : une réception n'a pas de patient, et en
    /// inventer un serait écrire un nom dans un registre pour rien.
    pub fn is_dispensing(self) -> bool {
        self == Kind::Sortie
    }

    /// Les quatre natures que l'on **écrit**.
    ///
    /// L'annulation n'en est pas : elle ne se choisit pas dans un
    /// formulaire, elle se demande sur la ligne à annuler. Un
    /// « annuler » posé à côté de « réception » et de « délivrance »
    /// serait une cinquième façon d'écrire une ligne, alors que c'est
    /// une façon d'en corriger une.
    pub const ALL: [Kind; 4] = [Kind::Entree, Kind::Sortie, Kind::Inventaire, Kind::Perte];

    /// Une ligne de cette nature peut-elle être annulée ?
    ///
    /// Tout sauf une annulation : annuler une annulation ferait un
    /// registre où l'on ne sait plus ce qui vaut, et la correction d'une
    /// annulation fautive est une ligne qui l'explique, pas une
    /// troisième couche.
    pub fn can_be_cancelled(self) -> bool {
        self != Kind::Annulation
    }
}

/// Ce que la réglementation demande d'un produit suivi.
///
/// Deux régimes, et les confondre serait enseigner une règle fausse. Un
/// **stupéfiant** s'inscrit au registre : c'est une obligation, et c'est
/// pour lui que le registre existe. Un **assimilé** — la buprénorphine
/// haut dosage, par exemple — relève de la réglementation des
/// stupéfiants pour la prescription et la délivrance (ordonnance
/// sécurisée, chevauchement interdit, délivrance fractionnée) mais
/// **pas** pour la comptabilité : l'officine peut le suivre ici, et
/// beaucoup le font, mais rien ne l'y oblige.
///
/// Le champ existe précisément pour que le catalogue livré puisse porter
/// les seconds sans faire croire aux premiers.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Status {
    Stupefiant,
    Assimile,
}

impl Status {
    pub fn as_key(self) -> &'static str {
        match self {
            Status::Stupefiant => "STUPEFIANT",
            Status::Assimile => "ASSIMILE",
        }
    }

    /// Ce que cette version ne connaît pas est lu comme un stupéfiant :
    /// le régime le plus exigeant. Se tromper dans ce sens fait tenir un
    /// registre de trop, dans l'autre il en manque un.
    pub fn from_key(key: &str) -> Status {
        match key {
            "ASSIMILE" => Status::Assimile,
            _ => Status::Stupefiant,
        }
    }

    pub fn label_key(self) -> &'static str {
        match self {
            Status::Stupefiant => "stup_status_stupefiant",
            Status::Assimile => "stup_status_assimile",
        }
    }

    /// Ce que ce régime demande, en une phrase, sous la souris.
    pub fn note_key(self) -> &'static str {
        match self {
            Status::Stupefiant => "stup_status_stupefiant_note",
            Status::Assimile => "stup_status_assimile_note",
        }
    }
}

/// Une ligne du registre, telle que l'arithmétique la lit.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Move<'a> {
    pub kind: Kind,
    /// Toujours positive : c'est [`Kind`] qui dit le sens. Une quantité
    /// signée serait une deuxième façon de dire la même chose, donc une
    /// deuxième façon de se tromper.
    pub quantity: f64,
    /// Le jour, ISO. L'ordre du registre est celui-ci et non celui des
    /// identifiants : une réception saisie le lendemain n'est pas une
    /// réception du lendemain.
    pub day: &'a str,
    /// L'ordre de saisie, qui départage deux lignes du même jour. C'est
    /// aussi l'identifiant par lequel une annulation la désigne.
    pub seq: i64,
    /// Le `seq` de la ligne que celle-ci annule, ou 0.
    pub cancels: i64,
    /// Ce que le registre disait avant un inventaire. Lu par une
    /// annulation d'inventaire, qui n'a que ce nombre pour rendre au
    /// solde ce que le comptage lui avait pris.
    pub expected: f64,
}

/// Le solde après toutes ces lignes.
///
/// Ce n'est **pas** une somme. Un inventaire fixe le solde à ce qui a
/// été compté : additionner l'écart *et* poser le compte reviendrait à
/// le compter deux fois, et le registre partirait à la dérive dès le
/// premier comptage qui ne tombe pas juste.
///
/// Les lignes sont triées ici, par jour puis par ordre de saisie : ce
/// que la base rend n'a pas à être dans l'ordre, et un inventaire lu
/// avant les sorties qui le précèdent donnerait un solde faux.
pub fn balance(moves: &[Move]) -> f64 {
    running(moves).last().copied().unwrap_or(0.0)
}

/// Ce qu'une ligne fait au solde.
///
/// Séparé parce que l'annulation ne se lit pas sur elle-même : elle
/// défait ce que la ligne qu'elle désigne avait fait, et cette ligne
/// doit donc être retrouvée. Une annulation dont la cible n'est pas dans
/// la tranche ne fait **rien** — ne rien inventer est le seul choix
/// honnête quand on ne sait pas ce qu'on annule.
fn apply(stock: &mut f64, m: &Move, all: &[&Move]) {
    match m.kind {
        Kind::Entree => *stock += m.quantity,
        Kind::Sortie | Kind::Perte => *stock -= m.quantity,
        Kind::Inventaire => *stock = m.quantity,
        Kind::Annulation => {
            let Some(target) = all.iter().find(|t| t.seq == m.cancels) else {
                return;
            };
            match target.kind {
                Kind::Entree => *stock -= target.quantity,
                Kind::Sortie | Kind::Perte => *stock += target.quantity,
                // Rendre au solde ce que le comptage lui avait posé :
                // le registre redit ce qu'il disait avant. C'est la
                // seule raison pour laquelle `expected` est écrit dans
                // la base au lieu d'être recalculé à la lecture.
                Kind::Inventaire => *stock = target.expected,
                Kind::Annulation => {}
            }
        }
    }
}

/// Une ligne a-t-elle été annulée ?
///
/// Elle reste au registre et continue de s'y lire — barrée, jamais
/// retirée. C'est ce que voit celui qui contrôle : la faute, et la
/// correction qui la nomme.
pub fn is_cancelled(moves: &[Move], seq: i64) -> bool {
    moves
        .iter()
        .any(|m| m.kind == Kind::Annulation && m.cancels == seq)
}

/// Le solde jour après jour, pour la courbe : une valeur par ligne, dans
/// l'ordre du registre.
///
/// Le dessin lit ça et rien d'autre — refaire l'arithmétique dans la vue
/// serait deux versions de la même règle, et un jour elles diffèrent.
pub fn running(moves: &[Move]) -> Vec<f64> {
    let mut ordered: Vec<&Move> = moves.iter().collect();
    ordered.sort_by(|a, b| a.day.cmp(b.day).then(a.seq.cmp(&b.seq)));
    let mut stock = 0.0;
    let mut out = Vec::with_capacity(ordered.len());
    for m in &ordered {
        apply(&mut stock, m, &ordered);
        out.push(stock);
    }
    out
}

/// Le prochain numéro d'ordonnancier de l'année.
///
/// Séquentiel dans l'année et **jamais réattribué** : `used` est ce que
/// le registre porte déjà pour cette année, et le prochain est un de
/// plus que le plus grand. Un trou dans la suite reste un trou — une
/// ligne annulée l'est par une contre-passation, et son numéro ne
/// revient pas servir une autre délivrance.
pub fn next_number(used: &[u32]) -> u32 {
    used.iter().copied().max().unwrap_or(0) + 1
}

/// Le numéro tel qu'il s'écrit et se lit : « 2026-0042 ».
///
/// L'année devant, parce que la suite repart à un chaque année et qu'un
/// « 42 » seul ne désigne rien dans un registre de dix ans.
pub fn number_label(year: u32, no: u32) -> String {
    format!("{year}-{no:04}")
}

/// Ce qu'un inventaire a trouvé.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Discrepancy {
    /// Ce que le registre disait avant le comptage.
    pub expected: f64,
    /// Ce qui a été compté.
    pub counted: f64,
}

impl Discrepancy {
    /// Compté moins attendu : négatif, il manque.
    pub fn gap(self) -> f64 {
        self.counted - self.expected
    }

    /// L'écart mérite-t-il d'être expliqué ?
    ///
    /// Tout écart non nul en mérite un — c'est la règle, et il n'y a pas
    /// de seuil de tolérance dans le code de la santé publique. La marge
    /// ici n'est qu'une marge de **calcul** : les quantités sont des
    /// flottants, et 0,1 + 0,2 n'est pas 0,3 sur une machine.
    pub fn matters(self) -> bool {
        self.gap().abs() > 1e-6
    }
}

/// Pourquoi un produit est sur la liste de contrôle.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Why {
    /// Le solde du registre est négatif : impossible, donc une ligne
    /// manque. C'est le seul motif qui est une erreur et pas un rappel.
    Negative,
    /// Le solde est à zéro ou en dessous du seuil que l'officine a posé.
    Low,
    /// Jamais compté, ou compté il y a trop longtemps.
    Uncounted,
}

impl Why {
    /// L'ordre d'affichage : l'impossible d'abord, le rappel en dernier.
    fn rank(self) -> u8 {
        match self {
            Why::Negative => 0,
            Why::Low => 1,
            Why::Uncounted => 2,
        }
    }

    pub fn label_key(self) -> &'static str {
        match self {
            Why::Negative => "stup_why_negative",
            Why::Low => "stup_why_low",
            Why::Uncounted => "stup_why_uncounted",
        }
    }
}

/// Un produit suivi, et ce que le registre en dit.
#[derive(Clone, PartialEq, Debug)]
pub struct Followed {
    pub id: i64,
    pub label: String,
    pub unit: String,
    /// Le solde du registre.
    pub stock: f64,
    /// Le plancher que l'officine a posé ; zéro veut dire « pas de
    /// plancher », et non « plancher à zéro ».
    pub threshold: f64,
    /// Le dernier inventaire, ISO ; vide si jamais compté.
    pub last_count: String,
}

/// Une ligne de la liste de contrôle.
#[derive(Clone, PartialEq, Debug)]
pub struct ToCheck {
    pub id: i64,
    pub label: String,
    pub unit: String,
    pub stock: f64,
    pub why: Why,
    /// Depuis combien de jours le produit n'a pas été compté ; `None`
    /// s'il ne l'a jamais été.
    pub days: Option<i64>,
}

/// Ce qu'il faut aller compter, et pourquoi.
///
/// `max_days` est le délai que l'officine se donne entre deux comptages
/// (la loi en demande un par an ; une officine sérieuse en fait un par
/// mois sur les produits qui bougent). `today` est le jour, donné et non
/// lu à une horloge, pour que la liste se teste.
///
/// Un produit n'apparaît qu'une fois, sous le motif le plus grave : un
/// stock négatif jamais compté est un stock négatif, et le dire deux
/// fois n'ajoute rien à la ligne qu'il faut aller chercher.
pub fn to_check(followed: &[Followed], today: &str, max_days: i64) -> Vec<ToCheck> {
    let mut out: Vec<ToCheck> = Vec::new();
    for f in followed {
        let days = days_between(&f.last_count, today);
        // Jamais compté et compté il y a trop longtemps sont le même
        // motif : dans les deux cas la ligne à aller chercher est la
        // même, et `days` dit lequel des deux c'est.
        let never = f.last_count.trim().is_empty();
        let why = if f.stock < -1e-6 {
            Some(Why::Negative)
        } else if f.threshold > 0.0 && f.stock <= f.threshold + 1e-6 {
            Some(Why::Low)
        } else if never || days.is_some_and(|d| d > max_days) {
            Some(Why::Uncounted)
        } else {
            None
        };
        if let Some(why) = why {
            out.push(ToCheck {
                id: f.id,
                label: f.label.clone(),
                unit: f.unit.clone(),
                stock: f.stock,
                why,
                days,
            });
        }
    }
    // Le plus grave en tête, puis le stock le plus bas, puis le nom —
    // un ordre stable, sans quoi la liste imprimée le lundi et celle du
    // mardi ne se comparent pas.
    out.sort_by(|a, b| {
        a.why
            .rank()
            .cmp(&b.why.rank())
            .then(a.stock.total_cmp(&b.stock))
            .then(a.label.cmp(&b.label))
    });
    out
}

/// Combien de jours séparent deux dates ISO, `None` si l'une des deux
/// n'en est pas une.
///
/// Le calendrier grégorien par le compte des jours depuis une origine
/// commune : pas de bibliothèque de dates, pas d'horloge, et un test qui
/// tient les années bissextiles.
/// Le calendrier est celui de [`crate::date`] et non le sien : cette
/// fonction portait sa propre formule julienne quand `location.rs` en
/// portait une civile, pour la même soustraction. Les tests plus bas
/// n'ont pas bougé — ils décrivent toujours ce que le registre attend
/// d'un écart de jours, et ils l'exigent maintenant du calendrier
/// partagé.
pub fn days_between(from: &str, to: &str) -> Option<i64> {
    crate::date::days_between(from, to)
}

// --- Ce que le registre sait dire de lui-même ----------------------
//
// Tout ceci vit **ici** et non dans un module de plus : l'entrée est
// `Move` et le sujet est le même. Un second module dupliquerait le type
// ou n'existerait que pour l'importer, et « ce que le registre dit »
// serait alors écrit à deux endroits.

/// Ce qui est sorti d'un produit sur une fenêtre, et sur combien de
/// jours.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Velocity {
    pub out: f64,
    pub days: i64,
    pub lines: usize,
}

impl Velocity {
    pub fn per_day(self) -> f64 {
        if self.days <= 0 {
            0.0
        } else {
            self.out / self.days as f64
        }
    }
}

/// Le rythme des `window_days` derniers jours.
///
/// **Les délivrances seules.** Une ampoule cassée est du stock qui est
/// parti, ce n'est pas de la consommation, et elle ne dit rien de la
/// demande de demain ; les pertes se lisent à part. Une ligne annulée
/// n'a pas eu lieu.
///
/// `None` quand la fenêtre ne porte aucune délivrance : un rythme de
/// zéro sur une fenêtre vide n'est pas « zéro par jour », c'est « rien à
/// dire », et les deux ne se peignent pas pareil.
pub fn velocity(moves: &[Move], today: &str, window_days: i64) -> Option<Velocity> {
    if window_days <= 0 {
        return None;
    }
    let mut out = 0.0;
    let mut lines = 0usize;
    for m in moves {
        if m.kind != Kind::Sortie || is_cancelled(moves, m.seq) {
            continue;
        }
        match crate::date::days_between(m.day, today) {
            Some(d) if (0..=window_days).contains(&d) => {
                out += m.quantity;
                lines += 1;
            }
            _ => {}
        }
    }
    (lines > 0).then_some(Velocity {
        out,
        days: window_days,
        lines,
    })
}

/// Combien de jours le solde tient au rythme mesuré.
///
/// `None` quand rien ne bouge — la question ne se pose pas — et quand le
/// solde est négatif : le registre est alors faux, et la réponse est
/// [`Why::Negative`], pas une prévision.
///
/// C'est une projection du passé et **pas une promesse** : une entrée en
/// soins palliatifs double le rythme du jour au lendemain. Ce n'est pas
/// non plus un point de commande, qui demanderait le délai du
/// grossiste — que la base n'a pas.
pub fn days_left(stock: f64, v: Velocity) -> Option<i64> {
    let rate = v.per_day();
    if rate <= 0.0 || stock < 0.0 {
        return None;
    }
    #[allow(clippy::cast_possible_truncation)]
    Some((stock / rate).floor().max(0.0) as i64)
}

/// Un comptage et ce qu'il a trouvé.
#[derive(Clone, Debug, PartialEq)]
pub struct Count {
    pub seq: i64,
    pub day: String,
    pub gap: Discrepancy,
    /// Un comptage annulé reste au registre et se lit barré.
    pub cancelled: bool,
}

/// Tous les inventaires, dans l'ordre du registre.
///
/// Avec leur `expected` **tel qu'il a été écrit** et jamais recalculé :
/// c'est la seule raison pour laquelle cette colonne existe, puisqu'un
/// recalcul d'aujourd'hui donnerait le solde d'aujourd'hui.
pub fn counts(moves: &[Move]) -> Vec<Count> {
    let mut sorted: Vec<&Move> = moves.iter().collect();
    sorted.sort_by(|a, b| a.day.cmp(b.day).then(a.seq.cmp(&b.seq)));
    sorted
        .into_iter()
        .filter(|m| m.kind == Kind::Inventaire)
        .map(|m| Count {
            seq: m.seq,
            day: m.day.to_owned(),
            gap: Discrepancy {
                expected: m.expected,
                counted: m.quantity,
            },
            cancelled: is_cancelled(moves, m.seq),
        })
        .collect()
}

/// Ce que les comptages disent ensemble.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gaps {
    pub taken: usize,
    pub matched: usize,
    /// Ce qui manquait, en valeur absolue.
    pub short: f64,
    /// Ce qu'il y avait en trop.
    pub over: f64,
}

/// Le bilan des comptages.
///
/// `short` et `over` sont séparés et **jamais nets** : un comptage à −3
/// et un comptage à +3 ne font pas « rien », ils font deux comptages
/// inexpliqués. Les additionner serait effacer précisément ce qu'un
/// contrôle vient chercher.
pub fn gaps(counts: &[Count]) -> Gaps {
    let mut g = Gaps {
        taken: 0,
        matched: 0,
        short: 0.0,
        over: 0.0,
    };
    for c in counts.iter().filter(|c| !c.cancelled) {
        g.taken += 1;
        let d = c.gap.gap();
        if !c.gap.matters() {
            g.matched += 1;
        } else if d < 0.0 {
            g.short += -d;
        } else {
            g.over += d;
        }
    }
    g
}

/// Un trou dans la suite des numéros, de `from` à `to` inclus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hole {
    pub from: u32,
    pub to: u32,
}

impl Hole {
    pub fn count(self) -> u32 {
        self.to.saturating_sub(self.from) + 1
    }
}

/// Ce que la suite d'une année porte, et ce qui lui manque.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sequence {
    pub first: u32,
    pub last: u32,
    pub count: usize,
    /// Les trous **intérieurs**, entre le premier et le dernier numéro
    /// portés.
    pub holes: Vec<Hole>,
    /// Les numéros portés deux fois. Pire qu'un trou, et invisible.
    pub doubled: Vec<u32>,
}

/// Lire la suite des numéros d'une année.
///
/// # Deux décisions, et elles comptent
///
/// **Les trous sont strictement intérieurs.** Compter depuis 1 serait
/// défendable — `next_number(&[])` vaut 1 — mais une officine qui entre
/// dans l'application en juillet commence légitimement à 300, et deux
/// cent quatre-vingt-dix-neuf trous fantômes à chaque lancement font un
/// détecteur que personne n'ouvre deux fois. Le module dit donc ce que
/// le registre **contient** et laisse le pharmacien juger du 300.
///
/// **Une délivrance annulée garde son numéro et ne fait pas un trou.**
/// C'est ce qu'un détecteur naïf rate : compter les seules lignes non
/// annulées signalerait chaque correction du registre comme un numéro
/// manquant, et ferait ressembler les corrections à une dissimulation.
/// L'appelant passe donc **tous** les numéros attribués, annulés
/// compris.
pub fn sequence(used: &[u32]) -> Option<Sequence> {
    if used.is_empty() {
        return None;
    }
    let mut sorted: Vec<u32> = used.to_vec();
    sorted.sort_unstable();
    let first = *sorted.first()?;
    let last = *sorted.last()?;
    let mut holes = Vec::new();
    let mut doubled = Vec::new();
    for w in sorted.windows(2) {
        if w[0] == w[1] {
            if !doubled.contains(&w[0]) {
                doubled.push(w[0]);
            }
        } else if w[1] > w[0] + 1 {
            holes.push(Hole {
                from: w[0] + 1,
                to: w[1] - 1,
            });
        }
    }
    Some(Sequence {
        first,
        last,
        count: sorted.len(),
        holes,
        doubled,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mv(kind: Kind, quantity: f64, day: &str, seq: i64) -> Move<'_> {
        Move {
            kind,
            quantity,
            day,
            seq,
            cancels: 0,
            expected: 0.0,
        }
    }

    /// Un inventaire tel que la base l'écrit : le comptage, et ce que le
    /// registre disait avant lui.
    fn count(counted: f64, expected: f64, day: &str, seq: i64) -> Move<'_> {
        Move {
            expected,
            ..mv(Kind::Inventaire, counted, day, seq)
        }
    }

    /// L'annulation de la ligne `target`.
    fn cancel(target: i64, day: &str, seq: i64) -> Move<'_> {
        Move {
            cancels: target,
            ..mv(Kind::Annulation, 0.0, day, seq)
        }
    }

    /// La balance n'est pas une somme : un inventaire **pose** le solde.
    ///
    /// C'est la règle qui décide de tout. Écrite comme une somme
    /// signée, un comptage qui trouve deux comprimés de moins ajouterait
    /// son écart au solde *et* laisserait le solde d'avant, et le
    /// registre dériverait à partir du premier comptage qui ne tombe pas
    /// juste — c'est-à-dire à partir du premier.
    #[test]
    fn an_inventory_sets_the_balance_it_does_not_add_to_it() {
        let moves = [
            mv(Kind::Entree, 30.0, "2026-01-05", 1),
            mv(Kind::Sortie, 14.0, "2026-01-08", 2),
            // Le registre dit 16 ; le comptage en trouve 15.
            mv(Kind::Inventaire, 15.0, "2026-01-10", 3),
            mv(Kind::Sortie, 5.0, "2026-01-12", 4),
        ];
        assert!((balance(&moves) - 10.0).abs() < 1e-9, "{}", balance(&moves));

        // Et l'écart de ce comptage se lit pour ce qu'il est.
        let before = balance(&moves[..2]);
        let d = Discrepancy {
            expected: before,
            counted: 15.0,
        };
        assert!((d.gap() + 1.0).abs() < 1e-9, "il manque un comprimé");
        assert!(d.matters());
        // Un comptage qui tombe juste ne demande pas d'explication.
        assert!(!Discrepancy {
            expected: 16.0,
            counted: 16.0
        }
        .matters());
    }

    /// L'ordre du registre est celui des jours, pas celui de la saisie
    /// ni celui que la base rend.
    ///
    /// Une réception notée le lendemain n'est pas une réception du
    /// lendemain, et un inventaire lu avant les sorties qui le précèdent
    /// donnerait un solde faux — c'est-à-dire un manquant inventé.
    #[test]
    fn the_register_is_read_in_the_order_of_its_days() {
        let jumbled = [
            mv(Kind::Sortie, 5.0, "2026-01-12", 4),
            mv(Kind::Inventaire, 15.0, "2026-01-10", 3),
            mv(Kind::Entree, 30.0, "2026-01-05", 1),
            mv(Kind::Sortie, 14.0, "2026-01-08", 2),
        ];
        assert!((balance(&jumbled) - 10.0).abs() < 1e-9);
        // Deux lignes du même jour sont départagées par l'ordre de
        // saisie : l'inventaire du matin puis la sortie de l'après-midi
        // ne donnent pas le même solde que l'inverse.
        let same_day = [
            mv(Kind::Inventaire, 20.0, "2026-02-02", 1),
            mv(Kind::Sortie, 6.0, "2026-02-02", 2),
        ];
        assert!((balance(&same_day) - 14.0).abs() < 1e-9);
        let reversed = [
            mv(Kind::Sortie, 6.0, "2026-02-02", 1),
            mv(Kind::Inventaire, 20.0, "2026-02-02", 2),
        ];
        assert!((balance(&reversed) - 20.0).abs() < 1e-9);
    }

    /// La courbe et le solde disent la même chose, parce que c'est le
    /// même calcul : la dernière valeur de la courbe **est** la balance.
    #[test]
    fn the_curve_ends_where_the_balance_is() {
        let moves = [
            mv(Kind::Entree, 30.0, "2026-01-05", 1),
            mv(Kind::Sortie, 14.0, "2026-01-08", 2),
            mv(Kind::Inventaire, 15.0, "2026-01-10", 3),
            mv(Kind::Perte, 2.0, "2026-01-11", 4),
        ];
        let curve = running(&moves);
        assert_eq!(curve.len(), moves.len());
        assert_eq!(curve, vec![30.0, 16.0, 15.0, 13.0]);
        assert!((curve.last().copied().unwrap() - balance(&moves)).abs() < 1e-9);
        assert!(running(&[]).is_empty());
        assert_eq!(balance(&[]), 0.0);
    }

    /// Une annulation défait ce que la ligne annulée avait fait — et
    /// **rien d'autre**.
    ///
    /// C'est là qu'est la règle. Une correction écrite « à la main »,
    /// c'est-à-dire une ligne de sens contraire dont on tape la
    /// quantité, rend ce qu'on a cru avoir pris ; une annulation rend ce
    /// qui a été pris. Les deux diffèrent précisément le jour où c'est
    /// la quantité qui était fausse, c'est-à-dire le jour où l'on
    /// corrige.
    #[test]
    fn a_cancellation_undoes_the_line_it_names_and_nothing_else() {
        // Une réception de 30, une délivrance de 14 saisie deux fois par
        // deux postes : le registre dit 2, la boîte en contient 16.
        let doubled = [
            mv(Kind::Entree, 30.0, "2026-01-05", 1),
            mv(Kind::Sortie, 14.0, "2026-01-08", 2),
            mv(Kind::Sortie, 14.0, "2026-01-08", 3),
        ];
        assert!((balance(&doubled) - 2.0).abs() < 1e-9);
        let fixed = [
            mv(Kind::Entree, 30.0, "2026-01-05", 1),
            mv(Kind::Sortie, 14.0, "2026-01-08", 2),
            mv(Kind::Sortie, 14.0, "2026-01-08", 3),
            cancel(3, "2026-01-09", 4),
        ];
        assert!((balance(&fixed) - 16.0).abs() < 1e-9, "{}", balance(&fixed));
        // La quantité portée par l'annulation n'est jamais lue : ce
        // qu'elle rend se lit sur la ligne annulée. Une annulation à qui
        // l'on ferait dire 999 rend quand même 14.
        let lying = [
            mv(Kind::Entree, 30.0, "2026-01-05", 1),
            mv(Kind::Sortie, 14.0, "2026-01-08", 2),
            Move {
                cancels: 2,
                ..mv(Kind::Annulation, 999.0, "2026-01-09", 3)
            },
        ];
        assert!((balance(&lying) - 30.0).abs() < 1e-9);
        // Annuler une réception fait redescendre le stock.
        let returned = [
            mv(Kind::Entree, 30.0, "2026-01-05", 1),
            cancel(1, "2026-01-06", 2),
        ];
        assert_eq!(balance(&returned), 0.0);
        // Et une perte annulée rend ce qu'elle avait pris.
        let broken = [
            mv(Kind::Entree, 30.0, "2026-01-05", 1),
            mv(Kind::Perte, 4.0, "2026-01-06", 2),
            cancel(2, "2026-01-07", 3),
        ];
        assert!((balance(&broken) - 30.0).abs() < 1e-9);
    }

    /// Annuler un inventaire rend au registre ce qu'il disait avant lui.
    ///
    /// Un inventaire **pose** le solde : il n'y a rien à soustraire pour
    /// le défaire, et le seul nombre qui permette de le faire est celui
    /// que le registre affichait au moment du comptage. C'est pour cela
    /// qu'il est écrit dans la base et non recalculé à la lecture — un
    /// recalcul d'aujourd'hui donnerait le solde d'aujourd'hui.
    #[test]
    fn cancelling_a_count_gives_the_register_back_what_it_said() {
        // 30 entrés, 14 sortis : le registre dit 16. Quelqu'un compte
        // 5 — il a compté la mauvaise boîte.
        let wrong = [
            mv(Kind::Entree, 30.0, "2026-01-05", 1),
            mv(Kind::Sortie, 14.0, "2026-01-08", 2),
            count(5.0, 16.0, "2026-01-10", 3),
        ];
        assert!((balance(&wrong) - 5.0).abs() < 1e-9);
        let fixed = [
            mv(Kind::Entree, 30.0, "2026-01-05", 1),
            mv(Kind::Sortie, 14.0, "2026-01-08", 2),
            count(5.0, 16.0, "2026-01-10", 3),
            cancel(3, "2026-01-10", 4),
        ];
        assert!((balance(&fixed) - 16.0).abs() < 1e-9, "{}", balance(&fixed));
        // Ce que le registre continue de porter, c'est les deux lignes :
        // le comptage fautif et son annulation. La faute ne disparaît
        // pas, elle se lit barrée.
        assert!(is_cancelled(&fixed, 3));
        assert!(!is_cancelled(&fixed, 2));
        assert_eq!(fixed.len(), 4);
    }

    /// Une annulation dont la cible manque ne fait rien.
    ///
    /// Le cas arrive si une tranche est lue produit par produit et qu'une
    /// ligne désigne l'ailleurs. Ne rien faire est le seul choix qui
    /// n'invente pas un mouvement ; deviner un sens serait déplacer un
    /// stock sur une hypothèse.
    #[test]
    fn a_cancellation_without_its_target_moves_nothing() {
        let orphan = [
            mv(Kind::Entree, 30.0, "2026-01-05", 1),
            cancel(77, "2026-01-06", 2),
        ];
        assert!((balance(&orphan) - 30.0).abs() < 1e-9);
        // Et une annulation d'annulation ne fait rien non plus : le
        // registre refuse de l'écrire, et l'arithmétique refuse de la
        // lire, pour que les deux disent la même chose.
        let stacked = [
            mv(Kind::Entree, 30.0, "2026-01-05", 1),
            cancel(1, "2026-01-06", 2),
            cancel(2, "2026-01-07", 3),
        ];
        assert_eq!(balance(&stacked), 0.0);
        assert!(Kind::Entree.can_be_cancelled());
        assert!(!Kind::Annulation.can_be_cancelled());
        // La courbe suit le même calcul : elle remonte à l'annulation.
        assert_eq!(running(&stacked), vec![30.0, 0.0, 0.0]);
    }

    /// Le numéro d'ordonnancier ne revient jamais en arrière et ne
    /// rebouche jamais un trou.
    ///
    /// Un numéro annulé l'est par une contre-passation, et le réattribuer
    /// ferait exister deux délivrances sous le même numéro — c'est-à-dire
    /// un registre qui ne prouve plus rien.
    #[test]
    fn a_dispensing_number_is_never_reused() {
        assert_eq!(next_number(&[]), 1);
        assert_eq!(next_number(&[1, 2, 3]), 4);
        // Le 3 a été annulé : le suivant est quand même le 5.
        assert_eq!(next_number(&[1, 2, 4]), 5);
        // L'ordre dans lequel la base les rend ne change rien.
        assert_eq!(next_number(&[4, 1, 2]), 5);
        assert_eq!(number_label(2026, 42), "2026-0042");
        assert_eq!(number_label(2026, 1), "2026-0001");
        // Au-delà de dix mille, le numéro s'écrit en entier plutôt que
        // d'être tronqué : une officine qui délivre beaucoup ne perd pas
        // ses quatre premiers chiffres.
        assert_eq!(number_label(2026, 12345), "2026-12345");
    }

    /// Une clé que cette version ne connaît pas est lue comme une perte.
    ///
    /// Le choix qui sous-estime le stock plutôt que de le surestimer :
    /// une ligne écrite par une version plus récente doit faire chercher
    /// une explication, jamais rassurer.
    #[test]
    fn an_unknown_line_lowers_the_stock_rather_than_raising_it() {
        for k in Kind::ALL.into_iter().chain([Kind::Annulation]) {
            assert_eq!(Kind::from_key(k.as_key()), k);
        }
        assert_eq!(Kind::from_key("QUELQUE CHOSE"), Kind::Perte);
        assert_eq!(Kind::from_key(""), Kind::Perte);
        // Et seule la délivrance porte un numéro et un dossier.
        assert!(Kind::Sortie.is_dispensing());
        for k in [
            Kind::Entree,
            Kind::Inventaire,
            Kind::Perte,
            Kind::Annulation,
        ] {
            assert!(!k.is_dispensing(), "{k:?}");
        }
        // L'annulation ne se choisit pas dans le formulaire : elle se
        // demande sur la ligne à annuler.
        assert!(!Kind::ALL.contains(&Kind::Annulation));
    }

    /// La liste de contrôle : un produit, un motif, le plus grave.
    #[test]
    fn the_control_list_says_what_to_go_and_count_and_why() {
        let f = |id: i64, label: &str, stock: f64, threshold: f64, last: &str| Followed {
            id,
            label: label.to_owned(),
            unit: "comprimé".to_owned(),
            stock,
            threshold,
            last_count: last.to_owned(),
        };
        let base = [
            // Impossible : une ligne manque au registre.
            f(1, "Skenan LP 30 mg", -2.0, 10.0, "2026-08-01"),
            // Sous le plancher que l'officine s'est donné.
            f(2, "Oxycontin 10 mg", 4.0, 10.0, "2026-08-20"),
            // Compté il y a trop longtemps.
            f(3, "Durogesic 25", 40.0, 5.0, "2025-06-01"),
            // Jamais compté.
            f(4, "Méthadone 40 mg", 25.0, 5.0, ""),
            // Rien à signaler.
            f(5, "Subutex 8 mg", 60.0, 10.0, "2026-08-25"),
            // Pas de plancher posé : zéro veut dire « pas de plancher »
            // et non « plancher à zéro », donc un stock bas ne suffit
            // pas — c'est le comptage récent qui le tient hors liste.
            f(6, "Ritaline 10 mg", 1.0, 0.0, "2026-08-25"),
        ];
        let list = to_check(&base, "2026-08-29", 60);
        let ids: Vec<i64> = list.iter().map(|c| c.id).collect();
        assert_eq!(
            ids,
            vec![1, 2, 4, 3],
            "l'impossible d'abord, le rappel après"
        );
        assert_eq!(list[0].why, Why::Negative);
        assert_eq!(list[1].why, Why::Low);
        assert_eq!(list[2].why, Why::Uncounted);
        assert_eq!(
            list[2].days, None,
            "jamais compté n'est pas « il y a n jours »"
        );
        assert_eq!(list[3].why, Why::Uncounted);
        assert_eq!(list[3].days, Some(454));
        // Un produit n'est jamais sur la liste deux fois, sous deux
        // motifs : la ligne à aller chercher est une ligne.
        let mut seen: Vec<i64> = list.iter().map(|c| c.id).collect();
        let n = seen.len();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), n);
        // Un stock exactement au plancher est déjà bas : le plancher est
        // le moment de recommander, pas celui d'être à court.
        let at = [f(9, "Pile au seuil", 10.0, 10.0, "2026-08-25")];
        assert_eq!(to_check(&at, "2026-08-29", 60)[0].why, Why::Low);
    }

    /// Chaque présentation nomme son dosage.
    ///
    /// C'est ce qui distingue un catalogue de registre d'une liste de
    /// molécules. Ce qu'on compte dans le coffre, ce n'est pas « de la
    /// morphine », c'est des gélules de 30 mg : une ligne « Skenan LP »
    /// sans dosage ferait un solde qui mélange cinq boîtes différentes,
    /// et cette confusion-là ne se rattrape pas — le registre est
    /// inaltérable.
    #[test]
    fn every_presentation_names_its_dosage_and_what_is_counted() {
        for family in CATALOGUE {
            assert!(!family.items.is_empty(), "{} est vide", family.name);
            for (label, unit) in family.items {
                assert!(
                    label.chars().any(|c| c.is_ascii_digit()),
                    "« {label} » ne porte pas de dosage"
                );
                assert!(
                    !unit.trim().is_empty(),
                    "« {label} » ne dit pas ce qu'on compte"
                );
                // Le libellé est ce qui s'écrit sur la ligne du registre :
                // il tient sur une ligne.
                assert!(label.chars().count() <= 40, "« {label} » est trop long");
            }
        }
    }

    /// Deux produits ne portent jamais le même libellé.
    ///
    /// Deux « Skenan LP 30 mg » dans la liste, ce sont deux soldes pour
    /// une boîte, et le contrôle trouve un manquant qui n'existe pas.
    /// La base le refuse déjà ; le catalogue ne doit pas le proposer.
    #[test]
    fn the_catalogue_never_offers_the_same_label_twice() {
        let mut labels: Vec<&str> = CATALOGUE
            .iter()
            .flat_map(|f| f.items.iter().map(|(l, _)| *l))
            .collect();
        let total = labels.len();
        labels.sort_unstable();
        labels.dedup();
        assert_eq!(labels.len(), total, "un libellé apparaît deux fois");
        // Le cliquet : le catalogue ne perd pas de présentations.
        assert!(total >= 106, "le catalogue a maigri : {total}");
        assert_eq!(total, catalogue_size());
    }

    /// Chaque famille dit sa règle, et la durée qu'elle porte est une
    /// durée réglementaire et non un chiffre rond.
    ///
    /// Les trois qui existent sont 28 jours, 14 pour le sirop de
    /// méthadone et 7 pour la voie parentérale. Une quatrième valeur
    /// dans cette table serait une invention, et c'est sur ce nombre
    /// qu'on refuse une ordonnance à un patient.
    #[test]
    fn every_family_carries_its_rule_and_a_lawful_duration() {
        for family in CATALOGUE {
            assert!(
                [7, 14, 28].contains(&family.max_days),
                "{} : {} jours n'est pas une durée réglementaire",
                family.name,
                family.max_days
            );
            assert!(
                family.note.chars().count() >= 80,
                "{} n'explique pas sa règle",
                family.name
            );
            // Le régime se relit tel qu'il est écrit, et il a de quoi
            // se dire à l'écran : un libellé et la phrase qui explique
            // ce qu'il demande.
            let status = Status::from_key(family.status);
            assert_eq!(status.as_key(), family.status, "{}", family.name);
            assert!(!crate::strings::tr(status.label_key()).is_empty());
            assert!(crate::strings::tr(status.note_key()).len() > 40);
            // Et un assimilé le dit, dans sa note, en toutes lettres :
            // le laisser passer pour un stupéfiant ferait tenir un
            // registre que la loi ne demande pas et croire à une
            // obligation qui n'existe pas.
            if status == Status::Assimile {
                assert!(
                    family.note.contains("registre"),
                    "{} ne dit pas ce que son régime demande",
                    family.name
                );
            }
        }
        // Les deux régimes sont représentés : une table qui n'aurait que
        // des stupéfiants n'aurait pas eu besoin du champ.
        let assimile = CATALOGUE
            .iter()
            .filter(|f| Status::from_key(f.status) == Status::Assimile)
            .count();
        assert!(assimile >= 1);
        assert!(assimile < CATALOGUE.len());
        // Ce que cette version ne connaît pas est lu comme le régime le
        // plus exigeant : se tromper dans ce sens fait tenir un registre
        // de trop, dans l'autre il en manque un.
        assert_eq!(Status::from_key("AUTRE CHOSE"), Status::Stupefiant);
    }

    /// Le compte des jours, y compris à travers un 29 février.
    #[test]
    fn the_days_are_counted_across_years_and_leap_days() {
        assert_eq!(days_between("2026-08-01", "2026-08-29"), Some(28));
        assert_eq!(days_between("2026-08-29", "2026-08-29"), Some(0));
        // 2024 est bissextile : février y a vingt-neuf jours.
        assert_eq!(days_between("2024-02-28", "2024-03-01"), Some(2));
        // 2023 ne l'est pas.
        assert_eq!(days_between("2023-02-28", "2023-03-01"), Some(1));
        // 2000 est bissextile, 1900 ne l'était pas : la règle des
        // siècles, que la plupart des calculs à la main oublient.
        assert_eq!(days_between("2000-02-28", "2000-03-01"), Some(2));
        assert_eq!(days_between("1900-02-28", "1900-03-01"), Some(1));
        // Une année entière.
        assert_eq!(days_between("2025-01-01", "2026-01-01"), Some(365));
        // Une date en arrière compte en négatif, et ce qui n'est pas une
        // date ne compte pas du tout.
        assert_eq!(days_between("2026-08-29", "2026-08-01"), Some(-28));
        for bad in ["", "hier", "2026-13-01", "2026-08-32", "2026-08", "x-y-z"] {
            assert_eq!(days_between(bad, "2026-08-29"), None, "{bad}");
            assert_eq!(days_between("2026-08-29", bad), None, "{bad}");
        }
    }

    /// Un rythme mesuré sur une fenêtre vide n'est pas un rythme de
    /// zéro, et une ampoule cassée n'est pas de la consommation.
    #[test]
    fn a_broken_ampoule_is_not_consumption_and_an_empty_window_is_not_a_rate() {
        let none: Vec<Move> = Vec::new();
        assert_eq!(velocity(&none, "2026-09-01", 30), None);
        // Des lignes, mais toutes hors fenêtre : « rien à dire », et non
        // « zéro par jour ». Les deux ne se peignent pas pareil.
        let old = [mv(Kind::Sortie, 14.0, "2026-01-05", 1)];
        assert_eq!(velocity(&old, "2026-09-01", 30), None);
        // Une fenêtre sans durée n'est pas une fenêtre.
        let now = [mv(Kind::Sortie, 14.0, "2026-08-25", 1)];
        assert_eq!(velocity(&now, "2026-09-01", 0), None);

        let v = velocity(&now, "2026-09-01", 30).expect("une délivrance dans la fenêtre");
        assert!((v.out - 14.0).abs() < 1e-9);
        assert_eq!(v.lines, 1);

        // La perte est du stock parti, pas de la demande : elle ne monte
        // pas le rythme, et l'entrée non plus.
        let mixed = [
            mv(Kind::Sortie, 14.0, "2026-08-25", 1),
            mv(Kind::Perte, 30.0, "2026-08-26", 2),
            mv(Kind::Entree, 60.0, "2026-08-27", 3),
        ];
        let v2 = velocity(&mixed, "2026-09-01", 30).expect("la délivrance compte seule");
        assert!((v2.out - 14.0).abs() < 1e-9, "{}", v2.out);
        assert_eq!(v2.lines, 1);

        // Et une délivrance annulée n'a pas eu lieu.
        let undone = [
            mv(Kind::Sortie, 14.0, "2026-08-25", 1),
            cancel(1, "2026-08-26", 2),
        ];
        assert_eq!(velocity(&undone, "2026-09-01", 30), None);
    }

    /// Les jours de stock sont une lecture du passé, pas une promesse.
    #[test]
    fn days_of_stock_are_a_reading_of_the_past_and_not_a_promise() {
        let rows = [
            mv(Kind::Sortie, 15.0, "2026-08-25", 1),
            mv(Kind::Sortie, 15.0, "2026-08-28", 2),
        ];
        let v = velocity(&rows, "2026-09-01", 30).expect("deux délivrances");
        // 30 unités sur 30 jours : une par jour.
        assert!((v.per_day() - 1.0).abs() < 1e-9, "{}", v.per_day());
        assert_eq!(days_left(12.0, v), Some(12));
        // Un solde négatif est un registre faux, pas une prévision.
        assert_eq!(days_left(-2.0, v), None);
        // Et rien qui bouge ne se projette pas.
        let still = Velocity {
            out: 0.0,
            days: 30,
            lines: 0,
        };
        assert_eq!(days_left(40.0, still), None);
    }

    /// Un manque et un excès ne s'annulent jamais l'un l'autre.
    #[test]
    fn a_shortfall_and_an_excess_never_cancel_each_other_out() {
        let rows = [
            mv(Kind::Entree, 30.0, "2026-01-05", 1),
            count(27.0, 30.0, "2026-02-01", 2),
            count(33.0, 30.0, "2026-03-01", 3),
            count(30.0, 30.0, "2026-04-01", 4),
        ];
        let list = counts(&rows);
        assert_eq!(list.len(), 3, "trois comptages, dans l'ordre du registre");
        let g = gaps(&list);
        assert_eq!(g.taken, 3);
        assert_eq!(g.matched, 1, "un seul tombe juste");
        assert!((g.short - 3.0).abs() < 1e-9, "{}", g.short);
        assert!((g.over - 3.0).abs() < 1e-9, "{}", g.over);
        // Le net serait zéro, et zéro dirait « rien à signaler » là où il
        // y a deux comptages inexpliqués.
    }

    /// Un comptage annulé reste dans l'histoire et sort des totaux.
    #[test]
    fn a_cancelled_count_stays_in_the_history_and_out_of_the_totals() {
        let rows = [
            mv(Kind::Entree, 30.0, "2026-01-05", 1),
            count(5.0, 30.0, "2026-02-01", 2),
            cancel(2, "2026-02-02", 3),
        ];
        let list = counts(&rows);
        assert_eq!(list.len(), 1);
        assert!(list[0].cancelled, "il reste écrit, et il se lit barré");
        let g = gaps(&list);
        assert_eq!(g.taken, 0, "mais il ne compte pas");
        assert!((g.short).abs() < 1e-9);
    }

    /// Une délivrance annulée garde son numéro et ne fait pas un trou.
    ///
    /// C'est ce qu'un détecteur naïf rate, et le rater ferait ressembler
    /// chaque correction du registre à une dissimulation.
    #[test]
    fn a_cancelled_delivery_is_not_a_hole_in_the_sequence() {
        let s = sequence(&[1, 2, 3]).expect("trois numéros");
        assert_eq!((s.first, s.last, s.count), (1, 3, 3));
        assert!(s.holes.is_empty(), "{:?}", s.holes);
        assert!(s.doubled.is_empty());
    }

    /// Une suite qui commence à trois cents le dit et n'invente pas de
    /// trous.
    #[test]
    fn a_sequence_that_starts_at_three_hundred_says_so_and_invents_no_holes() {
        let s = sequence(&[300, 301, 302]).expect("trois numéros");
        assert_eq!(s.first, 300);
        assert!(
            s.holes.is_empty(),
            "deux cent quatre-vingt-dix-neuf trous fantômes feraient un détecteur              que personne n'ouvre deux fois : {:?}",
            s.holes
        );
        // Un vrai trou intérieur, lui, se voit — et il porte sa taille.
        let gap = sequence(&[300, 301, 305, 306]).expect("un trou");
        assert_eq!(gap.holes, vec![Hole { from: 302, to: 304 }]);
        assert_eq!(gap.holes[0].count(), 3);
        assert_eq!(sequence(&[]), None);
    }

    /// Un numéro servi deux fois est pire qu'un numéro manquant.
    #[test]
    fn a_number_served_twice_is_worse_than_a_number_missing() {
        let s = sequence(&[1, 2, 2, 4]).expect("quatre lignes");
        assert_eq!(s.doubled, vec![2]);
        assert_eq!(
            s.holes,
            vec![Hole { from: 3, to: 3 }],
            "le trou et le doublon se rapportent séparément"
        );
        assert_eq!(
            s.count, 4,
            "le compte est celui des lignes, pas des numéros"
        );
    }
}
