use std::{env, time::Duration};

use chrono::{DateTime, Utc};
use reqwest::{
    header::{HeaderMap, HeaderValue, AUTHORIZATION},
    Client, Proxy,
};
use tracing::{error, warn};

use crate::{prices::PriceValue, types::ListingId};

// #[derive(Debug, PartialEq)]
// pub enum CsfloatBuyResult {
//     Success,
//     Error(String),
// }

// trait CsfloatAutobuyTrait {
//     async fn buy_listing(&self, listing_id: &ListingId, price: PriceValue) -> CsfloatBuyResult;
// }

pub struct CsfloatAutobuy {
    // pub api_key: String,
    pub next_call: DateTime<Utc>,
    pub client: Client,
}

impl CsfloatAutobuy {
    pub fn from_env() -> CsfloatAutobuy {
        let api_key = env::var("CSFLOAT_API_KEY").expect("CSFLOAT_API_KEY must be set");
        let proxy = match env::var("CSFLOAT_PROXY") {
            Ok(val) => Some(val),
            Err(e) => {
                error!("Proxy for csfloat is not set! {:?}", e);
                None
            }
        };
        CsfloatAutobuy::new(api_key, proxy)
    }

    pub fn new(api_key: String, proxy: Option<String>) -> CsfloatAutobuy {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(api_key.as_str()).unwrap(),
        );

        let mut client = reqwest::Client::builder();

        if let Some(proxy_value) = proxy {
            client = client
                .proxy(Proxy::http(proxy_value).expect("Failed to parse proxy provided from env!"))
        }

        let client = client
            // .proxy(proxy)
            .timeout(Duration::from_secs(10))
            .default_headers(headers)
            .build()
            .expect("Failed to build client for csfloat autobuy");

        CsfloatAutobuy {
            // api_key,
            next_call: Utc::now(),
            client,
        }
    }

    pub async fn buy_listing(
        &mut self,
        listing_id: &ListingId,
        price: PriceValue,
    ) -> Result<bool, reqwest::Error> {
        const BUY_NEXT_CALL: Duration = Duration::from_secs(10);
        let now = Utc::now();
        if self.next_call > now {
            warn!(
                "Locally rate-limited: next call {}  | now {}",
                self.next_call, now
            );
            return Ok(false);
        }

        self.next_call = now + BUY_NEXT_CALL;
        let url = "https://csfloat.com/api/v1/listings/buy";
        let body = serde_json::json!({
            "total_price": price,
            "contract_ids": [listing_id.to_string()]
        });
        // let mut headers = HeaderMap::new();
        // let api_key = env::var("CSFLOAT_API_KEY").expect("CSFLOAT_API_KEY must be set");
        // headers.insert(AUTHORIZATION, HeaderValue::from_str(api_key.as_str()).unwrap());

        // let client = reqwest::Client::builder()
        //     .proxy(Proxy::http(PROXY)?)
        //     .timeout(Duration::from_secs(10))
        //     .build()?;

        let response = self
            .client
            .post(url)
            // .headers(self.headers)
            .json(&body)
            .send()
            .await?;

        // {
        //     let data = response.text().await?;
        //     debug!("Response: {}", data);
        // }

        let response_json: serde_json::Value = response.json().await?;

        Ok(response_json["message"] == "all listings purchased")
    }

    pub async fn get_balance(&mut self) -> Result<PriceValue, reqwest::Error> {
        let url = "https://csfloat.com/api/v1/me";
        let response = self.client.get(url).send().await?;

        let data = response.json::<serde_json::Value>().await?;
        let balance = data["user"]["balance"].as_u64().unwrap_or(0);
        Ok(balance)
    }
}

// it's recommended to use this crate
// https://github.com/lipanski/mockito
