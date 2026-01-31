//! Fuzzy string matching for "did you mean" suggestions

use strsim::jaro_winkler;

/// Find best match from a list of valid options
/// Returns Some(suggestion) if confidence > threshold, None otherwise
pub fn find_best_match<'a>(
    input: &str,
    valid_options: &[&'a str],
    threshold: f64,
) -> Option<&'a str> {
    valid_options
        .iter()
        .map(|&option| (option, jaro_winkler(input, option)))
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .and_then(|(option, score)| {
            if score >= threshold {
                Some(option)
            } else {
                None
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match() {
        let options = &["low", "medium", "high", "urgent"];
        assert_eq!(find_best_match("high", options, 0.8), Some("high"));
    }

    #[test]
    fn test_close_match() {
        let options = &["low", "medium", "high", "urgent"];
        assert_eq!(find_best_match("hgh", options, 0.8), Some("high"));
    }

    #[test]
    fn test_typo_match() {
        let options = &["open", "in_progress", "review", "closed"];
        assert_eq!(
            find_best_match("in_progres", options, 0.8),
            Some("in_progress")
        );
    }

    #[test]
    fn test_no_match() {
        let options = &["low", "medium", "high", "urgent"];
        // "xyz" is too different from any option
        assert_eq!(find_best_match("xyz", options, 0.8), None);
    }

    #[test]
    fn test_case_insensitive_preparation() {
        let options = &["low", "medium", "high", "urgent"];
        // Caller should lowercase before calling
        assert_eq!(find_best_match("HIGH", options, 0.8), None);
        assert_eq!(find_best_match("high", options, 0.8), Some("high"));
    }
}
