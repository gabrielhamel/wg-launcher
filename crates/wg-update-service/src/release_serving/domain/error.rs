use std::{error::Error, fmt};

#[derive(Debug)]
pub(crate) enum ReleaseError {
    InvalidInput(String),
    InvalidPublicOrigin(&'static str),
}

impl fmt::Display for ReleaseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput(message) => message.fmt(formatter),
            Self::InvalidPublicOrigin(message) => formatter.write_str(message),
        }
    }
}

impl Error for ReleaseError {}

pub(super) type Result<T> = std::result::Result<T, ReleaseError>;

pub(super) fn invalid_input(message: impl Into<String>) -> ReleaseError {
    ReleaseError::InvalidInput(message.into())
}
