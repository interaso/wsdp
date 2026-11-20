use std::fmt;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use bytes::Bytes;
use futures_util::StreamExt;
use reqwest::header::{ACCEPT, CONTENT_TYPE};
use secrecy::SecretString;
use tokio::sync::Semaphore;

use crate::error::{Error, Result};
use crate::model::{Response, ServiceKind, ServiceStatus, XmlDateTime};
use crate::wire::soap::{self, XmlLimits, XmlResult};

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
            }),
        })
    }
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

    fn status_response() -> &'static str {
        r#"<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/"><soap:Body><StavWSResponse xmlns="urn:cz:gov:cuzk:iskn:types:wsdp:ciselnik:3.1" xmlns:c="urn:cz:gov:cuzk:iskn:types:common:3.1"><vysledek><c:zprava kod="0" uroven="INFORMACE">ok</c:zprava></vysledek><stavSluzby><verze>3.1-test</verze><zpracovano>2026-11-20T10:11:12.123Z</zpracovano></stavSluzby></StavWSResponse></soap:Body></soap:Envelope>"#
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
