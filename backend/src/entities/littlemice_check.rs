use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "littlemice_check")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub player_uuid: Uuid,
    pub service_token_id: Uuid,
    pub service_system_name: String,
    pub status: String,
    pub failure_reason: Option<String>,
    pub push_token_hash: Option<String>,
    pub push_token_expires_at: DateTimeUtc,
    pub requested_at: DateTimeUtc,
    pub received_at: Option<DateTimeUtc>,
    pub completed_at: Option<DateTimeUtc>,
    pub screenshot_s3_key: Option<String>,
    pub screenshot_content_type: Option<String>,
    pub screenshot_size_bytes: Option<i64>,
    pub screenshot2_s3_key: Option<String>,
    pub screenshot2_content_type: Option<String>,
    pub screenshot2_size_bytes: Option<i64>,
    pub log_s3_key: Option<String>,
    pub log_content_type: Option<String>,
    pub log_size_bytes: Option<i64>,
    pub client_info_text: Option<String>,
    pub client_info_size_bytes: Option<i64>,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
