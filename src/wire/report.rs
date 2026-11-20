use quick_xml::Writer;
use quick_xml::events::{BytesEnd, BytesStart, Event};
use roxmltree::Node;

use super::soap::{self, XmlResult};
use crate::client::Operation;
use crate::model::{ReportFamily, ReportVersion, ReportVersionInfo, ServiceKind};
use crate::{Error, Result};

pub(crate) fn versions_operation() -> Operation {
    Operation::new(
        ServiceKind::Reports,
        "dejVerzeSestav",
        "DejVerzeSestavRequest",
        "DejVerzeSestavResponse",
    )
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

#[cfg(test)]
mod tests {
    use roxmltree::Document;

    use super::*;

    const REPORTS_NS: &str = "urn:cz:gov:cuzk:iskn:types:wsdp:sestavy:3.1";

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
    fn versions_decoder_retains_future_families_and_versions() {
        let xml = format!(
            r#"<DejVerzeSestavResponse xmlns="{REPORTS_NS}" xmlns:c="{}"><verzeSestav><verzeSestavy><kodSestavy>Future</kodSestavy><nazevSestavy>Future report</nazevSestavy><seznamVerzi><c:verze>4.01</c:verze></seznamVerzi></verzeSestavy></verzeSestav></DejVerzeSestavResponse>"#,
            soap::COMMON_NS
        );
        let document = Document::parse(&xml).unwrap();
        let versions = decode_versions(document.root_element()).unwrap();
        assert_eq!(versions[0].family.as_str(), "Future");
        assert_eq!(versions[0].versions[0].as_str(), "4.01");
    }
}
