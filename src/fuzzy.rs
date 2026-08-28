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
pub fn score(query: &str, target: &str) -> Option<i32> {
    score_with_indices(query, target).map(|(s, _)| s)
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
    fn match_indices_point_at_the_matched_chars() {
        let (_, idx) = super::score_with_indices("jd", "Jean Dupont").unwrap();
        assert_eq!(idx, vec![0, 5]);
        // Char indices, not byte indices: 'é' counts as one position,
        // and the greedy matcher takes the first 'l' (in "Hélène").
        let (_, idx) = super::score_with_indices("hl", "Hélène Lefèvre").unwrap();
        assert_eq!(idx, vec![0, 2]);
    }
}
