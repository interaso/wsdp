use quick_xml::Writer;
use quick_xml::events::{BytesEnd, BytesStart, Event};
use roxmltree::Node;

use super::soap::{
    COMMON_NS, XmlResult, child, children, is_nil, require_tag, simple_content, text_element,
};
use crate::client::Operation;
use crate::error::{Error, Result};
use crate::model::search::{
    self, AddressNumberKind, AuthorizedSubject, Building, BuildingAttachment, BuildingIdentity,
    BuildingLocation, BuildingRight, BuildingRightIdentity, BuildingRightPurpose,
    BuildingRightSelector, BuildingSearchSelector, CadastralAreaReference, CollectionDocument,
    GeometricPlan, MunicipalityPartReference, Ownership, OwnershipTarget, Parcel, ParcelIdentity,
    ParcelKind, ParcelNumberingKind, ParcelSearchSelector, Proceeding, ProceedingSearchSelector,
    SearchRequest as Request, SearchResponse as Response, SearchResult, SubjectBase, SubjectKind,
    SubjectSearchSelector, Unit, UnitSearchSelector, XmlDate, YesNo,
};
use crate::model::search::{FractionComponent, ShortPersonalIdentifier};
use crate::model::{ServiceKind, XmlDateTime};

const SEARCH_NS: &str = "urn:cz:gov:cuzk:iskn:types:wsdp:vyhledat:3.1";
const SEARCH_ACTION: &str = "urn:cz:gov:cuzk:iskn:services:wsdp:vyhledat";

/// Returns the exact WSDL operation and SOAP action for a search request.
pub(crate) fn operation(request: &Request) -> Operation {
    let (name, request_name, response_name) = match request {
        Request::Parcel(_) => (
            "najdiParcelu",
            "NajdiParceluRequest",
            "NajdiParceluResponse",
        ),
        Request::Unit(_) => (
            "najdiJednotku",
            "NajdiJednotkuRequest",
            "NajdiJednotkuResponse",
        ),
        Request::Building(_) => ("najdiStavbu", "NajdiStavbuRequest", "NajdiStavbuResponse"),
        Request::Subject(_) => ("najdiOS", "NajdiOSRequest", "NajdiOSResponse"),
        Request::BuildingRight(_) => (
            "najdiPravoStavby",
            "NajdiPravoStavbyRequest",
            "NajdiPravoStavbyResponse",
        ),
        Request::Proceeding(_) => ("najdiRizeni", "NajdiRizeniRequest", "NajdiRizeniResponse"),
        Request::CollectionDocument(_) => (
            "najdiListinuVeSbirceListin",
            "NajdiListinuVeSbirceListinRequest",
            "NajdiListinuVeSbirceListinResponse",
        ),
        Request::GeometricPlan(_) => ("najdiGP", "NajdiGPRequest", "NajdiGPResponse"),
    };
    let operation = Operation::new(ServiceKind::Search, name, request_name, response_name);
    if matches!(request, Request::CollectionDocument(_)) {
        operation.with_action(format!("{SEARCH_ACTION}:najdiListinu"))
    } else {
        operation
    }
}

/// Writes the operation-specific children of the SOAP request element.
pub(crate) fn write_request(writer: &mut Writer<Vec<u8>>, request: &Request) -> XmlResult {
    match request {
        Request::Parcel(request) => {
            write_parcel_selector(writer, &request.selector)?;
            write_optional_text(writer, "datumK", request.valid_at.as_ref())
        }
        Request::Unit(request) => {
            write_unit_selector(writer, &request.selector)?;
            write_optional_text(writer, "datumK", request.valid_at.as_ref())
        }
        Request::Building(request) => {
            write_building_selector(writer, &request.selector)?;
            write_optional_text(writer, "datumK", request.valid_at.as_ref())
        }
        Request::Subject(request) => {
            match &request.selector {
                SubjectSearchSelector::LegalPerson(subject) => {
                    write_optional_text(writer, "ico", subject.company_id.as_ref())?;
                    write_optional_text(writer, "nazev", subject.name.as_ref())?;
                }
                SubjectSearchSelector::PersonOrPartnership(subject) => {
                    write_optional_text(writer, "jmeno", subject.given_name.as_ref())?;
                    write_optional_text(writer, "prijmeni", subject.surname.as_ref())?;
                    match &subject.birth {
                        Some(search::PersonBirthSelector::PersonalIdentifier(identifier)) => {
                            text_element(writer, "rc", identifier.as_str())?;
                        }
                        Some(search::PersonBirthSelector::BirthDate(date)) => {
                            text_element(writer, "datumNarozeni", date.as_str())?;
                        }
                        None => {}
                    }
                    if let Some(address) = &subject.address {
                        container(writer, "adresniUdaje", |writer| {
                            write_optional_text(writer, "obec", address.municipality.as_ref())?;
                            write_optional_text(
                                writer,
                                "castObce",
                                address.municipality_part.as_ref(),
                            )?;
                            write_optional_text(writer, "ulice", address.street.as_ref())?;
                            write_optional_text(
                                writer,
                                "cisloOrientacni",
                                address.orientation_number.as_ref(),
                            )?;
                            write_optional_text(
                                writer,
                                "cisloDomovni",
                                address.house_number.as_ref(),
                            )
                        })?;
                    }
                }
            }
            write_optional_text(writer, "datumK", request.valid_at.as_ref())
        }
        Request::BuildingRight(request) => {
            match &request.selector {
                BuildingRightSelector::Id(id) => {
                    text_element(writer, "pravoStavbyId", id.as_str())?
                }
                BuildingRightSelector::Parcel(ParcelSearchSelector::Id(id)) => {
                    text_element(writer, "parcelaId", id.as_str())?
                }
                BuildingRightSelector::Parcel(ParcelSearchSelector::Number(key)) => {
                    write_parcel_key(writer, key)?
                }
            }
            write_optional_text(writer, "datumK", request.valid_at.as_ref())
        }
        Request::Proceeding(request) => write_proceeding_selector(writer, &request.selector),
        Request::CollectionDocument(request) => {
            write_proceeding_selector(writer, &request.proceeding)
        }
        Request::GeometricPlan(request) => {
            text_element(writer, "katuzeKod", request.cadastral_area.as_str())?;
            text_element(
                writer,
                "cisloVysledkuZemCinnosti",
                &request.survey_result_number.to_string(),
            )
        }
    }
}

fn write_parcel_selector(
    writer: &mut Writer<Vec<u8>>,
    selector: &ParcelSearchSelector,
) -> XmlResult {
    match selector {
        ParcelSearchSelector::Id(id) => text_element(writer, "parcelaId", id.as_str()),
        ParcelSearchSelector::Number(key) => write_parcel_key(writer, key),
    }
}

fn write_parcel_key(writer: &mut Writer<Vec<u8>>, key: &search::ParcelSearchKey) -> XmlResult {
    text_element(writer, "katastrUzemiKod", key.cadastral_area.as_str())?;
    text_element(writer, "kmenoveCislo", &key.number.root.to_string())?;
    if let Some(subdivision) = key.number.subdivision {
        text_element(writer, "poddeleni", &subdivision.to_string())?;
    }
    Ok(())
}

fn write_building_key(writer: &mut Writer<Vec<u8>>, key: &search::BuildingSearchKey) -> XmlResult {
    text_element(writer, "castObceKod", key.municipality_part.as_str())?;
    text_element(writer, "typStavbyKod", key.building_type.as_str())?;
    text_element(writer, "cisloDomovni", &key.house_number.to_string())
}

fn write_building_selector(
    writer: &mut Writer<Vec<u8>>,
    selector: &BuildingSearchSelector,
) -> XmlResult {
    match selector {
        BuildingSearchSelector::Id(id) => text_element(writer, "stavbaId", id.as_str()),
        BuildingSearchSelector::Number(key) => write_building_key(writer, key),
    }
}

fn write_unit_selector(writer: &mut Writer<Vec<u8>>, selector: &UnitSearchSelector) -> XmlResult {
    match selector {
        UnitSearchSelector::Id(id) => text_element(writer, "jednotkaId", id.as_str()),
        UnitSearchSelector::Number(key) => {
            write_building_selector(writer, &key.building)?;
            if let Some(number) = key.number {
                text_element(writer, "cisloJednotky", &number.to_string())?;
            }
            Ok(())
        }
    }
}

fn write_proceeding_selector(
    writer: &mut Writer<Vec<u8>>,
    selector: &ProceedingSearchSelector,
) -> XmlResult {
    match selector {
        ProceedingSearchSelector::Id(id) => text_element(writer, "rizeniId", id.as_str()),
        ProceedingSearchSelector::Number(key) => {
            text_element(writer, "rizeniTyp", &key.proceeding_type)?;
            text_element(writer, "poradoveCislo", &key.serial_number.to_string())?;
            text_element(writer, "rok", &key.year.to_string())?;
            text_element(writer, "praresKod", key.workplace.as_str())
        }
    }
}

fn write_optional_text<T: ToString>(
    writer: &mut Writer<Vec<u8>>,
    name: &str,
    value: Option<&T>,
) -> XmlResult {
    if let Some(value) = value {
        text_element(writer, name, &value.to_string())?;
    }
    Ok(())
}

fn container(
    writer: &mut Writer<Vec<u8>>,
    name: &str,
    write_contents: impl FnOnce(&mut Writer<Vec<u8>>) -> XmlResult,
) -> XmlResult {
    writer.write_event(Event::Start(BytesStart::new(name)))?;
    write_contents(writer)?;
    writer.write_event(Event::End(BytesEnd::new(name)))
}

/// Decodes the operation-specific response element selected by `request`.
///
/// The client must call `SearchResponse::refine_completion` with the retained
/// service messages to apply codes 310 and 392.
pub(crate) fn decode_response(request: &Request, response: Node<'_, '_>) -> Result<Response> {
    require_tag(
        response,
        SEARCH_NS,
        match request {
            Request::Parcel(_) => "NajdiParceluResponse",
            Request::Unit(_) => "NajdiJednotkuResponse",
            Request::Building(_) => "NajdiStavbuResponse",
            Request::Subject(_) => "NajdiOSResponse",
            Request::BuildingRight(_) => "NajdiPravoStavbyResponse",
            Request::Proceeding(_) => "NajdiRizeniResponse",
            Request::CollectionDocument(_) => "NajdiListinuVeSbirceListinResponse",
            Request::GeometricPlan(_) => "NajdiGPResponse",
        },
    )?;
    match request {
        Request::Parcel(_) => required_results(response, "ParcelaList", "Parcela", decode_parcel)
            .map(Response::Parcels),
        Request::Unit(_) => {
            required_results(response, "JednotkaList", "Jednotka", decode_unit).map(Response::Units)
        }
        Request::Building(_) => required_results(response, "StavbaList", "Stavba", decode_building)
            .map(Response::Buildings),
        Request::Subject(_) => {
            required_results(response, "OSList", "OS", decode_subject).map(Response::Subjects)
        }
        Request::BuildingRight(_) => required_results(
            response,
            "PravoStavbyList",
            "pravoStavby",
            decode_building_right,
        )
        .map(Response::BuildingRights),
        Request::Proceeding(_) => {
            required_results(response, "RizeniList", "rizeni", decode_proceeding)
                .map(Response::Proceedings)
        }
        Request::CollectionDocument(_) => {
            let list = child(response, SEARCH_NS, "ListinaList")?;
            let items = if let Some(list) = list {
                let items = children(list, SEARCH_NS, "listina")
                    .map(decode_document)
                    .collect::<Result<Vec<_>>>()?;
                if items.is_empty() {
                    return Err(Error::invalid_response(
                        "ListinaList must contain at least one listina when present",
                    ));
                }
                items
            } else {
                Vec::new()
            };
            Ok(Response::CollectionDocuments(SearchResult::from_items(
                items,
            )))
        }
        Request::GeometricPlan(_) => {
            let list = child(response, SEARCH_NS, "geometrickePlany")?;
            let items = if let Some(list) = list {
                children(list, COMMON_NS, "geometrickyPlan")
                    .map(decode_geometric_plan)
                    .collect::<Result<Vec<_>>>()?
            } else {
                Vec::new()
            };
            Ok(Response::GeometricPlans(SearchResult::from_items(items)))
        }
    }
}

fn required_results<T>(
    response: Node<'_, '_>,
    list_name: &str,
    item_name: &str,
    decode: fn(Node<'_, '_>) -> Result<T>,
) -> Result<SearchResult<T>> {
    let list = required_child(response, SEARCH_NS, list_name)?;
    let items = children(list, SEARCH_NS, item_name)
        .map(decode)
        .collect::<Result<Vec<_>>>()?;
    Ok(SearchResult::from_items(items))
}

fn required_child<'a, 'input>(
    node: Node<'a, 'input>,
    namespace: &str,
    name: &str,
) -> Result<Node<'a, 'input>> {
    child(node, namespace, name)?
        .ok_or_else(|| Error::invalid_response(format!("required XML element {name} is missing")))
}

fn required_text(node: Node<'_, '_>, namespace: &str, name: &str) -> Result<String> {
    let element = required_child(node, namespace, name)?;
    if is_nil(element) {
        return Err(Error::invalid_response(format!(
            "required XML element {name} is nil"
        )));
    }
    simple_content(element, name)
}

fn optional_text(node: Node<'_, '_>, namespace: &str, name: &str) -> Result<Option<String>> {
    let Some(element) = child(node, namespace, name)? else {
        return Ok(None);
    };
    if is_nil(element) {
        return Ok(None);
    }
    simple_content(element, name).map(Some)
}

fn required_parse<T: std::str::FromStr>(
    node: Node<'_, '_>,
    namespace: &str,
    name: &str,
) -> Result<T> {
    parse_text(required_text(node, namespace, name)?, name)
}

fn optional_parse<T: std::str::FromStr>(
    node: Node<'_, '_>,
    namespace: &str,
    name: &str,
) -> Result<Option<T>> {
    optional_text(node, namespace, name)?
        .map(|value| parse_text(value, name))
        .transpose()
}

fn parse_text<T: std::str::FromStr>(value: String, name: &str) -> Result<T> {
    value
        .parse()
        .map_err(|_| Error::invalid_response(format!("XML element {name} has an invalid value")))
}

fn parse_node<T: std::str::FromStr>(node: Node<'_, '_>, name: &str) -> Result<T> {
    if is_nil(node) {
        return Err(Error::invalid_response(format!(
            "required XML element {name} is nil"
        )));
    }
    parse_text(simple_content(node, name)?, name)
}

pub(crate) fn decode_cadastral_area(node: Node<'_, '_>) -> Result<CadastralAreaReference> {
    Ok(CadastralAreaReference {
        code: optional_parse(node, COMMON_NS, "kod")?,
        name: optional_text(node, COMMON_NS, "nazev")?,
    })
}

fn decode_municipality_part(node: Node<'_, '_>) -> Result<MunicipalityPartReference> {
    require_tag(node, COMMON_NS, "castObce")?;
    Ok(MunicipalityPartReference {
        code: required_parse(node, COMMON_NS, "kod")?,
        name: required_text(node, COMMON_NS, "nazev")?,
    })
}

pub(crate) fn decode_parcel_identity(node: Node<'_, '_>) -> Result<ParcelIdentity> {
    Ok(ParcelIdentity {
        id: required_parse(node, COMMON_NS, "idParcely")?,
        ownership_sheet_id: optional_parse(node, COMMON_NS, "lvId")?,
        cadastral_area: required_parse(node, COMMON_NS, "katuzeKod")?,
        root_number: required_parse(node, COMMON_NS, "kmenoveCislo")?,
        kind: ParcelKind::from_raw(required_text(node, COMMON_NS, "parcelaType")?),
        numbering_kind: ParcelNumberingKind::from_raw(required_text(
            node,
            COMMON_NS,
            "druhCislovaniPar",
        )?),
        subdivision: optional_parse(node, COMMON_NS, "poddeleniCisla")?,
        original_cadastral_area: optional_parse(node, COMMON_NS, "katuzePuv")?,
        simplified_registration_kind: optional_text(node, COMMON_NS, "typZE")?,
        part: optional_parse(node, COMMON_NS, "dil")?,
    })
}

pub(crate) fn decode_building_identity(node: Node<'_, '_>) -> Result<BuildingIdentity> {
    let municipality_part = optional_parse(node, COMMON_NS, "castObceKod")?;
    let parcel = child(node, COMMON_NS, "parcelaIdentifikacni")?;
    let location = match (municipality_part, parcel) {
        (Some(municipality_part), None) => BuildingLocation::Address {
            municipality_part,
            house_number: required_text(node, COMMON_NS, "cisloDomovni")?,
        },
        (None, Some(parcel)) => BuildingLocation::Parcel(decode_parcel_identity(parcel)?),
        _ => {
            return Err(Error::invalid_response(
                "building identity must select an address or identifying parcel",
            ));
        }
    };
    Ok(BuildingIdentity {
        id: required_parse(node, COMMON_NS, "idStavby")?,
        ownership_sheet_id: optional_parse(node, COMMON_NS, "lvId")?,
        building_type: required_parse(node, COMMON_NS, "typStavbyKod")?,
        location,
    })
}

fn decode_building_right_identity(node: Node<'_, '_>) -> Result<BuildingRightIdentity> {
    Ok(BuildingRightIdentity {
        id: required_parse(node, COMMON_NS, "idPravoStavby")?,
        ownership_sheet_id: optional_parse(node, COMMON_NS, "lvId")?,
        identifying_parcel: decode_parcel_identity(required_child(
            node,
            COMMON_NS,
            "parcelaIdentifikacni",
        )?)?,
    })
}

pub(crate) fn decode_parcel(node: Node<'_, '_>) -> Result<Parcel> {
    Ok(Parcel {
        id: required_parse(node, COMMON_NS, "idParcely")?,
        kind: ParcelKind::from_raw(required_text(node, COMMON_NS, "parcelaType")?),
        cadastral_area: required_parse(node, COMMON_NS, "katuzeKod")?,
        original_cadastral_area: optional_parse(node, COMMON_NS, "katuzePuv")?,
        root_number: required_parse(node, COMMON_NS, "kmenoveCislo")?,
        subdivision: optional_parse(node, COMMON_NS, "poddeleniCisla")?,
        part: optional_parse(node, COMMON_NS, "dil")?,
        area_determination_code: optional_text(node, COMMON_NS, "zpUrVyKod")?,
        simplified_registration_kind: optional_text(node, COMMON_NS, "typZE")?,
        land_type_code: optional_text(node, COMMON_NS, "drupozKod")?,
        land_use_code: optional_text(node, COMMON_NS, "zpuvypaKod")?,
        parcel_type_code: optional_text(node, COMMON_NS, "typ")?,
        area: required_parse(node, COMMON_NS, "vymera")?,
        ownership_sheet_id: optional_parse(node, COMMON_NS, "lvId")?,
        numbering_kind: ParcelNumberingKind::from_raw(required_text(
            node,
            COMMON_NS,
            "druhCislovaniPar",
        )?),
        building: child(node, COMMON_NS, "stavba")?
            .map(decode_building_identity)
            .transpose()?,
        building_is_part: YesNo::from_raw(required_text(node, COMMON_NS, "stavbaSoucastiParcely")?),
        building_right: child(node, COMMON_NS, "pravoStavby")?
            .map(decode_building_right_identity)
            .transpose()?,
    })
}

pub(crate) fn decode_building(node: Node<'_, '_>) -> Result<Building> {
    Ok(Building {
        id: required_parse(node, COMMON_NS, "idStavby")?,
        building_type: required_parse(node, COMMON_NS, "stavbaTypKod")?,
        municipality_part: child(node, COMMON_NS, "castObce")?
            .map(decode_municipality_part)
            .transpose()?,
        house_number: optional_text(node, COMMON_NS, "cisloDomovni")?,
        use_code: optional_text(node, COMMON_NS, "zpuVyKod")?,
        identifying_parcel: child(node, COMMON_NS, "parcelaIdentifikacni")?
            .map(decode_parcel_identity)
            .transpose()?,
        ownership_sheet_id: optional_parse(node, COMMON_NS, "lvId")?,
        attachment: optional_text(node, COMMON_NS, "jeSoucasti")?.map(BuildingAttachment::from_raw),
        building_right: child(node, COMMON_NS, "pravoStavby")?
            .map(decode_building_right_identity)
            .transpose()?,
        temporary: optional_text(node, COMMON_NS, "docasnaStavba")?.map(YesNo::from_raw),
        defined_units: optional_text(node, COMMON_NS, "vymezeneJednotky")?.map(YesNo::from_raw),
    })
}

pub(crate) fn decode_unit(node: Node<'_, '_>) -> Result<Unit> {
    Ok(Unit {
        id: required_parse(node, COMMON_NS, "idJednotky")?,
        building: decode_building_identity(required_child(node, COMMON_NS, "stavba")?)?,
        unit_type_code: required_text(node, COMMON_NS, "typJedKod")?,
        number: required_parse(node, COMMON_NS, "cislo")?,
        use_code: optional_text(node, COMMON_NS, "zpuVyKod")?,
        ownership_sheet_id: optional_parse(node, COMMON_NS, "lvId")?,
    })
}

pub(crate) fn decode_building_right(node: Node<'_, '_>) -> Result<BuildingRight> {
    let purposes_node = required_child(node, COMMON_NS, "ucelyPravaStavby")?;
    let purposes = children(purposes_node, COMMON_NS, "ucelPravaStavby")
        .map(|purpose| {
            Ok(BuildingRightPurpose {
                code: required_text(purpose, COMMON_NS, "kod")?,
                name: required_text(purpose, COMMON_NS, "nazev")?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    if purposes.is_empty() {
        return Err(Error::invalid_response(
            "building right must contain at least one purpose",
        ));
    }
    let parcels_node = required_child(node, COMMON_NS, "parcely")?;
    let parcels = children(parcels_node, COMMON_NS, "parcela")
        .map(decode_parcel_identity)
        .collect::<Result<Vec<_>>>()?;
    if parcels.is_empty() {
        return Err(Error::invalid_response(
            "building right must contain at least one parcel",
        ));
    }
    Ok(BuildingRight {
        id: required_parse(node, COMMON_NS, "idPravoStavby")?,
        valid_until: required_parse(node, COMMON_NS, "datumKoncePlatnosti")?,
        ownership_sheet_id: optional_parse(node, COMMON_NS, "lvId")?,
        purposes,
        only_identifying_parcel: YesNo::from_raw(required_text(
            node,
            COMMON_NS,
            "pouzeIdentParcela",
        )?),
        parcels,
    })
}

pub(crate) fn decode_subject(node: Node<'_, '_>) -> Result<AuthorizedSubject> {
    let partner1 = child(node, COMMON_NS, "partner1")?
        .map(|partner| decode_subject_base(required_child(partner, COMMON_NS, "OS")?))
        .transpose()?;
    let partner2 = child(node, COMMON_NS, "partner2")?
        .map(|partner| decode_subject_base(required_child(partner, COMMON_NS, "OS")?))
        .transpose()?;
    Ok(AuthorizedSubject {
        base: decode_subject_base(node)?,
        partner1,
        partner2,
    })
}

fn decode_subject_base(node: Node<'_, '_>) -> Result<SubjectBase> {
    Ok(SubjectBase {
        id: required_parse(node, COMMON_NS, "idOS")?,
        name: optional_text(node, COMMON_NS, "nazev")?,
        company_id: optional_parse(node, COMMON_NS, "ico")?,
        company_id_suffix: optional_text(node, COMMON_NS, "doplnekIco")?,
        surname: optional_text(node, COMMON_NS, "prijmeni")?,
        given_name: optional_text(node, COMMON_NS, "jmeno")?,
        title_before: optional_text(node, COMMON_NS, "titulPred")?,
        title_after: optional_text(node, COMMON_NS, "titulZa")?,
        short_personal_id: optional_text(node, COMMON_NS, "rcZkracene")?
            .map(|value| {
                ShortPersonalIdentifier::new(value)
                    .map_err(|_| Error::invalid_response("invalid shortened personal identifier"))
            })
            .transpose()?,
        street: optional_text(node, COMMON_NS, "ulice")?,
        house_number: optional_text(node, COMMON_NS, "cisloDomovni")?,
        orientation_number: optional_text(node, COMMON_NS, "cisloOrientacni")?,
        municipality_part_name: optional_text(node, COMMON_NS, "castObceNazev")?,
        municipality_name: optional_text(node, COMMON_NS, "obecNazev")?,
        city_district_name: optional_text(node, COMMON_NS, "mestskaCastNazev")?,
        city_borough_name: optional_text(node, COMMON_NS, "mestskyObvodNazev")?,
        postal_code: optional_text(node, COMMON_NS, "psc")?,
        kind: SubjectKind::from_raw(required_text(node, COMMON_NS, "vlastnikTyp")?),
        address_number_kind: optional_text(node, COMMON_NS, "typCpCe")?
            .map(AddressNumberKind::from_raw),
        country_name: optional_text(node, COMMON_NS, "statNazev")?,
        district_name: optional_text(node, COMMON_NS, "okresNazev")?,
        characteristic_code: optional_text(node, COMMON_NS, "charOSType")?,
    })
}

pub(crate) fn decode_proceeding(node: Node<'_, '_>) -> Result<Proceeding> {
    Ok(Proceeding {
        id: required_parse(node, COMMON_NS, "idRizeni")?,
        workplace: required_parse(node, COMMON_NS, "praresKod")?,
        proceeding_type: required_text(node, COMMON_NS, "rizeniTyp")?,
        serial_number: required_parse(node, COMMON_NS, "poradoveCislo")?,
        year: required_parse(node, COMMON_NS, "rok")?,
        external_id: optional_text(node, COMMON_NS, "extIdentifikace")?,
        created_at: required_parse::<XmlDateTime>(node, COMMON_NS, "createDate")?,
    })
}

fn decode_document(node: Node<'_, '_>) -> Result<CollectionDocument> {
    Ok(CollectionDocument {
        id: required_parse(node, COMMON_NS, "listinaId")?,
        designation: required_text(node, COMMON_NS, "oznaceniListiny")?,
        executed_at: optional_parse(node, COMMON_NS, "datumVyhotoveni")?,
    })
}

fn decode_geometric_plan(node: Node<'_, '_>) -> Result<GeometricPlan> {
    Ok(GeometricPlan {
        cadastral_area: required_parse(node, COMMON_NS, "katuzeKod")?,
        survey_result_number: required_parse(node, COMMON_NS, "cisloVysledkuZemCinnosti")?,
        document_id: required_parse(node, COMMON_NS, "dokumentId")?,
        file_name: required_text(node, COMMON_NS, "nazevSouboru")?,
        proceeding: decode_proceeding(required_child(node, COMMON_NS, "rizeniPGP")?)?,
        change_proposal_number: optional_text(node, COMMON_NS, "cisloNavrhuZmenyKM")?,
        confirmed_on: optional_parse::<XmlDate>(node, COMMON_NS, "datumPotvrzeniGP")?,
    })
}

pub(crate) fn decode_ownership(node: Node<'_, '_>) -> Result<Ownership> {
    let subject = child(node, COMMON_NS, "osId")?;
    let parcel = child(node, COMMON_NS, "parcelaId")?;
    let building = child(node, COMMON_NS, "budovaId")?;
    let unit = child(node, COMMON_NS, "jednotkaId")?;
    let building_right = child(node, COMMON_NS, "pravoStavbyId")?;
    let target = match (subject, parcel, building, unit, building_right) {
        (Some(value), None, None, None, None) => {
            OwnershipTarget::Subject(parse_node(value, "osId")?)
        }
        (None, Some(value), None, None, None) => {
            OwnershipTarget::Parcel(parse_node(value, "parcelaId")?)
        }
        (None, None, Some(value), None, None) => {
            OwnershipTarget::Building(parse_node(value, "budovaId")?)
        }
        (None, None, None, Some(value), None) => {
            OwnershipTarget::Unit(parse_node(value, "jednotkaId")?)
        }
        (None, None, None, None, Some(value)) => {
            OwnershipTarget::BuildingRight(parse_node(value, "pravoStavbyId")?)
        }
        _ => {
            return Err(Error::invalid_response(
                "ownership must contain exactly one target choice",
            ));
        }
    };
    Ok(Ownership {
        id: required_parse(node, COMMON_NS, "idVlastnictvi")?,
        ownership_sheet_id: required_parse(node, COMMON_NS, "lvId")?,
        share_numerator: required_parse::<FractionComponent>(node, COMMON_NS, "podilCitatel")?,
        share_denominator: required_parse::<FractionComponent>(node, COMMON_NS, "podilJmenovatel")?,
        legal_relation_code: required_text(node, COMMON_NS, "typPravnihoVztahuKod")?,
        cadastral_area: required_parse(node, COMMON_NS, "katuzeKod")?,
        ownership_sheet_number: required_parse(node, COMMON_NS, "lvCislo")?,
        target,
    })
}

#[cfg(test)]
mod tests {
    use roxmltree::Document;

    use super::*;
    use crate::model::search::{
        BuildingRightSearch, BuildingSearch, BuildingSearchKey, BuildingTypeCode,
        CollectionDocumentSearch, GeometricPlanSearch, LegalPersonSearch, ParcelSearch,
        ParcelSearchKey, PersonBirthSelector, PersonOrPartnershipSearch, ProceedingSearch,
        ProceedingSearchKey, SearchCompletion, SearchResponse, SubjectAddressFilter, SubjectSearch,
        UnitSearch, UnitSearchKey,
    };
    use crate::model::{
        BuildingId, BuildingRightId, CadastralAreaCode, CompanyId, MessageLevel,
        MunicipalityPartCode, ParcelId, ParcelNumber, PersonalIdentifier, ProceedingId,
        ServiceMessage, UnitId, WorkplaceCode,
    };

    fn parcel_request(selector: ParcelSearchSelector) -> Request {
        Request::Parcel(ParcelSearch {
            selector,
            valid_at: None,
        })
    }

    fn parcel_key() -> ParcelSearchKey {
        ParcelSearchKey {
            cadastral_area: CadastralAreaCode::new("693936").unwrap(),
            number: ParcelNumber::new(77, Some(2)).unwrap(),
        }
    }

    fn proceeding_key() -> ProceedingSearchKey {
        ProceedingSearchKey::new("V", 123, 2026, WorkplaceCode::new("1").unwrap()).unwrap()
    }

    fn encoded(request: &Request) -> String {
        let mut writer = Writer::new(Vec::new());
        write_request(&mut writer, request).unwrap();
        String::from_utf8(writer.into_inner()).unwrap()
    }

    fn decoded(request: &Request, response_name: &str, contents: &str) -> Result<Response> {
        let xml = format!(
            r#"<{response_name} xmlns="{SEARCH_NS}" xmlns:c="{COMMON_NS}">{contents}</{response_name}>"#
        );
        let document = Document::parse(&xml).unwrap();
        decode_response(request, document.root_element())
    }

    #[test]
    fn all_operations_have_exact_wsdl_names_and_actions() {
        let requests = [
            parcel_request(ParcelSearchSelector::Id(ParcelId::new("1").unwrap())),
            Request::Unit(UnitSearch {
                selector: UnitSearchSelector::Id(UnitId::new("1").unwrap()),
                valid_at: None,
            }),
            Request::Building(BuildingSearch {
                selector: BuildingSearchSelector::Id(BuildingId::new("1").unwrap()),
                valid_at: None,
            }),
            Request::Subject(SubjectSearch {
                selector: SubjectSearchSelector::LegalPerson(
                    LegalPersonSearch::new(Some(CompanyId::new("12345678").unwrap()), None)
                        .unwrap(),
                ),
                valid_at: None,
            }),
            Request::BuildingRight(BuildingRightSearch {
                selector: BuildingRightSelector::Id(BuildingRightId::new("1").unwrap()),
                valid_at: None,
            }),
            Request::Proceeding(ProceedingSearch {
                selector: ProceedingSearchSelector::Id(ProceedingId::new("1").unwrap()),
            }),
            Request::CollectionDocument(CollectionDocumentSearch {
                proceeding: ProceedingSearchSelector::Id(ProceedingId::new("1").unwrap()),
            }),
            Request::GeometricPlan(
                GeometricPlanSearch::new(CadastralAreaCode::new("693936").unwrap(), -12).unwrap(),
            ),
        ];
        let expected = [
            (
                "najdiParcelu",
                "NajdiParceluRequest",
                "NajdiParceluResponse",
                "<parcelaId>1</parcelaId>",
            ),
            (
                "najdiJednotku",
                "NajdiJednotkuRequest",
                "NajdiJednotkuResponse",
                "<jednotkaId>1</jednotkaId>",
            ),
            (
                "najdiStavbu",
                "NajdiStavbuRequest",
                "NajdiStavbuResponse",
                "<stavbaId>1</stavbaId>",
            ),
            (
                "najdiOS",
                "NajdiOSRequest",
                "NajdiOSResponse",
                "<ico>12345678</ico>",
            ),
            (
                "najdiPravoStavby",
                "NajdiPravoStavbyRequest",
                "NajdiPravoStavbyResponse",
                "<pravoStavbyId>1</pravoStavbyId>",
            ),
            (
                "najdiRizeni",
                "NajdiRizeniRequest",
                "NajdiRizeniResponse",
                "<rizeniId>1</rizeniId>",
            ),
            (
                "najdiListinu",
                "NajdiListinuVeSbirceListinRequest",
                "NajdiListinuVeSbirceListinResponse",
                "<rizeniId>1</rizeniId>",
            ),
            (
                "najdiGP",
                "NajdiGPRequest",
                "NajdiGPResponse",
                "<katuzeKod>693936</katuzeKod><cisloVysledkuZemCinnosti>-12</cisloVysledkuZemCinnosti>",
            ),
        ];
        for (request, (action, request_name, response_name, body)) in requests.iter().zip(expected)
        {
            let operation = operation(request);
            assert_eq!(operation.action, format!("{SEARCH_ACTION}:{action}"));
            assert_eq!(operation.request, request_name);
            assert_eq!(operation.response, response_name);
            assert_eq!(encoded(request), body);
        }
    }

    #[test]
    fn request_choices_have_exact_element_names_and_order() {
        let mut parcel = parcel_request(ParcelSearchSelector::Number(parcel_key()));
        if let Request::Parcel(value) = &mut parcel {
            value.valid_at = Some(XmlDateTime::new("2026-01-02T03:04:05Z").unwrap());
        }
        assert_eq!(
            encoded(&parcel),
            "<katastrUzemiKod>693936</katastrUzemiKod><kmenoveCislo>77</kmenoveCislo><poddeleni>2</poddeleni><datumK>2026-01-02T03:04:05Z</datumK>"
        );

        let unit = Request::Unit(UnitSearch {
            selector: UnitSearchSelector::Number(
                UnitSearchKey::new(
                    BuildingSearchSelector::Number(
                        BuildingSearchKey::new(
                            MunicipalityPartCode::new("123").unwrap(),
                            BuildingTypeCode::new("1").unwrap(),
                            45,
                        )
                        .unwrap(),
                    ),
                    None,
                )
                .unwrap(),
            ),
            valid_at: None,
        });
        assert_eq!(
            encoded(&unit),
            "<castObceKod>123</castObceKod><typStavbyKod>1</typStavbyKod><cisloDomovni>45</cisloDomovni>"
        );

        let subject = Request::Subject(SubjectSearch {
            selector: SubjectSearchSelector::PersonOrPartnership(PersonOrPartnershipSearch {
                given_name: Some("Jan".into()),
                surname: Some("Novak".into()),
                birth: Some(PersonBirthSelector::PersonalIdentifier(
                    PersonalIdentifier::new("123456789").unwrap(),
                )),
                address: Some(SubjectAddressFilter {
                    municipality: Some("Praha".into()),
                    house_number: Some(7),
                    ..SubjectAddressFilter::default()
                }),
            }),
            valid_at: None,
        });
        assert_eq!(
            encoded(&subject),
            "<jmeno>Jan</jmeno><prijmeni>Novak</prijmeni><rc>123456789</rc><adresniUdaje><obec>Praha</obec><cisloDomovni>7</cisloDomovni></adresniUdaje>"
        );

        let proceeding = Request::Proceeding(ProceedingSearch {
            selector: ProceedingSearchSelector::Number(proceeding_key()),
        });
        assert_eq!(
            encoded(&proceeding),
            "<rizeniTyp>V</rizeniTyp><poradoveCislo>123</poradoveCislo><rok>2026</rok><praresKod>1</praresKod>"
        );

        let right = Request::BuildingRight(BuildingRightSearch {
            selector: BuildingRightSelector::Parcel(ParcelSearchSelector::Number(parcel_key())),
            valid_at: None,
        });
        assert_eq!(
            encoded(&right),
            "<katastrUzemiKod>693936</katastrUzemiKod><kmenoveCislo>77</kmenoveCislo><poddeleni>2</poddeleni>"
        );
    }

    fn parcel_xml(id: &str, kind: &str) -> String {
        format!(
            "<Parcela><c:idParcely>{id}</c:idParcely><c:parcelaType>{kind}</c:parcelaType><c:katuzeKod>693936</c:katuzeKod><c:kmenoveCislo>77</c:kmenoveCislo><c:vymera>100</c:vymera><c:druhCislovaniPar>1</c:druhCislovaniPar><c:stavbaSoucastiParcely>n</c:stavbaSoucastiParcely></Parcela>"
        )
    }

    #[test]
    fn repeated_results_and_unknown_codes_are_preserved() {
        let request = parcel_request(ParcelSearchSelector::Id(ParcelId::new("1").unwrap()));
        let contents = format!(
            "<ParcelaList>{}{}</ParcelaList>",
            parcel_xml("1", "PKN"),
            parcel_xml("2", "FUTURE")
        );
        let Response::Parcels(result) =
            decoded(&request, "NajdiParceluResponse", &contents).unwrap()
        else {
            panic!("wrong response variant");
        };
        assert_eq!(result.items.len(), 2);
        assert_eq!(result.items[0].kind, ParcelKind::Cadastral);
        assert_eq!(result.items[1].kind, ParcelKind::Other("FUTURE".into()));
        assert_eq!(result.completion, SearchCompletion::Complete);

        let mut response = SearchResponse::Parcels(result);
        response.refine_completion(&[ServiceMessage {
            code: crate::MessageCode::ResultLimitExceeded,
            level: MessageLevel::Warning,
            text: String::new(),
        }]);
        let SearchResponse::Parcels(result) = response else {
            unreachable!()
        };
        assert_eq!(result.completion, SearchCompletion::LimitExceeded);
    }

    #[test]
    fn required_empty_and_missing_lists_are_distinct() {
        let request = parcel_request(ParcelSearchSelector::Id(ParcelId::new("1").unwrap()));
        let Response::Parcels(result) =
            decoded(&request, "NajdiParceluResponse", "<ParcelaList/>").unwrap()
        else {
            panic!("wrong response variant");
        };
        assert!(result.items.is_empty());
        assert_eq!(result.completion, SearchCompletion::Empty);
        assert!(decoded(&request, "NajdiParceluResponse", "").is_err());
    }

    #[test]
    fn optional_document_and_plan_lists_retain_their_cardinality() {
        let documents = Request::CollectionDocument(CollectionDocumentSearch {
            proceeding: ProceedingSearchSelector::Id(ProceedingId::new("1").unwrap()),
        });
        let Response::CollectionDocuments(result) =
            decoded(&documents, "NajdiListinuVeSbirceListinResponse", "").unwrap()
        else {
            panic!("wrong response variant");
        };
        assert!(result.items.is_empty());
        assert!(
            decoded(
                &documents,
                "NajdiListinuVeSbirceListinResponse",
                "<ListinaList/>",
            )
            .is_err()
        );

        let plans = Request::GeometricPlan(
            GeometricPlanSearch::new(CadastralAreaCode::new("693936").unwrap(), 1).unwrap(),
        );
        for contents in ["", "<geometrickePlany/>"] {
            let Response::GeometricPlans(result) =
                decoded(&plans, "NajdiGPResponse", contents).unwrap()
            else {
                panic!("wrong response variant");
            };
            assert!(result.items.is_empty());
        }
    }

    fn proceeding_fields(id: &str) -> String {
        format!(
            "<c:idRizeni>{id}</c:idRizeni><c:praresKod>1</c:praresKod><c:rizeniTyp>V</c:rizeniTyp><c:poradoveCislo>123</c:poradoveCislo><c:rok>2026</c:rok><c:extIdentifikace>external</c:extIdentifikace><c:createDate>2026-01-02T03:04:05+01:00</c:createDate>"
        )
    }

    fn parcel_identity_fields(id: &str) -> String {
        format!(
            "<c:idParcely>{id}</c:idParcely><c:lvId>30</c:lvId><c:katuzeKod>693936</c:katuzeKod><c:kmenoveCislo>77</c:kmenoveCislo><c:parcelaType>PKN</c:parcelaType><c:druhCislovaniPar>1</c:druhCislovaniPar><c:poddeleniCisla>2</c:poddeleniCisla><c:katuzePuv>693900</c:katuzePuv><c:typZE>1</c:typZE><c:dil>1</c:dil>"
        )
    }

    #[test]
    fn buildings_units_and_building_rights_decode_nested_fields() {
        let building = Request::Building(BuildingSearch {
            selector: BuildingSearchSelector::Id(BuildingId::new("20").unwrap()),
            valid_at: None,
        });
        let building_xml = format!(
            "<StavbaList><Stavba><c:idStavby>20</c:idStavby><c:stavbaTypKod>3</c:stavbaTypKod><c:castObce><c:kod>123</c:kod><c:nazev>Cast</c:nazev></c:castObce><c:cisloDomovni>4</c:cisloDomovni><c:zpuVyKod>1234</c:zpuVyKod><c:parcelaIdentifikacni>{}</c:parcelaIdentifikacni><c:lvId>30</c:lvId><c:jeSoucasti>X</c:jeSoucasti><c:pravoStavby><c:idPravoStavby>40</c:idPravoStavby><c:parcelaIdentifikacni>{}</c:parcelaIdentifikacni></c:pravoStavby><c:docasnaStavba>a</c:docasnaStavba><c:vymezeneJednotky>future</c:vymezeneJednotky></Stavba></StavbaList>",
            parcel_identity_fields("1"),
            parcel_identity_fields("2"),
        );
        let Response::Buildings(result) =
            decoded(&building, "NajdiStavbuResponse", &building_xml).unwrap()
        else {
            panic!("wrong response variant");
        };
        let item = &result.items[0];
        assert_eq!(item.municipality_part.as_ref().unwrap().name, "Cast");
        assert_eq!(item.identifying_parcel.as_ref().unwrap().part, Some(1));
        assert_eq!(item.attachment, Some(BuildingAttachment::Other("X".into())));
        assert_eq!(item.defined_units, Some(YesNo::Other("future".into())));
        assert_eq!(item.building_right.as_ref().unwrap().id.as_str(), "40");

        let unit = Request::Unit(UnitSearch {
            selector: UnitSearchSelector::Id(UnitId::new("10").unwrap()),
            valid_at: None,
        });
        let unit_xml = |id: &str, number: u64| {
            format!(
                "<Jednotka><c:idJednotky>{id}</c:idJednotky><c:stavba><c:idStavby>20</c:idStavby><c:typStavbyKod>3</c:typStavbyKod><c:parcelaIdentifikacni>{}</c:parcelaIdentifikacni></c:stavba><c:typJedKod>01</c:typJedKod><c:cislo>{number}</c:cislo><c:zpuVyKod>0001</c:zpuVyKod><c:lvId>30</c:lvId></Jednotka>",
                parcel_identity_fields("1")
            )
        };
        let Response::Units(result) = decoded(
            &unit,
            "NajdiJednotkuResponse",
            &format!(
                "<JednotkaList>{}{}</JednotkaList>",
                unit_xml("10", 7),
                unit_xml("11", 8)
            ),
        )
        .unwrap() else {
            panic!("wrong response variant");
        };
        assert_eq!(result.items.len(), 2);
        assert_eq!(result.items[0].unit_type_code, "01");
        assert_eq!(result.items[0].use_code.as_deref(), Some("0001"));
        assert!(matches!(
            result.items[0].building.location,
            BuildingLocation::Parcel(_)
        ));

        let right = Request::BuildingRight(BuildingRightSearch {
            selector: BuildingRightSelector::Id(BuildingRightId::new("40").unwrap()),
            valid_at: None,
        });
        let right_xml = format!(
            "<PravoStavbyList><pravoStavby><c:idPravoStavby>40</c:idPravoStavby><c:datumKoncePlatnosti>2030-12-31Z</c:datumKoncePlatnosti><c:lvId>30</c:lvId><c:ucelyPravaStavby><c:ucelPravaStavby><c:kod>0</c:kod><c:nazev>First</c:nazev></c:ucelPravaStavby><c:ucelPravaStavby><c:kod>02</c:kod><c:nazev>Second</c:nazev></c:ucelPravaStavby></c:ucelyPravaStavby><c:pouzeIdentParcela>n</c:pouzeIdentParcela><c:parcely><c:parcela>{}</c:parcela><c:parcela>{}</c:parcela></c:parcely></pravoStavby></PravoStavbyList>",
            parcel_identity_fields("1"),
            parcel_identity_fields("2"),
        );
        let Response::BuildingRights(result) =
            decoded(&right, "NajdiPravoStavbyResponse", &right_xml).unwrap()
        else {
            panic!("wrong response variant");
        };
        assert_eq!(result.items[0].valid_until.as_str(), "2030-12-31Z");
        assert_eq!(result.items[0].purposes.len(), 2);
        assert_eq!(result.items[0].purposes[1].code, "02");
        assert_eq!(result.items[0].parcels.len(), 2);
    }

    #[test]
    fn proceedings_documents_and_geometric_plans_decode_dates_and_repetition() {
        let proceeding = Request::Proceeding(ProceedingSearch {
            selector: ProceedingSearchSelector::Id(ProceedingId::new("50").unwrap()),
        });
        let Response::Proceedings(result) = decoded(
            &proceeding,
            "NajdiRizeniResponse",
            &format!(
                "<RizeniList><rizeni>{}</rizeni><rizeni>{}</rizeni></RizeniList>",
                proceeding_fields("50"),
                proceeding_fields("51")
            ),
        )
        .unwrap() else {
            panic!("wrong response variant");
        };
        assert_eq!(result.items.len(), 2);
        assert_eq!(
            result.items[0].created_at.as_str(),
            "2026-01-02T03:04:05+01:00"
        );
        assert_eq!(result.items[0].external_id.as_deref(), Some("external"));

        let documents = Request::CollectionDocument(CollectionDocumentSearch {
            proceeding: ProceedingSearchSelector::Id(ProceedingId::new("50").unwrap()),
        });
        let Response::CollectionDocuments(result) = decoded(
            &documents,
            "NajdiListinuVeSbirceListinResponse",
            "<ListinaList><listina><c:listinaId>60</c:listinaId><c:oznaceniListiny>Deed</c:oznaceniListiny><c:datumVyhotoveni>2026-02-03T04:05:06Z</c:datumVyhotoveni></listina><listina><c:listinaId>61</c:listinaId><c:oznaceniListiny>Other</c:oznaceniListiny></listina></ListinaList>",
        )
        .unwrap()
        else {
            panic!("wrong response variant");
        };
        assert_eq!(result.items.len(), 2);
        assert_eq!(
            result.items[0].executed_at.as_ref().unwrap().as_str(),
            "2026-02-03T04:05:06Z"
        );
        assert!(result.items[1].executed_at.is_none());

        let plans = Request::GeometricPlan(
            GeometricPlanSearch::new(CadastralAreaCode::new("693936").unwrap(), -12).unwrap(),
        );
        let Response::GeometricPlans(result) = decoded(
            &plans,
            "NajdiGPResponse",
            &format!("<geometrickePlany><c:geometrickyPlan><c:katuzeKod>693936</c:katuzeKod><c:cisloVysledkuZemCinnosti>-12</c:cisloVysledkuZemCinnosti><c:dokumentId>70</c:dokumentId><c:nazevSouboru>gp.pdf</c:nazevSouboru><c:rizeniPGP>{}</c:rizeniPGP><c:cisloNavrhuZmenyKM>proposal</c:cisloNavrhuZmenyKM><c:datumPotvrzeniGP>2026-03-01</c:datumPotvrzeniGP></c:geometrickyPlan></geometrickePlany>", proceeding_fields("50")),
        )
        .unwrap()
        else {
            panic!("wrong response variant");
        };
        assert_eq!(result.items[0].survey_result_number, -12);
        assert_eq!(result.items[0].proceeding.id.as_str(), "50");
        assert_eq!(
            result.items[0].confirmed_on.as_ref().unwrap().as_str(),
            "2026-03-01"
        );
    }

    #[test]
    fn unknown_subject_and_boolean_codes_are_open() {
        let request = Request::Subject(SubjectSearch {
            selector: SubjectSearchSelector::LegalPerson(
                LegalPersonSearch::new(Some(CompanyId::new("12345678").unwrap()), None).unwrap(),
            ),
            valid_at: None,
        });
        let Response::Subjects(result) = decoded(
            &request,
            "NajdiOSResponse",
            "<OSList><OS><c:idOS>1</c:idOS><c:rcZkracene>123456</c:rcZkracene><c:cisloDomovni>0007</c:cisloDomovni><c:psc>01000</c:psc><c:vlastnikTyp>NOVA</c:vlastnikTyp><c:typCpCe>3</c:typCpCe><c:partner1><c:OS><c:idOS>2</c:idOS><c:vlastnikTyp>OFO</c:vlastnikTyp></c:OS></c:partner1><c:partner2><c:OS><c:idOS>3</c:idOS><c:vlastnikTyp>OFO</c:vlastnikTyp></c:OS></c:partner2></OS></OSList>",
        ).unwrap() else {
            panic!("wrong response variant");
        };
        let subject = &result.items[0];
        assert_eq!(subject.base.kind, SubjectKind::Other("NOVA".into()));
        assert_eq!(
            subject.base.address_number_kind,
            Some(AddressNumberKind::Other("3".into()))
        );
        assert_eq!(subject.base.house_number.as_deref(), Some("0007"));
        assert_eq!(subject.base.postal_code.as_deref(), Some("01000"));
        assert_eq!(subject.partner1.as_ref().unwrap().id.as_str(), "2");
        assert_eq!(subject.partner2.as_ref().unwrap().id.as_str(), "3");
        assert!(!format!("{subject:?}").contains("123456"));
    }
}
