# BPM-Caddy — notes for Claude Code

Clinical pharmacy desktop app (Rust + egui), UI in French, proprietary
license with free public releases. Spec: `docs/SPECIFICATIONS.txt`.

## Workspace

- root `bpm-caddy` — the app: `src/app.rs` (UI/state), `src/db.rs`
  (SQLCipher storage), `src/fuzzy.rs` (search), `src/pdf.rs` (Typst),
  `src/config.rs` (config.toml — and `Role`, the seven officine
  qualities: the field stays free text because it is what prints at the
  foot of a document, and the enum is a **reading** of it, folding the
  labels people actually type the way `classes.rs` folds drifted class
  labels. `is_pharmacist` returns `Option<bool>`: an étudiant is *not* a
  pharmacist, and a hand-written quality answers neither yes nor no), `src/vaccines.rs` (calendrier vaccinal
  rules and the traveller's country table — static, pure, tested),
  `src/bulletin.rs` (fills the official Assurance Maladie bulletins
  d'adhésion in `assets/bulletins/`), `src/ordonnance.rs` (what a
  positive TROD allows, and the choices behind the ordonnance),
  `src/codex.rs` (reading a preparation's formula and rescaling it —
  pure and tested; the preparations themselves live in the base),
  `src/entretien.rs` (what each thematic covers, printed on the fiche),
  `src/biology.rs` (the analytes, their usual intervals, and the rules
  that read a value against the patient's treatments — static, pure,
  tested. `read` takes **whole treatments**, like every other clinical
  module, and not a flattened list of words: the class carries the
  route, and without it twenty-one local forms were explaining results
  they cannot cause — an indometacin collyre accounting for renal
  failure, an anaemia and an iron deficiency; four antifungal creams for
  a cholestasis; a lithium gel for a drifting TSH. They are filtered
  once, through `classes::is_local_form`. What that knowingly gives up
  is written beside it: a very potent dermocorticoid over a large area
  *can* suppress the adrenal axis, so « cortisol bas sous
  corticothérapie » did mean something for it — but the same rule fired
  on a 0,5 % hydrocortisone cream bought off the shelf, and a reading
  right one time in twenty-one is a reading people stop believing. Two rules it turns on. **A target is not an interval**: an
  HbA1c of 7,4 % is a failure in a recent diabetic and a good result in
  a frail eighty-year-old, and the software does not know which one is
  at the counter — so that analyte has *no* `high`, like the INR, and
  what is genuinely sayable is said by a rule (« au-dessus de 9 %, au
  delà de tous les objectifs »). Writing `high: 7.0` made the second
  patient read « haute », which is the exact écueil the « HbA1c »
  reference table names: intensifier parce que le chiffre dépasse 7,
  « ce serait faire du mal ». And **two rules never make one reading**:
  `read` runs the whole table rather than stopping at the first rule
  that answers, so a pair keyed on the same analyte, side and threshold
  both fire — four pairs had drifted that way, each correct alone.
  `two_rules_on_one_value_never_both_answer_for_one_box` refuses the
  next, on the boxes actually shipped and only when the two claim the
  same *molecule*: Xigduo carries metformine and dapagliflozine, so its
  collapsed bicarbonate is two readings and both are wanted),
  `src/renal.rs` (what renal function does to an ordonnance: a table of
  molecules, each with **steps** — a DFG, a level, what the RCP says —
  a source, and a `never`: what the row does *not* claim although its
  words catch it. The table is keyed on the molecule, so a **local form
  of a systemic molecule** falls into it — Exocine is an ofloxacin
  collyre, Lithioderm a lithium gluconate gel, and both were being told
  to adapt for renal function while their own fiches write « aucune
  adaptation posologique n'est nécessaire compte tenu de
  l'administration locale ». The old answer was to amputate the
  molecule from `needs`, and indométacine lost its renal row that way so
  that Indocollyre would not have one; a veto costs less. It also
  settles « ciprofloxacine » and « lévofloxacine », which *contain*
  « ofloxacine » — stated rather than left to row order, because an
  order moves and a veto does not. And `claims()` is written **once**,
  used by `read` and by both confrontation tests: they each had their
  own copy, so adding the veto left two tests checking a rule the
  counter no longer follows. `biology.rs` answers « this figure, under this
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
  `src/hepatic.rs` (the same question for the other organ, and the
  reason it is a separate module is one sentence: **le foie n'a pas de
  DFG**. The kidney gives a figure the pharmacy reads off a lab slip;
  the liver gives a **stage** a clinician assigns from five clinical
  items, two of which are not laboratory values. `read` therefore takes
  a `Stage` and never a value, and the panel offers three buttons rather
  than a field — a field invites a made-up number, and a table that
  computed a Child-Pugh from what a pharmacy can see would return a
  wrong score with a right score's confidence. The steps also read the
  other way: « from this stage up », where the kidney says « below this
  figure », which is why the two modules do not share a type. A hundred
  and ten molecules, each copied from the card that says it, cited. Six rules,
  one test each — the three that are not `renal`'s: **« rien à changer »
  is an answer** (a line the table knows and this stage does not reach
  says so instead of vanishing; oxazépam justifies it alone — its card
  says no adaptation is needed in mild-to-moderate impairment, and it is
  precisely the benzodiazepine one looks for in a cirrhotic), **an
  active liver disease is not a stage** (the statins are contraindicated
  in « affection hépatique évolutive » whatever the Child-Pugh, so they
  are *not* in the table and a test demands their absence), and **a
  figure in a conduct is a ceiling, never a posology** — `renal` refuses
  every milligram because a reduced dose also depends on indication,
  weight and age, whereas « ne pas dépasser 3 g de paracétamol par
  jour » depends on none of that, so the rule is refined rather than
  copied. One trap when adding a molecule: these cards write hepatic
  adaptation in the field named `renal`, which is really their
  « adaptation posologique » section. Pure, tested, no clock),
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
  `src/gravidity.rs` (pregnancy and breastfeeding as a level: thirty-six
  molecules, each with a `never` — the same veto as `renal.rs`, for the
  same reason: a collyre, a pommade or a gel of a systemic molecule
  falls into its row and wears a level written for the general route.
  Exocine, Lithioderm and Auréomycine Evans did, and all three fiches
  say the opposite *by contrast with* the general route.
  `a_local_form_does_not_wear_the_systemic_level` keys on the **class**
  and never on the prose — « passage systémique négligeable » is also in
  ISRS cards that « prudence » describes perfectly — and carries two
  named exemptions: the vasoconstricteurs row, which is written *for*
  nasal forms, and Sterdex, whose own card is cautious. The condition is
  « anything but compatible », not `worrying()`: that one lets
  `Prudence` through, which is exactly the level Exocine was reading,
  and the bite test is what showed it. And **two levels each** — they are two questions, and codeine is
  usable pregnant and contraindicated while nursing, the AVKs the exact
  reverse.
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
  `src/elderly.rs` (the third terrain, and the only one whose **figure
  is already in the file**: the birth date has been there since the
  fiche was created, nothing has to be typed — and that is exactly why
  nobody looks at it. The kidney asks you to fetch a lab slip, the liver
  to click a stage; age asks nothing. Meanwhile the shipped cards write
  « chez le sujet âgé » **889 times**, and reading it meant opening them
  one by one. The one sentence that separates it from its renal
  neighbour: **le rein change la dose, l'âge change le choix.** A renal
  conduct says « halve it »; an age conduct says « there is better » —
  which is why the row carries a field `renal` has not and cannot have,
  `instead`, and why it is **mandatory**: `crush.rs`'s rule, « a "no"
  with no alternative leaves the problem whole ». Twenty-six rows, each
  from a published list (Laroche 2007, STOPP/START v2, Beers 2023, HAS,
  ANSM), each confronted with the shipped fiche it lands on. Six rules,
  one test each — three of them not in `renal`: **an alternative is
  never itself a row to avoid** (the fault no re-reading catches, since
  each row is right alone and the pair sends you round in circles — it
  fired on the tricyclic row, which named amitriptyline), **an
  alternative is a choice and never a posology** (no milligram in
  `instead`; the risk may cite the reference's threshold, which is
  `hepatic`'s refinement), and **no row claims a card that never speaks
  of age** — which spoke at every row added, nine fiches in all: eight
  were completed on what age changes there, and one (Praxilène) had its
  row withdrawn instead, because the row was not about age at all. It
  reads the **clinical** fields, not the whole card: Noroxine named the
  elderly elsewhere and described everything but them where one looks.
  A fourth trap it could *not* see: a **clinically heterogeneous class**.
  « myorelaxant » catches the two lombalgia adjuvants the row meant and
  also Liorésal, Dantrium and Botox — the spasticity of multiple
  sclerosis — where « paracetamol, heat, move early » is absurd and an
  abrupt baclofen stop gives seizures. The cards do not *contradict* the
  row; they answer another question. Before trusting a class word, list
  the cards it catches **and group them by class**: it is the number of
  distinct classes, not of cards, that warns. Two levels
  and not three: the French list's « efficacité discutable » axis is
  deliberately absent, because a modest evidence base is a revue
  question, not an age one — the naftidrofuryl row was written, then
  withdrawn for exactly that. And it never says « stop »: stopping a
  psychotropic abruptly in an elderly patient exposes more than
  continuing it, which the panel writes in its own footer. And the reading
  goes out on paper, as a « Ce que l'âge change » section of the bilan
  partagé de médication — the bilan is *for* the polymedicated patient,
  i.e. almost always an elderly one, and it is the only sheet of this
  reading that leaves with them for the prescriber. Pure, tested, no
  clock: the age is passed in),
  `src/revue.rs` (what a set of treatments says about itself:
  doublons, associations, cascades — same shape, same discipline, and
  the same « two rules never make one reading » guard as `biology.rs`,
  since `review` likewise runs the whole table. Here the key is the
  rule's **shape** — the variant, the group count, the `min` of a
  `Duplicate` — which is what keeps « Deux benzodiazépines » apart from
  « Trois sédatifs »: same words, different `min`, and that difference
  *is* the rule. A single shared word is not a duplicate either, or
  tramadol would fold « Deux opioïdes faibles » into
  « Deux sérotoninergiques »; what folds two rules is one word list
  being **covered** by the other),
  `src/conciliation.rs` (the file's ordonnance against the one a patient
  brings back from hospital: reads a pasted list, matches each line to a
  fiche, and says what was stopped, changed, added or replaced — pure,
  tested, no catalogue of its own),
  `src/surveillance.rs` (what a treatment asks to have measured and how
  often, read against the dates already in the file — the other half of
  `biology.rs`: that one reads the values that are there, this one names
  the ones that are not. **A local form asks for nothing**: the plan is
  keyed on the molecule, so seven lines came out for five boxes that do
  not reach the blood — a lithiémie, a TSH and a DFG for Lithioderm, a
  gel for seborrhoeic dermatitis; a DFG for Indocollyre and Ikervis; a
  glycaemia for Dérinox; triglycerides for Differine — on a sheet that
  is printed and taken to the laboratory. Filtered once, at the top of
  `due`, through `classes::is_local_form`, never Watch by Watch: none
  of them is about a local form, and the day one is — a beta-blocker
  collyre does slow the heart — it is that one line to reopen, with a
  word in the rule rather than an omission),
  There is no `stats` module: the figures the « Statistiques » view
  shows are counts over lists the session already holds (the 862 cards,
  the summaries) plus four aggregate queries, and a module that only
  counted would be a module that only imports. What *is* worth writing
  down: the aggregate queries are covered by
  `a_base_from_an_older_version_still_answers_every_query`, because the
  view reads them through `unwrap_or_default` and a mistyped table name
  therefore shows a confident zero rather than an error — which is
  exactly what happened (`bio_results` for `biology`).
  `src/content.rs` (the 1004 printed phrases the officine may rewrite —
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
  **And a day's total is one calculation too** — `App::planning_day_sum`.
  The planning's footer had its own, and it disagreed about the same
  Thursday with **both** views that read `planning_digests` — the
  agenda's month grid and its week header — which leave the figure blank
  when a worked shift has no end. Two said nothing, one said « 10 h 30 »,
  and the lying one was the planning, the view where hours are counted;
  its figure also went to the printed sheet — where a partial total is
  worst, since the paper goes on a wall and says nothing of the missing
  end. The rule is *un total partiel n'est pas le total du jour*, and
  `a_partial_day_has_no_total` holds its three cases: two closed shifts
  sum, a third with no end erases the total, and an empty day or one
  carrying only a leave is not an uncertainty.
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
  no egui, so the view only scales and paints. The **seventh** reading
  keyed on the molecule, and the last to be swept: an AOD's interactions
  section names the azoles, meaning the *systemic* ones, so the map put
  Kétoderm — a shampoo — face to face with an anticoagulant, which is
  the very sentence `cyp.rs` has always used to refuse it. Only the
  `Interaction` tie filters: two cards of the same **molecule** are
  genuinely the same molecule whatever the route, and a local centre
  still cites the local forms it names. Finding it showed a hole in the
  filter: the fiches say « local » as often as « topique », and
  nineteen boxes escaped on that one word),
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
  text and it drifted — **495 distinct labels over 862 cards, 330 of
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
  count. Plus the **catalogue**: 158 presentations of the French market
  in 13 families, each with its dosage, its counting unit, the maximum
  prescription length in days and the rule its family carries. A rule
  table, not seeded content — the officine *picks* from it, because a
  base shipped with 158 followed products is 158 zero balances and a
  control list nobody opens again. A box size is **not** in it: 158
  packagings written down are 158 multiplications applied blind to every
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
- **And an egui widget that paints itself from `widgets.*.bg_fill` comes
  out invisible here.** `apply` sets that field to `bg()` for every
  widget state — right for a button, which is a raised panel on the
  panel — so anything egui draws as a *recess* has no recess. The one
  `egui::Slider` left in the application was exactly that: measured on a
  capture of Options › Interface, two hundred and thirty pixels of
  background along the middle of the control and two pixels of thumb
  edge, on all eight palettes. Nothing said where 0,8 was, where 1,6
  was, nor where one stood — on « Taille du texte », the setting
  somebody who cannot read the screen goes to first.
  `motif::scale_range` is the house scale (sunken groove, raised thumb)
  and `no_egui_slider_is_drawn_under_a_style_that_hides_it` refuses the
  next one. A `DragValue` is fine: it carries a `bg_stroke`, so it still
  reads as a box.
- **A widget that takes a type must honour it.** `motif::list_row` took
  a `RichText`, kept its string and threw the rest away: three call
  sites had been painting an overdue rendez-vous in `alert()` since the
  day they were written, and it came out in the ordinary ink. Its font
  was a hardcoded 14 px too, so lists never grew with `[ui] text_scale`.
  Both now come from egui's own layout and the style.
- **The route is a question every molecule-keyed table has to ask, and
  it is asked in one place.** `classes::is_local_form` reads a card's
  *class* — « collyre — AINS », « dermocorticoïde fort », « antifongique
  topique », « antifongique **local** », « anesthésique local — crème »
  — and every clinical table consults it, because all seven are
  keyed on the **molecule** and a local form of a systemic molecule
  falls straight into its row. Shipped instances found in one sweep on
  2026-09-13: seven printed surveillance lines (a lithiémie demanded for
  a seborrhoeic-dermatitis gel), twenty-one biology attributions (a
  collyre accounting for renal failure), three pregnancy levels, two
  renal conducts, one CYP crossing. The older remedies were amputation
  (indométacine lost its renal row so Indocollyre would not have one;
  kétoconazole left the CYP table for Kétoderm) and they cost a true
  row each time. Four mots are deliberately **out** of the vocabulary —
  « sous-cutané », « percutané », « patch », « gel » alone — because
  heparin, an estradiol gel, the Neupro and Duodopa all reach the blood.
  And « anesthésique local » is struck from the class *before* the words
  are looked for: it names a class and not a route, and the five in the
  base are injected — their antidote is the lipid emulsion, the antidote
  of a **systemic** toxicity, so one column of the referential was
  already contradicting the filter. Emla keeps its local status through
  « crème », which no other shipped card carries. Same category error as
  « localisé », struck for the same reason. A local form is not
  automatically harmless either: a beta-blocker collyre slows the heart,
  Sterdex keeps its « éviter » because its own card is cautious, and
  `renal`/`gravidity` carry a per-row `never` for exactly those
  judgements. Each module decides what to do with the answer: `cyp`
  files a local form under **unknown** rather than inert, because that
  module names what it does not rule on instead of handing out a
  clean bill of health.
- **The numbers this file asserts are held by a test.** `CLAUDE.md` and
  `docs/CONTENU.md` are read before every decision and they state counts
  as fact — so many cards, so many presentations, so many printable
  phrases. Being prose, they age in silence: on 2026-09-13 **six of ten
  checked had drifted**, one of them contradicted by the application's
  own status bar (851 cards against 862).
  `the_documentation_counts_what_the_code_holds` (in `strings.rs`) looks
  for each sentence **verbatim**, so rewording it fails the test too —
  deliberately: a guard that can no longer find its phrase and says
  nothing is a dead guard. Adding a count to these files means adding a
  line there.

  **And it reads the paper too.** The same day, the README — which only
  had three of its figures held — carried **six more that had drifted**,
  two of them contradicting the README itself (158 presentations at the
  start of a sentence and 106 at its end; 1 736 posology lines in one
  bullet and 1 319 in another). And the `mode d'emploi` that `pdf.rs`
  *prints* announced « six onglets » on a file that has seven, and
  recopied the shortcut list by hand, missing `F2`, `F9`,
  `Ctrl+Shift+Tab` and the quick-act digits. A list recopied by hand
  ages where nobody re-reads it, and that one is printed and left beside
  the post. `assets/aide.md` solved it by **generating** its lists off
  the registers; printed prose cannot without becoming unreadable, so it
  is **confronted** — which is why `PatientTab::ALL` and
  `app::key_rows` are hoisted out of the draw functions, and why the
  test's sources include `src/pdf.rs`.
- **A test without `#[test]` is a dead guard, and it dies silently** —
  the suite goes green with one test fewer and nobody reads the number.
  It happened here: inserting one lint directly above another swallowed
  the `#[test]` of `no_font_size_is_written_in_pixels`, four hundred
  pixel literals stopped being refused, and everything passed.
  `no_test_has_lost_its_attribute` (in `strings.rs`) reads thirteen
  modules and refuses a parameterless, returnless `fn` inside a `mod
  tests` that carries no attribute — helpers take an argument or return
  something, so they fall outside it.
- **A guard that reads the source covers the module nobody has written
  yet.** Five clinical tables each had their own « refuse markup » test;
  two tables had none, and that is exactly where the two faults were —
  a stupéfiants catalogue note drawn in a tooltip, and a surveillance
  phrase that **prints**. A sixth module would have had the same hole.
  `no_static_table_writes_markup_in_what_it_draws` (in `strings.rs`)
  reads the *text* of fifteen modules through `include_str!`, the way
  `no_font_size_is_written_in_pixels` reads `app.rs`, and refuses
  markup in a drawn field value. Comments and doc-comments write plenty
  of it, and should: they go nowhere. When a rule holds across modules,
  write the guard across modules.
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
  is. `app::help_bound` is the second producer, and it is why the
  no-break space in the help pane is the **ordinary** U+00A0 and not the
  thin U+202F that French typography wants: binding « ; » to the word
  before it stops « ; rien n'est envoyé » starting a wrapped line, and
  the thin one would have drawn a box doing it. That character is listed
  in the glyph test with its reason, since no string carries it either.
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
  conversion, and the register elided its dates. **And `motif` is held
  to the same rule by its own test**
  (`no_text_size_in_this_crate_is_written_in_pixels`), because the
  `app.rs` one reads only `app.rs` — the crate where sizes are actually
  decided was covered by nothing, and it carried one: the caption of a
  horizontal bar, clamped between ten and thirteen pixels. Note the two
  tests differ on purpose: the `app.rs` one refuses a digit **stuck to**
  the call, and that is why this one escaped it —
  `FontId::proportional((row_h * 0.46).clamp(10.0, 13.0))` starts with a
  bracket. The `motif` test reads the whole argument, brackets counted.
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
- **An override wins over the colour a widget picks for itself, so a
  hint reached the screen in the ink of a value** — every one of them.
  `motif::apply` sets `override_text_color` on the
  whole context, and egui reads that *before* the weak colour it means
  to draw a field's placeholder in. Measured on a capture of the trame:
  « 14h », the hint of an afternoon nobody entered, and « 14:00 », one
  actually entered, both at (1, 1, 1) on the same ground — the fields
  read « 9 h – 12 h 30, then 14 h – 19 h 30 », the total column said
  3 h 30, and the total was right. It is the trap this file already
  names for the planning grid — two different things under one
  appearance — one level up: there an elision, here a colour. Hints go
  through `motif::hint`, which sets the colour explicitly (an explicit
  colour passes in front of the override); `text_faint` is the right
  step and the only one `every_palette_can_be_read` already guarantees
  legible **in a trough** — the surface one types on — across the eight
  palettes. Not `RichText::weak()`, which tints toward the panel's fill
  when it is the *field's* fill one has to move away from.
  `a_hint_is_never_written_in_the_ink_of_a_value` reads the text of
  `app.rs` and refuses the next bare one.
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
- **A rule the panel exists to state is drawn before the content, never
  after it.** Anything after the content sits behind a floating bar, and
  nobody scrolls past the last line that speaks about the file. The rein,
  grossesse and foie panels all wrote theirs in the foot: at 1400x900 —
  *the widest of the four required shapes* — the kidney came out on
  « l'adaptation reste la décision » and lost « du prescripteur » off the
  frame; pregnancy showed the first of its five treatments and nothing of
  its foot, i.e. nothing of the « ce panneau ne remplace pas le CRAT »
  the module's own doc calls its rule; and the liver wrote « le foie n'a
  pas de DFG, ce panneau attend un stade » *after* the three buttons that
  sentence explains. The shape to use is the one `bio_cyp_pane` already
  had: **short on screen, whole on hover, and at the head** — four lines
  of reserves above the subject would be the garnish that eats it, one
  line is not. `a_clinical_pane_writes_its_caveat_before_what_it_qualifies`
  refuses the next, and the same sweep found that the *scope* line was
  missing entirely from the « Croisement » view — the view that exists
  for crossings. **One table read in two places must state its limits in
  both, or the place that stays silent is the one people believe.**
- **A capped band that scrolls hides its tail in silence, because
  egui's scroll bar floats.** `spacing.scroll.floating` is egui's
  default and nothing here changes it: the bar is invisible until the
  pointer is near it, so a band capped with `whole_rows` shows whole
  rows and *nothing at all* to say there are more. The template picker
  read « there are seven » on thirty-two documents; the explorer's axes
  read as eight organs on twelve, so neuro, peau, digestif and oreille
  did not exist. Say the total when the band is cut — the condition is
  `wrapped_band_height(...) > cap` — which is the rule the neighbourhood
  map already follows for the rings it could not take. Turning the bars
  solid instead would cost `bar_width` off every scrolling area, and
  twelve pixels is exactly what `App::scrolled_width` exists to account
  for: it would reflow rows all over the application.
- **`spacing.scroll.floating` is egui's default and nothing here changes
  it *globally* — but where the hidden tail carries a gesture or the
  subject itself, that one region turns it off.** Saying the total works
  for a band of doors (« 13 axes en tout »); it says nothing useful
  about a form, a dialog or a table of records, where what is under the
  fold is the control you came to use. 33 regions now set
  `ui.spacing_mut().scroll.floating = false`, each for a loss seen on a
  capture at 1024x700: the planning's entry row (« Poser »), the
  register's write form (its natures and nothing else — the quantity,
  the date and the prescriber are below), the file's acts table (the
  cross that removes an act), the file's identity band (« +
  médicament », « 2 interaction(s) », the whole revue d'ordonnance), the
  quick-act picker (nine acts of ten, on a screen whose printed guide
  promises the tenth digit), the shortcut window (eight of twenty-six),
  a checklist's items (one of five, each with its buttons), the Options
  dialog (the eight skins, which is what that page exists to show) and
  the protocol ordonnance (the adjuvant, the advice and the free
  lines — all of them printed) and a self-monitoring sheet, whose first
  of four parts stopped mid-word. The last three are the **home
  screen's** own panels — the day's rendez-vous, the files last opened,
  the notes of the day: at 1024x700 with `text_scale = 1,6` the first
  showed two of five, the second sliced, and every one of those lines is
  a gesture (it opens the file). It is the first screen of the day, and
  what it hid was who is coming. The two after them are the **tables of
  records whose rows end in a gesture** — the vaccination carnet
  (« Modifier », the cross) and the biology results (the cross that
  removes one): both lose to their form, which is the house arbitration,
  and both were then showing a row and a half, the second sliced through
  its buttons. Their captions do say the total (« — 3 dose(s) »,
  « (8) »); a total says how many there are, not that you can go and
  reach them. The last two are the till: the drawer, which is a
  form (fifteen denominations, seven visible at 1024x700, and nothing
  saying the five-cent line exists — on the screen whose whole function
  that is), and its summary, where the gap is written first because it
  is the answer, but a calculation nothing says is there is a
  calculation nobody checks. The thirty-third is the crossing's own
  composed list: at 1024x700 with `text_scale = 1,6` the control row
  takes two rows of a band capped at a third of the pane, leaving
  nothing for the chips — the screen showed « Depuis le dossier », a
  field, and **not one line of what is being crossed**, on the view that
  exists for exactly that.
  A solid bar **takes twelve pixels off the content**, so a region that
  measures its own rows does it with `App::scrolled_width`; converting
  one without that is how a row that used to fit starts wrapping. A
  `ScrollArea::both` pays twice **only when it really overflows
  sideways**: Options takes it happily up to `text_scale = 1,25` (no
  ribbon appears) and does show one at 1,6 — where its label column
  alone is six hundred pixels of prose and « Chemin de la base » adds a
  field and a button; the bar is solid and says so, which is the point
  of putting it there. The trame dialog does not — there it cost a column *and* a row, the totals came
  out « 8 h 3 » and the ribbon ate the very line it was meant to
  announce. Tried, looked at, taken back out. And do not reach for
  `ScrollBarVisibility::AlwaysVisible` on a floating bar instead: egui
  paints that handle from `widgets.*.bg_fill`, which `motif::apply` sets
  to `bg()`, so it is invisible even when shown — the same trap as every
  other recess egui draws for itself.
- **An empty state is content, and it is the first content anyone
  reads.** Every sweep here runs on the demo seed, so nothing had ever
  shown what an officine meets on its *first* launch — starter cards, no
  patient, no act, no register line. Point `BPM_CADDY_DB` at a new file,
  skip `seed_demo`, let the app seed itself, and shoot: four faults in
  ten views. « Aujourd'hui » answered with a centred « — » where its two
  neighbours write a sentence; the register's right-hand panes said
  « Choisissez un produit à gauche » beside a list saying there are
  none; « Prochains RDV » left its frame blank; the caisse month printed
  the *same* long sentence under both of its captions, which reads as a
  rendering fault. Three rules fall out: a vide is written in words, it
  says something **different** in each pane, and when the list it points
  at is empty it points somewhere else (« ouvrez « Catalogue… » »).
- **And the converse: a view the demo leaves empty is a view nobody has
  ever looked at.** A new panel needs its view key to land on a state
  where it *speaks*, or every capture ever taken of it shows « rien à
  signaler » — which is the honest answer and teaches nothing about the
  panel. « Vigilance » lived that way with three rules waiting. The age
  panel nearly repeated it twice in one evening: the default pick (the
  first file with an email) is sixty-eight, and « the oldest file » is
  seventy-nine **with no treatments at all**. The key picks the oldest
  file that carries an ordonnance, and the « Croisement » demo list
  gained a ninth drug so its age section has something to say — which
  is also what exposed the vanishing chip above. When you seed a state,
  seed its neighbours too.
- **A chart counts what does not fit instead of painting it outside.**
  `motif::chart::hbars` laid its rows one under the next without looking
  at the rectangle's height: past it they painted outside the frame and
  the panel clipped them — « Par type » showed seven acts of ten at
  1024x700, the seventh cut through, with nothing saying there were
  more. The last row that fits now says how many are missing (« … »
  and « +3 », two signs and a figure — this module writes no
  sentences) — and only when at least two rows are left for data: the
  drugs home's « Classes de la base » fits **one** row at 1024x700, and
  the count was taking the place of the only class one could read. The
  arithmetic is `chart::hbar_fit`, written apart to be testable, and it
  is computed with an epsilon: when the rows just fit, `row_h` is
  exactly `height / rows` and the f32 division comes back 2.9999998 —
  `a_chart_that_fits_hides_nothing` sweeps a hundred thousand
  (height, rows) pairs and about four thousand of them land on it.
- **When a pane is too short for everything in it, the garnish goes
  first.** The register's stock curve is dropped below a floor expressed
  in lines so the register's own *lines* survive; a chart kept at the
  price of the rows it illustrates is a chart of nothing.
- `./scripts/eyeball.sh [dir]` captures every view at 1024x700 with
  `text_scale = 1.25` into a directory. Its extra `clé=valeur` pairs go
  into `layout.toml` (dock widths), except **`vierge=1`**, which skips
  the demo seed: the application then seeds only what it ships — the
  862 cards, the preparations, the checklists — and nothing else. No
  file, no interview, no register line, no till count. That is **the
  first screen an officine sees**, and no capture script had ever shown
  it; the empty states are re-read nowhere else, and it is where a
  badly-provisioned band cuts the pane's only sentence. It found one
  the day it was written (`App::label_line`). It sweeps **the same list as
  `smoke.sh`**, and a test holds the two together: a view that smoke
  opens and eyeball never captures is a view nobody ever *looks* at —
  it does not panic, and that is all anyone knows about it. Three were
  in that state (the two caisse pages and the Vitale reader). `smoke.sh` proves nothing
  panicked; it says nothing about a heading that wrapped, a button drawn
  half off a panel, or eight doors reflowing into three rows. Those are
  found by looking, and looking is only cheap when the pictures are one
  command away. The three capture scripts share one configuration —
  `scripts/demo-config.sh`, `demo_config` for the officine and
  `demo_home` for the three throwaway XDG directories: `XDG_CONFIG_HOME`
  alone leaves « À propos » reading the *operator's* installed launcher
  version off `XDG_DATA_HOME`. Every layout
  must survive 1024x700 with both docks open — `scripts/smoke.sh` opens
  every view **four times**: at 1400x900, at 1024x700 with
  `text_scale = 1.25`, at 1024x700 with `text_scale = 1.6`, which is
  where a computed floor crosses a computed cap, and at 1024x700 with
  **both docks dragged wide** — the fourth of the required shapes, and
  the one no script produced until the sweep of 2026-09-13 showed a real
  defect living there (every register line was costing two rows, one of
  them empty). A screenshot at those sizes is the eye check the panics
  test cannot do.
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
- **A chip in a wrapping row does not break in two.** `horizontal_wrapped`
  wraps the **text inside** a label, not only between labels: « AOD
  potentialisé » ended one row on « AOD » and resumed « potentialisé »
  on the next, each half carrying the chip's coloured background — two
  chips where there is one. And since the band counts whole rows, the
  second came out sliced: at 1280x800 with both docks dragged wide, all
  that remained was an amber sliver under the label that nothing
  explained. `egui::Label::new(…).wrap_mode(TextWrapMode::Extend)` keeps
  the label whole and lets the *row* wrap, which is what it knows how to
  do. The same remedy was already written a few hundred lines away, with
  the same reasoning.
- **And two controls that only mean something together are one item.**
  The chip keeps itself whole; nothing kept the chip and the cross that
  removes it on the same row. egui wraps between two widgets without
  knowing they are a pair, so at `text_scale = 1,6` a file's row ended on
  « Glucophage » and the next began with *its* cross, sitting under
  « Coversyl » — the cross that removes one treatment read as the cross
  of the treatment above, and nothing said so. The same shape was in
  seven other places: « ‹ 15/09/2026 › » split across two rows of the
  caisse; « jusqu'au » in the planning, « Sujet » in a protocol, « posé
  le » in a location and « Ajouter » in the crossing, each separated from
  the field it names — a word that names a field placed on the line above
  names nothing; and worst, the safe count, which is a *calculation*:
  « boîtes × par boîte + vrac » broke between two of its five pieces, a
  « × » at the end of one row and its factor at the start of the next.
  `App::keep_together` allocates the group in one piece so it is the
  group, whole, that wraps; `App::group_width` is the width it takes —
  *n* items and *n−1* gutters, never *n* — and the band's measurement
  counts that same one number, because two writings of one width diverge.
  `a_group_kept_together_never_wraps_between_its_parts` draws the real
  thing headless at three scales over seven widths and refuses the next
  one — and it *bites*: the same loop without `keep_together` must break
  at least once, or the test guards nothing.
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
  was there to say. **And it stacks when the column is too narrow to
  share.** Reserved at the right, « 14 gélules » takes two thirds of a
  160 px dock — the followed-stupéfiants list at `text_scale = 1,6` —
  and what was left of the name ran one letter per row: « Mé / tha /
  don / e ». A figure saved at the price of a label in confetti has
  saved nothing, so below half the width the two go one above the
  other. Its ink is the **row's**, not a `dim` boolean: the same widget
  draws a class the base does not populate, an archived product and a
  product to go and count, and only the third is red. A list that
  composes « label · figure » into one string cannot use any of that,
  which is how the balance was being elided in the one list that exists
  to show it. **And the reserve belongs to the column, not to the
  row**: a row cannot see its neighbours, so deciding the arrangement
  row by row made « IPP · 9 lignes » sit beside its name and « Statines
  / 12 lignes » sit under it, in one list — one character of difference.
  The caller passes the widest figure of the list; a list whose figures
  are all the same width passes its own and nothing changes.
- **A column measured on a template is a column that lies.** The caisse
  history gave its five money columns the width of « -1 234,56 € », the
  widest amount conceivable, so the écart — twenty euros on a big
  evening — paid for a month's takings. Added up, those generosities
  overran the panel and the table scrolled sideways (floating bar,
  invisible): « +20,00 € » came out without its euro, in the only
  column that screen exists for. Measure what the column *carries* —
  and write the cell's text **once**, read by the measurement and by
  the drawing, or the two diverge and it is the measurement that lies.
  Same for « Par »: three letters of template, and an operator who
  signs « Claire » read « Cla… ». And the *heading* is a template too:
  the batch sheet's balance column was measured on « Au registre » and
  its three snag labels, so « 21 comprimés sublinguaux » — the Subutex,
  on the shipped sheet — came out « 21 comprimés sublingu… », the unit
  elided on the one column a register exists to carry. Its hand-written
  list of what the column can draw also **omitted one of the four snag
  labels** — « Zéro n'est pas un mouvement », the longest — so the snags
  are now read off the type (`Snag::ALL`, like `Enzyme::ALL` and
  `PatientTab::ALL`) rather than recopied, and a fifth enters the
  measurement on its own. It now measures the **agreed** unit the cell
  will write (the plural: « comprimé sublingual » measured singular
  leaves two letters out, and two letters are enough to elide) and
  provisions the figure in characters —
  composing forty balances to measure them would be forty `format!` a
  frame. Held by `the_batch_sheet_keeps_its_name_and_its_reason_at_every_scale`,
  which now walks the whole shipped catalogue's units rather than a
  chosen few: it is « comprimé sublingual » that decides, and it only
  shows up when you take them all.
- **A chip is measured with the cross it carries.** The « ce qu'on
  croise » band counted its rows on the drug's *name* and drew
  « Zeclar × ». With eight chips that fitted anyway it cost nothing; the
  ninth wrapped onto a row the band had not reserved and **the chip
  vanished** — neither sliced nor announced: the drug was on the map and
  in the crossings, and no longer in the list of what is being crossed.
  `App::ddi_chip` writes the label once and both sides read it, and
  `a_chip_is_measured_with_the_cross_it_carries` refuses the next.
- **Measure with the width the drawing will use.** A band measured on
  `rect.width()` and drawn at `ui.available_width()` differ by the
  panel's own margin, and that was enough for the scans form to reserve
  two rows where it drew one — a hundred pixels held back on a pane with
  two hundred and fifty-five. Compute the widths once, above, and hand
  them to the drawing; two measurements of one thing always diverge.
- **And a `ScrollArea` takes its bar off that width** —
  `App::scrolled_width`. A dozen pixels, and they are enough to send the
  last control of a measured row onto a second row that the cap then
  slices. Found twice before it got a name: the register's band, then
  the biology form, where the control that fell off was « Ajouter » —
  the gesture that records the result. The lesson is the one above, but
  the *path* is what differs, so the subtraction needs a name rather
  than a comment.
- **A head that carries a sentence measures it** — `App::head_height`,
  over `App::prose_height`. « Two rows plus eighteen pixels » is right
  at scale 1 and wrong at 1,6, where the sentence wraps to three lines
  and comes out cut through the middle of the last. Paired with
  `a_head_is_as_tall_as_the_prose_it_carries`, which draws the real
  thing headless at three scales and three widths.
- **Never add a fixed gutter allowance on top of whole rows.** The batch
  sheet reserved five gutters — those of everything it *can* carry — and
  when neither the write-reply nor the subtitle was drawn, those
  ninety-two pixels covered nothing and showed one more row, cut. A cap
  that lands on a whole row has to be counted **from where the rows
  actually begin**; anything above them is measured, not provisioned.
- **Two rects that merely touch `intersects` in egui.** Reserving the
  places a label may not go — the graph does it — a name laid against
  its own node box therefore rejected itself, and three names in ten
  vanished for a tangency. Shrink one side by a pixel before testing.
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
- **A row of labels is at least `interact_size.y` tall, whatever the
  font says** — `App::label_line` is that height, gutter included, and
  it is what a band provisions for a sentence it draws under itself.
  The carnets band counted `text_style_height(Body)` for « Aucun
  dossier ouvert : la feuille portera une ligne pour écrire le nom. » —
  seventeen pixels for twenty-seven — so the one line saying whose name
  the sheet will carry came out cut in half. Three more bands were
  reserving the same way for a message, and two of them reserve it
  **only when there is one**: a line kept for nothing is blank, and
  blank reads as an intention. egui never lays a row shorter than that, so a band that
  measures its reading lines with `text_style_height(&Body)` reserves 16
  px for something that occupies 22 — and the six missing are the
  descenders of the last line. The register's head did exactly that, and
  what came out cut through the middle was « 12 comprimés au registre ·
  non inventorié · 12 comprimés à détruire », i.e. the balance, which is
  what one opens that screen for. Take `line.max(interact_size.y)`, the
  way `Self::row_height` already takes the larger of the two. And count
  the gutters **between** everything a band stacks, controls and lines
  alike: *n* things cost their heights plus *n − 1* spacings.
- **A widget's height comes from the widget.** Three call sites carved
  24 or 28 px for a strip whose own rule is `font.size + 14` — thirty-
  eight at `text_scale = 1.6`. Two got away with painting over the panel
  below; the third, inside a `motif::inside`, drew its tabs cut through
  the frame. `motif::tab_strip_height(ui)` is that height, written where
  it is decided. The same for anything a `motif::` function sizes
  itself: ask it, never copy the number.
  `motif::panel_chrome(ui, titled)` is the second one: two views were
  counting a panel's inset title, its rule and its margins by hand —
  « 44 px » in one, « body height plus 26 » in the other — and neither
  followed `[ui] text_scale`, though the caption does. At scale 1 the
  first reserved six pixels for nothing; from 1.4 it was short, which on
  a band already capped is one row of lenses gone.
- **And a margin you did not measure is a margin that does not exist.**
  The planning's entry band was « whole rows **plus fourteen pixels** »,
  a padding believed to be `motif::inside`'s — which takes none: it
  hands over the whole rect. Those fourteen pixels were therefore the
  *top of the next row*, so the band ended on buttons sliced through
  their height — the exact thing `whole_rows` exists to prevent — and
  they cost the grid the line where the red says a hole remains during
  opening hours. Before adding a constant to a measured height, find
  the function that spends it; if there is none, there is nothing to
  add.
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
- **A measured list and a drawn row are two writings of one thing, and
  nothing in the type holds them together.** `title_band_height` is
  handed the widths of the row's controls and answers how many rows it
  will wrap to; a control added to the row and forgotten in the list
  makes the band announce two rows where it draws three, and the
  subtitle's last line falls outside the rectangle — cut through a
  word, with nothing to say so. Three of the nine bands were lying:
  the caisse (« Modèle… »), the carnets (their own heading) and the
  caisse history (ten pixels of `add_space`). What vanished on the
  caisse was half the sentence saying a gap is noted and **not** fixed
  by changing the count — the rule of that screen.
  `every_control_a_title_band_draws_is_measured_with_it` reads the text
  of `app.rs` and refuses the next one; and measure **the phrase that
  is drawn**, not a shorter sibling of it (the classes band measured a
  54-character subtitle and drew a 118-character one).
- **Both directions, and the counter's own width among the samples.**
  `a_title_band_is_as_tall_as_what_it_holds` had the right shape and
  still missed a 70 px hole, twice over: it asserted only `band >=
  drawn` — the direction that *cuts* — and an over-reserve cuts nothing,
  it makes blank, which reads as an intention rather than a fault; and
  its three widths (420, 700, 1100) jumped clean over the window where
  the fault lived. That window was about forty pixels wide, because a
  row only tips when the last control lands within the missing margin of
  the edge. It contained 640, which is what a 1024 screen leaves between
  two open docks — the width almost every counter runs at. Sample the
  shapes people actually use, not round numbers, and assert the slack as
  well as the shortfall.
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
  This is now a test — `no_constant_ceiling_trusts_a_computed_floor`
  reads the text of `app.rs` and refuses `clamp(<computed>, <literal>)`,
  like the three lints beside it. It found three, and one of them was
  two pixels from the edge at `text_scale = 1.8` (238 against a 240
  cap). Another measured a hint the officine **rewrites** in
  « Libellés » — a long enough label and the application falls over at
  the counter. A floor here grows with the text scale, always: row
  height, line height, the width of a prompt.
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
- **The haystack a rule matches is not quite the one the tests build.**
  At run time a `revue::Treatment` carries the card's name, DCI, class
  and the officine's own free `tags` — which are **empty on every
  shipped card**, since `INSERT INTO drugs` seeds only `(name, dci,
  class, antidote)`. The confrontation tests build theirs from the
  `STARTER_DRUGS` 4-tuple, whose fourth field is the **antidote**. So a
  rule reachable in a test through « naloxone » or « pyridoxine » would
  be dead at the counter. None is today — the only rule naming an
  antidote word is the isoniazid `Without`, and it names it as what must
  be *absent*, which is why an officine writing « pyridoxine à
  associer » on the Rimifon card used to switch that rule off.
- **A `needs` fragment is a substring, and a substring lives inside
  other words.** Every clinical table (`renal`, `gravidity`, `crush`,
  `cyp`, `hepatic`, `biology`, `surveillance`) matches by
  `contains_folded`, which folds case and accents but keeps spaces — so
  the danger is not words running together, it is a short fragment
  sitting inside a longer one: « gr**ipp**e » contains « ipp » (Tamiflu
  read as a PPI), « dim**éth**ylfumarate » contains « imeth » (Skilarence
  read as methotrexate, in three modules at once — including
  « tératogène et abortif » on the pregnancy panel, and **no shipped card
  is even named Imeth**), « cholécalci**fér**ol » and « inter**fér**on »
  contain « fer », « anti**sep**tique » contains « SEP ».
  **Name the molecules; never the class abbreviation** — and when a
  class word is genuinely wanted (« AVK », « AOD », « IEC » match exactly
  the right cards, and a hand-typed card often has a class and no DCI),
  check what it catches first. Before trusting any fragment, list every
  card in `STARTER_DRUGS` whose folded text contains it and read the
  list: a table can be perfectly consistent with itself and still be
  wrong about a card, which is why only the *encounter* with the shipped
  fiches finds this.
- **And a backing test must judge every card a row catches — against
  the row that actually claims it.** Checking only the first card
  validates the row on the product it meant and lends its claims,
  silently, to everything it catches in passing: that is how
  « trimipramine » wore « imipramine »'s CYP profile, and how
  « desloratadine » would have worn « loratadine »'s hepatic conduct.
  The right shape is: for each shipped card, find the **first** row that
  matches it (what `of`/`read` will return) and check that row against
  that card. `cyp.rs` and `hepatic.rs` do it; correcting a fiche is
  usually the right answer, not deleting the row.
- **These tables are keyed on the molecule, not the presentation.** So a
  topical form of a systemic molecule is left out rather than warned
  about — an azithromycin collyre is not an azithromycin, an
  indometacin collyre is not an NSAID at the counter. `crush.rs` is the
  exception and keys on the presentation, because there the box decides.
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
  collision. `no_two_temporary_directories_share_a_name` (in
  `strings.rs`) now refuses the next: it reads seven modules for the
  literal template — the pid is the same for every test and separates
  nothing — and there are 88 templates for 88 sites. The failure it
  prevents is the worst kind, one that does not come back when you go
  looking for it.

## Env hooks (demo / e2e / screenshots)

- `BPM_CADDY_DB=<path>` — database path override
- `BPM_CADDY_PASSWORD=<pw>` — unlock silently at startup
- `BPM_CADDY_NO_KEYRING=1` — skip the OS credential manager
- `BPM_CADDY_START_VIEW=verrou|search|dashboard|patient|patient_edit|patient_new|drugs|drug_card|agenda|agenda_day|
  agenda_filtre|agenda_month|planning|planning_mois|protocols|protocol_open|template|options|about|tables|
  tables_search|calc|carnet|vaccins|bio|watch|revue|conciliation|
  vaccine_map|ordonnance|rein|grossesse|age|cyp|ddi|libelles|listes|base|codex|
  codex_open|dispositifs|dispositif_open|locations|keys|vitale|
  act_picker|goto|goto_jump|mono_search|mono_patient|graph|registres|stup|
  trame|
  stup_catalogue|saisie|ordonnancier|vigilance|destruction|scans|
  textes|carnets_edit|
  patient_scans|fil|explorer|explorer_organ|classes|classes_outside|export|
  finances|stats|companion|script|carnets|caisse|caisses|peaux|aide`
  — land on a specific view (screenshots, e2e). `aide` is not a view at
  all: it opens the right-hand dock on its « Aide » tab, which is in no
  tab strip and therefore reachable no other way.
  `about` is the Options
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
  a second window: one field, one sentence, four buttons. Opened by its
  view key it now takes that size too — it used to be a flag and
  nothing else, so `smoke.sh` and `eyeball.sh` looked at the companion
  inside a 1024-pixel window, which is the one shape it never has in
  use: the window whose whole point is being small was the only one
  never looked at small. At 460×300 the card never fits, so its pane
  carries a solid bar. `MinInnerSize`
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
view in four shapes and fails on any panic — that is how the Ctrl+N crash
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
always editable; 1004 phrases were not, and they were exactly the ones
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
    fixed it, which is worse. Two of the eleven sources had no such test
    — the TROD ordonnance and the traveller's advice — and both were
    correctly wired: what was missing was the proof, and the proof is
    what is missing again the day someone moves the resolution.
    `every_rewritable_document_has_its_paired_test` (in `strings.rs`)
    now refuses the twelfth, and checks the module count against the
    registry so the test cannot quietly stop reading a source.
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

## L'aide est un volet, et ce qu'elle énumère est lu

`assets/aide.md` is the prose — French, markdown, cut at its
first-level headings by `help_sections()` (memoised in a `OnceLock`: the
pane is redrawn sixty times a second). The section is the unit the
search keeps or drops, because a manual with every other sentence
missing does not read, and `the_manual_is_cut_into_named_sections`
refuses a duplicate title and any line that falls outside a section —
prose written before the first heading would belong to none and never
appear, with nothing saying so.

Everything that is a **list** is read off the register that already
holds it: `pdf::DOCS`, `content::documents()`, `MONO_FIELDS`,
`script::API`, `script::LIMITS`. A manual that recopied one of them
would be wrong at the first line added elsewhere — and wrong in
silence, since nobody re-reads a manual to check that it has aged.
`the_generated_sections_are_read_off_their_registers` holds the other
end. Note the two registers disagree on purpose: `pdf::DOCS` carries a
strings **key**, `content::documents()` a resolved **label**; each is
read as it is written.

The console's API is described **beside the `register_fn` calls** that
create it, in `src/script.rs`, and
`every_call_the_console_offers_is_described` reads the module's own
text, collects the names actually registered and refuses a drift in
either direction — plus it runs every example against the test
snapshot, because an example that fails is worse than no example: it is
tried before it is read. « Essayer » places the example in the console
**and runs it**.

The manual's own typography is bound before it is drawn
(`help_bound`): the pane wraps at two hundred pixels, and French double
punctuation left loose starts lines with « ; ». The section list, the
recollected paragraphs and the search's folded key are all computed in
that same `OnceLock` — three passes over six kilobytes, and the pane is
redrawn sixty times a second.

And the manual goes through the same glyph test as the strings file
(`every_symbol_the_code_draws_has_a_glyph_in_the_face_that_draws_it`):
it is text that reaches the screen without being in
`assets/strings.fr.toml`, and the application ships no font.

## Releases

Push a `v*` tag → `.github/workflows/release.yml` builds app + launcher
for Linux/Windows/macOS and attaches them to a GitHub Release. Asset
names (`bpm-caddy-linux-x86_64`, `-windows-x86_64.exe`, `-macos-arm64`)
must stay in sync with `APP_ASSET` in `launcher/src/main.rs`. Bump all
three crate versions and move the `Unreleased` changelog section before
tagging.
