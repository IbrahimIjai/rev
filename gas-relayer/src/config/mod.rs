use anyhow::Result;
use alloy::primitives::Address;
use serde::Deserialize;
use std::str::FromStr;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub server: ServerConfig,
    pub chains: Vec<ChainConfig>,
    pub relayer: RelayerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub policy: PolicyConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    /// CORS allowed origins
    pub allowed_origins: Vec<String>,
    /// Request size limit in bytes
    pub max_body_size: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8080,
            allowed_origins: vec!["*".to_string()],
            max_body_size: 1024 * 1024, // 1MB
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct ChainConfig {
    pub chain_id: u64,
    pub name: String,
    /// HTTP RPC endpoint
    pub rpc_url: String,
    /// Optional WebSocket RPC for event streaming
    pub ws_url: Option<String>,
    /// MinimalForwarder contract address
    pub forwarder_address: String,
    /// EIP-712 domain name
    pub domain_name: String,
    /// EIP-712 domain version
    pub domain_version: String,
    /// Max gas price in gwei (circuit breaker)
    pub max_gas_price_gwei: u64,
    /// Required block confirmations
    pub confirmations: u64,
    /// TX confirmation timeout in seconds
    pub confirmation_timeout_secs: u64,
}

impl ChainConfig {
    pub fn forwarder_address(&self) -> Result<Address> {
        Address::from_str(&self.forwarder_address)
            .map_err(|e| anyhow::anyhow!("invalid forwarder address: {}", e))
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct RelayerConfig {
    /// Private key of the relayer wallet (use HSM/KMS in production!)
    /// Format: hex string with or without 0x prefix
    pub private_key: String,
    /// Number of concurrent worker tasks
    pub worker_count: usize,
    /// Queue capacity
    pub queue_capacity: usize,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "postgresql://localhost/gas_relayer".to_string(),
            max_connections: 20,
            min_connections: 2,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct RedisConfig {
    pub url: String,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: "redis://localhost:6379".to_string(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct PolicyConfig {
    /// Default daily gas quota per user (in gas units)
    pub default_daily_gas_quota: Option<u64>,
    /// Default max gas per single request
    pub default_max_gas_per_request: u64,
    /// Default rate limit: requests per user per minute
    pub default_rate_limit_per_minute: u32,
}

impl Default for PolicyConfig {
    fn default() -> Self {
        Self {
            default_daily_gas_quota: Some(5_000_000),
            default_max_gas_per_request: 500_000,
            default_rate_limit_per_minute: 10,
        }
    }
}

/// Load configuration from environment and optional config file.
pub fn load_config() -> Result<Config> {
    let cfg = config::Config::builder()
        // Load from config.yaml if present
        .add_source(
            config::File::with_name("config")
                .required(false)
                .format(config::FileFormat::Yaml),
        )
        // Environment variables override file config
        // Prefix: RELAYER_ (e.g. RELAYER_SERVER__PORT=9090)
        .add_source(
            config::Environment::with_prefix("RELAYER")
                .separator("__")
                .try_parsing(true),
        )
        .build()?;

    Ok(cfg.try_deserialize()?)
}

/// Example config.yaml content for reference
pub const EXAMPLE_CONFIG: &str = r#"
server:
  host: "0.0.0.0"
  port: 8080
  allowed_origins: ["https://myapp.com"]
  max_body_size: 1048576

chains:
  - chain_id: 1
    name: "Ethereum Mainnet"
    rpc_url: "https://eth-mainnet.g.alchemy.com/v2/YOUR_KEY"
    ws_url: "wss://eth-mainnet.g.alchemy.com/v2/YOUR_KEY"
    forwarder_address: "0xYOUR_FORWARDER_CONTRACT"
    domain_name: "MyDAppRelayer"
    domain_version: "1"
    max_gas_price_gwei: 100
    confirmations: 2
    confirmation_timeout_secs: 120

  - chain_id: 137
    name: "Polygon"
    rpc_url: "https://polygon-mainnet.g.alchemy.com/v2/YOUR_KEY"
    forwarder_address: "0xYOUR_POLYGON_FORWARDER"
    domain_name: "MyDAppRelayer"
    domain_version: "1"
    max_gas_price_gwei: 500
    confirmations: 5
    confirmation_timeout_secs: 60

relayer:
  private_key: "${RELAYER_PRIVATE_KEY}"  # from env
  worker_count: 4
  queue_capacity: 1000

database:
  url: "${DATABASE_URL}"
  max_connections: 20
  min_connections: 2

redis:
  url: "${REDIS_URL}"

policy:
  default_daily_gas_quota: 5000000
  default_max_gas_per_request: 500000
  default_rate_limit_per_minute: 10
"#;
