use sea_orm::entity::prelude::*;
use rust_decimal::Decimal;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "spending_limits")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub project_id: Uuid,
    /// Max gas units per user per day (None = unlimited)
    pub daily_gas_quota_per_user: Option<Decimal>,
    pub max_gas_per_request: Decimal,
    pub max_gas_price_gwei: i32,
    pub rate_limit_per_minute: i32,
    /// JSON: {"type":"any"} or {"type":"allowlist","addresses":["0x..."]}
    pub allowed_targets: Json,
    /// JSON array of 4-byte hex selectors e.g. ["0xa9059cbb"]
    pub allowed_selectors: Json,
    pub webhook_url: Option<String>,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::project::Entity",
        from = "Column::ProjectId",
        to = "super::project::Column::Id"
    )]
    Project,
}

impl Related<super::project::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Project.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
