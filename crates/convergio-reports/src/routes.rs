//! HTTP API routes for convergio-reports.

use axum::Router;

/// Returns the router for this crate's API endpoints.
pub fn routes() -> Router {
    Router::new()
    // .route("/api/reports/health", get(health))
}
