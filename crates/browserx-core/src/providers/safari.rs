use std::path::PathBuf;

use crate::error::{BrowserExError, Result};
use crate::providers::CookieProvider;
use crate::types::{BrowserName, GetCookiesResult};

#[derive(Default)]
pub struct SafariProvider;

impl SafariProvider {
    pub fn new() -> Self {
        Self
    }

    fn cookies_path() -> PathBuf {
        let home = dirs::home_dir().unwrap_or_default();
        home.join("Library/Cookies/Cookies.binarycookies")
    }
}

impl CookieProvider for SafariProvider {
    fn browser(&self) -> BrowserName {
        BrowserName::Safari
    }

    fn is_available(&self) -> bool {
        cfg!(target_os = "macos") && Self::cookies_path().exists()
    }

    fn extract(
        &self,
        hosts: &[String],
        names: &[String],
        _profile: Option<&str>,
        include_expired: bool,
        _timeout_ms: u64,
    ) -> Result<GetCookiesResult> {
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (hosts, names, include_expired);
            Err(BrowserExError::PlatformNotSupported {
                operation: "Safari cookie extraction".into(),
                platform: std::env::consts::OS.into(),
            })
        }

        #[cfg(target_os = "macos")]
        {
            let cookie_file = Self::cookies_path();
            if !cookie_file.exists() {
                return Err(BrowserExError::CookieDbNotFound {
                    path: cookie_file.display().to_string(),
                });
            }

            debug!("Safari: reading {}", cookie_file.display());
            let data = std::fs::read(&cookie_file)?;
            parse_binary_cookies(&data, hosts, names, include_expired)
        }
    }
}

/// Parse Safari's Cookies.binarycookies file format.
///
/// File structure:
/// - 4 bytes: magic "cook"
/// - 4 bytes: page count (big-endian)
/// - [page_count * 4 bytes]: page sizes (big-endian)
/// - [pages]: cookie data pages
#[cfg(target_os = "macos")]
fn parse_binary_cookies(
    data: &[u8],
    hosts: &[String],
    names: &[String],
    include_expired: bool,
) -> Result<GetCookiesResult> {
    let mut warnings = Vec::new();

    if data.len() < 8 || &data[..4] != b"cook" {
        return Err(BrowserExError::Other(
            "invalid Safari binary cookies file (bad magic)".into(),
        ));
    }

    let page_count = u32::from_be_bytes([data[4], data[5], data[6], data[7]]) as usize;
    let mut cursor = 8;

    // Read page sizes
    let mut page_sizes = Vec::with_capacity(page_count);
    for _ in 0..page_count {
        if cursor + 4 > data.len() {
            warnings.push("truncated page size table".into());
            break;
        }
        let size = u32::from_be_bytes([
            data[cursor],
            data[cursor + 1],
            data[cursor + 2],
            data[cursor + 3],
        ]) as usize;
        page_sizes.push(size);
        cursor += 4;
    }

    // Parse pages
    let mut cookies = Vec::new();
    let now = chrono::Utc::now().timestamp();

    for page_size in page_sizes {
        if cursor + page_size > data.len() {
            warnings.push("truncated page data".into());
            break;
        }

        let page = &data[cursor..cursor + page_size];
        match parse_safari_page(page) {
            Ok(page_cookies) => {
                for c in page_cookies {
                    // Host filter
                    if !hosts.is_empty() {
                        let matches = hosts
                            .iter()
                            .any(|h| crate::util::host_match::domain_matches(&c.domain, h));
                        if !matches {
                            continue;
                        }
                    }

                    // Name filter
                    if !names.is_empty() && !names.iter().any(|n| n == &c.name) {
                        continue;
                    }

                    // Expiry filter
                    if !include_expired {
                        if let Some(exp) = c.expires {
                            if exp < now {
                                continue;
                            }
                        }
                    }

                    cookies.push(c);
                }
            }
            Err(e) => {
                warnings.push(format!("failed to parse Safari page: {e}"));
            }
        }

        cursor += page_size;
    }

    debug!("Safari: extracted {} cookies", cookies.len());
    Ok(GetCookiesResult { cookies, warnings })
}

/// Parse a single page of Safari binary cookies.
#[cfg(target_os = "macos")]
fn parse_safari_page(page: &[u8]) -> Result<Vec<Cookie>> {
    if page.len() < 8 {
        return Err(BrowserExError::Other("page too small".into()));
    }

    // Page header: first 4 bytes should be 0x00000100
    let cookie_count = u32::from_le_bytes([page[4], page[5], page[6], page[7]]) as usize;

    // Read cookie offsets
    let mut offsets = Vec::with_capacity(cookie_count);
    for i in 0..cookie_count {
        let base = 8 + i * 4;
        if base + 4 > page.len() {
            break;
        }
        let offset =
            u32::from_le_bytes([page[base], page[base + 1], page[base + 2], page[base + 3]])
                as usize;
        offsets.push(offset);
    }

    let mut cookies = Vec::new();

    for offset in offsets {
        if offset + 48 > page.len() {
            continue;
        }

        let record = &page[offset..];
        if let Some(cookie) = parse_safari_cookie_record(record) {
            cookies.push(cookie);
        }
    }

    Ok(cookies)
}

/// Parse a single Safari cookie record.
///
/// Record layout (little-endian):
/// - [0..4]: record size
/// - [4..8]: unknown flags
/// - [8..12]: flags (bit 0=secure, bit 2=httpOnly)
/// - [12..16]: unknown
/// - [16..20]: URL offset
/// - [20..24]: name offset
/// - [24..28]: path offset
/// - [28..32]: value offset
/// - [32..40]: comment (unused)
/// - [40..48]: expiry (f64 little-endian, Mac absolute time)
/// - [48..56]: creation (f64 little-endian)
#[cfg(target_os = "macos")]
fn parse_safari_cookie_record(record: &[u8]) -> Option<Cookie> {
    if record.len() < 48 {
        return None;
    }

    let _size = u32::from_le_bytes([record[0], record[1], record[2], record[3]]);
    let flags = u32::from_le_bytes([record[8], record[9], record[10], record[11]]);

    let url_offset = u32::from_le_bytes([record[16], record[17], record[18], record[19]]) as usize;
    let name_offset = u32::from_le_bytes([record[20], record[21], record[22], record[23]]) as usize;
    let path_offset = u32::from_le_bytes([record[24], record[25], record[26], record[27]]) as usize;
    let value_offset =
        u32::from_le_bytes([record[28], record[29], record[30], record[31]]) as usize;

    let expiry_bytes: [u8; 8] = record[40..48].try_into().ok()?;
    let expiry_mac = f64::from_le_bytes(expiry_bytes);

    let read_cstring = |offset: usize| -> String {
        if offset >= record.len() {
            return String::new();
        }
        let bytes = &record[offset..];
        let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
        String::from_utf8_lossy(&bytes[..end]).to_string()
    };

    let domain = read_cstring(url_offset);
    let name = read_cstring(name_offset);
    let path = read_cstring(path_offset);
    let value = read_cstring(value_offset);

    let secure = flags & 0x01 != 0;
    let http_only = flags & 0x04 != 0;

    Some(Cookie {
        name,
        value: SecretValue::new(value),
        domain,
        path,
        expires: epoch::safari_to_unix(expiry_mac),
        secure,
        http_only,
        same_site: None, // Safari binary format doesn't include SameSite
        source: Some(CookieSource {
            browser: BrowserName::Safari,
            profile: "default".to_string(),
            method: Some("binarycookies".to_string()),
        }),
    })
}
