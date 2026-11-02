//! "Did you mean" suggestions for misspelled names.

/// The candidate nearest to `written` by edit distance (see [`distance`]), if it is close enough:
/// at most 3 edits, and at most one per three characters of `written`. Ties go
/// to the alphabetically first candidate.
pub fn closest<'a>(
    written: &str,
    candidates: impl IntoIterator<Item = &'a str>,
) -> Option<&'a str> {
    let (distance, name) = candidates
        .into_iter()
        .map(|name| (distance(written, name), name))
        .min()?;
    (distance <= 3 && 3 * distance <= written.len()).then_some(name)
}

/// Edit distance where an insertion, a deletion, a substitution or a swap of two
/// neighbouring characters each count as one edit (optimal string alignment).
fn distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut d = vec![vec![0; b.len() + 1]; a.len() + 1];
    for (i, row) in d.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, cell) in d[0].iter_mut().enumerate() {
        *cell = j;
    }
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            d[i][j] = (d[i - 1][j] + 1)
                .min(d[i][j - 1] + 1)
                .min(d[i - 1][j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                d[i][j] = d[i][j].min(d[i - 2][j - 2] + 1);
            }
        }
    }
    d[a.len()][b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggest_rule() {
        assert_eq!(
            closest("Europe/Stokholm", ["Europe/Stockholm", "Europe/Oslo"]),
            Some("Europe/Stockholm")
        );
        // Two edits on a two-letter name is too far-fetched.
        assert_eq!(closest("ab", ["xy"]), None);
        assert_eq!(closest("SIGTREM", ["SIGTERM", "SIGTRAP"]), Some("SIGTERM"));
        assert_eq!(closest("Nowhere/Atall", ["Europe/Oslo"]), None);
        // A swap of two neighbours is one edit.
        assert_eq!(closest("TREM", ["TERM", "TRAP"]), Some("TERM"));
        assert_eq!(closest("FOO", ["FPE"]), None);
        // Ties go to the alphabetically first name.
        assert_eq!(closest("abcd", ["abcf", "abce"]), Some("abce"));
    }
}
