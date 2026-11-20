//! Decode the published September–December 2020 trial price archive in memory.

use wsdp::{
    AccountContext, CadastralAreaCode, ChargeApproval, Client, Credentials, Czk, Environment,
    PricesByCadastralAreaRequest, WaitOptions,
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
        .prices_by_cadastral_area_report(
            PricesByCadastralAreaRequest::new(
                CadastralAreaCode::new("732630")?,
                2020,
                9,
                12,
                AccountContext::Paying,
            ),
            WaitOptions::default(),
            ChargeApproval::up_to(Czk::from_major_units(10)),
        )
        .await?;
    println!(
        "Safely decoded monthly entries: {}",
        report.value.entries.len()
    );
    println!("Original ZIP bytes: {}", report.original_zip().len());
    client.delete_report(report.metadata.id()).await?;
    Ok(())
}
