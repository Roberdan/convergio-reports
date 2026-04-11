//! Extension trait implementation for convergio-reports.

use std::sync::Arc;

use convergio_db::pool::ConnPool;
use convergio_types::extension::{AppContext, Extension, Health, McpToolDef, Metric, Migration};
use convergio_types::manifest::{Capability, Manifest, ModuleKind};

use crate::routes::{reports_routes, ReportsState};

pub struct ReportsExtension {
    pool: ConnPool,
}

impl ReportsExtension {
    pub fn new(pool: ConnPool) -> Self {
        Self { pool }
    }

    fn state(&self) -> Arc<ReportsState> {
        Arc::new(ReportsState {
            pool: self.pool.clone(),
        })
    }
}

impl Extension for ReportsExtension {
    fn manifest(&self) -> Manifest {
        Manifest {
            id: "convergio-reports".to_string(),
            description: "Convergio Think Tank — professional report generation service"
                .to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            kind: ModuleKind::Extension,
            provides: vec![
                Capability {
                    name: "reports".to_string(),
                    version: "1.0.0".to_string(),
                    description: "CTT research report generation".to_string(),
                },
                Capability {
                    name: "reports-api".to_string(),
                    version: "1.0.0".to_string(),
                    description: "REST API for report CRUD and generation".to_string(),
                },
            ],
            requires: vec![],
            agent_tools: vec![],
            required_roles: vec!["orchestrator".into(), "all".into()],
        }
    }

    fn routes(&self, _ctx: &AppContext) -> Option<axum::Router> {
        Some(reports_routes(self.state()))
    }

    fn migrations(&self) -> Vec<Migration> {
        vec![Migration {
            version: 1,
            description: "reports table",
            up: "CREATE TABLE IF NOT EXISTS reports (\
                    id TEXT PRIMARY KEY,\
                    org_id TEXT NOT NULL DEFAULT 'convergio.io',\
                    topic TEXT NOT NULL,\
                    report_type TEXT NOT NULL,\
                    format TEXT NOT NULL DEFAULT 'markdown',\
                    status TEXT NOT NULL DEFAULT 'pending',\
                    audience TEXT,\
                    depth TEXT NOT NULL DEFAULT 'standard',\
                    content_md TEXT,\
                    pdf_path TEXT,\
                    sources_json TEXT,\
                    metadata_json TEXT,\
                    extra_context TEXT,\
                    error TEXT,\
                    created_at TEXT NOT NULL DEFAULT (datetime('now')),\
                    completed_at TEXT\
                );\
                CREATE INDEX IF NOT EXISTS idx_reports_org \
                    ON reports(org_id);\
                CREATE INDEX IF NOT EXISTS idx_reports_status \
                    ON reports(status);\
                CREATE INDEX IF NOT EXISTS idx_reports_type \
                    ON reports(report_type);",
        }]
    }

    fn health(&self) -> Health {
        match self.pool.get() {
            Ok(_) => Health::Ok,
            Err(e) => Health::Degraded {
                reason: format!("db: {e}"),
            },
        }
    }

    fn metrics(&self) -> Vec<Metric> {
        let count: f64 = self
            .pool
            .get()
            .ok()
            .and_then(|c| {
                c.query_row("SELECT COUNT(*) FROM reports", [], |r| r.get::<_, i64>(0))
                    .ok()
            })
            .unwrap_or(0) as f64;
        vec![Metric {
            name: "reports_total".to_string(),
            value: count,
            labels: vec![],
        }]
    }

    fn mcp_tools(&self) -> Vec<McpToolDef> {
        crate::mcp_defs::reports_tools()
    }
}
