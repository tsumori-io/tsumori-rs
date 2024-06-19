use crate::U256;
use alloy::providers::{Provider, ProviderBuilder, RootProvider};
use alloy::sol_types::SolCall;
use alloy::{primitives::Address, sol};
use hex::FromHex;
use hex_literal::hex;
use serde::Deserialize;
use std::{collections::HashMap, str::FromStr};

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

#[derive(Debug, serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LimitQueryParams<'a> {
    pub origin_chain_id: u32,
    pub input_token: &'a str,
    pub destination_chain_id: u32,
    pub output_token: &'a str,
}

impl<'a> From<&'a QuoteQueryParams<'a>> for LimitQueryParams<'a> {
    fn from(query_params: &'a QuoteQueryParams<'a>) -> Self {
        Self {
            origin_chain_id: query_params.origin_chain_id,
            input_token: query_params.input_token,
            destination_chain_id: query_params.destination_chain_id,
            output_token: query_params.output_token,
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TransferLimitsResponse {
    min_deposit: String,
    max_deposit: String,
    max_deposit_instant: String,
    max_deposit_short_delay: String,
    recommended_deposit_instant: String,
}

#[derive(Debug, serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct QuoteQueryParams<'a> {
    pub origin_chain_id: u32,
    pub input_token: &'a str,
    pub destination_chain_id: u32,
    pub output_token: &'a str,
    pub recipient: &'a str,
    pub amount: U256,
}

impl<'a> From<&'a crate::BridgeRequest> for QuoteQueryParams<'a> {
    fn from(request: &'a crate::BridgeRequest) -> Self {
        Self {
            origin_chain_id: request.src_chain_id,
            input_token: request.src_token.as_str(),
            destination_chain_id: request.dest_chain_id,
            output_token: request.dest_token.as_str(),
            recipient: request.dest_recipient.as_str(),
            amount: request.src_amount,
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct FeeDetails {
    pct: String,
    pub total: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SuggestedFeesResponse {
    capital_fee_pct: String,
    capital_fee_total: String,
    relay_gas_fee_pct: String,
    relay_gas_fee_total: String,
    relay_fee_pct: String,
    relay_fee_total: String,
    lp_fee_pct: String,
    pub timestamp: String,
    is_amount_too_low: bool,
    quote_block: String,
    pub spoke_pool_address: String,
    pub total_relay_fee: FeeDetails,
    relayer_capital_fee: FeeDetails,
    relayer_gas_fee: FeeDetails,
    lp_fee: FeeDetails,
}

#[derive(Debug, Clone)]
pub struct AcrossBridge {
    client: reqwest::Client,
    providers: HashMap<u32, RootProvider<alloy::transports::http::Http<reqwest::Client>>>,
}

impl AcrossBridge {
    pub fn new() -> Self {
        let supported_providers = utils::get_supported_chains()
            .iter()
            .map(|(id, chain)| {
                let rpc_url = reqwest::Url::parse(chain.rpc_url).unwrap();
                let provider = ProviderBuilder::new().on_http(rpc_url);
                (*id, provider)
            })
            .collect::<HashMap<_, _>>();

        Self {
            client: reqwest::Client::new(),
            providers: supported_providers,
        }
    }

    pub async fn get_transfer_limits(
        &self,
        params: &LimitQueryParams<'_>,
    ) -> Result<TransferLimitsResponse, reqwest::Error> {
        let url = "https://app.across.to/api/limits";
        let response = self.client.get(url).query(params).send().await.unwrap();
        response.json().await
    }

    pub async fn get_suggested_fees(
        &self,
        params: &QuoteQueryParams<'_>,
    ) -> Result<SuggestedFeesResponse, reqwest::Error> {
        let url = "https://app.across.to/api/suggested-fees";
        let response = self.client.get(url).query(params).send().await.unwrap();
        response.json().await
    }

    async fn get_latest_block_timestamp(&self, chain_id: u32) -> Result<u64, reqwest::Error> {
        let provider = self.providers.get(&chain_id).unwrap();
        let latest_block_number = provider.get_block_number().await.unwrap();
        let latest_block = provider
            .get_block_by_number(latest_block_number.into(), false)
            .await
            .unwrap()
            .unwrap();
        Ok(latest_block.header.timestamp)
    }

    fn get_tx_calldata<'a>(
        caller: &'a str,
        query_params: &'_ QuoteQueryParams<'_>,
        fees_response_timestamp: u32,
        fees_response_total_relay_fee: U256,
        block_timestamp: u64,
        calldata: Option<&'a str>,
    ) -> String {
        let calldata = depositV3Call {
            depositor: Address::from_str(query_params.recipient).unwrap(), // depositor is recipient
            recipient: Address::from_str(query_params.recipient).unwrap(),
            inputToken: Address::from_str(query_params.input_token).unwrap(),
            outputToken: Address::from_str(query_params.output_token).unwrap(),
            inputAmount: U256::from(query_params.amount),
            outputAmount: query_params
                .amount
                .checked_sub(fees_response_total_relay_fee)
                .unwrap(),
            destinationChainId: U256::from(query_params.destination_chain_id),
            exclusiveRelayer: hex!("0000000000000000000000000000000000000000").into(),
            quoteTimestamp: fees_response_timestamp,
            // block.timestamp + 21600, // fillDeadline: We reccomend a fill deadline of 6 hours out. The contract will reject this if it is beyond 8 hours from now.
            fillDeadline: block_timestamp.saturating_add(60 * 2) as u32, // 120s
            exclusivityDeadline: 0,
            message: calldata
                .map(|data| Vec::from_hex(data).unwrap_or_default())
                .unwrap_or_default()
                .into(),
        };
        let data = hex::encode(calldata.abi_encode());
        data
    }
}

impl crate::BridgeProvider for AcrossBridge {
    async fn get_bridging_data(
        &self,
        request: &crate::BridgeRequest,
    ) -> Result<crate::BridgeResponse, Box<dyn std::error::Error>> {
        let query_params: QuoteQueryParams = request.into();
        let limits_query_params: LimitQueryParams = (&query_params).into();

        // TODO: do the below in parallel
        let fees_response_fut = self.get_suggested_fees(&query_params);
        let limits_response_fut = self.get_transfer_limits(&limits_query_params);
        let block_timestamp_fut = self.get_latest_block_timestamp(request.src_chain_id);

        // parallel requests to get fee response, limits and latest block timestamp
        let (fees_response, limits_response, block_timestamp) = {
            let (fees_response, limits_response, block_timestamp) =
                tokio::join!(fees_response_fut, limits_response_fut, block_timestamp_fut);
            (fees_response?, limits_response?, block_timestamp?)
        };

        if request.src_amount > U256::from_str(&limits_response.max_deposit)? {
            return Err("requested amount exceeds max deposit limit".into());
        }

        if let Some(dest_amount) = request.dest_amount {
            let dest_output_amount = query_params
                .amount
                .checked_sub(fees_response.total_relay_fee.total.parse::<U256>().unwrap())
                .unwrap();
            if dest_amount < dest_output_amount {
                return Err("requested dest amount is less than output amount".into());
            }
        }

        let calldata = Self::get_tx_calldata(
            request.src_caller.as_str(),
            &query_params,
            fees_response.timestamp.parse().unwrap(),
            fees_response.total_relay_fee.total.parse::<U256>().unwrap(),
            block_timestamp,
            // request.calldata.as_ref().map(|tx| tx.data.as_str()),
            "".into(),
        );

        // if let Some(caller) = request.src_caller {
        //     // TODO: check if caller has approval
        //     // TODO: do this in parallel
        // }

        Ok(crate::BridgeResponse {
            provider: crate::SupportedProviders::Across,
            bridge_action: crate::BridgeAction::BridgingTx(crate::TxData {
                data: calldata,
                to: fees_response.spoke_pool_address,
                value: "0".to_string(),
                // value: request
                //     .calldata
                //     .as_ref()
                //     .map(|tx| tx.value.clone())
                //     .unwrap_or("0".to_string()),
            }),
        })
    }
}
