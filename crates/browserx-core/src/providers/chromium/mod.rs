pub mod crypto;
pub mod paths;
pub mod sqlite;

use tracing::{debug, warn};

use crate::error::{BrowserExError, Result};
use crate::providers::CookieProvider;
use crate::types::{BrowserName, GetCookiesResult};

/// Configuration for a specific Chromium-based browser.
///
/// All Chromium browsers share the same cookie DB schema and encryption,
/// only differing in paths and keychain service names.
#[derive(Debug, Clone)]
pub struct ChromiumConfig {
    pub browser: BrowserName,
    /// macOS Keychain service name (e.g., "Chrome Safe Storage")
    pub keychain_service: &'static str,
    /// macOS Keychain account name
    pub keychain_account: &'static str,
    /// Linux keyring application name
    pub linux_keyring_app: &'static str,
}

pub struct ChromiumProvider {
    config: ChromiumConfig,
}

impl ChromiumProvider {
    pub fn chrome() -> Self {
        Self {
            config: ChromiumConfig {
                browser: BrowserName::Chrome,
                keychain_service: "Chrome Safe Storage",
                keychain_account: "Chrome",
                linux_keyring_app: "chrome",
            },
        }
    }

    pub fn edge() -> Self {
        Self {
            config: ChromiumConfig {
                browser: BrowserName::Edge,
                keychain_service: "Microsoft Edge Safe Storage",
                keychain_account: "Microsoft Edge",
                linux_keyring_app: "chromium", // Edge on Linux uses chromium keyring
            },
        }
    }

    pub fn brave() -> Self {
        Self {
            config: ChromiumConfig {
                browser: BrowserName::Brave,
                keychain_service: "Brave Safe Storage",
                keychain_account: "Brave",
                linux_keyring_app: "brave",
            },
        }
    }

    pub fn arc() -> Self {
        Self {
            config: ChromiumConfig {
                browser: BrowserName::Arc,
                keychain_service: "Arc Safe Storage",
                keychain_account: "Arc",
                linux_keyring_app: "arc",
            },
        }
    }

    pub fn vivaldi() -> Self {
        Self {
            config: ChromiumConfig {
                browser: BrowserName::Vivaldi,
                keychain_service: "Vivaldi Safe Storage",
                keychain_account: "Vivaldi",
                linux_keyring_app: "vivaldi",
            },
        }
    }

    pub fn opera() -> Self {
        Self {
            config: ChromiumConfig {
                browser: BrowserName::Opera,
                keychain_service: "Opera Safe Storage",
                keychain_account: "Opera",
                linux_keyring_app: "opera",
            },
        }
    }

    pub fn chromium() -> Self {
        Self {
            config: ChromiumConfig {
                browser: BrowserName::Chromium,
                keychain_service: "Chromium Safe Storage",
                keychain_account: "Chromium",
                linux_keyring_app: "chromium",
            },
        }
    }
}

impl CookieProvider for ChromiumProvider {
    fn browser(&self) -> BrowserName {
        self.config.browser
    }

    fn is_available(&self) -> bool {
        paths::user_data_dir(self.config.browser)
            .map(|p| p.exists())
            .unwrap_or(false)
    }

    fn extract(
        &self,
        hosts: &[String],
        names: &[String],
        profile: Option<&str>,
        include_expired: bool,
        timeout_ms: u64,
    ) -> Result<GetCookiesResult> {
        let browser_name = self.config.browser.display_name();
        debug!("{}: starting cookie extraction", browser_name);

        // 1. Resolve cookie DB path
        let profile_name = profile.unwrap_or("Default");
        let db_path = paths::cookie_db_path(self.config.browser, profile_name)?;

        if !db_path.exists() {
            return Err(BrowserExError::CookieDbNotFound {
                path: db_path.display().to_string(),
            });
        }

        debug!("{}: found cookie DB at {}", browser_name, db_path.display());

        // 2. Obtain decryption key (platform-specific)
        let decrypt_key = match crypto::get_decryption_key(&self.config, timeout_ms) {
            Ok(key) => Some(key),
            Err(e) => {
                warn!("{}: failed to get decryption key: {}", browser_name, e);
                None
            }
        };

        // 3. Query and decrypt cookies from SQLite
        sqlite::query_cookies(
            &db_path,
            hosts,
            names,
            decrypt_key.as_deref(),
            include_expired,
            self.config.browser,
            profile_name,
        )
    }
}
