use url::Url;

/// Extract the origin (scheme + host) from a URL string.
///
/// Returns the host without scheme for cookie matching purposes.
///
/// # Examples
/// ```
/// use browserx_core::util::origin::extract_hosts;
/// let hosts = extract_hosts("https://accounts.google.com/signin").unwrap();
/// assert!(hosts.contains(&"accounts.google.com".to_string()));
/// assert!(hosts.contains(&"google.com".to_string()));
/// ```
pub fn extract_hosts(url_str: &str) -> Result<Vec<String>, String> {
    let url = Url::parse(url_str).map_err(|e| format!("invalid URL '{url_str}': {e}"))?;

    let host = url
        .host_str()
        .ok_or_else(|| format!("URL has no host: {url_str}"))?
        .to_string();

    let mut hosts = vec![host.clone()];

    // Expand parent domains for cookie matching.
    // e.g., "accounts.google.com" -> also match "google.com"
    let parts: Vec<&str> = host.split('.').collect();
    if parts.len() > 2 {
        for i in 1..parts.len().saturating_sub(1) {
            let parent = parts[i..].join(".");
            if !hosts.contains(&parent) {
                hosts.push(parent);
            }
        }
    }

    Ok(hosts)
}

/// Normalize multiple URL strings into a deduplicated list of hosts.
pub fn normalize_origins(urls: &[String]) -> Result<Vec<String>, String> {
    let mut all_hosts = Vec::new();

    for url_str in urls {
        let hosts = extract_hosts(url_str)?;
        for host in hosts {
            if !all_hosts.contains(&host) {
                all_hosts.push(host);
            }
        }
    }

    Ok(all_hosts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_simple_host() {
        let hosts = extract_hosts("https://example.com/path").unwrap();
        assert_eq!(hosts, vec!["example.com"]);
    }

    #[test]
    fn extract_subdomain_expands_parent() {
        let hosts = extract_hosts("https://accounts.google.com/signin").unwrap();
        assert!(hosts.contains(&"accounts.google.com".to_string()));
        assert!(hosts.contains(&"google.com".to_string()));
    }

    #[test]
    fn extract_deep_subdomain() {
        let hosts = extract_hosts("https://a.b.c.example.com/").unwrap();
        assert!(hosts.contains(&"a.b.c.example.com".to_string()));
        assert!(hosts.contains(&"b.c.example.com".to_string()));
        assert!(hosts.contains(&"c.example.com".to_string()));
        assert!(hosts.contains(&"example.com".to_string()));
    }

    #[test]
    fn invalid_url_returns_error() {
        assert!(extract_hosts("not-a-url").is_err());
    }

    #[test]
    fn normalize_deduplicates() {
        let urls = vec![
            "https://accounts.google.com".into(),
            "https://mail.google.com".into(),
        ];
        let hosts = normalize_origins(&urls).unwrap();
        // "google.com" should appear only once
        assert_eq!(
            hosts.iter().filter(|h| *h == "google.com").count(),
            1
        );
    }
}
