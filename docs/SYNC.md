# La synchronisation pair-à-pair — ce qui est construit, ce qui reste

`ROADMAP.md` portait depuis le début une ligne restée seule :

> Sur l'onglet « officine » : on devrait pouvoir régler une url, puis
> travailler l'UI/API pour avoir de la synchronisation (données
> chiffrées).

C'est le seul point de la feuille de route qui ouvre un chemin réseau
dans une application dont toute la posture tient en une phrase : tout
est local, la base est chiffrée, et la seule requête sortante est la
vérification de version, sur un bouton. Ce document dit ce qui a été
construit — le crate `sync/`, `bpm-sync`, entièrement testé et branché
sur rien —, ce qu'il garantit, ce qu'il ne garantit **pas**, et ce qu'il
reste à faire pour qu'une officine s'en serve.

Il s'adresse à qui reprend le sujet, y compris à moi-même dans six mois.

---

## 1. La règle d'entrée : optionnel veut dire absent

`bpm-sync` est un crate séparé, dépendance **optionnelle** de
l'application, derrière `--features sync`, éteint par défaut. Ce n'est
pas une commodité de compilation : un binaire construit sans ce drapeau
ne contient ni socket d'écoute, ni machine à états, ni rien à attaquer.
Une officine qui ne veut pas de réseau n'obtient pas « une option
décochée », elle obtient un logiciel où le code n'existe pas.

Le crate ne dépend de rien de l'application : il ne lit aucune base, ne
dessine aucun pixel, et ne sait pas ce qu'est un patient. Il reçoit des
octets et rend des octets. C'est ce qui permet de faire dialoguer deux
postes entiers — appairage, échange de clé, divergence comprise — dans
un seul `#[test]`, sans ouvrir de socket : la seule façon qu'un
protocole soit vraiment regardé.

Il est en revanche **membre de l'atelier**, donc l'intégration continue
le compile et le teste à chaque passe. Optionnel ne veut pas dire non
surveillé.

---

## 2. Le modèle de menace

### Ce qu'on protège

Le journal clinique d'une officine : ce qui a été délivré, à qui, par
qui, et les corrections. Contre trois choses.

* **Qui transporte.** Le commutateur de l'officine, le routeur, la
  liaison entre deux sites, et — le jour où il en existe un — un relais.
  Ils portent des octets qu'ils ne peuvent pas lire, et ils ne peuvent
  pas en ajouter.
* **Qui se met au milieu.** Une poignée de main chiffrée prouve qu'on
  parle à quelqu'un qui détient *une* clé ; elle ne dit pas que c'est
  l'autre poste de l'officine. Seuls deux humains qui se lisent un code
  peuvent le dire, et c'est exactement ce que l'appairage demande.
* **Qui rejoue.** Une conversation enregistrée ne se rejoue pas : la
  signature d'identité porte sur l'empreinte de *cette* poignée de main.

### Ce qu'on ne protège pas, et c'est écrit

* **Pas de confidentialité persistante (forward secrecy) au repos.** Le
  trousseau est de longue durée, parce qu'un poste qui rejoint
  l'officine en mars doit pouvoir lire ce qui a été écrit en janvier :
  un pharmacien qui ouvre un dossier veut le dossier, pas la partie du
  dossier postérieure à son portable. Qui obtient le trousseau **et**
  les enregistrements scellés les lit tous. La défense est celle que
  l'application a déjà : tout cela vit dans SQLCipher, sur un disque
  qui appartient à quelqu'un, derrière le mot de passe de l'officine.
* **Pas de défense contre un poste dont le disque et le mot de passe
  sont entre d'autres mains.** C'est un transport et une histoire, pas
  une deuxième serrure sur la porte.
* **Une image de trafic reste visible.** L'en-tête d'un enregistrement
  est en clair — il le faut, pour qu'un pair le place dans le graphe
  sans pouvoir le lire. Qui détient le journal sans le trousseau
  apprend donc que tel poste a écrit onze lignes de registre un mardi.
  Il n'en apprend pas un mot.
* **Révoquer n'est pas reprendre, et révoquer se fait partout.** Sortir
  un poste de la liste d'une machine l'empêche de se connecter *à cette
  machine-là*. Tout ce qu'il a écrit reste attribuable pour toujours —
  c'est signé, et `Fact` porte l'auteur. Mais un poste que les autres
  ont désavoué peut encore donner un enregistrement neuf à un poste qui
  ne l'a *pas* désavoué, et de là il arrive partout : un journal accepte
  ce qu'on lui donne, parce que refuser un auteur ferait diverger pour
  toujours deux journaux sur une liste qu'aucun des deux ne voit.
  Révoquer veut donc dire révoquer sur chaque poste. Et cela ne reprend
  pas ce que la machine détient déjà : pour le passé, il faut re-clé
  l'officine, exactement comme la seule réponse à une base volée est
  d'en changer le mot de passe. **La révocation est affaire d'avenir ;
  la re-clé est affaire de passé.**

---

## 3. Les quatre garanties, et la règle qui va avec chacune

### « Un pair ne voit jamais de clair »

Les enregistrements sont scellés avant d'être nommés
(XChaCha20-Poly1305, clé dérivée du trousseau par flux). Le nonce est
tiré au hasard à chaque fois : vingt-quatre octets, assez larges pour
qu'aucun poste n'ait à coordonner un compteur avec les autres — ce qui
est la seule disposition qui survive à trois machines qui écrivent
pendant que l'une d'elles est débranchée.

*Ce qui le tient :* `a_relay_cannot_read_what_it_carries` ramasse **tous
les octets** qui ont traversé une conversation complète et y cherche les
phrases écrites, le trousseau, et les graines des deux postes. Mesuré
sur les octets, pas argumenté depuis le dessin.

### « Un enregistrement ne se réécrit pas »

Son nom est le hachage de son contenu, et les suivants le nomment comme
parent : changer un octet le renomme et casse tout ce qui a été écrit
après. Une correction est un **autre** enregistrement qui dit lequel il
corrige ; le fautif reste écrit, et sur le papier il reste barré. C'est
la règle que l'ordonnancier de stupéfiants suit déjà (R. 5132-36),
généralisée : un journal qu'on peut éditer ne prouve rien, et un qu'on
peut éditer *à distance* prouve moins que rien.

*Ce qui le tient :* `a_record_is_named_by_its_content`,
`a_correction_never_removes_what_it_corrects`, et
`a_journal_can_only_ever_be_added_to`, qui lit le texte du module et
refuse le verbe qui enlèverait un enregistrement — comme
`the_register_can_only_ever_be_written_to` le fait dans `db.rs`.

### « L'horloge de personne ne décide »

L'ordre vient d'un compteur de Lamport et du nom de l'auteur. Un poste
dont la date est fausse écrit une ligne au mauvais endroit d'un
*calendrier* ; il ne peut pas réordonner le journal, ni faire passer une
ligne ancienne sous une récente. L'ordre est **total** (rang, puis
auteur, puis nom), sans quoi deux postes détenant les mêmes
enregistrements les afficheraient dans deux ordres — et deux écrans qui
montrent un registre différemment, personne ne peut agir dessus.

### « Un conflit se montre, il ne se tranche pas »

Deux postes qui corrigent le même enregistrement sans s'être vus : les
deux corrections restent, **et chacune nomme l'autre**, dans la donnée
et non à côté. C'est le refus que les avertissements de
compare-and-set font déjà au comptoir, pour la même raison : choisir en
silence entre deux affirmations cliniques, c'est se tromper une fois sur
deux avec l'assurance d'une réponse.

Et le corollaire, qui est la règle que la maison écrit partout :
**le silence ne vaut pas autorisation.** Un enregistrement que le
trousseau en main n'ouvre pas est **nommé** (`Reading::unopened`), pas
sauté : une liste discrètement amputée d'une ligne se lit exactement
comme une liste complète.

---

## 4. La forme du crate

| module | ce qu'il répond |
| --- | --- |
| `enc.rs` | l'encodage canonique — écrit à la main, parce que le *nom* d'un enregistrement est un hachage de ces octets-là |
| `keys.rs` | `Device` (cette machine, qui signe) et `Trousseau` (l'officine, qui ouvre) ; les empreintes lues à voix haute |
| `seal.rs` | un enregistrement : scellé, nommé par son contenu, signé |
| `journal.rs` | le graphe, la fusion, la lecture d'un flux, et ce qu'un pair doit recevoir |
| `wire.rs` | six trames, et rien d'autre |
| `session.rs` | le protocole comme machine à états qui ne touche à rien |
| `meter.rs` | les compteurs — un volet, pas une balise |
| `link.rs` | le seul endroit où des octets bougent : la porte, le lien, et de quoi frapper |

Tout sauf `link.rs` est pur. L'aléa est **passé** (`Entropy`), comme la
date est passée partout ailleurs dans cette application. La seule
exception est la clé éphémère de la poignée de main, que `snow` tire du
système : une poignée de main dont un test pourrait fournir l'aléa est
une poignée de main à laquelle il ne faut pas se fier, et faire semblant
du contraire par symétrie serait le mauvais échange.

### L'API, en entier

```rust
let device    = Device::generate(&mut OsEntropy);   // ce poste
let trousseau = Trousseau::generate(&mut OsEntropy); // cette officine
let mut journal = Journal::new();

journal.write(&device, &trousseau, Stream::Registre, octets, None, &mut e)?;
let lecture = journal.read(&trousseau, Stream::Registre);

let mut session = Session::new(&device, Some(&trousseau), Intent::Sync, true, &connus)?;
loop {
    match session.step(&mut meter)? {
        Step::Send(o)    => link.send(&o)?,
        Step::Await      => { let o = link.recv()?; session.deliver(&o, &mut journal, &mut meter)?; }
        Step::Confirm(c) => { /* montrer c.groups(), puis */ session.accept(&journal, &mut meter)?; }
        Step::Done       => break,
    }
}
```

C'est tout. Six types, quatre verbes.

---

## 5. Le protocole

**La poignée de main n'est pas inventée ici.** C'est Noise XX
(`Noise_XX_25519_ChaChaPoly_BLAKE2s`), via `snow`. Un protocole écrit
pour une seule application est un protocole que personne n'a attaqué, et
celui-ci porte des données de santé.

Ce que XX donne : un canal chiffré où chaque bout a prouvé détenir *une*
clé statique. Ce qu'il ne peut pas donner : une raison de croire que
cette clé est celle de l'autre poste de l'officine plutôt que de ce qui
s'est mis au milieu. Deux choses comblent l'écart, et aucune n'est
facultative.

1. **L'identité est liée au canal.** Juste après, chaque poste envoie un
   `Hello` portant son `DeviceId` et une signature Ed25519 *sur
   l'empreinte de la poignée de main*. Un enregistrement d'hier ne se
   rejoue sur rien.
2. **Un inconnu est admis par un humain, une fois.** Les deux opérateurs
   voient les mêmes cinq groupes de quatre caractères, dérivés de cette
   même empreinte, et se les lisent. Égaux : personne au milieu.
   Différents : quelqu'un au milieu, et l'appairage est refusé. On ne
   peut pas sauter l'étape — un poste non déjà connu n'est pas écouté
   tant que `accept()` n'a pas été appelé, et la seule chose qui appelle
   `accept()` est une personne.

**L'échange**, ensuite : les têtes, puis des tours. À chaque tour les
deux postes envoient un `Want` — *ce qu'il me manque et ce que j'ai* —,
répondent à celui de l'autre par les enregistrements qui lui manquent,
et ferment par `EndRound`. Un tour où les deux demandes étaient vides
est la fin : les deux journaux sont d'accord, et les deux le savent.

Deux détails qui ne sont pas des détails :

* Le `Want` porte **les deux moitiés**, si bien qu'une première
  synchronisation tient en un tour au lieu d'un tour par génération du
  graphe.
* `EndRound` porte un drapeau `more`. C'est la différence entre « tu as
  tout ce que j'ai » et « je me suis arrêté au plafond ». Sans lui, le
  demandeur ne peut pas distinguer un enregistrement que le pair n'a pas
  d'un qu'il n'a pas encore envoyé : il doit alors soit abandonner tôt —
  en perdant des enregistrements, en silence —, soit demander pour
  toujours. On le lui dit.

---

## 6. La télémétrie, et ce que le mot a le droit de vouloir dire ici

`meter.rs` tient des nombres : combien de conversations, combien
d'enregistrements ont traversé, combien d'octets, combien de refus et de
quelle sorte. Il existe parce qu'une synchronisation qui échoue en
silence est pire qu'une qui échoue bruyamment — quelqu'un doit pouvoir
ouvrir un volet et voir que ce poste n'a pas parlé à l'autre depuis
mardi.

**C'est un volet, pas une balise.** Rien n'y envoie rien nulle part. Pas
d'adresse, pas d'identifiant d'installation, pas de « statistiques
d'usage », et il n'y en aura pas : c'est une application qui tient des
données de santé, et la lecture honnête du mot télémétrie dans ce
cadre-là est *ce que l'officine peut voir de ses propres machines*.

**Elle compte, elle ne cite jamais.** Aucun champ n'est une chaîne. Un
message porte un fragment de ce dont il parle — un nom, un chemin, une
ligne — et l'endroit où personne ne cherche une fuite est un journal
que quelqu'un a allumé pour déboguer autre chose. Un refus est donc une
`Error`, une énumération sans rien dedans, et la phrase française est
écrite par l'écran qui montre le nombre.

`telemetry_counts_and_never_quotes_and_never_leaves` lit le texte du
module et refuse le jour où l'un ou l'autre change.

---

## 7. Ce qui reste à faire

Rien de ce qui suit n'est commencé. C'est délibéré : les décisions
ci-dessous appartiennent à l'officine, pas au module.

### 7.1 Où vivent les clés

La graine du poste et le trousseau sont deux fois trente-deux octets. Ils
vont dans la base chiffrée, pas dans `config.toml` — `config.toml` est un
fichier en clair, un par PC. Concrètement : une table `sync_keys` dans
`SCHEMA` **et** un `ALTER TABLE` idempotent dans `MIGRATIONS`, plus la
liste des postes appairés (`sync_peers` : `DeviceId`, empreinte, nom
donné par l'officine, date).

Conséquences à ne pas oublier, chacune déjà nommée dans `CLAUDE.md` :
`change_password` re-clé **trois** fichiers aujourd'hui ; « Copier la
base… » en copie trois ; et une base d'une version antérieure doit
continuer de répondre à toutes les requêtes
(`a_base_from_an_older_version_still_answers_every_query`).

### 7.2 La correspondance entre les tables et les flux

`Stream` a sept valeurs connues. La question ouverte est **ce qu'un
enregistrement contient** : une ligne SQL sérialisée, ou une opération
(« ce champ de ce dossier passe à cette valeur »). La deuxième fusionne
mieux — deux postes qui modifient deux champs du même dossier ne
divergent pas — et coûte une couche de projection dans les deux sens.
C'est la décision structurante qui reste, et elle se prend flux par
flux :

| flux | forme naturelle | pourquoi |
| --- | --- | --- |
| `Registre` | la ligne, telle quelle | il est déjà en ajout seul : l'union suffit, il ne peut pas diverger. **C'est par lui qu'il faut commencer.** |
| `Caisse` | la ligne | `caisse_counts` est en `INSERT` seul aussi |
| `Dossiers` | l'opération | c'est le seul endroit où deux postes touchent la même ligne le même jour |
| `Planning`, `Agenda` | l'opération | même raison, en plus calme |
| `Officine`, `Fiches` | la ligne | déjà édités « comme un tout » dans un dialogue |

### 7.3 La projection

Deux fonctions, pures, testées dans les deux sens, par flux : `base →
enregistrements` et `enregistrements → base`. Le piège est connu et
porte un nom dans cette maison : **deux écritures d'une même chose
divergent**. La lecture qui projette et l'écriture qui range doivent se
lire l'une l'autre, ou l'une des deux mentira.

Et une mise en garde de plus, qui est la règle de la maison : **rien de
coûteux dans une frame.** `Journal::read` trie tout le flux à chaque
appel — c'est bon marché pour une projection qui tourne au moment d'une
synchronisation, et c'est soixante fois par seconde si quelqu'un
l'appelle depuis un dessin. La projection écrit dans la base ; les vues
lisent la base, comme aujourd'hui.

Et la projection vers la base doit respecter la discipline qui existe
déjà : compare-and-set sur les valeurs affichées, avis en français quand
ça bouge sous les doigts, et `resync` qui **recharge des lectures et
jamais un tampon de saisie**.

### 7.4 L'écran

Options › Officine, puisque c'est là que la feuille de route le
demandait. Ce qu'il porte, au minimum :

* l'empreinte de ce poste, en cinq groupes de quatre, à lire au
  téléphone ;
* « Appairer un poste… » des deux côtés (inviter / rejoindre), avec le
  code de cinq groupes **en gros**, et les deux boutons « c'est le même
  code » / « ce n'est pas le même » ;
* la liste des postes connus, avec le droit d'en retirer un ;
* le volet de télémétrie : dernière conversation, ce qui a traversé, les
  refus par motif ;
* et la réserve, **écrite avant le contenu et pas en pied de panneau** —
  c'est la règle
  `a_clinical_pane_writes_its_caveat_before_what_it_qualifies`.

Attention aux règles d'affichage de la maison sur cet écran-là : la
boîte de dialogue Options a déjà une barre de défilement solide, les
champs se mesurent en caractères et pas en pixels, et un code lu à voix
haute ne s'élide jamais — il se réduit ou il ne se dessine pas.

### 7.5 Le transport

Les deux moitiés existent et ne dépendent que de `std::net` :
`link::Door` (ouvrir, dire où l'on est, accepter **un** poste) et
`link::dial` (frapper). Pour deux postes du même comptoir, c'est tout ce
qu'il faut, et le chiffrement est celui de bout en bout — pas de TLS,
pas de certificat, pas de seconde chose à faire correctement.

Deux choses y sont déjà réglées parce qu'elles se règlent mal plus
haut : une porte à laquelle personne ne vient **rend la main** (`std`
n'a pas d'`accept` avec échéance, alors elle scrute — c'est le seul
endroit du crate qui le fait, et c'est écrit là plutôt que dans
l'appelant), et frapper là où il n'y a personne répond tout de suite au
lieu des quatre-vingt-dix secondes du système.

Ce qui reste, côté application : un fil dédié, une adresse et un port
dans `config.toml` — c'est propre au poste, donc c'est bien là —, et la
règle que `link.rs` énonce et ne s'autorise pas à assouplir : *rien ici
ne décide de rien*. Pas de reconnexion automatique, pas d'horaire. Une
officine qui veut synchroniser à la fermeture appuie sur un bouton, ou
pose une tâche.

Le relais entre deux sites viendra après, s'il vient : il n'a rien à
apprendre de neuf, puisqu'il ne porte que des octets opaques.

### 7.6 Ce qui ne sera pas synchronisé

Les pièces scannées. `MAX_PAYLOAD` vaut quarante-huit mille octets et ce
chiffre n'est pas arbitraire : un message Noise en porte 65 535, et il
n'y a **aucun tampon de réassemblage** dans ce crate, volontairement —
un protocole qui découpe et recolle est un protocole avec une table de
choses à moitié arrivées, et cette table est exactement l'endroit où un
pair fait retenir de la mémoire qu'on ne lui a pas proposée. Une
ordonnance scannée n'est donc pas un enregistrement : c'est un fichier,
il vit dans `<base>_scans.db`, et il aura son propre arrangement le jour
où quelqu'un le voudra.

---

## 8. Les décisions qui appartiennent à l'officine

1. **Ligne ou opération, flux par flux** (§ 7.2). Commencer par le
   registre ne demande de trancher rien du tout, et donne la moitié de
   la valeur.
2. **Qui écoute.** Un poste « serveur » désigné, ou n'importe lequel ?
3. **Quand.** À la fermeture, sur bouton, ou en continu ?
4. **Entre sites.** Deux officines du même groupement, c'est un relais et
   une conversation sur l'appairage à distance — le code de cinq groupes
   se lit très bien au téléphone, et c'est précisément le cas pour
   lequel il est fait.
