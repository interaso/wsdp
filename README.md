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
- Credentials and report payloads are redacted from debugging and errors.
- Unknown server-maintained codes retain their raw values.
- IDs are string-backed where their XSD permits values wider than machine integers.
- Every decimal uses `rust_decimal::Decimal`; out-of-range values return an error rather than being rounded.
- Downloaded XML and binary content remains available without cryptographic seal-verification claims.

## License

MIT
