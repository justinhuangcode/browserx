# Troubleshooting

## Browser Not Found

**Symptom:** `browserx browsers` shows no browsers detected.

**macOS:**
- Chrome: `~/Library/Application Support/Google/Chrome/`
- Firefox: `~/Library/Application Support/Firefox/Profiles/`
- Safari: `~/Library/Cookies/Cookies.binarycookies`

**Linux:**
- Chrome: `~/.config/google-chrome/`
- Firefox: `~/.mozilla/firefox/`
- Brave: `~/.config/BraveSoftware/Brave-Browser/`

**Windows:**
- Chrome: `%LOCALAPPDATA%\Google\Chrome\User Data\`
- Firefox: `%APPDATA%\Mozilla\Firefox\Profiles\`
- Edge: `%LOCALAPPDATA%\Microsoft\Edge\User Data\`

## Keychain Access Prompts (macOS)

**Symptom:** macOS prompts for keychain access when extracting Chrome/Edge cookies.

**Solution:** Click "Always Allow" when prompted. This grants browserx access to the `Chrome Safe Storage` keychain entry. Alternatively, unlock your keychain before running: `security unlock-keychain ~/Library/Keychains/login.keychain-db`

## Encrypted Cookies Return Empty

**Symptom:** Cookies are found but values are empty.

**Cause:** The decryption key could not be obtained.

**macOS:** Ensure keychain is unlocked.
**Linux:** Ensure a keyring service is running (GNOME Keyring or KWallet). If no keyring is available, browserx falls back to the `"peanuts"` key (Chromium v10 format).
**Windows:** DPAPI should work automatically for the current user.

## Linux Keyring Not Available

**Symptom:** Warning about keyring not available on Linux.

**Solutions:**
1. Install and start GNOME Keyring: `sudo apt install gnome-keyring`
2. Or install KWallet: `sudo apt install kwalletmanager`
3. browserx will fall back to the v10 "peanuts" key, which works for most Chromium cookies

## Permission Errors on Vault

**Symptom:** Cannot read/write vault files.

**Solution:** Ensure `~/.browserx/vault/` is owned by your user:
```bash
chmod 700 ~/.browserx/vault/
chmod 600 ~/.browserx/vault/master.key
chmod 600 ~/.browserx/vault/vault.enc
```

## Profile Not Found

**Symptom:** Cookies are empty for a specific browser.

**Solution:** Check available profiles:
```bash
# Chrome profiles
ls ~/Library/Application\ Support/Google/Chrome/ | grep -E "^(Default|Profile)"

# Use a specific profile
browserx get --url https://github.com --browser chrome --profile "Profile 1"
```

## Firefox Database Locked

**Symptom:** SQLite error when reading Firefox cookies.

**Cause:** Firefox is running and has a lock on the database.

**Solution:** browserx copies the database to a temp directory to avoid locks. If this still fails, close Firefox temporarily or use `--browser chrome` instead.
