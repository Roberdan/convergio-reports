//! HTTP routes for the CTT report service.
//!
//! - POST /api/reports/generate   — create and start report generation
//! - GET  /api/reports            — list reports (with optional filters)
//! - GET  /api/reports/:id        — get a single report
//! - GET  /api/reports/:id/download — download PDF (placeholder)

use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json};
use axum::routing::{get, post};
use axum::Router;
use convergio_db::pool::ConnPool;
use serde::Deserialize;
use serde_json::{json, Value};
use tracing;
use uuid::Uuid;

use crate::types::{ReportRequest, ReportStatus};

pub struct ReportsState {
    pub pool: ConnPool,
}

pub fn reports_routes(state: Arc<ReportsState>) -> Router {
    Router::new()
        .route("/api/reports/generate", post(handle_generate))
        .route("/api/reports", get(handle_list))
        .route("/api/reports/:id", get(handle_get))
        .route("/api/reports/:id/download", get(handle_download))
        .with_state(state)
}

async fn handle_generate(
    State(s): State<Arc<ReportsState>>,
    Json(req): Json<ReportRequest>,
) -> impl IntoResponse {
    let report_id = format!("rpt-{}", Uuid::new_v4());

    let type_str = serde_json::to_value(req.report_type)
        .ok()
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_else(|| "general".to_string());
    let format_str = serde_json::to_value(req.format)
        .ok()
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_else(|| "markdown".to_string());
    let depth_str = serde_json::to_value(req.depth)
        .ok()
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_else(|| "standard".to_string());

    let conn = match s.pool.get() {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
        }
    };

    let insert_result = conn.execute(
        "INSERT INTO reports (id, org_id, topic, report_type, format, status, \
         audience, depth, extra_context, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, datetime('now'))",
        rusqlite::params![
            report_id,
            req.org_id,
            req.topic,
            type_str,
            format_str,
            ReportStatus::Pending.to_string(),
            req.audience,
            depth_str,
            req.extra_context,
        ],
    );

    if let Err(e) = insert_result {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        );
    }

    // Spawn async generation
    let pool = s.pool.clone();
    let rid = report_id.clone();
    tokio::spawn(async move {
        crate::engine::generate(&pool, &rid).await;
    });

    (
        StatusCode::OK,
        Json(json!({
            "report_id": report_id,
            "status": "pending",
        })),
    )
}

#[derive(Deserialize, Default)]
struct ListQuery {
    org_id: Option<String>,
    report_type: Option<String>,
    status: Option<String>,
    limit: Option<u32>,
    offset: Option<u32>,
}

async fn handle_list(
    State(s): State<Arc<ReportsState>>,
    Query(q): Query<ListQuery>,
) -> Json<Value> {
    let conn = match s.pool.get() {
        Ok(c) => c,
        Err(e) => return Json(json!({"error": e.to_string()})),
    };

    let limit = q.limit.unwrap_or(20).min(100);
    let offset = q.offset.unwrap_or(0);

    let mut conditions = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(ref org) = q.org_id {
        conditions.push(format!("org_id = ?{}", params.len() + 1));
        params.push(Box::new(org.clone()));
    }
    if let Some(ref rt) = q.report_type {
        conditions.push(format!("report_type = ?{}", params.len() + 1));
        params.push(Box::new(rt.clone()));
    }
    if let Some(ref st) = q.status {
        conditions.push(format!("status = ?{}", params.len() + 1));
        params.push(Box::new(st.clone()));
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let sql = format!(
        "SELECT id, org_id, topic, report_type, format, status, \
         created_at, completed_at \
         FROM reports {where_clause} \
         ORDER BY created_at DESC \
         LIMIT {limit} OFFSET {offset}"
    );

    let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let mut stmt = match conn.prepare(&sql) {
        Ok(s) => s,
        Err(e) => return Json(json!({"error": e.to_string()})),
    };

    let rows: Vec<Value> = stmt
        .query_map(param_refs.as_slice(), |r| {
            Ok(json!({
                "id": r.get::<_, String>(0)?,
                "org_id": r.get::<_, String>(1)?,
                "topic": r.get::<_, String>(2)?,
                "report_type": r.get::<_, String>(3)?,
                "format": r.get::<_, String>(4)?,
                "status": r.get::<_, String>(5)?,
                "created_at": r.get::<_, String>(6)?,
                "completed_at": r.get::<_, Option<String>>(7)?,
            }))
        })
        .map(|rows| rows.filter_map(|r| r.ok()).collect())
        .unwrap_or_default();

    Json(json!({"reports": rows}))
}

async fn handle_get(
    State(s): State<Arc<ReportsState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let conn = match s.pool.get() {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": e.to_string()})),
            )
        }
    };

    let result = conn.query_row(
        "SELECT id, org_id, topic, report_type, format, status, audience, depth, \
         content_md, pdf_path, sources_json, metadata_json, error, \
         created_at, completed_at \
         FROM reports WHERE id = ?1",
        rusqlite::params![id],
        |r| {
            Ok(json!({
                "id": r.get::<_, String>(0)?,
                "org_id": r.get::<_, String>(1)?,
                "topic": r.get::<_, String>(2)?,
                "report_type": r.get::<_, String>(3)?,
                "format": r.get::<_, String>(4)?,
                "status": r.get::<_, String>(5)?,
                "audience": r.get::<_, Option<String>>(6)?,
                "depth": r.get::<_, String>(7)?,
                "content_md": r.get::<_, Option<String>>(8)?,
                "pdf_path": r.get::<_, Option<String>>(9)?,
                "sources_json": r.get::<_, Option<String>>(10)?,
                "metadata_json": r.get::<_, Option<String>>(11)?,
                "error": r.get::<_, Option<String>>(12)?,
                "created_at": r.get::<_, String>(13)?,
                "completed_at": r.get::<_, Option<String>>(14)?,
            }))
        },
    );

    match result {
        Ok(report) => (StatusCode::OK, Json(report)),
        Err(_) => (StatusCode::NOT_FOUND, Json(json!({"error": "not found"}))),
    }
}

async fn handle_download(
    State(s): State<Arc<ReportsState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let conn = match s.pool.get() {
        Ok(c) => c,
        Err(e) => return err_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    };

    let row = conn.query_row(
        "SELECT format, pdf_path, status FROM reports WHERE id = ?1",
        rusqlite::params![id],
        |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, String>(2)?,
            ))
        },
    );

    let (format, pdf_path, status) = match row {
        Ok(r) => r,
        Err(_) => return err_response(StatusCode::NOT_FOUND, "report not found"),
    };

    if format != "pdf" {
        return err_response(StatusCode::BAD_REQUEST, "report is not PDF format");
    }
    if status != "completed" {
        return err_response(StatusCode::CONFLICT, &format!("status is '{status}'"));
    }

    let path = match pdf_path {
        Some(p) if std::path::Path::new(&p).exists() => {
            // Prevent path traversal — only allow files under the expected report dir
            let canonical = match std::fs::canonicalize(&p) {
                Ok(c) => c,
                Err(_) => return err_response(StatusCode::NOT_FOUND, "PDF file not available"),
            };
            if !canonical.starts_with("/tmp/ctt-reports") {
                tracing::warn!(path = %p, "blocked PDF download outside allowed directory");
                return err_response(StatusCode::FORBIDDEN, "invalid PDF path");
            }
            p
        }
        _ => return err_response(StatusCode::NOT_FOUND, "PDF file not available"),
    };

    match std::fs::read(&path) {
        Ok(bytes) => {
            let headers = [
                ("content-type", "application/pdf".to_string()),
                (
                    "content-disposition",
                    "attachment; filename=\"report.pdf\"".to_string(),
                ),
            ];
            (StatusCode::OK, headers, bytes).into_response()
        }
        Err(e) => err_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("read error: {e}"),
        ),
    }
}

fn err_response(status: StatusCode, msg: &str) -> axum::response::Response {
    (status, Json(json!({"error": msg}))).into_response()
}
