use crate::models::{ForwardRequest as ForwardRequestModel};
use anyhow::{Context, Result};
use alloy::{
    providers::Provider,
    signers::local::PrivateKeySigner,
    network::{Ethereum, TransactionBuilder},
    primitives::{Address, B256, U256, Bytes},
    sol,
    sol_types::SolCall,
    rpc::types::eth::{TransactionRequest, TransactionReceipt},
    transports::Transport,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use tracing::{info, warn};

sol! {
    #[sol(rpc)]
    interface IForwarder {
        struct ForwardRequest {
            address from;
            address to;
            uint256 value;
            uint256 gas;
            uint256 nonce;
            bytes data;
        }
        function execute(ForwardRequest calldata req, bytes calldata signature) external payable returns (bool, bytes memory);
    }
}

sol! {
    #[sol(rpc)]
    interface IForwarderDeadline {
        struct ForwardRequestWithDeadline {
            address from;
            address to;
            uint256 value;
            uint256 gas;
            uint256 nonce;
            uint256 deadline;
            bytes data;
        }
        function execute(ForwardRequestWithDeadline calldata req, bytes calldata signature) external payable returns (bool, bytes memory);
    }
}

pub fn encode_execute_call(req: &ForwardRequestModel, signature: &[u8]) -> Result<Bytes> {
    if let Some(deadline) = req.deadline {
        let sol_req = IForwarderDeadline::ForwardRequestWithDeadline {
            from: req.from,
            to: req.to,
            value: req.value,
            gas: req.gas,
            nonce: req.nonce,
            deadline,
            data: req.data.clone().into(),
        };
        let call = IForwarderDeadline::executeCall {
            req: sol_req,
            signature: signature.to_vec().into(),
        };
        Ok(call.abi_encode().into())
    } else {
        let sol_req = IForwarder::ForwardRequest {
            from: req.from,
            to: req.to,
            value: req.value,
            gas: req.gas,
            nonce: req.nonce,
            data: req.data.clone().into(),
        };
        let call = IForwarder::executeCall {
            req: sol_req,
            signature: signature.to_vec().into(),
        };
        Ok(call.abi_encode().into())
    }
}

const FORWARDER_OVERHEAD_GAS: u64 = 45_000;
const GAS_BUFFER_BPS: u64 = 2000;

#[derive(Debug, Clone)]
pub struct RelayExecutorConfig {
    pub forwarder_address: Address,
    pub chain_id: u64,
    pub confirmation_timeout_secs: u64,
    pub confirmations_required: u64,
    pub max_gas_price_gwei: u64,
}

pub struct RelayExecutor<P, T = alloy::transports::BoxTransport> {
    pub provider: Arc<P>,
    signer: PrivateKeySigner,
    config: RelayExecutorConfig,
    _marker: std::marker::PhantomData<T>,
}

impl<P, T> std::fmt::Debug for RelayExecutor<P, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RelayExecutor")
            .field("config", &self.config)
            .field("relayer_address", &self.signer.address())
            .finish()
    }
}

impl<P, T> RelayExecutor<P, T> 
where
    P: Provider<T, Ethereum>,
    T: Transport + Clone,
{
    pub fn new(
        provider: Arc<P>,
        signer: PrivateKeySigner,
        config: RelayExecutorConfig,
    ) -> Self {
        Self { provider, signer, config, _marker: std::marker::PhantomData }
    }

    pub fn relayer_address(&self) -> Address {
        self.signer.address()
    }

    pub async fn submit(
        &self,
        req: &ForwardRequestModel,
        signature: &[u8],
        nonce: u64,
    ) -> Result<B256> {
        let calldata = encode_execute_call(req, signature)?;

        let tx_est = TransactionRequest::default()
            .from(self.signer.address())
            .to(self.config.forwarder_address)
            .input(calldata.clone().into());

        let gas_limit = self.provider
            .estimate_gas(&tx_est)
            .await
            .unwrap_or_else(|e| {
                warn!("gas estimation failed, using fallback: {}", e);
                (U256::from(FORWARDER_OVERHEAD_GAS) + req.gas).to::<u128>()
            });

        let gas_limit_with_buffer = gas_limit * (10000 + GAS_BUFFER_BPS) as u128 / 10000;

        let fees = self.provider
            .estimate_eip1559_fees(None)
            .await
            .context("failed to fetch fee data")?;

        let max_allowed_gwei =
            U256::from(self.config.max_gas_price_gwei) * U256::from(10).pow(U256::from(9));
        anyhow::ensure!(
            U256::from(fees.max_fee_per_gas) <= max_allowed_gwei,
            "gas price too high"
        );

        let tx = TransactionRequest::default()
            .from(self.signer.address())
            .to(self.config.forwarder_address)
            .input(calldata.into())
            .gas_limit(gas_limit_with_buffer)
            .max_fee_per_gas(fees.max_fee_per_gas)
            .max_priority_fee_per_gas(fees.max_priority_fee_per_gas)
            .nonce(nonce)
            .value(req.value)
            .with_chain_id(self.config.chain_id);

        let pending = self.provider
            .send_transaction(tx)
            .await
            .context("failed to submit transaction")?;

        let tx_hash = *pending.tx_hash();
        info!(tx_hash = ?tx_hash, "transaction submitted");

        Ok(tx_hash)
    }

    pub async fn wait_for_confirmation(
        &self,
        tx_hash: B256,
    ) -> Result<TransactionReceipt> {
        let timeout_duration =
            Duration::from_secs(self.config.confirmation_timeout_secs);

        let receipt = timeout(timeout_duration, async {
            self.provider
                .get_transaction_receipt(tx_hash)
                .await?
                .ok_or_else(|| anyhow::anyhow!("transaction not found"))
        })
        .await
        .context("confirmation timeout")?
        .context("failed to get receipt")?;

        if !receipt.status() {
            anyhow::bail!("transaction reverted on-chain");
        }

        Ok(receipt)
    }
}
