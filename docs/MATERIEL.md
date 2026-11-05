# Scanners et imprimantes de l'officine — préparation

Ce document prépare le chantier « scanners (Brother, Epson, Fujitsu…) et
imprimantes de pharmacie (Epson, Brother) ». Rien n'est construit encore :
il dit ce qui existe, le matériel visé, la façon de lui parler, l'ordre
des travaux et ce qu'il faut demander avant d'écrire une ligne.

## 1. Ce qui existe

**Numérisation** (`src/scans.rs`, `App::run_scanner`, `[scans]` de
`config.toml`) :

- « Importer… » : un fichier choisi, accepté sur ses octets (PDF, PNG,
  JPEG, TIFF), plafonné à `max_mb`, rangé chiffré dans `_scans.db`.
- « Numériser » : la ligne de commande `[scans] command`, `{out}` étant le
  fichier à relire (`scanimage` sous Linux, l'utilitaire du constructeur
  ailleurs). Vide par défaut.
- **À reprendre** : `run_scanner` attend sur le fil de l'interface
  (`SCANNER_PATIENCE` = 10 min) — la fenêtre est figée pendant que le
  chargeur avale les feuilles. Le premier travail est de le passer sur un
  fil, livré par un `Receiver` réveillé par `woken`/`wake_ui`, comme
  `bio_watch`.

**Impression** (`src/pdf.rs`) : chaque document de `pdf::DOCS` se
compile en PDF (Typst) puis s'ouvre dans la visionneuse du système
(`compile_and_open` → `open::that_detached`), où l'opérateur choisit
l'imprimante. Seule concession au matériel : `[ui] flacon_etiquettes`
(« rouleau » = une page de 57 × 32 mm, « a4 » = planche 3 × 8).

## 2. Le matériel visé

Scanners de documents courants en officine :

| Famille | Modèles | Réseau (eSCL) | Pilote |
|---|---|---|---|
| Brother ADS | ADS-1200/1700W/1800W, ADS-2200/2700W, ADS-4300N/4700W/4900W | les modèles W/N | WIA, TWAIN, brscan5 (SANE) |
| Brother DS (mobiles) | DS-640, DS-740D, DS-940DW | non (sauf DW, à vérifier) | WIA, brscan5 |
| Epson | DS-C330/C490, ES-50/60W, DS-530 II, ES-580W | les modèles W | WIA, epsonscan2 (SANE) |
| Fujitsu / Ricoh | fi-800R, fi-7160, ScanSnap iX1300/iX1600 | non (ScanSnap : protocole propriétaire) | WIA/TWAIN (fi), PaperStream |
| Canon | imageFORMULA DR-C225 II, R40 | non | WIA, TWAIN |
| Multifonctions | la plupart des MFP récents | oui (AirScan/Mopria) | — |

Imprimantes de pharmacie :

| Usage | Modèles | Langage brut |
|---|---|---|
| Ticket 80/58 mm | Epson TM-T20III/T88VI/T88VII, TM-m30 | ESC/POS |
| Mentions au dos de l'ordonnance (slip) | Epson TM-H6000 V, TM-S9000 | ESC/POS (station slip) |
| Étiquettes | Brother QL-700/800/820NWB/1100, TD-4 ; Epson LW / ColorWorks ; Dymo LabelWriter ; Zebra | Brother QL raster, ZPL… |
| A4 | n'importe laquelle | — (pilote) |

## 3. Comment leur parler

### Numériser

1. **eSCL / AirScan** (scanners en réseau) — HTTP + XML, découverte mDNS
   `_uscan._tcp` / `_uscans._tcp`. Sans pilote, identique sur les trois
   systèmes, en Rust pur (`ureq` et `quick-xml` sont déjà là ; il manque
   un navigateur mDNS). Le travail : `ScannerCapabilities`,
   `POST /eSCL/ScanJobs` (source Feeder/Platen, recto-verso, 300 ppp,
   gris, JPEG ou PDF), `GET …/NextDocument` jusqu'au 404, `ScannerStatus`.
   Couvre Brother ADS en réseau, Epson W, les multifonctions.
2. **WIA 2.0** sous Windows — l'interface que Brother, Epson, Fujitsu et
   Canon fournissent tous. COM par le crate `windows` : énumérer
   (`IWiaDevMgr2`), choisir le chargeur
   (`WIA_IPS_DOCUMENT_HANDLING_SELECT`), recevoir chaque page. Couvre les
   modèles USB sans eSCL.
3. **SANE** sous Linux — reste la ligne de commande (`scanimage`, avec
   `brscan5`, `epsonscan2` ou `sane-airscan`), avec en plus une liste
   `scanimage -L` dans les Options et des préréglages par marque à la place
   de la commande à écrire à la main.
4. **TWAIN : non.** Une fenêtre du pilote à chaque fois, souvent 32 bits ;
   WIA couvre les mêmes appareils.
5. macOS : eSCL suffit pour le réseau ; ImageCaptureCore plus tard si
   quelqu'un en a l'usage.

Les pages reçues (JPEG/PNG) s'assemblent en **un PDF multipage**
(`lopdf`, déjà en dépendance) puis passent par `scans::accept` comme
aujourd'hui : rien ne change dans ce qui entre dans la base.

### Imprimer

1. **Impression directe sur une imprimante nommée**, sans visionneuse :
   - Linux et macOS : CUPS accepte le PDF, `lp -d <imprimante> -o
     media=<format>` ; liste par `lpstat -e`.
   - Windows : pas d'impression native d'un PDF. On rend la page en
     image (`typst-render`, même version que `typst`) et on l'imprime par
     GDI (`StartDoc`/`StretchDIBits`, crate `windows`) ; liste par
     `EnumPrinters`. Fonctionne avec n'importe quel pilote, dont ceux
     d'Epson et de Brother.
2. **Brut, sans pilote**, pour les tickets et les étiquettes : la page
   rendue par `typst-render` à la résolution de la tête (203 ou 300 ppp),
   tramée en 1 bit, puis encodée :
   - Epson ESC/POS : `ESC @`, raster `GS v 0`, coupe `GS V` ; station
     slip pour la TM-H6000.
   - Brother QL : raster command reference (`ESC i a 01`, `ESC i z`
     support/largeur, lignes `g`, `0x1A`).
   Envoyé par TCP 9100, par le spouleur Windows en `RAW` (`WritePrinter`),
   ou par `/dev/usb/lp*` sous Linux.
3. La **visionneuse reste le défaut** et le repli quand l'imprimante ne
   répond pas.

## 4. Où ça se règle

Par poste, dans `config.toml` (c'est le matériel branché ici qui décide),
jamais dans la base :

```toml
[scans]
# appareil = "escl:http://192.168.1.40/eSCL"  |  "wia:<id>"  |  "commande"
# source = "chargeur"   # ou "vitre"
# recto_verso = true
# resolution = 300
# couleur = "gris"      # "couleur", "noir"

[impression]
# Un document, une imprimante. Absent : la visionneuse, comme aujourd'hui.
# flacon = { imprimante = "QL-820NWB", mode = "brother-ql", support = "62x29" }
# conseil = { imprimante = "TM-T20III", mode = "escpos", largeur = 80 }
# fiche  = { imprimante = "Bureau-A4" }
```

`[ui] flacon_etiquettes` se fond dans `[impression]` (migration de la clé
lue, pas cassée). Dans les Options, une page « Matériel » : les scanners
et imprimantes trouvés, « Tester » sur chacun, le document envoyé à
chaque imprimante.

## 5. Modules

- `src/escl.rs` (pur) : construire les requêtes, lire capacités, état et
  réponses ; testé sur des réponses enregistrées d'appareils réels.
- `src/escpos.rs` et `src/brother_ql.rs` (purs) : d'une image 1 bit aux
  octets ; testés octet pour octet contre les références constructeur.
- `src/peripheriques.rs` (pur) : le choix d'un appareil et d'un mode à
  partir de la configuration, les formats de support, les refus lisibles.
- E/S (HTTP, TCP 9100, WIA, GDI, `lp`) dans un module du bureau, toujours
  sur un fil, jamais sur celui de l'interface.

## 6. Contraintes du projet qui s'appliquent

- **« Une application sans code réseau »** : `--no-default-features
  --features desktop` ne contient rien qui parle au réseau (SYNC.md).
  eSCL et TCP 9100 sont du réseau local : soit une feature `materiel`
  séparée (dans `default`), soit l'admettre explicitement. À décider.
- Chaque chaîne dans `assets/strings.fr.toml` ; Options en widgets
  `motif` ; les modules purs sans horloge ni disque.
- Une pièce scannée reste identifiée par ses octets, plafonnée, chiffrée.
- Les gabarits Typst restent la source de chaque document : un format de
  ticket ou d'étiquette est un gabarit de plus dans `pdf::DOCS`, avec ses
  marqueurs et ses tests.
- Rien ici ne demande un serveur ; tout reste entre le poste et son
  matériel.

## 7. Ordre des travaux

1. `run_scanner` hors du fil de l'interface (petit, utile tout de suite).
2. Impression directe vers une imprimante nommée + `[impression]` par
   document (CUPS, puis Windows GDI par `typst-render`).
3. Encodeurs ESC/POS et Brother QL, envoi brut (9100, RAW, USB) ;
   gabarits ticket 80 mm et étiquettes Brother (62 × 29, 29 × 90, continu
   62).
4. eSCL : découverte, capacités, chargeur recto-verso, PDF multipage.
5. WIA sous Windows.
6. Préréglages SANE et liste `scanimage -L` sous Linux.

## 8. À demander avant de commencer

- Les modèles exacts de l'officine (scanner, imprimante de tickets,
  imprimante d'étiquettes) et leur branchement : USB ou réseau.
- Faut-il imprimer **au dos de l'ordonnance** (TM-H6000) ? D'habitude
  c'est le LGO qui le fait ; BPM-Caddy ne devrait pas le doubler sans
  raison.
- Quels documents vont sur un ticket ou une étiquette plutôt qu'en A4
  (étiquette de flacon, rendez-vous, conseil, rappel de vaccination…).
- Le matériel disponible pour tester : sans appareil, les encodeurs se
  testent contre la documentation, mais eSCL et WIA demandent au moins une
  capture d'un appareil réel.
