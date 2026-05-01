use crate::models::{Eip712Domain as DomainModel, ForwardRequest as ForwardRequestModel};
use anyhow::{Context, Result};
use alloy::{
    primitives::{Address, B256, U256, Signature},
    sol,
    sol_types::{SolStruct, Eip712Domain},
};

sol! {
    #[derive(Debug, Default)]
    struct ForwardRequest {
        address from;
        address to;
        uint256 value;
        uint256 gas;
        uint256 nonce;
        bytes data;
    }

    #[derive(Debug, Default)]
    struct ForwardRequestWithDeadline {
        address from;
        address to;
        uint256 value;
        uint256 gas;
        uint256 nonce;
        uint256 deadline;
        bytes data;
    }
}

fn to_sol_request(req: &ForwardRequestModel) -> SolRequest {
    if let Some(deadline) = req.deadline {
        SolRequest::WithDeadline(ForwardRequestWithDeadline {
            from: req.from,
            to: req.to,
            value: req.value,
            gas: req.gas,
            nonce: req.nonce,
            deadline,
            data: req.data.clone(),
        })
    } else {
        SolRequest::Base(ForwardRequest {
            from: req.from,
            to: req.to,
            value: req.value,
            gas: req.gas,
            nonce: req.nonce,
            data: req.data.clone(),
        })
    }
}

enum SolRequest {
    Base(ForwardRequest),
    WithDeadline(ForwardRequestWithDeadline),
}

impl SolRequest {
    fn hash_struct(&self, domain: &Eip712Domain) -> B256 {
        match self {
            SolRequest::Base(r) => r.eip712_signing_hash(domain),
            SolRequest::WithDeadline(r) => r.eip712_signing_hash(domain),
        }
    }
}

pub fn recover_signer(digest: B256, signature: &[u8]) -> Result<Address> {
    let sig = Signature::try_from(signature)
        .context("invalid signature format")?;
    
    let signer = sig.recover_address_from_prehash(&digest)
        .context("ecrecover failed")?;

    Ok(signer)
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

        Self {
            domain,
            alloy_domain,
        }
    }

    pub fn hash_request(&self, req: &ForwardRequestModel) -> B256 {
        let sol_req = to_sol_request(req);
        sol_req.hash_struct(&self.alloy_domain)
    }

    pub fn verify(&self, req: &ForwardRequestModel, signature: &[u8]) -> Result<Address> {
        if let Some(deadline) = req.deadline {
            let now = U256::from(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            );
            anyhow::ensure!(
                deadline > now,
                "request deadline has passed (deadline={}, now={})",
                deadline,
                now
            );
        }

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
            name: "TestForwarder".to_string(),
            version: "1".to_string(),
            chain_id: 1,
            verifying_contract: "0x0000000000000000000000000000000000000001"
                .parse()
                .unwrap(),
        }
    }

    #[test]
    fn test_hash_deterministic() {
        let domain = test_domain();
        let verifier = Eip712Verifier::new(domain);

        let req = ForwardRequestModel {
            from: Address::ZERO,
            to: Address::ZERO,
            value: U256::ZERO,
            gas: U256::from(100_000u64),
            nonce: U256::ZERO,
            deadline: None,
            data: bytes!("deadbeef"),
        };

        let h1 = verifier.hash_request(&req);
        let h2 = verifier.hash_request(&req);
        assert_eq!(h1, h2);
    }
}
