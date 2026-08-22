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
    let q: Vec<char> = query
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(fold)
        .collect();
    if q.is_empty() {
        return Some(0);
    }
    let t: Vec<char> = target.chars().map(fold).collect();
    let mut qi = 0;
    let mut total = 0;
    let mut prev = usize::MAX;
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
            qi += 1;
        }
    }
    (qi == q.len()).then_some(total)
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
}
