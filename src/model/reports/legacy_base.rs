use rust_decimal::Decimal;

use crate::{Error, Result};

pub use super::super::id::LegacyId;

/// Legacy unit-type description (`TYP_JEDNOTKY_POPIS_TYPE`).
pub type UnitTypeDescription = String;

/// Legacy unit-use description (`ZPUSOB_VYUZITI_JEDNOTKY_POPIS_TYPE`).
pub type UnitUseDescription = String;

/// The kind of a house number in a legacy address (`AdresaCpCeType`).
///
/// A present but empty `cp_ce` element is retained as `Other(String::new())`;
/// absence is represented by the containing `Option`.
#[derive(Clone, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum HouseNumberKind {
    /// Descriptive number (`1`).
    Descriptive,
    /// Evidentiary number (`2`).
    Evidentiary,
    /// An empty, future, or otherwise unknown value, retained verbatim.
    Other(String),
}

impl HouseNumberKind {
    /// Converts an untouched XML value to a known or raw-preserving variant.
    pub fn from_raw(raw: String) -> Self {
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

/// A legacy address (`AdresaType`).
///
/// The XSD repeats the whole sequence rather than individual fields. Each
/// entry in `groups` therefore corresponds to one sequence occurrence, and its
/// order is significant.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct Address {
    /// Repeated, ordered address sequence groups.
    pub groups: Vec<AddressGroup>,
}

/// One occurrence of the repeating legacy address sequence.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct AddressGroup {
    /// Address-point identifier (`adresni_misto`), including a present-empty ID.
    pub address_point_id: Option<LegacyId>,
    /// Street name (`nazev_ulice`), without trimming.
    pub street_name: Option<String>,
    /// Descriptive/evidentiary house-number kind (`cp_ce`).
    pub house_number_kind: Option<HouseNumberKind>,
    /// House number (`cislo_domovni`), without trimming.
    pub house_number: Option<String>,
    /// Orientation number (`cislo_orientacni`), without trimming.
    pub orientation_number: Option<String>,
    /// Municipality part (`cast_obce`).
    pub municipality_part: Option<String>,
    /// Municipality (`obec`).
    pub municipality: Option<String>,
    /// City district (`mestska_cast`).
    pub city_district: Option<String>,
    /// City borough (`mestsky_obvod`).
    pub city_borough: Option<String>,
    /// Postal code (`psc`), retained as text.
    pub postal_code: Option<String>,
    /// Delivery post office (`dod_posta`).
    pub delivery_post_office: Option<String>,
    /// Country name (`stat`); no country is inferred when this is absent.
    pub country_name: Option<String>,
}

/// Legacy deed-name information (`NazevListType`).
///
/// `details` is `None` for an empty element. The XSD permits either that empty
/// form or the complete sequence represented by [`DeedNameDetails`].
#[derive(Clone, Default, Eq, PartialEq)]
pub struct DeedName {
    /// Complete deed-name details, or `None` for the XSD's empty sequence.
    pub details: Option<DeedNameDetails>,
}

/// The non-empty form of legacy deed-name information.
#[derive(Clone, Eq, PartialEq)]
pub struct DeedNameDetails {
    /// Deed type (`tlist`).
    pub deed_type: String,
    /// Additional data (`dalsi_udaje`).
    pub additional_data: String,
    /// Preparation serial number (`por_cis_zhot`).
    pub preparation_serial_number: String,
    /// Description (`popis`).
    pub description: String,
    /// Issuance text (`vystav_txt`).
    pub issuance_text: String,
    /// Legal-force text (`prav_moc_txt`).
    pub legal_force_text: String,
    /// Submission text (`podani_txt`).
    pub submission_text: String,
    /// Enforceability text (`vykonatelnost_txt`).
    pub enforceability_text: String,
    /// Additional submission text (`podani_z_txt`).
    pub submission_z_text: String,
    /// Effectiveness text (`zplatneni_txt`).
    pub effectiveness_text: String,
    /// Handling-workplace name (`prares_nazev`), when supplied.
    pub handling_workplace_name: Option<String>,
}

/// Presence-preserving `ParcelaUpType` wrapper.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct ParcelWrapper {
    /// Nested parcel, or `None` when the wrapper itself is empty.
    pub parcel: Option<Parcel>,
}

/// Legacy parcel identification (`ParcelaType`).
#[derive(Clone, Default, Eq, PartialEq)]
pub struct Parcel {
    /// Internal parcel ID (`id`), which may be present and empty.
    pub id: Option<LegacyId>,
    /// Parcel-kind abbreviation (`zkratka`).
    pub kind_abbreviation: Option<String>,
    /// Parcel-numbering kind (`druh_cis`), retained as a dynamic raw string.
    pub numbering_kind: Option<String>,
    /// Parcel number (`par_cis`), retained as text.
    pub number: Option<String>,
    /// Parcel subdivision (`podd_cis`), retained as text.
    pub subdivision: Option<String>,
    /// Parcel part (`dil_parcely`), with the exact signed `xs:byte` range.
    pub part: Option<i8>,
    /// Ownership-sheet ID (`tel_id`), which may be present and empty.
    pub ownership_sheet_id: Option<LegacyId>,
    /// Ownership-sheet number (`cislo_tel`).
    pub ownership_sheet_number: Option<String>,
    /// Cadastral-area name (`ku_nazev`).
    pub cadastral_area_name: Option<String>,
    /// Original cadastral-area name (`nazev_puv`).
    pub original_cadastral_area_name: Option<String>,
}

/// Presence-preserving `StavbaUpType` wrapper.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct BuildingWrapper {
    /// Nested building, or `None` when the wrapper itself is empty.
    pub building: Option<Building>,
}

/// Legacy building identification (`StavbaType`).
#[derive(Clone, Default, Eq, PartialEq)]
pub struct Building {
    /// Internal building ID (`id`), which may be present and empty.
    pub id: Option<LegacyId>,
    /// Municipality-part value (`caobce`), retained verbatim.
    pub municipality_part: Option<String>,
    /// Building-type abbreviation (`typbud_zkr`).
    pub building_type_abbreviation: Option<String>,
    /// House numbers (`cisla_domovni`).
    pub house_numbers: Option<String>,
    /// Building-use abbreviation (`vyuziti_zkr`).
    pub use_abbreviation: Option<String>,
    /// Ownership-sheet ID (`tel_id`), which may be present and empty.
    pub ownership_sheet_id: Option<LegacyId>,
    /// Without-ownership-sheet marker (`bez_lv`), retained verbatim.
    pub without_ownership_sheet: Option<String>,
    /// Ownership-sheet number (`cislo_tel`).
    pub ownership_sheet_number: Option<String>,
    /// Cadastral-area name (`ku_nazev`).
    pub cadastral_area_name: Option<String>,
    /// Temporary-building marker (`docasna`), retained verbatim.
    pub temporary: Option<String>,
    /// The `par_ident` wrapper and its repeated parcels.
    ///
    /// `None` means the wrapper is absent; `Some(Vec::new())` means it is
    /// present but empty.
    pub identifying_parcels: Option<Vec<Parcel>>,
}

/// Presence-preserving `JednotkaUpType` wrapper.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct UnitWrapper {
    /// Nested unit, or `None` when the wrapper itself is empty.
    pub unit: Option<Unit>,
}

/// Legacy unit identification (`JednotkaType`).
#[derive(Clone, Eq, PartialEq)]
pub struct Unit {
    /// Required internal unit ID (`id`); an empty ID is still a present value.
    pub id: LegacyId,
    /// Municipality-part value (`caobce`), retained verbatim.
    pub municipality_part: Option<String>,
    /// Building identification (`cp_ident`).
    pub building_identification: Option<String>,
    /// Unit identification (`jed_ident`).
    pub unit_identification: Option<String>,
    /// Ownership-sheet ID (`tel_id`), which may be present and empty.
    pub ownership_sheet_id: Option<LegacyId>,
    /// Ownership-sheet number (`cislo_tel`).
    pub ownership_sheet_number: Option<String>,
    /// Cadastral-area name (`ku_nazev`).
    pub cadastral_area_name: Option<String>,
}

/// Presence-preserving `PravoStavbyUpType` wrapper.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct BuildingRightWrapper {
    /// Nested building right, or `None` when the wrapper itself is empty.
    pub building_right: Option<BuildingRight>,
}

/// Legacy building-right identification (`PravoStavbyType`).
#[derive(Clone, Default, Eq, PartialEq)]
pub struct BuildingRight {
    /// Internal building-right ID (`id`), which may be present and empty.
    pub id: Option<LegacyId>,
    /// Ownership-sheet ID (`tel_id`), which may be present and empty.
    pub ownership_sheet_id: Option<LegacyId>,
    /// Repeated associated parcels, in document order.
    pub parcels: Vec<Parcel>,
    /// Ownership-sheet number (`cislo_tel`).
    pub ownership_sheet_number: Option<String>,
    /// Cadastral-area name (`ku_nazev`).
    pub cadastral_area_name: Option<String>,
}

/// Legacy proceeding identification (`RizeniType`).
///
/// The XSD permits an empty element, but requires all four fields whenever the
/// sequence is present.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct Proceeding {
    /// Complete proceeding number, or `None` for the XSD's empty sequence.
    pub identification: Option<ProceedingIdentification>,
}

/// The non-empty form of a legacy proceeding number.
#[derive(Clone, Eq, PartialEq)]
pub struct ProceedingIdentification {
    /// Dynamic proceeding-type code (`typriz_kod`).
    pub type_code: String,
    /// Serial number (`poradove_cislo`), covering all of `xs:unsignedInt`.
    pub serial_number: u32,
    /// Year (`rok`), covering all of `xs:unsignedShort`.
    pub year: u16,
    /// Workplace code (`prares_kod`), covering all of `xs:unsignedShort`.
    pub workplace_code: u16,
}

/// A legacy authorized-subject characteristic (`CharakteristikaType`).
#[derive(Clone, Default, Eq, PartialEq)]
pub struct Characteristic {
    /// Complete characteristic details, or `None` for the XSD's empty sequence.
    pub details: Option<CharacteristicDetails>,
}

/// Numeric legacy subject-characteristic code (`CharakteristikaOSKodType`).
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CharacteristicCode(u8);

impl CharacteristicCode {
    /// Validates the exact XSD range 1–99.
    pub fn new(value: u8) -> Result<Self> {
        if (1..=99).contains(&value) {
            Ok(Self(value))
        } else {
            Err(Error::invalid_input(
                "characteristic_code",
                "expected a code in 1..=99",
            ))
        }
    }

    /// Returns the numeric code.
    pub fn get(self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for CharacteristicCode {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self> {
        Self::new(value)
    }
}

impl From<CharacteristicCode> for u8 {
    fn from(value: CharacteristicCode) -> Self {
        value.get()
    }
}

/// The non-empty form of a legacy authorized-subject characteristic.
#[derive(Clone, Eq, PartialEq)]
pub struct CharacteristicDetails {
    /// Numeric code (`kod`) in the exact XSD range 1–99.
    pub code: CharacteristicCode,
    /// Abbreviation (`zkratka`).
    pub abbreviation: String,
    /// ALV abbreviation (`zkratka_alv`).
    pub alv_abbreviation: String,
}

/// Presence-preserving `CHAROS_TYPE` wrapper.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct CharacteristicWrapper {
    /// Nested characteristic, or `None` when `char_os` is absent.
    pub characteristic: Option<Characteristic>,
}

/// One entry in the repeating `OpsubIdentType` choice.
///
/// Every occurrence is retained separately, including repeated names, IDs,
/// characteristics, and addresses.
#[derive(Clone, Eq, PartialEq)]
#[non_exhaustive]
pub enum AuthorizedSubjectField {
    /// Authorized-subject ID (`id`).
    Id(LegacyId),
    /// Personal identifier fragment (`rc6`), retained as unrestricted text.
    PersonalNumber6(String),
    /// Personal identifier fragment (`rc7`), retained as unrestricted text.
    PersonalNumber7(String),
    /// Company identification number (`ico`), retained as unrestricted text.
    CompanyId(String),
    /// Company-ID suffix (`ico_doplnek`).
    CompanyIdSuffix(String),
    /// Subject characteristic (`char_os`).
    Characteristic(Characteristic),
    /// First partner's subject ID (`bsm1_id`).
    Partner1Id(LegacyId),
    /// Second partner's subject ID (`bsm2_id`).
    Partner2Id(LegacyId),
    /// Surname (`prijmeni`).
    Surname(String),
    /// Given name (`jmeno`).
    GivenName(String),
    /// Title before the name (`titul_pred`).
    TitleBefore(String),
    /// Title after the name (`titul_za`).
    TitleAfter(String),
    /// First partner's surname (`bsm1_prijmeni`).
    Partner1Surname(String),
    /// First partner's given name (`bsm1_jmeno`).
    Partner1GivenName(String),
    /// First partner's title before the name (`bsm1_pred`).
    Partner1TitleBefore(String),
    /// First partner's title after the name (`bsm1_za`).
    Partner1TitleAfter(String),
    /// Second partner's surname (`bsm2_prijmeni`).
    Partner2Surname(String),
    /// Second partner's given name (`bsm2_jmeno`).
    Partner2GivenName(String),
    /// Second partner's title before the name (`bsm2_pred`).
    Partner2TitleBefore(String),
    /// Second partner's title after the name (`bsm2_za`).
    Partner2TitleAfter(String),
    /// Legal-person or other subject name (`nazev`).
    Name(String),
    /// Address (`adresa`).
    Address(Address),
}

/// Repeating legacy authorized-subject identification (`OpsubIdentType`).
#[derive(Clone, Default, Eq, PartialEq)]
pub struct AuthorizedSubjectIdentification {
    /// Ordered, possibly empty or repeated choice entries.
    pub entries: Vec<AuthorizedSubjectField>,
}

/// A target in the repeating `VlaIdentType` choice.
#[derive(Clone, Eq, PartialEq)]
#[non_exhaustive]
pub enum OwnershipTarget {
    /// Authorized subject (`oprav_subjekt`).
    Subject(AuthorizedSubjectIdentification),
    /// Parcel (`parcela`).
    Parcel(Parcel),
    /// Building (`stavba`).
    Building(Building),
    /// Unit (`jednotka`).
    Unit(Unit),
    /// Building right (`pravo_stavby`).
    BuildingRight(BuildingRight),
}

/// Legacy ownership-target identification (`VlaIdentType`).
#[derive(Clone, Default, Eq, PartialEq)]
pub struct OwnershipIdentification {
    /// Empty, repeated, or mixed targets in their original document order.
    pub targets: Vec<OwnershipTarget>,
}

/// One entry in the nested `opravnena_po` choice in `OpsubIdentPoType`.
#[derive(Clone, Eq, PartialEq)]
#[non_exhaustive]
pub enum LegalEntityIdentificationEntry {
    /// A direct subject field.
    Field(AuthorizedSubjectField),
    /// Nested authorized-subject identification (`oprav_subjekt`).
    AuthorizedSubject(AuthorizedSubjectIdentification),
}

/// The nested `opravnena_po` identification block.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct LegalEntityIdentification {
    /// Ordered, possibly empty or repeated choice entries.
    pub entries: Vec<LegalEntityIdentificationEntry>,
}

/// One entry in the outer repeating `OpsubIdentPoType` choice.
#[derive(Clone, Eq, PartialEq)]
#[non_exhaustive]
pub enum AuthorizedSubjectPoEntry {
    /// Nested authorized legal entity (`opravnena_po`).
    AuthorizedLegalEntity(LegalEntityIdentification),
    /// A direct subject field.
    Field(AuthorizedSubjectField),
}

/// Legacy authorized-subject identification with a legal-entity block
/// (`OpsubIdentPoType`).
#[derive(Clone, Default, Eq, PartialEq)]
pub struct AuthorizedSubjectPoIdentification {
    /// Ordered, possibly empty or repeated choice entries.
    pub entries: Vec<AuthorizedSubjectPoEntry>,
}

/// A person's name and titles in `OpsubNazevType`.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct PersonNameForm {
    /// Surname.
    pub surname: Option<String>,
    /// Given name.
    pub given_name: Option<String>,
    /// Title before the name.
    pub title_before: Option<String>,
    /// Title after the name.
    pub title_after: Option<String>,
}

/// Fixed-sequence legacy subject-name form (`OpsubNazevType`).
#[derive(Clone, Default, Eq, PartialEq)]
pub struct SubjectNameForm {
    /// Authorized-subject ID (`id`), which may be present and empty.
    pub id: Option<LegacyId>,
    /// First partner's subject ID (`bsm1_id`).
    pub partner1_id: Option<LegacyId>,
    /// Second partner's subject ID (`bsm2_id`).
    pub partner2_id: Option<LegacyId>,
    /// Legal-person or other subject name (`nazev`).
    pub name: Option<String>,
    /// Primary natural-person name fields.
    pub person: PersonNameForm,
    /// First partner's name fields.
    pub partner1: PersonNameForm,
    /// Second partner's name fields.
    pub partner2: PersonNameForm,
}

/// A target in the repeating `NemIdentType` choice.
#[derive(Clone, Eq, PartialEq)]
#[non_exhaustive]
pub enum PropertyTarget {
    /// Parcel (`parcela`).
    Parcel(Parcel),
    /// Building (`stavba`).
    Building(Building),
    /// Unit (`jednotka`).
    Unit(Unit),
    /// Building right (`pravo_stavby`).
    BuildingRight(BuildingRight),
}

/// Legacy property identification (`NemIdentType`).
#[derive(Clone, Default, Eq, PartialEq)]
pub struct PropertyIdentification {
    /// Empty, repeated, or mixed property targets in document order.
    pub targets: Vec<PropertyTarget>,
}

/// A legacy ownership share (`PodilType`).
///
/// Components are independently optional and signed. In particular, no `1/1`
/// default is invented, and the legacy XSD does not require a nonzero
/// denominator. Parsed values have zero fractional digits.
#[derive(Clone, Copy, Default, Eq, PartialEq)]
pub struct ReportedShare {
    /// Numerator (`citatel`), when present.
    pub numerator: Option<Decimal>,
    /// Denominator (`jmenovatel`), when present; zero is permitted by the XSD.
    pub denominator: Option<Decimal>,
}

/// Legacy combined subject/property identification (`OsNemIdentType`).
///
/// Subject and property targets are independent optional fields, not an XSD
/// choice. Any schema-permitted subset can therefore be present together.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct SubjectPropertyIdentification {
    /// Legal-relation ID (`jpv_id`), which may be present and empty.
    pub legal_relation_id: Option<LegacyId>,
    /// Ownership-sheet ID (`tel_id`), which may be present and empty.
    pub ownership_sheet_id: Option<LegacyId>,
    /// Ownership-sheet number (`cislo_tel`).
    pub ownership_sheet_number: Option<String>,
    /// Authorized subject (`oprav_subjekt`).
    pub subject: Option<AuthorizedSubjectPoIdentification>,
    /// Parcel (`parcela`).
    pub parcel: Option<Parcel>,
    /// Building (`stavba`).
    pub building: Option<Building>,
    /// Unit (`jednotka`).
    pub unit: Option<Unit>,
    /// Building right (`pravo_stavby`).
    pub building_right: Option<BuildingRight>,
}

/// Section marker for a legacy deed list (`SekceListSekceType`).
#[derive(Clone, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum DeedSection {
    /// Section B1 (`B1`).
    B1,
    /// Section C (`C`).
    C,
    /// Section D (`D`).
    D,
    /// A future or otherwise unknown value, retained verbatim.
    Other(String),
}

impl DeedSection {
    /// Converts an untouched XML value to a known or raw-preserving variant.
    pub fn from_raw(raw: String) -> Self {
        match raw.as_str() {
            "B1" => Self::B1,
            "C" => Self::C,
            "D" => Self::D,
            _ => Self::Other(raw),
        }
    }

    /// Returns the official or untouched unknown value.
    pub fn as_str(&self) -> &str {
        match self {
            Self::B1 => "B1",
            Self::C => "C",
            Self::D => "D",
            Self::Other(raw) => raw,
        }
    }
}

/// A legacy yes/no value (`AnoNeType`) with forward-compatible raw fallback.
#[derive(Clone, Eq, Hash, PartialEq)]
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
    /// Converts an untouched XML value to a known or raw-preserving variant.
    pub fn from_raw(raw: String) -> Self {
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

/// An exact legacy `NUMBER(10,2)` value (`Number102Type`).
///
/// This is always backed by [`Decimal`]. Values that exceed `rust_decimal`'s
/// exact range are rejected by the XML parser with
/// [`Error::DecimalOutOfRange`], never rounded or truncated.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Number102(Decimal);

impl Number102 {
    /// Validates the XSD's ten-total-digit and two-fractional-digit limits.
    ///
    /// Insignificant trailing fractional zeroes are normalized before checking
    /// the value facets.
    pub fn new(value: Decimal) -> Result<Self> {
        let value = value.normalize();
        if value.scale() > 2 || decimal_digits(value) > 10 {
            return Err(Error::invalid_input(
                "number102",
                "expected an exact decimal with at most 10 total and 2 fractional digits",
            ));
        }
        Ok(Self(value))
    }

    /// Returns the exact decimal value.
    pub fn as_decimal(&self) -> &Decimal {
        &self.0
    }

    /// Consumes the wrapper and returns the exact decimal value.
    pub fn into_decimal(self) -> Decimal {
        self.0
    }
}

impl From<Number102> for Decimal {
    fn from(value: Number102) -> Self {
        value.into_decimal()
    }
}

fn decimal_digits(value: Decimal) -> usize {
    value.mantissa().unsigned_abs().to_string().len()
}
