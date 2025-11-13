/// Worktree registry caching system
///
/// Caches the list of worktrees to avoid expensive VCS queries on every command.
/// The cache is stored in `.hn-state/.registry-cache` and includes:
/// - List of worktrees
/// - Timestamp of last update
/// - Cache validity duration (TTL)
///
/// The cache is automatically invalidated on:
/// - Worktree create/remove operations
/// - Manual cache clear command
/// - TTL expiration
use crate::errors::{HnError, Result};
use crate::vcs::Worktree;
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// Default cache TTL: 30 seconds
const DEFAULT_TTL_SECS: u64 = 30;

/// Cached worktree registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedRegistry {
    pub worktrees: Vec<Worktree>,
    pub cached_at: SystemTime,
    pub ttl: Duration,
}

impl CachedRegistry {
    /// Create a new cached registry
    pub fn new(worktrees: Vec<Worktree>, ttl: Duration) -> Self {
        Self {
            worktrees,
            cached_at: SystemTime::now(),
            ttl,
        }
    }

    /// Check if the cache is still valid
    pub fn is_valid(&self) -> bool {
        let now = SystemTime::now();
        match now.duration_since(self.cached_at) {
            Ok(age) => age < self.ttl,
            Err(_) => false, // Clock went backwards, consider invalid
        }
    }

    /// Get the age of the cache
    pub fn age(&self) -> Result<Duration> {
        SystemTime::now()
            .duration_since(self.cached_at)
            .map_err(|e| HnError::ConfigError(format!("Failed to get cache age: {}", e)))
    }
}

/// Registry cache manager
pub struct RegistryCache {
    cache_file: PathBuf,
    ttl: Duration,
}

impl RegistryCache {
    /// Create a new registry cache
    ///
    /// # Arguments
    /// * `state_dir` - Path to the `.hn-state` directory
    /// * `ttl` - Optional time-to-live for cache entries (defaults to 30 seconds)
    pub fn new(state_dir: &Path, ttl: Option<Duration>) -> Result<Self> {
        // Ensure state directory exists
        if !state_dir.exists() {
            fs::create_dir_all(state_dir).map_err(|e| {
                HnError::ConfigError(format!("Failed to create state directory: {}", e))
            })?;
        }

        let cache_file = state_dir.join(".registry-cache");
        let ttl = ttl.unwrap_or_else(|| Duration::from_secs(DEFAULT_TTL_SECS));

        Ok(Self { cache_file, ttl })
    }

    /// Get cached worktrees if cache is valid
    pub fn get(&self) -> Result<Option<Vec<Worktree>>> {
        if !self.cache_file.exists() {
            return Ok(None);
        }

        // Open cache file with shared lock
        let file = File::open(&self.cache_file)
            .map_err(|e| HnError::ConfigError(format!("Failed to open cache file: {}", e)))?;

        // Acquire shared lock for reading
        file.lock_shared()
            .map_err(|e| HnError::ConfigError(format!("Failed to lock cache file: {}", e)))?;

        // Read and deserialize
        let cached: CachedRegistry = serde_json::from_reader(&file).map_err(|e| {
            // Release lock before returning error
            let _ = file.unlock();
            HnError::ConfigError(format!("Failed to deserialize cache: {}", e))
        })?;

        // Release lock
        file.unlock()
            .map_err(|e| HnError::ConfigError(format!("Failed to unlock cache file: {}", e)))?;

        // Check if cache is still valid
        if cached.is_valid() {
            Ok(Some(cached.worktrees))
        } else {
            Ok(None)
        }
    }

    /// Update the cache with new worktrees
    pub fn set(&self, worktrees: Vec<Worktree>) -> Result<()> {
        let cached = CachedRegistry::new(worktrees, self.ttl);

        // Create or open cache file with exclusive lock
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.cache_file)
            .map_err(|e| {
                HnError::ConfigError(format!("Failed to open cache file for writing: {}", e))
            })?;

        // Acquire exclusive lock for writing
        file.lock_exclusive().map_err(|e| {
            HnError::ConfigError(format!("Failed to lock cache file for writing: {}", e))
        })?;

        // Serialize and write
        serde_json::to_writer_pretty(&file, &cached).map_err(|e| {
            // Release lock before returning error
            let _ = file.unlock();
            HnError::ConfigError(format!("Failed to serialize cache: {}", e))
        })?;

        // Ensure data is written to disk
        file.sync_all().map_err(|e| {
            let _ = file.unlock();
            HnError::ConfigError(format!("Failed to sync cache file: {}", e))
        })?;

        // Release lock
        file.unlock()
            .map_err(|e| HnError::ConfigError(format!("Failed to unlock cache file: {}", e)))?;

        Ok(())
    }

    /// Invalidate the cache (delete the cache file)
    pub fn invalidate(&self) -> Result<()> {
        if self.cache_file.exists() {
            fs::remove_file(&self.cache_file)
                .map_err(|e| HnError::ConfigError(format!("Failed to delete cache file: {}", e)))?;
        }
        Ok(())
    }

    /// Get cache statistics
    pub fn stats(&self) -> Result<Option<CacheStats>> {
        if !self.cache_file.exists() {
            return Ok(None);
        }

        let file = File::open(&self.cache_file)
            .map_err(|e| HnError::ConfigError(format!("Failed to open cache file: {}", e)))?;

        file.lock_shared()
            .map_err(|e| HnError::ConfigError(format!("Failed to lock cache file: {}", e)))?;

        let cached: CachedRegistry = serde_json::from_reader(&file).map_err(|e| {
            let _ = file.unlock();
            HnError::ConfigError(format!("Failed to deserialize cache: {}", e))
        })?;

        file.unlock()
            .map_err(|e| HnError::ConfigError(format!("Failed to unlock cache file: {}", e)))?;

        let metadata = fs::metadata(&self.cache_file)
            .map_err(|e| HnError::ConfigError(format!("Failed to get cache metadata: {}", e)))?;

        Ok(Some(CacheStats {
            valid: cached.is_valid(),
            age: cached.age()?,
            worktree_count: cached.worktrees.len(),
            size_bytes: metadata.len(),
        }))
    }
}

/// Cache statistics
#[derive(Debug)]
pub struct CacheStats {
    pub valid: bool,
    pub age: Duration,
    pub worktree_count: usize,
    pub size_bytes: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_worktrees() -> Vec<Worktree> {
        vec![
            Worktree {
                name: "main".to_string(),
                path: PathBuf::from("/tmp/main"),
                branch: "main".to_string(),
                commit: "abc123".to_string(),
                parent: None,
            },
            Worktree {
                name: "feature-x".to_string(),
                path: PathBuf::from("/tmp/feature-x"),
                branch: "feature/x".to_string(),
                commit: "def456".to_string(),
                parent: Some("main".to_string()),
            },
        ]
    }

    #[test]
    fn test_cache_miss_on_empty() {
        let temp_dir = TempDir::new().unwrap();
        let cache = RegistryCache::new(temp_dir.path(), None).unwrap();

        let result = cache.get().unwrap();
        assert!(result.is_none(), "Cache should miss when empty");
    }

    #[test]
    fn test_cache_hit_when_valid() {
        let temp_dir = TempDir::new().unwrap();
        let cache = RegistryCache::new(temp_dir.path(), Some(Duration::from_secs(60))).unwrap();

        let worktrees = create_test_worktrees();
        cache.set(worktrees.clone()).unwrap();

        let result = cache.get().unwrap();
        assert!(result.is_some(), "Cache should hit when valid");

        let cached_worktrees = result.unwrap();
        assert_eq!(cached_worktrees.len(), 2);
        assert_eq!(cached_worktrees[0].name, "main");
        assert_eq!(cached_worktrees[1].name, "feature-x");
    }

    #[test]
    fn test_cache_miss_when_expired() {
        let temp_dir = TempDir::new().unwrap();
        let cache = RegistryCache::new(temp_dir.path(), Some(Duration::from_millis(1))).unwrap();

        let worktrees = create_test_worktrees();
        cache.set(worktrees).unwrap();

        // Wait for cache to expire
        std::thread::sleep(Duration::from_millis(10));

        let result = cache.get().unwrap();
        assert!(result.is_none(), "Cache should miss when expired");
    }

    #[test]
    fn test_cache_invalidation() {
        let temp_dir = TempDir::new().unwrap();
        let cache = RegistryCache::new(temp_dir.path(), Some(Duration::from_secs(60))).unwrap();

        let worktrees = create_test_worktrees();
        cache.set(worktrees).unwrap();

        // Verify cache exists
        assert!(cache.get().unwrap().is_some());

        // Invalidate cache
        cache.invalidate().unwrap();

        // Cache should now miss
        assert!(cache.get().unwrap().is_none());
    }

    #[test]
    fn test_cache_stats() {
        let temp_dir = TempDir::new().unwrap();
        let cache = RegistryCache::new(temp_dir.path(), Some(Duration::from_secs(60))).unwrap();

        // No stats when cache doesn't exist
        assert!(cache.stats().unwrap().is_none());

        // Create cache
        let worktrees = create_test_worktrees();
        cache.set(worktrees).unwrap();

        // Get stats
        let stats = cache.stats().unwrap().unwrap();
        assert!(stats.valid, "Cache should be valid");
        assert_eq!(stats.worktree_count, 2);
        assert!(stats.size_bytes > 0);
        assert!(stats.age.as_secs() < 1);
    }

    #[test]
    fn test_cache_update() {
        let temp_dir = TempDir::new().unwrap();
        let cache = RegistryCache::new(temp_dir.path(), Some(Duration::from_secs(60))).unwrap();

        // Initial cache
        let worktrees1 = create_test_worktrees();
        cache.set(worktrees1).unwrap();

        // Update cache
        let mut worktrees2 = create_test_worktrees();
        worktrees2.push(Worktree {
            name: "feature-y".to_string(),
            path: PathBuf::from("/tmp/feature-y"),
            branch: "feature/y".to_string(),
            commit: "ghi789".to_string(),
            parent: Some("main".to_string()),
        });
        cache.set(worktrees2).unwrap();

        // Verify updated cache
        let result = cache.get().unwrap().unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[2].name, "feature-y");
    }
}
