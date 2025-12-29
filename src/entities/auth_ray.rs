use chrono::Utc;
use sea_orm::entity::prelude::*;
use sea_orm::{ActiveValue, DatabaseConnection};
use anyhow::Result;
use crate::services::pow::generate_pow_prefix;

/// Способ доставки токена после авторизации
#[derive(Clone, Debug, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::None)")]
pub enum TokenDeliveryMethod {
    #[sea_orm(string_value = "redirect")]
    Redirect,
    #[sea_orm(string_value = "polling")]
    Polling,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "auth_ray")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub pow_prefix: String,
    pub pow_complexity: u8,
    pub delivery_method: TokenDeliveryMethod,
    pub delivery_target: String,
    pub created_at: chrono::DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl Entity {
    pub async fn create_with_complexity(
        db: &DatabaseConnection,
        pow_complexity: u8,
        delivery_method: TokenDeliveryMethod,
        delivery_target: String,
    ) -> Result<Model> {
        let pow_prefix = generate_pow_prefix();

        let new_auth_ray = ActiveModel {
            pow_prefix: ActiveValue::Set(pow_prefix),
            pow_complexity: ActiveValue::Set(pow_complexity),
            delivery_method: ActiveValue::Set(delivery_method),
            delivery_target: ActiveValue::Set(delivery_target),
            created_at: ActiveValue::Set(Utc::now()),
            ..Default::default()
        };

        let result = new_auth_ray.insert(db).await?;
        Ok(result)
    }

    pub async fn find_by_prefix(
        db: &DatabaseConnection,
        pow_prefix: &str,
    ) -> Result<Option<Model>> {
        let result = Self::find()
            .filter(Column::PowPrefix.eq(pow_prefix))
            .one(db)
            .await?;
        Ok(result)
    }

    pub async fn delete_older_than(
        db: &DatabaseConnection,
        seconds: i64,
    ) -> Result<u64> {
        let threshold = Utc::now() - chrono::Duration::seconds(seconds);
        let result = Self::delete_many()
            .filter(Column::CreatedAt.lt(threshold))
            .exec(db)
            .await?;
        Ok(result.rows_affected)
    }
}

