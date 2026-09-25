# Synchronisation pair-à-pair — état de la construction

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

> **Depuis 0.273.0, un premier usage est branché : le réseau
> d'officines** (`src/network.rs`). Il ne transporte qu'un flux,
> `Stream::Reseau` — les ruptures et ce qu'on a donné à la place, sans
> aucun patient —, sous un trousseau **propre au réseau** qui n'ouvre rien
> d'autre. Les décisions du § 8 ont été prises pour lui par l'officine :
> entre officines d'un groupement ; sur un bouton et à la fermeture ;
> personne n'écoute en permanence (une porte s'ouvre le temps d'une
> invitation) — **décision revue en 0.321.0, sur demande de l'officine :
> une porte tenue ouverte aux seules officines appairées, voir plus
> bas** ; et, en plus des adresses, un **dossier d'échange** où
> chaque officine dépose ses enregistrements scellés. La fonction `sync`
> est donc allumée par défaut ; `--no-default-features` rend toujours un
> binaire sans aucun code réseau. Les autres flux (dossiers, registre,
> caisse…) restent non branchés, et le § 7 vaut toujours pour eux.

---

## 1. Règle d'entrée : optionnel signifie absent

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

### Périmètre protégé

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

### Hors périmètre

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

### Re-clé : coût

Une officine ne jette pas quatre ans de registre parce qu'un poste a été
volé. Re-sceller tous les enregistrements sous la nouvelle clé les
**renommerait** tous — le nom est un hachage du chiffré — et un journal
renommé est un autre journal : ce n'est donc pas la voie.

La voie est celle-ci, et elle est déjà en place : la nouvelle clé scelle
ce qui s'écrit à partir de maintenant, les anciennes restent dans la
base, et `Journal::read_with` les essaie dans l'ordre, la plus récente
d'abord. Rien ne change sur le fil, aucun enregistrement ne bouge, et il
n'y a **pas de marqueur d'époque** dans l'en-tête à tenir à jour : le
sceau répond tout seul, puisqu'un AEAD sous la mauvaise clé ne s'ouvre
pas. Un enregistrement qu'aucune clé du trousseau n'ouvre reste
**nommé**, jamais sauté.

Ce que la re-clé ne rend pas : ce que le poste volé détient déjà. Elle
coupe la suite, pas le passé — la même chose exactement que changer le
mot de passe d'une base dont quelqu'un a pris une copie.

---

## 3. Les quatre garanties et leurs règles

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
let device    = Device::generate(&mut OsEntropy);    // ce poste
let trousseau = Trousseau::generate(&mut OsEntropy); // cette officine
let mut journal = Journal::new();

// Écrire un fait, lire un flux. Ce qui entre, ce sont des octets : ce
// crate ne sait pas ce qu'est une ligne de registre, et ne doit pas.
journal.write(&device, &trousseau, Stream::Registre, octets, None, &mut e)?;
let lecture = journal.read(&trousseau, Stream::Registre);

// Parler à un poste. `drive` porte la boucle, parce qu'il n'y a qu'une
// façon correcte de l'écrire : `confirm` reçoit le code, l'écran le
// montre, et répondre « non » arrête la conversation au lieu de la
// continuer en silence.
let mut session = Session::new(&device, Some(&trousseau), Intent::Sync, true, &connus)?;
let mut link = link::dial("192.168.1.14:7742", patience)?;
drive(&mut session, &mut link, &mut journal, &mut meter, &mut |code| {
    montrer(code.groups()) == Reponse::MemeCode
})?;
```

Et de l'autre côté : `let door = link::Door::open("0.0.0.0:7742")?;`
puis `door.accept(patience)?`, avec `Intent::Invite` la première fois.

C'est tout. `Session::step` / `deliver` restent publics pour qui veut
conduire la conversation autrement — sur autre chose qu'un socket, ou
au rythme d'une interface — mais personne n'est obligé de les toucher.

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
   `accept()` est une personne — ou un ticket.
3. **Le code d'invitation remplace la comparaison, sans l'affaiblir.**
   L'officine qui invite tire un `Ticket` (dix octets) pour cette seule
   porte et le montre dans son code d'invitation, transmis par un canal
   qu'elle choisit (téléphone, en personne). Une fois les `Hello`
   échangés, le poste qui rejoint envoie `Proof` : BLAKE3 à clé (dérivée
   du ticket) sur son rôle et **l'empreinte de cette poignée de main** —
   ou `Proof(None)`, sans ticket. L'invitant vérifie, répond par sa
   propre preuve (l'autre rôle), puis seulement envoie la clé. Un
   intermédiaire tient deux poignées de main, donc deux empreintes, et
   n'a de preuve pour aucune sans le ticket. **Une invitation à ticket
   est stricte** : `Proof(None)` y est refusé, sans repli sur la
   comparaison — une porte ouverte qu'un tiers devance ne doit pas
   devenir une comparaison bâclée. Une preuve fausse ferme l'invitation :
   quatre-vingts bits, une tentative. `Sync` ne prend pas de ticket.

**Les officines qu'on n'a pas ajoutées sont comptées, jamais gardées.**
Le dossier d'échange montre aussi ce que déposent des officines qui
détiennent la clé du réseau sans avoir été ajoutées ici (qu'un autre
membre a fait entrer, ou qu'on a retirées — la clé ne change pas). 0.318.0
les gardait au journal ; une revue de sécurité a montré qu'un tel auteur
pouvait alors **effacer** l'enregistrement d'une officine appairée (une
« correction » n'est pas liée à l'auteur de ce qu'elle corrige) ou porter
le rang de Lamport au plafond et brouiller pour de bon l'ordre du réseau,
et qu'un dossier inondé d'identités jetables figeait la vue. Depuis
0.320.0 : le dossier ne les fait pas entrer au journal ; ils sont
seulement **comptés** (32 identités au plus, les écartées non comptées)
et rapportés à
l'écran par la synchronisation ; la carte les propose, avec une alerte
quand le nom qu'elles écrivent est celui d'une officine déjà ajoutée. Les
ajouter (deux clics) relit le dossier, qui garde alors ce qu'elles
déposent ; le nom qu'elles écrivent n'est pas repris comme nom. Une
officine retirée est écartée d'office (`net_ignored`).

Ce qu'une **conversation directe** relaie d'auteurs qu'on n'a pas ajoutés
(les officines d'une officine appairée) reste gardé et relayé, comme
avant — le redemander à chaque échange coûterait tout le trafic. Mais
`absorb` lit à travers une **vue de confiance** (`trusted_view` : cette
officine et les siennes) : une correction venue d'un autre n'y efface
rien. Un tel auteur relayé ne règle pas non plus le rang de Lamport d'ici
(`Journal::trust`, 0.321.0).

**La connexion au lancement** (0.321.0). Le fil automatique, lancé à
chaque ouverture, tient une porte sur `[reseau] port_ecoute` (7745 ;
`ecouter`, oui par défaut) dès que l'officine est d'un réseau — **distinct
de `port`**, celui des invitations, que la porte tenue ouverte aurait
sinon pris. `Session::answer_any` répond
pour **n'importe lequel** de ses réseaux : son `Hello` ne nomme aucune
officine, celui de l'appelant nomme la sienne, qui doit être l'un des
réseaux d'ici *et* connaître l'appelant — sinon refus, avant le moindre
enregistrement ; `drive_any` ouvre alors l'échange sur le journal de ce
réseau. Chaque conversation a son fil, deux au plus, huit secondes par lecture
et **une minute en tout** (`TcpLink::with_deadline` : le délai par lecture
est une option de socket, et qui envoie un octet toutes les sept secondes
le satisfait sans fin) ; les appels sortants ont leur propre compte. L'annonce `BPMOFFICINE2` dit le port d'écoute et la ville
(celle de la configuration, sans code postal) ; une officine **appairée**
qu'on entend est composée à cette adresse (`dial_heard`), une fois toutes
les dix minutes au plus, et l'adresse n'est **gardée qu'une fois que
l'officine attendue y a répondu** — une annonce ne prouve rien, et une
annonce contrefaite ne remplace pas une adresse saisie. La ville annoncée
ne s'affiche que pour une officine qu'on n'a pas ajoutée. Au-delà du réseau local, sans serveur,
rien ne se découvre : restent les adresses saisies et le dossier.

**Entre deux sites** (0.326.0) : une officine joignable de l'extérieur
(routeur, VPN) déclare son adresse (`net_public_address`, fenêtre du
réseau) ; `publish` l'envoie une fois par valeur, **aux seuls membres du
réseau**, chiffrée et signée. `absorb` range la dernière adresse annoncée
par chaque officine appairée à part (`net_announced:<empreinte>`), jamais à
la place de l'adresse saisie ; la synchronisation l'essaie pour une
officine sans adresse ou dont l'adresse ne répond plus, et ne la retient
qu'une fois l'officine attendue jointe (`dial_heard`). Revue de sécurité
(0.328.0) : **une adresse IP et un port, rien d'autre**
(`usable_announced` — ni nom d'hôte, dont la résolution n'a pas de délai,
ni boucle locale, lien local, multidiffusion ou port nul), vérifiée à
l'enregistrement et à la réception ; jamais essayée à la fermeture, qui
attend la fin de la tâche ; le `Net` de la boucle est relu après un
`dial_heard` réussi, pour ne pas récrire par-dessus des valeurs plus
récentes ; l'annonce repart à chaque enregistrement (`…_seq`), et une
officine retirée perd l'adresse qu'elle avait annoncée.

**Revue d'ensemble (0.334.0).** Une revue de toute la couche depuis 0.315.0 :
- une **correction ne vaut que de l'auteur de ce qu'elle corrige**
  (`trusted_view`) — une officine appairée effaçait sinon une rupture
  d'une autre ;
- **qui a envoyé, c'est qui a signé** : ruptures, valeurs sourcées et
  versions prennent le nom de l'auteur du relevé (nom donné ici, sinon
  celui qu'il signe), jamais celui que la charge utile déclare ;
- la porte ne lit **aucun journal avant la poignée de main** (clés et
  membres seulement ; `drive_any` charge celui du réseau choisi), une
  conversation à la fois par adresse et six par minute ;
- une adresse apprise ou annoncée est rangée à part (`net_reached:`),
  jamais à la place de l'adresse saisie, et l'appel aux officines
  appairées entendues ne dépend plus de la règle d'affichage « la
  première adresse tient » (un essai par minute et par officine, une
  adresse en échec écartée dix minutes) ;
- l'invitation faite à une voisine depuis la carte n'admet **qu'elle**
  (`Session::expecting`) : une autre est refusée avant tout code, et la
  porte reste ouverte pour la bonne ; de même pour « Rejoindre son
  réseau » ;
- une publication et une lecture à la fois par processus, le journal
  relu sous le verrou avant d'écrire ; officine et appartenance écrites
  d'un seul tenant ; IPv4 écrite en IPv6 jugée comme telle.

**Reste ouvert** : un identifiant (`<base>:<n>`) repris par une officine
appairée hostile garde sa place `INSERT OR IGNORE` là où il arrive le
premier — désormais **attribué à qui l'a signé**, mais la bonne ligne est
écartée. La parade (l'auteur rangé avec chaque ligne reçue, un identifiant
déjà tenu par un autre auteur refusé) demande une colonne de plus dans
deux tables répliquées : à décider.

**L'horloge ne se règle que sur les auteurs de confiance** (0.321.0) :
`Journal::trust` — cette officine et les siennes — et un rang venu d'un
autre (relayé par une officine appairée) ne déplace plus celui des
écritures d'ici. Ce qui restait ouvert plus haut est fermé.

Les invitations faites **à une officine voisine** depuis la carte (vue
par son annonce UDP `BPMOFFICINE1`, nom déclaré par elle-même) n'ont pas
de ticket, sont les seules annoncées sur le réseau local, et passent
par la comparaison des cinq groupes.

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

## 6. Télémétrie : définition et limites

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

## 7. Intégration dans l'application (0.277)

Les décisions que cette section laissait à l'officine ont été prises
avec elle le 23/09/2026 ; ce qui suit est ce qui est construit, et où.

### 7.1 Où vivent les clés

Dans la base chiffrée, table `sync_local`, **qui ne voyage jamais** : la
graine du poste (`device`), la clé des postes (`trousseau`), le numéro
du poste (`post`), les empreintes des déclencheurs. La liste des postes,
elle, voyage (`sync_posts` : numéro, identité, nom, arrivée, retrait) —
chaque poste sait quels postes existent. Le réseau d'officines garde sa
propre clé dans `settings` (`net_trousseau`), qui ne scelle que
`Stream::Reseau` : les deux clés n'ouvrent rien l'une de l'autre, et un
test le vérifie sur les octets (`the_posts_key_and_the_network_key_open_nothing_of_each_other`).

### 7.2 Ligne ou opération : l'opération, champ par champ

`src/replica.rs` tient la liste des tables qui voyagent (`TABLES`, avec
leur flux) et de celles qui restent (`LOCAL`, avec la raison) ; un test
lit le schéma des deux fichiers et échoue sur une table rangée nulle part
(`every_table_either_travels_or_stays_and_none_is_forgotten`).

Une écriture voyage comme une opération : création (la ligne entière),
modification (**les seuls champs changés**, avec leur ancienne valeur),
suppression (la ligne vue). Reçue, une modification ne s'applique que
sur la valeur qu'elle remplaçait, champ par champ ; sinon elle attend
« à arbitrer » (`sync_conflicts`), et « Garder la mienne » renvoie la
valeur locale comme remplaçant l'autre, ce qui remet les postes
d'accord. Le registre, `stup_labels`, `stup_numbers`, la caisse et les
journaux des ruptures et des versions sont **en ajout seul** : une
modification reçue pour eux est refusée, et le refus se voit.

### 7.3 La projection

**Capturée par des déclencheurs** posés par `Db::install_capture` —
seulement sur une base qui fait partie d'un groupe : hors groupe, aucun
déclencheur, aucun coût, aucun changement. Chaque écriture d'une table
qui voyage laisse sa ligne avant et après dans `sync_log` (un par
fichier), dans la transaction qui écrit ; `Postes::publish` en fait des
lots de moins de quarante mille octets, scellés par flux. Ranger ce qui
vient d'ailleurs se fait **sans capture** (`sync_mute`, dans la
transaction qui range), si bien qu'une écriture ne revient jamais à son
poste. Les migrations aussi sont muettes : chaque poste fait les
siennes.

**Les numéros viennent d'un bloc par poste** (`sync_blocks`,
`db::next_id`) : chaque `INSERT` d'une table numérotée tire le suivant du
plus haut numéro du bloc de ce poste — jamais celui d'un autre, jamais un
numéro supprimé. Les dossiers ont des blocs de cent mille lisibles (le
fondateur garde les siens), le reste des blocs d'un million de millions.
`replica_inserts_draw_from_the_block` lit `db.rs` et refuse un `INSERT`
qui oublierait son `id`.

**Un seul poste numérote l'ordonnancier** — le poste de référence,
réglage `sync_reference` qui voyage. Une délivrance écrite ailleurs part
sans numéro ; il la numérote en la recevant (`stup_numbers`, en ajout
seul) et le numéro revient. Le même poste est le seul à semer le contenu
livré.

### 7.4 L'écran

Options › Base › « Postes de l'officine… » : l'empreinte du poste, le
groupe, les postes (nom, référence, retrait), ce qui attend d'être
arbitré, et les gestes — fonder, rejoindre (qui **remplace** ce que le
poste contenait), inviter, synchroniser, quitter. La réserve est écrite
avant le contenu.

### 7.5 Le transport

Trois chemins (`src/postes.rs`) : **automatique sur le réseau local**
tant que l'application est ouverte — une porte tenue ouverte aux postes
du groupe, une annonce UDP toutes les trois secondes qui ne porte que
l'identité et le port, une conversation dès qu'une écriture est
capturée ; un **dossier d'échange** (`<empreinte>.bpmposte`) ; des
**adresses écrites** ; plus le bouton et la fermeture. `link.rs` ne
décide toujours de rien : c'est l'application qui a un fil.

Réseau d'officines, contenu versionné : fiches, codex, protocoles et,
depuis la 0.295, **lignes du TROD** (`versions::Kind::Trod`, nom de
voyage `protocole · nom`, champs `versions::TROD_FIELDS` — tout sauf le
rang) et **vaccins du catalogue** (`Kind::Vaccin`, champs `label`,
`code` et `schedule`). Ces deux-là voyagent sous une **identité figée**
(`origin`) : le nom livré pour une entrée livrée, sinon le nom porté à
la première publication. Le nom affiché est un champ comme un autre — un
renommage est une version, pas une entrée de plus chez les autres. Une
entrée livrée absente chez celle qui reçoit (retirée par elle) n'est pas
recréée ; deux entrées de même identité ne sont versionnées qu'une fois. Une ligne reçue inconnue est créée dans un protocole que
l'application connaît, jamais dans un protocole inventé.

Réseau d'officines (`src/network.rs`) : chaque conversation directe
est notée sur la ligne de l'officine (`net_peers.last_try`, `last_ok`,
`last_error` — la raison en français, jamais l'erreur brute du lien), et
chaque lecture du journal note ce que l'officine a envoyé : le nom sous
lequel elle signe (`seen_as`, le champ `officine` de ses charges), le
compte de ses enregistrements (`received`) et, quand ce compte monte,
l'heure des dernières nouvelles (`last_heard`). Les heures viennent de
SQLite (`strftime(…, 'localtime')`) : aucune horloge dans le module. Ces
colonnes sont locales au poste, comme la table.

### 7.6 Données non répliquées

Les pièces scannées (§ 7.6 d'origine : des fichiers, pas des
enregistrements), la télémétrie du poste, le journal du réseau
d'officines et ce que ce poste en a déjà envoyé, et tout `sync_*` propre
au poste.

### 7.6 bis Plusieurs réseaux

Le réseau principal garde son rangement d'origine (`settings.net_trousseau`,
`net_records` à `network` vide, clés de `net_published` telles quelles) :
rien n'a été migré. Les réseaux de plus vivent dans `net_networks`
(empreinte de la clé, clé, nom, trois interrupteurs de partage ; voyage
entre les postes comme `settings`, écrit contre ce que l'écran montrait) ;
le dossier d'échange de chacun, propre au poste, dans `net_folders` ; qui
est de quel réseau, dans `net_memberships` (local, comme `net_peers`, avec
un compte d'enregistrements par réseau) — une officine sans ligne est du
principal. Rejoindre un réseau de plus se fait sur un journal vide (rien
du principal ne passe à celle qui invite) ; rejoindre un réseau dont on
est déjà ne range rien. Chaque réseau a son journal ; ce qui est parti se note
`<réseau>|<clé>`. Un dossier d'échange peut servir à plusieurs réseaux :
chacun y lit tous les fichiers `.bpmnet` et ne garde que les
enregistrements **que sa clé ouvre**. L'identité de l'appareil et sa clé
de boîte sont les mêmes partout. Un message part par chaque réseau qui
compte un de ses destinataires, scellé pour ceux de ce réseau.

### 7.7 La boîte scellée : un message pour quelques officines

Le trousseau du réseau ouvre tout ce que le réseau transporte, et toutes
les officines membres le tiennent. Un message pour deux officines sur
dix passe donc sous un **second sceau**, `bpm_sync::boxed` :

* chaque appareil a une **clé de boîte** X25519, troisième dérivation de
  sa graine (`Device::box_secret`, domaine `bpm-caddy/device-box/v1`) —
  jamais la clé de signature ;
* chaque officine **annonce** la partie publique une fois dans le journal
  (`{"t":"boite","k":…}`) ; la signature de l'enregistrement dit qui
  l'annonce, et `net_peers.box_key` la garde ;
* un message est scellé une fois sous une clé de contenu tirée au hasard
  (XChaCha20-Poly1305), elle-même scellée pour chaque destinataire sous
  une clé dérivée (BLAKE3 `derive_key`) d'un échange X25519 avec une clé
  jetable ; l'en-tête est lié au contenu ;
* la boîte **ne nomme pas** ses destinataires : chacun essaie les
  entrées ; l'expéditrice s'y met aussi, pour se relire ;
* dans le journal, la charge commence par `BOX1` : les lecteurs JSON
  (ruptures, versions, valeurs) la laissent passer.

Un **fichier joint** voyage en morceaux de 12 Kio (en hex, dans la table
`message_chunks` entre postes, un morceau par boîte entre officines),
décrit par `message_files` (nom, taille, empreinte BLAKE3, nombre de
morceaux). Il ne s'enregistre qu'entier et conforme à son empreinte ;
5 Mio au plus, parce que chaque morceau reste au journal de chaque poste
et de chaque officine du réseau.

Un message qui nomme un patient ne part qu'après confirmation, et
l'envoi est écrit au journal des accès (`transmis`). Un message dont une
destinataire n'a pas encore annoncé sa clé attend la synchronisation
suivante. Deux postes qui absorbent la même boîte avant de se
synchroniser entre eux peuvent l'écrire deux fois ; l'`uid` évite tout
autre doublon.

## 8. Travaux restants

1. **Le journal ne se compacte pas.** Chaque poste garde tout ce qui a
   été écrit depuis la fondation, et le relit en mémoire à chaque
   synchronisation. Une photographie datée, et l'oubli de ce qui la
   précède, viendront quand un groupe en aura besoin.
2. **Deux appairages simultanés** sur deux postes différents, hors
   ligne, donneraient le même numéro aux deux arrivants. Appairer un
   poste à la fois.
3. **L'ordre des lignes d'un même jour** se lit encore par numéro dans
   quelques vues ; entre deux postes, le bloc du second passe après celui
   du premier.
