//! Opt-in checks against the official shared WSDP trial environment.

use wsdp::{Client, Credentials, Environment, ServiceKind};

fn trial_client() -> Option<Client> {
    if std::env::var("WSDP_RUN_TRIAL_TESTS").as_deref() != Ok("1") {
        return None;
    }
    let username = std::env::var("WSDP_TRIAL_USERNAME").ok()?;
    let password = std::env::var("WSDP_TRIAL_PASSWORD").ok()?;
    Some(
        Client::builder(Environment::Trial)
            .credentials(Credentials::new(username, password))
            .build()
            .expect("valid trial client configuration"),
    )
}

#[tokio::test]
#[ignore = "contacts the shared official WSDP trial environment"]
async fn service_status_for_every_family() {
    let Some(client) = trial_client() else {
        panic!("set WSDP_RUN_TRIAL_TESTS=1 and WSDP_TRIAL_USERNAME/PASSWORD");
    };
    for service in [
        ServiceKind::CodeLists,
        ServiceKind::Search,
        ServiceKind::Information,
        ServiceKind::Reports,
        ServiceKind::Account,
    ] {
        let response = client.service_status(service).await.expect("trial stavWS");
        assert!(!response.messages.is_empty());
    }
}
