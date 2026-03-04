//! # browserx-vault
//!
//! Encrypted local storage for browser cookies.
//!
//! Cookies are encrypted at rest using ChaCha20-Poly1305 with a key
//! derived from a randomly generated master key stored alongside the vault.
//!
//! The vault directory is located at `~/.browserx/vault/`.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use anyhow::Result;
use base64::Engine;
use chacha20poly1305::{
    aead::{Aead, KeyInit, OsRng},
    ChaCha20Poly1305, Nonce,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use tracing::debug;

use browserx_core::Cookie;

/// A vault entry summary (for listing).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultEntry {
    pub url: String,
    pub label: Option<String>,
    pub cookie_count: usize,
    pub stored_at: String,
    pub expires_at: String,
}

/// Internal vault data (encrypted on disk).
#[derive(Debug, Serialize, Deserialize)]
struct VaultData {
    entries: HashMap<String, VaultRecord>,
}

#[derive(Debug, Serialize, Deserialize)]
struct VaultRecord {
    url: String,
    label: Option<String>,
    cookies_json: String, // Serialized Vec<Cookie>
    stored_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
}

/// Encrypted cookie vault.
pub struct Vault {
    vault_dir: PathBuf,
}

impl Vault {
    /// Open or create the vault at the default location (~/.browserx/vault/).
    pub fn open_or_create() -> Result<Self> {
        let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("cannot find home dir"))?;
        let vault_dir = home.join(".browserx").join("vault");
        fs::create_dir_all(&vault_dir)?;

        // Ensure master key exists
        let key_path = vault_dir.join("master.key");
        if !key_path.exists() {
            let key = chacha20poly1305::ChaCha20Poly1305::generate_key(&mut OsRng);
            let key_b64 = base64::engine::general_purpose::STANDARD.encode(key.as_slice());
            fs::write(&key_path, key_b64)?;

            // Set restrictive permissions on Unix
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&key_path, fs::Permissions::from_mode(0o600))?;
            }

            debug!("created new vault master key");
        }

        Ok(Self { vault_dir })
    }

    fn load_key(&self) -> Result<[u8; 32]> {
        let key_b64 = fs::read_to_string(self.vault_dir.join("master.key"))?;
        let key_bytes = base64::engine::general_purpose::STANDARD.decode(key_b64.trim())?;
        let mut key = [0u8; 32];
        if key_bytes.len() != 32 {
            anyhow::bail!(
                "invalid master key length: expected 32, got {}",
                key_bytes.len()
            );
        }
        key.copy_from_slice(&key_bytes);
        Ok(key)
    }

    fn data_path(&self) -> PathBuf {
        self.vault_dir.join("vault.enc")
    }

    fn load_data(&self) -> Result<VaultData> {
        let data_path = self.data_path();
        if !data_path.exists() {
            return Ok(VaultData {
                entries: HashMap::new(),
            });
        }

        let encrypted = fs::read(&data_path)?;
        let key = self.load_key()?;

        // First 12 bytes are nonce, rest is ciphertext
        if encrypted.len() < 12 {
            anyhow::bail!("vault data too short");
        }

        let nonce = Nonce::from_slice(&encrypted[..12]);
        let ciphertext = &encrypted[12..];

        let cipher = ChaCha20Poly1305::new_from_slice(&key)?;
        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| anyhow::anyhow!("vault decryption failed: {e}"))?;

        let data: VaultData = serde_json::from_slice(&plaintext)?;
        Ok(data)
    }

    fn save_data(&self, data: &VaultData) -> Result<()> {
        let key = self.load_key()?;
        let plaintext = serde_json::to_vec(data)?;

        let cipher = ChaCha20Poly1305::new_from_slice(&key)?;

        // Generate random nonce
        use chacha20poly1305::aead::AeadCore;
        let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);

        let ciphertext = cipher
            .encrypt(&nonce, plaintext.as_ref())
            .map_err(|e| anyhow::anyhow!("vault encryption failed: {e}"))?;

        // Write nonce + ciphertext
        let mut output = nonce.to_vec();
        output.extend_from_slice(&ciphertext);

        fs::write(self.data_path(), output)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(self.data_path(), fs::Permissions::from_mode(0o600))?;
        }

        Ok(())
    }

    /// Store cookies in the vault.
    pub fn store(
        &self,
        url: &str,
        cookies: &[Cookie],
        ttl_str: &str,
        label: Option<&str>,
    ) -> Result<()> {
        let ttl = parse_duration(ttl_str)?;
        let now = Utc::now();
        let expires_at = now + ttl;

        let cookies_json = serde_json::to_string(cookies)?;

        let record = VaultRecord {
            url: url.to_string(),
            label: label.map(String::from),
            cookies_json,
            stored_at: now,
            expires_at,
        };

        let mut data = self.load_data()?;
        data.entries.insert(url.to_string(), record);
        self.save_data(&data)?;

        debug!(
            "stored {} cookies for {} (expires: {})",
            cookies.len(),
            url,
            expires_at
        );
        Ok(())
    }

    /// Retrieve cookies from the vault.
    pub fn get(&self, url: &str) -> Result<Vec<Cookie>> {
        let data = self.load_data()?;

        match data.entries.get(url) {
            Some(record) => {
                if record.expires_at < Utc::now() {
                    return Ok(Vec::new());
                }
                let cookies: Vec<Cookie> = serde_json::from_str(&record.cookies_json)?;
                Ok(cookies)
            }
            None => Ok(Vec::new()),
        }
    }

    /// List all vault entries.
    pub fn list(&self) -> Result<Vec<VaultEntry>> {
        let data = self.load_data()?;

        let mut entries: Vec<VaultEntry> = data
            .entries
            .values()
            .map(|r| {
                let cookies: Vec<Cookie> =
                    serde_json::from_str(&r.cookies_json).unwrap_or_default();
                VaultEntry {
                    url: r.url.clone(),
                    label: r.label.clone(),
                    cookie_count: cookies.len(),
                    stored_at: r.stored_at.to_rfc3339(),
                    expires_at: r.expires_at.to_rfc3339(),
                }
            })
            .collect();

        entries.sort_by(|a, b| a.url.cmp(&b.url));
        Ok(entries)
    }

    /// Remove expired entries. Returns count removed.
    pub fn clean(&self) -> Result<usize> {
        let mut data = self.load_data()?;
        let now = Utc::now();
        let before = data.entries.len();

        data.entries.retain(|_, r| r.expires_at > now);

        let removed = before - data.entries.len();
        if removed > 0 {
            self.save_data(&data)?;
        }

        Ok(removed)
    }

    /// Remove a specific entry.
    pub fn remove(&self, url: &str) -> Result<()> {
        let mut data = self.load_data()?;
        data.entries.remove(url);
        self.save_data(&data)?;
        Ok(())
    }
}

/// Parse a human-readable duration string like "24h", "7d", "1h30m".
fn parse_duration(s: &str) -> Result<Duration> {
    let s = s.trim().to_lowercase();
    let mut total_secs: i64 = 0;
    let mut current_num = String::new();

    for ch in s.chars() {
        if ch.is_ascii_digit() {
            current_num.push(ch);
        } else {
            let num: i64 = current_num
                .parse()
                .map_err(|_| anyhow::anyhow!("invalid duration: {s}"))?;
            current_num.clear();

            match ch {
                's' => total_secs += num,
                'm' => total_secs += num * 60,
                'h' => total_secs += num * 3600,
                'd' => total_secs += num * 86400,
                'w' => total_secs += num * 604800,
                _ => anyhow::bail!("unknown duration unit: {ch} in '{s}'"),
            }
        }
    }

    if !current_num.is_empty() {
        // Default to hours if no unit
        let num: i64 = current_num
            .parse()
            .map_err(|_| anyhow::anyhow!("invalid duration: {s}"))?;
        total_secs += num * 3600;
    }

    if total_secs <= 0 {
        anyhow::bail!("duration must be positive: {s}");
    }

    Ok(Duration::seconds(total_secs))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_duration_hours() {
        let d = parse_duration("24h").unwrap();
        assert_eq!(d.num_seconds(), 86400);
    }

    #[test]
    fn parse_duration_days() {
        let d = parse_duration("7d").unwrap();
        assert_eq!(d.num_seconds(), 604800);
    }

    #[test]
    fn parse_duration_compound() {
        let d = parse_duration("1h30m").unwrap();
        assert_eq!(d.num_seconds(), 5400);
    }

    #[test]
    fn parse_duration_minutes() {
        let d = parse_duration("30m").unwrap();
        assert_eq!(d.num_seconds(), 1800);
    }

    #[test]
    fn parse_duration_bare_number_defaults_to_hours() {
        let d = parse_duration("2").unwrap();
        assert_eq!(d.num_seconds(), 7200);
    }

    #[test]
    fn parse_duration_invalid() {
        assert!(parse_duration("").is_err());
        assert!(parse_duration("abc").is_err());
    }
}
