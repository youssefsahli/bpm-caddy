# Le contenu clinique de BPM-Caddy — où il vit, comment on l'étend

L'application porte plus de contenu que de code. Ce document dit où
chaque chose est écrite, ce qui la contrôle, et ce qui se passe quand on
en ajoute. Il s'adresse à qui reprend le projet — y compris à moi-même
dans six mois.

Deux règles valent partout :

1. **Ce que l'équipe écrit dans l'application n'est jamais réécrit par
   une mise à jour.** Chaque contenu livré est semé une fois, et une
   fiche modifiée, vidée ou supprimée reste comme l'équipe l'a laissée.
2. **Chaque contenu a son test.** Un test qui vérifie sa forme (rien de
   vide, les colonnes alignées, les sources présentes) et, pour les
   règles, qu'elles peuvent effectivement se déclencher sur la base
   livrée. Du contenu inatteignable est du contenu que personne ne
   corrigera jamais.
3. **Chaque catalogue a son cliquet.** Un plancher sur le nombre de
   fiches, de tables, de préparations, de dispositifs, de protocoles,
   d'analytes et de règles : il ne peut que monter. Un contenu retiré
   est une question à laquelle le comptoir ne sait plus répondre, et
   sans plancher cela arrive sans que personne le voie. Le cas
   particulier est `the_posology_coverage_only_improves`, qui compte ce
   qui *manque* — les classes sans posologie — et dont le plafond ne
   peut que descendre. Il exige d'être abaissé dès qu'il prend de
   l'avance sur la réalité, ce qui l'empêche de devenir décoratif.

## Les fiches médicament

- **Où** : `src/db.rs`, `STARTER_DRUGS` (nom, DCI, classe, antidote) et
  `STARTER_DETAILS` (la monographie complète).
- **Semé par** : `Db::seed_drugs_if_empty` sur une base vide, et
  `fill_starter_details` pour compléter une base ancienne — champ par
  champ, sans jamais toucher à un champ que l'équipe a modifié
  (`drug_field_locks`).
- **Tests** : `every_starter_card_carries_a_sourced_monograph` exige
  treize champs remplis et des sources pour chaque fiche.
- **Ajouter une fiche** : une ligne dans `STARTER_DRUGS` **et** une
  entrée `StarterDetail` du même nom. Le test refuse l'une sans l'autre.

## Les posologies par indication

- **Où** : `src/db.rs`, `STARTER_POSOLOGIES` : `(spécialité, indication,
  posologie, remarque)`, dans l'ordre d'affichage.
- **Semé par** : `Db::seed_posologies`, sur les fiches qui n'ont aucune
  ligne. Une fiche dont l'équipe a commencé la liste est laissée
  entière.
- **Tests** : chaque ligne nomme une fiche réelle, et les lignes d'une
  même spécialité se suivent (un bloc coupé en deux perd sa seconde
  moitié au semis).
- **La remarque est le contenu** : la posologie se trouve dans un RCP,
  la remarque est ce qu'on dit au comptoir. Une remarque vide est un
  test qui passe et une ligne qui ne sert à rien.
- Les classes titrées par le spécialiste (antiépileptiques,
  antipsychotiques, immunosuppresseurs, inhibiteurs JAK,
  immunomodulateurs de la SEP) sont **délibérément vides**.

## « En cas d'oubli » et « Ce qui doit faire consulter »

- **Où** : `src/db.rs`, `STARTER_CONDUITE` : `(mot-clé, conduite en cas
  d'oubli, signes qui font consulter)`.
- **Comment ça matche** : le mot-clé est cherché dans la classe, les
  étiquettes, la DCI et le nom de la fiche. La première règle qui
  correspond remplit la fiche ; les règles les plus spécifiques se
  placent donc avant les plus générales.
- **Semé par** : `Db::seed_conduite`, seulement sur un champ vide et non
  verrouillé. La passe laisse une marque dans `seed_state` (le nombre de
  règles appliquées) : elle ne réécrit rien tant que ce nombre ne change
  pas.
- **Test** : chaque mot-clé doit atteindre au moins une fiche de départ.

## Les tables de référence

- **Où** : `src/tables.rs`, `TABLES` : nom court, titre, date de
  relecture, sources numérotées, colonnes, lignes.
- **Modifiable en place** : l'équipe corrige une cellule dans
  l'application (`table_cells`), et c'est sa version qui s'affiche, se
  cherche et s'imprime.
- **Tests** : pas de nom court en double, pas de cellule vide, largeur
  de ligne égale au nombre de colonnes, date de relecture présente, et
  les familles dont les actes dépendent doivent exister.
- **Ajouter une table** : une entrée `ConvTable`, et le nom de la
  famille dans la liste du test si elle est structurante.

## Le codex des préparations

- **Où** : `src/db.rs`, `STARTER_PREPARATIONS` ; la lecture et la mise à
  l'échelle des formules sont dans `src/codex.rs` (pur, testé).
- **Format d'une formule** : une ligne par matière première, `nom |
  quantité`, la quantité étant un nombre et une unité (`5 g`) ou un
  `qsp 100 g`. `yield_amount` dit ce que la formule produit — c'est ce
  qui permet de la mettre à une autre quantité.
- **Semé par** : `Db::seed_preparations`, une fois, par nom.
- **Tests** : toute formule doit se lire, avoir un rendement lisible et
  survivre à une mise à l'échelle sans perdre de ligne ; une « formule
  type » (dont les quantités viennent de l'ordonnance) est la seule
  exception et le dit dans son nom. Le compte est un **cliquet** : 80
  préparations aujourd'hui, et le test refuse d'en voir moins — une
  formule retirée est une formule que le préparatoire doit rouvrir un
  livre pour retrouver. Deux préparations ne peuvent pas porter le même
  nom.

## Les protocoles et les algorithmes de prise en charge

- **Où** : `src/db.rs`, `STARTER_PROTOCOLS` — un arbre par protocole,
  fait de questions (`q`) et de conduites (`act`).
- **Deux familles dans la même liste** : les protocoles *de comptoir*
  répondent à ce qui arrive (une rupture, une piqûre de tique, un oubli
  de pilule) ; les *algorithmes de prise en charge* répondent à ce que
  l'ordonnance devrait porter — la stratégie recommandée d'une maladie
  chronique, déroulée pas à pas. Le pharmacien ne prescrit pas ; il
  vérifie, il explique et il signale ce qui manque, et pour cela il faut
  connaître la marche.
- **Un algorithme cite sa source dans son sujet** (ESC, HAS, SFHTA,
  GINA…) et le dit : ce sont des recommandations et pas des ordres, et
  un prescripteur qui s'en écarte a le plus souvent une raison qui n'est
  pas sur l'ordonnance.
- **Semé par** : `Db::seed_protocols`, une fois, par titre.
- **Tests** : `every_protocol_asks_before_it_answers` — un protocole
  demande avant de répondre (au moins une question), toute question se
  termine par un point d'interrogation, aucune conduite n'est vide, pas
  deux titres identiques, et le compte est un cliquet.

## Le registre des stupéfiants

- **Où** : deux tables dans `src/db.rs` — `stupefiants` (les produits
  suivis, leur unité, leur seuil) et `stup_moves` (le registre). Le
  calcul est dans `src/ordonnancier.rs` (pur, testé).
- **Rien n'est livré** : un registre est celui de l'officine, et une
  ligne semée serait une ligne que personne n'a écrite. L'équipe ajoute
  les produits qu'elle suit.
- **La règle qui décide de tout** : le registre est inaltérable. Pas
  d'`UPDATE`, pas de `DELETE`, pas de méthode qui en proposerait. Une
  erreur se contre-passe. Un test relit `db.rs` et le refuse.
- **La balance n'est pas une somme** : un inventaire *pose* le solde.
  Additionner l'écart et poser le compte le compterait deux fois, et le
  registre dériverait dès le premier comptage qui ne tombe pas juste.
- **Le numéro d'ordonnancier** est séquentiel dans l'année, attribué par
  la base dans la transaction qui écrit la ligne, et jamais réattribué.
- **Réglages** : `[stock] suppliers` (les grossistes, le premier étant
  celui qu'on propose — c'est pour la vitesse de saisie) et
  `[stock] count_days` (au bout de combien de jours un produit non
  recompté revient sur la liste de contrôle ; la loi en demande un par
  an, c'est le rythme que l'officine se donne).

## Les pièces numérisées

- **Où** : la table `scans` dans `src/db.rs` ; ce qu'un fichier *est* et
  ce qu'une pièce peut être sont dans `src/scans.rs` (pur, testé).
- **Rien n'est livré** : ce sont les papiers de l'officine.
- **Le format se lit dans les octets, jamais dans le nom.** PDF, PNG,
  JPEG, TIFF, et le reste est refusé à l'entrée. L'application ressort
  la pièce sur le disque et la confie au système : ce qu'elle a accepté
  est ce qu'elle rendra, et l'extension vient du contenu.
- **Chiffrées, mais dans leur propre fichier** : `<base>_scans.db` à côté
  de la base, même SQLCipher et même clé. Une ordonnance numérisée posée
  en clair annulerait le chiffrement ; la laisser *dans* la base la fait
  grossir sans mesure — 200 ordonnances N&B portent une base de 6 Mo à
  56, et les quatorze sauvegardes quotidiennes à 840. La base garde la
  **fiche** de chaque pièce, pas ses octets : une base copiée seule
  montre encore ce qui existait.
- **Ce que la séparation impose** : `change_password` rechiffre les deux
  fichiers (un test le tient), « Copier la base… » copie les deux,
  `[scans] backups_keep` est à part (2 par défaut, contre 14 pour la
  base), et la lecture retombe sur l'ancienne colonne `bytes` pour qu'une
  base d'avant la séparation ouvre encore ses pièces.
- **SQLite ne rend jamais la place** : supprimer une pièce libère des
  pages, pas le fichier. « Compacter la base » déplace ce qui restait,
  balaie les orphelins, puis réécrit les deux fichiers — dans cet ordre.
- **Les octets ne se réécrivent pas** : une numérisation est ce que le
  scanner a produit. Le genre, le libellé, la date et la remarque se
  corrigent (compare-and-set) ; le fichier, non. On en range un autre.
- **Le genre est un vocabulaire** : ordonnance, accident du travail,
  biologie, courrier, facture, autre. La liste se colore dessus, et un
  genre inventé une fois resterait seul.
- **Le scanner est en configuration** (`[scans] command`, `{out}` étant
  le fichier à lire), comme la séquence APDU de la carte Vitale : le
  matériel d'une officine n'est pas connu du binaire. Vide = seul
  « Importer… ».

## Les dispositifs médicaux

- **Où** : `src/db.rs`, `STARTER_DISPOSITIFS` : nom, famille, indication,
  formes et tailles, pose, renouvellement, ligne LPP, ce qui va de
  travers, étiquettes, sources.
- **Semé par** : `Db::seed_dispositifs`, une fois, par nom, avec une
  marque dans `seed_state` : une base que l'équipe a vidée exprès reste
  vide — les fiches ne reviennent pas discuter. Seul
  « Réinitialiser la base… » les ramène, en effaçant les marques.
- **La famille est un vocabulaire, pas un texte libre** : la liste et
  l'impression groupent dessus, donc une famille inventée une fois reste
  seule pour toujours. Les onze familles sont dans le test.
- **La ligne LPP porte la règle, jamais le prix** : ce que la
  prescription doit mentionner, ce qui entre dans un forfait, ce qui se
  facture à part. Le tarif se vérifie sur ameli.fr au moment de la
  délivrance, et le test refuse un « € » dans ce champ — un tarif livré
  est un tarif faux dans l'année.
- **Test** : `the_dispositifs_seed_once_and_answer_the_counter` — noms
  uniques, neuf champs remplis sur chaque fiche, famille dans le
  vocabulaire, chaque famille utilisée, le matériel de location présent,
  écriture compare-and-set sur **toutes** les colonnes (une fiche n'a
  pas un champ qui porte tout son poids), et une base vidée qui le
  reste.
- **Ajouter une fiche** : une entrée `StarterDispositif`, une famille
  déjà utilisée, et les neuf champs. Ou, ce qui est mieux, directement
  dans l'application — c'est le contenu de l'équipe.

## Les protocoles

- **Où** : `src/db.rs`, `STARTER_PROTOCOLS` : un arbre de questions
  (`q(...)`) et de conduites (`act(...)`).
- **Semé par** : `Db::seed_protocols`, une fois, par titre. Un arbre
  réécrit par l'équipe n'est jamais remplacé ; un titre supprimé ne
  revient pas.
- **Ce que c'est** : pas seulement les ruptures. Un protocole vaut aussi
  pour ce qui entre sans ordonnance — un oubli de pilule, une piqûre de
  tique, une brûlure, une douleur thoracique — c'est-à-dire pour les
  situations où la bonne conduite est une suite de questions, pas une
  ligne de posologie.
- **Tests** : une seule racine, chaque branche pend d'une question, et
  chaque question porte ses deux réponses — un déroulé qui s'arrête sur
  « non » est pire que pas de protocole. Et
  `every_protocol_asks_before_it_answers` : au moins deux questions et
  trois conduites par arbre, des titres distincts, des questions qui se
  terminent par un point d'interrogation, et aucune conduite trop courte
  pour être suivie. Le nombre de protocoles livrés ne baisse jamais.

## La biologie

- **Où** : `src/biology.rs` — `CATALOGUE` (les analytes, leurs
  intervalles usuels et leurs seuils critiques) et `RULES` (ce qu'une
  valeur change pour un traitement).
- **Comment une règle matche** : un analyte, un côté (au-dessus ou
  au-dessous d'un seuil), et des mots cherchés dans les traitements du
  dossier (nom, DCI, classe, étiquettes). `needs` vide = la règle vaut
  pour tout le monde.
- **Tests** : chaque règle nomme un analyte du catalogue, chaque règle
  doit pouvoir se déclencher sur la base livrée, et **chaque analyte du
  catalogue porte au moins une règle** — un analyte sans règle n'est
  qu'un chiffre recopié du laboratoire, qui le fait déjà mieux. Ajouter
  un analyte, c'est écrire sa règle dans le même mouvement.
- Les intervalles sont ceux de l'adulte, et l'application le dit :
  celui du laboratoire prime toujours.

## La revue d'ordonnance

- **Où** : `src/revue.rs`, `RULES` : une `Combination` (chaque groupe de
  mots doit trouver un traitement — une association fixe compte pour ses
  deux moitiés), un `Duplicate` (N traitements distincts portant l'un de
  ces mots), ou un `Without` (tous les groupes matchent **et** rien sur
  l'ordonnance ne répond au dernier).
- **Ce que `Without` sert** : ce qui *manque* est la moitié de ce qu'un
  bilan trouve — un opioïde sans laxatif, une corticothérapie sans rien
  pour l'os. Le point nomme les traitements qui *sont* là, puisqu'une
  absence n'a pas de nom, et la phrase dit ce qui n'y est pas. La règle
  se tait dès que la ligne manquante apparaît, et c'est ce que le test
  vérifie.
- **Test** : chaque règle doit pouvoir se déclencher sur la base livrée
  — pour un `Without`, la chose dont l'absence est le constat doit
  exister aussi, sans quoi la règle parle de la base et non du patient.
  Chaque règle doit dire quoi faire, pas seulement ce qui ne va pas, et
  le nombre de règles ne baisse jamais.

## Ce qu'un traitement demande de surveiller

- **Où** : `src/surveillance.rs`, `WATCHES` : les mots qui désignent le
  traitement, le code d'un analyte de `biology::CATALOGUE`, le rythme en
  mois, et la raison.
- **La question qu'il pose** : la biologie répond à « ce chiffre, sous ce
  traitement, qu'est-ce que ça change ». Celui-ci pose la question
  d'avant — **quel chiffre n'a pas été demandé depuis trop longtemps**.
  Une règle ne peut rien dire d'un examen qu'on n'a pas fait, et c'est le
  trou que personne ne voit.
- **Tests** : chaque surveillance nomme un analyte du catalogue, dit
  pourquoi en une phrase (et pas en une étiquette), et doit pouvoir se
  déclencher sur la base livrée. Le compte est un cliquet.
- **Ajouter une surveillance** : une entrée `Watch`. Si l'analyte n'existe
  pas encore, il faut d'abord l'ajouter à `biology::CATALOGUE` — avec sa
  règle, que ce catalogue-là exige.
- **Ce qui ne s'y met pas** : un rythme que le prescripteur seul décide.
  Les rythmes sont ceux des RCP et des recommandations usuelles ; c'est
  un aide-mémoire de comptoir, et l'application le dit.

## La conciliation médicamenteuse

- **Où** : `src/conciliation.rs`. C'est le seul module « logique » qui ne
  porte **aucun contenu** : ni catalogue, ni règles écrites à la main.
  Tout ce qu'il sait, il le tient de ce qu'on lui passe — les
  traitements du dossier, la feuille collée, et la base de fiches.
- **Ce qui s'ajuste quand même** : la lecture d'une ligne
  (`split_line`), qui doit survivre aux vingt façons dont une ordonnance
  de sortie est écrite, et le rapprochement (`match_name`), qui refuse
  de deviner sur moins de trois lettres. Une feuille d'un format que le
  module lit mal se corrige **là**, avec un cas de plus dans
  `a_line_gives_up_its_product_and_its_dose` — jamais par une liste de
  noms en dur.
- **La règle qui compte** : le rapprochement passe par les traitements du
  dossier avant la base entière. Une reconduction affichée comme un
  arrêt suivi d'un ajout est une divergence inventée, et c'est la pire
  des deux erreurs possibles.
- **Test** : la comparaison doit rendre exactement la même chose deux
  fois de suite (elle s'affiche à chaque image), une ligne non
  rapprochée doit rester visible, et un remplacement de classe doit
  faire une ligne et non deux.

## Les listes d'entretien

- **Où** : `src/entretien.rs` — une liste par thème, et un fond commun
  pour les thèmes que l'officine écrit elle-même.
- **Test** : chaque thème de `db::THEMES` a sa liste, de cinq à neuf
  points. Une liste de vingt lignes est une liste que personne ne coche.

## Le calendrier vaccinal et les voyages

- **Où** : `src/vaccines.rs` — le catalogue des vaccins, les règles du
  calendrier (`due_lines`) et la table des pays.
- Pur et testé, sans horloge interne : la date du jour est passée en
  paramètre.

## Retrouver le contenu : « Dans le texte… »

La recherche plein texte (`mono_search`, `src/app.rs`) lit treize
sections de chaque fiche **et** les lignes de posologie, et rend la
phrase qui porte le mot. Un champ de prose ajouté à une fiche n'est
cherchable qu'une fois inscrit dans `MONO_FIELDS`, avec sa clé de
libellé : un champ absent de cette table est un champ que personne ne
retrouvera par ses mots. C'est le pendant de la règle du haut — tout
contenu a son test, et tout contenu a un chemin qui y mène.

## Les mentions imprimées

Elles ne sont **pas** du contenu livré : `[disclaimers]` dans
`config.toml` est vide par défaut, et l'application n'ajoute aucun
avertissement de son propre chef. Ajouter une mention, c'est ajouter une
clé dans `DisclaimersConfig`, un champ dans Options › Mentions, et
l'endroit qui l'imprime — jamais un texte en dur.
