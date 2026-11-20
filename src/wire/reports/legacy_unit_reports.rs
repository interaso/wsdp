//! Bounded, strict decoding of legacy unit, building-right, and proceeding XML.

use std::str::FromStr;

use quick_xml::events::{BytesStart, Event};
use roxmltree::{Document, Node, ParsingOptions};

use super::legacy_base as base;
use crate::error::{Error, Result};
use crate::model::reports::legacy_unit_reports::*;

const XSI_NS: &str = "http://www.w3.org/2001/XMLSchema-instance";

/// Parses a complete unit-information 1.0 document with explicit XML limits.
pub(crate) fn parse_unit_info(
    bytes: &[u8],
    max_nodes: u32,
    max_depth: u32,
) -> Result<UnitInfoReport> {
    parse_report(
        bytes,
        max_nodes,
        max_depth,
        "InformaceOJednotkach",
        |root, _| decode_unit_report(root),
    )
}

/// Parses a complete building-right-information 1.0 document with explicit XML limits.
pub(crate) fn parse_building_right_info(
    bytes: &[u8],
    max_nodes: u32,
    max_depth: u32,
) -> Result<BuildingRightInfoReport> {
    parse_report(
        bytes,
        max_nodes,
        max_depth,
        "InformaceOPravechStavby",
        |root, _| decode_building_right_report(root),
    )
}

/// Parses a complete proceeding-information 1.0 document with explicit XML limits.
pub(crate) fn parse_proceeding_info(
    bytes: &[u8],
    max_nodes: u32,
    max_depth: u32,
) -> Result<ProceedingInfoReport> {
    parse_report(
        bytes,
        max_nodes,
        max_depth,
        "InformaceORizeni",
        decode_proceeding_report,
    )
}

fn parse_report<T>(
    bytes: &[u8],
    max_nodes: u32,
    max_depth: u32,
    expected_root: &'static str,
    decode: impl FnOnce(Node<'_, '_>, &str) -> Result<T>,
) -> Result<T> {
    let xml = std::str::from_utf8(bytes)
        .map_err(|_| Error::invalid_response("legacy report is not UTF-8 XML"))?;
    preflight(xml, max_nodes, max_depth)?;
    let document = Document::parse_with_options(
        xml,
        ParsingOptions {
            allow_dtd: false,
            nodes_limit: max_nodes,
            ..ParsingOptions::default()
        },
    )
    .map_err(|_| Error::invalid_response("malformed legacy report XML"))?;
    let root = document.root_element();
    require_name(root, expected_root)?;
    reject_nil(root)?;
    decode(root, xml)
}

fn decode_unit_report(node: Node<'_, '_>) -> Result<UnitInfoReport> {
    const FIELDS: &[&str] = &[
        "KATASTR",
        "CS_POCET_JEDNOTEK",
        "PLATNOST",
        "VYHOTOVIL",
        "BEZPL_PRISTUP",
        "VYHOTOVENO",
        "INFO_VYSTUP",
    ];
    let fields = FieldSet::sequence(node, FIELDS)?;
    Ok(UnitInfoReport {
        cadastral_area: fields.parse_required(0, decode_unit_cadastral_area)?,
        total_units: fields.parse_required(1, integer)?,
        valid_at: fields.parse_required(2, simple_text)?,
        created_by: fields.parse_required(3, simple_text)?,
        free_access: fields.parse_required(4, simple_text)?,
        created_at: fields.parse_required(5, simple_text)?,
        output_info: fields.parse_optional(6, simple_text)?,
    })
}

fn decode_unit_cadastral_area(node: Node<'_, '_>) -> Result<UnitInfoCadastralArea> {
    const FIELDS: &[&str] = &["OKRES", "OBEC", "KATUZE", "STAVBY"];
    let fields = FieldSet::sequence(node, FIELDS)?;
    Ok(UnitInfoCadastralArea {
        text: mixed_text(node)?,
        district: fields.parse_required(0, decode_district)?,
        municipality: fields.parse_required(1, decode_numeric_code_name)?,
        cadastral_area: fields.parse_required(2, decode_numeric_code_name)?,
        buildings: fields.parse_required(3, |node| {
            decode_list(node, "STAVBA", true, decode_unit_building)
        })?,
    })
}

fn decode_district(node: Node<'_, '_>) -> Result<LegacyDistrict> {
    let fields = FieldSet::sequence(node, &["nuts4", "nazev"])?;
    Ok(LegacyDistrict {
        nuts4: fields.parse_optional(0, simple_text)?,
        name: fields.parse_optional(1, simple_text)?,
    })
}

fn decode_numeric_code_name(node: Node<'_, '_>) -> Result<LegacyNumericCodeName> {
    let fields = FieldSet::sequence(node, &["kod", "nazev"])?;
    Ok(LegacyNumericCodeName {
        code: fields.parse_required(0, integer)?,
        name: fields.parse_required(1, simple_text)?,
    })
}

fn decode_text_code_name(node: Node<'_, '_>) -> Result<LegacyTextCodeName> {
    let fields = FieldSet::sequence(node, &["kod", "nazev"])?;
    decode_text_code_name_fields(&fields)
}

fn decode_text_code_name_fields(fields: &FieldSet<'_, '_, '_>) -> Result<LegacyTextCodeName> {
    Ok(LegacyTextCodeName {
        code: fields.parse_required(0, simple_text)?,
        name: fields.parse_required(1, simple_text)?,
    })
}

fn decode_unit_building(node: Node<'_, '_>) -> Result<UnitInfoBuilding> {
    const FIELDS: &[&str] = &[
        "TYPBUD_KOD",
        "CISLO_DOMOVNI",
        "ZPVYBU_KOD",
        "BUDOVA",
        "BUD_PARCELY",
        "JEDNOTKY",
    ];
    let fields = FieldSet::sequence(node, FIELDS)?;
    Ok(UnitInfoBuilding {
        building_type_code: fields.parse_required(0, integer)?,
        house_number: fields.parse_required(1, simple_text)?,
        building_use_code: fields.parse_required(2, simple_text)?,
        building: fields.parse_required(3, |node| {
            optional_nested(node, "stavba", base::parse_building)
        })?,
        parcels: fields.parse_required(4, |node| {
            decode_list(node, "PARCELA", true, decode_unit_building_parcel)
        })?,
        units: fields.parse_required(5, |node| decode_list(node, "JEDNOTKA", true, decode_unit))?,
    })
}

fn decode_unit_building_parcel(node: Node<'_, '_>) -> Result<UnitInfoBuildingParcel> {
    let fields = FieldSet::sequence(node, &["parcela", "tel_id", "cislo_tel"])?;
    Ok(UnitInfoBuildingParcel {
        parcel: fields.parse_optional(0, base::parse_parcel)?,
        ownership_sheet_id: fields.parse_optional(1, base::parse_legacy_id)?,
        ownership_sheet_number: fields.parse_optional(2, simple_text)?,
    })
}

fn decode_unit(node: Node<'_, '_>) -> Result<UnitInfoUnit> {
    const FIELDS: &[&str] = &[
        "POCET_JEDNOTEK",
        "ZP_VYUZITI",
        "CISLO_JED",
        "VYUZ_TYP",
        "BUD_PODIL",
        "RIZENI",
        "PODLAHOVA_PLOCHA",
        "VAZBA_JPV",
        "VLASTNICTVI",
        "PARCELY",
        "PLOMBY",
    ];
    let fields = FieldSet::all(node, FIELDS)?;
    let floor_area = fields.one(6)?.map(decode_floor_area).transpose()?;
    Ok(UnitInfoUnit {
        unit_count: fields.parse_optional(0, integer)?,
        use_code: fields.parse_optional(1, simple_text)?,
        unit: fields.parse_required(2, |node| {
            optional_nested(node, "jednotka", base::parse_unit)
        })?,
        unit_type: fields.parse_required(3, simple_text)?,
        building_share: fields.parse_required(4, base::parse_reported_share)?,
        proceeding: fields.parse_required(5, base::parse_proceeding)?,
        floor_area,
        legal_relation_link: fields.parse_optional(7, decode_unit_legal_relation_link)?,
        ownership: fields.parse_required(8, decode_unit_ownership)?,
        parcels: fields.parse_required(9, |node| {
            decode_list(node, "PARCELA", false, decode_unit_parcel)
        })?,
        seals: fields.parse_required(10, |node| {
            decode_list(node, "PLOMBA", false, decode_unit_seal)
        })?,
    })
}

fn decode_floor_area(node: Node<'_, '_>) -> Result<UnitInfoFloorArea> {
    // The union's second branch is exactly the empty string, not whitespace.
    if simple_text(node)?.is_empty() {
        Ok(UnitInfoFloorArea { value: None })
    } else {
        base::parse_number102(node).map(|value| UnitInfoFloorArea { value: Some(value) })
    }
}

fn decode_unit_legal_relation_link(node: Node<'_, '_>) -> Result<UnitInfoLegalRelationLink> {
    let fields = FieldSet::sequence(node, &["TEXT"])?;
    Ok(UnitInfoLegalRelationLink {
        text: fields.parse_optional(0, simple_text)?,
    })
}

fn decode_unit_ownership(node: Node<'_, '_>) -> Result<UnitInfoOwnership> {
    let fields = FieldSet::sequence(node, &["LV", "SUBJEKTY"])?;
    if fields.is_empty() {
        return Ok(UnitInfoOwnership { details: None });
    }
    let ownership_sheet = fields.parse_required(0, |node| {
        let fields = FieldSet::sequence(node, &["tel_id", "cislo_tel"])?;
        if fields.is_empty() {
            return Ok(None);
        }
        Ok(Some(UnitInfoOwnershipSheet {
            id: fields.parse_required(0, base::parse_legacy_id)?,
            number: fields.parse_required(1, simple_text)?,
        }))
    })?;
    let subjects = fields.parse_required(1, |node| {
        decode_list(node, "SUBJEKT", true, decode_unit_subject)
    })?;
    Ok(UnitInfoOwnership {
        details: Some(UnitInfoOwnershipDetails {
            ownership_sheet,
            subjects,
        }),
    })
}

fn decode_unit_subject(node: Node<'_, '_>) -> Result<UnitInfoSubject> {
    const FIELDS: &[&str] = &[
        "VLA_IDENT",
        "PODIL",
        "TYPRAV_NAZEV",
        "CHAROS",
        "G_NAZ_VLA_PS",
    ];
    let fields = FieldSet::sequence(node, FIELDS)?;
    Ok(UnitInfoSubject {
        identification: fields.parse_required(0, base::parse_ownership_identification)?,
        share: fields.parse_required(1, base::parse_reported_share)?,
        legal_relation_name: fields.parse_required(2, simple_text)?,
        characteristic: fields.parse_required(3, base::parse_characteristic_wrapper)?,
        alternate_identification: fields.parse_optional(4, |node| {
            required_nested(node, "VLA_IDENT2", base::parse_ownership_identification)
        })?,
    })
}

fn decode_unit_parcel(node: Node<'_, '_>) -> Result<UnitInfoParcel> {
    const FIELDS: &[&str] = &[
        "PARCELNI_CISLO",
        "PAR_TEL",
        "PAR_TYP_VLA",
        "PAR_PLOMBY_RIZENI",
    ];
    let fields = FieldSet::sequence(node, FIELDS)?;
    Ok(UnitInfoParcel {
        parcel: fields.parse_required(0, |node| {
            optional_nested(node, "parcela", base::parse_parcel)
        })?,
        ownership_sheet: fields.parse_required(1, decode_unit_parcel_ownership_sheet)?,
        ownership_type: fields.parse_required(2, simple_text)?,
        seal_proceedings: fields.parse_required(3, |node| {
            decode_list(node, "PAR_PLOMBA_RIZENI", false, |node| {
                let proceeding = base::parse_proceeding(node)?;
                Ok(UnitInfoParcelSealProceeding {
                    text: mixed_text(node)?,
                    identification: proceeding.identification,
                })
            })
        })?,
    })
}

fn decode_unit_parcel_ownership_sheet(node: Node<'_, '_>) -> Result<UnitInfoParcelOwnershipSheet> {
    let fields = FieldSet::sequence(node, &["tel_id", "cislo_tel"])?;
    Ok(UnitInfoParcelOwnershipSheet {
        id: fields.parse_required(0, base::parse_legacy_id)?,
        number: fields.parse_required(1, integer)?,
    })
}

fn decode_unit_seal(node: Node<'_, '_>) -> Result<UnitInfoSeal> {
    let fields = FieldSet::sequence(node, &["PLOMBA_RIZENI"])?;
    Ok(UnitInfoSeal {
        text: mixed_text(node)?,
        proceedings: fields.parse_vec(0, base::parse_proceeding)?,
    })
}

fn decode_building_right_report(node: Node<'_, '_>) -> Result<BuildingRightInfoReport> {
    const FIELDS: &[&str] = &[
        "ZAHLAVI",
        "BEZPL_PRISTUP",
        "PLATNOST_K",
        "VYHOTOVIL",
        "INFO_VYSTUP",
        "VYHOTOVENO",
    ];
    let fields = FieldSet::all(node, FIELDS)?;
    Ok(BuildingRightInfoReport {
        header: fields.parse_required(0, decode_building_right_header)?,
        free_access: fields.parse_required(1, simple_text)?,
        valid_at: fields.parse_optional(2, simple_text)?,
        created_by: fields.parse_required(3, simple_text)?,
        output_info: fields.parse_optional(4, simple_text)?,
        created_at: fields.parse_required(5, simple_text)?,
    })
}

fn decode_building_right_header(node: Node<'_, '_>) -> Result<BuildingRightInfoHeader> {
    let fields = FieldSet::sequence(node, &["OKRES", "OBEC", "KU", "PRAVA_STAVEB"])?;
    Ok(BuildingRightInfoHeader {
        district: fields.parse_optional(0, decode_district)?,
        municipality: fields.parse_optional(1, decode_text_code_name)?,
        cadastral_area: fields.parse_optional(2, decode_text_code_name)?,
        records: fields.parse_optional(3, |node| {
            decode_list(node, "ZAZNAM", true, decode_building_right_record)
        })?,
    })
}

fn decode_building_right_record(node: Node<'_, '_>) -> Result<BuildingRightInfoRecord> {
    const FIELDS: &[&str] = &[
        "PLATNOST_DO",
        "ZP_OCHRANY",
        "IDENT_RIZENI",
        "UCELY_PRAVA_STAVBY",
        "PS_IDENT",
        "SOUCASTI_STAVBA",
        "LISTY_VLASTNICTVI",
    ];
    let fields = FieldSet::sequence(node, FIELDS)?;
    Ok(BuildingRightInfoRecord {
        valid_until: fields.parse_required(0, simple_text)?,
        protection_methods: fields
            .parse_required(1, |node| decode_list(node, "zpochr", false, simple_text))?,
        proceeding: fields.parse_optional(2, decode_building_right_proceeding)?,
        purposes: fields.parse_optional(3, |node| decode_list(node, "nazev", true, simple_text))?,
        building_right: fields.parse_optional(4, |node| {
            required_nested(node, "pravo_stavby", base::parse_building_right)
        })?,
        building_components: fields.parse_required(5, decode_building_right_components)?,
        ownership_sheets: fields.parse_optional(6, |node| {
            decode_list(node, "LIST_VLASTNICTVI", false, decode_building_right_sheet)
        })?,
    })
}

fn decode_building_right_proceeding(
    node: Node<'_, '_>,
) -> Result<BuildingRightInfoProceedingIdentification> {
    const FIELDS: &[&str] = &["typriz_kod", "poradove_cislo", "rok", "prares_kod"];
    let fields = FieldSet::sequence(node, FIELDS)?;
    Ok(BuildingRightInfoProceedingIdentification {
        type_code: fields.parse_required(0, simple_text)?,
        serial_number: fields.parse_required(1, simple_text)?,
        year: fields.parse_required(2, simple_text)?,
        workplace_code: fields.parse_required(3, simple_text)?,
    })
}

fn decode_building_right_components(node: Node<'_, '_>) -> Result<Vec<BuildingRightInfoComponent>> {
    require_legacy_element(node)?;
    let mut components = Vec::new();
    let mut current: Option<BuildingRightInfoComponent> = None;
    for child in element_children(node) {
        require_legacy_element(child)?;
        match child.tag_name().name() {
            "stavba" => {
                if let Some(component) = current.take() {
                    components.push(component);
                }
                current = Some(BuildingRightInfoComponent {
                    building: base::parse_building(child)?,
                    temporary: None,
                });
            }
            "docasna" => {
                let component = current.as_mut().ok_or_else(|| {
                    invalid_value("docasna", "must follow stavba in its sequence occurrence")
                })?;
                if component.temporary.is_some() {
                    return Err(invalid_value(
                        "docasna",
                        "singleton element occurs more than once in a sequence occurrence",
                    ));
                }
                component.temporary = Some(simple_text(child)?);
            }
            name => return Err(unexpected_field(name)),
        }
    }
    if let Some(component) = current {
        components.push(component);
    }
    Ok(components)
}

fn decode_building_right_sheet(node: Node<'_, '_>) -> Result<BuildingRightInfoOwnershipSheet> {
    let fields = FieldSet::sequence(node, &["LV", "VLASTNICTVI"])?;
    Ok(BuildingRightInfoOwnershipSheet {
        sheet: fields.parse_required(0, |node| {
            let fields = FieldSet::sequence(node, &["tel_id", "cislo_tel"])?;
            Ok(BuildingRightInfoOwnershipSheetReference {
                id: fields.parse_optional(0, base::parse_legacy_id)?,
                number: fields.parse_required(1, simple_text)?,
            })
        })?,
        ownerships: fields.parse_required_vec(1, decode_building_right_ownership)?,
    })
}

fn decode_building_right_ownership(node: Node<'_, '_>) -> Result<BuildingRightInfoOwnership> {
    const FIELDS: &[&str] = &["VLA_IDENT", "PODIL", "CHAROS", "TYP_VZTAHU", "VLA_IDENT2"];
    let fields = FieldSet::sequence(node, FIELDS)?;
    Ok(BuildingRightInfoOwnership {
        identification: fields.parse_required(0, base::parse_ownership_identification)?,
        share: fields.parse_required(1, base::parse_reported_share)?,
        characteristic: fields.parse_required(2, base::parse_characteristic_wrapper)?,
        relationship_type: fields.parse_required(3, simple_text)?,
        alternate_identifications: fields
            .parse_required_vec(4, base::parse_ownership_identification)?,
    })
}

fn decode_proceeding_report(node: Node<'_, '_>, xml: &str) -> Result<ProceedingInfoReport> {
    const FIELDS: &[&str] = &[
        "UCASTNICI",
        "RIZENI",
        "PREDMETY_RIZENI",
        "KATASTRY",
        "OBJEKTY_PARCELY",
        "OBJEKTY_STAVBY",
        "LISTINY",
        "OBJEKTY_JED",
        "GEOM_PLAN",
        "UDALOSTI",
        "VYK_OPERACE",
        "ZAVADY_GP",
        "NAVRHY_ZMEN",
        "OBJEKTY_PRAVA_STAVBY",
        "VYHOTOVIL",
        "BEZPL_PRISTUP",
        "VYHOTOVENO",
        "INFO_VYSTUP",
    ];
    let fields = FieldSet::all(node, FIELDS)?;
    Ok(ProceedingInfoReport {
        participants: fields.parse_required(0, |node| {
            decode_list(node, "UCASTNIK", false, decode_proceeding_participant)
        })?,
        proceeding: fields.parse_required(1, decode_proceeding_details)?,
        subjects: fields.parse_required(2, |node| {
            decode_list(node, "PREDMET", false, decode_proceeding_subject)
        })?,
        cadastral_areas: fields.parse_required(3, |node| {
            decode_list(node, "KATASTR", false, decode_proceeding_cadastral_group)
        })?,
        parcels: fields.parse_required(4, |node| {
            decode_list(node, "OBJEKT_PARCELA", false, decode_proceeding_parcel)
        })?,
        buildings: fields.parse_required(5, |node| {
            decode_list(node, "OBJEKT_STAVBA", false, decode_proceeding_building)
        })?,
        deeds: fields.parse_required(6, |node| {
            decode_list(node, "LISTINA", false, decode_proceeding_deed)
        })?,
        units: fields.parse_required(7, |node| {
            decode_list(node, "OBJEKT_JED", false, decode_proceeding_unit)
        })?,
        geometric_plan: fields.parse_optional(8, decode_proceeding_geometric_plan)?,
        events: fields.parse_required(9, |node| {
            decode_list(node, "UDALOST", false, decode_proceeding_event)
        })?,
        operations: fields.parse_required(10, |node| {
            require_legacy_element(node)?;
            let mut operations = Vec::new();
            for child in element_children(node) {
                require_name(child, "OPERACE")?;
                operations.push(decode_proceeding_operation(child, xml)?);
            }
            Ok(operations)
        })?,
        geometric_plan_defects: fields.parse_required(11, |node| {
            decode_list(node, "ZAVADA_GP", false, decode_proceeding_defect)
        })?,
        change_proposals: fields.parse_required(12, |node| {
            decode_list(node, "NAVRH_ZMEN", false, decode_proceeding_change_proposal)
        })?,
        building_rights: fields.parse_required(13, |node| {
            decode_list(
                node,
                "OBJEKTY_PRAVO_STAVBY",
                false,
                decode_proceeding_building_right,
            )
        })?,
        created_by: fields.parse_required(14, simple_text)?,
        free_access: fields.parse_required(15, simple_text)?,
        created_at: fields.parse_required(16, simple_text)?,
        output_info: fields.parse_optional(17, simple_text)?,
    })
}

fn decode_proceeding_participant(node: Node<'_, '_>) -> Result<ProceedingInfoParticipant> {
    const FIELDS: &[&str] = &["CELE_JMENO", "IDENTIFIKACE", "UCAST_ADRESA", "TYP_UCAST"];
    let fields = FieldSet::sequence(node, FIELDS)?;
    Ok(ProceedingInfoParticipant {
        name: fields.parse_required(0, decode_proceeding_participant_name)?,
        identification: fields.parse_required(1, decode_proceeding_participant_identification)?,
        addresses: fields.parse_required_vec(2, |node| {
            optional_nested(node, "adresa", base::parse_address)
        })?,
        participation_kinds: fields.parse_required_vec(3, |node| {
            let fields = FieldSet::sequence(node, &["TYPUCA_KOD", "TYPUCA_NAZEV"])?;
            if fields.is_empty() {
                return Ok(ProceedingInfoParticipationKind { details: None });
            }
            Ok(ProceedingInfoParticipationKind {
                details: Some(LegacyTextCodeName {
                    code: fields.parse_required(0, simple_text)?,
                    name: fields.parse_required(1, simple_text)?,
                }),
            })
        })?,
    })
}

fn decode_proceeding_participant_name(node: Node<'_, '_>) -> Result<ProceedingInfoParticipantName> {
    const FIELDS: &[&str] = &[
        "prijmeni",
        "jmeno",
        "titul_pred_jmenem",
        "titul_za_jmenem",
        "obchodni_jmeno",
    ];
    let fields = FieldSet::sequence(node, FIELDS)?;
    let organization = fields.parse_optional(4, simple_text)?;
    let person_present = (0..4).any(|index| fields.present(index));
    let value = match (person_present, organization) {
        (false, None) => None,
        (false, Some(organization)) => Some(ProceedingInfoParticipantNameChoice::Organization(
            organization,
        )),
        (true, None) => Some(ProceedingInfoParticipantNameChoice::Person(
            ProceedingInfoPersonName {
                surname: fields.parse_required(0, simple_text)?,
                given_name: fields.parse_required(1, simple_text)?,
                title_before: fields.parse_optional(2, simple_text)?,
                title_after: fields.parse_optional(3, simple_text)?,
            },
        )),
        (true, Some(_)) => {
            return Err(invalid_value(
                "CELE_JMENO",
                "person and organization choices are mutually exclusive",
            ));
        }
    };
    Ok(ProceedingInfoParticipantName { value })
}

fn decode_proceeding_participant_identification(
    node: Node<'_, '_>,
) -> Result<ProceedingInfoParticipantIdentification> {
    require_legacy_element(node)?;
    let mut entries = Vec::new();
    for child in element_children(node) {
        require_legacy_element(child)?;
        let entry = match child.tag_name().name() {
            "id" => ProceedingInfoParticipantIdentificationEntry::Id(base::parse_legacy_id(child)?),
            "rc6" => {
                ProceedingInfoParticipantIdentificationEntry::PersonalNumber6(simple_text(child)?)
            }
            "rc7" => {
                ProceedingInfoParticipantIdentificationEntry::PersonalNumber7(simple_text(child)?)
            }
            "ico" => ProceedingInfoParticipantIdentificationEntry::CompanyId(simple_text(child)?),
            "ico_doplnek" => {
                ProceedingInfoParticipantIdentificationEntry::CompanyIdSuffix(simple_text(child)?)
            }
            "nazev" => ProceedingInfoParticipantIdentificationEntry::Name(simple_text(child)?),
            name => return Err(unexpected_field(name)),
        };
        entries.push(entry);
    }
    Ok(ProceedingInfoParticipantIdentification { entries })
}

fn decode_proceeding_details(node: Node<'_, '_>) -> Result<ProceedingInfoDetails> {
    const FIELDS: &[&str] = &[
        "PRIRAZENO",
        "DATUM_PODANI",
        "OSVOBOZENO",
        "CISLO_RIZENI",
        "PRAC_NAZEV",
        "TYPRIZ_NAZEV",
        "STAV",
        "VYTVORENO",
        "STAV_UHRADY",
    ];
    let fields = FieldSet::sequence(node, FIELDS)?;
    Ok(ProceedingInfoDetails {
        assigned_at: fields.parse_required(0, simple_text)?,
        submitted_at: fields.parse_required(1, simple_text)?,
        exempt: fields.parse_required(2, simple_text)?,
        number: fields.parse_required(3, base::parse_proceeding)?,
        workplace_name: fields.parse_required(4, simple_text)?,
        proceeding_type_name: fields.parse_required(5, simple_text)?,
        state: fields.parse_required(6, simple_text)?,
        created_at: fields.parse_required(7, simple_text)?,
        payment_state: fields.parse_required(8, |node| {
            let fields = FieldSet::sequence(node, &["kod", "nazev"])?;
            if fields.is_empty() {
                return Ok(ProceedingInfoPaymentState { details: None });
            }
            Ok(ProceedingInfoPaymentState {
                details: Some(decode_text_code_name_fields(&fields)?),
            })
        })?,
    })
}

fn decode_proceeding_subject(node: Node<'_, '_>) -> Result<ProceedingInfoSubject> {
    let fields = FieldSet::sequence(node, &["TYPPRE_KOD", "TYPPRE_NAZEV"])?;
    Ok(ProceedingInfoSubject {
        type_code: fields.parse_required(0, simple_text)?,
        type_name: fields.parse_required(1, simple_text)?,
    })
}

fn decode_proceeding_cadastral_group(
    node: Node<'_, '_>,
) -> Result<ProceedingInfoCadastralAreaGroup> {
    let fields = FieldSet::sequence(node, &["KATUZE"])?;
    Ok(ProceedingInfoCadastralAreaGroup {
        areas: fields.parse_vec(0, |node| {
            let fields = FieldSet::sequence(node, &["kod", "nazev"])?;
            if fields.is_empty() {
                return Ok(ProceedingInfoCadastralArea { details: None });
            }
            Ok(ProceedingInfoCadastralArea {
                details: Some(LegacyNumericCodeName {
                    code: fields.parse_required(0, integer)?,
                    name: fields.parse_required(1, simple_text)?,
                }),
            })
        })?,
    })
}

fn decode_proceeding_parcel(node: Node<'_, '_>) -> Result<ProceedingInfoParcel> {
    const FIELDS: &[&str] = &["PARCELA", "TYP", "DIL_PARCELY", "ZDROJ_ZE", "PUV_KATUZE"];
    let fields = FieldSet::sequence(node, FIELDS)?;
    Ok(ProceedingInfoParcel {
        parcel: fields.parse_required(0, |node| {
            optional_nested(node, "parcela", base::parse_parcel)
        })?,
        kind: fields.parse_required(1, simple_text)?,
        part: fields.parse_required(2, simple_text)?,
        source_from: fields.parse_required(3, simple_text)?,
        original_cadastral_area: fields.parse_required(4, simple_text)?,
    })
}

fn decode_proceeding_building(node: Node<'_, '_>) -> Result<ProceedingInfoBuilding> {
    let fields = FieldSet::sequence(node, &["STAVBA", "TBUD_ZKR"])?;
    Ok(ProceedingInfoBuilding {
        building: fields.parse_required(0, |node| {
            optional_nested(node, "stavba", base::parse_building)
        })?,
        building_type_abbreviation: fields.parse_required(1, simple_text)?,
    })
}

fn decode_proceeding_deed(node: Node<'_, '_>) -> Result<ProceedingInfoDeed> {
    const FIELDS: &[&str] = &[
        "ID",
        "DATUM_VYHOTOVENI",
        "DATUM_PRAV_MOCI",
        "DALSI_UDAJE",
        "TYPLIS_NAZEV",
        "TYPLIST_KOD",
        "LISTIN_POPIS",
        "DATUM_HIST_OD",
        "DATUM_HIST_DO",
        "DALSI_UDAJE_DATUM_OD",
        "ZHOTOVITEL",
    ];
    let fields = FieldSet::sequence(node, FIELDS)?;
    Ok(ProceedingInfoDeed {
        id: fields.parse_optional(0, base::parse_legacy_id)?,
        prepared_at: fields.parse_optional(1, simple_text)?,
        legal_force_at: fields.parse_optional(2, simple_text)?,
        additional_data: fields.parse_optional(3, simple_text)?,
        deed_type_name: fields.parse_optional(4, simple_text)?,
        deed_type_code: fields.parse_optional(5, integer)?,
        description: fields.parse_optional(6, simple_text)?,
        history_from: fields.parse_optional(7, simple_text)?,
        history_to: fields.parse_optional(8, simple_text)?,
        additional_data_from: fields.parse_optional(9, simple_text)?,
        preparer: fields.parse_optional(10, simple_text)?,
    })
}

fn decode_proceeding_unit(node: Node<'_, '_>) -> Result<ProceedingInfoUnit> {
    let fields = FieldSet::sequence(node, &["C_JED", "ZP_VYUZ_JED", "BUDOVA"])?;
    Ok(ProceedingInfoUnit {
        unit: fields.parse_required(0, |node| {
            optional_nested(node, "jednotka", base::parse_unit)
        })?,
        use_description: fields.parse_required(1, simple_text)?,
        building: fields.parse_required(2, |node| {
            optional_nested(node, "stavba", base::parse_building)
        })?,
    })
}

fn decode_proceeding_geometric_plan(node: Node<'_, '_>) -> Result<ProceedingInfoGeometricPlan> {
    let fields = FieldSet::sequence(node, &["NZ_TYPE", "CISLO_PLANU", "UCEL"])?;
    Ok(ProceedingInfoGeometricPlan {
        kind: fields.parse_required(0, simple_text)?,
        number: fields.parse_required(1, simple_text)?,
        purposes: fields
            .parse_required(2, |node| decode_list(node, "nazev", false, simple_text))?,
    })
}

fn decode_proceeding_event(node: Node<'_, '_>) -> Result<ProceedingInfoEvent> {
    const FIELDS: &[&str] = &["TYPOPE_KOD", "TYPOPE_NAZEV", "TEXT", "VYTVORENO"];
    let fields = FieldSet::sequence(node, FIELDS)?;
    Ok(ProceedingInfoEvent {
        type_code: fields.parse_required(0, simple_text)?,
        type_name: fields.parse_required(1, simple_text)?,
        text: fields.parse_required(2, simple_text)?,
        created_at: fields.parse_required(3, simple_text)?,
    })
}

fn decode_proceeding_operation(node: Node<'_, '_>, xml: &str) -> Result<ProceedingInfoOperation> {
    const FIELDS: &[&str] = &[
        "TYPOPE_KOD",
        "TYPOPE_NAZEV",
        "TEXT",
        "VYTVORENO",
        "DATUM_UKONCENI",
    ];
    let fields = FieldSet::sequence(node, FIELDS)?;
    let text = fields.required(2)?;
    let raw_xml = xml
        .get(text.range())
        .ok_or_else(|| Error::invalid_response("could not retain untyped operation TEXT"))?
        .to_owned();
    Ok(ProceedingInfoOperation {
        type_code: fields.parse_required(0, simple_text)?,
        type_name: fields.parse_required(1, simple_text)?,
        text: ProceedingInfoOperationText { raw_xml },
        created_at: fields.parse_required(3, simple_text)?,
        completed_at: fields.parse_required(4, simple_text)?,
    })
}

fn decode_proceeding_defect(node: Node<'_, '_>) -> Result<ProceedingInfoDefect> {
    let fields = FieldSet::sequence(node, &["NAZEV", "UPRESNENI_ZAVADY"])?;
    Ok(ProceedingInfoDefect {
        name: fields.parse_required(0, simple_text)?,
        specification: fields.parse_required(1, simple_text)?,
    })
}

fn decode_proceeding_change_proposal(node: Node<'_, '_>) -> Result<ProceedingInfoChangeProposal> {
    const FIELDS: &[&str] = &["VYTVORIL", "CISLO_RIZENI", "STAV", "DATUM"];
    let fields = FieldSet::sequence(node, FIELDS)?;
    Ok(ProceedingInfoChangeProposal {
        created_by: fields.parse_required(0, simple_text)?,
        proceeding: fields.parse_required(1, base::parse_proceeding)?,
        state: fields.parse_required(2, simple_text)?,
        date: fields.parse_required(3, simple_text)?,
    })
}

fn decode_proceeding_building_right(node: Node<'_, '_>) -> Result<ProceedingInfoBuildingRight> {
    let fields = FieldSet::sequence(node, &["PS_PARCELA", "PS_UCEL", "PS_PLATNOST_DO"])?;
    Ok(ProceedingInfoBuildingRight {
        building_right: fields.parse_required(0, |node| {
            required_nested(node, "pravo_stavby", base::parse_building_right)
        })?,
        purpose: fields.parse_required(1, |node| required_nested(node, "nazev", simple_text))?,
        valid_until: fields.parse_required(2, simple_text)?,
    })
}

fn optional_nested<T>(
    node: Node<'_, '_>,
    child_name: &'static str,
    parser: fn(Node<'_, '_>) -> Result<T>,
) -> Result<Option<T>> {
    let names = [child_name];
    let fields = FieldSet::sequence(node, &names)?;
    fields.parse_optional(0, parser)
}

fn required_nested<T>(
    node: Node<'_, '_>,
    child_name: &'static str,
    parser: fn(Node<'_, '_>) -> Result<T>,
) -> Result<T> {
    let names = [child_name];
    let fields = FieldSet::sequence(node, &names)?;
    fields.parse_required(0, parser)
}

fn decode_list<T>(
    node: Node<'_, '_>,
    child_name: &'static str,
    require_nonempty: bool,
    parser: impl FnMut(Node<'_, '_>) -> Result<T>,
) -> Result<Vec<T>> {
    let names = [child_name];
    let fields = FieldSet::sequence(node, &names)?;
    if require_nonempty {
        fields.parse_required_vec(0, parser)
    } else {
        fields.parse_vec(0, parser)
    }
}

#[derive(Clone, Copy)]
enum FieldOrder {
    Sequence,
    All,
}

struct FieldSet<'names, 'a, 'input> {
    names: &'names [&'static str],
    values: Vec<Vec<Node<'a, 'input>>>,
}

impl<'names, 'a, 'input> FieldSet<'names, 'a, 'input> {
    fn sequence(node: Node<'a, 'input>, names: &'names [&'static str]) -> Result<Self> {
        Self::new(node, names, FieldOrder::Sequence)
    }

    fn all(node: Node<'a, 'input>, names: &'names [&'static str]) -> Result<Self> {
        Self::new(node, names, FieldOrder::All)
    }

    fn new(
        node: Node<'a, 'input>,
        names: &'names [&'static str],
        order: FieldOrder,
    ) -> Result<Self> {
        require_legacy_element(node)?;
        let mut values: Vec<Vec<Node<'a, 'input>>> = (0..names.len()).map(|_| Vec::new()).collect();
        let mut last = None;
        for child in element_children(node) {
            let index = field_index(child, names)?;
            if matches!(order, FieldOrder::Sequence) && last.is_some_and(|last| index < last) {
                return Err(invalid_value(
                    names[index],
                    "sequence element is out of order",
                ));
            }
            last = Some(index);
            values[index].push(child);
        }
        Ok(Self { names, values })
    }

    fn present(&self, index: usize) -> bool {
        !self.values[index].is_empty()
    }

    fn is_empty(&self) -> bool {
        self.values.iter().all(|values| values.is_empty())
    }

    fn one(&self, index: usize) -> Result<Option<Node<'a, 'input>>> {
        let values = &self.values[index];
        if values.len() > 1 {
            return Err(invalid_value(
                self.names[index],
                "singleton element occurs more than once",
            ));
        }
        Ok(values.first().copied())
    }

    fn required(&self, index: usize) -> Result<Node<'a, 'input>> {
        self.one(index)?.ok_or_else(|| {
            invalid_value(
                self.names[index],
                "required legacy report element is missing",
            )
        })
    }

    fn parse_required<T>(
        &self,
        index: usize,
        parser: impl FnOnce(Node<'a, 'input>) -> Result<T>,
    ) -> Result<T> {
        parser(self.required(index)?)
    }

    fn parse_optional<T>(
        &self,
        index: usize,
        parser: impl FnOnce(Node<'a, 'input>) -> Result<T>,
    ) -> Result<Option<T>> {
        self.one(index)?.map(parser).transpose()
    }

    fn parse_vec<T>(
        &self,
        index: usize,
        parser: impl FnMut(Node<'a, 'input>) -> Result<T>,
    ) -> Result<Vec<T>> {
        self.values[index]
            .iter()
            .copied()
            .map(parser)
            .collect::<Result<Vec<_>>>()
    }

    fn parse_required_vec<T>(
        &self,
        index: usize,
        parser: impl FnMut(Node<'a, 'input>) -> Result<T>,
    ) -> Result<Vec<T>> {
        if self.values[index].is_empty() {
            return Err(invalid_value(
                self.names[index],
                "at least one occurrence is required",
            ));
        }
        self.parse_vec(index, parser)
    }
}

fn field_index(node: Node<'_, '_>, names: &[&str]) -> Result<usize> {
    require_legacy_element(node)?;
    let name = node.tag_name().name();
    names
        .iter()
        .position(|candidate| *candidate == name)
        .ok_or_else(|| unexpected_field(name))
}

fn element_children<'a, 'input>(node: Node<'a, 'input>) -> impl Iterator<Item = Node<'a, 'input>> {
    node.children().filter(|child| child.is_element())
}

fn require_name(node: Node<'_, '_>, expected: &str) -> Result<()> {
    require_legacy_element(node)?;
    if node.tag_name().name() == expected {
        Ok(())
    } else {
        Err(Error::invalid_response(
            "unexpected legacy report root or element",
        ))
    }
}

fn require_legacy_element(node: Node<'_, '_>) -> Result<()> {
    if node.is_element() && node.tag_name().namespace().is_none() {
        Ok(())
    } else {
        Err(Error::invalid_response(
            "legacy report elements must have no XML namespace",
        ))
    }
}

fn ensure_not_nil(node: Node<'_, '_>) -> Result<()> {
    if node
        .attribute((XSI_NS, "nil"))
        .is_some_and(|value| value == "true" || value == "1")
    {
        Err(invalid_value(
            node.tag_name().name(),
            "legacy XSD element is not nillable",
        ))
    } else {
        Ok(())
    }
}

fn reject_nil(node: Node<'_, '_>) -> Result<()> {
    reject_nil_region(node, false)
}

fn reject_nil_region(node: Node<'_, '_>, inside_any_type: bool) -> Result<()> {
    if !inside_any_type && node.tag_name().namespace().is_none() {
        ensure_not_nil(node)?;
    }
    // OPERACE/TEXT is xs:anyType. Its outer element is non-nillable, but nested
    // content is intentionally opaque and must retain its own xsi:nil values.
    let children_are_any_type = inside_any_type
        || (node.tag_name().namespace().is_none()
            && node.tag_name().name() == "TEXT"
            && node.parent_element().is_some_and(|parent| {
                parent.tag_name().namespace().is_none() && parent.tag_name().name() == "OPERACE"
            }));
    for child in node.children().filter(|child| child.is_element()) {
        reject_nil_region(child, children_are_any_type)?;
    }
    Ok(())
}

fn simple_text(node: Node<'_, '_>) -> Result<String> {
    require_legacy_element(node)?;
    ensure_not_nil(node)?;
    if node.children().any(|child| child.is_element()) {
        return Err(invalid_value(
            node.tag_name().name(),
            "simple content cannot contain an element",
        ));
    }
    Ok(direct_text(node))
}

fn mixed_text(node: Node<'_, '_>) -> Result<String> {
    require_legacy_element(node)?;
    ensure_not_nil(node)?;
    Ok(direct_text(node))
}

fn direct_text(node: Node<'_, '_>) -> String {
    node.children()
        .filter(|child| child.is_text())
        .filter_map(|child| child.text())
        .collect()
}

fn integer<T: FromStr>(node: Node<'_, '_>) -> Result<T> {
    let raw = simple_text(node)?;
    trim_xml_whitespace(&raw).parse::<T>().map_err(|_| {
        invalid_value(
            node.tag_name().name(),
            "integer is invalid or outside its XSD range",
        )
    })
}

fn trim_xml_whitespace(value: &str) -> &str {
    value.trim_matches(|character| matches!(character, ' ' | '\t' | '\r' | '\n'))
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
                depth = depth
                    .checked_add(1)
                    .ok_or_else(|| Error::invalid_response("legacy XML nesting overflow"))?;
                if depth > max_depth {
                    return Err(Error::invalid_response(
                        "legacy XML exceeds the depth limit",
                    ));
                }
            }
            Ok(Event::Empty(element)) => {
                if depth >= max_depth {
                    return Err(Error::invalid_response(
                        "legacy XML exceeds the depth limit",
                    ));
                }
                nodes = preflight_node(nodes, max_nodes)?;
                nodes = preflight_attributes(&element, nodes, max_nodes)?;
            }
            Ok(Event::End(_)) => {
                depth = depth
                    .checked_sub(1)
                    .ok_or_else(|| Error::invalid_response("malformed legacy report XML"))?;
            }
            Ok(Event::DocType(_)) => {
                return Err(Error::invalid_response(
                    "DTD is not allowed in legacy report XML",
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
            Err(_) => return Err(Error::invalid_response("malformed legacy report XML")),
        }
    }
    if depth != 0 {
        return Err(Error::invalid_response("malformed legacy report XML"));
    }
    Ok(())
}

fn preflight_node(nodes: u32, maximum: u32) -> Result<u32> {
    let nodes = nodes
        .checked_add(1)
        .ok_or_else(|| Error::invalid_response("legacy XML exceeds the node limit"))?;
    if nodes > maximum {
        return Err(Error::invalid_response("legacy XML exceeds the node limit"));
    }
    Ok(nodes)
}

fn preflight_attributes(element: &BytesStart<'_>, mut nodes: u32, maximum: u32) -> Result<u32> {
    for attribute in element.attributes() {
        if attribute.is_err() {
            return Err(Error::invalid_response("malformed legacy report XML"));
        }
        nodes = preflight_node(nodes, maximum)?;
    }
    Ok(nodes)
}

#[cfg(test)]
#[path = "legacy_unit_reports_tests.rs"]
mod tests;
