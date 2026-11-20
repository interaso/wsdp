use std::fmt;

use crate::{MessageLevel, ServiceMessage};

/// Lifecycle stage associated with a report-scoped failure.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum ReportStage {
    /// Status/list refresh.
    Status,
    /// Bounded waiting.
    Wait,
    /// Approved download.
    Download,
    /// Typed payload decoding.
    Decode,
    /// Explicit deletion.
    Delete,
}

impl fmt::Display for ReportStage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Status => "status",
            Self::Wait => "wait",
            Self::Download => "download",
            Self::Decode => "decode",
            Self::Delete => "delete",
        })
    }
}

/// An application-level WSDP error containing the complete server message list.
#[derive(Clone, Eq, PartialEq)]
pub struct ServiceError {
    messages: Vec<ServiceMessage>,
}

impl ServiceError {
    pub(crate) fn new(messages: Vec<ServiceMessage>) -> Self {
        Self { messages }
    }

    /// Returns every message supplied by the service, including non-errors.
    pub fn messages(&self) -> &[ServiceMessage] {
        &self.messages
    }

    /// Returns the first application-error code, if present.
    pub fn code(&self) -> Option<&str> {
        self.messages
            .iter()
            .find(|message| message.level == MessageLevel::Error)
            .map(|message| message.code.as_str())
    }
}

impl fmt::Debug for ServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ServiceError")
            .field("message_count", &self.messages.len())
            .finish_non_exhaustive()
    }
}

impl fmt::Display for ServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.code() {
            Some(code) => write!(
                formatter,
                "WSDP rejected the operation with message code {code}"
            ),
            None => formatter.write_str("WSDP rejected the operation"),
        }
    }
}

impl std::error::Error for ServiceError {}

/// Errors produced while configuring or calling the WSDP client.
#[derive(thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// An input, identifier, or client configuration value is invalid.
    #[error("invalid input for {field}: {message}")]
    InvalidInput {
        /// Name of the invalid field.
        field: &'static str,
        /// Description of the violated invariant.
        message: String,
    },

    /// An HTTP request failed in transport.
    #[error("WSDP transport failed: {0}")]
    Transport(#[from] reqwest::Error),

    /// The server rejected authentication or authorization at HTTP level.
    #[error("WSDP authentication failed with HTTP status {status}")]
    Authentication {
        /// Numeric HTTP status code.
        status: u16,
    },

    /// The server returned an HTTP error without a usable SOAP fault.
    #[error("WSDP returned HTTP status {status}")]
    Http {
        /// Numeric HTTP status code.
        status: u16,
    },

    /// The SOAP endpoint returned a fault.
    #[error("SOAP fault {code}")]
    SoapFault {
        /// SOAP fault code.
        code: String,
        /// SOAP fault text.
        message: String,
    },

    /// WSDP returned an application `CHYBA` message.
    #[error("{0}")]
    Service(#[from] ServiceError),

    /// An exact decimal cannot be represented without rounding or truncation.
    #[error("decimal value for {field} exceeds the supported exact range")]
    DecimalOutOfRange {
        /// Safe field or schema-type name; the sensitive raw value is omitted.
        field: &'static str,
    },

    /// A response exceeded its configured buffered-byte limit.
    #[error("WSDP response exceeded the configured {limit}-byte limit")]
    ResponseTooLarge {
        /// Maximum permitted response size.
        limit: usize,
    },

    /// A response was malformed or did not match the requested operation.
    #[error("invalid WSDP response: {message}")]
    InvalidResponse {
        /// Safe description that never includes the response body.
        message: String,
    },

    /// Secure randomness needed for WS-Security could not be obtained.
    #[error("could not initialize WS-Security randomness")]
    Randomness,

    /// Report generation was dispatched but its acceptance cannot be established.
    #[error("report-generation outcome is uncertain: {source}")]
    SubmissionUncertain {
        /// Safe underlying transport, HTTP, or decoding error.
        source: Box<Error>,
    },

    /// A potentially charging report download has an ambiguous outcome.
    #[error("report download has an uncertain charging outcome: {source}")]
    ChargeUncertain {
        /// Report ID that can be reconciled through `report_status`.
        id: crate::ReportId,
        /// Safe underlying transport, HTTP, MTOM, or decoding error.
        source: Box<Error>,
    },

    /// Bounded report waiting elapsed before a ready state.
    #[error("report wait deadline elapsed")]
    ReportWaitTimeout {
        /// Report ID to resume later without regeneration.
        id: crate::ReportId,
    },

    /// A report is missing, not ready, failed, cancelled, or otherwise unavailable.
    #[error("report is not available for the requested {stage} operation")]
    ReportUnavailable {
        /// Affected report ID.
        id: crate::ReportId,
        /// Lifecycle stage that found the report unavailable.
        stage: ReportStage,
    },

    /// A report-scoped operation failed while retaining its resumable ID.
    #[error("report {stage} operation failed: {source}")]
    ReportOperation {
        /// Report ID to reuse for recovery; it is intentionally not displayed.
        id: crate::ReportId,
        /// Lifecycle stage that failed.
        stage: ReportStage,
        /// Safe underlying error.
        source: Box<Error>,
    },

    /// A workflow documented to produce one report returned zero or multiple outputs.
    #[error("expected one report, received {}", ids.len())]
    UnexpectedReportBatch {
        /// Every known report ID, retained in server order.
        ids: Vec<crate::ReportId>,
    },

    /// The server did not provide the current price needed for approval.
    #[error("report price is unavailable")]
    ReportPriceUnavailable {
        /// Affected report ID.
        id: crate::ReportId,
    },

    /// The exact server price exceeds the caller's approval.
    #[error("report price exceeds the explicit charge approval")]
    ChargeLimitExceeded {
        /// Affected report ID.
        id: crate::ReportId,
    },

    /// Credentials must be replaced after an ambiguous password change.
    #[error("credentials must be replaced explicitly before further requests")]
    CredentialsRequired,

    /// A password change was dispatched but its outcome cannot be established.
    #[error("password-change outcome is uncertain: {source}")]
    PasswordChangeUncertain {
        /// Safe underlying transport, HTTP, or decoding error.
        source: Box<Error>,
    },
}

impl fmt::Debug for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput { field, message } => formatter
                .debug_struct("InvalidInput")
                .field("field", field)
                .field("message", message)
                .finish(),
            Self::Transport(_) => formatter.write_str("Transport([REDACTED])"),
            Self::Authentication { status } => formatter
                .debug_struct("Authentication")
                .field("status", status)
                .finish(),
            Self::Http { status } => formatter
                .debug_struct("Http")
                .field("status", status)
                .finish(),
            Self::SoapFault { code, .. } => formatter
                .debug_struct("SoapFault")
                .field("code", code)
                .field("message", &"[REDACTED]")
                .finish(),
            Self::Service(error) => fmt::Debug::fmt(error, formatter),
            Self::DecimalOutOfRange { field } => formatter
                .debug_struct("DecimalOutOfRange")
                .field("field", field)
                .finish(),
            Self::ResponseTooLarge { limit } => formatter
                .debug_struct("ResponseTooLarge")
                .field("limit", limit)
                .finish(),
            Self::InvalidResponse { message } => formatter
                .debug_struct("InvalidResponse")
                .field("message", message)
                .finish(),
            Self::Randomness => formatter.write_str("Randomness"),
            Self::SubmissionUncertain { source } => formatter
                .debug_struct("SubmissionUncertain")
                .field("source", source)
                .finish(),
            Self::ChargeUncertain { id, source } => formatter
                .debug_struct("ChargeUncertain")
                .field("id", id)
                .field("source", source)
                .finish(),
            Self::ReportWaitTimeout { id } => formatter
                .debug_struct("ReportWaitTimeout")
                .field("id", id)
                .finish(),
            Self::ReportUnavailable { id, stage } => formatter
                .debug_struct("ReportUnavailable")
                .field("id", id)
                .field("stage", stage)
                .finish(),
            Self::ReportOperation { id, stage, source } => formatter
                .debug_struct("ReportOperation")
                .field("id", id)
                .field("stage", stage)
                .field("source", source)
                .finish(),
            Self::UnexpectedReportBatch { ids } => formatter
                .debug_struct("UnexpectedReportBatch")
                .field("report_count", &ids.len())
                .finish_non_exhaustive(),
            Self::ReportPriceUnavailable { id } => formatter
                .debug_struct("ReportPriceUnavailable")
                .field("id", id)
                .finish(),
            Self::ChargeLimitExceeded { id } => formatter
                .debug_struct("ChargeLimitExceeded")
                .field("id", id)
                .finish(),
            Self::CredentialsRequired => formatter.write_str("CredentialsRequired"),
            Self::PasswordChangeUncertain { source } => formatter
                .debug_struct("PasswordChangeUncertain")
                .field("source", source)
                .finish(),
        }
    }
}

impl Error {
    pub(crate) fn invalid_input(field: &'static str, message: impl Into<String>) -> Self {
        Self::InvalidInput {
            field,
            message: message.into(),
        }
    }

    pub(crate) fn invalid_response(message: impl Into<String>) -> Self {
        Self::InvalidResponse {
            message: message.into(),
        }
    }

    /// Returns true for HTTP or WS-Security authentication failures, including
    /// failures retained inside a report/password uncertainty wrapper.
    pub fn is_authentication_failure(&self) -> bool {
        match self {
            Self::Authentication { .. } => true,
            Self::SoapFault { code, .. } => {
                let code = code.trim_matches([' ', '\t', '\r', '\n']);
                let local_name = code.rsplit(':').next().unwrap_or(code);
                matches!(
                    local_name,
                    "FailedAuthentication"
                        | "FailedCheck"
                        | "InvalidSecurity"
                        | "InvalidSecurityToken"
                        | "SecurityTokenUnavailable"
                        | "UnsupportedSecurityToken"
                )
            }
            Self::ReportOperation { source, .. }
            | Self::ChargeUncertain { source, .. }
            | Self::SubmissionUncertain { source }
            | Self::PasswordChangeUncertain { source } => source.is_authentication_failure(),
            _ => false,
        }
    }

    fn service_error(&self) -> Option<&ServiceError> {
        match self {
            Self::Service(error) => Some(error),
            Self::ReportOperation { source, .. }
            | Self::ChargeUncertain { source, .. }
            | Self::SubmissionUncertain { source }
            | Self::PasswordChangeUncertain { source } => source.service_error(),
            _ => None,
        }
    }

    /// Returns true when WSDP reports a concurrent-user/load limit.
    pub fn is_busy(&self) -> bool {
        self.service_error().is_some_and(|error| {
            error
                .messages()
                .iter()
                .any(|message| message_code_is(message, "477") || message_code_is(message, "210"))
        })
    }

    /// Returns true for a relevant record-not-found response.
    pub fn is_not_found(&self) -> bool {
        self.service_error().is_some_and(|error| {
            error
                .messages()
                .iter()
                .any(|message| message_code_is(message, "304"))
        })
    }

    /// Returns a report ID retained by a lifecycle error, when known.
    pub fn report_id(&self) -> Option<&crate::ReportId> {
        match self {
            Self::ChargeUncertain { id, .. }
            | Self::ReportWaitTimeout { id }
            | Self::ReportUnavailable { id, .. }
            | Self::ReportPriceUnavailable { id }
            | Self::ChargeLimitExceeded { id }
            | Self::ReportOperation { id, .. } => Some(id),
            _ => None,
        }
    }

    /// Returns every report ID retained by an error, including all IDs from an
    /// unexpected multi-report result. An empty slice means zero reports returned.
    pub fn report_ids(&self) -> Option<&[crate::ReportId]> {
        match self {
            Self::UnexpectedReportBatch { ids } => Some(ids),
            _ => self.report_id().map(std::slice::from_ref),
        }
    }

    /// Returns the lifecycle stage retained by a report-scoped error.
    pub fn report_stage(&self) -> Option<ReportStage> {
        match self {
            Self::ReportOperation { stage, .. } | Self::ReportUnavailable { stage, .. } => {
                Some(*stage)
            }
            Self::ReportWaitTimeout { .. } => Some(ReportStage::Wait),
            Self::ChargeUncertain { .. }
            | Self::ReportPriceUnavailable { .. }
            | Self::ChargeLimitExceeded { .. } => Some(ReportStage::Download),
            _ => None,
        }
    }

    /// Returns true when dispatched report generation has an ambiguous outcome.
    pub fn submission_outcome_uncertain(&self) -> bool {
        matches!(self, Self::SubmissionUncertain { .. })
    }

    /// Returns true when a dispatched download has an ambiguous charging outcome.
    pub fn charge_outcome_uncertain(&self) -> bool {
        matches!(self, Self::ChargeUncertain { .. })
    }

    /// Returns true when a dispatched password change has an ambiguous outcome.
    pub fn password_change_outcome_uncertain(&self) -> bool {
        matches!(self, Self::PasswordChangeUncertain { .. })
    }

    /// Returns true when the server reports password expiration.
    pub fn is_password_expired(&self) -> bool {
        self.service_error().is_some_and(|error| {
            error
                .messages()
                .iter()
                .any(|message| message_code_is(message, "205"))
        })
    }
}

fn message_code_is(message: &ServiceMessage, expected: &str) -> bool {
    let raw = message.code.as_str().trim_matches([' ', '\t', '\r', '\n']);
    let digits = raw.strip_prefix('+').unwrap_or(raw);
    !digits.is_empty()
        && digits.bytes().all(|byte| byte.is_ascii_digit())
        && digits.trim_start_matches('0') == expected.trim_start_matches('0')
}

impl From<getrandom::Error> for Error {
    fn from(_: getrandom::Error) -> Self {
        Self::Randomness
    }
}

/// Result type used by this crate.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MessageCode;

    #[test]
    fn service_error_retrieves_codes_without_leaking_message_text_in_display() {
        let error = ServiceError::new(vec![ServiceMessage {
            code: MessageCode::ResultLimitExceeded,
            level: MessageLevel::Error,
            text: "sensitive server detail".to_owned(),
        }]);
        assert_eq!(error.code(), Some("310"));
        assert!(!error.to_string().contains("sensitive"));
        assert_eq!(error.messages().len(), 1);
    }

    #[test]
    fn classifiers_inspect_lifecycle_and_uncertainty_sources() {
        let service = |code| {
            Error::Service(ServiceError::new(vec![ServiceMessage {
                code: MessageCode::from_raw(code),
                level: MessageLevel::Error,
                text: "sensitive".to_owned(),
            }]))
        };
        let id = crate::ReportId::new("123").unwrap();
        let wrapped = |source| Error::ReportOperation {
            id: id.clone(),
            stage: ReportStage::Status,
            source: Box::new(source),
        };
        assert!(wrapped(service("304")).is_not_found());
        assert!(wrapped(service("  +000304 ")).is_not_found());
        assert!(wrapped(service("477")).is_busy());
        assert!(wrapped(service("210")).is_busy());
        assert!(wrapped(service("+000210")).is_busy());
        assert!(wrapped(service("205")).is_password_expired());
        assert!(wrapped(service("000205")).is_password_expired());
        assert!(!wrapped(service("0")).is_not_found());

        let charge = Error::ChargeUncertain {
            id,
            source: Box::new(Error::Authentication { status: 401 }),
        };
        assert!(charge.is_authentication_failure());
        for code in [
            "wsse:InvalidSecurity",
            "wsse:FailedAuthentication",
            " \twsse:FailedAuthentication\r\n",
        ] {
            assert!(
                Error::SoapFault {
                    code: code.to_owned(),
                    message: "redacted".to_owned(),
                }
                .is_authentication_failure()
            );
        }
    }

    #[test]
    fn debug_and_display_do_not_expose_server_message_or_fault_text() {
        let service = ServiceError::new(vec![ServiceMessage {
            code: MessageCode::RecordNotFound,
            level: MessageLevel::Error,
            text: "private service detail".to_owned(),
        }]);
        let error = Error::Service(service);
        assert!(!format!("{error:?}").contains("private service detail"));
        assert!(!error.to_string().contains("private service detail"));

        let fault = Error::SoapFault {
            code: "wsse:FailedAuthentication".to_owned(),
            message: "private fault detail".to_owned(),
        };
        assert!(!format!("{fault:?}").contains("private fault detail"));
        assert!(!fault.to_string().contains("private fault detail"));
        assert_eq!(fault.to_string(), "SOAP fault wsse:FailedAuthentication");
    }
}
