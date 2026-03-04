/// Check if a cookie's domain attribute matches a target host.
///
/// Cookie domain matching follows RFC 6265 Section 5.1.3:
/// - Exact match: cookie domain == host
/// - Suffix match: cookie domain starts with "." and host ends with it
///
/// # Examples
/// ```
/// use browserx_core::util::host_match::domain_matches;
/// assert!(domain_matches(".google.com", "accounts.google.com"));
/// assert!(domain_matches("google.com", "google.com"));
/// assert!(!domain_matches(".google.com", "evil-google.com"));
/// ```
pub fn domain_matches(cookie_domain: &str, host: &str) -> bool {
    let cookie_domain = cookie_domain.to_lowercase();
    let host = host.to_lowercase();

    // Exact match
    if cookie_domain == host {
        return true;
    }

    // Leading dot match: ".example.com" matches "sub.example.com"
    if let Some(stripped) = cookie_domain.strip_prefix('.') {
        if host == stripped {
            return true;
        }
        if host.ends_with(&cookie_domain) {
            return true;
        }
    }

    false
}

/// Build a SQL WHERE clause fragment for matching cookie host_key values
/// against a set of target hosts.
///
/// Returns a string like:
/// ```sql
/// (host_key = 'example.com' OR host_key = '.example.com' OR host_key LIKE '%.example.com')
/// ```
pub fn build_chromium_host_where(hosts: &[String]) -> String {
    let mut clauses = Vec::new();

    for host in hosts {
        // Exact match
        clauses.push(format!("host_key = '{host}'"));
        // Dot-prefixed match
        clauses.push(format!("host_key = '.{host}'"));
        // Subdomain wildcard
        clauses.push(format!("host_key LIKE '%.{host}'"));
    }

    if clauses.is_empty() {
        return "1=0".to_string();
    }

    format!("({})", clauses.join(" OR "))
}

/// Build a SQL WHERE clause fragment for Firefox's moz_cookies table.
///
/// Firefox uses `host` column instead of `host_key`.
pub fn build_firefox_host_where(hosts: &[String]) -> String {
    let mut clauses = Vec::new();

    for host in hosts {
        clauses.push(format!("host = '{host}'"));
        clauses.push(format!("host = '.{host}'"));
        clauses.push(format!("host LIKE '%.{host}'"));
    }

    if clauses.is_empty() {
        return "1=0".to_string();
    }

    format!("({})", clauses.join(" OR "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_match() {
        assert!(domain_matches("example.com", "example.com"));
    }

    #[test]
    fn dot_prefix_matches_subdomain() {
        assert!(domain_matches(".google.com", "accounts.google.com"));
        assert!(domain_matches(".google.com", "mail.google.com"));
    }

    #[test]
    fn dot_prefix_matches_exact_without_dot() {
        assert!(domain_matches(".google.com", "google.com"));
    }

    #[test]
    fn no_partial_match() {
        assert!(!domain_matches(".google.com", "evil-google.com"));
        assert!(!domain_matches("google.com", "notgoogle.com"));
    }

    #[test]
    fn case_insensitive() {
        assert!(domain_matches(".Google.COM", "accounts.google.com"));
    }

    #[test]
    fn where_clause_generation() {
        let hosts = vec!["example.com".into()];
        let clause = build_chromium_host_where(&hosts);
        assert!(clause.contains("host_key = 'example.com'"));
        assert!(clause.contains("host_key = '.example.com'"));
        assert!(clause.contains("host_key LIKE '%.example.com'"));
    }

    #[test]
    fn empty_hosts_returns_false_clause() {
        let clause = build_chromium_host_where(&[]);
        assert_eq!(clause, "1=0");
    }
}
