use sea_orm::prelude::Expr;
use sea_orm::{DbErr, DeriveIden};
use sea_orm_migration::prelude::*;

pub struct CreateMetricsTables;

impl MigrationName for CreateMetricsTables {
    fn name(&self) -> &str {
        "m20260710_000001_create_metrics_tables"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for CreateMetricsTables {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        create_ingestion(manager).await?;
        create_match(manager).await?;
        create_player(manager).await?;
        create_player_nickname(manager).await?;
        create_match_player(manager).await?;
        create_player_match_stat(manager).await?;
        create_discord_outbox(manager).await?;
        create_indexes(manager).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(MetricDiscordOutbox::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(MetricPlayerMatchStat::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(MetricMatchPlayer::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(MetricPlayerNickname::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(MetricPlayer::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(MetricMatch::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(
                Table::drop()
                    .table(MetricIngestion::Table)
                    .if_exists()
                    .to_owned(),
            )
            .await?;
        Ok(())
    }
}

async fn create_ingestion(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(MetricIngestion::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(MetricIngestion::Id)
                        .uuid()
                        .not_null()
                        .primary_key(),
                )
                .col(
                    ColumnDef::new(MetricIngestion::GameId)
                        .uuid()
                        .not_null()
                        .unique_key(),
                )
                .col(
                    ColumnDef::new(MetricIngestion::ContentSha256)
                        .string_len(64)
                        .not_null(),
                )
                .col(ColumnDef::new(MetricIngestion::Status).string().not_null())
                .col(
                    ColumnDef::new(MetricIngestion::ByteCount)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(MetricIngestion::EventCount)
                        .big_integer()
                        .not_null(),
                )
                .col(ColumnDef::new(MetricIngestion::Warnings).text().not_null())
                .col(timestamp(MetricIngestion::CreatedAt))
                .col(timestamp(MetricIngestion::CompletedAt))
                .to_owned(),
        )
        .await
}

async fn create_match(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(MetricMatch::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(MetricMatch::GameId)
                        .uuid()
                        .not_null()
                        .primary_key(),
                )
                .col(
                    ColumnDef::new(MetricMatch::IngestionId)
                        .uuid()
                        .not_null()
                        .unique_key(),
                )
                .col(ColumnDef::new(MetricMatch::Map).string().not_null())
                .col(timestamp(MetricMatch::StartedAt))
                .col(
                    ColumnDef::new(MetricMatch::EndedAt)
                        .timestamp_with_time_zone()
                        .null(),
                )
                .col(
                    ColumnDef::new(MetricMatch::StartTimestampMs)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(MetricMatch::EndTimestampMs)
                        .big_integer()
                        .null(),
                )
                .col(ColumnDef::new(MetricMatch::DurationMs).big_integer().null())
                .col(ColumnDef::new(MetricMatch::WinningTeam).string().null())
                .col(ColumnDef::new(MetricMatch::Status).string().not_null())
                .col(
                    ColumnDef::new(MetricMatch::EventCount)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(MetricMatch::PlayerCount)
                        .big_integer()
                        .not_null(),
                )
                .col(ColumnDef::new(MetricMatch::Warnings).text().not_null())
                .col(timestamp(MetricMatch::CreatedAt))
                .foreign_key(
                    ForeignKey::create()
                        .name("fk-metric-match-ingestion")
                        .from(MetricMatch::Table, MetricMatch::IngestionId)
                        .to(MetricIngestion::Table, MetricIngestion::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .to_owned(),
        )
        .await
}

async fn create_player(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(MetricPlayer::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(MetricPlayer::PlayerId)
                        .uuid()
                        .not_null()
                        .primary_key(),
                )
                .col(
                    ColumnDef::new(MetricPlayer::LastNickname)
                        .string()
                        .not_null(),
                )
                .col(timestamp(MetricPlayer::FirstSeenAt))
                .col(timestamp(MetricPlayer::LastSeenAt))
                .to_owned(),
        )
        .await
}

async fn create_player_nickname(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(MetricPlayerNickname::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(MetricPlayerNickname::Id)
                        .uuid()
                        .not_null()
                        .primary_key(),
                )
                .col(
                    ColumnDef::new(MetricPlayerNickname::PlayerId)
                        .uuid()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(MetricPlayerNickname::Nickname)
                        .string()
                        .not_null(),
                )
                .col(timestamp(MetricPlayerNickname::FirstSeenAt))
                .col(timestamp(MetricPlayerNickname::LastSeenAt))
                .foreign_key(
                    ForeignKey::create()
                        .name("fk-metric-player-nickname-player")
                        .from(MetricPlayerNickname::Table, MetricPlayerNickname::PlayerId)
                        .to(MetricPlayer::Table, MetricPlayer::PlayerId)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .to_owned(),
        )
        .await
}

async fn create_match_player(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(MetricMatchPlayer::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(MetricMatchPlayer::Id)
                        .uuid()
                        .not_null()
                        .primary_key(),
                )
                .col(ColumnDef::new(MetricMatchPlayer::GameId).uuid().not_null())
                .col(
                    ColumnDef::new(MetricMatchPlayer::PlayerId)
                        .uuid()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(MetricMatchPlayer::Nickname)
                        .string()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(MetricMatchPlayer::InitialTeam)
                        .string()
                        .null(),
                )
                .col(ColumnDef::new(MetricMatchPlayer::FinalTeam).string().null())
                .col(
                    ColumnDef::new(MetricMatchPlayer::ChangedTeam)
                        .boolean()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(MetricMatchPlayer::TeamChanges)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(MetricMatchPlayer::LeftCount)
                        .big_integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(MetricMatchPlayer::TimeInGameMs)
                        .big_integer()
                        .not_null(),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk-metric-match-player-match")
                        .from(MetricMatchPlayer::Table, MetricMatchPlayer::GameId)
                        .to(MetricMatch::Table, MetricMatch::GameId)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk-metric-match-player-player")
                        .from(MetricMatchPlayer::Table, MetricMatchPlayer::PlayerId)
                        .to(MetricPlayer::Table, MetricPlayer::PlayerId)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .to_owned(),
        )
        .await
}

async fn create_player_match_stat(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    let mut table = Table::create();
    table
        .table(MetricPlayerMatchStat::Table)
        .if_not_exists()
        .col(
            ColumnDef::new(MetricPlayerMatchStat::Id)
                .uuid()
                .not_null()
                .primary_key(),
        )
        .col(
            ColumnDef::new(MetricPlayerMatchStat::GameId)
                .uuid()
                .not_null(),
        )
        .col(
            ColumnDef::new(MetricPlayerMatchStat::PlayerId)
                .uuid()
                .not_null(),
        );
    for column in [
        MetricPlayerMatchStat::Kills,
        MetricPlayerMatchStat::Deaths,
        MetricPlayerMatchStat::Assists,
        MetricPlayerMatchStat::Teamkills,
        MetricPlayerMatchStat::Suicides,
        MetricPlayerMatchStat::EnvironmentDeaths,
        MetricPlayerMatchStat::Headshots,
        MetricPlayerMatchStat::VehicleFinalHits,
        MetricPlayerMatchStat::VehicleDestructions,
        MetricPlayerMatchStat::VehicleTeamkills,
        MetricPlayerMatchStat::VehiclesLost,
        MetricPlayerMatchStat::MatchesPlayed,
        MetricPlayerMatchStat::Wins,
        MetricPlayerMatchStat::Losses,
    ] {
        table.col(ColumnDef::new(column).big_integer().not_null().default(0));
    }
    for column in [
        MetricPlayerMatchStat::DamageDealt,
        MetricPlayerMatchStat::DamageTaken,
        MetricPlayerMatchStat::FriendlyDamage,
        MetricPlayerMatchStat::SelfDamage,
        MetricPlayerMatchStat::HeadshotDamage,
        MetricPlayerMatchStat::VehicleDamageDealt,
        MetricPlayerMatchStat::VehicleDamageTaken,
        MetricPlayerMatchStat::FriendlyVehicleDamage,
    ] {
        table.col(ColumnDef::new(column).double().not_null().default(0.0));
    }
    for column in [
        MetricPlayerMatchStat::KillsByWeapon,
        MetricPlayerMatchStat::DeathsBySource,
        MetricPlayerMatchStat::DamageByWeapon,
        MetricPlayerMatchStat::DamageBySource,
        MetricPlayerMatchStat::VehicleDamageByType,
        MetricPlayerMatchStat::VehicleDamageByWeapon,
        MetricPlayerMatchStat::VehicleKillsByType,
        MetricPlayerMatchStat::VehicleKillsByWeapon,
    ] {
        table.col(ColumnDef::new(column).text().not_null());
    }
    table
        .foreign_key(
            ForeignKey::create()
                .name("fk-metric-stat-match")
                .from(MetricPlayerMatchStat::Table, MetricPlayerMatchStat::GameId)
                .to(MetricMatch::Table, MetricMatch::GameId)
                .on_delete(ForeignKeyAction::Cascade),
        )
        .foreign_key(
            ForeignKey::create()
                .name("fk-metric-stat-player")
                .from(
                    MetricPlayerMatchStat::Table,
                    MetricPlayerMatchStat::PlayerId,
                )
                .to(MetricPlayer::Table, MetricPlayer::PlayerId)
                .on_delete(ForeignKeyAction::Cascade),
        );
    manager.create_table(table.to_owned()).await
}

async fn create_discord_outbox(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(MetricDiscordOutbox::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(MetricDiscordOutbox::GameId)
                        .uuid()
                        .not_null()
                        .primary_key(),
                )
                .col(
                    ColumnDef::new(MetricDiscordOutbox::Title)
                        .string()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(MetricDiscordOutbox::Description)
                        .text()
                        .not_null(),
                )
                .col(ColumnDef::new(MetricDiscordOutbox::Url).string().not_null())
                .col(
                    ColumnDef::new(MetricDiscordOutbox::Status)
                        .string()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(MetricDiscordOutbox::AttemptCount)
                        .integer()
                        .not_null()
                        .default(0),
                )
                .col(ColumnDef::new(MetricDiscordOutbox::LastError).text().null())
                .col(timestamp(MetricDiscordOutbox::NextAttemptAt))
                .col(
                    ColumnDef::new(MetricDiscordOutbox::LastAttemptAt)
                        .timestamp_with_time_zone()
                        .null(),
                )
                .col(
                    ColumnDef::new(MetricDiscordOutbox::DeliveredAt)
                        .timestamp_with_time_zone()
                        .null(),
                )
                .col(
                    ColumnDef::new(MetricDiscordOutbox::DiscordChannelId)
                        .string()
                        .null(),
                )
                .col(
                    ColumnDef::new(MetricDiscordOutbox::DiscordMessageId)
                        .string()
                        .null(),
                )
                .col(timestamp(MetricDiscordOutbox::CreatedAt))
                .col(timestamp(MetricDiscordOutbox::UpdatedAt))
                .foreign_key(
                    ForeignKey::create()
                        .name("fk-metric-outbox-match")
                        .from(MetricDiscordOutbox::Table, MetricDiscordOutbox::GameId)
                        .to(MetricMatch::Table, MetricMatch::GameId)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .to_owned(),
        )
        .await
}

async fn create_indexes(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    let indexes = [
        Index::create()
            .name("uq-metric-player-nickname")
            .table(MetricPlayerNickname::Table)
            .col(MetricPlayerNickname::PlayerId)
            .col(MetricPlayerNickname::Nickname)
            .unique()
            .if_not_exists()
            .to_owned(),
        Index::create()
            .name("uq-metric-match-player")
            .table(MetricMatchPlayer::Table)
            .col(MetricMatchPlayer::GameId)
            .col(MetricMatchPlayer::PlayerId)
            .unique()
            .if_not_exists()
            .to_owned(),
        Index::create()
            .name("uq-metric-player-match-stat")
            .table(MetricPlayerMatchStat::Table)
            .col(MetricPlayerMatchStat::GameId)
            .col(MetricPlayerMatchStat::PlayerId)
            .unique()
            .if_not_exists()
            .to_owned(),
        Index::create()
            .name("idx-metric-match-started-status")
            .table(MetricMatch::Table)
            .col(MetricMatch::StartedAt)
            .col(MetricMatch::Status)
            .if_not_exists()
            .to_owned(),
        Index::create()
            .name("idx-metric-stat-player-game")
            .table(MetricPlayerMatchStat::Table)
            .col(MetricPlayerMatchStat::PlayerId)
            .col(MetricPlayerMatchStat::GameId)
            .if_not_exists()
            .to_owned(),
        Index::create()
            .name("idx-metric-outbox-due")
            .table(MetricDiscordOutbox::Table)
            .col(MetricDiscordOutbox::Status)
            .col(MetricDiscordOutbox::NextAttemptAt)
            .if_not_exists()
            .to_owned(),
    ];
    for index in indexes {
        manager.create_index(index).await?;
    }
    Ok(())
}

fn timestamp<T: IntoIden>(column: T) -> ColumnDef {
    let mut definition = ColumnDef::new(column);
    definition
        .timestamp_with_time_zone()
        .not_null()
        .default(Expr::current_timestamp());
    definition
}

#[derive(Copy, Clone, DeriveIden)]
enum MetricIngestion {
    Table,
    Id,
    GameId,
    ContentSha256,
    Status,
    ByteCount,
    EventCount,
    Warnings,
    CreatedAt,
    CompletedAt,
}

#[derive(Copy, Clone, DeriveIden)]
enum MetricMatch {
    Table,
    GameId,
    IngestionId,
    Map,
    StartedAt,
    EndedAt,
    StartTimestampMs,
    EndTimestampMs,
    DurationMs,
    WinningTeam,
    Status,
    EventCount,
    PlayerCount,
    Warnings,
    CreatedAt,
}

#[derive(Copy, Clone, DeriveIden)]
enum MetricPlayer {
    Table,
    PlayerId,
    LastNickname,
    FirstSeenAt,
    LastSeenAt,
}

#[derive(Copy, Clone, DeriveIden)]
enum MetricPlayerNickname {
    Table,
    Id,
    PlayerId,
    Nickname,
    FirstSeenAt,
    LastSeenAt,
}

#[derive(Copy, Clone, DeriveIden)]
enum MetricMatchPlayer {
    Table,
    Id,
    GameId,
    PlayerId,
    Nickname,
    InitialTeam,
    FinalTeam,
    ChangedTeam,
    TeamChanges,
    LeftCount,
    TimeInGameMs,
}

#[derive(Copy, Clone, DeriveIden)]
enum MetricPlayerMatchStat {
    Table,
    Id,
    GameId,
    PlayerId,
    Kills,
    Deaths,
    Assists,
    Teamkills,
    Suicides,
    EnvironmentDeaths,
    DamageDealt,
    DamageTaken,
    FriendlyDamage,
    SelfDamage,
    Headshots,
    HeadshotDamage,
    VehicleDamageDealt,
    VehicleDamageTaken,
    FriendlyVehicleDamage,
    VehicleFinalHits,
    VehicleDestructions,
    VehicleTeamkills,
    VehiclesLost,
    MatchesPlayed,
    Wins,
    Losses,
    KillsByWeapon,
    DeathsBySource,
    DamageByWeapon,
    DamageBySource,
    VehicleDamageByType,
    VehicleDamageByWeapon,
    VehicleKillsByType,
    VehicleKillsByWeapon,
}

#[derive(Copy, Clone, DeriveIden)]
enum MetricDiscordOutbox {
    Table,
    GameId,
    Title,
    Description,
    Url,
    Status,
    AttemptCount,
    LastError,
    NextAttemptAt,
    LastAttemptAt,
    DeliveredAt,
    DiscordChannelId,
    DiscordMessageId,
    CreatedAt,
    UpdatedAt,
}
