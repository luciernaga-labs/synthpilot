# Contributing to SynthPilot

Thanks for your interest in SynthPilot. This guide explains how to propose changes.

## Ground rules

- Be respectful. This project follows the [Code of Conduct](CODE_OF_CONDUCT.md).
- Open an issue before starting large changes so the approach can be discussed.
- Keep pull requests focused on one topic.

## Development setup

You need a recent stable Rust toolchain. The minimum supported version is listed as `rust-version` in [`Cargo.toml`](Cargo.toml).

```sh
cargo run              # start the desktop app
cargo test --workspace # run all tests
```

Before opening a pull request, make sure these pass:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo coverage
```

On Linux, install the ALSA development package first (`libasound2-dev` on Debian or Ubuntu).

## Continuous integration

Every pull request runs:

- **Format** and **Clippy** with warnings treated as errors.
- **Unit tests** on Linux, macOS and Windows.
- **Coverage**: `cargo coverage` fails when line coverage drops below 90%. It needs [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov) (`cargo install cargo-llvm-cov` and `rustup component add llvm-tools-preview`). Rendering code (`crates/synthpilot/src/main.rs` and the egui views in `crates/synthpilot/src/app/`) and the hardware adapter (`crates/synthpilot-midi/src/device.rs`) are excluded, so keep logic out of those files and in modules that can be tested.
- **MSRV**: the workspace must build with the Rust version in `rust-version`.
- **Autofix**: for branches in this repository, `cargo clippy --fix` and `cargo fmt` run automatically and any changes are pushed back to the pull request as a new commit. Pull requests from forks are checked but not modified.

Commits pushed with the default `GITHUB_TOKEN` do not start new workflow runs. Maintainers can add a repository secret named `AUTOFIX_TOKEN` (a fine-grained token with *Contents: read and write*) so that CI runs again on the autofix commit.

## Protocol mappings

Protocol data is the heart of SynthPilot and must be trustworthy.

- Every implemented parameter must come from the manufacturer's official documentation. Cite the document and page in the mapping and in the device page under [`docs/`](docs/).
- Never guess or extrapolate an address, data size or value range. If something cannot be verified, add the parameter with `Entry::unverified` and explain why.
- Add golden byte tests for every new mapping, covering at least the minimum, the maximum and the default value.
- Keep manufacturer-specific code inside its device module. The domain model and the application must stay device-agnostic.

## Commit messages

Use short, imperative summaries, for example `Add FM-X operator feedback mapping`. Reference related issues in the body.

## License

By contributing, you agree that your contributions are licensed under the [MIT License](LICENSE).
