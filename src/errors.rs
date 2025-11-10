use thiserror::Error;

#[derive(Error, Debug)]
pub enum HnError {
    #[error("Git error: {0}")]
    Git(#[from] git2::Error),

    #[error("Not in a git repository")]
    NotInRepository,

    #[error("Worktree '{0}' not found")]
    WorktreeNotFound(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, HnError>;
