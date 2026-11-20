# wsdp

[![CI](https://github.com/interaso/wsdp/actions/workflows/ci.yml/badge.svg)](https://github.com/interaso/wsdp/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/wsdp.svg)](https://crates.io/crates/wsdp)
[![docs.rs](https://docs.rs/wsdp/badge.svg)](https://docs.rs/wsdp)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](#license)

A high-level asynchronous Rust client for the Czech Real Estate Cadastre's official **ČÚZK WSDP 3.1** services.

The crate covers code lists, object and subject searches, information views, and the complete asynchronous report lifecycle. It provides typed legacy and NTR 3.0 report models, exact decimal handling, bounded XML/ZIP decoding, and explicit approval before any potentially charged download.

> **Generation is free; the first download can charge.** Every report download requires explicit per-output `ChargeApproval`. Trial reports are free and use simulated prices.

The `reqwest`/rustls backend uses `aws-lc-rs`/`aws-lc-sys`, which builds bundled native code and may require a supported C/CMake toolchain.

## Features

- Explicit **trial or production** environment selection; production is never implicit.
- Async, cheaply cloneable `Client` with fixed HTTPS endpoints, TLS verification, and no redirects.
- All **33 code-list operations**, **eight searches**, and three information-view operations.
- Every report-generation operation, including all four ownership-sheet selectors.
- Typed models for all 13 versioned XML report families, monthly price ZIPs, and fixed-format PDF outputs.
- Separate legacy LV 1.0 and NTR LV 3.0 models selected by the actual expanded XML root.
- Raw-preserving open enums, original report bytes, and exact `rust_decimal::Decimal` values.
- Independent generation, status, bounded waiting, approved download, and deletion by report ID.
- Inline base64 and bounded MTOM/XOP downloads with response correlation.
- Bounded XML depth/node counts and ZIP entry, expansion, and compression-ratio limits.
- Redacted credentials, sensitive queries, report identifiers, messages, and payload debugging.
- Optional `serde` support for validated identifiers and privacy-safe `tracing` instrumentation.

## Installation

```sh
cargo add wsdp
```

The crate is pre-1.0, so API changes may occur between minor releases. Rust 1.89 or newer is supported.

## Quick start

The client requires a Tokio runtime. Construction performs no network request.

```rust,no_run
use wsdp::{Client, Credentials, Environment, ParcelId, ParcelSearch, ParcelSearchSelector};

async fn example() -> wsdp::Result<()> {
    let client = Client::builder(Environment::Trial)
        .credentials(Credentials::new(
            "username-from-your-secret-store",
            "password-from-your-secret-store",
        ))
        .build()?;

    let parcels = client
        .find_parcels(ParcelSearch {
            selector: ParcelSearchSelector::Id(ParcelId::new("2928347306")?),
            valid_at: None,
        })
        .await?;
    println!("{} matching parcels", parcels.value.items.len());
    Ok(())
}
```

Searches expose `Complete`, `Empty`, or `LimitExceeded`; no cursor or pagination is invented. Parcel MBR responses preserve the S-JTSK **Y, X** order and distinguish an envelope from a legitimate GML `Null`. Unit and LV information previews depend on account eligibility.

## Report lifecycle and charging

Generation, listing/status, bounded waiting, approved download, and deletion are independent operations. Once generation returns an ID, persist it and resume with that ID after a timeout or restart. Waiting polls only `seznamSestav`; it never retrieves content. Generation calls across client clones are paced at least two seconds apart.

```rust,no_run
use wsdp::{ChargeApproval, Czk, WaitOptions};

async fn download(client: &wsdp::Client, id: &wsdp::ReportId) -> wsdp::Result<()> {
    client.wait_for_report(id, WaitOptions::default()).await?;
    let downloaded = client
        .download_report(id, ChargeApproval::up_to(Czk::from_major_units(10)))
        .await?;
    let parsed = downloaded.parse_parcel_info()?;
    assert_eq!(parsed.original_xml(), downloaded.as_bytes());
    Ok(())
}
```

`download_report` performs one free status/price preflight before its single potentially charging `vratSestavu` request:

- `FreeOnly` accepts only an exact zero price.
- `UpTo` enforces an exact CZK ceiling.
- Explicit `Any` accepts any supplied non-negative price.
- A missing price or rejected approval prevents the download request.

Typed helpers combine generation, bounded waiting, price preflight, one approved download, and parsing. They accept only supported pinned XML versions and single outputs. PDF+XML bundles remain on the raw lifecycle: every parent/child ID is retained and every output needs its own approval. Service code **527** is a definitive per-family configuration rejection, not a reason to retry.

### Recovery and retries

No generation, download, deletion, password-change, or authentication operation is automatically retried.

After a dispatched download fails ambiguously, `charge_outcome_uncertain()` is true and `report_id()` retains the ID. Reconcile with `report_status`; do not regenerate or blindly repeat the request. According to the official first-open rule, a later explicit retrieval after `zaúčtován` is not charged again. `report_ids()` retains every output when a supposedly single-report request unexpectedly returns a batch.

Applications may apply bounded backoff to idempotent searches, code lists, and status checks. After an ambiguous password change, all clones require explicit confirmed replacement credentials rather than probing old and new passwords.

## Supported coverage

| Service | Coverage |
| --- | --- |
| `ciselnik` | All 33 code lists, filtering, update/history dates, and service status |
| `vyhledat` | Parcel, building, unit, subject, building-right, proceeding, deed, and GP searches |
| `informace` | Parcel MBR, unit overview, and LV preview |
| `sestavy` | Every generation operation, lifecycle, XML/ZIP/PDF outputs, versions, and recovery |
| `ucet` | Service status and explicit password change |

### Payloads and account restrictions

- Unknown dynamic codes and future protocol values use raw-preserving `Other` variants. Unknown coded and legacy name-only relations both have `is_ownership() == None`.
- Monthly price ZIPs are processed in memory without filesystem extraction.
- Cadastral-map, collection-document, and geometric-plan helpers require the `%PDF-` signature and return untouched bytes and metadata.
- Free accounts require their file/purpose fields. Verifying accounts require their verification fields and are restricted to eligible PDF outputs.
- Account statements use `AccountStatementRequest::with_account` for local format validation without serializing any additional authorization field.
- Seal and timestamp flags, and retained XMLDSIG/XAdES content, are metadata only. No cryptographic verification is claimed.

## Examples

The [`examples`](examples/) directory contains compile-checked consumer programs. Network examples use `WSDP_TRIAL_USERNAME` and `WSDP_TRIAL_PASSWORD`; supply them through your shell or secret manager.

| Example | Demonstrates |
| --- | --- |
| [status.rs](examples/status.rs) | Trial service health |
| [search.rs](examples/search.rs) | Parcel lookup by internal ID |
| [natural_key.rs](examples/natural_key.rs) | Parcel lookup by cadastral area and number |
| [building_units.rs](examples/building_units.rs) | Building resolution and unit search |
| [subject_overview.rs](examples/subject_overview.rs) | Subject resolution and typed ownership overview |
| [raw_report.rs](examples/raw_report.rs) | Raw generate → wait → approve → download → parse |
| [typed_report.rs](examples/typed_report.rs) | Typed parcel workflow |
| [typed_building.rs](examples/typed_building.rs) | Explicitly pinned legacy building XML |
| [typed_ownership.rs](examples/typed_ownership.rs) | Default NTR 3.0 ownership sheet |
| [pdf_xml.rs](examples/pdf_xml.rs) | Parent/child PDF+XML outputs with separate approvals |
| [recover_report.rs](examples/recover_report.rs) | Recovery by an application-persisted report ID |
| [monthly_prices.rs](examples/monthly_prices.rs) | Bounded monthly price ZIP decoding |

For example, `cargo run --example natural_key` contacts the selected trial endpoint.

## Optional features

Enable validated ID serialization with `serde`:

```sh
cargo add wsdp --features serde
```

Enable privacy-safe operation spans and response metadata with `tracing`:

```sh
cargo add wsdp --features tracing
```

The application configures its own tracing subscriber. The crate never installs a global subscriber and excludes credentials, queries, personal IDs, report IDs, filenames, and payloads from instrumentation.

## Official trial

The [official trial environment](https://cuzk.gov.cz/Katastr-nemovitosti/Poskytovani-udaju-z-KN/Dalkovy-pristup/Webove-sluzby-DP-na-zkousku.aspx) publishes shared credentials and sample identifiers. Reports are free, data are limited to an anonymized Prachatice sample, and displayed prices simulate production behavior. Do not change shared-account passwords or delete other users' reports.

Trial tests are `#[ignore]`d and require all three variables:

| Variable | Purpose |
| --- | --- |
| `WSDP_RUN_TRIAL_TESTS=1` | Explicit opt-in |
| `WSDP_TRIAL_USERNAME` | Trial account username |
| `WSDP_TRIAL_PASSWORD` | Trial account password |

Run them serially:

```sh
WSDP_RUN_TRIAL_TESTS=1 cargo test --test trial -- --ignored --test-threads=1
```

The suite includes the full report matrix with per-output approval and cleanup. Current trial can return a valid cadastral-map PDF, although availability remains configurable. Deed ordering is documented as non-functional and may lack any valid trial document; only the known 387/435 discovery outcomes are accepted for that limitation. Transport and decoding failures remain fatal.

Attached XML is family- and configuration-dependent. Live probes observed service **527** for parcel PDF+XML, service **579** for legacy LV 1.0 PDF+XML, and a PDF-only response to an NTR LV PDF+XML request. Complete two-output PDF+XML bundles were generated, downloaded, and parsed for ownership overview with properties and rights record; rejected or missing attached XML elsewhere is not counted as a verified bundle.

Normal tests and CI are entirely offline and credential-free.

## Production verification

Production verification is a separate maintainer operation requiring distinct opt-in, environment-only production credentials, explicit parcel/building selectors, and charge caps. The included smoke test checks all service statuses and runs parcel-information and building-by-ID XML lifecycles. It never retries a dispatched operation and does not print credentials, report IDs, or report content.

The configured total authorization cannot exceed **300 CZK**, and each report download is further hard-capped at **50 CZK**. If either exact server price exceeds 50 CZK or its remaining total budget, that download request is not dispatched. A separate generation-only parcel PDF+XML check confirms the same service **527** rejection as trial; it never downloads an output or authorizes a report-download charge.

```sh
set -a; . ./.env; set +a
WSDP_RUN_PRODUCTION_TESTS=1 cargo test --test production -- --ignored --test-threads=1
```

The gitignored `.env` must define `WSDP_PRODUCTION_USERNAME`, `WSDP_PRODUCTION_PASSWORD`, `WSDP_PRODUCTION_PARCEL_ID`, `WSDP_PRODUCTION_BUILDING_ID`, and `WSDP_PRODUCTION_MAX_CZK=300`.

## Safety and compatibility

- Redirects and automatic HTTP retries are disabled.
- Credentials, sensitive queries, report identifiers, and report payloads are redacted from debugging output.
- Server service-message and SOAP-fault text remain available through explicit fields but are omitted from error display/debug output.
- IDs are string-backed where their XSD permits values wider than machine integers.
- Every decimal uses `rust_decimal::Decimal`; out-of-range values return an error rather than being rounded.
- Downloaded XML and binary content remain available without cryptographic seal-verification claims.

## Development checks

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked
cargo test --all-features --locked
cargo check --examples --locked
RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps --locked
cargo package --locked --allow-dirty
```

Schema source URLs and SHA-256 hashes are recorded in [SCHEMAS.md](SCHEMAS.md). See [CONTRIBUTING.md](CONTRIBUTING.md) for development and live-test safety, and [CHANGELOG.md](CHANGELOG.md) for release history.

## Official references

- [WSDP 3.1 documentation](https://cuzk.gov.cz/Katastr-nemovitosti/Poskytovani-udaju-z-KN/Dalkovy-pristup/Webove-sluzby-dalkoveho-pristupu.aspx)
- [Official trial and sample objects](https://cuzk.gov.cz/Katastr-nemovitosti/Poskytovani-udaju-z-KN/Dalkovy-pristup/Webove-sluzby-DP-na-zkousku.aspx)
- [Output descriptions](https://cuzk.gov.cz/Katastr-nemovitosti/Poskytovani-udaju-z-KN/Dalkovy-pristup/Vystupy-z-KN-poskytovane-prostrednictvim-DP.aspx)
- [Charging rules](https://cuzk.gov.cz/Katastr-nemovitosti/Poskytovani-udaju-z-KN/Dalkovy-pristup/Uctovani-vystupu-z-KN-poskytovanych-DP-A-WSDP.aspx)
- [Security and password rules](https://cuzk.gov.cz/Katastr-nemovitosti/Poskytovani-udaju-z-KN/Dalkovy-pristup/Bezpecnostni-opatreni-a-pravidla-pro-uzivani-prist.aspx)

## License

Licensed under the MIT License ([LICENSE](./LICENSE) or <https://opensource.org/licenses/MIT>).

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this crate is licensed under the MIT License, without additional terms or conditions.
