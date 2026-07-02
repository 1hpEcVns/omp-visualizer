use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Json, Response},
    routing::get,
    Router,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tower_http::compression::CompressionLayer;
use flate2::write::GzEncoder;
use std::io::Write;

use crate::models::*;
use crate::session::{parser, store};
use crate::session::index::SessionIndex;

// ── Application State ──────────────────────────────────────────────

#[derive(Clone)]
pub struct AppState {
    pub sessions_dir: std::path::PathBuf,
    pub templates: Arc<tera::Tera>,
    pub index: Arc<Mutex<SessionIndex>>,
}

// ── Page Routes ────────────────────────────────────────────────────

pub fn page_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(dashboard))
        .route("/conversation/{session_id}", get(conversation_page))
        .route("/conversation/omp/{session_id}", get(conversation_page))
}

pub fn api_routes() -> Router<AppState> {
    Router::new()
        .route("/sessions", get(api_sessions))
        .route("/conversation/omp/{session_id}", get(api_conversation))
        .route("/conversation/omp/{session_id}/timeline", get(api_timeline))
        .route("/conversation/omp/{session_id}/track/{track_id}", get(api_track))
        .route("/conversation/omp/{session_id}/message", get(api_message))
        .route("/conversation/omp/{session_id}/raw_event", get(api_raw_event))
        .route("/conversation/omp/{session_id}/search", get(api_search))
        .layer(CompressionLayer::new())
}

pub fn default_state() -> AppState {
    let sessions_dir = store::default_sessions_dir();

    let template_dirs = ["templates", "backend/templates", "../backend/templates"];
    let mut tera = None;
    for dir in &template_dirs {
        let glob = format!("{}/**/*.html", dir);
        match tera::Tera::new(&glob) {
            Ok(t) => { tera = Some(t); break; }
            Err(_) => continue,
        }
    }
    let mut tera = tera.unwrap_or_else(|| {
        tracing::warn!("No templates found, using empty Tera");
        tera::Tera::default()
    });

    let index = SessionIndex::open(None)
        .unwrap_or_else(|e| {
            tracing::warn!("Could not open session index: {}", e);
            // Create in-memory fallback
            SessionIndex::open(Some(":memory:")).unwrap()
        });

    AppState {
        sessions_dir,
        templates: Arc::new(tera),
        index: Arc::new(Mutex::new(index)),
    }
}

// ── Dashboard Page ─────────────────────────────────────────────────

async fn dashboard(State(state): State<AppState>) -> impl IntoResponse {
    let sessions = store::list_sessions(&state.sessions_dir);

    let mut context = tera::Context::new();
    context.insert("title", "OMP Visualizer");
    context.insert("sessions", &sessions);

    match state.templates.render("dashboard.html", &context) {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            tracing::error!("Dashboard template error: {}", e);
            Html(dashboard_fallback(&sessions)).into_response()
        }
    }
}

fn dashboard_fallback(sessions: &[ConversationSummary]) -> String {
    let mut html = String::from(r#"<!doctype html><html><head><meta charset="UTF-8"><title>OMP Visualizer</title>
<link rel="stylesheet" href="/static/css/base.css"></head><body><main class="dashboard-shell">
<header class="app-header"><h1>OMP Visualizer</h1></header>
<section><ul class="session-list">"#);

    for s in sessions {
        html.push_str(&format!(
            r#"<li class="session-card"><a href="/conversation/omp/{}"><h3>{}</h3><p>{}</p></a></li>"#,
            s.id,
            s.title.as_deref().unwrap_or("Untitled"),
            s.directory.as_deref().unwrap_or("")
        ));
    }

    html.push_str("</ul></section></main></body></html>");
    html
}

// ── Conversation Page ──────────────────────────────────────────────

#[derive(Deserialize)]
struct ConversationPageParams {
    layout: Option<String>,
}

async fn conversation_page(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Query(params): Query<ConversationPageParams>,
) -> impl IntoResponse {
    let sessions = store::list_sessions(&state.sessions_dir);
    let summary = sessions.iter().find(|s| s.id == session_id);

    let initial_layout = match params.layout.as_deref() {
        Some("waterfall") | Some("focus") | Some("reader") => "reader",
        Some("timeline") | Some("graph") => "graph",
        _ => "graph",
    };

    if let Some(summary) = summary {
        let mut context = tera::Context::new();
        context.insert("conversation", &serde_json::json!({
            "summary": {
                "id": summary.id,
                "title": summary.title,
                "directory": summary.directory,
                "git_branch": summary.git_branch,
            }
        }));
        context.insert("agent", "omp");
        context.insert("slim_payload", "true");
        context.insert("initial_layout", initial_layout);
        context.insert("source", &serde_json::json!({
            "display_path": summary.directory.as_deref().unwrap_or(""),
        }));
        context.insert("back_href", "/");

        match state.templates.render("conversation.html", &context) {
            Ok(html) => Html(html).into_response(),
            Err(e) => {
                tracing::error!("Template error: {}", e);
                Html(conversation_page_fallback(summary)).into_response()
            }
        }
    } else {
        (StatusCode::NOT_FOUND, "Session not found").into_response()
    }
}

fn conversation_page_fallback(summary: &ConversationSummary) -> String {
    format!(r#"<!doctype html><html><head><meta charset="UTF-8">
<title>{} | OMP Visualizer</title>
<link rel="stylesheet" href="/static/css/base.css">
</head><body>
<div class="workbench-shell dual-workbench"
     data-testid="conversation-workbench"
     data-agent="omp"
     data-session-id="{}"
     data-slim-payload="true"
     data-default-layout="graph"
     data-layout="graph">
<header class="command-bar">
  <a class="back-link" href="/">Sessions</a>
  <h1>{}</h1>
</header>
<main id="transcript" class="transcript-surface">
  <section id="graphLayout" class="timeline-layout">
    <div id="graphViewport" class="timeline-viewport">
      <div id="sessionLoadingSkeleton" class="timeline-boot-skeleton">
        <div class="timeline-boot-skeleton-header">
          <div class="timeline-boot-status">Loading session data…</div>
        </div>
      </div>
      <div id="graphSizer" class="timeline-sizer"></div>
      <div id="graphLayer" class="timeline-layer">
        <svg id="graphEdges" class="timeline-connectors"></svg>
        <div id="graphLanes" class="timeline-tracks"></div>
        <div id="graphCapsules" class="timeline-blocks"></div>
      </div>
    </div>
  </section>
</main>
</div>
<div id="timelineDetailPanel" class="timeline-detail-dock hidden"></div>
<script src="/static/js/text_utils.js"></script>
<script src="/static/js/timeline_renderer.js"></script>
<script src="/static/js/opencode_renderer.js"></script>
<script src="/static/js/conversation.js"></script>
</body></html>"#,
        summary.title.as_deref().unwrap_or("Untitled"),
        summary.id,
        summary.title.as_deref().unwrap_or("Untitled"),
    )
}

// ── API: Session Listing ───────────────────────────────────────────

#[derive(Deserialize)]
struct SessionsQuery {
    agent: Option<String>,
    q: Option<String>,
    directory: Option<String>,
}

async fn api_sessions(
    State(state): State<AppState>,
    Query(params): Query<SessionsQuery>,
) -> Json<Vec<ConversationSummary>> {
    let sessions = store::list_sessions(&state.sessions_dir);
    let filtered = store::filter_sessions(
        &sessions,
        &params.q.unwrap_or_default(),
        &params.directory.unwrap_or_default(),
    );
    Json(filtered)
}

// ── API: Conversation (slim export) ────────────────────────────────

#[derive(Deserialize)]
struct ConversationQuery {
    slim: Option<bool>,
}

async fn api_conversation(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Query(params): Query<ConversationQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let slim = params.slim.unwrap_or(true);
    let sid = session_id.clone();
    let export = tokio::task::spawn_blocking(move || {
        build_conversation_export(&state, &sid)
    }).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)??;

    if slim {
        Ok(Json(slim_export_json(&export)))
    } else {
        Ok(Json(serde_json::to_value(&export).unwrap_or_default()))
    }
}

// ── API: Timeline Boot ─────────────────────────────────────────────

async fn api_timeline(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    req: axum::http::Request<axum::body::Body>,
) -> Result<Response, StatusCode> {
    // Check if we have a cached boot payload
    let session_file = find_session_file(&state.sessions_dir, &session_id)?;
    let fingerprint = SessionIndex::compute_fingerprint(&session_file);

    // Check cache
    if let Some(fp) = &fingerprint {
        let index = state.index.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        if let Some(cached) = index.get_cached_boot(&session_id, fp) {
            drop(index);
            // Check If-None-Match for 304
            let etag = format!("\"{}\"", fp);
            if let Some(if_none_match) = req.headers().get("if-none-match") {
                if if_none_match.to_str().unwrap_or("") == etag {
                    return Ok(Response::builder()
                        .status(StatusCode::NOT_MODIFIED)
                        .header("etag", &etag)
                        .body(axum::body::Body::empty())
                        .unwrap());
                }
            }
            // Serve from cache
            let accept_encoding = req.headers()
                .get(header::ACCEPT_ENCODING)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");
            if accept_encoding.contains("gzip") {
                return Ok(Response::builder()
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::CONTENT_ENCODING, "gzip")
                    .header("etag", &etag)
                    .header("vary", "Accept-Encoding")
                    .body(axum::body::Body::from(cached))
                    .unwrap());
            } else {
                // Decompress cached gzip
                use std::io::Read;
                let mut decoder = flate2::read::GzDecoder::new(&cached[..]);
                let mut decompressed = Vec::new();
                decoder.read_to_end(&mut decompressed).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                return Ok(Response::builder()
                    .header(header::CONTENT_TYPE, "application/json")
                    .header("etag", &etag)
                    .body(axum::body::Body::from(decompressed))
                    .unwrap());
            }
        }
    }

    tracing::info!("Starting timeline build for {}", session_id);
    let payload = build_timeline_payload(&state, &session_id)?;
    tracing::info!("Timeline build complete");

    let body = serde_json::to_vec(&payload).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let etag = fingerprint.as_ref().map(|fp| format!("\"{}\"", fp));

    let accept_encoding = req.headers()
        .get(header::ACCEPT_ENCODING)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    // Store in cache for future requests
    if let (Some(fp), Some(etag_str)) = (&fingerprint, &etag) {
        if let Ok(index) = state.index.lock() {
            let mut encoder = GzEncoder::new(Vec::new(), flate2::Compression::new(6));
            if encoder.write_all(&body).is_ok() {
                if let Ok(compressed) = encoder.finish() {
                    let _ = index.store_boot(&session_id, fp, &compressed);
                }
            }
        }
    }

    let accept_encoding = req.headers()
        .get(header::ACCEPT_ENCODING)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let mut resp = Response::builder()
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(ref etag_str) = etag {
        resp = resp.header("etag", etag_str);
    }

    if accept_encoding.contains("gzip") {
        let mut encoder = GzEncoder::new(Vec::new(), flate2::Compression::new(6));
        encoder.write_all(&body).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let compressed = encoder.finish().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        Ok(resp
            .header(header::CONTENT_ENCODING, "gzip")
            .body(axum::body::Body::from(compressed))
            .unwrap())
    } else {
        Ok(resp
            .body(axum::body::Body::from(body))
            .unwrap())
    }
}

fn build_timeline_payload(
    state: &AppState,
    session_id: &str,
) -> Result<serde_json::Value, StatusCode> {
    tracing::info!("Finding session file for {}", session_id);
    let session_file = find_session_file(&state.sessions_dir, session_id)?;
    tracing::info!("Found session file: {:?}", session_file);
    let jsonl_file_rel = session_file
        .strip_prefix(&state.sessions_dir)
        .unwrap_or(&session_file)
        .to_string_lossy()
        .to_string();

    tracing::info!("Parsing session file...");
    let parsed = match parser::parse_session_file(&session_file, &state.sessions_dir, &jsonl_file_rel) {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("Parse error: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };
    tracing::info!("Parsed {} entries", parsed.entries.len());

    tracing::info!("Building capsule seeds...");
    let seeds = parser::build_capsule_seeds(
        &parsed.entries,
        session_id,
        &parsed.jsonl_file,
        "main",
        0,
    );
    tracing::info!("Built {} seeds", seeds.len());

    tracing::info!("Building JSON response...");
    Ok(serde_json::json!({
        "protocol": 2,
        "jsonl_file": parsed.jsonl_file,
        "summary": {
            "id": session_id,
            "title": parsed.header.title,
            "model": "Unknown",
            "messageCount": parsed.entries.len(),
            "subagentCount": 0,
        },
        "capsule_seeds": seeds,
        "message_count": 0,
        "raw_event_count": 0,
        "subagent_transcripts": [],
    }))
}

fn build_subagent_skeletons(
    session_file: &std::path::Path,
    session_id: &str,
    sessions_dir: &std::path::Path,
) -> Vec<serde_json::Value> {
    use std::fs;

    let stem = session_file.file_stem().unwrap_or_default().to_string_lossy();
    let artifact_dir = session_file.parent().map(|p| p.join(&*stem));
    let Some(artifact_dir) = artifact_dir else { return Vec::new() };
    if !artifact_dir.exists() || !artifact_dir.is_dir() {
        return Vec::new();
    }

    let mut skeletons = Vec::new();
    if let Ok(entries) = fs::read_dir(&artifact_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "jsonl").unwrap_or(false) {
                let agent_name = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
                if let Ok(parsed) = parser::parse_session_file(&path, sessions_dir, &path.to_string_lossy()) {
                    let sub_seeds = parser::build_capsule_seeds(
                        &parsed.entries,
                        &format!("{}-{}", session_id, agent_name),
                        &path.to_string_lossy(),
                        &agent_name,
                        0,
                    );

                    let ts = chrono::DateTime::parse_from_rfc3339(&parsed.header.timestamp)
                        .ok()
                        .map(|dt| dt.timestamp_millis());

                    let first_nav = serde_json::json!({
                        "sessionId": format!("{}-{}", session_id, agent_name),
                        "jsonlFile": path.to_string_lossy(),
                        "lineNumber": 1,
                        "eventIndex": 0,
                        "scope": "sub",
                        "agentPath": agent_name,
                        "elementType": "event",
                        "view": "rendered",
                    });

                    skeletons.push(serde_json::json!({
                        "summary": {
                            "id": format!("{}-{}", session_id, agent_name),
                            "title": parsed.header.title,
                            "directory": parsed.header.cwd,
                            "model": "Unknown",
                        },
                        "agent_type": agent_name,
                        "agent_description": parsed.header.title.as_deref().unwrap_or(""),
                        "message_count": sub_seeds.len(),
                        "raw_event_count": 0,
                        "problem_flag_count": 0,
                        "capsule_count": sub_seeds.len(),
                        "first_nav": first_nav,
                        "capsule_seeds": sub_seeds,
                        "subagent_transcripts": [],
                        "parent_task_nav": null,
                        "parent_result_nav": null,
                    }));
                }
            }
        }
    }

    skeletons
}

// ── API: Track ─────────────────────────────────────────────────────

async fn api_track(
    State(state): State<AppState>,
    Path((session_id, track_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let sid = session_id.clone();
    let tid = track_id.clone();
    if tid == "main" {
        let export = tokio::task::spawn_blocking(move || {
            build_conversation_export(&state, &sid)
        }).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)??;
        let payload = single_track_payload(&export, &tid);
        Ok(Json(payload))
    } else {
        // Subagent track: find and parse
        let session_file = find_session_file(&state.sessions_dir, &session_id)?;
        let stem = session_file.file_stem().unwrap_or_default().to_string_lossy();
        let artifact_dir = session_file.parent().map(|p| p.join(&*stem));
        if let Some(dir) = artifact_dir {
            let sub_path = dir.join(format!("{}.jsonl", track_id));
            if sub_path.exists() {
                if let Ok(parsed) = parser::parse_session_file(&sub_path, &state.sessions_dir, &sub_path.to_string_lossy()) {
                    let sub_export = conversation_export_from_parsed(
                        &parsed,
                        &format!("{}-{}", session_id, track_id),
                        &state.sessions_dir,
                        &track_id,
                        "sub",
                    );
                    let payload = single_track_payload(&sub_export, &track_id);
                    return Ok(Json(payload));
                }
            }
        }
        Err(StatusCode::NOT_FOUND)
    }
}

fn single_track_payload(export: &ConversationExport, agent_path: &str) -> serde_json::Value {
    serde_json::json!({
        "summary": export.summary,
        "agentPath": agent_path,
        "messages": export.messages,
        "raw_events": export.raw_events.iter().map(|re| {
            let raw = &re.raw;
            serde_json::json!({
                "id": re.id,
                "nav": re.nav,
                "type": raw.get("type").and_then(|v| v.as_str()).unwrap_or(""),
                "subtype": raw.get("subtype").and_then(|v| v.as_str()).unwrap_or(""),
                "timestamp": raw.get("timestamp").and_then(|v| v.as_str()),
                "parse_error": re.parse_error,
            })
        }).collect::<Vec<_>>(),
        "problem_flags": export.problem_flags,
        "parser_diagnostics": export.parser_diagnostics,
        "agent_type": export.agent_type,
        "agent_description": export.agent_description,
    })
}

// ── API: Single Message ────────────────────────────────────────────

#[derive(Deserialize)]
struct MessageQuery {
    track_id: String,
    line_number: i64,
}

async fn api_message(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Query(params): Query<MessageQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let sid = session_id.clone();
    let export = tokio::task::spawn_blocking(move || {
        build_conversation_export(&state, &sid)
    }).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)??;

    let track = if params.track_id == "main" {
        Some(&export)
    } else {
        export.subagent_transcripts.iter().find(|s| {
            s.agent_type.as_deref() == Some(&params.track_id)
        })
    };

    match track {
        Some(t) => {
            for msg in &t.messages {
                if let Some(ref nav) = msg.nav {
                    if nav.line_number == params.line_number {
                        return Ok(Json(serde_json::json!({
                            "message": msg,
                            "raw_event": null,
                        })));
                    }
                }
            }
            for re in &t.raw_events {
                if re.nav.line_number == params.line_number {
                    return Ok(Json(serde_json::json!({
                        "raw_event": {
                            "id": re.id,
                            "nav": re.nav,
                            "raw": re.raw,
                            "parse_error": re.parse_error,
                        }
                    })));
                }
            }
            Err(StatusCode::NOT_FOUND)
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}

// ── API: Raw Event ─────────────────────────────────────────────────

#[derive(Deserialize)]
struct RawEventQuery {
    jsonl_file: String,
    line_number: i64,
}

async fn api_raw_event(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Query(params): Query<RawEventQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    use std::io::BufRead;
    use std::fs;

    let full_path = state.sessions_dir.join(&params.jsonl_file);
    if !full_path.starts_with(&state.sessions_dir) {
        return Err(StatusCode::FORBIDDEN);
    }

    let file = fs::File::open(&full_path).map_err(|_| StatusCode::NOT_FOUND)?;
    let reader = std::io::BufReader::new(file);

    for (i, line) in reader.lines().enumerate() {
        let line_number = (i + 1) as i64;
        if line_number == params.line_number {
            if let Ok(line) = line {
                if let Ok(raw) = serde_json::from_str::<serde_json::Value>(&line) {
                    return Ok(Json(serde_json::json!({
                        "id": raw.get("id").and_then(|v| v.as_str()).unwrap_or(""),
                        "nav": {
                            "sessionId": session_id,
                            "jsonlFile": params.jsonl_file,
                            "lineNumber": line_number,
                            "eventIndex": 0,
                            "scope": "main",
                            "agentPath": "main",
                            "elementType": "event",
                            "view": "raw",
                        },
                        "raw": raw,
                        "parse_error": null,
                    })));
                }
            }
            break;
        }
    }

    Err(StatusCode::NOT_FOUND)
}

// ── API: Search ────────────────────────────────────────────────────

#[derive(Deserialize)]
struct SearchQuery {
    q: String,
    scope: Option<String>,
}

async fn api_search(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Query(params): Query<SearchQuery>,
) -> Result<Json<Vec<serde_json::Value>>, StatusCode> {
    let sid = session_id.clone();
    let export = tokio::task::spawn_blocking(move || {
        build_conversation_export(&state, &sid)
    }).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)??;
    let q_lower = params.q.to_lowercase();
    let scope = params.scope.as_deref().unwrap_or("all");
    let mut results = Vec::new();

    if scope == "all" || scope == "main" {
        let main_results = search_in_export(&export, &q_lower, "main");
        results.extend(main_results);
    }

    if scope == "all" || scope == "subagents" {
        for sub in &export.subagent_transcripts {
            let agent_path = sub.agent_type.as_deref().unwrap_or("sub");
            let sub_results = search_in_export(sub, &q_lower, agent_path);
            results.extend(sub_results);
        }
    }

    results.truncate(200);
    Ok(Json(results))
}

fn search_in_export(export: &ConversationExport, q: &str, track_id: &str) -> Vec<serde_json::Value> {
    let mut results = Vec::new();

    for msg in &export.messages {
        for (ci, part) in msg.parts.iter().enumerate() {
            if let Some(text) = &part.text {
                if let Some(pos) = text.to_lowercase().find(q) {
                    let context_start = pos.saturating_sub(40);
                    let context_end = (pos + q.len() + 60).min(text.len());
                    let source_text = &text[context_start..context_end];

                    let nav = msg.nav.as_ref().or(part.nav.as_ref());
                    results.push(serde_json::json!({
                        "key": format!("{}-{}-{}", msg.id.as_deref().unwrap_or(""), ci, track_id),
                        "trackId": track_id,
                        "kindLabel": format!("{} message", msg.role),
                        "lineLabel": nav.map(|n| format!("L{}", n.line_number)).unwrap_or_default(),
                        "sourceText": source_text,
                        "nav": nav,
                    }));
                }
            }
        }
    }

    results
}

// ── Helpers ────────────────────────────────────────────────────────

fn find_session_file(sessions_dir: &std::path::Path, session_id: &str) -> Result<std::path::PathBuf, StatusCode> {
    use std::fs;

    let dirs = fs::read_dir(sessions_dir).map_err(|_| StatusCode::NOT_FOUND)?;

    for dir_entry in dirs.flatten() {
        let dir_path = dir_entry.path();
        if !dir_path.is_dir() {
            continue;
        }
        if let Ok(files) = fs::read_dir(&dir_path) {
            for file_entry in files.flatten() {
                let path = file_entry.path();
                let fname = path.file_name().unwrap_or_default().to_string_lossy();
                if fname.ends_with(".jsonl") && fname.contains(session_id) {
                    return Ok(path);
                }
            }
        }
    }

    Err(StatusCode::NOT_FOUND)
}

fn build_conversation_export(state: &AppState, session_id: &str) -> Result<ConversationExport, StatusCode> {
    tracing::info!("Finding session file for {}", session_id);
    let session_file = find_session_file(&state.sessions_dir, session_id)?;
    tracing::info!("Found session file: {:?}", session_file);

    let jsonl_file_rel = session_file
        .strip_prefix(&state.sessions_dir)
        .unwrap_or(&session_file)
        .to_string_lossy()
        .to_string();

    let parsed = parser::parse_session_file(&session_file, &state.sessions_dir, &jsonl_file_rel)
        .map_err(|e| {
            tracing::error!("Parse error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(conversation_export_from_parsed(&parsed, session_id, &state.sessions_dir, "main", "main"))
}

fn conversation_export_from_parsed(
    parsed: &parser::ParsedSession,
    session_id: &str,
    sessions_dir: &std::path::Path,
    agent_path: &str,
    scope: &str,
) -> ConversationExport {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let home_path = std::path::Path::new(&home);

    let dir_name = parsed.file_path
        .parent()
        .and_then(|p| p.file_name())
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let decoded_dir = store::decode_session_dir(&dir_name, home_path);

    let ts_parsed = chrono::DateTime::parse_from_rfc3339(&parsed.header.timestamp)
        .ok()
        .map(|dt| dt.timestamp_millis());

    let summary = ConversationSummary {
        id: session_id.to_string(),
        title: parsed.header.title.clone(),
        directory: Some(decoded_dir),
        git_branch: None,
        version: parsed.header.version.map(|v| v.to_string()),
        project_id: None,
        parent_id: parsed.header.parent_session.clone(),
        time_created: ts_parsed,
        time_updated: ts_parsed,
        model: Some("Unknown".to_string()),
        message_count: parsed.entries.len() as i64,
        subagent_count: 0,
        first_problem: None,
    };

    let mut messages = Vec::new();
    let mut raw_events = Vec::new();
    let mut line_number: i64 = 2;
    let mut event_index: i64 = 0;

    for entry in &parsed.entries {
        line_number += 1;
        if matches!(entry, crate::models::SessionEntry::SessionHeader(_)) {
            continue;
        }

        let nav = NavAddress {
            session_id: session_id.to_string(),
            jsonl_file: parsed.jsonl_file.clone(),
            line_number,
            event_index,
            scope: scope.to_string(),
            agent_path: agent_path.to_string(),
            element_type: "event".to_string(),
            view: "rendered".to_string(),
            message_id: None,
            content_index: None,
            tool_use_id: None,
            json_pointer: None,
            problem_id: None,
        };

        match entry {
            crate::models::SessionEntry::Message(msg_entry) => {
                let msg = convert_message(msg_entry, Some(nav));
                messages.push(msg);
            }
            _ => {
                let raw = serde_json::to_value(entry).unwrap_or_default();
                let entry_id = raw.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();

                raw_events.push(RawEvent {
                    id: entry_id,
                    nav,
                    raw,
                    parse_error: None,
                });
            }
        }

        event_index += 1;
    }

    let subagent_transcripts = load_subagents(&parsed.file_path, session_id, sessions_dir);

    ConversationExport {
        summary,
        messages,
        subagent_transcripts,
        task_part_id: None,
        task_message_id: None,
        parent_task_nav: None,
        parent_result_nav: None,
        previous_sibling_nav: None,
        next_sibling_nav: None,
        relationship_hint: None,
        relationship_basis: None,
        agent_type: Some(agent_path.to_string()),
        agent_description: None,
        raw_events,
        parser_diagnostics: Vec::new(),
        problem_flags: Vec::new(),
        nav_index: Vec::new(),
    }
}

fn convert_message(entry: &crate::models::MessageEntry, nav: Option<NavAddress>) -> Message {
    use crate::models::ContentBlock;

    let parts: Vec<GenericPart> = entry.message.content.iter().enumerate().map(|(ci, block)| {
        let (part_type, text, tool, state, tool_use_id) = match block {
            ContentBlock::Text { text } => ("text", Some(text.clone()), None, None, None),
            ContentBlock::Thinking { thinking, .. } => ("reasoning", Some(thinking.clone()), None, None, None),
            ContentBlock::ToolCall { id, name, arguments, .. } => (
                "tool", None, Some(name.clone()),
                Some(serde_json::json!({"id": id, "name": name, "input": arguments})),
                Some(id.clone()),
            ),
            ContentBlock::ToolUse { id, name, input } => (
                "tool", None, Some(name.clone()),
                Some(serde_json::json!({"id": id, "name": name, "input": input})),
                Some(id.clone()),
            ),
            ContentBlock::ToolResult { tool_use_id, content, is_error } => {
                let text_content = content.as_ref().and_then(|c| {
                    if let Some(arr) = c.as_array() {
                        Some(arr.iter()
                            .filter_map(|b| b.get("text").and_then(|t| t.as_str()))
                            .collect::<Vec<_>>()
                            .join("\n"))
                    } else {
                        c.as_str().map(|s| s.to_string())
                    }
                });
                ("tool_result", text_content, None,
                 Some(serde_json::json!({"toolUseId": tool_use_id, "isError": is_error})),
                 Some(tool_use_id.clone()))
            },
            ContentBlock::Image { source } => ("image", None, None, Some(source.clone()), None),
            ContentBlock::ImageUrl { image_url } => ("image", None, None, Some(image_url.clone()), None),
        };

        GenericPart {
            id: Some(format!("part-{}", ci)),
            part_type: part_type.to_string(),
            text,
            tool,
            state,
            tokens: None,
            time_created: entry.message.timestamp,
            synthetic: None,
            nav: nav.as_ref().map(|n| NavAddress {
                content_index: Some(ci as i64),
                element_type: "part".to_string(),
                tool_use_id: tool_use_id.clone(),
                ..n.clone()
            }),
        }
    }).collect();

    Message {
        id: Some(entry.id.clone()),
        role: entry.message.role.clone(),
        agent: None,
        model: entry.message.model.as_ref().map(|m| {
            serde_json::json!({"providerID": entry.message.provider.as_deref().unwrap_or(""), "modelID": m})
        }),
        model_id: entry.message.model.clone(),
        time_created: entry.message.timestamp,
        time_updated: entry.message.timestamp,
        summary: None,
        finish: None,
        parts,
        nav,
    }
}

fn load_subagents(
    session_file: &std::path::Path,
    session_id: &str,
    sessions_dir: &std::path::Path,
) -> Vec<ConversationExport> {
    use std::fs;

    let stem = session_file.file_stem().unwrap_or_default().to_string_lossy();
    let artifact_dir = session_file.parent().map(|p| p.join(&*stem));

    let Some(artifact_dir) = artifact_dir else { return Vec::new() };
    if !artifact_dir.exists() || !artifact_dir.is_dir() {
        return Vec::new();
    }

    let mut subagents = Vec::new();
    if let Ok(entries) = fs::read_dir(&artifact_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "jsonl").unwrap_or(false) {
                let agent_name = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
                if let Ok(parsed) = parser::parse_session_file(&path, sessions_dir, &path.to_string_lossy()) {
                    let sub_export = conversation_export_from_parsed(
                        &parsed,
                        &format!("{}-{}", session_id, agent_name),
                        sessions_dir,
                        &agent_name,
                        "sub",
                    );
                    subagents.push(sub_export);
                }
            }
        }
    }

    subagents
}

fn slim_export_json(export: &ConversationExport) -> serde_json::Value {
    serde_json::json!({
        "summary": export.summary,
        "messages": export.messages,
        "raw_events": export.raw_events.iter().map(|re| {
            let raw = &re.raw;
            serde_json::json!({
                "id": re.id,
                "nav": re.nav,
                "type": raw.get("type").and_then(|v| v.as_str()).unwrap_or(""),
                "subtype": raw.get("subtype").and_then(|v| v.as_str()).unwrap_or(""),
                "timestamp": raw.get("timestamp").and_then(|v| v.as_str()),
                "parse_error": re.parse_error,
            })
        }).collect::<Vec<_>>(),
        "problem_flags": export.problem_flags,
        "parser_diagnostics": export.parser_diagnostics,
        "task_part_id": export.task_part_id,
        "task_message_id": export.task_message_id,
        "parent_task_nav": export.parent_task_nav,
        "parent_result_nav": export.parent_result_nav,
        "previous_sibling_nav": export.previous_sibling_nav,
        "next_sibling_nav": export.next_sibling_nav,
        "relationship_hint": export.relationship_hint,
        "relationship_basis": export.relationship_basis,
        "agent_type": export.agent_type,
        "agent_description": export.agent_description,
        "subagent_transcripts": export.subagent_transcripts.iter().map(|sub| {
            serde_json::json!({
                "summary": sub.summary,
                "agent_type": sub.agent_type,
                "agent_description": sub.agent_description,
                "task_part_id": sub.task_part_id,
                "task_message_id": sub.task_message_id,
                "parent_task_nav": sub.parent_task_nav,
                "parent_result_nav": sub.parent_result_nav,
                "message_count": sub.messages.len(),
                "raw_event_count": sub.raw_events.len(),
                "problem_flag_count": sub.problem_flags.len(),
                "capsule_count": sub.messages.len() + sub.raw_events.len(),
                "subagent_transcripts": [],
            })
        }).collect::<Vec<_>>(),
    })
}
