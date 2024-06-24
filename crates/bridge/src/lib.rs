use core::future::Future;

use alloy::primitives::U256;

pub mod across;
pub mod debridge;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum SupportedProviders {
    Across,
    DeBridge,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum PermitSignature {
    EIP2612(String),
    Permit2(String),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeRequest {
    src_chain_id: u32,
    src_token: String,
    src_caller: String,
    src_amount: U256,
    src_chain_token_in_sender_permit: Option<PermitSignature>,
    dest_chain_id: u32,
    dest_token: String,
    dest_recipient: String,
    dest_amount: Option<U256>,
    // TODO - create/use custom struct based on debridge; to support solana + evm
    calldata: Option<String>,
    // TODO: potentially support simulation - via local anvil fork
    /// simulate flag forces bridge tx to validate the resulting transaction and estimate its gas consumption.
    /// You will find the estimation at tx.gasLimit field of the resulting object.
    /// Important: to estimate a transaction, you are required to provide sender address that will be used to execute this transaction.
    /// It should have enough assets on its balance to cover the amount specified in the src_amount property, and enough native gas token to cover the protocol global fixed fee. (DeBridge)
    /// Caution: if the input token (src_token) is not a native blockchain currency but an ERC-20 token, it is necessary to provide an approve to spend this token by the tx.allowanceTarget contract prior to such estimation
    /// This can be done either by executing increaseAllowance() on-chain or by providing the permit envelope via the srcChainTokenInSenderPermit property. Failing to provide a correct approve to spend will result an error during transaction.
    simulate: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(test, derive(PartialEq))]
pub struct TxData {
    data: String,
    to: String,
    value: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum BridgeAction {
    /// If no approvals are required, the bridging tx can be called directly
    BridgingTx(TxData),
    /// EIP-2612 payload, to be signed for BridgingTx.
    /// If the source token supports EIP-2612, the caller must sign this data, for a bridging tx to be returned
    PermitSignature(String),
    /// Permit2 payload, to be signed for BridgingTx
    /// If source token doesnt support eip-2612 and the bridging contract is not approved,
    /// but the caller has approved the canonical permit2 contract, the caller must sign this data
    Permit2Signature(String),
    /// Permit2 tx, to be called before BridgingTx
    /// If source token doesnt support eip-2612 and the bridging contract is not approved,
    /// and the caller has not approved the canonical permit2 contract, the caller must call this tx
    /// before the bridging tx
    Permit2Tx(TxData),
    /// Bridge approval tx, to be called before BridgingTx
    /// If the bridging contract requires an explicit approval tx, the caller must call this tx
    /// before the bridging tx
    BridgeApprovalTx(SupportedProviders, TxData),
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeResponse {
    provider: SupportedProviders,
    /// Optional approval_tx (either permit2 data to be signed, or explicit approval tx to be called before bridge_tx)
    /// Cross-chain source bridging tx
    bridge_action: BridgeAction,
}

pub trait BridgeProvider {
    fn get_bridging_data(
        &self,
        request: &BridgeRequest,
    ) -> impl Future<Output = eyre::Result<BridgeResponse>> + Send;
}
