use crate::{
    db::{
        self,
        entities::{api_key, gas_tank, project, relayer_wallet, spending_limit},
        Db,
    },
    handlers::auth::{decode_jwt, Claims},
};
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone)]
pub struct ProjectsState {
    pub db: Db,
    pub jwt_secret: String,
    pub encryption_secret: String,
}

fn extract_claims(headers: &HeaderMap, jwt_secret: &str) -> Result<Claims, &'static str> {
    let auth = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or("missing Authorization header")?;
    let token = auth.strip_prefix("Bearer ").ok_or("expected Bearer token")?;
    decode_jwt(token, jwt_secret).map_err(|_| "invalid token")
}

// ─── Request / Response types ───────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectRequest {
    pub name: String,
    pub chain_id: u64,
    /// Address of the deployed OZ ERC2771Forwarder on this chain
    pub forwarder_address: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectResponse {
    pub project_id: String,
    pub gas_tank_address: String,
    pub relayer_address: String,
    /// Raw API key — shown ONCE, never stored in plaintext
    pub api_key: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSummary {
    pub id: String,
    pub name: String,
    pub chain_id: i64,
    pub forwarder_address: String,
    pub active: bool,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateLimitsRequest {
    pub daily_gas_quota_per_user: Option<String>,
    pub max_gas_per_request: Option<String>,
    pub max_gas_price_gwei: Option<i32>,
    pub rate_limit_per_minute: Option<i32>,
    pub allowed_targets: Option<serde_json::Value>,
    pub allowed_selectors: Option<serde_json::Value>,
    pub webhook_url: Option<String>,
}

// ─── Handlers ───────────────────────────────────────────────────────────────

/// POST /api/projects
pub async fn create_project(
    State(state): State<ProjectsState>,
    headers: HeaderMap,
    Json(body): Json<CreateProjectRequest>,
) -> impl IntoResponse {
    let claims = match extract_claims(&headers, &state.jwt_secret) {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": e})),
            )
                .into_response()
        }
    };

    let business_id: Uuid = match claims.sub.parse() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "invalid business id in token"})),
            )
                .into_response()
        }
    };

    let now = Utc::now().fixed_offset();
    let project_id = Uuid::new_v4();

    // 1. Create project
    let proj = project::ActiveModel {
        id: Set(project_id),
        business_id: Set(business_id),
        name: Set(body.name),
        chain_id: Set(body.chain_id as i64),
        forwarder_address: Set(body.forwarder_address),
        active: Set(true),
        created_at: Set(now),
    };
    if let Err(e) = proj.insert(&state.db).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("failed to create project: {e}")})),
        )
            .into_response();
    }

    // 2. Generate and store relayer wallet
    let (relayer_addr, relayer_privkey) = db::generate_wallet();
    let encrypted_relayer = match db::keys::encrypt_key(&relayer_privkey, &state.encryption_secret) {
        Ok(e) => e,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("key encryption failed: {e}")})),
            )
                .into_response()
        }
    };
    let rw = relayer_wallet::ActiveModel {
        id: Set(Uuid::new_v4()),
        project_id: Set(project_id),
        address: Set(relayer_addr.clone()),
        chain_id: Set(body.chain_id as i64),
        key_reference: Set(encrypted_relayer),
        key_provider: Set("local_dev".to_string()),
        active: Set(true),
        created_at: Set(now),
    };
    if let Err(e) = rw.insert(&state.db).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("failed to save relayer wallet: {e}")})),
        )
            .into_response();
    }

    // 3. Generate and store gas tank wallet
    let (tank_addr, tank_privkey) = db::generate_wallet();
    let encrypted_tank = match db::keys::encrypt_key(&tank_privkey, &state.encryption_secret) {
        Ok(e) => e,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": format!("key encryption failed: {e}")})),
            )
                .into_response()
        }
    };
    let gt = gas_tank::ActiveModel {
        id: Set(Uuid::new_v4()),
        project_id: Set(project_id),
        chain_id: Set(body.chain_id as i64),
        address: Set(tank_addr.clone()),
        key_reference: Set(encrypted_tank),
        key_provider: Set("local_dev".to_string()),
        alert_threshold_wei: Set("100000000000000000".parse().unwrap()), // 0.1 ETH
        active: Set(true),
        created_at: Set(now),
    };
    if let Err(e) = gt.insert(&state.db).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("failed to save gas tank: {e}")})),
        )
            .into_response();
    }

    // 4. Create default spending limits
    let sl = spending_limit::ActiveModel {
        project_id: Set(project_id),
        daily_gas_quota_per_user: Set(Some("5000000".parse().unwrap())),
        max_gas_per_request: Set("500000".parse().unwrap()),
        max_gas_price_gwei: Set(150),
        rate_limit_per_minute: Set(10),
        allowed_targets: Set(serde_json::json!({"type": "any"})),
        allowed_selectors: Set(serde_json::json!([])),
        webhook_url: Set(None),
        updated_at: Set(now),
    };
    if let Err(e) = sl.insert(&state.db).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("failed to save spending limits: {e}")})),
        )
            .into_response();
    }

    // 5. Generate API key (returned once, only hash stored)
    let raw_key = db::generate_api_key();
    let key_hash = db::hash_api_key(&raw_key);
    let ak = api_key::ActiveModel {
        id: Set(Uuid::new_v4()),
        project_id: Set(project_id),
        key_hash: Set(key_hash),
        name: Set("default".to_string()),
        active: Set(true),
        created_at: Set(now),
        last_used_at: Set(None),
    };
    if let Err(e) = ak.insert(&state.db).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("failed to save api key: {e}")})),
        )
            .into_response();
    }

    (
        StatusCode::CREATED,
        Json(CreateProjectResponse {
            project_id: project_id.to_string(),
            gas_tank_address: tank_addr,
            relayer_address: relayer_addr,
            api_key: raw_key,
        }),
    )
        .into_response()
}

/// GET /api/projects
pub async fn list_projects(
    State(state): State<ProjectsState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let claims = match extract_claims(&headers, &state.jwt_secret) {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": e})),
            )
                .into_response()
        }
    };

    let business_id: Uuid = match claims.sub.parse() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "invalid business id"})),
            )
                .into_response()
        }
    };

    match project::Entity::find()
        .filter(project::Column::BusinessId.eq(business_id))
        .all(&state.db)
        .await
    {
        Ok(projects) => {
            let summaries: Vec<ProjectSummary> = projects
                .into_iter()
                .map(|p| ProjectSummary {
                    id: p.id.to_string(),
                    name: p.name,
                    chain_id: p.chain_id,
                    forwarder_address: p.forwarder_address,
                    active: p.active,
                    created_at: p.created_at.to_rfc3339(),
                })
                .collect();
            (StatusCode::OK, Json(serde_json::json!(summaries))).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("{e}")})),
        )
            .into_response(),
    }
}

/// GET /api/projects/:id
pub async fn get_project(
    State(state): State<ProjectsState>,
    headers: HeaderMap,
    Path(project_id): Path<Uuid>,
) -> impl IntoResponse {
    let claims = match extract_claims(&headers, &state.jwt_secret) {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": e})),
            )
                .into_response()
        }
    };

    let business_id: Uuid = match claims.sub.parse() {
        Ok(id) => id,
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "invalid business id"}))).into_response()
        }
    };

    let proj = match project::Entity::find_by_id(project_id)
        .filter(project::Column::BusinessId.eq(business_id))
        .one(&state.db)
        .await
    {
        Ok(Some(p)) => p,
        Ok(None) => {
            return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "project not found"}))).into_response()
        }
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("{e}")}))).into_response()
        }
    };

    let gas_tank = gas_tank::Entity::find()
        .filter(gas_tank::Column::ProjectId.eq(project_id))
        .one(&state.db)
        .await
        .ok()
        .flatten();

    let relayer = relayer_wallet::Entity::find()
        .filter(relayer_wallet::Column::ProjectId.eq(project_id))
        .one(&state.db)
        .await
        .ok()
        .flatten();

    let limits = spending_limit::Entity::find_by_id(project_id)
        .one(&state.db)
        .await
        .ok()
        .flatten();

    (StatusCode::OK, Json(serde_json::json!({
        "id": proj.id,
        "name": proj.name,
        "chainId": proj.chain_id,
        "forwarderAddress": proj.forwarder_address,
        "active": proj.active,
        "createdAt": proj.created_at,
        "gasTankAddress": gas_tank.as_ref().map(|g| &g.address),
        "relayerAddress": relayer.as_ref().map(|r| &r.address),
        "spendingLimits": limits.as_ref().map(|l| serde_json::json!({
            "dailyGasQuotaPerUser": l.daily_gas_quota_per_user,
            "maxGasPerRequest": l.max_gas_per_request,
            "maxGasPriceGwei": l.max_gas_price_gwei,
            "rateLimitPerMinute": l.rate_limit_per_minute,
            "allowedTargets": l.allowed_targets,
            "allowedSelectors": l.allowed_selectors,
            "webhookUrl": l.webhook_url,
        })),
    }))).into_response()
}

/// PUT /api/projects/:id/limits
pub async fn update_limits(
    State(state): State<ProjectsState>,
    headers: HeaderMap,
    Path(project_id): Path<Uuid>,
    Json(body): Json<UpdateLimitsRequest>,
) -> impl IntoResponse {
    let claims = match extract_claims(&headers, &state.jwt_secret) {
        Ok(c) => c,
        Err(e) => {
            return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": e}))).into_response()
        }
    };

    let business_id: Uuid = match claims.sub.parse() {
        Ok(id) => id,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "invalid business id"}))).into_response(),
    };

    // Verify project belongs to this business
    match project::Entity::find_by_id(project_id)
        .filter(project::Column::BusinessId.eq(business_id))
        .one(&state.db)
        .await
    {
        Ok(Some(_)) => {}
        Ok(None) => return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "project not found"}))).into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("{e}")}))).into_response(),
    }

    let existing = match spending_limit::Entity::find_by_id(project_id)
        .one(&state.db)
        .await
    {
        Ok(Some(s)) => s,
        Ok(None) => return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "spending limits not found"}))).into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("{e}")}))).into_response(),
    };

    let now = Utc::now().fixed_offset();
    let mut active: spending_limit::ActiveModel = existing.into();

    if let Some(q) = body.daily_gas_quota_per_user {
        active.daily_gas_quota_per_user = Set(q.parse().ok());
    }
    if let Some(m) = body.max_gas_per_request {
        if let Ok(v) = m.parse() {
            active.max_gas_per_request = Set(v);
        }
    }
    if let Some(g) = body.max_gas_price_gwei {
        active.max_gas_price_gwei = Set(g);
    }
    if let Some(r) = body.rate_limit_per_minute {
        active.rate_limit_per_minute = Set(r);
    }
    if let Some(t) = body.allowed_targets {
        active.allowed_targets = Set(t);
    }
    if let Some(s) = body.allowed_selectors {
        active.allowed_selectors = Set(s);
    }
    active.webhook_url = Set(body.webhook_url);
    active.updated_at = Set(now);

    match active.update(&state.db).await {
        Ok(_) => (StatusCode::OK, Json(serde_json::json!({"status": "updated"}))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("{e}")}))).into_response(),
    }
}

/// POST /api/projects/:id/api-keys
pub async fn create_api_key(
    State(state): State<ProjectsState>,
    headers: HeaderMap,
    Path(project_id): Path<Uuid>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let claims = match extract_claims(&headers, &state.jwt_secret) {
        Ok(c) => c,
        Err(e) => return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": e}))).into_response(),
    };

    let business_id: Uuid = match claims.sub.parse() {
        Ok(id) => id,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "invalid business id"}))).into_response(),
    };

    match project::Entity::find_by_id(project_id)
        .filter(project::Column::BusinessId.eq(business_id))
        .one(&state.db)
        .await
    {
        Ok(Some(_)) => {}
        Ok(None) => return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "project not found"}))).into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("{e}")}))).into_response(),
    }

    let name = body.get("name").and_then(|n| n.as_str()).unwrap_or("default").to_string();
    let raw_key = db::generate_api_key();
    let key_hash = db::hash_api_key(&raw_key);
    let now = Utc::now().fixed_offset();

    let ak = api_key::ActiveModel {
        id: Set(Uuid::new_v4()),
        project_id: Set(project_id),
        key_hash: Set(key_hash),
        name: Set(name),
        active: Set(true),
        created_at: Set(now),
        last_used_at: Set(None),
    };

    match ak.insert(&state.db).await {
        Ok(model) => (StatusCode::CREATED, Json(serde_json::json!({
            "id": model.id,
            "apiKey": raw_key,
            "name": model.name,
            "createdAt": model.created_at,
        }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("{e}")}))).into_response(),
    }
}
