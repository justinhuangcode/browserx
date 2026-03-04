# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2025-03-04

### Added

- Initial release
- Cookie extraction from 9 browsers (Chrome, Edge, Firefox, Safari, Brave, Arc, Vivaldi, Opera, Chromium)
- Cross-platform support (macOS, Linux, Windows)
- Native OS crypto integration (Keychain, DPAPI, D-Bus Secret Service)
- 5 output formats (JSON, curl, Netscape, ENV, table)
- Inline-first design (JSON, base64, file payloads)
- Encrypted cookie vault (ChaCha20-Poly1305) with TTL management
- Session health check
- SecretValue wrapper with zeroize-on-drop
- WAL-safe SQLite reads (temp directory copy)
- Safari binary cookie parser
- Firefox SQLite extraction
- Environment variable configuration
- CI/CD with GitHub Actions (test matrix: Ubuntu, macOS, Windows)
- Multi-platform release binaries (x86_64/ARM64)
- Homebrew formula
- crates.io publishing
