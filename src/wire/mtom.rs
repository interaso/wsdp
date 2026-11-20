use std::collections::HashSet;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use bytes::Bytes;
use mime::Mime;
use roxmltree::Node;

use super::report::decode_report_metadata;
use super::soap::{self, XmlLimits};
use crate::model::{DownloadedReport, ReportId, ReportMetadata, ServiceKind};
use crate::{Error, Result};

const XOP_INCLUDE_NS: &str = "http://www.w3.org/2004/08/xop/include";
const MAX_MIME_PARTS: usize = 4;
const MAX_PART_HEADERS: usize = 32;
const MAX_PART_HEADER_BYTES: usize = 16 * 1024;
const MAX_CONTENT_ID_BYTES: usize = 2 * 1024;

/// Buffered-download limits. XML limits apply to the SOAP envelope, while the
/// decoded limit applies to report bytes after inline base64 or MTOM extraction.
#[derive(Clone, Copy)]
pub(crate) struct DownloadLimits {
    pub(crate) max_decoded_bytes: usize,
    pub(crate) xml: XmlLimits,
}

/// Decodes either a `text/xml` inline-base64 response or a multipart/related
/// MTOM/XOP response and correlates its sole report with the requested ID.
pub(crate) fn decode_download(
    bytes: &[u8],
    content_type: &str,
    expected_report_id: &ReportId,
    limits: DownloadLimits,
) -> Result<DownloadedReport> {
    let content_type = content_type
        .trim()
        .parse::<Mime>()
        .map_err(|_| Error::invalid_response("invalid report download Content-Type"))?;
    if content_type.type_() == mime::TEXT && content_type.subtype() == mime::XML {
        return decode_inline(bytes, expected_report_id, limits);
    }
    if content_type.type_() == mime::MULTIPART && content_type.subtype().as_str() == "related" {
        return decode_mtom(bytes, &content_type, expected_report_id, limits);
    }
    Err(Error::invalid_response(
        "report download must be text/xml or multipart/related",
    ))
}

fn decode_inline(
    bytes: &[u8],
    expected_report_id: &ReportId,
    limits: DownloadLimits,
) -> Result<DownloadedReport> {
    let parts = soap::decode(
        bytes,
        ServiceKind::Reports,
        "GenerujSestavuResponse",
        limits.xml,
        |response| decode_report_payload(response, expected_report_id, limits, false),
    )?;
    let (metadata, payload) = parts.value;
    let Payload::Inline(bytes) = payload else {
        return Err(Error::invalid_response(
            "text/xml report response contains an external XOP reference",
        ));
    };
    Ok(DownloadedReport {
        metadata,
        messages: parts.messages,
        bytes,
    })
}

fn decode_mtom(
    bytes: &[u8],
    content_type: &Mime,
    expected_report_id: &ReportId,
    limits: DownloadLimits,
) -> Result<DownloadedReport> {
    let boundary = content_type
        .get_param("boundary")
        .map(|value| value.as_str())
        .ok_or_else(|| Error::invalid_response("multipart boundary is missing"))?;
    if boundary.is_empty()
        || boundary.len() > 70
        || !boundary.is_ascii()
        || boundary.bytes().any(|byte| byte == b'\r' || byte == b'\n')
    {
        return Err(Error::invalid_response("invalid multipart boundary"));
    }
    let start = content_type
        .get_param("start")
        .map(|value| normalize_content_id(value.as_str()))
        .transpose()?;
    let declared_root_type = content_type.get_param("type");
    let parts = parse_multipart(bytes, boundary)?;

    let root_index = match start {
        Some(start) => parts
            .iter()
            .position(|part| part.content_id == start)
            .ok_or_else(|| {
                Error::invalid_response("multipart start does not identify a MIME part")
            })?,
        None => 0,
    };
    let root = &parts[root_index];
    if !is_soap_xml(root.content_type.as_ref()) {
        return Err(Error::invalid_response(
            "multipart start part is not a SOAP XML document",
        ));
    }
    if let Some(declared) = declared_root_type
        && root
            .content_type
            .as_ref()
            .is_none_or(|actual| actual.essence_str() != declared.as_str())
    {
        return Err(Error::invalid_response(
            "multipart type parameter does not match its start part",
        ));
    }
    require_text_transfer_encoding(root.content_transfer_encoding.as_deref())?;

    let decoded = soap::decode(
        root.body,
        ServiceKind::Reports,
        "GenerujSestavuResponse",
        limits.xml,
        |response| decode_report_payload(response, expected_report_id, limits, true),
    )?;
    let (metadata, payload) = decoded.value;
    let report_bytes = match payload {
        Payload::Inline(bytes) => {
            if parts.len() != 1 {
                return Err(Error::invalid_response(
                    "inline multipart SOAP response contains unrelated attachments",
                ));
            }
            bytes
        }
        Payload::Attachment(content_id) => {
            if parts.len() != 2 {
                return Err(Error::invalid_response(
                    "MTOM response must contain one SOAP part and one attachment",
                ));
            }
            let attachment_index = parts
                .iter()
                .position(|part| part.content_id == content_id)
                .ok_or_else(|| {
                    Error::invalid_response("XOP reference has no matching MIME attachment")
                })?;
            if attachment_index == root_index {
                return Err(Error::invalid_response(
                    "XOP reference points to the SOAP start part",
                ));
            }
            let attachment = &parts[attachment_index];
            if !matches!(
                attachment.content_transfer_encoding.as_deref(),
                None | Some("binary")
            ) {
                return Err(Error::invalid_response(
                    "MTOM attachment must use binary content transfer encoding",
                ));
            }
            if attachment.body.len() > limits.max_decoded_bytes {
                return Err(Error::ResponseTooLarge {
                    limit: limits.max_decoded_bytes,
                });
            }
            Bytes::copy_from_slice(attachment.body)
        }
    };
    Ok(DownloadedReport {
        metadata,
        messages: decoded.messages,
        bytes: report_bytes,
    })
}

fn is_soap_xml(content_type: Option<&Mime>) -> bool {
    content_type.is_some_and(|content_type| {
        matches!(
            content_type.essence_str(),
            "text/xml" | "application/xop+xml" | "application/soap+xml"
        )
    })
}

fn require_text_transfer_encoding(encoding: Option<&str>) -> Result<()> {
    if matches!(
        encoding,
        None | Some("7bit") | Some("8bit") | Some("binary")
    ) {
        Ok(())
    } else {
        Err(Error::invalid_response(
            "SOAP MIME part has an unsupported transfer encoding",
        ))
    }
}

enum Payload {
    Inline(Bytes),
    Attachment(String),
}

fn decode_report_payload(
    response: Node<'_, '_>,
    expected_report_id: &ReportId,
    limits: DownloadLimits,
    allow_xop: bool,
) -> Result<(ReportMetadata, Payload)> {
    let namespace = ServiceKind::Reports.namespace();
    let list = soap::child(response, namespace, "reportList")?
        .ok_or_else(|| Error::invalid_response("download report list is missing"))?;
    let mut reports = soap::children(list, namespace, "report");
    let report = reports
        .next()
        .ok_or_else(|| Error::invalid_response("download report is missing"))?;
    if reports.next().is_some() {
        return Err(Error::invalid_response(
            "download response contains more than one report",
        ));
    }
    let metadata = decode_report_metadata(report)?;
    if metadata.id() != expected_report_id {
        return Err(Error::invalid_response(
            "downloaded report ID does not match the requested report",
        ));
    }
    let file = soap::child(report, namespace, "souborSestavy")?
        .ok_or_else(|| Error::invalid_response("downloaded report payload is missing"))?;
    let mut elements = file.children().filter(Node::is_element);
    if let Some(include) = elements.next() {
        if elements.next().is_some()
            || include.tag_name().namespace() != Some(XOP_INCLUDE_NS)
            || include.tag_name().name() != "Include"
            || !allow_xop
            || file
                .children()
                .filter(Node::is_text)
                .any(|text| text.text().is_some_and(|text| !text.trim().is_empty()))
        {
            return Err(Error::invalid_response(
                "report payload has an invalid XOP structure",
            ));
        }
        let href = include
            .attribute("href")
            .ok_or_else(|| Error::invalid_response("XOP Include href is missing"))?;
        return Ok((metadata, Payload::Attachment(parse_cid(href)?)));
    }
    let content = soap::simple_content(file, "souborSestavy")?;
    Ok((
        metadata,
        Payload::Inline(decode_inline_base64(&content, limits.max_decoded_bytes)?),
    ))
}

fn parse_cid(href: &str) -> Result<String> {
    let href = href.trim();
    let Some(cid) = href
        .get(..4)
        .filter(|scheme| scheme.eq_ignore_ascii_case("cid:"))
        .and_then(|_| href.get(4..))
    else {
        return Err(Error::invalid_response(
            "XOP Include href must use the cid scheme",
        ));
    };
    if cid.is_empty()
        || cid.len() > MAX_CONTENT_ID_BYTES
        || cid.bytes().any(|byte| byte.is_ascii_whitespace())
    {
        return Err(Error::invalid_response("invalid XOP content ID"));
    }
    Ok(cid.to_owned())
}

fn decode_inline_base64(content: &str, maximum: usize) -> Result<Bytes> {
    let mut encoded_length = 0_usize;
    let mut last = [0_u8; 2];
    for byte in content.bytes().filter(|byte| !byte.is_ascii_whitespace()) {
        encoded_length = encoded_length
            .checked_add(1)
            .ok_or_else(|| Error::invalid_response("invalid inline report base64 length"))?;
        last = [last[1], byte];
    }
    if encoded_length == 0 {
        return Ok(Bytes::new());
    }
    if !encoded_length.is_multiple_of(4) {
        return Err(Error::invalid_response("malformed inline report base64"));
    }
    let padding = if last == *b"==" {
        2
    } else if last[1] == b'=' {
        1
    } else {
        0
    };
    let decoded_length = (encoded_length / 4)
        .checked_mul(3)
        .and_then(|length| length.checked_sub(padding))
        .ok_or_else(|| Error::invalid_response("invalid inline report base64 length"))?;
    if decoded_length > maximum {
        return Err(Error::ResponseTooLarge { limit: maximum });
    }
    let mut encoded = Vec::with_capacity(encoded_length);
    encoded.extend(content.bytes().filter(|byte| !byte.is_ascii_whitespace()));
    let decoded = BASE64_STANDARD
        .decode(encoded)
        .map_err(|_| Error::invalid_response("malformed inline report base64"))?;
    if decoded.len() > maximum {
        return Err(Error::ResponseTooLarge { limit: maximum });
    }
    Ok(Bytes::from(decoded))
}

struct MimePart<'a> {
    content_id: String,
    content_type: Option<Mime>,
    content_transfer_encoding: Option<String>,
    body: &'a [u8],
}

fn parse_multipart<'a>(bytes: &'a [u8], boundary: &str) -> Result<Vec<MimePart<'a>>> {
    let mut marker = Vec::with_capacity(boundary.len() + 2);
    marker.extend_from_slice(b"--");
    marker.extend_from_slice(boundary.as_bytes());
    let first = find_marker(bytes, &marker, 0)
        .ok_or_else(|| Error::invalid_response("multipart opening boundary is missing"))?;
    let mut cursor = first + marker.len();
    let mut parts = Vec::new();
    let mut content_ids = HashSet::new();
    loop {
        if bytes.get(cursor..cursor + 2) == Some(b"--") {
            cursor += 2;
            while matches!(bytes.get(cursor), Some(b' ' | b'\t')) {
                cursor += 1;
            }
            if cursor != bytes.len() && bytes.get(cursor..cursor + 2) != Some(b"\r\n") {
                return Err(Error::invalid_response(
                    "malformed multipart closing boundary",
                ));
            }
            if parts.is_empty() {
                return Err(Error::invalid_response("multipart response has no parts"));
            }
            return Ok(parts);
        }
        if bytes.get(cursor..cursor + 2) != Some(b"\r\n") {
            return Err(Error::invalid_response("malformed multipart boundary"));
        }
        let header_start = cursor + 2;
        let header_end_relative = find_subslice(&bytes[header_start..], b"\r\n\r\n")
            .ok_or_else(|| Error::invalid_response("truncated MIME part headers"))?;
        let header_end = header_start + header_end_relative;
        if header_end - header_start > MAX_PART_HEADER_BYTES {
            return Err(Error::invalid_response("MIME part headers are too large"));
        }
        let body_start = header_end + 4;
        let next_marker = find_marker(bytes, &marker, body_start)
            .ok_or_else(|| Error::invalid_response("multipart closing boundary is missing"))?;
        let body_end = if next_marker == body_start {
            body_start
        } else {
            next_marker - 2
        };
        if next_marker != body_start && bytes.get(body_end..next_marker) != Some(b"\r\n") {
            return Err(Error::invalid_response("malformed multipart part boundary"));
        }
        let part = parse_part(
            &bytes[header_start..header_end],
            &bytes[body_start..body_end],
        )?;
        if !content_ids.insert(part.content_id.clone()) {
            return Err(Error::invalid_response("duplicate MIME Content-ID"));
        }
        if parts.len() >= MAX_MIME_PARTS {
            return Err(Error::invalid_response("too many MIME parts"));
        }
        parts.push(part);
        cursor = next_marker + marker.len();
    }
}

fn find_marker(haystack: &[u8], marker: &[u8], start: usize) -> Option<usize> {
    let mut search_start = start;
    while let Some(relative) = find_subslice(&haystack[search_start..], marker) {
        let position = search_start + relative;
        let begins_line = position == 0
            || (position >= 2 && haystack.get(position - 2..position) == Some(b"\r\n"));
        let after = position + marker.len();
        let valid_suffix = haystack.get(after..after + 2) == Some(b"\r\n")
            || haystack.get(after..after + 2) == Some(b"--");
        if begins_line && valid_suffix {
            return Some(position);
        }
        search_start = position.checked_add(1)?;
        if search_start >= haystack.len() {
            break;
        }
    }
    None
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || needle.len() > haystack.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn parse_part<'a>(headers: &[u8], body: &'a [u8]) -> Result<MimePart<'a>> {
    let headers = std::str::from_utf8(headers)
        .map_err(|_| Error::invalid_response("MIME part headers are not UTF-8"))?;
    let mut content_id = None;
    let mut content_type = None;
    let mut content_transfer_encoding = None;
    let mut count = 0;
    for line in headers.split("\r\n") {
        count += 1;
        if count > MAX_PART_HEADERS {
            return Err(Error::invalid_response("too many MIME part headers"));
        }
        if line.starts_with([' ', '\t']) {
            return Err(Error::invalid_response(
                "folded MIME part headers are not supported",
            ));
        }
        let (name, value) = line
            .split_once(':')
            .ok_or_else(|| Error::invalid_response("malformed MIME part header"))?;
        let value = value.trim();
        if name.eq_ignore_ascii_case("Content-ID") {
            if content_id.is_some() {
                return Err(Error::invalid_response("duplicate MIME Content-ID header"));
            }
            content_id = Some(normalize_content_id(value)?);
        } else if name.eq_ignore_ascii_case("Content-Type") {
            if content_type.is_some() {
                return Err(Error::invalid_response(
                    "duplicate MIME Content-Type header",
                ));
            }
            content_type = Some(
                value
                    .parse::<Mime>()
                    .map_err(|_| Error::invalid_response("invalid MIME part Content-Type"))?,
            );
        } else if name.eq_ignore_ascii_case("Content-Transfer-Encoding") {
            if content_transfer_encoding.is_some() {
                return Err(Error::invalid_response(
                    "duplicate MIME transfer-encoding header",
                ));
            }
            content_transfer_encoding = Some(value.to_ascii_lowercase());
        }
    }
    let content_id =
        content_id.ok_or_else(|| Error::invalid_response("MIME part Content-ID is missing"))?;
    Ok(MimePart {
        content_id,
        content_type,
        content_transfer_encoding,
        body,
    })
}

fn normalize_content_id(value: &str) -> Result<String> {
    let value = value.trim();
    let value = if value.starts_with('<') || value.ends_with('>') {
        if !(value.starts_with('<') && value.ends_with('>')) || value.len() < 3 {
            return Err(Error::invalid_response("invalid MIME Content-ID"));
        }
        &value[1..value.len() - 1]
    } else {
        value
    };
    if value.is_empty()
        || value.len() > MAX_CONTENT_ID_BYTES
        || value.bytes().any(|byte| byte.is_ascii_whitespace())
    {
        return Err(Error::invalid_response("invalid MIME Content-ID"));
    }
    Ok(value.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ReportState;

    const SOAP_NS: &str = "http://schemas.xmlsoap.org/soap/envelope/";
    const REPORTS_NS: &str = "urn:cz:gov:cuzk:iskn:types:wsdp:sestavy:3.1";

    fn limits(maximum: usize) -> DownloadLimits {
        DownloadLimits {
            max_decoded_bytes: maximum,
            xml: XmlLimits {
                max_nodes: 10_000,
                max_depth: 64,
            },
        }
    }

    fn soap(payload: &str) -> String {
        format!(
            r#"<soap:Envelope xmlns:soap="{SOAP_NS}"><soap:Body><GenerujSestavuResponse xmlns="{REPORTS_NS}" xmlns:c="{}"><vysledek><c:zprava kod="0" uroven="INFORMACE">ok</c:zprava></vysledek><reportList><report><id>123</id><nazev>report</nazev><cena>0.00</cena><datumPozadavku>2026-01-01T00:00:00Z</datumPozadavku><stav>zpracován</stav><format>pdf</format><souborSestavy>{payload}</souborSestavy></report></reportList></GenerujSestavuResponse></soap:Body></soap:Envelope>"#,
            soap::COMMON_NS
        )
    }

    #[test]
    fn inline_base64_preserves_bytes_metadata_and_messages() {
        let encoded = BASE64_STANDARD.encode(b"%PDF\n-secret\x00");
        let encoded = format!("\r\n {}\r\n{}", &encoded[..4], &encoded[4..]);
        let response = soap(&encoded);
        let expected = ReportId::new("123").unwrap();
        let report = decode_download(
            response.as_bytes(),
            "text/xml; charset=utf-8",
            &expected,
            limits(1024),
        )
        .unwrap();
        assert_eq!(report.bytes.as_ref(), b"%PDF\n-secret\x00");
        assert_eq!(report.id().as_str(), "123");
        assert_eq!(report.metadata.state, ReportState::Processed);
        assert_eq!(report.messages.len(), 1);
        let debug = format!("{report:?}");
        assert!(debug.contains("byte_len"));
        assert!(!debug.contains("PDF-secret"));
    }

    #[test]
    fn inline_decoder_enforces_decoded_limit_and_rejects_bad_base64_safely() {
        let expected = ReportId::new("123").unwrap();
        let response = soap("QUJD");
        let error =
            decode_download(response.as_bytes(), "text/xml", &expected, limits(2)).unwrap_err();
        assert!(matches!(error, Error::ResponseTooLarge { limit: 2 }));

        let response = soap("not*base64*sensitive");
        let error =
            decode_download(response.as_bytes(), "text/xml", &expected, limits(1024)).unwrap_err();
        assert!(!error.to_string().contains("sensitive"));
        assert!(matches!(error, Error::InvalidResponse { .. }));
    }

    fn multipart(root_xml: &str, attachment_id: &str, attachment: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(
            b"preamble\r\n--report-boundary\r\nContent-Type: application/xop+xml; charset=utf-8\r\nContent-Transfer-Encoding: 8bit\r\nContent-ID: <root@example>\r\n\r\n",
        );
        bytes.extend_from_slice(root_xml.as_bytes());
        bytes.extend_from_slice(b"\r\n--report-boundary\r\nContent-Type: application/octet-stream\r\nContent-Transfer-Encoding: binary\r\nContent-ID: <");
        bytes.extend_from_slice(attachment_id.as_bytes());
        bytes.extend_from_slice(b">\r\n\r\n");
        bytes.extend_from_slice(attachment);
        bytes.extend_from_slice(b"\r\n--report-boundary--\r\nepilogue");
        bytes
    }

    #[test]
    fn mtom_xop_decodes_binary_attachment_and_validates_start_part() {
        let root = soap(
            r#"<xop:Include xmlns:xop="http://www.w3.org/2004/08/xop/include" href="cid:report@example"/>"#,
        );
        let payload = b"%PDF\x00\xff\r\n--not-a-boundary\nsecret";
        let response = multipart(&root, "report@example", payload);
        let report = decode_download(
            &response,
            r#"multipart/related; boundary="report-boundary"; type="application/xop+xml"; start="<root@example>""#,
            &ReportId::new("123").unwrap(),
            limits(1024),
        )
        .unwrap();
        assert_eq!(report.bytes.as_ref(), payload);
    }

    #[test]
    fn mtom_rejects_cid_and_report_id_mismatches_without_content_leaks() {
        let expected = ReportId::new("123").unwrap();
        let root = soap(
            r#"<xop:Include xmlns:xop="http://www.w3.org/2004/08/xop/include" href="cid:missing@example"/>"#,
        );
        let response = multipart(&root, "other-secret@example", b"sensitive-content");
        let error = decode_download(
            &response,
            r#"multipart/related; boundary=report-boundary; start="<root@example>""#,
            &expected,
            limits(1024),
        )
        .unwrap_err();
        assert!(!error.to_string().contains("sensitive"));
        assert!(!error.to_string().contains("other-secret"));

        let root = soap(
            r#"<xop:Include xmlns:xop="http://www.w3.org/2004/08/xop/include" href="cid:report@example"/>"#,
        );
        let response = multipart(&root, "report@example", b"payload");
        let error = decode_download(
            &response,
            "multipart/related; boundary=report-boundary",
            &ReportId::new("999").unwrap(),
            limits(1024),
        )
        .unwrap_err();
        assert!(matches!(error, Error::InvalidResponse { .. }));
    }

    #[test]
    fn mtom_rejects_truncation_bad_start_and_attachment_over_limit() {
        let expected = ReportId::new("123").unwrap();
        let root = soap(
            r#"<xop:Include xmlns:xop="http://www.w3.org/2004/08/xop/include" href="cid:report@example"/>"#,
        );
        let response = multipart(&root, "report@example", b"1234");
        let truncated = &response[..response.len() - 30];
        let error = decode_download(
            truncated,
            "multipart/related; boundary=report-boundary",
            &expected,
            limits(1024),
        )
        .unwrap_err();
        assert!(matches!(error, Error::InvalidResponse { .. }));

        let error = decode_download(
            &response,
            r#"multipart/related; boundary=report-boundary; start="<absent@example>""#,
            &expected,
            limits(1024),
        )
        .unwrap_err();
        assert!(matches!(error, Error::InvalidResponse { .. }));

        let error = decode_download(
            &response,
            "multipart/related; boundary=report-boundary",
            &expected,
            limits(3),
        )
        .unwrap_err();
        assert!(matches!(error, Error::ResponseTooLarge { limit: 3 }));
    }

    #[test]
    fn every_truncated_mtom_prefix_is_handled_without_a_panic() {
        let root = soap(
            r#"<xop:Include xmlns:xop="http://www.w3.org/2004/08/xop/include" href="cid:report@example"/>"#,
        );
        let response = multipart(&root, "report@example", b"binary\0payload");
        for end in 0..response.len() {
            let result = decode_download(
                &response[..end],
                "multipart/related; boundary=report-boundary",
                &ReportId::new("123").unwrap(),
                limits(1024),
            );
            if let Ok(report) = result {
                let prefix = &response[..end];
                let closing = b"\r\n--report-boundary--";
                let closing_start = find_subslice(prefix, closing)
                    .unwrap_or_else(|| panic!("incomplete prefix {end} was accepted"));
                let after_closing = closing_start + closing.len();
                assert!(
                    after_closing == prefix.len()
                        || prefix.get(after_closing..after_closing + 2) == Some(b"\r\n")
                );
                assert_eq!(report.bytes.as_ref(), b"binary\0payload");
            }
        }
        assert!(find_marker(b"X--boundary", b"--boundary", 0).is_none());
    }

    #[test]
    fn malformed_content_types_and_non_xml_inline_payloads_are_rejected() {
        let expected = ReportId::new("123").unwrap();
        assert!(
            decode_download(b"secret", "application/octet-stream", &expected, limits(1)).is_err()
        );
        let response = soap(r#"<unexpected xmlns="urn:unexpected"/>"#);
        assert!(decode_download(response.as_bytes(), "text/xml", &expected, limits(100)).is_err());
    }
}
