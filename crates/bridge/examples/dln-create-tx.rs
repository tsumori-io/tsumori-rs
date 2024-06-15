use reqwest::blocking::Client;
use serde::Deserialize;
use std::collections::HashMap;

/// https://docs.dln.trade/dln-api/quick-start-guide/requesting-order-creation-transaction
/// https://api.dln.trade/v1.0/#/DLN/DlnOrderControllerV10_createOrder

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct TokenInfo {
    address: String,
    chain_id: u32,
    decimals: u8,
    name: String,
    symbol: String,
    amount: String,
    recommended_amount: Option<String>,
    max_theoretical_amount: Option<String>,
    approximate_operating_expense: Option<String>,
    mutated_with_operating_expense: Option<bool>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct CostDetails {
    chain: String,
    token_in: String,
    token_out: String,
    amount_in: String,
    amount_out: String,
    #[serde(rename = "type")]
    cost_type: String,
    payload: HashMap<String, String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Estimation {
    src_chain_token_in: TokenInfo,
    dst_chain_token_out: TokenInfo,
    costs_details: Vec<CostDetails>,
    recommended_slippage: f64,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct TxData {
    data: String,
    to: String,
    value: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Order {
    approximate_fulfillment_delay: u32,
    salt: u64,
    metadata: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct CreateTxResponse {
    estimation: Estimation,
    tx: TxData,
    order: Order,
    order_id: String,
    fix_fee: String,
    user_points: f64,
    integrator_points: f64,
}

fn main() {
    let src_chain_id = 42161; // Arbitrum
    let src_chain_token_in = "0xaf88d065e77c8cC2239327C5EDb3A432268e5831"; // USDC
    let src_chain_token_in_amount = 4_000_000u32; // 4 USDC
    let dst_chain_id = 8453; // Base
    let dst_chain_token_out = "0xd9aAEc86B65D86f6A7B5B1b0c42FFA531710b6CA"; // USDCbC Base

    // let dst_chain_token_out_amount = 99623050_u32; // optional

    let dst_chain_token_out_recipient = "0xD79842424f797feF2B713BAd555eDdD0b6c89a80";

    // same EOA account has authority on both chains
    let src_chain_order_authority_address = "0xD79842424f797feF2B713BAd555eDdD0b6c89a80";
    let dst_chain_order_authority_address = "0xD79842424f797feF2B713BAd555eDdD0b6c89a80";

    // Build the request URL
    let mut params = HashMap::new();
    params.insert("srcChainId", src_chain_id.to_string());
    params.insert("srcChainTokenIn", src_chain_token_in.to_string());
    params.insert(
        "srcChainTokenInAmount",
        src_chain_token_in_amount.to_string(),
    );
    params.insert("dstChainId", dst_chain_id.to_string());
    params.insert("dstChainTokenOut", dst_chain_token_out.to_string());
    // params.insert(
    //     "dstChainTokenOutAmount",
    //     dst_chain_token_out_amount.to_string(),
    // );
    params.insert(
        "dstChainTokenOutRecipient",
        dst_chain_token_out_recipient.to_string(),
    );
    params.insert(
        "srcChainOrderAuthorityAddress",
        src_chain_order_authority_address.to_string(),
    );
    params.insert(
        "dstChainOrderAuthorityAddress",
        dst_chain_order_authority_address.to_string(),
    );

    let client = Client::new();
    let response = client
        .get("https://api.dln.trade/v1.0/dln/order/create-tx")
        .query(&params)
        .send()
        .unwrap();

    // Check if the response status is OK (200)
    if response.status().is_success() {
        // Deserialize the JSON response into the struct
        let create_tx_response: CreateTxResponse = response.json().unwrap();
        println!("{:#?}", create_tx_response);
    } else {
        println!("Failed to fetch create-tx. Status: {}", response.status());
    }
}
