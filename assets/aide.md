# BPM-Caddy

Suivi des entretiens pharmaceutiques au comptoir : dossiers, actes,
agenda de l'équipe, registre des stupéfiants, caisse, et documents
imprimés au nom de l'officine.

Données locales. La base est chiffrée et reste sur le poste. L'application
n'émet que **deux** requêtes réseau, toutes deux à la demande : la
recherche d'une mise à jour, et la mise à jour de l'annuaire des
prescripteurs — uniquement si l'officine a renseigné une adresse ; sinon
le bouton n'apparaît pas. Les deux téléchargent, aucune n'envoie de
données.

# Plan de travail

Trois volets autour d'un cahier d'onglets.

- À gauche, la navigation : le mois, les prochains rendez-vous, la liste
  en cours de recherche.
- Au centre, la vue ouverte. Les onglets du haut en gardent plusieurs
  ouvertes à la fois.
- À droite, ce volet : notes d'équipe, carnet du jour, notes
  personnelles et aide.

Les volets latéraux se ferment et se redimensionnent ; leur largeur et
la vue ouverte sont conservées d'une session à l'autre.

L'onglet **Aide** ouvre en tête, sous « Sur cette vue », la section qui
parle de l'écran en cours — l'agenda, un dossier, la caisse… — puis
toutes les autres. Une recherche remplace cet ordre.

Le bouton « ? » de la barre du haut, ou `F12`, liste tous les raccourcis
clavier. Cette liste est générée par l'application et reste à jour.

Après une mise à jour, la fenêtre **« Nouveautés »** s'ouvre une fois
sur ce poste : ce que la version apporte, modifie et corrige — toutes
les versions sautées si plusieurs sont arrivées d'un coup. Elle se
rouvre sur les trois dernières versions depuis
Options › À propos › « Nouveautés… ».

Sur une base neuve, le tableau de bord ouvre un panneau **« Premiers
pas »** : la fiche de l'officine et son équipe, la trame de chaque
personne, les sauvegardes, les autres postes, le réseau d'officines. Il
disparaît au premier dossier.

# Recherche

« Aller à… » cherche partout d'un seul champ : les vues elles-mêmes, les
dossiers, les fiches, les tables de conversion, les préparations, les
protocoles, les dispositifs, les compléments nutritionnels, le registre,
les carnets de suivi, les scripts, les outils de calcul et les textes imprimés. Saisir le nom, un
fragment du nom ou les initiales.

Les outils se trouvent **par leur usage** et non par le nom de leur
écran : « enfant » ou « mg/kg » mène à la dose au poids, « DFG bas » à
l'adaptation rénale, « demi-vie » à la décroissance, « horaires » à la
trame de la semaine, « périmés » aux retours et à la destruction,
« sauvegarde » à la page de la base, « angine » aux lignes du protocole
TROD. Champ vide, la boîte liste les vues puis les outils : trame,
planning, réseau d'officines, postes, codex, dispositifs, ordonnancier,
vigilance, retours, pièces, textes imprimés, libellés, listes de
contrôle, croisement, lignes du TROD, modèles, options, base et
sauvegardes, règles et quotas, liste d'appel — précédés des cinq
dernières destinations choisies dans la boîte. Le récapitulatif de
facturation ne s'y propose pas : il se tape (« facturation »). La
fenêtre « ? » (F12) a deux pages : « Raccourcis », et « Outils, par
usage », qui liste les outils avec leur usage ; un clic ouvre l'outil.

**Favoris.** L'étoile d'un dossier (à côté du nom), d'une fiche
médicament (en tête des boutons de la fiche) ou d'un outil (page
« Outils, par usage ») l'épingle ; dans « Aller à… », Ctrl+D épingle ou
retire la ligne choisie. Les favoris passent en tête de la boîte ouverte
à vide, avant les récents, marqués d'une étoile. Ils sont à la personne
choisie dans le volet (« Opérateur ») et la suivent d'un poste à
l'autre ; sans opérateur, ce sont ceux du poste.

La recherche ignore la casse et les accents, et accepte des lettres non
contiguës dans l'ordre : « jndp » retrouve Jean Dupont.

Dans les monographies, « Dans le texte… » cherche dans le **contenu**
des fiches et non dans leur titre : chaque fiche trouvée s'affiche avec
la phrase qui contient le terme.

**Trois affichages de la monographie**, au choix dans Options ›
Interface : « Feuille » présente la fiche comme une page imprimée ;
« Dense » supprime la feuille et les marges pour afficher davantage ;
« Lecture » resserre la colonne pour une lecture suivie. Aucun ne
modifie la taille du texte, réglée une fois pour toute l'application à
« Taille du texte ».

# Dossier patient

Le bandeau porte l'identité et les traitements ; en dessous, les actes,
le journal, la biologie, les vaccins et les pièces scannées.

Saisie courte des dates : `230826` ou `2308` donne `23/08/2026`. Des
heures : `9`, `930`, `9h30`.

Les données partagées entre postes — identité, états d'acte,
rendez-vous — sont écrites par comparaison avec la valeur affichée. Si
un autre poste a modifié la même ligne entre-temps, l'écriture est
refusée et l'écran se recharge, sans écraser la saisie de l'autre poste.

## Lecture d'ordonnance

Sous la biologie, six lectures de la même ordonnance : interprétation
des résultats, surveillance biologique en retard, adaptation à la
clairance, grossesse et allaitement, sujet âgé, et interactions sur les
cytochromes.

La lecture du sujet âgé est la seule dont la donnée est **déjà au
dossier** : la date de naissance, saisie à la création. La fonction rénale
conduit à adapter la dose ; l'âge, à reconsidérer le choix de la molécule. Les lignes viennent de la liste
française de Laroche, des critères STOPP/START et des critères de Beers ;
chacune indique le risque et l'alternative. Aucun arrêt brutal n'est proposé :
l'arrêt d'un psychotrope chez le sujet âgé expose à davantage de risques
que sa poursuite, et le remplacement se prépare avec le prescripteur.

**Une forme locale n'est pas traitée comme la voie générale.** Collyres,
pommades, gels et pulvérisations nasales ne reçoivent ni palier rénal,
ni niveau de grossesse, ni demande d'examen : les tables sont indexées
par molécule, et une même molécule n'a pas les mêmes effets selon la
voie. La ciclosporine en collyre ne génère aucune interaction ; le
lithium en gel ne demande pas de lithiémie. Les précautions de la voie
locale figurent sur la fiche du produit.

La dernière lecture recherche les interactions par voie enzymatique
et par transporteur. Elle **ne connaît que sept cytochromes et cinq
transporteurs** — la glycoprotéine P, l'OATP1B1, la BCRP, et l'OCT2 et
le MATE1 du rein, qui portent les interactions qu'aucun cytochrome
n'explique : le dabigatran sous amiodarone, la digoxine sous vérapamil,
la rosuvastatine sous ciclosporine, la metformine sous dolutégravir. Les autres transporteurs, les glucuronoconjugaisons
et les effets additifs ne sont pas couverts : deux sédatifs n'interagissent
sur aucune enzyme, mais leurs effets s'additionnent. Une absence d'interaction enzymatique
n'est pas une absence d'interaction ; les lignes absentes de la table
sont **listées**. Deux lignes qui se rencontrent sur plusieurs voies —
la colchicine sous clarithromycine, par le CYP3A4 et la P-gp — tiennent
en une seule entrée qui les nomme toutes. Deux anti-interleukine 6, le
tocilizumab et le sarilumab, y figurent aussi : l'inflammation freine les
cytochromes, bloquer cette interleukine rétablit leur activité, et la simvastatine ou
la ciclosporine voient leur exposition baisser à l'instauration.

Une prodrogue s'y lit à l'envers, et la ligne le précise : inhiber
l'enzyme qui produit le métabolite actif du clopidogrel, de la codéine,
du tramadol, du losartan ou du tamoxifène ne les fait pas s'accumuler :
leur effet est supprimé.

Sous les voies, quand l'ordonnance en porte, **les paires qui se
rencontrent sur une même cascade** — comme au croisement : les effets
qui s'additionnent ou s'opposent, et le titre de la cascade qui l'ouvre
avec les deux molécules données.

Trois réponses possibles : interaction, **sans voie connue** (molécule
présente dans la table, qui ne passe par aucune des voies suivies —
critère utile pour choisir un traitement de remplacement), ou **absente
de la table**, ce qui ne permet aucune conclusion.

## Fiche de traitement

« Fiche traitement… », en haut du dossier, imprime la fiche remise au
patient lors d'un changement d'ordonnance. Trois parties.

**Plan de prise.** Une grille de quatre moments — matin, midi, soir,
coucher — établie à partir de la posologie retenue au dossier pour
**ce** patient. Un chiffre quand l'ordonnance précise la quantité, une
pastille quand elle précise le moment sans la quantité : la fiche
n'ajoute aucune unité non prescrite.

Une posologie non interprétée occupe une seule case sur les quatre
colonnes et est imprimée telle quelle : quatre cases vides se liraient
« rien à prendre ». Une prise « si besoin » est sortie de la grille avec
sa condition, pour ne pas devenir systématique au pilulier. Une prise
hebdomadaire est également sortie de la grille et écrite en toutes
lettres, comme pour le méthotrexate.

**Renouvellement.** Le rang de la délivrance sur le total, la période
couverte par la délivrance en cours, la date de fin de l'ordonnance, et
la date limite de consultation du prescripteur, fixée quelques jours
avant l'échéance pour laisser le temps d'obtenir un rendez-vous. Ce
délai et l'affichage de l'avancement — pastilles, jauge, dates ou texte
seul — se règlent dans Options › Règles.

Le rang de la délivrance est **saisi**, non déduit du calendrier : un
patient qui revient avec trois semaines de retard en est à sa deuxième
délivrance, et non à sa quatrième. Il se note sur la puce du traitement,
avec la date de l'ordonnance, la durée couverte par délivrance et le
nombre de renouvellements. Aucun champ n'est obligatoire : une ligne non
renseignée l'indique à l'impression.

**Détail par médicament.** Indication, posologie, conseils, conduite en
cas d'oubli et, dans un cadre distinct, signes d'alerte. Le contenu vient
des fiches du référentiel, corrigées par l'équipe à la source : il n'y a
pas de seconde table de conseils.

## Écrasement des formes orales

Le bouton « Écrasement » du dossier imprime une fiche pour l'EHPAD ou
l'infirmier : toute l'ordonnance, ligne par ligne, la conduite pour
chaque forme et l'alternative lorsqu'elle existe.

**Produit absent de la table.** Il reçoit « à vérifier », jamais une
ligne vide : une fiche limitée aux interdictions se lirait « tout le
reste est permis ».

Trois réponses pour les produits connus : oui, non, et « oui, gélule
ouverte » — les microgranules s'avalent sans être croqués. C'est le cas
le plus fréquent en gériatrie ; le classer en « non » ferait modifier
une ordonnance sans nécessité.

**La conduite dépend de la forme galénique et non de la molécule.** Pour la morphine :
Moscontin jamais, Skenan en ouvrant la gélule. Les formes non orales —
injectables, implants — ont aussi leur ligne, qui indique l'absence
d'objet.

Chaque refus indique l'alternative, ou l'absence d'alternative et le
recours au prescripteur. Certains refus protègent le soignant :
cytotoxiques et tératogènes exposent par la poussière, et la fiche le
mentionne.

## Conciliation de sortie

L'onglet « Conciliation » compare l'ordonnance du dossier à l'ordonnance
de sortie d'hospitalisation. La liste de sortie est collée ; chaque
ligne est rapprochée d'une fiche, et l'écran indique les arrêts,
modifications, ajouts et remplacements.

Six lectures, et la première passe avant les autres : la ligne non
rapprochée, donc non vérifiée. Puis le remplacement dans la même classe
— risque de doublon, le patient gardant les deux boîtes —, l'arrêt, la
modification de posologie, l'ajout, et la reconduction à l'identique.

La feuille s'imprime à l'attention du prescripteur, avec un cadre
réservé à sa réponse. Elle ne constitue pas un avis médical et le
mentionne.

# Croisement

L'écran « Croisement » applique les mêmes analyses à une liste libre,
sans dossier : reprise du dossier ouvert ou saisie des noms.

Dix chapitres sur la même liste : les **interactions citées** par les
monographies, sans déduction ; les interactions sur les
**cytochromes et les transporteurs** ; la **demi-vie plasmatique**, soit
le délai de retour à l'état antérieur après une modification
d'exposition ; la **revue d'ordonnance** — doublons, associations,
cascades de prescription ; **sur une même cascade** ; l'**adaptation
rénale** à la clairance saisie ; l'**adaptation hépatique** au stade
choisi ; le **sujet âgé** ; la **grossesse et l'allaitement** ; et
l'**écrasement des formes orales**.

**Un sommaire en tête** : une ligne de liens, un par chapitre, avec le
compte de ce que portent les cinq lectures qui croisent — interactions
citées, voies, demi-vies touchées, points de revue, paires sur une
cascade. Un clic fait défiler l'écran jusqu'au chapitre. Les tables du terrain
(rein, foie, âge, grossesse, écrasement) n'ont pas de compte : leurs
lignes portent aussi « aucune adaptation », qu'un compte présenterait
comme des alertes.

**Le foie ne s'évalue pas par un DFG.** La fonction rénale s'exprime par un chiffre
de laboratoire ; la fonction hépatique par un stade — Child-Pugh A, B ou
C — attribué par le clinicien sur cinq critères, dont deux cliniques.
Le panneau propose donc trois boutons et non un champ numérique.

Trois réponses, comme pour les cytochromes. « Non documenté » est en
gris ; « aucune adaptation » est écrit en toutes lettres. Exemple :
l'oxazépam, benzodiazépine de choix chez le cirrhotique, ne demande aucune
adaptation en insuffisance hépatique légère à modérée ; ne rien afficher le
confondrait avec un produit non documenté.

Une hépatopathie évolutive n'est pas un stade : les statines y sont
contre-indiquées quel que soit le score de Child-Pugh. Cette
contre-indication figure sur leur fiche, et non sous un palier.

**Sur une même cascade** nomme deux lignes qui agissent sur une même
cascade livrée — l'aspirine et le clopidogrel sur l'activation
plaquettaire, le nitré et le sildénafil sur le GMPc, l'oxybutynine et
le donépézil sur l'acétylcholine — et indique ce que la figure montre des
deux ensemble : elles **s'additionnent** sur un effet, ou elles
**s'opposent**. Ce chapitre complète celui des cytochromes : deux molécules
qui ne se croisent sur aucune enzyme peuvent se rencontrer sur un récepteur.
Le modèle est qualitatif : un sens de variation, jamais une gravité. Le
titre de la cascade l'ouvre, les deux molécules déjà données.

La carte trace un arc par interaction, du produit en cause vers le
produit affecté. La couleur indique l'ordre de lecture, non une gravité
clinique : dose, durée et terrain ne sont pas connus du logiciel. Deux
lignes qui se rencontrent sur une même cascade sont reliées **en
tirets**, sans pointe : la rencontre a lieu sur un récepteur, sans
enzyme ni sens ; la clé « même cascade » ne paraît que si un tel trait est
tracé.

**La durée compte autant que le sens.** « Exposition augmentée » n'a pas
la même portée pour une demi-vie de deux heures que pour une demi-vie de
cinquante jours, et un effet peut persister après l'élimination du produit : l'effet
antiagrégant du clopidogrel dure sept à dix jours pour une demi-vie de
six heures.

# Calculs

Accès par « Aller à… » — « enfant », « mg/kg », « clairance », « DFG
bas », « demi-vie » —, par « Calculs » au-dessus des tables de
conversion, et par le bouton « Calculs… » d'une fiche, qui les ouvre
**pour cette fiche**.

Sans fiche, les outils sont génériques. Avec une fiche, deux d'entre eux
répondent pour ce médicament.

**Clairance de la créatinine**, selon Cockcroft et Gault, avec le stade
correspondant. Estimation sur le poids réel : elle s'écarte chez
l'obèse, l'œdémateux ou le dénutri ; le laboratoire rend le DFG estimé
selon la formule en vigueur.

**Dose au poids.** Poids, milligrammes par kilo, nombre de prises. Avec
une fiche, **les doses au poids qu'elle indique** : ses lignes par
indication d'abord, chacune sous son titre, puis celles de la posologie
générale — chaque dose avec son rythme et la phrase source. Un plafond
(« sans dépasser… ») est affiché comme tel, une association précise la
molécule de chaque chiffre, et une dose cumulée par cure n'est pas
reprise. Le rythme fait partie de la dose : « 15 mg/kg par prise toutes
les 6 heures » et « 60 mg/kg par 24 heures » décrivent le même
traitement. L'indication aussi : une même fiche indique souvent
50 mg/kg/j pour l'angine et 80 pour l'otite. Une fiche sans dose au
poids n'en propose pas, et l'écran le signale : posologie à compléter.

**Adaptation rénale**, pour la fiche ouverte et un DFG saisi, ou repris
du calcul précédent d'un bouton. Même table que le panneau « Rein » du
dossier, appliquée à une fiche plutôt qu'à une ordonnance — utile
lorsque le dossier du patient n'est pas ouvert. Le palier retenu est le
plus bas des paliers franchis ; les seuils viennent des RCP ; la
décision revient au prescripteur.

**Décroissance et accumulation** : demi-vie, intervalle entre deux
prises, et délai d'élimination. La demi-vie peut être reprise d'une
fiche de la base.

# La carte pharmacologique

Depuis une fiche, « Carte… » affiche les produits liés sous forme de
graphe : la fiche au centre, les fiches liées autour ; un clic change le
centre. Usages : rupture de stock, contre-indication, apprentissage
d'une classe.

Trois anneaux, du plus proche au plus éloigné. **Molécule** : une autre
spécialité de la même DCI — la substitution, seul lien entre deux
produits de même principe actif. **Classe** : une autre molécule du même
groupe, en cas de rupture ou d'intolérance ; le groupe est celui du
référentiel et non le libellé de la fiche (« bisphosphonate » et
« biphosphonate » forment un seul anneau). **Interaction** : une fiche
qui interagit avec le centre, dans toute la base. Trois sources : les
interactions citées par les deux monographies, la table des cytochromes,
et la revue d'ordonnance — deux médicaments allongeant le QT, deux
sédatifs, un anticoagulant et un AINS interagissent même lorsque les
fiches ne se citent pas.

**Épaisseur et couleur du trait selon la gravité.** Trait épais et rouge
pour une association « contre-indiquée » dans la fiche ou une règle
d'alerte de la revue. Jamais pour une interaction enzymatique seule : la
table des cytochromes classe par ordre de lecture, non par gravité.
Quand un anneau est saturé, il retient **les liens les plus graves, et
un par motif avant un second** : dix anticoagulants liés à un AINS par
la même règle ne masquent pas le lithium, lié par une autre. Le pied de
la carte indique le nombre de liens non affichés. L'infobulle d'un
produit indique **le motif du lien**, source citée ; le survol du trait
lui-même l'affiche aussi et estompe les autres. Sur la carte de
l'ordonnance, le survol d'un arc indique les deux lignes et les motifs.

L'infobulle d'un produit reprend aussi la toxicité ou la marge
thérapeutique documentée par sa fiche. Une coche indique que le dossier
ouvert **comporte déjà** ce médicament. Sous la figure, la légende
nomme chaque couleur, et une ligne indique les produits non affichés
faute de place.

**Substitut en interaction avec l'ordonnance.** Avec un dossier ouvert,
un triangle rouge marque, sur les anneaux Molécule et Classe, le produit
qui interagirait avec une autre ligne de l'ordonnance s'il remplaçait le
centre. Son infobulle indique la ligne et le motif.

**Retour.** La touche Retour arrière — ou le bouton « précédent » de la
souris — revient au centre précédent, et ainsi de suite.

**Masquer un anneau.** Un clic sur une entrée de la légende — « même
classe », « interaction » — masque l'anneau et libère sa place pour les
autres : sur un AINS, masquer la classe affiche davantage
d'interactions. La pastille se vide, le pied de la carte indique
l'anneau masqué, et un second clic le rétablit.

**Carte de l'ordonnance.** Avec un dossier ouvert d'au moins deux
lignes, le mode « Ordonnance » dispose chaque ligne sur un cercle et
trace un arc entre les lignes qui interagissent — mêmes tables, même
gravité, mêmes couleurs. Une règle portant sur trois lignes, comme
l'association diurétique-IEC-AINS, relie les trois. Le survol d'une
ligne indique ses interactions et leurs motifs. Une ligne sans
interaction est grisée et listée sous la carte : sans interaction **dans
ces tables**, ce qui n'exclut pas une interaction. Un clic sur une ligne
la met au centre de la carte ; le mode « Fiche » revient à la fiche au
centre.

**Déplacement et zoom.** Glisser le fond à la souris ou les touches
fléchées déplacent la figure ; la molette zoome autour du pointeur. Au
clavier, `+` et `−` zooment, `0` rétablit taille et position ; les
boutons `−` et `+` de la barre font de même, et un double-clic dans le
vide rétablit la vue. Les deux cartes, fiche et ordonnance, se
parcourent ainsi. Le titre du cadre indique le facteur de zoom dès qu'il
diffère de cent pour cent, le nombre de noms non affichés, lisibles au
survol, et le nombre de produits hors du cadre.

Le zoom permet de passer de la vue d'ensemble à l'exploration. Réduite, la
carte affiche davantage de produits — dans la limite d'un plafond fixe —
au détriment des noms. Agrandie, elle s'étend au-delà du cadre et fait
entrer d'autres produits, tous nommés, jusqu'à huit fois la taille
d'origine : un anneau d'interactions trop fourni se parcourt en
agrandissant puis en déplaçant la carte. La position d'un produit sur
son anneau est calculée sur l'ensemble des candidats : un produit
supplémentaire prend une place libre sans déplacer les autres.

**Survol d'un produit** : nom, DCI, classe, indication, toxicité
documentée, et statut s'il diffère de « commercialisé » — une rupture
exclut une substitution. Le rayon du produit survolé reste net, les
autres s'estompent.

L'infobulle détaille aussi **la portée du lien**. La couleur donne la
nature du lien ; les conséquences s'affichent dessous en pastilles,
identiques à celles de la barre du comptoir, mêmes tables et mêmes
réserves. Un produit lié s'évalue **par rapport au centre** : deux AINS
forment un doublon, deux molécules interagissent sur un cytochrome, un
substitut peut demander une adaptation rénale que le produit initial ne
demandait pas. Le centre n'est relié à rien sur la figure : il s'évalue
par rapport à l'ordonnance du dossier ouvert.

# Barre de comptoir

`F9` réduit la fenêtre à une barre compacte, affichée au premier plan
des autres applications, à garder dans un coin d'écran. Elle est **sans
bordure** : elle se déplace par son en-tête, et le menu voisin
d'« Agrandir » la positionne — dans un coin, en bandeau en bas de
l'écran, en colonne à droite, ou librement. Saisir un nom ou une
molécule ; les flèches parcourent les résultats, Entrée ouvre la fiche
sélectionnée, Échap efface la saisie puis, sur une saisie vide, rétablit
la fenêtre. Le champ garde le focus : une douchette, qui émule un
clavier, y saisit directement. **Alt et 1 à 5** déclenchent les cinq
actions du bas, dans l'ordre : la barre s'utilise entièrement au
clavier, d'une seule main.

**Pastilles à forme et couleur** : cercle barré pour une
contre-indication, triangle pour une précaution, coche pour une
utilisation autorisée par la table, points de suspension quand une
donnée manque. Elles se lisent sans le texte et sans la couleur.
L'onglet « Signaux » prend la couleur de l'alerte la plus grave, visible
depuis « Conseils » ou « Posologie ».

**Cinq pages, parcourues avec les flèches gauche et droite.** Haut et
bas parcourent les résultats, gauche et droite les pages de la fiche.
« Signaux » regroupe les lectures des tables. « Posologie » affiche
d'abord **la posologie du dossier, propre au patient** — la fiche donne
la référence, le dossier la prescription —, puis la posologie de la
fiche, **par indication**, les formes et dosages disponibles, et **une
courbe** : le profil d'action pour une insuline (le pic de l'après-midi
d'une NPH à côté du profil plat d'une glargine), et pour les autres
produits la décroissance plasmatique avec la fraction restante à
vingt-quatre heures. « Conseils » affiche les conseils au patient, la
conduite en cas d'oubli, **les signes d'alerte en entier** — la page
« Signaux » n'en affiche que la première phrase — et les notes de
l'équipe sur la fiche. Une page vide n'est pas proposée ; avec une seule
page, la bande d'onglets disparaît.

**« Précautions » s'ouvre sur une rangée d'organes** : les organes
exposés à une toxicité, du plus grave au moins grave, en couleur. Pour
la Cordarone : thyroïde et poumon en rouge, puis cœur, œil, foie, peau,
système nerveux. Degré et
source au survol. La rangée ne retient que la **toxicité** :
le cœur est à la fois l'indication et un organe cible de la toxicité de
l'amiodarone, et mêler indication et toxicité afficherait deux fois
« Cœur ». Un organe absent ne signifie
pas une innocuité : le texte complet suit.

Viennent ensuite les contre-indications, effets indésirables et
surveillance, puis **la marge thérapeutique et l'antidote**, pour la
conduite en cas de surdosage, et enfin **les sources de la fiche**.

La cinquième page, « Dossier », porte sur l'ordonnance ouverte et non
sur la fiche recherchée : chaque ligne **avec la posologie du dossier**
— « Zeclar · 500 mg matin et soir, 7 jours » —, et **un clic sur une
ligne en fait la recherche en cours**. Un liseré signale les lignes
qu'une table demande de vérifier, pour trier avant de les ouvrir. Seules
ces lignes sont marquées : « à vérifier » et « sans donnée » sont des
réponses et non des alertes ; les marquer reviendrait à marquer toute l'ordonnance.
Cette page permet la revue d'ordonnance au comptoir sans ressaisir les
noms. Sans dossier ouvert, elle n'apparaît pas.

La barre **reprend les réponses des tables**, en pastilles, sans
conclure : chaque pastille cite le terme de sa table, et son infobulle
précise la portée du module avant la conduite. Onze lectures : ce sont
les interactions citées par les fiches du dossier, la revue d'ordonnance
avec ce produit ajouté, **la même molécule déjà présente au dossier sous
un autre nom**, l'interprétation de la biologie du dossier sous ce
traitement, la surveillance biologique demandée et la date du dernier
dosage, les cytochromes et les transporteurs, **les cascades où ce
produit rejoint une ligne du dossier** (« Cascade · 1 paire(s) », les
effets qui s'additionnent ou s'opposent au survol), l'écrasement, la
grossesse et l'allaitement,
l'adaptation à la clairance du dossier, et le sujet âgé, à partir de la
date de naissance du dossier.

**Alertes en premier.** Les pastilles sont classées de la plus grave à
la moins grave ; à gravité égale, l'ordre des tables est fixe. Sur une
barre repliée après la troisième pastille, une alerte en quatrième
position serait manquée.

Une table sans réponse n'affiche pas de pastille : une rangée de « à
vérifier » ne signale rien. Quand aucune table ne répond, une ligne
l'indique : **l'absence de donnée n'est pas une autorisation**.

Le foie n'y figure pas : il demande un stade de Child-Pugh, qu'aucun
dossier ne porte. Le stade se choisit au croisement.

Un clic sur une pastille ouvre le croisement avec l'ordonnance du
dossier **et** la fiche recherchée, **au chapitre de la pastille**.
Exception : la surveillance ouvre l'onglet « À surveiller » du dossier,
car elle lit les **dates** des résultats, absentes d'une liste libre.

« Copier » place la page affichée dans le presse-papier, pour la coller
dans le logiciel de dispensation (posologie ou conduite en cas d'oubli
dans le commentaire d'une ligne) sans risque d'erreur de recopie. Le
texte copié est celui de la page affichée, précédé du nom du produit.
Sur la page « Dossier », la copie porte sur la fiche : une liste de
traitements nominative ne sort pas de l'application.

L'en-tête porte le nom du dossier ouvert **et les deux valeurs lues par
les pastilles** : âge et clairance. Une valeur absente apparaît comme
telle.

Sur une saisie vide, la barre affiche **les dernières fiches
consultées** : pour comparer deux produits, Échap efface la saisie et
ramène à cette liste.

**Recherche dans le texte en l'absence de nom.** Le champ cherche un nom
ou une molécule ; « pamplemousse » ne correspond à aucun produit mais
figure dans cent seize passages. La barre cherche alors dans le texte
des fiches — les treize champs **et les lignes de posologie** (« à
jeun », « à distance du fer », pamplemousse) — et affiche chaque fiche
avec la phrase trouvée. Un nom trouvé reste prioritaire.

Un code-barres saisi est reconnu comme tel et signalé, plutôt que
« aucun résultat ». Aucune fiche ne porte de code : le seul lien entre
un code et un produit est celui enregistré au registre des stupéfiants,
boîte présentée.

**Données du DataMatrix.** Code, lot et péremption sont affichés sous
le code lu. Une péremption dépassée s'affiche en rouge ; une boîte
marquée 09/2026 est valable jusqu'au 30 inclus. Aucun seuil au-delà :
le nombre de jours restants est affiché, et l'appréciation revient au
pharmacien selon la durée du traitement. Un lot non terminé ou une
lecture interrompue sur un champ inconnu sont signalés : un lot erroné
affiché comme sûr serait recopié lors d'un rappel de lot.

Si le code est enregistré au registre, la barre nomme le produit suivi
et « Délivrance » ouvre le registre **sur ce produit**. Sinon, elle
ouvre le registre, où l'association du code se fait manuellement.

# Entretiens

Un acte porte sa thématique, son état, sa date, sa durée et les
initiales de l'intervenant. « Tout imprimer » produit la fiche
d'entretien, le bilan et le plan de prise en un seul document.

La fiche d'entretien porte, selon l'acte, un **tableau à remplir** :
suivi de l'INR pour un AVK, prises oubliées, saignements et DFG pour un
AOD, grades des effets indésirables pour un anticancéreux, étapes de la
technique d'inhalation pour l'asthme, problèmes repérés et propositions
au prescripteur pour un bilan partagé de médication, plan personnalisé
(sujet, objectif, orientation) pour un rendez-vous de prévention. Le
prochain rendez-vous déjà posé s'inscrit dans son cadre, et sur le
courrier au médecin sous la synthèse. Titres, colonnes et lignes se réécrivent dans
« Textes imprimés », sous « Fiche d'entretien ».

Le bilan partagé de médication comporte une section consacrée au sujet
âgé : il concerne le patient polymédiqué, le plus souvent âgé, et il est
transmis au prescripteur. Chaque ligne indique le risque et
l'alternative ; aucune ne prescrit un arrêt.

## Fiche de vaccination

Sur un acte Vaccination, « PDF » imprime la fiche de l'injection : les
questions à poser avant (allergie, fièvre, grossesse, immunodépression,
anticoagulant, malaise antérieur, vaccin récent), le vaccin tracé —
nom, dose, lot, péremption, voie, site, heure —, les suites
(surveillance de quinze minutes, inscription au carnet, consignes) et
ce que le calendrier vaccinal doit encore d'après le carnet du patient.
Une réponse « oui » est une question à instruire et non une
contre-indication calculée. Phrases réécrivables sous « Fiche de
vaccination » ; mise en page, modèle `vaccin`.

## Feuille de TROD

« Tout imprimer » suit l'acte : pour un TROD, la feuille puis le
courrier au médecin traitant ; pour une vaccination, la fiche puis le
carnet de vaccination.

Sur un acte TROD, « PDF » imprime la feuille du test et non la fiche
d'entretien : signes qui orientent vers le médecin sans tester, score de
Mac Isaac pour l'angine, lecture du test avec lot, péremption et heure,
conduite selon le résultat, et les lignes du protocole qui s'appliquent
au patient d'après son âge, son sexe et une grossesse en cours. L'âge
connu coche l'item d'âge du score ; un résultat déjà enregistré arrive
coché. Le reste se coche à la main, devant le patient.

« CR » imprime le courrier au médecin traitant : le test réalisé, son
résultat et la conduite tenue — dispensation selon le protocole, ou
traitement symptomatique et consigne de reconsulter. Sans résultat
enregistré, la conclusion reste à écrire à la main.

Les phrases de la feuille et du courrier se réécrivent dans l'écran
« Textes imprimés », sous « Feuille de TROD » ; leur mise en page, par
« Modèles… ».

## Ordonnance sous protocole

Après un test rapide positif — angine à streptocoque, cystite simple —,
l'écran compose l'ordonnance autorisée par le protocole : antibiotique,
posologie, adjuvant et conseils.

Les lignes proposées sont **celles de l'officine** : livrées avec les
protocoles en vigueur, elles se modifient avec « Modifier les
lignes… » — molécule ajoutée, posologie modifiée, borne d'âge déplacée
lors d'une révision du protocole. Les modifications valent pour tous les
postes.

**Âge, sexe et grossesse sont facultatifs.** Le dossier les fournit
lorsqu'il les connaît — date de naissance, sexe ou à défaut NIR —, et la
fenêtre permet de les corriger pour cette ordonnance. Renseignés, ils
grisent les lignes exclues par le protocole, avec le motif ; inconnus,
ils ne bloquent rien. Si aucune ligne ne s'applique, l'écran le signale :
orientation médicale.

Aucune ligne n'est présélectionnée et toute posologie est modifiable :
le choix revient au pharmacien.

Les adjuvants sont les fiches portant l'étiquette correspondante, avec
leurs propres lignes de posologie : ajouter un produit revient à ajouter
une fiche.

Aucune mention n'est imprimée par défaut ; les mentions souhaitées se
saisissent dans Options › Mentions.

# Agenda et planning

L'agenda s'affiche par jour, semaine ou mois. Le filtre **ne masque
pas** : les entrées écartées restent tracées en trait contre la
gouttière, et une entrée qui chevauche une entrée retenue reste affichée
en entier, pour que les conflits restent visibles.

Les quatre affichages restent **synchronisés** : changer d'affichage
conserve le jour consulté. Le jour détaillé est encadré dans la grille
de la semaine comme dans celle du mois, et « Imprimer la semaine »
imprime la semaine affichée.

Le plan de journée affiche **un pointillé à l'heure courante**, avec un
point rouge dans la marge, uniquement sur la journée du jour.

« Planning » est le quatrième affichage : l'équipe, une ligne par
personne, sept colonnes de jours. Le total d'un jour passe au rouge dès
qu'une **plage d'ouverture n'est pas couverte**, y compris partiellement
(matin couvert, après-midi vide).

## Récurrences

Un poste se répète selon une récurrence : jour unique, tous les jours
jusqu'à une date, chaque semaine, semaines paires, semaines impaires,
une semaine sur deux, sur trois, sur quatre.

**Semaines paires et une semaine sur deux diffèrent.** Les premières
suivent le numéro de semaine du calendrier, la seconde se compte depuis
la date de départ. Elles coïncident pendant des années et divergent
définitivement après une année de 53 semaines.

Un congé se saisit comme une plage : « tous les jours », du 12 au 26.
La date de fin est alors obligatoire.

## Trame hebdomadaire

« Trame… » saisit la semaine complète d'une personne en une fois. Elle
s'ouvre sur la trame existante, pour correction comme pour ajout. Un
clic sur le nom d'une personne, dans la grille du planning, ouvre aussi
sa trame.

Une journée coupée se saisit en **deux postes** — 9 h – 12 h 30 puis
14 h – 19 h 30 — et non en un poste avec pause : la pause n'a pas
d'horaire, et la couverture compterait la personne présente pendant sa
coupure.

Pour une alternance, le rythme « Paires et impaires » ouvre deux
onglets : semaine paire et semaine impaire.
Une journée identique sur les deux est enregistrée une fois, en
hebdomadaire.

« Reprendre de… » remplit la grille avec la trame d'un collègue — son
rythme et ses journées — pour la personne choisie ; rien n'est écrit
avant « Poser la trame », et la trame du collègue n'est pas touchée.

« Lundi à vendredi » et « Lundi à samedi » recopient la journée
modifiée en dernier (à défaut, la première écrite). Le « × » d'une
rangée vide cette journée.

La dernière colonne dessine chaque journée sur l'échelle des heures :
les postes en barres, sur le fond des horaires d'ouverture quand
l'officine les a déclarés. En alternance, deux pistes par jour — paires
au-dessus, impaires en dessous — : un jour présent une semaine sur deux
se voit comme un vide sur l'une des deux pistes. Une astreinte est un
cadre, une absence une bande pâle. Le survol détaille la journée. La
colonne s'efface quand la fenêtre est trop étroite.

Une trame qui ne tient pas sur deux semaines de sept jours n'est pas
approximée : l'écran le signale et ne propose pas de remplacement.

## Modification d'une occurrence

« Ce jour seulement » applique « Modifier » et « Supprimer » à une seule
occurrence. La trame est conservée, une exception s'applique ce jour-là.

# Listes de contrôle

« Listes » regroupe les listes à cocher : ouverture, fermeture, retour
de congés, vérifications avant délivrance. Impression A4, une case par
ligne, date et intervenant à remplir.

Une liste diffère d'un protocole : un protocole est un arbre de
décision, une liste une vérification d'exhaustivité.

**Aucune liste livrée** : une base neuve n'en comporte aucune. Chaque
officine rédige les siennes.

# Récepteurs et cascades

« Cascades » dessine un mécanisme d'action : le ligand en haut, son
récepteur, les relais, les messagers, et les effets en bas. Les flèches
pleines activent, les flèches barrées inhibent ; les points qui défilent
le long d'une flèche indiquent que le signal passe ; une flèche dont la
source est inactive ne transmet plus rien.

**Les molécules** se cochent dans la liste de gauche. Cochée, une
molécule est donnée à partir du temps affiché ; décochée, elle s'arrête.
La figure montre aussitôt ce qui monte (triangle pointe en haut) et ce
qui baisse (pointe en bas), étage par étage. Le survol d'un nœud donne sa
nature, sa note et son état ; un clic isole sa lignée — les nœuds en amont
et en aval restent en clair, le reste pâlit — et un second clic
rétablit toute la figure.

**Les scénarios**, sous les molécules, sont des séquences prédéfinies
— « Arrêt brutal du bisoprolol », « Cascade de prescription » : un clic
donne les molécules à leurs pas, remet le temps à zéro et lance la
lecture.

**Un dossier ouvert** : « Molécules du dossier » donne d'un coup les
molécules de la cascade que prend le patient, et chacune est marquée
« au dossier » dans la liste.

**Le temps** : « Lecture » (ou la barre d'espace) fait avancer les pas ;
un clic dans les courbes place le temps. Chaque effet a sa courbe, le
filet horizontal étant l'état de repos ; un clic sur une clé de la
légende masque ou rend une courbe. Un récepteur qui s'adapte se
multiplie quand il est longtemps bloqué et se raréfie quand il est
longtemps stimulé — son adaptation est tracée en tirets, et le filet
sous sa case la montre sur la figure. Ce mécanisme rend compte de la tolérance et du
rebond à l'arrêt brutal ; pour l'observer : donner un bêtabloquant, avancer, le décocher,
avancer encore.

Sur un écran bas, « Figure » et « Courbes » se montrent l'une après
l'autre. La figure se parcourt comme la carte pharmacologique :
glisser, molette, + et −, 0 pour revenir.

Le modèle est **qualitatif** : il montre des sens de variation, jamais
une dose ni un délai. Le temps est en pas, sans unité.

**Décrire** ouvre le texte de la cascade, et la figure se redessine à
mesure qu'on écrit. Une instruction par ligne :

- `titre :`, `sujet :` et `source :` en tête ;
- une déclaration : `ligand`, `récepteur`, `relais`, `enzyme`,
  `messager`, `canal`, `transporteur` ou `effet`, puis le nom, puis
  `: une note` au besoin ;
- une flèche : `A -> B` active, `A -| B` inhibe, une virgule pour en
  relier plusieurs, et elles s'enchaînent sur une ligne ;
- une molécule : `molécule bisoprolol : antagoniste Bêta-1` — les actions
  sont agoniste, agoniste partiel, agoniste partiel faible, antagoniste,
  inhibiteur, activateur et potentialisateur, séparées par `;` ;
- `adaptation Bêta-1` : le nœud s'adapte avec le temps ;
- `scénario Arrêt brutal : bisoprolol 10-70 ; propranolol 60-` : une
  séquence à jouer — chaque molécule avec son premier pas et son dernier
  (rien après le tiret : jusqu'au bout) ;
- `tonus faible Récepteur mu central` : le nœud est presque au repos
  sans molécule — le bloquer ne modifie presque rien, le stimuler produit
  tout l'effet (la naloxone seule est sans effet ; sous morphine, elle en
  antagonise l'effet) ;
- `#` commente la fin de la ligne.

« Dupliquer » en fait une copie à retravailler : une cascade livrée
réécrite ou supprimée ne revient pas, et sa copie laisse l'originale
intacte.

Ce qui ne se lit pas est signalé sous le texte avec son numéro de ligne,
et le reste est dessiné quand même. En orange, ce qui se lit mais
ressemble à une erreur : une molécule dont aucune fiche ne porte le nom
(aucune fiche ne mènera à la cascade), un nœud qu'aucune flèche ne
relie. Le rappel de la syntaxe s'affiche au survol de sa ligne ; Ctrl+S
enregistre.

**Les marques des nœuds** indiquent leur nature avant le nom : un disque
plein pour un ligand, un Y pour un récepteur, un cercle pour un relais,
un losange pour un enzyme, deux points pour un messager, deux barres
pour un canal, un carré traversé pour un transporteur, un carré plein
pour un effet — posé en creux.

**59 cascades livrées**. Récepteurs : bêta-adrénergiques, alpha-1,
muscariniques, dopaminergiques D2, histaminiques H1, opioïde mu,
GABA-A, synapse sérotoninergique, centre du vomissement, vessie
hyperactive. Neurologie et dépendances : lévodopa et dégradation de la
dopamine, cibles des antiépileptiques, migraine, glutamate et récepteur
NMDA, veille et sommeil, alcool, nicotine et arrêt du tabac. Cœur, rein
et métabolisme : système rénine-angiotensine-aldostérone, monoxyde
d'azote et GMPc, canaux calciques L, néphron et potassium, hyperkaliémie
et chélateurs, vasopressine et eau libre, canal hERG et QT long,
digoxine et pompe Na/K-ATPase, glycémie et incrétines, LDL-cholestérol,
activation plaquettaire, cyclo-oxygénases, coagulation, cycle de la
vitamine K, sécrétion acide, transit intestinal et laxatifs. Hormones
et os : axes thyréotrope, corticotrope et gonadotrope, contraception
hormonale, récepteur des androgènes, récepteurs des œstrogènes et
aromatase, calcium, parathormone et vitamine D, remodelage osseux,
rétinoïdes. Sang, bronches et infections : purines et goutte,
érythropoïèse, thrombopoïèse, bronchodilatation et inflammation
bronchique, ritonavir booster du CYP3A4, aminosides. Oncologie :
inhibiteurs de kinases, VEGF et angiogenèse. Immunité et cytokines :
lymphocytes T et B, circulation des lymphocytes, mastocyte, TNF-alpha,
interleukine 6, voie JAK-STAT, axe IL-23 et IL-17, inflammation de
type 2. Semées une fois : une cascade réécrite ou supprimée par
l'équipe ne revient pas.

Au-dessus de la liste, un filtre retient les cascades dont le titre,
le sujet ou une molécule contient le mot tapé : « digoxine » trouve la
digoxine et le QT long.

Depuis une fiche, « Cascade : … » ouvre la cascade qui nomme sa molécule,
la molécule déjà donnée. Dans l'autre sens, un clic sur la ligne d'une
molécule (ou un clic droit sur sa case) ouvre sa fiche.

# Carte vaccinale

Deux tables **indicatives**, chacune avec sa source à l'écran : le
calendrier vaccinal (vaccinations dues chez l'adulte) et la table du
voyageur (recommandations du BEH par pays).

Aucune ne remplace le texte source ; elles servent de rappel rapide au
comptoir.

La carte est un **cartogramme**, non une projection : chaque pays occupe
une case de même taille, regroupée par région. Elle sert à trouver un
pays, non à mesurer une distance.

Le carnet de vaccination d'un dossier s'imprime avec les doses, leurs
dates, le lot, le site d'injection et la date du prochain rappel
lorsqu'elle est connue. Les vaccinations **manquantes** s'affichent dans
le dossier, par comparaison du calendrier et des doses enregistrées.

# Campagne de vaccination

L'onglet **Campagne** regroupe ce que l'équipe consulte chaque jour
d'octobre à février. La saison court du 1er septembre au 31 août.

**À rappeler.** Les dossiers que le calendrier vaccinal déclare dus
pour la grippe, le COVID-19 ou le VRS (65 ans et plus, grossesse en
cours selon la date des dernières règles, 75 ans et plus pour le VRS)
et qui n'ont aucune dose de la saison au carnet. Les personnes les plus
âgées apparaissent en premier. Après l'appel, un bouton note l'issue :
**Prévenu**, **Vacciné ailleurs** et **Refus** retirent le dossier de la
liste pour la saison ; **Message** le laisse, avec la date de l'appel.
**Rendez-vous** ouvre sous la ligne le jour et l'heure : le rendez-vous
s'inscrit à l'agenda comme acte de vaccination planifié, et l'appel est
noté « Prévenu ». Le jour venu, quand la dose est inscrite au carnet,
« Créer l'acte » réalise ce rendez-vous au lieu d'en créer un second.
« Rendez-vous (n) », à côté des vaccins, remplace la liste par les
rendez-vous de vaccination planifiés, jour par jour avec leur nombre,
précédés de ceux dont le jour est passé sans acte réalisé ; un clic
ouvre le carnet.
Les facteurs de risque ne sont pas connus du logiciel ; la liste ajoute
cependant, sous leur propre intitulé, les personnes de moins de 65 ans
dont un traitement évoque un groupe visé pour la grippe et le COVID-19
(insuline ou antidiabétique, traitement de fond de l'asthme ou de la
BPCO, antiagrégant ou anticoagulant, immunosuppresseur, antirétroviral),
avec le médicament en cause. L'indication est à confirmer avec le
patient. Ouvrir un dossier depuis la liste prépare le carnet : le vaccin
de la campagne et le lot en stock sont déjà renseignés. « Imprimer la
liste » sort la liste du vaccin choisi, avec une colonne pour noter
l'issue de chaque appel.
Au clavier, une flèche active la liste et la parcourt ; Entrée ouvre
alors le carnet et les touches 1 à 4 notent l'issue de l'appel, dans
l'ordre des boutons. La saisie dans un champ désactive la liste.

**Doses de la saison.** Par vaccin : le jour, les sept derniers jours et
la saison, lus dans tous les carnets ; puis la courbe hebdomadaire du
vaccin choisi. « Bilan de la saison » imprime les doses par vaccin et
par tranche d'âge à la date de l'injection, puis semaine par semaine.
« Registre du jour » imprime les doses inscrites aux carnets dans la
journée, avec le patient, le lot, le site et l'opérateur.
« Exporter la saison » écrit les doses de la saison dans un fichier CSV
(dossier `exports` à côté de la base) : date, patient, vaccin, dose,
lot, site et opérateur. L'export est tracé dans le journal d'accès.

**Lots reçus.** Chaque lot est saisi à réception : vaccin, spécialité,
numéro, péremption (`06/2027` suffit, le dernier jour du mois est
retenu) et nombre de doses. Le stock restant n'est jamais décompté à la
main : il se calcule à partir des doses inscrites au carnet avec ce
numéro. Une dose corrigée ou supprimée au carnet se retrouve donc dans
le stock. Un lot est signalé périmé, épuisé, proche de la péremption
(moins de 30 jours) ou en stock bas (moins de 5 doses).

Sur un lot de Comirnaty, « Flacon ouvert » note la première ponction
d'un flacon multidose : la ligne donne l'heure limite
d'utilisation, 12 heures plus tard, et le temps restant.

Dans le carnet d'un dossier, le choix d'un vaccin propose le lot en
stock qui périme le premier. Une remarque s'affiche sous la saisie quand
la dose appelle une vérification : âge minimal de la vaccination à
l'officine, délai depuis la dernière dose de COVID-19, préférence pour
Efluelda ou Fluad à 65 ans et plus, Arexvy et COVID-19 le même jour,
vaccin vivant pendant la grossesse, intervalle entre deux doses,
seconde dose de grippe dans la même saison. La
remarque n'empêche pas l'enregistrement.

# Couverture des délivrances

Touche **F10**, bouton **Couverture** de la barre des médicaments ou des actions d'un
dossier (la couverture s'ouvre alors sur ses traitements), ou « Aller
à… » (couverture, QSP, boîte). Une ligne par délivrance : unités par boîte,
nombre de boîtes, posologie, jour de délivrance et, si elle est
prescrite, la durée de traitement en jours. La ligne peut aussi se saisir
en une fois : `mirtazapine 15 (28) 1-0-1`, ou `metformine 500 (2x90) 1-0-1`
pour deux boîtes. Avec un dossier ouvert, « Reprendre les traitements
du dossier » remplit le nom, le dosage, la posologie, la dernière
délivrance, la durée et les unités par boîte s'il est noté. Saisi sur une
ligne reprise du dossier, le nombre d'unités par boîte y est enregistré
à la sortie du champ et sert aux reprises suivantes.

**Plusieurs délivrances d'un même médicament** (même libellé) se
rangent sur une seule rangée ; « Renouveler » ajoute sous une ligne la
même délivrance, datée d'aujourd'hui. Une délivrance faite avant la fin de la
précédente attend son tour : sa période commence quand la précédente
finit. Une délivrance faite après laisse des jours sans traitement,
en rouge, comptés sous la saisie avec la part des jours couverts depuis
la première délivrance. Une délivrance attendue et non faite est marquée
en rouge jusqu'à aujourd'hui, sauf si la durée prescrite était écoulée.

**L'échelle de temps.** Mois en haut, lundis et week-ends dans le fond,
jours ou semaines en bas selon le zoom. Plein : période couverte.
Teinte claire : délivrances qui complètent la durée prescrite.
Triangle : jour de délivrance. Trait noir : délivrance suivante. La
molette zoome autour du pointeur, un glisser déplace, un double clic
montre toute la période ; le menu « Période » propose 1, 3 ou 6 mois.
Sous l'échelle, la lecture du jour pointé : rang du jour dans sa
délivrance et unités en main ce matin-là, ou jours sans traitement.
Un clic épingle un jour, que les touches fléchées gauche et droite
déplacent ; un second clic le libère.

**Aligner les renouvellements.** Le bouton « Aligner » propose le plus
tardif des jours de délivrance suivante (un autre jour peut être
saisi) et indique, pour chaque médicament, la quantité à délivrer en
plus pour tenir jusqu'à ce jour, en unités et en boîtes de la dernière
délivrance : en pointillé sur l'échelle. La décision de délivrer
revient au pharmacien, dans le cadre de la prescription.

« Imprimer » sort l'échelle sur toute la période, avec la lecture de
chaque médicament et l'alignement s'il est demandé. La posologie doit
dire la quantité de chaque prise (1-0-1, ½-0-½) : « 2 fois par jour »
ne permet pas de calculer une durée. Une journée entamée n'est pas comptée. Les
quantités en main supposent la posologie suivie depuis la délivrance.
Sur un écran court, « Saisie » et « Échelle de temps » se montrent
l'une après l'autre.

# Mesures et conseils

Ouvert par le bouton **Mesures et conseils** de la page des médicaments,
ou par « Aller à… ». Trois pages, chacune avec sa fiche imprimée remise
au patient : les relevés de la page, puis les conseils, que l'officine
peut réécrire dans « Textes ». Le dossier ouvert reçoit ce qui est
enregistré ; sans dossier, les calculs et l'impression restent
disponibles.

**Compression.** L'article (chaussettes, bas-cuisse, collant) fixe les
points à mesurer ; chaque point indique son repère au survol. La
vérification signale les points manquants, une mesure plus faible
au-dessus qu'au-dessous (repère ou saisie à reprendre) et l'écart entre
les deux jambes. Les grilles de tailles ne sont pas livrées : elles
diffèrent d'un modèle à l'autre. L'officine saisit celles des modèles
qu'elle délivre, une taille par ligne (`2 : cB 20-22 ; cC 30-37`), et la
page lit la taille de chaque jambe. Le renouvellement compte les paires
délivrées pour la même classe et la même taille : 2 par période de
6 mois, 4 par an au plus.

**Nutrition orale.** L'évaluation lit les critères de la HAS (2019 avant
70 ans, 2021 à partir de 70 ans) : un critère phénotypique et un critère
étiologique posent le diagnostic, et un seul critère de sévérité suffit
à la dénutrition sévère. Les critères de prise en charge des
compléments par la LPP sont affichés à part : ils ne sont pas les mêmes.
Le plan additionne les apports des produits choisis contre l'objectif de
400 kcal et/ou 30 g de protéines par jour. Les produits livrés viennent
des pages des fabricants ; l'équipe corrige leurs teneurs d'un clic
droit.

**Protections périodiques.** Les droits (moins de 26 ans, ou C2S), la
période annuelle ouverte par la première délivrance, les codes
individuels des fabricants et les règles de facturation. Pour une
culotte, le tour de bassin et la stature sont notés à la délivrance :
la grille de tailles du fabricant s'y réfère, et la fiche remise les
reprend.

# Carnets de suivi du patient

Six feuilles d'automesure à domicile : pression artérielle, glycémie,
poids, débit expiratoire de pointe, INR, douleur.

**Chaque feuille porte le protocole de mesure.** Une pression prise
après un café, debout, ou une glycémie notée de mémoire ne sont pas
interprétables. Chaque feuille comporte donc quatre parties : technique de
mesure, objectif, signes justifiant un appel immédiat, et grille de
relevés.

**Objectifs.** Lorsque l'objectif est individuel —
glycémie, zone d'INR, meilleure valeur personnelle de DEP —, la feuille
l'indique et laisse la ligne à remplir. Seule l'automesure tensionnelle
porte une cible chiffrée, issue d'une recommandation publique.

**Adaptation de dose.** La feuille n'en propose aucune ; elle indique
qui contacter et quand.

Le texte de chaque feuille est modifiable, comme tout document imprimé
au nom de l'officine.

# Registre des stupéfiants

Fichier dédié, chiffré comme la base, **sans suppression possible**. Une
ligne erronée reste écrite ; une ligne d'annulation la désigne et en
annule exactement l'effet.

Le solde comporte deux valeurs : le stock délivrable, et les retours
patients en attente de destruction.

Une case vide n'est pas un zéro. Une feuille de comptage ne porte que
les produits effectivement comptés.

Une ligne porte le **numéro de dossier**, jamais le nom : le registre
est imprimé et consultable au comptoir.

Le numéro d'ordonnancier est attribué à l'enregistrement et jamais
réattribué : une ligne annulée conserve le sien.

**Numérotation continue.** Elle ne repart pas au 1er janvier : un numéro
désigne une seule délivrance, condition pour le reporter sur
l'ordonnance.

Reprise d'un registre papier : Options › Base, « Premier numéro
d'ordonnancier », une seule fois. Un numéro inférieur saisi après coup
est sans effet.

## Vigilance

L'onglet « Vigilance » analyse le registre selon trois critères :
rapprochement des délivrances, multiplicité des prescripteurs,
augmentation des quantités. Ce sont **des signalements, non des
conclusions** : chacun cite les lignes en cause, à vérifier.

Le registre ne porte ni dose quotidienne ni durée prescrite : la date de
fin théorique d'un traitement n'est donc pas calculée.

La durée maximale de prescription de la famille sert uniquement à
**exclure** : au-delà, deux ordonnances ne peuvent pas se chevaucher.
Elle ne sert jamais à estimer un rythme — une ordonnance de sept jours
sous un plafond de vingt-huit paraîtrait quatre fois trop lente, et
chaque délivrance légitime serait signalée.

En deçà de trois délivrances antérieures, aucun signalement : pas de
rythme de référence.

# Liens d'une monographie

Dans le texte d'une fiche, le nom d'une autre fiche de la base — nom de
spécialité ou DCI — est un lien : un clic l'ouvre. La molécule écrite
sans son sel l'est aussi, quand elle ne désigne qu'une molécule de la
base : « valproate » ouvre la fiche du valproate de sodium,
« olmésartan » celle de l'olmésartan médoxomil. « Ténofovir », qui
désigne deux molécules, reste en texte simple, comme un nom de forme
locale : le « fluorouracil » d'une fiche de cancérologie est la
perfusion, et non la crème.

Sous les interactions, **« Cytochromes et transporteurs »** écrit ce que
la table des cytochromes sait de la fiche, voie par voie — « CYP3A4 —
substrat, voie principale » — avec les fiches de la base qu'elle y
rencontre, cliquables : ce qui la freine et ce qui l'accélère pour un
substrat, ce qu'elle fait monter ou baisser pour un inhibiteur ou un
inducteur. Pour une prodrogue, la section indique ce qui diminue ou augmente son effet. Les
plus puissants d'abord, douze noms au plus et le compte des autres ;
une molécule vendue sous plusieurs noms n'est citée qu'une fois. Une
fiche absente de la table, ou une forme locale, n'a pas cette section.

# Pharmacocinétique et pharmacodynamie

La colonne technique d'une fiche porte un tableau : biodisponibilité,
pic plasmatique, liaison aux protéines, volume de distribution, fraction
éliminée inchangée, demi-vie, cible, délai et durée d'action, marge
thérapeutique étroite.

**Origine des valeurs.** Une valeur provient de la fiche — extraite du
texte, phrase source au survol — ou de l'officine, qui la saisit avec
« Compléter… » et **sa source** (RCP, section 5.1 ou 5.2) : une valeur
sans source est refusée. Une valeur non documentée s'affiche « non
chiffré ». Une valeur sourcée prime sur l'extraction du texte, s'affiche
sur la monographie et s'imprime avec elle.

En réseau d'officines, les valeurs sourcées sont partagées ; celles des
autres officines s'affichent avec leur source et leur nom, après celles
de l'officine et avant l'extraction du texte.

# Versions d'une fiche

Chaque modification d'un champ crée une **version** : auteur, date,
valeur, valeur remplacée. « Historique… », dans la colonne technique,
les liste champ par champ, la plus récente en premier ; « Revenir à
cette version » crée une nouvelle version avec l'ancienne valeur. Aucune
suppression.

En réseau d'officines, les versions sont partagées : une fiche corrigée
dans une officine est corrigée dans les autres — **si leur fiche porte
encore la valeur remplacée**. Sinon, la version est « à arbitrer » :
« Adopter » l'applique, « Garder la mienne » la refuse ; aucun écrasement
silencieux. Chaque officine peut revenir à toute version de tout champ.
Les notes de l'équipe ne sont pas partagées, et aucune fiche ne porte de
donnée patient.

# Ruptures

Une rupture se déclare sur la fiche : « Signaler une rupture », dans la
colonne technique, puis « Rupture levée » au retour du produit. Une
rupture signalée s'affiche en tête de fiche, avec les substitutions
pratiquées par l'équipe.

« Noter une substitution… » enregistre le produit délivré à la place et
l'issue : maintenu, refusé par le patient, refusé par le prescripteur,
retour au produit initial. Les propositions viennent d'abord de la même
molécule, puis de la même classe. **Aucun patient n'est enregistré** :
un produit, un substitut, une date et des initiales.

La vue **Ruptures** (« Aller à… ») regroupe les ruptures en cours et
l'historique complet, y compris celui des officines du réseau, avec
leur nom.

**Une substitution pratiquée n'est pas une équivalence.** Quand le
substitut est d'une autre classe — dermocorticoïde modéré pour un
dermocorticoïde fort —, la ligne le signale ; dosage, forme et patient
se vérifient sur l'ordonnance. Une rupture signalée depuis plus de
quatre-vingt-dix jours sans mise à jour n'est plus « en cours ».

Le journal n'est pas modifiable ; une erreur se retire, et le retrait
reste enregistré.

# Réseau d'officines

Les officines d'un groupement peuvent partager ce journal : « Réseau
d'officines… », dans la vue Ruptures. **Sont partagés** les ruptures et
les substitutions, les valeurs de pharmacocinétique sourcées, et les
versions des fiches, des préparations, des protocoles, des lignes de
TROD et des vaccins du catalogue — **jamais un patient**, un dossier, le
registre ou la caisse —, chiffrés et signés, sous une clé propre au
réseau.

Une officine **crée** le réseau, puis **invite** les autres : « Inviter
une officine… » ouvre une invitation de 5 minutes et affiche un **code
d'invitation** (`K7QM-2XPA-9D3F-7H1Q@192.168.1.20:7742`), à transmettre
par téléphone ou en personne. L'autre officine le colle dans
« Rejoindre » ; rien d'autre à saisir ni à comparer. Code à usage
unique : un code erroné ferme l'invitation, une officine sans code est
refusée. Les deux officines doivent être à jour.

**Connexion au lancement.** Application ouverte, les officines appairées
se connectent automatiquement : chacune répond aux siennes (Options ›
Base, « Répondre aux officines appairées »), et une officine appairée
présente sur le réseau local est jointe dès qu'elle s'annonce, sans
adresse à saisir. Les autres ne reçoivent aucune réponse. Entre deux
sites : une officine joignable de l'extérieur (routeur, VPN) déclare son
adresse dans la fenêtre du réseau (« Adresse externe ») ; ses officines
appairées l'apprennent et s'y connectent au lancement.

**Les officines voisines.** Chaque officine s'annonce sur le réseau
local : nom, ville et empreinte, rien d'autre (Options › Base, « Annoncer
l'officine »). La carte des connexions les place sur l'arc « À portée ».
Chaque officine y porte **ses initiales dans une couleur tirée de son
empreinte**, identiques sur chaque poste. Un clic propose **Inviter cette
officine** (l'invitation apparaît sur sa carte), **Créer un réseau et
l'inviter** quand l'officine n'en a pas encore, ou **Rejoindre son
réseau** quand elle invite. Nom déclaré, non vérifié : ces invitations
n'ont pas de code d'invitation, et les deux officines comparent par
téléphone un code de cinq groupes. Un code identique des deux côtés
garantit l'absence d'intermédiaire.

La **synchronisation** se fait par un bouton, et à la fermeture si le
poste est configuré ainsi (Options › Base). Deux modes : connexion
directe aux officines qui acceptent les connexions, ou **dossier
d'échange** — partage réseau, dossier synchronisé — où chaque officine
dépose ses enregistrements et lit ceux des autres, sans ouverture de
port sur la box. Le dossier d'échange ne contient que des données
chiffrées.

Retirer une officine arrête ses envois ; ses envois antérieurs restent
au journal.

**Plusieurs réseaux.** Une officine peut être de plusieurs réseaux à la
fois — son groupement, une garde de secteur, ou un simple lien avec une
autre officine, qui est un réseau à deux. « Nouveau réseau… » en crée un
(puis « Inviter une officine… » y fait entrer chacune, avec son code
d'invitation) ; « Rejoindre un autre réseau » rejoint celui d'une officine
qui invite, sans quitter les autres. Un onglet par réseau : ses
officines, son dossier d'échange (plusieurs réseaux peuvent partager le
même dossier sans se lire), et **ce qui y est partagé** — ruptures,
versions des fiches et du contenu, valeurs sourcées — réglé réseau par
réseau. Les messages et les fichiers vont toujours aux seules officines
choisies, par le réseau qui les réunit. « Quitter ce réseau » retire sa
clé et ses enregistrements ; les autres réseaux ne sont pas modifiés. La carte
des connexions montre chaque réseau sur son arc, avec son nom.

**Suivi de chaque officine.** Sous chaque officine appairée, la liste
indique le nom sous lequel elle signe, le nombre d'enregistrements reçus
d'elle et la date des dernières nouvelles, qu'elles soient arrivées par
une connexion directe ou par le dossier d'échange. Pour une officine à
adresse, la dernière connexion directe réussie, ou, en rouge, l'heure du
dernier échec et sa raison : pas de réponse à l'adresse (poste éteint,
port fermé ou adresse hors d'atteinte), officine qui n'est pas celle
appairée, ou clé de réseau différente. Le nom sous lequel elle signe sert
d'invite au champ du nom. « Essayer », à côté d'une officine à adresse,
contacte cette officine seule et indique aussitôt si elle répond, sans
synchroniser les autres.

Dans « Connexions », **« Derniers reçus »** liste ce que les autres
officines ont envoyé — ruptures, levées, substitutions, versions de
fiches, de préparations et de protocoles —, daté et signé du nom de
l'officine, le plus récent d'abord.

Quand la synchronisation automatique apporte une **nouvelle rupture**,
la barre d'état l'annonce dix minutes (« Réseau : Diprosone en
rupture ») ; un clic ouvre les ruptures.

**Au tableau de bord**, « Ruptures en cours » liste les produits en
rupture — signalés par l'officine ou par le réseau —, depuis quand, par
combien d'officines, et le substitut le plus pratiqué. Un clic ouvre la
fiche. Le panneau n'apparaît que s'il y a une rupture en cours.

**Le codex et les protocoles sont aussi partagés**, selon les mêmes
règles que les fiches : une préparation modifiée ou créée dans une
officine l'est chez les autres, champ par champ ; un protocole est
partagé avec son sujet et son **arbre complet**. Une version reçue ne
s'applique que si l'entrée locale porte encore la valeur remplacée ;
sinon elle est à arbitrer, et « Historique… » — sur une préparation
comme sur un protocole — propose « Adopter » ou « Garder la mienne »,
ainsi que le retour à toute version. Dans un groupe de postes, le poste
de référence versionne les modifications des autres postes.

**Les lignes du protocole TROD et le catalogue des vaccins** voyagent de
même. Pour une ligne de TROD : posologies, bornes d'âge, sexe, grossesse
et précaution ; pour un vaccin, sous son libellé : son code et son
schéma. Une correction faite dans une officine s'applique chez les
autres, un ajout y est créé ; l'ordre reste celui de chaque officine.
« Historique… », dans l'éditeur des lignes comme dans celui du
catalogue, montre les versions.

Les valeurs de pharmacocinétique sourcées sont partagées de la même
façon, avec leur source. La barre de comptoir (F9) affiche une rupture
en priorité, avec les substitutions pratiquées.

# Postes de l'officine

Chaque poste peut disposer de **sa propre base** et recevoir toutes les
écritures des autres : dossiers, registre, caisse, planning, agenda,
fiches, réglages. « Postes de l'officine… », dans Options › Base. Les
échanges sont chiffrés sous une clé réservée aux postes de l'officine ;
le réseau d'officines a sa propre clé et n'accède à aucune de ces
données.

**Relier deux postes, pas à pas.** Un poste hors groupe indique dans
Connexions les postes détectés et la marche à suivre :

- Premier poste de l'officine : « Fonder le groupe sur cette base ». Sa
  base est conservée, avec sa numérotation des dossiers.
- Sur un poste du groupe : « Postes de l'officine… » puis « Inviter un
  poste… » (ou, sur la carte, clic sur le poste non relié puis « Inviter
  ce poste »). Un code de seize caractères s'affiche, valable 5 min.
- Sur le poste à relier : l'invitation apparaît d'elle-même dans
  Connexions. Y saisir le code — tirets et majuscules facultatifs —,
  puis « Rejoindre… ».

**Rejoindre remplace le contenu du poste** par les données du groupe :
faire une copie au préalable si nécessaire. Un code erroné se corrige
et se ressaisit tant que l'invitation est ouverte. Si elle n'apparaît pas
(autre réseau, pare-feu), coller le code complet, adresse comprise,
affiché sous le code.

Sur le réseau local, les postes se découvrent et se synchronisent
**automatiquement**, quelques secondes après chaque écriture. S'ils ne
se voient pas (réseau cloisonné, deux sites), un dossier d'échange ou
des adresses saisies dans Options › Base assurent la liaison ; le bouton
« Synchroniser » et la synchronisation à la fermeture restent
disponibles.

Deux postes n'attribuent jamais le même numéro de dossier : chacun a sa
plage (le deuxième poste commence à 100 000). Une modification reçue ne
s'applique que sur la valeur qu'elle remplace : deux postes modifiant
deux champs d'un même dossier ne se gênent pas ; le même champ modifié
des deux côtés est « à arbitrer » — « Garder la mienne » ou « Prendre la
leur ». Aucun écrasement silencieux.

**Une seule numérotation d'ordonnancier** : seul le poste de référence
numérote. Une délivrance saisie sur un autre poste est « en attente »
jusqu'à sa réception, quelques secondes sur le réseau local. Le
registre, la caisse et les journaux restent en ajout seul : une
modification reçue pour eux est refusée, et le refus est affiché. Le
poste de référence installe aussi le contenu livré avec les mises à
jour.

**L'officine** (nom, adresse, pharmacien, n° AM, équipe, horaires) se
corrige depuis n'importe quel poste, dans Options › Officine. Deux postes
qui la corrigent en même temps ne perdent rien : ce que chacun a changé
se réunit ; un même champ changé des deux côtés est signalé, la valeur
saisie reste affichée, et « Enregistrer » la garde. Après un
enregistrement refusé, « Remplacer par ces valeurs » enregistre
l'officine telle qu'affichée, pour tous les postes, sans tenir compte de
ce qu'un autre poste y a écrit.

Les pièces scannées restent sur le poste qui les a numérisées.

**Téléphones de l'équipe.** « Inviter un téléphone… » relie le téléphone
Android d'un membre de l'équipe — sur le téléphone, onglet Poste,
« Scanner le code » lit le code QR affiché, ou saisir le code : il lit les fiches, l'agenda, le
planning et la messagerie, écrit à l'équipe, ajoute ou retire une
entrée d'agenda, corrige une fiche. Il ne reçoit jamais la clé des
dossiers, du registre, de la caisse ni des réglages : il transporte ces
données chiffrées sans pouvoir les lire. Sa base s'ouvre avec le
verrouillage du téléphone et se referme après cinq minutes hors de
l'écran. Il se synchronise tant qu'il est à l'écran
(à l'ouverture, puis chaque minute) et après chaque écriture, jamais en
arrière-plan ; il n'est jamais poste de référence.

**Hors de l'officine**, un téléphone joint un poste « Joignable depuis
Internet » (Options › Base, éteint par défaut) : le poste écoute aussi
en IPv6, demande au routeur d'ouvrir son port (UPnP) et communique ses
adresses aux téléphones. Seuls les postes du groupe obtiennent une
réponse. À défaut, le dossier d'échange ou des adresses saisies sur le
téléphone.

**Sans rien de joignable** (pas d'IPv6, adresse partagée par
l'opérateur) : « Se faire relayer par le réseau » sur un poste, et une
officine amie du réseau qui coche « Relayer les téléphones des officines
du réseau ». Le poste s'y tient en attente ; le téléphone le joint par son
intermédiaire. L'officine relais ne lit ni ne conserve rien : la
conversation, chiffrée de bout en bout, ne fait que transiter.

# Messagerie

La vue **Messages** (barre d'état, « Aller à… » : « messages ») réunit
les conversations de l'équipe. Elles passent d'un poste à l'autre de
l'officine, et aux téléphones de l'équipe, chiffrées sous la clé des
postes — à part des dossiers.

Chacun lit et écrit **en son nom** : le nom choisi dans le volet
(« Opérateur »). Sans nom choisi, seules les conversations de toute
l'équipe s'affichent. La barre d'état compte les messages non lus.

« Nouvelle conversation » s'adresse à **des collègues**, à **un groupe**
ou à **toute l'équipe**, avec un titre facultatif. Écrire aux mêmes
collègues sans titre rouvre leur conversation plutôt que d'en ouvrir une
seconde. Une étoile devant un collègue en fait un **contact favori** :
en tête des destinataires, et un bouton en haut de la liste des
conversations pour lui écrire d'un clic.

« Groupes… » crée des **groupes de collègues** (« Préparateurs »,
« Garde »). Une conversation adressée à un groupe en suit la
composition : un membre ajouté la voit, un membre retiré ne la voit plus.

Un message peut **lier le dossier ouvert** : son nom s'affiche dans le
fil, et un clic ouvre le dossier. Ctrl+Entrée envoie ; un message envoyé
ne se modifie pas.

**Avec d'autres officines.** Quand l'officine est d'un réseau, une
nouvelle conversation peut s'adresser à des **officines** plutôt qu'à
l'équipe. Chaque message est chiffré pour les seules officines choisies :
les autres officines du réseau le transportent sans pouvoir le lire. Il
part tout de suite, par une synchronisation du réseau. Une officine qui
n'a pas encore annoncé sa clé le reçoit après sa prochaine
synchronisation. Lier le dossier ouvert à un message pour une autre
officine demande une **confirmation** : le message porte alors le nom et
la date de naissance du patient, et l'envoi est inscrit au journal des
accès. Les officines favorites (étoile) ont leur bouton en tête des
conversations.

**Fichiers.** « Joindre un fichier… » ajoute un fichier au message
(5 Mo au plus), chiffré comme lui. Il voyage en morceaux ; dans le fil,
il s'enregistre d'un clic une fois entier, après vérification de son
empreinte ; un fichier incomplet ou altéré ne s'enregistre pas. Le nom
reçu ne détermine pas l'emplacement d'enregistrement. Un fichier pour une autre
officine demande la même confirmation qu'un patient nommé.

# Connexions

L'icône à trois nœuds, en bas à droite de la barre d'état, indique le
nombre de postes du groupe joignables (« 2/3 poste(s) ») et
l'appartenance à un réseau d'officines ; un clic ouvre la vue
« Connexions ». Bleue quand tous les postes répondent, rouge après un
échec de synchronisation.

**La carte.** « Carte », dans la vue, dessine les connexions : ce poste
au centre, les autres postes de l'officine autour, les officines du
réseau au-delà. Chaque lien a la forme de son état : trait plein, la
dernière connexion directe a réussi ; trait fin, des nouvelles sans
connexion directe ; tirets, la dernière tentative a échoué ; pointillés,
rien entendu. Un clic sur un nœud le détaille à côté et propose ses
gestes : pour une officine, **Écrire**, **Envoyer un fichier…** (la
conversation s'ouvre, le fichier joint au prochain message) et
**Essayer** (une connexion directe, quand elle a une adresse) ; pour un
poste, le synchroniser ; pour ce poste, tout synchroniser, relier un
poste ou inviter une officine. Les **officines voisines** hors réseau
sont sur l'arc « À portée » : cercle pointillé, sans lien, pastille au
coin quand elle invite. Un **poste non relié** (BPM-Caddy hors groupe sur
le réseau local, peut-être un poste de l'officine) est un carré en
tirets sur le cercle des postes. « Ce poste » montre l'insigne vu par les
autres officines : initiales, couleur, nom et ville.
Les **officines présentées par le réseau** (qui détiennent sa clé et
déposent au dossier d'échange sans avoir été ajoutées ici) apparaissent
après une synchronisation, en cercle creux sur l'arc de leur réseau.
Leurs envois ne sont ni gardés ni lus avant ajout : « Ajouter » (deux
clics) ou « Écarter ». Alerte si le nom est déjà porté par une officine
ajoutée : vérifier l'empreinte par téléphone. Une officine retirée n'est
plus proposée. Une officine appairée qui se donne le nom d'une autre, ou
celui de cette officine, s'affiche suivie du début de son empreinte ; le
nom saisi ici dans la fenêtre du réseau fait foi.

**Connexion au lancement** : les postes du groupe se découvrent et se
synchronisent sur le réseau local ; le réseau d'officines se synchronise
au lancement puis toutes les quinze minutes (Options › Base). Un poste
isolé écoute seulement : la vue liste les postes détectés. Rejoindre
demande le code d'invitation affiché par le poste qui invite.

Quatre panneaux : postes (en ligne, hors ligne, retirés), réseau
d'officines (officines appairées, dernière et prochaine
synchronisation), éléments à arbitrer (entre postes, et versions de
fiches reçues d'autres officines), et activité depuis le lancement.
« Tout synchroniser » contacte immédiatement les postes joignables et
les officines du réseau.

# Caisse

Le comptage se fait en centimes entiers. L'écart avec la recette
attendue est **affiché, jamais compensé** : le compte n'est pas recalculé
à partir de la recette attendue.

Sans recette attendue, pas d'écart : la ligne reste vide.

La recette encaissée est le tiroir **moins le fond de caisse à
l'ouverture**, plus carte et chèques. Le fond est prérempli avec celui
du dernier comptage et se corrige s'il a changé.

Un recomptage crée une **seconde ligne**, non une correction : le mois
retient le dernier comptage de chaque soir et affiche les autres barrés.

# Documents imprimés

Chaque document imprimable a un modèle modifiable — bouton « Modèles… »
de la barre du haut. Les modèles sont écrits en Typst ; les
`{{MARQUEURS}}` acceptés sont listés dans l'éditeur, et l'aperçu utilise
le même rendu que l'impression.

Les textes imprimés au nom de l'officine se modifient dans la vue du
document ou dans l'écran « Textes imprimés ». Les modifications sont
enregistrées dans la base et valent pour tous les postes.

Les **libellés de l'interface** — boutons, invites, infobulles — se
consultent et se modifient dans « Libellés ». Ils sont enregistrés dans
un fichier à côté de la configuration, donc par poste, et s'appliquent
au prochain lancement.

Chaque modification garde la trace du texte d'origine. Si le texte livré
change, la modification est signalée pour relecture.

# Base de données

Un fichier chiffré, partageable entre postes. Le registre des
stupéfiants et les pièces scannées ont chacun leur fichier, à côté.

Sauvegardes quotidiennes, en nombre fixé ; « Copier la base… » copie les
trois fichiers. Le changement de mot de passe rechiffre les trois.

La suppression de pièces ne libère pas l'espace : seul « Compacter » le
fait.

Les données de l'officine — identité, équipe, horaires — sont
enregistrées dans la base et valent pour tous les postes. Le reste de
`config.toml` est propre au poste. Les modifications d'un autre poste
s'appliquent automatiquement, sans reverrouillage, sauf pendant qu'une
boîte de dialogue est ouverte.

**Export en un seul fichier.** « Exporter en un fichier… » regroupe la
base, les pièces et le registre dans un paquet chiffré du même mot de
passe, pour transport ou archivage. « Importer un paquet… » les restaure
dans un dossier **vide** : rien n'est écrasé, et la base y pointe au
redémarrage.

# Annuaire des prescripteurs

Options › Base, « Importer un annuaire… » : un fichier de prescripteurs
exporté de l'annuaire santé ou du logiciel de l'officine. Les colonnes
sont identifiées **par leur nom** et non par leur position, qui varie
d'une version à l'autre ; un fichier sans colonne de nom est refusé,
avec la liste des colonnes trouvées.

Un import remplace l'annuaire précédent : c'est un état à une date, et
un import cumulatif conserverait les praticiens partis.

Dans le champ « prescripteur » du registre, deux lettres suffisent à
proposer les praticiens correspondants — par nom, ville, spécialité ou
numéro RPPS. **Suggestion seulement** : la saisie reste inchangée tant
qu'aucune ligne n'est choisie, et un prescripteur absent de l'annuaire
se saisit librement.

**Mise à jour depuis une adresse.** L'officine peut indiquer une
adresse — export de son logiciel, fichier sur son serveur, jeu public
décompressé — dans `[prescribers] source_url` de `config.toml` ; un
second bouton apparaît alors : « Mettre à jour depuis l'adresse… ». Sans
adresse, **le bouton n'apparaît pas** et aucune connexion n'est ouverte.

Aucune requête sans action de l'utilisateur, aucune donnée envoyée, et
adresse obligatoirement en `https`. Une archive est détectée et refusée,
avec la consigne de la décompresser.

Le RPPS est contrôlé par sa clé (onze chiffres, le dernier de contrôle).
Un numéro invalide est **conservé** et compté à part, pour vérification
par l'officine.

# Compteurs d'usage

Quelques totaux : dossiers ouverts, fiches consultées, documents
imprimés, lignes au registre, nombre de journées. Consultables dans
**bpm-audit**, la fenêtre d'audit de l'officine.

Le compteur « écritures concurrentes signalées » indique le nombre de
rechargements provoqués par l'écriture préalable d'un autre poste. Il
signale les cas où deux personnes travaillent sur la même donnée au même
moment.

Aucun envoi : les compteurs restent dans la base chiffrée de l'officine
et se consultent dans bpm-audit.

Ils mesurent l'usage du logiciel, jamais celui d'une personne : aucun
opérateur, aucune initiale.

Actifs par défaut, désactivables dans Options › À propos, poste par
poste. La désactivation arrête le comptage sans rien effacer ;
l'effacement est un bouton distinct, avec double confirmation.

# Journal des accès

Le journal indique qui a ouvert quel dossier, et quand ; il se consulte dans bpm-audit.
Contrairement aux compteurs, le journal nomme la personne : il répond
à « qui a consulté ce dossier, le 12 mars ».

Une ligne porte le **numéro** de dossier, jamais le nom. Sans initiales
déclarées dans Options › Interface, la ligne porte un tiret.

Trois actions sont tracées : ouverture d'un dossier, export, et lecture
de la liste des patients depuis la console (noms compris). L'impression
n'est pas tracée.

Durée de conservation : `[audit] keep_days` de `config.toml`, un an par
défaut ; au-delà, les lignes sont purgées à l'ouverture de session, et
la purge est elle-même journalisée. `0` désactive la purge.

# Fenêtre d'audit

**bpm-audit** est un second programme, livré avec l'application. Il
demande le mot de passe de la base (ou le lit dans le trousseau) et
affiche quatre volets sur la période choisie — 7, 30, 90 ou 365 jours :
activité, accès aux dossiers, usage du logiciel, contrôles. Il n'écrit
rien dans la base. « Copier le rapport » et « Enregistrer le rapport… »
produisent le relevé texte, identique à la commande ci-dessous.

Les deux programmes partagent le même code d'accès à la base.

# Rapport d'audit

`bpm-caddy audit` écrit ce relevé sur la sortie standard, sans ouvrir de
fenêtre.

    bpm-caddy audit --jours 90 > audit-septembre.txt

Trois sections. **Activité** : actes par nature et par opérateur, lignes
au registre, caisses comptées et écart cumulé (sur les soirs où il est
calculé). **Accès** : contenu du journal des accès sur la période.
**Conformité** : libellés de classe non reconnus par le référentiel,
textes modifiés rendus obsolètes par la version livrée, fiches sans DCI
ou sans classe, produits du registre à recompter, locations dont le
renouvellement est dépassé.

Seuls les opérateurs sont nommés : un dossier est désigné par son
numéro, comme au registre.

Aucune saisie ni fenêtre : le mot de passe vient de
`BPM_CADDY_PASSWORD` ou du trousseau du système, ce qui permet une
exécution planifiée. Linux uniquement ; sur les autres systèmes, la
commande le signale.

# Console

Interrogation libre de la base, en quelques lignes de script. Les
fonctions disponibles sont détaillées plus bas, dans « L'API de la
console ».

**Coloration syntaxique** en cinq catégories : commentaires, chaînes,
nombres, mots-clés et fonctions reconnues par la console. Un nom de
fonction non coloré est inconnu du moteur : une faute de frappe est ainsi
repérable avant l'exécution.

**Complétion.** Dès le début d'un mot, une liste s'ouvre sous le
curseur : flèches haut et bas pour parcourir, tabulation pour insérer,
Échap pour fermer. Rien ne s'ouvre sur un mot complet ni sur une ligne
vide.
