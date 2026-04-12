# ADR-036: CTT Report Service as Extension Crate

**Status:** Accepted
**Date:** 2026-04-08

## Context

Convergio Think Tank (CTT) reports were previously generated manually by
launching the `research-report-generator` agent persona in Claude Code.
This required a human operator to trigger web research, assemble prompts,
generate LaTeX, and compile PDFs — a 30-60 minute manual workflow per report.

The organization needed automated, API-driven report generation that any
agent or CLI user could trigger on demand.

## Decision

1. **New crate `convergio-reports`** following the Extension trait pattern.
   Self-contained: types, routes, engine, template, LaTeX, PDF compiler.

2. **API-first architecture** — all report generation accessible via
   `POST /api/reports/generate`. Generation runs asynchronously (tokio::spawn).
   Clients poll `GET /api/reports/:id` for status updates.

3. **Self-call pattern for inference** — the engine calls
   `POST /api/inference/complete` on localhost to leverage the existing
   inference routing (tier selection, budget, model health). This keeps
   `convergio-reports` decoupled from inference internals at the cost of
   an HTTP round-trip per LLM call.

4. **Dual output format** — Markdown always generated. PDF is optional,
   requiring `pdflatex` on the system. Graceful degradation: if pdflatex
   is absent, the report completes as Markdown with a warning.

5. **6 report types** with Morgan Stanley methodology:
   `leadership-profile`, `company-deep-dive`, `industry-analysis`,
   `tech-analysis`, `market-report`, `general`.

6. **CTT branding + AI disclaimer** mandatory on every report.
   Confidence indicators (Verified/Reported/Uncertain) on claims.

## Alternatives Considered

- **Inline in kernel/inference** — rejected: reports are a distinct domain,
  not an inference feature. Separate crate follows the Extension pattern.
- **External service** — rejected: reports need access to the inference API
  and DB. Running inside the daemon avoids auth/network complexity.
- **Agent-only (no API)** — rejected: the manual workflow was the problem.
  API enables both human CLI and agent MCP access.

## Consequences

- New DB table `reports` with async status tracking.
- CLI contract test updated to include `convergio-reports` routes.
- Doctor smoke test covers `/api/reports`.
- MCP tools exposed for agent consumption.
- `research-report-generator` agent prompt updated to reference the API (v1.5.0).
- Agent tier bumped from t3 → t1 (Opus) per Learning #26.
