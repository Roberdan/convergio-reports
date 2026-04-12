//! LaTeX template for CTT PDF report generation.
//!
//! Converts Markdown report content to a professional LaTeX document
//! with Morgan Stanley-inspired design: custom colors, headers/footers,
//! booktabs tables, and CTT branding.

use crate::template::{CTT_BRAND, CTT_SHORT, DISCLAIMER};
use crate::types::ReportType;

/// LaTeX preamble with CTT styling.
fn preamble(topic: &str, report_type: ReportType, date: &str) -> String {
    format!(
        r#"\documentclass[11pt,a4paper]{{article}}
\usepackage[utf8]{{inputenc}}
\usepackage[T1]{{fontenc}}
\usepackage{{lmodern}}
\usepackage[margin=1in]{{geometry}}
\usepackage{{xcolor}}
\usepackage{{fancyhdr}}
\usepackage{{titlesec}}
\usepackage{{booktabs}}
\usepackage{{hyperref}}
\usepackage{{enumitem}}
\usepackage{{tabularx}}

% CTT Colors (Morgan Stanley inspired)
\definecolor{{msblue}}{{HTML}}{{003366}}
\definecolor{{msgray}}{{HTML}}{{666666}}
\definecolor{{msgreen}}{{HTML}}{{006633}}
\definecolor{{msred}}{{HTML}}{{CC0000}}
\definecolor{{mslightgray}}{{HTML}}{{F5F5F5}}

% Section formatting
\titleformat{{\section}}{{\Large\bfseries\color{{msblue}}}}{{}}{{0em}}{{}}
\titleformat{{\subsection}}{{\large\bfseries\color{{msblue!80}}}}{{}}{{0em}}{{}}
\titleformat{{\subsubsection}}{{\normalsize\bfseries\color{{msgray}}}}{{}}{{0em}}{{}}

% Header/Footer
\pagestyle{{fancy}}
\fancyhf{{}}
\fancyhead[L]{{\small\color{{msblue}}\textbf{{{brand}}} | {label}}}
\fancyhead[R]{{\small\color{{msgray}}{date}}}
\fancyfoot[L]{{\small\color{{msgray}}{short} | {topic} | CONFIDENTIAL}}
\fancyfoot[R]{{\small\color{{msgray}}Page \thepage}}
\renewcommand{{\headrulewidth}}{{0.4pt}}
\renewcommand{{\footrulewidth}}{{0.4pt}}

% Hyperlinks
\hypersetup{{colorlinks=true,linkcolor=msblue,urlcolor=msblue,citecolor=msblue}}

\begin{{document}}
"#,
        brand = CTT_BRAND,
        short = CTT_SHORT,
        label = report_type.label(),
    )
}

/// Convert Markdown content to LaTeX body.
pub fn markdown_to_latex(
    content_md: &str,
    topic: &str,
    report_type: ReportType,
    date: &str,
) -> String {
    let mut latex = preamble(topic, report_type, date);

    // Title page
    latex.push_str(&format!(
        r#"\begin{{center}}
\vspace*{{2cm}}
{{\Huge\bfseries\color{{msblue}} {brand}}}\\[0.5cm]
{{\Large\color{{msgray}} {label}}}\\[1cm]
{{\large\textbf{{Subject:}} {topic}}}\\[0.3cm]
{{\large\textbf{{Report Date:}} {date}}}\\[0.3cm]
{{\large\textbf{{Data Cutoff:}} {date}}}\\[2cm]
{{\small\color{{msgray}} CONFIDENTIAL}}
\end{{center}}
\newpage
\tableofcontents
\newpage
"#,
        brand = CTT_BRAND,
        label = report_type.label(),
    ));

    // Convert Markdown body to LaTeX
    for line in content_md.lines() {
        latex.push_str(&convert_md_line(line));
        latex.push('\n');
    }

    // Disclaimer page
    latex.push_str(&format!(
        r#"
\newpage
\section*{{Disclaimer}}
\small
{disclaimer}
"#,
        disclaimer = latex_escape(DISCLAIMER),
    ));

    latex.push_str("\\end{document}\n");
    latex
}

/// Convert a single Markdown line to LaTeX.
fn convert_md_line(line: &str) -> String {
    if line.starts_with("# ") {
        String::new()
    } else if let Some(rest) = line.strip_prefix("## ") {
        format!("\\section{{{}}}", latex_escape(rest))
    } else if let Some(rest) = line.strip_prefix("### ") {
        format!("\\subsection{{{}}}", latex_escape(rest))
    } else if let Some(rest) = line.strip_prefix("#### ") {
        format!("\\subsubsection{{{}}}", latex_escape(rest))
    } else if let Some(rest) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
        format!("\\item {}", latex_escape(rest))
    } else if let Some(rest) = line.strip_prefix("> ") {
        format!(
            "\\begin{{quote}}\\textit{{{}}}\\end{{quote}}",
            latex_escape(rest)
        )
    } else if line.starts_with("---") {
        "\\vspace{0.5em}\\hrule\\vspace{0.5em}".to_string()
    } else if line.is_empty() {
        String::from("\n")
    } else {
        latex_escape(line)
    }
}

/// Escape special LaTeX characters.
///
/// Uses single-pass replacement to avoid double-escaping (e.g. `\` →
/// `\textbackslash{}` must not have its braces re-escaped).
fn latex_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + text.len() / 4);
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str(r"\textbackslash{}"),
            '&' => out.push_str(r"\&"),
            '%' => out.push_str(r"\%"),
            '$' => out.push_str(r"\$"),
            '#' => out.push_str(r"\#"),
            '_' => out.push_str(r"\_"),
            '{' => out.push_str(r"\{"),
            '}' => out.push_str(r"\}"),
            '~' => out.push_str(r"\textasciitilde{}"),
            '^' => out.push_str(r"\textasciicircum{}"),
            c => out.push(c),
        }
    }
    out
}

/// Generate the output filename slug.
pub fn topic_slug(topic: &str) -> String {
    topic
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preamble_contains_colors() {
        let p = preamble("Test", ReportType::General, "01 Jan 2026");
        assert!(p.contains("msblue"));
        assert!(p.contains("003366"));
        assert!(p.contains(CTT_BRAND));
    }

    #[test]
    fn markdown_to_latex_has_structure() {
        let md = "## Summary\nHello world\n### Details\nMore info";
        let latex = markdown_to_latex(md, "Test", ReportType::General, "01 Jan 2026");
        assert!(latex.contains(r"\section{Summary}"));
        assert!(latex.contains(r"\subsection{Details}"));
        assert!(latex.contains(r"\end{document}"));
        assert!(latex.contains("Disclaimer"));
    }

    #[test]
    fn latex_escape_special_chars() {
        assert_eq!(latex_escape("a & b"), r"a \& b");
        assert_eq!(latex_escape("100%"), r"100\%");
        assert_eq!(latex_escape("$10"), r"\$10");
    }

    #[test]
    fn latex_escape_backslash_no_double_escape() {
        // Backslash must not have its braces re-escaped
        let result = latex_escape(r"a\b");
        assert_eq!(result, r"a\textbackslash{}b");
        assert!(!result.contains(r"\{"), "braces were double-escaped");
    }

    #[test]
    fn topic_slug_formats_correctly() {
        assert_eq!(topic_slug("Dedalus Group"), "dedalus-group");
        assert_eq!(topic_slug("Roberto D'Angelo"), "roberto-d-angelo");
    }
}
