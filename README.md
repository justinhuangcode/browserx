# BrowserX

**English** | [中文](./README_CN.md)

[![CI](https://github.com/justinhuangcode/browserx/actions/workflows/ci.yml/badge.svg)](https://github.com/justinhuangcode/browserx/actions/workflows/ci.yml)
[![Release](https://github.com/justinhuangcode/browserx/actions/workflows/release.yml/badge.svg)](https://github.com/justinhuangcode/browserx/actions/workflows/release.yml)
[![Crates.io](https://img.shields.io/crates/v/browserx?style=flat-square)](https://crates.io/crates/browserx)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg?style=flat-square)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg?style=flat-square&logo=rust&logoColor=white)](https://www.rust-lang.org)
[![GitHub Stars](https://img.shields.io/github/stars/justinhuangcode/browserx?style=flat-square&logo=github)](https://github.com/justinhuangcode/browserx/stargazers)
[![Last Commit](https://img.shields.io/github/last-commit/justinhuangcode/browserx?style=flat-square)](https://github.com/justinhuangcode/browserx/commits/main)
[![Issues](https://img.shields.io/github/issues/justinhuangcode/browserx?style=flat-square)](https://github.com/justinhuangcode/browserx/issues)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Linux%20%7C%20Windows-lightgrey?style=flat-square)](https://github.com/justinhuangcode/browserx)

A cross-platform browser cookie extraction CLI for session replay, authentication, and automation. 🍪

A single Rust binary that reads cookie databases from 9 browsers across macOS, Linux, and Windows, decrypts encrypted values using native OS APIs, and outputs in 5 formats. Designed for AI agents, CLI automation, and programmatic authenticated web access.

## Why BrowserX?

AI agents and CLI tools that make authenticated HTTP requests need to **reuse browser login sessions**. They need to extract cookies, build headers, check if sessions are still valid, and retry -- all without a human opening DevTools.

Existing tools don't fit this workflow:

| | browserx | sweet-cookie | cookie-editor | EditThisCookie |
|---|---|---|---|---|
| Designed for AI agents | Yes | No (library only) | No (extension) | No (extension) |
| Single binary, zero runtime | Yes (Rust) | No (Node >= 22) | No (Chrome) | No (Chrome) |
| CLI + library | CLI + Rust crate | Library only | Extension only | Extension only |
| Native OS crypto | Keychain / DPAPI / D-Bus | Shell-outs (`security`, `powershell`) | N/A | N/A |
| Browser support | 9 browsers | 4 browsers | 1 browser | 1 browser |
| Output formats | 5 (JSON/curl/Netscape/ENV/table) | 1 (JSON) | N/A | N/A |
| Encrypted vault | ChaCha20-Poly1305, TTL | No | No | No |
| Session health check | Yes | No | No | No |
| Inline-first design | Yes | Yes | No | No |

**The typical AI agent workflow with browserx:**

```
Agent needs to call an authenticated API
        |
browserx get --url https://github.com --format curl
        |
Agent uses the cookie header in HTTP requests
        |
browserx health --url https://github.com
        |
Agent checks if session is still valid, re-extracts if expired
```

No browser extension. No Node.js runtime. No test framework boilerplate. Just a single binary that reads your cookies and outputs them in the format you need.

## Features

- **9 browsers** -- Chrome, Edge, Firefox, Safari, Brave, Arc, Vivaldi, Opera, Chromium
- **3 platforms** -- macOS, Linux, Windows with platform-specific native crypto
- **5 output formats** -- JSON, curl, Netscape cookie.txt, environment variables, human-readable table
- **Inline-first design** -- Inline cookie payloads (JSON, base64, file) are checked before local browser databases, short-circuiting if they yield cookies
- **Encrypted vault** -- Store extracted cookies locally with ChaCha20-Poly1305 encryption and TTL-based auto-expiration
- **Session health check** -- Inspect cookie status (active, expiring soon, expired) without making HTTP requests
- **Native OS crypto** -- macOS Keychain via `security-framework`, Windows DPAPI via `windows-sys`, Linux Secret Service via D-Bus -- no subprocess calls
- **Secure by default** -- Cookie values wrapped in `SecretValue` (masked in `Debug`/`Display`, zeroed on drop via `zeroize`); never written to logs
- **JSON output** -- Pass `--format json` for machine-readable output on every command
- **WAL-safe reads** -- Copies SQLite databases + WAL/SHM sidecars to temp directory before reading; never locks the browser's live database
- **Cross-platform** -- macOS, Linux, and Windows with auto-detection of browser paths and profiles

## Installation

### Pre-built binaries (recommended)

Download the latest binary for your platform from [GitHub Releases](https://github.com/justinhuangcode/browserx/releases):

| Platform | Archive |
|---|---|
| Linux x86_64 | `browserx-v*-linux-x86_64.tar.gz` |
| Linux ARM64 | `browserx-v*-linux-arm64.tar.gz` |
| macOS Intel | `browserx-v*-macos-x86_64.tar.gz` |
| macOS Apple Silicon | `browserx-v*-macos-arm64.tar.gz` |
| Windows x86_64 | `browserx-v*-windows-x86_64.zip` |

Extract the archive and place the binary in your `$PATH`.

### Homebrew (macOS / Linux)

```bash
brew tap justinhuangcode/tap
brew install browserx
```

### Via Cargo (crates.io)

```bash
cargo install browserx
```

### From source

```bash
git clone https://github.com/justinhuangcode/browserx
cd browserx
cargo install --path crates/browserx-cli
```

**Requirements:** Rust 1.75+ only. No browser installation required -- browserx reads cookie databases directly from disk.

## Quick Start

```bash
# Extract cookies from all detected browsers
browserx get --url https://github.com

# Extract from Chrome, output as curl-compatible header
browserx get --url https://github.com --browser chrome --format curl

# Use with curl directly
curl -b "$(browserx get --url https://github.com --format curl)" https://github.com/api

# Filter by cookie names
browserx get --url https://x.com --names "auth_token,ct0"

# Export as Netscape cookie.txt
browserx get --url https://github.com --format netscape > cookies.txt

# List detected browsers and profiles
browserx browsers

# Check session health
browserx health --url https://github.com

# Store cookies in encrypted vault
browserx vault store --url https://github.com --ttl 24h

# Retrieve from vault (no browser access needed)
browserx vault get --url https://github.com

# Stop
browserx vault clean
```

## Commands

| Command | Description |
|---|---|
| `get` | Extract cookies from browser(s) for a given URL |
| `browsers` | List detected browsers and their profiles |
| `health` | Check session health (active, expiring, expired) |
| `vault store` | Store extracted cookies in encrypted local vault |
| `vault get` | Retrieve cookies from vault |
| `vault list` | List all vault entries with status |
| `vault clean` | Remove expired vault entries |

## Get Flags

| Flag | Default | Description |
|---|---|---|
| `--url <url>` | *(required)* | Target URL to extract cookies for |
| `--browser <name>` | auto-detect | Browser to extract from (chrome, edge, firefox, safari, brave, arc, vivaldi, opera, chromium) |
| `--format <fmt>` | json | Output format: json, curl, netscape, env, table |
| `--names <list>` | *(all)* | Comma-separated cookie name allowlist |
| `--origins <urls>` | *(none)* | Additional origins for OAuth/SSO flows |
| `--mode <mode>` | merge | Merge mode: `merge` (combine all browsers) or `first` (first successful) |
| `--include-expired` | false | Include expired cookies in output |
| `--profile <name>` | Default | Browser profile name |
| `--inline-json <str>` | *(none)* | Inline cookie payload as JSON string |
| `--inline-base64 <str>` | *(none)* | Inline cookie payload as base64-encoded JSON |
| `--inline-file <path>` | *(none)* | Inline cookie payload from file |

## Vault Flags

| Flag | Applies To | Description |
|---|---|---|
| `--url <url>` | `store`, `get` | Target URL |
| `--ttl <duration>` | `store` | Time-to-live: `24h`, `7d`, `1h30m`, etc. |
| `--label <name>` | `store` | Optional label for the entry |
| `--browser <name>` | `store` | Browser to extract from before storing |
| `--format <fmt>` | `get` | Output format for retrieved cookies |

## Output Formats

### JSON (default)

```bash
browserx get --url https://github.com
```

```json
[
  {
    "name": "_gh_sess",
    "value": "abc123...",
    "domain": ".github.com",
    "path": "/",
    "expires": 1735689600,
    "secure": true,
    "httpOnly": true,
    "sameSite": "lax"
  }
]
```

### curl

```bash
browserx get --url https://github.com --format curl
# Output: _gh_sess=abc123...; logged_in=yes
```

Pipe directly to curl:

```bash
curl -b "$(browserx get --url https://github.com --format curl)" https://github.com/api
```

### Netscape (cookie.txt)

```bash
browserx get --url https://github.com --format netscape > cookies.txt
curl --cookie cookies.txt https://github.com
wget --load-cookies cookies.txt https://github.com
```

### Environment Variables

```bash
eval $(browserx get --url https://github.com --format env)
# Sets COOKIE__GH_SESS, COOKIE_LOGGED_IN, etc.
```

### Table (human-readable)

```bash
browserx get --url https://github.com --format table
```

```
NAME                           DOMAIN                    SECURE HTTPONLY  EXPIRES
-------------------------------------------------------------------------------------
_gh_sess                       .github.com               yes    yes      in 14h 32m
logged_in                      .github.com               yes    no       in 365d 0h

Total: 2 cookies
```

## Inline Sources

browserx follows an **inline-first** design: if you provide inline cookie data, it's used immediately without touching any browser database. This is the escape hatch for when local reads fail (app-bound cookies, locked keychain, remote machines).

```bash
# From JSON string
browserx get --url https://x.com --inline-json '{"cookies":[{"name":"a","value":"1","domain":"x.com"}]}'

# From base64-encoded JSON
browserx get --url https://x.com --inline-base64 "eyJjb29raWVzIjpbey..."

# From exported file (compatible with sweet-cookie Chrome extension export)
browserx get --url https://x.com --inline-file ./exported-cookies.json
```

## Session Health Check

```bash
browserx health --url https://github.com
```

```
[OK] https://github.com -- HEALTHY
  Cookies: 5 total, 5 active, 0 expiring soon, 0 expired

  _gh_sess                 .github.com     Active (in 14h 32m)
  logged_in                .github.com     Active (in 365d 0h)
  _device_id               .github.com     Active (in 179d 23h)
  color_mode               .github.com     Session
  tz                       .github.com     Session
```

## Encrypted Vault

Store cookies locally with automatic expiration:

```bash
# Extract from Chrome and store with 24-hour TTL
browserx vault store --url https://github.com --browser chrome --ttl 24h

# Retrieve from vault (no browser access needed)
browserx vault get --url https://github.com --format curl

# List stored entries
browserx vault list

# Clean expired entries
browserx vault clean
```

The vault uses ChaCha20-Poly1305 encryption. The master key is stored at `~/.browserx/vault/master.key` with `0600` permissions (owner-only). Vault data is encrypted at rest in `~/.browserx/vault/vault.enc`.

## Supported Browsers

| Browser | macOS | Linux | Windows | Engine | Encryption |
|---|---|---|---|---|---|
| Google Chrome | Yes | Yes | Yes | Chromium | AES-128-CBC (macOS/Linux), AES-256-GCM (Windows) |
| Microsoft Edge | Yes | Yes | Yes | Chromium | AES-128-CBC (macOS/Linux), AES-256-GCM (Windows) |
| Mozilla Firefox | Yes | Yes | Yes | Gecko | None (plaintext) |
| Apple Safari | Yes | -- | -- | WebKit | None (binary format) |
| Brave | Yes | Yes | Yes | Chromium | AES-128-CBC (macOS/Linux), AES-256-GCM (Windows) |
| Arc | Yes | -- | Yes | Chromium | AES-128-CBC (macOS/Linux), AES-256-GCM (Windows) |
| Vivaldi | Yes | Yes | Yes | Chromium | AES-128-CBC (macOS/Linux), AES-256-GCM (Windows) |
| Opera | Yes | Yes | Yes | Chromium | AES-128-CBC (macOS/Linux), AES-256-GCM (Windows) |
| Chromium | Yes | Yes | Yes | Chromium | AES-128-CBC (macOS/Linux), AES-256-GCM (Windows) |

## Environment Variables

| Variable | Default | Description |
|---|---|---|
| `BROWSERX_BROWSERS` | auto-detect | Comma-separated browser list |
| `BROWSERX_MODE` | merge | `merge` (combine all) or `first` (first successful) |
| `BROWSERX_CHROME_PROFILE` | Default | Chrome profile name |
| `BROWSERX_EDGE_PROFILE` | Default | Edge profile name |
| `BROWSERX_FIREFOX_PROFILE` | *(auto)* | Firefox profile name |
| `BROWSERX_BRAVE_PROFILE` | Default | Brave profile name |

## How It Works

### Chromium-based browsers (Chrome, Edge, Brave, Arc, Vivaldi, Opera, Chromium)

1. **Locate** the cookie SQLite database via platform-specific paths:
   - macOS: `~/Library/Application Support/Google/Chrome/<Profile>/Network/Cookies`
   - Linux: `~/.config/google-chrome/<Profile>/Network/Cookies`
   - Windows: `%LOCALAPPDATA%\Google\Chrome\User Data\<Profile>\Network\Cookies`
2. **Copy** DB + WAL/SHM sidecars to a temp directory (avoids locking the live database)
3. **Obtain** decryption key via native OS API:
   - **macOS**: `security-framework` crate -> Keychain `Chrome Safe Storage` -> PBKDF2 (1003 rounds, SHA-1) -> AES-128-CBC key
   - **Linux**: D-Bus Secret Service API (`secret-tool`) or fallback `"peanuts"` -> PBKDF2 (1 round) -> AES-128-CBC key
   - **Windows**: `Local State` JSON -> `os_crypt.encrypted_key` -> DPAPI (`CryptUnprotectData`) -> AES-256-GCM master key
4. **Query** cookies by host (with parent domain expansion), decrypt encrypted values, filter by name allowlist and expiry, deduplicate by `name|domain|path`

### Firefox

1. **Locate** `cookies.sqlite` in the profile directory (prefers `default-release`)
2. **Copy** to temp dir, query `moz_cookies` table with host matching
3. **No decryption** needed (Firefox stores cookie values in plaintext)

### Safari (macOS only)

1. **Read** `~/Library/Cookies/Cookies.binarycookies` binary file
2. **Parse** the proprietary format: `cook` magic -> page table -> cookie records (48-byte header + C-strings)
3. **Convert** Mac absolute time (2001-01-01 epoch) to Unix timestamps

## Architecture

```
                    +-----------------+
                    |   browserx CLI  |     (clap derive, output formatters)
                    +--------+--------+
                             |
                    +--------v--------+
                    |  browserx-core  |     (providers, crypto, matching)
                    +--------+--------+
                             |
          +------------------+------------------+
          |                  |                  |
  +-------v-------+  +------v------+  +--------v--------+
  |   Chromium     |  |   Firefox   |  |     Safari      |
  |   Provider     |  |   Provider  |  |    Provider      |
  | (7 browsers)   |  | (SQLite)    |  | (binary parser)  |
  +-------+--------+  +------+------+  +--------+--------+
          |                  |                  |
  +-------v--------+        |                  |
  |   Platform     |        |                  |
  |   Crypto       |        |                  |
  | macOS: Keychain|        |                  |
  | Linux: D-Bus   |        |                  |
  | Win: DPAPI     |        |                  |
  +----------------+        |                  |
          |                  |                  |
          +------------------+------------------+
                             |
                    +--------v--------+
                    | browserx-vault  |     (ChaCha20-Poly1305 encrypted storage)
                    +-----------------+
```

## Project Structure

```
crates/
├── browserx-core/                   # Core extraction library
│   └── src/
│       ├── lib.rs                   # Public API: get_cookies(), to_cookie_header(), check_health()
│       ├── types.rs                 # Cookie, SecretValue, BrowserName, GetCookiesResult
│       ├── error.rs                 # Structured error types (BrowserXError)
│       ├── providers/
│       │   ├── mod.rs               # CookieProvider trait, auto-detection, dispatch
│       │   ├── inline.rs            # Inline-first source (JSON, base64, file)
│       │   ├── chromium/
│       │   │   ├── mod.rs           # ChromiumProvider (7 browsers)
│       │   │   ├── paths.rs         # Cross-platform profile path discovery
│       │   │   ├── crypto.rs        # AES-128-CBC / AES-256-GCM decryption, PBKDF2 key derivation
│       │   │   └── sqlite.rs        # SQLite query, WAL snapshot, cookie collection
│       │   ├── firefox.rs           # Firefox SQLite extraction
│       │   └── safari.rs            # Safari Cookies.binarycookies parser
│       ├── platform/
│       │   └── mod.rs               # Platform detection utilities
│       └── util/
│           ├── origin.rs            # URL -> host normalization, parent domain expansion
│           ├── host_match.rs        # RFC 6265 domain matching, SQL WHERE generation
│           └── epoch.rs             # Chromium/Safari/Firefox timestamp unification
├── browserx-cli/                    # CLI binary
│   └── src/
│       ├── main.rs                  # CLI entry point (clap derive), command dispatch
│       ├── commands/
│       │   ├── mod.rs               # Command module exports
│       │   ├── get.rs               # get command: extract + format + output
│       │   ├── browsers.rs          # browsers command: list detected browsers
│       │   ├── health.rs            # health command: session status check
│       │   └── vault.rs             # vault subcommands: store, get, list, clean
│       └── output/
│           └── mod.rs               # Output formatters (JSON, curl, Netscape, ENV, table)
└── browserx-vault/                  # Encrypted cookie storage
    └── src/
        └── lib.rs                   # ChaCha20-Poly1305 vault, TTL management, duration parsing
```

## Security & Threat Model

browserx is designed for **single-user, local-only** use on development machines. The following controls are in place:

| Layer | Control | Detail |
|---|---|---|
| **Cookie values** | `SecretValue` wrapper | `Debug` and `Display` print `***`; raw value only via `.expose()` |
| **Memory** | `zeroize` on drop | Cookie values zeroed when `SecretValue` goes out of scope |
| **SQLite access** | Read-only + temp copy | Opens with `SQLITE_OPEN_READ_ONLY`; copies DB + WAL to temp dir |
| **Vault encryption** | ChaCha20-Poly1305 | Random 12-byte nonce per write; master key at `~/.browserx/vault/master.key` |
| **Vault permissions** | Owner-only | Master key and vault data created with `0600` mode (Unix) |
| **Logging** | No cookie values | `tracing` output never includes raw cookie values |
| **SQL injection** | Host-based WHERE clauses | Inputs are trusted (from URL parsing); parameterized where possible |

### Not recommended for

- **Multi-user / shared machines** -- Other local users with root or same-UID access can read the vault master key. Restrict access to `~/.browserx/` via OS-level permissions or containers.
- **Untrusted inline payloads** -- Inline JSON/base64 inputs are deserialized without schema validation. Only use with payloads you generated or trust.
- **Production services** -- browserx is a development/automation tool. It does not implement TLS, rate limiting, or audit logging.

## Troubleshooting

See [TROUBLESHOOTING.md](TROUBLESHOOTING.md) for common issues and solutions, including:

- Browser not found -- profile path detection per platform
- Keychain access prompts (macOS)
- Encrypted cookies returning empty values
- Linux keyring not available
- Permission errors on vault files

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for release history.

## Acknowledgments

Inspired by [steipete/sweet-cookie](https://github.com/steipete/sweet-cookie). Built as a companion to [justinhuangcode/browsercli](https://github.com/justinhuangcode/browsercli).

## License

[MIT](LICENSE)
