# `wsdp`: async Rust client for ČÚZK WSDP 3.1

## 1. Goal and decisions

Build a crate named **`wsdp`**: a high-level, asynchronous Rust client for the
official ČÚZK **Webové služby dálkového přístupu (WSDP)** to the Czech Real
Estate Cadastre.

The crate should make the following workflows safe and ergonomic:

- resolve parcels, buildings, units, building rights, subjects, proceedings,
  deeds, and geometric plans from their natural or internal identifiers;
- retrieve code lists, parcel bounding boxes, and eligible lightweight
  information views;
- create, monitor, download, and delete WSDP reports;
- decode every supported XML report into complete, idiomatic Rust models while
  retaining the original XML;
- handle PDF and ZIP outputs without pretending that they are XML;
- make the billable part of the report lifecycle explicit and recoverable.

Decisions confirmed for the initial release:

| Decision | Choice |
| --- | --- |
| Crate name | `wsdp` |
| Protocol | **WSDP 3.1 only**, not the obsolete 2.9 interface used by the Kotlin reference |
| Initial functional scope | **Broad WSDP coverage**, including ownership, proceedings, prices, maps, and document outputs |
| Public report models | Complete models for supported XML reports, not a small lossy owner/parcel subset |
| Paid downloads | Explicit per-call charge approval, optionally bounded by a price ceiling |
| Architectural reference | Reuse the proven conventions of `../isds`, not the Kotlin implementation's design |
| Runtime | Async Rust with Tokio-based timing, `reqwest`, and rustls |
| Compatibility | Preserve unknown codes, statuses, and raw payloads; do not turn future server additions into unnecessary failures |

All phases in this plan are required for the intended broad first release; the
phases are implementation milestones, not a proposal to release a parcel-only
crate first.

## 2. Official protocol basis

Research was performed on **2026-11-20** against the official sources listed in
[section 17](#17-official-sources).

### 2.1 Environments and services

Use explicit environments with no implicit production default:

| Environment | WSDP 3.1 base URL |
| --- | --- |
| `Environment::Trial` | `https://wsdptrial.cuzk.gov.cz/trial/ws/wsdp/3.1` |
| `Environment::Production` | `https://katastr.cuzk.gov.cz/ws/wsdp/3.1` |

Implement the five core WSDP service families:

| Service | Path | Purpose |
| --- | --- | --- |
| Code lists | `/ciselnik` | Territorial, property, subject, legal-relation, purpose, and message code lists |
| Search | `/vyhledat` | Resolve subjects and cadastral objects to typed records and internal IDs |
| Information | `/informace` | Parcel MBRs and eligible current unit/LV overview responses |
| Reports | `/sestavy` | Asynchronous generation, status, download, versions, account statement, and deletion |
| Account | `/ucet` | Service status and explicit password change |

The protocol is SOAP 1.1 document/literal over HTTPS, with WS-Security
`UsernameToken11`. Report downloads can use MTOM/XOP, while the schema and
official examples also permit inline base64 content.

WSDP 2.9 was documented for shutdown in autumn 2026. Do not spend effort on a
multi-version SOAP abstraction. Keep protocol constants isolated so a future
version can be added deliberately.

### 2.2 Important service behavior

These behaviors must be treated as part of the public API design:

- Search, code lists, account operations, report generation, and report status
  operations are free. **The first `vratSestavu` retrieval charges the report if
  it has not already been charged.** Generation is therefore not consent to
  download.
- Report generation is asynchronous. Poll `seznamSestav`, not `vratSestavu`.
  Ordinary reports take seconds to tens of seconds.
- ČÚZK requests at least **two seconds between successive report-generation
  calls** in batch processing.
- ČÚZK can reject excess concurrency with message **477**; message **210** also
  describes a user limit. No numeric account QPS/concurrency allowance or
  general `Retry-After` contract is published.
- Three consecutive authentication failures can lock an account for 15 minutes.
  Never automatically retry authentication failures.
- Search/code-list results have no cursor or pagination fields. Their output is
  capped at 20,000 records. Message **310** means incomplete/limit-exceeded
  output; **392** means no matching data. Do not invent pagination.
- Current and historical requests differ. Omitted `datumK` means current server
  data. For report generation, a supplied value must be at least 20 minutes in
  the past and later than 2001-09-04. Do not automatically substitute `now()`.
- XML report versions are negotiable through `dejVerzeSestav`. ČÚZK recommends
  explicitly specifying `verze` for XML; if omitted, the service selects the
  highest available version.
- XML is UTF-8 and can contain presentation whitespace. It can be returned with
  informational message **371** when ČÚZK considers it XSD-invalid; preserve the
  payload and warning rather than silently discarding a potentially paid output.
- Selected PDF/XML outputs have electronic-seal/timestamp metadata. Legacy XML
  uses XMLDSIG and NTR 3.0 uses XAdES. A flag saying that a seal exists is not
  cryptographic verification.
- Trial reports are free and use simulated prices. Trial data are limited and
  anonymized. Cadastral-map generation and deed ordering are unavailable or
  limited in trial.

### 2.3 Account contexts

WSDP behavior depends on the account type:

- **Paying** accounts need no additional report authorization fields.
- **Free-of-charge** accounts require `cisloJednaci` and `ucelKod` for report
  generation, except the account statement.
- **Verifying** accounts require `poradoveCislo` and an explicit verification
  clause choice, are restricted to eligible reports, and receive PDF rather than
  XML outputs.

Represent these as typed per-report authorization options. Do not infer the
account type from a username, and do not cache it from a health response. Local
validation should catch malformed combinations, while server authorization
errors remain possible and must be preserved.

## 3. Scope

### 3.1 Initial release

Include:

1. All five core services and `stavWS` health/status calls.
2. All 33 code-list operations plus `stavWS`, using typed request/result families
   and convenient methods for common territorial lookups.
3. All eight lookup operations plus `stavWS`, including subject, parcel, building,
   unit, building-right, proceeding, deed, and geometric-plan lookup.
4. Parcel MBRs, current unit information, and current LV preview, with account
   eligibility documented rather than silently assumed.
5. Full asynchronous report lifecycle and runtime report-version discovery.
6. Full typed models for all 13 versioned XML report families, including both
   legacy and NTR 3.0 ownership-sheet formats.
7. Typed monthly price data from the price-by-cadastral-area ZIP payload.
8. Binary cadastral-map, deed-collection, and geometric-plan PDF outputs.
9. Explicit password change, without automatic retries or live tests against
   shared trial accounts.
10. Offline contract tests, opt-in official-trial lifecycle tests, examples,
    consumer documentation, schema provenance, and CI checks.

### 3.2 Deliberate non-goals

Do not include these in the initial crate:

- WSDP 2.9 compatibility.
- Separate ISKN interfaces for submissions (`podani`), geo services, WSNO, or
  the separately documented `ctiOS` service.
- OGC WFS/WMS, RÚIAN, Nahlížení do KN web scraping, or undocumented web
  endpoints.
- A general SOAP framework or public generated WSDL bindings.
- Cryptographic verification of XMLDSIG, XAdES, PDF seals, or timestamps. Keep
  original bytes and expose metadata so verification can be added later without
  changing the download API.
- Built-in persistent caching, scheduling, batch persistence, price tables, or
  automatic report deletion. These are application policies.
- Lossy normalization of countries, owner kinds, legal-relation labels, missing
  ownership fractions, or addresses. Preserve official values and provide only
  semantics-preserving convenience accessors.

## 4. Design principles

1. **High-level domain API over a private protocol layer.** Consumers use
   `ParcelSelector`, `OwnershipSheetRequest`, and `ChargeApproval`, not SOAP
   envelopes or Czech element names.
2. **Explicit side effects.** Generation, charging download, deletion, and
   password change are distinct operations. Documentation states request counts,
   costs, and failure ambiguity.
3. **No hidden billable retry.** Never automatically repeat generation,
   download, deletion, password change, or authentication failures.
4. **Recoverable asynchronous workflows.** Once a report ID exists, errors and
   timeouts retain it so callers can resume instead of regenerating.
5. **Complete but idiomatic models.** Model every field in the selected report
   XSDs, use enums for XSD choices, and retain raw XML for forward compatibility.
   Do not expose a mechanically generated schema object graph.
6. **Exact values.** No binary floating point for money, areas, coordinates, or
   shares. No `u64` for identifiers that can contain 30 digits.
7. **Forward-compatible decoding.** Unknown service codes and dynamic code-list
   values survive as raw values. Unknown enum variants use `Other(String)` or an
   equivalent raw fallback.
8. **Secure by default.** Fixed HTTPS endpoints, no redirects, secret redaction,
   bounded bodies and XML complexity, no payload logging, and no ambient global
   tracer or runtime.
9. **Offline correctness.** Normal tests never require credentials or an
   external service. Live tests are separate and explicitly opted in.

## 5. Public API proposal

The signatures below illustrate the intended shape; names can be refined during
implementation without changing the safety properties.

### 5.1 Client construction

```rust
use wsdp::{Client, Credentials, Environment};

let client = Client::builder(Environment::Trial)
    .credentials(Credentials::new(username, password))
    .connect_timeout(Duration::from_secs(10))
    .request_timeout(Duration::from_secs(60))
    .max_response_bytes(256 * 1024 * 1024)
    .build()?;
```

`Client` is an `Arc`-backed, cheaply cloneable value. Clones share the HTTP
connection pool, credentials, concurrency limiter, and report-generation pacing.
Construction performs no network request.

Builder requirements and behavior:

- `Environment` and credentials are mandatory.
- All timeouts, body limits, XML limits, and concurrency values must be nonzero.
- Initial defaults: 10-second connect timeout, 60-second per-request timeout,
  256 MiB buffered HTTP response limit, and one in-flight request per client.
  The concurrency default is deliberately conservative because ČÚZK publishes
  no numeric limit; callers can explicitly raise it.
- Redirects and reqwest-level automatic retries are disabled.
- Production endpoints cannot be overridden. A mock endpoint/transport seam is
  available through `#[cfg(test)]` and a feature-gated integration-test
  constructor that never forwards credentials to an arbitrary HTTPS host.
- `Debug` for the client and credentials redacts secrets.

Credentials live behind a small synchronized holder because password change has
to affect every clone. A confirmed `zmenHeslo` success atomically replaces the
shared password with the new secret. A definitive pre-dispatch or server-side
rejection leaves the existing credentials usable. Any post-dispatch outcome
uncertainty moves all clones into an explicit `CredentialsRequired` state:
subsequent operations fail locally until the caller supplies confirmed working
credentials through `replace_credentials`. The client must never probe old and
new passwords automatically, because repeated failures can lock the account.

### 5.2 Common response and message types

```rust
pub struct Response<T> {
    pub value: T,
    pub messages: Vec<ServiceMessage>,
}

pub struct ServiceMessage {
    pub code: MessageCode,
    pub level: MessageLevel,
    pub text: String,
}

#[non_exhaustive]
pub enum MessageCode {
    Success,                 // 0
    PasswordExpired,         // 205
    RecordNotFound,          // 304
    ResultLimitExceeded,     // 310
    NoData,                  // 392
    ConcurrentUsersExceeded, // 477
    Other(String),
}

#[non_exhaustive]
pub enum MessageLevel {
    Information,
    Warning,
    Error,
    Other(String),
}
```

All normal operations retain informational, warning, operational, and unknown
messages, including codes above 9000. `MessageCode` provides the known numeric
value or original lexical value through its accessors; `Other(String)` is used
because the XSD `nonNegativeInteger` has no upper bound. Any `CHYBA` message
produces a structured service error containing the complete message list.

Searches return a completion state in addition to items:

```rust
pub struct SearchResult<T> {
    pub items: Vec<T>,
    pub completion: SearchCompletion,
}

pub enum SearchCompletion {
    Complete,
    Empty,
    LimitExceeded, // Items, if any, must not be treated as a complete page.
}
```

### 5.3 Search and information operations

Use one request type per operation and enums for XSD `choice` structures:

```rust
pub enum ParcelSelector {
    Id(ParcelId),
    Number(ParcelNumber),
}

pub struct ParcelSearch {
    pub selector: ParcelSelector,
    pub valid_at: Option<XmlDateTime>,
}

let result = client.find_parcels(&ParcelSearch::by_number(
    CadastralAreaCode::new("693936")?,
    ParcelNumber::new(77, None)?,
    None,
)).await?;
```

Planned direct methods:

```text
service_status(service)
code_list(request)

find_subjects(query)
find_parcels(query)
find_buildings(query)
find_units(query)
find_building_rights(query)
find_proceedings(query)
find_collection_documents(query)
find_geometric_plans(query)

parcel_bounding_boxes(ids)
unit_overview(query)                 // dejInfoOJednotce; account eligibility applies
ownership_sheet_preview(query)      // dejNahledLV; current data, limited content
report_versions(filter)

change_password(new_password)        // explicit, never retried
```

API rules:

- Natural-key parcel/building searches always return collections, even if most
  calls return one item. ID-based variants can expose the same collection API
  while documenting their at-most-one server behavior.
- Omitted parcel subdivision means an unsubdivided parcel, not any subdivision.
- Unit lookup can select a building by ID or natural key and can omit the unit
  number to return all units in the selected building(s).
- Subject lookup distinguishes legal-person and natural-person/partnership
  selectors. Validate structural rules, but do not pretend to know every
  account-dependent combination rejected by the server.
- `OwnershipTarget` is an enum: subject, parcel, building, unit, or building
  right. Do not assume every ownership record points to a subject.
- MBR results distinguish `Envelope` from legitimate `Null` geometry and expose
  the documented S-JTSK **(Y, X)** coordinate order explicitly.
- Code-list filters preserve `%` exactly as supplied. Do not silently add
  wildcards or perform client-side case/diacritic conversion.
- `CodeListRequest`/`CodeListResult` cover every operation with typed variants;
  convenience wrappers cover common calls such as cadastral areas,
  municipalities, municipality parts, workplaces, legal-relation types, and
  service messages.

### 5.4 Report request types

Use a semantic `GenerateReport` enum for the raw lifecycle API. Each variant has
a dedicated request type containing its selector, historical timestamp where
supported, output options, and account authorization.

```rust
pub enum GenerateReport {
    OwnershipSheet(OwnershipSheetRequest),
    CadastralMap(CadastralMapRequest),
    ParcelInfo(ParcelInfoRequest),
    BuildingInfo(BuildingInfoRequest),
    UnitInfo(UnitInfoRequest),
    BuildingRightInfo(BuildingRightInfoRequest),
    ProceedingInfo(ProceedingInfoRequest),
    DefinitionPoints(DefinitionPointsRequest),
    OwnershipOverview(OwnershipOverviewRequest),
    OwnershipOverviewWithProperties(OwnershipOverviewRequest),
    RightsRecord(RightsRecordRequest),
    AccountStatement(AccountStatementRequest),
    PricesByProperties(PricesByPropertiesRequest),
    PricesByProceeding(PricesByProceedingRequest),
    PricesByCadastralArea(PricesByCadastralAreaRequest),
    CollectionDocument(CollectionDocumentRequest),
    GeometricPlan(GeometricPlanRequest),
}
```

`ReportOptions` covers:

- required output format where the operation offers a choice;
- XML version selection, with a family-specific pinned default, an explicitly
  pinned override, or an explicitly requested server-latest raw output;
- optional attached XML for eligible PDF outputs;
- `AccountContext::{Paying, Free, Verifying}` with the appropriate typed fields.

Use named versions and constructors for normal calls while retaining validation
for runtime-discovered values:

```rust
pub enum ReportVersionSelection {
    DefaultForFamily,
    Pinned(ReportVersion),
    LatestAvailable, // Omits `verze`; raw-output workflow only.
}

// Named constants such as ReportVersion::V1_0 and ReportVersion::V3_0.
let default_options = ReportOptions::xml_default(AccountContext::Paying);
let pinned_options = ReportOptions::xml(ReportVersion::V1_0, AccountContext::Paying);
let latest_raw_options = ReportOptions::xml_latest(AccountContext::Paying);
```

`ReportVersion::new("1.0")?`, `FromStr`, and a validated raw fallback remain
available for values returned by `dejVerzeSestav` or introduced in the future.
The string constructor is therefore useful, but should not be necessary in
ordinary examples. There is no global `Default for ReportVersion`: version
meaning and parser support depend on the report family.

Each typed report family exposes `DEFAULT_VERSION`. `xml_default` resolves and
**pins** that version on the wire before generation; it does not silently select
whatever version happens to be newest on the server. Initially use `V1_0` for
legacy XML families and NTR 3.0/`V3_0` for ownership sheets. Trial verification on
2026-11-20 confirmed that both LV 1.0 and NTR LV 3.0 are enabled. Changing a family
default is consumer-visible behavior and
must be documented in the changelog. Typed helpers reject `LatestAvailable`;
raw callers opting into it receive metadata and untouched bytes for independent
or later parsing.

Constructors should prevent invalid combinations such as XML for a verifying
account, PDF for definition points, a format field for map/deed/GP operations,
or attached XML for an operation that does not support it. Do not silently send
fields that the server would ignore.

Ownership-sheet requests support both formats explicitly:

```rust
pub enum OwnershipSheetVersion {
    Legacy,
    Ntr3,
}

pub enum OwnershipSheetReport {
    Legacy(LegacyOwnershipSheet),
    Ntr3(NtrOwnershipSheet),
}
```

The actual parser also checks the root expanded name and namespace; the request
variant is not trusted as proof of the returned format.

### 5.5 Raw report lifecycle

```rust
let batch = client.generate_report(request).await?;
let handle = batch.into_single_report()?; // Rejects unexpected multi-report output.

let ready = client
    .wait_for_report(handle.id(), WaitOptions::default())
    .await?;

// This method can charge the account. It refreshes the price before download.
let document = client
    .download_report(
        ready.id(),
        ChargeApproval::up_to(Czk::from_major_units(10)),
    )
    .await?;

let typed = document.parse_parcel_info()?;
```

Lifecycle methods:

```text
generate_report(request)              // Free; queues output; no automatic retry
list_reports(filter)                  // Free; no implicit pagination
report_status(id)                     // Free
wait_for_report(id, wait_options)     // Free, bounded status polling only
download_report(id, charge_approval)  // Potentially paid; explicit approval
delete_report(id)                     // Explicit destructive operation
report_versions(filter)               // Discover enabled XML report versions
```

Every stage is independently callable. `generate_report` returns as soon as the
service accepts the request; it neither waits nor starts background polling. A
caller can persist the validated `ReportId`, call `report_status` occasionally
from another task or process, and use `wait_for_report` only when bounded
blocking-style waiting is convenient. After a restart, reconstructing a client
for the same account and loading the ID is sufficient to resume. No call to
`wait_for_report` is required before a later status check, and no lifecycle stage
other than an explicit approved `download_report` can retrieve/charge the report.

`ReportBatch` retains every returned report and all parent/child report IDs.
PDF+XML generation can produce related reports; never call `.single()` blindly
or discard the associated XML report. A convenience `into_single_report()` is
only a checked operation for requests documented to produce one output.

`ReportMetadata` includes:

- report ID, name, units, pages, and exact server price;
- requested, started, and created timestamps;
- state, format, and lexical report version;
- parent and zero or more child report IDs;
- electronic-seal and timestamp flags;
- no report bytes until download.

`DownloadedReport` includes metadata, messages, and `bytes::Bytes`. Its `Debug`
shows only safe metadata and byte length, never report contents.

### 5.6 Charge approval

```rust
pub enum ChargeApproval {
    FreeOnly,
    UpTo(Czk),
    Any, // Constructed explicitly; there is no unlimited default.
}
```

Before every `vratSestavu` call:

1. Refresh `seznamSestav(id)` and require a ready state (`zpracován` or
   `zaúčtován`). Pending, failed, cancelled, missing, and unknown states never
   reach the charging operation.
2. Require the server-provided current price. Do not use a hardcoded price list.
3. Compare the exact CZK amount with the approval. `FreeOnly` accepts only zero;
   `UpTo` rejects a larger amount; `Any` is an explicit unlimited approval.
4. On missing price or exceeded limit, return before `vratSestavu` and make no
   charge attempt.
5. Only then perform one download request. Do not retry it automatically.

The preflight request count must be documented. A high-level workflow may reuse
fresh ready metadata obtained by its own wait step, but must not rely on a stale
caller-provided price.

Once the download request has been dispatched, **any** failure without an
authoritative server outcome has an ambiguous charging outcome: this includes
transport errors, HTTP errors, truncated or oversized responses, malformed
SOAP/MTOM, and report-ID correlation mismatches. Return an error carrying the
report ID and `charge_outcome_uncertain() == true`. Failures proven to occur
before `vratSestavu` is dispatched, including approval/preflight rejection, are
not charge-uncertain. Instruct the caller to inspect status: after `zaúčtován`, a
subsequent explicit retrieval should not be charged again according to the
official first-open rule.

### 5.7 Bounded waiting and pacing

`WaitOptions` contains initial delay, poll interval, maximum total wait, and
optionally a capped backoff. Initial defaults are a 2-second initial delay,
2-second poll interval, and 120-second total deadline, configurable after trial
measurements.

- Poll only `seznamSestav(id)`.
- Pending states: `čeká`, `vytváří se`, `podepisuje se`.
- Ready states: `zpracován`, `zaúčtován`.
- Failed states: `chyba při vytváření`, `chyba při zpracování`.
- Cancelled state: `zrušen`.
- Preserve unknown states as raw strings and fail closed rather than looping
  until the deadline without explanation.
- Dropping the future cancels waiting; no background task is spawned.
- HTTP permits are released during sleeps.

A shared per-client generation pacer reserves start times at least two seconds
apart across clones. It does not sleep while holding the general request permit.
It is not a global limiter across independently constructed clients.

### 5.8 High-level typed helpers

Each helper composes generation, bounded waiting, approved download, and typed
parsing. It accepts `WaitOptions` and `ChargeApproval`; there is no helper that
charges without an approval argument.

Typed XML helpers accept only single-output XML requests whose effective version
is a locally supported `ReportVersion`, selected either by the family default or
an explicit pinned override. They reject `LatestAvailable`, PDF output, and
`pripojitXML` **before generation**. PDF+XML bundles stay on the raw lifecycle:
every report ID remains visible, every charging retrieval requires its own
approval, and workflow failures retain the whole batch or all known IDs rather
than only a single child. One approval must never implicitly authorize multiple
downloads. Binary helpers likewise support only their single documented output.

```rust
let report = client
    .parcel_info_report(request, WaitOptions::default(), approval)
    .await?;
```

Planned helpers and outputs:

| Helper | Typed output |
| --- | --- |
| `parcel_info_report` | `XmlReport<ParcelInfoReport>` |
| `building_info_report` | `XmlReport<BuildingInfoReport>` |
| `unit_info_report` | `XmlReport<UnitInfoReport>` |
| `building_right_info_report` | `XmlReport<BuildingRightInfoReport>` |
| `proceeding_info_report` | `XmlReport<ProceedingInfoReport>` |
| `definition_points_report` | `XmlReport<DefinitionPointsReport>` |
| `ownership_sheet_report` | `XmlReport<OwnershipSheetReport>` with legacy/NTR variants |
| `ownership_overview_report` | `XmlReport<OwnershipOverviewReport>` |
| `ownership_overview_with_properties_report` | `XmlReport<OwnershipOverviewWithPropertiesReport>` |
| `rights_record_report` | `XmlReport<RightsRecordReport>` |
| `account_statement_report` | `XmlReport<AccountStatementReport>` when XML is selected |
| `prices_by_properties_report` | `XmlReport<PricesByPropertiesReport>` |
| `prices_by_proceeding_report` | `XmlReport<PricesByProceedingReport>` |
| `prices_by_cadastral_area_report` | `ZipReport<MonthlyPriceArchive>` |
| `cadastral_map_report` | `BinaryReport` with PDF format/metadata |
| `collection_document_report` | `BinaryReport` with PDF format/metadata |
| `geometric_plan_report` | `BinaryReport` with PDF format/metadata |

`XmlReport<T>` contains the parsed value, report metadata/messages, and original
XML bytes. `ZipReport<T>` additionally retains the original archive and safe
entry metadata. Unsupported versions are rejected before generation when the
typed helper cannot decode them; callers can still use the raw lifecycle for a
newer version.

Single-report workflow errors retain the stage and, once known, the report ID.
Raw bundle errors retain the batch or every known ID. A wait/decode/download
failure must never trigger blind regeneration. For post-download parse failures,
documentation explains how to retrieve the already charged report by ID through
the raw API.

## 6. Domain model plan

### 6.1 Scalar and identifier types

Use validated newtypes and exact values:

- Distinct `ParcelId`, `BuildingId`, `UnitId`, `BuildingRightId`, `SubjectId`,
  `OwnershipId`, `ProceedingId`, `OwnershipSheetId`, `AddressId`, `DocumentId`,
  and `ReportId` newtypes.
- WSDP `IdType` and current search/share numerator/denominator values permit up
  to 30 decimal digits. Store IDs as validated decimal strings, preserving their
  lexical form; do not use `u64` or floating point. Legacy report `ID_TYPE` is a
  separate contract: it can be empty and has no equivalent 30-digit restriction,
  so represent it separately rather than forcing it through `IdType` validation.
- Use **`rust_decimal::Decimal` for every decimal and fractional value**, including
  money, coordinates, and integral-valued shares; do not add `bigdecimal` or a
  second decimal representation. Apply sign, scale, positivity, and nonzero-
  denominator validation according to the particular XSD. Current search
  ownership shares are positive; legacy share fields can be signed and can have
  missing components.
- Document `rust_decimal`'s approximately 28–29 significant-digit range as an
  explicit crate limitation. IDs remain decimal strings and are unaffected.
  Although some XSD decimal/integer values permit 30 digits or have no
  `totalDigits` limit, an out-of-range value returns `Error::DecimalOutOfRange`;
  it is never rounded, truncated, or silently converted. The report ID and raw
  downloaded XML remain recoverable through the raw lifecycle.
- Account-statement charge/credit fields can be negative. Use a separate
  non-negative `Czk` wrapping `Decimal` only for queue prices and charge approval,
  never as a blanket type for every monetary element. No money type has a
  binary-float constructor. `ReportVersion` remains lexical even though its text
  resembles a decimal.
- Use fixed-width newtypes only where their XSD range actually fits, such as
  parcel root/subdivision numbers and LV numbers.
- Preserve leading zeroes in string-like identifiers such as IČO where required.
- Use **open enums with raw-preserving `Other` variants** for dynamic code lists
  and protocol classifications: known values receive named variants, while future
  values remain representable. Use a plain validated newtype only when there is
  no useful stable set of known variants. Labels are display metadata, never
  identifiers.
- `ReportVersion` is a validated lexical `MAJOR.MINOR` value, not `f64`, with
  named constants for supported versions, checked string parsing for dynamic
  values, and defaults defined per report family rather than globally.
- Use `XmlDate` for report/search `xs:date` values so optional offsets and the
  original lexical form are retained; code-list civil-date projections first
  undergo strict XML-date validation. Introduce `XmlDateTime` to preserve a civil
  date-time, optional offset, and arbitrary fractional-second precision for
  `xs:dateTime`. Generate WS-Security `Created` as UTC. Do not force naive server
  timestamps into UTC or assume an undocumented timezone.
- Map `a`/`n` to `bool` only at the public-model boundary and preserve unknown
  values in open-ended protocol enums where the XSD is not closed.

Provide the usual `new`, checked conversions, `as_str`/`Display`, `FromStr`, and
`TryFrom` implementations as appropriate. Serde deserialization, when enabled,
must call the same validators.

### 6.2 Shared domain types

Share only types whose semantics are genuinely shared:

- territorial codes and names;
- parcel/building/unit/right identifiers and natural keys;
- subjects, addresses, partnership members, and exact ownership shares;
- ownership targets and legal-relation codes;
- proceeding identifiers and document identifiers;
- service messages, report metadata, and output formats.

Do not force search records and report-specific records into one structure just
because their Czech local names match. For example, search `Parcel`,
report-parcel, NTR parcel, and simplified parcel-identification structures have
different fields and cardinality.

### 6.2.1 Ownership model sketch

NTR means **nové tiskové řešení** (“new print solution”), ČÚZK's newer XML
report technology. Legacy LV 1.0 is uppercase and presentation-shaped; NTR LV
3.0 is normalized, namespaced, and has richer metadata and recursive legal
relations. Both formats receive complete typed models in the first release, as
confirmed during design. NTR 3.0 is the default; trial `dejVerzeSestav` confirmed
its availability on 2026-11-20, and legacy remains explicitly selectable. NTR
currently changes the LV report, not the
parcel/building report schemas.

The following is an architectural sketch rather than the final field-by-field
API. It shows the shared semantic layer; the full legacy/NTR models retain every
schema field and their own stronger requiredness rules.

Subjects and addresses:

```rust
#[non_exhaustive]
pub enum Subject {
    NaturalPerson(Person),
    LegalPerson(Organization),
    JointProperty(JointProperty),
    Other(UnknownSubject),
}

pub struct Person {
    pub id: Option<SubjectId>,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    pub titles_before: Vec<String>,
    pub titles_after: Vec<String>,
    pub birth_number: Option<PersonalIdentifier>, // Redacted Debug.
    pub addresses: Vec<Address>,
}

pub struct Organization {
    pub id: Option<SubjectId>,
    pub name: Option<String>,
    pub company_id: Option<CompanyId>, // String-backed; preserves leading zeroes.
    pub company_id_suffix: Option<u16>,
    pub addresses: Vec<Address>,
}

pub struct JointProperty {
    pub id: Option<SubjectId>,
    pub name: Option<String>,
    pub characteristic: Option<SubjectCharacteristic>,
    // NTR requires one participant and permits a second; do not require exactly 2.
    pub participants: Vec<Person>,
}

pub struct Address {
    pub ruian_id: Option<AddressId>,
    pub street: Option<String>,
    pub house_number: Option<HouseNumber>,
    pub orientation_number: Option<String>,
    pub municipality_part: Option<String>,
    pub municipality: Option<String>,
    pub city_district: Option<String>,
    pub city_borough: Option<String>,
    pub district: Option<String>,
    pub postal_code: Option<String>,
    pub delivery_post_office: Option<String>,
    pub country_name: Option<String>,
}

pub struct HouseNumber {
    // The number and its kind are independently optional in the source schemas.
    pub kind: Option<HouseNumberKind>,
    pub value: String,
}

#[non_exhaustive]
pub enum HouseNumberKind {
    Descriptive, // XSD code 1 / číslo popisné
    Evidence,    // XSD code 2 / číslo evidenční
    Other(String),
}
```

The full models preserve distinct outer authorized-subject and nested person IDs,
partnership characteristics, repeated legacy identity/address elements, and all
report-specific fields. The shared projection uses vectors where legacy content
can repeat and never selects an arbitrary “primary” address silently.

Shares and legal relations:

```rust
pub struct ReportedShare {
    // Optional in the shared view because legacy components can be absent.
    // No 1/1 default is invented.
    pub numerator: Option<rust_decimal::Decimal>,   // Scale 0 when present.
    pub denominator: Option<rust_decimal::Decimal>, // Scale 0, nonzero when present.
}

#[non_exhaustive]
pub enum LegalRelationCode {
    Ownership,              // 30: Vlastnické právo
    ContractualMortgage,    // 32: Zástavní právo smluvní
    EasementByDocument,     // 36: Věcné břemeno (podle listiny)
    PreemptionRight,        // 54: Předkupní právo
    BuildingRight,          // 64: Právo stavby
    AccessoryCoOwnership,   // 221: Přídatné spoluvlastnictví
    ChangeOfPreemptionRight, // 320: Změna předkupního práva
    // Generate a distinct named variant for every KOD in the pinned full registry.
    Other(String),
}

pub struct LegalRelation {
    // NTR has a code; legacy parcel/building reports generally provide only text.
    pub code: Option<LegalRelationCode>,
    pub name: String,
}

pub struct LegalRelationType {
    // Direct response to seznamTypuPravnichVztahu; no bundled lookup is needed.
    pub code: LegalRelationCode,
    pub name: String,
    pub is_ownership: bool, // WSDP `vlastnictvi`
}

pub struct OwnershipSheetReference {
    pub id: Option<OwnershipSheetId>,
    pub number: Option<OwnershipSheetNumber>,
    // Nested references often omit this; containing-sheet context may fill it.
    pub cadastral_area: Option<CadastralAreaReference>,
}
```

`LegalRelationCode::as_str()` returns the official `KOD` and `Other` retains an
unrecognized value unchanged. Never derive a code or category by translating or
matching Czech relation names. Names are not unique in the official registry: for
example, codes 85/180, 86/181, and 196/307 form duplicate-name pairs. If only
legacy text is present, `code` remains `None`. Parsing does not require an online
code-list request.

#### Legal-relation registry and classification

Use the official **Číselníky právních vztahů a listin** page as the provenance and
update entry point. It publishes the extended `SC_T_PRAVNICH_VZT` CSV (ZIP), whose
`KOD`/`TPR_KOD`/`NAZEV`/`VLASTNICTVI`/.../`PRSV_ZAVISLE_JPV` columns exactly match
the supplied registry layout, and the separate `SC_T_PREDMETU_R` CSV (ZIP)
referenced by `TPR_KOD`. The current NTR guide explicitly cites both this web
registry and `seznamTypuPravnichVztahu` when defining the `VLASTNICTVI` flag.

Generate checked-in Rust mappings during development from complete, pinned CSV
snapshots, including historical rows. Do not fetch or generate them in `build.rs`.
Every `LegalRelationCode` known variant maps one-to-one by `KOD`; duplicate or
changed names never control generation. Keep variant identity stable when only
an official label changes, retain variants for expired rows, and review any actual
variant rename as a separate public-API change. Generate
`ProceedingSubjectMatterCode` separately from `SC_T_PREDMETU_R.KOD`. It must not
be confused with either a
legal-relation code or a person/subject identifier. Both generated code enums are
open and raw-preserving. A generated metadata record retains all official columns:

```rust
pub struct LegalRelationMetadata {
    pub code: LegalRelationCode,                         // KOD
    pub proceeding_subject_matter: ProceedingSubjectMatterCode, // TPR_KOD
    pub name: &'static str,                              // NAZEV
    pub is_ownership: bool,                              // VLASTNICTVI
    pub benefits_subject: bool,                          // PRO_OS
    pub benefits_property: bool,                         // PRO_NEMOVITOST
    pub burdens_subject: bool,                           // K_OS
    pub burdens_property: bool,                          // K_NEMOVITOSTI
    pub valid_from: Date,                                // PLATNOST_OD
    pub valid_to: Option<Date>,                          // PLATNOST_DO
    pub nationwide_scope: bool,                          // CELOSTATNI_PUSOBNOST
    pub three_way_link: Option<ThreeWayLinkCode>,        // TROJVAZBA
    pub section: Option<LegalRelationSection>,           // SEKCE
    pub joint_marital_property: bool,                    // SJM
    pub superior_legal_person: bool,                     // PRO_NADRIZENOU_PO
    pub creditor_share: Option<bool>,                    // PODIL_VERITELE
    pub requires_parent_relation: bool,                  // POVINNOST_R_JPV
    pub building_right: bool,                            // PRAVO_STAVBY
    pub ordering: Option<bool>,                          // PORADI
    pub accessory_co_ownership_restriction: Option<bool>, // PRSV_OMEZENI_PRAV
    pub accessory_co_ownership_dependent_relation: Option<bool>, // PRSV_ZAVISLE_JPV
}
```

Map populated `a`/`n` flags to `true`/`false`; fields for which the registry
permits blank values use `Option`. `ThreeWayLinkCode` preserves the observed
`M`, `O`, and `U` lexical codes plus `Other(String)` rather than inventing
semantics not defined by the registry page. `LegalRelationSection` has named `B`,
`C`, `D`, and documented section-A positions 1–9 plus `Other(String)`. These are
registry metadata, not substitutes for the actual XML relation/section, and an
expired `valid_to` must not make a historical report fail to parse.

Classification is explicitly tri-state:

```rust
impl LegalRelationCode {
    pub fn metadata(&self) -> Option<&'static LegalRelationMetadata>;
    pub fn is_ownership(&self) -> Option<bool> {
        self.metadata().map(|metadata| metadata.is_ownership)
    }
}

impl LegalRelation {
    pub fn is_ownership(&self) -> Option<bool> {
        self.code.as_ref().and_then(LegalRelationCode::is_ownership)
    }
}
```

Known current **and historical** codes return `Some(true)` or `Some(false)` from
metadata. `Other(String)` belongs to the **code layer**, not the Boolean
classification: an unknown coded relation has
`code = Some(LegalRelationCode::Other(raw_code))` and `is_ownership() == None`.
A legacy name-only relation has `code = None` and also returns `None`. Neither case
returns a guessed `false`; never manufacture an empty code or put the relation name
in `Other`. A directly fetched `LegalRelationType` retains the server's own concrete
`vlastnictvi` value even when the bundled snapshot predates that code. Do not
classify by `LegalRelationCode::Ownership`: codes 30, 115, 143, and 221 all have
`VLASTNICTVI = a` but represent different relationships. Similarly, `TPR_KOD` and
`SEKCE` are independent metadata, not ownership classifiers.

At each deliberate registry refresh, check the page's per-registry **Datum
aktualizace číselníku**, download both ZIPs manually, record retrieval dates and
SHA-256 hashes in `SCHEMAS.md`, review differences, regenerate the checked-in
mappings, and cross-check overlapping code/name/ownership values against
`seznamTypuPravnichVztahu` in the official trial environment. Never require runtime
updates or silently replace the pinned snapshot. Keep expired rows so historical
reports remain classifiable, and document each refreshed snapshot in the
changelog.

Ownership structure:

```rust
pub struct OwnershipSummary {
    pub scope: OwnershipScope,
    pub relations: Vec<OwnershipRelationGroup>,
}

pub enum OwnershipScope {
    Sheet(OwnershipSheetReference),
    Property {
        property: PropertyReference,
        containing_sheet: Option<OwnershipSheetReference>,
    },
}

pub struct OwnershipRelationGroup {
    pub relation: LegalRelation,
    pub records: Vec<OwnershipRecord>,
}

pub struct OwnershipRecord {
    // Required by full NTR records, unavailable in legacy ownership records.
    pub id: Option<OwnershipId>,
    pub parent_id: Option<OwnershipId>,

    // NTR has exactly one target; legacy VLA_IDENT can be empty, repeated, or mixed.
    pub targets: Vec<OwnershipTarget>,

    // The wrapper exists; individual components can be absent in legacy data.
    pub share: ReportedShare,

    // Retain structural nesting even when parent_id is absent.
    pub subordinate_relations: Vec<OwnershipRelationGroup>,
}

pub enum OwnershipTarget {
    Subject(Subject),
    Parcel(ParcelReference),
    Building(BuildingReference),
    Unit(UnitReference),
    BuildingRight(BuildingRightReference),
}
```

This structure deliberately handles:

- ordinary subject ownership and accessory co-ownership targeting another
  property;
- joint property with one or two participants rather than forcing a pair;
- empty/repeated legacy targets rather than selecting the first;
- NTR's optional `nadrizeneId` and independently nested subordinate legal
  relations without conflating the two hierarchy mechanisms;
- sheet-scoped LV section-A ownership without arbitrarily assigning it to one
  property. Building/parcel/unit information reports can provide a
  property-scoped summary; an LV report also exposes a sheet-scoped summary.

The full NTR model makes required IDs, share components, relation code/name, and
single targets non-optional. The full legacy model retains empty `ID_TYPE`,
repeated choices, and optional elements exactly. The shared model is a convenient
projection, not a replacement for either complete report model.

### 6.2.2 Open-enum policy

Use typed enums generously, with different rules for requests and responses:

| Kind of value | Public approach |
| --- | --- |
| Closed request input, such as building number type 1/2 | Strict enum; reject unsupported input locally |
| Closed response enum, such as OFO/OPO/BSM | Known variants plus `Other(String)` for forward compatibility |
| Server-maintained string code list, such as legal relations | `#[non_exhaustive]` enum with known variants and `Other(String)` |
| Numeric WSDP message code | Known `MessageCode` variants plus `Other(String)`, because `xs:nonNegativeInteger` is unbounded; typed known-value and raw lexical accessors |
| Frequently changing or context-specific code with no useful stable variants | Validated newtype, optionally accompanied by a separate classification enum |
| Report state/format/version | Typed known values plus raw fallback where future values are possible |

This matches the useful `isds` pattern: callers can match common values directly,
while a server addition does not make decoding fail. Mark open response enums
`#[non_exhaustive]` so adding a known variant is not a downstream exhaustive-match
breaking change. Never use a unit `Other` variant that discards the raw value.

Examples of open enums include `SubjectKind`, `HouseNumberKind`, `ParcelKind`,
`LegalRelationCode`, `ProceedingSubjectMatterCode`, `SubjectCharacteristicCode`,
land/building/unit usage codes, `ReportState`, `ReportFormat`, `MessageLevel`, and
`MessageCode`. Request-side
constructors remain stricter where sending an unknown value would be a local
programming error.

### 6.3 Complete XML report families

Implement full public models for these official schemas:

| Report family | Model notes |
| --- | --- |
| `InformaceOParcelach` | KN and simplified-register parcels, land use, area determination, BPEJ, buildings, LV references, ownership, seals |
| `InformaceOStavbach` | Buildings, associated parcels, units, LV references, ownership, seals/JPV |
| `InformaceOJednotkach` | Units, containing buildings, parcels, floor area, shares, ownership |
| `InformaceOPravechStavby` | Building rights, component buildings, related parcels/LVs and ownership |
| `InformaceORizeni` | Participants, proceeding metadata, affected properties, deeds, GP records, operations, defects, proposed changes |
| `SeznamDefinicnichBoduParcel` | Definition points, object references, S-JTSK coordinates and blank/inapplicable IDs |
| `PrehledVlastnictvi` | Subject and ownership bodies grouped by territorial unit and legal relation |
| `PrehledVlastnictviSNemovitostmi` | The above plus complete parcel/building/unit/right details |
| `EvidencePravProOsobu` | Full/partial/no-match branches, subject identification and recorded rights |
| `VypisStavuZakaznickehoUctu` | Account/credit/participant data, charge rows, units, totals and linked outputs |
| `CenoveUdajeDleNemovitosti` | Properties, parent properties, deeds, prices, currencies and historical relationships |
| `CenoveUdajeDleRizeni` | Proceedings, deeds, prices, currencies and related properties |
| `VypisZKatastruNemovitosti` legacy | Presentation-shaped sections A–F, simplified parcels, other rights, acquisition titles and BPEJ |
| `VypisZKatastruNemovitosti_3.0` NTR | Namespaced normalized LV model, metadata, recursive legal relations and optional XAdES signature |
| `cenoveUdaje/1.0` monthly ZIP entries | Proceedings, deeds/prices, properties, GML points and monthly/cadastral attributes |

Model XSD choices as enums, including accessory co-ownership targets. Preserve
missing, empty, and explicit-nil distinctions where they affect meaning. Do not
default absent share components to `1`, unknown countries to `CZ`, or unknown
legal-relation text to a guessed enum.

Legacy reports have no target namespace; NTR uses several namespaces and can
change namespace within an extended subtree. Always match expanded names
`{namespace}local`, never prefixes or local names alone.

Keep legacy LV and NTR LV modules separate. NTR supports multiple LVs in one
report, recursive legal relationships, and distinct report UUID/run/DP metadata;
none of these should be collapsed into queue-level report IDs.

### 6.4 Binary and ZIP outputs

- PDF helpers validate the declared report format and basic PDF signature, then
  return untouched bytes. They do not claim to validate electronic seals.
- MTOM and base64 decoding enforce decoded-byte limits, not only compressed HTTP
  limits.
- Price ZIP parsing enforces entry-count, per-entry, compression-ratio, and total
  uncompressed-size limits. Never extract entries to paths automatically and
  never trust entry filenames as filesystem paths.
- Monthly price entries are decoded by their expanded XML root name. Preserve
  original archive and entry bytes alongside typed values.

## 7. Internal architecture

```text
src/
  lib.rs                    # Crate docs and deliberate public re-exports
  client.rs                 # Arc client, builder, shared execution pipeline
  error.rs                  # Public errors and retry/charge uncertainty helpers
  model/
    id.rs                   # Validated IDs, codes, exact scalar values
    common.rs               # Messages, dates, addresses, shares, responses
    code_list.rs
    legal_relation.rs       # Generated KOD mappings and pinned registry metadata
    search.rs
    information.rs
    report.rs               # Requests, jobs, metadata, approval, raw outputs
    reports/
      parcel.rs
      building.rs
      unit.rs
      building_right.rs
      proceeding.rs
      definition_points.rs
      ownership_sheet_legacy.rs
      ownership_sheet_v3.rs
      ownership_overview.rs
      rights_record.rs
      account_statement.rs
      prices.rs
  wire/
    soap.rs                 # Envelope, faults, namespaces, bounded XML reader
    auth.rs                 # WS-Security UsernameToken11
    mtom.rs                 # multipart/related + XOP and inline base64
    code_list.rs
    search.rs
    information.rs
    report.rs               # Operations, report status and version discovery
    reports/                # Wire DTOs and full report conversion
  examples/                 # Consumer-oriented compile-checked examples
  tests/                    # Public API, fixtures, and opt-in live tests
```

Keep public modules/re-exports deliberate; wire types remain private. Reuse the
`../isds` pattern of operation-oriented methods, shared transport execution,
private XML code, explicit models, and consumer examples.

**Start from the actual `../isds` repository workflows rather than inventing new
ones.** Adapt its `AGENTS.md` thin pointer, `CONTRIBUTING.md` as the source of
truth, README/crate-doc relationship, changelog format, offline test layout,
feature matrix, CI commands, schema-provenance process, and release checklist.
Retain its command names and conventions where applicable. Deviations should be
limited to documented WSDP needs—especially paid-production test gating, report
lifecycle safety, explicit MSRV, and the actual repository/CI provider—rather
than a newly designed contribution or release process.

### 7.1 Shared execution pipeline

Every request goes through one path:

1. Select a fixed service endpoint and an explicit `Operation` descriptor.
2. Acquire the bounded request permit; apply generation pacing when required.
3. Serialize the operation body through namespace-aware XML writers.
4. Add SOAP headers, fresh WS-Security credentials, `SOAPAction`, content type,
   and a versioned crate user agent.
5. Execute with HTTPS validation, no redirects, and configured timeouts.
6. Classify authentication statuses before attempting to expose response bodies.
7. Read the body with a hard byte cap, checking `Content-Length` early and each
   streamed chunk thereafter.
8. Parse SOAP XML or bounded MTOM; reject DTDs/entities and excessive XML nodes
   or depth.
9. Decode SOAP faults and normal `vysledek` messages separately.
10. Return typed data plus non-error messages, or a structured error with safe
    operation context.

`SOAPAction` values come from the operation descriptor, not string concatenation.
In particular, `najdiListinuVeSbirceListin` uses the exceptional action ending in
`najdiListinu`.

### 7.2 SOAP and XML handling

- Use a real XML writer, not interpolated unescaped strings.
- Send `text/xml; charset=utf-8` for ordinary SOAP 1.1 requests and accept both
  SOAP XML and multipart responses for report downloads.
- Create a fresh random nonce, UTC `Created`, and unique token ID for each
  UsernameToken. PasswordText exists only inside the encrypted request body.
- Match namespaces and local names independently of prefixes.
- Reject duplicate singleton elements, malformed cardinality, invalid UTF-8,
  DTDs/entity declarations, and excessive nesting.
- Use separate limits for small SOAP envelopes and potentially large reports.
  Initial report limits can be 256 MiB, 2,000,000 XML nodes, and depth 256, all
  configurable and validated by large-report tests.
- Do not include response fragments in parser errors; reports contain personal
  and commercially sensitive data.
- Preserve original downloaded bytes before any parsing or normalization.

Use handwritten, schema-driven internal mappings and small helpers/macros for
repetitive structures. Do not add a WSDL code generator or network schema fetch
to `build.rs`. This keeps builds deterministic and allows the public model to be
idiomatic rather than schema-generated.

### 7.3 MTOM/XOP

Support both documented/observed response forms:

1. `text/xml` with inline `souborSestavy` base64;
2. `multipart/related` with an XOP SOAP part and binary attachment referenced by
   `xop:Include`/`cid:`.

Use a maintained MIME parser where suitable, behind a narrow private module.
Validate the multipart start part, content IDs, exactly the expected attachment
references, and decoded size. Correlate the returned report ID with the requested
ID before returning bytes. Do not silently ignore a mismatch.

Fully buffered but strictly bounded MTOM is acceptable for the first release.
Streaming-to-writer downloads can be added later without changing typed models.

## 8. Errors, retries, and recovery

Use a non-exhaustive `wsdp::Error` and crate `Result<T>` with at least:

- invalid local configuration or input;
- transport failure with safe `Operation` context;
- unexpected HTTP status;
- authentication/authorization failure;
- SOAP fault with fault code/string;
- application `ServiceError` with all `ServiceMessage` values and raw codes;
- response/report/archive size limit exceeded;
- malformed or mismatched response;
- `DecimalOutOfRange` for a value exceeding the deliberately uniform
  `rust_decimal` representation, without exposing the sensitive raw value in the
  error's display output;
- report failed/cancelled/unexpected state;
- report wait timeout containing `ReportId`;
- report price unavailable or charge limit exceeded;
- unsupported report version;
- report workflow failure with stage and the known report ID(s)/batch.

Provide narrowly useful classifiers rather than a large closed code enum:

- `is_authentication_failure()`;
- `is_busy()` for service messages 477/210;
- `is_not_found()` for relevant 304-style outcomes;
- `is_password_expired()` for 205;
- `report_id()`/`report_ids()`;
- `submission_outcome_uncertain()` after post-dispatch generation failure;
- `charge_outcome_uncertain()` after post-dispatch download failure;
- `password_change_outcome_uncertain()` after post-dispatch password-change
  failure.

Uncertainty is based on dispatch and authoritative outcome, not on one error
variant. After generation is dispatched, HTTP errors, body-limit aborts,
truncation, malformed responses, and correlation mismatches are just as ambiguous
as connection failures unless the server definitively establishes rejection.
The same rule applies to charging downloads and password changes. Validation or
connection failure proven to occur before dispatch is not marked uncertain.

Initial retry policy is deliberately simple: **no automatic application-level
retries**. Applications can retry idempotent searches, code lists, and status
checks with their own bounded backoff/jitter. Documentation should identify safe
candidates and explain why generation/download/password-change/authentication
are excluded. After an ambiguous password change, the shared credentials-required
state prevents automatic use of either password until the caller resolves it.

Never expose credentials, natural-person identifiers, report XML, PDF bytes, or
raw response bodies through `Display`/`Debug` of errors.

## 9. Dependencies and features

Follow `../isds` where appropriate, evaluating versions at implementation time:

| Purpose | Planned crate |
| --- | --- |
| HTTP/TLS | `reqwest` with default features off, rustls, and streaming |
| Async/time/synchronization | `tokio` with minimal library features |
| Bytes/streams | `bytes`, `futures-util` |
| XML writing/parsing | `quick-xml`, `roxmltree` or an equivalent namespace-safe bounded reader |
| Secrets | `secrecy` |
| Errors | `thiserror` |
| Dates/times | `jiff` |
| Exact decimals | `rust_decimal` for every decimal/fractional value; string-backed IDs; fixed-width integers only where their XSD range fits |
| Base64/random nonce | `base64`, `getrandom`/`rand` |
| MIME/MTOM | `mime` plus a maintained MIME parser if it fits the bounded XOP design |
| ZIP | `zip` with independent extraction limits |
| Optional integrations | `serde`, `tracing` |
| Offline HTTP tests | `wiremock` |

Avoid a native libxml/OpenSSL dependency. Evaluate rustls backend build costs as
in `isds`; check a musl target if the same deployment target is desired.

Features:

```toml
default = []
serde = ["dep:serde"]
tracing = ["dep:tracing"]
```

`serde` applies to public data types only where validation invariants can be
maintained. `tracing` instruments operation, fixed endpoint, environment, HTTP
status, and byte counts, but excludes credentials, queries, personal IDs, report
IDs, filenames, and payloads by default. The crate never installs a subscriber.

Use Rust 2024 and forbid unsafe code. Declare an explicit MSRV after dependency
resolution and test both that MSRV and current stable in CI; do not confuse
“current stable” with an MSRV policy.

## 10. Security and privacy checklist

- Store passwords/new passwords as secrets and redact them from all debug output.
- Never write supplied production credentials to the repository, fixtures,
  command logs, examples, `PLAN.md`, or CI output.
- Use only fixed official HTTPS endpoints for real credentials; validate TLS and
  disable redirects.
- Redact natural-person query fields, birth numbers, rights-record inputs, and
  downloaded contents from tracing and errors.
- Treat all report content as sensitive. Provide no implicit cache or background
  upload and no automatic logging of parsed models.
- Reject XML DTD/entity constructs and enforce body, decoded-body, XML complexity,
  recursion, and ZIP expansion limits.
- Do not automatically repeat authentication failures or password changes. On
  confirmed password-change success update shared credentials; on ambiguous
  outcome block further use until explicit credential replacement, and never
  probe both candidate passwords.
- Do not automatically delete reports. Deletion is explicit and limited to the
  supplied report ID.
- Keep seal/timestamp metadata distinct from `signature_verified`; the latter is
  not available until real cryptographic verification is implemented.

## 11. Testing strategy

### 11.1 Fixture and schema provenance

Create `SCHEMAS.md` recording official source URLs, retrieval dates, versions,
and SHA-256 hashes. Do not download schemas or registries during builds. Check in
the selected CSV snapshots used to generate legal-relation mappings, and record
hashes for both their ZIP containers and extracted CSV files.

The registry page reported **2026-11-20** as the update date for both registries
below when they were retrieved on **2026-11-20**. `SC_T_PRAVNICH_VZT.csv` contains
317 unique rows, with non-contiguous `KOD` values through **320**;
`SC_T_PREDMETU_R.csv` contains 96 unique rows. In particular, legal-relation code
320 (`Změna předkupního práva`) is valid from **2026-11-18**, so a supplied extract
ending at code 319 is not the current snapshot. The downloaded CSVs are
semicolon-delimited Windows-1250 text with LF line endings; the development
generator must decode that encoding explicitly rather than treating the files as
UTF-8. Reconfirm the format on every refresh instead of assuming it is an
unchangeable protocol contract.

Initial official archive and registry hashes:

```text
WSDP_10_4_2026.zip
b2ebd76b275c47f96f55fd3aff45eb91e3b324ceadd603556d2d5e1af2336e7c

NTR_20261006.zip
23811d2d11d330f6465d42624b3b0735bd70a4f8c2e62a8ab6781e601232e51e

ukazky.zip
b298ab533952329dba6157a5c8d568aa07f6add3f21eb355a402945f729c7ef6

SC_T_PRAVNICH_VZT.zip
14b981dc4b0bf00ff6f2b695a1267484b390a1c977810693b1c44120b748066c

SC_T_PRAVNICH_VZT.csv
31868ccea32fc19c5578e3c035b1c7fd17cd8263bfa25139dcdda45867f43c7a

SC_T_PREDMETU_R.zip
90cbba441c0ff81c13294184c36ab4ea9f5529bc11db4edeb0d9c80b3ea8b486

SC_T_PREDMETU_R.csv
b6d420ae3fb37ebe10ca5043adc60aa44d0f773f4e9cfb585909e4fde4f9de6f
```

Use the official XML examples as initial fixtures, but note that they date from
2022 and contain historical prices. Revalidate them against current schemas and
supplement them with current anonymized trial outputs. Store only non-secret,
anonymized, redistributable fixtures. Add focused synthetic fixtures for edge
cases that official samples do not cover.

### 11.2 Offline unit and contract tests

Test at several layers:

**Identifiers and scalar values**

- 30-digit IDs and share components, plus empty/unbounded legacy `ID_TYPE`;
- leading zeroes, boundary values, invalid signs/characters;
- exact money/decimal comparisons and no float constructors;
- `rust_decimal` boundary tests for every bounded XSD range and explicit
  `DecimalOutOfRange` tests for 30-digit values outside its capacity and unbounded
  `xs:decimal` fields, proving there is no rounding and the raw report remains
  recoverable;
- negative, zero, and missing legacy share/monetary components where allowed;
- lexical report versions and naive/offset XML date-times;
- unknown code/status fallbacks and validated serde conversions.

**SOAP requests**

- Method, path, service namespace, body sequence, and every `SOAPAction`, including
  the deed-search exception;
- selector choices and account-specific authorization fields;
- no invalid fields silently serialized;
- fresh nonce/created values through an injectable deterministic test source;
- no credential leakage in debug, errors, or traces.

**Responses and errors**

- Success with mixed informational/warning/operational messages;
- SOAP validation and WS-Security faults;
- 304/310/392/477 and unknown codes;
- empty, not-found, incomplete, and multi-result searches;
- required-but-empty versus optional containers;
- malformed XML, duplicate singletons, DTDs, excessive depth/nodes, bad UTF-8;
- authentication error precedence over misleading HTML/proxy bodies.

**Reports and billing**

- Every report state and unknown-state behavior;
- parent/child PDF+XML relationships and multi-report generation;
- wait timeout/failure retaining report ID;
- generation calls from client clones paced at least two seconds apart;
- no `vratSestavu` during generate/list/status/wait;
- free-only, below-limit, exact-limit, above-limit, missing-price, and
  already-charged preflight behavior;
- no automatic retry after generation/download timeout or authentication error;
- pre-dispatch failures are not uncertain, while post-dispatch HTTP 500,
  truncation, response-size abort, malformed SOAP/MTOM, and correlation mismatch
  set the applicable submission/charging uncertainty flag with exact request
  counts;
- family defaults pin the expected version, named constants and dynamic parsing
  agree, and typed helpers reject server-latest/unsupported versions, PDF, and
  attached XML before generation; raw `xml_latest` omits `verze` and preserves
  returned metadata/bytes; raw bundles retain all IDs and require separate
  download approvals;
- confirmed password success updates all clones, definitive rejection preserves
  old credentials, and ambiguous change blocks later requests until explicit
  replacement without probing either password;
- no automatic report deletion.

**Payloads and full models**

- Inline base64 and MTOM/XOP variants, CID mismatches, malformed multipart data,
  decoded-size overflow, and report-ID mismatch;
- One full fixture test per XML report family;
- Legacy and NTR LV roots, multiple LVs, recursive legal relations, mixed
  namespaces, optional signatures, and unknown future values;
- Open enums return named known values and raw-preserving `Other` values, including
  numeric message codes and string legal-relation codes;
- Generated legal-relation and proceeding-subject-matter mappings cover every
  unique `KOD` in the pinned 317-row/96-row CSV snapshots, including historical
  rows and code 320; every `TPR_KOD` resolves to the separate proceeding registry,
  and duplicate names never collapse mappings;
- Registry decoding preserves blank optional flags, raw `TROJVAZBA`/`SEKCE` codes,
  validity dates, and exact names; pinned snapshot row counts/hashes are checked
  during the documented regeneration workflow;
- NTR relation code 30 maps to `LegalRelationCode::Ownership`; codes 30, 115, 143,
  and 221 return `Some(true)` from metadata-based `is_ownership()`, code 32 returns
  `Some(false)`, and `Other` returns `None`; legacy name-only relations retain
  `code = None`, also return `None`, and are never classified from Czech text;
- A runtime `LegalRelationType` preserves the server's `vlastnictvi` value even if
  its code is not in the bundled snapshot;
- Legacy empty/repeated/mixed targets, NTR single targets, joint property with one
  or two participants, and recursive relations with and without parent IDs;
- Missing share components stay missing, while full NTR parsing enforces required
  positive components;
- Sheet-scoped LV ownership is not silently assigned to an arbitrary property;
- Every XSD choice/cardinality edge represented in the public models;
- XML warning 371 with payload retention;
- PDF signature checks without seal-verification claims;
- Monthly price ZIP decoding, multiple entries, unsafe paths, excessive entries,
  compression bombs, and total expansion limits.

Use `wiremock` to assert exact request counts and prove that rejected charge
approval makes zero download requests.

### 11.3 Official trial tests

The official trial endpoint is
`https://wsdptrial.cuzk.gov.cz/trial/ws/wsdp/3.1`. ČÚZK publishes these shared
accounts on the trial page (updated 2026-10-23); treat that page as the source of
truth if its credentials change:

| Account type | Username | Password |
| --- | --- | --- |
| Ordinary paying user | `WSTEST` | `WSHESLO` |
| Free-of-charge | `WSTESTB` | `WSHESLOB` |
| Verifying | `WSTESTO` | `WSHESLOO` |
| Czech POINT | `WSTESTC` | `WSHESLOC` |

These are publicly documented test credentials, not production secrets. Examples
and tests still accept them through `WSDP_TRIAL_USERNAME` and
`WSDP_TRIAL_PASSWORD`, selected explicitly for the desired account type, so the
same interface works with private trial credentials and no `.env` file is needed.
Trial reports are free and their prices merely simulate production charging;
`ChargeApproval` remains mandatory so billing safeguards are exercised rather
than bypassed. The fictitious/anonymized database covers a sample from the
Prachatice workplace, and the official trial page provides matching sample object
identifiers. Cadastral maps and deed ordering are unavailable or limited, and HTML
reports are unsupported.

Keep trial tests `#[ignore]`d and out of normal CI. Require all of:

- `WSDP_RUN_TRIAL_TESTS=1`;
- credentials supplied through environment/secret manager;
- `Environment::Trial` hardcoded in the test;
- serial execution for account/report lifecycle tests.

Cover service status, report-version discovery, territorial lookup, each search
family with published sample data, representative full XML reports, NTR LV when
enabled, inline/MTOM downloads, approval checks with simulated prices, and
cleanup of only report IDs created by that test. Cross-check the current
`seznamTypuPravnichVztahu` code/name/`vlastnictvi` values and registry update date
against the pinned web CSV without assuming that the runtime operation includes
historical rows. Where attached XML is enabled,
also inspect parent/child state and price transitions and retrieve each output
only with its own approval; verify the documented no-additional-charge behavior
against live metadata rather than inferring it from offline fixtures.

Do not assume the public shared trial account has an empty report list, do not
delete other users' reports, and never test password change on shared trial
credentials. Authentication-failure lockout behavior belongs only in offline
mocks.

### 11.4 Production verification

Production tests are a separate, manually requested maintainer step after all
offline and trial checks pass. They must require:

- a distinct production opt-in variable;
- production credentials from the environment or a secret manager;
- explicit test object selectors;
- a small maximum total approved charge;
- serial execution and no CI secret or automatic trigger.

Prefer the lowest-cost parcel/building outputs needed to confirm production-only
behavior. Do not test password changes, authentication lockout, arbitrary report
deletion, map/deed/GP outputs, or broad ownership searches merely for coverage.
Do not print owner/report contents. Production credentials supplied for real
verification must never become test fixtures or documentation.

No production operation is needed while writing this plan.

## 12. Implementation phases

### Phase 0 — Repository and protocol foundation

- Create the `wsdp` crate, license/metadata, README skeleton, contributor guidance,
  changelog, CI skeleton, `.gitignore`, and `SCHEMAS.md`.
- Record official 3.1/NTR schemas and both legal-relation/proceeding-subject CSV
  registries with hashes; establish fixture and registry-refresh policies.
- Add client/environment/credentials/error skeletons and shared HTTP execution.
- Run a **trial-only protocol spike** for `stavWS`, one search,
  `dejVerzeSestav`, one generated XML report, and both observed download forms.
- Resolve enabled runtime report versions through `dejVerzeSestav`; do not infer
  them from XSD file comments. Confirm the named constants and per-family default
  versions, especially the NTR 3.0 ownership-sheet default.

**Exit:** deterministic offline build plus a documented, reproducible trial
request/response lifecycle with no production calls.

### Phase 1 — Scalar models, SOAP, and synchronous services

- Implement IDs, exact values, `XmlDateTime`, messages, response/error types,
  WS-Security, bounded SOAP parsing, and operation descriptors.
- Implement all code lists and search operations with full request/result models.
- Generate and test the pinned legal-relation/proceeding-subject code mappings,
  complete registry metadata, and tri-state ownership classification.
- Implement MBR, unit overview, LV preview, status, report versions, and explicit
  password change.
- Add offline request/response/error/security tests and common examples.

**Exit:** all non-report core operations are covered, including empty/truncated
results and account-specific request shapes.

### Phase 2 — Raw asynchronous report lifecycle

- Implement all `GenerateReport` request variants and semantic validation.
- Implement report batches, parent/child relationships, status/list, bounded wait,
  deletion, pacing, charge approval, raw downloads, and workflow recovery.
- Implement inline base64 and MTOM/XOP with response correlation and limits.
- Add billing/no-retry/no-hidden-download tests before any typed convenience
  helper is added.

**Exit:** every official output can be requested and retrieved safely as raw
bytes with explicit billing and resumable report IDs.

### Phase 3 — Property, proceeding, and point reports

- Implement full models/parsers for parcel, building, unit, building-right,
  proceeding, and definition-point XML.
- Add typed convenience helpers and complete fixture coverage.
- Test multiple natural-key matches, optional LV references, accessory ownership,
  simplified parcels, empty identifiers, and historical timestamps.

**Exit:** all six report families pass full-field fixture and round-trip wire
contract tests.

### Phase 4 — Ownership and rights reports

- Implement ownership overviews with and without properties, rights-record
  output, and legacy LV.
- Implement NTR 3.0 LV as a separate namespaced model with metadata, multiple
  LVs, recursive legal relations, and raw signature retention.
- Add typed helpers and convenience read accessors that do not discard official
  codes or relationship structure.

**Exit:** every ownership/right report family has full models and legacy/NTR LV
coverage, with no Kotlin-style lossy conversions.

### Phase 5 — Prices, account statement, and monthly ZIP

- Implement property/proceeding price reports and account-statement XML.
- Implement bounded monthly price ZIP parsing and `cenoveUdaje/1.0` models.
- Test exact money values, missing price data, multiple properties/deeds/months,
  and archive expansion defenses.

**Exit:** all price outputs and the account statement are typed and fully tested.

### Phase 6 — Binary outputs and account restrictions

- Complete cadastral-map, deed-collection, and geometric-plan PDF helpers.
- Verify account-context restrictions and fixed output formats locally.
- Add offline binary/MTOM fixtures and document trial limitations and production
  charge behavior. Do not use paid production calls merely to fill a matrix.

**Exit:** all broad-scope generation operations have safe high-level APIs.

### Phase 7 — Hardening and release readiness

- Complete examples for search, raw lifecycle, approved typed reports, PDF+XML
  relationships, and application-managed retry/recovery.
- Run the complete offline matrix, trial lifecycle, docs build, feature checks,
  package dry run, and MSRV/stable CI.
- Review all public models against the XSD element/attribute inventory and all
  public methods for billing/request-count documentation.
- Perform production verification only if explicitly requested and separately
  approved with a charge cap.

**Exit:** the broad acceptance checklist below passes without skipped or weakened
checks.

## 13. Documentation and examples

Write consumer documentation in English, retaining Czech protocol terms where
they clarify official behavior. Include:

- environment and account-type selection;
- credential handling and secret redaction;
- natural-key versus internal-ID lookup and multiple results;
- current versus historical queries;
- code-list filtering and 20,000-record limits;
- the full report state machine;
- a prominent **“generation is free; first download can charge”** section;
- charge approval, exact prices, failed download recovery, and no-retry rules;
- PDF+XML parent/child reports and version selection;
- legacy versus NTR LV models;
- seal metadata versus actual signature verification;
- XML/body/ZIP limits and sensitive-data handling;
- official trial setup and production-test restrictions.

Compile-check examples for:

1. find a parcel by cadastral area and number;
2. find a building and its units;
3. resolve a subject and request an ownership overview;
4. raw generate → wait → approve → download → parse;
5. create a report, persist its ID, then poll/download it in a separate process;
6. high-level typed parcel/building/LV calls using default and pinned versions;
7. recover a timed-out report by ID without regeneration;
8. handle PDF plus attached XML reports;
9. decode a monthly price ZIP safely.

Create `AGENTS.md` as a thin pointer to `CONTRIBUTING.md`, following the `isds`
pattern. Keep development commands and live-test safety in `CONTRIBUTING.md`.

## 14. CI and local checks

Use the same baseline as `isds`, adapted to this crate:

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked
cargo test --all-features --locked
cargo check --examples --locked
RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps --locked
cargo package --locked --allow-dirty
```

Also:

- check `serde` and `tracing` independently;
- check MSRV and current stable;
- check musl if required by deployment and supported by selected dependencies;
- keep normal CI entirely offline and credential-free;
- run live tests only manually, never on every push;
- do not create releases or publish automatically as part of implementation.

## 15. Definition of done for the broad first release

The release is functionally complete only when:

- [x] `Environment::Trial` and `Environment::Production` use official WSDP 3.1
      endpoints, with production never the default.
- [x] All five core services and all documented code-list/search operations are
      implemented. An explicit WSDL-derived operation inventory in tests covers
      33 code-list operations, eight lookup operations, and each service's
      `stavWS`, so coverage does not depend on informal counts.
- [x] MBR, eligible unit/LV information views, account status, report versions,
      and password change are covered.
- [x] Every report-generation operation has a typed request and safe raw output
      path.
- [x] Every listed XML report family has a complete public model and fixture
      tests, including legacy and NTR LV and monthly price data.
- [x] PDF/ZIP outputs are returned without data loss and with bounded decoding.
- [x] Report generation, waiting, charging download, and deletion are separate;
      every potentially charging helper requires `ChargeApproval`.
- [x] No status poll calls `vratSestavu`; no rejected approval calls it either.
- [x] No automatic generation/download/authentication retries exist, and
      generation calls are paced across client clones.
- [x] Errors preserve report IDs and charging/submission uncertainty where
      applicable, without exposing sensitive payloads.
- [x] Unknown codes and future values do not cause unnecessary decoding failure;
      raw XML remains available.
- [x] Generated legal-relation/proceeding-subject mappings cover every `KOD` in
      their pinned official CSVs, retain historical metadata, and provide tri-state
      ownership classification without name matching.
- [x] Offline checks, docs, examples, package checks, and manual trial lifecycle
      pass with no credentials committed.
- [x] Any production verification is separately opted in, charge-capped, and
      leaves no secrets or personal report content in artifacts.

## 16. Verification questions for implementation spikes

These are not product-design blockers and should be resolved from official trial
behavior rather than production experimentation:

1. Which exact report versions does `dejVerzeSestav` currently enable for non-LV
   report families? Trial verification on 2026-11-20 already confirmed LV 1.0 and
   NTR LV 3.0.
2. Which operations return inline base64 versus MTOM/XOP in practice, and what
   multipart headers/start-part behavior must the decoder accept?
3. Does the current trial service require or merely recommend Nonce/Created?
   The client will send them regardless, matching current official examples.
4. Which trial account types can actually call `dejInfoOJednotce` and
   `dejNahledLV`? Keep the API capable for all and let server authorization
   decide; document only confirmed behavior.
5. Are current official examples sufficient for every report branch, or which
   additional anonymized trial/synthetic fixtures are needed?
6. Are the initial body/XML/archive limits suitable for the largest realistic
   reports? Adjust them from measurements while retaining hard bounds.

## 17. Official sources

Primary documentation:

- [ČÚZK — Webové služby dálkového přístupu](https://cuzk.gov.cz/Katastr-nemovitosti/Poskytovani-udaju-z-KN/Dalkovy-pristup/Webove-sluzby-dalkoveho-pristupu.aspx)
- [WSDP 3.1 documentation ZIP (`WSDP_10_4_2026.zip`)](https://cuzk.gov.cz/Katastr-nemovitosti/Poskytovani-udaju-z-KN/Dalkovy-pristup/Webove-sluzby-dalkoveho-pristupu/WSDP_10_4_2026.aspx) — guide effective 2026-05-04, WSDLs, XSDs
- [New print solution / NTR documentation ZIP (`NTR_20261006.zip`)](https://cuzk.gov.cz/Katastr-nemovitosti/Poskytovani-udaju-z-KN/Dalkovy-pristup/Webove-sluzby-dalkoveho-pristupu/NTR_20261006.aspx) — updated 2026-10-06
- [Official WSDP PDF/XML examples](https://cuzk.gov.cz/Katastr-nemovitosti/Poskytovani-udaju-z-KN/Dalkovy-pristup/Webove-sluzby-dalkoveho-pristupu/ukazky.aspx)

Legal-relation registry provenance:

- [ČÚZK — Číselníky právních vztahů a listin, `SC_T_PRAVNICH_VZT` section](https://cuzk.gov.cz/Katastr-nemovitosti/Poskytovani-udaju-z-KN/Ciselniky-ISKN/Ciselniky-pravnich-vztahu-a-listin.aspx#SC_T_PRAVNICH_VZT) — column definitions and per-registry update dates
- [Official extended legal-relation CSV ZIP (`SC_T_PRAVNICH_VZT.zip`)](https://services.cuzk.cz/sestavy/cis/SC_T_PRAVNICH_VZT.zip)
- [Official proceeding-subject-matter CSV ZIP (`SC_T_PREDMETU_R.zip`)](https://services.cuzk.cz/sestavy/cis/SC_T_PREDMETU_R.zip) — target of `SC_T_PRAVNICH_VZT.TPR_KOD`

Runtime contracts and test environment:

- [Production WSDP 3.1 WSDL directory](https://katastr.cuzk.gov.cz/dokumentace/ws3_1/wsdp/)
- [Trial WSDP 3.1 WSDL directory](https://wsdptrial.cuzk.gov.cz/trial/dokumentace/ws3_1/wsdp/)
- [Trial shared XSD directory](https://wsdptrial.cuzk.gov.cz/trial/dokumentace/xsd/)
- [WSDP trial information, sample objects, and public trial account types](https://cuzk.gov.cz/Katastr-nemovitosti/Poskytovani-udaju-z-KN/Dalkovy-pristup/Webove-sluzby-DP-na-zkousku.aspx)
- [Monthly price-data XSD 1.0](https://services.cuzk.gov.cz/xsd/cenoveUdaje/1.0/cenoveUdaje.xsd)

Behavior, content, and charging:

- [Official output descriptions](https://cuzk.gov.cz/Katastr-nemovitosti/Poskytovani-udaju-z-KN/Dalkovy-pristup/Vystupy-z-KN-poskytovane-prostrednictvim-DP.aspx)
- [Official output charging rules](https://cuzk.gov.cz/Katastr-nemovitosti/Poskytovani-udaju-z-KN/Dalkovy-pristup/Uctovani-vystupu-z-KN-poskytovanych-DP-A-WSDP.aspx)
- [Official security/password rules](https://cuzk.gov.cz/Katastr-nemovitosti/Poskytovani-udaju-z-KN/Dalkovy-pristup/Bezpecnostni-opatreni-a-pravidla-pro-uzivani-prist.aspx)

The WSDL/XSD files bundled in the official 3.1 and NTR archives are the primary
wire contract. Where prose and XSD differ, preserve the discrepancy in
`SCHEMAS.md`, follow the actual wire schema, validate conservatively, and use
trial behavior rather than silently guessing.
