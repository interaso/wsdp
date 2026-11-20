//! Run the raw lifecycle with an explicit price ceiling and typed decoding.

use wsdp::{
    AccountContext, ChargeApproval, Client, Credentials, Czk, Environment, GenerateReport,
    ParcelId, ParcelInfoRequest, ParcelSelector, ReportOptions, WaitOptions,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder(Environment::Trial)
        .credentials(Credentials::new(
            std::env::var("WSDP_TRIAL_USERNAME")?,
            std::env::var("WSDP_TRIAL_PASSWORD")?,
        ))
        .build()?;
    let batch = client
        .generate_report(GenerateReport::ParcelInfo(ParcelInfoRequest::new(
            ParcelSelector::Id(ParcelId::new("2928347306")?),
            ReportOptions::xml_default(AccountContext::Paying),
        )))
        .await?;
    let metadata = batch.value.into_single_report()?;
    let id = metadata.id().clone();

    // Persist `id.as_str()` in the calling application if recovery across a
    // restart is needed. Waiting never retrieves or charges the report.
    client.wait_for_report(&id, WaitOptions::default()).await?;
    let downloaded = client
        .download_report(&id, ChargeApproval::up_to(Czk::from_major_units(10)))
        .await?;
    let parsed = downloaded.parse_parcel_info()?;
    println!("Downloaded XML bytes: {}", parsed.original_xml().len());

    // Deletion is a separate, explicit operation affecting only this new report.
    client.delete_report(&id).await?;
    Ok(())
}
