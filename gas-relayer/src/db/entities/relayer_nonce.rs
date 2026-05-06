use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "relayer_nonces")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub wallet_address: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub chain_id: i64,
    pub next_nonce: i64,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
