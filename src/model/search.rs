use std::fmt;
use std::str::FromStr;

use super::common::{MessageCode, ServiceMessage, XmlDateTime};
use super::id::{
    BuildingId, BuildingRightId, CadastralAreaCode, CompanyId, DocumentId, MunicipalityPartCode,
    OwnershipId, OwnershipSheetId, ParcelId, ParcelNumber, PersonalIdentifier, ProceedingId,
    SubjectId, UnitId, WorkplaceCode,
};
use crate::{Error, Result};

/// An XML Schema date, retaining its lexical representation and optional offset.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct XmlDate(String);

impl XmlDate {
    /// Validates an `xs:date` without assigning a timezone to a naive date.
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        if valid_xml_date(&value) {
            Ok(Self(value))
        } else {
            Err(Error::invalid_input(
                "date",
                "expected an XML date with an optional offset",
            ))
        }
    }

    /// Returns the original lexical representation.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for XmlDate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for XmlDate {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        Self::new(value)
    }
}

fn valid_xml_date(value: &str) -> bool {
    if !value.is_ascii() || value.len() < 10 || value[..10].parse::<jiff::civil::Date>().is_err() {
        return false;
    }
    let suffix = &value[10..];
    if suffix.is_empty() || suffix == "Z" {
        return true;
    }
    let bytes = suffix.as_bytes();
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
    let (Ok(hour), Ok(minute)) = (suffix[1..3].parse::<u8>(), suffix[4..6].parse::<u8>()) else {
        return false;
    };
    minute <= 59 && (hour < 14 || (hour == 14 && minute == 0))
}

/// A building-type code (`KodTypuStavbyType`), preserving leading zeroes.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct BuildingTypeCode(String);

impl BuildingTypeCode {
    /// Validates the schema range 1–9 without discarding the lexical code.
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        let valid = !value.is_empty()
            && value.bytes().all(|byte| byte.is_ascii_digit())
            && value
                .parse::<u8>()
                .is_ok_and(|number| (1..=9).contains(&number));
        if valid {
            Ok(Self(value))
        } else {
            Err(Error::invalid_input(
                "building_type",
                "expected a decimal building-type code in 1..=9",
            ))
        }
    }

    /// Returns the original code.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for BuildingTypeCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for BuildingTypeCode {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        Self::new(value)
    }
}

/// A positive numerator or denominator of an ownership share.
///
/// The XSD permits 30 digits, so this deliberately does not use a machine integer.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct FractionComponent(String);

impl FractionComponent {
    /// Validates a positive decimal integer of at most 30 digits.
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        if value.is_empty()
            || value.len() > 30
            || !value.bytes().all(|byte| byte.is_ascii_digit())
            || value.bytes().all(|byte| byte == b'0')
        {
            return Err(Error::invalid_input(
                "fraction_component",
                "expected a positive decimal integer with at most 30 digits",
            ));
        }
        Ok(Self(value))
    }

    /// Returns the exact original integer text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for FractionComponent {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for FractionComponent {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        Self::new(value)
    }
}

/// A shortened personal identifier whose debug output is redacted.
#[derive(Clone, Eq, Hash, PartialEq)]
pub struct ShortPersonalIdentifier(String);

impl ShortPersonalIdentifier {
    /// Validates the response schema's 1–10 character constraint.
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        if !(1..=10).contains(&value.chars().count()) {
            return Err(Error::invalid_input(
                "short_personal_identifier",
                "expected between 1 and 10 characters",
            ));
        }
        Ok(Self(value))
    }

    /// Explicitly exposes the identifier to callers that need it.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for ShortPersonalIdentifier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ShortPersonalIdentifier([REDACTED])")
    }
}

/// Whether a parcel belongs to the full or simplified cadastral register.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum ParcelKind {
    /// Cadastral parcel (`PKN`).
    Cadastral,
    /// Simplified-register parcel (`PZE`).
    Simplified,
    /// A future or otherwise unknown value, retained verbatim.
    Other(String),
}

impl ParcelKind {
    pub(crate) fn from_raw(raw: String) -> Self {
        match raw.as_str() {
            "PKN" => Self::Cadastral,
            "PZE" => Self::Simplified,
            _ => Self::Other(raw),
        }
    }

    /// Returns the official or untouched unknown value.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Cadastral => "PKN",
            Self::Simplified => "PZE",
            Self::Other(raw) => raw,
        }
    }
}

/// The parcel numbering scheme.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum ParcelNumberingKind {
    /// Schema value `1`.
    One,
    /// Schema value `2`.
    Two,
    /// A future or otherwise unknown value, retained verbatim.
    Other(String),
}

impl ParcelNumberingKind {
    pub(crate) fn from_raw(raw: String) -> Self {
        match raw.as_str() {
            "1" => Self::One,
            "2" => Self::Two,
            _ => Self::Other(raw),
        }
    }

    /// Returns the official or untouched unknown value.
    pub fn as_str(&self) -> &str {
        match self {
            Self::One => "1",
            Self::Two => "2",
            Self::Other(raw) => raw,
        }
    }
}

/// A schema `AnoNeType` value.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum YesNo {
    /// Yes (`a`).
    Yes,
    /// No (`n`).
    No,
    /// A future or otherwise unknown value, retained verbatim.
    Other(String),
}

impl YesNo {
    pub(crate) fn from_raw(raw: String) -> Self {
        match raw.as_str() {
            "a" => Self::Yes,
            "n" => Self::No,
            _ => Self::Other(raw),
        }
    }

    /// Returns the official or untouched unknown value.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Yes => "a",
            Self::No => "n",
            Self::Other(raw) => raw,
        }
    }
}

/// The object of which a building forms a part.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum BuildingAttachment {
    /// A building right (`R`).
    BuildingRight,
    /// A parcel (`P`).
    Parcel,
    /// A future or otherwise unknown value, retained verbatim.
    Other(String),
}

impl BuildingAttachment {
    pub(crate) fn from_raw(raw: String) -> Self {
        match raw.as_str() {
            "R" => Self::BuildingRight,
            "P" => Self::Parcel,
            _ => Self::Other(raw),
        }
    }

    /// Returns the official or untouched unknown value.
    pub fn as_str(&self) -> &str {
        match self {
            Self::BuildingRight => "R",
            Self::Parcel => "P",
            Self::Other(raw) => raw,
        }
    }
}

/// The kind of an authorized subject.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum SubjectKind {
    /// Natural person (`OFO`).
    NaturalPerson,
    /// Legal person (`OPO`).
    LegalPerson,
    /// Marital or partner joint property (`BSM`).
    Partnership,
    /// A future or otherwise unknown value, retained verbatim.
    Other(String),
}

impl SubjectKind {
    pub(crate) fn from_raw(raw: String) -> Self {
        match raw.as_str() {
            "OFO" => Self::NaturalPerson,
            "OPO" => Self::LegalPerson,
            "BSM" => Self::Partnership,
            _ => Self::Other(raw),
        }
    }

    /// Returns the official or untouched unknown value.
    pub fn as_str(&self) -> &str {
        match self {
            Self::NaturalPerson => "OFO",
            Self::LegalPerson => "OPO",
            Self::Partnership => "BSM",
            Self::Other(raw) => raw,
        }
    }
}

/// Whether a house number is descriptive or evidentiary.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum AddressNumberKind {
    /// Descriptive number (`1`).
    Descriptive,
    /// Evidentiary number (`2`).
    Evidentiary,
    /// A future or otherwise unknown value, retained verbatim.
    Other(String),
}

impl AddressNumberKind {
    pub(crate) fn from_raw(raw: String) -> Self {
        match raw.as_str() {
            "1" => Self::Descriptive,
            "2" => Self::Evidentiary,
            _ => Self::Other(raw),
        }
    }

    /// Returns the official or untouched unknown value.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Descriptive => "1",
            Self::Evidentiary => "2",
            Self::Other(raw) => raw,
        }
    }
}

/// A cadastral area with its optional code and display name.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CadastralAreaReference {
    /// Numeric cadastral-area code.
    pub code: Option<CadastralAreaCode>,
    /// Official display name.
    pub name: Option<String>,
}

/// A municipality part with its code and display name.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MunicipalityPartReference {
    /// Numeric municipality-part code.
    pub code: MunicipalityPartCode,
    /// Official display name.
    pub name: String,
}

/// Identifying fields shared by full parcel records and nested parcel references.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParcelIdentity {
    /// Internal parcel ID.
    pub id: ParcelId,
    /// Ownership-sheet ID, when supplied.
    pub ownership_sheet_id: Option<OwnershipSheetId>,
    /// Current cadastral-area code.
    pub cadastral_area: CadastralAreaCode,
    /// Parcel root number.
    pub root_number: u32,
    /// Register kind.
    pub kind: ParcelKind,
    /// Parcel numbering scheme.
    pub numbering_kind: ParcelNumberingKind,
    /// Optional parcel subdivision.
    pub subdivision: Option<u16>,
    /// Original cadastral-area code, when supplied.
    pub original_cadastral_area: Option<CadastralAreaCode>,
    /// Simplified-register recording kind (`typZE`), preserved lexically.
    pub simplified_registration_kind: Option<String>,
    /// Optional parcel part number.
    pub part: Option<u16>,
}

/// The natural-key alternative for a building without a house number.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BuildingLocation {
    /// Municipality-part code and house number.
    Address {
        /// Municipality-part code.
        municipality_part: MunicipalityPartCode,
        /// Descriptive or evidentiary house number, retaining its lexical form.
        house_number: String,
    },
    /// Identifying parcel for building types that have no house number.
    Parcel(ParcelIdentity),
}

/// A nested building identifier (`StavbaIdentType`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuildingIdentity {
    /// Internal building ID.
    pub id: BuildingId,
    /// Ownership-sheet ID, when supplied.
    pub ownership_sheet_id: Option<OwnershipSheetId>,
    /// Building-type code.
    pub building_type: BuildingTypeCode,
    /// Address or identifying-parcel choice.
    pub location: BuildingLocation,
}

/// A nested building-right identifier (`PravoStavbyIdentType`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuildingRightIdentity {
    /// Internal building-right ID.
    pub id: BuildingRightId,
    /// Ownership-sheet ID, when supplied.
    pub ownership_sheet_id: Option<OwnershipSheetId>,
    /// Identifying parcel.
    pub identifying_parcel: ParcelIdentity,
}

/// A complete parcel returned by search or LV preview (`ParcelaType`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Parcel {
    /// Internal parcel ID.
    pub id: ParcelId,
    /// Register kind.
    pub kind: ParcelKind,
    /// Current cadastral-area code.
    pub cadastral_area: CadastralAreaCode,
    /// Original cadastral-area code, when supplied.
    pub original_cadastral_area: Option<CadastralAreaCode>,
    /// Parcel root number.
    pub root_number: u32,
    /// Optional parcel subdivision.
    pub subdivision: Option<u16>,
    /// Optional parcel part number.
    pub part: Option<u16>,
    /// Area-determination code, preserved lexically.
    pub area_determination_code: Option<String>,
    /// Simplified-register recording kind (`typZE`), preserved lexically.
    pub simplified_registration_kind: Option<String>,
    /// Land-type code, preserved lexically.
    pub land_type_code: Option<String>,
    /// Land-use code, preserved lexically.
    pub land_use_code: Option<String>,
    /// Parcel-type code, preserved lexically.
    pub parcel_type_code: Option<String>,
    /// Parcel area in the unit defined by WSDP (square metres).
    pub area: u32,
    /// Ownership-sheet ID, when supplied.
    pub ownership_sheet_id: Option<OwnershipSheetId>,
    /// Parcel numbering scheme.
    pub numbering_kind: ParcelNumberingKind,
    /// Related building identifier, when supplied.
    pub building: Option<BuildingIdentity>,
    /// Whether the building is a component of the parcel.
    pub building_is_part: YesNo,
    /// Related building-right identifier, when supplied.
    pub building_right: Option<BuildingRightIdentity>,
}

/// A building returned by search, unit overview, or LV preview (`StavbaType`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Building {
    /// Internal building ID.
    pub id: BuildingId,
    /// Building-type code.
    pub building_type: BuildingTypeCode,
    /// Municipality part, when supplied.
    pub municipality_part: Option<MunicipalityPartReference>,
    /// House number, when supplied, retaining its lexical form.
    pub house_number: Option<String>,
    /// Building-use code, preserved lexically.
    pub use_code: Option<String>,
    /// Identifying parcel, when supplied.
    pub identifying_parcel: Option<ParcelIdentity>,
    /// Ownership-sheet ID, when supplied.
    pub ownership_sheet_id: Option<OwnershipSheetId>,
    /// Whether this building is attached to a parcel or building right.
    pub attachment: Option<BuildingAttachment>,
    /// Related building-right identifier, when supplied.
    pub building_right: Option<BuildingRightIdentity>,
    /// Temporary-building flag.
    pub temporary: Option<YesNo>,
    /// Defined-units flag; the official schema notes that this is future data.
    pub defined_units: Option<YesNo>,
}

/// A unit returned by search (`JednotkaType`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Unit {
    /// Internal unit ID.
    pub id: UnitId,
    /// Building to which the unit belongs.
    pub building: BuildingIdentity,
    /// Unit-type code, preserved lexically.
    pub unit_type_code: String,
    /// Unit number.
    pub number: u64,
    /// Unit-use code, when supplied and preserved lexically.
    pub use_code: Option<String>,
    /// Ownership-sheet ID, when supplied.
    pub ownership_sheet_id: Option<OwnershipSheetId>,
}

/// A purpose assigned to a building right.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuildingRightPurpose {
    /// Dynamic purpose code, preserved lexically.
    pub code: String,
    /// Official purpose name.
    pub name: String,
}

/// A building right returned by search or LV preview (`PravoStavbyType`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuildingRight {
    /// Internal building-right ID.
    pub id: BuildingRightId,
    /// End of validity.
    pub valid_until: XmlDate,
    /// Ownership-sheet ID, when supplied.
    pub ownership_sheet_id: Option<OwnershipSheetId>,
    /// One or more purposes, as required by the XSD.
    pub purposes: Vec<BuildingRightPurpose>,
    /// Whether `parcels` contains only the identifying parcel.
    pub only_identifying_parcel: YesNo,
    /// One or more associated parcels, as required by the XSD.
    pub parcels: Vec<ParcelIdentity>,
}

/// Fields common to a primary authorized subject and each partner.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubjectBase {
    /// Internal authorized-subject ID.
    pub id: SubjectId,
    /// Legal-person name.
    pub name: Option<String>,
    /// Company identification number (IČO).
    pub company_id: Option<CompanyId>,
    /// Company-ID suffix, preserved lexically.
    pub company_id_suffix: Option<String>,
    /// Surname.
    pub surname: Option<String>,
    /// Given name.
    pub given_name: Option<String>,
    /// Title before the name.
    pub title_before: Option<String>,
    /// Title after the name.
    pub title_after: Option<String>,
    /// Shortened personal identifier, redacted from `Debug` output.
    pub short_personal_id: Option<ShortPersonalIdentifier>,
    /// Street name.
    pub street: Option<String>,
    /// House number, retaining its lexical form.
    pub house_number: Option<String>,
    /// Orientation number, preserved as text.
    pub orientation_number: Option<String>,
    /// Municipality-part name.
    pub municipality_part_name: Option<String>,
    /// Municipality name.
    pub municipality_name: Option<String>,
    /// City-district name.
    pub city_district_name: Option<String>,
    /// City-borough name.
    pub city_borough_name: Option<String>,
    /// Postal code, retaining leading zeroes and its lexical form.
    pub postal_code: Option<String>,
    /// Subject kind.
    pub kind: SubjectKind,
    /// Descriptive/evidentiary house-number kind.
    pub address_number_kind: Option<AddressNumberKind>,
    /// Country name.
    pub country_name: Option<String>,
    /// District name.
    pub district_name: Option<String>,
    /// Dynamic subject-characteristic code, preserved lexically.
    pub characteristic_code: Option<String>,
}

/// An authorized subject, including either optional partner (`OSType`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthorizedSubject {
    /// Fields common to every subject form.
    pub base: SubjectBase,
    /// First partner for joint-property subjects.
    pub partner1: Option<SubjectBase>,
    /// Second partner for joint-property subjects.
    pub partner2: Option<SubjectBase>,
}

/// A cadastral proceeding (`RizeniType`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Proceeding {
    /// Internal proceeding ID.
    pub id: ProceedingId,
    /// Cadastral-workplace code.
    pub workplace: WorkplaceCode,
    /// Dynamic proceeding-type code, preserved lexically.
    pub proceeding_type: String,
    /// Serial number within year, type, and workplace.
    pub serial_number: u32,
    /// Proceeding year.
    pub year: u16,
    /// External identifier, when supplied.
    pub external_id: Option<String>,
    /// Creation timestamp with its original XML timezone semantics.
    pub created_at: XmlDateTime,
}

/// A document in the collection of deeds (`ListinaType`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollectionDocument {
    /// Internal document ID.
    pub id: DocumentId,
    /// Official document designation.
    pub designation: String,
    /// Execution timestamp, when supplied.
    pub executed_at: Option<XmlDateTime>,
}

/// A geometric-plan identifier (`GPIdentType`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeometricPlan {
    /// Cadastral-area code.
    pub cadastral_area: CadastralAreaCode,
    /// Survey-result or field-sketch number, including its sign.
    pub survey_result_number: i32,
    /// Internal document ID.
    pub document_id: DocumentId,
    /// Document file name.
    pub file_name: String,
    /// Proceeding confirming the plan.
    pub proceeding: Proceeding,
    /// Cadastral-change proposal number, when supplied.
    pub change_proposal_number: Option<String>,
    /// Confirmation date, when supplied.
    pub confirmed_on: Option<XmlDate>,
}

/// The object or subject targeted by an ownership record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OwnershipTarget {
    /// An authorized subject.
    Subject(SubjectId),
    /// A parcel in additional co-ownership.
    Parcel(ParcelId),
    /// A building in additional co-ownership.
    Building(BuildingId),
    /// A unit in additional co-ownership.
    Unit(UnitId),
    /// A building right in additional co-ownership.
    BuildingRight(BuildingRightId),
}

/// An ownership record returned by information operations (`VlastnictviType`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Ownership {
    /// Internal ownership-record ID.
    pub id: OwnershipId,
    /// Ownership-sheet ID.
    pub ownership_sheet_id: OwnershipSheetId,
    /// Exact share numerator, supporting all 30 XSD digits.
    pub share_numerator: FractionComponent,
    /// Exact share denominator, supporting all 30 XSD digits.
    pub share_denominator: FractionComponent,
    /// Dynamic legal-relation code, preserved lexically.
    pub legal_relation_code: String,
    /// Cadastral-area code.
    pub cadastral_area: CadastralAreaCode,
    /// Ownership-sheet number.
    pub ownership_sheet_number: u32,
    /// Subject or object selected by the XSD choice.
    pub target: OwnershipTarget,
}

/// Parcel lookup by internal ID or natural key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParcelSearchSelector {
    /// Internal parcel ID.
    Id(ParcelId),
    /// Cadastral area and parcel number.
    Number(ParcelSearchKey),
}

/// A parcel natural key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParcelSearchKey {
    /// Cadastral-area code.
    pub cadastral_area: CadastralAreaCode,
    /// Root number and optional subdivision; omission means unsubdivided.
    pub number: ParcelNumber,
}

/// `najdiParcelu` request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParcelSearch {
    /// ID/natural-key choice.
    pub selector: ParcelSearchSelector,
    /// Historical timestamp; `None` requests current server data.
    pub valid_at: Option<XmlDateTime>,
}

/// Building lookup by internal ID or natural key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BuildingSearchSelector {
    /// Internal building ID.
    Id(BuildingId),
    /// Municipality part, building type, and house number.
    Number(BuildingSearchKey),
}

/// A building natural key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuildingSearchKey {
    /// Municipality-part code.
    pub municipality_part: MunicipalityPartCode,
    /// Building-type code.
    pub building_type: BuildingTypeCode,
    /// House number in 1–9999.
    pub house_number: u16,
}

impl BuildingSearchKey {
    /// Validates the house-number range from `CisloDomovniType`.
    pub fn new(
        municipality_part: MunicipalityPartCode,
        building_type: BuildingTypeCode,
        house_number: u16,
    ) -> Result<Self> {
        if !(1..=9999).contains(&house_number) {
            return Err(Error::invalid_input(
                "house_number",
                "house number must be in 1..=9999",
            ));
        }
        Ok(Self {
            municipality_part,
            building_type,
            house_number,
        })
    }
}

/// `najdiStavbu` request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuildingSearch {
    /// ID/natural-key choice.
    pub selector: BuildingSearchSelector,
    /// Historical timestamp; `None` requests current server data.
    pub valid_at: Option<XmlDateTime>,
}

/// Unit lookup by ID or by building and optional unit number.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UnitSearchSelector {
    /// Internal unit ID.
    Id(UnitId),
    /// Building ID/natural key and an optional unit number.
    Number(UnitSearchKey),
}

/// A unit natural key. Omitting `number` returns units in the selected building(s).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnitSearchKey {
    /// Building ID or natural key.
    pub building: BuildingSearchSelector,
    /// Unit number in 0–9999999999, or all units when omitted.
    pub number: Option<u64>,
}

impl UnitSearchKey {
    /// Validates the optional unit-number range from `CisloJednotkyType`.
    pub fn new(building: BuildingSearchSelector, number: Option<u64>) -> Result<Self> {
        if number.is_some_and(|value| value > 9_999_999_999) {
            return Err(Error::invalid_input(
                "unit_number",
                "unit number must be at most 9999999999",
            ));
        }
        Ok(Self { building, number })
    }
}

/// `najdiJednotku` request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnitSearch {
    /// ID/natural-key choice.
    pub selector: UnitSearchSelector,
    /// Historical timestamp; `None` requests current server data.
    pub valid_at: Option<XmlDateTime>,
}

/// Building-right lookup by its own ID or by a parcel selector.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BuildingRightSelector {
    /// Internal building-right ID.
    Id(BuildingRightId),
    /// Parcel selected by internal ID or natural key.
    Parcel(ParcelSearchSelector),
}

/// `najdiPravoStavby` request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuildingRightSearch {
    /// Building-right/parcel-ID/parcel-natural-key choice.
    pub selector: BuildingRightSelector,
    /// Historical timestamp; `None` requests current server data.
    pub valid_at: Option<XmlDateTime>,
}

/// A proceeding natural key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProceedingSearchKey {
    /// Dynamic proceeding-type code (1–3 characters).
    pub proceeding_type: String,
    /// Serial number in 1–99999999.
    pub serial_number: u32,
    /// Proceeding year in 1–9999.
    pub year: u16,
    /// Cadastral-workplace code.
    pub workplace: WorkplaceCode,
}

impl ProceedingSearchKey {
    /// Validates the proceeding natural-key fields.
    pub fn new(
        proceeding_type: impl Into<String>,
        serial_number: u32,
        year: u16,
        workplace: WorkplaceCode,
    ) -> Result<Self> {
        let proceeding_type = proceeding_type.into();
        if !(1..=3).contains(&proceeding_type.chars().count()) {
            return Err(Error::invalid_input(
                "proceeding_type",
                "proceeding type must contain 1 to 3 characters",
            ));
        }
        if !(1..=99_999_999).contains(&serial_number) {
            return Err(Error::invalid_input(
                "serial_number",
                "proceeding serial number must be in 1..=99999999",
            ));
        }
        if !(1..=9999).contains(&year) {
            return Err(Error::invalid_input(
                "year",
                "proceeding year must be in 1..=9999",
            ));
        }
        Ok(Self {
            proceeding_type,
            serial_number,
            year,
            workplace,
        })
    }
}

/// Proceeding lookup by internal ID or natural key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProceedingSearchSelector {
    /// Internal proceeding ID.
    Id(ProceedingId),
    /// Type, serial number, year, and workplace.
    Number(ProceedingSearchKey),
}

/// `najdiRizeni` request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProceedingSearch {
    /// ID/natural-key choice.
    pub selector: ProceedingSearchSelector,
}

/// `najdiListinuVeSbirceListin` request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollectionDocumentSearch {
    /// Proceeding ID/natural-key choice.
    pub proceeding: ProceedingSearchSelector,
}

/// `najdiGP` request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeometricPlanSearch {
    /// Cadastral-area code.
    pub cadastral_area: CadastralAreaCode,
    /// Survey-result or field-sketch number in -99999–99999.
    pub survey_result_number: i32,
}

impl GeometricPlanSearch {
    /// Validates the signed survey-result number.
    pub fn new(cadastral_area: CadastralAreaCode, survey_result_number: i32) -> Result<Self> {
        if !(-99_999..=99_999).contains(&survey_result_number) {
            return Err(Error::invalid_input(
                "survey_result_number",
                "survey-result number must be in -99999..=99999",
            ));
        }
        Ok(Self {
            cadastral_area,
            survey_result_number,
        })
    }
}

/// Legal-person subject search. At least one of company ID and name is required.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LegalPersonSearch {
    /// Company identification number.
    pub company_id: Option<CompanyId>,
    /// Legal-person name, using the exact server matching semantics.
    pub name: Option<String>,
}

impl LegalPersonSearch {
    /// Validates the mandatory-at-least-one rule and search-name length.
    pub fn new(company_id: Option<CompanyId>, name: Option<String>) -> Result<Self> {
        if company_id.is_none() && name.is_none() {
            return Err(Error::invalid_input(
                "legal_person",
                "company ID or name is required",
            ));
        }
        if name
            .as_deref()
            .is_some_and(|value| !(2..=255).contains(&value.chars().count()))
        {
            return Err(Error::invalid_input(
                "name",
                "legal-person search name must contain 2 to 255 characters",
            ));
        }
        Ok(Self { company_id, name })
    }
}

/// Personal identifier or birth-date choice for natural-person/partnership search.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PersonBirthSelector {
    /// Full personal identifier; debug output is redacted by its type.
    PersonalIdentifier(PersonalIdentifier),
    /// Birth date.
    BirthDate(XmlDate),
}

/// Address filters available for natural-person and partnership lookup.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SubjectAddressFilter {
    /// Municipality name.
    pub municipality: Option<String>,
    /// Municipality-part name.
    pub municipality_part: Option<String>,
    /// Street name.
    pub street: Option<String>,
    /// Orientation number, preserved as text.
    pub orientation_number: Option<String>,
    /// House number in 1–9999.
    pub house_number: Option<u16>,
}

/// Natural-person or marital/partner joint-property subject search.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PersonOrPartnershipSearch {
    /// Given name, with server-defined matching semantics.
    pub given_name: Option<String>,
    /// Surname, with server-defined matching semantics.
    pub surname: Option<String>,
    /// Personal identifier or birth date.
    pub birth: Option<PersonBirthSelector>,
    /// Optional address filters.
    pub address: Option<SubjectAddressFilter>,
}

/// Legal-person versus natural-person/partnership subject-search choice.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SubjectSearchSelector {
    /// Search by IČO and/or legal-person name.
    LegalPerson(LegalPersonSearch),
    /// Search natural persons and marital/partner joint-property subjects.
    PersonOrPartnership(PersonOrPartnershipSearch),
}

/// `najdiOS` request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubjectSearch {
    /// Subject-form choice.
    pub selector: SubjectSearchSelector,
    /// Historical timestamp; `None` requests current server data.
    pub valid_at: Option<XmlDateTime>,
}

/// One of the eight WSDP 3.1 `vyhledat` requests.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SearchRequest {
    /// `najdiParcelu`.
    Parcel(ParcelSearch),
    /// `najdiJednotku`.
    Unit(UnitSearch),
    /// `najdiStavbu`.
    Building(BuildingSearch),
    /// `najdiOS`.
    Subject(SubjectSearch),
    /// `najdiPravoStavby`.
    BuildingRight(BuildingRightSearch),
    /// `najdiRizeni`.
    Proceeding(ProceedingSearch),
    /// `najdiListinuVeSbirceListin`.
    CollectionDocument(CollectionDocumentSearch),
    /// `najdiGP`.
    GeometricPlan(GeometricPlanSearch),
}

/// Whether a search result is complete, empty, or truncated by the service limit.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SearchCompletion {
    /// The server returned a complete, non-empty result.
    Complete,
    /// No matching data was returned.
    Empty,
    /// Message 310 reports that the service result limit was exceeded.
    LimitExceeded,
}

/// Search items and their service-derived completion state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SearchResult<T> {
    /// Returned records. A natural-key lookup remains a collection even for one match.
    pub items: Vec<T>,
    /// Completion state refined from service messages by the client integration.
    pub completion: SearchCompletion,
}

impl<T> SearchResult<T> {
    /// Creates the response-only state; non-empty items are complete until refined.
    pub fn from_items(items: Vec<T>) -> Self {
        let completion = if items.is_empty() {
            SearchCompletion::Empty
        } else {
            SearchCompletion::Complete
        };
        Self { items, completion }
    }

    /// Applies message 310 and 392 semantics without inventing pagination.
    pub fn refine_completion(&mut self, messages: &[ServiceMessage]) {
        self.completion = if messages
            .iter()
            .any(|message| message.code == MessageCode::ResultLimitExceeded)
        {
            SearchCompletion::LimitExceeded
        } else if self.items.is_empty()
            || messages
                .iter()
                .any(|message| message.code == MessageCode::NoData)
        {
            SearchCompletion::Empty
        } else {
            SearchCompletion::Complete
        };
    }
}

/// Typed result of one of the eight search operations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SearchResponse {
    /// Parcel records.
    Parcels(SearchResult<Parcel>),
    /// Unit records.
    Units(SearchResult<Unit>),
    /// Building records.
    Buildings(SearchResult<Building>),
    /// Authorized-subject records.
    Subjects(SearchResult<AuthorizedSubject>),
    /// Building-right records.
    BuildingRights(SearchResult<BuildingRight>),
    /// Proceeding records.
    Proceedings(SearchResult<Proceeding>),
    /// Collection-document records.
    CollectionDocuments(SearchResult<CollectionDocument>),
    /// Geometric-plan identifiers.
    GeometricPlans(SearchResult<GeometricPlan>),
}

impl SearchResponse {
    /// Refines every search completion state from the messages retained by `Response`.
    pub fn refine_completion(&mut self, messages: &[ServiceMessage]) {
        match self {
            Self::Parcels(result) => result.refine_completion(messages),
            Self::Units(result) => result.refine_completion(messages),
            Self::Buildings(result) => result.refine_completion(messages),
            Self::Subjects(result) => result.refine_completion(messages),
            Self::BuildingRights(result) => result.refine_completion(messages),
            Self::Proceedings(result) => result.refine_completion(messages),
            Self::CollectionDocuments(result) => result.refine_completion(messages),
            Self::GeometricPlans(result) => result.refine_completion(messages),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MessageLevel, ServiceMessage};

    #[test]
    fn xml_date_retains_naive_and_offset_forms() {
        for raw in ["2026-11-20", "2026-11-20Z", "2026-11-20+01:00"] {
            assert_eq!(XmlDate::new(raw).unwrap().as_str(), raw);
        }
        assert!(XmlDate::new("20.11.2026").is_err());
        assert!(XmlDate::new("2026-11-20+15:00").is_err());
    }

    #[test]
    fn completion_uses_limit_and_no_data_messages() {
        let mut result = SearchResult::from_items(vec![1]);
        result.refine_completion(&[ServiceMessage {
            code: MessageCode::ResultLimitExceeded,
            level: MessageLevel::Warning,
            text: String::new(),
        }]);
        assert_eq!(result.completion, SearchCompletion::LimitExceeded);
        result.refine_completion(&[]);
        assert_eq!(result.completion, SearchCompletion::Complete);
        result.items.clear();
        result.refine_completion(&[]);
        assert_eq!(result.completion, SearchCompletion::Empty);
    }

    #[test]
    fn personal_identifiers_are_debug_redacted() {
        let short = ShortPersonalIdentifier::new("123456").unwrap();
        assert!(!format!("{short:?}").contains("123456"));
        let full = PersonalIdentifier::new("123456789").unwrap();
        let selector = PersonBirthSelector::PersonalIdentifier(full);
        assert!(!format!("{selector:?}").contains("123456789"));
    }
}
