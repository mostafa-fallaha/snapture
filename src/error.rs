use thiserror::Error;

#[derive(Debug, Error)]
pub enum SnaptureError {
    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Image(#[from] image::ImageError),
    #[error(transparent)]
    Portal(#[from] ashpd::Error),
    #[error("clipboard operation failed: {0}")]
    Clipboard(String),
    #[error("invalid file URI: {0}")]
    InvalidUri(String),
    #[error("no suitable system font found for text rendering")]
    MissingFont,
    #[error("{0}")]
    Message(String),
}

pub type AppResult<T> = Result<T, SnaptureError>;

impl From<arboard::Error> for SnaptureError {
    fn from(error: arboard::Error) -> Self {
        Self::Clipboard(error.to_string())
    }
}

impl From<url::ParseError> for SnaptureError {
    fn from(error: url::ParseError) -> Self {
        Self::Message(error.to_string())
    }
}
