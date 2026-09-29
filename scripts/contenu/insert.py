#!/usr/bin/env python3
"""Append snippet sections to the end of the matching tables.

Snippet format: lines `## TARGET` open a section; the lines that follow
(until the next `## `) are pasted verbatim before the closing `];` of the
table. Usage: insert.py REPO snippet.rs [snippet.rs...]
"""
import sys, re, os

TARGETS = {
    "DRUGS": ("src/db.rs", "pub(crate) const STARTER_DRUGS: &[(&str, &str, &str, &str)] = &["),
    "DETAILS": ("src/db.rs", "pub const STARTER_DETAILS: &[StarterDetail] = &["),
    "POSOLOGIES": ("src/db.rs", "pub const STARTER_POSOLOGIES: &[(&str, &str, &str, &str)] = &["),
    "HALF_LIVES": ("src/facets.rs", "const HALF_LIVES: &[(&str, f32, f32)] = &["),
    "NO_HALF_LIFE": ("src/facets.rs", "const NO_HALF_LIFE: &[(&str, Unknown)] = &["),
    "BEYOND": ("src/facets.rs", "const BEYOND: &[(&str, &str)] = &["),
    "IMPACTS": ("src/facets.rs", "const IMPACTS: &[(&str, Organ, Effect, Grade, &str)] = &["),
    "RENAL": ("src/renal.rs", "pub const TABLE: &[Adaptation] = &["),
    "HEPATIC": ("src/hepatic.rs", "pub const TABLE: &[Adaptation] = &["),
    "ELDERLY": ("src/elderly.rs", "pub const TABLE: &[Inappropriate] = &["),
    "CYP": ("src/cyp.rs", "pub const TABLE: &[Profile] = &["),
    "CRUSH": ("src/crush.rs", "pub const TABLE: &[Rule] = &["),
    "GRAVIDITY": ("src/gravidity.rs", "pub const TABLE: &[Advice] = &["),
    "WATCHES": ("src/surveillance.rs", "pub const WATCHES: &[Watch] = &["),
    "CLASSES": ("src/classes.rs", "pub const CLASSES: &[Class] = &["),
    "CONDUITE": ("src/db.rs", "pub const STARTER_CONDUITE: &[(&str, &str, &str)] = &["),
}


def parse(path):
    sections, cur = {}, None
    for line in open(path, encoding="utf-8"):
        m = re.match(r"^## ([A-Z_]+)(?:\s*@\s*(.+?))?\s*$", line)
        if m:
            cur = m.group(1) + ("@" + m.group(2) if m.group(2) else "")
            if cur == "NOTES":
                cur = None
                continue
            if cur.split("@")[0] not in TARGETS:
                sys.exit(f"{path}: unknown section {cur}")
            sections.setdefault(cur, [])
            continue
        if cur is None:
            continue
        sections[cur].append(line)
    return {k: "".join(v).strip("\n") for k, v in sections.items() if "".join(v).strip()}


def main():
    repo = sys.argv[1]
    merged = {}
    for p in sys.argv[2:]:
        for k, v in parse(p).items():
            merged.setdefault(k, []).append(v)
    files = {}
    for k, chunks in merged.items():
        base, _, before = k.partition("@")
        f, head = TARGETS[base]
        if f not in files:
            files[f] = open(os.path.join(repo, f), encoding="utf-8").read()
        src = files[f]
        i = src.index(head)
        j = src.index("\n];", i)
        if base == "CONDUITE" and before:
            key = before.rstrip('"')
            j = src.index('\n    (\n        "' + key + '",', i, j)
        elif before.startswith("//"):
            j = src.index("\n    " + before, i, j)
        elif before:
            lab = src.index('label: "' + before, i, j)
            # back to the line opening the struct
            j = src.rindex("\n    ", i, src.rindex("{\n", i, lab))
            j = src.rindex("\n", i, j + 1)
        text = "\n".join(chunks)
        # indent to four spaces if the snippet was written flush-left
        lines = text.split("\n")
        if lines and not lines[0].startswith("    "):
            lines = [("    " + l) if l.strip() else l for l in lines]
        files[f] = src[:j] + "\n" + "\n".join(lines) + src[j:]
        print(f"{k}: {len(chunks)} chunk(s) into {f}")
    for f, src in files.items():
        open(os.path.join(repo, f), "w", encoding="utf-8").write(src)


main()
