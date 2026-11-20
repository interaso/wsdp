use std::fmt;
use std::str::FromStr;

use rust_decimal::Decimal;

use super::common::{positive_integer_digits, trim_xml_whitespace};
use crate::{Error, Result};

macro_rules! string_id {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize))]
        #[cfg_attr(feature = "serde", serde(transparent))]
        pub struct $name(String);

        impl $name {
            #[doc = "Validates a positive decimal identifier of at most 30 digits, preserving its lexical form."]
            pub fn new(value: impl Into<String>) -> Result<Self> {
                let value = value.into();
                validate_id(&value)?;
                Ok(Self(value))
            }

            #[doc = "Returns the identifier without numeric conversion."]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(concat!(stringify!($name), "([ID])"))
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(self.as_str())
            }
        }

        impl FromStr for $name {
            type Err = Error;

            fn from_str(value: &str) -> Result<Self> {
                Self::new(value)
            }
        }

        impl TryFrom<&str> for $name {
            type Error = Error;

            fn try_from(value: &str) -> Result<Self> {
                Self::new(value)
            }
        }

        impl TryFrom<String> for $name {
            type Error = Error;

            fn try_from(value: String) -> Result<Self> {
                Self::new(value)
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }

        #[cfg(feature = "serde")]
        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let value = <String as serde::Deserialize>::deserialize(deserializer)?;
                Self::new(value).map_err(serde::de::Error::custom)
            }
        }
    };
}

string_id!(ParcelId, "Internal parcel identifier (`IdType`).");
string_id!(BuildingId, "Internal building identifier (`IdType`).");
string_id!(UnitId, "Internal unit identifier (`IdType`).");
string_id!(
    BuildingRightId,
    "Internal building-right identifier (`IdType`)."
);
string_id!(
    SubjectId,
    "Internal authorized-subject identifier (`IdType`)."
);
string_id!(
    OwnershipId,
    "Internal ownership-record identifier (`IdType`)."
);
string_id!(ProceedingId, "Internal proceeding identifier (`IdType`).");
string_id!(
    OwnershipSheetId,
    "Internal ownership-sheet identifier (`IdType`)."
);
string_id!(AddressId, "Internal address identifier (`IdType`).");
string_id!(DocumentId, "Internal document identifier (`IdType`).");
string_id!(
    ReportId,
    "Internal asynchronous report identifier (`IdType`)."
);

fn validate_id(value: &str) -> Result<()> {
    let Some(digits) = positive_integer_digits(value) else {
        return Err(Error::invalid_input(
            "id",
            "expected a positive decimal integer with at most 30 digits",
        ));
    };
    if digits.len() > 30 {
        return Err(Error::invalid_input(
            "id",
            "expected a positive decimal integer with at most 30 digits",
        ));
    }
    Ok(())
}

macro_rules! numeric_code {
    ($name:ident, $maximum:expr, $doc:literal) => {
        #[doc = $doc]
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize))]
        #[cfg_attr(feature = "serde", serde(transparent))]
        pub struct $name(String);

        impl $name {
            #[doc = "Validates a positive decimal code and preserves leading zeroes."]
            pub fn new(value: impl Into<String>) -> Result<Self> {
                let value = value.into();
                let Some(digits) = positive_integer_digits(&value) else {
                    return Err(Error::invalid_input(
                        "code",
                        "expected a positive decimal integer",
                    ));
                };
                let significant = digits.trim_start_matches('0');
                let numeric = significant.parse::<u64>().map_err(|_| {
                    Error::invalid_input("code", "code is outside the supported range")
                })?;
                if numeric > $maximum {
                    return Err(Error::invalid_input(
                        "code",
                        "code is outside its XSD range",
                    ));
                }
                Ok(Self(value))
            }

            #[doc = "Returns the original lexical code."]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(self.as_str())
            }
        }

        impl FromStr for $name {
            type Err = Error;

            fn from_str(value: &str) -> Result<Self> {
                Self::new(value)
            }
        }

        impl TryFrom<&str> for $name {
            type Error = Error;

            fn try_from(value: &str) -> Result<Self> {
                Self::new(value)
            }
        }

        #[cfg(feature = "serde")]
        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let value = <String as serde::Deserialize>::deserialize(deserializer)?;
                Self::new(value).map_err(serde::de::Error::custom)
            }
        }
    };
}

numeric_code!(CadastralAreaCode, 999_999, "Cadastral-area code.");
numeric_code!(MunicipalityCode, 999_999, "Municipality code.");
numeric_code!(MunicipalityPartCode, 999_999, "Municipality-part code.");
numeric_code!(CityDistrictCode, 999_999, "City-district code.");
numeric_code!(DistrictCode, 999_999, "District code.");
numeric_code!(RegionCode, 999_999, "Region code.");
numeric_code!(NewRegionCode, 999, "Post-1960 region/VÚSC code.");
numeric_code!(WorkplaceCode, 999, "Cadastral-workplace code.");
numeric_code!(StreetCode, 9_999_999, "Street code.");
numeric_code!(
    CompanyId,
    99_999_999,
    "String-backed company identification number (IČO)."
);

/// A legacy report identifier, which can be empty and has no `IdType` digit limit.
#[derive(Clone, Default, Eq, Hash, PartialEq)]
pub struct LegacyId(String);

impl LegacyId {
    /// Validates legacy `ID_TYPE`: either empty or an XML positive integer.
    ///
    /// Surrounding XML whitespace and a leading `+` are accepted for non-empty
    /// values, while the original text is retained. There is no numeric digit limit.
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        if value.is_empty() {
            return Ok(Self(value));
        }
        let normalized = trim_xml_whitespace(&value);
        if positive_integer_digits(normalized).is_none() {
            return Err(Error::invalid_input(
                "legacy_id",
                "expected empty text or a positive integer",
            ));
        }
        Ok(Self(value))
    }

    /// Returns the untouched identifier text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for LegacyId {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        Self::new(value)
    }
}

impl fmt::Display for LegacyId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl fmt::Debug for LegacyId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("LegacyId([ID])")
    }
}

/// A parcel root number and optional subdivision.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ParcelNumber {
    /// Root (`kmenove`) number, 0–99999.
    pub root: u32,
    /// Optional subdivision, 1–999. Omission means an unsubdivided parcel.
    pub subdivision: Option<u16>,
}

impl ParcelNumber {
    /// Validates a parcel number without interpreting omitted subdivision as a wildcard.
    pub fn new(root: u32, subdivision: Option<u16>) -> Result<Self> {
        let value = Self { root, subdivision };
        value.validate()?;
        Ok(value)
    }

    pub(crate) fn validate(&self) -> Result<()> {
        if self.root > 99_999 {
            return Err(Error::invalid_input(
                "root",
                "parcel root must be at most 99999",
            ));
        }
        if self
            .subdivision
            .is_some_and(|value| value == 0 || value > 999)
        {
            return Err(Error::invalid_input(
                "subdivision",
                "parcel subdivision must be in 1..=999",
            ));
        }
        Ok(())
    }
}

/// A natural-person or partnership identifier whose debug output is redacted.
#[derive(Clone, Eq, Hash, PartialEq)]
pub struct PersonalIdentifier(String);

impl PersonalIdentifier {
    /// Validates the XSD's 9–10 character constraint without inventing a checksum rule.
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        if !(9..=10).contains(&value.chars().count()) {
            return Err(Error::invalid_input(
                "personal_identifier",
                "expected 9 or 10 characters",
            ));
        }
        Ok(Self(value))
    }

    /// Explicitly exposes the identifier for request serialization.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for PersonalIdentifier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("PersonalIdentifier([REDACTED])")
    }
}

/// Exact non-negative Czech koruna amount used for report prices and charge approval.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Czk(Decimal);

impl Czk {
    /// Zero CZK.
    pub const ZERO: Self = Self(Decimal::ZERO);

    /// Creates a non-negative exact amount.
    pub fn new(value: Decimal) -> Result<Self> {
        if value.is_sign_negative() {
            return Err(Error::invalid_input(
                "amount",
                "CZK amount cannot be negative",
            ));
        }
        Ok(Self(value))
    }

    /// Creates an exact whole-CZK amount without floating point.
    pub fn from_major_units(value: u64) -> Self {
        Self(Decimal::from(value))
    }

    /// Returns the exact decimal amount.
    pub fn as_decimal(&self) -> &Decimal {
        &self.0
    }
}

impl FromStr for Czk {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        Self::new(super::parse_decimal(value, "CZK amount")?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_accept_thirty_digits_and_reject_numeric_overflow_or_zero() {
        let raw = "123456789012345678901234567890";
        assert_eq!(ParcelId::new(raw).unwrap().as_str(), raw);
        assert!(ParcelId::new("1234567890123456789012345678901").is_err());
        let with_xml_syntax = format!(" \t+{raw}\r\n");
        assert_eq!(
            ParcelId::new(with_xml_syntax.clone()).unwrap().as_str(),
            with_xml_syntax
        );
        assert!(ParcelId::new(" +1234567890123456789012345678901 ").is_err());
        assert!(ParcelId::new("0").is_err());
        assert!(ParcelId::new("+000").is_err());
        assert!(ParcelId::new("12x").is_err());
        assert!(ParcelId::new("++1").is_err());
        assert!(ParcelId::new("").is_err());
    }

    #[test]
    fn codes_preserve_leading_zeroes_and_validate_ranges() {
        assert_eq!(CadastralAreaCode::new("000001").unwrap().as_str(), "000001");
        let many_zeroes = format!("{}1", "0".repeat(100));
        assert_eq!(
            CadastralAreaCode::new(many_zeroes.as_str())
                .unwrap()
                .as_str(),
            many_zeroes
        );
        assert_eq!(
            CadastralAreaCode::new(" \t+000001\r\n").unwrap().as_str(),
            " \t+000001\r\n"
        );
        assert!(CadastralAreaCode::new("1000000").is_err());
        assert!(CadastralAreaCode::new("+1000000").is_err());
        assert!(WorkplaceCode::new("1000").is_err());
    }

    #[test]
    fn parcel_numbers_do_not_treat_missing_subdivision_as_wildcard() {
        assert_eq!(ParcelNumber::new(77, None).unwrap().subdivision, None);
        assert!(ParcelNumber::new(100_000, None).is_err());
        assert!(ParcelNumber::new(1, Some(0)).is_err());
    }

    #[test]
    fn personal_ids_and_internal_ids_have_redacted_debug() {
        assert!(!format!("{:?}", PersonalIdentifier::new("123456789").unwrap()).contains("123456"));
        assert!(!format!("{:?}", ParcelId::new("123456").unwrap()).contains("123456"));
    }

    #[test]
    fn czk_is_exact_and_nonnegative() {
        let value = Czk::from_str("10.25").unwrap();
        assert_eq!(value.as_decimal(), &Decimal::new(1025, 2));
        assert!(Czk::from_str("-0.01").is_err());
        assert!(matches!(
            Czk::from_str("not-decimal"),
            Err(Error::InvalidInput { .. })
        ));
        assert!(matches!(
            Czk::from_str("999999999999999999999999999999999999999"),
            Err(Error::DecimalOutOfRange { .. })
        ));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serde_deserialization_uses_the_same_id_validation() {
        let id: ParcelId = serde_json::from_str("\"000123\"").unwrap();
        assert_eq!(id.as_str(), "000123");
        let id: ParcelId = serde_json::from_str("\" \\t+000123\\n\"").unwrap();
        assert_eq!(id.as_str(), " \t+000123\n");
        assert!(serde_json::from_str::<ParcelId>("\"0\"").is_err());
    }
}
