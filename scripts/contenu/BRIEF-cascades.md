# Brief: draft new "Récepteurs et cascades" for BPM-Caddy (read-only on the repo)

Repo: /home/youssef/Projects/BPM_Caddy (Rust clinical-pharmacy app, UI in French).
DO NOT edit any repo file and DO NOT run cargo. Write your output ONLY to the file named in your task.

## What a cascade is
Read `src/cascade.rs` lines 1-100 (the language) and the 10 shipped cascades in `src/db.rs`
(search `pub const STARTER_CASCADES`, ~line 30499) — copy their style exactly (French, terse,
`#` comments explaining what is NOT drawn, `source :` lines, `scénario` lines).
Also read docs/CONTENU.md section "## Les récepteurs et cascades".

The model is qualitative: every node is 1 at rest; `->` activates, `-|` inhibits; molecules act
on nodes as agoniste / agoniste partiel / agoniste partiel faible / antagoniste / inhibiteur /
activateur / potentialisateur. `adaptation N` = the node's density adapts against its input
(blocked long → multiplies → rebound at stop; stimulated long → rarefies → tolerance). Only put
`adaptation` where tolerance/rebound through that node is established pharmacology.
`tonus faible N` = near-zero activity at rest (blocking it alone does nothing).
Loops are allowed (feedback `A -| B` written after the forward chain; loops are cut by text order).

## Hard rules (tests enforce them)
1. Every `molécule <nom>` must be the EXACT DCI of a shipped card (case/accents folded, whole name).
   The list of shipped cards is `drugs.tsv` in the scratchpad dir (name<TAB>DCI<TAB>class). Use the
   DCI column exactly as written (e.g. "acide acétylsalicylique"). If the DCI column has a combination
   or salt, the whole string must match — so prefer single-molecule cards. Grep drugs.tsv before
   using any molecule. Do not invent a molecule that has no card.
2. Every molecule must change at least one `effet` node (alone, or given together with another
   molecule of the same cascade).
3. Every declared node must be linked by at least one arrow. Undeclared nodes become `relais`.
4. Title unique and different from the 10 shipped titles; `titre`, `sujet`, at least one `source`.
5. No invented figures, no doses, no delays. Textbook arrows only (Rang & Dale, Goodman & Gilman,
   RCP/EMA SmPC). A `source :` line names a textbook or "RCP de X : <the clinical fact it supports>".
6. Effects named as what the counter reads: « Pression artérielle », « Kaliémie », « Glycémie »...
   with a `: note` saying what up/down means clinically when useful (see the shipped ones).
7. Only characters common in French text; no arrows (→), no U+202F, no emoji. ASCII `->`/`-|` only
   as the language's arrows.
8. Colour-neutral: never say "rouge/vert".

## Output format (in your output file)
For each cascade:
```
=== CASCADE ===
<the full cascade text exactly as it would sit inside the Rust string, no escaping needed>
=== CLAIMS ===
- given [mol1] : "<effet node>" Up|Down|Rest
- given [mol1, mol2] : "<node>" lower-than|higher-than given [mol1]
- ...
(3-6 claims per cascade: the counter-relevant statements a pharmacist would come to check, e.g.
 "sildénafil + trinitrine lowers Pression artérielle more than either alone". These become tests.)
=== CARD CHECK ===
For each molecule: the drugs.tsv line you matched.
```
Before finishing, re-read every cascade and simulate mentally: does each claimed direction follow
from the arrows (count the inhibitions along the path; an even number of `-|` means same sense)?
Keep each cascade to roughly 8-16 nodes: dense enough to explain, small enough to draw.
