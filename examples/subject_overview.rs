//! Resolve an anonymized legal person and generate its typed ownership overview.

use wsdp::{
    AccountContext, ChargeApproval, Client, CompanyId, Credentials, Czk, Environment,
    LegalPersonSearch, OwnershipOverviewRequest, ReportOptions, SubjectSearch,
    SubjectSearchSelector, WaitOptions,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder(Environment::Trial)
        .credentials(Credentials::new(
            std::env::var("WSDP_TRIAL_USERNAME")?,
            std::env::var("WSDP_TRIAL_PASSWORD")?,
        ))
        .build()?;
    let subjects = client
        .find_subjects(SubjectSearch {
            selector: SubjectSearchSelector::LegalPerson(LegalPersonSearch::new(
                Some(CompanyId::new("22829812")?),
                None,
            )?),
            valid_at: None,
        })
        .await?;
    let subject = subjects
        .value
        .items
        .first()
        .expect("published trial subject");
    let subject_id = subject.base.id.clone();
    let report = client
        .ownership_overview_report(
            OwnershipOverviewRequest::new(
                subject_id,
                ReportOptions::xml_default(AccountContext::Paying),
            ),
            WaitOptions::default(),
            ChargeApproval::up_to(Czk::from_major_units(10)),
        )
        .await?;
    println!("Overview districts: {}", report.value.districts.len());
    client.delete_report(report.metadata.id()).await?;
    Ok(())
}
