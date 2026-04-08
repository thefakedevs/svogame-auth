use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "service_token_audit")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub service_token_id: Uuid,
    pub action: String,
    pub actor_user_id: Uuid,
    pub reason: Option<String>,
    pub metadata: Option<String>,
    pub created_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
