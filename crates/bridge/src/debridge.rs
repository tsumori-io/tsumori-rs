use crate::{BridgeRequest, BridgeResponse, TxData};
use std::borrow::Cow;
use std::{collections::HashMap, str::FromStr};

use alloy::providers::{Provider, ProviderBuilder, RootProvider};
use alloy::{primitives::Address, sol};
use eyre::Result;

#[derive(Debug, serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CreateTxQueryParams<'a> {
    pub src_chain_id: u32,
    pub src_chain_token_in: &'a str,
    pub src_chain_token_in_amount: Cow<'a, str>,
    pub src_chain_token_in_sender_permit: Option<&'a str>,
    pub dst_chain_id: u32,
    pub dst_chain_token_out: &'a str,
    pub dst_chain_token_out_recipient: &'a str,
    pub dst_chain_token_out_amount: Option<Cow<'a, str>>,
    pub src_chain_order_authority_address: &'a str,
    pub dst_chain_order_authority_address: &'a str,
    pub external_call: Option<&'a String>,
}

impl<'a> From<&'a crate::BridgeRequest> for CreateTxQueryParams<'a> {
    fn from(request: &'a crate::BridgeRequest) -> Self {
        let src_chain_token_in_amount = Cow::Owned(request.src_amount.to_string());
        Self {
            src_chain_id: request.src_chain_id,
            src_chain_token_in: request.src_token.as_str(),
            src_chain_token_in_amount,
            src_chain_token_in_sender_permit: request
                .src_chain_token_in_sender_permit
                .as_ref()
                .map(|permit| match permit {
                    crate::PermitSignature::EIP2612(data) => data.as_str(),
                    crate::PermitSignature::Permit2(data) => data.as_str(),
                }),
            dst_chain_id: request.dest_chain_id,
            dst_chain_token_out: request.dest_token.as_str(),
            dst_chain_token_out_recipient: request.dest_recipient.as_str(),
            dst_chain_token_out_amount: request
                .dest_amount
                .map(|amount| Cow::Owned(amount.to_string())),
            src_chain_order_authority_address: request.src_caller.as_str(),
            dst_chain_order_authority_address: request.dest_recipient.as_str(),
            external_call: request.calldata.as_ref(),
        }
    }
}

#[derive(serde::Deserialize, Debug)]
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
    max_refund_amount: Option<String>,
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct CostDetails {
    chain: String,
    token_in: String,
    token_out: String,
    amount_in: String,
    amount_out: String,
    #[serde(rename = "type")]
    cost_type: String,
    payload: Option<HashMap<String, String>>,
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Estimation {
    src_chain_token_in: TokenInfo,
    src_chain_token_out: Option<TokenInfo>,
    costs_details: Vec<CostDetails>,
    recommended_slippage: f64,
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Order {
    approximate_fulfillment_delay: u32,
    salt: u64,
    metadata: String,
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CreateTxResponse {
    estimation: Estimation,
    tx: TxData,
    order: Order,
    order_id: String,
    fix_fee: String,
    user_points: f64,
    integrator_points: f64,
}

#[derive(Debug, Clone)]
pub struct DeBridge {
    client: reqwest::Client,
    providers: HashMap<u32, RootProvider<alloy::transports::http::Http<reqwest::Client>>>,
}

impl DeBridge {
    pub fn new() -> Self {
        let supported_providers = utils::get_supported_chains()
            .iter()
            .map(|(id, chain)| {
                let rpc_url = reqwest::Url::parse(chain.rpc_url).unwrap(); // infallible
                let provider = ProviderBuilder::new().on_http(rpc_url);
                (*id, provider)
            })
            .collect::<HashMap<_, _>>();

        Self {
            client: reqwest::Client::new(),
            providers: supported_providers,
        }
    }

    pub async fn get_create_tx(
        &self,
        params: &CreateTxQueryParams<'_>,
    ) -> Result<CreateTxResponse> {
        let response = self
            .client
            .get("https://api.dln.trade/v1.0/dln/order/create-tx")
            .query(&params)
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(eyre::eyre!(
                "DeBridge: failed to get create-tx: {}",
                response.text().await?
            ));
        }
        Ok(response.json().await?)
    }
}

impl crate::BridgeProvider for DeBridge {
    async fn get_bridging_data(
        &self,
        request: &crate::BridgeRequest,
    ) -> eyre::Result<crate::BridgeResponse> {
        let params = request.into();
        let response = self.get_create_tx(&params).await?;

        // TODO: validate for source chain as solana
        // if source chain is solana, explicit approval will not be required
        if request.src_chain_id == utils::Chain::Solana as u32 {
            return Ok(crate::BridgeResponse {
                provider: crate::SupportedProviders::DeBridge,
                bridge_action: crate::BridgeAction::BridgingTx(response.tx),
            });
        }

        // source chain must be evm compatible; perform approval checks...
        let provider = self
            .providers
            .get(&request.src_chain_id)
            .ok_or_else(|| eyre::eyre!("unsupported chain id: {}", request.src_chain_id))?;

        // TODO: this would only apply if source chain is EVM
        let allowance_action = utils::get_token_allowance_action(
            provider,
            &Address::from_str(&request.src_token)?,
            &request.src_amount,
            &Address::from_str(&request.src_caller)?,
            &Address::from_str(&response.tx.to)?,
        )
        .await?;

        // if there is an pre-allowance tx/sig/action required, it must be returned
        // to be executed by the caller
        if !matches!(allowance_action, utils::AllowanceAction::Ok) {
            // TODO: convert allowance action as-is, into BridgeResponse
            // TODO: impl From<AllowanceAction> for BridgeResponse
        };

        Ok(crate::BridgeResponse {
            provider: crate::SupportedProviders::DeBridge,
            bridge_action: crate::BridgeAction::BridgingTx(response.tx),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BridgeProvider;
    use crate::U256;

    #[tokio::test]
    async fn get_create_tx() {
        let debridge = DeBridge::new();
        let params = CreateTxQueryParams {
            src_chain_id: 42161,                                              // Arbitrum
            src_chain_token_in: "0xaf88d065e77c8cC2239327C5EDb3A432268e5831", // USDC
            src_chain_token_in_amount: "4000000".into(),                      // 4 USDC
            src_chain_token_in_sender_permit: None,
            dst_chain_id: 8453, // USDCbC Base
            dst_chain_token_out: "0xd9aAEc86B65D86f6A7B5B1b0c42FFA531710b6CA", // USDCbC Base
            dst_chain_token_out_recipient: "0xD79842424f797feF2B713BAd555eDdD0b6c89a80",
            dst_chain_token_out_amount: None,
            src_chain_order_authority_address: "0xD79842424f797feF2B713BAd555eDdD0b6c89a80",
            dst_chain_order_authority_address: "0xD79842424f797feF2B713BAd555eDdD0b6c89a80",
            external_call: None,
        };
        let response = debridge.get_create_tx(&params).await;
        assert!(response.is_ok());
        println!("{:?}", response);
        // assert!(false);
    }

    #[tokio::test]
    #[ignore]
    async fn get_bridging_data_no_inner_calldata() {
        let debridge = DeBridge::new();
        let request = crate::BridgeRequest {
            src_caller: "0x000007357111E4789005d4eBfF401a18D99770cE".into(),
            src_chain_id: utils::Chain::Base as u32, // Base
            src_token: "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913".into(), // USDC Base
            src_chain_token_in_sender_permit: None,
            src_amount: U256::from(2000_000u32), // 4 USDC
            dest_chain_id: utils::Chain::Arbitrum as u32, // Arbitrum
            dest_token: "0xaf88d065e77c8cC2239327C5EDb3A432268e5831".into(), // USDC Arbitrum
            dest_recipient: "0x000007357111E4789005d4eBfF401a18D99770cE".into(), // recipient
            dest_amount: None,
            calldata: None,
            simulate: false,
        };
        let response = debridge.get_bridging_data(&request).await;
        assert!(response.is_ok());
        println!("{:?}", response);
        // assert!(false);
    }

    #[tokio::test]
    async fn get_create_tx_not_across_supported() {
        let debridge = DeBridge::new();
        let request = crate::BridgeRequest {
            src_chain_id: utils::Chain::Base as u32, // Base
            src_token: "0xd9aAEc86B65D86f6A7B5B1b0c42FFA531710b6CA".into(), // USDCbC Base // NOTE: not supported by across
            src_caller: "0x000007357111E4789005d4eBfF401a18D99770cE".into(), // caller is recipient
            src_amount: crate::U256::from(2000_000u32),                     // 2 USDC
            src_chain_token_in_sender_permit: None,
            dest_chain_id: utils::Chain::Arbitrum as u32, // Arbitrum
            dest_token: "0xaf88d065e77c8cC2239327C5EDb3A432268e5831".into(), // USDC Arbitrum
            dest_recipient: "0x000007357111E4789005d4eBfF401a18D99770cE".into(), // recipient
            dest_amount: None,
            calldata: None,
            simulate: false,
        };
        let response = debridge.get_bridging_data(&request).await;
        assert!(response.is_ok());
        println!("{:?}", response);
        // assert!(false);
    }
}
