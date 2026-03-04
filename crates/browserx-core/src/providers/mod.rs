pub mod chromium;
pub mod firefox;
pub mod inline;
pub mod safari;

use crate::error::Result;
use crate::types::{BrowserName, GetCookiesResult};

/// Trait that all browser cookie providers must implement.
///
/// Each provider knows how to extract cookies from a specific browser
/// on the current platform. Providers are responsible for:
/// 1. Locating the cookie database
/// 2. Querying relevant cookies
/// 3. Decrypting encrypted values
/// 4. Returning a never-throwing result with warnings
pub trait CookieProvider: Send + Sync {
    /// The browser this provider handles.
    fn browser(&self) -> BrowserName;

    /// Check if this browser is installed on the current system.
    fn is_available(&self) -> bool;

    /// Extract cookies matching the given hosts and optional name filter.
    ///
    /// # Arguments
    /// * `hosts` - List of hosts to match cookies against
    /// * `names` - Optional cookie name filter (empty = all cookies)
    /// * `profile` - Optional profile name override
    /// * `include_expired` - Whether to include expired cookies
    /// * `timeout_ms` - Timeout for OS keychain operations
    fn extract(
        &self,
        hosts: &[String],
        names: &[String],
        profile: Option<&str>,
        include_expired: bool,
        timeout_ms: u64,
    ) -> Result<GetCookiesResult>;
}

/// Resolve the correct provider for a browser name.
pub fn provider_for(browser: BrowserName) -> Box<dyn CookieProvider> {
    match browser {
        // All Chromium-based browsers share the same extraction logic,
        // only differing in profile paths and keychain service names.
        BrowserName::Chrome => Box::new(chromium::ChromiumProvider::chrome()),
        BrowserName::Edge => Box::new(chromium::ChromiumProvider::edge()),
        BrowserName::Brave => Box::new(chromium::ChromiumProvider::brave()),
        BrowserName::Arc => Box::new(chromium::ChromiumProvider::arc()),
        BrowserName::Vivaldi => Box::new(chromium::ChromiumProvider::vivaldi()),
        BrowserName::Opera => Box::new(chromium::ChromiumProvider::opera()),
        BrowserName::Chromium => Box::new(chromium::ChromiumProvider::chromium()),
        BrowserName::Firefox => Box::new(firefox::FirefoxProvider::new()),
        BrowserName::Safari => Box::new(safari::SafariProvider::new()),
    }
}

/// Detect all browsers installed on the current system.
pub fn detect_browsers() -> Vec<BrowserName> {
    BrowserName::ALL
        .iter()
        .filter(|b| provider_for(**b).is_available())
        .copied()
        .collect()
}
