use base64::Engine;
use serde::Deserialize;
use std::path::Path;
use tracing::{debug, warn};

use crate::error::{BrowserExError, Result};
use crate::types::{Cookie, GetCookiesResult, SecretValue};

/// Inline cookie payload format (compatible with sweet-cookie extension export).
#[derive(Debug, Deserialize)]
struct InlinePayload {
    #[allow(dead_code)]
    version: Option<u32>,
    cookies: Vec<InlineCookie>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InlineCookie {
    name: String,
    value: String,
    #[serde(default)]
    domain: String,
    #[serde(default = "default_path")]
    path: String,
    #[serde(default)]
    secure: bool,
    #[serde(default)]
    http_only: bool,
    #[serde(default)]
    expires: Option<f64>,
    #[serde(default)]
    same_site: Option<String>,
}

fn default_path() -> String {
    "/".to_string()
}

/// Try to extract cookies from inline sources in priority order:
/// 1. `inline_json` - raw JSON string
/// 2. `inline_base64` - base64-encoded JSON string
/// 3. `inline_file` - path to a JSON file
///
/// Returns `Ok(None)` if no inline sources are provided.
/// Returns `Ok(Some(result))` if inline cookies were found (short-circuits browser reads).
pub fn try_inline(
    inline_json: Option<&str>,
    inline_base64: Option<&str>,
    inline_file: Option<&str>,
    hosts: &[String],
    names: &[String],
) -> Result<Option<GetCookiesResult>> {
    // Priority 1: Raw JSON
    if let Some(json_str) = inline_json {
        debug!("trying inline JSON source");
        let result = parse_inline_json(json_str, hosts, names)?;
        if !result.cookies.is_empty() {
            return Ok(Some(result));
        }
    }

    // Priority 2: Base64
    if let Some(b64_str) = inline_base64 {
        debug!("trying inline base64 source");
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(b64_str.trim())
            .map_err(|e| BrowserExError::InvalidInlinePayload {
                reason: format!("base64 decode failed: {e}"),
            })?;
        let json_str =
            String::from_utf8(decoded).map_err(|e| BrowserExError::InvalidInlinePayload {
                reason: format!("base64 payload is not valid UTF-8: {e}"),
            })?;
        let result = parse_inline_json(&json_str, hosts, names)?;
        if !result.cookies.is_empty() {
            return Ok(Some(result));
        }
    }

    // Priority 3: File
    if let Some(file_path) = inline_file {
        debug!("trying inline file source: {}", file_path);
        let path = Path::new(file_path);
        if !path.exists() {
            warn!("inline file not found: {}", file_path);
            return Ok(Some(
                GetCookiesResult::empty()
                    .with_warning(format!("inline file not found: {file_path}")),
            ));
        }
        let json_str =
            std::fs::read_to_string(path).map_err(|e| BrowserExError::InvalidInlinePayload {
                reason: format!("failed to read file '{file_path}': {e}"),
            })?;
        let result = parse_inline_json(&json_str, hosts, names)?;
        if !result.cookies.is_empty() {
            return Ok(Some(result));
        }
    }

    Ok(None)
}

fn parse_inline_json(
    json_str: &str,
    hosts: &[String],
    names: &[String],
) -> Result<GetCookiesResult> {
    let payload: InlinePayload =
        serde_json::from_str(json_str).map_err(|e| BrowserExError::InvalidInlinePayload {
            reason: format!("JSON parse failed: {e}"),
        })?;

    let mut warnings = Vec::new();
    let mut cookies = Vec::new();

    for c in payload.cookies {
        // Filter by host if specified
        if !hosts.is_empty() {
            let domain_match = hosts.iter().any(|h| {
                crate::util::host_match::domain_matches(&c.domain, h)
                    || crate::util::host_match::domain_matches(h, &c.domain)
                    || c.domain.trim_start_matches('.') == h.as_str()
            });
            if !domain_match {
                continue;
            }
        }

        // Filter by name allowlist
        if !names.is_empty() && !names.iter().any(|n| n == &c.name) {
            continue;
        }

        let same_site = c
            .same_site
            .as_deref()
            .and_then(|s| match s.to_lowercase().as_str() {
                "strict" => Some(crate::types::SameSite::Strict),
                "lax" => Some(crate::types::SameSite::Lax),
                "none" => Some(crate::types::SameSite::None),
                _ => None,
            });

        let expires = c.expires.and_then(|e| {
            if e <= 0.0 {
                None
            } else {
                Some(crate::util::epoch::normalize_expiration(e as i64).unwrap_or(e as i64))
            }
        });

        cookies.push(Cookie {
            name: c.name,
            value: SecretValue::new(c.value),
            domain: c.domain,
            path: c.path,
            expires,
            secure: c.secure,
            http_only: c.http_only,
            same_site,
            source: Some(crate::types::CookieSource {
                browser: crate::types::BrowserName::Chrome, // inline doesn't specify
                profile: "inline".to_string(),
                method: Some("inline".to_string()),
            }),
        });
    }

    if cookies.is_empty() {
        warnings.push("inline source provided but no matching cookies found".to_string());
    }

    Ok(GetCookiesResult { cookies, warnings })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_inline_json() {
        let json = r#"{
            "version": 1,
            "cookies": [
                {
                    "name": "session",
                    "value": "abc123",
                    "domain": "example.com",
                    "path": "/",
                    "secure": true,
                    "httpOnly": true
                }
            ]
        }"#;

        let result = parse_inline_json(json, &[], &[]).unwrap();
        assert_eq!(result.cookies.len(), 1);
        assert_eq!(result.cookies[0].name, "session");
        assert_eq!(result.cookies[0].value.expose(), "abc123");
    }

    #[test]
    fn parse_with_host_filter() {
        let json = r#"{
            "cookies": [
                { "name": "a", "value": "1", "domain": "example.com" },
                { "name": "b", "value": "2", "domain": "other.com" }
            ]
        }"#;

        let hosts = vec!["example.com".to_string()];
        let result = parse_inline_json(json, &hosts, &[]).unwrap();
        assert_eq!(result.cookies.len(), 1);
        assert_eq!(result.cookies[0].name, "a");
    }

    #[test]
    fn parse_with_name_filter() {
        let json = r#"{
            "cookies": [
                { "name": "session", "value": "1", "domain": "example.com" },
                { "name": "theme", "value": "dark", "domain": "example.com" }
            ]
        }"#;

        let names = vec!["session".to_string()];
        let result = parse_inline_json(json, &[], &names).unwrap();
        assert_eq!(result.cookies.len(), 1);
        assert_eq!(result.cookies[0].name, "session");
    }

    #[test]
    fn invalid_json_returns_error() {
        let result = parse_inline_json("not-json", &[], &[]);
        assert!(result.is_err());
    }

    #[test]
    fn try_inline_none_when_no_sources() {
        let result = try_inline(None, None, None, &[], &[]).unwrap();
        assert!(result.is_none());
    }
}
