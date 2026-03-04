use std::path::{Path, PathBuf};
use tracing::debug;

use crate::error::{BrowserExError, Result};
use crate::providers::CookieProvider;
use crate::types::{BrowserName, Cookie, CookieSource, GetCookiesResult, SameSite, SecretValue};
use crate::util::{epoch, host_match};

pub struct FirefoxProvider;

impl FirefoxProvider {
    pub fn new() -> Self {
        Self
    }

    fn profiles_dir() -> Result<PathBuf> {
        let home = dirs::home_dir()
            .ok_or_else(|| BrowserExError::Other("cannot determine home directory".into()))?;

        #[cfg(target_os = "macos")]
        let path = home.join("Library/Application Support/Firefox/Profiles");

        #[cfg(target_os = "linux")]
        let path = home.join(".mozilla/firefox");

        #[cfg(target_os = "windows")]
        let path = dirs::data_dir()
            .ok_or_else(|| BrowserExError::Other("cannot determine app data dir".into()))?
            .join("Mozilla/Firefox/Profiles");

        Ok(path)
    }

    /// Find the default profile directory.
    fn find_profile(profile: Option<&str>) -> Result<PathBuf> {
        let profiles_dir = Self::profiles_dir()?;

        if let Some(name) = profile {
            // Exact name match or suffix match
            let exact = profiles_dir.join(name);
            if exact.exists() {
                return Ok(exact);
            }

            // Try suffix match: "default-release" -> "*.default-release"
            if let Ok(entries) = std::fs::read_dir(&profiles_dir) {
                for entry in entries.flatten() {
                    let dir_name = entry.file_name().to_string_lossy().to_string();
                    if dir_name.ends_with(name) && entry.path().is_dir() {
                        return Ok(entry.path());
                    }
                }
            }

            return Err(BrowserExError::ProfileNotFound {
                browser: "Firefox".into(),
                profile: name.into(),
            });
        }

        // Auto-detect: prefer "default-release", then any "*.default*"
        if let Ok(entries) = std::fs::read_dir(&profiles_dir) {
            let mut candidates: Vec<_> = entries.flatten().filter(|e| e.path().is_dir()).collect();

            // Sort by preference
            candidates.sort_by_key(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                if name.ends_with(".default-release") {
                    0
                } else if name.contains(".default") {
                    1
                } else {
                    2
                }
            });

            if let Some(best) = candidates.first() {
                return Ok(best.path());
            }
        }

        Err(BrowserExError::ProfileNotFound {
            browser: "Firefox".into(),
            profile: "(auto-detect)".into(),
        })
    }
}

impl CookieProvider for FirefoxProvider {
    fn browser(&self) -> BrowserName {
        BrowserName::Firefox
    }

    fn is_available(&self) -> bool {
        Self::profiles_dir().map(|p| p.exists()).unwrap_or(false)
    }

    fn extract(
        &self,
        hosts: &[String],
        names: &[String],
        profile: Option<&str>,
        include_expired: bool,
        _timeout_ms: u64,
    ) -> Result<GetCookiesResult> {
        debug!("Firefox: starting cookie extraction");

        let profile_dir = Self::find_profile(profile)?;
        let db_path = profile_dir.join("cookies.sqlite");

        if !db_path.exists() {
            return Err(BrowserExError::CookieDbNotFound {
                path: db_path.display().to_string(),
            });
        }

        query_firefox_cookies(
            &db_path,
            hosts,
            names,
            include_expired,
            &profile_dir
                .file_name()
                .unwrap_or_default()
                .to_string_lossy(),
        )
    }
}

fn query_firefox_cookies(
    db_path: &Path,
    hosts: &[String],
    names: &[String],
    include_expired: bool,
    profile_name: &str,
) -> Result<GetCookiesResult> {
    let mut warnings = Vec::new();

    // Copy DB to temp directory
    let tmp_dir = tempfile::tempdir()?;
    let tmp_db = tmp_dir.path().join("cookies.sqlite");
    std::fs::copy(db_path, &tmp_db)?;

    // Copy WAL/SHM sidecars
    let db_dir = db_path.parent().unwrap_or(Path::new("."));
    for ext in &["-wal", "-shm"] {
        let sidecar = db_dir.join(format!("cookies.sqlite{ext}"));
        if sidecar.exists() {
            let tmp_sidecar = tmp_dir.path().join(format!("cookies.sqlite{ext}"));
            let _ = std::fs::copy(&sidecar, &tmp_sidecar);
        }
    }

    let conn = rusqlite::Connection::open_with_flags(
        &tmp_db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;

    let host_where = host_match::build_firefox_host_where(hosts);
    let expiry_clause = if include_expired {
        String::new()
    } else {
        let now = chrono::Utc::now().timestamp();
        format!(" AND (expiry = 0 OR expiry > {now})")
    };

    let sql = format!(
        "SELECT name, value, host, path, expiry, isSecure, isHttpOnly, sameSite \
         FROM moz_cookies \
         WHERE {host_where}{expiry_clause} \
         ORDER BY expiry DESC"
    );

    debug!("Firefox SQL: {}", sql);

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], |row| {
        Ok(FirefoxRow {
            name: row.get(0)?,
            value: row.get(1)?,
            host: row.get(2)?,
            path: row.get(3)?,
            expiry: row.get(4)?,
            is_secure: row.get(5)?,
            is_http_only: row.get(6)?,
            same_site: row.get(7)?,
        })
    })?;

    let mut cookies = Vec::new();

    for row_result in rows {
        let row = match row_result {
            Ok(r) => r,
            Err(e) => {
                warnings.push(format!("failed to read Firefox cookie row: {e}"));
                continue;
            }
        };

        if !names.is_empty() && !names.iter().any(|n| n == &row.name) {
            continue;
        }

        let same_site = match row.same_site {
            0 => Some(SameSite::None),
            1 => Some(SameSite::Lax),
            2 => Some(SameSite::Strict),
            _ => None,
        };

        cookies.push(Cookie {
            name: row.name,
            value: SecretValue::new(row.value),
            domain: row.host,
            path: row.path,
            expires: epoch::firefox_to_unix(row.expiry),
            secure: row.is_secure != 0,
            http_only: row.is_http_only != 0,
            same_site,
            source: Some(CookieSource {
                browser: BrowserName::Firefox,
                profile: profile_name.to_string(),
                method: Some("sqlite".to_string()),
            }),
        });
    }

    debug!("Firefox: extracted {} cookies", cookies.len());

    Ok(GetCookiesResult { cookies, warnings })
}

struct FirefoxRow {
    name: String,
    value: String,
    host: String,
    path: String,
    expiry: i64,
    is_secure: i32,
    is_http_only: i32,
    same_site: i32,
}
