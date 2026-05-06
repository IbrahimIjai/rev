use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20250101_000001_create_schema"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Businesses: Safe wallet owners who register on the platform
CREATE TABLE businesses (
    id           UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    safe_address TEXT NOT NULL UNIQUE,
    name         TEXT NOT NULL,
    email        TEXT,
    tier         TEXT NOT NULL DEFAULT 'free',
    active       BOOLEAN NOT NULL DEFAULT true,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Projects: one per dApp + chain combination
CREATE TABLE projects (
    id                UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    business_id       UUID NOT NULL REFERENCES businesses(id),
    name              TEXT NOT NULL,
    chain_id          BIGINT NOT NULL,
    forwarder_address TEXT NOT NULL,
    active            BOOLEAN NOT NULL DEFAULT true,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_projects_business_id ON projects(business_id);

-- Relayer wallets: hot wallets that sign and submit txs on-chain (one per project)
CREATE TABLE relayer_wallets (
    id            UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    project_id    UUID NOT NULL REFERENCES projects(id),
    address       TEXT NOT NULL UNIQUE,
    chain_id      BIGINT NOT NULL,
    key_reference TEXT NOT NULL,
    key_provider  TEXT NOT NULL DEFAULT 'local_dev',
    active        BOOLEAN NOT NULL DEFAULT true,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_relayer_wallets_project_id ON relayer_wallets(project_id);

-- Gas tanks: funded by businesses, auto-top-up to relayer wallets
CREATE TABLE gas_tanks (
    id                    UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    project_id            UUID NOT NULL REFERENCES projects(id),
    chain_id              BIGINT NOT NULL,
    address               TEXT NOT NULL UNIQUE,
    key_reference         TEXT NOT NULL,
    key_provider          TEXT NOT NULL DEFAULT 'local_dev',
    alert_threshold_wei   NUMERIC NOT NULL DEFAULT 100000000000000000,
    active                BOOLEAN NOT NULL DEFAULT true,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_gas_tanks_project_id ON gas_tanks(project_id);

-- Spending limits and relay policy per project
CREATE TABLE spending_limits (
    project_id               UUID PRIMARY KEY REFERENCES projects(id),
    daily_gas_quota_per_user NUMERIC,
    max_gas_per_request      NUMERIC NOT NULL DEFAULT 500000,
    max_gas_price_gwei       INTEGER NOT NULL DEFAULT 150,
    rate_limit_per_minute    INTEGER NOT NULL DEFAULT 10,
    allowed_targets          JSONB NOT NULL DEFAULT '{"type":"any"}',
    allowed_selectors        JSONB NOT NULL DEFAULT '[]',
    webhook_url              TEXT,
    updated_at               TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- API keys used by dApps to authenticate relay requests
CREATE TABLE api_keys (
    id           UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    project_id   UUID NOT NULL REFERENCES projects(id),
    key_hash     TEXT NOT NULL UNIQUE,
    name         TEXT NOT NULL DEFAULT 'default',
    active       BOOLEAN NOT NULL DEFAULT true,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_used_at TIMESTAMPTZ
);
CREATE INDEX idx_api_keys_project_id ON api_keys(project_id);
CREATE INDEX idx_api_keys_key_hash ON api_keys(key_hash);

-- Relay job status enum
CREATE TYPE job_status AS ENUM (
    'pending','queued','processing','submitted','confirmed','failed','reverted'
);

-- Relay jobs: full lifecycle tracking
CREATE TABLE relay_jobs (
    id                  UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    project_id          UUID NOT NULL REFERENCES projects(id),
    chain_id            BIGINT NOT NULL,
    req_from            TEXT NOT NULL,
    req_to              TEXT NOT NULL,
    req_value           NUMERIC NOT NULL DEFAULT 0,
    req_gas             NUMERIC NOT NULL,
    req_deadline        NUMERIC NOT NULL,
    req_data            TEXT NOT NULL,
    signature           TEXT NOT NULL,
    status              job_status NOT NULL DEFAULT 'pending',
    attempts            INTEGER NOT NULL DEFAULT 0,
    error               TEXT,
    tx_hash             TEXT,
    block_number        BIGINT,
    gas_used            NUMERIC,
    effective_gas_price NUMERIC,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    submitted_at        TIMESTAMPTZ,
    confirmed_at        TIMESTAMPTZ,
    next_attempt_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_relay_jobs_status ON relay_jobs(status);
CREATE INDEX idx_relay_jobs_project_id ON relay_jobs(project_id);
CREATE INDEX idx_relay_jobs_req_from ON relay_jobs(req_from);
CREATE INDEX idx_relay_jobs_tx_hash ON relay_jobs(tx_hash);
CREATE INDEX idx_relay_jobs_next_attempt ON relay_jobs(next_attempt_at)
    WHERE status IN ('queued', 'pending');

-- Per-project, per-user daily gas usage for quota enforcement
CREATE TABLE gas_usage (
    id           BIGSERIAL PRIMARY KEY,
    project_id   UUID NOT NULL REFERENCES projects(id),
    user_address TEXT NOT NULL,
    gas_used     NUMERIC NOT NULL,
    day_date     DATE NOT NULL DEFAULT CURRENT_DATE,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE UNIQUE INDEX idx_gas_usage_unique ON gas_usage(project_id, user_address, day_date);

-- Banned users per project
CREATE TABLE banned_users (
    id           BIGSERIAL PRIMARY KEY,
    project_id   UUID NOT NULL REFERENCES projects(id),
    user_address TEXT NOT NULL,
    reason       TEXT,
    banned_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(project_id, user_address)
);

-- Relayer wallet nonce tracking across restarts
CREATE TABLE relayer_nonces (
    wallet_address TEXT NOT NULL,
    chain_id       BIGINT NOT NULL,
    next_nonce     BIGINT NOT NULL DEFAULT 0,
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (wallet_address, chain_id)
);

-- Auto-update updated_at on relay_jobs
CREATE OR REPLACE FUNCTION update_updated_at()
RETURNS TRIGGER AS $$
BEGIN NEW.updated_at = NOW(); RETURN NEW; END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER relay_jobs_updated_at
    BEFORE UPDATE ON relay_jobs
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER spending_limits_updated_at
    BEFORE UPDATE ON spending_limits
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();
                "#,
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                r#"
DROP TABLE IF EXISTS relayer_nonces CASCADE;
DROP TABLE IF EXISTS banned_users CASCADE;
DROP TABLE IF EXISTS gas_usage CASCADE;
DROP TABLE IF EXISTS relay_jobs CASCADE;
DROP TYPE IF EXISTS job_status CASCADE;
DROP TABLE IF EXISTS api_keys CASCADE;
DROP TABLE IF EXISTS spending_limits CASCADE;
DROP TABLE IF EXISTS gas_tanks CASCADE;
DROP TABLE IF EXISTS relayer_wallets CASCADE;
DROP TABLE IF EXISTS projects CASCADE;
DROP TABLE IF EXISTS businesses CASCADE;
                "#,
            )
            .await?;
        Ok(())
    }
}
