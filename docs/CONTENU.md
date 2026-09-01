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
- **Rempli n'est pas écrit** : le test ne voyait que le vide, et une
  fiche dont les contre-indications tenaient dans « Hypersensibilité. »
  et le rein dans « Pas d'adaptation. » lui passait sous le nez. Dix des
  douze monographies les plus maigres de la base étaient des
  biosimilaires — un biologique injectable, surveillé, à côté d'un
  médicament de référence qui portait, lui, une page entière. Or c'est
  la fiche du biosimilaire que le comptoir a en main : le patient a été
  substitué, il n'ouvrira pas celle de la référence. Le test tient
  maintenant deux planchers sur les fiches étiquetées `biosimilaire` —
  trois mille caractères de corps clinique, et quatre-vingts par champ,
  de quoi faire une phrase et pas une étiquette.
- **La longueur n'est pas un critère partout** : une pastille pour la
  gorge ou une argile n'ont pas de marge dont parler, et leurs fiches
  sont courtes à juste titre. Le plancher ne vaut que là où la brièveté
  est une omission.
- **La section « Toxicité / marge thérapeutique »** n'est due qu'aux
  fiches où **une dose, une durée ou une exposition tue** — un
  antifongique local n'a pas de marge dont parler, et l'y forcer
  diluerait la section partout ailleurs. Elle porte ce qui se vérifie
  au comptoir et pas ce qui se lit dans un RCP : le chiffre qui décide
  (la clairance, le poids, l'INR, la kaliémie), l'association qui tue,
  la voie ou le geste à ne pas se tromper, ce qui arrive à l'arrêt, et
  ce que le patient ne dira jamais de lui-même. Le compte est un
  cliquet (`TOXIC_FLOOR`) : 473 fiches sur 851.
- **Remplir la colonne « antidote » oblige à écrire la section.** Nommer
  un antidote, c'est affirmer qu'il existe une dose à partir de laquelle
  il faut le donner ; la fiche doit alors dire laquelle et à quoi on la
  reconnaît. Le test le refuse autrement, et c'est la règle qui décide
  quand la section est due sans avoir à juger fiche par fiche.
- **Ajouter une fiche** : une ligne dans `STARTER_DRUGS` **et** une
  entrée `StarterDetail` du même nom. Le test refuse l'une sans l'autre.
  Et une ligne de posologie, sans quoi
  `the_posology_coverage_only_improves` tombe — c'est ainsi que la fiche
  Wegovy a été rattrapée.
- **Relire une fiche contre elle-même** : les erreurs qui restent ne
  sont pas des champs vides, ce sont des champs qui se contredisent. La
  section toxicité de Cardensiel décrivait l'amiodarone, sur une fiche
  dont le champ `half_life` annonçait dix à douze heures deux lignes
  plus haut. Confronter chaque section toxicité citant une demi-vie au
  champ `half_life` de sa propre fiche l'a trouvée ; confronter les
  antidotes et les couples `elimination`/`renal` n'a rien trouvé, ce qui
  est aussi un résultat. **Ces confrontations restent des techniques de
  relecture et non des tests** : « demi-vie » voisine trop souvent de
  « deux prises par jour » pour qu'une règle automatique ne crie pas au
  loup, et un test qui crie au loup finit désactivé.

## Les facettes : le référentiel trié par ce qu'il contient

- **Où** : `src/facets.rs` — `HALF_LIVES` et `NO_HALF_LIFE` (la demi-vie
  plasmatique en heures, ou la raison pour laquelle il n'y en a pas),
  `BEYOND` (ce qui dure au-delà du plasma) et `IMPACTS` (organe, sens,
  degré, et la clause qui le justifie) — 2 073 lignes sur douze axes,
  couvrant 733 des 851 fiches.
- **La question à laquelle ça répond** : toutes les autres vues partent
  du nom — on cherche « Cordarone » et on lit sa fiche. Celle-ci part de
  la propriété : quelle est la plus longue demi-vie, qu'est-ce qui pèse
  sur la thyroïde. Un paragraphe ne répond pas à ça.
- **Pourquoi ce ne sont pas des mots comptés** : compter les occurrences
  de « thyroïde » dans les monographies met la lévothyroxine et
  l'amiodarone dans le même classement, l'une remplaçant la glande et
  l'autre la détruisant, et ne dit rien du degré. D'où un `Effect`
  (`Traite` / `Altere`) et un `Grade` (`Mineur` / `Notable` / `Majeur`)
  écrits à la main, jamais dérivés du texte.
- **Une facette est adossée à ce que la fiche écrit.** Si la monographie
  ne chiffre pas la demi-vie — « courte », « de l'ordre de quelques
  heures », ou seulement une demi-vie osseuse —, la facette dit
  `NonChiffree` et n'invente pas un nombre que personne ne pourrait
  relire. 193 fiches sur 851 sont dans ce cas — 139 parce que la notion
  n'a pas de sens (produit non absorbé, ion, vaccin), 54 parce que la
  monographie reste qualitative — et elles se corrigent en corrigeant la
  fiche, pas la facette.
- **Ce que l'extraction automatique a fait de faux** : lue au premier
  nombre venu, la prose donne un classement faux en tête. « Demi-vie
  d'environ 5 jours, l'effet persistant plusieurs semaines » se lit
  quatre-vingt-quatre jours ; la demi-vie osseuse d'un bisphosphonate se
  lit dix ans ; « 5,8 jours » se lit « 5 à 8 ». Les tables ont donc été
  **amorcées** par un analyseur puis relues, et non produites par lui.
- **Le champ dit si c'est un impact, et le mot ne le dit pas.** La même
  phrase change de sens selon l'endroit où la fiche l'écrit :
  « insuffisance rénale » dans `contraindications` décrit un rein *déjà*
  malade dont dépend la dose — le rein commande le médicament —, et dans
  `adverse` un rein que le médicament abîme. Seul le second est un
  impact. C'est la règle qui a fait tomber presque tout le « surveiller
  la créatininémie » du référentiel : un médicament éliminé par le rein
  n'est pas un médicament néphrotoxique. Même chose pour la thyroïde,
  dont l'axe reste à dix-neuf fiches parce que « prudence en cas
  d'hyperthyroïdie » n'est pas une atteinte thyroïdienne.
- **Cette règle n'existait pas aux premières passes, et 212 lignes en
  portaient la trace** : 106 réécrites depuis le champ qui décrivait
  vraiment un effet, 35 retirées, 2 qui disaient l'inverse (Spasfon et
  Duspatalin précisent qu'ils n'ont *pas* de contre-indication au
  glaucome). `an_impact_describes_what_the_drug_does_not_who_may_not_take_it`
  la tient désormais, sous la forme la plus simple qui soit testable :
  une clause ne *commence* pas par un mot de terrain. Une
  contre-indication citée en fin de phrase reste utile
  (« bronchoconstriction, l'asthme restant une contre-indication ») ; en
  tête, elle signe une ligne écrite depuis le mauvais champ. La règle
  couvre aussi le *geste* de surveillance, plus répandu encore :
  cinquante-huit clauses commençaient par « transaminases avant
  l'instauration puis périodiquement », qui dit ce qu'on fait et jamais
  ce que le médicament fait — quarante-sept redessinées depuis les
  effets indésirables, onze retirées faute d'y trouver un effet.
- **`BEYOND` est le troisième axe, et le plus facile à oublier** : ce qui
  dure au-delà de la demi-vie plasmatique. Il comptait trente-sept
  entrées quand cinquante-quatre fiches disaient en toutes lettres que
  leur effet survit à leur demi-vie — inhibition plaquettaire
  irréversible, métabolite intracellulaire, produit fixé dans la
  kératine, métabolites actifs du diazépam. La question derrière est
  celle du comptoir : *quand puis-je opérer, relayer, arrêter*, et la
  demi-vie plasmatique seule y répond faux. Le repérage est le même que
  pour les organes (« irréversible », « persiste », « demi-vie
  tissulaire ») et le tri aussi : une phrase de *mécanisme* qui dit
  seulement « inhibiteur irréversible » ne suffit pas, il faut que la
  fiche en tire une conséquence de durée.
- **L'absence se vérifie comme la présence.** 158 fiches ne portent aucun
  impact, et il fallait savoir si c'était vrai ou si c'était un trou : on
  repasse le repérage sur elles seules. 38 déclenchent encore un mot-clé,
  et l'examen des 38 phrases donne un seul impact manquant (la kératite
  ponctuée de Virgan, écrite dans ses effets indésirables et masquée par
  une phrase de sa section toxicité qui, elle, parlait d'échec de
  traitement). Les 157 autres sont des locaux, des vaccins, des insulines
  et des antiseptiques dont les fiches ne décrivent réellement aucune
  lésion d'organe. Refaire cette passe est le moyen de savoir si un axe
  est fini.
- **Le grade ne veut pas dire la même chose des deux côtés.** Sur
  « altère » il dit la gravité ; sur « traite » il dit la **centralité** :
  majeur quand l'organe est la raison d'être du produit, mineur quand il
  ne fait que le protéger de loin. C'est ce qui empêche soixante
  antihypertenseurs de noyer les six médicaments de l'insuffisance
  cardiaque sur l'axe du cœur.
- **Le « traite » a sa propre table de mots** (`TREAT_WORDS`, dans
  `every_treatment_is_backed_by_its_own_indication`) parce que les deux
  versants ne parlent pas la même langue : la lévothyroxine ne dit jamais
  « dysthyroïdie », elle dit « hypothyroïdie » comme la maladie qu'elle
  corrige, et l'inhalateur dit « asthme » et non « bronchospasme ».
  Ajouter un axe demande donc **deux** entrées de vocabulaire, une par
  versant, sans quoi le versant oublié n'est relu par rien.
- **La négation est le piège qu'aucun mot-clé ne voit**, et elle s'est
  présentée cinq fois : le fondaparinux dont la fiche dit qu'il ne
  provoque *pas* de thrombopénie induite par l'héparine, l'étanercept qui
  n'a *pas* de place dans la maladie de Crohn (trois fiches), Vastarel
  dont les indications ORL ont été *supprimées*, Spasfon et Duspatalin
  qui précisent n'avoir *pas* de contre-indication au glaucome. C'est la
  raison pour laquelle le repérage propose et ne décide pas.
- **Une fiche peut traiter un organe *et* l'abîmer** — 178 le font, et
  ce sont les lignes les plus utiles de la table : le
  bisphosphonate traite l'os et nécrose la mâchoire, l'anti-VEGF sauve la
  rétine et la décolle parfois en y entrant. La clé d'unicité porte donc
  le sens (`nom`, `organe`, `Altere ?`) et non la seule paire
  `nom`/`organe` : une clé plus étroite aurait forcé à choisir entre deux
  vérités. Ce que le test interdit toujours, c'est la même fiche deux
  fois dans le même sens sur le même organe.
- **Tests** : chaque facette nomme une fiche réelle ; **l'axe demi-vie est
  complet** — un classement « la plus longue demi-vie » calculé sur une
  partie du référentiel répond faux sans le dire ; les bornes tiennent
  ensemble ; chaque impact dit pourquoi en une clause qu'une colonne peut
  afficher ; le classement met bien en tête une demi-vie *plasmatique* ;
  et **`every_impact_is_backed_by_its_own_card` relit la monographie de
  chaque impact et exige que le vocabulaire de l'organe y figure**. C'est
  ce test qui sépare cette table d'une liste écrite de mémoire, et il a
  effectivement trouvé des lignes écrites de mémoire — deux sur la
  thyroïde, quatre sur l'œil où « trouble visuel » désignait le signe
  d'alerte d'une thrombose sous contraceptif et non une lésion oculaire.
  Il ne vaut que pour `Altere` : un `Traite` est adossé à l'indication,
  qui parle une autre langue — la lévothyroxine ne dit pas
  « dysthyroïdie ».
- **La couverture d'un axe se voit, et son effectif est sur la porte** :
  l'explorateur écrit le nombre de fiches sur chaque onglet et rappelle
  sous le tableau qu'une fiche absente est une fiche qui n'en parle pas —
  sans quoi un axe court se lit « aucun médicament ne touche cet organe ».
- **Ajouter un axe** : une variante de `Organ`, son libellé, des lignes
  dans `IMPACTS`, et **le vocabulaire de l'organe dans les deux tables** —
  `HARM_WORDS` pour le versant « altère », `TREAT_WORDS` pour le versant
  « traite ». Sans la première, le test d'étayage refuse tout l'axe ;
  sans la seconde, il refuse tous ses « traite ». L'énumération peut
  rester plus large que ce qui est peuplé : l'explorateur ne propose que
  les axes renseignés, et le plancher exige que les douze le soient.

## Les classes thérapeutiques

- **Où** : `src/classes.rs` (pur, testé). Seize familles, 383 classes
  canoniques, et pour chacune les libellés qu'on rencontre réellement
  dans le champ `class` des fiches.
- **Pourquoi un référentiel plutôt qu'une réécriture** : le champ d'une
  fiche est du texte libre et il a dérivé — 495 libellés pour 851
  fiches, dont 331 sur une seule. Réécrire les 851 fiches écraserait ce
  que l'équipe a écrit ; un référentiel les *lit*. Une classe qu'il ne
  connaît pas reste lisible et se range sous « hors référentiel », où
  elle se voit.
- **Ce que la dérive coûtait** : `anti-TNF` et `anti-TNF alpha` étaient
  deux classes. La pastille de Humira annonçait sept voisins au lieu de
  dix, et Remicade n'était nulle part — sans que rien n'ait l'air cassé.
  C'est la question du comptoir un jour de rupture, et une réponse
  incomplète y est pire qu'une absence de réponse.
- **Ajouter une classe** : une ligne dans la famille qui convient, avec
  son nom canonique et, s'il y a lieu, les graphies qu'elle replie. Un
  alias n'est jamais le nom canonique d'une autre classe — un test le
  refuse, sans quoi une fiche tomberait dans deux classes selon l'ordre
  de la table.
- **Ajouter une famille** : une ligne dans `FAMILIES`, et au moins une
  classe dessous — une famille vide est une ligne cliquable qui n'ouvre
  rien, et le test la refuse.
- **Les tests** : chaque classe écrite sur une fiche livrée est dans le
  référentiel ; un libellé ne désigne qu'une classe ; les trois dérives
  mesurées se replient ; et le référentiel porte **au moins 112 classes
  de moins** que la base n'écrit de libellés — sans quoi il n'aurait
  rien replié.

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
  antipsychotiques, immunosuppresseurs, anticancéreux oraux) sont
  **délibérément vides**, et la liste `NO_POSOLOGY` du test les dispense.
- **Une exemption est une affirmation sur la molécule, et elle se
  vérifie.** Quatre d'entre elles étaient fausses : « titrée par le
  spécialiste » ne décrit ni le létrozole à 2,5 mg, ni le fingolimod à
  0,5 mg, ni le tofacitinib à 5 mg deux fois par jour — une dose, la
  même pour tout le monde, pendant des années. Les anti-aromatases, les
  immunomodulateurs de la SEP, le modulateur S1P et les inhibiteurs JAK
  ont donc quitté la liste, avec leurs posologies écrites. Avant
  d'exempter une classe, se demander si la phrase est vraie.

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

- **Où** : deux tables, et **dans leur propre fichier** —
  `<base>_stups.db` à côté de la base, même SQLCipher, même clé.
  `stupefiants` (les produits suivis, leur unité, leur seuil, leur
  famille, leur régime et leur durée maximale de prescription) et
  `stup_moves` (le registre). Le calcul est dans `src/ordonnancier.rs`
  (pur, testé). Le fichier séparé n'est pas une question de taille : dix
  ans de registre pèsent quelques centaines de kilo-octets. C'est que la
  base est un outil de travail — on la réinitialise, on la ressème, on
  la compacte, on l'essaie en copie — alors que le registre est une
  pièce comptable à conserver dix ans, qu'un contrôle demande seule.
- **Le catalogue est livré, le registre ne l'est pas.**
  `ordonnancier::CATALOGUE` porte 106 présentations du marché français
  en 12 familles, avec le dosage, l'unité de comptage, la durée maximale
  de prescription et la règle de la famille. On y **choisit** : une base
  livrée avec 106 produits suivis serait 106 soldes à zéro et 106
  « jamais compté » sur la liste de contrôle. Un produit qui n'y est pas
  s'inscrit quand même, sous le libellé qu'on tape.
  - **Ajouter une présentation** : une ligne dans la famille qui porte
    déjà sa règle. Le libellé porte le dosage (un test le refuse sans),
    l'unité dit ce qu'on compte, et le compte total est un cliquet.
  - **Ajouter une famille** : `name`, `status` (`STUPEFIANT` ou
    `ASSIMILE`), `max_days` (7, 14 ou 28 — un test refuse toute autre
    valeur, parce que c'est sur ce nombre qu'on refuse une ordonnance),
    et `note`, qui est la règle du comptoir en une ou deux phrases. Une
    famille `ASSIMILE` doit dire dans sa note ce que son régime demande
    vraiment : la buprénorphine haut dosage relève des stupéfiants pour
    la prescription et la délivrance, **pas** pour la comptabilité, et
    la ranger sans le dire enseignerait une obligation qui n'existe pas.
  - Ce qui n'y est **pas**, volontairement : ce que seul un hôpital
    détient (kétamine, sufentanil, péthidine) et les benzodiazépines à
    ordonnance sécurisée (clonazépam, midazolam, zolpidem), qui ne
    s'inscrivent à aucun registre.
- **La règle qui décide de tout** : le registre est inaltérable. Pas
  d'`UPDATE`, pas de `DELETE`, pas de méthode qui en proposerait. Une
  erreur se contre-passe. Un test relit `db.rs` et le refuse.
- **Corriger, c'est annuler** : une ligne `ANNULATION` désigne la ligne
  fautive, porte un motif obligatoire, et défait exactement ce que
  celle-ci avait fait au stock — lu sur la ligne annulée et jamais sur
  l'annulation, dont la quantité n'est jamais regardée. C'est le cœur de
  la règle : le jour où l'on corrige est le jour où la quantité était
  fausse. Une annulation ne s'annule pas, ne s'écrit pas deux fois
  (vérifié dans la transaction), ne désigne pas la ligne d'un autre
  produit, et ne prend jamais de numéro d'ordonnancier.
- **La balance n'est pas une somme** : un inventaire *pose* le solde.
  Additionner l'écart et poser le compte le compterait deux fois, et le
  registre dériverait dès le premier comptage qui ne tombe pas juste.
  Annuler un inventaire rend au registre son `expected` — la seule
  raison pour laquelle cette colonne est écrite et non recalculée.
- **Le numéro d'ordonnancier** est séquentiel dans l'année, attribué par
  la base dans la transaction qui écrit la ligne, et jamais réattribué.
  Une délivrance annulée garde le sien : un trou dans la suite se lit,
  un numéro servi deux fois ne se lit pas.
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
  le nombre de règles ne baisse jamais (`RULES_FLOOR`).
- **Avant d'ajouter une règle, chercher si elle existe déjà.** Les
  titres sont libres, et deux règles peuvent dire la même chose sous
  deux noms : une règle « inducteur + contraception » a été écrite puis
  retirée parce que « Contraception sous inducteur » existait, plus
  complète. Deux points identiques sur la même ordonnance sont du
  bruit, et le bruit est ce qui fait cesser de lire la revue.
- **Les sections toxicité des fiches sont un gisement de règles** : ce
  qui y est écrit comme « association contre-indiquée » ne sert au
  comptoir que si `revue.rs` la voit sur l'ordonnance. C'est de là que
  viennent le carbapénème sur valproate, le miconazole sur AVK et sur
  sulfamide, le gemfibrozil sur répaglinide, l'aminoside avec un
  diurétique de l'anse. Le même gisement alimente `surveillance.rs`.

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

## Le rendez-vous de prévention et l'export

- **Où** : `[prevention] subjects` dans config.toml, et la fenêtre
  d'export dans `src/app.rs` (`ExportBox`).
- **Pourquoi il n'a pas de thème** : on en couvre plusieurs dans la même
  séance. En stamper un sur l'acte nommerait le moindre en cachant les
  autres — `InterviewKind::has_theme` l'exclut, comme les TROD mais pour
  la raison inverse : le TROD n'a pas de sujet, celui-ci en a trop pour
  un seul champ.
- **Ce qu'il couvre se choisit à l'impression.** La fenêtre d'export
  propose la liste de `[prevention] subjects`, décochée : sur ce
  rendez-vous-là, choisir *est* la question. Pour tous les autres actes,
  elle propose la liste du thème, cochée — c'est ce que la feuille
  portait déjà.
- **Et ce qu'on ajoute à la main.** Une zone de texte libre, une ligne
  par point. Aucune liste livrée ne couvre ce qu'un entretien a
  réellement abordé ; c'est la moitié qui compte le plus.
- **Ajouter un sujet** : une ligne dans `[prevention] subjects`. La
  liste livrée est celle de « Mon bilan prévention » ; elle se réécrit
  entièrement, et la vider n'est pas une erreur — la fiche se contente
  alors de ce qu'on tape.
- **Le courrier** porte `{{POINTS}}` : sans point coché il garde son
  encadré vide, avec des points il imprime la liste et garde un encadré
  plus court pour la réponse du médecin.

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

## Deux modules qui ne livrent **aucun** contenu, exprès

`src/vigilance.rs` et `src/codebar.rs` sont des règles et des lectures,
pas des catalogues, et cela mérite d'être écrit ici pour que personne ne
vienne un jour « compléter » ce qui est vide à dessein.

### `src/vigilance.rs` — les questions que le registre pose

- **Où** : le module entier ; il ne connaît ni la base ni egui, et tout
  ce qu'il sait vient des lignes du registre de l'officine.
- **Ce qu'il ne saura jamais** : une ligne de délivrance porte un jour,
  une quantité en unités de comptage, un dossier et un prescripteur en
  texte libre. **Ni dose quotidienne, ni durée prescrite, ni à quelle
  ordonnance elle se rattache.** Donc « ce traitement aurait dû durer
  jusqu'au » ne se calcule pas, et `quantité / max_days` est faux dans
  le sens dangereux — le plafond est légal et non posologique, et
  l'utiliser ainsi ferait de chaque délivrance légitime un signalement.
- **Ce qu'il fait** : le dossier contre lui-même, sous **deux** silences
  qu'il faut tous deux franchir — le plafond de la famille, qui ne sert
  qu'à se taire, et la médiane des intervalles antérieurs de ce dossier.
  Sous trois délivrances antérieures, il ne dit rien.
- **Le cliquet n'est pas un compte** mais une forme :
  `every_signal_asks_a_question_rather_than_stating_one` exige que chaque
  clé finisse par un point d'interrogation, et
  `a_finding_carries_the_lines_that_produced_it_and_never_fewer_than_two`
  qu'une question cite les lignes qui la posent. Ajouter un signal, c'est
  ajouter une variante, sa question dans `strings.fr.toml`, et les lignes
  qui l'étayent — les deux tests refusent le reste.
- **Ce qu'on n'y mettra pas**, et le module le dit : un score de mésusage
  attaché à une personne, une dose quotidienne déduite, et l'équivalent
  morphine — le plus séduisant et le pire, puisque le registre ne connaît
  aucune dose quotidienne.

### `src/codebar.rs` — ce qu'une douchette a tapé

- **Où** : le module lit une chaîne de caractères. Une douchette USB est
  un clavier ; il n'y a ici ni pilote, ni image, ni décodage optique.
- **Aucune table CIP n'est livrée, et c'est la garantie** : l'application
  n'a rien avec quoi deviner à quel produit correspond un code. Un code
  inconnu attend qu'un humain désigne le produit, et le lien qui en
  résulte (`stup_codes`, dans le fichier du registre) est le contenu de
  l'officine, comme ses pièces numérisées et son registre.
- **La clé de contrôle décide, jamais la longueur** — c'est le même
  principe que le NIR dans `src/vitale.rs`. Treize chiffres dont la clé
  ne tombe pas sont treize chiffres, pas une coquille à rattraper.
- **Ajouter un identifiant d'application (AI)** : une branche dans
  `read_element_string` et son test. Un AI inconnu **arrête** la lecture
  et le dit (`read_to_end: false`) ; comprendre à moitié une chaîne est
  pire que s'arrêter.
- Le jour où la base publique des médicaments arrivera, elle viendra
  **sous** ces fiches et ne changera pas cette règle : un GTIN ne porte
  aucun nom, et rapprocher un scan d'un libellé resterait facile et faux.
