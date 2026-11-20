//! Resolve a published trial building, then list all units in that building.

use wsdp::{
    BuildingId, BuildingSearch, BuildingSearchSelector, Client, Credentials, Environment,
    UnitSearch, UnitSearchKey, UnitSearchSelector,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder(Environment::Trial)
        .credentials(Credentials::new(
            std::env::var("WSDP_TRIAL_USERNAME")?,
            std::env::var("WSDP_TRIAL_PASSWORD")?,
        ))
        .build()?;
    let building = BuildingId::new("286869306")?;
    let buildings = client
        .find_buildings(BuildingSearch {
            selector: BuildingSearchSelector::Id(building.clone()),
            valid_at: None,
        })
        .await?;
    let units = client
        .find_units(UnitSearch {
            selector: UnitSearchSelector::Number(UnitSearchKey::new(
                BuildingSearchSelector::Id(building),
                None,
            )?),
            valid_at: None,
        })
        .await?;
    println!(
        "Buildings: {}, units: {}",
        buildings.value.items.len(),
        units.value.items.len()
    );
    Ok(())
}
