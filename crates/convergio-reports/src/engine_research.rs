//! Research and LLM synthesis — the intelligence behind CTT reports.
//!
//! Handles: building search queries, calling inference API,
//! assembling the CTT prompt template, and parsing LLM output.

use serde_json::json;

use crate::types::ReportType;

const DAEMON_URL: &str = "http://localhost:8420";

/// Build web-search queries for the research phase based on topic and type.
pub fn build_search_queries(topic: &str, report_type: ReportType) -> Vec<String> {
    let base = vec![
        format!("{topic} overview latest news"),
        format!("{topic} key facts statistics"),
    ];
    let specific = match report_type {
        ReportType::LeadershipProfile => vec![
            format!("{topic} career background biography"),
            format!("{topic} achievements awards recognition"),
            format!("{topic} leadership impact contributions"),
        ],
        ReportType::CompanyDeepDive => vec![
            format!("{topic} company revenue financials"),
            format!("{topic} strategy leadership CEO"),
            format!("{topic} competitive landscape market position"),
        ],
        ReportType::IndustryAnalysis => vec![
            format!("{topic} industry market size trends"),
            format!("{topic} key players market share"),
            format!("{topic} regulatory landscape challenges"),
        ],
        ReportType::TechAnalysis => vec![
            format!("{topic} technology capabilities features"),
            format!("{topic} adoption market competitors"),
            format!("{topic} roadmap future developments"),
        ],
        ReportType::MarketReport => vec![
            format!("{topic} market trends economic factors"),
            format!("{topic} growth forecast projections"),
            format!("{topic} risks opportunities investors"),
        ],
        ReportType::General => vec![
            format!("{topic} analysis research findings"),
            format!("{topic} trends developments recent"),
        ],
    };
    [base, specific].concat()
}

/// Call the inference API to perform research for a single query.
pub async fn call_inference(prompt: &str, agent_id: &str) -> Result<String, String> {
    let client = reqwest::Client::new();
    let body = json!({
        "prompt": prompt,
        "max_tokens": 2048,
        "tier_hint": "t1",
        "agent_id": agent_id,
        "org_id": "convergio.io",
        "constraints": {}
    });

    let resp = client
        .post(format!("{DAEMON_URL}/api/inference/complete"))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("inference request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("inference returned {}", resp.status()));
    }

    let val: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    // Response shape: { "response": { "content": "..." }, "decision": {...} }
    let content = val
        .pointer("/response/content")
        .or_else(|| val.get("content"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    Ok(content)
}

/// Build the full CTT generation prompt from research data.
pub fn build_generation_prompt(
    topic: &str,
    report_type: ReportType,
    depth: &str,
    audience: Option<&str>,
    extra_context: Option<&str>,
    research_data: &str,
) -> String {
    let depth_instruction = match depth {
        "brief" => "Keep it concise: 3-5 pages equivalent, executive summary focus.",
        "standard" => "Standard depth: 8-12 pages equivalent, balanced analysis.",
        "full" => "Full depth: 15-25 pages equivalent, comprehensive deep-dive.",
        _ => "Standard depth.",
    };

    let audience_line = audience
        .map(|a| format!("Target audience: {a}."))
        .unwrap_or_default();
    let context_line = extra_context
        .map(|c| format!("Additional context: {c}."))
        .unwrap_or_default();

    format!(
        "You are the Convergio Think Tank (CTT) report generator.\n\
         Generate a professional {type_label} on: {topic}\n\n\
         {audience_line}\n{context_line}\n{depth_instruction}\n\n\
         ## Research Data\n{research_data}\n\n\
         ## Output Format (Morgan Stanley methodology)\n\
         Write the report in Markdown with these sections:\n\
         1. Executive Summary (2-3 dense paragraphs)\n\
         2. Key Takeaways (5-7 bullet points)\n\
         3. Deep Analysis (2-4 thematic sections with data)\n\
         4. KPI Dashboard (table with key metrics)\n\
         5. What Worked / Areas to Monitor (structured pro/contra)\n\
         6. Sources & Methodology\n\n\
         ## Rules\n\
         - Every claim must reference the research data provided.\n\
         - Use confidence indicators: [Verified], [Reported], [Uncertain].\n\
         - If data is unavailable, say 'Data not available' — never fabricate.\n\
         - Include a Data Cutoff Date section.\n\
         - Be professional, precise, and data-driven.\n",
        type_label = report_type.label(),
    )
}
