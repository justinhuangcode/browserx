// Platform-specific modules are conditionally compiled in their respective
// provider implementations (chromium/crypto.rs, etc.).
//
// This module provides shared platform detection utilities.

/// Get a human-readable platform name.
pub fn platform_name() -> &'static str {
    #[cfg(target_os = "macos")]
    { "macOS" }
    #[cfg(target_os = "linux")]
    { "Linux" }
    #[cfg(target_os = "windows")]
    { "Windows" }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    { "Unknown" }
}
