pub mod handlers;
pub mod service_tokens;
pub mod skins;
pub mod squads;

use axum::Router;
use axum::routing::{get, post};

pub fn router() -> Router<crate::app::state::SharedAppState> {
    Router::new()
        .route("/api/admin/health", get(handlers::health))
        .route("/api/admin/me", get(handlers::me))
        .route("/api/admin/users", get(handlers::list_users))
        .route(
            "/api/admin/users/{user_id}",
            get(handlers::get_user).patch(handlers::patch_user),
        )
        .route(
            "/api/admin/users/{user_id}/deactivate",
            post(handlers::deactivate_user),
        )
        .route(
            "/api/admin/users/{user_id}/activate",
            post(handlers::activate_user),
        )
        .route(
            "/api/admin/users/{user_id}/reset-auth-epoch",
            post(handlers::reset_auth_epoch),
        )
        .route(
            "/api/admin/users/{user_id}/grant-superuser",
            post(handlers::grant_superuser),
        )
        .route(
            "/api/admin/users/{user_id}/revoke-superuser",
            post(handlers::revoke_superuser),
        )
        .route(
            "/api/admin/service-tokens",
            get(service_tokens::list_service_tokens).post(service_tokens::create_service_token),
        )
        .route(
            "/api/admin/service-tokens/{token_id}",
            get(service_tokens::get_service_token),
        )
        .route(
            "/api/admin/service-tokens/{token_id}/audit",
            get(service_tokens::get_service_token_audit),
        )
        .route(
            "/api/admin/service-tokens/{token_id}/rotate",
            post(service_tokens::rotate_service_token),
        )
        .route(
            "/api/admin/service-tokens/{token_id}/revoke",
            post(service_tokens::revoke_service_token),
        )
        .route(
            "/api/admin/users/{user_id}/restrictions",
            get(handlers::get_user_restrictions),
        )
        .route(
            "/api/admin/users/{user_id}/restrictions/{restriction_key}",
            post(handlers::grant_user_restriction).delete(handlers::revoke_user_restriction),
        )
        .route("/api/admin/squads", get(squads::list_squads))
        .route(
            "/api/admin/squads/{squad_id}",
            get(squads::get_squad)
                .patch(squads::patch_squad)
                .delete(squads::delete_squad),
        )
        .route(
            "/api/admin/squads/{squad_id}/restrict",
            post(squads::restrict_squad),
        )
        .route(
            "/api/admin/squads/{squad_id}/unrestrict",
            post(squads::unrestrict_squad),
        )
        .route(
            "/api/admin/squads/{squad_id}/members/{user_id}/kick",
            post(squads::kick_member),
        )
        .route(
            "/api/admin/squads/{squad_id}/image",
            post(squads::upload_squad_image).delete(squads::delete_squad_image),
        )
        .route(
            "/api/admin/skins/default",
            get(skins::get_default_skin).post(skins::upload_default_skin),
        )
}
