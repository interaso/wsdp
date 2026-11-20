use std::str::FromStr;

use quick_xml::Writer;
use roxmltree::Node;
use rust_decimal::Decimal;

use super::search::{
    decode_building, decode_building_right, decode_cadastral_area, decode_ownership, decode_parcel,
    decode_subject, decode_unit,
};
use super::soap::{
    XmlResult, child as soap_child, children, is_nil, require_tag, simple_content, text_element,
};
use crate::client::Operation;
use crate::error::{Error, Result};
use crate::model::ServiceKind;
use crate::model::information::{
    BoundingEnvelope, BoundingGeometry, DetailedUnit, EnvelopeEncoding, GmlNull,
    InformationRequest as Request, InformationResponse as Response, OwnershipSheet,
    OwnershipSheetPreviewResponse, OwnershipSheetPreviewSelector, ParcelBoundingBox,
    ParcelBoundingBoxResponse, SJtskPoint, SpatialReference, UnitOverviewResponse,
    UnitOverviewSelector,
};
use crate::model::search::{trim_xml_whitespace, unsigned_integer_digits};

const INFORMATION_NS: &str = "urn:cz:gov:cuzk:iskn:types:wsdp:informace:3.1";
const GML_NS: &str = "http://www.opengis.net/gml/3.2";

/// Returns the exact WSDL operation and SOAP action for an information request.
pub(crate) fn operation(request: &Request) -> Operation {
    let (name, request_name, response_name) = match request {
        Request::ParcelBoundingBoxes(_) => (
            "dejMBRParcel",
            "DejMBRParcelRequest",
            "DejMBRParcelResponse",
        ),
        Request::UnitOverview(_) => (
            "dejInfoOJednotce",
            "DejInfoOJednotceRequest",
            "DejInfoOJednotceResponse",
        ),
        Request::OwnershipSheetPreview(_) => {
            ("dejNahledLV", "DejNahledLVRequest", "DejNahledLVResponse")
        }
    };
    Operation::new(ServiceKind::Information, name, request_name, response_name)
}

/// Writes the operation-specific children of the SOAP request element.
pub(crate) fn write_request(writer: &mut Writer<Vec<u8>>, request: &Request) -> XmlResult {
    request
        .validate()
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidInput, error))?;
    match request {
        Request::ParcelBoundingBoxes(request) => {
            for id in &request.parcel_ids {
                text_element(writer, "parcelaId", id.as_str())?;
            }
            Ok(())
        }
        Request::UnitOverview(request) => match &request.selector {
            UnitOverviewSelector::Id(id) => text_element(writer, "idJednotky", id.as_str()),
            UnitOverviewSelector::Number(key) => {
                text_element(writer, "castObceKod", key.municipality_part.as_str())?;
                text_element(writer, "stavbaTypKod", key.building_type.as_str())?;
                text_element(writer, "stavbaCislo", key.building_number.as_str())?;
                text_element(writer, "jednotkaCislo", &key.unit_number.to_string())
            }
        },
        Request::OwnershipSheetPreview(request) => match &request.selector {
            OwnershipSheetPreviewSelector::Id(id) => text_element(writer, "lvId", id.as_str()),
            OwnershipSheetPreviewSelector::Number(key) => {
                text_element(writer, "katuzeKod", key.cadastral_area.as_str())?;
                text_element(writer, "lvCislo", &key.sheet_number.to_string())
            }
        },
    }
}

/// Decodes the operation-specific response element selected by `request`.
pub(crate) fn decode_response(request: &Request, response: Node<'_, '_>) -> Result<Response> {
    require_tag(
        response,
        INFORMATION_NS,
        match request {
            Request::ParcelBoundingBoxes(_) => "DejMBRParcelResponse",
            Request::UnitOverview(_) => "DejInfoOJednotceResponse",
            Request::OwnershipSheetPreview(_) => "DejNahledLVResponse",
        },
    )?;
    match request {
        Request::ParcelBoundingBoxes(_) => {
            decode_mbr_response(response).map(Response::ParcelBoundingBoxes)
        }
        Request::UnitOverview(_) => Ok(Response::UnitOverview(Box::new(UnitOverviewResponse {
            ownership_sheet: child(response, INFORMATION_NS, "LV")?
                .map(decode_ownership_sheet)
                .transpose()?,
            unit: child(response, INFORMATION_NS, "jednotka")?
                .map(decode_detailed_unit)
                .transpose()?,
            building: child(response, INFORMATION_NS, "stavba")?
                .map(decode_building)
                .transpose()?,
            subjects: children(response, INFORMATION_NS, "OS")
                .map(decode_subject)
                .collect::<Result<Vec<_>>>()?,
            ownerships: children(response, INFORMATION_NS, "vlastnictvi")
                .map(decode_ownership)
                .collect::<Result<Vec<_>>>()?,
        }))),
        Request::OwnershipSheetPreview(_) => Ok(Response::OwnershipSheetPreview(
            OwnershipSheetPreviewResponse {
                ownership_sheet: child(response, INFORMATION_NS, "LV")?
                    .map(decode_ownership_sheet)
                    .transpose()?,
                subjects: children(response, INFORMATION_NS, "OS")
                    .map(decode_subject)
                    .collect::<Result<Vec<_>>>()?,
                ownerships: children(response, INFORMATION_NS, "vlastnictvi")
                    .map(decode_ownership)
                    .collect::<Result<Vec<_>>>()?,
                parcels: children(response, INFORMATION_NS, "parcela")
                    .map(decode_parcel)
                    .collect::<Result<Vec<_>>>()?,
                buildings: children(response, INFORMATION_NS, "stavba")
                    .map(decode_building)
                    .collect::<Result<Vec<_>>>()?,
                units: children(response, INFORMATION_NS, "jednotka")
                    .map(decode_detailed_unit)
                    .collect::<Result<Vec<_>>>()?,
                building_rights: children(response, INFORMATION_NS, "pravoStavby")
                    .map(decode_building_right)
                    .collect::<Result<Vec<_>>>()?,
            },
        )),
    }
}

fn reject_nil(node: Node<'_, '_>, name: &str) -> Result<()> {
    if is_nil(node) {
        return Err(Error::invalid_response(format!(
            "XML element {name} is nil but is not nillable"
        )));
    }
    Ok(())
}

fn decode_mbr_response(response: Node<'_, '_>) -> Result<ParcelBoundingBoxResponse> {
    let Some(info_list) = child(response, INFORMATION_NS, "infoList")? else {
        return Ok(ParcelBoundingBoxResponse { entries: None });
    };
    let mbr_list = required_child(info_list, INFORMATION_NS, "MBRList")?;
    let entries = children(mbr_list, INFORMATION_NS, "MBR")
        .map(decode_mbr)
        .collect::<Result<Vec<_>>>()?;
    Ok(ParcelBoundingBoxResponse {
        entries: Some(entries),
    })
}

fn decode_mbr(node: Node<'_, '_>) -> Result<ParcelBoundingBox> {
    reject_nil(node, "MBR")?;
    let id_list = required_child(node, INFORMATION_NS, "IDList")?;
    let id_kind = id_list.attribute("typ").unwrap_or("PAR_S").to_owned();
    if id_kind != "PAR_S" {
        return Err(Error::invalid_response(
            "MBR IDList has an invalid fixed type",
        ));
    }
    let parcel_ids = children(id_list, INFORMATION_NS, "ID")
        .map(|id| {
            if is_nil(id) {
                return Err(Error::invalid_response("MBR ID cannot be nil"));
            }
            simple_content(id, "ID")
        })
        .collect::<Result<Vec<_>>>()?;
    if parcel_ids.is_empty() {
        return Err(Error::invalid_response(
            "MBR IDList must contain at least one ID",
        ));
    }
    let bounded_by = required_child(node, GML_NS, "boundedBy")?;
    let envelope = child(bounded_by, GML_NS, "Envelope")?;
    let null = child(bounded_by, GML_NS, "Null")?;
    let geometry = match (envelope, null) {
        (Some(envelope), None) => BoundingGeometry::Envelope(Box::new(decode_envelope(envelope)?)),
        (None, Some(null)) => BoundingGeometry::Null(GmlNull {
            reason: simple_content(null, "Null")?,
        }),
        _ => {
            return Err(Error::invalid_response(
                "gml:boundedBy must contain exactly one Envelope or Null",
            ));
        }
    };
    Ok(ParcelBoundingBox {
        parcel_ids,
        id_kind,
        geometry,
        nil_reason: bounded_by.attribute("nilReason").map(str::to_owned),
    })
}

fn decode_envelope(node: Node<'_, '_>) -> Result<BoundingEnvelope> {
    reject_nil(node, "Envelope")?;
    let reference = spatial_reference(node)?;
    let lower = child(node, GML_NS, "lowerCorner")?;
    let upper = child(node, GML_NS, "upperCorner")?;
    let positions = children(node, GML_NS, "pos").collect::<Vec<_>>();
    let coordinates = child(node, GML_NS, "coordinates")?;
    let (lower, upper, encoding) = match (lower, upper, positions.len(), coordinates) {
        (Some(lower), Some(upper), 0, None) => (
            decode_position(lower)?,
            decode_position(upper)?,
            EnvelopeEncoding::Corners,
        ),
        (None, None, 2, None) => (
            decode_position(positions[0])?,
            decode_position(positions[1])?,
            EnvelopeEncoding::Positions,
        ),
        (None, None, 0, Some(coordinates)) => {
            let (lower, upper, encoding) = decode_coordinates(coordinates)?;
            (lower, upper, encoding)
        }
        _ => {
            return Err(Error::invalid_response(
                "GML Envelope must contain corners, exactly two pos elements, or coordinates",
            ));
        }
    };
    Ok(BoundingEnvelope {
        lower,
        upper,
        encoding,
        reference,
    })
}

fn decode_position(node: Node<'_, '_>) -> Result<SJtskPoint> {
    reject_nil(node, "position")?;
    let reference = spatial_reference(node)?;
    let text = simple_content(node, "position")?;
    let coordinates = text
        .split([' ', '\t', '\r', '\n'])
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    if coordinates.len() != 2 {
        return Err(Error::invalid_response(
            "an S-JTSK GML position must contain exactly Y and X",
        ));
    }
    Ok(SJtskPoint {
        y: parse_double(coordinates[0])?,
        x: parse_double(coordinates[1])?,
        reference,
    })
}

fn decode_coordinates(node: Node<'_, '_>) -> Result<(SJtskPoint, SJtskPoint, EnvelopeEncoding)> {
    reject_nil(node, "coordinates")?;
    let decimal_separator = node.attribute("decimal").unwrap_or(".").to_owned();
    let coordinate_separator = node.attribute("cs").unwrap_or(",").to_owned();
    let tuple_separator = node.attribute("ts").unwrap_or(" ").to_owned();
    if decimal_separator.is_empty() || coordinate_separator.is_empty() || tuple_separator.is_empty()
    {
        return Err(Error::invalid_response(
            "GML coordinate separators cannot be empty",
        ));
    }
    let content = simple_content(node, "coordinates")?;
    let tuples = if tuple_separator.chars().all(char::is_whitespace) {
        content.split_whitespace().collect::<Vec<_>>()
    } else {
        content.split(&tuple_separator).collect::<Vec<_>>()
    };
    if tuples.len() != 2 {
        return Err(Error::invalid_response(
            "GML coordinates must contain exactly two S-JTSK positions",
        ));
    }
    let mut points = Vec::with_capacity(2);
    for tuple in tuples {
        let ordinates = tuple.split(&coordinate_separator).collect::<Vec<_>>();
        if ordinates.len() != 2 {
            return Err(Error::invalid_response(
                "an S-JTSK coordinate tuple must contain exactly Y and X",
            ));
        }
        let y = normalize_decimal(ordinates[0].trim(), &decimal_separator);
        let x = normalize_decimal(ordinates[1].trim(), &decimal_separator);
        points.push(SJtskPoint {
            y: parse_double(&y)?,
            x: parse_double(&x)?,
            reference: SpatialReference::default(),
        });
    }
    Ok((
        points.remove(0),
        points.remove(0),
        EnvelopeEncoding::Coordinates {
            decimal_separator,
            coordinate_separator,
            tuple_separator,
        },
    ))
}

fn normalize_decimal(value: &str, separator: &str) -> String {
    if separator == "." {
        value.to_owned()
    } else {
        value.replace(separator, ".")
    }
}

fn parse_double(value: &str) -> Result<Decimal> {
    let value = trim_xml_whitespace(value);
    let invalid = || Error::invalid_input("double", "expected a finite XML double");
    if matches!(value, "INF" | "-INF" | "NaN") {
        return Err(invalid());
    }
    let (negative, unsigned) = match value.as_bytes().first() {
        Some(b'-') => (true, &value[1..]),
        Some(b'+') => (false, &value[1..]),
        _ => (false, value),
    };
    let exponent_start = unsigned.find(['e', 'E']);
    let (mantissa, exponent) = match exponent_start {
        Some(index) => (
            &unsigned[..index],
            parse_double_exponent(&unsigned[index + 1..]).ok_or_else(invalid)?,
        ),
        None => (unsigned, 0),
    };
    let (integer, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    if (integer.is_empty() && fraction.is_empty())
        || !integer
            .bytes()
            .chain(fraction.bytes())
            .all(|byte| byte.is_ascii_digit())
    {
        return Err(invalid());
    }
    let mut coefficient = String::with_capacity(integer.len() + fraction.len());
    coefficient.push_str(integer);
    coefficient.push_str(fraction);
    if coefficient.bytes().all(|byte| byte == b'0') {
        return Ok(Decimal::ZERO);
    }
    let significant = coefficient.trim_start_matches('0');
    let trailing_zeroes = significant.len() - significant.trim_end_matches('0').len();
    let significant = significant.trim_end_matches('0');
    let power = exponent
        .saturating_sub(fraction.len() as i128)
        .saturating_add(trailing_zeroes as i128);
    if significant.len() > 29 {
        return Err(Error::DecimalOutOfRange {
            field: "S-JTSK coordinate",
        });
    }

    let mut canonical = String::new();
    if negative {
        canonical.push('-');
    }
    if power >= 0 {
        if significant.len() as i128 + power > 29 {
            return Err(Error::DecimalOutOfRange {
                field: "S-JTSK coordinate",
            });
        }
        canonical.push_str(significant);
        canonical.extend(std::iter::repeat_n('0', power as usize));
    } else {
        let scale = power.saturating_neg();
        if scale > 28 {
            return Err(Error::DecimalOutOfRange {
                field: "S-JTSK coordinate",
            });
        }
        let decimal_position = significant.len() as i128 - scale;
        if decimal_position <= 0 {
            canonical.push_str("0.");
            canonical.extend(std::iter::repeat_n('0', (-decimal_position) as usize));
            canonical.push_str(significant);
        } else {
            let decimal_position = decimal_position as usize;
            canonical.push_str(&significant[..decimal_position]);
            canonical.push('.');
            canonical.push_str(&significant[decimal_position..]);
        }
    }
    crate::model::parse_decimal(&canonical, "S-JTSK coordinate")
}

fn parse_double_exponent(value: &str) -> Option<i128> {
    let (negative, digits) = match value.as_bytes().first() {
        Some(b'-') => (true, &value[1..]),
        Some(b'+') => (false, &value[1..]),
        _ => (false, value),
    };
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let exponent = digits.bytes().fold(0_i128, |value, digit| {
        value
            .saturating_mul(10)
            .saturating_add(i128::from(digit - b'0'))
    });
    Some(if negative {
        exponent.saturating_neg()
    } else {
        exponent
    })
}

fn spatial_reference(node: Node<'_, '_>) -> Result<SpatialReference> {
    let dimension = node
        .attribute("srsDimension")
        .map(|value| {
            let dimension = unsigned_integer_digits(value)
                .ok_or_else(|| {
                    Error::invalid_response("GML srsDimension is not a positive integer")
                })?
                .parse::<u32>()
                .map_err(|_| {
                    Error::invalid_response("GML srsDimension is not a positive integer")
                })?;
            if dimension == 0 {
                return Err(Error::invalid_response("GML srsDimension must be positive"));
            }
            Ok(dimension)
        })
        .transpose()?;
    let axis_labels = node.attribute("axisLabels").map(|value| {
        value
            .split_whitespace()
            .map(str::to_owned)
            .collect::<Vec<_>>()
    });
    let unit_labels = node.attribute("uomLabels").map(|value| {
        value
            .split_whitespace()
            .map(str::to_owned)
            .collect::<Vec<_>>()
    });
    if axis_labels.is_some() != unit_labels.is_some() {
        return Err(Error::invalid_response(
            "GML axisLabels and uomLabels must occur together",
        ));
    }
    Ok(SpatialReference {
        name: node.attribute("srsName").map(str::to_owned),
        dimension,
        axis_labels,
        unit_labels,
    })
}

fn decode_ownership_sheet(node: Node<'_, '_>) -> Result<OwnershipSheet> {
    reject_nil(node, "LV")?;
    Ok(OwnershipSheet {
        id: required_parse(node, INFORMATION_NS, "idLV")?,
        cadastral_area: decode_cadastral_area(required_child(
            node,
            INFORMATION_NS,
            "katastralniUzemi",
        )?)?,
        number: required_sheet_number(node)?,
    })
}

fn decode_detailed_unit(node: Node<'_, '_>) -> Result<DetailedUnit> {
    reject_nil(node, "jednotka")?;
    Ok(DetailedUnit {
        unit: decode_unit(node)?,
        type_name: optional_text(node, INFORMATION_NS, "typNazev")?,
        use_name: optional_text(node, INFORMATION_NS, "zpuVyNazev")?,
    })
}

fn child<'a, 'input>(
    node: Node<'a, 'input>,
    namespace: &str,
    name: &str,
) -> Result<Option<Node<'a, 'input>>> {
    let element = soap_child(node, namespace, name)?;
    if element.is_some_and(is_nil) {
        return Err(Error::invalid_response(format!(
            "XML element {name} is nil but is not nillable"
        )));
    }
    Ok(element)
}

fn required_child<'a, 'input>(
    node: Node<'a, 'input>,
    namespace: &str,
    name: &str,
) -> Result<Node<'a, 'input>> {
    child(node, namespace, name)?
        .ok_or_else(|| Error::invalid_response(format!("required XML element {name} is missing")))
}

fn required_sheet_number(node: Node<'_, '_>) -> Result<u32> {
    let raw = required_child(node, INFORMATION_NS, "lvCislo")?;
    let raw = simple_content(raw, "lvCislo")?;
    let number = unsigned_integer_digits(&raw)
        .and_then(|digits| digits.parse::<u32>().ok())
        .filter(|number| (1..=999_999).contains(number))
        .ok_or_else(|| Error::invalid_response("XML element lvCislo is outside its XSD range"))?;
    Ok(number)
}

fn required_parse<T: FromStr>(node: Node<'_, '_>, namespace: &str, name: &str) -> Result<T> {
    let node = required_child(node, namespace, name)?;
    parse_node(node, name)
}

fn optional_text(node: Node<'_, '_>, namespace: &str, name: &str) -> Result<Option<String>> {
    let Some(element) = child(node, namespace, name)? else {
        return Ok(None);
    };
    simple_content(element, name).map(Some)
}

fn parse_node<T: FromStr>(node: Node<'_, '_>, name: &str) -> Result<T> {
    if is_nil(node) {
        return Err(Error::invalid_response(format!(
            "required XML element {name} is nil"
        )));
    }
    simple_content(node, name)?
        .parse()
        .map_err(|_| Error::invalid_response(format!("XML element {name} has an invalid value")))
}

#[cfg(test)]
mod tests {
    use roxmltree::Document;
    use rust_decimal::Decimal;

    use super::super::soap::COMMON_NS;
    use super::*;
    use crate::model::information::{
        OwnershipSheetNaturalKey, ParcelBoundingBoxRequest, UnitOverviewNaturalKey,
        UnitOverviewRequest,
    };
    use crate::model::search::{BuildingTypeCode, OwnershipTarget};
    use crate::model::{
        CadastralAreaCode, MunicipalityPartCode, OwnershipSheetId, ParcelId, UnitId,
    };

    fn mbr_request() -> Request {
        Request::ParcelBoundingBoxes(
            ParcelBoundingBoxRequest::new(vec![ParcelId::new("1").unwrap()]).unwrap(),
        )
    }

    fn encoded(request: &Request) -> String {
        let mut writer = Writer::new(Vec::new());
        write_request(&mut writer, request).unwrap();
        String::from_utf8(writer.into_inner()).unwrap()
    }

    fn decoded(request: &Request, response_name: &str, contents: &str) -> Result<Response> {
        let xml = format!(
            r#"<{response_name} xmlns="{INFORMATION_NS}" xmlns:c="{COMMON_NS}" xmlns:gml="{GML_NS}" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">{contents}</{response_name}>"#
        );
        let document = Document::parse(&xml).unwrap();
        decode_response(request, document.root_element())
    }

    #[test]
    fn operations_and_request_choices_match_the_wsdl_and_xsd() {
        let mbr = Request::ParcelBoundingBoxes(
            ParcelBoundingBoxRequest::new(vec![
                ParcelId::new("1").unwrap(),
                ParcelId::new("2").unwrap(),
            ])
            .unwrap(),
        );
        let unit = Request::UnitOverview(UnitOverviewRequest {
            selector: UnitOverviewSelector::Number(
                UnitOverviewNaturalKey::new(
                    MunicipalityPartCode::new("123").unwrap(),
                    BuildingTypeCode::new("1").unwrap(),
                    crate::model::information::UnitBuildingNumber::new("456").unwrap(),
                    7,
                )
                .unwrap(),
            ),
        });
        let preview = Request::OwnershipSheetPreview(
            crate::model::information::OwnershipSheetPreviewRequest {
                selector: OwnershipSheetPreviewSelector::Number(
                    OwnershipSheetNaturalKey::new(CadastralAreaCode::new("693936").unwrap(), 10)
                        .unwrap(),
                ),
            },
        );
        for (request, action, request_name, response_name) in [
            (
                &mbr,
                "dejMBRParcel",
                "DejMBRParcelRequest",
                "DejMBRParcelResponse",
            ),
            (
                &unit,
                "dejInfoOJednotce",
                "DejInfoOJednotceRequest",
                "DejInfoOJednotceResponse",
            ),
            (
                &preview,
                "dejNahledLV",
                "DejNahledLVRequest",
                "DejNahledLVResponse",
            ),
        ] {
            let operation = operation(request);
            assert_eq!(
                operation.action,
                format!("urn:cz:gov:cuzk:iskn:services:wsdp:informace:{action}")
            );
            assert_eq!(operation.request, request_name);
            assert_eq!(operation.response, response_name);
        }

        assert_eq!(
            encoded(&mbr),
            "<parcelaId>1</parcelaId><parcelaId>2</parcelaId>"
        );
        assert_eq!(
            encoded(&unit),
            "<castObceKod>123</castObceKod><stavbaTypKod>1</stavbaTypKod><stavbaCislo>456</stavbaCislo><jednotkaCislo>7</jednotkaCislo>"
        );
        assert_eq!(
            encoded(&preview),
            "<katuzeKod>693936</katuzeKod><lvCislo>10</lvCislo>"
        );

        let unit_id = Request::UnitOverview(UnitOverviewRequest {
            selector: UnitOverviewSelector::Id(UnitId::new("10").unwrap()),
        });
        assert_eq!(encoded(&unit_id), "<idJednotky>10</idJednotky>");
        let preview_id = Request::OwnershipSheetPreview(
            crate::model::information::OwnershipSheetPreviewRequest {
                selector: OwnershipSheetPreviewSelector::Id(OwnershipSheetId::new("30").unwrap()),
            },
        );
        assert_eq!(encoded(&preview_id), "<lvId>30</lvId>");
    }

    #[test]
    fn serialization_rechecks_invalid_public_fields_before_writing() {
        let requests = vec![
            Request::ParcelBoundingBoxes(ParcelBoundingBoxRequest {
                parcel_ids: Vec::new(),
            }),
            Request::UnitOverview(UnitOverviewRequest {
                selector: UnitOverviewSelector::Number(UnitOverviewNaturalKey {
                    municipality_part: MunicipalityPartCode::new("123").unwrap(),
                    building_type: BuildingTypeCode::new("1").unwrap(),
                    building_number: crate::model::information::UnitBuildingNumber::new("456")
                        .unwrap(),
                    unit_number: 10_000_000_000,
                }),
            }),
            Request::OwnershipSheetPreview(
                crate::model::information::OwnershipSheetPreviewRequest {
                    selector: OwnershipSheetPreviewSelector::Number(OwnershipSheetNaturalKey {
                        cadastral_area: CadastralAreaCode::new("693936").unwrap(),
                        sheet_number: 0,
                    }),
                },
            ),
        ];
        for request in requests {
            let mut writer = Writer::new(Vec::new());
            assert!(write_request(&mut writer, &request).is_err());
            assert!(writer.into_inner().is_empty());
        }
    }

    #[test]
    fn mbr_decodes_repeated_ids_envelope_null_and_list_presence() {
        let request = mbr_request();
        let contents = r#"<infoList><MBRList>
            <MBR><IDList typ="PAR_S"><ID>1</ID><ID>2</ID></IDList><gml:boundedBy nilReason="withheld"><gml:Envelope srsName="urn:ogc:def:crs:EPSG::5514" srsDimension="2" axisLabels="Y X" uomLabels="m m"><gml:lowerCorner>-1000.25 -2000.50</gml:lowerCorner><gml:upperCorner>-900.00 -1900.00</gml:upperCorner></gml:Envelope></gml:boundedBy></MBR>
            <MBR><IDList><ID>schema-string</ID></IDList><gml:boundedBy><gml:Null>inapplicable</gml:Null></gml:boundedBy></MBR>
        </MBRList></infoList>"#;
        let Response::ParcelBoundingBoxes(result) =
            decoded(&request, "DejMBRParcelResponse", contents).unwrap()
        else {
            panic!("wrong response variant");
        };
        let entries = result.entries.unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].parcel_ids.len(), 2);
        assert_eq!(entries[0].nil_reason.as_deref(), Some("withheld"));
        assert_eq!(entries[1].parcel_ids[0], "schema-string");
        let BoundingGeometry::Envelope(envelope) = &entries[0].geometry else {
            panic!("expected envelope");
        };
        assert_eq!(envelope.lower.y, Decimal::from_str("-1000.25").unwrap());
        assert_eq!(envelope.lower.x, Decimal::from_str("-2000.50").unwrap());
        assert_eq!(envelope.reference.dimension, Some(2));
        assert_eq!(
            envelope.reference.axis_labels,
            Some(vec!["Y".into(), "X".into()])
        );
        assert!(
            matches!(entries[1].geometry, BoundingGeometry::Null(GmlNull { ref reason }) if reason == "inapplicable")
        );

        let Response::ParcelBoundingBoxes(missing) =
            decoded(&request, "DejMBRParcelResponse", "").unwrap()
        else {
            panic!("wrong response variant");
        };
        assert_eq!(missing.entries, None);
        let Response::ParcelBoundingBoxes(empty) = decoded(
            &request,
            "DejMBRParcelResponse",
            "<infoList><MBRList/></infoList>",
        )
        .unwrap() else {
            panic!("wrong response variant");
        };
        assert_eq!(empty.entries, Some(Vec::new()));
        assert!(decoded(&request, "DejMBRParcelResponse", "<infoList/>").is_err());
    }

    #[test]
    fn mbr_supports_deprecated_coordinates_and_rejects_decimal_overflow() {
        let request = mbr_request();
        let contents = r#"<infoList><MBRList><MBR><IDList><ID>1</ID></IDList><gml:boundedBy><gml:Envelope><gml:coordinates decimal="," cs=";" ts="|">-1000,25;-2000,50|-900,00;-1900,00</gml:coordinates></gml:Envelope></gml:boundedBy></MBR></MBRList></infoList>"#;
        let Response::ParcelBoundingBoxes(result) =
            decoded(&request, "DejMBRParcelResponse", contents).unwrap()
        else {
            panic!("wrong response variant");
        };
        let BoundingGeometry::Envelope(envelope) = &result.entries.unwrap()[0].geometry else {
            panic!("expected envelope");
        };
        assert_eq!(envelope.upper.y, Decimal::from_str("-900.00").unwrap());
        assert!(matches!(
            envelope.encoding,
            EnvelopeEncoding::Coordinates { .. }
        ));

        let overflow = r#"<infoList><MBRList><MBR><IDList><ID>1</ID></IDList><gml:boundedBy><gml:Envelope><gml:lowerCorner>999999999999999999999999999999999999999 1</gml:lowerCorner><gml:upperCorner>1 1</gml:upperCorner></gml:Envelope></gml:boundedBy></MBR></MBRList></infoList>"#;
        assert!(matches!(
            decoded(&request, "DejMBRParcelResponse", overflow),
            Err(Error::DecimalOutOfRange {
                field: "S-JTSK coordinate"
            })
        ));
    }

    #[test]
    fn direct_positions_accept_exact_finite_scientific_doubles() {
        for (raw, expected) in [
            ("-1.25e3", Decimal::new(-1250, 0)),
            ("+125E-2", Decimal::new(125, 2)),
            (".5e1", Decimal::new(5, 0)),
            ("5.e-1", Decimal::new(5, 1)),
            ("10e-29", Decimal::new(1, 28)),
            ("0e999999999999999999999999", Decimal::ZERO),
            (
                "7.9228162514264337593543950335e28",
                Decimal::from_str("79228162514264337593543950335").unwrap(),
            ),
        ] {
            let actual = parse_double(raw).unwrap_or_else(|error| panic!("{raw}: {error}"));
            assert_eq!(actual, expected, "{raw}");
        }
        for raw in ["INF", "-INF", "NaN", "1e", "1,2", "1e2e3"] {
            assert!(parse_double(raw).is_err(), "{raw}");
        }
        for raw in [
            "1e29",
            "1e-29",
            "7.9228162514264337593543950336e28",
            "1e999999999999999999999999",
        ] {
            assert!(matches!(
                parse_double(raw),
                Err(Error::DecimalOutOfRange {
                    field: "S-JTSK coordinate"
                })
            ));
        }

        let request = mbr_request();
        let coordinates = r#"<infoList><MBRList><MBR><IDList><ID>1</ID></IDList><gml:boundedBy><gml:Envelope><gml:lowerCorner>-1.25e3 -2.5E3</gml:lowerCorner><gml:upperCorner>125e-2 250e-2</gml:upperCorner></gml:Envelope></gml:boundedBy></MBR></MBRList></infoList>"#;
        let Response::ParcelBoundingBoxes(result) =
            decoded(&request, "DejMBRParcelResponse", coordinates).unwrap()
        else {
            panic!("wrong response variant");
        };
        let BoundingGeometry::Envelope(envelope) = &result.entries.unwrap()[0].geometry else {
            panic!("expected envelope");
        };
        assert_eq!(envelope.lower.y, Decimal::new(-1250, 0));
        assert_eq!(envelope.lower.x, Decimal::new(-2500, 0));
        assert_eq!(envelope.upper.y, Decimal::new(125, 2));
        assert_eq!(envelope.upper.x, Decimal::new(250, 2));

        let nonfinite = coordinates.replace("-1.25e3", "NaN");
        assert!(decoded(&request, "DejMBRParcelResponse", &nonfinite).is_err());
    }

    #[test]
    fn information_responses_reject_nil_and_enforce_lv_ranges() {
        let request = Request::UnitOverview(UnitOverviewRequest {
            selector: UnitOverviewSelector::Id(UnitId::new("10").unwrap()),
        });
        let lv = |number: &str| {
            format!(
                "<LV><idLV>30</idLV><katastralniUzemi><c:kod>693936</c:kod></katastralniUzemi><lvCislo>{number}</lvCislo></LV>"
            )
        };
        let Response::UnitOverview(result) =
            decoded(&request, "DejInfoOJednotceResponse", &lv(" +00010 ")).unwrap()
        else {
            panic!("wrong response variant");
        };
        assert_eq!(result.ownership_sheet.unwrap().number, 10);
        for invalid in ["0", "1000000", "-1"] {
            assert!(
                decoded(&request, "DejInfoOJednotceResponse", &lv(invalid)).is_err(),
                "{invalid}"
            );
        }
        let nil_lv = lv("10").replace("<LV>", "<LV xsi:nil=\"true\">");
        assert!(decoded(&request, "DejInfoOJednotceResponse", &nil_lv).is_err());

        let unit = r#"<jednotka><c:idJednotky>10</c:idJednotky><c:stavba><c:idStavby>20</c:idStavby><c:typStavbyKod>1</c:typStavbyKod><c:castObceKod>123</c:castObceKod><c:cisloDomovni>4</c:cisloDomovni></c:stavba><c:typJedKod>1</c:typJedKod><c:cislo>7</c:cislo><typNazev xsi:nil="true">Byt</typNazev></jednotka>"#;
        assert!(decoded(&request, "DejInfoOJednotceResponse", unit).is_err());
    }

    #[test]
    fn unit_overview_decodes_extensions_and_ownership_choices() {
        let request = Request::UnitOverview(UnitOverviewRequest {
            selector: UnitOverviewSelector::Id(UnitId::new("10").unwrap()),
        });
        let contents = r#"<LV><idLV>30</idLV><katastralniUzemi><c:kod>693936</c:kod><c:nazev>Test</c:nazev></katastralniUzemi><lvCislo>10</lvCislo></LV>
            <jednotka><c:idJednotky>10</c:idJednotky><c:stavba><c:idStavby>20</c:idStavby><c:typStavbyKod>1</c:typStavbyKod><c:castObceKod>123</c:castObceKod><c:cisloDomovni>4</c:cisloDomovni></c:stavba><c:typJedKod>1</c:typJedKod><c:cislo>7</c:cislo><typNazev>Byt</typNazev><zpuVyNazev>Bydleni</zpuVyNazev></jednotka>
            <OS><c:idOS>1</c:idOS><c:vlastnikTyp>OFO</c:vlastnikTyp></OS><OS><c:idOS>2</c:idOS><c:vlastnikTyp>OPO</c:vlastnikTyp></OS>
            <vlastnictvi><c:idVlastnictvi>5</c:idVlastnictvi><c:lvId>30</c:lvId><c:podilCitatel>123456789012345678901234567890</c:podilCitatel><c:podilJmenovatel>999999999999999999999999999999</c:podilJmenovatel><c:typPravnihoVztahuKod>V</c:typPravnihoVztahuKod><c:katuzeKod>693936</c:katuzeKod><c:lvCislo>10</c:lvCislo><c:jednotkaId>10</c:jednotkaId></vlastnictvi>"#;
        let Response::UnitOverview(result) =
            decoded(&request, "DejInfoOJednotceResponse", contents).unwrap()
        else {
            panic!("wrong response variant");
        };
        assert_eq!(
            result
                .ownership_sheet
                .unwrap()
                .cadastral_area
                .name
                .as_deref(),
            Some("Test")
        );
        let unit = result.unit.unwrap();
        assert_eq!(unit.type_name.as_deref(), Some("Byt"));
        assert_eq!(unit.use_name.as_deref(), Some("Bydleni"));
        assert_eq!(result.subjects.len(), 2);
        assert_eq!(result.ownerships.len(), 1);
        assert_eq!(
            result.ownerships[0].share_numerator.as_str(),
            "123456789012345678901234567890"
        );
        assert!(matches!(
            result.ownerships[0].target,
            OwnershipTarget::Unit(_)
        ));
    }

    #[test]
    fn lv_preview_decodes_every_response_collection() {
        let preview = Request::OwnershipSheetPreview(
            crate::model::information::OwnershipSheetPreviewRequest {
                selector: OwnershipSheetPreviewSelector::Id(OwnershipSheetId::new("30").unwrap()),
            },
        );
        let contents = r#"<LV><idLV>30</idLV><katastralniUzemi><c:kod>693936</c:kod><c:nazev>Test</c:nazev></katastralniUzemi><lvCislo>10</lvCislo></LV>
            <OS><c:idOS>1</c:idOS><c:vlastnikTyp>OFO</c:vlastnikTyp></OS>
            <vlastnictvi><c:idVlastnictvi>5</c:idVlastnictvi><c:lvId>30</c:lvId><c:podilCitatel>1</c:podilCitatel><c:podilJmenovatel>2</c:podilJmenovatel><c:typPravnihoVztahuKod>V</c:typPravnihoVztahuKod><c:katuzeKod>693936</c:katuzeKod><c:lvCislo>10</c:lvCislo><c:osId>1</c:osId></vlastnictvi>
            <parcela><c:idParcely>1</c:idParcely><c:parcelaType>PKN</c:parcelaType><c:katuzeKod>693936</c:katuzeKod><c:kmenoveCislo>77</c:kmenoveCislo><c:vymera>100</c:vymera><c:druhCislovaniPar>1</c:druhCislovaniPar><c:stavbaSoucastiParcely>n</c:stavbaSoucastiParcely></parcela>
            <stavba><c:idStavby>20</c:idStavby><c:stavbaTypKod>1</c:stavbaTypKod></stavba>
            <jednotka><c:idJednotky>10</c:idJednotky><c:stavba><c:idStavby>20</c:idStavby><c:typStavbyKod>1</c:typStavbyKod><c:castObceKod>123</c:castObceKod><c:cisloDomovni>4</c:cisloDomovni></c:stavba><c:typJedKod>1</c:typJedKod><c:cislo>7</c:cislo><typNazev>Byt</typNazev></jednotka>
            <pravoStavby><c:idPravoStavby>40</c:idPravoStavby><c:datumKoncePlatnosti>2030-01-01</c:datumKoncePlatnosti><c:ucelyPravaStavby><c:ucelPravaStavby><c:kod>0</c:kod><c:nazev> Purpose </c:nazev></c:ucelPravaStavby></c:ucelyPravaStavby><c:pouzeIdentParcela>a</c:pouzeIdentParcela><c:parcely><c:parcela><c:idParcely>1</c:idParcely><c:katuzeKod>693936</c:katuzeKod><c:kmenoveCislo>77</c:kmenoveCislo><c:parcelaType>PKN</c:parcelaType><c:druhCislovaniPar>1</c:druhCislovaniPar></c:parcela></c:parcely></pravoStavby>"#;
        let Response::OwnershipSheetPreview(result) =
            decoded(&preview, "DejNahledLVResponse", contents).unwrap()
        else {
            panic!("wrong response variant");
        };
        assert_eq!(result.ownership_sheet.unwrap().number, 10);
        assert_eq!(result.subjects.len(), 1);
        assert_eq!(result.ownerships.len(), 1);
        assert_eq!(result.parcels.len(), 1);
        assert_eq!(result.buildings.len(), 1);
        assert_eq!(result.units.len(), 1);
        assert_eq!(result.units[0].type_name.as_deref(), Some("Byt"));
        assert_eq!(result.building_rights.len(), 1);
        assert_eq!(result.building_rights[0].purposes[0].name, " Purpose ");
    }

    #[test]
    fn empty_unit_and_lv_responses_keep_all_optional_fields_empty() {
        let unit = Request::UnitOverview(UnitOverviewRequest {
            selector: UnitOverviewSelector::Id(UnitId::new("10").unwrap()),
        });
        let Response::UnitOverview(result) =
            decoded(&unit, "DejInfoOJednotceResponse", "").unwrap()
        else {
            panic!("wrong response variant");
        };
        assert!(
            result.ownership_sheet.is_none() && result.unit.is_none() && result.building.is_none()
        );
        assert!(result.subjects.is_empty() && result.ownerships.is_empty());

        let preview = Request::OwnershipSheetPreview(
            crate::model::information::OwnershipSheetPreviewRequest {
                selector: OwnershipSheetPreviewSelector::Id(OwnershipSheetId::new("30").unwrap()),
            },
        );
        let Response::OwnershipSheetPreview(result) =
            decoded(&preview, "DejNahledLVResponse", "").unwrap()
        else {
            panic!("wrong response variant");
        };
        assert!(result.ownership_sheet.is_none());
        assert!(
            result.subjects.is_empty() && result.ownerships.is_empty() && result.parcels.is_empty()
        );
        assert!(
            result.buildings.is_empty()
                && result.units.is_empty()
                && result.building_rights.is_empty()
        );
    }
}
