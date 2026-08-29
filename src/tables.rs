//! Conversion / equivalence and reference tables for the counter: dose
//! equivalences (IPP, statines, corticoïdes, opioïdes,
//! benzodiazépines), dosing references (HBPM, AOD, corticoïdes
//! inhalés, insulines, antalgiques), the decision aids the acts need
//! (fonction rénale, angine, cystite, contraception, vaccins, doses
//! pédiatriques, écrasement des formes orales), and the three that
//! answer a question asked without an ordonnance in hand
//! (interactions, urgence au comptoir, grossesse et allaitement).
//!
//! The values are the classic published reference equivalences taught
//! in French pharmacy practice. They are deliberately static reference
//! data (like the starter drug list) — each table carries its numbered
//! sources, shown on screen and on the printout.

pub struct ConvTable {
    /// Short name for the selector buttons.
    pub short: &'static str,
    /// Which drawer of the list this table files under. The tables were
    /// always grouped — this module's own first paragraph groups them —
    /// but the grouping lived in prose, and the selector was
    /// twenty-seven undifferentiated buttons. Five families, in the
    /// order [`FAMILIES`] gives them.
    pub family: &'static str,
    pub title: &'static str,
    /// When this table was last read against its sources, and against
    /// which edition. A reference table without a date is a reference
    /// table nobody dares use: this one says how old it is.
    pub reviewed: &'static str,
    /// Numbered under the table, on screen and in the PDF.
    pub sources: &'static [&'static str],
    pub columns: &'static [&'static str],
    pub rows: &'static [&'static [&'static str]],
}

/// The drawers of the table list, in reading order: what a dose
/// converts to, what a dose *is*, what changes it, what is decided
/// without an ordonnance in hand, and how the thing is actually taken.
pub const FAMILIES: [&str; 5] = [
    "Équivalences",
    "Posologies",
    "Adaptation",
    "Au comptoir",
    "Administration",
];

pub const TABLES: &[ConvTable] = &[
    ConvTable {
        short: "IPP",
        family: "Équivalences",
        title: "IPP — équivalences, formes et prise",
        reviewed: "Août 2026 — RCP à jour et fiche HAS sur le bon usage des IPP",
        sources: &[
            "RCP des spécialités, base de données publique des médicaments (ANSM)",
            "HAS — bon usage des inhibiteurs de la pompe à protons chez l'adulte",
        ],
        columns: &["DCI (spécialité)", "Pleine dose / j", "Demi-dose / j", "Formes et dosages usuels", "Moment de prise", "Remarque"],
        rows: &[
            &["Oméprazole (Mopral)", "20 mg", "10 mg", "Gélules gastro-résistantes 10 et 20 mg", "Le matin, 15 à 30 min avant le petit-déjeuner", "Inhibiteur du CYP2C19 : association au clopidogrel déconseillée"],
            &["Ésoméprazole (Inexium)", "40 mg", "20 mg", "Comprimés gastro-résistants 20 et 40 mg, sachets pédiatriques", "Le matin, avant le petit-déjeuner", "Énantiomère de l'oméprazole : même réserve avec le clopidogrel"],
            &["Lansoprazole (Lanzor, Ogast)", "30 mg", "15 mg", "Gélules 15 et 30 mg, comprimés orodispersibles (Ogastoro)", "Le matin, avant le petit-déjeuner", "Orodispersible utile en cas de troubles de la déglutition"],
            &["Pantoprazole (Inipomp, Eupantol)", "40 mg", "20 mg", "Comprimés gastro-résistants 20 et 40 mg", "Le matin, environ 1 h avant le repas", "Interaction la plus faible avec le clopidogrel : à préférer"],
            &["Rabéprazole (Pariet)", "20 mg", "10 mg", "Comprimés gastro-résistants 10 et 20 mg", "Le matin ; prise indifférente par rapport au repas", "Métabolisme peu dépendant du CYP2C19 : autre option avec le clopidogrel"],
            &["Éradication d'Helicobacter pylori", "Pleine dose x2/j", "—", "Selon la spécialité choisie", "Matin et soir, avant les repas", "Associé à l'antibiothérapie, durée selon le protocole"],
            &["Toutes les gélules à microgranules", "—", "—", "Gélule ouvrable", "Granules avalés entiers dans un aliment semi-liquide", "Ne jamais croquer ni écraser les granules"],
            &["IPP au long cours", "—", "Descendre à la demi-dose dès que possible", "—", "—", "Réévaluer l'indication : hypomagnésémie, carence en B12, fractures, infections digestives"],
            &["Arrêt d'un traitement prolongé", "—", "Décroissance puis prise à la demande", "—", "—", "L'arrêt brutal expose à un rebond d'hypersécrétion acide"],
        ],
    },
    ConvTable {
        short: "HBPM",
        family: "Posologies",
        title: "HBPM — posologies, rein, surveillance",
        reviewed: "Août 2026 — RCP à jour ; recommandations MTEV en vigueur",
        sources: &[
            "RCP des spécialités (ANSM)",
            "SFMV / SFAR — traitement anticoagulant de la MTEV",
        ],
        columns: &["DCI (spécialité)", "Curatif", "Prophylaxie", "Adaptation rénale", "Surveillance", "Antidote"],
        rows: &[
            &["Énoxaparine (Lovenox)", "100 UI/kg x2/j", "4 000 UI x1/j", "Curatif contre-indiqué si ClCr < 30 mL/min ; prudence de 30 à 60", "Plaquettes selon le contexte ; anti-Xa si poids extrême ou rein altéré", "Sulfate de protamine — neutralisation partielle, protocole hospitalier"],
            &["Tinzaparine (Innohep)", "175 UI/kg x1/j", "3 500 à 4 500 UI x1/j", "Utilisable jusqu'à ClCr 20 mL/min selon le RCP, avec surveillance", "Anti-Xa 4 h après l'injection si contrôle nécessaire", "Sulfate de protamine — neutralisation partielle"],
            &["Daltéparine (Fragmine)", "100 UI/kg x2/j (ou 200 UI/kg x1/j)", "2 500 à 5 000 UI x1/j", "Curatif contre-indiqué si ClCr < 30 mL/min", "Plaquettes en contexte chirurgical ; anti-Xa au cas par cas", "Sulfate de protamine — neutralisation partielle"],
            &["Nadroparine (Fraxiparine)", "85 UI/kg x2/j", "2 850 UI x1/j", "Contre-indiqué si ClCr < 30 mL/min ; réduction de dose si atteinte modérée", "Idem, selon le RCP", "Sulfate de protamine — neutralisation partielle"],
            &["Fondaparinux (Arixtra) — apparenté", "7,5 mg x1/j (50 à 100 kg)", "2,5 mg x1/j", "Curatif contre-indiqué si ClCr < 30 ; prophylaxie 1,5 mg si ClCr 20 à 50", "Pas de risque de TIH : pas de surveillance plaquettaire", "Aucun antidote : la protamine est inefficace"],
            &["Plaquettes — modalités", "—", "—", "—", "Numération avant traitement, puis 2 fois par semaine en contexte chirurgical ou traumatique", "Chute des plaquettes ou thrombose : arrêt immédiat, avis spécialisé"],
            &["Anti-Xa — modalités", "—", "—", "Utile si ClCr abaissée, poids extrême, grossesse, hémorragie", "Prélèvement environ 4 h après l'injection sous-cutanée", "—"],
            &["Repères pratiques", "Poids réel, injection sous-cutanée dans le pli abdominal", "Ne pas purger la seringue préremplie", "Peser le patient et estimer la ClCr avant l'instauration", "Alterner les sites d'injection, ne pas masser après", "Saignement : compression, arrêt, appel du prescripteur"],
        ],
    },
    ConvTable {
        short: "Statines",
        family: "Équivalences",
        title: "Statines — intensité, efficacité, interactions",
        reviewed: "Août 2026 — ESC/EAS 2019, RCP à jour",
        sources: &[
            "ESC/EAS 2019 — prise en charge des dyslipidémies",
            "HAS — bon usage des statines",
            "RCP des spécialités (ANSM)",
        ],
        columns: &["DCI (spécialité)", "Dose ≈ équivalente", "Baisse du LDL", "Intensité", "Interactions et précautions"],
        rows: &[
            &["Rosuvastatine (Crestor)", "5 mg", "environ 40 %", "Modérée à haute", "Peu de CYP3A4 ; prudence avec la ciclosporine et les fibrates"],
            &["Atorvastatine (Tahor)", "10 mg", "environ 35 à 40 %", "Modérée", "Substrat du CYP3A4 : macrolides, azolés, vérapamil, pamplemousse"],
            &["Simvastatine (Zocor)", "20 mg", "environ 35 à 40 %", "Modérée", "La plus sensible au CYP3A4 ; dose plafonnée avec amlodipine, vérapamil, diltiazem"],
            &["Pravastatine (Elisor, Vasten)", "40 mg", "environ 30 à 35 %", "Modérée", "Pas de métabolisme par le CYP3A4 : utile avec un macrolide ou un antirétroviral"],
            &["Fluvastatine (Lescol, Fractal)", "80 mg LP", "environ 30 à 35 %", "Faible à modérée", "Métabolisme CYP2C9 : surveiller l'INR sous AVK"],
            &["Atorvastatine — palier haute intensité", "40 à 80 mg", "50 % ou plus", "Haute", "Palier de la prévention secondaire à haut risque"],
            &["Rosuvastatine — palier haute intensité", "20 à 40 mg", "50 % ou plus", "Haute", "40 mg réservé au très haut risque, jamais en instauration"],
            &["Simvastatine forte dose", "80 mg", "—", "Haute", "À éviter : risque musculaire nettement accru, préférer une autre statine"],
            &["Ézétimibe (Ezetrol) en association", "10 mg", "environ 20 % en plus de la statine", "Complément, non statine", "Ajouté quand la cible de LDL n'est pas atteinte"],
            &["Fibrates associés", "Selon l'indication", "—", "—", "Gemfibrozil contre-indiqué avec les statines ; fénofibrate possible sous surveillance"],
            &["Jus de pamplemousse", "—", "—", "—", "À éviter avec simvastatine et atorvastatine ; sans effet sur pravastatine et rosuvastatine"],
            &["Douleurs musculaires", "—", "—", "—", "Rechercher une interaction, doser les CPK, avis médical avant d'arrêter"],
        ],
    },
    ConvTable {
        short: "Corticoïdes",
        family: "Équivalences",
        title: "Corticoïdes — équivalences, durée, formes",
        reviewed: "Août 2026 — RCP à jour",
        sources: &[
            "Équivalences anti-inflammatoires classiques (pharmacologie clinique)",
            "RCP des spécialités (ANSM)",
        ],
        columns: &["DCI (spécialité)", "Dose équivalente", "vs prednisone", "Durée d'action", "Effet minéralocorticoïde", "Formes usuelles et remarque"],
        rows: &[
            &["Prednisone (Cortancyl)", "5 mg", "référence", "Intermédiaire (12 à 36 h)", "Faible", "Comprimés 1, 5 et 20 mg"],
            &["Prednisolone (Solupred)", "5 mg", "x1", "Intermédiaire (12 à 36 h)", "Faible", "Comprimés orodispersibles 5 et 20 mg, solution buvable"],
            &["Méthylprednisolone (Médrol)", "4 mg", "x1,25", "Intermédiaire (12 à 36 h)", "Très faible", "Comprimés 4 et 16 mg"],
            &["Triamcinolone (Kenacort Retard)", "4 mg", "x1,25", "Intermédiaire, forme retard prolongée", "Négligeable", "Suspension injectable retard, pas de forme orale en France"],
            &["Hydrocortisone", "20 mg", "x0,25", "Courte (8 à 12 h)", "Marqué — le plus élevé", "Comprimés sécables 10 mg, traitement substitutif surrénalien"],
            &["Cortisone", "25 mg", "x0,2", "Courte (8 à 12 h)", "Marqué", "Référence historique, peu utilisée par voie orale en pratique courante"],
            &["Dexaméthasone (Dectancyl)", "0,75 mg", "x6,7", "Longue (36 à 72 h)", "Nul", "Comprimés 0,5 mg ; freine fortement l'axe corticotrope"],
            &["Bétaméthasone (Célestène)", "0,75 mg", "x6,7", "Longue (36 à 72 h)", "Nul", "Comprimés dispersibles et solution buvable en gouttes"],
            &["Cure courte", "Selon l'indication", "—", "—", "—", "Prise unique le matin au cours du repas ; arrêt sans décroissance si cure brève"],
            &["Cure prolongée", "Selon l'indication", "—", "—", "Surveiller la tension et les œdèmes si effet minéralocorticoïde", "Décroissance progressive, risque d'insuffisance surrénale ; calcium, vitamine D, régime pauvre en sel et en sucres rapides"],
        ],
    },
    ConvTable {
        short: "Opioïdes",
        family: "Équivalences",
        title: "Opioïdes — équianalgésie et repères pratiques",
        reviewed: "Août 2026 — recommandations douleur en vigueur, RCP à jour",
        sources: &[
            "SFETD — rotation des opioïdes et équianalgésie",
            "RCP des spécialités (ANSM)",
        ],
        columns: &["Opioïde", "Conversion (réf. morphine orale)", "Exemple ≈ 60 mg morphine orale / j", "Délai et durée d'action", "Formes disponibles", "Insuffisance rénale"],
        rows: &[
            &["Morphine orale LI (Actiskénan, Oramorph)", "référence", "60 mg/j répartis en 6 prises", "Début 30 à 45 min, durée 4 h", "Gélules, comprimés, solution buvable", "Métabolites actifs accumulés : réduire la dose et espacer les prises"],
            &["Morphine orale LP (Skenan LP, Moscontin)", "référence", "30 mg x2/j", "Début 2 à 4 h, durée 12 h", "Gélules LP ouvrables (granules entiers), comprimés LP", "Même précaution que la forme à libération immédiate"],
            &["Codéine (orale)", "÷ 6", "360 mg/j", "Début 30 à 60 min, durée 4 à 6 h", "Associations au paracétamol, sirops adulte", "À éviter si insuffisance rénale sévère ; métabolisme CYP2D6 variable"],
            &["Tramadol (oral)", "÷ 5", "300 mg/j", "LI : 30 à 60 min, 4 à 6 h ; LP : 12 h", "Gélules LI, comprimés LP, solution buvable en gouttes", "Espacer les prises et plafonner la dose ; risque convulsif et sérotoninergique"],
            &["Oxycodone (OxyNorm, OxyContin)", "x 2", "30 mg/j", "LI : 20 à 30 min, 4 à 6 h ; LP : 12 h", "Gélules LI, comprimés LP, solution buvable, injectable", "Débuter bas et adapter, métabolites éliminés par le rein"],
            &["Hydromorphone (Sophidone LP)", "x 7,5", "8 mg/j", "Forme LP : durée 12 h", "Gélules LP uniquement (ouvrables, granules entiers)", "Adaptation nécessaire, surveillance de la sédation"],
            &["Morphine SC", "x 2", "30 mg/j", "Début 15 à 30 min, durée 4 h", "Ampoules injectables", "Même accumulation que la voie orale"],
            &["Morphine IV", "x 3", "20 mg/j", "Début 5 à 10 min, durée 4 h", "Ampoules, pompe PCA", "Titration en milieu hospitalier"],
            &["Fentanyl transdermique (Durogésic)", "25 µg/h ≈ 60 mg/j", "Patch 25 µg/h", "Effet en 12 à 24 h, relais du patch toutes les 72 h", "Patchs 12, 25, 50, 75 et 100 µg/h", "Peu de métabolites actifs : option intéressante, jamais en instauration"],
            &["Fentanyl transmuqueux (accès paroxystiques)", "Pas de conversion linéaire", "Titration indépendante du traitement de fond", "Début 10 à 15 min, durée 1 à 2 h", "Comprimés et films sublinguaux, pulvérisation nasale", "Réservé au cancer, chez un patient déjà sous opioïde fort"],
            &["Interdoses", "1/10 à 1/6 de la dose quotidienne", "6 à 10 mg par interdose", "Forme à libération immédiate, renouvelable selon le protocole", "Gélules LI, solution buvable", "Plus de 4 interdoses par jour : réévaluer le traitement de fond"],
            &["Surdosage", "—", "—", "Naloxone : effet en quelques minutes, plus bref que celui de l'opioïde", "Naloxone injectable et pulvérisation nasale", "Somnolence et fréquence respiratoire basse : arrêt, naloxone, appel du 15"],
        ],
    },
    ConvTable {
        short: "Benzodiazépines",
        family: "Équivalences",
        title: "Benzodiazépines — équivalences, demi-vie, indication",
        reviewed: "Août 2026 — fiches HAS sur l'arrêt des benzodiazépines, RCP à jour",
        sources: &[
            "Ashton C. H. — Benzodiazepines: how they work and how to withdraw, 2002",
            "HAS — arrêt des benzodiazépines et médicaments apparentés",
            "RCP des spécialités (ANSM)",
        ],
        columns: &["DCI (spécialité)", "Dose ≈ diazépam 10 mg", "Demi-vie", "Indication", "Sujet âgé et précautions"],
        rows: &[
            &["Diazépam (Valium)", "10 mg — référence", "Longue : 30 à 60 h, métabolite actif encore plus long", "Anxiolytique, myorelaxant, sevrage alcoolique", "À éviter : accumulation, somnolence, chutes"],
            &["Clorazépate (Tranxène)", "15 mg", "Longue, via le nordazépam", "Anxiolytique", "À éviter, même accumulation que le diazépam"],
            &["Prazépam (Lysanxia)", "10 à 20 mg", "Longue, via le nordazépam", "Anxiolytique", "À éviter chez le sujet âgé"],
            &["Bromazépam (Lexomil)", "6 mg", "Intermédiaire : environ 20 h", "Anxiolytique", "Demi-dose ; barrette sécable en quarts, commode pour la décroissance"],
            &["Alprazolam (Xanax)", "0,5 mg", "Intermédiaire : 10 à 20 h", "Anxiolytique", "Rebond anxieux entre les prises, dépendance rapide"],
            &["Lorazépam (Temesta)", "1 mg", "Intermédiaire : 10 à 20 h, sans métabolite actif", "Anxiolytique", "Acceptable à dose réduite si le rein ou le foie sont altérés"],
            &["Oxazépam (Séresta)", "30 mg", "Courte : 4 à 15 h, sans métabolite actif", "Anxiolytique, sevrage alcoolique", "Molécule de choix chez le sujet âgé et l'insuffisant hépatique"],
            &["Clonazépam (Rivotril)", "environ 0,5 mg", "Longue : 30 à 40 h", "Antiépileptique, prescription restreinte", "Hors AMM dans l'anxiété et l'insomnie"],
            &["Lormétazépam (Noctamide)", "1 mg", "Courte à intermédiaire : environ 10 h", "Hypnotique", "Chutes nocturnes, confusion : demi-dose"],
            &["Zolpidem (Stilnox)", "20 mg", "Courte : environ 2,5 h", "Hypnotique apparenté", "Ordonnance sécurisée, 28 jours ; troubles du comportement nocturne"],
            &["Zopiclone (Imovane)", "15 mg", "Courte : environ 5 h", "Hypnotique apparenté", "Goût amer, somnolence résiduelle au réveil"],
            &["Arrêt progressif", "Relais possible par une molécule à demi-vie longue", "—", "Durée limitée : 12 semaines en anxiolytique, 4 semaines en hypnotique", "Diminution par paliers sur plusieurs semaines, jamais d'arrêt brutal"],
        ],
    },
    ConvTable {
        short: "AOD",
        family: "Posologies",
        title: "AOD — posologies, adaptation rénale et antidotes",
        reviewed: "Août 2026 — RCP à jour ; antidotes disponibles en France",
        sources: &[
            "RCP Eliquis, Xarelto, Pradaxa, Lixiana (ANSM, base de données publique des médicaments)",
            "ESC 2020 — prise en charge de la fibrillation atriale",
            "GIHP — gestion péri-opératoire et hémorragique des AOD",
        ],
        columns: &["DCI (spécialité)", "FA non valvulaire", "Dose réduite si", "MTEV", "Demi-vie et antidote", "Surveillance rénale"],
        rows: &[
            &["Apixaban (Eliquis)", "5 mg x2/j", "2,5 mg x2/j si 2 critères : âge ≥ 80 ans, poids ≤ 60 kg, créat. ≥ 133 µmol/L", "10 mg x2/j pendant 7 j puis 5 mg x2/j", "Environ 12 h ; andexanet alfa (Ondexxya) en hémorragie grave", "DFG au moins une fois par an, plus souvent si DFG < 60 ou sujet âgé"],
            &["Rivaroxaban (Xarelto)", "20 mg x1/j au cours d'un repas", "15 mg x1/j si DFG 15 à 49 mL/min", "15 mg x2/j pendant 21 j puis 20 mg x1/j", "5 à 13 h (allongée chez le sujet âgé) ; andexanet alfa", "DFG au moins une fois par an, plus souvent si DFG < 60"],
            &["Dabigatran (Pradaxa)", "150 mg x2/j", "110 mg x2/j si ≥ 80 ans, vérapamil ou risque hémorragique", "150 mg x2/j après au moins 5 jours d'héparine", "12 à 17 h, très allongée si insuffisance rénale ; idarucizumab (Praxbind)", "DFG au moins une fois par an ; tous les 6 mois si DFG 30 à 50 ou âge ≥ 75 ans"],
            &["Édoxaban (Lixiana)", "60 mg x1/j", "30 mg x1/j si DFG 15 à 50, poids ≤ 60 kg ou inhibiteur de la P-gp", "60 mg x1/j après au moins 5 jours d'héparine", "10 à 14 h ; pas d'antidote spécifique, CCP en hémorragie grave", "DFG au moins une fois par an, plus souvent si DFG abaissé"],
            &["Limites rénales", "Dabigatran : contre-indiqué si DFG < 30 — autres AOD : DFG < 15", "AVK si DFG effondré, valve mécanique ou SAPL", "Relais héparine ou AVK selon l'avis du prescripteur", "L'élimination rénale du dabigatran est la plus forte des quatre", "Recalculer le DFG à chaque épisode aigu : fièvre, diarrhée, canicule, diurétique"],
            &["Repères de dispensation", "Pas d'AINS ni d'aspirine sans avis médical", "Ne jamais réduire la dose de sa propre initiative", "La durée de traitement est fixée par le prescripteur", "Un oubli n'est pas rattrapé si la prise suivante est proche", "Signaler saignements, selles noires, anémie ou chute inexpliquée"],
            &["Prise et alimentation", "Rivaroxaban 15 et 20 mg : au cours d'un repas, sinon l'absorption chute", "Apixaban, dabigatran et édoxaban : indifférent", "Dabigatran : gélule avalée entière, jamais ouverte — la poudre multiplie l'absorption", "Apixaban et rivaroxaban peuvent être écrasés et donnés dans de l'eau ou de la compote", "Le dabigatran se conserve dans son flacon ou sa plaquette d'origine : l'humidité le dégrade"],
            &["Oubli d'une prise", "Deux prises par jour : prendre dès que possible dans les 6 h, sinon sauter", "Une prise par jour : prendre dans les 12 h, sinon sauter", "Jamais deux doses le même moment pour rattraper", "Un oubli répété se signale : c'est le motif d'échec le plus fréquent", "Un pilulier hebdomadaire n'est pas contre-indiqué, mais le dabigatran n'y va pas"],
            &["Interactions qui comptent", "Inhibiteurs puissants du CYP3A4 et de la P-gp (kétoconazole, itraconazole, ritonavir, clarithromycine) : exposition augmentée", "Inducteurs (rifampicine, carbamazépine, phénytoïne, millepertuis) : exposition diminuée, association déconseillée", "Vérapamil et amiodarone : dose de dabigatran à revoir", "AINS, aspirine, antiagrégants, ISRS : risque hémorragique additif", "Aucun contrôle biologique de routine : l'INR ne mesure rien ici"],
            &["Geste invasif ou chirurgie", "Arrêt 24 h avant un geste à risque hémorragique faible", "48 h avant un geste à risque élevé", "Plus long si la fonction rénale est altérée, surtout pour le dabigatran", "Reprise sur avis, en général 24 à 72 h après, quand l'hémostase est acquise", "Extraction dentaire simple : le plus souvent sans arrêt, avec des mesures locales"],
            &["Ce qui fait arrêter et appeler", "Saignement qui ne cède pas à la compression", "Selles noires ou sang dans les urines", "Traumatisme crânien, même sans perte de connaissance", "Grossesse : les AOD sont contre-indiqués", "Insuffisance hépatique sévère ou coagulopathie : contre-indication"],
        ],
    },
    ConvTable {
        short: "Cortico. inhalés",
        family: "Posologies",
        title: "Corticoïdes inhalés — paliers de dose, dispositifs et rinçage (adulte)",
        reviewed: "Août 2026 — GINA en vigueur, RCP à jour",
        sources: &[
            "GINA — Global Strategy for Asthma Management and Prevention, tableau des paliers de dose adulte",
            "RCP des dispositifs inhalés (ANSM)",
            "HAS — bon usage des corticoïdes inhalés et éducation à la technique d'inhalation",
        ],
        columns: &["DCI (exemples)", "Faible", "Moyenne", "Forte", "Dispositifs et chambre", "Rinçage et conseils"],
        rows: &[
            &["Béclométasone extrafine (Qvar, Foster)", "100 à 200 µg", "> 200 à 400 µg", "> 400 µg", "Aérosol-doseur : chambre d'inhalation utile ; Foster existe aussi en poudre (NEXThaler)", "Rincer la bouche et cracher après chaque prise"],
            &["Budésonide (Pulmicort, Symbicort)", "200 à 400 µg", "> 400 à 800 µg", "> 800 µg", "Poudre (Turbuhaler, Easyhaler) : pas de chambre ; suspension pour nébulisation disponible", "Rincer et cracher ; se laver le visage après une nébulisation au masque"],
            &["Fluticasone propionate (Flixotide, Seretide)", "100 à 250 µg", "> 250 à 500 µg", "> 500 µg", "Aérosol-doseur (chambre possible) et poudre Diskus", "Rincer et cracher ; prise avant le brossage des dents"],
            &["Ciclésonide (Alvesco)", "80 à 160 µg", "> 160 à 320 µg", "> 320 µg", "Aérosol-doseur, chambre d'inhalation possible", "Rincer quand même : prodrogue activée dans le poumon, effets locaux moindres"],
            &["Mométasone (Asmanex)", "200 µg", "400 µg", "> 400 µg", "Poudre (Twisthaler) : inspiration profonde et rapide, jamais de chambre", "Rincer et cracher après la prise"],
            &["Repères communs", "Viser la dose la plus faible qui contrôle l'asthme", "Toute augmentation est décidée par le prescripteur", "Doses fortes prolongées : effets systémiques possibles", "Chambre d'inhalation avec les aérosols-doseurs, jamais avec les poudres ; laver la chambre à l'eau savonneuse et laisser sécher à l'air", "Rincer la bouche prévient candidose et dysphonie ; vérifier la technique à chaque délivrance"],
            &["Association fixe corticoïde + bêta-2 de longue durée", "Le corticoïde ne s'arrête pas quand l'asthme va mieux", "Le bêta-2 de longue durée n'est jamais utilisé seul dans l'asthme", "Certaines associations servent aussi de traitement de secours : c'est l'ordonnance qui le dit, pas la boîte", "Même dispositif que le corticoïde seul", "Vérifier qu'un second corticoïde inhalé n'a pas été ajouté à côté : le doublon est fréquent après un changement de marque"],
            &["Ce qui trahit un asthme non contrôlé", "Symptômes diurnes plus de deux fois par semaine", "Réveils nocturnes", "Recours au traitement de secours plus de deux fois par semaine", "Limitation de l'activité physique", "Trois flacons de secours délivrés dans l'année, ou un flacon par mois : signaler au prescripteur"],
            &["Enfant", "Les paliers de dose sont plus bas que chez l'adulte : ne pas transposer ce tableau", "Chambre d'inhalation systématique avec les aérosols-doseurs", "Masque jusqu'à 3 ou 4 ans, embout buccal ensuite", "Surveiller la croissance sous doses fortes prolongées", "Rincer la bouche et laver le visage après le masque"],
            &["Effets locaux et ce qu'on en fait", "Candidose buccale : rinçage insuffisant ou technique perfectible", "Dysphonie : plus fréquente avec les poudres", "Toux à l'inhalation : souvent le débit d'inspiration ou l'excipient", "Aucun de ces effets ne justifie d'arrêter le traitement de fond", "Reprendre la technique, proposer une chambre, en parler au prescripteur"],
        ],
    },
    ConvTable {
        short: "Insulines",
        family: "Posologies",
        title: "Insulines — profils d'action, injection et conservation",
        reviewed: "Août 2026 — RCP à jour",
        sources: &[
            "RCP des spécialités (ANSM, base de données publique des médicaments)",
            "SFD — référentiel insulinothérapie et prise en charge du diabète",
            "HAS — éducation thérapeutique du patient insulinotraité",
        ],
        columns: &["Type (spécialités)", "Début", "Pic", "Durée", "Moment de l'injection", "Conservation"],
        rows: &[
            &["Analogue rapide (Humalog, NovoRapid, Apidra)", "10 à 20 min", "1 à 3 h", "3 à 5 h", "Juste avant le repas ; chez le jeune enfant, juste après si la prise alimentaire est incertaine", "2 à 8 °C non entamé ; stylo en cours à température ambiante 4 semaines"],
            &["Humaine rapide (Actrapid, Umuline Rapide)", "30 à 60 min", "2 à 4 h", "6 à 8 h", "20 à 30 min avant le repas", "2 à 8 °C non entamé ; durée d'utilisation après ouverture selon le RCP"],
            &["Prémix biphasique (NovoMix, Humalog Mix)", "10 à 20 min (fraction rapide)", "Double : rapide puis prolongé", "10 à 16 h", "Juste avant le repas, après homogénéisation", "2 à 8 °C non entamé ; remettre en suspension avant chaque injection"],
            &["NPH intermédiaire (Insulatard, Umuline NPH)", "1 à 2 h", "4 à 8 h", "12 à 16 h", "Indépendante du repas, souvent au coucher ; retourner le stylo une dizaine de fois", "2 à 8 °C non entamé ; durée après ouverture selon le RCP"],
            &["Glargine U100 (Lantus, Abasaglar)", "2 à 4 h", "Sans pic marqué", "20 à 24 h", "Une fois par jour à heure fixe, indépendamment des repas", "2 à 8 °C non entamé ; stylo en cours à température ambiante 4 semaines"],
            &["Glargine U300 (Toujeo)", "Environ 6 h", "Sans pic", "Plus de 24 h", "Une fois par jour à heure fixe", "Ne jamais transvaser dans une seringue : concentration U300, risque de surdosage"],
            &["Détémir (Levemir)", "1 à 2 h", "Peu marqué", "12 à 20 h", "Une à deux fois par jour à heure fixe", "2 à 8 °C non entamé ; durée après ouverture selon le RCP"],
            &["Dégludec (Tresiba)", "Environ 1 h", "Sans pic", "Plus de 42 h", "Une fois par jour, horaire souple d'un jour à l'autre", "2 à 8 °C non entamé ; stylo en cours à température ambiante 8 semaines"],
            &["Règles communes", "Le site d'injection modifie la vitesse d'absorption", "Une lipodystrophie fausse le profil : changer de site", "Ne jamais doubler une dose oubliée", "Aiguille neuve à chaque injection ; rotation des sites", "Jamais de congélation ni de soleil direct ; jeter la cartouche si l'aspect a changé"],
        ],
    },
    ConvTable {
        short: "Fonction rénale",
        family: "Adaptation",
        title: "Fonction rénale — stades et conséquences pratiques",
        reviewed: "Août 2026 — classification KDIGO, adaptations issues des RCP",
        sources: &[
            "Cockcroft D. W., Gault M. H. — Nephron, 1976",
            "KDIGO — classification de la maladie rénale chronique",
            "RCP metformine, AOD et HBPM (ANSM)",
        ],
        columns: &["Stade", "DFG (mL/min)", "Metformine", "AOD", "HBPM", "Autres repères"],
        rows: &[
            &["G1 — normal", "≥ 90", "Pleine dose", "Doses standard du RCP", "Doses standard, curatif et préventif", "Cockcroft : (140 − âge) x poids (kg) x k / créatinine (µmol/L), k = 1,23 homme et 1,04 femme"],
            &["G2 — légère", "60 à 89", "Pleine dose", "Doses standard du RCP", "Doses standard", "Contrôle du DFG au moins une fois par an"],
            &["G3a — modérée", "45 à 59", "Dose réduite, contrôle du DFG tous les 3 à 6 mois", "Réduction possible selon la molécule et les critères du RCP", "Curatif possible, surveillance clinique rapprochée", "Revoir tout le traitement à élimination rénale (allopurinol, gabapentine, colchicine)"],
            &["G3b — modérée à sévère", "30 à 44", "Dose réduite, réévaluation régulière, arrêt en situation à risque", "Rivaroxaban et édoxaban réduits ; dabigatran réduit ou évité", "Adaptation ou relais par HNF selon le RCP ; anti-Xa possible", "Éviter les AINS ; prudence avec les produits de contraste iodés"],
            &["G4 — sévère", "15 à 29", "Contre-indiquée", "Dabigatran contre-indiqué ; autres AOD à doses réduites et sur avis", "Curatif : adaptation ou HNF, avis spécialisé", "Réévaluer chaque ligne de l'ordonnance, avis néphrologique"],
            &["G5 — terminale", "< 15 ou dialyse", "Contre-indiquée", "Contre-indiqués", "Curatif non recommandé, préférer l'HNF", "AVK ou HNF sur avis néphrologique"],
            &["Situation aiguë", "DFG à recalculer", "Arrêt avant injection d'iode et en cas de déshydratation", "Recalculer le DFG avant toute reprise", "Recalculer le poids et le DFG avant l'adaptation", "Fièvre, diarrhée, vomissements, canicule, diurétiques : le DFG chute vite"],
            &["Limites du calcul", "Cockcroft ou CKD-EPI selon la source", "Les RCP de la metformine se réfèrent au DFG", "Les RCP des AOD se réfèrent à la clairance de Cockcroft", "Le calcul se fait sur le poids réel du patient", "Formules prises en défaut chez l'obèse, le dénutri, le sujet très âgé et l'amputé"],
        ],
    },
    ConvTable {
        short: "Angine",
        family: "Au comptoir",
        title: "Angine — score de Mac Isaac, TROD et antibiothérapie",
        reviewed: "Août 2026 — protocole de dispensation après TROD en vigueur",
        sources: &[
            "HAS / SPILF — angine aiguë de l'adulte et de l'enfant, test rapide d'orientation diagnostique",
            "Mac Isaac W. J. et al. — CMAJ, 1998",
            "RCP des antibiotiques concernés (ANSM)",
        ],
        columns: &["Critère (score de Mac Isaac)", "Points", "Situation", "Conduite à tenir", "Antibiothérapie et durée"],
        rows: &[
            &["Fièvre > 38 °C", "+1", "Adulte, score 0 ou 1", "Pas de TROD, pas d'antibiotique", "Traitement symptomatique : antalgique et antipyrétique"],
            &["Absence de toux", "+1", "Adulte, score ≥ 2", "TROD streptococcique recommandé", "Antibiotique uniquement si le test est positif"],
            &["Adénopathies cervicales sensibles", "+1", "TROD négatif", "Pas d'antibiotique, traitement de la douleur", "Reconsulter si aggravation ou fièvre persistante"],
            &["Atteinte amygdalienne (exsudat ou tuméfaction)", "+1", "TROD positif, adulte", "Antibiothérapie du streptocoque du groupe A", "Amoxicilline 1 g x2/j pendant 6 jours"],
            &["Âge 3 à 14 ans", "+1", "Enfant à partir de 3 ans", "TROD devant toute angine érythémateuse, sans calcul de score", "Si positif : amoxicilline 50 mg/kg/j en 2 prises, 6 jours"],
            &["Âge 15 à 44 ans", "0", "Allergie aux pénicillines sans contre-indication aux céphalosporines", "Céphalosporine de 2e ou 3e génération", "Céfuroxime-axétil ou cefpodoxime, durée courte selon la recommandation"],
            &["Âge ≥ 45 ans", "−1", "Contre-indication à toutes les bêta-lactamines", "Macrolide, après prélèvement de gorge pour culture", "Azithromycine 3 jours ou clarithromycine, selon la recommandation"],
            &["Enfant de moins de 3 ans (hors score)", "—", "Angine presque toujours virale", "Ni TROD ni antibiotique en règle générale", "Avis médical devant tout signe de gravité"],
        ],
    },
    ConvTable {
        short: "Cystite",
        family: "Au comptoir",
        title: "Cystite simple — traitements, contre-indications et suivi",
        reviewed: "Août 2026 — protocole de dispensation après TROD en vigueur, recommandations SPILF",
        sources: &[
            "SPILF — infections urinaires bactériennes communautaires de l'adulte",
            "HAS — cystite aiguë simple, prise en charge et dispensation à l'officine",
            "RCP des spécialités (ANSM)",
        ],
        columns: &["Rang ou situation", "Traitement", "Durée", "Contre-indication ou précaution", "Suivi"],
        rows: &[
            &["1re intention", "Fosfomycine trométamol (Monuril) 3 g", "Dose unique", "Prise à distance d'un repas, de préférence au coucher, après avoir uriné", "Pas d'ECBU de contrôle si l'évolution est favorable"],
            &["2e intention", "Pivmécillinam (Selexid) 400 mg x2/j", "3 à 5 jours", "Allergie aux pénicillines ; à avaler avec un grand verre d'eau, en position assise", "Réévaluation si les signes persistent au-delà de 72 h"],
            &["3e intention", "Nitrofurantoïne (Furadantine) 100 mg x3/j", "5 jours", "Jamais en traitement prolongé ni préventif : toxicité hépatique et pulmonaire ; contre-indiquée si insuffisance rénale", "Prévenir de la coloration brune des urines"],
            &["À éviter en probabiliste", "Fluoroquinolones, cotrimoxazole, amoxicilline seule", "—", "Résistances, tendinopathies et effets indésirables graves", "Réservés aux traitements guidés par l'antibiogramme"],
            &["Cystite à risque de complication", "Antibiothérapie guidée par l'ECBU quand elle peut être différée", "Plus longue que dans la cystite simple", "Grossesse, homme, immunodépression, insuffisance rénale sévère, uropathie", "ECBU systématique et avis médical"],
            &["Femme enceinte", "Traitement adapté à l'antibiogramme, avis médical", "Selon la molécule retenue", "Plusieurs molécules sont contre-indiquées selon le terme", "ECBU de contrôle après le traitement puis surveillance régulière"],
            &["Signes d'alerte", "Fièvre, frissons, douleur lombaire, vomissements, hématurie persistante", "—", "Faire suspecter une pyélonéphrite ou une prostatite", "Consultation le jour même, pas de délivrance de conseil seul"],
            &["Conseils au comptoir", "Boissons abondantes, mictions régulières et complètes", "—", "La canneberge n'est pas un traitement curatif", "Réévaluation à 72 h ; ECBU en cas de récidive ou d'échec"],
        ],
    },
    ConvTable {
        short: "Contraception",
        family: "Au comptoir",
        title: "Contraception — oubli, délai toléré et rattrapage",
        reviewed: "Août 2026 — recommandations HAS sur la contraception",
        sources: &[
            "HAS — contraception : conduite à tenir en cas d'oubli et contraception d'urgence",
            "RCP des spécialités contraceptives (ANSM)",
        ],
        columns: &["Situation", "Délai toléré", "Conduite à tenir", "Préservatif", "Contraception d'urgence"],
        rows: &[
            &["Œstroprogestatif, oubli de moins de 12 h", "12 h", "Prendre le comprimé oublié aussitôt et poursuivre à l'heure habituelle", "Inutile", "Non indiquée"],
            &["Œstroprogestatif, oubli de plus de 12 h", "Dépassé", "Prendre le dernier comprimé oublié et poursuivre la plaquette normalement", "Oui, pendant 7 jours", "Oui si rapport dans les 5 jours précédents"],
            &["Oubli pendant la 3e semaine de plaquette", "Dépassé", "Enchaîner la plaquette suivante sans intervalle libre", "Oui, pendant 7 jours", "Selon les rapports des 5 derniers jours"],
            &["Microprogestatif au désogestrel (Optimizette, Cerazette)", "12 h", "Prendre le comprimé oublié et poursuivre à l'heure habituelle", "Oui, 7 jours si le retard dépasse 12 h", "Oui si rapport dans les 5 jours précédents"],
            &["Microprogestatif au lévonorgestrel (Microval)", "3 h seulement", "Prendre le comprimé aussitôt et poursuivre", "Oui, 7 jours si le retard dépasse 3 h", "Oui si rapport dans les 5 jours précédents"],
            &["Vomissements ou diarrhée dans les 4 h suivant la prise", "—", "Comprimé considéré comme oublié : reprendre un comprimé", "Selon la règle de l'oubli correspondant", "Selon la règle de l'oubli correspondant"],
            &["Patch (Evra) décollé", "Moins de 24 h", "Recoller ou remplacer, en gardant le même jour de changement", "Oui, 7 jours si le décollement dépasse 24 h", "Oui si rapport et décollement prolongé"],
            &["Anneau (Nuvaring) expulsé ou retiré", "3 h", "Rincer à l'eau tiède et remettre en place aussitôt", "Oui, 7 jours si le retrait dépasse 3 h", "Oui si rapport et retrait prolongé"],
            &["Contraception d'urgence", "Le plus tôt possible après le rapport", "Lévonorgestrel (Norlevo) jusqu'à 72 h ; ulipristal (EllaOne) jusqu'à 120 h", "Préservatif jusqu'aux règles suivantes", "Délivrance sans ordonnance à l'officine ; le DIU au cuivre, jusqu'à 5 jours, est la méthode la plus efficace"],
        ],
    },
    ConvTable {
        short: "Antalgiques",
        family: "Posologies",
        title: "Antalgiques — palier, doses adulte et précautions",
        reviewed: "Août 2026 — RCP à jour, seuils de paracétamol révisés",
        sources: &[
            "RCP des spécialités (ANSM, base de données publique des médicaments)",
            "ANSM — bon usage du paracétamol, des AINS et des antalgiques opioïdes",
            "HAS / SFETD — prise en charge de la douleur",
        ],
        columns: &["Molécule", "Palier", "Dose usuelle", "Maximum / 24 h", "Durée max en automédication", "Précaution principale"],
        rows: &[
            &["Paracétamol", "Palier I", "500 mg à 1 g par prise, au moins 4 h entre deux prises", "3 g (4 g sur avis médical)", "5 jours pour la douleur, 3 jours pour la fièvre", "Hépatique : jamais deux spécialités en contenant ; réduire si poids faible, sujet âgé, alcool ou dénutrition"],
            &["Ibuprofène", "Palier I, AINS", "200 à 400 mg x3/j au cours d'un repas", "1 200 mg en automédication", "5 jours pour la douleur, 3 jours pour la fièvre", "Digestif et rénal ; contre-indiqué à partir du 6e mois de grossesse"],
            &["Kétoprofène", "Palier I, AINS", "Forme LP 100 mg x1 à 2/j", "200 mg", "Sur prescription", "Digestif et rénal ; photosensibilisation avec la forme gel"],
            &["Diclofénac", "Palier I, AINS", "50 mg x2 à 3/j", "150 mg", "Sur prescription", "Cardiovasculaire : éviter après un infarctus, un AVC ou en cas d'insuffisance cardiaque"],
            &["Naproxène", "Palier I, AINS", "250 à 500 mg x2/j", "1 100 mg", "Sur prescription (dosage conseil plus faible)", "Digestif ; majoration du risque hémorragique avec AVK et AOD"],
            &["Aspirine (usage antalgique)", "Palier I", "500 mg à 1 g par prise, espacées d'au moins 4 h", "3 g", "3 à 5 jours", "Digestif et hémorragique ; jamais chez l'enfant fébrile (syndrome de Reye)"],
            &["Néfopam (Acupan)", "Non opioïde, hors palier OMS", "20 mg x4 à 6/j", "120 mg", "Uniquement sur prescription", "Effets anticholinergiques : glaucome, adénome prostatique, antécédent de convulsions"],
            &["Codéine (en association)", "Palier II", "Selon l'association, toutes les 6 h", "Selon la spécialité", "Sur ordonnance obligatoire", "Contre-indiquée avant 12 ans et après amygdalectomie ; constipation, somnolence"],
            &["Tramadol", "Palier II", "50 à 100 mg toutes les 4 à 6 h en libération immédiate", "400 mg", "Sur ordonnance, durée de prescription limitée", "Convulsions, syndrome sérotoninergique, hyponatrémie ; prudence chez le sujet âgé"],
            &["Deux AINS ensemble", "—", "À proscrire", "—", "—", "Toxicité digestive et rénale cumulée sans gain d'efficacité ; interroger sur l'automédication"],
        ],
    },
    ConvTable {
        short: "Vaccins",
        family: "Au comptoir",
        title: "Vaccination à l'officine — population, schéma et rôle du pharmacien",
        reviewed: "Août 2026 — calendrier vaccinal en vigueur",
        sources: &[
            "Calendrier des vaccinations et recommandations vaccinales (ministère chargé de la Santé)",
            "HAS — extension des compétences vaccinales du pharmacien d'officine",
            "RCP des vaccins (ANSM)",
        ],
        columns: &["Vaccin", "Population concernée", "Schéma ou rythme", "Administration par le pharmacien", "Remarque"],
        rows: &[
            &["dTP (diphtérie, tétanos, poliomyélite)", "Tous les adultes", "Rappels à 25, 45 et 65 ans, puis tous les 10 ans", "Oui, à partir de 11 ans", "Le rappel de 25 ans comporte la valence coquelucheuse"],
            &["Coqueluche (dTcaP)", "Adultes au contact d'un nourrisson, femmes enceintes", "Rappel adulte et stratégie du cocooning ; vaccination pendant la grossesse selon le calendrier en vigueur", "Oui, à partir de 11 ans", "Protection indirecte du nourrisson avant ses premières doses"],
            &["Grippe saisonnière", "65 ans et plus, personnes à risque, femmes enceintes, entourage des nourrissons fragiles", "Une dose chaque automne", "Oui, à partir de 11 ans", "Bons de prise en charge de l'Assurance maladie ; co-administration possible avec le COVID-19"],
            &["COVID-19", "Personnes âgées et à risque, selon les recommandations de la campagne", "Rappel selon les recommandations en vigueur", "Oui, à partir de 11 ans", "Vérifier le délai depuis la dernière dose ou infection"],
            &["Pneumocoque", "Immunodéprimés et personnes à risque, nourrissons selon le calendrier", "Schéma conjugué puis polyosidique selon la recommandation en vigueur", "Oui, à partir de 11 ans", "Le schéma dépend des vaccins pneumococciques déjà reçus"],
            &["Zona (Shingrix)", "À partir de 65 ans ; immunodéprimés dès 18 ans", "2 doses espacées selon le RCP", "Oui, à partir de 11 ans", "Vaccin non vivant ; réactions locales et générales fréquentes, à annoncer"],
            &["HPV", "Filles et garçons de 11 à 14 ans, rattrapage jusqu'à 19 ans, HSH jusqu'à 26 ans", "2 doses avant 15 ans, 3 doses au-delà", "Oui, à partir de 11 ans", "Campagne proposée en classe de 5e"],
            &["ROR", "Personnes nées depuis 1980, non ou incomplètement vaccinées", "2 doses au total au cours de la vie", "Oui, sauf chez l'immunodéprimé (vaccin vivant)", "Contre-indiqué pendant la grossesse"],
            &["Hépatite B", "Nourrissons, personnes exposées professionnellement ou par le mode de vie", "Schéma selon l'âge et le RCP", "Oui, à partir de 11 ans", "Contrôle sérologique chez les professionnels de santé exposés"],
            &["Méningocoques", "Nourrissons et adolescents selon le calendrier en vigueur", "Selon l'âge et la valence (B, ACWY)", "Oui, à partir de 11 ans", "Les recommandations ont été élargies : vérifier le calendrier de l'année en cours"],
            &["Cadre de l'acte", "Personnes de 11 ans et plus", "Prescription et administration des vaccins du calendrier vaccinal", "Hors vaccins vivants chez l'immunodéprimé", "Tracer l'acte dans le carnet de vaccination et informer le médecin traitant"],
        ],
    },
    ConvTable {
        short: "Pédiatrie",
        family: "Adaptation",
        title: "Pédiatrie — doses usuelles, formes et maximum par jour",
        reviewed: "Août 2026 — RCP à jour",
        sources: &[
            "RCP pédiatriques (ANSM, base de données publique des médicaments)",
            "GPIP / SFP — antibiothérapie et prise en charge de la fièvre chez l'enfant",
            "OMS — solutions de réhydratation orale",
        ],
        columns: &["Molécule", "Dose par prise", "Rythme", "Maximum / 24 h", "Forme habituelle", "Repère pratique"],
        rows: &[
            &["Paracétamol", "15 mg/kg", "Toutes les 6 h, 4 h au minimum", "60 mg/kg", "Suspension buvable avec pipette graduée en kg ; suppositoire", "Lire la pipette en kilos, jamais en millilitres"],
            &["Ibuprofène (à partir de 3 mois)", "7,5 à 10 mg/kg", "Toutes les 6 à 8 h", "30 mg/kg", "Suspension buvable avec pipette-dose graduée en kg", "À éviter en cas de varicelle, de déshydratation ou d'infection cutanée"],
            &["Amoxicilline (otite moyenne aiguë, pneumonie)", "25 à 50 mg/kg", "En 2 à 3 prises par jour", "80 à 100 mg/kg/j selon l'indication", "Poudre pour suspension buvable reconstituée à l'eau", "Agiter avant chaque prise ; conservation limitée après reconstitution, voir le flacon"],
            &["Amoxicilline (angine à streptocoque A)", "25 mg/kg", "x2/j pendant 6 jours", "50 mg/kg/j", "Poudre pour suspension buvable", "TROD positif requis avant de traiter"],
            &["Solution de réhydratation orale", "Petites quantités répétées, à volonté", "Après chaque selle et en continu", "Un sachet dans 200 mL d'eau, sans autre dilution", "Sachet à reconstituer, conservé 24 h au réfrigérateur", "Reconsulter en cas de refus de boire, de léthargie ou de vomissements incoercibles"],
            &["Vitamine D (nourrisson)", "400 à 800 UI/j", "Quotidien", "Selon la prescription et les apports du lait", "Solution buvable en gouttes ou en ampoule", "Compter les gouttes dans une cuillère, jamais directement au flacon"],
            &["Aspirine", "Non recommandée comme antipyrétique chez l'enfant", "—", "—", "—", "Syndrome de Reye : à proscrire en cas de varicelle ou de virose"],
            &["Fièvre — conseils associés", "Découvrir l'enfant et proposer à boire souvent", "Contrôle de la température si besoin", "Pas d'alternance systématique de deux antipyrétiques", "—", "Avis médical avant 3 mois, si la fièvre dépasse 48 h ou devant tout signe de gravité"],
            &["Repères de poids", "Environ 3,5 kg à la naissance, 10 kg à 1 an, 20 kg à 6 ans", "—", "—", "—", "Peser l'enfant : ne jamais doser sur l'âge seul"],
        ],
    },
    ConvTable {
        short: "Broyage",
        family: "Administration",
        title: "Écraser ou ouvrir — règles, raisons et alternatives",
        reviewed: "Août 2026 — listes de l'OMÉDIT, RCP à jour",
        sources: &[
            "SFPC — liste nationale des médicaments écrasables et recommandations associées",
            "OMÉDIT — bon usage de l'écrasement des comprimés et de l'ouverture des gélules",
            "RCP des spécialités (ANSM)",
        ],
        columns: &["Forme ou molécule", "Écrasable", "Pourquoi", "Alternative pratique", "Précaution"],
        rows: &[
            &["Comprimé à libération prolongée (LP, LM, Chrono, Repetabs)", "Non", "Toute la dose serait libérée d'un coup", "Forme à libération immédiate équivalente, sur avis du prescripteur", "Le suffixe sur la boîte suffit à interdire l'écrasement"],
            &["Comprimé gastro-résistant (entérosoluble)", "Non", "Le principe actif est dégradé par l'acidité ou irrite l'estomac", "Forme buvable, orodispersible ou injectable selon la molécule", "Ne pas confondre un enrobage gastro-résistant avec un simple pelliculage"],
            &["Gélule à microgranules gastro-résistants (Inexium, Mopral, Créon)", "Ouvrir, ne pas écraser", "La protection gastrique est portée par chaque granule", "Sachet ou suspension buvable quand la spécialité existe", "Avaler les granules sans les croquer, dans une compote, aussitôt après ouverture"],
            &["Gélule LP à microgranules (Skenan LP)", "Ouvrir, ne pas croquer", "Croquer libère toute la morphine d'un coup", "Solution buvable de morphine sur prescription adaptée", "Rincer le contenant : la fraction restante fait partie de la dose"],
            &["Dabigatran (Pradaxa)", "Jamais ouvrir", "La biodisponibilité est doublée, risque hémorragique", "Autre anticoagulant sur avis du prescripteur", "Signaler au prescripteur toute gélule ouverte par erreur"],
            &["Apixaban (Eliquis), rivaroxaban (Xarelto)", "Oui", "Écrasement documenté par le laboratoire", "Administrer aussitôt dans de l'eau ou une compote", "Xarelto 15 et 20 mg : au cours d'un repas, même écrasé"],
            &["Comprimé orodispersible ou lyophilisat", "Ne pas écraser", "Il se délite déjà dans la bouche, l'écraser n'apporte rien", "C'est justement la forme à privilégier en cas de trouble de la déglutition", "Mains sèches, laisser fondre sans eau"],
            &["Comprimé sécable", "Diviser seulement", "La barre autorise la coupe en deux, pas le broyage", "Chercher un dosage inférieur existant", "Une barre décorative n'est pas une barre de sécabilité"],
            &["Cytotoxiques et immunosuppresseurs (capécitabine, méthotrexate, mycophénolate)", "Non", "Exposition du soignant et de l'entourage aux poussières", "Forme buvable prête à l'emploi ou préparation hospitalière", "Gants, pas d'aérosolisation, comprimé avalé entier"],
            &["Médicament irritant ou tératogène (bisphosphonates, finastéride, dutastéride)", "Non", "Lésions œsophagiennes ou risque au contact cutané", "Autre forme galénique ou autre rythme d'administration, sur avis", "Une femme enceinte ne doit pas manipuler ces comprimés écrasés"],
            &["En pratique", "Chercher d'abord une alternative", "Un écrasement non documenté modifie le médicament et engage la responsabilité", "Forme buvable, orodispersible, patch, suppositoire ou autre dosage", "Écraser juste avant l'administration, un médicament à la fois, matériel nettoyé entre chaque"],
        ],
    },
    ConvTable {
        short: "Interactions",
        family: "Au comptoir",
        title: "Interactions à repérer à la délivrance — aliments, plantes et inducteurs",
        reviewed: "Août 2026 — thésaurus des interactions médicamenteuses de l'ANSM",
        sources: &[
            "ANSM — Thésaurus des interactions médicamenteuses",
            "RCP des spécialités concernées, base de données publique des médicaments",
            "ANSM — millepertuis et interactions médicamenteuses, mise au point",
        ],
        columns: &["Ce qui interagit", "Mécanisme", "Médicaments concernés", "Conséquence attendue", "Conduite au comptoir"],
        rows: &[
            &["Pamplemousse (fruit et jus)", "Inhibition du CYP3A4 intestinal, durable après une seule prise", "Simvastatine et atorvastatine, inhibiteurs calciques dihydropyridines, ciclosporine, tacrolimus, colchicine", "Concentrations augmentées, parfois d'un facteur trois : myalgies, œdèmes, toxicité", "Suppression du pamplemousse, pas un simple espacement : l'inhibition dure plus de 24 heures. Les autres agrumes ne posent pas le même problème."],
            &["Millepertuis", "Induction du CYP3A4 et de la P-glycoprotéine", "Contraceptifs oraux, anticoagulants oraux directs, antivitamines K, immunosuppresseurs, antirétroviraux, antidépresseurs sérotoninergiques", "Perte d'efficacité pouvant aller jusqu'à l'échec de la contraception ou du greffon ; syndrome sérotoninergique avec les antidépresseurs", "Contre-indication ou association déconseillée selon le médicament. À rechercher activement : le patient ne le déclare pas, parce que c'est « une plante »."],
            &["Rifampicine et antiépileptiques inducteurs", "Induction enzymatique puissante, installée en quelques jours et persistante après l'arrêt", "Contraceptifs hormonaux, antivitamines K, corticoïdes, anticoagulants oraux directs", "Baisse marquée des concentrations, y compris pendant deux semaines après l'arrêt de l'inducteur", "Contraception : passage à un dispositif intra-utérin ou méthode mécanique. Contraception d'urgence : lévonorgestrel inefficace, préférer le dispositif intra-utérin."],
            &["Macrolides, sauf spiramycine", "Inhibition du CYP3A4", "Colchicine, statines, anticoagulants oraux directs, immunosuppresseurs", "Surdosage : diarrhée puis toxicité médullaire avec la colchicine, rhabdomyolyse avec les statines", "Colchicine et clarithromycine : association contre-indiquée. Préférer la spiramycine ou l'azithromycine quand le choix est possible."],
            &["Calcium, fer, magnésium, zinc", "Chélation dans le tube digestif", "Cyclines, fluoroquinolones, lévothyroxine, biphosphonates", "Absorption effondrée, échec du traitement sans autre signe", "Espacer d'au moins 2 heures, et de 4 heures pour la lévothyroxine et les biphosphonates. Vérifier aussi les eaux minérales riches en calcium."],
            &["Inhibiteurs de la pompe à protons", "Élévation du pH gastrique et inhibition du CYP2C19", "Clopidogrel, antifongiques azolés, fer, vitamine B12, atazanavir", "Activation réduite du clopidogrel ; absorption réduite des autres", "Sous clopidogrel, préférer le pantoprazole à l'oméprazole et à l'ésoméprazole. Réévaluer tout traitement prolongé par inhibiteur de la pompe à protons."],
            &["Anti-inflammatoires non stéroïdiens", "Inhibition des prostaglandines rénales", "Inhibiteurs de l'enzyme de conversion, sartans, diurétiques", "Insuffisance rénale aiguë fonctionnelle, la « triade néfaste », majorée par la déshydratation", "Refuser l'automédication par anti-inflammatoire sous ce type de traitement, en particulier par forte chaleur, en cas de fièvre ou de gastro-entérite."],
            &["Alcool", "Effets additifs et interférences métaboliques", "Métronidazole, paracétamol, benzodiazépines, opioïdes, sulfamides hypoglycémiants", "Effet antabuse, majoration de l'hépatotoxicité, dépression respiratoire", "Le rappeler explicitement à la délivrance du métronidazole, y compris pour les 48 heures suivant la fin du traitement."],
            &["Jus de canneberge et compléments", "Interférence avec le métabolisme des antivitamines K", "Antivitamines K", "INR déséquilibré à la hausse", "Ni suppression ni interdiction des légumes verts : c'est la régularité des apports qui compte, avec un contrôle d'INR après tout changement d'habitude."],
        ],
    },
    ConvTable {
        short: "Urgence",
        family: "Au comptoir",
        title: "Urgence au comptoir — reconnaître, agir, orienter",
        reviewed: "Août 2026 — recommandations de premiers secours en vigueur",
        sources: &[
            "Recommandations de la Société française de médecine d'urgence",
            "HAS — prise en charge de l'anaphylaxie ; conduite à tenir devant une hypoglycémie",
            "Guide des gestes d'urgence à l'officine, Ordre national des pharmaciens",
        ],
        columns: &["Situation", "Ce que l'on voit", "Geste immédiat", "Ce qu'il ne faut pas faire", "Orientation"],
        rows: &[
            &["Anaphylaxie", "Urticaire étendue avec gêne respiratoire, gonflement de la gorge, malaise ou chute de tension, souvent en quelques minutes", "Adrénaline intramusculaire dans la face externe de la cuisse, sans attendre ; allonger jambes surélevées", "Ne pas faire asseoir ni lever la personne, ne pas se contenter d'un antihistaminique ou d'un corticoïde", "Appel du 15 dans tous les cas, même si les signes cèdent : une deuxième vague est possible dans les heures qui suivent."],
            &["Hypoglycémie consciente", "Sueurs, tremblements, faim, pâleur, troubles du comportement chez un patient diabétique", "15 g de sucre rapide : 3 morceaux de sucre, un verre de jus de fruit ou de soda non light, puis un sucre lent une fois les signes passés", "Ne pas donner de chocolat ni de produit gras, dont le sucre passe trop lentement", "Recontrôler la glycémie à 15 minutes et resucrer si nécessaire ; rechercher la cause avant de laisser repartir."],
            &["Hypoglycémie avec trouble de conscience", "Somnolence, confusion, impossibilité d'avaler en sécurité, convulsions", "Position latérale de sécurité ; glucagon si disponible et si l'entourage sait l'utiliser", "Ne rien faire avaler, en aucun cas : risque d'inhalation", "Appel du 15 immédiat."],
            &["Suspicion d'accident vasculaire cérébral", "Bouche déformée, faiblesse d'un bras, parole troublée, installation brutale", "Noter l'heure exacte de début, allonger, surveiller", "Ne rien donner à boire ni à avaler, ne pas donner d'aspirine", "Appel du 15 sans délai : l'heure de début conditionne la thrombolyse."],
            &["Douleur thoracique", "Douleur en étau, irradiant au bras ou à la mâchoire, sueurs, essoufflement", "Mettre au repos assis ou demi-assis, rassurer", "Ne pas faire marcher jusqu'au cabinet médical, ne pas laisser repartir en voiture au volant", "Appel du 15."],
            &["Crise d'asthme sévère", "Difficulté à parler en phrases entières, tirage, agitation, bronchodilatateur habituel inefficace", "Bronchodilatateur de courte durée d'action à répéter, position assise penchée en avant", "Ne pas allonger, ne pas laisser seul", "Appel du 15 devant toute crise qui ne cède pas ou qui empêche de parler."],
            &["Intoxication médicamenteuse", "Prise volontaire ou accidentelle, notamment paracétamol chez l'adolescent, chloroquine ou antidépresseur chez l'adulte", "Recueillir l'heure, la nature et la quantité, garder les boîtes", "Ne pas faire vomir, ne pas donner de lait ni de charbon sans avis", "Centre antipoison ou 15 ; le paracétamol est traitable si l'antidote est donné à temps, y compris chez un patient sans aucun symptôme."],
            &["Brûlure thermique limitée", "Rougeur douloureuse, phlyctène de petite taille, hors visage, mains et zones génitales", "Refroidir à l'eau tempérée 15 minutes, protéger par un pansement gras", "Ne pas percer la phlyctène, ne pas appliquer de corps gras alimentaire ni de glace", "Avis médical si la surface dépasse la paume de la main, si la zone est fonctionnelle, ou chez le nourrisson et la personne âgée."],
            &["Malaise vagal", "Pâleur, sueurs, nausée, vue qui se brouille, après une émotion, une injection ou une station debout prolongée", "Allonger, jambes surélevées, desserrer les vêtements", "Ne pas asseoir ni relever trop vite, ne pas donner à boire tant que la conscience n'est pas claire", "Surveiller la reprise ; avis médical si la perte de connaissance a été complète ou si le patient est âgé ou anticoagulé."],
        ],
    },
    ConvTable {
        short: "Grossesse",
        family: "Adaptation",
        title: "Grossesse et allaitement — ce qui se délivre au comptoir",
        reviewed: "Août 2026 — CRAT consulté, RCP à jour",
        sources: &[
            "CRAT — Centre de référence sur les agents tératogènes",
            "ANSM — anti-inflammatoires non stéroïdiens et grossesse, point d'information",
            "RCP des spécialités concernées, base de données publique des médicaments",
        ],
        columns: &["Situation", "Possible sans avis", "À éviter ou à encadrer", "Contre-indiqué", "Repère"],
        rows: &[
            &["Douleur et fièvre", "Paracétamol, à la dose efficace la plus faible et sur la durée la plus courte", "Codéine et tramadol en fin de grossesse : dépression respiratoire du nouveau-né", "Tous les anti-inflammatoires non stéroïdiens, y compris l'aspirine à dose antalgique et les formes locales", "L'interdiction des anti-inflammatoires est absolue à partir de 24 semaines d'aménorrhée, soit 6 mois révolus, et vaut aussi pour l'automédication."],
            &["Nausées et vomissements", "Mesures diététiques, gingembre, vitamine B6", "Doxylamine, métoclopramide et ondansétron selon prescription", "—", "Des vomissements incoercibles avec perte de poids relèvent d'un avis le jour même : ce n'est plus le domaine du conseil."],
            &["Reflux et brûlures", "Pansements gastriques, alginates", "Inhibiteurs de la pompe à protons sur prescription, l'oméprazole étant le mieux documenté", "—", "Mesures posturales d'abord : repas fractionnés, délai avant le coucher, surélévation de la tête du lit."],
            &["Constipation", "Laxatifs de lest et osmotiques, macrogol", "Laxatifs stimulants sur de courtes durées seulement", "Huile de ricin, anthracéniques à forte dose", "Fibres et hydratation d'abord ; la constipation est quasi constante et n'est pas une raison de laisser sans solution."],
            &["Rhume et toux", "Lavage de nez au sérum physiologique, miel après le premier trimestre", "Antitussifs et mucolytiques : intérêt faible, à éviter", "Vasoconstricteurs par voie orale ou nasale, à tout terme", "Les vasoconstricteurs sont contre-indiqués chez la femme enceinte comme chez l'enfant de moins de 15 ans : c'est un refus de délivrance fréquent."],
            &["Allergie", "Cétirizine et loratadine, les mieux documentées", "Antihistaminiques sédatifs de première génération en fin de grossesse", "—", "Un antihistaminique n'est pas une raison de suspendre un traitement de fond de l'asthme, qui doit être poursuivi."],
            &["Carences et supplémentation", "Acide folique avant la conception et jusqu'à 12 semaines, vitamine D", "Fer selon la ferritine, iode selon le contexte", "Vitamine A à forte dose et rétinoïdes", "L'acide folique se prescrit idéalement deux mois avant la conception : la question se pose à toute femme qui évoque un projet de grossesse."],
            &["Allaitement, antalgiques", "Paracétamol, ibuprofène", "Anti-inflammatoires à demi-vie longue", "Codéine et tramadol, aspirine à dose antalgique", "La codéine est contre-indiquée pendant l'allaitement : métaboliseurs rapides et dépression respiratoire du nourrisson."],
            &["Allaitement, situations courantes", "La plupart des antibiotiques usuels, dont l'amoxicilline", "Fluconazole en dose unique, antihistaminiques peu sédatifs", "Codéine, dérivés de l'ergot de seigle, iode à forte dose", "Interrompre l'allaitement est rarement la bonne réponse : vérifier la molécule au CRAT avant de le proposer."],
        ],
    },
    ConvTable {
        short: "Sujet âgé",
        family: "Adaptation",
        title: "Sujet âgé — médicaments à réévaluer et ce qu'on propose à la place",
        reviewed: "Août 2026 — critères de Laroche adaptés à la pratique française, STOPP/START v2",
        sources: &[
            "Laroche M.-L. et al. — liste des médicaments potentiellement inappropriés à la personne âgée en France",
            "STOPP/START version 2, adaptation française",
            "HAS — programme « prescription médicamenteuse chez le sujet âgé »",
        ],
        columns: &["Médicament ou classe", "Pourquoi il pose problème après 75 ans", "Ce qui se propose à la place", "Si on le garde"],
        rows: &[
            &["Benzodiazépines à demi-vie longue (diazépam, clorazépate, bromazépam)", "Chutes, fractures, confusion et troubles de mémoire, par accumulation du métabolite actif", "Molécule à demi-vie courte à dose réduite, et surtout un plan d'arrêt progressif", "Dose de moitié, réévaluation à chaque renouvellement, jamais d'association à un autre sédatif"],
            &["Anticholinergiques (oxybutynine, hydroxyzine, antihistaminiques de 1re génération)", "Confusion, rétention urinaire, constipation, sécheresse buccale, glaucome aigu — la charge anticholinergique s'additionne sur toute l'ordonnance", "Antihistaminique de 2e génération, mesures non médicamenteuses pour la vessie", "Compter la charge anticholinergique de l'ordonnance entière, pas molécule par molécule"],
            &["AINS au long cours", "Hémorragie digestive, insuffisance rénale aiguë, décompensation d'insuffisance cardiaque, hypertension", "Paracétamol en première intention, topiques locaux, avis pour un antalgique de palier adapté", "Durée la plus courte possible, IPP associé, créatinine et tension contrôlées"],
            &["Sulfamides hypoglycémiants à demi-vie longue (glibenclamide)", "Hypoglycémies prolongées et graves, souvent nocturnes, chez un patient qui mange moins", "Molécule à demi-vie courte, ou une classe sans risque hypoglycémique", "Cible d'HbA1c relevée, resucrage expliqué à l'entourage"],
            &["Digoxine au-delà de 0,125 mg/j", "Marge étroite, élimination rénale, toxicité majorée par l'hypokaliémie", "Réévaluer l'indication ; contrôler kaliémie et fonction rénale", "Digoxinémie prélevée au moins six heures après la prise, kaliémie surveillée"],
            &["Association de trois psychotropes ou plus", "Chutes, syndrome confusionnel, dépendance ; chaque molécule est justifiable, l'addition ne l'est pas", "Hiérarchiser et arrêter dans l'ordre : le plus récent et le moins indispensable d'abord", "Un seul changement à la fois, à quinze jours d'intervalle"],
            &["IPP prescrit sans indication réévaluée", "Hypomagnésémie, carence en B12, fractures, infections digestives, interactions", "Demi-dose, puis prise à la demande, puis arrêt avec décroissance", "Réévaluer l'indication à chaque bilan de médication : c'est le premier candidat à l'arrêt"],
            &["Antihypertenseurs à dose inchangée après un amaigrissement", "Hypotension orthostatique et chutes : la dose n'a pas bougé, le patient si", "Mesure de la tension debout et couchée, allègement discuté avec le prescripteur", "Tension debout à chaque renouvellement, lever en deux temps expliqué"],
            &["Ce qui manque souvent (START)", "Vitamine D chez le sujet à risque de chute, anticoagulant dans la fibrillation atriale, IEC après infarctus, laxatif sous opioïde", "Le signaler au prescripteur : l'absence de traitement utile est aussi une erreur de prescription", "Le bilan partagé de médication est le moment de le dire"],
        ],
    },
    ConvTable {
        short: "Inhalateurs",
        family: "Administration",
        title: "Dispositifs inhalés — technique, contrôle et erreurs qui font échouer le traitement",
        reviewed: "Août 2026 — notices des dispositifs commercialisés en France, GINA en vigueur",
        sources: &[
            "Notices et RCP des dispositifs (ANSM)",
            "GINA — Global Initiative for Asthma, édition en vigueur",
            "Société de pneumologie de langue française — éducation thérapeutique de l'asthmatique",
        ],
        columns: &["Dispositif", "Comment on l'arme", "Comment on inspire", "L'erreur qui fait tout rater", "Ce qu'on vérifie"],
        rows: &[
            &["Aérosol-doseur pressurisé (spray)", "Agiter, retirer le capuchon, expirer à fond hors de l'appareil", "Inspiration lente et profonde, déclenchement au tout début de l'inspiration, puis 10 secondes d'apnée", "Déclencher avant ou après le début de l'inspiration : le produit se dépose dans la bouche", "Faire une démonstration à chaque renouvellement ; proposer une chambre d'inhalation dès qu'il y a un doute"],
            &["Spray + chambre d'inhalation", "Agiter, emboîter le spray, une bouffée à la fois dans la chambre", "Cinq à dix respirations calmes dans l'embout, ou masque bien appliqué chez l'enfant", "Envoyer deux bouffées d'un coup dans la chambre : la seconde se perd sur les parois", "Chambre lavée à l'eau savonneuse une fois par semaine et séchée à l'air libre, jamais essuyée"],
            &["Turbuhaler (poudre)", "Tenir vertical, tourner la molette jusqu'au déclic, ne pas secouer après", "Inspiration rapide, forte et profonde, puis apnée", "Souffler dans l'appareil : l'humidité colle la poudre et la dose est perdue", "Le patient doit sentir peu ou pas de goût : c'est normal, ce n'est pas un signe d'échec"],
            &["Diskus / Accuhaler (poudre)", "Ouvrir, pousser le levier jusqu'au clic, garder à plat", "Inspiration rapide et profonde par l'embout, puis apnée", "Incliner l'appareil après l'armement : la dose tombe", "Le compteur de doses : au rouge, la commande de renouvellement est déjà en retard"],
            &["Respimat (brumisat)", "Tourner la base d'un demi-tour jusqu'au déclic, ouvrir le capuchon", "Inspiration lente et profonde en appuyant sur le bouton, puis apnée", "Inspirer trop vite : le brouillard va se déposer dans la gorge", "Amorçage à la première utilisation, et après plus de sept jours sans emploi"],
            &["Nébuliseur", "Verser la dose dans la cuve, brancher le compresseur", "Respiration calme au masque ou à l'embout, dix à quinze minutes", "Cuve mal rincée : le résidu de la veille modifie la dose du jour", "Rincer la cuve après chaque séance, désinfecter selon la notice"],
            &["Corticoïde inhalé, quel que soit le dispositif", "—", "—", "Ne pas se rincer la bouche : candidose et dysphonie", "Rinçage systématique, eau recrachée ; brossage des dents après la prise du matin"],
            &["Bronchodilatateur de secours", "—", "—", "Une consommation qui augmente est un asthme qui se déséquilibre, pas un traitement qui marche", "Compter les flacons délivrés dans l'année : au-delà de trois, l'ordonnance de fond se réévalue"],
        ],
    },
    ConvTable {
        short: "Antidiabétiques",
        family: "Posologies",
        title: "Antidiabétiques oraux et injectables — ce qui change au comptoir",
        reviewed: "Août 2026 — RCP à jour, recommandations SFD en vigueur",
        sources: &[
            "RCP des spécialités (ANSM)",
            "Société francophone du diabète — prise de position sur la prise en charge du diabète de type 2",
            "HAS — parcours de soins du patient diabétique de type 2",
        ],
        columns: &["Classe (exemples)", "Risque d'hypoglycémie", "Fonction rénale", "Effets à annoncer", "Ce qu'on dit à la délivrance"],
        rows: &[
            &["Metformine (Glucophage, Stagid)", "Non en monothérapie", "Dose réduite de moitié si DFG 30 à 45, contre-indiquée en dessous de 30", "Troubles digestifs à l'instauration, goût métallique, carence en B12 au long cours", "Pendant ou après le repas, titration lente ; arrêt 48 h avant un examen avec produit de contraste iodé, et pendant toute déshydratation (fièvre, diarrhée, vomissements)"],
            &["Sulfamides (gliclazide, glimépiride)", "Oui, réel et parfois prolongé", "Prudence, contre-indiqués en insuffisance rénale sévère", "Prise de poids, hypoglycémies", "Ne jamais sauter le repas qui suit la prise ; resucrage expliqué à l'entourage ; l'alcool à jeun majore l'hypoglycémie"],
            &["Inhibiteurs de la DPP-4 (sitagliptine, vildagliptine)", "Non seuls", "Dose adaptée au DFG selon la molécule", "Généralement bien tolérés ; pancréatite rare", "Douleur abdominale intense et persistante : consulter sans attendre"],
            &["Agonistes du GLP-1 (liraglutide, dulaglutide, sémaglutide)", "Non seuls", "Utilisables jusqu'à un DFG bas selon la molécule", "Nausées et satiété à l'instauration, perte de poids, troubles digestifs", "Titration lente ; injection à jour fixe pour les formes hebdomadaires ; conservation au réfrigérateur avant première utilisation"],
            &["Inhibiteurs de SGLT2 (dapagliflozine, empagliflozine)", "Non seuls", "Bénéfice rénal et cardiaque démontré ; efficacité glycémique moindre quand le DFG baisse", "Infections génitales mycosiques, polyurie, déplétion volémique", "Boire suffisamment ; hygiène intime expliquée ; arrêt en cas de jeûne, de chirurgie ou de maladie aiguë — risque d'acidocétose à glycémie normale"],
            &["Insuline basale", "Oui", "Besoins réduits quand le rein s'altère", "Prise de poids, lipodystrophies", "Rotation des sites, contrôle de la glycémie au réveil, jamais de transvasement d'un stylo à un autre"],
            &["Situations qui changent tout", "—", "—", "Fièvre, vomissements, diarrhée, jeûne, chirurgie", "Règles de jour de maladie : hydratation, contrôle plus fréquent, arrêt temporaire de la metformine et des SGLT2, l'insuline ne s'arrête jamais"],
            &["Objectif d'HbA1c", "—", "—", "—", "7 % pour la plupart ; 8 % ou plus chez le sujet âgé fragile, où l'hypoglycémie coûte plus cher que quelques dixièmes"],
        ],
    },
    ConvTable {
        short: "Collyres",
        family: "Administration",
        title: "Collyres et formes ophtalmiques — ordre, délai, conservation",
        reviewed: "Août 2026 — RCP à jour, recommandations d'usage des collyres",
        sources: &[
            "RCP des spécialités ophtalmiques (ANSM)",
            "Société française d'ophtalmologie — bon usage des collyres",
        ],
        columns: &["Situation", "Règle", "Pourquoi", "Ce qu'on ajoute"],
        rows: &[
            &["Deux collyres à la même heure", "Attendre au moins 5 minutes entre les deux", "Le cul-de-sac conjonctival tient environ 30 µL : la deuxième goutte chasse la première", "L'ordre suit la viscosité : le plus fluide d'abord"],
            &["Collyre et gel ou pommade", "Le collyre d'abord, le gel ou la pommade en dernier, 5 à 10 minutes après", "La pommade forme un film qui empêche la pénétration de ce qui suit", "La pommade le soir de préférence : elle trouble la vision"],
            &["Technique d'instillation", "Tirer la paupière inférieure, regarder vers le haut, une seule goutte dans le cul-de-sac", "Une goutte de plus déborde et ne sert à rien, sauf à augmenter le passage systémique", "Fermer l'œil sans le serrer et comprimer l'angle interne 1 à 2 minutes"],
            &["Bêtabloquant en collyre (timolol)", "Compression de l'angle interne indispensable", "Le passage systémique existe : bradycardie, bronchospasme chez l'asthmatique", "Signaler tout essoufflement ou ralentissement du pouls ; prévenir le médecin traitant du collyre"],
            &["Lentilles de contact", "Retirer avant l'instillation, remettre 15 minutes après", "Les conservateurs, le chlorure de benzalkonium en tête, s'accumulent dans les lentilles souples", "Préférer les unidoses sans conservateur en cas de port permanent"],
            &["Après ouverture", "Flacon multidose : 15 jours sauf mention contraire du RCP ; unidose : usage immédiat", "Contamination du flacon, surtout en cas de contact avec l'œil ou les cils", "Noter la date d'ouverture sur le flacon ; ne pas toucher l'embout"],
            &["Un flacon, un patient", "Jamais de partage, même entre les deux yeux d'un même patient en cas d'infection", "Transmission d'une conjonctivite d'un œil à l'autre et d'une personne à l'autre", "Un flacon par œil en cas de conjonctivite unilatérale contagieuse"],
            &["Œil rouge au comptoir", "Douleur intense, baisse de vision, port de lentilles, traumatisme ou photophobie : orientation le jour même", "Ce sont les signes qui séparent une conjonctivite banale d'une urgence ophtalmologique", "Ne pas délivrer de corticoïde local sans avis : un herpès cornéen s'aggrave sous corticoïde"],
        ],
    },
    ConvTable {
        short: "Automédication",
        family: "Au comptoir",
        title: "Automédication — ce qui se refuse au comptoir, et ce qu'on propose",
        reviewed: "Août 2026 — RCP à jour, recommandations de bon usage en vigueur",
        sources: &[
            "RCP des spécialités de médication officinale (ANSM)",
            "Cespharm — fiches de conseil à l'officine",
            "ANSM — points d'information sur le bon usage des AINS et du paracétamol",
        ],
        columns: &["Demande", "Ce qui bloque", "Ce qu'on propose", "Quand on oriente"],
        rows: &[
            &["AINS chez un patient sous anticoagulant ou antiagrégant", "Risque hémorragique digestif multiplié ; l'association n'est pas une question de dose", "Paracétamol, topique local, chaud ou froid selon la douleur", "Douleur non soulagée à 48 h, ou saignement, selles noires, vomissement sanglant : le jour même"],
            &["AINS avec IEC ou sartan et diurétique", "La triade néfaste : insuffisance rénale aiguë, d'autant plus vite qu'il fait chaud", "Paracétamol, hydratation", "Prise de poids brutale, œdèmes, urines rares : consultation"],
            &["AINS après 15 semaines d'aménorrhée", "Contre-indication absolue : fermeture du canal artériel et atteinte rénale fœtale", "Paracétamol à la dose efficace la plus faible", "Toute douleur qui résiste chez une femme enceinte : avis médical"],
            &["Paracétamol « en plus » d'un autre médicament", "Doublon involontaire : le paracétamol est dans quantité de spécialités contre le rhume et la douleur", "Vérifier l'ordonnance et l'armoire à pharmacie, une seule source à la fois", "Dose cumulée dépassée, ou insuffisance hépatique : avis immédiat"],
            &["Vasoconstricteur oral ou nasal contre le rhume", "Accidents cardiovasculaires et neurologiques rapportés ; contre-indiqué en cas d'hypertension, de coronaropathie, de glaucome, avant 15 ans et pendant la grossesse", "Lavage de nez au sérum salé, humidification, paracétamol si fièvre", "Fièvre au-delà de trois jours, douleur de sinus unilatérale, essoufflement"],
            &["Sirop antitussif chez l'enfant", "Codéine et dérivés interdits avant 12 ans ; les antitussifs n'ont pas fait la preuve de leur intérêt", "Hydratation, lavage de nez, position surélevée, patience", "Toux fébrile, gêne respiratoire, toux qui dure plus d'une semaine"],
            &["Laxatif demandé de façon répétée", "Constipation chronique non explorée, et laxatifs stimulants au long cours", "Laxatif osmotique, fibres, eau, activité physique, horaire régulier", "Sang, amaigrissement, alternance diarrhée-constipation, changement récent du transit après 50 ans"],
            &["Antidiarrhéique ralentisseur du transit", "Contre-indiqué en cas de fièvre, de glaires ou de sang : on garde le germe", "Réhydratation orale d'abord, régime adapté, argile ou racécadotril selon le cas", "Fièvre, sang, déshydratation, nourrisson ou personne âgée : consultation"],
            &["Millepertuis demandé « parce que c'est naturel »", "Inducteur enzymatique puissant : contraceptifs, anticoagulants, immunosuppresseurs, antirétroviraux, anticancéreux perdent leur efficacité", "Vérifier l'ordonnance entière avant tout conseil", "Humeur triste durable, idées noires : consultation, pas une plante"],
            &["Complément « pour la fatigue » chez un patient polymédiqué", "Interactions ignorées, doublons de vitamines, surcharge en potassium ou en vitamine D", "Reprendre le sommeil, l'alimentation et l'observance avant d'ajouter quoi que ce soit", "Fatigue nouvelle et persistante : bilan médical, pas un complément"],
        ],
    },
    ConvTable {
        short: "Antibiotiques",
        family: "Posologies",
        title: "Antibiotiques — durée, prise, et ce qui fait échouer le traitement",
        reviewed: "Août 2026 — RCP à jour, recommandations SPILF en vigueur",
        sources: &[
            "RCP des spécialités (ANSM)",
            "SPILF — recommandations de bonne pratique en antibiothérapie",
            "HAS / Assurance Maladie — antibiotiques : les bons réflexes",
        ],
        columns: &["Famille (exemples)", "Durée usuelle", "Prise", "Ce qui réduit l'efficacité", "À surveiller"],
        rows: &[
            &["Amoxicilline (Clamoxyl)", "5 à 7 jours selon l'indication, 6 jours dans l'angine", "Au moment des repas ou non, en trois prises espacées", "Prises rapprochées la nuit et sautées le jour : c'est la régularité qui maintient la concentration", "Éruption cutanée, diarrhée ; une éruption au 7e jour n'est pas toujours une allergie"],
            &["Amoxicilline-acide clavulanique (Augmentin)", "5 à 7 jours", "Au début du repas : cela réduit nettement l'intolérance digestive", "Prise à jeun, qui fait arrêter pour diarrhée", "Diarrhée fréquente, cholestase possible — première cause d'hépatite médicamenteuse en ville"],
            &["Céfpodoxime, céfuroxime (C3G, C2G orales)", "5 à 7 jours", "Au cours d'un repas", "Antiacides et IPP pris en même temps", "Éruption, diarrhée ; allergie croisée rare avec les pénicillines"],
            &["Azithromycine, clarithromycine (macrolides)", "3 jours pour l'azithromycine, 5 à 10 jours pour les autres", "Indifférente pour l'azithromycine ; au repas pour la clarithromycine", "Interactions : statines, colchicine, AOD, antiarythmiques", "Allongement du QT, diarrhée, hépatotoxicité"],
            &["Doxycycline (cyclines)", "5 à 21 jours selon l'indication", "Avec un grand verre d'eau, assis ou debout, sans s'allonger dans l'heure qui suit", "Calcium, fer, magnésium et pansements gastriques pris à moins de deux heures", "Œsophagite, photosensibilisation ; contre-indiquée avant 8 ans et pendant la grossesse"],
            &["Lévofloxacine, ciprofloxacine (fluoroquinolones)", "5 à 14 jours selon l'indication", "À distance de deux heures des cations divalents", "Laitages, fer, antiacides, et le sport pendant le traitement", "Tendinopathie, neuropathie, confusion du sujet âgé, allongement du QT — arrêt à la moindre douleur tendineuse"],
            &["Cotrimoxazole (Bactrim)", "3 à 21 jours selon l'indication", "Au cours d'un repas, avec une bonne hydratation", "Association au méthotrexate, aux AVK et aux IEC", "Éruption grave, hyperkaliémie, cytopénies, INR qui grimpe sous AVK"],
            &["Fosfomycine trométamol (Monuril)", "Dose unique", "À jeun, deux heures avant ou après un repas, de préférence au coucher après avoir vidé la vessie", "Prise pendant un repas, qui divise l'absorption", "Diarrhée transitoire ; réservée à la cystite simple de la femme"],
            &["Nitrofurantoïne (Furadantine)", "5 à 7 jours dans la cystite documentée", "Au cours d'un repas", "Insuffisance rénale : inefficace et plus toxique au-dessous de 45 mL/min", "Jamais en traitement prolongé ni en prophylaxie : toxicité pulmonaire et hépatique"],
            &["Métronidazole (Flagyl)", "5 à 10 jours, ou dose unique selon l'indication", "Au cours du repas", "Alcool pendant le traitement et les trois jours qui suivent : effet antabuse", "Goût métallique, neuropathie si le traitement se prolonge"],
            &["La durée est le traitement", "Celle de l'ordonnance, ni plus ni moins", "—", "Arrêter dès que les symptômes cèdent : c'est ce qui sélectionne les résistances et fait rechuter", "Un reste de boîte ne se garde pas pour la prochaine fois, et ne se donne à personne"],
            &["Diarrhée pendant ou après", "Jusqu'à deux mois après la fin du traitement", "—", "Un ralentisseur du transit, qui garde le germe", "Diarrhée abondante, fièvre, douleurs : suspicion de colite à Clostridioides difficile, avis le jour même"],
        ],
    },
    ConvTable {
        short: "Arrêts",
        family: "Au comptoir",
        title: "Arrêts et sevrages — ce qui ne s'arrête jamais d'un coup",
        reviewed: "Août 2026 — fiches HAS sur l'arrêt des benzodiazépines et des antidépresseurs, RCP à jour",
        sources: &[
            "HAS — arrêt des benzodiazépines et médicaments apparentés chez le patient âgé",
            "HAS — arrêt de la consommation de tabac ; mésusage de l'alcool",
            "RCP des spécialités (ANSM)",
        ],
        columns: &["Traitement", "Pourquoi l'arrêt brutal pose problème", "Comment on décroît", "Ce qu'on surveille", "Ce qui fait appeler"],
        rows: &[
            &["Bêtabloquant", "Rebond adrénergique : poussée hypertensive, angor, infarctus", "Sur une à deux semaines au moins, par paliers, sous contrôle du pouls et de la tension", "Pouls, tension, douleur thoracique", "Douleur thoracique, palpitations, tension qui s'emballe"],
            &["Corticoïde au long cours", "Insuffisance surrénale aiguë après plus de trois semaines de traitement", "Paliers décroissants sur plusieurs semaines, plus lents en dessous de 7,5 mg d'équivalent prednisone", "Fatigue, nausées, hypotension, douleurs articulaires", "Malaise, vomissements, fièvre : l'insuffisance surrénale est une urgence"],
            &["Benzodiazépine et apparentés", "Anxiété et insomnie de rebond, confusion, et convulsions après un usage prolongé", "Réduction de 10 à 25 % de la dose toutes les une à quatre semaines, plus lentement en fin de décroissance", "Sommeil, anxiété, humeur, chutes", "Convulsion, confusion, hallucinations"],
            &["Antidépresseur (ISRS, IRSNa)", "Syndrome d'arrêt : décharges électriques, vertiges, irritabilité, insomnie — souvent pris pour une rechute", "Sur quatre semaines au moins, plus lentement pour la paroxétine et la venlafaxine", "Humeur, sommeil, symptômes d'arrêt", "Idées noires, réapparition franche des symptômes dépressifs"],
            &["Opioïde", "Syndrome de sevrage : sueurs, crampes, diarrhée, agitation, insomnie", "Réduction de 10 % par semaine environ, plus lente au-delà de plusieurs mois de traitement", "Douleur, sommeil, humeur, transit", "Sevrage mal supporté, reprise de la douleur, mésusage"],
            &["Antiépileptique", "Récidive de crise, y compris chez un patient stabilisé depuis des années", "Jamais sans le neurologue : la décroissance se compte en mois", "Fréquence des crises, conduite automobile", "Toute crise inhabituelle"],
            &["Clonidine et antihypertenseurs centraux", "Poussée hypertensive de rebond, parfois sévère", "Progressivement, sur plusieurs jours, en remplaçant par une autre classe", "Tension, céphalées", "Tension très élevée, céphalées, sueurs"],
            &["Inhibiteur de la pompe à protons", "Rebond d'hypersécrétion acide : les brûlures reviennent plus fort, ce qui fait reprendre le traitement", "Demi-dose deux à quatre semaines, puis prise à la demande, puis arrêt", "Brûlures, régurgitations", "Dysphagie, amaigrissement, anémie, selles noires"],
            &["Tabac", "Le manque n'est pas dangereux, mais il fait rechuter en quelques jours", "Substituts nicotiniques au bon dosage, associés au besoin, sur deux à trois mois avec décroissance", "Envies, irritabilité, prise de poids, sommeil", "Envies irrépressibles malgré les substituts : renforcer le dosage plutôt que reprendre"],
            &["Alcool", "Le sevrage non accompagné expose au delirium tremens et aux convulsions : c'est le seul sevrage qui tue", "Jamais seul et jamais brutal en cas de dépendance : encadrement médical, hydratation, vitamine B1", "Tremblement, sueurs, anxiété, confusion", "Tremblement majeur, hallucinations, fièvre, confusion : urgence"],
        ],
    },
    ConvTable {
        short: "Dermocorticoïdes",
        family: "Équivalences",
        title: "Dermocorticoïdes — classes, sites et quantités",
        reviewed: "Août 2026 — RCP à jour ; recommandations dermatologiques en vigueur",
        sources: &[
            "RCP des spécialités, base de données publique des médicaments (ANSM)",
            "Société française de dermatologie — traitement local de la dermatite atopique",
            "HAS — prise en charge de la dermatite atopique de l'enfant",
        ],
        columns: &["Classe", "Molécules et spécialités", "Où on l'applique", "Durée usuelle", "Quantité pour un adulte", "Ce qui va de travers"],
        rows: &[
            &["I — très forte", "Clobétasol (Dermoval, Clarelux)", "Paumes, plantes, cuir chevelu, lichénifications épaisses ; jamais le visage ni les plis", "Deux à quatre semaines, puis relais par une classe plus faible", "Moins de 50 g par semaine", "Atrophie cutanée, vergetures définitives, et freinage surrénalien sur grande surface"],
            &["II — forte", "Bétaméthasone dipropionate (Diprosone), désonide 0,1 %, difluprednate (Épitopic 0,05 %)", "Corps, membres, poussée de dermatite atopique de l'adulte", "Une à trois semaines par poussée", "30 à 60 g par mois selon l'étendue", "Rebond à l'arrêt brutal : espacer plutôt qu'arrêter net"],
            &["III — modérée", "Bétaméthasone valérate 0,05 % (Betneval), désonide 0,05 % (Locapred, Tridésonit)", "Visage de l'adulte, plis, corps de l'enfant", "Une à deux semaines", "15 à 30 g par mois", "Sur le visage, dermite péri-orale et couperose après quelques semaines"],
            &["IV — faible", "Hydrocortisone", "Paupières, nourrisson, entretien court", "Quelques jours", "Quelques grammes", "Peu efficace : une classe trop faible fait échouer le traitement et prolonger l'exposition"],
            &["Règle de l'unité phalangette", "Un ruban de crème du pli de la première phalange à l'extrémité de l'index", "Couvre deux paumes de main d'adulte, soit environ 0,5 g", "—", "Visage et cou : 2,5 unités. Un bras : 3. Une jambe : 6. Tronc face avant : 7", "La sous-utilisation est plus fréquente que l'excès : un tube qui dure six mois est un tube qu'on n'applique pas"],
            &["Rythme d'application", "Une fois par jour suffit pour presque tous", "Le soir, sur peau propre", "Jusqu'à disparition des lésions, puis arrêt", "—", "Deux applications par jour n'améliorent rien et doublent l'exposition"],
            &["Émollient associé", "Tout dermocorticoïde s'accompagne d'un émollient", "Sur tout le corps, y compris les zones saines", "En continu, y compris entre les poussées", "100 à 200 g par mois chez l'enfant atopique", "L'émollient s'applique à distance du corticoïde, pas par-dessus dans la même minute"],
            &["Occlusion", "Multiplie la pénétration par dix", "Uniquement sur prescription et sur une zone limitée", "Quelques jours", "—", "Une couche sur un siège de nourrisson est une occlusion : la classe se choisit en conséquence"],
            &["Corticophobie", "La peur du corticoïde fait plus de dégâts que le corticoïde", "—", "—", "—", "Un traitement sous-dosé prolonge la poussée, donc l'exposition totale : le dire explicitement à la délivrance"],
            &["Arrêt", "Pas de décroissance de dose, mais un espacement", "—", "Un jour sur deux, puis deux fois par semaine", "—", "L'arrêt net d'une classe forte sur une dermatose étendue donne un rebond"],
        ],
    },
    ConvTable {
        short: "Conduite",
        family: "Au comptoir",
        title: "Conduite automobile — les trois niveaux du pictogramme",
        reviewed: "Août 2026 — arrêté relatif aux pictogrammes de conduite, RCP à jour",
        sources: &[
            "ANSM — médicaments et conduite automobile, les trois niveaux de risque",
            "RCP des spécialités, base de données publique des médicaments (ANSM)",
            "Code de la route, article R412-6",
        ],
        columns: &["Niveau", "Ce que dit le pictogramme", "Classes concernées", "Ce qu'on dit au comptoir", "Ce qui aggrave"],
        rows: &[
            &["Niveau 1 — jaune", "« Soyez prudent »", "Antihistaminiques de deuxième génération, certains antalgiques, antitussifs, antiémétiques", "Le risque existe mais reste faible : lire la notice, et ne pas conduire si l'on se sent somnolent", "L'alcool, la fatigue, une première prise"],
            &["Niveau 2 — orange", "« Soyez très prudent — ne pas conduire sans l'avis d'un professionnel de santé »", "Benzodiazépines à demi-vie courte, antidépresseurs, antiépileptiques, opioïdes faibles, antihistaminiques sédatifs", "L'avis est celui du médecin ou du pharmacien : la conduite se discute, elle n'est pas interdite d'office", "Le début du traitement, tout changement de dose, l'association à un autre sédatif"],
            &["Niveau 3 — rouge", "« Attention, danger : ne pas conduire — pour la reprise, demandez l'avis d'un médecin »", "Hypnotiques, benzodiazépines de longue durée, opioïdes forts, certains collyres mydriatiques, anesthésiques", "Pas de conduite du tout, et la reprise se décide par le médecin", "La conduite le lendemain matin après une prise du soir : l'effet dure au-delà du réveil"],
            &["Le lendemain matin", "Un hypnotique pris à 23 h agit encore à 7 h", "Zolpidem, zopiclone, benzodiazépines de longue demi-vie", "Le risque du somnifère n'est pas la nuit, c'est le trajet du matin", "Une prise tardive, un réveil précoce, la personne âgée"],
            &["Première délivrance", "Le risque est maximal au début et à chaque augmentation", "Toutes classes", "« Ne prenez pas le volant tant que vous ne savez pas comment vous réagissez » : la phrase se dit à la première boîte", "Le patient qui a déjà pris la molécule il y a des années et croit la connaître"],
            &["Collyres mydriatiques", "Vision floue et éblouissement pendant plusieurs heures", "Tropicamide, atropine, cyclopentolate", "Pas de conduite après un fond d'œil : prévoir un accompagnant, et le dire à la prise de rendez-vous", "Le soleil, la conduite de nuit"],
            &["Association", "Deux sédatifs ne s'additionnent pas, ils se multiplient", "Benzodiazépine + opioïde, antihistaminique + alcool", "C'est l'association qui fait l'accident, pas la molécule seule", "L'alcool, même à faible dose"],
            &["Hypoglycémie", "Un malaise au volant ne prévient pas", "Insuline, sulfamides hypoglycémiants, glinides", "Glycémie avant de prendre le volant sur un long trajet, resucrage à portée de main, pause toutes les deux heures", "Le repas sauté, le bêtabloquant qui masque les signes"],
            &["Ce que dit la loi", "Conduire sous l'effet d'un médicament n'est pas une infraction en soi", "—", "Mais l'assurance peut réduire sa garantie, et la responsabilité reste engagée en cas d'accident", "Le pictogramme sur la boîte : il vaut information donnée"],
            &["Profession", "Chauffeurs, conducteurs d'engins, travail en hauteur", "—", "Le traitement se discute avec le médecin du travail, pas seulement avec le prescripteur", "Le patient qui ne dit pas son métier"],
        ],
    },
    ConvTable {
        short: "Aliments",
        family: "Au comptoir",
        title: "Aliments, boissons et médicaments — ce qui interfère vraiment",
        reviewed: "Août 2026 — RCP à jour ; référentiel des interactions de l'ANSM",
        sources: &[
            "ANSM — thésaurus des interactions médicamenteuses",
            "RCP des spécialités, base de données publique des médicaments (ANSM)",
            "CRAT et sociétés savantes pour les recommandations diététiques associées",
        ],
        columns: &["Aliment ou boisson", "Ce qu'il fait", "Médicaments concernés", "Ce qu'on conseille", "Ce qui n'est pas vrai"],
        rows: &[
            &["Pamplemousse", "Inhibe le CYP3A4 intestinal pour 24 à 72 heures ; l'effet ne se rattrape pas en espaçant", "Statines (simvastatine, atorvastatine), inhibiteurs calciques, immunosuppresseurs, certains antiarythmiques", "On n'espace pas, on supprime : un verre suffit et l'effet dure des jours", "Que le jus d'orange fasse la même chose — il n'inhibe pas le CYP3A4"],
            &["Vitamine K des légumes verts", "Antagonise l'AVK", "Warfarine, fluindione, acénocoumarol", "Ne pas supprimer les légumes verts, mais en manger une quantité régulière d'une semaine à l'autre : c'est la variation qui déséquilibre l'INR", "Qu'il faille les interdire — un régime pauvre en vitamine K rend l'INR instable, pas stable"],
            &["Produits laitiers et calcium", "Chélation dans l'intestin", "Cyclines, fluoroquinolones, lévothyroxine, fer, bisphosphonates", "Deux heures d'écart au moins, dans un sens ou dans l'autre", "Que le lait « protège l'estomac » sous antibiotique — il annule la moitié de la dose"],
            &["Thé et café", "Les tanins chélatent le fer ; la caféine s'accumule sous certains traitements", "Fer oral, théophylline, lithium", "Fer à distance du thé et du café. Sous théophylline, ne pas changer brutalement sa consommation de café", "Que le café « fasse passer » un médicament : il ne fait qu'accélérer le transit"],
            &["Millepertuis", "Inducteur enzymatique puissant, en vente libre", "Contraception orale, AVK, antirétroviraux, immunosuppresseurs, antidépresseurs", "Demander systématiquement les compléments alimentaires : c'est la question qui manque le plus souvent", "Que « c'est une plante, donc c'est sans risque » — c'est l'inducteur le plus dangereux du rayon"],
            &["Alcool", "Additionne ses effets sédatifs ; effet antabuse avec certaines molécules", "Benzodiazépines, opioïdes, antihistaminiques sédatifs, métronidazole, disulfirame, céphalosporines", "Zéro alcool sous métronidazole et jusqu'à 48 heures après. Sous sédatif, l'effet est multiplicatif et non additif", "Qu'un verre de vin soit « négligeable » sous benzodiazépine"],
            &["Réglisse", "Effet minéralocorticoïde : hypokaliémie et hypertension", "Diurétiques, digoxine, corticoïdes, antihypertenseurs", "Chercher la réglisse devant une hypokaliémie inexpliquée — pastilles, boissons anisées sans alcool, tisanes", "Que la quantité soit anodine : quelques dizaines de grammes par jour suffisent"],
            &["Tyramine (fromages affinés, charcuterie, vin rouge)", "Crise hypertensive avec les IMAO", "IMAO non sélectifs, linézolide, procarbazine", "Liste écrite remise au patient sous IMAO : c'est un des rares cas où le régime fait partie du traitement", "Que tous les fromages soient concernés : ce sont les affinés"],
            &["Repas gras", "Augmente ou diminue l'absorption selon la molécule", "Griséofulvine et itraconazole (à prendre avec), rifampicine et lévothyroxine (à jeun)", "Lire la consigne du RCP et la répéter : « au cours du repas » et « à jeun » ne sont pas des détails", "Qu'on puisse toujours « prendre au repas pour protéger l'estomac »"],
            &["Jus de canneberge", "Effet sur l'INR discuté et probablement modeste", "AVK", "Ne pas l'interdire, mais ne pas en changer brutalement la consommation, et contrôler l'INR si l'apport est important", "Qu'il prévienne les cystites récidivantes de façon démontrée — le niveau de preuve reste faible"],
            &["Sel de régime", "Riche en potassium", "IEC, sartans, antialdostérone, suppléments potassiques", "Le déconseiller chez tout patient sous bloqueur du système rénine-angiotensine : c'est une source de potassium que personne ne compte", "Qu'un sel « sans sodium » soit sans risque"],
            &["Eau minérale", "Certaines sont très riches en sodium ou en magnésium", "Régimes hyposodés, insuffisance cardiaque, insuffisance rénale", "Lire l'étiquette : quelques eaux dépassent 1 g de sodium par litre", "Que toutes les eaux minérales se valent"],
        ],
    },
    ConvTable {
        short: "Biosimilaires",
        family: "Au comptoir",
        title: "Biosimilaires — ce qui se substitue, ce qui se trace, ce qui se remontre",
        reviewed: "Août 2026 — état des lieux ANSM sur les biosimilaires, article L.5125-23-2 du code de la santé publique et l'arrêté qui fixe les groupes substituables",
        sources: &[
            "ANSM — état des lieux sur les médicaments biosimilaires",
            "Code de la santé publique, article L.5125-23-2 et l'arrêté fixant la liste des groupes biologiques similaires substituables",
            "EMA — Biosimilars in the EU, information guide for healthcare professionals",
        ],
        columns: &["Question", "La réponse", "Ce que fait le pharmacien", "Le piège"],
        rows: &[
            &["Un biosimilaire est-il un générique ?", "Non. Un générique est la même molécule chimique ; un biosimilaire est une protéine produite par une lignée cellulaire vivante, dont la comparabilité au médicament de référence est établie par un dossier de qualité, de pharmacocinétique et de clinique", "Le dire au patient dans ces termes : ce n'est pas une copie approximative, c'est une équivalence démontrée", "Employer le mot « générique » devant le patient : cela installe une méfiance qu'il faudra défaire ensuite"],
            &["Puis-je le substituer au comptoir ?", "Seulement pour les groupes biologiques similaires inscrits par arrêté. La liste est courte et se vérifie : elle a commencé par le filgrastim et le pegfilgrastim", "Vérifier la liste en vigueur avant toute substitution ; hors de ces groupes, la substitution en officine n'est pas permise", "Supposer que ce qui vaut pour un groupe vaut pour tous : chaque groupe est inscrit un par un"],
            &["Le prescripteur peut-il l'interdire ?", "Oui : la mention « non substituable » portée à la main sur l'ordonnance, avec sa justification, ferme la substitution pour cette ligne", "Respecter la mention et délivrer la spécialité prescrite ; ne pas la contourner", "Une mention pré-imprimée ou générale ne vaut pas : elle est manuscrite et propre à la ligne"],
            &["Que dois-je faire si je substitue ?", "Informer le patient, inscrire la spécialité délivrée sur l'ordonnance, et informer le prescripteur", "Les trois, à chaque fois — l'information du prescripteur n'est pas facultative", "Substituer sans le dire au patient : il découvrira une autre boîte et un autre stylo tout seul"],
            &["Et pour la suite du traitement ?", "La continuité prime : on délivre la même spécialité pour toute la durée du traitement", "Noter la spécialité choisie sur la fiche patient, pour que la délivrance suivante soit la même", "Changer de marque au gré du stock : c'est le changement répété, plus que le biosimilaire, qui inquiète le patient et brouille la traçabilité"],
            &["Comment se trace un biologique ?", "Par le nom de marque **et** le numéro de lot, jamais par la DCI seule", "Noter les deux à chaque délivrance : c'est ce qui permet de rattacher un effet indésirable au bon produit", "Écrire « adalimumab » sur la fiche : trois spécialités le sont, et une déclaration de pharmacovigilance sans marque ne sert à rien"],
            &["Le dispositif d'injection change-t-il ?", "Oui, presque toujours : les stylos, les seringues et leur gestuelle diffèrent d'une marque à l'autre, même à dose identique", "Remontrer la technique à chaque changement de spécialité, y compris à un patient qui s'injecte depuis des années", "Croire que « c'est la même molécule, donc le même geste » : la mauvaise technique est la première cause de dose non reçue"],
            &["Qui peut faire un switch ?", "Le prescripteur, à tout moment, dans les deux sens et entre biosimilaires, avec information du patient. C'est l'interchangeabilité, et elle est distincte de la substitution en officine", "Distinguer les deux mots devant le patient : le médecin interchange, le pharmacien substitue dans les groupes prévus", "Confondre les deux et croire qu'un switch décidé par le médecin autorise la substitution au comptoir"],
            &["Le patient a-t-il un mot à dire ?", "Oui : il est informé et son opposition se respecte", "Prendre le temps d'expliquer plutôt que d'imposer ; un refus se note sur la fiche", "Passer en force pour une différence de prix : le traitement abandonné coûte plus cher que l'écart"],
            &["L'efficacité est-elle moindre ?", "Non : la comparabilité clinique fait partie de l'autorisation, et le recul est maintenant de plus de quinze ans en Europe", "Répondre sur ce terrain, avec ce chiffre : c'est la question que les patients posent réellement", "Répondre « c'est pareil » sans expliquer : la réponse courte se lit comme un aveu"],
            &["Y a-t-il plus d'effets indésirables ?", "Non, y compris l'immunogénicité, qui est le point spécifiquement évalué pour ces produits", "Déclarer tout effet indésirable avec la marque et le lot ; c'est ce suivi qui alimente la connaissance", "Attribuer au biosimilaire un effet apparu au même moment qu'un autre changement de traitement"],
            &["Et la conservation ?", "Chaîne du froid entre 2 et 8 °C, jamais de congélation, et une durée hors réfrigérateur propre à chaque spécialité", "Donner un sac isotherme, dire la durée exacte pour cette marque-là, et rappeler qu'un produit congelé se jette", "Transposer la durée hors froid d'une marque à l'autre : elles ne sont pas les mêmes"],
        ],
    },
    ConvTable {
        short: "Canicule",
        family: "Au comptoir",
        title: "Chaleur et traitements — ce que la canicule fait à une ordonnance",
        reviewed: "Août 2026 — mise au point ANSM « Bon usage des médicaments en cas de vague de chaleur » et recommandations Santé publique France",
        sources: &[
            "ANSM — bon usage des médicaments en cas de vague de chaleur",
            "Santé publique France — plan national canicule, recommandations sanitaires",
            "HAS — repérage et prise en charge de la déshydratation du sujet âgé",
        ],
        columns: &["Traitement", "Ce que la chaleur en fait", "Ce qu'on surveille", "Ce qu'on ne fait pas"],
        rows: &[
            &["Diurétiques (thiazidiques, anse, antialdostérone)", "Ils font perdre de l'eau et du sel au moment où la sueur en fait perdre aussi : déshydratation, hyponatrémie, hypokaliémie", "Le poids tous les jours — deux kilos perdus en trois jours, c'est de l'eau — la soif, les urines rares et foncées, la confusion", "Ne jamais conseiller d'arrêter ni de doubler : c'est le prescripteur qui suspend un diurétique, souvent pour quelques jours seulement"],
            &["IEC et ARA II", "Ils lèvent l'autorégulation rénale : sur un rein déjà déshydraté, la filtration s'effondre", "Créatinine et kaliémie si la chaleur dure, et tout ce qui fait perdre de l'eau — diarrhée, vomissements, fièvre", "Ne pas ajouter d'AINS : diurétique, bloqueur du système rénine-angiotensine et AINS ensemble, c'est la triade classique de l'insuffisance rénale aiguë"],
            &["AINS, y compris ceux vendus sans ordonnance", "Ils réduisent le débit sanguin rénal exactement quand il faudrait le préserver", "La quantité d'urine, l'apparition d'œdèmes, la prise de poids brutale", "Ne pas délivrer un AINS de conseil à un patient âgé sous diurétique ou IEC pendant une vague de chaleur : proposer le paracétamol"],
            &["Lithium", "La perte de sodium fait remonter la lithémie : le surdosage arrive sans changement de dose", "Tremblements qui s'aggravent, nausées, diarrhée, somnolence, marche instable — et une lithémie si la chaleur dure", "Ne pas conseiller un régime sans sel ni une eau très minéralisée sans avis : c'est l'équilibre sodé qui tient la lithémie"],
            &["Anticholinergiques : antihistaminiques sédatifs, antiparkinsoniens, anticholinergiques vésicaux, tricycliques, néfopam", "Ils bloquent la sudation, qui est le seul moyen qu'a le corps de se refroidir", "Peau sèche et chaude sans transpiration, température qui monte, confusion : c'est le coup de chaleur, et il est mortel", "Ne pas ajouter un antihistaminique sédatif de conseil pour dormir chez une personne âgée traitée : les effets s'additionnent"],
            &["Neuroleptiques et antipsychotiques", "Ils dérèglent la thermorégulation centrale et gênent la sudation", "Température, vigilance, rigidité musculaire — un syndrome malin se discute devant une fièvre inexpliquée", "Ne pas suspendre sur décision de comptoir : l'arrêt brutal a ses propres accidents"],
            &["Metformine", "Une déshydratation avec insuffisance rénale expose à l'acidose lactique", "Crampes, douleurs abdominales, respiration rapide, malaise — et toute diarrhée ou vomissement qui dure", "Ne pas laisser poursuivre pendant une gastro-entérite fébrile : c'est la règle des jours de maladie, elle vaut aussi sous 38 °C dehors"],
            &["Patchs : fentanyl, trinitrine, rivastigmine, nicotine, buprénorphine", "La chaleur augmente le débit du dispositif : un patch chauffé délivre davantage, et le surdosage passe pour une fatigue", "Somnolence inhabituelle, confusion, nausées, dépression respiratoire pour les opioïdes", "Ne pas exposer le patch au soleil, à une bouillotte, à un bain chaud ni à une couverture chauffante — et ne jamais coller un patch sur une peau moite"],
            &["Insulines et analogues du GLP-1", "Au-delà de 30 °C la stabilité n'est plus garantie ; un stylo laissé dans une voiture est perdu", "L'aspect du produit, les glycémies qui montent sans raison — premier signe d'une insuline dégradée", "Ne pas remettre au réfrigérateur un stylo en cours d'utilisation ni congeler : une insuline congelée se jette, même redevenue liquide"],
            &["Médicaments de la chaîne du froid en général", "Le transport d'été est le maillon faible, pas le réfrigérateur", "La durée hors froid propre à chaque spécialité, qui n'est pas la même d'une marque à l'autre", "Ne pas donner un sac isotherme sans dire combien de temps il tient, ni ranger un vaccin dans la porte du réfrigérateur"],
            &["Antiépileptiques inhibant l'anhydrase carbonique : topiramate, zonisamide", "Ils réduisent la sudation, surtout chez l'enfant, et exposent à l'hyperthermie", "Enfant rouge, chaud et sec qui ne transpire pas pendant l'effort ou la chaleur", "Ne pas laisser faire du sport aux heures chaudes sans en avoir parlé"],
            &["Le conseil qui vaut pour tous", "Boire régulièrement sans attendre la soif, garder la pièce fermée le jour et aérée la nuit, mouiller la peau, éviter l'effort de 11 h à 21 h", "Chez une personne âgée seule : passer un appel par jour est ce qui sauve le plus", "Ne pas faire boire des quantités massives d'eau pure à quelqu'un sous diurétique : cela fabrique l'hyponatrémie qu'on veut éviter"],
        ],
    },
    ConvTable {
        short: "Soleil",
        family: "Au comptoir",
        title: "Photosensibilisation — les traitements qui font brûler la peau",
        reviewed: "Août 2026 — points d'information ANSM, dont la restriction du kétoprofène topique, et les RCP des spécialités citées",
        sources: &[
            "ANSM — photosensibilité médicamenteuse, points d'information",
            "ANSM — kétoprofène en gel : rappel des conditions d'utilisation",
            "Centre régional de pharmacovigilance — fiches de photosensibilisation médicamenteuse",
        ],
        columns: &["Médicament ou classe", "Type de réaction", "Ce qu'on dit au comptoir", "Le piège"],
        rows: &[
            &["Cyclines, doxycycline en tête", "Phototoxique et dose-dépendante : un coup de soleil démesuré pour une exposition banale, en quelques heures", "Chapeau, manches longues, indice 50 sur ce qui reste découvert, et pas de séance de bronzage pendant la cure", "La prescription d'été pour l'acné ou pour un voyage en zone impaludée est justement celle qu'on donne au moment où le soleil est le plus fort"],
            &["Fluoroquinolones", "Phototoxique, avec un érythème parfois bulleux", "Éviter le soleil pendant le traitement et les jours qui suivent", "La réaction peut survenir sur une exposition à travers une vitre : la voiture ne protège pas des UVA"],
            &["Sulfamides antibactériens, cotrimoxazole", "Photoallergique, donc retardée et non proportionnelle à la dose", "Protection complète, et signaler toute éruption qui déborde les zones exposées", "Une éruption sous cotrimoxazole n'est pas toujours un coup de soleil : c'est aussi le début possible d'une toxidermie grave"],
            &["Kétoprofène en gel", "Photoallergique, parfois sévère et étendue", "Se laver les mains après chaque application, couvrir la zone traitée par un vêtement pendant tout le traitement et les deux semaines qui suivent", "Deux semaines après l'arrêt, la zone reste sensible : c'est la seule règle de cette table qui continue après la dernière application"],
            &["Amiodarone", "Phototoxique, puis pigmentation ardoisée du visage et des mains", "Protection stricte et permanente, y compris l'hiver, pendant tout le traitement", "La pigmentation, une fois installée, met des mois à des années à disparaître — et parfois ne disparaît pas"],
            &["Diurétiques thiazidiques, hydrochlorothiazide", "Photosensibilité, et une exposition cumulée associée à un sur-risque de cancer cutané", "Protection au quotidien, surveillance des grains de beauté et des lésions qui ne cicatrisent pas", "C'est un traitement de fond pris pendant des années : le risque n'est pas celui d'une semaine de vacances mais celui d'une décennie"],
            &["Méthotrexate", "Réactivation d'un coup de soleil ancien sur la zone déjà brûlée, parfois plusieurs jours après la prise", "Éviter le soleil dans les jours qui entourent la prise hebdomadaire", "Le patient ne fait pas le lien : la brûlure réapparaît sans nouvelle exposition"],
            &["Rétinoïdes oraux et topiques, isotrétinoïne, adapalène", "Fragilité cutanée et sensibilité accrue, sans phototoxicité vraie", "Crème solaire tous les jours, pas de UV artificiels, et une lèvre protégée par un stick", "Le patient acnéique croit que le soleil améliore son acné : il la masque quelques semaines, puis elle revient plus forte"],
            &["Millepertuis", "Phototoxique à forte dose, surtout sur peau claire", "Prévenir, parce que le patient ne le compte pas comme un médicament", "C'est aussi un inducteur enzymatique majeur : la photosensibilité est le moindre de ses problèmes"],
            &["Voriconazole", "Phototoxicité sévère et, en traitement prolongé, risque de carcinome cutané", "Protection maximale, et surveillance dermatologique organisée par le prescripteur", "Le traitement est souvent long : la règle n'est pas saisonnière"],
            &["Phénothiazines, dont l'alimémazine et la prométhazine", "Phototoxique, avec une pigmentation possible", "Protection, et attention à ces molécules données le soir pour dormir ou pour la toux", "Elles sont aussi anticholinergiques : en cas de forte chaleur, la photosensibilité n'est pas le seul problème"],
            &["Essences de bergamote et de citrus, parfums et huiles essentielles", "Phototoxicité par les furocoumarines : coulées pigmentées en traînée", "Ne pas appliquer sur une peau qui va voir le soleil, et rincer avant l'exposition", "Le patient cherche la cause dans son ordonnance alors qu'elle est dans son eau de toilette"],
        ],
    },
    ConvTable {
        short: "Pilulier",
        family: "Administration",
        title: "Pilulier — ce qui ne s'y met pas, et pourquoi",
        reviewed: "Août 2026 — RCP des spécialités citées et recommandations de la Société française de pharmacie clinique sur la préparation des doses à administrer",
        sources: &[
            "Base de données publique des médicaments (ANSM) — RCP des spécialités citées",
            "Société française de pharmacie clinique — préparation des doses à administrer",
            "Ordre national des pharmaciens — recommandations sur la PDA en officine",
        ],
        columns: &["Forme ou médicament", "Pourquoi il n'y va pas", "Ce qu'on fait à la place"],
        rows: &[
            &["Pradaxa (dabigatran)", "Les gélules sont hygroscopiques : hors de leur plaquette ou de leur flacon d'origine, elles se dégradent, et un flacon entamé ne se garde que quatre mois", "Laisser dans la plaquette et découper la plaquette si le patient a besoin d'un repère de jour"],
            &["Comprimés effervescents et sachets", "L'humidité les fait réagir avant l'heure ; ils gonflent, collent et perdent leur dose", "Les laisser dans leur tube ou leur sachet, et les compter à part sur le plan de prise"],
            &["Lyophilisats oraux et orodispersibles", "Ils fondent à la moindre humidité et se brisent au moindre appui", "Les garder en plaquette : ils sont conçus pour être poussés au dernier moment"],
            &["Comprimés sublinguaux de trinitrine", "Le principe actif est volatil : hors du flacon d'origine, la dose part avant la crise", "Flacon d'origine, bien fermé, gardé sur soi — et vérifier la date, parce qu'un flacon ouvert ne se garde pas indéfiniment"],
            &["Capsules molles huileuses, vitamine D en ampoule", "Elles collent, se percent et souillent les autres cases", "Les délivrer à part, et rappeler la date de la prise mensuelle ou trimestrielle plutôt que de la déposer dans une case"],
            &["Cytotoxiques oraux et immunosuppresseurs à manipuler avec précaution", "Le déconditionnement expose le préparateur et l'entourage à la poussière du comprimé", "Ne pas déconditionner ; si le patient a besoin d'aide, organiser le plan de prise autour de la plaquette"],
            &["Médicaments photosensibles restant en plaquette opaque", "L'aluminium de la plaquette est ce qui les protège ; une case transparente ne protège rien", "Garder la plaquette et repérer les jours dessus"],
            &["Traitements « si besoin »", "Un pilulier dit quand prendre : y déposer un antalgique à la demande le transforme en prise systématique", "Les sortir du pilulier et les écrire sur le plan de prise avec leur condition et leur intervalle minimal"],
            &["Cures courtes d'antibiotiques", "Elles ne suivent pas le rythme hebdomadaire du pilulier et se terminent en milieu de semaine", "Les délivrer à part avec la date de fin écrite sur la boîte"],
            &["Formes à libération prolongée non sécables", "Couper pour faire entrer une demi-dose dans une case détruit la libération prolongée", "Vérifier la sécabilité — une barre gravée n'est pas toujours une barre de sécabilité — et demander un autre dosage au prescripteur"],
            &["Ce qui change en cours de semaine", "Un pilulier préparé le lundi porte l'ordonnance du lundi : une dose modifiée le mercredi reste fausse dans les cases suivantes", "Refaire le pilulier à chaque changement, et noter la date de préparation dessus"],
            &["Le pilulier lui-même", "Un pilulier gardé dans une salle de bain prend l'humidité, et un pilulier au soleil prend la chaleur", "Un endroit sec, à l'abri de la lumière, hors de portée des enfants — et un pilulier par personne dans un foyer qui en compte deux"],
        ],
    },
    ConvTable {
        short: "Foie",
        family: "Adaptation",
        title: "Insuffisance hépatique — ce qui change, ce qui s'évite",
        reviewed: "Août 2026 — RCP des spécialités citées, recommandations EASL sur l'encéphalopathie hépatique, et thésaurus des interactions de l'ANSM",
        sources: &[
            "RCP des spécialités (ANSM / base de données publique des médicaments)",
            "EASL — Clinical Practice Guidelines on the management of hepatic encephalopathy",
            "HAS — Prise en charge de la cirrhose",
        ],
        columns: &["Situation ou classe", "Ce que le foie change", "Conduite au comptoir", "Ce qu'on ne fait pas"],
        rows: &[
            &["Le score de Child-Pugh", "Il classe la cirrhose en A, B ou C sur cinq éléments : bilirubine, albumine, TP, ascite, encéphalopathie. Beaucoup de RCP ne parlent qu'en Child", "Demander au patient s'il connaît son stade : la plupart des contre-indications commencent à Child B", "Lire « insuffisance hépatique » comme un tout : entre un Child A et un Child C il y a l'écart entre une prudence et une contre-indication"],
            &["Paracétamol", "Métabolisé par le foie, mais c'est l'antalgique qui reste le plus sûr en cirrhose : c'est la dose qui change, pas la molécule", "3 g par jour au maximum, et 2 g en cas de dénutrition, d'alcoolisation active ou de faible poids. Espacer les prises de 6 h", "Le remplacer par un AINS, qui est bien plus dangereux ici : le paracétamol reste le premier choix"],
            &["AINS", "Ils précipitent le syndrome hépato-rénal et l'hémorragie digestive sur varices, et ils font la rétention hydrosodée qui remplit l'ascite", "À éviter en cirrhose, quelle qu'en soit la voie — gel compris chez le patient décompensé", "Délivrer un AINS de conseil sans demander : c'est le refus le plus fréquent et le plus justifié de ce tableau"],
            &["Benzodiazépines", "La demi-vie s'allonge et la sédation démasque ou aggrave l'encéphalopathie", "S'il en faut une, ce sont celles qui ne passent pas par l'oxydation hépatique : oxazépam, lorazépam, témazépam", "Diazépam, bromazépam, clobazam — longue demi-vie et métabolites actifs qui s'accumulent"],
            &["Statines", "Elles sont métabolisées par le foie, mais une cytolyse modérée stable n'est pas une contre-indication", "Contre-indiquées en hépatopathie évolutive ou devant des transaminases élevées inexpliquées et persistantes ; sinon poursuivies avec un contrôle", "Arrêter une statine sur une simple stéatose : la NASH est une indication, pas une contre-indication"],
            &["Anticoagulants oraux directs", "Le TP bas de la cirrhose ne protège de rien : ces patients saignent *et* thrombosent", "Child B : prudence, rivaroxaban contre-indiqué. Child C : toute la classe est contre-indiquée", "Lire un INR spontanément allongé comme une anticoagulation déjà en place"],
            &["Métronidazole, macrolides, antifongiques azolés", "Élimination hépatique, et inhibition enzymatique qui remonte tout le reste de l'ordonnance", "Doses réduites et durées courtes ; vérifier l'ordonnance entière avant de délivrer", "Ajouter un inhibiteur puissant à une ordonnance de cirrhotique sans relire les autres lignes"],
            &["Ce qui contient de l'alcool", "Sirops, solutions buvables, formes en ampoule et certains bains de bouche en contiennent", "Lire la composition et proposer une forme sans alcool", "Considérer que l'alcool d'un excipient est négligeable chez quelqu'un en sevrage"],
            &["Paracétamol codéiné et opioïdes", "La codéine et le tramadol demandent le foie pour devenir actifs, et la constipation précipite l'encéphalopathie", "Éviter la codéine ; si un opioïde est nécessaire, dose réduite, intervalle allongé, laxatif systématique", "Laisser partir un opioïde sans laxatif : la constipation est ici un facteur déclenchant, pas un inconfort"],
            &["Signes qui font consulter", "Ictère, urines foncées, selles décolorées, prurit, somnolence inhabituelle, confusion, inversion du rythme veille-sommeil", "Somnolence et confusion nouvelles chez un cirrhotique sont une encéphalopathie jusqu'à preuve du contraire : appeler le médecin le jour même", "Attendre le rendez-vous programmé : l'encéphalopathie se traite d'autant mieux qu'on la prend tôt"],
        ],
    },
    ConvTable {
        short: "Tabac",
        family: "Au comptoir",
        title: "Sevrage tabagique — substituts, doses et ce qui fait échouer",
        reviewed: "Août 2026 — RCP des substituts nicotiniques et recommandation HAS « Arrêt de la consommation de tabac »",
        sources: &[
            "HAS — Arrêt de la consommation de tabac : du dépistage individuel au maintien de l'abstinence",
            "RCP des substituts nicotiniques (ANSM)",
            "Assurance Maladie — prise en charge des substituts nicotiniques sur prescription",
        ],
        columns: &["Ce qu'on regarde", "Repère", "Ce qu'on propose", "Remarque de comptoir"],
        rows: &[
            &["Combien de cigarettes par jour", "Une cigarette apporte environ 1 mg de nicotine absorbée", "10 à 15 cigarettes : patch 14 mg/24 h. 15 à 25 : patch 21 mg/24 h. Au-delà de 25 : 21 mg plus une forme orale, ou deux patchs sur avis", "Le sous-dosage est la première cause d'échec, et il se voit : le patient continue de fumer sous patch"],
            &["Le délai de la première cigarette", "Fumer dans les cinq minutes du réveil signe une dépendance forte", "Patch 24 h plutôt que 16 h, et une forme orale prête au réveil", "C'est la question la plus utile du test de Fagerström, et elle tient en une phrase"],
            &["Le patch", "Libération continue ; il couvre le fond de la dépendance, pas l'envie soudaine", "Sur une peau sèche, glabre, propre, sans crème ; changer de site chaque jour ; le retirer après 24 h (ou au coucher pour le 16 h)", "Un patch qui décolle en douchant tient mieux posé après la douche, pas avant"],
            &["Les formes orales", "Gomme, pastille, comprimé sublingual, spray : pic en quelques minutes, pour l'envie qui monte", "Une prise dès l'envie, jusqu'à 8 à 12 par jour selon la forme. La gomme se mâche lentement puis se laisse dans la joue", "Mâcher la gomme comme un chewing-gum donne des brûlures d'estomac et un hoquet, et n'apporte presque rien"],
            &["Boissons acides", "Café, jus de fruit et sodas abaissent le pH buccal et bloquent l'absorption de la nicotine orale", "Rien d'acide dans les quinze minutes qui précèdent une gomme ou une pastille", "C'est la cause d'échec la plus fréquente des formes orales, et personne ne la dit spontanément"],
            &["Durée", "Le sevrage se joue sur des mois, pas sur des semaines", "Au moins 3 mois à dose pleine, puis décroissance progressive sur 1 à 3 mois selon la tolérance", "Arrêter les substituts à un mois parce que « ça va » est ce qui ramène le patient à la cigarette"],
            &["Effets attendus", "Insomnie et rêves marquants sous patch 24 h ; irritation locale ; hoquet et brûlures avec les formes orales", "Insomnie : passer au patch 16 h ou retirer le patch au coucher. Irritation : changer de site, laisser la peau respirer", "Une irritation locale n'est pas une allergie : elle se règle en tournant les sites"],
            &["Fumer sous patch", "Ce n'est pas dangereux au sens d'un surdosage grave, mais cela signe un dosage insuffisant", "Ne pas retirer le patch : revoir la dose à la hausse et ajouter une forme orale", "La phrase « il ne faut surtout pas fumer avec le patch » fait retirer le patch et rechuter : ce qu'il faut, c'est ajuster"],
            &["Grossesse", "Le tabac est plus dangereux que le substitut", "Substituts possibles, formes orales à préférer pour ajuster la dose, avis du prescripteur", "Refuser tout substitut à une femme enceinte qui fume la laisse fumer"],
            &["Interactions à l'arrêt", "Ce sont les goudrons, et non la nicotine, qui induisent le CYP1A2 : à l'arrêt, l'induction disparaît en une à deux semaines", "Signaler l'arrêt au prescripteur pour la clozapine, l'olanzapine, la théophylline, la caféine et la warfarine : les concentrations montent", "Traiter l'arrêt du tabac comme sans conséquence sur l'ordonnance — c'est une des rares interactions qui apparaissent en *arrêtant* quelque chose"],
            &["Prise en charge", "Les substituts inscrits sur la liste sont remboursés sur prescription, dans les conditions du droit commun", "Le pharmacien peut les prescrire ; la prescription se fait sur ordonnance et le patient présente sa carte Vitale", "Vendre hors prescription ce qui pouvait être prescrit fait payer le patient pour rien"],
        ],
    },
    ConvTable {
        short: "Sonde",
        family: "Administration",
        title: "Sonde et nutrition entérale — donner un médicament sans la bouche",
        reviewed: "Août 2026 — recommandations de la Société française de pharmacie clinique sur l'administration par sonde et RCP des spécialités citées",
        sources: &[
            "SFPC — Administration des médicaments par sonde de nutrition entérale",
            "RCP des spécialités (ANSM)",
            "ANSM — Bon usage : formes orales à ne pas écraser",
        ],
        columns: &["Point", "Ce qui se passe", "Ce qu'on fait"],
        rows: &[
            &["La règle qui prime toutes les autres", "Une sonde bouchée est une sonde à changer, et un changement de sonde est un geste et une attente", "Rincer avant, entre chaque médicament et après : 10 à 30 mL d'eau selon le calibre. C'est le rinçage entre deux médicaments qui manque le plus souvent"],
            &["Ce qui ne s'écrase jamais", "Libération prolongée, formes gastro-résistantes, comprimés enrobés d'un principe irritant, cytotoxiques", "Demander une autre forme au prescripteur : solution buvable, forme orodispersible dissoute, ou une autre molécule. La table « Broyage » dit laquelle"],
            &["Les gélules à microgranules", "Les granules gastro-résistants passent si le calibre le permet, mais ils bouchent les sondes fines", "Ouvrir, disperser dans de l'eau sans écraser les granules, administrer immédiatement et rincer largement. En sonde fine, chercher une autre forme"],
            &["Une forme par prise, jamais un mélange", "Deux médicaments broyés ensemble réagissent, précipitent et bouchent — et l'un peut annuler l'autre", "Un médicament, de l'eau, un rinçage, le suivant. Cela prend plus de temps et c'est ce qui évite le changement de sonde"],
            &["Lévothyroxine", "L'absorption s'effondre au contact de la nutrition entérale, et le patient est déséquilibré sans qu'on comprenne pourquoi", "Suspendre la nutrition 30 à 60 min avant et après, rincer, administrer, rincer. Contrôler la TSH après tout changement de mode d'administration"],
            &["Phénytoïne", "Elle se lie aux protéines de la nutrition : les concentrations chutent de moitié", "Arrêter la nutrition 1 à 2 h avant et après, rincer abondamment, et faire contrôler le dosage plasmatique après le changement"],
            &["Fluoroquinolones, tétracyclines, fer", "Le calcium et le magnésium de la nutrition les chélatent", "Suspendre la nutrition 1 h avant et 2 h après, et rincer entre les deux"],
            &["Oméprazole et IPP", "Les granules gastro-résistants ne doivent pas être écrasés, mais ils se déprotègent en milieu acide", "Disperser les granules dans une solution de bicarbonate à 8,4 % (le codex en donne la formule), ou utiliser une forme orodispersible dissoute dans de l'eau"],
            &["Les sirops très visqueux et les solutions hyperosmolaires", "Elles collent aux parois et provoquent diarrhées et crampes", "Diluer dans 10 à 30 mL d'eau avant d'administrer, et rincer après"],
            &["Le site de la sonde compte", "Une sonde jéjunale saute l'estomac : les formes gastro-résistantes n'ont plus de raison d'être, et certaines absorptions changent", "Demander où la sonde se termine avant de conseiller : gastrique et jéjunal ne suivent pas les mêmes règles"],
            &["Ce qu'on écrit", "L'entourage refait à la maison ce qu'on a montré une fois", "Écrire la séquence sur le plan de prise : rincer, donner, rincer — et le nom des médicaments à donner à distance de la nutrition"],
        ],
    },
    ConvTable {
        short: "Voyage",
        family: "Au comptoir",
        title: "Voyage — l'ordonnance, la trousse et le décalage horaire",
        reviewed: "Août 2026 — BEH « Recommandations sanitaires pour les voyageurs » de l'année en cours, et RCP des spécialités citées",
        sources: &[
            "Santé publique France — BEH, Recommandations sanitaires pour les voyageurs",
            "RCP des spécialités (ANSM)",
            "Ministère chargé de la santé — transport de médicaments à l'étranger",
        ],
        columns: &["Question", "Ce qu'il faut savoir", "Ce qu'on prépare"],
        rows: &[
            &["L'ordonnance", "Un nom de marque français ne veut rien dire ailleurs, et certains pays contrôlent les stupéfiants et les psychotropes à l'entrée", "Une ordonnance en DCI, et pour les stupéfiants une attestation de transport délivrée par l'ARS. Se renseigner sur le pays : ce qui est banal ici est interdit là-bas"],
            &["La quantité", "Une pharmacie n'est pas toujours trouvable, et un bagage se perd", "La durée du séjour plus une semaine, répartie entre le bagage cabine et le bagage en soute — jamais tout dans le même"],
            &["Les traitements en cabine", "La soute descend sous zéro : l'insuline gèle, et une insuline gelée est jetée", "Insuline, stylos, analogues du GLP-1 et biothérapies voyagent en cabine, dans leur boîte, avec l'ordonnance à portée de main pour le contrôle"],
            &["La chaîne du froid", "Une insuline non entamée se garde entre 2 et 8 °C ; entamée, elle tient à température ambiante le nombre de jours indiqué par son RCP", "Pochette isotherme sans contact direct avec le pain de glace. À l'arrivée, vérifier l'aspect : une insuline claire devenue trouble, ou l'inverse, ne s'injecte pas"],
            &["Le décalage horaire — thyroïde, statine, IPP", "Une prise quotidienne tolère quelques heures de décalage", "Passer à l'heure locale dès le premier jour, sans rattrapage"],
            &["Le décalage horaire — contraception", "C'est l'intervalle entre deux prises qui compte, et il ne doit pas s'allonger au-delà du délai toléré", "Garder l'heure française sur le téléphone pendant le voyage, ou décaler d'une heure par jour. Vers l'ouest la journée s'allonge : c'est le sens où l'oubli se produit"],
            &["Le décalage horaire — insuline et antidiabétiques", "Vers l'ouest la journée s'allonge et il faut plus de basale ; vers l'est elle raccourcit et il en faut moins", "Décalage de plus de 3 h : schéma préparé par le diabétologue avant le départ. Contrôles rapprochés pendant 48 h, et du sucre à portée de main"],
            &["Le décalage horaire — AVK et AOD", "Une prise sautée ou doublée compte davantage ici qu'ailleurs", "Ne jamais doubler. Garder l'heure de référence habituelle et reprendre le rythme local dès le lendemain"],
            &["La trousse de base", "Ce qui manque le plus est ce qui ne s'achète pas facilement sur place", "Antalgique, antidiarrhéique et soluté de réhydratation, antiseptique et pansements, répulsif anti-moustiques, crème solaire, thermomètre, et le traitement habituel"],
            &["La diarrhée du voyageur", "Elle est bénigne dans l'immense majorité des cas, et c'est la déshydratation qui fait le danger — surtout chez l'enfant et la personne âgée", "SRO avant tout. Lopéramide seulement en l'absence de fièvre et de sang dans les selles. Fièvre, sang ou durée au-delà de trois jours : consulter sur place"],
            &["Le soleil et les traitements", "Cyclines, AINS, certains diurétiques et l'amiodarone font brûler la peau à des doses de soleil ordinaires", "Relire l'ordonnance contre la table « Soleil » avant un départ vers le sud, et prévoir l'indice 50 plutôt que le conseil habituel"],
            &["Fièvre au retour", "Une fièvre dans les mois qui suivent un séjour en zone impaludée est un paludisme jusqu'à preuve du contraire", "Consulter en urgence en le disant : « je reviens de… ». C'est la phrase qui change la prise en charge"],
        ],
    },
    // --- Les tableaux des recommandations ---------------------------
    //
    // Les tables ci-dessus disent ce qu'une dose vaut, ou ce qu'on fait
    // d'un produit. Celles-ci disent la **cible** : le chiffre à
    // atteindre et ce qui range un patient dans une catégorie plutôt
    // qu'une autre. C'est ce qu'on cherche quand une ordonnance semble
    // légère et qu'on ne sait plus si c'est elle ou notre souvenir.
    ConvTable {
        short: "LDL",
        family: "Adaptation",
        title: "LDL — la cible dépend du risque, pas d'un seuil unique",
        reviewed: "Août 2026 — recommandations ESC/EAS sur les dyslipidémies, fiche HAS",
        sources: &[
            "ESC/EAS — prise en charge des dyslipidémies",
            "HAS — principales dyslipidémies : stratégies de prise en charge",
        ],
        columns: &[
            "Niveau de risque",
            "Ce qui range le patient là",
            "Cible de LDL",
            "En mmol/L",
            "Ce qu'on fait pour l'atteindre",
        ],
        rows: &[
            &[
                "Très haut",
                "Maladie cardiovasculaire avérée (infarctus, AVC, artériopathie), diabète avec atteinte d'organe ou ≥ 3 facteurs, IRC sévère, hypercholestérolémie familiale avec un autre facteur",
                "< 0,55 g/L et baisse d'au moins 50 %",
                "< 1,4 mmol/L",
                "Statine de forte intensité à dose maximale tolérée, puis ézétimibe, puis inhibiteur de PCSK9",
            ],
            &[
                "Haut",
                "Un facteur très élevé isolé (LDL > 1,90 g/L, PA ≥ 180/110), hypercholestérolémie familiale seule, diabète ≥ 10 ans sans atteinte, IRC modérée",
                "< 0,70 g/L et baisse d'au moins 50 %",
                "< 1,8 mmol/L",
                "Statine de forte intensité ; ézétimibe si la cible n'est pas atteinte",
            ],
            &[
                "Modéré",
                "Diabète de moins de 10 ans sans autre facteur, ou risque calculé intermédiaire",
                "< 1,00 g/L",
                "< 2,6 mmol/L",
                "Mesures de mode de vie d'abord ; statine si la cible n'est pas atteinte",
            ],
            &[
                "Faible",
                "Aucun facteur au-delà de l'âge, risque calculé bas",
                "< 1,16 g/L",
                "< 3,0 mmol/L",
                "Mode de vie. Une statine ici se discute et ne va pas de soi",
            ],
            &[
                "Ce qui se vérifie au comptoir",
                "La cible n'est pas sur l'ordonnance : elle se déduit du dossier",
                "—",
                "—",
                "Un LDL « normal » sur le compte rendu du laboratoire ne dit rien : les bornes usuelles du laboratoire ne sont pas des cibles thérapeutiques",
            ],
        ],
    },
    ConvTable {
        short: "IC — piliers",
        family: "Posologies",
        title: "Insuffisance cardiaque à FEVG altérée — les quatre piliers",
        reviewed: "Août 2026 — recommandations ESC sur l'insuffisance cardiaque, RCP",
        sources: &[
            "ESC — insuffisance cardiaque aiguë et chronique",
            "RCP des spécialités, base de données publique des médicaments (ANSM)",
            "HAS — guide du parcours de soins : insuffisance cardiaque",
        ],
        columns: &[
            "Pilier",
            "Molécules",
            "Dose de départ",
            "Dose cible",
            "Ce qui se surveille",
            "Le piège",
        ],
        rows: &[
            &[
                "Bloqueur du SRAA",
                "Ramipril, périndopril, énalapril ; losartan ou candésartan si toux ; sacubitril/valsartan",
                "Ramipril 1,25 mg × 2 ; sacubitril/valsartan 24/26 mg × 2",
                "Ramipril 5 mg × 2 ; sacubitril/valsartan 97/103 mg × 2",
                "Kaliémie et créatininémie à 1 et 4 semaines de chaque palier ; pression artérielle",
                "36 heures d'arrêt entre un IEC et le sacubitril/valsartan — sinon angio-œdème. Jamais les deux ensemble",
            ],
            &[
                "Bêtabloquant",
                "Bisoprolol, carvédilol, métoprolol succinate, nébivolol — ces quatre-là et pas un autre",
                "Bisoprolol 1,25 mg/j ; carvédilol 3,125 mg × 2",
                "Bisoprolol 10 mg/j ; carvédilol 25 mg × 2",
                "Fréquence cardiaque, pression artérielle, poids et signes de congestion",
                "Ne s'instaure pas en pleine décompensation, et ne s'arrête jamais brutalement — l'arrêt brusque expose à l'aggravation",
            ],
            &[
                "Antialdostérone",
                "Spironolactone, éplérénone",
                "12,5 à 25 mg/j",
                "25 à 50 mg/j",
                "Kaliémie et créatininémie à 1 semaine, 1 mois, puis tous les 4 mois",
                "Contre-indiqué si kaliémie > 5,0 mmol/L ou DFG < 30. Gynécomastie sous spironolactone : l'éplérénone la remplace",
            ],
            &[
                "Gliflozine",
                "Dapagliflozine, empagliflozine",
                "10 mg/j — pas de titration",
                "10 mg/j",
                "DFG au début puis annuellement ; signes d'infection génitale",
                "Avec ou sans diabète. Règle des jours de maladie : on l'arrête si l'on ne mange plus, si l'on vomit ou si l'on se déshydrate",
            ],
            &[
                "Ce que le comptoir vérifie",
                "Les quatre présents, et titrés en parallèle",
                "Tôt et bas",
                "Ou la dose maximale tolérée",
                "Poids quotidien : +2 kg en trois jours, c'est de la congestion",
                "Les quatre s'instaurent ensemble et se montent ensemble, jamais l'un après l'autre jusqu'à sa cible",
            ],
        ],
    },
    ConvTable {
        short: "FA — scores",
        family: "Au comptoir",
        title: "Fibrillation atriale — CHA₂DS₂-VASc et HAS-BLED",
        reviewed: "Août 2026 — recommandations ESC sur la fibrillation atriale",
        sources: &[
            "ESC — prise en charge de la fibrillation atriale",
            "HAS — fibrillation atriale : guide du parcours de soins",
        ],
        columns: &[
            "Item",
            "Score",
            "Lequel",
            "Ce que cela veut dire",
        ],
        rows: &[
            &["Insuffisance cardiaque ou FEVG altérée", "1", "CHA₂DS₂-VASc", "C"],
            &["Hypertension artérielle", "1", "Les deux", "H — traitée ou non"],
            &["Âge ≥ 75 ans", "2", "CHA₂DS₂-VASc", "A₂ — c'est le poids le plus lourd avec l'AVC"],
            &["Diabète", "1", "CHA₂DS₂-VASc", "D"],
            &["AVC, AIT ou embolie antérieurs", "2", "CHA₂DS₂-VASc", "S₂"],
            &["Maladie vasculaire (infarctus, artériopathie, plaque aortique)", "1", "CHA₂DS₂-VASc", "V"],
            &["Âge 65 à 74 ans", "1", "CHA₂DS₂-VASc", "A"],
            &["Sexe féminin", "1", "CHA₂DS₂-VASc", "Sc — modulateur, il ne fait pas indication à lui seul"],
            &["Fonction rénale ou hépatique altérée", "1 ou 2", "HAS-BLED", "R et L — un point chacun"],
            &["Antécédent hémorragique ou anémie", "1", "HAS-BLED", "B"],
            &["INR instable sous AVK", "1", "HAS-BLED", "L — sans objet sous AOD"],
            &["Âge > 65 ans", "1", "HAS-BLED", "E"],
            &["Alcool ou médicaments qui font saigner (AINS, antiagrégants)", "1 ou 2", "HAS-BLED", "D — un point chacun"],
            &["Comment on lit le total", "—", "—", "CHA₂DS₂-VASc ≥ 2 chez l'homme ou ≥ 3 chez la femme : anticoaguler. HAS-BLED ≥ 3 : ce n'est pas une contre-indication, c'est une liste de choses à corriger"],
        ],
    },
    ConvTable {
        short: "HbA1c",
        family: "Adaptation",
        title: "HbA1c — l'objectif se choisit sur le patient",
        reviewed: "Août 2026 — recommandations HAS/SFD sur le diabète de type 2",
        sources: &[
            "HAS — stratégie médicamenteuse du contrôle glycémique du diabète de type 2",
            "SFD — prise de position sur la prise en charge du diabète de type 2",
        ],
        columns: &["Profil", "Objectif d'HbA1c", "Pourquoi", "Ce qu'il ne faut pas faire"],
        rows: &[
            &[
                "Diabète récent, espérance de vie longue, sans comorbidité",
                "≤ 6,5 %",
                "Le bénéfice microvasculaire d'un contrôle strict se joue tôt et dure",
                "Y arriver par un sulfamide à dose forte : les hypoglycémies annulent le bénéfice",
            ],
            &[
                "Cas général",
                "≤ 7 %",
                "L'objectif de la plupart des patients suivis en ville",
                "Le lire comme une note : une HbA1c à 7,2 % chez un patient stable n'est pas un échec",
            ],
            &[
                "Comorbidité sévère, complications évoluées, diabète ancien",
                "≤ 8 %",
                "Le bénéfice d'un contrôle strict ne se voit plus, le risque d'hypoglycémie, si",
                "Intensifier parce que le chiffre dépasse 7 : ce serait faire du mal",
            ],
            &[
                "Sujet âgé dit « fragile »",
                "≤ 8 %",
                "Une hypoglycémie chez lui, c'est une chute, une fracture, une hospitalisation",
                "Laisser un sulfamide ou une insuline rapide sans réévaluer",
            ],
            &[
                "Sujet âgé dit « malade »",
                "< 9 % et éviter les glycémies < 1 g/L",
                "L'objectif devient le confort et l'absence d'hypoglycémie",
                "Poursuivre une intensification décidée dix ans plus tôt",
            ],
            &[
                "Grossesse (diabète préexistant)",
                "≤ 6,5 % avant la conception",
                "Le risque malformatif se joue aux premières semaines",
                "Poursuivre metformine hors avis, IEC, ARA2, statines : tous se revoient avant la conception",
            ],
        ],
    },
    ConvTable {
        short: "DFG — stades",
        family: "Adaptation",
        title: "Insuffisance rénale chronique — les stades et ce qu'ils changent",
        reviewed: "Août 2026 — guide HAS sur la maladie rénale chronique, RCP",
        sources: &[
            "HAS — guide du parcours de soins : maladie rénale chronique de l'adulte",
            "RCP des spécialités, base de données publique des médicaments (ANSM)",
        ],
        columns: &[
            "Stade",
            "DFG (mL/min/1,73 m²)",
            "Ce qui s'arrête ou se réduit",
            "Ce qui se surveille",
            "Ce qu'on dit au patient",
        ],
        rows: &[
            &[
                "1 et 2",
                "≥ 60 avec une anomalie rénale",
                "Rien au titre du DFG seul",
                "DFG et albuminurie une fois par an",
                "Les AINS et la déshydratation font baisser le DFG plus vite que la maladie",
            ],
            &[
                "3A",
                "45 à 59",
                "Metformine poursuivie ; AOD à vérifier ; AINS à éviter",
                "DFG et kaliémie tous les 6 mois",
                "Boire normalement, et signaler tout épisode de vomissements ou de diarrhée",
            ],
            &[
                "3B",
                "30 à 44",
                "Metformine à dose réduite (max 1 000 mg/j) ; doses d'AOD adaptées ; AINS proscrits",
                "DFG, kaliémie et hémoglobine tous les 3 à 6 mois",
                "Règle des jours de maladie : on suspend metformine et gliflozine quand on ne mange plus",
            ],
            &[
                "4",
                "15 à 29",
                "Metformine et la plupart des gliflozines arrêtées ; dabigatran contre-indiqué ; revoir toute l'ordonnance",
                "DFG, kaliémie, calcémie, phosphorémie, hémoglobine tous les 3 mois",
                "Néphrologue ; et ne jamais prendre d'anti-inflammatoire, même vendu sans ordonnance",
            ],
            &[
                "5",
                "< 15",
                "Ordonnance revue molécule par molécule avec le néphrologue",
                "Suivi néphrologique rapproché",
                "Toute automédication passe par un avis, y compris la phytothérapie",
            ],
            &[
                "Ce qui trompe",
                "Créatininémie normale ≠ DFG normal",
                "Chez le sujet âgé peu musclé, une créatininémie à 90 µmol/L peut recouvrir un DFG à 40",
                "Le DFG se calcule, il ne se lit pas",
                "La calculatrice de Cockcroft du module le donne en trois champs",
            ],
        ],
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    /// Every drawer of the list has something in it: a heading with no
    /// rows under it is a family somebody renamed on one side only.
    #[test]
    fn every_family_has_tables_in_it() {
        for f in FAMILIES {
            assert!(TABLES.iter().any(|t| t.family == f), "famille vide : {f}");
        }
    }

    #[test]
    fn tables_are_well_formed() {
        // The catalogue only ever grows: a table withdrawn is a question
        // the counter can no longer answer from the application.
        assert!(
            TABLES.len() >= 43,
            "{} tables livrées, il y en avait quarante-trois",
            TABLES.len()
        );
        let mut shorts = std::collections::HashSet::new();
        for t in TABLES {
            assert!(!t.title.is_empty());
            assert!(!t.sources.is_empty());
            // A reference table without a review date is one nobody
            // dares use: it must say how old it is.
            assert!(
                !t.reviewed.trim().is_empty(),
                "table sans date de relecture : {}",
                t.title
            );
            assert!(!t.rows.is_empty());
            assert!(!t.columns.is_empty());
            // The list shows `short`: two tables sharing one would put
            // two identical rows in the same drawer.
            assert!(
                shorts.insert(t.short),
                "nom de table en double : {}",
                t.short
            );
            // A family the list has no drawer for is a table nobody can
            // reach: the list is built from `FAMILIES`, in that order.
            assert!(
                FAMILIES.contains(&t.family),
                "famille inconnue « {} » sur la table « {} »",
                t.family,
                t.short
            );
            for row in t.rows {
                for cell in row.iter() {
                    assert!(!cell.trim().is_empty(), "cellule vide dans « {} »", t.title);
                }
            }
            for row in t.rows {
                assert_eq!(
                    row.len(),
                    t.columns.len(),
                    "row width mismatch in « {} »",
                    t.title
                );
            }
        }
        // The families the acts lean on are present.
        for prefix in [
            "IPP",
            "HBPM",
            "AOD",
            "Insulines",
            "Corticoïdes inhalés",
            "Angine",
            "Cystite",
            "Contraception",
            "Vaccination",
            "Fonction rénale",
            "Interactions",
            "Urgence",
            "Grossesse",
            "Pédiatrie",
            "Écraser",
            "Sujet âgé",
            "Dispositifs inhalés",
            "Antidiabétiques",
            "Collyres",
            "Automédication",
            "Antibiotiques",
            "Arrêts et sevrages",
            // Les tableaux des recommandations : la cible, et ce qui
            // range un patient dans une catégorie plutôt qu'une autre.
            "LDL",
            "Insuffisance cardiaque",
            "Fibrillation atriale",
            "HbA1c",
            "Insuffisance rénale chronique",
        ] {
            assert!(
                TABLES.iter().any(|t| t.title.starts_with(prefix)),
                "table « {prefix} » manquante"
            );
        }
        // Short names label the selector buttons: they must be unique.
        let mut shorts: Vec<&str> = TABLES.iter().map(|t| t.short).collect();
        shorts.sort_unstable();
        let n = shorts.len();
        shorts.dedup();
        assert_eq!(shorts.len(), n);
    }
}
