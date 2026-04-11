//! Types for the CTT report service.

use serde::{Deserialize, Serialize};

/// Request to generate a new report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportRequest {
    pub topic: String,
    pub report_type: ReportType,
    #[serde(default = "default_format")]
    pub format: ReportFormat,
    pub audience: Option<String>,
    #[serde(default = "default_depth")]
    pub depth: ReportDepth,
    #[serde(default = "default_org_id")]
    pub org_id: String,
    pub extra_context: Option<String>,
}

fn default_format() -> ReportFormat {
    ReportFormat::Markdown
}
fn default_depth() -> ReportDepth {
    ReportDepth::Standard
}
fn default_org_id() -> String {
    "convergio.io".to_string()
}

/// Supported report types following Morgan Stanley methodology.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReportType {
    LeadershipProfile,
    CompanyDeepDive,
    IndustryAnalysis,
    TechAnalysis,
    MarketReport,
    #[serde(alias = "analysis", alias = "research", alias = "report")]
    General,
}

impl ReportType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::LeadershipProfile => "Leadership & Social Impact Research",
            Self::CompanyDeepDive => "Company Deep-Dive Report",
            Self::IndustryAnalysis => "Industry Analysis",
            Self::TechAnalysis => "Technology Analysis",
            Self::MarketReport => "Market Report",
            Self::General => "General Research",
        }
    }
}

impl std::fmt::Display for ReportType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// Output format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReportFormat {
    Markdown,
    Pdf,
}

/// Research depth level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReportDepth {
    Brief,
    Standard,
    Full,
}

/// Report generation status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReportStatus {
    Pending,
    Researching,
    Generating,
    Compiling,
    Completed,
    Failed,
}

impl std::fmt::Display for ReportStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = serde_json::to_value(self)
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_default();
        f.write_str(&s)
    }
}

/// A stored report record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub id: String,
    pub org_id: String,
    pub topic: String,
    pub report_type: String,
    pub format: String,
    pub status: String,
    pub audience: Option<String>,
    pub depth: String,
    pub content_md: Option<String>,
    pub pdf_path: Option<String>,
    pub sources_json: Option<String>,
    pub metadata_json: Option<String>,
    pub error: Option<String>,
    pub created_at: String,
    pub completed_at: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_type_roundtrip() {
        let types = vec![
            ReportType::LeadershipProfile,
            ReportType::CompanyDeepDive,
            ReportType::IndustryAnalysis,
            ReportType::TechAnalysis,
            ReportType::MarketReport,
            ReportType::General,
        ];
        for t in &types {
            let json = serde_json::to_string(t).unwrap();
            let back: ReportType = serde_json::from_str(&json).unwrap();
            assert_eq!(*t, back);
        }
    }

    #[test]
    fn report_format_roundtrip() {
        for f in &[ReportFormat::Markdown, ReportFormat::Pdf] {
            let json = serde_json::to_string(f).unwrap();
            let back: ReportFormat = serde_json::from_str(&json).unwrap();
            assert_eq!(*f, back);
        }
    }

    #[test]
    fn report_status_roundtrip() {
        let statuses = vec![
            ReportStatus::Pending,
            ReportStatus::Researching,
            ReportStatus::Generating,
            ReportStatus::Compiling,
            ReportStatus::Completed,
            ReportStatus::Failed,
        ];
        for s in &statuses {
            let json = serde_json::to_string(s).unwrap();
            let back: ReportStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(*s, back);
        }
    }

    #[test]
    fn report_request_deserialize_defaults() {
        let json = r#"{"topic":"test","report_type":"general"}"#;
        let req: ReportRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.topic, "test");
        assert_eq!(req.format, ReportFormat::Markdown);
        assert_eq!(req.depth, ReportDepth::Standard);
        assert_eq!(req.org_id, "convergio.io");
    }

    #[test]
    fn report_type_labels_not_empty() {
        assert!(!ReportType::General.label().is_empty());
        assert!(!ReportType::CompanyDeepDive.label().is_empty());
    }
}
