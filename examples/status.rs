//! Query the official trial code-list service without exposing credentials.

use wsdp::{Client, Credentials, Environment, ServiceKind};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let username = std::env::var("WSDP_TRIAL_USERNAME")?;
    let password = std::env::var("WSDP_TRIAL_PASSWORD")?;
    let client = Client::builder(Environment::Trial)
        .credentials(Credentials::new(username, password))
        .build()?;
    let status = client.service_status(ServiceKind::CodeLists).await?;
    println!("Trial service version: {:?}", status.value.version);
    println!("Processed at: {:?}", status.value.processed_at);
    Ok(())
}
