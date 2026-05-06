use crate::{
    crypto::eip712::Eip712Verifier,
    db::{self, entities::{api_key, spending_limit}, hash_api_key, Db},
    models::{JobStatus, JobStatusResponse, RelayPayload, RelayResponse},
    services::{
        nonce_manager::NonceService,
        policy::{AllowedTargets, Policy, PolicyEnforcer},
        queue::{QueuedJob, RelayQueue},
    },
};
use alloy::{
    network::Ethereum,
    primitives::{Address, U256},
    providers::Provider,
    transports::Transport,
};
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use dashmap::DashMap;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::Serialize;
use std::{collections::HashSet, str::FromStr, sync::Arc};
use tracing::error;
use uuid::Uuid;

pub type JobStore = Arc<DashMap<Uuid, JobStatusResponse>>;

#[derive(Debug, Clone)]
pub struct AppState<P, T = alloy::transports::BoxTransport>
where
    P: Provider<T, Ethereum>,
    T: Transport + Clone,
{
    pub verifier: Arc<Eip712Verifier>,
    pub nonce_service: Arc<NonceService<P, T>>,
    pub relay_queue: Arc<RelayQueue>,
    pub db: Db,
    pub job_store: JobStore,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl ErrorResponse {
    pub fn new(code: &'static str, error: impl ToString) -> Self {
        Self { error: error.to_string(), code, details: None }
    }
}

/// Extract `Bearer <key>` from Authorization header, or `X-API-Key` header.
fn extract_api_key(headers: &HeaderMap) -> Option<String> {
    if let Some(auth) = headers.get("authorization").and_then(|v| v.to_str().ok()) {
        if let Some(key) = auth.strip_prefix("Bearer ") {
            return Some(key.to_string());
        }
    }
    headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

/// Look up project + spending limits from an API key. Returns (project_id, policy).
async fn resolve_project_policy(
    db: &Db,
    raw_key: &str,
    chain_id: u64,
) -> Result<(Uuid, Policy), (StatusCode, &'static str)> {
    let key_hash = hash_api_key(raw_key);

    let api_key_row = api_key::Entity::find()
        .filter(api_key::Column::KeyHash.eq(&key_hash))
        .filter(api_key::Column::Active.eq(true))
        .one(db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db error"))?
        .ok_or((StatusCode::UNAUTHORIZED, "invalid or inactive API key"))?;

    let project_id = api_key_row.project_id;

    // Load spending limits
    let limits = spending_limit::Entity::find_by_id(project_id)
        .one(db)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "db error"))?
        .ok_or((StatusCode::INTERNAL_SERVER_ERROR, "project has no spending limits"))?;

    // Parse allowed_targets from JSONB
    let allowed_targets = match limits.allowed_targets.get("type").and_then(|t| t.as_str()) {
        Some("allowlist") => {
            let addrs: HashSet<Address> = limits
                .allowed_targets
                .get("addresses")
                .and_then(|a| a.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .filter_map(|s| s.parse().ok())
                        .collect()
                })
                .unwrap_or_default();
            AllowedTargets::Allowlist(addrs)
        }
        _ => AllowedTargets::Any,
    };

    // Parse allowed_selectors from JSONB array
    let allowed_selectors: Vec<[u8; 4]> = limits
        .allowed_selectors
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .filter_map(|s| {
                    let bytes = hex::decode(s.trim_start_matches("0x")).ok()?;
                    bytes.try_into().ok()
                })
                .collect()
        })
        .unwrap_or_default();

    let daily_quota = limits
        .daily_gas_quota_per_user
        .as_ref()
        .and_then(|d| d.to_string().parse::<u64>().ok())
        .map(U256::from);

    let max_gas = limits
        .max_gas_per_request
        .to_string()
        .parse::<u64>()
        .map(U256::from)
        .unwrap_or(U256::from(500_000u64));

    let policy = Policy {
        id: project_id,
        name: project_id.to_string(),
        api_key_hash: key_hash,
        chain_id,
        forwarder_address: Address::ZERO, // looked up per-chain in executor
        allowed_targets,
        daily_gas_quota_per_user: daily_quota,
        max_gas_per_request: max_gas,
        active: true,
        allowed_selectors,
        rate_limit_per_user_per_minute: limits.rate_limit_per_minute as u32,
        relayer_address: Address::ZERO,
    };

    Ok((project_id, policy))
}

pub async fn relay_handler<P, T>(
    State(state): State<AppState<P, T>>,
    headers: HeaderMap,
    Json(payload): Json<RelayPayload>,
) -> impl IntoResponse
where
    P: Provider<T, Ethereum>,
    T: Transport + Clone,
{
    // 1. Require non-zero gas
    if payload.request.gas.is_zero() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("INVALID_GAS", "gas must be non-zero")),
        )
            .into_response();
    }

    // 2. Extract and validate API key → load project policy
    let raw_key = match extract_api_key(&headers) {
        Some(k) => k,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new("MISSING_API_KEY", "provide Authorization: Bearer <api_key>")),
            )
                .into_response()
        }
    };

    let (project_id, policy) = match resolve_project_policy(&state.db, &raw_key, payload.chain_id).await {
        Ok(p) => p,
        Err((status, msg)) => {
            return (status, Json(ErrorResponse::new("AUTH_ERROR", msg))).into_response()
        }
    };

    let enforcer = PolicyEnforcer::new(policy);

    // 3. Policy checks
    if let Err(e) = enforcer.check(&payload.request, payload.chain_id).await {
        return (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new("POLICY_VIOLATION", e)),
        )
            .into_response();
    }

    // 4. EIP-712 signature verification
    if let Err(e) = state.verifier.verify(&payload.request, &payload.signature) {
        enforcer.refund_quota(payload.request.from, payload.request.gas).await;
        return (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("INVALID_SIGNATURE", e)),
        )
            .into_response();
    }

    // 5. Enqueue
    let job = QueuedJob {
        id: Uuid::new_v4(),
        project_id,
        chain_id: payload.chain_id,
        request: payload.request,
        signature: payload.signature,
        attempts: 0,
        created_at: Utc::now(),
        next_attempt_at: Utc::now(),
    };

    match state.relay_queue.enqueue(job.clone()).await {
        Ok(()) => (
            StatusCode::ACCEPTED,
            Json(RelayResponse {
                job_id: job.id,
                status: JobStatus::Queued,
                estimated_seconds: Some(15),
            }),
        )
            .into_response(),
        Err(e) => {
            error!(error = %e, "failed to enqueue job");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("QUEUE_ERROR", "failed to enqueue relay job")),
            )
                .into_response()
        }
    }
}

pub async fn job_status_handler<P, T>(
    State(state): State<AppState<P, T>>,
    Path(job_id): Path<Uuid>,
) -> impl IntoResponse
where
    P: Provider<T, Ethereum>,
    T: Transport + Clone,
{
    match state.job_store.get(&job_id) {
        Some(result) => (StatusCode::OK, Json(result.clone())).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("JOB_NOT_FOUND", format!("job {} not found", job_id))),
        )
            .into_response(),
    }
}

pub async fn get_nonce_handler<P, T>(
    State(state): State<AppState<P, T>>,
    Path((chain_id, address)): Path<(u64, String)>,
) -> impl IntoResponse
where
    P: Provider<T, Ethereum>,
    T: Transport + Clone,
{
    let addr = match Address::from_str(&address) {
        Ok(a) => a,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new("INVALID_ADDRESS", "invalid Ethereum address")),
            )
                .into_response()
        }
    };

    match state.nonce_service.get_user_nonce(addr).await {
        Ok(nonce) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "address": addr,
                "chainId": chain_id,
                "nonce": format!("{:#x}", nonce),
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("RPC_ERROR", e)),
        )
            .into_response(),
    }
}

pub async fn health_handler() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(serde_json::json!({"status": "ok", "version": env!("CARGO_PKG_VERSION")})),
    )
        .into_response()
}

pub async fn domain_handler<P, T>(
    State(state): State<AppState<P, T>>,
    Path(chain_id): Path<u64>,
) -> impl IntoResponse
where
    P: Provider<T, Ethereum>,
    T: Transport + Clone,
{
    let domain = &state.verifier.domain;
    if domain.chain_id != chain_id {
        return (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "CHAIN_NOT_SUPPORTED",
                format!("chain {} is not supported", chain_id),
            )),
        )
            .into_response();
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "name": domain.name,
            "version": domain.version,
            "chainId": domain.chain_id,
            "verifyingContract": domain.verifying_contract,
            "domainSeparator": format!("{:?}", state.verifier.domain_separator()),
        })),
    )
        .into_response()
}
