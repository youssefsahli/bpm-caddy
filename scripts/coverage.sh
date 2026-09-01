#!/usr/bin/env bash
# Measure line coverage and refuse to let it fall.
#
# The same idea as `the_posology_coverage_only_improves` in db.rs: a
# floor that is checked, and that only ever moves up. A number nobody
# enforces is a number that drifts down one merge at a time.
#
# Two floors, because one would lie. The whole workspace is dominated by
# `src/app.rs`, which is fifteen thousand lines of egui layout that
# cannot be covered without a UI harness (see the note at the bottom).
# The *logic* modules — the pure, testable ones the clinical content
# actually runs on — are held to a much higher bar, and that is the
# figure worth defending.
#
# Requires cargo-llvm-cov (`cargo install cargo-llvm-cov`).
# Run from the repo root:
#   ./scripts/coverage.sh            # check the floors
#   ./scripts/coverage.sh --html     # and open the annotated report
set -uo pipefail

# Lower these never; raise them whenever a batch of tests lands.
#
# Et laissez-leur de la marge. Le plancher du workspace a été posé une
# fois **à la valeur mesurée exactement**, et le lot suivant de vues —
# des lignes que rien ne couvre, par construction — l'a fait tomber au
# dixième de point près. Un plancher sans marge n'est pas un plancher,
# c'est un cliquet qui casse.
TOTAL_FLOOR=45
LOGIC_FLOOR=89

# The modules that carry the decisions: pure, or nearly so, and the ones
# a wrong answer would reach a patient through.
#
# Named by what is **left out**, and not by a list of what is in. The
# list of what is in had fallen four modules behind — conciliation,
# surveillance, vitale and graph were logic nobody was counting, which
# is the one thing this script exists to prevent. Written this way, a
# module added tomorrow is measured the day it lands and has to be
# excluded on purpose, in writing, to escape.
#
# What is excluded, and why:
#   app.rs, main.rs   the interface; smoke.sh is what holds it
#   winscard.rs       the PC/SC library, opened by name at run time —
#                     there is no reader in CI and there never will be
# (`motif` and the launcher are separate crates and are excluded by the
#  path test below.)
LOGIC_SKIP=(src/app.rs src/main.rs src/winscard.rs)

if ! command -v cargo-llvm-cov >/dev/null 2>&1; then
    echo "cargo-llvm-cov absent : cargo install cargo-llvm-cov" >&2
    exit 2
fi

if [ "${1:-}" = "--html" ]; then
    cargo llvm-cov --workspace --html || exit 1
    echo "  target/llvm-cov/html/index.html"
fi

json=$(mktemp)
trap 'rm -f "$json"' EXIT
cargo llvm-cov --workspace --json --summary-only >"$json" || exit 1

TOTAL_FLOOR="$TOTAL_FLOOR" LOGIC_FLOOR="$LOGIC_FLOOR" \
LOGIC_SKIP="${LOGIC_SKIP[*]}" python3 - "$json" <<'PY'
import json, os, sys

data = json.load(open(sys.argv[1]))["data"][0]
total = data["totals"]["lines"]
skip = set(os.environ["LOGIC_SKIP"].split())

covered = count = 0
rows = []
for f in data["files"]:
    # llvm-cov reports absolute paths; match on the repo-relative tail.
    path = f["filename"]
    # A file of the root crate: /src/… and not /motif/src/… or
    # /launcher/src/…, which are the shell around the logic.
    root = "/src/" in path and "/motif/src/" not in path and "/launcher/src/" not in path
    counted = root and not any(path.endswith("/" + n) for n in skip)
    lines = f["summary"]["lines"]
    rows.append((("src/" if root else "") + path.split("/")[-1], lines["percent"], lines["count"]))
    if counted:
        covered += lines["covered"]
        count += lines["count"]

logic = 100.0 * covered / count if count else 0.0
rows.sort(key=lambda r: r[1])
print()
print(f"{'fichier':28} {'couverture':>11} {'lignes':>8}")
print("-" * 50)
for name, pct, n in rows:
    print(f"{name:28} {pct:10.1f}% {n:8}")
print("-" * 50)
print(f"{'logique métier':28} {logic:10.1f}% {count:8}")
print(f"{'workspace':28} {total['percent']:10.1f}% {total['count']:8}")
print()

fail = False
tf, lf = float(os.environ["TOTAL_FLOOR"]), float(os.environ["LOGIC_FLOOR"])
if logic < lf:
    print(f"ÉCHEC : la logique métier est à {logic:.1f} %, le plancher est {lf:.0f} %")
    fail = True
if total["percent"] < tf:
    print(f"ÉCHEC : le workspace est à {total['percent']:.1f} %, le plancher est {tf:.0f} %")
    fail = True
# A floor far below reality has stopped being a floor.
if not fail and logic > lf + 6:
    print(f"Le plancher logique ({lf:.0f} %) est loin derrière {logic:.1f} % — remontez-le.")
if not fail:
    print("Planchers tenus.")
sys.exit(1 if fail else 0)
PY
status=$?

cat <<'NOTE'

Pourquoi le workspace plafonne bas : src/app.rs fait ~15 000 lignes de
mise en page egui, soit plus de la moitié du dépôt, et une vue ne se
couvre pas sans harnais d'interface. egui_kittest existe pour cela mais
demande egui >= 0.30 ; le projet est sur 0.29. Tant que cette montée de
version n'est pas décidée, le chiffre à défendre est celui de la
logique métier, et scripts/smoke.sh est ce qui tient l'interface.
NOTE
exit $status
