use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "projects")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub business_id: Uuid,
    pub name: String,
    pub chain_id: i64,
    pub forwarder_address: String,
    pub active: bool,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::business::Entity",
        from = "Column::BusinessId",
        to = "super::business::Column::Id"
    )]
    Business,
    #[sea_orm(has_many = "super::relayer_wallet::Entity")]
    RelayerWallets,
    #[sea_orm(has_many = "super::gas_tank::Entity")]
    GasTanks,
    #[sea_orm(has_one = "super::spending_limit::Entity")]
    SpendingLimit,
    #[sea_orm(has_many = "super::api_key::Entity")]
    ApiKeys,
    #[sea_orm(has_many = "super::relay_job::Entity")]
    RelayJobs,
}

impl Related<super::business::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Business.def()
    }
}

impl Related<super::relayer_wallet::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::RelayerWallets.def()
    }
}

impl Related<super::gas_tank::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::GasTanks.def()
    }
}

impl Related<super::spending_limit::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SpendingLimit.def()
    }
}

impl Related<super::api_key::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ApiKeys.def()
    }
}

impl Related<super::relay_job::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::RelayJobs.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
