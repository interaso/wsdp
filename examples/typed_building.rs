//! Generate building information with an explicitly pinned legacy XML version.

use wsdp::{
    AccountContext, BuildingId, BuildingInfoRequest, BuildingSelector, ChargeApproval, Client,
    Credentials, Czk, Environment, ReportOptions, ReportVersion, WaitOptions,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder(Environment::Trial)
        .credentials(Credentials::new(
            std::env::var("WSDP_TRIAL_USERNAME")?,
            std::env::var("WSDP_TRIAL_PASSWORD")?,
        ))
        .build()?;
    let report = client
        .building_info_report(
            BuildingInfoRequest::new(
                BuildingSelector::Id(BuildingId::new("286869306")?),
                ReportOptions::xml(ReportVersion::V1_0, AccountContext::Paying),
            ),
            WaitOptions::default(),
            ChargeApproval::up_to(Czk::from_major_units(10)),
        )
        .await?;
    println!(
        "Original building XML bytes: {}",
        report.original_xml().len()
    );
    client.delete_report(report.metadata.id()).await?;
    Ok(())
}
