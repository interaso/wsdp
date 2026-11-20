use roxmltree::{Document, Node, ParsingOptions};
use rust_decimal::Decimal;

use crate::model::reports::account_statement::{
    AccountChargeGroup, AccountChargeRow, AccountParticipant, AccountProceeding,
    AccountStatementReport, CustomerAccount,
};
use crate::{Error, LegacyId, Result};

const XSI_NS: &str = "http://www.w3.org/2001/XMLSchema-instance";
const ROOT_SEQUENCE: &[&str] = &["UCET", "DATUM_OD", "DATUM_DO", "VYHOTOVIL", "VYHOTOVENO"];
const ACCOUNT_SEQUENCE: &[&str] = &[
    "UCET_USERNAME",
    "CISLO_UCTU",
    "C_RIZENI",
    "DAT_FAKTURACE",
    "KREDIT",
    "UCASTNIK",
    "SKUPINY",
    "CENA_CELKEM",
];
const PROCEEDING_SEQUENCE: &[&str] = &["typriz_kod", "poradove_cislo", "rok", "prares_kod"];
const PARTICIPANT_SEQUENCE: &[&str] = &[
    "titul_pred_jmenem",
    "prijmeni",
    "jmeno",
    "titul_za_jmenem",
    "obchodni_jmeno",
];
const GROUP_SEQUENCE: &[&str] = &["NAZEV", "RADKY"];
const ROW_SEQUENCE: &[&str] = &[
    "UZIVP_USERNAME",
    "CJ",
    "UCEL",
    "IDENT",
    "DAT_SPUST",
    "ZC_UCTU",
    "FUNKCE",
    "MJ",
    "STRANY",
    "CENA",
    "KOD",
    "ID_PODRIZENE_SESTAVY",
];

/// Parses a complete account-statement report with bounded XML complexity.
pub(crate) fn parse_account_statement(
    bytes: &[u8],
    max_nodes: u32,
    max_depth: u32,
) -> Result<AccountStatementReport> {
    let xml = std::str::from_utf8(bytes)
        .map_err(|_| Error::invalid_response("account statement is not UTF-8"))?;
    let document = Document::parse_with_options(
        xml,
        ParsingOptions {
            allow_dtd: false,
            nodes_limit: max_nodes,
            ..ParsingOptions::default()
        },
    )
    .map_err(|_| Error::invalid_response("malformed account-statement XML"))?;
    check_depth(document.root_element(), 1, max_depth)?;
    let root = document.root_element();
    if root.tag_name().namespace().is_some()
        || root.tag_name().name() != "VypisStavuZakaznickehoUctu"
    {
        return Err(Error::invalid_response("unexpected account-statement root"));
    }
    validate_sequence(root, ROOT_SEQUENCE)?;

    let account_node = required_child(root, "UCET")?;
    validate_sequence(account_node, ACCOUNT_SEQUENCE)?;
    let participant_node = required_child(account_node, "UCASTNIK")?;
    validate_sequence(participant_node, PARTICIPANT_SEQUENCE)?;
    let organization = optional_child(participant_node, "obchodni_jmeno")?;
    let title_before = optional_child(participant_node, "titul_pred_jmenem")?;
    let family_name = optional_child(participant_node, "prijmeni")?;
    let given_name = optional_child(participant_node, "jmeno")?;
    let title_after = optional_child(participant_node, "titul_za_jmenem")?;
    let person_present = title_before.is_some()
        || family_name.is_some()
        || given_name.is_some()
        || title_after.is_some();
    let participant = match (organization, person_present) {
        (Some(organization), false) => {
            AccountParticipant::Organization(text(organization, "obchodni_jmeno")?)
        }
        (None, true) => AccountParticipant::Person {
            title_before: title_before
                .map(|node| text(node, "titul_pred_jmenem"))
                .transpose()?,
            family_name: text(
                family_name.ok_or_else(|| {
                    Error::invalid_response("account person requires prijmeni and jmeno")
                })?,
                "prijmeni",
            )?,
            given_name: text(
                given_name.ok_or_else(|| {
                    Error::invalid_response("account person requires prijmeni and jmeno")
                })?,
                "jmeno",
            )?,
            title_after: title_after
                .map(|node| text(node, "titul_za_jmenem"))
                .transpose()?,
        },
        _ => {
            return Err(Error::invalid_response(
                "account participant must contain exactly one person or organization choice",
            ));
        }
    };

    let proceeding_node = required_child(account_node, "C_RIZENI")?;
    validate_sequence(proceeding_node, PROCEEDING_SEQUENCE)?;
    let proceeding_text = proceeding_node
        .children()
        .filter(|node| node.is_text())
        .filter_map(|node| node.text())
        .collect::<String>();
    let groups_node = required_child(account_node, "SKUPINY")?;
    validate_repeated(groups_node, "SKUPINA")?;
    let groups = element_children(groups_node)
        .map(parse_group)
        .collect::<Result<Vec<_>>>()?;

    Ok(AccountStatementReport {
        account: CustomerAccount {
            username: required_text(account_node, "UCET_USERNAME")?,
            number: decimal(
                required_text(account_node, "CISLO_UCTU")?,
                "CISLO_UCTU",
                Some(0),
            )?,
            proceeding: AccountProceeding {
                text: proceeding_text,
                kind: required_text(proceeding_node, "typriz_kod")?,
                sequence: parse_number(
                    required_text(proceeding_node, "poradove_cislo")?,
                    "poradove_cislo",
                )?,
                year: parse_number(required_text(proceeding_node, "rok")?, "rok")?,
                workplace: parse_number(
                    required_text(proceeding_node, "prares_kod")?,
                    "prares_kod",
                )?,
            },
            billing_date: required_text(account_node, "DAT_FAKTURACE")?,
            credit: required_text(account_node, "KREDIT")?,
            participant,
            groups,
            total: decimal(
                required_text(account_node, "CENA_CELKEM")?,
                "CENA_CELKEM",
                Some(2),
            )?,
        },
        from: required_text(root, "DATUM_OD")?,
        to: required_text(root, "DATUM_DO")?,
        created_by: required_text(root, "VYHOTOVIL")?,
        created_at: required_text(root, "VYHOTOVENO")?,
    })
}

fn parse_group(group: Node<'_, '_>) -> Result<AccountChargeGroup> {
    validate_sequence(group, GROUP_SEQUENCE)?;
    let rows_node = required_child(group, "RADKY")?;
    validate_repeated(rows_node, "RADEK")?;
    Ok(AccountChargeGroup {
        name: required_text(group, "NAZEV")?,
        rows: element_children(rows_node)
            .map(parse_row)
            .collect::<Result<Vec<_>>>()?,
    })
}

fn parse_row(row: Node<'_, '_>) -> Result<AccountChargeRow> {
    validate_sequence(row, ROW_SEQUENCE)?;
    Ok(AccountChargeRow {
        username: required_text(row, "UZIVP_USERNAME")?,
        file_number: required_text(row, "CJ")?,
        purpose: required_text(row, "UCEL")?,
        ident: parse_number(required_text(row, "IDENT")?, "IDENT")?,
        started_at: required_text(row, "DAT_SPUST")?,
        account_number: decimal(required_text(row, "ZC_UCTU")?, "ZC_UCTU", Some(0))?,
        function: required_text(row, "FUNKCE")?,
        units: parse_number(required_text(row, "MJ")?, "MJ")?,
        pages: required_text(row, "STRANY")?,
        price: decimal(required_text(row, "CENA")?, "CENA", Some(2))?,
        code: parse_number(required_text(row, "KOD")?, "KOD")?,
        child_report_id: optional_child(row, "ID_PODRIZENE_SESTAVY")?
            .map(|node| {
                let value = text(node, "ID_PODRIZENE_SESTAVY")?;
                LegacyId::new(value).map_err(|_| {
                    Error::invalid_response("invalid account-statement child report ID")
                })
            })
            .transpose()?,
    })
}

fn decimal(value: String, field: &'static str, max_scale: Option<u32>) -> Result<Decimal> {
    let decimal = crate::model::parse_decimal(trim_xml_whitespace(&value), field).map_err(
        |error| match error {
            error @ Error::DecimalOutOfRange { .. } => error,
            _ => Error::invalid_response(format!("{field} is not an exact supported decimal")),
        },
    )?;
    if max_scale.is_some_and(|scale| decimal.normalize().scale() > scale) {
        return Err(Error::invalid_response(format!(
            "{field} exceeds its XSD fractional-digit limit"
        )));
    }
    Ok(decimal)
}

fn parse_number<T>(value: String, name: &'static str) -> Result<T>
where
    T: std::str::FromStr,
{
    trim_xml_whitespace(&value)
        .parse()
        .map_err(|_| Error::invalid_response(format!("{name} has an invalid integer")))
}

fn validate_sequence(node: Node<'_, '_>, fields: &[&str]) -> Result<()> {
    reject_nil(node)?;
    let mut previous = None;
    for child in element_children(node) {
        reject_nil(child)?;
        let index = field_index(child, fields)?;
        if previous.is_some_and(|previous| index <= previous) {
            return Err(Error::invalid_response(format!(
                "element {} violates its XSD sequence or cardinality",
                child.tag_name().name()
            )));
        }
        previous = Some(index);
    }
    Ok(())
}

fn validate_repeated(node: Node<'_, '_>, name: &'static str) -> Result<()> {
    reject_nil(node)?;
    for child in element_children(node) {
        reject_nil(child)?;
        if child.tag_name().namespace().is_some() || child.tag_name().name() != name {
            return Err(unexpected_element(child));
        }
    }
    Ok(())
}

fn field_index(node: Node<'_, '_>, fields: &[&str]) -> Result<usize> {
    if node.tag_name().namespace().is_some() {
        return Err(unexpected_element(node));
    }
    fields
        .iter()
        .position(|name| *name == node.tag_name().name())
        .ok_or_else(|| unexpected_element(node))
}

fn unexpected_element(node: Node<'_, '_>) -> Error {
    Error::invalid_response(format!(
        "unexpected account-statement element {}",
        node.tag_name().name()
    ))
}

fn reject_nil(node: Node<'_, '_>) -> Result<()> {
    if node
        .attribute((XSI_NS, "nil"))
        .is_some_and(|value| matches!(trim_xml_whitespace(value), "true" | "1"))
    {
        Err(Error::invalid_response(format!(
            "account-statement element {} is not nillable",
            node.tag_name().name()
        )))
    } else {
        Ok(())
    }
}

fn trim_xml_whitespace(value: &str) -> &str {
    value.trim_matches(|character| matches!(character, ' ' | '\t' | '\r' | '\n'))
}

fn required_child<'a, 'input>(
    node: Node<'a, 'input>,
    name: &'static str,
) -> Result<Node<'a, 'input>> {
    optional_child(node, name)?
        .ok_or_else(|| Error::invalid_response(format!("required element {name} is missing")))
}

fn optional_child<'a, 'input>(
    node: Node<'a, 'input>,
    name: &'static str,
) -> Result<Option<Node<'a, 'input>>> {
    let mut matches = children(node, name);
    let first = matches.next();
    if matches.next().is_some() {
        return Err(Error::invalid_response(format!(
            "element {name} occurs more than once"
        )));
    }
    Ok(first)
}

fn children<'a, 'input>(
    node: Node<'a, 'input>,
    name: &'static str,
) -> impl Iterator<Item = Node<'a, 'input>> {
    node.children().filter(move |child| {
        child.is_element()
            && child.tag_name().namespace().is_none()
            && child.tag_name().name() == name
    })
}

fn element_children<'a, 'input>(node: Node<'a, 'input>) -> impl Iterator<Item = Node<'a, 'input>> {
    node.children().filter(|child| child.is_element())
}

fn text(node: Node<'_, '_>, name: &'static str) -> Result<String> {
    reject_nil(node)?;
    if node.children().any(|child| child.is_element()) {
        return Err(Error::invalid_response(format!(
            "scalar element {name} contains nested elements"
        )));
    }
    Ok(node
        .children()
        .filter(|child| child.is_text())
        .filter_map(|child| child.text())
        .collect())
}

fn required_text(node: Node<'_, '_>, name: &'static str) -> Result<String> {
    text(required_child(node, name)?, name)
}

fn check_depth(node: Node<'_, '_>, depth: u32, maximum: u32) -> Result<()> {
    if depth > maximum {
        return Err(Error::invalid_response(
            "account statement exceeds XML depth limit",
        ));
    }
    for child in node.children().filter(|child| child.is_element()) {
        check_depth(child, depth.saturating_add(1), maximum)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const PERSON: &str =
        "<titul_pred_jmenem>Ing.</titul_pred_jmenem><prijmeni>Novák</prijmeni><jmeno>Jan</jmeno>";

    fn report(account_number: &str, price: &str) -> String {
        report_with_values(account_number, price, PERSON)
    }

    fn report_with_participant(participant: &str) -> String {
        report_with_values("1", "0", participant)
    }

    fn report_with_values(account_number: &str, price: &str, participant: &str) -> String {
        format!(
            r#"<VypisStavuZakaznickehoUctu><UCET><UCET_USERNAME>user</UCET_USERNAME><CISLO_UCTU>{account_number}</CISLO_UCTU><C_RIZENI>mixed<typriz_kod>V</typriz_kod><poradove_cislo>42</poradove_cislo><rok>2026</rok><prares_kod>306</prares_kod></C_RIZENI><DAT_FAKTURACE>20.11.2026</DAT_FAKTURACE><KREDIT>100,00</KREDIT><UCASTNIK>{participant}</UCASTNIK><SKUPINY><SKUPINA><NAZEV>Reports</NAZEV><RADKY><RADEK><UZIVP_USERNAME>user</UZIVP_USERNAME><CJ>CJ-1</CJ><UCEL>test</UCEL><IDENT>18446744073709551615</IDENT><DAT_SPUST>20.11.2026 10:00</DAT_SPUST><ZC_UCTU>2</ZC_UCTU><FUNKCE>vratSestavu</FUNKCE><MJ>1</MJ><STRANY>2</STRANY><CENA>{price}</CENA><KOD>0</KOD><ID_PODRIZENE_SESTAVY></ID_PODRIZENE_SESTAVY></RADEK></RADKY></SKUPINA></SKUPINY><CENA_CELKEM>{price}</CENA_CELKEM></UCET><DATUM_OD>01.11.2026</DATUM_OD><DATUM_DO>20.11.2026</DATUM_DO><VYHOTOVIL>test</VYHOTOVIL><VYHOTOVENO>20.11.2026 11:00</VYHOTOVENO></VypisStavuZakaznickehoUctu>"#
        )
    }

    fn parses(xml: &str) -> bool {
        parse_account_statement(xml.as_bytes(), 100_000, 64).is_ok()
    }

    #[test]
    fn parses_every_account_field_and_signed_decimal() {
        let parsed =
            parse_account_statement(report("123", "-10.25").as_bytes(), 100_000, 64).unwrap();
        assert_eq!(parsed.account.number, Decimal::new(123, 0));
        assert_eq!(parsed.account.proceeding.text, "mixed");
        assert_eq!(parsed.account.groups[0].rows[0].ident, u64::MAX);
        assert_eq!(
            parsed.account.groups[0].rows[0].price,
            Decimal::new(-1025, 2)
        );
        assert_eq!(
            parsed.account.groups[0].rows[0]
                .child_report_id
                .as_ref()
                .map(LegacyId::as_str),
            Some("")
        );
        assert_eq!(parsed.account.credit, "100,00");
    }

    #[test]
    fn participant_choice_covers_all_person_fields_without_ignoring_extras() {
        let person_fields = [
            "<titul_pred_jmenem>Ing.</titul_pred_jmenem>",
            "<prijmeni>Novák</prijmeni>",
            "<jmeno>Jan</jmeno>",
            "<titul_za_jmenem>Ph.D.</titul_za_jmenem>",
        ];
        for field in person_fields {
            let mixed = report_with_participant(&format!(
                "{field}<obchodni_jmeno>Organization</obchodni_jmeno>"
            ));
            assert!(!parses(&mixed), "person field beside organization: {field}");
        }

        for incomplete in [
            "<titul_pred_jmenem>Ing.</titul_pred_jmenem>",
            "<titul_za_jmenem>Ph.D.</titul_za_jmenem>",
            "<jmeno>Jan</jmeno>",
            "<prijmeni>Novák</prijmeni>",
            "<titul_pred_jmenem>Ing.</titul_pred_jmenem><jmeno>Jan</jmeno>",
            "<prijmeni>Novák</prijmeni><titul_za_jmenem>Ph.D.</titul_za_jmenem>",
        ] {
            assert!(
                !parses(&report_with_participant(incomplete)),
                "incomplete person was accepted: {incomplete}"
            );
        }

        let organization = parse_account_statement(
            report_with_participant("<obchodni_jmeno>Organization</obchodni_jmeno>").as_bytes(),
            100_000,
            64,
        )
        .unwrap();
        assert!(matches!(
            organization.account.participant,
            AccountParticipant::Organization(name) if name == "Organization"
        ));

        let full_person = parse_account_statement(
            report_with_participant(
                "<titul_pred_jmenem>Ing.</titul_pred_jmenem><prijmeni>Novák</prijmeni><jmeno>Jan</jmeno><titul_za_jmenem>Ph.D.</titul_za_jmenem>",
            )
            .as_bytes(),
            100_000,
            64,
        )
        .unwrap();
        assert!(matches!(
            full_person.account.participant,
            AccountParticipant::Person {
                title_before: Some(before),
                family_name,
                given_name,
                title_after: Some(after),
            } if before == "Ing." && family_name == "Novák" && given_name == "Jan" && after == "Ph.D."
        ));
        assert!(!parses(&report_with_participant(&format!(
            "{PERSON}<unexpected>must not be ignored</unexpected>"
        ))));
        assert!(!parses(&report_with_participant("")));
    }

    #[test]
    fn rejects_unexpected_and_namespaced_children_in_every_container() {
        let valid = report("1", "0");
        let invalid = vec![
            valid.replace(
                "</VypisStavuZakaznickehoUctu>",
                "<unexpected/></VypisStavuZakaznickehoUctu>",
            ),
            valid.replace("</UCET>", "<unexpected/></UCET>"),
            valid.replace("</C_RIZENI>", "<unexpected/></C_RIZENI>"),
            valid.replace("<SKUPINY>", "<SKUPINY><unexpected/>"),
            valid.replace(
                "<SKUPINY>",
                "<SKUPINY><x:SKUPINA xmlns:x=\"urn:unexpected\"/>",
            ),
            valid.replace("</SKUPINA>", "<unexpected/></SKUPINA>"),
            valid.replace("</RADKY>", "<unexpected/></RADKY>"),
            valid.replace(
                "</RADKY>",
                "<x:RADEK xmlns:x=\"urn:unexpected\"/></RADKY>",
            ),
            valid.replace("</RADEK>", "<unexpected/></RADEK>"),
            valid.replace(
                "</VypisStavuZakaznickehoUctu>",
                "<x:DATUM_DO xmlns:x=\"urn:unexpected\">ignored</x:DATUM_DO></VypisStavuZakaznickehoUctu>",
            ),
        ];
        for xml in invalid {
            assert!(
                !parses(&xml),
                "unexpected or namespace-qualified child was accepted"
            );
        }
    }

    #[test]
    fn enforces_sequences_and_singleton_cardinality() {
        let valid = report("1", "0");
        let invalid = vec![
            valid.replace(
                "<DATUM_OD>01.11.2026</DATUM_OD><DATUM_DO>20.11.2026</DATUM_DO>",
                "<DATUM_DO>20.11.2026</DATUM_DO><DATUM_OD>01.11.2026</DATUM_OD>",
            ),
            valid.replace(
                "<UCET_USERNAME>user</UCET_USERNAME><CISLO_UCTU>1</CISLO_UCTU>",
                "<CISLO_UCTU>1</CISLO_UCTU><UCET_USERNAME>user</UCET_USERNAME>",
            ),
            valid.replace(
                "<typriz_kod>V</typriz_kod><poradove_cislo>42</poradove_cislo>",
                "<poradove_cislo>42</poradove_cislo><typriz_kod>V</typriz_kod>",
            ),
            valid.replace(
                "<UZIVP_USERNAME>user</UZIVP_USERNAME><CJ>CJ-1</CJ>",
                "<CJ>CJ-1</CJ><UZIVP_USERNAME>user</UZIVP_USERNAME>",
            ),
            valid.replace(
                "<VYHOTOVIL>test</VYHOTOVIL>",
                "<VYHOTOVIL>test</VYHOTOVIL><VYHOTOVIL>duplicate</VYHOTOVIL>",
            ),
            valid.replace("<KOD>0</KOD>", "<KOD>0</KOD><KOD>0</KOD>"),
            report_with_participant("<jmeno>Jan</jmeno><prijmeni>Novák</prijmeni>"),
        ];
        for xml in invalid {
            assert!(!parses(&xml), "invalid XSD sequence was accepted");
        }

        let group_out_of_order = valid
            .replace("<SKUPINA><NAZEV>Reports</NAZEV><RADKY>", "<SKUPINA><RADKY>")
            .replace(
                "</RADKY></SKUPINA>",
                "</RADKY><NAZEV>Reports</NAZEV></SKUPINA>",
            );
        assert!(!parses(&group_out_of_order));
    }

    #[test]
    fn rejects_xsi_nil_on_nonnillable_complex_and_scalar_elements() {
        let valid = report("1", "0").replace(
            "<VypisStavuZakaznickehoUctu>",
            "<VypisStavuZakaznickehoUctu xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\">",
        );
        let invalid = [
            valid.replace(
                "<VypisStavuZakaznickehoUctu xmlns:xsi=",
                "<VypisStavuZakaznickehoUctu xsi:nil=\"true\" xmlns:xsi=",
            ),
            valid.replace("<UCET>", "<UCET xsi:nil=\"true\">"),
            valid.replace("<RADEK>", "<RADEK xsi:nil=\"true\">"),
            valid.replace("<CENA>0</CENA>", "<CENA xsi:nil=\"true\">0</CENA>"),
            valid.replace(
                "<CISLO_UCTU>1</CISLO_UCTU>",
                "<CISLO_UCTU xsi:nil=\" 1 \">1</CISLO_UCTU>",
            ),
        ];
        for xml in invalid {
            assert!(!parses(&xml), "xsi:nil on nonnillable element was accepted");
        }
    }

    #[test]
    fn decimal_fraction_digits_are_checked_after_normalization() {
        let parsed =
            parse_account_statement(report(" \t1.0\r\n", " 1.000 ").as_bytes(), 100_000, 64)
                .unwrap();
        assert_eq!(parsed.account.number, Decimal::new(1, 0));
        assert_eq!(parsed.account.groups[0].rows[0].price, Decimal::new(1, 0));

        assert!(!parses(&report("1.2", "0")));
        assert!(!parses(&report("1", "1.001")));
        assert!(parses(&report("1", "1.2300")));
    }

    #[test]
    fn numeric_values_trim_only_xml_whitespace() {
        let xml_whitespace = report("1", "0")
            .replace(
                "<poradove_cislo>42</poradove_cislo>",
                "<poradove_cislo> \t42\r\n</poradove_cislo>",
            )
            .replace("<ZC_UCTU>2</ZC_UCTU>", "<ZC_UCTU>\n2.0\t</ZC_UCTU>");
        assert!(parses(&xml_whitespace));

        assert!(!parses(&report("\u{00a0}1\u{00a0}", "0")));
        assert!(!parses(&report("1", "\u{00a0}1.00\u{00a0}")));
        let unicode_integer = report("1", "0").replace(
            "<poradove_cislo>42</poradove_cislo>",
            "<poradove_cislo>\u{00a0}42\u{00a0}</poradove_cislo>",
        );
        assert!(!parses(&unicode_integer));
    }

    #[test]
    fn decimal_range_error_does_not_echo_the_value() {
        let raw = "999999999999999999999999999999";
        let error =
            parse_account_statement(report(raw, "0.00").as_bytes(), 100_000, 64).unwrap_err();
        assert!(matches!(error, Error::DecimalOutOfRange { .. }));
        assert!(!error.to_string().contains(raw));
    }

    #[test]
    fn rejects_ambiguous_participant_and_dtd() {
        let ambiguous = report("1", "0").replace(
            "<UCASTNIK>",
            "<UCASTNIK><obchodni_jmeno>Organization</obchodni_jmeno>",
        );
        assert!(parse_account_statement(ambiguous.as_bytes(), 100_000, 64).is_err());
        let dtd = format!(
            "<!DOCTYPE x [<!ENTITY e SYSTEM 'file:///etc/passwd'>]>{}",
            report("1", "0")
        );
        assert!(parse_account_statement(dtd.as_bytes(), 100_000, 64).is_err());
    }
}
