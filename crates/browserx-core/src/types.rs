use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

/// A browser cookie with all standard attributes.
///
/// The `value` field is wrapped in [`SecretValue`] to prevent
/// accidental logging or serialization of sensitive data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cookie {
    pub name: String,
    pub value: SecretValue,
    pub domain: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires: Option<i64>,
    pub secure: bool,
    pub http_only: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub same_site: Option<SameSite>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<CookieSource>,
}

/// A wrapper around cookie values that prevents accidental logging.
///
/// - [`Debug`] prints `"***"` instead of the actual value.
/// - [`Drop`] zeroes the memory.
/// - Use [`.expose()`] to intentionally access the raw value.
#[derive(Clone, Serialize, Deserialize, Zeroize)]
#[zeroize(drop)]
pub struct SecretValue(String);

impl SecretValue {
    pub fn new(value: String) -> Self {
        Self(value)
    }

    /// Intentionally expose the raw cookie value.
    /// Only call this when you need to output or transmit the value.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for SecretValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("\"***\"")
    }
}

impl std::fmt::Display for SecretValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("***")
    }
}

/// SameSite cookie attribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SameSite {
    Strict,
    Lax,
    None,
}

impl std::fmt::Display for SameSite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Strict => f.write_str("Strict"),
            Self::Lax => f.write_str("Lax"),
            Self::None => f.write_str("None"),
        }
    }
}

/// Metadata about where a cookie was extracted from.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CookieSource {
    pub browser: BrowserName,
    pub profile: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
}

/// Supported browsers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BrowserName {
    Chrome,
    Edge,
    Firefox,
    Safari,
    Brave,
    Arc,
    Vivaldi,
    Opera,
    Chromium,
}

impl BrowserName {
    /// All known browser variants.
    pub const ALL: &'static [BrowserName] = &[
        Self::Chrome,
        Self::Edge,
        Self::Firefox,
        Self::Safari,
        Self::Brave,
        Self::Arc,
        Self::Vivaldi,
        Self::Opera,
        Self::Chromium,
    ];

    /// Browsers based on the Chromium engine (share cookie DB format).
    pub fn is_chromium_based(&self) -> bool {
        matches!(
            self,
            Self::Chrome
                | Self::Edge
                | Self::Brave
                | Self::Arc
                | Self::Vivaldi
                | Self::Opera
                | Self::Chromium
        )
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Chrome => "Google Chrome",
            Self::Edge => "Microsoft Edge",
            Self::Firefox => "Mozilla Firefox",
            Self::Safari => "Apple Safari",
            Self::Brave => "Brave Browser",
            Self::Arc => "Arc Browser",
            Self::Vivaldi => "Vivaldi",
            Self::Opera => "Opera",
            Self::Chromium => "Chromium",
        }
    }
}

impl std::fmt::Display for BrowserName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.display_name())
    }
}

impl std::str::FromStr for BrowserName {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "chrome" | "google-chrome" => Ok(Self::Chrome),
            "edge" | "msedge" | "microsoft-edge" => Ok(Self::Edge),
            "firefox" | "mozilla-firefox" => Ok(Self::Firefox),
            "safari" | "apple-safari" => Ok(Self::Safari),
            "brave" => Ok(Self::Brave),
            "arc" => Ok(Self::Arc),
            "vivaldi" => Ok(Self::Vivaldi),
            "opera" => Ok(Self::Opera),
            "chromium" => Ok(Self::Chromium),
            _ => Err(format!("unknown browser: {s}")),
        }
    }
}

/// How to combine results from multiple browsers.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MergeMode {
    /// Combine cookies from all browsers, deduplicating by name+domain+path.
    #[default]
    Merge,
    /// Return cookies from the first browser that succeeds.
    First,
}

impl std::str::FromStr for MergeMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "merge" => Ok(Self::Merge),
            "first" => Ok(Self::First),
            _ => Err(format!("unknown merge mode: {s} (expected 'merge' or 'first')")),
        }
    }
}

/// Options for cookie extraction.
#[derive(Debug, Clone)]
pub struct GetCookiesOptions {
    /// Target URL to extract cookies for.
    pub url: String,

    /// Additional origins to include (e.g., OAuth/SSO domains).
    pub origins: Vec<String>,

    /// Filter cookies by name (empty = all cookies).
    pub names: Vec<String>,

    /// Browsers to try, in order. Empty = auto-detect all.
    pub browsers: Vec<BrowserName>,

    /// How to combine results from multiple browsers.
    pub mode: MergeMode,

    /// Include expired cookies.
    pub include_expired: bool,

    /// Timeout for OS keychain operations (milliseconds).
    pub timeout_ms: u64,

    // -- Inline sources (checked first, short-circuit if matched) --
    /// Raw JSON string containing exported cookies.
    pub inline_json: Option<String>,

    /// Base64-encoded JSON string containing exported cookies.
    pub inline_base64: Option<String>,

    /// Path to a JSON file containing exported cookies.
    pub inline_file: Option<String>,

    // -- Profile overrides --
    /// Chrome profile name or path (default: "Default").
    pub chrome_profile: Option<String>,

    /// Edge profile name or path.
    pub edge_profile: Option<String>,

    /// Firefox profile name or path.
    pub firefox_profile: Option<String>,

    /// Brave profile name or path.
    pub brave_profile: Option<String>,

    /// Direct path to Safari Cookies.binarycookies file.
    pub safari_cookies_file: Option<String>,
}

impl Default for GetCookiesOptions {
    fn default() -> Self {
        Self {
            url: String::new(),
            origins: Vec::new(),
            names: Vec::new(),
            browsers: Vec::new(),
            mode: MergeMode::default(),
            include_expired: false,
            timeout_ms: 5000,
            inline_json: None,
            inline_base64: None,
            inline_file: None,
            chrome_profile: None,
            edge_profile: None,
            firefox_profile: None,
            brave_profile: None,
            safari_cookies_file: None,
        }
    }
}

/// Result of a cookie extraction operation.
///
/// Never panics or throws -- errors are collected as warnings.
#[derive(Debug, Clone, Serialize)]
pub struct GetCookiesResult {
    pub cookies: Vec<Cookie>,
    pub warnings: Vec<String>,
}

impl GetCookiesResult {
    pub fn empty() -> Self {
        Self {
            cookies: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn with_warning(mut self, msg: impl Into<String>) -> Self {
        self.warnings.push(msg.into());
        self
    }

    pub fn merge(&mut self, other: GetCookiesResult) {
        self.cookies.extend(other.cookies);
        self.warnings.extend(other.warnings);
    }
}

/// Information about a detected browser installation.
#[derive(Debug, Clone, Serialize)]
pub struct BrowserInfo {
    pub name: BrowserName,
    pub version: Option<String>,
    pub profiles: Vec<ProfileInfo>,
    pub executable: Option<String>,
}

/// Information about a browser profile.
#[derive(Debug, Clone, Serialize)]
pub struct ProfileInfo {
    pub name: String,
    pub path: String,
    pub is_default: bool,
}

/// Session health status.
#[derive(Debug, Clone, Serialize)]
pub struct SessionHealth {
    pub url: String,
    pub status: HealthStatus,
    pub total_cookies: usize,
    pub active_cookies: usize,
    pub expiring_soon: usize,
    pub expired: usize,
    pub details: Vec<CookieHealth>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    /// All cookies are valid and not expiring soon.
    Healthy,
    /// Some cookies are expiring within 1 hour.
    Warning,
    /// Session cookies are expired or missing.
    Expired,
    /// No cookies found.
    Empty,
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Healthy => f.write_str("HEALTHY"),
            Self::Warning => f.write_str("WARNING"),
            Self::Expired => f.write_str("EXPIRED"),
            Self::Empty => f.write_str("EMPTY"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CookieHealth {
    pub name: String,
    pub domain: String,
    pub status: CookieStatus,
    pub expires_in: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CookieStatus {
    Active,
    ExpiringSoon,
    Expired,
    Session,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_value_debug_is_masked() {
        let secret = SecretValue::new("my-secret-token".into());
        let debug = format!("{:?}", secret);
        assert_eq!(debug, r#""***""#);
        assert!(!debug.contains("my-secret-token"));
    }

    #[test]
    fn secret_value_display_is_masked() {
        let secret = SecretValue::new("my-secret-token".into());
        let display = format!("{}", secret);
        assert_eq!(display, "***");
    }

    #[test]
    fn secret_value_expose_returns_raw() {
        let secret = SecretValue::new("my-secret-token".into());
        assert_eq!(secret.expose(), "my-secret-token");
    }

    #[test]
    fn browser_name_from_str() {
        assert_eq!("chrome".parse::<BrowserName>().unwrap(), BrowserName::Chrome);
        assert_eq!("edge".parse::<BrowserName>().unwrap(), BrowserName::Edge);
        assert_eq!("firefox".parse::<BrowserName>().unwrap(), BrowserName::Firefox);
        assert_eq!("safari".parse::<BrowserName>().unwrap(), BrowserName::Safari);
        assert_eq!("brave".parse::<BrowserName>().unwrap(), BrowserName::Brave);
        assert_eq!("arc".parse::<BrowserName>().unwrap(), BrowserName::Arc);
        assert!("unknown".parse::<BrowserName>().is_err());
    }

    #[test]
    fn chromium_based_detection() {
        assert!(BrowserName::Chrome.is_chromium_based());
        assert!(BrowserName::Edge.is_chromium_based());
        assert!(BrowserName::Brave.is_chromium_based());
        assert!(!BrowserName::Firefox.is_chromium_based());
        assert!(!BrowserName::Safari.is_chromium_based());
    }

    #[test]
    fn merge_mode_from_str() {
        assert_eq!("merge".parse::<MergeMode>().unwrap(), MergeMode::Merge);
        assert_eq!("first".parse::<MergeMode>().unwrap(), MergeMode::First);
        assert!("invalid".parse::<MergeMode>().is_err());
    }

    #[test]
    fn get_cookies_result_merge() {
        let mut a = GetCookiesResult {
            cookies: vec![Cookie {
                name: "a".into(),
                value: SecretValue::new("1".into()),
                domain: "example.com".into(),
                path: "/".into(),
                expires: None,
                secure: false,
                http_only: false,
                same_site: None,
                source: None,
            }],
            warnings: vec!["warn-a".into()],
        };
        let b = GetCookiesResult {
            cookies: vec![Cookie {
                name: "b".into(),
                value: SecretValue::new("2".into()),
                domain: "example.com".into(),
                path: "/".into(),
                expires: None,
                secure: false,
                http_only: false,
                same_site: None,
                source: None,
            }],
            warnings: vec!["warn-b".into()],
        };
        a.merge(b);
        assert_eq!(a.cookies.len(), 2);
        assert_eq!(a.warnings.len(), 2);
    }
}
