//! Opt-in checks against the official shared WSDP trial environment.

use wsdp::{
    AccountContext, CadastralAreaFilter, ChargeApproval, Client, CodeListRequest, CodeListResult,
    Credentials, Environment, GenerateReport, ParcelBoundingBoxRequest, ParcelId,
    ParcelInfoRequest, ParcelSearch, ParcelSearchSelector, ParcelSelector, ReportFamily,
    ReportFormat, ReportOptions, ReportVersion, ServiceKind, WaitOptions,
};

fn trial_client() -> Option<Client> {
    if std::env::var("WSDP_RUN_TRIAL_TESTS").as_deref() != Ok("1") {
        return None;
    }
    let username = std::env::var("WSDP_TRIAL_USERNAME").ok()?;
    let password = std::env::var("WSDP_TRIAL_PASSWORD").ok()?;
    Some(
        Client::builder(Environment::Trial)
            .credentials(Credentials::new(username, password))
            .build()
            .expect("valid trial client configuration"),
    )
}

#[tokio::test]
#[ignore = "contacts the shared official WSDP trial environment"]
async fn service_status_for_every_family() {
    let Some(client) = trial_client() else {
        panic!("set WSDP_RUN_TRIAL_TESTS=1 and WSDP_TRIAL_USERNAME/PASSWORD");
    };
    for service in [
        ServiceKind::CodeLists,
        ServiceKind::Search,
        ServiceKind::Information,
        ServiceKind::Reports,
        ServiceKind::Account,
    ] {
        let response = client.service_status(service).await.expect("trial stavWS");
        assert!(!response.messages.is_empty());
    }
}

#[tokio::test]
#[ignore = "contacts the shared official WSDP trial environment"]
async fn code_lists_and_legal_relation_metadata() {
    let Some(client) = trial_client() else {
        panic!("set WSDP_RUN_TRIAL_TESTS=1 and WSDP_TRIAL_USERNAME/PASSWORD");
    };
    let areas = client
        .code_list(CodeListRequest::CadastralAreas {
            filter: Some(CadastralAreaFilter::NameContains("Jáma%".to_owned())),
        })
        .await
        .expect("trial seznamKU");
    let CodeListResult::CadastralAreas(areas) = areas.value else {
        panic!("unexpected code-list result");
    };
    assert!(
        areas
            .items
            .iter()
            .any(|area| area.code.as_str() == "693936")
    );

    let relations = client
        .code_list(CodeListRequest::LegalRelationTypes)
        .await
        .expect("trial seznamTypuPravnichVztahu");
    let CodeListResult::LegalRelationTypes(relations) = relations.value else {
        panic!("unexpected legal-relation result");
    };
    assert!(
        relations
            .items
            .iter()
            .any(|relation| relation.code.as_str() == "30" && relation.is_ownership)
    );
}

#[tokio::test]
#[ignore = "contacts the shared official WSDP trial environment"]
async fn parcel_lookup_and_bounding_box() {
    let Some(client) = trial_client() else {
        panic!("set WSDP_RUN_TRIAL_TESTS=1 and WSDP_TRIAL_USERNAME/PASSWORD");
    };
    let id = ParcelId::new("2928347306").expect("published trial parcel ID");
    let parcels = client
        .find_parcels(ParcelSearch {
            selector: ParcelSearchSelector::Id(id.clone()),
            valid_at: None,
        })
        .await
        .expect("trial najdiParcelu");
    assert!(parcels.value.items.iter().any(|parcel| parcel.id == id));

    let boxes = client
        .parcel_bounding_boxes(ParcelBoundingBoxRequest::new(vec![id]).unwrap())
        .await
        .expect("trial dejMBRParcel");
    assert!(
        boxes
            .value
            .entries
            .is_some_and(|entries| !entries.is_empty())
    );
}

#[tokio::test]
#[ignore = "contacts the shared official WSDP trial environment"]
async fn all_search_families_with_published_samples() {
    use wsdp::{
        BuildingId, BuildingRightId, BuildingRightSearch, BuildingRightSelector, BuildingSearch,
        BuildingSearchSelector, CollectionDocumentSearch, CompanyId, GeometricPlanSearch,
        LegalPersonSearch, ProceedingSearch, ProceedingSearchKey, ProceedingSearchSelector,
        SubjectSearch, SubjectSearchSelector, UnitId, UnitSearch, UnitSearchSelector,
        WorkplaceCode,
    };

    let Some(client) = trial_client() else {
        panic!("set WSDP_RUN_TRIAL_TESTS=1 and WSDP_TRIAL_USERNAME/PASSWORD");
    };
    let buildings = client
        .find_buildings(BuildingSearch {
            selector: BuildingSearchSelector::Id(BuildingId::new("286869306").unwrap()),
            valid_at: None,
        })
        .await
        .expect("trial najdiStavbu");
    assert!(!buildings.value.items.is_empty());

    let units = client
        .find_units(UnitSearch {
            selector: UnitSearchSelector::Id(UnitId::new("18719306").unwrap()),
            valid_at: None,
        })
        .await
        .expect("trial najdiJednotku");
    assert!(!units.value.items.is_empty());

    let rights = client
        .find_building_rights(BuildingRightSearch {
            selector: BuildingRightSelector::Id(BuildingRightId::new("5270").unwrap()),
            valid_at: None,
        })
        .await
        .expect("trial najdiPravoStavby");
    assert!(!rights.value.items.is_empty());

    let subjects = client
        .find_subjects(SubjectSearch {
            selector: SubjectSearchSelector::LegalPerson(
                LegalPersonSearch::new(Some(CompanyId::new("22829812").unwrap()), None).unwrap(),
            ),
            valid_at: None,
        })
        .await
        .expect("trial najdiOS");
    assert!(!subjects.value.items.is_empty());

    let proceeding = ProceedingSearchSelector::Number(
        ProceedingSearchKey::new("V", 1335, 2016, WorkplaceCode::new("306").unwrap()).unwrap(),
    );
    let proceedings = client
        .find_proceedings(ProceedingSearch {
            selector: proceeding.clone(),
        })
        .await;
    match proceedings {
        Ok(response) => assert!(!response.value.items.is_empty()),
        Err(wsdp::Error::Service(error)) if error.code() == Some("387") => {
            // Current trial data reject the published proceeding samples with 387.
            // Other failures remain fatal; wire contracts are tested offline.
        }
        Err(error) => panic!("trial najdiRizeni failed: {error}"),
    }

    match client
        .find_collection_documents(CollectionDocumentSearch { proceeding })
        .await
    {
        Ok(_) => {}
        Err(wsdp::Error::Service(error)) if error.code() == Some("387") => {}
        Err(error) => panic!("trial najdiListinuVeSbirceListin failed: {error}"),
    }

    let plans = client
        .find_geometric_plans(
            GeometricPlanSearch::new(wsdp::CadastralAreaCode::new("693936").unwrap(), 307).unwrap(),
        )
        .await
        .expect("trial najdiGP");
    assert!(!plans.value.items.is_empty());
}

#[tokio::test]
#[ignore = "contacts the shared official WSDP trial environment"]
async fn raw_report_lifecycle_with_explicit_approval_and_cleanup() {
    let Some(client) = trial_client() else {
        panic!("set WSDP_RUN_TRIAL_TESTS=1 and WSDP_TRIAL_USERNAME/PASSWORD");
    };
    let batch = client
        .generate_report(GenerateReport::ParcelInfo(ParcelInfoRequest::new(
            ParcelSelector::Id(ParcelId::new("2928347306").unwrap()),
            ReportOptions::xml_default(AccountContext::Paying),
        )))
        .await
        .expect("trial report generation");
    let metadata = batch.value.into_single_report().expect("one parcel report");
    let id = metadata.id().clone();
    let lifecycle = async {
        let ready = client.wait_for_report(&id, WaitOptions::default()).await?;
        let downloaded = client.download_report(&id, ChargeApproval::Any).await?;
        Ok::<_, wsdp::Error>((ready, downloaded))
    }
    .await;
    let cleanup = client.delete_report(&id).await;
    let (ready, downloaded) = lifecycle.expect("approved trial report lifecycle");
    assert!(ready.value.state.is_ready());
    assert_eq!(downloaded.id(), &id);
    assert_eq!(downloaded.metadata.format, ReportFormat::Xml);
    assert!(!downloaded.bytes.is_empty());
    let parsed = downloaded
        .parse_parcel_info()
        .expect("typed trial parcel XML");
    assert_eq!(parsed.original_xml(), downloaded.as_bytes());
    cleanup.expect("delete only the report created by this test");
}

#[tokio::test]
#[ignore = "contacts the shared official WSDP trial environment"]
async fn typed_ntr_ownership_sheet_with_cleanup() {
    use wsdp::reports::OwnershipSheetReport;
    use wsdp::{
        CadastralAreaCode, OwnershipSheetNumber, OwnershipSheetRequest, OwnershipSheetSelector,
    };

    let Some(client) = trial_client() else {
        panic!("set WSDP_RUN_TRIAL_TESTS=1 and WSDP_TRIAL_USERNAME/PASSWORD");
    };
    let result = client
        .ownership_sheet_report(
            OwnershipSheetRequest::new(
                OwnershipSheetSelector::Number {
                    cadastral_area: CadastralAreaCode::new("693936").unwrap(),
                    number: OwnershipSheetNumber::new(318).unwrap(),
                },
                ReportOptions::xml_default(AccountContext::Paying),
            ),
            WaitOptions::default(),
            ChargeApproval::Any,
        )
        .await;
    let id = match &result {
        Ok(report) => Some(report.metadata.id().clone()),
        Err(error) => error.report_id().cloned(),
    };
    let cleanup = if let Some(id) = id {
        Some(client.delete_report(&id).await)
    } else {
        None
    };
    let report = result.expect("typed NTR LV 3.0 trial report");
    assert!(matches!(report.value, OwnershipSheetReport::Ntr3(_)));
    assert!(!report.original_xml().is_empty());
    if let Some(cleanup) = cleanup {
        cleanup.expect("delete only the NTR report created by this test");
    }
}

#[tokio::test]
#[ignore = "contacts the shared official WSDP trial environment"]
async fn report_version_discovery() {
    let Some(client) = trial_client() else {
        panic!("set WSDP_RUN_TRIAL_TESTS=1 and WSDP_TRIAL_USERNAME/PASSWORD");
    };
    let response = client
        .report_versions(None)
        .await
        .expect("trial dejVerzeSestav");
    let sheet = response
        .value
        .iter()
        .find(|info| info.family == ReportFamily::OwnershipSheet)
        .expect("ownership-sheet versions");
    assert!(sheet.versions.contains(&ReportVersion::V1_0));
    assert!(sheet.versions.contains(&ReportVersion::V3_0));
}
