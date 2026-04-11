//! PDF compilation — runs pdflatex to produce the final report PDF.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Compile LaTeX content to PDF.
///
/// Writes .tex to a temp dir, runs pdflatex (2 passes for TOC),
/// and returns the path to the resulting PDF.
pub fn compile_pdf(
    latex_content: &str,
    output_dir: &Path,
    filename: &str,
) -> Result<PathBuf, String> {
    // Check pdflatex availability
    let pdflatex = find_pdflatex()?;

    // Ensure output dir exists
    fs::create_dir_all(output_dir).map_err(|e| format!("cannot create output dir: {e}"))?;

    let tex_path = output_dir.join(format!("{filename}.tex"));
    let pdf_path = output_dir.join(format!("{filename}.pdf"));

    fs::write(&tex_path, latex_content).map_err(|e| format!("cannot write .tex: {e}"))?;

    // Pass 1: generate TOC references
    run_pdflatex(&pdflatex, &tex_path, output_dir)?;
    // Pass 2: resolve TOC + page numbers
    run_pdflatex(&pdflatex, &tex_path, output_dir)?;

    if pdf_path.exists() {
        // Cleanup auxiliary files
        for ext in &["aux", "log", "out", "toc", "tex"] {
            let p = output_dir.join(format!("{filename}.{ext}"));
            let _ = fs::remove_file(p);
        }
        Ok(pdf_path)
    } else {
        Err("pdflatex ran but PDF was not produced".into())
    }
}

fn find_pdflatex() -> Result<String, String> {
    // Try common paths (macOS with MacTeX, Linux, generic)
    for path in &[
        "/Library/TeX/texbin/pdflatex",
        "/usr/local/texlive/2024/bin/universal-darwin/pdflatex",
        "/usr/bin/pdflatex",
        "pdflatex",
    ] {
        if Command::new(path)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return Ok(path.to_string());
        }
    }
    Err("pdflatex not found. Install MacTeX (macOS) or texlive (Linux) for PDF generation.".into())
}

fn run_pdflatex(pdflatex: &str, tex_path: &Path, output_dir: &Path) -> Result<(), String> {
    let output = Command::new(pdflatex)
        .args([
            "-interaction=nonstopmode",
            "-halt-on-error",
            &format!("-output-directory={}", output_dir.display()),
        ])
        .arg(tex_path)
        .output()
        .map_err(|e| format!("pdflatex exec failed: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stdout);
        // Extract the last error from the log
        let error_lines: Vec<&str> = stderr
            .lines()
            .filter(|l| l.starts_with('!') || l.contains("Error"))
            .take(5)
            .collect();
        let detail = if error_lines.is_empty() {
            "unknown error (check .log file)".to_string()
        } else {
            error_lines.join("; ")
        };
        return Err(format!("pdflatex failed: {detail}"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_pdflatex_returns_result() {
        // Just verify it doesn't panic — may or may not find pdflatex
        let _ = find_pdflatex();
    }
}
