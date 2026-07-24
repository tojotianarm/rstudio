use thiserror::Error;

#[derive(Debug, Error)]
pub enum RStudioError {
    #[error("Audio error: {0}")]
    Audio(String),

    #[error("Invalid configuration: {0}")]
    Configuration(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, RStudioError>;