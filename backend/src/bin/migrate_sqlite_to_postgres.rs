#[path = "../app/mod.rs"]
mod app;
#[path = "../domains/mod.rs"]
mod domains;
#[path = "../entities/mod.rs"]
mod entities;
#[path = "../domains/skins/png_checker/mod.rs"]
mod png_checker;
#[path = "../services/mod.rs"]
mod services;
#[path = "../util/mod.rs"]
mod util;

use anyhow::{Context, Result};
use sea_orm::entity::prelude::*;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ConnectionTrait, Database, DatabaseBackend, DbConn,
    Statement, TransactionTrait,
};

use crate::entities::{auth_ray, user};

#[tokio::main]
async fn main() -> Result<()> {
    if let Err(e) = dotenvy::dotenv() {
        eprintln!("Warning: .env file not loaded: {}", e);
    }

    let source_url = std::env::var("SQLITE_MIGRATION_SOURCE")
        .or_else(|_| std::env::var("SQLITE_DATABASE_URL"))
        .context("SQLITE_MIGRATION_SOURCE not set")?;
    let target_url = std::env::var("DATABASE_URL").context("DATABASE_URL not set")?;

    if !is_sqlite_url(&source_url) {
        anyhow::bail!("SQLITE_MIGRATION_SOURCE must point to a SQLite database");
    }
    if !is_postgres_url(&target_url) {
        anyhow::bail!("DATABASE_URL must point to a PostgreSQL database");
    }

    let source = Database::connect(&source_url)
        .await
        .with_context(|| format!("Failed to connect to source SQLite database: {}", source_url))?;
    let target = Database::connect(&target_url)
        .await
        .with_context(|| format!("Failed to connect to target Postgres database: {}", target_url))?;

    services::db::run_migrations(&target).await?;

    migrate_users(&source, &target).await?;
    migrate_auth_rays(&source, &target).await?;

    println!("SQLite to Postgres migration completed successfully.");
    Ok(())
}

async fn migrate_users(source: &DbConn, target: &DbConn) -> Result<()> {
    let source_users = user::Entity::find()
        .all(source)
        .await
        .context("Failed to read users from SQLite")?;

    let txn = target
        .begin()
        .await
        .context("Failed to start Postgres transaction for users")?;

    user::Entity::delete_many()
        .exec(&txn)
        .await
        .context("Failed to clear target user table")?;

    for row in source_users {
        let active_model = user::ActiveModel {
            id: ActiveValue::Set(row.id),
            discord_id: ActiveValue::Set(row.discord_id),
            username: ActiveValue::Set(row.username),
            avatar_url: ActiveValue::Set(row.avatar_url),
            email: ActiveValue::Set(row.email),
            auth_epoch: ActiveValue::Set(row.auth_epoch),
            is_active: ActiveValue::Set(row.is_active),
            last_login_at: ActiveValue::Set(row.last_login_at),
            created_at: ActiveValue::Set(row.created_at),
        };

        active_model
            .insert(&txn)
            .await
            .context("Failed to insert user into Postgres")?;
    }

    txn.commit()
        .await
        .context("Failed to commit user migration transaction")?;

    println!("Migrated users.");
    Ok(())
}

async fn migrate_auth_rays(source: &DbConn, target: &DbConn) -> Result<()> {
    let source_auth_rays = auth_ray::Entity::find()
        .all(source)
        .await
        .context("Failed to read auth_ray rows from SQLite")?;

    let txn = target
        .begin()
        .await
        .context("Failed to start Postgres transaction for auth_ray")?;

    auth_ray::Entity::delete_many()
        .exec(&txn)
        .await
        .context("Failed to clear target auth_ray table")?;

    for row in source_auth_rays {
        let active_model = auth_ray::ActiveModel {
            id: ActiveValue::Set(row.id),
            pow_prefix: ActiveValue::Set(row.pow_prefix),
            pow_complexity: ActiveValue::Set(row.pow_complexity),
            delivery_method: ActiveValue::Set(row.delivery_method),
            delivery_target: ActiveValue::Set(row.delivery_target),
            created_at: ActiveValue::Set(row.created_at),
        };

        active_model
            .insert(&txn)
            .await
            .context("Failed to insert auth_ray row into Postgres")?;
    }

    reset_auth_ray_sequence(&txn).await?;

    txn.commit()
        .await
        .context("Failed to commit auth_ray migration transaction")?;

    println!("Migrated auth_ray rows.");
    Ok(())
}

async fn reset_auth_ray_sequence(connection: &impl ConnectionTrait) -> Result<()> {
    let sql = r#"
        SELECT setval(
            pg_get_serial_sequence('"auth_ray"', 'id'),
            COALESCE((SELECT MAX(id) FROM "auth_ray"), 1),
            EXISTS(SELECT 1 FROM "auth_ray")
        )
    "#;

    connection
        .execute(Statement::from_string(DatabaseBackend::Postgres, sql.to_string()))
        .await
        .context("Failed to reset auth_ray sequence")?;

    Ok(())
}

fn is_sqlite_url(url: &str) -> bool {
    url.starts_with("sqlite:") || url.starts_with("sqlite://")
}

fn is_postgres_url(url: &str) -> bool {
    url.starts_with("postgres://") || url.starts_with("postgresql://")
}
