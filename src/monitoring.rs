// Monitoring and observability for worktrees
//
// Provides:
// - Activity logging for worktree operations
// - Historical metrics tracking
// - Resource usage monitoring

use crate::errors::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Activity event types
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type")]
pub enum ActivityEvent {
    WorktreeCreated {
        timestamp: u64,
        branch: String,
        template: Option<String>,
    },
    WorktreeRemoved {
        timestamp: u64,
    },
    WorktreeSwitched {
        timestamp: u64,
        from: Option<String>,
    },
    DockerStarted {
        timestamp: u64,
        services: Vec<String>,
    },
    DockerStopped {
        timestamp: u64,
    },
    HookExecuted {
        timestamp: u64,
        hook: String,
        duration_ms: u64,
        success: bool,
    },
    IntegrationPerformed {
        timestamp: u64,
        source: String,
        target: String,
    },
    SnapshotCreated {
        timestamp: u64,
        snapshot_name: String,
    },
    SnapshotRestored {
        timestamp: u64,
        snapshot_name: String,
    },
}

/// Activity log for a worktree
#[derive(Serialize, Deserialize)]
pub struct ActivityLog {
    pub worktree: String,
    pub events: Vec<ActivityEvent>,
}

impl ActivityLog {
    pub fn new(worktree: String) -> Self {
        Self {
            worktree,
            events: Vec::new(),
        }
    }

    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            let worktree = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();
            return Ok(Self::new(worktree));
        }

        let content = fs::read_to_string(path)?;
        let log: ActivityLog = serde_json::from_str(&content)?;
        Ok(log)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(&self)?;
        fs::write(path, json)?;
        Ok(())
    }

    pub fn add_event(&mut self, event: ActivityEvent) {
        self.events.push(event);
    }

    /// Get events since a timestamp
    #[allow(dead_code)] // Reserved for future `hn activity` command
    pub fn events_since(&self, timestamp: u64) -> Vec<&ActivityEvent> {
        self.events
            .iter()
            .filter(|e| self.event_timestamp(e) >= timestamp)
            .collect()
    }

    /// Get last N events
    #[allow(dead_code)] // Reserved for future `hn activity` command
    pub fn last_events(&self, count: usize) -> Vec<&ActivityEvent> {
        let start = if self.events.len() > count {
            self.events.len() - count
        } else {
            0
        };
        self.events[start..].iter().collect()
    }

    #[allow(dead_code)] // Helper for public methods
    fn event_timestamp(&self, event: &ActivityEvent) -> u64 {
        match event {
            ActivityEvent::WorktreeCreated { timestamp, .. } => *timestamp,
            ActivityEvent::WorktreeRemoved { timestamp } => *timestamp,
            ActivityEvent::WorktreeSwitched { timestamp, .. } => *timestamp,
            ActivityEvent::DockerStarted { timestamp, .. } => *timestamp,
            ActivityEvent::DockerStopped { timestamp } => *timestamp,
            ActivityEvent::HookExecuted { timestamp, .. } => *timestamp,
            ActivityEvent::IntegrationPerformed { timestamp, .. } => *timestamp,
            ActivityEvent::SnapshotCreated { timestamp, .. } => *timestamp,
            ActivityEvent::SnapshotRestored { timestamp, .. } => *timestamp,
        }
    }
}

/// Metrics snapshot for a worktree
#[derive(Serialize, Deserialize, Clone)]
pub struct MetricsSnapshot {
    pub timestamp: u64,
    pub disk_usage: u64,
    pub state_dir_size: u64,
    pub docker_running: bool,
    pub docker_memory_mb: Option<u64>,
    pub docker_cpu_percent: Option<f64>,
}

/// Historical metrics for a worktree
#[derive(Serialize, Deserialize)]
pub struct MetricsHistory {
    pub worktree: String,
    pub snapshots: Vec<MetricsSnapshot>,
    pub max_snapshots: usize,
}

impl MetricsHistory {
    pub fn new(worktree: String) -> Self {
        Self {
            worktree,
            snapshots: Vec::new(),
            max_snapshots: 168, // 7 days at 1 hour intervals
        }
    }

    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            let worktree = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();
            return Ok(Self::new(worktree));
        }

        let content = fs::read_to_string(path)?;
        let history: MetricsHistory = serde_json::from_str(&content)?;
        Ok(history)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(&self)?;
        fs::write(path, json)?;
        Ok(())
    }

    pub fn add_snapshot(&mut self, snapshot: MetricsSnapshot) {
        self.snapshots.push(snapshot);

        // Keep only max_snapshots
        if self.snapshots.len() > self.max_snapshots {
            self.snapshots.drain(0..self.snapshots.len() - self.max_snapshots);
        }
    }

    /// Get snapshots in a time range
    pub fn range(&self, start: u64, end: u64) -> Vec<&MetricsSnapshot> {
        self.snapshots
            .iter()
            .filter(|s| s.timestamp >= start && s.timestamp <= end)
            .collect()
    }

    /// Get last N snapshots
    #[allow(dead_code)] // Reserved for future `hn stats --history` enhancements
    pub fn last_snapshots(&self, count: usize) -> Vec<&MetricsSnapshot> {
        let start = if self.snapshots.len() > count {
            self.snapshots.len() - count
        } else {
            0
        };
        self.snapshots[start..].iter().collect()
    }
}

/// Get activity log path for a worktree
pub fn get_activity_log_path(state_dir: &Path, worktree: &str) -> PathBuf {
    state_dir.join(worktree).join("activity.json")
}

/// Get metrics history path for a worktree
pub fn get_metrics_path(state_dir: &Path, worktree: &str) -> PathBuf {
    state_dir.join(worktree).join("metrics.json")
}

/// Log an activity event for a worktree
pub fn log_activity(state_dir: &Path, worktree: &str, event: ActivityEvent) -> Result<()> {
    let log_path = get_activity_log_path(state_dir, worktree);

    // Ensure parent directory exists
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut log = ActivityLog::load(&log_path)?;
    log.add_event(event);
    log.save(&log_path)?;

    Ok(())
}

/// Record a metrics snapshot for a worktree
pub fn record_metrics(state_dir: &Path, worktree: &str, snapshot: MetricsSnapshot) -> Result<()> {
    let metrics_path = get_metrics_path(state_dir, worktree);

    // Ensure parent directory exists
    if let Some(parent) = metrics_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut history = MetricsHistory::load(&metrics_path)?;
    history.add_snapshot(snapshot);
    history.save(&metrics_path)?;

    Ok(())
}

/// Get current timestamp in seconds
pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_activity_log() {
        let temp = TempDir::new().unwrap();
        let log_path = temp.path().join("activity.json");

        let mut log = ActivityLog::new("test".to_string());
        log.add_event(ActivityEvent::WorktreeCreated {
            timestamp: 1000,
            branch: "main".to_string(),
            template: None,
        });

        log.save(&log_path).unwrap();

        let loaded = ActivityLog::load(&log_path).unwrap();
        assert_eq!(loaded.events.len(), 1);
    }

    #[test]
    fn test_metrics_history() {
        let temp = TempDir::new().unwrap();
        let metrics_path = temp.path().join("metrics.json");

        let mut history = MetricsHistory::new("test".to_string());
        history.add_snapshot(MetricsSnapshot {
            timestamp: 1000,
            disk_usage: 1024,
            state_dir_size: 512,
            docker_running: false,
            docker_memory_mb: None,
            docker_cpu_percent: None,
        });

        history.save(&metrics_path).unwrap();

        let loaded = MetricsHistory::load(&metrics_path).unwrap();
        assert_eq!(loaded.snapshots.len(), 1);
    }

    #[test]
    fn test_metrics_max_snapshots() {
        let mut history = MetricsHistory::new("test".to_string());
        history.max_snapshots = 5;

        for i in 0..10 {
            history.add_snapshot(MetricsSnapshot {
                timestamp: i * 1000,
                disk_usage: 1024,
                state_dir_size: 512,
                docker_running: false,
                docker_memory_mb: None,
                docker_cpu_percent: None,
            });
        }

        assert_eq!(history.snapshots.len(), 5);
        assert_eq!(history.snapshots[0].timestamp, 5000);
    }
}
