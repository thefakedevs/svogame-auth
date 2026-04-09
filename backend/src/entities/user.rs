use anyhow::Result;
use sea_orm::entity::prelude::*;
use sea_orm::{ActiveModelBehavior, ActiveValue, DatabaseConnection, DeriveRelation, EnumIter};
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
    pub is_superuser: bool,
    pub squad_id: Option<Uuid>,
    pub deactivation_reason: Option<String>,
    pub last_login_at: chrono::DateTime<chrono::Utc>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

pub struct UpsertUserResult {
    pub user: Model,
    pub created: bool,
}

impl Entity {
    pub async fn update_or_register_by_discord_id(
        db: &DatabaseConnection,
        discord_id: String,
        username: String,
        avatar_url: Option<String>,
        email: Option<String>,
    ) -> Result<UpsertUserResult> {
        if let Some(existing_user) = Self::find()
            .filter(Column::DiscordId.eq(&discord_id))
            .one(db)
            .await?
        {
            let mut user: ActiveModel = existing_user.into();
            user.avatar_url = ActiveValue::Set(avatar_url);
            user.email = ActiveValue::Set(email);
            user.last_login_at = ActiveValue::Set(chrono::Utc::now());

            let updated_user = user.update(db).await?;
            Ok(UpsertUserResult {
                user: updated_user,
                created: false,
            })
        } else {
            let username =
                if is_valid_nickname(&username) && !is_nickname_taken(db, &username).await? {
                    username
                } else {
                    generate_unique_nickname(db).await?
                };
            let new_user = ActiveModel {
                id: ActiveValue::Set(Uuid::new_v4()),
                discord_id: ActiveValue::Set(discord_id.clone()),
                username: ActiveValue::Set(username),
                avatar_url: ActiveValue::Set(avatar_url),
                email: ActiveValue::Set(email),
                auth_epoch: ActiveValue::Set(0),
                is_active: ActiveValue::Set(true),
                is_superuser: ActiveValue::Set(false),
                squad_id: ActiveValue::Set(None),
                deactivation_reason: ActiveValue::Set(None),
                last_login_at: ActiveValue::Set(chrono::Utc::now()),
                created_at: ActiveValue::Set(chrono::Utc::now()),
            };

            let created_user = new_user.insert(db).await?;
            Ok(UpsertUserResult {
                user: created_user,
                created: true,
            })
        }
    }
}

pub const NICKNAME_REGEX: &str = r"^[a-zA-Z0-9_]{3,16}$";

fn is_valid_nickname(nickname: &str) -> bool {
    let nickname_regex = regex::Regex::new(NICKNAME_REGEX).unwrap();
    nickname_regex.is_match(nickname)
}

async fn is_nickname_taken(db: &DatabaseConnection, nickname: &str) -> Result<bool> {
    use sea_orm::EntityTrait;

    let count = Entity::find()
        .filter(Column::Username.eq(nickname))
        .count(db)
        .await?;

    Ok(count > 0)
}

async fn generate_unique_nickname(db: &DatabaseConnection) -> Result<String> {
    let mut tries = 0;
    loop {
        let nickname = crate::util::nickname::random_nickname();
        if !is_nickname_taken(db, &nickname).await? {
            return Ok(nickname);
        }
        tries += 1;
        if tries >= 5 {
            return Err(anyhow::anyhow!(
                "Failed to generate unique nickname after 5 tries"
            ));
        }
    }
}
