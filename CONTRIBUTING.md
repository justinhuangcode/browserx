# Contributing to BrowserX

Thank you for your interest in contributing to BrowserX!

## Development Setup

```bash
git clone https://github.com/justinhuangcode/browserx
cd browserx
cargo build
cargo test
```

## Code Style

- Run `cargo fmt` before committing
- Run `cargo clippy` and fix all warnings
- Add tests for new functionality

## Pull Request Process

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/my-feature`)
3. Commit your changes
4. Push to the branch
5. Open a Pull Request

## Adding a New Browser

To add support for a new Chromium-based browser:

1. Add a variant to `BrowserName` in `crates/browserx-core/src/types.rs`
2. Add profile paths in `crates/browserx-core/src/providers/chromium/paths.rs`
3. Add detection logic in `crates/browserx-core/src/providers/mod.rs`
4. Add tests

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
