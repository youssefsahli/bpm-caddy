# Changelog

All notable changes to BPM-Caddy will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.193.0] - 2026-09-12

### Added
- **La qualité d'une personne est une catégorie, sans cesser d'être du
  texte libre.** Sept qualités d'officine — pharmacien titulaire,
  adjoint, remplaçant, étudiant en pharmacie, préparateur, apprenti
  préparateur, rayonnagiste — se choisissent dans un menu, et la case
  qui s'imprime reste à côté : c'est elle qui part au bas d'un document,
  et une officine doit pouvoir y écrire « Pharmacien adjoint, DU de
  nutrition ».

  Le menu **lit** ce qui est écrit plutôt que de le remplacer, sur le
  modèle du référentiel des classes : les libellés réellement tapés se
  replient — féminins et abréviations compris, « Préparatrice » comme
  « PRÉPARATEUR EN PHARMACIE » —, et ce que la liste ne connaît pas reste
  lisible et se range en « Autre ». Rien à migrer : les qualités déjà
  saisies se lisent sans que la ligne change.

- **Et la bande de couverture dit si un pharmacien est là.** « Deux
  personnes au comptoir » ne répond pas à la question qu'une officine se
  pose en premier. Au survol, quand aucun pharmacien n'est inscrit à la
  tranche, la bande le dit.

  Trois états et non deux, pour la même raison que partout ici : **« on
  ne sait pas » n'est pas « non »**. Un étudiant en pharmacie n'est pas
  un pharmacien — il exerce sous la responsabilité de l'un d'eux, et
  c'est justement la distinction qu'un logiciel a le droit de
  connaître —, mais une qualité écrite à la main ne répond ni oui ni
  non, et la phrase dit alors combien de qualités elle n'a pas su lire.
  Rien ne s'affiche tant que l'officine n'a déclaré aucune qualité : la
  même règle que les creux, qui ne se lisent que contre des horaires
  écrits.

## [0.192.0] - 2026-09-12

### Fixed
- **« Poser la trame » ne disait pas ce qu'il avait remplacé.** Le
  commentaire du code l'affirmait, la phrase ne le portait pas : « Trame
  posée : 10 poste(s) » après en avoir effacé six est exactement le
  silence que « Remplacer » ne doit pas avoir. Elle dit les deux
  nombres.
- Trois commentaires de règles avaient vieilli d'une version : la
  relecture d'une trame déclarait encore « deux postes le même jour »
  illisible — c'est une journée coupée depuis 0.187.0 —, et les deux
  fonctions qui écrivent et relisent annonçaient moins de règles
  qu'elles n'en tiennent. Un commentaire faux est pire qu'un commentaire
  absent : c'est celui qu'on croit.

## [0.191.0] - 2026-09-12

### Added
- **« Retirer la trame » — le geste symétrique de « Poser ».** Il
  manquait : la grille de la semaine supprime une ligne rangée à la
  fois, ce qui fait neuf gestes pour une semaine de journées coupées, et
  neuf occasions d'en oublier une. La fenêtre retire ce qu'elle montre,
  et rien d'autre — les postes ponctuels et les absences ne sont pas
  touchés —, les exceptions partent avec leur ligne rangée, et
  « Annuler » remet le tout.

### Fixed
- Le retour arrière d'un retrait de trame s'annonçait « la trame de 0
  poste(s) », qui est exactement le libellé qu'on ne reconnaît pas, donc
  qu'on n'ose pas presser. Il dit maintenant « le retrait de la trame
  (N ligne(s)) ».

## [0.190.0] - 2026-09-12

### Added
- **La bande de couverture dit qui est là, et pas seulement combien.**
  Elle répondait « jusqu'à 3 personnes » — une question que personne ne
  pose. Celle qu'on pose devant un plan de journée est « puis-je prendre
  quelqu'un à 15 h, et avec qui ? ». Le survol nomme maintenant la
  tranche sous le pointeur : « 15 h 00 — CL, YS », ou « personne au
  comptoir ».

  Nommer et compter sont **la même question**, et c'est la même fonction
  qui y répond : `planning::who_is_in` rend les initiales, et la bande
  dénombre ce qu'elle rend. Deux calculs auraient fini par montrer trois
  carrés au-dessus de deux noms. Les mêmes exclusions valent pour les
  deux, et pour les mêmes raisons : l'astreinte porte des heures mais
  n'est pas devant le patient, et une absence ne tient rien du tout.

## [0.189.0] - 2026-09-12

### Added
- **La trame posée rend la semaine qu'elle promettait** — le seul test
  qui parcoure la chaîne entière : la grille de la fenêtre, l'écriture
  en base, le dépliage, et la semaine qu'on relit. Chaque maillon avait
  le sien ; celui-ci vérifie qu'ils sont attachés, sur une trame qui
  porte à la fois une alternance, une journée coupée et une journée
  hebdomadaire, lue sur deux semaines de parité contraire. Il finit par
  la raison d'être de la journée coupée : le comptoir est vide de midi
  et demi à deux heures, ce qu'un poste de 9 h à 19 h 30 avec une pause
  de quatre-vingt-dix minutes n'aurait jamais dit.
- La règle « on raccourcit, on n'élide pas » gagne son test, mené dans
  un contexte egui sans écran — la seule façon de mesurer un texte dans
  la fonte qui le dessinera.

### Fixed
- **Une date de fin antérieure à la première occurrence posait des
  lignes que personne ne verrait jamais.** « Les semaines paires jusqu'au
  10 » réglé pendant une semaine impaire écrit une trame dont la
  première occurrence tombe *après* sa fin : rien ne s'affiche, et on
  croit avoir posé ses horaires. La fenêtre le dit avant, et l'écriture
  le refuse.

## [0.188.0] - 2026-09-12

### Fixed
- **Une case du planning coupée en « 14 h–… » disait « sans fin ».** La
  colonne des jours faisait treize caractères écrits en dur, et une
  journée coupée — « 9 h–12 h 30 · 14 h–19 h » — en demande le double :
  elle s'élidait donc en « 14 h–… », qui est exactement ce que la grille
  écrit d'un poste dont personne n'a noté la fin. Deux choses
  différentes sous une même apparence est la seule élision qu'une case
  d'horaire ne peut pas se permettre. La colonne est mesurée sur ce
  qu'elle porte, bornée, et la grille défile au-delà — au mois comme à
  la semaine.
- **La fenêtre de trame débordait de l'écran.** Une fenêtre grandit avec
  son contenu : les huit colonnes d'une journée coupée, à
  `text_scale = 1,6`, la poussaient au-delà des bords, et comme elle est
  centrée, « Lundi » se lisait « undi ». Elle est désormais bornée à
  l'écran, et son corps défile dans les deux sens plutôt que de pousser
  ses murs.
- **« CL YS ·… » laissait croire qu'il manquait un nom.** La ligne qui
  dit qui tient le comptoir sous chaque jour de la semaine s'élidait ;
  elle prend maintenant la plus riche des écritures qui tienne — « CL YS
  · 14 h 00 », puis « CL YS » —, et ne se dessine pas du tout quand
  aucune ne tient. C'est de la garniture, et une garniture coupée n'en
  est plus une. Les noms passent avant les heures : la ligne existe pour
  répondre à « qui tient le comptoir jeudi ? » sans ouvrir le planning.

  La règle est écrite une fois, dans `richest_form` : **on raccourcit, on
  n'élide pas**, et les trois endroits qui la suivent — l'en-tête du
  jour, cette ligne-ci, et la colonne des jours — y passent tous.

## [0.187.0] - 2026-09-12

### Added
- **La journée coupée, écrite comme deux postes.** C'est la forme la
  plus ordinaire d'une officine française — 9 h – 12 h 30 puis 14 h –
  19 h 30 — et la fenêtre de trame ne savait pas la dire : elle n'avait
  qu'une rangée d'heures par jour, si bien qu'une journée coupée se
  lisait « illisible », ou s'écrivait comme un seul poste à longue
  pause.

  Or **une pause n'a pas d'heure.** C'est une durée, et la bande de
  couverture compte donc la personne au comptoir pendant sa coupure :
  une officine où tout le monde déjeune de midi et demi à deux
  s'annonçait tenue, et le creux ne se voyait nulle part. Deux postes
  disent *où* est le trou. La grille de la semaine sait d'ailleurs déjà
  les montrer — « 9 h–12 h 30 · 14 h–19 h » dans une case.

  La trame porte donc deux demi-journées par jour, la durée additionne
  les deux, et la relecture reconnaît une journée coupée : le poste le
  plus tôt tient la première moitié, quel que soit l'ordre où la base
  rend les deux lignes. Ce qu'elle refuse toujours, et pour la même
  raison qu'avant : **deux natures différentes le même jour** — la
  grille n'en montre qu'une, et la seconde tomberait en silence — et un
  troisième poste, puisqu'il n'y a que deux moitiés.

- **La règle du clavier est sortie du dessin.** « Sept pas à droite font
  une semaine, et rien ne se saute » est désormais une fonction pure
  avec son test : au bord, la semaine tourne et la case retombe de
  l'autre côté ; une case restée sur une autre semaine ramène à un bord
  plutôt que de figer la touche, parce qu'une touche qui ne fait rien
  laisse croire qu'elle n'existe pas.

- La démo ouvre **à midi fermé** — deux plages par jour, ce que le
  commentaire de `[pharmacy] horaires` appelle depuis toujours « le cas
  ordinaire » et que rien n'exerçait —, et Claire y travaille en journée
  coupée. Sa formation du jeudi ne remplace donc plus que sa matinée,
  ce qui est exactement ce qu'une exception doit savoir faire.

### Fixed
- **Cliquer une case du mois effaçait la sélection** : « Modifier » et
  « Supprimer » disparaissaient au moment même où l'on venait de
  désigner le jour sur lequel on voulait agir. La case se choisit, comme
  dans la semaine.
- Sans équipe déclarée, le sélecteur de personne de la trame est un champ
  libre, et il se relisait à **chaque frappe** : « C » puis « CL » sont
  deux personnes qui n'existent pas, et la grille se vidait lettre après
  lettre. Il attend maintenant qu'on sorte du champ.
- L'écriture serrée des cases du mois — « 9–12h30 » — gagne son test.
  Elle partage sa construction avec l'écriture longue, et c'est ce test
  qui garantit que les deux ne se mettent pas à dire deux choses.
- **La formation de la démo ne contredisait plus rien.** Sa ligne se
  cherchait par son rang (`OFFSET 3`), qui désignait le jeudi tant que
  Claire avait un poste par jour et le mardi matin dès que sa journée
  s'est coupée en deux : l'exception nommait une occurrence inexistante,
  donc n'effaçait rien, et la démo perdait sa formation sans que rien ne
  le dise. Elle se cherche par son jour.

## [0.186.0] - 2026-09-12

### Added
- **La trame se relit.** La fenêtre ne savait qu'ajouter : ouverte pour
  quelqu'un qui a déjà des horaires, elle montrait une grille vide, et
  « Poser » écrivait une *seconde* trame par-dessus la première. La
  personne travaillait deux fois et rien à l'écran ne le disait. Or le
  geste ordinaire n'est pas « ajouter une trame », c'est « corriger le
  mercredi de Claire ».

  Elle s'ouvre donc sur la trame de la personne choisie, et elle suit le
  menu : changer de personne, c'est ouvrir la sienne. « Remplacer » se
  coche alors tout seul — le seul endroit où cette case le fait — et la
  phrase au-dessus dit pourquoi.

- **Une journée hebdomadaire est celle des deux semaines.** C'est ce qui
  rend l'écran capable de montrer la trame qu'une officine écrit
  vraiment : du lundi au vendredi toutes les semaines, plus un samedi
  sur deux. Ce mélange de rythmes — le plus ordinaire qui soit — se
  serait lu « illisible » sans cette règle, puisque « chaque semaine »
  n'est ni les paires ni les impaires.

  Et la règle joue **dans les deux sens** : une journée identique sur les
  deux onglets se réécrit une fois, hebdomadaire, plutôt que deux fois en
  parités. C'est ce qui rend l'aller-retour exact — relire une trame puis
  la reposer sans rien changer redonne les mêmes lignes, six et non onze
  —, et c'est un test qui le tient.

  Trois autres règles, une par test : une alternance se relit comme une
  alternance, l'onglet des paires en tête et quel que soit l'ordre où la
  base rend les deux lignes ; une ligne d'avant les rythmes se relit
  (sept jours de `repeat_days` sans clé sont une trame hebdomadaire, et
  rien d'autre ne se devine) ; et **ce que deux semaines de sept jours ne
  portent pas n'est pas approximé**. Deux postes le même jour, un pas
  qu'on ne sait pas ranger, une date illisible : l'écran le dit, laisse
  la grille vide et **ne coche pas « Remplacer »** — approximer
  reviendrait à écraser ce qu'on n'avait pas su montrer.

- La ligne qui annonce ce que la trame va poser compte **trois sortes de
  lignes** et non deux : « 6 postes : 5 chaque semaine et 1 en
  alternance », suivi des dates de celles qui sautent une semaine, qui
  sont les seules qui surprennent. Une journée hebdomadaire annoncée
  « sur l'autre semaine » était fausse — elle est sur les deux.

- Une garde fait l'aller-retour : la base range « 26:00 », que personne
  n'écrit et que l'horloge du jour ne relit même pas ; le champ rend
  « 02:00 », et l'écriture la repousse au-delà de minuit toute seule.

- **Le creux se lit dans le pied du planning**, comme il se lit déjà sur
  le mois : le total d'un jour passe à l'encre d'alerte quand l'officine
  est ouverte et que personne n'est inscrit. La grille disait « 14 h 00 »
  sans dire que ces quatorze heures laissaient le comptoir vide de midi à
  deux, ce qui est la seule question qu'on pose à un planning. Le calcul
  est celui qui servait déjà au mois : il n'y a pas deux calculs de creux
  dans cette application.

- Trois fonctions pures du planning gagnent leur test : la fin d'un poste
  telle qu'on la tape, la relecture d'une heure rangée, et ce qu'une case
  du planning écrit — les heures, ou l'absence qui explique qu'il n'y en
  ait pas.

- **Le planning se lit aussi au mois, et par personne.** « Quand est-ce
  que je travaille le mois prochain ? » est la première question qu'on
  pose à un horaire, et la grille de la semaine y répondait en quatre
  clics sur la flèche — une semaine à la fois, sans jamais montrer le
  rythme.

  Un bouton « Mois » dans la rangée de saisie, et la même grille de
  postes se lit dans l'autre sens : une personne sur cinq semaines
  plutôt que l'équipe sur sept jours. Les flèches déplacent alors le
  mois, puisque c'est ce qu'on regarde, et le titre dit lequel et de
  qui — un tableau qui ne porte que des numéros de jour ne se lit pas.

  La première colonne porte le **numéro de semaine ISO**, et ce n'est pas
  un ornement : c'est lui qui rend le rythme lisible. Un samedi sur deux
  se voit comme une colonne qui s'allume aux numéros pairs et s'éteint
  aux impairs — ce qu'aucune semaine regardée seule ne peut montrer, et
  ce qui permet de vérifier d'un coup d'œil qu'une trame de parité tombe
  bien où on la croyait. Elle porte aussi le total de la semaine, et elle
  est **gelée au bord gauche** comme celle des noms : sur un écran de
  comptoir la grille défile sur trois jours, et « 9–12h30 » lu sans
  savoir de quelle semaine ne dit rien.

  Le numéro du jour dit le **creux** — c'est une propriété du comptoir,
  pas de la personne — et les heures disent la personne, de sa couleur.
  Le total du mois est en pied, et il dit combien de postes n'ont pas de
  fin écrite : un total seul se lirait comme le mois entier.

  Deux boutons disparaissent au mois : « Imprimer » et « Recopier » sont
  des gestes de semaine, et un bouton qui agirait sur une semaine qu'on
  ne regarde pas est un bouton qu'on presse une fois et qu'on n'ose plus
  presser. Les rangées, elles, remplissent le volet : un mois est un
  tableau qu'on lit de loin, et il a la place d'être lu.

- **Le planning se parcourt au clavier.** Les flèches déplacent la
  *case* choisie — de jour en jour, de personne en personne — et
  « Modifier » et « Supprimer » suivent ce qu'on vient d'atteindre. Le
  pas de semaine reste sur les ‹ › et sur « Aujourd'hui », qui sont à
  deux centimètres, là où arriver sur une case demandait la souris ;
  passer le bord du lundi ou du dimanche fait tourner la semaine et
  retombe de l'autre côté, si bien qu'on ne perd rien en chemin. Rien de
  cela ne prend le clavier à un champ de texte, ni à la fenêtre de
  trame — les flèches y déplaceraient une case *derrière* elle, et on ne
  le verrait qu'en la fermant. Au mois, elles déplacent le mois, qui est
  ce qu'on regarde.

### Fixed
- **« Remplacer la trame » aurait emporté les congés.** Une absence posée
  en plage — « du 12 au 26 octobre » — est une ligne qui revient tous les
  jours jusqu'à une date : elle répète, donc elle passait pour une trame,
  et réécrire les horaires de quelqu'un lui aurait supprimé ses vacances.
  Une absence n'est pas une trame, et ce que « Remplacer » retire est
  désormais **exactement ce que la fenêtre a montré** — la liste est
  retenue à la lecture au lieu d'être redemandée à l'écriture, si bien
  qu'une ligne que l'écran n'a pas su afficher, et pour laquelle il a dit
  qu'il ne toucherait à rien, ne peut plus disparaître.
- **Les sept en-têtes de la semaine se touchaient.** « Lun 07/09 » était
  peint sans borne au centre de sa colonne, et un `Painter` peint où on
  lui dit — rien ne le clipe : à `text_scale = 1,6` la rangée se lisait
  « Lun 07/09Mar 08/09Mer 09/09 ». L'en-tête prend désormais la forme la
  plus riche qui tienne — « Lun 07/09 », « Lun 07 », « 07 ». On
  raccourcit plutôt que d'élider : « Lun 07/0… » a perdu le mois *et* se
  lit cassé, là où « Lun 07 » ne dit pas le mois et se lit entier — or le
  mois est déjà écrit au-dessus de la grille. Trouvé en regardant, à
  1024x700 et à l'échelle 1,6.

## [0.185.0] - 2026-09-11

### Added
- **Le planning sait enfin dire « les semaines paires ».** Il ne savait
  dire que deux choses : ce jour-là, ou toutes les semaines. Or une
  officine ne travaille pas comme cela — on est là les semaines paires,
  un samedi sur deux, une semaine sur trois au dépôt. Écrire cela à la
  main, c'était poser vingt-six lignes par an et par personne, et se
  tromper d'une.

  Huit rythmes, rangés en deux familles que `src/planning.rs` **refuse
  de confondre** : celles qui se lisent sur le calendrier — les semaines
  paires, les impaires — et celles qui se comptent depuis le jour posé —
  une semaine sur deux, sur trois, sur quatre. La différence a l'air
  d'un détail et n'en est pas un : **une année ISO compte 52 ou 53
  semaines**, et au premier passage d'une année de 53, une trame écrite
  tous les quatorze jours cesse *pour toujours* de tomber sur les
  semaines paires. Le 31 décembre 2026 est en semaine 53, le 4 janvier
  2027 en semaine 1 : deux impaires de suite. C'est pour cette raison
  que la base range un rythme et non un nombre de jours, que les deux
  parités avancent de sept jours en s'en tenant une sur deux, et qu'un
  test tient la divergence date par date.

  `src/date.rs` gagne le jour de la semaine et le numéro de semaine ISO,
  avec sa règle en une phrase — **une semaine appartient à l'année de
  son jeudi** —, tenue sur chaque jour de deux siècles. C'est le
  calendrier de la maison, et il n'y en a toujours qu'un.

- **« Trame de la semaine » : sept journées et un rythme, en une
  fois.** L'écran qui manquait. Poser « 9 h – 19 h 30 du lundi au
  vendredi, les semaines paires » demandait cinq fois la rangée de
  saisie, puis cinq de plus pour l'autre semaine — dix gestes dont aucun
  ne montrait ce que les autres avaient écrit.

  Une grille de sept jours, et **deux onglets quand c'est une
  alternance** : la semaine paire et l'impaire côte à côte, avec un
  bouton pour recopier l'une sur l'autre. Trois choses lui donnent sa
  forme, et ce sont trois réponses à « de quoi doute-t-on en remplissant
  ce tableau ? » — le total de la semaine s'écrit au fur et à mesure,
  par jour et en bas ; la semaine de départ **dit son numéro et sa
  parité** (« la semaine du 07/09/2026 est la semaine 37 : impaire »),
  qui est la seule phrase qui lève le doute sur l'onglet qu'on remplit ;
  et les prochaines occurrences sont nommées **par leur date**. Régler
  « semaines paires » une semaine impaire ne pose rien cette semaine-là,
  et sans cette ligne on le découvre le mercredi suivant.

  Deux commandes font gagner les six autres rangées — « Lundi à
  vendredi », « Lundi à samedi » recopient la première journée écrite —
  et « Remplacer » retire d'abord les trames déjà posées pour la
  personne, parce qu'en poser une par-dessus une autre fait quelqu'un
  qui travaille deux fois et que rien à l'écran ne le dirait. Le tout
  est **un seul geste**, qu'un seul `Ctrl+Z` défait : les postes écrits
  s'enlèvent et les trames retirées reviennent, avec leurs exceptions.

  Quatre règles tenues par un test sans écran : une journée sans heure
  de début n'écrit rien, chaque journée se pose sur **son** jour de la
  semaine que la date nomme — en arrière comme en avant, sans quoi une
  trame réglée un mercredi poserait son lundi huit jours plus tard et
  les sept journées ne formeraient plus une semaine —, une alternance
  écrit deux trames indépendantes, et le pas rangé est celui du rythme,
  ce qui garde « Recopier » exact.

- **Un congé se pose comme une plage.** « Tous les jours » est le
  huitième rythme, et le seul qui **exige** une date de fin : « du 12 au
  26 octobre » devient une ligne rangée au lieu de quinze, et sans fin
  écrite ce n'est pas un congé mais quelqu'un d'absent pour toujours —
  la ligne est refusée et le dit. Le champ « jusqu'au » n'apparaît que
  lorsqu'un rythme le rend possible : sur un poste d'un seul jour il
  n'aurait rien à borner.

  Et il ne s'applique qu'à ce qui revient : changer de rythme pour « ce
  jour-là » fait disparaître le champ mais pas ce qu'on y avait tapé, et
  appliqué quand même il aurait écrit en base un poste que l'écran
  n'aurait jamais montré.
- **« Ce jour seulement » — l'exception, enfin écrite depuis l'écran.**
  La base la connaissait depuis le premier jour : une ligne qui remplace
  *une* occurrence d'une trame sans y toucher, `supersedes` et
  `cancelled`, la famille lue avant suppression, le retour arrière qui
  renumérote les liens. Le changelog de 0.175.0 disait déjà « Claire est
  absente mardi prochain ne doit pas effacer les mardis de Claire ».

  **Rien à l'écran n'en écrivait jamais une.** « Supprimer » emportait la
  trame entière, et il n'y avait pas d'autre issue. Le mécanisme était
  complet de la base au retour arrière, et inaccessible — ce qui ne se
  voit dans aucun test, puisque chaque moitié marchait.

  Une bascule, qui n'apparaît que sur l'occurrence d'une trame : les
  deux boutons portent alors sur ce jour-là et le disent (« Modifier ce
  jour », « Retirer ce jour »). Et sur une case qui *est* une exception,
  le bouton ne supprime pas, il **rétablit** — le jour que la trame
  portait revient, et la trame n'est pas touchée. Le nommer
  « Supprimer » y ferait croire qu'on efface la série ; c'est d'ailleurs
  ce qu'il faisait, puisqu'il visait la ligne rangée et non la case
  sélectionnée.
- Le rythme d'un poste se lit **au survol de sa case** : « 9 h – 19 h 30 »
  ne disait pas si c'était ce jour-là ou toutes les semaines paires, et
  c'est pourtant la première chose à savoir avant de supprimer, puisque
  supprimer emporte la série. Une case qui contredit sa trame le dit
  aussi : elle ressemble en tout à une case ordinaire, et c'est la seule
  dont « Supprimer » ne fait pas ce qu'on croit.
- **Les entrées de l'agenda reviennent au même rythme que les postes.**
  Elles en avaient un autre — « Une fois », « Chaque semaine »,
  « Toutes les 2 semaines », « Toutes les 4 semaines », un nombre de
  jours —, si bien qu'il y avait deux vocabulaires du « à quelle
  fréquence » dans le même écran, et que le plus visible des deux ne
  savait pas dire « les semaines paires ». C'est la même énumération, le
  même dépliage et le même calendrier des deux côtés maintenant ; la
  ligne d'une entrée qui revient porte **le nom de son rythme** plutôt
  que « · tous les 7 j », qui était un calcul là où « Chaque semaine »
  est ce qu'on avait choisi. Le quotidien n'y est pas offert : il exige
  une date de fin, la rangée de saisie d'une entrée n'en demande pas, et
  une réunion qui revient tous les jours pour toujours est mille
  occurrences dépliées à chaque lecture.
- La démo sème **un samedi en alternance** — Maya les semaines paires,
  Yanis les impaires —, qui est la façon dont une officine s'organise
  vraiment et le seul endroit où une capture montre une trame qui saute
  une semaine. Et le planning s'ouvre **avec une case déjà choisie**,
  comme la caisse s'ouvre sur un tiroir déjà compté : « Modifier »,
  « Supprimer » et la bascule n'existent que sur une sélection, et sans
  elle aucune capture ne les montrait.

### Removed
- **`[pharmacy] operators couleur`.** Le champ promettait « la couleur
  de cette personne au planning » et **rien ne l'a jamais lu** : une
  ligne de configuration qu'on pouvait remplir sans effet, livrée en
  même temps que l'écran qu'elle devait colorer. La
  promesse, elle, est tenue : le nom d'une personne est désormais peint
  de sa couleur dans la grille, déduite de ses initiales comme au
  journal des notes. C'est mieux qu'un champ : la même sur tous les
  postes, et elle ne bouge pas quand on insère quelqu'un au milieu de
  l'équipe — ce que le commentaire du champ donnait précisément comme
  son défaut.
- **L'horaire contractuel et le relevé d'heures.** `heures_semaine` sur
  chaque opérateur, l'écart « 34 h 15 / 35 h 00 » dans la grille, et la
  feuille « Relevé d'heures » avec ses deux cases de signature.

  Le module l'écrivait depuis le premier jour et ne s'y tenait pas
  jusqu'au bout : `day_total` compte une **présence**, pas une paie. Ce
  qu'on déduit d'un total d'heures — l'écart au contrat comme la
  majoration — est du droit du travail, il change, et une application de
  pharmacie qui l'imprime se trompera un jour sans que personne le voie.
  Un relevé mensuel destiné à la comptabilité était exactement le papier
  par lequel cela arrive. Le planning de l'équipe, lui, reste : c'est un
  horaire à punaiser, pas une pièce comptable.

### Fixed
- Les champs d'heure d'une cellule de `Grid` annonçaient leur largeur
  par `desired_width`, **qui ne l'annonce pas** : la colonne se rabattait
  sur ce que le texte déjà tapé demandait, et « 19h30 » rendait « 19h ».
  `add_sized`, comme le veut la règle des cellules de grille.
- La fenêtre de la trame mesurait plus haut que l'écran à 1024x700 et à
  `text_scale = 1,6` — et centrée, elle débordait des deux côtés : son
  titre coupé en haut, « Poser la trame » coupé en bas. Le corps défile,
  les boutons prennent leur propre rangée, et la largeur voulue suit
  l'échelle du texte plutôt que de rester à un nombre de pixels qui
  porte sept colonnes à l'échelle 1 et cinq à 1,6.
- **Défaire la suppression d'une exception ne réinsérait rien.** Le
  retour arrière cherchait, en tête de la famille, la ligne rangée qu'il
  devait renuméroter ; une exception supprimée seule est une famille
  d'une ligne, dont le `supersedes` nomme une trame qui, elle, n'a jamais
  bougé. Faute de trouver le rangé, il passait son chemin : le geste
  avait l'air défait et ne l'était pas — le pire des deux. Trouvé en
  suivant le chemin à la main, et tenu par un test qui supprime une
  exception seule et la remet.
- **La rangée de saisie se mesurait comme si tout y était un bouton**,
  ce qui oubliait trois choses : un menu déroulant est plus large que le
  texte qu'il affiche, la date du jour s'y écrit sans être un libellé, et
  un champ se mesure sur son gabarit. La bande sortait une rangée trop
  courte et « Supprimer » tombait hors du volet — sur l'écran même où il
  vient d'apparaître. Le modèle suit la rangée pièce par pièce.
- **La rangée de saisie du planning mangeait la grille.** « La saisie
  gagne » réglait l'arbitrage tant qu'elle tenait en deux rangées : on
  lui laissait tout sauf une ligne. À `text_scale = 1,6` sur un écran de
  comptoir elle en fait quatre, et il ne restait à la grille que sa
  ligne d'en-têtes — rien de ce pour quoi l'écran existe. Les deux
  défilent, chacune dans sa région : le partage se fait donc à la
  moitié, sur une **rangée entière**, et les deux moitiés défilent.

## [0.184.0] - 2026-09-11

### Added
- **Tous les textes imprimés se réécrivent.** Les fiches, les
  préparations, les dispositifs, les protocoles et les cellules des
  tables s'éditaient déjà. Sept cent soixante-douze phrases y
  échappaient, et ce sont précisément celles qui partent sur du papier
  au nom de l'officine : les six carnets du patient, les points de la
  fiche d'entretien, la feuille « peut-on écraser ? », le plan de
  surveillance, les interprétations de biologie, la revue d'ordonnance,
  la grossesse et l'allaitement, l'adaptation rénale, les conseils d'une
  ordonnance TROD, les vaccins du voyageur. Une tournure qui ne
  convenait pas était une tournure à subir.

  **Deux portes, un seul éditeur.** « Réécrire » dans la vue qui montre
  le document — la bonne porte quand on a la feuille sous les yeux — et
  l'écran « Textes imprimés », qui les liste tous : la porte de l'autre
  cas, quand on se souvient d'une phrase sans se souvenir d'où elle
  vient. Les deux passent par la même liste de phrases, celle du module ;
  deux listes des mêmes phrases finiraient par différer.

  Ce qui est réécrit est **rangé dans la base**, donc vaut pour tous les
  postes, et l'écriture se fait contre ce que l'écran affichait.

  **Une réécriture se souvient de la phrase qu'elle remplaçait** et ne
  s'applique que tant que celle-ci n'a pas changé. C'est ce qui rend
  tenable l'adressage d'une consigne par son rang, faute d'autre repère :
  le jour où une liste est réordonnée, la réécriture est montrée à relire
  — avec ce qu'on avait écrit et ce qu'elle visait — au lieu d'être posée
  sur une autre phrase.

  **Retaper le texte livré rétablit la phrase** : la ligne s'en va, et la
  phrase recommence à suivre les corrections des versions suivantes.

  **Une règle est adressée par ce qui l'identifie**, jamais par son rang
  ni par sa prose : le titre d'un point de revue, le libellé d'une
  présentation, le code d'un analyte, la molécule et le seuil de DFG d'un
  palier. Une adresse tirée du texte s'évanouirait le jour où on corrige
  ce texte — la réécriture ne deviendrait pas périmée, elle deviendrait
  introuvable. Chaque module porte le test qui prouve son identité
  unique : deux règles à la même adresse feraient hériter la seconde de
  la réécriture de la première, ce qui sur la feuille d'écrasement est un
  comprimé à libération prolongée écrasé.

### Fixed
- **Le bilan imprimait l'ancienne phrase après une réécriture.** La
  lecture d'intervalle — « Kaliémie 5,4 mmol/L — élevé (3,5 – 5). »
  suivie de la note de l'analyte — n'est pas une règle : elle est
  composée à la lecture. Réécrire cette note changeait l'infobulle à
  l'écran et laissait le bilan dire ce qu'il disait avant, sans que rien
  ne le signale. Trouvé en construisant l'édition, et c'est le pire
  défaut que cette fonction pouvait avoir : croire une phrase corrigée et
  voir le papier dire autre chose.
- **Le pire de deux niveaux de grossesse était calculé à l'envers** dans
  la couche de résolution : la table est déclarée du plus grave au moins
  grave, donc le pire est le plus petit. Une contre-indication de
  grossesse se serait affichée comme une prudence d'allaitement.

## [0.183.0] - 2026-09-11

### Added
- **L'officine est rangée dans la base, et non dans le fichier du
  poste.** `config.toml` est un fichier par PC : l'équipe déclarée sur
  celui du comptoir n'existait pas sur celui de l'arrière-boutique, et le
  nom de la pharmacie, l'adresse, le pharmacien signataire, le numéro AM
  et les horaires se retapaient sur chacun. C'est la raison qui avait
  déjà mis les notes d'équipe et les scripts à côté de la base plutôt
  qu'à côté de la configuration : la base est ce que les postes
  partagent.

  **Rien n'est perdu à la reprise** : le premier lancement de cette
  version sur une base qui n'en porte pas encore y verse ce que ce poste
  avait dans son fichier, et les postes suivants le lisent au lieu de le
  redemander. `[pharmacy]` reste dans `config.toml`, comme graine, et le
  fichier le dit maintenant en toutes lettres.

  L'enregistrement se fait **contre ce que l'écran avait sous les yeux** :
  deux postes qui ouvrent les Options en même temps ne s'écrasent plus,
  le second se voit refuser, et l'écran lui montre ce que le premier a
  écrit plutôt que d'effacer la personne qu'il venait d'ajouter.
- **La synchronisation automatique entre postes.** La base est partagée,
  et chaque écriture se faisait déjà contre les valeurs affichées : vingt
  messages « modifié depuis un autre poste » existent pour cela. Mais ils
  arrivaient tous **au moment d'écrire**. Entre-temps l'écran montrait ce
  que la base portait à son ouverture, et rien ne disait qu'il avait
  vieilli — on lisait une liste de rendez-vous d'il y a une heure, on
  rappelait un patient que l'autre poste avait déjà rappelé, et on ne
  l'apprenait qu'en enregistrant. Si on enregistrait.

  Le témoin est `PRAGMA data_version`, qui bouge quand une *autre*
  connexion valide et reste immobile pour ce que celle-ci écrit.
  Pourquoi lui plutôt qu'un compteur de révision tenu à la main : un
  compteur se pose dans chaque transaction d'écriture, et la première
  qu'on oublie est une modification qu'aucun poste ne voit jamais — un
  manque silencieux, la pire espèce. Le pragma ne peut pas être oublié,
  y compris par une version future.

  Cela **remplace** la relecture aveugle toutes les soixante secondes :
  une requête par minute sur le partage de l'officine pour presque
  toujours relire ce qu'on avait déjà, et malgré tout une minute de
  retard. Maintenant : trois pragmas toutes les deux secondes, et une
  relecture seulement quand quelqu'un d'autre a écrit. Elle couvre en
  plus ce que l'ancienne laissait dehors — le registre, le codex, les
  dispositifs, le fil du dossier, les pièces.

  **Elle ne touche jamais à ce qui est en train d'être tapé** : les
  listes et les résumés sont des lectures et se rechargent, les tampons
  de saisie appartiennent à la personne qui a les doigts dessus. Un test
  le tient, vérifié en l'enfreignant. Et la barre d'état le dit quelques
  secondes, parce qu'une liste qui change sous les yeux de quelqu'un sans
  rien dire lui fait croire qu'il a mal lu.

- **La saisie groupée du registre : une feuille, plusieurs produits.**
  Le formulaire du registre écrit une ligne sur un produit, ce qui est
  la bonne forme pour la délivrance qui arrive au comptoir. Deux gestes
  n'y entraient pas : l'inventaire du coffre — quarante produits, une
  date, un opérateur — et l'ordonnance qui porte deux stupéfiants, seize
  Actiskenan et cinq Durogesic. Faits produit par produit, ce sont
  autant d'allers-retours qu'il y a de lignes, à rechoisir chaque fois
  le produit, la nature et la date.

  Un onglet, tous les produits suivis dans l'ordre alphabétique — celui
  de la base, qui est déjà celui de l'étagère —, une case en face de
  chacun. Ce qui est commun se saisit une fois en haut : la nature, le
  jour, le dossier et le prescripteur d'une délivrance, le grossiste et
  le bon de livraison d'une réception. Ce qui appartient à un produit
  est sur sa ligne : la quantité, et le motif — qui se donne case par
  case, parce que deux produits qui manquent ne manquent pas pour la
  même raison.

  **Une case vide n'est pas un zéro**, et c'est la règle qui fait tenir
  tout le reste : une feuille de quarante produits dont on en compte six
  écrirait sinon trente-quatre inventaires à zéro, c'est-à-dire qu'elle
  viderait le coffre sur le papier. Une case illisible non plus — « 1O »
  avec un O se refuse et se dit. Chaque embarras s'écrit **en face de sa
  case** et non après le refus : sur une feuille de dix produits, un
  refus qui ne nomme pas la ligne fautive laisse chercher laquelle.

  **Et la feuille part entière ou ne part pas.** L'écriture est une
  transaction : une seule ligne refusée et rien n'est inscrit. Une
  feuille à moitié écrite est le pire état où laisser un registre —
  trois lignes sur cinq sont passées, rien ne dit lesquelles, et rien ne
  s'y efface. Les numéros d'ordonnancier se suivent à l'intérieur de la
  feuille : deux délivrances d'un même geste ne peuvent pas porter le
  même.

  Le **lot** n'y est pas, délibérément : un lot est le numéro d'une
  boîte et pas d'une feuille, et une seule case par produit ne saurait
  pas en porter deux. Une réception qu'il faut tracer au lot se fait
  ligne à ligne, où le champ existe.

### Changed
- **Les textes de l'interface, relus.** Les tournures en « ce que / ce
  qui / ce qu'il », les questions rhétoriques et les chutes de phrase en
  aphorisme sont remplacées par le terme professionnel : « Ce que cela
  donne » devient « Récapitulatif », « Ce qu'on vise » « Objectifs »,
  « Le plus loin du compte » « Écarts les plus importants ». Cent
  dix-huit chaînes dans `strings.fr.toml`, plus la liste de points de
  l'entretien imprimée sur chaque fiche, neuf libellés de tables de
  conversion, les titres imprimés et le mode d'emploi.
- **Et la prose clinique, au même titre.** Cent soixante-dix passages
  des monographies, des tables, de la biologie, de la surveillance et de
  la revue d'ordonnance : les clivées « X est ce qui Y » et « c'est ce
  qui Y » redeviennent « X Y », les formules d'appareil — « il est
  important de », « il convient de », « il ne s'agit pas de » —
  disparaissent au profit de l'impératif ou de la négation directe, et
  les clichés (« n'est pas anodin », « le vrai danger », « ne veut rien
  dire ») cèdent la place au terme clinique : « n'est pas interprétable »,
  « le risque principal ».

  **Deux constructions ont été gardées, délibérément.** « Ce n'est pas A,
  c'est B » reste partout où la distinction *est* le contenu clinique —
  « ce n'est pas de l'anxiété, c'est un effet du traitement » (akathisie),
  « ce n'est pas une infection, c'est la réaction attendue » (fièvre
  post-vaccinale), « ce n'est pas un bon résultat, c'est un risque
  d'hypoglycémie » (HbA1c basse chez le sujet âgé) : douze phrases où
  supprimer la tournure supprimerait ce qu'il y a à dire. Et « rien
  d'autre » reste là où il est une consigne et non une emphase — « c'est
  le 15 et rien d'autre ».

  Au passage, une double négation qui disait le contraire de son
  intention : le raloxifène « n'a pas l'effet protecteur du tamoxifène
  sur rien d'autre que la colonne » se lit maintenant « n'a l'effet
  protecteur du tamoxifène que sur la colonne ».

### Fixed
- **Trois tests partageaient un dossier temporaire avec un autre.** Ils
  tournent en parallèle dans le même processus : deux qui portent le
  même nom s'effacent la base l'un de l'autre, et le perdant échoue sur
  « disk I/O error » au hasard de l'ordonnancement. Trouvé en l'ayant
  provoqué — un test ajouté ici tombait sur le nom d'un test existant.

## [0.182.0] - 2026-09-10

### Added
- **« Tout » : la fin d'un entretien en un bouton et un seul PDF.**
  Fiche d'entretien, bilan et plan de prise, remplis et compilés d'un
  coup — c'était quatre boutons dans trois écrans, à la fin d'un
  rendez-vous où l'on est déjà en retard. Chaque partie est construite
  par la **même fonction** que son bouton : deux assemblages d'une même
  page finissent toujours par diverger, et c'est celui qu'on regarde le
  moins qui a tort.

  Le carnet de suivi n'y est pas, et c'est un choix : lequel ? Tension,
  glycémie, INR, souffle — le choisir est une décision, et une liasse qui
  devinerait imprimerait la mauvaise grille.
- **Ctrl+K trouve le registre, les carnets et les scripts.** Il savait
  les dossiers, les fiches, les tables, les préparations, les dispositifs
  et les protocoles ; il ignorait trois écrans qu'on atteignait en
  cliquant trois fois alors qu'on savait déjà comment ils s'appellent.

  Un produit suivi ouvre le registre sur *lui* ; un carnet ouvre sa
  feuille — et se trouve par son titre **et par son propos**, parce qu'on
  cherche « tension » quand la feuille s'appelle « Automesure
  tensionnelle » ; un script ouvre la console avec son texte.

## [0.181.0] - 2026-09-10

### Added
- **Grossesse et allaitement, comme niveau et non comme paragraphe.**
  Une ordonnance de treize lignes chez une femme enceinte, ce sont treize
  paragraphes à ouvrir. L'onglet « Grossesse » de la biologie en fait
  treize niveaux, et l'ordonnance se lit d'un coup — avec, sur l'onglet,
  le compte de celles qui demandent qu'on s'arrête.

  **Ce panneau ne remplace pas le CRAT**, et il le dit en pied. La
  référence française est tenue à jour molécule par molécule et elle est
  en ligne ; une table figée dans un binaire vieillit, elle non. Ce qui
  est ici est un rappel de comptoir qui cite sa source et dit où aller.

  Cinq règles, un test chacune. **« Pas de donnée » n'est pas « pas de
  risque »** : une molécule absente de la table reçoit « à vérifier au
  CRAT », jamais « compatible ». **Grossesse et allaitement sont deux
  questions** — la codéine est possible enceinte et déconseillée en
  allaitant, les AVK l'inverse, et un test exige qu'au moins un quart de
  la table réponde différemment des deux côtés, sinon deux colonnes
  seraient une colonne. **Le terme change la réponse** : un AINS n'est
  pas « à éviter », il est contre-indiqué à partir de vingt-quatre
  semaines d'aménorrhée, même en prise unique ; et pour l'aspirine c'est
  la dose qui décide, la même molécule étant un traitement de la
  grossesse à faible dose. **Une contre-indication dit ce qu'on met à la
  place.** Et **la table ne décide de rien** : l'arrêt d'un traitement
  chez une femme enceinte est une décision médicale, et une grossesse mal
  accompagnée est plus dangereuse qu'un traitement poursuivi.

### Fixed
- Les tables cliniques écrivaient du balisage — « **à partir de 24 SA** »
  — que `RichText` ne sait pas interpréter : les astérisques sortaient à
  l'écran telles quelles. Vu sur une capture, corrigé dans les trois
  tables, et un test par table le refuse désormais.

## [0.180.0] - 2026-09-10

### Added
- **« Peut-on écraser ? », en une feuille.** Une infirmière d'EHPAD
  appelle avec un pilulier et huit lignes ; un aidant demande devant le
  comptoir si le comprimé passe dans la compote. La réponse existait :
  écrite quelque part dans le paragraphe « forme » d'une fiche, à ouvrir
  et à traduire une par une. Le bouton « Écraser ? » du dossier imprime
  l'ordonnance entière, ligne par ligne, avec ce qu'on peut en faire et
  par quoi remplacer ce qu'on ne peut pas.

  Cinq règles, un test chacune, et la première est la seule qui compte
  vraiment : **le silence n'est pas une permission.** Une molécule que
  la table ne connaît pas reçoit une réponse — « à vérifier » — et
  jamais l'absence de ligne. Une feuille qui ne montrerait que les
  interdits se lirait comme une autorisation pour tout le reste, et
  c'est ainsi qu'on écrase un comprimé à libération prolongée.

  **La forme décide, pas la molécule** : Moscontin jamais, Skenan en
  ouvrant la gélule — une table par DCI répondrait faux une fois sur
  deux. **Ouvrir une gélule n'est pas écraser un comprimé** : trois
  réponses et non deux, parce que le « oui mais » est le cas le plus
  fréquent en gériatrie. **Un « non » sans solution laisse le problème
  entier** : chaque refus dit par quoi remplacer. Et **certains « non »
  protègent celui qui écrase**, pas le patient — un cytotoxique, un
  tératogène : c'est la poussière qui est le danger.

  La feuille rappelle aussi les trois choses qu'on oublie : un comprimé
  écrasé se donne aussitôt, un mortier se lave entre deux traitements, et
  « à vérifier » ne veut pas dire « oui ».

### Fixed
- Les six actions du dossier étaient six `&mut bool` passés à la même
  fonction — six occasions de brancher le mauvais au mauvais endroit,
  tous du même type, sans que le compilateur ait rien à dire. Un seul de
  ces boutons peut être pressé par image, et c'est ce que le type dit
  maintenant.

## [0.179.0] - 2026-09-10

### Added
- **L'adaptation rénale, comme règle et non comme paragraphe.** La
  biologie savait dire « ce chiffre, sous ce traitement, veut dire
  ceci » ; la surveillance, « ce chiffre n'a pas été demandé depuis trop
  longtemps ». Il manquait celle qu'on pose vraiment au comptoir :
  **ce dossier porte un DFG à 28, que devient chaque ligne de son
  ordonnance ?** Le pharmacien lisait le « rein » de la fiche d'un côté
  et le chiffre du laboratoire de l'autre, et rapprochait les deux de
  tête, ligne par ligne, sur une ordonnance qui en compte huit.

  Un onglet « Rein » dans la biologie du dossier lit l'ordonnance contre
  la clairance **la plus récente** — la plus récente et non la plus
  basse jamais vue : c'est l'état du rein aujourd'hui qui décide.
  Vingt-quatre molécules ou classes, chacune avec ses paliers et sa
  source : metformine, les quatre AOD, les AINS, la nitrofurantoïne, la
  colchicine, le méthotrexate, les anti-aldostérone, l'allopurinol, les
  bisphosphonates, les gabapentinoïdes, le baclofène, la digoxine,
  l'aciclovir, les fibrates, la rosuvastatine, le cotrimoxazole, la
  morphine, le tramadol, l'aténolol et le sotalol, l'amoxicilline, le
  lithium.

  Quatre règles, un test chacune. **Sans DFG, pas de verdict** : le
  panneau nomme ce qui dépend du rein et dit que le chiffre manque —
  avec **combien** de traitements l'attendent, parce que « aucun DFG »
  tout seul est une remarque là où « aucun DFG, et quatre lignes en
  dépendent » est une prise de sang à demander. **Le palier atteint est
  le plus bas des paliers franchis** : lue à 28, une molécule qui se
  réduit sous 60 et se contre-indique sous 30 est contre-indiquée, et
  non « à dose réduite ». **Un seuil vient du RCP, jamais d'une
  interpolation** — le module ne calcule pas une dose à partir d'une
  clairance. Et **la conduite est celle du RCP, la décision est celle du
  prescripteur**, écrit en pied du panneau et pas seulement dans le
  code.

## [0.178.0] - 2026-09-10

### Added
- **Les heures du mois, par personne**, dans « Statistiques » — l'écran
  des agrégats, qui n'a pas besoin d'un frère. Le panneau n'apparaît que
  si l'officine saisit un planning : un cadre vide sur chaque écran
  apprend à sauter le cadre.

  L'écart au contrat n'y est **pas**, et c'est un choix : un contrat est
  hebdomadaire, et en faire une cible mensuelle demande de décider
  combien de semaines compte un mois. C'est l'annualisation que ce
  module a dit qu'il ne connaîtrait pas. L'écart reste donc sur la ligne
  de la semaine, dans la grille, où il compare deux semaines.

### Fixed
- **Les légendes des graphiques en barres se peignaient par-dessus leurs
  barres.** La colonne des libellés était un nombre de pixels que chaque
  appelant devinait — 96, 130, 150, 160, 200 — et les cinq devinettes
  étaient fausses dans deux directions : « Méthadone AP-HP gélule 40 mg »
  perdait ses deux derniers mots *et* débordait sur sa barre, et à
  `[ui] text_scale = 1,6` c'était à peu près toutes les légendes. Un
  nombre de pixels ne suit pas l'échelle du texte.

  La colonne se mesure désormais dans `motif::chart`, seul endroit qui
  connaisse la fonte dans laquelle elle sera dessinée — et la colonne des
  valeurs avec elle, parce que « 124 h 30 » ne tient pas dans les
  quarante-six pixels que « 14 » demandait. Bornée à la moitié du
  panneau, et ce qui dépasse encore est élidé plutôt que peint sur la
  barre.
- Les panneaux de « Statistiques » choisissaient leur série **par leur
  rang dans une liste**, avec un bras `_` pour la dernière : insérer un
  panneau au milieu aurait dessiné la mauvaise série dans le bon cadre,
  sans rien casser et sans que rien ne le dise. Chacun nomme la sienne.

## [0.177.0] - 2026-09-10

### Added
- **« Recopier » : la semaine affichée, écrite sur la suivante.** La
  commande la plus utile d'un planning d'officine, et elle tient en deux
  règles. Une **trame hebdomadaire n'est pas recopiée** — elle couvre
  déjà la semaine suivante, et la doubler ferait un planning où chacun
  travaille deux fois. Et **ce qui est déjà posé sur la semaine
  d'arrivée gagne** : on recopie une trame, pas les absences de
  quelqu'un. Le nombre de postes écrits est dit ; un bouton silencieux
  laisse croire qu'il a fait quelque chose.
- **Un retour arrière sur le planning** — `Ctrl+Z`, et un bouton qui
  **nomme** ce qu'il va défaire (« le poste posé pour CL », « les 12
  postes recopiés ») : un geste qu'on ne reconnaît pas, on ne le presse
  pas. Vingt gestes gardés, en session et rien en base — un retour
  arrière qui survivrait à la nuit défferait le geste de quelqu'un
  d'autre.

  Défaire une suppression **réinsère**, donc les identifiants changent.
  Et comme une exception nomme sa ligne rangée, la réinsertion réécrit
  le lien de chaque exception avec l'identifiant que la base vient de
  rendre : sans cela, annuler la suppression d'une trame rendrait la
  trame et perdrait les absences qui la corrigeaient.
- Le titre du panneau porte le total de la semaine **et sur combien de
  jours il porte** : un jour dont un poste n'a pas de fin écrite n'a pas
  de total et n'entre pas dans la somme, et « 50 h 00 sur la semaine »
  se lirait comme la semaine entière alors qu'il y manque un jeudi.
  C'est la discipline de l'écart cumulé de la caisse. Il est dans le
  titre parce que le pied de la grille est la dernière rangée d'une
  région défilante — sur un écran de comptoir, exactement ce qu'on ne
  voit pas.

## [0.176.0] - 2026-09-10

### Added
- **Deux feuilles pour le planning.** *Planning de l'équipe* : la
  semaine affichée, une ligne par personne, à punaiser en
  arrière-boutique. *Relevé d'heures* : le mois d'une personne, jour par
  jour, avec le total, l'écart au contrat quand il est écrit, et deux
  cases de signature. Chacune a son modèle éditable dans Options ›
  Modèles, comme tout ce qui s'imprime ici.

  Le relevé porte en pied qu'il compte des **présences saisies à la main
  et non un pointage** — il n'enregistre pas les heures d'arrivée et de
  départ réelles, et ne porte ni majoration, ni heure supplémentaire, ni
  décompte de convention collective. C'est la phrase la plus importante
  de la feuille : sans elle, un papier imprimé par un logiciel se lit
  comme une mesure, et ce n'en est pas une.
- **La semaine et le mois disent qui tient le comptoir.** Sous chaque
  jour de la semaine, les initiales des présents et le total d'heures —
  en rouge s'il reste un creux pendant l'ouverture. Le mois porte les
  heures du jour en second chiffre quand la case est assez haute, et le
  numéro du jour passe en rouge quand il reste un creux, ce qui, lui, ne
  coûte pas un pixel et se voit à toutes les tailles.

  Et la règle qui va avec : **un jour dont personne n'a rien saisi n'est
  pas un creux.** Le rouge dit « quelqu'un est prévu, et il manque quand
  même du monde », jamais « rien n'est écrit ». Une officine qui déclare
  ses horaires avant d'avoir saisi son premier planning ne voit pas son
  mois entier en rouge.

## [0.175.0] - 2026-09-10

### Added
- **Le planning de l'équipe : qui est là, quand, et combien d'heures
  cela fait.** Un quatrième mode de l'agenda, cadré sur la semaine :
  une ligne par personne, sept colonnes de jours, le total de chacun à
  droite et le total de chaque jour en pied. La colonne des noms est
  gelée au bord gauche — une case qui dit « 9 h–19 h 30 » sans dire de
  qui ne dit rien —, et une ligne « non attribué » ferme la grille,
  parce qu'un poste écrit au nom d'une initiale que l'équipe ne connaît
  pas doit se voir quelque part.

  On y écrit : poser un poste, corriger ses heures, le supprimer.
  « Toutes les semaines » écrit **une trame**, pas cinquante-deux
  lignes — et c'est ce qui permet à une absence d'en contredire un seul
  jour sans effacer les autres. « Claire est absente mardi prochain » ne
  doit pas effacer les mardis de Claire : la ligne rangée reste, une
  seconde ligne la remplace ce jour-là, et la semaine dit la vérité.
  C'est l'idée de l'annulation du registre, sans son inaltérabilité —
  un planning se rectifie, un registre non.

  `src/planning.rs` porte les règles, une par test. **Les heures se
  comptent en minutes entières**, jamais en heures décimales : sept
  heures trente-cinq est 455, la même discipline qu'aux centimes de la
  caisse et pour la même raison. **Un poste sans fin n'est pas un poste
  de zéro heure** — la ligne affiche « — », et le total du jour aussi.
  **Une nuit est comptée au jour qui la commence** : une garde de 20 h à
  2 h fait six heures, en entier, au jour de début, et la semaine ne les
  voit pas deux fois. **Deux postes de la même personne qui se touchent
  ne sont pas un conflit** — 9 h–12 h 30 puis 14 h–19 h 30 est une
  journée coupée à midi. Et **une pause plus longue que le poste est
  refusée, pas soustraite** : une durée négative se propage dans la
  semaine sans se voir.

  Ce que le module ne fait pas, et ne fera pas : la paie. Ni majoration,
  ni heure supplémentaire, ni convention collective. Il compte des
  présences saisies ; ce qu'on en déduit est du droit du travail, il
  change, et une application de pharmacie qui imprimerait « dont 2 h
  majorées » se tromperait un jour sans que personne le voie.
- **La couverture du comptoir, le long des heures du plan de journée.**
  À chaque quart d'heure, combien de personnes sont là — une colonne
  étroite collée à la gouttière, qui se lit sans qu'on la cherche. Le
  rouge est le creux : une tranche pendant laquelle l'officine est
  ouverte et où personne n'est inscrit. Et **sans horaires d'ouverture
  écrits il n'y a pas de creux** : la bande se dessine quand même, elle
  compte des têtes ; ce qui disparaît, c'est le rouge. Une officine qui
  n'a rien déclaré ne se fait pas dire tous les matins qu'elle n'ouvre
  pas — la même règle qu'à la caisse.
- `[pharmacy] horaires` (les heures d'ouverture, deux lignes pour un
  jour à midi fermé) et, sur chaque opérateur, `heures_semaine` (le
  contrat — **sans contrat écrit, pas d'écart au contrat**) et
  `couleur`. Livrés vides.

### Fixed
- **La bande de l'agenda se coupait en son milieu.** Elle se mesurait
  sur « Semaine du {} » et dessinait « Semaine du 07/09/2026 », dix
  caractères de plus ; son plafond, arrondi à une hauteur de bouton,
  tombait au milieu d'une rangée dessinée. Elle prend désormais ce
  qu'elle a **réellement occupé** à l'image précédente. Et
  l'étiquette de la semaine ne s'enveloppe plus : dans une rangée
  enveloppée, egui la coupait en deux et la moitié basse se peignait
  par-dessus la rangée du dessus.

## [0.174.0] - 2026-09-10

### Added
- **L'agenda sait enfin ce qu'est un chevauchement — et le filtre
  n'efface plus rien.** Le plan de journée plaçait ses blocs par seau
  d'heure : 9 h 00 et 9 h 45 étaient dessinées côte à côte alors
  qu'elles ne se rencontrent jamais, la troisième entrée d'une même
  heure repeignait la première, et un rendez-vous était haut d'une ligne
  quelle que soit sa durée. `src/agenda.rs` remplace cela par de vraies
  voies : un coloriage du graphe d'intervalles, dont le nombre de
  colonnes est le nombre d'entrées qui partagent réellement une minute.
  Deux blocs qui en partagent une portent un liseré rouge — un conflit
  n'est pas une erreur, une officine en prend, à deux personnes, donc
  c'est un liseré et jamais un refus.

  Les rendez-vous portent désormais leur **durée**, leur **lieu** (au
  comptoir ou à distance) et l'**opérateur** qui les prend : les trois
  colonnes étaient en base depuis leurs migrations, seule la requête de
  l'agenda les laissait derrière.

  Deux interrupteurs « Au comptoir » · « À distance » s'ajoutent aux
  pastilles par type d'acte, et la règle qui les tient est la
  fonction : **un filtre n'efface pas, il éteint — et il n'éteint pas ce
  qui chevauche ce qu'il garde.** Un entretien à distance de 14 h 00 à
  14 h 30 reste dessiné, en aplat éteint, quand une vaccination au
  comptoir est posée à 14 h 15 : la cacher fabriquerait le conflit
  qu'elle devait montrer, et la question qu'on pose à un agenda est
  « puis-je prendre quelqu'un à 14 h 15 ? ». Ce qui ne passe pas le
  filtre et ne rencontre rien devient un trait de 3 px contre la
  gouttière des heures — la journée garde sa densité vraie, et le survol
  le nomme. Un compteur dit « 12 rendez-vous · 3 masqués · 1
  chevauchement » : le mot « masqués » est ce qui empêche de lire une
  journée filtrée comme une journée vide, et une journée vide est une
  journée où l'on prend un rendez-vous de plus.

  La semaine pâlit ses blocs éteints sans changer leur ligne et porte
  une pastille rouge sur le jour qui a un chevauchement ; le mois porte
  le même point dans le coin de la case. Et parce que la bande est
  plafonnée au tiers du volet — à `[ui] text_scale = 1,6` sur un écran
  de comptoir, les boutons de mode prennent à eux seuls les trois
  rangées qu'elle a —, le titre du panneau porte « JOUR — 2 masqué(s) »
  quand un filtre est posé. Un filtre qui éteint des rendez-vous sans
  qu'on puisse voir qu'il est là serait pire que pas de filtre.
- **Un rendez-vous est dessiné à sa durée.** Le plancher d'un bloc était
  une heure entière : tout ce qui dure moins — c'est-à-dire à peu près
  tous les entretiens — se dessinait comme un rendez-vous dont personne
  n'a noté la durée. Le plancher est désormais la ligne de texte du
  libellé, mesurée dans la fonte qui la dessine.

### Fixed
- **Filtrer l'agenda supprimait les rendez-vous.** La bande de filtres
  faisait un `retain` sur la liste de la session : ce qu'elle cachait,
  elle le jetait — le tableau de bord, le dock et la liste imprimée y
  perdaient les mêmes lignes jusqu'au rechargement suivant, sans que
  rien ne le dise. Le filtre est une lecture maintenant, posée à
  l'endroit du dessin.
- La légende de la semaine a disparu : elle disait exactement ce que la
  rangée de filtres dit déjà, à cela près qu'on ne pouvait pas cliquer
  dessus. Le calendrier récupère les deux lignes qu'elle prenait — à
  l'échelle 1,6, une rangée de rendez-vous de plus par jour.
- Un test de `motif` échouait une fois sur dix : `series_color(0)` est
  l'accent de la peau en vigueur, la peau est globale au processus, et
  ce test-là ne prenait pas le verrou que ses voisins prennent. Les deux
  moitiés d'une assertion pouvaient donc être lues sous deux peaux.

## [0.173.0] - 2026-09-10

### Added
- **L'historique de caisse : le mois, soir par soir.** Le comptage
  répondait à « le tiroir tombe-t-il juste ce soir ? » et à rien
  d'autre : les vingt derniers comptages défilaient en petites lignes
  dans un coin de la synthèse, sans total, sans écart cumulé, et sans
  moyen de remonter à février. La page « Historique » (le bouton de la
  vue Caisse, ou `caisses` dans la boîte de saut) porte le mois affiché
  soir par soir — espèces, autres encaissements, recette, attendu,
  écart, opérateur, remarque —, la bande des recettes par soir, le
  total, et la liste des soirs qui ne tombent pas juste, le plus gros
  écart d'abord. Les flèches changent de mois, Échap revient au
  comptage.

  Trois règles, et chacune est un test. **Un soir recompté ne compte
  qu'une fois** : la table est en insertion seule, un recomptage est une
  deuxième ligne, les deux restent à l'écran — la première nommée
  « recompté », en encre éteinte — et seule la dernière entre dans les
  totaux. Additionner les deux ferait une journée à double recette, ce
  qu'un total mensuel ne montre jamais. **L'écart cumulé dit sur combien
  de soirs il porte**, et quand aucun attendu n'a été saisi il n'y a pas
  de chiffre du tout, mais une phrase : « 0,00 € » se lirait « tout est
  tombé juste ». Et **un soir non compté n'est pas un soir à zéro
  euro** : il n'a ni ligne dans le tableau, ni case dans la bande, et ne
  pèse sur aucune moyenne.

  La page n'écrit rien. Un historique où l'on pourrait rattraper une
  soirée serait un historique qui ne prouve plus rien.
- **Trois feuilles de plus, et chacune a son modèle éditable.**
  - *Historique de caisse* — le mois qu'on garde ou qu'on donne au
    comptable, avec ses écarts et ce qu'ils font ensemble.
  - *Étiquettes de posologie* — le plan de prise découpé : une étiquette
    par traitement, à coller sur la boîte, avec la posologie **du
    dossier** et la phrase d'oubli de la fiche. La feuille A4 finit dans
    un tiroir ; l'étiquette reste sur ce qu'on ouvre. Le nom et la
    posologie passent entiers — ce sont eux qu'on lit sur la boîte —,
    c'est la phrase d'oubli qui est coupée, et un traitement qui n'en a
    pas donne une étiquette sans ligne vide.
  - *Plan de surveillance* — ce que l'ordonnance demande de faire doser
    et à quel rythme, avec une case à cocher devant ce qui est en retard
    ou n'a jamais été fait, et une colonne vide pour le laboratoire. Ce
    que `surveillance.rs` sait n'atteignait que l'écran et une section
    du bilan, alors que c'est la feuille qu'on emporte. **Aucun chiffre
    de résultat n'y figure** : une norme imprimée sur une feuille qui
    part à la maison est une invitation à s'interpréter seul.

### Changed
- Le plan de prise et les étiquettes lisent le dossier par **une seule**
  fonction (`treatment_lines`) : deux constructions d'une même liste
  finissent par diverger, et rien ne dirait alors laquelle a raison, la
  boîte ou la feuille.
- La base de démonstration sème vingt-quatre soirs de caisse, dont un
  recompté et deux sans recette attendue — sans quoi l'historique
  s'ouvrait sur une page vide et ne montrait rien de ce qu'il existe
  pour montrer.

- **La caisse peut se compter sans recette attendue.**
  `[ui] caisse_expected = false` (Options › Interface, « Comparer la
  caisse à une recette attendue ») retire le champ, la ligne d'écart, sa
  phrase, les deux colonnes « Attendu » et « Écart » de l'historique,
  l'écart cumulé, le plus loin du compte et la liste des soirs qui ne
  tombent pas juste. Reste ce qui est vrai sans attendu : le tiroir
  compté, la recette, les soirs du mois et leur bande.

  L'attendu se saisit à la main, et une officine qui ne le saisit pas
  voyait un champ laissé vide tous les soirs et deux colonnes de tirets.
  Vrai par défaut : c'est l'écart qui fait qu'un comptage se relit, et
  rien ne change pour qui s'en sert. **Ce qui est déjà rangé n'est pas
  effacé** — les attendus des soirs passés restent en base et
  reviennent, avec leurs écarts, le jour où la case est recochée : le
  filtre est à la lecture, en un seul endroit, et non un `if` à chaque
  chiffre dessiné.

  Une réserve, et elle est dite parce qu'elle se voit : sur la **feuille
  du soir**, « Recette attendue » et « Écart » restent, à « — ». Leurs
  libellés sont dans le modèle Typst, et c'est le modèle qu'on modifie
  pour les retirer du papier — deux lignes à effacer dans Options ›
  Modèles. Les mettre sous condition depuis le code reviendrait à
  remonter la mise en page dans le programme, ce que le registre des
  documents existe justement pour éviter.

### Fixed
- **Les montants à quatre chiffres s'écrivaient « 1□240,50 » à
  l'écran.** Le séparateur de milliers était l'espace fine insécable
  (U+202F), celle que la typographie française veut — et dont la fonte
  livrée avec l'application n'a pas de glyphe. C'est l'espace insécable
  ordinaire désormais, et le test des glyphes passe la **sortie** de
  `caisse::euros` dans la fonte qui la dessine : un caractère qu'aucune
  chaîne du fichier ne porte, parce qu'un format le produit, n'échappait
  à aucun test jusqu'ici.

## [0.172.0] - 2026-09-09

### Added
- **Le comptage de caisse, avec sa feuille à signer.** Une officine
  compte son tiroir tous les soirs, sur un carnet ou sur un coin de
  papier, et l'écart se perd entre les deux. La vue « Caisse » (`F`
  « caisse » dans la boîte de saut) porte les quinze coupures de l'euro,
  les encaissements qui ne sont pas dans le tiroir (carte, chèques : lus
  sur le ticket, pas comptés), le fond laissé pour demain, et ce que la
  journée devait faire.

  Trois règles, et chacune est un test. **L'argent se compte en centimes
  entiers, jamais en flottants** : douze pièces de dix centimes font
  1,20 € et non 1,1999999999999997, et un centime est exactement ce
  qu'un comptage de caisse existe pour voir — jusque dans la base, où la
  colonne est un `INTEGER`, parce que c'est en repassant par un `REAL`
  qu'un total juste redeviendrait faux. **Un écart n'est pas une
  correction** : il s'affiche, signé, en tête de la synthèse ; rien ne
  propose de le résorber en changeant le comptage, et la remarque est là
  pour l'expliquer — la discipline du registre des stupéfiants, appliquée
  à un tiroir. Et **sans recette attendue, il n'y a pas d'écart** : la
  ligne reste vide plutôt que d'annoncer « + 1 240,50 € d'excédent »
  tous les soirs.

  Les comptages se rangent dans la base et ne s'écrasent jamais : une
  caisse recomptée le même soir est une **deuxième ligne**, et les deux
  se lisent. La feuille imprimée porte le détail des coupures, la
  synthèse, l'écart et deux signatures.

### Changed
- **Chaque document imprimable a désormais son modèle éditable.** Quatre
  en avaient un — la fiche d'entretien, le courrier, le carnet,
  l'ordonnance — et vingt-deux n'en avaient pas : leur Typst était écrit
  en Rust, mise en page et données mélangées dans le même `format!`. Une
  officine qui voulait sa marge, son en-tête ou sa police sur la liste
  d'appel, le bilan, la monographie ou le registre n'avait rien à ouvrir.

  Les vingt-six sont maintenant au même registre (`pdf::DOCS`) : une
  clé, un nom, un modèle par défaut, la liste des marqueurs. Options ›
  Modèles parcourt ce registre au lieu d'énumérer quatre cas —
  **ajouter un document imprimable, c'est ajouter une ligne**, et il
  apparaît dans l'éditeur. Les modèles vivent dans `[templates] dir`
  (`modeles/` à côté de `config.toml` par défaut), un `<clé>.typ` par
  document ; les quatre chemins historiques restent lus tels quels, pour
  qu'une officine qui en a écrit un ne perde pas sa mise en page.

  Quatre règles, chacune tenue par un test : les marqueurs déclarés et
  ceux du modèle par défaut sont la même liste ; un modèle rempli ne
  contient plus de `{{` (un marqueur mal tapé s'imprimait en toutes
  lettres au milieu de la page — il est maintenant nommé et le modèle
  refusé) ; chaque modèle par défaut compile avec ses valeurs d'exemple,
  qui sont aussi celles de l'aperçu, de sorte que l'aperçu et
  l'impression passent par **une seule** fonction ; et toute fonction
  `open_*` de `pdf.rs` prend un chemin de modèle — vérifié en lisant le
  texte du module, avec une exemption nommée, le bulletin d'adhésion,
  qui n'est pas un Typst mais le PDF de l'Assurance Maladie dont on ne
  remplit que les champs.

## [0.171.0] - 2026-09-09

### Added
- **Les carnets de suivi : les feuilles que le patient emporte.** Une
  officine en donne tous les jours — trois jours d'automesure avant une
  consultation, une semaine de glycémies, le poids d'un insuffisant
  cardiaque — et elle les photocopie, quand elle en a, sur un modèle que
  personne n'a relu depuis dix ans ; le reste du temps elle dit « notez-le
  sur un papier », ce qui revient à ne rien donner.

  **Ce qui manque n'est pas la grille, c'est le protocole.** Une tension
  prise après le café, debout, sur le bras qui traîne ne veut rien dire ;
  une glycémie notée le soir de mémoire non plus. Chaque feuille porte
  donc quatre choses, et la grille n'est que la quatrième : comment
  mesurer, ce qu'on vise, ce qui s'appelle sans attendre, et où écrire.

  Six feuilles, et pas une de plus au hasard — chacune correspond à
  quelque chose que l'officine suit déjà : **automesure tensionnelle**
  (la règle des trois, avec les cases de moyenne, puisque ce que le
  médecin lit n'est aucune des dix-huit mesures), **glycémie
  capillaire**, **poids** dans l'insuffisance cardiaque, **débit
  expiratoire de pointe**, **INR** sous AVK, et **douleur** — celle-ci
  avec le compte des interdoses, qui est le nombre par lequel le registre
  des stupéfiants dit qu'il faut revoir le traitement de fond.

  Deux règles tenues par des tests. **Aucun chiffre inventé** : là où
  l'objectif est individuel — la glycémie, la zone d'INR, la meilleure
  valeur personnelle de souffle —, la feuille dit qu'il est individuel et
  laisse la ligne à remplir, plutôt qu'imprimer une valeur que le patient
  prendrait pour la sienne. Et **rien qui remplace le prescripteur** :
  aucune feuille ne dit d'adapter une dose, toutes disent à qui
  téléphoner et quand.

  L'écran est fait pour être lu à voix haute au comptoir avant
  d'imprimer. La feuille porte le nom du dossier ouvert quand il y en a
  un, et une ligne à remplir sinon — une feuille vierge se donne aussi
  bien, et c'est le cas courant.

## [0.170.0] - 2026-09-09

### Added
- **Le catalogue du registre porte le marché français, génériques
  compris.** Cent cinquante-huit présentations au lieu de cent six : les
  dosages qui manquaient aux princeps — Oxycontin 15, 30, 60 et 120 mg,
  les ampoules de chlorhydrate de morphine à 1 et 40 mg/mL —, les
  molécules sous lesquelles les génériques se délivrent — morphine
  sulfate LP et LI, oxycodone LP et LI, fentanyl transdermique,
  méthylphénidate LP —, et une famille entière qui manquait, la
  **lisdexamfétamine** (Elvanse), qu'une officine délivre depuis que le
  méthylphénidate a un recours.

- **Et le laboratoire s'écrit sur la ligne.** Le méthylphénidate LP
  36 mg d'un laboratoire et celui d'un autre sont deux boîtes, avec deux
  codes, sur la même étagère : un registre qui les confond compte juste
  et ne permet plus d'aller chercher la bonne — ce qui est tout ce qu'un
  comptage physique demande. Un champ en tête du catalogue, et le
  libellé suivi devient « Méthylphénidate LP 36 mg (EG) ». **Rien n'est
  livré de ce qui n'est pas vérifiable** : l'application ne dit jamais
  quel laboratoire vend quel dosage, elle propose d'écrire ce qui est
  sur la boîte.

- **Onze monographies de stupéfiants qui manquaient.** On suivait le
  produit au registre, on cliquait sur sa fiche, et il n'y en avait pas.
  Sevredol, Moscontin, Abstral, Effentora, Instanyl, Pecfent, Orobupré,
  Concerta, Quasym, Medikinet, Elvanse — chacune avec ses posologies de
  comptoir. Ce sont onze fiches et non quatre parce que chacune porte
  une particularité que sa voisine n'a pas : l'enveloppe du Concerta
  qu'on retrouve dans les selles, le repas obligatoire du Medikinet LM,
  le comprimé de Moscontin qui ne s'écrase jamais, la titration d'un
  fentanyl rapide qui ne se déduit **jamais** de la dose de fond, et le
  fait que deux fentanyls transmuqueux ne sont pas interchangeables
  microgramme pour microgramme.

- **Un code scanné suit le produit qu'on désigne.** Une boîte arrive, on
  la scanne, le code n'est attaché à rien : il fallait ouvrir le
  catalogue, trouver le produit, le suivre, puis **rescanner** la boîte
  pour lui attacher le code. Le code attend déjà ; il s'attache
  maintenant à ce qu'on choisit au catalogue. C'est toujours un humain
  qui désigne : rien n'est deviné d'un code, et l'application n'embarque
  toujours aucune table CIP.

- **Un chronomètre, dans les tests.** Il mesure ce que coûtent
  l'ouverture d'une session et chacune des lectures qu'une vue refait ;
  ses lignes ne s'affichent qu'avec `--nocapture`, ce que le harnais de
  Rust fait de tout ce qu'un test écrit. Ce qu'il faut y lire n'est pas
  un chiffre absolu — il dépend de la machine — mais le rapport entre
  les lignes.

  Ce qu'il **affirme**, en revanche, ce sont les comptes : chaque passe
  de contenu livré doit remplir ce qu'elle prétend remplir. C'est
  exactement ce qu'une réécriture pour la vitesse casse sans que rien ne
  le dise — et c'est aussi le premier test à parcourir les huit passes
  du premier lancement, qu'aucun autre ne couvrait.

### Changed
- **Le premier lancement passe de quatre secondes à moins d'une.**
  Mesuré, pas deviné, et pas là où on croyait. `seed_conduite`
  reconnaissait ses cent dix règles par quatre `LIKE` sur la classe, les
  étiquettes, la DCI et le nom : aucun index ne les sert, chaque règle
  balayait donc les huit cent soixante-deux fiches, et la passe coûtait
  **3,7 secondes** à elle seule. Une lecture, le rapprochement en Rust,
  une écriture par fiche retenue sur sa clé primaire : **178 ms**, pour
  exactement les mêmes 830 fiches remplies.

  Et `fill_starter_details`, douze mille cinq cents cases, est passé de
  **789 à 404 ms** — en préparant les dix-huit requêtes une fois au lieu
  d'en analyser une par case. La première tentative avait tourné colonne
  par colonne plutôt que fiche par fiche : elle a rendu la passe *plus
  lente* (674 ms), parce que chaque requête traversait alors la table
  entière. Les trois formes sont mesurées et le choix est écrit à côté
  du code, faute de quoi la prochaine intuition referait le même détour.

- **Les statistiques ne se recopient plus par image.** La vue clonait
  ses quatre séries à chaque dessin, soit quelques milliers
  d'allocations par seconde pour un écran qui ne change pas. Elle
  emprunte — et la signature l'y oblige désormais.

### Fixed
- **Le registre se lit à l'ouverture de la session.** Il n'était chargé
  que par son onglet : tout ce qui le lit ailleurs voyait du vide. Les
  statistiques annonçaient « rien n'est sorti sur quatre-vingt-dix
  jours » sur un registre qui portait des lignes, la console rendait un
  `registre()` vide, et le compte de produits suivis était zéro. Un
  chiffre à zéro qui a l'air d'une réponse est pire qu'un écran qui dit
  qu'il ne sait pas.

- **La bande du haut des stupéfiants n'était pas mesurée.** Sa hauteur
  était écrite « une rangée plus une ligne » : à 1024x700 en texte 1,6
  les contrôles passent à deux rangées, et « Douchette… » se peignait
  par-dessus le panneau d'en dessous — un `Painter` peint où on lui dit,
  rien ne le clippe. Elle se mesure, se plafonne en part du volet et
  défile dans sa part ; le message qu'elle porte est mesuré comme il est
  dessiné, c'est-à-dire enveloppé.

- **Un message d'échec qui ne nommait pas le coupable.** Le garde-fou
  qui refuse deux orthographes d'une même spécialité disait « 862 au
  lieu de 863 » : exact, et sans usage sur une liste de huit cent
  soixante-trois lignes. Il nomme la paire — et il a servi tout de
  suite : « Actiskenan » ajouté à côté d'« Actiskénan », qui était déjà
  là.

## [0.169.0] - 2026-09-09

### Added
- **Ce qui sort du registre, tous produits confondus.** Le registre le
  savait par produit — c'est la courbe de chaque fiche — et personne ne
  l'avait jamais lu en travers, alors que « qu'est-ce qui part le plus »
  est la question qu'on se pose en commandant. Une barre par produit sur
  les quatre-vingt-dix derniers jours, délivrances seules : une ampoule
  cassée est du stock qui part, ce n'est pas de la consommation, et une
  ligne annulée n'a pas eu lieu.

- **Et les actes par mois, en nombre.** La recette par mois est partie
  avec les recettes, sur un écran qu'on n'ouvre pas devant tout le
  monde ; le rythme, lui, se raconte.

### Fixed
- **Une statistique lisait le cache d'une autre vue.** « Ce qui sort »
  cherchait les libellés des produits dans `stup_labels`, que seul
  l'onglet du registre remplit : sur une session qui ne l'avait pas
  ouvert, le panneau était vide — c'est-à-dire un zéro qui a l'air d'une
  réponse. Les libellés viennent de la base, dans la requête qui compte
  déjà les produits suivis : une requête, deux réponses.

## [0.168.0] - 2026-09-09

### Added
- **La feuille collée se colore : chaque ligne dit si la base l'a
  reconnue.** La réponse était en bas de l'écran, en tableau, et il
  fallait la lire pour découvrir qu'une ligne sur six n'avait servi à
  rien. Elle est maintenant *sur* la ligne, pendant qu'on tape : ce que
  la base n'a pas su rapprocher est en encre d'alerte, le reste dans
  l'encre ordinaire.

  Elle ne coûte rien par image : les lignes non rapprochées sont ce que
  la conciliation a déjà répondu, mémoïsé contre le texte collé. Refaire
  la passe floue sur huit cent cinquante fiches dans un `layouter`, ce
  serait la refaire soixante fois par seconde.

## [0.167.0] - 2026-09-09

### Added
- **Une ordonnance entière se colle, se relit, et se reprend au
  dossier.** La conciliation savait lire une liste collée et dire ce qui
  change ; ce qu'elle en faisait s'arrêtait aux posologies. « Reprendre
  au dossier » met aussi **les traitements que la feuille apporte**,
  avec leur posologie — c'est ce qui manquait pour que ce volet soit la
  saisie d'une ordonnance entière, au lieu qu'on retape dans la bande,
  ligne par ligne, une liste qu'on vient de coller.

  Ce qu'il ne fait pas : arrêter ce que la feuille ne porte plus.
  Arrêter un traitement est une décision, pas une recopie, et elle se
  prend puce par puce. Et rien n'est repris d'une ligne que la base n'a
  pas su rapprocher : une fiche inventée à partir d'un mot mal lu est
  une fiche de plus dans un référentiel de huit cent cinquante, et
  personne ne la retrouvera pour la corriger.

  **Un bouton et non deux**, et c'est une mesure et pas un goût : le
  quatrième bouton faisait passer la rangée à deux lignes, et à 1024x700
  en texte 1,25 la seconde ligne coûtait la seule ligne que la table des
  divergences avait. Mesurer ce qu'un en-tête coûte avant de régler ce
  que les volets reçoivent — la règle de la maison, et cet onglet est
  celui où elle avait été apprise.

## [0.166.0] - 2026-09-09

### Added
- **La console : interroger sa propre base en quelques lignes.** Une
  officine se pose des questions que personne n'a prévues — « quels
  dossiers prennent une statine sans bilan lipidique depuis dix-huit
  mois », « combien de fiches de telle classe n'ont pas d'interaction
  écrite ». Chacune est un écran de plus si elle doit être programmée, et
  trois lignes si la base se laisse interroger. Un onglet « Console » :
  les scripts enregistrés à gauche, l'éditeur au milieu, la sortie à
  droite, et quatre exemples livrés pour montrer ce qu'on peut demander.

  **Ce qui rend la chose possible, et ce sont deux règles :** le langage
  n'a *aucune* entrée-sortie — ni fichier, ni réseau, ni processus —, si
  bien que le seul chemin d'un script vers le monde est le volet où il
  écrit ; et il ne peut **rien écrire** dans la base, puisqu'il lit un
  instantané pris avant l'exécution. Un script qui pourrait écrire au
  registre des stupéfiants serait un trou dans le seul endroit de
  l'application qui n'en a pas. Les deux sont tenues par des tests, dont
  un qui vérifie qu'aucune fonction d'ouverture de fichier ni d'écriture
  n'existe.

  Et il s'arrête : `while true {}` fait trois caractères, et une
  application de comptoir ne se rouvre pas d'un clic. La borne est en
  **opérations** et non en secondes — une horloge ferait passer le même
  script sur un poste et échouer sur l'autre, ce qui est la pire façon
  d'échouer.

  Les scripts s'enregistrent **à côté de la base**, comme les notes
  d'équipe : une question qu'une officine s'est écrite vaut pour
  l'officine et pas pour le poste. En clair, parce que ce sont des
  questions et non des données de santé — on peut les ouvrir dans un
  éditeur et les relire dans dix ans sans cette application.

## [0.165.0] - 2026-09-09

### Added
- **Le compagnon (F9) : la fenêtre réduite à une barre, au-dessus de
  tout.** À l'officine, le logiciel qui délivre est devant et BPM-Caddy
  est derrière : on tape un nom dans l'un et l'on voudrait la
  monographie de l'autre, sans changer de fenêtre. Une touche réduit la
  fenêtre à quatre cent soixante pixels sur trois cents, la pose
  au-dessus des autres, et n'y laisse qu'un champ, une phrase et quatre
  gestes — la fiche, un acte, une pièce, une délivrance —, chacun
  rendant la fenêtre et ouvrant l'écran qui va avec. La même touche la
  rend.

  Ce n'est **pas** une seconde fenêtre : c'est celle-ci, rétrécie. Une
  vue secondaire aurait demandé son propre contexte et un partage
  d'état pour un résultat que l'officine ne distinguerait pas.

  Ce qu'il ne fait **pas**, et délibérément : écouter le clavier des
  *autres* applications. Le compagnon lit ce qu'on tape dans son champ,
  et son champ reprend le foyer à chaque image — une douchette y tape
  comme un clavier. Un crochet à l'échelle du système capterait aussi
  les mots de passe et les noms de patients tapés ailleurs, dans une
  application qui tient par ailleurs des données de santé chiffrées ; le
  jeu n'en vaut pas la chandelle. Le geste qui manque est donc un clic
  dans le champ, une fois.

## [0.164.0] - 2026-09-09

### Added
- **Ce que la base sait d'elle-même.** Tout ce que l'application compte,
  elle le comptait pour une question à la fois : combien de fiches
  parlent du foie, combien d'actes ont été facturés ce mois-ci, quel
  produit ne bouge plus. Aucune vue ne disait ce que l'ensemble vaut.
  « Statistiques » répond à quatre questions qu'on se pose deux fois par
  an et qui décident du semestre suivant.

  **La base** : huit cent cinquante et une fiches, trois cent
  quatre-vingt-trois classes une fois les libellés repliés — et,
  champ par champ, combien de fiches le portent. C'est la liste de ce
  qu'il reste à écrire, et elle n'existait nulle part : on le découvrait
  fiche par fiche.

  **Les dossiers** : combien atteignent les cinq traitements du bilan
  partagé de médication, combien portent une biologie, un vaccin, une
  location ou une pièce, et la distribution des traitements par dossier.

  **L'activité** : qui fait les entretiens — les initiales sont sur
  chaque ligne depuis toujours et n'atteignaient aucun total —, et la
  durée moyenne par thématique, calculée sur les actes qui portent une
  durée : un acte dont personne n'a noté le temps n'est pas un acte de
  zéro minute, et le compter ferait une moyenne qui ne dit plus rien.

## [0.163.0] - 2026-09-09

### Added
- **Un traitement porte son dosage.** « Amlor » sur une ordonnance ne dit
  pas si c'est le 5 ou le 10, et c'est la première chose qu'on vérifie au
  comptoir : la puce le portait sans le dire, et il fallait ouvrir la
  posologie pour l'apprendre — quand quelqu'un l'y avait écrit. Une
  colonne de plus sur la ligne du dossier, séparée de la posologie parce
  que ce n'est pas la même question : la posologie dit quand et combien,
  le dosage dit **de quoi**. Il se lit sur la puce, sur le plan de prise
  qui part à la maison et sur le bilan partagé de médication.

  Il se choisit d'un clic ou **se tape**, et le champ libre est le point :
  la digoxine n'existe qu'en 0,125 mg quadrisécable, et le quart qu'un
  patient prend réellement est dans sa boîte à pilules même si aucun
  laboratoire ne le vend. Une liste fermée dirait qu'il n'existe pas.

- **Et ce que l'application propose, elle l'a appris.** Aucune table des
  présentations du marché n'est embarquée et il n'y en aura pas : cinq
  cents dosages écrits en dur seraient cinq cents propositions justes le
  jour où on les écrit et fausses le jour où un titulaire reconditionne.
  Les dosages offerts d'un clic sont les formes que l'officine a portées
  sur la fiche, **et ceux sous lesquels ce médicament est déjà inscrit
  dans la base** : trois dossiers portent « 5 mg », le quatrième se
  saisit d'un clic. C'est la règle des codes-barres du registre et des
  derniers prescripteurs rencontrés, appliquée ici.

  Ce qui n'a **pas** été fait, et pourquoi : une fiche par présentation —
  « Perindopril 4 mg BGR » — multiplierait les huit cent cinquante
  monographies par le nombre de laboratoires génériqueurs, et il faudrait
  tenir vingt copies de chacune. Le laboratoire n'est pas un fait
  clinique ; le dosage en est un. Une fiche par produit, donc, le dosage
  sur la ligne du dossier, et le laboratoire dans la posologie si
  l'officine y tient.

### Fixed
- **La rangée des traitements passait sous le pli.** À 1024x700 en texte
  1,6, la bande du dossier est plafonnée à 45 % du volet, et l'en-tête
  plus les deux rangées de boutons — « Supprimer… », « Modifier »,
  « Bilan… », « Plan de prise… » — la remplissaient entièrement : on
  ouvrait une fiche et on ne voyait pas ce que la personne prend. Ce
  qu'on lit à chaque fois passe devant ce qu'on fait de temps en temps :
  les boutons descendent sous les traitements quand la rangée du nom ne
  les porte plus. Rien ne bouge sur une fenêtre qui les porte.

- **Et le plafond coupait la deuxième rangée de puces par le milieu.** Il
  compte ce qui est dessiné au-dessus de la première rangée qu'on puisse
  couper, et il oubliait les vingt pixels d'air entre la ligne de
  contexte et les traitements. Le dessin et le plafond lisent la même
  constante maintenant, plutôt que deux nombres qu'il faudrait garder
  d'accord.

- **La bande se mesurait sur un libellé que le dessin n'emploie plus.**
  La largeur d'une puce était calculée sur le seul nom du médicament
  quand le dessin y ajoute le dosage : « Aricept ½ de 10 mg » demandait
  une rangée de plus que la bande n'en réservait. Les deux composent le
  libellé par la même fonction.

## [0.162.0] - 2026-09-09

### Added
- **Le registre tient deux comptes, et non un.** Ce qu'un patient
  rapporte entre bien à l'officine : cela se compte, cela s'enferme au
  même coffre, cela se justifie devant le même contrôle — et cela ne se
  délivrera plus à personne. Le remettre au solde ferait annoncer
  quarante gélules disponibles là où il y en a vingt-six et un sac
  scellé ; le passer en perte l'effacerait, alors que l'officine en
  répond jusqu'au procès-verbal. Deux natures nouvelles, donc — **retour
  patient** et **destruction** —, et un second solde qui se calcule dans
  la **même passe** que le premier : deux lectures du même registre
  finiraient par ne plus dire la même chose du même jour.

  Un retour porte le dossier — d'où sortent quarante gélules de morphine
  est la seule question qui compte — et ne prend **pas** de numéro
  d'ordonnancier, qui est celui d'une délivrance. Une destruction ne
  s'écrit pas sans citer ce qui la rend vérifiable : procès-verbal,
  confrère présent, collecteur. C'est le seul compte du registre dont
  personne d'autre ne tient la contrepartie, et la base le refuse à
  l'écriture, pas seulement le formulaire.

- **Un onglet « À détruire »**, trié du plus ancien au plus récent et
  par rien d'autre. Ce qu'on vient y chercher n'est pas la plus grosse
  quantité, c'est la plus vieille : un stock à détruire ne réclame rien
  tout seul — aucune échéance ne tombe, aucun patient ne rappelle,
  aucun grossiste ne le reprend — et c'est très exactement pour cela
  qu'un sac dort trois ans au fond d'un coffre. La date affichée est
  celle où le coffre a **quitté zéro** pour la dernière fois, et non
  celle du plus ancien retour du registre : entre les deux il y a
  peut-être eu une destruction, et dater d'un sac déjà parti ferait dire
  « en attente depuis onze mois » d'un retour d'avant-hier, c'est-à-dire
  un signal que personne ne croirait deux fois.

  Avec sa feuille : le **procès-verbal de destruction**, qui sort avant
  la destruction, se coche à mesure et porte les deux signatures au bas.
  La pièce que la ligne du registre cite n'existait nulle part.

- **On ne reçoit pas des unités, on reçoit des boîtes.** Le grossiste
  livre trois boîtes d'Actiskenan, qui en contiennent quatorze, quand on
  en délivre seize : la multiplication était la dernière chose qui se
  faisait de tête avant d'écrire dans un registre inaltérable, où une
  erreur ne se défait que par une contre-passation motivée. La rangée du
  comptage — « trois de quatorze, plus cinq » — sert désormais aussi au
  déballage, et le conditionnement se retient sur la fiche du produit.

  **Il n'est pas livré avec le catalogue** : cent six conditionnements
  écrits en dur sont cent six multiplications appliquées à l'aveugle,
  justes le jour où on les écrit et fausses le jour où le titulaire
  reconditionne. L'officine le dit une fois, en regardant la boîte —
  comme pour un code-barres, et pour la même raison.

- **Les recettes ont leur écran, et il n'a pas de porte.** Le chiffre
  d'affaires facturé, celui qui attend, le taux horaire et la courbe des
  douze mois étaient en tête du tableau de bord — c'est-à-dire sur
  l'écran qu'on ouvre devant un patient, un préparateur, un stagiaire,
  un visiteur. Le mode discret les masquait derrière un bouton qu'il
  suffit de cliquer : cela protège d'un regard et de rien d'autre.

  Ils sont dans « Recettes », qui ne figure dans aucun dock, dans aucune
  barre d'onglets tant qu'on ne l'a pas ouverte, et **pas même dans la
  liste que la boîte de saut propose à vide** : on y va en tapant son
  nom. Un écran « pour soi » offert en cinquième ligne d'un menu n'en
  est pas un. Le récapitulatif de facturation et l'export des actes
  l'ont suivi ; la liste d'appel reste au tableau de bord, parce qu'un
  résultat de biologie à commenter est une affaire clinique et pas une
  recette.

  Ce que le tableau de bord garde se lit à voix haute : les actes
  facturés, ceux qui restent à facturer, le temps passé, les dossiers.

- **Et l'écran des recettes dit quatre choses que personne ne comptait.**
  La **moyenne par acte** — pas le tarif d'un acte, mais ce que vaut une
  heure de travail de plus, qui est la question qu'on se pose en
  décidant d'en faire ou non. La **recette par thématique**, qui n'est
  pas le compte d'actes par thème : un bilan partagé de médication pèse
  plusieurs TROD, et le mélange qui remplit le mieux l'agenda n'est pas
  celui qui rapporte le plus. Le **CA annuel**, l'horizon d'une
  convention là où douze mois ne disent que le rythme. Et **où l'argent
  attend** : un acte fait et non facturé est de l'argent gagné qui dort,
  et c'est la seule barre de cet écran sur laquelle on puisse agir
  aujourd'hui.

- **Le fil du dossier.** Un dossier se lisait par onglets — les actes,
  les vaccins, la biologie, les locations, les pièces —, et chacun
  répondait bien à sa question. Aucun ne répondait à celle qu'on pose en
  ouvrant la fiche de quelqu'un qu'on n'a pas vu depuis six mois :
  **qu'est-ce qui s'est passé, et quand ?** La dernière délivrance
  derrière un onglet, le dernier vaccin derrière un autre, le dernier
  résultat derrière un troisième — chacun est une ligne, et il fallait
  trois clics pour lire trois lignes.

  Un onglet « Fil » les fond en une seule suite datée : la bande des
  douze derniers mois, le résumé « dernier de chaque nature », puis les
  lignes, du plus récent au plus ancien, chacune avec le repère de
  couleur de sa source. Trois règles le tiennent : **une ligne sans jour
  n'y entre pas** — la placer « quelque part » la daterait d'un jour qui
  n'est pas le sien —, **l'ordre est total**, sans quoi deux lignes du
  même jour changent de place d'une image à l'autre, et **un rendez-vous
  de la semaine prochaine n'est pas le dernier acte** : il est en tête,
  sous un intertitre qui le dit, et le résumé ne le compte pas.

  Le registre des stupéfiants se lit **aussi par dossier** pour
  l'occasion : il portait le numéro de dossier sur chaque délivrance
  depuis toujours, et « qu'a-t-on délivré à cette personne » demandait
  de parcourir tous les produits.

### Fixed
- **Le report du comptage en boîtes n'écrivait rien.** Le bouton
  « = 47 » posait le total dans une variable déjà lue quinze rangées
  plus haut : il s'affichait, il se cliquait, et le champ ne bougeait
  pas. Le défaut était invisible parce que l'affectation vivait dans une
  fermeture, où le compilateur ne peut pas la voir morte ; sortie de la
  fermeture pour être partagée avec la réception, elle a été signalée à
  la première compilation. La quantité s'écrit maintenant **après** les
  champs propres à chaque nature, et à un seul endroit.

- **Deux mesures d'une même largeur, dans la bande du produit.** La
  mesure comptait 70 et 90 pixels là où le dessin appelait
  `field_width(…)` : la règle de la maison, oubliée à l'endroit précis
  où elle décide si la bande tient sur une rangée ou deux. Les largeurs
  sont calculées une fois et le dessin les reçoit.

## [0.161.1] - 2026-09-08

### Fixed
- **Un plafond écrit pour une peau sombre n'est pas un plafond sur une
  peau claire** — la leçon de la veille, une fois de plus, et trouvée en
  comparant deux captures de la même carte. Les tons supplémentaires que
  la carte vaccinale tire de la rampe (dix-sept régions, huit couleurs)
  n'étaient bornés que par le plafond d'éblouissement : sur un fond
  clair, le ton pâle pouvait donc monter à six centièmes du fond, et
  « Afrique de l'Est » devenait une tuile qu'il fallait chercher. Ils
  tiennent maintenant dans **la bande** où `data_ramp` pose une rampe.

- **Et les trois tons se choisissent ensemble.** Calculés un par un ils
  se rejoignaient : sur « Nuit », le violet est assez haut pour que son
  ton sombre touche le plancher, fasse demi-tour et atterrisse à quatre
  centièmes de son propre ton clair — deux régions, une seule pastille,
  ce que la vue par groupes ne peut pas se permettre. La bande est
  coupée en trois, la case que la couleur occupe déjà est retirée, et
  les deux tons prennent les deux qui restent : une demi-bande d'écart
  par construction. `data_shade(c, up)` devient `data_tones(c)`, et un
  test tient les trois tons de chaque couleur de la rampe, sur les huit
  peaux.

- **Une teinte remise à l'échelle sortait de la bande de trois
  centièmes.** Quand un canal sature, le facteur ne porte pas toute la
  distance et le reste se fait vers le blanc ; ce reste se parcourait
  par vingtièmes, en s'arrêtant au premier pas *au-delà* de la cible.
  Il se bissecte, et la cible est atteinte au millième.

## [0.161.0] - 2026-09-08

### Added
- **Deux peaux sombres, « Nuit » et « Ambre ».** Les six palettes
  livrées jusqu'ici sont six gris clairs : l'ardoise d'une station de
  travail, le taupe de CDE, le vert-de-gris de HP. Une officine de garde
  à trois heures du matin n'est éclairée que par son écran, et la
  question n'est pas de goût. « Nuit » est cette ardoise-là en sombre,
  « Ambre » le phosphore d'un terminal — et l'une comme l'autre est
  **une palette et rien d'autre** : pas une branche du dessin ne
  distingue le jour de la nuit. La forme ne bouge jamais, c'est ce qui
  fait Motif.

  La monographie s'y lit sur une feuille sombre à l'encre pâle. Une page
  blanche au milieu d'un écran choisi pour ne plus brûler serait la
  seule chose qu'on ne pourrait pas regarder.

- **Un vocabulaire pour les couleurs qui ne viennent pas du thème.**
  `on_fill(fond)` donne l'encre qu'une pastille porte, décidée sur le
  fond réellement peint. `data_ramp` pose une rampe catégorielle dans
  la bande que la peau lui laisse, **d'un seul mouvement pour toute la
  rampe** : dix couleurs d'actes se lisent les unes contre les autres
  dans une même légende, et une pastille seule est une rampe d'une
  couleur — il n'y a volontairement pas de seconde fonction pour elle.
  `data_shade` en tire un second et un troisième ton quand un ensemble
  a plus de membres que la rampe n'a de couleurs.

- **Le choix de la peau se fait en la regardant.** Chacun des huit
  boutons d'Options › Interface est dessiné *dans la peau qu'il
  nomme* — son fond, son encre, son biseau — avec quatre de ses
  couleurs à côté : le fond, la sélection, l'alerte et la feuille sur
  laquelle se lit une monographie. Un `selectable_label` écrivait
  « Nuit » dans la palette en cours, c'est-à-dire la seule qu'il ne
  s'agit pas de montrer. La bande des huit couleurs reste dessous, et
  rien de tout cela ne demande de redémarrer.

  `BPM_CADDY_START_VIEW=peaux` ouvre cette page-là : c'est la seule
  chose de ce lot qu'aucun test ne peut regarder à votre place.

- **Le lanceur ouvre dans la peau choisie.** C'est la première fenêtre
  de la soirée, et elle s'ouvrait dans le bleu-gris de `mwm` quoi que
  l'officine ait réglé : qui a choisi « Nuit » recevait un rectangle
  allumé en pleine figure avant que la fenêtre sombre n'arrive. Le
  lanceur lit la clé `[ui] theme` du `config.toml` à la main — il ne
  dépend pas du crate de l'application, et il n'a pas besoin de toute
  une configuration pour une clé. Ce qu'il ne sait pas lire le laisse
  sur la palette classique, comme une clé inconnue.

### Changed
- **Toute règle sur la couleur est une distance, plus jamais une
  direction.** « L'encre est sombre » était vrai de six palettes et faux
  de deux ; « l'encre est loin du papier » est ce qu'on voulait dire à
  chaque fois. `every_palette_can_be_read` est réécrit ainsi, et les
  deux règles restées directionnelles — le biseau et la teinte de
  survol — le sont dans le *dessin* : un widget Motif est éclairé du
  coin supérieur gauche à toute heure.

- **Une rampe se pose dans la bande que la peau lui laisse.** Multiplier
  les trois canaux garde les rapports, donc la teinte, et *écarte* les
  membres en les remontant ; mais la rampe des actes, multipliée pour
  dégager son membre le plus sombre, sortait son vert à trois quarts de
  blanc — une lampe sur un écran choisi pour n'en plus être une. Trois
  cas, essayés dans l'ordre : multiplier tant que ça tient sous le
  plafond, translater quand multiplier éblouirait (les distances sont
  alors gardées à l'unité près), comprimer seulement si la bande est
  plus étroite que la rampe. Le mélange vers le blanc, lui, est refusé
  partout : il emmène toutes les teintes au même point, et deux séries
  du graphique se retrouvaient à vingt et un l'une de l'autre là où la
  règle de la rampe est trente-cinq.

- **`Color32::WHITE` écrit à la main a disparu des pastilles.** Trente
  endroits l'écrivaient, et ils avaient raison tant que tous les fonds
  étaient sombres — ce qui a cessé d'être vrai le jour où une peau a eu
  un fond clair. C'est `on_fill` qui choisit, à partir du fond peint :
  le « dépassé » rouge d'une location porte du noir sur « Nuit » et du
  blanc sur « Motif », sans que le site d'appel ait à le savoir.

- **L'emphase s'éloigne du fond au lieu de foncer.** La famille livrée
  n'a pas de gras, donc `*gras*` se porte par une encre plus forte —
  écrite « plus sombre », ce qui est une emphase de jour et un murmure
  de nuit. Même chose pour la bande zébrée d'un tableau et pour la
  grille d'un graphique : mêlées vers le biseau *opposé* au creux, car
  un creux est déjà la surface la plus sombre d'une peau de nuit.

### Fixed
- **Une couleur de chrome écrite en hexadécimal ne suit aucun thème**, et
  il en restait. La bande zébrée des tables de conversion était un gris
  bleuté fixe — juste sur le bleu de mwm, une tache sur le vert de HP
  VUE, un trou sur tout fond sombre. L'échelle ordinale de la carte
  vaccinale était écrite **deux fois**, une fois pour les tuiles et une
  fois pour la légende sous elles, libres de diverger : ce sont
  pourtant les deux choses qu'on lit l'une par l'autre. Et un opérateur
  sans initiales, comme un médicament sans statut connu, se dessinait
  dans l'ombre du biseau, invisible sur une peau sombre.
  `no_colour_is_written_in_hex_outside_a_named_ramp` lit le texte de
  `app.rs` et refuse le prochain littéral, comme les deux tests qui
  refusent une taille et une bascule en pixels : une teinte
  catégorielle vit dans une rampe `const` nommée, une seule fois, et
  rejoint l'écran par `data_ramp`.

- **Le modèle de `config.toml` nommait six palettes sur huit.** C'est
  le genre de liste qui vieillit sans rien casser : le fichier reste
  valide, l'application démarre, et la seule chose qui se passe est
  qu'une officine ne saura jamais que « nuit » existe — or ce fichier
  écrit au premier lancement est la documentation que tout le monde
  lit. Un test le tient maintenant contre `motif::THEMES`.

- **Une rangée qui enveloppe dans une cellule de `Grid` n'annonce que
  sa première ligne.** Huit boutons de palette passent à deux rangées,
  et la seconde se dessinait par-dessus l'« Aperçu » d'en dessous.
  Aucun test ne l'aurait dit ; une capture d'écran l'a dite tout de
  suite. Le choix de la peau vit maintenant dans le flot vertical de la
  boîte de dialogue, où une bande grandit comme elle doit.

- **La carte des groupes de pays sortait douze pays en blanc pur.** Dix-
  sept régions pour huit couleurs de rampe : le second tour était un
  `gamma_multiply(1.6)`, non borné, et appliqué à une couleur déjà
  remontée sur une peau de nuit il saturait à blanc. `data_shade` fait
  ce pas à l'intérieur de la même bande, et fait demi-tour quand il
  rencontre le mur plutôt que de rendre la teinte dont il part.

## [0.160.0] - 2026-09-08

### Added
- **Un comptage se saisit en boîtes et en vrac.** Un stupéfiant se
  compte devant le coffre : on aligne les boîtes pleines et l'entamée,
  et on dit « trois de quatorze, plus cinq ». Cette multiplication se
  faisait de tête juste avant d'écrire **un seul nombre** dans une
  pièce inaltérable, où une erreur ne se défait que par une
  contre-passation motivée. Trois champs — boîtes, par boîte, vrac — et
  un bouton qui reporte le total. Le champ du comptage reste maître :
  le total s'y reporte d'un clic et ne s'y écrit jamais tout seul,
  comme la glissière au-dessus. `counted_total` est pure et tenue par
  un test, y compris sur ce qui n'est pas un comptage — un champ à
  moitié tapé ne rend pas un total qui a l'air d'un résultat.

- **La couleur de l'acte devant son nom, dans le tableau du dossier.**
  C'était déjà sa marque partout ailleurs — la pastille de l'agenda, le
  carré du choix rapide, la barre du filtre de la semaine — et le
  tableau des entretiens était le seul endroit où elle manquait : sur
  un dossier qui porte un BPM, un TROD et une vaccination, les rangées
  ne se distinguaient qu'en lisant.

### Changed
- **Un écart d'inventaire se motive, et c'est la base qui l'exige.**
  `Discrepancy::matters` le disait en prose depuis toujours — tout écart
  non nul mérite une explication, et le code de la santé publique ne
  connaît pas de seuil de tolérance — et rien ne l'exigeait. Un comptage
  qui ne tombe pas juste et qui part sans motif est une ligne dont
  personne ne saura jamais si elle vient d'un vol, d'une casse ou d'une
  ligne oubliée. Refusé à l'écriture et non dans le formulaire : une
  règle qui ne tient que dans une vue ne tient pas — même place et même
  raison que le motif obligatoire de l'annulation. L'invite du champ le
  dit dès que l'écart s'affiche, donc **avant** de presser « Inscrire ».

### Fixed
- **La bande d'identité du dossier réservait quatre-vingt-dix pixels de
  gris vide** au-dessus des onglets, sur la vue la plus regardée de
  l'application : huit pixels de marge par rangée en plus de la
  gouttière, et un en-tête de quatre-vingt-seize là où le nom et la
  ligne de contexte en font une soixantaine. Elle emploie désormais le
  modèle que `a_wrapped_band_is_as_tall_as_its_model_says` tient. Le
  tableau des entretiens et le journal y gagnent chacun une rangée
  entière.

  Au passage, une leçon sur les deux emplois du mot « en-tête » : celui
  de la *hauteur* doit être juste — se tromper en plus y remet du gris —
  tandis que celui du *plafond* doit se surestimer, puisque se tromper
  en moins coupe une rangée par le milieu. À l'échelle 1,25, où la ligne
  de contexte enveloppe sur deux lignes, la rangée des traitements s'est
  retrouvée tranchée sous ses puces avant que les deux soient séparés.

## [0.159.2] - 2026-09-08

### Fixed
- **`motif::section` prend deux lignes quand chaque mot y tient.**
  Élidés, « Amérique du Nord » et « Amérique centrale » se lisaient
  tous deux « Amérique… » : l'intitulé ne distinguait plus rien, ce qui
  est exactement le défaut que `list_row` avait corrigé de son côté.

  Trois tentatives avaient échoué avant celle-ci, toutes en cherchant
  la bonne largeur — `available_width`, `clip_rect`, une borne
  calculée : la borne « ne mordait pas » et l'intitulé repartait se
  faire trancher par le volet. **La largeur était bonne depuis le
  début.** Ce qui la jetait est `Label::new(LayoutJob)`, qui écrase le
  `max_width` du job par la largeur d'enveloppement de l'`ui` et ne
  garde que le nombre de lignes. `motif::panel` posait déjà lui-même la
  galée de son titre pour cette raison exacte, sans que ce soit écrit
  nulle part ; `section` fait pareil, et la règle est notée : quand une
  borne ne mord pas, soupçonner le chemin avant la mesure.

  Le filet reste la décoration et c'est lui qui cède : sur un volet
  large il est entier, sur un volet étroit il se réduit pendant que
  l'intitulé, lui, se lit.

## [0.159.1] - 2026-09-08

### Added
- **Deux tests qui confrontent une mesure à son dessin**, sur le modèle
  de ceux du carnet et de la bande d'identité — et écrits parce que
  `tab_strip_height`, pourtant écrite au bon endroit la veille, était
  encore calculée de mémoire. **Écrire la mesure une fois ne suffit
  pas : il faut la confronter au dessin.**
  - `a_tab_strip_takes_the_height_it_announces` dessine une vraie bande
    d'onglets à quatre échelles et compare ce que `tab_strip_height`
    promet à ce que la bande occupe réellement.
  - `a_wrapped_band_is_as_tall_as_its_model_says` fait de même pour
    l'arithmétique dont dépendent *tous* les plafonds de la maison —
    une rangée de boutons enveloppée vaut `n × row_height + (n−1) ×
    gouttière`, où `n` vient de `wrapped_rows`. Trois échelles, trois
    largeurs, de une à six rangées. Rien ne la vérifiait, alors que la
    bande d'identité du dossier, celle de la fiche médicament, celle de
    la carte vaccinale et celle des pièces en dépendent toutes.

  Chacun a été vérifié en y remettant le défaut qu'il refuse. Le second
  reprend celui que le fichier refuse déjà ailleurs — réserver ses
  rangées avec `interact_size.y` —, et il l'attrape : cent dix-huit
  pixels annoncés pour cent cinquante dessinés, soit huit par rangée.

  Et le premier a d'abord attrapé le testeur : mesurée au curseur, une
  bande « consomme » une gouttière de plus que sa hauteur, et
  `tab_strip_height` paraissait courte de huit pixels. Cette
  gouttière-là appartient à la disposition qui suit — `split_rows` la
  compte déjà entre ses rangées — et la rendre l'aurait comptée deux
  fois. Ce qui doit tenir dans le rectangle taillé, c'est le contenu ;
  les deux tests le disent maintenant explicitement.

## [0.159.0] - 2026-09-08

La suite de la même passe : ce qu'une seconde tournée de captures a
rendu, et une relecture des huit cent cinquante et une fiches contre
elles-mêmes.

### Fixed
- **La vue la plus ouverte de l'application ne montrait que ses
  boutons.** Onze actions dans un `horizontal_wrapped` sans plafond :
  à 1024x700 et à l'échelle 1,6 elles enveloppaient sur quatre rangées,
  deux cent cinquante pixels sur cinq cent quarante-cinq, et ce qui
  restait de la fiche médicament était son nom coupé en deux, aucune
  ligne de monographie, et les trois panneaux du bas rognés. La bande
  se plafonne désormais en part du volet et s'arrête sur deux rangées
  entières, comme la bande d'identité du dossier, les portes de
  l'explorateur et la légende de l'agenda. Ce qui apparaît à la place,
  sur un anticoagulant : « Antidote : Andexanet alfa ». Sur un écran
  large, rien ne change.
- **La bande de regroupement de la carte vaccinale** s'arrêtait au
  milieu d'une rangée de pastilles ; elle tombe sur une rangée entière.
- **`motif::section` peignait par-dessus le bord de son volet** :
  « Amérique du Nord » se lisait « Amérique du » au ras du bord, sans
  même l'ellipse qui aurait dit qu'il manquait quelque chose.
- **La bande d'archivage des pièces** coupait sa seconde rangée de
  genres par le milieu, et « Pièces au dossier » n'avait plus qu'un
  filet sous son titre.
- **Le choix rapide d'un acte débordait de l'écran des deux côtés.**
  Dix actes, une liste de thèmes et un bouton : à 1024x700 et à
  l'échelle 1,6 la fenêtre mesurait plus haut que l'écran, et comme
  elle est centrée elle perdait son propre titre en haut et « Fermer »
  en bas. Échap la ferme et l'invite le dit, mais une fenêtre dont on
  ne voit ni le titre ni le bouton se lit cassée. La liste défile ; le
  thème armé et le bouton qui ferme gardent leur rangée, hors du
  défilement.

### Changed
- **Dix-huit fiches se contredisaient elles-mêmes.** Les erreurs qui
  restent dans une base relue ne sont pas des champs vides, ce sont des
  champs qui ne peuvent pas être vrais ensemble. Les plus graves sont
  les chiffres, parce que c'est le chiffre que la délivrance retient :
  - **Médrol** posait l'équivalence de 4 mg pour 5 mg de prednisone,
    puis illustrait le piège du relais par « un comprimé de 16 mg
    n'étant pas un comprimé de 20 mg de prednisone » — alors qu'avec
    son propre rapport, 16 mg en valent exactement 20 ;
  - **Aubagio** prescrivait deux rythmes incompatibles pour le même
    contrôle sur la même période, toutes les deux semaines et une fois
    par mois ;
  - **Maviret** et **Solian** portaient deux seuils pour la même
    contre-indication : un patient Child-Pugh B traitable selon un champ
    et interdit selon l'autre, une clairance à 20 mL/min qui vaut un
    tiers de dose ici et un refus de délivrance là ;
  - **Insulatard** donnait deux durées d'action ; c'est `insulin.rs`
    qui a tranché, son profil étant tenu par un test ;
  - plus **Xarelto**, **Lariam**, **Sporanox** sur des chiffres, et
    **Izilox**, **Zeclar**, **Sterdex**, **Nordaz**, **Noctamide**,
    **Scandicaïne**, **Orgaran**, **Arimidex**, **Xolair** sur deux
    phrases qui s'excluent.

  Trois constats de plus ont été écartés à la relecture, et un
  quatorzième corrigé dans l'autre sens que celui proposé.

## [0.158.0] - 2026-09-08

Une passe d'interface et de contenu. Les défauts d'interface viennent
tous d'une même passe de `eyeball.sh … 1024x700 1.6`, regardée image par
image — la forme qui trouve ; le contenu vient du gisement que
`docs/CONTENU.md` désigne, les sections « Toxicité / marge
thérapeutique » des huit cent cinquante et une fiches.

### Added
- **Vingt et une règles de revue d'ordonnance de plus** — de
  soixante-six à quatre-vingt-sept. Ce qui manquait, par famille : la
  kaliémie qu'aucune classe n'annonce (le triméthoprime et la
  drospirénone, que personne ne lit comme des traitements
  hyperkaliémiants) ; l'efficacité perdue sans aucun symptôme
  (tamoxifène sous paroxétine, anticancéreux oral sous IPP, intégrase
  sous pansement gastrique, quinolone sous calcium, AVK sous inducteur) ;
  les couples qu'un métabolisme explique (Méthergin sous macrolide en
  post-partum, fluconazole sur AVK, élétriptan sous azolé, lamotrigine
  sur valproate, sofosbuvir sur amiodarone) ; les opioïdes lus autrement
  (l'agoniste partiel qui déloge l'agoniste pur, la naltrexone, le
  gabapentinoïde) ; et deux absences, l'anti-aromatase sans rien pour
  l'os sur cinq ans et l'isoniazide sans pyridoxine.
- **Onze règles de biologie de plus** — de quatre-vingt-dix-sept à cent
  huit. Le potassium que rien ne lisait des deux côtés ; le chiffre qui
  alerte quand le chiffre habituel rassure (la réserve alcaline sous
  gliflozine, c'est-à-dire l'acidocétose à glycémie normale) ; le
  chiffre qui se lit à l'envers (une TSH basse sous antithyroïdien n'est
  pas un surdosage) ; plus la thyroïde sous antiangiogénique et sous
  anti-PD-1, la thyrotoxicose de l'amiodarone, les CPK sous
  antipsychotique et sous lévodopa, le LDL sous JAK et sous mTOR, le
  phosphore sous ténofovir, la lipase sous valproate.
- **Quatre surveillances de plus** — de soixante et une à
  soixante-cinq : la T4 libre sous antithyroïdien, et les transaminases
  sous dronédarone, sous inhibiteur de JAK et sous interféron bêta.
- **Sept protocoles de plus** — de quarante-sept à cinquante-quatre,
  tirés de ce que le comptoir refuse ou diffère sans avoir de marche
  écrite : la recevabilité d'une ordonnance de stupéfiant avant toute
  délivrance, le méthotrexate dont la dose est hebdomadaire, la
  prescription restreinte, l'alerte de retrait de lot, la déprescription
  d'un IPP, la décroissance d'une benzodiazépine, et l'acte vaccinal.
- **Trois tables de conversion de plus** — de quarante-trois à
  quarante-six. Les AOD, les HBPM, les statines, les corticoïdes et les
  opioïdes avaient chacun la leur ; pas les AVK, alors que c'est le
  traitement dont le comptoir répond le plus souvent. S'y ajoutent la
  vitamine D — une famille où les compléments sont libellés en µg et les
  médicaments en UI, où trois dosages d'ampoules partagent un nom, et où
  trois dérivés hydroxylés n'ont aucune équivalence — et la conversion
  des unités de biologie, avec les deux pièges qui trompent le plus :
  l'azote uréique des comptes rendus anglo-saxons, qui n'est pas l'urée,
  et le DFG de Cockcroft en mL/min, qui ne se compare pas au DFG indexé
  du laboratoire.
- **`motif::tab_strip_height`** : ce qu'une bande d'onglets prend, écrit
  là où la hauteur est décidée.

### Fixed
- **Le dossier ouvert s'annonçait « Jean… ».** « Retour », « Replier »
  et « Né(e) le 03/07/1958 » ne laissaient au nom que cent quarante
  pixels à l'échelle 1,6. Élider le nom, c'est élider la seule chose qui
  dise de qui est le dossier : la date de naissance descend sur la ligne
  de contexte, qui est faite pour envelopper, et seulement quand elle
  coûte le nom.
- **Le plafond de la bande d'identité tombe sur une rangée entière.**
  Plafonnée aux pixels, elle s'arrêtait au milieu d'une rangée : « Plan
  de prise… » coupé en deux dans le sens de la hauteur sur tous les
  onglets du dossier, et la rangée des traitements tranchée sous ses
  puces. Le compte de rangées peut tomber à zéro, ce qui distingue ce
  cas des portes de l'explorateur : là-bas la première rangée *est* le
  contenu, ici l'en-tête porte déjà le nom.
- **Le volet de droite débordait par le bas sur les quarante-huit
  vues.** Il réservait trente-quatre pixels au journal personnel quand
  la seule phrase de ce cas — « Opérateur non renseigné. » — en
  demandait quarante à l'échelle 1,6, emportant avec elle le trait du
  bas de l'éditeur. Le défaut le plus répété de l'application, et le
  plus discret.
- **Une bande d'onglets se taillait dans une hauteur devinée.** Trois
  appelants écrivaient 24 ou 28 pixels pour une bande qui en fait
  trente-huit à l'échelle 1,6. Deux débordaient sur le panneau d'en
  dessous ; la troisième, seule posée dans un `inside`, dessinait
  « Interprétation » et « À surveiller » coupés dans leur cadre.
- **La rangée où l'on saisit un résultat de biologie**, trois défauts
  sur cinq contrôles : une largeur de champ écrite deux fois, une mesure
  qui disait `92.0` pour un champ que le dessin taille à cent
  quatre-vingt-dix, et des rangées réservées avec `interact_size.y` là
  où elles portent des boutons. « Ajouter » tombait sous le bord du
  panneau.
- **Le volet du mois montrait trois colonnes sur sept.** « Aujourd'hui »
  se lisait « Aujo » entre deux flèches dont la seconde n'était pas
  dessinée. Ce qui manquait à cette rangée n'était pas de la place,
  c'était **quel mois on regarde** : le nom du mois le dit et ramène au
  mois courant. Et les jours ne se touchent plus — un chiffre suit sa
  case, qui est fixée par la largeur du volet et non par l'échelle du
  texte.
- **Le champ du voisinage ne tenait pas sa propre invite** (« Mettre au
  centre.. ») et **la date du carnet peignait par-dessus le bord du
  panneau** (« … — aujour »).
- **L'invite d'un antibiotique passait sous son bouton** dans la fenêtre
  d'ordonnance, et **« Renouvellement » se lisait « Renouvellemer »** :
  la rangée de boutons de la table des locations, non bornée,
  élargissait la colonne du nom et poussait l'échéance hors du panneau.

## [0.151.0] - 2026-09-07

### Added
- **Deux bandes de plus tenues par un test**, écrites sur le modèle de
  celui du carnet — mesurer comme la vue mesure, dessiner comme elle
  dessine, comparer, à trois échelles de texte et à trois largeurs :
  - la **bande de titre** que six vues partagent (codex, dispositifs,
    protocoles, vigilance, classes, explorateur). Vérifié en y remettant
    la supposition d'avant — « 116 px si plus étroit que 940, sinon
    64 » —, qui coupe le sous-titre de vingt et un pixels dès l'échelle
    1 ;
  - la **rangée d'ajout d'un journal**, c'est-à-dire « Nouve » :
    vérifié en lui faisant compter une rangée pour deux, ce qui fait
    sortir la seconde de vingt-cinq pixels.

- **Et la première ligne de la feuille de route est tenue par un
  test.** « Rien ne sort du panneau à droite » ne se voit sur aucune
  capture prise au bon endroit : il faut le mauvais dossier, le mauvais
  volet et la mauvaise échelle. L'arithmétique des colonnes du carnet
  est sortie de la boucle de dessin — `carnet_columns` — pour qu'un test
  la parcoure : six largeurs de panneau, trois échelles, un carnet dont
  une ligne est volontairement bavarde. Vérifié en remettant les sept
  colonnes de toujours, qui font cinq cent vingt-trois pixels dans une
  table de deux cent cinquante.

  Le workspace passe de 44,9 % à 46,1 % couvert : ce qui se mesure se
  teste, et c'est la moitié de `src/app.rs`.

## [0.157.0] - 2026-09-08

### Changed
- **Une rangée de liste prend deux lignes quand elle peut se couper
  proprement.** Elles étaient sur une ligne avec une ellipse, et sur un
  volet étroit cela coûtait précisément ce que la rangée est là pour
  porter : « Bain de bouche à la chlorhexidine » et « Bain de bouche au
  bicarbonate » se lisaient toutes les deux « Bain de bouche … », et
  « Paul Bernard » se lisait « Paul … ». Deux lignes le règlent — mais
  egui coupe où il peut un mot plus large que la colonne, si bien que
  « Benzodiazépines » devenait « Benzodiazép / ines », ce qui se lit
  plus mal que l'ellipse. La seconde ligne n'est donc accordée que
  lorsque **chaque mot tient** dans la colonne ; le mot le plus long est
  mesuré au gabarit du « 0 », plus large que la moyenne des lettres,
  donc l'erreur penche vers l'ellipse et non vers la coupe. Et la
  moitié discrète est ajoutée avec un **vrai espace** et non le seul
  `leading_space`, qui est un écart en pixels et non une frontière de
  mot : c'est ce qui coupait « Efferalgan paracétamol » en
  « paracéta / mol ».

### Fixed
- **La rangée où l'on tape n'est plus rognée dans le carnet de
  vaccination.** Sur un volet de comptoir à l'échelle 1,6, la table des
  doses et la rangée de saisie ne tiennent pas ensemble. L'arbitrage de
  la maison est que le formulaire gagne — une table à qui il manque une
  ligne se lit et défile, une rangée coupée ne se tape pas — mais lui
  laisser *une* ligne les trahissait tous les deux : cette ligne
  n'affichait rien de lisible, et « Imprimer le carnet » sortait quand
  même tranché de vingt pixels par le bas. La table cède donc
  entièrement, et **le compte passe dans la légende du panneau** —
  « CARNET DE VACCINATION — 3 DOSE(S) » —, qui est déjà dessinée et ne
  coûte pas une ligne. Une phrase dit où aller les lire quand il reste
  la place de l'écrire. Un test tient le partage, vérifié en remettant
  la ligne concédée.
  Au passage, une panique évitée : `Rect::NOTHING` passé à egui pour
  « ne dessine rien » a ses bornes à l'infini et `Layout` divise dedans.
  On n'appelle simplement pas la fonction.
- **La table de biologie sortait par la droite, et elle ne défile que
  vers le bas.** Six colonnes non bornées dans un panneau de comptoir :
  « Usuel » n'était pas seulement coupé, il était inatteignable, et à
  l'échelle 1,6 la croix de suppression l'était aussi. Les quatre
  colonnes fixes se mesurent, l'analyte et l'intervalle se partagent ce
  qui reste, et s'il n'y a pas de quoi lire les deux, c'est l'intervalle
  qui part — la colonne « Lecture » dit déjà normal ou élevé, qui est la
  question qu'on pose au comptoir. Son plancher n'est plus « trois
  lignes » mais son en-tête et une rangée de la hauteur d'un bouton,
  creux compris : trois lignes de corps font soixante-sept pixels quand
  la rangée en demande quatre-vingt-quatre, et la seule ligne visible
  était coupée en deux.

## [0.156.0] - 2026-09-08

### Fixed
- **Trois mesures comptaient encore en pixels après la conversion.**
  `elide` — ce qui raccourcit un libellé pour qu'il tienne — recevait sa
  taille en points et la lisait en pixels : élidé à onze et peint à
  dix-huit, le texte revenait plus large que ce qu'on venait de mesurer.
  La phrase d'un résultat de « Dans le texte… » de même. Et la bande de
  suivi des entretiens était posée sur quatre constantes — le libellé
  élidé à cent cinquante pixels, l'année à cent soixante-dix, les
  pastilles à deux cent cinquante, quinze par pastille — si bien qu'à
  1,6 « Anticancéreux long cours » se réduisait à deux mots pour laisser
  la place à une année qui n'en avait plus besoin. Toutes ses colonnes
  se mesurent maintenant.
- **Et la prose des monographies non plus.** C'est le texte le plus lu
  de l'application — les paragraphes d'une fiche, les notes datées — et
  il était écrit à treize pixels quels que soient les réglages : sur un
  poste réglé à 1,6, tout grandissait autour de lui sauf lui. Il passe
  par `motif::pt` comme le reste.

### Changed
- **Le tableau de l'explorateur se replie, il ne défile plus de côté.**
  C'était la dernière vue à défiler des deux côtés, et pour une raison
  qui ne tenait pas : ses colonnes ne sont pas des phrases entières
  comme celles des tables de conversion, mais un nom, une valeur courte
  et une liste — et une liste enveloppe. Il suffisait de borner la
  dernière colonne. Non bornée, elle prenait sa largeur naturelle dans
  un `ScrollArea::both` qui la lui accordait, et « Organes altérés »
  sortait par la droite. Les largeurs sont en caractères et non mesurées
  sur huit cent cinquante et un noms : une passe de mesure par fiche,
  soixante fois par seconde, est ce que ce fichier refuse ailleurs.
  Ce qui reste de `ScrollArea::both` ailleurs est un filet et non une
  disposition : le registre et l'ordonnancier se replient d'abord et ne
  défilent que si même la forme repliée ne tient pas, et les tables de
  conversion défilent par choix, la colonne qui nomme la ligne tenue au
  bord.
- **Le même champ portait deux noms.** « Conseils au patient » sur la
  fiche et sur le papier, « IUP » dans les résultats de « Dans le
  texte… » : un sigle en dit moins que ce qu'il abrège, et la recherche
  renvoyait vers une section que la fiche n'appelle pas ainsi. De même
  « Ce qui doit faire consulter » sur la fiche imprimée, quand l'écran
  dit « Signes d'alerte » depuis la passe sur les phrases — l'écran et
  le papier disent maintenant la même chose.
- Le README comptait trente-huit tables de conversion et huit cent
  cinquante fiches : il y en a quarante-trois et huit cent cinquante et
  une. Les cinq dernières tables — LDL, les quatre piliers de
  l'insuffisance cardiaque, CHA₂DS₂-VASc et HAS-BLED, l'HbA1c, les
  stades de DFG — y sont décrites comme les autres.

## [0.155.0] - 2026-09-08

### Fixed
- **Les quatre actions du dossier sortaient par la droite du panneau.**
  Quand le nom remplit la ligne, elles passent dessous — sur une rangée
  `horizontal`, qui n'enveloppe pas : à l'échelle 1,6, « Plan de
  prise… » se lisait « Plan de pr » et rien ne le ramenait. Elle
  enveloppe, et **la bande compte la rangée** : comptée pour rien, elle
  poussait hors du bandeau ce qui la suit, dont la croix qui retire un
  acte. Les libellés sont écrits une fois — deux listes auraient
  divergé, et c'est la hauteur qui aurait perdu. Le test de la bande
  d'identité le tient, vérifié en décomptant la rangée.

### Fixed
- **Quatre cent sept tailles de texte ne suivaient pas
  `[ui] text_scale`.** `RichText::size(11.0)` est un nombre de pixels,
  point : l'échelle passe par le style, et un littéral ne passe pas par
  le style. À 1,6, les légendes des panneaux, les comptes, les dates du
  registre, les libellés à côté de chaque chiffre restaient à onze
  pixels pendant que les boutons autour d'eux grandissaient de
  moitié — c'est-à-dire que le texte le plus petit, celui que quelqu'un
  qui agrandit la typographie a le plus besoin de voir grandir, était le
  seul à ne pas bouger. Le registre des stupéfiants était le pire :
  « dossier 1 » en grand à côté d'une date minuscule.
  `motif::pt(ui, 11.0)` est ce onze, mis à l'échelle, et **la mesure
  passe par la même fonction que le dessin** — mesurée à onze et peinte
  à dix-huit, une colonne élide tout ce qu'elle porte, ce que la
  première version du correctif a fait au registre pendant une capture.
  Un test lit le texte du fichier et refuse le prochain littéral.

- **Une légende se coupait au milieu d'une rangée de pastilles.** Celle
  de l'agenda porte dix actes : deux rangées au comptoir, dont la
  seconde était tranchée par le bas du panneau, et une demi-pastille de
  couleur ne dit rien de plus qu'une pastille absente. La légende se
  peint maintenant rangée par rangée contre le rectangle qu'on lui
  donne, s'arrête sur une rangée entière et **compte ce qu'elle
  laisse** — « +2 » dit ce que la demi-rangée cachait. Ses propres
  mesures suivent aussi `[ui] text_scale` : quatorze pixels de rangée
  pour un texte de dix-sept virgule six, c'était deux lignes qui se
  chevauchaient.
- **La bande de portes de l'explorateur s'arrêtait au milieu d'une
  rangée.** Plafonnée aux pixels, elle coupait sa troisième en deux.
  Elle défile, donc rien n'était perdu — mais une porte coupée dans le
  sens de la hauteur ne se lit pas « il y en a d'autres », elle se lit
  « cassé » : le même défaut que l'onglet tronqué, et la même réponse,
  montrer moins mais entier.

### Changed
- **Une feuille déjà collée ne coûte plus la seule divergence.**
  L'onglet de conciliation porte trois panneaux et n'a pas la place des
  trois sur un volet de comptoir : à 1024x700 avec le texte à 1,6, la
  tête du panneau des réponses coûtait deux rangées de boutons, la boîte
  de collage réclamait ses cinq lignes et demie, et il ne restait
  **rien** pour ce que l'onglet existe pour montrer. C'est la règle du
  registre — sur un panneau trop court, la garniture part la
  première — appliquée ici : une feuille *déjà collée* est une
  référence, elle se range en une rangée qui dit combien de lignes elle
  porte, et « Modifier… » la rouvre. Tant qu'elle est vide elle garde
  son volet : on ne range pas la boîte dans laquelle on va coller. Le
  partage est sorti de la vue pour être mesurable, et un test tient
  qu'il ne prend jamais plus qu'il ne faut — vérifié en remettant le
  plancher qui gagnait.

### Added
- **Un chevron dit qu'une bande d'onglets continue.** Elle défilait
  déjà, mais sans barre — cachée exprès, une barre horizontale sous des
  onglets se lit comme un défaut de dessin —, si bien qu'un onglet
  coupé au milieu d'un mot ne se lisait pas « il y en a d'autres » mais
  « cassé ». À l'échelle 1,6, les six onglets d'un dossier ne tiennent
  pas, et les dix vues de l'espace de travail non plus. Peint et non
  typé, comme les pictogrammes de la barre d'outils : la face
  proportionnelle d'egui ne porte pas de chevron. Écrit d'abord vers
  l'extérieur, il se peignait à côté de la bande, c'est-à-dire nulle
  part — un test tient maintenant qu'il se pose dessus.

## [0.154.0] - 2026-09-08

### Added
- **La colonne qui nomme la ligne ne s'en va plus par la gauche.** Un
  tableau de conversion à six colonnes de phrases ne tient pas dans un
  panneau de comptoir : il défile latéralement, et c'est la seule forme
  honnête qu'il ait. Mais la première colonne de chacun de ces tableaux
  *nomme* la ligne — DCI, molécule, stade, classe, pilier —, et « 20 mg »
  lu sans le médicament auquel il appartient n'est pas lu. Elle est
  désormais réservée là où la grille la met et **peinte au bord de ce
  qui est visible** : au repos c'est le même pixel, une fois glissé le
  nom reste. Elle se corrige d'un clic comme n'importe quelle autre
  cellule, et la correction s'ouvre là où le nom est écrit, pas là où la
  grille l'aurait mis. Un test l'impose à trois échelles et à trois
  positions de la barre ; vérifié en laissant le nom suivre le contenu,
  où il part à quatre-vingt-seize pixels *à gauche* de la fenêtre.

### Changed
- **Quatorze bascules de disposition se comptent maintenant en
  caractères, plus en pixels.** « Deux colonnes si la fenêtre fait plus
  de mille quatre-vingts pixels » ne suit pas `[ui] text_scale` : à 1,6
  les mêmes mille quatre-vingts pixels ne portent plus que les deux
  tiers du texte, et l'application gardait deux colonnes qui ne
  tenaient chacune qu'une demi-phrase. C'est la règle déjà tenue pour
  les largeurs de champ, étendue aux seuils ; un test la lit dans le
  texte du fichier, comme celui qui refuse un `UPDATE` au registre, et
  il a été vérifié en remettant l'un des quatorze.
- **Les intitulés des tables de conversion passent au registre du
  reste.** Quarante-deux en-têtes de colonne et dix-sept titres
  disaient « Ce qu'on surveille », « Ce qui va de travers », « Le
  piège », « Où on l'applique ». Là où un terme professionnel existe,
  c'est lui : Surveillance, Effets indésirables, Écueil, Zones
  d'application, Conduite à tenir, Signes d'alerte, Idée reçue, Motif
  de refus. Le contenu des cellules, lui, n'est pas touché — ce sont
  des phrases, et une phrase n'est pas un intitulé. Effet de bord utile :
  des en-têtes plus courts sont des colonnes plus étroites, donc un
  tableau qui défile moins souvent.
- **Et huit lignes de posologie retrouvent le registre de leurs
  voisines.** Dans une colonne d'indications — « Cystite aiguë simple
  de la femme », « Conservation et conduite à tenir » —, « Ce qui se dit
  avant la première perfusion » détonne : c'est devenu « Avant la
  première perfusion », « À vérifier avant la prise », « Mesures
  associées ». Le contenu ne change pas, seul son intitulé.
- **Entrée valide trois rangées de plus** : la note d'un journal, le
  résultat de biologie, le matériel qu'on pose. Un journal se tient note
  après note et un bilan analyte après analyte ; aller cliquer
  « Ajouter » entre chacun est le geste qu'on finit par ne plus faire,
  après quoi le journal ne dit plus rien de la journée. Sur le champ de
  l'analyte, Entrée sert encore à choisir dans le catalogue — elle ne
  vaut validation que là où elle ne sert à rien d'autre.

### Fixed
- **Le registre mesurait ses colonnes deux fois.** La liste demandait
  si les neuf colonnes tiennent — pour ouvrir une grille ou pas — et la
  ligne le redemandait à une seconde fonction qui répétait le même
  calcul. C'est la règle que ce fichier répète partout, enfreinte par ce
  qui venait d'être écrit : `stup_widths` mesure une fois, et les deux
  lisent la même réponse. Un test parcourt les largeurs de deux cents à
  mille six cents pixels et tient trois choses — la mention garde son
  plancher, la rangée dépliée tient dans le panneau, et élargir le volet
  ne replie jamais une ligne qui était dépliée. Vérifié en y remettant
  la part fixe de « trente-huit pour cent ».
- La largeur minimale d'une colonne des tables de conversion s'exprime
  elle aussi en caractères : cent trente-deux pixels sont seize
  caractères à l'échelle 1 et dix à 1,6, où « Comprimés
  gastro-résistants » repartait sur cinq lignes. Ces colonnes-là restent
  des phrases entières et la table défile encore latéralement quand elle
  ne tient pas — une phrase ne se replie pas en note de bas de ligne
  comme le fait un numéro de lot.

## [0.153.0] - 2026-09-08

### Added
- **Une boîte de dialogue tient dans l'écran, et c'est une règle** —
  `dialog_size` — plutôt que trois formules recopiées. Trois d'entre
  elles n'y tenaient pas : les options par les deux côtés, les
  raccourcis par le haut et le bas, l'ordonnance par son seul titre. Le
  test est arithmétique, donc il n'a pas besoin d'écran ; vérifié en
  retirant la borne.
- **La bande d'identité du dossier est tenue par un test.** C'est celle
  dont la hauteur décide de tout le reste : trop courte, elle perd
  « Nouvel entretien » et le choix rapide des actes ; trop haute, elle ne
  laisse pas une ligne à la table en dessous. Elle ne prend plus la
  session entière mais les treize faits dont elle dépend — dont, pour la
  première fois, **la largeur et la hauteur de la vue**, qu'elle lisait
  elle-même : un test ne pouvait donc pas lui poser la question qui
  compte, « et sur un volet de trois cent quatre-vingts pixels, avec cinq
  traitements ? ».
  - Vérifié en y remettant les deux défauts qu'il refuse : la rangée des
    puces comptée pour une ligne quel qu'en soit le nombre, et le
    plafond retiré. Le premier passage du test ne les voyait ni l'un ni
    l'autre — il mesurait sur une vue trop large et trop haute pour que
    l'un ou l'autre morde.

## [0.152.0] - 2026-09-07

### Changed
- **Le tableau des entretiens ne défile plus à l'horizontale.** C'était
  la dernière table à le faire, et la plus utilisée : dix colonnes, la
  plupart des boutons, qui demandent ensemble près de mille quatre cents
  pixels de contrôles là où le volet central en offre cinq cent
  quatre-vingt-dix à 1024x700. Aucune colonne qu'on retire n'y change
  quoi que ce soit — c'est mesuré, le seul thème et la seule date en
  font déjà quatre cent cinquante — et la barre horizontale mettait
  « » Réalisé », l'action pour laquelle on ouvre ce tableau, hors de
  l'écran.
  - **Trois dispositions, deux seuils mesurés sur les actes du
    dossier** : la table à dix colonnes pour les écrans qui la portent ;
    en dessous, l'acte se plie en deux lignes — ce qu'il *est*, puis ce
    qu'on en *fait* ; sous ce seuil-là, en trois, l'action restant sur
    la première. Un dossier qui ne porte que des BPM n'a ni les
    pastilles d'un TROD ni le bouton d'ordonnance : le mesurer sur le
    pire cas possible le punirait de ce qu'il n'a pas.
  - Et **les dix cellules sont écrites une seule fois**, appelées depuis
    les trois dispositions : `ActsRow` dit ce qu'une rangée lit,
    `ActsOut` les dix-sept écritures qu'elle peut demander. Deux copies
    du même code divergent le jour où l'on en corrige une.
- **Le registre des stupéfiants non plus, et c'était la dernière.** Ses
  neuf colonnes demandent près de huit cents pixels avec une mention
  lisible, et le panneau du registre en offre quatre cent vingt à
  1024x700 : la ligne se plie alors en deux — le jour, la nature, ce qui
  entre, ce qui sort, le solde au-dessus, c'est-à-dire la ligne telle
  qu'elle s'additionne ; en dessous le numéro, le produit, le dossier et
  la mention, qui répondent à « pour qui » et non à « combien ».
  - Les sept cellules sont écrites une seule fois et appelées depuis les
    deux dispositions : un registre dont deux copies divergeraient ne
    prouverait plus rien.
  - **Et leurs largeurs sont données, jamais prises au contenu.** Pliée,
    la ligne n'est plus dans une grille, et un registre dont les dates
    ne tombent pas les unes sous les autres ne se relit pas.
  - Le gabarit d'une quantité passe de « = 0000,000 » à « = 9999,9 » :
    trois décimales que la balance ne rend pas coûtaient quatre-vingts
    pixels par colonne, assez pour plier le registre sur un écran qui
    portait ses neuf colonnes.
- **Une cellule de table réserve sa place avant de la remplir.** Un
  `ui.scope` n'annonce pas sa taille : dans une rangée qui enveloppe,
  egui ne sait pas qu'elle ne tiendra pas et ne va pas à la ligne — le
  registre plié perdait sa colonne « Solde » par la droite sans qu'aucune
  barre ne le dise.
- La règle que quatre tables partagent porte enfin un nom —
  `table_shape`, « la forme la plus riche qui tienne » —, avec un test
  qui tient les trois choses qui comptent : qu'aucune forme retenue ne
  laisse la colonne qui plie sous son plancher, qu'un panneau plus large
  n'en rende jamais une plus pauvre, et qu'il en sorte toujours une.

## [0.150.1] - 2026-09-07

### Fixed
- `chars_wide` ajoutait un remplissage que le nombre qu'elle remplace
  comprenait déjà : les champs convertis sortaient un sixième trop
  larges. Mesuré plutôt que supposé — le « 0 » du corps fait huit pixels
  à l'échelle 1, donc trente caractères font bien les deux cent quarante
  que le champ demandait, et trois cents à l'échelle 1,25, ce qu'il
  n'avait jamais.

## [0.150.0] - 2026-09-07

### Added
- Trois bandes de titre — codex, dispositifs, protocoles — mesuraient
  leur champ de nom à « 220 pixels » pendant que le champ, lui, se
  mesurait sur son invite. Deux mesures d'une même chose divergent
  toujours : c'est la même expression des deux côtés maintenant.
- **Un test tient le défaut qui a ouvert la nuit.** La rangée de saisie
  du carnet comptait deux rangées et en dessinait trois : le test mesure
  comme la bande mesure, dessine comme la bande dessine, et compare — à
  trois échelles de texte, à trois largeurs de panneau, et dans les deux
  états de la rangée. Vérifié en remettant la mesure fausse.
  `carnet_form_widths` ne prend plus la session entière mais le seul
  booléen dont elle dépend : une mesure qui demande une base de données
  pour être vérifiée ne l'est jamais.
- **La largeur d'un champ de saisie ne s'écrit plus en pixels, et un
  test le refuse.** Elle s'exprime en *caractères* de la fonte du corps
  (`chars_wide`), ou par ce que son invite demande, ou par une mesure
  prise plus haut — vingt-neuf champs restaient à 80, 96, 240 ou 300
  pixels selon l'endroit, donc insensibles à `[ui] text_scale`. Le test
  lit le texte de `app.rs`, comme
  `the_register_can_only_ever_be_written_to` lit celui de `db.rs` : une
  relecture ne tient pas une règle. Vérifié en en remettant un.

## [0.149.0] - 2026-09-07

### Added
- **Un `egui::Context` tourne dans un test, sans harnais et sans
  écran.** `Context::run` dessine, les fontes sont là, et `ui.fonts(…)`
  mesure dans la fonte qui dessinera : les fonctions qui *mesurent* une
  bande avant de la carver sont donc testables, ce que « on ne peut pas
  couvrir une vue » avait laissé croire impossible. Deux tests le font,
  et ils sont le modèle pour la suite —
  `an_invitation_too_long_for_its_field_is_shortened`, et
  `a_wrapped_row_is_counted_as_it_is_drawn`, qui compare ce qu'une bande
  **mesure** à ce qu'elle **dessine**, à trois échelles de texte : c'est
  exactement le défaut qui coupait « Imprimer » du carnet.

### Fixed
- **Cinq champs annoncés modifiables ne l'étaient pas.** Les initiales
  et l'heure d'un entretien, et le libellé, le seuil et l'unité d'un
  produit du registre recevaient chacun une copie de la valeur
  refabriquée à chaque image — `itv.operator.clone()` juste avant le
  `TextEdit`. Or un `TextEdit` d'egui ne garde pas son contenu : il le
  tient dans le `String` qu'on lui prête, et lui en prêter un neuf
  soixante fois par seconde efface la lettre tapée avant qu'on ait pu la
  relire. Le texte en cours vit maintenant dans la session, et repart de
  la base une fois écrit.
- **Un forfait de location à décimales était impossible à saisir.** Le
  montant était relu et reformaté à *chaque frappe* : « 1, » se relit 1
  et se réécrit « 1 », donc taper « 1,5 » donnait « 15 ». Il est relu
  quand le champ se ferme.
- **Deux boîtes de dialogue de plus sortaient de l'écran** à
  `[ui] text_scale = 1,6` : l'ordonnance protocolisée et
  « Imprimer la fiche ». La première par son titre — « Ordonnance —
  Angine à streptocoque du groupe A — TROD positif » fait à lui seul
  plus large que l'écran, et la barre de titre d'une fenêtre egui ne se
  replie pas, elle élargit la fenêtre : l'indication se lit maintenant à
  l'intérieur, où elle s'enroule. La seconde par des hauteurs écrites en
  dur — deux cent soixante pixels de liste, soixante-quatorze de texte
  libre — qui ne laissaient plus de place aux boutons.
- **Le passage de fumée ouvre chaque vue dans une troisième forme**,
  1024x700 à `text_scale = 1.6` : c'est là que les planchers croisent
  les plafonds, et c'est la forme où les trois défauts ci-dessus se
  voyaient. Plus `scripts/shot.sh`, qui capture **une** vue dans une
  forme choisie — la boucle qu'on répète vingt fois en corrigeant une
  bande.
- **Les invites de recherche des listes se raccourcissent aussi.**
  « Chercher une préparation… » s'arrêtait à « Chercher une prépa » dès
  qu'on grossissait le texte, et pareil pour les protocoles, les
  dispositifs et le catalogue des stupéfiants. La même règle que le
  volet de gauche, par la même fonction.
- **Le mode d'emploi imprimé décrivait trois onglets au dossier
  patient ; il y en a six** depuis les locations, la conciliation et les
  pièces. Et il annonçait « vingt-sept références datées » là où la base
  en porte quarante-trois : un nombre écrit dans une phrase est un
  nombre qui vieillit, la phrase ne le cite plus.
- **Vingt-trois champs de saisie étaient plus étroits que leur
  invite** — ou le seraient devenus. Leur largeur était une constante en
  pixels, donc insensible à `[ui] text_scale` : « imprimé sur le
  bulletin d'adhésion » sortait de son champ de trois cents pixels dès
  l'échelle 1,4. Chacun est maintenant au moins aussi large que ce qu'il
  invite à écrire, et grandit avec le texte.
- **La fenêtre des options débordait par le bas au-delà de 1,25.** La
  hauteur de son corps était « l'écran moins deux cents pixels », ce
  qui ne compte pas la rangée d'onglets — laquelle passe à deux rangées
  dès qu'on grossit le texte — ni la rangée « Enregistrer / Fermer »,
  qui sortait alors par le bas. Elle prend ce qui reste, mesuré.
- **« Rechercher un p ».** C'est ce que le champ du volet de gauche
  affichait — sur *toutes* les vues de l'application, puisque ce volet y
  est toujours : cent trente pixels ne portent pas « Rechercher un
  patient… ». L'invite est raccourcie quand elle ne tient pas ; le
  panneau porte déjà son titre, « Patients » ou « Médicaments », et
  quand le nom ne tient pas on garde le verbe.

## [0.148.0] - 2026-09-07

### Changed
- **Le champ des traitements accepte enfin ce que la base ne connaît
  pas.** Une ordonnance porte des produits qui ne sont pas encore au
  référentiel, et taper leur nom ne rendait rien : il fallait quitter le
  dossier, aller à la base, créer la fiche, revenir. La dernière
  proposition est maintenant « + Créer la fiche « … » », et c'en est
  bien une — jamais un second catalogue de noms libres à côté du
  référentiel, qui ne serait ni cherchable, ni imprimable, ni relié à
  une interaction. Dernière de la liste et jamais sous le curseur au
  départ : on ne crée pas une fiche par mégarde en tapant vite.
- **L'onglet ouvert décide de la part du bandeau du dossier.** Sur
  « Entretiens » la bande *est* le poste de travail — les traitements,
  ce que la revue en dit, le choix rapide d'un acte — et elle garde ses
  quarante-cinq pour cent. Sur les autres onglets elle n'est plus que le
  contexte : on ouvre « Conciliation » pour voir la conciliation, et à
  1024x700 celle-ci montrait ses en-têtes au-dessus d'une seule ligne
  tranchée. C'est ce que l'application peut décider seule ; « Replier »
  reste le levier de l'opérateur.
- « Retour (Échap) » devient « Retour », la touche passant dans la
  bulle : le rappel du raccourci coûtait cinquante-cinq pixels sur la
  rangée où le nom du patient n'en avait déjà plus.

### Fixed
- **La fenêtre des raccourcis clavier dépassait par le haut et par le
  bas.** La liste est plus longue qu'un écran de comptoir : à 1024x700
  le titre et la dernière rangée étaient coupés par les bords, et une
  fenêtre centrée qu'on ne peut ni déplacer ni rapetisser ne laisse
  alors aucun recours. Elle défile dans ce que l'écran peut lui donner.
- **La fenêtre des options sortait de l'écran des deux côtés.** À
  1024x700 les libellés de sa colonne de gauche étaient coupés par le
  bord — et rien ne permettait de la rapetisser, puisqu'elle est
  centrée. Une fenêtre egui grandit avec son contenu tant qu'on ne lui
  donne pas de maximum, et sur la page « Base » ce contenu porte une
  grille dont les colonnes ont une largeur incompressible : un champ de
  chemin et son bouton « Parcourir… ». La fenêtre prend maintenant sa
  taille de l'écran, et son corps défile dans les deux sens : mieux vaut
  atteindre une grille en défilant que ne pas pouvoir lire les libellés.
- « MDP » sous le nom de l'application, sur l'écran de verrouillage,
  devient ce que l'application fait — le champ juste dessous dit déjà
  « Mot de passe ».
- **Une seule ligne bavarde élargissait tout le registre des
  stupéfiants.** Deux de ses neuf colonnes n'avaient pas de longueur
  bornée — le libellé du produit, et la mention qui recolle
  prescripteur, fournisseur, référence, remarque et opérateur — et un
  `Grid` d'egui donne à chaque colonne la largeur de son plus long
  contenu : la colonne « Dossier » sortait du panneau la première.
  Les huit premières largeurs se mesurent maintenant, la neuvième est
  la soustraction, et la mention se replie dedans. (À 1024x700 avec les
  deux volets ouverts, le registre demande toujours plus de largeur que
  le panneau n'en a : il défile, comme avant, mais du minimum.)
- **La rangée du jour de l'agenda dépassait par la droite.** Ses
  largeurs étaient des constantes — 110 pour la catégorie, 52 par
  heure, 130 pour la répétition — et le seuil « plus étroit que 560 »
  en était une autre : à `[ui] text_scale = 1,25` les listes
  déroulantes sont plus larges que ces nombres, la réserve était donc
  trop courte, le champ du titre tombait à son plancher et
  « Formation, réunion… » se lisait « Formation, réuni… ». Tout est
  mesuré, seuil compris.
- **« Ajouter » se dessinait sous la barre de défilement** — et se
  lisait « Ajout ». La rangée d'ajout d'un journal se mesurait sur
  `available_width`, qui compte la place que la barre prendra ; elle se
  mesure maintenant sur le rectangle de découpe, qui ne ment pas.
- **La date qu'on venait de taper était coupée dans le champ où on
  l'avait tapée.** Les cinq champs de date de l'application étaient
  larges de 80, 92, 96 ou 100 pixels selon l'endroit — des constantes,
  donc insensibles à `[ui] text_scale` —, et à 1,25 « 07/09/2026 » se
  lisait « 07/09/202 » dans le tableau des entretiens. Ils se mesurent
  maintenant, tous par la même fonction, sur leur invite et sur la date
  qu'ils afficheront ; les initiales et l'heure aussi.
- **Le tableau des entretiens comptait dix ou onze cellules par
  ligne.** L'heure n'est dessinée que lorsqu'un rendez-vous est posé, et
  dans un `Grid` chaque widget est une colonne : une ligne portant un
  RDV décalait donc tout ce qui la suivait — sur la table même que ce
  `Grid` a été introduit pour aligner. La date et l'heure sont une
  cellule.
- **La bande de l'agenda se coupait au milieu d'une rangée de
  boutons.** « Anticancéreux long cours » à moitié dessiné sous le filet
  du panneau se lit comme cassé ; la même bande arrêtée une rangée plus
  haut dit simplement qu'elle défile. Sa part s'arrondit au nombre
  entier de rangées, cadre du panneau compris.
- **« Nouve ».** C'est ce que le champ d'ajout du journal affichait sur
  la colonne « Notes datées » d'une fiche médicament, large de cent
  quatre-vingts pixels : le champ prenait « ce qui reste, au moins
  soixante », et soixante pixels ne portent pas « Nouvelle note… ». Sous
  la largeur qu'il faut à l'invite et au bouton réunis, le bouton passe
  **dessous** — et la place qu'il prend est comptée avant, sans quoi
  c'est la rangée entière qui sortait du panneau.
  - La fiche technique, à côté, rend de la largeur quand les deux listes
    n'en ont plus assez : elle se replie, le journal non.
  - Et le bandeau du bas garde un plancher **mesuré** sur ce qu'il
    porte — un cadre, une légende, une ligne de note et la rangée de
    saisie. Cent quarante pixels écrits en dur laissaient « Notes
    datées » avec quatre pixels de puits ; la monographie au-dessus ne
    perd rien, elle défile.
- « Fermer (Échap) » devient « Fermer » sur la fiche médicament, la
  touche passant dans la bulle : la bande de boutons de la fiche
  descend de quatre rangées à trois.
- **Deux autres tables sortaient par la droite**, du même défaut que le
  carnet : les locations et les pièces numérisées finissaient leur
  rangée par des boutons, et il fallait défiler à droite pour les
  atteindre puis à gauche pour relire de quoi il s'agissait. Elles
  prennent maintenant deux formes selon la largeur — toutes leurs
  colonnes, ou le libellé seul avec le reste en note sous lui et les
  boutons sur une rangée à eux.
  - Au passage, la liste des pièces posait ses boutons **directement
    dans la grille** : dans un `Grid` d'egui chaque appel est une
    cellule, donc l'état « justifier », qui pose jusqu'à six boutons,
    faisait passer la table de six colonnes à onze pour une seule ligne.
  - Et la colonne des dates s'y dessine en chasse fixe alors qu'elle
    était mesurée en proportionnelle : « 31/08/2026 » se lisait
    « 31/08/20… » sur une colonne qui avait la place.
- « Confirmer la suppression » devient « Confirmer ? » — un bouton de
  vingt-quatre lettres qui prend la place de « Repris… » et de
  « Renouv. » réunis, à l'endroit exact où « Supprimer… » se trouvait
  une fraction de seconde plus tôt.
- « Justifie… » devient « Justifier… » : le libellé était conjugué.
- **« Facturé sur ce dossier : -0 € ».** Une somme de flottants rend le
  zéro négatif dès qu'un terme vaut `-0.0`, et `{:.0}` le recopie tel
  quel : au comptoir, un montant écrit « -0 € » se lit comme une erreur
  de caisse. Un seul formatteur pour les trois endroits qui écrivaient
  des euros, et un test qui tient la règle.
- **Le tableau des entretiens montrait ses en-têtes au-dessus de rien,
  et cette fois c'est réglé.** Le récapitulatif des séquences — une
  ligne par famille d'actes, puis les totaux — prenait soixante pixels
  sur cent vingt-cinq d'un dossier à deux familles, et il ne restait
  rien pour les lignes. Il illustre le tableau, il ne le remplace pas :
  quand le panneau ne peut payer les deux, **la garniture part la
  première**, comme la courbe de stock du registre tombe pour que les
  lignes du registre survivent. Mesuré : le tableau garde ses en-têtes,
  une rangée de contrôles et sa barre, et le récapitulatif prend ce qui
  reste — ou rien.
- **Le carnet de vaccination ne sortait plus par la droite.** Corriger
  une ligne échangeait ses six colonnes contre six champs et deux
  boutons *dans la rangée du tableau* — plus large que le panneau à
  toute taille utile —, si bien qu'il fallait défiler à droite pour
  atteindre « Enregistrer » puis à gauche pour relire le nom du vaccin.
  - **La correction se tape dans la rangée du bas**, celle où l'on écrit
    déjà une dose : les mêmes champs, à une colonne près, et cette
    rangée-là sait se replier sur deux lignes quand le panneau est
    étroit. La ligne en cours de correction se voit dans la table, sinon
    on ne saurait pas laquelle la rangée du bas porte.
  - La table n'a donc plus qu'une barre verticale, et **trois formes**
    selon ce que la largeur permet : sept colonnes, quatre, ou deux. Ce
    que la largeur refuse descend sous le nom, à la ligne où le rappel
    dû et la remarque se lisaient déjà. Rien ne disparaît.
  - Et la note de bas de ligne était posée dans la colonne « Dose » : un
    `Grid` donne à chaque colonne la largeur de son plus large contenu,
    donc cette note élargissait la colonne à sa longueur et emportait
    tout ce qui suivait vers la droite. C'était **elle**, la barre
    horizontale, autant que la rangée de correction.
- **Le bandeau du dossier se replie.** À 1024x700 il prend la moitié de
  la hauteur utile, et l'onglet en dessous n'a plus de quoi montrer
  *une* ligne de sa table : le carnet affichait « Vaccin · Dose · Date »
  au-dessus de rien, sur l'onglet qui existe pour montrer les doses. Ce
  n'est pas un partage à régler — la contrainte est au-dessus du
  partage. « Replier » rend deux cents pixels d'un clic, `layout.toml`
  s'en souvient, et une correction en cours déplie d'office : on ne
  range pas le formulaire dans lequel on tape.
- **Le nom du patient tenait dans ce que les boutons laissent.** Posés
  sans limite, le nom et la date de naissance se peignaient par-dessus
  le bord droit du panneau : « Né(e) le 03/07/1958 » se lisait
  « …195 ». Même famille que le titre de `motif::panel` qui débordait
  sur le panneau d'à côté — un `Painter` peint où on lui dit.
- **Les largeurs de la rangée de saisie étaient des constantes en
  pixels.** 96 pour la date, 90 pour le lot : à `[ui] text_scale = 1,25`
  « JJ/MM/AAAA » sortait du champ par la droite, et une invite qu'on ne
  peut pas lire n'invite à rien. Elles se mesurent, par la fonction qui
  sert **à la fois** à mesurer la bande et à la dessiner — deux mesures
  d'une même chose divergent toujours, et celle-ci comptait deux rangées
  là où trois étaient dessinées : « Imprimer le carnet » tombait sous le
  bord du panneau.
- Les trente-quatre pixels réservés à chaque image aux deux avis du
  carnet — l'acte non créé, la mention de l'officine — se mesurent aussi.
  Ni l'un ni l'autre n'est là la plupart du temps.
- Et la rangée de saisie se mesurait sur seize pixels de moins que la
  largeur où elle est dessinée, « pour la barre de défilement » : trois
  rangées comptées, deux dessinées, et quatre-vingts pixels de vide sous
  le formulaire — pris à la table au-dessus.

### Changed
- **Le vaccin se saisit dans un seul champ, avec autocomplétion.** Il y
  avait une liste déroulante **et** un champ libre : deux widgets pour
  une seule information, et il fallait décider si le vaccin était au
  calendrier avant de pouvoir taper son nom. On tape, les propositions
  répondent, les flèches et Entrée choisissent — et ce que le calendrier
  ne connaît pas s'inscrit quand même. À l'écrit, cela rendait aussi les
  deux cents pixels qui faisaient passer la rangée à trois lignes.
- **Entrée valide la rangée du carnet**, à l'écriture comme à la
  correction, et le foyer revient au nom après chaque dose écrite. Une
  rangée de six champs qu'il faut ensuite aller cliquer n'est pas une
  rangée où l'on tape ; un carnet se remplit dose après dose, et chacune
  coûtait un aller-retour à la souris.
- **Une passe sur les phrases.** Le sous-titre du registre énonçait sa
  propre règle en une phrase que personne ne relit à chaque délivrance :
  il est parti, et la place vaut mieux à une ligne de registre. Le plan
  de prise ne dit plus « Mon plan de traitement », « À quoi ça sert »,
  « Quand le prendre » et « Ce qu'il faut savoir » mais « Plan de
  prise », « Indication », « Posologie » et « Remarques » — c'est une
  feuille qu'un professionnel remet, pas un livret. Et une quinzaine de
  bulles d'aide qui commentaient au lieu de dire ont été ramenées à ce
  qu'elles annoncent.

### Added
- **Des traits relient les traitements qu'une règle nomme ensemble.**
  Ce que la revue trouve *entre* deux lignes de l'ordonnance se lisait
  en pastilles à côté des puces, sans dire lesquelles étaient en cause :
  on lisait « IEC + AINS » et on cherchait des yeux lesquels des huit
  traitements c'était. Un trait le dit sans un mot.
  - Rouge pour ce qui alerte, ambre pour ce qui demande un regard, et le
    **niveau du trait veut dire quelque chose** : l'alerte passe au plus
    près des puces, la mise en garde juste en dessous. Sans cela deux
    liens sur la même rangée se superposaient au pixel près.
  - Sous la puce et jamais au-dessus, où le trait barrerait le nom ; et
    seulement entre deux puces de la même rangée, parce qu'un trait qui
    saute d'une ligne à l'autre ne montre plus rien. Le tout dans la
    gouttière qui existait déjà : la rangée des traitements est pleine à
    1024x700 et rien ne peut lui prendre un pixel.

### Changed
- **L'ambre des mises en garde vient du thème.** Il était écrit en dur —
  le même `0x7a5c1f` recopié à **neuf** endroits — alors que la maison
  veut que toute couleur de chrome sorte de `motif::THEMES`, précisément
  parce qu'une couleur choisie pour une palette détonne sur les cinq
  autres. Un ambre réglé sur le bleu-gris de mwm n'a rien à faire sur
  l'olive de HP VUE, et sur « contraste » il était trop pâle pour ce
  qu'il annonce.
  - `motif::warn()` rejoint donc `alert()` et `accent()`, avec sa valeur
    dans les six palettes, et `every_palette_can_be_read` l'y tient :
    elle porte du texte blanc comme l'alerte, donc elle doit être assez
    sombre pour lui — et **assez distincte de l'alerte**, puisque les
    deux se lisent côte à côte sur la même ordonnance et que deux rouges
    voisins ne disent plus lequel presse.
  - La correspondance gravité → couleur vit à un seul endroit
    (`severity_color`) au lieu d'être réécrite à chaque appel.

### Added
- **Le médicament et sa posologie d'un même geste, au clavier.** Ajouter
  un traitement se faisait à la souris — taper, puis cliquer l'un des
  quatre libellés proposés — et la posologie du dossier ne se posait
  que dans l'onglet Conciliation, en texte libre, alors que la base
  **livre** les posologies de chaque fiche, par indication, avec la
  remarque qui va avec.
  - Les flèches et Entrée parcourent les suggestions, comme dans les
    autres listes carvées de l'application, et la sélection **se voit** —
    des rangées et non des libellés cliquables, sans quoi les flèches
    déplacent un curseur invisible. Six propositions au lieu de quatre :
    une liste qui s'arrête avant la bonne réponse oblige à retaper.
  - Le médicament choisi, ses lignes de posologie s'affichent aussitôt :
    un clic les pose au dossier, et la remarque du comptoir — à jeun, à
    distance du fer, pas de pamplemousse — est sous la souris. Rien
    n'est imposé et rien n'est inventé : une fiche sans ligne livrée
    n'en propose aucune.
  - **Et le même champ atteint les traitements déjà là.** Ils étaient
    écartés des suggestions parce qu'on ne pouvait que les ajouter ;
    taper un nom qu'on avait déjà ne rendait donc rien, alors que c'est
    le geste qu'on fait pour changer une posologie. « + » ajoute, « ≡ »
    dose, et le signe le dit avant qu'on appuie.
  - Le tout **sans un pixel de plus** : la rangée des traitements est
    déjà pleine à 1024x700, et tout ce qu'on y ajoute — même une
    pastille — la fait passer à deux lignes, ce qui pousse « Nouvel
    entretien » hors de la bande plafonnée et emporte le choix rapide
    des actes. Les propositions occupent la rangée que la bande comptait
    déjà.
  - Le champ de saisie prend enfin la hauteur d'un champ : les vingt
    pixels écrits en dur étaient sous la taille minimale d'egui.

### Fixed
- **La table des entretiens montrait ses en-têtes au-dessus de rien.**
  Sur l'onglet du dossier à 1024x700, « ENTRETIENS » affichait
  « Type · Acte · Thème · Fait le / par · État » et **aucune ligne** —
  sur la vue la plus ouverte de l'application, et pour la table que le
  dossier existe pour montrer.
  - L'arbitrage était pourtant écrit, et le commentaire disait déjà que
    le tableau prime sur le journal : c'est le code qui ne le faisait
    pas. `notes_h` était remonté au plancher du journal **quoi qu'il
    reste**, si bien que le journal prenait ses cent dix pixels et que
    la table héritait du reste. Quand les deux planchers ne tiennent
    pas, c'est le journal qui cède maintenant, jusqu'à sa rangée de
    saisie.
  - Et ce plancher-là compte la légende du panneau, son filet et ses
    marges : le panneau les prend avant de rendre le moindre pixel
    d'intérieur, et sans eux la rangée « Nouvelle note… » sortait
    tranchée par le bas. Une rangée où l'on tape, coupée, ne se tape
    pas.
  - Les deux planchers du journal se mesurent à des échelles
    différentes — des lignes de texte d'un côté, une rangée de contrôles
    de l'autre — et rien ne garantissait leur ordre. À petite police le
    strict passait au-dessus du confortable, et `f32::clamp` ne rend pas
    un nombre bizarre dans ce cas : il fait tomber l'application. Ils
    sont ordonnés par construction.
  - **Ce que cela ne règle pas, et il faut le dire** : un dossier qui
    porte deux familles d'actes retrouve ses en-têtes entières mais
    toujours aucune ligne à cette taille. Le journal est alors à son
    plancher strict, il n'y a plus rien à prendre, et la contrainte est
    au-dessus — le bandeau du patient occupe la moitié de la hauteur.
    Le replier ou le faire défiler avec l'onglet est un changement de
    comportement, pas un réglage de partage, et cela se décide.

## [0.147.2] - 2026-09-04

### Changed
- **Le config.toml livré dit comment régler le lecteur de carte
  Vitale**, ce qu'aucune documentation ne portait et qui coûte une
  demi-journée à qui l'apprend seul.
  - Les lecteurs Kapelse — kap&link, eS-KAP-Ad, TI-KAP — sont livrés en
    mode PSS (l'ancien protocole GALSS) et **pas** en PC/SC. Le mode se
    règle sur l'appareil, et l'entrée n'existe qu'à partir du
    micrologiciel 04.22.
  - **Le mode PC/SC est un mode de la liaison USB.** Un lecteur relié en
    Ethernet parle PSS, que `SCardListReaders` ne voit pas : sur ces
    postes l'application ne trouve aucun lecteur, et ce n'est pas une
    panne — c'est la liaison qui ne mène pas là.
  - Sous Linux, le démon pcscd et le pilote libccid en plus ; le
    kap&link se présente en 2947:0101, que les anciennes versions de
    libccid ignoraient. Sous Windows il n'y a rien à installer : PC/SC
    fait partie du système.
  - Et l'avertissement qui compte : basculer en PC/SC un poste dont le
    LGO lit la carte en PSS lui retire sa lecture, donc sa facturation.

## [0.147.1] - 2026-09-01

### Fixed
- **La lecture de la carte Vitale disait « Carte lue (0 octets) » quand
  elle n'avait rien lu du tout.** Sans séquence APDU configurée, le
  lecteur est vu, la carte est connectée, son ATR s'affiche — et **rien
  ne lui est demandé**. Le message annonçait pourtant une lecture, et
  renvoyait vers la séquence en la disant « peut-être à compléter » là
  où c'est la cause certaine. Un diagnostic qui hésite quand il sait est
  un diagnostic qu'on ne suit pas.
  - Le cas est nommé : aucune commande configurée, et où la renseigner.
    La séquence relève des spécifications SESAM-Vitale, sous licence,
    et c'est pourquoi elle vit dans la configuration et non dans le
    programme.
  - Et le lecteur est rapporté **dans les deux cas**. Sans séquence *et*
    sans lecteur, les deux manquent, et n'en dire qu'un envoie corriger
    la mauvaise moitié.

## [0.147.0] - 2026-09-01

### Added
- **Le registre des stupéfiants : la refonte des onglets.** Il avait
  grandi en trois écrans — deux portes sur la page « Base médicaments »
  en 0.143, une vue à lui en 0.145, un troisième onglet en 0.146 — et
  jamais été dessiné une fois.
  - **Les listes sont des tableaux.** Les quatre listes du registre
    étaient des rangées de `horizontal` : chaque colonne commençait où la
    précédente avait fini, si bien que rien ne tombait sous rien et que
    le défaut empirait à chaque ligne écrite. C'était le seul endroit de
    l'application qui enfreignait encore cette règle. Neuf colonnes, les
    mêmes pour les trois listes, avec des cellules vides là où une liste
    ne s'en sert pas.
  - **Le solde en face de chaque ligne**, qui est la colonne par
    laquelle un registre se lit et qui n'existait pas :
    `ordonnancier::running` la calculait depuis toujours et seule la
    courbe la consommait. Il fallait additionner de tête pour vérifier
    quoi que ce soit.
  - **Une délivrance s'écrit sans quitter l'écran.** Le formulaire
    n'avait pas de champ « dossier » : il fallait que le dossier soit
    ouvert, et le seul chemin pour l'ouvrir — le tiroir des patients —
    bascule sur l'écran de recherche et referme le registre. Le message
    disait « ouvrez d'abord le dossier du patient » et le moyen de le
    faire mettait dehors. La règle ne bouge pas — une délivrance sans
    dossier ne prouve rien — c'est le moyen qui entre dans le
    formulaire.
  - **Le comptoir au clavier** : les quatre natures aux chiffres 1 à 4
    comme le choix rapide des actes, Entrée pour inscrire, les
    raccourcis de quantité tirés de la **durée maximale de la famille du
    produit** au lieu d'une liste écrite en dur qui proposait 28 pour un
    produit plafonné à 7, et les derniers prescripteurs de ce produit en
    pastilles.
  - **La douchette.** Un lecteur USB tape comme un clavier : scanner la
    boîte choisit le produit suivi. Rien n'est deviné — l'application
    n'embarque aucune table CIP, donc un code inconnu attend qu'un
    humain désigne le produit, et c'est ce qui garantit que le lien est
    toujours celui que quelqu'un a posé en présentant une boîte.
  - **La courbe compte les jours et non les lignes.** L'axe était
    l'indice de la ligne : un mois sans rien et une après-midi chargée
    occupaient la même largeur, si bien que la courbe montrait l'ordre
    des écritures et jamais le temps. Le seuil la traverse — il était
    calculé dans le plafond de l'échelle sans jamais être dessiné — et
    le jour survolé se lit.
  - **Le compte sur la porte** : « Produits suivis — 3 à compter ». La
    liste de contrôle se lisait en couleur, ligne par ligne, et personne
    ne la totalisait. Le nombre de jours depuis le dernier comptage,
    qui n'atteignait que le papier, se lit sous la souris.
  - **Ce que le registre sait dire et ne disait pas** : à quel rythme un
    produit part, combien de jours le solde tient à ce rythme, et si les
    comptages tombent juste. Les manques et les excédents sont comptés
    **séparément et jamais en net** — un comptage à −3 et un à +3 ne
    font pas « rien », ils font deux comptages inexpliqués.
- **Les trous de l'ordonnancier se voient.** Le commentaire de cette vue
  affirmait depuis toujours que « c'est la seule vue où le manque d'un
  numéro se voit », et rien ne le marquait : les lignes s'affichaient
  triées et il fallait que le lecteur remarque le saut sur deux cents
  numéros. Une délivrance annulée garde le sien et ne fait pas un trou —
  c'est ce qu'un détecteur naïf rate, et le rater ferait ressembler
  chaque correction à une dissimulation.
- **Le registre d'un produit s'imprime.** Deux documents existaient : la
  liste d'inventaire et l'ordonnancier de l'année. Celui-ci manquait, et
  c'est la page qu'une inspection demande.
- **Un onglet « Vigilance ».** Voir plus bas : le module était écrit,
  l'écran le montre. Les questions d'un côté, ce que l'officine a
  délivré sous chaque nom de l'autre — **trié par nom et non par
  volume**, parce qu'un tableau trié par volume est un classement de
  suspects.
- **Une pièce numérisée peut justifier une ligne du registre.** Une
  inspection demande la ligne *et* l'ordonnance qui la porte ; les deux
  existaient sans se connaître, et les rapprocher se faisait à la main
  sur une date et un nom de fichier.

### Fixed
- **`[stock] count_days` était lu et jeté.** Le champ existe dans
  `config.rs`, il est documenté dans le config.toml livré et dans
  CLAUDE.md, sa valeur par défaut est trente — et la session en portait
  trente en dur, sans jamais regarder la configuration. Le délai que
  l'officine se donne entre deux comptages ne se donnait pas : le
  modifier ne changeait rien à la liste de contrôle.
- **Le grossiste d'une réception repartait sur la délivrance
  suivante.** Le formulaire écrivait `prescriber`, `supplier` et
  `reference` sur toutes les natures, et n'effaçait après écriture que
  la quantité, la date, l'observation et le bon de livraison : on
  saisissait une réception chez CERP, on passait à Délivrance, et la
  délivrance partait au registre avec « CERP » à côté du nom du médecin.
  - Corrigé **à l'écriture** et pas seulement dans le formulaire qui le
    proposait. La base appliquait déjà cette discipline au dossier et à
    la ligne annulée — une réception n'a pas de patient — et elle
    l'applique maintenant aux trois autres champs : le prescripteur
    n'appartient qu'à une délivrance, le grossiste et le bon de
    livraison qu'à une réception. Une ligne fausse dans un registre
    inaltérable ne se corrige que par une annulation ; elle se refuse
    donc avant d'être écrite, et pour tous les appelants.
  - Le formulaire efface en plus ce que la nature choisie ne porte pas,
    au changement de nature comme après l'écriture.
- **Ctrl+F quittait le registre.** La touche cherche d'abord la
  recherche de la liste ouverte ; le registre n'était pas dans cette
  liste, alors elle tombait dans la dernière branche, basculait sur
  l'écran de recherche et refermait le dossier au passage. `stup_body`
  consommait pourtant `focus_list_search` depuis le premier jour :
  personne ne l'y posait jamais.
- **Une quantité illisible n'est plus zéro.** « 1,,5 » ou « 1O » se
  lisaient comme zéro, la glissière retombait à zéro, et le seul retour
  était le refus générique de la base — « la quantité doit être
  positive » — qui ne disait pas ce qui n'allait pas. Le vide et
  l'illisible se distinguent, et chacun le dit.
- Le message du registre s'efface en changeant d'onglet : c'est un seul
  emplacement pour les deux onglets qui l'affichent, et une erreur levée
  sur l'ordonnancier attendait encore sur les stupéfiants, où elle ne
  voulait plus rien dire.
- Renommer un produit suivi ne pouvait pas créer de doublon : le refus
  du même libellé existait à la création et pas au renommage, un
  contrôle sur la moitié des chemins qui y mènent. Deux produits du même
  nom se partageraient un registre en se croyant distincts.

### Groundwork
- **Un calendrier au lieu de trois.** `ordonnancier` comptait les jours
  par la formule julienne, `location` par la formule civile — deux
  arithmétiques grégoriennes pour la même soustraction. Aucune n'était
  fausse ; c'est exactement la situation que la maison refuse ailleurs,
  parce que deux mesures d'une même chose finissent par diverger et que
  celle qui divergera est celle que personne ne relit. `src/date.rs` les
  remplace, la paire civile étant gardée parce qu'elle a un inverse —
  ce dont la lecture d'une péremption aura besoin. Un test parcourt
  chaque jour de 1900 à 2100 et vérifie que l'aller et le retour se
  répondent ; le compte final n'est juste que si 1900 n'est pas
  bissextile et 2000 l'est.
  - `surveillance::months_between` reste à part, à dessein : il ne
    partage aucune arithmétique (des mois de calendrier, pas des jours)
    et sa lecture est volontairement plus stricte.
- **`src/vigilance.rs` : les trois questions que le registre pose.**
  Chaque délivrance porte depuis toujours son dossier, son prescripteur,
  son jour et sa quantité, et rien ne les interrogeait ensemble — alors
  que les monographies nomment les mêmes trois signaux page après page.
  Pur, testé, sans vue pour l'instant.
  - **Ce que le registre ne sait pas est écrit avant tout le reste** :
    ni dose quotidienne, ni durée prescrite, ni à quelle ordonnance une
    ligne se rattache. Donc « aurait dû durer jusqu'au » ne se calcule
    pas, et l'approximation qui vient à l'esprit — la quantité divisée
    par le plafond — est fausse dans le sens dangereux : le plafond est
    légal et non posologique, si bien qu'une ordonnance de sept jours
    sous un plafond de vingt-huit ferait de **chaque délivrance
    légitime** un signalement. Une règle qui crie au loup est une règle
    qu'on désactive.
  - Ce qui se sait, c'est le dossier contre lui-même : deux silences
    indépendants, tous deux à franchir. Le plafond, qui ne sert **qu'à
    se taire**, et la médiane des intervalles antérieurs de ce dossier.
    Sous trois délivrances antérieures il n'y a pas de médiane, donc pas
    de question — le module refuse de parler d'un dossier qu'il vient de
    rencontrer.
  - **Une question, jamais un verdict, et le type l'impose** : pas de
    phrase, pas de gravité, pas de score, donc nulle part où écrire une
    conclusion. L'accès s'appelle `question_key`, un test exige que
    chacune **finisse par un point d'interrogation**, une question ne
    peut pas exister sans au moins deux lignes du registre qui la
    posent, et le dossier n'est qu'un numéro.
  - Le tableau des prescripteurs est rendu **par nom et non par
    volume** : un tableau trié par volume est un classement de suspects,
    et il ne dit de toute façon rien du prescripteur — un médecin de
    soins palliatifs y sera en tête, correctement et innocemment.
  - Écartés délibérément, et dits dans le module : tout score de
    mésusage attaché à une personne, toute inférence d'une dose
    quotidienne, et l'équivalent morphine — le plus séduisant et le pire,
    puisque le registre ne connaît aucune dose quotidienne et qu'un
    « équivalent délivré » se lirait comme une dose.
- **Le libellé d'un produit suivi peut se corriger, et le changement
  s'inscrit** — côté base seulement pour l'instant : le champ arrive
  avec la refonte du volet, où la bande du haut est mesurée et
  plafonnée. Un quatrième contrôle sur la rangée actuelle la fait passer
  à deux lignes à 1024x700, et le registre y tombe à une ligne coupée ;
  la capture le montre, `smoke.sh` ne le voyait pas.
  - Le renommage ne dément pas l'inaltérabilité, et c'est la seule
    raison pour laquelle il est permis : **une ligne du registre désigne
    son produit par son identifiant et jamais par une chaîne**, donc
    rien de ce qui a été délivré ne bouge. Le test le vérifie plutôt que
    de l'affirmer — il renomme un produit qui porte déjà une ligne, puis
    relit la ligne et le solde.
  - Une page imprimée avant la correction porte l'ancien libellé, et
    rien n'expliquerait pourquoi elle ne ressemble pas à l'écran. D'où
    `stup_labels` : ce que le produit s'appelait, ce qu'il s'appelle,
    qui l'a fait et quand — écrit **dans la transaction qui renomme**,
    parce qu'un renommage sans sa trace, ou une trace sans son
    renommage, seraient l'un et l'autre pires que rien.

## [0.146.0] - 2026-08-31

### Changed
- **Les pièces numérisées : le volet refait.** Trois défauts, dont deux
  se voyaient et un se serait vu à la dixième pièce rangée.
  - **La liste n'était pas une liste.** Chaque ligne se dessinait dans
    son propre `horizontal`, donc chaque colonne commençait où la
    précédente avait fini : « Ordonnance » et « Biologie » n'ont pas la
    même largeur, si bien que les dates ne tombaient pas l'une sous
    l'autre, ni les tailles, ni les boutons. Cela se lit comme une pile
    de bouts de phrases, et le défaut empire avec chaque pièce ajoutée.
    C'est une `Grid` maintenant, avec ses en-têtes — genre, date,
    libellé, taille — et elle défile aussi latéralement, parce qu'une
    ligne finit par des boutons et qu'un bouton tombé hors du volet est
    un bouton que personne ne presse.
  - **Le formulaire cachait son propre bouton.** Sur l'onglet d'un
    dossier à 1024x700 — où le bandeau du patient prend déjà la moitié
    de l'espace — la bande plafonnée à la moitié du volet ne montrait
    que les six pastilles de genre : le champ « Libellé » et
    « Importer… » étaient sous la ligne de flottaison d'une zone
    défilante que rien ne signale. Le genre a une valeur par défaut ; le
    libellé et le fichier, non. Ce qui doit rester visible est donc ce
    qui est carvé, et ce sont les pastilles qui défilent dans ce qui
    reste. Les champs et les boutons tiennent désormais sur **une**
    rangée, ce qui rend cinquante pixels à une liste qui n'en avait que
    cent.
  - **« Pièces au dossier » sur le volet de l'officine** : il n'y a pas
    de dossier, ce sont les pièces de l'officine. Le titre le dit.
  - Et la mesure a été refaite : la hauteur de la bande, son partage et
    la largeur du champ sortaient de trois calculs voisins qui ne
    tombaient pas d'accord — la mesure prenait `rect.width()` quand le
    dessin prenait `ui.available_width()`, plus étroit de la marge du
    panneau, et déclarait deux rangées là où il n'en dessinait qu'une.
    Cent pixels réservés pour rien sur un volet qui en compte deux cent
    cinquante-cinq. Les largeurs sont calculées une fois et le dessin
    les reçoit ; c'est la seule façon que les deux ne divergent pas.

## [0.145.1] - 2026-08-31

### Fixed
- **Un thème caché sortait sur le papier.** Le choix rapide arme un
  thème pour l'acte suivant ; si celui-ci était un TROD ou — depuis la
  0.145 — un rendez-vous de prévention, le thème était écrit quand même.
  `has_theme` le cachait à l'écran, ce qui est le plus mauvais des deux
  mondes : invisible là où on aurait pu le corriger, et pourtant imprimé
  par `{{THEME}}` sur la fiche **et** sur le courrier au médecin, et
  exporté dans la colonne « Thème » du CSV. Un champ que rien ne montre
  et que tout ressort n'est jamais relu.
  - Corrigé à la source — un acte dont la nature ne porte pas de thème
    n'en garde pas — et **défendu aux trois sorties**, pour que les
    lignes déjà écrites dans les bases installées cessent de l'imprimer
    sans qu'on ait à réécrire quoi que ce soit dans la base.
  - `an_act_without_a_thematic_never_stores_one` passe les dix natures
    d'acte et vérifie les deux moitiés : celles qui portent un thème le
    gardent, les trois autres non. La liste des trois est vérifiée elle
    aussi — un test qui ne trouverait aucune nature sans thème passerait
    tout aussi bien.
- La colonne « Durée (min) » du CSV n'écrit plus « 0 » pour un TROD :
  il est chronométré par la bandelette, et zéro minute y affirmait un
  acte qui n'a pas eu lieu.

## [0.145.0] - 2026-08-31

### Added
- **Une fenêtre d'export avant chaque fiche et chaque courrier.** Les
  deux boutons imprimaient sans rien demander : la fiche portait la
  liste de son thème, le courrier un cadre vide, et l'officine n'avait
  pas voix au chapitre. C'est pourtant elle qui sait ce que *ce*
  rendez-vous-là a couvert.
  - Deux moitiés, et la seconde compte plus que la première : ce qu'on
    **coche** dans ce qui est proposé, et ce qu'on **ajoute** à la main,
    une ligne par point. Aucune liste livrée ne couvre ce qu'un entretien
    a réellement abordé, et une case à cocher de plus ne l'aurait pas
    couvert non plus.
  - Ce qui est proposé vient du thème quand il y en a un, **coché
    d'avance** : c'est ce que la feuille portait déjà, et l'export ne
    doit pas obliger à recocher sept cases pour retrouver l'existant.
  - Le courrier au médecin reçoit un marqueur `{{POINTS}}`. Sans point
    coché il garde l'encadré vide qu'il portait avant — imprimer une
    liste que personne n'a choisie ferait dire au pharmacien ce qu'il
    n'a pas dit. Avec des points, la liste s'imprime et l'encadré reste
    dessous, plus court : le médecin y répond, et c'est la moitié de
    l'intérêt d'envoyer la feuille.

### Changed
- **Le rendez-vous de prévention n'a plus de thème.** On en couvre
  plusieurs dans la même séance — vaccinations, sommeil, alimentation —
  et en stamper un sur l'acte nommait le moindre en cachant les autres.
  `InterviewKind::has_theme` l'exclut désormais, comme il excluait déjà
  les TROD, mais pour la raison inverse : le TROD n'a pas de sujet, le
  rendez-vous de prévention en a trop pour un seul champ.
  - Ce qu'il couvre se choisit à l'impression, dans `[prevention]
    subjects` — la grille de « Mon bilan prévention », livrée remplie et
    **modifiable**. C'est une liste de sujets et non un tarif : elle ne
    sera pas fausse dans l'année, et une officine qui organise ses
    rendez-vous autrement doit pouvoir la réécrire. Les cases y sont
    **décochées** d'avance : sur ce rendez-vous-là, choisir *est* la
    question.
- « Rangé » devient « Archivé », et « Ranger une pièce » « Archiver une
  pièce ». *Ranger* est le mot des rayons, pas celui d'un système de
  pièces : sur un bouton à bascule, l'adjectif seul ne disait pas ce qui
  était rangé ni pourquoi, et toute sa charge tenait dans l'infobulle —
  ce qui est l'inverse de ce qu'une étiquette doit faire.

## [0.144.0] - 2026-08-31

### Added
- **Les classes thérapeutiques deviennent un référentiel, et une vue.**
  Le champ « classe » d'une fiche est du texte libre, et il avait dérivé
  : **495 libellés distincts pour 851 fiches, dont 331 sur une seule
  fiche**. Ce n'est plus une classification, c'est une étiquette.
  - **La dérive n'était pas cosmétique.** `anti-TNF` et `anti-TNF alpha`
    étaient deux classes : la pastille de Humira annonçait sept voisins
    au lieu de dix, et Remicade n'était nulle part. La question derrière
    est celle du comptoir un jour de rupture — *qu'est-ce qu'il y a
    d'autre dans cette classe* — et une réponse incomplète y est pire
    qu'une absence de réponse, parce que rien n'a l'air cassé. Deux
    autres du même genre : `bêtabloquant` / `bêta-bloquant`, un trait
    d'union ; `biphosphonate` / `bisphosphonate`, une lettre.
  - `src/classes.rs` porte **seize familles et 383 classes canoniques**,
    chacune avec les libellés qu'on rencontre réellement dans les
    fiches. `canonical` les y ramène, `same` est ce sur quoi l'anneau
    « même classe » et la pastille comparent désormais — et non plus la
    chaîne brute.
  - **Le référentiel lit les fiches, il ne les réécrit pas.** Réécrire
    la classe de 851 fiches écraserait ce que l'équipe a écrit, ce que
    cette maison ne fait nulle part. Une classe inconnue reste lisible
    et se range sous « hors référentiel », où elle se **voit** : une
    fiche qu'aucune colonne n'affiche est une fiche perdue, et c'est
    exactement le défaut qu'on corrige.
  - **Une vue « Classes… »** depuis la base médicaments ou Ctrl+K :
    l'appareil, puis la classe, puis les fiches. C'est le seul ordre qui
    rende 383 classes consultables — à plat elles sont une liste qu'on
    referme, ce qu'étaient déjà les 495 libellés. Chaque ligne dit son
    compte avant qu'on clique, et une classe qui replie plusieurs
    graphies le dit en toutes lettres, sinon trouver Remicade sous
    « anti-TNF alpha » ressemblerait à une erreur.
  - **Quatre tests le tiennent** : chaque classe écrite sur une fiche
    livrée est dans le référentiel ; un libellé ne désigne qu'une classe
    (un alias qui serait le nom canonique d'une autre ferait tomber la
    fiche dans l'une ou l'autre selon l'ordre de la table — juste, puis
    faux, sans que rien n'ait changé) ; les trois dérives mesurées se
    replient ; et le référentiel porte **au moins 112 classes de moins**
    que la base n'écrit de libellés, sans quoi il n'aurait rien replié
    et serait la même liste avec une colonne de plus.
  - Le relevé a d'abord été fait à l'expression régulière et donnait 274
    libellés sur 528 fiches. Elle ne voyait que les entrées tenant sur
    une ligne, et la moitié de la base en fait cinq — **le piège est
    écrit dans le journal de la v0.109 et j'y suis retombé**. C'est le
    test qui l'a dit : 268 fiches hors référentiel d'un coup.

### Fixed
- **Un titre de panneau se peignait hors de son panneau.** `motif::panel`
  posait sa légende avec `layout_no_wrap` et sans limite de largeur :
  « PATIENTS SOUS CE TRAITEMENT » sur un volet étroit débordait la
  gouttière et se peignait sur le panneau d'à côté. Rien ne l'arrêtait —
  un `Painter` peint où on lui dit — et c'est la même famille que
  `ui.columns` qui ne découpe pas et que `list_row` qui jetait son
  `RichText` : le débordement ne casse rien, il se lit faux. Élidé sur
  une ligne désormais, dans le seul endroit qui dessine les légendes,
  donc partout à la fois.
- **L'onglet Conciliation n'affichait plus une seule divergence** à
  1024x700 les deux docks ouverts. Mesuré : la tête du panneau coûtait
  136 px, la bande de saisie en réclamait 110 au titre de son plancher,
  et il restait **un pixel** pour le tableau — c'est-à-dire pour ce que
  l'onglet existe pour montrer.
  - La cause n'était pas le partage mais la tête : le compte
    « 3 divergence(s) sur 5 ligne(s) comparée(s) » fait deux cent
    cinquante pixels, plus que le plus large des trois boutons, et
    c'est lui qui faisait passer la rangée à deux lignes. Sur sa propre
    ligne sous le titre il coûte seize pixels au lieu de quarante-deux,
    et il se lit mieux : un compte n'est pas une commande.
  - Et la gouttière de huit pixels n'était pas comptée dans la réserve,
    ce qui retirait encore une ligne au tableau.
- **`motif::list_row_count`**, la rangée à chiffre calé à droite. Ajoutée
  parce que la liste des classes en avait besoin et que la faire à la
  main l'aurait cassée : un compte ajouté après le libellé est la *fin*
  de la ligne, donc la première chose que l'élision mange —
  « Cardiologie et vaisseaux · … » est une ligne dont le seul contenu
  chiffré a disparu. Le libellé est mesuré après le chiffre et la rangée
  grandit avec lui : une classe peut s'appeler « antagoniste non
  stéroïdien des récepteurs minéralocorticoïdes », et allouer une ligne
  pour en peindre deux centrerait le texte hors de sa propre rangée.

## [0.143.1] - 2026-08-30

### Fixed
- **La recherche du registre ne trouvait rien.** `fuzzy::contains_folded`
  attend un foin **déjà plié** par `sort_key` — c'est ce qui lui permet
  de ne rien allouer pour les moteurs de règles, qui la interrogent des
  milliers de fois. Les trois recherches du registre lui passaient un
  libellé pris tel quel sur la ligne : l'aiguille pliée était alors
  comparée à un « S » brut, et « Skenan » ne trouvait pas « Skenan ». Un
  champ de recherche qui a l'air d'un champ de recherche et ne rend
  jamais rien est pire qu'une absence de recherche — on ne sait pas si
  la base est vide ou si l'écran est cassé.
  - `fuzzy::contains_loose` répond à la même question sans qu'aucun des
    deux côtés soit plié, et sans allouer non plus : plier le foin
    d'abord aurait donné la bonne réponse et une `String` par ligne et
    par image, ce que cette maison interdit dans un chemin de dessin.
  - Le piège est maintenant montré plutôt que décrit :
    `the_loose_search_folds_both_sides_and_the_folded_one_does_not`
    commence par affirmer que `contains_folded("Skenan LP 30 mg",
    "skenan")` est **faux**, puis que la nouvelle est vraie. La
    troisième correspondance rattachait aussi la fiche du médicament au
    produit suivi, et tombait de la même façon.

## [0.143.0] - 2026-08-30

### Added
- **Le registre des stupéfiants prend son propre fichier, son catalogue
  et sa façon de se corriger.** C'était la dernière ligne de la feuille
  de route, et elle en demandait sept.
  - **`<base>_stups.db`, à côté de la base.** Pas pour la taille : dix
    ans de registre pèsent quelques centaines de kilo-octets. Parce que
    ce n'est pas la même nature de chose. La base est un outil de
    travail — on la réinitialise, on la ressème, on la compacte, on en
    essaie une copie, on la recopie d'un poste à l'autre. Le registre
    est une **pièce comptable**, inaltérable, que R. 5132-36 demande de
    conserver dix ans à compter de sa dernière mention, et qu'un
    contrôle demande *seule*, sans les dossiers des patients, qui ne le
    regardent pas. Séparé, l'année close se range en copiant un fichier,
    et rien de ce que l'application fait à sa base ne peut l'atteindre.
    Même SQLCipher, même clé : pas une deuxième cryptographie écrite à
    la main.
  - Ce que la séparation impose est tenu, et chaque point a son test :
    `change_password` rechiffre **trois** fichiers, « Copier la base… »
    en copie trois, la copie quotidienne du registre suit le plus grand
    des deux autres comptes — il ne pèse rien, et c'est le seul fichier
    de la maison qu'on ne peut pas reconstituer —, et Options › Base dit
    combien de lignes il porte, sur combien de produits, et de quand à
    quand.
  - **Un registre écrit par une version d'avant est déplacé au premier
    lancement, identifiants compris.** C'est le point qui comptait : une
    ligne de délivrance porte un **numéro de dossier** et un produit
    l'identifiant de sa fiche médicament ; les renuméroter au passage
    les ferait désigner un autre patient et un autre médicament, ce
    qu'un registre ne peut pas se permettre. Le déplacement est marqué
    dans `seed_state` et non déduit de « le fichier est vide » — qui le
    rejouerait le jour où quelqu'un supprime le fichier, et le rejouerait
    sur des lignes écrites depuis. Et les lignes d'origine restent où
    elles sont : on ne supprime pas un registre, même pour le ranger
    ailleurs.
  - **Un catalogue de 106 présentations en 12 familles**, avec le
    dosage, l'unité de comptage, la durée maximale de prescription et la
    règle de la famille : morphine LP et à libération immédiate,
    opioïdes injectables, oxycodone, hydromorphone, fentanyl
    transdermique et transmuqueux, méthadone gélule et sirop,
    méthylphénidate, oxybate de sodium, buprénorphine haut dosage. Un
    clic suffit à suivre un produit, et tout arrive avec le nom —
    personne ne tape « gélue ».
  - **Le catalogue est une table de règles, pas un contenu semé.** Une
    base livrée avec 106 produits suivis, ce serait 106 soldes à zéro,
    106 « jamais compté » sur la liste de contrôle, et un écran qu'on
    n'ouvre plus. On y choisit. Et il ne ferme rien : quand la recherche
    ne trouve pas, la dernière ligne propose d'inscrire ce qui a été
    tapé.
  - **Le régime est un champ, parce que tout ne s'inscrit pas.** La
    buprénorphine haut dosage relève de la réglementation des
    stupéfiants pour la prescription et la délivrance — ordonnance
    sécurisée, 28 jours, fractionnement par 7 — mais **pas** pour la
    comptabilité : rien n'oblige à l'inscrire au registre, et beaucoup
    d'officines le font quand même. La ranger à côté de la morphine sans
    le dire aurait enseigné une obligation qui n'existe pas. Un test
    exige de toute famille « assimilé » qu'elle le dise dans sa note.
  - **La durée maximale est le nombre sur lequel on refuse une
    ordonnance**, et c'est pour cela qu'elle est sur la fiche du produit
    et pas dans une note générale : 28 jours pour presque tout, 14 pour
    le sirop de méthadone, **7 pour la voie parentérale**. Un test
    refuse toute autre valeur dans la table — un chiffre rond inventé
    là-dedans se paierait au comptoir.
  - **La saisie va vite.** Un clic sur le produit, une glissière Motif
    pour la quantité — la première du projet : cuvette en creux, curseur
    en relief, rien d'arrondi — et six pastilles pour ce qui revient
    (1, 2, 3, 7, 14, 28, qui sont ce que porte une boîte et ce que porte
    une délivrance fractionnée). Le champ reste **maître** : une
    glissière ne sait pas écrire « 2,5 », et un patch se coupe en deux
    moins souvent qu'on ne le croit mais cela arrive.
  - **Un onglet « Ordonnancier »**, qui n'est pas l'écran du stock. À
    gauche la suite des délivrances de l'année, tous produits confondus,
    dans l'ordre de leurs numéros — c'est le document qu'un contrôle
    demande, et la seule vue où le manque d'un numéro se voit ; il
    s'imprime en A4 paysage, annulations comprises. À droite le journal
    des quarante dernières lignes, réceptions comprises, du plus récent
    au plus ancien : c'est là qu'on retrouve ce qu'on vient d'écrire.
    L'un répond à « qu'a-t-on écrit », l'autre écran à « combien en
    reste-t-il », et les mélanger obligerait à trier deux fois la même
    liste dans deux ordres contraires.

### Changed
- **Corriger une ligne, c'est l'annuler — et c'est la seule façon.** La
  demande disait « les lignes peuvent être modifiées, mais que tout
  reste traçable », et c'est le seul point où les deux tiennent
  ensemble. Une ligne `ANNULATION` désigne la ligne fautive, porte un
  motif obligatoire, et défait **exactement** ce que celle-ci avait fait
  au stock — lu sur la ligne annulée et jamais sur l'annulation, dont la
  quantité n'est même pas regardée.
  - C'est là qu'est toute la différence avec une correction écrite à la
    main. Une ligne de sens contraire dont on tape la quantité rend ce
    qu'on *croit* avoir pris ; une annulation rend ce qui a été pris. Les
    deux diffèrent précisément le jour où c'est la quantité qui était
    fausse, c'est-à-dire le jour où l'on corrige.
  - Annuler un inventaire lui rend son `expected` : un inventaire *pose*
    le solde, il n'y a rien à en soustraire, et le seul nombre qui
    permette de le défaire est celui que le registre affichait au moment
    du comptage. C'est la seule raison pour laquelle cette colonne est
    écrite dans la base au lieu d'être recalculée à la lecture — un
    recalcul d'aujourd'hui donnerait le solde d'aujourd'hui.
  - Quatre refus, chacun avec sa raison : pas de motif, pas
    d'annulation ; une annulation ne s'annule pas (la correction d'une
    annulation fautive est une ligne qui l'explique, pas une troisième
    couche) ; deux fois la même annulation ferait remonter le stock du
    double, et le refus est **dans la transaction** parce que deux postes
    peuvent la demander en même temps ; et une ligne d'un autre produit
    ne s'annule pas depuis celui-ci, le stock qui remonterait n'étant pas
    le sien.
  - **Le numéro d'ordonnancier de la délivrance annulée ne revient
    pas.** Il reste pris, la suite continue après lui, et le trou se
    lit. Un numéro servi deux fois, lui, ne se lit pas.
  - Une ligne annulée reste à l'écran et sur le papier, **barrée**, avec
    « annulée » à côté et le motif sur la ligne d'en face. Un registre
    d'où l'on aurait ôté les erreurs ne prouverait rien : ce qu'un
    contrôle doit voir, c'est la faute *et* la correction qui la nomme.
- La colonne de gauche du registre bascule entre les produits suivis et
  le catalogue, avec une recherche sur les deux. Les trois champs qui
  servaient à inscrire un produit à la main ont quitté la bande du
  haut — c'est là qu'on choisit ce qu'on suit —, ce qui rend une rangée
  au registre. L'unité de comptage s'y corrige aussi, à côté du seuil :
  un produit inscrit à la main n'en a pas, et un solde sans unité ne dit
  pas s'il s'agit de boîtes ou de gélules.
- Les trois listes qui montrent des lignes du registre — le registre
  d'un produit, l'ordonnancier, le journal — dessinent la **même**
  fonction. Une ligne de registre qui ne se lirait pas pareil selon
  l'écran est une ligne dont on doute, et c'est la seule chose qu'un
  registre ne peut pas se permettre. Ce qu'un clic demande remonte hors
  de la boucle d'affichage ; « qui annule cette ligne » et « quel produit
  porte ce numéro » sont deux mémos refaits à l'écriture, jamais soixante
  fois par seconde sur quarante lignes.

## [0.142.0] - 2026-08-30

### Added
- **Un explorateur de facettes, et les données qui le nourrissent.** Les
  851 monographies répondent toutes à la même question, « que sais-je de
  ce médicament » ; il manquait l'inverse, « quels médicaments ont telle
  propriété » — la plus longue demi-vie, ce qui pèse sur la thyroïde. Un
  paragraphe ne répond pas à ça : on ne trie pas des phrases.
  - `src/facets.rs` porte la **demi-vie plasmatique des 851 fiches**, en
    heures : 658 chiffrées, 193 explicitement non chiffrées avec leur
    raison. Plus `IMPACTS`, **2 073 lignes sur douze axes d'organe
    couvrant 733 des 851 fiches**, chacune avec un **sens** (`Traite` /
    `Altere`), un **degré** et la clause qui le justifie : neurologique
    288, cœur 274, foie 265, œil 208, digestif 207, peau 187, moelle
    186, poumon 185, rein 138, os 84, oreille 32, thyroïde 19 — dont
    1 674 « altère » et 399 « traite ».
  - **Le sens et le degré sont ce qui distingue ces données d'un comptage
    de mots.** Compter « thyroïde » dans les monographies met Levothyrox
    et Euthyrox en tête des médicaments qui touchent la thyroïde — ils la
    remplacent — et laisse l'amiodarone derrière la Betadine. La bonne
    réponse à « qu'est-ce qui abîme le plus la thyroïde » est Cordarone,
    et il faut un champ pour le dire.
  - **Ce qu'une extraction automatique aurait produit était faux, et faux
    en tête.** Lue au premier nombre venu, la prose donne un classement
    dont les vingt premiers sont majoritairement erronés : « demi-vie
    d'environ 5 jours, l'effet persistant plusieurs semaines » devient
    quatre-vingt-quatre jours ; la demi-vie *osseuse* d'un bisphosphonate
    met Fosamax en tête à dix ans ; le stérilet met sa durée de
    libération, quatre ans, à la place des vingt-quatre heures du
    lévonorgestrel ; « 5,8 jours » se lit « 5 à 8 ». Les tables ont donc
    été **amorcées** par un analyseur puis relues à la main, et les
    trente-huit fiches dont le chiffre venait d'une lecture (« quelques
    heures », « plusieurs mois ») et non de la fiche sont marquées non
    chiffrées plutôt que devinées — c'est ce qui met enfin Cordarone
    (20 à 100 jours) en tête, à sa place.
  - **Une facette est adossée à ce que la fiche écrit** : là où la
    monographie ne donne pas de nombre, la facette le dit. On corrige une
    facette en corrigeant la fiche, et non deux contenus qui dérivent.
    C'est désormais un test et non une intention :
    `every_impact_is_backed_by_its_own_card` relit chaque monographie et
    exige que le vocabulaire de l'organe y figure. Il a trouvé des
    impacts écrits de mémoire — deux sur la thyroïde, quatre sur l'œil
    où « trouble visuel » désignait le signe d'alerte d'une thrombose
    sous contraceptif et non une lésion oculaire — et il les a fait
    retirer.
  - **Un impact décrit ce que le médicament fait, jamais le terrain qui
    l'interdit.** La même phrase change de sens selon le champ qui la
    porte : « insuffisance rénale » dans les contre-indications décrit un
    rein déjà malade dont dépend la dose, dans les effets indésirables un
    rein que le médicament abîme. Le premier n'est pas un impact. Sur
    l'axe rein, presque tout le « surveiller la créatininémie » du
    référentiel tombe pour cette raison, et ce qui reste — insuffisance
    rénale fonctionnelle sous diurétique, néphrite interstitielle sous
    céphalosporine, lithiase sous topiramate — est ce qu'on voulait.
  - **La règle a été repassée sur les lignes déjà écrites, et 212
    étaient dans ce cas.** L'axe digestif s'était rempli de « antécédent
    d'ulcère gastro-duodénal évolutif » sous AINS — un estomac déjà
    malade qui interdit le médicament, alors que l'impact existe et est
    écrit deux champs plus bas. 106 clauses ont été réécrites depuis le
    bon champ, 35 lignes retirées faute d'y trouver quoi que ce soit, et
    deux disaient l'inverse de ce qu'on leur faisait dire : Spasfon et
    Duspatalin précisent qu'ils n'ont *pas* de contre-indication au
    glaucome, et un mot-clé y avait lu une atteinte oculaire. Au passage,
    treize hémorragies digestives sous anticoagulant ou AINS étaient
    classées « mineur » : c'est ce qui tue dans ces classes, et le degré
    le dit maintenant. `an_impact_describes_what_the_drug_does_not_who_may_not_take_it`
    tient la règle — et elle vaut pour les deux façons d'écrire autre
    chose qu'un effet, car la seconde était plus répandue encore :
    cinquante-huit clauses commençaient par le *geste* de surveillance
    (« transaminases avant l'instauration puis périodiquement »), qui dit
    ce qu'on fait et jamais ce que le médicament fait. Quarante-sept ont
    été redessinées depuis les effets indésirables de la fiche, onze
    retirées : celles-là ne disaient rien du foie ailleurs que dans leur
    consigne de prise de sang.
  - **Le versant « traite » passe de 64 lignes à 399, et les douze axes
    sont peuplés.** Il était resté une esquisse pendant que « altère » se
    remplissait, avec deux axes à **zéro** — la peau et le digestif, là
    où l'officine pose la question le plus souvent : « qu'est-ce qu'on a
    pour le psoriasis, pour l'acné, pour le reflux ». Le grade n'y veut
    pas dire la même chose : du côté « altère » il dit la gravité, du
    côté « traite » il dit la **centralité**. Majeur quand l'organe est
    la raison d'être du produit, mineur quand il ne fait que le protéger
    de loin — sans quoi soixante antihypertenseurs noieraient les six
    médicaments de l'insuffisance cardiaque sur l'axe du cœur.
    `every_treatment_is_backed_by_its_own_indication` relit chaque ligne
    sur le vocabulaire de l'indication, qui n'est pas celui de la
    lésion : la lévothyroxine ne dit jamais « dysthyroïdie », elle dit
    « hypothyroïdie », et l'inhalateur dit « asthme » et non
    « bronchospasme ». Sans cette seconde table de mots, les 399 lignes
    n'auraient été relues par rien.
  - **La négation est le piège que le mot-clé ne voit jamais**, et il
    s'est présenté cinq fois : le fondaparinux dont la fiche dit qu'il ne
    provoque *pas* de thrombopénie induite par l'héparine ; l'étanercept
    dont trois fiches disent qu'il n'a *pas* de place dans la maladie de
    Crohn ; Vastarel, dont les indications ORL et vertigineuses ont été
    *supprimées* ; Spasfon et Duspatalin, qui précisent qu'ils n'ont
    *pas* de contre-indication au glaucome. Toutes auraient été publiées
    à l'envers par une table produite à la machine.
  - **Une fiche peut traiter un organe et l'abîmer**, et c'est la ligne
    la plus utile de la table : le bisphosphonate traite l'os et nécrose
    la mâchoire, l'anti-VEGF sauve la rétine et la décolle parfois en y
    entrant, le bêta-2 de secours lève le bronchospasme et en provoque
    parfois un. **178 fiches sont dans ce cas** — c'était vingt-trois
    avant que le versant « traite » soit rempli, et cette croissance est
    l'argument le plus net pour l'avoir fait : la moitié de ce que la
    table dit d'intéressant est dans la superposition des deux versants,
    pas dans l'un ou dans l'autre.
  - **« Ce qui dure au-delà » passe de 37 entrées à 91.** C'est le
    troisième axe et il se comptait moins bien que les deux autres :
    cinquante-quatre fiches disaient en toutes lettres que leur effet
    survit à leur demi-vie sans qu'aucune facette le note — l'inhibition
    plaquettaire irréversible qui dure le renouvellement des plaquettes,
    le métabolite intracellulaire d'un antirétroviral, la terbinafine
    dans la kératine, la charge iodée de la Betadine, les métabolites
    actifs du diazépam chez le sujet âgé. La question derrière est
    toujours la même au comptoir : *quand puis-je opérer, relayer,
    arrêter*, et la demi-vie plasmatique seule y répond faux. Un
    plancher tient maintenant ce compte comme les autres.
  - **La monographie nomme ses axes.** La fiche médicament porte une
    section « Retentissement organique » qui liste ses impacts, le plus
    lourd d'abord. C'est la même donnée lue dans l'autre sens :
    l'explorateur part de l'organe et arrive sur la fiche, la fiche part
    d'elle-même et nomme ses axes — sans quoi on lirait une monographie
    sans savoir que le référentiel sait la ranger.
  - **Le référentiel est indexé une fois, pas soixante fois par
    seconde.** Un classement construit dans la boucle de dessin coûtait,
    à 2 073 impacts, un million et demi de comparaisons de chaînes par
    image — ce que la maison interdit précisément dans un chemin de
    dessin. Les tables sont statiques : l'index se construit au premier
    accès et prête ensuite des tranches.
  - **Deux zones défilantes sans nom dans une même vue se marchent
    dessus.** egui leur dérive le même identifiant et peint « First use
    of ScrollArea ID … / Second use of … » en rouge en travers de
    l'écran. Cela ne provoque aucune panique : `smoke.sh` a ouvert
    l'explorateur deux fois, dans les deux formes, avec les bandeaux
    dessus, et seule la capture d'écran les a montrés. Les deux zones
    portent maintenant un `id_salt`.
  - La vue **Explorateur** trie tout cela : un axe par onglet portant son
    effectif — « Foie · 265 », la première question qu'on pose à un axe,
    et il serait absurde de devoir l'ouvrir pour l'apprendre —, la
    couverture affichée en toutes lettres — un axe partiel ne doit pas se
    lire « aucun médicament ne touche cet organe » —, le degré en couleur,
    et un clic sur un nom qui ouvre la fiche. Deux crochets
    `BPM_CADDY_START_VIEW=explorer|explorer_organ` la mettent sous
    `smoke.sh` et `eyeball.sh`, les deux axes se peignant tout autrement.

### Changed
- **« Toxicité / marge thérapeutique » passe de 65 fiches à 473.** La
  section n'est pas due à toute fiche — un antifongique local n'a pas
  de marge dont parler — mais elle l'était à toutes celles où une dose,
  une durée ou une exposition tue, et elle manquait à la quasi-totalité
  d'entre elles : 160 fiches portaient un antidote nommé dans
  `STARTER_DRUGS` sans une ligne sur ce qui justifie cet antidote. Les
  classes ont été reprises une par une, au comptoir plutôt qu'au RCP :
  - **Ce qui tue par la dose** : opioïdes (14), insulines et
    sulfamides (17), benzodiazépines et hypnotiques (14), digitaliques
    et antiarythmiques, héparines (7).
  - **Ce qui tue par l'interaction** : l'allopurinol ajouté sur une
    azathioprine, la clarithromycine sur une colchicine, le gemfibrozil
    sur un répaglinide, un carbapénème sur un valproate, le miconazole
    en gel buccal sur un sulfamide, l'oméprazole sur un clopidogrel.
  - **Ce qui tue par la voie ou le geste** : le bortézomib et la
    vinorelbine par voie intrathécale, l'Extencilline par voie
    intraveineuse, la ceftriaxone avec du calcium chez le nouveau-né,
    la prométhazine hors de la veine.
  - **Ce qui tue parce que personne n'y pense** : l'acidocétose à
    glycémie normale sous gliflozine, l'insuffisance surrénale à
    l'arrêt d'une corticothérapie, le syndrome akinéto-hyperthermique à
    l'arrêt d'une L-dopa, la réactivation d'une hépatite B à l'arrêt
    d'un antiviral, la LEMP sous natalizumab, l'ingestion de quelques
    comprimés de fer ou de Nivaquine par un enfant.
  - **Ce que le patient ne dira jamais spontanément** : les troubles du
    contrôle des impulsions sous agoniste dopaminergique ou sous
    aripiprazole, l'agueusie de la terbinafine, l'éjaculation
    rétrograde de la silodosine, la gynécomastie de la spironolactone.
- **Les quatre anticoagulants oraux directs avaient le conseil patient
  le plus court de la base.** Même classement que pour les
  biosimilaires, appliqué cette fois au champ « informations utiles au
  patient » : médiane de la base 1 065 caractères, Lixiana 266, Xarelto
  342, Pradaxa 411, Eliquis 457 — les quatre derniers, sur la classe où
  ce que le patient sait décide de ce qui lui arrive. Rien n'y était
  faux, tout y était bref. Les quatre passent à ~1 300 caractères, avec
  ce qui manquait : à quoi sert un médicament qui ne se sent pas et
  pourquoi on l'oublie, ce que coûtent deux jours d'arrêt, la carte
  d'anticoagulant, l'aspirine et les anti-inflammatoires qu'on achète
  sans y penser, le millepertuis, la fatigue et l'essoufflement qui
  révèlent une anémie, et la chute avec choc à la tête où l'on consulte
  même en se sentant bien. Thyrozol, cinquième plus court, a suivi :
  sa consigne d'agranulocytose est une consigne *au patient*. Quatre
  biosimilaires anti-TNF auto-injectés et un IEC ont suivi pour la même
  raison — le geste, la chaîne du froid, le stylo qui change avec la
  marque, la fièvre qui fait décaler l'injection ; et pour l'IEC, la
  toux sèche qu'on explore à tort et l'angio-œdème qui fait appeler le
  15 et ne jamais reprendre la classe.
- **Une fiche décrivait la mauvaise molécule.** La section toxicité de
  **Cardensiel** (bisoprolol) décrivait l'amiodarone : demi-vie de plus
  de cinquante jours, dysthyroïdie, pneumopathie interstitielle, dépôts
  cornéens — sur une fiche dont le champ `half_life`, deux lignes plus
  haut, annonce 10 à 12 heures. Trouvée en confrontant chaque section
  toxicité qui cite une demi-vie au champ `half_life` de sa propre
  fiche ; le même passage a corrigé trois formulations trop lâches
  (Abilify, Gardénal, Daonil). Une recherche de sections copiées d'une
  fiche à l'autre n'aurait rien vu : le texte était paraphrasé.
- **Les biosimilaires avaient des fiches de troisième ordre.** En
  classant les 850 monographies par la taille de leur corps clinique,
  les douze fiches de biosimilaire occupaient dix des douze dernières
  places : médiane de la base 4 688 caractères, Nivestim 921. Les
  champs n'étaient pas vides — le test ne voyait donc rien — mais
  « Hypersensibilité. » tenait lieu de contre-indications à un
  biologique injectable, « Pas d'adaptation. » de conduite rénale, et
  « À n'utiliser qu'en cas de nécessité. » de conduite pendant la
  grossesse, quand le médicament de référence posé à côté portait une
  page entière. C'est pourtant la fiche du biosimilaire que le comptoir
  a en main : le patient a été substitué, il n'ouvrira pas celle de la
  référence.
  - Les douze sont réécrites au niveau de la maison : Nivestim, Zarzio
    et Ziextenzo (filgrastim et pegfilgrastim), Retacrit (époétine),
    Erelzi et Benepali (étanercept), Imraldi, Hyrimoz et Amgevita
    (adalimumab), Semglee (insuline glargine), Terrosa (tériparatide),
    Inhixa (énoxaparine). Le corps clinique passe de 921–2 200 à
    3 000–5 700 caractères.
  - **Où ce texte arrive, et où il n'arrive pas** : `fill_starter_details`
    ne remplit qu'une colonne **vide** (`AND {column} = ''`), pour ne
    jamais réécrire par-dessus l'équipe. Les douze fiches réécrites
    n'étaient pas vides : une base ouverte avant aujourd'hui gardera
    donc les anciens textes, et seules une base neuve et
    « Réinitialiser la base… » verront les nouveaux. Le comportement est
    volontaire et n'a pas été touché ici — le changer pour rattraper les
    bases existantes est une décision sur le semis d'une base clinique
    partagée, pas un effet de bord d'une passe de contenu.
  - Ce qui manquait n'était pas de la longueur : le dépistage de la
    tuberculose et des hépatites avant un anti-TNF, la réactivation de
    l'hépatite B, la thrombopénie induite par l'héparine entre le
    cinquième et le vingt et unième jour, la carence martiale qui fait
    échouer une époétine, les vingt-quatre mois de tériparatide qui sont
    une limite de vie entière et non de cure, les deux concentrations
    d'insuline glargine qui ne se remplacent pas dose pour dose.
- **`the_posology_coverage_only_improves` : le plafond passe de 34 à
  16.** Le commentaire affirmait qu'à 34 le reste était « injecté ou
  perfusé par quelqu'un d'autre ». Ce n'était plus vrai : treize de ces
  classes se délivrent au comptoir. Trente lignes de posologie écrites
  pour le tocilizumab et l'abatacept sous-cutanés, le védolizumab
  d'entretien, le filgrastim, les gonadotrophines de FIV, le protocole
  d'IVG médicamenteuse en deux temps, l'immunoglobuline anti-D, l'oxybate
  de sodium, l'antiémétique de cure et les perfusions de zolédronate. À
  16, l'affirmation redevient vraie et le plafond la reflète.
- **Quatre exemptions de posologie étaient des affirmations fausses.**
  `NO_POSOLOGY` dispense une classe d'avoir une posologie au motif
  qu'elle est « titrée par le spécialiste, ou planifiée ailleurs ».
  Cela ne décrit ni le létrozole à 2,5 mg, ni l'anastrozole à 1 mg, ni
  le fingolimod à 0,5 mg, ni le tofacitinib à 5 mg deux fois par jour :
  une dose, la même pour tout le monde, pendant des années. Et c'est
  précisément sur ces molécules que le conseil de comptoir porte le
  traitement — les arthralgies qui font arrêter un anti-aromatase avant
  la fin, les six heures de surveillance à la reprise du fingolimod
  après une rupture de délivrance, la lymphopénie sous diméthyl
  fumarate qui annonce la LEMP. Vingt et une lignes écrites pour les
  anti-aromatases, les immunomodulateurs de la SEP, le modulateur S1P
  et les inhibiteurs JAK ; les quatre exemptions retirées. Le plafond
  de classes sans posologie reste à 16, et les 751 fiches sur 851 qui
  portent maintenant une posologie ne doivent rien à un assouplissement
  du critère.
- **Sept héparines gagnent leur section « toxicité / marge
  thérapeutique ».** Tous les AOD et tous les AVK en portaient une ; la
  classe posée entre les deux, la seule dont la dose se calcule au poids
  et dont l'antidote n'est que partiel, n'en avait aucune. Héparine
  sodique, Calciparine, Lovenox, Inhixa, Innohep, Fraxiparine, Fragmine.
  Ce qui s'y dit : les deux unités de l'énoxaparine qui se confondent
  sur la même boîte (1 mg = 100 UI anti-Xa), la protamine qui ne
  neutralise qu'environ 60 % de l'activité anti-Xa d'une HBPM contre la
  totalité d'une héparine non fractionnée, la tinzaparine dont la
  non-accumulation est la mieux documentée sans que cela déplace le
  seuil de 30 mL/min, et la daltéparine qui ne se dose qu'en unités et
  jamais en milligrammes. Le cliquet des fiches à section toxicité passe
  de 65 à 72.

### Fixed
- **Coumadine disait deux choses de la dose initiale du sujet âgé** :
  4 mg dans la monographie, « 2,5 mg ou moins » dans la remarque de la
  ligne de posologie, à deux centimètres l'une de l'autre sur la même
  fiche. Les deux se lisent côte à côte au comptoir. La remarque suit
  désormais la monographie, en gardant la nuance du sujet très âgé ou
  dénutri.

### Added
- **La fiche Wegovy**, 851e du référentiel. La fiche Ozempic disait déjà
  que « le sémaglutide dans l'obésité relève d'une autre spécialité et
  d'un autre schéma de doses » — et cette spécialité n'existait pas dans
  la base : le comptoir était renvoyé vers une fiche absente. Elle porte
  ce que la confusion entre les deux coûte, la titration de cinq mois
  qui ne s'accélère pas, la réévaluation qui décide de l'arrêt, la
  reprise de poids à l'annoncer avant la première injection, et la perte
  de masse musculaire quand les protéines et l'activité ne suivent pas.
  Le cliquet des posologies a fait son travail au passage : la fiche
  ajoutée sans ses lignes a fait tomber le test.
- **Un antidote nommé oblige la fiche à dire pourquoi.** `STARTER_DRUGS`
  porte une colonne « antidote », et la remplir revient à affirmer qu'il
  existe une dose à partir de laquelle il faut le donner : la fiche doit
  alors dire laquelle et à quoi on la reconnaît, sans quoi c'est une
  colonne remplie qui n'apprend rien à qui tient la boîte. Cent soixante
  fiches étaient dans ce cas au début de la passe, aucune ne l'est plus,
  et le test le tient désormais — vérifié en vidant la section de
  Tramadol : il tombe en le nommant. C'est le seul des trois
  recoupements de cette relecture assez net pour devenir un test ; ceux
  sur les demi-vies et sur les voies d'élimination restent des
  techniques de relecture, faute de savoir distinguer « demi-vie de
  trois jours » de « trois prises par jour » sans crier au loup.
- **Un plancher sur les fiches de biosimilaire**, dans
  `every_starter_card_carries_a_sourced_monograph` : trois mille
  caractères de corps clinique et quatre-vingts par champ — de quoi
  faire une phrase et pas une étiquette. Le test ne savait vérifier que
  le vide, et c'est exactement ce que douze fiches exploitaient sans que
  personne l'ait voulu. Vérifié en réintroduisant « Pas d'adaptation. »
  sur Hyrimoz : le test tombe. Le nombre de biosimilaires livrés est un
  cliquet comme les autres.
- **Deux règles de biologie et cinq listes élargies.** L'hypophosphatémie
  après une perfusion de carboxymaltose ferrique — effet connu, souvent
  profond, qui va jusqu'à l'ostéomalacie sur cures répétées — n'était
  lue par rien : la phosphorémie n'avait qu'une règle, et elle regardait
  vers le haut. L'hypokaliémie sous antiarythmique allongeant le QT non
  plus. Et la finérénone a rejoint la règle d'hyperkaliémie, le
  cinacalcet celle d'hypocalcémie, l'agomélatine, la vildagliptine, le
  tériflunomide, la terbinafine et le pazopanib celles de cytolyse.
- **Trois planchers écrits deux fois disaient deux choses.** Le même
  défaut dans trois fichiers : le chiffre dans l'assertion, les lettres
  dans le message, et les deux qui divergent au premier relèvement que
  personne ne relit en entier — `revue.rs` exigeait 61 en annonçant
  cinquante-cinq, `biology.rs` 55 en annonçant quarante-neuf et 95 en
  annonçant quatre-vingt-sept. Tous sont désormais des constantes
  nommées que le message relit. Les vrais chiffres, au passage,
  n'étaient pas ceux que je croyais : le cliquet a refusé deux fois un
  compte fait de tête.
- **Douze surveillances biologiques**, tirées elles aussi des sections
  toxicité, pour des traitements que `surveillance.rs` ne réclamait
  pour rien : la **créatininémie sous mésalazine** — une néphrite
  interstitielle sans signe d'appel, sous un traitement pris pendant des
  années et réputé anodin —, les transaminases sous agomélatine,
  vildagliptine, tériflunomide, terbinafine et pazopanib, les
  lymphocytes sous diméthyl fumarate et fingolimod (c'est le chiffre qui
  annonce la LEMP), la kaliémie sous finérénone et sous sotalol, la
  calcémie sous cinacalcet, la TSH sous sunitinib et la numération sous
  hydroxycarbamide. Le plancher passe de 49 à 61 et devient une
  constante nommée.
- **Cinq règles de revue d'ordonnance**, écrites à partir des
  interactions que la passe sur les sections toxicité a fait remonter et
  que `revue.rs` ne voyait pas : carbapénème sur valproate (état de
  mal), miconazole sur AVK et miconazole sur sulfamide hypoglycémiant —
  gel buccal et ovule compris, ce qui est tout l'intérêt de la règle,
  personne ne portant un gel pour la bouche sur la liste des
  traitements généraux —, gemfibrozil sur répaglinide, et aminoside
  avec diurétique de l'anse. Une sixième, inducteur enzymatique sur
  contraception, a été écrite puis **retirée** : la règle existait déjà
  sous un autre titre et plus complète, et un deuxième point disant la
  même chose sur la même ordonnance est du bruit. Le test
  `a_buccal_gel_is_read_as_a_general_treatment` tient les deux règles du
  miconazole, vérifié en cassant la règle : il tombe.
- **Le plancher des règles de revue est devenu une constante nommée**
  (`RULES_FLOOR`) et passe à 66 : l'assertion exigeait 61 quand le
  message parlait de cinquante-cinq. Même défaut, même correction que
  pour les sections toxicité.
- **Le plancher des sections toxicité est devenu une constante nommée**
  (`TOXIC_FLOOR`), que le message d'erreur relit. Il était écrit deux
  fois — en chiffres dans l'assertion, en toutes lettres dans le
  message — et les deux avaient divergé dès le troisième relèvement.
  Le cliquet a par ailleurs fait son travail deux fois pendant cette
  passe, en refusant un chiffre calculé de tête qui dépassait d'une
  unité le nombre réel de fiches.

## [0.141.0] - 2026-08-29

### Changed
- **Les pièces numérisées quittent la base et prennent leur propre
  fichier.** Mesuré avant de décider : une base semée sans pièce fait
  **6 Mo** ; deux cents ordonnances noir et blanc de 250 Ko la portent à
  **56 Mo**, dont 89 % de pièces ; et les quatorze sauvegardes
  quotidiennes en font **840 Mo**, recopiés chaque soir sur le partage
  de l'officine. Une année ordinaire à dix pièces par jour : ~700 Mo de
  base, ~10 Go avec les copies.
  - Les octets vont dans `<base>_scans.db`, à côté, **même SQLCipher et
    même clé** — pas une deuxième cryptographie écrite à la main. La
    base de travail reste à six mégaoctets : c'est elle qu'on ouvre cent
    fois par jour et qu'on recopie chaque soir.
  - `[scans] backups_keep` est séparé et vaut **2**, contre 14 pour la
    base. C'est tout l'intérêt de la séparation : une pièce ne change
    jamais après avoir été rangée, on n'a pas besoin d'en garder
    quatorze états.
  - La base garde la **fiche** de chaque pièce — libellé, genre, date, à
    qui elle est attachée. Volontairement : une base copiée seule montre
    encore ce qui existait, au lieu d'en perdre jusqu'à la trace, et
    l'ouvrir dit alors quel fichier manque.
  - Ce que la séparation impose, et qui est tenu : `change_password`
    rechiffre **les deux** fichiers — l'oublier laissait la liste des
    pièces s'afficher et pas une seule s'ouvrir, et c'est un test qui l'a
    trouvé —, « Copier la base… » copie les deux, et la lecture retombe
    sur l'ancienne colonne pour qu'une base d'avant ouvre encore ses
    pièces sans rien faire.
  - Le fichier est nommé d'après la base et non d'un nom fixe : deux
    bases posées dans le même dossier auraient partagé leurs pièces.

### Fixed
- **Supprimer une pièce ne rendait rien.** SQLite marque les pages
  libres et ne rétrécit jamais le fichier : effacer deux cents pièces
  laissait la base à cinquante-six mégaoctets, et la copie du soir
  recopiait les cinquante-six. Options › Base reçoit « Compacter la
  base » — sur son propre fil, comme les autres passes longues — qui
  déplace ce qui restait de pièces dans la base, balaie les octets
  orphelins, puis réécrit les deux fichiers. Dans cet ordre : compacter
  avant d'avoir déplacé réécrirait la base *avec* les pièces dedans.

## [0.140.0] - 2026-08-29

### Added
- **Les algorithmes de prise en charge, et les tableaux qui portent leurs
  cibles.** Les protocoles répondaient à ce qui arrive au comptoir — une
  rupture, une piqûre de tique, un oubli de pilule. Il manquait l'autre
  moitié : ce que l'ordonnance *devrait* porter. Le pharmacien ne
  prescrit pas ; il vérifie, il explique et il signale ce qui manque, et
  pour cela il faut connaître la marche.
  - **Sept algorithmes** (47 protocoles au lieu de 40) :
    l'**insuffisance cardiaque à FEVG altérée et ses quatre piliers** —
    demandée nommément —, où l'arbre demande classe par classe ce qui
    manque à l'ordonnance et rappelle les 36 heures entre un IEC et le
    sacubitril/valsartan ; l'**HTA** et sa marche (bithérapie d'emblée,
    trithérapie, puis spironolactone, et ce qui fait monter la pression
    avant de parler de résistance) ; le **diabète de type 2**, où
    l'atteinte cardiovasculaire ou rénale décide avant l'HbA1c ; la
    **dyslipidémie**, dont la cible se lit sur le risque et jamais sur un
    seuil unique ; la **fibrillation atriale** et le CHA₂DS₂-VASc, avec
    le rappel qu'un AOD est contre-indiqué sur valve mécanique et que
    l'aspirine n'est pas une alternative ; l'**asthme**, où trois flacons
    de salbutamol par an disent le non-contrôle mieux que le patient ;
    et l'**insuffisance rénale chronique**, stade par stade.
  - **Cinq tableaux** (43 au lieu de 38) : les cibles de LDL par niveau
    de risque, les quatre piliers de l'IC avec doses de départ, cibles et
    pièges, CHA₂DS₂-VASc et HAS-BLED item par item, les objectifs
    d'HbA1c selon le profil — y compris les deux du sujet âgé, où
    intensifier fait du mal —, et les stades de DFG avec ce que chacun
    arrête. Chacun cite ses sources et porte sa date de relecture.
  - Chaque algorithme nomme sa source dans son sujet et le dit : ce sont
    des recommandations et pas des ordres, et un prescripteur qui s'en
    écarte a le plus souvent une raison qui n'est pas sur l'ordonnance.
  - Les deux cliquets suivent : 47 protocoles, 43 tables.

## [0.139.0] - 2026-08-29

### Fixed
- **Troisième tour, et le plus instructif : deux réserves de hauteur
  fausses depuis longtemps.**
- **Les deux calculatrices se peignaient l'une sur l'autre.**
  `ui.columns` donne à chaque colonne son rectangle et ne découpe rien :
  à 1024 px en texte 1,25 les deux grilles étaient plus larges que leur
  moitié, et « 1050 mg par prise » se dessinait par-dessus « Clairance
  estimée : 62 mL/min ». Deux textes superposés ne se lisent ni l'un ni
  l'autre, et rien dans l'image ne dit lequel est lequel. Côte à côte si
  les deux tiennent — mesuré sur le plus long libellé —, l'un sous
  l'autre sinon.
- **`interact_size.y` n'est pas la hauteur d'une rangée.** À
  `text_scale = 1.25` elle vaut 27,5 px là où un bouton Motif en fait
  38 : une bande qui réservait ses rangées avec le premier nombre puis y
  dessinait des boutons était **dix pixels trop courte par rangée**. Le
  formulaire du carnet de vaccination en sortait avec sa deuxième
  rangée tranchée — précisément ce que les notes du projet interdisent,
  parce qu'une table à qui il manque une ligne se lit et défile, tandis
  qu'un formulaire coupé en deux ne se tape pas. La leçon était déjà
  écrite sur `button_height` ; elle est maintenant un `row_height` que
  les sept bandes concernées appellent.
- Et le partage du carnet est pris **dans l'autre sens** : le carnet
  demande d'abord ce que son formulaire exige, la bande « À faire /
  Voyage » prend ce qui reste et défile. Servie la première, elle
  laissait au formulaire six pixels de moins qu'il n'en faut.

## [0.138.0] - 2026-08-29

### Fixed
- **Deuxième tour de la passe sur l'interface**, sur les vues que le
  premier n'avait pas regardées. Quatre défauts, tous trouvés en
  ouvrant les images de `./scripts/eyeball.sh`.
- **Une bande noire le long du dock, sur l'agenda.** Le calendrier du
  mois demande plus large que le dock ne l'est : il peignait *à côté*,
  sur une bande que le panneau ne couvre pas, et l'on y voyait le fond
  de la fenêtre. `allocate_new_ui` ne fixe qu'un rectangle maximal et
  egui peint au travers — c'est écrit dans les notes du projet, et c'est
  exactement ce qui est arrivé. Les cinq navigateurs sont maintenant
  dessinés dans `motif::inside`, donc aucun ne peut en sortir.
- **Les boutons du tableau de bord débordaient vers la gauche.** Alignés
  à droite, une rangée plus large que son volet sort par le bord gauche,
  passe sous le dock, et « Récapitulatif de facturation… » se lisait
  « itulatif de facturation… » — le titre, lui, avait entièrement
  disparu. Un débordement vers la droite se voit ; celui-là se lit comme
  un bouton que quelqu'un a mal nommé. Le titre et les boutons partagent
  une ligne s'ils y tiennent, mesuré, et sinon les boutons prennent la
  leur — et de gauche à droite, où déborder se verrait.
- **Trois sous-titres coupés en deux** — codex, protocoles, dispositifs.
  Les trois bandes valaient « 116 px si plus étroit que 940, sinon 64 » :
  un même nombre deviné, recopié trois fois, juste à la taille de texte
  par défaut et faux à 1,25, où un titre est plus haut, un bouton est
  plus haut et le sous-titre passe à la ligne. `title_band_height` les
  mesure, sous-titre compris.

## [0.137.0] - 2026-08-29

### Changed
- **Une passe sur l'interface, faite en la regardant.** `smoke.sh` prouve
  que rien n'a paniqué ; il ne dit rien d'un bouton dessiné à moitié hors
  d'un panneau. `./scripts/eyeball.sh` prend chaque vue à 1024x700 en
  texte 1,25 et les pose dans un dossier. Voici ce qu'elles montraient.
- **Les registres quittent la base médicaments.** La page « Base
  médicaments » portait **huit** portes — trois rangées de rectangles
  gris indiscernables, au-dessus du champ de recherche pour lequel on
  ouvre cette page. Deux d'entre elles n'avaient rien à y faire : un
  registre de stupéfiants et une facture de grossiste ne sont pas de la
  référence sur le médicament, ce sont les papiers de l'officine. Les y
  avoir mises était commode à écrire et faux à lire.
  - « Registres » est une vue de l'espace de travail maintenant, comme
    l'agenda et le carnet, avec ses deux moitiés en onglets. Son volet de
    gauche est la liste des patients, et ce n'est pas un choix par
    défaut : pour inscrire une délivrance il faut le dossier ouvert, et
    c'est cette liste qui l'ouvre.
  - Six portes restent sur la base médicaments, toutes sur le
    médicament, et elles tiennent en deux rangées.
- **La bande d'onglets se déplace sur celui qui est actif.** Elle
  défilait, mais jamais *vers* quelque chose : six onglets de fiche
  patient à 1024 px en texte 1,25 laissaient le sixième coupé à une
  lettre, sans ascenseur pour l'expliquer, et un dossier ouvert
  directement dessus affichait une page dont l'onglet était hors écran.
- **Deux bandes mesurées mais non plafonnées** écrasaient le panneau
  qu'elles surmontaient : le formulaire des pièces réduisait « Pièces au
  dossier » à un titre au-dessus de rien, et celui du registre réduisait
  le registre à une ligne coupée. Plafonnées à la moitié, et les deux
  moitiés défilent.
- **La courbe du stock cède avant les lignes du registre.** Sur un volet
  court elle passe à la trappe : un graphique gardé au prix des lignes
  qu'il illustre est un graphique de rien. Le plancher est en lignes.
- **Le libellé d'une pièce cède, jamais ses boutons.** « Corriger » et
  « × » sortaient du volet sur un panneau étroit : une ligne qu'on ne
  peut ni corriger ni retirer, sans rien dans le cadre pour dire qu'ils
  sont là. La place qu'ils prennent est mesurée et retranchée d'abord.

## [0.136.0] - 2026-08-29

### Added
- **Les pièces numérisées.** L'ordonnance, la déclaration d'accident du
  travail, le courrier du spécialiste, le compte rendu de biologie : une
  officine reçoit du papier toute la journée et le range dans un
  classeur qui n'est pas à côté du dossier.
  - Un onglet « Pièces » sur la fiche patient — c'est là qu'on cherche
    une ordonnance, et l'y faire chercher ailleurs serait un clic de plus
    cent fois par jour. Le même volet sert la fiche médicament (une
    notice, un courrier de retrait de lot) et l'officine (les factures,
    le registre AT, ce qui n'appartient à aucun dossier).
  - **Le format se lit dans les octets, jamais dans le nom.**
    L'application garde la pièce puis la ressort sur le disque et la
    confie au système : accepter un fichier parce qu'il s'appelle `.pdf`
    serait accepter de rendre plus tard au système ce qu'on lui a pris
    sans le regarder. Quatre nombres magiques — PDF, PNG, JPEG, TIFF — et
    le reste est refusé à l'entrée, où le refus se comprend. L'extension
    sous laquelle la pièce ressort vient du contenu.
  - **Dans la base et non dans un dossier à côté** : une ordonnance
    numérisée posée en clair à côté d'une base chiffrée annule le
    chiffrement de la base. D'où un plafond de taille (`[scans] max_mb`,
    dix mégaoctets par défaut) et une ligne dans Options › Base qui dit
    ce que les pièces pèsent — elles voyagent dans chaque sauvegarde
    quotidienne, et une officine qui ne le voit pas s'en aperçoit le jour
    où la copie du soir ne tient plus.
  - **Les octets ne se réécrivent jamais.** Une numérisation est ce que
    le scanner a produit ; la remplacer sous le même libellé ferait
    mentir tout ce qui la cite. Le genre, le libellé, la date et la
    remarque se corrigent — compare-and-set, comme toute ligne partagée.
    Une pièce se supprime, elle, en deux clics : elle a pu être
    numérisée deux fois ou attachée au mauvais dossier.
  - **Le scanner est en configuration**, comme la séquence APDU de la
    carte Vitale : `scanimage` sur un poste Linux, le pilote du
    constructeur ailleurs, et l'officine sait quel est son matériel mieux
    que ce binaire. `{out}` est le fichier que l'application ira lire, et
    une commande qui ne le nomme pas est refusée avant d'être lancée.
    Réglable dans Options › Base ; vide, il n'y a qu'« Importer… ».
  - Le découpage de cette commande a son test, et il a servi : rempli
    avant d'être découpé, un chemin de sortie contenant une espace —
    « C:\Users\Jean Martin\… », la moitié des postes — devenait deux
    arguments, et le scanner écrivait dans « C:\Users\Jean ».

## [0.135.0] - 2026-08-29

### Added
- **L'ordonnancier des stupéfiants**, et la réception des commandes avec
  lui. « Stupéfiants… » depuis la vue Médicaments : les produits suivis
  à gauche avec leur solde, le registre du produit ouvert au milieu, la
  ligne qu'on écrit à droite.
  - **Le registre est inaltérable, et c'est un test et non une
    convention.** R. 5132-36 demande un registre qu'on ne rature pas :
    une ligne écrite ne se corrige pas, elle se contre-passe. Il n'y a
    donc sur `stup_moves` ni `UPDATE`, ni `DELETE`, ni méthode qui en
    proposerait — et `the_register_can_only_ever_be_written_to` relit le
    texte de `db.rs` et refuse qu'il en apparaisse un. Vérifié en
    ajoutant une correction « bien intentionnée » : le test la voit.
  - **La balance n'est pas une somme.** Un inventaire *pose* le solde au
    lieu de s'y ajouter : additionner l'écart et poser le compte le
    compterait deux fois, et le registre dériverait dès le premier
    comptage qui ne tombe pas juste — c'est-à-dire dès le premier. Le
    registre se lit dans l'ordre des **jours** et non des identifiants :
    une réception saisie le lendemain n'est pas une réception du
    lendemain, et un inventaire lu avant les sorties qui le précèdent
    invente un manquant.
  - **Le numéro d'ordonnancier** est attribué par la base, dans la
    transaction qui écrit la ligne : proposé par l'écran, il serait un
    numéro que le poste d'à côté peut avoir pris entre-temps. Séquentiel
    dans l'année, jamais réattribué — un numéro annulé laisse un trou, et
    le reboucher ferait exister deux délivrances sous le même numéro.
  - **Le patient est lié, jamais affiché.** Une ligne de délivrance porte
    « dossier 42 », cliquable ; le nom se lit en ouvrant le dossier. Un
    registre s'imprime et traîne au comptoir : ce qu'il doit permettre,
    c'est de *remonter* au patient. Et une délivrance sans dossier ouvert
    est refusée plutôt qu'écrite sans patient.
  - **La réception va vite** : `[stock] suppliers` donne les grossistes
    en pastilles, le premier étant celui qu'on propose — une réception se
    saisit entre deux clients, et taper « OCP » quinze fois par semaine
    est quinze fois de trop. Le formulaire change avec la nature de la
    ligne : une délivrance demande un prescripteur, une réception un
    grossiste et un bon de livraison, un inventaire rien qu'un comptage.
  - **La liste de contrôle** dit ce qu'il faut aller compter et pourquoi
    — solde négatif (le seul motif qui est une erreur et pas un rappel),
    sous le seuil, jamais recompté depuis `[stock] count_days`. Elle
    s'imprime avec le solde du registre en face et une colonne pour ce
    qu'on trouve : recompter un placard sans savoir ce qu'on cherche est
    ce qui fait qu'on ne le fait pas.
  - Et la courbe du stock, en couleurs par nature de ligne.

### Fixed
- **`motif::list_row` prenait un `RichText` et n'en gardait que la
  chaîne.** Trois endroits peignaient un rendez-vous en retard dans le
  rouge d'alerte, et il sortait dans l'encre ordinaire depuis le jour où
  ils ont été écrits. Rien n'échouait, rien n'avait l'air cassé, et le
  rouge n'était simplement pas là. La mise en page est celle d'egui
  maintenant : couleur, graisse et italique arrivent avec elle.
  - Au passage, la police venait d'un 14 px en dur : les listes ne
    grandissaient pas avec `[ui] text_scale`, exactement le défaut que
    les boutons avaient et avaient corrigé. Et la hauteur d'une ligne est
    **mesurée après** la mise en page : une police plus grande donne une
    ligne plus haute au lieu d'une ligne rognée.

## [0.134.0] - 2026-08-29

### Added
- **Le codex passe de 58 à 80 préparations**, sur trois établis que la
  liste ne couvrait pas.
  - **La dermatologie du comptoir** : le chlorure d'aluminium à 20 %
    (l'hyperhidrose, et le rappel qu'il s'applique sur une peau sèche le
    soir et se rince le matin — appliqué sur une peau humide ou après le
    rasage, il brûle), l'érythromycine et la clindamycine locales de
    l'acné, chacune avec la règle qui les tient (jamais en monothérapie
    prolongée, sous peine de sélectionner des résistances en quelques
    semaines), l'ichtammol du furoncle, le dermocorticoïde dilué au demi
    — qui baisse la concentration et **jamais la classe** —, le talc
    salicylé des pieds, la solution de Burow, l'eau oxygénée à 10
    volumes et le bleu de méthylène.
  - **La pédiatrie et la sonde** : hydrochlorothiazide, amlodipine,
    sildénafil de l'HTAP du nouveau-né, acétazolamide, chlorure de
    potassium et acide ursodésoxycholique en suspension ou en solution
    buvable, plus les gélules d'acide folique et de zinc. Chacune porte
    le piège qui lui est propre — le sel d'amlodipine qui change d'une
    spécialité à l'autre et change le calcul, le zinc élément qui n'est
    pas le sulfate de zinc et divise la dose par quatre si on les
    confond, le potassium dont l'unité est la mmol et non le mg, et les
    folates qui corrigent l'hémogramme d'une carence en B12 pendant que
    l'atteinte neurologique avance.
  - **La bouche et le nez** : le bain de bouche lidocaïne-bicarbonate-
    nystatine de la mucite (à prendre trente minutes avant un repas et
    non juste avant : manger sur une bouche anesthésiée est la fausse
    route), le lavage nasal bicarbonaté avec le geste du nourrisson —
    allongé, tête tournée, jamais assis tête en arrière —, et l'huile
    gomenolée, contre-indiquée avant trente mois.
  - Et les ovules et suppositoires à la glycérine, avec le facteur de
    déplacement qui n'est pas une formalité : sans lui les ovules sont
    sous-dosés du volume qu'occupe le principe actif.
  - Le cliquet suit : le test refuse désormais moins de quatre-vingts.

## [0.133.0] - 2026-08-29

### Added
- **La base en carte : « Voisinage… ».** Une fiche au centre, son
  voisinage autour, et un clic déplace le centre. La liste et la
  recherche répondent à « où est telle fiche » ; elles ne répondent pas
  à « qu'est-ce qu'il y a autour », qui est la question d'une rupture de
  stock, d'une contre-indication trouvée au comptoir, et de qui apprend
  une classe. Une classe s'explore en s'y déplaçant plutôt qu'en tapant
  un nom, le lisant, revenant en arrière et en tapant un autre.
  - Trois liens, trois anneaux, trois couleurs : **la molécule** (une
    autre marque de la même DCI, le seul lien où les deux boîtes
    contiennent la même chose), **la classe**, et **l'interaction** —
    une fiche que la monographie du centre nomme elle-même, le seul lien
    qui ne découle pas de la classification et qui peut traverser toute
    la base. Une fiche appartient à l'anneau **le plus proche** pour
    lequel elle est éligible, et à un seul : le même nom deux fois sur
    une carte, ce sont deux réponses à une question.
  - Un nom n'est reconnu dans la prose que comme **mot entier**, et
    jamais en dessous de quatre lettres : « fer » est dans « conférer »,
    et une carte qui relie deux fiches parce que trois lettres de l'une
    sont dans une phrase de l'autre est une carte de coïncidences.
  - Ce que l'anneau n'a pas pu prendre est **dit** : douze AINS dessinés
    sur quarante sans un mot se liraient « il y en a douze », une
    mauvaise réponse qui a l'air complète.
  - Les fiches à marge thérapeutique étroite portent un liseré d'alerte.
    C'est pour cela que l'interaction est ocre et non rouge : un liseré
    rouge autour d'un nœud rouge n'est pas un liseré.
  - Depuis la carte : ouvrir la fiche, ajouter la molécule à
    l'ordonnance du patient ouvert, ou repartir créer une fiche — avec le
    nom cherché et non trouvé, qui voyage jusqu'au bouton de création.
  - `src/graph.rs` est pur et testé, sans base et sans egui : il rend des
    points sur le cercle unité, la vue les met à l'échelle. C'est ce qui
    fait qu'une carte de 850 fiches ne coûte rien par image — la
    disposition est calculée quand le centre bouge, et peinte ensuite.
    Aucun tirage au sort, aucune simulation de forces : une image qui se
    range autrement à chaque ouverture ne s'apprend pas.

### Fixed
- **Le garde-fou de couverture ne regardait plus tout ce qu'il devait.**
  `coverage.sh` nommait les modules de logique un par un, et la liste
  avait pris quatre modules de retard : `conciliation`, `surveillance`,
  `vitale` et `graph` étaient de la logique que personne ne comptait —
  précisément ce que ce script existe pour empêcher. Elle est maintenant
  écrite par ce qu'elle **exclut** (`app.rs`, `main.rs`, `winscard.rs`,
  `motif`, le lanceur), donc un module ajouté demain est mesuré le jour
  où il arrive et doit être exclu exprès, par écrit, pour y échapper.
  L'ensemble mesuré passe de 11 861 à 13 353 lignes, et le chiffre monte
  de 87,6 à **88,4 %** : les modules qui manquaient étaient bien testés.
  Les deux planchers montent d'autant (88 et 43).

## [0.132.0] - 2026-08-29

### Added
- **Un clic sur la classe donne les fiches de cette classe.** Les deux
  questions du comptoir, la boîte à la main : « c'est la même chose ? »
  et « on met quoi à la place ? ». La pastille de la DCI et celle de la
  classe portent maintenant leur nombre — « AOD (3) » — et l'ouvrent en
  liste sur place, sous la pastille cliquée, chaque ligne ouvrant sa
  fiche. Le compte est déjà une réponse avant qu'on ait cliqué.
  - Les deux listes sont tenues à part parce que ce ne sont pas les
    mêmes réponses : une autre marque d'apixaban *est* de l'apixaban, un
    autre AOD ne l'est pas. Une fiche n'est jamais sa propre voisine, ni
    dans les deux listes à la fois. La casse et les accents sont pliés
    des deux côtés (« Bêtabloquant » et « betabloquant » sont une classe
    tapée deux fois), et une DCI ou une classe vide ne rapproche
    personne — sans quoi toutes les fiches que l'équipe n'a pas finies
    seraient voisines de tout le monde.
  - Les étiquettes, elles, continuent de lancer la recherche : une
    étiquette est un mot lâche, pas une liste.
  - `fuzzy::eq_folded` compare deux mots pliés sans allouer, parce que
    la question est posée sur 850 fiches chaque fois qu'on en ouvre une.
    La liste est calculée à l'ouverture, jamais par image.
- **La demi-vie répondait à une question sur deux.** La courbe qui
  descend dit quand c'est parti — ce que demandent une interaction, une
  opération, un changement de traitement. Il manquait celle qui monte :
  où en est un traitement régulier de son plein effet. C'est la question
  de la titration, et personne ne la fait de tête au comptoir.
  - Cinq demi-vies, quel que soit l'espacement des prises. Pour la
    lévothyroxine, dont la demi-vie est de sept jours : **cinq
    semaines** — et une TSH prélevée avant mesure un patient encore en
    train de monter, ce qui envoie le prescripteur courir après un
    chiffre qui bouge. Même arithmétique pour le lithium, l'amiodarone,
    la digoxine, un AVK à l'instauration, un ISRS.
  - Les deux courbes couvrent les mêmes cinq demi-vies, donc elles sont
    dans **un** cadre sur **une** échelle, où elles se croisent à 50 %
    après une demi-vie. `motif::chart::lines` est cette échelle
    partagée : deux `sparkline` se seraient chacune mise à ses propres
    données et auraient dessiné un plateau à 96,9 % à la hauteur d'un
    départ à 100 %.
  - Le test de `lines` a trouvé son premier trou : `f64::clamp` laisse
    un NaN passer, et une coordonnée NaN dans un `Shape::line` dessine
    ce qu'elle veut. Ce qui n'est pas un nombre vaut zéro.

## [0.131.0] - 2026-08-29

### Changed
- **La mise à jour du contenu de référence ne bloque plus rien, et elle
  est soixante-huit fois plus rapide.** « Synchroniser le contenu de
  référence » tournait là où on appuyait, entre deux images : la fenêtre
  cessait de répondre jusqu'au bout. Mesuré en compilation de release :
  **250 secondes**. Quatre minutes de comptoir sans curseur, sans moyen
  de distinguer une passe lente d'une application morte.
  - La cause n'était pas le chiffrement mais l'absence d'index. Il n'y en
    avait **aucun** dans le schéma : chaque `WHERE name = ?1` lisait les
    850 fiches, et `fill_starter_details` en pose dix-huit par fiche —
    quinze mille parcours de huit cent cinquante lignes, chacune une page
    que SQLCipher déchiffre. Quatorze index plus tard, la même passe
    prend **3,7 secondes**. Ils profitent à tout le reste : le dossier
    patient, les posologies d'une fiche, « qui prend ce médicament ».
    Ils sont créés **après** les migrations, parce qu'un index nomme une
    colonne et qu'une colonne ajoutée depuis n'existe pas encore quand le
    schéma passe.
  - Et les quatre passes longues — synchroniser, compléter les
    médicaments, compléter les fiches, réinitialiser — ont leur propre
    fil (`src/maintenance.rs`), qui ouvre sa propre connexion : la base
    est déjà partagée entre postes, un deuxième lecteur dans le même
    processus est le cas pour lequel elle est faite. L'étape en cours
    s'écrit sous le bouton qui l'a lancée, les quatre boutons grisent
    pendant ce temps, et une passe dont la fenêtre est partie s'arrête à
    l'étape suivante au lieu de finir d'écrire dans une base que
    personne ne lira.
  - Au passage : `seed_missing_drugs` appelait six des sept passes qui la
    suivaient dans la liste du bouton, donc chacune tournait deux fois.
    La liste est maintenant à un seul endroit — `maintenance::steps` —
    et le test de synchronisation la lit de là plutôt que de la recopier.

## [0.130.0] - 2026-08-29

### Fixed
- **La bibliothèque qui manquait au runner : `libxkbcommon-x11-0`.**
  winit l'ouvre au moment de l'exécution, comme eframe le fait d'OpenGL,
  et le paquet `-dev` que les autres travaux installent ne porte pas
  celui du système. Ce n'était donc pas une histoire de carte graphique.
  Le garde-fou l'a nommé lui-même — « Library libxkbcommon-x11.so could
  not be loaded » — à sa deuxième exécution, ce qui est précisément ce
  pour quoi on lui a fait dire pourquoi.

## [0.129.0] - 2026-08-29

### Fixed
- **Le garde-fou de démarrage dit pourquoi.** Il disait « l'application
  ne reste pas ouverte cinq secondes » et rien d'autre, ce qui envoie
  celui qui le lit deviner des paquets système. Il rapporte maintenant
  le code de sortie et ce que le programme a écrit en partant — la ligne
  qui nomme la bibliothèque manquante. Un garde-fou qui détecte sans
  expliquer fait la moitié du travail, et c'est la moitié facile.

## [0.128.0] - 2026-08-29

### Fixed
- **Le travail de vérification a de quoi dessiner.** eframe passe par
  OpenGL, qu'il ouvre à l'exécution plutôt que de le lier : un runner
  sans carte graphique a besoin du rasteriseur logiciel de Mesa, et il
  faut le lui dire au lieu de le laisser chercher un matériel qui n'est
  pas là. Les paquets sont ajoutés et `LIBGL_ALWAYS_SOFTWARE=1` est
  posé sur l'étape. Le passage complet a été refait localement dans ces
  conditions.
  - Et c'est écrit dans le fichier : si ce travail échoue un jour parce
    que l'application ne démarre pas, les deux réponses honnêtes sont de
    corriger les paquets ou de retirer le travail. Le faire passer quand
    même est la seule chose à ne pas faire — la vérification du script
    est « aucune panique n'est sortie », et un binaire qui n'a jamais
    tourné n'en sort pas non plus.

## [0.127.0] - 2026-08-29

Le seul garde-fou de l'interface tourne désormais à chaque poussée, et
il sait dire qu'il n'a rien vérifié.

### Fixed
- **Deux comptes corrigés.** Les cliquets disaient 47 surveillances et 94
  règles de biologie ; il y en a 49 et 95. Le delta annoncé à chaque fois
  était juste, l'absolu ne l'était pas — un compte de départ mal relevé,
  recopié ensuite. Les planchers sont remis sur la vérité, qui est ce
  qu'un lecteur peut vérifier.

### Added
- **`smoke.sh` est un travail de CI.** L'interface n'a pas de tests
  unitaires — une vue ne se couvre pas sans harnais, et `egui_kittest`
  demande egui ≥ 0.30 alors que le projet est sur 0.29 — donc
  `smoke.sh` est ce qui en tient lieu, et il ne tournait que lorsque
  quelqu'un y pensait.
- **Il prouve d'abord que l'application démarre.** Sa vérification est
  « aucune ligne de panique n'est sortie », et un binaire qui meurt
  avant de dessiner quoi que ce soit n'en sort pas non plus : une
  bibliothèque manquante, un affichage qui n'est pas monté, une
  construction qui n'a pas été refaite — chacun se lisait comme un
  passage propre. Le script lance donc l'application une fois et exige
  que `timeout` ait eu à la tuer : autre chose veut dire qu'elle s'est
  arrêtée toute seule, et les soixante-seize ouvertures qui suivent ne
  prouveraient rien.
  - Vérifié en remplaçant le binaire par un `exit 1` : le script
    s'arrête avec le message et le code 1, au lieu d'annoncer que tout
    va bien.

## [0.126.0] - 2026-08-29

La règle des migrations est tenue par un test au lieu d'être demandée
par une phrase.

### Added
- **`tests/fixtures/schema-0.109.0.sql`** : une photographie du schéma
  tel que cette version l'a livré, et un test qui crée une base avec,
  l'ouvre, et fait tourner dessus **chaque lecture que l'application
  fait** — les patients, les fiches, les posologies, la biologie, le
  carnet, les locations, le codex, les dispositifs, les protocoles, les
  notes, la liste d'appel, l'export, l'agenda.
  - `SCHEMA` crée ce dont une base *neuve* a besoin ; `MIGRATIONS` est la
    seule chose qui transforme une ancienne en celle-là. La règle — un
    changement de schéma va dans les deux — était écrite dans
    `CLAUDE.md` et rien ne la faisait respecter : on ajoute une colonne à
    `SCHEMA`, on la lit dans une requête, on oublie l'`ALTER`, et tous
    les tests passent. Ils tournent tous sur des bases que *cette*
    version a créées, où `SCHEMA` a mis la colonne de toute façon.
    L'officine qui met à jour est celle qui l'apprend.
  - Vérifié en retirant l'`ALTER` de `posology` : le test existant
    passe — son schéma ancien est écrit à la main et ne contient pas
    cette table — et le nouveau échoue sur « no such column: posology ».

## [0.125.0] - 2026-08-29

Le passage de vérification ouvre chaque vue dans deux formes, parce que
la panique qu'il cherche ne se produit pas dans la première.

### Changed
- **`scripts/smoke.sh` ouvre les trente-huit vues deux fois** : à
  1400x900, et à 1024x700 avec `text_scale = 1,25` — la forme qu'a un
  écran de comptoir et que n'a pas celui d'un développeur. `f32::clamp`
  fait tomber toute l'application quand un plancher calculé croise un
  plafond calculé, et les planchers ne croisent les plafonds que sur un
  panneau court en grand texte. `CLAUDE.md` demandait cette forme comme
  une vérification manuelle ; elle se fait maintenant à chaque passage.
  - Vérifié en cassant exprès : un `clamp` inversé dans le panneau de
    conciliation passe à 1400x900 et fait tomber l'application à
    1024x700 en texte 1,25. Un seul passage l'aurait laissé partir.
  - Chaque forme part d'une configuration jetable à elle, pour ne pas
    hériter des volets et de la fenêtre que la précédente a laissés dans
    `layout.toml`.
  - Les soixante-seize ouvertures passent.

## [0.124.0] - 2026-08-29

La dernière écriture partagée qui ne se comparait pas.

### Fixed
- **`delete_note` est en compare-and-set**, comme toutes les autres
  écritures sur une ligne partagée. Un relevé de tous les `UPDATE` et
  `DELETE` de `src/db.rs` n'a trouvé qu'elle : le journal était le seul
  endroit où l'on supprimait sur la foi de ce que l'écran montrait. Et
  c'est précisément là que cela compte — deux postes lisent la même
  page, l'un corrige une entrée, l'autre appuie sur le × à côté de ce
  qu'il croit encore lu, et une note que personne n'a lue disparaît.
  - Sept appels, sept listes déjà chargées : le texte attendu se relit
    sur la liste que la vue affiche, sans toucher aux widgets. `false`
    donne « Note modifiée depuis un autre poste — liste actualisée ».
  - Le test supprime avec un texte que personne n'a écrit (refusé), avec
    le bon (accepté), puis une seconde fois avec le bon (refusé aussi) :
    deux postes qui appuient sur le même × ne peuvent pas tous les deux
    croire avoir supprimé.

## [0.123.0] - 2026-08-29

Deux contrats que rien ne tenait : celui du lanceur avec le workflow, et
celui des couleurs de graphique avec les palettes.

### Added
- **Le lanceur a son premier test, et c'est celui qui compte.** Les noms
  de fichiers qu'il télécharge et ceux que le workflow de release publie
  vivent dans deux fichiers que rien ne rapprochait : `CLAUDE.md` le
  disait en prose, et une prose ne casse pas une compilation. Un
  renommage dans l'un des deux est invisible — la construction passe au
  vert, la release est publiée, et tous les lanceurs déjà installés
  cherchent un fichier qui n'existe pas. Silencieusement, jusqu'à ce que
  quelqu'un redémarre. Et il n'y a aucun moyen de corriger cela à
  distance, puisque ce qui irait chercher le correctif est justement ce
  qui est cassé.
  - Le test lit `.github/workflows/release.yml` à la compilation,
    reconstitue les trois noms que la matrice produit, et exige que les
    trois plateformes s'y retrouvent — un poste Linux doit remarquer
    qu'on a renommé l'asset Windows. Il vérifie aussi que l'étape
    d'envoi envoie bien le lanceur : une officine qui installe pour la
    première fois n'a que celui-là à télécharger.
- **36 → 40 protocoles de comptoir**, dont trois qui répondent à ce que
  les nouveautés de cette série ont rendu possible.
  - **« Ordonnance de sortie d'hôpital présentée au comptoir »** — le
    déroulé de la conciliation : la feuille est-elle là, un traitement
    a-t-il disparu, et surtout — le patient en a-t-il encore des boîtes
    chez lui. C'est la branche qui fait le plus de dégâts, parce qu'il
    continuera de le prendre.
  - **« Bilan biologique apporté au comptoir »** — un résultat critique
    arrive souvent à l'officine avant d'avoir été vu par le médecin, et
    c'est là qu'il se rattrape. Sinon, noter les valeurs : c'est ce qui
    permet à l'application de les lire contre les traitements, et de
    voir la fois d'après ce qui n'a pas été redemandé.
  - **« Erreur de délivrance constatée après coup »** — rien ne couvrait
    ce moment-là. La première question est si une dose a été prise, la
    seconde ce que c'était, et l'appel se fait tout de suite, pas à la
    fermeture.
  - **« Sevrage tabagique demandé au comptoir »**, qui déroule la table
    « Tabac » : le délai de la première cigarette, le nombre par jour,
    la dose de patch, et le rappel que le pharmacien peut prescrire —
    vendre hors ordonnance ferait payer pour rien.

- **Les couleurs de graphique sont tenues par un test.** Leur propre
  commentaire promettait qu'elles se distinguent et qu'elles se lisent
  sur les six palettes ; personne ne l'avait vérifié. La lisibilité est
  la moitié qui compte — une barre de la couleur de son fond est une
  barre que personne ne voit, et cela arriverait sans bruit parce qu'un
  graphique *ressemble* toujours à un graphique. La séparation est un
  cliquet plutôt qu'une règle : la paire la plus proche
  d'aujourd'hui — le vert de la série 2 et le bleu de la série 6 — fixe
  la limite, et aucune nouvelle couleur ne peut faire pire.

## [0.122.0] - 2026-08-29

Ce que le dossier de sauvegarde contient — parce que jusqu'ici rien ne le
disait — et six règles de revue de plus.

### Added
- **55 → 61 règles de revue d'ordonnance.**
  - **Bêtabloquant + anticholinestérasique** : les deux ralentissent le
    cœur par des voies différentes, et l'effet s'additionne. Bradycardie,
    syncope, et la chute qui s'ensuit chez quelqu'un qu'on traite déjà
    pour ses troubles cognitifs.
  - **Bêtabloquant + amiodarone** : l'association existe et se surveille,
    mais elle n'est jamais anodine — d'autant que l'amiodarone s'élimine
    sur des mois.
  - **Deux opioïdes faibles** : le pendant de « deux sources de
    paracétamol », et l'une des deux est souvent cachée dans une
    association.
  - **Sulfamide + insuline** : les deux seuls antidiabétiques qui font
    l'hypoglycémie, ensemble.
  - **Alpha-bloquant + antihypertenseur** : l'hypotension orthostatique
    est un mécanisme de chute que personne ne relie au traitement de la
    prostate.
  - **IPP + fer oral** : le fer a besoin de l'acidité que l'IPP
    supprime ; le traitement martial échoue et on augmente la dose au
    lieu de regarder l'ordonnance.
- **Options › Base dit ce que le dossier `backups` contient** : combien
  de copies, de quand à quand, et combien de méga-octets. La copie
  quotidienne écrit ses échecs sur la sortie d'erreur et nulle part
  ailleurs — c'est délibéré, une sauvegarde qui échoue ne doit pas
  bloquer le comptoir — mais cela veut dire qu'un disque plein, un
  partage réseau qui n'a pas monté ou un dossier passé en lecture seule
  sont totalement silencieux, et que l'officine peut se croire
  sauvegardée pendant des mois. La ligne dit aussi quoi regarder : si la
  date la plus récente n'est pas celle du jour, le dossier n'est pas
  accessible.
  - `db::backup_state` lit le dossier et ne crée rien. Un dossier absent
    est un dossier vide, ce qui est précisément ce qu'il faut annoncer.
    Il ne compte que les fichiers que l'application a nommés : une note
    laissée là ou un export de l'équipe n'est pas une copie, et le
    compter ferait croire à une copie de plus.
  - Le nom et le dossier des sauvegardes sont maintenant dits à un seul
    endroit (`db::backup_dir` / `db::backup_name`), au lieu d'être
    réécrits par la routine quotidienne et par ce qui la relit.
- `BPM_CADDY_START_VIEW=base` ouvre les options sur cette page — la seule
  qui lise le disque, et donc la seule que le passage de `smoke.sh` ne
  couvrait pas.

## [0.121.0] - 2026-08-29

La liste d'appel s'imprime, et une posologie à moitié tapée ne fait plus
refuser l'écriture.

### Added
- **« Liste d'appel… »** sur le tableau de bord, à côté de l'export et du
  récapitulatif. Le tableau dit qui rappeler et ne dit rien de ce qu'on a
  fait de l'appel : cette feuille porte le nom, le numéro — lu sur la
  fiche du patient, parce qu'une liste sans numéro oblige à chercher
  chaque nom —, le motif en trois mots, ce que dit le dossier, une case
  à cocher et une colonne vide pour ce qui a été dit. Elle se coche au
  téléphone, et c'est ce qui permet de reprendre la liste le lendemain
  sans rappeler deux fois les mêmes.

### Fixed
- **« Reprendre les posologies » se refusait elle-même.** L'écriture
  comparait la valeur attendue au *tampon de saisie* — ce qu'il y a dans
  le champ, à moitié tapé compris — et non à ce que la base avait donné.
  Dès qu'on avait touché un champ sans valider, la reprise échouait sur
  « posologie modifiée depuis un autre poste », ce qui était faux. Le
  test tape dans le champ exprès, et échoue si la correction est
  retirée.

## [0.120.0] - 2026-08-29

Les moteurs de règles à l'échelle d'une vraie base, et trois carrés
creux que personne n'avait vus.

### Added
- **49 → 55 analytes, 87 → 95 règles, 48 → 49 surveillances.** Six lignes
  que la ville imprime et que le catalogue n'avait pas, chacune entrant
  avec la règle qui la lit contre un traitement.
  - **Le TCA** est le test de l'héparine non fractionnée et de rien
    d'autre : sous HBPM il ne veut rien dire, et c'est l'anti-Xa qui
    répond. Au-delà de 3 fois le témoin c'est un surdosage — et il faut
    vérifier l'heure du prélèvement, parce qu'un TCA fait trop tôt
    surestime toujours. Une surveillance va avec.
  - **Les réticulocytes** disent si la moelle répond : une anémie avec
    des réticulocytes bas sous méthotrexate ou azathioprine est une
    toxicité médullaire, pas une carence.
  - **Les LDH et l'haptoglobine** se lisent ensemble : LDH hautes,
    haptoglobine effondrée, c'est une hémolyse — et sous
    nitrofurantoïne, dapsone ou sulfamide, on pense au déficit en G6PD.
  - **Le cortisol à 8 h** bas sous ou après corticothérapie prolongée :
    la surrénale ne s'est pas remise en route. C'est la règle des jours
    de maladie qu'il faut expliquer, et c'est au comptoir qu'on
    l'explique.
  - **L'ammoniémie** monte sous valproate à valproatémie normale, et le
    topiramate associé aggrave : l'encéphalopathie hyperammoniémique se
    manifeste par une somnolence qu'on met sur le compte du traitement.

- **34 → 38 tables de référence.**
  - **« Foie »** : l'insuffisance hépatique n'avait pas sa table alors
    que le rein avait la sienne. Le score de Child-Pugh, parce que la
    plupart des RCP ne parlent qu'en Child et que la moitié des
    contre-indications commencent à Child B ; le paracétamol, qui reste
    le premier choix mais à 3 g et à 2 g chez le dénutri ; les AINS, qui
    précipitent le syndrome hépato-rénal et l'hémorragie sur varices ;
    les benzodiazépines qui passent — oxazépam, lorazépam, témazépam —
    et celles qui s'accumulent ; l'AOD contre-indiqué en Child C ; et
    l'alcool des excipients, qu'on oublie chez quelqu'un en sevrage.
    Avec une colonne « ce qu'on ne fait pas », parce qu'ici la moitié
    des erreurs sont des gestes de bonne foi.
  - **« Tabac »** : les substituts nicotiniques, dose comprise. Une
    cigarette vaut environ 1 mg absorbé, le sous-dosage est la première
    cause d'échec et il se voit ; le délai de la première cigarette est
    la question la plus utile du Fagerström ; rien d'acide dans les
    quinze minutes qui précèdent une gomme, ce que personne ne dit
    spontanément ; fumer sous patch demande d'augmenter la dose et non
    de retirer le patch ; et l'induction du CYP1A2 qui disparaît à
    l'arrêt — une des rares interactions qui apparaissent en *arrêtant*
    quelque chose.
  - **« Sonde »** : donner un médicament sans la bouche. Le rinçage
    entre deux médicaments, qui est ce qui manque le plus souvent ; un
    médicament à la fois et jamais un mélange ; la lévothyroxine et la
    phénytoïne dont l'absorption s'effondre au contact de la nutrition ;
    l'IPP dispersé dans le bicarbonate plutôt qu'écrasé ; et la question
    qu'on oublie de poser — où la sonde se termine.
  - **« Voyage »** : l'ordonnance en DCI, parce qu'un nom de marque
    français ne veut rien dire ailleurs ; l'insuline en cabine, parce
    que la soute descend sous zéro ; le décalage horaire traité
    séparément pour la thyroïde, la contraception, l'insuline et
    l'anticoagulant, qui n'ont pas la même tolérance ; et la phrase qui
    change une prise en charge au retour — « je reviens de… ».

### Changed
- **`fuzzy::contains_folded` : les moteurs de règles n'allouent plus.**
  La revue passe cinquante-cinq règles sur une ordonnance, la plupart
  nommant une demi-douzaine de mots, contre une dizaine de traitements —
  et le tableau de bord fait tourner l'ensemble sur **chaque dossier de
  la base**. Écrit `hay.contains(&sort_key(w))`, c'était un `String` par
  mot *et par traitement*, quelque dix mille allocations par patient. Le
  repliage se fait maintenant à la volée pendant la comparaison : les
  aiguilles sont courtes, les bottes de foin font une ligne, et la passe
  y gagne environ un tiers. La biologie et la surveillance suivent la
  même route.
- **La liste d'appel se lit quand on la regarde.** Elle parcourt toute
  la base et y fait tourner les trois moteurs ; `refresh_dashboard` est
  appelé depuis dix-huit endroits, dont chaque changement d'onglet de
  l'espace de travail. Elle y est maintenant seulement *marquée à
  relire*, et c'est la vue qui la montre qui la lit.

### Fixed
- **Trois carrés creux.** L'application ne livre aucune police : elle
  dessine avec celles d'egui, dont la romaine n'a pas la flèche U+2192.
  « Saisie courte : 230826 → 23/08/2026 » s'affichait donc avec un carré
  au milieu — et la pastille « ← → » juste à côté était correcte, parce
  qu'elle est en chasse fixe, qui les a. Trois textes étaient touchés :
  celui-là, la ligne de touches du déroulé d'un protocole, et la phrase
  écrite au journal quand on enregistre un protocole parcouru.
  - Deux tests le tiennent désormais : l'un passe **chaque caractère de
    chaque chaîne livrée** dans la police qui la dessinera, l'autre fait
    de même pour les symboles que le code écrit lui-même — et il exige
    en retour que les flèches n'aient *pas* de glyphe en romain, pour
    que la règle « les flèches restent dans les pastilles » cesse d'être
    vraie le jour où elle cesse d'être nécessaire.

## [0.119.0] - 2026-08-29

Ce qui n'a pas été fait remonte au tableau de bord, et s'imprime sur le
bilan.

### Added
- **La liste d'appel parle aussi des examens qui manquent.** « À revoir »
  ne connaissait que ce que les chiffres présents disent. Elle porte
  maintenant les dossiers dont un analyte réclamé par l'ordonnance a un
  dernier résultat plus vieux que son rythme, avec la ligne qui dit
  lequel et depuis combien de mois. Un dossier sans alerte ni
  avertissement peut désormais y figurer pour cette seule raison.
  - **Seulement « à refaire », jamais « jamais noté ».** Le premier est un
    fait : on a la date, elle est dépassée. Le second est une absence de
    donnée, et une liste d'appel qui s'ouvrirait dessus serait la base
    entière le jour de l'installation.
- **« À faire vérifier » sur le bilan partagé de médication.** Une
  section de plus sur la feuille qu'on tient pendant l'entretien : l'état,
  l'analyte, le rythme, la date du dernier résultat et ce qui le demande.
  Les lignes déjà à jour n'y sont pas — une feuille tenue pendant un
  entretien est une liste de choses à faire. C'est la seule section du
  bilan qui parle de ce qui *manque*.

## [0.118.0] - 2026-08-29

Ce que l'ordonnance demande de faire vérifier, et depuis combien de temps
ça ne l'a pas été.

### Added
- **« À surveiller ».** La biologie répondait à « ce chiffre, sous ce
  traitement, qu'est-ce que ça change ». Il manquait la question d'avant,
  celle que personne ne pose : **quel chiffre n'a pas été demandé depuis
  trop longtemps**. Un INR qui alerte est un INR qu'on a fait ; le
  patient sous AVK dont le dernier INR date de neuf mois ne déclenche
  aucune règle, parce qu'il n'y a rien à lire.
  - Un second onglet dans le panneau de la vue Biologie : pour
    l'ordonnance du dossier, quels analytes elle réclame, tous les
    combien, la date du dernier résultat noté, et depuis combien de mois.
  - **48 surveillances** sur les classes courantes : l'INR mensuel de
    l'AVK, la clairance annuelle de l'AOD, les plaquettes de l'héparine,
    la kaliémie de l'anti-aldostérone, la natrémie du thiazidique, la
    lithémie trimestrielle, la TSH de l'amiodarone et de la
    lévothyroxine, la numération de la clozapine et de l'antithyroïdien,
    l'HbA1c, le LDL, l'uricémie qui dit si la dose d'hypo-uricémiant
    suffit, la calcémie **avant** l'injection de dénosumab, la
    magnésémie des IPP au long cours, la B12 de la metformine…
  - **Quatre niveaux, et un seul est une alerte.** « À refaire » est un
    fait : on a la date, elle est dépassée. « Jamais noté » est une
    absence de donnée, qui n'est pas une absence d'examen — sur une base
    qui démarre c'est presque toute la liste, et cela ne se dit pas du
    même ton.
  - Deux traitements qui demandent le même analyte font **une** ligne, au
    rythme du plus serré : c'est une prise de sang, pas deux.
  - Un clic sur une ligne charge l'analyte dans le formulaire au bas du
    tableau et affiche sa courbe : le panneau dit quoi demander, le clic
    est la façon de noter la réponse.
  - Les rythmes sont ceux des RCP et des recommandations usuelles, et
    l'espacement réel est décidé par le prescripteur : `surveillance.rs`
    ne prescrit rien, et il le dit.

### Fixed
- Deux `tab_strip` sur un même écran partageaient un identifiant écrit en
  dur dans le widget, et egui peignait « First use of ScrollArea ID … »
  en travers des deux le jour où un second est apparu. Un widget fait
  pour servir deux fois ne peut pas se nommer lui-même : il prend
  maintenant son sel en argument.

## [0.117.0] - 2026-08-29

La conciliation part chez le prescripteur, et deux recherches de plus
quittent la boucle d'affichage.

### Added
- **La fiche de conciliation, pour le prescripteur.** « Imprimer… » sur
  l'onglet Conciliation écrit une A4 : le patient, sa date de naissance,
  le médecin nommé sur le dossier, le rapprochement complet en tableau —
  statut, traitement, ce que portait le dossier, ce que porte
  l'ordonnance de sortie, la remarque — et un encadré vide pour la
  réponse.
  - Elle porte les **reconductions** aussi, et pas seulement les
    divergences : une feuille qui liste cinq changements ne dit rien des
    douze lignes qu'elle n'a pas regardées, et le médecin n'a aucun
    moyen de faire la différence.
  - L'encadré « Avis du prescripteur » est la raison de l'envoyer : la
    réponse revient sur la même feuille.
  - Une mention de plus dans `[disclaimers]` (Options › Mentions), vide
    par défaut comme les sept autres.

### Changed
- **La recherche de patients et la boîte « Aller à… » répondent une fois
  par question.** La première était demandée trois fois par image — le
  panneau de gauche, la vue de recherche, et la branche qui décide
  d'ouvrir la création —, chacune un passage sur tout le fichier et une
  copie de vingt patients. La seconde est une fenêtre egui, donc
  repeinte avec le reste de l'image : elle classait les vues, tous les
  patients, les 850 fiches, les tables, les préparations, les
  dispositifs et les protocoles, et fabriquait un `String` par résultat,
  pendant qu'on y tape trois lettres.
  - Il a fallu deux révisions de plus, et la seconde a demandé de faire
    une fonction de la ligne « recharger les protocoles », écrite neuf
    fois dans le fichier — neuf endroits où un memo bâti dessus aurait
    pu vieillir sans que personne le voie.
  - Les deux memos se reconstruisent aussi quand la liste qu'ils portent
    est **vide** : les vues les empruntent en les sortant de la session,
    et un chemin de sortie peut ne pas les rendre. « Prêté » et
    « vraiment vide » se répondent de la même façon.

### Fixed
- **Plus aucun `unwrap()` dans `src/app.rs` hors tests.** Les quatre
  derniers étaient « demander si c'est ouvert, puis déballer » sur la
  même Option — l'heure d'un rendez-vous, son déplacement, le nœud d'un
  protocole, la ligne de posologie.
- Le troisième bouton de l'onglet Conciliation faisait passer sa rangée
  à deux lignes, et poussait toutes les divergences hors du panneau à
  1024x700 : la hauteur de cette rangée est maintenant mesurée, et c'est
  la bande du bas qui cède. Les libellés ont raccourci avec.

## [0.116.0] - 2026-08-29

La conciliation médicamenteuse, la posologie du patient, et quatre
réponses que l'interface refaisait soixante fois par seconde.

### Added
- **Conciliation médicamenteuse.** Un cinquième onglet sur la fiche
  patient. On y colle l'ordonnance de sortie d'hôpital telle qu'elle est
  écrite — une ligne par traitement, la marque ou la DCI, le dosage et
  le rythme derrière — et le tableau dit de chaque traitement ce qui lui
  est arrivé : **reconduit**, **dose changée**, **arrêté**, **ajouté**,
  **remplacé** par une molécule de sa classe, ou **non rapproché**.
  - Ce qui a *disparu* de l'ordonnance est la raison d'être de l'acte :
    c'est ce que le patient continuera de prendre s'il lui reste une
    boîte et que personne ne le lui dit. Les arrêts sont donc en tête du
    tableau, avec les remplacements — la divergence dont on repart avec
    les deux boîtes.
  - Une ligne que la base ne sait pas rapprocher n'est jamais écartée en
    silence : elle est affichée telle qu'elle a été tapée, tout en haut,
    parce que personne ne l'a vérifiée.
  - Le rapprochement passe par les traitements du dossier avant la base
    entière : une ligne qui nomme quelque chose que le patient prend
    déjà désigne *ce* traitement-là, et pas un homonyme trouvé parmi les
    850 fiches. Sinon une reconduction s'afficherait comme un arrêt
    suivi d'un ajout — une divergence inventée, la pire des deux
    erreurs.
  - « Écrire au journal » verse la conciliation dans le journal du
    dossier : c'est la trace de l'acte, et elle part avec la fiche.
  - `src/conciliation.rs` est pur et testé, comme la biologie et la
    revue. Il ne décide rien : la conciliation est un acte du
    pharmacien.
- **La posologie du patient, sur son dossier.** `patient_drugs` porte une
  colonne `posology` : la ligne qui est sur *son* ordonnance, distincte
  des posologies de la molécule, qui vivent sur la fiche et valent pour
  tout le monde. Elle s'écrit dans l'onglet Conciliation, en
  compare-and-set comme toute ligne partagée, et « Reprendre les
  posologies de la sortie » la met à jour depuis la feuille.
  - Le **plan de prise** imprime désormais celle-là en priorité.
    Imprimer la posologie usuelle de la molécule sur une feuille qui
    part chez le patient, c'est lui faire lire un schéma qui n'est pas
    le sien.

### Changed
- **`fuzzy::score` n'alloue plus rien.** C'est la fonction la plus
  sollicitée de l'application — une frappe dans la recherche de
  médicaments l'interroge sur 850 fiches et quatre champs chacune, et le
  codex, les dispositifs et les tables l'interrogent sur leurs propres
  listes. Elle répondait en appelant la variante qui surligne et en
  jetant les surlignages : deux `Vec<char>` et un `Vec<usize>` par
  appel, plus un parcours de la cible entière même après que la requête
  était épuisée. Écrite comme une marche sur deux itérateurs : **3,3×
  plus rapide**, et un test tient les deux écritures ensemble.
- **Trois réponses passent de l'image à l'événement qui les change** —
  ce que le reste de l'application fait déjà des interactions et de la
  revue : les 87 règles de biologie (qui repliaient une copie du nom, de
  la DCI, de la classe et des étiquettes de chaque traitement à chaque
  image), le calendrier vaccinal (dont chaque ligne est un `format!`),
  et les adjuvants de l'ordonnance (une requête pour les fiches
  étiquetées, plus une par fiche trouvée, sur chaque image d'une boîte
  qui reste ouverte le temps de rédiger — quelque dix mille requêtes par
  minute pour une réponse qui n'avait pas bougé).

### Fixed
- Le panneau d'évolution de la biologie choisissait l'analyte le plus
  souvent mesuré dans une table de hachage : à égalité — le cas ordinaire
  d'un bilan qui apporte douze lignes datées du même jour — il en
  désignait un autre à chaque image. Compté dans l'ordre de lecture, le
  premier gagne et reste gagnant.
- Le dossier était ouvert *après* le chargement du carnet, si bien que
  le calendrier vaccinal, une fois calculé une seule fois, l'aurait été
  contre l'âge du dossier précédent.

## [0.115.0] - 2026-08-28

Six lignes que tous les bilans portent et que le catalogue n'avait pas.

### Added
- **43 → 49 analytes, 78 → 87 règles.** L'urée, le taux de prothrombine,
  le cholestérol total, les leucocytes, l'hématocrite et la
  parathormone : six lignes imprimées sur les bilans ordinaires, que les
  patients apportent au comptoir pour se les faire expliquer. Chacune
  gagne sa place par ce qu'un traitement en fait.
  - **L'urée** monte avant la créatinine quand le rein manque d'eau :
    c'est le chiffre qui attrape une déshydratation pendant qu'il suffit
    encore de suspendre le diurétique quelques jours. Élevée avec une
    créatinine peu modifiée sous AINS ou sous anticoagulant, c'est aussi
    la signature d'un saignement digestif — le sang digéré fabrique de
    l'urée.
  - **Le TP** est la face que les laboratoires français impriment, et la
    règle dit quoi en faire : sous AVK, on lit l'INR et rien d'autre.
    Hors AVK, un TP qui chute sous un médicament hépatotoxique est plus
    grave que les transaminases, parce que le foie a cessé de fabriquer
    les facteurs.
  - **Les leucocytes** : une hyperleucocytose sous corticoïde est le
    corticoïde qui démargine les polynucléaires, pas une infection — et
    c'est une cause classique d'antibiothérapie inutile, alors que la
    même molécule masque la fièvre d'une vraie. Une leucopénie sous
    clozapine ou antithyroïdien reçoit la consigne de sa classe.
  - **L'hématocrite** est l'effet qui compte sous testostérone et sous
    agent stimulant l'érythropoïèse, parce qu'il fait le risque
    thrombotique.
  - **La PTH** ne se lit qu'avec la calcémie et le phosphore, et la
    ligne du comptoir est que le chélateur du phosphore se prend au
    milieu du repas : à jeun, il ne chélate rien.
  - **Le cholestérol total** est le chiffre que le patient retient et le
    moins utile des quatre ; au-delà de 3 g/L malgré un hypolipémiant,
    il demande d'abord si le traitement est pris, puis s'il faut
    dépister la famille.

## [0.114.0] - 2026-08-28

Ce que la saison, le soleil et le pilulier font à une ordonnance, et sept
règles que la revue ne trouvait pas.

### Added
- **31 → 34 tables de référence.**
  - **« Canicule »** lit l'ordonnance contre la chaleur : le diurétique
    et l'IEC qui vident un patient qui perd déjà de l'eau, l'AINS qui
    complète la triade en insuffisance rénale aiguë, le lithium dont la
    concentration monte sans changement de dose, les anticholinergiques
    et les neuroleptiques qui suppriment la sudation — le seul
    refroidissement dont dispose le corps —, le patch qui délivre
    davantage quand il fait chaud, et le stylo à insuline oublié dans
    une voiture. Chaque ligne porte une colonne « ce qu'on ne fait
    pas » : l'essentiel des accidents de canicule vient d'un traitement
    arrêté ou doublé de sa propre initiative.
  - **« Soleil »** distingue les photosensibilisants par mécanisme,
    parce que le conseil en dépend : la doxycycline brûle en quelques
    heures et proportionnellement à la dose, l'éruption sous
    cotrimoxazole est retardée et n'est pas toujours un coup de soleil,
    le kétoprofène en gel reste dangereux deux semaines après la
    dernière application — la seule règle de la table qui survit au
    traitement —, et le thiazidique est un risque qui se compte en
    décennies et non en vacances.
  - **« Pilulier »** dit ce qui ne se déconditionne pas : le Pradaxa
    hors de sa plaquette, les effervescents et les orodispersibles qui
    prennent l'humidité, la trinitrine sublinguale dont le principe
    actif est volatil, la dose « si besoin » qu'une semaine en cases
    transforme en prise systématique, et le pilulier préparé le lundi
    qui porte encore l'ordonnance du lundi le mercredi.
- **48 → 55 règles de revue d'ordonnance.** Le bêtabloquant caché est
  celle qu'il a fallu écrire avec soin : « bêtabloquant » désigne aussi
  le collyre, et une combinaison laisse un seul traitement satisfaire ses
  deux groupes — un patient sous Timoptol seul se serait vu annoncer
  qu'il en prenait deux. La moitié orale est donc une liste de
  molécules, et un test la tient. Les six autres : la contraception sous
  inducteur enzymatique, qui échoue en silence jusqu'au test de
  grossesse et où le millepertuis acheté sans ordonnance compte comme la
  carbamazépine ; l'anticoagulant oral direct avec un inhibiteur de la
  P-gp, dont l'exposition monte sans INR pour le dire ; la
  corticothérapie avec un antidiabétique ; trois corticoïdes comptés
  toutes voies confondues ; l'AVK avec l'amiodarone, dont l'INR s'emballe
  une à trois semaines après l'introduction ; et le thiazidique avec le
  calcium et la vitamine D, dont l'hypercalcémie lente se lit comme de la
  vieillesse.

## [0.113.0] - 2026-08-28

Six molécules et six préparations que la base ne savait pas traiter, et
la suite des tests qui cesse de laisser ses bases dans `/tmp`.

### Added
- **844 → 850 fiches.** Six molécules, et non six marques de plus pour
  des molécules déjà présentes — c'est la pente à ne pas suivre, et le
  relevé des manques a dû être refait une fois : chercher « Anoro » dans
  une liste qui contient « Anoro Ellipta » avait déclaré manquante une
  fiche présente depuis toujours.
  - **Eklira Genuair** (aclidinium) est le seul anticholinergique inhalé
    à deux prises par jour : un patient qui vient du tiotropium oubliera
    celle du soir si personne ne le lui dit. Sa fenêtre de contrôle est
    verte quand la dose est prête et rouge quand elle a été inhalée —
    verte après l'inspiration, la dose n'est pas passée, et cela se
    vérifie au comptoir.
  - **Alvesco** (ciclésonide) n'est actif qu'une fois hydrolysé par les
    estérases du poumon : il se dépose peu dans la gorge, donc beaucoup
    moins de candidoses et d'enrouements. C'est sa raison d'être et la
    raison de le connaître.
  - **Zaditen collyre** (kétotifène) tient le rayon de la conjonctivite
    allergique : antihistaminique et antidégranulant à la fois, et le
    chlorure de benzalkonium du flacon multidose est ce qui fait donner
    les unidoses à un porteur de lentilles.
  - **Maxilase** (alpha-amylase) est le produit du mal de gorge le plus
    demandé de France, et la fiche dit ce qu'il vaut : un appoint, un
    service médical rendu jugé insuffisant, cinq jours au maximum — la
    durée et le score de Mac Isaac sont ce qui compte vraiment.
  - **Vydura** (rimégépant) ouvre la classe des gépants, que la base
    n'avait pas : pas de vasoconstriction, donc l'option quand un
    antécédent cardiovasculaire interdit les triptans.
  - **Nilemdo** (acide bempédoïque) est ce qui reste quand la statine ne
    passe pas — activé dans le foie et pas dans le muscle, c'est tout
    son argument — avec la crise de goutte comme effet à annoncer.

  Deux règles de conduite les accompagnent (« anti-inflammatoire
  enzymatique », « gépant »).
- **52 → 58 préparations.** L'établi de la pédiatrie, d'abord :
  suspensions de captopril, de furosémide et de propranolol — la
  première instable au-delà d'une semaine, la deuxième photosensible, la
  troisième donnée pendant la tétée parce que l'hypoglycémie d'un
  nourrisson bêtabloqué se manifeste par une somnolence et non par des
  sueurs. Puis les gouttes auriculaires au bicarbonate pour le bouchon
  de cérumen, les gélules de mélatonine, et une lotion mentholée dont la
  contre-indication est ce qu'il faut retenir : jamais sur le visage
  d'un enfant de moins de trente mois.

### Fixed
- **La suite de tests ne laisse plus ses bases dans `/tmp`.** Chaque
  test travaille sur un vrai fichier SQLCipher dans un répertoire nommé
  d'après le processus, et aucun ne l'effaçait : un `cargo test` en
  fuyait quarante-cinq, une machine qui lance la suite quelques dizaines
  de fois par jour en accumulait six gigaoctets, et la suite finissait
  par échouer sur « database or disk is full » — ce qui se lit comme un
  bug du code testé et non comme les restes des passes précédentes.
  `Swept` est un répertoire qui s'efface à la fin du test, y compris
  quand il panique.

## [0.112.0] - 2026-08-28

La carte Vitale ouvre la fiche, et deux ajouts de contenu que le
comptoir attendait.

### Added
- **Lecture de la carte Vitale.** Un lecteur PC/SC sur le poste, et
  présenter la carte devient une façon de nommer un patient : les
  bénéficiaires de la carte s'affichent sous le champ de recherche — le
  titulaire et ses ayants droit, parce que la personne au comptoir n'est
  pas toujours le titulaire — et un clic ouvre la fiche qui porte ce
  NIR, ou en pré-remplit une nouvelle avec le nom, le prénom et la date
  de naissance.

  Trois décisions valent d'être écrites, parce qu'elles portent toute la
  fonction :

  - **C'est le NIR qui rapproche, pas le nom.** Deux personnes portent
    le même nom, personne ne partage un numéro. Une fiche ouverte sur
    une homonymie est une erreur qui ne se voit pas.
  - **Rien n'est lu à un offset.** Les octets de la carte sont
    parcourus à la recherche d'une suite de quinze chiffres dont la clé
    de contrôle se vérifie — une chance sur quatre-vingt-dix-sept qu'une
    suite quelconque passe, et les noms autour confirment. La
    disposition des fichiers change d'une génération de carte à
    l'autre : un offset deviné lirait un patient plausible et faux, et
    c'est la seule panne que ce module n'a pas le droit d'avoir. Une
    date trouvée à côté ne devient une date de naissance que si elle
    s'accorde avec le mois et l'année que le NIR porte lui-même.
  - **La bibliothèque PC/SC du système est ouverte par son nom au
    moment où une carte est demandée, jamais liée au binaire.** Un
    exécutable lié à `libpcsclite` ne démarre pas du tout sur un poste
    qui ne l'a pas — l'éditeur de liens échoue avant `main`, et
    l'utilisateur ne voit qu'une fenêtre qui ne s'ouvre jamais. Une
    officine sans lecteur est le cas courant, pas l'exception, et elle
    garde l'application qu'elle avait.

  La séquence de commandes APDU est en configuration (Options › Base) et
  non dans le programme : elle relève des spécifications SESAM-Vitale,
  qui sont sous licence et versionnées. Vide, l'application liste quand
  même les lecteurs et affiche l'ATR — de quoi répondre à « le lecteur
  et la carte sont-ils vus » sans développeur sur place. Les échecs du
  comptoir sont nommés en français : service non démarré, aucune carte,
  carte retirée, carte déjà prise par un autre logiciel.

  **Elle ne facture rien.** Une feuille de soins électronique demande un
  progiciel SESAM-Vitale agréé, une carte CPS et un concentrateur ; le
  LGO reste l'outil qui facture, et rien ici ne prétend le contraire.
- **Thyrozol et douze biosimilaires : 831 → 844 fiches.** Le thiamazole
  manquait à côté du carbimazole, alors que la règle d'agranulocytose de
  la biologie le nommait déjà. Les douze biosimilaires — Amgevita,
  Hyrimoz et Imraldi (adalimumab), Benepali et Erelzi (étanercept),
  Zarzio et Nivestim (filgrastim), Ziextenzo (pegfilgrastim), Semglee
  (insuline glargine), Retacrit (époétine zêta), Terrosa (tériparatide)
  et Inhixa (énoxaparine) — gardent la classe de leur référent
  (« anti-TNF alpha », « HBPM », « insuline lente »…), de sorte que
  toutes les règles déjà écrites — biologie, revue d'ordonnance, conduite
  en cas d'oubli — s'appliquent sans être réécrites.
- **Table « Biosimilaires » : 30 → 31 tables.** Ce qui se substitue au
  comptoir et ce qui ne s'y substitue pas, pourquoi « générique » est le
  mot à ne pas employer, pourquoi un biologique se trace par sa marque
  **et** son numéro de lot et jamais par sa DCI seule, et pourquoi la
  technique d'injection se remontre à chaque changement de spécialité —
  le stylo n'est pas le même stylo. Quatre colonnes, dont « Le piège ».

### Changed
- `scripts/smoke.sh` ouvre trente-cinq vues au lieu de trente-quatre :
  la lecture de carte est rejouée sur un relevé capturé
  (`BPM_CADDY_VITALE_DUMP`), donc tout le chemin tourne à chaque passe —
  sans matériel, sans carte réelle, sans l'identité de personne.

## [0.111.0] - 2026-08-28

Une seconde passe de contenu : ce qui manquait aux catalogues, et la
section que la feuille de route avait laissée ouverte.

### Added
- **« Toxicité / marge thérapeutique » : 40 → 64 fiches.** La ligne
  restée ouverte depuis la v0.85.0, où elle était dite « éditoriale,
  fiche par fiche ». Les vingt-quatre ajoutées sont celles où une dose,
  une durée ou une exposition tue : tricycliques (cardiotoxiques avant
  d'être neurologiques, sans antidote, chez la population même qui fait
  des gestes), théophylline (toxique dès 20 mg/L, et arrêter de fumer
  suffit à l'y amener), hydroxychloroquine (un comprimé peut tuer un
  enfant), ciclosporine et tacrolimus, insuline et sulfamides
  (l'hypoglycémie du sulfamide récidive vingt-quatre à quarante-huit
  heures après le resucrage), baclofène, inhibiteurs calciques,
  potassium, isotrétinoïne, vancomycine et aminosides, anesthésiques
  locaux, méthadone, fentanyl transdermique (le patch délivre encore
  après le retrait, et la chaleur augmente l'absorption), oxycodone LP,
  allopurinol et cotrimoxazole (hypersensibilité retardée, sans rapport
  avec la dose), valproate et amiodarone.
- **Dispositifs : 47 → 55.** Pansement hémostatique, orthèse de poignet,
  bas anti-thrombose, poche de stomie vidable contre fermée, seringues
  et aiguilles, nébuliseur et son consommable, pilulier, automesure de
  la tension et de la température.
- **Revue d'ordonnance : 40 → 48 règles.** Méthotrexate +
  cotrimoxazole, lévothyroxine chélatée, digoxine potentialisée,
  syndrome sérotoninergique (le tramadol y est le plus souvent en cause
  parce qu'il passe pour un simple antalgique), PDE5 avec un donneur de
  NO, fluoroquinolone avec un corticoïde et le tendon d'Achille, statine
  avec colchicine, et un corticoïde inhalé sans bronchodilatateur de
  secours.
- **Biologie : 62 → 78 règles.** La moitié du catalogue ne portait
  qu'une règle, soit le minimum exigé et non ce à quoi le module sert.
  Parmi les nouvelles : une créatinine qui monte sous triméthoprime,
  dolutégravir ou cimétidine n'est pas une insuffisance rénale ; une
  HbA1c *basse* sous sulfamide après 75 ans est un risque et non un bon
  résultat ; une anémie sous AINS ou anticoagulant est un saignement
  digestif jusqu'à preuve du contraire.
- **Quatre thématiques d'entretien** (12 en tout), chacune avec sa
  liste : sortie d'hôpital et conciliation, douleur chronique, sommeil,
  chute et autonomie.
- **Huit fiches du rayon conseil** (831 en tout) : pseudoéphédrine,
  oxymétazoline, flurbiprofène en pastille, tétracaïne, montmorillonite,
  fluticasone nasale, acide méfénamique, benzydamine.

### Changed
- Un cliquet de plus sur chaque catalogue ajouté, et les noms en double
  refusés partout.

## [0.110.0] - 2026-08-28

Une passe de contenu clinique, et un cliquet sur chaque catalogue.

### Added
- **Dispositifs médicaux : 35 → 47.** Pansement siliconé, mèche de plaie
  cavitaire, pansement à l'acide hyaluronique, bande cohésive, manchon
  du lymphœdème, alèse de literie, set de perfusion sous-cutanée, poche
  de recueil de nuit, lecteur de glycémie, capteur de glucose en
  continu, débitmètre de pointe, concentrateur d'oxygène en location.
  Chacun dit ce qui n'est pas sur la boîte : une bande cohésive posée en
  l'étirant fait un garrot dans l'heure, le glucose interstitiel retarde
  de cinq à quinze minutes sur la glycémie capillaire, l'oxygène fait
  flamber les corps gras, et une mèche oubliée au fond d'une cavité se
  retrouve des semaines plus tard.
- **Revue d'ordonnance : 28 → 40 règles**, et un nouveau genre de règle.
  `Kind::Without` se déclenche quand tous les termes sont présents **et**
  que rien sur l'ordonnance ne répond au dernier groupe : ce qui *manque*
  est la moitié de ce qu'un bilan trouve. Un opioïde sans laxatif, une
  corticothérapie sans rien pour l'os — et la règle se tait dès que la
  ligne manquante apparaît. Les dix autres : deux sources de paracétamol
  sous deux noms différents, statine + macrolide ou azolé, deux
  anticoagulants, allopurinol + azathioprine, le triple risque
  hyperkaliémique, AVK + antibiotique, la règle des jours de maladie
  sous metformine, la double antiagrégation qui a une durée, le
  bêtabloquant qui masque l'hypoglycémie, l'AINS qui dérègle la tension.
- **Biologie : 34 → 43 analytes et 47 → 62 règles.** HDL, bicarbonates,
  NT-proBNP, PSA, coefficient de saturation de la transferrine, T4 libre,
  éosinophiles, lymphocytes, activité anti-Xa — chacun avec sa règle. Le
  PSA sous inhibiteur de la 5-alpha-réductase est divisé par deux et doit
  être doublé pour être lu ; la lymphopénie sous traitement de fond de la
  SEP a un seuil auquel on suspend ; l'anti-Xa ne veut rien dire hors du
  pic à quatre heures.
- **Trois tables de référence** (30 en tout) : les dermocorticoïdes avec
  la règle de l'unité phalangette et la quantité que chaque partie du
  corps réclame vraiment ; les trois niveaux du pictogramme de conduite,
  avec ce qu'ils ne disent pas — le risque d'un hypnotique n'est pas la
  nuit mais le trajet du matin ; et « aliments et médicaments », dont
  une colonne entière dit ce qui n'est **pas** vrai, parce que le
  comptoir passe autant de temps à défaire des croyances qu'à conseiller.
- **Codex : 42 → 52 préparations**, dont six soins de bouche — le bain de
  bouche bicarbonaté qui se répète six à huit fois par jour justement
  parce qu'il n'a ni alcool ni antiseptique, l'amphotéricine B, le bain
  anesthésique qui se prend quinze minutes avant le repas et non pendant.
  Plus la suspension d'oméprazole, où le bicarbonate remplace l'enrobage
  gastro-résistant qu'il a lui-même dissous.
- **Protocoles : 28 → 36.** Conjonctivite, lombalgie aiguë, vomissements
  de l'enfant, éruption sous traitement, automédication après 75 ans,
  retard de règles, nourrisson qui pleure — et « ordonnance illisible,
  douteuse ou périmée », le premier arbre qui porte sur l'acte de
  délivrance et non sur un symptôme.

### Changed
- **La couverture des posologies passe de 546 à 697 fiches sur 823**, et
  le plafond du cliquet de 170 classes à 34. 151 lignes écrites, dont
  celles des biologiques que le patient s'injecte chez lui — un Stelara
  délivré sans ses douze semaines est un rendez-vous manqué. Ce qui
  reste n'est plus une dette d'écriture : les 34 fiches sans posologie
  sont injectées ou perfusées par quelqu'un d'autre, et inventer une
  posologie de comptoir serait pire que le blanc. Elles ne sont
  délibérément pas versées dans la liste des exemptions — une liste
  d'exemptions blanchirait la dette, et l'intérêt du cliquet est
  précisément qu'elle reste visible.
- **Chaque catalogue a son cliquet** : fiches, tables, préparations,
  dispositifs, protocoles, analytes et règles ont désormais un plancher
  qui ne peut que monter, et les noms en double sont refusés partout.
  C'est la troisième règle de `docs/CONTENU.md`, qui n'était jusqu'ici
  suivie que par habitude.

## [0.109.0] - 2026-08-28

Les quatre lignes ajoutées à la feuille de route, et les deux de données.

### Added
- **Six palettes X/Motif**, choisies dans Options › Interface : Motif
  (le bleu-gris de mwm), CDE, DECwindows, Indigo Magic, HP VUE, et
  « Contraste » pour un comptoir en plein soleil. Un thème ne porte que
  les couleurs : angles droits, biseaux de deux pixels, widgets en
  relief et cuvettes en creux sont ce qui fait le Motif, et aucune peau
  n'a son mot à dire là-dessus. Les huit couleurs de la palette
  s'affichent à côté de la liste, avant d'enregistrer. `[ui] theme` dans
  config.toml ; un nom inconnu n'est pas une erreur, c'est la palette
  classique — un fichier rapporté d'une version plus récente ne laisse
  jamais l'officine devant une fenêtre blanche.
- **Tracer une entrée sur le plan de la journée.** Balayer de 14 h à
  16 h remplit la ligne « Ajouter » avec ces deux heures et met le
  curseur dans le titre : le geste répond à *quand*, le clavier à
  *quoi*. Les heures s'accrochent au quart d'heure. Une entrée a
  maintenant une fin (`end_time`), et son bloc est dessiné jusqu'à
  l'heure où il s'arrête — la formation de deux heures ressemble à deux
  heures, sur l'écran comme sur le plan de semaine imprimé.
- **Le déroulé d'un protocole s'écrit sur la fiche.** Il garde le chemin
  parcouru et l'affiche au-dessus de l'étape en cours ; à la fin, avec
  un patient ouvert, un clic écrit le protocole suivi, les réponses
  données et la conduite atteinte dans son journal, daté et signé comme
  toute note. Une décision prise au comptoir appartient au dossier.
- **Dix préparations de plus au codex** (42) : liniment oléo-calcaire,
  eau de Dalibour, lotion à la calamine, pâte de Lassar, permanganate à
  1/10 000, chlorhexidine alcoolique à 0,5 %, SRO de l'OMS,
  suppositoires à excipient semi-synthétique, pommade à la trinitrine de
  la fissure anale, crème au métronidazole des plaies malodorantes.
- **Dix fiches de plus** (823) : Telfast, Wystamm, Phénergan, Nautamine,
  Sibélium, Jamylène, Surbronc, Biltricide, Veinamitol, Dexeryl —
  monographie complète, lignes de posologie et mise en garde du comptoir
  pour chacune.
- **Huit protocoles de plus** (28) : toux, constipation, douleur
  dentaire, insomnie demandée, mycose vaginale, chute chez la personne
  âgée, voyage (trousse et ordonnance), interaction repérée à la
  délivrance.

### Changed
- **Les tables de conversion ont une liste et non un mur de boutons.**
  Le sélecteur était vingt-sept boutons indistincts en travers du haut
  de page — six rangées à la largeur d'un comptoir, ce qui laissait une
  ligne de la table au-dessus de la ligne de flottaison. C'est un
  panneau à gauche, dans les cinq familles où le module était déjà
  organisé mais en prose seulement : Équivalences, Posologies,
  Adaptation, Au comptoir, Administration. Chaque ligne lit
  « IPP *9 lignes* » et marque les tables que l'équipe a corrigées.
- **Un protocole se lit avant de s'écrire.** Les commandes
  « + si oui / + si non / × / + Question » passent derrière
  « Modifier » ; la branche devient une pastille colorée au lieu de deux
  mots gris qui diffèrent d'une lettre, et chaque niveau tire un filet
  dans la gouttière pour qu'on suive une branche jusqu'à la question qui
  l'a ouverte.

### Fixed
- Une table plus large que son volet ne part plus vers la gauche : la
  première colonne — celle qui nomme la ligne — restait hors écran sans
  moyen d'y revenir. Et le volet apprend enfin que la table dépasse :
  `allocate_new_ui` ne réserve aucune place, donc l'ascenseur horizontal
  n'existait pas et les colonnes de droite étaient dessinées sans être
  atteignables.
- La bande de titre des tables est mesurée : à 1024 px avec le texte à
  1,25 elle prend trois lignes, et une bande devinée coupait le champ de
  recherche en deux derrière la liste.

## [0.108.0] - 2026-08-28

Toute la liste `UI:` de la feuille de route, et la passe de robustesse
et de performance qui la clôt.

### Added
- **Options › À propos** : ce qu'est cette installation (version,
  plateforme, version posée par le lanceur, chemin et taille de la base,
  ce qu'elle contient), puis « Vérifier la dernière version… » qui
  interroge GitHub sur un fil à part — la seule requête réseau de
  l'application, et seulement sur clic. Les autres boutons « … »
  continuent de passer une adresse au navigateur et rien de plus. La
  comparaison se fait par nombres et non par texte : `v0.10.0` se range
  sous `v0.9.0` comme chaîne, et une officine à qui l'on dit qu'elle est
  en avance sur une version qu'elle n'a pas ne regarde plus jamais.
- **« Synchroniser le contenu de référence »** : le contenu de référence
  voyage *dans* le binaire, donc se mettre au niveau de la dernière
  version, c'est mettre à jour l'application (le lanceur le fait à
  chaque démarrage) puis verser dans la base ce que cette version
  apporte et qui manque. Le bouton fait la seconde moitié, la
  vérification dit si la première est due. Rien de ce que l'équipe a
  écrit n'est réécrit, et une seconde pression n'ajoute rien.
- **Arbre A–Z dans la liste de gauche.** 813 fiches ne sont pas une
  liste qu'on fait défiler ; ce sont vingt-sept tiroirs qu'on ouvre. La
  recherche remplie donne la liste plate des correspondances comme
  avant ; vide, elle donne l'arbre, le tiroir de la fiche ouverte
  toujours déplié. Dans les deux cas une ligne se lit « Aclasta *acide
  zolédronique* » — le nom est ce qui est sur la boîte, la DCI est la
  question qu'on se posait, et c'est la DCI qui est coupée quand le
  panneau est rétréci.
- **Catalogue de locations fourni** : les six matériels et le libellé de
  la ligne LPP de chacun, comme les six fiches de la famille
  « Location ». Ce qui est fourni à zéro, c'est l'euro : un forfait
  encore à zéro affiche « forfait à compléter » en rouge plutôt que
  « 0 € par semaine », qui se lirait comme un tarif décidé par
  l'application. Options › Locations reçoit « Compléter le catalogue ».
- **La forme dans laquelle le poste a été laissé** est enregistrée dans
  `layout.toml` : l'état des deux panneaux, le contenu du panneau de
  droite, la vue à l'écran, et la version qui a écrit tout cela. Les
  options de démarrage décident du premier lancement et de rien après.
- **Base trouvée au premier lancement** : sans chemin écrit et sans rien
  à l'emplacement par défaut, le répertoire du lanceur et celui de
  l'exécutable sont regardés avant qu'on demande un mot de passe pour
  une base qui n'existe pas — et l'écran de verrouillage dit que le
  chemin a été trouvé, pas configuré. Sans cela : on tape le mot de
  passe maître et on obtient un fichier vide tout neuf à côté de la
  vraie base de l'officine.

### Fixed
- **Le choix d'analyte en biologie n'est plus coupé en bas.** Sa bande
  valait 58 px devinés — une ligne d'un formulaire qui en fait deux sur
  une fiche étroite, et la ligne de suggestions en dessous, seul moyen
  de choisir un analyte, était la moitié qui tombait. Mesurée, le
  formulaire l'emportant sur le tableau et défilant dans sa part quand
  même cela ne suffit pas. La bande du volet passe de 240/90 px à douze
  et cinq *lignes* : `text_scale = 1,25` ne coûte plus rien.
- **La correction d'une vaccination n'est plus coupée à gauche et à
  droite** : la ligne échange six colonnes contre six champs et deux
  boutons, plus large que le volet dès qu'un panneau est ouvert, et
  l'ascenseur n'était que vertical.
- **Cliquer une fiche à gauche l'affiche.** Avec le codex, les
  dispositifs, les tables, les protocoles ou la recherche plein texte
  ouverts, la fiche s'ouvrait derrière eux et rien ne bougeait à
  l'écran ; chacun de ces écrans prend tout le panneau central.
- **Les fenêtres de réglages assombrissent l'espace de travail** et
  celui-ci cesse de répondre à la souris.

### Changed
- **110 textes réécrits en notes professionnelles.** « Aucun résultat
  enregistré. Ajoutez-en un ci-dessous : choisissez l'analyte, tapez la
  valeur, la date si ce n'est pas aujourd'hui. » devient « Aucun
  résultat enregistré. » Un pharmacien n'a pas besoin qu'on lui explique
  comment taper dans un champ qu'il regarde. Les faits restent — les
  réserves cliniques, ce que veut dire une ligne modifiée ailleurs, ce
  dont dépend un tarif — et le tutoriel autour s'en va. Le champ de
  recherche des tables compte les tables au lieu d'annoncer un nombre
  qui dériverait.
- **Le travail refait soixante fois par seconde ne l'est plus.** La
  recherche dans les tables (une requête et quelque sept mille
  allocations par image), la recherche de médicaments (trois mille
  correspondances floues, deux fois par image), l'année demandée à la
  base par le carnet à chaque image, et les 813 fiches recopiées par
  l'arbre de gauche : toutes gardées d'une image à l'autre et refaites
  quand la question ou la base change.
- **Cinq panics de moins** : `partial_cmp().unwrap()` sur des hauteurs
  calculées (un NaN en amont et l'application disparaît, sur une
  question de mise en page), et quatre `unwrap()` sur une fiche ou une
  ligne de carnet ouverte — inatteignables aujourd'hui, et pas dignes
  d'un plantage au comptoir le jour où l'un d'eux cesserait de l'être.
- `src/release.rs` rejoint les modules de logique de `coverage.sh` ;
  le chiffre reste à 87,1 %.

## [0.107.0] - 2026-08-28

### Fixed
- **Les cinq portes de la base ne sont plus posées en travers de « Base
  médicaments »** : elles ne partagent la ligne du titre que si elles y
  tiennent, mesurées et non devinées. Le seuil était « plus large que
  620 px », un test que la page — plafonnée à 720 — passait à toutes les
  tailles de fenêtre, alors que les cinq boutons ensemble sont plus
  larges que la page : aucune largeur ne marchait. Sur leur propre
  ligne, elles passent maintenant sous le sous-titre, qui reste collé au
  titre qu'il explique.

## [0.106.0] - 2026-08-28

### Changed
- **Les captures du README montraient une application plus ancienne que
  celle qui est livrée** : elles dataient d'avant la refonte des vues.
  Les douze sont refaites, et trois s'ajoutent — protocoles taillés,
  dispositifs, locations.
- **Le pointeur ne traîne plus dans l'image.** Xvfb gare le pointeur au
  centre de l'écran, c'est-à-dire au milieu de la fenêtre : chaque
  capture revenait avec l'infobulle de ce qui se trouvait dessous. L'
  écran virtuel fait maintenant trois fois la largeur de la fenêtre,
  celle-ci reste à gauche, et l'image est recadrée — le pointeur ne
  survole plus rien.

## [0.105.0] - 2026-08-28

### Changed
- **Ctrl+F va au champ de la liste ouverte.** Les flèches et Entrée
  parcourent les listes taillées depuis la 0.103 — mais seulement une
  fois le curseur dans leur champ, et l'y mettre était la seule étape
  qui demandait encore la souris. Avec les tables, les protocoles, le
  codex ou les dispositifs ouverts, Ctrl+F y va ; ailleurs il fait ce
  qu'il a toujours fait, la recherche patient. Le chemin complet se fait
  donc au clavier : Ctrl+F, on tape, flèches, Entrée.

## [0.104.0] - 2026-08-28

### Added
- **Le déroulé d'un protocole se répond au clavier.** C'est un arbre
  qu'on parcourt en parlant à quelqu'un : reprendre la souris à chaque
  « oui » est exactement la friction qu'il existe pour supprimer. **O**
  ou **←** répondent oui, **N** ou **→** répondent non, **Entrée** ou
  **Espace** passent à la suite. Pas pendant qu'un champ a le clavier —
  le titre au-dessus en est un. La ligne de rappel est sous les boutons.
- **Alt + flèches change d'onglet sur la fiche patient.** Entretiens,
  Vaccinations, Biologie, Locations : quatre moitiés d'un même dossier,
  et trois n'avaient aucun chemin au clavier. Alt, pour que les flèches
  nues continuent de piloter le tableau des actes et l'agenda. Le
  raccourci est listé dans F12.

## [0.103.0] - 2026-08-28

### Added
- **Les trois listes taillées répondent enfin au clavier.** Protocoles,
  préparations et dispositifs avaient un champ de recherche et rien
  d'autre : il fallait la souris pour ouvrir la ligne trouvée. Elles
  répondent maintenant aux trois touches auxquelles la liste des
  patients répond depuis toujours — on tape, les flèches parcourent,
  Entrée ouvre — et seulement pendant que leur propre champ a le focus,
  pour que les flèches continuent de piloter l'agenda et le tableau des
  actes partout ailleurs. La ligne ouverte reste marquée ; le curseur
  clavier marque là où Entrée irait, ce qui n'est pas toujours la même.
- Le raccourci est écrit dans F12 et dans le mode d'emploi imprimé.

### Changed
- L'arithmétique du curseur est sortie dans une fonction pure et testée.
  Le cas qui compte est celui qu'aucune capture d'écran ne montre : le
  curseur resté au-delà de la fin après que la recherche a réduit la
  liste — vingt protocoles filtrés à trois, curseur sur le douzième.
  Il revient dans la liste **avant** que la touche ne s'applique, donc
  Entrée ouvre la dernière ligne et non une fiche périmée.

## [0.102.0] - 2026-08-28

### Fixed
- **Un plantage à une retouche de distance.** Le partage entre le
  tableau des actes et le journal se termine par un `clamp`, et
  `f32::clamp` ne borne pas : il panique quand le minimum dépasse le
  maximum. Sur un volet court, le plancher du journal passe bel et bien
  au-dessus des deux cinquièmes qui lui servent de plafond. Les chiffres
  livrés jusqu'ici passaient à côté par chance ; en essayant de relever
  ce plancher, toute l'application est tombée — `min = 128,6,
  max = 111,7`. Le plafond est désormais remonté au plancher plutôt que
  supposé au-dessus.
- Les deux autres `clamp` à bornes calculées ont été relus :
  `column_count` a un minimum littéral, et l'heure de fin de journée
  d'agenda est déjà protégée par un `min(23)` sur l'heure de début — un
  `day_start_hour = 24` dans config.toml ne fait rien tomber.

### Checked
- **Densité compacte** balayée à 1024x700 : rien à signaler, c'est le
  réglage qui fait tenir davantage. Les quatre axes de la passe — 1024,
  1280, échelle du texte 1,25 et panneaux tirés au large — sont couverts.

## [0.101.0] - 2026-08-28

### Fixed
Dernier axe de la passe : **les deux panneaux latéraux tirés au large**.
C'est le défaut que CLAUDE.md décrit — « un dock qui a grandi au-delà de
la largeur qu'il s'était réservée laisse la vue centrale disposée plus
large qu'elle n'est visible » — et il se produisait bel et bien.

- **Les panneaux étaient plafonnés chacun de son côté**, à 30 % et 36 %
  de la fenêtre. Chacun pour soi, jusqu'à ce que les deux soient tirés :
  le travail au milieu — la raison pour laquelle les deux autres sont à
  l'écran — tombait alors à 40 % et ses tables se dessinaient sous eux.
  Ils se plafonnent maintenant **l'un contre l'autre** : quoi qu'on
  tire, le centre garde 560 px. À 1280, les six colonnes d'une table de
  référence redeviennent lisibles au lieu de passer sous le panneau
  d'équipe.
- **Le bouton « Ajouter » du journal passait à travers le bas de son
  panneau** dans la même situation : le puits gardait 28 px quoi qu'il
  reste. Il n'a plus de plancher — il peut se réduire à un filet, il
  défile — et c'est la ligne où l'on écrit qui reste entière.

### Non retenu
- Une tentative de faire défiler la table de conversion horizontalement
  a été annulée : `allocate_new_ui` ne réserve pas de place, la barre
  n'apparaissait donc jamais, et le contenu débordait toujours. Plafonner
  les panneaux règle le problème à la source, et pour toutes les vues.

## [0.100.0] - 2026-08-28

### Fixed
Une passe sur l'axe qu'on n'avait jamais essayé : **« échelle du
texte » à 1,25**, ce que l'officine met quand l'écran du comptoir se lit
debout. Les hauteurs réservées étaient en pixels alors que le texte, lui,
grandissait — donc plus le réglage sert, moins les vues tiennent.

- **Le tableau des entretiens reperdait sa ligne** à 1,25 : le journal
  gardait son plancher de 170 px, à peine trois de ses propres lignes à
  cette échelle, pendant que le tableau au-dessus en demandait un quart
  de plus. Les deux planchers se comptent maintenant **en lignes** — la
  répartition est la même à toutes les échelles.
- **« Voyage » perdait son bouton « Retirer »** sous le bord du panneau,
  pour la même raison : son plancher de 96 px valait cinq lignes à
  l'échelle normale et quatre à 1,25.
- Vérifié aux deux échelles et aux deux tailles : rien n'a régressé à
  1,0.

Les correctifs des versions précédentes tiennent d'eux-mêmes à 1,25,
parce qu'ils mesurent (`button_width`, `wrapped_rows`) au lieu de
supposer — ce qui est l'argument pour mesurer.

## [0.99.0] - 2026-08-28

### Changed
- **Les champs de titre d'un protocole étaient fixes** — 260 px pour le
  nom, 200 pour le sujet — quelle que soit la largeur du panneau :
  « Allergie à la pénicilline annoncée au comptoir » s'y lisait aux deux
  tiers. Ils sont proportionnels, et le sujet passe à la ligne quand les
  deux ne tiennent pas ensemble, ce à quoi sert une rangée qui se replie.
- **Les invites des trois champs « Créer » étaient tronquées** en plein
  mot : « Nom du protocole (ex. : AOD indisp… ». Elles disent le nom et
  s'arrêtent là — l'exemple entre parenthèses n'y tenait pas et se
  devine.

## [0.98.0] - 2026-08-28

### Fixed
La même passe à **1280x800**, la taille d'un portable de comptoir, et
celle qui tombe juste sous le seuil de 1320 px où le tableau des actes
et le journal se mettent côte à côte. Trois vues y coupaient une ligne
en deux — ce qui se lit comme une panne, alors qu'une ligne manquante ne
se lit que comme une liste à faire défiler.

- **Le tableau des entretiens montrait une demi-ligne**, boutons et
  listes déroulantes tranchés par le bord du panneau. La réserve qu'il
  garde avant que le journal ne se serve se mesure maintenant sur la
  hauteur réelle d'une ligne, qui est plus haute qu'une ligne de texte
  puisqu'elle est pleine de contrôles.
- **Le bouton « Ajouter » de la journée d'agenda sortait du panneau.**
  La réserve laissée au champ de titre était un nombre rond qui avait
  oublié la largeur du bouton lui-même. Elle est la somme des contrôles
  fixes.
- **Le carnet de vaccination** : le formulaire garde ce qu'il a mesuré,
  le tableau prend le reste, et « À faire » et « Voyage » lui laissent
  240 px avant de se servir — leur propre plancher est à 96, de quoi
  tenir leur bouton et une ligne. Résultat aux deux tailles : le
  formulaire entier, l'en-tête du tableau entier, et une ligne de
  vaccination à 1280.

## [0.97.0] - 2026-08-28

### Fixed
Suite de la passe à 1024x700 : trois vues qui repoussaient hors du cadre
ce qu'on venait y chercher.

- **Les tables de conversion montraient une ligne de table.** Vingt-sept
  boutons de sélection prennent six rangées à cette largeur, et la table
  elle-même commençait sous le pli. Le sélecteur est mesuré et plafonné
  à trois rangées : au-delà il défile dans sa propre boîte, et la table
  — ce pour quoi la vue existe — montre trois lignes au lieu d'une.
- **Le codex effaçait le nom de ses préparations.** La forme galénique
  était accolée au nom puis le tout tronqué : dans une colonne de
  200 px, « Bain de bouche à la bétaméthasone » devenait « Bain de
  bouche à la … ». Le nom prend la ligne, la forme passe au survol —
  comme les protocoles depuis la 0.94.
- **Le bouton « Ajouter » des notes sortait du panneau** sur une fiche
  médicament, où la boîte fait 180 px : le champ était forcé à 120 px
  minimum quoi qu'il reste. La largeur du bouton se mesure, et le champ
  prend ce qui reste.

## [0.96.0] - 2026-08-28

### Fixed
- **Les boutons de la fiche s'imprimaient par-dessus le nom du
  patient.** Ils passaient sous le nom « si la vue fait moins de
  620 px » — et à 1024x700 les deux panneaux ouverts en laissent 645,
  juste au-dessus du seuil : les quatre boutons se posaient alors en
  travers de « Jean Dupont ». Un nom long ou un panneau plus large
  rouvrait le défaut à chaque fois. La place nécessaire se mesure
  maintenant — nom, date de naissance et boutons — au lieu d'être
  devinée.
- **L'onglet Entretiens n'affichait pas un seul acte.** Le journal des
  notes sous le tableau avait un plancher de 170 px et prenait plus de
  la moitié des 290 px de cet onglet : le tableau pour lequel la fiche
  existe montrait sa ligne de récapitulation et rien d'autre. Les actes
  gardent 150 px avant que le journal ne se serve — c'est le journal
  qui défile.
- **La ligne de saisie du carnet de vaccination était coupée en deux.**
  Le plafond qui protégeait le tableau rognait le formulaire, dont la
  seconde rangée de champs — date, n° de lot, site — disparaissait sous
  le bord du panneau. C'est le tableau qui cède désormais : une liste à
  laquelle il manque une ligne se fait défiler, un champ coupé en deux
  ne se remplit pas.

### Known
- À 1024x700 avec les deux panneaux ouverts, la fiche patient laisse
  environ 290 px sous les onglets. Les tableaux y montrent leurs
  en-têtes et une ligne ou deux, le reste se fait défiler : c'est un
  arbitrage, pas un oubli. Deux tentatives de reprendre la place
  ailleurs ont été annulées — l'une coupait le tableau des actes,
  l'autre les boutons de « À faire » et « Voyage ».

## [0.95.0] - 2026-08-27

### Fixed
Une passe sur les vues à 1024x700, les deux panneaux latéraux ouverts —
la taille que la règle du projet impose et que `smoke.sh` ne vérifie
pas : il attrape les plantages, pas ce qui sort de son cadre.

- **Le champ « + médicament… » disparaissait** sur une fiche à cinq
  traitements : la rangée des puces ne se repliait pas et poussait hors
  du panneau précisément ce qui sert à ajouter le sixième. Elle se
  replie.
- **La vue Mois montrait trois semaines sur six.** La bande de filtres
  se réservait deux cinquièmes de la hauteur et la grille du mois — la
  vue dont c'est le nom — devait se faire défiler. La bande passe à un
  tiers et défile à sa place ; les cases du mois se resserrent jusqu'à
  30 px au lieu de 44, et sur une case courte les pastilles d'activité
  passent à côté du numéro du jour plutôt qu'en dessous, où il n'y avait
  plus de place.
- **L'onglet Biologie n'affichait que ses en-têtes de colonnes.** Le
  bandeau « ce que ça change » prenait sa part d'une zone qui n'en avait
  pas : les résultats gardent 240 px avant qu'il ne se serve, et son
  propre plancher descend à 90.

## [0.94.0] - 2026-08-27

### Changed
- **Les protocoles sont enfin taillés comme le reste.** C'était une
  colonne de 900 pixels centrée sur un écran de 1600, avec l'éditeur
  empilé sous la liste. Ça tenait à cinq protocoles ; à vingt, non — et
  c'est précisément la forme que cette application s'interdit. La liste
  passe à gauche, dans son panneau, avec **un champ de recherche** et la
  ligne ouverte en surbrillance ; l'arbre passe à droite. Le sujet du
  protocole, qui prenait la moitié d'une ligne, est au survol.
- **L'arbre ne déborde plus de son panneau.** Chaque nœud est un bloc
  indenté : son étiquette de branche, sa phrase qui va à la ligne dans
  la largeur du panneau, et ses boutons en dessous. Avant, la largeur de
  repli se lisait *dans* la rangée, où « largeur disponible » veut dire
  « ce qui reste sur cette ligne » — un nombre différent à chaque
  indentation, et les conduites sortaient par la droite avec les boutons
  derrière elles. Elle se prend maintenant une fois, sur le panneau.
- Un nouveau protocole s'ouvre directement dans l'éditeur : il est vide,
  il n'y a rien à lire à son sujet dans la liste.
- Vérifié à 1024x700 avec les deux panneaux latéraux ouverts, comme le
  veut la règle : rien ne sort de son cadre.

## [0.93.0] - 2026-08-27

### Added
- **La couverture de tests est mesurée et tenue.**
  `./scripts/coverage.sh` affiche le tableau par fichier et **échoue si
  deux planchers sont franchis** — même idée que le cliquet des
  posologies : ils ne descendent jamais. CI le lance à chaque poussée.
- **Onze tests sur la logique qui restait nue** : les lignes
  effectivement facturables et la date qu'elles portent, l'URL passée au
  navigateur (la seule chaîne que cette application hors ligne envoie
  quelque part), les interactions citées sur le bilan — une paire de
  traitements citée une fois, jamais la fiche se citant elle-même —, les
  locations qui atteignent le récapitulatif, la liste d'appel des
  renouvellements dépassés, la configuration de bout en bout sur le
  disque (premier lancement, modèle commenté, sauvegarde, rechargement,
  géométrie de fenêtre), la journée ordinaire de la base (agenda,
  carnet, voyage, note, cellule corrigée, posologie, fiche modifiée puis
  supprimée), et les libellés de la carte du voyageur.
- config.rs passe de 78 à 96 %, db.rs de 80 à 83,5 %, vaccines.rs de
  82,5 à 91 %. **La logique métier est à 87,1 %**, dans la fourchette
  que la feuille de route demandait.

### Changed
- Le chiffre du workspace (40,4 %) est bas et le restera tant que le
  moteur n'est pas monté : `src/app.rs` fait 15 000 lignes de mise en
  page egui, plus de la moitié du dépôt, et une vue ne se couvre pas
  sans harnais d'interface. `egui_kittest` demande egui >= 0.30 quand le
  projet est sur 0.29. Le script le dit en toutes lettres plutôt que de
  laisser croire à un oubli, et `scripts/smoke.sh` reste ce qui tient
  l'interface — 31 vues ouvertes à chaque fois, échec sur tout panic.

## [0.92.0] - 2026-08-27

### Added
- **Les insulines ont enfin une forme.** « Lente » sur une boîte
  recouvre une glargine sans pic et une NPH qui culmine à six heures —
  et c'est ce pic qui fait l'hypoglycémie de fin d'après-midi. Douze
  insulines du marché français portent maintenant leur profil d'action :
  début, pic, durée, et la courbe qui va avec.
- **Sur une fiche d'insuline, ce profil remplace la courbe de
  décroissance.** La demi-vie d'un dépôt sous-cutané répond à une
  question que personne ne pose ; « quand est-ce que ça tape » est celle
  qu'on entend tous les jours.
- **Dans Calculs, les courbes se superposent** : on clique les insulines
  à comparer et elles se dessinent sur le même axe, avec un repère sous
  le pic de chacune. La glargine à côté de la NPH, c'est une explication
  qu'on peut montrer au patient.
- **Les trois règles**, sous le dessin : règle des 500 (grammes de
  glucides couverts par une unité, 450 pour l'insuline humaine), règle
  des 1800 (facteur de sensibilité), bolus repas, dose de correction, et
  la titration de la basale. Une glycémie sous la cible affiche « rien à
  corriger » plutôt qu'une dose négative à retrancher.
- `BPM_CADDY_DRUG=<nom>` ouvre une fiche donnée avec
  `START_VIEW=drug_card` — pour vérifier une courbe sur la bonne fiche.
- **Douze préparations de plus au codex**, qui passe de 20 à 32. Les
  excipients qu'on refait sans arrêt (cold cream, cérat de Galien), les
  antiseptiques et leurs pièges (Dakin, qui meurt à la lumière et dégage
  du chlore avec un acide ; Milian, qui tache tout ce qu'elle touche et
  qu'il faut annoncer avant de délivrer), la solution alcoolique
  salicylée du cuir chevelu, la vaseline boriquée du nez, et six
  formules types dont les quantités viennent de l'ordonnance : nystatine
  buvable, gel de kétoprofène — avec la photosensibilisation qui dure
  deux semaines après l'arrêt —, morphine buvable avec sa double
  vérification de concentration, bain de bouche corticoïde qui ne
  s'avale pas, macrogol, et le lavage oculaire qui commence par « si
  vous ne pouvez pas stériliser, n'en faites pas ».

### Fixed
- **Le facteur de sensibilité était dix fois trop petit.** 1800/DTQ est
  en mg/dL, et 100 mg/dL font 1 g/L : le chiffre en g/L est 18/DTQ et
  non 1,8/DTQ. Trouvé en regardant le panneau afficher 0,045 g/L et une
  correction de 17,8 UI là où il fallait lire 0,45 g/L et 1,8 UI. Un
  test tient désormais les deux unités ensemble à toutes les doses —
  1 mmol/L de glucose vaut 0,18 g/L, et l'écart ne peut plus s'installer.

## [0.91.0] - 2026-08-27

### Added
- **Les locations de matériel se comptent toutes seules.** Un forfait
  court : personne ne relit un nébuliseur toutes les semaines, et
  l'ordonnance qui a expiré en mars avec la machine encore chez le
  patient en juin est exactement la paire que personne ne remarque.
  Un onglet « Locations » sur la fiche patient pose le matériel, le
  reprend, enregistre un renouvellement — et compte les périodes
  entamées contre le forfait **tel qu'il était le jour de la pose**,
  jamais tel qu'il est aujourd'hui : un tarif qui bouge en juin ne doit
  pas réécrire ce qui a été délivré en mars.
- **Le tableau de bord appelle.** Un panneau apparaît — et seulement
  s'il a quelque chose à dire — avec les ordonnances de location
  dépassées en rouge et celles qui arrivent à échéance dans le délai de
  prévenance. Un clic ouvre la fiche directement sur son onglet.
- **Le récapitulatif de facturation** imprime les locations dans leur
  propre tableau, avec leur propre total : ce ne sont pas des actes,
  elles n'ont ni code acte ni étape, et les mêler à la grille des actes
  reviendrait à les inviter dans le total des actes.
- `src/location.rs` porte le calcul : périodes entamées, plafond de la
  ligne LPP, date du prochain renouvellement. Pur, testé, sans horloge
  interne — le jour est passé en paramètre, comme dans `vaccines.rs`.

### Changed
- **Les forfaits sont vides par défaut, et c'est délibéré.**
  `[locations]` dans config.toml, éditable dans Options › Locations :
  libellé, ligne LPP, période (jour, semaine, mois), forfait, délai de
  renouvellement, plafond de périodes payées. La LPP bouge, et un tarif
  livré serait faux dans l'année — c'est la même règle que le champ LPP
  d'une fiche de dispositif.
- Le plafond est dans l'arithmétique et non dans une remarque que
  personne ne lit : facturer au-delà de ce que la ligne paie, c'est ce
  qui vaut un indu.

## [0.90.0] - 2026-08-27

### Added
- **Les dispositifs médicaux ont enfin leurs fiches.** L'officine en
  délivre autant que de médicaments et n'avait nulle part où écrire ce
  qu'elle en sait. « Dispositifs… », depuis la vue Médicaments ou
  Ctrl+K : 35 fiches en onze familles — les pansements dans l'ordre où
  on les choisit (hydrocolloïde, hydrocellulaire, alginate, hydrofibre,
  hydrogel, interface, argent, charbon, film, compresses), la fixation,
  la compression, la stomie (colostomie, iléostomie, urostomie et les
  accessoires qui sauvent la peau), le sondage et l'incontinence, les
  sets de soins, l'injection et le diabète, le respiratoire, et le
  matériel qu'on loue.
- Chaque fiche répond aux six questions du comptoir : pour quelle
  situation, quelles tailles existent, comment ça se pose, à quel rythme
  ça se renouvelle, ce que dit la ligne LPP, et ce qui va de travers.
  C'est là que sont le gel jaune de l'hydrocolloïde qu'on prend pour du
  pus, l'alginate qu'on ne retire jamais à sec, la découpe trop large
  qui fait la peau rouge autour d'une stomie, l'étui pénien qu'on mesure
  au gabarit, la chambre d'inhalation qu'on laisse sécher sans
  l'essuyer, et les barrières de lit qui ne sont pas une contention.
- **La ligne LPP porte la règle, jamais le tarif** : ce que la
  prescription doit mentionner, ce qui entre dans un forfait, ce qui se
  facture à part. Un prix livré dans une fiche est un prix faux dans
  l'année — un test refuse le « € » dans ce champ.
- La fiche s'imprime seule en A4, ou toutes ensemble en livret deux
  colonnes groupé par famille, à afficher près du stock.

### Changed
- Comme le codex : semé une fois, et à l'équipe ensuite. Une fiche
  réécrite n'est jamais remplacée, et une base vidée exprès reste vide.
  L'écriture est en compare-and-set sur **toutes** les colonnes — une
  fiche de dispositif n'a pas un champ qui porte tout son poids, et le
  rythme de renouvellement se corrige bien plus souvent que le nom.

## [0.89.0] - 2026-08-27

### Added
- **Quinze protocoles de plus, et l'outil sort de la rupture de
  stock.** Il n'en avait que cinq, tous sur ce qui manque en tiroir :
  un bel arbre de décision qui ne répondait qu'à la moitié des
  questions qu'on se pose au comptoir. Les nouveaux sont ce qui entre
  sans ordonnance — l'oubli de pilule et les trois cas qui n'ont pas la
  même conduite selon la semaine de plaquette, la contraception
  d'urgence et le poids qui fait choisir l'ulipristal, la gastro du
  nourrisson et le pli cutané qui décide, la tique qu'on retire en
  tournant et sans éther, la brûlure indolore qui est la plus grave,
  l'hypoglycémie du sulfamide qui récidive après le resucrage, la
  double dose de méthotrexate qui est une urgence sans aucun symptôme,
  la demande d'opioïde qui revient trop tôt. Et celui qu'on espère ne
  jamais dérouler : les signes d'AVC, l'heure de début qu'on note parce
  que c'est elle qui décide de la thrombolyse.
- **Dix-neuf règles de biologie**, et plus un seul analyte muet. Le
  catalogue en comptait trente-quatre, quatorze ne déclenchaient rien :
  l'application recopiait un chiffre que le laboratoire imprimait déjà
  mieux. Ils parlent maintenant — l'hypomagnésémie de l'IPP au long
  cours qui explique la kaliémie qui ne se corrige pas, la ferritine
  basse sous anticoagulant qui est un saignement digestif jusqu'à
  preuve du contraire, la calcémie qu'on corrige *avant* l'injection de
  dénosumab et jamais après, les ASAT plus hautes que les ALAT sous
  statine qui parlent du muscle et non du foie, la créatinine qui monte
  chez qui a la triade et une gastro, l'albuminurie sous IEC où la
  molécule est le traitement et pas la cause.

### Changed
- Les protocoles ne s'appellent plus « de substitution » : ils couvrent
  le comptoir.
- Deux tests de plus tiennent ce contenu : un protocole doit poser au
  moins deux questions et finir sur des conduites assez précises pour
  être suivies, et **chaque analyte du catalogue doit porter au moins
  une règle** — en ajouter un sans elle échoue désormais.

## [0.88.1] - 2026-08-27

### Fixed
- **« Replier » faisait disparaître le bouton qui déplie.** Sur un écran
  large, la fiche technique repliée n'avait plus que sa barre de titre :
  les 30 pixels qu'on lui laissait étaient entièrement mangés par la
  marge du panneau, le libellé et son filet, et le bouton « Déplier »
  était rogné. Replier la fiche était une porte à sens unique — il
  fallait relancer l'application pour la retrouver. La hauteur repliée
  est maintenant mesurée sur la police et l'espacement réels, donc une
  échelle de texte plus grande ne peut pas ramener le défaut.
- Côté à côté, replier rend désormais sa **largeur** au lieu de garder
  un tiers de la rangée pour ne rien montrer : le rappel patients et le
  journal récupèrent la place.

## [0.88.0] - 2026-08-27

### Added
- **Trente-trois posologies de plus**, et le cliquet retombe de 200 à
  170. Ce lot va chercher les associations fixes et ce qu'elles
  cachent : le diurétique dans le Bipréterax et le ionogramme qu'il
  impose, les œdèmes de chevilles de l'amlodipine qu'on prend pour une
  insuffisance cardiaque, les trente-six heures obligatoires entre un
  IEC et l'Entresto sous peine d'angio-œdème, la kaliémie qui pilote le
  Kerendia. Puis les inhalés, où le geste vaut la molécule — la gélule
  du Breezhaler qui ne s'avale jamais et qu'on vérifie vide, les deux
  bouffées du Respimat qui comptent pour une dose. Et le reste du
  comptoir : le Maalox à deux heures de tout, le patch de rotigotine
  qu'on retire avant d'en poser un autre, l'Apokinon dont l'antiémétique
  ne doit jamais être un neuroleptique, le Colchimax dont l'opium masque
  justement la diarrhée qui signale l'intoxication, l'EPO qui ne fait
  rien sans fer. 1 447 lignes sur 536 fiches.

## [0.87.0] - 2026-08-27

### Added
- **Trente-quatre posologies pour le fond de rayon**, et le cliquet
  descend de 250 à 200 classes découvertes. Ce sont les gestes qu'on
  explique dix fois par jour : le corticoïde inhalé qu'on prend même
  quand tout va bien et après lequel on se rince la bouche, le Movicol
  qui met un à deux jours et n'est donc pas un laxatif de secours, la
  trinitrine qu'on pulvérise assis parce que la chute de tension fait
  tomber, les insulines — l'ultra-rapide qui colle au repas, la NPH
  qu'on roule jusqu'à ce qu'elle soit uniformément blanche —, le Zinnat
  qui s'absorbe mieux au cours du repas là où la plupart des
  antibiotiques veulent l'inverse, l'Oracilline dont les dix jours
  préviennent le rhumatisme articulaire aigu même quand la gorge ne fait
  plus mal, le Dicetel qu'on ne prend jamais couché, l'Ultra-Levure
  qu'on n'ouvre pas près d'un cathéter, et EllaOne dont le délai va
  jusqu'à cinq jours mais dont le plus tôt reste le mieux. 1 414 lignes
  sur 503 fiches.

## [0.86.1] - 2026-08-27

### Added
- **Un cliquet sur la couverture des posologies.** La liste des classes
  qui n'en auront jamais est explicite — celles que le spécialiste titre
  contre son patient, et les vaccins dont les schémas vivent dans
  `vaccines.rs`. Pour le reste, la mesure est franche : il manque encore
  une ligne à un bon tiers des fiches, et inventer une liste
  d'exemptions aurait transformé cette dette en décision. Le test
  enregistre donc le nombre de classes encore découvertes et échoue s'il
  augmente : une fiche ajoutée sans sa posologie est attrapée, et chaque
  lot écrit fait baisser le chiffre. Il refuse aussi qu'on laisse le
  plafond dériver loin devant la réalité — un plafond que personne
  n'abaisse cesse d'être un cliquet.

## [0.86.0] - 2026-08-27

### Added
- **Vingt-six posologies pour ce qui se renouvelle tous les mois** et
  n'avait pas sa ligne : les dermocorticoïdes forts et le schéma
  d'entretien deux jours par semaine qui espace les poussées — celui
  qu'on ne fait pas si personne ne l'a dit —, les hypnotiques et leur
  durée qui est réglementaire autant que clinique, les antithyroïdiens
  dont la fièvre impose une numération avant même de savoir pourquoi,
  les associations fixes antihypertensives et le ionogramme qui va avec,
  les sulfamides hypoglycémiants et la règle « pas de repas, pas de
  comprimé », les résines échangeuses qu'on ne délaye jamais dans un jus
  de fruit, les dérivés actifs de la vitamine D qui n'ont pas besoin du
  rein pour agir — et c'est ce qui les rend rapidement hypercalcémiants
  —, et les estroprogestatifs avec la règle des douze heures. 1 380
  lignes sur 469 fiches.

## [0.85.0] - 2026-08-27

### Added
- **« Toxicité / marge thérapeutique » sur ce que le comptoir vend le
  plus.** La section n'existait que sur les molécules à marge étroite ;
  elle porte maintenant les surdosages qu'une officine rencontre pour de
  vrai (40 fiches). Le paracétamol d'abord : 150 mg par kilo, quatre
  grammes seulement entre la dose maximale et celle qui détruit le foie,
  et surtout vingt-quatre heures sans le moindre symptôme — la fiche dit
  d'appeler le jour même, sans attendre un signe qui ne viendra pas. Le
  fer ensuite, première cause d'intoxication mortelle du petit enfant,
  avec son accalmie trompeuse. Puis les AINS, dont le danger n'est pas
  la dose massive mais la dose ordinaire chez un patient déshydraté sous
  IEC et diurétique ; l'aspirine et le syndrome de Reye ; les opioïdes
  et leurs trois signes qui vont ensemble ; les benzodiazépines, larges
  seules et étroites en association ; la metformine dont l'acidose
  lactique vient du rein et non de la dose ; la colchicine dont la
  diarrhée est déjà l'intoxication ; la vitamine D dont la parade est
  d'écrire la date de l'ampoule.

### Changed
- `Db::refresh_toxicity` devient une passe versionnée : elle remplit
  aussi les sections restées vides, et son marqueur passe à « 2 » pour
  que les bases qui avaient déjà reçu la correction de la v0.81.0
  reçoivent celles écrites depuis. Une cellule que l'équipe a écrite ou
  verrouillée n'est jamais touchée.

## [0.84.0] - 2026-08-27

### Added
- **La liste du travail qui restait est vide.** Vingt-trois règles de
  plus (228 au total) : les vingt-cinq fiches nommées une par une hier
  ont leurs deux réponses. Ce sont des traitements de spécialité, et
  c'est justement au comptoir qu'on n'a pas la réponse sous la main — le
  bosentan et sa contraception qui doit être non hormonale parce que la
  molécule rend la pilule inefficace, le tolvaptan dont la soif est le
  garde-fou, le cinacalcet et les fourmillements autour de la bouche qui
  annoncent la calcémie basse, le Kaftrio qu'un repas sans gras ampute
  de moitié, la desmopressine dont l'hyponatrémie impose d'arrêter de
  boire, le filgrastim dont 38 °C est l'urgence, le géfitinib et sa
  pneumopathie interstitielle, la résine échangeuse qu'on ne délaye
  jamais dans un jus de fruit.
- Sur les 813 fiches, **781 portent maintenant « En cas d'oubli » et
  « Ce qui doit faire consulter »**. Les 32 autres sont administrées par
  un professionnel et le restent : une perfusion n'a pas de dose
  oubliée. Le test garde la liste d'attente vide — une fiche ajoutée
  demain sans ses deux sections échoue à la construction, au lieu
  d'arriver au comptoir avec un blanc qui ressemble à un choix.

## [0.83.0] - 2026-08-27

### Added
- **Quarante règles de conduite de plus** (205 au total) : les fiches
  que la mesure précédente n'avait pas vues — l'antibiotique urinaire en
  dose unique, le potassium qu'on avale assis avec un grand verre d'eau,
  l'antithyroïdien dont la fièvre et le mal de gorge imposent une
  numération le jour même, le nicorandil et ses ulcérations tardives,
  la théophylline dont deux doses rapprochées suffisent à intoxiquer,
  l'acarbose qui se resucre au glucose pur et jamais au sucre de table,
  les traitements de substitution aux opiacés, la crème anesthésiante
  posée trop tard qui décale le geste plutôt que d'écourter la pose,
  l'auto-injecteur d'adrénaline dont ce qui s'oublie n'est pas une dose
  mais la péremption et le second stylo, et le millepertuis dont le
  danger n'est pas lui mais tout ce qu'il annule.
- **Un test nomme la part qui manque.** Chaque fiche doit désormais
  porter les deux réponses, ou figurer sur l'une de deux listes
  explicites : celle des produits qu'un professionnel administre — une
  perfusion n'a pas de dose oubliée — et celle du travail qui reste,
  vingt-cinq fiches nommées une par une. Une section vide ressemble
  exactement à une section volontairement vide : la seule façon de ne
  pas confondre les deux est de les écrire.

## [0.82.0] - 2026-08-27

### Added
- **Cinquante-cinq règles de conduite de plus** : « En cas d'oubli » et
  « Ce qui doit faire consulter » couvrent maintenant 484 des 505 fiches
  de départ, contre 424. Ce sont les classes que le comptoir délivre et
  qui n'avaient encore aucune des deux réponses — les antitussifs (dont
  l'opiacé, à part), l'anticholinergique inhalé et son glaucome aigu par
  projection oculaire, la colchicine dont la diarrhée *est* le signe de
  surdosage, les antiacides et pansements gastriques, le cotrimoxazole
  et son éruption qui fait arrêter le jour même, la clindamycine et sa
  diarrhée jusqu'à deux mois après, les vitamines une par une, le
  calcium et le magnésium, l'addictologie, le TDAH et la narcolepsie
  dont la prise tardive coûte une nuit, le raloxifène et sa thrombose,
  le dénosumab dont l'oubli est celui de l'injection semestrielle.
- Les 21 fiches restantes — perfusions, injections intravitréennes,
  ocytociques, antibiotiques hospitaliers — **restent délibérément sans
  règle** : « en cas d'oubli » ne veut rien dire pour un produit qu'une
  équipe administre, et une phrase inventée y serait pire que le vide.

## [0.81.1] - 2026-08-27

### Changed
- **Les quatre fiches AOD étaient les plus minces de la base** — celle
  de l'édoxaban en tête, à moitié moins fournie que la moyenne, alors
  que l'AOD est l'un des quatre thèmes d'entretien de l'application.
  Mécanisme, effets indésirables et surveillance sont mis au niveau du
  reste : la biodisponibilité du rivaroxaban qui tombe d'un tiers à jeun
  aux dosages de 15 et 20 mg — première cause de sous-anticoagulation
  réelle, et à revérifier à chaque renouvellement —, la dyspepsie du
  dabigatran et ce qui la rend supportable, l'anémie qui se manifeste
  d'abord par une fatigue ou un malaise, et le réflexe qui va avec :
  devant une fatigue inexpliquée sous AOD, l'hémogramme avant tout
  autre examen.

## [0.81.0] - 2026-08-27

### Changed
- **La section « Toxicité / marge thérapeutique » dit enfin quelque
  chose.** Treize fiches portaient la même phrase — « marge
  thérapeutique étroite… voir les sections Interactions et
  Surveillance » —, c'est-à-dire un champ qui renvoie à un autre champ.
  Chacune porte maintenant ce qu'on cherche quand on ouvre cette
  section : la cible (INR 2-3, lithémie 0,5-0,8, digoxinémie 0,5-0,9,
  carbamazépine 4-12), le seuil où la toxicité commence, ce à quoi elle
  ressemble, et le piège propre à la molécule — la prise quotidienne de
  méthotrexate au lieu d'hebdomadaire, la vitesse de titration de la
  lamotrigine, l'hypersensibilité de la fluindione, l'uracilémie avant
  la première capécitabine, la NFS qui conditionne la clozapine.

### Added
- **Les quatre AOD reçoivent la leur**, qu'ils n'avaient pas : leur
  marge n'est pas une concentration mais un jeu de critères — les deux
  sur trois de l'apixaban, le repas obligatoire du rivaroxaban, les
  80 % rénaux du dabigatran et ses gélules qu'on n'ouvre jamais, la
  limite haute de l'édoxaban au-dessus de 95 mL/min — et l'antidote
  disponible pour chacun.
- `Db::refresh_toxicity` porte le nouveau texte aux bases existantes,
  une seule fois, et **uniquement** là où l'ancienne phrase est encore
  présente mot pour mot : une fiche sur laquelle l'équipe a écrit ne
  correspond plus, et rien ne lui arrive.

## [0.80.1] - 2026-08-27

### Changed
- Le mode d'emploi imprimable (F12 › « Mode d'emploi ») rattrape
  l'application : une section pour les deux recherches — « Aller à… »
  et « Dans le texte… » —, Ctrl+K dans la ligne des raccourcis, et les
  deux chiffres qui avaient vieilli (les fiches, les tables). Un
  exemplaire près du poste qui décrit une version d'avant ne sert à
  personne.

## [0.80.0] - 2026-08-27

### Added
- **La recherche plein texte se restreint au dossier ouvert.** Avec une
  fiche patient ouverte, un bouton « Seulement chez X (n traitements) »
  limite la recherche à son ordonnance : « lesquels de *ceux-là* parlent
  de pamplemousse », « lesquels disent insuffisance rénale ». C'est la
  question du comptoir, et elle n'avait pas de réponse en un geste —
  vingt passages sur cinq fiches au lieu de deux cents sur huit cents.

### Changed
- Une recherche sans résultat le dit, au lieu de laisser un cadre vide
  qui se lit « ça charge » : le message diffère selon qu'elle a lu toute
  la base ou seulement les traitements de la fiche, et propose la sortie.

## [0.79.1] - 2026-08-27

### Fixed
- La recherche plein texte ne rendait que ses 200 premiers passages et
  les comptait comme si c'était tout : elle dit maintenant qu'elle
  s'arrête là, et propose de préciser le mot. Un plafond muet se lit
  « voilà tout ce qu'il y a ».
- « Aller à… » vers « Médicaments » ramène à l'index de la base, et non
  là où la vue avait été laissée : c'est la bande d'onglets qui rouvre
  un codex resté ouvert, la boîte de recherche est ce qui en sort.

## [0.79.0] - 2026-08-27

### Changed
- **Le mot cherché est surligné dans la phrase qui le porte.** Cent
  vingt-trois passages de « pamplemousse » se lisaient jusqu'ici comme
  cent vingt-trois paragraphes ; l'œil tombe maintenant dessus. Encre
  sombre sur fond clair, comme les widgets Motif marquent une sélection,
  faute de graisse dans la famille embarquée.

## [0.78.0] - 2026-08-27

### Added
- **Trente-cinq fiches de plus reçoivent leur posologie** : ce qui se
  délivre en ville et qu'aucune ligne ne couvrait encore — le
  Parkinson en complément de la lévodopa, la narcolepsie et le TDAH
  (avec leurs règles de prescription et de délivrance), les IMAO et
  leurs associations interdites, l'addictologie entière (Aotal, Revia,
  Baclocur, Zyban), le patch de capsaïcine, la gynécologie de ville, la
  contraception par implant, et les quelques anticancéreux oraux qui
  passent au comptoir. 1 354 lignes sur 443 fiches.

## [0.77.0] - 2026-08-27

### Changed
- **La recherche plein texte lit aussi les posologies.** Les 1 319
  lignes de posologie de la base — la dose et surtout la remarque à côté
  d'elle — sont de la prose comme le reste, et c'est celle où les
  réponses du comptoir sont écrites : « à jeun », « à distance du fer »,
  « resucrage expliqué à l'entourage », « sortir le stylo trente minutes
  avant ». Elles reviennent sous leur propre indication, pour que deux
  lignes d'une même fiche ne se lisent pas pareil. Une seule requête les
  lit toutes, à l'ouverture de la recherche.

## [0.76.0] - 2026-08-27

### Added
- **« Aller à… » ouvre sur le texte des fiches.** Sa dernière ligne est
  toujours « Chercher « … » dans le texte des fiches » : ce que la
  recherche par nom n'a pas trouvé est souvent écrit *dans* une
  monographie. « QT », « allaitement », « pamplemousse » ne sont le nom
  de rien — trois lettres suffisent, Entrée mène à la recherche plein
  texte et à ses phrases. Les deux recherches ne font plus qu'un geste.

## [0.75.0] - 2026-08-27

### Added
- **« Dans le texte… » : une recherche plein texte des monographies.**
  La recherche par nom répond à « où est l'Eliquis » ; celle-ci répond à
  l'autre moitié des questions du comptoir — « lesquelles de ces fiches
  parlent de pamplemousse », « lesquelles allongent le QT », « lesquelles
  sont photosensibilisantes ». Chaque passage revient avec la phrase
  telle que la fiche l'écrit, sous le nom de la section d'où elle vient,
  et le nom de la spécialité ouvre la fiche. Treize sections sont lues —
  indications, mécanisme, posologie, contre-indications, interactions,
  effets indésirables, toxicité, surveillance, IUP, « en cas d'oubli »,
  « ce qui doit faire consulter », formes et dosages, notes de l'équipe.
  Recherche exacte et non floue, insensible aux accents et à la casse :
  sur huit cents monographies, une sous-séquence approximative trouve
  tout et ne répond à rien. La base n'est relue que lorsque le texte
  change, jamais à chaque image.

### Changed
- Vingt-sept clés de `assets/strings.fr.toml` avaient survécu aux vues
  qui les affichaient — trois barres d'outils, un en-tête d'agenda, un
  tableau d'honoraires. Elles sont retirées, et un test tient l'inverse
  de celui qui existait : une clé que plus personne n'affiche fait
  échouer la construction. Le fichier est celui que l'officine surcharge,
  et chaque ligne est une promesse que la modifier change quelque chose.

## [0.74.0] - 2026-08-27

### Added
- **« Aller à… » (`Ctrl+K`) : une boîte au-dessus de tout.** Une ligne
  de texte, et dessous tout ce que la base contient — les patients, les
  fiches et leur DCI, les tables de référence, les préparations du
  codex, les protocoles de substitution et les six vues permanentes.
  Les flèches parcourent la liste, `Entrée` ouvre, `Échap` referme et
  laisse la vue exactement où elle était. Chaque genre prend au plus
  quatre des douze lignes avant que les autres aient leur tour : avec
  huit cents fiches dans la base, un classement brut n'aurait montré
  que des fiches, et « co » enterrait la patiente et la table de
  Cockcroft sous Codoliprane, Colchicine et Coltramyl. Ce qui reste de
  place se remplit ensuite dans l'ordre du score, pour qu'une recherche
  qui ne trouve que des fiches remplisse quand même la boîte. Un bouton
  dans la barre l'ouvre aussi : un raccourci que personne n'annonce est
  un raccourci que personne n'utilise.

## [0.73.0] - 2026-08-27

### Added
- **Trente-trois fiches de comptoir reçoivent leur posologie** : la
  dermatologie et les topiques (gale, poux, impétigo, psoriasis, acné),
  les collyres, le digestif et l'ORL de premier recours, les vitamines,
  le fer et le substitut nicotinique. Ce sont les produits qu'on tend
  sans ordonnance ou presque, et dont tout se joue dans la technique :
  le temps de pose, la seconde application au 8e jour, l'heure de la
  prise. 1 319 lignes sur 408 fiches.

### Changed
- `cargo clippy` tourne maintenant avec `--all-targets` : le code des
  tests est relu comme le reste. Trois avertissements qui dormaient là
  sont corrigés.

## [0.72.1] - 2026-08-27

### Changed
- `docs/CONTENU.md` maps the clinical content: where each kind of it
  lives, what seeds it, what test holds it, and how to add to it — the
  drug cards, the posologies, the two counter answers, the reference
  tables, the codex, the protocols, the biology, the ordonnance rules,
  the entretien checklists and the vaccine calendar. Two rules run
  through all of them: what the team writes is never overwritten, and
  every content has a test that can reach it.
- The demo's own substitution protocol is renamed « AOD et fonction
  rénale » : it sat next to the shipped « Anticoagulant oral direct
  indisponible » under a name close enough to look like a duplicate.

## [0.72.0] - 2026-08-27

### Added
- **Quarante lignes de posologie de plus**, sur les fiches que le
  comptoir voit tous les jours et qui n'en avaient pas : les
  antiarythmiques et l'équivalence bumétanide-furosémide, l'ivabradine
  qui ne marche qu'en rythme sinusal, le glimépiride qui ne se prend
  pas sans le repas qui suit, le sémaglutide oral et ses trente minutes
  à jeun, le tirzépatide et sa titration, la dégludec dont l'heure peut
  bouger, l'insuline intermédiaire qu'on remet en suspension, le
  sévélamer au milieu du repas, le calcium à distance de tout, le
  bromure d'ipratropium et l'œil, le salmétérol qui ne s'utilise jamais
  seul dans l'asthme, la pancréatine pendant le repas, la terbinafine
  et le goût qui s'en va, l'acide fusidique et ses sept jours,
  l'entécavir qu'on n'arrête pas, le dénosumab et son rendez-vous à six
  mois, le léflunomide et son wash-out, le tamoxifène et les ISRS qui
  le désactivent, l'isotrétinoïne et sa contraception sans exception,
  le timolol et la compression de l'angle interne. 1 286 lignes sur
  375 fiches.

## [0.71.0] - 2026-08-27

### Added
- **Forty posology lines more, on thirty-nine cards that had none** —
  les biothérapies délivrées à l'officine, où le rythme est le
  traitement (Humira toutes les deux semaines, Enbrel le même jour
  chaque semaine, l'induction de Cosentyx, le passage de Taltz à quatre
  semaines à la douzième, la semaine 3 sans injection de Kesimpta), les
  trois anti-CGRP et leurs deux rythmes, les anti-PCSK9 qui ne
  remplacent pas la statine, les HBPM au poids, les antiparasitaires et
  leur seconde prise quinze jours après, les antiseptiques qui
  s'inactivent l'un l'autre, les acides biliaires, les aminosalicylés
  dont l'entretien se poursuit en période calme, l'entacapone qui ne se
  prend jamais seule, l'opicapone à distance de la lévodopa, les fonds
  de migraine, deux benzodiazépines, l'oxycodone-naloxone et son
  laxatif dès le premier jour, l'hydromorphone et son équivalence, les
  anticholinergiques vésicaux et deux progestatifs. 1 246 lignes sur
  336 fiches.
- Les classes titrées par le spécialiste — antiépileptiques,
  antipsychotiques, immunosuppresseurs, inhibiteurs JAK,
  immunomodulateurs de la sclérose en plaques — restent délibérément
  vides, comme depuis la v0.51.0 : une ligne plausible y serait pire
  qu'une ligne absente.

## [0.70.1] - 2026-08-27

### Changed
- The « en cas d'oubli » content pass no longer writes a hundred and
  ten updates at every launch: it leaves a mark saying how many rules
  it applied, and does nothing until that number changes. On a base
  sitting on a pharmacy network drive, every statement is a round trip.

## [0.70.0] - 2026-08-27

### Added
- **The template editor lists the markers each document may use** —
  `{{PATIENT_NAME}}`, `{{TREATMENTS}}`, `{{CHECKLIST}}` and the rest,
  per template, under the file's path. A marker nobody knows about is a
  marker nobody uses, and a mistyped one prints itself on the page. A
  test keeps the list and the templates in step, in both directions.

## [0.69.1] - 2026-08-27

### Fixed
- **An act was stamped in UTC while the whole application works in the
  counter's own time.** Between 22:00 and midnight UTC — that is, after
  midnight in France — a new act and a new patient carried the previous
  day: the act did not show as done today, and its day placed it in the
  wrong cycle, which is what picks the fee. A pharmacie de garde works
  at those hours. New rows are stamped in local time, on an existing
  base as on a new one, and two tests hold it: an act created now
  carries today, and moving an act to another day moves its year and
  its rank with it.

## [0.69.0] - 2026-08-27

### Added
- **The vaccination act and the carnet line stop being written twice.**
  From a « Vaccination » act, « Carnet » jumps to the carnet with the
  day and the initials already filled. And when a dose has been
  recorded today with no act created for it, the carnet says so and
  offers to create it in one click — a dose given without its acte is
  work the officine has already done and does not bill.

## [0.68.1] - 2026-08-27

### Fixed
- The codex's header ran off the right edge at 1024x700 with both docks
  open: « Imprimer » and « + Nouvelle » now wrap to a second line, and
  the sentence under them wraps instead of being cut mid-word.

## [0.68.0] - 2026-08-27

### Added
- **La fiche d'entretien porte ce qu'il faut couvrir.** The sheet was a
  title and four empty boxes; it now carries the treatments the file
  knows and the checklist of the act's own theme, as tick-boxes: sept
  points pour l'initiation, l'observance chiffrée sans jugement, le
  contrôle et sa cible pour la biologie, la démonstration du dispositif
  pour la technique d'inhalation, l'automédication et les plantes pour
  les interactions… A theme the officine wrote itself gets the common
  ground, which is never wrong. `src/entretien.rs` holds the lists —
  static, pure, tested, and short on purpose: a checklist of twenty
  lines is a checklist nobody ticks.
- The two placeholders are `{{TREATMENTS}}` and `{{CHECKLIST}}` : a
  template written before this version simply ignores them. A test
  holds the sheet to one page.

## [0.67.0] - 2026-08-27

### Added
- **« À programmer » on the dashboard** — the accompaniments whose year
  is started, not finished, and with nothing in the agenda. A sequence
  left half-done pays half and makes the patient wait twice as long;
  the panel says which files are in that state, how far each one is
  (« BPM 2/4 »), when the last entretien was, and opens the file in one
  click. Read from the same export the CSV is made of, and it appears
  only when it has something to say.

### Changed
- The screenshots are regenerated on the new demo base, and the export
  rows carry the file's id so a reading of the export can open it.

## [0.66.0] - 2026-08-27

### Added
- **Le codex s'imprime en entier** — une fiche par préparation, formule,
  mode opératoire, conservation, mise en garde et sources : ce qui va
  dans le classeur du préparatoire. Le bouton est à côté de « + Nouvelle ».
- **Deux tables de référence de plus.** « Antibiotiques » — durée
  usuelle, moment de prise, ce qui réduit l'efficacité et ce qu'on
  surveille, famille par famille, avec les deux lignes qui comptent : la
  durée est le traitement, et la diarrhée qui suit peut venir deux mois
  après. « Arrêts et sevrages » — ce qui ne s'arrête jamais d'un coup :
  bêtabloquant, corticoïde au long cours, benzodiazépine, antidépresseur,
  opioïde, antiépileptique, clonidine, IPP, tabac et alcool, avec la
  façon de décroître et ce qui fait appeler. Vingt-sept tables,
  259 lignes.

## [0.65.1] - 2026-08-27

### Fixed
- **The upgrade path is tested.** A base written by the first version —
  no DCI, no class, no monograph, no biology, no codex — is opened, and
  every column the current code reads has to be there. It is what the
  migrations are for, and it is only true if they run. Five columns of
  that first version are listed among them too: they cost nothing when
  they already exist, and they turn a « no such column » on a
  hand-repaired base into a no-op.
- The Options dialog writes the whole configuration back through the
  TOML serializer; a round trip with the team list and the mentions is
  now a test, since an array of tables in the wrong place is exactly
  what that serializer refuses.

## [0.65.0] - 2026-08-27

### Added
- **Eight preparations more in the codex** — l'alcool à 70 % dilué de
  l'alcool à 90 % (où le « qsp » absorbe la contraction de volume), la
  chlorhexidine aqueuse à 0,05 %, la pommade à l'oxyde de zinc, le talc
  mentholé (contre-indiqué avant 30 mois), la crème à l'urée à 30 %,
  la pommade de Whitfield, le gel hydroalcoolique de la formule OMS
  n° 1 (et les 72 heures d'attente avant usage), et le lavage nasal
  hypertonique à 3 %. Vingt préparations.

### Fixed
- At 1024x700 with both docks open, the biology tab gave the whole
  work area to « Ce que ça change » and left the results panel showing
  its title: the band is capped like the carnet's, and the two share
  what there is.
- The patient band did not count the interactions line and the revue
  chips in its height, so the eligibility note dropped off the bottom
  on a small screen. Both are measured now, and both wrap.

## [0.64.0] - 2026-08-27

### Added
- **Eight analytes more** — ASAT, phosphatases alcalines, bilirubine
  totale, VGM, vitamine B12, folates, lipasémie, phosphorémie — and
  **six reading rules** with them: la macrocytose sous metformine, la
  B12 basse sous metformine ou IPP au long cours, la cholestase sous
  amoxicilline-acide clavulanique, la lipase à trois fois la normale
  sous incrétinomimétique, le chélateur du phosphore qui ne sert à rien
  pris à distance du repas, les folates bas sous méthotrexate.
  Thirty-four analytes, twenty-seven rules.
- **Six ordonnance rules more**: deux sérotoninergiques (le tramadol et
  les triptans comptent), bêtabloquant avec vérapamil ou diltiazem, la
  colchicine exposée aux macrolides et aux azolés, la digoxine majorée
  par l'amiodarone, AINS avec corticoïde, et le millepertuis inducteur.
  Twenty-eight rules.
- **Une fiche Millepertuis**, monographie complète et deux posologies :
  la plante qui interagit le plus au comptoir méritait une fiche, ne
  serait-ce que pour que la revue d'ordonnance puisse la voir. 813
  fiches.

## [0.63.0] - 2026-08-27

### Added
- **A mode d'emploi the team can hold.** The shortcut window (F12) now
  prints the application's own handout: fifteen sections in two columns
  on one sheet — ouvrir la base, trouver un patient, créer et suivre un
  entretien, ce que l'acte imprime, le bilan et le plan de prise, la
  biologie, le carnet, le référentiel, les tables et le codex, l'agenda,
  le tableau de bord, les réglages, les raccourcis, et ce que
  l'application ne décide pas. One copy beside the counter PC, one in
  the binder. A test keeps it to a sheet.

## [0.62.0] - 2026-08-27

### Added
- **« Plan de prise… » — the patient's own copy.** One line per
  treatment: le médicament, à quoi ça sert, quand le prendre, et ce
  qu'il faut savoir — la conduite en cas d'oubli en premier, puisque
  c'est la question qui revient. Underneath, a box for « mes questions
  pour la prochaine fois », the pharmacy's phone number and who
  prepared it. The bilan stays at the officine; this one goes home.
  `[disclaimers] plan` adds the officine's own line at the foot.
- **Three content invariants, as tests.** Every conduite rule must
  reach at least one starter card; every biology and ordonnance rule
  must be able to fire on the base as shipped; every codex formula must
  parse, carry a readable yield and survive being rescaled without
  losing a line. Content that can never be reached is content nobody
  will ever fix.

## [0.61.0] - 2026-08-27

### Added
- **Five protocols to start from.** « Protocoles… » was a decision-tree
  editor with nothing in it. A fresh base — and any base opened after
  this version — now carries the trees the counter actually walks: la
  rupture de stock (conduite générale), l'anticoagulant oral direct
  indisponible (où la seule chose à ne pas faire est d'interrompre),
  l'écrasement d'un comprimé demandé (qui renvoie à la table « Écraser
  ou ouvrir »), l'allergie à la pénicilline annoncée au comptoir (neuf
  « allergiques » sur dix ne le sont pas), et la fièvre chez un patient
  sous anticancéreux, immunosuppresseur, clozapine ou antithyroïdien.
  They seed once by title: a tree the team has rewritten is never
  replaced, and a title they deleted never comes back.
- A test now holds their shape: one root, every branch hanging from a
  question, and every question carrying both of its answers — a
  walk-through that dead-ends on « non » is worse than no protocol.

### Fixed
- A protocol step is a sentence, not a caption: long conduites wrap to
  the room left beside the buttons instead of running off the right
  edge of the panel.

## [0.60.0] - 2026-08-27

### Added
- **One search across the twenty-five reference tables.** At the counter
  the question is « où est-ce que j'ai lu ça », and it was answered by
  clicking through twenty-five tabs. Typing in the new field shows every
  row that matches, wherever it lives, with the table it comes from —
  and the team's own corrections are what is searched and shown, so
  paper, screen and search never disagree. Clicking a table's name
  opens it. Escape clears the search before closing the tables.
- A test now refuses a UI string key that exists in the code and not in
  `assets/strings.fr.toml` — a typo used to reach the counter as a raw
  key on screen, visible only to whoever opened that view.

### Fixed
- The biology trend panel followed the analyte clicked on the *previous*
  patient; it now resets with the file.
- Typing over an analyte picked from the catalogue unpicks it: a result
  could be stored with one analyte's name and another's code and unit.

## [0.59.0] - 2026-08-27

### Changed
- **The dashboard's call list reads both** — the biology against each
  patient's treatments *and* each ordonnance against itself. « À
  revoir » now surfaces a file whose ordonnance carries a triade
  néfaste even when no biology has ever been recorded on it, and the
  line on hover is whichever of the two speaks loudest.

## [0.58.0] - 2026-08-27

### Added
- **La revue d'ordonnance** — what a set of treatments says about
  itself, which is the other half of a bilan partagé de médication.
  Twenty-two rules read the classes and the tags the cards already
  carry: la triade néfaste (bloqueur du SRA + diurétique + AINS), le
  double blocage IEC-sartan, l'anticoagulant avec un AINS, la
  benzodiazépine avec un opioïde, l'anticholinergique donné sous
  anticholinestérasique, le lithium exposé, le méthotrexate exposé,
  deux AINS, deux IPP, deux benzodiazépines, deux allongeurs du QT, la
  charge anticholinergique, trois sédatifs, la statine avec un fibrate,
  le clopidogrel avec l'oméprazole, l'ISRS avec un antithrombotique, la
  digoxine sous diurétique, l'œdème traité par un diurétique, la
  lévothyroxine et les bisphosphonates à distance des cations. Une
  association fixe compte pour ses deux moitiés ; un doublon demande
  deux boîtes distinctes.
- The points show as chips on the patient file, coloured by how loudly
  they ask, with the sentence and the médicaments concerned on hover —
  and in full on the bilan, under « Revue de l'ordonnance ».
- `BPM_CADDY_START_VIEW=revue` opens the file whose ordonnance has
  something to say, and the demo's second patient now carries the
  ordonnance a bilan exists for.

## [0.57.0] - 2026-08-27

### Added
- **« Biologie à revoir » on the dashboard**: the files whose latest
  results say something about their own treatments, loudest first, with
  the reason on hover and one click straight to that patient's biology
  tab. The whole base is read in two queries when the dashboard
  refreshes, and the panel only claims a place when it has something to
  say — an empty « rien à revoir » box on every dashboard would train
  the eye to skip it.
- **Sixty-four posology lines more**, on the cards that had none: les
  macrolides et les cyclines, les fluoroquinolones et leur tendon, les
  antituberculeux, l'itraconazole qu'un IPP annule, le Tamiflu dans
  ses 48 heures, les équivalences des corticoïdes et les
  dermocorticoïdes par zone, les trois IPP restants, le prasugrel après
  un AVC, les sartans, les nitrés et leur fenêtre libre, les gliptines,
  les analogues rapides de l'insuline, la lévothyroxine et sa TSH à
  six semaines, les bisphosphonates debout trente minutes, les laxatifs
  qui demandent de l'eau, les anticholinestérasiques, les ISRS à dose
  plafonnée, et les antitussifs qui ne dépassent pas cinq jours.
  1 204 lignes sur 296 fiches.
- The demo base now carries four treatments and three kaliémies on its
  first patient, so the reading rules, the trend and the call list show
  what they are for.

## [0.56.1] - 2026-08-26

### Fixed
- « En cas d'oubli » and « Ce qui doit faire consulter » now respect the
  same field locks as the rest of a card: a field the team cleared on
  purpose stays cleared instead of being refilled at the next launch.

### Changed
- The biology tab under a narrow work area puts « Ce que ça change »
  and the trend side by side instead of stacking them, and gives the
  band the height a finding actually takes: it used to show one and a
  half lines of the first one. The results table gains a row.
- The README says what the app now does — the biology, the bilan, the
  codex, the clickable molecules and the technical sheet, the team by
  name, the mentions the officine writes itself — and the screenshots
  are regenerated, with the biology tab and the codex among them.

## [0.56.0] - 2026-08-26

### Added
- **Le bilan partagé de médication, imprimé avec ce que la fiche
  sait.** Un bouton « Bilan… » sur le dossier patient : les traitements
  avec leur DCI, leur classe et leur posologie ; **les interactions que
  la fiche repère elle-même entre ces traitements** — pour chaque
  médicament, les phrases de sa propre monographie qui nomment un autre
  médicament du dossier, citées telles quelles, une paire à la fois ;
  la biologie avec sa lecture et ce qu'elle change ; ce que le
  calendrier vaccinal réclame encore ; les actes de l'année. Puis les
  deux cadres qui se remplissent pendant l'entretien — analyse
  pharmaceutique et plan d'action — et la signature. Un dossier vide
  imprime le formulaire qu'on remplit à la main.
- **Five reference tables more**, on the questions the twenty others
  did not answer. « Sujet âgé » — les médicaments à réévaluer après 75
  ans, pourquoi, et ce qui se propose à la place, y compris ce qui
  manque (START). « Inhalateurs » — la technique dispositif par
  dispositif, et l'erreur qui fait rater le traitement pour chacun.
  « Antidiabétiques » — hypoglycémie, rein, effets à annoncer, et les
  règles de jour de maladie. « Collyres » — l'ordre, le délai de cinq
  minutes, la compression de l'angle interne, l'œil rouge qu'on
  oriente. « Automédication » — ce qui se refuse au comptoir, et ce
  qu'on propose à la place. Vingt-cinq tables, 237 lignes.
- **Chaque table dit quand elle a été relue**, et contre quelle
  édition de ses sources : la ligne « Relu en… » s'affiche sous la
  table et s'imprime avec elle. Une table de référence qu'on ne peut
  pas dater est une table dont on ne se sert pas.
- **Deux réponses de plus sur chaque monographie** : « En cas d'oubli »
  et « Ce qui doit faire consulter ». Ce sont les deux questions les
  plus posées au comptoir et les moins écrites quelque part. 110 règles
  de classe les remplissent — l'oubli d'un AVK constaté le lendemain
  qui ne se rattrape pas, la gélule de dabigatran, le sulfamide qu'on
  ne prend pas sans le repas, le méthotrexate hebdomadaire, le
  bêtabloquant qu'on n'arrête jamais d'un coup — et 423 des 505 fiches
  de départ en héritent. Elles ne remplissent qu'un champ vide : une
  fiche que l'équipe a écrite garde ce qu'elle dit. Les deux sections
  s'affichent sur la fiche, s'éditent dans le formulaire et
  s'impriment avec la monographie.
- **Les deux tables les plus minces sont étoffées.** Les AOD gagnent la
  prise et l'alimentation (le rivaroxaban 15 et 20 mg au repas, la
  gélule de dabigatran qu'on n'ouvre jamais), la conduite en cas
  d'oubli, les interactions qui comptent, l'arrêt avant un geste
  invasif et ce qui fait appeler. Les corticoïdes inhalés gagnent
  l'association fixe, les signes d'un asthme non contrôlé, l'enfant, et
  les effets locaux qui ne justifient pas d'arrêter le fond.

## [0.55.0] - 2026-08-26

### Added
- **The patient's biology, and what it changes.** A third tab on the
  file: the results as the laboratory gave them, each read against its
  usual adult interval — normal, bas, élevé, or the critical threshold
  where it stops being a deviation. Twenty-six analytes to start with,
  from the DFG to the lithémie, each carrying the sentence that matters
  at the counter.
- **The reading against the treatments.** Twenty-one rules tie a value
  to what the file says the patient takes: a kaliémie above 5 under IEC
  or spironolactone, a DFG under 30 with an AOD or de la metformine, un
  INR au-dessus de 5 sous AVK, des CPK à cinq fois la normale sous
  statine, une thrombopénie sous héparine, une hyponatrémie sous ISRS.
  A value alone is reported; a value with the treatment behind it is an
  alert. Only the most recent reading of each analyte is read — a
  kaliémie corrected since is not an alert today.
- **A trend per analyte**: click a name and its series is drawn, with
  the bounds of the reference interval across it. Three kaliémies in a
  row say something a single one does not.
- Values are corrected in place (click the value), added with the date
  the counter types (`200826`, empty meaning today), and every write is
  compare-and-set like every other shared row.

### Fixed
- Two side columns asked for two flexible rows, and each flexible row
  takes the whole height that is left: the second panel was drawn past
  the bottom of the window. The carnet's « Voyage » panel and the drug
  card's journal are back on screen.

## [0.54.0] - 2026-08-26

### Added
- **A codex of preparations**, reached from the drugs view or from a
  card (« Codex… », which opens it already searched on that molecule).
  Twelve officinal formulas to start with — vaseline salicylée à 5 et
  10 %, pâte à l'eau, vaseline soufrée, crème à l'urée, coaltar
  saponiné, éosine aqueuse, gélules pédiatriques, sirop simple,
  bicarbonate à 1,4 %, chlorure de sodium à 0,9 %, glycérolé d'amidon —
  each with its formula, its mode opératoire, its conservation, what
  goes wrong, and its sources. They seed once and then belong to the
  team: a rewritten formula survives every launch, and adding a
  preparation is adding a fiche.
- **The formula at the quantity actually being made.** Type « 60 g »
  and every line is rescaled — the excipient's « qsp 100 g » becomes
  « qsp 60 g » — with each ingredient's strength read off the formula
  beside it. A quantity in another unit than the formula's is refused
  rather than guessed.
- **A printable fiche de fabrication**: the formula at the quantity
  prepared, a blank column for the lot of every raw material, the
  operator and the date, and the boxes for the control and the
  labelling — the record the bonnes pratiques de préparation ask for.
- **Three calculators under the sheet**, opening on a worked example:
  the titre (x % of y g), the dilution (C1·V1 = C2·V2, with what to
  take and what to make up with), and a batch of capsules (unit dose ×
  count, plus the overage), with the apparent volumes of the empty
  capsule sizes.

## [0.53.0] - 2026-08-26

### Added
- **The molecules in a monograph are clickable.** Every name of another
  card, wherever it appears in the prose — the ketoconazole that
  contraindicates, the phenytoine that lowers the exposure, the
  antidote — is a link to that card. Matching is accent- and
  case-insensitive, takes a two-word DCI whole (« acide
  acétylsalicylique »), ignores the card being read and words too short
  to be a molecule. The links are cut once, when the card is opened,
  not sixty times a second.
- **A technical sheet beside the monograph**, collapsible. The DCI, the
  class and the tags as chips that search the base for them; what is
  left of the drug 24 h after the last dose, as a meter and a decay
  curve; the narrow therapeutic margin first, in red; then status,
  formes, demi-vie, AUC, élimination, rein, grossesse, IUP, antidote
  and SMR as property and value. Folded, it gives its height back to
  the recall list and the journal.
- **PubChem and PubMed**, beside the ANSM lookup. PubChem answers what
  the molecule is, PubMed what has been published on it — sorted newest
  first, since that is the reason to ask. The query is the DCI when the
  card has one. The application stays offline: it hands a URL to the
  browser and nothing else.
- **« Compléter le carnet… »** writes the whole vaccine schedule into
  the carnet in one click — every dose the calendar says is owed, as
  undated lines the counter then fills in, corrects or deletes. Nothing
  is recorded as given, and a dose already planned is not planned
  twice.

### Changed
- The last two standing mentions — under the carnet on screen and under
  the vaccine map — join `[disclaimers]` as `vaccins`, empty by
  default like the rest.

## [0.52.0] - 2026-08-26

### Added
- **The team, by name.** `[pharmacy] operators` lists who works at the
  counter — initials, nom, qualité — and the Options › Officine page
  edits that list. The initials field beside the notes picks from it,
  and shows who is behind the letters.
- **An act records who did it.** A new « par » field beside the date,
  filled at creation with the initials at the counter and correctable
  after. That person signs the fiche, the courrier au médecin traitant
  and the ordonnance — not whoever happens to print them, three days
  later. The initials travel to the CSV export as their own column.
- **The date of an act is the day it was held.** The « Créé le » column
  was the day it was typed in; it is now « Fait le », editable, in the
  same compact form as every other date field (`230826`, `2308`). It is
  what places an act in its cycle, and the cycle picks the fee — an
  entretien entered the morning after was billing on the wrong day.

### Changed
- **No disclaimer is written by the application any more.** The five
  mentions it used to print or show — the ordonnance's header and
  footer, the box above the ordonnance on screen, the foot of the
  printed carnet de vaccination, the line under the calculators — are
  now `[disclaimers]` in `config.toml`, empty by default, editable in
  Options › Mentions. An empty one prints no line at all; the previous
  wording is in the config template, commented out, for an officine
  that wants it back.
- **A TROD has no theme and no duration.** It has a result. Both
  columns show a dash on the two TROD acts, and the CSV no longer
  carries a thematic that meant nothing on a test.

## [0.51.0] - 2026-08-26

### Added
- **Posologies for nine more classes.** 52 lines across the IEC, the
  bêtabloquants, les statines, les inhibiteurs calciques, les fibrates,
  les substituts nicotiniques, les mucolytiques, les insulines basales
  et les vaccins — with the counter point that goes with each: le
  captopril à distance des repas quand les autres IEC s'en moquent, le
  céliprolol à jeun, le comprimé d'Adalate qu'on retrouve intact dans
  les selles, l'œdème des chevilles des dihydropyridines que ni le
  régime sans sel ni un diurétique ne corrigent, le gemfibrozil qu'on
  n'associe pas à une statine, le mâcher-parquer des gommes à la
  nicotine, la glargine 300 qu'on ne transvase jamais.
- The seeding spot-check now covers ten cards across both passes rather
  than five.
- **Three reference tables**, answering the questions asked without an
  ordonnance in hand — the ones the other seventeen never covered.
  « Interactions » (pamplemousse, millepertuis, inducteurs, chélation,
  la triade néfaste), « Urgence » (anaphylaxie, hypoglycémie avec et
  sans trouble de conscience, AVC, douleur thoracique, intoxication),
  et « Grossesse » (ce qui se délivre, ce qui s'encadre, ce qui se
  refuse, à la grossesse comme à l'allaitement). Twenty tables now,
  each with its numbered sources on screen and on the printout.
- The table test now refuses a duplicate selector name and an empty
  cell, on top of the row-width check.

### Notes
- The specialist-titrated classes are deliberately still empty:
  antiépileptiques, antipsychotiques, immunosuppresseurs, inhibiteurs
  JAK et immunomodulateurs de la sclérose en plaques. Their doses are
  set by titration and by indication, and a plausible-looking line
  there would be worse than a blank one.

## [0.50.0] - 2026-08-26

### Added
- **Posologies for ten classes that had none.** 74 lines across the
  triptans, the AINS, the antihistaminiques H1, the corticoïdes nasaux,
  the antifongiques topiques, the collyres antiglaucomateux, the
  myorelaxants, the antipaludiques and the dermocorticoïdes — indication
  by indication, each with the counter remark that goes with it (the
  4-hour interval that is Naramig's alone, Inorial taken away from
  food, Malarone's 7 days after the return where the others ask 4
  weeks, the fatty meal Riamet needs to be absorbed at all).
- **Two content invariants, as tests.** Every starter card must carry a
  full monograph and its sources, and every posology line must name a
  real card and keep its lines in one run — a brand split into two runs
  silently loses the second at seeding, and neither failure is visible
  at the counter.

### Fixed
- 52 posology lines shipped with an empty remark, so the card showed an
  indication and a dose with nothing beside them. All 52 now carry the
  point that matters at the counter — the weekly-not-daily rule on
  méthotrexate, the 30 minutes upright after Fosamax, the eruption that
  means stopping Zyloric, the accord de soins on Dépakote.

### Changed
- **The ordonnance's adjuvants come from the drug base, not a built-in
  list.** Any card tagged « probiotique » is offered, with the posology
  lines the team wrote on that card as its schemas. Adding Lactibiane,
  Lactéol, an Aragan or an Aromasantis product is adding a fiche and
  tagging it — no second catalogue inside the app to keep in step with
  the base, and nothing to recompile.
- `[ordonnance] adjuvant_tag` in `config.toml` picks the tag, for an
  officine that files its conseil associé under another word.
- The section is « Adjuvant (probiotique, conseil associé) », and says
  where to add one when the base has nothing tagged.

## [0.49.0] - 2026-08-26

### Added
- **The bulletin d'adhésion, pre-filled on the official form.** Each act
  under the accompaniment convention gets an « Adhésion » button beside
  its PDF and CR. It opens the Assurance Maladie's own bulletin — the
  five PDFs from ameli.fr, one per theme, embedded as downloaded — with
  the identity blocks typed into their AcroForm fields. The app fills
  the official form; it does not redraw it.
- `patients.nir` and `patients.regime`, entered on the patient's
  correction form, and `[pharmacy] am_number` in the options. Any of
  them left empty leaves that line of the printed bulletin blank, with
  its dotted rule intact.

- **Ordonnance after a positive TROD.** The two TROD acts carry what
  the test read (« TROD + » / « TROD − », compare-and-set like every
  other shared value). A positive result opens a box offering the
  antibiotics that indication allows, each with the situation it is for
  and its usual posology pre-filled — pick one, or write the posology by
  hand. A probiotic can be added, two toggles switch on the conseils
  hygiéno-diététiques and the temps de prise, and free lines take
  anything else. It prints as an A4 ordonnance carrying the officine's
  N° AM, editable in « Modèles PDF… » like the fiche and the CR.
- The molecules, doses and durations come from the app's own « Angine »
  and « Cystite » reference tables — a test fails if the two ever drift
  apart, so what the pharmacist reads at the counter and what the
  patient is handed can never disagree.

### Notes
- The app proposes and the pharmacist decides: every posology is a plain
  text field, nothing is pre-selected, and the box says so above the
  print button.
- Every checkbox is left unticked and the date and signatures left
  blank: OUI/NON on the adhésion, OUI/NON on informing the médecin
  traitant and « à l'initiative du pharmacien » are the patient's
  decisions, taken in front of the form.
- The five forms disagree about their own field names — the pharmacy's
  Assurance Maladie number is `N AM`, `Num identification` or `fill_11`
  depending on the form, and `Adresse 1`/`Adresse 2` is the patient's
  address on three of them and the pharmacy's on the other two. The
  names were read off the rendered forms by position; a test checks
  every one still exists, and another that the two address blocks never
  collapse into one.
- Filled text is drawn through the form's `/Helv` (Latin-1) rather than
  the `/DA` default of Arial/MacRoman, which turned « Hélène » into
  « HÈlËne » on three of the five bulletins.

## [0.48.0] - 2026-08-26

### Added
- **Carnet de vaccination, per patient.** The patient file is now a
  notebook: « Entretiens » as before, and « Vaccinations » beside it.
  The carnet records a dose the way it is written on paper — vaccine,
  dose, date, lot, site, operator — correctable line by line, and
  prints as an A4 carnet. Dates take the same shorthand as the rest of
  the app; a dose read into the future is re-read as a past one, so
  `230850` is 1950.
- **« À faire » reads the carnet against the calendrier vaccinal.** dTP
  by milestone (25, 45, 65, then every ten years — a booster at 25 is
  not overdue at 36), the flu and COVID campaigns counted from the 1st
  of September, zona from 65, VRS from 75, pneumocoque, ROR for the
  1980 cohort and HPV in its window. Clicking a line loads that vaccine
  into the form at the foot of the carnet.
- **« Voyage » ticks the destinations off.** Countries recorded on the
  file list what a traveller owes for them, each marked *au carnet* or
  *manquant* against the doses already recorded.
- **A vaccination map (`F7`).** The world as a cartogram — one square
  per country, regions laid out roughly where they belong. Hover gives
  the group, the yellow-fever status, the malaria risk and the vaccines
  recommended; click pins the country, and one button records it as a
  destination on the open patient's file. Seven lenses recolour the
  map. Around 200 countries, with the year's BEH named as the
  authority on every panel that shows the data.

### Fixed
- The drug monograph drew its scrollbar down the middle of the sheet.
  The card's scroll area shrank to the reading measure of the document
  inside it instead of keeping the card's width, so the bar landed
  against the text rather than against the panel edge.
- `motif::section` allocated `available_width - 8` for its rule, which
  goes negative when a heading long enough fills the row — egui panics
  on a negative allocation. The rule gives way instead. Found by
  `scripts/smoke.sh` on the map's navigator.

### Changed
- `BPM_CADDY_START_VIEW` reaches `vaccins` and `vaccine_map`, and
  `scripts/smoke.sh` covers both.

## [0.47.0] - 2026-08-26

### Added
- The patient file totals itself: what the accompaniment has brought in
  and what is still owed on it, under the sequence strip. The dashboard
  totals the officine; nothing totalled the file in front of you. Masked
  with the rest of the figures in discreet mode.

### Fixed
- The protocol tree's branch markers used an arrow the bundled
  proportional face has no glyph for, so every "Oui" and "Non" in a
  decision tree was followed by a tofu box.
- « Calculs » drew its panel a full page below the table it sits under,
  so a tool just asked for had to be scrolled to. It is drawn directly
  under the table selector now.

### Changed
- `BPM_CADDY_START_VIEW` reaches the calculators and an open protocol,
  and `scripts/smoke.sh` covers both.

## [0.46.0] - 2026-08-26

### Added
- `scripts/smoke.sh` opens every view once — including the ones that
  only exist while a dialog is open — and fails on any panic. That is
  how the Ctrl+N crash was found, and the cheapest way to keep finding
  that class of bug: a code path only drawn under a keystroke is one no
  test ever reaches.

### Changed
- The conversion tables stripe their rows. Six columns of full
  sentences, where a row wraps to four lines in one column and one in
  the next, cannot be followed across without a band behind it — in a
  shade of the trough the table sits in, not egui's hover blue.
- The template editor is a share of the screen instead of a fixed
  680×540 box: a Typst template is a page of code, and editing it meant
  scrolling the whole thing through a porthole.
- « Tables de conversion » and « Protocoles… » drop to their own line
  when the drug base's page is narrow; at 1024 px with both docks open
  they were drawn over the title.

## [0.45.0] - 2026-08-26

### Fixed
- **Ctrl+N crashed the application.** The quick picker's digit table
  held nine keys for ten acts, and the loop that reads them indexed past
  its end on the first frame the picker was open — so the shortcut the
  app advertises for creating an entretien panicked instead of opening.
  The table is declared as `[egui::Key; InterviewKind::ALL.len()]` now,
  so the two can no longer diverge without failing to compile, and the
  tenth act answers to `0`. Present in 0.43.0 and 0.44.0.

### Added
- **The workspace reopens where it was left.** The window opened at
  1024×700 every morning and both docks reset to their default share, so
  the first thing anyone did each day was arrange the screen again. The
  size and the two dock widths live in a `layout.toml` beside
  `config.toml` — its own file, because the configuration is
  hand-editable and carries the operator's comments, and rewriting it on
  every quit to note a window size would quietly throw those away. It is
  written on a debounce, not only from `on_exit`, which never runs if
  the post is switched off at the counter.
- The act mix reports what each theme has earned as well as how often it
  was done: a count alone never says which acts carry the month.
- Each day of the agenda's week grid carries a load bar under its
  header, so the week's shape is read off the top of the grid.

### Changed
- **The navigator answers to the keys the search always did.** Typing in
  the dock and pressing Enter did nothing — the arrows and Enter were
  wired to the results list in the middle of the screen, which the home
  view replaced. The dock's field now drives its list (type, arrow down,
  Enter), with a keyboard cursor distinct from the mark on the open
  file, and the list scrolls to follow it.
- Ctrl+F puts the cursor in that field instead of closing the open
  patient to reach a search bar in the middle of the screen: that was
  the shape of the app before it had a dock.

## [0.44.0] - 2026-08-26

### Added
- **A keyboard reference (F12)**, or the toolbar's « ? ». The app is
  driven from the keyboard — that is the point of it at a counter — and
  until now the only way to learn a shortcut was to be told one. Every
  key it answers to is on one page, grouped by what it acts on, each
  drawn as a keycap so the left column can be scanned.
- **Where each accompaniment stands**, above the acts table: one row per
  act kind with its année d'accompagnement and its sequence as filled
  squares. The table says what was done; this says what is left and
  still billable, which the counter had to answer by counting rows.
- **The half-life as a curve** on the drug monograph. How much is left a
  day after the last dose is the question behind "puis-je opérer,
  relayer, arrêter", and reading it off "≈ 12 heures" was arithmetic
  done in the head.
- **The drug base opens on what it holds**: the classes it covers and
  how deeply, every card that names an antidote — the one lookup nobody
  wants to be searching for — and the cards carrying a status, each with
  its condition on a coloured chip.

### Changed
- **The Options dialog is five pages**, not one five-screen scroll: an
  auto-lock timeout and a 9×3 fee matrix were the same distance from
  the top, and the window was always as tall as the screen allowed. The
  left dock's own "ouverte au démarrage" setting joins the documentation
  pane's on the Interface page.
- **The status bar says what needs doing.** It counted patients,
  interviews and drugs — three numbers that never change through a shift
  and never ask for anything. It now also carries what is late, what is
  due today and what is waiting to be billed, each a click from the view
  that handles it, and names the operator stamping the notes.
- **The unlock screen is a Motif dialog box** — raised, hard-shadowed,
  with the app's painted mark at its head — instead of four centred
  lines on a field of grey. It is sized from the type scale, so at 1.4×
  the unlock button no longer falls out through the bottom edge.
- The agenda's day plan and month grid drop their own « ‹ Aujourd'hui › »
  rows — the control band drives all three modes — and both fill the
  pane they are given rather than a fixed 34 px row and 62 px cell.
- The docs dock reserved 185 px for the operator's private journal
  whether or not an operator was set; the reserve follows the field now,
  and the save state shares the operator's row.
- The dashboard's panels stretch to fill a tall window instead of ending
  in a band of grey, and fall to two lanes from 680 px rather than 800.
- The protocol list is a sunken list box like the patients and the
  drugs; the carnet reads at 900 px rather than 700.

### Fixed
- A notes box whose journal overflowed painted its last entries under
  the frame and pushed its "Ajouter" row through the bottom edge of the
  panel: `allocate_new_ui` only sets a max rect, and egui draws through
  it. The well is clipped to itself, and the box works out its own fit.
- `motif::list_row` ends a too-long row in an ellipsis rather than
  mid-letter — a row clipped by the panel edge reads as a rendering
  fault and hides the fact that there was more to read.
- The agenda's band names the month as a month, not as a date inside
  its grid.

## [0.43.0] - 2026-08-26

### Added
- **The workspace.** The screen is a notebook between three docks
  instead of one view at a time. Open patients and drug cards become
  tabs — `Ctrl+Tab` cycles, `Ctrl+W` closes, a middle click closes —
  so two records stay one click apart all day. The active tab is
  derived from the live view, so reaching a file by any other route
  (a dashboard row, a search result, Escape) still points the strip at
  it, and a tab whose patient another post deleted drops itself.
- **A left navigator dock** (`F6`, `[ui] show_nav_on_start`): the list
  the active view is browsing, beside the work instead of replacing it
  — the patient list, the drug index, a mini-month tinted by the day's
  load, or the carnet's days.
- **Charts.** `motif::chart` paints bars with a value grid and axis
  labels, horizontal-bar lists, stacked composition bars, sparklines,
  segmented meters, calendar heat strips and legends, all in the Motif
  idiom and all hand-painted.
- **A home screen.** An empty query used to draw every patient in a box
  down the middle of the screen. Until something is typed, the search
  view now shows the day: today's rendez-vous and whatever is overdue,
  the files the team touched last, and what was written today.

### Changed
- **The dashboard is a grid of panels that reflows with the window** —
  two lanes wide, one narrow, packed shortest-lane-first — rather than
  a 900 px column three screens tall. Indicator tiles size their figure
  to fit and carry a revenue sparkline; the pipeline is a proper funnel;
  monthly revenue is gridded, axis-labelled and legended, with a
  per-month tooltip; the act mix is a stacked bar over per-theme bars
  that report their yearly quota; and a 28-day heat strip puts a month
  of work in one strip of pixels.
- **The patient fiche is an identity band over a table-and-journal
  split.** The acts table — the reason to open a fiche at all — used to
  begin below the fold under the name, the buttons, the treatments,
  eleven act buttons and the journal. It now takes the width it needs
  and scrolls both ways instead of losing its right-hand columns
  silently, and the journal sits beside it when there is room.
- **The agenda is a control band over a calendar-and-day split.** One
  set of ‹ Aujourd'hui › buttons serves all three modes, the calendar
  fills the height it is given, and the selected day and the queue of
  rendez-vous are panels beside it.
- **The drug card reads at 860 px** with its recall list and its dated
  notes as panels outside the monograph's scroll — a column beside it
  when the window is wide, a band under it otherwise. The base's title
  block is drawn for the index only, not over an open card.
- The conversion tables use the width their cells need instead of a
  940 px cap, so "Formes et dosages usuels" stops wrapping to four
  lines with a quarter of the screen empty beside it.
- The toolbar's five view buttons are gone: they said what you could
  reach but never where you were. The notebook says both.

### Fixed
- The posology editor hard-coded 190/230/210 px for its three fields,
  which pushed "Remarque" off the card as soon as a dock was open. The
  three now share what the card has.
- The monograph sheet measures against the visible width rather than
  the width the panel claimed, so it stops losing its right margin —
  and with it the right-hand column of the posology table — to an open
  dock.
- Every band that measures its own content is capped and scrolls past
  its share, and every wrapped row is measured rather than assumed to
  be one line: the agenda's filters no longer lose their last act
  kinds, and the patient's act buttons no longer crowd out the panes
  beneath them.
- The docks take a share of the window rather than a fixed slab. At
  1024 px a 232 px navigator and a 340 px notes pane left the work
  itself 430 px, narrower than either of them.
- The screenshot script runs against a throwaway configuration: discreet
  mode was masking every figure it shot, and the run rewrote the
  operator's own `config.toml`.

## [0.42.0] - 2026-08-25

### Fixed
- The editable drug card was unusable. A multiline field grows with what
  it holds and ignores the height it is given, so now that every card
  carries a full monograph, one field ran down over the rows beneath it
  and drew across their labels. Each field is a box of its own height
  now, with the text scrolling inside it.
- The card's form no longer forces two columns into a narrow window: it
  puts one half under the other below 720 px, where two columns left
  five words to a line.

### Changed
- The entretien table is one aligned line per act. The act code and the
  step it pays move to a column of their own — "BMI · 2", with the full
  step name, the amount, the year and the coverage in the tooltip — and
  the two flags that change what is billed sit beside it as TPH and Δ.
  Before, the step name wrapped over three lines and pushed the flags
  out of their row, so no column lined up with its heading. Rows are
  striped.
- The agenda's week grid is as tall as its busiest day rather than a
  fixed height, so entries stop hiding behind a "+N" while there is room
  on screen for them, and an entry too long for its column ends in an
  ellipsis instead of being cut mid-letter.

## [0.41.0] - 2026-08-25

### Fixed
- The text scale had no effect on any Motif button: they were drawn at a
  hardcoded 14 px while the rest of the interface grew around them, and
  their padding ignored the compact density too. Both now come from the
  style.
- Opening the options twice enlarged the text twice: the scale was
  multiplied onto whatever size was already set instead of being applied
  to the base ladder.
- Secondary labels were painted in the bevel shadow colour, which is
  meant for a two-pixel edge and is far too light to read a word in.
  They now have colours of their own, and so do the agenda's hour
  column, its weekday heads, the half-life axis and the days outside the
  displayed month.
- List rows kept a fixed height whatever the density, so compact mode
  saved nothing in a list.

### Changed
- A deliberate type scale replaces the egui defaults: heading, body,
  button, small and monospace, all moving together with the text scale.
- Buttons that hold a state — agenda mode and filters, template target,
  situation, "à distance", "changement de traitement" — are one widget
  now, raised when off and sunken when on, instead of five hand-rolled
  copies of the same idiom.
- The dashboard reads at a glance: each figure sits under a small
  spaced caption and a hairline, and the per-theme counts are chips
  (sunken when the theme has entretiens, quiet when it has none)
  instead of one long line that broke between a label and its count.
- The patient card puts the acts where the work is: identity,
  treatments, entretiens, then the follow-up journal underneath — the
  reason to open a fiche is the entretien in progress, not last week's
  note.
- The agenda's act filters carry each act's colour and the same
  raised/sunken idiom as the rest of the interface.

## [0.40.0] - 2026-08-25

### Fixed
- Drug editing. Three things stood in the way and all three are gone.
  The team pane could grow wider than the width it had reserved, which
  left the whole central view laid out wider than it was visible and cut
  its right edge away — the drug card's buttons among it. The card's
  actions sat at the very bottom of the page, so on a full monograph
  "Modifier" and "Enregistrer" were several screens down. And a field
  the team cleared on purpose was refilled from the reference data by
  the next "Compléter les médicaments de départ".
- A field the team writes to is now theirs: the top-up fills only what
  they have never touched, and a field they emptied stays empty.
- The card's actions are a bar at the top, above the scroll, wrapping to
  a second line rather than running off the edge, with the deletion set
  apart from the rest.
- Any centred column now lays out inside the part of the panel that is
  actually on screen, so no view can be clipped that way again.
- The team pane's three tabs are short enough to fit it, and wrap if
  they do not.

## [0.39.0] - 2026-08-25

### Added
- The drug base is complete: all 812 cards carry a full monograph —
  indications, mechanism, posology, contraindications, interactions,
  adverse effects, surveillance, counter advice, half-life, elimination,
  renal adaptation, pregnancy and sources. The last 155 close the
  remaining gaps: contraception, opioid substitution, local
  anaesthetics, hypnotics and benzodiazepines, dermocorticoids, vitamin
  D and bone, antiretrovirals and hepatitis C, ophthalmology,
  gynaecology and the ward products.

### Fixed
- The treatment-change derogation read the wrong year when the change
  fell in a year after the first, asking for the année 1 minimums
  instead of the lighter ones, and counted the entretiens after the
  change beyond the sequence it opens.

## [0.38.0] - 2026-08-25

### Added
- 101 more monographs: neurology (epilepsy, migraine, Parkinson,
  multiple sclerosis, myorelaxants) and rheumatology and immunology
  (AINS, corticosteroids, biotherapies, JAK inhibitors, bone). 657 of
  the 812 cards now carry a full monograph — indications, mechanism,
  posology, contraindications, interactions, adverse effects,
  surveillance, counter advice, pharmacokinetics, renal adaptation,
  pregnancy and sources.

## [0.37.0] - 2026-08-25

### Added
- A printable billing recap, beside the CSV export on the dashboard: the
  entretiens performed and not yet billed, one line each with the date,
  the patient, the theme, the act code (with TPH when it was held
  remotely), the step of the sequence, the situation to declare, the
  coverage rate and the amount, and the total at the foot. A landscape
  A4 page carrying the memo's practical rules underneath, so the sheet
  can go straight to whoever does the invoicing.

## [0.36.0] - 2026-08-25

### Added
- The memo's anticancéreux derogation. An entretien can be marked as
  following a treatment change: it opens a new billable sequence at
  once, at the "années suivantes" tariff, without waiting out the twelve
  months. The button only appears on the two anticancéreux themes, the
  only ones the derogation still covers, and the fiche says which of the
  memo's conditions is not met yet — how many entretiens are missing
  before the change and after it. It travels to the CSV export.
- The "autres traitements anticancéreux" sequence may be finalised
  before twelve months, as the memo allows when entretiens are brought
  closer together at treatment initiation: a completed sequence opens
  the next one straight away instead of being held back by the quota.
- The bilan partagé de médication states its eligibility rule on the
  fiche: the memo reserves it to the patient on at least five treatments
  for six months or more, and the fiche says how many it knows of.
- The memo's practical rules under the Options fee table: tiers payant,
  billed independently of any CIP code, prices TTC, one pharmacy only
  per patient, and the ADRI service when the carte Vitale is missing.
- 102 more monographs: psychiatry, gastro-enterology and hepatology,
  oncology, urology, gynaecology and haematology. 556 of the 812 cards
  now carry a full monograph.

## [0.35.0] - 2026-08-25

### Changed
- The billing follows the Assurance Maladie memo *Aide à la
  facturation — accompagnement pharmaceutique* instead of the fee model
  the app had invented. Every entretien now carries the act code the
  memo prescribes (BMI/BMS, ASI/ASS, AC1/AC3, AC2/AC4), the step of the
  sequence it fills, and the amount that step bills: BMI 15 + 15 + 15 +
  20 = 65 €, ASI 15 + 15 + 20 = 50 €, AC1 15 + 15 + 30 = 60 €, AC2 15 +
  15 + 50 = 80 €, and 10 + 20 = 30 € for every "années suivantes"
  sequence. The code and the step show under the type on the patient
  card; hovering gives the amount, the year of accompaniment and the
  coverage rate.
- The anticancéreux theme splits in two, as the memo does: *anticancéreux
  au long cours* (AC1/AC3) and *anticancéreux (autres)* (AC2/AC4), which
  bill differently. Interviews recorded under the old single theme are
  read as *long cours*.
- The quota per year is no longer a number to set by hand: it is the
  length of the sequence the memo defines for that theme and that year
  — four entretiens for a first bilan de médication, three for a first
  AOD/AVK/asthme year, two for every following year.
- The Options fee grid is the memo's own table: one line per theme and
  per year, the act code, the amount of each entretien of the sequence,
  and the annual total. `config.toml` takes the same two rows
  (`annee_1`, `annees_suivantes`); a file written for an earlier version
  is still read and keeps billing what it billed.
- The CSV export gains Code acte, Année, Étape, À distance, Situation
  and Prise en charge (%).

### Added
- The patient's situation — ALD, AT/MP, maternité — is recorded on the
  fiche and travels to the export, the memo requiring it to be taken
  into account when billing.
- An "À distance" button on each entretien of an accompaniment: the TPH
  code the memo adds for a remote entretien, billed on top of the act
  code. Its amount is an option, the memo giving none.
- The code traceur TAC (adhésion, 0,01 €), billed once per patient and
  per theme when they join, is an option of its own.
- 179 more monographs: cardiology and hypertension, lipids and diabetes,
  anti-infectives, vaccines and pneumology. 454 of the 812 cards now
  carry a full monograph.

## [0.34.0] - 2026-08-25

### Added
- A day view: the counter's opening hours down the left, each
  rendez-vous and entry placed on its line, two abreast when they share
  an hour, and what has no hour listed underneath so nothing is
  hidden. The amplitude is an option (`day_start_hour`, `day_end_hour`).
  Clicking a day in the week or the month opens it.
- Recurring entries: a formation or a delivery repeats every week,
  fortnight or four weeks. It is stored once, shown on every day it
  falls on, and removed as a series.
- An overdue banner above the agenda: how many rendez-vous have slipped
  past their date, the oldest one, and the patients to reopen.
- The catalogue grows from 275 to 812 drugs, covering the essentials of
  the French market: cardiology and diabetes, anti-infectives,
  pneumology and ORL, neurology and psychiatry, analgesia and
  rheumatology, gastro-enterology, dermatology, gynaecology, urology,
  ophthalmology, haematology and the smoking-cessation products. The
  275 cards that had a monograph keep it; the new ones ship their
  identity, class and antidote for the team to fill in.

## [0.33.1] - 2026-08-25

### Changed
- The printed week fills the page: full-height day columns, so the plan
  can be written on during the week rather than only read.

## [0.33.0] - 2026-08-25

### Added
- Rendez-vous now have an hour. It is typed the fast way — 9, 9h30,
  930, 09:30 — on the patient's interview table or straight from the
  agenda's day panel, it leads the block on the week grid and the line
  in the day list, and the day is ordered by it with the untimed
  rendez-vous last. Agenda entries carry one too.
- A rendez-vous can be moved from the agenda: "Déplacer" takes a date
  in the usual compact form, without opening the record. Both writes
  are compare-and-set on what the screen showed.
- The agenda filters by act kind: click the kinds to narrow the grid,
  the day panel and the list at once, "Tous" to see everything again.
- "Imprimer la semaine" typesets the week on a landscape A4 page, one
  column per day, rendez-vous and other entries in the order of the
  day.
- The left and right arrows move the agenda a week — or a month in
  month view.
- 1017 posology lines over 168 drugs, against 368 over 60: the
  anticoagulants and opioids, the inhalers and insulins, Parkinson,
  psychiatry, urology and contraception, each with the lesser-known
  uses marked where they lie outside the AMM.

## [0.32.0] - 2026-08-25

### Added
- Every one of the 275 drug cards now ships a full monograph: the last
  ones are the oral tyrosine-kinase inhibitors and the cytotoxics taken
  at home (osimertinib, erlotinib, sunitinib, sorafénib, dasatinib,
  nilotinib, olaparib, témozolomide, hydroxycarbamide), the remaining
  insulins and the sulfamide. Each carries its indications, mechanism,
  posology, contraindications, interactions, adverse effects,
  monitoring, counselling points, pharmacokinetics and numbered
  sources.

## [0.31.0] - 2026-08-25

### Added
- Reference monographs for 247 of the 275 drug cards, against 141:
  oral anticancer drugs and hormonothérapies, the remaining HBPM,
  cardiology, inhalers and insulins, Parkinson and psychiatry, urology,
  dermatology with the dermocorticoid strength classes, ophthalmic
  drops, contraception, biologics and anti-infectives.

### Fixed
- The starter catalogue held the same product twice, "Kaléorid" and
  "Kaleorid", so one brand had two cards and only one of them a
  monograph. The duplicate is gone, and the uniqueness test now folds
  accents, case, spaces and hyphens so a second spelling cannot slip
  in again.

## [0.30.0] - 2026-08-25

### Added
- Reference monographs for 141 drugs, against 61: the analgesics and
  NSAIDs, the opioids, cardiology and diabetes, gastro-enterology,
  allergy and ORL, more psychotropes, and the counter staples — each
  with its indications, mechanism, posology, contraindications,
  interactions, adverse effects, monitoring, counselling points,
  pharmacokinetics and numbered sources.

### Fixed
A review of the day's work found eleven defects; all are fixed.
- A migration meant to run once was replayed at every unlock and
  destroyed table corrections made after it, including on a PC whose
  clock ran behind. It is gone; corrections are only ever removed by
  "Rétablir la table".
- Class notes and table cells were written blind: a colleague's
  paragraph or correction could be overwritten without notice. Both are
  compare-and-set now, with a French notice when the view was stale, as
  is the removal of a protocol step and its subtree.
- A class note written as "avk" and one written as "AVK" were two
  different rows, so an edit could vanish. One note per class now,
  whatever the spelling.
- `cycle_months` and the enforcement choice were inert: the rule, the
  fee ranks and the patient table all hardcoded twelve months, and
  "informer" or "refuser" still behaved like "avertir".
- The half-life reader took the "min" inside "administration" for
  minutes, turning a five-hour half-life into five minutes on the decay
  curve.
- The side pane's carnet shared its buffers with the patient and drug
  journals: text typed in one appeared in the other, and could be
  posted as a transmission.
- A protocol could not be renamed — the fields were re-cloned on every
  frame — and clicking a patient in the month view's day panel did
  nothing.
- A mistyped font path crashed the app on start, with no way back
  except editing config.toml by hand; the file is now parsed first and
  ignored when it is not a font.

## [0.29.0] - 2026-08-25

### Added
- Substitution protocols ("Protocoles…" in the drug base): what to
  dispense when a drug cannot be, written as a decision tree. A step is
  either a question — "clairance inférieure à 30 mL/min ?", "apixaban
  disponible ?" — with its oui and non branches, or a conduite to
  follow. Steps are added, rewritten and removed in place, each write
  compare-and-set like the rest.
- "Dérouler" walks the tree one question at a time, so the protocol can
  be followed at the counter without reading the whole thing, and
  "Imprimer" typesets it as an indented A4 page for the binder.
- The demo database ships one written the way a team would: AOD
  indisponible, branching on the clairance and on what the wholesaler
  has.

## [0.28.0] - 2026-08-25

### Added
- Posologies by indication: every drug card can carry a table of what
  it is prescribed for, the dose for that indication and what changes
  it — read on the monograph, edited line by line in the form, printed
  with the A4 sheet, and removed with the card.
- 368 shipped lines over 60 drugs, mainstream and lesser-known alike:
  spironolactone in acne and hirsutism, propranolol in essential
  tremor, migraine and performance anxiety, fosfomycine as monthly
  prophylaxis, doxycycline at anti-inflammatory dose in rosacea,
  amitriptyline in neuropathic pain, gabapentine in restless legs,
  aspirin in pre-eclampsia prevention — each marked when it is outside
  the AMM. They only ever fill a card whose list is still empty.

## [0.27.0] - 2026-08-25

### Added
- The right pane holds three contents, switched by a tab row: the team
  documentation, the day's carnet (readable and writable without
  leaving the current view), and the operator's personal notes. Which
  one opens with the app is an option.
- Font selection: point `[ui] font_path` at a .ttf or .otf — or pick it
  from Options — and the whole interface uses it; the embedded family
  remains the fallback.

### Fixed
- Wrapped text in the dashboard's two lists is no longer justified by
  `columns`, which stretched the spaces between words.

## [0.26.1] - 2026-08-25

### Fixed
- A `mut` left on the markup renderer's closure failed `cargo clippy
  -D warnings`, so the 0.26.0 release build did not compile under the
  CI gate.

## [0.26.0] - 2026-08-25

### Added
- The interface adapts to the screen and the eye: a text scale (0.8 to
  1.6) and a "compact" density that fits noticeably more on a small
  screen, both in Options and in `config.toml`, applied live.
- Optional toolbar pictograms, painted rather than typed — the bundled
  font carries almost no symbols — in the same square Motif style: a
  sheet for the documentation, bars for the dashboard, a capsule for
  the drug base, a month grid for the agenda, a pen for the carnet, a
  padlock, a cog and a template.
- Light formatting in the team's free text: `*gras*`, `_italique_` and
  `=surligné=` are rendered wherever the text is read — monograph
  sections and every note journal — while the editors stay plain text.

## [0.25.0] - 2026-08-25

### Changed
- The seventeen reference tables are rewritten wider and deeper: 95
  columns and 158 rows in all, against 40 and 108. The IPP table gains
  the forms, the moment of intake and the clopidogrel remark; HBPM the
  renal threshold, the monitoring and the antidote; the statins their
  LDL band, intensity and interaction risk; the corticoids their
  duration of action and mineralocorticoid effect; the opioids their
  delay, duration, forms and renal caution; the benzodiazepines their
  half-life, indication and elderly caution — and the same for the
  eleven others (AOD antidote and renal follow-up, inhaler devices and
  rinsing, insulin timing and storage, what each CKD stage changes for
  metformine, AOD and HBPM, the conduct per Mac Isaac band, cystitis
  durations and follow-up, missed-pill delays, analgesic paliers and
  cautions, who may be vaccinated by the pharmacist, paediatric forms
  and daily maxima, and the alternative to crushing).
- The printed reference now typesets in fixed fractional columns with
  French hyphenation, so a long word wraps inside its cell instead of
  spilling over the next one. It runs to sixteen A4 pages.

### Note
- Table corrections made with 0.23.0 or 0.24.0 are dropped on upgrade:
  the tables changed shape, so a cell edited before no longer points at
  the value it was written for. Corrections made from 0.25.0 on are
  kept as usual.

## [0.24.0] - 2026-08-25

### Added
- Formes et dosages on every drug card, shown with the
  pharmacokinetics on screen and on the printed monograph.
- Class notes: a note shared by every card of the same therapeutic
  class ("Note de classe…"), written once and read on all of them.
- "Rechercher…" opens the public ANSM medicines database in the
  browser, pre-filled with the card's brand name and DCI. The app
  itself stays offline.

## [0.23.0] - 2026-08-25

### Added
- The reference tables are editable in place: click a value, correct
  it, and it is shown in the accent colour with the shipped value on
  hover. "Annuler la dernière" undoes the last correction and
  "Rétablir la table" restores everything as shipped. The corrections
  are stored in the shared database and print with the table.
- A "Calculs" panel under the tables: clairance de la créatinine
  (Cockcroft & Gault, with the CKD stage), dose par kilo (par prise et
  par jour), and the decay curve of a drug — time to near-complete
  elimination and accumulation ratio at steady state, fed by any half-
  life from the drug base.

### Changed
- The tables view scrolls as a whole, so the sources, the corrections
  and the calculators stay reachable in a small window.

## [0.22.0] - 2026-08-25

### Added
- The dashboard opens on where the team left off: the last patients
  whose file moved (one click to reopen) and everything written today —
  the day's notes and the day's transmissions.
- The carnet is printed from an editable template, like the interview
  sheet and the CR letter: "Modèles…" gained a "Carnet" tab, validated
  and previewable, saved next to config.toml (`carnet_layout.typ`).
- Operator colours: each set of initials gets a stable colour, on the
  note stamps in every journal and on the printed carnet page, so a
  page can be scanned by who wrote what.

## [0.21.0] - 2026-08-25

### Added
- The agenda holds what is not a billable act: formations, réunions,
  livraisons, congés and free entries, created from the day panel and
  drawn on the grid in their own muted colour.
- A month view next to the week: a Monday-aligned grid with one chip
  per act and per entry, today highlighted, the days outside the month
  dimmed, and week/month navigation.
- A day panel under the grid — click a day (or a column header) to
  detail it: its rendez-vous with one-click access to the patient, its
  entries, and its own dated notes journal.

## [0.20.0] - 2026-08-25

### Added
- Four more fields on every drug card: statut administratif (badge
  coloured by what it says — rupture, retrait, hors AMM,
  commercialisé), évaluation SMR / ASMR, étiquettes, and toxicité /
  marge thérapeutique. All four print on the A4 monograph.
- The reference cards ship with their étiquettes and their toxicité
  derived from the monograph itself (classe, marge étroite,
  surveillance biologique, contre-indication grossesse, vigilance
  conduite), plus the encadrements that change dispensing (Previscan
  en poursuite seulement, NFS de la clozapine, ordonnance sécurisée du
  zolpidem, accord de soins du valproate).
- The drug search matches the class and the étiquettes as well as the
  brand and the DCI: typing "statine" or "marge étroite" finds the
  cards, ranked below an identity match.

## [0.19.0] - 2026-08-25

### Fixed
- The lock screen accepts `Entrée` again: pressing it made the field
  surrender focus, and the immediate re-focus cancelled the submission,
  so only the button worked.

### Added
- The fee matrix follows the quotas: an act limited to N per cycle
  shows N price columns, the ranks beyond it are struck out.
- The cycle length is configurable (`[rules] cycle_months`, 12 by
  default) — an entretien of year 0 and the first of year 1 are that
  many months apart, and the quota window follows.
- What happens when the quota is reached is now a choice: avertir
  (message with an explicit "créer quand même", the previous
  behaviour), informer seulement (the act is created, the rule is
  stated), or refuser (no override).

## [0.18.0] - 2026-08-25

### Added
- Drug cards open as a **monograph on a sheet of paper**: uppercase
  section headings over hairlines, the sections in reading order
  (indications, mécanisme d'action, posologie, contre-indications,
  interactions, effets indésirables, surveillance, conseils au
  patient), the pharmacokinetics as a definition list and the numbered
  sources at the foot. "Modifier" switches to the editable form,
  "Imprimer" typesets the same sheet as an A4 PDF.
- Six new fields on every card — indications, mécanisme d'action,
  contre-indications, effets indésirables, surveillance and sources —
  stored, edited and saved compare-and-set like the rest.
- Reference monographs for ~60 drugs, written at monograph depth: the
  anticoagulants (AOD, AVK, HBPM), the inhalers, the narrow-margin
  drugs, the oral anticancer drugs, and now the antibiotics and the
  psychotropes, each with its own numbered sources.

### Changed
- Reference tables carry **numbered sources** instead of a prose
  caution line, on screen and in the printout.
- The monograph headings read in full: "Posologie", "Conseils au
  patient", "Notes de l'équipe".

## [0.17.0] - 2026-08-25

### Added
- Reference clinical data on the ~30 drug cards the interviews turn
  around (the four AOD, the three AVK, énoxaparine, the inhalers,
  méthotrexate, lithium, digoxine, amiodarone, lévothyroxine,
  metformine, sémaglutide, capécitabine, dénosumab…): posology,
  interactions to watch, the advice to give the patient (plan de prise,
  technique, signaux d'alerte) and the pharmacokinetics. Cards outside
  that list keep their clinical fields empty, as before.
- "Compléter les fiches de référence" in Options fills those fields on
  an existing base, column by column and only where a field is still
  empty — the team's own text is never overwritten.
- Two more tables (seventeen in all): paediatric doses by weight, and
  what may be crushed or opened (LP and gastro-resistant forms,
  microgranules, dabigatran, cytotoxics) with the practical rules.

### Changed
- Posology, interactions, patient advice and elimination are now
  multi-line fields on the drug card, so a full reference text is
  readable without scrolling inside the field.
- The demo database no longer overrides the Eliquis card: it shows the
  shipped reference text.

## [0.16.0] - 2026-08-25

### Added
- Nine new reference tables, bringing the counter set to fifteen: AOD
  posologies with their renal adaptation, inhaled-corticosteroid dose
  steps, insulin action profiles, renal function (Cockcroft formula and
  CKD stages, with the metformine thresholds), the Mac Isaac score and
  what to do with the angina TROD, first-line cystitis treatments,
  missed-pill conduct, non-opioid analgesic doses, and the adult
  vaccination boosters — each with its own caution line, on screen and
  in the printed A4 reference.
- The starter drug base grows from ~200 to ~275 entries: more oral
  anticancer drugs (osimertinib, olaparib, ITK…), cardiology,
  pneumology and diabetes complements (insulins, tirzépatide, triple
  inhalers), neurology and psychiatry, gastro-enterology, urology,
  dermatology, ORL, ophthalmology, anti-infectives and immunology.

### Changed
- The tables view widened to 940 px and its selector wraps over several
  rows; long reference cells now wrap inside their column and the
  sunken box grows with the table instead of clipping it.

## [0.15.2] - 2026-08-25

### Fixed
- A partial fee table in `config.toml` no longer bills 0 €: writing
  `bpm = { initial = 65.0 }` (or misspelling a key) now keeps the
  default fee for every rank it does not mention, instead of zeroing
  the suivi fees in the dashboard, the chart and the CSV.
- Escape with the quick picker open closes the picker instead of
  leaving the patient view (and leaving the picker armed for the next
  patient).
- The theme chosen in the quick picker is dropped when the picker is
  closed without creating an act; it can no longer attach itself
  silently to a later act created from the direct buttons — including
  onto the CR letter and the export.
- The picker's 1-9 shortcuts are ignored while a field has the
  keyboard, so typing a duration or a date behind the dialog cannot
  create billable acts.

## [0.15.1] - 2026-08-25

### Added
- The thematic is printed on both documents: a "Thème" line in the
  interview sheet's header box and under the CR letter's subject
  (`{{THEME}}` in either template; an empty theme prints a dash).

### Changed
- The interview sheet's note boxes lost 2 mm each so the signature and
  next-RDV boxes still fit on the page under the new header line.

## [0.15.0] - 2026-08-25

### Added
- Per-rank fee schedule: each act kind is now paid by its rank inside
  the année d'accompagnement (entretien initial / 1er suivi / 2e suivi
  et au-delà). The Options dialog edits the nine acts as a matrix, and
  `config.toml` accepts both `bpm = { initial = 60, suivi_1 = 20,
  suivi_2 = 20 }` and the legacy flat `bpm_fee = 60`. Ranks follow the
  same 12-month cycles as the quota rules, and drive the dashboard,
  the patient table and the CSV export.
- Thematics on every entretien (observance, biologie/INR, technique
  d'inhalation, interactions…): a drop-down per row, compare-and-set
  like every other shared write, exported in a new CSV column.
- Quick act picker (Ctrl+N or "Choix rapide"): the nine acts with
  digit shortcuts and colour chips, plus the theme the new act will
  carry — one keystroke from patient to created act.
- Database maintenance in Options: "Compléter les médicaments de
  départ" tops up a base created before the starter list grew, and
  "Réinitialiser la base…" (two-step, red) wipes every row and reseeds
  the drugs — for debugging and demos.
- `BPM_CADDY_WINDOW=1280x1100` opens the window at a given size
  (screenshots, e2e).
- The drug view warns when the base holds only a handful of cards and
  points at the top-up button.

### Changed
- The Options dialog now sizes itself to the window instead of a fixed
  560 px, which clipped the last sections on small screens.

## [0.14.0] - 2026-08-24

### Added
- Three new act kinds complete the conventioned set: accompagnement
  AVK, accompagnement anticancéreux oraux, and vaccination — each with
  its own fee, yearly quota, agenda color, and act button (rows now
  wrap).
- Drug base grown to ~200 starter entries: oral anticancer drugs
  (capécitabine, imatinib, CDK4/6, hormonothérapie…), the missing HBPM
  brands (Innohep, Fraxiparine, Fragmine), Parkinson, antipsychotics,
  uro/gynéco, os/rhumato, and more counter staples.
- Database file tools in Options: browse to an existing base with a
  native file dialog, write a consistent encrypted copy anywhere
  (VACUUM INTO), or move the base — copy, repoint config, old file
  kept as a fallback.

## [0.13.0] - 2026-08-24

### Added
- Carnet de transmissions ("Carnet", F5): the end-of-day team handover
  logbook — one page per day, entries stamped heure · opérateur,
  chronological within the day, browsable day by day (‹ jumps to the
  previous day with entries, "Aujourd'hui" returns), and printable as
  an A4 page for the binder. Past pages are read-only: new entries
  always land on today's page

### Changed
- The toolbar gained "Carnet (F5)"; the version number moved into the
  BPM-Caddy tooltip (with the database and config paths) so all eight
  buttons fit at the default width; "Modèle PDF…" became "Modèles…"

## [0.12.0] - 2026-08-24

### Added
- Standalone dated notes: an append-only journal (date · heure ·
  opérateur, deletable with confirmation) attached to each patient
  ("Notes de suivi" on the patient page), each drug ("Notes datées" on
  the drug page), and each operator (personal notes at the bottom of
  the documentation pane, keyed by the operator initials). Patient and
  drug journals are removed with their subject; operator notes are
  personal and survive

## [0.11.0] - 2026-08-24

### Added
- Drug pages: each card grows into a two-column page — "Fiche
  clinique" (identity, dosage, interactions, IUP, antidote, notes) and
  "Pharmacocinétique" (demi-vie, AUC / exposition, élimination,
  adaptation DFG, grossesse / allaitement); all team-filled, saved with
  compare-and-set like the rest

### Changed
- Professional layout pass: every view now aligns to a fixed-width
  centered content column (`motif::column`) — headings centered,
  content on one left grid; the last dozen magic centering offsets are
  gone, form grids share a common label column width, the alert color
  is a single `motif::ALERT`, and the window has a minimum size so
  layouts cannot collapse
- The note-stamp timestamp is only queried on click instead of every
  frame (was one SQLite query per frame with the docs pane open)

## [0.10.0] - 2026-08-24

### Added
- Convention rules enforced at act creation: each kind allows N acts
  per "année d'accompagnement" (12 months from the cycle's first act;
  the next cycle starts at least 12 months later). A blocked creation
  explains the rule and shows the next possible date, with an explicit
  "Créer quand même" override. Quotas configurable per kind
  (`[rules]`, 0 = no limit; defaults: BPM/AOD/Asthme 3, TROD 0,
  Prévention 1)
- Global options editor ("Options…" in the toolbar): pharmacy identity,
  interface, auto-lock, backups, database path, fees, and yearly-rule
  quotas — all edited in-app and saved to config.toml, applied live
  (path change takes effect on restart). The master-password change
  moved inside it, keeping the toolbar compact

## [0.9.0] - 2026-08-24

### Added
- Conversion tables at the counter ("Tables de conversion" in the drug
  view): IPP dose equivalences, HBPM usual dosing (curatif /
  prophylaxie), statine equivalent doses, corticoid anti-inflammatory
  equivalences, opioid equianalgesia (réf. morphine orale), and
  benzodiazepine equivalences (Ashton) — each with its caution line,
  browsable per tab and printable as a two-page A4 reference

## [0.8.0] - 2026-08-24

### Added
- Motif list boxes: patient and drug searches render in proper sunken
  list panels with full-width selection bars, hover tint, and tight
  rows (new `motif::list_row` / `list_row_job` / `section` widgets) —
  fuzzy-match highlighting kept
- A status bar: patient / in-progress / drug counts on the left, the
  database file on the right (replaces the under-search totals line)
- "Entretiens" section separator on the patient view
- The starter drug base grows from ~58 to ~135 common French drugs
  (anti-infectieux, AINS, gastro, allergie, psychiatrie, neurologie,
  cardio-métabolisme, divers) — still brand + DCI + class only, with
  textbook antidotes where they exist

## [0.7.0] - 2026-08-24

### Added
- CR letter to the médecin traitant ("CR" button on each interview
  row): a Typst-generated letter with the pharmacy letterhead
  (`[pharmacy]` in config.toml — name, address, phone, pharmacist),
  the addressed physician, the act and date, the patient's known
  treatments (name, DCI, class, dosage), and boxes for the handwritten
  synthesis and signature; names are escaped like everywhere else
- The template editor now handles both templates: "Fiche entretien"
  and "Courrier CR" tabs, each validated, previewable with sample
  data, and saved to its own file (`cr_layout.typ` next to config.toml
  or `[templates] cr_template_path`)
- Reverse treatment lookup on the drug card: "Patients sous ce
  traitement" chips (recall / alert question), one click from the
  patient's record

## [0.6.0] - 2026-08-24

### Added
- Fuller patient record: médecin traitant, e-mail and address, shown
  on the patient view and edited via "Modifier" (compare-and-set like
  the rest); the CR recipient is finally on the record
- Current treatments on the patient: drugs linked from the shared base
  as chips on the patient view — click opens the drug card, "×"
  unlinks, and a small fuzzy picker (brand or DCI) adds one; links are
  removed atomically with the patient
- Drug cards gain the therapeutic class ("Classe"), shown in the card
  header and the search rows; the ~58 starter drugs now carry their
  class (AOD, AVK, statine, IPP, benzodiazépine…)

## [0.5.0] - 2026-08-24

### Added
- The agenda opens on a colored week grid (Mon–Sun, current week by
  default): one block per RDV, colored by act kind, today highlighted,
  hover shows patient/kind/phone, click opens the patient; week
  navigation (‹ Aujourd'hui ›) and a color legend; the day-grouped
  list (with overdue) stays below
- New billable acts: TROD angine, TROD cystite, and RDV prévention —
  buttons on the patient view, own colors, fees in `[billing]`
  (`trod_angine_fee`, `trod_cystite_fee`, `prevention_fee`), counted
  everywhere (dashboard, CSV, PDF sheets)
- In-app editor for the Typst PDF template ("Modèle PDF…" in the
  toolbar): edit the sheet's source with validation (invalid templates
  are refused with the Typst error), a sample-patient PDF preview, and
  reset-to-default; saved next to `config.toml` (or at
  `[templates] bpm_template_path` when configured) and picked up by the
  next "Fiche PDF"
- Toolbar labels shortened so all views fit at the default window width

## [0.4.0] - 2026-08-24

### Added
- Agenda ("Agenda", F4): the upcoming patient appointments grouped by
  day with French weekday names, "aujourd'hui / demain / en retard"
  flags, phone numbers, one-click access to the patient, and printing
- Drug cards gain the DCI (dénomination commune internationale): shown
  under the name, searchable ("elix" and "apixa" both find Eliquis),
  and included in notes inserts; the card layout was reworked (identity
  header, antidote banner in red, dim labels, wider fields)
- A fresh drug base is seeded with ~55 common French drugs (brand name,
  DCI, and textbook antidotes only — dosage/interactions/IUP are left
  for the team to fill from the references they trust); seeding happens
  once and never resurrects deliberately deleted cards
- All UI strings live in an embedded TOML (`assets/strings.fr.toml`);
  any wording can be overridden — or the app translated — by a
  `strings.toml` placed next to `config.toml`, without recompiling
- Patient forms polished: dim labels, wider fields, consistent with
  the drug card
- Drug reference base ("Médicaments", F3): team-shared encrypted cards
  (dosage, interactions, IUP, antidote, notes personnelles) with the
  same fuzzy search / quick-create / compare-and-set workflow as
  patients — typing two letters shows dosage and antidote at a glance,
  and "→ Notes d'équipe" inserts name + dosage into the shared notes
- Note-entry aids in the documentation pane: an "Opérateur" field
  (default from `[ui] operator` in config.toml) and a "+ Entrée" button
  stamping "— date heure · opérateur · patient courant : " into the
  notes, for succinct team entries
- Discreet finances (on by default, `[ui] discreet_finances`): dashboard
  amounts are masked ("•••") and the monthly revenue chart hidden; a
  small unlabeled control in the dashboard corner reveals them, and they
  re-mask on leaving the dashboard or locking
- "RDV à venir" on the dashboard: planned interviews not yet performed,
  soonest first, overdue ones flagged in red ("en retard"); clicking a
  row opens the patient (never masked — dates are not financial data)
- A misclicked state advance can be undone: each interview row gains a
  small "«" button that steps back to the previous pipeline state,
  including un-billing
- Enter submits the quick-create patient form from any of its fields
  (no mouse needed, per the shortcut-driven spec)
- Patients can be found by typing their phone number in the search, the
  patient list is kept alphabetical (accent-insensitive) when browsing
  with an empty query, and the CSV export includes the phone column
- The number of daily backups kept is configurable
  (`[database] backups_keep`, default 14, 0 disables them)
- CSV export from the dashboard ("Exporter CSV") for billing
  reconciliation: every interview with patient, dates, duration and
  fee, written to `exports/` next to the database and opened in the
  default spreadsheet (French Excel conventions: BOM, semicolons,
  decimal comma)
- The app and launcher windows have an icon (`motif::icon()`, a Motif
  bevel square drawn programmatically) so they are recognizable in the
  taskbar and alt-tab
- A commented `config.toml` template is written on first launch, so the
  available options are discoverable without reading the documentation
- Launcher: network timeouts (10 s connect, 30 s per read) so a hung
  connection can no longer block startup, and the downloaded binary's
  size is verified against the release metadata before it replaces the
  installed copy (no more silently truncated updates)
- Fiche PDF: embedded fonts are parsed once per session instead of on
  every click, and each sheet gets a unique file name so regenerating
  while the previous PDF is still open no longer fails on Windows
- Automatic daily backups: after each unlock, a consistent encrypted
  snapshot (`VACUUM INTO`) is written to `backups/bpm_caddy-AAAA-MM-JJ.db`
  next to the database; the 14 most recent are kept
- The master password can be changed from the toolbar ("Mot de passe…"):
  the database is re-encrypted (SQLCipher rekey) and a password
  remembered in the OS credential manager is updated in place
- Patient records can be corrected and deleted from the patient view:
  "Modifier" edits the identity (name typo, wrong birth date), and
  "Supprimer…" removes the patient with a two-step confirmation (the
  patient's interviews are deleted atomically with them)
- A single interview can be removed with the "×" button on its row
  (two-step confirmation), for entries added by mistake
- Patients gain a phone number and a free-form comment (allergies,
  preferences…), edited via "Modifier" and shown on the patient view;
  the dashboard's "RDV à venir" list shows the phone so the patient can
  be called about the appointment
- The RDV list can be printed ("Imprimer" next to "RDV à venir"): a
  Typst-generated A4 table of the upcoming appointments with phone
  numbers, opened in the PDF viewer (patient names are safely escaped)
- Escape leaves the dashboard back to the search, and appointments
  scheduled for today are highlighted "aujourd'hui" on the dashboard
- Search results show a "n entretien(s) en cours" badge for patients
  with not-yet-billed interviews, and the letters matched by the fuzzy
  query are underlined in the result names
- The dashboard shows the interview count per type under the funnel,
  and the "Fiche PDF" is dated with the planned RDV when one is set
- The interview table has column headers, the search screen shows the
  patient / in-progress totals, and hovering the version number reveals
  the database and configuration paths in use (multi-post support aid)
- Error messages clear as soon as a following operation succeeds
  instead of lingering
- The open patient view follows background refreshes: identity edits
  from another post appear within a minute, and the view closes if the
  patient was deleted elsewhere
- `scripts/screenshots.sh` regenerates the README screenshots
  reproducibly (seeded demo, xvfb)
- Multi-PC robustness on a shared database: a 5-second busy timeout
  instead of immediate "database is locked" errors; state advances are
  compare-and-set (a click based on a stale view is rejected with a
  message instead of silently overwriting a colleague's change); open
  views re-read the database every minute; the quick-create form
  re-checks the patient list before offering creation (no duplicates
  when another post just created the patient); shared team notes pick
  up other posts' edits while clean and merge line-by-line on
  concurrent saves instead of last-writer-wins
- Compact date entry everywhere a date is typed: "230826" (JJMMAA),
  "23082026" (JJMMAAAA), "2308" / "23/08" (current year), and two-digit
  years in separator form ("3/7/58"). Two-digit years expand by context:
  birth dates never land in the future ("49" → 1949), RDV dates are
  always 20xx

### Fixed
- Review round on the multi-post work: RDV dates, durations, patient
  corrections and interview deletions are now all compare-and-set (a
  stale field or form can no longer silently revert or destroy a
  colleague's newer change — deleting an interview a colleague meanwhile billed is
  refused); an RDV typed but not tabbed out of is committed when the
  view changes or the app locks; patient names are escaped in the
  interview sheet too (Typst injection); a yearless date ("2308") is
  rejected for birth dates instead of storing a current-year birth; the
  CSV gains a "Facturé (€)" column so summing it matches the dashboard
  (the tariff column alone over-declared); the shared-notes merge no
  longer rewrites the text under a focused cursor; the daily backup
  runs on a background thread (no UI freeze at unlock on a network
  share); the notes sync only polls while the pane is shown; and the
  quick-create duplicate check is throttled instead of re-reading the
  database on every keystroke
- Dates are validated for real: 31/02, 31/04 or 29/02 outside leap years
  are now rejected instead of being stored as impossible ISO dates
- The interview creation date is displayed as JJ/MM/AAAA instead of ISO
- Escape while typing in a field of the patient view only drops focus
  (egui's behavior) instead of also closing the view and discarding the
  in-progress edit
- Fuzzy search now folds uppercase accented letters ("ÉMILE" matches
  "emile"); previously only lowercase accents were stripped
- Quick-create opens the patient by the id returned from the insert
  instead of relying on unspecified row order
- The team documentation pane is never shown on the lock screen, and a
  dirty document auto-saves even while the pane is hidden

## [0.3.0] - 2026-08-22

### Added
- Time tracking per interview (inline "min" field) feeding the hourly ROI
  KPI ("Taux horaire") on the dashboard
- Master password can be remembered in the OS credential manager (Windows
  Credential Manager, macOS Keychain, Secret Service on Linux) for silent
  unlock at startup; unchecking the box removes the stored copy
- Planned interview dates ("RDV JJ/MM/AAAA" per interview row)
- "Verrouiller" toolbar button and Ctrl+F back-to-search shortcut
- Screenshots in the README, captured from the running app

### Fixed
- Dashboard KPI row no longer overflows narrow windows; monthly chart
  labels months as MM/YY

## [0.2.0] - 2026-08-22

### Added
- Encrypted patient database: SQLCipher (256-bit AES) with a master-password
  unlock screen; wrong passwords are rejected before any data is touched
- Diacritic-insensitive fuzzy patient search ("jndp" finds "Jean Dupont"),
  keyboard navigation (arrows + Enter), and seamless quick-creation form
  (Nom / Prénom / Date de naissance) when no patient matches
- Interview lifecycle state machine (Identifié → Planifié → Réalisé →
  CR envoyé → Facturé) with one-click advancement from the patient view
- `config.toml` support: database path (shareable network drive — the team
  documentation follows the database), auto-lock timeout, per-kind billing
  fees, UI defaults
- Auto-lock: the app returns to the password screen after the configured
  inactivity timeout
- Financial dashboard (F2): billed vs pending revenue KPIs, pipeline
  funnel, and a monthly billed/pending bar chart, all Motif-styled
- Embedded Typst engine: one-click "Fiche PDF" from the patient view
  compiles a single-page A4 interview sheet (patient header + rounded
  boxes for handwritten notes) in memory and opens it in the OS PDF
  viewer; the template is overridable via `[templates]` in `config.toml`

## [0.1.0] - 2026-08-22

### Added
- Project specification (`docs/SPECIFICATIONS.txt`)
- Application skeleton (egui shell)
- Release build pipeline for Windows / macOS / Linux
- `bpm-caddy-launcher`: auto-updating launcher that fetches the latest release
  on startup, with a download progress bar and an offline fallback to the
  installed version
- `motif` crate: old-school X/Motif theme for egui (mwm blue-grey palette,
  square corners, raised/sunken bevels, Motif-style buttons and progress bars),
  applied to both the app and the launcher
- Docked team documentation pane in the app (French, `F1` to toggle,
  debounced auto-save) for shared notes at the counter
