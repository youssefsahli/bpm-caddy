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
    pub title: &'static str,
    /// Numbered under the table, on screen and in the PDF.
    pub sources: &'static [&'static str],
    pub columns: &'static [&'static str],
    pub rows: &'static [&'static [&'static str]],
}

pub const TABLES: &[ConvTable] = &[
    ConvTable {
        short: "IPP",
        title: "IPP — équivalences, formes et prise",
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
        title: "HBPM — posologies, rein, surveillance",
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
        title: "Statines — intensité, efficacité, interactions",
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
        title: "Corticoïdes — équivalences, durée, formes",
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
        title: "Opioïdes — équianalgésie et repères pratiques",
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
        title: "Benzodiazépines — équivalences, demi-vie, indication",
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
        title: "AOD — posologies, adaptation rénale et antidotes",
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
        ],
    },
    ConvTable {
        short: "Cortico. inhalés",
        title: "Corticoïdes inhalés — paliers de dose, dispositifs et rinçage (adulte)",
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
        ],
    },
    ConvTable {
        short: "Insulines",
        title: "Insulines — profils d'action, injection et conservation",
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
        title: "Fonction rénale — stades et conséquences pratiques",
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
        title: "Angine — score de Mac Isaac, TROD et antibiothérapie",
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
        title: "Cystite simple — traitements, contre-indications et suivi",
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
        title: "Contraception — oubli, délai toléré et rattrapage",
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
        title: "Antalgiques — palier, doses adulte et précautions",
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
        title: "Vaccination à l'officine — population, schéma et rôle du pharmacien",
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
        title: "Pédiatrie — doses usuelles, formes et maximum par jour",
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
        title: "Écraser ou ouvrir — règles, raisons et alternatives",
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
        title: "Interactions à repérer à la délivrance — aliments, plantes et inducteurs",
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
        title: "Urgence au comptoir — reconnaître, agir, orienter",
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
        title: "Grossesse et allaitement — ce qui se délivre au comptoir",
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
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_are_well_formed() {
        assert!(TABLES.len() >= 6);
        let mut shorts = std::collections::HashSet::new();
        for t in TABLES {
            assert!(!t.title.is_empty());
            assert!(!t.sources.is_empty());
            assert!(!t.rows.is_empty());
            assert!(!t.columns.is_empty());
            // The selector shows `short`: two tables sharing one would
            // put two identical buttons side by side.
            assert!(
                shorts.insert(t.short),
                "nom de table en double : {}",
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
