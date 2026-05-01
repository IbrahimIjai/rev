use crate::{
    crypto::eip712::Eip712Verifier,
    models::{
        JobStatus, RelayPayload, RelayResponse,
    },
    services::{
        nonce_manager::NonceService,
        policy::PolicyEnforcer,
        queue::{QueuedJob, RelayQueue},
    },
};
use anyhow::{Context, Result};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use alloy::{
    primitives::{Address},
    network::{Ethereum},
    transports::Transport,
    providers::Provider,
};
use serde::{Serialize};
use std::sync::Arc;
use std::str::FromStr;
use tracing::error;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AppState<P, T = alloy::transports::BoxTransport> 
where
    P: Provider<T, Ethereum>,
    T: Transport + Clone,
{
    pub verifier: Arc<Eip712Verifier>,
    pub policy_enforcer: Arc<PolicyEnforcer>,
    pub nonce_service: Arc<NonceService<P, T>>,
    pub relay_queue: Arc<RelayQueue>,
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
        Self {
            error: error.to_string(),
            code,
            details: None,
        }
    }
}

pub async fn relay_handler<P, T>(
    State(state): State<AppState<P, T>>,
    Json(payload): Json<RelayPayload>,
) -> impl IntoResponse 
where
    P: Provider<T, Ethereum>,
    T: Transport + Clone,
{
    if payload.request.gas.is_zero() {
        return (StatusCode::BAD_REQUEST, Json(ErrorResponse::new("INVALID_GAS", "gas must be non-zero"))).into_response();
    }

    match state.policy_enforcer.check(&payload.request, payload.chain_id).await {
        Ok(()) => {}
        Err(e) => return (StatusCode::FORBIDDEN, Json(ErrorResponse::new("POLICY_VIOLATION", e))).into_response(),
    }

    match state.verifier.verify(&payload.request, &payload.signature) {
        Ok(_) => {}
        Err(e) => {
            state.policy_enforcer.refund_quota(payload.request.from, payload.request.gas).await;
            return (StatusCode::UNAUTHORIZED, Json(ErrorResponse::new("INVALID_SIGNATURE", e))).into_response();
        }
    }

    match state.nonce_service.validate_user_nonce(payload.request.from, payload.request.nonce).await {
        Ok(()) => {}
        Err(e) => {
            state.policy_enforcer.refund_quota(payload.request.from, payload.request.gas).await;
            return (StatusCode::BAD_REQUEST, Json(ErrorResponse::new("NONCE_MISMATCH", e))).into_response();
        }
    }

    let job = QueuedJob {
        id: Uuid::new_v4(),
        chain_id: payload.chain_id,
        request: payload.request,
        signature: payload.signature,
        attempts: 0,
        created_at: Utc::now(),
        next_attempt_at: Utc::now(),
    };

    match state.relay_queue.enqueue(job.clone()).await {
        Ok(()) => (StatusCode::ACCEPTED, Json(RelayResponse { job_id: job.id, status: JobStatus::Queued, estimated_seconds: Some(15) })).into_response(),
        Err(e) => {
            error!(error = %e, "failed to enqueue job");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new("QUEUE_ERROR", "failed to enqueue relay job"))).into_response()
        }
    }
}

pub async fn job_status_handler<P, T>(
    State(_state): State<AppState<P, T>>,
    Path(job_id): Path<Uuid>,
) -> impl IntoResponse 
where
    P: Provider<T, Ethereum>,
    T: Transport + Clone,
{
    (StatusCode::NOT_FOUND, Json(ErrorResponse::new("JOB_NOT_FOUND", format!("job {} not found", job_id)))).into_response()
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
        Err(_) => return (StatusCode::BAD_REQUEST, Json(ErrorResponse::new("INVALID_ADDRESS", "invalid Ethereum address"))).into_response(),
    };

    match state.nonce_service.get_user_nonce(addr).await {
        Ok(nonce) => (StatusCode::OK, Json(serde_json::json!({"address": addr, "chainId": chain_id, "nonce": format!("{:#x}", nonce)}))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse::new("RPC_ERROR", e))).into_response(),
    }
}

pub async fn health_handler() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({"status": "ok", "version": env!("CARGO_PKG_VERSION")}))).into_response()
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
        return (StatusCode::NOT_FOUND, Json(ErrorResponse::new("CHAIN_NOT_SUPPORTED", format!("chain {} is not supported", chain_id)))).into_response();
    }

    (StatusCode::OK, Json(serde_json::json!({
        "name": domain.name.clone(),
        "version": domain.version.clone(),
        "chainId": domain.chain_id,
        "verifyingContract": domain.verifying_contract,
        "domainSeparator": format!("{:?}", state.verifier.domain_separator()),
    }))).into_response()
}
