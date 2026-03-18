# Contributing to asterisk-rs

## Building

```sh
cargo build --workspace
```

## Testing

```sh
cargo test --workspace
```

## Linting

```sh
cargo clippy --workspace --all-targets -- -D warnings
```

## Formatting

```sh
cargo fmt --all
```

## Pull Requests

- One logical change per PR.
- Include tests for new behavior.
- All CI checks must pass before merge.
- Use [conventional commits](https://www.conventionalcommits.org/) for commit messages:
  `feat:`, `fix:`, `docs:`, `chore:`, `refactor:`, `test:`, `ci:`.

## Code of Conduct

This project follows the [Contributor Covenant](CODE_OF_CONDUCT.md). By
participating, you agree to uphold it.
