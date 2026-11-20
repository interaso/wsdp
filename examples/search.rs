//! Resolve the published trial parcel by its internal ID.

use wsdp::{Client, Credentials, Environment, ParcelId, ParcelSearch, ParcelSearchSelector};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder(Environment::Trial)
        .credentials(Credentials::new(
            std::env::var("WSDP_TRIAL_USERNAME")?,
            std::env::var("WSDP_TRIAL_PASSWORD")?,
        ))
        .build()?;
    let response = client
        .find_parcels(ParcelSearch {
            selector: ParcelSearchSelector::Id(ParcelId::new("2928347306")?),
            valid_at: None,
        })
        .await?;
    println!("Parcel count: {}", response.value.items.len());
    println!("Completion: {:?}", response.value.completion);
    println!("Service messages: {}", response.messages.len());
    Ok(())
}
