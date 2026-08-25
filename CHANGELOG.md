# Changelog

All notable changes to BPM-Caddy will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.42.0] - 2026-08-25

### Fixed
- The editable drug card was unusable. A multiline field grows with what
  it holds and ignores the height it is given, so now that every card
  carries a full monograph, one field ran down over the rows beneath it
  and drew across their labels. Each field is a box of its own height
  now, with the text scrolling inside it.
- The card's form no longer forces two columns into a narrow window: it
  puts one half under the other below 720 px, where two columns left
  five words to a line.

### Changed
- The entretien table is one aligned line per act. The act code and the
  step it pays move to a column of their own — "BMI · 2", with the full
  step name, the amount, the year and the coverage in the tooltip — and
  the two flags that change what is billed sit beside it as TPH and Δ.
  Before, the step name wrapped over three lines and pushed the flags
  out of their row, so no column lined up with its heading. Rows are
  striped.
- The agenda's week grid is as tall as its busiest day rather than a
  fixed height, so entries stop hiding behind a "+N" while there is room
  on screen for them, and an entry too long for its column ends in an
  ellipsis instead of being cut mid-letter.

## [0.41.0] - 2026-08-25

### Fixed
- The text scale had no effect on any Motif button: they were drawn at a
  hardcoded 14 px while the rest of the interface grew around them, and
  their padding ignored the compact density too. Both now come from the
  style.
- Opening the options twice enlarged the text twice: the scale was
  multiplied onto whatever size was already set instead of being applied
  to the base ladder.
- Secondary labels were painted in the bevel shadow colour, which is
  meant for a two-pixel edge and is far too light to read a word in.
  They now have colours of their own, and so do the agenda's hour
  column, its weekday heads, the half-life axis and the days outside the
  displayed month.
- List rows kept a fixed height whatever the density, so compact mode
  saved nothing in a list.

### Changed
- A deliberate type scale replaces the egui defaults: heading, body,
  button, small and monospace, all moving together with the text scale.
- Buttons that hold a state — agenda mode and filters, template target,
  situation, "à distance", "changement de traitement" — are one widget
  now, raised when off and sunken when on, instead of five hand-rolled
  copies of the same idiom.
- The dashboard reads at a glance: each figure sits under a small
  spaced caption and a hairline, and the per-theme counts are chips
  (sunken when the theme has entretiens, quiet when it has none)
  instead of one long line that broke between a label and its count.
- The patient card puts the acts where the work is: identity,
  treatments, entretiens, then the follow-up journal underneath — the
  reason to open a fiche is the entretien in progress, not last week's
  note.
- The agenda's act filters carry each act's colour and the same
  raised/sunken idiom as the rest of the interface.

## [0.40.0] - 2026-08-25

### Fixed
- Drug editing. Three things stood in the way and all three are gone.
  The team pane could grow wider than the width it had reserved, which
  left the whole central view laid out wider than it was visible and cut
  its right edge away — the drug card's buttons among it. The card's
  actions sat at the very bottom of the page, so on a full monograph
  "Modifier" and "Enregistrer" were several screens down. And a field
  the team cleared on purpose was refilled from the reference data by
  the next "Compléter les médicaments de départ".
- A field the team writes to is now theirs: the top-up fills only what
  they have never touched, and a field they emptied stays empty.
- The card's actions are a bar at the top, above the scroll, wrapping to
  a second line rather than running off the edge, with the deletion set
  apart from the rest.
- Any centred column now lays out inside the part of the panel that is
  actually on screen, so no view can be clipped that way again.
- The team pane's three tabs are short enough to fit it, and wrap if
  they do not.

## [0.39.0] - 2026-08-25

### Added
- The drug base is complete: all 812 cards carry a full monograph —
  indications, mechanism, posology, contraindications, interactions,
  adverse effects, surveillance, counter advice, half-life, elimination,
  renal adaptation, pregnancy and sources. The last 155 close the
  remaining gaps: contraception, opioid substitution, local
  anaesthetics, hypnotics and benzodiazepines, dermocorticoids, vitamin
  D and bone, antiretrovirals and hepatitis C, ophthalmology,
  gynaecology and the ward products.

### Fixed
- The treatment-change derogation read the wrong year when the change
  fell in a year after the first, asking for the année 1 minimums
  instead of the lighter ones, and counted the entretiens after the
  change beyond the sequence it opens.

## [0.38.0] - 2026-08-25

### Added
- 101 more monographs: neurology (epilepsy, migraine, Parkinson,
  multiple sclerosis, myorelaxants) and rheumatology and immunology
  (AINS, corticosteroids, biotherapies, JAK inhibitors, bone). 657 of
  the 812 cards now carry a full monograph — indications, mechanism,
  posology, contraindications, interactions, adverse effects,
  surveillance, counter advice, pharmacokinetics, renal adaptation,
  pregnancy and sources.

## [0.37.0] - 2026-08-25

### Added
- A printable billing recap, beside the CSV export on the dashboard: the
  entretiens performed and not yet billed, one line each with the date,
  the patient, the theme, the act code (with TPH when it was held
  remotely), the step of the sequence, the situation to declare, the
  coverage rate and the amount, and the total at the foot. A landscape
  A4 page carrying the memo's practical rules underneath, so the sheet
  can go straight to whoever does the invoicing.

## [0.36.0] - 2026-08-25

### Added
- The memo's anticancéreux derogation. An entretien can be marked as
  following a treatment change: it opens a new billable sequence at
  once, at the "années suivantes" tariff, without waiting out the twelve
  months. The button only appears on the two anticancéreux themes, the
  only ones the derogation still covers, and the fiche says which of the
  memo's conditions is not met yet — how many entretiens are missing
  before the change and after it. It travels to the CSV export.
- The "autres traitements anticancéreux" sequence may be finalised
  before twelve months, as the memo allows when entretiens are brought
  closer together at treatment initiation: a completed sequence opens
  the next one straight away instead of being held back by the quota.
- The bilan partagé de médication states its eligibility rule on the
  fiche: the memo reserves it to the patient on at least five treatments
  for six months or more, and the fiche says how many it knows of.
- The memo's practical rules under the Options fee table: tiers payant,
  billed independently of any CIP code, prices TTC, one pharmacy only
  per patient, and the ADRI service when the carte Vitale is missing.
- 102 more monographs: psychiatry, gastro-enterology and hepatology,
  oncology, urology, gynaecology and haematology. 556 of the 812 cards
  now carry a full monograph.

## [0.35.0] - 2026-08-25

### Changed
- The billing follows the Assurance Maladie memo *Aide à la
  facturation — accompagnement pharmaceutique* instead of the fee model
  the app had invented. Every entretien now carries the act code the
  memo prescribes (BMI/BMS, ASI/ASS, AC1/AC3, AC2/AC4), the step of the
  sequence it fills, and the amount that step bills: BMI 15 + 15 + 15 +
  20 = 65 €, ASI 15 + 15 + 20 = 50 €, AC1 15 + 15 + 30 = 60 €, AC2 15 +
  15 + 50 = 80 €, and 10 + 20 = 30 € for every "années suivantes"
  sequence. The code and the step show under the type on the patient
  card; hovering gives the amount, the year of accompaniment and the
  coverage rate.
- The anticancéreux theme splits in two, as the memo does: *anticancéreux
  au long cours* (AC1/AC3) and *anticancéreux (autres)* (AC2/AC4), which
  bill differently. Interviews recorded under the old single theme are
  read as *long cours*.
- The quota per year is no longer a number to set by hand: it is the
  length of the sequence the memo defines for that theme and that year
  — four entretiens for a first bilan de médication, three for a first
  AOD/AVK/asthme year, two for every following year.
- The Options fee grid is the memo's own table: one line per theme and
  per year, the act code, the amount of each entretien of the sequence,
  and the annual total. `config.toml` takes the same two rows
  (`annee_1`, `annees_suivantes`); a file written for an earlier version
  is still read and keeps billing what it billed.
- The CSV export gains Code acte, Année, Étape, À distance, Situation
  and Prise en charge (%).

### Added
- The patient's situation — ALD, AT/MP, maternité — is recorded on the
  fiche and travels to the export, the memo requiring it to be taken
  into account when billing.
- An "À distance" button on each entretien of an accompaniment: the TPH
  code the memo adds for a remote entretien, billed on top of the act
  code. Its amount is an option, the memo giving none.
- The code traceur TAC (adhésion, 0,01 €), billed once per patient and
  per theme when they join, is an option of its own.
- 179 more monographs: cardiology and hypertension, lipids and diabetes,
  anti-infectives, vaccines and pneumology. 454 of the 812 cards now
  carry a full monograph.

## [0.34.0] - 2026-08-25

### Added
- A day view: the counter's opening hours down the left, each
  rendez-vous and entry placed on its line, two abreast when they share
  an hour, and what has no hour listed underneath so nothing is
  hidden. The amplitude is an option (`day_start_hour`, `day_end_hour`).
  Clicking a day in the week or the month opens it.
- Recurring entries: a formation or a delivery repeats every week,
  fortnight or four weeks. It is stored once, shown on every day it
  falls on, and removed as a series.
- An overdue banner above the agenda: how many rendez-vous have slipped
  past their date, the oldest one, and the patients to reopen.
- The catalogue grows from 275 to 812 drugs, covering the essentials of
  the French market: cardiology and diabetes, anti-infectives,
  pneumology and ORL, neurology and psychiatry, analgesia and
  rheumatology, gastro-enterology, dermatology, gynaecology, urology,
  ophthalmology, haematology and the smoking-cessation products. The
  275 cards that had a monograph keep it; the new ones ship their
  identity, class and antidote for the team to fill in.

## [0.33.1] - 2026-08-25

### Changed
- The printed week fills the page: full-height day columns, so the plan
  can be written on during the week rather than only read.

## [0.33.0] - 2026-08-25

### Added
- Rendez-vous now have an hour. It is typed the fast way — 9, 9h30,
  930, 09:30 — on the patient's interview table or straight from the
  agenda's day panel, it leads the block on the week grid and the line
  in the day list, and the day is ordered by it with the untimed
  rendez-vous last. Agenda entries carry one too.
- A rendez-vous can be moved from the agenda: "Déplacer" takes a date
  in the usual compact form, without opening the record. Both writes
  are compare-and-set on what the screen showed.
- The agenda filters by act kind: click the kinds to narrow the grid,
  the day panel and the list at once, "Tous" to see everything again.
- "Imprimer la semaine" typesets the week on a landscape A4 page, one
  column per day, rendez-vous and other entries in the order of the
  day.
- The left and right arrows move the agenda a week — or a month in
  month view.
- 1017 posology lines over 168 drugs, against 368 over 60: the
  anticoagulants and opioids, the inhalers and insulins, Parkinson,
  psychiatry, urology and contraception, each with the lesser-known
  uses marked where they lie outside the AMM.

## [0.32.0] - 2026-08-25

### Added
- Every one of the 275 drug cards now ships a full monograph: the last
  ones are the oral tyrosine-kinase inhibitors and the cytotoxics taken
  at home (osimertinib, erlotinib, sunitinib, sorafénib, dasatinib,
  nilotinib, olaparib, témozolomide, hydroxycarbamide), the remaining
  insulins and the sulfamide. Each carries its indications, mechanism,
  posology, contraindications, interactions, adverse effects,
  monitoring, counselling points, pharmacokinetics and numbered
  sources.

## [0.31.0] - 2026-08-25

### Added
- Reference monographs for 247 of the 275 drug cards, against 141:
  oral anticancer drugs and hormonothérapies, the remaining HBPM,
  cardiology, inhalers and insulins, Parkinson and psychiatry, urology,
  dermatology with the dermocorticoid strength classes, ophthalmic
  drops, contraception, biologics and anti-infectives.

### Fixed
- The starter catalogue held the same product twice, "Kaléorid" and
  "Kaleorid", so one brand had two cards and only one of them a
  monograph. The duplicate is gone, and the uniqueness test now folds
  accents, case, spaces and hyphens so a second spelling cannot slip
  in again.

## [0.30.0] - 2026-08-25

### Added
- Reference monographs for 141 drugs, against 61: the analgesics and
  NSAIDs, the opioids, cardiology and diabetes, gastro-enterology,
  allergy and ORL, more psychotropes, and the counter staples — each
  with its indications, mechanism, posology, contraindications,
  interactions, adverse effects, monitoring, counselling points,
  pharmacokinetics and numbered sources.

### Fixed
A review of the day's work found eleven defects; all are fixed.
- A migration meant to run once was replayed at every unlock and
  destroyed table corrections made after it, including on a PC whose
  clock ran behind. It is gone; corrections are only ever removed by
  "Rétablir la table".
- Class notes and table cells were written blind: a colleague's
  paragraph or correction could be overwritten without notice. Both are
  compare-and-set now, with a French notice when the view was stale, as
  is the removal of a protocol step and its subtree.
- A class note written as "avk" and one written as "AVK" were two
  different rows, so an edit could vanish. One note per class now,
  whatever the spelling.
- `cycle_months` and the enforcement choice were inert: the rule, the
  fee ranks and the patient table all hardcoded twelve months, and
  "informer" or "refuser" still behaved like "avertir".
- The half-life reader took the "min" inside "administration" for
  minutes, turning a five-hour half-life into five minutes on the decay
  curve.
- The side pane's carnet shared its buffers with the patient and drug
  journals: text typed in one appeared in the other, and could be
  posted as a transmission.
- A protocol could not be renamed — the fields were re-cloned on every
  frame — and clicking a patient in the month view's day panel did
  nothing.
- A mistyped font path crashed the app on start, with no way back
  except editing config.toml by hand; the file is now parsed first and
  ignored when it is not a font.

## [0.29.0] - 2026-08-25

### Added
- Substitution protocols ("Protocoles…" in the drug base): what to
  dispense when a drug cannot be, written as a decision tree. A step is
  either a question — "clairance inférieure à 30 mL/min ?", "apixaban
  disponible ?" — with its oui and non branches, or a conduite to
  follow. Steps are added, rewritten and removed in place, each write
  compare-and-set like the rest.
- "Dérouler" walks the tree one question at a time, so the protocol can
  be followed at the counter without reading the whole thing, and
  "Imprimer" typesets it as an indented A4 page for the binder.
- The demo database ships one written the way a team would: AOD
  indisponible, branching on the clairance and on what the wholesaler
  has.

## [0.28.0] - 2026-08-25

### Added
- Posologies by indication: every drug card can carry a table of what
  it is prescribed for, the dose for that indication and what changes
  it — read on the monograph, edited line by line in the form, printed
  with the A4 sheet, and removed with the card.
- 368 shipped lines over 60 drugs, mainstream and lesser-known alike:
  spironolactone in acne and hirsutism, propranolol in essential
  tremor, migraine and performance anxiety, fosfomycine as monthly
  prophylaxis, doxycycline at anti-inflammatory dose in rosacea,
  amitriptyline in neuropathic pain, gabapentine in restless legs,
  aspirin in pre-eclampsia prevention — each marked when it is outside
  the AMM. They only ever fill a card whose list is still empty.

## [0.27.0] - 2026-08-25

### Added
- The right pane holds three contents, switched by a tab row: the team
  documentation, the day's carnet (readable and writable without
  leaving the current view), and the operator's personal notes. Which
  one opens with the app is an option.
- Font selection: point `[ui] font_path` at a .ttf or .otf — or pick it
  from Options — and the whole interface uses it; the embedded family
  remains the fallback.

### Fixed
- Wrapped text in the dashboard's two lists is no longer justified by
  `columns`, which stretched the spaces between words.

## [0.26.1] - 2026-08-25

### Fixed
- A `mut` left on the markup renderer's closure failed `cargo clippy
  -D warnings`, so the 0.26.0 release build did not compile under the
  CI gate.

## [0.26.0] - 2026-08-25

### Added
- The interface adapts to the screen and the eye: a text scale (0.8 to
  1.6) and a "compact" density that fits noticeably more on a small
  screen, both in Options and in `config.toml`, applied live.
- Optional toolbar pictograms, painted rather than typed — the bundled
  font carries almost no symbols — in the same square Motif style: a
  sheet for the documentation, bars for the dashboard, a capsule for
  the drug base, a month grid for the agenda, a pen for the carnet, a
  padlock, a cog and a template.
- Light formatting in the team's free text: `*gras*`, `_italique_` and
  `=surligné=` are rendered wherever the text is read — monograph
  sections and every note journal — while the editors stay plain text.

## [0.25.0] - 2026-08-25

### Changed
- The seventeen reference tables are rewritten wider and deeper: 95
  columns and 158 rows in all, against 40 and 108. The IPP table gains
  the forms, the moment of intake and the clopidogrel remark; HBPM the
  renal threshold, the monitoring and the antidote; the statins their
  LDL band, intensity and interaction risk; the corticoids their
  duration of action and mineralocorticoid effect; the opioids their
  delay, duration, forms and renal caution; the benzodiazepines their
  half-life, indication and elderly caution — and the same for the
  eleven others (AOD antidote and renal follow-up, inhaler devices and
  rinsing, insulin timing and storage, what each CKD stage changes for
  metformine, AOD and HBPM, the conduct per Mac Isaac band, cystitis
  durations and follow-up, missed-pill delays, analgesic paliers and
  cautions, who may be vaccinated by the pharmacist, paediatric forms
  and daily maxima, and the alternative to crushing).
- The printed reference now typesets in fixed fractional columns with
  French hyphenation, so a long word wraps inside its cell instead of
  spilling over the next one. It runs to sixteen A4 pages.

### Note
- Table corrections made with 0.23.0 or 0.24.0 are dropped on upgrade:
  the tables changed shape, so a cell edited before no longer points at
  the value it was written for. Corrections made from 0.25.0 on are
  kept as usual.

## [0.24.0] - 2026-08-25

### Added
- Formes et dosages on every drug card, shown with the
  pharmacokinetics on screen and on the printed monograph.
- Class notes: a note shared by every card of the same therapeutic
  class ("Note de classe…"), written once and read on all of them.
- "Rechercher…" opens the public ANSM medicines database in the
  browser, pre-filled with the card's brand name and DCI. The app
  itself stays offline.

## [0.23.0] - 2026-08-25

### Added
- The reference tables are editable in place: click a value, correct
  it, and it is shown in the accent colour with the shipped value on
  hover. "Annuler la dernière" undoes the last correction and
  "Rétablir la table" restores everything as shipped. The corrections
  are stored in the shared database and print with the table.
- A "Calculs" panel under the tables: clairance de la créatinine
  (Cockcroft & Gault, with the CKD stage), dose par kilo (par prise et
  par jour), and the decay curve of a drug — time to near-complete
  elimination and accumulation ratio at steady state, fed by any half-
  life from the drug base.

### Changed
- The tables view scrolls as a whole, so the sources, the corrections
  and the calculators stay reachable in a small window.

## [0.22.0] - 2026-08-25

### Added
- The dashboard opens on where the team left off: the last patients
  whose file moved (one click to reopen) and everything written today —
  the day's notes and the day's transmissions.
- The carnet is printed from an editable template, like the interview
  sheet and the CR letter: "Modèles…" gained a "Carnet" tab, validated
  and previewable, saved next to config.toml (`carnet_layout.typ`).
- Operator colours: each set of initials gets a stable colour, on the
  note stamps in every journal and on the printed carnet page, so a
  page can be scanned by who wrote what.

## [0.21.0] - 2026-08-25

### Added
- The agenda holds what is not a billable act: formations, réunions,
  livraisons, congés and free entries, created from the day panel and
  drawn on the grid in their own muted colour.
- A month view next to the week: a Monday-aligned grid with one chip
  per act and per entry, today highlighted, the days outside the month
  dimmed, and week/month navigation.
- A day panel under the grid — click a day (or a column header) to
  detail it: its rendez-vous with one-click access to the patient, its
  entries, and its own dated notes journal.

## [0.20.0] - 2026-08-25

### Added
- Four more fields on every drug card: statut administratif (badge
  coloured by what it says — rupture, retrait, hors AMM,
  commercialisé), évaluation SMR / ASMR, étiquettes, and toxicité /
  marge thérapeutique. All four print on the A4 monograph.
- The reference cards ship with their étiquettes and their toxicité
  derived from the monograph itself (classe, marge étroite,
  surveillance biologique, contre-indication grossesse, vigilance
  conduite), plus the encadrements that change dispensing (Previscan
  en poursuite seulement, NFS de la clozapine, ordonnance sécurisée du
  zolpidem, accord de soins du valproate).
- The drug search matches the class and the étiquettes as well as the
  brand and the DCI: typing "statine" or "marge étroite" finds the
  cards, ranked below an identity match.

## [0.19.0] - 2026-08-25

### Fixed
- The lock screen accepts `Entrée` again: pressing it made the field
  surrender focus, and the immediate re-focus cancelled the submission,
  so only the button worked.

### Added
- The fee matrix follows the quotas: an act limited to N per cycle
  shows N price columns, the ranks beyond it are struck out.
- The cycle length is configurable (`[rules] cycle_months`, 12 by
  default) — an entretien of year 0 and the first of year 1 are that
  many months apart, and the quota window follows.
- What happens when the quota is reached is now a choice: avertir
  (message with an explicit "créer quand même", the previous
  behaviour), informer seulement (the act is created, the rule is
  stated), or refuser (no override).

## [0.18.0] - 2026-08-25

### Added
- Drug cards open as a **monograph on a sheet of paper**: uppercase
  section headings over hairlines, the sections in reading order
  (indications, mécanisme d'action, posologie, contre-indications,
  interactions, effets indésirables, surveillance, conseils au
  patient), the pharmacokinetics as a definition list and the numbered
  sources at the foot. "Modifier" switches to the editable form,
  "Imprimer" typesets the same sheet as an A4 PDF.
- Six new fields on every card — indications, mécanisme d'action,
  contre-indications, effets indésirables, surveillance and sources —
  stored, edited and saved compare-and-set like the rest.
- Reference monographs for ~60 drugs, written at monograph depth: the
  anticoagulants (AOD, AVK, HBPM), the inhalers, the narrow-margin
  drugs, the oral anticancer drugs, and now the antibiotics and the
  psychotropes, each with its own numbered sources.

### Changed
- Reference tables carry **numbered sources** instead of a prose
  caution line, on screen and in the printout.
- The monograph headings read in full: "Posologie", "Conseils au
  patient", "Notes de l'équipe".

## [0.17.0] - 2026-08-25

### Added
- Reference clinical data on the ~30 drug cards the interviews turn
  around (the four AOD, the three AVK, énoxaparine, the inhalers,
  méthotrexate, lithium, digoxine, amiodarone, lévothyroxine,
  metformine, sémaglutide, capécitabine, dénosumab…): posology,
  interactions to watch, the advice to give the patient (plan de prise,
  technique, signaux d'alerte) and the pharmacokinetics. Cards outside
  that list keep their clinical fields empty, as before.
- "Compléter les fiches de référence" in Options fills those fields on
  an existing base, column by column and only where a field is still
  empty — the team's own text is never overwritten.
- Two more tables (seventeen in all): paediatric doses by weight, and
  what may be crushed or opened (LP and gastro-resistant forms,
  microgranules, dabigatran, cytotoxics) with the practical rules.

### Changed
- Posology, interactions, patient advice and elimination are now
  multi-line fields on the drug card, so a full reference text is
  readable without scrolling inside the field.
- The demo database no longer overrides the Eliquis card: it shows the
  shipped reference text.

## [0.16.0] - 2026-08-25

### Added
- Nine new reference tables, bringing the counter set to fifteen: AOD
  posologies with their renal adaptation, inhaled-corticosteroid dose
  steps, insulin action profiles, renal function (Cockcroft formula and
  CKD stages, with the metformine thresholds), the Mac Isaac score and
  what to do with the angina TROD, first-line cystitis treatments,
  missed-pill conduct, non-opioid analgesic doses, and the adult
  vaccination boosters — each with its own caution line, on screen and
  in the printed A4 reference.
- The starter drug base grows from ~200 to ~275 entries: more oral
  anticancer drugs (osimertinib, olaparib, ITK…), cardiology,
  pneumology and diabetes complements (insulins, tirzépatide, triple
  inhalers), neurology and psychiatry, gastro-enterology, urology,
  dermatology, ORL, ophthalmology, anti-infectives and immunology.

### Changed
- The tables view widened to 940 px and its selector wraps over several
  rows; long reference cells now wrap inside their column and the
  sunken box grows with the table instead of clipping it.

## [0.15.2] - 2026-08-25

### Fixed
- A partial fee table in `config.toml` no longer bills 0 €: writing
  `bpm = { initial = 65.0 }` (or misspelling a key) now keeps the
  default fee for every rank it does not mention, instead of zeroing
  the suivi fees in the dashboard, the chart and the CSV.
- Escape with the quick picker open closes the picker instead of
  leaving the patient view (and leaving the picker armed for the next
  patient).
- The theme chosen in the quick picker is dropped when the picker is
  closed without creating an act; it can no longer attach itself
  silently to a later act created from the direct buttons — including
  onto the CR letter and the export.
- The picker's 1-9 shortcuts are ignored while a field has the
  keyboard, so typing a duration or a date behind the dialog cannot
  create billable acts.

## [0.15.1] - 2026-08-25

### Added
- The thematic is printed on both documents: a "Thème" line in the
  interview sheet's header box and under the CR letter's subject
  (`{{THEME}}` in either template; an empty theme prints a dash).

### Changed
- The interview sheet's note boxes lost 2 mm each so the signature and
  next-RDV boxes still fit on the page under the new header line.

## [0.15.0] - 2026-08-25

### Added
- Per-rank fee schedule: each act kind is now paid by its rank inside
  the année d'accompagnement (entretien initial / 1er suivi / 2e suivi
  et au-delà). The Options dialog edits the nine acts as a matrix, and
  `config.toml` accepts both `bpm = { initial = 60, suivi_1 = 20,
  suivi_2 = 20 }` and the legacy flat `bpm_fee = 60`. Ranks follow the
  same 12-month cycles as the quota rules, and drive the dashboard,
  the patient table and the CSV export.
- Thematics on every entretien (observance, biologie/INR, technique
  d'inhalation, interactions…): a drop-down per row, compare-and-set
  like every other shared write, exported in a new CSV column.
- Quick act picker (Ctrl+N or "Choix rapide"): the nine acts with
  digit shortcuts and colour chips, plus the theme the new act will
  carry — one keystroke from patient to created act.
- Database maintenance in Options: "Compléter les médicaments de
  départ" tops up a base created before the starter list grew, and
  "Réinitialiser la base…" (two-step, red) wipes every row and reseeds
  the drugs — for debugging and demos.
- `BPM_CADDY_WINDOW=1280x1100` opens the window at a given size
  (screenshots, e2e).
- The drug view warns when the base holds only a handful of cards and
  points at the top-up button.

### Changed
- The Options dialog now sizes itself to the window instead of a fixed
  560 px, which clipped the last sections on small screens.

## [0.14.0] - 2026-08-24

### Added
- Three new act kinds complete the conventioned set: accompagnement
  AVK, accompagnement anticancéreux oraux, and vaccination — each with
  its own fee, yearly quota, agenda color, and act button (rows now
  wrap).
- Drug base grown to ~200 starter entries: oral anticancer drugs
  (capécitabine, imatinib, CDK4/6, hormonothérapie…), the missing HBPM
  brands (Innohep, Fraxiparine, Fragmine), Parkinson, antipsychotics,
  uro/gynéco, os/rhumato, and more counter staples.
- Database file tools in Options: browse to an existing base with a
  native file dialog, write a consistent encrypted copy anywhere
  (VACUUM INTO), or move the base — copy, repoint config, old file
  kept as a fallback.

## [0.13.0] - 2026-08-24

### Added
- Carnet de transmissions ("Carnet", F5): the end-of-day team handover
  logbook — one page per day, entries stamped heure · opérateur,
  chronological within the day, browsable day by day (‹ jumps to the
  previous day with entries, "Aujourd'hui" returns), and printable as
  an A4 page for the binder. Past pages are read-only: new entries
  always land on today's page

### Changed
- The toolbar gained "Carnet (F5)"; the version number moved into the
  BPM-Caddy tooltip (with the database and config paths) so all eight
  buttons fit at the default width; "Modèle PDF…" became "Modèles…"

## [0.12.0] - 2026-08-24

### Added
- Standalone dated notes: an append-only journal (date · heure ·
  opérateur, deletable with confirmation) attached to each patient
  ("Notes de suivi" on the patient page), each drug ("Notes datées" on
  the drug page), and each operator (personal notes at the bottom of
  the documentation pane, keyed by the operator initials). Patient and
  drug journals are removed with their subject; operator notes are
  personal and survive

## [0.11.0] - 2026-08-24

### Added
- Drug pages: each card grows into a two-column page — "Fiche
  clinique" (identity, dosage, interactions, IUP, antidote, notes) and
  "Pharmacocinétique" (demi-vie, AUC / exposition, élimination,
  adaptation DFG, grossesse / allaitement); all team-filled, saved with
  compare-and-set like the rest

### Changed
- Professional layout pass: every view now aligns to a fixed-width
  centered content column (`motif::column`) — headings centered,
  content on one left grid; the last dozen magic centering offsets are
  gone, form grids share a common label column width, the alert color
  is a single `motif::ALERT`, and the window has a minimum size so
  layouts cannot collapse
- The note-stamp timestamp is only queried on click instead of every
  frame (was one SQLite query per frame with the docs pane open)

## [0.10.0] - 2026-08-24

### Added
- Convention rules enforced at act creation: each kind allows N acts
  per "année d'accompagnement" (12 months from the cycle's first act;
  the next cycle starts at least 12 months later). A blocked creation
  explains the rule and shows the next possible date, with an explicit
  "Créer quand même" override. Quotas configurable per kind
  (`[rules]`, 0 = no limit; defaults: BPM/AOD/Asthme 3, TROD 0,
  Prévention 1)
- Global options editor ("Options…" in the toolbar): pharmacy identity,
  interface, auto-lock, backups, database path, fees, and yearly-rule
  quotas — all edited in-app and saved to config.toml, applied live
  (path change takes effect on restart). The master-password change
  moved inside it, keeping the toolbar compact

## [0.9.0] - 2026-08-24

### Added
- Conversion tables at the counter ("Tables de conversion" in the drug
  view): IPP dose equivalences, HBPM usual dosing (curatif /
  prophylaxie), statine equivalent doses, corticoid anti-inflammatory
  equivalences, opioid equianalgesia (réf. morphine orale), and
  benzodiazepine equivalences (Ashton) — each with its caution line,
  browsable per tab and printable as a two-page A4 reference

## [0.8.0] - 2026-08-24

### Added
- Motif list boxes: patient and drug searches render in proper sunken
  list panels with full-width selection bars, hover tint, and tight
  rows (new `motif::list_row` / `list_row_job` / `section` widgets) —
  fuzzy-match highlighting kept
- A status bar: patient / in-progress / drug counts on the left, the
  database file on the right (replaces the under-search totals line)
- "Entretiens" section separator on the patient view
- The starter drug base grows from ~58 to ~135 common French drugs
  (anti-infectieux, AINS, gastro, allergie, psychiatrie, neurologie,
  cardio-métabolisme, divers) — still brand + DCI + class only, with
  textbook antidotes where they exist

## [0.7.0] - 2026-08-24

### Added
- CR letter to the médecin traitant ("CR" button on each interview
  row): a Typst-generated letter with the pharmacy letterhead
  (`[pharmacy]` in config.toml — name, address, phone, pharmacist),
  the addressed physician, the act and date, the patient's known
  treatments (name, DCI, class, dosage), and boxes for the handwritten
  synthesis and signature; names are escaped like everywhere else
- The template editor now handles both templates: "Fiche entretien"
  and "Courrier CR" tabs, each validated, previewable with sample
  data, and saved to its own file (`cr_layout.typ` next to config.toml
  or `[templates] cr_template_path`)
- Reverse treatment lookup on the drug card: "Patients sous ce
  traitement" chips (recall / alert question), one click from the
  patient's record

## [0.6.0] - 2026-08-24

### Added
- Fuller patient record: médecin traitant, e-mail and address, shown
  on the patient view and edited via "Modifier" (compare-and-set like
  the rest); the CR recipient is finally on the record
- Current treatments on the patient: drugs linked from the shared base
  as chips on the patient view — click opens the drug card, "×"
  unlinks, and a small fuzzy picker (brand or DCI) adds one; links are
  removed atomically with the patient
- Drug cards gain the therapeutic class ("Classe"), shown in the card
  header and the search rows; the ~58 starter drugs now carry their
  class (AOD, AVK, statine, IPP, benzodiazépine…)

## [0.5.0] - 2026-08-24

### Added
- The agenda opens on a colored week grid (Mon–Sun, current week by
  default): one block per RDV, colored by act kind, today highlighted,
  hover shows patient/kind/phone, click opens the patient; week
  navigation (‹ Aujourd'hui ›) and a color legend; the day-grouped
  list (with overdue) stays below
- New billable acts: TROD angine, TROD cystite, and RDV prévention —
  buttons on the patient view, own colors, fees in `[billing]`
  (`trod_angine_fee`, `trod_cystite_fee`, `prevention_fee`), counted
  everywhere (dashboard, CSV, PDF sheets)
- In-app editor for the Typst PDF template ("Modèle PDF…" in the
  toolbar): edit the sheet's source with validation (invalid templates
  are refused with the Typst error), a sample-patient PDF preview, and
  reset-to-default; saved next to `config.toml` (or at
  `[templates] bpm_template_path` when configured) and picked up by the
  next "Fiche PDF"
- Toolbar labels shortened so all views fit at the default window width

## [0.4.0] - 2026-08-24

### Added
- Agenda ("Agenda", F4): the upcoming patient appointments grouped by
  day with French weekday names, "aujourd'hui / demain / en retard"
  flags, phone numbers, one-click access to the patient, and printing
- Drug cards gain the DCI (dénomination commune internationale): shown
  under the name, searchable ("elix" and "apixa" both find Eliquis),
  and included in notes inserts; the card layout was reworked (identity
  header, antidote banner in red, dim labels, wider fields)
- A fresh drug base is seeded with ~55 common French drugs (brand name,
  DCI, and textbook antidotes only — dosage/interactions/IUP are left
  for the team to fill from the references they trust); seeding happens
  once and never resurrects deliberately deleted cards
- All UI strings live in an embedded TOML (`assets/strings.fr.toml`);
  any wording can be overridden — or the app translated — by a
  `strings.toml` placed next to `config.toml`, without recompiling
- Patient forms polished: dim labels, wider fields, consistent with
  the drug card
- Drug reference base ("Médicaments", F3): team-shared encrypted cards
  (dosage, interactions, IUP, antidote, notes personnelles) with the
  same fuzzy search / quick-create / compare-and-set workflow as
  patients — typing two letters shows dosage and antidote at a glance,
  and "→ Notes d'équipe" inserts name + dosage into the shared notes
- Note-entry aids in the documentation pane: an "Opérateur" field
  (default from `[ui] operator` in config.toml) and a "+ Entrée" button
  stamping "— date heure · opérateur · patient courant : " into the
  notes, for succinct team entries
- Discreet finances (on by default, `[ui] discreet_finances`): dashboard
  amounts are masked ("•••") and the monthly revenue chart hidden; a
  small unlabeled control in the dashboard corner reveals them, and they
  re-mask on leaving the dashboard or locking
- "RDV à venir" on the dashboard: planned interviews not yet performed,
  soonest first, overdue ones flagged in red ("en retard"); clicking a
  row opens the patient (never masked — dates are not financial data)
- A misclicked state advance can be undone: each interview row gains a
  small "«" button that steps back to the previous pipeline state,
  including un-billing
- Enter submits the quick-create patient form from any of its fields
  (no mouse needed, per the shortcut-driven spec)
- Patients can be found by typing their phone number in the search, the
  patient list is kept alphabetical (accent-insensitive) when browsing
  with an empty query, and the CSV export includes the phone column
- The number of daily backups kept is configurable
  (`[database] backups_keep`, default 14, 0 disables them)
- CSV export from the dashboard ("Exporter CSV") for billing
  reconciliation: every interview with patient, dates, duration and
  fee, written to `exports/` next to the database and opened in the
  default spreadsheet (French Excel conventions: BOM, semicolons,
  decimal comma)
- The app and launcher windows have an icon (`motif::icon()`, a Motif
  bevel square drawn programmatically) so they are recognizable in the
  taskbar and alt-tab
- A commented `config.toml` template is written on first launch, so the
  available options are discoverable without reading the documentation
- Launcher: network timeouts (10 s connect, 30 s per read) so a hung
  connection can no longer block startup, and the downloaded binary's
  size is verified against the release metadata before it replaces the
  installed copy (no more silently truncated updates)
- Fiche PDF: embedded fonts are parsed once per session instead of on
  every click, and each sheet gets a unique file name so regenerating
  while the previous PDF is still open no longer fails on Windows
- Automatic daily backups: after each unlock, a consistent encrypted
  snapshot (`VACUUM INTO`) is written to `backups/bpm_caddy-AAAA-MM-JJ.db`
  next to the database; the 14 most recent are kept
- The master password can be changed from the toolbar ("Mot de passe…"):
  the database is re-encrypted (SQLCipher rekey) and a password
  remembered in the OS credential manager is updated in place
- Patient records can be corrected and deleted from the patient view:
  "Modifier" edits the identity (name typo, wrong birth date), and
  "Supprimer…" removes the patient with a two-step confirmation (the
  patient's interviews are deleted atomically with them)
- A single interview can be removed with the "×" button on its row
  (two-step confirmation), for entries added by mistake
- Patients gain a phone number and a free-form comment (allergies,
  preferences…), edited via "Modifier" and shown on the patient view;
  the dashboard's "RDV à venir" list shows the phone so the patient can
  be called about the appointment
- The RDV list can be printed ("Imprimer" next to "RDV à venir"): a
  Typst-generated A4 table of the upcoming appointments with phone
  numbers, opened in the PDF viewer (patient names are safely escaped)
- Escape leaves the dashboard back to the search, and appointments
  scheduled for today are highlighted "aujourd'hui" on the dashboard
- Search results show a "n entretien(s) en cours" badge for patients
  with not-yet-billed interviews, and the letters matched by the fuzzy
  query are underlined in the result names
- The dashboard shows the interview count per type under the funnel,
  and the "Fiche PDF" is dated with the planned RDV when one is set
- The interview table has column headers, the search screen shows the
  patient / in-progress totals, and hovering the version number reveals
  the database and configuration paths in use (multi-post support aid)
- Error messages clear as soon as a following operation succeeds
  instead of lingering
- The open patient view follows background refreshes: identity edits
  from another post appear within a minute, and the view closes if the
  patient was deleted elsewhere
- `scripts/screenshots.sh` regenerates the README screenshots
  reproducibly (seeded demo, xvfb)
- Multi-PC robustness on a shared database: a 5-second busy timeout
  instead of immediate "database is locked" errors; state advances are
  compare-and-set (a click based on a stale view is rejected with a
  message instead of silently overwriting a colleague's change); open
  views re-read the database every minute; the quick-create form
  re-checks the patient list before offering creation (no duplicates
  when another post just created the patient); shared team notes pick
  up other posts' edits while clean and merge line-by-line on
  concurrent saves instead of last-writer-wins
- Compact date entry everywhere a date is typed: "230826" (JJMMAA),
  "23082026" (JJMMAAAA), "2308" / "23/08" (current year), and two-digit
  years in separator form ("3/7/58"). Two-digit years expand by context:
  birth dates never land in the future ("49" → 1949), RDV dates are
  always 20xx

### Fixed
- Review round on the multi-post work: RDV dates, durations, patient
  corrections and interview deletions are now all compare-and-set (a
  stale field or form can no longer silently revert or destroy a
  colleague's newer change — deleting an interview a colleague meanwhile billed is
  refused); an RDV typed but not tabbed out of is committed when the
  view changes or the app locks; patient names are escaped in the
  interview sheet too (Typst injection); a yearless date ("2308") is
  rejected for birth dates instead of storing a current-year birth; the
  CSV gains a "Facturé (€)" column so summing it matches the dashboard
  (the tariff column alone over-declared); the shared-notes merge no
  longer rewrites the text under a focused cursor; the daily backup
  runs on a background thread (no UI freeze at unlock on a network
  share); the notes sync only polls while the pane is shown; and the
  quick-create duplicate check is throttled instead of re-reading the
  database on every keystroke
- Dates are validated for real: 31/02, 31/04 or 29/02 outside leap years
  are now rejected instead of being stored as impossible ISO dates
- The interview creation date is displayed as JJ/MM/AAAA instead of ISO
- Escape while typing in a field of the patient view only drops focus
  (egui's behavior) instead of also closing the view and discarding the
  in-progress edit
- Fuzzy search now folds uppercase accented letters ("ÉMILE" matches
  "emile"); previously only lowercase accents were stripped
- Quick-create opens the patient by the id returned from the insert
  instead of relying on unspecified row order
- The team documentation pane is never shown on the lock screen, and a
  dirty document auto-saves even while the pane is hidden

## [0.3.0] - 2026-08-22

### Added
- Time tracking per interview (inline "min" field) feeding the hourly ROI
  KPI ("Taux horaire") on the dashboard
- Master password can be remembered in the OS credential manager (Windows
  Credential Manager, macOS Keychain, Secret Service on Linux) for silent
  unlock at startup; unchecking the box removes the stored copy
- Planned interview dates ("RDV JJ/MM/AAAA" per interview row)
- "Verrouiller" toolbar button and Ctrl+F back-to-search shortcut
- Screenshots in the README, captured from the running app

### Fixed
- Dashboard KPI row no longer overflows narrow windows; monthly chart
  labels months as MM/YY

## [0.2.0] - 2026-08-22

### Added
- Encrypted patient database: SQLCipher (256-bit AES) with a master-password
  unlock screen; wrong passwords are rejected before any data is touched
- Diacritic-insensitive fuzzy patient search ("jndp" finds "Jean Dupont"),
  keyboard navigation (arrows + Enter), and seamless quick-creation form
  (Nom / Prénom / Date de naissance) when no patient matches
- Interview lifecycle state machine (Identifié → Planifié → Réalisé →
  CR envoyé → Facturé) with one-click advancement from the patient view
- `config.toml` support: database path (shareable network drive — the team
  documentation follows the database), auto-lock timeout, per-kind billing
  fees, UI defaults
- Auto-lock: the app returns to the password screen after the configured
  inactivity timeout
- Financial dashboard (F2): billed vs pending revenue KPIs, pipeline
  funnel, and a monthly billed/pending bar chart, all Motif-styled
- Embedded Typst engine: one-click "Fiche PDF" from the patient view
  compiles a single-page A4 interview sheet (patient header + rounded
  boxes for handwritten notes) in memory and opens it in the OS PDF
  viewer; the template is overridable via `[templates]` in `config.toml`

## [0.1.0] - 2026-08-22

### Added
- Project specification (`docs/SPECIFICATIONS.txt`)
- Application skeleton (egui shell)
- Release build pipeline for Windows / macOS / Linux
- `bpm-caddy-launcher`: auto-updating launcher that fetches the latest release
  on startup, with a download progress bar and an offline fallback to the
  installed version
- `motif` crate: old-school X/Motif theme for egui (mwm blue-grey palette,
  square corners, raised/sunken bevels, Motif-style buttons and progress bars),
  applied to both the app and the launcher
- Docked team documentation pane in the app (French, `F1` to toggle,
  debounced auto-save) for shared notes at the counter
