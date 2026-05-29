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
use std::collections::HashMap;
use std::sync::Arc;

/// Shared application state passed to axum handlers.
type AppState = Arc<(DcmDiffResult, bool)>; // (result, approx)

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
// Helpers: build detailed change data for modal comparison view
// ---------------------------------------------------------------------------

fn build_change_detail(
    old: &crate::value::Value,
    new: &crate::value::Value,
    block_type: &str,
    old_axis: Option<&[f64]>,
    new_axis: Option<&[f64]>,
) -> serde_json::Value {
    match (old, new) {
        (crate::value::Value::WERT(old_vals), crate::value::Value::WERT(new_vals)) => {
            let kind = if old_axis.is_some() {
                "curve"
            } else if old_vals.len() <= 1 {
                "scalar"
            } else {
                "multi"
            };
            let mut detail = serde_json::json!({
                "kind": kind,
                "block_type": block_type,
                "old_values": old_vals,
                "new_values": new_vals,
            });
            if let (Some(oa), Some(na)) = (old_axis, new_axis) {
                detail["old_axis"] = serde_json::json!(oa);
                detail["new_axis"] = serde_json::json!(na);
            }
            detail
        }
        (crate::value::Value::TEXT(old_texts), crate::value::Value::TEXT(new_texts)) => {
            let kind = if old_texts.len() <= 1 { "scalar" } else { "multi" };
            serde_json::json!({
                "kind": kind,
                "block_type": block_type,
                "old_values": old_texts,
                "new_values": new_texts,
            })
        }
        _ => {
            serde_json::json!({
                "kind": "mixed",
                "block_type": block_type,
            })
        }
    }
}

fn build_map_change_detail(detail: &crate::diff::MapChangeDetail) -> serde_json::Value {
    serde_json::json!({
        "kind": "map",
        "block_type": "GRUPPENKENNFELD",
        "dim_x": detail.old_values.dim_x,
        "dim_y": detail.old_values.dim_y,
        "old_values": detail.old_values.values,
        "new_values": detail.new_values.values,
        "old_x_axis": detail.old_values.x_axis,
        "new_x_axis": detail.new_values.x_axis,
        "old_y_axis": detail.old_values.y_axis,
        "new_y_axis": detail.new_values.y_axis,
        "x_axis_name": detail.old_values.x_axis_name,
        "y_axis_name": detail.old_values.y_axis_name,
    })
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

    let mut new_idx = 1usize;
    let mut deleted_idx = 1usize;
    let mut changed_idx = 1usize;
    let mut change_details: HashMap<String, serde_json::Value> = HashMap::new();

    for diff in &result.differences {
        match diff {
            DcmDiff::New { name, description } => {
                let desc = description.as_deref().unwrap_or("");
                new_items.push(serde_json::json!({
                    "index": new_idx,
                    "name": name,
                    "type": block_type_from_desc(desc),
                    "description": desc,
                }));
                new_idx += 1;
            }
            DcmDiff::Deleted { name, description } => {
                let desc = description.as_deref().unwrap_or("");
                deleted_items.push(serde_json::json!({
                    "index": deleted_idx,
                    "name": name,
                    "type": block_type_from_desc(desc),
                    "description": desc,
                }));
                deleted_idx += 1;
            }
            DcmDiff::Changed { name, old, new, description, old_axis, new_axis } => {
                let desc = description.as_deref().unwrap_or("");
                let btype = block_type_from_desc(desc);
                change_details.insert(name.clone(), build_change_detail(old, new, btype, old_axis.as_deref(), new_axis.as_deref()));
                changed_items.push(serde_json::json!({
                    "index": changed_idx,
                    "name": name,
                    "type": btype,
                    "change_summary": desc,
                }));
                changed_idx += 1;
            }
            DcmDiff::ChangedMap { name, description, detail } => {
                let desc = description.as_deref().unwrap_or("");
                let btype = block_type_from_desc(desc);
                change_details.insert(name.clone(), build_map_change_detail(detail));
                changed_items.push(serde_json::json!({
                    "index": changed_idx,
                    "name": name,
                    "type": btype,
                    "change_summary": desc,
                }));
                changed_idx += 1;
            }
        }
    }

    let details_json = serde_json::to_string(&change_details).unwrap_or_default();
    // Prevent "</script>" from breaking out of the embedded script tag
    let safe_details_json = details_json.replace("</", "<\\/");

    let ctx = serde_json::json!({
        "metadata": result.metadata,
        "summary": result.summary,
        "approx": approx,
        "new_items": new_items,
        "deleted_items": deleted_items,
        "changed_items": changed_items,
        "change_details_json": safe_details_json,
        "has_changes": !change_details.is_empty(),
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

/// Escape a field value for CSV per RFC 4180, with formula injection prevention.
fn csv_escape_field(s: &str) -> String {
    // Prevent formula injection in Excel/Sheets
    let sanitized = if s.starts_with(['=', '+', '-', '@']) {
        format!("'{}", s)
    } else {
        s.to_string()
    };
    if sanitized.contains(',') || sanitized.contains('"') || sanitized.contains('\n') || sanitized.contains('\r') {
        format!("\"{}\"", sanitized.replace('"', "\"\""))
    } else {
        sanitized
    }
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
        csv.push_str(&csv_escape_field(name));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::{DcmDiffResult, DiffMetadata, DiffSummary};
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::util::ServiceExt;

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
                    old_axis: None,
                    new_axis: None,
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

    // -- Unit tests for build_change_detail --

    #[test]
    fn test_build_change_detail_text_single() {
        let old = crate::value::Value::TEXT(vec!["SprgLvlEnum_None".into()]);
        let new = crate::value::Value::TEXT(vec!["SprgLvlEnum_Auto".into()]);
        let detail = build_change_detail(&old, &new, "FESTWERT", None, None);
        assert_eq!(detail["kind"], "scalar");
        assert_eq!(detail["old_values"][0], "SprgLvlEnum_None");
        assert_eq!(detail["new_values"][0], "SprgLvlEnum_Auto");
    }

    #[test]
    fn test_build_change_detail_text_multi() {
        let old = crate::value::Value::TEXT(vec!["a".into(), "b".into(), "c".into()]);
        let new = crate::value::Value::TEXT(vec!["a".into(), "x".into(), "c".into()]);
        let detail = build_change_detail(&old, &new, "FESTWERTEBLOCK", None, None);
        assert_eq!(detail["kind"], "multi");
        assert_eq!(detail["old_values"].as_array().unwrap().len(), 3);
        assert_eq!(detail["new_values"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn test_build_change_detail_wert_scalar() {
        let old = crate::value::Value::WERT(vec![1.0]);
        let new = crate::value::Value::WERT(vec![2.0]);
        let detail = build_change_detail(&old, &new, "FESTWERT", None, None);
        assert_eq!(detail["kind"], "scalar");
    }

    #[test]
    fn test_build_change_detail_wert_multi() {
        let old = crate::value::Value::WERT(vec![1.0, 2.0, 3.0]);
        let new = crate::value::Value::WERT(vec![1.0, 4.0, 3.0]);
        let detail = build_change_detail(&old, &new, "FESTWERTEBLOCK", None, None);
        assert_eq!(detail["kind"], "multi");
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
        // Verify change details JSON is embedded for modal
        assert!(body.contains("change-details"));
        assert!(body.contains("\"kind\":\"scalar\""));
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
