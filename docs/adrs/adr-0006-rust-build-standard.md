# Architectural decision record (ADR) 0006: Rust build standard

## Status

Accepted. Adopts Concordat's `rust-build-defaults` and
`rust-dev-build-footprint` rules.

## Date

2026-10-31.

## Context and problem statement

Development, test, lint and typecheck builds ran with no parallel `rustc`
frontend, the platform linker and the LLVM backend, and a dev build kept
incremental state and debug information that nobody read. Cargo selects one
`rustflags` source rather than merging them, so a flag set in
`.cargo/config.toml` is lost as soon as a recipe or a workflow assigns
`RUSTFLAGS`, and CI's `setup-rust` exports one.

## Decision drivers

- Faster development builds without changing what ships or what coverage
  measures.
- A flag lost through a recipe or workflow edit must fail a test.
- Whitaker runs under the toolchain its installer manages, not the pin.

## Decision outcome

- `.cargo/config.toml` sets `-Zthreads=8` in `[build]` and repeats it with
  `-Clink-arg=-fuse-ld=mold` in a Linux-only table, because `mold` ships for
  Linux only and Cargo does not merge the sources.
- The `Netsukefile` composes the same flags onto any inherited `RUSTFLAGS` in
  the `build`, `test`, `lint` (rustdoc and Clippy) and `typecheck` recipes.
  `release` assigns the inherited value alone, so a shipped artefact keeps the
  platform linker and the single-threaded frontend.
- Cranelift is the development-profile backend (`[profile.dev] codegen-backend`
  with `[unstable] codegen-backend`), because the whole suite passes under it;
  the measurement is in the developers' guide. Coverage selects LLVM, which
  instrumentation needs, and assigns `RUSTFLAGS: -D warnings` so it takes
  neither flag.
- The dev profile sets `incremental = false` and `debug = 0`, and
  `[unstable] embed-metadata = false` is the native Cargo key for the dated
  nightly (the `rust-dev-build-footprint` rule).
- Whitaker is isolated: the `lint` recipe starts it from the checkout's parent
  with an absolute `--manifest-path` and the literal `RUSTFLAGS="-D warnings"`
  (clearing an inherited `CARGO_ENCODED_RUSTFLAGS` first, which Cargo ranks
  above `RUSTFLAGS`), so it takes none of the standard and Cargo never
  discovers this checkout's `.cargo/config.toml` and its Cranelift profile,
  which the installer-managed toolchain does not carry. CI installs Whitaker
  with `cranelift: 'true'` for the same reason.

## Consequences

- Linux builds require `mold`, reached through `cc` (GCC 12.1 or newer, or
  clang). CI's `setup-rust` steps install it.
- Whitaker links with the platform linker, and a caller's `RUSTFLAGS` no longer
  reaches it.
- `tests/build_standard_contract.rs` holds the configuration, the pin and the
  workflows; `tests/netsuke_actions.rs` drives the real `netsuke` against the
  manifest and proves the composed flags, the release exclusion and Whitaker's
  working directory, manifest path and literal flags;
  `tests/build_footprint_contract.rs` proves the footprint at the Cargo
  boundary.
- Revisit Cranelift if a later bump fails the suite under it: record the failing
  tests in the guide and remove the backend.
