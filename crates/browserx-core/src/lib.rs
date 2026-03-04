//! # browserx-core
//!
//! Core library for extracting browser cookies across platforms.
//!
//! Supports Chrome, Edge, Firefox, Safari, Brave, Arc, Vivaldi, Opera,
//! and Chromium on macOS, Linux, and Windows.
//!
//! ## Design Principles
//!
//! - **Inline-first**: Inline cookie payloads (JSON, base64, file) are checked
//!   before local browser databases, short-circuiting if they yield cookies.
//! - **Never throws**: The public API returns `GetCookiesResult` with cookies
//!   and warnings. Errors are caught internally and converted to warnings.
//! - **Secure by default**: Cookie values are wrapped in `SecretValue` which
//!   masks them in `Debug`/`Display` and zeroes memory on drop.
//! - **Zero shell-outs** (where possible): Uses native OS APIs via Rust crates
//!   instead of spawning subprocesses.
//!
//! ## Usage
//!
//! ```no_run
//! use browserx_core::{get_cookies, GetCookiesOptions, to_cookie_header};
//!
//! let options = GetCookiesOptions {
//!     url: "https://github.com".into(),
//!     ..Default::default()
//! };
//!
//! let result = get_cookies(options);
//! for w in &result.warnings {
//!     eprintln!("warning: {w}");
//! }
//!
//! let header = to_cookie_header(&result.cookies);
//! println!("Cookie: {header}");
//! ```

pub mod error;
pub mod platform;
pub mod providers;
pub mod types;
pub mod util;

// Re-export public API at crate root
pub use types::{
    BrowserInfo, BrowserName, Cookie, CookieHealth, CookieSource, CookieStatus,
    GetCookiesOptions, GetCookiesResult, HealthStatus, MergeMode, SameSite,
    SecretValue, SessionHealth,
};

use std::collections::HashSet;
use tracing::{debug, info};

/// Extract cookies from browsers based on the given options.
///
/// This is the main entry point. It:
/// 1. Checks inline sources first (short-circuits if found)
/// 2. Tries each specified browser (or auto-detects all)
/// 3. Merges or returns first result based on mode
/// 4. Deduplicates by name+domain+path
///
/// **Never panics or throws** -- all errors are captured as warnings.
pub fn get_cookies(options: GetCookiesOptions) -> GetCookiesResult {
    let mut result = GetCookiesResult::empty();

    // Resolve hosts from URL + extra origins
    let mut all_urls = vec![options.url.clone()];
    all_urls.extend(options.origins.clone());

    let hosts = match util::origin::normalize_origins(&all_urls) {
        Ok(h) => h,
        Err(e) => {
            return result.with_warning(format!("failed to parse URLs: {e}"));
        }
    };

    debug!("resolved hosts: {:?}", hosts);

    // 1. Try inline sources first (inline-first design)
    match providers::inline::try_inline(
        options.inline_json.as_deref(),
        options.inline_base64.as_deref(),
        options.inline_file.as_deref(),
        &hosts,
        &options.names,
    ) {
        Ok(Some(inline_result)) => {
            info!(
                "inline source provided {} cookies",
                inline_result.cookies.len()
            );
            return inline_result;
        }
        Ok(None) => {
            debug!("no inline sources provided");
        }
        Err(e) => {
            result.warnings.push(format!("inline source error: {e}"));
        }
    }

    // 2. Determine browsers to try
    let browsers = if options.browsers.is_empty() {
        providers::detect_browsers()
    } else {
        options.browsers.clone()
    };

    if browsers.is_empty() {
        return result.with_warning("no browsers detected on this system");
    }

    debug!("browsers to try: {:?}", browsers);

    // 3. Extract from each browser
    for browser in &browsers {
        let provider = providers::provider_for(*browser);

        if !provider.is_available() {
            debug!("{}: not available, skipping", browser.display_name());
            continue;
        }

        let profile = match browser {
            BrowserName::Chrome | BrowserName::Chromium => options.chrome_profile.as_deref(),
            BrowserName::Edge => options.edge_profile.as_deref(),
            BrowserName::Firefox => options.firefox_profile.as_deref(),
            BrowserName::Brave => options.brave_profile.as_deref(),
            _ => None,
        };

        match provider.extract(
            &hosts,
            &options.names,
            profile,
            options.include_expired,
            options.timeout_ms,
        ) {
            Ok(browser_result) => {
                info!(
                    "{}: extracted {} cookies",
                    browser.display_name(),
                    browser_result.cookies.len()
                );

                if options.mode == MergeMode::First && !browser_result.cookies.is_empty() {
                    // First mode: return immediately on first success
                    return browser_result;
                }

                result.merge(browser_result);
            }
            Err(e) => {
                result
                    .warnings
                    .push(format!("{}: {e}", browser.display_name()));
            }
        }
    }

    // 4. Deduplicate by name+domain+path
    deduplicate(&mut result.cookies);

    result
}

/// Build a `Cookie` HTTP header value from a list of cookies.
///
/// Format: `name1=value1; name2=value2`
pub fn to_cookie_header(cookies: &[Cookie]) -> String {
    cookies
        .iter()
        .map(|c| format!("{}={}", c.name, c.value.expose()))
        .collect::<Vec<_>>()
        .join("; ")
}

/// Check session health for a URL.
pub fn check_health(cookies: &[Cookie], url: &str) -> SessionHealth {
    let now = chrono::Utc::now().timestamp();
    let one_hour = 3600;

    let mut active = 0;
    let mut expiring_soon = 0;
    let mut expired = 0;
    let mut details = Vec::new();

    for cookie in cookies {
        let (status, expires_in) = match cookie.expires {
            None => (CookieStatus::Session, None),
            Some(exp) if exp < now => {
                expired += 1;
                (CookieStatus::Expired, Some(util::epoch::expires_in_human(exp)))
            }
            Some(exp) if exp < now + one_hour => {
                expiring_soon += 1;
                active += 1;
                (CookieStatus::ExpiringSoon, Some(util::epoch::expires_in_human(exp)))
            }
            Some(exp) => {
                active += 1;
                (CookieStatus::Active, Some(util::epoch::expires_in_human(exp)))
            }
        };

        details.push(CookieHealth {
            name: cookie.name.clone(),
            domain: cookie.domain.clone(),
            status,
            expires_in,
        });
    }

    let health_status = if cookies.is_empty() {
        HealthStatus::Empty
    } else if expired > 0 && active == 0 {
        HealthStatus::Expired
    } else if expiring_soon > 0 {
        HealthStatus::Warning
    } else {
        HealthStatus::Healthy
    };

    SessionHealth {
        url: url.to_string(),
        status: health_status,
        total_cookies: cookies.len(),
        active_cookies: active,
        expiring_soon,
        expired,
        details,
    }
}

/// Deduplicate cookies by name+domain+path. Last occurrence wins.
fn deduplicate(cookies: &mut Vec<Cookie>) {
    let mut seen = HashSet::new();
    let mut unique = Vec::with_capacity(cookies.len());

    // Iterate in reverse so last occurrence wins
    for cookie in cookies.drain(..).rev() {
        let key = format!(
            "{}|{}|{}",
            cookie.name,
            cookie.domain.to_lowercase(),
            cookie.path
        );
        if seen.insert(key) {
            unique.push(cookie);
        }
    }

    unique.reverse();
    *cookies = unique;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_cookie_header_format() {
        let cookies = vec![
            Cookie {
                name: "a".into(),
                value: SecretValue::new("1".into()),
                domain: "example.com".into(),
                path: "/".into(),
                expires: None,
                secure: false,
                http_only: false,
                same_site: None,
                source: None,
            },
            Cookie {
                name: "b".into(),
                value: SecretValue::new("2".into()),
                domain: "example.com".into(),
                path: "/".into(),
                expires: None,
                secure: false,
                http_only: false,
                same_site: None,
                source: None,
            },
        ];

        let header = to_cookie_header(&cookies);
        assert_eq!(header, "a=1; b=2");
    }

    #[test]
    fn deduplicate_by_name_domain_path() {
        let mut cookies = vec![
            Cookie {
                name: "session".into(),
                value: SecretValue::new("old".into()),
                domain: "example.com".into(),
                path: "/".into(),
                expires: None,
                secure: false,
                http_only: false,
                same_site: None,
                source: None,
            },
            Cookie {
                name: "session".into(),
                value: SecretValue::new("new".into()),
                domain: "example.com".into(),
                path: "/".into(),
                expires: None,
                secure: false,
                http_only: false,
                same_site: None,
                source: None,
            },
        ];

        deduplicate(&mut cookies);
        assert_eq!(cookies.len(), 1);
        assert_eq!(cookies[0].value.expose(), "new");
    }

    #[test]
    fn health_check_empty() {
        let health = check_health(&[], "https://example.com");
        assert_eq!(health.status, HealthStatus::Empty);
    }

    #[test]
    fn health_check_session_cookies_are_healthy() {
        let cookies = vec![Cookie {
            name: "session".into(),
            value: SecretValue::new("abc".into()),
            domain: "example.com".into(),
            path: "/".into(),
            expires: None,
            secure: true,
            http_only: true,
            same_site: None,
            source: None,
        }];

        let health = check_health(&cookies, "https://example.com");
        assert_eq!(health.status, HealthStatus::Healthy);
    }
}
