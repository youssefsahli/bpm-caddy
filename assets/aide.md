# BPM-Caddy

Le suivi des entretiens pharmaceutiques au comptoir : les dossiers, les
actes, l'agenda de l'équipe, le registre des stupéfiants, la caisse — et
ce qui s'imprime au nom de l'officine.

Tout est local. La base est chiffrée et ne sort pas du poste ; rien n'y
est envoyé nulle part. L'application ne fait que **deux** requêtes
réseau, et aucune ne part toute seule : la recherche d'une mise à jour,
et la mise à jour de l'annuaire des prescripteurs — cette dernière
seulement si l'officine a écrit une adresse, sans quoi le bouton
n'existe pas. Les deux vont chercher, aucune n'envoie.

# Le plan de travail

Trois volets autour d'un cahier d'onglets.

- À gauche, la navigation : le mois, les prochains rendez-vous, la liste
  qu'on cherche.
- Au centre, la vue ouverte. Les onglets du haut en gardent plusieurs
  ouvertes à la fois.
- À droite, ce volet-ci : les notes d'équipe, le carnet du jour, les
  notes personnelles, et cette aide.

Les deux volets latéraux se ferment et se redimensionnent ; leur largeur
et la vue ouverte sont retrouvées à la session suivante.

Le bouton « ? » de la barre du haut, ou `F12`, liste tous les raccourcis
clavier.
C'est la liste à jour : elle est tenue par l'application elle-même, et
non recopiée ici.

# Trouver quelque chose

« Aller à… » cherche partout d'un seul champ : les vues elles-mêmes, les
dossiers, les fiches, les tables de conversion, les préparations, les
protocoles, les dispositifs, le registre, les carnets de suivi, les
scripts, les outils de calcul et les textes imprimés. Tapez ce dont vous
vous souvenez — le nom, un bout du nom, ou les initiales.

Les outils se cherchent par **la question qu'ils répondent** et non par
le nom de l'écran qui les porte : « enfant » ou « mg/kg » trouve la dose
au poids, « DFG bas » ce que le rein change, « demi-vie » la
décroissance.

La recherche ignore la casse et les accents, et accepte les lettres dans
l'ordre sans qu'elles se suivent : « jndp » retrouve Jean Dupont.

Dans les monographies, « Dans le texte… » cherche les **mots** des
fiches et non leur titre : il rend chaque fiche qui dit le mot, avec la
phrase qui le porte.

**Une monographie se lit de trois façons**, au choix dans Options ›
Interface : « Feuille » la pose sur du papier au milieu du volet — ce
qu'on imprimerait ; « Dense » enlève la feuille et les marges, et on en
voit deux fois plus d'un coup d'œil ; « Lecture » resserre la colonne et
met de l'air autour, pour une fiche qu'on lit d'un bout à l'autre.
Aucune des trois ne change la taille des lettres : celle-là se règle une
fois, à « Taille du texte », pour toute l'application.

# Le dossier

Le bandeau porte l'identité et les traitements ; en dessous, les actes,
le journal, la biologie, les vaccins et les pièces scannées.

Les dates s'écrivent court : `230826` ou `2308` suffisent, et la case
affiche `23/08/2026`. Les heures aussi : `9`, `930`, `9h30`.

Ce qui est partagé entre les postes — l'identité, les états d'acte, les
rendez-vous — est écrit contre ce que l'écran affichait. Si un autre
poste a modifié la même ligne entre-temps, l'écriture est refusée et
l'écran se recharge plutôt que d'écraser le travail de quelqu'un.

## Lecture d'ordonnance

Sous la biologie, six lectures de la même ordonnance : l'interprétation
des résultats, ce qui n'a pas été demandé depuis trop longtemps, ce que
la clairance du jour change, ce que la grossesse et l'allaitement
changent, ce que l'âge du dossier change, et les croisements sur les
cytochromes.

Celle de l'âge est la seule dont le chiffre soit **déjà au dossier** :
la date de naissance y est depuis la création de la fiche, rien n'est à
taper, et c'est pour cela que personne ne la regarde. Le rein change la
dose, l'âge change le choix — les lignes viennent de la liste française
de Laroche, des critères STOPP/START et de ceux de Beers, et chacune
nomme deux choses : ce que l'âge fait courir, et ce qu'on met à la
place. Un « non » sans alternative laisse le problème entier. Rien ne
s'arrête d'un coup : arrêter brutalement un psychotrope chez un sujet
âgé expose davantage que de le poursuivre, et le remplacement se prépare
avec le prescripteur.

**Une forme locale n'est pas lue comme la voie générale.** Un collyre,
une pommade, un gel ou une pulvérisation nasale ne reçoivent ni palier
rénal, ni niveau de grossesse, ni demande d'examen : ces tables sont
rangées par molécule, et la même molécule ne fait pas la même chose
selon la voie. Un collyre à la ciclosporine ne croise rien, un gel au
lithium ne demande pas de lithiémie. Ce qui vaut pour la voie locale
reste écrit sur la fiche du produit, qui est l'endroit où le lire.

Le dernier répond à « ces deux lignes se rencontrent-elles sur une
enzyme ? ». Il **ne connaît que sept cytochromes** : ni la glycoprotéine
P, ni les transporteurs hépatiques, ni les additions d'effets — deux
sédatifs ne se rencontrent sur aucune enzyme et s'additionnent quand
même. Une ordonnance sans croisement n'est pas une ordonnance sans
interaction, et les lignes que la table ne connaît pas sont **nommées**
plutôt que passées sous silence.

Une prodrogue s'y lit à l'envers, et c'est écrit sur la ligne : freiner
l'enzyme qui fabrique le métabolite actif du clopidogrel, de la codéine,
du tramadol, du losartan ou du tamoxifène ne les fait pas s'accumuler,
cela supprime leur effet.

Trois réponses et non deux. Une ligne peut croiser, être **sans voie
connue** — la table la connaît et elle ne passe par aucune des enzymes
suivies, ce qui est la réponse qu'on cherche en se demandant par quoi
remplacer un traitement —, ou être **inconnue de la table**, ce qui
n'est pas la même chose et ne l'innocente pas.

## La fiche de traitement

« Fiche traitement… », en haut du dossier, imprime la feuille que le
patient emporte quand son ordonnance change. Elle répond dans l'ordre
aux quatre questions qu'on se pose sur le trottoir en sortant.

**Les prises de la journée.** Une grille de quatre moments — matin, midi,
soir, coucher — lue sur la posologie que le dossier retient pour **ce**
patient. Un chiffre quand l'ordonnance donne la quantité, une pastille
quand elle donne le moment sans le nombre : la feuille n'invente pas un
comprimé que personne n'a prescrit.

Une posologie que la grille ne sait pas lire garde ses quatre colonnes
en une seule case et dit sa phrase : quatre cases blanches se liraient
« rien à prendre ». Un « si besoin » sort de la grille avec sa
condition — un pilulier dit **quand** prendre, et y déposer un antalgique
à la demande le transforme en prise systématique. Une prise
hebdomadaire en sort aussi, en toutes lettres : une croix dans la
colonne « matin » d'une grille dont l'en-tête est une journée se lit
« tous les matins », et le méthotrexate est hebdomadaire.

**Le renouvellement.** La délivrance en cours sur le total, la période
que couvre la boîte du jour, le jour où l'ordonnance est épuisée, et la
ligne qui compte : le jour avant lequel il faut avoir **vu** le
prescripteur — quelques jours plus tôt, parce qu'un rendez-vous ne se
prend pas le matin pour le soir. Le délai et la façon de dessiner
l'avancement — pastilles, jauge, dates, ou la phrase seule — se règlent
dans Options › Règles.

Le rang de la délivrance se **compte**, il ne se déduit pas du
calendrier : un patient qui revient trois semaines en retard en est à sa
deuxième délivrance, tard, et non à sa quatrième. Il se note dans le
dossier, sur la puce du traitement, avec le jour de l'ordonnance, la
durée qu'une délivrance couvre et le nombre de renouvellements. Rien
n'est obligatoire : une ligne dont rien n'est noté imprime qu'elle ne
l'est pas, ce qui est une information.

**Le détail par médicament.** Indication, posologie, conseils, conduite
en cas d'oubli, et dans un cadre à part les signes d'alerte. Tout vient
des fiches du référentiel, que l'équipe corrige là où elles sont : il
n'y a pas de seconde table de conseils à tenir à jour.

## Peut-on écraser ?

Le bouton « Écraser ? » du dossier imprime une feuille pour l'EHPAD ou
l'infirmière : toute l'ordonnance, ligne par ligne, la conduite pour
chaque forme et l'alternative quand il y en a une.

**Le silence n'est pas une permission.** Un produit que la table ne
connaît pas reçoit « à vérifier », et jamais une ligne absente : une
feuille qui ne montrerait que les interdits se lirait « tout le reste,
oui ».

Trois réponses et non deux, pour ce que la table connaît : oui, non, et
« oui en ouvrant la gélule » — les microgranules s'avalent et ne se
croquent pas, c'est le cas le plus fréquent en gériatrie, et le réduire
à « non » ferait changer une ordonnance qui n'en avait pas besoin.

**C'est la forme qui décide, pas la molécule.** La morphine s'écrase ou
ne s'écrase pas selon la boîte : Moscontin jamais, Skenan en ouvrant la
gélule. Une forme qui ne passe pas par la bouche — injectable,
implantable — reçoit sa ligne elle aussi, pour dire qu'il n'y a rien à
écraser.

Chaque refus dit par quoi remplacer, ou dit qu'il n'y a rien et qu'il
faut appeler le prescripteur. Certains refus protègent celui qui écrase
plutôt que le patient : un cytotoxique, un tératogène — c'est la
poussière qui est le danger, et la feuille le dit.

## La conciliation de sortie

L'onglet « Conciliation » compare l'ordonnance du dossier à celle qu'un
patient rapporte de l'hôpital. On colle la liste de sortie ; chaque
ligne est rapprochée d'une fiche, et l'écran dit ce qui a été arrêté,
changé, ajouté ou remplacé.

Six lectures, et la première passe avant les autres : la ligne que
personne n'a pu rapprocher, parce que personne ne l'a vérifiée. Puis le
remplacement dans la même classe — la divergence dont le patient repart
avec les deux boîtes —, l'arrêt, le changement de posologie, l'ajout, et
ce qui est reconduit sans changement.

La feuille s'imprime à l'attention du prescripteur, avec un cadre laissé
pour sa réponse. Elle ne vaut pas avis médical, et elle l'écrit.

# Croiser une liste

L'écran « Croisement » pose les mêmes questions à une liste qu'on
compose soi-même, sans dossier : depuis le dossier ouvert, ou en tapant
des noms.

Neuf chapitres sur la même liste : les **interactions citées** par les
monographies les unes sur les autres, sans rien de déduit ; les
croisements sur les **cytochromes** ; la **demi-vie plasmatique**, soit
le délai de retour d'une exposition déplacée ; la **revue
d'ordonnance** — doublons, associations, cascades ; l'**adaptation
rénale** à la clairance saisie ; l'**adaptation hépatique** au stade
désigné ; le **retentissement de l'âge** ; la **grossesse et
l'allaitement** ; et « peut-on écraser ? ».

**Le foie n'a pas de DFG.** Le rein donne un chiffre qui se lit sur un
compte rendu ; le foie donne un stade — Child-Pugh A, B ou C — qu'un
clinicien attribue à partir de cinq éléments dont deux ne sont pas des
valeurs de laboratoire. Le panneau offre donc trois boutons et non un
champ : un champ inviterait à écrire un chiffre, et il n'y en a pas.

Trois réponses et non deux, comme pour les cytochromes. « On ne sait
pas » est en gris ; « on sait, et il n'y a rien à changer » est écrit en
toutes lettres. L'oxazépam est le cas qui le justifie : sa fiche dit
qu'aucune adaptation n'est nécessaire en insuffisance légère à modérée,
et c'est précisément la benzodiazépine qu'on cherche chez un
cirrhotique. Une liste qui la tairait la rendrait aussi muette qu'un
produit dont personne n'a rien écrit.

Une hépatopathie évolutive n'est pas un stade : les statines y sont
contre-indiquées quel que soit le Child-Pugh, et les ranger sous un
palier dirait la chose à un stade en la taisant aux autres. Leur fiche
le dit là où c'est vrai.

La carte dessine une corde par croisement, de la ligne qui agit vers
celle qui bouge. Sa couleur est l'ordre de lecture, et non une gravité
clinique : le logiciel ne sait ni la dose, ni la durée, ni le terrain.

**Le temps compte autant que le sens.** « Exposition augmentée » ne dit
pas la même chose d'un produit dont la demi-vie est de deux heures et
d'un autre dont elle est de cinquante jours, et un effet peut durer bien
après le produit — l'effet antiplaquettaire du clopidogrel tient sept à
dix jours quand sa demi-vie est de six heures.

# Les calculs du comptoir

Ils se trouvent en tapant ce qu'on cherche dans « Aller à… » — « enfant »,
« mg/kg », « clairance », « DFG bas », « demi-vie » —, depuis « Calculs »
au-dessus des tables de conversion, et depuis le bouton « Calculs… »
d'une fiche ouverte, qui les ouvre **avec cette fiche en main**.

Une fiche en main change tout : sans elle les outils restent généraux et
il faut déjà connaître ses chiffres, c'est-à-dire ne pas avoir besoin
d'eux. Avec elle, deux d'entre eux répondent pour ce médicament-là.

**La clairance de la créatinine**, par Cockcroft et Gault, avec le stade
qui lui correspond. C'est un estimateur sur le poids réel : chez
l'obèse, l'œdémateux ou le dénutri il s'écarte, et le laboratoire donne
le DFG estimé sur la formule en vigueur.

**La dose par kilo.** Un poids, des milligrammes par kilo, un nombre de
prises. Et, lorsqu'une fiche est en main, **ce qu'elle écrit au
poids** : ses lignes par indication d'abord, chacune sous son titre,
puis ce que sa posologie ajoute — chaque dose avec son rythme et avec la
phrase d'où elle vient. Un plafond (« sans dépasser… ») s'écrit comme
tel, une association nomme la molécule de chaque chiffre, et une dose
cumulée sur toute une cure n'est pas lue. Le rythme n'est pas un détail, c'est la moitié de la
dose — « 15 mg/kg par prise toutes les 6 heures » et « 60 mg/kg par
24 heures » sont le même traitement, et quatre fois l'un de l'autre. La
phrase non plus : une même fiche écrit souvent cinquante par jour pour
l'angine et quatre-vingts pour l'otite, et rien dans les nombres ne dit
laquelle. Une fiche qui n'écrit pas de dose au poids ne s'en voit pas
proposer : l'écran le dit, et c'est le signe qu'il y a une posologie à
compléter.

**Ce que le rein change**, pour la fiche en main et un DFG qu'on saisit —
celui qu'on vient de calculer, d'un bouton. C'est la même table que le
panneau « Rein » du dossier, lue sur une fiche plutôt que sur une
ordonnance : la question « la metformine, à trente, on fait quoi » se
pose souvent pour quelqu'un dont le dossier n'est pas ouvert. Le palier
qui parle est le plus bas de ceux qui sont franchis, les seuils viennent
des RCP, et la décision reste celle du prescripteur.

**La décroissance et l'accumulation** : une demi-vie, un intervalle
entre deux prises, et le temps qu'il faut pour que le produit ait quitté
l'organisme. La demi-vie se reprend d'une fiche de la base.

# La carte du voisinage

Depuis une fiche, « Voisinage… » ouvre la base **en image** : la fiche au
milieu, ce qu'il y a autour, et un clic pour déplacer le milieu. La liste
et la recherche répondent à « où est telle fiche » ; celle-ci répond à
« qu'est-ce qu'il y a autour », qui est la question d'une rupture de
stock, d'une contre-indication trouvée au comptoir, et de qui apprend une
classe.

Trois anneaux, du plus proche au plus lointain, et ce ne sont pas trois
fois la même chose. **La molécule** : une autre spécialité de la même
DCI — la question de la substitution, et le seul lien où les deux boîtes
contiennent le même médicament. **La classe** : une autre molécule du
même groupe, ce que demandent une rupture ou une intolérance ; le groupe
est celui du référentiel et non le libellé écrit sur la fiche, si bien
que « bisphosphonate » et « biphosphonate » sont bien le même anneau.
**L'interaction** : une fiche que la monographie du centre nomme
elle-même — le lien qui ne découle pas du classement, et le seul qui
traverse toute la base.

Un cerclage rouge dit que la fiche documente une toxicité ou une marge
thérapeutique ; c'est ce qui fait rouvrir la fiche d'un voisin qu'on
allait proposer. Une coche dit que le dossier ouvert **prend déjà** ce
médicament — la réponse à la question qu'on se pose en cherchant une
substitution. Sous la figure, la légende nomme chaque couleur, et une
phrase dit ce que les anneaux n'ont pas pu prendre : un anneau coupé se
dit, il ne se devine pas.

**Se déplacer et grossir.** La figure se glisse à la souris et se
grossit à la molette — autour du pointeur, si bien que ce qu'on regarde
reste où on le regarde. Au clavier, `+` et `−` font la même chose et `0`
remet la carte à sa taille et à sa place ; deux boutons dans un coin de
la figure les doublent, un double-clic dans le vide la remet à plat, et
le titre du cadre porte le grossissement dès qu'il n'est plus de cent
pour cent.

Le grossissement est le réglage entre **tout voir** et **tout lire**.
Réduite, la carte prend davantage de voisins — jusqu'au plafond de
lecture, qui ne bouge pas — au prix des noms, que la place ne peut plus
tous écrire ; grossie, elle garde les siens et leur donne enfin de quoi
s'écrire. Rien ne bouge en chemin : la place d'un voisin sur son anneau
se compte sur tous ses candidats et non sur ceux qu'on en dessine, si
bien qu'en ouvrir un de plus le fait venir **se poser dans un trou**
sans déplacer les autres.

**Passer sur un nœud** ouvre cette petite fiche : le nom, la DCI, la
classe, à quoi le médicament sert, ce que sa fiche écrit d'une toxicité,
et son statut lorsqu'il dit autre chose que « commercialisé » — une
rupture arrête une substitution avant tout le reste. Le rayon du nœud
qu'on désigne se lit seul, les autres s'estompent sans disparaître.

Elle dit aussi **ce que le trait veut dire**. La couleur donne la nature
du lien ; ce qu'il implique se lit dessous, en pastilles : ce sont les
mêmes que celles de la barre du comptoir, les mêmes tables et les mêmes
réserves au survol. Un voisin se lit **contre le centre** — deux AINS
font un doublon, deux molécules se croisent sur un cytochrome, et celui
qu'on allait proposer demande peut-être une adaptation au rein que
l'autre ne demandait pas. Le moyeu, lui, n'est relié à rien sur la
figure : il se lit contre l'ordonnance du dossier ouvert.

# La barre au-dessus des autres fenêtres

`F9` réduit la fenêtre à une barre de quelques centaines de pixels,
posée au-dessus des autres applications — c'est la forme qu'on garde
dans un coin d'écran pendant qu'on travaille ailleurs. Elle n'a **pas
de bordure** : on la déplace en la prenant par sa tête, et le menu à
côté d'« Agrandir » la pose d'un coup — dans un coin, en bandeau sur
toute la largeur en bas de l'écran, en colonne à droite, ou libre. On tape un nom
ou une molécule ; les flèches parcourent les fiches qui répondent,
Entrée ouvre celle qu'on lit, Échap efface la question puis, sur une
question déjà vide, rend la fenêtre. Le champ garde le foyer, si bien
qu'une douchette — qui est un clavier — tape dedans sans rien installer.
**Alt et 1 à 5** appellent les cinq gestes du bas, dans leur ordre : la
barre se conduit alors entièrement au clavier, ce qui est la façon dont
on s'en sert pendant qu'on tient une boîte de l'autre main.

**Les puces portent une forme autant qu'une couleur** : un cercle barré
pour ce qu'on ne fait pas, un triangle pour ce sur quoi on s'arrête, une
coche pour ce que la table autorise, trois points quand il manque un
chiffre pour conclure. On les reconnaît sans les lire, et sans voir la
couleur. L'onglet « Signaux » porte, lui, la couleur de ce qui presse le
plus : depuis « Conseils » ou « Posologie », on voit qu'il y a quelque
chose à lire une page plus loin.

**Cinq pages, et les flèches gauche et droite pour tourner.** Haut et
bas parcourent les fiches qui répondent, gauche et droite ce que la
fiche lue dit : deux questions perpendiculaires, deux paires de flèches.
« Signaux » porte les lectures des tables ; « Posologie » d'abord **la
posologie du dossier, pour cette personne-là** — la fiche dit la
référence, le dossier dit le sien —, puis la prose de la fiche, **ses
lignes indication par indication** — parce que « combien » n'a pas de
réponse sans « pour quoi » — et les formes et dosages disponibles, qu'on
demande au comptoir aussi souvent que la dose — et **une courbe** : le profil
d'action pour une insuline — « début 15 min, pic 1 à 3 h, durée 5 h »
demande de se représenter une forme, et la forme se voit, le pic de
l'après-midi d'une NPH se lisant d'un coup d'œil à côté de la ligne
plate d'une glargine — et pour tout le reste la décroissance
plasmatique, avec ce qu'il en reste à vingt-quatre heures. « Combien de
temps ça reste » est une question de comptoir, et « demi-vie ≈ 12
heures » demande une arithmétique dont la page dispense ; « Conseils »
les explications à donner à la personne, la conduite en cas de prise
oubliée, **les signes d'alerte en entier** — la page des signaux n'en montre que la première
phrase, et « ce qui doit vous faire consulter » se dit en entier — et ce
que l'équipe a écrit elle-même sur la fiche. Une page qui n'a rien à
dire n'est pas offerte, et quand une seule parle il n'y a pas de bande
d'onglets.

**« Précautions » s'ouvre sur une rangée d'organes** : ce que ce produit
peut abîmer, du plus grave au moins, en couleur. Sur la Cordarone,
thyroïde et poumon en rouge, puis cœur, œil, foie, peau, puis le
neurologique — le profil de l'amiodarone en un coup d'œil, là où trois
paragraphes le disent en désordre. Le degré et la clause sont au survol.
La rangée ne nomme que ce que le produit **abîme** : l'amiodarone traite
le cœur et l'altère, et mêler les deux sens ferait paraître « Cœur »
deux fois en sens contraires — le « traite » passerait même devant les
deux atteintes qui font arrêter ce traitement-là. Un organe absent n'est
pas une innocuité : la prose entière est juste dessous.

Vient ensuite cette prose — ce qui contre-indique, ce qui arrive, ce
qu'on surveille —, puis **la marge thérapeutique et l'antidote**, les
deux choses qu'on cherche quand la question cesse d'être « est-ce que je
délivre » pour devenir « qu'est-ce qui se passe si ». Et pour finir
**les sources de la fiche** : cette page porte ses affirmations les plus
fortes, et on les répète au comptoir.

La cinquième, « Dossier », est la seule qui ne parle pas de la fiche
cherchée : c'est l'ordonnance ouverte, ligne par ligne **avec ce que le
dossier retient de chacune** — « Zeclar · 500 mg matin et soir, 7
jours », et non « Zeclar » —, et **une ligne cliquée devient la
question**. Un liseré signale les lignes
qu'une table demande de vérifier : le tri du comptoir, avant de les
ouvrir une par une. Il ne marque que cela : « à vérifier » et
« sans donnée » sont des réponses, pas des arrêts, et les marquer
marquerait toute l'ordonnance. C'est le geste de la révision au
comptoir — descendre une ordonnance en posant la même question à chaque
ligne —, qui demandait jusqu'ici de retaper huit noms dont on ne se
rappelle ni l'orthographe ni le dosage. Sans dossier ouvert, la page
n'est pas là.

Elle **rapporte ce que les tables disent**, en pastilles, et ne conclut
rien à leur place : chaque pastille cite le mot de sa table, et son
survol donne la portée du module avant la conduite. Dix lectures : ce
que les fiches du dossier disent de celle-ci, ce que la revue
d'ordonnance en dit une fois qu'on l'ajoute aux autres, **si le dossier
porte déjà cette molécule sous un autre nom**, ce que les valeurs de
biologie du dossier veulent dire sous ce traitement-là, ce que ce
traitement demande qu'on mesure et depuis combien de temps personne ne
l'a demandé, les cytochromes, « peut-on écraser ? », la grossesse et l'allaitement,
ce que la clairance du dossier change, et ce que l'âge change — l'âge est au dossier depuis le jour où la fiche a été créée, et
c'est bien pour cela que personne ne le regarde.

**Ce qui arrête passe devant.** Les pastilles descendent du plus
pressant au plus calme, et à ton égal l'ordre des tables ne bouge pas :
sur une barre où le pli tombe après la troisième, une alerte rangée
quatrième est une alerte manquée.

Une table qui n'a rien à dire ne dessine pas de pastille : une rangée de
« à vérifier » ne signale rien et apprend à ne plus regarder la bande.
Quand elles se taisent toutes, une phrase le dit — **le silence n'est
pas une autorisation**.

Le foie n'y est pas, et c'est voulu. Il demande un stade de Child-Pugh,
qu'aucun dossier ne porte et ne portera : la pastille dirait « dépend du
stade » sur une carte de deux, pour toujours. Le stade se désigne au
croisement, où trois boutons l'attendent.

Cliquer une pastille ouvre le croisement chargé de l'ordonnance du
dossier **et** de la fiche cherchée, **sur le chapitre que la pastille
nomme** : c'est la question du téléphone passée entière, et non un écran
vide à recomposer ni un écran de neuf chapitres ouvert au mauvais. La
surveillance fait exception et mène à l'onglet « À surveiller » du
dossier : elle lit les **dates** de celui-ci, et « depuis combien de
temps personne n'a demandé cet examen » n'a pas de sens sur une liste
composée à la main.

« Copier » met la page lue dans le presse-papier, pour la coller dans le
logiciel de comptoir : une posologie ou une conduite en cas d'oubli
finit souvent dans le commentaire d'une ligne, et la retaper depuis
l'écran d'à côté est l'occasion de se tromper sur le chiffre qu'on vient
de vérifier. Ce qui est copié est ce qui est lu, page comprise, le nom du
produit en tête. La page « Dossier » fait exception et copie la fiche :
une liste de traitements nominative sortie d'ici finirait collée dans un
champ dont personne ne sait où il va.

La tête porte le nom du dossier ouvert **et les deux chiffres que les
pastilles lisent** : l'âge et la clairance. Sans eux on lisait « Rein ·
dépend du DFG » sans savoir si le dossier en portait un ; le chiffre
absent se voit maintenant à sa place vide, ce qui est la réponse.

Et sur une question vide, la barre montre **les dernières fiches lues**.
Au comptoir on compare deux produits — celui de l'ordonnance et celui
que le médecin propose — et taper le second effaçait le premier : Échap
efface la question, donc il ramène à cette rangée, et il devient un
retour en arrière.

**Et quand aucun nom ne répond, la prose répond.** Le champ cherche un
nom et une molécule ; tapez « pamplemousse » et il n'en trouve aucun,
alors que cent seize passages le nomment. La barre ouvre alors le texte
des fiches — les treize champs **et les lignes de posologie**, où
s'écrivent « à jeun », « à distance du fer » et le pamplemousse
justement —, et chaque fiche arrive avec la phrase qui l'a fait
répondre : sans elle on ne saurait ni laquelle porte le mot, ni ce
qu'elle en dit. Tant qu'un nom répond, c'est le nom qui répond.

Un code-barres n'est pas un nom qu'on n'aurait pas trouvé, et la barre
le dit plutôt que de répondre « aucun résultat ». Aucune fiche ne porte
de code : le seul lien entre un code et une boîte est celui qu'un humain
a posé au registre des stupéfiants, en présentant la boîte.

**Mais la boîte, elle, parle.** Son DataMatrix porte son code, son lot
et sa péremption, et ceux-là ne demandent la permission d'aucune table :
la barre les écrit sous le code lu. Une péremption passée est dite en
rouge — le jour est calculé, et une boîte marquée 09/2026 est bonne
jusqu'au trente inclus, parce qu'une péremption est un mois. Aucun seuil
ne décide au-delà : « il reste onze jours » est un fait, « c'est trop
peu » dépend de la durée du traitement, et c'est le comptoir qui
tranche. Un lot que rien n'a fermé, ou une lecture arrêtée sur un champ
inconnu, sont signalés comme tels : un lot faux affiché comme sûr est
pire que pas de lot du tout, car c'est celui-là qu'on recopie sur un
rappel de lot.

Et si le registre a appris ce code — en présentant la boîte, ce qui est
le seul lien code-produit de ce logiciel —, la barre nomme le produit
suivi et « Délivrance » ouvre le registre **sur lui**. Sinon elle ouvre
le registre où il s'ouvre : c'est là qu'on lui apprend le code, et ce
n'est pas un geste qu'un logiciel fait à votre place.

# Les entretiens

Un acte porte sa thématique, son état, sa date, sa durée et les
initiales de qui l'a fait. « Tout imprimer » rend la fiche d'entretien,
le bilan et le plan de prise en un seul document.

Le bilan porte une section **« Ce que l'âge change »** : le bilan
partagé de médication est fait pour le patient polymédiqué,
c'est-à-dire presque toujours pour un sujet âgé, et c'est la seule
feuille de cette lecture qui parte avec lui chez le prescripteur.
Chaque ligne y nomme le risque et ce qu'on met à la place ; aucune ne
dit d'arrêter.

## L'ordonnance sous protocole

Après un test rapide positif — angine à streptocoque, cystite simple —,
l'écran compose l'ordonnance que le protocole autorise : l'antibiotique,
sa posologie, l'adjuvant et les conseils.

Les lignes proposées sont **celles de l'officine** : livrées avec les
protocoles en vigueur, elles se réécrivent avec « Modifier les
lignes… » — une molécule ajoutée, une posologie changée, une borne
d'âge déplacée le jour où le protocole change. Ce qui est réécrit vaut
pour tous les postes.

**L'âge, le sexe et la grossesse sont facultatifs.** Le dossier les
donne quand il les connaît — la date de naissance, le sexe ou à défaut
le NIR —, et la fenêtre les laisse corriger pour cette ordonnance. Sus,
ils grisent les lignes que le protocole n'ouvre pas à cette personne,
avec la raison ; inconnus, ils ne bloquent rien. Quand aucune ligne ne
s'applique, l'écran le dit : c'est une orientation vers un médecin.

Rien n'est jamais sélectionné d'office et toute posologie proposée est
modifiable : l'application propose, le pharmacien décide.

Les adjuvants ne sont pas une liste du programme. Ce sont les fiches
portant l'étiquette voulue, avec leurs propres lignes de posologie :
ajouter un produit, c'est ajouter une fiche.

Aucune mention n'est imprimée d'office ; celles que l'officine veut voir
s'écrivent dans Options › Mentions.

# L'agenda et le planning

L'agenda se lit par jour, par semaine ou par mois. Le filtre **n'efface
pas** : ce qu'il écarte reste dessiné en trait contre la gouttière, et
ce qui chevauche une entrée retenue reste dessiné en entier — sans quoi
le filtre fabriquerait le conflit qu'il devait montrer.

Les quatre lectures parlent **du même moment** : changer de lecture
recadre les autres sur le jour qu'on regardait. Le jour détaillé porte
son liseré dans la grille de la semaine comme dans celle du mois, et
« Imprimer la semaine » sort la semaine affichée.

Le plan de journée porte **un pointillé à l'heure qu'il est**, avec un
point rouge dans la marge. Il ne se dessine que sur la journée
d'aujourd'hui : posé sur un autre jour, il dirait l'heure d'un jour
qu'on ne regarde pas.

« Planning » est la quatrième lecture : l'équipe, une ligne par
personne, sept colonnes de jours. Le total d'un jour passe au rouge dès
qu'un **creux reste pendant l'ouverture** — pas seulement quand
personne n'est inscrit : une journée tenue le matin et vide l'après-midi
est un creux, et c'est celui-là qu'on ne voit pas en lisant la grille.

## Les rythmes

Un poste revient selon un rythme : ce jour-là, tous les jours jusqu'à
une date, chaque semaine, les semaines paires, les semaines impaires,
une semaine sur deux, sur trois, sur quatre.

**« Les semaines paires » n'est pas « une semaine sur deux ».** L'une se
lit sur le calendrier, l'autre se compte depuis le jour posé. Elles
tombent ensemble pendant des années et se séparent pour toujours au
premier passage d'une année de 53 semaines.

Un congé se pose comme une plage : « tous les jours », du 12 au 26. Une
date de fin est alors obligatoire.

## La trame de la semaine

« Trame… » écrit la semaine entière d'une personne en une fois. Elle
s'ouvre sur la trame déjà posée : c'est un écran pour corriger, pas
seulement pour ajouter.

Une journée coupée s'y écrit en **deux postes** — 9 h – 12 h 30 puis
14 h – 19 h 30 — et non en un poste à longue pause : une pause n'a pas
d'heure, si bien que la bande de couverture compterait la personne au
comptoir pendant sa coupure.

Sur une alternance, deux onglets : la semaine paire et l'impaire. Une
journée identique sur les deux est écrite une fois, hebdomadaire.

Ce que deux semaines de sept jours ne portent pas n'est pas approximé :
l'écran le dit et ne propose pas de remplacer ce qu'il ne montre pas.

## Corriger un jour sans effacer la série

« Ce jour seulement » fait porter « Modifier » et « Supprimer » sur une
seule occurrence. La trame reste écrite, une ligne la contredit ce
jour-là, et la semaine dit la vérité.

# Les listes de contrôle

« Listes » tient les suites de choses à cocher : l'ouverture, la
fermeture, le retour de vacances, ce qu'on vérifie avant de délivrer.
Elles s'impriment en A4, une case par ligne, la date et la personne
laissées à remplir.

Ce n'est pas un protocole. Un protocole répond à « que fait-on dans ce
cas-là » et se lit en descendant un arbre ; une liste répond à
« qu'est-ce qu'on n'a pas oublié » et se lit en cochant.

**Rien n'est livré** : une base neuve n'a aucune liste. Une liste
d'ouverture écrite ailleurs qu'à l'officine est une liste que personne
ne coche.

# La carte vaccinale

Deux tables, toutes deux **indicatives**, et chacune nomme sa source à
l'écran : le calendrier vaccinal, qui dit ce qu'un adulte doit
aujourd'hui, et la table du voyageur, qui porte les recommandations du
BEH par pays.

Ni l'une ni l'autre ne remplace le texte dont elle vient. Elles rendent
d'un coup d'œil une question que le comptoir pose vingt fois par jour,
et c'est tout ce qu'elles prétendent.

La carte est un **cartogramme et non une projection** : chaque pays
reçoit le même carré, rangé dans le bloc de sa région. C'est ce qu'on
demande à une table de référence — trouver un pays, pas mesurer une
distance.

Le carnet de vaccination d'un dossier s'imprime avec ses doses, leurs
dates, le lot et le site d'injection, et la date du prochain rappel
quand elle est connue. Ce qui **manque**, c'est l'écran du dossier qui
le dit, en lisant le calendrier contre les doses déjà portées.

# Les carnets que le patient emporte

Six feuilles à remplir chez soi : automesure tensionnelle, glycémie,
poids, débit de pointe, INR, douleur.

**Ce qui manque à une grille photocopiée n'est pas la grille, c'est le
protocole.** Une tension prise après le café, debout, sur le bras qui
traîne ne veut rien dire ; une glycémie notée le soir de mémoire non
plus. Chaque feuille porte donc quatre choses, et la grille n'est que la
quatrième : comment mesurer, ce qu'on vise, ce qui s'appelle sans
attendre, et où écrire.

**Aucun chiffre inventé.** Là où l'objectif est individuel — la
glycémie, la zone d'INR, la meilleure valeur personnelle de souffle —,
la feuille dit qu'il est individuel et laisse la ligne à remplir,
plutôt que d'imprimer une valeur que le patient prendrait pour la
sienne. La seule cible chiffrée est celle de l'automesure tensionnelle,
qui est une recommandation publique et non la décision d'un médecin.

**Rien qui remplace le prescripteur.** Aucune feuille n'adapte une
dose ; toutes disent à qui téléphoner et quand.

Le texte de chaque feuille se réécrit, comme tout ce qui part sur du
papier au nom de l'officine.

# Le registre des stupéfiants

Il vit dans son propre fichier, chiffré comme la base, et **il ne
s'efface pas**. Une ligne fautive reste écrite ; une seconde ligne la
nomme et défait exactement ce qu'elle avait fait.

Le solde est deux nombres et non un : ce qui est délivrable, et ce qu'un
patient a rapporté et qui attend sa destruction.

Une case vide n'est pas un zéro. Sur une feuille de comptage, seuls les
produits qu'on a réellement comptés sont inscrits.

Une ligne porte le **numéro de dossier** et jamais le nom : un registre
s'imprime et se laisse sur un comptoir.

Le numéro d'ordonnancier est attribué au moment de l'écriture et n'est
jamais réattribué : une ligne annulée garde le sien, et la suite
continue après lui.

**Une seule suite, continue.** Elle ne repart pas au premier janvier :
« le 18950 » désigne une délivrance et une seule, ce qui est la
condition pour l'écrire sur une ordonnance et la retrouver.

Une officine qui s'installe continue son registre de papier : Options ›
Base, « Premier numéro d'ordonnancier », une fois. Déclaré trop bas
après coup, il ne fait rien — un numéro posé est posé.

## Les questions que le registre pose

L'onglet « Vigilance » relit le registre et pose trois questions :
rapprochement des délivrances, pluralité de prescripteurs, escalade des
quantités. Ce sont **des questions et non des verdicts** : chacune cite
les lignes qui la motivent, et se vérifie en les relisant.

Ce que le registre ne sait pas, il ne l'invente pas. Une ligne porte un
jour, une quantité, un dossier et un prescripteur — ni la dose
quotidienne, ni la durée prescrite. « Ce traitement aurait dû durer
jusqu'au » ne se calcule donc pas.

La durée maximale de la famille ne sert qu'à **se taire** : au-delà,
deux ordonnances ne peuvent pas se chevaucher, et la question ne se pose
pas. Elle ne sert jamais à déduire un rythme — une ordonnance de sept
jours sous un plafond de vingt-huit passerait pour quatre fois trop
lente, et chaque délivrance légitime deviendrait un signalement. Une
règle qui crie au loup est une règle qu'on éteint.

Sous trois délivrances précédentes, rien n'est dit : il n'y a pas encore
de cadence à laquelle comparer.

# Pharmacocinétique et pharmacodynamie

La colonne technique d'une fiche porte un tableau : biodisponibilité,
pic plasmatique, liaison aux protéines, volume de distribution, part
éliminée inchangée, demi-vie, cible, délai et durée d'action, marge
thérapeutique étroite.

**Aucun chiffre n'y est inventé.** Une valeur vient de la fiche — lue
dans sa prose, et la phrase se lit au survol — ou de l'officine, qui la
saisit avec « Compléter… » et **sa source** (le RCP, section 5.1 ou
5.2) : une valeur sans source est refusée. Ce que personne n'a chiffré
s'écrit « non chiffré ». Une valeur sourcée l'emporte sur la lecture de
la prose, s'affiche sur la monographie et s'imprime avec elle.

Quand l'officine fait partie d'un réseau, les valeurs qu'elle a sourcées
y voyagent aussi, et celles des autres officines s'affichent avec leur
source et leur nom — après les siennes, avant la prose.

# Les versions d'une fiche

Chaque modification d'un champ d'une fiche est une **version** : qui,
quand, quelle valeur, et ce qu'elle remplaçait. « Historique… », dans la
colonne technique, les montre champ par champ, la plus récente en haut ;
« Revenir à cette version » en écrit une de plus avec l'ancienne valeur.
Rien ne s'efface.

Quand l'officine fait partie d'un réseau, les versions voyagent : une
fiche corrigée dans une officine se corrige chez les autres — **si leur
fiche dit encore ce que la version remplaçait**. Sinon la version attend,
« à arbitrer » : « Adopter » la prend, « Garder la mienne » la laisse,
et rien n'est écrasé en silence. Chaque officine peut revenir à
n'importe quelle version de n'importe quel champ. Les notes de l'équipe
ne voyagent pas, et aucune fiche ne porte de patient.

# Les ruptures

Quand un produit manque, sa fiche le signale : « Signaler une rupture »,
dans la colonne technique, et « Rupture levée » quand il revient. Une
rupture signalée s'affiche en tête de la fiche, avec ce que les
collègues ont donné à la place.

« Noter une substitution… » garde ce qui a été délivré à la place, et
comment ça s'est passé : a tenu, refusé par le patient, refusé par le
prescripteur, revenu. Les propositions viennent d'abord de la même
molécule, puis de la même classe. **Aucun patient n'est noté** — un
produit, un autre, une date et des initiales.

La vue **Ruptures** (« Aller à… ») rassemble ce qui manque en ce moment
et tout le journal. Un nouveau pharmacien y lit ce que l'équipe sait
depuis des mois ; quand l'officine est reliée à d'autres, ce qu'elles
ont noté s'y lit aussi, avec leur nom.

**Ce qui a été tenté n'est pas une équivalence.** Quand le substitut
n'est pas de la même classe — un dermocorticoïde modéré pour un fort —,
la ligne le dit ; le dosage, la forme et le patient se vérifient devant
l'ordonnance. Une rupture signalée depuis plus de quatre-vingt-dix jours
sans nouvelle n'est plus « en cours » : quelqu'un a oublié de la lever.

Le journal ne se réécrit pas ; une erreur se retire, et le retrait reste
écrit.

# Le réseau d'officines

Les officines d'un groupement peuvent partager ce journal : « Réseau
d'officines… », dans la vue Ruptures. **Seuls les ruptures et les
substitutions voyagent** — jamais un patient, un dossier, le registre ou
la caisse —, chiffrés et signés, sous une clé propre au réseau.

Une officine **crée** le réseau, puis **invite** les autres : elle ouvre
une porte le temps que l'autre compose son adresse, et les deux se
lisent au téléphone un code de cinq groupes. S'il est le même des deux
côtés, personne n'est entre elles.

Ensuite on **synchronise** sur un bouton, et à la fermeture si le poste
le veut (Options › Base). Deux chemins : composer l'adresse des
officines qui ont une porte ouverte, ou un **dossier d'échange** — un
partage réseau, un dossier synchronisé — où chacune dépose ses
enregistrements et lit ceux des autres, ce qui traverse les box sans
rien ouvrir. Qui tient le dossier ne lit rien.

Retirer une officine arrête ce qu'elle enverra ; ce qu'elle a déjà
envoyé reste au journal.

Les valeurs de pharmacocinétique sourcées voyagent par le même chemin :
ce qu'une officine a lu dans un RCP profite aux autres, avec sa source.
Et la barre du comptoir (F9) dit une rupture avant tout le reste, avec
ce que les collègues ont donné à la place.

# Les postes de l'officine

Chaque poste peut garder **sa propre base**, et recevoir tout ce que les
autres écrivent : dossiers, registre, caisse, planning, agenda, fiches,
réglages. « Postes de l'officine… », dans Options › Base. Tout voyage
chiffré sous une clé que seuls les postes de l'officine détiennent ; le
réseau d'officines a la sienne et n'ouvre rien de tout cela.

Un poste **fonde** le groupe : sa base garde tout, et ses numéros de
dossier. Il **invite** ensuite les autres, comme pour le réseau : une
porte ouverte, une adresse composée, un code de cinq groupes lu des deux
côtés. **Rejoindre remplace ce que le poste contenait** par les données
du groupe — faites une copie avant si elle compte.

Ensuite, sur le réseau local, les postes se trouvent et se synchronisent
**tout seuls**, quelques secondes après chaque écriture. S'ils ne se
voient pas (réseau cloisonné, deux sites), un dossier d'échange ou des
adresses écrites dans Options › Base font le chemin ; le bouton
« Synchroniser » et la fermeture restent là.

Deux postes ne donnent jamais le même numéro de dossier : chacun a son
bloc (le deuxième poste commence à 100 000). Une modification reçue ne
s'applique que sur la valeur qu'elle remplaçait : deux postes qui
modifient deux champs d'un dossier ne se gênent pas ; le même champ des
deux côtés attend « à arbitrer » — « Garder la mienne » ou « Prendre la
leur ». Rien n'est écrasé en silence.

**L'ordonnancier n'a qu'une suite** : seul le poste de référence
numérote. Une délivrance écrite sur un autre poste se lit « en attente »
jusqu'à ce qu'il la reçoive — quelques secondes sur le réseau local. Le
registre, la caisse et les journaux restent en ajout seul en voyageant :
une modification reçue pour eux est refusée, et le refus se voit. Le
poste de référence est aussi celui qui installe le contenu livré avec
les mises à jour.

Les pièces scannées restent sur le poste qui les a numérisées.

# Les connexions

La marque à trois nœuds, en bas à droite de la barre d'état, dit combien
de postes du groupe on entend (« 2/3 poste(s) ») et si l'officine fait
partie d'un réseau ; un clic ouvre la vue « Connexions ». Elle est
bleue quand tous les postes répondent, rouge quand une synchronisation
a échoué.

**Tout se connecte au lancement** : les postes du groupe se cherchent et
se synchronisent sur le réseau local, et le réseau d'officines se
synchronise au lancement puis toutes les quinze minutes (Options ›
Base). Un poste seul écoute seulement : la vue montre les postes qui
s'annoncent, avec leur adresse, et « Rejoindre… » la reprend. Rejoindre
demande toujours le code de cinq groupes, lu des deux côtés — c'est ce
qui garantit que personne ne s'est glissé entre les deux postes.

La vue a quatre panneaux : les postes (en ligne, hors ligne, retirés),
le réseau d'officines (les officines appairées, la dernière
synchronisation, la prochaine), ce qui attend d'être arbitré (entre
postes, et les versions de fiches reçues d'autres officines), et
l'activité depuis le lancement. « Tout synchroniser » parle tout de
suite aux postes entendus et aux officines du réseau.

# La caisse

Le comptage se fait en centimes entiers, jamais en flottants. L'écart
avec la recette attendue est **énoncé, jamais résorbé** : le compte
n'est pas recalculé depuis ce qui était attendu.

Sans recette attendue, il n'y a pas d'écart et la ligne reste vide.

La recette encaissée est le tiroir **moins le fond trouvé à
l'ouverture**, plus la carte et les chèques. Ce fond est pré-rempli avec
celui que le dernier comptage a laissé ; il se corrige s'il a changé
entre-temps.

Un soir recompté est une **seconde ligne**, pas une correction : le mois
garde le dernier comptage de chaque soir et montre les autres barrés.

# Ce qui s'imprime

Chaque document imprimable a un modèle éditable — le bouton
« Modèles… » de la barre du haut. Un
modèle est du Typst ; les `{{MARQUEURS}}` qu'il accepte sont listés dans
l'éditeur, et l'aperçu passe par la même fonction que l'impression.

Les phrases qui partent sur du papier au nom de l'officine se
réécrivent, dans la vue qui montre le document ou dans l'écran « Textes
imprimés ». Ce qui est réécrit est rangé dans la base et vaut donc pour
tous les postes.

Les **libellés de l'interface** — ce que l'application dit d'elle-même :
boutons, invites, infobulles — se relisent et se réécrivent dans
« Libellés ». Ils vivent dans un fichier à côté de la configuration,
donc par poste, et une réécriture s'affiche à la prochaine ouverture.

Une réécriture se souvient de la phrase qu'elle remplaçait. Si la phrase
livrée change, la réécriture est montrée à relire plutôt que posée sur
une autre phrase.

# La base

Un seul fichier chiffré, que plusieurs postes peuvent partager. Le
registre des stupéfiants et les pièces scannées ont chacun le leur, à
côté.

Les sauvegardes sont quotidiennes et gardées en nombre fixé ; « Copier
la base… » emporte les trois fichiers. Changer le mot de passe rechiffre
les trois.

Supprimer des pièces ne rend pas la place : seul « Compacter » le fait.

Ce qui appartient à l'officine — son identité, l'équipe, les horaires —
est rangé dans la base et vaut pour tous les postes. Le reste de
`config.toml` appartient au poste. Ce qu'un autre poste y change arrive
tout seul, sans qu'on ait à se reverrouiller — sauf pendant qu'un
dialogue est ouvert, où l'on n'écrit pas sous les doigts de quelqu'un.

**Les trois fichiers en un seul.** « Exporter en un fichier… » écrit la
base, les pièces et le registre dans un seul paquet chiffré du même mot
de passe, à emporter ou à archiver. « Importer un paquet… » les rend
dans un dossier **vide** : rien n'est écrasé, et la base y pointe au
redémarrage.

# L'annuaire des prescripteurs

Options › Base, « Importer un annuaire… » : un fichier de prescripteurs
exporté de l'annuaire santé ou de votre propre logiciel. Les colonnes y
sont trouvées **par leur nom** et non par leur rang — ces fichiers
changent d'ordre d'une version à l'autre —, et un fichier qui ne porte
pas de colonne de nom est refusé en disant ce qu'il portait.

L'annuaire précédent est remplacé et non complété : c'est une
photographie à une date, et importer par-dessus laisserait les
praticiens partis sous ceux qui les remplacent.

Sous le champ « prescripteur » du registre, taper deux lettres propose
les praticiens qui correspondent — par le nom, la ville, la spécialité
ou le numéro, qui est écrit sur l'ordonnance. **Il propose, il ne décide
pas** : ce qui est tapé reste ce qui sera écrit tant que personne n'a
choisi une ligne, et un prescripteur que l'annuaire ne connaît pas
s'écrit comme avant.

**Le mettre à jour depuis une adresse.** Si l'officine en a une — un
export de son propre logiciel, un fichier posé sur son serveur, un jeu
public décompressé —, elle l'écrit dans `[prescribers] source_url` de
`config.toml` et un second bouton apparaît : « Mettre à jour depuis
l'adresse… ». Tant qu'aucune adresse n'est écrite, **le bouton n'existe
pas** et l'application n'ouvre aucune connexion pour cela.

Rien ne part tant que personne n'appuie, rien n'est envoyé — on demande
un fichier, on ne raconte rien —, et l'adresse doit être en `https` :
un annuaire de noms propres ne traverse pas le réseau en clair. Une
archive est reconnue et refusée, avec la consigne de la décompresser.

Le RPPS est vérifié par sa clé — onze chiffres dont le dernier prouve
les dix autres. Un numéro qui ne se prouve pas est **gardé** et compté à
part : c'est à l'officine de regarder, pas au logiciel de trancher.

# Les compteurs d'usage

Quelques nombres : combien de dossiers ouverts, de fiches consultées, de
documents imprimés, de lignes au registre, sur combien de journées. Ils
se lisent dans **bpm-audit**, la fenêtre d'audit de l'officine.

Le plus utile est le dernier : « écritures concurrentes signalées »
compte les fois où un autre poste avait écrit le premier et où celui-ci
s'est rechargé plutôt que d'écraser. C'est ainsi qu'on découvre que deux
personnes travaillent sur la même chose au même moment, et aucun autre
écran ne le dit.

Ils ne sortent pas. Il n'y a pas d'adresse à régler et il n'y en aura
pas : ils vivent dans la base chiffrée de l'officine et se lisent dans
bpm-audit.

Ils comptent le logiciel, jamais la personne — aucun opérateur, aucune
initiale. « Combien de dossiers ont été ouverts » est une question sur
laquelle on décide ; « combien un tel en a ouverts » n'en est pas une.

Allumés au départ, éteints en un clic dans Options › À propos, et poste
par poste. Décocher arrête le comptage tout de suite et ne perd rien :
effacer est un bouton à part, qui demande deux fois.

# Le journal des accès

Qui a ouvert quel dossier, et quand — lu lui aussi dans bpm-audit. C'est l'exact contraire des compteurs — ceux-là comptent le
logiciel et jamais la personne, celui-ci nomme la personne et jamais le
logiciel, parce qu'il n'existe que pour répondre à « qui a regardé ce
dossier-là, le 12 mars ».

Une ligne porte un **numéro** de dossier et jamais un nom : ce qui
s'imprime doit permettre de remonter au patient, pas de l'afficher.
Sans initiales déclarées dans Options › Interface, la ligne écrit un
tiret — elle ne devine pas.

Trois gestes sont tracés : ouvrir un dossier, exporter, et lire la liste
des patients depuis la console — qui la rend noms compris. L'impression
ne l'est pas, et mieux vaut le savoir que le croire.

La durée de conservation est dans `[audit] keep_days` de `config.toml`,
un an au départ ; au-delà, les lignes sont purgées à l'ouverture de la
séance, et la purge s'écrit dans le journal qu'elle purge. `0` ne purge
rien.

# La fenêtre d'audit

**bpm-audit** est un second programme, livré à côté de l'application :
on l'ouvre depuis l'arrière-boutique, il demande le mot de passe de la
base (ou le trouve dans le trousseau), et il lit quatre volets sur la
période choisie — 7, 30, 90 ou 365 jours : l'activité, les accès aux
dossiers, l'usage du logiciel et les contrôles. Il n'écrit rien dans la
base. « Copier le rapport » et « Enregistrer le rapport… » rendent le
relevé en texte, le même que la commande ci-dessous.

Les deux programmes partagent le même code de lecture de la base : il
n'y a pas deux écritures du schéma, et donc pas de jour où elles
divergent.

# Le rapport d'audit

`bpm-caddy audit` écrit ce relevé sur la sortie standard, au lieu
d'ouvrir une fenêtre.

    bpm-caddy audit --jours 90 > audit-septembre.txt

Trois sections. **L'activité** : les actes par nature et par opérateur,
les lignes portées au registre, les caisses comptées et l'écart cumulé —
annoncé sur les soirs où il est calculé, qui ne sont pas tous ceux qui
ont été comptés. **Les accès** : ce que le journal ci-dessus contient
sur la période. **La conformité** : les libellés de classe que le
référentiel ne sait pas replier, les phrases réécrites que la version
livrée a périmées, les fiches sans DCI ou sans classe, les produits du
registre à aller compter, et les locations dont le renouvellement est
dépassé.

Il ne nomme que les opérateurs : un dossier y est désigné par son
numéro, comme au registre.

Il ne demande rien et n'ouvre rien : le mot de passe vient de
`BPM_CADDY_PASSWORD` ou du trousseau du système, ce qui le rend posable
dans une tâche de nuit. Sous Linux seulement ; ailleurs il le dit.

# La console

Un endroit pour poser à la base une question que personne n'a prévue, en
quelques lignes. Le détail de ce qu'elle sait lire est plus bas, dans
« L'API de la console ».

**Ce qu'on tape est coloré**, et cinq natures se distinguent : les
commentaires, les chaînes, les nombres, les mots du langage et les
appels que la console connaît vraiment. Ce dernier point est le plus
utile : un nom d'appel qui reste de la couleur ordinaire est un nom que
le moteur ne connaît pas — une faute de frappe se voit avant d'exécuter.

**Et elle propose.** Dès qu'un mot est commencé, une liste s'ouvre sous
le curseur : les flèches haut et bas la parcourent, la tabulation écrit
ce qui est pointé, Échap la referme. Rien ne s'ouvre sur un mot déjà
fini ni sur le vide.
