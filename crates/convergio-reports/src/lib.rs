//! convergio-reports — Convergio Think Tank (CTT) report generation service.
//!
//! Professional research reports: leadership profiles, company deep-dives,
//! industry analysis, technology assessments, and market reports.
//! Outputs Markdown or PDF (via LaTeX). Branded CTT with AI disclaimer.

pub mod engine;
pub mod engine_research;
pub mod ext;
pub mod latex;
pub mod mcp_defs;
pub mod pdf_compiler;
pub mod routes;
pub mod template;
pub mod types;

pub use ext::ReportsExtension;
pub use types::{Report, ReportFormat, ReportRequest, ReportStatus, ReportType};
