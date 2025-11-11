/// Fuzzy matching utilities for worktree names
use crate::errors::{HnError, Result};

/// Find the best fuzzy match for a query string
///
/// Returns the best match if one exists, or an error if no matches found
/// or if the query is ambiguous (multiple equally good matches).
pub fn find_best_match(query: &str, candidates: &[String]) -> Result<String> {
    if candidates.is_empty() {
        return Err(HnError::WorktreeNotFound(query.to_string()));
    }

    // First try exact match (lazy - returns immediately on first match)
    if let Some(exact) = candidates.iter().find(|c| c.as_str() == query) {
        return Ok(exact.clone());
    }

    // Try case-insensitive prefix match with lazy evaluation
    let query_lower = query.to_lowercase();
    let mut prefix_iter = candidates
        .iter()
        .filter(|c| c.to_lowercase().starts_with(&query_lower));

    match (prefix_iter.next(), prefix_iter.next()) {
        (Some(first), None) => {
            // Exactly one prefix match - return it
            return Ok(first.clone());
        }
        (Some(first), Some(second)) => {
            // Multiple prefix matches - collect them for error message
            let mut all_prefix: Vec<&str> = vec![first.as_str(), second.as_str()];
            all_prefix.extend(prefix_iter.map(|s| s.as_str()));
            return Err(HnError::AmbiguousWorktreeName(
                query.to_string(),
                all_prefix.iter().map(|s| s.to_string()).collect(),
            ));
        }
        _ => { /* No prefix matches, continue */ }
    }

    // Try case-insensitive substring match with lazy evaluation
    let mut substring_iter = candidates
        .iter()
        .filter(|c| c.to_lowercase().contains(&query_lower));

    match (substring_iter.next(), substring_iter.next()) {
        (Some(first), None) => {
            // Exactly one substring match - return it
            return Ok(first.clone());
        }
        (Some(_), Some(_)) => {
            // Multiple substring matches - continue to fuzzy scoring to disambiguate
        }
        _ => { /* No substring matches, continue */ }
    }

    // Try fuzzy matching with scoring
    let mut matches: Vec<(&str, i32)> = candidates
        .iter()
        .filter_map(|candidate| {
            fuzzy_score(query, candidate).map(|score| (candidate.as_str(), score))
        })
        .collect();

    if matches.is_empty() {
        return Err(HnError::WorktreeNotFound(query.to_string()));
    }

    // Sort by score (highest first)
    matches.sort_by(|a, b| b.1.cmp(&a.1));

    // Check if there are multiple matches with the same top score
    if matches.len() > 1 && matches[0].1 == matches[1].1 {
        let ambiguous: Vec<String> = matches
            .iter()
            .filter(|m| m.1 == matches[0].1)
            .map(|m| m.0.to_string())
            .collect();
        return Err(HnError::AmbiguousWorktreeName(query.to_string(), ambiguous));
    }

    Ok(matches[0].0.to_string())
}

/// Calculate fuzzy match score
///
/// Returns Some(score) if all query characters are found in order in candidate,
/// None otherwise. Higher scores are better matches.
fn fuzzy_score(query: &str, candidate: &str) -> Option<i32> {
    let query_lower = query.to_lowercase();
    let candidate_lower = candidate.to_lowercase();

    let mut query_chars = query_lower.chars().peekable();
    let mut candidate_chars = candidate_lower.chars().enumerate();

    let mut score = 0;
    let mut last_match_pos = 0;
    let mut consecutive_matches = 0;

    while let Some(query_char) = query_chars.peek() {
        let mut found = false;

        for (pos, candidate_char) in candidate_chars.by_ref() {
            if *query_char == candidate_char {
                found = true;
                query_chars.next();

                // Bonus for consecutive matches
                if pos == last_match_pos + 1 {
                    consecutive_matches += 1;
                    score += 5 + consecutive_matches; // Increasing bonus for longer runs
                } else {
                    consecutive_matches = 0;
                    score += 1;
                }

                // Bonus for matching at word boundaries (after -, _, or start)
                if pos == 0 {
                    score += 10; // Start of string
                } else if let Some(prev_char) = candidate_lower.chars().nth(pos - 1) {
                    if prev_char == '-' || prev_char == '_' || prev_char == '/' {
                        score += 5; // Word boundary
                    }
                }

                last_match_pos = pos;
                break;
            }
        }

        if !found {
            return None; // Not all query characters found
        }
    }

    Some(score)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match() {
        let candidates = vec!["feature-x".to_string(), "feature-y".to_string()];
        assert_eq!(
            find_best_match("feature-x", &candidates).unwrap(),
            "feature-x"
        );
    }

    #[test]
    fn test_prefix_match() {
        let candidates = vec!["feature-auth".to_string(), "feature-billing".to_string()];
        assert_eq!(
            find_best_match("feature-a", &candidates).unwrap(),
            "feature-auth"
        );
    }

    #[test]
    fn test_fuzzy_match() {
        let candidates = vec![
            "feature-authentication".to_string(),
            "fix-login-bug".to_string(),
        ];
        let result = find_best_match("fauth", &candidates).unwrap();
        // Should match "feature-authentication" because it has consecutive "aut" in "authentication"
        assert_eq!(result, "feature-authentication");
    }

    #[test]
    fn test_no_match() {
        let candidates = vec!["feature-x".to_string()];
        assert!(find_best_match("nonexistent", &candidates).is_err());
    }
}
