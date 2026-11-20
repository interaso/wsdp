//! Bounded, namespace-aware decoding of NTR 3.0 ownership-sheet reports.

use std::str::FromStr;

use quick_xml::events::{BytesStart, Event};
use roxmltree::{Document, Node, ParsingOptions};
use rust_decimal::Decimal;

use crate::LegalRelationCode;
use crate::error::{Error, Result};
use crate::model::parse_decimal;
use crate::model::reports::ownership_sheet_v3::*;

const NTR_NS: &str = "urn:cz:gov:cuzk:iskn:sestavy:VypisZKatastruNemovitosti:3.0";
const COMMON_NS: &str = "urn:cz:gov:cuzk:iskn:sestavy:types:common:3.0";
const DS_NS: &str = "http://www.w3.org/2000/09/xmldsig#";
const XSI_NS: &str = "http://www.w3.org/2001/XMLSchema-instance";

/// Parses a complete NTR 3.0 ownership-sheet XML document with explicit XML limits.
pub(crate) fn parse_ntr_ownership_sheet(
    bytes: &[u8],
    max_nodes: u32,
    max_depth: u32,
) -> Result<NtrOwnershipSheet> {
    let xml = std::str::from_utf8(bytes)
        .map_err(|_| Error::invalid_response("NTR ownership sheet is not UTF-8 XML"))?;
    preflight(xml, max_nodes, max_depth)?;
    let options = ParsingOptions {
        allow_dtd: false,
        nodes_limit: max_nodes,
        ..ParsingOptions::default()
    };
    let document = Document::parse_with_options(xml, options)
        .map_err(|_| Error::invalid_response("malformed NTR ownership-sheet XML"))?;
    let root = document.root_element();
    require_tag(root, NTR_NS, "vypisZKatastruNemovitosti")?;

    let general_data = decode_general_data(required_child(root, NTR_NS, "obecneUdaje")?)?;
    let sheets_wrapper = required_child(root, NTR_NS, "listyVlastnictvi")?;
    ensure_not_nil(sheets_wrapper, "listyVlastnictvi")?;
    let ownership_sheets = children(sheets_wrapper, NTR_NS, "listVlastnictvi")
        .map(decode_sheet)
        .collect::<Result<Vec<_>>>()?;
    let metadata = decode_metadata(required_child(root, NTR_NS, "metadata")?)?;
    let signature = child(root, DS_NS, "Signature")?
        .map(|node| {
            ensure_not_nil(node, "Signature")?;
            let raw_xml = xml
                .get(node.range())
                .ok_or_else(|| Error::invalid_response("could not retain the XML signature"))?
                .to_owned();
            Ok::<NtrXmlSignature, Error>(NtrXmlSignature { raw_xml })
        })
        .transpose()?;

    Ok(NtrOwnershipSheet {
        general_data,
        ownership_sheets,
        metadata,
        signature,
    })
}

fn decode_general_data(node: Node<'_, '_>) -> Result<NtrGeneralData> {
    ensure_not_nil(node, "obecneUdaje")?;
    Ok(NtrGeneralData {
        // These five elements originate in common_3.0.xsd's abstract base type.
        valid_at: required_parse(node, COMMON_NS, "platnost")?,
        workplace: optional_child(node, COMMON_NS, "pracoviste")?
            .map(decode_workplace_with_contacts)
            .transpose()?,
        privacy_notice: optional_text(node, COMMON_NS, "pouceni")?,
        user: optional_child(node, COMMON_NS, "uzivatel")?
            .map(decode_user)
            .transpose()?,
        created_at: required_parse(node, COMMON_NS, "vyhotoveno")?,
        // These elements are added by the report schema's extension.
        is_partial: required_code(node, NTR_NS, "castecnyVypis", NtrYesNo::new)?,
        non_public_deed_header: optional_text(node, NTR_NS, "neverejnaListina")?,
        wsgp_header: optional_text(node, NTR_NS, "wsgp")?,
        verification: optional_child(node, NTR_NS, "overeni")?
            .map(decode_verification)
            .transpose()?,
    })
}

fn decode_workplace_with_contacts(node: Node<'_, '_>) -> Result<NtrWorkplaceWithContacts> {
    ensure_not_nil(node, "pracoviste")?;
    Ok(NtrWorkplaceWithContacts {
        workplace: decode_workplace_fields(node)?,
        address: optional_child(node, COMMON_NS, "adresa")?
            .map(decode_address)
            .transpose()?,
        contact: optional_child(node, COMMON_NS, "kontakt")?
            .map(decode_contact)
            .transpose()?,
    })
}

fn decode_workplace(node: Node<'_, '_>) -> Result<NtrWorkplace> {
    ensure_not_nil(node, "pracoviste")?;
    decode_workplace_fields(node)
}

fn decode_workplace_fields(node: Node<'_, '_>) -> Result<NtrWorkplace> {
    let code = required_text(node, COMMON_NS, "kod")?;
    validate_positive_integer(&code, 999, "workplace code")?;
    Ok(NtrWorkplace {
        code: parse_response_value(code.trim(), "workplace code")?,
        name: optional_text(node, COMMON_NS, "nazev")?,
    })
}

fn decode_contact(node: Node<'_, '_>) -> Result<NtrContact> {
    ensure_not_nil(node, "kontakt")?;
    Ok(NtrContact {
        telephone: optional_text(node, COMMON_NS, "telefon")?,
        fax: optional_text(node, COMMON_NS, "fax")?,
        email: optional_text(node, COMMON_NS, "email")?,
        data_box_id: optional_text(node, COMMON_NS, "idds")?,
    })
}

fn decode_user(node: Node<'_, '_>) -> Result<NtrReportUser> {
    ensure_not_nil(node, "uzivatel")?;
    Ok(NtrReportUser {
        username: optional_text(node, COMMON_NS, "uzivatelskeJmeno")?,
        full_name: optional_text(node, COMMON_NS, "celeJmeno")?,
    })
}

fn decode_verification(node: Node<'_, '_>) -> Result<NtrOutputVerification> {
    ensure_not_nil(node, "overeni")?;
    Ok(NtrOutputVerification {
        clause: optional_text(node, COMMON_NS, "dolozka")?,
        verified: optional_text(node, COMMON_NS, "overeno")?,
        free_access: optional_text(node, COMMON_NS, "bezplatny")?,
    })
}

fn decode_metadata(node: Node<'_, '_>) -> Result<NtrReportMetadata> {
    ensure_not_nil(node, "metadata")?;
    Ok(NtrReportMetadata {
        report_uuid: required_parse(node, COMMON_NS, "identifikatorSestavy")?,
        run_id: optional_parse(node, COMMON_NS, "behId")?,
        processing_id: optional_parse(node, COMMON_NS, "dpId")?,
        environment: required_text(node, COMMON_NS, "prostredi")?,
        copyright: required_text(node, COMMON_NS, "copyright")?,
        application: required_text(node, COMMON_NS, "aplikace")?,
        application_version: required_text(node, COMMON_NS, "verzeAplikace")?,
        report_code: required_text(node, COMMON_NS, "kodSestavy")?,
        report_version: required_parse(node, COMMON_NS, "verzeSestavy")?,
    })
}

fn decode_sheet(node: Node<'_, '_>) -> Result<NtrOwnershipSheetEntry> {
    ensure_not_nil(node, "listVlastnictvi")?;
    let owners = required_child(node, NTR_NS, "vlastnici")?;
    ensure_not_nil(owners, "vlastnici")?;
    let owner_relations = optional_nonempty_list(
        owners,
        NTR_NS,
        "pravniVztahy",
        "pravniVztah",
        decode_legal_relation,
    )?;
    Ok(NtrOwnershipSheetEntry {
        id: required_parse(node, NTR_NS, "id")?,
        number: required_bounded(node, NTR_NS, "cislo", 1, 999_999)?,
        district: decode_district(required_child(node, NTR_NS, "okres")?)?,
        municipality: decode_municipality(required_child(node, NTR_NS, "obec")?)?,
        cadastral_area: decode_cadastral_area(required_child(node, NTR_NS, "katastrUzemi")?)?,
        workplace: decode_workplace(required_child(node, NTR_NS, "pracoviste")?)?,
        numbering_series: decode_numbering_series(required_child(node, NTR_NS, "ciselnaRada")?)?,
        owner_relations,
        properties: decode_properties(required_child(node, NTR_NS, "nemovitosti")?)?,
        other_rights: required_list(node, NTR_NS, "jinaPrava", "jinePravo", decode_other_right)?,
        notices: required_list(
            node,
            NTR_NS,
            "seznamUpozorneni",
            "upozorneni",
            decode_notice,
        )?,
        acquisition_titles: required_list(
            node,
            NTR_NS,
            "nabyvaciTituly",
            "nabyvaciTitul",
            decode_acquisition_title,
        )?,
        bonities: required_list(node, NTR_NS, "bonity", "bonita", decode_bonity)?,
    })
}

fn decode_district(node: Node<'_, '_>) -> Result<NtrDistrict> {
    ensure_not_nil(node, "okres")?;
    let raw = required_text(node, COMMON_NS, "kod")?;
    validate_positive_integer(&raw, 9_999, "district code")?;
    Ok(NtrDistrict {
        code: parse_response_value(raw.trim(), "district code")?,
        name: required_text(node, COMMON_NS, "nazev")?,
        nuts4: optional_text(node, COMMON_NS, "nuts4")?,
    })
}

fn decode_municipality(node: Node<'_, '_>) -> Result<NtrMunicipality> {
    ensure_not_nil(node, "obec")?;
    let raw = required_text(node, COMMON_NS, "kod")?;
    validate_positive_integer(&raw, 999_999, "municipality code")?;
    Ok(NtrMunicipality {
        code: parse_response_value(raw.trim(), "municipality code")?,
        name: required_text(node, COMMON_NS, "nazev")?,
    })
}

fn decode_cadastral_area(node: Node<'_, '_>) -> Result<NtrCadastralArea> {
    ensure_not_nil(node, "katastrUzemi")?;
    let raw = required_text(node, COMMON_NS, "kod")?;
    validate_positive_integer(&raw, 999_999, "cadastral-area code")?;
    Ok(NtrCadastralArea {
        code: parse_response_value(raw.trim(), "cadastral-area code")?,
        name: required_text(node, COMMON_NS, "nazev")?,
    })
}

fn decode_municipality_part(node: Node<'_, '_>) -> Result<NtrMunicipalityPart> {
    ensure_not_nil(node, "castObce")?;
    let raw = required_text(node, COMMON_NS, "kod")?;
    validate_positive_integer(&raw, 999_999, "municipality-part code")?;
    Ok(NtrMunicipalityPart {
        code: parse_response_value(raw.trim(), "municipality-part code")?,
        name: required_text(node, COMMON_NS, "nazev")?,
    })
}

fn decode_numbering_series(node: Node<'_, '_>) -> Result<NtrParcelNumberingSeries> {
    ensure_not_nil(node, "ciselnaRada")?;
    Ok(NtrParcelNumberingSeries {
        code: required_numeric_code(node, "kod", 9, NtrParcelNumberingSeriesCode::new)?,
        name: required_text(node, COMMON_NS, "nazev")?,
    })
}

fn decode_properties(node: Node<'_, '_>) -> Result<NtrProperties> {
    ensure_not_nil(node, "nemovitosti")?;
    Ok(NtrProperties {
        parcels: optional_list_ns(node, NTR_NS, "parcely", COMMON_NS, "parcela", decode_parcel)?,
        buildings: optional_list_ns(node, NTR_NS, "stavby", COMMON_NS, "stavba", decode_building)?,
        building_rights: optional_list_ns(
            node,
            NTR_NS,
            "pravaStavby",
            COMMON_NS,
            "pravoStavby",
            decode_building_right,
        )?,
        units: optional_list_ns(node, NTR_NS, "jednotky", COMMON_NS, "jednotka", decode_unit)?,
        simplified_parcels: optional_list_ns(
            node,
            NTR_NS,
            "parcelyZe",
            COMMON_NS,
            "parcela",
            decode_parcel,
        )?,
    })
}

fn decode_legal_relation(node: Node<'_, '_>) -> Result<NtrLegalRelation> {
    ensure_not_nil(node, "pravniVztah")?;
    let code = required_text(node, NTR_NS, "kod")?;
    Ok(NtrLegalRelation {
        code: LegalRelationCode::new(code),
        name: required_text(node, NTR_NS, "nazev")?,
        ownership_records: required_list(
            node,
            NTR_NS,
            "seznamVlastnictvi",
            "vlastnictvi",
            decode_ownership_record,
        )?,
    })
}

fn decode_ownership_record(node: Node<'_, '_>) -> Result<NtrOwnershipRecord> {
    ensure_not_nil(node, "vlastnictvi")?;
    Ok(NtrOwnershipRecord {
        ownership: decode_ownership_fields(node)?,
        subordinate_relations: optional_nonempty_list(
            node,
            NTR_NS,
            "podrizenePravniVztahy",
            "pravniVztah",
            decode_legal_relation,
        )?,
    })
}

fn decode_unit_ownership(node: Node<'_, '_>) -> Result<NtrOwnership> {
    ensure_not_nil(node, "vlastnictvi")?;
    decode_ownership_fields(node)
}

fn decode_ownership_fields(node: Node<'_, '_>) -> Result<NtrOwnership> {
    Ok(NtrOwnership {
        id: required_parse(node, COMMON_NS, "id")?,
        parent_id: optional_parse(node, COMMON_NS, "nadrizeneId")?,
        share: decode_share(required_child(node, COMMON_NS, "podil")?)?,
        target: decode_ownership_target(node)?,
    })
}

fn decode_share(node: Node<'_, '_>) -> Result<NtrShare> {
    ensure_not_nil(node, "podil")?;
    let numerator_text = required_text(node, COMMON_NS, "citatel")?;
    let denominator_text = required_text(node, COMMON_NS, "jmenovatel")?;
    let numerator = parse_share_component(&numerator_text, "NTR share numerator")?;
    let denominator = parse_share_component(&denominator_text, "NTR share denominator")?;
    NtrShare::new(numerator, denominator)
        .map_err(|_| Error::invalid_response("NTR share components must be positive integers"))
}

fn parse_share_component(raw: &str, field: &'static str) -> Result<Decimal> {
    let value = raw.trim();
    let Some(digits) = xsd_unsigned_integer_digits(value) else {
        return Err(Error::invalid_response(
            "NTR share component must be a positive integer of at most 30 digits",
        ));
    };
    if digits.len() > 30 || digits.bytes().all(|byte| byte == b'0') {
        return Err(Error::invalid_response(
            "NTR share component must be a positive integer of at most 30 digits",
        ));
    }
    parse_decimal(value, field).map_err(|error| match error {
        Error::DecimalOutOfRange { .. } => error,
        _ => Error::invalid_response("NTR share component has invalid decimal syntax"),
    })
}

fn decode_ownership_target(node: Node<'_, '_>) -> Result<NtrOwnershipTarget> {
    let targets = node
        .children()
        .filter(|candidate| {
            candidate.is_element()
                && candidate.tag_name().namespace() == Some(COMMON_NS)
                && matches!(
                    candidate.tag_name().name(),
                    "opravnenySubjekt" | "parcela" | "stavba" | "jednotka" | "pravoStavby"
                )
        })
        .collect::<Vec<_>>();
    if targets.len() != 1 {
        return Err(Error::invalid_response(
            "NTR ownership must contain exactly one subject or property target",
        ));
    }
    let target = targets[0];
    ensure_not_nil(target, target.tag_name().name())?;
    match target.tag_name().name() {
        "opravnenySubjekt" => decode_subject(target)
            .map(Box::new)
            .map(NtrOwnershipTarget::Subject),
        "parcela" => decode_parcel(target)
            .map(Box::new)
            .map(NtrOwnershipTarget::Parcel),
        "stavba" => decode_building(target)
            .map(Box::new)
            .map(NtrOwnershipTarget::Building),
        "jednotka" => decode_unit(target)
            .map(Box::new)
            .map(NtrOwnershipTarget::Unit),
        "pravoStavby" => decode_building_right(target)
            .map(Box::new)
            .map(NtrOwnershipTarget::BuildingRight),
        _ => Err(Error::invalid_response("unsupported NTR ownership target")),
    }
}

fn decode_subject(node: Node<'_, '_>) -> Result<NtrAuthorizedSubject> {
    ensure_not_nil(node, "opravnenySubjekt")?;
    let characteristic = required_text(node, COMMON_NS, "charKod")?;
    validate_positive_integer(&characteristic, 99, "subject characteristic code")?;
    Ok(NtrAuthorizedSubject {
        id: required_parse(node, COMMON_NS, "id")?,
        kind: required_code(node, COMMON_NS, "typ", NtrSubjectKind::new)?,
        characteristic_code: NtrSubjectCharacteristicCode::new(characteristic.trim()),
        characteristic_abbreviation: optional_text(node, COMMON_NS, "charZkratka")?,
        alv_abbreviation: optional_text(node, COMMON_NS, "charZkratkaAlv")?,
        joint_property_name: optional_text(node, COMMON_NS, "nazevSJ")?,
        primary: decode_subject_details(required_child(node, COMMON_NS, "oS")?)?,
        secondary: optional_child(node, COMMON_NS, "oS2")?
            .map(decode_subject_details)
            .transpose()?,
    })
}

fn decode_subject_details(node: Node<'_, '_>) -> Result<NtrSubjectDetails> {
    ensure_not_nil(node, "oS")?;
    Ok(NtrSubjectDetails {
        id: required_parse(node, COMMON_NS, "id")?,
        kind: required_code(node, COMMON_NS, "typ", NtrSubjectKind::new)?,
        given_name: optional_text(node, COMMON_NS, "jmeno")?,
        surname: optional_text(node, COMMON_NS, "prijmeni")?,
        title_before: optional_text(node, COMMON_NS, "titulPred")?,
        title_after: optional_text(node, COMMON_NS, "titulZa")?,
        organization_name: optional_text(node, COMMON_NS, "nazev")?,
        birth_number: optional_parse(node, COMMON_NS, "rodneCislo")?,
        company_id: optional_text(node, COMMON_NS, "ico")?,
        company_id_suffix: optional_bounded(node, COMMON_NS, "icoDoplnek", 1, 999)?,
        address: decode_address(required_child(node, COMMON_NS, "adresa")?)?,
    })
}

fn decode_address(node: Node<'_, '_>) -> Result<NtrAddress> {
    ensure_not_nil(node, "adresa")?;
    let postal_code = optional_text(node, COMMON_NS, "psc")?;
    if let Some(raw) = &postal_code {
        validate_nonnegative_integer(raw, 99_999, "postal code")?;
    }
    let ruian_id = optional_text(node, COMMON_NS, "kodAdrm")?;
    if let Some(raw) = &ruian_id {
        validate_positive_integer(raw, 999_999_999, "RUIAN address ID")?;
    }
    Ok(NtrAddress {
        orientation_number: optional_text(node, COMMON_NS, "cisloOrientacni")?,
        house_number: optional_bounded(node, COMMON_NS, "cisloDomovni", 1, 9_999)?,
        house_number_kind: optional_code(
            node,
            COMMON_NS,
            "cisloDomovniTyp",
            NtrHouseNumberKind::new,
        )?,
        street_name: optional_text(node, COMMON_NS, "nazevUlice")?,
        municipality_part: optional_text(node, COMMON_NS, "castObce")?,
        city_district: optional_text(node, COMMON_NS, "mestskaCast")?,
        city_borough: optional_text(node, COMMON_NS, "mestskyObvod")?,
        municipality: optional_text(node, COMMON_NS, "obec")?,
        district: optional_text(node, COMMON_NS, "okres")?,
        postal_code: postal_code.map(|raw| raw.trim().to_owned()),
        delivery_post_office: optional_text(node, COMMON_NS, "dodPosta")?,
        country: optional_text(node, COMMON_NS, "stat")?,
        ruian_id: ruian_id
            .map(|raw| parse_response_value(raw.trim(), "RUIAN address ID"))
            .transpose()?,
    })
}

fn decode_parcel(node: Node<'_, '_>) -> Result<NtrParcel> {
    ensure_not_nil(node, "parcela")?;
    Ok(NtrParcel {
        id: required_parse(node, COMMON_NS, "id")?,
        building_id: optional_parse(node, COMMON_NS, "budId")?,
        identifies_building: optional_code(node, COMMON_NS, "budIdent", NtrYesNo::new)?,
        building_right_id: optional_parse(node, COMMON_NS, "psId")?,
        identifies_building_right: optional_code(node, COMMON_NS, "psIdent", NtrYesNo::new)?,
        kind: required_code(node, COMMON_NS, "typ", NtrParcelKind::new)?,
        character: optional_child(node, COMMON_NS, "charakter")?
            .map(decode_parcel_character)
            .transpose()?,
        root_number: required_bounded(node, COMMON_NS, "kmenCis", 1, 99_999)?,
        subdivision: optional_bounded(node, COMMON_NS, "poddCis", 1, 999)?,
        seal: optional_code(node, COMMON_NS, "plomba", NtrSeal::new)?,
        part: optional_bounded(node, COMMON_NS, "dil", 1, 9)?,
        area: required_bounded(node, COMMON_NS, "vymera", 0, 999_999_999)?,
        numbering_source: optional_child(node, COMMON_NS, "zdrojParcel")?
            .map(decode_numbering_source)
            .transpose()?,
        numbering_method: optional_child(node, COMMON_NS, "druhCislovani")?
            .map(decode_numbering_method)
            .transpose()?,
        land_kind: optional_child(node, COMMON_NS, "druhPozemku")?
            .map(decode_land_kind)
            .transpose()?,
        land_use: optional_child(node, COMMON_NS, "zpusobVyuziti")?
            .map(decode_parcel_use)
            .transpose()?,
        protection_methods: optional_list(
            node,
            COMMON_NS,
            "zpusobyOchrany",
            "zpusobOchrany",
            decode_protection_method,
        )?,
        cadastral_area: decode_cadastral_area(required_child(node, COMMON_NS, "katastrUzemi")?)?,
        original_cadastral_area: optional_child(node, COMMON_NS, "katastrUzemiPuv")?
            .map(decode_cadastral_area)
            .transpose()?,
        ownership_sheet: optional_child(node, COMMON_NS, "lv")?
            .map(decode_sheet_reference)
            .transpose()?,
        has_building_as_part: optional_code(node, COMMON_NS, "soucasti", NtrYesNo::new)?,
        building: optional_child(node, COMMON_NS, "stavba")?
            .map(|building| decode_building(building).map(Box::new))
            .transpose()?,
    })
}

fn decode_numbering_source(node: Node<'_, '_>) -> Result<NtrParcelNumberingSource> {
    ensure_not_nil(node, "zdrojParcel")?;
    Ok(NtrParcelNumberingSource {
        code: required_numeric_code(node, "kod", 9, NtrParcelNumberingSourceCode::new)?,
        name: required_text(node, COMMON_NS, "nazev")?,
        abbreviation: optional_text(node, COMMON_NS, "zkratka")?,
    })
}

fn decode_numbering_method(node: Node<'_, '_>) -> Result<NtrParcelNumberingMethod> {
    ensure_not_nil(node, "druhCislovani")?;
    Ok(NtrParcelNumberingMethod {
        code: required_numeric_code(node, "kod", 9, NtrParcelNumberingMethodCode::new)?,
        name: optional_text(node, COMMON_NS, "nazev")?,
        abbreviation: optional_text(node, COMMON_NS, "zkratka")?,
    })
}

fn decode_parcel_character(node: Node<'_, '_>) -> Result<NtrParcelCharacter> {
    ensure_not_nil(node, "charakter")?;
    Ok(NtrParcelCharacter {
        code: required_numeric_code(node, "kod", 9, NtrParcelCharacterCode::new)?,
        name: optional_text(node, COMMON_NS, "nazev")?,
        abbreviation: optional_text(node, COMMON_NS, "zkratka")?,
    })
}

fn decode_land_kind(node: Node<'_, '_>) -> Result<NtrLandKind> {
    ensure_not_nil(node, "druhPozemku")?;
    Ok(NtrLandKind {
        code: required_numeric_code(node, "kod", 99, NtrLandKindCode::new)?,
        name: required_text(node, COMMON_NS, "nazev")?,
    })
}

fn decode_parcel_use(node: Node<'_, '_>) -> Result<NtrParcelUse> {
    ensure_not_nil(node, "zpusobVyuziti")?;
    Ok(NtrParcelUse {
        code: required_numeric_code(node, "kod", 9_999, NtrParcelUseCode::new)?,
        name: required_text(node, COMMON_NS, "nazev")?,
        abbreviation: optional_text(node, COMMON_NS, "zkratka")?,
    })
}

fn decode_building_use(node: Node<'_, '_>) -> Result<NtrBuildingUse> {
    ensure_not_nil(node, "zpusobVyuziti")?;
    Ok(NtrBuildingUse {
        code: required_numeric_code(node, "kod", 9_999, NtrBuildingUseCode::new)?,
        name: required_text(node, COMMON_NS, "nazev")?,
        abbreviation: optional_text(node, COMMON_NS, "zkratka")?,
    })
}

fn decode_unit_use(node: Node<'_, '_>) -> Result<NtrUnitUse> {
    ensure_not_nil(node, "zpusobVyuziti")?;
    Ok(NtrUnitUse {
        code: required_numeric_code(node, "kod", 9_999, NtrUnitUseCode::new)?,
        name: required_text(node, COMMON_NS, "nazev")?,
        abbreviation: optional_text(node, COMMON_NS, "zkratka")?,
    })
}

fn decode_protection_method(node: Node<'_, '_>) -> Result<NtrProtectionMethod> {
    ensure_not_nil(node, "zpusobOchrany")?;
    Ok(NtrProtectionMethod {
        code: required_numeric_code(node, "kod", 9_999, NtrProtectionMethodCode::new)?,
        name: required_text(node, COMMON_NS, "nazev")?,
    })
}

fn decode_building(node: Node<'_, '_>) -> Result<NtrBuilding> {
    ensure_not_nil(node, "stavba")?;
    Ok(NtrBuilding {
        id: required_parse(node, COMMON_NS, "id")?,
        parcel_id: optional_parse(node, COMMON_NS, "parId")?,
        building_right_id: optional_parse(node, COMMON_NS, "psId")?,
        house_numbers: optional_text(node, COMMON_NS, "cislaDomovni")?,
        temporary: optional_code(node, COMMON_NS, "docasna", NtrYesNo::new)?,
        seal: optional_code(node, COMMON_NS, "plomba", NtrSeal::new)?,
        municipality_part: optional_child(node, COMMON_NS, "castObce")?
            .map(decode_municipality_part)
            .transpose()?,
        building_type: optional_child(node, COMMON_NS, "typBudovy")?
            .map(decode_building_type)
            .transpose()?,
        building_use: optional_child(node, COMMON_NS, "zpusobVyuziti")?
            .map(decode_building_use)
            .transpose()?,
        protection_methods: optional_list(
            node,
            COMMON_NS,
            "zpusobyOchrany",
            "zpusobOchrany",
            decode_protection_method,
        )?,
        cadastral_area: decode_cadastral_area(required_child(node, COMMON_NS, "katastrUzemi")?)?,
        ownership_sheet: optional_child(node, COMMON_NS, "lv")?
            .map(decode_sheet_reference)
            .transpose()?,
        parcels: optional_list(node, COMMON_NS, "parcely", "parcela", decode_parcel)?,
        units: optional_list(node, COMMON_NS, "jednotky", "jednotka", decode_unit)?,
        part_of: optional_child(node, COMMON_NS, "soucasti")?
            .map(decode_building_part)
            .transpose()?,
    })
}

fn decode_building_type(node: Node<'_, '_>) -> Result<NtrBuildingType> {
    ensure_not_nil(node, "typBudovy")?;
    Ok(NtrBuildingType {
        code: required_numeric_code(node, "kod", 9, NtrBuildingTypeCode::new)?,
        name: required_text(node, COMMON_NS, "nazev")?,
        abbreviation: optional_text(node, COMMON_NS, "zkratka")?,
    })
}

fn decode_building_part(node: Node<'_, '_>) -> Result<NtrBuildingPart> {
    ensure_not_nil(node, "soucasti")?;
    let kind = node
        .attribute("nem")
        .map(NtrBuildingPartKind::new)
        .ok_or_else(|| Error::invalid_response("building soucasti is missing its nem attribute"))?;
    let targets = node
        .children()
        .filter(|candidate| {
            candidate.is_element()
                && candidate.tag_name().namespace() == Some(COMMON_NS)
                && matches!(candidate.tag_name().name(), "parcela" | "pravoStavby")
        })
        .collect::<Vec<_>>();
    if targets.len() != 1 {
        return Err(Error::invalid_response(
            "building soucasti must contain exactly one parcel or building right",
        ));
    }
    let target = match targets[0].tag_name().name() {
        "parcela" => decode_parcel(targets[0])
            .map(Box::new)
            .map(NtrBuildingPartTarget::Parcel)?,
        "pravoStavby" => decode_building_right(targets[0])
            .map(Box::new)
            .map(NtrBuildingPartTarget::BuildingRight)?,
        _ => {
            return Err(Error::invalid_response(
                "unsupported building soucasti target",
            ));
        }
    };
    Ok(NtrBuildingPart { kind, target })
}

fn decode_building_right(node: Node<'_, '_>) -> Result<NtrBuildingRight> {
    ensure_not_nil(node, "pravoStavby")?;
    Ok(NtrBuildingRight {
        id: required_parse(node, COMMON_NS, "id")?,
        ended_on: optional_parse(node, COMMON_NS, "datumUkonceni")?,
        seal: optional_code(node, COMMON_NS, "plomba", NtrSeal::new)?,
        cadastral_area: decode_cadastral_area(required_child(node, COMMON_NS, "katastrUzemi")?)?,
        ownership_sheet: optional_child(node, COMMON_NS, "lv")?
            .map(decode_sheet_reference)
            .transpose()?,
        parcels: optional_list(node, COMMON_NS, "parcely", "parcela", decode_parcel)?,
        buildings: optional_list(node, COMMON_NS, "stavby", "stavba", decode_building)?,
        purposes: optional_list(
            node,
            COMMON_NS,
            "ucely",
            "ucel",
            decode_building_right_purpose,
        )?,
        protection_methods: optional_list(
            node,
            COMMON_NS,
            "zpusobyOchrany",
            "zpusobOchrany",
            decode_protection_method,
        )?,
    })
}

fn decode_building_right_purpose(node: Node<'_, '_>) -> Result<NtrBuildingRightPurpose> {
    ensure_not_nil(node, "ucel")?;
    Ok(NtrBuildingRightPurpose {
        code: required_numeric_code(node, "kod", 99, NtrBuildingRightPurposeCode::new)?,
        name: required_text(node, COMMON_NS, "nazev")?,
    })
}

fn decode_unit(node: Node<'_, '_>) -> Result<NtrUnit> {
    ensure_not_nil(node, "jednotka")?;
    Ok(NtrUnit {
        id: required_parse(node, COMMON_NS, "id")?,
        number: required_bounded(node, COMMON_NS, "cisloJednotky", 1, 9_999_999_999)?,
        house_number: optional_bounded(node, COMMON_NS, "cisloDomovni", 1, 9_999)?,
        derived_number: required_bounded(node, COMMON_NS, "cislo", 0, 9_999)?,
        description: optional_text(node, COMMON_NS, "popis")?,
        seal: optional_code(node, COMMON_NS, "plomba", NtrSeal::new)?,
        ownership_sheet: optional_child(node, COMMON_NS, "lv")?
            .map(decode_sheet_reference)
            .transpose()?,
        share: decode_share(required_child(node, COMMON_NS, "podil")?)?,
        unit_type: decode_unit_type(required_child(node, COMMON_NS, "typ")?)?,
        unit_use: optional_child(node, COMMON_NS, "zpusobVyuziti")?
            .map(decode_unit_use)
            .transpose()?,
        protection_methods: optional_list(
            node,
            COMMON_NS,
            "zpusobyOchrany",
            "zpusobOchrany",
            decode_protection_method,
        )?,
        co_ownership: optional_list(
            node,
            COMMON_NS,
            "seznamVlastnictvi",
            "vlastnictvi",
            decode_unit_ownership,
        )?,
        parcels: optional_list(node, COMMON_NS, "parcely", "parcela", decode_parcel)?,
        building: optional_child(node, COMMON_NS, "stavba")?
            .map(|building| decode_building(building).map(Box::new))
            .transpose()?,
        buildings: optional_list(node, COMMON_NS, "stavby", "stavba", decode_building)?,
        building_rights: optional_list(
            node,
            COMMON_NS,
            "pravaStavby",
            "pravoStavby",
            decode_building_right,
        )?,
    })
}

fn decode_unit_type(node: Node<'_, '_>) -> Result<NtrUnitType> {
    ensure_not_nil(node, "typ")?;
    Ok(NtrUnitType {
        code: required_numeric_code(node, "kod", 99, NtrUnitTypeCode::new)?,
        name: required_text(node, COMMON_NS, "nazev")?,
        abbreviation: optional_text(node, COMMON_NS, "zkratka")?,
    })
}

fn decode_sheet_reference(node: Node<'_, '_>) -> Result<NtrOwnershipSheetReference> {
    ensure_not_nil(node, "lv")?;
    Ok(NtrOwnershipSheetReference {
        id: required_parse(node, COMMON_NS, "id")?,
        number: required_bounded(node, COMMON_NS, "cislo", 1, 999_999)?,
    })
}

fn decode_document(node: Node<'_, '_>) -> Result<NtrDocument> {
    ensure_not_nil(node, "listina")?;
    Ok(NtrDocument {
        formatted_text: optional_text(node, COMMON_NS, "formatText")?,
        id: required_parse(node, COMMON_NS, "id")?,
        created_on: optional_parse(node, COMMON_NS, "vyhotoveno")?,
        legal_force_on: optional_parse(node, COMMON_NS, "pravniMoc")?,
        enforceability_on: optional_parse(node, COMMON_NS, "vykonatelnost")?,
        change_statement_item: optional_text(node, COMMON_NS, "pvz")?,
        document_type: decode_document_type(required_child(node, COMMON_NS, "typListiny")?)?,
        description: optional_text(node, COMMON_NS, "popis")?,
        details: optional_list(
            node,
            COMMON_NS,
            "dalsiUdaje",
            "dalsiUdaj",
            decode_document_detail,
        )?,
        abbreviation: optional_text(node, COMMON_NS, "zkratka")?,
        originator: optional_text(node, COMMON_NS, "zhotovitel")?,
        originator_serial_number: optional_text(node, COMMON_NS, "poradoveCislo")?,
        originator_year: optional_bounded(node, COMMON_NS, "rokVyhotoveni", 1, 9_999)?,
        supplement: optional_text(node, COMMON_NS, "doplnek")?,
        workplace: optional_child(node, COMMON_NS, "pracoviste")?
            .map(decode_workplace)
            .transpose()?,
        proceeding: decode_proceeding(required_child(node, COMMON_NS, "rizeni")?)?,
    })
}

fn decode_document_type(node: Node<'_, '_>) -> Result<NtrDocumentType> {
    ensure_not_nil(node, "typListiny")?;
    Ok(NtrDocumentType {
        code: required_numeric_code(node, "kod", 99_999_999, NtrDocumentTypeCode::new)?,
        name: required_text(node, COMMON_NS, "nazev")?,
    })
}

fn decode_document_detail(node: Node<'_, '_>) -> Result<NtrDocumentDetail> {
    ensure_not_nil(node, "dalsiUdaj")?;
    Ok(NtrDocumentDetail {
        code: required_numeric_code(node, "kod", 9_999_999_999, NtrDocumentDetailCode::new)?,
        name: optional_text(node, COMMON_NS, "nazev")?,
    })
}

fn decode_proceeding(node: Node<'_, '_>) -> Result<NtrProceeding> {
    ensure_not_nil(node, "rizeni")?;
    Ok(NtrProceeding {
        id: required_parse(node, COMMON_NS, "id")?,
        proceeding_type: decode_proceeding_type(required_child(node, COMMON_NS, "typ")?)?,
        serial_number: required_bounded(node, COMMON_NS, "poradoveCislo", 1, 99_999_999)?,
        year: required_bounded(node, COMMON_NS, "rok", 1, 9_999)?,
        workplace: decode_workplace(required_child(node, COMMON_NS, "pracoviste")?)?,
        received_at: optional_parse(node, COMMON_NS, "prijato")?,
        validated_on: optional_parse(node, COMMON_NS, "zplatneno")?,
        external_id: optional_text(node, COMMON_NS, "extIdentifikace")?,
    })
}

fn decode_proceeding_type(node: Node<'_, '_>) -> Result<NtrProceedingType> {
    ensure_not_nil(node, "typ")?;
    Ok(NtrProceedingType {
        code: NtrProceedingTypeCode::new(required_text(node, COMMON_NS, "kod")?),
    })
}

fn decode_other_right(node: Node<'_, '_>) -> Result<NtrOtherRight> {
    ensure_not_nil(node, "jinePravo")?;
    Ok(NtrOtherRight {
        section: decode_other_right_section(required_child(node, NTR_NS, "sekce")?)?,
        group_id: optional_parse(node, NTR_NS, "hjpvId")?,
        parent_group_id: optional_parse(node, NTR_NS, "nadrizHjpvId")?,
        legal_relation: optional_child(node, NTR_NS, "pravniVztah")?
            .map(decode_coded_legal_relation)
            .transpose()?,
        ordering: optional_child(node, NTR_NS, "poradi")?
            .map(decode_other_right_ordering)
            .transpose()?,
        related: optional_code(node, NTR_NS, "souvisejiciJpv", NtrYesNo::new)?,
        description: optional_text(node, NTR_NS, "popis")?,
        for_relation: optional_child(node, NTR_NS, "jpvPro")?
            .map(decode_other_right_relations)
            .transpose()?,
        against_relation: optional_child(node, NTR_NS, "jpvK")?
            .map(decode_other_right_relations)
            .transpose()?,
        unbound_relation: optional_child(node, NTR_NS, "jpv")?
            .map(decode_other_right_relations)
            .transpose()?,
    })
}

fn decode_other_right_section(node: Node<'_, '_>) -> Result<NtrOtherRightSection> {
    ensure_not_nil(node, "sekce")?;
    Ok(NtrOtherRightSection {
        code: required_code(node, COMMON_NS, "kod", NtrOtherRightSectionCode::new)?,
        name: required_text(node, COMMON_NS, "nazev")?,
        abbreviation: required_code(
            node,
            COMMON_NS,
            "zkratka",
            NtrOtherRightSectionAbbreviation::new,
        )?,
    })
}

fn decode_coded_legal_relation(node: Node<'_, '_>) -> Result<NtrCodedLegalRelation> {
    ensure_not_nil(node, "pravniVztah")?;
    Ok(NtrCodedLegalRelation {
        code: LegalRelationCode::new(required_text(node, COMMON_NS, "kod")?),
        name: required_text(node, COMMON_NS, "nazev")?,
    })
}

fn decode_other_right_ordering(node: Node<'_, '_>) -> Result<NtrOtherRightOrdering> {
    ensure_not_nil(node, "poradi")?;
    Ok(NtrOtherRightOrdering {
        time: optional_parse(node, COMMON_NS, "cas")?,
        text: optional_text(node, COMMON_NS, "text")?,
    })
}

fn decode_other_right_relations(node: Node<'_, '_>) -> Result<NtrOtherRightRelations> {
    ensure_not_nil(node, "jpv")?;
    Ok(NtrOtherRightRelations {
        subjects: optional_list(
            node,
            NTR_NS,
            "opravneneSubjekty",
            "opravnenySubjekt",
            decode_creditor_subject,
        )?,
        parcels: optional_list_ns(node, NTR_NS, "parcely", COMMON_NS, "parcela", decode_parcel)?,
        buildings: optional_list_ns(node, NTR_NS, "stavby", COMMON_NS, "stavba", decode_building)?,
        units: optional_list_ns(node, NTR_NS, "jednotky", COMMON_NS, "jednotka", decode_unit)?,
        building_rights: optional_list_ns(
            node,
            NTR_NS,
            "pravaStavby",
            COMMON_NS,
            "pravoStavby",
            decode_building_right,
        )?,
        ownership_sheet: optional_child(node, NTR_NS, "lv")?
            .map(decode_sheet_reference)
            .transpose()?,
        documents: optional_list_ns(
            node,
            NTR_NS,
            "listiny",
            COMMON_NS,
            "listina",
            decode_document,
        )?,
    })
}

fn decode_creditor_subject(node: Node<'_, '_>) -> Result<NtrCreditorSubject> {
    ensure_not_nil(node, "opravnenySubjekt")?;
    Ok(NtrCreditorSubject {
        subject: decode_subject(node)?,
        authorized_representative: optional_child(node, COMMON_NS, "opravnenaPo")?
            .map(decode_subject)
            .transpose()?,
        // This field is added by the report namespace's extension.
        claim_share: optional_text(node, NTR_NS, "podilPohledavky")?,
    })
}

fn decode_notice(node: Node<'_, '_>) -> Result<NtrNotice> {
    ensure_not_nil(node, "upozorneni")?;
    Ok(NtrNotice {
        name: required_text(node, NTR_NS, "nazev")?,
        proceeding_objects: required_list_ns(
            node,
            NTR_NS,
            "objektyRizeni",
            COMMON_NS,
            "objektRizeni",
            decode_proceeding_object,
        )?,
    })
}

fn decode_proceeding_object(node: Node<'_, '_>) -> Result<NtrProceedingObject> {
    ensure_not_nil(node, "objektRizeni")?;
    Ok(NtrProceedingObject {
        proceeding: decode_proceeding(required_child(node, COMMON_NS, "rizeni")?)?,
        parcels: optional_list(node, COMMON_NS, "parcely", "parcela", decode_parcel)?,
        buildings: optional_list(node, COMMON_NS, "stavby", "stavba", decode_building)?,
        units: optional_list(node, COMMON_NS, "jednotky", "jednotka", decode_unit)?,
        building_rights: optional_list(
            node,
            COMMON_NS,
            "pravaStavby",
            "pravoStavby",
            decode_building_right,
        )?,
    })
}

fn decode_acquisition_title(node: Node<'_, '_>) -> Result<NtrAcquisitionTitle> {
    ensure_not_nil(node, "nabyvaciTitul")?;
    Ok(NtrAcquisitionTitle {
        document: decode_document(required_child(node, NTR_NS, "listina")?)?,
        subjects: optional_list_ns(
            node,
            NTR_NS,
            "opravneneSubjekty",
            COMMON_NS,
            "opravnenySubjekt",
            decode_subject,
        )?,
    })
}

fn decode_bonity(node: Node<'_, '_>) -> Result<NtrBonity> {
    ensure_not_nil(node, "bonita")?;
    Ok(NtrBonity {
        bpej_code: required_text(node, NTR_NS, "bpejKod")?,
        area: required_bounded(node, NTR_NS, "vymera", 0, 999_999_999)?,
        parcel: decode_parcel(required_child(node, NTR_NS, "parcela")?)?,
    })
}

fn required_list<'a, 'input, T>(
    node: Node<'a, 'input>,
    namespace: &str,
    wrapper: &str,
    item: &str,
    decode: fn(Node<'a, 'input>) -> Result<T>,
) -> Result<Vec<T>> {
    required_list_ns(node, namespace, wrapper, namespace, item, decode)
}

fn required_list_ns<'a, 'input, T>(
    node: Node<'a, 'input>,
    wrapper_namespace: &str,
    wrapper: &str,
    item_namespace: &str,
    item: &str,
    decode: fn(Node<'a, 'input>) -> Result<T>,
) -> Result<Vec<T>> {
    let wrapper_node = required_child(node, wrapper_namespace, wrapper)?;
    ensure_not_nil(wrapper_node, wrapper)?;
    children(wrapper_node, item_namespace, item)
        .map(decode)
        .collect()
}

fn optional_list<'a, 'input, T>(
    node: Node<'a, 'input>,
    namespace: &str,
    wrapper: &str,
    item: &str,
    decode: fn(Node<'a, 'input>) -> Result<T>,
) -> Result<Option<Vec<T>>> {
    optional_list_ns(node, namespace, wrapper, namespace, item, decode)
}

fn optional_nonempty_list<'a, 'input, T>(
    node: Node<'a, 'input>,
    namespace: &str,
    wrapper: &str,
    item: &str,
    decode: fn(Node<'a, 'input>) -> Result<T>,
) -> Result<Option<Vec<T>>> {
    let values = optional_list(node, namespace, wrapper, item, decode)?;
    if values.as_ref().is_some_and(Vec::is_empty) {
        return Err(Error::invalid_response(format!(
            "NTR XML element {wrapper} must contain at least one {item}"
        )));
    }
    Ok(values)
}

fn optional_list_ns<'a, 'input, T>(
    node: Node<'a, 'input>,
    wrapper_namespace: &str,
    wrapper: &str,
    item_namespace: &str,
    item: &str,
    decode: fn(Node<'a, 'input>) -> Result<T>,
) -> Result<Option<Vec<T>>> {
    let Some(wrapper_node) = optional_child(node, wrapper_namespace, wrapper)? else {
        return Ok(None);
    };
    ensure_not_nil(wrapper_node, wrapper)?;
    children(wrapper_node, item_namespace, item)
        .map(decode)
        .collect::<Result<Vec<_>>>()
        .map(Some)
}

fn require_tag(node: Node<'_, '_>, namespace: &str, name: &str) -> Result<()> {
    if node.tag_name().namespace() == Some(namespace) && node.tag_name().name() == name {
        Ok(())
    } else {
        Err(Error::invalid_response(format!(
            "expected NTR XML element {{{namespace}}}{name}"
        )))
    }
}

fn child<'a, 'input>(
    node: Node<'a, 'input>,
    namespace: &str,
    name: &str,
) -> Result<Option<Node<'a, 'input>>> {
    let mut matches = children(node, namespace, name);
    let first = matches.next();
    if matches.next().is_some() {
        return Err(Error::invalid_response(format!(
            "NTR XML element {name} occurs more than once"
        )));
    }
    Ok(first)
}

fn optional_child<'a, 'input>(
    node: Node<'a, 'input>,
    namespace: &str,
    name: &str,
) -> Result<Option<Node<'a, 'input>>> {
    child(node, namespace, name)
}

fn required_child<'a, 'input>(
    node: Node<'a, 'input>,
    namespace: &str,
    name: &str,
) -> Result<Node<'a, 'input>> {
    child(node, namespace, name)?.ok_or_else(|| {
        Error::invalid_response(format!("required NTR XML element {name} is missing"))
    })
}

fn children<'a, 'input>(
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

fn ensure_not_nil(node: Node<'_, '_>, name: &str) -> Result<()> {
    if node
        .attribute((XSI_NS, "nil"))
        .is_some_and(|value| value == "true" || value == "1")
    {
        Err(Error::invalid_response(format!(
            "NTR XML element {name} cannot be nil"
        )))
    } else {
        Ok(())
    }
}

fn required_text(node: Node<'_, '_>, namespace: &str, name: &str) -> Result<String> {
    let element = required_child(node, namespace, name)?;
    ensure_not_nil(element, name)?;
    simple_content(element, name)
}

fn optional_text(node: Node<'_, '_>, namespace: &str, name: &str) -> Result<Option<String>> {
    let Some(element) = optional_child(node, namespace, name)? else {
        return Ok(None);
    };
    ensure_not_nil(element, name)?;
    simple_content(element, name).map(Some)
}

fn simple_content(node: Node<'_, '_>, name: &str) -> Result<String> {
    let mut content = String::new();
    for child in node.children() {
        if child.is_element() {
            return Err(Error::invalid_response(format!(
                "NTR XML element {name} contains a nested element"
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

fn required_parse<T: FromStr>(
    node: Node<'_, '_>,
    namespace: &str,
    name: &'static str,
) -> Result<T> {
    let raw = required_text(node, namespace, name)?;
    parse_response_value(raw.trim(), name)
}

fn optional_parse<T: FromStr>(
    node: Node<'_, '_>,
    namespace: &str,
    name: &'static str,
) -> Result<Option<T>> {
    optional_text(node, namespace, name)?
        .map(|raw| parse_response_value(raw.trim(), name))
        .transpose()
}

fn parse_response_value<T: FromStr>(raw: &str, name: &'static str) -> Result<T> {
    raw.parse().map_err(|_| {
        Error::invalid_response(format!("NTR XML element {name} has an invalid value"))
    })
}

fn required_bounded<T>(
    node: Node<'_, '_>,
    namespace: &str,
    name: &'static str,
    minimum: u64,
    maximum: u64,
) -> Result<T>
where
    T: FromStr + TryFrom<u64>,
{
    let raw = required_text(node, namespace, name)?;
    parse_bounded(&raw, name, minimum, maximum)
}

fn optional_bounded<T>(
    node: Node<'_, '_>,
    namespace: &str,
    name: &'static str,
    minimum: u64,
    maximum: u64,
) -> Result<Option<T>>
where
    T: FromStr + TryFrom<u64>,
{
    optional_text(node, namespace, name)?
        .map(|raw| parse_bounded(&raw, name, minimum, maximum))
        .transpose()
}

fn parse_bounded<T>(raw: &str, name: &'static str, minimum: u64, maximum: u64) -> Result<T>
where
    T: FromStr + TryFrom<u64>,
{
    let Some(digits) = xsd_unsigned_integer_digits(raw) else {
        return Err(Error::invalid_response(format!(
            "NTR XML element {name} must contain a decimal integer"
        )));
    };
    let number = digits
        .parse::<u64>()
        .map_err(|_| Error::invalid_response(format!("NTR XML element {name} is out of range")))?;
    if !(minimum..=maximum).contains(&number) {
        return Err(Error::invalid_response(format!(
            "NTR XML element {name} is out of range"
        )));
    }
    T::try_from(number)
        .map_err(|_| Error::invalid_response(format!("NTR XML element {name} is out of range")))
}

fn required_numeric_code<T>(
    node: Node<'_, '_>,
    name: &'static str,
    maximum: u64,
    decode: impl FnOnce(String) -> T,
) -> Result<T> {
    let raw = required_text(node, COMMON_NS, name)?;
    validate_positive_integer(&raw, maximum, name)?;
    Ok(decode(raw.trim().to_owned()))
}

fn xsd_unsigned_integer_digits(raw: &str) -> Option<&str> {
    let value = raw.trim();
    let digits = value.strip_prefix('+').unwrap_or(value);
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        None
    } else {
        Some(digits)
    }
}

fn validate_positive_integer(raw: &str, maximum: u64, name: &'static str) -> Result<()> {
    let Some(digits) = xsd_unsigned_integer_digits(raw) else {
        return Err(Error::invalid_response(format!(
            "NTR XML element {name} must contain a positive decimal integer"
        )));
    };
    let number = digits
        .parse::<u64>()
        .map_err(|_| Error::invalid_response(format!("NTR XML element {name} is out of range")))?;
    if number == 0 || number > maximum {
        return Err(Error::invalid_response(format!(
            "NTR XML element {name} is out of range"
        )));
    }
    Ok(())
}

fn validate_nonnegative_integer(raw: &str, maximum: u64, name: &'static str) -> Result<()> {
    let Some(digits) = xsd_unsigned_integer_digits(raw) else {
        return Err(Error::invalid_response(format!(
            "NTR XML element {name} must contain a non-negative decimal integer"
        )));
    };
    match digits.parse::<u64>() {
        Ok(number) if number <= maximum => {}
        _ => {
            return Err(Error::invalid_response(format!(
                "NTR XML element {name} is out of range"
            )));
        }
    }
    Ok(())
}

fn required_code<T>(
    node: Node<'_, '_>,
    namespace: &str,
    name: &'static str,
    decode: impl FnOnce(String) -> T,
) -> Result<T> {
    required_text(node, namespace, name).map(decode)
}

fn optional_code<T>(
    node: Node<'_, '_>,
    namespace: &str,
    name: &'static str,
    decode: impl FnOnce(String) -> T,
) -> Result<Option<T>> {
    optional_text(node, namespace, name).map(|value| value.map(decode))
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
                    .ok_or_else(|| Error::invalid_response("NTR XML exceeds the nesting limit"))?;
                if depth > max_depth {
                    return Err(Error::invalid_response("NTR XML exceeds the nesting limit"));
                }
            }
            Ok(Event::Empty(element)) => {
                if depth >= max_depth {
                    return Err(Error::invalid_response("NTR XML exceeds the nesting limit"));
                }
                nodes = preflight_node(nodes, max_nodes)?;
                nodes = preflight_attributes(&element, nodes, max_nodes)?;
            }
            Ok(Event::End(_)) => {
                depth = depth
                    .checked_sub(1)
                    .ok_or_else(|| Error::invalid_response("malformed NTR XML"))?;
            }
            Ok(Event::DocType(_)) => {
                return Err(Error::invalid_response("DTD is not allowed in NTR XML"));
            }
            Ok(
                Event::Text(_)
                | Event::CData(_)
                | Event::Comment(_)
                | Event::Decl(_)
                | Event::PI(_)
                | Event::GeneralRef(_),
            ) => nodes = preflight_node(nodes, max_nodes)?,
            Err(_) => return Err(Error::invalid_response("malformed NTR XML")),
        }
    }
    if depth != 0 {
        return Err(Error::invalid_response("malformed NTR XML"));
    }
    Ok(())
}

fn preflight_node(nodes: u32, maximum: u32) -> Result<u32> {
    let nodes = nodes
        .checked_add(1)
        .ok_or_else(|| Error::invalid_response("NTR XML exceeds the node limit"))?;
    if nodes > maximum {
        return Err(Error::invalid_response("NTR XML exceeds the node limit"));
    }
    Ok(nodes)
}

fn preflight_attributes(element: &BytesStart<'_>, mut nodes: u32, maximum: u32) -> Result<u32> {
    for attribute in element.attributes() {
        if attribute.is_err() {
            return Err(Error::invalid_response("malformed NTR XML"));
        }
        nodes = preflight_node(nodes, maximum)?;
    }
    Ok(nodes)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAX_NODES: u32 = 100_000;
    const MAX_DEPTH: u32 = 128;

    fn parse(xml: &str) -> Result<NtrOwnershipSheet> {
        parse_ntr_ownership_sheet(xml.as_bytes(), MAX_NODES, MAX_DEPTH)
    }

    fn document(sheets: &str, signature: &str) -> String {
        format!(
            r#"<v:vypisZKatastruNemovitosti xmlns:v="{NTR_NS}" xmlns:q="{COMMON_NS}" xmlns:sig="{DS_NS}">
              <v:obecneUdaje>
                <q:platnost>2026-11-20T10:11:12+01:00</q:platnost>
                <q:pracoviste><q:kod>100</q:kod><q:nazev>Praha</q:nazev>
                  <q:adresa><q:cisloOrientacni>12a</q:cisloOrientacni><q:cisloDomovni>123</q:cisloDomovni><q:cisloDomovniTyp>1</q:cisloDomovniTyp><q:nazevUlice>Testovací</q:nazevUlice><q:castObce>Část</q:castObce><q:mestskaCast>MČ</q:mestskaCast><q:mestskyObvod>MO</q:mestskyObvod><q:obec>Praha</q:obec><q:okres>Praha</q:okres><q:psc>01000</q:psc><q:dodPosta>Praha 1</q:dodPosta><q:stat>Česko</q:stat><q:kodAdrm>12345</q:kodAdrm></q:adresa>
                  <q:kontakt><q:telefon>123</q:telefon><q:fax>456</q:fax><q:email>a@example.cz</q:email><q:idds>abc1234</q:idds></q:kontakt>
                </q:pracoviste>
                <q:pouceni>Poučení</q:pouceni><q:uzivatel><q:uzivatelskeJmeno>user</q:uzivatelskeJmeno><q:celeJmeno>Dálkový přístup</q:celeJmeno></q:uzivatel>
                <q:vyhotoveno>2026-11-20T10:12:13Z</q:vyhotoveno><v:castecnyVypis>n</v:castecnyVypis>
                <v:neverejnaListina>Neveřejné</v:neverejnaListina><v:wsgp>WSGP</v:wsgp>
                <v:overeni><q:dolozka>Doložka</q:dolozka><q:overeno>Ověřeno</q:overeno><q:bezplatny>Bezplatně</q:bezplatny></v:overeni>
              </v:obecneUdaje>
              <v:listyVlastnictvi>{sheets}</v:listyVlastnictvi>
              <v:metadata><q:identifikatorSestavy>f814d6ba-1c0b-4f7d-b42a-f4197e584b1d</q:identifikatorSestavy><q:behId>123456789012345678901234567890</q:behId><q:dpId>30</q:dpId><q:prostredi>TEST - ISKNE</q:prostredi><q:copyright>© ČÚZK</q:copyright><q:aplikace>ReportCoreService</q:aplikace><q:verzeAplikace>10.4.0.1</q:verzeAplikace><q:kodSestavy>PU010</q:kodSestavy><q:verzeSestavy>3.0</q:verzeSestavy></v:metadata>
              {signature}
            </v:vypisZKatastruNemovitosti>"#
        )
    }

    fn empty_sections() -> String {
        "<v:jinaPrava/><v:seznamUpozorneni/><v:nabyvaciTituly/><v:bonity/>".to_owned()
    }

    fn sheet(id: u32, owners: &str, properties: &str, sections: &str) -> String {
        format!(
            r#"<v:listVlastnictvi><v:id>{id}</v:id><v:cislo>{id}</v:cislo>
              <v:okres><q:kod>3201</q:kod><q:nazev>Benešov</q:nazev><q:nuts4>CZ0201</q:nuts4></v:okres>
              <v:obec><q:kod>500135</q:kod><q:nazev>Kozlov</q:nazev></v:obec>
              <v:katastrUzemi><q:kod>600521</q:kod><q:nazev>Aš</q:nazev></v:katastrUzemi>
              <v:pracoviste><q:kod>100</q:kod><q:nazev>Praha</q:nazev></v:pracoviste>
              <v:ciselnaRada><q:kod>1</q:kod><q:nazev>Jedna řada</q:nazev></v:ciselnaRada>
              {owners}<v:nemovitosti>{properties}</v:nemovitosti>{sections}
            </v:listVlastnictvi>"#
        )
    }

    fn minimal_sheet(id: u32, relations: &str) -> String {
        let owners = if relations.is_empty() {
            "<v:vlastnici/>".to_owned()
        } else {
            format!("<v:vlastnici><v:pravniVztahy>{relations}</v:pravniVztahy></v:vlastnici>")
        };
        sheet(id, &owners, "", &empty_sections())
    }

    fn address() -> &'static str {
        "<q:adresa><q:obec>Praha</q:obec><q:psc>11000</q:psc></q:adresa>"
    }

    fn participant(element: &str, id: u32, given_name: &str) -> String {
        format!(
            "<q:{element}><q:id>{id}</q:id><q:typ>OFO</q:typ><q:jmeno>{given_name}</q:jmeno><q:prijmeni>Novák</q:prijmeni><q:titulPred>Ing.</q:titulPred><q:titulZa>Ph.D.</q:titulZa><q:rodneCislo>123456</q:rodneCislo>{}</q:{element}>",
            address()
        )
    }

    fn subject(outer_id: u32, first_id: u32, second_id: Option<u32>) -> String {
        let second = second_id
            .map(|id| participant("oS2", id, "Jana"))
            .unwrap_or_default();
        format!(
            "<q:opravnenySubjekt><q:id>{outer_id}</q:id><q:typ>BSM</q:typ><q:charKod>16</q:charKod><q:charZkratka>manželé</q:charZkratka><q:charZkratkaAlv>MCP</q:charZkratkaAlv><q:nazevSJ>SJ Novákovi</q:nazevSJ>{}{second}</q:opravnenySubjekt>",
            participant("oS", first_id, "Jan")
        )
    }

    fn subject_ownership(
        id: u32,
        parent_id: Option<u32>,
        numerator: &str,
        denominator: &str,
        target: &str,
        subordinate: &str,
    ) -> String {
        let parent = parent_id
            .map(|id| format!("<q:nadrizeneId>{id}</q:nadrizeneId>"))
            .unwrap_or_default();
        let subordinate = if subordinate.is_empty() {
            String::new()
        } else {
            format!("<v:podrizenePravniVztahy>{subordinate}</v:podrizenePravniVztahy>")
        };
        format!(
            "<v:vlastnictvi><q:id>{id}</q:id>{parent}<q:podil><q:citatel>{numerator}</q:citatel><q:jmenovatel>{denominator}</q:jmenovatel></q:podil>{target}{subordinate}</v:vlastnictvi>"
        )
    }

    fn relation(code: &str, name: &str, records: &str) -> String {
        format!(
            "<v:pravniVztah><v:kod>{code}</v:kod><v:nazev>{name}</v:nazev><v:seznamVlastnictvi>{records}</v:seznamVlastnictvi></v:pravniVztah>"
        )
    }

    fn one_subject_report(numerator: &str, denominator: &str) -> String {
        let target = subject(10, 11, None);
        let ownership = subject_ownership(1, None, numerator, denominator, &target, "");
        document(
            &minimal_sheet(1, &relation("30", "Vlastnické právo", &ownership)),
            "",
        )
    }

    fn cadastral_area() -> &'static str {
        "<q:katastrUzemi><q:kod>600521</q:kod><q:nazev>Aš</q:nazev></q:katastrUzemi>"
    }

    fn parcel(id: u32) -> String {
        format!(
            "<q:parcela><q:id>{id}</q:id><q:typ>PKN</q:typ><q:kmenCis>1</q:kmenCis><q:vymera>100</q:vymera>{}</q:parcela>",
            cadastral_area()
        )
    }

    fn building(id: u32) -> String {
        format!("<q:stavba><q:id>{id}</q:id>{}</q:stavba>", cadastral_area())
    }

    fn building_right(id: u32) -> String {
        format!(
            "<q:pravoStavby><q:id>{id}</q:id>{}</q:pravoStavby>",
            cadastral_area()
        )
    }

    fn unit(id: u32) -> String {
        format!(
            "<q:jednotka><q:id>{id}</q:id><q:cisloJednotky>1001</q:cisloJednotky><q:cislo>1</q:cislo><q:podil><q:citatel>1</q:citatel><q:jmenovatel>10</q:jmenovatel></q:podil><q:typ><q:kod>1</q:kod><q:nazev>Byt</q:nazev></q:typ></q:jednotka>"
        )
    }

    #[test]
    fn parses_multiple_sheets_and_distinguishes_optional_empty_wrappers() {
        let first = sheet(
            1,
            "<v:vlastnici/>",
            "<v:parcely/><v:stavby/>",
            &empty_sections(),
        );
        let second = minimal_sheet(2, "");
        let report = parse(&document(&format!("{first}{second}"), "")).unwrap();
        assert_eq!(report.ownership_sheets.len(), 2);
        assert_eq!(report.ownership_sheets[0].id.as_str(), "1");
        assert_eq!(report.ownership_sheets[1].number, 2);
        assert!(report.ownership_sheets[0].owner_relations.is_none());
        assert_eq!(
            report.ownership_sheets[0]
                .properties
                .parcels
                .as_ref()
                .map(Vec::len),
            Some(0)
        );
        assert_eq!(
            report.ownership_sheets[0]
                .properties
                .buildings
                .as_ref()
                .map(Vec::len),
            Some(0)
        );
        assert!(report.ownership_sheets[0].properties.units.is_none());
        assert!(report.ownership_sheets[1].properties.parcels.is_none());
        assert_eq!(
            report
                .general_data
                .workplace
                .as_ref()
                .unwrap()
                .workplace
                .code
                .as_str(),
            "100"
        );
        assert_eq!(
            report
                .general_data
                .workplace
                .as_ref()
                .unwrap()
                .address
                .as_ref()
                .unwrap()
                .postal_code
                .as_deref(),
            Some("01000")
        );
        assert_eq!(
            report
                .general_data
                .verification
                .as_ref()
                .unwrap()
                .clause
                .as_deref(),
            Some("Doložka")
        );
        assert_eq!(report.metadata.run_id.as_ref().unwrap().as_str().len(), 30);
        assert_eq!(
            report.metadata.processing_id.as_ref().unwrap().as_str(),
            "30"
        );
    }

    #[test]
    fn bsm_accepts_one_or_two_participants_and_preserves_outer_and_nested_ids() {
        let one = subject_ownership(1, None, "1", "2", &subject(10, 11, None), "");
        let two = subject_ownership(2, None, "1", "2", &subject(20, 21, Some(22)), "");
        let xml = document(
            &minimal_sheet(
                1,
                &relation("30", "Vlastnické právo", &format!("{one}{two}")),
            ),
            "",
        );
        let report = parse(&xml).unwrap();
        let records =
            &report.ownership_sheets[0].owner_relations.as_ref().unwrap()[0].ownership_records;
        let first = match &records[0].ownership.target {
            NtrOwnershipTarget::Subject(subject) => subject,
            _ => panic!("expected a subject"),
        };
        let second = match &records[1].ownership.target {
            NtrOwnershipTarget::Subject(subject) => subject,
            _ => panic!("expected a subject"),
        };
        assert_eq!(first.id.as_str(), "10");
        assert_eq!(first.primary.id.as_str(), "11");
        assert!(first.secondary.is_none());
        assert_eq!(second.id.as_str(), "20");
        assert_eq!(second.primary.id.as_str(), "21");
        assert_eq!(second.secondary.as_ref().unwrap().id.as_str(), "22");
        assert!(!format!("{:?}", second.primary.birth_number).contains("123456"));
    }

    #[test]
    fn recursive_relations_retain_structure_independently_of_parent_ids() {
        let grandchild = relation(
            "9999",
            "Budoucí vztah",
            &subject_ownership(3, Some(2), "1", "3", &subject(30, 31, None), ""),
        );
        let child = relation(
            "32",
            "Zástavní právo smluvní",
            &subject_ownership(2, None, "1", "2", &subject(20, 21, None), &grandchild),
        );
        let parent = subject_ownership(1, None, "1", "1", &subject(10, 11, None), &child);
        let xml = document(
            &minimal_sheet(1, &relation("30", "Vlastnické právo", &parent)),
            "",
        );
        let report = parse(&xml).unwrap();
        let parent =
            &report.ownership_sheets[0].owner_relations.as_ref().unwrap()[0].ownership_records[0];
        assert!(parent.ownership.parent_id.is_none());
        let child = &parent.subordinate_relations.as_ref().unwrap()[0].ownership_records[0];
        assert!(child.ownership.parent_id.is_none());
        let grandchild_relation = &child.subordinate_relations.as_ref().unwrap()[0];
        assert_eq!(
            grandchild_relation.code,
            LegalRelationCode::Other("9999".to_owned())
        );
        assert_eq!(grandchild_relation.is_ownership(), None);
        assert_eq!(
            grandchild_relation.ownership_records[0]
                .ownership
                .parent_id
                .as_ref()
                .unwrap()
                .as_str(),
            "2"
        );
    }

    #[test]
    fn present_relation_wrappers_require_at_least_one_item() {
        let empty_owners = sheet(
            1,
            "<v:vlastnici><v:pravniVztahy/></v:vlastnici>",
            "",
            &empty_sections(),
        );
        assert!(parse(&document(&empty_owners, "")).is_err());

        let ownership = subject_ownership(1, None, "1", "1", &subject(10, 11, None), "").replace(
            "</v:vlastnictvi>",
            "<v:podrizenePravniVztahy/></v:vlastnictvi>",
        );
        let xml = document(
            &minimal_sheet(1, &relation("30", "Vlastnické právo", &ownership)),
            "",
        );
        assert!(parse(&xml).is_err());
    }

    #[test]
    fn ownership_requires_exactly_one_target() {
        for target in [
            String::new(),
            format!("{}{}", subject(10, 11, None), parcel(12)),
        ] {
            let ownership = subject_ownership(1, None, "1", "1", &target, "");
            let xml = document(
                &minimal_sheet(1, &relation("30", "Vlastnické právo", &ownership)),
                "",
            );
            assert!(parse(&xml).is_err());
        }
        let ownership = subject_ownership(1, None, "1", "1", &parcel(12), "");
        let xml = document(
            &minimal_sheet(1, &relation("221", "Přídatné spoluvlastnictví", &ownership)),
            "",
        );
        let report = parse(&xml).unwrap();
        assert!(matches!(
            report.ownership_sheets[0].owner_relations.as_ref().unwrap()[0].ownership_records[0]
                .ownership
                .target,
            NtrOwnershipTarget::Parcel(_)
        ));
    }

    #[test]
    fn missing_required_ids_shares_and_relation_fields_are_rejected() {
        let valid = one_subject_report("1", "2");
        for needle in [
            "<v:id>1</v:id>",
            "<q:jmenovatel>2</q:jmenovatel>",
            "<v:kod>30</v:kod>",
            "<v:nazev>Vlastnické právo</v:nazev>",
            "<q:id>1</q:id>",
        ] {
            assert!(valid.contains(needle), "missing test needle {needle}");
            let invalid = valid.replacen(needle, "", 1);
            assert!(parse(&invalid).is_err(), "removing {needle} must fail");
        }
    }

    #[test]
    fn share_decimals_are_exact_and_reject_invalid_or_oversized_lexemes() {
        let maximum = "79228162514264337593543950335";
        let report = parse(&one_subject_report(&format!("+{maximum}"), "+2")).unwrap();
        let share = &report.ownership_sheets[0].owner_relations.as_ref().unwrap()[0]
            .ownership_records[0]
            .ownership
            .share;
        assert_eq!(share.numerator(), Decimal::from_str(maximum).unwrap());
        assert_eq!(share.denominator(), Decimal::TWO);

        for oversized in [
            "79228162514264337593543950336",
            "999999999999999999999999999999",
        ] {
            let error = match parse(&one_subject_report(oversized, "2")) {
                Err(error) => error,
                Ok(_) => panic!("oversized decimal must not be rounded"),
            };
            assert!(matches!(
                error,
                Error::DecimalOutOfRange {
                    field: "NTR share numerator"
                }
            ));
        }
        for malformed in ["1e2", "1_0", "1.0", "++1", "+0", "0"] {
            let error = match parse(&one_subject_report(malformed, "2")) {
                Err(error) => error,
                Ok(_) => panic!("invalid positiveInteger {malformed} was accepted"),
            };
            assert!(matches!(error, Error::InvalidResponse { .. }));
        }
    }

    #[test]
    fn xsd_integers_accept_one_leading_plus_and_apply_numeric_ranges() {
        for raw in ["1", "+1", "  +0001\t"] {
            assert_eq!(parse_bounded::<u16>(raw, "test", 1, 999).unwrap(), 1);
            validate_positive_integer(raw, 999, "test").unwrap();
            validate_nonnegative_integer(raw, 999, "test").unwrap();
        }
        assert_eq!(parse_bounded::<u16>("+0", "test", 0, 999).unwrap(), 0);
        validate_nonnegative_integer("+0", 999, "test").unwrap();
        assert!(validate_positive_integer("+0", 999, "test").is_err());
        assert!(parse_bounded::<u16>("+1000", "test", 1, 999).is_err());
        for raw in ["", "+", "++1", "+-1", "-1", "1+", "+ 1"] {
            assert!(parse_bounded::<u16>(raw, "test", 0, 999).is_err());
        }
    }

    #[test]
    fn signed_ntr_ids_retain_their_lexical_form_and_digit_limit() {
        let raw = "123456789012345678901234567890";
        let xml = document(&minimal_sheet(1, ""), "")
            .replace(
                "<q:behId>123456789012345678901234567890</q:behId>",
                &format!("<q:behId>+{raw}</q:behId>"),
            )
            .replace("<q:dpId>30</q:dpId>", "<q:dpId>+0030</q:dpId>");
        let report = parse(&xml).unwrap();
        assert_eq!(
            report.metadata.run_id.as_ref().unwrap().as_str(),
            format!("+{raw}")
        );
        assert_eq!(
            report.metadata.processing_id.as_ref().unwrap().as_str(),
            "+0030"
        );
        assert_eq!(
            NtrOtherRightGroupId::new("+0001").unwrap().as_str(),
            "+0001"
        );
        assert!(NtrReportRunId::new(format!("+{raw}0")).is_err());
        for raw in ["", "+", "++1", "+0", "-1"] {
            assert!(NtrReportRunId::new(raw).is_err());
        }
    }

    #[test]
    fn unknown_dynamic_and_closed_codes_are_retained() {
        let target = subject(10, 11, None)
            .replace("<q:typ>BSM</q:typ>", "<q:typ>FUTURE</q:typ>")
            .replace("<q:charKod>16</q:charKod>", "<q:charKod>99</q:charKod>");
        let ownership = subject_ownership(1, None, "1", "1", &target, "");
        let xml = one_subject_report("1", "1")
            .replace(
                &subject_ownership(1, None, "1", "1", &subject(10, 11, None), ""),
                &ownership,
            )
            .replace("<v:kod>30</v:kod>", "<v:kod>abcd</v:kod>")
            .replace(
                "<v:castecnyVypis>n</v:castecnyVypis>",
                "<v:castecnyVypis>future</v:castecnyVypis>",
            );
        let report = parse(&xml).unwrap();
        assert_eq!(
            report.general_data.is_partial,
            NtrYesNo::Other("future".to_owned())
        );
        let relation = &report.ownership_sheets[0].owner_relations.as_ref().unwrap()[0];
        assert_eq!(relation.code, LegalRelationCode::Other("abcd".to_owned()));
        let NtrOwnershipTarget::Subject(subject) = &relation.ownership_records[0].ownership.target
        else {
            panic!("expected a subject");
        };
        assert_eq!(subject.kind, NtrSubjectKind::Other("FUTURE".to_owned()));
        assert_eq!(
            subject.characteristic_code,
            NtrSubjectCharacteristicCode::Other("99".to_owned())
        );
    }

    #[test]
    fn optional_signature_is_retained_verbatim() {
        let without = parse(&document(&minimal_sheet(1, ""), "")).unwrap();
        assert!(without.signature.is_none());
        let signature = "<sig:Signature Id=\"sig-1\"><sig:SignedInfo><xades:QualifyingProperties xmlns:xades=\"http://uri.etsi.org/01903/v1.3.2#\"/></sig:SignedInfo></sig:Signature>";
        let with = parse(&document(&minimal_sheet(1, ""), signature)).unwrap();
        assert_eq!(with.signature.unwrap().raw_xml, signature);
        let wrong_namespace = document(&minimal_sheet(1, ""), "");
        assert!(parse(&wrong_namespace.replace(NTR_NS, "urn:example:wrong")).is_err());
    }

    #[test]
    fn node_depth_and_dtd_limits_are_enforced() {
        let xml = one_subject_report("1", "1");
        assert!(parse_ntr_ownership_sheet(xml.as_bytes(), 2, MAX_DEPTH).is_err());
        assert!(parse_ntr_ownership_sheet(xml.as_bytes(), MAX_NODES, 2).is_err());
        let dtd = format!(
            "<!DOCTYPE v:vypisZKatastruNemovitosti [<!ENTITY xxe SYSTEM 'file:///etc/passwd'>]>{xml}"
        );
        assert!(parse(&dtd).is_err());
    }

    fn proceeding(id: u32) -> String {
        format!(
            "<q:rizeni><q:id>{id}</q:id><q:typ><q:kod>XX</q:kod></q:typ><q:poradoveCislo>123</q:poradoveCislo><q:rok>2026</q:rok><q:pracoviste><q:kod>100</q:kod><q:nazev>Praha</q:nazev></q:pracoviste><q:prijato>2026-01-01T01:02:03Z</q:prijato><q:zplatneno>2026-01-02</q:zplatneno><q:extIdentifikace>external</q:extIdentifikace></q:rizeni>"
        )
    }

    fn document_record(id: u32) -> String {
        format!(
            "<q:listina><q:formatText>Listina {id}</q:formatText><q:id>{id}</q:id><q:vyhotoveno>2026-01-01</q:vyhotoveno><q:pravniMoc>2026-01-02</q:pravniMoc><q:vykonatelnost>2026-01-03</q:vykonatelnost><q:pvz>PVZ</q:pvz><q:typListiny><q:kod>999</q:kod><q:nazev>Budoucí listina</q:nazev></q:typListiny><q:popis>Popis</q:popis><q:dalsiUdaje><q:dalsiUdaj><q:kod>999999</q:kod><q:nazev>Detail</q:nazev></q:dalsiUdaj></q:dalsiUdaje><q:zkratka>Z</q:zkratka><q:zhotovitel>Zhotovitel</q:zhotovitel><q:poradoveCislo>000001</q:poradoveCislo><q:rokVyhotoveni>2026</q:rokVyhotoveni><q:doplnek>Doplněk</q:doplnek><q:pracoviste><q:kod>100</q:kod></q:pracoviste>{}</q:listina>",
            proceeding(70)
        )
    }

    #[test]
    fn all_xml_dates_accept_and_retain_utc_and_numeric_offsets() {
        for suffix in ["Z", "+01:00"] {
            let record = document_record(60)
                .replacen(
                    "<q:listina>",
                    &format!("<q:listina xmlns:q=\"{COMMON_NS}\">"),
                    1,
                )
                .replace(
                    "<q:vyhotoveno>2026-01-01</q:vyhotoveno>",
                    &format!("<q:vyhotoveno>2026-01-01{suffix}</q:vyhotoveno>"),
                )
                .replace(
                    "<q:pravniMoc>2026-01-02</q:pravniMoc>",
                    &format!("<q:pravniMoc>2026-01-02{suffix}</q:pravniMoc>"),
                )
                .replace(
                    "<q:vykonatelnost>2026-01-03</q:vykonatelnost>",
                    &format!("<q:vykonatelnost>2026-01-03{suffix}</q:vykonatelnost>"),
                )
                .replace(
                    "<q:zplatneno>2026-01-02</q:zplatneno>",
                    &format!("<q:zplatneno>2026-01-02{suffix}</q:zplatneno>"),
                );
            let tree = Document::parse(&record).unwrap();
            let parsed = decode_document(tree.root_element()).unwrap();
            assert_eq!(
                parsed.created_on.as_ref().unwrap().as_str(),
                format!("2026-01-01{suffix}")
            );
            assert_eq!(
                parsed.legal_force_on.as_ref().unwrap().as_str(),
                format!("2026-01-02{suffix}")
            );
            assert_eq!(
                parsed.enforceability_on.as_ref().unwrap().as_str(),
                format!("2026-01-03{suffix}")
            );
            assert_eq!(
                parsed.proceeding.validated_on.as_ref().unwrap().as_str(),
                format!("2026-01-02{suffix}")
            );

            let right = building_right(3)
                .replacen(
                    "<q:pravoStavby>",
                    &format!("<q:pravoStavby xmlns:q=\"{COMMON_NS}\">"),
                    1,
                )
                .replace(
                    "<q:id>3</q:id>",
                    &format!("<q:id>3</q:id><q:datumUkonceni>2030-01-01{suffix}</q:datumUkonceni>"),
                );
            let tree = Document::parse(&right).unwrap();
            let parsed = decode_building_right(tree.root_element()).unwrap();
            assert_eq!(
                parsed.ended_on.as_ref().unwrap().as_str(),
                format!("2030-01-01{suffix}")
            );
        }
    }

    #[test]
    fn complete_property_proceeding_and_auxiliary_graph_uses_expanded_names() {
        let full_parcel = parcel(1)
            .replace("<q:id>1</q:id>", "<q:id>1</q:id><q:budId>2</q:budId><q:budIdent>a</q:budIdent><q:psId>3</q:psId><q:psIdent>n</q:psIdent>")
            .replace("<q:typ>PKN</q:typ>", "<q:typ>NEW</q:typ><q:charakter><q:kod>9</q:kod><q:nazev>Charakter</q:nazev><q:zkratka>CH</q:zkratka></q:charakter>")
            .replace("<q:kmenCis>1</q:kmenCis>", "<q:kmenCis>1</q:kmenCis><q:poddCis>2</q:poddCis><q:plomba>P</q:plomba><q:dil>1</q:dil>")
            .replace("<q:vymera>100</q:vymera>", "<q:vymera>100</q:vymera><q:zdrojParcel><q:kod>9</q:kod><q:nazev>Zdroj</q:nazev><q:zkratka>ZD</q:zkratka></q:zdrojParcel><q:druhCislovani><q:kod>9</q:kod><q:nazev>Druh</q:nazev><q:zkratka>D</q:zkratka></q:druhCislovani><q:druhPozemku><q:kod>99</q:kod><q:nazev>Pozemek</q:nazev></q:druhPozemku><q:zpusobVyuziti><q:kod>9999</q:kod><q:nazev>Využití</q:nazev><q:zkratka>V</q:zkratka></q:zpusobVyuziti><q:zpusobyOchrany><q:zpusobOchrany><q:kod>9999</q:kod><q:nazev>Ochrana</q:nazev></q:zpusobOchrany></q:zpusobyOchrany>")
            .replace("</q:parcela>", &format!("<q:katastrUzemiPuv>{}</q:katastrUzemiPuv><q:lv><q:id>90</q:id><q:cislo>90</q:cislo></q:lv><q:soucasti>a</q:soucasti>{}</q:parcela>", "<q:kod>600521</q:kod><q:nazev>Aš</q:nazev>", building(2)));
        let full_building = building(2)
            .replace("<q:id>2</q:id>", "<q:id>2</q:id><q:parId>1</q:parId><q:psId>3</q:psId><q:cislaDomovni>1, 2</q:cislaDomovni><q:docasna>n</q:docasna><q:plomba>P</q:plomba><q:castObce><q:kod>19</q:kod><q:nazev>Abertamy</q:nazev></q:castObce><q:typBudovy><q:kod>9</q:kod><q:nazev>Typ</q:nazev><q:zkratka>T</q:zkratka></q:typBudovy><q:zpusobVyuziti><q:kod>9999</q:kod><q:nazev>Využití</q:nazev><q:zkratka>V</q:zkratka></q:zpusobVyuziti><q:zpusobyOchrany/>")
            .replace("</q:stavba>", &format!("<q:lv><q:id>91</q:id><q:cislo>91</q:cislo></q:lv><q:parcely/><q:jednotky/><q:soucasti nem=\"P\">{}</q:soucasti></q:stavba>", parcel(4)));
        let full_right = building_right(3)
            .replace("<q:id>3</q:id>", "<q:id>3</q:id><q:datumUkonceni>2030-01-01</q:datumUkonceni><q:plomba>P</q:plomba>")
            .replace("</q:pravoStavby>", "<q:lv><q:id>92</q:id><q:cislo>92</q:cislo></q:lv><q:parcely/><q:stavby/><q:ucely><q:ucel><q:kod>99</q:kod><q:nazev>Účel</q:nazev></q:ucel></q:ucely><q:zpusobyOchrany/></q:pravoStavby>");
        let coowner = subject_ownership(8, None, "1", "4", &subject(80, 81, None), "")
            .replace("v:vlastnictvi", "q:vlastnictvi");
        let full_unit = unit(4)
            .replace("<q:cisloJednotky>1001</q:cisloJednotky>", "<q:cisloJednotky>1001</q:cisloJednotky><q:cisloDomovni>10</q:cisloDomovni>")
            .replace("<q:cislo>1</q:cislo>", "<q:cislo>1</q:cislo><q:popis>Jednotka</q:popis><q:plomba>P</q:plomba><q:lv><q:id>93</q:id><q:cislo>93</q:cislo></q:lv>")
            .replace("<q:nazev>Byt</q:nazev>", "<q:nazev>Byt</q:nazev><q:zkratka>B</q:zkratka>")
            .replace("</q:jednotka>", &format!("<q:zpusobVyuziti><q:kod>9999</q:kod><q:nazev>Využití</q:nazev><q:zkratka>V</q:zkratka></q:zpusobVyuziti><q:zpusobyOchrany/><q:seznamVlastnictvi>{coowner}</q:seznamVlastnictvi><q:parcely/>{}<q:stavby/><q:pravaStavby/></q:jednotka>", building(5)));
        let properties = format!(
            "<v:parcely>{full_parcel}</v:parcely><v:stavby>{full_building}</v:stavby><v:pravaStavby>{full_right}</v:pravaStavby><v:jednotky>{full_unit}</v:jednotky><v:parcelyZe>{}</v:parcelyZe>",
            parcel(6).replace("<q:typ>PKN</q:typ>", "<q:typ>PZE</q:typ>")
        );
        let creditor = subject(40, 41, None)
            .replace(
                "</q:opravnenySubjekt>",
                &format!(
                    "{}<v:podilPohledavky>1/5</v:podilPohledavky></q:opravnenySubjekt>",
                    subject(42, 43, None).replace("q:opravnenySubjekt", "q:opravnenaPo")
                ),
            )
            .replace("q:opravnenySubjekt", "v:opravnenySubjekt");
        let other_right = format!(
            "<v:jinePravo><v:sekce><q:kod>9</q:kod><q:nazev>Budoucí sekce</q:nazev><q:zkratka>X</q:zkratka></v:sekce><v:hjpvId>50</v:hjpvId><v:nadrizHjpvId>49</v:nadrizHjpvId><v:pravniVztah><q:kod>abcd</q:kod><q:nazev>Neznámý</q:nazev></v:pravniVztah><v:poradi><q:cas>2026-01-01T00:00:00Z</q:cas><q:text>První</q:text></v:poradi><v:souvisejiciJpv>a</v:souvisejiciJpv><v:popis>JPV</v:popis><v:jpvPro><v:opravneneSubjekty>{creditor}</v:opravneneSubjekty><v:parcely/><v:stavby/><v:jednotky/><v:pravaStavby/><v:lv><q:id>94</q:id><q:cislo>94</q:cislo></v:lv><v:listiny>{}</v:listiny></v:jpvPro><v:jpvK/><v:jpv/></v:jinePravo>",
            document_record(60)
        );
        let notice = format!(
            "<v:upozorneni><v:nazev>Plomba</v:nazev><v:objektyRizeni><q:objektRizeni>{}<q:parcely/><q:stavby/><q:jednotky/><q:pravaStavby/></q:objektRizeni></v:objektyRizeni></v:upozorneni>",
            proceeding(71)
        );
        let acquisition = format!(
            "<v:nabyvaciTitul>{}<v:opravneneSubjekty>{}</v:opravneneSubjekty></v:nabyvaciTitul>",
            document_record(61).replace("q:listina", "v:listina"),
            subject(44, 45, None)
        );
        let bonity = format!(
            "<v:bonita><v:bpejKod>nebonitováno</v:bpejKod><v:vymera>25</v:vymera>{}</v:bonita>",
            parcel(7).replace("q:parcela", "v:parcela")
        );
        let sections = format!(
            "<v:jinaPrava>{other_right}</v:jinaPrava><v:seznamUpozorneni>{notice}</v:seznamUpozorneni><v:nabyvaciTituly>{acquisition}</v:nabyvaciTituly><v:bonity>{bonity}</v:bonity>"
        );
        let xml = document(&sheet(1, "<v:vlastnici/>", &properties, &sections), "");
        let report = parse(&xml).unwrap();
        let sheet = &report.ownership_sheets[0];
        let properties = &sheet.properties;
        assert_eq!(properties.parcels.as_ref().unwrap().len(), 1);
        assert_eq!(properties.buildings.as_ref().unwrap().len(), 1);
        assert_eq!(properties.building_rights.as_ref().unwrap().len(), 1);
        assert_eq!(properties.units.as_ref().unwrap().len(), 1);
        assert_eq!(
            properties.simplified_parcels.as_ref().unwrap()[0].kind,
            NtrParcelKind::Simplified
        );
        let parsed_parcel = &properties.parcels.as_ref().unwrap()[0];
        assert_eq!(parsed_parcel.kind, NtrParcelKind::Other("NEW".to_owned()));
        assert!(parsed_parcel.building.is_some());
        assert_eq!(parsed_parcel.protection_methods.as_ref().unwrap().len(), 1);
        assert!(matches!(
            properties.buildings.as_ref().unwrap()[0]
                .part_of
                .as_ref()
                .unwrap()
                .target,
            NtrBuildingPartTarget::Parcel(_)
        ));
        assert_eq!(
            properties.units.as_ref().unwrap()[0]
                .co_ownership
                .as_ref()
                .unwrap()
                .len(),
            1
        );
        let right = &sheet.other_rights[0];
        assert_eq!(
            right.section.code,
            NtrOtherRightSectionCode::Other("9".to_owned())
        );
        let relations = right.for_relation.as_ref().unwrap();
        assert_eq!(
            relations.subjects.as_ref().unwrap()[0]
                .claim_share
                .as_deref(),
            Some("1/5")
        );
        assert!(
            relations.subjects.as_ref().unwrap()[0]
                .authorized_representative
                .is_some()
        );
        assert_eq!(
            relations.documents.as_ref().unwrap()[0]
                .proceeding
                .id
                .as_str(),
            "70"
        );
        assert_eq!(
            relations.documents.as_ref().unwrap()[0]
                .details
                .as_ref()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            sheet.notices[0].proceeding_objects[0]
                .proceeding
                .id
                .as_str(),
            "71"
        );
        assert_eq!(
            sheet.acquisition_titles[0].subjects.as_ref().unwrap().len(),
            1
        );
        assert_eq!(sheet.bonities[0].parcel.id.as_str(), "7");
    }
}
