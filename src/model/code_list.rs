use jiff::civil::Date;

use super::legal_relation::LegalRelationType;
use super::{
    CadastralAreaCode, CityDistrictCode, DistrictCode, MessageCode, MunicipalityCode,
    MunicipalityPartCode, NewRegionCode, RegionCode, StreetCode, WorkplaceCode,
};

/// A typed request for one of the 33 WSDP 3.1 code-list operations.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum CodeListRequest {
    /// Lists available code lists, optionally restricted to records changed on or after a date.
    CodeListInventory {
        /// Inclusive `zmenyOd` filter.
        changed_since: Option<Date>,
    },
    /// Lists municipality parts (`seznamCastiObci`).
    MunicipalityParts {
        /// Optional single XSD choice used as the `kde` condition.
        filter: Option<MunicipalityPartFilter>,
    },
    /// Lists parcel-numbering methods (`seznamCislovaniParcel`).
    ParcelNumberingMethods,
    /// Lists land kinds (`seznamDruhuPozemku`).
    LandKinds,
    /// Lists historical cadastral areas (`seznamHistKU`).
    CadastralAreaHistory {
        /// Inclusive `zmenyOd` filter.
        changed_since: Option<Date>,
        /// Optional single XSD choice used as the `kde` condition.
        filter: Option<CadastralAreaHistoryFilter>,
    },
    /// Lists regions (`seznamKraju`).
    Regions,
    /// Lists current cadastral areas (`seznamKU`).
    CadastralAreas {
        /// Optional single XSD choice used as the `kde` condition.
        filter: Option<CadastralAreaFilter>,
    },
    /// Lists city districts (`seznamMestskychCasti`).
    CityDistricts {
        /// Optional single XSD choice used as the `kde` condition.
        filter: Option<CityDistrictFilter>,
    },
    /// Lists municipalities (`seznamObci`).
    Municipalities {
        /// Optional single XSD choice used as the `kde` condition.
        filter: Option<MunicipalityFilter>,
    },
    /// Lists districts (`seznamOkresu`).
    Districts {
        /// Optional single XSD choice used as the `kde` condition.
        filter: Option<DistrictFilter>,
    },
    /// Lists proceeding operations (`seznamOperaciRizeni`).
    ProceedingOperations,
    /// Lists cadastral workplaces (`seznamPracovist`).
    Workplaces {
        /// Optional single XSD choice used as the `kde` condition.
        filter: Option<WorkplaceFilter>,
    },
    /// Lists proceeding subject matters (`seznamPredmetuRizeni`).
    ProceedingSubjects,
    /// Lists countries (`seznamStatu`).
    Countries {
        /// Optional country-name condition.
        filter: Option<CountryFilter>,
    },
    /// Lists unit types (`seznamTypuJednotek`).
    UnitTypes,
    /// Lists operation types (`seznamTypuOperaci`).
    OperationTypes,
    /// Lists authorized-subject types (`seznamTypuOS`).
    SubjectTypes,
    /// Lists parcel types from `seznamTypuParcel`.
    ParcelTypes,
    /// Lists legal-relation types (`seznamTypuPravnichVztahu`).
    LegalRelationTypes,
    /// Lists proceeding types (`seznamTypuRizeni`).
    ProceedingTypes,
    /// Lists building types (`seznamTypuStaveb`).
    BuildingTypes,
    /// Lists request purposes (`seznamUcelu`).
    RequestPurposes,
    /// Lists streets (`seznamUlic`).
    Streets {
        /// Optional single XSD choice used as the `kde` condition.
        filter: Option<StreetFilter>,
    },
    /// Lists area-determination methods (`seznamUrceniVymery`).
    AreaDeterminationMethods,
    /// Lists unit uses (`seznamVyuzitiJednotky`).
    UnitUses,
    /// Lists land uses (`seznamVyuzitiPozemku`).
    LandUses,
    /// Lists building uses (`seznamVyuzitiStavby`).
    BuildingUses,
    /// Lists protection methods (`seznamZpusobOchrany`).
    ProtectionMethods,
    /// Lists simplified-register numbering sources (`seznamZdrojuCislovaniZE`).
    SimplifiedRegisterNumberingSources,
    /// Lists service-message definitions (`seznamZprav`).
    ServiceMessages,
    /// Lists building-right purposes (`seznamUceluPS`).
    BuildingRightPurposes,
    /// Lists parcel kinds from the distinct `seznamTypParcely` operation.
    ParcelKinds,
    /// Lists authorized-subject characteristics (`seznamCharakteristikOS`).
    SubjectCharacteristics,
}

/// The single `kde` choice supported by `seznamCastiObci`.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum MunicipalityPartFilter {
    /// Selects parts belonging to this municipality (`kodObce/odpovida`).
    Municipality(MunicipalityCode),
    /// Selects an exact municipality-part code (`kodCastiObce/odpovida`).
    Code(MunicipalityPartCode),
    /// Selects names using the supplied SQL-LIKE pattern (`nazevCastiObce/obsahuje`).
    NameContains(String),
}

/// The single `kde` choice supported by `seznamHistKU`.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum CadastralAreaHistoryFilter {
    /// Selects an exact cadastral-area code (`kodKU/odpovida`).
    Code(CadastralAreaCode),
    /// Selects names using the supplied SQL-LIKE pattern (`nazevKU/obsahuje`).
    NameContains(String),
}

/// The single `kde` choice supported by `seznamKU`.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum CadastralAreaFilter {
    /// Selects areas belonging to this municipality (`kodObce/odpovida`).
    Municipality(MunicipalityCode),
    /// Selects an exact cadastral-area code (`kodKU/odpovida`).
    Code(CadastralAreaCode),
    /// Selects names using the supplied SQL-LIKE pattern (`nazevKU/obsahuje`).
    NameContains(String),
}

/// The single `kde` choice supported by `seznamMestskychCasti`.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum CityDistrictFilter {
    /// Selects districts belonging to this municipality (`kodObce/odpovida`).
    Municipality(MunicipalityCode),
    /// Selects an exact city-district code (`kodMC/odpovida`).
    Code(CityDistrictCode),
    /// Selects names using the supplied SQL-LIKE pattern (`nazevMC/obsahuje`).
    NameContains(String),
}

/// The single `kde` choice supported by `seznamObci`.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum MunicipalityFilter {
    /// Selects an exact municipality code (`kodObce/odpovida`).
    Code(MunicipalityCode),
    /// Selects names using the supplied SQL-LIKE pattern (`nazevObce/obsahuje`).
    NameContains(String),
    /// Selects municipalities in this district (`kodOkresu/odpovida`).
    District(DistrictCode),
}

/// The single `kde` choice supported by `seznamOkresu`.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum DistrictFilter {
    /// Selects an exact district code (`kodOkresu/odpovida`).
    Code(DistrictCode),
    /// Selects names using the supplied SQL-LIKE pattern (`nazevOkresu/obsahuje`).
    NameContains(String),
    /// Selects districts in this post-1960 region/VÚSC (`kodNovehoKraje/odpovida`).
    NewRegion(NewRegionCode),
}

/// The single `kde` choice supported by `seznamPracovist`.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum WorkplaceFilter {
    /// Selects an exact workplace code (`kodKP/odpovida`).
    Code(WorkplaceCode),
    /// Selects names using the supplied SQL-LIKE pattern (`nazevKP/obsahuje`).
    NameContains(String),
    /// Selects an exact email address (`email/odpovida`).
    Email(String),
    /// Selects an exact data-box identifier (`idDS/odpovida`).
    DataBoxId(String),
}

/// The single `kde` choice supported by `seznamStatu`.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum CountryFilter {
    /// Selects names using the supplied SQL-LIKE pattern (`nazevStatu/obsahuje`).
    NameContains(String),
}

/// The single `kde` choice supported by `seznamUlic`.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum StreetFilter {
    /// Selects streets belonging to this municipality (`kodObce/odpovida`).
    Municipality(MunicipalityCode),
    /// Selects an exact street code (`kodUlice/odpovida`).
    Code(StreetCode),
    /// Selects names using the supplied SQL-LIKE pattern (`nazevUlice/obsahuje`).
    NameContains(String),
}

/// A decoded code list with its optional response-level update date.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodeList<T> {
    /// Every returned item, including an empty list when the server returns no item elements.
    pub items: Vec<T>,
    /// The response attribute `posledniAktualizaceCiselniku`, when present.
    pub last_updated: Option<Date>,
}

/// The typed result corresponding to a [`CodeListRequest`] variant.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum CodeListResult {
    /// Result of `seznamCiselniku`.
    CodeListInventory(CodeList<CodeListDescription>),
    /// Result of `seznamCastiObci`.
    MunicipalityParts(CodeList<MunicipalityPart>),
    /// Result of `seznamCislovaniParcel`.
    ParcelNumberingMethods(CodeList<CodeListItem>),
    /// Result of `seznamDruhuPozemku`.
    LandKinds(CodeList<CodeListItem>),
    /// Result of `seznamHistKU`.
    CadastralAreaHistory(CodeList<HistoricalCadastralArea>),
    /// Result of `seznamKraju`.
    Regions(CodeList<Region>),
    /// Result of `seznamKU`.
    CadastralAreas(CodeList<CadastralArea>),
    /// Result of `seznamMestskychCasti`.
    CityDistricts(CodeList<CityDistrict>),
    /// Result of `seznamObci`.
    Municipalities(CodeList<Municipality>),
    /// Result of `seznamOkresu`.
    Districts(CodeList<District>),
    /// Result of `seznamOperaciRizeni`.
    ProceedingOperations(CodeList<ProceedingOperation>),
    /// Result of `seznamPracovist`.
    Workplaces(CodeList<Workplace>),
    /// Result of `seznamPredmetuRizeni`.
    ProceedingSubjects(CodeList<CodeListItem>),
    /// Result of `seznamStatu`.
    Countries(CodeList<CodeListItem>),
    /// Result of `seznamTypuJednotek`.
    UnitTypes(CodeList<CodeListItem>),
    /// Result of `seznamTypuOperaci`.
    OperationTypes(CodeList<CodeListItem>),
    /// Result of `seznamTypuOS`.
    SubjectTypes(CodeList<CodeListItem>),
    /// Result of `seznamTypuParcel`.
    ParcelTypes(CodeList<CodeListItem>),
    /// Result of `seznamTypuPravnichVztahu`.
    LegalRelationTypes(CodeList<LegalRelationType>),
    /// Result of `seznamTypuRizeni`.
    ProceedingTypes(CodeList<CodeListItem>),
    /// Result of `seznamTypuStaveb`.
    BuildingTypes(CodeList<CodeListItem>),
    /// Result of `seznamUcelu`.
    RequestPurposes(CodeList<CodeListItem>),
    /// Result of `seznamUlic`.
    Streets(CodeList<Street>),
    /// Result of `seznamUrceniVymery`.
    AreaDeterminationMethods(CodeList<CodeListItem>),
    /// Result of `seznamVyuzitiJednotky`.
    UnitUses(CodeList<CodeListItem>),
    /// Result of `seznamVyuzitiPozemku`.
    LandUses(CodeList<CodeListItem>),
    /// Result of `seznamVyuzitiStavby`.
    BuildingUses(CodeList<CodeListItem>),
    /// Result of `seznamZpusobOchrany`.
    ProtectionMethods(CodeList<CodeListItem>),
    /// Result of `seznamZdrojuCislovaniZE`.
    SimplifiedRegisterNumberingSources(CodeList<SimplifiedRegisterNumberingSource>),
    /// Result of `seznamZprav`.
    ServiceMessages(CodeList<ServiceMessageInfo>),
    /// Result of `seznamUceluPS`.
    BuildingRightPurposes(CodeList<CodeListItem>),
    /// Result of the distinct `seznamTypParcely` operation.
    ParcelKinds(CodeList<CodeListItem>),
    /// Result of `seznamCharakteristikOS`.
    SubjectCharacteristics(CodeList<SubjectCharacteristic>),
}

/// One available code list returned by `seznamCiselniku`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodeListDescription {
    /// Operation name of the available code list.
    pub name: String,
    /// Required `datumPosledniZmeny` value for this individual code list.
    pub last_changed: Date,
}

/// A code/name pair used by operations without additional response fields.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodeListItem {
    /// Raw server-maintained code, retained without numeric conversion or known-value restrictions.
    pub code: String,
    /// Human-readable code-list name.
    pub name: String,
}

/// A region returned by `seznamKraju`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Region {
    /// Region code.
    pub code: RegionCode,
    /// Region name.
    pub name: String,
}

/// A current cadastral area returned by `seznamKU`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CadastralArea {
    /// Cadastral-area code.
    pub code: CadastralAreaCode,
    /// Cadastral-area name.
    pub name: String,
    /// Optional parent municipality code (`kodObce`).
    pub municipality_code: Option<MunicipalityCode>,
}

/// A historical cadastral-area record returned by `seznamHistKU`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoricalCadastralArea {
    /// Cadastral-area code.
    pub code: CadastralAreaCode,
    /// Historical cadastral-area name.
    pub name: String,
    /// Optional parent municipality code (`obecKod`).
    pub municipality_code: Option<MunicipalityCode>,
    /// Inclusive start of validity (`platnostOd`).
    pub valid_from: Date,
    /// Optional end of validity (`platnostDo`).
    pub valid_to: Option<Date>,
}

/// A city district returned by `seznamMestskychCasti`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CityDistrict {
    /// City-district code.
    pub code: CityDistrictCode,
    /// City-district name.
    pub name: String,
    /// Required parent municipality code (`kodObce`).
    pub municipality_code: MunicipalityCode,
}

/// A municipality returned by `seznamObci`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Municipality {
    /// Municipality code.
    pub code: MunicipalityCode,
    /// Municipality name.
    pub name: String,
    /// Parent district code (`okresKod`).
    pub district_code: DistrictCode,
}

/// A municipality part returned by `seznamCastiObci`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MunicipalityPart {
    /// Municipality-part code.
    pub code: MunicipalityPartCode,
    /// Municipality-part name.
    pub name: String,
    /// Parent municipality code (`obecKod`).
    pub municipality_code: MunicipalityCode,
}

/// A district returned by `seznamOkresu`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct District {
    /// District code.
    pub code: DistrictCode,
    /// District name.
    pub name: String,
    /// Parent post-1960 region/VÚSC code (`nkrajKod`).
    pub new_region_code: NewRegionCode,
}

/// A street returned by `seznamUlic`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Street {
    /// Street code.
    pub code: StreetCode,
    /// Street name.
    pub name: String,
    /// Parent municipality code (`kodObce`).
    pub municipality_code: MunicipalityCode,
}

/// A proceeding operation returned by `seznamOperaciRizeni`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProceedingOperation {
    /// Raw operation code.
    pub code: String,
    /// Operation name.
    pub name: String,
    /// Optional unrestricted operation state (`stav`).
    pub state: Option<String>,
}

/// A cadastral workplace with all address and contact fields from `seznamPracovist`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Workplace {
    /// Workplace code.
    pub code: WorkplaceCode,
    /// Workplace name.
    pub name: String,
    /// Optional post-1960 region/VÚSC code (`nkrajKod`).
    pub new_region_code: Option<NewRegionCode>,
    /// Optional email address.
    pub email: Option<String>,
    /// Optional data-box identifier (`datovaSchranka`).
    pub data_box_id: Option<String>,
    /// Optional street name (`nazevUlice`).
    pub street_name: Option<String>,
    /// Optional building/house number (`cisloDomovni`), retaining its lexical representation.
    pub house_number: Option<String>,
    /// Optional orientation number (`cisloOrientacni`).
    pub orientation_number: Option<String>,
    /// Optional postal code (`psc`), retaining leading zeroes.
    pub postal_code: Option<String>,
    /// Optional municipality name (`obec`).
    pub municipality: Option<String>,
    /// Optional municipality-part name (`castObce`).
    pub municipality_part: Option<String>,
    /// Optional district name (`okres`).
    pub district: Option<String>,
}

/// A simplified-register parcel-numbering source returned by `seznamZdrojuCislovaniZE`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SimplifiedRegisterNumberingSource {
    /// Raw source code.
    pub code: String,
    /// Source name.
    pub name: String,
    /// Required source abbreviation (`zkratka`).
    pub abbreviation: String,
}

/// A service-message definition returned by `seznamZprav`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServiceMessageInfo {
    /// Known message code or its untouched unknown lexical value.
    pub code: MessageCode,
    /// Raw one-character message type (`typ`).
    pub kind: String,
    /// Human-readable message name.
    pub name: String,
}

/// An authorized-subject characteristic returned by `seznamCharakteristikOS`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubjectCharacteristic {
    /// Raw characteristic code.
    pub code: String,
    /// Characteristic name.
    pub name: String,
    /// Optional standard abbreviation (`zkratka`).
    pub abbreviation: Option<String>,
    /// Optional ALV abbreviation (`zkratkaALV`).
    pub alv_abbreviation: Option<String>,
}
