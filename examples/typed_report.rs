//! Use the single-output typed parcel workflow with an explicit charge ceiling.

use wsdp::{
    AccountContext, ChargeApproval, Client, Credentials, Czk, Environment, ParcelId,
    ParcelInfoRequest, ParcelSelector, ReportOptions, WaitOptions,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder(Environment::Trial)
        .credentials(Credentials::new(
            std::env::var("WSDP_TRIAL_USERNAME")?,
            std::env::var("WSDP_TRIAL_PASSWORD")?,
        ))
        .build()?;
    let request = ParcelInfoRequest::new(
        ParcelSelector::Id(ParcelId::new("2928347306")?),
        ReportOptions::xml_default(AccountContext::Paying),
    );
    let report = client
        .parcel_info_report(
            request,
            WaitOptions::default(),
            ChargeApproval::up_to(Czk::from_major_units(10)),
        )
        .await?;
    println!(
        "Cadastral data present: {}",
        report.value.cadastre.is_some()
    );
    println!("Original XML bytes: {}", report.original_xml().len());

    // Typed helpers never delete implicitly; remove this explicitly created report.
    client.delete_report(report.metadata.id()).await?;
    Ok(())
}
