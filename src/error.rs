use std::fmt;

use crate::{MessageLevel, ServiceMessage};

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

    /// Credentials must be replaced after an ambiguous password change.
    #[error("credentials must be replaced explicitly before further requests")]
    CredentialsRequired,
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
