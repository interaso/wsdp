//! Full opt-in report matrix for the official shared trial environment.
//!
//! No report IDs, personal values, or payload contents are printed. Every output
//! receives its own charge approval, and every ID created by this test is cleaned
//! up explicitly, including multi-output PDF+XML and multi-LV responses.

use std::collections::HashSet;
use std::time::Duration;

use wsdp::reports::OwnershipSheetReport;
use wsdp::reports::ownership_sheet_v3::NtrOwnershipTarget;
use wsdp::{
    AccountContext, AccountStatementRequest, BuildingId, BuildingInfoRequest, BuildingRightId,
    BuildingRightInfoRequest, BuildingSelector, CadastralAreaCode, CadastralMapRequest,
    CadastralMapSelector, ChargeApproval, Client, CollectionDocumentRequest,
    CollectionDocumentSearch, DefinitionPointsRequest, DefinitionPointsSelector, DocumentId,
    DownloadedReport, GenerateReport, GeometricPlanRequest, GeometricPlanSearch, MapPaperFormat,
    MapScale, OwnershipOverviewRequest, OwnershipSheetNumber, OwnershipSheetRequest,
    OwnershipSheetSelector, ParcelId, ParcelInfoRequest, ParcelSelector,
    PricesByCadastralAreaRequest, PricesByProceedingRequest, PricesByPropertiesRequest,
    ProceedingId, ProceedingInfoRequest, ProceedingSearchSelector, ProceedingSelector,
    ReportFormat, ReportObjectList, ReportOptions, ReportVersion, RightsRecordIdentifier,
    RightsRecordRequest, RightsRecordSubjectKind, SubjectId, UnitId, UnitInfoRequest, UnitSelector,
    WaitOptions, XmlDateTime,
};

#[derive(Clone, Copy)]
enum Decode {
    ParcelXml,
    ParcelPdfWithXml,
    BuildingXml,
    UnitXml,
    BuildingRightXml,
    ProceedingXml,
    DefinitionPointsXml,
    OwnershipOverviewXml,
    OwnershipOverviewWithPropertiesXml,
    OwnershipOverviewWithPropertiesPdfWithXml,
    RightsRecordXml,
    RightsRecordPdfWithXml,
    AccountStatementXml,
    PricesByPropertiesXml,
    PricesByProceedingXml,
    OwnershipSheetNtrXml,
    OwnershipSheetLegacyXml,
    OwnershipSheetPdfWithXml,
    MonthlyPriceZip,
    Pdf,
}

#[derive(Default)]
struct LvDiscovery {
    id: Option<wsdp::OwnershipSheetId>,
    subject_id: Option<SubjectId>,
    parcel_id: Option<ParcelId>,
}

fn xml() -> ReportOptions {
    ReportOptions::xml_default(AccountContext::Paying)
}

fn pdf() -> ReportOptions {
    ReportOptions::pdf(AccountContext::Paying)
}

fn ownership_sheet(selector: OwnershipSheetSelector, options: ReportOptions) -> GenerateReport {
    GenerateReport::OwnershipSheet(OwnershipSheetRequest::new(selector, options))
}

fn lv_number() -> OwnershipSheetSelector {
    OwnershipSheetSelector::Number {
        cadastral_area: CadastralAreaCode::new("693936").unwrap(),
        number: OwnershipSheetNumber::new(318).unwrap(),
    }
}

fn account_request(format: wsdp::OutputFormat) -> GenerateReport {
    let to = jiff::Timestamp::now();
    let from = to
        .checked_sub(jiff::Span::new().hours(24))
        .expect("valid account-statement interval");
    let mut request =
        AccountStatementRequest::new(XmlDateTime::new(from.to_string()).unwrap(), format);
    request.to = Some(XmlDateTime::new(to.to_string()).unwrap());
    GenerateReport::AccountStatement(request)
}

fn monthly_price_request() -> GenerateReport {
    GenerateReport::PricesByCadastralArea(PricesByCadastralAreaRequest::new(
        CadastralAreaCode::new("732630").unwrap(),
        2020,
        9,
        12,
        AccountContext::Paying,
    ))
}

fn parcel_pdf_with_xml_request() -> GenerateReport {
    GenerateReport::ParcelInfo(ParcelInfoRequest::new(
        ParcelSelector::Id(ParcelId::new("2928347306").unwrap()),
        ReportOptions::pdf_with_xml(AccountContext::Paying),
    ))
}

fn trial_wait() -> WaitOptions {
    WaitOptions {
        max_wait: Duration::from_secs(300),
        ..WaitOptions::default()
    }
}

fn require_format(report: &DownloadedReport, expected: ReportFormat) -> Result<(), String> {
    if report.metadata.format == expected {
        Ok(())
    } else {
        Err("server returned an unexpected report format".to_owned())
    }
}

fn discover_lv(report: &wsdp::reports::NtrOwnershipSheet, discovery: &mut LvDiscovery) {
    for sheet in &report.ownership_sheets {
        if discovery.id.is_none() {
            discovery.id = Some(sheet.id.clone());
        }
        for parcel in sheet
            .properties
            .parcels
            .iter()
            .flatten()
            .chain(sheet.properties.simplified_parcels.iter().flatten())
        {
            if discovery.parcel_id.is_none() {
                discovery.parcel_id = Some(parcel.id.clone());
            }
        }
        for relation in sheet.owner_relations.iter().flatten() {
            for record in &relation.ownership_records {
                if let NtrOwnershipTarget::Subject(subject) = &record.ownership.target
                    && discovery.subject_id.is_none()
                {
                    discovery.subject_id = Some(subject.id.clone());
                }
            }
        }
    }
}

fn decode_report(
    downloaded: &DownloadedReport,
    decode: Decode,
    discovery: &mut LvDiscovery,
) -> Result<(), String> {
    if downloaded.bytes.is_empty() {
        return Err("server returned an empty report payload".to_owned());
    }
    match decode {
        Decode::ParcelXml => {
            require_format(downloaded, ReportFormat::Xml)?;
            downloaded
                .parse_parcel_info()
                .map_err(|error| error.to_string())?;
        }
        Decode::ParcelPdfWithXml => match &downloaded.metadata.format {
            ReportFormat::Pdf => require_pdf(downloaded)?,
            ReportFormat::Xml => {
                downloaded
                    .parse_parcel_info()
                    .map_err(|error| error.to_string())?;
            }
            _ => return Err("PDF+XML bundle returned an unexpected format".to_owned()),
        },
        Decode::BuildingXml => {
            require_format(downloaded, ReportFormat::Xml)?;
            downloaded
                .parse_building_info()
                .map_err(|error| error.to_string())?;
        }
        Decode::UnitXml => {
            require_format(downloaded, ReportFormat::Xml)?;
            downloaded
                .parse_unit_info()
                .map_err(|error| error.to_string())?;
        }
        Decode::BuildingRightXml => {
            require_format(downloaded, ReportFormat::Xml)?;
            downloaded
                .parse_building_right_info()
                .map_err(|error| error.to_string())?;
        }
        Decode::ProceedingXml => {
            require_format(downloaded, ReportFormat::Xml)?;
            downloaded
                .parse_proceeding_info()
                .map_err(|error| error.to_string())?;
        }
        Decode::DefinitionPointsXml => {
            require_format(downloaded, ReportFormat::Xml)?;
            downloaded
                .parse_definition_points()
                .map_err(|error| error.to_string())?;
        }
        Decode::OwnershipOverviewXml => {
            require_format(downloaded, ReportFormat::Xml)?;
            downloaded
                .parse_ownership_overview()
                .map_err(|error| error.to_string())?;
        }
        Decode::OwnershipOverviewWithPropertiesXml => {
            require_format(downloaded, ReportFormat::Xml)?;
            downloaded
                .parse_ownership_overview_with_properties()
                .map_err(|error| error.to_string())?;
        }
        Decode::OwnershipOverviewWithPropertiesPdfWithXml => match &downloaded.metadata.format {
            ReportFormat::Pdf => require_pdf(downloaded)?,
            ReportFormat::Xml => {
                downloaded
                    .parse_ownership_overview_with_properties()
                    .map_err(|error| error.to_string())?;
            }
            _ => return Err("overview PDF+XML returned an unexpected format".to_owned()),
        },
        Decode::RightsRecordXml => {
            require_format(downloaded, ReportFormat::Xml)?;
            downloaded
                .parse_rights_record()
                .map_err(|error| error.to_string())?;
        }
        Decode::RightsRecordPdfWithXml => match &downloaded.metadata.format {
            ReportFormat::Pdf => require_pdf(downloaded)?,
            ReportFormat::Xml => {
                downloaded
                    .parse_rights_record()
                    .map_err(|error| error.to_string())?;
            }
            _ => return Err("rights-record PDF+XML returned an unexpected format".to_owned()),
        },
        Decode::AccountStatementXml => {
            require_format(downloaded, ReportFormat::Xml)?;
            downloaded
                .parse_account_statement()
                .map_err(|error| error.to_string())?;
        }
        Decode::PricesByPropertiesXml => {
            require_format(downloaded, ReportFormat::Xml)?;
            downloaded
                .parse_prices_by_properties()
                .map_err(|error| error.to_string())?;
        }
        Decode::PricesByProceedingXml => {
            require_format(downloaded, ReportFormat::Xml)?;
            downloaded
                .parse_prices_by_proceeding()
                .map_err(|error| error.to_string())?;
        }
        Decode::OwnershipSheetNtrXml => {
            require_format(downloaded, ReportFormat::Xml)?;
            let parsed = downloaded
                .parse_ownership_sheet()
                .map_err(|error| error.to_string())?;
            let OwnershipSheetReport::Ntr3(report) = &parsed.value else {
                return Err("NTR request returned legacy LV XML".to_owned());
            };
            discover_lv(report, discovery);
        }
        Decode::OwnershipSheetLegacyXml => {
            require_format(downloaded, ReportFormat::Xml)?;
            let parsed = downloaded
                .parse_ownership_sheet()
                .map_err(|error| error.to_string())?;
            if !matches!(parsed.value, OwnershipSheetReport::Legacy(_)) {
                return Err("legacy LV 1.0 request returned NTR XML".to_owned());
            }
        }
        Decode::OwnershipSheetPdfWithXml => match &downloaded.metadata.format {
            ReportFormat::Pdf => require_pdf(downloaded)?,
            ReportFormat::Xml => {
                let parsed = downloaded
                    .parse_ownership_sheet()
                    .map_err(|error| error.to_string())?;
                if let OwnershipSheetReport::Ntr3(report) = &parsed.value {
                    discover_lv(report, discovery);
                }
            }
            _ => return Err("LV PDF+XML bundle returned an unexpected format".to_owned()),
        },
        Decode::MonthlyPriceZip => {
            require_format(downloaded, ReportFormat::Zip)?;
            downloaded
                .parse_monthly_price_archive()
                .map_err(|error| error.to_string())?;
        }
        Decode::Pdf => {
            require_format(downloaded, ReportFormat::Pdf)?;
            require_pdf(downloaded)?;
        }
    }
    Ok(())
}

fn require_pdf(downloaded: &DownloadedReport) -> Result<(), String> {
    if downloaded.bytes.starts_with(b"%PDF-") {
        Ok(())
    } else {
        Err("PDF report lacks the PDF signature".to_owned())
    }
}

async fn process_batch(
    client: &Client,
    batch: wsdp::ReportBatch,
    decode: Decode,
    discovery: &mut LvDiscovery,
) -> Result<usize, String> {
    let mut ids = Vec::new();
    let mut seen = HashSet::new();
    for report in batch.reports {
        if seen.insert(report.id.clone()) {
            ids.push(report.id);
        }
        for child in report.child_ids {
            if seen.insert(child.clone()) {
                ids.push(child);
            }
        }
    }
    if ids.is_empty() {
        return Err("generation returned no report IDs".to_owned());
    }

    let requires_pdf_xml = matches!(
        decode,
        Decode::ParcelPdfWithXml
            | Decode::OwnershipSheetPdfWithXml
            | Decode::OwnershipOverviewWithPropertiesPdfWithXml
            | Decode::RightsRecordPdfWithXml
    );
    let mut saw_pdf = false;
    let mut saw_xml = false;
    let mut failure = None;
    let mut index = 0;
    while index < ids.len() {
        let id = ids[index].clone();
        index += 1;
        let outcome = async {
            let ready = client
                .wait_for_report(&id, trial_wait())
                .await
                .map_err(|error| error.to_string())?;
            for child in &ready.value.child_ids {
                if seen.insert(child.clone()) {
                    ids.push(child.clone());
                }
            }
            let downloaded = client
                .download_report(&id, ChargeApproval::Any)
                .await
                .map_err(|error| error.to_string())?;
            if let Some(parent) = &downloaded.metadata.parent_id
                && seen.insert(parent.clone())
            {
                ids.push(parent.clone());
            }
            for child in &downloaded.metadata.child_ids {
                if seen.insert(child.clone()) {
                    ids.push(child.clone());
                }
            }
            decode_report(&downloaded, decode, discovery)?;
            match downloaded.metadata.format {
                ReportFormat::Pdf => saw_pdf = true,
                ReportFormat::Xml => saw_xml = true,
                _ => {}
            }
            Ok(())
        }
        .await;
        if let Err(error) = outcome {
            failure = Some(error);
            break;
        }
    }
    if failure.is_none() && requires_pdf_xml && !(saw_pdf && saw_xml) {
        failure = Some(format!(
            "PDF+XML response did not expose both formats (PDF: {saw_pdf}, XML: {saw_xml})"
        ));
    }

    // Cleanup follows all processing so removing a parent cannot make a newly
    // discovered child unavailable. Only IDs returned for this request are used.
    for id in &ids {
        let cleanup = client.delete_report(id).await;
        if failure.is_none()
            && let Err(error) = cleanup
        {
            failure = Some(format!("cleanup failed: {error}"));
        }
    }
    match failure {
        Some(error) => Err(error),
        None => Ok(ids.len()),
    }
}

async fn run_report(
    client: &Client,
    request: GenerateReport,
    decode: Decode,
    allow_service_rejection: bool,
    discovery: &mut LvDiscovery,
) -> Result<usize, String> {
    match client.generate_report(request).await {
        Ok(response) => process_batch(client, response.value, decode, discovery).await,
        Err(wsdp::Error::Service(_)) if allow_service_rejection => Ok(0),
        Err(error) => Err(error.to_string()),
    }
}

async fn run_configurable_attachment(
    client: &Client,
    request: GenerateReport,
    decode: Decode,
    unsupported_codes: &[&str],
    discovery: &mut LvDiscovery,
) -> Result<usize, String> {
    match client.generate_report(request).await {
        Ok(response) => process_batch(client, response.value, decode, discovery).await,
        Err(wsdp::Error::Service(error))
            if error
                .code()
                .is_some_and(|code| unsupported_codes.contains(&code)) =>
        {
            Ok(0)
        }
        Err(error) => Err(error.to_string()),
    }
}

#[tokio::test]
#[ignore = "runs the complete report matrix against the shared official WSDP trial"]
async fn all_report_types_and_output_formats_with_cleanup() {
    let Some(client) = super::trial_client() else {
        panic!("set WSDP_RUN_TRIAL_TESTS=1 and WSDP_TRIAL_USERNAME/PASSWORD");
    };
    let mut discovery = LvDiscovery::default();

    let initial_outputs = run_report(
        &client,
        ownership_sheet(lv_number(), xml()),
        Decode::OwnershipSheetNtrXml,
        false,
        &mut discovery,
    )
    .await
    .expect("NTR LV by number must provide selectors for the remaining LV operations");
    assert!(initial_outputs > 0);
    let lv_id = discovery.id.clone().expect("NTR report contains an LV ID");
    let subject_id = discovery
        .subject_id
        .clone()
        .expect("NTR report contains an owner subject ID");
    let lv_parcel_id = discovery
        .parcel_id
        .clone()
        .expect("NTR report contains a parcel ID");

    let plans = client
        .find_geometric_plans(
            GeometricPlanSearch::new(CadastralAreaCode::new("693936").unwrap(), 307).unwrap(),
        )
        .await
        .expect("published trial GP search");
    let plan = plans
        .value
        .items
        .first()
        .expect("published trial GP")
        .clone();
    let proceeding_id: ProceedingId = plan.proceeding.id.clone();
    let price_proceeding = ProceedingSelector::Id(ProceedingId::new("93871870010").unwrap());
    let price_properties = ReportObjectList::Parcels(vec![ParcelId::new("2357731101").unwrap()]);
    let subject_overview =
        || OwnershipOverviewRequest::new(SubjectId::new("627990601").unwrap(), xml());
    let rights = |options| {
        GenerateReport::RightsRecord(RightsRecordRequest::new(
            RightsRecordSubjectKind::LegalPerson,
            "AG družstvo Kružberk",
            RightsRecordIdentifier::new("22829812").unwrap(),
            options,
        ))
    };

    let requests = vec![
        (
            "parcel XML",
            GenerateReport::ParcelInfo(ParcelInfoRequest::new(
                ParcelSelector::Id(ParcelId::new("2928347306").unwrap()),
                xml(),
            )),
            Decode::ParcelXml,
        ),
        (
            "parcel PDF",
            GenerateReport::ParcelInfo(ParcelInfoRequest::new(
                ParcelSelector::Id(ParcelId::new("2928347306").unwrap()),
                pdf(),
            )),
            Decode::Pdf,
        ),
        (
            "building XML",
            GenerateReport::BuildingInfo(BuildingInfoRequest::new(
                BuildingSelector::Id(BuildingId::new("286869306").unwrap()),
                xml(),
            )),
            Decode::BuildingXml,
        ),
        (
            "building PDF",
            GenerateReport::BuildingInfo(BuildingInfoRequest::new(
                BuildingSelector::Id(BuildingId::new("286869306").unwrap()),
                pdf(),
            )),
            Decode::Pdf,
        ),
        (
            "unit XML",
            GenerateReport::UnitInfo(UnitInfoRequest::new(
                UnitSelector::Id(UnitId::new("18719306").unwrap()),
                xml(),
            )),
            Decode::UnitXml,
        ),
        (
            "unit PDF",
            GenerateReport::UnitInfo(UnitInfoRequest::new(
                UnitSelector::Id(UnitId::new("18719306").unwrap()),
                pdf(),
            )),
            Decode::Pdf,
        ),
        (
            "building-right XML",
            GenerateReport::BuildingRightInfo(BuildingRightInfoRequest::new(
                BuildingRightId::new("5270").unwrap(),
                xml(),
            )),
            Decode::BuildingRightXml,
        ),
        (
            "building-right PDF",
            GenerateReport::BuildingRightInfo(BuildingRightInfoRequest::new(
                BuildingRightId::new("5270").unwrap(),
                pdf(),
            )),
            Decode::Pdf,
        ),
        (
            "proceeding XML",
            GenerateReport::ProceedingInfo(ProceedingInfoRequest::new(
                ProceedingSelector::Id(proceeding_id.clone()),
                xml(),
            )),
            Decode::ProceedingXml,
        ),
        (
            "proceeding PDF",
            GenerateReport::ProceedingInfo(ProceedingInfoRequest::new(
                ProceedingSelector::Id(proceeding_id.clone()),
                pdf(),
            )),
            Decode::Pdf,
        ),
        (
            "definition points XML",
            GenerateReport::DefinitionPoints(DefinitionPointsRequest::new(
                DefinitionPointsSelector::Parcel(ParcelId::new("2928347306").unwrap()),
                xml(),
            )),
            Decode::DefinitionPointsXml,
        ),
        (
            "ownership overview XML",
            GenerateReport::OwnershipOverview(subject_overview()),
            Decode::OwnershipOverviewXml,
        ),
        (
            "ownership overview PDF",
            GenerateReport::OwnershipOverview(OwnershipOverviewRequest::new(
                SubjectId::new("627990601").unwrap(),
                pdf(),
            )),
            Decode::Pdf,
        ),
        (
            "ownership overview with properties XML",
            GenerateReport::OwnershipOverviewWithProperties(subject_overview()),
            Decode::OwnershipOverviewWithPropertiesXml,
        ),
        (
            "ownership overview with properties PDF",
            GenerateReport::OwnershipOverviewWithProperties(OwnershipOverviewRequest::new(
                SubjectId::new("627990601").unwrap(),
                pdf(),
            )),
            Decode::Pdf,
        ),
        ("rights-record XML", rights(xml()), Decode::RightsRecordXml),
        ("rights-record PDF", rights(pdf()), Decode::Pdf),
        (
            "prices by properties XML",
            GenerateReport::PricesByProperties(PricesByPropertiesRequest::new(
                price_properties.clone(),
                xml(),
            )),
            Decode::PricesByPropertiesXml,
        ),
        (
            "prices by properties PDF",
            GenerateReport::PricesByProperties(PricesByPropertiesRequest::new(
                price_properties,
                pdf(),
            )),
            Decode::Pdf,
        ),
        (
            "prices by proceeding XML",
            GenerateReport::PricesByProceeding(PricesByProceedingRequest::new(
                price_proceeding.clone(),
                xml(),
            )),
            Decode::PricesByProceedingXml,
        ),
        (
            "prices by proceeding PDF",
            GenerateReport::PricesByProceeding(PricesByProceedingRequest::new(
                price_proceeding,
                pdf(),
            )),
            Decode::Pdf,
        ),
        (
            "legacy LV 1.0 XML by number",
            ownership_sheet(
                lv_number(),
                ReportOptions::xml(ReportVersion::V1_0, AccountContext::Paying),
            ),
            Decode::OwnershipSheetLegacyXml,
        ),
        (
            "NTR LV XML by ID",
            ownership_sheet(OwnershipSheetSelector::Id(lv_id), xml()),
            Decode::OwnershipSheetNtrXml,
        ),
        (
            "NTR LV XML through subject",
            ownership_sheet(
                OwnershipSheetSelector::Subject {
                    cadastral_area: CadastralAreaCode::new("693936").unwrap(),
                    subject_id,
                },
                xml(),
            ),
            Decode::OwnershipSheetNtrXml,
        ),
        (
            "NTR LV XML through objects",
            ownership_sheet(
                OwnershipSheetSelector::Objects {
                    objects: ReportObjectList::Parcels(vec![lv_parcel_id]),
                    partial: false,
                },
                xml(),
            ),
            Decode::OwnershipSheetNtrXml,
        ),
        (
            "NTR LV PDF",
            ownership_sheet(lv_number(), pdf()),
            Decode::Pdf,
        ),
        (
            "geometric-plan PDF",
            GenerateReport::GeometricPlan(GeometricPlanRequest::new(
                plan.document_id.clone(),
                AccountContext::Paying,
            )),
            Decode::Pdf,
        ),
    ];

    let mut failures = Vec::new();
    for (name, request, decode) in requests {
        match run_report(&client, request, decode, false, &mut discovery).await {
            Ok(outputs) => println!("{name}: {outputs} output(s) passed"),
            Err(error) => failures.push(format!("{name}: {error}")),
        }
    }

    let map = GenerateReport::CadastralMap(CadastralMapRequest {
        selector: CadastralMapSelector::Parcel(discovery.parcel_id.clone().unwrap()),
        paper: MapPaperFormat::A4,
        landscape: false,
        scale: MapScale::Scale1000,
        easements: false,
        as_of: None,
        account: AccountContext::Paying,
    });
    match run_report(&client, map, Decode::Pdf, true, &mut discovery).await {
        Ok(0) => println!("cadastral-map PDF: definitive rejection; officially unavailable"),
        Ok(outputs) => println!("cadastral-map PDF: {outputs} output(s) passed"),
        Err(error) => failures.push(format!("cadastral-map PDF: {error}")),
    }

    let documents = client
        .find_collection_documents(CollectionDocumentSearch {
            proceeding: ProceedingSearchSelector::Id(plan.proceeding.id.clone()),
        })
        .await;
    let document_id: Option<DocumentId> = match documents {
        Ok(response) => response
            .value
            .items
            .first()
            .map(|document| document.id.clone()),
        Err(wsdp::Error::Service(error)) if matches!(error.code(), Some("387" | "435")) => None,
        Err(error) => {
            failures.push(format!("collection-document discovery: {error}"));
            None
        }
    };
    if let Some(document_id) = document_id {
        let request = GenerateReport::CollectionDocument(CollectionDocumentRequest::new(
            document_id,
            AccountContext::Paying,
        ));
        match run_report(&client, request, Decode::Pdf, true, &mut discovery).await {
            Ok(0) => {
                println!("collection-document PDF: definitive rejection; officially unavailable")
            }
            Ok(outputs) => println!("collection-document PDF: {outputs} output(s) passed"),
            Err(error) => failures.push(format!("collection-document PDF: {error}")),
        }
    } else {
        println!(
            "collection-document PDF: no valid trial document; ordering is officially unavailable"
        );
    }

    assert!(
        failures.is_empty(),
        "trial report matrix failures:\n{}",
        failures.join("\n")
    );
}

#[tokio::test]
#[ignore = "runs both account-statement formats against the shared official WSDP trial"]
async fn account_statement_outputs_with_cleanup() {
    let Some(client) = super::trial_client() else {
        panic!("set WSDP_RUN_TRIAL_TESTS=1 and WSDP_TRIAL_USERNAME/PASSWORD");
    };
    let mut discovery = LvDiscovery::default();
    for (name, format, decode) in [
        (
            "account-statement XML",
            wsdp::OutputFormat::Xml,
            Decode::AccountStatementXml,
        ),
        (
            "account-statement PDF",
            wsdp::OutputFormat::Pdf,
            Decode::Pdf,
        ),
    ] {
        let outputs = run_report(
            &client,
            account_request(format),
            decode,
            false,
            &mut discovery,
        )
        .await
        .unwrap_or_else(|error| panic!("{name}: {error}"));
        println!("{name}: {outputs} output(s) passed");
    }
}

#[tokio::test]
#[ignore = "downloads and decodes the monthly price ZIP from the shared official WSDP trial"]
async fn monthly_price_zip_with_cleanup() {
    let Some(client) = super::trial_client() else {
        panic!("set WSDP_RUN_TRIAL_TESTS=1 and WSDP_TRIAL_USERNAME/PASSWORD");
    };
    let outputs = run_report(
        &client,
        monthly_price_request(),
        Decode::MonthlyPriceZip,
        false,
        &mut LvDiscovery::default(),
    )
    .await
    .unwrap_or_else(|error| panic!("monthly price ZIP: {error}"));
    println!("monthly price ZIP: {outputs} output(s) passed");
}

#[tokio::test]
#[ignore = "checks configurable PDF+XML outputs against the shared official WSDP trial"]
async fn pdf_with_xml_when_enabled_with_cleanup() {
    let Some(client) = super::trial_client() else {
        panic!("set WSDP_RUN_TRIAL_TESTS=1 and WSDP_TRIAL_USERNAME/PASSWORD");
    };
    let mut discovery = LvDiscovery::default();
    let parcel = run_configurable_attachment(
        &client,
        parcel_pdf_with_xml_request(),
        Decode::ParcelPdfWithXml,
        &["527"],
        &mut discovery,
    )
    .await
    .unwrap_or_else(|error| panic!("parcel PDF+XML: {error}"));
    if parcel == 0 {
        println!("parcel PDF+XML: service 527; disabled for this report family");
    } else {
        println!("parcel PDF+XML: {parcel} output(s) passed");
    }

    let lv = run_configurable_attachment(
        &client,
        ownership_sheet(
            lv_number(),
            ReportOptions {
                version: wsdp::ReportVersionSelection::Pinned(ReportVersion::V1_0),
                ..ReportOptions::pdf_with_xml(AccountContext::Paying)
            },
        ),
        Decode::OwnershipSheetPdfWithXml,
        &["527", "579"],
        &mut discovery,
    )
    .await
    .unwrap_or_else(|error| panic!("legacy LV PDF+XML: {error}"));
    if lv == 0 {
        println!("legacy LV PDF+XML: current trial rejection (527/579)");
    } else {
        println!("legacy LV PDF+XML: {lv} output(s) passed");
    }

    let overview = run_configurable_attachment(
        &client,
        GenerateReport::OwnershipOverviewWithProperties(OwnershipOverviewRequest::new(
            SubjectId::new("627990601").unwrap(),
            ReportOptions::pdf_with_xml(AccountContext::Paying),
        )),
        Decode::OwnershipOverviewWithPropertiesPdfWithXml,
        &["527"],
        &mut discovery,
    )
    .await
    .unwrap_or_else(|error| panic!("overview-with-properties PDF+XML: {error}"));
    if overview == 0 {
        println!("overview-with-properties PDF+XML: service 527; disabled for this family");
    } else {
        println!("overview-with-properties PDF+XML: {overview} output(s) passed");
    }

    let rights = run_configurable_attachment(
        &client,
        GenerateReport::RightsRecord(RightsRecordRequest::new(
            RightsRecordSubjectKind::LegalPerson,
            "AG družstvo Kružberk",
            RightsRecordIdentifier::new("22829812").unwrap(),
            ReportOptions::pdf_with_xml(AccountContext::Paying),
        )),
        Decode::RightsRecordPdfWithXml,
        &["527"],
        &mut discovery,
    )
    .await
    .unwrap_or_else(|error| panic!("rights-record PDF+XML: {error}"));
    if rights == 0 {
        println!("rights-record PDF+XML: service 527; disabled for this family");
    } else {
        println!("rights-record PDF+XML: {rights} output(s) passed");
    }
}
