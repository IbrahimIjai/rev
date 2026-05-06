use crate::db::{self, entities::business, Db};
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::Arc,
};
use tokio::sync::Mutex;
use alloy::primitives::{Address, Signature};
use anyhow::Context;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,         // business_id (UUID)
    pub address: String,     // Safe/EOA address
    pub exp: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NonceRequest {
    pub address: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NonceResponse {
    pub nonce: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VerifyRequest {
    pub address: String,
    pub message: String,
    pub signature: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthResponse {
    pub token: String,
    pub business_id: String,
}

/// In-memory nonce store: address → (nonce, expiry)
pub type NonceStore = Arc<Mutex<HashMap<String, (String, chrono::DateTime<Utc>)>>>;

pub fn new_nonce_store() -> NonceStore {
    Arc::new(Mutex::new(HashMap::new()))
}

#[derive(Clone)]
pub struct AuthState {
    pub db: Db,
    pub nonce_store: NonceStore,
    pub jwt_secret: String,
}

pub fn encode_jwt(business_id: &str, address: &str, secret: &str) -> anyhow::Result<String> {
    let exp = (Utc::now() + Duration::hours(24)).timestamp() as usize;
    let claims = Claims {
        sub: business_id.to_string(),
        address: address.to_string(),
        exp,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .context("failed to encode JWT")
}

pub fn decode_jwt(token: &str, secret: &str) -> anyhow::Result<Claims> {
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    )
    .context("invalid token")?;
    Ok(data.claims)
}

/// GET /api/auth/nonce?address=0x...
pub async fn nonce_handler(
    State(state): State<AuthState>,
    axum::extract::Query(params): axum::extract::Query<NonceRequest>,
) -> impl IntoResponse {
    use rand::RngCore;
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    let nonce = hex::encode(bytes);

    let expiry = Utc::now() + Duration::minutes(5);
    state
        .nonce_store
        .lock()
        .await
        .insert(params.address.to_lowercase(), (nonce.clone(), expiry));

    (StatusCode::OK, Json(NonceResponse { nonce }))
}

/// POST /api/auth/verify
/// Body: { address, message, signature }
/// Returns JWT on success.
pub async fn verify_handler(
    State(state): State<AuthState>,
    Json(body): Json<VerifyRequest>,
) -> impl IntoResponse {
    let addr_lower = body.address.to_lowercase();

    // 1. Check nonce
    let stored_nonce = {
        let mut store = state.nonce_store.lock().await;
        match store.remove(&addr_lower) {
            Some((nonce, expiry)) if Utc::now() < expiry => nonce,
            Some(_) => {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(serde_json::json!({"error": "nonce expired"})),
                )
                    .into_response()
            }
            None => {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(serde_json::json!({"error": "no nonce issued for this address"})),
                )
                    .into_response()
            }
        }
    };

    // 2. Verify the nonce appears in the message
    if !body.message.contains(&stored_nonce) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "message does not contain expected nonce"})),
        )
            .into_response();
    }

    // 3. Recover signer from personal_sign (Ethereum prefixed hash)
    let claimed_addr: Address = match body.address.parse() {
        Ok(a) => a,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "invalid address"})),
            )
                .into_response()
        }
    };

    let sig_bytes = match hex::decode(body.signature.trim_start_matches("0x")) {
        Ok(b) => b,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "invalid signature hex"})),
            )
                .into_response()
        }
    };

    let message_hash = {
        use sha3::{Digest, Keccak256};
        let prefix = format!("\x19Ethereum Signed Message:\n{}", body.message.len());
        let mut hasher = Keccak256::new();
        hasher.update(prefix.as_bytes());
        hasher.update(body.message.as_bytes());
        alloy::primitives::B256::from_slice(&hasher.finalize())
    };

    let recovered = match Signature::try_from(sig_bytes.as_slice())
        .and_then(|sig| sig.recover_address_from_prehash(&message_hash))
    {
        Ok(addr) => addr,
        Err(_) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "signature recovery failed"})),
            )
                .into_response()
        }
    };

    if recovered != claimed_addr {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "signer does not match claimed address"})),
        )
            .into_response();
    }

    // 4. Upsert business record
    let address_str = format!("{:?}", claimed_addr);
    let business_id = match business::Entity::find()
        .filter(business::Column::SafeAddress.eq(&address_str))
        .one(&state.db)
        .await
    {
        Ok(Some(existing)) => existing.id,
        Ok(None) => {
            let new_id = Uuid::new_v4();
            let now = chrono::Utc::now().fixed_offset();
            let model = business::ActiveModel {
                id: Set(new_id),
                safe_address: Set(address_str.clone()),
                name: Set(address_str.clone()),
                email: Set(None),
                tier: Set("free".to_string()),
                active: Set(true),
                created_at: Set(now),
            };
            match model.insert(&state.db).await {
                Ok(m) => m.id,
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({"error": format!("db error: {e}")})),
                    )
                        .into_response()
                }
            }
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("db error: {e}")})),
            )
                .into_response()
        }
    };

    // 5. Issue JWT
    match encode_jwt(&business_id.to_string(), &address_str, &state.jwt_secret) {
        Ok(token) => (
            StatusCode::OK,
            Json(AuthResponse {
                token,
                business_id: business_id.to_string(),
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("{e}")})),
        )
            .into_response(),
    }
}
