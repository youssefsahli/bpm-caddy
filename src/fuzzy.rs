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
    fn match_indices_point_at_the_matched_chars() {
        let (_, idx) = super::score_with_indices("jd", "Jean Dupont").unwrap();
        assert_eq!(idx, vec![0, 5]);
        // Char indices, not byte indices: 'é' counts as one position,
        // and the greedy matcher takes the first 'l' (in "Hélène").
        let (_, idx) = super::score_with_indices("hl", "Hélène Lefèvre").unwrap();
        assert_eq!(idx, vec![0, 2]);
    }
}
