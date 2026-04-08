pub mod handlers;

use axum::routing::{delete, get, post};
use axum::Router;

pub fn router() -> Router<crate::app::state::SharedAppState> {
    Router::new()
        .route("/api/squads", post(handlers::create_squad))
        .route(
            "/api/squads/{squad_id}",
            get(handlers::get_squad)
                .patch(handlers::patch_squad)
                .delete(handlers::delete_squad),
        )
        .route("/api/squads/{squad_id}/members", get(handlers::get_squad_members))
        .route("/api/squads/{squad_id}/members/{user_id}/kick", post(handlers::kick_member))
        .route("/api/squads/{squad_id}/leave", post(handlers::leave_squad))
        .route("/api/squads/{squad_id}/invites", post(handlers::create_invite))
        .route(
            "/api/squads/{squad_id}/image",
            post(handlers::upload_squad_image).delete(handlers::delete_squad_image),
        )
        .route("/api/squad-invites/{invite_id}/accept", post(handlers::accept_invite))
        .route("/api/squad-invites/{invite_id}/decline", post(handlers::decline_invite))
        .route("/api/squad-invites/{invite_id}", delete(handlers::revoke_invite))
        .route("/api/user/me/squad", get(handlers::get_my_squad))
        .route("/api/user/me/squad-invites", get(handlers::list_my_invites))
}
