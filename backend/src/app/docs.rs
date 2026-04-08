use utoipa::OpenApi;
use crate::domains::admin::handlers as admin_handlers;
use crate::domains::admin::service_tokens as admin_service_tokens;
use crate::domains::admin::squads as admin_squads;
use crate::domains::auth::handlers as auth_handlers;
use crate::domains::auth::polling as auth_polling;
use crate::domains::auth::verification as auth_verification;
use crate::domains::compat::gamervii as compat_gamervii;
use crate::domains::meta::handlers as meta_handlers;
use crate::domains::ownership::handlers as ownership_handlers;
use crate::domains::squads::handlers as squads_handlers;
use crate::domains::skins::handlers as skins_handlers;
use crate::domains::skins::types as skins_types;
use crate::domains::system::health as system_health;
use crate::domains::users::handlers;
use crate::services::ownership::catalog as ownership_catalog;
use crate::services::ownership::types as ownership_types;

#[derive(OpenApi)]
#[openapi(
    paths(
        admin_handlers::health,
        admin_handlers::me,
        admin_handlers::list_users,
        admin_handlers::get_user,
        admin_handlers::patch_user,
        admin_handlers::deactivate_user,
        admin_handlers::activate_user,
        admin_handlers::reset_auth_epoch,
        admin_handlers::grant_superuser,
        admin_handlers::revoke_superuser,
        admin_handlers::get_user_restrictions,
        admin_handlers::grant_user_restriction,
        admin_handlers::revoke_user_restriction,
        admin_service_tokens::list_service_tokens,
        admin_service_tokens::create_service_token,
        admin_service_tokens::get_service_token,
        admin_service_tokens::get_service_token_audit,
        admin_service_tokens::rotate_service_token,
        admin_service_tokens::revoke_service_token,
        admin_squads::list_squads,
        admin_squads::get_squad,
        admin_squads::patch_squad,
        admin_squads::restrict_squad,
        admin_squads::unrestrict_squad,
        admin_squads::delete_squad,
        admin_squads::kick_member,
        admin_squads::upload_squad_image,
        admin_squads::delete_squad_image,
        auth_handlers::prepare_auth,
        auth_handlers::authorize,
        auth_polling::poll_auth_status,
        auth_verification::verify,
        handlers::get_me,
        handlers::search_users,
        handlers::update_nickname,
        handlers::get_my_restrictions,
        compat_gamervii::gamervii_auth,
        meta_handlers::get_restrictions_meta,
        meta_handlers::get_squads_config,
        ownership_handlers::list_public_assets,
        ownership_handlers::get_public_asset,
        ownership_handlers::list_admin_assets,
        ownership_handlers::get_admin_asset,
        ownership_handlers::create_asset,
        ownership_handlers::patch_asset,
        ownership_handlers::get_my_inventory,
        ownership_handlers::check_my_inventory_presence,
        ownership_handlers::get_my_stackables,
        ownership_handlers::get_my_entitlements,
        ownership_handlers::get_my_active_expirables,
        ownership_handlers::get_my_wallet,
        ownership_handlers::get_my_wallet_balance,
        ownership_handlers::get_my_wallet_transactions,
        ownership_handlers::get_user_inventory,
        ownership_handlers::get_user_inventory_history,
        ownership_handlers::grant_entitlement,
        ownership_handlers::revoke_entitlement,
        ownership_handlers::add_stackable,
        ownership_handlers::remove_stackable,
        ownership_handlers::set_stackable,
        ownership_handlers::prolong_expirable,
        ownership_handlers::set_expiration,
        ownership_handlers::revoke_expirable,
        ownership_handlers::get_user_wallet_transactions,
        ownership_handlers::credit_wallet,
        ownership_handlers::debit_wallet,
        ownership_handlers::adjust_wallet_balance,
        squads_handlers::create_squad,
        squads_handlers::get_squad,
        squads_handlers::get_squad_members,
        squads_handlers::patch_squad,
        squads_handlers::upload_squad_image,
        squads_handlers::delete_squad_image,
        squads_handlers::delete_squad,
        squads_handlers::leave_squad,
        squads_handlers::create_invite,
        squads_handlers::list_my_invites,
        squads_handlers::accept_invite,
        squads_handlers::decline_invite,
        squads_handlers::revoke_invite,
        squads_handlers::kick_member,
        squads_handlers::get_my_squad,
        skins_handlers::upload_my_skin,
        skins_handlers::get_default_skin,
        skins_handlers::get_skin,
        skins_handlers::get_default_skin_admin,
        skins_handlers::upload_default_skin_admin,
        system_health::health
    ),
    components(
        schemas(
            admin_handlers::AdminHealthResponse,
            admin_handlers::AdminMeResponse,
            admin_handlers::AdminUserResponse,
            admin_handlers::AdminUsersListResponse,
            admin_handlers::PatchAdminUserRequest,
            admin_handlers::DeactivateAdminUserRequest,
            admin_handlers::RestrictionReasonRequest,
            admin_handlers::AdminUserRestrictionResponse,
            admin_service_tokens::CreateServiceTokenRequest,
            admin_service_tokens::RotateServiceTokenRequest,
            admin_service_tokens::RevokeServiceTokenRequest,
            admin_service_tokens::ServiceTokenResponse,
            admin_service_tokens::ServiceTokenAuditResponse,
            admin_squads::ListSquadsQuery,
            admin_squads::PatchAdminSquadRequest,
            admin_squads::ReasonRequest,
            admin_squads::AdminSquadResponse,
            admin_squads::AdminSquadListResponse,
            admin_squads::ActionResponse,
            auth_handlers::PrepareAuthResponse,
            auth_handlers::PrepareAuthRequest,
            auth_handlers::AuthorizeResponse,
            auth_handlers::AuthorizeRequest,
            auth_polling::PollResponse,
            auth_verification::VerifyQuery,
            auth_verification::VerifyResponse,
            handlers::UserResponse,
            handlers::UserSearchItemResponse,
            handlers::UserRestrictionResponse,
            handlers::UpdateNicknameRequest,
            compat_gamervii::GamerViiAuthRequest,
            compat_gamervii::GamerViiAuthResponse,
            meta_handlers::RestrictionMetaResponse,
            meta_handlers::RestrictionLocale,
            meta_handlers::RestrictionLocaleEntry,
            meta_handlers::SquadConfigResponse,
            ownership_handlers::AssetListQuery,
            ownership_handlers::MutationBody,
            ownership_handlers::AssetResponse,
            ownership_handlers::AssetListResponse,
            ownership_handlers::StackableResponse,
            ownership_handlers::EntitlementResponse,
            ownership_handlers::ExpirableResponse,
            ownership_handlers::InventoryResponse,
            ownership_handlers::InventoryPresenceResponse,
            ownership_handlers::InventoryOperationResponse,
            ownership_handlers::WalletBalanceResponse,
            ownership_handlers::WalletTransactionResponse,
            ownership_handlers::OkResponse,
            ownership_catalog::CreateAssetDefinitionInput,
            ownership_catalog::UpdateAssetDefinitionInput,
            ownership_types::AssetKind,
            ownership_types::OwnershipModel,
            squads_handlers::CreateSquadRequest,
            squads_handlers::PatchSquadRequest,
            squads_handlers::CreateInviteRequest,
            squads_handlers::SquadMemberResponse,
            squads_handlers::SquadResponse,
            squads_handlers::SquadInviteResponse,
            squads_handlers::SquadActionResponse,
            skins_handlers::DefaultSkinResponse,
            skins_types::ModelParam,
            skins_types::SkinModel,
            skins_types::UploadSkinResponse,
            system_health::HealthResponse
        )
    ),
    tags(
        (name = "admin", description = "Administrative API"),
        (name = "auth", description = "Authentication API"),
        (name = "compat", description = "Compatibility endpoints for legacy or external clients"),
        (name = "debug", description = "Debug and test-only endpoints"),
        (name = "ownership", description = "Public and self-service ownership catalog and inventory API"),
        (name = "ownership-admin", description = "Administrative inventory and asset catalog API"),
        (name = "meta", description = "Public metadata used by clients to drive UI and validation"),
        (name = "skins", description = "Skin upload and retrieval API"),
        (name = "squads", description = "Squad creation, membership, invite, and moderation API"),
        (name = "system", description = "Operational health endpoints"),
        (name = "users", description = "Authenticated user profile and self-service account endpoints"),
        (name = "wallet", description = "Self-service wallet API for currency balances and transactions"),
        (name = "wallet-admin", description = "Administrative wallet mutation and investigation API")
    ),
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;

use utoipa::Modify;
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.as_mut().unwrap(); // we can unwrap safely since we defined components in macro
        components.add_security_scheme(
            "bearer_auth",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("JWT")
                    .build(),
            ),
        );
    }
}

