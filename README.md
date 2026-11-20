# wsdp

A high-level asynchronous Rust client for the Czech Real Estate Cadastre's official **ČÚZK WSDP 3.1** services.

The crate uses explicit trial or production environments, SOAP 1.1 with WS-Security `UsernameToken11`, exact decimal values, bounded response decoding, and an asynchronous report lifecycle. Production is never selected implicitly.

> **Billing:** report generation and status checks are free, but the first report download can charge the account. Every download requires explicit per-report charge approval. Trial reports are free and use simulated prices.

## Installation

```sh
cargo add wsdp
```

## Client construction

```rust
use wsdp::{Client, Credentials, Environment};

let client = Client::builder(Environment::Trial)
    .credentials(Credentials::new("username", "password"))
    .build()?;
# Ok::<(), wsdp::Error>(())
```

Construction performs no network request. The client requires a Tokio runtime for network operations.

## Searches and information views

All eight lookup operations and all 33 code-list operations have typed request/result models. Searches expose `Complete`, `Empty`, or `LimitExceeded`; no cursor or pagination is invented. Parcel MBR responses preserve the S-JTSK **Y, X** order and distinguish an envelope from a legitimate GML `Null`.

```rust,no_run
use wsdp::{ParcelId, ParcelSearch, ParcelSearchSelector};

# async fn example(client: &wsdp::Client) -> wsdp::Result<()> {
let result = client.find_parcels(ParcelSearch {
    selector: ParcelSearchSelector::Id(ParcelId::new("2928347306")?),
    valid_at: None,
}).await?;
println!("{} matches", result.value.items.len());
# Ok(())
# }
```

Unit and LV information previews depend on account eligibility; the server can reject otherwise valid requests. See `examples/search.rs` for a complete trial program.

## Report lifecycle and charging

Generation, listing/status, bounded waiting, approved download, and deletion are independent operations. Once generation returns an ID, persist it and resume with that ID after a timeout or restart. Waiting polls only `seznamSestav`; it never retrieves content. Generation calls across client clones are paced at least two seconds apart.

```rust,no_run
use wsdp::{ChargeApproval, Czk, WaitOptions};

# async fn example(client: &wsdp::Client, id: &wsdp::ReportId) -> wsdp::Result<()> {
client.wait_for_report(id, WaitOptions::default()).await?;
let downloaded = client
    .download_report(id, ChargeApproval::up_to(Czk::from_major_units(10)))
    .await?;
let parsed = downloaded.parse_parcel_info()?;
assert_eq!(parsed.original_xml(), downloaded.as_bytes());
# Ok(())
# }
```

`download_report` always performs one free status/price preflight before its single potentially charging `vratSestavu` request. `FreeOnly` accepts only zero, `UpTo` enforces an exact CZK ceiling, and explicit `Any` accepts any supplied non-negative price. A missing price or rejected approval prevents the download request. No generation, download, deletion, authentication, or password-change operation is automatically retried.

After a dispatched download fails ambiguously, `charge_outcome_uncertain()` is true and `report_id()` retains the ID. Reconcile with `report_status`; do not regenerate or blindly repeat the request. According to the official first-open rule, a later explicit retrieval after `zaúčtován` is not charged again. `report_ids()` retains every output when a supposedly single-report request unexpectedly returns a batch. Raw report parsing borrows the download, so original bytes remain available after a decode failure.

Typed helpers combine generation, bounded waiting, price preflight, one approved download, and parsing. They accept only supported pinned XML versions and single outputs. PDF+XML bundles remain on the raw lifecycle: every parent/child ID is retained and every output needs its own approval. See `examples/raw_report.rs`, `typed_report.rs`, `pdf_xml.rs`, and `recover_report.rs`.

## Payloads and account restrictions

- Complete legacy and NTR 3.0 ownership-sheet models are separate; the actual XML root selects the parser.
- Unknown dynamic codes and future protocol values use raw-preserving `Other` variants. Unknown coded and legacy name-only relations both have `is_ownership() == None`.
- Monthly price ZIPs are processed in memory with entry-count, per-entry, total-expansion, compression-ratio, and XML-complexity limits. No filesystem extraction occurs.
- Cadastral-map, collection-document, and geometric-plan helpers require the `%PDF-` signature and return untouched bytes and metadata.
- Free accounts require their file/purpose fields. Verifying accounts require their verification fields and are restricted to eligible PDF outputs. The library validates these combinations but does not infer account type from a username. Account statements use `AccountStatementRequest::with_account` for this local check without serializing any additional authorization field.
- Seal and timestamp flags, and retained XMLDSIG/XAdES content, are metadata only. No cryptographic verification is claimed.

## Retries

Applications may apply their own bounded backoff to idempotent searches, code lists, and status checks. Do not automatically retry generation, downloads, deletion, password changes, or authentication failures. After an ambiguous password change, all clones require explicit confirmed replacement credentials rather than probing old and new passwords.

## Official trial

The [official trial environment](https://cuzk.gov.cz/Katastr-nemovitosti/Poskytovani-udaju-z-KN/Dalkovy-pristup/Webove-sluzby-DP-na-zkousku.aspx) publishes shared credentials and sample identifiers. Reports are free, data are limited to an anonymized Prachatice sample, and displayed prices simulate production behavior. Do not change shared-account passwords or delete other users' reports.

Network examples and ignored trial tests read `WSDP_TRIAL_USERNAME` and `WSDP_TRIAL_PASSWORD`. Trial tests additionally require `WSDP_RUN_TRIAL_TESTS=1` and must be run serially:

```sh
WSDP_RUN_TRIAL_TESTS=1 \
WSDP_TRIAL_USERNAME=WSTEST \
WSDP_TRIAL_PASSWORD=WSHESLO \
cargo test --test trial -- --ignored --test-threads=1
```

Normal tests and CI are entirely offline and credential-free.

## Safety and compatibility

- Redirects and automatic HTTP retries are disabled.
- Credentials, sensitive queries, report identifiers, and report payloads are redacted from debugging output. Server service-message and SOAP-fault text remain available through explicit fields but are omitted from error display/debug output.
- Unknown server-maintained codes retain their raw values.
- IDs are string-backed where their XSD permits values wider than machine integers.
- Every decimal uses `rust_decimal::Decimal`; out-of-range values return an error rather than being rounded.
- Downloaded XML and binary content remains available without cryptographic seal-verification claims.

## License

MIT
