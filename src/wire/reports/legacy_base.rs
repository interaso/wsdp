use std::str::FromStr;

use roxmltree::Node;
use rust_decimal::Decimal;

use crate::error::{Error, Result};
use crate::model::reports::legacy_base::*;

const XSI_NS: &str = "http://www.w3.org/2001/XMLSchema-instance";

/// Parses a legacy `ID_TYPE` without numeric conversion or a digit limit.
///
/// Empty text is valid and remains distinguishable from an absent element by
/// the caller's `Option`. XML whitespace around a positive integer is accepted
/// as required by `xs:positiveInteger`, while the original text is retained.
pub(crate) fn parse_legacy_id(node: Node<'_, '_>) -> Result<LegacyId> {
    LegacyId::new(simple_text(node)?)
        .map_err(|_| invalid_value("ID_TYPE", "expected empty text or a positive integer"))
}

/// Parses `AdresaCpCeType`, including its present-empty value.
pub(crate) fn parse_house_number_kind(node: Node<'_, '_>) -> Result<HouseNumberKind> {
    simple_text(node).map(HouseNumberKind::from_raw)
}

/// Parses `AdresaType`, preserving repeated sequence groups and their order.
pub(crate) fn parse_address(node: Node<'_, '_>) -> Result<Address> {
    const FIELDS: &[&str] = &[
        "adresni_misto",
        "nazev_ulice",
        "cp_ce",
        "cislo_domovni",
        "cislo_orientacni",
        "cast_obce",
        "obec",
        "mestska_cast",
        "mestsky_obvod",
        "psc",
        "dod_posta",
        "stat",
    ];

    require_legacy_element(node)?;
    let mut address = Address::default();
    let mut previous = None;
    for child in element_children(node) {
        let index = field_index(child, FIELDS)?;
        if previous.is_none_or(|previous| index <= previous) {
            address.groups.push(AddressGroup::default());
        }
        previous = Some(index);
        let group = address
            .groups
            .last_mut()
            .expect("a group was just inserted");
        match index {
            0 => group.address_point_id = Some(parse_legacy_id(child)?),
            1 => group.street_name = Some(simple_text(child)?),
            2 => group.house_number_kind = Some(parse_house_number_kind(child)?),
            3 => group.house_number = Some(simple_text(child)?),
            4 => group.orientation_number = Some(simple_text(child)?),
            5 => group.municipality_part = Some(simple_text(child)?),
            6 => group.municipality = Some(simple_text(child)?),
            7 => group.city_district = Some(simple_text(child)?),
            8 => group.city_borough = Some(simple_text(child)?),
            9 => group.postal_code = Some(simple_text(child)?),
            10 => group.delivery_post_office = Some(simple_text(child)?),
            11 => group.country_name = Some(simple_text(child)?),
            _ => unreachable!("field_index returned an in-range index"),
        }
    }
    Ok(address)
}

/// Parses `NazevListType`, retaining the difference between empty and complete.
pub(crate) fn parse_deed_name(node: Node<'_, '_>) -> Result<DeedName> {
    const FIELDS: &[&str] = &[
        "tlist",
        "dalsi_udaje",
        "por_cis_zhot",
        "popis",
        "vystav_txt",
        "prav_moc_txt",
        "podani_txt",
        "vykonatelnost_txt",
        "podani_z_txt",
        "zplatneni_txt",
        "prares_nazev",
    ];

    let mut cursor = SequenceCursor::new(node, FIELDS)?;
    let mut values: Vec<Option<String>> = (0..FIELDS.len()).map(|_| None).collect();
    let mut has_children = false;
    for child in element_children(node) {
        has_children = true;
        let index = cursor.singleton(child)?;
        values[index] = Some(simple_text(child)?);
    }
    if !has_children {
        return Ok(DeedName { details: None });
    }

    let mut values = values.into_iter();
    let mut required = |index: usize| {
        values
            .next()
            .flatten()
            .ok_or_else(|| invalid_value(FIELDS[index], "required legacy element is missing"))
    };
    let details = DeedNameDetails {
        deed_type: required(0)?,
        additional_data: required(1)?,
        preparation_serial_number: required(2)?,
        description: required(3)?,
        issuance_text: required(4)?,
        legal_force_text: required(5)?,
        submission_text: required(6)?,
        enforceability_text: required(7)?,
        submission_z_text: required(8)?,
        effectiveness_text: required(9)?,
        handling_workplace_name: values.next().flatten(),
    };
    Ok(DeedName {
        details: Some(details),
    })
}

/// Parses the presence-preserving `ParcelaUpType` wrapper.
pub(crate) fn parse_parcel_wrapper(node: Node<'_, '_>) -> Result<ParcelWrapper> {
    let mut cursor = SequenceCursor::new(node, &["parcela"])?;
    let mut parcel = None;
    for child in element_children(node) {
        cursor.singleton(child)?;
        parcel = Some(parse_parcel(child)?);
    }
    Ok(ParcelWrapper { parcel })
}

/// Parses `ParcelaType`, including exact signed-byte validation for its part.
pub(crate) fn parse_parcel(node: Node<'_, '_>) -> Result<Parcel> {
    const FIELDS: &[&str] = &[
        "id",
        "zkratka",
        "druh_cis",
        "par_cis",
        "podd_cis",
        "dil_parcely",
        "tel_id",
        "cislo_tel",
        "ku_nazev",
        "nazev_puv",
    ];

    let mut cursor = SequenceCursor::new(node, FIELDS)?;
    let mut parcel = Parcel::default();
    for child in element_children(node) {
        match cursor.singleton(child)? {
            0 => parcel.id = Some(parse_legacy_id(child)?),
            1 => parcel.kind_abbreviation = Some(simple_text(child)?),
            2 => parcel.numbering_kind = Some(simple_text(child)?),
            3 => parcel.number = Some(simple_text(child)?),
            4 => parcel.subdivision = Some(simple_text(child)?),
            5 => parcel.part = Some(parse_integer(child, "dil_parcely")?),
            6 => parcel.ownership_sheet_id = Some(parse_legacy_id(child)?),
            7 => parcel.ownership_sheet_number = Some(simple_text(child)?),
            8 => parcel.cadastral_area_name = Some(simple_text(child)?),
            9 => parcel.original_cadastral_area_name = Some(simple_text(child)?),
            _ => unreachable!("field_index returned an in-range index"),
        }
    }
    Ok(parcel)
}

/// Parses the presence-preserving `StavbaUpType` wrapper.
pub(crate) fn parse_building_wrapper(node: Node<'_, '_>) -> Result<BuildingWrapper> {
    let mut cursor = SequenceCursor::new(node, &["stavba"])?;
    let mut building = None;
    for child in element_children(node) {
        cursor.singleton(child)?;
        building = Some(parse_building(child)?);
    }
    Ok(BuildingWrapper { building })
}

/// Parses `StavbaType`, including the optional `par_ident` wrapper.
pub(crate) fn parse_building(node: Node<'_, '_>) -> Result<Building> {
    const FIELDS: &[&str] = &[
        "id",
        "caobce",
        "typbud_zkr",
        "cisla_domovni",
        "vyuziti_zkr",
        "tel_id",
        "bez_lv",
        "cislo_tel",
        "ku_nazev",
        "docasna",
        "par_ident",
    ];

    let mut cursor = SequenceCursor::new(node, FIELDS)?;
    let mut building = Building::default();
    for child in element_children(node) {
        match cursor.singleton(child)? {
            0 => building.id = Some(parse_legacy_id(child)?),
            1 => building.municipality_part = Some(simple_text(child)?),
            2 => building.building_type_abbreviation = Some(simple_text(child)?),
            3 => building.house_numbers = Some(simple_text(child)?),
            4 => building.use_abbreviation = Some(simple_text(child)?),
            5 => building.ownership_sheet_id = Some(parse_legacy_id(child)?),
            6 => building.without_ownership_sheet = Some(simple_text(child)?),
            7 => building.ownership_sheet_number = Some(simple_text(child)?),
            8 => building.cadastral_area_name = Some(simple_text(child)?),
            9 => building.temporary = Some(simple_text(child)?),
            10 => building.identifying_parcels = Some(parse_identifying_parcels(child)?),
            _ => unreachable!("field_index returned an in-range index"),
        }
    }
    Ok(building)
}

fn parse_identifying_parcels(node: Node<'_, '_>) -> Result<Vec<Parcel>> {
    require_legacy_element(node)?;
    element_children(node)
        .map(|child| {
            require_legacy_name(child, "parcela")?;
            parse_parcel(child)
        })
        .collect()
}

/// Parses the presence-preserving `JednotkaUpType` wrapper.
pub(crate) fn parse_unit_wrapper(node: Node<'_, '_>) -> Result<UnitWrapper> {
    let mut cursor = SequenceCursor::new(node, &["jednotka"])?;
    let mut unit = None;
    for child in element_children(node) {
        cursor.singleton(child)?;
        unit = Some(parse_unit(child)?);
    }
    Ok(UnitWrapper { unit })
}

/// Parses `JednotkaType` and requires its `id` element, which may be empty.
pub(crate) fn parse_unit(node: Node<'_, '_>) -> Result<Unit> {
    const FIELDS: &[&str] = &[
        "id",
        "caobce",
        "cp_ident",
        "jed_ident",
        "tel_id",
        "cislo_tel",
        "ku_nazev",
    ];

    let mut cursor = SequenceCursor::new(node, FIELDS)?;
    let mut id = None;
    let mut municipality_part = None;
    let mut building_identification = None;
    let mut unit_identification = None;
    let mut ownership_sheet_id = None;
    let mut ownership_sheet_number = None;
    let mut cadastral_area_name = None;
    for child in element_children(node) {
        match cursor.singleton(child)? {
            0 => id = Some(parse_legacy_id(child)?),
            1 => municipality_part = Some(simple_text(child)?),
            2 => building_identification = Some(simple_text(child)?),
            3 => unit_identification = Some(simple_text(child)?),
            4 => ownership_sheet_id = Some(parse_legacy_id(child)?),
            5 => ownership_sheet_number = Some(simple_text(child)?),
            6 => cadastral_area_name = Some(simple_text(child)?),
            _ => unreachable!("field_index returned an in-range index"),
        }
    }
    Ok(Unit {
        id: required(id, "jednotka/id")?,
        municipality_part,
        building_identification,
        unit_identification,
        ownership_sheet_id,
        ownership_sheet_number,
        cadastral_area_name,
    })
}

/// Parses `PravoStavbyType`, retaining every repeated parcel in order.
pub(crate) fn parse_building_right(node: Node<'_, '_>) -> Result<BuildingRight> {
    const FIELDS: &[&str] = &["id", "tel_id", "parcela", "cislo_tel", "ku_nazev"];

    let mut cursor = SequenceCursor::new(node, FIELDS)?;
    let mut building_right = BuildingRight::default();
    for child in element_children(node) {
        let index =
            if child.tag_name().name() == "parcela" && child.tag_name().namespace().is_none() {
                cursor.repeated(child)?
            } else {
                cursor.singleton(child)?
            };
        match index {
            0 => building_right.id = Some(parse_legacy_id(child)?),
            1 => building_right.ownership_sheet_id = Some(parse_legacy_id(child)?),
            2 => building_right.parcels.push(parse_parcel(child)?),
            3 => building_right.ownership_sheet_number = Some(simple_text(child)?),
            4 => building_right.cadastral_area_name = Some(simple_text(child)?),
            _ => unreachable!("field_index returned an in-range index"),
        }
    }
    Ok(building_right)
}

/// Parses `RizeniType`, retaining its valid empty form and exact integer types.
pub(crate) fn parse_proceeding(node: Node<'_, '_>) -> Result<Proceeding> {
    const FIELDS: &[&str] = &["typriz_kod", "poradove_cislo", "rok", "prares_kod"];

    let mut cursor = SequenceCursor::new(node, FIELDS)?;
    let mut type_code = None;
    let mut serial_number = None;
    let mut year = None;
    let mut workplace_code = None;
    let mut has_children = false;
    for child in element_children(node) {
        has_children = true;
        match cursor.singleton(child)? {
            0 => type_code = Some(simple_text(child)?),
            1 => serial_number = Some(parse_integer(child, "poradove_cislo")?),
            2 => year = Some(parse_integer(child, "rok")?),
            3 => workplace_code = Some(parse_integer(child, "prares_kod")?),
            _ => unreachable!("field_index returned an in-range index"),
        }
    }
    if !has_children {
        return Ok(Proceeding {
            identification: None,
        });
    }
    Ok(Proceeding {
        identification: Some(ProceedingIdentification {
            type_code: required(type_code, "typriz_kod")?,
            serial_number: required(serial_number, "poradove_cislo")?,
            year: required(year, "rok")?,
            workplace_code: required(workplace_code, "prares_kod")?,
        }),
    })
}

/// Parses `CharakteristikaType`, including its exact code range of 1–99.
pub(crate) fn parse_characteristic(node: Node<'_, '_>) -> Result<Characteristic> {
    const FIELDS: &[&str] = &["kod", "zkratka", "zkratka_alv"];

    let mut cursor = SequenceCursor::new(node, FIELDS)?;
    let mut code = None;
    let mut abbreviation = None;
    let mut alv_abbreviation = None;
    let mut has_children = false;
    for child in element_children(node) {
        has_children = true;
        match cursor.singleton(child)? {
            0 => {
                let value: u8 = parse_integer(child, "CharakteristikaOSKodType")?;
                code = Some(CharacteristicCode::new(value).map_err(|_| {
                    invalid_value("CharakteristikaOSKodType", "expected a code in 1..=99")
                })?);
            }
            1 => abbreviation = Some(simple_text(child)?),
            2 => alv_abbreviation = Some(simple_text(child)?),
            _ => unreachable!("field_index returned an in-range index"),
        }
    }
    if !has_children {
        return Ok(Characteristic { details: None });
    }
    Ok(Characteristic {
        details: Some(CharacteristicDetails {
            code: required(code, "char_os/kod")?,
            abbreviation: required(abbreviation, "char_os/zkratka")?,
            alv_abbreviation: required(alv_abbreviation, "char_os/zkratka_alv")?,
        }),
    })
}

/// Parses the presence-preserving `CHAROS_TYPE` wrapper.
pub(crate) fn parse_characteristic_wrapper(node: Node<'_, '_>) -> Result<CharacteristicWrapper> {
    let mut cursor = SequenceCursor::new(node, &["char_os"])?;
    let mut characteristic = None;
    for child in element_children(node) {
        cursor.singleton(child)?;
        characteristic = Some(parse_characteristic(child)?);
    }
    Ok(CharacteristicWrapper { characteristic })
}

/// Parses `OpsubIdentType` as ordered repeating enum entries.
pub(crate) fn parse_authorized_subject(
    node: Node<'_, '_>,
) -> Result<AuthorizedSubjectIdentification> {
    require_legacy_element(node)?;
    let entries = element_children(node)
        .map(parse_authorized_subject_field)
        .collect::<Result<Vec<_>>>()?;
    Ok(AuthorizedSubjectIdentification { entries })
}

fn parse_authorized_subject_field(node: Node<'_, '_>) -> Result<AuthorizedSubjectField> {
    require_legacy_element(node)?;
    let field = match node.tag_name().name() {
        "id" => AuthorizedSubjectField::Id(parse_legacy_id(node)?),
        "rc6" => AuthorizedSubjectField::PersonalNumber6(simple_text(node)?),
        "rc7" => AuthorizedSubjectField::PersonalNumber7(simple_text(node)?),
        "ico" => AuthorizedSubjectField::CompanyId(simple_text(node)?),
        "ico_doplnek" => AuthorizedSubjectField::CompanyIdSuffix(simple_text(node)?),
        "char_os" => AuthorizedSubjectField::Characteristic(parse_characteristic(node)?),
        "bsm1_id" => AuthorizedSubjectField::Partner1Id(parse_legacy_id(node)?),
        "bsm2_id" => AuthorizedSubjectField::Partner2Id(parse_legacy_id(node)?),
        "prijmeni" => AuthorizedSubjectField::Surname(simple_text(node)?),
        "jmeno" => AuthorizedSubjectField::GivenName(simple_text(node)?),
        "titul_pred" => AuthorizedSubjectField::TitleBefore(simple_text(node)?),
        "titul_za" => AuthorizedSubjectField::TitleAfter(simple_text(node)?),
        "bsm1_prijmeni" => AuthorizedSubjectField::Partner1Surname(simple_text(node)?),
        "bsm1_jmeno" => AuthorizedSubjectField::Partner1GivenName(simple_text(node)?),
        "bsm1_pred" => AuthorizedSubjectField::Partner1TitleBefore(simple_text(node)?),
        "bsm1_za" => AuthorizedSubjectField::Partner1TitleAfter(simple_text(node)?),
        "bsm2_prijmeni" => AuthorizedSubjectField::Partner2Surname(simple_text(node)?),
        "bsm2_jmeno" => AuthorizedSubjectField::Partner2GivenName(simple_text(node)?),
        "bsm2_pred" => AuthorizedSubjectField::Partner2TitleBefore(simple_text(node)?),
        "bsm2_za" => AuthorizedSubjectField::Partner2TitleAfter(simple_text(node)?),
        "nazev" => AuthorizedSubjectField::Name(simple_text(node)?),
        "adresa" => AuthorizedSubjectField::Address(parse_address(node)?),
        name => return Err(unexpected_field(name)),
    };
    Ok(field)
}

/// Parses `VlaIdentType` without collapsing empty, repeated, or mixed targets.
pub(crate) fn parse_ownership_identification(
    node: Node<'_, '_>,
) -> Result<OwnershipIdentification> {
    require_legacy_element(node)?;
    let targets = element_children(node)
        .map(|child| {
            require_legacy_element(child)?;
            match child.tag_name().name() {
                "oprav_subjekt" => parse_authorized_subject(child).map(OwnershipTarget::Subject),
                "parcela" => parse_parcel(child).map(OwnershipTarget::Parcel),
                "stavba" => parse_building(child).map(OwnershipTarget::Building),
                "jednotka" => parse_unit(child).map(OwnershipTarget::Unit),
                "pravo_stavby" => parse_building_right(child).map(OwnershipTarget::BuildingRight),
                name => Err(unexpected_field(name)),
            }
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(OwnershipIdentification { targets })
}

/// Parses `OpsubIdentPoType`, including both levels of repeating choices.
pub(crate) fn parse_authorized_subject_po(
    node: Node<'_, '_>,
) -> Result<AuthorizedSubjectPoIdentification> {
    require_legacy_element(node)?;
    let entries = element_children(node)
        .map(|child| {
            require_legacy_element(child)?;
            if child.tag_name().name() == "opravnena_po" {
                parse_legal_entity(child).map(AuthorizedSubjectPoEntry::AuthorizedLegalEntity)
            } else {
                parse_authorized_subject_field(child).map(AuthorizedSubjectPoEntry::Field)
            }
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(AuthorizedSubjectPoIdentification { entries })
}

fn parse_legal_entity(node: Node<'_, '_>) -> Result<LegalEntityIdentification> {
    require_legacy_element(node)?;
    let entries = element_children(node)
        .map(|child| {
            require_legacy_element(child)?;
            if child.tag_name().name() == "oprav_subjekt" {
                parse_authorized_subject(child)
                    .map(LegalEntityIdentificationEntry::AuthorizedSubject)
            } else {
                parse_authorized_subject_field(child).map(LegalEntityIdentificationEntry::Field)
            }
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(LegalEntityIdentification { entries })
}

/// Parses the fixed `OpsubNazevType` subject-name forms.
pub(crate) fn parse_subject_name(node: Node<'_, '_>) -> Result<SubjectNameForm> {
    const FIELDS: &[&str] = &[
        "id",
        "bsm1_id",
        "bsm2_id",
        "nazev",
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
    ];

    let mut cursor = SequenceCursor::new(node, FIELDS)?;
    let mut value = SubjectNameForm::default();
    for child in element_children(node) {
        match cursor.singleton(child)? {
            0 => value.id = Some(parse_legacy_id(child)?),
            1 => value.partner1_id = Some(parse_legacy_id(child)?),
            2 => value.partner2_id = Some(parse_legacy_id(child)?),
            3 => value.name = Some(simple_text(child)?),
            4 => value.person.surname = Some(simple_text(child)?),
            5 => value.person.given_name = Some(simple_text(child)?),
            6 => value.person.title_before = Some(simple_text(child)?),
            7 => value.person.title_after = Some(simple_text(child)?),
            8 => value.partner1.surname = Some(simple_text(child)?),
            9 => value.partner1.given_name = Some(simple_text(child)?),
            10 => value.partner1.title_before = Some(simple_text(child)?),
            11 => value.partner1.title_after = Some(simple_text(child)?),
            12 => value.partner2.surname = Some(simple_text(child)?),
            13 => value.partner2.given_name = Some(simple_text(child)?),
            14 => value.partner2.title_before = Some(simple_text(child)?),
            15 => value.partner2.title_after = Some(simple_text(child)?),
            _ => unreachable!("field_index returned an in-range index"),
        }
    }
    Ok(value)
}

/// Parses `NemIdentType` without collapsing empty, repeated, or mixed targets.
pub(crate) fn parse_property_identification(node: Node<'_, '_>) -> Result<PropertyIdentification> {
    require_legacy_element(node)?;
    let targets = element_children(node)
        .map(|child| {
            require_legacy_element(child)?;
            match child.tag_name().name() {
                "parcela" => parse_parcel(child).map(PropertyTarget::Parcel),
                "stavba" => parse_building(child).map(PropertyTarget::Building),
                "jednotka" => parse_unit(child).map(PropertyTarget::Unit),
                "pravo_stavby" => parse_building_right(child).map(PropertyTarget::BuildingRight),
                name => Err(unexpected_field(name)),
            }
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(PropertyIdentification { targets })
}

/// Parses `OsNemIdentType` with independent optional subject/property targets.
pub(crate) fn parse_subject_property_identification(
    node: Node<'_, '_>,
) -> Result<SubjectPropertyIdentification> {
    const FIELDS: &[&str] = &[
        "jpv_id",
        "tel_id",
        "cislo_tel",
        "oprav_subjekt",
        "parcela",
        "stavba",
        "jednotka",
        "pravo_stavby",
    ];

    let mut cursor = SequenceCursor::new(node, FIELDS)?;
    let mut value = SubjectPropertyIdentification::default();
    for child in element_children(node) {
        match cursor.singleton(child)? {
            0 => value.legal_relation_id = Some(parse_legacy_id(child)?),
            1 => value.ownership_sheet_id = Some(parse_legacy_id(child)?),
            2 => value.ownership_sheet_number = Some(simple_text(child)?),
            3 => value.subject = Some(parse_authorized_subject_po(child)?),
            4 => value.parcel = Some(parse_parcel(child)?),
            5 => value.building = Some(parse_building(child)?),
            6 => value.unit = Some(parse_unit(child)?),
            7 => value.building_right = Some(parse_building_right(child)?),
            _ => unreachable!("field_index returned an in-range index"),
        }
    }
    Ok(value)
}

/// Parses `PodilType` with independently optional signed exact components.
///
/// No nonzero restriction is applied to the denominator because the legacy XSD
/// has none. Valid decimal lexemes outside `rust_decimal`'s exact range return
/// [`Error::DecimalOutOfRange`].
pub(crate) fn parse_reported_share(node: Node<'_, '_>) -> Result<ReportedShare> {
    const FIELDS: &[&str] = &["citatel", "jmenovatel"];

    let mut cursor = SequenceCursor::new(node, FIELDS)?;
    let mut share = ReportedShare::default();
    for child in element_children(node) {
        match cursor.singleton(child)? {
            0 => {
                share.numerator = Some(parse_share_component(child, "PodilType/citatel")?);
            }
            1 => {
                share.denominator = Some(parse_share_component(child, "PodilType/jmenovatel")?);
            }
            _ => unreachable!("field_index returned an in-range index"),
        }
    }
    Ok(share)
}

/// Parses the required no-namespace `SEKCE` attribute as `SekceListSekceType`.
pub(crate) fn parse_deed_section(node: Node<'_, '_>) -> Result<DeedSection> {
    require_legacy_element(node)?;
    node.attribute("SEKCE")
        .map(|raw| DeedSection::from_raw(raw.to_owned()))
        .ok_or_else(|| invalid_value("SEKCE", "required section attribute is missing"))
}

/// Parses `AnoNeType` and retains unknown values in `YesNo::Other`.
pub(crate) fn parse_yes_no(node: Node<'_, '_>) -> Result<YesNo> {
    simple_text(node).map(YesNo::from_raw)
}

/// Parses an exact `Number102Type` decimal and enforces its XSD facets.
pub(crate) fn parse_number102(node: Node<'_, '_>) -> Result<Number102> {
    let value = parse_decimal(node, "Number102Type")?;
    Number102::new(value).map_err(|_| {
        invalid_value(
            "Number102Type",
            "expected at most 10 total and 2 fractional digits",
        )
    })
}

fn parse_share_component(node: Node<'_, '_>, field: &'static str) -> Result<Decimal> {
    let value = parse_decimal(node, field)?.normalize();
    if value.scale() != 0 {
        return Err(invalid_value(
            field,
            "expected an integral xs:decimal value",
        ));
    }
    if value.mantissa().unsigned_abs().to_string().len() > 30 {
        return Err(invalid_value(field, "expected at most 30 total digits"));
    }
    Ok(value)
}

fn parse_decimal(node: Node<'_, '_>, field: &'static str) -> Result<Decimal> {
    let raw = simple_text(node)?;
    let normalized = trim_xml_whitespace(&raw);
    match crate::model::parse_decimal(normalized, field) {
        Ok(value) => Ok(value),
        Err(error @ Error::DecimalOutOfRange { .. }) => Err(error),
        Err(_) => Err(invalid_value(field, "invalid xs:decimal lexical value")),
    }
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

fn simple_text(node: Node<'_, '_>) -> Result<String> {
    require_legacy_element(node)?;
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

struct SequenceCursor<'a> {
    fields: &'a [&'static str],
    last: Option<usize>,
    seen: Vec<bool>,
}

impl<'a> SequenceCursor<'a> {
    fn new(node: Node<'_, '_>, fields: &'a [&'static str]) -> Result<Self> {
        require_legacy_element(node)?;
        Ok(Self {
            fields,
            last: None,
            seen: vec![false; fields.len()],
        })
    }

    fn singleton(&mut self, node: Node<'_, '_>) -> Result<usize> {
        let index = self.enter(node)?;
        if self.seen[index] {
            return Err(invalid_value(
                self.fields[index],
                "singleton legacy element occurs more than once",
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
        if self.last.is_some_and(|last| index < last) {
            return Err(invalid_value(
                self.fields[index],
                "legacy sequence element is out of order",
            ));
        }
        self.last = Some(index);
        Ok(index)
    }
}

fn field_index(node: Node<'_, '_>, fields: &[&str]) -> Result<usize> {
    require_legacy_element(node)?;
    let name = node.tag_name().name();
    fields
        .iter()
        .position(|candidate| *candidate == name)
        .ok_or_else(|| unexpected_field(name))
}

fn element_children<'a, 'input>(node: Node<'a, 'input>) -> impl Iterator<Item = Node<'a, 'input>> {
    node.children().filter(|child| child.is_element())
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

fn require_legacy_name(node: Node<'_, '_>, expected: &str) -> Result<()> {
    require_legacy_element(node)?;
    if node.tag_name().name() == expected {
        Ok(())
    } else {
        Err(unexpected_field(node.tag_name().name()))
    }
}

fn trim_xml_whitespace(value: &str) -> &str {
    value.trim_matches(|character| matches!(character, ' ' | '\t' | '\r' | '\n'))
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

#[cfg(test)]
mod tests {
    use roxmltree::Document;

    use super::*;

    fn try_parse<T>(
        xml: &str,
        parser: for<'a, 'input> fn(Node<'a, 'input>) -> Result<T>,
    ) -> Result<T> {
        let document = Document::parse(xml).unwrap();
        parser(document.root_element())
    }

    fn parse<T>(xml: &str, parser: for<'a, 'input> fn(Node<'a, 'input>) -> Result<T>) -> T {
        try_parse(xml, parser).unwrap()
    }

    #[test]
    fn address_retains_repeated_sequence_groups_order_and_untouched_text() {
        let long_id = "9".repeat(120);
        let xml = format!(
            "<adresa><adresni_misto/><nazev_ulice>  First street  </nazev_ulice><obec>One</obec><adresni_misto>{long_id}</adresni_misto><cp_ce>3</cp_ce><obec>Two</obec></adresa>"
        );
        let address = parse(&xml, parse_address);
        assert_eq!(address.groups.len(), 2);
        assert_eq!(
            address.groups[0]
                .address_point_id
                .as_ref()
                .map(LegacyId::as_str),
            Some("")
        );
        assert_eq!(
            address.groups[0].street_name.as_deref(),
            Some("  First street  ")
        );
        assert_eq!(
            address.groups[1]
                .address_point_id
                .as_ref()
                .map(LegacyId::as_str),
            Some(long_id.as_str())
        );
        assert_eq!(
            address.groups[1].house_number_kind,
            Some(HouseNumberKind::Other("3".to_owned()))
        );
        assert_eq!(address.groups[1].municipality.as_deref(), Some("Two"));
        assert!(address.groups[1].street_name.is_none());

        let repeated_streets = parse(
            "<adresa><nazev_ulice>one</nazev_ulice><nazev_ulice>two</nazev_ulice></adresa>",
            parse_address,
        );
        assert_eq!(repeated_streets.groups.len(), 2);
        assert_eq!(
            repeated_streets.groups[1].street_name.as_deref(),
            Some("two")
        );
        assert!(parse("<adresa/>", parse_address).groups.is_empty());
    }

    #[test]
    fn legacy_ids_distinguish_absent_empty_and_unbounded_positive_values() {
        assert!(
            parse("<parcela/>", parse_parcel).id.is_none(),
            "absence must not become an empty ID"
        );
        assert_eq!(
            parse("<parcela><id/></parcela>", parse_parcel)
                .id
                .as_ref()
                .map(LegacyId::as_str),
            Some("")
        );

        let unbounded = "1234567890".repeat(20);
        let parsed = parse(
            &format!("<parcela><id>{unbounded}</id></parcela>"),
            parse_parcel,
        );
        assert_eq!(
            parsed.id.as_ref().map(LegacyId::as_str),
            Some(&unbounded[..])
        );

        let with_numeric_whitespace =
            parse("<parcela><id> \n+000123\t</id></parcela>", parse_parcel);
        assert_eq!(
            with_numeric_whitespace.id.as_ref().map(LegacyId::as_str),
            Some(" \n+000123\t")
        );

        for invalid in ["0", "+000", "-1", "1.0", " ", "abc"] {
            assert!(
                try_parse(
                    &format!("<parcela><id>{invalid}</id></parcela>"),
                    parse_parcel,
                )
                .is_err(),
                "invalid ID was accepted"
            );
        }
    }

    #[test]
    fn ownership_choices_preserve_empty_repeated_and_interleaved_targets() {
        assert!(
            parse("<VLA_IDENT/>", parse_ownership_identification)
                .targets
                .is_empty()
        );
        let ownership = parse(
            "<VLA_IDENT><parcela><id>1</id></parcela><oprav_subjekt><jmeno>A</jmeno><id/><jmeno>B</jmeno><adresa/><jmeno>C</jmeno></oprav_subjekt><stavba><id>2</id></stavba><oprav_subjekt/><jednotka><id/></jednotka></VLA_IDENT>",
            parse_ownership_identification,
        );
        assert_eq!(ownership.targets.len(), 5);
        assert!(matches!(ownership.targets[0], OwnershipTarget::Parcel(_)));
        let OwnershipTarget::Subject(subject) = &ownership.targets[1] else {
            panic!("second target must be a subject");
        };
        assert_eq!(subject.entries.len(), 5);
        assert!(matches!(
            subject.entries[0],
            AuthorizedSubjectField::GivenName(ref name) if name == "A"
        ));
        assert!(matches!(
            subject.entries[1],
            AuthorizedSubjectField::Id(ref id) if id.as_str().is_empty()
        ));
        assert!(matches!(
            subject.entries[2],
            AuthorizedSubjectField::GivenName(ref name) if name == "B"
        ));
        assert!(matches!(
            subject.entries[3],
            AuthorizedSubjectField::Address(_)
        ));
        assert!(matches!(
            subject.entries[4],
            AuthorizedSubjectField::GivenName(ref name) if name == "C"
        ));
        assert!(matches!(ownership.targets[2], OwnershipTarget::Building(_)));
        let OwnershipTarget::Subject(empty) = &ownership.targets[3] else {
            panic!("fourth target must be an empty subject");
        };
        assert!(empty.entries.is_empty());
        assert!(matches!(ownership.targets[4], OwnershipTarget::Unit(_)));

        let properties = parse(
            "<NEM_IDENT><pravo_stavby/><parcela/><pravo_stavby/></NEM_IDENT>",
            parse_property_identification,
        );
        assert_eq!(properties.targets.len(), 3);
        assert!(matches!(
            properties.targets[0],
            PropertyTarget::BuildingRight(_)
        ));
        assert!(matches!(properties.targets[1], PropertyTarget::Parcel(_)));
        assert!(matches!(
            properties.targets[2],
            PropertyTarget::BuildingRight(_)
        ));
    }

    #[test]
    fn every_authorized_subject_choice_variant_is_exposed() {
        let names = [
            "id",
            "rc6",
            "rc7",
            "ico",
            "ico_doplnek",
            "char_os",
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
            "adresa",
        ];
        let xml = names
            .iter()
            .map(|name| format!("<{name}/>"))
            .collect::<String>();
        let value = parse(&format!("<OPSUB>{xml}</OPSUB>"), parse_authorized_subject);
        let empty_id = || LegacyId::new("").unwrap();
        let empty_text = String::new;
        let expected = vec![
            AuthorizedSubjectField::Id(empty_id()),
            AuthorizedSubjectField::PersonalNumber6(empty_text()),
            AuthorizedSubjectField::PersonalNumber7(empty_text()),
            AuthorizedSubjectField::CompanyId(empty_text()),
            AuthorizedSubjectField::CompanyIdSuffix(empty_text()),
            AuthorizedSubjectField::Characteristic(Characteristic::default()),
            AuthorizedSubjectField::Partner1Id(empty_id()),
            AuthorizedSubjectField::Partner2Id(empty_id()),
            AuthorizedSubjectField::Surname(empty_text()),
            AuthorizedSubjectField::GivenName(empty_text()),
            AuthorizedSubjectField::TitleBefore(empty_text()),
            AuthorizedSubjectField::TitleAfter(empty_text()),
            AuthorizedSubjectField::Partner1Surname(empty_text()),
            AuthorizedSubjectField::Partner1GivenName(empty_text()),
            AuthorizedSubjectField::Partner1TitleBefore(empty_text()),
            AuthorizedSubjectField::Partner1TitleAfter(empty_text()),
            AuthorizedSubjectField::Partner2Surname(empty_text()),
            AuthorizedSubjectField::Partner2GivenName(empty_text()),
            AuthorizedSubjectField::Partner2TitleBefore(empty_text()),
            AuthorizedSubjectField::Partner2TitleAfter(empty_text()),
            AuthorizedSubjectField::Name(empty_text()),
            AuthorizedSubjectField::Address(Address::default()),
        ];
        assert_eq!(value.entries, expected);
    }

    #[test]
    fn nested_legal_entity_choices_retain_both_choice_levels() {
        let value = parse(
            "<OPSUB><nazev>outer</nazev><opravnena_po><id>9</id><oprav_subjekt><prijmeni>nested</prijmeni></oprav_subjekt><id>10</id></opravnena_po><ico>00raw</ico><opravnena_po/></OPSUB>",
            parse_authorized_subject_po,
        );
        assert_eq!(value.entries.len(), 4);
        assert!(matches!(
            value.entries[0],
            AuthorizedSubjectPoEntry::Field(AuthorizedSubjectField::Name(ref name))
                if name == "outer"
        ));
        let AuthorizedSubjectPoEntry::AuthorizedLegalEntity(entity) = &value.entries[1] else {
            panic!("second entry must be a legal entity");
        };
        assert_eq!(entity.entries.len(), 3);
        assert!(matches!(
            entity.entries[0],
            LegalEntityIdentificationEntry::Field(AuthorizedSubjectField::Id(ref id))
                if id.as_str() == "9"
        ));
        assert!(matches!(
            entity.entries[1],
            LegalEntityIdentificationEntry::AuthorizedSubject(_)
        ));
        assert!(matches!(
            entity.entries[2],
            LegalEntityIdentificationEntry::Field(AuthorizedSubjectField::Id(ref id))
                if id.as_str() == "10"
        ));
        assert!(matches!(
            value.entries[2],
            AuthorizedSubjectPoEntry::Field(AuthorizedSubjectField::CompanyId(ref raw))
                if raw == "00raw"
        ));
        let AuthorizedSubjectPoEntry::AuthorizedLegalEntity(empty) = &value.entries[3] else {
            panic!("last entry must be an empty legal entity");
        };
        assert!(empty.entries.is_empty());
    }

    #[test]
    fn combined_identification_has_independent_optional_targets() {
        let value = parse(
            "<OS_NEM><oprav_subjekt/><stavba/><jednotka><id/></jednotka><pravo_stavby/></OS_NEM>",
            parse_subject_property_identification,
        );
        assert!(value.subject.is_some());
        assert!(value.parcel.is_none());
        assert!(value.building.is_some());
        assert!(value.unit.is_some());
        assert!(value.building_right.is_some());

        let parcel_only = parse(
            "<OS_NEM><parcela><id>1</id></parcela></OS_NEM>",
            parse_subject_property_identification,
        );
        assert!(parcel_only.subject.is_none());
        assert!(parcel_only.parcel.is_some());
        assert!(parcel_only.building.is_none());
        assert!(
            try_parse(
                "<OS_NEM><stavba/><stavba/></OS_NEM>",
                parse_subject_property_identification,
            )
            .is_err()
        );
    }

    #[test]
    fn empty_wrappers_and_optional_sequences_remain_distinct() {
        assert!(parse("<PAR_IDENT/>", parse_parcel_wrapper).parcel.is_none());
        assert!(
            parse("<STAVBA_UP/>", parse_building_wrapper)
                .building
                .is_none()
        );
        assert!(parse("<JEDNOTKA_UP/>", parse_unit_wrapper).unit.is_none());
        let wrapper = parse("<CHAROS><char_os/></CHAROS>", parse_characteristic_wrapper);
        assert!(
            wrapper
                .characteristic
                .is_some_and(|value| value.details.is_none())
        );
        assert!(
            parse("<CHAROS/>", parse_characteristic_wrapper)
                .characteristic
                .is_none()
        );
        assert!(
            parse("<RIZENI/>", parse_proceeding)
                .identification
                .is_none()
        );
        assert!(parse("<LISTINA_INFO/>", parse_deed_name).details.is_none());
        assert!(
            try_parse(
                "<LISTINA_INFO><tlist>V</tlist></LISTINA_INFO>",
                parse_deed_name
            )
            .is_err()
        );
        let deed = parse(
            "<LISTINA_INFO><tlist>V</tlist><dalsi_udaje>additional</dalsi_udaje><por_cis_zhot>7</por_cis_zhot><popis>description</popis><vystav_txt>issued</vystav_txt><prav_moc_txt>force</prav_moc_txt><podani_txt>submission</podani_txt><vykonatelnost_txt>enforceable</vykonatelnost_txt><podani_z_txt>submission-z</podani_z_txt><zplatneni_txt>effective</zplatneni_txt><prares_nazev>workplace</prares_nazev></LISTINA_INFO>",
            parse_deed_name,
        );
        let details = deed.details.unwrap();
        assert_eq!(details.deed_type, "V");
        assert_eq!(details.additional_data, "additional");
        assert_eq!(details.preparation_serial_number, "7");
        assert_eq!(details.description, "description");
        assert_eq!(details.issuance_text, "issued");
        assert_eq!(details.legal_force_text, "force");
        assert_eq!(details.submission_text, "submission");
        assert_eq!(details.enforceability_text, "enforceable");
        assert_eq!(details.submission_z_text, "submission-z");
        assert_eq!(details.effectiveness_text, "effective");
        assert_eq!(
            details.handling_workplace_name.as_deref(),
            Some("workplace")
        );

        let building = parse("<stavba><par_ident/></stavba>", parse_building);
        assert_eq!(building.identifying_parcels, Some(Vec::new()));
        assert!(
            parse("<stavba/>", parse_building)
                .identifying_parcels
                .is_none()
        );
    }

    #[test]
    fn subject_name_forms_retain_primary_and_both_partners() {
        let value = parse(
            "<OPSUB_NAZEV><id/><bsm1_id>11</bsm1_id><nazev>  Organization  </nazev><prijmeni>Primary surname</prijmeni><titul_za>Primary title</titul_za><bsm1_jmeno>First</bsm1_jmeno><bsm2_pred>Second title</bsm2_pred><bsm2_za>Second suffix</bsm2_za></OPSUB_NAZEV>",
            parse_subject_name,
        );
        assert_eq!(value.id.as_ref().map(LegacyId::as_str), Some(""));
        assert_eq!(value.partner1_id.as_ref().map(LegacyId::as_str), Some("11"));
        assert_eq!(value.name.as_deref(), Some("  Organization  "));
        assert_eq!(value.person.surname.as_deref(), Some("Primary surname"));
        assert_eq!(value.person.title_after.as_deref(), Some("Primary title"));
        assert_eq!(value.partner1.given_name.as_deref(), Some("First"));
        assert_eq!(value.partner2.title_before.as_deref(), Some("Second title"));
        assert_eq!(value.partner2.title_after.as_deref(), Some("Second suffix"));
    }

    #[test]
    fn share_components_are_signed_zero_capable_and_independently_optional() {
        let share = parse(
            "<PODIL><citatel>-12</citatel><jmenovatel>0</jmenovatel></PODIL>",
            parse_reported_share,
        );
        assert_eq!(share.numerator, Some(Decimal::new(-12, 0)));
        assert_eq!(share.denominator, Some(Decimal::ZERO));

        let empty = parse("<PODIL/>", parse_reported_share);
        assert_eq!(empty.numerator, None);
        assert_eq!(empty.denominator, None);
        let numerator = parse(
            "<PODIL><citatel>-3.0</citatel></PODIL>",
            parse_reported_share,
        );
        assert_eq!(numerator.numerator, Some(Decimal::new(-3, 0)));
        assert_eq!(numerator.numerator.unwrap().scale(), 0);
        assert_eq!(numerator.denominator, None);
        let denominator = parse(
            "<PODIL><jmenovatel>-4</jmenovatel></PODIL>",
            parse_reported_share,
        );
        assert_eq!(denominator.numerator, None);
        assert_eq!(denominator.denominator, Some(Decimal::new(-4, 0)));
        let alternative_lexical_forms = parse(
            "<PODIL><citatel>+1.</citatel><jmenovatel>.0</jmenovatel></PODIL>",
            parse_reported_share,
        );
        assert_eq!(alternative_lexical_forms.numerator, Some(Decimal::ONE));
        assert_eq!(alternative_lexical_forms.denominator, Some(Decimal::ZERO));
        assert!(
            try_parse(
                "<PODIL><citatel>1.5</citatel></PODIL>",
                parse_reported_share,
            )
            .is_err()
        );
        for invalid in ["1e2", "1_000"] {
            let error = try_parse(
                &format!("<PODIL><citatel>{invalid}</citatel></PODIL>"),
                parse_reported_share,
            )
            .unwrap_err();
            assert!(matches!(error, Error::InvalidResponse { .. }));
            assert!(try_parse(&format!("<CENA>{invalid}</CENA>"), parse_number102,).is_err());
        }
    }

    #[test]
    fn out_of_range_share_decimals_use_the_dedicated_error() {
        let too_large = "100000000000000000000000000000";
        for (element, expected_field) in [
            ("citatel", "PodilType/citatel"),
            ("jmenovatel", "PodilType/jmenovatel"),
        ] {
            let error = try_parse(
                &format!("<PODIL><{element}>{too_large}</{element}></PODIL>"),
                parse_reported_share,
            )
            .unwrap_err();
            assert!(matches!(
                error,
                Error::DecimalOutOfRange { field } if field == expected_field
            ));
            assert!(!error.to_string().contains(too_large));
        }

        let maximum = "79228162514264337593543950335";
        let share = parse(
            &format!("<PODIL><citatel>{maximum}</citatel></PODIL>"),
            parse_reported_share,
        );
        assert_eq!(share.numerator.unwrap().to_string(), maximum);
    }

    #[test]
    fn legacy_integer_and_decimal_ranges_are_enforced_exactly() {
        for raw in ["-128", "127"] {
            let value = parse(
                &format!("<parcela><dil_parcely>{raw}</dil_parcely></parcela>"),
                parse_parcel,
            );
            assert_eq!(value.part, Some(raw.parse::<i8>().unwrap()));
        }
        for raw in ["-129", "128"] {
            assert!(
                try_parse(
                    &format!("<parcela><dil_parcely>{raw}</dil_parcely></parcela>"),
                    parse_parcel,
                )
                .is_err()
            );
        }

        let proceeding = parse(
            "<RIZENI><typriz_kod>V</typriz_kod><poradove_cislo>4294967295</poradove_cislo><rok>65535</rok><prares_kod>65535</prares_kod></RIZENI>",
            parse_proceeding,
        );
        let identification = proceeding.identification.unwrap();
        assert_eq!(identification.serial_number, u32::MAX);
        assert_eq!(identification.year, u16::MAX);
        assert_eq!(identification.workplace_code, u16::MAX);
        assert!(
            try_parse(
                "<RIZENI><typriz_kod>V</typriz_kod><poradove_cislo>4294967296</poradove_cislo><rok>1</rok><prares_kod>1</prares_kod></RIZENI>",
                parse_proceeding,
            )
            .is_err()
        );
        for code in ["1", "99"] {
            let characteristic = parse(
                &format!("<char_os><kod>{code}</kod><zkratka/><zkratka_alv/></char_os>"),
                parse_characteristic,
            );
            assert_eq!(
                characteristic.details.unwrap().code,
                CharacteristicCode::new(code.parse::<u8>().unwrap()).unwrap()
            );
        }
        for code in ["0", "100"] {
            assert!(
                try_parse(
                    &format!("<char_os><kod>{code}</kod><zkratka/><zkratka_alv/></char_os>"),
                    parse_characteristic,
                )
                .is_err()
            );
        }

        assert_eq!(
            parse("<CENA>-99999999.99</CENA>", parse_number102)
                .as_decimal()
                .to_string(),
            "-99999999.99"
        );
        assert_eq!(
            parse("<CENA>12345678.900</CENA>", parse_number102)
                .as_decimal()
                .to_string(),
            "12345678.9"
        );
        for invalid in ["999999999.99", "0.001"] {
            assert!(try_parse(&format!("<CENA>{invalid}</CENA>"), parse_number102).is_err());
        }
        assert!(matches!(
            try_parse(
                "<CENA>999999999999999999999999999999</CENA>",
                parse_number102,
            ),
            Err(Error::DecimalOutOfRange {
                field: "Number102Type"
            })
        ));
    }

    #[test]
    fn open_codes_retain_empty_and_unknown_values() {
        assert_eq!(
            parse("<cp_ce/>", parse_house_number_kind),
            HouseNumberKind::Other(String::new())
        );
        assert_eq!(
            parse("<OMEZENI>future</OMEZENI>", parse_yes_no),
            YesNo::Other("future".to_owned())
        );
        assert_eq!(
            parse("<LISTINA SEKCE=\"E\"/>", parse_deed_section),
            DeedSection::Other("E".to_owned())
        );
        assert_eq!(
            parse("<LISTINA SEKCE=\"B1\"/>", parse_deed_section),
            DeedSection::B1
        );
        assert!(try_parse("<LISTINA/>", parse_deed_section).is_err());
    }

    #[test]
    fn duplicate_singletons_bad_order_namespaces_and_nil_are_rejected() {
        for xml in [
            "<parcela><id>1</id><id>2</id></parcela>",
            "<parcela><par_cis>1</par_cis><id>2</id></parcela>",
            "<PODIL><citatel>1</citatel><citatel>2</citatel></PODIL>",
            "<PODIL><jmenovatel>2</jmenovatel><citatel>1</citatel></PODIL>",
            "<PAR_IDENT><parcela/><parcela/></PAR_IDENT>",
        ] {
            let result = if xml.contains("PODIL") {
                try_parse(xml, parse_reported_share).map(|_| ())
            } else if xml.contains("PAR_IDENT") {
                try_parse(xml, parse_parcel_wrapper).map(|_| ())
            } else {
                try_parse(xml, parse_parcel).map(|_| ())
            };
            assert!(result.is_err(), "invalid sequence was accepted: {xml}");
        }
        assert!(
            try_parse(
                "<parcela xmlns=\"urn:unexpected\"><id>1</id></parcela>",
                parse_parcel,
            )
            .is_err()
        );
        assert!(
            try_parse(
                "<parcela xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\"><id xsi:nil=\"true\"/></parcela>",
                parse_parcel,
            )
            .is_err()
        );
        assert!(try_parse("<parcela><future/></parcela>", parse_parcel).is_err());
    }
}
