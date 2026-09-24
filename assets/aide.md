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
clavier. Cette liste est tenue par l'application et toujours à jour.

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
protocoles, les dispositifs, le registre, les carnets de suivi, les
scripts, les outils de calcul et les textes imprimés. Saisir le nom, un
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
fenêtre « ? » (F12) liste aussi les outils sous les raccourcis, avec
leur usage ; un clic ouvre l'outil.

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
dossier** : la date de naissance, saisie à la création. Le rein modifie
la dose, l'âge modifie le choix. Les lignes viennent de la liste
française de Laroche, des critères STOPP/START et des critères de Beers ;
chacune indique le risque et l'alternative. Aucun arrêt brutal : l'arrêt
d'un psychotrope chez le sujet âgé expose davantage que sa poursuite, et
le remplacement se prépare avec le prescripteur.

**Une forme locale n'est pas traitée comme la voie générale.** Collyres,
pommades, gels et pulvérisations nasales ne reçoivent ni palier rénal,
ni niveau de grossesse, ni demande d'examen : les tables sont indexées
par molécule, et une même molécule n'a pas les mêmes effets selon la
voie. La ciclosporine en collyre ne génère aucune interaction ; le
lithium en gel ne demande pas de lithiémie. Les précautions de la voie
locale figurent sur la fiche du produit.

La dernière lecture recherche les interactions par voie enzymatique.
Elle **ne connaît que sept cytochromes** : ni la glycoprotéine P, ni les
transporteurs hépatiques, ni les effets additifs — deux sédatifs
n'interagissent sur aucune enzyme et leurs effets s'additionnent. Une
absence d'interaction enzymatique n'est pas une absence d'interaction ;
les lignes absentes de la table sont **listées**.

Une prodrogue s'y lit à l'envers, et la ligne le précise : inhiber
l'enzyme qui produit le métabolite actif du clopidogrel, de la codéine,
du tramadol, du losartan ou du tamoxifène ne les fait pas s'accumuler :
leur effet est supprimé.

Trois réponses possibles : interaction, **sans voie connue** (molécule
présente dans la table, métabolisée par aucune des enzymes suivies —
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
lettres : le méthotrexate est hebdomadaire.

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

**L'absence de donnée n'est pas une autorisation.** Un produit absent de
la table reçoit « à vérifier », jamais une ligne vide : une fiche qui ne
listerait que les interdictions se lirait « tout le reste est permis ».

Trois réponses pour les produits connus : oui, non, et « oui, gélule
ouverte » — les microgranules s'avalent sans être croqués. C'est le cas
le plus fréquent en gériatrie ; le classer en « non » ferait modifier
une ordonnance sans nécessité.

**La forme galénique décide, non la molécule.** Pour la morphine :
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

Neuf chapitres sur la même liste : les **interactions citées** par les
monographies, sans déduction ; les interactions sur les
**cytochromes** ; la **demi-vie plasmatique**, soit le délai de retour à
l'état antérieur après une modification d'exposition ; la **revue
d'ordonnance** — doublons, associations, cascades ; l'**adaptation
rénale** à la clairance saisie ; l'**adaptation hépatique** au stade
choisi ; le **sujet âgé** ; la **grossesse et l'allaitement** ; et
l'**écrasement des formes orales**.

**Le foie n'a pas de DFG.** La fonction rénale s'exprime par un chiffre
de laboratoire ; la fonction hépatique par un stade — Child-Pugh A, B ou
C — attribué par le clinicien sur cinq critères, dont deux cliniques.
Le panneau propose donc trois boutons et non un champ numérique.

Trois réponses, comme pour les cytochromes. « Non documenté » est en
gris ; « aucune adaptation » est écrit en toutes lettres. Exemple :
l'oxazépam ne demande aucune adaptation en insuffisance hépatique légère
à modérée, et c'est la benzodiazépine de choix chez le cirrhotique ; le
passer sous silence le confondrait avec un produit non documenté.

Une hépatopathie évolutive n'est pas un stade : les statines y sont
contre-indiquées quel que soit le score de Child-Pugh. Cette
contre-indication figure sur leur fiche, et non sous un palier.

La carte trace un arc par interaction, du produit en cause vers le
produit affecté. La couleur indique l'ordre de lecture, non une gravité
clinique : dose, durée et terrain ne sont pas connus du logiciel.

**La durée compte autant que le sens.** « Exposition augmentée » n'a pas
la même portée pour une demi-vie de deux heures que pour une demi-vie de
cinquante jours, et un effet peut survivre au produit : l'effet
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

Le zoom arbitre entre **vue d'ensemble** et **exploration**. Réduite, la
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
clavier, une boîte dans l'autre main.

**Pastilles à forme et couleur** : cercle barré pour une
contre-indication, triangle pour une précaution, coche pour une
utilisation autorisée par la table, points de suspension quand une
donnée manque. Elles se lisent sans le texte et sans la couleur.
L'onglet « Signaux » prend la couleur de l'alerte la plus grave : visible
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
système nerveux — le profil de l'amiodarone d'un coup d'œil. Degré et
source au survol. La rangée ne retient que la **toxicité** :
l'amiodarone traite le cœur et l'altère, et mêler indication et
toxicité afficherait deux fois « Cœur ». Un organe absent ne signifie
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
réponses, non des alertes, et les marquer marquerait toute l'ordonnance.
Cette page permet la revue d'ordonnance au comptoir sans ressaisir les
noms. Sans dossier ouvert, elle n'apparaît pas.

La barre **reprend les réponses des tables**, en pastilles, sans
conclure : chaque pastille cite le terme de sa table, et son infobulle
précise la portée du module avant la conduite. Dix lectures : ce sont
les interactions citées par les fiches du dossier, la revue d'ordonnance
avec ce produit ajouté, **la même molécule déjà présente au dossier sous
un autre nom**, l'interprétation de la biologie du dossier sous ce
traitement, la surveillance biologique demandée et la date du dernier
dosage, les cytochromes, l'écrasement, la grossesse et l'allaitement,
l'adaptation à la clairance du dossier, et le sujet âgé, à partir de la
date de naissance du dossier.

**Alertes en premier.** Les pastilles sont classées de la plus grave à
la moins grave ; à gravité égale, l'ordre des tables est fixe. Sur une
barre repliée après la troisième pastille, une alerte en quatrième
position serait manquée.

Une table sans réponse n'affiche pas de pastille : une rangée de « à
vérifier » ne signale rien. Quand aucune table ne répond, une ligne
l'indique — **l'absence de donnée n'est pas une autorisation**.

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
prochain rendez-vous déjà posé
s'inscrit dans son cadre. Titres, colonnes et lignes se réécrivent dans
« Textes imprimés », sous « Fiche d'entretien ».

Le bilan partagé de médication comporte une section consacrée au sujet
âgé : il concerne le patient polymédiqué, le plus souvent âgé, et c'est
le document transmis au prescripteur. Chaque ligne indique le risque et
l'alternative ; aucune ne prescrit un arrêt.

## Fiche de vaccination

Sur un acte Vaccination, « PDF » imprime la fiche de l'injection : les
questions à poser avant (allergie, fièvre, grossesse, immunodépression,
anticoagulant, malaise antérieur, vaccin récent), le vaccin tracé —
nom, dose, lot, péremption, voie, site, heure —, les suites
(surveillance de quinze minutes, inscription au carnet, consignes) et
ce que le calendrier vaccinal doit encore d'après le carnet du patient.
Une réponse « oui » est une question à instruire, pas une
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
l'application propose, le pharmacien décide.

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
s'ouvre sur la trame existante, pour correction comme pour ajout.

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

Une liste n'est pas un protocole : un protocole est un arbre de
décision ; une liste est une vérification d'exhaustivité.

**Aucune liste livrée** : une base neuve n'en comporte aucune. Chaque
officine rédige les siennes.

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

# Carnets de suivi du patient

Six feuilles d'automesure à domicile : pression artérielle, glycémie,
poids, débit expiratoire de pointe, INR, douleur.

**Chaque feuille porte le protocole de mesure.** Une pression prise
après un café, debout, ou une glycémie notée de mémoire n'ont pas de
valeur. Chaque feuille comporte donc quatre parties : technique de
mesure, objectif, signes justifiant un appel immédiat, et grille de
relevés.

**Aucun chiffre inventé.** Lorsque l'objectif est individuel —
glycémie, zone d'INR, meilleure valeur personnelle de DEP —, la feuille
l'indique et laisse la ligne à remplir. Seule l'automesure tensionnelle
porte une cible chiffrée, issue d'une recommandation publique.

**Aucune adaptation de dose.** Chaque feuille indique qui contacter et
quand.

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

# Pharmacocinétique et pharmacodynamie

La colonne technique d'une fiche porte un tableau : biodisponibilité,
pic plasmatique, liaison aux protéines, volume de distribution, fraction
éliminée inchangée, demi-vie, cible, délai et durée d'action, marge
thérapeutique étroite.

**Aucun chiffre inventé.** Une valeur provient de la fiche — extraite du
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

Une officine **crée** le réseau, puis **invite** les autres : elle
accepte une connexion le temps que l'autre saisisse son adresse, et les
deux se lisent par téléphone un code de cinq groupes. Un code identique
des deux côtés garantit l'absence d'intermédiaire.

La **synchronisation** se fait par un bouton, et à la fermeture si le
poste est configuré ainsi (Options › Base). Deux modes : connexion
directe aux officines qui acceptent les connexions, ou **dossier
d'échange** — partage réseau, dossier synchronisé — où chaque officine
dépose ses enregistrements et lit ceux des autres, sans ouverture de
port sur la box. Le dossier d'échange ne contient que des données
chiffrées.

Retirer une officine arrête ses envois ; ses envois antérieurs restent
au journal.

**Suivi de chaque officine.** Sous chaque officine appairée, la liste
indique le nom sous lequel elle signe, le nombre d'enregistrements reçus
d'elle et la date des dernières nouvelles, qu'elles soient arrivées par
une connexion directe ou par le dossier d'échange. Pour une officine à
adresse, la dernière connexion directe réussie, ou, en rouge, l'heure du
dernier échec et sa raison : pas de réponse à l'adresse (poste éteint,
port fermé ou adresse hors d'atteinte), officine qui n'est pas celle
appairée, ou clé de réseau différente. Le nom sous lequel elle signe sert
d'invite au champ du nom.

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

Un poste **fonde** le groupe : sa base est conservée, avec sa
numérotation des dossiers. Il **invite** ensuite les autres, comme pour
le réseau : connexion acceptée, adresse saisie, code de cinq groupes lu
des deux côtés. **Rejoindre remplace le contenu du poste** par les
données du groupe : faire une copie au préalable si nécessaire.

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

Les pièces scannées restent sur le poste qui les a numérisées.

# Connexions

L'icône à trois nœuds, en bas à droite de la barre d'état, indique le
nombre de postes du groupe joignables (« 2/3 poste(s) ») et
l'appartenance à un réseau d'officines ; un clic ouvre la vue
« Connexions ». Bleue quand tous les postes répondent, rouge après un
échec de synchronisation.

**Connexion au lancement** : les postes du groupe se découvrent et se
synchronisent sur le réseau local ; le réseau d'officines se synchronise
au lancement puis toutes les quinze minutes (Options › Base). Un poste
isolé écoute seulement : la vue liste les postes détectés, avec leur
adresse, et « Rejoindre… » la reprend. Rejoindre demande toujours le
code de cinq groupes, lu des deux côtés, qui garantit l'absence
d'intermédiaire.

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
adresse, **le bouton n'existe pas** et aucune connexion n'est ouverte.

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

Le plus utile : « écritures concurrentes signalées », soit le nombre de
rechargements provoqués par l'écriture préalable d'un autre poste. Il
révèle deux personnes travaillant sur la même donnée au même moment.

Aucun envoi : les compteurs restent dans la base chiffrée de l'officine
et se consultent dans bpm-audit.

Ils mesurent l'usage du logiciel, jamais celui d'une personne : aucun
opérateur, aucune initiale.

Actifs par défaut, désactivables dans Options › À propos, poste par
poste. La désactivation arrête le comptage sans rien effacer ;
l'effacement est un bouton distinct, avec double confirmation.

# Journal des accès

Qui a ouvert quel dossier, et quand — consultable dans bpm-audit.
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
fonction non coloré est inconnu du moteur : la faute de frappe se voit
avant l'exécution.

**Complétion.** Dès le début d'un mot, une liste s'ouvre sous le
curseur : flèches haut et bas pour parcourir, tabulation pour insérer,
Échap pour fermer. Rien ne s'ouvre sur un mot complet ni sur une ligne
vide.
