//! Complete models for the legacy parcel- and building-information reports.
//!
//! These reports have no XML namespace. Optional wrapper elements are retained
//! as `Option` fields; in particular, an empty ownership or building wrapper is
//! not confused with an absent wrapper.

use crate::LegalRelation;

use super::legacy_base::{
    Address, Building, BuildingWrapper, CharacteristicWrapper, LegacyId, Parcel, Proceeding,
    ReportedShare, SubjectNameForm, YesNo,
};
use crate::model::reports::legacy_base::OwnershipIdentification;

/// Complete `InformaceOParcelach` report.
#[derive(Clone, Eq, PartialEq)]
pub struct ParcelInfoReport {
    /// The optional `KATASTR` entry inside the required `KATASTRY` wrapper.
    pub cadastre: Option<ParcelCadastre>,
    /// Lexical validity timestamp (`PLATNOST`).
    pub valid_at: String,
    /// Lexical creation timestamp (`VYHOTOVENO`).
    pub created_at: String,
    /// Server-provided cadastral-parcel count (`CS_POCET_PARCEL1`).
    pub cadastral_parcel_count: String,
    /// Server-provided simplified-register parcel count (`CS_POCET_PARCEL`).
    pub simplified_parcel_count: String,
    /// Lexical report creator (`VYHOTOVIL`).
    pub created_by: String,
    /// Free-access text (`BEZPL_PRISTUP`), retained verbatim.
    pub free_access: String,
    /// Optional additional output information (`INFO_VYSTUP`).
    pub output_info: Option<String>,
}

/// Complete `InformaceOStavbach` report.
#[derive(Clone, Eq, PartialEq)]
pub struct BuildingInfoReport {
    /// Territorial and building data from the required `REPORT` wrapper.
    pub report: BuildingReport,
    /// Optional server-provided building count (`CS_POCET_STAVEB`).
    pub building_count: Option<String>,
    /// Lexical validity timestamp (`PLATNOST`).
    pub valid_at: String,
    /// Lexical report creator (`VYHOTOVIL`).
    pub created_by: String,
    /// Free-access text (`BEZPL_PRISTUP`), retained verbatim.
    pub free_access: String,
    /// Lexical creation timestamp (`VYHOTOVENO`).
    pub created_at: String,
    /// Optional additional output information (`INFO_VYSTUP`).
    pub output_info: Option<String>,
}

/// District data shared by the legacy property-information reports.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct PropertyDistrict {
    /// Optional NUTS 4 identifier (`nuts4`).
    pub nuts4: Option<String>,
    /// Optional district name (`nazev`).
    pub name: Option<String>,
}

/// Municipality data with the schema's required unsigned code and name.
#[derive(Clone, Eq, PartialEq)]
pub struct PropertyMunicipality {
    /// Municipality code (`kod`).
    pub code: u32,
    /// Municipality name (`nazev`).
    pub name: String,
}

/// One cadastral territorial unit in `InformaceOParcelach`.
#[derive(Clone, Eq, PartialEq)]
pub struct ParcelCadastre {
    /// District (`OKRES`).
    pub district: PropertyDistrict,
    /// Municipality (`OBEC`).
    pub municipality: PropertyMunicipality,
    /// Cadastral area (`KATUZE`), whose whole identifying sequence is optional.
    pub cadastral_area: ParcelCadastralArea,
    /// Full cadastral-register parcels from `PARCELY_KN`.
    pub cadastral_parcels: Vec<CadastralParcel>,
    /// The optional single source in the required `ZDROJE` wrapper.
    pub source: Option<ParcelSource>,
}

/// Cadastral-area data whose `kod`/`nazev` sequence may be empty.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct ParcelCadastralArea {
    /// Complete area identity, or `None` for an empty `KATUZE` element.
    pub details: Option<ParcelCadastralAreaDetails>,
}

/// Non-empty cadastral-area identity in the parcel report.
#[derive(Clone, Eq, PartialEq)]
pub struct ParcelCadastralAreaDetails {
    /// Cadastral-area code (`kod`).
    pub code: u32,
    /// Cadastral-area name (`nazev`).
    pub name: String,
}

/// One `PARCELA_KN` full cadastral-register parcel.
#[derive(Clone, Eq, PartialEq)]
pub struct CadastralParcel {
    /// Optional report sequence number (`POCET_1`).
    pub sequence_number: Option<u32>,
    /// Parcel-part text (`DIL`).
    pub part: String,
    /// Optional parcel identity inside `PARCELNI_CISLO`.
    pub identification: Option<Parcel>,
    /// Parcel area in the server-reported unit (`VYMERA_PARCELY`).
    pub area: u32,
    /// Area-determination description (`URC_VYMERY`).
    pub area_determination: String,
    /// Land-kind description (`DRUH_POZEMKU`).
    pub land_kind: String,
    /// Optional land-use description (`ZP_VYUZITI`).
    pub use_description: Option<String>,
    /// Every protection description in document order (`OCHRANA/zpochr`).
    pub protections: Vec<String>,
    /// Map reference (`MAPA`).
    pub map: String,
    /// Proceeding identification (`RIZENI`), including its valid empty form.
    pub proceeding: Proceeding,
    /// Numeric cadastral-area code (`KATUZE_KOD`).
    pub cadastral_area_code: u32,
    /// Required primary-use value, including its schema-permitted empty form.
    pub primary_use: ParcelPrimaryUseCode,
    /// Building forming part of the parcel, or an empty `SOUCASTI_STAVBA`.
    pub component_building: Option<ParcelComponentBuilding>,
    /// Optional `STAVBA_NA_PARCELE` wrapper and its optional building.
    pub building_on_parcel: Option<BuildingWrapper>,
    /// Restriction value, or `None` when `PARCELA_OMEZENI` is absent.
    pub restriction: Option<YesNo>,
    /// Ownership block, including the valid completely empty form.
    pub ownership: PropertyOwnership,
    /// Every cadastral-register seal proceeding in document order.
    pub seals: Vec<CadastralParcelSeal>,
    /// Every BPEJ soil-quality entry in document order.
    pub bonities: Vec<CadastralBonity>,
}

/// The `ZPURVY_KOD` union of `xs:unsignedByte` and the empty string.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub enum ParcelPrimaryUseCode {
    /// A numeric primary-use code.
    Code(u8),
    /// The explicitly present empty string.
    Empty,
}

/// Building forming part of a cadastral-register parcel.
#[derive(Clone, Eq, PartialEq)]
pub struct ParcelComponentBuilding {
    /// Building identity (`stavba`).
    pub building: Building,
    /// Optional temporary-building marker (`docasna`).
    pub temporary: Option<String>,
}

/// One `PLOMBA_RIZENI_KN` entry.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct CadastralParcelSeal {
    /// Optional third parcel identifier (`PAR_ID3`).
    pub parcel_id3: Option<u64>,
    /// Seal proceeding, or `None` for the entry's empty sequence.
    pub proceeding: Option<Proceeding>,
}

/// One full-register BPEJ entry (`BDP_KN`).
#[derive(Clone, Eq, PartialEq)]
pub struct CadastralBonity {
    /// BPEJ code, retained as raw text (`BPEJ_KOD`).
    pub code: String,
    /// BPEJ area (`VYMERA`).
    pub area: u32,
}

/// The single simplified-register source represented by `ZDROJ`.
#[derive(Clone, Eq, PartialEq)]
pub struct ParcelSource {
    /// Numeric source code (`ZE_KOD`).
    pub source_code: u32,
    /// Origin description (`PUVOD`).
    pub origin: String,
    /// All simplified-register parcels from `PARCELY_ZE`.
    pub parcels: Vec<SimplifiedParcel>,
}

/// One `PARCELA_ZE` simplified-register parcel.
#[derive(Clone, Eq, PartialEq)]
pub struct SimplifiedParcel {
    /// Current cadastral-area code (`KATUZE_KOD`).
    pub cadastral_area_code: u32,
    /// Original cadastral-area code, retained as text (`KATUZE_KOD_PUV`).
    pub original_cadastral_area_code: String,
    /// Optional parcel identity in `PARCELNI_CISLO`.
    pub identification: Option<Parcel>,
    /// Parcel-part text (`DIL_PARCELY`).
    pub part: String,
    /// Optional original cadastral-area identity in `PUVODNI_KU`.
    pub original_cadastral_area: Option<ParcelCadastralAreaDetails>,
    /// Parcel type text (`TYP`).
    pub kind: String,
    /// Optional linked cadastral-register parcel in `PARCELA_KN`.
    pub cadastral_parcel: Option<Parcel>,
    /// Parcel area (`VYMERA_PARCELY`).
    pub area: u32,
    /// Proceeding identification (`RIZENI`).
    pub proceeding: Proceeding,
    /// All simplified-register seal entries in document order.
    pub seals: Vec<SimplifiedParcelSeal>,
    /// Restriction value, or `None` when `PARCELA_OMEZENI` is absent.
    pub restriction: Option<YesNo>,
    /// Ownership block, including the valid completely empty form.
    pub ownership: PropertyOwnership,
    /// All simplified-register BPEJ entries in document order.
    pub bonities: Vec<SimplifiedBonity>,
}

/// One `PLOMBA_RIZENI_ZE` entry.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct SimplifiedParcelSeal {
    /// Optional third parcel identifier (`PAR_ID3`).
    pub parcel_id3: Option<u64>,
    /// All proceedings in this seal entry; an empty entry has an empty vector.
    pub proceedings: Vec<Proceeding>,
}

/// One simplified-register BPEJ entry (`BDP_ZE`).
#[derive(Clone, Eq, PartialEq)]
pub struct SimplifiedBonity {
    /// Numeric BPEJ code (`BPEJ_KOD`).
    pub code: u32,
    /// BPEJ area (`VYMERA`).
    pub area: u32,
}

/// Presence-preserving ownership block shared by both property reports.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct PropertyOwnership {
    /// LV and subject details, or `None` for the schema's empty sequence.
    pub details: Option<PropertyOwnershipDetails>,
}

/// Non-empty ownership details shared by parcel and building reports.
#[derive(Clone, Eq, PartialEq)]
pub struct PropertyOwnershipDetails {
    /// Required `LV` wrapper; its fields can be empty where the XSD permits it.
    pub sheet: PropertyOwnershipSheet,
    /// All `SUBJEKT` entries in document order.
    pub subjects: Vec<PropertyOwnershipSubject>,
}

/// Ownership-sheet reference used by legacy property reports.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct PropertyOwnershipSheet {
    /// Ownership-sheet ID (`tel_id`), including a present-empty ID.
    pub id: Option<LegacyId>,
    /// Ownership-sheet number (`cislo_tel`), retained as text.
    pub number: Option<String>,
}

/// One subject or accessory-co-ownership record in a property report.
#[derive(Clone, Eq, PartialEq)]
pub struct PropertyOwnershipSubject {
    /// Primary ordered, possibly empty or mixed targets (`VLA_IDENT`).
    pub identification: OwnershipIdentification,
    /// Exact share with independently optional components (`PODIL`).
    pub share: ReportedShare,
    /// Name-only legal relation (`TYPRAV_NAZEV`); no code is inferred.
    pub legal_relation: LegalRelation,
    /// Subject-characteristic wrapper (`CHAROS`).
    pub characteristic: CharacteristicWrapper,
    /// Every repeated `VLA_IDENT2` block in document order.
    pub additional_identifications: Vec<OwnershipIdentification>,
}

/// Territorial report body in `InformaceOStavbach`.
#[derive(Clone, Eq, PartialEq)]
pub struct BuildingReport {
    /// District (`OKRES`).
    pub district: PropertyDistrict,
    /// Municipality (`OBEC`).
    pub municipality: PropertyMunicipality,
    /// Cadastral area (`KATUZE`) with required code and name.
    pub cadastral_area: BuildingCadastralArea,
    /// Every building in `STAVBY`, in document order.
    pub buildings: Vec<BuildingInfo>,
}

/// Required cadastral-area identity in the building report.
#[derive(Clone, Eq, PartialEq)]
pub struct BuildingCadastralArea {
    /// Cadastral-area code (`kod`).
    pub code: u32,
    /// Cadastral-area name (`nazev`).
    pub name: String,
}

/// One detailed `STAVBA` building entry.
#[derive(Clone, Eq, PartialEq)]
pub struct BuildingInfo {
    /// Optional report sequence count (`POCET_STAVEB`).
    pub construction_count: Option<u32>,
    /// Numeric building-type code (`TYP_KOD`).
    pub type_code: u8,
    /// House number text (`CISLO_DOMOVNI`).
    pub house_number: String,
    /// Building-use description (`ZP_VYUZ`).
    pub use_description: String,
    /// Optional building identity inside the required `CP_CE` wrapper.
    pub identification: Option<Building>,
    /// Building-type description (`TYP_STAVBY`).
    pub type_description: String,
    /// Every repeated address sequence group in `ADRESA_STAVBY`.
    pub addresses: Vec<BuildingAddressGroup>,
    /// Every protection description in `OCHRANA/zpochr`.
    pub protections: Vec<String>,
    /// Proceeding identification (`RIZENI`).
    pub proceeding: Proceeding,
    /// Building description (`POPIS_STAVBY`).
    pub description: String,
    /// Ordered repeated choices describing what the building forms part of.
    pub additional_details: Vec<BuildingPart>,
    /// Optional temporary-building marker (`DOCASNA_STAVBA`).
    pub temporary: Option<String>,
    /// Optional `LIST_G_VAZBA_JPV` wrapper and all of its entries.
    pub legal_relation_links: Option<Vec<BuildingLegalRelationLink>>,
    /// All `NA_PARCELE` parcel references in `BUD_NA_PARCELE`.
    pub parcels_beneath: Vec<Parcel>,
    /// Optional `VAZBA_JPV` wrapper, retaining an empty wrapper versus no text.
    pub legal_relation_text: Option<BuildingLegalRelationText>,
    /// Ordered repeated `PARCELY` sequence groups.
    pub parcel_groups: Vec<BuildingParcelGroup>,
    /// Building-level ownership block.
    pub ownership: PropertyOwnership,
    /// Optional `PLOMBY_STAV` wrapper and all seal proceedings.
    pub seals: Option<Vec<Proceeding>>,
}

/// One occurrence of the repeating `ADRESA_STAVBY` sequence.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct BuildingAddressGroup {
    /// Address-point ID (`adresni_misto`), including a present-empty ID.
    pub address_point_id: Option<LegacyId>,
    /// Street name (`ulice`), without trimming.
    pub street: Option<String>,
    /// House number (`cdomu`), without trimming.
    pub house_number: Option<String>,
    /// Orientation number (`co`), without trimming.
    pub orientation_number: Option<String>,
}

/// One entry in the repeating `DALSI_UDAJE` choice.
#[derive(Clone, Eq, PartialEq)]
pub enum BuildingPart {
    /// The building is part of a parcel (`stavba_soucasti_pozemku`).
    Parcel(Box<BuildingPartOfParcel>),
    /// The building is part of a building right (`stavba_soucasti_prava_stavby`).
    BuildingRight(Box<BuildingPartOfRight>),
}

/// Building-part-of-parcel data in the building-information report.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct BuildingPartOfParcel {
    /// Optional parcel identity (`parcela`).
    pub parcel: Option<Parcel>,
    /// Optional ownership-sheet ID (`tel_id`).
    pub ownership_sheet_id: Option<LegacyId>,
    /// Optional ownership-sheet number (`cislo_tel`).
    pub ownership_sheet_number: Option<String>,
}

/// Building-part-of-building-right data.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct BuildingPartOfRight {
    /// Optional ownership-sheet ID (`tel_id`).
    pub ownership_sheet_id: Option<LegacyId>,
    /// Optional ownership-sheet number (`cislo_tel`).
    pub ownership_sheet_number: Option<String>,
    /// Optional cadastral-area name (`ku_nazev`).
    pub cadastral_area_name: Option<String>,
}

/// One `G_VAZBA_JPV` legal-relation link.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct BuildingLegalRelationLink {
    /// Optional third building identifier (`BUD_ID3`), retained as text.
    pub building_id3: Option<String>,
    /// Optional legal-relation type text (`T_JPV`).
    pub relation_type: Option<String>,
}

/// Presence-preserving `VAZBA_JPV` wrapper.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct BuildingLegalRelationText {
    /// Optional text (`TEXT`).
    pub text: Option<String>,
}

/// One occurrence of the outer repeating `PARCELY` sequence.
#[derive(Clone, Eq, PartialEq)]
pub struct BuildingParcelGroup {
    /// Main parcel details (`PARCELA`).
    pub parcel: BuildingParcel,
    /// Optional outer `LV_PARCELY` block following `PARCELA`.
    pub ownership_sheet: Option<BuildingParcelSheet>,
    /// Optional outer cadastral-area name (`LV_KATUZE` as a string).
    pub cadastral_area_name: Option<String>,
}

/// Main `PARCELA` details associated with a building.
#[derive(Clone, Eq, PartialEq)]
pub struct BuildingParcel {
    /// Required parcel identity in `PARCELNI_CISLO`.
    pub identification: Parcel,
    /// All proceedings in the required `PLOMBY_PAR` wrapper.
    pub seals: Vec<Proceeding>,
    /// Optional inner `LV_PARCELY` block, including its valid empty form.
    pub inner_ownership_sheet: Option<BuildingParcelSheet>,
    /// Optional structured inner `LV_KATUZE` block.
    pub cadastral_area: Option<BuildingParcelCadastralArea>,
    /// Parcel ownership details.
    pub ownership: PropertyOwnership,
}

/// Presence-preserving `LV_PARCELY` block.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct BuildingParcelSheet {
    /// Sheet identity and owners, or `None` for an empty inner sequence.
    pub details: Option<BuildingParcelSheetDetails>,
}

/// Non-empty `LV_PARCELY` details.
#[derive(Clone, Eq, PartialEq)]
pub struct BuildingParcelSheetDetails {
    /// Required sheet identity (`C_TEL`).
    pub sheet: BuildingParcelSheetId,
    /// All required `OPSUB` entries in `OS_PARCELY`.
    pub owners: Vec<BuildingParcelOwner>,
}

/// Numeric sheet reference in `C_TEL`.
#[derive(Clone, Eq, PartialEq)]
pub struct BuildingParcelSheetId {
    /// Ownership-sheet ID (`tel_id`), which may be present and empty.
    pub id: LegacyId,
    /// Unsigned ownership-sheet number (`cislo_tel`).
    pub number: u32,
}

/// One `OPSUB` owner in `OS_PARCELY`.
#[derive(Clone, Eq, PartialEq)]
pub struct BuildingParcelOwner {
    /// Fixed-sequence subject-name data (`OPSUB_NAZEV`).
    pub name: SubjectNameForm,
    /// All addresses in `OPSUB_ADRESA`; the schema requires at least one.
    pub addresses: Vec<Address>,
    /// Subject-characteristic wrapper (`CHAROS`).
    pub characteristic: CharacteristicWrapper,
    /// Exact share (`PODIL`).
    pub share: ReportedShare,
    /// Name-only legal relation (`TYPRAV_NAZEV`).
    pub legal_relation: LegalRelation,
}

/// Presence-preserving structured inner `LV_KATUZE` block.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct BuildingParcelCadastralArea {
    /// Area code and name, or `None` for the block's empty sequence.
    pub details: Option<BuildingParcelCadastralAreaDetails>,
}

/// Non-empty structured `LV_KATUZE` identity.
#[derive(Clone, Eq, PartialEq)]
pub struct BuildingParcelCadastralAreaDetails {
    /// Cadastral-area code (`LV_KATUZE_KOD`).
    pub code: u32,
    /// Cadastral-area name (`LV_KATUZE_NAZEV`).
    pub name: String,
}
