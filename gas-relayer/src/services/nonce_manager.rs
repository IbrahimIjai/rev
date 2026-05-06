use anyhow::Result;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use alloy::{
    providers::Provider,
    primitives::{Address, U256},
    sol,
    network::Ethereum,
    transports::Transport,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, warn};

/// OZ ERC2771Forwarder uses `nonces(address)` (ERC-2612 style), not `getNonce`.
sol! {
    #[sol(rpc)]
    interface IERC2771Forwarder {
        function nonces(address owner) external view returns (uint256);
    }
}

#[derive(Debug, Clone)]
struct CachedNonce {
    nonce: U256,
    fetched_at: DateTime<Utc>,
}

#[derive(Debug)]
pub struct UserNonceCache {
    cache: DashMap<(u64, Address), CachedNonce>,
    ttl_seconds: u64,
}

impl UserNonceCache {
    pub fn new(ttl_seconds: u64) -> Self {
        Self { cache: DashMap::new(), ttl_seconds }
    }

    pub fn get(&self, chain_id: u64, user: Address) -> Option<U256> {
        let key = (chain_id, user);
        if let Some(entry) = self.cache.get(&key) {
            let age = Utc::now().signed_duration_since(entry.fetched_at).num_seconds() as u64;
            if age < self.ttl_seconds {
                return Some(entry.nonce);
            }
        }
        None
    }

    pub fn set(&self, chain_id: u64, user: Address, nonce: U256) {
        self.cache.insert((chain_id, user), CachedNonce { nonce, fetched_at: Utc::now() });
    }

    pub fn invalidate(&self, chain_id: u64, user: Address) {
        self.cache.remove(&(chain_id, user));
    }
}

#[derive(Debug)]
pub struct RelayerNonceManager {
    pub address: Address,
    pub chain_id: u64,
    inner: Mutex<RelayerNonceState>,
}

#[derive(Debug)]
struct RelayerNonceState {
    next_nonce: u64,
    initialized: bool,
    pending: Vec<u64>,
}

impl RelayerNonceManager {
    pub fn new(address: Address, chain_id: u64) -> Self {
        Self {
            address,
            chain_id,
            inner: Mutex::new(RelayerNonceState {
                next_nonce: 0,
                initialized: false,
                pending: Vec::new(),
            }),
        }
    }

    pub async fn acquire_nonce<P, T>(&self, provider: &P) -> Result<u64>
    where
        P: Provider<T, Ethereum>,
        T: Transport + Clone,
    {
        let mut state = self.inner.lock().await;
        if !state.initialized {
            let on_chain = provider.get_transaction_count(self.address).pending().await?;
            state.next_nonce = on_chain;
            state.initialized = true;
        }
        let nonce = state.next_nonce;
        state.next_nonce += 1;
        state.pending.push(nonce);
        Ok(nonce)
    }

    pub async fn confirm_nonce(&self, nonce: u64) {
        self.inner.lock().await.pending.retain(|&n| n != nonce);
    }

    pub async fn reset_from_chain<P, T>(&self, provider: &P) -> Result<()>
    where
        P: Provider<T, Ethereum>,
        T: Transport + Clone,
    {
        let on_chain = provider.get_transaction_count(self.address).pending().await?;
        let mut state = self.inner.lock().await;
        state.next_nonce = on_chain;
        state.pending.clear();
        state.initialized = true;
        Ok(())
    }
}

pub struct NonceService<P, T = alloy::transports::BoxTransport> {
    pub user_cache: Arc<UserNonceCache>,
    pub provider: Arc<P>,
    forwarder_address: Address,
    chain_id: u64,
    _marker: std::marker::PhantomData<T>,
}

impl<P, T> std::fmt::Debug for NonceService<P, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NonceService").finish()
    }
}

impl<P, T> NonceService<P, T>
where
    P: Provider<T, Ethereum>,
    T: Transport + Clone,
{
    pub fn new(
        provider: Arc<P>,
        forwarder_address: Address,
        chain_id: u64,
    ) -> Self {
        Self {
            user_cache: Arc::new(UserNonceCache::new(30)),
            provider,
            forwarder_address,
            chain_id,
            _marker: std::marker::PhantomData,
        }
    }

    /// Fetch user's current nonce from the OZ ERC2771Forwarder.
    /// Used by the GET /nonce endpoint; validation is done on-chain by the forwarder.
    pub async fn get_user_nonce(&self, user: Address) -> Result<U256> {
        if let Some(cached) = self.user_cache.get(self.chain_id, user) {
            return Ok(cached);
        }
        let forwarder = IERC2771Forwarder::new(self.forwarder_address, &*self.provider);
        let IERC2771Forwarder::noncesReturn { _0: nonce } =
            forwarder.nonces(user).call().await?;
        self.user_cache.set(self.chain_id, user, nonce);
        Ok(nonce)
    }

    pub fn invalidate_user_nonce(&self, user: Address) {
        self.user_cache.invalidate(self.chain_id, user);
    }
}
