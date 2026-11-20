//! Complete domain models for legacy unit, building-right, and proceeding reports.
//!
//! These reports have no XML namespace. Text and dynamic codes are retained
//! without trimming. An `Option<Vec<T>>` represents an optional XML wrapper:
//! `None` means absent and `Some(vec![])` means present but empty where the XSD
//! permits that form. Plain vectors represent required wrappers.

use super::legacy_base::{
    Address, Building, BuildingRight, CharacteristicWrapper, LegacyId, Number102,
    OwnershipIdentification, Parcel, Proceeding, ProceedingIdentification, ReportedShare, Unit,
};

/// A district (`OKRES`) shared by legacy unit and building-right reports.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct LegacyDistrict {
    /// Optional NUTS 4 code (`nuts4`).
    pub nuts4: Option<String>,
    /// Optional district name (`nazev`).
    pub name: Option<String>,
}

/// A required numeric code and name pair using XSD `xs:unsignedInt`.
#[derive(Clone, Eq, PartialEq)]
pub struct LegacyNumericCodeName {
    /// Numeric code (`kod`).
    pub code: u32,
    /// Display name (`nazev`).
    pub name: String,
}

/// A required textual code and name pair.
#[derive(Clone, Eq, PartialEq)]
pub struct LegacyTextCodeName {
    /// Dynamic code (`kod`), retained verbatim.
    pub code: String,
    /// Display name (`nazev`).
    pub name: String,
}

/// Complete legacy `InformaceOJednotkach` report (unit information 1.0).
#[derive(Clone, Eq, PartialEq)]
pub struct UnitInfoReport {
    /// Required cadastral-area report body (`KATASTR`).
    pub cadastral_area: UnitInfoCadastralArea,
    /// Total number of units (`CS_POCET_JEDNOTEK`).
    pub total_units: u32,
    /// Lexical validity timestamp (`PLATNOST`).
    pub valid_at: String,
    /// Report creator (`VYHOTOVIL`).
    pub created_by: String,
    /// Free-access marker (`BEZPL_PRISTUP`), retained verbatim.
    pub free_access: String,
    /// Lexical report creation timestamp (`VYHOTOVENO`).
    pub created_at: String,
    /// Optional output information (`INFO_VYSTUP`).
    pub output_info: Option<String>,
}

/// The required `KATASTR` body of a unit-information report.
#[derive(Clone, Eq, PartialEq)]
pub struct UnitInfoCadastralArea {
    /// Concatenated direct character content permitted by `mixed="true"`.
    /// Character content inside child elements is not duplicated here.
    pub text: String,
    /// District (`OKRES`).
    pub district: LegacyDistrict,
    /// Municipality (`OBEC`).
    pub municipality: LegacyNumericCodeName,
    /// Cadastral-area identification (`KATUZE`).
    pub cadastral_area: LegacyNumericCodeName,
    /// Every building in `STAVBY`; the XSD requires at least one.
    pub buildings: Vec<UnitInfoBuilding>,
}

/// One `STAVBA` occurrence in a unit-information report.
#[derive(Clone, Eq, PartialEq)]
pub struct UnitInfoBuilding {
    /// Building-type code (`TYPBUD_KOD`), covering all of `xs:unsignedByte`.
    pub building_type_code: u8,
    /// House number (`CISLO_DOMOVNI`).
    pub house_number: String,
    /// Dynamic building-use code (`ZPVYBU_KOD`), retained verbatim.
    pub building_use_code: String,
    /// Optional nested building identification in the required `BUDOVA` wrapper.
    pub building: Option<Building>,
    /// Building parcels from `BUD_PARCELY`; at least one is required.
    pub parcels: Vec<UnitInfoBuildingParcel>,
    /// Units from `JEDNOTKY`; at least one is required.
    pub units: Vec<UnitInfoUnit>,
}

/// One building-associated `PARCELA` under `BUD_PARCELY`.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct UnitInfoBuildingParcel {
    /// Optional parcel identification (`parcela`).
    pub parcel: Option<Parcel>,
    /// Optional ownership-sheet ID (`tel_id`), including a present-empty ID.
    pub ownership_sheet_id: Option<LegacyId>,
    /// Optional ownership-sheet number (`cislo_tel`).
    pub ownership_sheet_number: Option<String>,
}

/// One `JEDNOTKA` occurrence in a unit-information report.
#[derive(Clone, Eq, PartialEq)]
pub struct UnitInfoUnit {
    /// Optional unit count (`POCET_JEDNOTEK`).
    pub unit_count: Option<u32>,
    /// Optional dynamic use code (`ZP_VYUZITI`), retained verbatim.
    pub use_code: Option<String>,
    /// Optional unit identification inside the required `CISLO_JED` wrapper.
    pub unit: Option<Unit>,
    /// Unit type/use description (`VYUZ_TYP`).
    pub unit_type: String,
    /// Building share (`BUD_PODIL`).
    pub building_share: ReportedShare,
    /// Proceeding (`RIZENI`), including its XSD-permitted empty form.
    pub proceeding: Proceeding,
    /// Floor area (`PODLAHOVA_PLOCHA`).
    ///
    /// The XSD requires this element. `None` is a deliberately narrow
    /// compatibility exception for official generator output exemplified by
    /// `170619983011_InfOJedn_10Kc.xml`, which omits only this required field.
    /// `Some` with `value == None` instead represents a present empty element.
    pub floor_area: Option<UnitInfoFloorArea>,
    /// Optional legal-relation link wrapper (`VAZBA_JPV`).
    pub legal_relation_link: Option<UnitInfoLegalRelationLink>,
    /// Required ownership wrapper (`VLASTNICTVI`).
    pub ownership: UnitInfoOwnership,
    /// Parcels in the required `PARCELY` wrapper; it may be empty.
    pub parcels: Vec<UnitInfoParcel>,
    /// Seals in the required `PLOMBY` wrapper; it may be empty.
    pub seals: Vec<UnitInfoSeal>,
}

/// The value of a present `PODLAHOVA_PLOCHA` element.
#[derive(Clone, Copy, Default, Eq, PartialEq)]
pub struct UnitInfoFloorArea {
    /// Exact `Number102Type` value, or `None` for the union's empty-string form.
    pub value: Option<Number102>,
}

/// Optional `VAZBA_JPV` content associated with a unit.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct UnitInfoLegalRelationLink {
    /// Optional link text (`TEXT`); an empty wrapper has no text field.
    pub text: Option<String>,
}

/// Required `VLASTNICTVI` wrapper for a unit.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct UnitInfoOwnership {
    /// Complete ownership details, or `None` for the wrapper's empty sequence.
    /// When present, both the ownership sheet block and subject list exist.
    pub details: Option<UnitInfoOwnershipDetails>,
}

/// Non-empty content of a unit's `VLASTNICTVI` wrapper.
#[derive(Clone, Eq, PartialEq)]
pub struct UnitInfoOwnershipDetails {
    /// Required `LV` block, whose own sequence may be empty.
    pub ownership_sheet: Option<UnitInfoOwnershipSheet>,
    /// Subjects from `SUBJEKTY`; the non-empty ownership form requires at least one.
    pub subjects: Vec<UnitInfoSubject>,
}

/// The optional sequence inside a unit's required `LV` element.
#[derive(Clone, Eq, PartialEq)]
pub struct UnitInfoOwnershipSheet {
    /// Ownership-sheet ID (`tel_id`), including a present-empty ID.
    pub id: LegacyId,
    /// Ownership-sheet number (`cislo_tel`).
    pub number: String,
}

/// One `SUBJEKT` in unit ownership.
#[derive(Clone, Eq, PartialEq)]
pub struct UnitInfoSubject {
    /// Ordered ownership targets (`VLA_IDENT`).
    pub identification: OwnershipIdentification,
    /// Ownership share (`PODIL`), with independently optional components.
    pub share: ReportedShare,
    /// Dynamic legal-relation name (`TYPRAV_NAZEV`), retained verbatim.
    pub legal_relation_name: String,
    /// Required characteristic wrapper (`CHAROS`).
    pub characteristic: CharacteristicWrapper,
    /// Optional `G_NAZ_VLA_PS`, containing its required `VLA_IDENT2` element.
    pub alternate_identification: Option<OwnershipIdentification>,
}

/// One parcel entry in a unit's `PARCELY` wrapper.
#[derive(Clone, Eq, PartialEq)]
pub struct UnitInfoParcel {
    /// Optional parcel in the required `PARCELNI_CISLO` wrapper.
    pub parcel: Option<Parcel>,
    /// Required ownership-sheet reference (`PAR_TEL`).
    pub ownership_sheet: UnitInfoParcelOwnershipSheet,
    /// Dynamic parcel ownership/relation name (`PAR_TYP_VLA`).
    pub ownership_type: String,
    /// Sealing proceedings from `PAR_PLOMBY_RIZENI`; the wrapper may be empty.
    pub seal_proceedings: Vec<UnitInfoParcelSealProceeding>,
}

/// Required `PAR_TEL` reference attached to a unit parcel.
#[derive(Clone, Eq, PartialEq)]
pub struct UnitInfoParcelOwnershipSheet {
    /// Ownership-sheet ID (`tel_id`), including a present-empty ID.
    pub id: LegacyId,
    /// Numeric ownership-sheet number (`cislo_tel`).
    pub number: u32,
}

/// One mixed-content `PAR_PLOMBA_RIZENI` occurrence.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct UnitInfoParcelSealProceeding {
    /// Concatenated direct character content permitted by `mixed="true"`.
    pub text: String,
    /// Complete proceeding identification, or `None` for the empty sequence.
    pub identification: Option<ProceedingIdentification>,
}

/// One mixed-content `PLOMBA` occurrence for a unit.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct UnitInfoSeal {
    /// Concatenated direct character content permitted by `mixed="true"`.
    pub text: String,
    /// Every `PLOMBA_RIZENI`; the optional sequence may be empty.
    pub proceedings: Vec<Proceeding>,
}

/// Complete legacy `InformaceOPravechStavby` report (building-right information 1.0).
#[derive(Clone, Eq, PartialEq)]
pub struct BuildingRightInfoReport {
    /// Required report header (`ZAHLAVI`).
    pub header: BuildingRightInfoHeader,
    /// Free-access marker (`BEZPL_PRISTUP`), retained verbatim.
    pub free_access: String,
    /// Optional lexical validity timestamp (`PLATNOST_K`).
    pub valid_at: Option<String>,
    /// Report creator (`VYHOTOVIL`).
    pub created_by: String,
    /// Optional output information (`INFO_VYSTUP`).
    pub output_info: Option<String>,
    /// Lexical report creation timestamp (`VYHOTOVENO`).
    pub created_at: String,
}

/// Required `ZAHLAVI` section of a building-right report.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct BuildingRightInfoHeader {
    /// Optional district (`OKRES`).
    pub district: Option<LegacyDistrict>,
    /// Optional municipality (`OBEC`) with a textual code.
    pub municipality: Option<LegacyTextCodeName>,
    /// Optional cadastral area (`KU`) with a textual code.
    pub cadastral_area: Option<LegacyTextCodeName>,
    /// Optional `PRAVA_STAVEB` wrapper. If present, at least one record is required.
    pub records: Option<Vec<BuildingRightInfoRecord>>,
}

/// One `ZAZNAM` building-right record.
#[derive(Clone, Eq, PartialEq)]
pub struct BuildingRightInfoRecord {
    /// Lexical end of validity (`PLATNOST_DO`).
    pub valid_until: String,
    /// Protection methods (`ZP_OCHRANY/zpochr`); the sequence may be empty.
    pub protection_methods: Vec<String>,
    /// Optional proceeding identification (`IDENT_RIZENI`), whose fields are text.
    pub proceeding: Option<BuildingRightInfoProceedingIdentification>,
    /// Optional `UCELY_PRAVA_STAVBY` wrapper; each purpose retains its text.
    pub purposes: Option<Vec<String>>,
    /// Optional building-right identification from `PS_IDENT`.
    pub building_right: Option<BuildingRight>,
    /// Building components from required `SOUCASTI_STAVBA`; it may be empty.
    pub building_components: Vec<BuildingRightInfoComponent>,
    /// Optional `LISTY_VLASTNICTVI` wrapper, which may be present and empty.
    pub ownership_sheets: Option<Vec<BuildingRightInfoOwnershipSheet>>,
}

/// Textual proceeding identification in building-right `IDENT_RIZENI`.
#[derive(Clone, Eq, PartialEq)]
pub struct BuildingRightInfoProceedingIdentification {
    /// Dynamic proceeding-type code (`typriz_kod`).
    pub type_code: String,
    /// Serial number (`poradove_cislo`), intentionally retained as schema text.
    pub serial_number: String,
    /// Year (`rok`), intentionally retained as schema text.
    pub year: String,
    /// Workplace code (`prares_kod`), intentionally retained as schema text.
    pub workplace_code: String,
}

/// One occurrence of the repeating `SOUCASTI_STAVBA` sequence.
#[derive(Clone, Eq, PartialEq)]
pub struct BuildingRightInfoComponent {
    /// Required building identification (`stavba`).
    pub building: Building,
    /// Optional direct temporary-building marker (`docasna`).
    pub temporary: Option<String>,
}

/// One `LIST_VLASTNICTVI` occurrence.
#[derive(Clone, Eq, PartialEq)]
pub struct BuildingRightInfoOwnershipSheet {
    /// Required ownership-sheet reference (`LV`).
    pub sheet: BuildingRightInfoOwnershipSheetReference,
    /// Ownership entries (`VLASTNICTVI`); at least one is required.
    pub ownerships: Vec<BuildingRightInfoOwnership>,
}

/// Required `LV` reference in a building-right ownership-sheet entry.
#[derive(Clone, Eq, PartialEq)]
pub struct BuildingRightInfoOwnershipSheetReference {
    /// Optional ownership-sheet ID (`tel_id`), including a present-empty ID.
    pub id: Option<LegacyId>,
    /// Required ownership-sheet number (`cislo_tel`).
    pub number: String,
}

/// One building-right `VLASTNICTVI` occurrence.
#[derive(Clone, Eq, PartialEq)]
pub struct BuildingRightInfoOwnership {
    /// Ordered ownership targets (`VLA_IDENT`).
    pub identification: OwnershipIdentification,
    /// Ownership share (`PODIL`).
    pub share: ReportedShare,
    /// Required characteristic wrapper (`CHAROS`).
    pub characteristic: CharacteristicWrapper,
    /// Dynamic relationship type (`TYP_VZTAHU`), retained verbatim.
    pub relationship_type: String,
    /// Every required-list `VLA_IDENT2` occurrence; at least one is required.
    pub alternate_identifications: Vec<OwnershipIdentification>,
}

/// Complete legacy `InformaceORizeni` report (proceeding information 1.0).
#[derive(Clone, Eq, PartialEq)]
pub struct ProceedingInfoReport {
    /// Participants from required `UCASTNICI`; the list may be empty.
    pub participants: Vec<ProceedingInfoParticipant>,
    /// Required proceeding details (`RIZENI`).
    pub proceeding: ProceedingInfoDetails,
    /// Proceeding subjects from `PREDMETY_RIZENI`; the list may be empty.
    pub subjects: Vec<ProceedingInfoSubject>,
    /// Cadastral-area groups from `KATASTRY`; the outer list may be empty.
    pub cadastral_areas: Vec<ProceedingInfoCadastralAreaGroup>,
    /// Parcel objects from `OBJEKTY_PARCELY`; the list may be empty.
    pub parcels: Vec<ProceedingInfoParcel>,
    /// Building objects from `OBJEKTY_STAVBY`; the list may be empty.
    pub buildings: Vec<ProceedingInfoBuilding>,
    /// Deeds from `LISTINY`; the list may be empty.
    pub deeds: Vec<ProceedingInfoDeed>,
    /// Unit objects from `OBJEKTY_JED`; the list may be empty.
    pub units: Vec<ProceedingInfoUnit>,
    /// Optional geometric plan (`GEOM_PLAN`).
    pub geometric_plan: Option<ProceedingInfoGeometricPlan>,
    /// Events from `UDALOSTI`; the list may be empty.
    pub events: Vec<ProceedingInfoEvent>,
    /// Performed operations from `VYK_OPERACE`; the list may be empty.
    pub operations: Vec<ProceedingInfoOperation>,
    /// Geometric-plan defects from `ZAVADY_GP`; the list may be empty.
    pub geometric_plan_defects: Vec<ProceedingInfoDefect>,
    /// Change proposals from `NAVRHY_ZMEN`; the list may be empty.
    pub change_proposals: Vec<ProceedingInfoChangeProposal>,
    /// Building-right objects from `OBJEKTY_PRAVA_STAVBY`; the list may be empty.
    pub building_rights: Vec<ProceedingInfoBuildingRight>,
    /// Report creator (`VYHOTOVIL`).
    pub created_by: String,
    /// Free-access marker (`BEZPL_PRISTUP`), retained verbatim.
    pub free_access: String,
    /// Lexical report creation timestamp (`VYHOTOVENO`).
    pub created_at: String,
    /// Optional output information (`INFO_VYSTUP`).
    pub output_info: Option<String>,
}

/// One `UCASTNIK` in a proceeding report.
#[derive(Clone, Eq, PartialEq)]
pub struct ProceedingInfoParticipant {
    /// Required full-name wrapper (`CELE_JMENO`), including its empty choice.
    pub name: ProceedingInfoParticipantName,
    /// Ordered identification choices (`IDENTIFIKACE`).
    pub identification: ProceedingInfoParticipantIdentification,
    /// Every required-list `UCAST_ADRESA`; each item may have an empty sequence.
    pub addresses: Vec<Option<Address>>,
    /// Every required-list `TYP_UCAST`; each item may have an empty sequence.
    pub participation_kinds: Vec<ProceedingInfoParticipationKind>,
}

/// Required `CELE_JMENO` wrapper for a proceeding participant.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct ProceedingInfoParticipantName {
    /// Person or organization choice, or `None` for the XSD's empty choice.
    pub value: Option<ProceedingInfoParticipantNameChoice>,
}

/// The person-or-organization choice in `CELE_JMENO`.
#[derive(Clone, Eq, PartialEq)]
#[non_exhaustive]
pub enum ProceedingInfoParticipantNameChoice {
    /// Natural-person name sequence.
    Person(ProceedingInfoPersonName),
    /// Organization/trade name (`obchodni_jmeno`).
    Organization(String),
}

/// Natural-person name fields in a proceeding participant.
#[derive(Clone, Eq, PartialEq)]
pub struct ProceedingInfoPersonName {
    /// Surname (`prijmeni`).
    pub surname: String,
    /// Given name (`jmeno`).
    pub given_name: String,
    /// Optional title before the name (`titul_pred_jmenem`).
    pub title_before: Option<String>,
    /// Optional title after the name (`titul_za_jmenem`).
    pub title_after: Option<String>,
}

/// Repeating participant identification choices in `IDENTIFIKACE`.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct ProceedingInfoParticipantIdentification {
    /// Every choice entry in document order; the choice may be empty or repeated.
    pub entries: Vec<ProceedingInfoParticipantIdentificationEntry>,
}

/// One participant identification choice entry.
#[derive(Clone, Eq, PartialEq)]
#[non_exhaustive]
pub enum ProceedingInfoParticipantIdentificationEntry {
    /// Internal subject ID (`id`), including a present-empty ID.
    Id(LegacyId),
    /// Personal identifier fragment (`rc6`), retained as text.
    PersonalNumber6(String),
    /// Personal identifier fragment (`rc7`), retained as text.
    PersonalNumber7(String),
    /// Company identification number (`ico`), retained as text.
    CompanyId(String),
    /// Company-ID suffix (`ico_doplnek`), retained as text.
    CompanyIdSuffix(String),
    /// Subject name (`nazev`), retained as text.
    Name(String),
}

/// One `TYP_UCAST` occurrence.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct ProceedingInfoParticipationKind {
    /// Complete code/name pair (`TYPUCA_KOD`, `TYPUCA_NAZEV`), or `None` for an
    /// empty optional sequence. The generic field names map to those elements.
    pub details: Option<LegacyTextCodeName>,
}

/// Required `RIZENI` details in a proceeding-information report.
#[derive(Clone, Eq, PartialEq)]
pub struct ProceedingInfoDetails {
    /// Lexical assignment timestamp (`PRIRAZENO`).
    pub assigned_at: String,
    /// Lexical submission date (`DATUM_PODANI`).
    pub submitted_at: String,
    /// Fee-exemption marker (`OSVOBOZENO`), retained verbatim.
    pub exempt: String,
    /// Proceeding number (`CISLO_RIZENI`), including its empty form.
    pub number: Proceeding,
    /// Workplace name (`PRAC_NAZEV`).
    pub workplace_name: String,
    /// Proceeding-type name (`TYPRIZ_NAZEV`).
    pub proceeding_type_name: String,
    /// Dynamic proceeding state (`STAV`), retained verbatim.
    pub state: String,
    /// Lexical creation timestamp (`VYTVORENO`).
    pub created_at: String,
    /// Required payment-state wrapper (`STAV_UHRADY`).
    pub payment_state: ProceedingInfoPaymentState,
}

/// Required `STAV_UHRADY` wrapper with an optional code/name sequence.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct ProceedingInfoPaymentState {
    /// Complete payment-state code/name pair, or `None` for an empty wrapper.
    pub details: Option<LegacyTextCodeName>,
}

/// One `PREDMET` of a proceeding.
#[derive(Clone, Eq, PartialEq)]
pub struct ProceedingInfoSubject {
    /// Dynamic subject-type code (`TYPPRE_KOD`), retained verbatim.
    pub type_code: String,
    /// Subject-type name (`TYPPRE_NAZEV`).
    pub type_name: String,
}

/// One repeating `KATASTR` group under `KATASTRY`.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct ProceedingInfoCadastralAreaGroup {
    /// Every `KATUZE` occurrence in this group; the group may be empty.
    pub areas: Vec<ProceedingInfoCadastralArea>,
}

/// One `KATUZE` occurrence within a proceeding cadastral-area group.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct ProceedingInfoCadastralArea {
    /// Complete numeric code/name pair, or `None` for an empty optional sequence.
    pub details: Option<LegacyNumericCodeName>,
}

/// One `OBJEKT_PARCELA` in a proceeding report.
#[derive(Clone, Eq, PartialEq)]
pub struct ProceedingInfoParcel {
    /// Optional parcel in the required `PARCELA` wrapper.
    pub parcel: Option<Parcel>,
    /// Dynamic object type (`TYP`), retained verbatim.
    pub kind: String,
    /// Parcel-part value (`DIL_PARCELY`), retained as schema text.
    pub part: String,
    /// Numbering/source marker (`ZDROJ_ZE`), retained verbatim.
    pub source_from: String,
    /// Original cadastral-area value (`PUV_KATUZE`).
    pub original_cadastral_area: String,
}

/// One `OBJEKT_STAVBA` in a proceeding report.
#[derive(Clone, Eq, PartialEq)]
pub struct ProceedingInfoBuilding {
    /// Optional building in the required `STAVBA` wrapper.
    pub building: Option<Building>,
    /// Building-type abbreviation (`TBUD_ZKR`).
    pub building_type_abbreviation: String,
}

/// One `LISTINA` associated with a proceeding.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct ProceedingInfoDeed {
    /// Optional deed ID (`ID`), including a present-empty ID.
    pub id: Option<LegacyId>,
    /// Optional lexical preparation date (`DATUM_VYHOTOVENI`).
    pub prepared_at: Option<String>,
    /// Optional lexical legal-force date (`DATUM_PRAV_MOCI`).
    pub legal_force_at: Option<String>,
    /// Optional additional data (`DALSI_UDAJE`).
    pub additional_data: Option<String>,
    /// Optional deed-type name (`TYPLIS_NAZEV`).
    pub deed_type_name: Option<String>,
    /// Optional deed-type code (`TYPLIST_KOD`), covering `xs:unsignedShort`.
    pub deed_type_code: Option<u16>,
    /// Optional deed description (`LISTIN_POPIS`).
    pub description: Option<String>,
    /// Optional lexical history start (`DATUM_HIST_OD`).
    pub history_from: Option<String>,
    /// Optional lexical history end (`DATUM_HIST_DO`).
    pub history_to: Option<String>,
    /// Optional additional-data start date (`DALSI_UDAJE_DATUM_OD`).
    pub additional_data_from: Option<String>,
    /// Optional preparer (`ZHOTOVITEL`).
    pub preparer: Option<String>,
}

/// One `OBJEKT_JED` in a proceeding report.
#[derive(Clone, Eq, PartialEq)]
pub struct ProceedingInfoUnit {
    /// Optional unit in the required `C_JED` wrapper.
    pub unit: Option<Unit>,
    /// Unit-use description (`ZP_VYUZ_JED`).
    pub use_description: String,
    /// Optional building in the required `BUDOVA` wrapper.
    pub building: Option<Building>,
}

/// Optional `GEOM_PLAN` associated with a proceeding.
#[derive(Clone, Eq, PartialEq)]
pub struct ProceedingInfoGeometricPlan {
    /// Dynamic plan-type value (`NZ_TYPE`), retained verbatim.
    pub kind: String,
    /// Plan number (`CISLO_PLANU`).
    pub number: String,
    /// Every purpose name (`UCEL/nazev`); the list may be empty.
    pub purposes: Vec<String>,
}

/// One `UDALOST` in a proceeding report.
#[derive(Clone, Eq, PartialEq)]
pub struct ProceedingInfoEvent {
    /// Dynamic operation-type code (`TYPOPE_KOD`), retained verbatim.
    pub type_code: String,
    /// Operation-type name (`TYPOPE_NAZEV`).
    pub type_name: String,
    /// Event text (`TEXT`).
    pub text: String,
    /// Lexical creation timestamp (`VYTVORENO`).
    pub created_at: String,
}

/// One performed `OPERACE` in a proceeding report.
#[derive(Clone, Eq, PartialEq)]
pub struct ProceedingInfoOperation {
    /// Dynamic operation-type code (`TYPOPE_KOD`), retained verbatim.
    pub type_code: String,
    /// Operation-type name (`TYPOPE_NAZEV`).
    pub type_name: String,
    /// Untyped `TEXT` content, retained as exact source XML.
    pub text: ProceedingInfoOperationText,
    /// Lexical creation timestamp (`VYTVORENO`).
    pub created_at: String,
    /// Lexical completion timestamp (`DATUM_UKONCENI`).
    pub completed_at: String,
}

/// Exact source representation of an untyped proceeding-operation `TEXT` element.
///
/// The legacy XSD gives this element no type, so it is `xs:anyType` and may
/// contain arbitrary attributes, text, or nested elements. `raw_xml` includes
/// the outer `TEXT` element and loses none of those possibilities.
#[derive(Clone, Eq, PartialEq)]
pub struct ProceedingInfoOperationText {
    /// Exact UTF-8 source slice for the complete outer `TEXT` element.
    pub raw_xml: String,
}

/// One `ZAVADA_GP` geometric-plan defect.
#[derive(Clone, Eq, PartialEq)]
pub struct ProceedingInfoDefect {
    /// Defect name (`NAZEV`).
    pub name: String,
    /// Defect specification (`UPRESNENI_ZAVADY`).
    pub specification: String,
}

/// One `NAVRH_ZMEN` change proposal.
#[derive(Clone, Eq, PartialEq)]
pub struct ProceedingInfoChangeProposal {
    /// Proposal creator (`VYTVORIL`).
    pub created_by: String,
    /// Proceeding number (`CISLO_RIZENI`), including its empty form.
    pub proceeding: Proceeding,
    /// Dynamic proposal state (`STAV`), retained verbatim.
    pub state: String,
    /// Lexical proposal date (`DATUM`).
    pub date: String,
}

/// One `OBJEKTY_PRAVO_STAVBY` building-right object.
#[derive(Clone, Eq, PartialEq)]
pub struct ProceedingInfoBuildingRight {
    /// Required building-right identification (`PS_PARCELA/pravo_stavby`).
    pub building_right: BuildingRight,
    /// Required single purpose name (`PS_UCEL/nazev`).
    pub purpose: String,
    /// Lexical end of validity (`PS_PLATNOST_DO`).
    pub valid_until: String,
}
