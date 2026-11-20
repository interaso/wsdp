use super::Client;
use crate::error::{Error, ReportStage, Result};
use crate::model::reports::{
    AccountStatementReport, BinaryReport, BuildingInfoReport, BuildingRightInfoReport,
    DefinitionPointsReport, MonthlyPriceArchive, MonthlyPriceArchiveLimits,
    OwnershipOverviewReport, OwnershipOverviewWithPropertiesReport, OwnershipSheetReport,
    ParcelInfoReport, PricesByProceedingReport, PricesByPropertiesReport, ProceedingInfoReport,
    RightsRecordReport, UnitInfoReport, XmlReport, ZipReport,
};
use crate::model::{
    AccountStatementRequest, BuildingInfoRequest, BuildingRightInfoRequest, CadastralMapRequest,
    ChargeApproval, CollectionDocumentRequest, DefinitionPointsRequest, GenerateReport,
    GeometricPlanRequest, OutputFormat, OwnershipOverviewRequest, OwnershipSheetRequest,
    ParcelInfoRequest, PricesByCadastralAreaRequest, PricesByProceedingRequest,
    PricesByPropertiesRequest, ProceedingInfoRequest, ReportFamily, ReportFormat, ReportId,
    ReportOptions, ReportVersion, ReportVersionSelection, RightsRecordRequest, UnitInfoRequest,
    WaitOptions,
};
use crate::wire::reports as wire;

const LEGACY_VERSIONS: &[ReportVersion] = &[ReportVersion::V1_0];
const OWNERSHIP_SHEET_VERSIONS: &[ReportVersion] = &[ReportVersion::V1_0, ReportVersion::V3_0];

impl Client {
    /// Generates, waits for, preflights, downloads, and decodes one typed XML report.
    async fn typed_xml<T>(
        &self,
        request: GenerateReport,
        wait: WaitOptions,
        approval: ChargeApproval,
        parser: impl FnOnce(&[u8], u32, u32) -> Result<T>,
    ) -> Result<XmlReport<T>> {
        let downloaded = self.execute_single_report(request, wait, approval).await?;
        let id = downloaded.id().clone();
        if downloaded.metadata.format != ReportFormat::Xml {
            return Err(decode_failure(
                &id,
                Error::invalid_response("typed XML workflow received a non-XML report"),
            ));
        }
        let value = parser(
            &downloaded.bytes,
            self.inner.xml_limits.max_nodes,
            self.inner.xml_limits.max_depth,
        )
        .map_err(|source| decode_failure(&id, source))?;
        Ok(XmlReport::new(
            value,
            downloaded.metadata,
            downloaded.messages,
            downloaded.bytes,
        ))
    }

    pub(crate) async fn execute_single_report(
        &self,
        request: GenerateReport,
        wait: WaitOptions,
        approval: ChargeApproval,
    ) -> Result<crate::DownloadedReport> {
        validate_wait(wait)?;
        let batch = self.generate_report(request).await?;
        let metadata = batch.value.into_single_report()?;
        let id = metadata.id().clone();
        self.wait_for_report(&id, wait).await?;
        self.download_report(&id, approval).await
    }

    /// Generates and decodes a legacy ownership sheet or NTR 3.0 ownership sheet.
    ///
    /// The helper performs one generation, bounded status polling, one fresh price
    /// preflight, and one explicitly approved download. PDF, attached XML,
    /// `LatestAvailable`, and unsupported pinned versions fail before generation.
    pub async fn ownership_sheet_report(
        &self,
        request: OwnershipSheetRequest,
        wait: WaitOptions,
        approval: ChargeApproval,
    ) -> Result<XmlReport<OwnershipSheetReport>> {
        validate_xml_options(
            &request.options,
            &ReportFamily::OwnershipSheet,
            OWNERSHIP_SHEET_VERSIONS,
        )?;
        self.typed_xml(
            GenerateReport::OwnershipSheet(request),
            wait,
            approval,
            wire::ownership_sheet::parse_ownership_sheet,
        )
        .await
    }

    /// Generates and decodes an XML account statement with explicit charge approval.
    ///
    /// The same single-report generation/wait/preflight/download sequence applies.
    /// PDF, `LatestAvailable`, and unsupported pinned versions fail before generation.
    pub async fn account_statement_report(
        &self,
        request: AccountStatementRequest,
        wait: WaitOptions,
        approval: ChargeApproval,
    ) -> Result<XmlReport<AccountStatementReport>> {
        if request.format != OutputFormat::Xml {
            return Err(Error::invalid_input(
                "format",
                "the typed account-statement helper requires XML output",
            ));
        }
        validate_version(
            &request.version,
            &ReportFamily::AccountStatement,
            LEGACY_VERSIONS,
        )?;
        self.typed_xml(
            GenerateReport::AccountStatement(request),
            wait,
            approval,
            wire::account_statement::parse_account_statement,
        )
        .await
    }

    /// Generates, safely expands, and decodes a monthly price ZIP report.
    ///
    /// One generation, bounded status polling, one price preflight, and one approved
    /// download are performed. Default limits bound entry count, entry/total expanded
    /// bytes, compression ratio, and per-entry XML complexity. Nothing is extracted
    /// to the filesystem.
    pub async fn prices_by_cadastral_area_report(
        &self,
        request: PricesByCadastralAreaRequest,
        wait: WaitOptions,
        approval: ChargeApproval,
    ) -> Result<ZipReport<MonthlyPriceArchive>> {
        let downloaded = self
            .execute_single_report(
                GenerateReport::PricesByCadastralArea(request),
                wait,
                approval,
            )
            .await?;
        let id = downloaded.id().clone();
        if downloaded.metadata.format != ReportFormat::Zip {
            return Err(decode_failure(
                &id,
                Error::invalid_response("monthly price workflow received a non-ZIP report"),
            ));
        }
        let value = wire::prices::parse_monthly_price_archive_with_xml_limits(
            &downloaded.bytes,
            MonthlyPriceArchiveLimits::default(),
            self.inner.xml_limits.max_nodes,
            self.inner.xml_limits.max_depth,
        )
        .map_err(|source| decode_failure(&id, source))?;
        Ok(ZipReport::new(
            value,
            downloaded.metadata,
            downloaded.messages,
            downloaded.bytes,
        ))
    }

    /// Generates and downloads a fixed-format cadastral-map PDF.
    ///
    /// One generation, bounded status polling, one price preflight, and one approved
    /// download are performed. Trial map availability is limited by ČÚZK.
    pub async fn cadastral_map_report(
        &self,
        request: CadastralMapRequest,
        wait: WaitOptions,
        approval: ChargeApproval,
    ) -> Result<BinaryReport> {
        let downloaded = self
            .execute_single_report(GenerateReport::CadastralMap(request), wait, approval)
            .await?;
        binary_report(downloaded)
    }

    /// Generates and downloads a fixed-format collection-document PDF.
    ///
    /// One generation, bounded status polling, one price preflight, and one approved
    /// download are performed. Trial deed ordering is limited by ČÚZK.
    pub async fn collection_document_report(
        &self,
        request: CollectionDocumentRequest,
        wait: WaitOptions,
        approval: ChargeApproval,
    ) -> Result<BinaryReport> {
        let downloaded = self
            .execute_single_report(GenerateReport::CollectionDocument(request), wait, approval)
            .await?;
        binary_report(downloaded)
    }

    /// Generates and downloads a fixed-format geometric-plan PDF.
    ///
    /// One generation, bounded status polling, one price preflight, and one approved
    /// download are performed. No XML parser or electronic-seal verification is implied.
    pub async fn geometric_plan_report(
        &self,
        request: GeometricPlanRequest,
        wait: WaitOptions,
        approval: ChargeApproval,
    ) -> Result<BinaryReport> {
        let downloaded = self
            .execute_single_report(GenerateReport::GeometricPlan(request), wait, approval)
            .await?;
        binary_report(downloaded)
    }
}

fn binary_report(downloaded: crate::DownloadedReport) -> Result<BinaryReport> {
    if downloaded.metadata.format != ReportFormat::Pdf {
        return Err(decode_failure(
            downloaded.id(),
            Error::invalid_response("binary workflow received a non-PDF report"),
        ));
    }
    if !downloaded.bytes.starts_with(b"%PDF-") {
        return Err(decode_failure(
            downloaded.id(),
            Error::invalid_response("binary workflow payload lacks the PDF signature"),
        ));
    }
    Ok(BinaryReport::from_downloaded(downloaded))
}

// Keep the family expression and request variant explicit without duplicating the
// lifecycle implementation in thirteen public methods.
macro_rules! legacy_family_xml_helper {
    (
        $(#[$meta:meta])*
        $name:ident,
        $request:ty,
        $variant:ident,
        $family:expr,
        $output:ty,
        $parser:path
    ) => {
        $(#[$meta])*
        ///
        /// The helper performs one generation, bounded status polling, one fresh
        /// price preflight, and one explicitly approved download. PDF, attached XML,
        /// `LatestAvailable`, and unsupported pinned versions fail before generation.
        pub async fn $name(
            &self,
            request: $request,
            wait: WaitOptions,
            approval: ChargeApproval,
        ) -> Result<XmlReport<$output>> {
            validate_xml_options(&request.options, &$family, LEGACY_VERSIONS)?;
            self.typed_xml(
                GenerateReport::$variant(request),
                wait,
                approval,
                $parser,
            )
            .await
        }
    };
}

impl Client {
    legacy_family_xml_helper!(
        /// Generates and decodes parcel information.
        parcel_info_report,
        ParcelInfoRequest,
        ParcelInfo,
        ReportFamily::ParcelInfo,
        ParcelInfoReport,
        wire::legacy_property_info::parse_parcel_info
    );
    legacy_family_xml_helper!(
        /// Generates and decodes building information.
        building_info_report,
        BuildingInfoRequest,
        BuildingInfo,
        ReportFamily::BuildingInfo,
        BuildingInfoReport,
        wire::legacy_property_info::parse_building_info
    );
    legacy_family_xml_helper!(
        /// Generates and decodes unit information.
        unit_info_report,
        UnitInfoRequest,
        UnitInfo,
        ReportFamily::UnitInfo,
        UnitInfoReport,
        wire::legacy_unit_reports::parse_unit_info
    );
    legacy_family_xml_helper!(
        /// Generates and decodes building-right information.
        building_right_info_report,
        BuildingRightInfoRequest,
        BuildingRightInfo,
        ReportFamily::BuildingRightInfo,
        BuildingRightInfoReport,
        wire::legacy_unit_reports::parse_building_right_info
    );
    legacy_family_xml_helper!(
        /// Generates and decodes proceeding information.
        proceeding_info_report,
        ProceedingInfoRequest,
        ProceedingInfo,
        ReportFamily::ProceedingInfo,
        ProceedingInfoReport,
        wire::legacy_unit_reports::parse_proceeding_info
    );
    legacy_family_xml_helper!(
        /// Generates and decodes a parcel definition-point list.
        definition_points_report,
        DefinitionPointsRequest,
        DefinitionPoints,
        ReportFamily::DefinitionPoints,
        DefinitionPointsReport,
        wire::definition_points::parse_definition_points
    );
    legacy_family_xml_helper!(
        /// Generates and decodes an ownership overview.
        ownership_overview_report,
        OwnershipOverviewRequest,
        OwnershipOverview,
        ReportFamily::OwnershipOverview,
        OwnershipOverviewReport,
        wire::legacy_ownership_overview::parse_ownership_overview
    );
    legacy_family_xml_helper!(
        /// Generates and decodes an ownership overview with property details.
        ownership_overview_with_properties_report,
        OwnershipOverviewRequest,
        OwnershipOverviewWithProperties,
        ReportFamily::OwnershipOverviewWithProperties,
        OwnershipOverviewWithPropertiesReport,
        wire::legacy_ownership_overview::parse_ownership_overview_with_properties
    );
    legacy_family_xml_helper!(
        /// Generates and decodes a rights record for a person.
        rights_record_report,
        RightsRecordRequest,
        RightsRecord,
        ReportFamily::RightsRecord,
        RightsRecordReport,
        wire::legacy_ownership_overview::parse_rights_record
    );
    legacy_family_xml_helper!(
        /// Generates and decodes prices selected by properties.
        prices_by_properties_report,
        PricesByPropertiesRequest,
        PricesByProperties,
        ReportFamily::PricesByProperties,
        PricesByPropertiesReport,
        wire::prices::parse_prices_by_properties_with_limits
    );
    legacy_family_xml_helper!(
        /// Generates and decodes prices selected by a proceeding.
        prices_by_proceeding_report,
        PricesByProceedingRequest,
        PricesByProceeding,
        ReportFamily::PricesByProceeding,
        PricesByProceedingReport,
        wire::prices::parse_prices_by_proceeding_with_limits
    );
}

fn validate_xml_options(
    options: &ReportOptions,
    family: &ReportFamily,
    supported: &[ReportVersion],
) -> Result<()> {
    if options.format != OutputFormat::Xml {
        return Err(Error::invalid_input(
            "format",
            "typed XML helpers require XML output",
        ));
    }
    if options.attach_xml {
        return Err(Error::invalid_input(
            "attach_xml",
            "typed XML helpers accept only a single XML output",
        ));
    }
    validate_version(&options.version, family, supported)
}

fn validate_version(
    selection: &ReportVersionSelection,
    family: &ReportFamily,
    supported: &[ReportVersion],
) -> Result<()> {
    let selected = match selection {
        ReportVersionSelection::DefaultForFamily => family.default_version().ok_or_else(|| {
            Error::invalid_input("version", "no typed default exists for this report family")
        })?,
        ReportVersionSelection::Pinned(version) => version.clone(),
        ReportVersionSelection::LatestAvailable => {
            return Err(Error::invalid_input(
                "version",
                "typed helpers require a supported pinned version, not LatestAvailable",
            ));
        }
    };
    if supported.contains(&selected) {
        Ok(())
    } else {
        Err(Error::invalid_input(
            "version",
            "the selected report version has no typed parser; use the raw lifecycle",
        ))
    }
}

fn validate_wait(options: WaitOptions) -> Result<()> {
    options.validate()?;
    if options.max_wait.is_zero() {
        return Err(Error::invalid_input(
            "max_wait",
            "maximum wait must be greater than zero",
        ));
    }
    Ok(())
}

fn decode_failure(id: &ReportId, source: Error) -> Error {
    Error::ReportOperation {
        id: id.clone(),
        stage: ReportStage::Decode,
        source: Box::new(source),
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;

    use super::*;
    use crate::{ReportMetadata, ReportState, XmlDateTime};

    #[test]
    fn binary_helper_rejects_a_missing_pdf_signature_with_recovery_context() {
        let id = ReportId::new("123").unwrap();
        let downloaded = crate::DownloadedReport {
            metadata: ReportMetadata {
                id: id.clone(),
                name: "test".to_owned(),
                units: None,
                pages: None,
                price: None,
                requested_at: XmlDateTime::new("2026-11-20T10:00:00Z").unwrap(),
                started_at: None,
                created_at: None,
                parent_id: None,
                child_ids: Vec::new(),
                state: ReportState::Charged,
                format: ReportFormat::Pdf,
                version: None,
                electronic_seal: None,
                timestamp: None,
            },
            messages: Vec::new(),
            bytes: Bytes::from_static(b"not a PDF"),
        };
        let error = binary_report(downloaded).unwrap_err();
        assert_eq!(error.report_id(), Some(&id));
        assert_eq!(error.report_stage(), Some(ReportStage::Decode));
    }
}
