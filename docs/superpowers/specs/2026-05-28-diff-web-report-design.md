# DCM Diff Web Report Design

## Overview

Add a `--web` flag to the `diff` subcommand that, when set, starts an embedded HTTP server to render diff results as an interactive web page. The page shows metadata, statistics, and three categorized tables (new, deleted, changed variables). Each category has a CSV export button that downloads variable names only. The existing JSON file output is preserved alongside.

## Motivation

The current diff command outputs only a JSON file and terminal summary. For calibration engineers reviewing large ECU parameter changes, a browser-based visual diff report is more productive — color-coded categories, sortable mental model, and the ability to export variable name lists for downstream tooling.

## Architecture

```
CLI: diff --dcm a.DCM --dcm b.DCM --web
  │
  ├──► Existing path (unchanged):
  │      compute DcmDiffResult
  │      write diff.json
  │      print terminal summary
  │
  └──► New path (when --web):
         Arc<DcmDiffResult>  ──► axum Router (shared state)
                                       │
         GET  /                    → serve.html.hbs rendered with diff data
         GET  /export/new          → CSV: new variable names
         GET  /export/deleted      → CSV: deleted variable names
         GET  /export/changed      → CSV: changed variable names
```

Server runs on `127.0.0.1:<random-free-port>`. Browser opens automatically. Ctrl+C stops the server.

## CLI Changes

Add one flag to the `Diff` subcommand:

```rust
/// Serve diff results as a web page instead of only writing JSON
#[arg(long, default_value_t = false)]
web: bool,
```

## New Module: `src/serve/`

```
src/serve/
├── mod.rs           # build_router(), start_server(), find_free_port()
└── serve.html.hbs   # Handlebars template (embedded via include_str!)
```

### mod.rs

- `find_free_port() -> u16` — bind to `127.0.0.1:0`, read assigned port, drop socket
- `build_router(data: Arc<DcmDiffResult>) -> Router` — axum Router with shared state
- `start_server(data: DcmDiffResult) -> Result<(), Box<dyn Error>>` — find port, build router, spawn server, open browser, block on ctrl_c

### Handlers

- `GET /` — render `serve.html.hbs` template with diff data as JSON context
- `GET /export/new` — filter `New` diffs, return CSV of names
- `GET /export/deleted` — filter `Deleted` diffs, return CSV of names
- `GET /export/changed` — filter `Changed + ChangedMap` diffs, return CSV of names

### Handlebars Template Data

The existing `DcmDiffResult` is already `Serialize`. The template receives it directly as context, plus three pre-computed lists for the tables:

```json
{
  "metadata": { ... },
  "summary": { "new_count": 12, "deleted_count": 5, "changed_count": 23, "total": 40 },
  "new_items": [ { "name": "VAR_0042", "type": "FESTWERT", "description": "..." } ],
  "changed_items": [ { "name": "VAR_0019", "type": "FESTWERT", "change_summary": "value: 1800 → 1850" } ],
  "deleted_items": [ { "name": "VAR_OLD1", "type": "FESTWERTEBLOCK", "description": "..." } ]
}
```

## Page Layout (5 Sections)

All dark theme, inline CSS in `<style>` tag. No external CSS/JS files. Zero JavaScript.

1. **Metadata** — original file, modified file, timestamp, approx mode
2. **Statistics Cards** — 4 colored cards: New (green), Deleted (red), Changed (yellow), Total (blue)
3. **New Variables Table** — `#`, Variable Name, Type, Description
4. **Changed Variables Table** — `#`, Variable Name, Type, Change Summary
5. **Deleted Variables Table** — `#`, Variable Name, Type, Description

Each category section header has an `<a>` export button pointing to the corresponding `/export/*` endpoint.

## CSV Export Format

Single-column CSV with header:

```csv
Variable Name
VAR_0042
VAR_0088
```

No type, description, or values — just names. Served with `Content-Type: text/csv` and `Content-Disposition: attachment; filename="new_variables.csv"`.

## Dependencies Added

```toml
axum = "0.8"
tokio = { version = "1", features = ["full"] }
open = "5"
```

Handlebars is already a project dependency — reused, no new dep needed.

## Error Handling

| Scenario | Behavior |
|----------|----------|
| Port finding fails | Print error, exit |
| Server bind fails | Print error, exit |
| Browser can't open | Print manual URL `Open http://localhost:<port> in your browser`, continue serving |
| Template render fails | Return 500 with plain-text error |
| Ctrl+C | Graceful shutdown via `axum::serve` graceful shutdown signal |

## Testing

- `find_free_port()` — unit-testable: binds, returns valid port
- `GET /` — integration test: start server, fetch `/`, assert 200 with expected HTML content
- `GET /export/new` — integration test: verify CSV content
- All tests use a synthetic `DcmDiffResult` with known data

## Not in Scope

- Sorting tables by clicking headers (requires JS)
- Filtering/searching within the page (requires JS)
- Pagination for large diff sets (page loads all at once)
- Live-reload or hot-reload
- HTTPS/TLS support
