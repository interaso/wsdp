//! Complete domain model for the NTR 3.0 ownership-sheet report.
//!
//! Optional collection fields correspond to optional XML wrapper elements:
//! `None` means that the wrapper was absent, while `Some(vec![])` means that it
//! was present but empty. Required XML wrappers are represented by plain
//! vectors.

use std::convert::Infallible;
use std::fmt;
use std::str::FromStr;

use rust_decimal::Decimal;

use super::super::common::XmlDateTime;
use super::super::id::{
    AddressId, BuildingId, BuildingRightId, CadastralAreaCode, DistrictCode, DocumentId,
    MunicipalityCode, MunicipalityPartCode, OwnershipId, OwnershipSheetId, ParcelId, ProceedingId,
    SubjectId, UnitId, WorkplaceCode,
};
use super::super::legal_relation::LegalRelationCode;
use super::super::report::ReportVersion;
use crate::{Error, Result, XmlDate};

macro_rules! open_code_enum {
    (
        $(#[$meta:meta])*
        $name:ident {
            $(
                $(#[$variant_meta:meta])*
                $variant:ident => $raw:literal
            ),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Clone, Eq, Hash, PartialEq)]
        #[non_exhaustive]
        pub enum $name {
            $(
                $(#[$variant_meta])*
                $variant,
            )+
            /// An unrecognized value, retained without changing its lexical form.
            Other(String),
        }

        impl $name {
            /// Parses a known value or retains an unrecognized value in [`Self::Other`].
            pub fn new(raw: impl Into<String>) -> Self {
                let raw = raw.into();
                match raw.as_str() {
                    $($raw => Self::$variant,)+
                    _ => Self::Other(raw),
                }
            }

            /// Returns the known value or the untouched unrecognized value.
            pub fn as_str(&self) -> &str {
                match self {
                    $(Self::$variant => $raw,)+
                    Self::Other(raw) => raw,
                }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(self.as_str())
            }
        }

        impl FromStr for $name {
            type Err = Infallible;

            fn from_str(raw: &str) -> std::result::Result<Self, Self::Err> {
                Ok(Self::new(raw))
            }
        }
    };
}

open_code_enum! {
    /// An NTR `AnoNeType` value.
    NtrYesNo {
        /// Yes (`a`).
        Yes => "a",
        /// No (`n`).
        No => "n"
    }
}

impl NtrYesNo {
    /// Returns `Some` for the schema's Boolean values and `None` for an unknown value.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Yes => Some(true),
            Self::No => Some(false),
            Self::Other(_) => None,
        }
    }
}

open_code_enum! {
    /// The kind of an authorized subject (`TypOSType`).
    NtrSubjectKind {
        /// Natural person (`OFO`).
        NaturalPerson => "OFO",
        /// Legal person (`OPO`).
        LegalPerson => "OPO",
        /// Marital or partner joint property (`BSM`).
        JointProperty => "BSM"
    }
}

open_code_enum! {
    /// A descriptive or evidentiary house-number kind (`CpCeType`).
    NtrHouseNumberKind {
        /// Descriptive house number (`1`).
        Descriptive => "1",
        /// Evidentiary house number (`2`).
        Evidentiary => "2"
    }
}

open_code_enum! {
    /// A cadastral parcel kind.
    NtrParcelKind {
        /// Full cadastral-register parcel (`PKN`).
        Cadastral => "PKN",
        /// Simplified-register parcel (`PZE`).
        Simplified => "PZE"
    }
}

open_code_enum! {
    /// A parcel-numbering method.
    NtrParcelNumberingMethodCode {
        /// Building parcel (`1`).
        BuildingParcel => "1",
        /// Land parcel (`2`).
        LandParcel => "2"
    }
}

open_code_enum! {
    /// The parcel-numbering series used in a cadastral area.
    NtrParcelNumberingSeriesCode {
        /// One numbering series (`1`).
        One => "1",
        /// Two numbering series (`2`).
        Two => "2"
    }
}

open_code_enum! {
    /// A seal marker (`PlombaType`).
    NtrSeal {
        /// The object is sealed (`P`).
        Sealed => "P"
    }
}

open_code_enum! {
    /// The kind of property of which a building forms a part.
    NtrBuildingPartKind {
        /// A parcel (`P`).
        Parcel => "P",
        /// A building right (`R`).
        BuildingRight => "R"
    }
}

open_code_enum! {
    /// A code for another-right section.
    NtrOtherRightSectionCode {
        /// Rights benefiting property in section B (`1`).
        BenefitingProperty => "1",
        /// Rights burdening property in section B (`2`).
        BurdeningProperty => "2",
        /// Notes and similar information (`4`).
        Notes => "4"
    }
}

open_code_enum! {
    /// The printed abbreviation of another-right section.
    NtrOtherRightSectionAbbreviation {
        /// Section B1 (`B1`).
        B1 => "B1",
        /// Section C (`C`).
        C => "C",
        /// Section D (`D`).
        D => "D"
    }
}

open_code_enum! {
    /// A server-maintained authorized-subject characteristic code.
    NtrSubjectCharacteristicCode {
        /// Spouses under foreign law (`16`).
        ForeignLawSpouses => "16",
        /// Unit fund (`17`).
        UnitFund => "17",
        /// Investment company (`18`).
        InvestmentCompany => "18"
    }
}

open_code_enum! {
    /// A simplified-register parcel-numbering source code.
    NtrParcelNumberingSourceCode {
        /// Property register (`3`).
        PropertyRegister => "3",
        /// Land cadastre (`4`).
        LandCadastre => "4",
        /// Allocation plan or another basis (`6`).
        AllocationPlanOrOther => "6"
    }
}

open_code_enum! {
    /// A simplified-register parcel character code.
    NtrParcelCharacterCode {
        /// Whole parcel (`1`).
        WholeParcel => "1",
        /// Parcel or parcel part without an area (`3`).
        WithoutArea => "3",
        /// Parcel group (`4`).
        ParcelGroup => "4"
    }
}

open_code_enum! {
    /// A server-maintained land-kind code.
    NtrLandKindCode {
        /// Arable land (`2`).
        ArableLand => "2",
        /// Hop field (`3`).
        HopField => "3",
        /// Vineyard (`4`).
        Vineyard => "4",
        /// Garden (`5`).
        Garden => "5"
    }
}

open_code_enum! {
    /// A server-maintained parcel-use code.
    NtrParcelUseCode {
        /// Greenhouse or hotbed (`1`).
        GreenhouseOrHotbed => "1",
        /// Nursery (`2`).
        Nursery => "2",
        /// Woody-plant plantation (`3`).
        Plantation => "3",
        /// Non-commercial forest (`4`).
        NonCommercialForest => "4"
    }
}

open_code_enum! {
    /// A server-maintained building-use code.
    NtrBuildingUseCode {
        /// Industrial building (`1`).
        Industrial => "1",
        /// Agricultural homestead (`2`).
        AgriculturalHomestead => "2",
        /// Residential building (`3`).
        Residential => "3"
    }
}

open_code_enum! {
    /// A server-maintained unit-use code.
    NtrUnitUseCode {
        /// Apartment (`1`).
        Apartment => "1",
        /// Studio (`2`).
        Studio => "2",
        /// Garage (`3`).
        Garage => "3",
        /// Workshop or operating premises (`4`).
        Workshop => "4"
    }
}

open_code_enum! {
    /// A server-maintained property-protection method code.
    NtrProtectionMethodCode {
        /// Small protected area (`1`).
        SmallProtectedArea => "1",
        /// Large protected area (`2`).
        LargeProtectedArea => "2",
        /// Specially protected natural area (`3`).
        SpecialNaturalArea => "3",
        /// Heritage-protected area (`4`).
        HeritageProtectedArea => "4"
    }
}

open_code_enum! {
    /// A server-maintained building-right purpose code.
    NtrBuildingRightPurposeCode {
        /// Apartment building (`1`).
        ApartmentBuilding => "1",
        /// Family house (`2`).
        FamilyHouse => "2",
        /// Sport and recreation structure (`3`).
        SportAndRecreation => "3",
        /// Services structure (`4`).
        Services => "4"
    }
}

open_code_enum! {
    /// A server-maintained building-type code.
    NtrBuildingTypeCode {
        /// Building with a descriptive number (`1`).
        DescriptiveNumber => "1",
        /// Building with an evidentiary number (`2`).
        EvidentiaryNumber => "2",
        /// Building without a descriptive or evidentiary number (`3`).
        WithoutHouseNumber => "3",
        /// Unfinished building (`4`).
        Unfinished => "4"
    }
}

open_code_enum! {
    /// A server-maintained unit-type code.
    NtrUnitTypeCode {
        /// Apartment or non-residential premises (`1`).
        ApartmentOrNonResidential => "1",
        /// Unfinished apartment or non-residential premises (`2`).
        Unfinished => "2",
        /// Unit defined under condominium legislation (`3`).
        CondominiumAct => "3",
        /// Unit defined under the Civil Code (`4`).
        CivilCode => "4"
    }
}

open_code_enum! {
    /// A server-maintained document-type code.
    NtrDocumentTypeCode {
        /// Property-transfer contract (`1`).
        PropertyTransferContract => "1",
        /// Personal-use-right agreement (`2`).
        PersonalUseRightAgreement => "2",
        /// Transfer-restriction agreement (`3`).
        TransferRestrictionAgreement => "3",
        /// Apartment-ownership-transfer contract (`4`).
        ApartmentTransferContract => "4",
        /// Inheritance decision (`5`).
        InheritanceDecision => "5"
    }
}

open_code_enum! {
    /// A server-maintained proceeding-type code.
    NtrProceedingTypeCode {
        /// Deposit proceeding (`Z`).
        Deposit => "Z",
        /// Entry proceeding (`V`).
        Entry => "V",
        /// `PU` proceeding.
        Pu => "PU",
        /// `PD` proceeding.
        Pd => "PD"
    }
}

open_code_enum! {
    /// A server-maintained detailed document-classification code.
    NtrDocumentDetailCode {
        /// Property administration (`650`).
        PropertyAdministration => "650",
        /// Enterprise seizure (`652`).
        EnterpriseSeizure => "652",
        /// Plant seizure (`738`).
        PlantSeizure => "738"
    }
}

macro_rules! ntr_decimal_id {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(String);

        impl $name {
            /// Validates a positive decimal identifier of at most 30 digits,
            /// retaining an optional leading `+` and all leading zeroes.
            pub fn new(value: impl Into<String>) -> Result<Self> {
                let value = value.into();
                let digits = value.strip_prefix('+').unwrap_or(&value);
                if digits.is_empty()
                    || digits.len() > 30
                    || !digits.bytes().all(|byte| byte.is_ascii_digit())
                    || digits.bytes().all(|byte| byte == b'0')
                {
                    return Err(Error::invalid_input(
                        "ntr_id",
                        "expected a positive decimal integer with at most 30 digits",
                    ));
                }
                Ok(Self(value))
            }

            /// Returns the identifier without numeric conversion.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(concat!(stringify!($name), "([ID])"))
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(self.as_str())
            }
        }

        impl FromStr for $name {
            type Err = Error;

            fn from_str(value: &str) -> Result<Self> {
                Self::new(value)
            }
        }
    };
}

ntr_decimal_id!(
    NtrReportRunId,
    "An ISKNI report-run identifier (`behId`), distinct from a WSDP queue report ID."
);
ntr_decimal_id!(
    NtrReportProcessingId,
    "An ISKNE report-processing identifier (`dpId`), distinct from a WSDP queue report ID."
);
ntr_decimal_id!(
    NtrOtherRightGroupId,
    "An identifier of a group or parent group of other legal relations."
);

/// The UUID assigned to an NTR report for external communication.
#[derive(Clone, Eq, Hash, PartialEq)]
pub struct NtrReportUuid(String);

impl NtrReportUuid {
    /// Validates and retains an UUID in the schema's 36-character lexical form.
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        let bytes = value.as_bytes();
        let valid = bytes.len() == 36
            && bytes.iter().enumerate().all(|(index, byte)| {
                if matches!(index, 8 | 13 | 18 | 23) {
                    *byte == b'-'
                } else {
                    byte.is_ascii_hexdigit()
                }
            });
        if valid {
            Ok(Self(value))
        } else {
            Err(Error::invalid_input(
                "report_uuid",
                "expected the UUID form prescribed by the NTR schema",
            ))
        }
    }

    /// Returns the original UUID text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for NtrReportUuid {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("NtrReportUuid([ID])")
    }
}

impl fmt::Display for NtrReportUuid {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for NtrReportUuid {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        Self::new(value)
    }
}

/// A natural-person birth number with redacted debug output.
#[derive(Clone, Eq, Hash, PartialEq)]
pub struct NtrBirthNumber(String);

impl NtrBirthNumber {
    /// Validates the NTR schema's 6–10 character constraint.
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        if (6..=10).contains(&value.chars().count()) {
            Ok(Self(value))
        } else {
            Err(Error::invalid_input(
                "birth_number",
                "expected between 6 and 10 characters",
            ))
        }
    }

    /// Explicitly exposes the birth number to callers that need it.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for NtrBirthNumber {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("NtrBirthNumber([REDACTED])")
    }
}

impl FromStr for NtrBirthNumber {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        Self::new(value)
    }
}

/// A complete NTR 3.0 `vypisZKatastruNemovitosti` document.
#[derive(Clone, Eq, PartialEq)]
pub struct NtrOwnershipSheet {
    /// General report-production and validity data.
    pub general_data: NtrGeneralData,
    /// Every ownership sheet in the report; a single report can contain multiple LVs.
    pub ownership_sheets: Vec<NtrOwnershipSheetEntry>,
    /// NTR-specific report UUID, run, application, and version metadata.
    pub metadata: NtrReportMetadata,
    /// The optional raw XMLDSIG/XAdES signature subtree, without interpretation or verification.
    pub signature: Option<NtrXmlSignature>,
}

/// General data from both `ObecneUdajeBaseType` and its LV extension.
#[derive(Clone, Eq, PartialEq)]
pub struct NtrGeneralData {
    /// Date and time at which the output is valid (`platnost`).
    pub valid_at: XmlDateTime,
    /// Workplace that produced the output, including optional contacts.
    pub workplace: Option<NtrWorkplaceWithContacts>,
    /// Optional GDPR notice (`pouceni`).
    pub privacy_notice: Option<String>,
    /// User who produced the output.
    pub user: Option<NtrReportUser>,
    /// Output creation timestamp (`vyhotoveno`).
    pub created_at: XmlDateTime,
    /// Whether this is a partial report (`castecnyVypis`).
    pub is_partial: NtrYesNo,
    /// Optional non-public-deed header (`neverejnaListina`).
    pub non_public_deed_header: Option<String>,
    /// Optional WSGP header (`wsgp`).
    pub wsgp_header: Option<String>,
    /// Optional output-verification information.
    pub verification: Option<NtrOutputVerification>,
}

/// Output-verification text supplied by ČÚZK.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct NtrOutputVerification {
    /// Verification clause (`dolozka`).
    pub clause: Option<String>,
    /// Text describing production or verification (`overeno`).
    pub verified: Option<String>,
    /// Text displayed for free-of-charge access (`bezplatny`).
    pub free_access: Option<String>,
}

/// Identification of the user who created an NTR output.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct NtrReportUser {
    /// Login name (`uzivatelskeJmeno`), normally omitted by WSDP.
    pub username: Option<String>,
    /// Full name or another display identification (`celeJmeno`).
    pub full_name: Option<String>,
}

/// A cadastral workplace.
#[derive(Clone, Eq, PartialEq)]
pub struct NtrWorkplace {
    /// Workplace code.
    pub code: WorkplaceCode,
    /// Optional official workplace name.
    pub name: Option<String>,
}

/// A cadastral workplace with optional address and contact details.
#[derive(Clone, Eq, PartialEq)]
pub struct NtrWorkplaceWithContacts {
    /// Base workplace identification.
    pub workplace: NtrWorkplace,
    /// Workplace address.
    pub address: Option<NtrAddress>,
    /// Telephone, fax, email, and data-box contacts.
    pub contact: Option<NtrContact>,
}

/// Contact details for a cadastral workplace.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct NtrContact {
    /// Telephone number.
    pub telephone: Option<String>,
    /// Fax number.
    pub fax: Option<String>,
    /// Email address.
    pub email: Option<String>,
    /// Data-box identifier (`idds`).
    pub data_box_id: Option<String>,
}

/// A district as represented by NTR `OkresType`.
#[derive(Clone, Eq, PartialEq)]
pub struct NtrDistrict {
    /// District code; NTR restricts it to four digits.
    pub code: DistrictCode,
    /// District name.
    pub name: String,
    /// Optional NUTS 4 code.
    pub nuts4: Option<String>,
}

/// A municipality as represented by NTR `ObecType`.
#[derive(Clone, Eq, PartialEq)]
pub struct NtrMunicipality {
    /// Municipality code.
    pub code: MunicipalityCode,
    /// Municipality name.
    pub name: String,
}

/// A municipality part as represented by NTR `CastObceType`.
#[derive(Clone, Eq, PartialEq)]
pub struct NtrMunicipalityPart {
    /// Municipality-part code.
    pub code: MunicipalityPartCode,
    /// Municipality-part name.
    pub name: String,
}

/// A cadastral area with its required code and name.
#[derive(Clone, Eq, PartialEq)]
pub struct NtrCadastralArea {
    /// Cadastral-area code.
    pub code: CadastralAreaCode,
    /// Cadastral-area name.
    pub name: String,
}

/// The numbering series used for parcels in a cadastral area.
#[derive(Clone, Eq, PartialEq)]
pub struct NtrParcelNumberingSeries {
    /// Numbering-series code.
    pub code: NtrParcelNumberingSeriesCode,
    /// Human-readable description.
    pub name: String,
}

/// One complete LV contained in an NTR ownership-sheet report.
#[derive(Clone, Eq, PartialEq)]
pub struct NtrOwnershipSheetEntry {
    /// Internal LV identifier.
    pub id: OwnershipSheetId,
    /// LV number within its cadastral area.
    pub number: u32,
    /// District containing the LV's cadastral area.
    pub district: NtrDistrict,
    /// Municipality in which the LV's property lies.
    pub municipality: NtrMunicipality,
    /// Cadastral area for which the LV is maintained.
    pub cadastral_area: NtrCadastralArea,
    /// Cadastral workplace responsible for the LV.
    pub workplace: NtrWorkplace,
    /// Parcel-numbering series in the cadastral area.
    pub numbering_series: NtrParcelNumberingSeries,
    /// Section A legal relations; `None` preserves an absent `pravniVztahy` wrapper.
    pub owner_relations: Option<Vec<NtrLegalRelation>>,
    /// Section B property collections.
    pub properties: NtrProperties,
    /// Other legal relations in sections B1, C, and D.
    pub other_rights: Vec<NtrOtherRight>,
    /// Seals and notices attached to the LV.
    pub notices: Vec<NtrNotice>,
    /// Acquisition titles and other supporting documents.
    pub acquisition_titles: Vec<NtrAcquisitionTitle>,
    /// Soil-quality parcel parts in section F.
    pub bonities: Vec<NtrBonity>,
}

/// All property collections in section B of an LV.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct NtrProperties {
    /// Full-register parcels (`parcely`).
    pub parcels: Option<Vec<NtrParcel>>,
    /// Buildings (`stavby`).
    pub buildings: Option<Vec<NtrBuilding>>,
    /// Building rights (`pravaStavby`).
    pub building_rights: Option<Vec<NtrBuildingRight>>,
    /// Units (`jednotky`).
    pub units: Option<Vec<NtrUnit>>,
    /// Simplified-register parcels (`parcelyZe`).
    pub simplified_parcels: Option<Vec<NtrParcel>>,
}

/// A coded legal relation with its recursively nested ownership records.
#[derive(Clone, Eq, PartialEq)]
pub struct NtrLegalRelation {
    /// Required legal-relation code, including raw preservation of unknown codes.
    pub code: LegalRelationCode,
    /// Required relation name supplied by the report.
    pub name: String,
    /// Ownership or accessory-co-ownership records in the required `seznamVlastnictvi` wrapper.
    pub ownership_records: Vec<NtrOwnershipRecord>,
}

impl NtrLegalRelation {
    /// Returns registry ownership classification, or `None` for an unknown code.
    pub fn is_ownership(&self) -> Option<bool> {
        self.code.is_ownership()
    }
}

/// An ownership record extended with optional subordinate legal relations.
#[derive(Clone, Eq, PartialEq)]
pub struct NtrOwnershipRecord {
    /// Required base ownership data.
    pub ownership: NtrOwnership,
    /// Structural subordinate relations, retained independently of `parent_id` links.
    pub subordinate_relations: Option<Vec<NtrLegalRelation>>,
}

/// Base NTR ownership data (`VlastnictviType`).
#[derive(Clone, Eq, PartialEq)]
pub struct NtrOwnership {
    /// Required ownership-record identifier.
    pub id: OwnershipId,
    /// Optional identifier of a superior ownership record (`nadrizeneId`).
    pub parent_id: Option<OwnershipId>,
    /// Required positive exact share.
    pub share: NtrShare,
    /// Exactly one subject or property selected by the XSD choice.
    pub target: NtrOwnershipTarget,
}

/// The single subject or property targeted by an NTR ownership record.
#[derive(Clone, Eq, PartialEq)]
pub enum NtrOwnershipTarget {
    /// An authorized subject.
    Subject(Box<NtrAuthorizedSubject>),
    /// A parcel, used for accessory co-ownership.
    Parcel(Box<NtrParcel>),
    /// A building, used for accessory co-ownership.
    Building(Box<NtrBuilding>),
    /// A unit, used for accessory co-ownership.
    Unit(Box<NtrUnit>),
    /// A building right, used for accessory co-ownership.
    BuildingRight(Box<NtrBuildingRight>),
}

/// An exact positive ownership or unit share.
///
/// Both components are positive integers. Values outside `rust_decimal`'s exact
/// range are rejected during parsing rather than rounded or truncated.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct NtrShare {
    numerator: Decimal,
    denominator: Decimal,
}

impl NtrShare {
    /// Creates a share after validating positive integral numerator and denominator.
    pub fn new(numerator: Decimal, denominator: Decimal) -> Result<Self> {
        if !positive_integer(numerator) || !positive_integer(denominator) {
            return Err(Error::invalid_input(
                "share",
                "share components must be positive integers",
            ));
        }
        Ok(Self {
            numerator,
            denominator,
        })
    }

    /// Returns the positive numerator.
    pub fn numerator(&self) -> Decimal {
        self.numerator
    }

    /// Returns the positive, nonzero denominator.
    pub fn denominator(&self) -> Decimal {
        self.denominator
    }
}

fn positive_integer(value: Decimal) -> bool {
    value.is_sign_positive() && !value.is_zero() && value.fract().is_zero()
}

/// An authorized subject, including one required and one optional participant.
#[derive(Clone, Eq, PartialEq)]
pub struct NtrAuthorizedSubject {
    /// Outer authorized-subject identifier, distinct from nested participant IDs.
    pub id: SubjectId,
    /// Subject kind (`OFO`, `OPO`, or `BSM`).
    pub kind: NtrSubjectKind,
    /// Required dynamic characteristic code (`charKod`).
    pub characteristic_code: NtrSubjectCharacteristicCode,
    /// Optional characteristic abbreviation (`charZkratka`).
    pub characteristic_abbreviation: Option<String>,
    /// Optional section-A characteristic abbreviation (`charZkratkaAlv`).
    pub alv_abbreviation: Option<String>,
    /// Optional joint-property name (`nazevSJ`).
    pub joint_property_name: Option<String>,
    /// Primary subject or first joint-property participant (`oS`).
    pub primary: NtrSubjectDetails,
    /// Optional second joint-property participant (`oS2`).
    pub secondary: Option<NtrSubjectDetails>,
}

/// Nested details of a person, organization, or joint-property participant.
#[derive(Clone, Eq, PartialEq)]
pub struct NtrSubjectDetails {
    /// Required nested subject or participant identifier.
    pub id: SubjectId,
    /// Nested subject kind.
    pub kind: NtrSubjectKind,
    /// Given name (`jmeno`).
    pub given_name: Option<String>,
    /// Family name (`prijmeni`).
    pub surname: Option<String>,
    /// Title before the name (`titulPred`).
    pub title_before: Option<String>,
    /// Title after the name (`titulZa`).
    pub title_after: Option<String>,
    /// Organization name (`nazev`).
    pub organization_name: Option<String>,
    /// Birth number, redacted from debug output (`rodneCislo`).
    pub birth_number: Option<NtrBirthNumber>,
    /// Company identification number, retaining leading zeroes (`ico`).
    pub company_id: Option<String>,
    /// Numeric company-ID suffix (`icoDoplnek`).
    pub company_id_suffix: Option<u16>,
    /// Required person or organization address.
    pub address: NtrAddress,
}

/// A subject or workplace address (`AdresaType`).
#[derive(Clone, Default, Eq, PartialEq)]
pub struct NtrAddress {
    /// Orientation number, retained as text (`cisloOrientacni`).
    pub orientation_number: Option<String>,
    /// House number (`cisloDomovni`).
    pub house_number: Option<u16>,
    /// Descriptive or evidentiary house-number kind.
    pub house_number_kind: Option<NtrHouseNumberKind>,
    /// Street name (`nazevUlice`).
    pub street_name: Option<String>,
    /// Municipality-part name (`castObce`).
    pub municipality_part: Option<String>,
    /// City-district name (`mestskaCast`).
    pub city_district: Option<String>,
    /// City-borough name (`mestskyObvod`).
    pub city_borough: Option<String>,
    /// Municipality name (`obec`).
    pub municipality: Option<String>,
    /// District name (`okres`).
    pub district: Option<String>,
    /// Postal code, retaining its lexical representation (`psc`).
    pub postal_code: Option<String>,
    /// Delivery post office (`dodPosta`).
    pub delivery_post_office: Option<String>,
    /// Country name (`stat`); no default country is invented.
    pub country: Option<String>,
    /// RÚIAN address-point identifier (`kodAdrm`).
    pub ruian_id: Option<AddressId>,
}

/// A parcel, including nested building and ownership-sheet relations.
#[derive(Clone, Eq, PartialEq)]
pub struct NtrParcel {
    /// Required internal parcel identifier.
    pub id: ParcelId,
    /// Related building identifier (`budId`).
    pub building_id: Option<BuildingId>,
    /// Whether this parcel identifies the related building (`budIdent`).
    pub identifies_building: Option<NtrYesNo>,
    /// Related building-right identifier (`psId`).
    pub building_right_id: Option<BuildingRightId>,
    /// Whether this parcel identifies the related building right (`psIdent`).
    pub identifies_building_right: Option<NtrYesNo>,
    /// Full or simplified register kind (`typ`).
    pub kind: NtrParcelKind,
    /// Simplified-register parcel character (`charakter`).
    pub character: Option<NtrParcelCharacter>,
    /// Root parcel number (`kmenCis`).
    pub root_number: u32,
    /// Optional parcel subdivision (`poddCis`).
    pub subdivision: Option<u16>,
    /// Optional parcel seal.
    pub seal: Option<NtrSeal>,
    /// Optional parcel part (`dil`).
    pub part: Option<u8>,
    /// Parcel area in square metres (`vymera`).
    pub area: u32,
    /// Simplified-register numbering source (`zdrojParcel`).
    pub numbering_source: Option<NtrParcelNumberingSource>,
    /// Parcel-numbering method (`druhCislovani`).
    pub numbering_method: Option<NtrParcelNumberingMethod>,
    /// Land kind (`druhPozemku`).
    pub land_kind: Option<NtrLandKind>,
    /// Parcel use (`zpusobVyuziti`).
    pub land_use: Option<NtrParcelUse>,
    /// Optional protection-method wrapper and its items.
    pub protection_methods: Option<Vec<NtrProtectionMethod>>,
    /// Current cadastral area.
    pub cadastral_area: NtrCadastralArea,
    /// Original cadastral area (`katastrUzemiPuv`).
    pub original_cadastral_area: Option<NtrCadastralArea>,
    /// LV on which the parcel is recorded; absence means no LV.
    pub ownership_sheet: Option<NtrOwnershipSheetReference>,
    /// Whether a building is a component of the parcel (`soucasti`).
    pub has_building_as_part: Option<NtrYesNo>,
    /// Building on the parcel. Boxing breaks the recursive parcel/building graph.
    pub building: Option<Box<NtrBuilding>>,
}

/// Simplified-register parcel-numbering source information.
#[derive(Clone, Eq, PartialEq)]
pub struct NtrParcelNumberingSource {
    /// Dynamic source code.
    pub code: NtrParcelNumberingSourceCode,
    /// Source name.
    pub name: String,
    /// Optional source abbreviation.
    pub abbreviation: Option<String>,
}

/// Parcel-numbering method information.
#[derive(Clone, Eq, PartialEq)]
pub struct NtrParcelNumberingMethod {
    /// Numbering-method code.
    pub code: NtrParcelNumberingMethodCode,
    /// Optional method name.
    pub name: Option<String>,
    /// Optional method abbreviation.
    pub abbreviation: Option<String>,
}

/// Simplified-register parcel character information.
#[derive(Clone, Eq, PartialEq)]
pub struct NtrParcelCharacter {
    /// Dynamic parcel-character code.
    pub code: NtrParcelCharacterCode,
    /// Optional character name.
    pub name: Option<String>,
    /// Optional character abbreviation.
    pub abbreviation: Option<String>,
}

/// Land-kind code and required name.
#[derive(Clone, Eq, PartialEq)]
pub struct NtrLandKind {
    /// Dynamic land-kind code.
    pub code: NtrLandKindCode,
    /// Land-kind name.
    pub name: String,
}

/// Parcel-use code, name, and optional abbreviation.
#[derive(Clone, Eq, PartialEq)]
pub struct NtrParcelUse {
    /// Dynamic parcel-use code.
    pub code: NtrParcelUseCode,
    /// Parcel-use name.
    pub name: String,
    /// Optional parcel-use abbreviation.
    pub abbreviation: Option<String>,
}

/// Building-use code, name, and optional abbreviation.
#[derive(Clone, Eq, PartialEq)]
pub struct NtrBuildingUse {
    /// Dynamic building-use code.
    pub code: NtrBuildingUseCode,
    /// Building-use name.
    pub name: String,
    /// Optional building-use abbreviation.
    pub abbreviation: Option<String>,
}

/// Unit-use code, name, and optional abbreviation.
#[derive(Clone, Eq, PartialEq)]
pub struct NtrUnitUse {
    /// Dynamic unit-use code.
    pub code: NtrUnitUseCode,
    /// Unit-use name.
    pub name: String,
    /// Optional unit-use abbreviation.
    pub abbreviation: Option<String>,
}

/// A property-protection method.
#[derive(Clone, Eq, PartialEq)]
pub struct NtrProtectionMethod {
    /// Dynamic protection-method code.
    pub code: NtrProtectionMethodCode,
    /// Protection-method name.
    pub name: String,
}

/// A building and its nested properties and units.
#[derive(Clone, Eq, PartialEq)]
pub struct NtrBuilding {
    /// Required internal building identifier.
    pub id: BuildingId,
    /// Related parcel identifier (`parId`).
    pub parcel_id: Option<ParcelId>,
    /// Related building-right identifier (`psId`).
    pub building_right_id: Option<BuildingRightId>,
    /// Unstructured list of house numbers (`cislaDomovni`).
    pub house_numbers: Option<String>,
    /// Temporary-building flag (`docasna`).
    pub temporary: Option<NtrYesNo>,
    /// Optional building seal.
    pub seal: Option<NtrSeal>,
    /// Municipality part in which the building lies.
    pub municipality_part: Option<NtrMunicipalityPart>,
    /// Building type (`typBudovy`).
    pub building_type: Option<NtrBuildingType>,
    /// Building use (`zpusobVyuziti`).
    pub building_use: Option<NtrBuildingUse>,
    /// Optional protection-method wrapper and its items.
    pub protection_methods: Option<Vec<NtrProtectionMethod>>,
    /// Cadastral area in which the building lies.
    pub cadastral_area: NtrCadastralArea,
    /// LV on which the building is recorded; absence means no LV.
    pub ownership_sheet: Option<NtrOwnershipSheetReference>,
    /// Parcels on which the building stands.
    pub parcels: Option<Vec<NtrParcel>>,
    /// Units defined in the building.
    pub units: Option<Vec<NtrUnit>>,
    /// Parcel or building right of which this building is a component (`soucasti`).
    pub part_of: Option<NtrBuildingPart>,
}

/// Building-type code, name, and optional abbreviation.
#[derive(Clone, Eq, PartialEq)]
pub struct NtrBuildingType {
    /// Dynamic building-type code.
    pub code: NtrBuildingTypeCode,
    /// Building-type name.
    pub name: String,
    /// Optional building-type abbreviation.
    pub abbreviation: Option<String>,
}

/// The required `nem` attribute and selected target of a building's `soucasti` element.
#[derive(Clone, Eq, PartialEq)]
pub struct NtrBuildingPart {
    /// Raw property-kind attribute, retained independently of the selected element.
    pub kind: NtrBuildingPartKind,
    /// The one parcel or building-right child selected by the XSD choice.
    pub target: NtrBuildingPartTarget,
}

/// The property of which a building forms a part.
#[derive(Clone, Eq, PartialEq)]
pub enum NtrBuildingPartTarget {
    /// A parcel target, boxed to keep the choice small and recursive graphs indirection-based.
    Parcel(Box<NtrParcel>),
    /// A building-right target, boxed to keep both choice variants equally small.
    BuildingRight(Box<NtrBuildingRight>),
}

/// A building right and its related parcels, buildings, and purposes.
#[derive(Clone, Eq, PartialEq)]
pub struct NtrBuildingRight {
    /// Required internal building-right identifier.
    pub id: BuildingRightId,
    /// Optional end of validity (`datumUkonceni`).
    pub ended_on: Option<XmlDate>,
    /// Optional building-right seal.
    pub seal: Option<NtrSeal>,
    /// Cadastral area in which the right is registered.
    pub cadastral_area: NtrCadastralArea,
    /// LV on which the right is recorded; absence means no LV.
    pub ownership_sheet: Option<NtrOwnershipSheetReference>,
    /// Parcels subject to the building right.
    pub parcels: Option<Vec<NtrParcel>>,
    /// Buildings forming part of the building right.
    pub buildings: Option<Vec<NtrBuilding>>,
    /// Optional purpose wrapper and its items (`ucely`).
    pub purposes: Option<Vec<NtrBuildingRightPurpose>>,
    /// Optional protection-method wrapper and its items.
    pub protection_methods: Option<Vec<NtrProtectionMethod>>,
}

/// A purpose assigned to a building right.
#[derive(Clone, Eq, PartialEq)]
pub struct NtrBuildingRightPurpose {
    /// Dynamic purpose code.
    pub code: NtrBuildingRightPurposeCode,
    /// Purpose name.
    pub name: String,
}

/// A unit and all of its NTR relationships.
#[derive(Clone, Eq, PartialEq)]
pub struct NtrUnit {
    /// Required internal unit identifier.
    pub id: UnitId,
    /// Full unit number (`cisloJednotky`).
    pub number: u64,
    /// Building house number (`cisloDomovni`).
    pub house_number: Option<u16>,
    /// Non-negative number derived from the full unit number (`cislo`).
    pub derived_number: u16,
    /// Unit description (`popis`).
    pub description: Option<String>,
    /// Optional unit seal.
    pub seal: Option<NtrSeal>,
    /// LV on which the unit is recorded; absence means no LV.
    pub ownership_sheet: Option<NtrOwnershipSheetReference>,
    /// Required positive share in common parts.
    pub share: NtrShare,
    /// Unit type.
    pub unit_type: NtrUnitType,
    /// Optional unit use.
    pub unit_use: Option<NtrUnitUse>,
    /// Optional protection-method wrapper and its items.
    pub protection_methods: Option<Vec<NtrProtectionMethod>>,
    /// Optional unit co-ownership wrapper (`seznamVlastnictvi`).
    pub co_ownership: Option<Vec<NtrOwnership>>,
    /// Parcels on the same LV as the unit's building.
    pub parcels: Option<Vec<NtrParcel>>,
    /// Building containing the unit. Boxing keeps recursive object graphs bounded in size.
    pub building: Option<Box<NtrBuilding>>,
    /// Buildings on the same LV as the unit's building.
    pub buildings: Option<Vec<NtrBuilding>>,
    /// Building rights on the same LV as the unit's building.
    pub building_rights: Option<Vec<NtrBuildingRight>>,
}

/// Unit-type code, name, and optional abbreviation.
#[derive(Clone, Eq, PartialEq)]
pub struct NtrUnitType {
    /// Dynamic unit-type code.
    pub code: NtrUnitTypeCode,
    /// Unit-type name.
    pub name: String,
    /// Optional unit-type abbreviation.
    pub abbreviation: Option<String>,
}

/// A nested LV reference (`LvType`).
#[derive(Clone, Eq, PartialEq)]
pub struct NtrOwnershipSheetReference {
    /// Internal LV identifier.
    pub id: OwnershipSheetId,
    /// LV number.
    pub number: u32,
}

/// A document in the collection of deeds (`ListinaType`).
#[derive(Clone, Eq, PartialEq)]
pub struct NtrDocument {
    /// Formatted document text (`formatText`).
    pub formatted_text: Option<String>,
    /// Required internal document identifier.
    pub id: DocumentId,
    /// Document creation date (`vyhotoveno`).
    pub created_on: Option<XmlDate>,
    /// Date on which the document acquired legal force (`pravniMoc`).
    pub legal_force_on: Option<XmlDate>,
    /// Decision enforceability date (`vykonatelnost`).
    pub enforceability_on: Option<XmlDate>,
    /// Change-statement item (`pvz`).
    pub change_statement_item: Option<String>,
    /// Required document type.
    pub document_type: NtrDocumentType,
    /// Additional description (`popis`).
    pub description: Option<String>,
    /// Optional detailed-classification wrapper and its items (`dalsiUdaje`).
    pub details: Option<Vec<NtrDocumentDetail>>,
    /// Proceeding-number abbreviation (`zkratka`).
    pub abbreviation: Option<String>,
    /// Document originator (`zhotovitel`).
    pub originator: Option<String>,
    /// Originator's serial number, retained as text (`poradoveCislo`).
    pub originator_serial_number: Option<String>,
    /// Originator's document year (`rokVyhotoveni`).
    pub originator_year: Option<u16>,
    /// Proceeding-number supplement (`doplnek`).
    pub supplement: Option<String>,
    /// Workplace at which the document is stored.
    pub workplace: Option<NtrWorkplace>,
    /// Proceeding for which this is a supporting document.
    pub proceeding: NtrProceeding,
}

/// Document-type code and required name.
#[derive(Clone, Eq, PartialEq)]
pub struct NtrDocumentType {
    /// Dynamic document-type code.
    pub code: NtrDocumentTypeCode,
    /// Document-type name.
    pub name: String,
}

/// Detailed document-classification information.
#[derive(Clone, Eq, PartialEq)]
pub struct NtrDocumentDetail {
    /// Dynamic detail code.
    pub code: NtrDocumentDetailCode,
    /// Optional detail name.
    pub name: Option<String>,
}

/// A cadastral proceeding (`RizeniType`).
#[derive(Clone, Eq, PartialEq)]
pub struct NtrProceeding {
    /// Required internal proceeding identifier.
    pub id: ProceedingId,
    /// Proceeding type.
    pub proceeding_type: NtrProceedingType,
    /// Serial number (`poradoveCislo`).
    pub serial_number: u32,
    /// Proceeding year (`rok`).
    pub year: u16,
    /// Workplace that initiated the proceeding.
    pub workplace: NtrWorkplace,
    /// Receipt timestamp (`prijato`).
    pub received_at: Option<XmlDateTime>,
    /// Validation date (`zplatneno`).
    pub validated_on: Option<XmlDate>,
    /// Client-supplied external identifier (`extIdentifikace`).
    pub external_id: Option<String>,
}

/// A proceeding-type code (`TypRizeniType`).
#[derive(Clone, Eq, PartialEq)]
pub struct NtrProceedingType {
    /// Dynamic proceeding-type code.
    pub code: NtrProceedingTypeCode,
}

/// A proceeding and the properties affected by it (`ObjektRizeniType`).
#[derive(Clone, Eq, PartialEq)]
pub struct NtrProceedingObject {
    /// Proceeding identification.
    pub proceeding: NtrProceeding,
    /// Parcels affected by the proceeding.
    pub parcels: Option<Vec<NtrParcel>>,
    /// Buildings affected by the proceeding.
    pub buildings: Option<Vec<NtrBuilding>>,
    /// Units affected by the proceeding.
    pub units: Option<Vec<NtrUnit>>,
    /// Building rights affected by the proceeding.
    pub building_rights: Option<Vec<NtrBuildingRight>>,
}

/// A coded legal relation attached to another right.
#[derive(Clone, Eq, PartialEq)]
pub struct NtrCodedLegalRelation {
    /// Required legal-relation code, including raw unknown values.
    pub code: LegalRelationCode,
    /// Required legal-relation name.
    pub name: String,
}

impl NtrCodedLegalRelation {
    /// Returns registry ownership classification, or `None` for an unknown code.
    pub fn is_ownership(&self) -> Option<bool> {
        self.code.is_ownership()
    }
}

/// Another legal relation in section B1, C, or D (`JinePravoType`).
#[derive(Clone, Eq, PartialEq)]
pub struct NtrOtherRight {
    /// Required section identification.
    pub section: NtrOtherRightSection,
    /// Optional unique group identifier (`hjpvId`).
    pub group_id: Option<NtrOtherRightGroupId>,
    /// Optional superior group identifier (`nadrizHjpvId`).
    pub parent_group_id: Option<NtrOtherRightGroupId>,
    /// Optional coded legal-relation type.
    pub legal_relation: Option<NtrCodedLegalRelation>,
    /// Optional temporal or textual ordering (`poradi`).
    pub ordering: Option<NtrOtherRightOrdering>,
    /// Optional related-JPV flag (`souvisejiciJpv`).
    pub related: Option<NtrYesNo>,
    /// Optional free-text description (`popis`).
    pub description: Option<String>,
    /// Relations benefiting a subject or property (`jpvPro`).
    pub for_relation: Option<NtrOtherRightRelations>,
    /// Relations burdening a subject or property (`jpvK`).
    pub against_relation: Option<NtrOtherRightRelations>,
    /// Relations without property or subject links (`jpv`).
    pub unbound_relation: Option<NtrOtherRightRelations>,
}

/// Section identification for another legal relation.
#[derive(Clone, Eq, PartialEq)]
pub struct NtrOtherRightSection {
    /// Numeric section code.
    pub code: NtrOtherRightSectionCode,
    /// Section name.
    pub name: String,
    /// Printed section abbreviation (`B1`, `C`, or `D`).
    pub abbreviation: NtrOtherRightSectionAbbreviation,
}

/// Temporal or textual ordering of another legal relation.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct NtrOtherRightOrdering {
    /// Optional ordering timestamp (`cas`).
    pub time: Option<XmlDateTime>,
    /// Optional priority-order text (`text`).
    pub text: Option<String>,
}

/// Subject, property, LV, and document links associated with another right.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct NtrOtherRightRelations {
    /// Optional authorized-subject wrapper, including representatives and claim shares.
    pub subjects: Option<Vec<NtrCreditorSubject>>,
    /// Optional parcel wrapper.
    pub parcels: Option<Vec<NtrParcel>>,
    /// Optional building wrapper.
    pub buildings: Option<Vec<NtrBuilding>>,
    /// Optional unit wrapper.
    pub units: Option<Vec<NtrUnit>>,
    /// Optional building-right wrapper.
    pub building_rights: Option<Vec<NtrBuildingRight>>,
    /// Optional LV associated with an otherwise unbound relation.
    pub ownership_sheet: Option<NtrOwnershipSheetReference>,
    /// Optional collection-of-deeds wrapper.
    pub documents: Option<Vec<NtrDocument>>,
}

/// An authorized subject with an optional representative and claim share.
#[derive(Clone, Eq, PartialEq)]
pub struct NtrCreditorSubject {
    /// Base authorized-subject data.
    pub subject: NtrAuthorizedSubject,
    /// Optional exercising authorized representative (`opravnenaPo`).
    pub authorized_representative: Option<NtrAuthorizedSubject>,
    /// Optional textual claim share (`podilPohledavky`), not coerced to a numeric value.
    pub claim_share: Option<String>,
}

/// A seal or notice linked to proceeding objects.
#[derive(Clone, Eq, PartialEq)]
pub struct NtrNotice {
    /// Notice text (`nazev`).
    pub name: String,
    /// Proceeding objects in the required `objektyRizeni` wrapper.
    pub proceeding_objects: Vec<NtrProceedingObject>,
}

/// An acquisition title or other supporting basis in section E.
#[derive(Clone, Eq, PartialEq)]
pub struct NtrAcquisitionTitle {
    /// Required acquisition-title document.
    pub document: NtrDocument,
    /// Optional authorized-subject wrapper and its items.
    pub subjects: Option<Vec<NtrAuthorizedSubject>>,
}

/// A soil-quality parcel part in section F.
#[derive(Clone, Eq, PartialEq)]
pub struct NtrBonity {
    /// BPEJ code or the literal `nebonitovano` (`bpejKod`).
    pub bpej_code: String,
    /// Area of the soil-quality part in square metres (`vymera`).
    pub area: u32,
    /// Parcel to which the soil-quality part belongs.
    pub parcel: NtrParcel,
}

/// NTR report metadata, kept separate from WSDP asynchronous queue metadata.
#[derive(Clone, Eq, PartialEq)]
pub struct NtrReportMetadata {
    /// UUID used for external communication (`identifikatorSestavy`).
    pub report_uuid: NtrReportUuid,
    /// Optional ISKNI report-run identifier (`behId`).
    pub run_id: Option<NtrReportRunId>,
    /// Optional ISKNE report-processing identifier (`dpId`).
    pub processing_id: Option<NtrReportProcessingId>,
    /// Environment and ISKNI/ISKNE discriminator (`prostredi`).
    pub environment: String,
    /// Copyright text.
    pub copyright: String,
    /// Producing application name (`aplikace`).
    pub application: String,
    /// Full producing-application version (`verzeAplikace`).
    pub application_version: String,
    /// Application module code (`kodSestavy`).
    pub report_code: String,
    /// Report-module version in `MAJOR.MINOR` form (`verzeSestavy`).
    pub report_version: ReportVersion,
}

/// An untouched XMLDSIG/XAdES `ds:Signature` subtree.
///
/// Retaining this XML does not assert authenticity or perform cryptographic
/// verification. The original report bytes remain the authoritative source for
/// canonicalization-sensitive verification.
#[derive(Clone, Eq, PartialEq)]
pub struct NtrXmlSignature {
    /// Exact UTF-8 source substring for the signature element and all descendants.
    pub raw_xml: String,
}
