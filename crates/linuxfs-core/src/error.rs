use std::{error::Error as StdError, fmt};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    StorageAccess,
    PermissionDenied,
    InvalidImage,
    PartitionTable,
    UnsupportedFilesystem,
    UnsupportedFeature,
    FilesystemCorrupt,
    FilesystemNeedsRecovery,
    MountPointUnavailable,
    WinFspUnavailable,
    WinFspFailure,
    Configuration,
    Internal,
}
#[derive(Debug)]
pub struct Error {
    category: ErrorCategory,
    message: String,
    source: Option<Box<dyn StdError + Send + Sync>>,
}
pub type Result<T> = std::result::Result<T, Error>;
impl Error {
    pub fn new(category: ErrorCategory, message: impl Into<String>) -> Self {
        Self {
            category,
            message: message.into(),
            source: None,
        }
    }
    pub fn with_source(
        category: ErrorCategory,
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        Self {
            category,
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }
    pub const fn category(&self) -> ErrorCategory {
        self.category
    }
}
impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}
impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source
            .as_deref()
            .map(|source| source as &(dyn StdError + 'static))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn error_preserves_category_and_message() {
        let error = Error::new(ErrorCategory::InvalidImage, "bad image");
        assert_eq!(error.category(), ErrorCategory::InvalidImage);
        assert_eq!(error.to_string(), "bad image");
    }
}
