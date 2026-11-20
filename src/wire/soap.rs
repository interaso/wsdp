use quick_xml::Writer;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use roxmltree::{Document, Node, ParsingOptions};

use super::auth::{SecurityHeader, write_security};
use crate::error::{Error, Result, ServiceError};
use crate::{Credentials, MessageCode, MessageLevel, ServiceKind, ServiceMessage};

pub(crate) const SOAP_NS: &str = "http://schemas.xmlsoap.org/soap/envelope/";
pub(crate) const COMMON_NS: &str = "urn:cz:gov:cuzk:iskn:types:common:3.1";
const XSI_NS: &str = "http://www.w3.org/2001/XMLSchema-instance";
const WSSE_NS: &str =
    "http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-secext-1.0.xsd";
const WSU_NS: &str =
    "http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-utility-1.0.xsd";

#[derive(Clone, Copy)]
pub(crate) struct XmlLimits {
    pub(crate) max_nodes: u32,
    pub(crate) max_depth: u32,
}

pub(crate) type XmlResult = std::result::Result<(), std::io::Error>;

pub(crate) fn request(
    operation: &str,
    namespace: &str,
    credentials: &Credentials,
    write_operation: impl FnOnce(&mut Writer<Vec<u8>>) -> XmlResult,
) -> Result<Vec<u8>> {
    let security = SecurityHeader::fresh()?;
    request_with_security(
        operation,
        namespace,
        credentials,
        &security,
        write_operation,
    )
}

fn request_with_security(
    operation: &str,
    namespace: &str,
    credentials: &Credentials,
    security: &SecurityHeader,
    write_operation: impl FnOnce(&mut Writer<Vec<u8>>) -> XmlResult,
) -> Result<Vec<u8>> {
    let mut writer = Writer::new(Vec::new());
    writer
        .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
        .and_then(|()| {
            let mut envelope = BytesStart::new("soap:Envelope");
            envelope.push_attribute(("xmlns:soap", SOAP_NS));
            envelope.push_attribute(("xmlns:xsi", XSI_NS));
            envelope.push_attribute(("xmlns:wsse", WSSE_NS));
            envelope.push_attribute(("xmlns:wsu", WSU_NS));
            writer.write_event(Event::Start(envelope))
        })
        .and_then(|()| writer.write_event(Event::Start(BytesStart::new("soap:Header"))))
        .and_then(|()| write_security(&mut writer, credentials, security))
        .and_then(|()| writer.write_event(Event::End(BytesEnd::new("soap:Header"))))
        .and_then(|()| writer.write_event(Event::Start(BytesStart::new("soap:Body"))))
        .and_then(|()| {
            let mut operation_start = BytesStart::new(operation);
            operation_start.push_attribute(("xmlns", namespace));
            writer.write_event(Event::Start(operation_start))
        })
        .and_then(|()| write_operation(&mut writer))
        .and_then(|()| writer.write_event(Event::End(BytesEnd::new(operation))))
        .and_then(|()| writer.write_event(Event::End(BytesEnd::new("soap:Body"))))
        .and_then(|()| writer.write_event(Event::End(BytesEnd::new("soap:Envelope"))))
        .map_err(|error| {
            Error::invalid_input("xml", format!("could not encode request: {error}"))
        })?;
    Ok(writer.into_inner())
}

pub(crate) fn text_element(writer: &mut Writer<Vec<u8>>, name: &str, value: &str) -> XmlResult {
    writer.write_event(Event::Start(BytesStart::new(name)))?;
    writer.write_event(Event::Text(BytesText::new(value)))?;
    writer.write_event(Event::End(BytesEnd::new(name)))?;
    Ok(())
}

pub(crate) fn decode<T>(
    bytes: &[u8],
    service: ServiceKind,
    operation: &str,
    limits: XmlLimits,
    decode_response: impl FnOnce(Node<'_, '_>) -> Result<T>,
) -> Result<ResponseParts<T>> {
    let xml = std::str::from_utf8(bytes)
        .map_err(|_| Error::invalid_response("response is not UTF-8 XML"))?;
    preflight(xml, limits)?;
    let options = ParsingOptions {
        allow_dtd: false,
        nodes_limit: limits.max_nodes,
        ..ParsingOptions::default()
    };
    let document = Document::parse_with_options(xml, options)
        .map_err(|_| Error::invalid_response("malformed XML response"))?;
    let envelope = document.root_element();
    require_tag(envelope, SOAP_NS, "Envelope")?;
    let body = child(envelope, SOAP_NS, "Body")?
        .ok_or_else(|| Error::invalid_response("SOAP Body is missing"))?;

    if let Some(fault) = child(body, SOAP_NS, "Fault")? {
        let code = descendant_text(fault, "faultcode")?.unwrap_or_else(|| "unknown".to_owned());
        let message = descendant_text(fault, "faultstring")?
            .unwrap_or_else(|| "unspecified SOAP fault".to_owned());
        return Err(Error::SoapFault { code, message });
    }

    let response = child(body, service.namespace(), operation)?.ok_or_else(|| {
        Error::invalid_response(format!("SOAP Body does not contain {operation}"))
    })?;
    let messages = decode_messages(response, service.namespace())?;
    if messages
        .iter()
        .any(|message| message.level == MessageLevel::Error)
    {
        return Err(ServiceError::new(messages).into());
    }
    let value = decode_response(response)?;
    Ok(ResponseParts { value, messages })
}

pub(crate) struct ResponseParts<T> {
    pub(crate) value: T,
    pub(crate) messages: Vec<ServiceMessage>,
}

impl<T> ResponseParts<T> {
    pub(crate) fn into_result(self) -> Result<crate::Response<T>> {
        if self
            .messages
            .iter()
            .any(|message| message.level == MessageLevel::Error)
        {
            Err(ServiceError::new(self.messages).into())
        } else {
            Ok(crate::Response {
                value: self.value,
                messages: self.messages,
            })
        }
    }
}

fn decode_messages(response: Node<'_, '_>, service_ns: &str) -> Result<Vec<ServiceMessage>> {
    let Some(result) = child_namespaces(response, &[service_ns, COMMON_NS], "vysledek")? else {
        return Err(Error::invalid_response("WSDP result container is missing"));
    };
    children(result, COMMON_NS, "zprava")
        .map(|message| {
            let code = message
                .attribute("kod")
                .ok_or_else(|| Error::invalid_response("service message code is missing"))?;
            let level = message
                .attribute("uroven")
                .ok_or_else(|| Error::invalid_response("service message level is missing"))?;
            Ok(ServiceMessage {
                code: MessageCode::from_raw(code),
                level: MessageLevel::from_raw(level),
                text: simple_content(message, "zprava")?,
            })
        })
        .collect()
}

fn preflight(xml: &str, limits: XmlLimits) -> Result<()> {
    let mut reader = quick_xml::Reader::from_str(xml);
    let mut nodes = 0_u32;
    let mut depth = 0_u32;
    loop {
        match reader.read_event() {
            Ok(Event::Eof) => break,
            Ok(Event::Start(element)) => {
                nodes = preflight_node(nodes, limits.max_nodes)?;
                nodes = preflight_attributes(&element, nodes, limits.max_nodes)?;
                depth = depth.checked_add(1).ok_or_else(|| {
                    Error::invalid_response("XML response exceeds the nesting limit")
                })?;
                if depth > limits.max_depth {
                    return Err(Error::invalid_response(
                        "XML response exceeds the nesting limit",
                    ));
                }
            }
            Ok(Event::Empty(element)) => {
                if depth >= limits.max_depth {
                    return Err(Error::invalid_response(
                        "XML response exceeds the nesting limit",
                    ));
                }
                nodes = preflight_node(nodes, limits.max_nodes)?;
                nodes = preflight_attributes(&element, nodes, limits.max_nodes)?;
            }
            Ok(Event::End(_)) => {
                depth = depth
                    .checked_sub(1)
                    .ok_or_else(|| Error::invalid_response("malformed XML response"))?;
            }
            Ok(Event::DocType(_)) => {
                return Err(Error::invalid_response(
                    "DTD is not allowed in XML responses",
                ));
            }
            Ok(
                Event::Text(_)
                | Event::CData(_)
                | Event::Comment(_)
                | Event::Decl(_)
                | Event::PI(_)
                | Event::GeneralRef(_),
            ) => nodes = preflight_node(nodes, limits.max_nodes)?,
            Err(_) => return Err(Error::invalid_response("malformed XML response")),
        }
    }
    if depth != 0 {
        return Err(Error::invalid_response("malformed XML response"));
    }
    Ok(())
}

fn preflight_node(nodes: u32, maximum: u32) -> Result<u32> {
    let nodes = nodes
        .checked_add(1)
        .ok_or_else(|| Error::invalid_response("XML response exceeds the node limit"))?;
    if nodes > maximum {
        return Err(Error::invalid_response(
            "XML response exceeds the node limit",
        ));
    }
    Ok(nodes)
}

fn preflight_attributes(element: &BytesStart<'_>, mut nodes: u32, maximum: u32) -> Result<u32> {
    for attribute in element.attributes() {
        if attribute.is_err() {
            return Err(Error::invalid_response("malformed XML response"));
        }
        nodes = preflight_node(nodes, maximum)?;
    }
    Ok(nodes)
}

pub(crate) fn require_tag(node: Node<'_, '_>, namespace: &str, name: &str) -> Result<()> {
    if node.tag_name().namespace() == Some(namespace) && node.tag_name().name() == name {
        Ok(())
    } else {
        Err(Error::invalid_response(format!(
            "expected XML element {name}"
        )))
    }
}

pub(crate) fn child<'a, 'input>(
    node: Node<'a, 'input>,
    namespace: &str,
    name: &str,
) -> Result<Option<Node<'a, 'input>>> {
    let mut matches = children(node, namespace, name);
    let first = matches.next();
    if matches.next().is_some() {
        return Err(Error::invalid_response(format!(
            "XML element {name} occurs more than once"
        )));
    }
    Ok(first)
}

pub(crate) fn child_namespaces<'a, 'input>(
    node: Node<'a, 'input>,
    namespaces: &[&str],
    name: &str,
) -> Result<Option<Node<'a, 'input>>> {
    let mut matches = node.children().filter(|candidate| {
        candidate.is_element()
            && candidate.tag_name().name() == name
            && candidate
                .tag_name()
                .namespace()
                .is_some_and(|namespace| namespaces.contains(&namespace))
    });
    let first = matches.next();
    if matches.next().is_some() {
        return Err(Error::invalid_response(format!(
            "XML element {name} occurs more than once"
        )));
    }
    Ok(first)
}

pub(crate) fn children<'a, 'input>(
    node: Node<'a, 'input>,
    namespace: &str,
    name: &str,
) -> impl Iterator<Item = Node<'a, 'input>> {
    node.children().filter(move |candidate| {
        candidate.is_element()
            && candidate.tag_name().namespace() == Some(namespace)
            && candidate.tag_name().name() == name
    })
}

pub(crate) fn is_nil(node: Node<'_, '_>) -> bool {
    node.attribute((XSI_NS, "nil"))
        .is_some_and(|value| value == "true" || value == "1")
}

pub(crate) fn simple_content(node: Node<'_, '_>, name: &str) -> Result<String> {
    let mut content = String::new();
    for child in node.children() {
        if child.is_element() {
            return Err(Error::invalid_response(format!(
                "XML element {name} contains a nested element"
            )));
        }
        if child.is_text()
            && let Some(text) = child.text()
        {
            content.push_str(text);
        }
    }
    Ok(content)
}

fn descendant_text(node: Node<'_, '_>, name: &str) -> Result<Option<String>> {
    node.descendants()
        .find(|candidate| candidate.is_element() && candidate.tag_name().name() == name)
        .map(|candidate| simple_content(candidate, name))
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIMITS: XmlLimits = XmlLimits {
        max_nodes: 10_000,
        max_depth: 64,
    };

    fn envelope(contents: &str) -> String {
        format!(
            r#"<soap:Envelope xmlns:soap="{SOAP_NS}"><soap:Body><StavWSResponse xmlns="urn:cz:gov:cuzk:iskn:types:wsdp:ciselnik:3.1" xmlns:c="{COMMON_NS}">{contents}</StavWSResponse></soap:Body></soap:Envelope>"#
        )
    }

    #[test]
    fn request_escapes_credentials_and_body() {
        let security = SecurityHeader {
            token_id: "UsernameToken-fixed".to_owned(),
            nonce: "bm9uY2U=".to_owned(),
            created: "2026-11-20T00:00:00Z".to_owned(),
        };
        let request = request_with_security(
            "StavWSRequest",
            ServiceKind::CodeLists.namespace(),
            &Credentials::new("user&name", "pass<word"),
            &security,
            |writer| text_element(writer, "value", "Příliš & <žluťoučký>"),
        )
        .unwrap();
        let xml = String::from_utf8(request).unwrap();
        assert!(xml.contains("user&amp;name"));
        assert!(xml.contains("pass&lt;word"));
        assert!(xml.contains("Příliš &amp; &lt;žluťoučký&gt;"));
        assert!(xml.contains("soap:mustUnderstand=\"1\""));
    }

    #[test]
    fn messages_and_unknown_values_are_retained() {
        let xml = envelope(
            r#"<vysledek><c:zprava kod="0" uroven="INFORMACE">ok</c:zprava><c:zprava kod="999999999999999999999" uroven="FUTURE">future</c:zprava></vysledek>"#,
        );
        let response = decode(
            xml.as_bytes(),
            ServiceKind::CodeLists,
            "StavWSResponse",
            LIMITS,
            |_| Ok(()),
        )
        .unwrap()
        .into_result()
        .unwrap();
        assert_eq!(response.messages.len(), 2);
        assert_eq!(response.messages[1].code.as_str(), "999999999999999999999");
        assert_eq!(response.messages[1].level.as_str(), "FUTURE");
    }

    #[test]
    fn errors_retain_all_messages() {
        let xml = envelope(
            r#"<vysledek><c:zprava kod="0" uroven="INFORMACE">ok</c:zprava><c:zprava kod="477" uroven="CHYBA">busy</c:zprava></vysledek>"#,
        );
        let error = match decode(
            xml.as_bytes(),
            ServiceKind::CodeLists,
            "StavWSResponse",
            LIMITS,
            |_| Ok(()),
        ) {
            Err(error) => error,
            Ok(_) => panic!("an application error must be returned"),
        };
        assert!(error.is_busy());
        assert!(matches!(error, Error::Service(ref service) if service.messages().len() == 2));
    }

    #[test]
    fn dtd_and_duplicate_singletons_are_rejected() {
        let dtd = "<!DOCTYPE thing [<!ENTITY xxe SYSTEM 'file:///etc/passwd'>]><x/>";
        assert!(preflight(dtd, LIMITS).is_err());

        let duplicate = envelope(r#"<vysledek/><vysledek/><c:zprava xmlns:c="unused"/>"#);
        assert!(
            decode(
                duplicate.as_bytes(),
                ServiceKind::CodeLists,
                "StavWSResponse",
                LIMITS,
                |_| Ok(()),
            )
            .is_err()
        );
    }
}
