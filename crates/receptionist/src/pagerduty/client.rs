use crate::pagerduty::models::OncallList;
use anyhow::{anyhow, Result};
use hyper::client::{Client, HttpConnector};
use hyper::header::AUTHORIZATION;
use hyper::{Body, Request, Uri};
use hyper_rustls::{ConfigBuilderExt, HttpsConnector, HttpsConnectorBuilder};
use serde_json::from_slice;

const DEFAULT_PD_URL: &str = "https://api.pagerduty.com";

#[derive(Debug, Clone)]
pub struct PagerDuty {
    auth: String,
    base_url: String,
    client: Client<HttpsConnector<HttpConnector>>,
}

impl PagerDuty {
    pub async fn get_oncalls(&self, escalation_policy: String) -> Result<OncallList> {
        let resource = self.base_url.clone()
            + "/oncalls?earliest=true&include[]=users&escalation_policy_ids[]="
            + escalation_policy.as_str();

        let uri: Uri = resource
            .parse()
            .map_err(|e| anyhow!("invalid url schema: {} {}", resource, e))?;

        let authorization_token = "Token token=".to_owned() + self.auth.as_str();
        let request = Request::builder()
            .header(AUTHORIZATION, authorization_token)
            .uri(uri)
            .body(Body::empty())?;

        let response = self.client.request(request).await?;
        let bytes = hyper::body::to_bytes(response.into_body()).await?;

        let mut oncalls_list: OncallList = from_slice(bytes.as_ref())?;

        oncalls_list
            .oncalls
            .sort_by(|a, b| a.escalation_level.cmp(&b.escalation_level));

        Ok(oncalls_list)
    }
}

impl PagerDuty {
    pub fn new(auth: String, base_url: Option<String>) -> Self {
        // let https_connector = HttpsConnector::new();

        // let mut root_store = rustls::RootCertStore::empty();
        // root_store.add_server_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.0.iter().map(|ta| {
        //     rustls::OwnedTrustAnchor::from_subject_spki_name_constraints(
        //         ta.subject,
        //         ta.spki,
        //         ta.name_constraints,
        //     )
        // }));
        let https = HttpsConnectorBuilder::new()
            .with_tls_config(
                rustls::ClientConfig::builder()
                    .with_safe_defaults()
                    // .with_native_roots()
                    // .with_root_certificates(root_store)
                    .with_native_roots()
                    // .with_webpki_roots()
                    .with_no_client_auth(),
            )
            .https_or_http()
            .enable_http1()
            .build();
        let client = Client::builder().build::<_, Body>(https);

        Self {
            auth,
            base_url: base_url.unwrap_or_else(|| DEFAULT_PD_URL.to_string()),
            client,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn should_get_oncalls() {
        dotenv::dotenv().ok();
        let pd = PagerDuty::new(
            std::env::var("PAGERDUTY_TOKEN")
                .expect("Missing PAGERDUTY_TOKEN environment variable. Add it to .env file."),
            std::env::var("PAGERDUTY_BASE_URL").ok(),
        );

        // let resp = pd.get_oncalls(String::from("PS32312")).await;
        let resp = pd.get_oncalls(String::from("PLMIEBZ")).await;

        match resp {
            Ok(ok) => {
                assert!(!ok.oncalls.is_empty())
            }
            Err(ko) => {
                dbg!(ko);
                assert!(false, "Unexpected failure")
            }
        }
        assert!(true);
    }
}
