use crate::models::ForwardRequest;
use anyhow::Result;
use alloy::primitives::{Address, U256};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub id: uuid::Uuid,
    pub name: String,
    pub api_key_hash: String,
    pub chain_id: u64,
    pub forwarder_address: Address,
    pub allowed_targets: AllowedTargets,
    pub daily_gas_quota_per_user: Option<U256>,
    pub max_gas_per_request: U256,
    pub active: bool,
    pub allowed_selectors: Vec<[u8; 4]>,
    pub rate_limit_per_user_per_minute: u32,
    pub relayer_address: Address,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AllowedTargets {
    Any, // use only for testing
    Allowlist(HashSet<Address>),
}

impl AllowedTargets {
    pub fn contains(&self, addr: &Address) -> bool {
        match self {
            AllowedTargets::Any => true,
            AllowedTargets::Allowlist(set) => set.contains(addr),
        }
    }
}


#[derive(Debug, thiserror::Error)]
pub enum PolicyViolation {
    #[error("policy is not active")]
    PolicyInactive,

    #[error("chain mismatch: policy is for chain {policy_chain}, request is for chain {request_chain}")]
    ChainMismatch {
        policy_chain: u64,
        request_chain: u64,
    },

    #[error("target contract {target:?} is not in allowed list")]
    TargetNotAllowed { target: Address },

    #[error("function selector {selector:?} is not in allowed list")]
    SelectorNotAllowed { selector: [u8; 4] },

    #[error("gas limit {requested} exceeds policy maximum {maximum}")]
    GasLimitExceeded { requested: U256, maximum: U256 },

    #[error("user {user:?} has exceeded daily gas quota")]
    QuotaExceeded { user: Address },

    #[error("user {user:?} is rate limited")]
    RateLimited { user: Address },

    #[error("user {user:?} is banned")]
    UserBanned { user: Address },

    #[error("request data too short to contain function selector")]
    DataTooShort,
}


#[derive(Debug)]
pub struct PolicyEnforcer {
    pub policy: Policy,
    banned_users: Arc<RwLock<HashSet<Address>>>,
    // address -> (used_gas, day_start_unix_secs)
    gas_usage: Arc<RwLock<HashMap<Address, (U256, u64)>>>,
}

impl PolicyEnforcer {
    pub fn new(policy: Policy) -> Self {
        Self {
            policy,
            banned_users: Arc::new(RwLock::new(HashSet::new())),
            gas_usage: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn check(
        &self,
        req: &ForwardRequest,
        request_chain_id: u64,
    ) -> Result<(), PolicyViolation> {
        // 1. Policy must be active
        if !self.policy.active {
            return Err(PolicyViolation::PolicyInactive);
        }

        // 2. Chain must match
        if self.policy.chain_id != request_chain_id {
            return Err(PolicyViolation::ChainMismatch {
                policy_chain: self.policy.chain_id,
                request_chain: request_chain_id,
            });
        }

        // 3. User must not be banned
        {
            let banned = self.banned_users.read().await;
            if banned.contains(&req.from) {
                return Err(PolicyViolation::UserBanned { user: req.from });
            }
        }

        // 4. Target contract must be allowed
        if !self.policy.allowed_targets.contains(&req.to) {
            return Err(PolicyViolation::TargetNotAllowed { target: req.to });
        }

        // 5. Function selector must be allowed (if selector list is non-empty)
        if !self.policy.allowed_selectors.is_empty() {
            if req.data.len() < 4 {
                return Err(PolicyViolation::DataTooShort);
            }
            let selector: [u8; 4] = req.data[..4].try_into().unwrap();
            if !self.policy.allowed_selectors.contains(&selector) {
                return Err(PolicyViolation::SelectorNotAllowed { selector });
            }
        }

        // 6. Gas limit must be within policy bounds
        if req.gas > self.policy.max_gas_per_request {
            return Err(PolicyViolation::GasLimitExceeded {
                requested: req.gas,
                maximum: self.policy.max_gas_per_request,
            });
        }

        // 7. Daily gas quota check
        if let Some(daily_quota) = self.policy.daily_gas_quota_per_user {
            self.check_quota(req.from, req.gas, daily_quota).await?;
        }

        Ok(())
    }

    async fn check_quota(
        &self,
        user: Address,
        requested_gas: U256,
        quota: U256,
    ) -> Result<(), PolicyViolation> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let day_start = now - (now % 86400);

        let mut usage_map = self.gas_usage.write().await;
        let entry = usage_map.entry(user).or_insert((U256::ZERO, day_start));

        if entry.1 < day_start {
            *entry = (U256::ZERO, day_start);
        }

        let new_total = entry.0 + requested_gas;
        if new_total > quota {
            return Err(PolicyViolation::QuotaExceeded { user });
        }

        entry.0 = new_total;
        Ok(())
    }

    pub async fn record_actual_gas_used(
        &self,
        user: Address,
        estimated_gas: U256,
        actual_gas: U256,
    ) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let day_start = now - (now % 86400);

        let mut usage_map = self.gas_usage.write().await;
        if let Some(entry) = usage_map.get_mut(&user) {
            if entry.1 == day_start && entry.0 >= estimated_gas {
                entry.0 = entry.0 - estimated_gas + actual_gas;
            }
        }
    }

    pub async fn refund_quota(&self, user: Address, refund_gas: U256) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let day_start = now - (now % 86400);

        let mut usage_map = self.gas_usage.write().await;
        if let Some(entry) = usage_map.get_mut(&user) {
            if entry.1 == day_start && entry.0 >= refund_gas {
                entry.0 -= refund_gas;
            }
        }
    }

    pub async fn ban_user(&self, user: Address) {
        self.banned_users.write().await.insert(user);
    }

    pub async fn unban_user(&self, user: Address) {
        self.banned_users.write().await.remove(&user);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::primitives::Bytes;

    fn test_policy() -> Policy {
        Policy {
            id: uuid::Uuid::new_v4(),
            name: "Test Policy".to_string(),
            api_key_hash: "hash".to_string(),
            chain_id: 1,
            forwarder_address: Address::ZERO,
            allowed_targets: AllowedTargets::Allowlist(
                vec!["0x1111111111111111111111111111111111111111"
                    .parse::<Address>()
                    .unwrap()]
                .into_iter()
                .collect(),
            ),
            daily_gas_quota_per_user: Some(U256::from(1_000_000u64)),
            max_gas_per_request: U256::from(500_000u64),
            active: true,
            allowed_selectors: vec![],
            rate_limit_per_user_per_minute: 10,
            relayer_address: Address::ZERO,
        }
    }

    fn test_request(target: Address, gas: U256) -> ForwardRequest {
        ForwardRequest {
            from: "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                .parse()
                .unwrap(),
            to: target,
            value: U256::ZERO,
            gas,
            nonce: U256::ZERO,
            deadline: None,
            data: Bytes::from(vec![0xa9, 0x05, 0x9c, 0xbb, 0x00, 0x00, 0x00, 0x00]), // transfer(...)
        }
    }

    #[tokio::test]
    async fn test_allowed_target_passes() {
        let policy = test_policy();
        let enforcer = PolicyEnforcer::new(policy);
        let target: Address = "0x1111111111111111111111111111111111111111"
            .parse()
            .unwrap();
        let req = test_request(target, U256::from(100_000u64));
        assert!(enforcer.check(&req, 1).await.is_ok());
    }

    #[tokio::test]
    async fn test_disallowed_target_fails() {
        let policy = test_policy();
        let enforcer = PolicyEnforcer::new(policy);
        let bad_target: Address = "0x2222222222222222222222222222222222222222"
            .parse()
            .unwrap();
        let req = test_request(bad_target, U256::from(100_000u64));
        let result = enforcer.check(&req, 1).await;
        assert!(matches!(result, Err(PolicyViolation::TargetNotAllowed { .. })));
    }

    #[tokio::test]
    async fn test_gas_exceeds_max() {
        let policy = test_policy();
        let enforcer = PolicyEnforcer::new(policy);
        let target: Address = "0x1111111111111111111111111111111111111111"
            .parse()
            .unwrap();
        let req = test_request(target, U256::from(600_000u64)); // > 500k max
        let result = enforcer.check(&req, 1).await;
        assert!(matches!(result, Err(PolicyViolation::GasLimitExceeded { .. })));
    }

    #[tokio::test]
    async fn test_quota_accumulates_and_rejects() {
        let policy = test_policy(); // quota = 1,000,000
        let enforcer = PolicyEnforcer::new(policy);
        let target: Address = "0x1111111111111111111111111111111111111111"
            .parse()
            .unwrap();

        // Two 400k requests = 800k — ok
        let req = test_request(target, U256::from(400_000u64));
        assert!(enforcer.check(&req, 1).await.is_ok());
        assert!(enforcer.check(&req, 1).await.is_ok());

        // Third request puts us at 1.2M > 1M quota — should fail
        let result = enforcer.check(&req, 1).await;
        assert!(matches!(result, Err(PolicyViolation::QuotaExceeded { .. })));
    }

    #[tokio::test]
    async fn test_banned_user_rejected() {
        let policy = test_policy();
        let enforcer = PolicyEnforcer::new(policy);
        let user: Address = "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            .parse()
            .unwrap();
        let target: Address = "0x1111111111111111111111111111111111111111"
            .parse()
            .unwrap();

        enforcer.ban_user(user).await;
        let req = test_request(target, U256::from(100_000u64));
        let result = enforcer.check(&req, 1).await;
        assert!(matches!(result, Err(PolicyViolation::UserBanned { .. })));
    }
}
