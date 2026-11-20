//! Complete model for the legacy LV 1.0 `VypisZKatastruNemovitosti` report.
//!
//! The legacy report is presentation-shaped and has no XML namespace. Models
//! retain optional wrappers, empty sequence forms, repeated choice entries, and
//! exact legacy lexical values rather than normalizing them into the NTR model.

use crate::LegalRelation;

use super::legacy_base::{
    AuthorizedSubjectIdentification, Building, BuildingRight, BuildingWrapper,
    CharacteristicWrapper, DeedName, DeedSection, LegacyId, Parcel, ParcelWrapper, Proceeding,
    ReportedShare, SubjectNameForm, SubjectPropertyIdentification, Unit, UnitWrapper,
};

/// Complete legacy ownership-sheet report.
#[derive(Clone, Eq, PartialEq)]
pub struct LegacyOwnershipSheetReport {
    /// Every `TEXTY` occurrence in the required `LIST_TEXT` wrapper.
    pub texts: Vec<LegacyOwnershipSheetText>,
    /// Optional `LIST_ZAPATI_LV` wrapper, including its valid empty form.
    pub footer: Option<LegacyOwnershipSheetFooter>,
    /// Optional `NEM_BEZ_LV` collection of properties without an LV.
    pub properties_without_sheet: Option<LegacyPropertiesWithoutSheet>,
    /// Lexical validity timestamp (`PLATNOST`).
    pub valid_at: String,
    /// Free-access text (`BEZPL_PRISTUP`), retained verbatim.
    pub free_access: String,
    /// Lexical creation timestamp (`VYHOTOVENO`).
    pub created_at: String,
    /// Optional additional output information (`INFO_VYSTUP`).
    pub output_info: Option<String>,
    /// Optional raw XMLDSIG signature subtree; it is retained, not verified.
    pub signature: Option<LegacyXmlSignature>,
}

/// An untouched XMLDSIG `Signature` subtree.
#[derive(Clone, Eq, PartialEq)]
pub struct LegacyXmlSignature {
    /// Exact source XML for the complete signature element.
    pub raw_xml: String,
}

/// Untyped legacy XML content (`xs:anyType` by default).
#[derive(Clone, Eq, PartialEq)]
pub struct LegacyRawXml {
    /// Exact source XML for the complete element, including any children.
    pub raw_xml: String,
}

/// One `TEXTY` ownership-sheet body.
#[derive(Clone, Eq, PartialEq)]
pub struct LegacyOwnershipSheetText {
    /// LV header (`ZAHLAVI_LV`).
    pub header: LegacySheetHeader,
    /// Section-A owner and other authorized-subject relation groups.
    pub owner_relations: Vec<LegacyOwnerRelation>,
    /// Full-register parcels in section B (`POZEMKY/PARCELA`).
    pub parcels: Vec<LegacyParcel>,
    /// Buildings in section B (`STAVBY/BUDOVA`).
    pub buildings: Vec<LegacyBuilding>,
    /// Optional `PRAVA_STAVBY` wrapper and all building rights.
    pub building_rights: Option<Vec<LegacyBuildingRight>>,
    /// Acquisition-title records (`E_NABYVACI_TITULY`).
    pub acquisition_titles: Vec<LegacyAcquisitionTitle>,
    /// Standalone unit records (`JEDNOTKY/JEDNOTKA`).
    pub units: Vec<LegacyStandaloneUnit>,
    /// Soil-quality records (`F_BONITY/F_BONITA`).
    pub bonities: Vec<LegacyBonity>,
    /// Repeated D1 notice groups (`D1_UPOZORNENI`).
    pub notices: Vec<LegacyNoticeGroup>,
    /// Optional `ZMENY_PV` wrapper and all legal-relation changes.
    pub legal_relation_changes: Option<Vec<LegacyLegalRelationChange>>,
    /// Simplified-register parcel source groups (`PARCELY_ZE`).
    pub simplified_parcel_sources: Vec<LegacySimplifiedParcelSource>,
    /// Cadastral operations (`UKONY_KN`).
    pub operations: Vec<LegacyCadastralOperation>,
    /// Exactly three unique section entries in `JINA_PRAVA` in valid documents.
    pub other_right_sections: Vec<LegacyOtherRightsSection>,
    /// First warning text (`UPOZORNENI1`).
    pub warning1: String,
    /// Second warning text (`UPOZORNENI2`).
    pub warning2: String,
    /// Partial-report marker (`CASTECNY`), retained as raw text.
    pub partial: String,
}

/// Legacy LV header (`ZAHLAVI_LV`).
#[derive(Clone, Eq, PartialEq)]
pub struct LegacySheetHeader {
    /// District information (`OKRES`).
    pub district: LegacyDistrict,
    /// Municipality with string-backed code (`OBEC`).
    pub municipality: LegacySheetMunicipality,
    /// Cadastral area with optional original name (`KATASTR_UZEMI`).
    pub cadastral_area: LegacySheetCadastralArea,
    /// Parcel-numbering-series description (`CIS_RADA`).
    pub numbering_series: String,
    /// Ownership-sheet ID (`TEL_ID`), which may be present and empty.
    pub id: LegacyId,
    /// Ownership-sheet number (`CISLO_LV`), retained as text.
    pub number: String,
}

/// Legacy district identity.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct LegacyDistrict {
    /// Optional NUTS 4 identifier (`nuts4`).
    pub nuts4: Option<String>,
    /// Optional district name (`nazev`).
    pub name: Option<String>,
}

/// Municipality identity with the LV header's string code.
#[derive(Clone, Eq, PartialEq)]
pub struct LegacySheetMunicipality {
    /// Municipality code (`kod`), retained without numeric conversion.
    pub code: String,
    /// Municipality name (`nazev`).
    pub name: String,
}

/// Cadastral-area identity in the LV header.
#[derive(Clone, Eq, PartialEq)]
pub struct LegacySheetCadastralArea {
    /// Cadastral-area code (`kod`), retained without numeric conversion.
    pub code: String,
    /// Cadastral-area name (`nazev`).
    pub name: String,
    /// Optional original cadastral-area name (`nazev_puv`).
    pub original_name: Option<String>,
}

/// One name-only legal-relation group in `VLASTNICI_JINI_OPRAVNENI`.
#[derive(Clone, Eq, PartialEq)]
pub struct LegacyOwnerRelation {
    /// Relation name (`TYP_VZTAHU`); `code` is always `None` in this legacy XML.
    pub relation: LegalRelation,
    /// All subjects belonging to this relation.
    pub subjects: Vec<LegacyOwner>,
}

/// One section-A subject (`SUBJEKT`).
#[derive(Clone, Eq, PartialEq)]
pub struct LegacyOwner {
    /// Ordered primary targets (`VLA_IDENT`).
    pub identification: super::legacy_base::OwnershipIdentification,
    /// Optional raw subject-kind code (`OPSUB_TYP`).
    pub subject_kind: Option<String>,
    /// Optional `OPSUB_ZKRATKA` characteristic wrapper.
    pub characteristic: Option<CharacteristicWrapper>,
    /// Optional fixed-sequence name form (`OPSUB_NAZEV`).
    pub name: Option<SubjectNameForm>,
    /// Optional repeating-choice subject identification (`OPSUB_IDENT`).
    pub alternate_identification: Option<AuthorizedSubjectIdentification>,
    /// Exact reported share (`PODIL`).
    pub share: ReportedShare,
}

/// One full-register parcel in section B.
#[derive(Clone, Eq, PartialEq)]
pub struct LegacyParcel {
    /// All required, repeatable `PAR_IDENT` wrappers.
    pub identifications: Vec<ParcelWrapper>,
    /// Lexical parcel area (`VYMERA_PARCELY`).
    pub area: String,
    /// Land-kind description (`DRUH_POZ`).
    pub land_kind: String,
    /// Optional land-use description (`ZP_VYUZITI`).
    pub use_description: Option<String>,
    /// All protection descriptions in `ZP_OCHRANY/zpochr`.
    pub protections: Vec<String>,
    /// Parcel seal text (`PLOMBA_PARCELY`).
    pub seal: String,
    /// Raw component marker (`SOUCASTI`).
    pub components: String,
    /// Optional `STAVBA_NA_VICE_PARCELACH` wrapper and all parcels.
    pub buildings_on_multiple_parcels: Option<Vec<Parcel>>,
    /// Component-building list and optional shared temporary marker.
    pub component_buildings: LegacyComponentBuildings,
    /// Optional `STAVBA_NA_PARCELE` wrapper.
    pub building_on_parcel: Option<BuildingWrapper>,
    /// Ordered choices describing buildings forming part of other property.
    pub additional_details: Vec<LegacyBuildingPart>,
    /// Optional `JEDNOTKY` wrapper and all units on the parcel.
    pub units: Option<Vec<LegacyParcelUnit>>,
}

/// Shared `SOUCASTI_STAVBA` content.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct LegacyComponentBuildings {
    /// Every `stavba` in document order.
    pub buildings: Vec<Building>,
    /// Optional temporary-building marker (`docasna`).
    pub temporary: Option<String>,
}

/// One entry in an LV `DALSI_UDAJE` repeating choice.
#[derive(Clone, Eq, PartialEq)]
pub enum LegacyBuildingPart {
    /// Building forms part of a parcel (`stavba_soucasti_pozemku`).
    Parcel(Box<LegacyBuildingPartOfParcel>),
    /// Building forms part of a building right (`stavba_soucasti_prava_stavby`).
    BuildingRight(Box<LegacyBuildingPartOfRight>),
}

/// LV-specific building-part-of-parcel details.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct LegacyBuildingPartOfParcel {
    /// Optional parcel identity (`parcela`).
    pub parcel: Option<Parcel>,
    /// Optional ownership-sheet ID (`tel_id`).
    pub ownership_sheet_id: Option<LegacyId>,
    /// Optional ownership-sheet number (`cislo_tel`).
    pub ownership_sheet_number: Option<String>,
    /// Optional cadastral-area name (`ku_nazev`).
    pub cadastral_area_name: Option<String>,
}

/// LV building-part-of-building-right details.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct LegacyBuildingPartOfRight {
    /// Optional ownership-sheet ID (`tel_id`).
    pub ownership_sheet_id: Option<LegacyId>,
    /// Optional ownership-sheet number (`cislo_tel`).
    pub ownership_sheet_number: Option<String>,
    /// Optional cadastral-area name (`ku_nazev`).
    pub cadastral_area_name: Option<String>,
}

/// One parcel unit, retaining the valid empty `JEDNOTKA` sequence.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct LegacyParcelUnit {
    /// Complete unit data, or `None` for an empty `JEDNOTKA` element.
    pub details: Option<LegacyParcelUnitDetails>,
}

/// Non-empty parcel-unit sequence.
#[derive(Clone, Eq, PartialEq)]
pub struct LegacyParcelUnitDetails {
    /// Optional unit-use description (`JED_ZP_VYUZITI`).
    pub use_description: Option<String>,
    /// Unit wrapper with an optional nested unit (`CISLO_JEDNOTKY`).
    pub identification: UnitWrapper,
    /// Required coded unit type (`TYP_JEDNOTKY`).
    pub unit_type: LegacyUnitType,
    /// Exact share of the building (`PODIL_BA`).
    pub building_share: ReportedShare,
    /// Unit description (`POPIS`).
    pub description: String,
    /// Current ownership-sheet ID (`TEL_ID`).
    pub ownership_sheet_id: LegacyId,
    /// `NA_LV` wrapper with an optional complete sheet reference.
    pub on_sheet: LegacyOptionalSheetReference,
    /// All ownership records for this unit.
    pub ownerships: Vec<LegacyUnitOwnership>,
}

/// Coded unit type with required raw `kod` attribute.
#[derive(Clone, Eq, PartialEq)]
pub struct LegacyUnitType {
    /// Unit-type description retained without trimming.
    pub description: String,
    /// Raw unit-type code from the `kod` attribute.
    pub code: String,
}

/// Unit use with optional raw `kod` attribute.
#[derive(Clone, Eq, PartialEq)]
pub struct LegacyUnitUse {
    /// Unit-use description retained without trimming.
    pub description: String,
    /// Optional raw unit-use code from the `kod` attribute.
    pub code: Option<String>,
}

/// Required pair identifying another ownership sheet.
#[derive(Clone, Eq, PartialEq)]
pub struct LegacySheetReference {
    /// Ownership-sheet ID (`tel_id`), including a present-empty ID.
    pub id: LegacyId,
    /// Ownership-sheet number (`cislo_tel`), retained as text.
    pub number: String,
}

/// Presence-preserving `NA_LV` wrapper with an optional inner sequence.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct LegacyOptionalSheetReference {
    /// Complete reference, or `None` for an empty `NA_LV` element.
    pub details: Option<LegacySheetReference>,
}

/// Ownership record used by all three LV unit contexts.
#[derive(Clone, Eq, PartialEq)]
pub struct LegacyUnitOwnership {
    /// Ordered primary targets (`VLA_IDENT`).
    pub identification: super::legacy_base::OwnershipIdentification,
    /// Every `OPSUB_IDENT`; cardinality is enforced by the containing context.
    pub alternate_identifications: Vec<AuthorizedSubjectIdentification>,
    /// Optional subject-name form (`OPSUB_NAZEV`).
    pub name: Option<SubjectNameForm>,
    /// Exact share (`PODIL`), including its optional components and empty forms.
    pub share: ReportedShare,
}

/// One section-B building (`BUDOVA`).
#[derive(Clone, Eq, PartialEq)]
pub struct LegacyBuilding {
    /// Optional building ID (`ID`).
    pub id: Option<LegacyId>,
    /// Untyped building-seal XML (`PLOMBA_BUDOVY`).
    pub seal: LegacyRawXml,
    /// Raw building code (`KOD`).
    pub code: String,
    /// Building wrapper used for house-number identification (`CISLO_DOMOVNI`).
    pub identification: BuildingWrapper,
    /// Optional building-use description (`ZP_VYUZITI`).
    pub use_description: Option<String>,
    /// All protection descriptions (`ZP_OCHRANY/zpochr`).
    pub protections: Vec<String>,
    /// Optional temporary-building marker (`DOCASNA_STAVBA`).
    pub temporary: Option<String>,
    /// All parcel wrappers in the required `PARCELY` collection.
    pub parcels: Vec<ParcelWrapper>,
    /// Optional `JEDNOTKY` wrapper and all units in the building.
    pub units: Option<Vec<LegacyBuildingUnit>>,
}

/// One unit nested in a section-B building.
#[derive(Clone, Eq, PartialEq)]
pub struct LegacyBuildingUnit {
    /// Optional unit-use description (`JED_ZP_VYUZITI`).
    pub use_description: Option<String>,
    /// Unit wrapper (`CISLO_JEDNOTKY`).
    pub identification: UnitWrapper,
    /// Unit-type description without a code attribute in this context.
    pub unit_type: String,
    /// Exact building share (`PODIL_BA`).
    pub building_share: ReportedShare,
    /// Unit description (`POPIS`).
    pub description: String,
    /// Current ownership-sheet ID (`TEL_ID`).
    pub ownership_sheet_id: LegacyId,
    /// Numeric ownership-sheet number (`NA_LV`).
    pub on_sheet: u32,
    /// All unit ownership records.
    pub ownerships: Vec<LegacyUnitOwnership>,
}

/// One building right, retaining its valid empty sequence.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct LegacyBuildingRight {
    /// Complete right data, or `None` for an empty `PRAVO_STAVBY` element.
    pub details: Option<LegacyBuildingRightDetails>,
}

/// Non-empty building-right sequence.
#[derive(Clone, Eq, PartialEq)]
pub struct LegacyBuildingRightDetails {
    /// Building-right ID (`ID`).
    pub id: LegacyId,
    /// Seal text (`PLOMBA_PRAVA_STAVBY`).
    pub seal: String,
    /// All protection descriptions (`ZP_OCHRANY/zpochr`).
    pub protections: Vec<String>,
    /// Lexical end of validity (`PLATNOST_DO`).
    pub valid_until: String,
    /// Component-building list (`SOUCASTI_STAVBA`).
    pub component_buildings: LegacyComponentBuildings,
    /// All purpose descriptions (`UCELY_PRAVA_STAVBY/UCEL`).
    pub purposes: Vec<String>,
    /// All parcel wrappers in `PARCELY`.
    pub parcels: Vec<ParcelWrapper>,
    /// Optional `JEDNOTKY` wrapper and all units associated with the right.
    pub units: Option<Vec<LegacyBuildingRightUnit>>,
}

/// One unit in a building right, retaining its valid empty sequence.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct LegacyBuildingRightUnit {
    /// Complete unit data, or `None` for an empty `JEDNOTKA` element.
    pub details: Option<LegacyBuildingRightUnitDetails>,
}

/// Non-empty building-right unit sequence.
#[derive(Clone, Eq, PartialEq)]
pub struct LegacyBuildingRightUnitDetails {
    /// Optional unit-use description (`ZP_VYUZITI`).
    pub use_description: Option<String>,
    /// Required unit identity in `CISLO_JEDNOTKY`.
    pub identification: Unit,
    /// Unit-type description (`TYP_JEDNOTKY`).
    pub unit_type: String,
    /// Exact building share (`PODIL_BA`).
    pub building_share: ReportedShare,
    /// Unit description (`POPIS`).
    pub description: String,
    /// Current ownership-sheet ID (`TEL_ID`).
    pub ownership_sheet_id: LegacyId,
    /// Required complete `NA_LV` sheet reference.
    pub on_sheet: LegacySheetReference,
    /// All unit ownership records.
    pub ownerships: Vec<LegacyUnitOwnership>,
}

/// One acquisition title (`E_NABYVACI_TITUL`).
#[derive(Clone, Eq, PartialEq)]
pub struct LegacyAcquisitionTitle {
    /// Optional legal-relation/deed reference (`POLVZ`).
    pub reference: Option<String>,
    /// Optional lexical proceeding text (`RIZENI_E`).
    pub proceeding_text: Option<String>,
    /// Deed-name details (`NAZEV_LIST_E`).
    pub deed_name: DeedName,
    /// Proceeding identification (`IDENT_RIZENI`).
    pub proceeding: Proceeding,
    /// All repeated `NT_PRO` sequence groups.
    pub beneficiary_groups: Vec<LegacyAcquisitionBeneficiaryGroup>,
}

/// One `NT_PRO` sequence occurrence containing `OPRAV_SUBJEKT`.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct LegacyAcquisitionBeneficiaryGroup {
    /// Nested subject identification, or `None` for an empty `OPRAV_SUBJEKT`.
    pub subject: Option<AuthorizedSubjectIdentification>,
}

/// One standalone LV unit (`JEDNOTKY/JEDNOTKA`).
#[derive(Clone, Eq, PartialEq)]
pub struct LegacyStandaloneUnit {
    /// Unit seal text (`PLOMBA_JEDNOTKY`).
    pub seal: String,
    /// Unit wrapper (`CISLO_JEDNOTKY`).
    pub identification: UnitWrapper,
    /// Optional coded unit use (`ZP_VYUZITI`).
    pub use_description: Option<LegacyUnitUse>,
    /// All protection descriptions (`ZP_OCHRANY/zpochr`).
    pub protections: Vec<String>,
    /// Required coded unit type (`TYP_JEDNOTKY`).
    pub unit_type: LegacyUnitType,
    /// Exact building share (`PODIL_B`).
    pub building_share: ReportedShare,
    /// Unit description (`POPIS`).
    pub description: String,
    /// All building records in `BUDOVY`.
    pub buildings: Vec<LegacyUnitBuilding>,
    /// All building-right groups in `PRAVA_STAVBY`.
    pub building_right_groups: Vec<LegacyUnitBuildingRightGroup>,
    /// All parcels in `PARCELY` where a building is part of the parcel.
    pub parcels: Vec<LegacyUnitParcel>,
}

/// One building associated with a standalone unit.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct LegacyUnitBuilding {
    /// Complete building association, or `None` for an empty `BUDOVA` sequence.
    pub details: Option<LegacyUnitBuildingDetails>,
}

/// Non-empty standalone-unit building association.
#[derive(Clone, Eq, PartialEq)]
pub struct LegacyUnitBuildingDetails {
    /// Every `JED_OPSUB` building wrapper.
    pub subjects: Vec<LegacyUnitBuildingSubject>,
    /// Every parcel wrapper in `PARCELY`.
    pub parcels: Vec<ParcelWrapper>,
}

/// One `JED_OPSUB` wrapper.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct LegacyUnitBuildingSubject {
    /// Optional building identity (`stavba`).
    pub building: Option<Building>,
}

/// One outer `PRAVA_STAVBY` sequence group for a standalone unit.
#[derive(Clone, Eq, PartialEq)]
pub struct LegacyUnitBuildingRightGroup {
    /// Required `PRAVO_STAVBY` data, including its repeated inner entries.
    pub right: LegacyUnitBuildingRight,
    /// All `PARCELA` entries following the right in this sequence group.
    pub parcels: Vec<LegacyUnitRightParcel>,
}

/// Building-right data nested in a standalone unit.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct LegacyUnitBuildingRight {
    /// Ordered repeated `PS_IDENT`/`PLATNOST_DO` pairs.
    pub entries: Vec<LegacyUnitBuildingRightEntry>,
}

/// One building-right identification and validity pair.
#[derive(Clone, Eq, PartialEq)]
pub struct LegacyUnitBuildingRightEntry {
    /// Required building-right identity from `PS_IDENT`.
    pub identification: BuildingRight,
    /// Lexical end of validity (`PLATNOST_DO`).
    pub valid_until: String,
}

/// Parcel associated with a unit's building right.
#[derive(Clone, Eq, PartialEq)]
pub struct LegacyUnitRightParcel {
    /// Parcel wrapper (`PAR_IDENT`).
    pub identification: ParcelWrapper,
    /// Land-kind description (`DRUH_POZ`).
    pub land_kind: String,
    /// Lexical parcel area (`VYMERA_PARCELY`).
    pub area: String,
}

/// Parcel associated with a unit where the building is part of the parcel.
#[derive(Clone, Eq, PartialEq)]
pub struct LegacyUnitParcel {
    /// Parcel wrapper (`PAR_IDENT`).
    pub identification: ParcelWrapper,
    /// Land-kind description (`DRUH_POZ`).
    pub land_kind: String,
    /// Lexical parcel area (`VYMERA_PARCELY`).
    pub area: String,
    /// Optional land-use description (`ZP_VYUZITI`).
    pub use_description: Option<String>,
    /// Component-building list (`SOUCASTI_STAVBA`).
    pub component_buildings: LegacyComponentBuildings,
}

/// One section-F soil-quality parcel (`F_BONITA`).
#[derive(Clone, Eq, PartialEq)]
pub struct LegacyBonity {
    /// Special parcel identification with a valid empty sequence.
    pub parcel: LegacyBonityParcel,
    /// Other-cadastral-area wrapper with a valid empty sequence.
    pub other_cadastral_area: LegacyOtherCadastralArea,
    /// All BPEJ entries in `BONITY_BPEJ`.
    pub entries: Vec<LegacyBpej>,
}

/// Presence-preserving `BON_PARCELNI_CISLO` identification.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct LegacyBonityParcel {
    /// Complete identification, or `None` for an empty sequence.
    pub details: Option<LegacyBonityParcelDetails>,
}

/// Non-empty section-F parcel identification.
#[derive(Clone, Eq, PartialEq)]
pub struct LegacyBonityParcelDetails {
    /// Parcel ID (`id`).
    pub id: LegacyId,
    /// Parcel-kind abbreviation (`zkratka`).
    pub kind_abbreviation: String,
    /// Parcel-numbering kind (`druh_cis`), retained as raw text.
    pub numbering_kind: String,
    /// Parcel number (`par_cis`), retained as text.
    pub number: String,
    /// Optional parcel subdivision (`podd_cis`).
    pub subdivision: Option<String>,
    /// Optional parcel part (`dil`) with exact signed-byte range.
    pub part: Option<i8>,
    /// Ownership-sheet ID (`tel_id`).
    pub ownership_sheet_id: LegacyId,
}

/// Presence-preserving `JINE_KU` wrapper.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct LegacyOtherCadastralArea {
    /// Complete area identity, or `None` for an empty sequence.
    pub details: Option<LegacyOtherCadastralAreaDetails>,
}

/// Non-empty other-cadastral-area identity.
#[derive(Clone, Eq, PartialEq)]
pub struct LegacyOtherCadastralAreaDetails {
    /// Cadastral-area code (`kod`).
    pub code: u32,
    /// Cadastral-area name (`nazev`).
    pub name: String,
}

/// One section-F BPEJ entry.
#[derive(Clone, Eq, PartialEq)]
pub struct LegacyBpej {
    /// Raw BPEJ code (`BPEJ_KOD`).
    pub code: String,
    /// BPEJ area (`BPEJ_VYMERA`).
    pub area: u32,
}

/// One repeated D1 notice group beginning with `NAZEV`.
#[derive(Clone, Eq, PartialEq)]
pub struct LegacyNoticeGroup {
    /// Notice name (`NAZEV`).
    pub name: String,
    /// All `SEZNAM_RIZENI` entries associated with this name.
    pub proceedings: Vec<LegacyNoticeProceeding>,
}

/// One `SEZNAM_RIZENI` entry with a valid empty sequence.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct LegacyNoticeProceeding {
    /// Complete proceeding notice, or `None` for an empty sequence.
    pub details: Option<LegacyNoticeProceedingDetails>,
}

/// Non-empty D1 proceeding notice.
#[derive(Clone, Eq, PartialEq)]
pub struct LegacyNoticeProceedingDetails {
    /// Lexical submission date (`DATUM_PODANI`).
    pub submitted_at: String,
    /// Proceeding identification (`IDENT_RIZENI`).
    pub proceeding: Proceeding,
    /// All affected property groups (`NEMOVITOSTI`).
    pub properties: Vec<LegacyNoticeProperty>,
}

/// One affected-property sequence with a valid empty form.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct LegacyNoticeProperty {
    /// Complete property collection, or `None` for an empty `NEMOVITOST`.
    pub details: Option<LegacyNoticePropertyDetails>,
}

/// Non-empty affected-property collection.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct LegacyNoticePropertyDetails {
    /// All parcels in schema order.
    pub parcels: Vec<Parcel>,
    /// All buildings in schema order.
    pub buildings: Vec<Building>,
    /// All units in schema order.
    pub units: Vec<Unit>,
    /// All building rights in schema order.
    pub building_rights: Vec<BuildingRight>,
}

/// One `ZMENA_PV` legal-relation change.
#[derive(Clone, Eq, PartialEq)]
pub struct LegacyLegalRelationChange {
    /// `PLOMBA` proceeding-shaped value, including its valid empty sequence.
    pub seal: Proceeding,
}

/// One `ZDROJ_PARCEL_ZE` source with a valid empty sequence.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct LegacySimplifiedParcelSource {
    /// Complete source data, or `None` for an empty source sequence.
    pub details: Option<LegacySimplifiedParcelSourceDetails>,
}

/// Non-empty simplified-register parcel source.
#[derive(Clone, Eq, PartialEq)]
pub struct LegacySimplifiedParcelSourceDetails {
    /// Raw source code (`KOD_ZDROJE_ZE`).
    pub code: String,
    /// Source description wrapper (`ZDROJ_ZE`).
    pub source: LegacySimplifiedSource,
    /// All simplified parcels in `PARCELY_ZE_A_PK`.
    pub parcels: Vec<LegacySimplifiedParcel>,
}

/// Presence-preserving `ZDROJ_ZE` description.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct LegacySimplifiedSource {
    /// Complete source labels, or `None` for an empty sequence.
    pub details: Option<LegacySimplifiedSourceDetails>,
}

/// Non-empty simplified-register source labels.
#[derive(Clone, Eq, PartialEq)]
pub struct LegacySimplifiedSourceDetails {
    /// Land name (`pozemek_nazev`).
    pub land_name: String,
    /// Land abbreviation (`pozemek_zkratka`).
    pub land_abbreviation: String,
}

/// One simplified-register parcel in an LV.
#[derive(Clone, Eq, PartialEq)]
pub struct LegacySimplifiedParcel {
    /// Untyped parcel-part XML (`DIL_PARCELY_ZE`).
    pub part: LegacyRawXml,
    /// Parcel wrapper (`PARCELNI_CISLO_ZE`).
    pub identification: ParcelWrapper,
    /// Original cadastral-area data (`PUV_NAZEV_KU`).
    pub original_cadastral_area: LegacyOriginalCadastralArea,
    /// Parcel type text (`TYP_1`).
    pub kind: String,
    /// Arbitrary-precision lexical `xs:integer` area (`VYMERA_PARCELY_ZE`).
    pub area: String,
    /// Seal text (`PLOMBA_PARCELY_1`).
    pub seal: String,
}

/// Original cadastral-area data with independently optional fields.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct LegacyOriginalCadastralArea {
    /// Optional numeric code (`kod`).
    pub code: Option<u32>,
    /// Optional name (`nazev`).
    pub name: Option<String>,
}

/// One cadastral operation (`UKON_KN`).
#[derive(Clone, Eq, PartialEq)]
pub struct LegacyCadastralOperation {
    /// Lexical proceeding identification (`IDENT_RIZENI`).
    pub proceeding: String,
    /// Optional repeated `RIZ_TYP_PREDMETU` wrappers.
    pub subject_matters: Option<Vec<LegacyOperationSubjectMatter>>,
}

/// One proceeding-subject-matter wrapper with an optional inner sequence.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct LegacyOperationSubjectMatter {
    /// Proceeding name, or `None` for an empty wrapper.
    pub name: Option<String>,
}

/// One `SEKCE_LIST` and its legal relations.
#[derive(Clone, Eq, PartialEq)]
pub struct LegacyOtherRightsSection {
    /// Raw-preserving section code from the required `SEKCE` attribute.
    pub section: DeedSection,
    /// All `JPV` legal-relation records in `JPV_LIST`.
    pub relations: Vec<LegacyOtherRight>,
}

/// One other legal relation (`JPV`).
#[derive(Clone, Eq, PartialEq)]
pub struct LegacyOtherRight {
    /// Legal-relation ID (`HJPV_ID`), which may be present and empty.
    pub id: LegacyId,
    /// Name-only relation (`TYP_PRAVNIHO_VZTAHU`); no code is inferred.
    pub relation: LegalRelation,
    /// Optional `PORADI_K` wrapper and all repeated priority groups.
    pub priority: Option<Vec<LegacyRelationPriority>>,
    /// Optional marker hiding the obligated property (`NEZOBRAZ_K_NEM`).
    pub hide_obligated_property: Option<String>,
    /// Optional parent legal-relation ID (`NADRIZ_HJPV_ID`).
    pub parent_id: Option<LegacyId>,
    /// All relation descriptions (`JPV_POPISY/JPV_POPIS`).
    pub descriptions: Vec<LegacyOtherRightDescription>,
    /// All benefiting-subject records (`JPV_OPRAV_PRO_OS`).
    pub benefiting_subjects: Vec<LegacyBenefitingSubject>,
    /// Optional benefiting-property collection (`JPV_OPRAV_PRO_NEM`).
    pub benefiting_properties: Option<Vec<LegacyBenefitingProperty>>,
    /// All obligated-subject records (`JPV_POVIN_K_OS`).
    pub obligated_subjects: Vec<LegacyObligatedSubject>,
    /// Optional obligated-property collection (`JPV_POVIN_K_NEM`).
    pub obligated_properties: Option<Vec<LegacyObligatedProperty>>,
    /// All supporting deeds (`JPV_LISTINY/JPV_LISTINA`).
    pub deeds: Vec<LegacyOtherRightDeed>,
}

/// One repeated priority sequence in `PORADI_K`.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct LegacyRelationPriority {
    /// Optional lexical priority date (`datum`).
    pub date: Option<String>,
    /// Optional priority explanation (`text_o_prednostnim_poradi`).
    pub text: Option<String>,
}

/// One legal-relation description (`JPV_POPIS`).
#[derive(Clone, Eq, PartialEq)]
pub struct LegacyOtherRightDescription {
    /// Optional `JPV_RIZENI` wrapper and all proceeding entries.
    pub proceedings: Option<Vec<LegacyDescriptionProceeding>>,
    /// Relation description (`POPIS_PRAVNIHO_VZTAHU`).
    pub text: String,
}

/// One `JPV_RIZ` description-proceeding entry.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct LegacyDescriptionProceeding {
    /// Proceeding identification, or `None` for an empty `JPV_RIZ` sequence.
    pub proceeding: Option<Proceeding>,
}

/// One `JPV_PRO_OS` benefiting-subject record.
#[derive(Clone, Eq, PartialEq)]
pub struct LegacyBenefitingSubject {
    /// Optional combined subject/property identification (`PRO_IDENT_OS`).
    pub identification: Option<SubjectPropertyIdentification>,
    /// Creditor-share text (`PODIL_POHLEDAVKA`).
    pub creditor_share: String,
}

/// One `JPV_PRO_NEM` benefiting-property record.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct LegacyBenefitingProperty {
    /// Optional combined identification (`PRO_IDENT_NEM`).
    pub identification: Option<SubjectPropertyIdentification>,
}

/// One `JPV_K_OS` obligated-subject record.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct LegacyObligatedSubject {
    /// Optional combined identification (`K_IDENT_OS`).
    pub identification: Option<SubjectPropertyIdentification>,
}

/// One `JPV_K_NEM` obligated-property record.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct LegacyObligatedProperty {
    /// Every `K_IDENT_NEM`; a present empty value remains an empty struct entry.
    pub identifications: Vec<SubjectPropertyIdentification>,
}

/// One deed attached to another legal relation (`JPV_LISTINA`).
#[derive(Clone, Eq, PartialEq)]
pub struct LegacyOtherRightDeed {
    /// Deed-name details (`NAZEV_LIST`).
    pub deed_name: DeedName,
    /// Proceeding identification (`IDENT_RIZENI`).
    pub proceeding: Proceeding,
    /// Deed ID (`LISTIN_ID`), which may be present and empty.
    pub id: LegacyId,
    /// Untyped deed-description XML (`POPIS_LIST`).
    pub description: LegacyRawXml,
}

/// Optional LV footer wrapper (`LIST_ZAPATI_LV`).
#[derive(Clone, Default, Eq, PartialEq)]
pub struct LegacyOwnershipSheetFooter {
    /// Optional single workplace footer (`ZAPATI_LV`).
    pub workplace: Option<LegacyWorkplaceFooter>,
}

/// Workplace footer (`ZAPATI_LV`).
#[derive(Clone, Eq, PartialEq)]
pub struct LegacyWorkplaceFooter {
    /// Workplace code (`KODKP`).
    pub workplace_code: u16,
    /// Full workplace text (`KP`).
    pub workplace: String,
}

/// Optional `NEM_BEZ_LV` wrapper.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct LegacyPropertiesWithoutSheet {
    /// Optional single `NEMOVITOST` group.
    pub group: Option<LegacyPropertyWithoutSheetGroup>,
}

/// One `NEMOVITOST` group outside an LV.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct LegacyPropertyWithoutSheetGroup {
    /// All `PARCELA_BEZ_LV` wrappers.
    pub parcels: Vec<LegacyParcelWithoutSheet>,
}

/// One `PARCELA_BEZ_LV` wrapper.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct LegacyParcelWithoutSheet {
    /// Optional parcel identity (`parcela`).
    pub parcel: Option<Parcel>,
}
