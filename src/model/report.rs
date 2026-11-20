use std::collections::HashSet;
use std::fmt;
use std::str::FromStr;
use std::time::Duration;

use bytes::Bytes;
use rust_decimal::Decimal;

use super::common::{ServiceMessage, XmlDateTime};
use super::id::{
    BuildingId, BuildingRightId, CadastralAreaCode, Czk, DocumentId, MunicipalityCode,
    MunicipalityPartCode, OwnershipSheetId, ParcelId, ParcelNumber, ProceedingId, RegionCode,
    ReportId, SubjectId, UnitId,
};
use super::legal_relation::LegalRelationCode;
use crate::{Error, Result};

/// Versioned WSDP XML report family.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum ReportFamily {
    /// Prices by properties.
    PricesByProperties,
    /// Prices by proceeding.
    PricesByProceeding,
    /// Rights record for a person.
    RightsRecord,
    /// Unit information.
    UnitInfo,
    /// Parcel information.
    ParcelInfo,
    /// Building-right information.
    BuildingRightInfo,
    /// Proceeding information.
    ProceedingInfo,
    /// Building information.
    BuildingInfo,
    /// Ownership overview.
    OwnershipOverview,
    /// Ownership overview with property details.
    OwnershipOverviewWithProperties,
    /// Definition-point list.
    DefinitionPoints,
    /// Customer account statement.
    AccountStatement,
    /// Legacy or NTR ownership sheet.
    OwnershipSheet,
    /// A future server-maintained family code.
    Other(String),
}

impl ReportFamily {
    pub(crate) const ALL: &'static [Self] = &[
        Self::PricesByProperties,
        Self::PricesByProceeding,
        Self::RightsRecord,
        Self::UnitInfo,
        Self::ParcelInfo,
        Self::BuildingRightInfo,
        Self::ProceedingInfo,
        Self::BuildingInfo,
        Self::OwnershipOverview,
        Self::OwnershipOverviewWithProperties,
        Self::DefinitionPoints,
        Self::AccountStatement,
        Self::OwnershipSheet,
    ];

    /// Returns the official report-family code.
    pub fn as_str(&self) -> &str {
        match self {
            Self::PricesByProperties => "CenoveUdajeDleNemovitosti",
            Self::PricesByProceeding => "CenoveUdajeDleRizeni",
            Self::RightsRecord => "EvidencePravProOsobu",
            Self::UnitInfo => "InformaceOJednotkach",
            Self::ParcelInfo => "InformaceOParcelach",
            Self::BuildingRightInfo => "InformaceOPravechStavby",
            Self::ProceedingInfo => "InformaceORizeni",
            Self::BuildingInfo => "InformaceOStavbach",
            Self::OwnershipOverview => "PrehledVlastnictvi",
            Self::OwnershipOverviewWithProperties => "PrehledVlastnictviSNemovitostmi",
            Self::DefinitionPoints => "SeznamDefinicnichBoduParcel",
            Self::AccountStatement => "VypisStavuZakaznickehoUctu",
            Self::OwnershipSheet => "VypisZKatastruNemovitosti",
            Self::Other(raw) => raw,
        }
    }

    /// Returns the version pinned by default for this family.
    ///
    /// Ownership sheets default to NTR 3.0; every other currently known XML
    /// family defaults to legacy XML 1.0. Unknown future families have no safe
    /// client-selected default and therefore return `None`.
    pub fn default_version(&self) -> Option<ReportVersion> {
        match self {
            Self::Other(_) => None,
            Self::OwnershipSheet => Some(ReportVersion::V3_0),
            _ => Some(ReportVersion::V1_0),
        }
    }

    pub(crate) fn from_raw(raw: impl Into<String>) -> Self {
        let raw = raw.into();
        Self::ALL
            .iter()
            .find(|family| family.as_str() == raw)
            .cloned()
            .unwrap_or(Self::Other(raw))
    }
}

impl fmt::Display for ReportFamily {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Lexical `MAJOR.MINOR` XML report version.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum ReportVersion {
    /// Legacy XML version 1.0.
    V1_0,
    /// NTR ownership-sheet version 3.0.
    V3_0,
    /// A runtime-discovered or future version.
    Other(String),
}

impl ReportVersion {
    /// Validates a version while retaining future values.
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        let mut parts = value.split('.');
        let valid =
            parts.next().is_some_and(|part| {
                !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit())
            }) && parts.next().is_some_and(|part| {
                !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit())
            }) && parts.next().is_none();
        if !valid {
            return Err(Error::invalid_input(
                "report_version",
                "expected MAJOR.MINOR decimal digits",
            ));
        }
        Ok(match value.as_str() {
            "1.0" => Self::V1_0,
            "3.0" => Self::V3_0,
            _ => Self::Other(value),
        })
    }

    /// Returns the lexical version without decimal conversion.
    pub fn as_str(&self) -> &str {
        match self {
            Self::V1_0 => "1.0",
            Self::V3_0 => "3.0",
            Self::Other(raw) => raw,
        }
    }
}

impl FromStr for ReportVersion {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        Self::new(value)
    }
}

impl fmt::Display for ReportVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Versions currently enabled for one report family.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct ReportVersionInfo {
    /// Official family code.
    pub family: ReportFamily,
    /// Human-readable server-supplied name.
    pub name: String,
    /// Enabled versions in server order.
    pub versions: Vec<ReportVersion>,
}

/// Selects the XML version sent in a generation request.
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub enum ReportVersionSelection {
    /// Resolve and explicitly pin the family's documented default version.
    #[default]
    DefaultForFamily,
    /// Explicitly pin a named or runtime-discovered version.
    Pinned(ReportVersion),
    /// Omit `verze` and let the server choose its latest available version.
    LatestAvailable,
}

/// Output format accepted by generation operations with a format choice.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum OutputFormat {
    /// Formatted PDF output.
    Pdf,
    /// Data-oriented XML output.
    Xml,
}

impl OutputFormat {
    /// Returns the official lowercase wire value.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pdf => "pdf",
            Self::Xml => "xml",
        }
    }
}

/// Account-specific authorization fields supplied during generation.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum AccountContext {
    /// An ordinary paying account; no additional XML fields are sent.
    Paying,
    /// A free-of-charge account requiring a file reference and purpose code.
    Free {
        /// `cisloJednaci`, containing 1–50 characters.
        cislo_jednaci: String,
        /// `ucelKod`, in the range 1–99.
        ucel_kod: u8,
    },
    /// A verifying account requiring an ISVS sequence number and clause choice.
    Verifying {
        /// `poradoveCislo`, containing 1–40 characters.
        poradove_cislo: String,
        /// Whether `overovaciDolozka` is requested.
        overovaci_dolozka: bool,
    },
}

impl AccountContext {
    fn validate(&self) -> Result<()> {
        match self {
            Self::Paying => Ok(()),
            Self::Free {
                cislo_jednaci,
                ucel_kod,
            } => {
                validate_character_length("cislo_jednaci", cislo_jednaci, 1, 50)?;
                if !(1..=99).contains(ucel_kod) {
                    return Err(Error::invalid_input(
                        "ucel_kod",
                        "report purpose code must be in 1..=99",
                    ));
                }
                Ok(())
            }
            Self::Verifying { poradove_cislo, .. } => {
                validate_character_length("poradove_cislo", poradove_cislo, 1, 40)
            }
        }
    }

    fn is_verifying(&self) -> bool {
        matches!(self, Self::Verifying { .. })
    }
}

/// Output and account options shared by generation operations using
/// `SestavyBaseType`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReportOptions {
    /// Requested primary output format.
    pub format: OutputFormat,
    /// XML version selection. The default is resolved per report family.
    pub version: ReportVersionSelection,
    /// Requests a related XML report alongside an eligible PDF report.
    pub attach_xml: bool,
    /// Account-specific generation authorization.
    pub account: AccountContext,
}

impl ReportOptions {
    /// Creates options for validation in the context of a generation request.
    pub fn new(
        format: OutputFormat,
        version: ReportVersionSelection,
        attach_xml: bool,
        account: AccountContext,
    ) -> Self {
        Self {
            format,
            version,
            attach_xml,
            account,
        }
    }

    /// Selects XML and explicitly pins the family-specific default on the wire.
    pub fn xml_default(account: AccountContext) -> Self {
        Self::new(
            OutputFormat::Xml,
            ReportVersionSelection::DefaultForFamily,
            false,
            account,
        )
    }

    /// Selects XML with an explicitly pinned version.
    pub fn xml(version: ReportVersion, account: AccountContext) -> Self {
        Self::new(
            OutputFormat::Xml,
            ReportVersionSelection::Pinned(version),
            false,
            account,
        )
    }

    /// Selects raw XML while allowing the server to choose its latest version.
    pub fn xml_latest(account: AccountContext) -> Self {
        Self::new(
            OutputFormat::Xml,
            ReportVersionSelection::LatestAvailable,
            false,
            account,
        )
    }

    /// Selects PDF with the family default pinned for any associated XML data.
    pub fn pdf(account: AccountContext) -> Self {
        Self::new(
            OutputFormat::Pdf,
            ReportVersionSelection::DefaultForFamily,
            false,
            account,
        )
    }

    /// Selects PDF and requests a related XML report using the family default.
    pub fn pdf_with_xml(account: AccountContext) -> Self {
        Self::new(
            OutputFormat::Pdf,
            ReportVersionSelection::DefaultForFamily,
            true,
            account,
        )
    }

    fn validate(
        &self,
        family: &ReportFamily,
        xml_only: bool,
        verifying_allowed: bool,
    ) -> Result<()> {
        self.account.validate()?;
        if self.account.is_verifying() && !verifying_allowed {
            return Err(Error::invalid_input(
                "account",
                "this generation operation is not available to verifying accounts",
            ));
        }
        if self.account.is_verifying() && self.format != OutputFormat::Pdf {
            return Err(Error::invalid_input(
                "format",
                "verifying accounts may request only PDF output",
            ));
        }
        if xml_only && self.format != OutputFormat::Xml {
            return Err(Error::invalid_input(
                "format",
                "this generation operation supports only XML output",
            ));
        }
        if self.attach_xml && self.format != OutputFormat::Pdf {
            return Err(Error::invalid_input(
                "attach_xml",
                "attached XML requires a PDF primary output",
            ));
        }
        if self.attach_xml && self.account.is_verifying() {
            return Err(Error::invalid_input(
                "attach_xml",
                "verifying accounts cannot request attached XML output",
            ));
        }
        if self.version == ReportVersionSelection::DefaultForFamily
            && family.default_version().is_none()
        {
            return Err(Error::invalid_input(
                "version",
                "no family-specific default version is known",
            ));
        }
        Ok(())
    }
}

/// Natural parcel key used when an internal parcel ID is unavailable.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ParcelNaturalKey {
    /// Cadastral-area code.
    pub cadastral_area: CadastralAreaCode,
    /// Root number and optional subdivision. Omitted subdivision is not a wildcard.
    pub number: ParcelNumber,
}

/// Selects a parcel by internal ID or natural key.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum ParcelSelector {
    /// Internal parcel ID (`parcelaId`).
    Id(ParcelId),
    /// Cadastral area and parcel number.
    NaturalKey(ParcelNaturalKey),
}

/// Building numbering kind (`typStavbyKod`).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum BuildingNumberKind {
    /// Registration number (`číslo popisné`, code 1).
    Registration,
    /// Evidence number (`číslo evidenční`, code 2).
    Evidence,
}

impl BuildingNumberKind {
    /// Returns the official numeric wire code.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Registration => "1",
            Self::Evidence => "2",
        }
    }
}

macro_rules! report_number {
    ($name:ident, $integer:ty, $minimum:expr, $maximum:expr, $doc:literal) => {
        #[doc = $doc]
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name($integer);

        impl $name {
            /// Validates the official inclusive numeric range.
            pub fn new(value: $integer) -> Result<Self> {
                if !($minimum..=$maximum).contains(&value) {
                    return Err(Error::invalid_input(
                        "number",
                        concat!(
                            "value is outside the official range for ",
                            stringify!($name)
                        ),
                    ));
                }
                Ok(Self(value))
            }

            /// Returns the numeric value without changing its meaning.
            pub fn get(self) -> $integer {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(formatter)
            }
        }
    };
}

report_number!(
    BuildingNumber,
    u16,
    1,
    9_999,
    "A building registration or evidence number in the range 1–9999."
);
report_number!(
    UnitNumber,
    u64,
    0,
    9_999_999_999,
    "A unit number in the official range 0–9999999999."
);
report_number!(
    OwnershipSheetNumber,
    u32,
    1,
    999_999,
    "An ownership-sheet number in the official range 1–999999."
);

/// Natural building key used by building and unit report requests.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct BuildingNaturalKey {
    /// Municipality-part code (`castObceKod`).
    pub municipality_part: MunicipalityPartCode,
    /// Registration versus evidence numbering.
    pub number_kind: BuildingNumberKind,
    /// Building number within the selected numbering sequence.
    pub number: BuildingNumber,
}

/// Selects a building by internal ID or natural key.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum BuildingSelector {
    /// Internal building ID (`stavbaId`).
    Id(BuildingId),
    /// Municipality part, numbering kind, and building number.
    NaturalKey(BuildingNaturalKey),
}

/// Natural unit key, including its containing building.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct UnitNaturalKey {
    /// Natural key of the containing building.
    pub building: BuildingNaturalKey,
    /// Unit number (`cisloJednotky`).
    pub unit_number: UnitNumber,
}

/// Selects a unit by internal ID or natural key.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum UnitSelector {
    /// Internal unit ID (`jednotkaId`).
    Id(UnitId),
    /// Building natural key and unit number.
    NaturalKey(UnitNaturalKey),
}

/// A proceeding number in the official `XXX-čččččččč/RRRR-ččč` shape.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ProceedingNumber(String);

impl ProceedingNumber {
    /// Validates the XSD lexical pattern without converting numeric components.
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        if valid_proceeding_number(&value) {
            Ok(Self(value))
        } else {
            Err(Error::invalid_input(
                "proceeding_number",
                "expected 1–3 letters, 1–8 digits, a four-digit year, and 2–3 workplace digits",
            ))
        }
    }

    /// Returns the original proceeding number.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ProceedingNumber {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for ProceedingNumber {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        Self::new(value)
    }
}

fn valid_proceeding_number(value: &str) -> bool {
    let Some((kind, remainder)) = value.split_once('-') else {
        return false;
    };
    let Some((sequence, remainder)) = remainder.split_once('/') else {
        return false;
    };
    let Some((year, workplace)) = remainder.split_once('-') else {
        return false;
    };
    (1..=3).contains(&kind.chars().count())
        && kind.chars().all(char::is_alphabetic)
        && (1..=8).contains(&sequence.len())
        && sequence.bytes().all(|byte| byte.is_ascii_digit())
        && year.len() == 4
        && year.bytes().all(|byte| byte.is_ascii_digit())
        && (2..=3).contains(&workplace.len())
        && workplace.bytes().all(|byte| byte.is_ascii_digit())
}

/// Selects a proceeding by internal ID or full proceeding number.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum ProceedingSelector {
    /// Internal proceeding ID (`rizeniId`).
    Id(ProceedingId),
    /// Full proceeding number (`rizeniCislo`).
    Number(ProceedingNumber),
}

/// A homogeneous list of property identifiers used by price and LV reports.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReportObjectList {
    /// Parcel IDs, serialized with type `PAR_S`.
    Parcels(Vec<ParcelId>),
    /// Building IDs, serialized with type `STA_S`.
    Buildings(Vec<BuildingId>),
    /// Unit IDs, serialized with type `JED_S`.
    Units(Vec<UnitId>),
    /// Building-right IDs, serialized with type `PRA_S`.
    BuildingRights(Vec<BuildingRightId>),
}

impl ReportObjectList {
    /// Returns the official `TypSeznamuObjektuType` code.
    pub fn kind_code(&self) -> &'static str {
        match self {
            Self::Parcels(_) => "PAR_S",
            Self::Buildings(_) => "STA_S",
            Self::Units(_) => "JED_S",
            Self::BuildingRights(_) => "PRA_S",
        }
    }

    /// Returns the number of selected object IDs.
    pub fn len(&self) -> usize {
        match self {
            Self::Parcels(ids) => ids.len(),
            Self::Buildings(ids) => ids.len(),
            Self::Units(ids) => ids.len(),
            Self::BuildingRights(ids) => ids.len(),
        }
    }

    /// Returns whether no object IDs were supplied.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn validate(&self) -> Result<()> {
        if self.is_empty() {
            return Err(Error::invalid_input(
                "objects",
                "the report object list cannot be empty",
            ));
        }
        let mut unique = HashSet::with_capacity(self.len());
        let no_duplicates = match self {
            Self::Parcels(ids) => ids.iter().all(|id| unique.insert(id.as_str())),
            Self::Buildings(ids) => ids.iter().all(|id| unique.insert(id.as_str())),
            Self::Units(ids) => ids.iter().all(|id| unique.insert(id.as_str())),
            Self::BuildingRights(ids) => ids.iter().all(|id| unique.insert(id.as_str())),
        };
        if no_duplicates {
            Ok(())
        } else {
            Err(Error::invalid_input(
                "objects",
                "the report object list contains duplicate identifiers",
            ))
        }
    }
}

/// Exact S-JTSK map coordinates in the XSD's X/Y field order.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MapCoordinates {
    /// `souradniceX`, with at most nine total and two fractional digits.
    pub x: Decimal,
    /// `souradniceY`, with at most eight total and two fractional digits.
    pub y: Decimal,
}

impl MapCoordinates {
    /// Creates coordinates after applying the XSD decimal limits.
    pub fn new(x: Decimal, y: Decimal) -> Result<Self> {
        let coordinates = Self { x, y };
        coordinates.validate()?;
        Ok(coordinates)
    }

    fn validate(&self) -> Result<()> {
        validate_decimal("x", self.x, 9, 2)?;
        validate_decimal("y", self.y, 8, 2)
    }
}

fn validate_decimal(field: &'static str, value: Decimal, digits: usize, scale: u32) -> Result<()> {
    let digits_used = value.mantissa().unsigned_abs().to_string().len();
    if value.scale() > scale || digits_used > digits {
        return Err(Error::invalid_input(
            field,
            "decimal exceeds the official total-digit or fractional-digit limit",
        ));
    }
    Ok(())
}

/// Kind of definition point sought around map coordinates.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DefinitionPointKind {
    /// Parcel definition point (`p`).
    Parcel,
    /// Building definition point (`s`).
    Building,
}

impl DefinitionPointKind {
    /// Returns the official wire code.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Parcel => "p",
            Self::Building => "s",
        }
    }
}

/// Coordinate-based definition-point query.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DefinitionPointQuery {
    /// Center coordinates.
    pub coordinates: MapCoordinates,
    /// Optional search distance in metres. Omission selects the nearest point.
    pub distance: Option<Decimal>,
    /// Parcel or building point kind.
    pub kind: DefinitionPointKind,
}

impl DefinitionPointQuery {
    fn validate(&self) -> Result<()> {
        self.coordinates.validate()?;
        if let Some(distance) = self.distance {
            validate_decimal("distance", distance, 8, 2)?;
        }
        Ok(())
    }
}

/// Selects objects for the definition-point report.
#[derive(Clone, Debug, PartialEq)]
pub enum DefinitionPointsSelector {
    /// All definition points in a cadastral area.
    CadastralArea(CadastralAreaCode),
    /// Definition points for one parcel.
    Parcel(ParcelId),
    /// Definition points for one building.
    Building(BuildingId),
    /// Definition points around exact coordinates.
    Coordinates(DefinitionPointQuery),
}

/// Four official ways to request one or more ownership sheets.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OwnershipSheetSelector {
    /// `generujLV`: internal ownership-sheet ID.
    Id(OwnershipSheetId),
    /// `generujLVZjednodusene`: cadastral area and LV number.
    Number {
        /// Cadastral-area code.
        cadastral_area: CadastralAreaCode,
        /// Ownership-sheet number.
        number: OwnershipSheetNumber,
    },
    /// `generujLVPresOS`: all sheets for a subject in a cadastral area.
    Subject {
        /// Cadastral-area code.
        cadastral_area: CadastralAreaCode,
        /// Internal authorized-subject ID.
        subject_id: SubjectId,
    },
    /// `generujLVPresObjekty`: sheets selected through a homogeneous object list.
    Objects {
        /// Parcel, building, unit, or building-right IDs.
        objects: ReportObjectList,
        /// `castecnyVypis`; `false` requests a complete sheet.
        partial: bool,
    },
}

/// Optional territorial restriction for ownership-overview reports.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum OwnershipOverviewTerritory {
    /// Region code (`krajKod`).
    Region(RegionCode),
    /// Municipality code (`obecKod`).
    Municipality(MunicipalityCode),
}

macro_rules! selected_report_request {
    ($name:ident, $selector:ty, $doc:literal) => {
        #[doc = $doc]
        #[derive(Clone, Debug, PartialEq)]
        pub struct $name {
            /// Operation-specific object selector.
            pub selector: $selector,
            /// Optional historical timestamp (`datumK`).
            pub as_of: Option<XmlDateTime>,
            /// Output and account options.
            pub options: ReportOptions,
        }

        impl $name {
            /// Creates a current-data request. Use the public `as_of` field for a
            /// historical request.
            pub fn new(selector: $selector, options: ReportOptions) -> Self {
                Self {
                    selector,
                    as_of: None,
                    options,
                }
            }
        }
    };
}

selected_report_request!(
    ParcelInfoRequest,
    ParcelSelector,
    "Request for `generujInfoOParcelach`."
);
selected_report_request!(
    BuildingInfoRequest,
    BuildingSelector,
    "Request for `generujInfoOStavbach`."
);
selected_report_request!(
    UnitInfoRequest,
    UnitSelector,
    "Request for `generujInfoOJednotkach`."
);

/// Request for `generujInfoOPravuStavby`.
#[derive(Clone, Debug, PartialEq)]
pub struct BuildingRightInfoRequest {
    /// Internal building-right ID.
    pub building_right_id: BuildingRightId,
    /// Optional historical timestamp (`datumK`).
    pub as_of: Option<XmlDateTime>,
    /// Output and account options.
    pub options: ReportOptions,
}

impl BuildingRightInfoRequest {
    /// Creates a current-data building-right report request.
    pub fn new(building_right_id: BuildingRightId, options: ReportOptions) -> Self {
        Self {
            building_right_id,
            as_of: None,
            options,
        }
    }
}

/// Request for `generujInfoORizeni` or `generujCenoveUdajeDleRizeni`.
#[derive(Clone, Debug, PartialEq)]
pub struct ProceedingInfoRequest {
    /// Proceeding ID or full number.
    pub selector: ProceedingSelector,
    /// Output and account options.
    pub options: ReportOptions,
}

impl ProceedingInfoRequest {
    /// Creates a proceeding-information request.
    pub fn new(selector: ProceedingSelector, options: ReportOptions) -> Self {
        Self { selector, options }
    }
}

/// Request for the XML-only `generujInfoOBodech` operation.
#[derive(Clone, Debug, PartialEq)]
pub struct DefinitionPointsRequest {
    /// Cadastral area, object ID, or coordinate query.
    pub selector: DefinitionPointsSelector,
    /// Output and account options; validation requires XML and no attached XML.
    pub options: ReportOptions,
}

impl DefinitionPointsRequest {
    /// Creates a definition-point request.
    pub fn new(selector: DefinitionPointsSelector, options: ReportOptions) -> Self {
        Self { selector, options }
    }
}

/// Request covering all four official LV generation operations.
#[derive(Clone, Debug, PartialEq)]
pub struct OwnershipSheetRequest {
    /// ID, natural number, subject, or object-list selection.
    pub selector: OwnershipSheetSelector,
    /// Optional historical timestamp (`datumK`).
    pub as_of: Option<XmlDateTime>,
    /// Output and account options. The default version is NTR 3.0.
    pub options: ReportOptions,
}

impl OwnershipSheetRequest {
    /// Creates a current-data ownership-sheet request.
    pub fn new(selector: OwnershipSheetSelector, options: ReportOptions) -> Self {
        Self {
            selector,
            as_of: None,
            options,
        }
    }
}

/// Request shared by ownership overview with and without property details.
#[derive(Clone, Debug, PartialEq)]
pub struct OwnershipOverviewRequest {
    /// Internal authorized-subject ID (`osId`). The XSD requires this field.
    pub subject_id: SubjectId,
    /// Optional region or municipality restriction.
    pub territory: Option<OwnershipOverviewTerritory>,
    /// Optional legal-relation code (`typPravnihoVztahuKod`).
    pub legal_relation: Option<LegalRelationCode>,
    /// Optional historical timestamp (`datumK`).
    pub as_of: Option<XmlDateTime>,
    /// Output and account options.
    pub options: ReportOptions,
}

impl OwnershipOverviewRequest {
    /// Creates an unrestricted current-data overview for a subject.
    pub fn new(subject_id: SubjectId, options: ReportOptions) -> Self {
        Self {
            subject_id,
            territory: None,
            legal_relation: None,
            as_of: None,
            options,
        }
    }
}

/// Natural-person or legal-person kind accepted by the rights-record report.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RightsRecordSubjectKind {
    /// Natural person (`OFO`).
    NaturalPerson,
    /// Legal person (`OPO`).
    LegalPerson,
}

impl RightsRecordSubjectKind {
    /// Returns the official subject-kind code.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NaturalPerson => "OFO",
            Self::LegalPerson => "OPO",
        }
    }
}

/// The rights-record `rcIco` value, which may contain a personal identifier,
/// birth-date-shaped value, or company ID.
#[derive(Clone, Eq, Hash, PartialEq)]
pub struct RightsRecordIdentifier(String);

impl RightsRecordIdentifier {
    /// Validates the XSD's 1–10 character constraint.
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        validate_character_length("rc_ico", &value, 1, 10)?;
        Ok(Self(value))
    }

    /// Explicitly exposes the identifier for request serialization.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for RightsRecordIdentifier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("RightsRecordIdentifier([REDACTED])")
    }
}

/// Request for `generujEvidenciPravProOsobu`.
#[derive(Clone, Debug, PartialEq)]
pub struct RightsRecordRequest {
    /// Natural or legal person (`typ`).
    pub subject_kind: RightsRecordSubjectKind,
    /// Person or organization name (`nazev`), 1–255 characters.
    pub name: String,
    /// Personal/company identifier (`rcIco`), redacted from `Debug`.
    pub identifier: RightsRecordIdentifier,
    /// Output and account options.
    pub options: ReportOptions,
}

impl RightsRecordRequest {
    /// Creates a rights-record request.
    pub fn new(
        subject_kind: RightsRecordSubjectKind,
        name: impl Into<String>,
        identifier: RightsRecordIdentifier,
        options: ReportOptions,
    ) -> Self {
        Self {
            subject_kind,
            name: name.into(),
            identifier,
            options,
        }
    }
}

/// Request for `vypisUctu`. This operation intentionally has neither account
/// authorization fields nor `pripojitXML`.
#[derive(Clone, Debug, PartialEq)]
pub struct AccountStatementRequest {
    /// Required inclusive start timestamp (`datumOd`).
    pub from: XmlDateTime,
    /// Optional end timestamp (`datumDo`).
    pub to: Option<XmlDateTime>,
    /// PDF or XML output.
    pub format: OutputFormat,
    /// XML version selection; the family default is 1.0.
    pub version: ReportVersionSelection,
}

impl AccountStatementRequest {
    /// Creates an account-statement request using the family default version.
    pub fn new(from: XmlDateTime, format: OutputFormat) -> Self {
        Self {
            from,
            to: None,
            format,
            version: ReportVersionSelection::DefaultForFamily,
        }
    }
}

/// Request for `generujCenoveUdajeDleNemovitosti`.
#[derive(Clone, Debug, PartialEq)]
pub struct PricesByPropertiesRequest {
    /// Homogeneous property ID list.
    pub objects: ReportObjectList,
    /// Optional historical timestamp (`datumK`).
    pub as_of: Option<XmlDateTime>,
    /// Output and account options.
    pub options: ReportOptions,
}

impl PricesByPropertiesRequest {
    /// Creates a current-data price-by-properties request.
    pub fn new(objects: ReportObjectList, options: ReportOptions) -> Self {
        Self {
            objects,
            as_of: None,
            options,
        }
    }
}

/// Request for `generujCenoveUdajeDleRizeni`.
#[derive(Clone, Debug, PartialEq)]
pub struct PricesByProceedingRequest {
    /// Proceeding ID or full number.
    pub selector: ProceedingSelector,
    /// Output and account options.
    pub options: ReportOptions,
}

impl PricesByProceedingRequest {
    /// Creates a price-by-proceeding request.
    pub fn new(selector: ProceedingSelector, options: ReportOptions) -> Self {
        Self { selector, options }
    }
}

/// Request for monthly ZIP output from `generujCenoveUdajeDleKu`.
///
/// The operation has no `format`, `verze`, or `pripojitXML` fields.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PricesByCadastralAreaRequest {
    /// Cadastral-area code.
    pub cadastral_area: CadastralAreaCode,
    /// Year in the official range 1789–9999.
    pub year: u16,
    /// First included month, 1–12.
    pub month_from: u8,
    /// Last included month, 1–12 and not before `month_from`.
    pub month_to: u8,
    /// Account-specific authorization.
    pub account: AccountContext,
}

impl PricesByCadastralAreaRequest {
    /// Creates a monthly price-data request; validation checks all date ranges.
    pub fn new(
        cadastral_area: CadastralAreaCode,
        year: u16,
        month_from: u8,
        month_to: u8,
        account: AccountContext,
    ) -> Self {
        Self {
            cadastral_area,
            year,
            month_from,
            month_to,
            account,
        }
    }
}

/// Paper size for a cadastral-map PDF.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum MapPaperFormat {
    /// A3 paper.
    A3,
    /// A4 paper.
    A4,
}

impl MapPaperFormat {
    /// Returns the official paper-format code.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::A3 => "A3",
            Self::A4 => "A4",
        }
    }
}

/// Permitted cadastral-map denominator.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum MapScale {
    /// 1:500.
    Scale500,
    /// 1:1000.
    Scale1000,
    /// 1:2000.
    Scale2000,
}

impl MapScale {
    /// Returns the official denominator.
    pub fn as_u16(self) -> u16 {
        match self {
            Self::Scale500 => 500,
            Self::Scale1000 => 1_000,
            Self::Scale2000 => 2_000,
        }
    }
}

/// Parcel or coordinate selection for a cadastral map.
#[derive(Clone, Debug, PartialEq)]
pub enum CadastralMapSelector {
    /// Center the map using a parcel's definition point.
    Parcel(ParcelId),
    /// Center the map at explicit S-JTSK coordinates.
    Coordinates(MapCoordinates),
}

/// Request for `generujMapu`.
///
/// The operation always produces PDF and has no `format`, `verze`, or
/// `pripojitXML` fields.
#[derive(Clone, Debug, PartialEq)]
pub struct CadastralMapRequest {
    /// Parcel or coordinate center.
    pub selector: CadastralMapSelector,
    /// A3 or A4 paper.
    pub paper: MapPaperFormat,
    /// `orientaceNaSirku`.
    pub landscape: bool,
    /// Map scale denominator.
    pub scale: MapScale,
    /// Whether to include easements (`vecnaBremena`).
    pub easements: bool,
    /// Optional historical timestamp (`datumK`).
    pub as_of: Option<XmlDateTime>,
    /// Account-specific authorization.
    pub account: AccountContext,
}

/// Request for PDF output from `generujVystupZeSbirkyListin`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollectionDocumentRequest {
    /// Internal deed/document ID (`listinaId`).
    pub document_id: DocumentId,
    /// Account-specific authorization.
    pub account: AccountContext,
}

impl CollectionDocumentRequest {
    /// Creates a collection-document request.
    pub fn new(document_id: DocumentId, account: AccountContext) -> Self {
        Self {
            document_id,
            account,
        }
    }
}

/// Request for PDF output from `generujVystupGP`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeometricPlanRequest {
    /// Internal geometric-plan document ID (`dokumentId`).
    pub document_id: DocumentId,
    /// Account-specific authorization.
    pub account: AccountContext,
}

impl GeometricPlanRequest {
    /// Creates a geometric-plan output request.
    pub fn new(document_id: DocumentId, account: AccountContext) -> Self {
        Self {
            document_id,
            account,
        }
    }
}

/// Semantic request for every official asynchronous report-generation operation.
#[derive(Clone, Debug, PartialEq)]
pub enum GenerateReport {
    /// Any of the four ownership-sheet generation modes.
    OwnershipSheet(OwnershipSheetRequest),
    /// Fixed-format cadastral-map PDF.
    CadastralMap(CadastralMapRequest),
    /// Parcel information.
    ParcelInfo(ParcelInfoRequest),
    /// Building information.
    BuildingInfo(BuildingInfoRequest),
    /// Unit information.
    UnitInfo(UnitInfoRequest),
    /// Building-right information.
    BuildingRightInfo(BuildingRightInfoRequest),
    /// Proceeding information.
    ProceedingInfo(ProceedingInfoRequest),
    /// XML-only definition-point output.
    DefinitionPoints(DefinitionPointsRequest),
    /// Ownership overview without property details.
    OwnershipOverview(OwnershipOverviewRequest),
    /// Ownership overview including property details.
    OwnershipOverviewWithProperties(OwnershipOverviewRequest),
    /// Rights record for a person or organization.
    RightsRecord(RightsRecordRequest),
    /// Customer account statement.
    AccountStatement(AccountStatementRequest),
    /// Prices selected by properties.
    PricesByProperties(PricesByPropertiesRequest),
    /// Prices selected by proceeding.
    PricesByProceeding(PricesByProceedingRequest),
    /// Monthly price ZIP selected by cadastral area.
    PricesByCadastralArea(PricesByCadastralAreaRequest),
    /// PDF from the deed collection.
    CollectionDocument(CollectionDocumentRequest),
    /// Geometric-plan PDF.
    GeometricPlan(GeometricPlanRequest),
}

impl GenerateReport {
    /// Returns the versioned XML family, or `None` for fixed binary/ZIP outputs.
    pub fn family(&self) -> Option<ReportFamily> {
        Some(match self {
            Self::OwnershipSheet(_) => ReportFamily::OwnershipSheet,
            Self::ParcelInfo(_) => ReportFamily::ParcelInfo,
            Self::BuildingInfo(_) => ReportFamily::BuildingInfo,
            Self::UnitInfo(_) => ReportFamily::UnitInfo,
            Self::BuildingRightInfo(_) => ReportFamily::BuildingRightInfo,
            Self::ProceedingInfo(_) => ReportFamily::ProceedingInfo,
            Self::DefinitionPoints(_) => ReportFamily::DefinitionPoints,
            Self::OwnershipOverview(_) => ReportFamily::OwnershipOverview,
            Self::OwnershipOverviewWithProperties(_) => {
                ReportFamily::OwnershipOverviewWithProperties
            }
            Self::RightsRecord(_) => ReportFamily::RightsRecord,
            Self::AccountStatement(_) => ReportFamily::AccountStatement,
            Self::PricesByProperties(_) => ReportFamily::PricesByProperties,
            Self::PricesByProceeding(_) => ReportFamily::PricesByProceeding,
            Self::CadastralMap(_)
            | Self::PricesByCadastralArea(_)
            | Self::CollectionDocument(_)
            | Self::GeometricPlan(_) => return None,
        })
    }

    /// Validates structural and account/format restrictions before dispatch.
    ///
    /// Server-side authorization, historical-date, and current-period checks can
    /// still reject a locally valid request.
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::OwnershipSheet(request) => {
                if let OwnershipSheetSelector::Objects { objects, .. } = &request.selector {
                    objects.validate()?;
                }
                request
                    .options
                    .validate(&ReportFamily::OwnershipSheet, false, true)
            }
            Self::CadastralMap(request) => {
                if let CadastralMapSelector::Coordinates(coordinates) = &request.selector {
                    coordinates.validate()?;
                }
                validate_fixed_account(&request.account, true)
            }
            Self::ParcelInfo(request) => {
                request
                    .options
                    .validate(&ReportFamily::ParcelInfo, false, false)
            }
            Self::BuildingInfo(request) => {
                request
                    .options
                    .validate(&ReportFamily::BuildingInfo, false, false)
            }
            Self::UnitInfo(request) => {
                request
                    .options
                    .validate(&ReportFamily::UnitInfo, false, false)
            }
            Self::BuildingRightInfo(request) => {
                request
                    .options
                    .validate(&ReportFamily::BuildingRightInfo, false, false)
            }
            Self::ProceedingInfo(request) => {
                request
                    .options
                    .validate(&ReportFamily::ProceedingInfo, false, false)
            }
            Self::DefinitionPoints(request) => {
                if let DefinitionPointsSelector::Coordinates(query) = &request.selector {
                    query.validate()?;
                }
                request
                    .options
                    .validate(&ReportFamily::DefinitionPoints, true, false)
            }
            Self::OwnershipOverview(request) => {
                validate_overview(request)?;
                request
                    .options
                    .validate(&ReportFamily::OwnershipOverview, false, false)
            }
            Self::OwnershipOverviewWithProperties(request) => {
                validate_overview(request)?;
                request.options.validate(
                    &ReportFamily::OwnershipOverviewWithProperties,
                    false,
                    true,
                )
            }
            Self::RightsRecord(request) => {
                validate_character_length("name", &request.name, 1, 255)?;
                request
                    .options
                    .validate(&ReportFamily::RightsRecord, false, true)
            }
            Self::AccountStatement(request) => {
                if request.version == ReportVersionSelection::DefaultForFamily
                    && ReportFamily::AccountStatement.default_version().is_none()
                {
                    return Err(Error::invalid_input(
                        "version",
                        "no family-specific default version is known",
                    ));
                }
                Ok(())
            }
            Self::PricesByProperties(request) => {
                request.objects.validate()?;
                request
                    .options
                    .validate(&ReportFamily::PricesByProperties, false, false)
            }
            Self::PricesByProceeding(request) => {
                request
                    .options
                    .validate(&ReportFamily::PricesByProceeding, false, false)
            }
            Self::PricesByCadastralArea(request) => {
                if !(1_789..=9_999).contains(&request.year) {
                    return Err(Error::invalid_input("year", "year must be in 1789..=9999"));
                }
                if !(1..=12).contains(&request.month_from) || !(1..=12).contains(&request.month_to)
                {
                    return Err(Error::invalid_input("month", "months must be in 1..=12"));
                }
                if request.month_from > request.month_to {
                    return Err(Error::invalid_input(
                        "month_to",
                        "last month cannot precede the first month",
                    ));
                }
                validate_fixed_account(&request.account, false)
            }
            Self::CollectionDocument(request) => validate_fixed_account(&request.account, true),
            Self::GeometricPlan(request) => validate_fixed_account(&request.account, false),
        }
    }
}

fn validate_fixed_account(account: &AccountContext, verifying_allowed: bool) -> Result<()> {
    account.validate()?;
    if account.is_verifying() && !verifying_allowed {
        Err(Error::invalid_input(
            "account",
            "this generation operation is not available to verifying accounts",
        ))
    } else {
        Ok(())
    }
}

fn validate_overview(request: &OwnershipOverviewRequest) -> Result<()> {
    if let Some(code) = &request.legal_relation
        && !(1..=4).contains(&code.as_str().chars().count())
    {
        return Err(Error::invalid_input(
            "legal_relation",
            "legal-relation code must contain 1–4 characters",
        ));
    }
    Ok(())
}

fn validate_character_length(
    field: &'static str,
    value: &str,
    minimum: usize,
    maximum: usize,
) -> Result<()> {
    let length = value.chars().count();
    if (minimum..=maximum).contains(&length) {
        Ok(())
    } else {
        Err(Error::invalid_input(
            field,
            format!("expected {minimum}–{maximum} characters"),
        ))
    }
}

/// Processing state returned in report metadata.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum ReportState {
    /// `čeká`: accepted and queued.
    Waiting,
    /// `vytváří se`: generation has started.
    Creating,
    /// `podepisuje se`: electronic sealing is in progress.
    Signing,
    /// `zpracován`: ready for its first retrieval.
    Processed,
    /// `chyba při vytváření`: creation or sealing failed.
    CreationFailed,
    /// `chyba při zpracování`: processing failed before creation.
    ProcessingFailed,
    /// `zaúčtován`: already retrieved and charged.
    Charged,
    /// `zrušen`: cancelled or marked deleted.
    Cancelled,
    /// A future state retained without normalization.
    Other(String),
}

/// Closed-world lifecycle classification with an explicit unknown branch.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ReportStateClass {
    /// Waiting can continue within the configured deadline.
    Pending,
    /// The report is ready for an approved retrieval.
    Ready,
    /// The server reported generation or processing failure.
    Failed,
    /// The report was cancelled or deleted.
    Cancelled,
    /// A state unknown to this crate; callers must fail closed.
    Unknown,
}

impl ReportState {
    /// Converts an official or future raw state without discarding its text.
    pub fn from_raw(raw: impl Into<String>) -> Self {
        let raw = raw.into();
        match raw.as_str() {
            "čeká" => Self::Waiting,
            "vytváří se" => Self::Creating,
            "podepisuje se" => Self::Signing,
            "zpracován" => Self::Processed,
            "chyba při vytváření" => Self::CreationFailed,
            "chyba při zpracování" => Self::ProcessingFailed,
            "zaúčtován" => Self::Charged,
            "zrušen" => Self::Cancelled,
            _ => Self::Other(raw),
        }
    }

    /// Returns the official or untouched unknown state.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Waiting => "čeká",
            Self::Creating => "vytváří se",
            Self::Signing => "podepisuje se",
            Self::Processed => "zpracován",
            Self::CreationFailed => "chyba při vytváření",
            Self::ProcessingFailed => "chyba při zpracování",
            Self::Charged => "zaúčtován",
            Self::Cancelled => "zrušen",
            Self::Other(raw) => raw,
        }
    }

    /// Classifies this state for bounded waiting and download preflight.
    pub fn classification(&self) -> ReportStateClass {
        match self {
            Self::Waiting | Self::Creating | Self::Signing => ReportStateClass::Pending,
            Self::Processed | Self::Charged => ReportStateClass::Ready,
            Self::CreationFailed | Self::ProcessingFailed => ReportStateClass::Failed,
            Self::Cancelled => ReportStateClass::Cancelled,
            Self::Other(_) => ReportStateClass::Unknown,
        }
    }

    /// Returns true for `čeká`, `vytváří se`, and `podepisuje se`.
    pub fn is_pending(&self) -> bool {
        self.classification() == ReportStateClass::Pending
    }

    /// Returns true for `zpracován` and `zaúčtován`.
    pub fn is_ready(&self) -> bool {
        self.classification() == ReportStateClass::Ready
    }

    /// Returns true for either official failure state.
    pub fn is_failed(&self) -> bool {
        self.classification() == ReportStateClass::Failed
    }

    /// Returns true for `zrušen`.
    pub fn is_cancelled(&self) -> bool {
        self.classification() == ReportStateClass::Cancelled
    }

    /// Returns true only for a raw state unknown to this crate.
    pub fn is_unknown(&self) -> bool {
        self.classification() == ReportStateClass::Unknown
    }
}

impl fmt::Display for ReportState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Format returned in report metadata, including future server values.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum ReportFormat {
    /// PDF output.
    Pdf,
    /// XML output.
    Xml,
    /// HTML output.
    Html,
    /// ZIP output.
    Zip,
    /// A future output format retained lexically.
    Other(String),
}

impl ReportFormat {
    /// Converts a raw server format without discarding unknown values.
    pub fn from_raw(raw: impl Into<String>) -> Self {
        let raw = raw.into();
        match raw.as_str() {
            "pdf" => Self::Pdf,
            "xml" => Self::Xml,
            "html" => Self::Html,
            "zip" => Self::Zip,
            _ => Self::Other(raw),
        }
    }

    /// Returns the official or untouched unknown format.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Pdf => "pdf",
            Self::Xml => "xml",
            Self::Html => "html",
            Self::Zip => "zip",
            Self::Other(raw) => raw,
        }
    }
}

impl fmt::Display for ReportFormat {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Queue and output metadata for one report. Report bytes are deliberately
/// absent until an explicit approved download.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct ReportMetadata {
    /// Internal report ID.
    pub id: ReportId,
    /// Server-provided report name.
    pub name: String,
    /// Number of chargeable units (`pocetJednotek`).
    pub units: Option<u64>,
    /// Number of pages (`pocetStran`).
    pub pages: Option<u16>,
    /// Exact current server price. `None` is never treated as free.
    pub price: Option<Czk>,
    /// Request submission timestamp (`datumPozadavku`).
    pub requested_at: XmlDateTime,
    /// Generation start timestamp (`datumSpusteni`).
    pub started_at: Option<XmlDateTime>,
    /// Output creation timestamp (`datumVytvoreni`).
    pub created_at: Option<XmlDateTime>,
    /// Parent report ID, if this is a related output.
    pub parent_id: Option<ReportId>,
    /// Every related child report ID, in server order.
    pub child_ids: Vec<ReportId>,
    /// Current processing state.
    pub state: ReportState,
    /// Declared output format.
    pub format: ReportFormat,
    /// Lexical XML report version, when supplied.
    pub version: Option<ReportVersion>,
    /// Presence of an electronic seal (`elZnacka`), not cryptographic verification.
    pub electronic_seal: Option<bool>,
    /// Presence of a timestamp (`casoveRaziko`), not cryptographic verification.
    pub timestamp: Option<bool>,
}

impl ReportMetadata {
    /// Returns the validated report ID used by subsequent lifecycle calls.
    pub fn id(&self) -> &ReportId {
        &self.id
    }
}

/// Every report returned by generation or `seznamSestav`, including related
/// parent/child outputs.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ReportBatch {
    /// Reports in server order. No item is discarded for convenience.
    pub reports: Vec<ReportMetadata>,
}

impl ReportBatch {
    /// Creates a batch while retaining all metadata entries.
    pub fn new(reports: Vec<ReportMetadata>) -> Self {
        Self { reports }
    }

    /// Returns all reports without assuming a single-output operation.
    pub fn as_slice(&self) -> &[ReportMetadata] {
        &self.reports
    }

    /// Consumes the batch only when exactly one report was returned.
    pub fn into_single_report(self) -> Result<ReportMetadata> {
        if self.reports.len() == 1 {
            Ok(self
                .reports
                .into_iter()
                .next()
                .expect("length checked immediately before extraction"))
        } else {
            Err(Error::invalid_response(format!(
                "expected one report, received {}",
                self.reports.len()
            )))
        }
    }
}

/// Explicit authorization for the potential charge caused by `vratSestavu`.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ChargeApproval {
    /// Accept only an exact zero price.
    FreeOnly,
    /// Accept a price no greater than this exact amount.
    UpTo(Czk),
    /// Explicitly accept any server-provided non-negative price.
    Any,
}

impl ChargeApproval {
    /// Creates an exact upper-limit approval.
    pub fn up_to(maximum: Czk) -> Self {
        Self::UpTo(maximum)
    }

    /// Tests a freshly retrieved optional server price.
    ///
    /// Missing prices are rejected for every approval mode, including `Any`.
    pub fn accepts(&self, price: Option<Czk>) -> bool {
        let Some(price) = price else {
            return false;
        };
        match self {
            Self::FreeOnly => price == Czk::ZERO,
            Self::UpTo(maximum) => price.as_decimal() <= maximum.as_decimal(),
            Self::Any => true,
        }
    }
}

/// Bounded polling controls for report waiting.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WaitOptions {
    /// Delay before the first status poll.
    pub initial_delay: Duration,
    /// Initial interval between status polls.
    pub poll_interval: Duration,
    /// Maximum total waiting time.
    pub max_wait: Duration,
    /// Optional cap enabling exponential polling backoff.
    pub max_poll_interval: Option<Duration>,
}

impl WaitOptions {
    /// Validates settings that would otherwise cause busy or non-progressing polls.
    pub fn validate(&self) -> Result<()> {
        if self.poll_interval.is_zero() {
            return Err(Error::invalid_input(
                "poll_interval",
                "poll interval must be greater than zero",
            ));
        }
        if self
            .max_poll_interval
            .is_some_and(|interval| interval.is_zero())
        {
            return Err(Error::invalid_input(
                "max_poll_interval",
                "backoff cap must be greater than zero when supplied",
            ));
        }
        Ok(())
    }
}

impl Default for WaitOptions {
    fn default() -> Self {
        Self {
            initial_delay: Duration::from_secs(2),
            poll_interval: Duration::from_secs(2),
            max_wait: Duration::from_secs(120),
            max_poll_interval: None,
        }
    }
}

/// Raw report bytes together with correlated metadata and all service messages.
#[derive(Clone, Eq, PartialEq)]
pub struct DownloadedReport {
    /// Metadata returned with the downloaded report.
    pub metadata: ReportMetadata,
    /// All non-error messages from the download response.
    pub messages: Vec<ServiceMessage>,
    /// Untouched decoded report bytes.
    pub bytes: Bytes,
}

impl DownloadedReport {
    /// Returns the downloaded report ID.
    pub fn id(&self) -> &ReportId {
        self.metadata.id()
    }

    /// Returns the untouched report bytes.
    pub fn as_bytes(&self) -> &Bytes {
        &self.bytes
    }
}

impl fmt::Debug for DownloadedReport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DownloadedReport")
            .field("metadata", &self.metadata)
            .field("message_count", &self.messages.len())
            .field("byte_len", &self.bytes.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paying_xml() -> ReportOptions {
        ReportOptions::xml_default(AccountContext::Paying)
    }

    fn parcel_request(options: ReportOptions) -> GenerateReport {
        GenerateReport::ParcelInfo(ParcelInfoRequest::new(
            ParcelSelector::Id(ParcelId::new("123").unwrap()),
            options,
        ))
    }

    #[test]
    fn report_versions_are_lexical_and_open() {
        assert_eq!(ReportVersion::new("1.0").unwrap(), ReportVersion::V1_0);
        assert_eq!(ReportVersion::new("3.0").unwrap(), ReportVersion::V3_0);
        assert_eq!(ReportVersion::new("03.00").unwrap().as_str(), "03.00");
        assert!(ReportVersion::new("1").is_err());
        assert!(ReportVersion::new("1.0.1").is_err());
        assert!(ReportVersion::new("1.a").is_err());
    }

    #[test]
    fn all_thirteen_families_have_unique_official_codes() {
        let mut codes: Vec<_> = ReportFamily::ALL.iter().map(ReportFamily::as_str).collect();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(ReportFamily::ALL.len(), 13);
        assert_eq!(codes.len(), 13);
        assert_eq!(
            ReportFamily::from_raw("InformaceOParcelach"),
            ReportFamily::ParcelInfo
        );
        assert_eq!(ReportFamily::from_raw("Future").as_str(), "Future");
    }

    #[test]
    fn defaults_are_family_specific_and_pinned() {
        assert_eq!(
            ReportFamily::OwnershipSheet.default_version(),
            Some(ReportVersion::V3_0)
        );
        for family in ReportFamily::ALL {
            if *family != ReportFamily::OwnershipSheet {
                assert_eq!(family.default_version(), Some(ReportVersion::V1_0));
            }
        }
        assert_eq!(ReportFamily::Other("future".into()).default_version(), None);
        assert_eq!(
            paying_xml().version,
            ReportVersionSelection::DefaultForFamily
        );
        assert_eq!(
            ReportOptions::xml_latest(AccountContext::Paying).version,
            ReportVersionSelection::LatestAvailable
        );
    }

    #[test]
    fn account_fields_and_verifying_combinations_are_validated() {
        let invalid_free = AccountContext::Free {
            cislo_jednaci: String::new(),
            ucel_kod: 1,
        };
        assert!(
            parcel_request(ReportOptions::xml_default(invalid_free))
                .validate()
                .is_err()
        );
        let invalid_purpose = AccountContext::Free {
            cislo_jednaci: "A-1".into(),
            ucel_kod: 0,
        };
        assert!(
            parcel_request(ReportOptions::xml_default(invalid_purpose))
                .validate()
                .is_err()
        );

        let verifying = AccountContext::Verifying {
            poradove_cislo: "42".into(),
            overovaci_dolozka: true,
        };
        assert!(
            parcel_request(ReportOptions::pdf(verifying.clone()))
                .validate()
                .is_err()
        );
        let sheet = GenerateReport::OwnershipSheet(OwnershipSheetRequest::new(
            OwnershipSheetSelector::Id(OwnershipSheetId::new("1").unwrap()),
            ReportOptions::pdf(verifying.clone()),
        ));
        assert!(sheet.validate().is_ok());
        let sheet_xml = GenerateReport::OwnershipSheet(OwnershipSheetRequest::new(
            OwnershipSheetSelector::Id(OwnershipSheetId::new("1").unwrap()),
            ReportOptions::xml_default(verifying),
        ));
        assert!(sheet_xml.validate().is_err());
    }

    #[test]
    fn xml_only_and_attached_xml_combinations_fail_before_serialization() {
        let points = GenerateReport::DefinitionPoints(DefinitionPointsRequest::new(
            DefinitionPointsSelector::CadastralArea(CadastralAreaCode::new("1").unwrap()),
            ReportOptions::pdf(AccountContext::Paying),
        ));
        assert!(points.validate().is_err());
        assert!(
            parcel_request(ReportOptions::new(
                OutputFormat::Xml,
                ReportVersionSelection::DefaultForFamily,
                true,
                AccountContext::Paying,
            ))
            .validate()
            .is_err()
        );
        assert!(
            parcel_request(ReportOptions::pdf_with_xml(AccountContext::Paying))
                .validate()
                .is_ok()
        );
    }

    #[test]
    fn object_lists_are_homogeneous_nonempty_and_unique() {
        assert!(ReportObjectList::Parcels(Vec::new()).validate().is_err());
        assert!(
            ReportObjectList::Parcels(vec![
                ParcelId::new("1").unwrap(),
                ParcelId::new("1").unwrap(),
            ])
            .validate()
            .is_err()
        );
        assert_eq!(
            ReportObjectList::BuildingRights(vec![BuildingRightId::new("9").unwrap()]).kind_code(),
            "PRA_S"
        );
    }

    #[test]
    fn proceeding_numbers_and_decimal_map_fields_follow_xsd_limits() {
        assert!(ProceedingNumber::new("V-123/2015-101").is_ok());
        assert!(ProceedingNumber::new("ABC-1/2015-01").is_ok());
        assert!(ProceedingNumber::new("V123/2015-101").is_err());
        assert!(ProceedingNumber::new("V-123456789/2015-101").is_err());
        assert!(
            MapCoordinates::new(Decimal::new(-123456789, 2), Decimal::new(-12345678, 2),).is_ok()
        );
        assert!(MapCoordinates::new(Decimal::new(1, 3), Decimal::ZERO).is_err());
        assert!(MapCoordinates::new(Decimal::new(1234567890, 2), Decimal::ZERO).is_err());
    }

    #[test]
    fn monthly_period_and_fixed_output_account_rules_are_checked() {
        let request = GenerateReport::PricesByCadastralArea(PricesByCadastralAreaRequest::new(
            CadastralAreaCode::new("1").unwrap(),
            2026,
            11,
            10,
            AccountContext::Paying,
        ));
        assert!(request.validate().is_err());
        let request = GenerateReport::GeometricPlan(GeometricPlanRequest::new(
            DocumentId::new("1").unwrap(),
            AccountContext::Verifying {
                poradove_cislo: "1".into(),
                overovaci_dolozka: false,
            },
        ));
        assert!(request.validate().is_err());
    }

    #[test]
    fn every_known_report_state_has_stable_classification_and_raw_text() {
        let cases = [
            ("čeká", ReportState::Waiting, ReportStateClass::Pending),
            (
                "vytváří se",
                ReportState::Creating,
                ReportStateClass::Pending,
            ),
            (
                "podepisuje se",
                ReportState::Signing,
                ReportStateClass::Pending,
            ),
            ("zpracován", ReportState::Processed, ReportStateClass::Ready),
            (
                "chyba při vytváření",
                ReportState::CreationFailed,
                ReportStateClass::Failed,
            ),
            (
                "chyba při zpracování",
                ReportState::ProcessingFailed,
                ReportStateClass::Failed,
            ),
            ("zaúčtován", ReportState::Charged, ReportStateClass::Ready),
            (
                "zrušen",
                ReportState::Cancelled,
                ReportStateClass::Cancelled,
            ),
        ];
        for (raw, state, class) in cases {
            assert_eq!(ReportState::from_raw(raw), state);
            assert_eq!(state.as_str(), raw);
            assert_eq!(state.classification(), class);
        }
        let unknown = ReportState::from_raw("budouci stav");
        assert!(unknown.is_unknown());
        assert_eq!(unknown.as_str(), "budouci stav");
        assert_eq!(ReportFormat::from_raw("future").as_str(), "future");
    }

    #[test]
    fn approval_requires_a_price_and_compares_exact_amounts() {
        let zero = Czk::ZERO;
        let ten = Czk::from_major_units(10);
        let above = Czk::from_str("10.01").unwrap();
        assert!(ChargeApproval::FreeOnly.accepts(Some(zero)));
        assert!(!ChargeApproval::FreeOnly.accepts(Some(ten)));
        assert!(ChargeApproval::UpTo(ten).accepts(Some(ten)));
        assert!(!ChargeApproval::UpTo(ten).accepts(Some(above)));
        assert!(ChargeApproval::Any.accepts(Some(above)));
        assert!(!ChargeApproval::Any.accepts(None));
    }

    #[test]
    fn wait_defaults_are_bounded_and_validated() {
        let options = WaitOptions::default();
        assert_eq!(options.initial_delay, Duration::from_secs(2));
        assert_eq!(options.poll_interval, Duration::from_secs(2));
        assert_eq!(options.max_wait, Duration::from_secs(120));
        assert_eq!(options.max_poll_interval, None);
        assert!(options.validate().is_ok());
        let invalid = WaitOptions {
            poll_interval: Duration::ZERO,
            ..options
        };
        assert!(invalid.validate().is_err());
    }
}
