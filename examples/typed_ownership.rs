//! Generate trial LV 318 using the default, explicitly pinned NTR 3.0 version.

use wsdp::reports::OwnershipSheetReport;
use wsdp::{
    AccountContext, CadastralAreaCode, ChargeApproval, Client, Credentials, Czk, Environment,
    OwnershipSheetNumber, OwnershipSheetRequest, OwnershipSheetSelector, ReportOptions,
    WaitOptions,
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
        .ownership_sheet_report(
            OwnershipSheetRequest::new(
                OwnershipSheetSelector::Number {
                    cadastral_area: CadastralAreaCode::new("693936")?,
                    number: OwnershipSheetNumber::new(318)?,
                },
                ReportOptions::xml_default(AccountContext::Paying),
            ),
            WaitOptions::default(),
            ChargeApproval::up_to(Czk::from_major_units(10)),
        )
        .await?;
    assert!(matches!(report.value, OwnershipSheetReport::Ntr3(_)));
    println!("NTR LV bytes: {}", report.original_xml().len());
    client.delete_report(report.metadata.id()).await?;
    Ok(())
}
