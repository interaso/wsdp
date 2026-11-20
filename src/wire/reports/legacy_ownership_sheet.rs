//! Bounded decoding of the legacy LV 1.0 ownership-sheet report.

use std::collections::HashSet;
use std::str::FromStr;

use roxmltree::{Document, Node, ParsingOptions};

use crate::LegalRelation;
use crate::error::{Error, Result};
use crate::model::reports::legacy_base::*;
use crate::model::reports::legacy_ownership_sheet::*;
use crate::wire::reports::legacy_base::*;

const DS_NS: &str = "http://www.w3.org/2000/09/xmldsig#";
const XSI_NS: &str = "http://www.w3.org/2001/XMLSchema-instance";

/// Parses a complete legacy ownership sheet with explicit XML complexity limits.
pub(crate) fn parse_legacy_ownership_sheet(
    bytes: &[u8],
    max_nodes: u32,
    max_depth: u32,
) -> Result<LegacyOwnershipSheetReport> {
    let xml = std::str::from_utf8(bytes)
        .map_err(|_| Error::invalid_response("legacy ownership sheet is not UTF-8"))?;
    let document = Document::parse_with_options(
        xml,
        ParsingOptions {
            allow_dtd: false,
            nodes_limit: max_nodes,
            ..ParsingOptions::default()
        },
    )
    .map_err(|_| Error::invalid_response("malformed legacy ownership-sheet XML"))?;
    let root = document.root_element();
    check_depth(root, 1, max_depth)?;
    require_element(root)?;
    if root.tag_name().name() != "VypisZKatastruNemovitosti" {
        return Err(Error::invalid_response(
            "unexpected legacy ownership-sheet root",
        ));
    }

    const FIELDS: &[&str] = &[
        "LIST_TEXT",
        "LIST_ZAPATI_LV",
        "NEM_BEZ_LV",
        "PLATNOST",
        "BEZPL_PRISTUP",
        "VYHOTOVENO",
        "INFO_VYSTUP",
    ];
    let mut cursor = Cursor::all(root, FIELDS)?;
    let mut texts = None;
    let mut footer = None;
    let mut properties_without_sheet = None;
    let mut valid_at = None;
    let mut free_access = None;
    let mut created_at = None;
    let mut output_info = None;
    let mut signature = None;
    for child in elements(root) {
        if child.tag_name().namespace() == Some(DS_NS) && child.tag_name().name() == "Signature" {
            if signature.is_some() {
                return Err(invalid(
                    "Signature",
                    "singleton element occurs more than once",
                ));
            }
            signature = Some(LegacyXmlSignature {
                raw_xml: raw_xml(child, xml)?,
            });
            continue;
        }
        match cursor.singleton(child)? {
            0 => texts = Some(parse_list(child, "TEXTY", |node| parse_text(node, xml))?),
            1 => footer = Some(Some(parse_footer(child)?)),
            2 => properties_without_sheet = Some(Some(parse_properties_without_sheet(child)?)),
            3 => valid_at = Some(text(child)?),
            4 => free_access = Some(text(child)?),
            5 => created_at = Some(text(child)?),
            6 => output_info = Some(text(child)?),
            _ => unreachable!(),
        }
    }
    Ok(LegacyOwnershipSheetReport {
        texts: required(texts, "LIST_TEXT")?,
        footer: footer.unwrap_or(None),
        properties_without_sheet: properties_without_sheet.unwrap_or(None),
        valid_at: required(valid_at, "PLATNOST")?,
        free_access: required(free_access, "BEZPL_PRISTUP")?,
        created_at: required(created_at, "VYHOTOVENO")?,
        output_info,
        signature,
    })
}

fn parse_text(node: Node<'_, '_>, xml: &str) -> Result<LegacyOwnershipSheetText> {
    const FIELDS: &[&str] = &[
        "ZAHLAVI_LV",
        "VLASTNICI_JINI_OPRAVNENI",
        "POZEMKY",
        "STAVBY",
        "PRAVA_STAVBY",
        "E_NABYVACI_TITULY",
        "JEDNOTKY",
        "F_BONITY",
        "D1_UPOZORNENI",
        "ZMENY_PV",
        "PARCELY_ZE",
        "UKONY_KN",
        "JINA_PRAVA",
        "UPOZORNENI1",
        "UPOZORNENI2",
        "CASTECNY",
    ];
    let mut cursor = Cursor::all(node, FIELDS)?;
    let mut header = None;
    let mut owner_relations = None;
    let mut parcels = None;
    let mut buildings = None;
    let mut building_rights = None;
    let mut acquisition_titles = None;
    let mut units = None;
    let mut bonities = None;
    let mut notices = None;
    let mut legal_relation_changes = None;
    let mut simplified_parcel_sources = None;
    let mut operations = None;
    let mut other_right_sections = None;
    let mut warning1 = None;
    let mut warning2 = None;
    let mut partial = None;
    for child in elements(node) {
        match cursor.singleton(child)? {
            0 => header = Some(parse_header(child)?),
            1 => {
                owner_relations = Some(parse_nonempty_list(
                    child,
                    "TYP_VZTAHU",
                    parse_owner_relation,
                )?);
            }
            2 => parcels = Some(parse_list(child, "PARCELA", parse_parcel_entry)?),
            3 => {
                buildings = Some(parse_list(child, "BUDOVA", |node| {
                    parse_building_entry(node, xml)
                })?)
            }
            4 => {
                building_rights = Some(Some(parse_list(
                    child,
                    "PRAVO_STAVBY",
                    parse_building_right_entry,
                )?));
            }
            5 => {
                acquisition_titles = Some(parse_list(
                    child,
                    "E_NABYVACI_TITUL",
                    parse_acquisition_title,
                )?);
            }
            6 => units = Some(parse_list(child, "JEDNOTKA", parse_standalone_unit)?),
            7 => bonities = Some(parse_list(child, "F_BONITA", parse_bonity)?),
            8 => notices = Some(parse_notice_groups(child)?),
            9 => {
                legal_relation_changes = Some(Some(parse_list(
                    child,
                    "ZMENA_PV",
                    parse_legal_relation_change,
                )?));
            }
            10 => {
                simplified_parcel_sources = Some(parse_list(child, "ZDROJ_PARCEL_ZE", |node| {
                    parse_simplified_source(node, xml)
                })?);
            }
            11 => operations = Some(parse_operations(child)?),
            12 => other_right_sections = Some(parse_other_right_sections(child, xml)?),
            13 => warning1 = Some(text(child)?),
            14 => warning2 = Some(text(child)?),
            15 => partial = Some(text(child)?),
            _ => unreachable!(),
        }
    }
    Ok(LegacyOwnershipSheetText {
        header: required(header, "ZAHLAVI_LV")?,
        owner_relations: required(owner_relations, "VLASTNICI_JINI_OPRAVNENI")?,
        parcels: required(parcels, "POZEMKY")?,
        buildings: required(buildings, "STAVBY")?,
        building_rights: building_rights.unwrap_or(None),
        acquisition_titles: required(acquisition_titles, "E_NABYVACI_TITULY")?,
        units: required(units, "JEDNOTKY")?,
        bonities: required(bonities, "F_BONITY")?,
        notices: required(notices, "D1_UPOZORNENI")?,
        legal_relation_changes: legal_relation_changes.unwrap_or(None),
        simplified_parcel_sources: required(simplified_parcel_sources, "PARCELY_ZE")?,
        operations: required(operations, "UKONY_KN")?,
        other_right_sections: required(other_right_sections, "JINA_PRAVA")?,
        warning1: required(warning1, "UPOZORNENI1")?,
        warning2: required(warning2, "UPOZORNENI2")?,
        partial: required(partial, "CASTECNY")?,
    })
}

fn parse_header(node: Node<'_, '_>) -> Result<LegacySheetHeader> {
    const FIELDS: &[&str] = &[
        "OKRES",
        "OBEC",
        "KATASTR_UZEMI",
        "CIS_RADA",
        "TEL_ID",
        "CISLO_LV",
    ];
    let mut cursor = Cursor::sequence(node, FIELDS)?;
    let mut district = None;
    let mut municipality = None;
    let mut cadastral_area = None;
    let mut numbering_series = None;
    let mut id = None;
    let mut number = None;
    for child in elements(node) {
        match cursor.singleton(child)? {
            0 => district = Some(parse_district(child)?),
            1 => municipality = Some(parse_sheet_municipality(child)?),
            2 => cadastral_area = Some(parse_sheet_area(child)?),
            3 => numbering_series = Some(text(child)?),
            4 => id = Some(parse_legacy_id(child)?),
            5 => number = Some(text(child)?),
            _ => unreachable!(),
        }
    }
    Ok(LegacySheetHeader {
        district: required(district, "OKRES")?,
        municipality: required(municipality, "OBEC")?,
        cadastral_area: required(cadastral_area, "KATASTR_UZEMI")?,
        numbering_series: required(numbering_series, "CIS_RADA")?,
        id: required(id, "TEL_ID")?,
        number: required(number, "CISLO_LV")?,
    })
}

fn parse_district(node: Node<'_, '_>) -> Result<LegacyDistrict> {
    let mut cursor = Cursor::sequence(node, &["nuts4", "nazev"])?;
    let mut value = LegacyDistrict::default();
    for child in elements(node) {
        match cursor.singleton(child)? {
            0 => value.nuts4 = Some(text(child)?),
            1 => value.name = Some(text(child)?),
            _ => unreachable!(),
        }
    }
    Ok(value)
}

fn parse_sheet_municipality(node: Node<'_, '_>) -> Result<LegacySheetMunicipality> {
    let (code, name) = parse_string_code_name(node)?;
    Ok(LegacySheetMunicipality { code, name })
}

fn parse_sheet_area(node: Node<'_, '_>) -> Result<LegacySheetCadastralArea> {
    let mut cursor = Cursor::sequence(node, &["kod", "nazev", "nazev_puv"])?;
    let mut code = None;
    let mut name = None;
    let mut original_name = None;
    for child in elements(node) {
        match cursor.singleton(child)? {
            0 => code = Some(text(child)?),
            1 => name = Some(text(child)?),
            2 => original_name = Some(text(child)?),
            _ => unreachable!(),
        }
    }
    Ok(LegacySheetCadastralArea {
        code: required(code, "kod")?,
        name: required(name, "nazev")?,
        original_name,
    })
}

fn parse_string_code_name(node: Node<'_, '_>) -> Result<(String, String)> {
    let mut cursor = Cursor::sequence(node, &["kod", "nazev"])?;
    let mut code = None;
    let mut name = None;
    for child in elements(node) {
        match cursor.singleton(child)? {
            0 => code = Some(text(child)?),
            1 => name = Some(text(child)?),
            _ => unreachable!(),
        }
    }
    Ok((required(code, "kod")?, required(name, "nazev")?))
}

fn parse_owner_relation(node: Node<'_, '_>) -> Result<LegacyOwnerRelation> {
    let mut cursor = Cursor::sequence(node, &["TYP_VZTAHU", "SUBJEKTY"])?;
    let mut name = None;
    let mut subjects = None;
    for child in elements(node) {
        match cursor.singleton(child)? {
            0 => name = Some(text(child)?),
            1 => subjects = Some(parse_nonempty_list(child, "SUBJEKT", parse_owner)?),
            _ => unreachable!(),
        }
    }
    Ok(LegacyOwnerRelation {
        relation: LegalRelation {
            code: None,
            name: required(name, "TYP_VZTAHU")?,
        },
        subjects: required(subjects, "SUBJEKTY")?,
    })
}

fn parse_owner(node: Node<'_, '_>) -> Result<LegacyOwner> {
    const FIELDS: &[&str] = &[
        "VLA_IDENT",
        "OPSUB_TYP",
        "OPSUB_ZKRATKA",
        "OPSUB_NAZEV",
        "OPSUB_IDENT",
        "PODIL",
    ];
    let mut cursor = Cursor::sequence(node, FIELDS)?;
    let mut identification = None;
    let mut subject_kind = None;
    let mut characteristic = None;
    let mut name = None;
    let mut alternate_identification = None;
    let mut share = None;
    for child in elements(node) {
        match cursor.singleton(child)? {
            0 => identification = Some(parse_ownership_identification(child)?),
            1 => subject_kind = Some(text(child)?),
            2 => characteristic = Some(parse_characteristic_wrapper(child)?),
            3 => name = Some(parse_subject_name(child)?),
            4 => alternate_identification = Some(parse_authorized_subject(child)?),
            5 => share = Some(parse_reported_share(child)?),
            _ => unreachable!(),
        }
    }
    Ok(LegacyOwner {
        identification: required(identification, "VLA_IDENT")?,
        subject_kind,
        characteristic,
        name,
        alternate_identification,
        share: required(share, "PODIL")?,
    })
}

fn parse_parcel_entry(node: Node<'_, '_>) -> Result<LegacyParcel> {
    const FIELDS: &[&str] = &[
        "PAR_IDENT",
        "VYMERA_PARCELY",
        "DRUH_POZ",
        "ZP_VYUZITI",
        "ZP_OCHRANY",
        "PLOMBA_PARCELY",
        "SOUCASTI",
        "STAVBA_NA_VICE_PARCELACH",
        "SOUCASTI_STAVBA",
        "STAVBA_NA_PARCELE",
        "DALSI_UDAJE",
        "JEDNOTKY",
    ];
    let mut cursor = Cursor::sequence(node, FIELDS)?;
    let mut identifications = Vec::new();
    let mut area = None;
    let mut land_kind = None;
    let mut use_description = None;
    let mut protections = None;
    let mut seal = None;
    let mut components = None;
    let mut buildings_on_multiple_parcels = None;
    let mut component_buildings = None;
    let mut building_on_parcel = None;
    let mut additional_details = None;
    let mut units = None;
    for child in elements(node) {
        let index = if child.tag_name().name() == "PAR_IDENT" {
            cursor.repeated(child)?
        } else {
            cursor.singleton(child)?
        };
        match index {
            0 => identifications.push(parse_parcel_wrapper(child)?),
            1 => area = Some(text(child)?),
            2 => land_kind = Some(text(child)?),
            3 => use_description = Some(text(child)?),
            4 => protections = Some(parse_protections(child)?),
            5 => seal = Some(text(child)?),
            6 => components = Some(text(child)?),
            7 => {
                buildings_on_multiple_parcels =
                    Some(Some(parse_list(child, "parcela", parse_parcel)?));
            }
            8 => component_buildings = Some(parse_component_buildings(child)?),
            9 => building_on_parcel = Some(parse_building_wrapper(child)?),
            10 => additional_details = Some(parse_building_parts(child)?),
            11 => units = Some(Some(parse_list(child, "JEDNOTKA", parse_parcel_unit)?)),
            _ => unreachable!(),
        }
    }
    if identifications.is_empty() {
        return Err(missing("PAR_IDENT"));
    }
    Ok(LegacyParcel {
        identifications,
        area: required(area, "VYMERA_PARCELY")?,
        land_kind: required(land_kind, "DRUH_POZ")?,
        use_description,
        protections: required(protections, "ZP_OCHRANY")?,
        seal: required(seal, "PLOMBA_PARCELY")?,
        components: required(components, "SOUCASTI")?,
        buildings_on_multiple_parcels: buildings_on_multiple_parcels.unwrap_or(None),
        component_buildings: required(component_buildings, "SOUCASTI_STAVBA")?,
        building_on_parcel,
        additional_details: required(additional_details, "DALSI_UDAJE")?,
        units: units.unwrap_or(None),
    })
}

fn parse_protections(node: Node<'_, '_>) -> Result<Vec<String>> {
    parse_list(node, "zpochr", text)
}

fn parse_component_buildings(node: Node<'_, '_>) -> Result<LegacyComponentBuildings> {
    let mut cursor = Cursor::sequence(node, &["stavba", "docasna"])?;
    let mut value = LegacyComponentBuildings::default();
    for child in elements(node) {
        let index = if child.tag_name().name() == "stavba" {
            cursor.repeated(child)?
        } else {
            cursor.singleton(child)?
        };
        match index {
            0 => value.buildings.push(parse_building(child)?),
            1 => value.temporary = Some(text(child)?),
            _ => unreachable!(),
        }
    }
    if value.temporary.is_some() && value.buildings.is_empty() {
        return Err(missing("SOUCASTI_STAVBA/stavba"));
    }
    Ok(value)
}

fn parse_building_parts(node: Node<'_, '_>) -> Result<Vec<LegacyBuildingPart>> {
    require_element(node)?;
    elements(node)
        .map(|child| {
            require_element(child)?;
            match child.tag_name().name() {
                "stavba_soucasti_pozemku" => parse_part_of_parcel(child)
                    .map(Box::new)
                    .map(LegacyBuildingPart::Parcel),
                "stavba_soucasti_prava_stavby" => parse_part_of_right(child)
                    .map(Box::new)
                    .map(LegacyBuildingPart::BuildingRight),
                name => Err(unexpected(name)),
            }
        })
        .collect()
}

fn parse_part_of_parcel(node: Node<'_, '_>) -> Result<LegacyBuildingPartOfParcel> {
    let mut cursor = Cursor::sequence(node, &["parcela", "tel_id", "cislo_tel", "ku_nazev"])?;
    let mut value = LegacyBuildingPartOfParcel::default();
    for child in elements(node) {
        match cursor.singleton(child)? {
            0 => value.parcel = Some(parse_parcel(child)?),
            1 => value.ownership_sheet_id = Some(parse_legacy_id(child)?),
            2 => value.ownership_sheet_number = Some(text(child)?),
            3 => value.cadastral_area_name = Some(text(child)?),
            _ => unreachable!(),
        }
    }
    Ok(value)
}

fn parse_part_of_right(node: Node<'_, '_>) -> Result<LegacyBuildingPartOfRight> {
    let mut cursor = Cursor::sequence(node, &["tel_id", "cislo_tel", "ku_nazev"])?;
    let mut value = LegacyBuildingPartOfRight::default();
    for child in elements(node) {
        match cursor.singleton(child)? {
            0 => value.ownership_sheet_id = Some(parse_legacy_id(child)?),
            1 => value.ownership_sheet_number = Some(text(child)?),
            2 => value.cadastral_area_name = Some(text(child)?),
            _ => unreachable!(),
        }
    }
    Ok(value)
}

fn parse_parcel_unit(node: Node<'_, '_>) -> Result<LegacyParcelUnit> {
    const FIELDS: &[&str] = &[
        "JED_ZP_VYUZITI",
        "CISLO_JEDNOTKY",
        "TYP_JEDNOTKY",
        "PODIL_BA",
        "POPIS",
        "TEL_ID",
        "NA_LV",
        "VLASTNICTVI",
    ];
    let mut cursor = Cursor::sequence(node, FIELDS)?;
    let mut use_description = None;
    let mut identification = None;
    let mut unit_type = None;
    let mut building_share = None;
    let mut description = None;
    let mut ownership_sheet_id = None;
    let mut on_sheet = None;
    let mut ownerships = Vec::new();
    let mut has_children = false;
    for child in elements(node) {
        has_children = true;
        let index = if child.tag_name().name() == "VLASTNICTVI" {
            cursor.repeated(child)?
        } else {
            cursor.singleton(child)?
        };
        match index {
            0 => use_description = Some(text(child)?),
            1 => identification = Some(parse_unit_wrapper(child)?),
            2 => unit_type = Some(parse_unit_type(child)?),
            3 => building_share = Some(parse_reported_share(child)?),
            4 => description = Some(text(child)?),
            5 => ownership_sheet_id = Some(parse_legacy_id(child)?),
            6 => on_sheet = Some(parse_optional_sheet_reference(child)?),
            7 => ownerships.push(parse_unit_ownership(child, false)?),
            _ => unreachable!(),
        }
    }
    if !has_children {
        return Ok(LegacyParcelUnit::default());
    }
    if ownerships.is_empty() {
        return Err(missing("VLASTNICTVI"));
    }
    Ok(LegacyParcelUnit {
        details: Some(LegacyParcelUnitDetails {
            use_description,
            identification: required(identification, "CISLO_JEDNOTKY")?,
            unit_type: required(unit_type, "TYP_JEDNOTKY")?,
            building_share: required(building_share, "PODIL_BA")?,
            description: required(description, "POPIS")?,
            ownership_sheet_id: required(ownership_sheet_id, "TEL_ID")?,
            on_sheet: required(on_sheet, "NA_LV")?,
            ownerships,
        }),
    })
}

fn parse_unit_type(node: Node<'_, '_>) -> Result<LegacyUnitType> {
    require_element(node)?;
    let code = node
        .attribute("kod")
        .ok_or_else(|| missing("TYP_JEDNOTKY/@kod"))?
        .to_owned();
    Ok(LegacyUnitType {
        description: text(node)?,
        code,
    })
}

fn parse_unit_use(node: Node<'_, '_>) -> Result<LegacyUnitUse> {
    require_element(node)?;
    Ok(LegacyUnitUse {
        description: text(node)?,
        code: node.attribute("kod").map(str::to_owned),
    })
}

fn parse_optional_sheet_reference(node: Node<'_, '_>) -> Result<LegacyOptionalSheetReference> {
    let mut cursor = Cursor::sequence(node, &["tel_id", "cislo_tel"])?;
    let mut id = None;
    let mut number = None;
    let mut has_children = false;
    for child in elements(node) {
        has_children = true;
        match cursor.singleton(child)? {
            0 => id = Some(parse_legacy_id(child)?),
            1 => number = Some(text(child)?),
            _ => unreachable!(),
        }
    }
    Ok(LegacyOptionalSheetReference {
        details: if has_children {
            Some(LegacySheetReference {
                id: required(id, "NA_LV/tel_id")?,
                number: required(number, "NA_LV/cislo_tel")?,
            })
        } else {
            None
        },
    })
}

fn parse_sheet_reference(node: Node<'_, '_>) -> Result<LegacySheetReference> {
    let value = parse_optional_sheet_reference(node)?;
    required(value.details, "NA_LV/tel_id")
}

fn parse_unit_ownership(
    node: Node<'_, '_>,
    allow_multiple_identifications: bool,
) -> Result<LegacyUnitOwnership> {
    const FIELDS: &[&str] = &["VLA_IDENT", "OPSUB_IDENT", "OPSUB_NAZEV", "PODIL"];
    let mut cursor = Cursor::sequence(node, FIELDS)?;
    let mut identification = None;
    let mut alternate_identifications = Vec::new();
    let mut name = None;
    let mut share = None;
    for child in elements(node) {
        let index = if child.tag_name().name() == "OPSUB_IDENT" {
            cursor.repeated(child)?
        } else {
            cursor.singleton(child)?
        };
        match index {
            0 => identification = Some(parse_ownership_identification(child)?),
            1 => {
                if !allow_multiple_identifications && !alternate_identifications.is_empty() {
                    return Err(invalid(
                        "OPSUB_IDENT",
                        "singleton element occurs more than once",
                    ));
                }
                alternate_identifications.push(parse_authorized_subject(child)?);
            }
            2 => name = Some(parse_subject_name(child)?),
            3 => share = Some(parse_reported_share(child)?),
            _ => unreachable!(),
        }
    }
    Ok(LegacyUnitOwnership {
        identification: required(identification, "VLA_IDENT")?,
        alternate_identifications,
        name,
        share: required(share, "PODIL")?,
    })
}

fn parse_building_entry(node: Node<'_, '_>, xml: &str) -> Result<LegacyBuilding> {
    const FIELDS: &[&str] = &[
        "ID",
        "PLOMBA_BUDOVY",
        "KOD",
        "CISLO_DOMOVNI",
        "ZP_VYUZITI",
        "ZP_OCHRANY",
        "DOCASNA_STAVBA",
        "PARCELY",
        "JEDNOTKY",
    ];
    let mut cursor = Cursor::sequence(node, FIELDS)?;
    let mut id = None;
    let mut seal = None;
    let mut code = None;
    let mut identification = None;
    let mut use_description = None;
    let mut protections = None;
    let mut temporary = None;
    let mut parcels = None;
    let mut units = None;
    for child in elements(node) {
        match cursor.singleton(child)? {
            0 => id = Some(parse_legacy_id(child)?),
            1 => {
                seal = Some(LegacyRawXml {
                    raw_xml: raw_xml(child, xml)?,
                })
            }
            2 => code = Some(text(child)?),
            3 => identification = Some(parse_building_wrapper(child)?),
            4 => use_description = Some(text(child)?),
            5 => protections = Some(parse_protections(child)?),
            6 => temporary = Some(text(child)?),
            7 => parcels = Some(parse_list(child, "PAR_IDENT", parse_parcel_wrapper)?),
            8 => units = Some(Some(parse_list(child, "JEDNOTKA", parse_building_unit)?)),
            _ => unreachable!(),
        }
    }
    Ok(LegacyBuilding {
        id,
        seal: required(seal, "PLOMBA_BUDOVY")?,
        code: required(code, "KOD")?,
        identification: required(identification, "CISLO_DOMOVNI")?,
        use_description,
        protections: required(protections, "ZP_OCHRANY")?,
        temporary,
        parcels: required(parcels, "PARCELY")?,
        units: units.unwrap_or(None),
    })
}

fn parse_building_unit(node: Node<'_, '_>) -> Result<LegacyBuildingUnit> {
    const FIELDS: &[&str] = &[
        "JED_ZP_VYUZITI",
        "CISLO_JEDNOTKY",
        "TYP_JEDNOTKY",
        "PODIL_BA",
        "POPIS",
        "TEL_ID",
        "NA_LV",
        "VLASTNICTVI",
    ];
    let mut cursor = Cursor::sequence(node, FIELDS)?;
    let mut use_description = None;
    let mut identification = None;
    let mut unit_type = None;
    let mut building_share = None;
    let mut description = None;
    let mut ownership_sheet_id = None;
    let mut on_sheet = None;
    let mut ownerships = Vec::new();
    for child in elements(node) {
        let index = if child.tag_name().name() == "VLASTNICTVI" {
            cursor.repeated(child)?
        } else {
            cursor.singleton(child)?
        };
        match index {
            0 => use_description = Some(text(child)?),
            1 => identification = Some(parse_unit_wrapper(child)?),
            2 => unit_type = Some(text(child)?),
            3 => building_share = Some(parse_reported_share(child)?),
            4 => description = Some(text(child)?),
            5 => ownership_sheet_id = Some(parse_legacy_id(child)?),
            6 => on_sheet = Some(integer(child, "NA_LV")?),
            7 => ownerships.push(parse_unit_ownership(child, true)?),
            _ => unreachable!(),
        }
    }
    if ownerships.is_empty() {
        return Err(missing("VLASTNICTVI"));
    }
    Ok(LegacyBuildingUnit {
        use_description,
        identification: required(identification, "CISLO_JEDNOTKY")?,
        unit_type: required(unit_type, "TYP_JEDNOTKY")?,
        building_share: required(building_share, "PODIL_BA")?,
        description: required(description, "POPIS")?,
        ownership_sheet_id: required(ownership_sheet_id, "TEL_ID")?,
        on_sheet: required(on_sheet, "NA_LV")?,
        ownerships,
    })
}

fn parse_building_right_entry(node: Node<'_, '_>) -> Result<LegacyBuildingRight> {
    const FIELDS: &[&str] = &[
        "ID",
        "PLOMBA_PRAVA_STAVBY",
        "ZP_OCHRANY",
        "PLATNOST_DO",
        "SOUCASTI_STAVBA",
        "UCELY_PRAVA_STAVBY",
        "PARCELY",
        "JEDNOTKY",
    ];
    let mut cursor = Cursor::sequence(node, FIELDS)?;
    let mut id = None;
    let mut seal = None;
    let mut protections = None;
    let mut valid_until = None;
    let mut component_buildings = None;
    let mut purposes = None;
    let mut parcels = None;
    let mut units = None;
    let mut has_children = false;
    for child in elements(node) {
        has_children = true;
        match cursor.singleton(child)? {
            0 => id = Some(parse_legacy_id(child)?),
            1 => seal = Some(text(child)?),
            2 => protections = Some(parse_protections(child)?),
            3 => valid_until = Some(text(child)?),
            4 => component_buildings = Some(parse_component_buildings(child)?),
            5 => purposes = Some(parse_list(child, "UCEL", text)?),
            6 => parcels = Some(parse_list(child, "PAR_IDENT", parse_parcel_wrapper)?),
            7 => {
                units = Some(Some(parse_list(
                    child,
                    "JEDNOTKA",
                    parse_building_right_unit,
                )?));
            }
            _ => unreachable!(),
        }
    }
    if !has_children {
        return Ok(LegacyBuildingRight::default());
    }
    Ok(LegacyBuildingRight {
        details: Some(LegacyBuildingRightDetails {
            id: required(id, "ID")?,
            seal: required(seal, "PLOMBA_PRAVA_STAVBY")?,
            protections: required(protections, "ZP_OCHRANY")?,
            valid_until: required(valid_until, "PLATNOST_DO")?,
            component_buildings: required(component_buildings, "SOUCASTI_STAVBA")?,
            purposes: required(purposes, "UCELY_PRAVA_STAVBY")?,
            parcels: required(parcels, "PARCELY")?,
            units: units.unwrap_or(None),
        }),
    })
}

fn parse_building_right_unit(node: Node<'_, '_>) -> Result<LegacyBuildingRightUnit> {
    const FIELDS: &[&str] = &[
        "ZP_VYUZITI",
        "CISLO_JEDNOTKY",
        "TYP_JEDNOTKY",
        "PODIL_BA",
        "POPIS",
        "TEL_ID",
        "NA_LV",
        "VLASTNICTVI",
    ];
    let mut cursor = Cursor::sequence(node, FIELDS)?;
    let mut use_description = None;
    let mut identification = None;
    let mut unit_type = None;
    let mut building_share = None;
    let mut description = None;
    let mut ownership_sheet_id = None;
    let mut on_sheet = None;
    let mut ownerships = Vec::new();
    let mut has_children = false;
    for child in elements(node) {
        has_children = true;
        let index = if child.tag_name().name() == "VLASTNICTVI" {
            cursor.repeated(child)?
        } else {
            cursor.singleton(child)?
        };
        match index {
            0 => use_description = Some(text(child)?),
            1 => identification = Some(required_unit(child)?),
            2 => unit_type = Some(text(child)?),
            3 => building_share = Some(parse_reported_share(child)?),
            4 => description = Some(text(child)?),
            5 => ownership_sheet_id = Some(parse_legacy_id(child)?),
            6 => on_sheet = Some(parse_sheet_reference(child)?),
            7 => ownerships.push(parse_unit_ownership(child, false)?),
            _ => unreachable!(),
        }
    }
    if !has_children {
        return Ok(LegacyBuildingRightUnit::default());
    }
    if ownerships.is_empty() {
        return Err(missing("VLASTNICTVI"));
    }
    Ok(LegacyBuildingRightUnit {
        details: Some(LegacyBuildingRightUnitDetails {
            use_description,
            identification: required(identification, "CISLO_JEDNOTKY")?,
            unit_type: required(unit_type, "TYP_JEDNOTKY")?,
            building_share: required(building_share, "PODIL_BA")?,
            description: required(description, "POPIS")?,
            ownership_sheet_id: required(ownership_sheet_id, "TEL_ID")?,
            on_sheet: required(on_sheet, "NA_LV")?,
            ownerships,
        }),
    })
}

fn required_unit(node: Node<'_, '_>) -> Result<Unit> {
    let mut cursor = Cursor::sequence(node, &["jednotka"])?;
    let mut unit = None;
    for child in elements(node) {
        cursor.singleton(child)?;
        unit = Some(parse_unit(child)?);
    }
    required(unit, "jednotka")
}

fn parse_acquisition_title(node: Node<'_, '_>) -> Result<LegacyAcquisitionTitle> {
    const FIELDS: &[&str] = &[
        "POLVZ",
        "RIZENI_E",
        "NAZEV_LIST_E",
        "IDENT_RIZENI",
        "NT_PRO",
    ];
    let mut cursor = Cursor::sequence(node, FIELDS)?;
    let mut reference = None;
    let mut proceeding_text = None;
    let mut deed_name = None;
    let mut proceeding = None;
    let mut beneficiary_groups = None;
    for child in elements(node) {
        match cursor.singleton(child)? {
            0 => reference = Some(text(child)?),
            1 => proceeding_text = Some(text(child)?),
            2 => deed_name = Some(parse_deed_name(child)?),
            3 => proceeding = Some(parse_proceeding(child)?),
            4 => {
                beneficiary_groups = Some(parse_list(
                    child,
                    "OPRAV_SUBJEKT",
                    parse_acquisition_beneficiary,
                )?);
            }
            _ => unreachable!(),
        }
    }
    Ok(LegacyAcquisitionTitle {
        reference,
        proceeding_text,
        deed_name: required(deed_name, "NAZEV_LIST_E")?,
        proceeding: required(proceeding, "IDENT_RIZENI")?,
        beneficiary_groups: required(beneficiary_groups, "NT_PRO")?,
    })
}

fn parse_acquisition_beneficiary(node: Node<'_, '_>) -> Result<LegacyAcquisitionBeneficiaryGroup> {
    let mut cursor = Cursor::sequence(node, &["oprav_subjekt"])?;
    let mut subject = None;
    for child in elements(node) {
        cursor.singleton(child)?;
        subject = Some(parse_authorized_subject(child)?);
    }
    Ok(LegacyAcquisitionBeneficiaryGroup { subject })
}

fn parse_standalone_unit(node: Node<'_, '_>) -> Result<LegacyStandaloneUnit> {
    const FIELDS: &[&str] = &[
        "PLOMBA_JEDNOTKY",
        "CISLO_JEDNOTKY",
        "ZP_VYUZITI",
        "ZP_OCHRANY",
        "TYP_JEDNOTKY",
        "PODIL_B",
        "POPIS",
        "BUDOVY",
        "PRAVA_STAVBY",
        "PARCELY",
    ];
    let mut cursor = Cursor::all(node, FIELDS)?;
    let mut seal = None;
    let mut identification = None;
    let mut use_description = None;
    let mut protections = None;
    let mut unit_type = None;
    let mut building_share = None;
    let mut description = None;
    let mut buildings = None;
    let mut building_right_groups = None;
    let mut parcels = None;
    for child in elements(node) {
        match cursor.singleton(child)? {
            0 => seal = Some(text(child)?),
            1 => identification = Some(parse_unit_wrapper(child)?),
            2 => use_description = Some(parse_unit_use(child)?),
            3 => protections = Some(parse_protections(child)?),
            4 => unit_type = Some(parse_unit_type(child)?),
            5 => building_share = Some(parse_reported_share(child)?),
            6 => description = Some(text(child)?),
            7 => buildings = Some(parse_list(child, "BUDOVA", parse_unit_building)?),
            8 => building_right_groups = Some(parse_unit_building_right_groups(child)?),
            9 => parcels = Some(parse_list(child, "PARCELA", parse_unit_parcel)?),
            _ => unreachable!(),
        }
    }
    Ok(LegacyStandaloneUnit {
        seal: required(seal, "PLOMBA_JEDNOTKY")?,
        identification: required(identification, "CISLO_JEDNOTKY")?,
        use_description,
        protections: required(protections, "ZP_OCHRANY")?,
        unit_type: required(unit_type, "TYP_JEDNOTKY")?,
        building_share: required(building_share, "PODIL_B")?,
        description: required(description, "POPIS")?,
        buildings: required(buildings, "BUDOVY")?,
        building_right_groups: required(building_right_groups, "PRAVA_STAVBY")?,
        parcels: required(parcels, "PARCELY")?,
    })
}

fn parse_unit_building(node: Node<'_, '_>) -> Result<LegacyUnitBuilding> {
    let mut cursor = Cursor::sequence(node, &["JED_OPSUB", "PARCELY"])?;
    let mut subjects = Vec::new();
    let mut parcels = None;
    let mut has_children = false;
    for child in elements(node) {
        has_children = true;
        let index = if child.tag_name().name() == "JED_OPSUB" {
            cursor.repeated(child)?
        } else {
            cursor.singleton(child)?
        };
        match index {
            0 => subjects.push(parse_unit_building_subject(child)?),
            1 => parcels = Some(parse_list(child, "PAR_IDENT", parse_parcel_wrapper)?),
            _ => unreachable!(),
        }
    }
    if !has_children {
        return Ok(LegacyUnitBuilding::default());
    }
    if subjects.is_empty() {
        return Err(missing("JED_OPSUB"));
    }
    Ok(LegacyUnitBuilding {
        details: Some(LegacyUnitBuildingDetails {
            subjects,
            parcels: required(parcels, "PARCELY")?,
        }),
    })
}

fn parse_unit_building_subject(node: Node<'_, '_>) -> Result<LegacyUnitBuildingSubject> {
    // This function receives JED_OPSUB; its nested stavba is optional.
    let mut cursor = Cursor::sequence(node, &["stavba"])?;
    let mut building = None;
    for child in elements(node) {
        cursor.singleton(child)?;
        building = Some(parse_building(child)?);
    }
    Ok(LegacyUnitBuildingSubject { building })
}

fn parse_unit_building_right_groups(
    node: Node<'_, '_>,
) -> Result<Vec<LegacyUnitBuildingRightGroup>> {
    require_element(node)?;
    let mut groups = Vec::new();
    for child in elements(node) {
        require_element(child)?;
        match child.tag_name().name() {
            "PRAVO_STAVBY" => groups.push(LegacyUnitBuildingRightGroup {
                right: parse_unit_building_right(child)?,
                parcels: Vec::new(),
            }),
            "PARCELA" => {
                let group = groups
                    .last_mut()
                    .ok_or_else(|| invalid("PARCELA", "must follow PRAVO_STAVBY"))?;
                group.parcels.push(parse_unit_right_parcel(child)?);
            }
            name => return Err(unexpected(name)),
        }
    }
    Ok(groups)
}

fn parse_unit_building_right(node: Node<'_, '_>) -> Result<LegacyUnitBuildingRight> {
    require_element(node)?;
    let mut entries = Vec::new();
    let mut identification = None;
    let mut expecting_valid_until = false;
    for child in elements(node) {
        require_element(child)?;
        match child.tag_name().name() {
            "PS_IDENT" => {
                if expecting_valid_until || identification.is_some() {
                    return Err(invalid(
                        "PS_IDENT",
                        "PLATNOST_DO must follow each identification",
                    ));
                }
                identification = Some(parse_ps_identification(child)?);
                expecting_valid_until = true;
            }
            "PLATNOST_DO" => {
                if !expecting_valid_until {
                    return Err(invalid("PLATNOST_DO", "must follow PS_IDENT"));
                }
                entries.push(LegacyUnitBuildingRightEntry {
                    identification: identification.take().expect("identification was checked"),
                    valid_until: text(child)?,
                });
                expecting_valid_until = false;
            }
            name => return Err(unexpected(name)),
        }
    }
    if expecting_valid_until {
        return Err(missing("PLATNOST_DO"));
    }
    Ok(LegacyUnitBuildingRight { entries })
}

fn parse_ps_identification(node: Node<'_, '_>) -> Result<BuildingRight> {
    let mut cursor = Cursor::sequence(node, &["pravo_stavby"])?;
    let mut value = None;
    for child in elements(node) {
        cursor.singleton(child)?;
        value = Some(parse_building_right(child)?);
    }
    required(value, "pravo_stavby")
}

fn parse_unit_right_parcel(node: Node<'_, '_>) -> Result<LegacyUnitRightParcel> {
    let mut cursor = Cursor::sequence(node, &["PAR_IDENT", "DRUH_POZ", "VYMERA_PARCELY"])?;
    let mut identification = None;
    let mut land_kind = None;
    let mut area = None;
    for child in elements(node) {
        match cursor.singleton(child)? {
            0 => identification = Some(parse_parcel_wrapper(child)?),
            1 => land_kind = Some(text(child)?),
            2 => area = Some(text(child)?),
            _ => unreachable!(),
        }
    }
    Ok(LegacyUnitRightParcel {
        identification: required(identification, "PAR_IDENT")?,
        land_kind: required(land_kind, "DRUH_POZ")?,
        area: required(area, "VYMERA_PARCELY")?,
    })
}

fn parse_unit_parcel(node: Node<'_, '_>) -> Result<LegacyUnitParcel> {
    const FIELDS: &[&str] = &[
        "PAR_IDENT",
        "DRUH_POZ",
        "VYMERA_PARCELY",
        "ZP_VYUZITI",
        "SOUCASTI_STAVBA",
    ];
    let mut cursor = Cursor::sequence(node, FIELDS)?;
    let mut identification = None;
    let mut land_kind = None;
    let mut area = None;
    let mut use_description = None;
    let mut component_buildings = None;
    for child in elements(node) {
        match cursor.singleton(child)? {
            0 => identification = Some(parse_parcel_wrapper(child)?),
            1 => land_kind = Some(text(child)?),
            2 => area = Some(text(child)?),
            3 => use_description = Some(text(child)?),
            4 => component_buildings = Some(parse_component_buildings(child)?),
            _ => unreachable!(),
        }
    }
    Ok(LegacyUnitParcel {
        identification: required(identification, "PAR_IDENT")?,
        land_kind: required(land_kind, "DRUH_POZ")?,
        area: required(area, "VYMERA_PARCELY")?,
        use_description,
        component_buildings: required(component_buildings, "SOUCASTI_STAVBA")?,
    })
}

fn parse_bonity(node: Node<'_, '_>) -> Result<LegacyBonity> {
    let mut cursor = Cursor::sequence(node, &["BON_PARCELNI_CISLO", "JINE_KU", "BONITY_BPEJ"])?;
    let mut parcel = None;
    let mut other_cadastral_area = None;
    let mut entries = None;
    for child in elements(node) {
        match cursor.singleton(child)? {
            0 => parcel = Some(parse_bonity_parcel(child)?),
            1 => other_cadastral_area = Some(parse_other_area(child)?),
            2 => entries = Some(parse_list(child, "BPEJ", parse_bpej)?),
            _ => unreachable!(),
        }
    }
    Ok(LegacyBonity {
        parcel: required(parcel, "BON_PARCELNI_CISLO")?,
        other_cadastral_area: required(other_cadastral_area, "JINE_KU")?,
        entries: required(entries, "BONITY_BPEJ")?,
    })
}

fn parse_bonity_parcel(node: Node<'_, '_>) -> Result<LegacyBonityParcel> {
    const FIELDS: &[&str] = &[
        "id", "zkratka", "druh_cis", "par_cis", "podd_cis", "dil", "tel_id",
    ];
    let mut cursor = Cursor::sequence(node, FIELDS)?;
    let mut id = None;
    let mut kind_abbreviation = None;
    let mut numbering_kind = None;
    let mut number = None;
    let mut subdivision = None;
    let mut part = None;
    let mut ownership_sheet_id = None;
    let mut has_children = false;
    for child in elements(node) {
        has_children = true;
        match cursor.singleton(child)? {
            0 => id = Some(parse_legacy_id(child)?),
            1 => kind_abbreviation = Some(text(child)?),
            2 => numbering_kind = Some(text(child)?),
            3 => number = Some(text(child)?),
            4 => subdivision = Some(text(child)?),
            5 => part = Some(integer(child, "dil")?),
            6 => ownership_sheet_id = Some(parse_legacy_id(child)?),
            _ => unreachable!(),
        }
    }
    if !has_children {
        return Ok(LegacyBonityParcel::default());
    }
    Ok(LegacyBonityParcel {
        details: Some(LegacyBonityParcelDetails {
            id: required(id, "id")?,
            kind_abbreviation: required(kind_abbreviation, "zkratka")?,
            numbering_kind: required(numbering_kind, "druh_cis")?,
            number: required(number, "par_cis")?,
            subdivision,
            part,
            ownership_sheet_id: required(ownership_sheet_id, "tel_id")?,
        }),
    })
}

fn parse_other_area(node: Node<'_, '_>) -> Result<LegacyOtherCadastralArea> {
    let mut cursor = Cursor::sequence(node, &["kod", "nazev"])?;
    let mut code = None;
    let mut name = None;
    let mut has_children = false;
    for child in elements(node) {
        has_children = true;
        match cursor.singleton(child)? {
            0 => code = Some(integer(child, "kod")?),
            1 => name = Some(text(child)?),
            _ => unreachable!(),
        }
    }
    Ok(LegacyOtherCadastralArea {
        details: if has_children {
            Some(LegacyOtherCadastralAreaDetails {
                code: required(code, "JINE_KU/kod")?,
                name: required(name, "JINE_KU/nazev")?,
            })
        } else {
            None
        },
    })
}

fn parse_bpej(node: Node<'_, '_>) -> Result<LegacyBpej> {
    let mut cursor = Cursor::sequence(node, &["BPEJ_KOD", "BPEJ_VYMERA"])?;
    let mut code = None;
    let mut area = None;
    for child in elements(node) {
        match cursor.singleton(child)? {
            0 => code = Some(text(child)?),
            1 => area = Some(integer(child, "BPEJ_VYMERA")?),
            _ => unreachable!(),
        }
    }
    Ok(LegacyBpej {
        code: required(code, "BPEJ_KOD")?,
        area: required(area, "BPEJ_VYMERA")?,
    })
}

fn parse_notice_groups(node: Node<'_, '_>) -> Result<Vec<LegacyNoticeGroup>> {
    require_element(node)?;
    let mut groups = Vec::new();
    for child in elements(node) {
        require_element(child)?;
        match child.tag_name().name() {
            "NAZEV" => groups.push(LegacyNoticeGroup {
                name: text(child)?,
                proceedings: Vec::new(),
            }),
            "SEZNAM_RIZENI" => {
                let group = groups
                    .last_mut()
                    .ok_or_else(|| invalid("SEZNAM_RIZENI", "must follow NAZEV"))?;
                group.proceedings.push(parse_notice_proceeding(child)?);
            }
            name => return Err(unexpected(name)),
        }
    }
    Ok(groups)
}

fn parse_notice_proceeding(node: Node<'_, '_>) -> Result<LegacyNoticeProceeding> {
    let mut cursor = Cursor::sequence(node, &["DATUM_PODANI", "IDENT_RIZENI", "NEMOVITOSTI"])?;
    let mut submitted_at = None;
    let mut proceeding = None;
    let mut properties = None;
    let mut has_children = false;
    for child in elements(node) {
        has_children = true;
        match cursor.singleton(child)? {
            0 => submitted_at = Some(text(child)?),
            1 => proceeding = Some(parse_proceeding(child)?),
            2 => properties = Some(parse_list(child, "NEMOVITOST", parse_notice_property)?),
            _ => unreachable!(),
        }
    }
    if !has_children {
        return Ok(LegacyNoticeProceeding::default());
    }
    Ok(LegacyNoticeProceeding {
        details: Some(LegacyNoticeProceedingDetails {
            submitted_at: required(submitted_at, "DATUM_PODANI")?,
            proceeding: required(proceeding, "IDENT_RIZENI")?,
            properties: required(properties, "NEMOVITOSTI")?,
        }),
    })
}

fn parse_notice_property(node: Node<'_, '_>) -> Result<LegacyNoticeProperty> {
    const FIELDS: &[&str] = &["parcela", "stavba", "jednotka", "pravo_stavby"];
    let mut cursor = Cursor::sequence(node, FIELDS)?;
    let mut details = LegacyNoticePropertyDetails::default();
    let mut has_children = false;
    for child in elements(node) {
        has_children = true;
        match cursor.repeated(child)? {
            0 => details.parcels.push(parse_parcel(child)?),
            1 => details.buildings.push(parse_building(child)?),
            2 => details.units.push(parse_unit(child)?),
            3 => details.building_rights.push(parse_building_right(child)?),
            _ => unreachable!(),
        }
    }
    Ok(LegacyNoticeProperty {
        details: has_children.then_some(details),
    })
}

fn parse_legal_relation_change(node: Node<'_, '_>) -> Result<LegacyLegalRelationChange> {
    let mut cursor = Cursor::sequence(node, &["PLOMBA"])?;
    let mut seal = None;
    for child in elements(node) {
        cursor.singleton(child)?;
        seal = Some(parse_proceeding(child)?);
    }
    Ok(LegacyLegalRelationChange {
        seal: required(seal, "PLOMBA")?,
    })
}

fn parse_simplified_source(node: Node<'_, '_>, xml: &str) -> Result<LegacySimplifiedParcelSource> {
    let mut cursor = Cursor::sequence(node, &["KOD_ZDROJE_ZE", "ZDROJ_ZE", "PARCELY_ZE_A_PK"])?;
    let mut code = None;
    let mut source = None;
    let mut parcels = None;
    let mut has_children = false;
    for child in elements(node) {
        has_children = true;
        match cursor.singleton(child)? {
            0 => code = Some(text(child)?),
            1 => source = Some(parse_simplified_source_description(child)?),
            2 => {
                parcels = Some(parse_list(child, "PARCELA_ZE", |node| {
                    parse_simplified_parcel(node, xml)
                })?);
            }
            _ => unreachable!(),
        }
    }
    if !has_children {
        return Ok(LegacySimplifiedParcelSource::default());
    }
    Ok(LegacySimplifiedParcelSource {
        details: Some(LegacySimplifiedParcelSourceDetails {
            code: required(code, "KOD_ZDROJE_ZE")?,
            source: required(source, "ZDROJ_ZE")?,
            parcels: required(parcels, "PARCELY_ZE_A_PK")?,
        }),
    })
}

fn parse_simplified_source_description(node: Node<'_, '_>) -> Result<LegacySimplifiedSource> {
    let mut cursor = Cursor::sequence(node, &["pozemek_nazev", "pozemek_zkratka"])?;
    let mut land_name = None;
    let mut land_abbreviation = None;
    let mut has_children = false;
    for child in elements(node) {
        has_children = true;
        match cursor.singleton(child)? {
            0 => land_name = Some(text(child)?),
            1 => land_abbreviation = Some(text(child)?),
            _ => unreachable!(),
        }
    }
    Ok(LegacySimplifiedSource {
        details: if has_children {
            Some(LegacySimplifiedSourceDetails {
                land_name: required(land_name, "pozemek_nazev")?,
                land_abbreviation: required(land_abbreviation, "pozemek_zkratka")?,
            })
        } else {
            None
        },
    })
}

fn parse_simplified_parcel(node: Node<'_, '_>, xml: &str) -> Result<LegacySimplifiedParcel> {
    const FIELDS: &[&str] = &[
        "DIL_PARCELY_ZE",
        "PARCELNI_CISLO_ZE",
        "PUV_NAZEV_KU",
        "TYP_1",
        "VYMERA_PARCELY_ZE",
        "PLOMBA_PARCELY_1",
    ];
    let mut cursor = Cursor::sequence(node, FIELDS)?;
    let mut part = None;
    let mut identification = None;
    let mut original_cadastral_area = None;
    let mut kind = None;
    let mut area = None;
    let mut seal = None;
    for child in elements(node) {
        match cursor.singleton(child)? {
            0 => {
                part = Some(LegacyRawXml {
                    raw_xml: raw_xml(child, xml)?,
                });
            }
            1 => identification = Some(parse_parcel_wrapper(child)?),
            2 => original_cadastral_area = Some(parse_original_area(child)?),
            3 => kind = Some(text(child)?),
            4 => area = Some(parse_arbitrary_integer(child, "VYMERA_PARCELY_ZE")?),
            5 => seal = Some(text(child)?),
            _ => unreachable!(),
        }
    }
    Ok(LegacySimplifiedParcel {
        part: required(part, "DIL_PARCELY_ZE")?,
        identification: required(identification, "PARCELNI_CISLO_ZE")?,
        original_cadastral_area: required(original_cadastral_area, "PUV_NAZEV_KU")?,
        kind: required(kind, "TYP_1")?,
        area: required(area, "VYMERA_PARCELY_ZE")?,
        seal: required(seal, "PLOMBA_PARCELY_1")?,
    })
}

fn parse_original_area(node: Node<'_, '_>) -> Result<LegacyOriginalCadastralArea> {
    let mut cursor = Cursor::sequence(node, &["kod", "nazev"])?;
    let mut value = LegacyOriginalCadastralArea::default();
    for child in elements(node) {
        match cursor.singleton(child)? {
            0 => value.code = Some(integer(child, "kod")?),
            1 => value.name = Some(text(child)?),
            _ => unreachable!(),
        }
    }
    Ok(value)
}

fn parse_arbitrary_integer(node: Node<'_, '_>, field: &'static str) -> Result<String> {
    let raw = text(node)?;
    let normalized = trim_xml_whitespace(&raw);
    let digits = normalized
        .strip_prefix('+')
        .or_else(|| normalized.strip_prefix('-'))
        .unwrap_or(normalized);
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(invalid(field, "invalid xs:integer lexical value"));
    }
    Ok(raw)
}

fn parse_operations(node: Node<'_, '_>) -> Result<Vec<LegacyCadastralOperation>> {
    parse_list(node, "UKON_KN", parse_operation)
}

fn parse_operation(node: Node<'_, '_>) -> Result<LegacyCadastralOperation> {
    let mut cursor = Cursor::sequence(node, &["IDENT_RIZENI", "RIZ_TYP_PREDMETU"])?;
    let mut proceeding = None;
    let mut subject_matters = None;
    for child in elements(node) {
        let index = if child.tag_name().name() == "RIZ_TYP_PREDMETU" {
            cursor.repeated(child)?
        } else {
            cursor.singleton(child)?
        };
        match index {
            0 => proceeding = Some(text(child)?),
            1 => {
                subject_matters
                    .get_or_insert_with(Vec::new)
                    .push(parse_operation_subject_matter(child)?);
            }
            _ => unreachable!(),
        }
    }
    Ok(LegacyCadastralOperation {
        proceeding: required(proceeding, "IDENT_RIZENI")?,
        subject_matters,
    })
}

fn parse_operation_subject_matter(node: Node<'_, '_>) -> Result<LegacyOperationSubjectMatter> {
    let mut cursor = Cursor::sequence(node, &["NAZEV_RIZENI"])?;
    let mut value = LegacyOperationSubjectMatter::default();
    for child in elements(node) {
        cursor.singleton(child)?;
        value.name = Some(text(child)?);
    }
    Ok(value)
}

fn parse_other_right_sections(
    node: Node<'_, '_>,
    xml: &str,
) -> Result<Vec<LegacyOtherRightsSection>> {
    let mut cursor = Cursor::sequence(node, &["SEKCE_LIST"])?;
    let mut sections = Vec::new();
    let mut seen = HashSet::new();
    for child in elements(node) {
        cursor.repeated(child)?;
        let section = parse_deed_section(child)?;
        if !seen.insert(section.as_str().to_owned()) {
            return Err(invalid("SEKCE", "section values must be unique"));
        }
        let relations = parse_single(child, "JPV_LIST", |list| {
            parse_list(list, "JPV", |node| parse_other_right(node, xml))
        })?;
        sections.push(LegacyOtherRightsSection { section, relations });
    }
    if sections.len() != 3 {
        return Err(invalid(
            "JINA_PRAVA",
            "exactly three SEKCE_LIST elements are required",
        ));
    }
    Ok(sections)
}

fn parse_other_right(node: Node<'_, '_>, xml: &str) -> Result<LegacyOtherRight> {
    const FIELDS: &[&str] = &[
        "HJPV_ID",
        "TYP_PRAVNIHO_VZTAHU",
        "PORADI_K",
        "NEZOBRAZ_K_NEM",
        "NADRIZ_HJPV_ID",
        "JPV_POPISY",
        "JPV_OPRAV_PRO_OS",
        "JPV_OPRAV_PRO_NEM",
        "JPV_POVIN_K_OS",
        "JPV_POVIN_K_NEM",
        "JPV_LISTINY",
    ];
    let mut cursor = Cursor::sequence(node, FIELDS)?;
    let mut id = None;
    let mut relation = None;
    let mut priority = None;
    let mut hide_obligated_property = None;
    let mut parent_id = None;
    let mut descriptions = None;
    let mut benefiting_subjects = None;
    let mut benefiting_properties = None;
    let mut obligated_subjects = None;
    let mut obligated_properties = None;
    let mut deeds = None;
    for child in elements(node) {
        match cursor.singleton(child)? {
            0 => id = Some(parse_legacy_id(child)?),
            1 => {
                relation = Some(LegalRelation {
                    code: None,
                    name: text(child)?,
                });
            }
            2 => priority = Some(Some(parse_priorities(child)?)),
            3 => hide_obligated_property = Some(text(child)?),
            4 => parent_id = Some(parse_legacy_id(child)?),
            5 => {
                descriptions = Some(parse_list(
                    child,
                    "JPV_POPIS",
                    parse_other_right_description,
                )?)
            }
            6 => {
                benefiting_subjects =
                    Some(parse_list(child, "JPV_PRO_OS", parse_benefiting_subject)?);
            }
            7 => {
                benefiting_properties = Some(Some(parse_list(
                    child,
                    "JPV_PRO_NEM",
                    parse_benefiting_property,
                )?));
            }
            8 => {
                obligated_subjects = Some(parse_list(child, "JPV_K_OS", parse_obligated_subject)?);
            }
            9 => {
                obligated_properties = Some(Some(parse_list(
                    child,
                    "JPV_K_NEM",
                    parse_obligated_property,
                )?));
            }
            10 => {
                deeds = Some(parse_list(child, "JPV_LISTINA", |node| {
                    parse_other_right_deed(node, xml)
                })?);
            }
            _ => unreachable!(),
        }
    }
    Ok(LegacyOtherRight {
        id: required(id, "HJPV_ID")?,
        relation: required(relation, "TYP_PRAVNIHO_VZTAHU")?,
        priority: priority.unwrap_or(None),
        hide_obligated_property,
        parent_id,
        descriptions: required(descriptions, "JPV_POPISY")?,
        benefiting_subjects: required(benefiting_subjects, "JPV_OPRAV_PRO_OS")?,
        benefiting_properties: benefiting_properties.unwrap_or(None),
        obligated_subjects: required(obligated_subjects, "JPV_POVIN_K_OS")?,
        obligated_properties: obligated_properties.unwrap_or(None),
        deeds: required(deeds, "JPV_LISTINY")?,
    })
}

fn parse_priorities(node: Node<'_, '_>) -> Result<Vec<LegacyRelationPriority>> {
    const FIELDS: &[&str] = &["datum", "text_o_prednostnim_poradi"];
    require_element(node)?;
    let mut values = Vec::new();
    let mut previous = None;
    for child in elements(node) {
        let index = field_index(child, FIELDS)?;
        if previous.is_none_or(|previous| index <= previous) {
            values.push(LegacyRelationPriority::default());
        }
        previous = Some(index);
        let value = values
            .last_mut()
            .expect("a priority group was just inserted");
        match index {
            0 => value.date = Some(text(child)?),
            1 => value.text = Some(text(child)?),
            _ => unreachable!(),
        }
    }
    Ok(values)
}

fn parse_other_right_description(node: Node<'_, '_>) -> Result<LegacyOtherRightDescription> {
    let mut cursor = Cursor::sequence(node, &["JPV_RIZENI", "POPIS_PRAVNIHO_VZTAHU"])?;
    let mut proceedings = None;
    let mut description = None;
    for child in elements(node) {
        match cursor.singleton(child)? {
            0 => {
                proceedings = Some(Some(parse_list(
                    child,
                    "JPV_RIZ",
                    parse_description_proceeding,
                )?));
            }
            1 => description = Some(text(child)?),
            _ => unreachable!(),
        }
    }
    Ok(LegacyOtherRightDescription {
        proceedings: proceedings.unwrap_or(None),
        text: required(description, "POPIS_PRAVNIHO_VZTAHU")?,
    })
}

fn parse_description_proceeding(node: Node<'_, '_>) -> Result<LegacyDescriptionProceeding> {
    let mut cursor = Cursor::sequence(node, &["IDENT_RIZENI"])?;
    let mut proceeding = None;
    for child in elements(node) {
        cursor.singleton(child)?;
        proceeding = Some(parse_proceeding(child)?);
    }
    Ok(LegacyDescriptionProceeding { proceeding })
}

fn parse_benefiting_subject(node: Node<'_, '_>) -> Result<LegacyBenefitingSubject> {
    let mut cursor = Cursor::sequence(node, &["PRO_IDENT_OS", "PODIL_POHLEDAVKA"])?;
    let mut identification = None;
    let mut creditor_share = None;
    for child in elements(node) {
        match cursor.singleton(child)? {
            0 => identification = Some(parse_subject_property_identification(child)?),
            1 => creditor_share = Some(text(child)?),
            _ => unreachable!(),
        }
    }
    Ok(LegacyBenefitingSubject {
        identification,
        creditor_share: required(creditor_share, "PODIL_POHLEDAVKA")?,
    })
}

fn parse_benefiting_property(node: Node<'_, '_>) -> Result<LegacyBenefitingProperty> {
    parse_optional_identification(node, "PRO_IDENT_NEM")
        .map(|identification| LegacyBenefitingProperty { identification })
}

fn parse_obligated_subject(node: Node<'_, '_>) -> Result<LegacyObligatedSubject> {
    parse_optional_identification(node, "K_IDENT_OS")
        .map(|identification| LegacyObligatedSubject { identification })
}

fn parse_optional_identification(
    node: Node<'_, '_>,
    name: &'static str,
) -> Result<Option<SubjectPropertyIdentification>> {
    let fields = [name];
    let mut cursor = Cursor::sequence(node, &fields)?;
    let mut identification = None;
    for child in elements(node) {
        cursor.singleton(child)?;
        identification = Some(parse_subject_property_identification(child)?);
    }
    Ok(identification)
}

fn parse_obligated_property(node: Node<'_, '_>) -> Result<LegacyObligatedProperty> {
    let identifications = parse_list(node, "K_IDENT_NEM", parse_subject_property_identification)?;
    Ok(LegacyObligatedProperty { identifications })
}

fn parse_other_right_deed(node: Node<'_, '_>, xml: &str) -> Result<LegacyOtherRightDeed> {
    let mut cursor = Cursor::sequence(
        node,
        &["NAZEV_LIST", "IDENT_RIZENI", "LISTIN_ID", "POPIS_LIST"],
    )?;
    let mut deed_name = None;
    let mut proceeding = None;
    let mut id = None;
    let mut description = None;
    for child in elements(node) {
        match cursor.singleton(child)? {
            0 => deed_name = Some(parse_deed_name(child)?),
            1 => proceeding = Some(parse_proceeding(child)?),
            2 => id = Some(parse_legacy_id(child)?),
            3 => {
                description = Some(LegacyRawXml {
                    raw_xml: raw_xml(child, xml)?,
                });
            }
            _ => unreachable!(),
        }
    }
    Ok(LegacyOtherRightDeed {
        deed_name: required(deed_name, "NAZEV_LIST")?,
        proceeding: required(proceeding, "IDENT_RIZENI")?,
        id: required(id, "LISTIN_ID")?,
        description: required(description, "POPIS_LIST")?,
    })
}

fn parse_footer(node: Node<'_, '_>) -> Result<LegacyOwnershipSheetFooter> {
    let mut cursor = Cursor::sequence(node, &["ZAPATI_LV"])?;
    let mut workplace = None;
    for child in elements(node) {
        cursor.singleton(child)?;
        let mut inner = Cursor::sequence(child, &["KODKP", "KP"])?;
        let mut code = None;
        let mut name = None;
        for grandchild in elements(child) {
            match inner.singleton(grandchild)? {
                0 => code = Some(integer(grandchild, "KODKP")?),
                1 => name = Some(text(grandchild)?),
                _ => unreachable!(),
            }
        }
        workplace = Some(LegacyWorkplaceFooter {
            workplace_code: required(code, "KODKP")?,
            workplace: required(name, "KP")?,
        });
    }
    Ok(LegacyOwnershipSheetFooter { workplace })
}

fn parse_properties_without_sheet(node: Node<'_, '_>) -> Result<LegacyPropertiesWithoutSheet> {
    let mut cursor = Cursor::sequence(node, &["NEMOVITOST"])?;
    let mut group = None;
    for child in elements(node) {
        cursor.singleton(child)?;
        let parcels = parse_list(child, "PARCELA_BEZ_LV", |parcel| {
            let mut inner = Cursor::sequence(parcel, &["parcela"])?;
            let mut value = None;
            for nested in elements(parcel) {
                inner.singleton(nested)?;
                value = Some(parse_parcel(nested)?);
            }
            Ok(LegacyParcelWithoutSheet { parcel: value })
        })?;
        group = Some(LegacyPropertyWithoutSheetGroup { parcels });
    }
    Ok(LegacyPropertiesWithoutSheet { group })
}

fn parse_list<T, F>(node: Node<'_, '_>, name: &'static str, parser: F) -> Result<Vec<T>>
where
    F: for<'a, 'input> FnMut(Node<'a, 'input>) -> Result<T>,
{
    let fields = [name];
    let mut cursor = Cursor::sequence(node, &fields)?;
    let mut values = Vec::new();
    let mut parser = parser;
    for child in elements(node) {
        cursor.repeated(child)?;
        values.push(parser(child)?);
    }
    Ok(values)
}

fn parse_nonempty_list<T, F>(node: Node<'_, '_>, name: &'static str, parser: F) -> Result<Vec<T>>
where
    F: for<'a, 'input> FnMut(Node<'a, 'input>) -> Result<T>,
{
    let values = parse_list(node, name, parser)?;
    if values.is_empty() {
        return Err(invalid(name, "at least one element is required"));
    }
    Ok(values)
}

fn parse_single<T, F>(node: Node<'_, '_>, name: &'static str, parser: F) -> Result<T>
where
    F: for<'a, 'input> FnOnce(Node<'a, 'input>) -> Result<T>,
{
    let fields = [name];
    let mut cursor = Cursor::sequence(node, &fields)?;
    let mut result = None;
    let mut parser = Some(parser);
    for child in elements(node) {
        cursor.singleton(child)?;
        result = Some(parser.take().expect("singleton parser is called once")(
            child,
        )?);
    }
    required(result, name)
}

fn raw_xml(node: Node<'_, '_>, xml: &str) -> Result<String> {
    xml.get(node.range())
        .map(str::to_owned)
        .ok_or_else(|| Error::invalid_response("could not retain raw legacy XML"))
}

fn text(node: Node<'_, '_>) -> Result<String> {
    require_element(node)?;
    if node
        .attribute((XSI_NS, "nil"))
        .is_some_and(|value| value == "true" || value == "1")
    {
        return Err(invalid(node.tag_name().name(), "element is not nillable"));
    }
    let mut value = String::new();
    for child in node.children() {
        if child.is_element() {
            return Err(invalid(
                node.tag_name().name(),
                "simple content cannot contain an element",
            ));
        }
        if child.is_text()
            && let Some(part) = child.text()
        {
            value.push_str(part);
        }
    }
    Ok(value)
}

fn integer<T: FromStr>(node: Node<'_, '_>, field: &'static str) -> Result<T> {
    trim_xml_whitespace(&text(node)?)
        .parse()
        .map_err(|_| invalid(field, "integer is invalid or outside its XSD range"))
}

fn trim_xml_whitespace(value: &str) -> &str {
    value.trim_matches(|character| matches!(character, ' ' | '\t' | '\r' | '\n'))
}

fn elements<'a, 'input>(node: Node<'a, 'input>) -> impl Iterator<Item = Node<'a, 'input>> {
    node.children().filter(|child| child.is_element())
}

fn field_index(node: Node<'_, '_>, fields: &[&str]) -> Result<usize> {
    require_element(node)?;
    let name = node.tag_name().name();
    fields
        .iter()
        .position(|candidate| *candidate == name)
        .ok_or_else(|| unexpected(name))
}

struct Cursor<'a> {
    fields: &'a [&'static str],
    ordered: bool,
    last: Option<usize>,
    seen: Vec<bool>,
}

impl<'a> Cursor<'a> {
    fn sequence(node: Node<'_, '_>, fields: &'a [&'static str]) -> Result<Self> {
        Self::new(node, fields, true)
    }

    fn all(node: Node<'_, '_>, fields: &'a [&'static str]) -> Result<Self> {
        Self::new(node, fields, false)
    }

    fn new(node: Node<'_, '_>, fields: &'a [&'static str], ordered: bool) -> Result<Self> {
        require_element(node)?;
        Ok(Self {
            fields,
            ordered,
            last: None,
            seen: vec![false; fields.len()],
        })
    }

    fn singleton(&mut self, node: Node<'_, '_>) -> Result<usize> {
        let index = self.enter(node)?;
        if self.seen[index] {
            return Err(invalid(
                self.fields[index],
                "singleton element occurs more than once",
            ));
        }
        self.seen[index] = true;
        Ok(index)
    }

    fn repeated(&mut self, node: Node<'_, '_>) -> Result<usize> {
        self.enter(node)
    }

    fn enter(&mut self, node: Node<'_, '_>) -> Result<usize> {
        let index = field_index(node, self.fields)?;
        if self.ordered && self.last.is_some_and(|last| index < last) {
            return Err(invalid(
                self.fields[index],
                "sequence element is out of order",
            ));
        }
        self.last = Some(index);
        Ok(index)
    }
}

fn check_depth(node: Node<'_, '_>, depth: u32, maximum: u32) -> Result<()> {
    if depth > maximum {
        return Err(Error::invalid_response(
            "legacy ownership sheet exceeds XML depth limit",
        ));
    }
    for child in elements(node) {
        check_depth(child, depth.saturating_add(1), maximum)?;
    }
    Ok(())
}

fn require_element(node: Node<'_, '_>) -> Result<()> {
    if node.is_element() && node.tag_name().namespace().is_none() {
        Ok(())
    } else {
        Err(Error::invalid_response(
            "legacy ownership-sheet elements must have no XML namespace",
        ))
    }
}

fn required<T>(value: Option<T>, field: &str) -> Result<T> {
    value.ok_or_else(|| missing(field))
}

fn missing(field: &str) -> Error {
    invalid(field, "required element is missing")
}

fn unexpected(name: &str) -> Error {
    Error::invalid_response(format!("unexpected legacy ownership-sheet element {name}"))
}

fn invalid(field: &str, message: &str) -> Error {
    Error::invalid_response(format!(
        "invalid legacy ownership-sheet element {field}: {message}"
    ))
}

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;

    use super::*;

    const NODES: u32 = 100_000;
    const DEPTH: u32 = 64;

    fn proceeding() -> &'static str {
        "<IDENT_RIZENI><typriz_kod>raw</typriz_kod><poradove_cislo>4294967295</poradove_cislo><rok>65535</rok><prares_kod>65535</prares_kod></IDENT_RIZENI>"
    }

    fn deed(element: &str) -> String {
        format!(
            "<{element}><tlist>raw deed</tlist><dalsi_udaje>additional</dalsi_udaje><por_cis_zhot>raw serial</por_cis_zhot><popis>description</popis><vystav_txt>issued</vystav_txt><prav_moc_txt>force</prav_moc_txt><podani_txt>submission</podani_txt><vykonatelnost_txt>enforceable</vykonatelnost_txt><podani_z_txt>submission-z</podani_z_txt><zplatneni_txt>effective</zplatneni_txt><prares_nazev>workplace</prares_nazev></{element}>"
        )
    }

    fn header() -> &'static str {
        "<ZAHLAVI_LV><OKRES><nuts4>NUTS</nuts4><nazev>District</nazev></OKRES><OBEC><kod> 000raw </kod><nazev>Municipality</nazev></OBEC><KATASTR_UZEMI><kod>area raw</kod><nazev>Area</nazev><nazev_puv>Old area</nazev_puv></KATASTR_UZEMI><CIS_RADA>numbering</CIS_RADA><TEL_ID/><CISLO_LV> 7 </CISLO_LV></ZAHLAVI_LV>"
    }

    fn owner_relations() -> &'static str {
        "<VLASTNICI_JINI_OPRAVNENI><TYP_VZTAHU><TYP_VZTAHU>Vlastnické právo</TYP_VZTAHU><SUBJEKTY><SUBJEKT><VLA_IDENT><oprav_subjekt><id>1</id><nazev>Subject</nazev></oprav_subjekt><parcela><id/></parcela><oprav_subjekt/></VLA_IDENT><OPSUB_TYP>raw-kind</OPSUB_TYP><OPSUB_ZKRATKA><char_os><kod>99</kod><zkratka>raw</zkratka><zkratka_alv>raw-alv</zkratka_alv></char_os></OPSUB_ZKRATKA><OPSUB_NAZEV><id>2</id><nazev>Organization</nazev><prijmeni>Surname</prijmeni><jmeno>Given</jmeno></OPSUB_NAZEV><OPSUB_IDENT><id>2</id><ico>000raw</ico><adresa><obec>Municipality</obec></adresa><id>3</id></OPSUB_IDENT><PODIL><citatel>-1</citatel><jmenovatel>0</jmenovatel></PODIL></SUBJEKT></SUBJEKTY></TYP_VZTAHU></VLASTNICI_JINI_OPRAVNENI>"
    }

    fn unit_ownership(extra_ident: &str) -> String {
        format!(
            "<VLASTNICTVI><VLA_IDENT><parcela/><jednotka><id/></jednotka></VLA_IDENT><OPSUB_IDENT><id>4</id></OPSUB_IDENT>{extra_ident}<OPSUB_NAZEV><nazev>Owner</nazev></OPSUB_NAZEV><PODIL><jmenovatel>-3</jmenovatel></PODIL></VLASTNICTVI>"
        )
    }

    fn parcel() -> String {
        let ownership = unit_ownership("");
        format!(
            r#"<POZEMKY><PARCELA><PAR_IDENT/><PAR_IDENT><parcela><id>10</id><par_cis> 1 </par_cis></parcela></PAR_IDENT><VYMERA_PARCELY> lexical area </VYMERA_PARCELY><DRUH_POZ>land</DRUH_POZ><ZP_VYUZITI>use</ZP_VYUZITI><ZP_OCHRANY><zpochr>one</zpochr><zpochr>two</zpochr></ZP_OCHRANY><PLOMBA_PARCELY>seal</PLOMBA_PARCELY><SOUCASTI>raw</SOUCASTI><STAVBA_NA_VICE_PARCELACH><parcela><id>11</id></parcela><parcela><id>12</id></parcela></STAVBA_NA_VICE_PARCELACH><SOUCASTI_STAVBA><stavba><id>13</id></stavba><stavba><id>14</id></stavba><docasna>raw</docasna></SOUCASTI_STAVBA><STAVBA_NA_PARCELE><stavba><id>15</id></stavba></STAVBA_NA_PARCELE><DALSI_UDAJE><stavba_soucasti_pozemku><parcela><id>16</id></parcela><tel_id>17</tel_id><cislo_tel>18</cislo_tel><ku_nazev>Area</ku_nazev></stavba_soucasti_pozemku><stavba_soucasti_prava_stavby><tel_id/><cislo_tel>19</cislo_tel><ku_nazev>Right area</ku_nazev></stavba_soucasti_prava_stavby><stavba_soucasti_pozemku/></DALSI_UDAJE><JEDNOTKY><JEDNOTKA/><JEDNOTKA><JED_ZP_VYUZITI>unit use</JED_ZP_VYUZITI><CISLO_JEDNOTKY/><TYP_JEDNOTKY kod="raw-type">unit type</TYP_JEDNOTKY><PODIL_BA><citatel>2</citatel><jmenovatel>3</jmenovatel></PODIL_BA><POPIS>unit description</POPIS><TEL_ID>20</TEL_ID><NA_LV/>{ownership}</JEDNOTKA></JEDNOTKY></PARCELA></POZEMKY>"#
        )
    }

    fn buildings() -> String {
        let ownership = unit_ownership("<OPSUB_IDENT><ico>raw</ico></OPSUB_IDENT>");
        format!(
            r#"<STAVBY><BUDOVA><ID/><PLOMBA_BUDOVY attr="raw">seal<child/></PLOMBA_BUDOVY><KOD>raw-code</KOD><CISLO_DOMOVNI/><ZP_VYUZITI>building use</ZP_VYUZITI><ZP_OCHRANY><zpochr>protection</zpochr></ZP_OCHRANY><DOCASNA_STAVBA>raw</DOCASNA_STAVBA><PARCELY><PAR_IDENT/><PAR_IDENT><parcela><id>21</id></parcela></PAR_IDENT></PARCELY><JEDNOTKY><JEDNOTKA><JED_ZP_VYUZITI>unit use</JED_ZP_VYUZITI><CISLO_JEDNOTKY><jednotka><id>22</id></jednotka></CISLO_JEDNOTKY><TYP_JEDNOTKY>plain type</TYP_JEDNOTKY><PODIL_BA/><POPIS>description</POPIS><TEL_ID>23</TEL_ID><NA_LV>4294967295</NA_LV>{ownership}</JEDNOTKA></JEDNOTKY></BUDOVA></STAVBY>"#
        )
    }

    fn building_rights() -> String {
        let ownership = unit_ownership("");
        format!(
            r#"<PRAVA_STAVBY><PRAVO_STAVBY/><PRAVO_STAVBY><ID>24</ID><PLOMBA_PRAVA_STAVBY>seal</PLOMBA_PRAVA_STAVBY><ZP_OCHRANY><zpochr>protection</zpochr></ZP_OCHRANY><PLATNOST_DO>valid until</PLATNOST_DO><SOUCASTI_STAVBA><stavba><id>25</id></stavba><docasna>n</docasna></SOUCASTI_STAVBA><UCELY_PRAVA_STAVBY><UCEL>one</UCEL><UCEL>two</UCEL></UCELY_PRAVA_STAVBY><PARCELY><PAR_IDENT><parcela><id>26</id></parcela></PAR_IDENT></PARCELY><JEDNOTKY><JEDNOTKA/><JEDNOTKA><ZP_VYUZITI>use</ZP_VYUZITI><CISLO_JEDNOTKY><jednotka><id>27</id></jednotka></CISLO_JEDNOTKY><TYP_JEDNOTKY>type</TYP_JEDNOTKY><PODIL_BA><citatel>4</citatel></PODIL_BA><POPIS>description</POPIS><TEL_ID>28</TEL_ID><NA_LV><tel_id>29</tel_id><cislo_tel>30</cislo_tel></NA_LV>{ownership}</JEDNOTKA></JEDNOTKY></PRAVO_STAVBY></PRAVA_STAVBY>"#
        )
    }

    fn acquisition_titles() -> String {
        let deed = deed("NAZEV_LIST_E");
        format!(
            "<E_NABYVACI_TITULY><E_NABYVACI_TITUL><POLVZ>reference</POLVZ><RIZENI_E>proceeding text</RIZENI_E>{deed}{}<NT_PRO><OPRAV_SUBJEKT/><OPRAV_SUBJEKT><oprav_subjekt><id>31</id><adresa/></oprav_subjekt></OPRAV_SUBJEKT></NT_PRO></E_NABYVACI_TITUL></E_NABYVACI_TITULY>",
            proceeding()
        )
    }

    fn standalone_units() -> String {
        let right_parcel = "<PARCELA><PAR_IDENT/><DRUH_POZ>right land</DRUH_POZ><VYMERA_PARCELY>right area</VYMERA_PARCELY></PARCELA>";
        format!(
            r#"<JEDNOTKY><JEDNOTKA><PLOMBA_JEDNOTKY>seal</PLOMBA_JEDNOTKY><CISLO_JEDNOTKY/><ZP_VYUZITI kod="raw-use">use</ZP_VYUZITI><ZP_OCHRANY><zpochr>protection</zpochr></ZP_OCHRANY><TYP_JEDNOTKY kod="raw-type">type</TYP_JEDNOTKY><PODIL_B><citatel>-5</citatel><jmenovatel>6</jmenovatel></PODIL_B><POPIS>description</POPIS><BUDOVY><BUDOVA/><BUDOVA><JED_OPSUB/><JED_OPSUB><stavba><id>32</id></stavba></JED_OPSUB><PARCELY><PAR_IDENT/><PAR_IDENT><parcela><id>33</id></parcela></PAR_IDENT></PARCELY></BUDOVA></BUDOVY><PRAVA_STAVBY><PRAVO_STAVBY/><PRAVO_STAVBY><PS_IDENT><pravo_stavby><id>34</id><parcela/><parcela><id>35</id></parcela></pravo_stavby></PS_IDENT><PLATNOST_DO>until one</PLATNOST_DO><PS_IDENT><pravo_stavby><id>36</id></pravo_stavby></PS_IDENT><PLATNOST_DO>until two</PLATNOST_DO></PRAVO_STAVBY>{right_parcel}{right_parcel}</PRAVA_STAVBY><PARCELY><PARCELA><PAR_IDENT><parcela><id>37</id></parcela></PAR_IDENT><DRUH_POZ>land</DRUH_POZ><VYMERA_PARCELY>area</VYMERA_PARCELY><ZP_VYUZITI>use</ZP_VYUZITI><SOUCASTI_STAVBA><stavba><id>38</id></stavba><docasna>raw</docasna></SOUCASTI_STAVBA></PARCELA></PARCELY></JEDNOTKA></JEDNOTKY>"#
        )
    }

    fn bonities() -> &'static str {
        "<F_BONITY><F_BONITA><BON_PARCELNI_CISLO/><JINE_KU/><BONITY_BPEJ/></F_BONITA><F_BONITA><BON_PARCELNI_CISLO><id>39</id><zkratka>raw</zkratka><druh_cis>raw</druh_cis><par_cis>40</par_cis><podd_cis>1</podd_cis><dil>-128</dil><tel_id>41</tel_id></BON_PARCELNI_CISLO><JINE_KU><kod>42</kod><nazev>Other area</nazev></JINE_KU><BONITY_BPEJ><BPEJ><BPEJ_KOD>raw</BPEJ_KOD><BPEJ_VYMERA>43</BPEJ_VYMERA></BPEJ></BONITY_BPEJ></F_BONITA></F_BONITY>"
    }

    fn notices() -> &'static str {
        "<D1_UPOZORNENI><NAZEV>first</NAZEV><SEZNAM_RIZENI/><SEZNAM_RIZENI><DATUM_PODANI>submitted</DATUM_PODANI><IDENT_RIZENI/><NEMOVITOSTI><NEMOVITOST/><NEMOVITOST><parcela/><parcela><id>44</id></parcela><stavba/><jednotka><id>45</id></jednotka><pravo_stavby/></NEMOVITOST></NEMOVITOSTI></SEZNAM_RIZENI><NAZEV>second</NAZEV></D1_UPOZORNENI>"
    }

    fn simplified_sources() -> String {
        let huge = "9".repeat(120);
        format!(
            r#"<PARCELY_ZE><ZDROJ_PARCEL_ZE/><ZDROJ_PARCEL_ZE><KOD_ZDROJE_ZE>raw-source</KOD_ZDROJE_ZE><ZDROJ_ZE><pozemek_nazev>land</pozemek_nazev><pozemek_zkratka>abbr</pozemek_zkratka></ZDROJ_ZE><PARCELY_ZE_A_PK><PARCELA_ZE><DIL_PARCELY_ZE attr="raw">part<child/></DIL_PARCELY_ZE><PARCELNI_CISLO_ZE/><PUV_NAZEV_KU><kod>46</kod><nazev>Old area</nazev></PUV_NAZEV_KU><TYP_1>type</TYP_1><VYMERA_PARCELY_ZE> +{huge} </VYMERA_PARCELY_ZE><PLOMBA_PARCELY_1>seal</PLOMBA_PARCELY_1></PARCELA_ZE></PARCELY_ZE_A_PK></ZDROJ_PARCEL_ZE></PARCELY_ZE>"#
        )
    }

    fn other_right(id: &str) -> String {
        let deed = deed("NAZEV_LIST");
        format!(
            r#"<JPV><HJPV_ID>{id}</HJPV_ID><TYP_PRAVNIHO_VZTAHU>Name-only relation</TYP_PRAVNIHO_VZTAHU><PORADI_K><datum>one</datum><text_o_prednostnim_poradi>priority one</text_o_prednostnim_poradi><datum>two</datum></PORADI_K><NEZOBRAZ_K_NEM>raw</NEZOBRAZ_K_NEM><NADRIZ_HJPV_ID/><JPV_POPISY><JPV_POPIS><JPV_RIZENI><JPV_RIZ/><JPV_RIZ><IDENT_RIZENI/></JPV_RIZ></JPV_RIZENI><POPIS_PRAVNIHO_VZTAHU>description</POPIS_PRAVNIHO_VZTAHU></JPV_POPIS></JPV_POPISY><JPV_OPRAV_PRO_OS><JPV_PRO_OS><PRO_IDENT_OS/><PODIL_POHLEDAVKA>raw share</PODIL_POHLEDAVKA></JPV_PRO_OS><JPV_PRO_OS><PODIL_POHLEDAVKA/></JPV_PRO_OS></JPV_OPRAV_PRO_OS><JPV_OPRAV_PRO_NEM><JPV_PRO_NEM/><JPV_PRO_NEM><PRO_IDENT_NEM><jpv_id>47</jpv_id><parcela/></PRO_IDENT_NEM></JPV_PRO_NEM></JPV_OPRAV_PRO_NEM><JPV_POVIN_K_OS><JPV_K_OS/><JPV_K_OS><K_IDENT_OS/></JPV_K_OS></JPV_POVIN_K_OS><JPV_POVIN_K_NEM><JPV_K_NEM/><JPV_K_NEM><K_IDENT_NEM/><K_IDENT_NEM><jpv_id>48</jpv_id><stavba/></K_IDENT_NEM></JPV_K_NEM></JPV_POVIN_K_NEM><JPV_LISTINY><JPV_LISTINA>{deed}{}<LISTIN_ID>{id}</LISTIN_ID><POPIS_LIST attr="raw">deed description<child/></POPIS_LIST></JPV_LISTINA></JPV_LISTINY></JPV>"#,
            proceeding()
        )
    }

    fn other_rights() -> String {
        let long_id = "9".repeat(80);
        format!(
            "<JINA_PRAVA><SEKCE_LIST SEKCE=\"B1\"><JPV_LIST>{}</JPV_LIST></SEKCE_LIST><SEKCE_LIST SEKCE=\"C\"><JPV_LIST>{}</JPV_LIST></SEKCE_LIST><SEKCE_LIST SEKCE=\"D\"><JPV_LIST>{}</JPV_LIST></SEKCE_LIST></JINA_PRAVA>",
            other_right(&long_id),
            other_right("49"),
            other_right("50")
        )
    }

    fn full_text() -> String {
        format!(
            "<TEXTY>{}{}{}{}{}{}{}{}{}<ZMENY_PV><ZMENA_PV><PLOMBA/></ZMENA_PV></ZMENY_PV>{}<UKONY_KN><UKON_KN><IDENT_RIZENI>raw proceeding</IDENT_RIZENI><RIZ_TYP_PREDMETU/><RIZ_TYP_PREDMETU><NAZEV_RIZENI>matter</NAZEV_RIZENI></RIZ_TYP_PREDMETU></UKON_KN></UKONY_KN>{}<UPOZORNENI1>warning one</UPOZORNENI1><UPOZORNENI2>warning two</UPOZORNENI2><CASTECNY>raw partial</CASTECNY></TEXTY>",
            header(),
            owner_relations(),
            parcel(),
            buildings(),
            building_rights(),
            acquisition_titles(),
            standalone_units(),
            bonities(),
            notices(),
            simplified_sources(),
            other_rights(),
        )
    }

    fn full_document() -> String {
        format!(
            "<VypisZKatastruNemovitosti><ds:Signature xmlns:ds=\"{DS_NS}\"><ds:SignedInfo/></ds:Signature><LIST_TEXT>{}</LIST_TEXT><LIST_ZAPATI_LV><ZAPATI_LV><KODKP>65535</KODKP><KP>Workplace</KP></ZAPATI_LV></LIST_ZAPATI_LV><NEM_BEZ_LV><NEMOVITOST><PARCELA_BEZ_LV/><PARCELA_BEZ_LV><parcela><id>51</id></parcela></PARCELA_BEZ_LV></NEMOVITOST></NEM_BEZ_LV><PLATNOST> valid </PLATNOST><BEZPL_PRISTUP>free</BEZPL_PRISTUP><VYHOTOVENO>created</VYHOTOVENO><INFO_VYSTUP>info</INFO_VYSTUP></VypisZKatastruNemovitosti>",
            full_text()
        )
    }

    fn minimal_document() -> String {
        format!(
            "<VypisZKatastruNemovitosti><LIST_TEXT><TEXTY>{}<VLASTNICI_JINI_OPRAVNENI><TYP_VZTAHU><TYP_VZTAHU>Raw relation</TYP_VZTAHU><SUBJEKTY><SUBJEKT><VLA_IDENT/><PODIL/></SUBJEKT></SUBJEKTY></TYP_VZTAHU></VLASTNICI_JINI_OPRAVNENI><POZEMKY/><STAVBY/><E_NABYVACI_TITULY/><JEDNOTKY/><F_BONITY/><D1_UPOZORNENI/><PARCELY_ZE/><UKONY_KN/><JINA_PRAVA><SEKCE_LIST SEKCE=\"B1\"><JPV_LIST/></SEKCE_LIST><SEKCE_LIST SEKCE=\"C\"><JPV_LIST/></SEKCE_LIST><SEKCE_LIST SEKCE=\"D\"><JPV_LIST/></SEKCE_LIST></JINA_PRAVA><UPOZORNENI1/><UPOZORNENI2/><CASTECNY/></TEXTY></LIST_TEXT><PLATNOST/><BEZPL_PRISTUP/><VYHOTOVENO/></VypisZKatastruNemovitosti>",
            header()
        )
    }

    #[test]
    fn complete_document_preserves_every_major_schema_branch() {
        let xml = full_document();
        let report = parse_legacy_ownership_sheet(xml.as_bytes(), NODES, DEPTH).unwrap();
        assert_eq!(report.valid_at, " valid ");
        assert_eq!(
            report.signature.unwrap().raw_xml,
            format!("<ds:Signature xmlns:ds=\"{DS_NS}\"><ds:SignedInfo/></ds:Signature>")
        );
        assert_eq!(
            report.footer.unwrap().workplace.unwrap().workplace_code,
            u16::MAX
        );
        assert_eq!(
            report
                .properties_without_sheet
                .unwrap()
                .group
                .unwrap()
                .parcels
                .len(),
            2
        );
        let text = &report.texts[0];
        assert_eq!(text.header.municipality.code, " 000raw ");
        assert_eq!(text.header.id.as_str(), "");
        assert_eq!(text.owner_relations[0].relation.code, None);
        assert_eq!(text.owner_relations[0].relation.is_ownership(), None);
        assert_eq!(
            text.owner_relations[0].subjects[0]
                .identification
                .targets
                .len(),
            3
        );
        assert_eq!(
            text.owner_relations[0].subjects[0].share.numerator,
            Some(Decimal::new(-1, 0))
        );
        assert_eq!(
            text.owner_relations[0].subjects[0].share.denominator,
            Some(Decimal::ZERO)
        );
        assert_eq!(text.parcels[0].identifications.len(), 2);
        assert_eq!(
            text.parcels[0]
                .buildings_on_multiple_parcels
                .as_ref()
                .unwrap()
                .len(),
            2
        );
        assert_eq!(text.parcels[0].component_buildings.buildings.len(), 2);
        assert_eq!(text.parcels[0].additional_details.len(), 3);
        let units = text.parcels[0].units.as_ref().unwrap();
        assert!(units[0].details.is_none());
        let unit = units[1].details.as_ref().unwrap();
        assert_eq!(unit.unit_type.code, "raw-type");
        assert!(unit.identification.unit.is_none());
        assert!(unit.on_sheet.details.is_none());
        assert_eq!(unit.ownerships[0].identification.targets.len(), 2);
        assert_eq!(
            text.buildings[0].seal.raw_xml,
            "<PLOMBA_BUDOVY attr=\"raw\">seal<child/></PLOMBA_BUDOVY>"
        );
        assert_eq!(
            text.buildings[0].units.as_ref().unwrap()[0].on_sheet,
            u32::MAX
        );
        assert_eq!(
            text.buildings[0].units.as_ref().unwrap()[0].ownerships[0]
                .alternate_identifications
                .len(),
            2
        );
        assert!(text.building_rights.as_ref().unwrap()[0].details.is_none());
        let right = text.building_rights.as_ref().unwrap()[1]
            .details
            .as_ref()
            .unwrap();
        assert_eq!(right.purposes, ["one", "two"]);
        assert!(right.units.as_ref().unwrap()[0].details.is_none());
        assert_eq!(
            right.units.as_ref().unwrap()[1]
                .details
                .as_ref()
                .unwrap()
                .on_sheet
                .number,
            "30"
        );
        let acquisition = &text.acquisition_titles[0];
        assert_eq!(
            acquisition
                .deed_name
                .details
                .as_ref()
                .unwrap()
                .handling_workplace_name
                .as_deref(),
            Some("workplace")
        );
        assert_eq!(acquisition.beneficiary_groups.len(), 2);
        let standalone = &text.units[0];
        assert_eq!(
            standalone.use_description.as_ref().unwrap().code.as_deref(),
            Some("raw-use")
        );
        assert_eq!(
            standalone.building_share.numerator,
            Some(Decimal::new(-5, 0))
        );
        assert!(standalone.buildings[0].details.is_none());
        let building = standalone.buildings[1].details.as_ref().unwrap();
        assert_eq!(building.subjects.len(), 2);
        assert!(building.subjects[0].building.is_none());
        assert_eq!(building.parcels.len(), 2);
        assert!(standalone.building_right_groups[0].right.entries.is_empty());
        assert_eq!(standalone.building_right_groups[1].right.entries.len(), 2);
        assert_eq!(standalone.building_right_groups[1].parcels.len(), 2);
        assert_eq!(standalone.parcels[0].component_buildings.buildings.len(), 1);
        assert!(text.bonities[0].parcel.details.is_none());
        assert!(text.bonities[0].other_cadastral_area.details.is_none());
        assert_eq!(text.bonities[1].entries[0].area, 43);
        assert_eq!(text.notices.len(), 2);
        assert!(text.notices[0].proceedings[0].details.is_none());
        let notice = text.notices[0].proceedings[1].details.as_ref().unwrap();
        assert_eq!(
            notice.properties[1].details.as_ref().unwrap().parcels.len(),
            2
        );
        assert_eq!(text.legal_relation_changes.as_ref().unwrap().len(), 1);
        let source = text.simplified_parcel_sources[1].details.as_ref().unwrap();
        assert!(text.simplified_parcel_sources[0].details.is_none());
        assert_eq!(source.parcels[0].area, format!(" +{} ", "9".repeat(120)));
        assert_eq!(
            source.parcels[0].part.raw_xml,
            "<DIL_PARCELY_ZE attr=\"raw\">part<child/></DIL_PARCELY_ZE>"
        );
        assert_eq!(
            text.operations[0].subject_matters.as_ref().unwrap().len(),
            2
        );
        assert_eq!(text.other_right_sections.len(), 3);
        let relation = &text.other_right_sections[0].relations[0];
        assert_eq!(relation.id.as_str().len(), 80);
        assert_eq!(relation.relation.code, None);
        assert_eq!(relation.relation.is_ownership(), None);
        assert_eq!(relation.priority.as_ref().unwrap().len(), 2);
        assert_eq!(
            relation.descriptions[0].proceedings.as_ref().unwrap().len(),
            2
        );
        assert_eq!(relation.benefiting_subjects.len(), 2);
        assert_eq!(relation.benefiting_properties.as_ref().unwrap().len(), 2);
        assert_eq!(relation.obligated_subjects.len(), 2);
        assert_eq!(
            relation.obligated_properties.as_ref().unwrap()[1]
                .identifications
                .len(),
            2
        );
        assert_eq!(
            relation.deeds[0].description.raw_xml,
            "<POPIS_LIST attr=\"raw\">deed description<child/></POPIS_LIST>"
        );
    }

    #[test]
    fn minimal_document_retains_absent_and_present_empty_forms() {
        let report =
            parse_legacy_ownership_sheet(minimal_document().as_bytes(), NODES, DEPTH).unwrap();
        let text = &report.texts[0];
        assert!(text.building_rights.is_none());
        assert!(text.legal_relation_changes.is_none());
        assert!(text.parcels.is_empty());
        assert!(text.units.is_empty());
        assert_eq!(text.other_right_sections.len(), 3);
        assert!(
            text.other_right_sections
                .iter()
                .all(|section| section.relations.is_empty())
        );
        let owner = &text.owner_relations[0].subjects[0];
        assert!(owner.identification.targets.is_empty());
        assert!(owner.subject_kind.is_none());
        assert_eq!(owner.share, ReportedShare::default());
        assert!(report.footer.is_none());
        assert!(report.signature.is_none());

        let minimal = minimal_document();
        let start = minimal.find("<LIST_TEXT>").unwrap();
        let end = minimal.find("</LIST_TEXT>").unwrap() + "</LIST_TEXT>".len();
        let empty_lists = format!("{}<LIST_TEXT/>{}", &minimal[..start], &minimal[end..]);
        let report = parse_legacy_ownership_sheet(empty_lists.as_bytes(), NODES, DEPTH).unwrap();
        assert!(report.texts.is_empty());
    }

    #[test]
    fn rejects_wrong_root_dtd_limits_duplicates_and_invalid_sequences() {
        let xml = full_document();
        assert!(parse_legacy_ownership_sheet(b"<Other/>", NODES, DEPTH).is_err());
        assert!(
            parse_legacy_ownership_sheet(b"<!DOCTYPE x><VypisZKatastruNemovitosti/>", NODES, DEPTH)
                .is_err()
        );
        assert!(parse_legacy_ownership_sheet(xml.as_bytes(), 1, DEPTH).is_err());
        assert!(parse_legacy_ownership_sheet(xml.as_bytes(), NODES, 1).is_err());
        let duplicate = xml.replacen("<PLATNOST> valid </PLATNOST>", "<PLATNOST/><PLATNOST/>", 1);
        assert!(parse_legacy_ownership_sheet(duplicate.as_bytes(), NODES, DEPTH).is_err());
        let duplicate_parcel_field = xml.replacen(
            "<VYMERA_PARCELY> lexical area </VYMERA_PARCELY>",
            "<VYMERA_PARCELY/><VYMERA_PARCELY/>",
            1,
        );
        assert!(
            parse_legacy_ownership_sheet(duplicate_parcel_field.as_bytes(), NODES, DEPTH).is_err()
        );
        let too_few_sections =
            minimal_document().replacen("<SEKCE_LIST SEKCE=\"D\"><JPV_LIST/></SEKCE_LIST>", "", 1);
        assert!(parse_legacy_ownership_sheet(too_few_sections.as_bytes(), NODES, DEPTH).is_err());
        let duplicate_section = minimal_document().replacen("SEKCE=\"D\"", "SEKCE=\"C\"", 1);
        assert!(parse_legacy_ownership_sheet(duplicate_section.as_bytes(), NODES, DEPTH).is_err());
        let invalid_integer = xml
            .replace("> +", ">+")
            .replace(" </VYMERA_PARCELY_ZE>", ".0</VYMERA_PARCELY_ZE>");
        assert!(parse_legacy_ownership_sheet(invalid_integer.as_bytes(), NODES, DEPTH).is_err());
    }
}
