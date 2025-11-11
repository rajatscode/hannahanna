use thiserror::Error;

#[derive(Error, Debug)]
pub enum HnError {
    #[error("Git error: {0}")]
    Git(#[from] git2::Error),

    #[error("Not in a git repository")]
    NotInRepository,

    #[error("Worktree '{0}' not found")]
    WorktreeNotFound(String),

    #[error("Invalid worktree name: {0}")]
    InvalidWorktreeName(String),

    #[error("Worktree '{0}' already exists")]
    WorktreeAlreadyExists(String),

    #[error("Ambiguous worktree name '{0}'. Did you mean one of: {}", .1.join(", "))]
    AmbiguousWorktreeName(String, Vec<String>),

    #[error("Worktree '{0}' has no parent. It was not created from another worktree.")]
    NoParent(String),

    #[error("Config error: {0}")]
    ConfigError(String),

    #[error("Hook error: {0}")]
    HookError(String),

    #[error("Symlink error: {0}")]
    SymlinkError(String),

    #[error("Copy error: {0}")]
    CopyError(String),

    #[error("Docker error: {0}")]
    DockerError(String),

    #[error("Port allocation error: {0}")]
    PortAllocationError(String),

    #[error("Command failed: {0}")]
    CommandFailed(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, HnError>;
