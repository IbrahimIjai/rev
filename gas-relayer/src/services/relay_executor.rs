use crate::models::ForwardRequest as ForwardRequestModel;
use anyhow::{Context, Result};
use alloy::{
    consensus::{SignableTransaction, TxEip1559, TxEnvelope},
    eips::eip2718::Encodable2718,
    network::{Ethereum, TransactionBuilder},
    primitives::{Address, Bytes, TxKind, B256, U256},
    providers::Provider,
    rpc::types::eth::{TransactionReceipt, TransactionRequest},
    signers::{local::PrivateKeySigner, SignerSync},
    sol,
    sol_types::SolCall,
    transports::Transport,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use tracing::{info, warn};

/// Bindings for OZ ERC2771Forwarder.
/// execute() takes a single ForwardRequestData struct that includes the signature.
sol! {
    #[sol(rpc)]
    interface IERC2771Forwarder {
        struct ForwardRequestData {
            address from;
            address to;
            uint256 value;
            uint256 gas;
            uint48 deadline;
            bytes data;
            bytes signature;
        }
        function execute(ForwardRequestData calldata request) external payable;
    }
}

pub fn encode_execute_call(req: &ForwardRequestModel, signature: &[u8]) -> Result<Bytes> {
    let sol_req = IERC2771Forwarder::ForwardRequestData {
        from: req.from,
        to: req.to,
        value: req.value,
        gas: req.gas,
        deadline: req.deadline,
        data: req.data.clone().into(),
        signature: signature.to_vec().into(),
    };
    let call = IERC2771Forwarder::executeCall { request: sol_req };
    Ok(call.abi_encode().into())
}

const FORWARDER_OVERHEAD_GAS: u64 = 50_000;
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
    pub fn new(provider: Arc<P>, signer: PrivateKeySigner, config: RelayExecutorConfig) -> Self {
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

        let gas_limit = self
            .provider
            .estimate_gas(&tx_est)
            .await
            .unwrap_or_else(|e| {
                warn!("gas estimation failed, using fallback: {}", e);
                (U256::from(FORWARDER_OVERHEAD_GAS) + req.gas).to::<u128>()
            });

        let gas_limit_with_buffer = gas_limit * (10000 + GAS_BUFFER_BPS) as u128 / 10000;

        let fees = self
            .provider
            .estimate_eip1559_fees(None)
            .await
            .context("failed to fetch fee data")?;

        let max_allowed_gwei =
            U256::from(self.config.max_gas_price_gwei) * U256::from(10).pow(U256::from(9));
        anyhow::ensure!(
            U256::from(fees.max_fee_per_gas) <= max_allowed_gwei,
            "gas price {:.1} gwei exceeds circuit breaker {}",
            fees.max_fee_per_gas as f64 / 1e9,
            self.config.max_gas_price_gwei
        );

        // Build typed EIP-1559 tx, sign locally, send raw —
        // public RPCs reject unsigned eth_sendTransaction.
        let mut typed_tx = TxEip1559 {
            chain_id: self.config.chain_id,
            nonce,
            gas_limit: gas_limit_with_buffer,
            max_fee_per_gas: fees.max_fee_per_gas,
            max_priority_fee_per_gas: fees.max_priority_fee_per_gas,
            to: TxKind::Call(self.config.forwarder_address),
            value: req.value,
            input: calldata,
            access_list: Default::default(),
        };

        let sig = self.signer
            .sign_hash_sync(&typed_tx.signature_hash())
            .context("failed to sign transaction")?;

        let raw = TxEnvelope::Eip1559(typed_tx.into_signed(sig)).encoded_2718();

        let pending = self
            .provider
            .send_raw_transaction(&raw)
            .await
            .context("failed to submit transaction")?;

        let tx_hash = *pending.tx_hash();
        info!(tx_hash = ?tx_hash, "transaction submitted");
        Ok(tx_hash)
    }

    pub async fn wait_for_confirmation(&self, tx_hash: B256) -> Result<TransactionReceipt> {
        let timeout_duration = Duration::from_secs(self.config.confirmation_timeout_secs);
        let poll_interval = Duration::from_secs(2);

        let receipt = timeout(timeout_duration, async {
            loop {
                match self.provider.get_transaction_receipt(tx_hash).await {
                    Ok(Some(r)) => return Ok(r),
                    Ok(None) => tokio::time::sleep(poll_interval).await,
                    Err(e) => return Err(anyhow::anyhow!("rpc error: {e}")),
                }
            }
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
