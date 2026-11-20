use super::{
    AccountStatementReport, BuildingInfoReport, BuildingRightInfoReport, DefinitionPointsReport,
    MonthlyPriceArchive, MonthlyPriceArchiveLimits, OwnershipOverviewReport,
    OwnershipOverviewWithPropertiesReport, OwnershipSheetReport, ParcelInfoReport,
    PricesByProceedingReport, PricesByPropertiesReport, ProceedingInfoReport, RightsRecordReport,
    UnitInfoReport, XmlReport, ZipReport,
};
use crate::{DownloadedReport, Error, ReportStage, Result};

const DEFAULT_MAX_XML_NODES: u32 = 2_000_000;
const DEFAULT_MAX_XML_DEPTH: u32 = 256;

impl DownloadedReport {
    fn parse_xml<T>(
        &self,
        parser: impl FnOnce(&[u8], u32, u32) -> Result<T>,
    ) -> Result<XmlReport<T>> {
        let value = parser(&self.bytes, DEFAULT_MAX_XML_NODES, DEFAULT_MAX_XML_DEPTH)
            .map_err(|source| self.decode_failure(source))?;
        Ok(self.wrap_xml(value))
    }

    fn wrap_xml<T>(&self, value: T) -> XmlReport<T> {
        XmlReport::new(
            value,
            self.metadata.clone(),
            self.messages.clone(),
            self.bytes.clone(),
        )
    }

    fn wrap_zip<T>(&self, value: T) -> ZipReport<T> {
        ZipReport::new(
            value,
            self.metadata.clone(),
            self.messages.clone(),
            self.bytes.clone(),
        )
    }

    fn decode_failure(&self, source: Error) -> Error {
        Error::ReportOperation {
            id: self.metadata.id.clone(),
            stage: ReportStage::Decode,
            source: Box::new(source),
        }
    }

    /// Decodes legacy parcel information while retaining this downloaded report.
    pub fn parse_parcel_info(&self) -> Result<XmlReport<ParcelInfoReport>> {
        self.parse_xml(crate::wire::reports::legacy_property_info::parse_parcel_info)
    }

    /// Decodes legacy building information while retaining this downloaded report.
    pub fn parse_building_info(&self) -> Result<XmlReport<BuildingInfoReport>> {
        self.parse_xml(crate::wire::reports::legacy_property_info::parse_building_info)
    }

    /// Decodes legacy unit information while retaining this downloaded report.
    pub fn parse_unit_info(&self) -> Result<XmlReport<UnitInfoReport>> {
        self.parse_xml(crate::wire::reports::legacy_unit_reports::parse_unit_info)
    }

    /// Decodes legacy building-right information while retaining this downloaded report.
    pub fn parse_building_right_info(&self) -> Result<XmlReport<BuildingRightInfoReport>> {
        self.parse_xml(crate::wire::reports::legacy_unit_reports::parse_building_right_info)
    }

    /// Decodes legacy proceeding information while retaining this downloaded report.
    pub fn parse_proceeding_info(&self) -> Result<XmlReport<ProceedingInfoReport>> {
        self.parse_xml(crate::wire::reports::legacy_unit_reports::parse_proceeding_info)
    }

    /// Decodes a parcel definition-point list.
    pub fn parse_definition_points(&self) -> Result<XmlReport<DefinitionPointsReport>> {
        self.parse_xml(crate::wire::reports::definition_points::parse_definition_points)
    }

    /// Decodes legacy or NTR ownership-sheet XML according to its actual expanded root.
    pub fn parse_ownership_sheet(&self) -> Result<XmlReport<OwnershipSheetReport>> {
        self.parse_xml(crate::wire::reports::ownership_sheet::parse_ownership_sheet)
    }

    /// Decodes a legacy ownership overview.
    pub fn parse_ownership_overview(&self) -> Result<XmlReport<OwnershipOverviewReport>> {
        self.parse_xml(crate::wire::reports::legacy_ownership_overview::parse_ownership_overview)
    }

    /// Decodes a legacy ownership overview including property details.
    pub fn parse_ownership_overview_with_properties(
        &self,
    ) -> Result<XmlReport<OwnershipOverviewWithPropertiesReport>> {
        self.parse_xml(
            crate::wire::reports::legacy_ownership_overview::parse_ownership_overview_with_properties,
        )
    }

    /// Decodes a legacy rights record for a person.
    pub fn parse_rights_record(&self) -> Result<XmlReport<RightsRecordReport>> {
        self.parse_xml(crate::wire::reports::legacy_ownership_overview::parse_rights_record)
    }

    /// Decodes a customer account statement, including signed charge/credit amounts.
    pub fn parse_account_statement(&self) -> Result<XmlReport<AccountStatementReport>> {
        self.parse_xml(crate::wire::reports::account_statement::parse_account_statement)
    }

    /// Decodes legacy prices selected by properties.
    pub fn parse_prices_by_properties(&self) -> Result<XmlReport<PricesByPropertiesReport>> {
        let value = crate::wire::reports::prices::parse_prices_by_properties(&self.bytes)
            .map_err(|source| self.decode_failure(source))?;
        Ok(self.wrap_xml(value))
    }

    /// Decodes legacy prices selected by a proceeding.
    pub fn parse_prices_by_proceeding(&self) -> Result<XmlReport<PricesByProceedingReport>> {
        let value = crate::wire::reports::prices::parse_prices_by_proceeding(&self.bytes)
            .map_err(|source| self.decode_failure(source))?;
        Ok(self.wrap_xml(value))
    }

    /// Safely decodes a monthly price ZIP with the default expansion limits.
    pub fn parse_monthly_price_archive(&self) -> Result<ZipReport<MonthlyPriceArchive>> {
        let value = crate::wire::reports::prices::parse_monthly_price_archive(
            &self.bytes,
            MonthlyPriceArchiveLimits::default(),
        )
        .map_err(|source| self.decode_failure(source))?;
        Ok(self.wrap_zip(value))
    }

    /// Safely decodes a monthly price ZIP with explicit expansion limits.
    ///
    /// No entry is written to the filesystem. Entry count, per-entry and total
    /// expansion, compression ratio, and XML complexity are all bounded.
    pub fn parse_monthly_price_archive_with_limits(
        &self,
        limits: MonthlyPriceArchiveLimits,
    ) -> Result<ZipReport<MonthlyPriceArchive>> {
        let value = crate::wire::reports::prices::parse_monthly_price_archive_with_xml_limits(
            &self.bytes,
            limits,
            DEFAULT_MAX_XML_NODES,
            DEFAULT_MAX_XML_DEPTH,
        )
        .map_err(|source| self.decode_failure(source))?;
        Ok(self.wrap_zip(value))
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;

    use super::*;
    use crate::{ReportFormat, ReportId, ReportMetadata, ReportState, ReportVersion, XmlDateTime};

    fn downloaded(bytes: &'static [u8]) -> DownloadedReport {
        DownloadedReport {
            metadata: ReportMetadata {
                id: ReportId::new("123").unwrap(),
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
                format: ReportFormat::Xml,
                version: Some(ReportVersion::V1_0),
                electronic_seal: None,
                timestamp: None,
            },
            messages: Vec::new(),
            bytes: Bytes::from_static(bytes),
        }
    }

    #[test]
    fn decode_failure_retains_id_stage_and_original_download() {
        let report = downloaded(b"not XML; sensitive payload");
        let error = report.parse_parcel_info().unwrap_err();
        assert_eq!(error.report_id(), report.metadata.id().into());
        assert_eq!(error.report_stage(), Some(ReportStage::Decode));
        assert_eq!(report.bytes.as_ref(), b"not XML; sensitive payload");
        assert!(!error.to_string().contains("sensitive"));
    }

    #[test]
    fn typed_debug_never_contains_report_payload() {
        let report = downloaded(b"<Other>sensitive-value</Other>");
        let error = report.parse_definition_points().unwrap_err();
        assert!(!format!("{error:?}").contains("sensitive-value"));
    }
}
