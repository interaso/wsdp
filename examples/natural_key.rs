//! Find a published trial parcel by cadastral area and full parcel number.

use wsdp::{
    CadastralAreaCode, Client, Credentials, Environment, ParcelNumber, ParcelSearch,
    ParcelSearchKey, ParcelSearchSelector,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder(Environment::Trial)
        .credentials(Credentials::new(
            std::env::var("WSDP_TRIAL_USERNAME")?,
            std::env::var("WSDP_TRIAL_PASSWORD")?,
        ))
        .build()?;
    let result = client
        .find_parcels(ParcelSearch {
            selector: ParcelSearchSelector::Number(ParcelSearchKey {
                cadastral_area: CadastralAreaCode::new("608530")?,
                number: ParcelNumber::new(154, Some(16))?,
            }),
            valid_at: None,
        })
        .await?;
    println!("Matching parcels: {}", result.value.items.len());
    Ok(())
}
