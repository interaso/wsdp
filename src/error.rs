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
#[derive(Clone, Debug, Eq, PartialEq)]
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
#[derive(Debug, thiserror::Error)]
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
    #[error("SOAP fault {code}: {message}")]
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

    /// Returns true for HTTP or WS-Security authentication failures.
    pub fn is_authentication_failure(&self) -> bool {
        matches!(self, Self::Authentication { .. })
            || matches!(self, Self::SoapFault { code, .. } if code.contains("Security"))
    }

    /// Returns true when WSDP reports a concurrent-user/load limit.
    pub fn is_busy(&self) -> bool {
        matches!(self, Self::Service(error) if error.messages().iter().any(|message| matches!(message.code, crate::MessageCode::ConcurrentUsersExceeded) || message.code.as_str() == "210"))
    }

    /// Returns true for a relevant record-not-found response.
    pub fn is_not_found(&self) -> bool {
        matches!(self, Self::Service(error) if error.messages().iter().any(|message| message.code == crate::MessageCode::RecordNotFound))
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
        matches!(self, Self::Service(error) if error.messages().iter().any(|message| message.code == crate::MessageCode::PasswordExpired))
    }
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
}
