//! Redacted formatting for report payloads; content is never recursively formatted.

macro_rules! redacted_debug {
    ($(super::$module:ident::$type:ident),+ $(,)?) => {
        $(
            impl std::fmt::Debug for super::$module::$type {
                fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    formatter.write_str(concat!(stringify!($type), "([REDACTED])"))
                }
            }
        )+
    };
}

redacted_debug!(
    super::account_statement::AccountStatementReport,
    super::account_statement::CustomerAccount,
    super::account_statement::AccountProceeding,
    super::account_statement::AccountParticipant,
    super::account_statement::AccountChargeGroup,
    super::account_statement::AccountChargeRow,
);

redacted_debug!(
    super::definition_points::DefinitionPointsReport,
    super::definition_points::DefinitionPoint,
);

redacted_debug!(
    super::legacy_base::HouseNumberKind,
    super::legacy_base::Address,
    super::legacy_base::AddressGroup,
    super::legacy_base::DeedName,
    super::legacy_base::DeedNameDetails,
    super::legacy_base::ParcelWrapper,
    super::legacy_base::Parcel,
    super::legacy_base::BuildingWrapper,
    super::legacy_base::Building,
    super::legacy_base::UnitWrapper,
    super::legacy_base::Unit,
    super::legacy_base::BuildingRightWrapper,
    super::legacy_base::BuildingRight,
    super::legacy_base::Proceeding,
    super::legacy_base::ProceedingIdentification,
    super::legacy_base::Characteristic,
    super::legacy_base::CharacteristicCode,
    super::legacy_base::CharacteristicDetails,
    super::legacy_base::CharacteristicWrapper,
    super::legacy_base::AuthorizedSubjectField,
    super::legacy_base::AuthorizedSubjectIdentification,
    super::legacy_base::OwnershipTarget,
    super::legacy_base::OwnershipIdentification,
    super::legacy_base::LegalEntityIdentificationEntry,
    super::legacy_base::LegalEntityIdentification,
    super::legacy_base::AuthorizedSubjectPoEntry,
    super::legacy_base::AuthorizedSubjectPoIdentification,
    super::legacy_base::PersonNameForm,
    super::legacy_base::SubjectNameForm,
    super::legacy_base::PropertyTarget,
    super::legacy_base::PropertyIdentification,
    super::legacy_base::ReportedShare,
    super::legacy_base::SubjectPropertyIdentification,
    super::legacy_base::DeedSection,
    super::legacy_base::YesNo,
    super::legacy_base::Number102,
);

redacted_debug!(
    super::legacy_ownership_overview::LegacySubjectIdentification,
    super::legacy_ownership_overview::LegacySubjectIdentifier,
    super::legacy_ownership_overview::LegacyReportDistrict,
    super::legacy_ownership_overview::LegacyReportCodeName,
    super::legacy_ownership_overview::OwnershipOverviewReport,
    super::legacy_ownership_overview::OwnershipOverviewSubject,
    super::legacy_ownership_overview::OwnershipOverviewDistrictGroup,
    super::legacy_ownership_overview::OwnershipOverviewMunicipalityGroup,
    super::legacy_ownership_overview::OwnershipOverviewCadastralAreaGroup,
    super::legacy_ownership_overview::OwnershipOverviewLegalRelationGroup,
    super::legacy_ownership_overview::OwnershipOverviewRecord,
    super::legacy_ownership_overview::OwnershipOverviewWithPropertiesReport,
    super::legacy_ownership_overview::OwnershipPropertiesSubject,
    super::legacy_ownership_overview::OwnershipPropertiesDistrictGroup,
    super::legacy_ownership_overview::OwnershipPropertiesMunicipalityGroup,
    super::legacy_ownership_overview::OwnershipPropertiesCadastralAreaGroup,
    super::legacy_ownership_overview::OwnershipPropertiesRecord,
    super::legacy_ownership_overview::OwnershipParcel,
    super::legacy_ownership_overview::OwnershipBuilding,
    super::legacy_ownership_overview::OwnershipUnit,
    super::legacy_ownership_overview::OwnershipSource,
    super::legacy_ownership_overview::OwnershipSourceParcel,
    super::legacy_ownership_overview::OwnershipOriginalCadastralArea,
    super::legacy_ownership_overview::OwnershipBuildingRight,
    super::legacy_ownership_overview::OwnershipBuildingRightEntry,
    super::legacy_ownership_overview::RightsRecordReport,
    super::legacy_ownership_overview::RightsRecordQuery,
    super::legacy_ownership_overview::RightsExactMatch,
    super::legacy_ownership_overview::RightsMismatch,
    super::legacy_ownership_overview::RightsPartialMatch,
    super::legacy_ownership_overview::RightsPartialMatchDetails,
    super::legacy_ownership_overview::RightsPartialSubject,
    super::legacy_ownership_overview::RightsOwnership,
    super::legacy_ownership_overview::RightsIdentificationMatch,
    super::legacy_ownership_overview::RightsSubject,
    super::legacy_ownership_overview::RightsLegalRelationGroup,
    super::legacy_ownership_overview::RightsDistrict,
    super::legacy_ownership_overview::RightsMunicipality,
    super::legacy_ownership_overview::RightsCadastralArea,
    super::legacy_ownership_overview::RightsOwnershipRecord,
);

redacted_debug!(
    super::legacy_ownership_sheet::LegacyOwnershipSheetReport,
    super::legacy_ownership_sheet::LegacyXmlSignature,
    super::legacy_ownership_sheet::LegacyRawXml,
    super::legacy_ownership_sheet::LegacyOwnershipSheetText,
    super::legacy_ownership_sheet::LegacySheetHeader,
    super::legacy_ownership_sheet::LegacyDistrict,
    super::legacy_ownership_sheet::LegacySheetMunicipality,
    super::legacy_ownership_sheet::LegacySheetCadastralArea,
    super::legacy_ownership_sheet::LegacyOwnerRelation,
    super::legacy_ownership_sheet::LegacyOwner,
    super::legacy_ownership_sheet::LegacyParcel,
    super::legacy_ownership_sheet::LegacyComponentBuildings,
    super::legacy_ownership_sheet::LegacyBuildingPart,
    super::legacy_ownership_sheet::LegacyBuildingPartOfParcel,
    super::legacy_ownership_sheet::LegacyBuildingPartOfRight,
    super::legacy_ownership_sheet::LegacyParcelUnit,
    super::legacy_ownership_sheet::LegacyParcelUnitDetails,
    super::legacy_ownership_sheet::LegacyUnitType,
    super::legacy_ownership_sheet::LegacyUnitUse,
    super::legacy_ownership_sheet::LegacySheetReference,
    super::legacy_ownership_sheet::LegacyOptionalSheetReference,
    super::legacy_ownership_sheet::LegacyUnitOwnership,
    super::legacy_ownership_sheet::LegacyBuilding,
    super::legacy_ownership_sheet::LegacyBuildingUnit,
    super::legacy_ownership_sheet::LegacyBuildingRight,
    super::legacy_ownership_sheet::LegacyBuildingRightDetails,
    super::legacy_ownership_sheet::LegacyBuildingRightUnit,
    super::legacy_ownership_sheet::LegacyBuildingRightUnitDetails,
    super::legacy_ownership_sheet::LegacyAcquisitionTitle,
    super::legacy_ownership_sheet::LegacyAcquisitionBeneficiaryGroup,
    super::legacy_ownership_sheet::LegacyStandaloneUnit,
    super::legacy_ownership_sheet::LegacyUnitBuilding,
    super::legacy_ownership_sheet::LegacyUnitBuildingDetails,
    super::legacy_ownership_sheet::LegacyUnitBuildingSubject,
    super::legacy_ownership_sheet::LegacyUnitBuildingRightGroup,
    super::legacy_ownership_sheet::LegacyUnitBuildingRight,
    super::legacy_ownership_sheet::LegacyUnitBuildingRightEntry,
    super::legacy_ownership_sheet::LegacyUnitRightParcel,
    super::legacy_ownership_sheet::LegacyUnitParcel,
    super::legacy_ownership_sheet::LegacyBonity,
    super::legacy_ownership_sheet::LegacyBonityParcel,
    super::legacy_ownership_sheet::LegacyBonityParcelDetails,
    super::legacy_ownership_sheet::LegacyOtherCadastralArea,
    super::legacy_ownership_sheet::LegacyOtherCadastralAreaDetails,
    super::legacy_ownership_sheet::LegacyBpej,
    super::legacy_ownership_sheet::LegacyNoticeGroup,
    super::legacy_ownership_sheet::LegacyNoticeProceeding,
    super::legacy_ownership_sheet::LegacyNoticeProceedingDetails,
    super::legacy_ownership_sheet::LegacyNoticeProperty,
    super::legacy_ownership_sheet::LegacyNoticePropertyDetails,
    super::legacy_ownership_sheet::LegacyLegalRelationChange,
    super::legacy_ownership_sheet::LegacySimplifiedParcelSource,
    super::legacy_ownership_sheet::LegacySimplifiedParcelSourceDetails,
    super::legacy_ownership_sheet::LegacySimplifiedSource,
    super::legacy_ownership_sheet::LegacySimplifiedSourceDetails,
    super::legacy_ownership_sheet::LegacySimplifiedParcel,
    super::legacy_ownership_sheet::LegacyOriginalCadastralArea,
    super::legacy_ownership_sheet::LegacyCadastralOperation,
    super::legacy_ownership_sheet::LegacyOperationSubjectMatter,
    super::legacy_ownership_sheet::LegacyOtherRightsSection,
    super::legacy_ownership_sheet::LegacyOtherRight,
    super::legacy_ownership_sheet::LegacyRelationPriority,
    super::legacy_ownership_sheet::LegacyOtherRightDescription,
    super::legacy_ownership_sheet::LegacyDescriptionProceeding,
    super::legacy_ownership_sheet::LegacyBenefitingSubject,
    super::legacy_ownership_sheet::LegacyBenefitingProperty,
    super::legacy_ownership_sheet::LegacyObligatedSubject,
    super::legacy_ownership_sheet::LegacyObligatedProperty,
    super::legacy_ownership_sheet::LegacyOtherRightDeed,
    super::legacy_ownership_sheet::LegacyOwnershipSheetFooter,
    super::legacy_ownership_sheet::LegacyWorkplaceFooter,
    super::legacy_ownership_sheet::LegacyPropertiesWithoutSheet,
    super::legacy_ownership_sheet::LegacyPropertyWithoutSheetGroup,
    super::legacy_ownership_sheet::LegacyParcelWithoutSheet,
);

redacted_debug!(
    super::legacy_property_info::ParcelInfoReport,
    super::legacy_property_info::BuildingInfoReport,
    super::legacy_property_info::PropertyDistrict,
    super::legacy_property_info::PropertyMunicipality,
    super::legacy_property_info::ParcelCadastre,
    super::legacy_property_info::ParcelCadastralArea,
    super::legacy_property_info::ParcelCadastralAreaDetails,
    super::legacy_property_info::CadastralParcel,
    super::legacy_property_info::ParcelPrimaryUseCode,
    super::legacy_property_info::ParcelComponentBuilding,
    super::legacy_property_info::CadastralParcelSeal,
    super::legacy_property_info::CadastralBonity,
    super::legacy_property_info::ParcelSource,
    super::legacy_property_info::SimplifiedParcel,
    super::legacy_property_info::SimplifiedParcelSeal,
    super::legacy_property_info::SimplifiedBonity,
    super::legacy_property_info::PropertyOwnership,
    super::legacy_property_info::PropertyOwnershipDetails,
    super::legacy_property_info::PropertyOwnershipSheet,
    super::legacy_property_info::PropertyOwnershipSubject,
    super::legacy_property_info::BuildingReport,
    super::legacy_property_info::BuildingCadastralArea,
    super::legacy_property_info::BuildingInfo,
    super::legacy_property_info::BuildingAddressGroup,
    super::legacy_property_info::BuildingPart,
    super::legacy_property_info::BuildingPartOfParcel,
    super::legacy_property_info::BuildingPartOfRight,
    super::legacy_property_info::BuildingLegalRelationLink,
    super::legacy_property_info::BuildingLegalRelationText,
    super::legacy_property_info::BuildingParcelGroup,
    super::legacy_property_info::BuildingParcel,
    super::legacy_property_info::BuildingParcelSheet,
    super::legacy_property_info::BuildingParcelSheetDetails,
    super::legacy_property_info::BuildingParcelSheetId,
    super::legacy_property_info::BuildingParcelOwner,
    super::legacy_property_info::BuildingParcelCadastralArea,
    super::legacy_property_info::BuildingParcelCadastralAreaDetails,
);

redacted_debug!(
    super::legacy_unit_reports::LegacyDistrict,
    super::legacy_unit_reports::LegacyNumericCodeName,
    super::legacy_unit_reports::LegacyTextCodeName,
    super::legacy_unit_reports::UnitInfoReport,
    super::legacy_unit_reports::UnitInfoCadastralArea,
    super::legacy_unit_reports::UnitInfoBuilding,
    super::legacy_unit_reports::UnitInfoBuildingParcel,
    super::legacy_unit_reports::UnitInfoUnit,
    super::legacy_unit_reports::UnitInfoFloorArea,
    super::legacy_unit_reports::UnitInfoLegalRelationLink,
    super::legacy_unit_reports::UnitInfoOwnership,
    super::legacy_unit_reports::UnitInfoOwnershipDetails,
    super::legacy_unit_reports::UnitInfoOwnershipSheet,
    super::legacy_unit_reports::UnitInfoSubject,
    super::legacy_unit_reports::UnitInfoParcel,
    super::legacy_unit_reports::UnitInfoParcelOwnershipSheet,
    super::legacy_unit_reports::UnitInfoParcelSealProceeding,
    super::legacy_unit_reports::UnitInfoSeal,
    super::legacy_unit_reports::BuildingRightInfoReport,
    super::legacy_unit_reports::BuildingRightInfoHeader,
    super::legacy_unit_reports::BuildingRightInfoRecord,
    super::legacy_unit_reports::BuildingRightInfoProceedingIdentification,
    super::legacy_unit_reports::BuildingRightInfoComponent,
    super::legacy_unit_reports::BuildingRightInfoOwnershipSheet,
    super::legacy_unit_reports::BuildingRightInfoOwnershipSheetReference,
    super::legacy_unit_reports::BuildingRightInfoOwnership,
    super::legacy_unit_reports::ProceedingInfoReport,
    super::legacy_unit_reports::ProceedingInfoParticipant,
    super::legacy_unit_reports::ProceedingInfoParticipantName,
    super::legacy_unit_reports::ProceedingInfoParticipantNameChoice,
    super::legacy_unit_reports::ProceedingInfoPersonName,
    super::legacy_unit_reports::ProceedingInfoParticipantIdentification,
    super::legacy_unit_reports::ProceedingInfoParticipantIdentificationEntry,
    super::legacy_unit_reports::ProceedingInfoParticipationKind,
    super::legacy_unit_reports::ProceedingInfoDetails,
    super::legacy_unit_reports::ProceedingInfoPaymentState,
    super::legacy_unit_reports::ProceedingInfoSubject,
    super::legacy_unit_reports::ProceedingInfoCadastralAreaGroup,
    super::legacy_unit_reports::ProceedingInfoCadastralArea,
    super::legacy_unit_reports::ProceedingInfoParcel,
    super::legacy_unit_reports::ProceedingInfoBuilding,
    super::legacy_unit_reports::ProceedingInfoDeed,
    super::legacy_unit_reports::ProceedingInfoUnit,
    super::legacy_unit_reports::ProceedingInfoGeometricPlan,
    super::legacy_unit_reports::ProceedingInfoEvent,
    super::legacy_unit_reports::ProceedingInfoOperation,
    super::legacy_unit_reports::ProceedingInfoOperationText,
    super::legacy_unit_reports::ProceedingInfoDefect,
    super::legacy_unit_reports::ProceedingInfoChangeProposal,
    super::legacy_unit_reports::ProceedingInfoBuildingRight,
);

redacted_debug!(
    super::ownership_sheet::OwnershipSheetVersion,
    super::ownership_sheet::OwnershipSheetReport,
);

redacted_debug!(
    super::prices::PricesByPropertiesReport,
    super::prices::PriceCadastralUnit,
    super::prices::PriceDistrict,
    super::prices::PriceMunicipality,
    super::prices::PriceCadastralArea,
    super::prices::PriceProperty,
    super::prices::PriceParentProperty,
    super::prices::PriceParentPropertyDetails,
    super::prices::PricePropertyIdent,
    super::prices::PricePropertyIdentItem,
    super::prices::PriceParcel,
    super::prices::PriceBuilding,
    super::prices::PriceUnit,
    super::prices::PriceBuildingRight,
    super::prices::PriceProceedingReference,
    super::prices::PriceProceedingNumber,
    super::prices::PriceDeed,
    super::prices::PriceDeedInfo,
    super::prices::PriceDeedDetails,
    super::prices::PriceRelatedType,
    super::prices::PriceRelatedTypeDetails,
    super::prices::PriceRelatedProperty,
    super::prices::PriceRelatedPropertyDetails,
    super::prices::PricesByProceedingReport,
    super::prices::PriceWorkplace,
    super::prices::PriceProceeding,
    super::prices::MonthlyPriceData,
    super::prices::MonthlyPriceDataProperty,
    super::prices::MonthlyYesNo,
    super::prices::MonthlyPropertyKind,
    super::prices::MonthlyProceeding,
    super::prices::MonthlyDeedCollection,
    super::prices::MonthlyPropertyCollection,
    super::prices::MonthlyDeed,
    super::prices::MonthlyProperty,
    super::prices::MonthlyPropertyBody,
    super::prices::MonthlyPointProperty,
    super::prices::MonthlyPoint,
    super::prices::MonthlyPointPosition,
    super::prices::MonthlyDirectPosition,
    super::prices::MonthlyCoordinates,
    super::prices::MonthlyExpandedName,
    super::prices::MonthlyXmlAttribute,
    super::prices::MonthlyXmlAttributes,
    super::prices::MonthlyRawXmlElement,
    super::prices::MonthlyPriceArchiveLimits,
    super::prices::MonthlyPriceArchive,
    super::prices::MonthlyPriceEntry,
    super::prices::MonthlyPriceEntryMetadata,
);

redacted_debug!(
    super::ownership_sheet_v3::NtrYesNo,
    super::ownership_sheet_v3::NtrSubjectKind,
    super::ownership_sheet_v3::NtrHouseNumberKind,
    super::ownership_sheet_v3::NtrParcelKind,
    super::ownership_sheet_v3::NtrParcelNumberingMethodCode,
    super::ownership_sheet_v3::NtrParcelNumberingSeriesCode,
    super::ownership_sheet_v3::NtrSeal,
    super::ownership_sheet_v3::NtrBuildingPartKind,
    super::ownership_sheet_v3::NtrOtherRightSectionCode,
    super::ownership_sheet_v3::NtrOtherRightSectionAbbreviation,
    super::ownership_sheet_v3::NtrSubjectCharacteristicCode,
    super::ownership_sheet_v3::NtrParcelNumberingSourceCode,
    super::ownership_sheet_v3::NtrParcelCharacterCode,
    super::ownership_sheet_v3::NtrLandKindCode,
    super::ownership_sheet_v3::NtrParcelUseCode,
    super::ownership_sheet_v3::NtrBuildingUseCode,
    super::ownership_sheet_v3::NtrUnitUseCode,
    super::ownership_sheet_v3::NtrProtectionMethodCode,
    super::ownership_sheet_v3::NtrBuildingRightPurposeCode,
    super::ownership_sheet_v3::NtrBuildingTypeCode,
    super::ownership_sheet_v3::NtrUnitTypeCode,
    super::ownership_sheet_v3::NtrDocumentTypeCode,
    super::ownership_sheet_v3::NtrProceedingTypeCode,
    super::ownership_sheet_v3::NtrDocumentDetailCode,
    super::ownership_sheet_v3::NtrOwnershipSheet,
    super::ownership_sheet_v3::NtrGeneralData,
    super::ownership_sheet_v3::NtrOutputVerification,
    super::ownership_sheet_v3::NtrReportUser,
    super::ownership_sheet_v3::NtrWorkplace,
    super::ownership_sheet_v3::NtrWorkplaceWithContacts,
    super::ownership_sheet_v3::NtrContact,
    super::ownership_sheet_v3::NtrDistrict,
    super::ownership_sheet_v3::NtrMunicipality,
    super::ownership_sheet_v3::NtrMunicipalityPart,
    super::ownership_sheet_v3::NtrCadastralArea,
    super::ownership_sheet_v3::NtrParcelNumberingSeries,
    super::ownership_sheet_v3::NtrOwnershipSheetEntry,
    super::ownership_sheet_v3::NtrProperties,
    super::ownership_sheet_v3::NtrLegalRelation,
    super::ownership_sheet_v3::NtrOwnershipRecord,
    super::ownership_sheet_v3::NtrOwnership,
    super::ownership_sheet_v3::NtrOwnershipTarget,
    super::ownership_sheet_v3::NtrShare,
    super::ownership_sheet_v3::NtrAuthorizedSubject,
    super::ownership_sheet_v3::NtrSubjectDetails,
    super::ownership_sheet_v3::NtrAddress,
    super::ownership_sheet_v3::NtrParcel,
    super::ownership_sheet_v3::NtrParcelNumberingSource,
    super::ownership_sheet_v3::NtrParcelNumberingMethod,
    super::ownership_sheet_v3::NtrParcelCharacter,
    super::ownership_sheet_v3::NtrLandKind,
    super::ownership_sheet_v3::NtrParcelUse,
    super::ownership_sheet_v3::NtrBuildingUse,
    super::ownership_sheet_v3::NtrUnitUse,
    super::ownership_sheet_v3::NtrProtectionMethod,
    super::ownership_sheet_v3::NtrBuilding,
    super::ownership_sheet_v3::NtrBuildingType,
    super::ownership_sheet_v3::NtrBuildingPart,
    super::ownership_sheet_v3::NtrBuildingPartTarget,
    super::ownership_sheet_v3::NtrBuildingRight,
    super::ownership_sheet_v3::NtrBuildingRightPurpose,
    super::ownership_sheet_v3::NtrUnit,
    super::ownership_sheet_v3::NtrUnitType,
    super::ownership_sheet_v3::NtrOwnershipSheetReference,
    super::ownership_sheet_v3::NtrDocument,
    super::ownership_sheet_v3::NtrDocumentType,
    super::ownership_sheet_v3::NtrDocumentDetail,
    super::ownership_sheet_v3::NtrProceeding,
    super::ownership_sheet_v3::NtrProceedingType,
    super::ownership_sheet_v3::NtrProceedingObject,
    super::ownership_sheet_v3::NtrCodedLegalRelation,
    super::ownership_sheet_v3::NtrOtherRight,
    super::ownership_sheet_v3::NtrOtherRightSection,
    super::ownership_sheet_v3::NtrOtherRightOrdering,
    super::ownership_sheet_v3::NtrOtherRightRelations,
    super::ownership_sheet_v3::NtrCreditorSubject,
    super::ownership_sheet_v3::NtrNotice,
    super::ownership_sheet_v3::NtrAcquisitionTitle,
    super::ownership_sheet_v3::NtrBonity,
    super::ownership_sheet_v3::NtrReportMetadata,
    super::ownership_sheet_v3::NtrXmlSignature,
);

#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use bytes::Bytes;
    use rust_decimal::Decimal;

    use super::super::{BinaryReport, XmlReport, ZipReport, ownership_sheet_v3::*, prices::*};
    use crate::{
        AddressId, DownloadedReport, MessageCode, MessageLevel, OwnershipId, ReportFormat,
        ReportId, ReportMetadata, ReportState, ReportVersion, ServiceMessage, SubjectId,
        XmlDateTime,
    };

    const SENTINEL: &str = "WSDP_PRIVATE_SENTINEL_7C9F2A";
    const UUID: &str = "123e4567-e89b-12d3-a456-426614174000";
    const REPORT_ID: &str = "111222333444555";
    const PARENT_REPORT_ID: &str = "222333444555666";
    const CHILD_REPORT_ID: &str = "333444555666777";
    const SUBJECT_ID: &str = "444555666777888";
    const OWNERSHIP_ID: &str = "555666777888999";
    const ADDRESS_ID: &str = "666777888999111";
    const RUN_ID: &str = "777888999111222";
    const BIRTH_NUMBER: &str = "9876543210";

    #[track_caller]
    fn assert_debug_without<T: Debug + ?Sized>(value: &T, sentinels: &[&str]) {
        let rendered = format!("{value:?}");
        assert!(!rendered.is_empty(), "redacted debug output was empty");
        for sentinel in std::iter::once(SENTINEL).chain(sentinels.iter().copied()) {
            assert!(
                !rendered.contains(sentinel),
                "debug output contained {sentinel:?}: {rendered}"
            );
        }
    }

    fn prices_report() -> PricesByPropertiesReport {
        PricesByPropertiesReport {
            cadastral_units: vec![PriceCadastralUnit {
                district: PriceDistrict {
                    nuts4: Some(SENTINEL.to_owned()),
                    name: Some(SENTINEL.to_owned()),
                },
                municipality: PriceMunicipality {
                    code: 1,
                    name: SENTINEL.to_owned(),
                },
                cadastral_area: Some(PriceCadastralArea {
                    code: 2,
                    name: SENTINEL.to_owned(),
                }),
                properties: Vec::new(),
            }],
            free_access: SENTINEL.to_owned(),
            validity: SENTINEL.to_owned(),
            prepared_by: SENTINEL.to_owned(),
            output_info: SENTINEL.to_owned(),
            prepared_at: SENTINEL.to_owned(),
        }
    }

    fn monthly_raw_xml() -> MonthlyRawXmlElement {
        let name = MonthlyExpandedName {
            namespace: Some(SENTINEL.to_owned()),
            local_name: SENTINEL.to_owned(),
        };
        MonthlyRawXmlElement {
            name: name.clone(),
            attributes: MonthlyXmlAttributes {
                values: vec![MonthlyXmlAttribute {
                    name,
                    value: SENTINEL.to_owned(),
                }],
            },
            xml: format!("<gml:description>{SENTINEL}</gml:description>"),
        }
    }

    fn monthly_price_archive() -> MonthlyPriceArchive {
        MonthlyPriceArchive {
            original_bytes: SENTINEL.as_bytes().to_vec(),
            entries: vec![MonthlyPriceEntry {
                metadata: MonthlyPriceEntryMetadata {
                    index: 0,
                    name: SENTINEL.to_owned(),
                    name_raw: SENTINEL.as_bytes().to_vec(),
                    compressed_size: 28,
                    uncompressed_size: 28,
                    crc32: 42,
                },
                original_bytes: SENTINEL.as_bytes().to_vec(),
                value: MonthlyPriceData {
                    without_measurement: vec![MonthlyYesNo::Yes],
                    months: vec![11],
                    year: SENTINEL.to_owned(),
                    cadastral_area_code: 123_456,
                    proceedings: Vec::new(),
                },
            }],
        }
    }

    fn ntr_report() -> NtrOwnershipSheet {
        let timestamp = XmlDateTime::new("2026-11-20T10:00:00Z").unwrap();
        NtrOwnershipSheet {
            general_data: NtrGeneralData {
                valid_at: timestamp.clone(),
                workplace: None,
                privacy_notice: Some(SENTINEL.to_owned()),
                user: Some(NtrReportUser {
                    username: Some(SENTINEL.to_owned()),
                    full_name: Some(SENTINEL.to_owned()),
                }),
                created_at: timestamp,
                is_partial: NtrYesNo::No,
                non_public_deed_header: Some(SENTINEL.to_owned()),
                wsgp_header: Some(SENTINEL.to_owned()),
                verification: Some(NtrOutputVerification {
                    clause: Some(SENTINEL.to_owned()),
                    verified: Some(SENTINEL.to_owned()),
                    free_access: Some(SENTINEL.to_owned()),
                }),
            },
            ownership_sheets: Vec::new(),
            metadata: NtrReportMetadata {
                report_uuid: NtrReportUuid::new(UUID).unwrap(),
                run_id: Some(NtrReportRunId::new(RUN_ID).unwrap()),
                processing_id: None,
                environment: SENTINEL.to_owned(),
                copyright: SENTINEL.to_owned(),
                application: SENTINEL.to_owned(),
                application_version: SENTINEL.to_owned(),
                report_code: SENTINEL.to_owned(),
                report_version: ReportVersion::V3_0,
            },
            signature: Some(NtrXmlSignature {
                raw_xml: format!("<ds:Signature>{SENTINEL}</ds:Signature>"),
            }),
        }
    }

    fn report_metadata() -> ReportMetadata {
        ReportMetadata {
            id: ReportId::new(REPORT_ID).unwrap(),
            name: SENTINEL.to_owned(),
            units: None,
            pages: None,
            price: None,
            requested_at: XmlDateTime::new("2026-11-20T10:00:00Z").unwrap(),
            started_at: None,
            created_at: None,
            parent_id: Some(ReportId::new(PARENT_REPORT_ID).unwrap()),
            child_ids: vec![ReportId::new(CHILD_REPORT_ID).unwrap()],
            state: ReportState::Charged,
            format: ReportFormat::Other(SENTINEL.to_owned()),
            version: Some(ReportVersion::V3_0),
            electronic_seal: None,
            timestamp: None,
        }
    }

    fn service_messages() -> Vec<ServiceMessage> {
        vec![ServiceMessage {
            code: MessageCode::Other(SENTINEL.to_owned()),
            level: MessageLevel::Other(SENTINEL.to_owned()),
            text: SENTINEL.to_owned(),
        }]
    }

    #[test]
    fn prices_and_monthly_raw_content_are_debug_redacted() {
        let report = prices_report();
        assert_debug_without(&report, &[]);
        assert_debug_without(&report.cadastral_units[0], &[]);
        assert_debug_without(&report.cadastral_units[0].district, &[]);
        assert_debug_without(
            report.cadastral_units[0].cadastral_area.as_ref().unwrap(),
            &[],
        );

        let raw = monthly_raw_xml();
        assert_debug_without(&raw, &[]);
        assert_debug_without(&raw.name, &[]);
        assert_debug_without(&raw.attributes, &[]);
        assert_debug_without(&raw.attributes.values[0], &[]);

        let archive = monthly_price_archive();
        assert_debug_without(&archive, &[]);
        assert_debug_without(&archive.entries[0], &[]);
        assert_debug_without(&archive.entries[0].metadata, &[]);
        assert_debug_without(&archive.entries[0].value, &[]);

        assert_eq!(
            format!("{:?}", MonthlyYesNo::Yes),
            "MonthlyYesNo([REDACTED])"
        );
        assert_eq!(
            format!("{:?}", MonthlyPriceArchiveLimits::default()),
            "MonthlyPriceArchiveLimits([REDACTED])"
        );
    }

    #[test]
    fn ntr_nested_owners_identifiers_and_raw_xml_are_debug_redacted() {
        let subject = NtrSubjectDetails {
            id: SubjectId::new(SUBJECT_ID).unwrap(),
            kind: NtrSubjectKind::Other(SENTINEL.to_owned()),
            given_name: Some(SENTINEL.to_owned()),
            surname: Some(SENTINEL.to_owned()),
            title_before: None,
            title_after: None,
            organization_name: Some(SENTINEL.to_owned()),
            birth_number: Some(NtrBirthNumber::new(BIRTH_NUMBER).unwrap()),
            company_id: Some(SENTINEL.to_owned()),
            company_id_suffix: None,
            address: NtrAddress {
                street_name: Some(SENTINEL.to_owned()),
                postal_code: Some(SENTINEL.to_owned()),
                ruian_id: Some(AddressId::new(ADDRESS_ID).unwrap()),
                ..NtrAddress::default()
            },
        };
        assert_debug_without(&subject, &[SUBJECT_ID, ADDRESS_ID, BIRTH_NUMBER]);
        assert_debug_without(&subject.address, &[ADDRESS_ID]);
        assert_debug_without(subject.birth_number.as_ref().unwrap(), &[BIRTH_NUMBER]);

        let owner = NtrAuthorizedSubject {
            id: SubjectId::new(SUBJECT_ID).unwrap(),
            kind: NtrSubjectKind::NaturalPerson,
            characteristic_code: NtrSubjectCharacteristicCode::Other(SENTINEL.to_owned()),
            characteristic_abbreviation: Some(SENTINEL.to_owned()),
            alv_abbreviation: Some(SENTINEL.to_owned()),
            joint_property_name: Some(SENTINEL.to_owned()),
            primary: subject.clone(),
            secondary: Some(subject),
        };
        let ownership = NtrOwnership {
            id: OwnershipId::new(OWNERSHIP_ID).unwrap(),
            parent_id: None,
            share: NtrShare::new(
                Decimal::from(123_456_789_u64),
                Decimal::from(987_654_321_u64),
            )
            .unwrap(),
            target: NtrOwnershipTarget::Subject(Box::new(owner)),
        };
        let sensitive_ids = &[
            SUBJECT_ID,
            OWNERSHIP_ID,
            ADDRESS_ID,
            BIRTH_NUMBER,
            "123456789",
            "987654321",
        ];
        assert_debug_without(&ownership, sensitive_ids);
        assert_debug_without(&ownership.target, sensitive_ids);
        assert_debug_without(&ownership.share, &["123456789", "987654321"]);
        let NtrOwnershipTarget::Subject(nested_owner) = &ownership.target else {
            unreachable!()
        };
        assert_debug_without(nested_owner, sensitive_ids);
        assert_debug_without(&nested_owner.primary, sensitive_ids);

        let uuid = NtrReportUuid::new(UUID).unwrap();
        assert_eq!(format!("{uuid:?}"), "NtrReportUuid([ID])");
        assert_debug_without(&uuid, &[UUID]);
        let run_id = NtrReportRunId::new(RUN_ID).unwrap();
        assert_eq!(format!("{run_id:?}"), "NtrReportRunId([ID])");
        assert_debug_without(&run_id, &[RUN_ID]);

        let unknown = NtrYesNo::Other(SENTINEL.to_owned());
        assert_eq!(format!("{unknown:?}"), "NtrYesNo([REDACTED])");
        assert_debug_without(&unknown, &[]);

        let report = ntr_report();
        assert_debug_without(&report, &[UUID, RUN_ID]);
        assert_debug_without(&report.general_data, &[]);
        assert_debug_without(&report.metadata, &[UUID, RUN_ID]);
        assert_debug_without(report.signature.as_ref().unwrap(), &[]);
    }

    #[test]
    fn report_wrappers_hide_payload_metadata_messages_and_original_bytes() {
        let wrapper_sentinels = &[UUID, RUN_ID, REPORT_ID, PARENT_REPORT_ID, CHILD_REPORT_ID];
        let xml = XmlReport::new(
            ntr_report(),
            report_metadata(),
            service_messages(),
            Bytes::from_static(SENTINEL.as_bytes()),
        );
        assert_debug_without(&xml, wrapper_sentinels);
        assert_debug_without(&xml.value, wrapper_sentinels);

        let zip = ZipReport::new(
            monthly_price_archive(),
            report_metadata(),
            service_messages(),
            Bytes::from_static(SENTINEL.as_bytes()),
        );
        assert_debug_without(&zip, wrapper_sentinels);
        assert_debug_without(&zip.value, wrapper_sentinels);
        assert_debug_without(&zip.value.entries[0], wrapper_sentinels);

        let binary = BinaryReport::from_downloaded(DownloadedReport {
            metadata: report_metadata(),
            messages: service_messages(),
            bytes: Bytes::from_static(SENTINEL.as_bytes()),
        });
        assert_debug_without(&binary, wrapper_sentinels);
    }
}
