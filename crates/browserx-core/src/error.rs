use thiserror::Error;

/// Errors that can occur during cookie extraction.
///
/// Note: The public API ([`crate::get_cookies`]) never returns these directly.
/// They are caught internally and converted to warnings in [`crate::GetCookiesResult`].
/// These error types exist for internal control flow and debugging.
#[derive(Debug, Error)]
pub enum BrowserExError {
    #[error("browser not found: {browser} (searched: {searched_paths})")]
    BrowserNotFound {
        browser: String,
        searched_paths: String,
    },

    #[error("profile not found: {profile} in {browser}")]
    ProfileNotFound { browser: String, profile: String },

    #[error("cookie database not found: {path}")]
    CookieDbNotFound { path: String },

    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("decryption failed for {browser} on {platform}: {reason}")]
    Decryption {
        browser: String,
        platform: String,
        reason: String,
    },

    #[error("keychain access failed: {reason}")]
    KeychainAccess { reason: String },

    #[error("invalid inline payload: {reason}")]
    InvalidInlinePayload { reason: String },

    #[error("invalid URL: {url}")]
    InvalidUrl { url: String },

    #[error("operation timed out after {timeout_ms}ms: {operation}")]
    Timeout { operation: String, timeout_ms: u64 },

    #[error("platform not supported: {operation} on {platform}")]
    PlatformNotSupported { operation: String, platform: String },

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}

/// Internal result type for browserx-core.
pub type Result<T> = std::result::Result<T, BrowserExError>;
