use std::collections::HashMap;
use std::sync::OnceLock;

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
