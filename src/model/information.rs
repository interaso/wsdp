use std::fmt;
use std::str::FromStr;

use rust_decimal::Decimal;

use super::id::{CadastralAreaCode, MunicipalityPartCode, OwnershipSheetId, ParcelId, UnitId};
use super::search::{
    AuthorizedSubject, Building, BuildingRight, BuildingTypeCode, CadastralAreaReference,
    Ownership, Parcel, Unit,
};
use crate::{Error, Result};

/// A building number used by the information natural key (`CisloStavbyType`).
///
/// This is not an internal `BuildingId`; the XSD independently permits 30 digits.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct UnitBuildingNumber(String);

impl UnitBuildingNumber {
    /// Validates a positive decimal integer of at most 30 digits.
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        if value.is_empty()
            || value.len() > 30
            || !value.bytes().all(|byte| byte.is_ascii_digit())
            || value.bytes().all(|byte| byte == b'0')
        {
            return Err(Error::invalid_input(
                "building_number",
                "expected a positive decimal integer with at most 30 digits",
            ));
        }
        Ok(Self(value))
    }

    /// Returns the exact original number.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for UnitBuildingNumber {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for UnitBuildingNumber {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        Self::new(value)
    }
}

/// `dejMBRParcel` request. At least one parcel ID is required.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParcelBoundingBoxRequest {
    /// Internal parcel IDs, encoded as repeated `parcelaId` elements.
    pub parcel_ids: Vec<ParcelId>,
}

impl ParcelBoundingBoxRequest {
    /// Validates the XSD's non-empty ID list.
    pub fn new(parcel_ids: Vec<ParcelId>) -> Result<Self> {
        if parcel_ids.is_empty() {
            return Err(Error::invalid_input(
                "parcel_ids",
                "at least one parcel ID is required",
            ));
        }
        Ok(Self { parcel_ids })
    }
}

/// Unit-overview lookup by internal ID or natural key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UnitOverviewSelector {
    /// Internal unit ID (`idJednotky`).
    Id(UnitId),
    /// Municipality part, building type/number, and unit number.
    Number(UnitOverviewNaturalKey),
}

/// Natural key accepted by `dejInfoOJednotce`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnitOverviewNaturalKey {
    /// Municipality-part code.
    pub municipality_part: MunicipalityPartCode,
    /// Building-type code.
    pub building_type: BuildingTypeCode,
    /// Building number, which is distinct from an internal building ID.
    pub building_number: UnitBuildingNumber,
    /// Unit number in 0–9999999999.
    pub unit_number: u64,
}

impl UnitOverviewNaturalKey {
    /// Validates the unit-number range from `CisloJednotkyType`.
    pub fn new(
        municipality_part: MunicipalityPartCode,
        building_type: BuildingTypeCode,
        building_number: UnitBuildingNumber,
        unit_number: u64,
    ) -> Result<Self> {
        if unit_number > 9_999_999_999 {
            return Err(Error::invalid_input(
                "unit_number",
                "unit number must be at most 9999999999",
            ));
        }
        Ok(Self {
            municipality_part,
            building_type,
            building_number,
            unit_number,
        })
    }
}

/// `dejInfoOJednotce` request. Account eligibility remains a server decision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnitOverviewRequest {
    /// ID/natural-key choice.
    pub selector: UnitOverviewSelector,
}

/// Ownership-sheet preview lookup by internal ID or natural key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OwnershipSheetPreviewSelector {
    /// Internal ownership-sheet ID (`lvId`).
    Id(OwnershipSheetId),
    /// Cadastral area and ownership-sheet number.
    Number(OwnershipSheetNaturalKey),
}

/// Natural key accepted by `dejNahledLV`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OwnershipSheetNaturalKey {
    /// Cadastral-area code.
    pub cadastral_area: CadastralAreaCode,
    /// Ownership-sheet number in 1–999999.
    pub sheet_number: u32,
}

impl OwnershipSheetNaturalKey {
    /// Validates the ownership-sheet number range.
    pub fn new(cadastral_area: CadastralAreaCode, sheet_number: u32) -> Result<Self> {
        if !(1..=999_999).contains(&sheet_number) {
            return Err(Error::invalid_input(
                "sheet_number",
                "ownership-sheet number must be in 1..=999999",
            ));
        }
        Ok(Self {
            cadastral_area,
            sheet_number,
        })
    }
}

/// `dejNahledLV` request for current, limited LV data. Account eligibility applies.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OwnershipSheetPreviewRequest {
    /// ID/natural-key choice.
    pub selector: OwnershipSheetPreviewSelector,
}

/// One of the three WSDP 3.1 `informace` requests.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InformationRequest {
    /// `dejMBRParcel`.
    ParcelBoundingBoxes(ParcelBoundingBoxRequest),
    /// `dejInfoOJednotce`.
    UnitOverview(UnitOverviewRequest),
    /// `dejNahledLV`.
    OwnershipSheetPreview(OwnershipSheetPreviewRequest),
}

/// Optional GML coordinate-reference metadata from `SRSReferenceGroup`.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SpatialReference {
    /// CRS URI or well-known identifier (`srsName`).
    pub name: Option<String>,
    /// Coordinate dimension (`srsDimension`).
    pub dimension: Option<u32>,
    /// Ordered axis labels (`axisLabels`).
    pub axis_labels: Option<Vec<String>>,
    /// Ordered unit labels (`uomLabels`).
    pub unit_labels: Option<Vec<String>>,
}

/// One two-dimensional S-JTSK position in explicit **Y, X** order.
///
/// Coordinates are exact decimals; no binary floating-point conversion is used.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SJtskPoint {
    /// S-JTSK Y ordinate (the first value on the wire).
    pub y: Decimal,
    /// S-JTSK X ordinate (the second value on the wire).
    pub x: Decimal,
    /// Metadata placed directly on `lowerCorner`, `upperCorner`, or `pos`.
    pub reference: SpatialReference,
}

/// The GML representation used for the two envelope corners.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EnvelopeEncoding {
    /// Preferred `lowerCorner` and `upperCorner` elements.
    Corners,
    /// Deprecated pair of `pos` elements, normalized into lower and upper points.
    Positions,
    /// Deprecated `coordinates` element, parsed using its explicit separators.
    Coordinates {
        /// Decimal separator, defaulting to `.`.
        decimal_separator: String,
        /// Separator between Y and X, defaulting to `,`.
        coordinate_separator: String,
        /// Separator between positions, defaulting to a space.
        tuple_separator: String,
    },
}

/// A GML envelope with normalized lower and upper S-JTSK **Y, X** corners.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundingEnvelope {
    /// Corner containing the minimal ordinates.
    pub lower: SJtskPoint,
    /// Corner containing the maximal ordinates.
    pub upper: SJtskPoint,
    /// Original GML corner representation and deprecated-format separators.
    pub encoding: EnvelopeEncoding,
    /// CRS metadata on the `Envelope` itself.
    pub reference: SpatialReference,
}

/// A legitimate GML `Null` geometry and its optional-reason text.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GmlNull {
    /// The `gml:Null` value, retained verbatim (including an empty anyURI value).
    pub reason: String,
}

/// The required GML `boundedBy` choice.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BoundingGeometry {
    /// A two-dimensional envelope.
    Envelope(Box<BoundingEnvelope>),
    /// No geometry, represented explicitly rather than as a missing result.
    Null(GmlNull),
}

/// One MBR entry and all parcel IDs grouped under it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParcelBoundingBox {
    /// One or more parcel IDs from `IDList/ID`; this response element is `xs:string`.
    pub parcel_ids: Vec<String>,
    /// Fixed XSD object-kind attribute, normally `PAR_S`.
    pub id_kind: String,
    /// Envelope or explicit GML null geometry.
    pub geometry: BoundingGeometry,
    /// Optional `nilReason` on `gml:boundedBy`.
    pub nil_reason: Option<String>,
}

/// Response to `dejMBRParcel`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParcelBoundingBoxResponse {
    /// `None` means optional `infoList` was absent; `Some(vec![])` means it was
    /// present with an empty `MBRList`.
    pub entries: Option<Vec<ParcelBoundingBox>>,
}

/// Lightweight ownership-sheet identity returned by information operations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OwnershipSheet {
    /// Internal ownership-sheet ID.
    pub id: OwnershipSheetId,
    /// Cadastral area with optional code and name.
    pub cadastral_area: CadastralAreaReference,
    /// Ownership-sheet number.
    pub number: u32,
}

/// A unit extended with display names for its type and use (`JednotkaLongType`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DetailedUnit {
    /// All base `JednotkaType` fields.
    pub unit: Unit,
    /// Unit-type display name.
    pub type_name: Option<String>,
    /// Unit-use display name.
    pub use_name: Option<String>,
}

/// Response to `dejInfoOJednotce`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnitOverviewResponse {
    /// Related ownership sheet, when supplied.
    pub ownership_sheet: Option<OwnershipSheet>,
    /// Unit with optional display names, when supplied.
    pub unit: Option<DetailedUnit>,
    /// Building, when supplied.
    pub building: Option<Building>,
    /// Zero or more authorized subjects.
    pub subjects: Vec<AuthorizedSubject>,
    /// Zero or more ownership records.
    pub ownerships: Vec<Ownership>,
}

/// Current limited-content response to `dejNahledLV`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OwnershipSheetPreviewResponse {
    /// Ownership sheet, when supplied.
    pub ownership_sheet: Option<OwnershipSheet>,
    /// Zero or more authorized subjects.
    pub subjects: Vec<AuthorizedSubject>,
    /// Zero or more ownership records.
    pub ownerships: Vec<Ownership>,
    /// Zero or more parcels.
    pub parcels: Vec<Parcel>,
    /// Zero or more buildings.
    pub buildings: Vec<Building>,
    /// Zero or more detailed units.
    pub units: Vec<DetailedUnit>,
    /// Zero or more building rights.
    pub building_rights: Vec<BuildingRight>,
}

/// Typed result of one of the three information operations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InformationResponse {
    /// Parcel MBR entries.
    ParcelBoundingBoxes(ParcelBoundingBoxResponse),
    /// Unit overview.
    UnitOverview(Box<UnitOverviewResponse>),
    /// LV preview.
    OwnershipSheetPreview(OwnershipSheetPreviewResponse),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_validate_nonempty_and_natural_key_ranges() {
        assert!(ParcelBoundingBoxRequest::new(Vec::new()).is_err());
        assert!(OwnershipSheetNaturalKey::new(CadastralAreaCode::new("1").unwrap(), 0).is_err());
        assert!(UnitBuildingNumber::new("123456789012345678901234567890").is_ok());
        assert!(UnitBuildingNumber::new("1234567890123456789012345678901").is_err());
    }
}
