use sea_orm::entity::prelude::*;
use rust_decimal::Decimal;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "relay_jobs")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub project_id: Uuid,
    pub chain_id: i64,
    pub req_from: String,
    pub req_to: String,
    pub req_value: Decimal,
    pub req_gas: Decimal,
    pub req_deadline: Decimal,
    pub req_data: String,
    pub signature: String,
    pub status: String,
    pub attempts: i32,
    pub error: Option<String>,
    pub tx_hash: Option<String>,
    pub block_number: Option<i64>,
    pub gas_used: Option<Decimal>,
    pub effective_gas_price: Option<Decimal>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    pub submitted_at: Option<DateTimeWithTimeZone>,
    pub confirmed_at: Option<DateTimeWithTimeZone>,
    pub next_attempt_at: DateTimeWithTimeZone,
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
