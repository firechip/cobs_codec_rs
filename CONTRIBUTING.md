# Contributing to cobs_codec_rs

Thanks for your interest in improving `cobs_codec_rs`!

## Getting started

A stable Rust toolchain (>= 1.81, the crate's MSRV) with `rustfmt` and `clippy`:

```console
git clone https://github.com/firechip/cobs_codec_rs.git
cd cobs_codec_rs
cargo test
```

## Development workflow

Before opening a pull request, make sure all of the following pass — this is the
bar CI enforces:

```console
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo clippy --no-default-features -- -D warnings   # no_std
cargo test --all-features
cargo build --no-default-features                    # no_std builds
```

The crate is `#![no_std]` and dependency-free; keep it that way. Anything that
allocates goes behind the `alloc` feature.

## Correctness bar

COBS and COBS/R are exact, well-specified algorithms:

- Changes to `src/cobs.rs` or `src/cobsr.rs` must keep the golden vectors in
  `tests/vectors.rs` passing. Those vectors come from the reference
  implementations and must **not** be changed to make new code pass.
- The crate is validated against the shared
  [firechip/cobs-conformance](https://github.com/firechip/cobs-conformance)
  vectors and is byte-identical to the Dart and Kotlin implementations.
- New behaviour needs new tests.

## Git workflow: Trunk-Based Development with tbdflow

This project uses [Trunk-Based Development](https://trunkbaseddevelopment.com/):
small, frequent changes integrated into `main` (the trunk) rather than
long-lived branches. We use the [`tbdflow`](https://github.com/cladam/tbdflow)
CLI (`cargo install tbdflow`) so the safe path is the easy path.

`tbdflow commit` pulls `main`, creates a Conventional Commit, and pushes:

```console
tbdflow commit --type fix --scope decode -m "reject truncated final block"
```

For a change that needs review, use a short-lived branch and merge it back
quickly: `tbdflow branch --type feat --name my-change`, then `tbdflow complete`.

Two committed files drive this workflow: **`.tbdflow.yml`** (workflow +
Conventional Commit lint rules) and **`.dod.yml`** (the Definition of Done
checklist shown before each commit). Bypass the checklist for a trivial change
with `--no-verify`.

## Conventional Commits

Every commit message follows
[Conventional Commits](https://www.conventionalcommits.org):
`type(scope): short imperative subject`. Allowed **types**: `build`, `chore`,
`ci`, `docs`, `feat`, `fix`, `perf`, `refactor`, `revert`, `style`, `test`. The
subject is lowercase, imperative, and has no trailing period; breaking changes
use `!` (`feat!:`) or a `BREAKING CHANGE:` footer.

This is enforced locally by `tbdflow commit` and in CI by the **Commit lint**
workflow.

## License

By contributing, you agree that your contributions are licensed under the
project's [MIT License](LICENSE).
