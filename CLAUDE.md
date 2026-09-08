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
  tested), `src/revue.rs` (what a set of treatments says about itself:
  doublons, associations, cascades — same shape, same discipline),
  `src/conciliation.rs` (the file's ordonnance against the one a patient
  brings back from hospital: reads a pasted list, matches each line to a
  fiche, and says what was stopped, changed, added or replaced — pure,
  tested, no catalogue of its own),
  `src/surveillance.rs` (what a treatment asks to have measured and how
  often, read against the dates already in the file — the other half of
  `biology.rs`: that one reads the values that are there, this one names
  the ones that are not),
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
  Pure, tested, no clock: the day is passed in),
  `src/date.rs` (the calendar, written **once**: it was written three
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

## Env hooks (demo / e2e / screenshots)

- `BPM_CADDY_DB=<path>` — database path override
- `BPM_CADDY_PASSWORD=<pw>` — unlock silently at startup
- `BPM_CADDY_NO_KEYRING=1` — skip the OS credential manager
- `BPM_CADDY_START_VIEW=dashboard|patient|drugs|drug_card|agenda|agenda_day|
  agenda_month|protocols|protocol_open|template|options|about|tables|
  tables_search|calc|carnet|vaccins|bio|watch|revue|conciliation|
  vaccine_map|ordonnance|base|codex|
  codex_open|dispositifs|dispositif_open|locations|keys|vitale|
  act_picker|goto|goto_jump|mono_search|mono_patient|graph|registres|stup|
  stup_catalogue|ordonnancier|vigilance|destruction|scans|
  patient_scans|fil|explorer|explorer_organ|classes|classes_outside|export|
  finances|peaux`
  — land on a specific view (screenshots, e2e). `about` is the Options
  dialog on its « À propos » page, `base` on « Base », and `peaux` on
  « Interface », where the eight skins are picked — each drawn in its
  own palette, which is the one thing only a screenshot can check.
  `finances` is the recettes view, which has **no door**: it is in no
  dock, in no tab strip until it has been opened, and not even in the
  list the jump box offers on an empty query — it is reached by typing
  its name. That is deliberate (an « for me » screen offered on the
  fifth line of a menu is not one), and it is also why the smoke test
  has to open it by key: nothing else would.
- `BPM_CADDY_WINDOW=1280x1100` — open the window at that size
- `BPM_CADDY_DRUG_EDIT=1` — with `START_VIEW=drug_card`, land on the
  editable form rather than the monograph
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

The codex works the same way: `src/db.rs` ships `STARTER_PREPARATIONS`
into the `preparations` table once, and everything after that is the
team's — a formula rewritten in the app is never re-seeded over. Adding
a preparation is adding a fiche, never a second table in the code.

The ordonnance's adjuvants are **not** a list in the code: they are the
drug cards tagged `[ordonnance] adjuvant_tag` (default `probiotique`),
with that card's own posology lines as its schemas. Adding a product is
adding a fiche. Resist any pull to hard-code a second catalogue.

## Releases

Push a `v*` tag → `.github/workflows/release.yml` builds app + launcher
for Linux/Windows/macOS and attaches them to a GitHub Release. Asset
names (`bpm-caddy-linux-x86_64`, `-windows-x86_64.exe`, `-macos-arm64`)
must stay in sync with `APP_ASSET` in `launcher/src/main.rs`. Bump all
three crate versions and move the `Unreleased` changelog section before
tagging.
