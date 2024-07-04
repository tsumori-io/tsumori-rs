use std::{collections::HashMap, ops::Add, str::FromStr, sync::OnceLock};

use alloy::{
    hex::ToHexExt,
    primitives::{Address, Bytes, U256},
    providers::{network::TransactionBuilder, Provider, ProviderBuilder, RootProvider},
    rpc::types::TransactionRequest,
    sol,
    sol_types::SolCall,
};

sol! {
    // check approval for caller
    function allowance(address owner, address spender) external view returns (uint256);

    // EIP-2612 permit function signature
    function permit(
        address owner,
        address spender,
        uint256 value,
        uint256 deadline,
        uint8 v,
        bytes32 r,
        bytes32 s
    ) external;

    function approve(address spender, uint256 amount) external returns (bool);
}

static CHAINS: OnceLock<HashMap<u32, &'static ChainData>> = OnceLock::new();

pub fn get_supported_chains() -> &'static HashMap<u32, &'static ChainData> {
    CHAINS.get_or_init(|| {
        let mut map = HashMap::new();
        for chain in [
            Chain::Ethereum,
            Chain::Arbitrum,
            Chain::Base,
            Chain::Solana,
            // Add new chains here
        ] {
            let data: &ChainData = chain.into();
            map.insert(data.id, data);
        }
        map
    })
}

#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct ChainData {
    pub id: u32,
    pub name: &'static str,
    pub rpc_url: &'static str,
}

#[derive(Debug)]
pub enum Chain {
    Ethereum = 1,
    Arbitrum = 42161,
    Base = 8453,
    Solana = 7565164,
}

impl TryFrom<u32> for Chain {
    type Error = &'static str;

    fn try_from(id: u32) -> Result<Self, Self::Error> {
        match id {
            1 => Ok(Chain::Ethereum),
            42161 => Ok(Chain::Arbitrum),
            8453 => Ok(Chain::Base),
            7565164 => Ok(Chain::Solana),
            _ => Err("Unsupported chain"),
        }
    }
}

impl Into<&'static ChainData> for Chain {
    fn into(self) -> &'static ChainData {
        match self {
            Chain::Ethereum => &ChainData {
                id: Chain::Ethereum as u32,
                name: "Ethereum",
                rpc_url: "https://eth.llamarpc.com",
            },
            Chain::Arbitrum => &ChainData {
                id: Chain::Arbitrum as u32,
                name: "Arbitrum",
                rpc_url: "https://arb1.arbitrum.io/rpc",
            },
            Chain::Base => &ChainData {
                id: Chain::Base as u32,
                name: "Base",
                rpc_url: "https://mainnet.base.org",
            },
            Chain::Solana => &ChainData {
                id: Chain::Solana as u32,
                name: "Solana",
                rpc_url: "https://api.mainnet-beta.solana.com",
            },
        }
    }
}

pub async fn get_token_allowance_action(
    provider: &RootProvider<alloy::transports::http::Http<reqwest::Client>>,
    token_addr: &Address,
    amount: &U256,
    owner: &Address,
    spender: &Address,
) -> eyre::Result<AllowanceAction> {
    // perform allowance check on token contract for caller and spender
    let allowance = get_allowance(provider, token_addr, owner, spender).await?;
    if allowance >= *amount {
        // token has explicit allowance for spender from owner
        return Ok(AllowanceAction::Ok);
    }

    // check if token contract supports eip-2612 (permit)
    // we do this by explicitly checking that deployed bytecode contains permit func selector
    // TODO: cache this for future operations, assume non-metamorphic token contract
    let token_contract_code = provider.get_code_at(*token_addr).latest().await?;
    let supports_permit = token_contract_code
        .encode_hex()
        .contains(&permitCall::SELECTOR.encode_hex());
    if supports_permit {
        // return back the permit signature to by signed by the caller
        // to give explicit permission to sender
        return Ok(AllowanceAction::PermitSignature("TODO".into()));
    }

    // TODO: check if chain contains deployment of canonical permit2 contract
    let permit2_contract_addr = Some("some-addr");
    if let Some(permit2_contract_addr) = permit2_contract_addr {
        let permit2_contract_addr = Address::from_str(permit2_contract_addr)?;
        // check if canonical permit2 contract has atleast the allowance
        let permit2_allowance =
            get_allowance(provider, token_addr, owner, &permit2_contract_addr).await?;
        if permit2_allowance >= *amount {
            // token has explicit allowance for permit2 contract from owner
            // return back permit2 signature to by signed by the caller
            // to give explicit permission to sender
            return Ok(AllowanceAction::Permit2Signature("TODO".into()));
        }

        // construct TxData to give canonical permit2 contract max permissions
        // also provide permit2 signature to caller so that they can sign and provide it
        // in a future http call (save roundtrip to get bridging tx)
        let data: Bytes = approveCall {
            spender: permit2_contract_addr,
            amount: U256::MAX,
        }
        .abi_encode()
        .into();
        let permit2_tx = TxData {
            to: token_addr.to_string(),
            data: data.to_string(),
            value: "0".into(),
        };
        return Ok(AllowanceAction::Permit2Tx(
            permit2_tx,
            "TODO: permit2-signature".to_string(),
        ));
    };

    // at this point, no canonical permit2 exists on-chain.. simply give explicit max-allowance to the spender..
    let data: Bytes = approveCall {
        spender: *spender,
        amount: U256::MAX,
    }
    .abi_encode()
    .into();
    return Ok(AllowanceAction::ApprovalTx(TxData {
        to: token_addr.to_string(),
        data: data.to_string(),
        value: "0".into(),
    }));
}

/// Simply utilizes the provider to call the allowance mapping on an erc20
/// token for a owner and spender
pub async fn get_allowance(
    provider: &RootProvider<alloy::transports::http::Http<reqwest::Client>>,
    token_addr: &Address,
    owner: &Address,
    spender: &Address,
) -> eyre::Result<U256> {
    // perform allowance check on token contract for caller and spender
    let tx = TransactionRequest::default()
        .with_to(*token_addr)
        .with_input::<Bytes>(
            allowanceCall {
                owner: *owner,
                spender: *spender,
            }
            .abi_encode()
            .into(),
        );
    let response = provider.call(&tx).await?;
    Ok(U256::from_str(&response.to_string())?)
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(test, derive(PartialEq))]
pub struct TxData {
    data: String,
    to: String,
    value: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum AllowanceAction {
    /// No approvals or signatures signing is required.
    /// The spender has already been approved the desired/specified amount.
    Ok,
    /// EIP-2612 payload, to be signed for BridgingTx.
    /// If the source token supports EIP-2612, the caller must sign this data, for a bridging tx to be returned.
    PermitSignature(String),
    /// Permit2 payload, to be signed for BridgingTx
    /// If source token doesnt support eip-2612 and the bridging contract is not approved,
    /// but the caller has approved the canonical permit2 contract, the caller must sign this data.
    Permit2Signature(String),
    /// Permit2 tx, to be called before BridgingTx
    /// If source token doesnt support eip-2612 and the bridging contract is not approved,
    /// and the caller has not approved the canonical permit2 contract, the caller must call this tx
    /// before the bridging tx.
    Permit2Tx(TxData, String),
    /// Explicit approval tx to give max allowance for the spender.
    ApprovalTx(TxData),
}

#[cfg(test)]
mod tests {
    use super::*;
    // use alloy::node_bindings::Anvil;

    #[test]
    fn permit_function_selector() {
        // cast sig "permit(address,address,uint256,uint256,uint8,bytes32,bytes32)"
        assert_eq!(permitCall::SELECTOR.encode_hex(), "d505accf");
    }

    // #[tokio::test]
    // async fn get_token_allowance_action() {
    //     Anvil::new()
    // }
}
