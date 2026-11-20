//! Complete typed models for WSDP report payloads.

use std::fmt;

use bytes::Bytes;

use crate::{ReportMetadata, ReportVersion, ServiceMessage};

/// A typed XML report together with its original bytes and lifecycle metadata.
#[derive(Clone)]
pub struct XmlReport<T> {
    /// Fully decoded report value.
    pub value: T,
    /// Metadata for the downloaded report.
    pub metadata: ReportMetadata,
    /// All non-error messages returned during download.
    pub messages: Vec<ServiceMessage>,
    original_xml: Bytes,
}

impl<T> fmt::Debug for XmlReport<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("XmlReport")
            .field("message_count", &self.messages.len())
            .field("byte_len", &self.original_xml.len())
            .finish_non_exhaustive()
    }
}

impl<T> XmlReport<T> {
    pub(crate) fn new(
        value: T,
        metadata: ReportMetadata,
        messages: Vec<ServiceMessage>,
        original_xml: Bytes,
    ) -> Self {
        Self {
            value,
            metadata,
            messages,
            original_xml,
        }
    }

    /// Returns the untouched downloaded XML bytes.
    pub fn original_xml(&self) -> &Bytes {
        &self.original_xml
    }

    /// Consumes the wrapper and returns the original XML bytes.
    pub fn into_original_xml(self) -> Bytes {
        self.original_xml
    }
}

/// A typed ZIP report together with its original archive and lifecycle metadata.
#[derive(Clone)]
pub struct ZipReport<T> {
    /// Fully decoded archive value.
    pub value: T,
    /// Metadata for the downloaded report.
    pub metadata: ReportMetadata,
    /// All non-error messages returned during download.
    pub messages: Vec<ServiceMessage>,
    original_zip: Bytes,
}

impl<T> fmt::Debug for ZipReport<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ZipReport")
            .field("message_count", &self.messages.len())
            .field("byte_len", &self.original_zip.len())
            .finish_non_exhaustive()
    }
}

impl<T> ZipReport<T> {
    pub(crate) fn new(
        value: T,
        metadata: ReportMetadata,
        messages: Vec<ServiceMessage>,
        original_zip: Bytes,
    ) -> Self {
        Self {
            value,
            metadata,
            messages,
            original_zip,
        }
    }

    /// Returns the untouched downloaded ZIP bytes.
    pub fn original_zip(&self) -> &Bytes {
        &self.original_zip
    }

    /// Consumes the wrapper and returns the original ZIP bytes.
    pub fn into_original_zip(self) -> Bytes {
        self.original_zip
    }
}

/// Customer account-statement models.
pub mod account_statement;
/// Binary report wrapper.
mod binary;
/// Parcel definition-point models.
pub mod definition_points;
#[cfg(test)]
mod fixtures;
/// Shared legacy report models.
pub mod legacy_base;
/// Legacy ownership-overview and rights-record models.
pub mod legacy_ownership_overview;
/// Legacy ownership-sheet models.
pub mod legacy_ownership_sheet;
/// Legacy parcel and building information models.
pub mod legacy_property_info;
/// Legacy unit, building-right, and proceeding information models.
pub mod legacy_unit_reports;
/// Ownership-sheet format selection and unified report variants.
mod ownership_sheet;
/// Complete NTR 3.0 ownership-sheet models.
pub mod ownership_sheet_v3;
mod parsing;
/// Price-report and bounded monthly-archive models.
pub mod prices;
mod redacted;

pub use account_statement::{
    AccountChargeGroup, AccountChargeRow, AccountParticipant, AccountProceeding,
    AccountStatementReport, CustomerAccount,
};
pub use binary::BinaryReport;
pub use definition_points::{DefinitionPoint, DefinitionPointsReport};
pub use legacy_ownership_overview::{
    OwnershipOverviewReport, OwnershipOverviewWithPropertiesReport, RightsRecordReport,
};
pub use legacy_ownership_sheet::LegacyOwnershipSheetReport;
pub use legacy_property_info::{BuildingInfoReport, ParcelInfoReport};
pub use legacy_unit_reports::{BuildingRightInfoReport, ProceedingInfoReport, UnitInfoReport};
pub use ownership_sheet::{OwnershipSheetReport, OwnershipSheetVersion};
pub use ownership_sheet_v3::NtrOwnershipSheet;
pub use prices::{
    MonthlyPriceArchive, MonthlyPriceArchiveLimits, MonthlyPriceData, PricesByProceedingReport,
    PricesByPropertiesReport,
};

macro_rules! default_report_version {
    ($report:ty, $version:expr) => {
        impl $report {
            /// XML version pinned by the typed helper for this report family.
            pub const DEFAULT_VERSION: ReportVersion = $version;
        }
    };
}

default_report_version!(ParcelInfoReport, ReportVersion::V1_0);
default_report_version!(BuildingInfoReport, ReportVersion::V1_0);
default_report_version!(UnitInfoReport, ReportVersion::V1_0);
default_report_version!(BuildingRightInfoReport, ReportVersion::V1_0);
default_report_version!(ProceedingInfoReport, ReportVersion::V1_0);
default_report_version!(DefinitionPointsReport, ReportVersion::V1_0);
default_report_version!(OwnershipOverviewReport, ReportVersion::V1_0);
default_report_version!(OwnershipOverviewWithPropertiesReport, ReportVersion::V1_0);
default_report_version!(RightsRecordReport, ReportVersion::V1_0);
default_report_version!(AccountStatementReport, ReportVersion::V1_0);
default_report_version!(PricesByPropertiesReport, ReportVersion::V1_0);
default_report_version!(PricesByProceedingReport, ReportVersion::V1_0);
default_report_version!(OwnershipSheetReport, ReportVersion::V3_0);
default_report_version!(LegacyOwnershipSheetReport, ReportVersion::V1_0);
default_report_version!(NtrOwnershipSheet, ReportVersion::V3_0);
