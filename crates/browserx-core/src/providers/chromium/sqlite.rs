use std::path::Path;
use tracing::debug;

use crate::error::Result;
use crate::providers::chromium::crypto;
use crate::types::{
    BrowserName, Cookie, CookieSource, GetCookiesResult, SameSite, SecretValue,
};
use crate::util::{epoch, host_match};

/// Query cookies from a Chromium SQLite database.
///
/// Creates a temporary copy of the DB (+ WAL/SHM sidecars) to avoid
/// locking the browser's live database.
pub fn query_cookies(
    db_path: &Path,
    hosts: &[String],
    names: &[String],
    decrypt_key: Option<&[u8]>,
    include_expired: bool,
    browser: BrowserName,
    profile: &str,
) -> Result<GetCookiesResult> {
    let mut warnings = Vec::new();

    // Copy DB to temp directory (avoid locking browser's live DB)
    let tmp_dir = tempfile::tempdir()?;
    let tmp_db = tmp_dir.path().join("Cookies");

    std::fs::copy(db_path, &tmp_db)?;

    // Also copy WAL and SHM sidecars if they exist
    let db_dir = db_path.parent().unwrap_or(Path::new("."));
    for ext in &["-wal", "-shm"] {
        let sidecar = db_dir.join(format!(
            "{}{}",
            db_path.file_name().unwrap().to_string_lossy(),
            ext
        ));
        if sidecar.exists() {
            let tmp_sidecar = tmp_dir.path().join(format!("Cookies{ext}"));
            if let Err(e) = std::fs::copy(&sidecar, &tmp_sidecar) {
                debug!("failed to copy sidecar {}: {}", sidecar.display(), e);
            }
        }
    }

    // Open in read-only mode
    let conn = rusqlite::Connection::open_with_flags(
        &tmp_db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;

    // Build query
    let host_where = host_match::build_chromium_host_where(hosts);
    let expiry_clause = if include_expired {
        String::new()
    } else {
        let now = chrono::Utc::now().timestamp();
        // expires_utc = 0 means session cookie (always include)
        format!(" AND (expires_utc = 0 OR expires_utc > {})", now * 1_000_000 + WINDOWS_EPOCH_DELTA_US)
    };

    let sql = format!(
        "SELECT name, value, host_key, path, expires_utc, is_secure, \
         is_httponly, samesite, encrypted_value \
         FROM cookies \
         WHERE {host_where}{expiry_clause} \
         ORDER BY expires_utc DESC"
    );

    debug!("executing SQL: {}", sql);

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], |row| {
        Ok(RawCookieRow {
            name: row.get(0)?,
            value: row.get(1)?,
            host_key: row.get(2)?,
            path: row.get(3)?,
            expires_utc: row.get(4)?,
            is_secure: row.get(5)?,
            is_httponly: row.get(6)?,
            samesite: row.get(7)?,
            encrypted_value: row.get(8)?,
        })
    })?;

    let mut cookies = Vec::new();

    for row_result in rows {
        let row = match row_result {
            Ok(r) => r,
            Err(e) => {
                warnings.push(format!("failed to read cookie row: {e}"));
                continue;
            }
        };

        // Filter by name allowlist
        if !names.is_empty() && !names.iter().any(|n| n == &row.name) {
            continue;
        }

        // Decrypt value
        let value = if !row.value.is_empty() {
            // Plain text value available
            row.value
        } else if !row.encrypted_value.is_empty() {
            // Need to decrypt
            match decrypt_key {
                Some(key) => match crypto::decrypt_cookie_value(&row.encrypted_value, key) {
                    Ok(Some(v)) => v,
                    Ok(None) => {
                        warnings.push(format!("cookie '{}' decrypted to empty", row.name));
                        continue;
                    }
                    Err(e) => {
                        warnings.push(format!("failed to decrypt cookie '{}': {e}", row.name));
                        continue;
                    }
                },
                None => {
                    warnings.push(format!(
                        "cookie '{}' is encrypted but no decryption key available",
                        row.name
                    ));
                    continue;
                }
            }
        } else {
            String::new()
        };

        let same_site = match row.samesite {
            0 => Some(SameSite::None),
            1 => Some(SameSite::Lax),
            2 => Some(SameSite::Strict),
            _ => None,
        };

        cookies.push(Cookie {
            name: row.name,
            value: SecretValue::new(value),
            domain: row.host_key,
            path: row.path,
            expires: epoch::chromium_to_unix(row.expires_utc),
            secure: row.is_secure != 0,
            http_only: row.is_httponly != 0,
            same_site,
            source: Some(CookieSource {
                browser,
                profile: profile.to_string(),
                method: Some("sqlite".to_string()),
            }),
        });
    }

    debug!(
        "{}: extracted {} cookies ({} warnings)",
        browser.display_name(),
        cookies.len(),
        warnings.len()
    );

    Ok(GetCookiesResult { cookies, warnings })
}

/// Windows FILETIME epoch delta in microseconds.
const WINDOWS_EPOCH_DELTA_US: i64 = 11_644_473_600 * 1_000_000;

struct RawCookieRow {
    name: String,
    value: String,
    host_key: String,
    path: String,
    expires_utc: i64,
    is_secure: i32,
    is_httponly: i32,
    samesite: i32,
    encrypted_value: Vec<u8>,
}
