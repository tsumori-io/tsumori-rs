use serde::Deserialize;
use std::{collections::HashMap, str::FromStr};

use alloy_primitives::{Address, U256};
use alloy_provider::{network::AnyNetwork, Provider, ProviderBuilder};
use alloy_sol_types::{sol, sol_data::Uint, SolCall, SolType};
use hex_literal::hex;

sol! {
    #[derive(Debug)]
    function depositV3(
        address depositor,
        address recipient,
        address inputToken,
        address outputToken,
        uint256 inputAmount,
        uint256 outputAmount,
        uint256 destinationChainId,
        address exclusiveRelayer,
        uint32 quoteTimestamp,
        uint32 fillDeadline,
        uint32 exclusivityDeadline,
        bytes calldata message
    ) external;
}

/// https://docs.across.to/integration-guides/across-bridge-integration#getting-a-quote
/// https://docs.across.to/reference/api#api-endpoints

#[derive(Debug, serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct QuoteQueryParams<'a> {
    origin_chain_id: u32,
    input_token: &'a str,
    destination_chain_id: u32,
    output_token: &'a str,
    recipient: &'a str,
    amount: U256,
}

#[derive(Deserialize, Debug)]
struct FeeDetails {
    pct: String,
    total: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct SuggestedFeesResponse {
    capital_fee_pct: String,
    capital_fee_total: String,
    relay_gas_fee_pct: String,
    relay_gas_fee_total: String,
    relay_fee_pct: String,
    relay_fee_total: String,
    lp_fee_pct: String,
    timestamp: String,
    is_amount_too_low: bool,
    quote_block: String,
    spoke_pool_address: String,
    total_relay_fee: FeeDetails,
    relayer_capital_fee: FeeDetails,
    relayer_gas_fee: FeeDetails,
    lp_fee: FeeDetails,
}

#[tokio::main]
async fn main() {
    // ORIGIN_CHAIN_ID=8453
    // ORIGIN_TOKEN=0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913
    // DESTINATION_CHAIN_ID=42161
    // DESTINATION_TOKEN=0xFF970A61A04b1cA14834A43f5dE4533eBDDB5CC8
    // RECIPIENT=0xD79842424f797feF2B713BAd555eDdD0b6c89a80
    // AMOUNT=4000000
    // curl "https://app.across.to/api/suggested-fees?originChainId=${ORIGIN_CHAIN_ID}&inputToken=${ORIGIN_TOKEN}&destinationChainId=${DESTINATION_CHAIN_ID}&outputToken=${DESTINATION_TOKEN}&amount=${AMOUNT}&recipient=${RECIPIENT}"

    // define query parameters
    let query_params = QuoteQueryParams {
        origin_chain_id: utils::Chain::Base as u32, // Base
        input_token: "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913", // USDC Base
        // origin_token: &hex!("d9aAEc86B65D86f6A7B5B1b0c42FFA531710b6CA"), // USDCbC Base
        destination_chain_id: utils::Chain::Arbitrum as u32, // Arbitrum
        output_token: "0xaf88d065e77c8cC2239327C5EDb3A432268e5831", // USDC Arbitrum
        // destination_token: &hex!("FF970A61A04b1cA14834A43f5dE4533eBDDB5CC8"), // USDC.e
        recipient: "0x000007357111E4789005d4eBfF401a18D99770cE", // recipient
        amount: U256::from(2000_000u32),                         // 4 USDC
    };

    // let client = reqwest::blocking::Client::new();
    let client = reqwest::Client::new();
    let fee_response_fut = client
        .get("https://app.across.to/api/suggested-fees")
        .query(&query_params)
        .send();
    let block_timestamp_fut = get_latest_block_timestamp(&query_params.origin_chain_id);

    // parallel request to get fee response and latest block timestamp
    let (fee_response, latest_block_timestamp) =
        tokio::join!(fee_response_fut, block_timestamp_fut);
    let fee_response = fee_response.unwrap();

    let suggested_fees: SuggestedFeesResponse = if fee_response.status().is_success() {
        fee_response.json().await.unwrap()
    } else {
        panic!(
            "failed to fetch suggested fees. Status: {}",
            fee_response.status()
        );
    };
    println!("{:#?}", suggested_fees);

    let calldata = get_tx_calldata(&query_params, &suggested_fees, latest_block_timestamp);
    println!(
        "address: {:?}",
        Address::from_str(&suggested_fees.spoke_pool_address).unwrap()
    );
    println!("calldata: 0x{}", calldata);
}

fn get_tx_calldata<'a>(
    query_params: &'_ QuoteQueryParams<'_>,
    fees_response: &'_ SuggestedFeesResponse,
    block_timestamp: u64,
) -> String {
    let calldata = depositV3Call {
        depositor: Address::from_str(query_params.recipient).unwrap(), // depositor is recipient
        recipient: Address::from_str(query_params.recipient).unwrap(),
        inputToken: Address::from_str(query_params.input_token).unwrap(),
        outputToken: Address::from_str(query_params.output_token).unwrap(),
        inputAmount: U256::from(query_params.amount),
        outputAmount: query_params
            .amount
            .checked_sub(fees_response.total_relay_fee.total.parse::<U256>().unwrap())
            .unwrap(),
        destinationChainId: U256::from(query_params.destination_chain_id),
        exclusiveRelayer: hex!("0000000000000000000000000000000000000000").into(),
        quoteTimestamp: fees_response.timestamp.parse().unwrap(),
        // block.timestamp + 21600, // fillDeadline: We reccomend a fill deadline of 6 hours out. The contract will reject this if it is beyond 8 hours from now.
        fillDeadline: block_timestamp.saturating_add(60 * 2) as u32, // 120s
        exclusivityDeadline: 0,
        message: hex!("").into(),
    };
    let data = hex::encode(calldata.abi_encode());
    data
}

async fn get_latest_block_timestamp(chain_id: &u32) -> u64 {
    let src_chain_data = utils::get_supported_chains().get(chain_id).unwrap();
    let provider: alloy_provider::RootProvider<alloy_transport::BoxTransport, AnyNetwork> =
        ProviderBuilder::<_, _, AnyNetwork>::default()
            .on_builtin(src_chain_data.rpc_url)
            .await
            .unwrap();
    let latest_block_number = provider.get_block_number().await.unwrap();
    let latest_block = provider
        .get_block_by_number(latest_block_number.into(), false)
        .await
        .unwrap()
        .unwrap();
    latest_block.header.timestamp
}
