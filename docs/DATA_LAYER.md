# Couche de données structurée — proposition

*État : proposition, à relire avant toute ligne de code. Rien de ce qui suit
n'est construit.*

## 1. Constat chiffré

Aujourd'hui une fiche est de la **prose annotée** : un nom, une DCI, une
classe en texte libre, et une dizaine de champs rédigés. Huit tables
cliniques la lisent **par sous-chaîne** — `renal` (38 lignes), `hepatic`
(111), `gravidity` (37), `elderly` (29), `crush` (40), `cyp` (114),
`biology` (102), `surveillance` (66) — et chacune repose sur
`contains_folded` sur le nom, la DCI et la classe.

Ce que cette lecture a coûté, et qui est écrit dans `ARCHITECTURE.md` :

- **Un fragment vit dans un autre mot.** « ipp » dans « grippe » (le Tamiflu
  lu comme un IPP), « imeth » dans « diméthylfumarate » (Skilarence lu comme
  du méthotrexate dans trois modules), « fer » dans « cholécalciférol ».
- **La voie n'est pas une donnée.** Un collyre d'indométacine tombait dans la
  ligne rénale de l'indométacine ; la parade (`classes::is_local_form`) lit la
  voie *dans la classe*, en texte, avec une liste de mots inclus et exclus.
- **La classe dérive.** 495 libellés pour 862 fiches, `anti-TNF` et
  `anti-TNF alpha` comptés comme deux classes ; la parade (`classes.rs`)
  replie après coup.
- **Une association n'est pas une molécule.** Xigduo porte deux molécules ;
  une table qui s'arrête à la première ligne qui répond perd la seconde.
- **Chaque table a son test de confrontation** avec les fiches livrées, parce
  que seule la rencontre avec les vraies fiches trouve ces pièges.

Toutes ces parades sont justes, et toutes sont la même chose : reconstruire,
à la lecture, une identité que la donnée ne porte pas.

## 2. Proposition : deux couches, la seconde porte les identités

**Couche 1 — la prose annotée (ce qui existe).** Inchangée. Elle reste ce
qu'on lit, ce qu'on imprime, ce que l'officine réécrit. Elle est la
**référence** : toute affirmation de la couche 2 renvoie à un passage de la
couche 1 ou à une source extérieure citée.

**Couche 2 — des entités et des affirmations, à la façon de Wikidata.**

- **Entités**, chacune avec un identifiant stable et des alias :
  - *Molécule* (`M:apixaban`) — DCI, synonymes, code ATC, et, facultatif,
    l'identifiant Wikidata (`Q…`) et le code de la BDPM.
  - *Présentation* (`P:eliquis-5-cp`) — la boîte : marque, forme, voie,
    dosage, CIP13 connus.
  - *Classe* (`C:aod`) — les classes canoniques de `classes.rs`, avec leur
    famille.
  - *Voie* (`R:orale`, `R:ophtalmique`, `R:cutanee`…) — une liste fermée.
  - *Enzyme / transporteur* (`E:cyp3a4`, `E:pgp`).
  - *Axe / organe* (`A:rein`, `A:foie`…) — ceux de `facets.rs`.
  - *Source* (`S:rcp-eliquis`, `S:has-2024-vrs`, `S:crat`) — ce qui fonde une
    affirmation.
- **Affirmations** : *(sujet, propriété, valeur, qualificatifs, références,
  rang)*. Exemples :
  - `P:eliquis-5-cp` —*contient*→ `M:apixaban` (qualificatif : 5 mg)
  - `P:indocollyre` —*voie*→ `R:ophtalmique`
  - `M:apixaban` —*classe*→ `C:aod`
  - `M:apixaban` —*substrat de*→ `E:cyp3a4` (qualificatif : majeur ;
    référence : `S:rcp-eliquis`, section 4.5)
  - `M:metformine` —*adaptation rénale*→ « contre-indiquée »
    (qualificatifs : DFG < 30 ; référence : RCP 4.2)
  - `M:amoxicilline` —*demi-vie*→ 1 h (qualificatif : forme orale)
- **Rang** (préféré / normal / déprécié), comme Wikidata : deux sources qui
  divergent restent **toutes deux** écrites, et l'une est préférée. C'est la
  règle « un conflit est montré, jamais tranché ».

**Couche 3 — les lectures** (les modules actuels) interrogent la couche 2
**par identité** : « les présentations de voie générale qui contiennent une
molécule substrat majeur du CYP3A4 », et plus « les fiches dont le texte
contient "3A4" ».

## 3. Conséquences

| Aujourd'hui | Avec la couche 2 |
|---|---|
| `needs: &["ipp"]` attrape « grippe » | la ligne vise `C:ipp` ; seules les molécules de cette classe répondent |
| `never` et `is_local_form` rattrapent les collyres | la présentation porte `R:ophtalmique` ; la table demande la voie générale |
| Xigduo : la première ligne qui répond gagne | la présentation contient deux molécules ; chacune est lue |
| une classe mal orthographiée crée une classe | la classe est une entité ; un libellé libre n'est qu'un alias |
| « combien de fiches ont telle propriété » : impossible sans relire la prose | une requête |

Et une chose que la couche 1 ne peut pas donner : **une affirmation sait
d'où elle vient.** Le panneau rénal peut écrire « RCP Glucophage, 4.2 » à
côté de sa ligne, parce que la référence est une donnée.

## 4. Stockage

Dans la base SQLCipher existante, quatre tables et rien d'autre :

```
entities   (id TEXT PRIMARY KEY, kind TEXT, label TEXT)
aliases    (entity TEXT, text TEXT, folded TEXT)          -- recherche
statements (id INTEGER PRIMARY KEY, subject TEXT, property TEXT,
            value_entity TEXT, value_text TEXT, value_num REAL,
            unit TEXT, rank INTEGER)
qualifiers (statement INTEGER, property TEXT, value_entity TEXT,
            value_text TEXT, value_num REAL, unit TEXT)
refs       (statement INTEGER, source TEXT, locator TEXT)  -- « 4.2 »
```

Livrées **comme les fiches** : semées une fois depuis le binaire, puis
réécrites par l'officine sous le même mécanisme que `content_overrides`
(une réécriture se souvient de ce qu'elle remplaçait et se montre à relire
quand le livré change).

**Pas de réseau.** Wikidata sert de *modèle* et, facultativement, de
*correspondance* (un QID par molécule, rempli hors ligne par un outil de
préparation des données) — jamais d'appel depuis l'application. La posture
« tout est local » ne bouge pas.

## 5. Migration par tranches, chacune tenue par un test de parité

1. **Identité des molécules.** Extraire les DCI des 862 fiches en entités
   `M:` (les associations en deux ou trois), et relier chaque fiche à ses
   molécules. Test : chaque fiche livrée résout ≥ 1 molécule ; les
   associations connues en résolvent autant qu'elles en contiennent.
2. **La voie.** Une affirmation *voie* par présentation, calculée **une
   fois** depuis `is_local_form` puis relue à la main là où elle hésite.
   Test : parité exacte avec `is_local_form` sur les fiches livrées, puis
   `is_local_form` ne sert plus qu'à la migration.
3. **Une table portée : `renal`.** Ses lignes deviennent des affirmations
   *adaptation rénale* sur des molécules. Test de parité : pour chaque fiche
   livrée, l'ancienne lecture et la nouvelle rendent le même résultat — et
   chaque différence est listée, lue, et tranchée à la main (c'est là qu'on
   trouvera les fragments qui attrapaient la mauvaise fiche).
4. **Les autres tables**, une par version : `gravidity`, `crush` (sur les
   présentations), `hepatic`, `elderly`, `cyp`, `biology`, `surveillance`.
5. **Le retrait des `needs`.** Quand toutes les tables lisent par identité,
   le champ `needs` et `contains_folded` disparaissent des tables cliniques,
   et le piège des fragments avec eux.

Chaque tranche est livrable seule, et aucune ne change ce que l'écran dit —
sauf là où le test de parité montre que l'ancien disait faux.

## 6. Décisions en attente

1. **Le périmètre des entités.** Molécules, présentations, classes et voies
   sont indispensables. Enzymes, axes et sources sont utiles tout de suite
   pour `cyp`, `facets` et la traçabilité. Les pathologies et les
   indications (pour la posologie « selon l'indication ») sont un second
   temps — les inclure dès maintenant ?
2. **Les identifiants externes.** Livrer les codes ATC et les QID Wikidata
   (préparés hors ligne), ou seulement nos identifiants ?
3. **La présentation et le CIP13.** Aujourd'hui aucun catalogue CIP n'est
   livré, par choix (le code attend qu'un humain nomme le produit). La
   couche 2 peut *porter* des CIP appris par l'officine sans en livrer —
   c'est la proposition ; à confirmer.
4. **L'édition.** Faut-il un écran pour éditer les affirmations (avec
   références), ou la couche 2 reste-t-elle livrée et relue, l'officine
   n'éditant que la prose ?
5. **L'ordre.** La tranche 3 (`renal`) est proposée en premier parce que
   c'est la plus petite table dont les pièges sont documentés. `cyp` a le
   plus à gagner mais c'est la plus grosse.
