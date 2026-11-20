use roxmltree::{Document, ParsingOptions};

use super::legacy_ownership_sheet::parse_legacy_ownership_sheet;
use super::ownership_sheet_v3::parse_ntr_ownership_sheet;
use crate::model::reports::OwnershipSheetReport;
use crate::{Error, Result};

const NTR_NS: &str = "urn:cz:gov:cuzk:iskn:sestavy:VypisZKatastruNemovitosti:3.0";

/// Selects the parser by the actual expanded root, never by the requested format.
pub(crate) fn parse_ownership_sheet(
    bytes: &[u8],
    max_nodes: u32,
    max_depth: u32,
) -> Result<OwnershipSheetReport> {
    let xml = std::str::from_utf8(bytes)
        .map_err(|_| Error::invalid_response("ownership sheet is not UTF-8 XML"))?;
    let document = Document::parse_with_options(
        xml,
        ParsingOptions {
            allow_dtd: false,
            nodes_limit: max_nodes,
            ..ParsingOptions::default()
        },
    )
    .map_err(|_| Error::invalid_response("malformed ownership-sheet XML"))?;
    let root = document.root_element();
    match (root.tag_name().namespace(), root.tag_name().name()) {
        (None, "VypisZKatastruNemovitosti") => {
            parse_legacy_ownership_sheet(bytes, max_nodes, max_depth)
                .map(OwnershipSheetReport::Legacy)
        }
        (Some(NTR_NS), "vypisZKatastruNemovitosti") => {
            parse_ntr_ownership_sheet(bytes, max_nodes, max_depth)
                .map(Box::new)
                .map(OwnershipSheetReport::Ntr3)
        }
        _ => Err(Error::invalid_response(
            "ownership-sheet XML has an unexpected expanded root",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatch_rejects_unknown_and_wrong_namespace_roots() {
        for xml in [
            "<VypisZKatastruNemovitosti xmlns='unexpected'/>",
            "<vypisZKatastruNemovitosti/>",
            "<!DOCTYPE x><VypisZKatastruNemovitosti/>",
        ] {
            assert!(parse_ownership_sheet(xml.as_bytes(), 100, 16).is_err());
        }
    }
}
