//! Keep every PDF+XML parent/child output visible and approve each download separately.

use std::collections::HashSet;

use wsdp::{
    AccountContext, ChargeApproval, Client, Credentials, Czk, Environment, GenerateReport,
    ParcelId, ParcelInfoRequest, ParcelSelector, ReportFormat, ReportOptions, WaitOptions,
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
        ReportOptions::pdf_with_xml(AccountContext::Paying),
    );
    let batch = client
        .generate_report(GenerateReport::ParcelInfo(request))
        .await?;

    // A server may list both outputs or expose a child only through its parent.
    // Retain all known IDs instead of assuming that the batch has one entry.
    let mut ids = Vec::new();
    let mut seen = HashSet::new();
    for report in &batch.value.reports {
        if seen.insert(report.id.clone()) {
            ids.push(report.id.clone());
        }
        for child in &report.child_ids {
            if seen.insert(child.clone()) {
                ids.push(child.clone());
            }
        }
    }
    for id in ids {
        client.wait_for_report(&id, WaitOptions::default()).await?;
        // This is a fresh approval and price preflight for this one report ID.
        // It does not authorize any other parent or child output.
        let output = client
            .download_report(&id, ChargeApproval::up_to(Czk::from_major_units(10)))
            .await?;
        match output.metadata.format {
            ReportFormat::Pdf => println!("PDF bytes: {}", output.bytes.len()),
            ReportFormat::Xml => {
                let xml = output.parse_parcel_info()?;
                println!("Related XML bytes: {}", xml.original_xml().len());
            }
            _ => println!("Other output bytes: {}", output.bytes.len()),
        }
        // Persist each ID for explicit later retrieval or deletion. This example
        // deliberately performs no automatic deletion of a multi-output bundle.
    }
    Ok(())
}
