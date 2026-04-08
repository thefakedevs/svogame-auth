use axum::Router;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace;
use tower_http::trace::{HttpMakeClassifier, TraceLayer};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::app::docs::ApiDoc;
use crate::app::state::SharedAppState;
use crate::domains;

pub fn build_router(state: SharedAppState) -> Router {
    let router = Router::new()
        .merge(SwaggerUi::new("/api/docs").url("/api/openapi.json", ApiDoc::openapi()))
        .merge(domains::system::router())
        .merge(domains::meta::router())
        .merge(domains::ownership::router())
        .merge(domains::admin::router())
        .merge(domains::auth::router())
        .merge(domains::squads::router())
        .merge(domains::skins::router())
        .merge(domains::users::router())
        .merge(domains::compat::router())
        .layer(build_cors_layer())
        .layer(prepare_tracing());

    #[cfg(debug_assertions)]
    let router = router.merge(domains::test_support::router());

    router.with_state(state)
}

fn build_cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any)
}

fn prepare_tracing() -> TraceLayer<HttpMakeClassifier> {
    let _ = tracing_subscriber::fmt()
        .with_target(false)
        .with_timer(tracing_subscriber::fmt::time::uptime())
        .with_level(true)
        .with_env_filter("info,tower_http=debug")
        .try_init();

    TraceLayer::new_for_http()
        .make_span_with(trace::DefaultMakeSpan::new().level(tracing::Level::INFO))
        .on_response(trace::DefaultOnResponse::new().level(tracing::Level::INFO))
        .on_failure(trace::DefaultOnFailure::new().level(tracing::Level::ERROR))
}
