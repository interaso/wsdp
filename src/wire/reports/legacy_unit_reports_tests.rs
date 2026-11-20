use rust_decimal::Decimal;

use super::*;

const MAX_NODES: u32 = 100_000;
const MAX_DEPTH: u32 = 128;
const FLOOR_AREA: &str = "<PODLAHOVA_PLOCHA>12345678.90</PODLAHOVA_PLOCHA>";

fn unit_document(floor_area: &str) -> String {
    format!(
        r#"<InformaceOJednotkach><KATASTR> cadastral mixed text <OKRES><nuts4>CZ0100</nuts4><nazev> District </nazev></OKRES><OBEC><kod>4294967295</kod><nazev>Municipality</nazev></OBEC><KATUZE><kod>727873</kod><nazev>Cadastral area</nazev></KATUZE><STAVBY><STAVBA><TYPBUD_KOD>255</TYPBUD_KOD><CISLO_DOMOVNI> 1161 </CISLO_DOMOVNI><ZPVYBU_KOD> raw-code </ZPVYBU_KOD><BUDOVA><stavba><id>1021111101</id><caobce>Part</caobce><typbud_zkr>č.p.</typbud_zkr><cisla_domovni>1160, 1161</cisla_domovni><vyuziti_zkr>use</vyuziti_zkr><tel_id>1973192101</tel_id><bez_lv>raw</bez_lv><cislo_tel>2269</cislo_tel><ku_nazev>Area</ku_nazev><docasna>n</docasna><par_ident><parcela><id>1</id></parcela><parcela/></par_ident></stavba></BUDOVA><BUD_PARCELY><PARCELA><parcela><id>2224809101</id><zkratka/><druh_cis>raw</druh_cis><par_cis> 2612</par_cis><podd_cis>47  </podd_cis><dil_parcely>-128</dil_parcely><tel_id>1973192101</tel_id><cislo_tel>2269</cislo_tel><ku_nazev>Area</ku_nazev><nazev_puv>Old area</nazev_puv></parcela><tel_id>1973192101</tel_id><cislo_tel>2269</cislo_tel></PARCELA><PARCELA/></BUD_PARCELY><JEDNOTKY><JEDNOTKA><PLOMBY><PLOMBA>seal text </PLOMBA><PLOMBA><PLOMBA_RIZENI/><PLOMBA_RIZENI><typriz_kod> raw-Z </typriz_kod><poradove_cislo>4294967295</poradove_cislo><rok>65535</rok><prares_kod>65535</prares_kod></PLOMBA_RIZENI></PLOMBA></PLOMBY><VAZBA_JPV><TEXT> relation text </TEXT></VAZBA_JPV><VLASTNICTVI><LV><tel_id>2261492101</tel_id><cislo_tel>4054</cislo_tel></LV><SUBJEKTY><SUBJEKT><VLA_IDENT><oprav_subjekt><id>627994601</id><nazev>Subject</nazev><adresa><obec>Municipality</obec></adresa></oprav_subjekt><parcela/><stavba/><jednotka><id/></jednotka><pravo_stavby/></VLA_IDENT><PODIL><citatel>-1</citatel><jmenovatel>0</jmenovatel></PODIL><TYPRAV_NAZEV> raw relation </TYPRAV_NAZEV><CHAROS><char_os><kod>99</kod><zkratka>z</zkratka><zkratka_alv>alv</zkratka_alv></char_os></CHAROS><G_NAZ_VLA_PS><VLA_IDENT2><oprav_subjekt/></VLA_IDENT2></G_NAZ_VLA_PS></SUBJEKT><SUBJEKT><VLA_IDENT/><PODIL/><TYPRAV_NAZEV/><CHAROS/></SUBJEKT></SUBJEKTY></VLASTNICTVI><PARCELY><PARCELA><PARCELNI_CISLO><parcela><id>2224810101</id></parcela></PARCELNI_CISLO><PAR_TEL><tel_id>1973192101</tel_id><cislo_tel>4294967295</cislo_tel></PAR_TEL><PAR_TYP_VLA>raw ownership</PAR_TYP_VLA><PAR_PLOMBY_RIZENI><PAR_PLOMBA_RIZENI>parcel seal text </PAR_PLOMBA_RIZENI><PAR_PLOMBA_RIZENI><typriz_kod>Z</typriz_kod><poradove_cislo>40538</poradove_cislo><rok>2017</rok><prares_kod>101</prares_kod></PAR_PLOMBA_RIZENI></PAR_PLOMBY_RIZENI></PARCELA><PARCELA><PARCELNI_CISLO/><PAR_TEL><tel_id/><cislo_tel>0</cislo_tel></PAR_TEL><PAR_TYP_VLA/><PAR_PLOMBY_RIZENI/></PARCELA></PARCELY><RIZENI><typriz_kod>Z</typriz_kod><poradove_cislo>40538</poradove_cislo><rok>2017</rok><prares_kod>101</prares_kod></RIZENI><BUD_PODIL><citatel>344</citatel><jmenovatel>35966</jmenovatel></BUD_PODIL><VYUZ_TYP>byt byt.z.</VYUZ_TYP><CISLO_JED><jednotka><id>1005719101</id><caobce/><cp_ident>1161</cp_ident><jed_ident>10</jed_ident><tel_id>2261492101</tel_id><cislo_tel>4054</cislo_tel><ku_nazev>Area</ku_nazev></jednotka></CISLO_JED><ZP_VYUZITI> raw-use </ZP_VYUZITI><POCET_JEDNOTEK>4294967295</POCET_JEDNOTEK>{floor_area}</JEDNOTKA></JEDNOTKY></STAVBA></STAVBY></KATASTR><CS_POCET_JEDNOTEK>4294967295</CS_POCET_JEDNOTEK><PLATNOST>14.06.2022 11:35</PLATNOST><VYHOTOVIL>VPUSER</VYHOTOVIL><BEZPL_PRISTUP> raw-free </BEZPL_PRISTUP><VYHOTOVENO>14.06.2022 11:46</VYHOTOVENO><INFO_VYSTUP> output </INFO_VYSTUP></InformaceOJednotkach>"#
    )
}

fn building_right_document() -> String {
    r#"<InformaceOPravechStavby><VYHOTOVENO>14.06.2022 14:28</VYHOTOVENO><INFO_VYSTUP> output </INFO_VYSTUP><ZAHLAVI><OKRES><nuts4>CZ0100</nuts4><nazev>District</nazev></OKRES><OBEC><kod> 00554782 </kod><nazev>Praha</nazev></OBEC><KU><kod>raw-ku</kod><nazev>Karlín</nazev></KU><PRAVA_STAVEB><ZAZNAM><PLATNOST_DO>13.01.2115 00:00:00</PLATNOST_DO><ZP_OCHRANY><zpochr> raw-protection </zpochr><zpochr>other</zpochr></ZP_OCHRANY><IDENT_RIZENI><typriz_kod> raw-Z </typriz_kod><poradove_cislo>001900</poradove_cislo><rok>02020</rok><prares_kod>raw-workplace</prares_kod></IDENT_RIZENI><UCELY_PRAVA_STAVBY><nazev>purpose one</nazev><nazev> purpose two </nazev></UCELY_PRAVA_STAVBY><PS_IDENT><pravo_stavby><id>15836</id><tel_id>1920963101</tel_id><parcela><id>51498797010</id></parcela><parcela/><cislo_tel>116</cislo_tel><ku_nazev>Karlín</ku_nazev></pravo_stavby></PS_IDENT><SOUCASTI_STAVBA><stavba><id>1</id></stavba><docasna> raw-temporary </docasna><stavba/></SOUCASTI_STAVBA><LISTY_VLASTNICTVI><LIST_VLASTNICTVI><LV><tel_id/><cislo_tel>00116</cislo_tel></LV><VLASTNICTVI><VLA_IDENT><oprav_subjekt><id>660299507</id></oprav_subjekt></VLA_IDENT><PODIL><citatel>1</citatel><jmenovatel>2</jmenovatel></PODIL><CHAROS><char_os><kod>20</kod><zkratka/><zkratka_alv/></char_os></CHAROS><TYP_VZTAHU> raw-relationship </TYP_VZTAHU><VLA_IDENT2/><VLA_IDENT2><parcela/></VLA_IDENT2></VLASTNICTVI><VLASTNICTVI><VLA_IDENT/><PODIL/><CHAROS/><TYP_VZTAHU/><VLA_IDENT2/></VLASTNICTVI></LIST_VLASTNICTVI></LISTY_VLASTNICTVI></ZAZNAM></PRAVA_STAVEB></ZAHLAVI><VYHOTOVIL>VPUSER</VYHOTOVIL><PLATNOST_K>14.06.2022 14:15</PLATNOST_K><BEZPL_PRISTUP> raw-free </BEZPL_PRISTUP></InformaceOPravechStavby>"#.to_owned()
}

fn minimal_proceeding_document() -> String {
    r#"<InformaceORizeni><INFO_VYSTUP/><VYHOTOVENO>created-report</VYHOTOVENO><ZAVADY_GP/><UCASTNICI/><RIZENI><PRIRAZENO>assigned</PRIRAZENO><DATUM_PODANI>submitted</DATUM_PODANI><OSVOBOZENO>raw-exempt</OSVOBOZENO><CISLO_RIZENI/><PRAC_NAZEV>workplace</PRAC_NAZEV><TYPRIZ_NAZEV>type</TYPRIZ_NAZEV><STAV>raw-state</STAV><VYTVORENO>created</VYTVORENO><STAV_UHRADY/></RIZENI><PREDMETY_RIZENI/><KATASTRY/><OBJEKTY_PARCELY/><OBJEKTY_STAVBY/><LISTINY/><OBJEKTY_JED/><UDALOSTI/><VYK_OPERACE/><NAVRHY_ZMEN/><OBJEKTY_PRAVA_STAVBY/><VYHOTOVIL>creator</VYHOTOVIL><BEZPL_PRISTUP>free</BEZPL_PRISTUP></InformaceORizeni>"#.to_owned()
}

fn proceeding_document() -> String {
    minimal_proceeding_document()
        .replace(
            "<UCASTNICI/>",
            r#"<UCASTNICI><UCASTNIK><CELE_JMENO><prijmeni>Surname</prijmeni><jmeno>Given</jmeno><titul_pred_jmenem>Ing.</titul_pred_jmenem><titul_za_jmenem>Ph.D.</titul_za_jmenem></CELE_JMENO><IDENTIFIKACE><id/><rc6> raw-rc6 </rc6><rc7>rc7</rc7><ico>00123raw</ico><ico_doplnek>raw-suffix</ico_doplnek><nazev>raw-name</nazev><id>999999999999999999999999999999999999</id></IDENTIFIKACE><UCAST_ADRESA><adresa><adresni_misto/><nazev_ulice>Street</nazev_ulice><cp_ce>future</cp_ce><cislo_domovni>1</cislo_domovni><cislo_orientacni>2</cislo_orientacni><cast_obce>Part</cast_obce><obec>Municipality</obec><mestska_cast>District</mestska_cast><mestsky_obvod>Borough</mestsky_obvod><psc>raw-postal</psc><dod_posta>Post</dod_posta><stat>Country</stat></adresa></UCAST_ADRESA><UCAST_ADRESA/><TYP_UCAST><TYPUCA_KOD> raw-participation </TYPUCA_KOD><TYPUCA_NAZEV>Participant</TYPUCA_NAZEV></TYP_UCAST><TYP_UCAST/></UCASTNIK><UCASTNIK><CELE_JMENO><obchodni_jmeno> Organization </obchodni_jmeno></CELE_JMENO><IDENTIFIKACE/><UCAST_ADRESA/><TYP_UCAST/></UCASTNIK><UCASTNIK><CELE_JMENO/><IDENTIFIKACE/><UCAST_ADRESA/><TYP_UCAST/></UCASTNIK></UCASTNICI>"#,
        )
        .replace(
            "<RIZENI><PRIRAZENO>assigned</PRIRAZENO><DATUM_PODANI>submitted</DATUM_PODANI><OSVOBOZENO>raw-exempt</OSVOBOZENO><CISLO_RIZENI/><PRAC_NAZEV>workplace</PRAC_NAZEV><TYPRIZ_NAZEV>type</TYPRIZ_NAZEV><STAV>raw-state</STAV><VYTVORENO>created</VYTVORENO><STAV_UHRADY/></RIZENI>",
            r#"<RIZENI><PRIRAZENO> assigned </PRIRAZENO><DATUM_PODANI>submitted</DATUM_PODANI><OSVOBOZENO> raw-exempt </OSVOBOZENO><CISLO_RIZENI><typriz_kod> raw-Z </typriz_kod><poradove_cislo>4294967295</poradove_cislo><rok>65535</rok><prares_kod>65535</prares_kod></CISLO_RIZENI><PRAC_NAZEV>workplace</PRAC_NAZEV><TYPRIZ_NAZEV>type</TYPRIZ_NAZEV><STAV> raw-state </STAV><VYTVORENO>created</VYTVORENO><STAV_UHRADY><kod> raw-payment </kod><nazev>Paid</nazev></STAV_UHRADY></RIZENI>"#,
        )
        .replace(
            "<PREDMETY_RIZENI/>",
            r#"<PREDMETY_RIZENI><PREDMET><TYPPRE_KOD> raw-subject </TYPPRE_KOD><TYPPRE_NAZEV>Subject</TYPPRE_NAZEV></PREDMET><PREDMET><TYPPRE_KOD/><TYPPRE_NAZEV/></PREDMET></PREDMETY_RIZENI>"#,
        )
        .replace(
            "<KATASTRY/>",
            r#"<KATASTRY><KATASTR><KATUZE/><KATUZE><kod>4294967295</kod><nazev>Area</nazev></KATUZE></KATASTR><KATASTR/></KATASTRY>"#,
        )
        .replace(
            "<OBJEKTY_PARCELY/>",
            r#"<OBJEKTY_PARCELY><OBJEKT_PARCELA><PARCELA><parcela><id>1</id></parcela></PARCELA><TYP> raw-parcel-kind </TYP><DIL_PARCELY>raw-part</DIL_PARCELY><ZDROJ_ZE>raw-source</ZDROJ_ZE><PUV_KATUZE>Old area</PUV_KATUZE></OBJEKT_PARCELA><OBJEKT_PARCELA><PARCELA/><TYP/><DIL_PARCELY/><ZDROJ_ZE/><PUV_KATUZE/></OBJEKT_PARCELA></OBJEKTY_PARCELY>"#,
        )
        .replace(
            "<OBJEKTY_STAVBY/>",
            r#"<OBJEKTY_STAVBY><OBJEKT_STAVBA><STAVBA><stavba><id>2</id></stavba></STAVBA><TBUD_ZKR>raw-building-type</TBUD_ZKR></OBJEKT_STAVBA><OBJEKT_STAVBA><STAVBA/><TBUD_ZKR/></OBJEKT_STAVBA></OBJEKTY_STAVBY>"#,
        )
        .replace(
            "<LISTINY/>",
            r#"<LISTINY><LISTINA><ID/><DATUM_VYHOTOVENI>prepared</DATUM_VYHOTOVENI><DATUM_PRAV_MOCI>force</DATUM_PRAV_MOCI><DALSI_UDAJE>additional</DALSI_UDAJE><TYPLIS_NAZEV>deed type</TYPLIS_NAZEV><TYPLIST_KOD>65535</TYPLIST_KOD><LISTIN_POPIS>description</LISTIN_POPIS><DATUM_HIST_OD>from</DATUM_HIST_OD><DATUM_HIST_DO>to</DATUM_HIST_DO><DALSI_UDAJE_DATUM_OD>additional-from</DALSI_UDAJE_DATUM_OD><ZHOTOVITEL>preparer</ZHOTOVITEL></LISTINA><LISTINA/></LISTINY>"#,
        )
        .replace(
            "<OBJEKTY_JED/>",
            r#"<OBJEKTY_JED><OBJEKT_JED><C_JED><jednotka><id>3</id></jednotka></C_JED><ZP_VYUZ_JED> raw-unit-use </ZP_VYUZ_JED><BUDOVA><stavba><id>2</id></stavba></BUDOVA></OBJEKT_JED><OBJEKT_JED><C_JED/><ZP_VYUZ_JED/><BUDOVA/></OBJEKT_JED></OBJEKTY_JED>"#,
        )
        .replace(
            "<UDALOSTI/>",
            r#"<GEOM_PLAN><NZ_TYPE> raw-plan-kind </NZ_TYPE><CISLO_PLANU>raw-plan-number</CISLO_PLANU><UCEL><nazev>purpose one</nazev><nazev> purpose two </nazev></UCEL></GEOM_PLAN><UDALOSTI><UDALOST><TYPOPE_KOD> raw-event </TYPOPE_KOD><TYPOPE_NAZEV>Event</TYPOPE_NAZEV><TEXT> event text </TEXT><VYTVORENO>event-created</VYTVORENO></UDALOST></UDALOSTI>"#,
        )
        .replace(
            "<VYK_OPERACE/>",
            r#"<VYK_OPERACE><OPERACE><TYPOPE_KOD> raw-operation </TYPOPE_KOD><TYPOPE_NAZEV>Operation</TYPOPE_NAZEV><TEXT custom="raw">before<extra xmlns="urn:extra">nested &amp; retained</extra>after</TEXT><VYTVORENO>operation-created</VYTVORENO><DATUM_UKONCENI>operation-completed</DATUM_UKONCENI></OPERACE></VYK_OPERACE>"#,
        )
        .replace(
            "<ZAVADY_GP/>",
            r#"<ZAVADY_GP><ZAVADA_GP><NAZEV>Defect</NAZEV><UPRESNENI_ZAVADY>Specification</UPRESNENI_ZAVADY></ZAVADA_GP></ZAVADY_GP>"#,
        )
        .replace(
            "<NAVRHY_ZMEN/>",
            r#"<NAVRHY_ZMEN><NAVRH_ZMEN><VYTVORIL>proposal-creator</VYTVORIL><CISLO_RIZENI/><STAV> raw-proposal-state </STAV><DATUM>proposal-date</DATUM></NAVRH_ZMEN></NAVRHY_ZMEN>"#,
        )
        .replace(
            "<OBJEKTY_PRAVA_STAVBY/>",
            r#"<OBJEKTY_PRAVA_STAVBY><OBJEKTY_PRAVO_STAVBY><PS_PARCELA><pravo_stavby><id>4</id></pravo_stavby></PS_PARCELA><PS_UCEL><nazev> right purpose </nazev></PS_UCEL><PS_PLATNOST_DO>right-valid-until</PS_PLATNOST_DO></OBJEKTY_PRAVO_STAVBY></OBJEKTY_PRAVA_STAVBY>"#,
        )
}

fn replace_element(xml: &str, name: &str, replacement: &str) -> String {
    let empty_marker = format!("<{name}/>");
    if let Some(start) = xml.find(&empty_marker) {
        let mut result = xml.to_owned();
        result.replace_range(start..start + empty_marker.len(), replacement);
        return result;
    }
    let start_marker = format!("<{name}>");
    let end_marker = format!("</{name}>");
    let start = xml.find(&start_marker).unwrap();
    let end = xml[start..].find(&end_marker).unwrap() + start + end_marker.len();
    let mut result = xml.to_owned();
    result.replace_range(start..end, replacement);
    result
}

fn remove_element(xml: &str, name: &str) -> String {
    replace_element(xml, name, "")
}

fn parse_unit(xml: &str) -> Result<UnitInfoReport> {
    parse_unit_info(xml.as_bytes(), MAX_NODES, MAX_DEPTH)
}

fn parse_building_right(xml: &str) -> Result<BuildingRightInfoReport> {
    parse_building_right_info(xml.as_bytes(), MAX_NODES, MAX_DEPTH)
}

fn parse_proceeding(xml: &str) -> Result<ProceedingInfoReport> {
    parse_proceeding_info(xml.as_bytes(), MAX_NODES, MAX_DEPTH)
}

#[test]
fn unit_report_covers_every_section_choice_and_repetition() {
    let parsed = parse_unit(&unit_document(FLOOR_AREA)).unwrap();
    assert_eq!(parsed.total_units, u32::MAX);
    assert_eq!(parsed.output_info.as_deref(), Some(" output "));
    assert_eq!(parsed.free_access, " raw-free ");
    let cadastral = &parsed.cadastral_area;
    assert_eq!(cadastral.text, " cadastral mixed text ");
    assert_eq!(cadastral.district.nuts4.as_deref(), Some("CZ0100"));
    assert_eq!(cadastral.district.name.as_deref(), Some(" District "));
    assert_eq!(cadastral.municipality.code, u32::MAX);
    assert_eq!(cadastral.cadastral_area.code, 727_873);
    let building = &cadastral.buildings[0];
    assert_eq!(building.building_type_code, u8::MAX);
    assert_eq!(building.house_number, " 1161 ");
    assert_eq!(building.building_use_code, " raw-code ");
    assert_eq!(
        building
            .building
            .as_ref()
            .unwrap()
            .identifying_parcels
            .as_ref()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(building.parcels.len(), 2);
    assert_eq!(
        building.parcels[0].parcel.as_ref().unwrap().part,
        Some(-128)
    );
    assert!(building.parcels[1].parcel.is_none());

    let unit = &building.units[0];
    assert_eq!(unit.unit_count, Some(u32::MAX));
    assert_eq!(unit.use_code.as_deref(), Some(" raw-use "));
    assert_eq!(unit.unit.as_ref().unwrap().id.as_str(), "1005719101");
    assert_eq!(unit.unit_type, "byt byt.z.");
    assert_eq!(unit.building_share.numerator, Some(Decimal::new(344, 0)));
    assert_eq!(
        unit.floor_area.unwrap().value.unwrap().as_decimal(),
        &Decimal::new(1_234_567_890, 2).normalize()
    );
    assert_eq!(
        unit.legal_relation_link.as_ref().unwrap().text.as_deref(),
        Some(" relation text ")
    );
    let ownership = unit.ownership.details.as_ref().unwrap();
    assert_eq!(ownership.ownership_sheet.as_ref().unwrap().number, "4054");
    assert_eq!(ownership.subjects.len(), 2);
    assert_eq!(ownership.subjects[0].identification.targets.len(), 5);
    assert_eq!(
        ownership.subjects[0].share.numerator,
        Some(Decimal::NEGATIVE_ONE)
    );
    assert_eq!(ownership.subjects[0].share.denominator, Some(Decimal::ZERO));
    assert_eq!(ownership.subjects[0].legal_relation_name, " raw relation ");
    assert!(ownership.subjects[0].alternate_identification.is_some());
    assert!(ownership.subjects[1].alternate_identification.is_none());
    assert_eq!(unit.parcels.len(), 2);
    assert_eq!(unit.parcels[0].ownership_sheet.number, u32::MAX);
    assert_eq!(unit.parcels[0].seal_proceedings.len(), 2);
    assert_eq!(
        unit.parcels[0].seal_proceedings[0].text,
        "parcel seal text "
    );
    assert!(unit.parcels[0].seal_proceedings[0].identification.is_none());
    assert_eq!(
        unit.parcels[0].seal_proceedings[1]
            .identification
            .as_ref()
            .unwrap()
            .serial_number,
        40_538
    );
    assert_eq!(unit.parcels[1].ownership_sheet.id.as_str(), "");
    assert_eq!(unit.seals.len(), 2);
    assert_eq!(unit.seals[0].text, "seal text ");
    assert!(unit.seals[0].proceedings.is_empty());
    assert_eq!(unit.seals[1].proceedings.len(), 2);
    let proceeding = unit.seals[1].proceedings[1]
        .identification
        .as_ref()
        .unwrap();
    assert_eq!(proceeding.type_code, " raw-Z ");
    assert_eq!(proceeding.serial_number, u32::MAX);
    assert_eq!(proceeding.year, u16::MAX);
    assert_eq!(proceeding.workplace_code, u16::MAX);
}

#[test]
fn unit_floor_area_has_only_the_documented_missing_field_exception() {
    let omitted = parse_unit(&unit_document("")).unwrap();
    assert!(
        omitted.cadastral_area.buildings[0].units[0]
            .floor_area
            .is_none()
    );

    let empty = parse_unit(&unit_document("<PODLAHOVA_PLOCHA/>")).unwrap();
    assert!(
        empty.cadastral_area.buildings[0].units[0]
            .floor_area
            .unwrap()
            .value
            .is_none()
    );

    for required in ["VYUZ_TYP", "BUD_PODIL", "RIZENI", "VLASTNICTVI", "PARCELY"] {
        assert!(
            parse_unit(&remove_element(&unit_document(FLOOR_AREA), required)).is_err(),
            "missing required {required} was accepted"
        );
    }
    for whitespace in [" ", "\n"] {
        assert!(
            parse_unit(&unit_document(&format!(
                "<PODLAHOVA_PLOCHA>{whitespace}</PODLAHOVA_PLOCHA>"
            )))
            .is_err(),
            "whitespace is not the union's empty-string value"
        );
    }
}

#[test]
fn unit_report_enforces_required_nonempty_lists_and_singletons() {
    for wrapper in ["STAVBY", "BUD_PARCELY", "JEDNOTKY", "SUBJEKTY"] {
        let xml = replace_element(
            &unit_document(FLOOR_AREA),
            wrapper,
            &format!("<{wrapper}/>"),
        );
        assert!(parse_unit(&xml).is_err(), "empty {wrapper} was accepted");
    }

    let duplicate = unit_document(FLOOR_AREA).replace(
        "<VYUZ_TYP>byt byt.z.</VYUZ_TYP>",
        "<VYUZ_TYP>one</VYUZ_TYP><VYUZ_TYP>two</VYUZ_TYP>",
    );
    assert!(parse_unit(&duplicate).is_err());

    let empty_ownership =
        replace_element(&unit_document(FLOOR_AREA), "VLASTNICTVI", "<VLASTNICTVI/>");
    assert!(
        parse_unit(&empty_ownership)
            .unwrap()
            .cadastral_area
            .buildings[0]
            .units[0]
            .ownership
            .details
            .is_none()
    );
    let empty_lists = replace_element(
        &replace_element(&unit_document(FLOOR_AREA), "PARCELY", "<PARCELY/>"),
        "PLOMBY",
        "<PLOMBY/>",
    );
    let parsed = parse_unit(&empty_lists).unwrap();
    assert!(
        parsed.cadastral_area.buildings[0].units[0]
            .parcels
            .is_empty()
    );
    assert!(parsed.cadastral_area.buildings[0].units[0].seals.is_empty());
}

#[test]
fn unit_decimals_preserve_range_errors_and_reject_exponents() {
    let exponent = unit_document("<PODLAHOVA_PLOCHA>1e2</PODLAHOVA_PLOCHA>");
    assert!(matches!(
        parse_unit(&exponent),
        Err(Error::InvalidResponse { .. })
    ));

    let too_large = "999999999999999999999999999999";
    let floor = unit_document(&format!("<PODLAHOVA_PLOCHA>{too_large}</PODLAHOVA_PLOCHA>"));
    let error = parse_unit(&floor).unwrap_err();
    assert!(matches!(
        error,
        Error::DecimalOutOfRange {
            field: "Number102Type"
        }
    ));
    assert!(!error.to_string().contains(too_large));

    let share = unit_document(FLOOR_AREA).replace(
        "<citatel>-1</citatel>",
        &format!("<citatel>{too_large}</citatel>"),
    );
    assert!(matches!(
        parse_unit(&share),
        Err(Error::DecimalOutOfRange {
            field: "PodilType/citatel"
        })
    ));
    assert!(
        parse_unit(&unit_document(FLOOR_AREA).replace(
            "<TYPBUD_KOD>255</TYPBUD_KOD>",
            "<TYPBUD_KOD>256</TYPBUD_KOD>"
        ))
        .is_err()
    );
}

#[test]
fn building_right_report_covers_all_fields_wrappers_and_repetition() {
    let parsed = parse_building_right(&building_right_document()).unwrap();
    assert_eq!(parsed.created_at, "14.06.2022 14:28");
    assert_eq!(parsed.output_info.as_deref(), Some(" output "));
    assert_eq!(parsed.free_access, " raw-free ");
    assert_eq!(parsed.valid_at.as_deref(), Some("14.06.2022 14:15"));
    let header = parsed.header;
    assert_eq!(header.district.unwrap().nuts4.as_deref(), Some("CZ0100"));
    assert_eq!(header.municipality.unwrap().code, " 00554782 ".to_owned());
    assert_eq!(header.cadastral_area.unwrap().code, "raw-ku");
    let record = &header.records.unwrap()[0];
    assert_eq!(record.protection_methods.len(), 2);
    assert_eq!(record.protection_methods[0], " raw-protection ");
    let proceeding = record.proceeding.as_ref().unwrap();
    assert_eq!(proceeding.type_code, " raw-Z ");
    assert_eq!(proceeding.serial_number, "001900");
    assert_eq!(proceeding.year, "02020");
    assert_eq!(proceeding.workplace_code, "raw-workplace");
    assert_eq!(record.purposes.as_ref().unwrap().len(), 2);
    assert_eq!(record.building_right.as_ref().unwrap().parcels.len(), 2);
    assert_eq!(record.building_components.len(), 2);
    assert_eq!(
        record.building_components[0].temporary.as_deref(),
        Some(" raw-temporary ")
    );
    assert!(record.building_components[1].temporary.is_none());
    let sheets = record.ownership_sheets.as_ref().unwrap();
    assert_eq!(sheets.len(), 1);
    assert_eq!(sheets[0].sheet.id.as_ref().unwrap().as_str(), "");
    assert_eq!(sheets[0].sheet.number, "00116");
    assert_eq!(sheets[0].ownerships.len(), 2);
    assert_eq!(
        sheets[0].ownerships[0].relationship_type,
        " raw-relationship "
    );
    assert_eq!(sheets[0].ownerships[0].alternate_identifications.len(), 2);
    assert!(
        sheets[0].ownerships[0].alternate_identifications[0]
            .targets
            .is_empty()
    );
}

#[test]
fn building_right_optional_wrappers_and_requiredness_are_exact() {
    let empty = replace_element(
        &replace_element(
            &building_right_document(),
            "SOUCASTI_STAVBA",
            "<SOUCASTI_STAVBA/>",
        ),
        "LISTY_VLASTNICTVI",
        "<LISTY_VLASTNICTVI/>",
    );
    let parsed = parse_building_right(&empty).unwrap();
    let record = &parsed.header.records.unwrap()[0];
    assert!(record.building_components.is_empty());
    assert_eq!(record.ownership_sheets, Some(Vec::new()));

    for wrapper in ["PRAVA_STAVEB", "UCELY_PRAVA_STAVBY"] {
        let xml = replace_element(
            &building_right_document(),
            wrapper,
            &format!("<{wrapper}/>"),
        );
        assert!(
            parse_building_right(&xml).is_err(),
            "empty required-list wrapper {wrapper} was accepted"
        );
    }
    let no_alternates = building_right_document()
        .replace("<VLA_IDENT2/>", "")
        .replace("<VLA_IDENT2><parcela/></VLA_IDENT2>", "");
    assert!(parse_building_right(&no_alternates).is_err());
    assert!(
        parse_building_right(&remove_element(&building_right_document(), "TYP_VZTAHU")).is_err()
    );
    let duplicate = building_right_document().replace(
        "<BEZPL_PRISTUP> raw-free </BEZPL_PRISTUP>",
        "<BEZPL_PRISTUP/><BEZPL_PRISTUP/>",
    );
    assert!(parse_building_right(&duplicate).is_err());
}

#[test]
fn proceeding_report_covers_every_transitive_field_and_any_type_text() {
    let parsed = parse_proceeding(&proceeding_document()).unwrap();
    assert_eq!(parsed.created_by, "creator");
    assert_eq!(parsed.created_at, "created-report");
    assert_eq!(parsed.output_info.as_deref(), Some(""));
    assert_eq!(parsed.participants.len(), 3);
    let participant = &parsed.participants[0];
    let Some(ProceedingInfoParticipantNameChoice::Person(person)) = &participant.name.value else {
        panic!("first participant must be a person");
    };
    assert_eq!(person.surname, "Surname");
    assert_eq!(person.given_name, "Given");
    assert_eq!(person.title_before.as_deref(), Some("Ing."));
    assert_eq!(person.title_after.as_deref(), Some("Ph.D."));
    assert_eq!(participant.identification.entries.len(), 7);
    assert!(matches!(
        participant.identification.entries[1],
        ProceedingInfoParticipantIdentificationEntry::PersonalNumber6(ref raw)
            if raw == " raw-rc6 "
    ));
    assert!(matches!(
        participant.identification.entries[6],
        ProceedingInfoParticipantIdentificationEntry::Id(ref id)
            if id.as_str() == "999999999999999999999999999999999999"
    ));
    assert_eq!(participant.addresses.len(), 2);
    assert!(participant.addresses[1].is_none());
    assert_eq!(participant.participation_kinds.len(), 2);
    assert_eq!(
        participant.participation_kinds[0]
            .details
            .as_ref()
            .unwrap()
            .code,
        " raw-participation "
    );
    assert!(participant.participation_kinds[1].details.is_none());
    assert!(matches!(
        parsed.participants[1].name.value,
        Some(ProceedingInfoParticipantNameChoice::Organization(ref name))
            if name == " Organization "
    ));
    assert!(parsed.participants[2].name.value.is_none());

    let details = &parsed.proceeding;
    assert_eq!(details.assigned_at, " assigned ");
    assert_eq!(details.exempt, " raw-exempt ");
    let number = details.number.identification.as_ref().unwrap();
    assert_eq!(number.type_code, " raw-Z ");
    assert_eq!(number.serial_number, u32::MAX);
    assert_eq!(number.year, u16::MAX);
    assert_eq!(number.workplace_code, u16::MAX);
    assert_eq!(
        details.payment_state.details.as_ref().unwrap().code,
        " raw-payment "
    );
    assert_eq!(parsed.subjects.len(), 2);
    assert_eq!(parsed.subjects[0].type_code, " raw-subject ");
    assert_eq!(parsed.cadastral_areas.len(), 2);
    assert!(parsed.cadastral_areas[0].areas[0].details.is_none());
    assert_eq!(
        parsed.cadastral_areas[0].areas[1]
            .details
            .as_ref()
            .unwrap()
            .code,
        u32::MAX
    );
    assert!(parsed.cadastral_areas[1].areas.is_empty());
    assert_eq!(parsed.parcels.len(), 2);
    assert_eq!(parsed.parcels[0].kind, " raw-parcel-kind ");
    assert_eq!(parsed.parcels[0].part, "raw-part");
    assert!(parsed.parcels[1].parcel.is_none());
    assert_eq!(parsed.buildings.len(), 2);
    assert!(parsed.buildings[1].building.is_none());
    assert_eq!(parsed.deeds.len(), 2);
    assert_eq!(parsed.deeds[0].id.as_ref().unwrap().as_str(), "");
    assert_eq!(parsed.deeds[0].deed_type_code, Some(u16::MAX));
    assert_eq!(parsed.deeds[0].prepared_at.as_deref(), Some("prepared"));
    assert_eq!(parsed.deeds[0].legal_force_at.as_deref(), Some("force"));
    assert_eq!(
        parsed.deeds[0].additional_data.as_deref(),
        Some("additional")
    );
    assert_eq!(parsed.deeds[0].deed_type_name.as_deref(), Some("deed type"));
    assert_eq!(parsed.deeds[0].description.as_deref(), Some("description"));
    assert_eq!(parsed.deeds[0].history_from.as_deref(), Some("from"));
    assert_eq!(parsed.deeds[0].history_to.as_deref(), Some("to"));
    assert_eq!(
        parsed.deeds[0].additional_data_from.as_deref(),
        Some("additional-from")
    );
    assert_eq!(parsed.deeds[0].preparer.as_deref(), Some("preparer"));
    assert_eq!(parsed.deeds[1], ProceedingInfoDeed::default());
    assert_eq!(parsed.units.len(), 2);
    assert_eq!(parsed.units[0].use_description, " raw-unit-use ");
    assert!(parsed.units[1].unit.is_none());
    let plan = parsed.geometric_plan.as_ref().unwrap();
    assert_eq!(plan.kind, " raw-plan-kind ");
    assert_eq!(plan.number, "raw-plan-number");
    assert_eq!(plan.purposes.len(), 2);
    assert_eq!(parsed.events.len(), 1);
    assert_eq!(parsed.events[0].type_code, " raw-event ");
    assert_eq!(parsed.events[0].text, " event text ");
    assert_eq!(parsed.operations.len(), 1);
    assert_eq!(parsed.operations[0].type_code, " raw-operation ");
    assert_eq!(
        parsed.operations[0].text.raw_xml,
        r#"<TEXT custom="raw">before<extra xmlns="urn:extra">nested &amp; retained</extra>after</TEXT>"#
    );
    assert_eq!(parsed.operations[0].created_at, "operation-created");
    assert_eq!(parsed.operations[0].completed_at, "operation-completed");
    assert_eq!(parsed.geometric_plan_defects.len(), 1);
    assert_eq!(parsed.geometric_plan_defects[0].name, "Defect");
    assert_eq!(
        parsed.geometric_plan_defects[0].specification,
        "Specification"
    );
    assert_eq!(parsed.change_proposals.len(), 1);
    assert!(
        parsed.change_proposals[0]
            .proceeding
            .identification
            .is_none()
    );
    assert_eq!(parsed.change_proposals[0].state, " raw-proposal-state ");
    assert_eq!(parsed.building_rights.len(), 1);
    assert_eq!(
        parsed.building_rights[0]
            .building_right
            .id
            .as_ref()
            .unwrap()
            .as_str(),
        "4"
    );
    assert_eq!(parsed.building_rights[0].purpose, " right purpose ");
    assert_eq!(parsed.building_rights[0].valid_until, "right-valid-until");
}

#[test]
fn proceeding_empty_sequences_choices_and_required_wrappers_are_distinct() {
    let parsed = parse_proceeding(&minimal_proceeding_document()).unwrap();
    assert!(parsed.participants.is_empty());
    assert!(parsed.subjects.is_empty());
    assert!(parsed.cadastral_areas.is_empty());
    assert!(parsed.parcels.is_empty());
    assert!(parsed.buildings.is_empty());
    assert!(parsed.deeds.is_empty());
    assert!(parsed.units.is_empty());
    assert!(parsed.geometric_plan.is_none());
    assert!(parsed.events.is_empty());
    assert!(parsed.operations.is_empty());
    assert!(parsed.geometric_plan_defects.is_empty());
    assert!(parsed.change_proposals.is_empty());
    assert!(parsed.building_rights.is_empty());
    assert!(parsed.proceeding.number.identification.is_none());
    assert!(parsed.proceeding.payment_state.details.is_none());

    for wrapper in [
        "UCASTNICI",
        "PREDMETY_RIZENI",
        "KATASTRY",
        "OBJEKTY_PARCELY",
        "OBJEKTY_STAVBY",
        "LISTINY",
        "OBJEKTY_JED",
        "UDALOSTI",
        "VYK_OPERACE",
        "ZAVADY_GP",
        "NAVRHY_ZMEN",
        "OBJEKTY_PRAVA_STAVBY",
    ] {
        let xml = remove_element(&minimal_proceeding_document(), wrapper);
        assert!(
            parse_proceeding(&xml).is_err(),
            "missing required wrapper {wrapper} was accepted"
        );
    }

    let partial_payment = minimal_proceeding_document().replace(
        "<STAV_UHRADY/>",
        "<STAV_UHRADY><kod>only-code</kod></STAV_UHRADY>",
    );
    assert!(parse_proceeding(&partial_payment).is_err());
    let partial_cadastral = minimal_proceeding_document().replace(
        "<KATASTRY/>",
        "<KATASTRY><KATASTR><KATUZE><kod>1</kod></KATUZE></KATASTR></KATASTRY>",
    );
    assert!(parse_proceeding(&partial_cadastral).is_err());
}

#[test]
fn proceeding_participant_choices_and_required_lists_are_enforced() {
    let no_address = minimal_proceeding_document().replace(
        "<UCASTNICI/>",
        "<UCASTNICI><UCASTNIK><CELE_JMENO/><IDENTIFIKACE/><TYP_UCAST/></UCASTNIK></UCASTNICI>",
    );
    assert!(parse_proceeding(&no_address).is_err());
    let no_kind = minimal_proceeding_document().replace(
        "<UCASTNICI/>",
        "<UCASTNICI><UCASTNIK><CELE_JMENO/><IDENTIFIKACE/><UCAST_ADRESA/></UCASTNIK></UCASTNICI>",
    );
    assert!(parse_proceeding(&no_kind).is_err());
    let incomplete_person = minimal_proceeding_document().replace(
        "<UCASTNICI/>",
        "<UCASTNICI><UCASTNIK><CELE_JMENO><prijmeni>Only surname</prijmeni></CELE_JMENO><IDENTIFIKACE/><UCAST_ADRESA/><TYP_UCAST/></UCASTNIK></UCASTNICI>",
    );
    assert!(parse_proceeding(&incomplete_person).is_err());
    let ambiguous = proceeding_document().replace(
        "<titul_za_jmenem>Ph.D.</titul_za_jmenem></CELE_JMENO>",
        "<titul_za_jmenem>Ph.D.</titul_za_jmenem><obchodni_jmeno>Ambiguous</obchodni_jmeno></CELE_JMENO>",
    );
    assert!(parse_proceeding(&ambiguous).is_err());

    let overflow = proceeding_document().replace(
        "<TYPLIST_KOD>65535</TYPLIST_KOD>",
        "<TYPLIST_KOD>65536</TYPLIST_KOD>",
    );
    assert!(parse_proceeding(&overflow).is_err());
    let duplicate = minimal_proceeding_document().replace(
        "<BEZPL_PRISTUP>free</BEZPL_PRISTUP>",
        "<BEZPL_PRISTUP/><BEZPL_PRISTUP/>",
    );
    assert!(parse_proceeding(&duplicate).is_err());
}

fn assert_common_rejections<T>(parser: fn(&[u8], u32, u32) -> Result<T>, valid: &str, root: &str) {
    assert!(parser(b"\xff", MAX_NODES, MAX_DEPTH).is_err());
    assert!(parser(b"<Other/>", MAX_NODES, MAX_DEPTH).is_err());
    let namespaced = valid.replacen(
        &format!("<{root}>"),
        &format!("<{root} xmlns=\"urn:unexpected\">"),
        1,
    );
    assert!(parser(namespaced.as_bytes(), MAX_NODES, MAX_DEPTH).is_err());
    let dtd = format!("<!DOCTYPE {root} [<!ENTITY e SYSTEM 'file:///etc/passwd'>]>{valid}");
    assert!(parser(dtd.as_bytes(), MAX_NODES, MAX_DEPTH).is_err());
    assert!(parser(valid.as_bytes(), 0, MAX_DEPTH).is_err());
    assert!(parser(valid.as_bytes(), 1, MAX_DEPTH).is_err());
    assert!(parser(valid.as_bytes(), MAX_NODES, 0).is_err());
    assert!(parser(valid.as_bytes(), MAX_NODES, 1).is_err());
}

#[test]
fn all_parsers_reject_dtd_bad_roots_namespaces_and_limit_violations() {
    assert_common_rejections(
        parse_unit_info,
        &unit_document(FLOOR_AREA),
        "InformaceOJednotkach",
    );
    assert_common_rejections(
        parse_building_right_info,
        &building_right_document(),
        "InformaceOPravechStavby",
    );
    assert_common_rejections(
        parse_proceeding_info,
        &minimal_proceeding_document(),
        "InformaceORizeni",
    );

    let nil = minimal_proceeding_document().replace(
        "<PREDMETY_RIZENI/>",
        r#"<PREDMETY_RIZENI xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:nil="true"/>"#,
    );
    assert!(parse_proceeding(&nil).is_err());

    let any_type_nil = proceeding_document().replace(
        r#"<extra xmlns="urn:extra">nested &amp; retained</extra>"#,
        r#"<extra xmlns="urn:extra" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:nil="true"/>"#,
    );
    let parsed = parse_proceeding(&any_type_nil).unwrap();
    assert!(
        parsed.operations[0]
            .text
            .raw_xml
            .contains("xsi:nil=\"true\"")
    );

    let unknown = minimal_proceeding_document().replace(
        "<VYHOTOVIL>creator</VYHOTOVIL>",
        "<UNKNOWN/><VYHOTOVIL>creator</VYHOTOVIL>",
    );
    assert!(parse_proceeding(&unknown).is_err());
}
