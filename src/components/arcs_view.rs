use dioxus::prelude::*;

/// Returns true if `name` should be kept for the given search `query`.
/// An empty / whitespace-only query matches everything; otherwise matching is
/// a case-insensitive substring test.
fn name_matches(name: &str, query: &str) -> bool {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return true;
    }
    name.to_lowercase().contains(&q)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_query_matches_everything() {
        assert!(name_matches("Rocketeer", ""));
        assert!(name_matches("Anything", "   "));
    }

    #[test]
    fn matches_case_insensitive_substring() {
        assert!(name_matches("Rocketeer", "rocket"));
        assert!(name_matches("rocketeer", "ROCKET"));
        assert!(name_matches("Tick Bot", "bot"));
    }

    #[test]
    fn non_match_returns_false() {
        assert!(!name_matches("Bombardier", "rocket"));
    }
}
