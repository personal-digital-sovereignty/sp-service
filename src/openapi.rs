use axum::{
    http::{header},
    response::IntoResponse,
    routing::get,
    Router,
};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Sovereign Pair API",
        description = "REST API for the Sovereign Pair platform — chat, vault, projects, settings, telemetry, and AI training",
        version = "1.4.0"
    ),
    tags(
        (name = "chat", description = "Chat completions and session management"),
        (name = "vault", description = "File system operations and document management"),
        (name = "projects", description = "Project and task CRUD operations"),
        (name = "health", description = "API health and analytics"),
        (name = "settings", description = "Platform configuration and provider settings"),
        (name = "trainer", description = "AI training and deep research controls")
    )
)]
pub struct ApiDoc;

/// Serves the OpenAPI JSON spec at `/api-docs/openapi.json`.
pub async fn openapi_json() -> impl IntoResponse {
    let spec = ApiDoc::openapi();
    let json = serde_json::to_string_pretty(&spec).expect("Failed to serialize OpenAPI spec");
    (
        [(header::CONTENT_TYPE, "application/json")],
        json,
    )
}

/// Serves the Swagger UI HTML at `/swagger-ui`.
pub async fn swagger_ui_html() -> impl IntoResponse {
    let html = r##"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Swagger UI - Sovereign Pair API</title>
    <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/swagger-ui-dist@5/swagger-ui.css">
</head>
<body>
    <div id="swagger-ui"></div>
    <script src="https://cdn.jsdelivr.net/npm/swagger-ui-dist@5/swagger-ui-bundle.js"></script>
    <script>
        SwaggerUIBundle({
            url: "/api-docs/openapi.json",
            dom_id: "#swagger-ui",
            presets: [SwaggerUIBundle.presets.apis],
            layout: "BaseLayout",
        });
    </script>
</body>
</html>"##;
    ([(header::CONTENT_TYPE, "text/html; charset=utf-8")], html)
}

/// Mounts the Swagger UI and OpenAPI JSON routes onto the given router.
pub fn mount_swagger<S>(router: Router<S>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    router
        .route("/api-docs/openapi.json", get(openapi_json))
        .route("/swagger-ui", get(swagger_ui_html))
}
