# BPM-Caddy

Le suivi des entretiens pharmaceutiques au comptoir : les dossiers, les
actes, l'agenda de l'équipe, le registre des stupéfiants, la caisse — et
ce qui s'imprime au nom de l'officine.

Tout est local. La base est chiffrée et ne sort pas du poste ; rien n'est
envoyé nulle part. La seule requête réseau de l'application est la
recherche d'une mise à jour, et elle ne part que si on presse le bouton
qui la demande.

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
scripts et les textes imprimés. Tapez ce dont vous vous souvenez — le
nom, un bout du nom, ou les initiales.

La recherche ignore la casse et les accents, et accepte les lettres dans
l'ordre sans qu'elles se suivent : « jndp » retrouve Jean Dupont.

Dans les monographies, « Dans le texte… » cherche les **mots** des
fiches et non leur titre : il rend chaque fiche qui dit le mot, avec la
phrase qui le porte.

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

Neuf chapitres sur la même liste : ce que les fiches en disent les unes
des autres — cité, rien n'est déduit —, les croisements sur les
cytochromes, le temps que met une exposition déplacée à revenir, la
revue d'ordonnance — doublons, associations, cascades —, ce que la
clairance change si on la tape, ce que le foie change au stade qu'on
désigne, ce que l'âge change, la grossesse et l'allaitement, et
« peut-on écraser ? ».

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

# La barre au-dessus des autres fenêtres

`F9` réduit la fenêtre à une barre de quelques centaines de pixels,
posée au-dessus des autres applications — c'est la forme qu'on garde
dans un coin d'écran pendant qu'on travaille ailleurs. On tape un nom
ou une molécule ; les flèches parcourent les fiches qui répondent,
Entrée ouvre celle qu'on lit, Échap efface la question puis, sur une
question déjà vide, rend la fenêtre. Le champ garde le foyer, si bien
qu'une douchette — qui est un clavier — tape dedans sans rien installer.
**Alt et 1 à 5** appellent les cinq gestes du bas, dans leur ordre : la
barre se conduit alors entièrement au clavier, ce qui est la façon dont
on s'en sert pendant qu'on tient une boîte de l'autre main.

**Cinq pages, et les flèches gauche et droite pour tourner.** Haut et
bas parcourent les fiches qui répondent, gauche et droite ce que la
fiche lue dit : deux questions perpendiculaires, deux paires de flèches.
« Signaux » porte ce que les tables disent ; « Posologie » la prose de
la fiche **et ses lignes indication par indication**, parce que
« combien » n'a pas de réponse sans « pour quoi » ; « Conseils » ce
qu'on dit à la personne et ce qu'on fait d'une prise oubliée, qui
sortent par la même bouche au même moment ; « Précautions » ce qui
contre-indique, ce qui arrive et ce qu'on surveille. Une page qui n'a
rien à dire n'est pas offerte, et quand une seule parle il n'y a pas de
bande d'onglets.

La cinquième, « Dossier », est la seule qui ne parle pas de la fiche
cherchée : c'est l'ordonnance ouverte, ligne par ligne, et **une ligne
cliquée devient la question**. Un liseré marque les lignes sur
lesquelles une table demande qu'on s'arrête — le tri du comptoir, avant
de les ouvrir une par une. Il ne marque que cela : « à vérifier » et
« sans donnée » sont des réponses, pas des arrêts, et les marquer
marquerait toute l'ordonnance. C'est le geste de la révision au
comptoir — descendre une ordonnance en posant la même question à chaque
ligne —, qui demandait jusqu'ici de retaper huit noms dont on ne se
rappelle ni l'orthographe ni le dosage. Sans dossier ouvert, la page
n'est pas là.

Elle **rapporte ce que les tables disent**, en pastilles, et ne conclut
rien à leur place : chaque pastille cite le mot de sa table, et son
survol donne la portée du module avant la conduite. Neuf lectures : ce
que les fiches du dossier disent de celle-ci, ce que la revue
d'ordonnance en dit une fois qu'on l'ajoute aux autres, **si le dossier
porte déjà cette molécule sous un autre nom**, ce que ce traitement
demande qu'on mesure et depuis combien de temps personne ne l'a demandé,
les cytochromes, « peut-on écraser ? », la grossesse et l'allaitement,
ce que la clairance du dossier change, et ce que l'âge change — l'âge est au dossier depuis le jour où la fiche a été créée, et
c'est bien pour cela que personne ne le regarde.

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

Les molécules, les doses et les durées sont **celles des tables de
conversion** qu'on lit au comptoir, dans le même ordre et les mêmes
mots : la table qu'on consulte et le document qu'on remet ne peuvent pas
dire deux choses différentes, et un test le tient.

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

Le numéro de délivrance est attribué au moment de l'écriture et n'est
jamais réattribué : une ligne annulée garde le sien, et la suite
continue après lui.

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

# La caisse

Le comptage se fait en centimes entiers, jamais en flottants. L'écart
avec la recette attendue est **énoncé, jamais résorbé** : le compte
n'est pas recalculé depuis ce qui était attendu.

Sans recette attendue, il n'y a pas d'écart et la ligne reste vide.

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
`config.toml` appartient au poste.

# La console

Un endroit pour poser à la base une question que personne n'a prévue, en
quelques lignes. Le détail de ce qu'elle sait lire est plus bas, dans
« L'API de la console ».
