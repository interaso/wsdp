# Contributing

Thank you for contributing to `wsdp`. Keep changes focused, document public APIs, and add tests for behavior changes.

## Development setup

Install a current stable Rust toolchain with `rustfmt` and Clippy. The crate uses Rust 2024 and forbids unsafe code. The declared MSRV must be checked after dependency updates.

Run the same offline checks as CI before submitting a change:

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked
cargo test --all-features --locked
cargo check --examples --locked
RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps --locked
cargo package --locked --allow-dirty
```

Check `serde` and `tracing` independently. CI also checks the declared MSRV and an all-feature `x86_64-unknown-linux-musl` build when supported by the resolved dependencies. Normal tests never contact ČÚZK or require credentials.

## Public API and protocol changes

- Prefer descriptive English API names and established ecosystem crates over protocol reimplementation.
- Preserve explicit environment selection and never make production the default.
- Do not add automatic retries to generation, download, deletion, password-change, or authentication operations. Their outcomes can be ambiguous after dispatch.
- Keep generation, status/waiting, approved download, and deletion independently callable. Never retrieve a report during polling.
- Keep credentials, queries, personal identifiers, report IDs, and downloaded content out of logs and debugging output.
- Preserve unknown codes and original report bytes. Do not classify legal relations from Czech names.
- Use `rust_decimal::Decimal` for every decimal; do not add another decimal representation or binary-float money constructors.
- Update README examples, crate documentation, and `CHANGELOG.md` when changing consumer-visible behavior.
- Record official source URLs, retrieval dates, versions, and hashes in `SCHEMAS.md` when updating protocol or registry inputs. Never fetch them during builds.

## Trial tests

Live tests are optional maintainer checks, never part of CI. They may use only the official WSDP trial environment and require `WSDP_RUN_TRIAL_TESTS=1`, `WSDP_TRIAL_USERNAME`, and `WSDP_TRIAL_PASSWORD`. Run them with `cargo test --test trial -- --ignored --test-threads=1`.

The official trial page publishes shared account credentials and sample objects. These accounts are shared: never test password changes or authentication lockout, never assume an empty report list, and delete only reports created by the current test. Trial reports are free but use simulated prices, so explicit charge approval is still required.

Production verification is a separate, explicitly requested maintainer step. It requires distinct opt-in, production credentials from a secret manager, explicit object selectors, serial execution, and a small total charge cap. Do not print owner or report contents. The included parcel/building smoke test hard-caps the configured total at 300 CZK and each report download at 50 CZK. It never retries a dispatched operation:

```sh
set -a; . ./.env; set +a
WSDP_RUN_PRODUCTION_TESTS=1 cargo test --test production -- --ignored --test-threads=1
```

The gitignored `.env` supplies `WSDP_PRODUCTION_USERNAME`, `WSDP_PRODUCTION_PASSWORD`, `WSDP_PRODUCTION_PARCEL_ID`, `WSDP_PRODUCTION_BUILDING_ID`, and `WSDP_PRODUCTION_MAX_CZK=300`. Never commit this file.

## Changelog and releases

Record consumer-visible changes under `Unreleased` in `CHANGELOG.md`, using categories such as Added, Changed, Fixed, and Removed. Do not assign a version or date until a release is explicitly prepared.

Releases are maintainer-only and use the published-release workflow:

1. Start from a reviewed `main` branch with passing CI and the required manual trial lifecycle.
2. Update `Cargo.toml`, dependency examples, and the dated changelog section/links.
3. Review and commit release changes, then prepare a GitHub release with a `vX.Y.Z` tag exactly matching the package version.
4. Publishing the GitHub release triggers offline verification, package dry-run, and crates.io publication. The workflow never creates a GitHub release or runs live tests.
5. Verify the crates.io page, docs.rs build, release notes, and documentation links.

The GitHub repository requires a suitably scoped `CARGO_REGISTRY_TOKEN` Actions secret. Never place release credentials in workflow files or committed content.

## Licensing

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this crate is licensed under the MIT License, without additional terms or conditions.
