use crate::BridgeProvider;

#[derive(Debug, Clone)]
pub struct BridgeService {
    across: crate::across::AcrossBridge,
    debridge: crate::debridge::DeBridge,
}

impl BridgeService {
    pub fn new() -> Self {
        Self {
            across: crate::across::AcrossBridge::new(),
            debridge: crate::debridge::DeBridge::new(),
        }
    }

    pub fn get_supported_chains(&self) -> Vec<&utils::ChainData> {
        let chain_data: Vec<_> = utils::get_supported_chains()
            .iter()
            .map(|(_, &chaindata)| chaindata)
            .collect();
        chain_data
    }

    pub async fn get_tx(&self, req: &crate::BridgeRequest) -> eyre::Result<crate::BridgeResponse> {
        let _ = utils::get_supported_chains()
            .get(&req.src_chain_id)
            .ok_or_else(|| eyre::eyre!("unsupported source chain: {}", req.src_chain_id))?;

        // if dest chain not in map, return error
        let _ = utils::get_supported_chains()
            .get(&req.dest_chain_id)
            .ok_or_else(|| eyre::eyre!("unsupported dest chain: {}", req.dest_chain_id))?;

        // if src or dest chain is solana; simply use debridge
        if req.src_chain_id == utils::Chain::Solana as u32
            || req.dest_chain_id == utils::Chain::Solana as u32
        {
            return self.debridge.get_bridging_data(req).await;
        }

        // fire off 2 reqeusts in parallel; 1 to across, the other to debridge; run both futures concurrently
        let across_fut = self.across.get_bridging_data(req);
        let debridge_fut = self.debridge.get_bridging_data(req);
        tokio::pin!(across_fut);
        tokio::pin!(debridge_fut);

        // TODO: add logging on the path taken by the futures
        let bridge_result = tokio::select! {
            across_res = &mut across_fut => {
                match across_res {
                  Ok(response) => Ok(response),
                  // across errored, we wait and fallback to debridge response
                  Err(_) => debridge_fut.await,
                }
            },
            debridge_res = &mut debridge_fut => {
                match debridge_res {
                  // debridge finished first, but we wait for across - as it must be prioritised if succcessful
                  Ok(response) => across_fut.await.or(Ok(response)),
                  // debridge errored, we still wait for across
                  Err(_) => across_fut.await.or_else(|e| Err(e)),
                }
            },
        };
        bridge_result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn get_tx_across_bridging_tx_approved_sender() {
        let bridge = BridgeService::new();
        let request = crate::BridgeRequest {
            src_chain_id: utils::Chain::Base as u32, // Base
            src_token: "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913".into(), // USDC Base
            src_caller: "0x000007357111E4789005d4eBfF401a18D99770cE".into(), // caller is recipient
            src_amount: crate::U256::from(2000_000u32), // 2 USDC
            src_chain_token_in_sender_permit: None,
            dest_chain_id: utils::Chain::Arbitrum as u32, // Arbitrum
            dest_token: "0xaf88d065e77c8cC2239327C5EDb3A432268e5831".into(), // USDC Arbitrum
            dest_recipient: "0x000007357111E4789005d4eBfF401a18D99770cE".into(), // recipient
            dest_amount: None,
            calldata: None,
            simulate: false,
        };
        let response = bridge.get_tx(&request).await;
        assert!(response.is_ok());
        let response = response.unwrap();

        assert_eq!(response.provider, crate::SupportedProviders::Across);
        assert!(matches!(
            response.bridge_action,
            crate::BridgeAction::BridgingTx(_)
        ));

        // println!("{:#?}", response);
        // assert!(false);
    }

    #[tokio::test]
    async fn get_tx_across_bridging_tx_non_approved_sender() {
        let bridge = BridgeService::new();
        let request = crate::BridgeRequest {
            src_chain_id: utils::Chain::Base as u32, // Base
            src_token: "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913".into(), // USDC Base
            src_caller: "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913".into(), // caller is random account
            src_amount: crate::U256::from(2000_000u32),                      // 2 USDC
            src_chain_token_in_sender_permit: None,
            dest_chain_id: utils::Chain::Arbitrum as u32, // Arbitrum
            dest_token: "0xaf88d065e77c8cC2239327C5EDb3A432268e5831".into(), // USDC Arbitrum
            dest_recipient: "0x000007357111E4789005d4eBfF401a18D99770cE".into(), // recipient
            dest_amount: None,
            calldata: None,
            simulate: false,
        };
        let response = bridge.get_tx(&request).await;
        assert!(response.is_ok());
        let response = response.unwrap();

        assert_eq!(response.provider, crate::SupportedProviders::Across);
        assert!(matches!(
            response.bridge_action,
            crate::BridgeAction::BridgeApprovalTx(_, _),
        ));

        // println!("{:#?}", response);
        // assert!(false);
    }

    // TODO: permit2 test for across, return a permit2 sig for a token which doesnt support eip-2612
    // TODO: sign sig request with signer, provide this in next http req, get executable bridging tx

    // TODO: permit test for across; look for a token with eip-2612 support which can be bridged using across, require signature for it
    // TODO: sign sig request with signer, provide this in next http req, get executable bridging tx

    #[tokio::test]
    async fn get_tx_debridge_bridging_tx_approved_sender() {
        let bridge = BridgeService::new();
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
        let response = bridge.get_tx(&request).await;
        assert!(response.is_ok());
        let response = response.unwrap();

        assert_eq!(response.provider, crate::SupportedProviders::DeBridge);
        assert!(matches!(
            response.bridge_action,
            crate::BridgeAction::BridgingTx(_),
        ));

        // println!("{:#?}", response);
        // assert!(false);
    }

    // TODO: permit2 test for debridge, return a permit2 sig for a token which doesnt support eip-2612
    // TODO: sign sig request with signer, provide this in next http req, get executable bridging tx

    // TODO: permit test for debridge; look for a token with eip-2612 support which can be bridged using debridge, require signature for it
    // TODO: sign sig request with signer, provide this in next http req, get executable bridging tx

    // TODO: validate source tx for sol uses debridge, returns executable tx data

    // TODO: validate dest tx for sol uses debridge, returns executable tx data
}
