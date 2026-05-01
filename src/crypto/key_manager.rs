use anyhow::{Context, Result};
use alloy::{
    signers::{Signer, local::PrivateKeySigner},
    primitives::{Address, Bytes, Signature, B256},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;


#[async_trait::async_trait]
pub trait TransactionSigner: Send + Sync + std::fmt::Debug {
    fn address(&self) -> Address;
    async fn sign_message(&self, message: &[u8]) -> Result<Signature>;
    async fn sign_hash(&self, hash: &B256) -> Result<Signature>;
}


#[derive(Debug, Clone)]
pub struct LocalSigner {
    signer: PrivateKeySigner,
}

impl LocalSigner {
    pub fn from_env(var: &str) -> Result<Self> {
        let key = std::env::var(var)
            .with_context(|| format!("env var {} not set", var))?;
        let signer = key.parse::<PrivateKeySigner>()
            .context("invalid private key")?;
        Ok(Self { signer })
    }

    pub fn from_hex(key: &str) -> Result<Self> {
        let signer = key.parse::<PrivateKeySigner>()
            .context("invalid private key")?;
        Ok(Self { signer })
    }
}

#[async_trait::async_trait]
impl TransactionSigner for LocalSigner {
    fn address(&self) -> Address {
        self.signer.address()
    }

    async fn sign_message(&self, message: &[u8]) -> Result<Signature> {
        self.signer.sign_message(message).await
            .context("local message signing failed")
    }

    async fn sign_hash(&self, hash: &B256) -> Result<Signature> {
        self.signer.sign_hash(hash).await
            .context("local hash signing failed")
    }
}


#[derive(Debug)]
pub struct AwsKmsSigner {
    pub key_arn: String,
    pub address: Address,
}

impl AwsKmsSigner {
    pub async fn new(key_arn: String) -> Result<Self> {
        todo!("Implement AWS KMS integration with Alloy primitives.")
    }
}

#[async_trait::async_trait]
impl TransactionSigner for AwsKmsSigner {
    fn address(&self) -> Address {
        self.address
    }

    async fn sign_message(&self, _message: &[u8]) -> Result<Signature> {
        todo!()
    }

    async fn sign_hash(&self, _hash: &B256) -> Result<Signature> {
        todo!()
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KeyProvider {
    Local { private_key_hex: String },
    AwsKms { key_arn: String },
    Vault { vault_addr: String, key_path: String },
    GcpKms { resource_name: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyReference {
    pub wallet_address: Address,
    pub chain_id: u64,
    pub provider: KeyProvider,
}

#[derive(Debug)]
pub struct KeyManager {
    cache: dashmap::DashMap<Address, Arc<dyn TransactionSigner>>,
}

impl KeyManager {
    pub fn new() -> Self {
        Self {
            cache: dashmap::DashMap::new(),
        }
    }

    pub async fn get_signer(
        &self,
        key_ref: &KeyReference,
    ) -> Result<Arc<dyn TransactionSigner>> {
        if let Some(signer) = self.cache.get(&key_ref.wallet_address) {
            return Ok(signer.clone());
        }

        let signer: Arc<dyn TransactionSigner> = match &key_ref.provider {
            KeyProvider::Local { private_key_hex } => {
                Arc::new(LocalSigner::from_hex(private_key_hex)?)
            }
            KeyProvider::AwsKms { key_arn } => {
                Arc::new(AwsKmsSigner::new(key_arn.clone()).await?)
            }
            _ => todo!("Other signers not implemented"),
        };

        self.cache.insert(key_ref.wallet_address, signer.clone());
        Ok(signer)
    }

    pub fn invalidate(&self, address: Address) {
        self.cache.remove(&address);
    }
}

pub fn address_from_key(private_key_hex: &str) -> Result<Address> {
    let signer = private_key_hex.parse::<PrivateKeySigner>()
        .context("invalid private key")?;
    Ok(signer.address())
}
