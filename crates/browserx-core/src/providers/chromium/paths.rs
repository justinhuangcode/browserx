use std::path::PathBuf;

use crate::error::{BrowserExError, Result};
use crate::types::BrowserName;

/// Get the user data directory for a Chromium-based browser.
pub fn user_data_dir(browser: BrowserName) -> Result<PathBuf> {
    let home = dirs::home_dir()
        .ok_or_else(|| BrowserExError::Other("cannot determine home directory".into()))?;

    #[cfg(target_os = "macos")]
    let base = home.join("Library/Application Support");

    #[cfg(target_os = "linux")]
    let base = home.join(".config");

    #[cfg(target_os = "windows")]
    let base = dirs::data_local_dir()
        .ok_or_else(|| BrowserExError::Other("cannot determine local app data dir".into()))?;

    let subdir = match browser {
        BrowserName::Chrome => {
            #[cfg(target_os = "macos")]
            {
                "Google/Chrome"
            }
            #[cfg(target_os = "linux")]
            {
                "google-chrome"
            }
            #[cfg(target_os = "windows")]
            {
                "Google/Chrome/User Data"
            }
        }
        BrowserName::Edge => {
            #[cfg(target_os = "macos")]
            {
                "Microsoft Edge"
            }
            #[cfg(target_os = "linux")]
            {
                "microsoft-edge"
            }
            #[cfg(target_os = "windows")]
            {
                "Microsoft/Edge/User Data"
            }
        }
        BrowserName::Brave => {
            #[cfg(target_os = "macos")]
            {
                "BraveSoftware/Brave-Browser"
            }
            #[cfg(target_os = "linux")]
            {
                "BraveSoftware/Brave-Browser"
            }
            #[cfg(target_os = "windows")]
            {
                "BraveSoftware/Brave-Browser/User Data"
            }
        }
        BrowserName::Arc => {
            #[cfg(target_os = "macos")]
            {
                "Arc/User Data"
            }
            #[cfg(target_os = "linux")]
            {
                "arc"
            }
            #[cfg(target_os = "windows")]
            {
                "Arc/User Data"
            }
        }
        BrowserName::Vivaldi => {
            #[cfg(target_os = "macos")]
            {
                "Vivaldi"
            }
            #[cfg(target_os = "linux")]
            {
                "vivaldi"
            }
            #[cfg(target_os = "windows")]
            {
                "Vivaldi/User Data"
            }
        }
        BrowserName::Opera => {
            #[cfg(target_os = "macos")]
            {
                "com.operasoftware.Opera"
            }
            #[cfg(target_os = "linux")]
            {
                "opera"
            }
            #[cfg(target_os = "windows")]
            {
                "Opera Software/Opera Stable"
            }
        }
        BrowserName::Chromium => {
            #[cfg(target_os = "macos")]
            {
                "Chromium"
            }
            #[cfg(target_os = "linux")]
            {
                "chromium"
            }
            #[cfg(target_os = "windows")]
            {
                "Chromium/User Data"
            }
        }
        _ => {
            return Err(BrowserExError::Other(format!(
                "{} is not a Chromium-based browser",
                browser.display_name()
            )));
        }
    };

    Ok(base.join(subdir))
}

/// Get the path to the Cookies SQLite database for a specific profile.
pub fn cookie_db_path(browser: BrowserName, profile: &str) -> Result<PathBuf> {
    let user_data = user_data_dir(browser)?;
    let profile_dir = user_data.join(profile);

    // Modern Chromium stores cookies in Network/ subdirectory
    let modern = profile_dir.join("Network/Cookies");
    if modern.exists() {
        return Ok(modern);
    }

    // Legacy path
    let legacy = profile_dir.join("Cookies");
    if legacy.exists() {
        return Ok(legacy);
    }

    // Return the modern path even if it doesn't exist yet
    // (the caller will handle the "not found" case)
    Ok(modern)
}

/// Get the Local State file path (contains encrypted key on Windows).
pub fn local_state_path(browser: BrowserName) -> Result<PathBuf> {
    let user_data = user_data_dir(browser)?;
    Ok(user_data.join("Local State"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_data_dir_returns_path() {
        // Should not error on any platform
        let path = user_data_dir(BrowserName::Chrome).unwrap();
        assert!(path.to_string_lossy().contains("hrome"));
    }

    #[test]
    fn non_chromium_browser_errors() {
        assert!(user_data_dir(BrowserName::Firefox).is_err());
        assert!(user_data_dir(BrowserName::Safari).is_err());
    }

    #[test]
    fn cookie_db_path_includes_profile() {
        let path = cookie_db_path(BrowserName::Chrome, "Profile 1").unwrap();
        assert!(path.to_string_lossy().contains("Profile 1"));
    }
}
