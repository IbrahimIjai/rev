-- migrations/001_initial.sql
-- Gas Relayer Database Schema

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- ─────────────────────────────────────────────
-- Teams / API Keys
-- ─────────────────────────────────────────────

CREATE TABLE teams (
    id            UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name          TEXT NOT NULL,
    api_key_hash  TEXT NOT NULL UNIQUE,  -- sha256 of the raw API key
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    active        BOOLEAN NOT NULL DEFAULT TRUE
);

-- ─────────────────────────────────────────────
-- Policies (per team, per chain)
-- ─────────────────────────────────────────────

CREATE TABLE policies (
    id                          UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    team_id                     UUID NOT NULL REFERENCES teams(id),
    name                        TEXT NOT NULL,
    chain_id                    BIGINT NOT NULL,
    forwarder_address           TEXT NOT NULL,
    relayer_address             TEXT NOT NULL,
    allowed_targets             JSONB NOT NULL DEFAULT '{"type": "any"}',
    allowed_selectors           TEXT[] NOT NULL DEFAULT '{}',
    daily_gas_quota_per_user    NUMERIC,
    max_gas_per_request         NUMERIC NOT NULL DEFAULT 500000,
    rate_limit_per_minute       INTEGER NOT NULL DEFAULT 10,
    active                      BOOLEAN NOT NULL DEFAULT TRUE,
    created_at                  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at                  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_policies_team_id ON policies(team_id);
CREATE INDEX idx_policies_chain_id ON policies(chain_id);

-- ─────────────────────────────────────────────
-- Relay Jobs
-- ─────────────────────────────────────────────

CREATE TYPE job_status AS ENUM (
    'pending',
    'queued',
    'processing',
    'submitted',
    'confirmed',
    'failed',
    'reverted'
);

CREATE TABLE relay_jobs (
    id                  UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    policy_id           UUID NOT NULL REFERENCES policies(id),
    chain_id            BIGINT NOT NULL,

    -- ForwardRequest fields
    req_from            TEXT NOT NULL,
    req_to              TEXT NOT NULL,
    req_value           NUMERIC NOT NULL DEFAULT 0,
    req_gas             NUMERIC NOT NULL,
    req_nonce           NUMERIC NOT NULL,
    req_deadline        NUMERIC,
    req_data            TEXT NOT NULL,  -- hex-encoded
    signature           TEXT NOT NULL,  -- hex-encoded 65-byte sig

    -- Lifecycle
    status              job_status NOT NULL DEFAULT 'pending',
    attempts            INTEGER NOT NULL DEFAULT 0,
    error               TEXT,

    -- On-chain results
    tx_hash             TEXT,
    block_number        BIGINT,
    gas_used            NUMERIC,
    effective_gas_price NUMERIC,

    -- Timestamps
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    submitted_at        TIMESTAMPTZ,
    confirmed_at        TIMESTAMPTZ,
    next_attempt_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_relay_jobs_status ON relay_jobs(status);
CREATE INDEX idx_relay_jobs_req_from ON relay_jobs(req_from);
CREATE INDEX idx_relay_jobs_chain_id ON relay_jobs(chain_id);
CREATE INDEX idx_relay_jobs_tx_hash ON relay_jobs(tx_hash);
CREATE INDEX idx_relay_jobs_next_attempt ON relay_jobs(next_attempt_at)
    WHERE status IN ('queued', 'pending');

-- ─────────────────────────────────────────────
-- Gas Usage Tracking (for quota enforcement)
-- ─────────────────────────────────────────────

CREATE TABLE gas_usage (
    id          BIGSERIAL PRIMARY KEY,
    policy_id   UUID NOT NULL REFERENCES policies(id),
    user_address TEXT NOT NULL,
    gas_used    NUMERIC NOT NULL,
    day_date    DATE NOT NULL DEFAULT CURRENT_DATE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX idx_gas_usage_unique ON gas_usage(policy_id, user_address, day_date);

-- Daily totals view
CREATE VIEW gas_usage_daily AS
SELECT
    policy_id,
    user_address,
    day_date,
    SUM(gas_used) AS total_gas
FROM gas_usage
GROUP BY policy_id, user_address, day_date;

-- ─────────────────────────────────────────────
-- Banned Users
-- ─────────────────────────────────────────────

CREATE TABLE banned_users (
    id           BIGSERIAL PRIMARY KEY,
    policy_id    UUID NOT NULL REFERENCES policies(id),
    user_address TEXT NOT NULL,
    reason       TEXT,
    banned_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(policy_id, user_address)
);

-- ─────────────────────────────────────────────
-- Relayer Wallets (encrypted key storage reference)
-- ─────────────────────────────────────────────
-- NOTE: Private keys are NEVER stored here.
-- This table stores metadata only. Keys live in AWS KMS / HashiCorp Vault.

CREATE TABLE relayer_wallets (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    address         TEXT NOT NULL UNIQUE,
    chain_id        BIGINT NOT NULL,
    -- Reference to the KMS key ARN or Vault path
    -- e.g. "arn:aws:kms:us-east-1:123456789:key/abc123"
    -- or   "vault:secret/relayers/mainnet"
    key_reference   TEXT NOT NULL,
    key_provider    TEXT NOT NULL CHECK (key_provider IN ('aws_kms', 'hashicorp_vault', 'gcp_kms', 'local_dev')),
    active          BOOLEAN NOT NULL DEFAULT TRUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ─────────────────────────────────────────────
-- Nonce Tracking (relayer wallet nonces)
-- ─────────────────────────────────────────────

CREATE TABLE relayer_nonces (
    wallet_address  TEXT NOT NULL,
    chain_id        BIGINT NOT NULL,
    next_nonce      BIGINT NOT NULL DEFAULT 0,
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (wallet_address, chain_id)
);

-- Auto-update updated_at trigger
CREATE OR REPLACE FUNCTION update_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER relay_jobs_updated_at
    BEFORE UPDATE ON relay_jobs
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER policies_updated_at
    BEFORE UPDATE ON policies
    FOR EACH ROW EXECUTE FUNCTION update_updated_at();
