//! MCP tool definitions for the reports extension.

use convergio_types::extension::McpToolDef;
use serde_json::json;

pub fn reports_tools() -> Vec<McpToolDef> {
    vec![
        McpToolDef {
            name: "cvg_generate_report".into(),
            description: "Generate a CTT research report on a given topic.".into(),
            method: "POST".into(),
            path: "/api/reports/generate".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "topic": {"type": "string", "description": "Report topic"},
                    "report_type": {"type": "string", "description": "Type of report", "enum": ["leadership-profile", "company-deep-dive", "industry-analysis", "tech-analysis", "market-report", "general", "analysis", "research"]},
                    "format": {"type": "string", "enum": ["markdown", "pdf"]},
                    "depth": {"type": "string", "enum": ["brief", "standard", "full"]}
                },
                "required": ["topic", "report_type"]
            }),
            min_ring: "trusted".into(),
            path_params: vec![],
        },
        McpToolDef {
            name: "cvg_list_reports".into(),
            description: "List CTT research reports with optional filters.".into(),
            method: "GET".into(),
            path: "/api/reports".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "report_type": {"type": "string"},
                    "status": {"type": "string"},
                    "limit": {"type": "integer"}
                }
            }),
            min_ring: "sandboxed".into(),
            path_params: vec![],
        },
    ]
}
