/// Fuzzy matching utilities for worktree names
use crate::errors::{HnError, Result};

/// Match result with score
#[derive(Debug, Clone)]
pub struct FuzzyMatch {
    pub name: String,
    pub score: i32,
}

/// Find the best fuzzy match for a query string
///
/// Returns the best match if one exists, or an error if no matches found
/// or if the query is ambiguous (multiple equally good matches).
pub fn find_best_match(query: &str, candidates: &[String]) -> Result<String> {
    if candidates.is_empty() {
        return Err(HnError::WorktreeNotFound(query.to_string()));
    }

    // First try exact match
    for candidate in candidates {
        if candidate == query {
            return Ok(candidate.clone());
        }
    }

    // Try prefix match
    let prefix_matches: Vec<&String> = candidates
        .iter()
        .filter(|c| c.starts_with(query))
        .collect();

    if prefix_matches.len() == 1 {
        return Ok(prefix_matches[0].clone());
    } else if prefix_matches.len() > 1 {
        return Err(HnError::AmbiguousWorktreeName(
            query.to_string(),
            prefix_matches.iter().map(|s| s.to_string()).collect(),
        ));
    }

    // Try fuzzy matching with scoring
    let mut matches: Vec<FuzzyMatch> = candidates
        .iter()
        .filter_map(|candidate| {
            fuzzy_score(query, candidate).map(|score| FuzzyMatch {
                name: candidate.clone(),
                score,
            })
        })
        .collect();

    if matches.is_empty() {
        return Err(HnError::WorktreeNotFound(query.to_string()));
    }

    // Sort by score (highest first)
    matches.sort_by(|a, b| b.score.cmp(&a.score));

    // Check if there are multiple matches with the same top score
    if matches.len() > 1 && matches[0].score == matches[1].score {
        let ambiguous: Vec<String> = matches
            .iter()
            .filter(|m| m.score == matches[0].score)
            .map(|m| m.name.clone())
            .collect();
        return Err(HnError::AmbiguousWorktreeName(query.to_string(), ambiguous));
    }

    Ok(matches[0].name.clone())
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
        assert_eq!(find_best_match("feature-x", &candidates).unwrap(), "feature-x");
    }

    #[test]
    fn test_prefix_match() {
        let candidates = vec!["feature-auth".to_string(), "feature-billing".to_string()];
        assert_eq!(find_best_match("feature-a", &candidates).unwrap(), "feature-auth");
    }

    #[test]
    fn test_fuzzy_match() {
        let candidates = vec!["feature-authentication".to_string(), "fix-auth-bug".to_string()];
        let result = find_best_match("fauth", &candidates).unwrap();
        assert_eq!(result, "feature-authentication");
    }

    #[test]
    fn test_no_match() {
        let candidates = vec!["feature-x".to_string()];
        assert!(find_best_match("nonexistent", &candidates).is_err());
    }
}
