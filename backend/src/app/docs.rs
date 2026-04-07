use utoipa::OpenApi;
use crate::domains::skins::handlers as skins_handlers;
use crate::domains::skins::types as skins_types;
use crate::domains::users::handlers;

#[derive(OpenApi)]
#[openapi(
    paths(
        handlers::get_me,
        handlers::update_nickname,
        skins_handlers::upload_my_skin,
        skins_handlers::get_skin,
        // Add other routes here
    ),
    components(
        schemas(
            handlers::UserResponse,
            handlers::UpdateNicknameRequest,
            skins_types::ModelParam,
            skins_types::SkinModel,
            skins_types::UploadSkinResponse,
            // Add other schemas here
        )
    ),
    tags(
        (name = "auth", description = "Authentication API")
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

