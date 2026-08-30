//! Diacritic-insensitive subsequence matching: typing "jndp" retrieves
//! "Jean Dupont".

/// Lowercase and strip the accents used in French names.
fn fold(c: char) -> char {
    // Full Unicode lowercasing: 'É' → 'é' ('to_ascii_lowercase' would
    // leave uppercase accented letters untouched).
    let c = c.to_lowercase().next().unwrap_or(c);
    match c {
        'à' | 'â' | 'ä' => 'a',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'î' | 'ï' => 'i',
        'ô' | 'ö' => 'o',
        'ù' | 'û' | 'ü' => 'u',
        'ç' => 'c',
        c => c,
    }
}

/// Score `query` as a subsequence of `target`. Higher is better; `None`
/// means no match. Word-start and consecutive matches score extra, so
/// initials ("jd") and prefixes rank naturally.
///
/// This is the hottest function in the application: a keystroke in the
/// medicine search asks it about 850 fiches on four fields each, the
/// codex and the dispositifs do the same on their own lists, and the
/// tables ask it about every cell. It therefore allocates **nothing** —
/// it walks both sides as iterators rather than collecting them into
/// `Vec<char>` the way [`score_with_indices`] must, and stops the
/// moment the query is exhausted instead of scanning the rest of a
/// monograph field that can no longer change the answer.
pub fn score(query: &str, target: &str) -> Option<i32> {
    let mut q = query
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(fold)
        .peekable();
    if q.peek().is_none() {
        return Some(0);
    }
    let mut total = 0;
    // The index of the previous match, so a run of consecutive letters
    // scores, and the character just before the one being looked at, so
    // a match at the start of a word scores.
    let mut prev = usize::MAX;
    let mut before: Option<char> = None;
    for (i, c) in target.chars().map(fold).enumerate() {
        if q.peek() == Some(&c) {
            let word_start = before.is_none_or(|p| !p.is_alphanumeric());
            total += 1
                + if word_start { 3 } else { 0 }
                + if prev != usize::MAX && i == prev + 1 {
                    2
                } else {
                    0
                };
            prev = i;
            q.next();
            if q.peek().is_none() {
                return Some(total);
            }
        }
        before = Some(c);
    }
    None
}

/// Lowercased, accent-stripped copy of `s` — a collation key so
/// "Lefèvre" sorts with "Lefevre" instead of after "Z".
pub fn sort_key(s: &str) -> String {
    s.chars().map(fold).collect()
}

/// Does `hay` — **already folded**, by [`sort_key`] — contain `needle`
/// once folded? Without folding a copy of the needle first.
///
/// This is what the three rule engines ask, and they ask it a great many
/// times: the revue runs fifty-five rules over an ordonnance, most of
/// them naming half a dozen words, against ten treatments — and the
/// dashboard runs the whole thing for every file in the base. Written as
/// `hay.contains(&sort_key(w))` that was one `String` per word *per
/// treatment*, some ten thousand allocations per patient. The needles
/// are short and the haystacks are a line long, so a plain scan with the
/// folding done on the fly costs nothing and allocates nothing.
pub fn contains_folded(hay: &str, needle: &str) -> bool {
    let Some(first) = needle.chars().next().map(fold) else {
        return true;
    };
    for (i, c) in hay.char_indices() {
        if c != first {
            continue;
        }
        let mut rest = hay[i..].chars();
        if needle.chars().map(fold).all(|n| rest.next() == Some(n)) {
            return true;
        }
    }
    false
}

/// The same question when **neither** side has been folded.
///
/// [`contains_folded`] wants a haystack that has already been through
/// [`sort_key`], and that is a trap the moment the haystack is a label
/// read straight off a row: the folded needle is then compared against a
/// raw « S », and « Skenan » does not find « Skenan ». Folding the
/// haystack first would answer correctly and allocate a `String` per row
/// per frame, which is what this house forbids in a draw path — so this
/// folds both sides as it walks, and allocates nothing either.
pub fn contains_loose(hay: &str, needle: &str) -> bool {
    let Some(first) = needle.chars().next().map(fold) else {
        return true;
    };
    for (i, c) in hay.char_indices() {
        if fold(c) != first {
            continue;
        }
        let mut rest = hay[i..].chars().map(fold);
        if needle.chars().map(fold).all(|n| rest.next() == Some(n)) {
            return true;
        }
    }
    false
}

/// Are these the same word, ignoring case, accents and the space
/// around them? Without allocating a folded copy of either.
///
/// `sort_key(a) == sort_key(b)` is the same answer and two `String`s,
/// which is the wrong shape for a question asked over eight hundred and
/// fifty cards — « les autres de la même classe », « les autres marques
/// de cette molécule » — every time one is opened.
pub fn eq_folded(a: &str, b: &str) -> bool {
    let mut a = a.trim().chars().map(fold);
    let mut b = b.trim().chars().map(fold);
    loop {
        match (a.next(), b.next()) {
            (None, None) => return true,
            (x, y) if x == y => {}
            _ => return false,
        }
    }
}

/// The letter `s` files under in an A–Z index: the first letter,
/// accent-folded and capitalised, or `#` for a name that starts with a
/// digit or a symbol.
///
/// Folding matters here: « Élugan » belongs under E with the rest of
/// its neighbours, not in a section of its own between Z and nothing.
pub fn index_letter(s: &str) -> char {
    s.chars()
        .find(|c| !c.is_whitespace())
        .map(fold)
        .filter(|c| c.is_alphabetic())
        .map(|c| c.to_ascii_uppercase())
        .unwrap_or('#')
}

/// Like [`score`], also returning the char indices of `target` that
/// matched (ascending), so the UI can highlight them.
pub fn score_with_indices(query: &str, target: &str) -> Option<(i32, Vec<usize>)> {
    let q: Vec<char> = query
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(fold)
        .collect();
    if q.is_empty() {
        return Some((0, Vec::new()));
    }
    let t: Vec<char> = target.chars().map(fold).collect();
    let mut qi = 0;
    let mut total = 0;
    let mut prev = usize::MAX;
    let mut indices = Vec::with_capacity(q.len());
    for (i, &c) in t.iter().enumerate() {
        if qi < q.len() && c == q[qi] {
            let word_start = i == 0 || !t[i - 1].is_alphanumeric();
            total += 1
                + if word_start { 3 } else { 0 }
                + if prev != usize::MAX && i == prev + 1 {
                    2
                } else {
                    0
                };
            prev = i;
            indices.push(i);
            qi += 1;
        }
    }
    (qi == q.len()).then_some((total, indices))
}

#[cfg(test)]
mod tests {
    use super::score;

    #[test]
    fn initials_match() {
        assert!(score("jndp", "Jean Dupont").is_some());
        assert!(score("jd", "Jean Dupont").is_some());
    }

    #[test]
    fn diacritics_are_ignored() {
        assert!(score("helene", "Hélène Lefèvre").is_some());
        // Uppercase accented letters must fold too.
        assert!(score("emile", "ÉMILE LEFÈVRE").is_some());
        assert!(score("lefevre", "LEFÈVRE Émile").is_some());
    }

    #[test]
    fn non_subsequence_fails() {
        assert!(score("xyz", "Jean Dupont").is_none());
    }

    #[test]
    fn word_starts_rank_higher() {
        // "jd" as initials of "Jean Dupont" must beat an interior match.
        let initials = score("jd", "Jean Dupont").unwrap();
        let interior = score("jd", "Amjad Hamidi").unwrap();
        assert!(initials > interior);
    }

    #[test]
    fn empty_query_matches_everything() {
        assert_eq!(score("", "anyone"), Some(0));
    }

    #[test]
    fn sort_key_folds_case_and_accents() {
        assert_eq!(super::sort_key("Lefèvre"), "lefevre");
        assert!(super::sort_key("Lefèvre") < super::sort_key("Martin"));
        assert!(super::sort_key("Émile") < super::sort_key("Zoé"));
    }

    /// The same answer as comparing two `sort_key`s, and no allocation.
    /// That equivalence is the contract, so it is what is asserted.
    #[test]
    fn folded_equality_ignores_case_accents_and_the_space_around() {
        for (a, b) in [
            ("AOD", "aod"),
            ("Bêtabloquant", "betabloquant"),
            ("  IEC ", "iec"),
            ("acide zolédronique", "ACIDE ZOLEDRONIQUE"),
            ("", "   "),
        ] {
            assert!(super::eq_folded(a, b), "{a:?} = {b:?}");
        }
        for (a, b) in [
            ("AOD", "AVK"),
            // A prefix is not the word: « statine » must not answer for
            // « statines », nor « IEC » for « IECA ».
            ("statine", "statines"),
            ("IEC", "IECA"),
            ("", "a"),
        ] {
            assert!(!super::eq_folded(a, b), "{a:?} ≠ {b:?}");
        }
        // And it agrees with the two-allocation form it replaces.
        for (a, b) in [("Élugan", "elugan"), ("AOD", "AVK"), ("x", "x")] {
            assert_eq!(
                super::eq_folded(a, b),
                super::sort_key(a.trim()) == super::sort_key(b.trim()),
                "{a:?} / {b:?}"
            );
        }
    }

    #[test]
    fn the_index_letter_folds_accents_and_names_the_rest() {
        assert_eq!(super::index_letter("Aclasta"), 'A');
        assert_eq!(super::index_letter("aclasta"), 'A');
        // Folded, so an accented brand files with its neighbours
        // instead of in a section of its own.
        assert_eq!(super::index_letter("Élugan"), 'E');
        assert_eq!(super::index_letter("  Zovirax"), 'Z');
        // Everything that is not a letter shares one drawer.
        assert_eq!(super::index_letter("5-FU"), '#');
        assert_eq!(super::index_letter(""), '#');
        assert_eq!(super::index_letter("   "), '#');
    }

    #[test]
    fn the_fast_score_and_the_highlighting_one_never_disagree() {
        // `score` is written a second time, without allocating, because
        // it is asked about several thousand strings per keystroke.
        // Two implementations of one rule drift unless something holds
        // them together: this is that thing. Every pair below is a case
        // that separated them at some point — an empty query, a query
        // longer than its target, accents on both sides, a match that
        // completes before the end of the target, and a target whose
        // first character is not a letter.
        let queries = [
            "", "  ", "a", "jd", "jndp", "elq", "eliquis", "xyz", "é", "eee", "5f", "l e",
            "lefevre", "co",
        ];
        let targets = [
            "",
            "a",
            "Jean Dupont",
            "Hélène Lefèvre",
            "ÉMILE LEFÈVRE",
            "5-FU",
            "Eliquis",
            "  Zovirax",
            "acide acétylsalicylique",
            "Co-Renitec 20 mg/12,5 mg",
            "aaaaaaaaaa",
        ];
        for q in queries {
            for t in targets {
                assert_eq!(
                    super::score(q, t),
                    super::score_with_indices(q, t).map(|(s, _)| s),
                    "score(«{q}», «{t}») diverge"
                );
            }
        }
    }

    #[test]
    fn the_folding_search_answers_what_the_folded_copy_would_have() {
        // Same rule written twice again, and held together the same way:
        // `contains_folded(hay, n)` must answer exactly what
        // `hay.contains(&sort_key(n))` answered before it.
        let hays = [
            "coversyl perindopril iec",
            "aldactone spironolactone anti-aldosterone diuretique",
            "",
            "eee",
            "methotrexate",
            "5-fu antimetabolite",
        ];
        let needles = [
            "IEC",
            "iec",
            "éplérénone",
            "spironolactone",
            "ALDOSTÉRONE",
            "",
            "e",
            "eeee",
            "méthotrexate",
            "5-FU",
            "z",
        ];
        for hay in hays {
            for n in needles {
                assert_eq!(
                    super::contains_folded(hay, n),
                    hay.contains(&super::sort_key(n)),
                    "contains_folded(«{hay}», «{n}») diverge"
                );
            }
        }
        // The haystack is folded by the caller; the needle is not, and
        // that is the whole point.
        assert!(super::contains_folded(
            &super::sort_key("Éplérénone 25 mg"),
            "ÉPLÉRÉNONE"
        ));
        // An empty needle is contained in everything, `str::contains`
        // included.
        assert!(super::contains_folded("", ""));
    }

    /// `contains_loose` answers the same question with **neither** side
    /// folded — and the trap it exists for is real.
    ///
    /// Given a haystack straight off a row, `contains_folded` compares a
    /// folded needle against a raw « S » and finds nothing: « Skenan »
    /// does not find « Skenan ». A search field wired that way looks
    /// like a search field, returns nothing, and nobody can tell whether
    /// the base is empty or the search is broken.
    #[test]
    fn the_loose_search_folds_both_sides_and_the_folded_one_does_not() {
        // The trap, shown rather than described.
        assert!(!super::contains_folded("Skenan LP 30 mg", "skenan"));
        assert!(super::contains_loose("Skenan LP 30 mg", "skenan"));
        assert!(super::contains_loose("Skenan LP 30 mg", "SKENAN"));
        // Accents on either side, or on both.
        assert!(super::contains_loose(
            "Méthadone AP-HP gélule 40 mg",
            "methadone"
        ));
        assert!(super::contains_loose("Methadone gelule", "gélule"));
        assert!(super::contains_loose("Oxycodone", "oxy"));
        assert!(!super::contains_loose("Oxycodone", "oxyz"));
        // And it agrees with the folded pair wherever both apply.
        for hay in ["Skenan LP 30 mg", "Fentanyl transmuqueux", "", "Durogesic"] {
            for n in ["", "z", "trans", "DUROGÉSIC", "30 mg"] {
                assert_eq!(
                    super::contains_loose(hay, n),
                    super::contains_folded(&super::sort_key(hay), n),
                    "contains_loose(«{hay}», «{n}») diverge"
                );
            }
        }
    }

    #[test]
    fn match_indices_point_at_the_matched_chars() {
        let (_, idx) = super::score_with_indices("jd", "Jean Dupont").unwrap();
        assert_eq!(idx, vec![0, 5]);
        // Char indices, not byte indices: 'é' counts as one position,
        // and the greedy matcher takes the first 'l' (in "Hélène").
        let (_, idx) = super::score_with_indices("hl", "Hélène Lefèvre").unwrap();
        assert_eq!(idx, vec![0, 2]);
    }
}
