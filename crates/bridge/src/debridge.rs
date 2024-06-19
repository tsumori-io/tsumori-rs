use crate::{BridgeRequest, BridgeResponse, TxData};
use std::borrow::Cow;
use std::{collections::HashMap, str::FromStr};

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
    payload: HashMap<String, String>,
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Estimation {
    src_chain_token_in: TokenInfo,
    dst_chain_token_out: TokenInfo,
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
}

impl DeBridge {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    pub async fn get_create_tx(
        &self,
        params: &CreateTxQueryParams<'_>,
    ) -> Result<CreateTxResponse, reqwest::Error> {
        let response = self
            .client
            .get("https://api.dln.trade/v1.0/dln/order/create-tx")
            .query(&params)
            .send()
            .await
            .unwrap();
        response.json().await
    }
}

impl crate::BridgeProvider for DeBridge {
    async fn get_bridging_data(
        &self,
        request: &crate::BridgeRequest,
    ) -> Result<crate::BridgeResponse, Box<dyn std::error::Error>> {
        let params = request.into();
        let response = self.get_create_tx(&params).await?;
        Ok(crate::BridgeResponse {
            provider: crate::SupportedProviders::DeBridge,
            bridge_action: crate::BridgeAction::BridgingTx(response.tx),
        })
    }
}
