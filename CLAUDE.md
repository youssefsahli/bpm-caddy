# BPM-Caddy — notes for Claude Code

Clinical pharmacy desktop app (Rust + egui), UI in French, proprietary
license with free public releases. Spec: `docs/SPECIFICATIONS.txt`.

## Workspace

- root `bpm-caddy` — the app: `src/app.rs` (UI/state), `src/db.rs`
  (SQLCipher storage), `src/fuzzy.rs` (search), `src/pdf.rs` (Typst),
  `src/config.rs` (config.toml), `src/vaccines.rs` (calendrier vaccinal
  rules and the traveller's country table — static, pure, tested),
  `src/bulletin.rs` (fills the official Assurance Maladie bulletins
  d'adhésion in `assets/bulletins/`), `src/ordonnance.rs` (what a
  positive TROD allows, and the choices behind the ordonnance),
  `src/codex.rs` (reading a preparation's formula and rescaling it —
  pure and tested; the preparations themselves live in the base),
  `src/entretien.rs` (what each thematic covers, printed on the fiche),
  `src/biology.rs` (the analytes, their usual intervals, and the rules
  that read a value against the patient's treatments — static, pure,
  tested),
  `src/renal.rs` (what renal function does to an ordonnance: a table of
  molecules, each with **steps** — a DFG, a level, what the RCP says —
  and a source. `biology.rs` answers « this figure, under this
  treatment, means this » and `surveillance.rs` « this figure has not
  been asked for in too long »; this one answers the question actually
  asked at the counter — **this file has a DFG of 28, what becomes of
  each line of its ordonnance?** Both halves existed and nothing put
  them face to face. Four rules, one test each: **no DFG, no verdict**
  (the module names what depends on the kidney and says the figure is
  missing — `level` is an `Option`, and that is what stops it
  concluding), **the step that speaks is the lowest one crossed** (read
  at 28, a drug that halves under 60 and is contraindicated under 30 is
  contraindicated — the first step in the list would say « reduce the
  dose » of a drug to stop), **a threshold comes from the RCP, never
  from an interpolation**, and **the conduct is the RCP's, the decision
  is the prescriber's** — written on the panel, not only in the code.
  Pure, tested, no clock),
  `src/crush.rs` (« peut-on écraser ? » — the question asked every day,
  whose answer sits in the prose of dozens of cards. A table keyed on
  the **presentation** and a printable sheet for the EHPAD or the nurse.
  Five rules, one test each, and the first is the only one that really
  matters: **silence is not permission** — a drug the table does not
  know gets an answer, « à vérifier », never an absent line. A sheet
  showing only the refusals reads as a green light for everything else,
  and that is how a modified-release tablet gets crushed. **The
  presentation decides, not the molecule** (Moscontin never, Skenan by
  opening the capsule — a table by DCI would be wrong half the time),
  **opening a capsule is not crushing a tablet** (three answers, not
  two), **a « no » with no alternative leaves the problem whole**, and
  **some « no »s protect whoever crushes**, not the patient — a
  cytotoxic, a teratogen: the dust is the danger. A test holds the
  table's *order*, since the first row that matches wins: it earned its
  keep the day it was written — « actiskenan » contains « skenan », and
  immediate-release Actiskenan was getting Skenan LP's answer),
  `src/gravidity.rs` (pregnancy and breastfeeding as a level: twenty-four
  molecules, **two levels each** — they are two questions, and codeine is
  usable pregnant and discouraged while nursing, the AVKs the reverse.
  **It does not replace the CRAT**, and the panel says so in its footer:
  the French reference is kept up to date molecule by molecule and it is
  online, where a table frozen in a binary ages. Rules, one test each:
  **« no data » is not « no risk »** (`SansDonnee` is a level apart from
  `Compatible`), **the term decides** (an NSAID is not « to avoid », it
  is contraindicated from 24 weeks of amenorrhoea, even as a single
  dose — and for aspirin it is the *dose* that decides, the same
  molecule being a treatment *of* pregnancy at 75 mg), and **the table
  decides nothing**: stopping a treatment in a pregnant woman is a
  medical decision, and a badly-accompanied pregnancy is more dangerous
  than a treatment continued),
  `src/gravidity.rs` (pregnancy and breastfeeding as a level: twenty-four
  molecules, **two levels each** — they are two questions, and codeine is
  usable pregnant and discouraged while nursing, the AVKs the reverse.
  **It does not replace the CRAT**, and the panel says so in its footer:
  the French reference is kept up to date molecule by molecule and it is
  online, where a table frozen in a binary ages. Rules, one test each:
  **« no data » is not « no risk »** (`SansDonnee` is a level apart from
  `Compatible`), **the term decides** (an NSAID is not « to avoid », it
  is contraindicated from 24 weeks of amenorrhoea, even as a single
  dose — and for aspirin it is the *dose* that decides, the same molecule
  being a treatment *of* pregnancy at 75 mg), and **the table decides
  nothing**: stopping a treatment in a pregnant woman is a medical
  decision, and a badly-accompanied pregnancy is more dangerous than a
  treatment continued. A test in each of the three clinical tables
  refuses markup in what they write: `RichText` interprets none, so an
  asterisk typed for emphasis reaches the screen as an asterisk),
  `src/revue.rs` (what a set of treatments says about itself:
  doublons, associations, cascades — same shape, same discipline),
  `src/conciliation.rs` (the file's ordonnance against the one a patient
  brings back from hospital: reads a pasted list, matches each line to a
  fiche, and says what was stopped, changed, added or replaced — pure,
  tested, no catalogue of its own),
  `src/surveillance.rs` (what a treatment asks to have measured and how
  often, read against the dates already in the file — the other half of
  `biology.rs`: that one reads the values that are there, this one names
  the ones that are not),
  There is no `stats` module: the figures the « Statistiques » view
  shows are counts over lists the session already holds (the 851 cards,
  the summaries) plus four aggregate queries, and a module that only
  counted would be a module that only imports. What *is* worth writing
  down: the aggregate queries are covered by
  `a_base_from_an_older_version_still_answers_every_query`, because the
  view reads them through `unwrap_or_default` and a mistyped table name
  therefore shows a confident zero rather than an error — which is
  exactly what happened (`bio_results` for `biology`).
  `src/content.rs` (the 772 printed phrases the officine may rewrite —
  see « Réécrire les phrases imprimées » in `docs/CONTENU.md` and the
  convention below. Pure, tested, no database: the table is read once and
  passed in),
  `src/selfcheck.rs` (the sheets a patient takes home: automesure
  tensionnelle, glycémie, poids, débit de pointe, INR, douleur. **What
  is missing from a photocopied grid is not the grid, it is the
  protocol** — a blood pressure taken after the coffee, standing, on
  whichever arm is free means nothing, and a glucose written up from
  memory in the evening means nothing either. Each sheet therefore
  carries four things and the grid is the fourth: how to measure, what
  to aim for, what not to wait on, and where to write. Two rules held by
  tests: **no invented figure** — where the target is individual (the
  glucose, the INR range, a personal best peak flow) the sheet says so
  and leaves the line blank rather than print a number the patient would
  take for theirs; and **nothing that replaces the prescriber** — no
  sheet adjusts a dose, every sheet says whom to telephone and when.
  Pure, tested, no clock),
  `src/caisse.rs` (counting the till: what is in the drawer at closing,
  what is left for tomorrow, and the gap against what the day should
  have taken. Three rules, one test each. **Money is counted in whole
  centimes, never in floats** — twelve ten-cent coins make 1,20 € and
  not 1.1999999999999997, and a centime is exactly what a till count
  exists to see; everything is `i64` and the conversion happens at the
  display. **A gap is not a correction**: the module states it, never
  resorbs it, and the count is never recomputed from the expected —
  the same discipline as the register of stupéfiants. And **with no
  expected takings there is no gap** — `gap` is `None` and the sheet
  leaves the line blank, rather than announcing « + 1 240,50 €
  d'excédent » every evening. Pure, tested, no clock; the day is
  passed. The counts are rows of `caisse_counts`, INSERT-only: a till
  recounted the same evening is a *second line*, and both are read.
  Which is exactly what the month has to reckon with: `per_day` keeps
  **one count per evening — the last written**, `superseded` names the
  others so the view can show them struck rather than hide them, and
  `summarize` adds up only what `per_day` kept. Adding both lines of a
  recounted evening makes a day with double the takings, and no monthly
  total ever shows it. The same summary says **over how many evenings**
  its cumulative gap is computed: a sum of gaps with no such number
  beside it reads as though it covered the month),
  `src/script.rs` (the console: what the officine can ask its own base
  in a few lines, without waiting for someone to write a screen for it.
  Two rules make it possible at all, and both are tested: the engine has
  **no I/O whatsoever** — no file, no network, no process — so the only
  path a script has to the world is the pane it prints into, which is
  why it is Rhai and not something else; and it **cannot write** —
  the console reads a snapshot taken before the run, and a script that
  could write to the register of stupéfiants would be a hole in the one
  place in this application that has none. And it stops: `while true {}`
  is three characters, so the engine is bounded in **operations** and
  not in seconds — a clock would make the same script pass on one post
  and fail on another. Pure, tested; the snapshot is passed in),
  `src/timeline.rs` (the file's thread: everything the base knows about
  one person, in the order of the days. Seven tabs each answer their own
  question and none answers the one you ask opening the file of somebody
  you have not seen for six months — *what happened, and when*. The last
  dispensing is behind one tab, the last vaccine behind another, the last
  laboratory result behind a third: each is one line, and it took three
  clicks to read three lines. The module knows none of the sources — the
  caller hands it `Event`s — and three rules decide it: **an undated line
  is not in a thread** (placing it « somewhere » dates it a day that is
  not its own), **the order is total** (day, then nature, then title, or
  two lines of the same day swap places between frames), and **a
  rendez-vous next week is not the last act** — which is the only reason
  `latest` knows what day it is. Pure, tested, writes nothing: the
  thread is a reading, so `Kind` has no database key),
  `src/agenda.rs` (the day's intervals: what overlaps, which lane it is
  drawn in, and what a filter is allowed to turn down. The day plan used
  to place its blocks by **hour bucket** — `offset / row_h`, then
  `index % 2` — so 9 h 00 and 9 h 45 sat side by side though they never
  meet, and the third entry of an hour repainted the first. Three rules:
  **touching bounds do not overlap** (an interval covers `[start, end[`,
  never the minute of its end), **a rendez-vous with no duration is not
  a rendez-vous of zero minutes** but a point in the day, and — the one
  that is the whole function — **a filter does not erase, it dims, and
  it does not dim what overlaps what it keeps**. Cacher l'entretien à
  distance de 14 h 00 – 14 h 30 quand une vaccination est posée à 14 h 15
  au comptoir *fabrique* le conflit qu'il devait montrer: the filtered
  entry stays drawn, in context. The module knows **neither egui, nor
  the base, nor the act kinds** — the view posts a `kept: bool` per
  entry — the same boundary as `timeline.rs`, and for the same reason:
  the day the team's shifts overlap, they go through this calculation
  untouched. Pure, tested, no clock),
  `src/planning.rs` (the team's shifts: who is there, when, how many
  hours that makes, and which slices leave the counter empty. « Poste »
  is a person's hours, not a workstation; the **garde** is a nature of
  shift and not another screen, and so are the **absences** — a leave is
  a shift that carries no hours, and it is precisely what *explains* a
  hole at the counter, so filing it elsewhere would separate the hole
  from its reason. It does not recompute overlap: `agenda.rs` knows it
  already, and **there are not two overlap calculations in this
  application**. Naming who is at the counter and counting them is **one** question:
  `who_is_in` returns the initials and `coverage` counts what it returns —
  two calculations would end up drawing three squares above two names.
  Six rules, one test each: **hours are counted in whole
  minutes** (7 h 35 is 455 — the centimes of the caisse, for the same
  reason), **a shift with no end is not a shift of zero hours** (`None`,
  and the line reads « — »), **a night is counted at the day it begins**
  (20 h → 2 h is `1200 → 1560`, six hours, whole, or whoever adds the
  columns counts it twice), **two shifts of the same person that touch
  are not a clash** (9 h–12 h 30 then 14 h–19 h 30 is a day cut at
  noon), and **a pause longer than the shift is refused, not
  subtracted** (a negative total propagates through the week unseen).
  A sixth carries the `Cadence` type: **« les semaines paires » is not
  « une semaine sur deux »** — one is read off the calendar, the other
  counted from the day it was set. They agree for years, then diverge
  *forever* at the first ISO year of 53 weeks (31 Dec 2026 is week 53,
  4 Jan 2027 is week 1: two odd weeks running). That is why the base
  stores a rhythm rather than a number of days, and why both parities
  step **seven** days and drop every other one. The daily rhythm is how
  a date range is written — « congé du 12 au 26 » is one stored row, not
  fifteen — and it is the only one that *requires* an end: without one
  it is not a leave, it is somebody absent forever. `Cadence` is the
  **only** vocabulary for « how often » here: the agenda's own entries
  (`events`) carry the same key and unfold through the same
  `db::stored_step` and `Cadence::accepte`. They had a second one — a
  number of days behind « Toutes les 2 semaines » — in the same screen,
  and it could not say « les semaines paires » at all. And a seventh that
  lives in the naming: `day_total` counts a
  **presence**, never a wage — no premium, no overtime, no collective
  agreement, no contractual week, and there will be none; that is why
  `heures_semaine` and the « Relevé d'heures » sheet were removed in
  0.185.0 rather than kept. A shift bound is read with
  `parse_bound` and not the day's clock, which stops at 23:59: a garde
  ends at « 26:00 ». Pure, tested, no clock. The **exception** — one
  occurrence of a pattern contradicted without erasing it — lives in the
  base (`supersedes`, `cancelled`) and is written from the planning
  row's « Ce jour seulement »; it had been complete from the schema to
  the undo since 0.175.0 and **unreachable from the screen** until
  0.185.0, which no test could show because each half worked),
  `src/graph.rs` (a card's neighbourhood as points on the unit circle:
  same molecule, same class, named in its interactions — pure, tested,
  no egui, so the view only scales and paints),
  `src/facets.rs` (what each card says about itself, **as data rather
  than sentences**: the plasma half-life in hours, and what the card
  *treats* or *alters*, by organ and by grade. The monographs answer
  « que sais-je de ce médicament » ; this answers the other half,
  « quels médicaments ont telle propriété », which no paragraph can. A
  facet is backed by what the fiche writes: where the monograph gives no
  number, the facet says « non chiffrée » rather than invent one, so a
  facet is corrected by correcting the fiche — two tests enforce that,
  one per side, on *different* vocabularies, because a lesion and an
  indication do not speak the same language. Two rules the whole module
  turns on: **the field a sentence comes from decides whether it is an
  impact** (« insuffisance rénale » in the contraindications is a kidney
  that governs the dose, in the adverse effects a kidney the drug
  harms — a drug cleared by the kidney is not a nephrotoxic drug), and
  **the grade means severity when harming but centrality when
  treating**, without which sixty antihypertensives would bury the six
  heart-failure drugs. Pure, tested, no egui; the index is built once,
  never in the draw loop),
  `src/classes.rs` (the therapeutic classes: what they are and which of
  sixteen families they sit under. The `class` field of a card is free
  text and it drifted — **495 distinct labels over 851 cards, 331 of
  them on a single card** — and the drift was not cosmetic: `anti-TNF`
  and `anti-TNF alpha` were two classes, so Humira's chip said seven
  neighbours instead of ten and Remicade was nowhere, with nothing
  visibly broken. Also `bêtabloquant`/`bêta-bloquant` (a hyphen) and
  `biphosphonate`/`bisphosphonate` (a letter). **The fix does not
  rewrite the cards**: a referential reads them, each canonical class
  carrying the labels actually met, and `canonical` folds them. A class
  the referential does not know stays readable rather than overwritten —
  the same rule as everywhere here. `same()` is what the neighbourhood
  and the chip compare on, never the raw string. Pure, tested, no egui;
  the index is built once. Four tests hold it: every shipped card's
  class resolves, a label names exactly one class, the three measured
  drifts fold, and the referential must carry **at least 112 fewer**
  classes than the base has labels — otherwise it would be the same list
  with one more column),
  `src/ordonnancier.rs` (the register of stupéfiants: the balance — which
  is **not** a sum, an inventory *sets* it, and which is **two numbers
  and not one**: what is dispensable, and what a patient brought back
  and is waiting to be destroyed. Putting a return back into the stock
  would announce forty available where there are twenty-six and a sealed
  bag; passing it as a loss would erase it, when the officine answers
  for it until the procès-verbal. Both are computed in one pass over the
  same sorted lines, and `apply` writes each nature's effect **once** —
  a cancellation walks it backwards rather than repeating it inverted,
  because a nature added to one and forgotten in the other is a stock
  that goes wrong silently —, the **cancellation**,
  which is the only correction there is (a bad line stays written and a
  further line names it and undoes exactly what it did — never the
  quantity that line carries, which is the one thing that may have been
  wrong), the dispensing number, the inventory gap, and what to go and
  count. Plus the **catalogue**: 106 presentations of the French market
  in 12 families, each with its dosage, its counting unit, the maximum
  prescription length in days and the rule its family carries. A rule
  table, not seeded content — the officine *picks* from it, because a
  base shipped with 106 followed products is 106 zero balances and a
  control list nobody opens again. A box size is **not** in it: 106
  packagings written down are 106 multiplications applied blind to every
  reception, right the day they are written and wrong the day a
  marketing authorisation holder repackages. The officine says it once,
  looking at the box — the same rule as a barcode, for the same reason.
  And `plan`, which reads a **sheet** rather than a line: the safe count
  is forty products, one date, one operator, and an ordonnance carrying
  two stupéfiants is two lines of one gesture. Five rules, one test each,
  and the first carries the rest: **an empty box is not a zero** — forty
  products of which six are counted would otherwise write thirty-four
  inventories at zero, emptying the safe on paper. An unreadable box is
  not a zero either (« 1O » with an O is refused and said so); zero is a
  figure only for an inventory; a gap and a procès-verbal are motivated
  **box by box**, because two products that are short are not short for
  the same reason. Each snag is named against its own box rather than
  after the refusal — on a sheet of ten, a refusal that does not name
  the offending line leaves you hunting for it. The writing itself is
  `Db::add_stup_moves`, one transaction: **the sheet goes whole or not
  at all**, since a half-written sheet is the worst state to leave a
  register in — three lines of five went through, nothing says which,
  and nothing there erases. Pure, tested, no clock: the day is passed
  in),
  `src/date.rs` (the calendar, written **once** — day arithmetic, the
  end of a month, the ISO weekday and the ISO week number, whose rule
  is one sentence: **a week belongs to the year of its Thursday**. It
  was written three
  times — `ordonnancier` by the julienne formula, `location` by the
  civil one, plus a separate ISO reader in `surveillance`. None was
  wrong, and that is exactly what this codebase refuses elsewhere. The
  civil pair is kept because it has an inverse, which reading a box's
  expiry needs. One test walks every day from 1900 to 2100),
  `src/vigilance.rs` (the three questions the register asks —
  rapprochement, several prescribers, dose escalation — and **what it
  cannot know**, written before anything else: no daily dose, no
  prescribed duration. The family's ceiling is used **only to fall
  silent**, never to infer a rate; under three prior deliveries the
  module says nothing. A question, never a verdict, and the type
  enforces it: no field to write a conclusion in, the accessor is
  `question_key`, a test demands the question mark, and a finding cites
  at least two register lines. Pure, tested, **ships no catalogue**),
  `src/codebar.rs` (CIP13 and the GS1 DataMatrix as a scanner types them
  — it is a keyboard, so no driver and no image. The check digit
  decides, never the length, like the NIR in `vitale.rs`. AI 01 being
  fixed-length and first, identifying the product never depends on the
  FNC1 separator: only the lot and expiry do, and a lot nothing closed
  says so. **No CIP table is shipped** — an unknown code waits for a
  human to name the product),
  `src/scans.rs` (the scanned pieces: what a file **is**, read in its
  bytes and never in its name, what a piece can be, and how the
  officine's own scanner is asked for one — pure, tested, reads no
  disk),
  `src/maintenance.rs` (the long passes over the base — synchroniser,
  compléter, réinitialiser — as named steps run on a thread of their own
  against a connection of their own). The
  dispositifs médicaux have no module: they are fiches in the base
  (`STARTER_DISPOSITIFS` in `src/db.rs`), like the codex.
  `src/location.rs` (what a rental of material owes and when its
  ordonnance runs out — pure, tested, no internal clock; the forfaits
  live in `[locations]` of config.toml and ship empty),
  `src/insulin.rs` (each insulin's action profile as a curve, and the
  500 / 1800 / titration rules — static, pure, tested),
  `src/release.rs` (what version this is, and — only on a button press,
  from Options › À propos — what GitHub says the newest release is; the
  **only** network request in the application, everything else hands a
  URL to the browser),
  `src/vitale.rs` (finding the beneficiaries on a carte Vitale — the NIR
  proves itself by its control key, so nothing is read at an offset
  anybody guessed; pure and tested apart from the one transmission
  function), `src/winscard.rs` (the PC/SC library, opened **by name at
  the moment a card is asked for and never linked against**: a binary
  linked to `libpcsclite` does not start at all on a post that has none,
  and most posts have none — which is also why no system package is
  needed to build this, in CI or on the release runners either)
- `launcher/` — `bpm-caddy-launcher`, auto-updates from GitHub Releases.
  It reads `[ui] theme` out of `config.toml` by hand and opens in that
  skin: it is the first window of the evening, and it does not depend on
  the application crate
- `motif/` — X/Motif theme for egui (palette, bevels, custom widgets)

Always build/lint with `--workspace`: plain `cargo build` only builds the
root package. CI enforces `cargo fmt --all --check`,
`cargo clippy --workspace --all-targets -- -D warnings` (the tests are
linted too), `cargo test --workspace`, `./scripts/coverage.sh`, and
`./scripts/smoke.sh` — which used to run only when somebody remembered,
and is the only guard the interface has.

**Measure before optimising, and write the measurement down.**
`the_first_launch_seeds_what_it_says_and_is_timed` (run it with
`cargo test the_first_launch -- --nocapture`) times the session's
opening and each read a view repeats. It is **not** ignored — an ignored
body is code nothing runs, and this is the only test that walks the
eight seeding passes of a first launch. What it *asserts* is the counts,
not the durations: a rewrite for speed breaks those silently, and a
timing that fails on a loaded machine is a guard people learn to skip.
It earned its keep the day it was written — a first launch cost four seconds, the intuition said « too
many SQL parses », and the intuition was wrong twice over: the cost was
`seed_conduite` scanning 862 cards with four `LIKE '%…%'` per rule
(3.7 s to 178 ms, by matching in Rust), and the first attempt at
`fill_starter_details` — column-major instead of card-major — made that
pass *slower* (789 to 674 ms, where card-major with prepared statements
gives 404). The three timings are written beside the code, so the next
intuition does not redo the detour.

`scripts/coverage.sh` holds two floors that only ever move up: the
**logic modules** and the workspace as a whole. The logic set is named
by what it *leaves out* — `app.rs`, `main.rs`, `winscard.rs` (a library
opened by name at run time, with no reader in CI), `motif` and the
launcher — and never by a list of what is in: that list had fallen four
modules behind, and `conciliation`, `surveillance`, `vitale` and `graph`
were logic nobody was counting. The workspace figure is low
because `src/app.rs` is ~15 000 lines of egui layout, more than half the
repo. But **a helper that measures or draws can be tested**: an
`egui::Context` runs headless in a plain `#[test]` — `Context::run`
draws, the fonts are there, and `ui.fonts(…)` measures in the face that
will paint. Two tests do it, and they are the model for the rest:
`an_invitation_too_long_for_its_field_is_shortened`, and
`a_wrapped_row_is_counted_as_it_is_drawn`, which compares what a band
*measures* to what it *draws*, at three text scales — the exact defect
that cut « Imprimer » off the carnet. `egui_kittest` (egui ≥ 0.30) would
add clicking and typing; it is not the price of entry.

## Conventions

- User-facing strings are French; code and comments are English. UI
  strings live in `assets/strings.fr.toml`, accessed via
  `strings::tr/trf/trn` (user override: `strings.toml` next to
  `config.toml`) — never hardcode a new UI string.
- Dates: stored ISO `YYYY-MM-DD`, displayed `JJ/MM/AAAA`
  (`db::parse_french_date` / `db::format_french_date`). Input accepts
  compact shorthand (`230826`, `2308`); `db::YearHint` picks how
  two-digit years expand (birth dates → past, RDV → 20xx).
- Schema changes go in `SCHEMA` **and** as an idempotent `ALTER TABLE` in
  `MIGRATIONS` (`src/db.rs`). This is now enforced rather than asked
  for: `tests/fixtures/schema-0.109.0.sql` is a photograph of the schema
  as that version shipped it, and
  `a_base_from_an_older_version_still_answers_every_query` creates a
  base from it, opens it, and runs every read the application makes.
  Forget the `ALTER` and every other test still passes — they all run on
  bases *this* version created, where `SCHEMA` put the column there
  anyway — and that one fails with « no such column ».
- Keep the Motif look: square corners, `motif::bevel` raised for
  buttons/panels, sunken for inputs/troughs; charts are painted by hand,
  no plotting library — `motif::chart` has bars, hbars, stacked,
  sparkline, `lines` (several series on one shared scale), meter, pips,
  heat strip and legend.
- **A widget that takes a type must honour it.** `motif::list_row` took
  a `RichText`, kept its string and threw the rest away: three call
  sites had been painting an overdue rendez-vous in `alert()` since the
  day they were written, and it came out in the ordinary ink. Its font
  was a hardcoded 14 px too, so lists never grew with `[ui] text_scale`.
  Both now come from egui's own layout and the style.
- **The application ships no font.** It draws with egui's own faces, and
  the proportional one has no arrows (U+2192 and friends) — the
  monospace one does. So an arrow may appear in a key chip and never in
  a sentence, and three strings were hollow boxes on screen for months
  because the chip beside them was right. Two tests in `src/strings.rs`
  pass every shipped character through the face that will draw it.
  **And a character no string carries still reaches the screen**: the
  thousands separator of `caisse::euros` was U+202F, the narrow no-break
  space French typography wants and the shipped face has no glyph for,
  so every four-figure amount drew « 1□240,50 » — invisible to a test
  that reads the strings file, because nothing writes that character
  down. The glyph test now passes the *output* of that function, not a
  constant; a formatted character is checked the same way a written one
  is.
- **A font size comes from `motif::pt`, never from a literal.**
  `RichText::size(11.0)` is a number of pixels: `[ui] text_scale` scales
  the `TextStyle` ladder, and a literal does not go through the ladder.
  Four hundred and seven of them meant that at 1,6 the panel captions,
  the counts, the register's dates and the label beside every figure
  stayed at eleven pixels while the buttons around them grew by half —
  the smallest text, the text someone who enlarges the type most needs
  enlarged, was the only text that did not move. `motif::pt(ui, 11.0)`
  is that eleven, scaled; `no_font_size_is_written_in_pixels` reads the
  text of `app.rs` and refuses the next literal. **And the measurement
  goes through the same function as the drawing** — `Self::widest` and
  the register's own templates still counted in pixels after the
  conversion, and the register elided its dates.
- **Colour comes from the theme, never from a literal.** `motif::bg()`,
  `text_dim()`, `accent()`… are functions over `motif::THEMES` (eight
  palettes, `[ui] theme`); a hard-coded `Color32::from_rgb` in the
  chrome is a colour that will look wrong on seven of the eight. A new
  palette must pass `every_palette_can_be_read`, and
  `no_colour_is_written_in_hex_outside_a_named_ramp` reads the text of
  `app.rs` and refuses the next literal — like the two that refuse a
  pixel size and a pixel-measured layout switch.
- **Two of the eight are dark, and that is a palette and nothing else:**
  no branch anywhere draws differently for a night skin. What it does
  change is that **every rule about colour is a distance, never a
  direction**. « L'encre est sombre » was true of six palettes and is
  false of two; « l'encre est loin du papier » is what was meant every
  time. `every_palette_can_be_read` is written that way now, and the
  two rules that stayed directional — the bevel and the hover tint —
  are directional in the *look*: a Motif widget is lit from the top
  left whatever the hour.
- **A colour written down for one ground is adapted before it is drawn
  on another.** `on_fill(fill)` is the ink a badge carries (black or
  white, decided by the fill actually painted — thirty call sites wrote
  `Color32::WHITE` by hand, correct only while every fill was dark).
  `data_ramp(ramp, AS_TEXT | AS_FILL)` fits a categorical set into the
  band the shell leaves it, **as a set**: moving each member as far as
  that member needs is what closes the gaps between them, and a lone
  badge is a ramp of one — there is deliberately no second function for
  it. Three transforms tried in order, each giving up something the one
  before kept: **scale** the three channels (keeps the hues saturated
  and spreads the ramp), **translate** (keeps the distances exactly, at
  some saturation) when scaling would pass `NO_GLARE`, **compress**
  only when the band is narrower than the ramp. A mix toward white is
  never one of them: it walks every hue to the same point, and the
  chart test caught two series landing within twenty-one of each other
  where the ramp's own rule is thirty-five. On the daylight palettes no
  member needs moving and the ramp is returned as written.
- **A categorical hue lives in one named `const` ramp**, and reaches the
  screen through `data_ramp`. Adding a colour is adding a line to that
  array — never a second array, and never a literal at the point of
  use. `motif::data_tones(c)` gives a set with more members than the ramp
  has colours its three tones — **chosen together**, inside that same
  band: computed one at a time they collide (a darker tone that hits the
  floor and turns round lands on its own lighter tone), and bounded by
  absolute limits rather than by the band they wash out against a
  daylight background. A bare `gamma_multiply(1.6)` did neither and
  clipped a dozen countries of the vaccine map to white. `motif::stripe()` is the zebra band,
  `motif::emphasize(c)` is bold where there is no bold face (further
  from the ground, not darker), `motif::readable_on(ink, surface)`
  keeps a sentence's own colour on a highlight unless the highlight has
  swallowed it.
- `./scripts/shot.sh <vue> <fichier> <taille> <échelle> theme=nuit`
  captures a view under a chosen skin — `theme=` is the one key that
  goes into `config.toml` rather than into `layout.toml`.
- **Layout is carved, not stacked.** A view computes rectangles with
  `motif::split_rows` / `split_columns` and fills them with
  `motif::panel` / `well` / `inside`; it does not centre a fixed-width
  column and let content run down the page. Measure against
  `motif::visible_rect(ui)`, never `available_rect_before_wrap` alone —
  a dock that grew past the width it reserved leaves the central view
  laid out wider than it is visible.
- A band whose height depends on its content (wrapped buttons, filters)
  measures it with `Self::wrapped_rows` and is **capped** — as a *share
  of the pane*, not at a constant — scrolling past its share rather than
  crowding out the panes under it. Measured-and-uncapped is the trap:
  the scans form asked for its eight rows and got them, and « Pièces au
  dossier » became a caption over nothing. Cap at half, and let both
  halves scroll.
- **When a pane is too short for everything in it, the garnish goes
  first.** The register's stock curve is dropped below a floor expressed
  in lines so the register's own *lines* survive; a chart kept at the
  price of the rows it illustrates is a chart of nothing.
- `./scripts/eyeball.sh [dir]` captures every view at 1024x700 with
  `text_scale = 1.25` into a directory. `smoke.sh` proves nothing
  panicked; it says nothing about a heading that wrapped, a button drawn
  half off a panel, or eight doors reflowing into three rows. Those are
  found by looking, and looking is only cheap when the pictures are one
  command away. Every layout
  must survive 1024x700 with both docks open — `scripts/smoke.sh` opens
  every view **three times**: at 1400x900, at 1024x700 with
  `text_scale = 1.25`, and at 1024x700 with `text_scale = 1.6`, which is
  where a computed floor crosses a computed cap. A screenshot at those
  sizes is the eye check the panics test cannot do.
- **And the shape that finds things is `eyeball.sh <dir> 1024x700 1.6`,
  looked at.** One pass over those fifty pictures found, in a single
  sitting: « Plan de prise » running off the band (a `horizontal` that
  cannot wrap) and the row it moved to uncounted, so the cross that
  removes an act fell off the bottom; a tab strip hiding half its tabs
  with nothing to say so; the conciliation showing **no divergence at
  all**; the explorer's doors and the agenda's legend each stopping in
  the middle of a row; and four hundred and seven font sizes that did
  not follow `[ui] text_scale`. None of the five was visible at 1,25,
  and none would ever have appeared in a panic.
- **Two unnamed `ScrollArea` in one view collide.** egui derives their id
  from position, gives both the same one, and paints « First use of
  ScrollArea ID … / Second use of … » in red across the screen. It does
  not panic: `smoke.sh` passed the explorer twice, in both shapes, with
  the banners on it — only the screenshot showed them. Any second
  scrolling region in a view takes an `.id_salt("…")`, like the
  `id_salt` already threaded through `motif`. And the moral is wider
  than the bug: capturing the pictures is not the eye check, **looking
  at them** is.
- A control that must stay visible under a widget that grows (a button
  under a text box) gets its **own carved row**, taken off the bottom
  with `split_rows` before the widget is drawn — not a height reserved
  in the flow above it. `add_sized` sizes the *text*, and a
  `TextEdit::multiline` adds its own frame margin on top, so a reserve
  computed from `available_height` is always a few pixels short and the
  button ends up half painted. `Self::button_height` is the button's
  real height; `interact_size.y` is not — at `text_scale = 1.25` it is
  27.5 px where a button is 38, so a band reserving rows with it and
  then drawing buttons is **ten pixels short per row**.
  `Self::row_height` is the number every band that carves rows uses.
- **A `Painter` paints where it is told — nothing clips it.**
  `motif::panel` laid its caption out with `layout_no_wrap` and no width
  limit, so « PATIENTS SOUS CE TRAITEMENT » on a narrow pane painted
  across the gutter and onto the next panel. Same family as the two
  below: the overflow breaks nothing, it just reads wrong. Any text laid
  out for a carved rectangle takes a `max_width` and, if it must stay on
  one line, `max_rows: 1` with an overflow character.
- **Shorten, don't elide — `motif`-adjacent rule, `richest_form` in
  `app.rs`.** « Lun 07/0… » has lost the month *and* reads broken, where
  « Lun 07 » does not say the month and reads whole; « CL YS ·… » makes
  it look as though a name is missing, where « CL YS » says exactly what
  it knows. Give the richest-to-poorest forms and take the first that
  fits; draw nothing when none does. And the elision that **must** never
  happen: a planning cell cut to « 14 h–… » is character-for-character
  what the grid writes for a shift whose end nobody noted — two
  different things under one appearance. That column is measured on what
  it carries.
- **A list row takes two lines when it can break cleanly, one when it
  cannot.** `list_row` and `list_row_pair` were one line with an
  ellipsis, and on a narrow dock that cost the information the row was
  there to carry: « Bain de bouche à la chlorhexidine » and « Bain de
  bouche au bicarbonate » both read « Bain de bouche … », and « Paul
  Bernard » read « Paul … ». Two lines fix that — but egui breaks a word
  wider than the column wherever it must (`break_anywhere: false` cannot
  save it), so « Benzodiazépines » came out « Benzodiazép / ines », which
  reads worse than the ellipsis. `motif::label_rows` measures the longest
  word against the column and allows the second line only when every word
  fits; the estimate uses the « 0 » template, wider than the average
  letter, so it errs toward the ellipsis rather than the break. And the
  secondary half is appended with a **real space**, not only
  `leading_space`, which is a gap in pixels and not a word boundary.
- **A row that wraps must be measured before it is allocated.**
  `motif::list_row_count` lays out its label first and sizes the row to
  the galley: allocating one line and painting two centres the text
  outside its own row and into its neighbours. And the figure is
  reserved *before* the label, because a count appended after it is the
  end of the line and therefore the first thing elision eats — a row
  reading « Cardiologie et vaisseaux · … » has lost the only thing it
  was there to say.
- **Measure with the width the drawing will use.** A band measured on
  `rect.width()` and drawn at `ui.available_width()` differ by the
  panel's own margin, and that was enough for the scans form to reserve
  two rows where it drew one — a hundred pixels held back on a pane with
  two hundred and fifty-five. Compute the widths once, above, and hand
  them to the drawing; two measurements of one thing always diverge.
- **A list of records needs a `Grid`, not a row of `horizontal`.** Drawn
  one row at a time, every column starts where the previous one ended,
  so nothing lines up between rows — and the defect grows with each
  record. `egui::Grid` with headers, inside a `ScrollArea::both` because
  a row that ends in buttons must be able to scroll to them.
- **And every cell of that `Grid` must announce its width.** A `Grid`
  column is as wide as its widest cell, so one cell that does not
  declare its share widens the column and pushes the next one out of the
  panel. `ui.scope` does not declare it — that is why `Self::grid_cell`
  allocates — and neither does a bare `ui.horizontal`: in the locations
  table the three buttons of the tight shape measured 480 px inside a
  440 px column, and « Renouvellement », the one thing that table exists
  to say, came out « Renouvellemer ». A row of buttons in a cell takes
  `allocate_ui_with_layout` **and** `with_main_wrap(true)`, so it wraps
  instead of shoving its neighbour.
- **A widget's height comes from the widget.** Three call sites carved
  24 or 28 px for a strip whose own rule is `font.size + 14` — thirty-
  eight at `text_scale = 1.6`. Two got away with painting over the panel
  below; the third, inside a `motif::inside`, drew its tabs cut through
  the frame. `motif::tab_strip_height(ui)` is that height, written where
  it is decided. The same for anything a `motif::` function sizes
  itself: ask it, never copy the number.
- **And writing the measurement in the right place is still not
  enough — confront it with the drawing.** `tab_strip_height` was
  written where the height is decided and was *still* wrong, because it
  was computed from memory rather than checked.
  `a_tab_strip_takes_the_height_it_announces` and
  `a_wrapped_band_is_as_tall_as_its_model_says` draw the real thing
  headless at three or four text scales and compare promise to
  occupancy. The second holds the arithmetic **every** cap in the
  application rests on — `n × row_height + (n−1) × item_spacing.y`,
  with `n` from `wrapped_rows` — which nothing checked, though the
  patient band, the drug card, the vaccine map and the scans form all
  depend on it. This is the cheap shape for the next one: a helper that
  measures, a headless draw, and an assertion in both directions.
- **Never subtract the layout's own gutter from a constant.** A width
  written as "the mark plus the air after it" has to give the air back,
  because `ui.horizontal` already inserts `item_spacing.x` — and that
  subtraction goes *negative* the moment the style scales past the
  constant (10 − 4 − 16 at `text_scale = 1.6`). A negative
  `ui.add_space` walks the cursor backwards and the next widget paints
  over the previous one. Derive the width instead
  (`4.0 + item_spacing.x`), let the layout supply the gutter, and have
  the measurement call the same function the drawing does — they then
  agree by construction rather than by arithmetic somebody has to keep
  correct. Found on the act colour mark, invisible at `text_scale = 1`
  because there the number happens to stay positive.
- **A bound that "doesn't bite" is usually the path, not the measure.**
  `egui::Label::new(LayoutJob)` **overwrites** the job's
  `wrap.max_width` with the ui's own wrap width and keeps only
  `max_rows`, so a width computed by the caller is thrown away — the
  label then paints as wide as it likes and the panel clips it
  silently. `motif::panel` lays its title's galley out itself
  (`ui.fonts(|f| f.layout_job(job))`) for exactly this reason, and
  `motif::section` now does the same. Three attempts at the section
  header were spent hunting a "correct" width that was correct all
  along.
- **Compare against the content, not the cursor.** A band's cursor
  advance includes one trailing `item_spacing.y` that belongs to the
  layout *after* it — `split_rows` already counts that gutter between
  its rows, so folding it into the height counts it twice. The tab
  strip test caught its own author this way: measured at the cursor,
  the function looked eight pixels short and « fixing » it over-
  reserved every strip in the app. Subtract the gutter, then compare.
- **A cap that cuts a row is worse than a cap that drops it** —
  `whole_rows`, already applied to the explorer's doors and the agenda's
  legend, and now to the patient band. But the patient band showed the
  limit of the helper: `whole_rows` always keeps one row, which is right
  for a band of doors where the first row *is* the content, and wrong
  where a measured head already carries the name. Forced to one row too
  many the band overran its cap by seventeen pixels and ate the biology
  table's only line. Below a head, the row count may fall to zero.
- **A band's floor can starve the pane it shares with.** The conciliation
  showed **no divergence at all** at 1024x700: the answer panel's head
  cost 136 px, the paste band claimed 110 as its floor, and one pixel
  was left for the table the tab exists to show. The fix was not the
  share but the head — a 250 px count label sitting in the wrapped
  button row forced it to two lines. Measure what a head *costs* before
  tuning what the panes get, and count the gutter in the reserve.
- **`ui.columns` does not clip.** Each column gets a rect and content
  wider than it paints straight into the neighbour: the two counter
  calculators drew « 1050 mg par prise » over « Clairance estimée ».
  Measure whether both fit, and stack them when they do not.
- `allocate_new_ui` only sets a max rect and egui paints through it: use
  `motif::inside` when content must not escape its frame. It also
  reserves **no space**, so a `ScrollArea` around it never learns the
  content is wider than the viewport and offers no bar.
- **Heights are the exception, and deliberately so.** `add_sized([w,
  24.0], TextEdit…)` is fine: egui raises a text field to
  `spacing.interact_size.y`, and `motif::apply_scale` scales *that* with
  `[ui] text_scale`. The literal is a floor the style overrides, not a
  size the style ignores — which is exactly what a width literal was.
  Don't "fix" the heights.
- **A text field's width is never written in pixels.** It is
  `chars_wide` (a width in characters of the body face), `field_width`
  (what its own hint needs), or a measurement taken above and handed
  down. A number does not follow `[ui] text_scale`, so the hint the
  field carries ends up cut the moment the text grows — and that shows
  on no screenshot taken at scale 1, which is every screenshot anyone
  takes. Twenty-nine of them had drifted (80 for one date, 96 for
  another, 300 for a numéro AM), and this is now a test that reads the
  text of `app.rs`: `no_text_field_is_measured_in_pixels`, verified by
  putting one back.
- **Measure, never guess a threshold.** Every layout bug found in the
  0.94–0.102 pass was a pixel constant standing in for a measurement:
  « narrower than 620 px » put the file's buttons across the patient's
  name at 645; a 360 px reserve for the agenda's title field had
  forgotten the width of the button after it; a 170 px floor under the
  notes journal was three of its own lines at text scale 1,25. Use
  `Self::button_width`, `Self::wrapped_rows(_of)`,
  `ui.text_style_height(&TextStyle::Body)` — and express floors in
  **lines**, so `[ui] text_scale` costs nothing.
- **And a layout-switch threshold in characters, for the same reason.**
  « Two columns above 1080 px » does not follow `[ui] text_scale`: at
  1,6 those same 1080 px carry two thirds of the text, and the view
  keeps two columns holding half a sentence each. Fourteen switches were
  written that way; they are `chars_wide(ui, n)` now, and
  `no_layout_switch_is_measured_in_pixels` reads the text of `app.rs`
  and refuses a new one — like `the_register_can_only_ever_be_written_to`
  refuses an `UPDATE`. Small numbers are still allowed: a guard floor
  (« this pane is too narrow for anything ») is not a switch.
- **A table whose columns are prose does not fold — it holds its first
  column instead.** The conversion tables are six columns of full
  sentences and scroll sideways, because a sentence does not fold into a
  footnote the way a lot number does. But the first column *names* the
  row, and « 20 mg » read without its drug is read for nothing: that
  column is allocated where the grid puts it (so the row keeps its
  height) and **painted at the viewport's left edge** — `frozen_left`
  for the x, `frozen_cell` for the band and the text position, measured
  once because the first version painted the band at the edge and the
  text with the grid, which is invisible until the bar is dragged.
- **When two panes cannot both fit, the one you type into wins.** A
  table missing a row still reads and scrolls; a form whose second row
  of fields is cut cannot be used. The same rule settled the carnet
  (form over table), the journal (add row over well) and the acts tab
  (table over journal — there the table *is* the subject).
- `f32::clamp` **panics** when min > max, and computed floors do cross
  computed caps on a short pane. Raise the cap to the floor
  (`cap.max(floor)`) rather than trusting one to sit above the other.
- Every layout must survive four things, not one: 1024x700, 1280x800,
  `[ui] text_scale = 1.25`, and both docks dragged wide (the docks cap
  against each other so the middle keeps `App::WORK_MIN`).
- **Nothing expensive on the frame.** A view is redrawn sixty times a
  second: a query, a fuzzy pass over the base or a cloned list in a draw
  path is that cost sixty times over. Memoise against the *question* and
  a revision counter the writer moves (`set_drugs`, `table_rev`), never
  against a timestamp; a list row holds an index, not a copy of the row.
  `Session::year_now` reads the year off the date already in hand rather
  than asking the base.
- **A panic is worse than a wrong pixel.** `partial_cmp().unwrap()` on
  computed geometry, `unwrap()` on state that « must » be open,
  `f32::clamp` with a computed min: all of them take the whole
  application down at the counter. Prefer `total_cmp`, an `if let`, and
  a cap raised to its floor.
- **The register of stupéfiants is inalterable.** `stup_moves` takes an
  `INSERT` and nothing else: no `UPDATE`, no `DELETE`, no
  `update_stup_move`, no `delete_stup_move`. A mistake is corrected by
  an opposite line that says why. That is what R. 5132-36 asks of a
  register and the only reason it proves anything, so it is a test and
  not a convention — `the_register_can_only_ever_be_written_to` reads
  the text of `db.rs` and refuses such a verb (verified by adding one).
  The dispensing number is assigned **inside** the inserting
  transaction, never by the caller: two PCs dispensing at once would ask
  for the same one.
- **A correction is a `Kind::Annulation` line that names the line it
  cancels**, carries a mandatory reason, and undoes exactly what that
  line did to the stock — read off the *cancelled* line, never off the
  cancelling one, whose own quantity is never looked at. That matters:
  the day you correct is the day the quantity was wrong, so a
  hand-typed opposite line gives back what somebody *believed* was
  taken. Cancelling an inventory restores its stored `expected`, which
  is the only reason that column is written rather than recomputed. An
  annulation cannot itself be cancelled, cannot be written twice
  (checked inside the transaction — two PCs would double the stock),
  cannot name a line of another product, and never takes a dispensing
  number: the cancelled delivery keeps its own, and the sequence goes
  on after it. A cancelled line stays on screen and on paper, struck
  through — a register with the mistakes removed proves nothing.
- **The register lives in its own encrypted file** —
  `<base>_stups.db`, same SQLCipher, same key. Not for size: ten years
  of register is a few hundred kilobytes. Because it is not the same
  kind of thing. The base is a working tool — reset, reseeded,
  compacted, copied from PC to PC; the register is an accounting record
  that R. 5132-36 asks to be kept ten years and that an inspection asks
  for *alone*, without the patient files. Consequences, and a test
  holds each: `change_password` rekeys **three** files now, « Copier la
  base… » copies three, the daily backup keeps as many copies of the
  register as of the larger of the other two, and a register written by
  an older version is **moved** across on first launch — ids preserved
  (a line names a file number and a product names a drug card:
  renumbering would make them point at someone else), marked in
  `seed_state` rather than deduced from « the file is empty », and the
  original rows are left where they are. One does not delete a register,
  even to file it elsewhere.
- **A scanned piece's format is read in its bytes, never in its name.**
  The application keeps it and later hands it back to the OS to open;
  accepting a file because it is called `.pdf` is agreeing to hand back
  something nobody looked at. Four magic numbers (PDF, PNG, JPEG, TIFF)
  and the rest is refused at the door.
- **The pieces' bytes live in their own encrypted file** —
  `<base>_scans.db` beside the base, same SQLCipher, same key. Measured
  before deciding: 200 B&W ordonnances of 250 KB took a 6 MB base to
  56 MB (89 % pieces), and `backups_keep = 14` made that 840 MB copied
  across the officine's share every morning. The base keeps each piece's
  **record** (label, kind, date, whom it belongs to) so a base copied
  alone still shows what existed. Consequences to keep in mind:
  `change_password` must rekey **both** files (a test catches it),
  « Copier la base… » copies both, `[scans] backups_keep` is separate
  and defaults to 2, and reading falls back to the legacy `bytes` column
  so a base from before the split still opens its pieces.
- **SQLite never shrinks a file.** Deleting 200 pieces frees pages and
  leaves the file at 56 MB. Only `VACUUM` gives the disk back — that is
  `Job::Compact`, which also moves any legacy bytes out and sweeps
  orphans, in that order (compacting first would rewrite the base *with*
  the pieces still in it).
- A line of a register carries the patient's **file number**, never the
  name: a register is printed and left on a counter, and what it must
  allow is going *back* to the patient, not displaying them.
- The database is shared between PCs: every write to a shared row
  (states, RDV dates, patient identity, deletions) is compare-and-set
  against the values the UI displayed (`WHERE … AND <old values>`,
  returning `bool`; `false` → reload + French notice). UI caches must
  be reloadable, and the team-notes file merges (`merge_team_notes`) —
  never blind last-writer-wins on shared data.
- **And the screen learns of another post's write without being asked.**
  The compare-and-set notices all fire *at the moment of writing*; until
  then a view showed whatever the base held when it opened, with nothing
  saying it had aged. `Session::sync_if_others_wrote` reads
  `PRAGMA data_version` every two seconds — it moves when *another*
  connection commits and stays put for what this one writes, which is
  exactly the question — and calls `resync` when it moves. Chosen over a
  hand-maintained revision counter because a counter has to be bumped in
  every write transaction, and the first one forgotten is a change no
  post ever sees; the pragma cannot be forgotten, including by a future
  version. It replaced a blind 60-second re-read. **`resync` reloads
  readings and never a typing buffer** — the lists and summaries, never
  the register form, the batch sheet, a cancellation reason or an open
  drug form; a resync that replaced what someone has their fingers on
  would be worse than the stale screen it fixes. Suspended while a
  maintenance pass runs: it has its own connection, so the witness sees
  it as another post.
- **What belongs to the officine goes in the base; what belongs to the
  post stays in `config.toml`.** That file is one per PC — the team
  declared at the counter did not exist in the back office, and the
  pharmacy's name was retyped on every machine. `[pharmacy]` (identity,
  signing pharmacist, AM number, team, opening hours) now lives in the
  `settings` table as its TOML fragment, read by `app::adopt_officine`
  at unlock and written by Options › Officine against the value the
  screen displayed. A base that has none yet is seeded from that post's
  file, so nobody retypes what they already wrote, and `config.toml`
  says it is only a seed. One key/value table rather than three tables
  because the section is edited as a unit in one dialog, is small, and
  already has an exact serde round-trip; three would have added joins
  and a second migration for `planning`, which keys on initials.
- **Each test gets its own temp directory.** They run in parallel in one
  process, so two tests sharing a `format!("bpm-caddy-x-{}", pid)` name
  delete each other's database and the loser fails on « disk I/O
  error », at the mercy of the scheduling. Three pairs had drifted into
  collision.

## Env hooks (demo / e2e / screenshots)

- `BPM_CADDY_DB=<path>` — database path override
- `BPM_CADDY_PASSWORD=<pw>` — unlock silently at startup
- `BPM_CADDY_NO_KEYRING=1` — skip the OS credential manager
- `BPM_CADDY_START_VIEW=dashboard|patient|drugs|drug_card|agenda|agenda_day|
  agenda_filtre|agenda_month|planning|protocols|protocol_open|template|options|about|tables|
  tables_search|calc|carnet|vaccins|bio|watch|revue|conciliation|
  vaccine_map|ordonnance|rein|grossesse|base|codex|
  codex_open|dispositifs|dispositif_open|locations|keys|vitale|
  act_picker|goto|goto_jump|mono_search|mono_patient|graph|registres|stup|
  trame|
  stup_catalogue|saisie|ordonnancier|vigilance|destruction|scans|
  textes|carnets_edit|
  patient_scans|fil|explorer|explorer_organ|classes|classes_outside|export|
  finances|stats|companion|script|carnets|caisse|caisses|peaux`
  — land on a specific view (screenshots, e2e). `about` is the Options
  dialog on its « À propos » page, `base` on « Base », and `peaux` on
  « Interface », where the eight skins are picked — each drawn in its
  own palette, which is the one thing only a screenshot can check.
  `caisse` opens the till count **with a drawer already counted**:
  fifteen lines at zero show neither the summary, nor the gap, nor the
  red it carries — that is, none of what the view exists to draw.
  Counting against expected takings is `[ui] caisse_expected`, true by
  default: with it off the field, the gap and the two history columns
  are **gone**, not filled with dashes — and the filter is applied once,
  where the month is read (`load_caisse_month`), never at each figure
  drawn. What is already stored is never rewritten: the expected figures
  of past evenings come back with their gaps the day the box is ticked
  again.
  `caisses` is its other page, the month: the demo seeds twenty-four
  evenings, one of them recounted and two with no expected takings,
  because those are the two cases the view has to know how to write.
  `planning_mois` is the planning read the other way: **one person's
  month** instead of the team's week, with the ISO week number and that
  week's total down a frozen first column — which is what makes « un
  samedi sur deux » legible, a column that lights up on even numbers and
  goes dark on odd ones. No single week can show that. A month cell
  carries the day number *and* the hours in the width a week cell gives
  the hours alone, so it spells them tight (« 9–12h30 ») through the
  same cell builder with a different hour writer — one construction, two
  spellings.
  A trame day carries **two half-days**: 9 h–12 h 30 then 14 h–19 h 30 is
  the ordinary French shape, and it is written as *two shifts* rather
  than one shift with a long pause — **a pause has no hour**, so
  `coverage` counts the person at the counter through their break and
  `gaps` sees none. Two shifts say where the hole is. The reader folds a
  second row of the same day (and the same nature) into the second half;
  two *different* natures on one day, or a third shift, stay
  « illisible » rather than lose one in silence.
  In `planning` the arrow keys walk the **grid** — days left/right,
  people up/down, rolling the week over at the edges — rather than
  stepping the week, which stays on the ‹ › buttons; the keys stand down
  for a focused field and for the open trame window.
  `trame` opens the week-pattern dialog **on what the base already
  holds** — Claire's canonical pattern, weekdays every week plus one
  Saturday in two, which is the only state where the two tabs, the
  parity sentence and the next-occurrence line say anything; empty, the
  window shows none of what it exists for. The dialog reads a person's
  stored patterns back (`frame_from_patterns`, pure and tested): **a
  weekly day is a day of both weeks**, in both directions — read onto
  both tabs, and written back once as `HEBDO` when the two tabs agree,
  so opening it and validating without a change rewrites nothing. What
  two weeks of seven days cannot hold is **said, not approximated**, and
  « Remplacer » stays unticked.
  `finances` is the recettes view, which has **no door**: it is in no
  dock, in no tab strip until it has been opened, and not even in the
  list the jump box offers on an empty query — it is reached by typing
  its name. That is deliberate (an « for me » screen offered on the
  fifth line of a menu is not one), and it is also why the smoke test
  has to open it by key: nothing else would.
- `BPM_CADDY_WINDOW=1280x1100` — open the window at that size
- `companion` (F9) is the window **shrunk to a bar and put on top**, not
  a second window: one field, one sentence, four buttons. `MinInnerSize`
  is sent with it, because `main.rs` gives the window a 960x640 floor and
  a window manager that honours it would make the companion 960 px wide.
  The size is deliberately *not* scaled by `[ui] text_scale`: a window is
  placed in a screen corner in pixels; what follows the scale is what it
  holds, which scrolls. And the layout record skips the companion's size,
  or the next session would open on a 460 px workspace
- `BPM_CADDY_DRUG_EDIT=1` — with `START_VIEW=drug_card`, land on the
  editable form rather than the monograph
- `BPM_CADDY_CARNET=<clé>` — with `START_VIEW=carnets`, open that
  self-monitoring sheet (`tension`, `glycemie`, `poids`, `souffle`,
  `inr`, `douleur`) rather than the widest one
- `BPM_CADDY_CARNET=<clé>` — with `START_VIEW=carnets`, open that
  self-monitoring sheet (`tension`, `glycemie`, `poids`, `souffle`,
  `inr`, `douleur`) rather than the widest one
- `BPM_CADDY_DRUG=<nom>` — with `START_VIEW=drug_card`, open that card
  rather than Eliquis (checking an insulin's action profile, say)
- `BPM_CADDY_KIN=dci|class` — with `START_VIEW=drug_card`, land with that
  neighbour list unfolded in the technical pane (its tallest shape)
- `BPM_CADDY_VITALE_DUMP=<path>` — replay a captured card instead of
  talking to a reader, so `START_VIEW=vitale` exercises the whole path
  (parsing, matching, the picker) with no hardware and nobody's identity

The workspace's shape — window size, dock widths, whether each dock is
open, the right pane's content, the view on screen — is remembered in
`layout.toml` beside `config.toml`, stamped with the version that wrote
it. The `[ui] show_*_on_start` options decide the *first* launch and
nothing after it, so a test that expects a particular starting shape
must use a throwaway `XDG_CONFIG_HOME` (both scripts already do).

The display settings that break layouts are not env hooks — they are
config: write `[ui] text_scale = 1.25` (or `density = "compact"`) into a
throwaway `XDG_CONFIG_HOME`, and `nav_width` / `docs_width` into
`layout.toml` beside it, to reproduce a wide-dock or large-text screen.
- `BPM_CADDY_SEED_DB=<path> cargo test seed_demo` — create a demo database
- `BPM_CADDY_TEST_PDF_OUT=<dir> cargo test pdf` — write the sample PDF

Xvfb parks the pointer at the centre of the screen, which on a screen
the size of the window is *inside* it: every capture came back with the
tooltip of whatever sat underneath. `screenshots.sh` opens a virtual
screen three times the window's width, keeps the window at its left
edge, and crops back — do the same in any new capture script.

Headless runs: `./scripts/screenshots.sh` regenerates the README
screenshots from a fresh demo seed, and `./scripts/smoke.sh` opens every
view in three shapes and fails on any panic — that is how the Ctrl+N crash
(nine digit keys for ten acts) was found. The second shape is the point:
`f32::clamp` panics when a computed floor crosses a computed cap, and
floors only cross caps on a short pane at large text. A deliberately
inverted clamp in the conciliation pane passes at 1400x900 and brings
the application down at 1024x700 with `text_scale = 1,25` — one pass
would have shipped it. All three shoot against a throwaway
`XDG_CONFIG_HOME`, never the operator's own config.

`./scripts/shot.sh <vue> [fichier] [taille] [échelle] [clé=valeur…]`
captures **one** view in a chosen shape — the same throwaway config, and
the extra `clé=valeur` pairs go into `layout.toml`, which is where the
workspace's own shape lives (`patient_band_folded=true`, dock widths).
That is the loop for correcting a band: change, capture, look. For manual runs:
`xvfb-run` + ImageMagick `import`, and **`unset WAYLAND_DISPLAY` inside
the xvfb shell** or the window opens on the real desktop instead.

## The official bulletins d'adhésion

`assets/bulletins/*.pdf` are the Assurance Maladie's own forms, byte for
byte as ameli.fr serves them — never regenerate, recompress or redraw
them. `src/bulletin.rs` writes their AcroForm fields and nothing else.
The five forms disagree about their field names (and `Adresse 1` means
the patient on three of them and the pharmacy on the other two), so the
mapping is an explicit table read off the rendered pages; the tests in
that module are what keep it honest. Consent boxes, the date and the
signatures are never pre-filled.

## Clinical content

`docs/CONTENU.md` is the map: where each kind of content lives, what
seeds it, what test holds it, and how to add to it. Read it before
adding a card, a posology, a table, a preparation, a protocol, an
analyte or a rule.

`src/ordonnance.rs` and `src/tables.rs` must agree: the antibiotics the
ordonnance offers are the ones the « Angine » and « Cystite » reference
tables list, and `every_molecule_appears_in_its_reference_table` fails
if they drift. Change a protocol in one place and change it in the
other. Nothing there is auto-selected and every posology is editable —
the app proposes, the pharmacist decides. It says so only if the
officine asks it to: every printed or displayed mention lives in
`[disclaimers]` (config.toml, Options › Mentions) and is empty by
default. Never hardcode a new caveat — add a key there.

A new prose field on a drug card is not searchable until it is in
`MONO_FIELDS` (`src/app.rs`) with a label key: « Dans le texte… » reads
that table and nothing else, so a field left out of it is a field nobody
will ever find by its words.

**Every printed phrase can be rewritten by the officine.** The cards,
preparations, dispositifs, protocols and reference-table *cells* were
always editable; 772 phrases were not, and they were exactly the ones
that **go out on paper** in the officine's name — the patient's carnets,
the entretien checklist, the « peut-on écraser ? » sheet, the
surveillance plan, the biology readings, the revue, grossesse, rein, the
TROD advice, the traveller. `src/content.rs` holds the mechanism,
**generalised from `table_cells`** rather than written beside it, and
`content_overrides` (in the base, so shared across posts) holds only
real differences. The full account is in `docs/CONTENU.md` under
« Réécrire les phrases imprimées »; the parts that bite:
  - A module exposes `phrases()` and `resolve()` and stays **static and
    pure** — its tests keep covering what ships, and resolution is a thin
    layer at drawing or printing time.
  - **An address comes from what identifies a rule**, never from its rank
    nor its prose: a revue point's title, a presentation's label, an
    analyte's code, a molecule plus its DFG threshold. Prose-derived
    addresses vanish when the prose is corrected — the rewrite does not
    go stale, it goes *unreachable* — and rank-derived ones stale twenty
    rewrites for one inserted rule. Each module has the test proving its
    identity unique; two rules sharing an address make the second inherit
    the first's rewrite, which on the crush sheet is a modified-release
    tablet crushed.
  - An override remembers the phrase it replaced and applies only while
    that phrase is unchanged; otherwise it is **shown for review**, never
    laid over a different sentence. Writing the shipped wording back
    deletes the row, so a restored phrase follows updates again.
  - Adding an editable document is `phrases()` + `resolve()` + one line
    in `content::documents()` + a label key + **the paired test in both
    directions**. A phrase missing from `phrases()` cannot be corrected;
    one missing from `resolve()` prints as shipped while you believe you
    fixed it, which is worse.
  - A phrase **composed at read time** needs its own path: the bilan's
    interval reading ends with the analyte's note, so rewriting that note
    changed the tooltip and left the printed bilan saying the old
    sentence.

The codex works the same way: `src/db.rs` ships `STARTER_PREPARATIONS`
into the `preparations` table once, and everything after that is the
team's — a formula rewritten in the app is never re-seeded over. Adding
a preparation is adding a fiche, never a second table in the code.

The ordonnance's adjuvants are **not** a list in the code: they are the
drug cards tagged `[ordonnance] adjuvant_tag` (default `probiotique`),
with that card's own posology lines as its schemas. Adding a product is
adding a fiche. Resist any pull to hard-code a second catalogue.

## Every printable document has an editable template

`pdf::DOCS` is the register: one entry per printable document, carrying
its key, the strings key of its name, its embedded Typst default and the
`{{MARKERS}}` it accepts. `pdf::fill` substitutes, `pdf::template_source`
reads the officine's own file when there is one, and Options › Modèles
iterates `DOCS` rather than matching on an enum — **adding a printable
document is adding one line to that array**, and it appears in the
editor. The four historical `[templates] *_path` keys still point where
they always did (an officine that wrote `bpm_layout.typ` a year ago must
keep printing with it); everything else lives in `[templates] dir`
(default `modeles/` beside `config.toml`) as `<clé>.typ`.

Four rules, each a test:

- **A document's declared markers and the ones its default template
  writes are the same list.** A marker declared and absent is a promise
  nothing keeps; one written and undeclared is a marker the editor never
  shows, so nobody uses it and the next rewrite of the template deletes
  it silently.
- **A filled template contains no `{{`.** A mistyped marker prints
  itself, in full, in the middle of the page — so `check_doc` names it
  instead, and the editor refuses to save.
- **Every default template compiles with its sample values**, which are
  never empty: a template validated on empty strings compiles and breaks
  on the first real printing. Those same values are the editor's preview,
  so preview and printing go through *one* function — two constructions
  of one page always diverge, and it is the preview that lies.
- **Every `pub fn open_*` of `pdf.rs` takes a `template_path`**, checked
  by reading the module's own text (`every_printable_document_takes_a_template`),
  with exactly one named exemption: `open_bulletin`, which is not Typst
  but the Assurance Maladie's own PDF with its form fields written (see
  `bulletin.rs`). Giving that one a "template" would mean redrawing it.

Some documents expose a marker per field (the ordonnance, the caisse, the
register); others — the monograph, the bilan, the codex — expose the
frame (`#set page`, `#set text`, the `#let sec` helper) plus one
`{{BODY}}`. That is the honest split: the frame is what an officine
edits, and a body whose structure is computed from a file cannot be
re-columned from a template.

## Releases

Push a `v*` tag → `.github/workflows/release.yml` builds app + launcher
for Linux/Windows/macOS and attaches them to a GitHub Release. Asset
names (`bpm-caddy-linux-x86_64`, `-windows-x86_64.exe`, `-macos-arm64`)
must stay in sync with `APP_ASSET` in `launcher/src/main.rs`. Bump all
three crate versions and move the `Unreleased` changelog section before
tagging.
