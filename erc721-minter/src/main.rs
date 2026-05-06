use alloy::{
    primitives::{Address, Bytes, U256},
    signers::{local::PrivateKeySigner, SignerSync},
    sol,
    sol_types::{Eip712Domain, SolStruct},
};
use anyhow::{Context, Result};
use serde::Deserialize;

const RELAYER_URL: &str = "http://localhost:8080";
const CHAIN_ID: u64 = 84532;
const NFT_ADDRESS: &str = "0xBe9ec79854e459F38E0B868A0c3429AAbf6784b2";
const PRIVATE_KEY: &str = "0xa9abdb067cab927e7d71167429ba99b789737ab631593fe2bb346bd8f265debb";
const API_KEY: &str = "pk_live_f4efae493567845d4bbe921d4aa62fa20096d08bd992c4ad";

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

#[derive(Deserialize)]
struct NonceResp {
    nonce: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DomainResp {
    name: String,
    version: String,
    verifying_contract: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RelayResp {
    job_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct JobResp {
    status: String,
    tx_hash: Option<String>,
    error: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let client = reqwest::Client::new();

    let signer: PrivateKeySigner = PRIVATE_KEY.parse().context("invalid private key")?;
    let user = signer.address();
    println!("Wallet:  {user}");
    println!("NFT:     {NFT_ADDRESS}");
    println!("Network: Base Sepolia ({})", CHAIN_ID);
    println!();

    let nonce_resp: NonceResp = client
        .get(format!("{RELAYER_URL}/nonce/{CHAIN_ID}/{user}"))
        .send()
        .await?
        .json()
        .await
        .context("failed to fetch nonce")?;
    let nonce_hex = nonce_resp.nonce.trim_start_matches("0x").to_string();
    let nonce = U256::from_str_radix(&nonce_hex, 16).context("invalid nonce")?;
    println!("Nonce:   {nonce}");

    let domain_resp: DomainResp = client
        .get(format!("{RELAYER_URL}/domain/{CHAIN_ID}"))
        .send()
        .await?
        .json()
        .await
        .context("failed to fetch domain")?;
    let forwarder: Address = domain_resp.verifying_contract.parse()?;
    println!("Domain:  {} (forwarder {forwarder})", domain_resp.name);
    println!();

    let domain = Eip712Domain {
        name: Some(domain_resp.name.into()),
        version: Some(domain_resp.version.into()),
        chain_id: Some(U256::from(CHAIN_ID)),
        verifying_contract: Some(forwarder),
        salt: None,
    };

    let deadline = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs()
        + 3600;

    let nft: Address = NFT_ADDRESS.parse()?;

    let req = ForwardRequest {
        from: user,
        to: nft,
        value: U256::ZERO,
        gas: U256::from(200_000u64),
        nonce,
        deadline,
        data: Bytes::from_static(&[0x12, 0x49, 0xc5, 0x8b]),
    };

    let signing_hash = req.eip712_signing_hash(&domain);
    let sig = signer.sign_hash_sync(&signing_hash)?;
    let sig_hex = format!("0x{}", alloy::hex::encode(sig.as_bytes()));
    println!("Signing ForwardRequest...");
    println!("Sig:     {}...{}", &sig_hex[..10], &sig_hex[sig_hex.len() - 8..]);
    println!();

    let payload = serde_json::json!({
        "request": {
            "from":     format!("{user:?}"),
            "to":       NFT_ADDRESS,
            "value":    "0x0",
            "gas":      format!("0x{:x}", 200_000u64),
            "nonce":    format!("0x{nonce:x}"),
            "deadline": deadline,
            "data":     "0x1249c58b"
        },
        "signature": sig_hex,
        "chainId":   CHAIN_ID
    });

    println!("Submitting to relayer...");
    let relay_http = client
        .post(format!("{RELAYER_URL}/relay"))
        .header("Authorization", format!("Bearer {API_KEY}"))
        .json(&payload)
        .send()
        .await?;

    if !relay_http.status().is_success() {
        let body = relay_http.text().await?;
        anyhow::bail!("relayer rejected request: {body}");
    }

    let relay: RelayResp = relay_http.json().await.context("invalid relay response")?;
    println!("Job ID:  {}", relay.job_id);
    println!();

    println!("Waiting for on-chain confirmation...");
    for _ in 0..30 {
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
        print!(".");
        std::io::Write::flush(&mut std::io::stdout())?;

        let resp = client
            .get(format!("{RELAYER_URL}/relay/{}", relay.job_id))
            .send()
            .await?;

        if resp.status() == 404 {
            continue;
        }

        let job: JobResp = resp.json().await.context("invalid job response")?;

        match job.status.as_str() {
            "confirmed" => {
                let tx = job.tx_hash.unwrap_or_default();
                println!();
                println!();
                println!("✓ NFT minted!");
                println!("  https://base-sepolia.blockscout.com/tx/{tx}");
                return Ok(());
            }
            "failed" => {
                println!();
                anyhow::bail!("relay failed: {}", job.error.unwrap_or_default());
            }
            _ => {}
        }
    }

    println!();
    anyhow::bail!("timed out waiting for confirmation — check job {} manually", relay.job_id);
}
