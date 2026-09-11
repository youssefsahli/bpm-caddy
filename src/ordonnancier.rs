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

/// Les laboratoires dont une officine française voit passer les boîtes.
///
/// **Ce n'est pas une donnée clinique**, et c'est pour cela qu'elle peut
/// être livrée : ce sont des noms de laboratoires, pas une affirmation
/// sur ce que chacun commercialise. L'application ne dit jamais « EG
/// vend le 36 mg » — elle propose d'écrire « EG » sur la ligne du
/// produit que l'officine a en main, ce qui est la seule façon honnête
/// de suivre deux génériques d'un même dosage.
///
/// Et c'est nécessaire : le méthylphénidate LP 36 mg d'un laboratoire et
/// celui d'un autre sont deux boîtes, avec deux codes, sur la même
/// étagère. Un registre qui les confond compte juste et ne permet plus
/// d'aller chercher la bonne boîte — ce qui est tout ce qu'un comptage
/// physique demande.
///
/// La liste est ouverte : le champ à côté accepte n'importe quoi, parce
/// qu'un laboratoire de plus est une ligne à écrire et pas une mise à
/// jour à attendre.
/// L'ordre est celui des boîtes qu'une officine française voit le plus
/// souvent, et non l'alphabet : la liste est **coupée à ce qui tient sur
/// une rangée**, et l'alphabet mettrait Accord et Arrow devant Biogaran.
pub const LABS: &[&str] = &[
    "Biogaran", "EG", "Viatris", "Sandoz", "Teva", "Zentiva", "Arrow", "Accord", "Cristers",
    "Krka", "Mylan",
];

/// Le libellé d'une présentation suivie sous un laboratoire donné.
///
/// Écrit **une fois** : c'est ce qui s'inscrit sur chaque ligne du
/// registre, et deux façons de le composer donneraient deux produits
/// pour une boîte le jour où l'une des deux gagne une espace.
pub fn labelled(item: &str, lab: &str) -> String {
    let lab = lab.trim();
    if lab.is_empty() {
        item.trim().to_owned()
    } else {
        format!("{} ({lab})", item.trim())
    }
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
            // Le générique se délivre sous le nom de la molécule, et
            // c'est *cette* ligne-là que le laboratoire vient préciser.
            ("Morphine sulfate LP 10 mg", "gélule"),
            ("Morphine sulfate LP 30 mg", "gélule"),
            ("Morphine sulfate LP 60 mg", "gélule"),
            ("Morphine sulfate LP 100 mg", "gélule"),
            ("Morphine sulfate LP 200 mg", "gélule"),
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
            ("Morphine sulfate 5 mg", "gélule"),
            ("Morphine sulfate 10 mg", "gélule"),
            ("Morphine sulfate 20 mg", "gélule"),
            ("Morphine sulfate 30 mg", "gélule"),
        ],
    },
    Family {
        name: "Opioïdes injectables",
        status: "STUPEFIANT",
        max_days: 7,
        note: "Voie parentérale : prescription limitée à 7 jours, portée à 28 jours lorsque l'administration se fait par un système actif de perfusion. C'est la seule famille où la durée n'est pas de 28 jours, et l'oublier fait délivrer une ordonnance périmée.",
        items: &[
            ("Chlorhydrate de morphine 1 mg/mL", "ampoule"),
            ("Chlorhydrate de morphine 10 mg/mL", "ampoule"),
            ("Chlorhydrate de morphine 20 mg/mL", "ampoule"),
            ("Chlorhydrate de morphine 40 mg/mL", "ampoule"),
            ("Chlorhydrate de morphine 50 mg/5 mL", "ampoule"),
            ("Chlorhydrate de morphine 100 mg/10 mL", "ampoule"),
            ("Oxycodone 10 mg/mL injectable", "ampoule"),
            ("Oxycodone 50 mg/mL injectable", "ampoule"),
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
            ("Oxycontin LP 15 mg", "comprimé"),
            ("Oxycontin LP 20 mg", "comprimé"),
            ("Oxycontin LP 30 mg", "comprimé"),
            ("Oxycontin LP 40 mg", "comprimé"),
            ("Oxycontin LP 60 mg", "comprimé"),
            ("Oxycontin LP 80 mg", "comprimé"),
            ("Oxycontin LP 120 mg", "comprimé"),
            ("Oxycodone LP 5 mg", "comprimé"),
            ("Oxycodone LP 10 mg", "comprimé"),
            ("Oxycodone LP 15 mg", "comprimé"),
            ("Oxycodone LP 20 mg", "comprimé"),
            ("Oxycodone LP 30 mg", "comprimé"),
            ("Oxycodone LP 40 mg", "comprimé"),
            ("Oxycodone LP 60 mg", "comprimé"),
            ("Oxycodone LP 80 mg", "comprimé"),
            ("Oxycodone LP 120 mg", "comprimé"),
            ("Oxynorm 5 mg", "gélule"),
            ("Oxynorm 10 mg", "gélule"),
            ("Oxynorm 20 mg", "gélule"),
            ("Oxycodone 5 mg", "gélule"),
            ("Oxycodone 10 mg", "gélule"),
            ("Oxycodone 20 mg", "gélule"),
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
            ("Fentanyl 12 µg/h", "dispositif transdermique"),
            ("Fentanyl 25 µg/h", "dispositif transdermique"),
            ("Fentanyl 50 µg/h", "dispositif transdermique"),
            ("Fentanyl 75 µg/h", "dispositif transdermique"),
            ("Fentanyl 100 µg/h", "dispositif transdermique"),
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
            // Les génériques, sous le nom de la molécule. Ce sont ceux
            // que le laboratoire vient préciser : « Méthylphénidate LP
            // 36 mg (EG) » et « … (Biogaran) » sont deux boîtes sur la
            // même étagère, et le comptage physique les distingue.
            //
            // Deux formes LP et non une : le comprimé osmotique et la
            // gélule à microgranules ne se libèrent pas pareil et ne
            // sont pas interchangeables — c'est la règle de la famille,
            // et un catalogue qui les mélangerait l'enseignerait mal.
            ("Méthylphénidate LP 18 mg", "comprimé"),
            ("Méthylphénidate LP 36 mg", "comprimé"),
            ("Méthylphénidate LP 54 mg", "comprimé"),
            ("Méthylphénidate LP 10 mg", "gélule"),
            ("Méthylphénidate LP 20 mg", "gélule"),
            ("Méthylphénidate LP 30 mg", "gélule"),
            ("Méthylphénidate LP 40 mg", "gélule"),
            ("Méthylphénidate 10 mg", "comprimé"),
        ],
    },
    Family {
        name: "Lisdexamfétamine",
        status: "STUPEFIANT",
        max_days: 28,
        note: "Prodrogue de la dexamfétamine : elle n'est active qu'une fois hydrolysée dans le sang, ce qui lui donne son délai d'action d'une heure et rend l'écrasement ou l'inhalation sans intérêt. Prescription initiale annuelle réservée aux spécialistes, ordonnance sécurisée, 28 jours. Une prise unique le matin ; une seconde dans la journée fait passer la nuit debout. La gélule peut s'ouvrir et se disperser dans un verre d'eau, à boire aussitôt.",
        items: &[
            ("Elvanse 20 mg", "gélule"),
            ("Elvanse 30 mg", "gélule"),
            ("Elvanse 40 mg", "gélule"),
            ("Elvanse 50 mg", "gélule"),
            ("Elvanse 60 mg", "gélule"),
            ("Elvanse 70 mg", "gélule"),
            ("Elvanse Adulte 30 mg", "gélule"),
            ("Elvanse Adulte 50 mg", "gélule"),
            ("Elvanse Adulte 70 mg", "gélule"),
        ],
    },
    Family {
        name: "Oxybate de sodium",
        status: "STUPEFIANT",
        max_days: 28,
        note: "Les deux prises se font déjà couché, la seconde deux heures et demie à quatre heures après la première, et au moins deux heures après le dîner. L'alcool et tout autre dépresseur respiratoire sont formellement contre-indiqués le soir de la prise.",
        items: &[
            ("Xyrem 500 mg/mL solution buvable", "flacon"),
            ("Oxybate de sodium 500 mg/mL buvable", "flacon"),
        ],
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
    /// Casse, péremption, vol : le stock descend, hors délivrance. La
    /// ligne porte ce qui s'est passé.
    Perte,
    /// Ce qu'un patient rapporte. Le stock délivrable **ne bouge pas**,
    /// et c'est toute la question — voir [`Balance`].
    Retour,
    /// Ce qui a été détruit : le stock à détruire descend. La ligne
    /// porte le procès-verbal, et elle ne s'écrit pas sans lui.
    Destruction,
    /// Ce qui a **périmé au coffre**. Le stock délivrable descend, et
    /// le troisième compte monte — voir [`Balance`].
    ///
    /// C'était une perte, et c'était faux : une perte est ce qui n'est
    /// plus là, cassé, volé, écoulé. Une boîte périmée est toujours
    /// dans le coffre, et elle y reste jusqu'au procès-verbal. Notée en
    /// perte, elle disparaissait du registre en restant sur l'étagère —
    /// exactement l'erreur que le compte des retours patients avait
    /// été créé pour réparer, sur l'autre étagère.
    Peremption,
    /// Ce qui quitte le coffre parmi les périmés : le troisième compte
    /// descend. Comme la destruction des retours, la ligne porte son
    /// procès-verbal et ne s'écrit pas sans lui.
    ///
    /// Une nature à elle, et non la même que [`Kind::Destruction`] :
    /// une ligne de registre dit **ce qu'elle a fait**, et deux piles
    /// vidées par la même nature obligeraient à deviner laquelle.
    DestructionPerimes,
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
            Kind::Retour => "RETOUR",
            Kind::Destruction => "DESTRUCTION",
            Kind::Peremption => "PEREMPTION",
            Kind::DestructionPerimes => "DESTRUCTION_PERIMES",
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
            "RETOUR" => Kind::Retour,
            "DESTRUCTION" => Kind::Destruction,
            "PEREMPTION" => Kind::Peremption,
            "DESTRUCTION_PERIMES" => Kind::DestructionPerimes,
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
            Kind::Retour => "stup_kind_retour",
            Kind::Destruction => "stup_kind_destruction",
            Kind::Peremption => "stup_kind_peremption",
            Kind::DestructionPerimes => "stup_kind_destruction_perimes",
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
            Kind::Retour => 4,
            Kind::Destruction => 6,
            Kind::Peremption => 3,
            Kind::DestructionPerimes => 6,
            Kind::Annulation => 5,
        }
    }

    /// Porte-t-elle un numéro d'ordonnancier et un dossier ? Seule la
    /// délivrance en porte : une réception n'a pas de patient, et en
    /// inventer un serait écrire un nom dans un registre pour rien.
    pub fn is_dispensing(self) -> bool {
        self == Kind::Sortie
    }

    /// Porte-t-elle un numéro de dossier ?
    ///
    /// **Séparé de [`Self::is_dispensing`], et c'est le point.** Cette
    /// question-là servait aux deux : porter un dossier et prendre un
    /// numéro d'ordonnancier. Un retour vient de quelqu'un — c'est même
    /// la seule chose qui compte le jour où l'on cherche d'où sortent
    /// quarante gélules de morphine — et il ne prend aucun numéro : le
    /// numéro d'ordonnancier est celui d'une délivrance, et lui en
    /// donner un ferait un trou dans la suite qui ne dit rien.
    pub fn carries_file(self) -> bool {
        matches!(self, Kind::Sortie | Kind::Retour)
    }

    /// La ligne touche-t-elle une **boîte**, et porte-t-elle donc un
    /// numéro de lot ?
    ///
    /// Une réception, une délivrance, un retour et une destruction en
    /// portent un : chacune passe par une boîte qu'on a en main. Un
    /// inventaire compte un coffre entier — plusieurs lots à la fois —
    /// et une annulation ne fait que défaire une ligne qui, elle,
    /// portait le sien. Écrire un lot sur celles-là dirait qu'on sait
    /// laquelle, et on ne le sait pas.
    ///
    /// C'est ce qui permettra à un rappel de l'ANSM de demander « ce
    /// lot est-il passé chez nous, et chez qui ».
    pub fn carries_lot(self) -> bool {
        matches!(
            self,
            Kind::Entree | Kind::Sortie | Kind::Retour | Kind::Destruction
        )
    }

    /// Écrit-elle dans le stock à détruire plutôt que dans le stock
    /// délivrable ? Voir [`Balance`].
    pub fn is_destruction_side(self) -> bool {
        matches!(self, Kind::Retour | Kind::Destruction)
    }

    /// Écrit-elle dans le compte des périmés ?
    pub fn is_expiry_side(self) -> bool {
        matches!(self, Kind::Peremption | Kind::DestructionPerimes)
    }

    /// La ligne vide un coffre sur procès-verbal, et **ne s'écrit pas
    /// sans lui** : c'est la pièce extérieure à laquelle elle renvoie.
    pub fn needs_record(self) -> bool {
        matches!(self, Kind::Destruction | Kind::DestructionPerimes)
    }

    /// Les huit natures que l'on **écrit**.
    ///
    /// L'annulation n'en est pas : elle ne se choisit pas dans un
    /// formulaire, elle se demande sur la ligne à annuler. Un
    /// « annuler » posé à côté de « réception » et de « délivrance »
    /// serait une façon de plus d'écrire une ligne, alors que c'est
    /// une façon d'en corriger une.
    ///
    /// L'ordre est celui du comptoir : ce qu'on écrit tous les jours
    /// d'abord, ce qui vide le coffre à la fin.
    pub const ALL: [Kind; 8] = [
        Kind::Entree,
        Kind::Sortie,
        Kind::Inventaire,
        Kind::Perte,
        Kind::Peremption,
        Kind::Retour,
        Kind::Destruction,
        Kind::DestructionPerimes,
    ];

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

/// Les deux soldes d'un produit, après une ligne.
///
/// **Deux, et jamais un.** Ce qu'un patient rapporte entre bien dans
/// l'officine, se compte, s'enferme au même coffre et se justifie devant
/// le même contrôle — mais cela ne se délivre plus à personne. Le
/// remettre dans le solde ferait dire au registre qu'il y a quarante
/// gélules disponibles là où il y en a vingt-six et un sac scellé qui
/// attend la destruction ; le passer en perte l'effacerait purement et
/// simplement, alors que l'officine en répond jusqu'au procès-verbal.
///
/// Les deux nombres se calculent donc **dans la même passe**, sur les
/// mêmes lignes triées dans le même ordre : deux fonctions qui
/// reliraient le registre chacune de son côté finiraient par ne plus
/// dire la même chose du même jour.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct Balance {
    /// Ce qui est délivrable.
    pub stock: f64,
    /// Ce qu'un patient a rapporté et qui attend d'être détruit.
    pub to_destroy: f64,
    /// Ce qui a périmé **au coffre** et attend son procès-verbal.
    ///
    /// Un troisième compte et non une part du deuxième : les deux
    /// piles ne suivent pas le même chemin — l'une vient du dehors et
    /// l'officine en répond jusqu'au procès-verbal, l'autre est du
    /// stock qu'elle a acheté et qui n'a jamais quitté le coffre. Les
    /// additionner annoncerait « quarante à détruire » là où il y a
    /// deux sacs scellés qui ne se comptent pas ensemble.
    pub expired: f64,
}

/// Où en sont les deux comptes après toutes ces lignes.
///
/// Ce n'est **pas** une somme. Un inventaire fixe le solde à ce qui a
/// été compté : additionner l'écart *et* poser le compte reviendrait à
/// le compter deux fois, et le registre partirait à la dérive dès le
/// premier comptage qui ne tombe pas juste.
///
/// Les lignes sont triées ici, par jour puis par ordre de saisie : ce
/// que la base rend n'a pas à être dans l'ordre, et un inventaire lu
/// avant les sorties qui le précèdent donnerait un solde faux.
///
/// Une seule fonction pour les deux comptes, et pas une par compte :
/// deux lectures du même registre finiraient par ne plus dire la même
/// chose du même jour.
pub fn balance(moves: &[Move]) -> Balance {
    running(moves).last().copied().unwrap_or_default()
}

/// Ce qu'une ligne fait au solde.
///
/// Séparé parce que l'annulation ne se lit pas sur elle-même : elle
/// défait ce que la ligne qu'elle désigne avait fait, et cette ligne
/// doit donc être retrouvée. Une annulation dont la cible n'est pas dans
/// la tranche ne fait **rien** — ne rien inventer est le seul choix
/// honnête quand on ne sait pas ce qu'on annule.
fn apply(b: &mut Balance, m: &Move, all: &[&Move]) {
    // Ce qu'une ligne ordinaire fait, `sign` valant −1 pour l'annuler.
    // Écrit **une fois** : l'annulation défaisait ce que la ligne
    // faisait, dans un second `match` qui répétait le premier à
    // l'envers, et une nature ajoutée à l'un et oubliée à l'autre est
    // exactement le genre de faute qu'un registre ne peut pas se
    // permettre.
    fn shift(b: &mut Balance, kind: Kind, quantity: f64, sign: f64) {
        match kind {
            Kind::Entree => b.stock += sign * quantity,
            Kind::Sortie | Kind::Perte => b.stock -= sign * quantity,
            // Un retour ne rend rien au stock délivrable : il entre au
            // coffre, du côté de ce qui ne se délivrera plus.
            Kind::Retour => b.to_destroy += sign * quantity,
            Kind::Destruction => b.to_destroy -= sign * quantity,
            // Une péremption sort du délivrable **sans disparaître** :
            // la boîte est encore au coffre, et elle y reste jusqu'au
            // procès-verbal.
            Kind::Peremption => {
                b.stock -= sign * quantity;
                b.expired += sign * quantity;
            }
            Kind::DestructionPerimes => b.expired -= sign * quantity,
            // Un inventaire **pose** le solde, il ne s'y ajoute pas :
            // il n'a donc pas de contraire, et son annulation est
            // traitée à part.
            Kind::Inventaire | Kind::Annulation => {}
        }
    }
    match m.kind {
        Kind::Inventaire => b.stock = m.quantity,
        Kind::Annulation => {
            let Some(target) = all.iter().find(|t| t.seq == m.cancels) else {
                return;
            };
            // Rendre au solde ce que le comptage lui avait posé : le
            // registre redit ce qu'il disait avant. C'est la seule
            // raison pour laquelle `expected` est écrit dans la base au
            // lieu d'être recalculé à la lecture.
            if target.kind == Kind::Inventaire {
                b.stock = target.expected;
            } else {
                shift(b, target.kind, target.quantity, -1.0);
            }
        }
        kind => shift(b, kind, m.quantity, 1.0),
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

/// Les deux soldes ligne après ligne, dans l'ordre du registre.
///
/// Le dessin lit ça et rien d'autre — refaire l'arithmétique dans la vue
/// serait deux versions de la même règle, et un jour elles diffèrent.
pub fn running(moves: &[Move]) -> Vec<Balance> {
    let mut ordered: Vec<&Move> = moves.iter().collect();
    ordered.sort_by(|a, b| a.day.cmp(b.day).then(a.seq.cmp(&b.seq)));
    let mut b = Balance::default();
    let mut out = Vec::with_capacity(ordered.len());
    for m in &ordered {
        apply(&mut b, m, &ordered);
        out.push(b);
    }
    out
}

/// Depuis quel jour quelque chose attend d'être détruit.
///
/// Le jour où le stock à détruire a **quitté zéro** pour la dernière
/// fois, et non celui du plus ancien retour : entre les deux il y a
/// peut-être eu une destruction, et dater d'un sac déjà parti ferait
/// dire à l'écran « en attente depuis onze mois » d'un retour d'avant-
/// hier. `None` quand rien n'attend.
///
/// C'est ce que l'onglet trie : un coffre où dort un sac depuis un an
/// est un problème, et il n'a pas d'autre façon de se signaler — aucune
/// échéance ne tombe, personne ne le réclame.
pub fn waiting_since(moves: &[Move]) -> Option<String> {
    let mut ordered: Vec<&Move> = moves.iter().collect();
    ordered.sort_by(|a, b| a.day.cmp(b.day).then(a.seq.cmp(&b.seq)));
    let mut b = Balance::default();
    let mut since: Option<String> = None;
    for m in &ordered {
        let was = b.to_destroy;
        apply(&mut b, m, &ordered);
        if was.abs() <= 1e-6 && b.to_destroy.abs() > 1e-6 {
            since = Some(m.day.to_owned());
        } else if b.to_destroy.abs() <= 1e-6 {
            since = None;
        }
    }
    since
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

/// Des boîtes pleines, ce que chacune contient, et le vrac.
///
/// Un stupéfiant ne se compte pas d'un seul nombre. On sort le coffre,
/// on aligne les boîtes entamées et pleines, et on dit « trois boîtes
/// de quatorze, plus cinq ». Cette multiplication-là se faisait de tête
/// avant d'écrire un seul nombre dans un registre inaltérable, où une
/// erreur ne se corrige que par une contre-passation motivée. Elle se
/// fait ici.
///
/// **La même fonction sert au comptage et à la réception**, et ce n'est
/// pas une économie : c'est la même phrase. Une commande de stupéfiants
/// n'arrive jamais en unités — le grossiste livre trois boîtes
/// d'Actiskenan, qui en contiennent quatorze — alors qu'on en délivre
/// seize. Le registre, lui, ne connaît que l'unité : c'est cette
/// multiplication-là qui manquait, et elle se faisait de tête au moment
/// exact où une erreur coûte le plus cher.
///
/// Les nombres négatifs n'ont pas de sens ici et valent zéro : un champ
/// à moitié tapé — « - », « 1e » — ne doit pas rendre un total qui a
/// l'air d'un résultat.
pub fn boxed_total(boxes: f64, per_box: f64, loose: f64) -> f64 {
    let keep = |n: f64| if n.is_finite() && n > 0.0 { n } else { 0.0 };
    keep(boxes) * keep(per_box) + keep(loose)
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
    /// Ce qui attend d'être détruit — voir [`Balance`].
    pub to_destroy: f64,
    /// Depuis quel jour, ISO ; vide si rien n'attend. Voir
    /// [`waiting_since`].
    pub waiting_since: String,
}

/// Une ligne de ce qui attend la destruction.
#[derive(Clone, PartialEq, Debug)]
pub struct Awaiting {
    pub id: i64,
    pub label: String,
    pub unit: String,
    pub quantity: f64,
    /// Depuis quel jour, ISO ; vide si le registre ne le dit pas.
    pub since: String,
    /// Depuis combien de jours ; `None` si le jour n'est pas lisible.
    pub days: Option<i64>,
}

/// Ce qui dort au coffre en attendant le procès-verbal.
///
/// Trié du plus ancien au plus récent, et **c'est tout le tri** : ce
/// qu'on cherche en ouvrant cet onglet n'est pas le produit dont il y a
/// le plus, c'est celui qui attend depuis le plus longtemps. Un stock à
/// détruire ne réclame rien tout seul — aucune échéance ne tombe, aucun
/// patient ne rappelle — et c'est précisément pour cela qu'il s'oublie.
///
/// Un solde négatif y figure aussi : il veut dire qu'on a détruit plus
/// qu'on n'avait reçu, donc qu'une ligne manque, et le cacher serait
/// cacher la seule erreur que ce compte-là sache montrer.
pub fn awaiting(followed: &[Followed], today: &str) -> Vec<Awaiting> {
    let mut out: Vec<Awaiting> = followed
        .iter()
        .filter(|f| f.to_destroy.abs() > 1e-6)
        .map(|f| Awaiting {
            id: f.id,
            label: f.label.clone(),
            unit: f.unit.clone(),
            quantity: f.to_destroy,
            since: f.waiting_since.clone(),
            days: days_between(&f.waiting_since, today),
        })
        .collect();
    // Le plus ancien d'abord ; ce dont on ne connaît pas le jour passe
    // devant, parce qu'« on ne sait pas depuis quand » est au moins
    // aussi inquiétant que « depuis longtemps ». Puis le nom, pour que
    // la liste de lundi et celle de mardi se comparent.
    out.sort_by(|a, b| {
        b.days
            .unwrap_or(i64::MAX)
            .cmp(&a.days.unwrap_or(i64::MAX))
            .then(a.label.cmp(&b.label))
    });
    out
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

/// Une case de la feuille de saisie : un produit, ce qui a été tapé en
/// face, et de quoi juger la ligne.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Slot<'a> {
    pub stup_id: i64,
    /// Le texte **tel qu'il est tapé**, jamais un nombre déjà lu : une
    /// case vide et une case illisible sont deux choses, et un
    /// `Option<f64>` les confond.
    pub typed: &'a str,
    /// Le solde délivrable du produit, tel que le registre le donne.
    pub expected: f64,
    /// Le motif tapé sur cette ligne.
    pub reason: &'a str,
}

/// Ce qui empêche une case de partir au registre.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Snag {
    /// « 1O », « 1,,5 » : un nombre qu'on n'arrive pas à lire.
    Unreadable,
    /// Zéro là où zéro n'est pas un mouvement.
    NotPositive,
    /// Un comptage qui ne tombe pas sur le registre, sans motif.
    GapWithoutReason,
    /// Une destruction sans sa pièce — voir [`Kind::needs_record`].
    RecordRequired,
}

impl Snag {
    pub fn label_key(self) -> &'static str {
        match self {
            Snag::Unreadable => "batch_snag_unreadable",
            Snag::NotPositive => "batch_snag_zero",
            Snag::GapWithoutReason => "batch_snag_gap",
            Snag::RecordRequired => "batch_snag_record",
        }
    }
}

/// Une ligne prête à partir au registre.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Planned {
    pub stup_id: i64,
    pub quantity: f64,
    /// Le solde d'avant, qu'un inventaire range pour que son annulation
    /// puisse le rendre — voir `add_stup_move`.
    pub expected: f64,
}

/// Ce qu'une feuille de saisie donnerait si on l'inscrivait maintenant.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Plan {
    /// Dans l'ordre de la feuille.
    pub lines: Vec<Planned>,
    /// Les cases qui coincent, et ce qui coince.
    pub snags: Vec<(i64, Snag)>,
}

impl Plan {
    /// **Rien ne part tant que tout ne peut pas partir.**
    ///
    /// Une feuille à moitié écrite laisse le registre avec trois lignes
    /// sur cinq et aucune trace des deux autres — et comme rien ne s'y
    /// efface, les rattraper demande de savoir lesquelles sont passées.
    /// L'écriture est donc une transaction, et ce booléen est ce qui
    /// l'autorise.
    pub fn ready(&self) -> bool {
        self.snags.is_empty() && !self.lines.is_empty()
    }

    /// Ce qui coince sur ce produit, pour l'écrire en face de sa case
    /// plutôt qu'après le refus.
    pub fn snag(&self, stup_id: i64) -> Option<Snag> {
        self.snags
            .iter()
            .find(|(id, _)| *id == stup_id)
            .map(|(_, s)| *s)
    }
}

/// Lire une feuille de saisie groupée : quarante produits alignés, six
/// cases remplies.
///
/// L'inventaire d'un coffre et la délivrance d'une ordonnance qui porte
/// deux stupéfiants sont la même geste — plusieurs produits, une seule
/// date, un seul opérateur — et le formulaire produit par produit les
/// faisait tous les deux en autant d'allers-retours qu'il y a de
/// lignes.
///
/// # Les règles, et pourquoi
///
/// **Une case vide n'est pas un zéro.** C'est la règle qui fait tenir
/// tout le reste : une feuille de quarante produits dont on en compte
/// six écrirait sinon trente-quatre inventaires à zéro, c'est-à-dire
/// qu'elle viderait le coffre sur le papier. Une case qu'on n'a pas
/// touchée n'écrit rien.
///
/// **Une case illisible n'est pas un zéro non plus** — « 1O » avec un O
/// se refuse et se dit, là où le prendre pour zéro écrirait un
/// mouvement que personne n'a voulu.
///
/// **Zéro n'est un chiffre que pour un inventaire** : un coffre compté
/// vide est une information, une délivrance de zéro unité n'en est pas
/// une.
///
/// **Un écart se motive case par case**, et pas une fois pour la
/// feuille : deux produits qui manquent ne manquent pas pour la même
/// raison, et un motif commun n'expliquerait ni l'un ni l'autre. Il en
/// va de même de la pièce que réclame une destruction. La base les
/// redemandera ligne à ligne de toute façon ; le dire ici, c'est le dire
/// **avant** l'écriture, en face de la case, plutôt qu'après un refus
/// qui ne nomme pas la ligne fautive.
pub fn plan(kind: Kind, slots: &[Slot]) -> Plan {
    let mut out = Plan::default();
    for slot in slots {
        let typed = slot.typed.trim();
        if typed.is_empty() {
            continue;
        }
        let Some((quantity, _)) = crate::codex::parse_amount(typed) else {
            out.snags.push((slot.stup_id, Snag::Unreadable));
            continue;
        };
        if quantity <= 0.0 && kind != Kind::Inventaire {
            out.snags.push((slot.stup_id, Snag::NotPositive));
            continue;
        }
        if kind == Kind::Inventaire {
            let gap = Discrepancy {
                expected: slot.expected,
                counted: quantity,
            };
            if gap.matters() && slot.reason.trim().is_empty() {
                out.snags.push((slot.stup_id, Snag::GapWithoutReason));
                continue;
            }
        }
        if kind.needs_record() && slot.reason.trim().is_empty() {
            out.snags.push((slot.stup_id, Snag::RecordRequired));
            continue;
        }
        out.lines.push(Planned {
            stup_id: slot.stup_id,
            quantity,
            expected: slot.expected,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    /// **Une boîte périmée est encore dans le coffre.**
    ///
    /// C'était une perte, et c'était faux : une perte est ce qui n'est
    /// plus là — cassé, volé, écoulé. Une boîte périmée y reste jusqu'au
    /// procès-verbal, et l'officine en répond. Notée en perte, elle
    /// disparaissait du registre en restant sur l'étagère : exactement
    /// l'erreur que le compte des retours patients avait été créé pour
    /// réparer, sur l'autre étagère.
    ///
    /// Trois comptes et non deux, donc — et ils ne s'additionnent pas :
    /// ce qu'un patient rapporte et ce qui a périmé au coffre ne
    /// suivent pas le même chemin, et « quarante à détruire » là où il
    /// y a deux sacs scellés ne veut rien dire.
    #[test]
    fn an_expired_box_leaves_the_shelf_but_not_the_safe() {
        let line = |seq: i64, kind: super::Kind, quantity: f64| super::Move {
            seq,
            day: "2026-09-01",
            kind,
            quantity,
            expected: 0.0,
            cancels: 0,
        };
        let b = super::balance(&[
            line(1, super::Kind::Entree, 30.0),
            line(2, super::Kind::Sortie, 6.0),
            line(3, super::Kind::Peremption, 4.0),
        ]);
        // Le délivrable descend de quatre, et les quatre sont ailleurs
        // — pas nulle part.
        assert!((b.stock - 20.0).abs() < 1e-9, "délivrable {}", b.stock);
        assert!((b.expired - 4.0).abs() < 1e-9, "périmés {}", b.expired);
        assert!(b.to_destroy.abs() < 1e-9, "rien d'un patient");

        // Le procès-verbal vide le troisième compte, et lui seul.
        let after = super::balance(&[
            line(1, super::Kind::Entree, 30.0),
            line(2, super::Kind::Peremption, 4.0),
            line(3, super::Kind::Retour, 7.0),
            line(4, super::Kind::DestructionPerimes, 4.0),
        ]);
        assert!(after.expired.abs() < 1e-9, "périmés détruits");
        assert!(
            (after.to_destroy - 7.0).abs() < 1e-9,
            "le sac du patient n'a pas bougé : {}",
            after.to_destroy
        );

        // Et une annulation défait la péremption des **deux** côtés :
        // une nature ajoutée à l'un et oubliée à l'autre est un stock
        // qui part de travers en silence.
        let mut cancel = line(3, super::Kind::Annulation, 0.0);
        cancel.cancels = 2;
        let undone = super::balance(&[
            line(1, super::Kind::Entree, 30.0),
            line(2, super::Kind::Peremption, 4.0),
            cancel,
        ]);
        assert!((undone.stock - 30.0).abs() < 1e-9, "rendu au délivrable");
        assert!(undone.expired.abs() < 1e-9, "retiré des périmés");
    }

    /// **Un lot n'appartient qu'aux lignes qui touchent une boîte.**
    ///
    /// Une réception, une délivrance, un retour et une destruction en
    /// portent un : chacune passe par une boîte qu'on a en main. Un
    /// inventaire compte un coffre entier — plusieurs lots à la fois —
    /// et une annulation ne fait que défaire une ligne qui, elle,
    /// portait le sien. En écrire un sur celles-là dirait qu'on sait
    /// laquelle, et on ne le sait pas.
    #[test]
    fn a_lot_belongs_only_to_the_lines_that_touch_a_box() {
        use super::Kind;
        for k in [Kind::Entree, Kind::Sortie, Kind::Retour, Kind::Destruction] {
            assert!(k.carries_lot(), "{k:?} passe par une boîte");
        }
        for k in [Kind::Inventaire, Kind::Perte, Kind::Annulation] {
            assert!(!k.carries_lot(), "{k:?} ne nomme pas une boîte");
        }
    }

    /// **Un comptage se fait en boîtes et en vrac.**
    ///
    /// « Trois boîtes de quatorze, plus cinq » fait quarante-sept, et
    /// c'est ce nombre-là qui part au registre. La multiplication se
    /// faisait de tête avant d'écrire dans une pièce inaltérable, où
    /// une erreur ne se défait que par une contre-passation motivée.
    ///
    /// Ce que le test tient en plus du calcul : un champ à moitié tapé
    /// ne rend pas un total qui a l'air d'un résultat. Un « - » seul,
    /// un « 1e » sans exposant, une soustraction — le comptage n'a pas
    /// de nombres négatifs, et un `NaN` qui traverserait rendrait un
    /// total invisible mais faux.
    #[test]
    fn a_count_is_boxes_and_loose_units() {
        use super::boxed_total;
        // Le cas de tous les jours.
        assert!((boxed_total(3.0, 14.0, 5.0) - 47.0).abs() < 1e-9);
        // Rien que du vrac : une boîte entamée et rien d'autre.
        assert!((boxed_total(0.0, 14.0, 9.0) - 9.0).abs() < 1e-9);
        // Rien que des boîtes pleines.
        assert!((boxed_total(2.0, 28.0, 0.0) - 56.0).abs() < 1e-9);
        // Un coffre vide se compte, et il se compte à zéro : c'est un
        // résultat, pas une absence de saisie.
        assert!(boxed_total(0.0, 0.0, 0.0).abs() < 1e-9);
        // Les fractions existent — un sirop se compte en flacons et en
        // millilitres.
        assert!((boxed_total(1.0, 7.0, 2.5) - 9.5).abs() < 1e-9);
        // Et rien de ce qui n'est pas un comptage ne passe.
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -3.0] {
            for total in [
                boxed_total(bad, 14.0, 5.0),
                boxed_total(3.0, bad, 5.0),
                boxed_total(3.0, 14.0, bad),
            ] {
                assert!(total.is_finite(), "{bad} rend {total}");
                assert!(total >= 0.0, "{bad} rend {total}");
            }
        }
        // Un négatif ne se soustrait pas : il ne compte pas.
        assert!((boxed_total(3.0, 14.0, -5.0) - 42.0).abs() < 1e-9);
    }

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
        assert!(
            (balance(&moves).stock - 10.0).abs() < 1e-9,
            "{}",
            balance(&moves).stock
        );

        // Et l'écart de ce comptage se lit pour ce qu'il est.
        let before = balance(&moves[..2]).stock;
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
        assert!((balance(&jumbled).stock - 10.0).abs() < 1e-9);
        // Deux lignes du même jour sont départagées par l'ordre de
        // saisie : l'inventaire du matin puis la sortie de l'après-midi
        // ne donnent pas le même solde que l'inverse.
        let same_day = [
            mv(Kind::Inventaire, 20.0, "2026-02-02", 1),
            mv(Kind::Sortie, 6.0, "2026-02-02", 2),
        ];
        assert!((balance(&same_day).stock - 14.0).abs() < 1e-9);
        let reversed = [
            mv(Kind::Sortie, 6.0, "2026-02-02", 1),
            mv(Kind::Inventaire, 20.0, "2026-02-02", 2),
        ];
        assert!((balance(&reversed).stock - 20.0).abs() < 1e-9);
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
        assert_eq!(
            curve.iter().map(|b| b.stock).collect::<Vec<_>>(),
            vec![30.0, 16.0, 15.0, 13.0]
        );
        assert!((curve.last().copied().unwrap().stock - balance(&moves).stock).abs() < 1e-9);
        assert!(running(&[]).is_empty());
        assert_eq!(balance(&[]), Balance::default());
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
        assert!((balance(&doubled).stock - 2.0).abs() < 1e-9);
        let fixed = [
            mv(Kind::Entree, 30.0, "2026-01-05", 1),
            mv(Kind::Sortie, 14.0, "2026-01-08", 2),
            mv(Kind::Sortie, 14.0, "2026-01-08", 3),
            cancel(3, "2026-01-09", 4),
        ];
        assert!(
            (balance(&fixed).stock - 16.0).abs() < 1e-9,
            "{}",
            balance(&fixed).stock
        );
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
        assert!((balance(&lying).stock - 30.0).abs() < 1e-9);
        // Annuler une réception fait redescendre le stock.
        let returned = [
            mv(Kind::Entree, 30.0, "2026-01-05", 1),
            cancel(1, "2026-01-06", 2),
        ];
        assert_eq!(balance(&returned).stock, 0.0);
        // Et une perte annulée rend ce qu'elle avait pris.
        let broken = [
            mv(Kind::Entree, 30.0, "2026-01-05", 1),
            mv(Kind::Perte, 4.0, "2026-01-06", 2),
            cancel(2, "2026-01-07", 3),
        ];
        assert!((balance(&broken).stock - 30.0).abs() < 1e-9);
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
        assert!((balance(&wrong).stock - 5.0).abs() < 1e-9);
        let fixed = [
            mv(Kind::Entree, 30.0, "2026-01-05", 1),
            mv(Kind::Sortie, 14.0, "2026-01-08", 2),
            count(5.0, 16.0, "2026-01-10", 3),
            cancel(3, "2026-01-10", 4),
        ];
        assert!(
            (balance(&fixed).stock - 16.0).abs() < 1e-9,
            "{}",
            balance(&fixed).stock
        );
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
        assert!((balance(&orphan).stock - 30.0).abs() < 1e-9);
        // Et une annulation d'annulation ne fait rien non plus : le
        // registre refuse de l'écrire, et l'arithmétique refuse de la
        // lire, pour que les deux disent la même chose.
        let stacked = [
            mv(Kind::Entree, 30.0, "2026-01-05", 1),
            cancel(1, "2026-01-06", 2),
            cancel(2, "2026-01-07", 3),
        ];
        assert_eq!(balance(&stacked).stock, 0.0);
        assert!(Kind::Entree.can_be_cancelled());
        assert!(!Kind::Annulation.can_be_cancelled());
        // La courbe suit le même calcul : elle remonte à l'annulation.
        assert_eq!(
            running(&stacked)
                .iter()
                .map(|b| b.stock)
                .collect::<Vec<_>>(),
            vec![30.0, 0.0, 0.0]
        );
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
        // Et seule la délivrance porte un numéro d'ordonnancier.
        assert!(Kind::Sortie.is_dispensing());
        for k in [
            Kind::Entree,
            Kind::Inventaire,
            Kind::Perte,
            Kind::Retour,
            Kind::Destruction,
            Kind::Annulation,
        ] {
            assert!(!k.is_dispensing(), "{k:?}");
        }
        // Le dossier, lui, est une autre question : un retour vient de
        // quelqu'un, et il ne prend pas de numéro pour autant.
        assert!(Kind::Sortie.carries_file());
        assert!(Kind::Retour.carries_file());
        for k in [
            Kind::Entree,
            Kind::Inventaire,
            Kind::Perte,
            Kind::Destruction,
            Kind::Annulation,
        ] {
            assert!(!k.carries_file(), "{k:?}");
        }
        // L'annulation ne se choisit pas dans le formulaire : elle se
        // demande sur la ligne à annuler.
        assert!(!Kind::ALL.contains(&Kind::Annulation));
    }

    /// **Ce qu'un patient rapporte ne revient pas au stock.**
    ///
    /// C'est la règle qui justifie deux soldes plutôt qu'un. Les
    /// quatorze gélules qu'une famille rapporte après un décès sont
    /// bien à l'officine, elles se comptent, elles s'enferment au même
    /// coffre et elles se justifient devant le même contrôle — et elles
    /// ne se délivreront à personne. Un registre qui les remettrait au
    /// solde annoncerait quarante disponibles là où il y en a
    /// vingt-six ; un registre qui les passerait en perte les
    /// effacerait, alors que l'officine en répond jusqu'au
    /// procès-verbal.
    #[test]
    fn what_a_patient_brings_back_never_returns_to_the_stock() {
        let moves = [
            mv(Kind::Entree, 30.0, "2026-01-05", 1),
            mv(Kind::Sortie, 14.0, "2026-01-08", 2),
            // Le patient meurt, la famille rapporte ce qui restait.
            mv(Kind::Retour, 9.0, "2026-01-20", 3),
        ];
        assert!(
            (balance(&moves).stock - 16.0).abs() < 1e-9,
            "{}",
            balance(&moves).stock
        );
        assert!((balance(&moves).to_destroy - 9.0).abs() < 1e-9);
        // La destruction vide l'autre solde et ne touche pas au premier.
        let destroyed = [
            mv(Kind::Entree, 30.0, "2026-01-05", 1),
            mv(Kind::Sortie, 14.0, "2026-01-08", 2),
            mv(Kind::Retour, 9.0, "2026-01-20", 3),
            mv(Kind::Destruction, 9.0, "2026-02-10", 4),
        ];
        assert!((balance(&destroyed).stock - 16.0).abs() < 1e-9);
        assert!(balance(&destroyed).to_destroy.abs() < 1e-9);
        // Et une perte reste une perte : la casse ne va pas au coffre
        // des retours, elle sort du stock. Les deux natures existent
        // précisément parce qu'elles ne disent pas la même chose.
        let broken = [
            mv(Kind::Entree, 30.0, "2026-01-05", 1),
            mv(Kind::Perte, 2.0, "2026-01-06", 2),
        ];
        assert!((balance(&broken).stock - 28.0).abs() < 1e-9);
        assert!(balance(&broken).to_destroy.abs() < 1e-9);
    }

    /// Une annulation défait la ligne du **bon côté**.
    ///
    /// La contre-passation d'un retour ne rend rien au stock
    /// délivrable : elle retire du coffre des retours ce que le retour
    /// y avait mis. Écrite comme un `match` à l'envers du premier, la
    /// règle se serait dédoublée et une nature ajoutée à l'un des deux
    /// aurait rendu le stock faux sans que rien ne le dise — c'est
    /// pourquoi `apply` n'écrit qu'un seul sens et le parcourt à
    /// l'envers.
    #[test]
    fn cancelling_a_return_gives_back_to_the_side_it_came_from() {
        // Un retour saisi deux fois par deux postes.
        let doubled = [
            mv(Kind::Entree, 30.0, "2026-01-05", 1),
            mv(Kind::Retour, 9.0, "2026-01-20", 2),
            mv(Kind::Retour, 9.0, "2026-01-20", 3),
            cancel(3, "2026-01-21", 4),
        ];
        assert!((balance(&doubled).stock - 30.0).abs() < 1e-9);
        assert!((balance(&doubled).to_destroy - 9.0).abs() < 1e-9);
        // Et une destruction annulée remet au coffre ce qu'elle en
        // avait sorti, sans jamais toucher au délivrable.
        let undone = [
            mv(Kind::Entree, 30.0, "2026-01-05", 1),
            mv(Kind::Retour, 9.0, "2026-01-20", 2),
            mv(Kind::Destruction, 9.0, "2026-02-10", 3),
            cancel(3, "2026-02-11", 4),
        ];
        assert!((balance(&undone).stock - 30.0).abs() < 1e-9);
        assert!((balance(&undone).to_destroy - 9.0).abs() < 1e-9);
        // Un inventaire pose le solde délivrable et ne dit rien du
        // coffre des retours : compter le stock ne compte pas le sac
        // scellé, et faire croire le contraire viderait ce compte-là
        // sans qu'aucune ligne ne l'explique.
        let counted = [
            mv(Kind::Entree, 30.0, "2026-01-05", 1),
            mv(Kind::Retour, 9.0, "2026-01-20", 2),
            count(28.0, 30.0, "2026-01-25", 3),
        ];
        assert!((balance(&counted).stock - 28.0).abs() < 1e-9);
        assert!((balance(&counted).to_destroy - 9.0).abs() < 1e-9);
    }

    /// Ce qui attend d'être détruit, et **depuis quand**.
    ///
    /// Depuis le jour où le coffre des retours a quitté zéro pour la
    /// dernière fois, et non depuis le plus ancien retour du registre :
    /// entre les deux il y a peut-être eu une destruction, et dater d'un
    /// sac déjà parti ferait dire « en attente depuis onze mois » d'un
    /// retour d'avant-hier — c'est-à-dire un signal que personne ne
    /// croira la deuxième fois.
    #[test]
    fn what_waits_for_destruction_is_dated_from_when_it_started_waiting() {
        let moves = [
            mv(Kind::Retour, 9.0, "2025-03-02", 1),
            mv(Kind::Destruction, 9.0, "2025-06-10", 2),
            mv(Kind::Retour, 4.0, "2026-08-30", 3),
        ];
        assert_eq!(waiting_since(&moves).as_deref(), Some("2026-08-30"));
        // Rien n'attend : il n'y a pas de date à donner.
        let cleared = [
            mv(Kind::Retour, 9.0, "2025-03-02", 1),
            mv(Kind::Destruction, 9.0, "2025-06-10", 2),
        ];
        assert_eq!(waiting_since(&cleared), None);
        assert_eq!(waiting_since(&[]), None);
        // Un second retour n'efface pas la date du premier : c'est
        // depuis le premier que le coffre n'est plus vide.
        let piled = [
            mv(Kind::Retour, 9.0, "2026-01-05", 1),
            mv(Kind::Retour, 4.0, "2026-05-05", 2),
        ];
        assert_eq!(waiting_since(&piled).as_deref(), Some("2026-01-05"));

        // Et la liste se lit du plus ancien au plus récent.
        let f = |id: i64, label: &str, qty: f64, since: &str| Followed {
            id,
            label: label.to_owned(),
            unit: "gélule".to_owned(),
            stock: 40.0,
            threshold: 0.0,
            last_count: String::new(),
            to_destroy: qty,
            waiting_since: since.to_owned(),
        };
        let list = awaiting(
            &[
                f(1, "Actiskenan 10 mg", 4.0, "2026-08-30"),
                // Rien n'attend : la ligne n'a rien à faire là.
                f(2, "Oxynorm 5 mg", 0.0, ""),
                f(3, "Skenan LP 30 mg", 14.0, "2025-11-02"),
                // On a détruit plus qu'on n'avait reçu : une ligne
                // manque, et cela se montre au lieu de se taire.
                f(4, "Sevredol 20 mg", -2.0, "2026-09-01"),
            ],
            "2026-09-08",
        );
        assert_eq!(
            list.iter().map(|a| a.id).collect::<Vec<_>>(),
            vec![3, 1, 4],
            "du plus ancien au plus récent, et le produit sans rien à \
             détruire n'y est pas"
        );
        assert_eq!(list[0].days, Some(310));
        assert!((list[2].quantity + 2.0).abs() < 1e-9);
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
            to_destroy: 0.0,
            waiting_since: String::new(),
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
                // **Et il tient encore une fois le laboratoire ajouté.**
                // C'est le libellé réel d'un générique suivi —
                // « Méthylphénidate LP 36 mg (Biogaran) » —, et c'est
                // lui qui s'imprime. Une présentation dont le nom ne
                // laisse plus la place au laboratoire oblige à choisir
                // entre suivre le bon produit et lire sa ligne.
                let longest = LABS.iter().copied().max_by_key(|l| l.chars().count());
                let full = labelled(label, longest.unwrap_or_default());
                assert!(
                    full.chars().count() <= 56,
                    "« {full} » ne tient plus une fois le laboratoire écrit"
                );
            }
        }
    }

    /// Le laboratoire s'écrit **d'une seule façon**.
    ///
    /// C'est ce qui part sur chaque ligne du registre : deux façons de
    /// le composer donneraient deux produits pour une boîte le jour où
    /// l'une des deux gagne une espace, et le registre est inaltérable.
    #[test]
    fn a_generic_is_named_by_its_presentation_and_its_laboratory() {
        assert_eq!(
            labelled("Méthylphénidate LP 36 mg", "EG"),
            "Méthylphénidate LP 36 mg (EG)"
        );
        // Sans laboratoire, la présentation garde son nom : le princeps
        // n'a pas à porter une parenthèse vide.
        assert_eq!(labelled("Skenan LP 30 mg", ""), "Skenan LP 30 mg");
        assert_eq!(labelled("Skenan LP 30 mg", "   "), "Skenan LP 30 mg");
        // Et l'espace autour ne fait pas un second produit.
        assert_eq!(
            labelled("  Oxycodone LP 10 mg  ", "  Biogaran  "),
            "Oxycodone LP 10 mg (Biogaran)"
        );
        // La liste livrée ne porte ni doublon ni ligne vide : ce sont
        // des pastilles, et deux pastilles du même nom ne se
        // distinguent pas.
        let mut labs = LABS.to_vec();
        labs.sort_unstable();
        let before = labs.len();
        labs.dedup();
        assert_eq!(labs.len(), before);
        assert!(LABS.iter().all(|l| !l.trim().is_empty()));
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

    /// Une case du même produit, pour les cinq tests qui suivent.
    fn slot<'a>(id: i64, typed: &'a str, expected: f64, reason: &'a str) -> Slot<'a> {
        Slot {
            stup_id: id,
            typed,
            expected,
            reason,
        }
    }

    /// **Une case vide n'est pas un zéro.**
    ///
    /// C'est la règle qui rend la feuille utilisable : on aligne les
    /// quarante produits du coffre et on en compte six. Prise pour un
    /// zéro, chaque case laissée tranquille écrirait un inventaire à
    /// zéro — trente-quatre lignes qui vident le coffre sur le papier,
    /// dans une pièce où rien ne s'efface.
    #[test]
    fn an_empty_box_is_not_a_zero() {
        let p = plan(
            Kind::Inventaire,
            &[
                slot(1, "", 40.0, ""),
                slot(2, "  ", 12.0, ""),
                slot(3, "16", 16.0, ""),
            ],
        );
        assert_eq!(p.lines.len(), 1, "une seule case a été remplie");
        assert_eq!(p.lines[0].stup_id, 3);
        assert!(p.snags.is_empty(), "une case vide ne coince pas : {p:?}");
        assert!(p.ready());
    }

    /// Une case illisible n'est pas un zéro non plus : elle se dit.
    ///
    /// « O » pour zéro, un tiret laissé devant : lus comme zéro, ils
    /// écriraient un mouvement que personne n'a voulu. Ce qui commence
    /// par un chiffre se lit en revanche jusqu'à son unité — « 16
    /// gélules » vaut seize, comme dans le formulaire produit par
    /// produit.
    #[test]
    fn an_unreadable_box_is_named_rather_than_read_as_zero() {
        let p = plan(
            Kind::Sortie,
            &[slot(1, "O", 40.0, ""), slot(2, "5", 9.0, "")],
        );
        assert_eq!(
            plan(Kind::Sortie, &[slot(9, "16 gélules", 40.0, "")]).lines[0].quantity,
            16.0,
            "l'unité écrite à la suite ne rend pas la case illisible"
        );
        assert_eq!(p.snag(1), Some(Snag::Unreadable));
        assert_eq!(p.lines.len(), 1, "l'autre case reste lisible");
        assert!(
            !p.ready(),
            "et pourtant rien ne part : une feuille part entière ou pas du tout"
        );
    }

    /// Zéro est un comptage ; ce n'est pas une délivrance.
    #[test]
    fn zero_counts_a_safe_and_dispenses_nothing() {
        let counted = plan(Kind::Inventaire, &[slot(1, "0", 0.0, "")]);
        assert_eq!(counted.lines.len(), 1, "un coffre vide est une information");
        assert!(counted.ready());
        let given = plan(Kind::Sortie, &[slot(1, "0", 40.0, "")]);
        assert_eq!(given.snag(1), Some(Snag::NotPositive));
        assert!(given.lines.is_empty());
    }

    /// **Un écart se motive case par case.**
    ///
    /// Deux produits qui manquent ne manquent pas pour la même raison,
    /// et un motif écrit une fois pour la feuille n'expliquerait ni
    /// l'un ni l'autre.
    #[test]
    fn a_gap_is_explained_line_by_line() {
        let p = plan(
            Kind::Inventaire,
            &[
                // Tombe juste : aucun motif à donner.
                slot(1, "40", 40.0, ""),
                // Deux manquent, et personne ne dit pourquoi.
                slot(2, "10", 12.0, ""),
                // Deux manquent aussi, et celui-là le dit.
                slot(3, "10", 12.0, "casse au comptoir"),
            ],
        );
        assert_eq!(p.snag(1), None);
        assert_eq!(p.snag(2), Some(Snag::GapWithoutReason));
        assert_eq!(p.snag(3), None);
        assert_eq!(p.lines.len(), 2);
        assert!(!p.ready());
    }

    /// Chaque ligne emporte le solde d'avant : c'est lui que rendra son
    /// annulation, et il n'est pas le même d'un produit à l'autre.
    #[test]
    fn every_line_carries_its_own_expected_balance() {
        let p = plan(
            Kind::Inventaire,
            &[
                slot(1, "40", 41.0, "une gélule cassée"),
                slot(2, "5", 5.0, ""),
            ],
        );
        assert!(p.ready());
        assert_eq!(p.lines[0].expected, 41.0);
        assert_eq!(p.lines[1].expected, 5.0);
        assert_eq!(p.lines[0].quantity, 40.0);
    }

    /// Une destruction sans sa pièce se dit **en face de la case**.
    ///
    /// La base la refuse déjà — `add_stup_move` l'exige, et c'est là que
    /// la règle tient. Mais un refus qui arrive après « Inscrire » ne
    /// nomme pas la ligne fautive : sur une feuille de dix produits, il
    /// laisse chercher lequel.
    #[test]
    fn a_destruction_says_which_line_lacks_its_record() {
        let p = plan(
            Kind::DestructionPerimes,
            &[
                slot(1, "4", 4.0, ""),
                slot(2, "2", 2.0, "PV 2026-03 · Dr Martin témoin"),
            ],
        );
        assert_eq!(p.snag(1), Some(Snag::RecordRequired));
        assert_eq!(p.snag(2), None);
        assert!(!p.ready());
        // Et une nature qui n'en demande pas n'en réclame pas.
        assert!(plan(Kind::Entree, &[slot(1, "28", 0.0, "")]).ready());
    }

    /// Une feuille vide n'est pas une feuille prête.
    #[test]
    fn an_untouched_sheet_writes_nothing() {
        let p = plan(Kind::Inventaire, &[slot(1, "", 40.0, "")]);
        assert!(!p.ready(), "rien à écrire n'est pas prêt à écrire");
        assert!(p.lines.is_empty() && p.snags.is_empty());
    }
}
