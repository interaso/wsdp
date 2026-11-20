//! Bounded decoding of legacy ownership-overview and person-rights XML reports.

use std::str::FromStr;

use quick_xml::events::{BytesStart, Event};
use roxmltree::{Document, Node, ParsingOptions};

use crate::LegalRelation;
use crate::error::{Error, Result};
use crate::model::reports::legacy_base::{
    Address, BuildingRight, CharacteristicCode, CharacteristicWrapper, LegacyId, Parcel,
    Proceeding, SubjectNameForm,
};
use crate::model::reports::legacy_ownership_overview::*;

use super::legacy_base::{
    parse_address, parse_building, parse_building_right, parse_building_wrapper,
    parse_characteristic_wrapper, parse_parcel_wrapper, parse_proceeding,
    parse_property_identification, parse_reported_share, parse_subject_name, parse_unit_wrapper,
    parse_yes_no,
};

const XSI_NS: &str = "http://www.w3.org/2001/XMLSchema-instance";

/// Parses a complete `PrehledVlastnictvi` report with explicit XML limits.
pub(crate) fn parse_ownership_overview(
    bytes: &[u8],
    max_nodes: u32,
    max_depth: u32,
) -> Result<OwnershipOverviewReport> {
    parse_document(
        bytes,
        "PrehledVlastnictvi",
        max_nodes,
        max_depth,
        decode_ownership_overview,
    )
}

/// Parses a complete `PrehledVlastnictviSNemovitostmi` report with explicit XML limits.
pub(crate) fn parse_ownership_overview_with_properties(
    bytes: &[u8],
    max_nodes: u32,
    max_depth: u32,
) -> Result<OwnershipOverviewWithPropertiesReport> {
    parse_document(
        bytes,
        "PrehledVlastnictviSNemovitostmi",
        max_nodes,
        max_depth,
        decode_ownership_overview_with_properties,
    )
}

/// Parses a complete `EvidencePravProOsobu` report with explicit XML limits.
pub(crate) fn parse_rights_record(
    bytes: &[u8],
    max_nodes: u32,
    max_depth: u32,
) -> Result<RightsRecordReport> {
    parse_document(
        bytes,
        "EvidencePravProOsobu",
        max_nodes,
        max_depth,
        decode_rights_record,
    )
}

fn parse_document<T>(
    bytes: &[u8],
    root_name: &'static str,
    max_nodes: u32,
    max_depth: u32,
    decode: impl FnOnce(Node<'_, '_>) -> Result<T>,
) -> Result<T> {
    let xml = std::str::from_utf8(bytes)
        .map_err(|_| Error::invalid_response("legacy ownership report is not UTF-8 XML"))?;
    preflight(xml, max_nodes, max_depth)?;
    let document = Document::parse_with_options(
        xml,
        ParsingOptions {
            allow_dtd: false,
            nodes_limit: max_nodes,
            ..ParsingOptions::default()
        },
    )
    .map_err(|_| Error::invalid_response("malformed legacy ownership-report XML"))?;
    let root = document.root_element();
    require_element(root)?;
    if root.tag_name().name() != root_name {
        return Err(Error::invalid_response(
            "unexpected legacy ownership-report root",
        ));
    }
    decode(root)
}

fn decode_ownership_overview(node: Node<'_, '_>) -> Result<OwnershipOverviewReport> {
    const FIELDS: &[&str] = &[
        "PRAV_VZTAHY",
        "OPRAV_SUBJEKT",
        "OKRESY",
        "CP_OMEZENI",
        "CP_PRACOVISTE",
        "PLATNOST",
        "VYHOTOVIL",
        "BEZPL_PRISTUP",
        "VYHOTOVENO",
        "INFO_VYSTUP",
    ];

    let mut cursor = Cursor::all(node, FIELDS)?;
    let mut requested_legal_relation = None;
    let mut subject = None;
    let mut districts = None;
    let mut territorial_restriction = None;
    let mut workplace = None;
    let mut valid_at = None;
    let mut created_by = None;
    let mut free_access = None;
    let mut created_at = None;
    let mut output_info = None;
    for child in element_children(node) {
        match cursor.singleton(child)? {
            0 => requested_legal_relation = Some(parse_requested_legal_relation(child)?),
            1 => subject = Some(parse_overview_subject(child)?),
            2 => districts = Some(parse_overview_districts(child)?),
            3 => territorial_restriction = Some(simple_text(child)?),
            4 => workplace = Some(simple_text(child)?),
            5 => valid_at = Some(simple_text(child)?),
            6 => created_by = Some(simple_text(child)?),
            7 => free_access = Some(simple_text(child)?),
            8 => created_at = Some(simple_text(child)?),
            9 => output_info = Some(simple_text(child)?),
            _ => unreachable!("cursor returned an in-range index"),
        }
    }
    Ok(OwnershipOverviewReport {
        requested_legal_relation: required(requested_legal_relation, "PRAV_VZTAHY")?,
        subject: required(subject, "OPRAV_SUBJEKT")?,
        districts: required(districts, "OKRESY")?,
        territorial_restriction: required(territorial_restriction, "CP_OMEZENI")?,
        workplace: required(workplace, "CP_PRACOVISTE")?,
        valid_at: required(valid_at, "PLATNOST")?,
        created_by: required(created_by, "VYHOTOVIL")?,
        free_access: required(free_access, "BEZPL_PRISTUP")?,
        created_at: required(created_at, "VYHOTOVENO")?,
        output_info,
    })
}

fn decode_ownership_overview_with_properties(
    node: Node<'_, '_>,
) -> Result<OwnershipOverviewWithPropertiesReport> {
    const FIELDS: &[&str] = &[
        "PRAV_VZTAHY",
        "OPRAV_SUBJEKT",
        "OKRESY",
        "INFO_VYSTUP",
        "RIZENI",
        "VYHOTOVIL",
        "VYHOTOVENO",
        "PLATNOST",
        "PRACOVISTE",
        "BEZPL_PRISTUP",
    ];

    let mut cursor = Cursor::all(node, FIELDS)?;
    let mut requested_legal_relation = None;
    let mut subject = None;
    let mut districts = None;
    let mut output_info = None;
    let mut proceeding = None;
    let mut created_by = None;
    let mut created_at = None;
    let mut valid_at = None;
    let mut workplace = None;
    let mut free_access = None;
    for child in element_children(node) {
        match cursor.singleton(child)? {
            0 => requested_legal_relation = Some(parse_requested_legal_relation(child)?),
            1 => subject = Some(parse_properties_subject(child)?),
            2 => districts = Some(parse_properties_districts(child)?),
            3 => output_info = Some(simple_text(child)?),
            4 => proceeding = Some(parse_proceeding(child)?),
            5 => created_by = Some(simple_text(child)?),
            6 => created_at = Some(simple_text(child)?),
            7 => valid_at = Some(simple_text(child)?),
            8 => workplace = Some(simple_text(child)?),
            9 => free_access = Some(simple_text(child)?),
            _ => unreachable!("cursor returned an in-range index"),
        }
    }
    Ok(OwnershipOverviewWithPropertiesReport {
        requested_legal_relation: required(requested_legal_relation, "PRAV_VZTAHY")?,
        subject: required(subject, "OPRAV_SUBJEKT")?,
        districts: required(districts, "OKRESY")?,
        output_info: required(output_info, "INFO_VYSTUP")?,
        proceeding: required(proceeding, "RIZENI")?,
        created_by: required(created_by, "VYHOTOVIL")?,
        created_at: required(created_at, "VYHOTOVENO")?,
        valid_at: required(valid_at, "PLATNOST")?,
        workplace: required(workplace, "PRACOVISTE")?,
        free_access: required(free_access, "BEZPL_PRISTUP")?,
    })
}

fn decode_rights_record(node: Node<'_, '_>) -> Result<RightsRecordReport> {
    const FIELDS: &[&str] = &[
        "ZADANE_UDAJE",
        "UPLNA_SHODA",
        "NESHODA",
        "CASTECNA_SHODA",
        "PLATNOST",
        "VYHOTOVIL",
        "BEZPL_PRISTUP",
        "VYHOTOVENO",
    ];

    let mut cursor = Cursor::all(node, FIELDS)?;
    let mut query = None;
    let mut exact_match = None;
    let mut mismatch = None;
    let mut partial_match = None;
    let mut valid_at = None;
    let mut created_by = None;
    let mut free_access = None;
    let mut created_at = None;
    for child in element_children(node) {
        match cursor.singleton(child)? {
            0 => query = Some(parse_rights_query(child)?),
            1 => exact_match = Some(parse_exact_match(child)?),
            2 => mismatch = Some(parse_mismatch(child)?),
            3 => partial_match = Some(parse_partial_match(child)?),
            4 => valid_at = Some(simple_text(child)?),
            5 => created_by = Some(simple_text(child)?),
            6 => free_access = Some(simple_text(child)?),
            7 => created_at = Some(simple_text(child)?),
            _ => unreachable!("cursor returned an in-range index"),
        }
    }
    Ok(RightsRecordReport {
        query: required(query, "ZADANE_UDAJE")?,
        exact_match,
        mismatch,
        partial_match,
        valid_at: required(valid_at, "PLATNOST")?,
        created_by: required(created_by, "VYHOTOVIL")?,
        free_access: required(free_access, "BEZPL_PRISTUP")?,
        created_at: required(created_at, "VYHOTOVENO")?,
    })
}

fn parse_requested_legal_relation(node: Node<'_, '_>) -> Result<LegalRelation> {
    let mut cursor = Cursor::sequence(node, &["TYPPRA"])?;
    let mut relation = None;
    for child in element_children(node) {
        cursor.singleton(child)?;
        relation = Some(parse_requested_relation_type(child)?);
    }
    required(relation, "PRAV_VZTAHY/TYPPRA")
}

fn parse_requested_relation_type(node: Node<'_, '_>) -> Result<LegalRelation> {
    let mut cursor = Cursor::sequence(node, &["TYPRAV_NAZEV"])?;
    let mut name = None;
    for child in element_children(node) {
        cursor.singleton(child)?;
        name = Some(simple_text(child)?);
    }
    Ok(LegalRelation {
        code: None,
        name: required(name, "TYPPRA/TYPRAV_NAZEV")?,
    })
}

fn parse_overview_subject(node: Node<'_, '_>) -> Result<OwnershipOverviewSubject> {
    const FIELDS: &[&str] = &["CHAROS", "OPSUB_NAZEV", "OPSUB_IDENT", "OPSUB_ADRESA"];
    let mut cursor = Cursor::sequence(node, FIELDS)?;
    let mut characteristic = None;
    let mut name = None;
    let mut identification = None;
    let mut addresses = None;
    for child in element_children(node) {
        match cursor.singleton(child)? {
            0 => characteristic = Some(parse_characteristic_wrapper(child)?),
            1 => name = Some(parse_overview_subject_name(child)?),
            2 => identification = Some(parse_legacy_identification(child, false)?),
            3 => addresses = Some(parse_addresses(child)?),
            _ => unreachable!("cursor returned an in-range index"),
        }
    }
    Ok(OwnershipOverviewSubject {
        characteristic: required(characteristic, "OPRAV_SUBJEKT/CHAROS")?,
        name: required(name, "OPRAV_SUBJEKT/OPSUB_NAZEV")?,
        identification: required(identification, "OPRAV_SUBJEKT/OPSUB_IDENT")?,
        addresses: required(addresses, "OPRAV_SUBJEKT/OPSUB_ADRESA")?,
    })
}

fn parse_properties_subject(node: Node<'_, '_>) -> Result<OwnershipPropertiesSubject> {
    const FIELDS: &[&str] = &["CHAROS", "OPSUB_NAZEV", "OPSUB_IDENT", "OPSUB_ADRESA"];
    let mut cursor = Cursor::sequence(node, FIELDS)?;
    let mut characteristic = None;
    let mut name = None;
    let mut identification = None;
    let mut addresses = None;
    for child in element_children(node) {
        match cursor.singleton(child)? {
            0 => characteristic = Some(parse_characteristic_code(child)?),
            1 => name = Some(parse_overview_subject_name(child)?),
            2 => identification = Some(parse_legacy_identification(child, false)?),
            3 => addresses = Some(parse_addresses(child)?),
            _ => unreachable!("cursor returned an in-range index"),
        }
    }
    Ok(OwnershipPropertiesSubject {
        characteristic: required(characteristic, "OPRAV_SUBJEKT/CHAROS")?,
        name: required(name, "OPRAV_SUBJEKT/OPSUB_NAZEV")?,
        identification: required(identification, "OPRAV_SUBJEKT/OPSUB_IDENT")?,
        addresses: required(addresses, "OPRAV_SUBJEKT/OPSUB_ADRESA")?,
    })
}

fn parse_overview_subject_name(node: Node<'_, '_>) -> Result<SubjectNameForm> {
    const FIELDS: &[&str] = &[
        "id",
        "bsm1_id",
        "bsm2_id",
        "prijmeni",
        "jmeno",
        "titul_pred",
        "titul_za",
        "bsm1_prijmeni",
        "bsm1_jmeno",
        "bsm1_pred",
        "bsm1_za",
        "bsm2_prijmeni",
        "bsm2_jmeno",
        "bsm2_pred",
        "bsm2_za",
        "nazev",
    ];
    let mut cursor = Cursor::sequence(node, FIELDS)?;
    let mut value = SubjectNameForm::default();
    for child in element_children(node) {
        match cursor.singleton(child)? {
            0 => value.id = Some(parse_legacy_id(child)?),
            1 => value.partner1_id = Some(parse_legacy_id(child)?),
            2 => value.partner2_id = Some(parse_legacy_id(child)?),
            3 => value.person.surname = Some(simple_text(child)?),
            4 => value.person.given_name = Some(simple_text(child)?),
            5 => value.person.title_before = Some(simple_text(child)?),
            6 => value.person.title_after = Some(simple_text(child)?),
            7 => value.partner1.surname = Some(simple_text(child)?),
            8 => value.partner1.given_name = Some(simple_text(child)?),
            9 => value.partner1.title_before = Some(simple_text(child)?),
            10 => value.partner1.title_after = Some(simple_text(child)?),
            11 => value.partner2.surname = Some(simple_text(child)?),
            12 => value.partner2.given_name = Some(simple_text(child)?),
            13 => value.partner2.title_before = Some(simple_text(child)?),
            14 => value.partner2.title_after = Some(simple_text(child)?),
            15 => value.name = Some(simple_text(child)?),
            _ => unreachable!("cursor returned an in-range index"),
        }
    }
    Ok(value)
}

fn parse_legacy_identification(
    node: Node<'_, '_>,
    require_nonempty: bool,
) -> Result<LegacySubjectIdentification> {
    require_element(node)?;
    let entries = element_children(node)
        .map(parse_legacy_identifier)
        .collect::<Result<Vec<_>>>()?;
    ensure_nonempty(&entries, node.tag_name().name(), require_nonempty)?;
    Ok(LegacySubjectIdentification { entries })
}

fn parse_legacy_identifier(node: Node<'_, '_>) -> Result<LegacySubjectIdentifier> {
    require_element(node)?;
    match node.tag_name().name() {
        "id" => parse_legacy_id(node).map(LegacySubjectIdentifier::Id),
        "rc6" => simple_text(node).map(LegacySubjectIdentifier::PersonalNumber6),
        "rc7" => simple_text(node).map(LegacySubjectIdentifier::PersonalNumber7),
        "ico" => simple_text(node).map(LegacySubjectIdentifier::CompanyId),
        "ico_doplnek" => simple_text(node).map(LegacySubjectIdentifier::CompanyIdSuffix),
        "bsm1_id" => parse_legacy_id(node).map(LegacySubjectIdentifier::Partner1Id),
        "bsm2_id" => parse_legacy_id(node).map(LegacySubjectIdentifier::Partner2Id),
        name => Err(unexpected_field(name)),
    }
}

fn parse_addresses(node: Node<'_, '_>) -> Result<Vec<Address>> {
    parse_list(node, "adresa", false, parse_address)
}

fn parse_overview_districts(node: Node<'_, '_>) -> Result<Vec<OwnershipOverviewDistrictGroup>> {
    require_element(node)?;
    let mut groups = Vec::new();
    let mut pending_district = None;
    for child in element_children(node) {
        require_element(child)?;
        match child.tag_name().name() {
            "OKRES" => {
                if pending_district.is_some() {
                    return Err(invalid_value("OKRESY/OBCE", "required element is missing"));
                }
                pending_district = Some(parse_district(child)?);
            }
            "OBCE" => {
                let district = required(pending_district.take(), "OKRESY/OKRES")?;
                groups.push(OwnershipOverviewDistrictGroup {
                    district,
                    municipalities: parse_overview_municipalities(child)?,
                });
            }
            name => return Err(unexpected_field(name)),
        }
    }
    if pending_district.is_some() {
        return Err(invalid_value("OKRESY/OBCE", "required element is missing"));
    }
    Ok(groups)
}

fn parse_overview_municipalities(
    node: Node<'_, '_>,
) -> Result<Vec<OwnershipOverviewMunicipalityGroup>> {
    require_element(node)?;
    let mut groups = Vec::new();
    let mut pending_municipality = None;
    for child in element_children(node) {
        require_element(child)?;
        match child.tag_name().name() {
            "OBEC" => {
                if pending_municipality.is_some() {
                    return Err(invalid_value(
                        "OBCE/KATASTRALNI_UZEMI",
                        "required element is missing",
                    ));
                }
                pending_municipality = Some(parse_code_name(child)?);
            }
            "KATASTRALNI_UZEMI" => {
                let municipality = required(pending_municipality.take(), "OBCE/OBEC")?;
                groups.push(OwnershipOverviewMunicipalityGroup {
                    municipality,
                    cadastral_areas: parse_overview_cadastral_areas(child)?,
                });
            }
            name => return Err(unexpected_field(name)),
        }
    }
    if pending_municipality.is_some() {
        return Err(invalid_value(
            "OBCE/KATASTRALNI_UZEMI",
            "required element is missing",
        ));
    }
    Ok(groups)
}

fn parse_overview_cadastral_areas(
    node: Node<'_, '_>,
) -> Result<Vec<OwnershipOverviewCadastralAreaGroup>> {
    require_element(node)?;
    let mut groups = Vec::new();
    let mut pending_area = None;
    for child in element_children(node) {
        require_element(child)?;
        match child.tag_name().name() {
            "KATUZE" => {
                if pending_area.is_some() {
                    return Err(invalid_value(
                        "KATASTRALNI_UZEMI/TYPY_PRAV_VZTAHU",
                        "required element is missing",
                    ));
                }
                pending_area = Some(parse_code_name(child)?);
            }
            "TYPY_PRAV_VZTAHU" => {
                let cadastral_area = required(pending_area.take(), "KATASTRALNI_UZEMI/KATUZE")?;
                groups.push(OwnershipOverviewCadastralAreaGroup {
                    cadastral_area,
                    legal_relations: parse_list(
                        child,
                        "TYP_PRAV_VZTAHU",
                        false,
                        parse_overview_legal_relation,
                    )?,
                });
            }
            name => return Err(unexpected_field(name)),
        }
    }
    if pending_area.is_some() {
        return Err(invalid_value(
            "KATASTRALNI_UZEMI/TYPY_PRAV_VZTAHU",
            "required element is missing",
        ));
    }
    ensure_nonempty(&groups, "KATASTRALNI_UZEMI", true)?;
    Ok(groups)
}

fn parse_overview_legal_relation(
    node: Node<'_, '_>,
) -> Result<OwnershipOverviewLegalRelationGroup> {
    let mut cursor = Cursor::sequence(node, &["TYPRAV_NAZEV", "TELESA"])?;
    let mut legal_relation = None;
    let mut records = None;
    for child in element_children(node) {
        match cursor.singleton(child)? {
            0 => {
                legal_relation = Some(LegalRelation {
                    code: None,
                    name: simple_text(child)?,
                });
            }
            1 => {
                records = Some(parse_list(child, "TELESO", true, parse_overview_record)?);
            }
            _ => unreachable!("cursor returned an in-range index"),
        }
    }
    Ok(OwnershipOverviewLegalRelationGroup {
        legal_relation: required(legal_relation, "TYP_PRAV_VZTAHU/TYPRAV_NAZEV")?,
        records: required(records, "TYP_PRAV_VZTAHU/TELESA")?,
    })
}

fn parse_overview_record(node: Node<'_, '_>) -> Result<OwnershipOverviewRecord> {
    const FIELDS: &[&str] = &["LV", "PRSV_IDENT", "PODIL", "TEL_ID", "JPV_LV", "RIZENI"];
    let mut cursor = Cursor::sequence(node, FIELDS)?;
    let mut ownership_sheet_number = None;
    let mut properties = None;
    let mut share = None;
    let mut ownership_sheet_id = None;
    let mut legal_relation_sheet = None;
    let mut proceeding = None;
    for child in element_children(node) {
        match cursor.singleton(child)? {
            0 => ownership_sheet_number = Some(simple_text(child)?),
            1 => properties = Some(parse_property_identification(child)?),
            2 => share = Some(parse_reported_share(child)?),
            3 => ownership_sheet_id = Some(parse_legacy_id(child)?),
            4 => legal_relation_sheet = Some(simple_text(child)?),
            5 => proceeding = Some(parse_proceeding(child)?),
            _ => unreachable!("cursor returned an in-range index"),
        }
    }
    Ok(OwnershipOverviewRecord {
        ownership_sheet_number: required(ownership_sheet_number, "TELESO/LV")?,
        properties: required(properties, "TELESO/PRSV_IDENT")?,
        share: required(share, "TELESO/PODIL")?,
        ownership_sheet_id: required(ownership_sheet_id, "TELESO/TEL_ID")?,
        legal_relation_sheet: required(legal_relation_sheet, "TELESO/JPV_LV")?,
        proceeding: required(proceeding, "TELESO/RIZENI")?,
    })
}

fn parse_properties_districts(node: Node<'_, '_>) -> Result<Vec<OwnershipPropertiesDistrictGroup>> {
    require_element(node)?;
    let mut groups = Vec::new();
    let mut pending_district = None;
    for child in element_children(node) {
        require_element(child)?;
        match child.tag_name().name() {
            "OKRES" => {
                if pending_district.is_some() {
                    return Err(invalid_value("OKRESY/OBCE", "required element is missing"));
                }
                pending_district = Some(parse_district(child)?);
            }
            "OBCE" => {
                let district = required(pending_district.take(), "OKRESY/OKRES")?;
                groups.push(OwnershipPropertiesDistrictGroup {
                    district,
                    municipalities: parse_properties_municipalities(child)?,
                });
            }
            name => return Err(unexpected_field(name)),
        }
    }
    if pending_district.is_some() {
        return Err(invalid_value("OKRESY/OBCE", "required element is missing"));
    }
    Ok(groups)
}

fn parse_properties_municipalities(
    node: Node<'_, '_>,
) -> Result<Vec<OwnershipPropertiesMunicipalityGroup>> {
    require_element(node)?;
    let mut groups = Vec::new();
    let mut pending_municipality = None;
    for child in element_children(node) {
        require_element(child)?;
        match child.tag_name().name() {
            "OBEC" => {
                if pending_municipality.is_some() {
                    return Err(invalid_value(
                        "OBCE/KATASTRALNI_UZEMI",
                        "required element is missing",
                    ));
                }
                pending_municipality = Some(parse_code_name(child)?);
            }
            "KATASTRALNI_UZEMI" => {
                let municipality = required(pending_municipality.take(), "OBCE/OBEC")?;
                groups.push(OwnershipPropertiesMunicipalityGroup {
                    municipality,
                    cadastral_areas: parse_properties_cadastral_areas(child)?,
                });
            }
            name => return Err(unexpected_field(name)),
        }
    }
    if pending_municipality.is_some() {
        return Err(invalid_value(
            "OBCE/KATASTRALNI_UZEMI",
            "required element is missing",
        ));
    }
    Ok(groups)
}

fn parse_properties_cadastral_areas(
    node: Node<'_, '_>,
) -> Result<Vec<OwnershipPropertiesCadastralAreaGroup>> {
    require_element(node)?;
    let mut groups = Vec::new();
    let mut pending_area = None;
    for child in element_children(node) {
        require_element(child)?;
        match child.tag_name().name() {
            "KATUZE" => {
                if pending_area.is_some() {
                    return Err(invalid_value(
                        "KATASTRALNI_UZEMI/TELESA",
                        "required element is missing",
                    ));
                }
                pending_area = Some(parse_code_name(child)?);
            }
            "TELESA" => {
                let cadastral_area = required(pending_area.take(), "KATASTRALNI_UZEMI/KATUZE")?;
                groups.push(OwnershipPropertiesCadastralAreaGroup {
                    cadastral_area,
                    records: parse_list(child, "TELESO", true, parse_properties_record)?,
                });
            }
            name => return Err(unexpected_field(name)),
        }
    }
    if pending_area.is_some() {
        return Err(invalid_value(
            "KATASTRALNI_UZEMI/TELESA",
            "required element is missing",
        ));
    }
    ensure_nonempty(&groups, "KATASTRALNI_UZEMI", true)?;
    Ok(groups)
}

fn parse_properties_record(node: Node<'_, '_>) -> Result<OwnershipPropertiesRecord> {
    const FIELDS: &[&str] = &[
        "PODIL",
        "LV",
        "PARCELY_KN",
        "STAVBY",
        "JEDNOTKY",
        "ZDROJE",
        "PRAVA_STAVBY",
    ];
    let mut cursor = Cursor::sequence(node, FIELDS)?;
    let mut share = None;
    let mut ownership_sheet_number = None;
    let mut parcels = None;
    let mut buildings = None;
    let mut units = None;
    let mut sources = None;
    let mut building_rights = None;
    for child in element_children(node) {
        match cursor.singleton(child)? {
            0 => share = Some(parse_reported_share(child)?),
            1 => ownership_sheet_number = Some(simple_text(child)?),
            2 => {
                parcels = Some(parse_list(
                    child,
                    "PARCELA_KN",
                    false,
                    parse_ownership_parcel,
                )?);
            }
            3 => {
                buildings = Some(parse_list(
                    child,
                    "STAVBA",
                    false,
                    parse_ownership_building,
                )?);
            }
            4 => {
                units = Some(parse_list(child, "JEDNOTKA", false, parse_ownership_unit)?);
            }
            5 => {
                sources = Some(parse_list(child, "ZDROJ", false, parse_ownership_source)?);
            }
            6 => {
                building_rights = Some(parse_list(
                    child,
                    "PRAVO_STAVBY",
                    false,
                    parse_ownership_building_right,
                )?);
            }
            _ => unreachable!("cursor returned an in-range index"),
        }
    }
    Ok(OwnershipPropertiesRecord {
        share: required(share, "TELESO/PODIL")?,
        ownership_sheet_number: required(ownership_sheet_number, "TELESO/LV")?,
        parcels: required(parcels, "TELESO/PARCELY_KN")?,
        buildings: required(buildings, "TELESO/STAVBY")?,
        units: required(units, "TELESO/JEDNOTKY")?,
        sources: required(sources, "TELESO/ZDROJE")?,
        building_rights: required(building_rights, "TELESO/PRAVA_STAVBY")?,
    })
}

fn parse_ownership_parcel(node: Node<'_, '_>) -> Result<OwnershipParcel> {
    const FIELDS: &[&str] = &[
        "ZP_VYUZITI",
        "PARCELNI_CISLO",
        "DIL",
        "VYMERA_PARCELY",
        "URC_VYMERY",
        "DRUH_POZEMKU",
        "OCHRANA",
        "MAPA",
        "STAVBA_NA_PARCELE",
    ];
    let mut cursor = Cursor::all(node, FIELDS)?;
    let mut use_description = None;
    let mut parcel = None;
    let mut part = None;
    let mut area = None;
    let mut area_determination = None;
    let mut land_kind = None;
    let mut protection = None;
    let mut map = None;
    let mut building_on_parcel = None;
    for child in element_children(node) {
        match cursor.singleton(child)? {
            0 => use_description = Some(simple_text(child)?),
            1 => parcel = Some(parse_parcel_wrapper(child)?),
            2 => part = Some(simple_text(child)?),
            3 => area = Some(parse_integer(child, "VYMERA_PARCELY")?),
            4 => area_determination = Some(simple_text(child)?),
            5 => land_kind = Some(simple_text(child)?),
            6 => protection = Some(parse_yes_no(child)?),
            7 => map = Some(simple_text(child)?),
            8 => building_on_parcel = Some(parse_building_wrapper(child)?),
            _ => unreachable!("cursor returned an in-range index"),
        }
    }
    Ok(OwnershipParcel {
        use_description,
        parcel: required(parcel, "PARCELA_KN/PARCELNI_CISLO")?,
        part: required(part, "PARCELA_KN/DIL")?,
        area: required(area, "PARCELA_KN/VYMERA_PARCELY")?,
        area_determination: required(area_determination, "PARCELA_KN/URC_VYMERY")?,
        land_kind: required(land_kind, "PARCELA_KN/DRUH_POZEMKU")?,
        protection: required(protection, "PARCELA_KN/OCHRANA")?,
        map: required(map, "PARCELA_KN/MAPA")?,
        building_on_parcel,
    })
}

fn parse_ownership_building(node: Node<'_, '_>) -> Result<OwnershipBuilding> {
    const FIELDS: &[&str] = &["CP_CE", "PARCELNI_CISLO", "TYP_STAVBY", "RIZENI"];
    let mut cursor = Cursor::all(node, FIELDS)?;
    let mut house_number_buildings = None;
    let mut parcel = None;
    let mut building_type = None;
    let mut proceeding = None;
    for child in element_children(node) {
        match cursor.singleton(child)? {
            0 => {
                house_number_buildings = Some(parse_list(child, "stavba", false, parse_building)?);
            }
            1 => parcel = Some(parse_required_parcel(child, "STAVBA/PARCELNI_CISLO")?),
            2 => building_type = Some(simple_text(child)?),
            3 => proceeding = Some(parse_proceeding(child)?),
            _ => unreachable!("cursor returned an in-range index"),
        }
    }
    Ok(OwnershipBuilding {
        house_number_buildings: required(house_number_buildings, "STAVBA/CP_CE")?,
        parcel: required(parcel, "STAVBA/PARCELNI_CISLO")?,
        building_type: required(building_type, "STAVBA/TYP_STAVBY")?,
        proceeding: required(proceeding, "STAVBA/RIZENI")?,
    })
}

fn parse_required_parcel(node: Node<'_, '_>, field: &'static str) -> Result<Parcel> {
    required(parse_parcel_wrapper(node)?.parcel, field)
}

fn parse_ownership_unit(node: Node<'_, '_>) -> Result<OwnershipUnit> {
    const FIELDS: &[&str] = &["CISLO_JED", "ZP_VYUZITI", "BUD_PODIL", "BUDOVA", "RIZENI"];
    let mut cursor = Cursor::all(node, FIELDS)?;
    let mut unit = None;
    let mut use_description = None;
    let mut building_share = None;
    let mut building = None;
    let mut proceeding = None;
    for child in element_children(node) {
        match cursor.singleton(child)? {
            0 => unit = Some(parse_unit_wrapper(child)?),
            1 => use_description = Some(simple_text(child)?),
            2 => building_share = Some(parse_reported_share(child)?),
            3 => building = Some(parse_building_wrapper(child)?),
            4 => proceeding = Some(parse_proceeding(child)?),
            _ => unreachable!("cursor returned an in-range index"),
        }
    }
    Ok(OwnershipUnit {
        unit: required(unit, "JEDNOTKA/CISLO_JED")?,
        use_description: required(use_description, "JEDNOTKA/ZP_VYUZITI")?,
        building_share: required(building_share, "JEDNOTKA/BUD_PODIL")?,
        building: required(building, "JEDNOTKA/BUDOVA")?,
        proceeding: required(proceeding, "JEDNOTKA/RIZENI")?,
    })
}

fn parse_ownership_source(node: Node<'_, '_>) -> Result<OwnershipSource> {
    let mut cursor = Cursor::sequence(node, &["PUVOD", "PARCELY_ZE"])?;
    let mut origin = None;
    let mut parcels = None;
    for child in element_children(node) {
        match cursor.singleton(child)? {
            0 => origin = Some(simple_text(child)?),
            1 => {
                parcels = Some(parse_list(
                    child,
                    "PARCELA_ZE",
                    false,
                    parse_ownership_source_parcel,
                )?);
            }
            _ => unreachable!("cursor returned an in-range index"),
        }
    }
    Ok(OwnershipSource {
        origin: required(origin, "ZDROJ/PUVOD")?,
        parcels: required(parcels, "ZDROJ/PARCELY_ZE")?,
    })
}

fn parse_ownership_source_parcel(node: Node<'_, '_>) -> Result<OwnershipSourceParcel> {
    const FIELDS: &[&str] = &[
        "PARCELNI_CISLO",
        "DIL",
        "TYP",
        "VYMERA",
        "PUVODNI_KU",
        "PARCELA_KN",
    ];
    let mut cursor = Cursor::all(node, FIELDS)?;
    let mut parcel = None;
    let mut part = None;
    let mut kind = None;
    let mut area = None;
    let mut original_cadastral_area = None;
    let mut current_parcel = None;
    for child in element_children(node) {
        match cursor.singleton(child)? {
            0 => parcel = Some(parse_parcel_wrapper(child)?),
            1 => part = Some(simple_text(child)?),
            2 => kind = Some(simple_text(child)?),
            3 => area = Some(parse_integer(child, "VYMERA")?),
            4 => original_cadastral_area = Some(parse_original_cadastral_area(child)?),
            5 => current_parcel = Some(parse_parcel_wrapper(child)?),
            _ => unreachable!("cursor returned an in-range index"),
        }
    }
    Ok(OwnershipSourceParcel {
        parcel: required(parcel, "PARCELA_ZE/PARCELNI_CISLO")?,
        part: required(part, "PARCELA_ZE/DIL")?,
        kind: required(kind, "PARCELA_ZE/TYP")?,
        area: required(area, "PARCELA_ZE/VYMERA")?,
        original_cadastral_area: required(original_cadastral_area, "PARCELA_ZE/PUVODNI_KU")?,
        current_parcel: required(current_parcel, "PARCELA_ZE/PARCELA_KN")?,
    })
}

fn parse_original_cadastral_area(node: Node<'_, '_>) -> Result<OwnershipOriginalCadastralArea> {
    let mut cursor = Cursor::sequence(node, &["kod", "nazev"])?;
    let mut code = None;
    let mut name = None;
    let mut has_children = false;
    for child in element_children(node) {
        has_children = true;
        match cursor.singleton(child)? {
            0 => code = Some(parse_integer(child, "PUVODNI_KU/kod")?),
            1 => name = Some(simple_text(child)?),
            _ => unreachable!("cursor returned an in-range index"),
        }
    }
    if !has_children {
        return Ok(OwnershipOriginalCadastralArea {
            cadastral_area: None,
        });
    }
    Ok(OwnershipOriginalCadastralArea {
        cadastral_area: Some(LegacyReportCodeName {
            code: required(code, "PUVODNI_KU/kod")?,
            name: required(name, "PUVODNI_KU/nazev")?,
        }),
    })
}

fn parse_ownership_building_right(node: Node<'_, '_>) -> Result<OwnershipBuildingRight> {
    require_element(node)?;
    let mut entries = Vec::new();
    let mut current: Option<BuildingRightEntryBuilder> = None;
    for child in element_children(node) {
        require_element(child)?;
        if child.tag_name().name() == "PS_IDENT" {
            if let Some(builder) = current.take() {
                entries.push(builder.finish()?);
            }
            current = Some(BuildingRightEntryBuilder::new(parse_ps_ident(child)?));
            continue;
        }
        let builder = current
            .as_mut()
            .ok_or_else(|| invalid_value("PRAVO_STAVBY/PS_IDENT", "required element is missing"))?;
        builder.consume(child)?;
    }
    if let Some(builder) = current {
        entries.push(builder.finish()?);
    }
    Ok(OwnershipBuildingRight { entries })
}

struct BuildingRightEntryBuilder {
    building_right: BuildingRight,
    purposes: Option<Vec<String>>,
    valid_until: Option<String>,
    proceeding: Option<Proceeding>,
    last: usize,
}

impl BuildingRightEntryBuilder {
    fn new(building_right: BuildingRight) -> Self {
        Self {
            building_right,
            purposes: None,
            valid_until: None,
            proceeding: None,
            last: 0,
        }
    }

    fn consume(&mut self, node: Node<'_, '_>) -> Result<()> {
        require_element(node)?;
        let index = match node.tag_name().name() {
            "UCELY_PRAVA_STAVBY" => 1,
            "PLATNOST_DO" => 2,
            "RIZENI" => 3,
            name => return Err(unexpected_field(name)),
        };
        if index <= self.last {
            return Err(invalid_value(
                node.tag_name().name(),
                "building-right sequence element is duplicated or out of order",
            ));
        }
        self.last = index;
        match index {
            1 => self.purposes = Some(parse_list(node, "nazev", true, simple_text)?),
            2 => self.valid_until = Some(simple_text(node)?),
            3 => self.proceeding = Some(parse_proceeding(node)?),
            _ => unreachable!("building-right index is in range"),
        }
        Ok(())
    }

    fn finish(self) -> Result<OwnershipBuildingRightEntry> {
        Ok(OwnershipBuildingRightEntry {
            building_right: self.building_right,
            purposes: self.purposes,
            valid_until: required(self.valid_until, "PRAVO_STAVBY/PLATNOST_DO")?,
            proceeding: required(self.proceeding, "PRAVO_STAVBY/RIZENI")?,
        })
    }
}

fn parse_ps_ident(node: Node<'_, '_>) -> Result<BuildingRight> {
    let mut cursor = Cursor::sequence(node, &["pravo_stavby"])?;
    let mut building_right = None;
    for child in element_children(node) {
        cursor.singleton(child)?;
        building_right = Some(parse_building_right(child)?);
    }
    required(building_right, "PS_IDENT/pravo_stavby")
}

fn parse_rights_query(node: Node<'_, '_>) -> Result<RightsRecordQuery> {
    const FIELDS: &[&str] = &["TYP_OS", "NAZEV_OS", "ICO", "PRIJMENI", "JMENO", "RC"];
    let mut cursor = Cursor::sequence(node, FIELDS)?;
    let mut values: Vec<Option<String>> = (0..FIELDS.len()).map(|_| None).collect();
    for child in element_children(node) {
        let index = cursor.singleton(child)?;
        values[index] = Some(simple_text(child)?);
    }
    let mut values = values.into_iter();
    Ok(RightsRecordQuery {
        subject_kind: required(values.next().flatten(), "ZADANE_UDAJE/TYP_OS")?,
        subject_name: required(values.next().flatten(), "ZADANE_UDAJE/NAZEV_OS")?,
        company_id: required(values.next().flatten(), "ZADANE_UDAJE/ICO")?,
        surname: required(values.next().flatten(), "ZADANE_UDAJE/PRIJMENI")?,
        given_name: required(values.next().flatten(), "ZADANE_UDAJE/JMENO")?,
        personal_number: required(values.next().flatten(), "ZADANE_UDAJE/RC")?,
    })
}

fn parse_exact_match(node: Node<'_, '_>) -> Result<RightsExactMatch> {
    let mut cursor = Cursor::sequence(node, &["VLASTNICTVI"])?;
    let mut ownership = None;
    for child in element_children(node) {
        cursor.singleton(child)?;
        ownership = Some(parse_rights_ownership(child)?);
    }
    Ok(RightsExactMatch { ownership })
}

fn parse_mismatch(node: Node<'_, '_>) -> Result<RightsMismatch> {
    let mut cursor = Cursor::sequence(node, &["TEXT"])?;
    let mut text = None;
    for child in element_children(node) {
        cursor.singleton(child)?;
        text = Some(simple_text(child)?);
    }
    Ok(RightsMismatch { text })
}

fn parse_partial_match(node: Node<'_, '_>) -> Result<RightsPartialMatch> {
    let mut cursor = Cursor::sequence(node, &["TEXT", "OPRAV_SUBJEKTY"])?;
    let mut text = None;
    let mut subjects = None;
    let mut has_children = false;
    for child in element_children(node) {
        has_children = true;
        match cursor.singleton(child)? {
            0 => text = Some(simple_text(child)?),
            1 => {
                subjects = Some(parse_list(
                    child,
                    "OPSUB",
                    true,
                    parse_rights_partial_subject,
                )?);
            }
            _ => unreachable!("cursor returned an in-range index"),
        }
    }
    if !has_children {
        return Ok(RightsPartialMatch { details: None });
    }
    Ok(RightsPartialMatch {
        details: Some(RightsPartialMatchDetails {
            text: required(text, "CASTECNA_SHODA/TEXT")?,
            subjects: required(subjects, "CASTECNA_SHODA/OPRAV_SUBJEKTY")?,
        }),
    })
}

fn parse_rights_partial_subject(node: Node<'_, '_>) -> Result<RightsPartialSubject> {
    let (name, identification, addresses, characteristic) = parse_rights_subject_identity(node)?;
    Ok(RightsPartialSubject {
        name,
        identification,
        addresses,
        characteristic,
    })
}

fn parse_rights_subject_identity(
    node: Node<'_, '_>,
) -> Result<(
    SubjectNameForm,
    LegacySubjectIdentification,
    Vec<Address>,
    Option<CharacteristicWrapper>,
)> {
    const FIELDS: &[&str] = &["NAZEV", "IDENT", "ADRESA", "CHAROS"];
    let mut cursor = Cursor::sequence(node, FIELDS)?;
    let mut name = None;
    let mut identification = None;
    let mut addresses = None;
    let mut characteristic = None;
    for child in element_children(node) {
        match cursor.singleton(child)? {
            0 => name = Some(parse_subject_name(child)?),
            1 => identification = Some(parse_legacy_identification(child, false)?),
            2 => addresses = Some(parse_addresses(child)?),
            3 => characteristic = Some(parse_characteristic_wrapper(child)?),
            _ => unreachable!("cursor returned an in-range index"),
        }
    }
    Ok((
        required(name, "OPSUB/NAZEV")?,
        required(identification, "OPSUB/IDENT")?,
        required(addresses, "OPSUB/ADRESA")?,
        characteristic,
    ))
}

fn parse_rights_ownership(node: Node<'_, '_>) -> Result<RightsOwnership> {
    let mut cursor = Cursor::sequence(node, &["ODPOVIDA_IDENTIFIKACI", "OPRAV_SUBJEKT", "TEXT"])?;
    let mut identification_match = None;
    let mut subjects = None;
    let mut text = None;
    for child in element_children(node) {
        match cursor.singleton(child)? {
            0 => {
                identification_match =
                    Some(RightsIdentificationMatch::from_raw(simple_text(child)?));
            }
            1 => subjects = Some(parse_rights_subjects(child)?),
            2 => text = Some(simple_text(child)?),
            _ => unreachable!("cursor returned an in-range index"),
        }
    }
    Ok(RightsOwnership {
        identification_match: required(identification_match, "VLASTNICTVI/ODPOVIDA_IDENTIFIKACI")?,
        subjects: required(subjects, "VLASTNICTVI/OPRAV_SUBJEKT")?,
        text: required(text, "VLASTNICTVI/TEXT")?,
    })
}

fn parse_rights_subjects(node: Node<'_, '_>) -> Result<Vec<RightsSubject>> {
    require_element(node)?;
    let mut subjects = Vec::new();
    let mut current: Option<RightsSubjectBuilder> = None;
    for child in element_children(node) {
        require_element(child)?;
        match child.tag_name().name() {
            "NAZEV" => {
                if let Some(builder) = current.take() {
                    subjects.push(builder.finish()?);
                }
                current = Some(RightsSubjectBuilder::new(node, child)?);
            }
            "IDENT" | "ADRESA" | "CHAROS" | "PRAV_VZTAHY" => {
                current
                    .as_mut()
                    .ok_or_else(|| {
                        invalid_value("VLASTNICTVI/OPRAV_SUBJEKT", "subject NAZEV is missing")
                    })?
                    .consume(child)?;
            }
            name => return Err(unexpected_field(name)),
        }
    }
    if let Some(builder) = current {
        subjects.push(builder.finish()?);
    }
    ensure_nonempty(&subjects, "VLASTNICTVI/OPRAV_SUBJEKT", true)?;
    Ok(subjects)
}

struct RightsSubjectBuilder {
    cursor: Cursor,
    name: Option<SubjectNameForm>,
    identification: Option<LegacySubjectIdentification>,
    addresses: Option<Vec<Address>>,
    characteristic: Option<CharacteristicWrapper>,
    legal_relations: Option<Vec<RightsLegalRelationGroup>>,
}

impl RightsSubjectBuilder {
    const FIELDS: &'static [&'static str] = &["NAZEV", "IDENT", "ADRESA", "CHAROS", "PRAV_VZTAHY"];

    fn new(parent: Node<'_, '_>, name_node: Node<'_, '_>) -> Result<Self> {
        let mut cursor = Cursor::sequence(parent, Self::FIELDS)?;
        cursor.singleton(name_node)?;
        Ok(Self {
            cursor,
            name: Some(parse_subject_name(name_node)?),
            identification: None,
            addresses: None,
            characteristic: None,
            legal_relations: None,
        })
    }

    fn consume(&mut self, node: Node<'_, '_>) -> Result<()> {
        match self.cursor.singleton(node)? {
            1 => self.identification = Some(parse_legacy_identification(node, false)?),
            2 => self.addresses = Some(parse_addresses(node)?),
            3 => self.characteristic = Some(parse_characteristic_wrapper(node)?),
            4 => {
                self.legal_relations = Some(parse_list(
                    node,
                    "TYPRAV",
                    false,
                    parse_rights_legal_relation,
                )?);
            }
            _ => unreachable!("only non-NAZEV fields are consumed"),
        }
        Ok(())
    }

    fn finish(self) -> Result<RightsSubject> {
        Ok(RightsSubject {
            name: required(self.name, "OPRAV_SUBJEKT/NAZEV")?,
            identification: required(self.identification, "OPRAV_SUBJEKT/IDENT")?,
            addresses: required(self.addresses, "OPRAV_SUBJEKT/ADRESA")?,
            characteristic: self.characteristic,
            legal_relations: required(self.legal_relations, "OPRAV_SUBJEKT/PRAV_VZTAHY")?,
        })
    }
}

fn parse_rights_legal_relation(node: Node<'_, '_>) -> Result<RightsLegalRelationGroup> {
    let mut cursor = Cursor::sequence(node, &["TYPRAV_NAZEV", "OKRESY"])?;
    let mut legal_relation = None;
    let mut districts = None;
    for child in element_children(node) {
        match cursor.singleton(child)? {
            0 => {
                legal_relation = Some(LegalRelation {
                    code: None,
                    name: simple_text(child)?,
                });
            }
            1 => districts = Some(parse_list(child, "OKRES", true, parse_rights_district)?),
            _ => unreachable!("cursor returned an in-range index"),
        }
    }
    Ok(RightsLegalRelationGroup {
        legal_relation: required(legal_relation, "TYPRAV/TYPRAV_NAZEV")?,
        districts: required(districts, "TYPRAV/OKRESY")?,
    })
}

fn parse_rights_district(node: Node<'_, '_>) -> Result<RightsDistrict> {
    let mut cursor = Cursor::sequence(node, &["OKRES", "OBCE"])?;
    let mut district = None;
    let mut municipalities = None;
    for child in element_children(node) {
        match cursor.singleton(child)? {
            0 => district = Some(parse_district(child)?),
            1 => {
                municipalities = Some(parse_list(child, "OBEC", true, parse_rights_municipality)?);
            }
            _ => unreachable!("cursor returned an in-range index"),
        }
    }
    Ok(RightsDistrict {
        district: required(district, "OKRES/OKRES")?,
        municipalities: required(municipalities, "OKRES/OBCE")?,
    })
}

fn parse_rights_municipality(node: Node<'_, '_>) -> Result<RightsMunicipality> {
    let mut cursor = Cursor::sequence(node, &["OBEC", "KATASTRY"])?;
    let mut municipality = None;
    let mut cadastral_areas = None;
    for child in element_children(node) {
        match cursor.singleton(child)? {
            0 => municipality = Some(parse_code_name(child)?),
            1 => {
                cadastral_areas = Some(parse_list(
                    child,
                    "KATASTR",
                    true,
                    parse_rights_cadastral_area,
                )?);
            }
            _ => unreachable!("cursor returned an in-range index"),
        }
    }
    Ok(RightsMunicipality {
        municipality: required(municipality, "OBEC/OBEC")?,
        cadastral_areas: required(cadastral_areas, "OBEC/KATASTRY")?,
    })
}

fn parse_rights_cadastral_area(node: Node<'_, '_>) -> Result<RightsCadastralArea> {
    let mut cursor = Cursor::sequence(node, &["KATUZE", "TELESA"])?;
    let mut cadastral_area = None;
    let mut records = None;
    for child in element_children(node) {
        match cursor.singleton(child)? {
            0 => cadastral_area = Some(parse_code_name(child)?),
            1 => {
                records = Some(parse_list(
                    child,
                    "TELESO",
                    true,
                    parse_rights_ownership_record,
                )?);
            }
            _ => unreachable!("cursor returned an in-range index"),
        }
    }
    Ok(RightsCadastralArea {
        cadastral_area: required(cadastral_area, "KATASTR/KATUZE")?,
        records: required(records, "KATASTR/TELESA")?,
    })
}

fn parse_rights_ownership_record(node: Node<'_, '_>) -> Result<RightsOwnershipRecord> {
    const FIELDS: &[&str] = &["LV", "JPV_LV", "TEL_ID", "PRSV_IDENT", "RIZENI", "PODIL"];
    let mut cursor = Cursor::sequence(node, FIELDS)?;
    let mut ownership_sheet_number = None;
    let mut legal_relation_sheet = None;
    let mut ownership_sheet_id = None;
    let mut properties = None;
    let mut proceeding = None;
    let mut share = None;
    for child in element_children(node) {
        match cursor.singleton(child)? {
            0 => ownership_sheet_number = Some(simple_text(child)?),
            1 => legal_relation_sheet = Some(simple_text(child)?),
            2 => ownership_sheet_id = Some(parse_legacy_id(child)?),
            3 => properties = Some(parse_property_identification(child)?),
            4 => proceeding = Some(parse_proceeding(child)?),
            5 => share = Some(parse_reported_share(child)?),
            _ => unreachable!("cursor returned an in-range index"),
        }
    }
    Ok(RightsOwnershipRecord {
        ownership_sheet_number: required(ownership_sheet_number, "TELESO/LV")?,
        legal_relation_sheet: required(legal_relation_sheet, "TELESO/JPV_LV")?,
        ownership_sheet_id: required(ownership_sheet_id, "TELESO/TEL_ID")?,
        properties: required(properties, "TELESO/PRSV_IDENT")?,
        proceeding: required(proceeding, "TELESO/RIZENI")?,
        share: required(share, "TELESO/PODIL")?,
    })
}

fn parse_district(node: Node<'_, '_>) -> Result<LegacyReportDistrict> {
    let mut cursor = Cursor::sequence(node, &["nuts4", "nazev"])?;
    let mut district = LegacyReportDistrict::default();
    for child in element_children(node) {
        match cursor.singleton(child)? {
            0 => district.nuts4 = Some(simple_text(child)?),
            1 => district.name = Some(simple_text(child)?),
            _ => unreachable!("cursor returned an in-range index"),
        }
    }
    Ok(district)
}

fn parse_code_name(node: Node<'_, '_>) -> Result<LegacyReportCodeName> {
    let mut cursor = Cursor::sequence(node, &["kod", "nazev"])?;
    let mut code = None;
    let mut name = None;
    for child in element_children(node) {
        match cursor.singleton(child)? {
            0 => code = Some(parse_integer(child, "kod")?),
            1 => name = Some(simple_text(child)?),
            _ => unreachable!("cursor returned an in-range index"),
        }
    }
    Ok(LegacyReportCodeName {
        code: required(code, "kod")?,
        name: required(name, "nazev")?,
    })
}

fn parse_characteristic_code(node: Node<'_, '_>) -> Result<CharacteristicCode> {
    let value: u8 = parse_integer(node, "CharakteristikaOSKodType")?;
    CharacteristicCode::new(value)
        .map_err(|_| invalid_value("CharakteristikaOSKodType", "expected a code in 1..=99"))
}

fn parse_legacy_id(node: Node<'_, '_>) -> Result<LegacyId> {
    super::legacy_base::parse_legacy_id(node)
}

fn parse_list<T>(
    node: Node<'_, '_>,
    child_name: &'static str,
    require_nonempty: bool,
    mut parse: impl FnMut(Node<'_, '_>) -> Result<T>,
) -> Result<Vec<T>> {
    require_element(node)?;
    let mut values = Vec::new();
    for child in element_children(node) {
        require_element(child)?;
        if child.tag_name().name() != child_name {
            return Err(unexpected_field(child.tag_name().name()));
        }
        values.push(parse(child)?);
    }
    ensure_nonempty(&values, node.tag_name().name(), require_nonempty)?;
    Ok(values)
}

fn ensure_nonempty<T>(values: &[T], field: &str, required_nonempty: bool) -> Result<()> {
    if required_nonempty && values.is_empty() {
        Err(invalid_value(field, "at least one occurrence is required"))
    } else {
        Ok(())
    }
}

struct Cursor {
    fields: &'static [&'static str],
    ordered: bool,
    last: Option<usize>,
    seen: Vec<bool>,
}

impl Cursor {
    fn sequence(node: Node<'_, '_>, fields: &'static [&'static str]) -> Result<Self> {
        Self::new(node, fields, true)
    }

    fn all(node: Node<'_, '_>, fields: &'static [&'static str]) -> Result<Self> {
        Self::new(node, fields, false)
    }

    fn new(node: Node<'_, '_>, fields: &'static [&'static str], ordered: bool) -> Result<Self> {
        require_element(node)?;
        Ok(Self {
            fields,
            ordered,
            last: None,
            seen: vec![false; fields.len()],
        })
    }

    fn singleton(&mut self, node: Node<'_, '_>) -> Result<usize> {
        require_element(node)?;
        let name = node.tag_name().name();
        let index = self
            .fields
            .iter()
            .position(|candidate| *candidate == name)
            .ok_or_else(|| unexpected_field(name))?;
        if self.ordered && self.last.is_some_and(|last| index < last) {
            return Err(invalid_value(
                name,
                "legacy sequence element is out of order",
            ));
        }
        if self.seen[index] {
            return Err(invalid_value(
                name,
                "singleton legacy element occurs more than once",
            ));
        }
        self.last = Some(index);
        self.seen[index] = true;
        Ok(index)
    }
}

fn element_children<'a, 'input>(node: Node<'a, 'input>) -> impl Iterator<Item = Node<'a, 'input>> {
    node.children().filter(|child| child.is_element())
}

fn simple_text(node: Node<'_, '_>) -> Result<String> {
    require_element(node)?;
    if node
        .attribute((XSI_NS, "nil"))
        .is_some_and(|value| value == "true" || value == "1")
    {
        return Err(invalid_value(
            node.tag_name().name(),
            "legacy XSD element is not nillable",
        ));
    }
    let mut content = String::new();
    for child in node.children() {
        if child.is_element() {
            return Err(invalid_value(
                node.tag_name().name(),
                "simple content cannot contain an element",
            ));
        }
        if child.is_text()
            && let Some(text) = child.text()
        {
            content.push_str(text);
        }
    }
    Ok(content)
}

fn parse_integer<T>(node: Node<'_, '_>, field: &'static str) -> Result<T>
where
    T: FromStr,
{
    let raw = simple_text(node)?;
    trim_xml_whitespace(&raw)
        .parse::<T>()
        .map_err(|_| invalid_value(field, "integer is invalid or outside its XSD range"))
}

fn trim_xml_whitespace(value: &str) -> &str {
    value.trim_matches(|character| matches!(character, ' ' | '\t' | '\r' | '\n'))
}

fn require_element(node: Node<'_, '_>) -> Result<()> {
    if node.is_element() && node.tag_name().namespace().is_none() {
        Ok(())
    } else {
        Err(Error::invalid_response(
            "legacy report elements must have no XML namespace",
        ))
    }
}

fn required<T>(value: Option<T>, field: &str) -> Result<T> {
    value.ok_or_else(|| invalid_value(field, "required legacy element is missing"))
}

fn unexpected_field(name: &str) -> Error {
    Error::invalid_response(format!("unexpected legacy report element {name}"))
}

fn invalid_value(field: &str, message: &str) -> Error {
    Error::invalid_response(format!("invalid legacy report element {field}: {message}"))
}

fn preflight(xml: &str, max_nodes: u32, max_depth: u32) -> Result<()> {
    let mut reader = quick_xml::Reader::from_str(xml);
    let mut nodes = 0_u32;
    let mut depth = 0_u32;
    loop {
        match reader.read_event() {
            Ok(Event::Eof) => break,
            Ok(Event::Start(element)) => {
                nodes = preflight_node(nodes, max_nodes)?;
                nodes = preflight_attributes(&element, nodes, max_nodes)?;
                depth = depth.checked_add(1).ok_or_else(|| {
                    Error::invalid_response("legacy ownership XML exceeds the nesting limit")
                })?;
                if depth > max_depth {
                    return Err(Error::invalid_response(
                        "legacy ownership XML exceeds the nesting limit",
                    ));
                }
            }
            Ok(Event::Empty(element)) => {
                if depth >= max_depth {
                    return Err(Error::invalid_response(
                        "legacy ownership XML exceeds the nesting limit",
                    ));
                }
                nodes = preflight_node(nodes, max_nodes)?;
                nodes = preflight_attributes(&element, nodes, max_nodes)?;
            }
            Ok(Event::End(_)) => {
                depth = depth.checked_sub(1).ok_or_else(|| {
                    Error::invalid_response("malformed legacy ownership-report XML")
                })?;
            }
            Ok(Event::DocType(_)) => {
                return Err(Error::invalid_response(
                    "DTD is not allowed in legacy ownership-report XML",
                ));
            }
            Ok(
                Event::Text(_)
                | Event::CData(_)
                | Event::Comment(_)
                | Event::Decl(_)
                | Event::PI(_)
                | Event::GeneralRef(_),
            ) => nodes = preflight_node(nodes, max_nodes)?,
            Err(_) => {
                return Err(Error::invalid_response(
                    "malformed legacy ownership-report XML",
                ));
            }
        }
    }
    if depth != 0 {
        return Err(Error::invalid_response(
            "malformed legacy ownership-report XML",
        ));
    }
    Ok(())
}

fn preflight_node(nodes: u32, maximum: u32) -> Result<u32> {
    let nodes = nodes
        .checked_add(1)
        .ok_or_else(|| Error::invalid_response("legacy ownership XML exceeds the node limit"))?;
    if nodes > maximum {
        return Err(Error::invalid_response(
            "legacy ownership XML exceeds the node limit",
        ));
    }
    Ok(nodes)
}

fn preflight_attributes(element: &BytesStart<'_>, mut nodes: u32, maximum: u32) -> Result<u32> {
    for attribute in element.attributes() {
        if attribute.is_err() {
            return Err(Error::invalid_response(
                "malformed legacy ownership-report XML",
            ));
        }
        nodes = preflight_node(nodes, maximum)?;
    }
    Ok(nodes)
}

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;

    use super::*;
    use crate::model::reports::legacy_base::{PropertyTarget, YesNo};

    const MAX_NODES: u32 = 100_000;
    const MAX_DEPTH: u32 = 128;

    fn overview_xml() -> &'static str {
        r#"<PrehledVlastnictvi>
          <VYHOTOVENO>created-at</VYHOTOVENO><PRAV_VZTAHY><TYPPRA><TYPRAV_NAZEV>requested relation</TYPRAV_NAZEV></TYPPRA></PRAV_VZTAHY>
          <OPRAV_SUBJEKT>
            <CHAROS><char_os><kod>4</kod><zkratka>OPO</zkratka><zkratka_alv>OPO-ALV</zkratka_alv></char_os></CHAROS>
            <OPSUB_NAZEV><id>101</id><bsm1_id>11</bsm1_id><bsm2_id/><prijmeni>surname</prijmeni><jmeno>given</jmeno><titul_pred>before</titul_pred><titul_za>after</titul_za><bsm1_prijmeni>p1-surname</bsm1_prijmeni><bsm1_jmeno>p1-given</bsm1_jmeno><bsm1_pred>p1-before</bsm1_pred><bsm1_za>p1-after</bsm1_za><bsm2_prijmeni>p2-surname</bsm2_prijmeni><bsm2_jmeno>p2-given</bsm2_jmeno><bsm2_pred>p2-before</bsm2_pred><bsm2_za>p2-after</bsm2_za><nazev>organization</nazev></OPSUB_NAZEV>
            <OPSUB_IDENT><id>101</id><rc6>123456</rc6><rc7>1234567</rc7><ico>00000123</ico><ico_doplnek>suffix</ico_doplnek><bsm1_id>11</bsm1_id><bsm2_id>12</bsm2_id><id/></OPSUB_IDENT>
            <OPSUB_ADRESA><adresa><adresni_misto>107</adresni_misto><nazev_ulice>  street  </nazev_ulice><cp_ce>2</cp_ce><cislo_domovni>1a</cislo_domovni><cislo_orientacni>2b</cislo_orientacni><cast_obce>part</cast_obce><obec>municipality</obec><mestska_cast>district</mestska_cast><mestsky_obvod>borough</mestsky_obvod><psc>01000</psc><dod_posta>post</dod_posta><stat>country</stat></adresa><adresa/></OPSUB_ADRESA>
          </OPRAV_SUBJEKT>
          <OKRESY>
            <OKRES><nuts4>CZ0000</nuts4><nazev>district-name</nazev></OKRES><OBCE>
              <OBEC><kod>4294967295</kod><nazev>municipality-name</nazev></OBEC><KATASTRALNI_UZEMI>
                <KATUZE><kod>0</kod><nazev>area-name</nazev></KATUZE><TYPY_PRAV_VZTAHU>
                  <TYP_PRAV_VZTAHU><TYPRAV_NAZEV>relation-name</TYPRAV_NAZEV><TELESA>
                    <TELESO><LV>sheet-one</LV><PRSV_IDENT><parcela><id>102</id><zkratka>pkn</zkratka><druh_cis>numbering</druh_cis><par_cis> 12 </par_cis><podd_cis>3</podd_cis><dil_parcely>-128</dil_parcely><tel_id>103</tel_id><cislo_tel>parcel-sheet</cislo_tel><ku_nazev>area</ku_nazev><nazev_puv>original-area</nazev_puv></parcela><stavba><id>104</id></stavba><jednotka><id>105</id></jednotka><pravo_stavby><id>106</id></pravo_stavby></PRSV_IDENT><PODIL><citatel>-7</citatel><jmenovatel>0</jmenovatel></PODIL><TEL_ID>103</TEL_ID><JPV_LV>relation-sheet</JPV_LV><RIZENI><typriz_kod>V</typriz_kod><poradove_cislo>4294967295</poradove_cislo><rok>65535</rok><prares_kod>65535</prares_kod></RIZENI></TELESO>
                    <TELESO><LV/><PRSV_IDENT/><PODIL/><TEL_ID/><JPV_LV/><RIZENI/></TELESO>
                  </TELESA></TYP_PRAV_VZTAHU>
                </TYPY_PRAV_VZTAHU>
              </KATASTRALNI_UZEMI>
            </OBCE><OKRES/><OBCE/>
          </OKRESY>
          <CP_OMEZENI>territory</CP_OMEZENI><CP_PRACOVISTE>workplace</CP_PRACOVISTE><PLATNOST>valid-at</PLATNOST><VYHOTOVIL>creator</VYHOTOVIL><BEZPL_PRISTUP>free</BEZPL_PRISTUP><INFO_VYSTUP>output</INFO_VYSTUP>
        </PrehledVlastnictvi>"#
    }

    fn properties_xml() -> &'static str {
        r#"<PrehledVlastnictviSNemovitostmi>
          <BEZPL_PRISTUP>free</BEZPL_PRISTUP><PRACOVISTE>workplace</PRACOVISTE><PLATNOST>valid-at</PLATNOST><VYHOTOVENO>created-at</VYHOTOVENO><VYHOTOVIL>creator</VYHOTOVIL><RIZENI/><INFO_VYSTUP/>
          <PRAV_VZTAHY><TYPPRA><TYPRAV_NAZEV>requested-properties-relation</TYPRAV_NAZEV></TYPPRA></PRAV_VZTAHY>
          <OPRAV_SUBJEKT><CHAROS>99</CHAROS><OPSUB_NAZEV><id>108</id><nazev>owner</nazev></OPSUB_NAZEV><OPSUB_IDENT><id>108</id><ico>00000123</ico></OPSUB_IDENT><OPSUB_ADRESA/></OPRAV_SUBJEKT>
          <OKRESY><OKRES/><OBCE><OBEC><kod>1</kod><nazev>municipality</nazev></OBEC><KATASTRALNI_UZEMI><KATUZE><kod>2</kod><nazev>area</nazev></KATUZE><TELESA><TELESO>
            <PODIL><citatel>1</citatel><jmenovatel>4</jmenovatel></PODIL><LV>sheet</LV>
            <PARCELY_KN>
              <PARCELA_KN><MAPA>map</MAPA><OCHRANA>a</OCHRANA><DRUH_POZEMKU>land-kind</DRUH_POZEMKU><URC_VYMERY>determination</URC_VYMERY><VYMERA_PARCELY>4294967295</VYMERA_PARCELY><DIL>part</DIL><PARCELNI_CISLO><parcela><id>109</id></parcela></PARCELNI_CISLO><ZP_VYUZITI>parcel-use</ZP_VYUZITI><STAVBA_NA_PARCELE><stavba><id>110</id></stavba></STAVBA_NA_PARCELE></PARCELA_KN>
              <PARCELA_KN><PARCELNI_CISLO/><DIL/><VYMERA_PARCELY>0</VYMERA_PARCELY><URC_VYMERY/><DRUH_POZEMKU/><OCHRANA>n</OCHRANA><MAPA/></PARCELA_KN>
            </PARCELY_KN>
            <STAVBY><STAVBA><RIZENI/><TYP_STAVBY>building-type</TYP_STAVBY><PARCELNI_CISLO><parcela><id>111</id></parcela></PARCELNI_CISLO><CP_CE><stavba><id>112</id></stavba><stavba><id>113</id><par_ident><parcela><id>114</id></parcela></par_ident></stavba></CP_CE></STAVBA></STAVBY>
            <JEDNOTKY><JEDNOTKA><RIZENI/><BUDOVA/><BUD_PODIL><citatel>2</citatel><jmenovatel>5</jmenovatel></BUD_PODIL><ZP_VYUZITI>unit-use</ZP_VYUZITI><CISLO_JED><jednotka><id>105</id></jednotka></CISLO_JED></JEDNOTKA></JEDNOTKY>
            <ZDROJE>
              <ZDROJ><PUVOD>origin</PUVOD><PARCELY_ZE>
                <PARCELA_ZE><PARCELA_KN><parcela><id>115</id></parcela></PARCELA_KN><PUVODNI_KU><kod>3</kod><nazev>original-area</nazev></PUVODNI_KU><VYMERA>42</VYMERA><TYP>source-kind</TYP><DIL>source-part</DIL><PARCELNI_CISLO><parcela><id>116</id></parcela></PARCELNI_CISLO></PARCELA_ZE>
                <PARCELA_ZE><PARCELNI_CISLO/><DIL/><TYP/><VYMERA>0</VYMERA><PUVODNI_KU/><PARCELA_KN/></PARCELA_ZE>
              </PARCELY_ZE></ZDROJ><ZDROJ><PUVOD>empty-source</PUVOD><PARCELY_ZE/></ZDROJ>
            </ZDROJE>
            <PRAVA_STAVBY>
              <PRAVO_STAVBY><PS_IDENT><pravo_stavby><id>117</id><parcela><id>118</id></parcela><parcela><id>119</id></parcela></pravo_stavby></PS_IDENT><UCELY_PRAVA_STAVBY><nazev>purpose-one</nazev><nazev>purpose-two</nazev></UCELY_PRAVA_STAVBY><PLATNOST_DO>until-one</PLATNOST_DO><RIZENI><typriz_kod>Z</typriz_kod><poradove_cislo>1</poradove_cislo><rok>2026</rok><prares_kod>2</prares_kod></RIZENI><PS_IDENT><pravo_stavby><id>120</id></pravo_stavby></PS_IDENT><PLATNOST_DO/><RIZENI/></PRAVO_STAVBY>
              <PRAVO_STAVBY/>
            </PRAVA_STAVBY>
          </TELESO></TELESA></KATASTRALNI_UZEMI></OBCE></OKRESY>
        </PrehledVlastnictviSNemovitostmi>"#
    }

    fn rights_xml() -> &'static str {
        r#"<EvidencePravProOsobu>
          <ZADANE_UDAJE><TYP_OS>kind</TYP_OS><NAZEV_OS>query-name</NAZEV_OS><ICO>00000123</ICO><PRIJMENI>query-surname</PRIJMENI><JMENO>query-given</JMENO><RC>query-personal-number</RC></ZADANE_UDAJE>
          <UPLNA_SHODA><VLASTNICTVI><ODPOVIDA_IDENTIFIKACI>a</ODPOVIDA_IDENTIFIKACI><OPRAV_SUBJEKT>
            <NAZEV><id>121</id><bsm1_id>11</bsm1_id><bsm2_id>12</bsm2_id><nazev>organization</nazev><prijmeni>surname</prijmeni><jmeno>given</jmeno><titul_pred>before</titul_pred><titul_za>after</titul_za><bsm1_prijmeni>p1-surname</bsm1_prijmeni><bsm1_jmeno>p1-given</bsm1_jmeno><bsm1_pred>p1-before</bsm1_pred><bsm1_za>p1-after</bsm1_za><bsm2_prijmeni>p2-surname</bsm2_prijmeni><bsm2_jmeno>p2-given</bsm2_jmeno><bsm2_pred>p2-before</bsm2_pred><bsm2_za>p2-after</bsm2_za></NAZEV>
            <IDENT><id>121</id><rc6>123456</rc6><rc7>1234567</rc7><ico>00000123</ico><ico_doplnek>suffix</ico_doplnek><bsm1_id>11</bsm1_id><bsm2_id>12</bsm2_id></IDENT>
            <ADRESA><adresa><obec>address-municipality</obec></adresa><adresa/></ADRESA><CHAROS><char_os><kod>1</kod><zkratka/><zkratka_alv/></char_os></CHAROS>
            <PRAV_VZTAHY><TYPRAV><TYPRAV_NAZEV>rights-relation</TYPRAV_NAZEV><OKRESY><OKRES><OKRES><nuts4>CZ0000</nuts4><nazev>district</nazev></OKRES><OBCE><OBEC><OBEC><kod>4</kod><nazev>municipality</nazev></OBEC><KATASTRY><KATASTR><KATUZE><kod>5</kod><nazev>area</nazev></KATUZE><TELESA>
              <TELESO><LV>rights-sheet</LV><JPV_LV>rights-relation-sheet</JPV_LV><TEL_ID>122</TEL_ID><PRSV_IDENT><parcela><id>123</id></parcela><stavba><id>124</id></stavba><jednotka><id>125</id></jednotka><pravo_stavby><id>126</id></pravo_stavby></PRSV_IDENT><RIZENI/><PODIL><citatel>3</citatel><jmenovatel>8</jmenovatel></PODIL></TELESO>
              <TELESO><LV/><JPV_LV/><TEL_ID/><PRSV_IDENT/><RIZENI/><PODIL/></TELESO>
            </TELESA></KATASTR></KATASTRY></OBEC></OBCE></OKRES></OKRESY></TYPRAV></PRAV_VZTAHY>
            <NAZEV/><IDENT><id/></IDENT><ADRESA/><PRAV_VZTAHY/>
            </OPRAV_SUBJEKT><TEXT>exact-match-text</TEXT>
          </VLASTNICTVI></UPLNA_SHODA>
          <NESHODA><TEXT>mismatch-text</TEXT></NESHODA>
          <CASTECNA_SHODA><TEXT>partial-text</TEXT><OPRAV_SUBJEKTY>
            <OPSUB><NAZEV><id>127</id></NAZEV><IDENT><ico>partial-ico</ico></IDENT><ADRESA/><CHAROS/></OPSUB>
            <OPSUB><NAZEV/><IDENT><id>128</id></IDENT><ADRESA><adresa/></ADRESA></OPSUB>
          </OPRAV_SUBJEKTY></CASTECNA_SHODA>
          <PLATNOST>valid-at</PLATNOST><VYHOTOVIL>creator</VYHOTOVIL><BEZPL_PRISTUP>free</BEZPL_PRISTUP><VYHOTOVENO>created-at</VYHOTOVENO>
        </EvidencePravProOsobu>"#
    }

    #[test]
    fn ownership_overview_retains_every_transitive_field_and_choice() {
        let report = parse_ownership_overview(overview_xml().as_bytes(), MAX_NODES, MAX_DEPTH)
            .expect("complete overview fixture must parse");
        assert_eq!(report.requested_legal_relation.code, None);
        assert_eq!(report.requested_legal_relation.is_ownership(), None);
        assert_eq!(report.requested_legal_relation.name, "requested relation");
        assert_eq!(report.output_info.as_deref(), Some("output"));
        assert_eq!(report.territorial_restriction, "territory");
        assert_eq!(report.workplace, "workplace");
        assert_eq!(report.valid_at, "valid-at");
        assert_eq!(report.created_by, "creator");
        assert_eq!(report.created_at, "created-at");
        assert_eq!(report.free_access, "free");

        let subject = &report.subject;
        assert_eq!(
            subject
                .characteristic
                .characteristic
                .as_ref()
                .and_then(|value| value.details.as_ref())
                .map(|value| value.code.get()),
            Some(4)
        );
        assert_eq!(subject.name.id.as_ref().map(LegacyId::as_str), Some("101"));
        assert_eq!(
            subject.name.partner1_id.as_ref().map(LegacyId::as_str),
            Some("11")
        );
        assert_eq!(
            subject.name.partner2_id.as_ref().map(LegacyId::as_str),
            Some("")
        );
        assert_eq!(subject.name.name.as_deref(), Some("organization"));
        assert_eq!(subject.name.person.surname.as_deref(), Some("surname"));
        assert_eq!(subject.name.person.given_name.as_deref(), Some("given"));
        assert_eq!(subject.name.person.title_before.as_deref(), Some("before"));
        assert_eq!(subject.name.person.title_after.as_deref(), Some("after"));
        assert_eq!(subject.name.partner1.surname.as_deref(), Some("p1-surname"));
        assert_eq!(
            subject.name.partner1.given_name.as_deref(),
            Some("p1-given")
        );
        assert_eq!(
            subject.name.partner1.title_before.as_deref(),
            Some("p1-before")
        );
        assert_eq!(
            subject.name.partner1.title_after.as_deref(),
            Some("p1-after")
        );
        assert_eq!(subject.name.partner2.surname.as_deref(), Some("p2-surname"));
        assert_eq!(
            subject.name.partner2.given_name.as_deref(),
            Some("p2-given")
        );
        assert_eq!(
            subject.name.partner2.title_before.as_deref(),
            Some("p2-before")
        );
        assert_eq!(
            subject.name.partner2.title_after.as_deref(),
            Some("p2-after")
        );
        assert_eq!(subject.identification.entries.len(), 8);
        assert!(
            matches!(subject.identification.entries[1], LegacySubjectIdentifier::PersonalNumber6(ref value) if value == "123456")
        );
        assert!(
            matches!(subject.identification.entries[2], LegacySubjectIdentifier::PersonalNumber7(ref value) if value == "1234567")
        );
        assert!(
            matches!(subject.identification.entries[3], LegacySubjectIdentifier::CompanyId(ref value) if value == "00000123")
        );
        assert!(
            matches!(subject.identification.entries[4], LegacySubjectIdentifier::CompanyIdSuffix(ref value) if value == "suffix")
        );
        assert!(matches!(
            subject.identification.entries[5],
            LegacySubjectIdentifier::Partner1Id(_)
        ));
        assert!(matches!(
            subject.identification.entries[6],
            LegacySubjectIdentifier::Partner2Id(_)
        ));
        assert!(
            matches!(subject.identification.entries[7], LegacySubjectIdentifier::Id(ref value) if value.as_str().is_empty())
        );
        assert_eq!(subject.addresses.len(), 2);
        assert_eq!(
            subject.addresses[0].groups[0].street_name.as_deref(),
            Some("  street  ")
        );
        assert_eq!(
            subject.addresses[0].groups[0]
                .house_number_kind
                .as_ref()
                .map(|value| value.as_str()),
            Some("2")
        );
        assert!(subject.addresses[1].groups.is_empty());

        assert_eq!(report.districts.len(), 2);
        let district = &report.districts[0];
        assert_eq!(district.district.nuts4.as_deref(), Some("CZ0000"));
        assert_eq!(district.district.name.as_deref(), Some("district-name"));
        assert_eq!(district.municipalities[0].municipality.code, u32::MAX);
        assert_eq!(
            district.municipalities[0].municipality.name,
            "municipality-name"
        );
        let area = &district.municipalities[0].cadastral_areas[0];
        assert_eq!(area.cadastral_area.code, 0);
        assert_eq!(area.cadastral_area.name, "area-name");
        assert_eq!(area.legal_relations[0].legal_relation.code, None);
        assert_eq!(area.legal_relations[0].legal_relation.is_ownership(), None);
        assert_eq!(area.legal_relations[0].records.len(), 2);
        let record = &area.legal_relations[0].records[0];
        assert_eq!(record.ownership_sheet_number, "sheet-one");
        assert_eq!(record.ownership_sheet_id.as_str(), "103");
        assert_eq!(record.legal_relation_sheet, "relation-sheet");
        assert_eq!(record.share.numerator, Some(Decimal::new(-7, 0)));
        assert_eq!(record.share.denominator, Some(Decimal::ZERO));
        assert_eq!(record.properties.targets.len(), 4);
        assert!(matches!(
            record.properties.targets[0],
            PropertyTarget::Parcel(_)
        ));
        assert!(matches!(
            record.properties.targets[1],
            PropertyTarget::Building(_)
        ));
        assert!(matches!(
            record.properties.targets[2],
            PropertyTarget::Unit(_)
        ));
        assert!(matches!(
            record.properties.targets[3],
            PropertyTarget::BuildingRight(_)
        ));
        let proceeding = record.proceeding.identification.as_ref().unwrap();
        assert_eq!(proceeding.serial_number, u32::MAX);
        assert_eq!(proceeding.year, u16::MAX);
        assert_eq!(proceeding.workplace_code, u16::MAX);
        assert!(report.districts[1].district.name.is_none());
        assert!(report.districts[1].municipalities.is_empty());
    }

    #[test]
    fn property_overview_retains_all_property_categories_and_empty_wrappers() {
        let report = parse_ownership_overview_with_properties(
            properties_xml().as_bytes(),
            MAX_NODES,
            MAX_DEPTH,
        )
        .expect("complete property fixture must parse");
        assert_eq!(
            report.requested_legal_relation.name,
            "requested-properties-relation"
        );
        assert_eq!(report.requested_legal_relation.code, None);
        assert_eq!(report.output_info, "");
        assert!(report.proceeding.identification.is_none());
        assert_eq!(report.subject.characteristic.get(), 99);
        assert!(report.subject.addresses.is_empty());
        let record = &report.districts[0].municipalities[0].cadastral_areas[0].records[0];
        assert_eq!(record.ownership_sheet_number, "sheet");
        assert_eq!(record.share.denominator, Some(Decimal::new(4, 0)));

        assert_eq!(record.parcels.len(), 2);
        let parcel = &record.parcels[0];
        assert_eq!(parcel.use_description.as_deref(), Some("parcel-use"));
        assert_eq!(parcel.area, u32::MAX);
        assert_eq!(parcel.area_determination, "determination");
        assert_eq!(parcel.part, "part");
        assert_eq!(parcel.land_kind, "land-kind");
        assert_eq!(parcel.map, "map");
        assert_eq!(parcel.protection, YesNo::Yes);
        assert_eq!(
            parcel
                .parcel
                .parcel
                .as_ref()
                .unwrap()
                .id
                .as_ref()
                .map(LegacyId::as_str),
            Some("109")
        );
        assert_eq!(
            parcel
                .building_on_parcel
                .as_ref()
                .unwrap()
                .building
                .as_ref()
                .unwrap()
                .id
                .as_ref()
                .map(LegacyId::as_str),
            Some("110")
        );
        assert!(record.parcels[1].use_description.is_none());
        assert!(record.parcels[1].building_on_parcel.is_none());
        assert!(record.parcels[1].parcel.parcel.is_none());

        assert_eq!(record.buildings.len(), 1);
        let building = &record.buildings[0];
        assert_eq!(building.house_number_buildings.len(), 2);
        assert_eq!(
            building.house_number_buildings[1]
                .identifying_parcels
                .as_ref()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            building.parcel.id.as_ref().map(LegacyId::as_str),
            Some("111")
        );
        assert_eq!(building.building_type, "building-type");
        assert!(building.proceeding.identification.is_none());

        let unit = &record.units[0];
        assert_eq!(unit.unit.unit.as_ref().unwrap().id.as_str(), "105");
        assert_eq!(unit.use_description, "unit-use");
        assert_eq!(unit.building_share.numerator, Some(Decimal::new(2, 0)));
        assert!(unit.building.building.is_none());

        assert_eq!(record.sources.len(), 2);
        assert_eq!(record.sources[0].origin, "origin");
        assert_eq!(record.sources[0].parcels.len(), 2);
        let source = &record.sources[0].parcels[0];
        assert_eq!(source.part, "source-part");
        assert_eq!(source.kind, "source-kind");
        assert_eq!(source.area, 42);
        assert_eq!(
            source
                .original_cadastral_area
                .cadastral_area
                .as_ref()
                .unwrap()
                .code,
            3
        );
        assert_eq!(
            source
                .current_parcel
                .parcel
                .as_ref()
                .unwrap()
                .id
                .as_ref()
                .map(LegacyId::as_str),
            Some("115")
        );
        assert!(
            record.sources[0].parcels[1]
                .original_cadastral_area
                .cadastral_area
                .is_none()
        );
        assert!(record.sources[0].parcels[1].parcel.parcel.is_none());
        assert!(record.sources[1].parcels.is_empty());

        assert_eq!(record.building_rights.len(), 2);
        assert_eq!(record.building_rights[0].entries.len(), 2);
        let right = &record.building_rights[0].entries[0];
        assert_eq!(
            right.building_right.id.as_ref().map(LegacyId::as_str),
            Some("117")
        );
        assert_eq!(right.building_right.parcels.len(), 2);
        assert_eq!(
            right.purposes.as_deref(),
            Some(&["purpose-one".to_owned(), "purpose-two".to_owned()][..])
        );
        assert_eq!(right.valid_until, "until-one");
        assert_eq!(right.proceeding.identification.as_ref().unwrap().year, 2026);
        assert!(record.building_rights[0].entries[1].purposes.is_none());
        assert!(record.building_rights[1].entries.is_empty());
    }

    #[test]
    fn rights_record_retains_exact_partial_and_mismatch_content() {
        let report = parse_rights_record(rights_xml().as_bytes(), MAX_NODES, MAX_DEPTH).unwrap();
        assert_eq!(report.query.subject_kind, "kind");
        assert_eq!(report.query.subject_name, "query-name");
        assert_eq!(report.query.company_id, "00000123");
        assert_eq!(report.query.surname, "query-surname");
        assert_eq!(report.query.given_name, "query-given");
        assert_eq!(report.query.personal_number, "query-personal-number");
        assert_eq!(
            report.mismatch.as_ref().unwrap().text.as_deref(),
            Some("mismatch-text")
        );

        let ownership = report
            .exact_match
            .as_ref()
            .unwrap()
            .ownership
            .as_ref()
            .unwrap();
        assert_eq!(
            ownership.identification_match,
            RightsIdentificationMatch::Matches
        );
        assert_eq!(ownership.text, "exact-match-text");
        assert_eq!(ownership.subjects.len(), 2);
        let subject = &ownership.subjects[0];
        assert_eq!(subject.name.name.as_deref(), Some("organization"));
        assert_eq!(
            subject.name.partner2.title_after.as_deref(),
            Some("p2-after")
        );
        assert_eq!(subject.identification.entries.len(), 7);
        assert_eq!(subject.addresses.len(), 2);
        assert!(
            subject
                .characteristic
                .as_ref()
                .unwrap()
                .characteristic
                .is_some()
        );
        assert_eq!(subject.legal_relations.len(), 1);
        let relation = &subject.legal_relations[0];
        assert_eq!(relation.legal_relation.code, None);
        assert_eq!(relation.legal_relation.is_ownership(), None);
        assert_eq!(
            relation.districts[0].district.name.as_deref(),
            Some("district")
        );
        assert_eq!(relation.districts[0].municipalities[0].municipality.code, 4);
        let records = &relation.districts[0].municipalities[0].cadastral_areas[0].records;
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].ownership_sheet_number, "rights-sheet");
        assert_eq!(records[0].legal_relation_sheet, "rights-relation-sheet");
        assert_eq!(records[0].ownership_sheet_id.as_str(), "122");
        assert_eq!(records[0].properties.targets.len(), 4);
        assert_eq!(records[0].share.numerator, Some(Decimal::new(3, 0)));
        assert_eq!(records[0].share.denominator, Some(Decimal::new(8, 0)));
        assert!(ownership.subjects[1].characteristic.is_none());
        assert!(ownership.subjects[1].legal_relations.is_empty());

        let partial = report
            .partial_match
            .as_ref()
            .unwrap()
            .details
            .as_ref()
            .unwrap();
        assert_eq!(partial.text, "partial-text");
        assert_eq!(partial.subjects.len(), 2);
        assert_eq!(
            partial.subjects[0].name.id.as_ref().map(LegacyId::as_str),
            Some("127")
        );
        assert!(
            partial.subjects[0]
                .characteristic
                .as_ref()
                .unwrap()
                .characteristic
                .is_none()
        );
        assert_eq!(partial.subjects[1].addresses.len(), 1);
    }

    #[test]
    fn optional_rights_wrappers_distinguish_absent_empty_and_complete() {
        let root_start = "<EvidencePravProOsobu><ZADANE_UDAJE><TYP_OS/><NAZEV_OS/><ICO/><PRIJMENI/><JMENO/><RC/></ZADANE_UDAJE>";
        let root_end =
            "<PLATNOST/><VYHOTOVIL/><BEZPL_PRISTUP/><VYHOTOVENO/></EvidencePravProOsobu>";
        let absent = parse_rights_record(
            format!("{root_start}{root_end}").as_bytes(),
            MAX_NODES,
            MAX_DEPTH,
        )
        .unwrap();
        assert!(absent.exact_match.is_none());
        assert!(absent.mismatch.is_none());
        assert!(absent.partial_match.is_none());

        let empty = parse_rights_record(
            format!("{root_start}<UPLNA_SHODA/><NESHODA/><CASTECNA_SHODA/>{root_end}").as_bytes(),
            MAX_NODES,
            MAX_DEPTH,
        )
        .unwrap();
        assert!(empty.exact_match.as_ref().unwrap().ownership.is_none());
        assert!(empty.mismatch.as_ref().unwrap().text.is_none());
        assert!(empty.partial_match.as_ref().unwrap().details.is_none());
    }

    #[test]
    fn raw_unknown_codes_and_decimal_errors_are_preserved_or_bounded() {
        let unknown = rights_xml().replace(
            "<ODPOVIDA_IDENTIFIKACI>a</ODPOVIDA_IDENTIFIKACI>",
            "<ODPOVIDA_IDENTIFIKACI>future-raw</ODPOVIDA_IDENTIFIKACI>",
        );
        let report = parse_rights_record(unknown.as_bytes(), MAX_NODES, MAX_DEPTH).unwrap();
        let value = &report
            .exact_match
            .unwrap()
            .ownership
            .unwrap()
            .identification_match;
        assert_eq!(
            value,
            &RightsIdentificationMatch::Other("future-raw".to_owned())
        );
        assert_eq!(value.as_str(), "future-raw");

        let unknown_protection = properties_xml().replace(
            "<OCHRANA>a</OCHRANA>",
            "<OCHRANA>future-protection</OCHRANA>",
        );
        let properties = parse_ownership_overview_with_properties(
            unknown_protection.as_bytes(),
            MAX_NODES,
            MAX_DEPTH,
        )
        .unwrap();
        assert!(matches!(
            properties.districts[0].municipalities[0].cadastral_areas[0].records[0].parcels[0]
                .protection,
            YesNo::Other(ref raw) if raw == "future-protection"
        ));

        let raw = "999999999999999999999999999999";
        let oversized = overview_xml().replace(
            "<citatel>-7</citatel>",
            &format!("<citatel>{raw}</citatel>"),
        );
        let error =
            parse_ownership_overview(oversized.as_bytes(), MAX_NODES, MAX_DEPTH).unwrap_err();
        assert!(matches!(error, Error::DecimalOutOfRange { .. }));
        assert!(!error.to_string().contains(raw));

        let coded = LegalRelation {
            code: Some(crate::LegalRelationCode::Other("future-code".to_owned())),
            name: "name is not classified".to_owned(),
        };
        assert_eq!(coded.is_ownership(), None);
    }

    #[test]
    fn rejects_dtd_roots_namespaces_limits_and_cardinality_violations() {
        assert!(parse_ownership_overview(b"<Other/>", MAX_NODES, MAX_DEPTH).is_err());
        assert!(
            parse_ownership_overview_with_properties(b"<Other/>", MAX_NODES, MAX_DEPTH).is_err()
        );
        assert!(parse_rights_record(b"<Other/>", MAX_NODES, MAX_DEPTH).is_err());
        assert!(
            parse_rights_record(
                b"<EvidencePravProOsobu xmlns='urn:wrong'/>",
                MAX_NODES,
                MAX_DEPTH
            )
            .is_err()
        );
        assert!(parse_ownership_overview(b"\xff", MAX_NODES, MAX_DEPTH).is_err());
        assert!(
            parse_ownership_overview(
                format!(
                    "<!DOCTYPE x [<!ENTITY e SYSTEM 'file:///etc/passwd'>]>{}",
                    overview_xml()
                )
                .as_bytes(),
                MAX_NODES,
                MAX_DEPTH
            )
            .is_err()
        );
        assert!(parse_ownership_overview(overview_xml().as_bytes(), 1, MAX_DEPTH).is_err());
        assert!(parse_ownership_overview(overview_xml().as_bytes(), MAX_NODES, 1).is_err());

        let duplicate = overview_xml().replace(
            "<CP_OMEZENI>territory</CP_OMEZENI>",
            "<CP_OMEZENI>territory</CP_OMEZENI><CP_OMEZENI>duplicate</CP_OMEZENI>",
        );
        assert!(parse_ownership_overview(duplicate.as_bytes(), MAX_NODES, MAX_DEPTH).is_err());
        let missing_required = properties_xml().replace("<INFO_VYSTUP/>", "");
        assert!(
            parse_ownership_overview_with_properties(
                missing_required.as_bytes(),
                MAX_NODES,
                MAX_DEPTH
            )
            .is_err()
        );
        let mut empty_required = overview_xml().to_owned();
        let start = empty_required.find("<TELESA>").unwrap();
        let end = empty_required.find("</TELESA>").unwrap() + "</TELESA>".len();
        empty_required.replace_range(start..end, "<TELESA/>");
        assert!(parse_ownership_overview(empty_required.as_bytes(), MAX_NODES, MAX_DEPTH).is_err());
        let empty_rights_ident = rights_xml().replace("<IDENT><id>121</id><rc6>123456</rc6><rc7>1234567</rc7><ico>00000123</ico><ico_doplnek>suffix</ico_doplnek><bsm1_id>11</bsm1_id><bsm2_id>12</bsm2_id></IDENT>", "<IDENT/>");
        assert!(parse_rights_record(empty_rights_ident.as_bytes(), MAX_NODES, MAX_DEPTH).is_ok());
        let incomplete_right = properties_xml().replace("<PLATNOST_DO>until-one</PLATNOST_DO>", "");
        assert!(
            parse_ownership_overview_with_properties(
                incomplete_right.as_bytes(),
                MAX_NODES,
                MAX_DEPTH
            )
            .is_err()
        );
        let out_of_order = overview_xml().replace(
            "<LV>sheet-one</LV><PRSV_IDENT>",
            "<PRSV_IDENT><parcela/></PRSV_IDENT><LV>sheet-one</LV><PRSV_IDENT>",
        );
        assert!(parse_ownership_overview(out_of_order.as_bytes(), MAX_NODES, MAX_DEPTH).is_err());
    }

    #[test]
    fn overview_empty_identification_and_empty_relation_list_follow_xsd() {
        let xml = overview_xml()
            .replace("<OPSUB_IDENT><id>101</id><rc6>123456</rc6><rc7>1234567</rc7><ico>00000123</ico><ico_doplnek>suffix</ico_doplnek><bsm1_id>11</bsm1_id><bsm2_id>12</bsm2_id><id/></OPSUB_IDENT>", "<OPSUB_IDENT/>")
            .replace("<TYPY_PRAV_VZTAHU>\n                  <TYP_PRAV_VZTAHU><TYPRAV_NAZEV>relation-name</TYPRAV_NAZEV><TELESA>\n                    <TELESO><LV>sheet-one</LV><PRSV_IDENT><parcela><id>102</id><zkratka>pkn</zkratka><druh_cis>numbering</druh_cis><par_cis> 12 </par_cis><podd_cis>3</podd_cis><dil_parcely>-128</dil_parcely><tel_id>103</tel_id><cislo_tel>parcel-sheet</cislo_tel><ku_nazev>area</ku_nazev><nazev_puv>original-area</nazev_puv></parcela><stavba><id>104</id></stavba><jednotka><id>105</id></jednotka><pravo_stavby><id>106</id></pravo_stavby></PRSV_IDENT><PODIL><citatel>-7</citatel><jmenovatel>0</jmenovatel></PODIL><TEL_ID>103</TEL_ID><JPV_LV>relation-sheet</JPV_LV><RIZENI><typriz_kod>V</typriz_kod><poradove_cislo>4294967295</poradove_cislo><rok>65535</rok><prares_kod>65535</prares_kod></RIZENI></TELESO>\n                    <TELESO><LV/><PRSV_IDENT/><PODIL/><TEL_ID/><JPV_LV/><RIZENI/></TELESO>\n                  </TELESA></TYP_PRAV_VZTAHU>\n                </TYPY_PRAV_VZTAHU>", "<TYPY_PRAV_VZTAHU/>");
        let report = parse_ownership_overview(xml.as_bytes(), MAX_NODES, MAX_DEPTH).unwrap();
        assert!(report.subject.identification.entries.is_empty());
        assert!(
            report.districts[0].municipalities[0].cadastral_areas[0]
                .legal_relations
                .is_empty()
        );
    }
}
