//! Separately opted-in, charge-capped production smoke tests.
//!
//! Credentials and selectors come only from the environment. No owner data,
//! report IDs, credentials, or report contents are printed.

use std::collections::HashSet;
use std::io;

use wsdp::{
    AccountContext, BuildingId, BuildingInfoRequest, BuildingSelector, ChargeApproval, Client,
    Credentials, Czk, DownloadedReport, Environment, Error, GenerateReport, ParcelId,
    ParcelInfoRequest, ParcelSelector, ReportId, ReportOptions, ServiceKind, WaitOptions,
};

const MAX_AUTHORIZED_TOTAL: u64 = 300;
const MAX_INDIVIDUAL_REPORT_CHARGE: u64 = 50;

fn required(name: &str) -> Result<String, io::Error> {
    std::env::var(name).map_err(|_| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("required production-test variable {name} is missing or not Unicode"),
        )
    })
}

fn enabled() -> Result<bool, io::Error> {
    match std::env::var("WSDP_RUN_PRODUCTION_TESTS") {
        Ok(value) if value == "1" => Ok(true),
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "WSDP_RUN_PRODUCTION_TESTS must be exactly 1 when set",
        )),
        Err(std::env::VarError::NotPresent) => Ok(false),
        Err(std::env::VarError::NotUnicode(_)) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "WSDP_RUN_PRODUCTION_TESTS is not Unicode",
        )),
    }
}

fn production_client() -> Result<Client, Box<dyn std::error::Error>> {
    Ok(Client::builder(Environment::Production)
        .credentials(Credentials::new(
            required("WSDP_PRODUCTION_USERNAME")?,
            required("WSDP_PRODUCTION_PASSWORD")?,
        ))
        .build()?)
}

fn individual_cap(total_cap: Czk, spent: Czk) -> wsdp::Result<Czk> {
    let remaining = Czk::new(total_cap.as_decimal() - spent.as_decimal())?;
    if remaining.as_decimal() > Czk::from_major_units(MAX_INDIVIDUAL_REPORT_CHARGE).as_decimal() {
        Ok(Czk::from_major_units(MAX_INDIVIDUAL_REPORT_CHARGE))
    } else {
        Ok(remaining)
    }
}

async fn run_xml_report(
    client: &Client,
    request: GenerateReport,
    total_cap: Czk,
    spent: Czk,
    created_ids: &mut Vec<ReportId>,
    validate: impl FnOnce(&DownloadedReport) -> wsdp::Result<()>,
) -> wsdp::Result<Czk> {
    let report_cap = individual_cap(total_cap, spent)?;
    let batch = client.generate_report(request).await?;
    let metadata = batch.value.into_single_report()?;
    let id = metadata.id().clone();
    created_ids.push(id.clone());
    client.wait_for_report(&id, WaitOptions::default()).await?;

    // This additional free status request establishes the exact remaining total
    // budget before the download helper performs its own fresh price preflight.
    let status = client.report_status(&id).await?;
    let price = status
        .value
        .price
        .ok_or_else(|| Error::ReportPriceUnavailable { id: id.clone() })?;
    if price.as_decimal() > report_cap.as_decimal() {
        return Err(Error::ChargeLimitExceeded { id: id.clone() });
    }

    let downloaded = client
        .download_report(&id, ChargeApproval::up_to(report_cap))
        .await?;
    let charged = downloaded
        .metadata
        .price
        .ok_or_else(|| Error::ReportPriceUnavailable { id: id.clone() })?;
    if charged.as_decimal() > report_cap.as_decimal() {
        return Err(Error::ChargeLimitExceeded { id: id.clone() });
    }
    let new_total = Czk::new(spent.as_decimal() + charged.as_decimal())?;
    if new_total.as_decimal() > total_cap.as_decimal() {
        return Err(Error::ChargeLimitExceeded { id: id.clone() });
    }
    validate(&downloaded)?;
    Ok(charged)
}

#[tokio::test]
#[ignore = "uses production credentials and can charge at most the explicit configured caps"]
async fn parcel_and_building_xml_lifecycles_within_charge_caps()
-> Result<(), Box<dyn std::error::Error>> {
    if !enabled()? {
        eprintln!("skipping production test: set WSDP_RUN_PRODUCTION_TESTS=1 explicitly");
        return Ok(());
    }
    let cap: Czk = required("WSDP_PRODUCTION_MAX_CZK")?.parse()?;
    if cap.as_decimal() > Czk::from_major_units(MAX_AUTHORIZED_TOTAL).as_decimal() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "production cap exceeds the separately authorized 300 CZK maximum",
        )
        .into());
    }
    let parcel_id = ParcelId::new(required("WSDP_PRODUCTION_PARCEL_ID")?)?;
    let building_id = BuildingId::new(required("WSDP_PRODUCTION_BUILDING_ID")?)?;
    let client = production_client()?;

    let mut created_ids = Vec::new();
    let mut result: wsdp::Result<()> = async {
        for service in [
            ServiceKind::CodeLists,
            ServiceKind::Search,
            ServiceKind::Information,
            ServiceKind::Reports,
            ServiceKind::Account,
        ] {
            client.service_status(service).await?;
        }

        let parcel_charge = run_xml_report(
            &client,
            GenerateReport::ParcelInfo(ParcelInfoRequest::new(
                ParcelSelector::Id(parcel_id),
                ReportOptions::xml_default(AccountContext::Paying),
            )),
            cap,
            Czk::ZERO,
            &mut created_ids,
            |downloaded| {
                let parsed = downloaded.parse_parcel_info()?;
                if parsed.original_xml() != downloaded.as_bytes() {
                    return Err(Error::InvalidResponse {
                        message: "parsed production parcel did not retain original bytes"
                            .to_owned(),
                    });
                }
                Ok(())
            },
        )
        .await?;
        run_xml_report(
            &client,
            GenerateReport::BuildingInfo(BuildingInfoRequest::new(
                BuildingSelector::Id(building_id),
                ReportOptions::xml_default(AccountContext::Paying),
            )),
            cap,
            parcel_charge,
            &mut created_ids,
            |downloaded| {
                let parsed = downloaded.parse_building_info()?;
                if parsed.original_xml() != downloaded.as_bytes() {
                    return Err(Error::InvalidResponse {
                        message: "parsed production building did not retain original bytes"
                            .to_owned(),
                    });
                }
                Ok(())
            },
        )
        .await?;
        Ok(())
    }
    .await;

    // An ambiguous first download is never repeated. Leave only that report
    // available for status reconciliation rather than deleting its charge state.
    let uncertain_id = result
        .as_ref()
        .err()
        .filter(|error| error.charge_outcome_uncertain())
        .and_then(Error::report_id)
        .cloned();
    for id in created_ids {
        if uncertain_id.as_ref() == Some(&id) {
            continue;
        }
        let cleanup = client.delete_report(&id).await;
        if result.is_ok() {
            result = cleanup.map(|_| ());
        }
    }
    result?;
    println!(
        "production parcel/building XML lifecycles passed within the 50 CZK per-report and total caps"
    );
    Ok(())
}

#[tokio::test]
#[ignore = "uses production credentials for parcel PDF+XML generation but never downloads"]
async fn parcel_pdf_with_xml_generation_without_download() -> Result<(), Box<dyn std::error::Error>>
{
    if !enabled()? {
        eprintln!("skipping production test: set WSDP_RUN_PRODUCTION_TESTS=1 explicitly");
        return Ok(());
    }
    let parcel_id = ParcelId::new(required("WSDP_PRODUCTION_PARCEL_ID")?)?;
    let client = production_client()?;
    let request = GenerateReport::ParcelInfo(ParcelInfoRequest::new(
        ParcelSelector::Id(parcel_id),
        ReportOptions::pdf_with_xml(AccountContext::Paying),
    ));
    let batch = match client.generate_report(request).await {
        Ok(response) => response.value,
        Err(Error::Service(error)) if error.code() == Some("527") => {
            println!("production parcel PDF+XML generation was rejected with service 527");
            return Ok(());
        }
        Err(error) => return Err(error.into()),
    };

    let mut created_ids = Vec::new();
    let mut seen = HashSet::new();
    for report in batch.reports {
        if seen.insert(report.id.clone()) {
            created_ids.push(report.id);
        }
        if let Some(parent) = report.parent_id
            && seen.insert(parent.clone())
        {
            created_ids.push(parent);
        }
        for child in report.child_ids {
            if seen.insert(child.clone()) {
                created_ids.push(child);
            }
        }
    }

    // Status checks are free. Discover late relationships before cleanup, but
    // deliberately do not approve or dispatch any potentially charged download.
    let mut result: wsdp::Result<()> = async {
        let mut index = 0;
        while index < created_ids.len() {
            let id = created_ids[index].clone();
            index += 1;
            let ready = client.wait_for_report(&id, WaitOptions::default()).await?;
            if let Some(parent) = ready.value.parent_id
                && seen.insert(parent.clone())
            {
                created_ids.push(parent);
            }
            for child in ready.value.child_ids {
                if seen.insert(child.clone()) {
                    created_ids.push(child);
                }
            }
        }
        Ok(())
    }
    .await;
    for id in &created_ids {
        let cleanup = client.delete_report(id).await;
        if result.is_ok() {
            result = cleanup.map(|_| ());
        }
    }
    result?;
    println!("production parcel PDF+XML generation succeeded; no output was downloaded");
    Ok(())
}
