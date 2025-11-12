// Worktree tagging system (v0.6)
//
// Allows users to organize worktrees with tags for better filtering and organization

use crate::errors::{HnError, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

/// Tag index for fast lookup
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TagIndex {
    /// Map from tag to set of worktree names
    pub tags: HashMap<String, HashSet<String>>,
    /// Map from worktree name to set of tags
    pub worktrees: HashMap<String, HashSet<String>>,
}

impl TagIndex {
    /// Load tag index from disk
    pub fn load(state_dir: &Path) -> Result<Self> {
        let index_path = state_dir.join("tag-index.json");

        if !index_path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&index_path)?;
        let index: TagIndex = serde_json::from_str(&content)
            .map_err(|e| HnError::StateError(format!("Failed to parse tag index: {}", e)))?;

        Ok(index)
    }

    /// Save tag index to disk
    pub fn save(&self, state_dir: &Path) -> Result<()> {
        let index_path = state_dir.join("tag-index.json");
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&index_path, content)?;
        Ok(())
    }

    /// Add tags to a worktree
    pub fn add_tags(&mut self, worktree: &str, tags: &[String]) {
        // Add to worktree -> tags mapping
        let worktree_tags = self.worktrees.entry(worktree.to_string()).or_default();
        for tag in tags {
            worktree_tags.insert(tag.clone());
        }

        // Add to tag -> worktrees mapping
        for tag in tags {
            let tag_worktrees = self.tags.entry(tag.clone()).or_default();
            tag_worktrees.insert(worktree.to_string());
        }
    }

    /// Remove tags from a worktree
    #[allow(dead_code)]
    pub fn remove_tags(&mut self, worktree: &str, tags: &[String]) {
        // Remove from worktree -> tags mapping
        if let Some(worktree_tags) = self.worktrees.get_mut(worktree) {
            for tag in tags {
                worktree_tags.remove(tag);
            }
            if worktree_tags.is_empty() {
                self.worktrees.remove(worktree);
            }
        }

        // Remove from tag -> worktrees mapping
        for tag in tags {
            if let Some(tag_worktrees) = self.tags.get_mut(tag) {
                tag_worktrees.remove(worktree);
                if tag_worktrees.is_empty() {
                    self.tags.remove(tag);
                }
            }
        }
    }

    /// Remove all tags from a worktree
    #[allow(dead_code)]
    pub fn remove_worktree(&mut self, worktree: &str) {
        if let Some(tags) = self.worktrees.remove(worktree) {
            for tag in tags {
                if let Some(tag_worktrees) = self.tags.get_mut(&tag) {
                    tag_worktrees.remove(worktree);
                    if tag_worktrees.is_empty() {
                        self.tags.remove(&tag);
                    }
                }
            }
        }
    }

    /// Get all tags for a worktree
    pub fn get_worktree_tags(&self, worktree: &str) -> Vec<String> {
        self.worktrees
            .get(worktree)
            .map(|tags| {
                let mut tags: Vec<_> = tags.iter().cloned().collect();
                tags.sort();
                tags
            })
            .unwrap_or_default()
    }

    /// Get all worktrees with a specific tag
    pub fn get_worktrees_by_tag(&self, tag: &str) -> Vec<String> {
        self.tags
            .get(tag)
            .map(|worktrees| {
                let mut worktrees: Vec<_> = worktrees.iter().cloned().collect();
                worktrees.sort();
                worktrees
            })
            .unwrap_or_default()
    }

    /// Get all tags sorted by name
    pub fn get_all_tags(&self) -> Vec<String> {
        let mut tags: Vec<_> = self.tags.keys().cloned().collect();
        tags.sort();
        tags
    }

    /// Get tag count (number of worktrees with this tag)
    pub fn get_tag_count(&self, tag: &str) -> usize {
        self.tags.get(tag).map(|w| w.len()).unwrap_or(0)
    }
}

/// Add tags to a worktree
pub fn add_tags(state_dir: &Path, worktree: &str, tags: &[String]) -> Result<()> {
    // Validate tag names
    for tag in tags {
        validate_tag_name(tag)?;
    }

    // Load index
    let mut index = TagIndex::load(state_dir)?;

    // Add tags
    index.add_tags(worktree, tags);

    // Save index
    index.save(state_dir)?;

    // Also save tags to worktree's tags file for redundancy
    let worktree_state_dir = state_dir.join(worktree);
    if worktree_state_dir.exists() {
        let tags_file = worktree_state_dir.join("tags.txt");
        let all_tags = index.get_worktree_tags(worktree);
        fs::write(&tags_file, all_tags.join("\n"))?;
    }

    Ok(())
}

/// Remove tags from a worktree
#[allow(dead_code)]
pub fn remove_tags(state_dir: &Path, worktree: &str, tags: &[String]) -> Result<()> {
    // Load index
    let mut index = TagIndex::load(state_dir)?;

    // Remove tags
    index.remove_tags(worktree, tags);

    // Save index
    index.save(state_dir)?;

    // Update worktree's tags file
    let worktree_state_dir = state_dir.join(worktree);
    if worktree_state_dir.exists() {
        let tags_file = worktree_state_dir.join("tags.txt");
        let all_tags = index.get_worktree_tags(worktree);
        if all_tags.is_empty() {
            let _ = fs::remove_file(&tags_file); // Ignore if doesn't exist
        } else {
            fs::write(&tags_file, all_tags.join("\n"))?;
        }
    }

    Ok(())
}

/// Get all tags for a worktree
pub fn get_worktree_tags(state_dir: &Path, worktree: &str) -> Result<Vec<String>> {
    let index = TagIndex::load(state_dir)?;
    Ok(index.get_worktree_tags(worktree))
}

/// Get all worktrees with a specific tag
pub fn get_worktrees_by_tag(state_dir: &Path, tag: &str) -> Result<Vec<String>> {
    let index = TagIndex::load(state_dir)?;
    Ok(index.get_worktrees_by_tag(tag))
}

/// List all tags
pub fn list_all_tags(state_dir: &Path) -> Result<HashMap<String, usize>> {
    let index = TagIndex::load(state_dir)?;
    let mut result = HashMap::new();

    for tag in index.get_all_tags() {
        let count = index.get_tag_count(&tag);
        result.insert(tag, count);
    }

    Ok(result)
}

/// Validate tag name
fn validate_tag_name(tag: &str) -> Result<()> {
    if tag.is_empty() {
        return Err(HnError::ValidationError("Tag cannot be empty".to_string()));
    }

    if tag.len() > 50 {
        return Err(HnError::ValidationError(
            "Tag cannot be longer than 50 characters".to_string(),
        ));
    }

    // Tags should be alphanumeric with hyphens and underscores
    if !tag
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err(HnError::ValidationError(
            "Tag can only contain letters, numbers, hyphens, and underscores".to_string(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_tag_index() {
        let temp_dir = TempDir::new().unwrap();
        let state_dir = temp_dir.path();

        // Create index
        let mut index = TagIndex::default();

        // Add tags
        index.add_tags("wt1", &["backend".to_string(), "urgent".to_string()]);
        index.add_tags("wt2", &["frontend".to_string()]);
        index.add_tags("wt3", &["backend".to_string()]);

        // Check mappings
        assert_eq!(index.get_worktree_tags("wt1"), vec!["backend", "urgent"]);
        assert_eq!(index.get_worktree_tags("wt2"), vec!["frontend"]);

        assert_eq!(index.get_worktrees_by_tag("backend"), vec!["wt1", "wt3"]);
        assert_eq!(index.get_tag_count("backend"), 2);

        // Save and load
        index.save(state_dir).unwrap();
        let loaded = TagIndex::load(state_dir).unwrap();

        assert_eq!(loaded.get_worktree_tags("wt1"), vec!["backend", "urgent"]);
    }

    #[test]
    fn test_tag_validation() {
        assert!(validate_tag_name("backend").is_ok());
        assert!(validate_tag_name("urgent-fix").is_ok());
        assert!(validate_tag_name("test_123").is_ok());

        assert!(validate_tag_name("").is_err());
        assert!(validate_tag_name("a".repeat(51).as_str()).is_err());
        assert!(validate_tag_name("invalid tag").is_err()); // Space not allowed
        assert!(validate_tag_name("invalid/tag").is_err()); // Slash not allowed
    }
}
