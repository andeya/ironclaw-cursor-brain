# Contributing

Thanks for your interest in contributing to ironclaw-cursor-brain.

## Prerequisites

- [Rust](https://rustup.rs) (stable)
- [Cursor](https://cursor.com) or `cursor-agent` on PATH (for running the service)

## Build and run

```bash
cargo build --release
./target/release/ironclaw-cursor-brain   # or: cargo run
```

On Windows: `.\target\release\ironclaw-cursor-brain.exe` or `cargo run`.

## Code quality

- Format: `cargo fmt`
- Lint: `cargo clippy` (fix all warnings before submitting)

## Submitting changes

1. Open an issue to discuss larger changes, or pick an existing issue.
2. Fork the repo, create a branch, make your changes.
3. Run `cargo fmt` and `cargo clippy`; ensure tests pass if present.
4. Open a pull request with a clear description and reference to any issue.

## License

By contributing, you agree that your contributions will be licensed under the same [LICENSE](LICENSE) (MIT) as the project.
