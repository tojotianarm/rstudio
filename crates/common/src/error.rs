use thiserror::Error;

#[derive(Debug, Error)]
pub enum RStudioError {
    #[error("invalid configuration for {field}: {reason}")]
    InvalidConfiguration { field: &'static str, reason: &'static str },

    #[error("invalid audio buffer: {0}")]
    InvalidAudioBuffer(&'static str),

    #[error("audio error: {0}")]
    Audio(String),
}

pub type Result<T> = std::result::Result<T, RStudioError>;
