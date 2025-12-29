use sea_orm::entity::prelude::*;
use sea_orm::{ActiveModelBehavior, ActiveValue, DatabaseConnection, DeriveRelation, EnumIter};
use anyhow::Result;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "user")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub discord_id: String,
    pub username: String,
    pub avatar_url: Option<String>,
    pub email: Option<String>,
    pub auth_epoch: i32,
    pub is_active: bool,
    pub last_login_at: chrono::DateTime<chrono::Utc>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl Entity {
    pub async fn update_or_register_by_discord_id(
        db: &DatabaseConnection,
        discord_id: String,
        username: String,
        avatar_url: Option<String>,
        email: Option<String>,
    ) -> Result<Model> {
        if let Some(existing_user) = Self::find()
            .filter(Column::DiscordId.eq(&discord_id))
            .one(db)
            .await?
        {
            let mut user: ActiveModel = existing_user.into();
            user.username = ActiveValue::Set(username);
            user.avatar_url = ActiveValue::Set(avatar_url);
            user.email = ActiveValue::Set(email);
            user.last_login_at = ActiveValue::Set(chrono::Utc::now());

            let updated_user = user.update(db).await?;
            Ok(updated_user)
        } else {
            let new_user = ActiveModel {
                id: ActiveValue::Set(Uuid::new_v4()),
                discord_id: ActiveValue::Set(discord_id),
                username: ActiveValue::Set(username),
                avatar_url: ActiveValue::Set(avatar_url),
                email: ActiveValue::Set(email),
                auth_epoch: ActiveValue::Set(0),
                is_active: ActiveValue::Set(true),
                last_login_at: ActiveValue::Set(chrono::Utc::now()),
                created_at: ActiveValue::Set(chrono::Utc::now()),
            };

            let created_user = new_user.insert(db).await?;
            Ok(created_user)
        }
    }
}

