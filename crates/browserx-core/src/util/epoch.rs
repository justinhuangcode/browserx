/// Epoch conversion utilities for browser cookie timestamps.
///
/// Different browsers use different epoch bases:
/// - **Chromium**: Microseconds since 1601-01-01 (Windows FILETIME)
/// - **Firefox**: Unix epoch seconds
/// - **Safari**: Seconds since 2001-01-01 (Mac absolute time)

/// Delta between Windows FILETIME epoch (1601) and Unix epoch (1970) in seconds.
const WINDOWS_EPOCH_DELTA_SECS: i64 = 11_644_473_600;

/// Delta between Mac absolute time epoch (2001) and Unix epoch (1970) in seconds.
const MAC_EPOCH_DELTA_SECS: i64 = 978_307_200;

/// Convert a Chromium cookie timestamp to Unix epoch seconds.
///
/// Chromium stores `expires_utc` as microseconds since 1601-01-01.
/// A value of 0 means "session cookie" (no expiry).
///
/// # Examples
/// ```
/// use browserx_core::util::epoch::chromium_to_unix;
/// // 0 = session cookie
/// assert_eq!(chromium_to_unix(0), None);
/// // 13_300_000_000_000_000 = some future date
/// let unix = chromium_to_unix(13_300_000_000_000_000);
/// assert!(unix.is_some());
/// ```
pub fn chromium_to_unix(chromium_us: i64) -> Option<i64> {
    if chromium_us <= 0 {
        return None;
    }

    let unix_secs = chromium_us / 1_000_000 - WINDOWS_EPOCH_DELTA_SECS;

    // Sanity check: should be a reasonable date (after 2000, before 2100)
    if unix_secs < 946_684_800 || unix_secs > 4_102_444_800 {
        return None;
    }

    Some(unix_secs)
}

/// Convert a Safari Mac absolute time to Unix epoch seconds.
///
/// Safari stores expiry as seconds (f64) since 2001-01-01T00:00:00Z.
/// A value <= 0 means "session cookie".
pub fn safari_to_unix(mac_time: f64) -> Option<i64> {
    if mac_time <= 0.0 {
        return None;
    }

    let unix_secs = mac_time as i64 + MAC_EPOCH_DELTA_SECS;

    if unix_secs < 946_684_800 || unix_secs > 4_102_444_800 {
        return None;
    }

    Some(unix_secs)
}

/// Firefox stores expiry as Unix epoch seconds directly.
/// A value of 0 means "session cookie".
pub fn firefox_to_unix(expiry: i64) -> Option<i64> {
    if expiry <= 0 {
        return None;
    }

    Some(expiry)
}

/// Normalize any raw expiration value to Unix epoch seconds.
///
/// Heuristic detection:
/// - > 10^16: Chromium microseconds (since 1601)
/// - > 10^12: Milliseconds since Unix epoch
/// - > 10^9: Seconds since Unix epoch
/// - Otherwise: invalid
pub fn normalize_expiration(raw: i64) -> Option<i64> {
    if raw <= 0 {
        return None;
    }

    // Chromium: microseconds since 1601-01-01
    if raw > 10_000_000_000_000_000 {
        return chromium_to_unix(raw);
    }

    // Chromium alternate: sometimes stored as microseconds since Unix epoch
    if raw > 1_000_000_000_000_000 {
        let secs = raw / 1_000_000;
        return Some(secs);
    }

    // Milliseconds since Unix epoch
    if raw > 1_000_000_000_000 {
        return Some(raw / 1000);
    }

    // Seconds since Unix epoch
    if raw > 1_000_000_000 {
        return Some(raw);
    }

    None
}

/// Check if a cookie has expired based on its Unix timestamp.
pub fn is_expired(expires_unix: Option<i64>) -> bool {
    match expires_unix {
        None => false, // Session cookies don't expire
        Some(ts) => {
            let now = chrono::Utc::now().timestamp();
            ts < now
        }
    }
}

/// Human-readable "expires in" string.
pub fn expires_in_human(expires_unix: i64) -> String {
    let now = chrono::Utc::now().timestamp();
    let diff = expires_unix - now;

    if diff < 0 {
        let abs = -diff;
        return format!("expired {}ago", duration_human(abs));
    }

    format!("in {}", duration_human(diff))
}

fn duration_human(secs: i64) -> String {
    if secs < 60 {
        return format!("{secs}s ");
    }
    if secs < 3600 {
        return format!("{}m ", secs / 60);
    }
    if secs < 86400 {
        let hours = secs / 3600;
        let mins = (secs % 3600) / 60;
        return format!("{hours}h {mins}m ");
    }

    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    format!("{days}d {hours}h ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chromium_zero_is_session() {
        assert_eq!(chromium_to_unix(0), None);
    }

    #[test]
    fn chromium_valid_timestamp() {
        // 2025-01-01T00:00:00Z in Chromium format
        let chromium = (1_735_689_600 + WINDOWS_EPOCH_DELTA_SECS) * 1_000_000;
        let unix = chromium_to_unix(chromium).unwrap();
        assert_eq!(unix, 1_735_689_600);
    }

    #[test]
    fn safari_zero_is_session() {
        assert_eq!(safari_to_unix(0.0), None);
    }

    #[test]
    fn safari_valid_timestamp() {
        // 2025-01-01 in Safari time = Unix - MAC_EPOCH_DELTA
        let safari_time = (1_735_689_600 - MAC_EPOCH_DELTA_SECS) as f64;
        let unix = safari_to_unix(safari_time).unwrap();
        assert_eq!(unix, 1_735_689_600);
    }

    #[test]
    fn firefox_zero_is_session() {
        assert_eq!(firefox_to_unix(0), None);
    }

    #[test]
    fn firefox_passthrough() {
        assert_eq!(firefox_to_unix(1_735_689_600), Some(1_735_689_600));
    }

    #[test]
    fn normalize_chromium_microseconds() {
        let raw = (1_735_689_600 + WINDOWS_EPOCH_DELTA_SECS) * 1_000_000;
        let unix = normalize_expiration(raw).unwrap();
        assert_eq!(unix, 1_735_689_600);
    }

    #[test]
    fn normalize_milliseconds() {
        let raw = 1_735_689_600_000;
        let unix = normalize_expiration(raw).unwrap();
        assert_eq!(unix, 1_735_689_600);
    }

    #[test]
    fn normalize_seconds() {
        let raw = 1_735_689_600;
        let unix = normalize_expiration(raw).unwrap();
        assert_eq!(unix, 1_735_689_600);
    }

    #[test]
    fn duration_human_seconds() {
        assert_eq!(duration_human(30), "30s ");
    }

    #[test]
    fn duration_human_minutes() {
        assert_eq!(duration_human(150), "2m ");
    }

    #[test]
    fn duration_human_hours() {
        assert_eq!(duration_human(7260), "2h 1m ");
    }

    #[test]
    fn duration_human_days() {
        assert_eq!(duration_human(90000), "1d 1h ");
    }
}
