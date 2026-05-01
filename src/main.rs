mod config;
mod crypto;
mod handlers;
mod models;
mod services;

use crate::{
    crypto::eip712::Eip712Verifier,
    handlers::relay::{
        domain_handler, get_nonce_handler, health_handler, job_status_handler,
        relay_handler, AppState,
    },
    models::Eip712Domain,
    services::{
        nonce_manager::NonceService,
        policy::{AllowedTargets, Policy, PolicyEnforcer},
        queue::{spawn_worker_pool, ProcessorContext, RelayQueue},
        relay_executor::{RelayExecutor, RelayExecutorConfig},
    },
};

use anyhow::{Context, Result};
use axum::{
    routing::{get, post},
    Router,
};
use alloy::{
    providers::ProviderBuilder,
    signers::local::PrivateKeySigner,
    primitives::{Address, U256},
};
use std::sync::Arc;
use std::str::FromStr;
use tower_http::{
    cors::{Any, CorsLayer},
    limit::RequestBodyLimitLayer,
    trace::TraceLayer,
};
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("gas_relayer=debug,tower_http=info")),
        )
        .with_target(true)
        .with_level(true)
        .init();

    info!(version = env!("CARGO_PKG_VERSION"), "🚀 Gas Relayer starting");

    let chain_id: u64 = std::env::var("CHAIN_ID")
        .unwrap_or_else(|_| "1".to_string())
        .parse()
        .context("invalid CHAIN_ID")?;

    let rpc_url = std::env::var("RPC_URL")
        .unwrap_or_else(|_| "http://localhost:8545".to_string());

    let forwarder_address = std::env::var("FORWARDER_ADDRESS")
        .unwrap_or_else(|_| "0x0000000000000000000000000000000000000001".to_string());

    let private_key = std::env::var("RELAYER_PRIVATE_KEY")
        .unwrap_or_else(|_| {
            "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80".to_string()
        });

    let domain_name = std::env::var("DOMAIN_NAME")
        .unwrap_or_else(|_| "GasRelayer".to_string());

    let rpc_url_parsed = rpc_url.parse::<url::Url>().context("invalid RPC URL")?;
    let provider = Arc::new(
        ProviderBuilder::new()
            .on_http(rpc_url_parsed)
            .boxed()
    );
    info!(rpc_url = %rpc_url, chain_id, "connected to RPC");

    let signer: PrivateKeySigner = private_key.parse().context("invalid private key")?;
    info!(relayer = ?signer.address(), "relayer wallet loaded");

    let forwarder_addr = Address::from_str(&forwarder_address).context("invalid forwarder address")?;

    let domain = Eip712Domain {
        name: domain_name.clone(),
        version: "1".to_string(),
        chain_id,
        verifying_contract: forwarder_addr,
    };
    let verifier = Arc::new(Eip712Verifier::new(domain));
    info!(domain_name = %domain_name, forwarder = ?forwarder_addr, "EIP-712 verifier initialized");

    let nonce_service = Arc::new(NonceService::new(
        provider.clone(),
        forwarder_addr,
        signer.address(),
        chain_id,
    ));

    let policy = Policy {
        id: uuid::Uuid::new_v4(),
        name: "Default Policy".to_string(),
        api_key_hash: "placeholder".to_string(),
        chain_id,
        forwarder_address: forwarder_addr,
        allowed_targets: AllowedTargets::Any,
        daily_gas_quota_per_user: Some(U256::from(5_000_000u64)),
        max_gas_per_request: U256::from(500_000u64),
        active: true,
        allowed_selectors: vec![],
        rate_limit_per_user_per_minute: 10,
        relayer_address: signer.address(),
    };
    let policy_enforcer = Arc::new(PolicyEnforcer::new(policy));

    let executor = Arc::new(RelayExecutor::new(
        provider.clone(),
        signer,
        RelayExecutorConfig {
            forwarder_address: forwarder_addr,
            chain_id,
            confirmation_timeout_secs: 120,
            confirmations_required: 1,
            max_gas_price_gwei: 200,
        },
    ));

    let (queue, receiver) = RelayQueue::new(1000);
    let queue = Arc::new(queue);

    let processor_ctx = Arc::new(ProcessorContext {
        executor: executor.clone(),
        nonce_service: nonce_service.clone(),
        policy_enforcer: policy_enforcer.clone(),
    });

    spawn_worker_pool(receiver, processor_ctx, 4).await;

    let app_state = AppState {
        verifier,
        policy_enforcer,
        nonce_service,
        relay_queue: queue,
    };

    let cors = CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any);

    let app = Router::new()
        .route("/relay", post(relay_handler))
        .route("/relay/:id", get(job_status_handler))
        .route("/nonce/:chain_id/:address", get(get_nonce_handler))
        .route("/domain/:chain_id", get(domain_handler))
        .route("/health", get(health_handler))
        .with_state(app_state)
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .layer(RequestBodyLimitLayer::new(1024 * 1024));

    let bind_addr = format!(
        "{}:{}",
        std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
        std::env::var("PORT").unwrap_or_else(|_| "8080".to_string()),
    );

    let listener = tokio::net::TcpListener::bind(&bind_addr).await.context("failed to bind")?;
    info!(addr = %bind_addr, "starting HTTP server");
    axum::serve(listener, app).await.context("server error")?;

    Ok(())
}
