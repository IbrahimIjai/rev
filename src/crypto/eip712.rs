use crate::models::{Eip712Domain as DomainModel, ForwardRequest as ForwardRequestModel};
use anyhow::{Context, Result};
use alloy::{
    primitives::{Address, B256, U256, Signature},
    sol,
    sol_types::{SolStruct, Eip712Domain},
};

/// Matches OZ ERC2771Forwarder v5.x ForwardRequestData signing struct.
/// Typehash: "ForwardRequest(address from,address to,uint256 value,uint256 gas,uint256 nonce,uint48 deadline,bytes data)"
///
/// Important: `nonce` is in the EIP-712 hash but NOT in the on-chain ForwardRequestData struct.
/// The forwarder reads nonces(from) on-chain and includes it in its own hash computation.
/// Clients must sign with the current nonce so the hashes match.
sol! {
    #[derive(Debug, Default)]
    struct ForwardRequest {
        address from;
        address to;
        uint256 value;
        uint256 gas;
        uint256 nonce;
        uint48 deadline;
        bytes data;
    }
}

fn to_sol_request(req: &ForwardRequestModel) -> ForwardRequest {
    ForwardRequest {
        from: req.from,
        to: req.to,
        value: req.value,
        gas: req.gas,
        nonce: req.nonce,
        deadline: req.deadline,
        data: req.data.clone(),
    }
}

pub fn recover_signer(digest: B256, signature: &[u8]) -> Result<Address> {
    let sig = Signature::try_from(signature).context("invalid signature format")?;
    sig.recover_address_from_prehash(&digest).context("ecrecover failed")
}

#[derive(Debug, Clone)]
pub struct Eip712Verifier {
    pub domain: DomainModel,
    alloy_domain: Eip712Domain,
}

impl Eip712Verifier {
    pub fn new(domain: DomainModel) -> Self {
        let alloy_domain = Eip712Domain {
            name: Some(domain.name.clone().into()),
            version: Some(domain.version.clone().into()),
            chain_id: Some(U256::from(domain.chain_id)),
            verifying_contract: Some(domain.verifying_contract),
            salt: None,
        };
        Self { domain, alloy_domain }
    }

    pub fn hash_request(&self, req: &ForwardRequestModel) -> B256 {
        to_sol_request(req).eip712_signing_hash(&self.alloy_domain)
    }

    pub fn verify(&self, req: &ForwardRequestModel, signature: &[u8]) -> Result<Address> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        anyhow::ensure!(
            req.deadline > now,
            "request deadline has passed (deadline={}, now={})",
            req.deadline,
            now
        );

        let digest = self.hash_request(req);
        let signer = recover_signer(digest, signature)?;

        anyhow::ensure!(
            signer == req.from,
            "signer mismatch: expected {:?}, recovered {:?}",
            req.from,
            signer
        );

        Ok(signer)
    }

    pub fn domain_separator(&self) -> B256 {
        self.alloy_domain.hash_struct()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::primitives::bytes;

    fn test_domain() -> DomainModel {
        DomainModel {
            name: "GasRelayForwarder".to_string(),
            version: "1".to_string(),
            chain_id: 1,
            verifying_contract: "0x0000000000000000000000000000000000000001"
                .parse()
                .unwrap(),
        }
    }

    #[test]
    fn test_hash_deterministic() {
        let verifier = Eip712Verifier::new(test_domain());
        let req = ForwardRequestModel {
            from: Address::ZERO,
            to: Address::ZERO,
            value: U256::ZERO,
            gas: U256::from(100_000u64),
            nonce: U256::ZERO,
            deadline: 9999999999u64,
            data: bytes!("deadbeef"),
        };
        assert_eq!(verifier.hash_request(&req), verifier.hash_request(&req));
    }
}
