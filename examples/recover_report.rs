//! Resume from an application-persisted report ID without regenerating the report.

use wsdp::{ChargeApproval, Client, Credentials, Czk, Environment, ReportId, WaitOptions};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder(Environment::Trial)
        .credentials(Credentials::new(
            std::env::var("WSDP_TRIAL_USERNAME")?,
            std::env::var("WSDP_TRIAL_PASSWORD")?,
        ))
        .build()?;
    let id = ReportId::new(std::env::var("WSDP_REPORT_ID")?)?;

    let status = client.report_status(&id).await?;
    if status.value.state.is_pending()
        && let Err(error) = client.wait_for_report(&id, WaitOptions::default()).await
    {
        // `error.report_id()` retains the ID to persist for a later attempt.
        // Do not regenerate merely because waiting timed out or failed.
        return Err(error.into());
    }

    match client
        .download_report(&id, ChargeApproval::up_to(Czk::from_major_units(10)))
        .await
    {
        Ok(report) => {
            println!("Recovered report bytes: {}", report.bytes.len());
        }
        Err(error) if error.charge_outcome_uncertain() => {
            // Reconcile with report_status later. After `zaúčtován`, the official
            // first-open rule says an explicit subsequent retrieval is not charged
            // again. Never blind-retry a timed-out first download.
            eprintln!("Download outcome is uncertain; reconcile status before retrieval");
            return Err(error.into());
        }
        Err(error) => return Err(error.into()),
    }
    Ok(())
}
