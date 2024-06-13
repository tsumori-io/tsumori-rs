use reqwest::blocking::Client;
use serde::Deserialize;

/// https://docs.across.to/integration-guides/across-bridge-integration#checking-limits
/// https://docs.across.to/reference/api#api-endpoints

#[derive(Debug, serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct LimitQueryParams<'a> {
    origin_chain_id: u32,
    input_token: &'a str,
    destination_chain_id: u32,
    output_token: &'a str,
}

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

    let params = LimitQueryParams {
        origin_chain_id: utils::Chain::Arbitrum as u32,
        input_token: "0xaf88d065e77c8cC2239327C5EDb3A432268e5831", // USDC
        destination_chain_id: utils::Chain::Base as u32,
        output_token: "0xd9aAEc86B65D86f6A7B5B1b0c42FFA531710b6CA", // USDCbC Base
    };

    let client = Client::new();
    let response = client
        .get("https://app.across.to/api/limits")
        .query(&params)
        .send()
        .unwrap();

    if response.status().is_success() {
        // deserialize the JSON response into the struct
        let transfer_limits: TransferLimitsResponse = response.json().unwrap();
        println!("{:#?}", transfer_limits);
    } else {
        eprintln!(
            "Failed to fetch transfer limits. Status: {}",
            response.status()
        );
    }
}
