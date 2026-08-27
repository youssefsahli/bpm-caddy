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
  exception et le dit dans son nom.

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

- **Où** : `src/revue.rs`, `RULES` : soit une `Combination` (chaque
  groupe de mots doit trouver un traitement — une association fixe
  compte pour ses deux moitiés), soit un `Duplicate` (N traitements
  distincts portant l'un de ces mots).
- **Test** : chaque règle doit pouvoir se déclencher sur la base
  livrée ; chaque règle doit dire quoi faire, pas seulement ce qui ne va
  pas.

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
