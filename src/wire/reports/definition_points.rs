use roxmltree::{Document, Node, ParsingOptions};

use crate::model::reports::definition_points::{DefinitionPoint, DefinitionPointsReport};
use crate::{Error, Result};

const XSI_NS: &str = "http://www.w3.org/2001/XMLSchema-instance";
const ROOT_SEQUENCE: &[&str] = &[
    "LIST_G_SOURADNICE",
    "PLATNOST",
    "VYHOTOVIL",
    "BEZPL_PRISTUP",
    "VYHOTOVENO",
];
const POINT_SEQUENCE: &[&str] = &[
    "SOURADNICE_Y",
    "SOURADNICE_X",
    "KATUZE_KOD",
    "PAR_ID",
    "CAOBCE_KOD",
    "STA_ID",
];

pub(crate) fn parse_definition_points(
    bytes: &[u8],
    max_nodes: u32,
    max_depth: u32,
) -> Result<DefinitionPointsReport> {
    let xml = std::str::from_utf8(bytes)
        .map_err(|_| Error::invalid_response("definition points are not UTF-8"))?;
    let document = Document::parse_with_options(
        xml,
        ParsingOptions {
            allow_dtd: false,
            nodes_limit: max_nodes,
            ..ParsingOptions::default()
        },
    )
    .map_err(|_| Error::invalid_response("malformed definition-point XML"))?;
    let root = document.root_element();
    if root.tag_name().namespace().is_some()
        || root.tag_name().name() != "SeznamDefinicnichBoduParcel"
    {
        return Err(Error::invalid_response("unexpected definition-point root"));
    }
    check_depth(root, 1, max_depth)?;
    validate_sequence(root, ROOT_SEQUENCE)?;

    let list = child(root, "LIST_G_SOURADNICE")?
        .ok_or_else(|| Error::invalid_response("definition-point list is missing"))?;
    validate_repeated(list, "G_SOURADNICE")?;
    let points = element_children(list)
        .map(|point| {
            validate_sequence(point, POINT_SEQUENCE)?;
            Ok(DefinitionPoint {
                coordinate_y: required_text(point, "SOURADNICE_Y")?,
                coordinate_x: required_text(point, "SOURADNICE_X")?,
                cadastral_area_code: required_text(point, "KATUZE_KOD")?,
                parcel_id: required_text(point, "PAR_ID")?,
                municipality_part_code: required_text(point, "CAOBCE_KOD")?,
                building_id: required_text(point, "STA_ID")?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(DefinitionPointsReport {
        points,
        valid_at: required_text(root, "PLATNOST")?,
        created_by: required_text(root, "VYHOTOVIL")?,
        free_access: required_text(root, "BEZPL_PRISTUP")?,
        created_at: required_text(root, "VYHOTOVENO")?,
    })
}

fn validate_sequence(node: Node<'_, '_>, fields: &[&str]) -> Result<()> {
    reject_nil(node)?;
    let mut previous = None;
    for child in element_children(node) {
        reject_nil(child)?;
        let index = field_index(child, fields)?;
        if previous.is_some_and(|previous| index <= previous) {
            return Err(Error::invalid_response(format!(
                "element {} violates its XSD sequence or cardinality",
                child.tag_name().name()
            )));
        }
        previous = Some(index);
    }
    Ok(())
}

fn validate_repeated(node: Node<'_, '_>, name: &'static str) -> Result<()> {
    reject_nil(node)?;
    for child in element_children(node) {
        reject_nil(child)?;
        if child.tag_name().namespace().is_some() || child.tag_name().name() != name {
            return Err(unexpected_element(child));
        }
    }
    Ok(())
}

fn field_index(node: Node<'_, '_>, fields: &[&str]) -> Result<usize> {
    if node.tag_name().namespace().is_some() {
        return Err(unexpected_element(node));
    }
    fields
        .iter()
        .position(|name| *name == node.tag_name().name())
        .ok_or_else(|| unexpected_element(node))
}

fn unexpected_element(node: Node<'_, '_>) -> Error {
    Error::invalid_response(format!(
        "unexpected definition-point element {}",
        node.tag_name().name()
    ))
}

fn reject_nil(node: Node<'_, '_>) -> Result<()> {
    if node
        .attribute((XSI_NS, "nil"))
        .is_some_and(|value| matches!(trim_xml_whitespace(value), "true" | "1"))
    {
        Err(Error::invalid_response(format!(
            "definition-point element {} is not nillable",
            node.tag_name().name()
        )))
    } else {
        Ok(())
    }
}

fn trim_xml_whitespace(value: &str) -> &str {
    value.trim_matches(|character| matches!(character, ' ' | '\t' | '\r' | '\n'))
}

fn element_children<'a, 'input>(node: Node<'a, 'input>) -> impl Iterator<Item = Node<'a, 'input>> {
    node.children().filter(|child| child.is_element())
}

fn child<'a, 'input>(node: Node<'a, 'input>, name: &str) -> Result<Option<Node<'a, 'input>>> {
    let mut matches = node.children().filter(|child| {
        child.is_element()
            && child.tag_name().namespace().is_none()
            && child.tag_name().name() == name
    });
    let first = matches.next();
    if matches.next().is_some() {
        return Err(Error::invalid_response(format!("duplicate element {name}")));
    }
    Ok(first)
}

fn required_text(node: Node<'_, '_>, name: &str) -> Result<String> {
    let element = child(node, name)?
        .ok_or_else(|| Error::invalid_response(format!("required element {name} is missing")))?;
    reject_nil(element)?;
    if element.children().any(|child| child.is_element()) {
        return Err(Error::invalid_response(format!(
            "nested scalar element {name}"
        )));
    }
    Ok(element
        .children()
        .filter(|child| child.is_text())
        .filter_map(|child| child.text())
        .collect())
}

fn check_depth(node: Node<'_, '_>, depth: u32, maximum: u32) -> Result<()> {
    if depth > maximum {
        return Err(Error::invalid_response(
            "definition-point XML depth limit exceeded",
        ));
    }
    for child in node.children().filter(|child| child.is_element()) {
        check_depth(child, depth.saturating_add(1), maximum)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const XML: &str = r#"<SeznamDefinicnichBoduParcel><LIST_G_SOURADNICE><G_SOURADNICE><SOURADNICE_Y> 1234.50 </SOURADNICE_Y><SOURADNICE_X>-987.25</SOURADNICE_X><KATUZE_KOD>693936</KATUZE_KOD><PAR_ID>999999999999999999999999999999999999</PAR_ID><CAOBCE_KOD></CAOBCE_KOD><STA_ID></STA_ID></G_SOURADNICE></LIST_G_SOURADNICE><PLATNOST>20.11.2026 10:00</PLATNOST><VYHOTOVIL>test</VYHOTOVIL><BEZPL_PRISTUP>free</BEZPL_PRISTUP><VYHOTOVENO>20.11.2026 11:00</VYHOTOVENO></SeznamDefinicnichBoduParcel>"#;
    const POINT: &str = "<G_SOURADNICE><SOURADNICE_Y> 1234.50 </SOURADNICE_Y><SOURADNICE_X>-987.25</SOURADNICE_X><KATUZE_KOD>693936</KATUZE_KOD><PAR_ID>999999999999999999999999999999999999</PAR_ID><CAOBCE_KOD></CAOBCE_KOD><STA_ID></STA_ID></G_SOURADNICE>";

    fn parses(xml: &str) -> bool {
        parse_definition_points(xml.as_bytes(), 10_000, 32).is_ok()
    }

    #[test]
    fn parses_all_fields_and_present_empty_list() {
        let parsed = parse_definition_points(XML.as_bytes(), 10_000, 32).unwrap();
        assert_eq!(parsed.points.len(), 1);
        assert_eq!(parsed.points[0].coordinate_y, " 1234.50 ");
        assert_eq!(parsed.points[0].parcel_id.len(), 36);
        assert_eq!(parsed.points[0].building_id, "");

        let empty = XML.replace(POINT, "");
        assert!(
            parse_definition_points(empty.as_bytes(), 10_000, 32)
                .unwrap()
                .points
                .is_empty()
        );
    }

    #[test]
    fn rejects_unexpected_and_namespace_qualified_children_without_dropping_rows() {
        let invalid = [
            XML.replace(
                "</SeznamDefinicnichBoduParcel>",
                "<unexpected/></SeznamDefinicnichBoduParcel>",
            ),
            XML.replace(
                "</SeznamDefinicnichBoduParcel>",
                "<x:PLATNOST xmlns:x=\"urn:unexpected\">ignored</x:PLATNOST></SeznamDefinicnichBoduParcel>",
            ),
            XML.replace(
                "<LIST_G_SOURADNICE>",
                "<LIST_G_SOURADNICE><unexpected/>",
            ),
            XML.replace(
                "<LIST_G_SOURADNICE>",
                "<LIST_G_SOURADNICE><x:G_SOURADNICE xmlns:x=\"urn:unexpected\"/>",
            ),
            XML.replace("</G_SOURADNICE>", "<unexpected/></G_SOURADNICE>"),
            XML.replace(
                "</G_SOURADNICE>",
                "<x:STA_ID xmlns:x=\"urn:unexpected\">ignored</x:STA_ID></G_SOURADNICE>",
            ),
            XML.replace(
                "<SOURADNICE_Y> 1234.50 </SOURADNICE_Y>",
                "<SOURADNICE_Y><x:value xmlns:x=\"urn:unexpected\">1234.50</x:value></SOURADNICE_Y>",
            ),
        ];
        for xml in invalid {
            assert!(
                !parses(&xml),
                "unexpected or namespace-qualified child was accepted"
            );
        }
    }

    #[test]
    fn enforces_exact_root_and_point_sequences_and_cardinality() {
        let invalid = [
            XML.replace(
                "<PLATNOST>20.11.2026 10:00</PLATNOST><VYHOTOVIL>test</VYHOTOVIL>",
                "<VYHOTOVIL>test</VYHOTOVIL><PLATNOST>20.11.2026 10:00</PLATNOST>",
            ),
            XML.replace(
                "<SOURADNICE_Y> 1234.50 </SOURADNICE_Y><SOURADNICE_X>-987.25</SOURADNICE_X>",
                "<SOURADNICE_X>-987.25</SOURADNICE_X><SOURADNICE_Y> 1234.50 </SOURADNICE_Y>",
            ),
            XML.replace("<KATUZE_KOD>693936</KATUZE_KOD><PAR_ID>", "<PAR_ID>")
                .replace(
                    "</PAR_ID><CAOBCE_KOD>",
                    "</PAR_ID><KATUZE_KOD>693936</KATUZE_KOD><CAOBCE_KOD>",
                ),
            XML.replace(
                "<LIST_G_SOURADNICE>",
                "<LIST_G_SOURADNICE></LIST_G_SOURADNICE><LIST_G_SOURADNICE>",
            ),
            XML.replace(
                "<PLATNOST>20.11.2026 10:00</PLATNOST>",
                "<PLATNOST>20.11.2026 10:00</PLATNOST><PLATNOST>duplicate</PLATNOST>",
            ),
            XML.replace("<STA_ID></STA_ID>", "<STA_ID></STA_ID><STA_ID/>"),
            XML.replace(
                format!("<LIST_G_SOURADNICE>{POINT}</LIST_G_SOURADNICE>").as_str(),
                "",
            ),
            XML.replace("<CAOBCE_KOD></CAOBCE_KOD>", ""),
        ];
        for xml in invalid {
            assert!(!parses(&xml), "invalid XSD sequence was accepted");
        }
    }

    #[test]
    fn rejects_xsi_nil_on_all_nonnillable_element_levels() {
        let valid = XML.replace(
            "<SeznamDefinicnichBoduParcel>",
            "<SeznamDefinicnichBoduParcel xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\">",
        );
        let invalid = [
            valid.replace(
                "<SeznamDefinicnichBoduParcel xmlns:xsi=",
                "<SeznamDefinicnichBoduParcel xsi:nil=\"true\" xmlns:xsi=",
            ),
            valid.replace(
                "<LIST_G_SOURADNICE>",
                "<LIST_G_SOURADNICE xsi:nil=\"true\">",
            ),
            valid.replace("<G_SOURADNICE>", "<G_SOURADNICE xsi:nil=\"true\">"),
            valid.replace(
                "<SOURADNICE_Y> 1234.50 </SOURADNICE_Y>",
                "<SOURADNICE_Y xsi:nil=\"true\"> 1234.50 </SOURADNICE_Y>",
            ),
            valid.replace(
                "<PLATNOST>20.11.2026 10:00</PLATNOST>",
                "<PLATNOST xsi:nil=\" \t1\r\n\">20.11.2026 10:00</PLATNOST>",
            ),
        ];
        for xml in invalid {
            assert!(!parses(&xml), "xsi:nil on nonnillable element was accepted");
        }
    }

    #[test]
    fn rejects_dtd_wrong_root_and_excess_depth() {
        assert!(
            parse_definition_points(b"<!DOCTYPE x><SeznamDefinicnichBoduParcel/>", 100, 5).is_err()
        );
        assert!(parse_definition_points(b"<Other/>", 100, 5).is_err());
        assert!(parse_definition_points(b"<a><b/></a>", 100, 1).is_err());
    }
}
