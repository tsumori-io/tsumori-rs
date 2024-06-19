use reqwest::blocking::Client;

/// https://docs.dln.trade/dln-api/quick-start-guide/requesting-order-creation-transaction
/// https://api.dln.trade/v1.0/#/DLN/DlnOrderControllerV10_createOrder

fn main() {
    let query_params = bridge::debridge::CreateTxQueryParams {
        src_chain_id: 42161,                                              // Arbitrum
        src_chain_token_in: "0xaf88d065e77c8cC2239327C5EDb3A432268e5831", // USDC
        src_chain_token_in_amount: "4000000".into(),                      // 4 USDC
        src_chain_token_in_sender_permit: None,
        dst_chain_id: 8453,                                                // Base
        dst_chain_token_out: "0xd9aAEc86B65D86f6A7B5B1b0c42FFA531710b6CA", // USDCbC Base
        dst_chain_token_out_recipient: "0xD79842424f797feF2B713BAd555eDdD0b6c89a80", // recipient
        dst_chain_token_out_amount: None,
        src_chain_order_authority_address: "0xD79842424f797feF2B713BAd555eDdD0b6c89a80", // same EOA account has authority on both chains
        dst_chain_order_authority_address: "0xD79842424f797feF2B713BAd555eDdD0b6c89a80", // same EOA account has authority on both chains
        external_call: None,
    };

    let client = Client::new();
    let response = client
        .get("https://api.dln.trade/v1.0/dln/order/create-tx")
        .query(&query_params)
        .send()
        .unwrap();

    // Check if the response status is OK (200)
    if response.status().is_success() {
        // Deserialize the JSON response into the struct
        let create_tx_response: bridge::debridge::CreateTxResponse = response.json().unwrap();
        println!("{:#?}", create_tx_response);
    } else {
        println!("Failed to fetch create-tx. Status: {}", response.status());
    }
}
