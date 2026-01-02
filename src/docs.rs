use utoipa::OpenApi;
use crate::routes;

#[derive(OpenApi)]
#[openapi(
    paths(
        routes::get_me,
        routes::update_nickname,
        // Add other routes here
    ),
    components(
        schemas(
            routes::UserResponse,
            routes::UpdateNicknameRequest,
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

