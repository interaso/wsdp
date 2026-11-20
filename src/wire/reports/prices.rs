//! Namespace-aware legacy and monthly price report decoding.

use std::io::{Cursor, Read};
use std::path::{Component, Path};
use std::str::FromStr;

use quick_xml::events::{BytesStart, Event};
use roxmltree::{Document, Node, ParsingOptions};
use rust_decimal::Decimal;
use zip::ZipArchive;

use crate::model::reports::prices::*;
use crate::{Error, LegacyId, Result, XmlDateTime};

const MONTHLY_NS: &str = "http://services.cuzk.cz/xsd/cenoveUdaje/1.0";
const GML_NS: &str = "http://www.opengis.net/gml/3.2";
const DEFAULT_MAX_XML_NODES: u32 = 2_000_000;
const DEFAULT_MAX_XML_DEPTH: u32 = 256;

/// Parses a complete legacy `CenoveUdajeDleNemovitosti` document.
pub fn parse_prices_by_properties(bytes: &[u8]) -> Result<PricesByPropertiesReport> {
    parse_prices_by_properties_with_limits(bytes, DEFAULT_MAX_XML_NODES, DEFAULT_MAX_XML_DEPTH)
}

/// Parses a prices-by-properties document with explicit XML complexity limits.
pub fn parse_prices_by_properties_with_limits(
    bytes: &[u8],
    max_nodes: u32,
    max_depth: u32,
) -> Result<PricesByPropertiesReport> {
    parse_xml(
        bytes,
        None,
        "CenoveUdajeDleNemovitosti",
        max_nodes,
        max_depth,
        |root, _| {
            let cadastral_units =
                children(required_child(root, None, "KATASTRY")?, None, "KATASTR")
                    .map(decode_cadastral_unit)
                    .collect::<Result<Vec<_>>>()?;
            Ok(PricesByPropertiesReport {
                cadastral_units,
                free_access: required_text(root, None, "BEZPL_PRISTUP")?,
                validity: required_text(root, None, "PLATNOST")?,
                prepared_by: required_text(root, None, "VYHOTOVIL")?,
                output_info: required_text(root, None, "INFO_VYSTUP")?,
                prepared_at: required_text(root, None, "VYHOTOVENO")?,
            })
        },
    )
}

/// Parses a complete legacy `CenoveUdajeDleRizeni` document.
pub fn parse_prices_by_proceeding(bytes: &[u8]) -> Result<PricesByProceedingReport> {
    parse_prices_by_proceeding_with_limits(bytes, DEFAULT_MAX_XML_NODES, DEFAULT_MAX_XML_DEPTH)
}

/// Parses a prices-by-proceeding document with explicit XML complexity limits.
pub fn parse_prices_by_proceeding_with_limits(
    bytes: &[u8],
    max_nodes: u32,
    max_depth: u32,
) -> Result<PricesByProceedingReport> {
    parse_xml(
        bytes,
        None,
        "CenoveUdajeDleRizeni",
        max_nodes,
        max_depth,
        |root, _| {
            let workplaces = children(required_child(root, None, "PRACOVISTE")?, None, "KP")
                .map(decode_workplace)
                .collect::<Result<Vec<_>>>()?;
            Ok(PricesByProceedingReport {
                workplaces,
                free_access: required_text(root, None, "BEZPL_PRISTUP")?,
                prepared_by: required_text(root, None, "VYHOTOVIL")?,
                output_info: required_text(root, None, "INFO_VYSTUP")?,
                prepared_at: required_text(root, None, "VYHOTOVENO")?,
            })
        },
    )
}

/// Parses one monthly document with explicit XML complexity limits.
pub fn parse_monthly_prices_with_limits(
    bytes: &[u8],
    max_nodes: u32,
    max_depth: u32,
) -> Result<MonthlyPriceData> {
    parse_xml(
        bytes,
        Some(MONTHLY_NS),
        "CenoveUdaje",
        max_nodes,
        max_depth,
        decode_monthly_data,
    )
}

/// Parses a monthly price ZIP entirely in memory under the supplied expansion limits.
///
/// Every entry must have a safe, non-directory, non-symbolic-link name and an
/// expanded `{http://services.cuzk.cz/xsd/cenoveUdaje/1.0}CenoveUdaje` root.
/// No archive entry is extracted to the filesystem.
pub fn parse_monthly_price_archive(
    bytes: &[u8],
    limits: MonthlyPriceArchiveLimits,
) -> Result<MonthlyPriceArchive> {
    parse_monthly_price_archive_with_xml_limits(
        bytes,
        limits,
        DEFAULT_MAX_XML_NODES,
        DEFAULT_MAX_XML_DEPTH,
    )
}

/// Parses a monthly ZIP with independent archive-expansion and per-entry XML limits.
pub fn parse_monthly_price_archive_with_xml_limits(
    bytes: &[u8],
    limits: MonthlyPriceArchiveLimits,
    max_nodes: u32,
    max_depth: u32,
) -> Result<MonthlyPriceArchive> {
    preflight_zip_entry_count(bytes, limits.max_entries)?;
    let cursor = Cursor::new(bytes);
    let mut archive = ZipArchive::new(cursor)
        .map_err(|_| Error::invalid_response("invalid monthly price ZIP archive"))?;
    if archive.len() > limits.max_entries {
        return Err(Error::invalid_response(
            "monthly price ZIP exceeds the entry-count limit",
        ));
    }

    let mut entries = Vec::with_capacity(archive.len());
    let mut total_uncompressed = 0_u64;
    for index in 0..archive.len() {
        let mut file = archive
            .by_index(index)
            .map_err(|_| Error::invalid_response("could not read monthly price ZIP entry"))?;
        let name = safe_entry_name(&file)?;
        if !file.is_file() {
            return Err(Error::invalid_response(
                "monthly price ZIP entries must be regular files",
            ));
        }

        let compressed_size = file.compressed_size();
        let declared_uncompressed = file.size();
        check_entry_size(declared_uncompressed, limits.max_entry_uncompressed_bytes)?;
        check_compression_ratio(
            declared_uncompressed,
            compressed_size,
            limits.max_compression_ratio,
        )?;
        total_uncompressed = check_total_size(total_uncompressed, declared_uncompressed, limits)?;

        let read_cap = limits
            .max_entry_uncompressed_bytes
            .min(limits.max_total_uncompressed_bytes);
        let original_bytes = read_bounded(&mut file, read_cap)?;
        let actual_uncompressed = original_bytes.len() as u64;
        check_entry_size(actual_uncompressed, limits.max_entry_uncompressed_bytes)?;
        check_compression_ratio(
            actual_uncompressed,
            compressed_size,
            limits.max_compression_ratio,
        )?;
        if actual_uncompressed != declared_uncompressed {
            return Err(Error::invalid_response(
                "monthly price ZIP entry size does not match its metadata",
            ));
        }

        let metadata = MonthlyPriceEntryMetadata {
            index,
            name,
            name_raw: file.name_raw().to_vec(),
            compressed_size,
            uncompressed_size: actual_uncompressed,
            crc32: file.crc32(),
        };
        let value = parse_monthly_prices_with_limits(&original_bytes, max_nodes, max_depth)?;
        entries.push(MonthlyPriceEntry {
            metadata,
            original_bytes,
            value,
        });
    }

    Ok(MonthlyPriceArchive {
        original_bytes: bytes.to_vec(),
        entries,
    })
}

fn preflight_zip_entry_count(bytes: &[u8], max_entries: usize) -> Result<()> {
    let eocd = find_zip_eocd(bytes)
        .ok_or_else(|| Error::invalid_response("invalid monthly price ZIP archive"))?;
    let entries_on_disk = zip_u16(bytes, eocd + 8);
    let entries_total = zip_u16(bytes, eocd + 10);
    let central_size = zip_u32(bytes, eocd + 12);
    let central_offset = zip_u32(bytes, eocd + 16);
    let (Some(entries_on_disk), Some(entries_total), Some(central_size), Some(central_offset)) =
        (entries_on_disk, entries_total, central_size, central_offset)
    else {
        return Err(Error::invalid_response("invalid monthly price ZIP archive"));
    };

    check_zip_entry_count(u64::from(entries_on_disk), max_entries)?;
    check_zip_entry_count(u64::from(entries_total), max_entries)?;
    let needs_zip64 = entries_on_disk == u16::MAX
        || entries_total == u16::MAX
        || central_size == u32::MAX
        || central_offset == u32::MAX;
    let locator = eocd
        .checked_sub(20)
        .filter(|offset| bytes.get(*offset..*offset + 4) == Some(b"PK\x06\x07".as_slice()));
    if let Some(locator) = locator {
        let record_offset = zip_u64(bytes, locator + 8)
            .and_then(|offset| usize::try_from(offset).ok())
            .ok_or_else(|| Error::invalid_response("invalid monthly price ZIP64 locator"))?;
        let record = record_offset
            .checked_add(56)
            .and_then(|end| bytes.get(record_offset..end))
            .filter(|record| record.get(..4) == Some(b"PK\x06\x06".as_slice()))
            .filter(|record| zip_u64(record, 4).is_some_and(|size| size >= 44))
            .ok_or_else(|| Error::invalid_response("invalid monthly price ZIP64 directory"))?;
        let zip64_entries_on_disk = zip_u64(record, 24)
            .ok_or_else(|| Error::invalid_response("invalid monthly price ZIP64 directory"))?;
        let zip64_entries = zip_u64(record, 32)
            .ok_or_else(|| Error::invalid_response("invalid monthly price ZIP64 directory"))?;
        check_zip_entry_count(zip64_entries_on_disk, max_entries)?;
        check_zip_entry_count(zip64_entries, max_entries)?;
    } else if needs_zip64 {
        return Err(Error::invalid_response(
            "monthly price ZIP64 end record is missing",
        ));
    }
    Ok(())
}

fn find_zip_eocd(bytes: &[u8]) -> Option<usize> {
    const EOCD_LEN: usize = 22;
    let last = bytes.len().checked_sub(EOCD_LEN)?;
    let first = bytes.len().saturating_sub(EOCD_LEN + usize::from(u16::MAX));
    (first..=last).rev().find(|offset| {
        bytes.get(*offset..*offset + 4) == Some(b"PK\x05\x06".as_slice())
            && zip_u16(bytes, *offset + 20).is_some_and(|comment_len| {
                *offset + EOCD_LEN + usize::from(comment_len) == bytes.len()
            })
    })
}

fn check_zip_entry_count(count: u64, maximum: usize) -> Result<()> {
    if count > u64::try_from(maximum).unwrap_or(u64::MAX) {
        Err(Error::invalid_response(
            "monthly price ZIP exceeds the entry-count limit",
        ))
    } else {
        Ok(())
    }
}

fn zip_u16(bytes: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        bytes.get(offset..offset.checked_add(2)?)?.try_into().ok()?,
    ))
}

fn zip_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        bytes.get(offset..offset.checked_add(4)?)?.try_into().ok()?,
    ))
}

fn zip_u64(bytes: &[u8], offset: usize) -> Option<u64> {
    Some(u64::from_le_bytes(
        bytes.get(offset..offset.checked_add(8)?)?.try_into().ok()?,
    ))
}

fn read_bounded(reader: &mut impl Read, maximum: u64) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    let take_limit = maximum.saturating_add(1);
    reader
        .take(take_limit)
        .read_to_end(&mut bytes)
        .map_err(|_| Error::invalid_response("could not decompress monthly price ZIP entry"))?;
    if bytes.len() as u64 > maximum {
        return Err(Error::invalid_response(
            "monthly price ZIP entry exceeds an uncompressed-byte limit",
        ));
    }
    Ok(bytes)
}

fn check_entry_size(size: u64, maximum: u64) -> Result<()> {
    if size > maximum {
        Err(Error::invalid_response(
            "monthly price ZIP entry exceeds the per-entry uncompressed-byte limit",
        ))
    } else {
        Ok(())
    }
}

fn check_total_size(
    current: u64,
    additional: u64,
    limits: MonthlyPriceArchiveLimits,
) -> Result<u64> {
    let total = current.checked_add(additional).ok_or_else(|| {
        Error::invalid_response("monthly price ZIP total uncompressed size overflowed")
    })?;
    if total > limits.max_total_uncompressed_bytes {
        Err(Error::invalid_response(
            "monthly price ZIP exceeds the total uncompressed-byte limit",
        ))
    } else {
        Ok(total)
    }
}

fn check_compression_ratio(uncompressed: u64, compressed: u64, maximum: u64) -> Result<()> {
    let excessive = if uncompressed == 0 {
        false
    } else if compressed == 0 {
        true
    } else {
        u128::from(uncompressed) > u128::from(compressed) * u128::from(maximum)
    };
    if excessive {
        Err(Error::invalid_response(
            "monthly price ZIP entry exceeds the compression-ratio limit",
        ))
    } else {
        Ok(())
    }
}

fn safe_entry_name(file: &zip::read::ZipFile<'_, impl Read>) -> Result<String> {
    let name = file
        .name()
        .map_err(|_| Error::invalid_response("monthly ZIP entry name cannot be decoded"))?
        .into_owned();
    let raw = file.name_raw();
    let unsafe_raw = raw.iter().any(|byte| matches!(byte, 0 | b'\\'))
        || raw.starts_with(b"/")
        || raw
            .split(|byte| *byte == b'/')
            .any(|part| part.is_empty() || part == b"." || part == b"..");
    let unsafe_components = name.is_empty()
        || name.contains('\0')
        || name.contains('\\')
        || name.starts_with('/')
        || name.split('/').any(|part| {
            part.is_empty()
                || part == "."
                || part == ".."
                || part.contains(':')
                || part.chars().any(char::is_control)
        });
    let path = Path::new(&name);
    let unsafe_platform_path = path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
        || file.enclosed_name().as_deref() != Some(path);
    if unsafe_raw || unsafe_components || unsafe_platform_path || file.is_symlink() {
        return Err(Error::invalid_response(
            "monthly price ZIP contains an unsafe entry path",
        ));
    }
    Ok(name)
}

fn parse_xml<T>(
    bytes: &[u8],
    namespace: Option<&'static str>,
    root_name: &'static str,
    max_nodes: u32,
    max_depth: u32,
    decode: impl FnOnce(Node<'_, '_>, &str) -> Result<T>,
) -> Result<T> {
    let xml = std::str::from_utf8(bytes)
        .map_err(|_| Error::invalid_response("price report is not UTF-8 XML"))?;
    preflight(xml, max_nodes, max_depth)?;
    let document = Document::parse_with_options(
        xml,
        ParsingOptions {
            allow_dtd: false,
            nodes_limit: max_nodes,
            ..ParsingOptions::default()
        },
    )
    .map_err(|_| Error::invalid_response("malformed price-report XML"))?;
    let root = document.root_element();
    require_tag(root, namespace, root_name)?;
    decode(root, xml)
}

fn decode_cadastral_unit(node: Node<'_, '_>) -> Result<PriceCadastralUnit> {
    let district_node = required_child(node, None, "OKRES")?;
    let municipality_node = required_child(node, None, "OBEC")?;
    let cadastral_node = required_child(node, None, "KATUZE")?;
    let cadastral_area = match (
        optional_text(cadastral_node, None, "kod")?,
        optional_text(cadastral_node, None, "nazev")?,
    ) {
        (None, None) => None,
        (code, name) => Some(PriceCadastralArea {
            code: parse_required_numeric_text(code, "KATUZE/kod")?,
            name: name.ok_or_else(|| {
                Error::invalid_response("KATUZE/nazev is missing from a non-empty group")
            })?,
        }),
    };
    let properties = children(
        required_child(node, None, "NEMOVITOSTI")?,
        None,
        "NEMOVITOST",
    )
    .map(decode_property)
    .collect::<Result<Vec<_>>>()?;
    Ok(PriceCadastralUnit {
        district: PriceDistrict {
            nuts4: optional_text(district_node, None, "nuts4")?,
            name: optional_text(district_node, None, "nazev")?,
        },
        municipality: PriceMunicipality {
            code: required_parse(municipality_node, None, "kod")?,
            name: required_text(municipality_node, None, "nazev")?,
        },
        cadastral_area,
        properties,
    })
}

fn decode_property(node: Node<'_, '_>) -> Result<PriceProperty> {
    let components = children(node, None, "SOUCASTI_JE")
        .map(decode_property_ident)
        .collect::<Result<Vec<_>>>()?;
    if components.is_empty() {
        return Err(Error::invalid_response(
            "NEMOVITOST must contain at least one SOUCASTI_JE",
        ));
    }
    let parents = children(
        required_child(node, None, "NADRAZENE_NEMOVITOSTI")?,
        None,
        "NADRAZENA_NEMOVITOST",
    )
    .map(decode_parent_property)
    .collect::<Result<Vec<_>>>()?;
    Ok(PriceProperty {
        property_type: optional_text(node, None, "TYP_NEM_TEXT")?,
        identification: decode_property_ident(required_child(node, None, "NEM_IDENT")?)?,
        components,
        parents,
    })
}

fn decode_parent_property(node: Node<'_, '_>) -> Result<PriceParentProperty> {
    if !has_element_children(node) {
        return Ok(PriceParentProperty { details: None });
    }
    let identification = decode_property_ident(required_child(node, None, "NEM_IDENT")?)?;
    let components = children(node, None, "SOUCASTI_JE")
        .map(decode_property_ident)
        .collect::<Result<Vec<_>>>()?;
    if components.is_empty() {
        return Err(Error::invalid_response(
            "non-empty NADRAZENA_NEMOVITOST must contain at least one SOUCASTI_JE",
        ));
    }
    let deeds = optional_child(node, None, "LISTINY")?
        .map(|list| children(list, None, "LISTINA").map(decode_deed).collect())
        .transpose()?;
    Ok(PriceParentProperty {
        details: Some(PriceParentPropertyDetails {
            property_type: optional_text(node, None, "TYP_NEM_TEXT")?,
            identification,
            components,
            deeds,
        }),
    })
}

fn decode_property_ident(node: Node<'_, '_>) -> Result<PricePropertyIdent> {
    let items = node
        .children()
        .filter(|child| child.is_element() && child.tag_name().namespace().is_none())
        .map(|child| match child.tag_name().name() {
            "parcela" => decode_parcel(child).map(PricePropertyIdentItem::Parcel),
            "stavba" => decode_building(child).map(PricePropertyIdentItem::Building),
            "jednotka" => decode_unit(child).map(PricePropertyIdentItem::Unit),
            "pravo_stavby" => {
                decode_building_right(child).map(PricePropertyIdentItem::BuildingRight)
            }
            _ => Err(Error::invalid_response(
                "unknown property choice in legacy NEM_IDENT",
            )),
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(PricePropertyIdent { items })
}

fn decode_parcel(node: Node<'_, '_>) -> Result<PriceParcel> {
    Ok(PriceParcel {
        id: optional_legacy_id(node, "id")?,
        abbreviation: optional_text(node, None, "zkratka")?,
        land_kind_number: optional_text(node, None, "druh_cis")?,
        parcel_number: optional_text(node, None, "par_cis")?,
        subdivision_number: optional_text(node, None, "podd_cis")?,
        parcel_part: optional_parse(node, None, "dil_parcely")?,
        body_id: optional_legacy_id(node, "tel_id")?,
        body_number: optional_text(node, None, "cislo_tel")?,
        cadastral_area_name: optional_text(node, None, "ku_nazev")?,
        original_name: optional_text(node, None, "nazev_puv")?,
    })
}

fn decode_building(node: Node<'_, '_>) -> Result<PriceBuilding> {
    let parcels = optional_child(node, None, "par_ident")?
        .map(|wrapper| {
            children(wrapper, None, "parcela")
                .map(decode_parcel)
                .collect()
        })
        .transpose()?;
    Ok(PriceBuilding {
        id: optional_legacy_id(node, "id")?,
        municipality_part: optional_text(node, None, "caobce")?,
        building_type_abbreviation: optional_text(node, None, "typbud_zkr")?,
        house_numbers: optional_text(node, None, "cisla_domovni")?,
        use_abbreviation: optional_text(node, None, "vyuziti_zkr")?,
        body_id: optional_legacy_id(node, "tel_id")?,
        without_ownership_sheet: optional_text(node, None, "bez_lv")?,
        body_number: optional_text(node, None, "cislo_tel")?,
        cadastral_area_name: optional_text(node, None, "ku_nazev")?,
        temporary: optional_text(node, None, "docasna")?,
        parcels,
    })
}

fn decode_unit(node: Node<'_, '_>) -> Result<PriceUnit> {
    Ok(PriceUnit {
        id: required_legacy_id(node, "id")?,
        municipality_part: optional_text(node, None, "caobce")?,
        house_identification: optional_text(node, None, "cp_ident")?,
        unit_identification: optional_text(node, None, "jed_ident")?,
        body_id: optional_legacy_id(node, "tel_id")?,
        body_number: optional_text(node, None, "cislo_tel")?,
        cadastral_area_name: optional_text(node, None, "ku_nazev")?,
    })
}

fn decode_building_right(node: Node<'_, '_>) -> Result<PriceBuildingRight> {
    Ok(PriceBuildingRight {
        id: optional_legacy_id(node, "id")?,
        body_id: optional_legacy_id(node, "tel_id")?,
        parcels: children(node, None, "parcela")
            .map(decode_parcel)
            .collect::<Result<Vec<_>>>()?,
        body_number: optional_text(node, None, "cislo_tel")?,
        cadastral_area_name: optional_text(node, None, "ku_nazev")?,
    })
}

fn decode_deed(node: Node<'_, '_>) -> Result<PriceDeed> {
    let related_wrapper = required_child(node, None, "SOUVISEJICI_TYPY")?;
    Ok(PriceDeed {
        price: required_text(node, None, "CENA_Z_LISTINY")?,
        proceeding: decode_proceeding_reference(required_child(node, None, "IDENT_RIZENI")?)?,
        currency: required_text(node, None, "MENA_KOD")?,
        information: decode_deed_info(required_child(node, None, "LISTINA_INFO")?)?,
        related_types: children(related_wrapper, None, "SOUVISEJICI_TYP")
            .map(decode_related_type)
            .collect::<Result<Vec<_>>>()?,
    })
}

fn decode_proceeding_reference(node: Node<'_, '_>) -> Result<PriceProceedingReference> {
    let proceeding_type = optional_text(node, None, "typriz_kod")?;
    let sequence = optional_text(node, None, "poradove_cislo")?;
    let year = optional_text(node, None, "rok")?;
    let workplace = optional_text(node, None, "prares_kod")?;
    let identification = if proceeding_type.is_none()
        && sequence.is_none()
        && year.is_none()
        && workplace.is_none()
    {
        None
    } else {
        Some(PriceProceedingNumber {
            proceeding_type_code: proceeding_type.ok_or_else(|| {
                Error::invalid_response("typriz_kod is missing from IDENT_RIZENI")
            })?,
            sequence_number: parse_required_numeric_text(sequence, "IDENT_RIZENI/poradove_cislo")?,
            year: parse_required_numeric_text(year, "IDENT_RIZENI/rok")?,
            workplace_code: parse_required_numeric_text(workplace, "IDENT_RIZENI/prares_kod")?,
        })
    };
    Ok(PriceProceedingReference { identification })
}

fn decode_deed_info(node: Node<'_, '_>) -> Result<PriceDeedInfo> {
    let deed_type = optional_text(node, None, "tlist")?;
    let additional_data = optional_text(node, None, "dalsi_udaje")?;
    let creation_sequence = optional_text(node, None, "por_cis_zhot")?;
    let description = optional_text(node, None, "popis")?;
    let issued = optional_text(node, None, "vystav_txt")?;
    let legal_force = optional_text(node, None, "prav_moc_txt")?;
    let submission = optional_text(node, None, "podani_txt")?;
    let enforceability = optional_text(node, None, "vykonatelnost_txt")?;
    let submission_z = optional_text(node, None, "podani_z_txt")?;
    let validation = optional_text(node, None, "zplatneni_txt")?;
    let proceeding_type = optional_text(node, None, "prares_nazev")?;
    let empty = deed_type.is_none()
        && additional_data.is_none()
        && creation_sequence.is_none()
        && description.is_none()
        && issued.is_none()
        && legal_force.is_none()
        && submission.is_none()
        && enforceability.is_none()
        && submission_z.is_none()
        && validation.is_none()
        && proceeding_type.is_none();
    let details = if empty {
        None
    } else {
        Some(PriceDeedDetails {
            deed_type: required_group_text(deed_type, "LISTINA_INFO/tlist")?,
            additional_data: required_group_text(additional_data, "LISTINA_INFO/dalsi_udaje")?,
            creation_sequence_number: required_group_text(
                creation_sequence,
                "LISTINA_INFO/por_cis_zhot",
            )?,
            description: required_group_text(description, "LISTINA_INFO/popis")?,
            issued_text: required_group_text(issued, "LISTINA_INFO/vystav_txt")?,
            legal_force_text: required_group_text(legal_force, "LISTINA_INFO/prav_moc_txt")?,
            submission_text: required_group_text(submission, "LISTINA_INFO/podani_txt")?,
            enforceability_text: required_group_text(
                enforceability,
                "LISTINA_INFO/vykonatelnost_txt",
            )?,
            submission_z_text: required_group_text(submission_z, "LISTINA_INFO/podani_z_txt")?,
            validation_text: required_group_text(validation, "LISTINA_INFO/zplatneni_txt")?,
            proceeding_type_name: proceeding_type,
        })
    };
    Ok(PriceDeedInfo { details })
}

fn decode_related_type(node: Node<'_, '_>) -> Result<PriceRelatedType> {
    if !has_element_children(node) {
        return Ok(PriceRelatedType { details: None });
    }
    let properties = children(
        required_child(node, None, "SOUVISEJICI_NEMOVISTOSTI")?,
        None,
        "SOUVISEJICI_NEMOVISTOST",
    )
    .map(decode_related_property)
    .collect::<Result<Vec<_>>>()?;
    Ok(PriceRelatedType {
        details: Some(PriceRelatedTypeDetails {
            type_text: required_text(node, None, "TYP_TEXT")?,
            properties,
        }),
    })
}

fn decode_related_property(node: Node<'_, '_>) -> Result<PriceRelatedProperty> {
    if !has_element_children(node) {
        return Ok(PriceRelatedProperty { details: None });
    }
    let identification = decode_property_ident(required_child(node, None, "NEM_IDENT")?)?;
    let components = children(node, None, "SOUCASTI_JE")
        .map(decode_property_ident)
        .collect::<Result<Vec<_>>>()?;
    if components.is_empty() {
        return Err(Error::invalid_response(
            "non-empty SOUVISEJICI_NEMOVISTOST must contain at least one SOUCASTI_JE",
        ));
    }
    Ok(PriceRelatedProperty {
        details: Some(PriceRelatedPropertyDetails {
            identification,
            components,
        }),
    })
}

fn decode_workplace(node: Node<'_, '_>) -> Result<PriceWorkplace> {
    let proceedings = children(required_child(node, None, "RIZENI")?, None, "RIZ")
        .map(decode_proceeding)
        .collect::<Result<Vec<_>>>()?;
    Ok(PriceWorkplace {
        code: required_text(node, None, "KOD")?,
        name: required_text(node, None, "NAZEV")?,
        proceedings,
    })
}

fn decode_proceeding(node: Node<'_, '_>) -> Result<PriceProceeding> {
    let deeds = children(required_child(node, None, "LISTINY")?, None, "LISTINA")
        .map(decode_deed)
        .collect::<Result<Vec<_>>>()?;
    Ok(PriceProceeding {
        proceeding: decode_proceeding_reference(required_child(node, None, "IDENT_RIZENI")?)?,
        deeds,
    })
}

fn decode_monthly_data(root: Node<'_, '_>, _xml: &str) -> Result<MonthlyPriceData> {
    let without_measurement = required_attribute(root, "bez_mj")?
        .split_whitespace()
        .map(|value| match value {
            "a" => Ok(MonthlyYesNo::Yes),
            "n" => Ok(MonthlyYesNo::No),
            _ => Err(Error::invalid_response("invalid monthly bez_mj list item")),
        })
        .collect::<Result<Vec<_>>>()?;
    let months = required_attribute(root, "mesic")?
        .split_whitespace()
        .map(|value| {
            let month = value
                .parse::<u8>()
                .map_err(|_| Error::invalid_response("invalid monthly mesic list item"))?;
            if !(1..=12).contains(&month) {
                return Err(Error::invalid_response(
                    "monthly mesic list item is outside 1..=12",
                ));
            }
            Ok(month)
        })
        .collect::<Result<Vec<_>>>()?;
    let year = required_attribute(root, "rok")?;
    if !valid_gyear(year.trim()) {
        return Err(Error::invalid_response("invalid monthly xs:gYear"));
    }
    Ok(MonthlyPriceData {
        without_measurement,
        months,
        year: year.to_owned(),
        cadastral_area_code: parse_integer(
            required_attribute(root, "katuze_kod_cu")?,
            "katuze_kod_cu",
        )?,
        proceedings: children(root, Some(MONTHLY_NS), "Rizeni")
            .map(|node| decode_monthly_proceeding(node, _xml))
            .collect::<Result<Vec<_>>>()?,
    })
}

fn decode_monthly_proceeding(node: Node<'_, '_>, xml: &str) -> Result<MonthlyProceeding> {
    Ok(MonthlyProceeding {
        id: parse_integer(required_attribute(node, "id")?, "Rizeni/id")?,
        number: required_text(node, Some(MONTHLY_NS), "cisloRizeni")?,
        submitted_at: required_datetime(node, "datumPodani")?,
        validated_at: required_datetime(node, "datumZplatneni")?,
        workplace_code: required_parse(node, Some(MONTHLY_NS), "praresKod")?,
        deeds: decode_monthly_deeds(required_child(node, Some(MONTHLY_NS), "listiny")?)?,
        properties: decode_monthly_properties(
            required_child(node, Some(MONTHLY_NS), "nemovitosti")?,
            xml,
        )?,
    })
}

fn decode_monthly_deeds(node: Node<'_, '_>) -> Result<MonthlyDeedCollection> {
    let deeds = children(node, Some(MONTHLY_NS), "Listina")
        .map(decode_monthly_deed)
        .collect::<Result<Vec<_>>>()?;
    if deeds.is_empty() {
        return Err(Error::invalid_response(
            "monthly listiny must contain at least one Listina",
        ));
    }
    Ok(MonthlyDeedCollection {
        deeds,
        attributes: decode_attributes(node),
    })
}

fn decode_monthly_deed(node: Node<'_, '_>) -> Result<MonthlyDeed> {
    let price = required_text(node, Some(MONTHLY_NS), "cena")?;
    Ok(MonthlyDeed {
        id: parse_integer(required_attribute(node, "id")?, "Listina/id")?,
        price: response_decimal(price.trim(), "monthly Listina/cena")?,
        currency: required_text(node, Some(MONTHLY_NS), "mena")?,
        deed_type: required_text(node, Some(MONTHLY_NS), "listinaTyp")?,
        attributes: decode_attributes(node),
    })
}

fn decode_monthly_properties(node: Node<'_, '_>, xml: &str) -> Result<MonthlyPropertyCollection> {
    let properties = children(node, Some(MONTHLY_NS), "Nemovitost")
        .map(|property| decode_monthly_property(property, xml))
        .collect::<Result<Vec<_>>>()?;
    if properties.is_empty() {
        return Err(Error::invalid_response(
            "monthly nemovitosti must contain at least one Nemovitost",
        ));
    }
    Ok(MonthlyPropertyCollection {
        properties,
        attributes: decode_attributes(node),
    })
}

fn decode_monthly_property(node: Node<'_, '_>, xml: &str) -> Result<MonthlyProperty> {
    let gml_id = optional_nonempty_gml_id(node, "monthly property")?;
    let kinds = required_attribute(node, "typ")?
        .split_whitespace()
        .map(|value| match value {
            "PAR" => Ok(MonthlyPropertyKind::Parcel),
            "JED" => Ok(MonthlyPropertyKind::Unit),
            "BUD" => Ok(MonthlyPropertyKind::Building),
            "PS" => Ok(MonthlyPropertyKind::BuildingRight),
            _ => Err(Error::invalid_response(
                "invalid monthly property typ list item",
            )),
        })
        .collect::<Result<Vec<_>>>()?;
    let gml_elements = node
        .children()
        .filter(|child| child.is_element() && child.tag_name().namespace() == Some(GML_NS))
        .map(|child| raw_element(child, xml))
        .collect::<Result<Vec<_>>>()?;
    let body_node = required_child(node, Some(MONTHLY_NS), "teleso")?;
    Ok(MonthlyProperty {
        id: parse_integer(required_attribute(node, "id")?, "Nemovitost/id")?,
        gml_id,
        kinds,
        gml_elements,
        cadastral_area_code: required_parse(node, Some(MONTHLY_NS), "katuzeKod")?,
        text: optional_text(node, Some(MONTHLY_NS), "text")?,
        description: optional_text(node, Some(MONTHLY_NS), "popis")?,
        point: optional_child(node, Some(MONTHLY_NS), "bod")?
            .map(|point| decode_point_property(point, xml))
            .transpose()?,
        body: MonthlyPropertyBody {
            id: body_node
                .attribute("id")
                .map(|value| parse_integer(value, "teleso/id"))
                .transpose()?,
            number: required_parse(body_node, Some(MONTHLY_NS), "cislo")?,
        },
    })
}

fn decode_point_property(node: Node<'_, '_>, xml: &str) -> Result<MonthlyPointProperty> {
    Ok(MonthlyPointProperty {
        point: optional_child(node, Some(GML_NS), "Point")?
            .map(|point| decode_point(point, xml))
            .transpose()?,
        attributes: decode_attributes(node),
    })
}

fn decode_point(node: Node<'_, '_>, xml: &str) -> Result<MonthlyPoint> {
    let gml_id = optional_nonempty_gml_id(node, "monthly point")?;
    let position = match (
        optional_child(node, Some(GML_NS), "pos")?,
        optional_child(node, Some(GML_NS), "coordinates")?,
    ) {
        (Some(position), None) => MonthlyPointPosition::Position(decode_direct_position(position)?),
        (None, Some(coordinates)) => {
            MonthlyPointPosition::Coordinates(decode_legacy_coordinates(coordinates)?)
        }
        _ => {
            return Err(Error::invalid_response(
                "monthly GML Point must contain exactly one pos or coordinates element",
            ));
        }
    };
    let gml_elements = node
        .children()
        .filter(|child| {
            child.is_element()
                && child.tag_name().namespace() == Some(GML_NS)
                && !matches!(child.tag_name().name(), "pos" | "coordinates")
        })
        .map(|child| raw_element(child, xml))
        .collect::<Result<Vec<_>>>()?;
    Ok(MonthlyPoint {
        gml_id,
        attributes: decode_attributes(node),
        gml_elements,
        position,
    })
}

fn decode_direct_position(node: Node<'_, '_>) -> Result<MonthlyDirectPosition> {
    let raw = simple_content(node)?;
    let values = raw
        .split_whitespace()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if values.iter().any(|value| !valid_xsd_double(value)) {
        return Err(Error::invalid_response(
            "invalid xs:double token in monthly GML pos",
        ));
    }
    let coordinates = values
        .iter()
        .map(|value| crate::model::parse_decimal(value, "monthly GML coordinate").ok())
        .collect::<Option<Vec<_>>>();
    Ok(MonthlyDirectPosition {
        raw,
        values,
        coordinates,
        attributes: decode_attributes(node),
    })
}

fn valid_xsd_double(value: &str) -> bool {
    if matches!(value, "INF" | "-INF" | "NaN") {
        return true;
    }
    let (mantissa, exponent) = value.find(['e', 'E']).map_or((value, None), |index| {
        (&value[..index], Some(&value[index + 1..]))
    });
    valid_xsd_decimal_syntax(mantissa)
        && exponent.is_none_or(|exponent| {
            let digits = exponent.strip_prefix(['+', '-']).unwrap_or(exponent);
            !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
        })
}

fn valid_xsd_decimal_syntax(value: &str) -> bool {
    let unsigned = value.strip_prefix(['+', '-']).unwrap_or(value);
    let mut parts = unsigned.split('.');
    let whole = parts.next().unwrap_or_default();
    let fraction = parts.next();
    if parts.next().is_some() {
        return false;
    }
    let digits = |part: &str| part.bytes().all(|byte| byte.is_ascii_digit());
    match fraction {
        Some(fraction) => {
            digits(whole) && digits(fraction) && (!whole.is_empty() || !fraction.is_empty())
        }
        None => !whole.is_empty() && digits(whole),
    }
}

fn decode_legacy_coordinates(node: Node<'_, '_>) -> Result<MonthlyCoordinates> {
    let raw = simple_content(node)?;
    let decimal_separator = node.attribute("decimal").unwrap_or(".").to_owned();
    let coordinate_separator = node.attribute("cs").unwrap_or(",").to_owned();
    let tuple_separator = node.attribute("ts").unwrap_or(" ").to_owned();
    let numeric_tuples = decode_numeric_tuples(
        &raw,
        &decimal_separator,
        &coordinate_separator,
        &tuple_separator,
    )?;
    Ok(MonthlyCoordinates {
        raw,
        numeric_tuples,
        decimal_separator,
        coordinate_separator,
        tuple_separator,
        attributes: decode_attributes(node),
    })
}

fn decode_numeric_tuples(
    raw: &str,
    decimal_separator: &str,
    coordinate_separator: &str,
    tuple_separator: &str,
) -> Result<Option<Vec<Vec<Decimal>>>> {
    if decimal_separator.is_empty() || coordinate_separator.is_empty() || tuple_separator.is_empty()
    {
        return Ok(None);
    }
    let tuples: Vec<&str> = raw.split(tuple_separator).collect();
    let mut decoded = Vec::with_capacity(tuples.len());
    for tuple in tuples {
        let mut ordinates = Vec::new();
        for ordinate in tuple.split(coordinate_separator) {
            let normalized = if decimal_separator == "." {
                ordinate.trim().to_owned()
            } else {
                ordinate.trim().replace(decimal_separator, ".")
            };
            match crate::model::parse_decimal(&normalized, "monthly GML coordinate") {
                Ok(value) => ordinates.push(value),
                Err(_) => return Ok(None),
            }
        }
        decoded.push(ordinates);
    }
    Ok(Some(decoded))
}

fn raw_element(node: Node<'_, '_>, xml: &str) -> Result<MonthlyRawXmlElement> {
    Ok(MonthlyRawXmlElement {
        name: expanded_name(node),
        attributes: decode_attributes(node),
        xml: xml
            .get(node.range())
            .ok_or_else(|| Error::invalid_response("could not retain monthly GML XML"))?
            .to_owned(),
    })
}

fn decode_attributes(node: Node<'_, '_>) -> MonthlyXmlAttributes {
    MonthlyXmlAttributes {
        values: node
            .attributes()
            .map(|attribute| MonthlyXmlAttribute {
                name: MonthlyExpandedName {
                    namespace: attribute.namespace().map(str::to_owned),
                    local_name: attribute.name().to_owned(),
                },
                value: attribute.value().to_owned(),
            })
            .collect(),
    }
}

fn expanded_name(node: Node<'_, '_>) -> MonthlyExpandedName {
    MonthlyExpandedName {
        namespace: node.tag_name().namespace().map(str::to_owned),
        local_name: node.tag_name().name().to_owned(),
    }
}

fn response_decimal(raw: &str, field: &'static str) -> Result<Decimal> {
    crate::model::parse_decimal(raw, field).map_err(|error| match error {
        Error::DecimalOutOfRange { .. } => error,
        _ => Error::invalid_response(format!("invalid exact decimal in {field}")),
    })
}

fn required_datetime(node: Node<'_, '_>, name: &'static str) -> Result<XmlDateTime> {
    let raw = required_text(node, Some(MONTHLY_NS), name)?;
    XmlDateTime::new(raw.trim())
        .map_err(|_| Error::invalid_response(format!("invalid xs:dateTime in {name}")))
}

fn valid_gyear(value: &str) -> bool {
    if value.is_empty() {
        return false;
    }
    let zone_start = value
        .char_indices()
        .skip(1)
        .find(|(_, character)| matches!(character, '+' | '-'))
        .map(|(index, _)| index)
        .or_else(|| value.strip_suffix('Z').map(|_| value.len() - 1));
    let (year, zone) = zone_start.map_or((value, ""), |index| value.split_at(index));
    let digits = year.strip_prefix('-').unwrap_or(year);
    if digits.len() < 4
        || !digits.bytes().all(|byte| byte.is_ascii_digit())
        || digits.bytes().all(|byte| byte == b'0')
        || digits.len() > 4 && digits.starts_with('0')
    {
        return false;
    }
    if zone.is_empty() || zone == "Z" {
        return true;
    }
    let zone = &zone[1..];
    if zone.len() != 5 || zone.as_bytes().get(2) != Some(&b':') {
        return false;
    }
    let Ok(hour) = zone[..2].parse::<u8>() else {
        return false;
    };
    let Ok(minute) = zone[3..].parse::<u8>() else {
        return false;
    };
    hour < 14 && minute <= 59 || hour == 14 && minute == 0
}

fn optional_legacy_id(node: Node<'_, '_>, name: &'static str) -> Result<Option<LegacyId>> {
    optional_child(node, None, name)?
        .map(|node| {
            LegacyId::new(simple_content(node)?)
                .map_err(|_| Error::invalid_response("invalid legacy price-report ID_TYPE"))
        })
        .transpose()
}

fn required_legacy_id(node: Node<'_, '_>, name: &'static str) -> Result<LegacyId> {
    LegacyId::new(required_text(node, None, name)?)
        .map_err(|_| Error::invalid_response("invalid legacy price-report ID_TYPE"))
}

fn parse_required_numeric_text<T>(value: Option<String>, field: &'static str) -> Result<T>
where
    T: FromStr,
{
    let value = value.ok_or_else(|| {
        Error::invalid_response(format!("required numeric value {field} is missing"))
    })?;
    parse_integer(&value, field)
}

fn required_parse<T>(
    node: Node<'_, '_>,
    namespace: Option<&'static str>,
    name: &'static str,
) -> Result<T>
where
    T: FromStr,
{
    parse_integer(&required_text(node, namespace, name)?, name)
}

fn optional_parse<T>(
    node: Node<'_, '_>,
    namespace: Option<&'static str>,
    name: &'static str,
) -> Result<Option<T>>
where
    T: FromStr,
{
    optional_text(node, namespace, name)?
        .map(|value| parse_integer(&value, name))
        .transpose()
}

fn parse_integer<T>(raw: &str, field: &'static str) -> Result<T>
where
    T: FromStr,
{
    raw.trim()
        .parse::<T>()
        .map_err(|_| Error::invalid_response(format!("invalid numeric value in {field}")))
}

fn required_group_text(value: Option<String>, field: &'static str) -> Result<String> {
    value.ok_or_else(|| {
        Error::invalid_response(format!("incomplete legacy optional sequence at {field}"))
    })
}

fn required_attribute<'a, 'input>(node: Node<'a, 'input>, name: &'static str) -> Result<&'a str> {
    node.attribute(name)
        .ok_or_else(|| Error::invalid_response(format!("required attribute {name} is missing")))
}

fn optional_nonempty_gml_id(node: Node<'_, '_>, field: &'static str) -> Result<Option<String>> {
    node.attribute((GML_NS, "id"))
        .map(|value| {
            if value.is_empty() {
                Err(Error::invalid_response(format!("{field} gml:id is empty")))
            } else {
                Ok(value.to_owned())
            }
        })
        .transpose()
}

fn require_tag(
    node: Node<'_, '_>,
    namespace: Option<&'static str>,
    name: &'static str,
) -> Result<()> {
    if node.tag_name().namespace() == namespace && node.tag_name().name() == name {
        Ok(())
    } else {
        Err(Error::invalid_response(format!(
            "expected expanded XML root {name}"
        )))
    }
}

fn child<'a, 'input>(
    node: Node<'a, 'input>,
    namespace: Option<&'static str>,
    name: &'static str,
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

fn optional_child<'a, 'input>(
    node: Node<'a, 'input>,
    namespace: Option<&'static str>,
    name: &'static str,
) -> Result<Option<Node<'a, 'input>>> {
    child(node, namespace, name)
}

fn required_child<'a, 'input>(
    node: Node<'a, 'input>,
    namespace: Option<&'static str>,
    name: &'static str,
) -> Result<Node<'a, 'input>> {
    child(node, namespace, name)?
        .ok_or_else(|| Error::invalid_response(format!("required XML element {name} is missing")))
}

fn children<'a, 'input>(
    node: Node<'a, 'input>,
    namespace: Option<&'static str>,
    name: &'static str,
) -> impl Iterator<Item = Node<'a, 'input>> {
    node.children().filter(move |candidate| {
        candidate.is_element()
            && candidate.tag_name().namespace() == namespace
            && candidate.tag_name().name() == name
    })
}

fn has_element_children(node: Node<'_, '_>) -> bool {
    node.children().any(|child| child.is_element())
}

fn optional_text(
    node: Node<'_, '_>,
    namespace: Option<&'static str>,
    name: &'static str,
) -> Result<Option<String>> {
    optional_child(node, namespace, name)?
        .map(simple_content)
        .transpose()
}

fn required_text(
    node: Node<'_, '_>,
    namespace: Option<&'static str>,
    name: &'static str,
) -> Result<String> {
    simple_content(required_child(node, namespace, name)?)
}

fn simple_content(node: Node<'_, '_>) -> Result<String> {
    let mut content = String::new();
    for child in node.children() {
        if child.is_element() {
            return Err(Error::invalid_response(format!(
                "XML scalar element {} contains a nested element",
                node.tag_name().name()
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
                    Error::invalid_response("price XML exceeds the nesting limit")
                })?;
                if depth > max_depth {
                    return Err(Error::invalid_response(
                        "price XML exceeds the nesting limit",
                    ));
                }
            }
            Ok(Event::Empty(element)) => {
                if depth >= max_depth {
                    return Err(Error::invalid_response(
                        "price XML exceeds the nesting limit",
                    ));
                }
                nodes = preflight_node(nodes, max_nodes)?;
                nodes = preflight_attributes(&element, nodes, max_nodes)?;
            }
            Ok(Event::End(_)) => {
                depth = depth
                    .checked_sub(1)
                    .ok_or_else(|| Error::invalid_response("malformed price XML"))?;
            }
            Ok(Event::DocType(_)) => {
                return Err(Error::invalid_response("DTD is not allowed in price XML"));
            }
            Ok(
                Event::Text(_)
                | Event::CData(_)
                | Event::Comment(_)
                | Event::Decl(_)
                | Event::PI(_)
                | Event::GeneralRef(_),
            ) => nodes = preflight_node(nodes, max_nodes)?,
            Err(_) => return Err(Error::invalid_response("malformed price XML")),
        }
    }
    if depth != 0 {
        return Err(Error::invalid_response("malformed price XML"));
    }
    Ok(())
}

fn preflight_node(nodes: u32, maximum: u32) -> Result<u32> {
    let nodes = nodes
        .checked_add(1)
        .ok_or_else(|| Error::invalid_response("price XML exceeds the node limit"))?;
    if nodes > maximum {
        return Err(Error::invalid_response("price XML exceeds the node limit"));
    }
    Ok(nodes)
}

fn preflight_attributes(element: &BytesStart<'_>, mut nodes: u32, maximum: u32) -> Result<u32> {
    for attribute in element.attributes() {
        if attribute.is_err() {
            return Err(Error::invalid_response("malformed price XML"));
        }
        nodes = preflight_node(nodes, maximum)?;
    }
    Ok(nodes)
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use rust_decimal::Decimal;
    use zip::write::SimpleFileOptions;
    use zip::{CompressionMethod, ZipWriter};

    use super::*;

    const XLINK_NS: &str = "http://www.w3.org/1999/xlink";

    fn parse_monthly_prices(bytes: &[u8]) -> Result<MonthlyPriceData> {
        parse_monthly_prices_with_limits(bytes, DEFAULT_MAX_XML_NODES, DEFAULT_MAX_XML_DEPTH)
    }

    fn full_deed_info() -> &'static str {
        "<tlist>deed type</tlist><dalsi_udaje>additional &amp; data</dalsi_udaje><por_cis_zhot>0000000000000000001</por_cis_zhot><popis>description</popis><vystav_txt>issued 01.02.2026</vystav_txt><prav_moc_txt>force 02.02.2026</prav_moc_txt><podani_txt>submitted 03.02.2026</podani_txt><vykonatelnost_txt>enforceable 04.02.2026</vykonatelnost_txt><podani_z_txt>submitted-z 05.02.2026</podani_z_txt><zplatneni_txt>validated 06.02.2026</zplatneni_txt><prares_nazev>proceeding type</prares_nazev>"
    }

    fn properties_xml() -> String {
        format!(
            r#"<?xml version="1.0" encoding="utf-8"?>
<CenoveUdajeDleNemovitosti><KATASTRY><KATASTR>
<OKRES><nuts4>CZ0100</nuts4><nazev>Hlavní město Praha</nazev></OKRES>
<OBEC><kod>554782</kod><nazev>Praha</nazev></OBEC>
<KATUZE><kod>730475</kod><nazev>Kobylisy</nazev></KATUZE>
<NEMOVITOSTI><NEMOVITOST><TYP_NEM_TEXT> Parcel &amp; building </TYP_NEM_TEXT>
<NEM_IDENT>
<parcela><id>000123</id><zkratka>zkr</zkratka><druh_cis>01</druh_cis><par_cis> 00060 </par_cis><podd_cis> 02 </podd_cis><dil_parcely>-12</dil_parcely><tel_id></tel_id><cislo_tel>0001</cislo_tel><ku_nazev>Kobylisy</ku_nazev><nazev_puv>original</nazev_puv></parcela>
<stavba><id>456</id><caobce>Kobylisy</caobce><typbud_zkr>č.p.</typbud_zkr><cisla_domovni>001,002</cisla_domovni><vyuziti_zkr>ob.</vyuziti_zkr><tel_id>789</tel_id><bez_lv>n</bez_lv><cislo_tel>0002</cislo_tel><ku_nazev>Kobylisy</ku_nazev><docasna></docasna><par_ident><parcela><id>999999999999999999999999999999999999</id><par_cis> 7 </par_cis></parcela></par_ident></stavba>
</NEM_IDENT>
<SOUCASTI_JE><jednotka><id></id><caobce>part</caobce><cp_ident>cp 001</cp_ident><jed_ident>unit 0001</jed_ident><tel_id>1000</tel_id><cislo_tel>0003</cislo_tel><ku_nazev>Kobylisy</ku_nazev></jednotka></SOUCASTI_JE>
<SOUCASTI_JE><pravo_stavby><id>2000</id><tel_id>2001</tel_id><parcela><id>2002</id><cislo_tel>0004</cislo_tel></parcela><cislo_tel>0005</cislo_tel><ku_nazev>Kobylisy</ku_nazev></pravo_stavby></SOUCASTI_JE>
<NADRAZENE_NEMOVITOSTI>
<NADRAZENA_NEMOVITOST><TYP_NEM_TEXT></TYP_NEM_TEXT><NEM_IDENT><stavba><id>3000</id></stavba></NEM_IDENT><SOUCASTI_JE></SOUCASTI_JE><LISTINY><LISTINA>
<CENA_Z_LISTINY> 00000123456789012345678901234567890.000 </CENA_Z_LISTINY>
<IDENT_RIZENI><typriz_kod>V</typriz_kod><poradove_cislo>42</poradove_cislo><rok>2026</rok><prares_kod>1000</prares_kod></IDENT_RIZENI>
<MENA_KOD>CZK</MENA_KOD><LISTINA_INFO>{}</LISTINA_INFO>
<SOUVISEJICI_TYPY><SOUVISEJICI_TYP><TYP_TEXT>historical relation</TYP_TEXT><SOUVISEJICI_NEMOVISTOSTI><SOUVISEJICI_NEMOVISTOST><NEM_IDENT><parcela><id>4000</id><par_cis> 8 </par_cis></parcela></NEM_IDENT><SOUCASTI_JE><pravo_stavby><id></id></pravo_stavby></SOUCASTI_JE></SOUVISEJICI_NEMOVISTOST><SOUVISEJICI_NEMOVISTOST/></SOUVISEJICI_NEMOVISTOSTI></SOUVISEJICI_TYP><SOUVISEJICI_TYP/></SOUVISEJICI_TYPY>
</LISTINA></LISTINY></NADRAZENA_NEMOVITOST>
<NADRAZENA_NEMOVITOST/>
</NADRAZENE_NEMOVITOSTI></NEMOVITOST></NEMOVITOSTI>
</KATASTR></KATASTRY><BEZPL_PRISTUP>free &amp; public</BEZPL_PRISTUP><PLATNOST>20.11.2026 10:11</PLATNOST><VYHOTOVIL>tester</VYHOTOVIL><INFO_VYSTUP></INFO_VYSTUP><VYHOTOVENO>20.11.2026 12:13</VYHOTOVENO></CenoveUdajeDleNemovitosti>"#,
            full_deed_info()
        )
    }

    #[test]
    fn legacy_properties_retain_every_field_and_legacy_lexical_types() {
        let report = parse_prices_by_properties(properties_xml().as_bytes()).unwrap();
        assert_eq!(report.free_access, "free & public");
        assert_eq!(report.validity, "20.11.2026 10:11");
        assert_eq!(report.prepared_by, "tester");
        assert_eq!(report.output_info, "");
        assert_eq!(report.prepared_at, "20.11.2026 12:13");
        assert_eq!(report.cadastral_units.len(), 1);
        let unit = &report.cadastral_units[0];
        assert_eq!(unit.district.nuts4.as_deref(), Some("CZ0100"));
        assert_eq!(unit.district.name.as_deref(), Some("Hlavní město Praha"));
        assert_eq!(unit.municipality.code, 554_782);
        assert_eq!(unit.municipality.name, "Praha");
        assert_eq!(unit.cadastral_area.as_ref().unwrap().code, 730_475);
        assert_eq!(unit.cadastral_area.as_ref().unwrap().name, "Kobylisy");

        let property = &unit.properties[0];
        assert_eq!(
            property.property_type.as_deref(),
            Some(" Parcel & building ")
        );
        assert_eq!(property.identification.items.len(), 2);
        let PricePropertyIdentItem::Parcel(parcel) = &property.identification.items[0] else {
            panic!("expected parcel");
        };
        assert_eq!(parcel.id.as_ref().unwrap().as_str(), "000123");
        assert_eq!(parcel.abbreviation.as_deref(), Some("zkr"));
        assert_eq!(parcel.land_kind_number.as_deref(), Some("01"));
        assert_eq!(parcel.parcel_number.as_deref(), Some(" 00060 "));
        assert_eq!(parcel.subdivision_number.as_deref(), Some(" 02 "));
        assert_eq!(parcel.parcel_part, Some(-12));
        assert_eq!(parcel.body_id.as_ref().unwrap().as_str(), "");
        assert_eq!(parcel.body_number.as_deref(), Some("0001"));
        assert_eq!(parcel.cadastral_area_name.as_deref(), Some("Kobylisy"));
        assert_eq!(parcel.original_name.as_deref(), Some("original"));
        let PricePropertyIdentItem::Building(building) = &property.identification.items[1] else {
            panic!("expected building");
        };
        assert_eq!(building.id.as_ref().unwrap().as_str(), "456");
        assert_eq!(building.municipality_part.as_deref(), Some("Kobylisy"));
        assert_eq!(building.building_type_abbreviation.as_deref(), Some("č.p."));
        assert_eq!(building.house_numbers.as_deref(), Some("001,002"));
        assert_eq!(building.use_abbreviation.as_deref(), Some("ob."));
        assert_eq!(building.body_id.as_ref().unwrap().as_str(), "789");
        assert_eq!(building.without_ownership_sheet.as_deref(), Some("n"));
        assert_eq!(building.body_number.as_deref(), Some("0002"));
        assert_eq!(building.temporary.as_deref(), Some(""));
        let associated = &building.parcels.as_ref().unwrap()[0];
        assert_eq!(
            associated.id.as_ref().unwrap().as_str(),
            "999999999999999999999999999999999999"
        );
        assert_eq!(associated.parcel_number.as_deref(), Some(" 7 "));

        let PricePropertyIdentItem::Unit(unit_value) = &property.components[0].items[0] else {
            panic!("expected unit");
        };
        assert_eq!(unit_value.id.as_str(), "");
        assert_eq!(unit_value.municipality_part.as_deref(), Some("part"));
        assert_eq!(unit_value.house_identification.as_deref(), Some("cp 001"));
        assert_eq!(unit_value.unit_identification.as_deref(), Some("unit 0001"));
        assert_eq!(unit_value.body_id.as_ref().unwrap().as_str(), "1000");
        assert_eq!(unit_value.body_number.as_deref(), Some("0003"));
        let PricePropertyIdentItem::BuildingRight(right) = &property.components[1].items[0] else {
            panic!("expected building right");
        };
        assert_eq!(right.id.as_ref().unwrap().as_str(), "2000");
        assert_eq!(right.body_id.as_ref().unwrap().as_str(), "2001");
        assert_eq!(right.parcels[0].body_number.as_deref(), Some("0004"));
        assert_eq!(right.body_number.as_deref(), Some("0005"));

        assert_eq!(property.parents.len(), 2);
        let parent = property.parents[0].details.as_ref().unwrap();
        assert_eq!(parent.property_type.as_deref(), Some(""));
        assert_eq!(parent.components[0].items.len(), 0);
        assert_eq!(property.parents[1].details, None);
        let deeds = parent.deeds.as_ref().unwrap();
        let deed = &deeds[0];
        assert_eq!(deed.price, " 00000123456789012345678901234567890.000 ");
        assert_eq!(deed.currency, "CZK");
        let proceeding = deed.proceeding.identification.as_ref().unwrap();
        assert_eq!(proceeding.proceeding_type_code, "V");
        assert_eq!(proceeding.sequence_number, 42);
        assert_eq!(proceeding.year, 2026);
        assert_eq!(proceeding.workplace_code, 1000);
        let details = deed.information.details.as_ref().unwrap();
        assert_eq!(details.deed_type, "deed type");
        assert_eq!(details.additional_data, "additional & data");
        assert_eq!(details.creation_sequence_number, "0000000000000000001");
        assert_eq!(details.description, "description");
        assert_eq!(details.issued_text, "issued 01.02.2026");
        assert_eq!(details.legal_force_text, "force 02.02.2026");
        assert_eq!(details.submission_text, "submitted 03.02.2026");
        assert_eq!(details.enforceability_text, "enforceable 04.02.2026");
        assert_eq!(details.submission_z_text, "submitted-z 05.02.2026");
        assert_eq!(details.validation_text, "validated 06.02.2026");
        assert_eq!(
            details.proceeding_type_name.as_deref(),
            Some("proceeding type")
        );
        assert_eq!(deed.related_types.len(), 2);
        let relation = deed.related_types[0].details.as_ref().unwrap();
        assert_eq!(relation.type_text, "historical relation");
        assert_eq!(relation.properties.len(), 2);
        let related = relation.properties[0].details.as_ref().unwrap();
        let PricePropertyIdentItem::Parcel(related_parcel) = &related.identification.items[0]
        else {
            panic!("expected related parcel");
        };
        assert_eq!(related_parcel.parcel_number.as_deref(), Some(" 8 "));
        let PricePropertyIdentItem::BuildingRight(related_right) = &related.components[0].items[0]
        else {
            panic!("expected related building right");
        };
        assert_eq!(related_right.id.as_ref().unwrap().as_str(), "");
        assert_eq!(relation.properties[1].details, None);
        assert_eq!(deed.related_types[1].details, None);
    }

    #[test]
    fn legacy_optional_sequences_require_complete_nonempty_content() {
        let original = properties_xml();
        let mut without_main_components = original.clone();
        let components_start = without_main_components
            .find("<SOUCASTI_JE><jednotka>")
            .unwrap();
        let components_end = without_main_components
            .find("<NADRAZENE_NEMOVITOSTI>")
            .unwrap();
        without_main_components.replace_range(components_start..components_end, "");
        assert!(parse_prices_by_properties(without_main_components.as_bytes()).is_err());

        for invalid in [
            original.replace(
                "<NADRAZENA_NEMOVITOST/>",
                "<NADRAZENA_NEMOVITOST><NEM_IDENT/></NADRAZENA_NEMOVITOST>",
            ),
            original.replace(
                "<NADRAZENA_NEMOVITOST/>",
                "<NADRAZENA_NEMOVITOST><SOUCASTI_JE/></NADRAZENA_NEMOVITOST>",
            ),
            original.replace(
                "<SOUVISEJICI_TYP/>",
                "<SOUVISEJICI_TYP><TYP_TEXT/></SOUVISEJICI_TYP>",
            ),
            original.replace(
                "<SOUVISEJICI_TYP/>",
                "<SOUVISEJICI_TYP><SOUVISEJICI_NEMOVISTOSTI/></SOUVISEJICI_TYP>",
            ),
            original.replace(
                "<SOUVISEJICI_NEMOVISTOST/>",
                "<SOUVISEJICI_NEMOVISTOST><NEM_IDENT/></SOUVISEJICI_NEMOVISTOST>",
            ),
            original.replace(
                "<SOUVISEJICI_NEMOVISTOST/>",
                "<SOUVISEJICI_NEMOVISTOST><SOUCASTI_JE/></SOUVISEJICI_NEMOVISTOST>",
            ),
        ] {
            assert!(parse_prices_by_properties(invalid.as_bytes()).is_err());
        }
    }

    #[test]
    fn legacy_proceedings_retain_full_hierarchy_empty_ids_and_prices() {
        let xml = format!(
            r#"<CenoveUdajeDleRizeni><PRACOVISTE><KP><KOD>0001000</KOD><NAZEV>Workplace</NAZEV><RIZENI><RIZ><IDENT_RIZENI><typriz_kod>V</typriz_kod><poradove_cislo>9</poradove_cislo><rok>2025</rok><prares_kod>65535</prares_kod></IDENT_RIZENI><LISTINY><LISTINA><IDENT_RIZENI></IDENT_RIZENI><CENA_Z_LISTINY></CENA_Z_LISTINY><MENA_KOD>EUR</MENA_KOD><LISTINA_INFO>{}</LISTINA_INFO><SOUVISEJICI_TYPY><SOUVISEJICI_TYP><TYP_TEXT></TYP_TEXT><SOUVISEJICI_NEMOVISTOSTI><SOUVISEJICI_NEMOVISTOST><NEM_IDENT></NEM_IDENT><SOUCASTI_JE></SOUCASTI_JE></SOUVISEJICI_NEMOVISTOST></SOUVISEJICI_NEMOVISTOSTI></SOUVISEJICI_TYP></SOUVISEJICI_TYPY></LISTINA></LISTINY></RIZ><RIZ><IDENT_RIZENI></IDENT_RIZENI><LISTINY></LISTINY></RIZ></RIZENI></KP></PRACOVISTE><BEZPL_PRISTUP></BEZPL_PRISTUP><VYHOTOVIL>tester</VYHOTOVIL><INFO_VYSTUP>all fields</INFO_VYSTUP><VYHOTOVENO>31.12.2025 23:59</VYHOTOVENO></CenoveUdajeDleRizeni>"#,
            full_deed_info()
        );
        let report = parse_prices_by_proceeding(xml.as_bytes()).unwrap();
        assert_eq!(report.free_access, "");
        assert_eq!(report.prepared_by, "tester");
        assert_eq!(report.output_info, "all fields");
        assert_eq!(report.prepared_at, "31.12.2025 23:59");
        let workplace = &report.workplaces[0];
        assert_eq!(workplace.code, "0001000");
        assert_eq!(workplace.name, "Workplace");
        assert_eq!(workplace.proceedings.len(), 2);
        let first = &workplace.proceedings[0];
        assert_eq!(first.proceeding.identification.as_ref().unwrap().year, 2025);
        assert_eq!(
            first
                .proceeding
                .identification
                .as_ref()
                .unwrap()
                .workplace_code,
            u16::MAX
        );
        assert_eq!(first.deeds[0].price, "");
        assert_eq!(first.deeds[0].currency, "EUR");
        assert_eq!(first.deeds[0].proceeding.identification, None);
        assert!(first.deeds[0].information.details.is_some());
        let relation = first.deeds[0].related_types[0].details.as_ref().unwrap();
        assert_eq!(relation.type_text, "");
        assert!(
            relation.properties[0]
                .details
                .as_ref()
                .unwrap()
                .identification
                .items
                .is_empty()
        );
        assert!(workplace.proceedings[1].deeds.is_empty());
        assert_eq!(workplace.proceedings[1].proceeding.identification, None);
    }

    fn monthly_xml(month: u8, text: &str) -> String {
        format!(
            r##"<?xml version="1.0" encoding="UTF-8"?><cu:CenoveUdaje xmlns:cu="{MONTHLY_NS}" xmlns:gml="{GML_NS}" xmlns:xlink="{XLINK_NS}" bez_mj="a n" mesic="{month} 12" rok="2026+01:00" katuze_kod_cu="730475"><cu:Rizeni id="9223372036854775807"><cu:cisloRizeni>V-0001/2026</cu:cisloRizeni><cu:datumPodani>2026-01-02T03:04:05.123456+01:00</cu:datumPodani><cu:datumZplatneni>2026-02-03T04:05:06Z</cu:datumZplatneni><cu:praresKod>-100</cu:praresKod><cu:listiny xlink:href="#deed-1" xlink:type="simple" owns="true" nilReason="inapplicable" gml:remoteSchema="https://example.test/deed.xsd"><cu:Listina id="-9" owns="false" xlink:title="deed title"><cu:cena>12345678901234567890.1234</cu:cena><cu:mena>CZK</cu:mena><cu:listinaTyp>kupní smlouva</cu:listinaTyp></cu:Listina></cu:listiny><cu:nemovitosti owns="true" xlink:role="urn:properties"><cu:Nemovitost typ="PAR JED BUD PS" id="77" gml:id="feature-1"><gml:metaDataProperty about="urn:metadata"><gml:GenericMetaData gml:id="metadata-1">metadata</gml:GenericMetaData></gml:metaDataProperty><gml:description>GML &amp; feature</gml:description><gml:descriptionReference xlink:href="https://example.test/description"/><gml:identifier codeSpace="urn:authority">feature-code</gml:identifier><gml:name codeSpace="urn:names">feature name</gml:name><gml:boundedBy nilReason="inapplicable"><gml:Null>inapplicable</gml:Null></gml:boundedBy><gml:location><gml:LocationString>near</gml:LocationString></gml:location><cu:katuzeKod>730475</cu:katuzeKod><cu:text>{text}</cu:text><cu:popis>monthly description</cu:popis><cu:bod owns="true" xlink:title="point link" nilReason="unknown"><gml:Point gml:id="point-1" srsName="urn:ogc:def:crs:EPSG::5514" srsDimension="2" axisLabels="Y X" uomLabels="m m"><gml:description>point description</gml:description><gml:name>point name</gml:name><gml:pos srsName="urn:ogc:def:crs:EPSG::5514" srsDimension="2">-123456.70 987654321.00</gml:pos></gml:Point></cu:bod><cu:teleso id="88"><cu:cislo>1685</cu:cislo></cu:teleso></cu:Nemovitost></cu:nemovitosti></cu:Rizeni></cu:CenoveUdaje>"##
        )
    }

    fn empty_monthly_xml(month: u8) -> String {
        format!(
            r#"<CenoveUdaje xmlns="{MONTHLY_NS}" bez_mj="n" mesic="{month}" rok="-0001Z" katuze_kod_cu="-2"/>"#
        )
    }

    fn make_zip(entries: &[(&str, &[u8])], compression: CompressionMethod) -> Vec<u8> {
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        for (name, bytes) in entries {
            writer
                .start_file(
                    name,
                    SimpleFileOptions::default().compression_method(compression),
                )
                .unwrap();
            writer.write_all(bytes).unwrap();
        }
        writer.finish().unwrap().into_inner()
    }

    #[test]
    fn monthly_archive_retains_multiple_entries_attributes_gml_and_exact_decimals() {
        let first = monthly_xml(1, "monthly text");
        let second = empty_monthly_xml(2);
        let zip = make_zip(
            &[
                ("nested/first.xml", first.as_bytes()),
                ("second.xml", second.as_bytes()),
            ],
            CompressionMethod::Deflated,
        );
        let archive =
            parse_monthly_price_archive(&zip, MonthlyPriceArchiveLimits::default()).unwrap();
        assert_eq!(archive.original_bytes, zip);
        assert_eq!(archive.entries.len(), 2);
        assert_eq!(archive.entries[0].original_bytes, first.as_bytes());
        assert_eq!(archive.entries[1].original_bytes, second.as_bytes());
        assert_eq!(archive.entries[0].metadata.index, 0);
        assert_eq!(archive.entries[0].metadata.name, "nested/first.xml");
        assert_eq!(
            archive.entries[0].metadata.name_raw,
            b"nested/first.xml".to_vec()
        );
        assert_eq!(
            archive.entries[0].metadata.uncompressed_size,
            first.len() as u64
        );

        let data = &archive.entries[0].value;
        assert_eq!(
            data.without_measurement,
            vec![MonthlyYesNo::Yes, MonthlyYesNo::No]
        );
        assert_eq!(data.months, vec![1, 12]);
        assert_eq!(data.year, "2026+01:00");
        assert_eq!(data.cadastral_area_code, 730_475);
        let proceeding = &data.proceedings[0];
        assert_eq!(proceeding.id, i64::MAX);
        assert_eq!(proceeding.number, "V-0001/2026");
        assert_eq!(
            proceeding.submitted_at.as_str(),
            "2026-01-02T03:04:05.123456+01:00"
        );
        assert_eq!(proceeding.validated_at.as_str(), "2026-02-03T04:05:06Z");
        assert_eq!(proceeding.workplace_code, -100);
        assert_eq!(
            proceeding.deeds.attributes.get(Some(XLINK_NS), "href"),
            Some("#deed-1")
        );
        assert_eq!(
            proceeding
                .deeds
                .attributes
                .get(Some(GML_NS), "remoteSchema"),
            Some("https://example.test/deed.xsd")
        );
        assert_eq!(proceeding.deeds.attributes.get(None, "owns"), Some("true"));
        assert_eq!(
            proceeding.deeds.attributes.get(None, "nilReason"),
            Some("inapplicable")
        );
        let deed = &proceeding.deeds.deeds[0];
        assert_eq!(deed.id, -9);
        assert_eq!(
            deed.price,
            Decimal::from_str_exact("12345678901234567890.1234").unwrap()
        );
        assert_eq!(deed.currency, "CZK");
        assert_eq!(deed.deed_type, "kupní smlouva");
        assert_eq!(deed.attributes.get(None, "owns"), Some("false"));
        assert_eq!(
            deed.attributes.get(Some(XLINK_NS), "title"),
            Some("deed title")
        );
        assert_eq!(
            proceeding.properties.attributes.get(None, "owns"),
            Some("true")
        );
        assert_eq!(
            proceeding.properties.attributes.get(Some(XLINK_NS), "role"),
            Some("urn:properties")
        );

        let property = &proceeding.properties.properties[0];
        assert_eq!(property.id, 77);
        assert_eq!(property.gml_id.as_deref(), Some("feature-1"));
        assert_eq!(
            property.kinds,
            vec![
                MonthlyPropertyKind::Parcel,
                MonthlyPropertyKind::Unit,
                MonthlyPropertyKind::Building,
                MonthlyPropertyKind::BuildingRight,
            ]
        );
        assert_eq!(property.gml_elements.len(), 7);
        assert_eq!(property.gml_elements[0].name.local_name, "metaDataProperty");
        assert_eq!(
            property.gml_elements[0].attributes.get(None, "about"),
            Some("urn:metadata")
        );
        assert!(property.gml_elements[0].xml.contains("GenericMetaData"));
        assert!(property.gml_elements[1].xml.contains("GML &amp; feature"));
        assert_eq!(property.gml_elements[4].name.local_name, "name");
        assert_eq!(property.gml_elements[5].name.local_name, "boundedBy");
        assert_eq!(property.gml_elements[6].name.local_name, "location");
        assert_eq!(property.cadastral_area_code, 730_475);
        assert_eq!(property.text.as_deref(), Some("monthly text"));
        assert_eq!(property.description.as_deref(), Some("monthly description"));
        assert_eq!(property.body.id, Some(88));
        assert_eq!(property.body.number, 1685);
        let point_property = property.point.as_ref().unwrap();
        assert_eq!(point_property.attributes.get(None, "owns"), Some("true"));
        assert_eq!(
            point_property.attributes.get(Some(XLINK_NS), "title"),
            Some("point link")
        );
        assert_eq!(
            point_property.attributes.get(None, "nilReason"),
            Some("unknown")
        );
        let point = point_property.point.as_ref().unwrap();
        assert_eq!(point.gml_id.as_deref(), Some("point-1"));
        assert_eq!(point.attributes.get(Some(GML_NS), "id"), Some("point-1"));
        assert_eq!(
            point.attributes.get(None, "srsName"),
            Some("urn:ogc:def:crs:EPSG::5514")
        );
        assert_eq!(point.gml_elements.len(), 2);
        let MonthlyPointPosition::Position(position) = &point.position else {
            panic!("expected gml:pos");
        };
        assert_eq!(position.raw, "-123456.70 987654321.00");
        assert_eq!(position.values, ["-123456.70", "987654321.00"]);
        assert_eq!(
            position.coordinates,
            Some(vec![
                Decimal::from_str_exact("-123456.70").unwrap(),
                Decimal::from_str_exact("987654321.00").unwrap(),
            ])
        );
        assert_eq!(position.attributes.get(None, "srsDimension"), Some("2"));

        let second_data = &archive.entries[1].value;
        assert_eq!(second_data.without_measurement, vec![MonthlyYesNo::No]);
        assert_eq!(second_data.months, vec![2]);
        assert_eq!(second_data.year, "-0001Z");
        assert_eq!(second_data.cadastral_area_code, -2);
        assert!(second_data.proceedings.is_empty());
    }

    #[test]
    fn monthly_gml_ids_follow_the_optional_xsd_attributes() {
        let missing = monthly_xml(3, "missing IDs")
            .replace(" gml:id=\"feature-1\"", "")
            .replace(" gml:id=\"point-1\"", "");
        let parsed = parse_monthly_prices(missing.as_bytes()).unwrap();
        let property = &parsed.proceedings[0].properties.properties[0];
        assert_eq!(property.gml_id, None);
        assert_eq!(
            property
                .point
                .as_ref()
                .unwrap()
                .point
                .as_ref()
                .unwrap()
                .gml_id,
            None
        );

        for xml in [
            monthly_xml(3, "empty feature ID").replace("gml:id=\"feature-1\"", "gml:id=\"\""),
            monthly_xml(3, "empty point ID").replace("gml:id=\"point-1\"", "gml:id=\"\""),
        ] {
            assert!(parse_monthly_prices(xml.as_bytes()).is_err());
        }
    }

    #[test]
    fn monthly_deprecated_coordinates_are_exact_or_raw_preserved() {
        let original = monthly_xml(3, "coordinates");
        let xml = original.replace(
            "<gml:pos srsName=\"urn:ogc:def:crs:EPSG::5514\" srsDimension=\"2\">-123456.70 987654321.00</gml:pos>",
            "<gml:coordinates decimal=\",\" cs=\";\" ts=\"|\">-1,25;2,50|3,00;4,00</gml:coordinates>",
        );
        let parsed = parse_monthly_prices(xml.as_bytes()).unwrap();
        let property = &parsed.proceedings[0].properties.properties[0];
        let point = property.point.as_ref().unwrap().point.as_ref().unwrap();
        let MonthlyPointPosition::Coordinates(coordinates) = &point.position else {
            panic!("expected gml:coordinates");
        };
        assert_eq!(
            coordinates.numeric_tuples,
            Some(vec![
                vec![
                    Decimal::from_str_exact("-1.25").unwrap(),
                    Decimal::from_str_exact("2.50").unwrap(),
                ],
                vec![
                    Decimal::from_str_exact("3.00").unwrap(),
                    Decimal::from_str_exact("4.00").unwrap(),
                ],
            ])
        );
        assert_eq!(coordinates.raw, "-1,25;2,50|3,00;4,00");

        let mixed = original.replace(
            "<gml:pos srsName=\"urn:ogc:def:crs:EPSG::5514\" srsDimension=\"2\">-123456.70 987654321.00</gml:pos>",
            "<gml:coordinates>1,text</gml:coordinates>",
        );
        let parsed = parse_monthly_prices(mixed.as_bytes()).unwrap();
        let property = &parsed.proceedings[0].properties.properties[0];
        let point = property.point.as_ref().unwrap().point.as_ref().unwrap();
        let MonthlyPointPosition::Coordinates(coordinates) = &point.position else {
            panic!("expected gml:coordinates");
        };
        assert_eq!(coordinates.numeric_tuples, None);
        assert_eq!(coordinates.raw, "1,text");
    }

    #[test]
    fn monthly_coordinates_honor_exact_whitespace_tuple_separators() {
        let original = monthly_xml(3, "coordinate separators");
        let tabbed = original.replace(
            "<gml:pos srsName=\"urn:ogc:def:crs:EPSG::5514\" srsDimension=\"2\">-123456.70 987654321.00</gml:pos>",
            "<gml:coordinates ts=\"&#9;\">1,2&#9;3,4</gml:coordinates>",
        );
        let parsed = parse_monthly_prices(tabbed.as_bytes()).unwrap();
        let point = parsed.proceedings[0].properties.properties[0]
            .point
            .as_ref()
            .unwrap()
            .point
            .as_ref()
            .unwrap();
        let MonthlyPointPosition::Coordinates(coordinates) = &point.position else {
            panic!("expected gml:coordinates");
        };
        assert_eq!(coordinates.tuple_separator, "\t");
        assert_eq!(coordinates.raw, "1,2\t3,4");
        assert_eq!(
            coordinates.numeric_tuples,
            Some(vec![
                vec![Decimal::ONE, Decimal::TWO],
                vec![Decimal::from(3), Decimal::from(4)],
            ])
        );

        for (attributes, raw) in [(" ts=\"&#9;\"", "1,2&#9;3,4\n5,6"), ("", "1,2\t3,4")] {
            let xml = original.replace(
                "<gml:pos srsName=\"urn:ogc:def:crs:EPSG::5514\" srsDimension=\"2\">-123456.70 987654321.00</gml:pos>",
                &format!("<gml:coordinates{attributes}>{raw}</gml:coordinates>"),
            );
            let parsed = parse_monthly_prices(xml.as_bytes()).unwrap();
            let point = parsed.proceedings[0].properties.properties[0]
                .point
                .as_ref()
                .unwrap()
                .point
                .as_ref()
                .unwrap();
            let MonthlyPointPosition::Coordinates(coordinates) = &point.position else {
                panic!("expected gml:coordinates");
            };
            assert_eq!(coordinates.numeric_tuples, None);
        }

        let oversized = original.replace(
            "<gml:pos srsName=\"urn:ogc:def:crs:EPSG::5514\" srsDimension=\"2\">-123456.70 987654321.00</gml:pos>",
            "<gml:coordinates>999999999999999999999999999999999999999,2</gml:coordinates>",
        );
        let parsed = parse_monthly_prices(oversized.as_bytes()).unwrap();
        let point = parsed.proceedings[0].properties.properties[0]
            .point
            .as_ref()
            .unwrap()
            .point
            .as_ref()
            .unwrap();
        let MonthlyPointPosition::Coordinates(coordinates) = &point.position else {
            panic!("expected gml:coordinates");
        };
        assert_eq!(coordinates.numeric_tuples, None);
    }

    #[test]
    fn monthly_prices_never_round_or_accept_exponents() {
        let original = monthly_xml(1, "decimal limits");
        for price in [
            "999999999999999999999999999999999999999",
            "0.00000000000000000000000000001",
        ] {
            let xml = original.replace("12345678901234567890.1234", price);
            assert!(matches!(
                parse_monthly_prices(xml.as_bytes()),
                Err(Error::DecimalOutOfRange {
                    field: "monthly Listina/cena"
                })
            ));
        }
        for price in ["1e2", "1_000", "NaN"] {
            let xml = original.replace("12345678901234567890.1234", price);
            assert!(parse_monthly_prices(xml.as_bytes()).is_err());
        }
    }

    #[test]
    fn monthly_pos_preserves_double_tokens_and_optional_exact_decimals() {
        let original = monthly_xml(1, "double positions");
        for lexical in [
            "1e2",
            "+1.5E-2",
            "INF",
            "-INF",
            "NaN",
            "999999999999999999999999999999999999999",
            "0.00000000000000000000000000001",
        ] {
            let xml = original.replace("-123456.70 987654321.00", lexical);
            let parsed = parse_monthly_prices(xml.as_bytes()).unwrap();
            let point = parsed.proceedings[0].properties.properties[0]
                .point
                .as_ref()
                .unwrap()
                .point
                .as_ref()
                .unwrap();
            let MonthlyPointPosition::Position(position) = &point.position else {
                panic!("expected gml:pos");
            };
            assert_eq!(position.raw, lexical);
            assert_eq!(position.values, [lexical]);
            assert_eq!(position.coordinates, None);
        }

        let mixed = original.replace("-123456.70 987654321.00", "1.25 2E3");
        let parsed = parse_monthly_prices(mixed.as_bytes()).unwrap();
        let point = parsed.proceedings[0].properties.properties[0]
            .point
            .as_ref()
            .unwrap()
            .point
            .as_ref()
            .unwrap();
        let MonthlyPointPosition::Position(position) = &point.position else {
            panic!("expected gml:pos");
        };
        assert_eq!(position.values, ["1.25", "2E3"]);
        assert_eq!(position.coordinates, None);

        for invalid in ["1_000", "+INF", "inf", "1e", "--1"] {
            let xml = original.replace("-123456.70", invalid);
            assert!(parse_monthly_prices(xml.as_bytes()).is_err(), "{invalid}");
        }
    }

    #[test]
    fn monthly_gyear_rejects_zero_padded_extended_years() {
        for year in ["02026", "-02026"] {
            let xml = empty_monthly_xml(1).replace("-0001Z", year);
            assert!(parse_monthly_prices(xml.as_bytes()).is_err(), "{year}");
        }
        for year in ["10000", "-10000Z"] {
            let xml = empty_monthly_xml(1).replace("-0001Z", year);
            assert!(parse_monthly_prices(xml.as_bytes()).is_ok(), "{year}");
        }
    }

    #[test]
    fn archive_rejects_unsafe_paths_and_unknown_roots() {
        let monthly = empty_monthly_xml(1);
        for path in [
            "../escape.xml",
            "/absolute.xml",
            "..\\escape.xml",
            "C:\\escape.xml",
            "C:/escape.xml",
            "folder:name/entry.xml",
            "entry.xml:ads",
            "folder/./entry.xml",
            "folder//entry.xml",
        ] {
            let zip = make_zip(&[(path, monthly.as_bytes())], CompressionMethod::Stored);
            assert!(
                parse_monthly_price_archive(&zip, MonthlyPriceArchiveLimits::default()).is_err(),
                "unsafe path was accepted: {path}"
            );
        }
        let harmless = make_zip(
            &[("monthly..xml", monthly.as_bytes())],
            CompressionMethod::Stored,
        );
        assert!(
            parse_monthly_price_archive(&harmless, MonthlyPriceArchiveLimits::default()).is_ok()
        );
        let unknown = make_zip(&[("unknown.xml", b"<Unknown/>")], CompressionMethod::Stored);
        assert!(
            parse_monthly_price_archive(&unknown, MonthlyPriceArchiveLimits::default()).is_err()
        );
        assert!(
            parse_monthly_prices(
                empty_monthly_xml(1)
                    .replace(&format!(" xmlns=\"{MONTHLY_NS}\""), "")
                    .as_bytes()
            )
            .is_err()
        );
        assert!(
            parse_prices_by_properties(
                properties_xml()
                    .replace(
                        "<CenoveUdajeDleNemovitosti>",
                        "<CenoveUdajeDleNemovitosti xmlns=\"urn:wrong\">"
                    )
                    .as_bytes()
            )
            .is_err()
        );
    }

    #[test]
    fn archive_entry_budget_preflights_eocd_and_zip64_before_opening() {
        let mut eocd = vec![0_u8; 22];
        eocd[..4].copy_from_slice(b"PK\x05\x06");
        eocd[8..10].copy_from_slice(&1_u16.to_le_bytes());
        eocd[10..12].copy_from_slice(&1_u16.to_le_bytes());
        let error = parse_monthly_price_archive(
            &eocd,
            MonthlyPriceArchiveLimits {
                max_entries: 0,
                ..MonthlyPriceArchiveLimits::default()
            },
        )
        .expect_err("the forged EOCD count must be rejected before ZIP opening");
        assert!(matches!(
            error,
            Error::InvalidResponse { message }
                if message == "monthly price ZIP exceeds the entry-count limit"
        ));

        let mut zip64 = vec![0_u8; 56];
        zip64[..4].copy_from_slice(b"PK\x06\x06");
        zip64[4..12].copy_from_slice(&44_u64.to_le_bytes());
        zip64[24..32].copy_from_slice(&65_536_u64.to_le_bytes());
        zip64[32..40].copy_from_slice(&65_536_u64.to_le_bytes());
        zip64.extend_from_slice(&[0_u8; 20]);
        zip64[56..60].copy_from_slice(b"PK\x06\x07");
        zip64[64..72].copy_from_slice(&0_u64.to_le_bytes());
        zip64.extend_from_slice(&[0_u8; 22]);
        zip64[76..80].copy_from_slice(b"PK\x05\x06");
        zip64[84..86].copy_from_slice(&u16::MAX.to_le_bytes());
        zip64[86..88].copy_from_slice(&u16::MAX.to_le_bytes());
        let error = parse_monthly_price_archive(
            &zip64,
            MonthlyPriceArchiveLimits {
                max_entries: usize::from(u16::MAX),
                ..MonthlyPriceArchiveLimits::default()
            },
        )
        .expect_err("the forged ZIP64 count must be rejected before ZIP opening");
        assert!(matches!(
            error,
            Error::InvalidResponse { message }
                if message == "monthly price ZIP exceeds the entry-count limit"
        ));
    }

    #[test]
    fn archive_enforces_entry_count_per_entry_total_and_ratio_limits() {
        let first = empty_monthly_xml(1);
        let second = empty_monthly_xml(2);
        let zip = make_zip(
            &[
                ("first.xml", first.as_bytes()),
                ("second.xml", second.as_bytes()),
            ],
            CompressionMethod::Stored,
        );
        let limits = MonthlyPriceArchiveLimits {
            max_entries: 1,
            ..MonthlyPriceArchiveLimits::default()
        };
        assert!(parse_monthly_price_archive(&zip, limits).is_err());

        let limits = MonthlyPriceArchiveLimits {
            max_entry_uncompressed_bytes: first.len() as u64 - 1,
            ..MonthlyPriceArchiveLimits::default()
        };
        assert!(parse_monthly_price_archive(&zip, limits).is_err());

        let limits = MonthlyPriceArchiveLimits {
            max_total_uncompressed_bytes: (first.len() + second.len()) as u64 - 1,
            ..MonthlyPriceArchiveLimits::default()
        };
        assert!(parse_monthly_price_archive(&zip, limits).is_err());

        let large = monthly_xml(1, &"A".repeat(100_000));
        let compressed = make_zip(
            &[("large.xml", large.as_bytes())],
            CompressionMethod::Deflated,
        );
        let limits = MonthlyPriceArchiveLimits {
            max_compression_ratio: 2,
            ..MonthlyPriceArchiveLimits::default()
        };
        assert!(parse_monthly_price_archive(&compressed, limits).is_err());
        assert!(
            parse_monthly_price_archive(&compressed, MonthlyPriceArchiveLimits::default()).is_ok()
        );
    }

    #[test]
    fn xml_rejects_dtd_duplicate_singletons_and_complexity_over_limits() {
        let dtd = "<!DOCTYPE CenoveUdajeDleRizeni><CenoveUdajeDleRizeni/>";
        assert!(parse_prices_by_proceeding(dtd.as_bytes()).is_err());
        let duplicate = properties_xml().replace(
            "<BEZPL_PRISTUP>free &amp; public</BEZPL_PRISTUP>",
            "<BEZPL_PRISTUP>free &amp; public</BEZPL_PRISTUP><BEZPL_PRISTUP/>",
        );
        assert!(parse_prices_by_properties(duplicate.as_bytes()).is_err());
        assert!(
            parse_prices_by_properties_with_limits(properties_xml().as_bytes(), 1, 256).is_err()
        );
        assert!(
            parse_prices_by_properties_with_limits(properties_xml().as_bytes(), 2_000_000, 1)
                .is_err()
        );
    }
}
