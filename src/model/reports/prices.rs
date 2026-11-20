use rust_decimal::Decimal;

use crate::model::{LegacyId, XmlDateTime};

/// Complete `CenoveUdajeDleNemovitosti` 1.05 report.
#[derive(Clone, Eq, PartialEq)]
pub struct PricesByPropertiesReport {
    /// Territorial groups from `KATASTRY/KATASTR`, in document order.
    pub cadastral_units: Vec<PriceCadastralUnit>,
    /// Uninterpreted `BEZPL_PRISTUP` text.
    pub free_access: String,
    /// Uninterpreted `PLATNOST` text; the legacy XSD declares it as `xs:string`.
    pub validity: String,
    /// Uninterpreted `VYHOTOVIL` text.
    pub prepared_by: String,
    /// Uninterpreted `INFO_VYSTUP` text.
    pub output_info: String,
    /// Uninterpreted `VYHOTOVENO` text; the legacy XSD declares it as `xs:string`.
    pub prepared_at: String,
}

/// One `KATASTR` territorial group in a prices-by-properties report.
#[derive(Clone, Eq, PartialEq)]
pub struct PriceCadastralUnit {
    /// District information from `OKRES`.
    pub district: PriceDistrict,
    /// Municipality information from `OBEC`.
    pub municipality: PriceMunicipality,
    /// Cadastral-area information, absent when `KATUZE` is empty.
    pub cadastral_area: Option<PriceCadastralArea>,
    /// Properties from `NEMOVITOSTI/NEMOVITOST`.
    pub properties: Vec<PriceProperty>,
}

/// District details attached to a legacy price report.
#[derive(Clone, Eq, PartialEq)]
pub struct PriceDistrict {
    /// Optional `nuts4` value, retained as text.
    pub nuts4: Option<String>,
    /// Optional district name.
    pub name: Option<String>,
}

/// Municipality details attached to a legacy price report.
#[derive(Clone, Eq, PartialEq)]
pub struct PriceMunicipality {
    /// Numeric `OBEC/kod` (`xs:unsignedInt`).
    pub code: u32,
    /// Municipality name.
    pub name: String,
}

/// Cadastral-area details attached to a legacy price report.
#[derive(Clone, Eq, PartialEq)]
pub struct PriceCadastralArea {
    /// Numeric `KATUZE/kod` (`xs:unsignedInt`).
    pub code: u32,
    /// Cadastral-area name.
    pub name: String,
}

/// One primary `NEMOVITOST` entry.
#[derive(Clone, Eq, PartialEq)]
pub struct PriceProperty {
    /// Optional human-readable property type (`TYP_NEM_TEXT`).
    pub property_type: Option<String>,
    /// Ordered property choices from `NEM_IDENT`.
    pub identification: PricePropertyIdent,
    /// One or more component property identities from repeated `SOUCASTI_JE` elements.
    pub components: Vec<PricePropertyIdent>,
    /// Parent properties from `NADRAZENE_NEMOVITOSTI`.
    pub parents: Vec<PriceParentProperty>,
}

/// Legacy `NADRAZENA_NEMOVITOST`, whose entire inner sequence is optional.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct PriceParentProperty {
    /// Parent details, or `None` for a wholly empty `NADRAZENA_NEMOVITOST`.
    pub details: Option<PriceParentPropertyDetails>,
}

/// Fields in a non-empty legacy `NADRAZENA_NEMOVITOST` sequence.
#[derive(Clone, Eq, PartialEq)]
pub struct PriceParentPropertyDetails {
    /// Optional human-readable property type.
    pub property_type: Option<String>,
    /// Ordered property choices from `NEM_IDENT`.
    pub identification: PricePropertyIdent,
    /// One or more component property identities from repeated `SOUCASTI_JE` elements.
    pub components: Vec<PricePropertyIdent>,
    /// `None` when the optional `LISTINY` wrapper is absent; otherwise all deeds,
    /// including an explicitly empty wrapper.
    pub deeds: Option<Vec<PriceDeed>>,
}

/// Price-specific representation of legacy `NemIdentType`.
///
/// The XSD choice is repeatable, so one identity can contain several different
/// property records. Their original order is retained.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct PricePropertyIdent {
    /// Ordered `parcela`, `stavba`, `jednotka`, and `pravo_stavby` choices.
    pub items: Vec<PricePropertyIdentItem>,
}

/// One property choice inside legacy `NemIdentType`.
#[derive(Clone, Eq, PartialEq)]
pub enum PricePropertyIdentItem {
    /// A `parcela` record.
    Parcel(PriceParcel),
    /// A `stavba` record.
    Building(PriceBuilding),
    /// A `jednotka` record.
    Unit(PriceUnit),
    /// A `pravo_stavby` record.
    BuildingRight(PriceBuildingRight),
}

/// Complete price-specific legacy `ParcelaType`.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct PriceParcel {
    /// Optional legacy internal ID (`id`), including the XSD-permitted empty value.
    pub id: Option<LegacyId>,
    /// Optional abbreviation (`zkratka`).
    pub abbreviation: Option<String>,
    /// Optional land-kind number (`druh_cis`), retained as text.
    pub land_kind_number: Option<String>,
    /// Optional parcel number (`par_cis`), retained verbatim including whitespace.
    pub parcel_number: Option<String>,
    /// Optional subdivision number (`podd_cis`), retained verbatim.
    pub subdivision_number: Option<String>,
    /// Optional parcel part (`dil_parcely`, `xs:byte`).
    pub parcel_part: Option<i8>,
    /// Optional legacy body ID (`tel_id`).
    pub body_id: Option<LegacyId>,
    /// Optional body number (`cislo_tel`), retained as text.
    pub body_number: Option<String>,
    /// Optional cadastral-area name (`ku_nazev`).
    pub cadastral_area_name: Option<String>,
    /// Optional original name (`nazev_puv`).
    pub original_name: Option<String>,
}

/// Complete price-specific legacy `StavbaType`.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct PriceBuilding {
    /// Optional legacy internal ID.
    pub id: Option<LegacyId>,
    /// Optional municipality-part text (`caobce`).
    pub municipality_part: Option<String>,
    /// Optional building-type abbreviation (`typbud_zkr`).
    pub building_type_abbreviation: Option<String>,
    /// Optional house numbers (`cisla_domovni`), retained as text.
    pub house_numbers: Option<String>,
    /// Optional use abbreviation (`vyuziti_zkr`).
    pub use_abbreviation: Option<String>,
    /// Optional legacy body ID.
    pub body_id: Option<LegacyId>,
    /// Optional `bez_lv` marker, retained without interpreting its text.
    pub without_ownership_sheet: Option<String>,
    /// Optional body number, retained as text.
    pub body_number: Option<String>,
    /// Optional cadastral-area name.
    pub cadastral_area_name: Option<String>,
    /// Optional temporary-building marker (`docasna`).
    pub temporary: Option<String>,
    /// Associated parcels from the optional `par_ident` wrapper. `None` and an
    /// explicitly empty wrapper remain distinguishable.
    pub parcels: Option<Vec<PriceParcel>>,
}

/// Complete price-specific legacy `JednotkaType`.
#[derive(Clone, Eq, PartialEq)]
pub struct PriceUnit {
    /// Required legacy unit ID, which may be empty under legacy `ID_TYPE`.
    pub id: LegacyId,
    /// Optional municipality-part text (`caobce`).
    pub municipality_part: Option<String>,
    /// Optional house identification (`cp_ident`).
    pub house_identification: Option<String>,
    /// Optional unit identification (`jed_ident`).
    pub unit_identification: Option<String>,
    /// Optional legacy body ID.
    pub body_id: Option<LegacyId>,
    /// Optional body number, retained as text.
    pub body_number: Option<String>,
    /// Optional cadastral-area name.
    pub cadastral_area_name: Option<String>,
}

/// Complete price-specific legacy `PravoStavbyType`.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct PriceBuildingRight {
    /// Optional legacy building-right ID.
    pub id: Option<LegacyId>,
    /// Optional legacy body ID.
    pub body_id: Option<LegacyId>,
    /// All directly repeated associated `parcela` records.
    pub parcels: Vec<PriceParcel>,
    /// Optional body number, retained as text.
    pub body_number: Option<String>,
    /// Optional cadastral-area name.
    pub cadastral_area_name: Option<String>,
}

/// Legacy `IDENT_RIZENI`, whose entire inner sequence is optional.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct PriceProceedingReference {
    /// Proceeding number details, or `None` for an empty `IDENT_RIZENI`.
    pub identification: Option<PriceProceedingNumber>,
}

/// Complete legacy `RizeniType` value.
#[derive(Clone, Eq, PartialEq)]
pub struct PriceProceedingNumber {
    /// Proceeding-type code (`typriz_kod`), retained as text.
    pub proceeding_type_code: String,
    /// Sequence number (`poradove_cislo`, `xs:unsignedInt`).
    pub sequence_number: u32,
    /// Year (`rok`, `xs:unsignedShort`).
    pub year: u16,
    /// Cadastral workplace number (`prares_kod`, `xs:unsignedShort`).
    pub workplace_code: u16,
}

/// A deed and price shared by both legacy price report families.
#[derive(Clone, Eq, PartialEq)]
pub struct PriceDeed {
    /// `CENA_Z_LISTINY` exactly as text. It is not converted to `Decimal`, because
    /// the legacy XSD deliberately declares this field as `xs:string`.
    pub price: String,
    /// Proceeding identification associated with this deed.
    pub proceeding: PriceProceedingReference,
    /// Currency code (`MENA_KOD`), retained verbatim.
    pub currency: String,
    /// Complete `LISTINA_INFO` value.
    pub information: PriceDeedInfo,
    /// Historical relation groups from `SOUVISEJICI_TYPY`.
    pub related_types: Vec<PriceRelatedType>,
}

/// Legacy `NazevListType`, retaining the distinction between an empty value and
/// its complete optional sequence.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct PriceDeedInfo {
    /// All deed details, or `None` when `LISTINA_INFO` is empty.
    pub details: Option<PriceDeedDetails>,
}

/// Fields in the optional sequence of legacy `NazevListType`.
#[derive(Clone, Eq, PartialEq)]
pub struct PriceDeedDetails {
    /// Deed type (`tlist`).
    pub deed_type: String,
    /// Additional data (`dalsi_udaje`).
    pub additional_data: String,
    /// Creation sequence number (`por_cis_zhot`), retained as text.
    pub creation_sequence_number: String,
    /// Description (`popis`).
    pub description: String,
    /// Issue text (`vystav_txt`).
    pub issued_text: String,
    /// Legal-force text (`prav_moc_txt`).
    pub legal_force_text: String,
    /// Submission text (`podani_txt`).
    pub submission_text: String,
    /// Enforceability text (`vykonatelnost_txt`).
    pub enforceability_text: String,
    /// Additional submission text (`podani_z_txt`).
    pub submission_z_text: String,
    /// Validation text (`zplatneni_txt`).
    pub validation_text: String,
    /// Optional proceeding-type name (`prares_nazev`).
    pub proceeding_type_name: Option<String>,
}

/// Legacy `SOUVISEJICI_TYP`, whose entire inner sequence is optional.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct PriceRelatedType {
    /// Relation details, or `None` for a wholly empty `SOUVISEJICI_TYP`.
    pub details: Option<PriceRelatedTypeDetails>,
}

/// Fields in a non-empty legacy `SOUVISEJICI_TYP` sequence.
#[derive(Clone, Eq, PartialEq)]
pub struct PriceRelatedTypeDetails {
    /// Relation type text (`TYP_TEXT`), never mapped to a guessed legal enum.
    pub type_text: String,
    /// Related properties in document order; the required wrapper may be empty.
    pub properties: Vec<PriceRelatedProperty>,
}

/// Legacy `SOUVISEJICI_NEMOVISTOST` (the spelling is from the official XSD), whose
/// entire inner sequence is optional.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct PriceRelatedProperty {
    /// Related-property details, or `None` for a wholly empty value.
    pub details: Option<PriceRelatedPropertyDetails>,
}

/// Fields in a non-empty legacy `SOUVISEJICI_NEMOVISTOST` sequence.
#[derive(Clone, Eq, PartialEq)]
pub struct PriceRelatedPropertyDetails {
    /// Ordered property choices from `NEM_IDENT`.
    pub identification: PricePropertyIdent,
    /// One or more component property identities from repeated `SOUCASTI_JE` elements.
    pub components: Vec<PricePropertyIdent>,
}

/// Complete `CenoveUdajeDleRizeni` 1.03 report.
#[derive(Clone, Eq, PartialEq)]
pub struct PricesByProceedingReport {
    /// Cadastral workplaces from `PRACOVISTE/KP`.
    pub workplaces: Vec<PriceWorkplace>,
    /// Uninterpreted `BEZPL_PRISTUP` text.
    pub free_access: String,
    /// Uninterpreted `VYHOTOVIL` text.
    pub prepared_by: String,
    /// Uninterpreted `INFO_VYSTUP` text.
    pub output_info: String,
    /// Uninterpreted `VYHOTOVENO` text; the legacy XSD declares it as `xs:string`.
    pub prepared_at: String,
}

/// One legacy cadastral workplace (`KP`) and its proceedings.
#[derive(Clone, Eq, PartialEq)]
pub struct PriceWorkplace {
    /// Raw workplace code (`KOD`), which is an `xs:string` in this report.
    pub code: String,
    /// Workplace name (`NAZEV`).
    pub name: String,
    /// Proceedings from `RIZENI/RIZ`.
    pub proceedings: Vec<PriceProceeding>,
}

/// One proceeding (`RIZ`) in a prices-by-proceeding report.
#[derive(Clone, Eq, PartialEq)]
pub struct PriceProceeding {
    /// Proceeding-level identification.
    pub proceeding: PriceProceedingReference,
    /// Deeds from the required `LISTINY` wrapper; an empty wrapper yields an empty vector.
    pub deeds: Vec<PriceDeed>,
}

/// Monthly `CenoveUdaje` 1.0 document.
#[derive(Clone, Eq, PartialEq)]
pub struct MonthlyPriceData {
    /// Values of the `bez_mj` XSD list, in lexical order.
    pub without_measurement: Vec<MonthlyYesNo>,
    /// Values of the `mesic` XSD list. Every value is in `1..=12`; an empty list
    /// remains empty rather than being treated as a missing attribute.
    pub months: Vec<u8>,
    /// Exact lexical `rok` value (`xs:gYear`), including any original timezone.
    pub year: String,
    /// Required `katuze_kod_cu` value (`xs:long`).
    pub cadastral_area_code: i64,
    /// Proceedings in document order.
    pub proceedings: Vec<MonthlyProceeding>,
}

/// The otherwise-unused `CenoveUdajePropertyType` from the monthly schema.
#[derive(Clone, Eq, PartialEq)]
pub struct MonthlyPriceDataProperty {
    /// Inline `CenoveUdaje` value.
    pub value: MonthlyPriceData,
    /// All association attributes, retained with expanded names.
    pub attributes: MonthlyXmlAttributes,
}

/// One item of the monthly `bez_mj` list.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub enum MonthlyYesNo {
    /// Czech `a` (yes).
    Yes,
    /// Czech `n` (no).
    No,
}

impl MonthlyYesNo {
    /// Returns the exact schema token.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Yes => "a",
            Self::No => "n",
        }
    }
}

/// One item of the monthly property `typ` list.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub enum MonthlyPropertyKind {
    /// Parcel (`PAR`).
    Parcel,
    /// Unit (`JED`).
    Unit,
    /// Building (`BUD`).
    Building,
    /// Building right (`PS`).
    BuildingRight,
}

impl MonthlyPropertyKind {
    /// Returns the exact schema token.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Parcel => "PAR",
            Self::Unit => "JED",
            Self::Building => "BUD",
            Self::BuildingRight => "PS",
        }
    }
}

/// One monthly `Rizeni` proceeding.
#[derive(Clone, Eq, PartialEq)]
pub struct MonthlyProceeding {
    /// Required `id` (`xs:long`).
    pub id: i64,
    /// Proceeding number (`cisloRizeni`), retained as text.
    pub number: String,
    /// Submission date-time (`datumPodani`), retaining offset semantics.
    pub submitted_at: XmlDateTime,
    /// Validation date-time (`datumZplatneni`), retaining offset semantics.
    pub validated_at: XmlDateTime,
    /// Workplace number (`praresKod`, `xs:int`).
    pub workplace_code: i32,
    /// Required deed collection.
    pub deeds: MonthlyDeedCollection,
    /// Required property collection.
    pub properties: MonthlyPropertyCollection,
}

/// Monthly `ListinyPropertyType` value.
#[derive(Clone, Eq, PartialEq)]
pub struct MonthlyDeedCollection {
    /// One or more inline deeds in valid schema documents.
    pub deeds: Vec<MonthlyDeed>,
    /// Every GML association/ownership attribute, plus any other supplied
    /// attributes, retained with expanded names.
    pub attributes: MonthlyXmlAttributes,
}

/// Monthly `NemovitostiPropertyType` value.
#[derive(Clone, Eq, PartialEq)]
pub struct MonthlyPropertyCollection {
    /// One or more inline properties in valid schema documents.
    pub properties: Vec<MonthlyProperty>,
    /// Every GML association/ownership attribute, plus any other supplied
    /// attributes, retained with expanded names.
    pub attributes: MonthlyXmlAttributes,
}

/// One monthly `Listina` deed with an exact decimal price.
#[derive(Clone, Eq, PartialEq)]
pub struct MonthlyDeed {
    /// Required `id` (`xs:long`).
    pub id: i64,
    /// Exact `cena` (`xs:decimal`).
    pub price: Decimal,
    /// Currency (`mena`), retained verbatim.
    pub currency: String,
    /// Deed type (`listinaTyp`), retained verbatim.
    pub deed_type: String,
    /// Every GML association/ownership attribute, plus any other supplied
    /// attributes, retained with expanded names.
    pub attributes: MonthlyXmlAttributes,
}

/// One monthly `Nemovitost`, including inherited GML feature content.
#[derive(Clone, Eq, PartialEq)]
pub struct MonthlyProperty {
    /// Required unqualified monthly `id` (`xs:long`).
    pub id: i64,
    /// Required `gml:id` from `gml:AbstractGMLType`.
    pub gml_id: String,
    /// Values of the required `typ` XSD list, in lexical order.
    pub kinds: Vec<MonthlyPropertyKind>,
    /// Inherited GML object/feature elements (`metaDataProperty`, `description`,
    /// `descriptionReference`, `identifier`, `name`, `boundedBy`, and `location`)
    /// in document order. Raw XML preserves arbitrary imported GML content.
    pub gml_elements: Vec<MonthlyRawXmlElement>,
    /// Cadastral-area code (`katuzeKod`, `xs:int`).
    pub cadastral_area_code: i32,
    /// Optional monthly `text`; this is distinct from `gml:description`.
    pub text: Option<String>,
    /// Optional monthly `popis` description.
    pub description: Option<String>,
    /// Optional monthly `bod` point property.
    pub point: Option<MonthlyPointProperty>,
    /// Required `teleso` value.
    pub body: MonthlyPropertyBody,
}

/// Monthly `TelesoPropertyType` value.
#[derive(Clone, Eq, PartialEq)]
pub struct MonthlyPropertyBody {
    /// Optional unqualified `id` (`xs:long`).
    pub id: Option<i64>,
    /// Body number (`cislo`, `xs:int`).
    pub number: i32,
}

/// A monthly `bod` using GML 3.2 `PointPropertyType`.
#[derive(Clone, Eq, PartialEq)]
pub struct MonthlyPointProperty {
    /// Inline `gml:Point`, or `None` for a by-reference property.
    pub point: Option<MonthlyPoint>,
    /// Every GML association/ownership attribute, plus any other supplied
    /// attributes, retained with expanded names.
    pub attributes: MonthlyXmlAttributes,
}

/// Inline GML 3.2 point.
#[derive(Clone, Eq, PartialEq)]
pub struct MonthlyPoint {
    /// Required `gml:id`.
    pub gml_id: String,
    /// All point attributes, including `gml:id` and the SRS attribute group.
    pub attributes: MonthlyXmlAttributes,
    /// Inherited GML object properties in document order, preserved as raw XML.
    pub gml_elements: Vec<MonthlyRawXmlElement>,
    /// The point's `gml:pos` or deprecated `gml:coordinates` choice.
    pub position: MonthlyPointPosition,
}

/// Coordinates accepted by GML 3.2 `PointType`.
#[derive(Clone, Eq, PartialEq)]
pub enum MonthlyPointPosition {
    /// Modern `gml:pos` direct position.
    Position(MonthlyDirectPosition),
    /// Deprecated `gml:coordinates` encoding.
    Coordinates(MonthlyCoordinates),
}

/// GML `DirectPositionType`, preserving its `doubleList` lexical values.
#[derive(Clone, Eq, PartialEq)]
pub struct MonthlyDirectPosition {
    /// Exact decoded `gml:pos` content, including its original whitespace.
    pub raw: String,
    /// Raw `xs:double` tokens in lexical order. Exponents, `INF`, `-INF`, and `NaN`
    /// remain available without conversion through a binary float.
    pub values: Vec<String>,
    /// Ordinates as exact decimals, or `None` unless every token uses exact XSD
    /// decimal syntax and is representable by `Decimal` without rounding.
    pub coordinates: Option<Vec<Decimal>>,
    /// All SRS attributes with expanded names.
    pub attributes: MonthlyXmlAttributes,
}

/// Deprecated GML `CoordinatesType` without discarding its lexical encoding.
#[derive(Clone, Eq, PartialEq)]
pub struct MonthlyCoordinates {
    /// Exact decoded element text, including non-numeric text allowed by GML.
    pub raw: String,
    /// Numeric tuples parsed according to `decimal`, `cs`, and `ts`. This is
    /// `None` unless every normalized component is an exact, representable XSD
    /// decimal; `raw` retains every other value without loss.
    pub numeric_tuples: Option<Vec<Vec<Decimal>>>,
    /// Decimal separator from `decimal`, defaulting to `.`.
    pub decimal_separator: String,
    /// Component separator from `cs`, defaulting to `,`.
    pub coordinate_separator: String,
    /// Tuple separator from `ts`, defaulting to one space.
    pub tuple_separator: String,
    /// All supplied attributes with expanded names.
    pub attributes: MonthlyXmlAttributes,
}

/// An expanded XML name used by monthly GML preservation structures.
#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MonthlyExpandedName {
    /// Namespace URI, or `None` for an unqualified name.
    pub namespace: Option<String>,
    /// Local XML name.
    pub local_name: String,
}

/// One XML attribute with its expanded name and decoded value.
#[derive(Clone, Eq, PartialEq)]
pub struct MonthlyXmlAttribute {
    /// Expanded attribute name, independent of its source prefix.
    pub name: MonthlyExpandedName,
    /// Attribute value as decoded by the XML parser.
    pub value: String,
}

/// Ordered, raw-value-preserving monthly XML attributes.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct MonthlyXmlAttributes {
    /// Attributes in source order. Namespace declarations are represented by the
    /// enclosing raw XML rather than exposed as ordinary XML attributes.
    pub values: Vec<MonthlyXmlAttribute>,
}

impl MonthlyXmlAttributes {
    /// Finds an attribute by expanded name.
    pub fn get(&self, namespace: Option<&str>, local_name: &str) -> Option<&str> {
        self.values
            .iter()
            .find(|attribute| {
                attribute.name.namespace.as_deref() == namespace
                    && attribute.name.local_name == local_name
            })
            .map(|attribute| attribute.value.as_str())
    }
}

/// An imported GML element retained without flattening its extensible content.
#[derive(Clone, Eq, PartialEq)]
pub struct MonthlyRawXmlElement {
    /// Expanded element name.
    pub name: MonthlyExpandedName,
    /// Attributes in source order with expanded names.
    pub attributes: MonthlyXmlAttributes,
    /// Exact UTF-8 XML slice for the complete element.
    pub xml: String,
}

/// Bounded expansion policy for [`MonthlyPriceArchive`] parsing.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct MonthlyPriceArchiveLimits {
    /// Maximum number of ZIP entries, including entries that later fail XML decoding.
    pub max_entries: usize,
    /// Maximum uncompressed bytes in any one entry.
    pub max_entry_uncompressed_bytes: u64,
    /// Maximum sum of uncompressed bytes across all entries.
    pub max_total_uncompressed_bytes: u64,
    /// Maximum permitted integer ratio `uncompressed_size / compressed_size`.
    /// A non-empty entry with zero compressed bytes is always rejected.
    pub max_compression_ratio: u64,
}

impl Default for MonthlyPriceArchiveLimits {
    fn default() -> Self {
        Self {
            max_entries: 4_096,
            max_entry_uncompressed_bytes: 64 * 1024 * 1024,
            max_total_uncompressed_bytes: 256 * 1024 * 1024,
            max_compression_ratio: 100,
        }
    }
}

/// A monthly price ZIP with untouched source bytes and all decoded entries.
#[derive(Clone, Eq, PartialEq)]
pub struct MonthlyPriceArchive {
    /// Exact original ZIP archive bytes.
    pub original_bytes: Vec<u8>,
    /// Entries in central-directory order. Multiple entries and months are retained.
    pub entries: Vec<MonthlyPriceEntry>,
}

/// One safely named monthly ZIP entry.
#[derive(Clone, Eq, PartialEq)]
pub struct MonthlyPriceEntry {
    /// Safe ZIP metadata; no filesystem path is created from it.
    pub metadata: MonthlyPriceEntryMetadata,
    /// Exact uncompressed XML bytes before decoding or normalization.
    pub original_bytes: Vec<u8>,
    /// Parsed monthly document.
    pub value: MonthlyPriceData,
}

/// Non-filesystem metadata for one validated monthly ZIP entry.
#[derive(Clone, Eq, PartialEq)]
pub struct MonthlyPriceEntryMetadata {
    /// Zero-based central-directory entry index.
    pub index: usize,
    /// Decoded entry name after rejection of absolute, parent-relative, Windows
    /// separator/drive, NUL, directory, and symbolic-link paths.
    pub name: String,
    /// Original entry-name bytes from the ZIP central directory.
    pub name_raw: Vec<u8>,
    /// Compressed size reported by the archive.
    pub compressed_size: u64,
    /// Verified uncompressed size.
    pub uncompressed_size: u64,
    /// CRC-32 from the archive metadata.
    pub crc32: u32,
}
