use quick_xml::Writer;
use quick_xml::events::{BytesEnd, BytesStart, Event};
use roxmltree::Node;

use super::soap::{self, XmlResult};
use crate::client::Operation;
use crate::model::{
    AccountContext, BuildingNaturalKey, BuildingRightInfoRequest, BuildingSelector,
    CadastralMapRequest, CadastralMapSelector, CollectionDocumentRequest, DefinitionPointsRequest,
    DefinitionPointsSelector, GenerateReport, GeometricPlanRequest, MapCoordinates,
    OwnershipOverviewRequest, OwnershipOverviewTerritory, OwnershipSheetRequest,
    OwnershipSheetSelector, ParcelSelector, PricesByCadastralAreaRequest,
    PricesByProceedingRequest, ProceedingSelector, ReportBatch, ReportFamily, ReportFormat,
    ReportId, ReportMetadata, ReportObjectList, ReportOptions, ReportState, ReportVersion,
    ReportVersionInfo, ReportVersionSelection, RightsRecordRequest, ServiceKind, UnitSelector,
    XmlDateTime,
};
use crate::{Czk, Error, Result};

pub(crate) fn versions_operation() -> Operation {
    Operation::new(
        ServiceKind::Reports,
        "dejVerzeSestav",
        "DejVerzeSestavRequest",
        "DejVerzeSestavResponse",
    )
}

/// Selects the exact operation descriptor for a semantic generation request.
pub(crate) fn operation(request: &GenerateReport) -> Operation {
    let (operation, request_element) = match request {
        GenerateReport::OwnershipSheet(request) => match &request.selector {
            OwnershipSheetSelector::Id(_) => ("generujLV", "GenerujLVRequest"),
            OwnershipSheetSelector::Number { .. } => {
                ("generujLVZjednodusene", "GenerujLVZjednoduseneRequest")
            }
            OwnershipSheetSelector::Subject { .. } => ("generujLVPresOS", "GenerujLVPresOSRequest"),
            OwnershipSheetSelector::Objects { .. } => {
                ("generujLVPresObjekty", "GenerujLVPresObjektyRequest")
            }
        },
        GenerateReport::CadastralMap(_) => ("generujMapu", "GenerujMapuRequest"),
        GenerateReport::ParcelInfo(_) => ("generujInfoOParcelach", "GenerujInfoOParcelachRequest"),
        GenerateReport::BuildingInfo(_) => ("generujInfoOStavbach", "GenerujInfoOStavbachRequest"),
        GenerateReport::UnitInfo(_) => ("generujInfoOJednotkach", "GenerujInfoOJednotkachRequest"),
        GenerateReport::BuildingRightInfo(_) => {
            ("generujInfoOPravuStavby", "GenerujInfoOPravuStavbyRequest")
        }
        GenerateReport::ProceedingInfo(_) => ("generujInfoORizeni", "GenerujInfoORizeniRequest"),
        GenerateReport::DefinitionPoints(_) => ("generujInfoOBodech", "GenerujInfoOBodechRequest"),
        GenerateReport::OwnershipOverview(_) => (
            "generujPrehledVlastnictvi",
            "GenerujPrehledVlastnictviRequest",
        ),
        GenerateReport::OwnershipOverviewWithProperties(_) => (
            "generujPrehledVlastnictviSNemovitostmi",
            "GenerujPrehledVlastnictviSNemovitostmiRequest",
        ),
        GenerateReport::RightsRecord(_) => (
            "generujEvidenciPravProOsobu",
            "GenerujEvidenciPravProOsobuRequest",
        ),
        GenerateReport::AccountStatement(_) => ("vypisUctu", "VypisUctuRequest"),
        GenerateReport::PricesByProperties(_) => (
            "generujCenoveUdajeDleNemovitosti",
            "GenerujCenoveUdajeDleNemovitostiRequest",
        ),
        GenerateReport::PricesByProceeding(_) => (
            "generujCenoveUdajeDleRizeni",
            "GenerujCenoveUdajeDleRizeniRequest",
        ),
        GenerateReport::PricesByCadastralArea(_) => {
            ("generujCenoveUdajeDleKu", "GenerujCenoveUdajeDleKuRequest")
        }
        GenerateReport::CollectionDocument(_) => (
            "generujVystupZeSbirkyListin",
            "GenerujVystupZeSbirkyListinRequest",
        ),
        GenerateReport::GeometricPlan(_) => ("generujVystupGP", "GenerujVystupGPRequest"),
    };
    Operation::new(
        ServiceKind::Reports,
        operation,
        request_element,
        "GenerujSestavuResponse",
    )
}

pub(crate) fn list_operation() -> Operation {
    Operation::new(
        ServiceKind::Reports,
        "seznamSestav",
        "SeznamSestavRequest",
        "GenerujSestavuResponse",
    )
}

pub(crate) fn status_operation() -> Operation {
    list_operation()
}

pub(crate) fn download_operation() -> Operation {
    Operation::new(
        ServiceKind::Reports,
        "vratSestavu",
        "VratSestavuRequest",
        "GenerujSestavuResponse",
    )
}

pub(crate) fn delete_operation() -> Operation {
    Operation::new(
        ServiceKind::Reports,
        "smazSestavu",
        "SmazSestavuRequest",
        "GenerujSestavuResponse",
    )
}

/// Validates and serializes a generation body in exact XSD sequence order.
pub(crate) fn write_generate(writer: &mut Writer<Vec<u8>>, request: &GenerateReport) -> Result<()> {
    request.validate()?;
    match request {
        GenerateReport::OwnershipSheet(request) => write_ownership_sheet(writer, request),
        GenerateReport::CadastralMap(request) => write_map(writer, request),
        GenerateReport::ParcelInfo(request) => {
            write_parcel_selector(writer, &request.selector)?;
            write_optional_text(writer, "datumK", request.as_of.as_ref())?;
            write_options(writer, &request.options, &ReportFamily::ParcelInfo)
        }
        GenerateReport::BuildingInfo(request) => {
            write_building_selector(writer, &request.selector)?;
            write_optional_text(writer, "datumK", request.as_of.as_ref())?;
            write_options(writer, &request.options, &ReportFamily::BuildingInfo)
        }
        GenerateReport::UnitInfo(request) => {
            match &request.selector {
                UnitSelector::Id(id) => write_text(writer, "jednotkaId", id.as_str())?,
                UnitSelector::NaturalKey(key) => {
                    write_building_key(writer, &key.building)?;
                    write_text(writer, "cisloJednotky", &key.unit_number.to_string())?;
                }
            }
            write_optional_text(writer, "datumK", request.as_of.as_ref())?;
            write_options(writer, &request.options, &ReportFamily::UnitInfo)
        }
        GenerateReport::BuildingRightInfo(request) => write_building_right(writer, request),
        GenerateReport::ProceedingInfo(request) => {
            write_proceeding_selector(writer, &request.selector)?;
            write_options(writer, &request.options, &ReportFamily::ProceedingInfo)
        }
        GenerateReport::DefinitionPoints(request) => write_definition_points(writer, request),
        GenerateReport::OwnershipOverview(request) => {
            write_overview(writer, request, &ReportFamily::OwnershipOverview)
        }
        GenerateReport::OwnershipOverviewWithProperties(request) => write_overview(
            writer,
            request,
            &ReportFamily::OwnershipOverviewWithProperties,
        ),
        GenerateReport::RightsRecord(request) => write_rights_record(writer, request),
        GenerateReport::AccountStatement(request) => {
            write_text(writer, "datumOd", request.from.as_str())?;
            write_optional_text(writer, "datumDo", request.to.as_ref())?;
            write_text(writer, "format", request.format.as_str())?;
            write_version(writer, &request.version, &ReportFamily::AccountStatement)
        }
        GenerateReport::PricesByProperties(request) => {
            write_objects(writer, &request.objects, "seznamObjektuTyp")?;
            write_optional_text(writer, "datumK", request.as_of.as_ref())?;
            write_options(writer, &request.options, &ReportFamily::PricesByProperties)
        }
        GenerateReport::PricesByProceeding(request) => write_prices_by_proceeding(writer, request),
        GenerateReport::PricesByCadastralArea(request) => write_prices_by_area(writer, request),
        GenerateReport::CollectionDocument(request) => write_collection_document(writer, request),
        GenerateReport::GeometricPlan(request) => write_geometric_plan(writer, request),
    }
}

fn write_ownership_sheet(
    writer: &mut Writer<Vec<u8>>,
    request: &OwnershipSheetRequest,
) -> Result<()> {
    match &request.selector {
        OwnershipSheetSelector::Id(id) => write_text(writer, "lvId", id.as_str())?,
        OwnershipSheetSelector::Number {
            cadastral_area,
            number,
        } => {
            write_text(writer, "katastrUzemiKod", cadastral_area.as_str())?;
            write_text(writer, "lvCislo", &number.to_string())?;
        }
        OwnershipSheetSelector::Subject {
            cadastral_area,
            subject_id,
        } => {
            write_text(writer, "katastrUzemiKod", cadastral_area.as_str())?;
            write_text(writer, "osId", subject_id.as_str())?;
        }
        OwnershipSheetSelector::Objects { objects, partial } => {
            // `seznamuObjektuTyp` is the spelling in the official 3.1 XSD.
            write_objects(writer, objects, "seznamuObjektuTyp")?;
            write_bool(writer, "castecnyVypis", *partial)?;
        }
    }
    write_optional_text(writer, "datumK", request.as_of.as_ref())?;
    write_options(writer, &request.options, &ReportFamily::OwnershipSheet)
}

fn write_building_right(
    writer: &mut Writer<Vec<u8>>,
    request: &BuildingRightInfoRequest,
) -> Result<()> {
    write_text(writer, "pravoStavbyId", request.building_right_id.as_str())?;
    write_optional_text(writer, "datumK", request.as_of.as_ref())?;
    write_options(writer, &request.options, &ReportFamily::BuildingRightInfo)
}

fn write_definition_points(
    writer: &mut Writer<Vec<u8>>,
    request: &DefinitionPointsRequest,
) -> Result<()> {
    match &request.selector {
        DefinitionPointsSelector::CadastralArea(code) => {
            write_text(writer, "katastrUzemiKod", code.as_str())?
        }
        DefinitionPointsSelector::Parcel(id) => write_text(writer, "parcelaId", id.as_str())?,
        DefinitionPointsSelector::Building(id) => write_text(writer, "stavbaId", id.as_str())?,
        DefinitionPointsSelector::Coordinates(query) => {
            write_coordinates(writer, &query.coordinates)?;
            if let Some(distance) = query.distance {
                write_text(writer, "vzdalenost", &distance.to_string())?;
            }
            write_text(writer, "typBodu", query.kind.as_str())?;
        }
    }
    write_options(writer, &request.options, &ReportFamily::DefinitionPoints)
}

fn write_overview(
    writer: &mut Writer<Vec<u8>>,
    request: &OwnershipOverviewRequest,
    family: &ReportFamily,
) -> Result<()> {
    write_text(writer, "osId", request.subject_id.as_str())?;
    match &request.territory {
        Some(OwnershipOverviewTerritory::Region(code)) => {
            write_text(writer, "krajKod", code.as_str())?
        }
        Some(OwnershipOverviewTerritory::Municipality(code)) => {
            write_text(writer, "obecKod", code.as_str())?
        }
        None => {}
    }
    if let Some(code) = &request.legal_relation {
        write_text(writer, "typPravnihoVztahuKod", code.as_str())?;
    }
    write_optional_text(writer, "datumK", request.as_of.as_ref())?;
    write_options(writer, &request.options, family)
}

fn write_rights_record(writer: &mut Writer<Vec<u8>>, request: &RightsRecordRequest) -> Result<()> {
    write_text(writer, "typ", request.subject_kind.as_str())?;
    write_text(writer, "nazev", &request.name)?;
    write_text(writer, "rcIco", request.identifier.as_str())?;
    write_options(writer, &request.options, &ReportFamily::RightsRecord)
}

fn write_prices_by_proceeding(
    writer: &mut Writer<Vec<u8>>,
    request: &PricesByProceedingRequest,
) -> Result<()> {
    write_proceeding_selector(writer, &request.selector)?;
    write_options(writer, &request.options, &ReportFamily::PricesByProceeding)
}

fn write_prices_by_area(
    writer: &mut Writer<Vec<u8>>,
    request: &PricesByCadastralAreaRequest,
) -> Result<()> {
    write_text(writer, "katastrUzemiKod", request.cadastral_area.as_str())?;
    write_text(writer, "rok", &request.year.to_string())?;
    write_text(writer, "mesicOd", &request.month_from.to_string())?;
    write_text(writer, "mesicDo", &request.month_to.to_string())?;
    write_account(writer, &request.account).map_err(xml_error)
}

fn write_map(writer: &mut Writer<Vec<u8>>, request: &CadastralMapRequest) -> Result<()> {
    match &request.selector {
        CadastralMapSelector::Parcel(id) => write_text(writer, "parcelaId", id.as_str())?,
        CadastralMapSelector::Coordinates(coordinates) => write_coordinates(writer, coordinates)?,
    }
    write_text(writer, "formatPapiru", request.paper.as_str())?;
    write_bool(writer, "orientaceNaSirku", request.landscape)?;
    write_text(writer, "meritkoMapy", &request.scale.as_u16().to_string())?;
    write_bool(writer, "vecnaBremena", request.easements)?;
    write_optional_text(writer, "datumK", request.as_of.as_ref())?;
    write_account(writer, &request.account).map_err(xml_error)
}

fn write_collection_document(
    writer: &mut Writer<Vec<u8>>,
    request: &CollectionDocumentRequest,
) -> Result<()> {
    write_text(writer, "listinaId", request.document_id.as_str())?;
    write_account(writer, &request.account).map_err(xml_error)
}

fn write_geometric_plan(
    writer: &mut Writer<Vec<u8>>,
    request: &GeometricPlanRequest,
) -> Result<()> {
    write_text(writer, "dokumentId", request.document_id.as_str())?;
    write_account(writer, &request.account).map_err(xml_error)
}

fn write_parcel_selector(writer: &mut Writer<Vec<u8>>, selector: &ParcelSelector) -> Result<()> {
    match selector {
        ParcelSelector::Id(id) => write_text(writer, "parcelaId", id.as_str()),
        ParcelSelector::NaturalKey(key) => {
            write_text(writer, "katastrUzemiKod", key.cadastral_area.as_str())?;
            write_text(writer, "kmenoveCislo", &key.number.root.to_string())?;
            if let Some(subdivision) = key.number.subdivision {
                write_text(writer, "poddeleni", &subdivision.to_string())?;
            }
            Ok(())
        }
    }
}

fn write_building_selector(
    writer: &mut Writer<Vec<u8>>,
    selector: &BuildingSelector,
) -> Result<()> {
    match selector {
        BuildingSelector::Id(id) => write_text(writer, "stavbaId", id.as_str()),
        BuildingSelector::NaturalKey(key) => write_building_key(writer, key),
    }
}

fn write_building_key(writer: &mut Writer<Vec<u8>>, key: &BuildingNaturalKey) -> Result<()> {
    write_text(writer, "castObceKod", key.municipality_part.as_str())?;
    write_text(writer, "typStavbyKod", key.number_kind.as_str())?;
    write_text(writer, "cisloDomovni", &key.number.to_string())
}

fn write_proceeding_selector(
    writer: &mut Writer<Vec<u8>>,
    selector: &ProceedingSelector,
) -> Result<()> {
    match selector {
        ProceedingSelector::Id(id) => write_text(writer, "rizeniId", id.as_str()),
        ProceedingSelector::Number(number) => write_text(writer, "rizeniCislo", number.as_str()),
    }
}

fn write_coordinates(writer: &mut Writer<Vec<u8>>, coordinates: &MapCoordinates) -> Result<()> {
    write_text(writer, "souradniceX", &coordinates.x.to_string())?;
    write_text(writer, "souradniceY", &coordinates.y.to_string())
}

fn write_objects(
    writer: &mut Writer<Vec<u8>>,
    objects: &ReportObjectList,
    kind_element: &str,
) -> Result<()> {
    write_text(writer, kind_element, objects.kind_code())?;
    match objects {
        ReportObjectList::Parcels(ids) => {
            for id in ids {
                write_text(writer, "seznamObjektu", id.as_str())?;
            }
        }
        ReportObjectList::Buildings(ids) => {
            for id in ids {
                write_text(writer, "seznamObjektu", id.as_str())?;
            }
        }
        ReportObjectList::Units(ids) => {
            for id in ids {
                write_text(writer, "seznamObjektu", id.as_str())?;
            }
        }
        ReportObjectList::BuildingRights(ids) => {
            for id in ids {
                write_text(writer, "seznamObjektu", id.as_str())?;
            }
        }
    }
    Ok(())
}

fn write_options(
    writer: &mut Writer<Vec<u8>>,
    options: &ReportOptions,
    family: &ReportFamily,
) -> Result<()> {
    write_text(writer, "format", options.format.as_str())?;
    write_version(writer, &options.version, family)?;
    if options.attach_xml {
        write_bool(writer, "pripojitXML", true)?;
    }
    write_account(writer, &options.account).map_err(xml_error)
}

fn write_version(
    writer: &mut Writer<Vec<u8>>,
    selection: &ReportVersionSelection,
    family: &ReportFamily,
) -> Result<()> {
    let version = match selection {
        ReportVersionSelection::DefaultForFamily => family.default_version(),
        ReportVersionSelection::Pinned(version) => Some(version.clone()),
        ReportVersionSelection::LatestAvailable => None,
    };
    if let Some(version) = version {
        write_text(writer, "verze", version.as_str())?;
    }
    Ok(())
}

fn write_account(writer: &mut Writer<Vec<u8>>, account: &AccountContext) -> XmlResult {
    match account {
        AccountContext::Paying => {}
        AccountContext::Free {
            cislo_jednaci,
            ucel_kod,
        } => {
            soap::text_element(writer, "cisloJednaci", cislo_jednaci)?;
            soap::text_element(writer, "ucelKod", &ucel_kod.to_string())?;
        }
        AccountContext::Verifying {
            poradove_cislo,
            overovaci_dolozka,
        } => {
            soap::text_element(writer, "poradoveCislo", poradove_cislo)?;
            soap::text_element(
                writer,
                "overovaciDolozka",
                if *overovaci_dolozka { "a" } else { "n" },
            )?;
        }
    }
    Ok(())
}

fn write_text(writer: &mut Writer<Vec<u8>>, name: &str, value: &str) -> Result<()> {
    soap::text_element(writer, name, value).map_err(xml_error)
}

fn write_optional_text(
    writer: &mut Writer<Vec<u8>>,
    name: &str,
    value: Option<&XmlDateTime>,
) -> Result<()> {
    if let Some(value) = value {
        write_text(writer, name, value.as_str())?;
    }
    Ok(())
}

fn write_bool(writer: &mut Writer<Vec<u8>>, name: &str, value: bool) -> Result<()> {
    write_text(writer, name, if value { "a" } else { "n" })
}

fn xml_error(error: std::io::Error) -> Error {
    Error::invalid_input("xml", format!("could not encode report request: {error}"))
}

pub(crate) fn write_list(writer: &mut Writer<Vec<u8>>, id: Option<&ReportId>) -> XmlResult {
    if let Some(id) = id {
        soap::text_element(writer, "idSestavy", id.as_str())?;
    }
    Ok(())
}

pub(crate) fn write_status(writer: &mut Writer<Vec<u8>>, id: &ReportId) -> XmlResult {
    write_list(writer, Some(id))
}

pub(crate) fn write_download(writer: &mut Writer<Vec<u8>>, id: &ReportId) -> XmlResult {
    soap::text_element(writer, "idSestavy", id.as_str())
}

pub(crate) fn write_delete(writer: &mut Writer<Vec<u8>>, id: &ReportId) -> XmlResult {
    soap::text_element(writer, "idSestavy", id.as_str())
}

pub(crate) fn write_versions(
    writer: &mut Writer<Vec<u8>>,
    families: Option<&[ReportFamily]>,
) -> XmlResult {
    if let Some(families) = families {
        writer.write_event(Event::Start(BytesStart::new("kodySestav")))?;
        for family in families {
            soap::text_element(writer, "kodSestavy", family.as_str())?;
        }
        writer.write_event(Event::End(BytesEnd::new("kodySestav")))?;
    }
    Ok(())
}

/// Decodes every report in generation/list/status responses, including all
/// parent and child IDs. No `souborSestavy` payload is returned by this function.
pub(crate) fn decode_report_batch(response: Node<'_, '_>) -> Result<ReportBatch> {
    let namespace = ServiceKind::Reports.namespace();
    let Some(list) = soap::child(response, namespace, "reportList")? else {
        return Ok(ReportBatch::default());
    };
    let reports = soap::children(list, namespace, "report")
        .map(decode_report_metadata)
        .collect::<Result<Vec<_>>>()?;
    if reports.is_empty() {
        return Err(Error::invalid_response(
            "reportList must contain at least one report",
        ));
    }
    Ok(ReportBatch::new(reports))
}

pub(crate) fn decode_delete(response: Node<'_, '_>) -> Result<()> {
    decode_report_batch(response).map(|_| ())
}

pub(crate) fn decode_report_metadata(report: Node<'_, '_>) -> Result<ReportMetadata> {
    let namespace = ServiceKind::Reports.namespace();
    let id = parse_id(required_text(report, namespace, "id")?, "report ID")?;
    let name = required_text(report, namespace, "nazev")?;
    let units = optional_text(report, namespace, "pocetJednotek")?
        .map(parse_units)
        .transpose()?;
    let pages = optional_text(report, namespace, "pocetStran")?
        .map(|value| {
            let pages: u16 = parse_number(value.trim(), "report page count")?;
            if pages > 9_999 {
                return Err(Error::invalid_response("report page count exceeds 9999"));
            }
            Ok(pages)
        })
        .transpose()?;
    let price = optional_text(report, namespace, "cena")?
        .map(|value| match value.trim().parse::<Czk>() {
            Ok(price) => Ok(price),
            Err(Error::DecimalOutOfRange { .. }) => Err(Error::DecimalOutOfRange {
                field: "report price",
            }),
            Err(_) => Err(Error::invalid_response("invalid report price")),
        })
        .transpose()?;
    let requested_at = parse_timestamp(required_text(report, namespace, "datumPozadavku")?)?;
    let started_at = optional_text(report, namespace, "datumSpusteni")?
        .map(parse_timestamp)
        .transpose()?;
    let created_at = optional_text(report, namespace, "datumVytvoreni")?
        .map(parse_timestamp)
        .transpose()?;
    let parent_id = optional_text(report, namespace, "idNadrizeneSestavy")?
        .map(|value| parse_id(value, "parent report ID"))
        .transpose()?;
    let child_ids = soap::children(report, namespace, "idPodrizeneSestavy")
        .map(|child| {
            let value = soap::simple_content(child, "idPodrizeneSestavy")?;
            parse_id(value, "child report ID")
        })
        .collect::<Result<Vec<_>>>()?;
    let state = ReportState::from_raw(required_text(report, namespace, "stav")?);
    let format = ReportFormat::from_raw(required_text(report, namespace, "format")?);
    let version = optional_text(report, namespace, "verze")?
        .map(|value| {
            ReportVersion::new(value.trim())
                .map_err(|_| Error::invalid_response("invalid report version"))
        })
        .transpose()?;
    let electronic_seal = optional_text(report, namespace, "elZnacka")?
        .map(|value| parse_bool(value.trim(), "electronic-seal flag"))
        .transpose()?;
    let timestamp = optional_text(report, namespace, "casoveRaziko")?
        .map(|value| parse_bool(value.trim(), "timestamp flag"))
        .transpose()?;
    // Check singleton cardinality without exposing or buffering report contents.
    soap::child(report, namespace, "souborSestavy")?;
    Ok(ReportMetadata {
        id,
        name,
        units,
        pages,
        price,
        requested_at,
        started_at,
        created_at,
        parent_id,
        child_ids,
        state,
        format,
        version,
        electronic_seal,
        timestamp,
    })
}

fn parse_id(value: String, field: &'static str) -> Result<ReportId> {
    ReportId::new(value.trim_matches([' ', '\t', '\r', '\n']))
        .map_err(|_| Error::invalid_response(format!("invalid {field}")))
}

fn parse_units(value: String) -> Result<rust_decimal::Decimal> {
    let value = value.trim_matches([' ', '\t', '\r', '\n']);
    let digits = value.strip_prefix('+').unwrap_or(value);
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(Error::invalid_response("invalid report unit count"));
    }
    crate::model::parse_decimal(value, "report unit count").map_err(|error| match error {
        Error::DecimalOutOfRange { .. } => Error::DecimalOutOfRange {
            field: "report unit count",
        },
        _ => Error::invalid_response("invalid report unit count"),
    })
}

fn parse_number<T>(value: &str, field: &'static str) -> Result<T>
where
    T: std::str::FromStr,
{
    value
        .trim_matches([' ', '\t', '\r', '\n'])
        .parse()
        .map_err(|_| Error::invalid_response(format!("invalid {field}")))
}

fn parse_timestamp(value: String) -> Result<XmlDateTime> {
    XmlDateTime::new(value).map_err(|_| Error::invalid_response("invalid report timestamp"))
}

fn parse_bool(value: &str, field: &'static str) -> Result<bool> {
    match value {
        "a" => Ok(true),
        "n" => Ok(false),
        _ => Err(Error::invalid_response(format!("invalid {field}"))),
    }
}

pub(crate) fn decode_versions(response: Node<'_, '_>) -> Result<Vec<ReportVersionInfo>> {
    let namespace = ServiceKind::Reports.namespace();
    let root = soap::child(response, namespace, "verzeSestav")?
        .ok_or_else(|| Error::invalid_response("report-version container is missing"))?;
    soap::children(root, namespace, "verzeSestavy")
        .map(|entry| {
            let family = required_text(entry, namespace, "kodSestavy")?;
            let name = required_text(entry, namespace, "nazevSestavy")?;
            let list = soap::child(entry, namespace, "seznamVerzi")?
                .ok_or_else(|| Error::invalid_response("report-version list is missing"))?;
            let versions = list
                .children()
                .filter(|node| {
                    node.is_element()
                        && node.tag_name().name() == "verze"
                        && node
                            .tag_name()
                            .namespace()
                            .is_some_and(|ns| ns == soap::COMMON_NS || ns == namespace)
                })
                .map(|version| {
                    let raw = soap::simple_content(version, "verze")?;
                    ReportVersion::new(raw.trim())
                        .map_err(|_| Error::invalid_response("invalid report version"))
                })
                .collect::<Result<Vec<_>>>()?;
            Ok(ReportVersionInfo {
                family: ReportFamily::from_raw(family),
                name,
                versions,
            })
        })
        .collect()
}

fn required_text(node: Node<'_, '_>, namespace: &str, name: &str) -> Result<String> {
    let element = soap::child(node, namespace, name)?
        .ok_or_else(|| Error::invalid_response(format!("required element {name} is missing")))?;
    soap::simple_content(element, name)
}

fn optional_text(node: Node<'_, '_>, namespace: &str, name: &str) -> Result<Option<String>> {
    soap::child(node, namespace, name)?
        .map(|element| soap::simple_content(element, name))
        .transpose()
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use roxmltree::Document;
    use rust_decimal::Decimal;

    use super::*;
    const REPORTS_NS: &str = "urn:cz:gov:cuzk:iskn:types:wsdp:sestavy:3.1";

    use crate::model::{
        AccountStatementRequest, BuildingInfoRequest, BuildingNumber, BuildingNumberKind,
        BuildingRightId, CadastralAreaCode, CadastralMapRequest, DefinitionPointKind,
        DefinitionPointQuery, DocumentId, MapPaperFormat, MapScale, MunicipalityCode,
        MunicipalityPartCode, OwnershipSheetId, OwnershipSheetNumber, ParcelId, ParcelInfoRequest,
        ParcelNaturalKey, ParcelNumber, PricesByPropertiesRequest, ProceedingNumber, RegionCode,
        RightsRecordIdentifier, RightsRecordSubjectKind, SubjectId, UnitId, UnitInfoRequest,
        UnitNaturalKey, UnitNumber,
    };

    fn render(request: &GenerateReport) -> String {
        let mut writer = Writer::new(Vec::new());
        write_generate(&mut writer, request).unwrap();
        String::from_utf8(writer.into_inner()).unwrap()
    }

    fn xml() -> ReportOptions {
        ReportOptions::xml_default(AccountContext::Paying)
    }

    fn xml_body(selector: &str) -> String {
        format!("{selector}<format>xml</format><verze>1.0</verze>")
    }

    #[test]
    fn request_distinguishes_omitted_and_empty_filters() {
        let mut omitted = Writer::new(Vec::new());
        write_versions(&mut omitted, None).unwrap();
        assert!(omitted.into_inner().is_empty());
        let mut empty = Writer::new(Vec::new());
        write_versions(&mut empty, Some(&[])).unwrap();
        assert_eq!(
            String::from_utf8(empty.into_inner()).unwrap(),
            "<kodySestav></kodySestav>"
        );
    }

    #[test]
    fn parcel_building_unit_and_right_requests_use_choice_specific_order() {
        let parcel = GenerateReport::ParcelInfo(ParcelInfoRequest {
            selector: ParcelSelector::NaturalKey(ParcelNaturalKey {
                cadastral_area: CadastralAreaCode::new("000001").unwrap(),
                number: ParcelNumber::new(0, Some(2)).unwrap(),
            }),
            as_of: Some(XmlDateTime::new("2026-01-02T03:04:05").unwrap()),
            options: xml(),
        });
        assert_eq!(operation(&parcel).request, "GenerujInfoOParcelachRequest");
        assert_eq!(
            render(&parcel),
            xml_body(
                "<katastrUzemiKod>000001</katastrUzemiKod><kmenoveCislo>0</kmenoveCislo><poddeleni>2</poddeleni><datumK>2026-01-02T03:04:05</datumK>"
            )
        );

        let key = BuildingNaturalKey {
            municipality_part: MunicipalityPartCode::new("000123").unwrap(),
            number_kind: BuildingNumberKind::Evidence,
            number: BuildingNumber::new(42).unwrap(),
        };
        let building = GenerateReport::BuildingInfo(BuildingInfoRequest::new(
            BuildingSelector::NaturalKey(key.clone()),
            xml(),
        ));
        assert_eq!(operation(&building).request, "GenerujInfoOStavbachRequest");
        assert_eq!(
            render(&building),
            xml_body(
                "<castObceKod>000123</castObceKod><typStavbyKod>2</typStavbyKod><cisloDomovni>42</cisloDomovni>"
            )
        );
        let unit = GenerateReport::UnitInfo(UnitInfoRequest::new(
            UnitSelector::NaturalKey(UnitNaturalKey {
                building: key,
                unit_number: UnitNumber::new(0).unwrap(),
            }),
            xml(),
        ));
        assert_eq!(operation(&unit).request, "GenerujInfoOJednotkachRequest");
        assert_eq!(
            render(&unit),
            xml_body(
                "<castObceKod>000123</castObceKod><typStavbyKod>2</typStavbyKod><cisloDomovni>42</cisloDomovni><cisloJednotky>0</cisloJednotky>"
            )
        );
        let right = GenerateReport::BuildingRightInfo(BuildingRightInfoRequest::new(
            BuildingRightId::new("77").unwrap(),
            xml(),
        ));
        assert_eq!(operation(&right).request, "GenerujInfoOPravuStavbyRequest");
        assert_eq!(
            render(&right),
            xml_body("<pravoStavbyId>77</pravoStavbyId>")
        );
    }

    #[test]
    fn all_four_lv_modes_pin_ntr3_and_preserve_official_typo() {
        let id = GenerateReport::OwnershipSheet(OwnershipSheetRequest::new(
            OwnershipSheetSelector::Id(OwnershipSheetId::new("1").unwrap()),
            xml(),
        ));
        assert_eq!(operation(&id).request, "GenerujLVRequest");
        assert_eq!(
            render(&id),
            "<lvId>1</lvId><format>xml</format><verze>3.0</verze>"
        );

        let number = GenerateReport::OwnershipSheet(OwnershipSheetRequest::new(
            OwnershipSheetSelector::Number {
                cadastral_area: CadastralAreaCode::new("1").unwrap(),
                number: OwnershipSheetNumber::new(100).unwrap(),
            },
            xml(),
        ));
        assert_eq!(operation(&number).request, "GenerujLVZjednoduseneRequest");
        assert!(
            render(&number)
                .starts_with("<katastrUzemiKod>1</katastrUzemiKod><lvCislo>100</lvCislo>")
        );

        let subject = GenerateReport::OwnershipSheet(OwnershipSheetRequest::new(
            OwnershipSheetSelector::Subject {
                cadastral_area: CadastralAreaCode::new("1").unwrap(),
                subject_id: SubjectId::new("2").unwrap(),
            },
            xml(),
        ));
        assert_eq!(operation(&subject).request, "GenerujLVPresOSRequest");
        assert!(render(&subject).contains("<osId>2</osId>"));

        let objects = GenerateReport::OwnershipSheet(OwnershipSheetRequest::new(
            OwnershipSheetSelector::Objects {
                objects: ReportObjectList::Units(vec![
                    UnitId::new("10").unwrap(),
                    UnitId::new("11").unwrap(),
                ]),
                partial: false,
            },
            xml(),
        ));
        assert_eq!(operation(&objects).request, "GenerujLVPresObjektyRequest");
        assert_eq!(
            render(&objects),
            "<seznamuObjektuTyp>JED_S</seznamuObjektuTyp><seznamObjektu>10</seznamObjektu><seznamObjektu>11</seznamObjektu><castecnyVypis>n</castecnyVypis><format>xml</format><verze>3.0</verze>"
        );
    }

    #[test]
    fn proceeding_definition_and_overview_requests_cover_every_selector() {
        let proceeding = GenerateReport::ProceedingInfo(crate::model::ProceedingInfoRequest::new(
            ProceedingSelector::Number(ProceedingNumber::new("V-1/2026-020").unwrap()),
            xml(),
        ));
        assert_eq!(operation(&proceeding).request, "GenerujInfoORizeniRequest");
        assert_eq!(
            render(&proceeding),
            xml_body("<rizeniCislo>V-1/2026-020</rizeniCislo>")
        );

        let points = GenerateReport::DefinitionPoints(DefinitionPointsRequest::new(
            DefinitionPointsSelector::Coordinates(DefinitionPointQuery {
                coordinates: MapCoordinates::new(
                    Decimal::new(-123456, 2),
                    Decimal::new(-654321, 2),
                )
                .unwrap(),
                distance: Some(Decimal::new(250, 2)),
                kind: DefinitionPointKind::Parcel,
            }),
            xml(),
        ));
        assert_eq!(operation(&points).request, "GenerujInfoOBodechRequest");
        assert_eq!(
            render(&points),
            xml_body(
                "<souradniceX>-1234.56</souradniceX><souradniceY>-6543.21</souradniceY><vzdalenost>2.50</vzdalenost><typBodu>p</typBodu>"
            )
        );

        let mut overview_request =
            OwnershipOverviewRequest::new(SubjectId::new("5").unwrap(), xml());
        overview_request.territory = Some(OwnershipOverviewTerritory::Region(
            RegionCode::new("19").unwrap(),
        ));
        overview_request.legal_relation = Some(crate::LegalRelationCode::Ownership);
        overview_request.as_of = Some(XmlDateTime::new("2026-01-01T00:00:00Z").unwrap());
        let overview = GenerateReport::OwnershipOverview(overview_request.clone());
        assert_eq!(
            operation(&overview).request,
            "GenerujPrehledVlastnictviRequest"
        );
        assert_eq!(
            render(&overview),
            xml_body(
                "<osId>5</osId><krajKod>19</krajKod><typPravnihoVztahuKod>30</typPravnihoVztahuKod><datumK>2026-01-01T00:00:00Z</datumK>"
            )
        );
        overview_request.territory = Some(OwnershipOverviewTerritory::Municipality(
            MunicipalityCode::new("500001").unwrap(),
        ));
        let with_properties = GenerateReport::OwnershipOverviewWithProperties(overview_request);
        assert_eq!(
            operation(&with_properties).request,
            "GenerujPrehledVlastnictviSNemovitostmiRequest"
        );
        assert!(render(&with_properties).contains("<obecKod>500001</obecKod>"));
    }

    #[test]
    fn rights_account_and_price_requests_have_exact_fields() {
        let rights = GenerateReport::RightsRecord(RightsRecordRequest::new(
            RightsRecordSubjectKind::LegalPerson,
            "A & B",
            RightsRecordIdentifier::new("01234567").unwrap(),
            xml(),
        ));
        assert_eq!(
            operation(&rights).request,
            "GenerujEvidenciPravProOsobuRequest"
        );
        assert_eq!(
            render(&rights),
            xml_body("<typ>OPO</typ><nazev>A &amp; B</nazev><rcIco>01234567</rcIco>")
        );

        let mut statement = AccountStatementRequest::new(
            XmlDateTime::new("2026-01-01T00:00:00Z").unwrap(),
            crate::OutputFormat::Xml,
        );
        statement.to = Some(XmlDateTime::new("2026-01-31T23:59:00Z").unwrap());
        statement.version = ReportVersionSelection::LatestAvailable;
        let statement = GenerateReport::AccountStatement(statement);
        assert_eq!(operation(&statement).request, "VypisUctuRequest");
        assert_eq!(
            render(&statement),
            "<datumOd>2026-01-01T00:00:00Z</datumOd><datumDo>2026-01-31T23:59:00Z</datumDo><format>xml</format>"
        );

        let properties = GenerateReport::PricesByProperties(PricesByPropertiesRequest::new(
            ReportObjectList::Parcels(vec![ParcelId::new("8").unwrap()]),
            xml(),
        ));
        assert_eq!(
            operation(&properties).request,
            "GenerujCenoveUdajeDleNemovitostiRequest"
        );
        assert_eq!(
            render(&properties),
            xml_body("<seznamObjektuTyp>PAR_S</seznamObjektuTyp><seznamObjektu>8</seznamObjektu>")
        );
        let proceeding = GenerateReport::PricesByProceeding(PricesByProceedingRequest::new(
            ProceedingSelector::Id(crate::ProceedingId::new("9").unwrap()),
            xml(),
        ));
        assert_eq!(
            operation(&proceeding).request,
            "GenerujCenoveUdajeDleRizeniRequest"
        );
        assert_eq!(render(&proceeding), xml_body("<rizeniId>9</rizeniId>"));
    }

    #[test]
    fn fixed_format_requests_never_emit_format_version_or_attached_xml() {
        let map = GenerateReport::CadastralMap(CadastralMapRequest {
            selector: CadastralMapSelector::Coordinates(
                MapCoordinates::new(Decimal::new(-100, 0), Decimal::new(-200, 0)).unwrap(),
            ),
            paper: MapPaperFormat::A4,
            landscape: true,
            scale: MapScale::Scale1000,
            easements: false,
            as_of: None,
            account: AccountContext::Verifying {
                poradove_cislo: "ISVS-1".into(),
                overovaci_dolozka: true,
            },
        });
        assert_eq!(operation(&map).request, "GenerujMapuRequest");
        assert_eq!(
            render(&map),
            "<souradniceX>-100</souradniceX><souradniceY>-200</souradniceY><formatPapiru>A4</formatPapiru><orientaceNaSirku>a</orientaceNaSirku><meritkoMapy>1000</meritkoMapy><vecnaBremena>n</vecnaBremena><poradoveCislo>ISVS-1</poradoveCislo><overovaciDolozka>a</overovaciDolozka>"
        );
        let monthly = GenerateReport::PricesByCadastralArea(PricesByCadastralAreaRequest::new(
            CadastralAreaCode::new("1").unwrap(),
            2026,
            1,
            12,
            AccountContext::Free {
                cislo_jednaci: "CJ-1".into(),
                ucel_kod: 99,
            },
        ));
        assert_eq!(
            operation(&monthly).request,
            "GenerujCenoveUdajeDleKuRequest"
        );
        assert_eq!(
            render(&monthly),
            "<katastrUzemiKod>1</katastrUzemiKod><rok>2026</rok><mesicOd>1</mesicOd><mesicDo>12</mesicDo><cisloJednaci>CJ-1</cisloJednaci><ucelKod>99</ucelKod>"
        );
        let deed = GenerateReport::CollectionDocument(CollectionDocumentRequest::new(
            DocumentId::new("12").unwrap(),
            AccountContext::Paying,
        ));
        assert_eq!(
            operation(&deed).request,
            "GenerujVystupZeSbirkyListinRequest"
        );
        assert_eq!(render(&deed), "<listinaId>12</listinaId>");
        let gp = GenerateReport::GeometricPlan(GeometricPlanRequest::new(
            DocumentId::new("13").unwrap(),
            AccountContext::Paying,
        ));
        assert_eq!(operation(&gp).request, "GenerujVystupGPRequest");
        assert_eq!(render(&gp), "<dokumentId>13</dokumentId>");
    }

    #[test]
    fn options_pin_versions_and_order_attached_xml_before_account_fields() {
        let request = GenerateReport::OwnershipSheet(OwnershipSheetRequest::new(
            OwnershipSheetSelector::Id(OwnershipSheetId::new("1").unwrap()),
            ReportOptions::new(
                crate::OutputFormat::Pdf,
                ReportVersionSelection::Pinned(ReportVersion::new("3.1").unwrap()),
                true,
                AccountContext::Free {
                    cislo_jednaci: "CJ&1".into(),
                    ucel_kod: 5,
                },
            ),
        ));
        assert_eq!(
            render(&request),
            "<lvId>1</lvId><format>pdf</format><verze>3.1</verze><pripojitXML>a</pripojitXML><cisloJednaci>CJ&amp;1</cisloJednaci><ucelKod>5</ucelKod>"
        );
        let latest = GenerateReport::ParcelInfo(ParcelInfoRequest::new(
            ParcelSelector::Id(ParcelId::new("1").unwrap()),
            ReportOptions::xml_latest(AccountContext::Paying),
        ));
        assert!(!render(&latest).contains("verze"));
        let invalid = GenerateReport::DefinitionPoints(DefinitionPointsRequest::new(
            DefinitionPointsSelector::CadastralArea(CadastralAreaCode::new("1").unwrap()),
            ReportOptions::pdf(AccountContext::Paying),
        ));
        let mut writer = Writer::new(Vec::new());
        assert!(write_generate(&mut writer, &invalid).is_err());
        assert!(writer.into_inner().is_empty());
    }

    #[test]
    fn lifecycle_writers_and_descriptors_match_wsdl() {
        let id = ReportId::new("123").unwrap();
        for (name, operation) in [
            ("SeznamSestavRequest", list_operation()),
            ("SeznamSestavRequest", status_operation()),
            ("VratSestavuRequest", download_operation()),
            ("SmazSestavuRequest", delete_operation()),
        ] {
            assert_eq!(operation.request, name);
            assert_eq!(operation.response, "GenerujSestavuResponse");
        }
        let mut writer = Writer::new(Vec::new());
        write_list(&mut writer, None).unwrap();
        assert!(writer.into_inner().is_empty());
        for write in [
            write_status as fn(&mut Writer<Vec<u8>>, &ReportId) -> XmlResult,
            write_download,
            write_delete,
        ] {
            let mut writer = Writer::new(Vec::new());
            write(&mut writer, &id).unwrap();
            assert_eq!(
                String::from_utf8(writer.into_inner()).unwrap(),
                "<idSestavy>123</idSestavy>"
            );
        }
    }

    #[test]
    fn report_batch_decodes_every_field_and_parent_child_relationship() {
        let xml = format!(
            r#"<GenerujSestavuResponse xmlns="{REPORTS_NS}"><reportList>
            <report><id>000123</id><nazev>PDF output</nazev><pocetJednotek>12</pocetJednotek><pocetStran>9999</pocetStran><cena>10.25</cena><datumPozadavku>2026-01-01T01:02:03</datumPozadavku><datumSpusteni>2026-01-01T01:02:04+01:00</datumSpusteni><datumVytvoreni>2026-01-01T01:02:05.123Z</datumVytvoreni><idPodrizeneSestavy>124</idPodrizeneSestavy><idPodrizeneSestavy>125</idPodrizeneSestavy><stav>zpracován</stav><format>pdf</format><verze>3.0</verze><elZnacka>a</elZnacka><casoveRaziko>n</casoveRaziko><souborSestavy>ignored</souborSestavy></report>
            <report><id>124</id><nazev>XML output</nazev><datumPozadavku>2026-01-01T01:02:03Z</datumPozadavku><idNadrizeneSestavy>000123</idNadrizeneSestavy><stav>budouci stav</stav><format>future</format></report>
            </reportList></GenerujSestavuResponse>"#
        );
        let document = Document::parse(&xml).unwrap();
        let batch = decode_report_batch(document.root_element()).unwrap();
        assert_eq!(batch.reports.len(), 2);
        let first = &batch.reports[0];
        assert_eq!(first.id.as_str(), "000123");
        assert_eq!(first.units, Some(Decimal::from(12)));
        assert_eq!(first.pages, Some(9999));
        assert_eq!(first.price.unwrap().as_decimal(), &Decimal::new(1025, 2));
        assert_eq!(first.requested_at.as_str(), "2026-01-01T01:02:03");
        assert_eq!(
            first.started_at.as_ref().unwrap().as_str(),
            "2026-01-01T01:02:04+01:00"
        );
        assert_eq!(first.parent_id, None);
        assert_eq!(
            first
                .child_ids
                .iter()
                .map(ReportId::as_str)
                .collect::<Vec<_>>(),
            ["124", "125"]
        );
        assert_eq!(first.state, ReportState::Processed);
        assert_eq!(first.format, ReportFormat::Pdf);
        assert_eq!(first.version, Some(ReportVersion::V3_0));
        assert_eq!(first.electronic_seal, Some(true));
        assert_eq!(first.timestamp, Some(false));
        assert_eq!(
            batch.reports[1].parent_id.as_ref().unwrap().as_str(),
            "000123"
        );
        assert_eq!(batch.reports[1].price, None);
        assert!(batch.reports[1].state.is_unknown());
        assert_eq!(batch.reports[1].format.as_str(), "future");
        let single = ReportBatch::new(vec![batch.reports[0].clone()])
            .into_single_report()
            .unwrap();
        assert_eq!(single.id.as_str(), "000123");
        assert_eq!(
            ReportBatch::default()
                .into_single_report()
                .unwrap_err()
                .report_ids(),
            Some(&[][..])
        );
        let error = batch.into_single_report().unwrap_err();
        assert_eq!(error.report_ids().unwrap().len(), 2);
        assert_eq!(error.report_ids().unwrap()[0].as_str(), "000123");
        assert_eq!(error.report_ids().unwrap()[1].as_str(), "124");
    }

    #[test]
    fn report_units_use_exact_unbounded_integer_semantics() {
        let value = "18446744073709551616"; // u64::MAX + 1
        let xml = format!(
            r#"<GenerujSestavuResponse xmlns="{REPORTS_NS}"><reportList><report><id>123</id><nazev>test</nazev><pocetJednotek>{value}</pocetJednotek><datumPozadavku>2026-01-01T00:00:00Z</datumPozadavku><stav>čeká</stav><format>xml</format></report></reportList></GenerujSestavuResponse>"#
        );
        let document = Document::parse(&xml).unwrap();
        let batch = decode_report_batch(document.root_element()).unwrap();
        assert_eq!(
            batch.reports[0].units,
            Some(Decimal::from_str(value).unwrap())
        );
        let invalid = xml.replace(value, "1.0");
        let document = Document::parse(&invalid).unwrap();
        assert!(decode_report_batch(document.root_element()).is_err());
        let oversized = xml.replace(value, "999999999999999999999999999999999999999");
        let document = Document::parse(&oversized).unwrap();
        assert!(matches!(
            decode_report_batch(document.root_element()),
            Err(Error::DecimalOutOfRange {
                field: "report unit count"
            })
        ));
    }

    #[test]
    fn malformed_metadata_and_empty_lists_fail_without_echoing_content() {
        let empty = format!(
            r#"<GenerujSestavuResponse xmlns="{REPORTS_NS}"><reportList/></GenerujSestavuResponse>"#
        );
        let document = Document::parse(&empty).unwrap();
        assert!(decode_report_batch(document.root_element()).is_err());

        let bad = format!(
            r#"<GenerujSestavuResponse xmlns="{REPORTS_NS}"><reportList><report><id>sensitive-value</id><nazev>x</nazev><datumPozadavku>2026-01-01T00:00:00Z</datumPozadavku><stav>čeká</stav><format>xml</format></report></reportList></GenerujSestavuResponse>"#
        );
        let document = Document::parse(&bad).unwrap();
        let error = decode_report_batch(document.root_element()).unwrap_err();
        assert!(!error.to_string().contains("sensitive-value"));
    }

    #[test]
    fn versions_decoder_retains_future_families_and_versions() {
        let xml = format!(
            r#"<DejVerzeSestavResponse xmlns="{REPORTS_NS}" xmlns:c="{}"><verzeSestav><verzeSestavy><kodSestavy>Future</kodSestavy><nazevSestavy>Future report</nazevSestavy><seznamVerzi><c:verze>4.01</c:verze></seznamVerzi></verzeSestavy></verzeSestav></DejVerzeSestavResponse>"#,
            soap::COMMON_NS
        );
        let document = Document::parse(&xml).unwrap();
        let versions = decode_versions(document.root_element()).unwrap();
        assert_eq!(versions[0].family.as_str(), "Future");
        assert_eq!(versions[0].versions[0].as_str(), "4.01");
        assert_eq!(Czk::from_str("0.00").unwrap(), Czk::ZERO);
    }
}
