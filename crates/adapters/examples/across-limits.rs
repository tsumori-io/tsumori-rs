use reqwest::blocking::Client;
use serde::Deserialize;
use std::collections::HashMap;

/// https://docs.across.to/integration-guides/across-bridge-integration#checking-limits
/// https://docs.across.to/reference/api#api-endpoints

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct TransferLimitsResponse {
    min_deposit: String,
    max_deposit: String,
    max_deposit_instant: String,
    max_deposit_short_delay: String,
    recommended_deposit_instant: String,
}

fn main() {
    // ORIGIN_CHAIN_ID=42161
    // ORIGIN_TOKEN=0xaf88d065e77c8cC2239327C5EDb3A432268e5831
    // DESTINATION_CHAIN_ID=8453
    // DESTINATION_TOKEN=0xd9aAEc86B65D86f6A7B5B1b0c42FFA531710b6CA
    // curl "https://app.across.to/api/limits?originChainId=${ORIGIN_CHAIN_ID}&destinationChainId=${DESTINATION_CHAIN_ID}&inputToken=${ORIGIN_TOKEN}&outputToken=${DESTINATION_TOKEN}"

    // define query parameters
    let origin_chain_id = 42161; // Arbitrum
    let origin_token = "0xaf88d065e77c8cC2239327C5EDb3A432268e5831"; // USDC
    let destination_chain_id = 8453; // Base
    let destination_token = "0xd9aAEc86B65D86f6A7B5B1b0c42FFA531710b6CA"; // USDCbC Base

    // build the request URL
    let mut params = HashMap::new();
    params.insert("originChainId", origin_chain_id.to_string());
    params.insert("inputToken", origin_token.to_string());
    params.insert("destinationChainId", destination_chain_id.to_string());
    params.insert("outputToken", destination_token.to_string());

    let client = Client::new();
    let response = client
        .get("https://app.across.to/api/limits")
        .query(&params)
        .send()
        .unwrap();

    // Check if the response status is OK (200)
    if response.status().is_success() {
        // Deserialize the JSON response into the struct
        let transfer_limits: TransferLimitsResponse = response.json().unwrap();
        println!("{:#?}", transfer_limits);
    } else {
        eprintln!(
            "Failed to fetch transfer limits. Status: {}",
            response.status()
        );
    }
}
