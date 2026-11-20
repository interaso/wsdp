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
    cleanup.expect("delete only the report created by this test");
}

#[tokio::test]
#[ignore = "contacts the shared official WSDP trial environment"]
async fn report_version_discovery() {
    let Some(client) = trial_client() else {
        panic!("set WSDP_RUN_TRIAL_TESTS=1 and WSDP_TRIAL_USERNAME/PASSWORD");
    };
    let response = client
        .report_versions(Some(&[ReportFamily::OwnershipSheet]))
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
