use anyhow::Result;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter};
use serde::Serialize;
use uuid::Uuid;

use crate::entities::{UserRestriction, UserRestrictionColumn, UserRestrictionModel};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RestrictionKind {
    CreateSquad,
    JoinSquad,
    UploadSkin,
}

impl RestrictionKind {
    pub const ALL: [RestrictionKind; 3] = [
        RestrictionKind::CreateSquad,
        RestrictionKind::JoinSquad,
        RestrictionKind::UploadSkin,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            RestrictionKind::CreateSquad => "create_squad",
            RestrictionKind::JoinSquad => "join_squad",
            RestrictionKind::UploadSkin => "upload_skin",
        }
    }

    pub fn title_en(self) -> &'static str {
        match self {
            RestrictionKind::CreateSquad => "Squad creation restricted",
            RestrictionKind::JoinSquad => "Squad joining restricted",
            RestrictionKind::UploadSkin => "Skin uploads restricted",
        }
    }

    pub fn description_en(self) -> &'static str {
        match self {
            RestrictionKind::CreateSquad => {
                "User cannot create new squads until this restriction is removed."
            }
            RestrictionKind::JoinSquad => {
                "User cannot accept invites or join squads until this restriction is removed."
            }
            RestrictionKind::UploadSkin => {
                "User cannot upload or replace their personal skin until this restriction is removed."
            }
        }
    }
}

impl std::str::FromStr for RestrictionKind {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "create_squad" => Ok(Self::CreateSquad),
            "join_squad" => Ok(Self::JoinSquad),
            "upload_skin" => Ok(Self::UploadSkin),
            _ => Err(anyhow::anyhow!("Unknown restriction key")),
        }
    }
}

pub async fn list_user_restrictions<C>(db: &C, user_id: Uuid) -> Result<Vec<UserRestrictionModel>>
where
    C: ConnectionTrait,
{
    Ok(UserRestriction::find()
        .filter(UserRestrictionColumn::UserId.eq(user_id))
        .all(db)
        .await?)
}

pub async fn has_restriction<C>(db: &C, user_id: Uuid, kind: RestrictionKind) -> Result<bool>
where
    C: ConnectionTrait,
{
    let count = UserRestriction::find()
        .filter(UserRestrictionColumn::UserId.eq(user_id))
        .filter(UserRestrictionColumn::RestrictionKey.eq(kind.as_str()))
        .count(db)
        .await?;

    Ok(count > 0)
}
