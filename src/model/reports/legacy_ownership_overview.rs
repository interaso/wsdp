//! Complete models for the legacy ownership-overview and person-rights reports.
//!
//! Required XML collection wrappers are represented by their contained vectors.
//! Optional wrappers remain `Option`s, and wrappers whose own content is
//! optional use the presence-preserving types from [`legacy_base`](crate::reports::legacy_base).

use crate::LegalRelation;

use super::legacy_base::{
    Address, Building, BuildingRight, BuildingWrapper, CharacteristicCode, CharacteristicWrapper,
    LegacyId, Parcel, ParcelWrapper, Proceeding, PropertyIdentification, ReportedShare,
    SubjectNameForm, UnitWrapper, YesNo,
};

/// The restricted subject-identification choice shared by these legacy reports.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct LegacySubjectIdentification {
    /// Every selected identifier in document order, including repeated choices.
    pub entries: Vec<LegacySubjectIdentifier>,
}

/// One entry in the restricted legacy subject-identification choice.
#[derive(Clone, Eq, PartialEq)]
#[non_exhaustive]
pub enum LegacySubjectIdentifier {
    /// Authorized-subject ID (`id`), including a present-empty ID.
    Id(LegacyId),
    /// Personal identifier fragment (`rc6`), retained as text.
    PersonalNumber6(String),
    /// Personal identifier fragment (`rc7`), retained as text.
    PersonalNumber7(String),
    /// Company identification number (`ico`), retained verbatim.
    CompanyId(String),
    /// Company-ID suffix (`ico_doplnek`).
    CompanyIdSuffix(String),
    /// First partner's subject ID (`bsm1_id`).
    Partner1Id(LegacyId),
    /// Second partner's subject ID (`bsm2_id`).
    Partner2Id(LegacyId),
}

/// District text used by the legacy ownership and rights reports.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct LegacyReportDistrict {
    /// NUTS 4 code (`nuts4`), retained as text.
    pub nuts4: Option<String>,
    /// District name (`nazev`).
    pub name: Option<String>,
}

/// A required legacy `xs:unsignedInt` code and its required name.
#[derive(Clone, Eq, PartialEq)]
pub struct LegacyReportCodeName {
    /// Numeric municipality or cadastral-area code (`kod`).
    pub code: u32,
    /// Municipality or cadastral-area name (`nazev`).
    pub name: String,
}

/// Complete legacy `PrehledVlastnictvi` ownership-overview report.
#[derive(Clone, Eq, PartialEq)]
pub struct OwnershipOverviewReport {
    /// Requested legal relation (`PRAV_VZTAHY/TYPPRA`).
    ///
    /// Its code is `None`, because this legacy report supplies only a name.
    pub requested_legal_relation: LegalRelation,
    /// Authorized subject for whom the overview was generated.
    pub subject: OwnershipOverviewSubject,
    /// Ordered district/municipality groups from `OKRESY`.
    pub districts: Vec<OwnershipOverviewDistrictGroup>,
    /// Territorial restriction (`CP_OMEZENI`).
    pub territorial_restriction: String,
    /// Cadastral workplace (`CP_PRACOVISTE`).
    pub workplace: String,
    /// Lexical validity timestamp (`PLATNOST`).
    pub valid_at: String,
    /// Lexical report creator (`VYHOTOVIL`).
    pub created_by: String,
    /// Free-access text (`BEZPL_PRISTUP`).
    pub free_access: String,
    /// Lexical report creation timestamp (`VYHOTOVENO`).
    pub created_at: String,
    /// Optional output information (`INFO_VYSTUP`).
    pub output_info: Option<String>,
}

/// Authorized-subject block in an ownership overview.
#[derive(Clone, Eq, PartialEq)]
pub struct OwnershipOverviewSubject {
    /// Required `CHAROS` wrapper, which may itself be empty.
    pub characteristic: CharacteristicWrapper,
    /// Subject name and partner-name fields (`OPSUB_NAZEV`).
    pub name: SubjectNameForm,
    /// Restricted repeating identifiers (`OPSUB_IDENT`).
    pub identification: LegacySubjectIdentification,
    /// Every address supplied by `OPSUB_ADRESA`, in document order.
    pub addresses: Vec<Address>,
}

/// One occurrence of the repeating district sequence in `OKRESY`.
#[derive(Clone, Eq, PartialEq)]
pub struct OwnershipOverviewDistrictGroup {
    /// District identification (`OKRES`).
    pub district: LegacyReportDistrict,
    /// Ordered municipality groups from this occurrence's `OBCE` wrapper.
    pub municipalities: Vec<OwnershipOverviewMunicipalityGroup>,
}

/// One occurrence of the repeating municipality sequence in `OBCE`.
#[derive(Clone, Eq, PartialEq)]
pub struct OwnershipOverviewMunicipalityGroup {
    /// Municipality code and name (`OBEC`).
    pub municipality: LegacyReportCodeName,
    /// Nonempty ordered cadastral-area groups from `KATASTRALNI_UZEMI`.
    pub cadastral_areas: Vec<OwnershipOverviewCadastralAreaGroup>,
}

/// One cadastral area and its legal-relation groups.
#[derive(Clone, Eq, PartialEq)]
pub struct OwnershipOverviewCadastralAreaGroup {
    /// Cadastral-area code and name (`KATUZE`).
    pub cadastral_area: LegacyReportCodeName,
    /// Legal-relation groups from `TYPY_PRAV_VZTAHU`; the required wrapper may be empty.
    pub legal_relations: Vec<OwnershipOverviewLegalRelationGroup>,
}

/// One name-only legal relation and its ownership records.
#[derive(Clone, Eq, PartialEq)]
pub struct OwnershipOverviewLegalRelationGroup {
    /// Relation supplied by `TYPRAV_NAZEV`, with no inferred code or ownership classification.
    pub legal_relation: LegalRelation,
    /// Nonempty ordered list of `TELESO` ownership records.
    pub records: Vec<OwnershipOverviewRecord>,
}

/// One `TELESO` record in an ownership overview.
#[derive(Clone, Eq, PartialEq)]
pub struct OwnershipOverviewRecord {
    /// Ownership-sheet number (`LV`), retained as text.
    pub ownership_sheet_number: String,
    /// Repeating parcel, building, unit, and building-right targets (`PRSV_IDENT`).
    pub properties: PropertyIdentification,
    /// Reported ownership share (`PODIL`).
    pub share: ReportedShare,
    /// Ownership-sheet ID (`TEL_ID`), including a present-empty ID.
    pub ownership_sheet_id: LegacyId,
    /// Legal-relation/ownership-sheet text (`JPV_LV`).
    pub legal_relation_sheet: String,
    /// Associated proceeding (`RIZENI`), including its valid empty form.
    pub proceeding: Proceeding,
}

/// Complete legacy `PrehledVlastnictviSNemovitostmi` report.
#[derive(Clone, Eq, PartialEq)]
pub struct OwnershipOverviewWithPropertiesReport {
    /// Requested legal relation (`PRAV_VZTAHY/TYPPRA`), with no inferred code.
    pub requested_legal_relation: LegalRelation,
    /// Authorized subject for whom the overview was generated.
    pub subject: OwnershipPropertiesSubject,
    /// Ordered district/municipality groups from `OKRESY`.
    pub districts: Vec<OwnershipPropertiesDistrictGroup>,
    /// Required output information (`INFO_VYSTUP`), which may be empty text.
    pub output_info: String,
    /// Report-level proceeding (`RIZENI`), including its valid empty form.
    pub proceeding: Proceeding,
    /// Lexical report creator (`VYHOTOVIL`).
    pub created_by: String,
    /// Lexical report creation timestamp (`VYHOTOVENO`).
    pub created_at: String,
    /// Lexical validity timestamp (`PLATNOST`).
    pub valid_at: String,
    /// Cadastral workplace (`PRACOVISTE`).
    pub workplace: String,
    /// Free-access text (`BEZPL_PRISTUP`).
    pub free_access: String,
}

/// Authorized-subject block in an ownership overview with properties.
#[derive(Clone, Eq, PartialEq)]
pub struct OwnershipPropertiesSubject {
    /// Required numeric characteristic code (`CHAROS`) in the XSD range 1–99.
    pub characteristic: CharacteristicCode,
    /// Subject name and partner-name fields (`OPSUB_NAZEV`).
    pub name: SubjectNameForm,
    /// Restricted repeating identifiers (`OPSUB_IDENT`).
    pub identification: LegacySubjectIdentification,
    /// Every address supplied by `OPSUB_ADRESA`, in document order.
    pub addresses: Vec<Address>,
}

/// One occurrence of the repeating district sequence in the property overview.
#[derive(Clone, Eq, PartialEq)]
pub struct OwnershipPropertiesDistrictGroup {
    /// District identification (`OKRES`).
    pub district: LegacyReportDistrict,
    /// Ordered municipality groups from this occurrence's `OBCE` wrapper.
    pub municipalities: Vec<OwnershipPropertiesMunicipalityGroup>,
}

/// One occurrence of the repeating municipality sequence in the property overview.
#[derive(Clone, Eq, PartialEq)]
pub struct OwnershipPropertiesMunicipalityGroup {
    /// Municipality code and name (`OBEC`).
    pub municipality: LegacyReportCodeName,
    /// Nonempty ordered cadastral-area groups from `KATASTRALNI_UZEMI`.
    pub cadastral_areas: Vec<OwnershipPropertiesCadastralAreaGroup>,
}

/// One cadastral area and its detailed ownership records.
#[derive(Clone, Eq, PartialEq)]
pub struct OwnershipPropertiesCadastralAreaGroup {
    /// Cadastral-area code and name (`KATUZE`).
    pub cadastral_area: LegacyReportCodeName,
    /// Nonempty ordered list of `TELESO` records from `TELESA`.
    pub records: Vec<OwnershipPropertiesRecord>,
}

/// One detailed `TELESO` record in an ownership overview with properties.
#[derive(Clone, Eq, PartialEq)]
pub struct OwnershipPropertiesRecord {
    /// Reported ownership share (`PODIL`).
    pub share: ReportedShare,
    /// Ownership-sheet number (`LV`), retained as text.
    pub ownership_sheet_number: String,
    /// Parcels from the required, possibly empty `PARCELY_KN` wrapper.
    pub parcels: Vec<OwnershipParcel>,
    /// Buildings from the required, possibly empty `STAVBY` wrapper.
    pub buildings: Vec<OwnershipBuilding>,
    /// Units from the required, possibly empty `JEDNOTKY` wrapper.
    pub units: Vec<OwnershipUnit>,
    /// Sources from the required, possibly empty `ZDROJE` wrapper.
    pub sources: Vec<OwnershipSource>,
    /// Building rights from the required, possibly empty `PRAVA_STAVBY` wrapper.
    pub building_rights: Vec<OwnershipBuildingRight>,
}

/// One `PARCELA_KN` entry in the property overview.
#[derive(Clone, Eq, PartialEq)]
pub struct OwnershipParcel {
    /// Optional parcel-use description (`ZP_VYUZITI`).
    pub use_description: Option<String>,
    /// Required `PARCELNI_CISLO` wrapper and its optional parcel.
    pub parcel: ParcelWrapper,
    /// Parcel-part text (`DIL`).
    pub part: String,
    /// Parcel area (`VYMERA_PARCELY`) as the XSD's full `xs:unsignedInt` range.
    pub area: u32,
    /// Area-determination text (`URC_VYMERY`).
    pub area_determination: String,
    /// Land-kind text (`DRUH_POZEMKU`).
    pub land_kind: String,
    /// Protection marker (`OCHRANA`), preserving unknown raw values.
    pub protection: YesNo,
    /// Map text (`MAPA`).
    pub map: String,
    /// Optional `STAVBA_NA_PARCELE` wrapper; `Some` with no building means present-empty.
    pub building_on_parcel: Option<BuildingWrapper>,
}

/// One `STAVBA` entry in the property overview.
#[derive(Clone, Eq, PartialEq)]
pub struct OwnershipBuilding {
    /// All building forms from the required, possibly empty `CP_CE` wrapper.
    pub house_number_buildings: Vec<Building>,
    /// Required parcel from `PARCELNI_CISLO`.
    pub parcel: Parcel,
    /// Building-type description (`TYP_STAVBY`).
    pub building_type: String,
    /// Associated proceeding (`RIZENI`), including its valid empty form.
    pub proceeding: Proceeding,
}

/// One `JEDNOTKA` entry in the property overview.
#[derive(Clone, Eq, PartialEq)]
pub struct OwnershipUnit {
    /// Required `CISLO_JED` wrapper and its optional unit.
    pub unit: UnitWrapper,
    /// Unit-use description (`ZP_VYUZITI`).
    pub use_description: String,
    /// Share of the building (`BUD_PODIL`).
    pub building_share: ReportedShare,
    /// Required `BUDOVA` wrapper and its optional building.
    pub building: BuildingWrapper,
    /// Associated proceeding (`RIZENI`), including its valid empty form.
    pub proceeding: Proceeding,
}

/// One `ZDROJ` origin group in the property overview.
#[derive(Clone, Eq, PartialEq)]
pub struct OwnershipSource {
    /// Origin description (`PUVOD`).
    pub origin: String,
    /// Source parcels from the required `PARCELY_ZE` wrapper, which may be empty.
    pub parcels: Vec<OwnershipSourceParcel>,
}

/// One `PARCELA_ZE` source-parcel entry.
#[derive(Clone, Eq, PartialEq)]
pub struct OwnershipSourceParcel {
    /// Required source `PARCELNI_CISLO` wrapper and its optional parcel.
    pub parcel: ParcelWrapper,
    /// Parcel-part text (`DIL`).
    pub part: String,
    /// Source-parcel type (`TYP`).
    pub kind: String,
    /// Source area (`VYMERA`) as the XSD's full `xs:unsignedInt` range.
    pub area: u32,
    /// Required `PUVODNI_KU` wrapper and its optional complete code/name sequence.
    pub original_cadastral_area: OwnershipOriginalCadastralArea,
    /// Required current `PARCELA_KN` wrapper and its optional parcel.
    pub current_parcel: ParcelWrapper,
}

/// Presence-preserving content of a source parcel's `PUVODNI_KU` wrapper.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct OwnershipOriginalCadastralArea {
    /// Complete original cadastral-area code/name, or `None` for an empty sequence.
    pub cadastral_area: Option<LegacyReportCodeName>,
}

/// One `PRAVO_STAVBY` element, whose entire detail sequence may repeat or be empty.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct OwnershipBuildingRight {
    /// Ordered occurrences of the nested building-right detail sequence.
    pub entries: Vec<OwnershipBuildingRightEntry>,
}

/// One occurrence of the repeating detail sequence inside `PRAVO_STAVBY`.
#[derive(Clone, Eq, PartialEq)]
pub struct OwnershipBuildingRightEntry {
    /// Required building right from `PS_IDENT`.
    pub building_right: BuildingRight,
    /// Optional `UCELY_PRAVA_STAVBY` wrapper and its nonempty purpose list.
    pub purposes: Option<Vec<String>>,
    /// Lexical end of validity (`PLATNOST_DO`).
    pub valid_until: String,
    /// Associated proceeding (`RIZENI`), including its valid empty form.
    pub proceeding: Proceeding,
}

/// Complete legacy `EvidencePravProOsobu` rights record for a person or subject.
#[derive(Clone, Eq, PartialEq)]
pub struct RightsRecordReport {
    /// Search data echoed by `ZADANE_UDAJE`.
    pub query: RightsRecordQuery,
    /// Optional `UPLNA_SHODA` wrapper; its ownership sequence may independently be empty.
    pub exact_match: Option<RightsExactMatch>,
    /// Optional `NESHODA` wrapper, including a present-empty form.
    pub mismatch: Option<RightsMismatch>,
    /// Optional `CASTECNA_SHODA` wrapper, including a present-empty form.
    pub partial_match: Option<RightsPartialMatch>,
    /// Lexical validity timestamp (`PLATNOST`).
    pub valid_at: String,
    /// Lexical report creator (`VYHOTOVIL`).
    pub created_by: String,
    /// Free-access text (`BEZPL_PRISTUP`).
    pub free_access: String,
    /// Lexical report creation timestamp (`VYHOTOVENO`).
    pub created_at: String,
}

/// Input identification fields echoed by a rights record.
#[derive(Clone, Eq, PartialEq)]
pub struct RightsRecordQuery {
    /// Subject-kind text (`TYP_OS`).
    pub subject_kind: String,
    /// Subject name (`NAZEV_OS`).
    pub subject_name: String,
    /// Company identification number (`ICO`), retained verbatim.
    pub company_id: String,
    /// Surname (`PRIJMENI`).
    pub surname: String,
    /// Given name (`JMENO`).
    pub given_name: String,
    /// Personal identifier (`RC`), retained verbatim.
    pub personal_number: String,
}

/// Content of the optional `UPLNA_SHODA` wrapper.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct RightsExactMatch {
    /// Complete `VLASTNICTVI` block, or `None` when the outer sequence is empty.
    pub ownership: Option<RightsOwnership>,
}

/// Content of the optional `NESHODA` wrapper.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct RightsMismatch {
    /// Optional mismatch explanation (`TEXT`).
    pub text: Option<String>,
}

/// Content of the optional `CASTECNA_SHODA` wrapper.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct RightsPartialMatch {
    /// Complete text/subject sequence, or `None` for a present-empty wrapper.
    pub details: Option<RightsPartialMatchDetails>,
}

/// Nonempty form of a partial match.
#[derive(Clone, Eq, PartialEq)]
pub struct RightsPartialMatchDetails {
    /// Explanation text (`TEXT`).
    pub text: String,
    /// Nonempty ordered list of candidate subjects (`OPRAV_SUBJEKTY/OPSUB`).
    pub subjects: Vec<RightsPartialSubject>,
}

/// A partially matched subject without legal-relation details.
#[derive(Clone, Eq, PartialEq)]
pub struct RightsPartialSubject {
    /// Subject and partner names (`NAZEV`).
    pub name: SubjectNameForm,
    /// Restricted identification choice (`IDENT`), including its valid empty form.
    pub identification: LegacySubjectIdentification,
    /// Every address supplied by `ADRESA`, in document order.
    pub addresses: Vec<Address>,
    /// Optional `CHAROS` wrapper, which may itself be empty.
    pub characteristic: Option<CharacteristicWrapper>,
}

/// Complete `VLASTNICTVI` block in an exact match.
#[derive(Clone, Eq, PartialEq)]
pub struct RightsOwnership {
    /// Raw-preserving `ODPOVIDA_IDENTIFIKACI` value.
    pub identification_match: RightsIdentificationMatch,
    /// Nonempty ordered list of repeated `OPRAV_SUBJEKT` sequence occurrences.
    pub subjects: Vec<RightsSubject>,
    /// Explanation text (`TEXT`).
    pub text: String,
}

/// The single-value legacy identification-match code, with raw fallback.
#[derive(Clone, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum RightsIdentificationMatch {
    /// The returned subject matches the supplied identification (`a`).
    Matches,
    /// A future or otherwise unknown value, retained verbatim.
    Other(String),
}

impl RightsIdentificationMatch {
    /// Converts an untouched XML value to a known or raw-preserving variant.
    pub fn from_raw(raw: String) -> Self {
        match raw.as_str() {
            "a" => Self::Matches,
            _ => Self::Other(raw),
        }
    }

    /// Returns the official or untouched unknown value.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Matches => "a",
            Self::Other(raw) => raw,
        }
    }
}

/// One repeated exact-match subject sequence, including its legal relations.
#[derive(Clone, Eq, PartialEq)]
pub struct RightsSubject {
    /// Subject and partner names (`NAZEV`).
    pub name: SubjectNameForm,
    /// Restricted identification choice (`IDENT`), including its valid empty form.
    pub identification: LegacySubjectIdentification,
    /// Every address supplied by `ADRESA`, in document order.
    pub addresses: Vec<Address>,
    /// Optional `CHAROS` wrapper, which may itself be empty.
    pub characteristic: Option<CharacteristicWrapper>,
    /// Name-only legal-relation groups from `PRAV_VZTAHY`; the sequence may be empty.
    pub legal_relations: Vec<RightsLegalRelationGroup>,
}

/// One exact-match legal relation and its territorial ownership records.
#[derive(Clone, Eq, PartialEq)]
pub struct RightsLegalRelationGroup {
    /// Relation supplied by `TYPRAV_NAZEV`, with no inferred code or ownership classification.
    pub legal_relation: LegalRelation,
    /// Nonempty ordered list of districts (`OKRESY/OKRES`).
    pub districts: Vec<RightsDistrict>,
}

/// One district group in an exact-match legal relation.
#[derive(Clone, Eq, PartialEq)]
pub struct RightsDistrict {
    /// Nested district identification (`OKRES/OKRES`).
    pub district: LegacyReportDistrict,
    /// Nonempty ordered list of municipality groups (`OBCE/OBEC`).
    pub municipalities: Vec<RightsMunicipality>,
}

/// One municipality group in an exact-match legal relation.
#[derive(Clone, Eq, PartialEq)]
pub struct RightsMunicipality {
    /// Nested municipality code and name (`OBEC/OBEC`).
    pub municipality: LegacyReportCodeName,
    /// Nonempty ordered list of cadastral areas (`KATASTRY/KATASTR`).
    pub cadastral_areas: Vec<RightsCadastralArea>,
}

/// One cadastral area in an exact-match legal relation.
#[derive(Clone, Eq, PartialEq)]
pub struct RightsCadastralArea {
    /// Cadastral-area code and name (`KATUZE`).
    pub cadastral_area: LegacyReportCodeName,
    /// Nonempty ordered list of ownership records (`TELESA/TELESO`).
    pub records: Vec<RightsOwnershipRecord>,
}

/// One exact-match `TELESO` ownership record.
#[derive(Clone, Eq, PartialEq)]
pub struct RightsOwnershipRecord {
    /// Ownership-sheet number (`LV`), retained as text.
    pub ownership_sheet_number: String,
    /// Legal-relation/ownership-sheet text (`JPV_LV`).
    pub legal_relation_sheet: String,
    /// Ownership-sheet ID (`TEL_ID`), including a present-empty ID.
    pub ownership_sheet_id: LegacyId,
    /// Repeating parcel, building, unit, and building-right targets (`PRSV_IDENT`).
    pub properties: PropertyIdentification,
    /// Associated proceeding (`RIZENI`), including its valid empty form.
    pub proceeding: Proceeding,
    /// Reported ownership share (`PODIL`).
    pub share: ReportedShare,
}
