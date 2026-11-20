use std::fmt;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

use bytes::Bytes;
use futures_util::StreamExt;
use reqwest::header::{ACCEPT, CONTENT_TYPE};
use secrecy::SecretString;
use tokio::sync::Semaphore;

use crate::error::{Error, ReportStage, Result};
use crate::model::{
    AuthorizedSubject, Building, BuildingRight, BuildingRightSearch, BuildingSearch,
    ChargeApproval, CodeListRequest, CodeListResult, CollectionDocument, CollectionDocumentSearch,
    DownloadedReport, GenerateReport, GeometricPlan, GeometricPlanSearch, InformationRequest,
    InformationResponse, NewPassword, OwnershipSheetPreviewRequest, OwnershipSheetPreviewResponse,
    Parcel, ParcelBoundingBoxRequest, ParcelBoundingBoxResponse, ParcelSearch, Proceeding,
    ProceedingSearch, ReportBatch, ReportFamily, ReportId, ReportMetadata, ReportVersionInfo,
    Response, SearchRequest, SearchResponse, SearchResult, ServiceKind, ServiceStatus,
    SubjectSearch, Unit, UnitOverviewRequest, UnitOverviewResponse, UnitSearch, WaitOptions,
    XmlDateTime,
};
use crate::wire::soap::{self, XmlLimits, XmlResult};
use crate::wire::{account, code_list, information, mtom, report, search};

const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const DEFAULT_MAX_RESPONSE_BYTES: usize = 256 * 1024 * 1024;
const DEFAULT_MAX_XML_NODES: u32 = 2_000_000;
const DEFAULT_MAX_XML_DEPTH: u32 = 256;

/// ČÚZK WSDP deployment to which a client sends requests.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Environment {
    /// The free, public WSDP trial deployment.
    Trial,
    /// The paid production deployment. There is no implicit default.
    Production,
}

impl Environment {
    fn base_url(self) -> &'static str {
        match self {
            Self::Trial => "https://wsdptrial.cuzk.gov.cz/trial/ws/wsdp/3.1",
            Self::Production => "https://katastr.cuzk.gov.cz/ws/wsdp/3.1",
        }
    }
}

/// Username and password used for WSDP WS-Security authentication.
///
/// Both fields are redacted from `Debug` output.
#[derive(Clone)]
pub struct Credentials {
    username: String,
    pub(crate) password: SecretString,
}

impl Credentials {
    /// Creates WSDP credentials.
    pub fn new(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            username: username.into(),
            password: SecretString::new(password.into().into_boxed_str()),
        }
    }

    /// Returns the username for placement in a WS-Security token.
    pub fn username(&self) -> &str {
        &self.username
    }
}

impl fmt::Debug for Credentials {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Credentials")
            .field("username", &"[REDACTED]")
            .field("password", &"[REDACTED]")
            .finish()
    }
}

struct CredentialState {
    credentials: Credentials,
    replacement_required: bool,
}

struct ClientInner {
    http: reqwest::Client,
    credentials: RwLock<CredentialState>,
    environment: Environment,
    base_url: String,
    max_response_bytes: usize,
    xml_limits: XmlLimits,
    permits: Semaphore,
    generation_last: Mutex<Option<tokio::time::Instant>>,
}

/// A concurrent, asynchronous WSDP client.
///
/// Cloning is cheap. Clones share the connection pool, credentials, concurrency
/// limiter, and—when report generation is used—generation pacing state.
#[derive(Clone)]
pub struct Client {
    inner: Arc<ClientInner>,
}

impl fmt::Debug for Client {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Client")
            .field("environment", &self.inner.environment)
            .field("base_url", &self.inner.base_url)
            .field("max_response_bytes", &self.inner.max_response_bytes)
            .field("credentials", &"[REDACTED]")
            .finish()
    }
}

/// Builder for an asynchronous WSDP client.
pub struct ClientBuilder {
    environment: Environment,
    credentials: Option<Credentials>,
    connect_timeout: Duration,
    request_timeout: Duration,
    max_response_bytes: usize,
    max_xml_nodes: u32,
    max_xml_depth: u32,
    max_concurrency: usize,
    #[cfg(test)]
    test_base_url: Option<String>,
}

impl Client {
    /// Starts building a client for an explicitly selected environment.
    pub fn builder(environment: Environment) -> ClientBuilder {
        ClientBuilder {
            environment,
            credentials: None,
            connect_timeout: DEFAULT_CONNECT_TIMEOUT,
            request_timeout: DEFAULT_REQUEST_TIMEOUT,
            max_response_bytes: DEFAULT_MAX_RESPONSE_BYTES,
            max_xml_nodes: DEFAULT_MAX_XML_NODES,
            max_xml_depth: DEFAULT_MAX_XML_DEPTH,
            max_concurrency: 1,
            #[cfg(test)]
            test_base_url: None,
        }
    }

    /// Returns the explicitly selected environment.
    pub fn environment(&self) -> Environment {
        self.inner.environment
    }

    /// Replaces shared credentials after an ambiguous password-change outcome.
    ///
    /// No request is performed and neither old nor new passwords are probed.
    pub fn replace_credentials(&self, credentials: Credentials) {
        let mut state = self
            .inner
            .credentials
            .write()
            .unwrap_or_else(|e| e.into_inner());
        *state = CredentialState {
            credentials,
            replacement_required: false,
        };
    }

    /// Queries one WSDP service family's `stavWS` operation.
    ///
    /// This is a free, independently callable status request.
    pub async fn service_status(&self, service: ServiceKind) -> Result<Response<ServiceStatus>> {
        let operation = Operation::new(service, "stavWS", "StavWSRequest", "StavWSResponse");
        self.execute(
            &operation,
            |_writer| Ok(()),
            |response| {
                let status = soap::child_namespaces(
                    response,
                    &[service.namespace(), soap::COMMON_NS],
                    "stavSluzby",
                )?
                .ok_or_else(|| Error::invalid_response("service status container is missing"))?;
                let version = optional_text(status, service, "verze")?;
                let username = optional_text(status, service, "prihlasovaciJmeno")?;
                let processed_at = optional_text(status, service, "zpracovano")?
                    .map(|value| {
                        XmlDateTime::new(value).map_err(|_| {
                            Error::invalid_response("invalid service status timestamp")
                        })
                    })
                    .transpose()?;
                Ok(ServiceStatus {
                    version,
                    username,
                    processed_at,
                })
            },
        )
        .await
    }

    /// Executes any of the 33 typed WSDP 3.1 code-list operations.
    ///
    /// Filters are serialized exactly as supplied; `%` is neither added nor
    /// interpreted locally. There is no pagination or hidden follow-up request.
    pub async fn code_list(&self, request: CodeListRequest) -> Result<Response<CodeListResult>> {
        let operation = code_list::operation(&request);
        self.execute(
            &operation,
            |writer| code_list::write_request(writer, &request),
            |response| code_list::decode_response(response, &request),
        )
        .await
    }

    /// Executes one of the eight typed WSDP lookup operations.
    pub async fn search(&self, request: SearchRequest) -> Result<Response<SearchResponse>> {
        let operation = search::operation(&request);
        let mut response = self
            .execute(
                &operation,
                |writer| search::write_request(writer, &request),
                |node| search::decode_response(&request, node),
            )
            .await?;
        response.value.refine_completion(&response.messages);
        Ok(response)
    }

    /// Resolves parcels by internal ID or natural key.
    pub async fn find_parcels(
        &self,
        query: ParcelSearch,
    ) -> Result<Response<SearchResult<Parcel>>> {
        let response = self.search(SearchRequest::Parcel(query)).await?;
        map_operation_response(response, |value| match value {
            SearchResponse::Parcels(result) => Ok(result),
            _ => Err(Error::invalid_response("unexpected parcel-search result")),
        })
    }

    /// Resolves buildings by internal ID or natural key.
    pub async fn find_buildings(
        &self,
        query: BuildingSearch,
    ) -> Result<Response<SearchResult<Building>>> {
        let response = self.search(SearchRequest::Building(query)).await?;
        map_operation_response(response, |value| match value {
            SearchResponse::Buildings(result) => Ok(result),
            _ => Err(Error::invalid_response("unexpected building-search result")),
        })
    }

    /// Resolves units by internal ID or building/natural-key selector.
    pub async fn find_units(&self, query: UnitSearch) -> Result<Response<SearchResult<Unit>>> {
        let response = self.search(SearchRequest::Unit(query)).await?;
        map_operation_response(response, |value| match value {
            SearchResponse::Units(result) => Ok(result),
            _ => Err(Error::invalid_response("unexpected unit-search result")),
        })
    }

    /// Resolves legal or natural-person/partnership subjects.
    pub async fn find_subjects(
        &self,
        query: SubjectSearch,
    ) -> Result<Response<SearchResult<AuthorizedSubject>>> {
        let response = self.search(SearchRequest::Subject(query)).await?;
        map_operation_response(response, |value| match value {
            SearchResponse::Subjects(result) => Ok(result),
            _ => Err(Error::invalid_response("unexpected subject-search result")),
        })
    }

    /// Resolves building rights by ID, parcel ID, or parcel natural key.
    pub async fn find_building_rights(
        &self,
        query: BuildingRightSearch,
    ) -> Result<Response<SearchResult<BuildingRight>>> {
        let response = self.search(SearchRequest::BuildingRight(query)).await?;
        map_operation_response(response, |value| match value {
            SearchResponse::BuildingRights(result) => Ok(result),
            _ => Err(Error::invalid_response(
                "unexpected building-right-search result",
            )),
        })
    }

    /// Resolves proceedings by ID or complete natural key.
    pub async fn find_proceedings(
        &self,
        query: ProceedingSearch,
    ) -> Result<Response<SearchResult<Proceeding>>> {
        let response = self.search(SearchRequest::Proceeding(query)).await?;
        map_operation_response(response, |value| match value {
            SearchResponse::Proceedings(result) => Ok(result),
            _ => Err(Error::invalid_response(
                "unexpected proceeding-search result",
            )),
        })
    }

    /// Resolves deeds in the collection using the proceeding selector.
    pub async fn find_collection_documents(
        &self,
        query: CollectionDocumentSearch,
    ) -> Result<Response<SearchResult<CollectionDocument>>> {
        let response = self
            .search(SearchRequest::CollectionDocument(query))
            .await?;
        map_operation_response(response, |value| match value {
            SearchResponse::CollectionDocuments(result) => Ok(result),
            _ => Err(Error::invalid_response(
                "unexpected collection-document result",
            )),
        })
    }

    /// Resolves geometric-plan identifiers by cadastral area and ZPMZ number.
    pub async fn find_geometric_plans(
        &self,
        query: GeometricPlanSearch,
    ) -> Result<Response<SearchResult<GeometricPlan>>> {
        let response = self.search(SearchRequest::GeometricPlan(query)).await?;
        map_operation_response(response, |value| match value {
            SearchResponse::GeometricPlans(result) => Ok(result),
            _ => Err(Error::invalid_response("unexpected geometric-plan result")),
        })
    }

    /// Executes one of the three lightweight information operations.
    pub async fn information(
        &self,
        request: InformationRequest,
    ) -> Result<Response<InformationResponse>> {
        let operation = information::operation(&request);
        self.execute(
            &operation,
            |writer| information::write_request(writer, &request),
            |node| information::decode_response(&request, node),
        )
        .await
    }

    /// Returns parcel bounding boxes with explicit S-JTSK Y/X ordering.
    pub async fn parcel_bounding_boxes(
        &self,
        request: ParcelBoundingBoxRequest,
    ) -> Result<Response<ParcelBoundingBoxResponse>> {
        let response = self
            .information(InformationRequest::ParcelBoundingBoxes(request))
            .await?;
        map_operation_response(response, |value| match value {
            InformationResponse::ParcelBoundingBoxes(result) => Ok(result),
            _ => Err(Error::invalid_response("unexpected MBR information result")),
        })
    }

    /// Returns current unit information when the account is eligible.
    pub async fn unit_overview(
        &self,
        request: UnitOverviewRequest,
    ) -> Result<Response<UnitOverviewResponse>> {
        let response = self
            .information(InformationRequest::UnitOverview(request))
            .await?;
        map_operation_response(response, |value| match value {
            InformationResponse::UnitOverview(result) => Ok(*result),
            _ => Err(Error::invalid_response(
                "unexpected unit-information result",
            )),
        })
    }

    /// Returns the limited current LV preview when the account is eligible.
    pub async fn ownership_sheet_preview(
        &self,
        request: OwnershipSheetPreviewRequest,
    ) -> Result<Response<OwnershipSheetPreviewResponse>> {
        let response = self
            .information(InformationRequest::OwnershipSheetPreview(request))
            .await?;
        map_operation_response(response, |value| match value {
            InformationResponse::OwnershipSheetPreview(result) => Ok(result),
            _ => Err(Error::invalid_response("unexpected LV-preview result")),
        })
    }

    /// Discovers XML report versions currently enabled by the server.
    ///
    /// `None` omits the family filter. `Some(&[])` deliberately sends an empty
    /// filter. No version is inferred from schema comments.
    pub async fn report_versions(
        &self,
        families: Option<&[ReportFamily]>,
    ) -> Result<Response<Vec<ReportVersionInfo>>> {
        self.execute(
            &report::versions_operation(),
            |writer| report::write_versions(writer, families),
            report::decode_versions,
        )
        .await
    }

    /// Queues a report without waiting or downloading it.
    ///
    /// Generation is free. Calls across clones reserve start times at least two
    /// seconds apart, and no automatic retry is attempted after dispatch.
    pub async fn generate_report(&self, request: GenerateReport) -> Result<Response<ReportBatch>> {
        request.validate()?;
        self.reserve_generation_start().await;
        let operation = report::operation(&request);
        let result = self
            .execute(
                &operation,
                |writer| {
                    report::write_generate(writer, &request).map_err(|error| {
                        std::io::Error::new(std::io::ErrorKind::InvalidInput, error)
                    })
                },
                report::decode_report_batch,
            )
            .await;
        match result {
            Err(error @ Error::Service(_)) | Err(error @ Error::Authentication { .. }) => {
                Err(error)
            }
            Err(error @ Error::Transport(_)) if matches!(&error, Error::Transport(source) if source.is_connect()) => {
                Err(error)
            }
            Err(
                error @ (Error::InvalidInput { .. }
                | Error::Randomness
                | Error::CredentialsRequired),
            ) => Err(error),
            Err(error) => Err(Error::SubmissionUncertain {
                source: Box::new(error),
            }),
            Ok(response) => Ok(response),
        }
    }

    /// Lists all reports or one explicitly selected report using `seznamSestav`.
    ///
    /// This free operation never downloads report content and performs no hidden
    /// pagination or follow-up request.
    pub async fn list_reports(&self, id: Option<&ReportId>) -> Result<Response<ReportBatch>> {
        self.execute(
            &report::list_operation(),
            |writer| report::write_list(writer, id),
            report::decode_report_batch,
        )
        .await
    }

    /// Refreshes metadata for one report without downloading it.
    pub async fn report_status(&self, id: &ReportId) -> Result<Response<ReportMetadata>> {
        self.refresh_report(id)
            .await
            .map_err(|source| report_failure(id, ReportStage::Status, source))
    }

    async fn refresh_report(&self, id: &ReportId) -> Result<Response<ReportMetadata>> {
        let response = self
            .execute(
                &report::status_operation(),
                |writer| report::write_status(writer, id),
                report::decode_report_batch,
            )
            .await?;
        let metadata = response.value.into_single_report()?;
        if metadata.id() != id {
            return Err(Error::invalid_response(
                "report-status ID does not match the requested ID",
            ));
        }
        Ok(Response {
            value: metadata,
            messages: response.messages,
        })
    }

    /// Waits with bounded status polling only. Dropping the future cancels the
    /// wait and does not spawn background work.
    pub async fn wait_for_report(
        &self,
        id: &ReportId,
        options: WaitOptions,
    ) -> Result<Response<ReportMetadata>> {
        options
            .validate()
            .map_err(|source| report_failure(id, ReportStage::Wait, source))?;
        if options.max_wait.is_zero() {
            return Err(report_failure(
                id,
                ReportStage::Wait,
                Error::invalid_input("max_wait", "maximum wait must be greater than zero"),
            ));
        }
        let started = tokio::time::Instant::now();
        let deadline = started + options.max_wait;
        let mut next_poll = started + options.initial_delay;
        let mut interval = options.poll_interval;
        loop {
            if next_poll >= deadline {
                return Err(Error::ReportWaitTimeout { id: id.clone() });
            }
            tokio::time::sleep_until(next_poll).await;
            let response = self
                .refresh_report(id)
                .await
                .map_err(|source| report_failure(id, ReportStage::Wait, source))?;
            if response.value.state.is_ready() {
                return Ok(response);
            }
            if !response.value.state.is_pending() {
                return Err(Error::ReportUnavailable {
                    id: id.clone(),
                    stage: ReportStage::Wait,
                });
            }
            let now = tokio::time::Instant::now();
            if let Some(cap) = options.max_poll_interval {
                interval = interval.saturating_mul(2).min(cap);
            }
            next_poll = now + interval;
        }
    }

    /// Downloads one ready report after an explicit per-call charge approval.
    ///
    /// This performs one free status preflight and, only when its freshly returned
    /// price is accepted, one `vratSestavu` request. Pending, failed, missing, and
    /// unknown states are rejected before dispatch. Downloads are never retried.
    /// Post-dispatch HTTP, transport, size, SOAP/MTOM, and correlation failures
    /// return [`Error::ChargeUncertain`] with the report ID. An application-level
    /// `CHYBA` response is an authoritative rejection rather than an uncertain
    /// transport outcome. Inspect [`Client::report_status`] before any later
    /// explicit retrieval; never regenerate the report to recover from ambiguity.
    pub async fn download_report(
        &self,
        id: &ReportId,
        approval: ChargeApproval,
    ) -> Result<DownloadedReport> {
        let metadata = self
            .refresh_report(id)
            .await
            .map_err(|source| report_failure(id, ReportStage::Download, source))?
            .value;
        if !metadata.state.is_ready() {
            return Err(Error::ReportUnavailable {
                id: id.clone(),
                stage: ReportStage::Download,
            });
        }
        let Some(price) = metadata.price else {
            return Err(Error::ReportPriceUnavailable { id: id.clone() });
        };
        if !approval.accepts(Some(price)) {
            return Err(Error::ChargeLimitExceeded { id: id.clone() });
        }
        self.execute_download(id)
            .await
            .map_err(|error| match error {
                Error::Service(_) => report_failure(id, ReportStage::Download, error),
                Error::Transport(ref source) if source.is_connect() => {
                    report_failure(id, ReportStage::Download, error)
                }
                Error::InvalidInput { .. } | Error::Randomness | Error::CredentialsRequired => {
                    report_failure(id, ReportStage::Download, error)
                }
                source => Error::ChargeUncertain {
                    id: id.clone(),
                    source: Box::new(source),
                },
            })
    }

    async fn execute_download(&self, id: &ReportId) -> Result<DownloadedReport> {
        let permit = self
            .inner
            .permits
            .acquire()
            .await
            .map_err(|_| Error::invalid_input("concurrency", "request limiter is closed"))?;
        let credentials = {
            let state = self
                .inner
                .credentials
                .read()
                .unwrap_or_else(|error| error.into_inner());
            if state.replacement_required {
                return Err(Error::CredentialsRequired);
            }
            state.credentials.clone()
        };
        let operation = report::download_operation();
        let request = soap::request(
            operation.request,
            operation.service.namespace(),
            &credentials,
            |writer| report::write_download(writer, id),
        )?;
        let url = format!("{}/{}", self.inner.base_url, operation.service.path());
        let response = self
            .inner
            .http
            .post(url)
            .header(CONTENT_TYPE, "text/xml; charset=utf-8")
            .header(ACCEPT, "text/xml, multipart/related")
            .header("SOAPAction", format!("\"{}\"", operation.action))
            .body(request)
            .send()
            .await?;
        let status = response.status();
        if !status.is_success() {
            let source = if status == reqwest::StatusCode::UNAUTHORIZED
                || status == reqwest::StatusCode::FORBIDDEN
            {
                Error::Authentication {
                    status: status.as_u16(),
                }
            } else {
                Error::Http {
                    status: status.as_u16(),
                }
            };
            return Err(source);
        }
        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .ok_or_else(|| Error::invalid_response("report download Content-Type is missing"))?
            .to_str()
            .map_err(|_| Error::invalid_response("invalid report download Content-Type"))?
            .to_owned();
        let body = read_bounded(response, self.inner.max_response_bytes).await?;
        let report = mtom::decode_download(
            &body,
            &content_type,
            id,
            mtom::DownloadLimits {
                max_decoded_bytes: self.inner.max_response_bytes,
                xml: self.inner.xml_limits,
            },
        );
        drop(permit);
        tokio::task::yield_now().await;
        report
    }

    /// Explicitly deletes one report. This operation is never retried or called
    /// automatically by generation, waiting, or download helpers.
    pub async fn delete_report(&self, id: &ReportId) -> Result<Response<()>> {
        self.execute(
            &report::delete_operation(),
            |writer| report::write_delete(writer, id),
            report::decode_delete,
        )
        .await
        .map_err(|source| report_failure(id, ReportStage::Delete, source))
    }

    async fn reserve_generation_start(&self) {
        let scheduled = {
            let mut last = self
                .inner
                .generation_last
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            let now = tokio::time::Instant::now();
            let scheduled = last
                .map(|previous| (previous + Duration::from_secs(2)).max(now))
                .unwrap_or(now);
            *last = Some(scheduled);
            scheduled
        };
        tokio::time::sleep_until(scheduled).await;
    }

    /// Changes the account password once, without automatic retry.
    ///
    /// Confirmed success updates every clone. A definitive application or
    /// authentication rejection retains the existing credentials. Any ambiguous
    /// post-dispatch outcome blocks subsequent requests until
    /// [`Client::replace_credentials`] is called with confirmed working values.
    /// Never use this method with the shared public trial accounts.
    pub async fn change_password(&self, new_password: NewPassword) -> Result<Response<()>> {
        let result = self
            .execute(
                &account::change_password_operation(),
                |writer| account::write_change_password(writer, &new_password),
                |_response| Ok(()),
            )
            .await;
        match result {
            Ok(response) => {
                let mut state = self
                    .inner
                    .credentials
                    .write()
                    .unwrap_or_else(|error| error.into_inner());
                let username = state.credentials.username.clone();
                state.credentials = Credentials {
                    username,
                    password: new_password.0,
                };
                Ok(response)
            }
            Err(error @ Error::Service(_)) | Err(error @ Error::Authentication { .. }) => {
                Err(error)
            }
            Err(error @ Error::Transport(_)) if matches!(&error, Error::Transport(source) if source.is_connect()) => {
                Err(error)
            }
            Err(
                error @ (Error::InvalidInput { .. }
                | Error::Randomness
                | Error::CredentialsRequired),
            ) => Err(error),
            Err(error) => {
                let mut state = self
                    .inner
                    .credentials
                    .write()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                state.replacement_required = true;
                Err(Error::PasswordChangeUncertain {
                    source: Box::new(error),
                })
            }
        }
    }

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            name = "wsdp.request",
            level = "debug",
            skip_all,
            fields(operation = operation.request, endpoint = operation.service.path(), environment = ?self.inner.environment)
        )
    )]
    pub(crate) async fn execute<T>(
        &self,
        operation: &Operation,
        write_request: impl FnOnce(&mut quick_xml::Writer<Vec<u8>>) -> XmlResult,
        decode_response: impl FnOnce(roxmltree::Node<'_, '_>) -> Result<T>,
    ) -> Result<Response<T>> {
        let permit = self
            .inner
            .permits
            .acquire()
            .await
            .map_err(|_| Error::invalid_input("concurrency", "request limiter is closed"))?;
        let credentials = {
            let state = self
                .inner
                .credentials
                .read()
                .unwrap_or_else(|e| e.into_inner());
            if state.replacement_required {
                return Err(Error::CredentialsRequired);
            }
            state.credentials.clone()
        };
        let request = soap::request(
            operation.request,
            operation.service.namespace(),
            &credentials,
            write_request,
        )?;
        let url = format!("{}/{}", self.inner.base_url, operation.service.path());
        let response = self
            .inner
            .http
            .post(url)
            .header(CONTENT_TYPE, "text/xml; charset=utf-8")
            .header(ACCEPT, "text/xml, multipart/related")
            .header("SOAPAction", format!("\"{}\"", operation.action))
            .body(request)
            .send()
            .await?;
        let status = response.status();
        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            return Err(Error::Authentication {
                status: status.as_u16(),
            });
        }
        let body = read_bounded(response, self.inner.max_response_bytes).await?;
        #[cfg(feature = "tracing")]
        tracing::debug!(
            http_status = status.as_u16(),
            response_bytes = body.len(),
            "WSDP response received"
        );
        let decoded = soap::decode(
            &body,
            operation.service,
            operation.response,
            self.inner.xml_limits,
            decode_response,
        );
        let result = if status.is_success() {
            decoded?.into_result()
        } else {
            match decoded {
                Err(error @ (Error::SoapFault { .. } | Error::Service(_))) => Err(error),
                _ => Err(Error::Http {
                    status: status.as_u16(),
                }),
            }
        };
        drop(permit);
        tokio::task::yield_now().await;
        result
    }

    #[cfg(test)]
    pub(crate) fn test_client(base_url: String) -> Self {
        ClientBuilder {
            test_base_url: Some(base_url),
            ..Client::builder(Environment::Trial)
        }
        .credentials(Credentials::new("test-user", "sentinel-password"))
        .build()
        .unwrap()
    }
}

pub(crate) struct Operation {
    pub(crate) service: ServiceKind,
    pub(crate) request: &'static str,
    pub(crate) response: &'static str,
    pub(crate) action: String,
}

impl Operation {
    pub(crate) fn new(
        service: ServiceKind,
        operation: &str,
        request: &'static str,
        response: &'static str,
    ) -> Self {
        Self {
            service,
            request,
            response,
            action: service.soap_action(operation),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn with_action(mut self, action: impl Into<String>) -> Self {
        self.action = action.into();
        self
    }
}

impl ClientBuilder {
    /// Sets credentials used by every clone.
    pub fn credentials(mut self, credentials: Credentials) -> Self {
        self.credentials = Some(credentials);
        self
    }

    /// Sets the connection timeout; the default is 10 seconds.
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// Sets the total per-request timeout; the default is 60 seconds.
    pub fn request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = timeout;
        self
    }

    /// Sets the maximum buffered HTTP response size; the default is 256 MiB.
    pub fn max_response_bytes(mut self, limit: usize) -> Self {
        self.max_response_bytes = limit;
        self
    }

    /// Sets the maximum XML node count; the default is 2,000,000.
    pub fn max_xml_nodes(mut self, limit: u32) -> Self {
        self.max_xml_nodes = limit;
        self
    }

    /// Sets the maximum XML nesting depth; the default is 256.
    pub fn max_xml_depth(mut self, limit: u32) -> Self {
        self.max_xml_depth = limit;
        self
    }

    /// Sets the maximum number of in-flight requests per client; the default is 1.
    pub fn max_concurrency(mut self, limit: usize) -> Self {
        self.max_concurrency = limit;
        self
    }

    /// Builds the client without performing a network request.
    pub fn build(self) -> Result<Client> {
        let credentials = self
            .credentials
            .ok_or_else(|| Error::invalid_input("credentials", "credentials are required"))?;
        for (field, invalid) in [
            ("connect_timeout", self.connect_timeout.is_zero()),
            ("request_timeout", self.request_timeout.is_zero()),
            ("max_response_bytes", self.max_response_bytes == 0),
            ("max_xml_nodes", self.max_xml_nodes == 0),
            ("max_xml_depth", self.max_xml_depth == 0),
            ("max_concurrency", self.max_concurrency == 0),
        ] {
            if invalid {
                return Err(Error::invalid_input(
                    field,
                    "value must be greater than zero",
                ));
            }
        }

        #[cfg(test)]
        let (base_url, https_only) = match self.test_base_url {
            Some(url) => (url, false),
            None => (self.environment.base_url().to_owned(), true),
        };
        #[cfg(not(test))]
        let (base_url, https_only) = (self.environment.base_url().to_owned(), true);

        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .https_only(https_only)
            .connect_timeout(self.connect_timeout)
            .timeout(self.request_timeout)
            .user_agent(concat!("wsdp/", env!("CARGO_PKG_VERSION")))
            .build()?;

        Ok(Client {
            inner: Arc::new(ClientInner {
                http,
                credentials: RwLock::new(CredentialState {
                    credentials,
                    replacement_required: false,
                }),
                environment: self.environment,
                base_url: base_url.trim_end_matches('/').to_owned(),
                max_response_bytes: self.max_response_bytes,
                xml_limits: XmlLimits {
                    max_nodes: self.max_xml_nodes,
                    max_depth: self.max_xml_depth,
                },
                permits: Semaphore::new(self.max_concurrency),
                generation_last: Mutex::new(None),
            }),
        })
    }
}

fn report_failure(id: &ReportId, stage: ReportStage, source: Error) -> Error {
    Error::ReportOperation {
        id: id.clone(),
        stage,
        source: Box::new(source),
    }
}

fn map_operation_response<A, B>(
    response: Response<A>,
    map: impl FnOnce(A) -> Result<B>,
) -> Result<Response<B>> {
    Ok(Response {
        value: map(response.value)?,
        messages: response.messages,
    })
}

fn optional_text(
    node: roxmltree::Node<'_, '_>,
    service: ServiceKind,
    name: &str,
) -> Result<Option<String>> {
    let Some(element) =
        soap::child_namespaces(node, &[service.namespace(), soap::COMMON_NS], name)?
    else {
        return Ok(None);
    };
    if soap::is_nil(element) {
        return Ok(None);
    }
    soap::simple_content(element, name).map(Some)
}

async fn read_bounded(response: reqwest::Response, limit: usize) -> Result<Bytes> {
    if response
        .content_length()
        .is_some_and(|length| length > limit as u64)
    {
        return Err(Error::ResponseTooLarge { limit });
    }
    let mut stream = response.bytes_stream();
    let mut body = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        if body.len().saturating_add(chunk.len()) > limit {
            return Err(Error::ResponseTooLarge { limit });
        }
        body.extend_from_slice(&chunk);
    }
    Ok(Bytes::from(body))
}

#[cfg(test)]
mod tests {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

    fn report_response(id: &str, state: &str) -> String {
        priced_report_response(id, state, "0")
    }

    fn priced_report_response(id: &str, state: &str, price: &str) -> String {
        format!(
            r#"<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/"><soap:Body><GenerujSestavuResponse xmlns="urn:cz:gov:cuzk:iskn:types:wsdp:sestavy:3.1" xmlns:c="urn:cz:gov:cuzk:iskn:types:common:3.1"><vysledek><c:zprava kod="0" uroven="INFORMACE">ok</c:zprava></vysledek><reportList><report><id>{id}</id><nazev>Test report</nazev><cena>{price}</cena><datumPozadavku>2026-11-20T10:00:00Z</datumPozadavku><stav>{state}</stav><format>xml</format><verze>1.0</verze></report></reportList></GenerujSestavuResponse></soap:Body></soap:Envelope>"#
        )
    }

    fn download_response(id: &str, payload: &[u8]) -> String {
        use base64::Engine as _;
        let payload = base64::engine::general_purpose::STANDARD.encode(payload);
        let mut response = report_response(id, "zpracován");
        response = response.replacen(
            "</report>",
            &format!("<souborSestavy>{payload}</souborSestavy></report>"),
            1,
        );
        response.replacen("<stav>zpracován</stav>", "<stav>zaúčtován</stav>", 1)
    }

    fn status_response() -> &'static str {
        r#"<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/"><soap:Body><StavWSResponse xmlns="urn:cz:gov:cuzk:iskn:types:wsdp:ciselnik:3.1" xmlns:c="urn:cz:gov:cuzk:iskn:types:common:3.1"><vysledek><c:zprava kod="0" uroven="INFORMACE">ok</c:zprava></vysledek><stavSluzby><verze>3.1-test</verze><zpracovano>2026-11-20T10:11:12.123Z</zpracovano></stavSluzby></StavWSResponse></soap:Body></soap:Envelope>"#
    }

    struct PendingThenReady(std::sync::Arc<std::sync::atomic::AtomicUsize>);

    impl wiremock::Respond for PendingThenReady {
        fn respond(&self, _request: &wiremock::Request) -> ResponseTemplate {
            use std::sync::atomic::Ordering;
            let call = self.0.fetch_add(1, Ordering::SeqCst);
            let state = if call == 0 { "čeká" } else { "zpracován" };
            ResponseTemplate::new(200).set_body_string(report_response("123", state))
        }
    }

    #[tokio::test]
    async fn status_uses_the_exact_endpoint_action_and_security_header() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/ciselnik"))
            .and(|request: &wiremock::Request| {
                request.headers.contains_key("SOAPAction")
                    && String::from_utf8_lossy(&request.body).contains("wsse:UsernameToken")
            })
            .respond_with(ResponseTemplate::new(200).set_body_string(status_response()))
            .expect(1)
            .mount(&server)
            .await;
        let client = Client::test_client(server.uri());
        let response = client.service_status(ServiceKind::CodeLists).await.unwrap();
        assert_eq!(response.value.version.as_deref(), Some("3.1-test"));
        assert_eq!(
            response.value.processed_at.unwrap().as_str(),
            "2026-11-20T10:11:12.123Z"
        );
    }

    #[test]
    fn builder_validates_configuration_and_redacts_debug() {
        assert!(Client::builder(Environment::Trial).build().is_err());
        let client = Client::builder(Environment::Production)
            .credentials(Credentials::new("private-user", "private-password"))
            .build()
            .unwrap();
        let debug = format!("{client:?}");
        assert!(!debug.contains("private-user"));
        assert!(!debug.contains("private-password"));
        assert!(client.environment() == Environment::Production);

        for builder in [
            Client::builder(Environment::Trial)
                .credentials(Credentials::new("u", "p"))
                .max_response_bytes(0),
            Client::builder(Environment::Trial)
                .credentials(Credentials::new("u", "p"))
                .max_concurrency(0),
            Client::builder(Environment::Trial)
                .credentials(Credentials::new("u", "p"))
                .request_timeout(Duration::ZERO),
        ] {
            assert!(builder.build().is_err());
        }
    }

    #[tokio::test]
    async fn report_version_discovery_retains_family_and_versions() {
        let server = MockServer::start().await;
        let body = r#"<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/"><soap:Body><DejVerzeSestavResponse xmlns="urn:cz:gov:cuzk:iskn:types:wsdp:sestavy:3.1" xmlns:c="urn:cz:gov:cuzk:iskn:types:common:3.1"><vysledek><c:zprava kod="0" uroven="INFORMACE">ok</c:zprava></vysledek><verzeSestav><verzeSestavy><kodSestavy>VypisZKatastruNemovitosti</kodSestavy><nazevSestavy>LV</nazevSestavy><seznamVerzi><c:verze>1.0</c:verze><c:verze>3.0</c:verze></seznamVerzi></verzeSestavy></verzeSestav></DejVerzeSestavResponse></soap:Body></soap:Envelope>"#;
        Mock::given(method("POST"))
            .and(path("/sestavy"))
            .respond_with(ResponseTemplate::new(200).set_body_string(body))
            .expect(1)
            .mount(&server)
            .await;
        let client = Client::test_client(server.uri());
        let versions = client.report_versions(None).await.unwrap().value;
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].family, ReportFamily::OwnershipSheet);
        assert_eq!(
            versions[0].versions,
            vec![crate::ReportVersion::V1_0, crate::ReportVersion::V3_0]
        );
    }

    #[tokio::test]
    async fn report_status_correlates_the_requested_id() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/sestavy"))
            .respond_with(
                ResponseTemplate::new(200).set_body_string(report_response("999", "zpracován")),
            )
            .expect(1)
            .mount(&server)
            .await;
        let client = Client::test_client(server.uri());
        let id = ReportId::new("123").unwrap();
        let error = client.report_status(&id).await.unwrap_err();
        assert_eq!(error.report_id(), Some(&id));
        assert_eq!(error.report_stage(), Some(ReportStage::Status));
        assert!(matches!(
            error,
            Error::ReportOperation { source, .. }
                if matches!(*source, Error::InvalidResponse { .. })
        ));
    }

    #[tokio::test]
    async fn waiting_uses_only_status_until_ready() {
        let server = MockServer::start().await;
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        Mock::given(method("POST"))
            .and(path("/sestavy"))
            .respond_with(PendingThenReady(calls.clone()))
            .expect(2)
            .mount(&server)
            .await;
        let client = Client::test_client(server.uri());
        let response = client
            .wait_for_report(
                &ReportId::new("123").unwrap(),
                WaitOptions {
                    initial_delay: Duration::from_millis(1),
                    poll_interval: Duration::from_millis(1),
                    max_wait: Duration::from_secs(1),
                    max_poll_interval: None,
                },
            )
            .await
            .unwrap();
        assert_eq!(response.value.state, crate::ReportState::Processed);
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn generation_pacing_is_shared_across_clones() {
        use crate::{AccountContext, ParcelId, ParcelInfoRequest, ParcelSelector, ReportOptions};

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/sestavy"))
            .respond_with(
                ResponseTemplate::new(200).set_body_string(report_response("123", "čeká")),
            )
            .expect(2)
            .mount(&server)
            .await;
        let client = Client::test_client(server.uri());
        let request = || {
            GenerateReport::ParcelInfo(ParcelInfoRequest::new(
                ParcelSelector::Id(ParcelId::new("123").unwrap()),
                ReportOptions::xml_default(AccountContext::Paying),
            ))
        };
        client.generate_report(request()).await.unwrap();
        let first = *client.inner.generation_last.lock().unwrap();
        client.clone().generate_report(request()).await.unwrap();
        let second = *client.inner.generation_last.lock().unwrap();
        assert_eq!(second.unwrap() - first.unwrap(), Duration::from_secs(2));
    }

    #[tokio::test]
    async fn approved_download_preflights_price_and_preserves_bytes() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/sestavy"))
            .and(|request: &wiremock::Request| {
                String::from_utf8_lossy(&request.body).contains("SeznamSestavRequest")
            })
            .respond_with(
                ResponseTemplate::new(200).set_body_string(report_response("123", "zpracován")),
            )
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/sestavy"))
            .and(|request: &wiremock::Request| {
                String::from_utf8_lossy(&request.body).contains("VratSestavuRequest")
            })
            .respond_with(ResponseTemplate::new(200).set_body_raw(
                download_response("123", b"original\x00report"),
                "text/xml; charset=utf-8",
            ))
            .expect(1)
            .mount(&server)
            .await;
        let client = Client::test_client(server.uri());
        let id = ReportId::new("123").unwrap();
        let report = client
            .download_report(&id, ChargeApproval::FreeOnly)
            .await
            .unwrap();
        assert_eq!(report.bytes, Bytes::from_static(b"original\x00report"));
        assert_eq!(report.metadata.state, crate::ReportState::Charged);
        assert!(!format!("{report:?}").contains("original"));
    }

    #[tokio::test]
    async fn declined_price_never_dispatches_download() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/sestavy"))
            .and(|request: &wiremock::Request| {
                String::from_utf8_lossy(&request.body).contains("SeznamSestavRequest")
            })
            .respond_with(
                ResponseTemplate::new(200).set_body_string(priced_report_response(
                    "123",
                    "zpracován",
                    "10.01",
                )),
            )
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/sestavy"))
            .and(|request: &wiremock::Request| {
                String::from_utf8_lossy(&request.body).contains("VratSestavuRequest")
            })
            .respond_with(ResponseTemplate::new(500))
            .expect(0)
            .mount(&server)
            .await;
        let client = Client::test_client(server.uri());
        let id = ReportId::new("123").unwrap();
        let error = client
            .download_report(&id, ChargeApproval::up_to("10.00".parse().unwrap()))
            .await
            .unwrap_err();
        assert!(matches!(error, Error::ChargeLimitExceeded { .. }));
        assert_eq!(error.report_id(), Some(&id));
        assert_eq!(error.report_stage(), Some(ReportStage::Download));
        assert!(!error.charge_outcome_uncertain());
    }

    #[tokio::test]
    async fn malformed_dispatched_download_is_charge_uncertain() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/sestavy"))
            .and(|request: &wiremock::Request| {
                String::from_utf8_lossy(&request.body).contains("SeznamSestavRequest")
            })
            .respond_with(
                ResponseTemplate::new(200).set_body_string(report_response("123", "zpracován")),
            )
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/sestavy"))
            .and(|request: &wiremock::Request| {
                String::from_utf8_lossy(&request.body).contains("VratSestavuRequest")
            })
            .respond_with(ResponseTemplate::new(200).set_body_raw("not XML", "text/xml"))
            .expect(1)
            .mount(&server)
            .await;
        let client = Client::test_client(server.uri());
        let id = ReportId::new("123").unwrap();
        let error = client
            .download_report(&id, ChargeApproval::FreeOnly)
            .await
            .unwrap_err();
        assert!(error.charge_outcome_uncertain());
        assert_eq!(error.report_id(), Some(&id));
        assert_eq!(error.report_stage(), Some(ReportStage::Download));
    }

    #[tokio::test]
    async fn confirmed_password_change_updates_all_clones() {
        use secrecy::ExposeSecret as _;

        let server = MockServer::start().await;
        let body = r#"<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/"><soap:Body><ZmenHesloResponse xmlns="urn:cz:gov:cuzk:iskn:types:wsdp:ucet:3.1" xmlns:c="urn:cz:gov:cuzk:iskn:types:common:3.1"><vysledek><c:zprava kod="0" uroven="INFORMACE">ok</c:zprava></vysledek></ZmenHesloResponse></soap:Body></soap:Envelope>"#;
        Mock::given(method("POST"))
            .and(path("/ucet"))
            .respond_with(ResponseTemplate::new(200).set_body_string(body))
            .expect(1)
            .mount(&server)
            .await;
        let client = Client::test_client(server.uri());
        let clone = client.clone();
        client
            .change_password(NewPassword::new("new-sentinel").unwrap())
            .await
            .unwrap();
        let state = clone.inner.credentials.read().unwrap();
        assert_eq!(state.credentials.password.expose_secret(), "new-sentinel");
        assert!(!state.replacement_required);
    }

    #[tokio::test]
    async fn ambiguous_password_change_blocks_clones_until_replacement() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/ucet"))
            .respond_with(ResponseTemplate::new(200).set_body_string("not XML"))
            .expect(1)
            .mount(&server)
            .await;
        let client = Client::test_client(server.uri());
        let clone = client.clone();
        let error = client
            .change_password(NewPassword::new("new-sentinel").unwrap())
            .await
            .unwrap_err();
        assert!(error.password_change_outcome_uncertain());
        assert!(matches!(
            clone.service_status(ServiceKind::Account).await,
            Err(Error::CredentialsRequired)
        ));
        clone.replace_credentials(Credentials::new("confirmed", "confirmed-password"));
        assert!(!clone.inner.credentials.read().unwrap().replacement_required);
    }

    #[tokio::test]
    async fn redirects_are_not_followed() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/ciselnik"))
            .respond_with(ResponseTemplate::new(302).insert_header("Location", "/elsewhere"))
            .expect(1)
            .mount(&server)
            .await;
        let client = Client::test_client(server.uri());
        assert!(matches!(
            client.service_status(ServiceKind::CodeLists).await,
            Err(Error::Http { status: 302 })
        ));
    }
}
