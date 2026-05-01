use chrono::{DateTime, Utc};
use alloy::primitives::{Address, Bytes, B256, U256};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct ForwardRequest {
    pub from: Address,
    pub to: Address,
    pub value: U256,
    pub gas: U256,
    pub nonce: U256,
    // Not in base ERC-2771 spec — strongly recommended for production
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deadline: Option<U256>,
    pub data: Bytes,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct RelayPayload {
    pub request: ForwardRequest,
    pub signature: Bytes,
    pub chain_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayJob {
    pub id: Uuid,
    pub chain_id: u64,
    pub request: ForwardRequest,
    pub signature: Bytes,
    pub status: JobStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub tx_hash: Option<B256>,
    pub block_number: Option<u64>,
    pub gas_used: Option<U256>,
    pub effective_gas_price: Option<U256>,
    pub attempts: u32,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Pending,
    Queued,
    Processing,
    Submitted,
    Confirmed,
    Failed,
    Reverted,
}

impl std::fmt::Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = serde_json::to_value(self)
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_default();
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelayResponse {
    pub job_id: Uuid,
    pub status: JobStatus,
    pub estimated_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobStatusResponse {
    pub job_id: Uuid,
    pub status: JobStatus,
    pub tx_hash: Option<B256>,
    pub block_number: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Eip712Domain {
    pub name: String,
    pub version: String,
    pub chain_id: u64,
    pub verifying_contract: Address,
}

#[derive(Debug, Clone)]
pub struct GasPrice {
    pub max_fee_per_gas: U256,
    pub max_priority_fee_per_gas: U256,
    pub base_fee: U256,
}

#[derive(Debug, Clone)]
pub struct NonceInfo {
    pub address: Address,
    pub chain_id: u64,
    pub nonce: U256,
    pub fetched_at: DateTime<Utc>,
}
