# Diff Web Report Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `--web` flag to the `diff` subcommand that starts an embedded axum HTTP server rendering diff results as a browser page with CSV export buttons.

**Architecture:** New `src/serve/` module containing axum router + handlers + Handlebars template. The `diff` handler in `main.rs` gains a `--web` branch that calls `serve::start()` after the existing terminal+JSON output. The server shares the `DcmDiffResult` via `Arc` and serves one HTML page plus three CSV download endpoints.

**Tech Stack:** axum 0.8, tokio 1 (full), open 5, handlebars (existing)

---

## File Structure

| Action | File | Purpose |
|--------|------|---------|
| Modify | `Cargo.toml` | Add axum, tokio, open dependencies |
| Create | `src/serve/mod.rs` | Router, handlers, `start()` entry point |
| Create | `src/serve/serve.html.hbs` | Handlebars template for the report page |
| Modify | `src/lib.rs` | Add `pub mod serve;` |
| Modify | `src/main.rs` | Add `--web` flag to Diff subcommand, wire up serve call |

---

### Task 1: Add dependencies

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Add axum, tokio, open to Cargo.toml**

Add under `[dependencies]`:

```toml
axum = "0.8"
tokio = { version = "1", features = ["full"] }
open = "5"
```

- [ ] **Step 2: Run cargo check to pull deps**

```
cargo check
```

Expected: dependencies resolve and download. Unused import warnings are OK at this stage.

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "build: add axum, tokio, open for diff web report"
```

---

### Task 2: Create serve module (mod.rs)

**Files:**
- Create: `src/serve/mod.rs`

This is the core of the feature. Contains the axum router, four handlers, the `start()` entry point, and a helper to extract block types from diff descriptions.

- [ ] **Step 1: Write the serve module**

```rust
use crate::diff::{DcmDiff, DcmDiffResult};
use axum::{
    body::Body,
    extract::State,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use handlebars::Handlebars;
use std::sync::Arc;

/// Shared application state passed to axum handlers.
type AppState = Arc<(DcmDiffResult, bool)>;  // (result, approx)

// ---------------------------------------------------------------------------
// Helper: extract block type from a diff description string
// ---------------------------------------------------------------------------
// Description patterns:
//   New:        "New FESTWERT block 'NAME'"
//   Deleted:    "Deleted FESTWERTEBLOCK block 'NAME'"
//   Changed:    "FESTWERT 'NAME' value changed"
//   ChangedMap: "GRUPPENKENNFELD 'NAME' changed: dimensions: (3,7) -> (3,7)"
fn block_type_from_desc(desc: &str) -> &str {
    if desc.starts_with("New ") || desc.starts_with("Deleted ") {
        desc.split_whitespace().nth(1).unwrap_or("unknown")
    } else {
        desc.split_whitespace().next().unwrap_or("unknown")
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// GET / — render the main report page as HTML
async fn index(State(state): State<AppState>) -> Result<Response, AppError> {
    let (result, approx) = state.as_ref();
    let template = include_str!("serve.html.hbs");
    let mut reg = Handlebars::new();
    reg.register_template_string("report", template)
        .map_err(|e| AppError::Template(e.to_string()))?;

    // Build template context — pre-compute 1-based index for each item
    let mut new_items = Vec::new();
    let mut deleted_items = Vec::new();
    let mut changed_items = Vec::new();

    for (i, diff) in result.differences.iter().enumerate() {
        match diff {
            DcmDiff::New { name, description } => {
                let desc = description.as_deref().unwrap_or("");
                new_items.push(serde_json::json!({
                    "index": i + 1,
                    "name": name,
                    "type": block_type_from_desc(desc),
                    "description": desc,
                }));
            }
            DcmDiff::Deleted { name, description } => {
                let desc = description.as_deref().unwrap_or("");
                deleted_items.push(serde_json::json!({
                    "index": i + 1,
                    "name": name,
                    "type": block_type_from_desc(desc),
                    "description": desc,
                }));
            }
            DcmDiff::Changed { name, description, .. }
            | DcmDiff::ChangedMap { name, description, .. } => {
                let desc = description.as_deref().unwrap_or("");
                changed_items.push(serde_json::json!({
                    "index": i + 1,
                    "name": name,
                    "type": block_type_from_desc(desc),
                    "change_summary": desc,
                }));
            }
        }
    }

    let ctx = serde_json::json!({
        "metadata": result.metadata,
        "summary": result.summary,
        "approx": approx,
        "new_items": new_items,
        "deleted_items": deleted_items,
        "changed_items": changed_items,
    });

    let html = reg
        .render("report", &ctx)
        .map_err(|e| AppError::Template(e.to_string()))?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .body(Body::from(html))
        .unwrap())
}

/// GET /export/new — CSV of new variable names
async fn export_new(State(state): State<AppState>) -> Response {
    csv_response("new_variables.csv", &state.0.differences, |d| {
        matches!(d, DcmDiff::New { .. })
    })
}

/// GET /export/deleted — CSV of deleted variable names
async fn export_deleted(State(state): State<AppState>) -> Response {
    csv_response("deleted_variables.csv", &state.0.differences, |d| {
        matches!(d, DcmDiff::Deleted { .. })
    })
}

/// GET /export/changed — CSV of changed variable names
async fn export_changed(State(state): State<AppState>) -> Response {
    csv_response("changed_variables.csv", &state.0.differences, |d| {
        matches!(d, DcmDiff::Changed { .. } | DcmDiff::ChangedMap { .. })
    })
}

fn csv_response(
    filename: &str,
    diffs: &[DcmDiff],
    predicate: fn(&DcmDiff) -> bool,
) -> Response {
    let mut csv = String::from("Variable Name\n");
    for diff in diffs.iter().filter(|d| predicate(d)) {
        let name = match diff {
            DcmDiff::New { name, .. }
            | DcmDiff::Deleted { name, .. }
            | DcmDiff::Changed { name, .. }
            | DcmDiff::ChangedMap { name, .. } => name,
        };
        csv.push_str(name);
        csv.push('\n');
    }

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/csv; charset=utf-8")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", filename),
        )
        .body(Body::from(csv))
        .unwrap()
}

// ---------------------------------------------------------------------------
// App error type
// ---------------------------------------------------------------------------

enum AppError {
    Template(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::Template(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Template error: {}", msg),
            )
                .into_response(),
        }
    }
}

// ---------------------------------------------------------------------------
// Router + startup
// ---------------------------------------------------------------------------

fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/export/new", get(export_new))
        .route("/export/deleted", get(export_deleted))
        .route("/export/changed", get(export_changed))
        .with_state(state)
}

/// Start the web server on a random free port, open the browser, and block
/// until Ctrl+C. Returns Ok(()) on graceful shutdown.
pub fn start(result: DcmDiffResult, approx: bool) -> Result<(), Box<dyn std::error::Error>> {
    let state = Arc::new((result, approx));

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    rt.block_on(async {
        // Bind on port 0 to get a random free port via tokio (correct nonblocking)
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let port = listener.local_addr()?.port();
        let router = build_router(state);

        println!("Serving diff report at http://localhost:{}", port);
        println!("Press Ctrl+C to stop");

        // Open browser (best-effort)
        if let Err(e) = open::that(format!("http://localhost:{}", port)) {
            eprintln!(
                "Could not open browser automatically: {}. Open the URL manually.",
                e
            );
        }

        axum::serve(listener, router)
            .with_graceful_shutdown(shutdown_signal())
            .await?;

        Ok(())
    })
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to listen for Ctrl+C");
}
```

- [ ] **Step 2: Run cargo check to verify compilation**

```
cargo check
```

Expected: compile OK. May have warnings about unused items until wired up in main.

- [ ] **Step 3: Commit**

```bash
git add src/serve/mod.rs
git commit -m "feat: add serve module with axum router and handlers"
```

---

### Task 3: Create Handlebars template

**Files:**
- Create: `src/serve/serve.html.hbs`

- [ ] **Step 1: Write the template**

```html
<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>DCM Diff Report</title>
<style>
  *, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
  body {
    background: #0d1117; color: #c9d1d9; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
    padding: 24px; max-width: 1200px; margin: 0 auto;
  }
  h1 { font-size: 24px; margin-bottom: 24px; color: #f0f6fc; }
  h2 { font-size: 18px; margin-bottom: 12px; color: #f0f6fc; }

  .meta { background: #161b22; border: 1px solid #30363d; border-radius: 8px; padding: 16px; margin-bottom: 24px; }
  .meta table { width: 100%; border-collapse: collapse; }
  .meta td { padding: 4px 12px; font-size: 14px; }
  .meta td:first-child { color: #8b949e; width: 160px; }
  .meta td:last-child { font-family: monospace; color: #c9d1d9; }

  .stats { display: flex; gap: 16px; flex-wrap: wrap; margin-bottom: 32px; }
  .stat-card { flex: 1; min-width: 120px; border-radius: 8px; padding: 20px 16px; text-align: center; border: 1px solid; }
  .stat-card .num { font-size: 36px; font-weight: bold; }
  .stat-card .label { font-size: 13px; margin-top: 6px; text-transform: uppercase; letter-spacing: 1px; }
  .stat-new { background: #0d2818; border-color: #1a5c2a; }
  .stat-new .num { color: #4ade80; } .stat-new .label { color: #4ade80; }
  .stat-del { background: #2d1111; border-color: #5c1a1a; }
  .stat-del .num { color: #f87171; } .stat-del .label { color: #f87171; }
  .stat-chg { background: #2d2411; border-color: #5c4a1a; }
  .stat-chg .num { color: #fbbf24; } .stat-chg .label { color: #fbbf24; }
  .stat-tot { background: #0d1a2d; border-color: #1a3a5c; }
  .stat-tot .num { color: #93c5fd; } .stat-tot .label { color: #93c5fd; }

  .section { margin-bottom: 24px; }
  .section-header {
    display: flex; justify-content: space-between; align-items: center;
    padding: 10px 16px; border-radius: 8px 8px 0 0; border: 1px solid; border-bottom: none;
  }
  .section-header h2 { margin: 0; font-size: 15px; }
  .export-btn { text-decoration: none; border: 1px solid; padding: 4px 12px; border-radius: 4px; font-size: 12px; }

  .sec-new .section-header { background: #0d2818; border-color: #1a5c2a; } .sec-new h2 { color: #4ade80; } .sec-new .export-btn { color: #4ade80; border-color: #4ade80; }
  .sec-del .section-header { background: #2d1111; border-color: #5c1a1a; } .sec-del h2 { color: #f87171; } .sec-del .export-btn { color: #f87171; border-color: #f87171; }
  .sec-chg .section-header { background: #2d2411; border-color: #5c4a1a; } .sec-chg h2 { color: #fbbf24; } .sec-chg .export-btn { color: #fbbf24; border-color: #fbbf24; }

  table.diff-table { width: 100%; border-collapse: collapse; border: 1px solid #30363d; border-top: none; border-radius: 0 0 8px 8px; overflow: hidden; }
  table.diff-table th { padding: 6px 12px; text-align: left; color: #8b949e; font-weight: normal; font-size: 12px; border-bottom: 1px solid #21262d; background: #161b22; }
  table.diff-table td { padding: 5px 12px; font-size: 13px; border-bottom: 1px solid #21262d; }
  table.diff-table .col-num { color: #484f58; width: 40px; }
  table.diff-table .col-name { font-family: monospace; }
  .sec-new .col-name { color: #4ade80; }
  .sec-del .col-name { color: #f87171; }
  .sec-chg .col-name { color: #fbbf24; }

  .empty-msg { padding: 16px; color: #484f58; text-align: center; border: 1px solid #30363d; border-radius: 0 0 8px 8px; }
  .footer { margin-top: 24px; padding: 12px 16px; background: #161b22; border-radius: 8px; font-size: 12px; color: #484f58; }
</style>
</head>
<body>

<h1>DCM Diff Report</h1>

<!-- Metadata -->
<div class="meta">
  <table>
    <tr><td>Original (Left)</td><td>{{metadata.original_file}}</td></tr>
    <tr><td>Modified (Right)</td><td>{{metadata.modified_file}}</td></tr>
    <tr><td>Timestamp</td><td>{{metadata.timestamp}}</td></tr>
    <tr><td>Approx Mode</td><td>{{approx}}</td></tr>
  </table>
</div>

<!-- Statistics -->
<div class="stats">
  <div class="stat-card stat-new"><div class="num">{{summary.new_count}}</div><div class="label">New</div></div>
  <div class="stat-card stat-del"><div class="num">{{summary.deleted_count}}</div><div class="label">Deleted</div></div>
  <div class="stat-card stat-chg"><div class="num">{{summary.changed_count}}</div><div class="label">Changed</div></div>
  <div class="stat-card stat-tot"><div class="num">{{summary.total}}</div><div class="label">Total</div></div>
</div>

<!-- New Variables -->
<div class="section sec-new">
  <div class="section-header">
    <h2>+ {{summary.new_count}} New Variables</h2>
    <a class="export-btn" href="/export/new" download>&darr; Export CSV</a>
  </div>
  {{#if new_items}}
  <table class="diff-table">
    <tr><th class="col-num">#</th><th>Variable Name</th><th>Type</th><th>Description</th></tr>
    {{#each new_items}}
    <tr><td class="col-num">{{index}}</td><td class="col-name">{{name}}</td><td>{{type}}</td><td>{{description}}</td></tr>
    {{/each}}
  </table>
  {{else}}
  <div class="empty-msg">No new variables</div>
  {{/if}}
</div>

<!-- Changed Variables -->
<div class="section sec-chg">
  <div class="section-header">
    <h2>~ {{summary.changed_count}} Changed Variables</h2>
    <a class="export-btn" href="/export/changed" download>&darr; Export CSV</a>
  </div>
  {{#if changed_items}}
  <table class="diff-table">
    <tr><th class="col-num">#</th><th>Variable Name</th><th>Type</th><th>Change Summary</th></tr>
    {{#each changed_items}}
    <tr><td class="col-num">{{index}}</td><td class="col-name">{{name}}</td><td>{{type}}</td><td>{{change_summary}}</td></tr>
    {{/each}}
  </table>
  {{else}}
  <div class="empty-msg">No changed variables</div>
  {{/if}}
</div>

<!-- Deleted Variables -->
<div class="section sec-del">
  <div class="section-header">
    <h2>&minus; {{summary.deleted_count}} Deleted Variables</h2>
    <a class="export-btn" href="/export/deleted" download>&darr; Export CSV</a>
  </div>
  {{#if deleted_items}}
  <table class="diff-table">
    <tr><th class="col-num">#</th><th>Variable Name</th><th>Type</th><th>Description</th></tr>
    {{#each deleted_items}}
    <tr><td class="col-num">{{index}}</td><td class="col-name">{{name}}</td><td>{{type}}</td><td>{{description}}</td></tr>
    {{/each}}
  </table>
  {{else}}
  <div class="empty-msg">No deleted variables</div>
  {{/if}}
</div>

<div class="footer">Generated by dcm_utils &mdash; {{metadata.timestamp}}</div>

</body>
</html>
```

Note: template uses `{{index}}` for 1-based row numbers, pre-computed in the handler.

- [ ] **Step 2: Run cargo check**

```
cargo check
```

Expected: compile OK.

- [ ] **Step 3: Commit**

```bash
git add src/serve/serve.html.hbs src/serve/mod.rs
git commit -m "feat: add Handlebars template for diff report page"
```

---

### Task 4: Wire up --web flag in lib.rs and main.rs

**Files:**
- Modify: `src/lib.rs` (add `pub mod serve;`)
- Modify: `src/main.rs` (add `web` flag, call `serve::start()`)

- [ ] **Step 1: Register serve module in lib.rs**

Add after `pub mod gen;`:

```rust
pub mod serve;
```

- [ ] **Step 2: Add --web flag to Diff subcommand in main.rs**

In the `Diff` struct, add after the `approx` field:

```rust
/// Serve diff results as a web page
#[arg(long, default_value_t = false)]
web: bool,
```

- [ ] **Step 3: Add the web branch in the Diff handler**

After the JSON write block in the `Commands::Diff` arm (after line ~311 `std::fs::write(&output, json)`, before the closing `}`), add:

```rust
if web {
    println!();
    serve::start(result, approx).unwrap_or_else(|e| {
        eprintln!("Server error: {}", e);
        std::process::exit(1);
    });
}
```

Update the destructuring to include `web`:

```rust
Commands::Diff {
    dcm,
    a2l,
    hex,
    output,
    approx,
    web,
} => {
```

- [ ] **Step 4: Run cargo check**

```
cargo check
```

Expected: compile OK, no warnings.

- [ ] **Step 5: Commit**

```bash
git add src/lib.rs src/main.rs
git commit -m "feat: wire up --web flag to serve diff results as web page"
```

---

### Task 5: Tests

**Files:**
- Modify: `src/serve/mod.rs` (add `#[cfg(test)]` module)

- [ ] **Step 1: Add test module at the bottom of serve/mod.rs**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::{DcmDiffResult, DiffMetadata, DiffSummary};
    use axum::body::Body;
    use axum::http::{Request, StatusCode};

    fn make_result() -> DcmDiffResult {
        DcmDiffResult {
            metadata: DiffMetadata::new("left.DCM", "right.DCM"),
            summary: DiffSummary {
                new_count: 1,
                deleted_count: 1,
                changed_count: 1,
                total: 3,
            },
            differences: vec![
                DcmDiff::New {
                    name: "NEW_VAR".into(),
                    description: Some("New FESTWERT block 'NEW_VAR'".into()),
                },
                DcmDiff::Deleted {
                    name: "OLD_VAR".into(),
                    description: Some("Deleted FESTWERTEBLOCK block 'OLD_VAR'".into()),
                },
                DcmDiff::Changed {
                    name: "CHG_VAR".into(),
                    old: crate::value::Value::WERT(vec![1.0]),
                    new: crate::value::Value::WERT(vec![2.0]),
                    description: Some("FESTWERT 'CHG_VAR' value changed".into()),
                },
            ],
        }
    }

    // -- Unit tests for block_type_from_desc helper --

    #[test]
    fn test_block_type_from_desc_new() {
        assert_eq!(
            block_type_from_desc("New FESTWERT block 'VAR_0042'"),
            "FESTWERT"
        );
    }

    #[test]
    fn test_block_type_from_desc_deleted() {
        assert_eq!(
            block_type_from_desc("Deleted GRUPPENKENNLINIE block 'VAR_X'"),
            "GRUPPENKENNLINIE"
        );
    }

    #[test]
    fn test_block_type_from_desc_changed() {
        assert_eq!(
            block_type_from_desc("FESTWERT 'VAR_0019' value changed"),
            "FESTWERT"
        );
    }

    #[test]
    fn test_block_type_from_desc_changed_map() {
        assert_eq!(
            block_type_from_desc(
                "GRUPPENKENNFELD 'VAR_0530' changed: dimensions: (3, 7) -> (3, 7)"
            ),
            "GRUPPENKENNFELD"
        );
    }

    // -- HTTP integration tests using axum Router::oneshot --

    #[tokio::test]
    async fn test_index_returns_200_with_html() {
        let state = Arc::new((make_result(), false));
        let router = build_router(state);

        let request = Request::builder()
            .uri("/")
            .body(Body::empty())
            .unwrap();
        let response = router.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let content_type = response.headers().get("content-type").unwrap().to_str().unwrap();
        assert!(content_type.contains("text/html"));
        let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        let body = String::from_utf8(body_bytes.to_vec()).unwrap();
        assert!(body.contains("DCM Diff Report"));
        assert!(body.contains("left.DCM"));
        assert!(body.contains("right.DCM"));
        assert!(body.contains("NEW_VAR"));
        assert!(body.contains("CHG_VAR"));
        assert!(body.contains("OLD_VAR"));
    }

    #[tokio::test]
    async fn test_export_new_csv() {
        let state = Arc::new((make_result(), false));
        let router = build_router(state);

        let request = Request::builder()
            .uri("/export/new")
            .body(Body::empty())
            .unwrap();
        let response = router.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let content_type = response.headers().get("content-type").unwrap().to_str().unwrap();
        assert!(content_type.contains("text/csv"));
        let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        let body = String::from_utf8(body_bytes.to_vec()).unwrap();
        assert!(body.contains("Variable Name"));
        assert!(body.contains("NEW_VAR"));
        assert!(!body.contains("OLD_VAR")); // only new items
    }

    #[tokio::test]
    async fn test_export_deleted_csv() {
        let state = Arc::new((make_result(), false));
        let router = build_router(state);
        let request = Request::builder()
            .uri("/export/deleted")
            .body(Body::empty())
            .unwrap();
        let response = router.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        let body = String::from_utf8(body_bytes.to_vec()).unwrap();
        assert!(body.contains("OLD_VAR"));
        assert!(!body.contains("NEW_VAR"));
    }

    #[tokio::test]
    async fn test_export_changed_csv() {
        let state = Arc::new((make_result(), false));
        let router = build_router(state);

        let request = Request::builder()
            .uri("/export/changed")
            .body(Body::empty())
            .unwrap();
        let response = router.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
        let body = String::from_utf8(body_bytes.to_vec()).unwrap();
        assert!(body.contains("CHG_VAR"));
        assert!(!body.contains("NEW_VAR"));
        assert!(!body.contains("OLD_VAR"));
    }
}
```

- [ ] **Step 2: Run the tests**

```
cargo test serve
```

Expected: all 9 tests pass (4 unit + 5 HTTP integration).

- [ ] **Step 3: Commit**

```bash
git add src/serve/mod.rs
git commit -m "test: add unit and integration tests for serve module"
```

---

### Task 6: End-to-end verification

- [ ] **Step 1: Build release**

```
cargo build --release
```

Expected: clean build, no warnings.

- [ ] **Step 2: Run existing test suite to confirm no regressions**

```
cargo test
```

Expected: all existing tests pass.

- [ ] **Step 3: Manual smoke test with --web flag**

```
cargo run -- diff --dcm test-dcms/test_sample_673.DCM --dcm test-dcms/test_sample_677.DCM --web
```

Expected: terminal summary, JSON file written, browser opens with the diff report page. Verify:
- Metadata section shows both file paths
- Statistics cards show correct counts
- New/Deleted/Changed tables render
- Export CSV buttons trigger downloads
- Ctrl+C stops the server gracefully

- [ ] **Step 4: Test --web flag is off by default**

```
cargo run -- diff --dcm test-dcms/test_sample_673.DCM --dcm test-dcms/test_sample_677.DCM
```

Expected: same behavior as before (terminal + JSON, no server, no browser open).

- [ ] **Step 5: Final commit for any fixes from smoke test**

```bash
git add -A
git commit -m "chore: final adjustments from smoke test"
```
