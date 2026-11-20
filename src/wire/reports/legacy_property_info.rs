//! Bounded decoding of legacy parcel- and building-information reports.

use std::str::FromStr;

use roxmltree::{Document, Node, ParsingOptions};

use crate::LegalRelation;
use crate::error::{Error, Result};
use crate::model::reports::legacy_base::*;
use crate::model::reports::legacy_property_info::*;
use crate::wire::reports::legacy_base::*;

const XSI_NS: &str = "http://www.w3.org/2001/XMLSchema-instance";

/// Parses a complete legacy parcel-information report with explicit XML limits.
pub(crate) fn parse_parcel_info(
    bytes: &[u8],
    max_nodes: u32,
    max_depth: u32,
) -> Result<ParcelInfoReport> {
    let xml = std::str::from_utf8(bytes)
        .map_err(|_| Error::invalid_response("parcel information is not UTF-8"))?;
    let document = parse_document(xml, "InformaceOParcelach", max_nodes, max_depth)?;
    let root = document.root_element();
    const FIELDS: &[&str] = &[
        "KATASTRY",
        "PLATNOST",
        "VYHOTOVENO",
        "CS_POCET_PARCEL1",
        "CS_POCET_PARCEL",
        "VYHOTOVIL",
        "BEZPL_PRISTUP",
        "INFO_VYSTUP",
    ];
    let mut cursor = Cursor::sequence(root, FIELDS)?;
    let mut cadastre = None;
    let mut cadastres_seen = false;
    let mut valid_at = None;
    let mut created_at = None;
    let mut cadastral_parcel_count = None;
    let mut simplified_parcel_count = None;
    let mut created_by = None;
    let mut free_access = None;
    let mut output_info = None;
    for child in elements(root) {
        match cursor.singleton(child)? {
            0 => {
                cadastres_seen = true;
                cadastre = parse_cadastres(child)?;
            }
            1 => valid_at = Some(text(child)?),
            2 => created_at = Some(text(child)?),
            3 => cadastral_parcel_count = Some(text(child)?),
            4 => simplified_parcel_count = Some(text(child)?),
            5 => created_by = Some(text(child)?),
            6 => free_access = Some(text(child)?),
            7 => output_info = Some(text(child)?),
            _ => unreachable!(),
        }
    }
    if !cadastres_seen {
        return Err(missing("KATASTRY"));
    }
    Ok(ParcelInfoReport {
        cadastre,
        valid_at: required(valid_at, "PLATNOST")?,
        created_at: required(created_at, "VYHOTOVENO")?,
        cadastral_parcel_count: required(cadastral_parcel_count, "CS_POCET_PARCEL1")?,
        simplified_parcel_count: required(simplified_parcel_count, "CS_POCET_PARCEL")?,
        created_by: required(created_by, "VYHOTOVIL")?,
        free_access: required(free_access, "BEZPL_PRISTUP")?,
        output_info,
    })
}

/// Parses a complete legacy building-information report with explicit XML limits.
pub(crate) fn parse_building_info(
    bytes: &[u8],
    max_nodes: u32,
    max_depth: u32,
) -> Result<BuildingInfoReport> {
    let xml = std::str::from_utf8(bytes)
        .map_err(|_| Error::invalid_response("building information is not UTF-8"))?;
    let document = parse_document(xml, "InformaceOStavbach", max_nodes, max_depth)?;
    let root = document.root_element();
    const FIELDS: &[&str] = &[
        "REPORT",
        "CS_POCET_STAVEB",
        "PLATNOST",
        "VYHOTOVIL",
        "BEZPL_PRISTUP",
        "VYHOTOVENO",
        "INFO_VYSTUP",
    ];
    let mut cursor = Cursor::sequence(root, FIELDS)?;
    let mut report = None;
    let mut building_count = None;
    let mut valid_at = None;
    let mut created_by = None;
    let mut free_access = None;
    let mut created_at = None;
    let mut output_info = None;
    for child in elements(root) {
        match cursor.singleton(child)? {
            0 => report = Some(parse_building_report(child)?),
            1 => building_count = Some(text(child)?),
            2 => valid_at = Some(text(child)?),
            3 => created_by = Some(text(child)?),
            4 => free_access = Some(text(child)?),
            5 => created_at = Some(text(child)?),
            6 => output_info = Some(text(child)?),
            _ => unreachable!(),
        }
    }
    Ok(BuildingInfoReport {
        report: required(report, "REPORT")?,
        building_count,
        valid_at: required(valid_at, "PLATNOST")?,
        created_by: required(created_by, "VYHOTOVIL")?,
        free_access: required(free_access, "BEZPL_PRISTUP")?,
        created_at: required(created_at, "VYHOTOVENO")?,
        output_info,
    })
}

fn parse_document<'a>(
    xml: &'a str,
    expected_root: &str,
    max_nodes: u32,
    max_depth: u32,
) -> Result<Document<'a>> {
    let document = Document::parse_with_options(
        xml,
        ParsingOptions {
            allow_dtd: false,
            nodes_limit: max_nodes,
            ..ParsingOptions::default()
        },
    )
    .map_err(|_| Error::invalid_response("malformed legacy property-information XML"))?;
    let root = document.root_element();
    check_depth(root, 1, max_depth)?;
    require_element(root)?;
    if root.tag_name().name() != expected_root {
        return Err(Error::invalid_response(format!(
            "unexpected legacy property-information root; expected {expected_root}"
        )));
    }
    Ok(document)
}

fn parse_cadastres(node: Node<'_, '_>) -> Result<Option<ParcelCadastre>> {
    let mut cursor = Cursor::sequence(node, &["KATASTR"])?;
    let mut cadastre = None;
    for child in elements(node) {
        cursor.singleton(child)?;
        cadastre = Some(parse_parcel_cadastre(child)?);
    }
    Ok(cadastre)
}

fn parse_parcel_cadastre(node: Node<'_, '_>) -> Result<ParcelCadastre> {
    const FIELDS: &[&str] = &["OKRES", "OBEC", "KATUZE", "PARCELY_KN", "ZDROJE"];
    let mut cursor = Cursor::sequence(node, FIELDS)?;
    let mut district = None;
    let mut municipality = None;
    let mut cadastral_area = None;
    let mut cadastral_parcels = None;
    let mut source = None;
    for child in elements(node) {
        match cursor.singleton(child)? {
            0 => district = Some(parse_district(child)?),
            1 => municipality = Some(parse_municipality(child)?),
            2 => cadastral_area = Some(parse_parcel_cadastral_area(child)?),
            3 => {
                cadastral_parcels = Some(parse_list(child, "PARCELA_KN", parse_cadastral_parcel)?);
            }
            4 => source = Some(parse_sources(child)?),
            _ => unreachable!(),
        }
    }
    Ok(ParcelCadastre {
        district: required(district, "OKRES")?,
        municipality: required(municipality, "OBEC")?,
        cadastral_area: required(cadastral_area, "KATUZE")?,
        cadastral_parcels: required(cadastral_parcels, "PARCELY_KN")?,
        source: required(source, "ZDROJE")?,
    })
}

fn parse_district(node: Node<'_, '_>) -> Result<PropertyDistrict> {
    let mut cursor = Cursor::sequence(node, &["nuts4", "nazev"])?;
    let mut district = PropertyDistrict::default();
    for child in elements(node) {
        match cursor.singleton(child)? {
            0 => district.nuts4 = Some(text(child)?),
            1 => district.name = Some(text(child)?),
            _ => unreachable!(),
        }
    }
    Ok(district)
}

fn parse_municipality(node: Node<'_, '_>) -> Result<PropertyMunicipality> {
    let (code, name) = parse_required_code_name(node)?;
    Ok(PropertyMunicipality { code, name })
}

fn parse_required_code_name(node: Node<'_, '_>) -> Result<(u32, String)> {
    let mut cursor = Cursor::sequence(node, &["kod", "nazev"])?;
    let mut code = None;
    let mut name = None;
    for child in elements(node) {
        match cursor.singleton(child)? {
            0 => code = Some(integer(child, "kod")?),
            1 => name = Some(text(child)?),
            _ => unreachable!(),
        }
    }
    Ok((required(code, "kod")?, required(name, "nazev")?))
}

fn parse_parcel_cadastral_area(node: Node<'_, '_>) -> Result<ParcelCadastralArea> {
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
    Ok(ParcelCadastralArea {
        details: if has_children {
            Some(ParcelCadastralAreaDetails {
                code: required(code, "KATUZE/kod")?,
                name: required(name, "KATUZE/nazev")?,
            })
        } else {
            None
        },
    })
}

fn parse_cadastral_parcel(node: Node<'_, '_>) -> Result<CadastralParcel> {
    const FIELDS: &[&str] = &[
        "POCET_1",
        "DIL",
        "PARCELNI_CISLO",
        "VYMERA_PARCELY",
        "URC_VYMERY",
        "DRUH_POZEMKU",
        "ZP_VYUZITI",
        "OCHRANA",
        "MAPA",
        "RIZENI",
        "KATUZE_KOD",
        "ZPURVY_KOD",
        "SOUCASTI_STAVBA",
        "STAVBA_NA_PARCELE",
        "PARCELY_OMEZENI",
        "VLASTNICTVI",
        "PLOMBY_RIZENI_KN",
        "BONITY_KN",
    ];
    let mut cursor = Cursor::all(node, FIELDS)?;
    let mut sequence_number = None;
    let mut part = None;
    let mut identification = None;
    let mut area = None;
    let mut area_determination = None;
    let mut land_kind = None;
    let mut use_description = None;
    let mut protections = None;
    let mut map = None;
    let mut proceeding = None;
    let mut cadastral_area_code = None;
    let mut primary_use = None;
    let mut component_building = None;
    let mut building_on_parcel = None;
    let mut restriction = None;
    let mut ownership = None;
    let mut seals = None;
    let mut bonities = None;
    for child in elements(node) {
        match cursor.singleton(child)? {
            0 => sequence_number = Some(integer(child, "POCET_1")?),
            1 => part = Some(text(child)?),
            2 => identification = Some(parse_optional_parcel(child)?),
            3 => area = Some(integer(child, "VYMERA_PARCELY")?),
            4 => area_determination = Some(text(child)?),
            5 => land_kind = Some(text(child)?),
            6 => use_description = Some(text(child)?),
            7 => protections = Some(parse_protections(child)?),
            8 => map = Some(text(child)?),
            9 => proceeding = Some(parse_proceeding(child)?),
            10 => cadastral_area_code = Some(integer(child, "KATUZE_KOD")?),
            11 => primary_use = Some(parse_primary_use(child)?),
            12 => component_building = Some(parse_component_building(child)?),
            13 => building_on_parcel = Some(parse_building_wrapper(child)?),
            14 => restriction = Some(parse_restriction(child)?),
            15 => ownership = Some(parse_ownership(child, true, true)?),
            16 => seals = Some(parse_list(child, "PLOMBA_RIZENI_KN", parse_cadastral_seal)?),
            17 => bonities = Some(parse_list(child, "BDP_KN", parse_cadastral_bonity)?),
            _ => unreachable!(),
        }
    }
    Ok(CadastralParcel {
        sequence_number,
        part: required(part, "DIL")?,
        identification: required(identification, "PARCELNI_CISLO")?,
        area: required(area, "VYMERA_PARCELY")?,
        area_determination: required(area_determination, "URC_VYMERY")?,
        land_kind: required(land_kind, "DRUH_POZEMKU")?,
        use_description,
        protections: required(protections, "OCHRANA")?,
        map: required(map, "MAPA")?,
        proceeding: required(proceeding, "RIZENI")?,
        cadastral_area_code: required(cadastral_area_code, "KATUZE_KOD")?,
        primary_use: required(primary_use, "ZPURVY_KOD")?,
        component_building: required(component_building, "SOUCASTI_STAVBA")?,
        building_on_parcel,
        restriction: required(restriction, "PARCELY_OMEZENI")?,
        ownership: required(ownership, "VLASTNICTVI")?,
        seals: required(seals, "PLOMBY_RIZENI_KN")?,
        bonities: required(bonities, "BONITY_KN")?,
    })
}

fn parse_optional_parcel(node: Node<'_, '_>) -> Result<Option<Parcel>> {
    let mut cursor = Cursor::sequence(node, &["parcela"])?;
    let mut parcel = None;
    for child in elements(node) {
        cursor.singleton(child)?;
        parcel = Some(parse_parcel(child)?);
    }
    Ok(parcel)
}

fn parse_primary_use(node: Node<'_, '_>) -> Result<ParcelPrimaryUseCode> {
    let raw = text(node)?;
    if raw.is_empty() {
        return Ok(ParcelPrimaryUseCode::Empty);
    }
    integer_node_text(&raw, "ZPURVY_KOD").map(ParcelPrimaryUseCode::Code)
}

fn parse_component_building(node: Node<'_, '_>) -> Result<Option<ParcelComponentBuilding>> {
    let mut cursor = Cursor::sequence(node, &["stavba", "docasna"])?;
    let mut building = None;
    let mut temporary = None;
    let mut has_children = false;
    for child in elements(node) {
        has_children = true;
        match cursor.singleton(child)? {
            0 => building = Some(parse_building(child)?),
            1 => temporary = Some(text(child)?),
            _ => unreachable!(),
        }
    }
    if !has_children {
        return Ok(None);
    }
    Ok(Some(ParcelComponentBuilding {
        building: required(building, "SOUCASTI_STAVBA/stavba")?,
        temporary,
    }))
}

fn parse_protections(node: Node<'_, '_>) -> Result<Vec<String>> {
    let mut cursor = Cursor::sequence(node, &["zpochr"])?;
    let mut values = Vec::new();
    for child in elements(node) {
        cursor.repeated(child)?;
        values.push(text(child)?);
    }
    Ok(values)
}

fn parse_restriction(node: Node<'_, '_>) -> Result<Option<YesNo>> {
    let mut cursor = Cursor::sequence(node, &["PARCELA_OMEZENI"])?;
    let mut restriction = None;
    for child in elements(node) {
        cursor.singleton(child)?;
        let mut inner = Cursor::sequence(child, &["OMEZENI"])?;
        let mut value = None;
        for grandchild in elements(child) {
            inner.singleton(grandchild)?;
            value = Some(parse_yes_no(grandchild)?);
        }
        restriction = Some(required(value, "OMEZENI")?);
    }
    Ok(restriction)
}

fn parse_ownership(
    node: Node<'_, '_>,
    sheet_fields_required: bool,
    repeated_additional_identifications: bool,
) -> Result<PropertyOwnership> {
    let mut cursor = Cursor::sequence(node, &["LV", "SUBJEKTY"])?;
    let mut sheet = None;
    let mut subjects = None;
    let mut has_children = false;
    for child in elements(node) {
        has_children = true;
        match cursor.singleton(child)? {
            0 => sheet = Some(parse_ownership_sheet(child, sheet_fields_required)?),
            1 => {
                subjects = Some(parse_nonempty_list(child, "SUBJEKT", |subject| {
                    parse_ownership_subject(subject, repeated_additional_identifications)
                })?);
            }
            _ => unreachable!(),
        }
    }
    if !has_children {
        return Ok(PropertyOwnership::default());
    }
    Ok(PropertyOwnership {
        details: Some(PropertyOwnershipDetails {
            sheet: required(sheet, "VLASTNICTVI/LV")?,
            subjects: required(subjects, "VLASTNICTVI/SUBJEKTY")?,
        }),
    })
}

fn parse_ownership_sheet(
    node: Node<'_, '_>,
    fields_required: bool,
) -> Result<PropertyOwnershipSheet> {
    let mut cursor = Cursor::sequence(node, &["tel_id", "cislo_tel"])?;
    let mut sheet = PropertyOwnershipSheet::default();
    let mut has_children = false;
    for child in elements(node) {
        has_children = true;
        match cursor.singleton(child)? {
            0 => sheet.id = Some(parse_legacy_id(child)?),
            1 => sheet.number = Some(text(child)?),
            _ => unreachable!(),
        }
    }
    if fields_required && has_children && (sheet.id.is_none() || sheet.number.is_none()) {
        return Err(invalid(
            "LV",
            "tel_id and cislo_tel are required together in this report",
        ));
    }
    Ok(sheet)
}

fn parse_ownership_subject(
    node: Node<'_, '_>,
    repeated_additional_identifications: bool,
) -> Result<PropertyOwnershipSubject> {
    const FIELDS: &[&str] = &["VLA_IDENT", "PODIL", "TYPRAV_NAZEV", "CHAROS", "VLA_IDENT2"];
    let mut cursor = Cursor::sequence(node, FIELDS)?;
    let mut identification = None;
    let mut share = None;
    let mut legal_relation = None;
    let mut characteristic = None;
    let mut additional_identifications = Vec::new();
    for child in elements(node) {
        let index = if child.tag_name().name() == "VLA_IDENT2" {
            cursor.repeated(child)?
        } else {
            cursor.singleton(child)?
        };
        match index {
            0 => identification = Some(parse_ownership_identification(child)?),
            1 => share = Some(parse_reported_share(child)?),
            2 => {
                legal_relation = Some(LegalRelation {
                    code: None,
                    name: text(child)?,
                });
            }
            3 => characteristic = Some(parse_characteristic_wrapper(child)?),
            4 => {
                if !repeated_additional_identifications && !additional_identifications.is_empty() {
                    return Err(invalid(
                        "VLA_IDENT2",
                        "singleton element occurs more than once",
                    ));
                }
                additional_identifications.push(parse_ownership_identification(child)?);
            }
            _ => unreachable!(),
        }
    }
    if additional_identifications.is_empty() {
        return Err(missing("VLA_IDENT2"));
    }
    Ok(PropertyOwnershipSubject {
        identification: required(identification, "VLA_IDENT")?,
        share: required(share, "PODIL")?,
        legal_relation: required(legal_relation, "TYPRAV_NAZEV")?,
        characteristic: required(characteristic, "CHAROS")?,
        additional_identifications,
    })
}

fn parse_cadastral_seal(node: Node<'_, '_>) -> Result<CadastralParcelSeal> {
    let mut cursor = Cursor::sequence(node, &["PAR_ID3", "PLOMBA_RIZENI_KN"])?;
    let mut parcel_id3 = None;
    let mut proceeding = None;
    let mut has_children = false;
    for child in elements(node) {
        has_children = true;
        match cursor.singleton(child)? {
            0 => parcel_id3 = Some(integer(child, "PAR_ID3")?),
            1 => proceeding = Some(parse_proceeding(child)?),
            _ => unreachable!(),
        }
    }
    if has_children && proceeding.is_none() {
        return Err(missing("PLOMBA_RIZENI_KN"));
    }
    Ok(CadastralParcelSeal {
        parcel_id3,
        proceeding,
    })
}

fn parse_cadastral_bonity(node: Node<'_, '_>) -> Result<CadastralBonity> {
    let mut cursor = Cursor::sequence(node, &["BPEJ_KOD", "VYMERA"])?;
    let mut code = None;
    let mut area = None;
    for child in elements(node) {
        match cursor.singleton(child)? {
            0 => code = Some(text(child)?),
            1 => area = Some(integer(child, "VYMERA")?),
            _ => unreachable!(),
        }
    }
    Ok(CadastralBonity {
        code: required(code, "BPEJ_KOD")?,
        area: required(area, "VYMERA")?,
    })
}

fn parse_sources(node: Node<'_, '_>) -> Result<Option<ParcelSource>> {
    let mut cursor = Cursor::sequence(node, &["ZDROJ"])?;
    let mut source = None;
    for child in elements(node) {
        cursor.singleton(child)?;
        source = Some(parse_parcel_source(child)?);
    }
    Ok(source)
}

fn parse_parcel_source(node: Node<'_, '_>) -> Result<ParcelSource> {
    let mut cursor = Cursor::sequence(node, &["ZE_KOD", "PUVOD", "PARCELY_ZE"])?;
    let mut source_code = None;
    let mut origin = None;
    let mut parcels = None;
    for child in elements(node) {
        match cursor.singleton(child)? {
            0 => source_code = Some(integer(child, "ZE_KOD")?),
            1 => origin = Some(text(child)?),
            2 => parcels = Some(parse_list(child, "PARCELA_ZE", parse_simplified_parcel)?),
            _ => unreachable!(),
        }
    }
    Ok(ParcelSource {
        source_code: required(source_code, "ZE_KOD")?,
        origin: required(origin, "PUVOD")?,
        parcels: required(parcels, "PARCELY_ZE")?,
    })
}

fn parse_simplified_parcel(node: Node<'_, '_>) -> Result<SimplifiedParcel> {
    const FIELDS: &[&str] = &[
        "KATUZE_KOD",
        "KATUZE_KOD_PUV",
        "PARCELNI_CISLO",
        "DIL_PARCELY",
        "PUVODNI_KU",
        "TYP",
        "PARCELA_KN",
        "VYMERA_PARCELY",
        "RIZENI",
        "PLOMBY_RIZENI_ZE",
        "PARCELY_OMEZENI",
        "VLASTNICTVI",
        "BONITY_ZE",
    ];
    let mut cursor = Cursor::all(node, FIELDS)?;
    let mut cadastral_area_code = None;
    let mut original_cadastral_area_code = None;
    let mut identification = None;
    let mut part = None;
    let mut original_cadastral_area = None;
    let mut kind = None;
    let mut cadastral_parcel = None;
    let mut area = None;
    let mut proceeding = None;
    let mut seals = None;
    let mut restriction = None;
    let mut ownership = None;
    let mut bonities = None;
    for child in elements(node) {
        match cursor.singleton(child)? {
            0 => cadastral_area_code = Some(integer(child, "KATUZE_KOD")?),
            1 => original_cadastral_area_code = Some(text(child)?),
            2 => identification = Some(parse_optional_parcel(child)?),
            3 => part = Some(text(child)?),
            4 => original_cadastral_area = Some(parse_original_area(child)?),
            5 => kind = Some(text(child)?),
            6 => cadastral_parcel = Some(parse_optional_parcel(child)?),
            7 => area = Some(integer(child, "VYMERA_PARCELY")?),
            8 => proceeding = Some(parse_proceeding(child)?),
            9 => {
                seals = Some(parse_list(
                    child,
                    "PLOMBA_RIZENI_ZE",
                    parse_simplified_seal,
                )?)
            }
            10 => restriction = Some(parse_restriction(child)?),
            11 => ownership = Some(parse_ownership(child, true, true)?),
            12 => bonities = Some(parse_list(child, "BDP_ZE", parse_simplified_bonity)?),
            _ => unreachable!(),
        }
    }
    Ok(SimplifiedParcel {
        cadastral_area_code: required(cadastral_area_code, "KATUZE_KOD")?,
        original_cadastral_area_code: required(original_cadastral_area_code, "KATUZE_KOD_PUV")?,
        identification: required(identification, "PARCELNI_CISLO")?,
        part: required(part, "DIL_PARCELY")?,
        original_cadastral_area: required(original_cadastral_area, "PUVODNI_KU")?,
        kind: required(kind, "TYP")?,
        cadastral_parcel: required(cadastral_parcel, "PARCELA_KN")?,
        area: required(area, "VYMERA_PARCELY")?,
        proceeding: required(proceeding, "RIZENI")?,
        seals: required(seals, "PLOMBY_RIZENI_ZE")?,
        restriction: required(restriction, "PARCELY_OMEZENI")?,
        ownership: required(ownership, "VLASTNICTVI")?,
        bonities: required(bonities, "BONITY_ZE")?,
    })
}

fn parse_original_area(node: Node<'_, '_>) -> Result<Option<ParcelCadastralAreaDetails>> {
    let area = parse_parcel_cadastral_area(node)?;
    Ok(area.details)
}

fn parse_simplified_seal(node: Node<'_, '_>) -> Result<SimplifiedParcelSeal> {
    let mut cursor = Cursor::sequence(node, &["PAR_ID3", "PLOMBA_RIZENI_ZE"])?;
    let mut parcel_id3 = None;
    let mut proceedings = Vec::new();
    for child in elements(node) {
        let index = if child.tag_name().name() == "PLOMBA_RIZENI_ZE" {
            cursor.repeated(child)?
        } else {
            cursor.singleton(child)?
        };
        match index {
            0 => parcel_id3 = Some(integer(child, "PAR_ID3")?),
            1 => proceedings.push(parse_proceeding(child)?),
            _ => unreachable!(),
        }
    }
    if parcel_id3.is_some() && proceedings.is_empty() {
        return Err(missing("PLOMBA_RIZENI_ZE"));
    }
    Ok(SimplifiedParcelSeal {
        parcel_id3,
        proceedings,
    })
}

fn parse_simplified_bonity(node: Node<'_, '_>) -> Result<SimplifiedBonity> {
    let mut cursor = Cursor::sequence(node, &["BPEJ_KOD", "VYMERA"])?;
    let mut code = None;
    let mut area = None;
    for child in elements(node) {
        match cursor.singleton(child)? {
            0 => code = Some(integer(child, "BPEJ_KOD")?),
            1 => area = Some(integer(child, "VYMERA")?),
            _ => unreachable!(),
        }
    }
    Ok(SimplifiedBonity {
        code: required(code, "BPEJ_KOD")?,
        area: required(area, "VYMERA")?,
    })
}

fn parse_building_report(node: Node<'_, '_>) -> Result<BuildingReport> {
    let mut cursor = Cursor::sequence(node, &["OKRES", "OBEC", "KATUZE", "STAVBY"])?;
    let mut district = None;
    let mut municipality = None;
    let mut cadastral_area = None;
    let mut buildings = None;
    for child in elements(node) {
        match cursor.singleton(child)? {
            0 => district = Some(parse_district(child)?),
            1 => municipality = Some(parse_municipality(child)?),
            2 => {
                let (code, name) = parse_required_code_name(child)?;
                cadastral_area = Some(BuildingCadastralArea { code, name });
            }
            3 => buildings = Some(parse_list(child, "STAVBA", parse_building_info_entry)?),
            _ => unreachable!(),
        }
    }
    Ok(BuildingReport {
        district: required(district, "OKRES")?,
        municipality: required(municipality, "OBEC")?,
        cadastral_area: required(cadastral_area, "KATUZE")?,
        buildings: required(buildings, "STAVBY")?,
    })
}

fn parse_building_info_entry(node: Node<'_, '_>) -> Result<BuildingInfo> {
    const FIELDS: &[&str] = &[
        "POCET_STAVEB",
        "TYP_KOD",
        "CISLO_DOMOVNI",
        "ZP_VYUZ",
        "CP_CE",
        "TYP_STAVBY",
        "ADRESA_STAVBY",
        "OCHRANA",
        "RIZENI",
        "POPIS_STAVBY",
        "DALSI_UDAJE",
        "DOCASNA_STAVBA",
        "LIST_G_VAZBA_JPV",
        "BUD_NA_PARCELE",
        "VAZBA_JPV",
        "PARCELY",
        "VLASTNICTVI",
        "PLOMBY_STAV",
    ];
    let mut cursor = Cursor::all(node, FIELDS)?;
    let mut construction_count = None;
    let mut type_code = None;
    let mut house_number = None;
    let mut use_description = None;
    let mut identification = None;
    let mut type_description = None;
    let mut addresses = None;
    let mut protections = None;
    let mut proceeding = None;
    let mut description = None;
    let mut additional_details = None;
    let mut temporary = None;
    let mut legal_relation_links = None;
    let mut parcels_beneath = None;
    let mut legal_relation_text = None;
    let mut parcel_groups = None;
    let mut ownership = None;
    let mut seals = None;
    for child in elements(node) {
        match cursor.singleton(child)? {
            0 => construction_count = Some(integer(child, "POCET_STAVEB")?),
            1 => type_code = Some(integer(child, "TYP_KOD")?),
            2 => house_number = Some(text(child)?),
            3 => use_description = Some(text(child)?),
            4 => identification = Some(parse_optional_building(child)?),
            5 => type_description = Some(text(child)?),
            6 => addresses = Some(parse_building_addresses(child)?),
            7 => protections = Some(parse_protections(child)?),
            8 => proceeding = Some(parse_proceeding(child)?),
            9 => description = Some(text(child)?),
            10 => additional_details = Some(parse_building_parts(child)?),
            11 => temporary = Some(text(child)?),
            12 => {
                legal_relation_links = Some(Some(parse_list(
                    child,
                    "G_VAZBA_JPV",
                    parse_building_relation_link,
                )?));
            }
            13 => parcels_beneath = Some(parse_parcels_beneath(child)?),
            14 => legal_relation_text = Some(Some(parse_relation_text(child)?)),
            15 => parcel_groups = Some(parse_building_parcel_groups(child)?),
            16 => ownership = Some(parse_ownership(child, false, false)?),
            17 => seals = Some(Some(parse_list(child, "PLOMBA_RIZENI", parse_proceeding)?)),
            _ => unreachable!(),
        }
    }
    Ok(BuildingInfo {
        construction_count,
        type_code: required(type_code, "TYP_KOD")?,
        house_number: required(house_number, "CISLO_DOMOVNI")?,
        use_description: required(use_description, "ZP_VYUZ")?,
        identification: required(identification, "CP_CE")?,
        type_description: required(type_description, "TYP_STAVBY")?,
        addresses: required(addresses, "ADRESA_STAVBY")?,
        protections: required(protections, "OCHRANA")?,
        proceeding: required(proceeding, "RIZENI")?,
        description: required(description, "POPIS_STAVBY")?,
        additional_details: required(additional_details, "DALSI_UDAJE")?,
        temporary,
        legal_relation_links: legal_relation_links.unwrap_or(None),
        parcels_beneath: required(parcels_beneath, "BUD_NA_PARCELE")?,
        legal_relation_text: legal_relation_text.unwrap_or(None),
        parcel_groups: required(parcel_groups, "PARCELY")?,
        ownership: required(ownership, "VLASTNICTVI")?,
        seals: seals.unwrap_or(None),
    })
}

fn parse_optional_building(node: Node<'_, '_>) -> Result<Option<Building>> {
    let mut cursor = Cursor::sequence(node, &["stavba"])?;
    let mut building = None;
    for child in elements(node) {
        cursor.singleton(child)?;
        building = Some(parse_building(child)?);
    }
    Ok(building)
}

fn parse_building_addresses(node: Node<'_, '_>) -> Result<Vec<BuildingAddressGroup>> {
    const FIELDS: &[&str] = &["adresni_misto", "ulice", "cdomu", "co"];
    require_element(node)?;
    let mut groups = Vec::new();
    let mut previous = None;
    for child in elements(node) {
        let index = field_index(child, FIELDS)?;
        if previous.is_none_or(|previous| index <= previous) {
            groups.push(BuildingAddressGroup::default());
        }
        previous = Some(index);
        let group = groups
            .last_mut()
            .expect("an address group was just inserted");
        match index {
            0 => group.address_point_id = Some(parse_legacy_id(child)?),
            1 => group.street = Some(text(child)?),
            2 => group.house_number = Some(text(child)?),
            3 => group.orientation_number = Some(text(child)?),
            _ => unreachable!(),
        }
    }
    Ok(groups)
}

fn parse_building_parts(node: Node<'_, '_>) -> Result<Vec<BuildingPart>> {
    require_element(node)?;
    elements(node)
        .map(|child| {
            require_element(child)?;
            match child.tag_name().name() {
                "stavba_soucasti_pozemku" => parse_part_of_parcel(child)
                    .map(Box::new)
                    .map(BuildingPart::Parcel),
                "stavba_soucasti_prava_stavby" => parse_part_of_right(child)
                    .map(Box::new)
                    .map(BuildingPart::BuildingRight),
                name => Err(unexpected(name)),
            }
        })
        .collect()
}

fn parse_part_of_parcel(node: Node<'_, '_>) -> Result<BuildingPartOfParcel> {
    let mut cursor = Cursor::sequence(node, &["parcela", "tel_id", "cislo_tel"])?;
    let mut value = BuildingPartOfParcel::default();
    for child in elements(node) {
        match cursor.singleton(child)? {
            0 => value.parcel = Some(parse_parcel(child)?),
            1 => value.ownership_sheet_id = Some(parse_legacy_id(child)?),
            2 => value.ownership_sheet_number = Some(text(child)?),
            _ => unreachable!(),
        }
    }
    Ok(value)
}

fn parse_part_of_right(node: Node<'_, '_>) -> Result<BuildingPartOfRight> {
    let mut cursor = Cursor::sequence(node, &["tel_id", "cislo_tel", "ku_nazev"])?;
    let mut value = BuildingPartOfRight::default();
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

fn parse_building_relation_link(node: Node<'_, '_>) -> Result<BuildingLegalRelationLink> {
    let mut cursor = Cursor::sequence(node, &["BUD_ID3", "T_JPV"])?;
    let mut link = BuildingLegalRelationLink::default();
    for child in elements(node) {
        match cursor.singleton(child)? {
            0 => link.building_id3 = Some(text(child)?),
            1 => link.relation_type = Some(text(child)?),
            _ => unreachable!(),
        }
    }
    Ok(link)
}

fn parse_parcels_beneath(node: Node<'_, '_>) -> Result<Vec<Parcel>> {
    parse_nonempty_list(node, "NA_PARCELE", |entry| {
        let mut cursor = Cursor::sequence(entry, &["PARCELNI_CISLO"])?;
        let mut parcel = None;
        for child in elements(entry) {
            cursor.singleton(child)?;
            parcel = Some(required_parcel(child)?);
        }
        required(parcel, "PARCELNI_CISLO")
    })
}

fn required_parcel(node: Node<'_, '_>) -> Result<Parcel> {
    let mut cursor = Cursor::sequence(node, &["parcela"])?;
    let mut parcel = None;
    for child in elements(node) {
        cursor.singleton(child)?;
        parcel = Some(parse_parcel(child)?);
    }
    required(parcel, "parcela")
}

fn parse_relation_text(node: Node<'_, '_>) -> Result<BuildingLegalRelationText> {
    let mut cursor = Cursor::sequence(node, &["TEXT"])?;
    let mut value = BuildingLegalRelationText::default();
    for child in elements(node) {
        cursor.singleton(child)?;
        value.text = Some(text(child)?);
    }
    Ok(value)
}

fn parse_building_parcel_groups(node: Node<'_, '_>) -> Result<Vec<BuildingParcelGroup>> {
    require_element(node)?;
    let mut groups: Vec<BuildingParcelGroup> = Vec::new();
    for child in elements(node) {
        require_element(child)?;
        match child.tag_name().name() {
            "PARCELA" => groups.push(BuildingParcelGroup {
                parcel: parse_building_parcel(child)?,
                ownership_sheet: None,
                cadastral_area_name: None,
            }),
            "LV_PARCELY" => {
                let group = groups
                    .last_mut()
                    .ok_or_else(|| invalid("LV_PARCELY", "must follow PARCELA"))?;
                if group.ownership_sheet.is_some() || group.cadastral_area_name.is_some() {
                    return Err(invalid(
                        "LV_PARCELY",
                        "element is duplicated or out of order",
                    ));
                }
                let sheet = parse_building_parcel_sheet(child)?;
                if sheet.details.is_none() {
                    return Err(missing("LV_PARCELY/C_TEL"));
                }
                group.ownership_sheet = Some(sheet);
            }
            "LV_KATUZE" => {
                let group = groups
                    .last_mut()
                    .ok_or_else(|| invalid("LV_KATUZE", "must follow PARCELA"))?;
                if group.cadastral_area_name.is_some() {
                    return Err(invalid("LV_KATUZE", "element occurs more than once"));
                }
                group.cadastral_area_name = Some(text(child)?);
            }
            name => return Err(unexpected(name)),
        }
    }
    Ok(groups)
}

fn parse_building_parcel(node: Node<'_, '_>) -> Result<BuildingParcel> {
    const FIELDS: &[&str] = &[
        "PARCELNI_CISLO",
        "PLOMBY_PAR",
        "LV_PARCELY",
        "LV_KATUZE",
        "VLASTNICTVI",
    ];
    let mut cursor = Cursor::all(node, FIELDS)?;
    let mut identification = None;
    let mut seals = None;
    let mut inner_ownership_sheet = None;
    let mut cadastral_area = None;
    let mut ownership = None;
    for child in elements(node) {
        match cursor.singleton(child)? {
            0 => identification = Some(required_parcel(child)?),
            1 => seals = Some(parse_list(child, "PLOMBA_RIZENI", parse_proceeding)?),
            2 => inner_ownership_sheet = Some(parse_building_parcel_sheet(child)?),
            3 => cadastral_area = Some(parse_building_parcel_area(child)?),
            4 => ownership = Some(parse_ownership(child, false, false)?),
            _ => unreachable!(),
        }
    }
    Ok(BuildingParcel {
        identification: required(identification, "PARCELNI_CISLO")?,
        seals: required(seals, "PLOMBY_PAR")?,
        inner_ownership_sheet,
        cadastral_area,
        ownership: required(ownership, "VLASTNICTVI")?,
    })
}

fn parse_building_parcel_sheet(node: Node<'_, '_>) -> Result<BuildingParcelSheet> {
    let mut cursor = Cursor::sequence(node, &["C_TEL", "OS_PARCELY"])?;
    let mut sheet = None;
    let mut owners = None;
    let mut has_children = false;
    for child in elements(node) {
        has_children = true;
        match cursor.singleton(child)? {
            0 => sheet = Some(parse_building_parcel_sheet_id(child)?),
            1 => owners = Some(parse_list(child, "OPSUB", parse_building_parcel_owner)?),
            _ => unreachable!(),
        }
    }
    if !has_children {
        return Ok(BuildingParcelSheet::default());
    }
    let owners = required(owners, "OS_PARCELY")?;
    if owners.is_empty() {
        return Err(invalid("OS_PARCELY", "at least one OPSUB is required"));
    }
    Ok(BuildingParcelSheet {
        details: Some(BuildingParcelSheetDetails {
            sheet: required(sheet, "C_TEL")?,
            owners,
        }),
    })
}

fn parse_building_parcel_sheet_id(node: Node<'_, '_>) -> Result<BuildingParcelSheetId> {
    let mut cursor = Cursor::sequence(node, &["tel_id", "cislo_tel"])?;
    let mut id = None;
    let mut number = None;
    for child in elements(node) {
        match cursor.singleton(child)? {
            0 => id = Some(parse_legacy_id(child)?),
            1 => number = Some(integer(child, "cislo_tel")?),
            _ => unreachable!(),
        }
    }
    Ok(BuildingParcelSheetId {
        id: required(id, "C_TEL/tel_id")?,
        number: required(number, "C_TEL/cislo_tel")?,
    })
}

fn parse_building_parcel_owner(node: Node<'_, '_>) -> Result<BuildingParcelOwner> {
    const FIELDS: &[&str] = &[
        "OPSUB_NAZEV",
        "OPSUB_ADRESA",
        "CHAROS",
        "PODIL",
        "TYPRAV_NAZEV",
    ];
    let mut cursor = Cursor::sequence(node, FIELDS)?;
    let mut name = None;
    let mut addresses = None;
    let mut characteristic = None;
    let mut share = None;
    let mut legal_relation = None;
    for child in elements(node) {
        match cursor.singleton(child)? {
            0 => name = Some(parse_building_owner_name(child)?),
            1 => addresses = Some(parse_list(child, "adresa", parse_address)?),
            2 => characteristic = Some(parse_characteristic_wrapper(child)?),
            3 => share = Some(parse_reported_share(child)?),
            4 => {
                legal_relation = Some(LegalRelation {
                    code: None,
                    name: text(child)?,
                });
            }
            _ => unreachable!(),
        }
    }
    let addresses = required(addresses, "OPSUB_ADRESA")?;
    if addresses.is_empty() {
        return Err(invalid("OPSUB_ADRESA", "at least one adresa is required"));
    }
    Ok(BuildingParcelOwner {
        name: required(name, "OPSUB_NAZEV")?,
        addresses,
        characteristic: required(characteristic, "CHAROS")?,
        share: required(share, "PODIL")?,
        legal_relation: required(legal_relation, "TYPRAV_NAZEV")?,
    })
}

fn parse_building_owner_name(node: Node<'_, '_>) -> Result<SubjectNameForm> {
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
    for child in elements(node) {
        match cursor.singleton(child)? {
            0 => value.id = Some(parse_legacy_id(child)?),
            1 => value.partner1_id = Some(parse_legacy_id(child)?),
            2 => value.partner2_id = Some(parse_legacy_id(child)?),
            3 => value.person.surname = Some(text(child)?),
            4 => value.person.given_name = Some(text(child)?),
            5 => value.person.title_before = Some(text(child)?),
            6 => value.person.title_after = Some(text(child)?),
            7 => value.partner1.surname = Some(text(child)?),
            8 => value.partner1.given_name = Some(text(child)?),
            9 => value.partner1.title_before = Some(text(child)?),
            10 => value.partner1.title_after = Some(text(child)?),
            11 => value.partner2.surname = Some(text(child)?),
            12 => value.partner2.given_name = Some(text(child)?),
            13 => value.partner2.title_before = Some(text(child)?),
            14 => value.partner2.title_after = Some(text(child)?),
            15 => value.name = Some(text(child)?),
            _ => unreachable!(),
        }
    }
    Ok(value)
}

fn parse_building_parcel_area(node: Node<'_, '_>) -> Result<BuildingParcelCadastralArea> {
    let mut cursor = Cursor::sequence(node, &["LV_KATUZE_KOD", "LV_KATUZE_NAZEV"])?;
    let mut code = None;
    let mut name = None;
    let mut has_children = false;
    for child in elements(node) {
        has_children = true;
        match cursor.singleton(child)? {
            0 => code = Some(integer(child, "LV_KATUZE_KOD")?),
            1 => name = Some(text(child)?),
            _ => unreachable!(),
        }
    }
    Ok(BuildingParcelCadastralArea {
        details: if has_children {
            Some(BuildingParcelCadastralAreaDetails {
                code: required(code, "LV_KATUZE_KOD")?,
                name: required(name, "LV_KATUZE_NAZEV")?,
            })
        } else {
            None
        },
    })
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
    integer_node_text(&text(node)?, field)
}

fn integer_node_text<T: FromStr>(raw: &str, field: &'static str) -> Result<T> {
    trim_xml_whitespace(raw)
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
            "legacy property information exceeds XML depth limit",
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
            "legacy property-information elements must have no XML namespace",
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
    Error::invalid_response(format!(
        "unexpected legacy property-information element {name}"
    ))
}

fn invalid(field: &str, message: &str) -> Error {
    Error::invalid_response(format!(
        "invalid legacy property-information element {field}: {message}"
    ))
}

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;

    use super::*;

    const LIMIT: u32 = 100_000;
    const DEPTH: u32 = 64;

    fn parcel_document() -> String {
        let long_id = "9".repeat(80);
        format!(
            r#"<InformaceOParcelach><KATASTRY><KATASTR>
<OKRES><nuts4>CZ0100</nuts4><nazev>District</nazev></OKRES><OBEC><kod>4294967295</kod><nazev>Municipality</nazev></OBEC><KATUZE><kod>700001</kod><nazev>Area</nazev></KATUZE>
<PARCELY_KN><PARCELA_KN><POCET_1>7</POCET_1><DIL>part</DIL><PARCELNI_CISLO><parcela><id>{long_id}</id><zkratka>kn</zkratka><druh_cis>raw-kind</druh_cis><par_cis> 12 </par_cis><podd_cis>3</podd_cis><dil_parcely>-128</dil_parcely><tel_id/><cislo_tel>99</cislo_tel><ku_nazev>Area</ku_nazev><nazev_puv>Old area</nazev_puv></parcela></PARCELNI_CISLO><VYMERA_PARCELY>1234</VYMERA_PARCELY><URC_VYMERY>map</URC_VYMERY><DRUH_POZEMKU>land</DRUH_POZEMKU><ZP_VYUZITI>use</ZP_VYUZITI><OCHRANA><zpochr>one</zpochr><zpochr>two</zpochr></OCHRANA><MAPA>map 1</MAPA><RIZENI><typriz_kod>raw</typriz_kod><poradove_cislo>4294967295</poradove_cislo><rok>65535</rok><prares_kod>65535</prares_kod></RIZENI><KATUZE_KOD>700001</KATUZE_KOD><ZPURVY_KOD></ZPURVY_KOD><SOUCASTI_STAVBA><stavba><id>55</id><docasna>raw</docasna></stavba><docasna>n</docasna></SOUCASTI_STAVBA><STAVBA_NA_PARCELE/><PARCELY_OMEZENI><PARCELA_OMEZENI><OMEZENI>future</OMEZENI></PARCELA_OMEZENI></PARCELY_OMEZENI><VLASTNICTVI><LV><tel_id>{long_id}</tel_id><cislo_tel>99</cislo_tel></LV><SUBJEKTY><SUBJEKT><VLA_IDENT><parcela><id/></parcela><oprav_subjekt><id>8</id><nazev>Subject</nazev></oprav_subjekt><oprav_subjekt/></VLA_IDENT><PODIL><citatel>-2</citatel><jmenovatel>0</jmenovatel></PODIL><TYPRAV_NAZEV>Vlastnické právo</TYPRAV_NAZEV><CHAROS><char_os><kod>99</kod><zkratka>raw</zkratka><zkratka_alv>raw-alv</zkratka_alv></char_os></CHAROS><VLA_IDENT2/><VLA_IDENT2><stavba><id>3</id></stavba></VLA_IDENT2></SUBJEKT></SUBJEKTY></VLASTNICTVI><PLOMBY_RIZENI_KN><PLOMBA_RIZENI_KN/><PLOMBA_RIZENI_KN><PAR_ID3>18446744073709551615</PAR_ID3><PLOMBA_RIZENI_KN/></PLOMBA_RIZENI_KN></PLOMBY_RIZENI_KN><BONITY_KN><BDP_KN><BPEJ_KOD>raw-bpej</BPEJ_KOD><VYMERA>12</VYMERA></BDP_KN></BONITY_KN></PARCELA_KN></PARCELY_KN>
<ZDROJE><ZDROJ><ZE_KOD>4</ZE_KOD><PUVOD>origin</PUVOD><PARCELY_ZE><PARCELA_ZE><KATUZE_KOD>700001</KATUZE_KOD><KATUZE_KOD_PUV> 000raw </KATUZE_KOD_PUV><PARCELNI_CISLO/><DIL_PARCELY>ze-part</DIL_PARCELY><PUVODNI_KU><kod>6</kod><nazev>Old</nazev></PUVODNI_KU><TYP>raw-type</TYP><PARCELA_KN><parcela><id>10</id></parcela></PARCELA_KN><VYMERA_PARCELY>20</VYMERA_PARCELY><RIZENI/><PLOMBY_RIZENI_ZE><PLOMBA_RIZENI_ZE><PAR_ID3>9</PAR_ID3><PLOMBA_RIZENI_ZE/><PLOMBA_RIZENI_ZE><typriz_kod>V</typriz_kod><poradove_cislo>1</poradove_cislo><rok>2020</rok><prares_kod>100</prares_kod></PLOMBA_RIZENI_ZE></PLOMBA_RIZENI_ZE><PLOMBA_RIZENI_ZE/></PLOMBY_RIZENI_ZE><PARCELY_OMEZENI/><VLASTNICTVI/><BONITY_ZE><BDP_ZE><BPEJ_KOD>8</BPEJ_KOD><VYMERA>30</VYMERA></BDP_ZE></BONITY_ZE></PARCELA_ZE></PARCELY_ZE></ZDROJ></ZDROJE>
</KATASTR></KATASTRY><PLATNOST> valid </PLATNOST><VYHOTOVENO>created</VYHOTOVENO><CS_POCET_PARCEL1>raw 1</CS_POCET_PARCEL1><CS_POCET_PARCEL>raw 2</CS_POCET_PARCEL><VYHOTOVIL>user</VYHOTOVIL><BEZPL_PRISTUP>free</BEZPL_PRISTUP><INFO_VYSTUP>info</INFO_VYSTUP></InformaceOParcelach>"#
        )
    }

    fn ownership_subject() -> &'static str {
        "<SUBJEKT><VLA_IDENT/><PODIL/><TYPRAV_NAZEV>Raw relation</TYPRAV_NAZEV><CHAROS/><VLA_IDENT2/></SUBJEKT>"
    }

    fn parcel_sheet() -> String {
        "<LV_PARCELY><C_TEL><tel_id>88</tel_id><cislo_tel>99</cislo_tel></C_TEL><OS_PARCELY><OPSUB><OPSUB_NAZEV><id>7</id><prijmeni>Surname</prijmeni><nazev>Name last</nazev></OPSUB_NAZEV><OPSUB_ADRESA><adresa><nazev_ulice>Street</nazev_ulice></adresa></OPSUB_ADRESA><CHAROS/><PODIL><citatel>1</citatel></PODIL><TYPRAV_NAZEV>Relation</TYPRAV_NAZEV></OPSUB></OS_PARCELY></LV_PARCELY>".to_owned()
    }

    fn building_document() -> String {
        let inner_sheet = parcel_sheet();
        let outer_sheet = parcel_sheet().replace("88", "188");
        format!(
            r#"<InformaceOStavbach><REPORT><OKRES><nazev>District</nazev></OKRES><OBEC><kod>1</kod><nazev>Municipality</nazev></OBEC><KATUZE><kod>2</kod><nazev>Area</nazev></KATUZE><STAVBY><STAVBA><POCET_STAVEB>3</POCET_STAVEB><TYP_KOD>255</TYP_KOD><CISLO_DOMOVNI> 12 </CISLO_DOMOVNI><ZP_VYUZ>use</ZP_VYUZ><CP_CE><stavba><id>4</id><par_ident><parcela/><parcela><id>5</id></parcela></par_ident></stavba></CP_CE><TYP_STAVBY>type</TYP_STAVBY><ADRESA_STAVBY><adresni_misto/><ulice>One</ulice><cdomu>1</cdomu><co>2</co><adresni_misto>9</adresni_misto><ulice>Two</ulice></ADRESA_STAVBY><OCHRANA><zpochr>protection</zpochr></OCHRANA><RIZENI/><POPIS_STAVBY>description</POPIS_STAVBY><DALSI_UDAJE><stavba_soucasti_pozemku><parcela><id>6</id></parcela><tel_id>7</tel_id><cislo_tel>8</cislo_tel></stavba_soucasti_pozemku><stavba_soucasti_prava_stavby><tel_id/><cislo_tel>9</cislo_tel><ku_nazev>Right area</ku_nazev></stavba_soucasti_prava_stavby><stavba_soucasti_pozemku/></DALSI_UDAJE><DOCASNA_STAVBA>raw</DOCASNA_STAVBA><LIST_G_VAZBA_JPV><G_VAZBA_JPV/><G_VAZBA_JPV><BUD_ID3>raw-id</BUD_ID3><T_JPV>raw-relation</T_JPV></G_VAZBA_JPV></LIST_G_VAZBA_JPV><BUD_NA_PARCELE><NA_PARCELE><PARCELNI_CISLO><parcela><id>10</id></parcela></PARCELNI_CISLO></NA_PARCELE></BUD_NA_PARCELE><VAZBA_JPV><TEXT>link text</TEXT></VAZBA_JPV><PARCELY><PARCELA><PARCELNI_CISLO><parcela><id>11</id></parcela></PARCELNI_CISLO><PLOMBY_PAR><PLOMBA_RIZENI/><PLOMBA_RIZENI/></PLOMBY_PAR><LV_PARCELY>{inner_sheet_inner}</LV_PARCELY><LV_KATUZE><LV_KATUZE_KOD>12</LV_KATUZE_KOD><LV_KATUZE_NAZEV>Inner area</LV_KATUZE_NAZEV></LV_KATUZE><VLASTNICTVI><LV><tel_id>13</tel_id></LV><SUBJEKTY>{subject}</SUBJEKTY></VLASTNICTVI></PARCELA><LV_KATUZE>Outer area</LV_KATUZE><PARCELA><PARCELNI_CISLO><parcela><id>14</id></parcela></PARCELNI_CISLO><PLOMBY_PAR/><VLASTNICTVI/></PARCELA>{outer_sheet}<LV_KATUZE></LV_KATUZE></PARCELY><VLASTNICTVI><LV/><SUBJEKTY>{subject}</SUBJEKTY></VLASTNICTVI><PLOMBY_STAV/></STAVBA></STAVBY></REPORT><CS_POCET_STAVEB>raw count</CS_POCET_STAVEB><PLATNOST>valid</PLATNOST><VYHOTOVIL>user</VYHOTOVIL><BEZPL_PRISTUP></BEZPL_PRISTUP><VYHOTOVENO>created</VYHOTOVENO><INFO_VYSTUP>info</INFO_VYSTUP></InformaceOStavbach>"#,
            inner_sheet_inner = inner_sheet
                .strip_prefix("<LV_PARCELY>")
                .unwrap()
                .strip_suffix("</LV_PARCELY>")
                .unwrap(),
            outer_sheet = outer_sheet,
            subject = ownership_subject(),
        )
    }

    #[test]
    fn parcel_report_covers_kn_and_simplified_fields_and_choices() {
        let report = parse_parcel_info(parcel_document().as_bytes(), LIMIT, DEPTH).unwrap();
        let cadastre = report.cadastre.unwrap();
        assert_eq!(cadastre.municipality.code, u32::MAX);
        assert_eq!(cadastre.cadastral_area.details.unwrap().name, "Area");
        let parcel = &cadastre.cadastral_parcels[0];
        assert_eq!(parcel.sequence_number, Some(7));
        assert_eq!(parcel.identification.as_ref().unwrap().part, Some(-128));
        assert_eq!(
            parcel
                .identification
                .as_ref()
                .unwrap()
                .id
                .as_ref()
                .unwrap()
                .as_str()
                .len(),
            80
        );
        assert_eq!(parcel.primary_use, ParcelPrimaryUseCode::Empty);
        assert_eq!(parcel.protections, ["one", "two"]);
        assert_eq!(parcel.building_on_parcel, Some(BuildingWrapper::default()));
        assert_eq!(parcel.restriction, Some(YesNo::Other("future".to_owned())));
        let ownership = parcel.ownership.details.as_ref().unwrap();
        assert_eq!(ownership.subjects.len(), 1);
        let subject = &ownership.subjects[0];
        assert_eq!(subject.identification.targets.len(), 3);
        assert_eq!(subject.additional_identifications.len(), 2);
        assert_eq!(subject.share.numerator, Some(Decimal::new(-2, 0)));
        assert_eq!(subject.share.denominator, Some(Decimal::ZERO));
        assert_eq!(subject.legal_relation.code, None);
        assert_eq!(subject.legal_relation.is_ownership(), None);
        assert_eq!(parcel.seals[1].parcel_id3, Some(u64::MAX));
        assert!(parcel.seals[0].proceeding.is_none());
        let source = cadastre.source.unwrap();
        let simplified = &source.parcels[0];
        assert_eq!(simplified.original_cadastral_area_code, " 000raw ");
        assert_eq!(simplified.original_cadastral_area.as_ref().unwrap().code, 6);
        assert!(simplified.identification.is_none());
        assert_eq!(simplified.seals[0].proceedings.len(), 2);
        assert!(simplified.seals[1].proceedings.is_empty());
        assert!(simplified.ownership.details.is_none());
        assert_eq!(simplified.bonities[0].code, 8);
        assert_eq!(report.valid_at, " valid ");

        let empty = "<InformaceOParcelach><KATASTRY/><PLATNOST/><VYHOTOVENO/><CS_POCET_PARCEL1/><CS_POCET_PARCEL/><VYHOTOVIL/><BEZPL_PRISTUP/></InformaceOParcelach>";
        assert!(
            parse_parcel_info(empty.as_bytes(), LIMIT, DEPTH)
                .unwrap()
                .cadastre
                .is_none()
        );
    }

    #[test]
    fn building_report_covers_addresses_parts_and_both_parcel_sheet_levels() {
        let report = parse_building_info(building_document().as_bytes(), LIMIT, DEPTH).unwrap();
        let building = &report.report.buildings[0];
        assert_eq!(building.type_code, u8::MAX);
        assert_eq!(building.addresses.len(), 2);
        assert_eq!(
            building.addresses[0]
                .address_point_id
                .as_ref()
                .unwrap()
                .as_str(),
            ""
        );
        assert_eq!(building.addresses[1].street.as_deref(), Some("Two"));
        assert_eq!(building.additional_details.len(), 3);
        assert!(matches!(
            building.additional_details[0],
            BuildingPart::Parcel(_)
        ));
        assert!(matches!(
            building.additional_details[1],
            BuildingPart::BuildingRight(_)
        ));
        assert_eq!(building.legal_relation_links.as_ref().unwrap().len(), 2);
        assert_eq!(building.parcels_beneath.len(), 1);
        assert_eq!(
            building
                .legal_relation_text
                .as_ref()
                .unwrap()
                .text
                .as_deref(),
            Some("link text")
        );
        assert_eq!(building.parcel_groups.len(), 2);
        let first = &building.parcel_groups[0].parcel;
        let sheet = first
            .inner_ownership_sheet
            .as_ref()
            .unwrap()
            .details
            .as_ref()
            .unwrap();
        assert_eq!(
            sheet.owners[0].name.person.surname.as_deref(),
            Some("Surname")
        );
        assert_eq!(sheet.owners[0].name.name.as_deref(), Some("Name last"));
        assert_eq!(sheet.owners[0].addresses.len(), 1);
        assert_eq!(
            first
                .cadastral_area
                .as_ref()
                .unwrap()
                .details
                .as_ref()
                .unwrap()
                .code,
            12
        );
        assert_eq!(
            building.parcel_groups[0].cadastral_area_name.as_deref(),
            Some("Outer area")
        );
        assert!(
            building.parcel_groups[1]
                .ownership_sheet
                .as_ref()
                .unwrap()
                .details
                .is_some()
        );
        assert_eq!(building.seals, Some(Vec::new()));
        let relation = &building.ownership.details.as_ref().unwrap().subjects[0].legal_relation;
        assert_eq!(relation.code, None);
        assert_eq!(relation.is_ownership(), None);
    }

    #[test]
    fn rejects_bad_limits_roots_duplicates_and_invalid_numbers() {
        let parcel = parcel_document();
        assert!(parse_parcel_info(b"<!DOCTYPE x><InformaceOParcelach/>", LIMIT, DEPTH).is_err());
        assert!(parse_parcel_info(b"<Other/>", LIMIT, DEPTH).is_err());
        assert!(parse_parcel_info(parcel.as_bytes(), 1, DEPTH).is_err());
        assert!(parse_parcel_info(parcel.as_bytes(), LIMIT, 1).is_err());
        let duplicate =
            parcel.replacen("<PLATNOST> valid </PLATNOST>", "<PLATNOST/><PLATNOST/>", 1);
        assert!(parse_parcel_info(duplicate.as_bytes(), LIMIT, DEPTH).is_err());
        let duplicate_dil = parcel.replacen("<DIL>part</DIL>", "<DIL/><DIL/>", 1);
        assert!(parse_parcel_info(duplicate_dil.as_bytes(), LIMIT, DEPTH).is_err());
        for raw in ["256", "-1", "1.0"] {
            let invalid = parcel.replace(
                "<ZPURVY_KOD></ZPURVY_KOD>",
                &format!("<ZPURVY_KOD>{raw}</ZPURVY_KOD>"),
            );
            assert!(parse_parcel_info(invalid.as_bytes(), LIMIT, DEPTH).is_err());
        }

        let building = building_document();
        assert!(parse_building_info(b"<InformaceOParcelach/>", LIMIT, DEPTH).is_err());
        assert!(parse_building_info(building.as_bytes(), 1, DEPTH).is_err());
        assert!(parse_building_info(building.as_bytes(), LIMIT, 1).is_err());
        let duplicate =
            building.replacen("<PLATNOST>valid</PLATNOST>", "<PLATNOST/><PLATNOST/>", 1);
        assert!(parse_building_info(duplicate.as_bytes(), LIMIT, DEPTH).is_err());
        let duplicate_vla = building.replace("<VLA_IDENT2/>", "<VLA_IDENT2/><VLA_IDENT2/>");
        assert!(parse_building_info(duplicate_vla.as_bytes(), LIMIT, DEPTH).is_err());
    }
}
