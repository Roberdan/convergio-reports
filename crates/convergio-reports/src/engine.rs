//! Report generation engine — orchestrates research, LLM, and formatting.
//!
//! Pipeline: load → research → synthesize → generate → format → store.
//! Research and LLM calls are in engine_research.rs.

use convergio_db::pool::ConnPool;
use serde_json::json;

use crate::engine_research::{build_generation_prompt, build_search_queries, call_inference};
use crate::template;
use crate::types::{ReportStatus, ReportType};

const AGENT_ID: &str = "ctt-report-engine";

/// Run the full report generation pipeline for a given report ID.
pub async fn generate(pool: &ConnPool, report_id: &str) {
    tracing::info!(report_id, "CTT report generation starting");

    let row = match load_report(pool, report_id) {
        Some(r) => r,
        None => {
            tracing::error!(report_id, "report not found");
            return;
        }
    };

    let report_type: ReportType =
        serde_json::from_value(json!(row.report_type_str)).unwrap_or(ReportType::General);
    let date = template::report_date();

    // Phase 1: Research
    update_status(pool, report_id, ReportStatus::Researching);
    let research = run_research(pool, report_id, &row.topic, report_type).await;

    // Phase 2: Generate via LLM
    update_status(pool, report_id, ReportStatus::Generating);
    let content = run_generation(
        &row.topic,
        report_type,
        &date,
        &row.depth,
        row.audience.as_deref(),
        row.extra_context.as_deref(),
        &research,
    )
    .await;

    // Phase 3: PDF compilation (if requested)
    let pdf_path = if row.format_str == "pdf" {
        update_status(pool, report_id, ReportStatus::Compiling);
        let report_type: ReportType =
            serde_json::from_value(json!(row.report_type_str)).unwrap_or(ReportType::General);
        let latex_content =
            crate::latex::markdown_to_latex(&content, &row.topic, report_type, &date);
        let slug = crate::latex::topic_slug(&row.topic);
        let filename = format!("ctt-{slug}-{}", chrono::Utc::now().format("%Y%m%d"));
        let output_dir = std::path::PathBuf::from("/tmp/ctt-reports");
        match crate::pdf_compiler::compile_pdf(&latex_content, &output_dir, &filename) {
            Ok(path) => {
                tracing::info!(report_id, ?path, "PDF compiled");
                Some(path.to_string_lossy().to_string())
            }
            Err(e) => {
                tracing::warn!(report_id, error = %e, "PDF compilation failed, markdown still saved");
                None
            }
        }
    } else {
        None
    };

    // Phase 4: Store result
    let word_count = content.split_whitespace().count();
    let section_count = content.matches("\n## ").count();
    let metadata = json!({
        "word_count": word_count,
        "section_count": section_count,
        "source_count": 0,
        "format": row.format_str,
        "depth": row.depth,
        "pdf_compiled": pdf_path.is_some(),
    });

    save_content(pool, report_id, &content, &metadata.to_string());
    if let Some(ref path) = pdf_path {
        save_pdf_path(pool, report_id, path);
    }
    update_status(pool, report_id, ReportStatus::Completed);
    tracing::info!(report_id, word_count, "CTT report generation complete");
}

async fn run_research(
    _pool: &ConnPool,
    report_id: &str,
    topic: &str,
    report_type: ReportType,
) -> String {
    let queries = build_search_queries(topic, report_type);
    let mut research_parts = Vec::new();

    for query in &queries {
        let prompt = format!(
            "Search the web for: {query}\n\
             Return factual, well-sourced information. Include URLs where available."
        );
        match call_inference(&prompt, AGENT_ID).await {
            Ok(result) if !result.is_empty() => {
                research_parts.push(result);
            }
            Ok(_) => {
                tracing::debug!(query, "empty research result");
            }
            Err(e) => {
                tracing::warn!(query, error = %e, "research query failed");
            }
        }
    }

    if research_parts.is_empty() {
        tracing::warn!(report_id, "no research data gathered — using topic only");
        format!("Topic: {topic}. No additional research data available.")
    } else {
        research_parts.join("\n\n---\n\n")
    }
}

async fn run_generation(
    topic: &str,
    report_type: ReportType,
    date: &str,
    depth: &str,
    audience: Option<&str>,
    extra_context: Option<&str>,
    research: &str,
) -> String {
    let prompt =
        build_generation_prompt(topic, report_type, depth, audience, extra_context, research);

    let llm_content = match call_inference(&prompt, AGENT_ID).await {
        Ok(content) if !content.is_empty() => content,
        Ok(_) | Err(_) => {
            tracing::warn!("LLM generation failed, using fallback template");
            fallback_content(topic, report_type, date)
        }
    };

    // Wrap with CTT branding
    let mut md = template::format_header(topic, report_type, date);
    md.push_str(&llm_content);
    md.push_str(&template::format_sources(&[]));
    md.push_str(&template::format_disclaimer());
    md.push_str(&template::format_footer(topic, date));
    md
}

fn fallback_content(topic: &str, report_type: ReportType, _date: &str) -> String {
    format!(
        "## Executive Summary\n\n\
         This {label} report on **{topic}** was generated by {brand}. \
         The inference service was unavailable during generation; \
         this is a structural template that will be populated when \
         the service is restored.\n\n\
         ## Key Takeaways\n\n\
         1. Report infrastructure is operational.\n\
         2. CTT branding and disclaimer applied.\n\
         3. Full research-backed content pending inference availability.\n\n",
        label = report_type.label(),
        brand = template::CTT_BRAND,
    )
}

// --- DB helpers ---

struct ReportRow {
    topic: String,
    report_type_str: String,
    format_str: String,
    depth: String,
    audience: Option<String>,
    extra_context: Option<String>,
}

fn load_report(pool: &ConnPool, report_id: &str) -> Option<ReportRow> {
    let conn = pool.get().ok()?;
    conn.query_row(
        "SELECT topic, report_type, format, depth, audience, extra_context \
         FROM reports WHERE id = ?1",
        rusqlite::params![report_id],
        |r| {
            Ok(ReportRow {
                topic: r.get(0)?,
                report_type_str: r.get(1)?,
                format_str: r.get(2)?,
                depth: r.get(3)?,
                audience: r.get(4)?,
                extra_context: r.get(5)?,
            })
        },
    )
    .ok()
}

fn update_status(pool: &ConnPool, report_id: &str, status: ReportStatus) {
    if let Ok(conn) = pool.get() {
        let status_str = status.to_string();
        let _ = conn.execute(
            "UPDATE reports SET status = ?1 WHERE id = ?2",
            rusqlite::params![status_str, report_id],
        );
    }
}

fn save_content(pool: &ConnPool, report_id: &str, content: &str, metadata: &str) {
    if let Ok(conn) = pool.get() {
        let _ = conn.execute(
            "UPDATE reports SET content_md = ?1, metadata_json = ?2, \
             completed_at = datetime('now') WHERE id = ?3",
            rusqlite::params![content, metadata, report_id],
        );
    }
}

fn save_pdf_path(pool: &ConnPool, report_id: &str, pdf_path: &str) {
    if let Ok(conn) = pool.get() {
        let _ = conn.execute(
            "UPDATE reports SET pdf_path = ?1 WHERE id = ?2",
            rusqlite::params![pdf_path, report_id],
        );
    }
}
