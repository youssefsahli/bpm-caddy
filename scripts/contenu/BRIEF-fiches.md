# Drafting new drug cards for BPM-Caddy — brief

You draft **new monographs (drug cards)** for a French clinical pharmacy
app, as a snippet file that another session pastes into the Rust tables.
Repo: /home/youssef/Projects/BPM_Caddy (READ-ONLY for you — do not edit
any repo file, do not run cargo, do not run git). Write only your snippet
file(s) under the scratchpad path you are given.

Scratchpad: <brouillon>/

## The worked example — copy its shape exactly

`scratchpad/b0/palexia.rs` is a complete, accepted card (tapentadol).
Read it first. Every snippet file you write uses the same `## SECTION`
markers; lines after a marker are pasted verbatim at the end of that
table. Sections: DRUGS, DETAILS, POSOLOGIES, HALF_LIVES, NO_HALF_LIFE,
BEYOND, IMPACTS, RENAL, HEPATIC, ELDERLY, CYP, CRUSH, GRAVIDITY, and
NOTES (free text for the integrator: a new class label you needed, a
doubt, a figure you could not confirm). One file per card:
`<dir>/<brandname-lowercase>.rs`.

Read, for tone and density, two or three existing cards of the same
family in `src/db.rs` (search `name: "Tramadol"`, `name: "Eliquis"`, and
a card close to yours). Matching their length and precision is the job.

## The card

- `DRUGS`: `("Brand", "dci", "class", "antidote")`. The name is the
  French brand as sold in France (or the DCI when the product is mostly
  generic). The class: **reuse an existing class label** whenever one
  fits — `scratchpad/cards.tsv` lists name, DCI, class, antidote of all
  862 shipped cards. If none fits, write a new sober label and say so in
  NOTES. Antidote: usually `""`; if you name one, the `toxicity`
  section is mandatory.
- `DETAILS` (`StarterDetail`): all of indications, mechanism, dosage,
  contraindications, ddi, adverse, monitoring, iup, half_life,
  elimination, renal, pregnancy, sources, tags are required and must be
  real prose (several sentences; at least ~200 characters for the
  clinical fields; the iup ~600-1200 characters, addressed to the
  patient as « vous », plain words). `status`, `smr`, `forms`: `""`.
  - `sources`: one per line joined with `\n`: first
    `RCP <Brand> — base de données publique des médicaments (ANSM)`, then
    HAS / ANSM / learned-society / CRAT references that genuinely apply.
  - `tags`: short comma list reusing the existing vocabulary
    (`surveillance biologique`, `contre-indiqué grossesse`,
    `vigilance conduite`, `stupéfiant`, `ordonnance sécurisée`,
    `anticancéreux oral`, `biosimilaire`, `marge thérapeutique étroite`,
    `collyre`, `dermatologie`, `pédiatrie`, `vaccin`, class words…).
    Tags are searched as substrings by the clinical tables: never put a
    word in a tag that would make a table mistake this drug for another.
  - `toxicity` only where **a dose, a duration or an exposure kills**
    (opioids, narrow margin, anticancer, insulin-like, QT, K+…); ≥ 120
    characters; it says what is checked at the counter: the figure that
    decides, the association that kills, the gesture not to get wrong,
    what happens at stopping. Otherwise `""`.
- `POSOLOGIES`: 1 to 4 rows `("Brand", "indication", "posologie",
  "remarque")`, contiguous. The remarque is what is said at the counter
  — never empty.
- Facets (`src/facets.rs`):
  - `HALF_LIVES`: `("Brand", low_hours, high_hours)` — must match what
    your `half_life` field writes (convert days to hours). If the field
    is qualitative, use `NO_HALF_LIFE` with `Unknown::NonChiffree`; for a
    non-absorbed product, a vaccine, an ion: `Unknown::SansObjet`. Every
    card needs exactly one of the two.
  - `IMPACTS`: `("Brand", Organ::X, Effect::Altere|Traite,
    Grade::Mineur|Notable|Majeur, "short clause")`. Organs: Thyroide,
    Coeur, Foie, Rein, Moelle, Oeil, Os, Poumon, Neuro, Peau, Digestif,
    Oreille. An `Altere` must be backed by the organ's vocabulary in the
    card's own text (adverse/toxicity), describing what the drug does to
    the organ — the clause never *starts* with a patient-terrain word
    (« insuffisance… », « antécédent… ») or a monitoring gesture
    (« surveiller… », « transaminases avant… »). `Traite` = the organ is
    what the drug treats (grade = centrality). Clause ≤ ~90 chars.
  - `BEYOND`: `("Brand", "clause")` only if the card says the effect
    outlasts plasma half-life (irreversible inhibition, depot, tissue).

## Optional clinical-table rows (write them when the card warrants it)

Match strings (`needs`) are lowercase, **accent-folded** substrings
searched in « name dci class tags » of every treatment. A fragment also
matches *inside other words* (« ipp » is inside « grippe »). Before using
a fragment, check what it catches among the shipped cards:

```
python3 - <<'EOF'
import unicodedata,sys
def f(s): return ''.join(c for c in unicodedata.normalize('NFD',s) if unicodedata.category(c)!='Mn').lower()
frag='tapentadol'
for l in open('<brouillon>/cards.tsv'):
    if frag in f(l): print(l.strip())
EOF
```

Name molecules (and the brand), not class words. Look at the existing
table (`src/renal.rs` `TABLE`, `src/hepatic.rs`, `src/elderly.rs`,
`src/cyp.rs`, `src/crush.rs`, `src/gravidity.rs`) for the struct shape
and for a row that **already claims your card by class or by a shared
fragment** — the first row that claims a card is the one that speaks, and
a duplicate claim fails a test. If an existing row already covers it,
don't add one (say so in NOTES).

- RENAL `Adaptation { needs, never: &[], label, steps: &[Step { below:
  <mL/min>, level: Level::Contraindicated|Reduce|Watch, conduct }],
  source }` — only when the card's `renal` field asks for something;
  must not contradict it. (« non recommandé » is written Contraindicated,
  as existing rows do.)
- HEPATIC `Adaptation { needs, label, steps: &[step(Mild|Moderate|Severe,
  Contraindicated|Reduce|Watch, "conduct")], source }` — **a conduct
  carries no figure except a ceiling** (no « 50 mg »); source quotes the
  card verbatim: `"Brand : « exact words from the card » ."`; the card
  must speak of the liver.
- ELDERLY `Inappropriate { needs, never: &[], label, from: 65|75,
  level: Level::Avoid|Caution, risk, instead, source }` — threshold from
  a published list (STOPP/START, Beers 2023, Laroche); `instead` is a
  choice, never a dose, and never a drug that itself is a row to avoid;
  the card must speak of age / sujet âgé.
- CYP `Profile { needs, label, actions: &[Action::new(Enzyme, Role,
  Some(Force))] or Action::prodrug(Enzyme, Some(Force)), source }` —
  Enzyme ∈ Cyp1a2, Cyp2b6, Cyp2c8, Cyp2c9, Cyp2c19, Cyp2d6, Cyp3a4; Role
  ∈ Substrate, Inhibitor, Inducer; Force ∈ Weak, Moderate, Strong. The
  card must name each enzyme; source quotes the card verbatim as above.
- CRUSH `Rule { needs, label, verdict: Verdict::No|Conditional|Yes,
  why, instead, source }` — oral forms only; `instead` required when No;
  a verdict never contradicts its reason.
- GRAVIDITY `Advice { needs, never: &[], label, pregnancy:
  Level::Interdit|Eviter|Prudence|Compatible|SansDonnee, term,
  pregnancy_note, breastfeeding: Level::…, breastfeeding_note, source }`
  — source cites the CRAT; « Compatible » never contradicts the card.

## Hard rules

- French, terse professional register (it is read by pharmacists at the
  counter). Code/section markers stay as given.
- **No invented figures.** Every dose, ceiling, half-life, threshold must
  be one the RCP / official source writes. If you are not sure of a
  number, say it qualitatively and flag it in NOTES. No posology the
  source does not write. Silence is not permission: unknown = « à
  vérifier ».
- Characters: straight apostrophe `'`, French quotes « » with plain
  spaces, `≥ ≤ ≈ µ` allowed. **No arrows, no U+202F / U+00A0, no curly
  quotes, no double quote `"` inside a string, no emoji.** `\n` only in
  `sources`.
- Rust string literals: every field is one line `field: "…",`.
- Don't reuse an existing card name (check cards.tsv).

When done, reply with: the files written, the class labels used (flag new
ones), and every figure or claim you are less than sure of.

## Lessons from the first wave (read these — they are now rules)

- **Check the name is new against `src/db.rs` itself** (`grep -n '"Brand"' src/db.rs`), not only cards.tsv: a drafter rewrote Xeplion, which already existed.
- `cards.tsv` now lists all 963 shipped cards, including the 101 added this morning.
- **New class label** → add a `## CLASSES` section:
  `Class { name: "…", family: "<key>", aliases: &[], },` with family key
  one of cardio, hemato, neuro, psy, douleur, infectio, respi, digestif,
  endocrino, uro, gyneco, derm, ophtalmo, immuno, os, divers.
- **Every card needs a conduite** (« en cas d'oubli » / « signes qui font
  consulter »), found by keyword in class / tags / DCI / name among the
  rules of `STARTER_CONDUITE` in src/db.rs (first match wins). If no
  existing keyword reaches your card, add a `## CONDUITE` section:
  `( "keyword", "missed-dose conduct", "red flags" ),` — keyword unique,
  catching only what it should. Vaccines and professional-administered
  products are exempt (look at the test
  `every_card_has_the_two_answers_or_is_a_named_exception`).
- **Renal conducts never carry milligrams** (write a fraction of the
  usual dose, « la plus faible dose », « plafond réduit »). Hepatic
  conducts carry no figure except a « ne pas dépasser » ceiling.
- A renal row must not claim a card whose `renal` field says « pas
  d'adaptation » without any mL/min figure.
- **Placement**: a row that must speak before an existing row goes in a
  section `## HEPATIC @ Exact existing label"` (note the closing double
  quote: the label is matched exactly) — it is inserted just above that
  row. Specific CRUSH rows are placed automatically before the general
  rules; don't worry about that.
- Local forms (class containing collyre, topique, local, crème, gel
  buccal…): renal and hepatic tables now skip them automatically, so
  write no renal/hepatic rows for them. Gravidity still reads them — a
  systemic row that claims a local form fails
  `a_local_form_does_not_wear_the_systemic_level`; say so in NOTES.
- Impacts: `Altere` needs the organ's harm vocabulary in the card text
  (see `HARM_WORDS` in src/facets.rs tests), `Traite` needs the organ's
  treat vocabulary in the indication (`TREAT_WORDS`). Check both lists.
- The BDPM (base-donnees-publique.medicaments.gouv.fr) and EMA product
  information can be downloaded with curl; drafters who did so produced
  the most reliable cards. Check marketing status there too.
# Addendum to BRIEF.md (2026-09-29) — read after the brief, it overrides it

- The brief lives at <brouillon>/BRIEF.md ; its worked example is `scripts/contenu/exemple-palexia.rs` in that same scratchpad. Read both.
- Use the CURRENT card list: <brouillon>/cards/cards.tsv (1117 cards). Write your snippet files into the directory you are given under .../scratchpad/cards/.
- **Download the French RCP for every card** (BDPM `affichageDoc.php?specid=<CIS>&typedoc=R`, or the EMA French EPAR PDF `https://www.ema.europa.eu/fr/documents/product-information/<name>-epar-product-information_fr.pdf`; curl -L with a browser User-Agent; pdftotext). Cards written from memory had many errors yesterday. Put the document and date you read in NOTES. Check marketing status in France.
- CYP rows: `Enzyme` now also has transporters `Pgp`, `Oatp1b1`, `Bcrp`, `Oct2`, `Mate1`. The card text must name each one (« P-gp » / « glycoprotéine P », « OATP1B1 », « BCRP », « OCT2 », « MATE1 »). Force only if the card writes « puissant / modéré / faible », else None: `Action::new(Cyp3a4, Inhibitor, None)`. Before adding a CYP row, grep src/cyp.rs for your molecule (a row may exist already with the DCI in `needs`).
- A card whose molecule appears in a shipped cascade (grep `molécule <dci>` in src/db.rs STARTER_CASCADES) gets its cascade door automatically; no action needed.
- Keep every `needs` fragment specific (molecule name or brand) and check what it catches in cards.tsv.
