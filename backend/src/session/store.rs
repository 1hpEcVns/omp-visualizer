use std::path::{Path, PathBuf};
use std::fs;
use chrono::DateTime;

use crate::models::*;
use super::parser;

/// Represents a discovered session for listing
#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub id: String,
    pub title: Option<String>,
    pub cwd: String,
    pub timestamp: String,
    pub file_path: PathBuf,
    pub jsonl_file: String,
    pub message_count: i64,
    pub subagent_count: i64,
}

/// Get the default sessions directory
pub fn default_sessions_dir() -> PathBuf {
    let home = dirs_fallback();
    home.join(".omp").join("agent").join("sessions")
}

fn dirs_fallback() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}

/// Decode a session directory name back to a human-readable path.
/// Encoding: `/`, `\`, `:` → `-`; home-relative → `-<rel>`; tmp → `-tmp-<rel>`
pub fn decode_session_dir(encoded: &str, home: &Path) -> String {
    if encoded == "-" {
        return home.display().to_string();
    }
    if let Some(rel) = encoded.strip_prefix("-tmp-") {
        let tmp = std::env::temp_dir();
        let decoded = rel.replace('-', "/");
        return format!("{}/{}", tmp.display(), decoded);
    }
    if let Some(rel) = encoded.strip_prefix('-') {
        let decoded = rel.replace('-', "/");
        return format!("{}/{}", home.display(), decoded);
    }
    // Legacy absolute form: --<path>--
    if encoded.starts_with("--") && encoded.ends_with("--") {
        let inner = &encoded[2..encoded.len()-2];
        let decoded = inner.replace('-', "/");
        return format!("/{}", decoded);
    }
    encoded.to_string()
}

/// List all sessions from the sessions directory.
pub fn list_sessions(sessions_dir: &Path) -> Vec<ConversationSummary> {
    let mut summaries = Vec::new();
    let home = dirs_fallback();

    let dirs = match fs::read_dir(sessions_dir) {
        Ok(d) => d,
        Err(e) => {
            tracing::warn!("Cannot read sessions dir {}: {}", sessions_dir.display(), e);
            return summaries;
        }
    };

    for dir_entry in dirs.flatten() {
        let dir_path = dir_entry.path();
        if !dir_path.is_dir() {
            continue;
        }

        let dir_name = dir_path.file_name().unwrap_or_default().to_string_lossy().to_string();
        let decoded_dir = decode_session_dir(&dir_name, &home);

        // Find JSONL files in this directory
        let jsonl_entries = match fs::read_dir(&dir_path) {
            Ok(d) => d,
            Err(_) => continue,
        };

        for file_entry in jsonl_entries.flatten() {
            let file_path = file_entry.path();
            if !file_path.is_file() {
                continue;
            }
            let fname = file_path.file_name().unwrap_or_default().to_string_lossy();
            if !fname.ends_with(".jsonl") {
                continue;
            }

            // Read header to extract metadata
            match read_session_metadata(&file_path) {
                Ok(info) => {
                    // Count subagent files (subdirectories or sibling .jsonl files)
                    let stem = file_path.file_stem().unwrap_or_default().to_string_lossy();
                    let artifact_dir = file_path.parent().map(|p| p.join(stem.as_ref())).unwrap_or_default();
                    let subagent_count = count_subagent_files(&artifact_dir);

                    let ts_parsed = DateTime::parse_from_rfc3339(&info.timestamp)
                        .ok()
                        .map(|dt| dt.timestamp_millis());

                    summaries.push(ConversationSummary {
                        id: info.id.clone(),
                        title: info.title.clone(),
                        directory: Some(decoded_dir.clone()),
                        git_branch: None,
                        version: Some("3".to_string()),
                        project_id: None,
                        parent_id: None,
                        time_created: ts_parsed,
                        time_updated: ts_parsed,
                        model: Some("Unknown".to_string()),
                        message_count: info.message_count,
                        subagent_count,
                        first_problem: None,
                    });
                }
                Err(e) => {
                    tracing::debug!("Skipping {}: {}", file_path.display(), e);
                }
            }
        }
    }

    // Sort by time_updated descending (most recent first)
    summaries.sort_by(|a, b| b.time_updated.cmp(&a.time_updated));
    summaries
}

/// Read just the header metadata from a session file (first ~4KB).
fn read_session_metadata(path: &Path) -> Result<SessionInfo, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Cannot read {}: {}", path.display(), e))?;

    // Find the session header line (it may not be line 1 if there's a title entry)
    let mut id = String::new();
    let mut title: Option<String> = None;
    let mut cwd = String::new();
    let mut timestamp = String::new();

    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(entry) = serde_json::from_str::<serde_json::Value>(line) {
            let entry_type = entry.get("type").and_then(|v| v.as_str()).unwrap_or("");
            match entry_type {
                "session" => {
                    id = entry.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    title = entry.get("title").and_then(|v| v.as_str()).map(|s| s.to_string());
                    cwd = entry.get("cwd").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    timestamp = entry.get("timestamp").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    break;
                }
                "title" => {
                    // title entries appear before session header in some files
                    if title.is_none() {
                        title = entry.get("title").and_then(|v| v.as_str()).map(|s| s.to_string());
                    }
                }
                _ => {}
            }
        }
    }

    if id.is_empty() {
        return Err("No session header found".to_string());
    }

    // Count messages (non-header, non-empty lines)
    let message_count = content.lines()
        .filter(|l| !l.trim().is_empty())
        .count() as i64 - 1; // minus header line

    // Relative jsonl_file path
    let sessions_dir = default_sessions_dir();
    let jsonl_file = path.strip_prefix(&sessions_dir)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string();

    Ok(SessionInfo {
        id,
        title,
        cwd,
        timestamp,
        file_path: path.to_path_buf(),
        jsonl_file,
        message_count: message_count.max(0),
        subagent_count: 0,
    })
}

fn count_subagent_files(artifact_dir: &Path) -> i64 {
    if !artifact_dir.exists() || !artifact_dir.is_dir() {
        return 0;
    }
    match fs::read_dir(artifact_dir) {
        Ok(entries) => entries
            .flatten()
            .filter(|e| {
                e.path().extension()
                    .map(|ext| ext == "jsonl")
                    .unwrap_or(false)
            })
            .count() as i64,
        Err(_) => 0,
    }
}

/// Filter sessions by query string (matches title, id, or directory).
pub fn filter_sessions(sessions: &[ConversationSummary], q: &str, directory: &str) -> Vec<ConversationSummary> {
    let q_lower = q.to_lowercase();
    let dir_lower = directory.to_lowercase();

    sessions.iter()
        .filter(|s| {
            let matches_q = q.is_empty()
                || s.id.to_lowercase().contains(&q_lower)
                || s.title.as_ref().map(|t| t.to_lowercase().contains(&q_lower)).unwrap_or(false)
                || s.directory.as_ref().map(|d| d.to_lowercase().contains(&q_lower)).unwrap_or(false);

            let matches_dir = directory.is_empty()
                || s.directory.as_ref().map(|d| d.to_lowercase().starts_with(&dir_lower)).unwrap_or(false);

            matches_q && matches_dir
        })
        .cloned()
        .collect()
}
