use std::fmt;
use std::str::FromStr;

use rust_decimal::Decimal;

/// A WSDP service family whose status can be queried independently.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ServiceKind {
    /// Code-list operations (`ciselnik`).
    CodeLists,
    /// Object and subject lookup (`vyhledat`).
    Search,
    /// Lightweight information views (`informace`).
    Information,
    /// Asynchronous reports (`sestavy`).
    Reports,
    /// Account operations (`ucet`).
    Account,
}

impl ServiceKind {
    pub(crate) fn path(self) -> &'static str {
        match self {
            Self::CodeLists => "ciselnik",
            Self::Search => "vyhledat",
            Self::Information => "informace",
            Self::Reports => "sestavy",
            Self::Account => "ucet",
        }
    }

    pub(crate) fn namespace(self) -> &'static str {
        match self {
            Self::CodeLists => "urn:cz:gov:cuzk:iskn:types:wsdp:ciselnik:3.1",
            Self::Search => "urn:cz:gov:cuzk:iskn:types:wsdp:vyhledat:3.1",
            Self::Information => "urn:cz:gov:cuzk:iskn:types:wsdp:informace:3.1",
            Self::Reports => "urn:cz:gov:cuzk:iskn:types:wsdp:sestavy:3.1",
            Self::Account => "urn:cz:gov:cuzk:iskn:types:wsdp:ucet:3.1",
        }
    }

    pub(crate) fn soap_action(self, operation: &str) -> String {
        format!(
            "urn:cz:gov:cuzk:iskn:services:wsdp:{}:{operation}",
            self.path()
        )
    }
}

/// A successful WSDP value together with every non-error service message.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Response<T> {
    /// Operation-specific returned value.
    pub value: T,
    /// Informational, warning, operational, and unknown messages.
    pub messages: Vec<ServiceMessage>,
}

/// A message returned in the WSDP `vysledek` container.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServiceMessage {
    /// Typed known code or the untouched lexical value.
    pub code: MessageCode,
    /// Message severity.
    pub level: MessageLevel,
    /// Human-readable server text.
    pub text: String,
}

/// A WSDP message code with raw preservation for future values.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum MessageCode {
    /// Successful operation (`0`).
    Success,
    /// Password expiration (`205`).
    PasswordExpired,
    /// Record not found (`304`).
    RecordNotFound,
    /// Result limit exceeded (`310`).
    ResultLimitExceeded,
    /// No matching data (`392`).
    NoData,
    /// Concurrent-user limit exceeded (`477`).
    ConcurrentUsersExceeded,
    /// A code unknown to this crate version.
    Other(String),
}

impl MessageCode {
    pub(crate) fn from_raw(raw: impl Into<String>) -> Self {
        let raw = raw.into();
        match raw.as_str() {
            "0" => Self::Success,
            "205" => Self::PasswordExpired,
            "304" => Self::RecordNotFound,
            "310" => Self::ResultLimitExceeded,
            "392" => Self::NoData,
            "477" => Self::ConcurrentUsersExceeded,
            _ => Self::Other(raw),
        }
    }

    /// Returns the official or untouched unknown lexical code.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Success => "0",
            Self::PasswordExpired => "205",
            Self::RecordNotFound => "304",
            Self::ResultLimitExceeded => "310",
            Self::NoData => "392",
            Self::ConcurrentUsersExceeded => "477",
            Self::Other(raw) => raw,
        }
    }
}

impl fmt::Display for MessageCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Severity of a WSDP service message.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum MessageLevel {
    /// Informational message (`INFORMACE`).
    Information,
    /// Warning (`VAROVANI`).
    Warning,
    /// Application error (`CHYBA`).
    Error,
    /// An unknown severity, preserved lexically.
    Other(String),
}

impl MessageLevel {
    pub(crate) fn from_raw(raw: impl Into<String>) -> Self {
        let raw = raw.into();
        match raw.as_str() {
            "INFORMACE" => Self::Information,
            "VAROVANI" => Self::Warning,
            "CHYBA" => Self::Error,
            _ => Self::Other(raw),
        }
    }

    /// Returns the official or untouched unknown lexical level.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Information => "INFORMACE",
            Self::Warning => "VAROVANI",
            Self::Error => "CHYBA",
            Self::Other(raw) => raw,
        }
    }
}

/// An XML date-time preserving its original offset and fractional seconds.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct XmlDateTime(String);

impl XmlDateTime {
    /// Validates and retains an XML date-time without assigning a timezone to a
    /// naive value.
    pub fn new(value: impl Into<String>) -> crate::Result<Self> {
        let value = value.into();
        if value.parse::<jiff::Timestamp>().is_ok()
            || value.parse::<jiff::civil::DateTime>().is_ok()
        {
            Ok(Self(value))
        } else {
            Err(crate::Error::invalid_input(
                "date_time",
                "expected an XML date-time with an optional offset",
            ))
        }
    }

    /// Returns the original lexical representation.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for XmlDateTime {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for XmlDateTime {
    type Err = crate::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::new(value)
    }
}

/// Service health information returned by `stavWS`.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct ServiceStatus {
    /// Server implementation version, when supplied.
    pub version: Option<String>,
    /// Authenticated login name, when supplied by the server.
    pub username: Option<String>,
    /// Server processing timestamp with its original offset semantics.
    pub processed_at: Option<XmlDateTime>,
}

pub(crate) fn parse_decimal(raw: &str, field: &'static str) -> crate::Result<Decimal> {
    let bytes = raw.as_bytes();
    let mut index = usize::from(matches!(bytes.first(), Some(b'+' | b'-')));
    let mut digits = 0_usize;
    let mut decimal_point = false;
    while index < bytes.len() {
        match bytes[index] {
            b'0'..=b'9' => digits += 1,
            b'.' if !decimal_point => decimal_point = true,
            _ => {
                return Err(crate::Error::invalid_input(
                    "decimal",
                    "expected XML decimal lexical syntax",
                ));
            }
        }
        index += 1;
    }
    if digits == 0 {
        return Err(crate::Error::invalid_input(
            "decimal",
            "XML decimal requires at least one digit",
        ));
    }
    Decimal::from_str_exact(raw).map_err(|error| match error {
        rust_decimal::Error::ExceedsMaximumPossibleValue
        | rust_decimal::Error::LessThanMinimumPossibleValue
        | rust_decimal::Error::Underflow
        | rust_decimal::Error::ScaleExceedsMaximumPrecision(_) => {
            crate::Error::DecimalOutOfRange { field }
        }
        rust_decimal::Error::ErrorString(message) if message.contains("overflow") => {
            crate::Error::DecimalOutOfRange { field }
        }
        _ => crate::Error::invalid_input("decimal", "invalid XML decimal value"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_codes_preserve_known_and_unknown_lexical_values() {
        for (raw, expected) in [
            ("0", MessageCode::Success),
            ("310", MessageCode::ResultLimitExceeded),
            ("000310", MessageCode::Other("000310".to_owned())),
            (
                "999999999999999999999",
                MessageCode::Other("999999999999999999999".to_owned()),
            ),
        ] {
            let code = MessageCode::from_raw(raw);
            assert_eq!(code, expected);
            assert_eq!(code.as_str(), raw);
        }
    }

    #[test]
    fn date_time_accepts_naive_and_offset_values() {
        for raw in ["2026-11-20T12:13:14", "2026-11-20T12:13:14.123456+01:00"] {
            assert_eq!(XmlDateTime::new(raw).unwrap().as_str(), raw);
        }
        assert!(XmlDateTime::new("20.11.2026").is_err());
    }

    #[test]
    fn exact_decimal_parser_never_rounds_or_accepts_non_xsd_syntax() {
        assert_eq!(
            parse_decimal("-12.25", "test").unwrap(),
            Decimal::new(-1225, 2)
        );
        assert_eq!(parse_decimal(".5", "test").unwrap(), Decimal::new(5, 1));
        for invalid in ["", "1e2", "1_000", "NaN", "1.2.3"] {
            assert!(parse_decimal(invalid, "test").is_err(), "{invalid}");
        }
        assert!(matches!(
            parse_decimal("999999999999999999999999999999999999999", "test"),
            Err(crate::Error::DecimalOutOfRange { .. })
        ));
        assert!(matches!(
            parse_decimal("0.00000000000000000000000000001", "test"),
            Err(crate::Error::DecimalOutOfRange { .. })
        ));
    }
}
