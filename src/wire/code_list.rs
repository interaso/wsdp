use jiff::civil::Date;
use quick_xml::Writer;
use quick_xml::events::{BytesEnd, BytesStart, Event};
use roxmltree::Node;

use super::soap::{self, XmlResult};
use crate::client::Operation;
use crate::error::{Error, Result};
use crate::model::{
    CadastralArea, CadastralAreaFilter, CadastralAreaHistoryFilter, CityDistrict,
    CityDistrictFilter, CodeList, CodeListDescription, CodeListItem, CodeListRequest,
    CodeListResult, CountryFilter, District, DistrictFilter, HistoricalCadastralArea,
    LegalRelationCode, LegalRelationType, MessageCode, Municipality, MunicipalityFilter,
    MunicipalityPart, MunicipalityPartFilter, ProceedingOperation, Region, ServiceKind,
    ServiceMessageInfo, SimplifiedRegisterNumberingSource, Street, StreetFilter,
    SubjectCharacteristic, Workplace, WorkplaceFilter,
};

const NS: &str = "urn:cz:gov:cuzk:iskn:types:wsdp:ciselnik:3.1";

/// Resolves the exact WSDL operation, document roots, and SOAP action.
pub(crate) fn operation(request: &CodeListRequest) -> Operation {
    let (operation, request_root, response_root) = match request {
        CodeListRequest::CodeListInventory { .. } => (
            "seznamCiselniku",
            "SeznamCiselnikuRequest",
            "SeznamCiselnikuResponse",
        ),
        CodeListRequest::MunicipalityParts { .. } => (
            "seznamCastiObci",
            "SeznamCastiObciRequest",
            "SeznamCastiObciResponse",
        ),
        CodeListRequest::ParcelNumberingMethods => (
            "seznamCislovaniParcel",
            "SeznamCislovaniParcelRequest",
            "SeznamCislovaniParcelResponse",
        ),
        CodeListRequest::LandKinds => (
            "seznamDruhuPozemku",
            "SeznamDruhuPozemkuRequest",
            "SeznamDruhuPozemkuResponse",
        ),
        CodeListRequest::CadastralAreaHistory { .. } => (
            "seznamHistKU",
            "SeznamHistKURequest",
            "SeznamHistKUResponse",
        ),
        CodeListRequest::Regions => ("seznamKraju", "SeznamKrajuRequest", "SeznamKrajuResponse"),
        CodeListRequest::CadastralAreas { .. } => {
            ("seznamKU", "SeznamKURequest", "SeznamKUResponse")
        }
        CodeListRequest::CityDistricts { .. } => (
            "seznamMestskychCasti",
            "SeznamMestskychCastiRequest",
            "SeznamMestskychCastiResponse",
        ),
        CodeListRequest::Municipalities { .. } => {
            ("seznamObci", "SeznamObciRequest", "SeznamObciResponse")
        }
        CodeListRequest::Districts { .. } => (
            "seznamOkresu",
            "SeznamOkresuRequest",
            "SeznamOkresuResponse",
        ),
        CodeListRequest::ProceedingOperations => (
            "seznamOperaciRizeni",
            "SeznamOperaciRizeniRequest",
            "SeznamOperaciRizeniResponse",
        ),
        CodeListRequest::Workplaces { .. } => (
            "seznamPracovist",
            "SeznamPracovistRequest",
            "SeznamPracovistResponse",
        ),
        CodeListRequest::ProceedingSubjects => (
            "seznamPredmetuRizeni",
            "SeznamPredmetuRizeniRequest",
            "SeznamPredmetuRizeniResponse",
        ),
        CodeListRequest::Countries { .. } => {
            ("seznamStatu", "SeznamStatuRequest", "SeznamStatuResponse")
        }
        CodeListRequest::UnitTypes => (
            "seznamTypuJednotek",
            "SeznamTypuJednotekRequest",
            "SeznamTypuJednotekResponse",
        ),
        CodeListRequest::OperationTypes => (
            "seznamTypuOperaci",
            "SeznamTypuOperaciRequest",
            "SeznamTypuOperaciResponse",
        ),
        CodeListRequest::SubjectTypes => (
            "seznamTypuOS",
            "SeznamTypuOSRequest",
            "SeznamTypuOSResponse",
        ),
        CodeListRequest::ParcelTypes => (
            "seznamTypuParcel",
            "SeznamTypuParcelRequest",
            "SeznamTypuParcelResponse",
        ),
        CodeListRequest::LegalRelationTypes => (
            "seznamTypuPravnichVztahu",
            "SeznamTypuPravnichVztahuRequest",
            "SeznamTypuPravnichVztahuResponse",
        ),
        CodeListRequest::ProceedingTypes => (
            "seznamTypuRizeni",
            "SeznamTypuRizeniRequest",
            "SeznamTypuRizeniResponse",
        ),
        CodeListRequest::BuildingTypes => (
            "seznamTypuStaveb",
            "SeznamTypuStavebRequest",
            "SeznamTypuStavebResponse",
        ),
        CodeListRequest::RequestPurposes => {
            ("seznamUcelu", "SeznamUceluRequest", "SeznamUceluResponse")
        }
        CodeListRequest::Streets { .. } => {
            ("seznamUlic", "SeznamUlicRequest", "SeznamUlicResponse")
        }
        CodeListRequest::AreaDeterminationMethods => (
            "seznamUrceniVymery",
            "SeznamUrceniVymeryRequest",
            "SeznamUrceniVymeryResponse",
        ),
        CodeListRequest::UnitUses => (
            "seznamVyuzitiJednotky",
            "SeznamVyuzitiJednotkyRequest",
            "SeznamVyuzitiJednotkyResponse",
        ),
        CodeListRequest::LandUses => (
            "seznamVyuzitiPozemku",
            "SeznamVyuzitiPozemkuRequest",
            "SeznamVyuzitiPozemkuResponse",
        ),
        CodeListRequest::BuildingUses => (
            "seznamVyuzitiStavby",
            "SeznamVyuzitiStavbyRequest",
            "SeznamVyuzitiStavbyResponse",
        ),
        CodeListRequest::ProtectionMethods => (
            "seznamZpusobOchrany",
            "SeznamZpusobOchranyRequest",
            "SeznamZpusobOchranyResponse",
        ),
        CodeListRequest::SimplifiedRegisterNumberingSources => (
            "seznamZdrojuCislovaniZE",
            "SeznamZdrojuCislovaniZERequest",
            "SeznamZdrojuCislovaniZEResponse",
        ),
        CodeListRequest::ServiceMessages => {
            ("seznamZprav", "SeznamZpravRequest", "SeznamZpravResponse")
        }
        CodeListRequest::BuildingRightPurposes => (
            "seznamUceluPS",
            "SeznamUceluPSRequest",
            "SeznamUceluPSResponse",
        ),
        CodeListRequest::ParcelKinds => (
            "seznamTypParcely",
            "SeznamTypParcelyRequest",
            "SeznamTypParcelyResponse",
        ),
        CodeListRequest::SubjectCharacteristics => (
            "seznamCharakteristikOS",
            "SeznamCharakteristikOSRequest",
            "SeznamCharakteristikOSResponse",
        ),
    };
    Operation::new(
        ServiceKind::CodeLists,
        operation,
        request_root,
        response_root,
    )
}

/// Writes the operation's inner XML content; the SOAP envelope owns the request root.
pub(crate) fn write_request(writer: &mut Writer<Vec<u8>>, request: &CodeListRequest) -> XmlResult {
    match request {
        CodeListRequest::CodeListInventory { changed_since } => {
            write_changed_since(writer, *changed_since)?;
        }
        CodeListRequest::MunicipalityParts { filter } => {
            write_choice(
                writer,
                filter.as_ref().map(|filter| match filter {
                    MunicipalityPartFilter::Municipality(code) => {
                        ("kodObce", "odpovida", code.as_str())
                    }
                    MunicipalityPartFilter::Code(code) => {
                        ("kodCastiObce", "odpovida", code.as_str())
                    }
                    MunicipalityPartFilter::NameContains(pattern) => {
                        ("nazevCastiObce", "obsahuje", pattern.as_str())
                    }
                }),
            )?;
        }
        CodeListRequest::CadastralAreaHistory {
            changed_since,
            filter,
        } => {
            write_changed_since(writer, *changed_since)?;
            write_choice(
                writer,
                filter.as_ref().map(|filter| match filter {
                    CadastralAreaHistoryFilter::Code(code) => ("kodKU", "odpovida", code.as_str()),
                    CadastralAreaHistoryFilter::NameContains(pattern) => {
                        ("nazevKU", "obsahuje", pattern.as_str())
                    }
                }),
            )?;
        }
        CodeListRequest::CadastralAreas { filter } => {
            write_choice(
                writer,
                filter.as_ref().map(|filter| match filter {
                    CadastralAreaFilter::Municipality(code) => {
                        ("kodObce", "odpovida", code.as_str())
                    }
                    CadastralAreaFilter::Code(code) => ("kodKU", "odpovida", code.as_str()),
                    CadastralAreaFilter::NameContains(pattern) => {
                        ("nazevKU", "obsahuje", pattern.as_str())
                    }
                }),
            )?;
        }
        CodeListRequest::CityDistricts { filter } => {
            write_choice(
                writer,
                filter.as_ref().map(|filter| match filter {
                    CityDistrictFilter::Municipality(code) => {
                        ("kodObce", "odpovida", code.as_str())
                    }
                    CityDistrictFilter::Code(code) => ("kodMC", "odpovida", code.as_str()),
                    CityDistrictFilter::NameContains(pattern) => {
                        ("nazevMC", "obsahuje", pattern.as_str())
                    }
                }),
            )?;
        }
        CodeListRequest::Municipalities { filter } => {
            write_choice(
                writer,
                filter.as_ref().map(|filter| match filter {
                    MunicipalityFilter::Code(code) => ("kodObce", "odpovida", code.as_str()),
                    MunicipalityFilter::NameContains(pattern) => {
                        ("nazevObce", "obsahuje", pattern.as_str())
                    }
                    MunicipalityFilter::District(code) => ("kodOkresu", "odpovida", code.as_str()),
                }),
            )?;
        }
        CodeListRequest::Districts { filter } => {
            write_choice(
                writer,
                filter.as_ref().map(|filter| match filter {
                    DistrictFilter::Code(code) => ("kodOkresu", "odpovida", code.as_str()),
                    DistrictFilter::NameContains(pattern) => {
                        ("nazevOkresu", "obsahuje", pattern.as_str())
                    }
                    DistrictFilter::NewRegion(code) => {
                        ("kodNovehoKraje", "odpovida", code.as_str())
                    }
                }),
            )?;
        }
        CodeListRequest::Workplaces { filter } => {
            write_choice(
                writer,
                filter.as_ref().map(|filter| match filter {
                    WorkplaceFilter::Code(code) => ("kodKP", "odpovida", code.as_str()),
                    WorkplaceFilter::NameContains(pattern) => {
                        ("nazevKP", "obsahuje", pattern.as_str())
                    }
                    WorkplaceFilter::Email(email) => ("email", "odpovida", email.as_str()),
                    WorkplaceFilter::DataBoxId(id) => ("idDS", "odpovida", id.as_str()),
                }),
            )?;
        }
        CodeListRequest::Countries { filter } => {
            write_choice(
                writer,
                filter.as_ref().map(|filter| match filter {
                    CountryFilter::NameContains(pattern) => {
                        ("nazevStatu", "obsahuje", pattern.as_str())
                    }
                }),
            )?;
        }
        CodeListRequest::Streets { filter } => {
            write_choice(
                writer,
                filter.as_ref().map(|filter| match filter {
                    StreetFilter::Municipality(code) => ("kodObce", "odpovida", code.as_str()),
                    StreetFilter::Code(code) => ("kodUlice", "odpovida", code.as_str()),
                    StreetFilter::NameContains(pattern) => {
                        ("nazevUlice", "obsahuje", pattern.as_str())
                    }
                }),
            )?;
        }
        CodeListRequest::ParcelNumberingMethods
        | CodeListRequest::LandKinds
        | CodeListRequest::Regions
        | CodeListRequest::ProceedingOperations
        | CodeListRequest::ProceedingSubjects
        | CodeListRequest::UnitTypes
        | CodeListRequest::OperationTypes
        | CodeListRequest::SubjectTypes
        | CodeListRequest::ParcelTypes
        | CodeListRequest::LegalRelationTypes
        | CodeListRequest::ProceedingTypes
        | CodeListRequest::BuildingTypes
        | CodeListRequest::RequestPurposes
        | CodeListRequest::AreaDeterminationMethods
        | CodeListRequest::UnitUses
        | CodeListRequest::LandUses
        | CodeListRequest::BuildingUses
        | CodeListRequest::ProtectionMethods
        | CodeListRequest::SimplifiedRegisterNumberingSources
        | CodeListRequest::ServiceMessages
        | CodeListRequest::BuildingRightPurposes
        | CodeListRequest::ParcelKinds
        | CodeListRequest::SubjectCharacteristics => {}
    }
    Ok(())
}

fn write_changed_since(writer: &mut Writer<Vec<u8>>, changed_since: Option<Date>) -> XmlResult {
    if let Some(date) = changed_since {
        soap::text_element(writer, "zmenyOd", &date.to_string())?;
    }
    Ok(())
}

fn write_choice(
    writer: &mut Writer<Vec<u8>>,
    choice: Option<(&'static str, &'static str, &str)>,
) -> XmlResult {
    let Some((condition, operator, value)) = choice else {
        return Ok(());
    };
    writer.write_event(Event::Start(BytesStart::new("kde")))?;
    writer.write_event(Event::Start(BytesStart::new(condition)))?;
    soap::text_element(writer, operator, value)?;
    writer.write_event(Event::End(BytesEnd::new(condition)))?;
    writer.write_event(Event::End(BytesEnd::new("kde")))?;
    Ok(())
}

/// Decodes the inner SOAP response according to the originating request variant.
pub(crate) fn decode_response(
    response: Node<'_, '_>,
    request: &CodeListRequest,
) -> Result<CodeListResult> {
    match request {
        CodeListRequest::CodeListInventory { .. } => Ok(CodeListResult::CodeListInventory(
            decode_list(response, "ciselnik", decode_description)?,
        )),
        CodeListRequest::MunicipalityParts { .. } => Ok(CodeListResult::MunicipalityParts(
            decode_list(response, "castObce", decode_municipality_part)?,
        )),
        CodeListRequest::ParcelNumberingMethods => Ok(CodeListResult::ParcelNumberingMethods(
            decode_list(response, "cislovaniParcely", decode_item)?,
        )),
        CodeListRequest::LandKinds => Ok(CodeListResult::LandKinds(decode_list(
            response,
            "druhPozemku",
            decode_item,
        )?)),
        CodeListRequest::CadastralAreaHistory { .. } => Ok(CodeListResult::CadastralAreaHistory(
            decode_list(response, "katastralniUzemi", decode_historical_area)?,
        )),
        CodeListRequest::Regions => Ok(CodeListResult::Regions(decode_list(
            response,
            "kraj",
            decode_region,
        )?)),
        CodeListRequest::CadastralAreas { .. } => Ok(CodeListResult::CadastralAreas(decode_list(
            response,
            "katastralniUzemi",
            decode_cadastral_area,
        )?)),
        CodeListRequest::CityDistricts { .. } => Ok(CodeListResult::CityDistricts(decode_list(
            response,
            "mestskaCast",
            decode_city_district,
        )?)),
        CodeListRequest::Municipalities { .. } => Ok(CodeListResult::Municipalities(decode_list(
            response,
            "obec",
            decode_municipality,
        )?)),
        CodeListRequest::Districts { .. } => Ok(CodeListResult::Districts(decode_list(
            response,
            "okres",
            decode_district,
        )?)),
        CodeListRequest::ProceedingOperations => Ok(CodeListResult::ProceedingOperations(
            decode_list(response, "operaceRizeni", decode_proceeding_operation)?,
        )),
        CodeListRequest::Workplaces { .. } => Ok(CodeListResult::Workplaces(decode_list(
            response,
            "pracoviste",
            decode_workplace,
        )?)),
        CodeListRequest::ProceedingSubjects => Ok(CodeListResult::ProceedingSubjects(decode_list(
            response,
            "predmetRizeni",
            decode_item,
        )?)),
        CodeListRequest::Countries { .. } => Ok(CodeListResult::Countries(decode_list(
            response,
            "stat",
            decode_item,
        )?)),
        CodeListRequest::UnitTypes => Ok(CodeListResult::UnitTypes(decode_list(
            response,
            "typJednotky",
            decode_item,
        )?)),
        CodeListRequest::OperationTypes => Ok(CodeListResult::OperationTypes(decode_list(
            response,
            "typOperace",
            decode_item,
        )?)),
        CodeListRequest::SubjectTypes => Ok(CodeListResult::SubjectTypes(decode_list(
            response,
            "typOS",
            decode_item,
        )?)),
        CodeListRequest::ParcelTypes => Ok(CodeListResult::ParcelTypes(decode_list(
            response,
            "typParcely",
            decode_item,
        )?)),
        CodeListRequest::LegalRelationTypes => Ok(CodeListResult::LegalRelationTypes(decode_list(
            response,
            "typPravnihoVztahu",
            decode_legal_relation,
        )?)),
        CodeListRequest::ProceedingTypes => Ok(CodeListResult::ProceedingTypes(decode_list(
            response,
            "typRizeni",
            decode_item,
        )?)),
        CodeListRequest::BuildingTypes => Ok(CodeListResult::BuildingTypes(decode_list(
            response,
            "typStavby",
            decode_item,
        )?)),
        CodeListRequest::RequestPurposes => Ok(CodeListResult::RequestPurposes(decode_list(
            response,
            "ucel",
            decode_item,
        )?)),
        CodeListRequest::Streets { .. } => Ok(CodeListResult::Streets(decode_list(
            response,
            "ulice",
            decode_street,
        )?)),
        CodeListRequest::AreaDeterminationMethods => Ok(CodeListResult::AreaDeterminationMethods(
            decode_list(response, "urceniVymery", decode_item)?,
        )),
        CodeListRequest::UnitUses => Ok(CodeListResult::UnitUses(decode_list(
            response,
            "vyuzitiJednotky",
            decode_item,
        )?)),
        CodeListRequest::LandUses => Ok(CodeListResult::LandUses(decode_list(
            response,
            "vyuzitiPozemku",
            decode_item,
        )?)),
        CodeListRequest::BuildingUses => Ok(CodeListResult::BuildingUses(decode_list(
            response,
            "vyuzitiStavby",
            decode_item,
        )?)),
        CodeListRequest::ProtectionMethods => Ok(CodeListResult::ProtectionMethods(decode_list(
            response,
            "zpusobOchrany",
            decode_item,
        )?)),
        CodeListRequest::SimplifiedRegisterNumberingSources => {
            Ok(CodeListResult::SimplifiedRegisterNumberingSources(
                decode_list(response, "zdrojCislovaniZE", decode_numbering_source)?,
            ))
        }
        CodeListRequest::ServiceMessages => Ok(CodeListResult::ServiceMessages(decode_list(
            response,
            "zprava",
            decode_service_message,
        )?)),
        CodeListRequest::BuildingRightPurposes => Ok(CodeListResult::BuildingRightPurposes(
            decode_list(response, "ucelPS", decode_item)?,
        )),
        CodeListRequest::ParcelKinds => Ok(CodeListResult::ParcelKinds(decode_list(
            response,
            "typParcely",
            decode_item,
        )?)),
        CodeListRequest::SubjectCharacteristics => Ok(CodeListResult::SubjectCharacteristics(
            decode_list(response, "charakteristikaOS", decode_subject_characteristic)?,
        )),
    }
}

fn decode_list<T>(
    response: Node<'_, '_>,
    item_name: &'static str,
    mut decode_item: impl FnMut(Node<'_, '_>) -> Result<T>,
) -> Result<CodeList<T>> {
    let items = soap::children(response, NS, item_name)
        .map(&mut decode_item)
        .collect::<Result<Vec<_>>>()?;
    let last_updated = response
        .attribute("posledniAktualizaceCiselniku")
        .map(|value| parse_date(value, "posledniAktualizaceCiselniku"))
        .transpose()?;
    Ok(CodeList {
        items,
        last_updated,
    })
}

fn required_element<'a, 'input>(node: Node<'a, 'input>, name: &str) -> Result<Node<'a, 'input>> {
    soap::child(node, NS, name)?.ok_or_else(|| {
        Error::invalid_response(format!("required code-list element {name} is missing"))
    })
}

fn required_text(node: Node<'_, '_>, name: &str) -> Result<String> {
    let element = required_element(node, name)?;
    if soap::is_nil(element) {
        return Err(Error::invalid_response(format!(
            "required code-list element {name} is nil"
        )));
    }
    soap::simple_content(element, name)
}

fn optional_text(node: Node<'_, '_>, name: &str) -> Result<Option<String>> {
    let Some(element) = soap::child(node, NS, name)? else {
        return Ok(None);
    };
    if soap::is_nil(element) {
        return Ok(None);
    }
    soap::simple_content(element, name).map(Some)
}

fn required_code<T>(node: Node<'_, '_>, name: &str) -> Result<T>
where
    T: std::str::FromStr,
{
    required_text(node, name)?
        .parse::<T>()
        .map_err(|_| Error::invalid_response(format!("invalid code-list value in {name}")))
}

fn optional_code<T>(node: Node<'_, '_>, name: &str) -> Result<Option<T>>
where
    T: std::str::FromStr,
{
    optional_text(node, name)?
        .map(|value| {
            value
                .parse::<T>()
                .map_err(|_| Error::invalid_response(format!("invalid code-list value in {name}")))
        })
        .transpose()
}

fn required_date(node: Node<'_, '_>, name: &str) -> Result<Date> {
    parse_date(&required_text(node, name)?, name)
}

fn optional_date(node: Node<'_, '_>, name: &str) -> Result<Option<Date>> {
    optional_text(node, name)?
        .map(|value| parse_date(&value, name))
        .transpose()
}

fn parse_date(value: &str, name: &str) -> Result<Date> {
    value.trim().parse::<Date>().map_err(|_| {
        Error::invalid_response(format!("invalid xs:date in code-list element {name}"))
    })
}

fn decode_item(node: Node<'_, '_>) -> Result<CodeListItem> {
    Ok(CodeListItem {
        code: required_text(node, "kod")?,
        name: required_text(node, "nazev")?,
    })
}

fn decode_description(node: Node<'_, '_>) -> Result<CodeListDescription> {
    Ok(CodeListDescription {
        name: required_text(node, "nazev")?,
        last_changed: required_date(node, "datumPosledniZmeny")?,
    })
}

fn decode_region(node: Node<'_, '_>) -> Result<Region> {
    Ok(Region {
        code: required_code(node, "kod")?,
        name: required_text(node, "nazev")?,
    })
}

fn decode_cadastral_area(node: Node<'_, '_>) -> Result<CadastralArea> {
    Ok(CadastralArea {
        code: required_code(node, "kod")?,
        name: required_text(node, "nazev")?,
        municipality_code: optional_code(node, "kodObce")?,
    })
}

fn decode_historical_area(node: Node<'_, '_>) -> Result<HistoricalCadastralArea> {
    Ok(HistoricalCadastralArea {
        code: required_code(node, "kod")?,
        name: required_text(node, "nazev")?,
        municipality_code: optional_code(node, "obecKod")?,
        valid_from: required_date(node, "platnostOd")?,
        valid_to: optional_date(node, "platnostDo")?,
    })
}

fn decode_city_district(node: Node<'_, '_>) -> Result<CityDistrict> {
    Ok(CityDistrict {
        code: required_code(node, "kod")?,
        name: required_text(node, "nazev")?,
        municipality_code: required_code(node, "kodObce")?,
    })
}

fn decode_municipality(node: Node<'_, '_>) -> Result<Municipality> {
    Ok(Municipality {
        code: required_code(node, "kod")?,
        name: required_text(node, "nazev")?,
        district_code: required_code(node, "okresKod")?,
    })
}

fn decode_municipality_part(node: Node<'_, '_>) -> Result<MunicipalityPart> {
    Ok(MunicipalityPart {
        code: required_code(node, "kod")?,
        name: required_text(node, "nazev")?,
        municipality_code: required_code(node, "obecKod")?,
    })
}

fn decode_district(node: Node<'_, '_>) -> Result<District> {
    Ok(District {
        code: required_code(node, "kod")?,
        name: required_text(node, "nazev")?,
        new_region_code: required_code(node, "nkrajKod")?,
    })
}

fn decode_street(node: Node<'_, '_>) -> Result<Street> {
    Ok(Street {
        code: required_code(node, "kod")?,
        name: required_text(node, "nazev")?,
        municipality_code: required_code(node, "kodObce")?,
    })
}

fn decode_proceeding_operation(node: Node<'_, '_>) -> Result<ProceedingOperation> {
    Ok(ProceedingOperation {
        code: required_text(node, "kod")?,
        name: required_text(node, "nazev")?,
        state: optional_text(node, "stav")?,
    })
}

fn decode_workplace(node: Node<'_, '_>) -> Result<Workplace> {
    Ok(Workplace {
        code: required_code(node, "kod")?,
        name: required_text(node, "nazev")?,
        new_region_code: optional_code(node, "nkrajKod")?,
        email: optional_text(node, "email")?,
        data_box_id: optional_text(node, "datovaSchranka")?,
        street_name: optional_text(node, "nazevUlice")?,
        house_number: optional_text(node, "cisloDomovni")?,
        orientation_number: optional_text(node, "cisloOrientacni")?,
        postal_code: optional_text(node, "psc")?,
        municipality: optional_text(node, "obec")?,
        municipality_part: optional_text(node, "castObce")?,
        district: optional_text(node, "okres")?,
    })
}

fn decode_legal_relation(node: Node<'_, '_>) -> Result<LegalRelationType> {
    let is_ownership = match required_text(node, "vlastnictvi")?.trim() {
        "true" | "1" => true,
        "false" | "0" => false,
        _ => {
            return Err(Error::invalid_response(
                "invalid xs:boolean in code-list element vlastnictvi",
            ));
        }
    };
    Ok(LegalRelationType {
        code: LegalRelationCode::new(required_text(node, "kod")?),
        name: required_text(node, "nazev")?,
        is_ownership,
    })
}

fn decode_numbering_source(node: Node<'_, '_>) -> Result<SimplifiedRegisterNumberingSource> {
    Ok(SimplifiedRegisterNumberingSource {
        code: required_text(node, "kod")?,
        name: required_text(node, "nazev")?,
        abbreviation: required_text(node, "zkratka")?,
    })
}

fn decode_service_message(node: Node<'_, '_>) -> Result<ServiceMessageInfo> {
    Ok(ServiceMessageInfo {
        code: MessageCode::from_raw(required_text(node, "kod")?),
        kind: required_text(node, "typ")?,
        name: required_text(node, "nazev")?,
    })
}

fn decode_subject_characteristic(node: Node<'_, '_>) -> Result<SubjectCharacteristic> {
    Ok(SubjectCharacteristic {
        code: required_text(node, "kod")?,
        name: required_text(node, "nazev")?,
        abbreviation: optional_text(node, "zkratka")?,
        alv_abbreviation: optional_text(node, "zkratkaALV")?,
    })
}

#[cfg(test)]
mod tests {
    use roxmltree::Document;

    use super::*;
    use crate::model::{
        CadastralAreaCode, CityDistrictCode, DistrictCode, MunicipalityCode, MunicipalityPartCode,
        NewRegionCode, StreetCode, WorkplaceCode,
    };

    fn date(value: &str) -> Date {
        value.parse().unwrap()
    }

    fn all_requests() -> Vec<CodeListRequest> {
        vec![
            CodeListRequest::CodeListInventory {
                changed_since: None,
            },
            CodeListRequest::MunicipalityParts { filter: None },
            CodeListRequest::ParcelNumberingMethods,
            CodeListRequest::LandKinds,
            CodeListRequest::CadastralAreaHistory {
                changed_since: None,
                filter: None,
            },
            CodeListRequest::Regions,
            CodeListRequest::CadastralAreas { filter: None },
            CodeListRequest::CityDistricts { filter: None },
            CodeListRequest::Municipalities { filter: None },
            CodeListRequest::Districts { filter: None },
            CodeListRequest::ProceedingOperations,
            CodeListRequest::Workplaces { filter: None },
            CodeListRequest::ProceedingSubjects,
            CodeListRequest::Countries { filter: None },
            CodeListRequest::UnitTypes,
            CodeListRequest::OperationTypes,
            CodeListRequest::SubjectTypes,
            CodeListRequest::ParcelTypes,
            CodeListRequest::LegalRelationTypes,
            CodeListRequest::ProceedingTypes,
            CodeListRequest::BuildingTypes,
            CodeListRequest::RequestPurposes,
            CodeListRequest::Streets { filter: None },
            CodeListRequest::AreaDeterminationMethods,
            CodeListRequest::UnitUses,
            CodeListRequest::LandUses,
            CodeListRequest::BuildingUses,
            CodeListRequest::ProtectionMethods,
            CodeListRequest::SimplifiedRegisterNumberingSources,
            CodeListRequest::ServiceMessages,
            CodeListRequest::BuildingRightPurposes,
            CodeListRequest::ParcelKinds,
            CodeListRequest::SubjectCharacteristics,
        ]
    }

    fn render_request(request: &CodeListRequest) -> String {
        let mut writer = Writer::new(Vec::new());
        write_request(&mut writer, request).unwrap();
        String::from_utf8(writer.into_inner()).unwrap()
    }

    fn decode_xml(
        request: &CodeListRequest,
        attributes: &str,
        contents: &str,
    ) -> Result<CodeListResult> {
        let response = operation(request).response;
        let xml =
            format!(r#"<{response} xmlns="{NS}"{attributes}><vysledek/>{contents}</{response}>"#);
        let document = Document::parse(&xml).unwrap();
        decode_response(document.root_element(), request)
    }

    fn item_count(result: &CodeListResult) -> usize {
        match result {
            CodeListResult::CodeListInventory(list) => list.items.len(),
            CodeListResult::MunicipalityParts(list) => list.items.len(),
            CodeListResult::ParcelNumberingMethods(list) => list.items.len(),
            CodeListResult::LandKinds(list) => list.items.len(),
            CodeListResult::CadastralAreaHistory(list) => list.items.len(),
            CodeListResult::Regions(list) => list.items.len(),
            CodeListResult::CadastralAreas(list) => list.items.len(),
            CodeListResult::CityDistricts(list) => list.items.len(),
            CodeListResult::Municipalities(list) => list.items.len(),
            CodeListResult::Districts(list) => list.items.len(),
            CodeListResult::ProceedingOperations(list) => list.items.len(),
            CodeListResult::Workplaces(list) => list.items.len(),
            CodeListResult::ProceedingSubjects(list) => list.items.len(),
            CodeListResult::Countries(list) => list.items.len(),
            CodeListResult::UnitTypes(list) => list.items.len(),
            CodeListResult::OperationTypes(list) => list.items.len(),
            CodeListResult::SubjectTypes(list) => list.items.len(),
            CodeListResult::ParcelTypes(list) => list.items.len(),
            CodeListResult::LegalRelationTypes(list) => list.items.len(),
            CodeListResult::ProceedingTypes(list) => list.items.len(),
            CodeListResult::BuildingTypes(list) => list.items.len(),
            CodeListResult::RequestPurposes(list) => list.items.len(),
            CodeListResult::Streets(list) => list.items.len(),
            CodeListResult::AreaDeterminationMethods(list) => list.items.len(),
            CodeListResult::UnitUses(list) => list.items.len(),
            CodeListResult::LandUses(list) => list.items.len(),
            CodeListResult::BuildingUses(list) => list.items.len(),
            CodeListResult::ProtectionMethods(list) => list.items.len(),
            CodeListResult::SimplifiedRegisterNumberingSources(list) => list.items.len(),
            CodeListResult::ServiceMessages(list) => list.items.len(),
            CodeListResult::BuildingRightPurposes(list) => list.items.len(),
            CodeListResult::ParcelKinds(list) => list.items.len(),
            CodeListResult::SubjectCharacteristics(list) => list.items.len(),
        }
    }

    fn result_operation(result: &CodeListResult) -> &'static str {
        match result {
            CodeListResult::CodeListInventory(_) => "seznamCiselniku",
            CodeListResult::MunicipalityParts(_) => "seznamCastiObci",
            CodeListResult::ParcelNumberingMethods(_) => "seznamCislovaniParcel",
            CodeListResult::LandKinds(_) => "seznamDruhuPozemku",
            CodeListResult::CadastralAreaHistory(_) => "seznamHistKU",
            CodeListResult::Regions(_) => "seznamKraju",
            CodeListResult::CadastralAreas(_) => "seznamKU",
            CodeListResult::CityDistricts(_) => "seznamMestskychCasti",
            CodeListResult::Municipalities(_) => "seznamObci",
            CodeListResult::Districts(_) => "seznamOkresu",
            CodeListResult::ProceedingOperations(_) => "seznamOperaciRizeni",
            CodeListResult::Workplaces(_) => "seznamPracovist",
            CodeListResult::ProceedingSubjects(_) => "seznamPredmetuRizeni",
            CodeListResult::Countries(_) => "seznamStatu",
            CodeListResult::UnitTypes(_) => "seznamTypuJednotek",
            CodeListResult::OperationTypes(_) => "seznamTypuOperaci",
            CodeListResult::SubjectTypes(_) => "seznamTypuOS",
            CodeListResult::ParcelTypes(_) => "seznamTypuParcel",
            CodeListResult::LegalRelationTypes(_) => "seznamTypuPravnichVztahu",
            CodeListResult::ProceedingTypes(_) => "seznamTypuRizeni",
            CodeListResult::BuildingTypes(_) => "seznamTypuStaveb",
            CodeListResult::RequestPurposes(_) => "seznamUcelu",
            CodeListResult::Streets(_) => "seznamUlic",
            CodeListResult::AreaDeterminationMethods(_) => "seznamUrceniVymery",
            CodeListResult::UnitUses(_) => "seznamVyuzitiJednotky",
            CodeListResult::LandUses(_) => "seznamVyuzitiPozemku",
            CodeListResult::BuildingUses(_) => "seznamVyuzitiStavby",
            CodeListResult::ProtectionMethods(_) => "seznamZpusobOchrany",
            CodeListResult::SimplifiedRegisterNumberingSources(_) => "seznamZdrojuCislovaniZE",
            CodeListResult::ServiceMessages(_) => "seznamZprav",
            CodeListResult::BuildingRightPurposes(_) => "seznamUceluPS",
            CodeListResult::ParcelKinds(_) => "seznamTypParcely",
            CodeListResult::SubjectCharacteristics(_) => "seznamCharakteristikOS",
        }
    }

    #[test]
    fn all_33_operations_have_exact_wsdl_roots_and_actions() {
        let expected = [
            (
                "seznamCiselniku",
                "SeznamCiselnikuRequest",
                "SeznamCiselnikuResponse",
            ),
            (
                "seznamCastiObci",
                "SeznamCastiObciRequest",
                "SeznamCastiObciResponse",
            ),
            (
                "seznamCislovaniParcel",
                "SeznamCislovaniParcelRequest",
                "SeznamCislovaniParcelResponse",
            ),
            (
                "seznamDruhuPozemku",
                "SeznamDruhuPozemkuRequest",
                "SeznamDruhuPozemkuResponse",
            ),
            (
                "seznamHistKU",
                "SeznamHistKURequest",
                "SeznamHistKUResponse",
            ),
            ("seznamKraju", "SeznamKrajuRequest", "SeznamKrajuResponse"),
            ("seznamKU", "SeznamKURequest", "SeznamKUResponse"),
            (
                "seznamMestskychCasti",
                "SeznamMestskychCastiRequest",
                "SeznamMestskychCastiResponse",
            ),
            ("seznamObci", "SeznamObciRequest", "SeznamObciResponse"),
            (
                "seznamOkresu",
                "SeznamOkresuRequest",
                "SeznamOkresuResponse",
            ),
            (
                "seznamOperaciRizeni",
                "SeznamOperaciRizeniRequest",
                "SeznamOperaciRizeniResponse",
            ),
            (
                "seznamPracovist",
                "SeznamPracovistRequest",
                "SeznamPracovistResponse",
            ),
            (
                "seznamPredmetuRizeni",
                "SeznamPredmetuRizeniRequest",
                "SeznamPredmetuRizeniResponse",
            ),
            ("seznamStatu", "SeznamStatuRequest", "SeznamStatuResponse"),
            (
                "seznamTypuJednotek",
                "SeznamTypuJednotekRequest",
                "SeznamTypuJednotekResponse",
            ),
            (
                "seznamTypuOperaci",
                "SeznamTypuOperaciRequest",
                "SeznamTypuOperaciResponse",
            ),
            (
                "seznamTypuOS",
                "SeznamTypuOSRequest",
                "SeznamTypuOSResponse",
            ),
            (
                "seznamTypuParcel",
                "SeznamTypuParcelRequest",
                "SeznamTypuParcelResponse",
            ),
            (
                "seznamTypuPravnichVztahu",
                "SeznamTypuPravnichVztahuRequest",
                "SeznamTypuPravnichVztahuResponse",
            ),
            (
                "seznamTypuRizeni",
                "SeznamTypuRizeniRequest",
                "SeznamTypuRizeniResponse",
            ),
            (
                "seznamTypuStaveb",
                "SeznamTypuStavebRequest",
                "SeznamTypuStavebResponse",
            ),
            ("seznamUcelu", "SeznamUceluRequest", "SeznamUceluResponse"),
            ("seznamUlic", "SeznamUlicRequest", "SeznamUlicResponse"),
            (
                "seznamUrceniVymery",
                "SeznamUrceniVymeryRequest",
                "SeznamUrceniVymeryResponse",
            ),
            (
                "seznamVyuzitiJednotky",
                "SeznamVyuzitiJednotkyRequest",
                "SeznamVyuzitiJednotkyResponse",
            ),
            (
                "seznamVyuzitiPozemku",
                "SeznamVyuzitiPozemkuRequest",
                "SeznamVyuzitiPozemkuResponse",
            ),
            (
                "seznamVyuzitiStavby",
                "SeznamVyuzitiStavbyRequest",
                "SeznamVyuzitiStavbyResponse",
            ),
            (
                "seznamZpusobOchrany",
                "SeznamZpusobOchranyRequest",
                "SeznamZpusobOchranyResponse",
            ),
            (
                "seznamZdrojuCislovaniZE",
                "SeznamZdrojuCislovaniZERequest",
                "SeznamZdrojuCislovaniZEResponse",
            ),
            ("seznamZprav", "SeznamZpravRequest", "SeznamZpravResponse"),
            (
                "seznamUceluPS",
                "SeznamUceluPSRequest",
                "SeznamUceluPSResponse",
            ),
            (
                "seznamTypParcely",
                "SeznamTypParcelyRequest",
                "SeznamTypParcelyResponse",
            ),
            (
                "seznamCharakteristikOS",
                "SeznamCharakteristikOSRequest",
                "SeznamCharakteristikOSResponse",
            ),
        ];
        let requests = all_requests();
        assert_eq!(requests.len(), 33);
        assert_eq!(expected.len(), requests.len());
        for (request, (name, request_root, response_root)) in requests.iter().zip(expected) {
            let operation = operation(request);
            assert_eq!(operation.service, ServiceKind::CodeLists);
            assert_eq!(operation.request, request_root);
            assert_eq!(operation.response, response_root);
            assert_eq!(
                operation.action,
                format!("urn:cz:gov:cuzk:iskn:services:wsdp:ciselnik:{name}")
            );
        }
        assert_eq!(
            operation(&CodeListRequest::ParcelTypes).request,
            "SeznamTypuParcelRequest"
        );
        assert_eq!(
            operation(&CodeListRequest::ParcelKinds).request,
            "SeznamTypParcelyRequest"
        );
    }

    #[test]
    fn empty_requests_write_no_inner_elements() {
        for request in all_requests() {
            assert_eq!(render_request(&request), "");
        }
    }

    #[test]
    fn every_filter_choice_writes_its_exact_xsd_shape() {
        let cases = vec![
            (
                CodeListRequest::MunicipalityParts {
                    filter: Some(MunicipalityPartFilter::Municipality(
                        MunicipalityCode::new("000123").unwrap(),
                    )),
                },
                "<kde><kodObce><odpovida>000123</odpovida></kodObce></kde>",
            ),
            (
                CodeListRequest::MunicipalityParts {
                    filter: Some(MunicipalityPartFilter::Code(
                        MunicipalityPartCode::new("000456").unwrap(),
                    )),
                },
                "<kde><kodCastiObce><odpovida>000456</odpovida></kodCastiObce></kde>",
            ),
            (
                CodeListRequest::MunicipalityParts {
                    filter: Some(MunicipalityPartFilter::NameContains("part".to_owned())),
                },
                "<kde><nazevCastiObce><obsahuje>part</obsahuje></nazevCastiObce></kde>",
            ),
            (
                CodeListRequest::CadastralAreaHistory {
                    changed_since: None,
                    filter: Some(CadastralAreaHistoryFilter::Code(
                        CadastralAreaCode::new("000789").unwrap(),
                    )),
                },
                "<kde><kodKU><odpovida>000789</odpovida></kodKU></kde>",
            ),
            (
                CodeListRequest::CadastralAreaHistory {
                    changed_since: None,
                    filter: Some(CadastralAreaHistoryFilter::NameContains("old".to_owned())),
                },
                "<kde><nazevKU><obsahuje>old</obsahuje></nazevKU></kde>",
            ),
            (
                CodeListRequest::CadastralAreas {
                    filter: Some(CadastralAreaFilter::Municipality(
                        MunicipalityCode::new("123").unwrap(),
                    )),
                },
                "<kde><kodObce><odpovida>123</odpovida></kodObce></kde>",
            ),
            (
                CodeListRequest::CadastralAreas {
                    filter: Some(CadastralAreaFilter::Code(
                        CadastralAreaCode::new("456").unwrap(),
                    )),
                },
                "<kde><kodKU><odpovida>456</odpovida></kodKU></kde>",
            ),
            (
                CodeListRequest::CadastralAreas {
                    filter: Some(CadastralAreaFilter::NameContains("area".to_owned())),
                },
                "<kde><nazevKU><obsahuje>area</obsahuje></nazevKU></kde>",
            ),
            (
                CodeListRequest::CityDistricts {
                    filter: Some(CityDistrictFilter::Municipality(
                        MunicipalityCode::new("123").unwrap(),
                    )),
                },
                "<kde><kodObce><odpovida>123</odpovida></kodObce></kde>",
            ),
            (
                CodeListRequest::CityDistricts {
                    filter: Some(CityDistrictFilter::Code(
                        CityDistrictCode::new("456").unwrap(),
                    )),
                },
                "<kde><kodMC><odpovida>456</odpovida></kodMC></kde>",
            ),
            (
                CodeListRequest::CityDistricts {
                    filter: Some(CityDistrictFilter::NameContains("district".to_owned())),
                },
                "<kde><nazevMC><obsahuje>district</obsahuje></nazevMC></kde>",
            ),
            (
                CodeListRequest::Municipalities {
                    filter: Some(MunicipalityFilter::Code(
                        MunicipalityCode::new("123").unwrap(),
                    )),
                },
                "<kde><kodObce><odpovida>123</odpovida></kodObce></kde>",
            ),
            (
                CodeListRequest::Municipalities {
                    filter: Some(MunicipalityFilter::NameContains("town".to_owned())),
                },
                "<kde><nazevObce><obsahuje>town</obsahuje></nazevObce></kde>",
            ),
            (
                CodeListRequest::Municipalities {
                    filter: Some(MunicipalityFilter::District(
                        DistrictCode::new("456").unwrap(),
                    )),
                },
                "<kde><kodOkresu><odpovida>456</odpovida></kodOkresu></kde>",
            ),
            (
                CodeListRequest::Districts {
                    filter: Some(DistrictFilter::Code(DistrictCode::new("123").unwrap())),
                },
                "<kde><kodOkresu><odpovida>123</odpovida></kodOkresu></kde>",
            ),
            (
                CodeListRequest::Districts {
                    filter: Some(DistrictFilter::NameContains("district".to_owned())),
                },
                "<kde><nazevOkresu><obsahuje>district</obsahuje></nazevOkresu></kde>",
            ),
            (
                CodeListRequest::Districts {
                    filter: Some(DistrictFilter::NewRegion(
                        NewRegionCode::new("078").unwrap(),
                    )),
                },
                "<kde><kodNovehoKraje><odpovida>078</odpovida></kodNovehoKraje></kde>",
            ),
            (
                CodeListRequest::Workplaces {
                    filter: Some(WorkplaceFilter::Code(WorkplaceCode::new("012").unwrap())),
                },
                "<kde><kodKP><odpovida>012</odpovida></kodKP></kde>",
            ),
            (
                CodeListRequest::Workplaces {
                    filter: Some(WorkplaceFilter::NameContains("office".to_owned())),
                },
                "<kde><nazevKP><obsahuje>office</obsahuje></nazevKP></kde>",
            ),
            (
                CodeListRequest::Workplaces {
                    filter: Some(WorkplaceFilter::Email("a@example.test".to_owned())),
                },
                "<kde><email><odpovida>a@example.test</odpovida></email></kde>",
            ),
            (
                CodeListRequest::Workplaces {
                    filter: Some(WorkplaceFilter::DataBoxId("abc1234".to_owned())),
                },
                "<kde><idDS><odpovida>abc1234</odpovida></idDS></kde>",
            ),
            (
                CodeListRequest::Countries {
                    filter: Some(CountryFilter::NameContains("country".to_owned())),
                },
                "<kde><nazevStatu><obsahuje>country</obsahuje></nazevStatu></kde>",
            ),
            (
                CodeListRequest::Streets {
                    filter: Some(StreetFilter::Municipality(
                        MunicipalityCode::new("123").unwrap(),
                    )),
                },
                "<kde><kodObce><odpovida>123</odpovida></kodObce></kde>",
            ),
            (
                CodeListRequest::Streets {
                    filter: Some(StreetFilter::Code(StreetCode::new("0000456").unwrap())),
                },
                "<kde><kodUlice><odpovida>0000456</odpovida></kodUlice></kde>",
            ),
            (
                CodeListRequest::Streets {
                    filter: Some(StreetFilter::NameContains("street".to_owned())),
                },
                "<kde><nazevUlice><obsahuje>street</obsahuje></nazevUlice></kde>",
            ),
        ];
        assert_eq!(cases.len(), 25);
        for (request, expected) in cases {
            assert_eq!(render_request(&request), expected);
        }
    }

    #[test]
    fn dates_precede_history_filters_and_percent_patterns_are_verbatim() {
        let request = CodeListRequest::CodeListInventory {
            changed_since: Some(date("2026-11-20")),
        };
        assert_eq!(render_request(&request), "<zmenyOd>2026-11-20</zmenyOd>");

        let request = CodeListRequest::CadastralAreaHistory {
            changed_since: Some(date("2020-01-02")),
            filter: Some(CadastralAreaHistoryFilter::NameContains(
                "%A&B<%".to_owned(),
            )),
        };
        assert_eq!(
            render_request(&request),
            "<zmenyOd>2020-01-02</zmenyOd><kde><nazevKU><obsahuje>%A&amp;B&lt;%</obsahuje></nazevKU></kde>"
        );
    }

    #[test]
    fn every_operation_decodes_an_empty_item_list() {
        for request in all_requests() {
            let result = decode_xml(&request, "", "").unwrap();
            assert_eq!(item_count(&result), 0);
            let expected_action = operation(&request).action;
            let expected_name = expected_action.rsplit(':').next().unwrap();
            assert_eq!(result_operation(&result), expected_name);
        }
    }

    #[test]
    fn named_items_allow_repetition_and_retain_unknown_codes_and_update_date() {
        let request = CodeListRequest::ParcelNumberingMethods;
        let result = decode_xml(
            &request,
            r#" posledniAktualizaceCiselniku="2026-11-20""#,
            r#"<cislovaniParcely><kod>000001</kod><nazev>first</nazev></cislovaniParcely><cislovaniParcely><kod>FUTURE</kod><nazev>future</nazev></cislovaniParcely>"#,
        )
        .unwrap();
        let CodeListResult::ParcelNumberingMethods(list) = result else {
            panic!("wrong result variant");
        };
        assert_eq!(list.last_updated, Some(date("2026-11-20")));
        assert_eq!(list.items.len(), 2);
        assert_eq!(list.items[0].code, "000001");
        assert_eq!(list.items[1].code, "FUTURE");
        assert_eq!(list.items[1].name, "future");

        assert!(
            decode_xml(
                &request,
                r#" posledniAktualizaceCiselniku="2026-02-30""#,
                "",
            )
            .is_err()
        );
    }

    #[test]
    fn required_item_cardinality_is_enforced_without_requiring_items() {
        let request = CodeListRequest::ParcelNumberingMethods;
        assert!(
            decode_xml(
                &request,
                "",
                r#"<cislovaniParcely><kod>1</kod></cislovaniParcely>"#,
            )
            .is_err()
        );
        assert!(
            decode_xml(
                &request,
                "",
                r#"<cislovaniParcely><kod>1</kod><kod>2</kod><nazev>duplicate</nazev></cislovaniParcely>"#,
            )
            .is_err()
        );
    }

    #[test]
    fn inventory_and_history_validate_all_dates() {
        let result = decode_xml(
            &CodeListRequest::CodeListInventory {
                changed_since: None,
            },
            "",
            r#"<ciselnik><nazev>seznamKU</nazev><datumPosledniZmeny>2026-11-19</datumPosledniZmeny></ciselnik>"#,
        )
        .unwrap();
        let CodeListResult::CodeListInventory(list) = result else {
            panic!("wrong result variant");
        };
        assert_eq!(list.last_updated, None);
        assert_eq!(list.items[0].name, "seznamKU");
        assert_eq!(list.items[0].last_changed, date("2026-11-19"));

        let request = CodeListRequest::CadastralAreaHistory {
            changed_since: None,
            filter: None,
        };
        let result = decode_xml(
            &request,
            "",
            r#"<katastralniUzemi><kod>000123</kod><nazev>old</nazev><obecKod>000456</obecKod><platnostOd>2020-01-02</platnostOd><platnostDo>2021-03-04</platnostDo></katastralniUzemi><katastralniUzemi><kod>123</kod><nazev>current</nazev><platnostOd>2021-03-05</platnostOd></katastralniUzemi>"#,
        )
        .unwrap();
        let CodeListResult::CadastralAreaHistory(list) = result else {
            panic!("wrong result variant");
        };
        assert_eq!(list.items.len(), 2);
        assert_eq!(list.items[0].code.as_str(), "000123");
        assert_eq!(
            list.items[0].municipality_code.as_ref().unwrap().as_str(),
            "000456"
        );
        assert_eq!(list.items[0].valid_from, date("2020-01-02"));
        assert_eq!(list.items[0].valid_to, Some(date("2021-03-04")));
        assert_eq!(list.items[1].municipality_code, None);
        assert_eq!(list.items[1].valid_to, None);

        assert!(
            decode_xml(
                &request,
                "",
                r#"<katastralniUzemi><kod>123</kod><nazev>bad</nazev><platnostOd>20.11.2026</platnostOd></katastralniUzemi>"#,
            )
            .is_err()
        );
        assert!(
            decode_xml(
                &CodeListRequest::CodeListInventory {
                    changed_since: None,
                },
                "",
                r#"<ciselnik><nazev>bad</nazev><datumPosledniZmeny>not-a-date</datumPosledniZmeny></ciselnik>"#,
            )
            .is_err()
        );
    }

    #[test]
    fn territorial_results_decode_their_distinct_parent_codes() {
        let result = decode_xml(
            &CodeListRequest::CadastralAreas { filter: None },
            "",
            r#"<katastralniUzemi><kod>000123</kod><nazev>area</nazev><kodObce>000456</kodObce></katastralniUzemi>"#,
        )
        .unwrap();
        let CodeListResult::CadastralAreas(list) = result else {
            panic!("wrong result variant");
        };
        assert_eq!(list.items[0].code.as_str(), "000123");
        assert_eq!(
            list.items[0].municipality_code.as_ref().unwrap().as_str(),
            "000456"
        );

        let result = decode_xml(
            &CodeListRequest::MunicipalityParts { filter: None },
            "",
            r#"<castObce><kod>000111</kod><nazev>part</nazev><obecKod>000222</obecKod></castObce>"#,
        )
        .unwrap();
        let CodeListResult::MunicipalityParts(list) = result else {
            panic!("wrong result variant");
        };
        assert_eq!(list.items[0].municipality_code.as_str(), "000222");

        let result = decode_xml(
            &CodeListRequest::CityDistricts { filter: None },
            "",
            r#"<mestskaCast><kod>000333</kod><nazev>district</nazev><kodObce>000444</kodObce></mestskaCast>"#,
        )
        .unwrap();
        let CodeListResult::CityDistricts(list) = result else {
            panic!("wrong result variant");
        };
        assert_eq!(list.items[0].municipality_code.as_str(), "000444");

        let result = decode_xml(
            &CodeListRequest::Municipalities { filter: None },
            "",
            r#"<obec><kod>000555</kod><nazev>town</nazev><okresKod>000666</okresKod></obec>"#,
        )
        .unwrap();
        let CodeListResult::Municipalities(list) = result else {
            panic!("wrong result variant");
        };
        assert_eq!(list.items[0].district_code.as_str(), "000666");

        let result = decode_xml(
            &CodeListRequest::Districts { filter: None },
            "",
            r#"<okres><kod>000777</kod><nazev>district</nazev><nkrajKod>078</nkrajKod></okres>"#,
        )
        .unwrap();
        let CodeListResult::Districts(list) = result else {
            panic!("wrong result variant");
        };
        assert_eq!(list.items[0].new_region_code.as_str(), "078");

        let result = decode_xml(
            &CodeListRequest::Streets { filter: None },
            "",
            r#"<ulice><kod>0000888</kod><nazev>street</nazev><kodObce>000999</kodObce></ulice>"#,
        )
        .unwrap();
        let CodeListResult::Streets(list) = result else {
            panic!("wrong result variant");
        };
        assert_eq!(list.items[0].municipality_code.as_str(), "000999");
    }

    #[test]
    fn workplace_decodes_every_address_and_contact_field() {
        let result = decode_xml(
            &CodeListRequest::Workplaces { filter: None },
            "",
            r#"<pracoviste><kod>012</kod><nazev>office</nazev><nkrajKod>078</nkrajKod><email>a@example.test</email><datovaSchranka>abc1234</datovaSchranka><nazevUlice>Main</nazevUlice><cisloDomovni>0012</cisloDomovni><cisloOrientacni>3a</cisloOrientacni><psc>01234</psc><obec>Town</obec><castObce>Part</castObce><okres>District</okres></pracoviste>"#,
        )
        .unwrap();
        let CodeListResult::Workplaces(list) = result else {
            panic!("wrong result variant");
        };
        assert_eq!(
            list.items,
            vec![Workplace {
                code: WorkplaceCode::new("012").unwrap(),
                name: "office".to_owned(),
                new_region_code: Some(NewRegionCode::new("078").unwrap()),
                email: Some("a@example.test".to_owned()),
                data_box_id: Some("abc1234".to_owned()),
                street_name: Some("Main".to_owned()),
                house_number: Some("0012".to_owned()),
                orientation_number: Some("3a".to_owned()),
                postal_code: Some("01234".to_owned()),
                municipality: Some("Town".to_owned()),
                municipality_part: Some("Part".to_owned()),
                district: Some("District".to_owned()),
            }]
        );
    }

    #[test]
    fn operation_state_ze_abbreviation_and_subject_abbreviations_are_retained() {
        let result = decode_xml(
            &CodeListRequest::ProceedingOperations,
            "",
            r#"<operaceRizeni><kod>1</kod><nazev>operation</nazev><stav>FUTURE STATE</stav></operaceRizeni><operaceRizeni><kod>2</kod><nazev>without state</nazev></operaceRizeni>"#,
        )
        .unwrap();
        let CodeListResult::ProceedingOperations(list) = result else {
            panic!("wrong result variant");
        };
        assert_eq!(list.items[0].state.as_deref(), Some("FUTURE STATE"));
        assert_eq!(list.items[1].state, None);

        let result = decode_xml(
            &CodeListRequest::SimplifiedRegisterNumberingSources,
            "",
            r#"<zdrojCislovaniZE><kod>7</kod><nazev>source</nazev><zkratka>ZE-X</zkratka></zdrojCislovaniZE>"#,
        )
        .unwrap();
        let CodeListResult::SimplifiedRegisterNumberingSources(list) = result else {
            panic!("wrong result variant");
        };
        assert_eq!(list.items[0].abbreviation, "ZE-X");

        let result = decode_xml(
            &CodeListRequest::SubjectCharacteristics,
            "",
            r#"<charakteristikaOS><kod>future</kod><nazev>characteristic</nazev><zkratka>AB</zkratka><zkratkaALV>CD</zkratkaALV></charakteristikaOS>"#,
        )
        .unwrap();
        let CodeListResult::SubjectCharacteristics(list) = result else {
            panic!("wrong result variant");
        };
        assert_eq!(list.items[0].code, "future");
        assert_eq!(list.items[0].abbreviation.as_deref(), Some("AB"));
        assert_eq!(list.items[0].alv_abbreviation.as_deref(), Some("CD"));
    }

    #[test]
    fn service_message_definitions_retain_unknown_code_type_and_name() {
        let result = decode_xml(
            &CodeListRequest::ServiceMessages,
            "",
            r#"<zprava><kod>998</kod><typ>X</typ><nazev>Future message</nazev></zprava>"#,
        )
        .unwrap();
        let CodeListResult::ServiceMessages(list) = result else {
            panic!("wrong result variant");
        };
        assert_eq!(list.items[0].code, MessageCode::Other("998".to_owned()));
        assert_eq!(list.items[0].kind, "X");
        assert_eq!(list.items[0].name, "Future message");
    }

    #[test]
    fn legal_relation_uses_server_boolean_and_preserves_unknown_codes() {
        for (lexical, expected) in [("true", true), ("1", true), ("false", false), ("0", false)] {
            let contents = format!(
                r#"<typPravnihoVztahu><kod>30</kod><nazev>ownership</nazev><vlastnictvi>{lexical}</vlastnictvi></typPravnihoVztahu>"#
            );
            let result = decode_xml(&CodeListRequest::LegalRelationTypes, "", &contents).unwrap();
            let CodeListResult::LegalRelationTypes(list) = result else {
                panic!("wrong result variant");
            };
            assert_eq!(list.items[0].code, LegalRelationCode::Ownership);
            assert_eq!(list.items[0].is_ownership, expected);
        }

        let result = decode_xml(
            &CodeListRequest::LegalRelationTypes,
            "",
            r#"<typPravnihoVztahu><kod>XYZ</kod><nazev>unknown</nazev><vlastnictvi>false</vlastnictvi></typPravnihoVztahu>"#,
        )
        .unwrap();
        let CodeListResult::LegalRelationTypes(list) = result else {
            panic!("wrong result variant");
        };
        assert_eq!(
            list.items[0].code,
            LegalRelationCode::Other("XYZ".to_owned())
        );
        assert!(!list.items[0].is_ownership);

        assert!(
            decode_xml(
                &CodeListRequest::LegalRelationTypes,
                "",
                r#"<typPravnihoVztahu><kod>30</kod><nazev>bad</nazev><vlastnictvi>yes</vlastnictvi></typPravnihoVztahu>"#,
            )
            .is_err()
        );
    }
}
