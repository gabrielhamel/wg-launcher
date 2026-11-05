use std::{error::Error, fmt};

#[derive(Debug)]
pub(crate) struct ReleaseError(String);

impl fmt::Display for ReleaseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl Error for ReleaseError {}

pub(super) type Result<T> = std::result::Result<T, ReleaseError>;

pub(super) fn invalid_input(message: impl Into<String>) -> ReleaseError {
    ReleaseError(message.into())
}
