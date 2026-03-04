use crate::error::{BrowserExError, Result};
use crate::providers::chromium::ChromiumConfig;

/// Obtain the decryption key for Chromium cookie values.
///
/// Platform-specific:
/// - **macOS**: Keychain → PBKDF2 (1003 iterations) → AES-128-CBC key
/// - **Linux**: Keyring or hardcoded "peanuts" → PBKDF2 (1 iteration) → AES-128-CBC key
/// - **Windows**: Local State → DPAPI unwrap → AES-256-GCM master key
pub fn get_decryption_key(config: &ChromiumConfig, _timeout_ms: u64) -> Result<Vec<u8>> {
    #[cfg(target_os = "macos")]
    {
        get_key_macos(config)
    }

    #[cfg(target_os = "linux")]
    {
        get_key_linux(config)
    }

    #[cfg(target_os = "windows")]
    {
        get_key_windows(config)
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        let _ = config;
        Err(BrowserExError::PlatformNotSupported {
            operation: "chromium key derivation".into(),
            platform: std::env::consts::OS.into(),
        })
    }
}

// ─── macOS: Keychain → PBKDF2 → AES-128-CBC key ───────────────────────────

#[cfg(target_os = "macos")]
fn get_key_macos(config: &ChromiumConfig) -> Result<Vec<u8>> {
    use security_framework::passwords::get_generic_password;

    let password =
        get_generic_password(config.keychain_service, config.keychain_account).map_err(|e| {
            BrowserExError::KeychainAccess {
                reason: format!("{}: {e}", config.browser.display_name()),
            }
        })?;

    derive_aes128_key(&password, 1003)
}

// ─── Linux: Keyring → PBKDF2 → AES-128-CBC key ────────────────────────────

#[cfg(target_os = "linux")]
fn get_key_linux(config: &ChromiumConfig) -> Result<Vec<u8>> {
    // Try keyring first (v11), then fallback to "peanuts" (v10)
    match try_linux_keyring(config) {
        Ok(password) => derive_aes128_key(password.as_bytes(), 1),
        Err(e) => {
            tracing::debug!(
                "{}: keyring access failed ({e}), falling back to v10 'peanuts'",
                config.browser.display_name()
            );
            // v10 hardcoded password
            derive_aes128_key(b"peanuts", 1)
        }
    }
}

#[cfg(target_os = "linux")]
fn try_linux_keyring(config: &ChromiumConfig) -> Result<String> {
    // Try GNOME keyring via secret-tool
    let output = std::process::Command::new("secret-tool")
        .args(["lookup", "application", config.linux_keyring_app])
        .output()
        .map_err(|e| BrowserExError::KeychainAccess {
            reason: format!("secret-tool not found: {e}"),
        })?;

    if output.status.success() {
        let password = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !password.is_empty() {
            return Ok(password);
        }
    }

    Err(BrowserExError::KeychainAccess {
        reason: "keyring returned empty password".into(),
    })
}

// ─── Windows: Local State → DPAPI → AES-256-GCM master key ────────────────

#[cfg(target_os = "windows")]
fn get_key_windows(config: &ChromiumConfig) -> Result<Vec<u8>> {
    use crate::providers::chromium::paths;
    use base64::Engine;

    // Read Local State JSON
    let local_state_path = paths::local_state_path(config.browser)?;
    let local_state_str =
        std::fs::read_to_string(&local_state_path).map_err(|e| BrowserExError::KeychainAccess {
            reason: format!("cannot read Local State: {e}"),
        })?;

    let local_state: serde_json::Value =
        serde_json::from_str(&local_state_str).map_err(|e| BrowserExError::KeychainAccess {
            reason: format!("cannot parse Local State JSON: {e}"),
        })?;

    // Extract encrypted key
    let encrypted_key_b64 = local_state["os_crypt"]["encrypted_key"]
        .as_str()
        .ok_or_else(|| BrowserExError::KeychainAccess {
            reason: "os_crypt.encrypted_key not found in Local State".into(),
        })?;

    let encrypted_key = base64::engine::general_purpose::STANDARD
        .decode(encrypted_key_b64)
        .map_err(|e| BrowserExError::KeychainAccess {
            reason: format!("base64 decode of encrypted_key failed: {e}"),
        })?;

    // Strip "DPAPI" prefix (5 bytes)
    if encrypted_key.len() < 5 || &encrypted_key[..5] != b"DPAPI" {
        return Err(BrowserExError::KeychainAccess {
            reason: "encrypted_key does not have DPAPI prefix".into(),
        });
    }

    let dpapi_blob = &encrypted_key[5..];

    // DPAPI unprotect
    dpapi_unprotect(dpapi_blob)
}

#[cfg(target_os = "windows")]
fn dpapi_unprotect(data: &[u8]) -> Result<Vec<u8>> {
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Cryptography::{CryptUnprotectData, CRYPT_INTEGER_BLOB};

    unsafe {
        let mut blob_in = CRYPT_INTEGER_BLOB {
            cbData: data.len() as u32,
            pbData: data.as_ptr() as *mut u8,
        };
        let mut blob_out = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: std::ptr::null_mut(),
        };

        let success = CryptUnprotectData(
            &mut blob_in,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            0,
            &mut blob_out,
        );

        if success == 0 {
            return Err(BrowserExError::Decryption {
                browser: "chromium".into(),
                platform: "windows".into(),
                reason: "CryptUnprotectData failed".into(),
            });
        }

        let result = std::slice::from_raw_parts(blob_out.pbData, blob_out.cbData as usize).to_vec();

        LocalFree(blob_out.pbData as _);

        Ok(result)
    }
}

// ─── Shared: AES-128-CBC key derivation via PBKDF2 ────────────────────────

fn derive_aes128_key(password: &[u8], iterations: u32) -> Result<Vec<u8>> {
    let mut key = [0u8; 16];
    pbkdf2::pbkdf2_hmac::<sha1::Sha1>(password, b"saltysalt", iterations, &mut key);
    Ok(key.to_vec())
}

// ─── Cookie value decryption ───────────────────────────────────────────────

/// Decrypt a Chromium encrypted cookie value.
///
/// Returns `None` if the value is not encrypted (plain text).
pub fn decrypt_cookie_value(encrypted: &[u8], key: &[u8]) -> Result<Option<String>> {
    if encrypted.is_empty() {
        return Ok(None);
    }

    // Check for version prefix
    let (version, payload) = if encrypted.len() >= 3 {
        let prefix = &encrypted[..3];
        if prefix == b"v10" || prefix == b"v11" {
            (Some(prefix.to_vec()), &encrypted[3..])
        } else {
            (None, encrypted)
        }
    } else {
        (None, encrypted)
    };

    match version.as_deref() {
        // v10/v11: AES-128-CBC (macOS/Linux)
        Some(b"v10") | Some(b"v11") => {
            if key.len() != 16 {
                return Err(BrowserExError::Decryption {
                    browser: "chromium".into(),
                    platform: std::env::consts::OS.into(),
                    reason: format!("expected 16-byte key for AES-128-CBC, got {}", key.len()),
                });
            }
            decrypt_aes128_cbc(payload, key)
        }
        // Windows: AES-256-GCM
        _ if key.len() == 32 && payload.len() > 15 => decrypt_aes256_gcm(payload, key),
        // Not encrypted -- treat as plain text
        _ => {
            let text = String::from_utf8_lossy(encrypted).to_string();
            Ok(Some(text))
        }
    }
}

/// AES-128-CBC decryption (macOS/Linux).
fn decrypt_aes128_cbc(ciphertext: &[u8], key: &[u8]) -> Result<Option<String>> {
    use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, KeyIvInit};
    type Aes128CbcDec = cbc::Decryptor<aes::Aes128>;

    if ciphertext.is_empty() {
        return Ok(None);
    }

    // IV is 16 bytes of 0x20 (space)
    let iv = [0x20u8; 16];

    let mut buf = ciphertext.to_vec();
    let plaintext = Aes128CbcDec::new(key.into(), &iv.into())
        .decrypt_padded_mut::<Pkcs7>(&mut buf)
        .map_err(|e| BrowserExError::Decryption {
            browser: "chromium".into(),
            platform: std::env::consts::OS.into(),
            reason: format!("AES-128-CBC decryption failed: {e}"),
        })?;

    // Chromium >= 24: strip 32-byte SHA-256 hash prefix
    let value_bytes = if plaintext.len() > 32 {
        &plaintext[32..]
    } else {
        plaintext
    };

    Ok(Some(String::from_utf8_lossy(value_bytes).to_string()))
}

/// AES-256-GCM decryption (Windows).
fn decrypt_aes256_gcm(payload: &[u8], key: &[u8]) -> Result<Option<String>> {
    use aes_gcm::{aead::Aead, Aes256Gcm, KeyInit, Nonce};

    // Layout: 12-byte nonce + ciphertext (includes 16-byte auth tag)
    if payload.len() < 28 {
        return Err(BrowserExError::Decryption {
            browser: "chromium".into(),
            platform: "windows".into(),
            reason: "payload too short for AES-256-GCM".into(),
        });
    }

    let nonce = &payload[..12];
    let ciphertext_with_tag = &payload[12..];

    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| BrowserExError::Decryption {
        browser: "chromium".into(),
        platform: "windows".into(),
        reason: format!("invalid AES-256-GCM key: {e}"),
    })?;

    let plaintext = cipher
        .decrypt(Nonce::from_slice(nonce), ciphertext_with_tag)
        .map_err(|e| BrowserExError::Decryption {
            browser: "chromium".into(),
            platform: "windows".into(),
            reason: format!("AES-256-GCM decryption failed: {e}"),
        })?;

    // Chromium >= 24: strip 32-byte SHA-256 hash prefix
    let value_bytes = if plaintext.len() > 32 {
        &plaintext[32..]
    } else {
        &plaintext
    };

    Ok(Some(String::from_utf8_lossy(value_bytes).to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derive_key_produces_16_bytes() {
        let key = derive_aes128_key(b"test-password", 1003).unwrap();
        assert_eq!(key.len(), 16);
    }

    #[test]
    fn derive_key_deterministic() {
        let key1 = derive_aes128_key(b"password", 1).unwrap();
        let key2 = derive_aes128_key(b"password", 1).unwrap();
        assert_eq!(key1, key2);
    }

    #[test]
    fn empty_value_returns_none() {
        let key = derive_aes128_key(b"test", 1).unwrap();
        let result = decrypt_cookie_value(&[], &key).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn plain_text_passthrough() {
        let key = derive_aes128_key(b"test", 1).unwrap();
        let result = decrypt_cookie_value(b"hello", &key).unwrap();
        assert_eq!(result.unwrap(), "hello");
    }
}
