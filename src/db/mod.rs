pub mod entities;

use anyhow::{Context, Result};
use sea_orm::{Database, DatabaseConnection};

pub type Db = DatabaseConnection;

pub async fn connect(database_url: &str) -> Result<Db> {
    let db = Database::connect(database_url)
        .await
        .context("failed to connect to database")?;
    Ok(db)
}

/// Wallet key management helpers — encrypt/decrypt private keys for local_dev provider.
pub mod keys {
    use aes_gcm::{
        aead::{Aead, KeyInit, OsRng as AeadOsRng},
        Aes256Gcm, Nonce,
    };
    use anyhow::{Context, Result};
    use rand::RngCore;
    use sha2::{Digest, Sha256};

    const NONCE_LEN: usize = 12;

    fn derive_key(secret: &str) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(secret.as_bytes());
        hasher.finalize().into()
    }

    pub fn encrypt_key(plaintext_hex: &str, secret: &str) -> Result<String> {
        let key_bytes = derive_key(secret);
        let cipher = Aes256Gcm::new_from_slice(&key_bytes).context("invalid key")?;

        let mut nonce_bytes = [0u8; NONCE_LEN];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, plaintext_hex.as_bytes())
            .map_err(|e| anyhow::anyhow!("encrypt failed: {e}"))?;

        // nonce || ciphertext, hex-encoded
        let mut combined = nonce_bytes.to_vec();
        combined.extend_from_slice(&ciphertext);
        Ok(hex::encode(combined))
    }

    pub fn decrypt_key(encrypted_hex: &str, secret: &str) -> Result<String> {
        let data = hex::decode(encrypted_hex).context("invalid hex")?;
        anyhow::ensure!(data.len() > NONCE_LEN, "ciphertext too short");

        let (nonce_bytes, ciphertext) = data.split_at(NONCE_LEN);
        let nonce = Nonce::from_slice(nonce_bytes);

        let key_bytes = derive_key(secret);
        let cipher = Aes256Gcm::new_from_slice(&key_bytes).context("invalid key")?;

        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| anyhow::anyhow!("decrypt failed: {e}"))?;

        String::from_utf8(plaintext).context("invalid utf8 after decrypt")
    }
}

/// Generate a fresh EOA keypair and return (address_hex, privkey_hex).
pub fn generate_wallet() -> (String, String) {
    use k256::ecdsa::SigningKey;
    use rand::rngs::OsRng;
    use alloy::signers::local::PrivateKeySigner;

    let signing_key = SigningKey::random(&mut OsRng);
    let privkey_bytes = signing_key.to_bytes();
    let privkey_hex = format!("0x{}", hex::encode(privkey_bytes));

    let signer = PrivateKeySigner::from_signing_key(signing_key);
    let address = format!("{:?}", signer.address());

    (address, privkey_hex)
}

/// Hash an API key for storage (SHA-256, hex output).
pub fn hash_api_key(raw_key: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(raw_key.as_bytes());
    hex::encode(hasher.finalize())
}

/// Generate a new random API key with a "pk_live_" prefix.
pub fn generate_api_key() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 24];
    rand::thread_rng().fill_bytes(&mut bytes);
    format!("pk_live_{}", hex::encode(bytes))
}
