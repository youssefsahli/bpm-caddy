//! Les classes thérapeutiques : ce qu'elles sont, et sous quelles
//! grandes familles elles se rangent.
//!
//! Le champ `class` d'une fiche est du texte libre, et il a dérivé. Sur
//! les 851 fiches livrées, on comptait **495 libellés distincts, dont
//! 331 pour une seule fiche** : ce n'est plus une
//! classification, c'est une étiquette. Et la dérive n'était pas
//! seulement cosmétique — trois exemples relevés en la mesurant :
//!
//! * `anti-TNF` et `anti-TNF alpha`, deux classes pour une, dix fiches
//!   séparées en trois et sept ;
//! * `bêtabloquant` et `bêta-bloquant`, un trait d'union ;
//! * `biphosphonate` et `bisphosphonate`, une lettre.
//!
//! Ce qui coûte : la pastille de la fiche annonce « anti-TNF alpha (7) »
//! et cache Remicade, et l'anneau « même classe » du voisinage ne le
//! trouve pas non plus. La question derrière est celle du comptoir un
//! jour de rupture — *qu'est-ce qu'il y a d'autre dans cette classe* —
//! et une réponse incomplète y est pire qu'une absence de réponse.
//!
//! **La correction ne réécrit pas les fiches.** Un référentiel les lit :
//! chaque classe canonique porte les libellés qu'on a réellement
//! rencontrés, et [`canonical`] les y ramène. Une équipe qui écrit
//! « anti-TNF » tombe sur la même classe que celle qui écrit
//! « anti-TNF alpha », et une classe que le référentiel ne connaît pas
//! reste lisible plutôt que d'être écrasée — c'est la même règle que
//! partout ici : on ne réécrit pas ce que l'officine a écrit.
//!
//! **Et une famille au-dessus.** 383 classes ne se parcourent pas à
//! plat ; seize familles d'une vingtaine de classes, si. C'est ce qui
//! permet à la vue « Classes… » d'exister : on descend de l'appareil à
//! la classe, puis de la classe aux fiches.
//!
//! Pur et testé, sans base et sans egui. L'index est construit une fois
//! et jamais dans la boucle d'affichage.

/// Une grande famille : l'appareil, ou le champ de la pratique.
pub struct Family {
    /// La clé, stable : elle est comparée, jamais affichée.
    pub key: &'static str,
    /// Ce que la vue montre.
    pub label: &'static str,
}

/// Les seize familles, dans l'ordre où elles se lisent — l'appareil
/// cardiovasculaire d'abord, parce que c'est ce que l'officine délivre
/// le plus, et le rayon conseil en dernier.
pub const FAMILIES: &[Family] = &[
    Family {
        key: "cardio",
        label: "Cardiologie et vaisseaux",
    },
    Family {
        key: "hemato",
        label: "Sang et hémostase",
    },
    Family {
        key: "neuro",
        label: "Neurologie",
    },
    Family {
        key: "psy",
        label: "Psychiatrie",
    },
    Family {
        key: "douleur",
        label: "Douleur et inflammation",
    },
    Family {
        key: "infectio",
        label: "Infectiologie",
    },
    Family {
        key: "respi",
        label: "Pneumologie et ORL",
    },
    Family {
        key: "digestif",
        label: "Appareil digestif",
    },
    Family {
        key: "endocrino",
        label: "Endocrinologie et métabolisme",
    },
    Family {
        key: "uro",
        label: "Urologie et néphrologie",
    },
    Family {
        key: "gyneco",
        label: "Gynécologie et obstétrique",
    },
    Family {
        key: "derm",
        label: "Dermatologie",
    },
    Family {
        key: "ophtalmo",
        label: "Ophtalmologie",
    },
    Family {
        key: "immuno",
        label: "Immunologie et cancérologie",
    },
    Family {
        key: "os",
        label: "Os et rhumatologie",
    },
    Family {
        key: "divers",
        label: "Nutrition, vitamines et conseil",
    },
];

/// Une classe thérapeutique, et les libellés qui la désignent.
pub struct Class {
    /// Le nom canonique : celui que la vue affiche et sous lequel les
    /// fiches se rassemblent.
    pub name: &'static str,
    /// La clé de sa [`Family`].
    pub family: &'static str,
    /// Les autres libellés rencontrés dans les fiches, et qui désignent
    /// cette classe. Un alias n'est **jamais** le nom canonique d'une
    /// autre classe — un test le tient, sans quoi une fiche tomberait
    /// dans deux classes selon l'ordre de la table.
    pub aliases: &'static [&'static str],
}

/// Les 383 classes, par famille puis par nom.
pub const CLASSES: &[Class] = &[
    // --- Cardiologie et vaisseaux ---
    Class {
        name: "activateur des canaux potassiques",
        family: "cardio",
        aliases: &[],
    },
    Class {
        name: "alpha-bloquant",
        family: "cardio",
        aliases: &[],
    },
    Class {
        name: "alpha-bloquant antihypertenseur",
        family: "cardio",
        aliases: &[],
    },
    Class {
        name: "antagoniste de l'endothéline (HTAP)",
        family: "cardio",
        aliases: &[],
    },
    Class {
        name: "antagoniste non stéroïdien des récepteurs minéralocorticoïdes",
        family: "cardio",
        aliases: &[],
    },
    Class {
        name: "anti-PCSK9",
        family: "cardio",
        aliases: &[],
    },
    Class {
        name: "antialdostérone",
        family: "cardio",
        aliases: &[],
    },
    Class {
        name: "antiangineux métabolique",
        family: "cardio",
        aliases: &[],
    },
    Class {
        name: "antiarythmique",
        family: "cardio",
        aliases: &[
            "antiarythmique classe Ia",
            "antiarythmique classe Ic",
            "bêta-bloquant antiarythmique",
        ],
    },
    Class {
        name: "antiarythmique injectable",
        family: "cardio",
        aliases: &[
            "antiarythmique (réduction de Bouveret)",
            "antiarythmique (tachycardie jonctionnelle)",
        ],
    },
    Class {
        name: "antihypertenseur central",
        family: "cardio",
        aliases: &[],
    },
    Class {
        name: "antihypertenseur central (grossesse)",
        family: "cardio",
        aliases: &[],
    },
    Class {
        name: "ARA II",
        family: "cardio",
        aliases: &[],
    },
    Class {
        name: "ARA II + diurétique",
        family: "cardio",
        aliases: &["ARA II + inhibiteur calcique"],
    },
    Class {
        name: "bêtabloquant",
        family: "cardio",
        aliases: &["bêta-bloquant"],
    },
    Class {
        name: "bêtabloquant alpha et bêta",
        family: "cardio",
        aliases: &["bêtabloquant (alpha et bêta)"],
    },
    Class {
        name: "chélateur des acides biliaires",
        family: "cardio",
        aliases: &[],
    },
    Class {
        name: "digitalique",
        family: "cardio",
        aliases: &[],
    },
    Class {
        name: "diurétique de l'anse",
        family: "cardio",
        aliases: &[],
    },
    Class {
        name: "diurétique thiazidique",
        family: "cardio",
        aliases: &["diurétique apparenté thiazidique"],
    },
    Class {
        name: "diurétique épargneur de potassium",
        family: "cardio",
        aliases: &["diurétique épargneur K+"],
    },
    Class {
        name: "diurétique épargneur K+ + thiazidique",
        family: "cardio",
        aliases: &[],
    },
    Class {
        name: "dérivé nitré",
        family: "cardio",
        aliases: &["dérivé nitré (spray sublingual)"],
    },
    Class {
        name: "fibrate",
        family: "cardio",
        aliases: &[],
    },
    Class {
        name: "hypolipémiant",
        family: "cardio",
        aliases: &[],
    },
    Class {
        name: "IEC",
        family: "cardio",
        aliases: &[],
    },
    Class {
        name: "IEC + diurétique",
        family: "cardio",
        aliases: &[
            "IEC + diurétique + inhibiteur calcique",
            "IEC + inhibiteur calcique",
        ],
    },
    Class {
        name: "IEC/ARA2 — insuffisance cardiaque",
        family: "cardio",
        aliases: &[],
    },
    Class {
        name: "inhibiteur calcique",
        family: "cardio",
        aliases: &["inhibiteur calcique (générique)"],
    },
    Class {
        name: "inhibiteur calcique (vasospasme cérébral)",
        family: "cardio",
        aliases: &[],
    },
    Class {
        name: "inhibiteur calcique bradycardisant",
        family: "cardio",
        aliases: &[],
    },
    Class {
        name: "inhibiteur du courant If",
        family: "cardio",
        aliases: &[],
    },
    Class {
        name: "petit ARN interférent anti-PCSK9",
        family: "cardio",
        aliases: &[],
    },
    Class {
        name: "statine",
        family: "cardio",
        aliases: &["statine (générique)"],
    },
    Class {
        name: "statine + inhibiteur de l'absorption",
        family: "cardio",
        aliases: &[],
    },
    Class {
        name: "stimulateur de la guanylate cyclase soluble",
        family: "cardio",
        aliases: &[],
    },
    Class {
        name: "sympathomimétique (hypotension orthostatique)",
        family: "cardio",
        aliases: &[],
    },
    Class {
        name: "vasodilatateur (artériopathie)",
        family: "cardio",
        aliases: &[],
    },
    Class {
        name: "vasodilatateur donneur de NO",
        family: "cardio",
        aliases: &[],
    },
    Class {
        name: "veinotonique",
        family: "cardio",
        aliases: &[],
    },
    // --- Sang et hémostase ---
    Class {
        name: "agent stimulant l'érythropoïèse",
        family: "hemato",
        aliases: &[],
    },
    Class {
        name: "anti-IIa direct injectable",
        family: "hemato",
        aliases: &["anti-IIa direct IV (TIH)"],
    },
    Class {
        name: "anti-Xa injectable",
        family: "hemato",
        aliases: &[],
    },
    Class {
        name: "antiagrégant",
        family: "hemato",
        aliases: &[],
    },
    Class {
        name: "antiagrégant (association)",
        family: "hemato",
        aliases: &["antiagrégant (thiénopyridine)"],
    },
    Class {
        name: "anticoagulant (alternative en TIH)",
        family: "hemato",
        aliases: &[],
    },
    Class {
        name: "antidote des AVK / vitamine K",
        family: "hemato",
        aliases: &[],
    },
    Class {
        name: "antifibrinolytique",
        family: "hemato",
        aliases: &[],
    },
    Class {
        name: "AOD",
        family: "hemato",
        aliases: &[],
    },
    Class {
        name: "AVK",
        family: "hemato",
        aliases: &[],
    },
    Class {
        name: "facteur de croissance granulocytaire",
        family: "hemato",
        aliases: &[],
    },
    Class {
        name: "fer",
        family: "hemato",
        aliases: &[],
    },
    Class {
        name: "fer + vitamine C",
        family: "hemato",
        aliases: &[],
    },
    Class {
        name: "fer injectable",
        family: "hemato",
        aliases: &[],
    },
    Class {
        name: "HBPM",
        family: "hemato",
        aliases: &[],
    },
    Class {
        name: "héparine",
        family: "hemato",
        aliases: &[],
    },
    Class {
        name: "héparine non fractionnée",
        family: "hemato",
        aliases: &["héparine (voie sous-cutanée)"],
    },
    Class {
        name: "prévention de l'allo-immunisation Rhésus",
        family: "hemato",
        aliases: &[],
    },
    // --- Neurologie ---
    Class {
        name: "agoniste dopaminergique",
        family: "neuro",
        aliases: &[],
    },
    Class {
        name: "agoniste dopaminergique injectable",
        family: "neuro",
        aliases: &["agoniste dopaminergique — patch"],
    },
    Class {
        name: "agoniste dopaminergique — hyperprolactinémie",
        family: "neuro",
        aliases: &[],
    },
    Class {
        name: "amélioration de la marche — SEP",
        family: "neuro",
        aliases: &[],
    },
    Class {
        name: "anesthésique local",
        family: "neuro",
        aliases: &["anesthésique local — crème/patch"],
    },
    Class {
        name: "anti-CD20 — SEP",
        family: "neuro",
        aliases: &[],
    },
    Class {
        name: "anti-CGRP — migraine",
        family: "neuro",
        aliases: &[],
    },
    Class {
        name: "anticholinergique antiparkinsonien",
        family: "neuro",
        aliases: &[],
    },
    Class {
        name: "anticholinergique — mal des transports",
        family: "neuro",
        aliases: &[],
    },
    Class {
        name: "anticholinestérasique",
        family: "neuro",
        aliases: &[],
    },
    Class {
        name: "anticholinestérasique — myasthénie",
        family: "neuro",
        aliases: &[],
    },
    Class {
        name: "anticorps monoclonal — SEP",
        family: "neuro",
        aliases: &[],
    },
    Class {
        name: "antiglutamate — Alzheimer",
        family: "neuro",
        aliases: &[],
    },
    Class {
        name: "antiglutamate — SLA",
        family: "neuro",
        aliases: &[],
    },
    Class {
        name: "antimigraineux de crise",
        family: "neuro",
        aliases: &[],
    },
    Class {
        name: "antimigraineux de fond",
        family: "neuro",
        aliases: &[],
    },
    Class {
        name: "antiparkinsonien",
        family: "neuro",
        aliases: &[],
    },
    Class {
        name: "antiparkinsonien — L-dopa",
        family: "neuro",
        aliases: &["antiparkinsonien — gel intestinal"],
    },
    Class {
        name: "antivertigineux",
        family: "neuro",
        aliases: &["anti-vertigineux"],
    },
    Class {
        name: "antiépileptique",
        family: "neuro",
        aliases: &["antiépileptique — absences"],
    },
    Class {
        name: "antiépileptique — syndrome de Dravet",
        family: "neuro",
        aliases: &[],
    },
    Class {
        name: "barbiturique antiépileptique",
        family: "neuro",
        aliases: &[],
    },
    Class {
        name: "benzodiazépine antiépileptique",
        family: "neuro",
        aliases: &["benzodiazépine — crise convulsive"],
    },
    Class {
        name: "emplâtre — douleur neuropathique",
        family: "neuro",
        aliases: &[],
    },
    Class {
        name: "gabapentinoïde",
        family: "neuro",
        aliases: &["antiépileptique / douleur neuropathique"],
    },
    Class {
        name: "gépant",
        family: "neuro",
        aliases: &[],
    },
    Class {
        name: "ICOMT",
        family: "neuro",
        aliases: &[],
    },
    Class {
        name: "IMAO-B",
        family: "neuro",
        aliases: &["IMAO-B (générique)"],
    },
    Class {
        name: "immunomodulateur — SEP",
        family: "neuro",
        aliases: &[],
    },
    Class {
        name: "immunosuppresseur — SEP",
        family: "neuro",
        aliases: &[],
    },
    Class {
        name: "modulateur S1P — SEP",
        family: "neuro",
        aliases: &[],
    },
    Class {
        name: "myorelaxant",
        family: "neuro",
        aliases: &[],
    },
    Class {
        name: "myorelaxant — spasticité",
        family: "neuro",
        aliases: &[],
    },
    Class {
        name: "narcolepsie — cataplexie",
        family: "neuro",
        aliases: &[],
    },
    Class {
        name: "patch — douleur neuropathique",
        family: "neuro",
        aliases: &[],
    },
    Class {
        name: "triptan",
        family: "neuro",
        aliases: &["triptan (générique)"],
    },
    Class {
        name: "éveillant — narcolepsie",
        family: "neuro",
        aliases: &[],
    },
    // --- Psychiatrie ---
    Class {
        name: "antidépresseur",
        family: "psy",
        aliases: &[],
    },
    Class {
        name: "antidépresseur multimodal",
        family: "psy",
        aliases: &[
            "antidépresseur mélatoninergique",
            "antidépresseur tétracyclique",
        ],
    },
    Class {
        name: "antidépresseur tricyclique",
        family: "psy",
        aliases: &[],
    },
    Class {
        name: "antipsychotique atypique",
        family: "psy",
        aliases: &["antipsychotique"],
    },
    Class {
        name: "antipsychotique retard",
        family: "psy",
        aliases: &["antipsychotique atypique retard"],
    },
    Class {
        name: "antipsychotique typique",
        family: "psy",
        aliases: &["antipsychotique sédatif"],
    },
    Class {
        name: "antipsychotique — NFS obligatoire",
        family: "psy",
        aliases: &[],
    },
    Class {
        name: "anxiolytique non benzodiazépinique",
        family: "psy",
        aliases: &[],
    },
    Class {
        name: "benzodiazépine",
        family: "psy",
        aliases: &[],
    },
    Class {
        name: "correcteur anticholinergique",
        family: "psy",
        aliases: &[],
    },
    Class {
        name: "hypnotique",
        family: "psy",
        aliases: &[],
    },
    Class {
        name: "hypnotique antihistaminique",
        family: "psy",
        aliases: &[],
    },
    Class {
        name: "hypnotique benzodiazépinique",
        family: "psy",
        aliases: &[],
    },
    Class {
        name: "IMAO-A",
        family: "psy",
        aliases: &[],
    },
    Class {
        name: "IRSNa",
        family: "psy",
        aliases: &[],
    },
    Class {
        name: "ISRS",
        family: "psy",
        aliases: &[],
    },
    Class {
        name: "mélatonine",
        family: "psy",
        aliases: &[],
    },
    Class {
        name: "psychostimulant — TDAH",
        family: "psy",
        aliases: &[],
    },
    Class {
        name: "sevrage alcoolique",
        family: "psy",
        aliases: &["alcoolodépendance", "sevrage alcoolique et opiacés"],
    },
    Class {
        name: "sevrage alcoolique — effet antabuse",
        family: "psy",
        aliases: &["réduction de la consommation d'alcool"],
    },
    Class {
        name: "sevrage tabagique",
        family: "psy",
        aliases: &[],
    },
    Class {
        name: "sevrage tabagique (générique)",
        family: "psy",
        aliases: &[],
    },
    Class {
        name: "substitut nicotinique",
        family: "psy",
        aliases: &["substitut nicotinique — patch"],
    },
    Class {
        name: "TDAH — non psychostimulant",
        family: "psy",
        aliases: &[],
    },
    Class {
        name: "thymorégulateur",
        family: "psy",
        aliases: &[],
    },
    Class {
        name: "traitement de substitution aux opiacés",
        family: "psy",
        aliases: &[],
    },
    // --- Douleur et inflammation ---
    Class {
        name: "AINS",
        family: "douleur",
        aliases: &[],
    },
    Class {
        name: "AINS coxib",
        family: "douleur",
        aliases: &[],
    },
    Class {
        name: "AINS local",
        family: "douleur",
        aliases: &["AINS local — bouche et gorge"],
    },
    Class {
        name: "AINS — pastille pour la gorge",
        family: "douleur",
        aliases: &[],
    },
    Class {
        name: "antagoniste opioïde périphérique",
        family: "douleur",
        aliases: &[],
    },
    Class {
        name: "antalgique non opioïde",
        family: "douleur",
        aliases: &["antalgique"],
    },
    Class {
        name: "antalgique opioïde faible",
        family: "douleur",
        aliases: &["antalgique opiacé"],
    },
    Class {
        name: "antidote des opiacés",
        family: "douleur",
        aliases: &["antidote des opiacés — kit d'urgence"],
    },
    Class {
        name: "antispasmodique",
        family: "douleur",
        aliases: &[],
    },
    Class {
        name: "corticoïde",
        family: "douleur",
        aliases: &[],
    },
    Class {
        name: "corticoïde substitutif",
        family: "douleur",
        aliases: &["minéralocorticoïde"],
    },
    Class {
        name: "opioïde agoniste-antagoniste",
        family: "douleur",
        aliases: &["opioïde antalgique"],
    },
    Class {
        name: "opioïde faible",
        family: "douleur",
        aliases: &[],
    },
    Class {
        name: "opioïde fort",
        family: "douleur",
        aliases: &["opioïde"],
    },
    Class {
        name: "opioïde à libération immédiate",
        family: "douleur",
        aliases: &[
            "opioïde — solution buvable",
            "opioïde transmuqueux — accès douloureux",
        ],
    },
    // --- Infectiologie ---
    Class {
        name: "aminoside",
        family: "infectio",
        aliases: &[],
    },
    Class {
        name: "antibiotique non absorbé",
        family: "infectio",
        aliases: &["antiseptique intestinal"],
    },
    Class {
        name: "antibiotique urinaire",
        family: "infectio",
        aliases: &[],
    },
    Class {
        name: "anticorps monoclonal anti-VRS (bronchiolite)",
        family: "infectio",
        aliases: &[],
    },
    Class {
        name: "antifongique",
        family: "infectio",
        aliases: &[],
    },
    Class {
        name: "antifongique azolé",
        family: "infectio",
        aliases: &[],
    },
    Class {
        name: "antifongique polyénique",
        family: "infectio",
        aliases: &["antifongique local digestif"],
    },
    Class {
        name: "antifongique systémique (teignes)",
        family: "infectio",
        aliases: &[],
    },
    Class {
        name: "antipaludique",
        family: "infectio",
        aliases: &[],
    },
    Class {
        name: "antipaludéen de synthèse",
        family: "infectio",
        aliases: &[],
    },
    Class {
        name: "antiparasitaire",
        family: "infectio",
        aliases: &["antihelminthique"],
    },
    Class {
        name: "antiparasitaire (toxoplasmose)",
        family: "infectio",
        aliases: &[],
    },
    Class {
        name: "antirétroviral",
        family: "infectio",
        aliases: &["antirétroviral INTI", "antirétroviral — antiprotéase"],
    },
    Class {
        name: "antirétroviral / PrEP",
        family: "infectio",
        aliases: &["antirétroviraux INTI"],
    },
    Class {
        name: "antirétroviral — inhibiteur d'intégrase",
        family: "infectio",
        aliases: &["trithérapie antirétrovirale"],
    },
    Class {
        name: "antiseptique",
        family: "infectio",
        aliases: &[],
    },
    Class {
        name: "antituberculeux",
        family: "infectio",
        aliases: &[],
    },
    Class {
        name: "antituberculeux — inducteur enzymatique",
        family: "infectio",
        aliases: &["antituberculeux (association)"],
    },
    Class {
        name: "antiviral",
        family: "infectio",
        aliases: &["antiviral anti-CMV", "antiviral — hépatite B"],
    },
    Class {
        name: "antiviral (Covid-19)",
        family: "infectio",
        aliases: &["antiviral (hépatite B)"],
    },
    Class {
        name: "antiviral à action directe (hépatite C)",
        family: "infectio",
        aliases: &[],
    },
    Class {
        name: "antiviral — grippe",
        family: "infectio",
        aliases: &["antigrippal inhalé"],
    },
    Class {
        name: "bain de bouche antiseptique",
        family: "infectio",
        aliases: &[],
    },
    Class {
        name: "carbapénème",
        family: "infectio",
        aliases: &[],
    },
    Class {
        name: "cycline",
        family: "infectio",
        aliases: &["cycline (acné)"],
    },
    Class {
        name: "céphalosporine de 1re génération",
        family: "infectio",
        aliases: &["céphalosporine de 1re génération injectable"],
    },
    Class {
        name: "céphalosporine de 2e génération",
        family: "infectio",
        aliases: &[],
    },
    Class {
        name: "céphalosporine de 3e génération",
        family: "infectio",
        aliases: &[
            "céphalosporine C3G",
            "céphalosporine de 3e génération injectable",
        ],
    },
    Class {
        name: "fluoroquinolone",
        family: "infectio",
        aliases: &["fluoroquinolone urinaire"],
    },
    Class {
        name: "glycopeptide",
        family: "infectio",
        aliases: &[],
    },
    Class {
        name: "lincosamide",
        family: "infectio",
        aliases: &[],
    },
    Class {
        name: "macrolide",
        family: "infectio",
        aliases: &[],
    },
    Class {
        name: "nitro-imidazolé",
        family: "infectio",
        aliases: &[],
    },
    Class {
        name: "oxazolidinone",
        family: "infectio",
        aliases: &[],
    },
    Class {
        name: "pénicilline",
        family: "infectio",
        aliases: &[],
    },
    Class {
        name: "pénicilline + inhibiteur de bêtalactamase",
        family: "infectio",
        aliases: &["pénicilline + inhibiteur"],
    },
    Class {
        name: "pénicilline V",
        family: "infectio",
        aliases: &["pénicilline retard IM"],
    },
    Class {
        name: "streptogramine",
        family: "infectio",
        aliases: &[],
    },
    Class {
        name: "sulfamide antibactérien",
        family: "infectio",
        aliases: &[],
    },
    // --- Pneumologie et ORL ---
    Class {
        name: "AMLA + BDLA inhalés",
        family: "respi",
        aliases: &["BDLA + AMLA inhalés"],
    },
    Class {
        name: "anti-IgE",
        family: "respi",
        aliases: &["anti-IgE (asthme sévère)"],
    },
    Class {
        name: "anti-IL-5",
        family: "respi",
        aliases: &["anti-IL-5 (asthme sévère)", "anti-IL-5R (asthme sévère)"],
    },
    Class {
        name: "anti-inflammatoire enzymatique",
        family: "respi",
        aliases: &[],
    },
    Class {
        name: "anticholinergique inhalé",
        family: "respi",
        aliases: &["anticholinergique"],
    },
    Class {
        name: "anticholinergique inhalé de longue durée",
        family: "respi",
        aliases: &[],
    },
    Class {
        name: "antifibrosant (fibrose pulmonaire)",
        family: "respi",
        aliases: &[],
    },
    Class {
        name: "antihistaminique + corticoïde nasal",
        family: "respi",
        aliases: &[],
    },
    Class {
        name: "antihistaminique H1",
        family: "respi",
        aliases: &[],
    },
    Class {
        name: "antihistaminique H1 sédatif",
        family: "respi",
        aliases: &[],
    },
    Class {
        name: "antileucotriène",
        family: "respi",
        aliases: &[],
    },
    Class {
        name: "antiseptique de la gorge",
        family: "respi",
        aliases: &[],
    },
    Class {
        name: "antitussif",
        family: "respi",
        aliases: &["antitussif antihistaminique"],
    },
    Class {
        name: "antitussif opiacé",
        family: "respi",
        aliases: &[],
    },
    Class {
        name: "auto-injecteur — choc anaphylactique",
        family: "respi",
        aliases: &[],
    },
    Class {
        name: "bronchodilatateur xanthique",
        family: "respi",
        aliases: &[],
    },
    Class {
        name: "bêta-2 de longue durée",
        family: "respi",
        aliases: &["BDLA inhalé"],
    },
    Class {
        name: "bêta-2 mimétique",
        family: "respi",
        aliases: &[],
    },
    Class {
        name: "corticoïde inhalé",
        family: "respi",
        aliases: &[],
    },
    Class {
        name: "corticoïde inhalé + BDLA",
        family: "respi",
        aliases: &["corticoïde inhalé + BALA"],
    },
    Class {
        name: "corticoïde nasal",
        family: "respi",
        aliases: &[],
    },
    Class {
        name: "CSI + BDLA",
        family: "respi",
        aliases: &[],
    },
    Class {
        name: "décongestionnant local",
        family: "respi",
        aliases: &[],
    },
    Class {
        name: "gouttes auriculaires antalgiques",
        family: "respi",
        aliases: &[],
    },
    Class {
        name: "gouttes auriculaires antibiotiques",
        family: "respi",
        aliases: &[],
    },
    Class {
        name: "immunothérapie allergénique",
        family: "respi",
        aliases: &[],
    },
    Class {
        name: "modulateur CFTR (mucoviscidose)",
        family: "respi",
        aliases: &["mucolytique inhalé (mucoviscidose)"],
    },
    Class {
        name: "mucolytique",
        family: "respi",
        aliases: &[],
    },
    Class {
        name: "trithérapie inhalée",
        family: "respi",
        aliases: &["trithérapie inhalée CSI + BDLA + AMLA"],
    },
    Class {
        name: "vasoconstricteur nasal",
        family: "respi",
        aliases: &[],
    },
    Class {
        name: "vasoconstricteur oral",
        family: "respi",
        aliases: &[],
    },
    // --- Appareil digestif ---
    Class {
        name: "acide biliaire",
        family: "digestif",
        aliases: &[],
    },
    Class {
        name: "aminosalicylé — MICI",
        family: "digestif",
        aliases: &[],
    },
    Class {
        name: "antagoniste NK1",
        family: "digestif",
        aliases: &[],
    },
    Class {
        name: "anti-H2",
        family: "digestif",
        aliases: &[],
    },
    Class {
        name: "anti-intégrine — MICI",
        family: "digestif",
        aliases: &[],
    },
    Class {
        name: "antiacide",
        family: "digestif",
        aliases: &["pansement gastrique"],
    },
    Class {
        name: "antidiarrhéique",
        family: "digestif",
        aliases: &["antisécrétoire intestinal"],
    },
    Class {
        name: "antiflatulent",
        family: "digestif",
        aliases: &[],
    },
    Class {
        name: "antispasmodique musculotrope",
        family: "digestif",
        aliases: &["antispasmodique + antiflatulent"],
    },
    Class {
        name: "antiémétique",
        family: "digestif",
        aliases: &[],
    },
    Class {
        name: "antiémétique antihistaminique",
        family: "digestif",
        aliases: &[],
    },
    Class {
        name: "antiémétique de chimiothérapie",
        family: "digestif",
        aliases: &[],
    },
    Class {
        name: "corticoïde à action locale — MICI",
        family: "digestif",
        aliases: &["corticoïde à action locale"],
    },
    Class {
        name: "enzymes pancréatiques",
        family: "digestif",
        aliases: &[],
    },
    Class {
        name: "IPP",
        family: "digestif",
        aliases: &[],
    },
    Class {
        name: "laxatif de lest",
        family: "digestif",
        aliases: &[],
    },
    Class {
        name: "laxatif lubrifiant",
        family: "digestif",
        aliases: &["laxatif émollient"],
    },
    Class {
        name: "laxatif osmotique",
        family: "digestif",
        aliases: &[],
    },
    Class {
        name: "laxatif rectal",
        family: "digestif",
        aliases: &[],
    },
    Class {
        name: "laxatif stimulant",
        family: "digestif",
        aliases: &[],
    },
    Class {
        name: "probiotique — antidiarrhéique",
        family: "digestif",
        aliases: &["levure — antidiarrhéique"],
    },
    Class {
        name: "préparation colique",
        family: "digestif",
        aliases: &[],
    },
    Class {
        name: "sétron",
        family: "digestif",
        aliases: &["sétron antiémétique"],
    },
    // --- Endocrinologie et métabolisme ---
    Class {
        name: "analogue de la vasopressine",
        family: "endocrino",
        aliases: &[],
    },
    Class {
        name: "analogue GLP-1",
        family: "endocrino",
        aliases: &[
            "analogue GLP-1 (voie orale)",
            "analogue GLP-1 — obésité",
            "agoniste GIP/GLP-1",
        ],
    },
    Class {
        name: "anti-goutteux",
        family: "endocrino",
        aliases: &[],
    },
    Class {
        name: "antithyroïdien de synthèse",
        family: "endocrino",
        aliases: &[],
    },
    Class {
        name: "biguanide",
        family: "endocrino",
        aliases: &[],
    },
    Class {
        name: "calcimimétique (hyperparathyroïdie)",
        family: "endocrino",
        aliases: &[],
    },
    Class {
        name: "glinide",
        family: "endocrino",
        aliases: &[],
    },
    Class {
        name: "gliptine",
        family: "endocrino",
        aliases: &["gliptine (inhibiteur DPP-4)", "iDPP-4"],
    },
    Class {
        name: "gliptine + biguanide",
        family: "endocrino",
        aliases: &["inhibiteur SGLT2 + biguanide"],
    },
    Class {
        name: "hormone hyperglycémiante (hypoglycémie sévère)",
        family: "endocrino",
        aliases: &[],
    },
    Class {
        name: "hormone thyroïdienne",
        family: "endocrino",
        aliases: &["hormone thyroïdienne T3"],
    },
    Class {
        name: "hormone thyroïdienne T4 + T3",
        family: "endocrino",
        aliases: &[],
    },
    Class {
        name: "hypo-uricémiant",
        family: "endocrino",
        aliases: &[],
    },
    Class {
        name: "inhibiteur des alpha-glucosidases",
        family: "endocrino",
        aliases: &[],
    },
    Class {
        name: "insuline intermédiaire",
        family: "endocrino",
        aliases: &[],
    },
    Class {
        name: "insuline lente",
        family: "endocrino",
        aliases: &["insuline"],
    },
    Class {
        name: "insuline prémélangée",
        family: "endocrino",
        aliases: &[],
    },
    Class {
        name: "insuline rapide",
        family: "endocrino",
        aliases: &[],
    },
    Class {
        name: "insuline ultra-rapide",
        family: "endocrino",
        aliases: &[],
    },
    Class {
        name: "insuline ultralente",
        family: "endocrino",
        aliases: &[],
    },
    Class {
        name: "iSGLT2",
        family: "endocrino",
        aliases: &[],
    },
    Class {
        name: "sulfamide hypoglycémiant",
        family: "endocrino",
        aliases: &[],
    },
    // --- Urologie et néphrologie ---
    Class {
        name: "alcalinisant (acidose métabolique)",
        family: "uro",
        aliases: &[],
    },
    Class {
        name: "alpha-bloquant + 5-alpha-réductase",
        family: "uro",
        aliases: &[],
    },
    Class {
        name: "alpha-bloquant — HBP",
        family: "uro",
        aliases: &[],
    },
    Class {
        name: "antagoniste des récepteurs V2 (polykystose rénale)",
        family: "uro",
        aliases: &[],
    },
    Class {
        name: "anticholinergique vésical",
        family: "uro",
        aliases: &[],
    },
    Class {
        name: "bêta-3 agoniste vésical",
        family: "uro",
        aliases: &[],
    },
    Class {
        name: "chélateur du phosphore",
        family: "uro",
        aliases: &[],
    },
    Class {
        name: "chélateur du potassium",
        family: "uro",
        aliases: &[
            "chélateur du potassium (hyperkaliémie)",
            "résine échangeuse de cations (hyperkaliémie)",
        ],
    },
    Class {
        name: "inhibiteur 5-alpha-réductase",
        family: "uro",
        aliases: &[
            "inhibiteur de la 5-alpha-réductase",
            "inhibiteur de la 5-alpha-réductase — alopécie",
        ],
    },
    Class {
        name: "inhibiteur de l'anhydrase carbonique",
        family: "uro",
        aliases: &[],
    },
    Class {
        name: "inhibiteur PDE5",
        family: "uro",
        aliases: &[],
    },
    Class {
        name: "phytothérapie — HBP",
        family: "uro",
        aliases: &[],
    },
    // --- Gynécologie et obstétrique ---
    Class {
        name: "analogue de la GnRH",
        family: "gyneco",
        aliases: &[],
    },
    Class {
        name: "antagoniste de la GnRH",
        family: "gyneco",
        aliases: &[],
    },
    Class {
        name: "anti-infectieux vaginal",
        family: "gyneco",
        aliases: &["antifongique local vaginal", "probiotique vaginal"],
    },
    Class {
        name: "antiprogestatif — IVG",
        family: "gyneco",
        aliases: &[],
    },
    Class {
        name: "bouffées de chaleur — non hormonal",
        family: "gyneco",
        aliases: &[],
    },
    Class {
        name: "contraception d'urgence",
        family: "gyneco",
        aliases: &[],
    },
    Class {
        name: "contraception estroprogestative",
        family: "gyneco",
        aliases: &["contraception estroprogestative triphasique"],
    },
    Class {
        name: "contraception progestative seule",
        family: "gyneco",
        aliases: &["contraception microprogestative"],
    },
    Class {
        name: "contraception — anneau vaginal",
        family: "gyneco",
        aliases: &[],
    },
    Class {
        name: "contraception — DIU hormonal",
        family: "gyneco",
        aliases: &[],
    },
    Class {
        name: "contraception — implant",
        family: "gyneco",
        aliases: &[],
    },
    Class {
        name: "contraception — patch",
        family: "gyneco",
        aliases: &[],
    },
    Class {
        name: "estrogène en gel percutané",
        family: "gyneco",
        aliases: &[],
    },
    Class {
        name: "estrogène local vaginal",
        family: "gyneco",
        aliases: &[],
    },
    Class {
        name: "estrogénothérapie de la ménopause",
        family: "gyneco",
        aliases: &["THM estroprogestatif"],
    },
    Class {
        name: "gonadotrophine — FIV",
        family: "gyneco",
        aliases: &[],
    },
    Class {
        name: "inducteur de l'ovulation",
        family: "gyneco",
        aliases: &["déclenchement de l'ovulation"],
    },
    Class {
        name: "ocytocique",
        family: "gyneco",
        aliases: &[],
    },
    Class {
        name: "progestatif",
        family: "gyneco",
        aliases: &["progestatif naturel"],
    },
    Class {
        name: "prostaglandine — IVG",
        family: "gyneco",
        aliases: &[],
    },
    Class {
        name: "tocolytique",
        family: "gyneco",
        aliases: &[],
    },
    Class {
        name: "utérotonique — hémorragie de la délivrance",
        family: "gyneco",
        aliases: &["prostaglandine — hémorragie de la délivrance"],
    },
    // --- Dermatologie ---
    Class {
        name: "antibactérien topique — brûlures",
        family: "derm",
        aliases: &["émulsion — brûlures et radiodermites"],
    },
    Class {
        name: "antibiotique topique",
        family: "derm",
        aliases: &["antibiotique local"],
    },
    Class {
        name: "antibiotique topique — acné",
        family: "derm",
        aliases: &[],
    },
    Class {
        name: "antifongique topique",
        family: "derm",
        aliases: &["antifongique local", "antifongique local et buccal"],
    },
    Class {
        name: "antimitotique topique — condylomes",
        family: "derm",
        aliases: &[],
    },
    Class {
        name: "antinéoplasique topique",
        family: "derm",
        aliases: &[],
    },
    Class {
        name: "antipoux local",
        family: "derm",
        aliases: &[],
    },
    Class {
        name: "antipsoriasique oral",
        family: "derm",
        aliases: &[],
    },
    Class {
        name: "antipsoriasique topique",
        family: "derm",
        aliases: &[],
    },
    Class {
        name: "antipsoriasique topique — mousse",
        family: "derm",
        aliases: &[],
    },
    Class {
        name: "antiseptique asséchant",
        family: "derm",
        aliases: &[],
    },
    Class {
        name: "cicatrisant topique",
        family: "derm",
        aliases: &[],
    },
    Class {
        name: "dermocorticoïde faible",
        family: "derm",
        aliases: &["dermocorticoïde d'activité modérée"],
    },
    Class {
        name: "dermocorticoïde fort",
        family: "derm",
        aliases: &[],
    },
    Class {
        name: "dermocorticoïde fort + kératolytique",
        family: "derm",
        aliases: &[],
    },
    Class {
        name: "dermocorticoïde modéré",
        family: "derm",
        aliases: &[],
    },
    Class {
        name: "dermocorticoïde très fort",
        family: "derm",
        aliases: &[],
    },
    Class {
        name: "immunomodulateur topique",
        family: "derm",
        aliases: &[],
    },
    Class {
        name: "rétinoïde oral — psoriasis",
        family: "derm",
        aliases: &[],
    },
    Class {
        name: "rétinoïde topique — acné",
        family: "derm",
        aliases: &[],
    },
    Class {
        name: "rétinoïde — tératogène",
        family: "derm",
        aliases: &[],
    },
    Class {
        name: "scabicide",
        family: "derm",
        aliases: &["scabicide local"],
    },
    Class {
        name: "topique — acné",
        family: "derm",
        aliases: &[],
    },
    Class {
        name: "topique — acné / rosacée",
        family: "derm",
        aliases: &[],
    },
    Class {
        name: "topique — dermite séborrhéique",
        family: "derm",
        aliases: &[],
    },
    Class {
        name: "traitement de l'alopécie",
        family: "derm",
        aliases: &[],
    },
    Class {
        name: "vernis antifongique (onychomycose)",
        family: "derm",
        aliases: &[],
    },
    Class {
        name: "zinc oral — acné",
        family: "derm",
        aliases: &[],
    },
    Class {
        name: "émollient",
        family: "derm",
        aliases: &[],
    },
    // --- Ophtalmologie ---
    Class {
        name: "anti-VEGF intravitréen",
        family: "ophtalmo",
        aliases: &["anti-VEGF", "anti-VEGF intravitréen (DMLA)"],
    },
    Class {
        name: "collyre antiallergique",
        family: "ophtalmo",
        aliases: &[],
    },
    Class {
        name: "collyre antibiotique",
        family: "ophtalmo",
        aliases: &[],
    },
    Class {
        name: "collyre antiglaucomateux",
        family: "ophtalmo",
        aliases: &["collyre — association antiglaucomateuse"],
    },
    Class {
        name: "collyre antiseptique",
        family: "ophtalmo",
        aliases: &[],
    },
    Class {
        name: "collyre bêta-bloquant",
        family: "ophtalmo",
        aliases: &[],
    },
    Class {
        name: "collyre mydriatique",
        family: "ophtalmo",
        aliases: &[],
    },
    Class {
        name: "collyre — AINS",
        family: "ophtalmo",
        aliases: &[],
    },
    Class {
        name: "collyre — alpha-2 agoniste",
        family: "ophtalmo",
        aliases: &[],
    },
    Class {
        name: "collyre — antibiotique aminoside",
        family: "ophtalmo",
        aliases: &["collyre — antibiotique macrolide"],
    },
    Class {
        name: "collyre — antihistaminique",
        family: "ophtalmo",
        aliases: &[],
    },
    Class {
        name: "collyre — corticoïde",
        family: "ophtalmo",
        aliases: &[],
    },
    Class {
        name: "collyre — immunomodulateur (sécheresse oculaire sévère)",
        family: "ophtalmo",
        aliases: &[],
    },
    Class {
        name: "collyre — inhibiteur de l'anhydrase carbonique",
        family: "ophtalmo",
        aliases: &["collyre — inhibiteur anhydrase carbonique"],
    },
    Class {
        name: "collyre — prostaglandine",
        family: "ophtalmo",
        aliases: &[],
    },
    Class {
        name: "gel ophtalmique — antiviral",
        family: "ophtalmo",
        aliases: &[],
    },
    Class {
        name: "larmes artificielles",
        family: "ophtalmo",
        aliases: &[],
    },
    Class {
        name: "pommade ophtalmique corticoïde + antibiotique",
        family: "ophtalmo",
        aliases: &[],
    },
    // --- Immunologie et cancérologie ---
    Class {
        name: "alcaloïde de la pervenche",
        family: "immuno",
        aliases: &[],
    },
    Class {
        name: "alkylant",
        family: "immuno",
        aliases: &[],
    },
    Class {
        name: "anti-androgène",
        family: "immuno",
        aliases: &[],
    },
    Class {
        name: "anti-CD20",
        family: "immuno",
        aliases: &[],
    },
    Class {
        name: "anti-estrogène",
        family: "immuno",
        aliases: &[],
    },
    Class {
        name: "anti-HER2",
        family: "immuno",
        aliases: &[],
    },
    Class {
        name: "anti-IL-17",
        family: "immuno",
        aliases: &[],
    },
    Class {
        name: "anti-IL-23",
        family: "immuno",
        aliases: &["anti-IL-12/23"],
    },
    Class {
        name: "anti-IL-4/IL-13",
        family: "immuno",
        aliases: &[],
    },
    Class {
        name: "anti-IL-6",
        family: "immuno",
        aliases: &[],
    },
    Class {
        name: "anti-TNF alpha",
        family: "immuno",
        aliases: &["anti-TNF"],
    },
    Class {
        name: "anticancéreux oral — alkylant",
        family: "immuno",
        aliases: &[
            "anticancéreux oral — antimétabolite",
            "anticancéreux oral — fluoropyrimidine",
        ],
    },
    Class {
        name: "anticancéreux oral — hormonothérapie",
        family: "immuno",
        aliases: &[],
    },
    Class {
        name: "anticancéreux oral — immunomodulateur",
        family: "immuno",
        aliases: &[],
    },
    Class {
        name: "anticancéreux oral — inhibiteur CDK4/6",
        family: "immuno",
        aliases: &[],
    },
    Class {
        name: "anticancéreux oral — inhibiteur PARP",
        family: "immuno",
        aliases: &[],
    },
    Class {
        name: "anticancéreux oral — ITK BCR-ABL",
        family: "immuno",
        aliases: &[
            "anticancéreux oral — ITK multicible",
            "inhibiteur de tyrosine kinase EGFR",
        ],
    },
    Class {
        name: "antidote des cyanures",
        family: "immuno",
        aliases: &[],
    },
    Class {
        name: "antidote du méthotrexate",
        family: "immuno",
        aliases: &[],
    },
    Class {
        name: "antimétabolite",
        family: "immuno",
        aliases: &[],
    },
    Class {
        name: "hormonothérapie — anti-aromatase",
        family: "immuno",
        aliases: &[],
    },
    Class {
        name: "hormonothérapie — SERM",
        family: "immuno",
        aliases: &[],
    },
    Class {
        name: "immunomodulateur — DMARD",
        family: "immuno",
        aliases: &["DMARD"],
    },
    Class {
        name: "immunosuppresseur",
        family: "immuno",
        aliases: &[],
    },
    Class {
        name: "immunosuppresseur — inhibiteur mTOR",
        family: "immuno",
        aliases: &[],
    },
    Class {
        name: "immunothérapie anti-PD-1",
        family: "immuno",
        aliases: &[],
    },
    Class {
        name: "inhibiteur CDK4/6",
        family: "immuno",
        aliases: &[],
    },
    Class {
        name: "inhibiteur de tyrosine kinase",
        family: "immuno",
        aliases: &["anticancéreux oral — ITK", "anticancéreux oral — ITK EGFR"],
    },
    Class {
        name: "inhibiteur du protéasome",
        family: "immuno",
        aliases: &[],
    },
    Class {
        name: "inhibiteur JAK",
        family: "immuno",
        aliases: &[],
    },
    Class {
        name: "inhibiteur PDE4 — rhumatisme psoriasique",
        family: "immuno",
        aliases: &[],
    },
    Class {
        name: "modulateur de la costimulation",
        family: "immuno",
        aliases: &[],
    },
    Class {
        name: "phytothérapie — inducteur enzymatique",
        family: "immuno",
        aliases: &[],
    },
    Class {
        name: "vaccin",
        family: "immuno",
        aliases: &[
            "vaccin (rappel adulte)",
            "vaccin (65 ans et plus)",
            "vaccin (nourrisson)",
        ],
    },
    Class {
        name: "vaccin vivant atténué",
        family: "immuno",
        aliases: &[],
    },
    // --- Os et rhumatologie ---
    Class {
        name: "analogue de la PTH — ostéoporose",
        family: "os",
        aliases: &[],
    },
    Class {
        name: "anti-RANKL",
        family: "os",
        aliases: &["anti-RANKL (semestriel)"],
    },
    Class {
        name: "antiarthrosique d'action lente",
        family: "os",
        aliases: &[],
    },
    Class {
        name: "bisphosphonate",
        family: "os",
        aliases: &["biphosphonate"],
    },
    Class {
        name: "bisphosphonate injectable",
        family: "os",
        aliases: &[
            "biphosphonate injectable",
            "biphosphonate — perfusion annuelle",
        ],
    },
    Class {
        name: "calcium",
        family: "os",
        aliases: &[],
    },
    Class {
        name: "dérivé actif de la vitamine D",
        family: "os",
        aliases: &["dérivé hydroxylé de la vitamine D"],
    },
    Class {
        name: "SERM — ostéoporose",
        family: "os",
        aliases: &[],
    },
    Class {
        name: "supplémentation calcique",
        family: "os",
        aliases: &[],
    },
    Class {
        name: "vitamine D",
        family: "os",
        aliases: &[],
    },
    // --- Nutrition, vitamines et conseil ---
    Class {
        name: "magnésium",
        family: "divers",
        aliases: &[],
    },
    Class {
        name: "supplémentation potassique",
        family: "divers",
        aliases: &["supplément potassique"],
    },
    Class {
        name: "vitamine B1",
        family: "divers",
        aliases: &[],
    },
    Class {
        name: "vitamine B12",
        family: "divers",
        aliases: &[],
    },
    Class {
        name: "vitamine B9",
        family: "divers",
        aliases: &[],
    },
    Class {
        name: "vitamine C",
        family: "divers",
        aliases: &[],
    },
];

/// De quel côté ranger un libellé : la classe canonique qu'il désigne,
/// ou `None` s'il n'est d'aucune que le référentiel connaisse.
///
/// Insensible à la casse et aux accents, comme toute correspondance de
/// nom ici : une fiche qui porte « BÊTABLOQUANT » est un bêtabloquant.
pub fn canonical(label: &str) -> Option<&'static Class> {
    let key = crate::fuzzy::sort_key(label.trim());
    if key.is_empty() {
        return None;
    }
    index().get(key.as_str()).map(|i| &CLASSES[*i])
}

/// Deux fiches sont-elles de la même classe ?
///
/// Sur la classe **canonique** et non sur la chaîne : c'est toute la
/// raison d'être du référentiel, et l'endroit où il se paie. Sans lui,
/// « anti-TNF » et « anti-TNF alpha » sont deux classes, la pastille de
/// Humira annonce sept voisins au lieu de dix, et Remicade n'est nulle
/// part — sans que rien n'ait l'air cassé.
///
/// Deux libellés que le référentiel ne connaît ni l'un ni l'autre se
/// comparent tels quels : une classe écrite par l'officine groupe ses
/// fiches sans avoir à être déclarée ici. Un connu et un inconnu ne sont
/// jamais la même classe — c'est le seul choix qui n'invente rien.
pub fn same(a: &str, b: &str) -> bool {
    match (canonical(a), canonical(b)) {
        (Some(x), Some(y)) => std::ptr::eq(x, y),
        (None, None) => crate::fuzzy::eq_folded(a, b),
        _ => false,
    }
}

/// Le nom sous lequel afficher une classe : le canonique quand il y en a
/// un, et sinon ce que la fiche écrit.
///
/// Jamais rien de vide en retour : une pastille sans texte est une
/// pastille sur laquelle on clique sans savoir sur quoi.
pub fn display_name(label: &str) -> &str {
    match canonical(label) {
        Some(c) => c.name,
        None => label.trim(),
    }
}

/// La famille de cette clé.
pub fn family(key: &str) -> Option<&'static Family> {
    FAMILIES.iter().find(|f| f.key == key)
}

/// Les classes d'une famille, dans l'ordre de la table.
pub fn classes_of(family_key: &str) -> impl Iterator<Item = &'static Class> {
    let key = family_key.to_owned();
    CLASSES.iter().filter(move |c| c.family == key)
}

/// Le libellé plié → la classe, construit une fois.
///
/// 224 classes et leurs alias font quelque 280 comparaisons par
/// résolution ; la vue en demande une par fiche, sur 851 fiches, à
/// chaque image. C'est le calcul que cette maison interdit dans un
/// chemin de dessin, et la table est statique : l'index se construit au
/// premier accès et répond ensuite en une recherche.
fn index() -> &'static std::collections::HashMap<String, usize> {
    static INDEX: std::sync::OnceLock<std::collections::HashMap<String, usize>> =
        std::sync::OnceLock::new();
    INDEX.get_or_init(|| {
        let mut map = std::collections::HashMap::with_capacity(CLASSES.len() * 2);
        for (i, c) in CLASSES.iter().enumerate() {
            map.insert(crate::fuzzy::sort_key(c.name), i);
            for a in c.aliases {
                map.insert(crate::fuzzy::sort_key(a), i);
            }
        }
        map
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Chaque classe écrite sur une fiche livrée est dans le référentiel.
    ///
    /// C'est l'invariant qui donne au reste sa valeur. Une classe qui
    /// n'y est pas est une fiche que la vue « Classes… » ne montre nulle
    /// part, et un anneau « même classe » qui rate ses voisins — et
    /// personne ne s'en aperçoit, parce que rien ne casse : la liste est
    /// juste un peu plus courte qu'elle ne devrait.
    ///
    /// Le test lit les fiches **de départ** et non la base : c'est le
    /// contenu livré qu'on tient. Une équipe reste libre d'écrire la
    /// classe qu'elle veut, et ce qu'elle écrit se lit tel quel.
    #[test]
    fn every_class_the_shipped_cards_carry_is_in_the_referential() {
        let mut orphans: Vec<&str> = Vec::new();
        for (name, _, class, _) in crate::db::starter_drugs() {
            if class.trim().is_empty() {
                continue;
            }
            if canonical(class).is_none() {
                orphans.push(name);
            }
        }
        assert!(
            orphans.is_empty(),
            "{} fiches portent une classe hors référentiel : {:?}",
            orphans.len(),
            &orphans[..orphans.len().min(10)]
        );
    }

    /// Un libellé ne désigne qu'une classe.
    ///
    /// Le piège est le nom canonique d'une classe glissé dans les alias
    /// d'une autre : la fiche tomberait alors dans l'une ou dans l'autre
    /// selon l'ordre de la table, ce qui est la pire sorte de bogue —
    /// juste, puis faux, sans que rien n'ait changé.
    #[test]
    fn a_label_names_one_class_and_only_one() {
        let mut seen: std::collections::HashMap<String, &str> = std::collections::HashMap::new();
        for c in CLASSES {
            for label in std::iter::once(&c.name).chain(c.aliases.iter()) {
                let key = crate::fuzzy::sort_key(label);
                if let Some(other) = seen.insert(key, c.name) {
                    assert_eq!(other, c.name, "« {label} » désigne deux classes");
                }
            }
        }
        // Et l'index en rend autant qu'il y a de libellés distincts.
        assert_eq!(index().len(), seen.len());
    }

    /// Les alias ramènent à leur classe, et la casse et les accents ne
    /// changent rien.
    ///
    /// Les trois cas relevés en mesurant la dérive, écrits ici pour que
    /// la correction ne se défasse pas : le trait d'union, la lettre, et
    /// la classe coupée en deux.
    #[test]
    fn the_drift_that_was_measured_folds_back_to_one_class() {
        // Un trait d'union.
        assert_eq!(canonical("bêta-bloquant").unwrap().name, "bêtabloquant");
        assert_eq!(canonical("bêtabloquant").unwrap().name, "bêtabloquant");
        // Une lettre.
        assert_eq!(canonical("biphosphonate").unwrap().name, "bisphosphonate");
        // Dix fiches séparées en trois et sept.
        assert_eq!(canonical("anti-TNF").unwrap().name, "anti-TNF alpha");
        assert_eq!(canonical("anti-TNF alpha").unwrap().name, "anti-TNF alpha");
        // Trois classes pour une, dont deux ne se distinguaient que par
        // une parenthèse.
        for l in [
            "anti-VEGF",
            "anti-VEGF intravitréen",
            "anti-VEGF intravitréen (DMLA)",
        ] {
            assert_eq!(canonical(l).unwrap().name, "anti-VEGF intravitréen", "{l}");
        }
        // La casse et les accents ne décident de rien.
        assert_eq!(canonical("  BÊTABLOQUANT  ").unwrap().name, "bêtabloquant");
        assert_eq!(canonical("BETABLOQUANT").unwrap().name, "bêtabloquant");
        // Ce que le référentiel ne connaît pas reste inconnu : il ne
        // devine pas, et la fiche garde ce que l'équipe a écrit.
        assert!(canonical("classe maison").is_none());
        assert!(canonical("").is_none());
        assert!(canonical("   ").is_none());
    }

    /// Deux fiches sont de la même classe quand leur classe *canonique*
    /// l'est — et c'est là que le référentiel se paie.
    ///
    /// C'est la fonction sur laquelle l'anneau « même classe » et la
    /// pastille de la fiche comparent. Comparées comme deux chaînes,
    /// Humira et Remicade n'étaient pas voisins.
    #[test]
    fn two_cards_share_a_class_when_their_canonical_class_is_the_same() {
        // Le cas qui a motivé tout ceci.
        assert!(same("anti-TNF", "anti-TNF alpha"));
        assert!(same("anti-TNF alpha", "anti-TNF"));
        assert!(!crate::fuzzy::eq_folded("anti-TNF", "anti-TNF alpha"));
        // Deux classes distinctes le restent : replier n'est pas
        // confondre, et un IEC n'est pas un ARA II.
        assert!(!same("IEC", "ARA II"));
        assert!(!same("bêtabloquant", "inhibiteur calcique"));
        // Ce que le référentiel ignore des deux côtés se compare tel
        // quel : une classe écrite par l'officine groupe ses fiches sans
        // avoir été déclarée ici.
        assert!(same("classe maison", "CLASSE MAISON"));
        assert!(same("classe maison", "  classe maison  "));
        assert!(!same("classe maison", "autre classe maison"));
        // Un connu et un inconnu ne sont jamais la même classe : c'est
        // le seul choix qui n'invente rien.
        assert!(!same("IEC", "classe maison"));
        assert!(!same("classe maison", "IEC"));
        // Le nom affiché est le canonique quand il y en a un, et ce que
        // la fiche écrit sinon — jamais rien de vide, une pastille sans
        // texte est une pastille sur laquelle on clique à l'aveugle.
        assert_eq!(display_name("anti-TNF"), "anti-TNF alpha");
        assert_eq!(display_name("  bêta-bloquant "), "bêtabloquant");
        assert_eq!(display_name("classe maison"), "classe maison");
        assert_eq!(display_name("  classe maison  "), "classe maison");
    }

    /// Chaque classe est dans une famille qui existe, et chaque famille
    /// porte des classes.
    ///
    /// Une famille vide serait une ligne cliquable qui n'ouvre rien, et
    /// une classe dans une famille inconnue serait une classe qu'aucune
    /// colonne ne montre.
    #[test]
    fn every_class_sits_in_a_family_and_every_family_holds_classes() {
        for c in CLASSES {
            assert!(
                family(c.family).is_some(),
                "« {} » est rangée sous « {} », qui n'existe pas",
                c.name,
                c.family
            );
            assert!(!c.name.trim().is_empty());
        }
        for f in FAMILIES {
            let n = classes_of(f.key).count();
            assert!(n > 0, "{} ne porte aucune classe", f.label);
            assert!(!f.label.trim().is_empty());
        }
        assert!(family("rien du tout").is_none());
        // Le cliquet : le référentiel ne perd ni classes ni familles.
        assert_eq!(FAMILIES.len(), 16);
        assert!(
            CLASSES.len() >= 383,
            "le référentiel a maigri : {}",
            CLASSES.len()
        );
    }

    /// Le référentiel range **moins** de classes que la base n'écrit de
    /// libellés, et c'est tout l'intérêt.
    ///
    /// S'il en portait autant, il n'aurait rien replié : ce serait la
    /// même liste avec une colonne de plus. Le compte des libellés
    /// repliés est un cliquet — il ne peut que monter.
    #[test]
    fn the_referential_folds_the_drift_rather_than_copying_it() {
        let labels: std::collections::HashSet<String> = crate::db::starter_drugs()
            .iter()
            .map(|(_, _, class, _)| crate::fuzzy::sort_key(class.trim()))
            .filter(|c| !c.is_empty())
            .collect();
        let resolved: std::collections::HashSet<&str> = labels
            .iter()
            .filter_map(|l| canonical(l))
            .map(|c| c.name)
            .collect();
        // 495 libellés pour 383 classes : 112 repliés. Le cliquet tient
        // ce nombre — il ne peut que monter, jamais redescendre.
        assert!(
            resolved.len() + 112 <= labels.len(),
            "{} libellés pour {} classes : {} repliés, il en fallait 112",
            labels.len(),
            resolved.len(),
            labels.len() - resolved.len()
        );
    }
}
