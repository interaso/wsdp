#![cfg(test)]

use bytes::Bytes;

use super::OwnershipSheetReport;
use crate::{
    DownloadedReport, ReportFormat, ReportId, ReportMetadata, ReportState, ReportVersion,
    XmlDateTime,
};

fn downloaded(bytes: &'static [u8]) -> DownloadedReport {
    DownloadedReport {
        metadata: ReportMetadata {
            id: ReportId::new("123").unwrap(),
            name: "official anonymized fixture".to_owned(),
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

macro_rules! official_fixture {
    ($name:ident, $file:literal, $method:ident) => {
        #[test]
        fn $name() {
            let bytes = include_bytes!(concat!("../../../tests/fixtures/legacy/", $file));
            let downloaded = downloaded(bytes);
            let parsed = downloaded.$method().unwrap();
            assert_eq!(parsed.original_xml().as_ref(), bytes);
            assert_eq!(parsed.metadata.id.as_str(), "123");
        }
    };
}

official_fixture!(parcel_info, "parcel_info.xml", parse_parcel_info);
official_fixture!(building_info, "building_info.xml", parse_building_info);
official_fixture!(unit_info, "unit_info.xml", parse_unit_info);
official_fixture!(
    building_right_info,
    "building_right_info.xml",
    parse_building_right_info
);
official_fixture!(
    ownership_overview,
    "ownership_overview.xml",
    parse_ownership_overview
);
official_fixture!(
    ownership_overview_with_properties,
    "ownership_overview_with_properties.xml",
    parse_ownership_overview_with_properties
);
official_fixture!(rights_record, "rights_record.xml", parse_rights_record);
official_fixture!(
    prices_by_properties,
    "prices_by_properties.xml",
    parse_prices_by_properties
);

#[test]
fn legacy_ownership_sheet_uses_actual_root_and_preserves_xml() {
    let bytes = include_bytes!("../../../tests/fixtures/legacy/ownership_sheet.xml");
    let downloaded = downloaded(bytes);
    let parsed = downloaded.parse_ownership_sheet().unwrap();
    assert!(matches!(parsed.value, OwnershipSheetReport::Legacy(_)));
    assert_eq!(parsed.original_xml().as_ref(), bytes);
}
