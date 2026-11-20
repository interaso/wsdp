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

pub(crate) fn trim_xml_whitespace(value: &str) -> &str {
    value.trim_matches([' ', '\t', '\r', '\n'])
}

pub(crate) fn unsigned_integer_digits(value: &str) -> Option<&str> {
    let normalized = trim_xml_whitespace(value);
    let digits = normalized.strip_prefix('+').unwrap_or(normalized);
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    Some(digits)
}

pub(crate) fn positive_integer_digits(value: &str) -> Option<&str> {
    unsigned_integer_digits(value).filter(|digits| digits.bytes().any(|byte| byte != b'0'))
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
#[derive(Clone, Eq, PartialEq)]
pub struct ServiceMessage {
    /// Typed known code or the untouched lexical value.
    pub code: MessageCode,
    /// Message severity.
    pub level: MessageLevel,
    /// Human-readable server text.
    pub text: String,
}

impl fmt::Debug for ServiceMessage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let code = match &self.code {
            MessageCode::Other(_) => "[REDACTED]",
            _ => self.code.as_str(),
        };
        let level = match &self.level {
            MessageLevel::Other(_) => "[REDACTED]",
            _ => self.level.as_str(),
        };
        formatter
            .debug_struct("ServiceMessage")
            .field("code", &code)
            .field("level", &level)
            .field("text", &"[REDACTED]")
            .finish()
    }
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
    /// Simultaneous formatted and XML output is not allowed for this report type (`527`).
    CombinedFormattedAndXmlNotAllowed,
    /// Simultaneous formatted and XML output is not currently available (`538`).
    CombinedFormattedAndXmlUnavailable,
    /// A code unknown to this crate version.
    Other(String),
}

impl MessageCode {
    pub(crate) fn from_raw(raw: impl Into<String>) -> Self {
        let raw = raw.into();
        let Some(digits) = unsigned_integer_digits(&raw) else {
            return Self::Other(raw);
        };
        let canonical = digits.trim_start_matches('0');
        let canonical = if canonical.is_empty() { "0" } else { canonical };
        match canonical {
            "0" => Self::Success,
            "205" => Self::PasswordExpired,
            "304" => Self::RecordNotFound,
            "310" => Self::ResultLimitExceeded,
            "392" => Self::NoData,
            "477" => Self::ConcurrentUsersExceeded,
            "527" => Self::CombinedFormattedAndXmlNotAllowed,
            "538" => Self::CombinedFormattedAndXmlUnavailable,
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
            Self::CombinedFormattedAndXmlNotAllowed => "527",
            Self::CombinedFormattedAndXmlUnavailable => "538",
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
        if valid_xml_date_time(trim_xml_whitespace(&value)) {
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

pub(crate) fn valid_xml_date(value: &str) -> bool {
    valid_xml_date_value(value, true)
}

fn valid_xml_date_value(value: &str, allow_timezone: bool) -> bool {
    let Some((year, month, day, timezone)) = xml_date_parts(value) else {
        return false;
    };
    if (!allow_timezone && !timezone.is_empty()) || !valid_xml_timezone(timezone) {
        return false;
    }
    let Some(month) = two_ascii_digits(month) else {
        return false;
    };
    let Some(day) = two_ascii_digits(day) else {
        return false;
    };
    let maximum_day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => return false,
    };
    day >= 1 && day <= maximum_day
}

fn valid_xml_date_time(value: &str) -> bool {
    let Some((date, time_and_zone)) = value.split_once('T') else {
        return false;
    };
    if !valid_xml_date_value(date, false) || time_and_zone.len() < 8 {
        return false;
    }
    let bytes = time_and_zone.as_bytes();
    if bytes[2] != b':'
        || bytes[5] != b':'
        || !time_and_zone.is_ascii()
        || !bytes[..2]
            .iter()
            .chain(&bytes[3..5])
            .chain(&bytes[6..8])
            .all(u8::is_ascii_digit)
    {
        return false;
    }
    let (Some(hour), Some(minute), Some(second)) = (
        two_ascii_digits(&time_and_zone[..2]),
        two_ascii_digits(&time_and_zone[3..5]),
        two_ascii_digits(&time_and_zone[6..8]),
    ) else {
        return false;
    };
    if minute > 59 || second > 59 || hour > 24 {
        return false;
    }

    let mut suffix = &time_and_zone[8..];
    let mut fraction = None;
    if let Some(rest) = suffix.strip_prefix('.') {
        let fraction_len = rest
            .bytes()
            .position(|byte| !byte.is_ascii_digit())
            .unwrap_or(rest.len());
        if fraction_len == 0 {
            return false;
        }
        fraction = Some(&rest[..fraction_len]);
        suffix = &rest[fraction_len..];
    }
    if hour == 24
        && (minute != 0
            || second != 0
            || fraction.is_some_and(|digits| digits.bytes().any(|byte| byte != b'0')))
    {
        return false;
    }
    valid_xml_timezone(suffix)
}

fn xml_date_parts(value: &str) -> Option<(&str, &str, &str, &str)> {
    if !value.is_ascii() {
        return None;
    }
    let unsigned = value.strip_prefix('-').unwrap_or(value);
    let year_end = unsigned.find('-')?;
    let year = &unsigned[..year_end];
    let month_day_and_zone = &unsigned[year_end + 1..];
    if year.len() < 4
        || !year.bytes().all(|byte| byte.is_ascii_digit())
        || year.bytes().all(|byte| byte == b'0')
        || (year.len() > 4 && year.starts_with('0'))
        || month_day_and_zone.len() < 5
        || month_day_and_zone.as_bytes()[2] != b'-'
    {
        return None;
    }
    Some((
        year,
        &month_day_and_zone[..2],
        &month_day_and_zone[3..5],
        &month_day_and_zone[5..],
    ))
}

fn two_ascii_digits(value: &str) -> Option<u8> {
    let bytes = value.as_bytes();
    if bytes.len() == 2 && bytes.iter().all(u8::is_ascii_digit) {
        Some((bytes[0] - b'0') * 10 + bytes[1] - b'0')
    } else {
        None
    }
}

fn is_leap_year(year: &str) -> bool {
    fn modulo(digits: &str, modulus: u16) -> u16 {
        digits.bytes().fold(0, |remainder, digit| {
            (remainder * 10 + u16::from(digit - b'0')) % modulus
        })
    }

    modulo(year, 4) == 0 && (modulo(year, 100) != 0 || modulo(year, 400) == 0)
}

fn valid_xml_timezone(value: &str) -> bool {
    if value.is_empty() || value == "Z" {
        return true;
    }
    let bytes = value.as_bytes();
    if bytes.len() != 6
        || !matches!(bytes[0], b'+' | b'-')
        || bytes[3] != b':'
        || !bytes[1..3]
            .iter()
            .chain(&bytes[4..])
            .all(u8::is_ascii_digit)
    {
        return false;
    }
    let (Some(hour), Some(minute)) = (
        two_ascii_digits(&value[1..3]),
        two_ascii_digits(&value[4..6]),
    ) else {
        return false;
    };
    minute <= 59 && (hour < 14 || (hour == 14 && minute == 0))
}

/// Service health information returned by `stavWS`.
#[derive(Clone, Eq, PartialEq)]
#[non_exhaustive]
pub struct ServiceStatus {
    /// Server implementation version, when supplied.
    pub version: Option<String>,
    /// Authenticated login name, when supplied by the server.
    pub username: Option<String>,
    /// Server processing timestamp with its original offset semantics.
    pub processed_at: Option<XmlDateTime>,
}

impl fmt::Debug for ServiceStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ServiceStatus")
            .field("version", &self.version)
            .field("username", &self.username.as_ref().map(|_| "[REDACTED]"))
            .field("processed_at", &self.processed_at)
            .finish()
    }
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
    let canonical = canonical_decimal(raw);
    Decimal::from_str_exact(&canonical).map_err(|error| match error {
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

fn canonical_decimal(raw: &str) -> String {
    let (negative, unsigned) = match raw.as_bytes().first() {
        Some(b'-') => (true, &raw[1..]),
        Some(b'+') => (false, &raw[1..]),
        _ => (false, raw),
    };
    let (integer, fraction) = unsigned.split_once('.').unwrap_or((unsigned, ""));
    let integer = integer.trim_start_matches('0');
    let fraction = fraction.trim_end_matches('0');
    let mut result = String::with_capacity(raw.len());
    if negative {
        result.push('-');
    }
    if integer.is_empty() {
        result.push('0');
    } else {
        result.push_str(integer);
    }
    if !fraction.is_empty() {
        result.push('.');
        result.push_str(fraction);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_codes_canonicalize_known_values_and_preserve_unknown_raw_text() {
        for (raw, expected, canonical) in [
            ("0", MessageCode::Success, "0"),
            (" \t+000205\r\n", MessageCode::PasswordExpired, "205"),
            ("000304", MessageCode::RecordNotFound, "304"),
            ("+000310", MessageCode::ResultLimitExceeded, "310"),
            ("000392", MessageCode::NoData, "392"),
            ("000477", MessageCode::ConcurrentUsersExceeded, "477"),
            (
                "000527",
                MessageCode::CombinedFormattedAndXmlNotAllowed,
                "527",
            ),
            (
                "+000538",
                MessageCode::CombinedFormattedAndXmlUnavailable,
                "538",
            ),
        ] {
            let code = MessageCode::from_raw(raw);
            assert_eq!(code, expected);
            assert_eq!(code.as_str(), canonical);
        }
        let many_zeroes = format!(" \n+{}310\t", "0".repeat(1000));
        assert_eq!(
            MessageCode::from_raw(many_zeroes),
            MessageCode::ResultLimitExceeded
        );
        for raw in [
            "579",
            "000579",
            "999999999999999999999",
            " +000999 ",
            "31 0",
            "-310",
            "+",
        ] {
            let code = MessageCode::from_raw(raw);
            assert_eq!(code, MessageCode::Other(raw.to_owned()));
            assert_eq!(code.as_str(), raw);
        }
    }

    #[test]
    fn date_time_enforces_xsd_lexical_and_calendar_rules_without_changing_text() {
        for raw in [
            "2026-11-20T12:13:14",
            "2026-11-20T12:13:14Z",
            "2026-11-20T12:13:14.123456789012345678901234567890+01:00",
            " \t2024-02-29T24:00:00.000-14:00\r\n",
            "-0001-01-01T00:00:00",
            "10000-01-01T00:00:00+14:00",
        ] {
            assert_eq!(XmlDateTime::new(raw).unwrap().as_str(), raw);
        }
        for raw in [
            "20.11.2026",
            "2026-02-29T00:00:00",
            "2026-11-20 12:13:14",
            "2026-11-20t12:13:14z",
            "2026-11-20T12:13:14,5Z",
            "2026-11-20T12:13:14.Z",
            "2026-11-20T12:13:14+01:00:30",
            "2026-11-20T12:13:14+14:01",
            "2026-11-20T12:13:14+15:00",
            "2026-11-20T24:00:00.1Z",
            "2026-11-20ZT12:13:14",
            "2026-11-20+01:00T12:13:14",
            "\u{a0}2026-11-20T12:13:14\u{a0}",
        ] {
            assert!(XmlDateTime::new(raw).is_err(), "{raw}");
        }
    }

    #[test]
    fn service_debug_output_redacts_text_unknown_values_and_username() {
        let message = ServiceMessage {
            code: MessageCode::Other("secret-code-123".to_owned()),
            level: MessageLevel::Other("secret-level-456".to_owned()),
            text: "secret-message-789".to_owned(),
        };
        assert_eq!(message.code.as_str(), "secret-code-123");
        assert_eq!(message.level.as_str(), "secret-level-456");
        assert_eq!(message.text, "secret-message-789");
        let response = Response {
            value: crate::model::search::SearchResult::from_items(Vec::<u8>::new()),
            messages: vec![message],
        };
        let debug = format!("{response:?}");
        for sentinel in ["secret-code-123", "secret-level-456", "secret-message-789"] {
            assert!(!debug.contains(sentinel), "{debug}");
        }
        assert!(debug.contains("[REDACTED]"));

        let status = ServiceStatus {
            version: Some("3.1".to_owned()),
            username: Some("secret-user-name".to_owned()),
            processed_at: Some(XmlDateTime::new("2026-11-20T12:13:14Z").unwrap()),
        };
        let debug = format!("{status:?}");
        assert!(!debug.contains("secret-user-name"));
        assert!(debug.contains("3.1"));
        assert_eq!(status.username.as_deref(), Some("secret-user-name"));
    }

    #[test]
    fn exact_decimal_parser_never_rounds_or_accepts_non_xsd_syntax() {
        assert_eq!(
            parse_decimal("-12.25", "test").unwrap(),
            Decimal::new(-1225, 2)
        );
        assert_eq!(parse_decimal(".5", "test").unwrap(), Decimal::new(5, 1));
        assert_eq!(
            parse_decimal("0.00000000000000000000000000000", "test").unwrap(),
            Decimal::ZERO
        );
        assert_eq!(parse_decimal("-.000", "test").unwrap(), Decimal::ZERO);
        assert_eq!(parse_decimal("1.", "test").unwrap(), Decimal::ONE);
        assert_eq!(
            parse_decimal("+0001.2300", "test").unwrap(),
            Decimal::new(123, 2)
        );
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
