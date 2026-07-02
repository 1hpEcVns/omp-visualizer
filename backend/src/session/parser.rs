use std::path::{Path, PathBuf};
use std::fs;
use std::io::{BufRead, BufReader};

use crate::models::*;

/// A parsed session file with its header and entries
pub struct ParsedSession {
    pub header: SessionHeader,
    pub entries: Vec<SessionEntry>,
    pub file_path: PathBuf,
    pub jsonl_file: String,  // relative path from sessions dir
}

/// Parse a single omp JSONL session file.
pub fn parse_session_file(path: &Path, _sessions_dir: &Path, jsonl_file: &str) -> Result<ParsedSession, String> {
    let file = fs::File::open(path).map_err(|e| format!("Cannot open {}: {}", path.display(), e))?;
    let reader = BufReader::new(file);

    let mut header: Option<SessionHeader> = None;
    let mut entries: Vec<SessionEntry> = Vec::new();

    for line in reader.lines() {
        let line = line.map_err(|e| format!("Read error: {}", e))?;
        if line.trim().is_empty() {
            continue;
        }

        match serde_json::from_str::<SessionEntry>(&line) {
            Ok(SessionEntry::SessionHeader(h)) => {
                tracing::info!("Session header found: id={}", h.id);
                if header.is_none() {
                    header = Some(h);
                }
            }
            Ok(entry) => {
                entries.push(entry);
            }
            Err(e) => {
                tracing::debug!("Skipping unparseable line in {}: {}", path.display(), e);
            }
        }
    }

    let header = header.ok_or_else(|| format!("No session header found in {}", path.display()))?;

    Ok(ParsedSession {
        header,
        entries,
        file_path: path.to_path_buf(),
        jsonl_file: jsonl_file.to_string(),
    })
}


/// Parse a single line as a SessionEntry. Returns None if unparseable.
pub fn parse_entry(line: &str) -> Option<SessionEntry> {
    serde_json::from_str::<SessionEntry>(line).ok()
}

/// Determine the kind string for a capsule seed (matching minelogue frontend expectations).
pub fn entry_kind(entry: &SessionEntry) -> &'static str {
    match entry {
        SessionEntry::Message(msg) => {
            match msg.message.role.as_str() {
                "user" | "developer" => "message.user",
                "assistant" => "message.assistant",
                "toolResult" | "tool_result" => "message.tool_result",
                _ => "message.other",
            }
        }
        SessionEntry::Compaction(_) => "raw.compaction",
        SessionEntry::BranchSummary(_) => "raw.branch_summary",
        SessionEntry::ModelChange(_) => "raw.model_change",
        SessionEntry::ThinkingLevelChange(_) => "raw.thinking_level_change",
        SessionEntry::ModeChange(_) => "raw.mode_change",
        SessionEntry::TitleChange(_) | SessionEntry::Title(_) => "raw.title_change",
        SessionEntry::Label(_) => "raw.label",
        SessionEntry::Custom(_) => "raw.custom",
        SessionEntry::CustomMessage(_) => "raw.custom_message",
        SessionEntry::TtsrInjection(_) => "raw.ttsr_injection",
        SessionEntry::SessionInit(_) => "raw.session_init",
        SessionEntry::McpToolSelection(_) => "raw.mcp_tool_selection",
        SessionEntry::ServiceTierChange(_) => "raw.service_tier_change",
        SessionEntry::SessionHeader(_) => "raw.session",
    }
}

/// Extract a 140-char preview from entry data for capsule seeds.
pub fn extract_preview(entry: &SessionEntry) -> String {
    match entry {
        SessionEntry::Message(msg) => {
            let role = &msg.message.role;
            let content_preview: String = msg.message.content.iter()
                .filter_map(|block| match block {
                    ContentBlock::Text { text } => Some(text.as_str()),
                    ContentBlock::Thinking { thinking, .. } => Some(thinking.as_str()),
                    ContentBlock::ToolCall { name: _, .. } => {
                        Some("")
                    },
                    ContentBlock::ToolUse { name: _, .. } => {
                        Some("")
                    },
                    ContentBlock::ToolResult { is_error, .. } => {
                        if *is_error { Some("[Error]") } else { Some("[Tool result]") }
                    },
                    ContentBlock::Image { .. } | ContentBlock::ImageUrl { .. } => Some("[Image]"),
                })
                .take(1)
                .next()
                .unwrap_or("")
                .to_string();

            if content_preview.len() > 140 {
                format!("{}: {:.137}...", role, content_preview)
            } else {
                format!("{}: {}", role, content_preview)
            }
        }
        SessionEntry::Compaction(c) => {
            c.summary.as_deref().unwrap_or("Compaction").to_string()
        }
        SessionEntry::BranchSummary(b) => {
            b.summary.as_deref().unwrap_or("Branch summary").to_string()
        }
        SessionEntry::ModelChange(m) => {
            format!("Model: {}", m.model)
        }
        SessionEntry::ThinkingLevelChange(t) => {
            format!("Thinking: {}", t.thinking_level)
        }
        SessionEntry::ModeChange(m) => {
            format!("Mode: {}", m.mode)
        }
        SessionEntry::TitleChange(t) => {
            format!("Title: {}", t.title)
        }
        SessionEntry::Title(t) => {
            format!("Title: {}", t.title)
        }
        SessionEntry::Label(l) => {
            format!("Label: {}", l.label.as_deref().unwrap_or("(clear)"))
        }
        SessionEntry::Custom(c) => {
            format!("Custom: {}", c.custom_type)
        }
        SessionEntry::CustomMessage(cm) => {
            format!("Custom msg: {}", cm.custom_type)
        }
        SessionEntry::TtsrInjection(ti) => {
            format!("TTSR: {} rules", ti.injected_rules.len())
        }
        SessionEntry::SessionInit(si) => {
            si.task.as_deref().unwrap_or("Session init").to_string()
        }
        SessionEntry::McpToolSelection(mts) => {
            format!("MCP tools: {}", mts.selected_tool_names.join(", "))
        }
        SessionEntry::ServiceTierChange(stc) => {
            format!("Service tier: {:?}", stc.service_tier)
        }
        SessionEntry::SessionHeader(_) => "Session".to_string()
    }
}


/// Build capsule seeds in the compact format expected by the minelogue frontend JS.
/// Format: { k, ln, ei, mid?, role?, parts?, pv, ts, rt?, rs?, pe? }
pub fn build_capsule_seeds(
    entries: &[SessionEntry],
    _session_id: &str,
    _jsonl_file: &str,
    _agent_path: &str,
    start_event_index: i64,
) -> Vec<serde_json::Value> {
    let mut line_number = 2i64; // Entries start at line 3 (line 1=title, line 2=session header)
    let mut event_index = start_event_index;
    let mut seeds = Vec::new();

    for entry in entries {
        line_number += 1;
        if matches!(entry, SessionEntry::SessionHeader(_)) {
            continue;
        }

        let ts_ms = entry_timestamp_ms(entry);
        let preview = extract_preview(entry);

        let seed = match entry {
            SessionEntry::Message(msg) => {
                let entry_id = &msg.id;
                let role = &msg.message.role;

                let parts: Vec<serde_json::Value> = msg.message.content.iter().enumerate().map(|(ci, block)| {
                    let (ptype, tool, state, error, elem_type) = part_descriptor(block);
                    let tool_use_id = block_tool_use_id(block);
                    serde_json::json!([
                        ptype,
                        tool,
                        state,
                        error,
                        elem_type,
                        ci,
                        tool_use_id,
                        serde_json::Value::Null,
                    ])
                }).collect();

                serde_json::json!({
                    "k": "m",
                    "ln": line_number,
                    "ei": event_index,
                    "mid": entry_id,
                    "role": role,
                    "parts": parts,
                    "pv": preview,
                    "ts": ts_ms,
                })
            }
            _ => {
                let raw_type = entry_raw_type(entry);
                let raw_subtype = entry_raw_subtype(entry);
                serde_json::json!({
                    "k": "r",
                    "ln": line_number,
                    "ei": event_index,
                    "rt": raw_type,
                    "rs": raw_subtype,
                    "pv": preview,
                    "ts": ts_ms,
                })
            }
        };

        seeds.push(seed);
        event_index += 1;
    }

    seeds
}

fn entry_timestamp_ms(entry: &SessionEntry) -> Option<i64> {
    let ts_str = match entry {
        SessionEntry::Message(m) => Some(&m.timestamp),
        SessionEntry::Compaction(c) => Some(&c.timestamp),
        SessionEntry::BranchSummary(b) => Some(&b.timestamp),
        SessionEntry::ModelChange(m) => Some(&m.timestamp),
        SessionEntry::ThinkingLevelChange(t) => Some(&t.timestamp),
        SessionEntry::ModeChange(m) => Some(&m.timestamp),
        SessionEntry::TitleChange(t) => Some(&t.timestamp),
        SessionEntry::Title(t) => t.timestamp.as_ref(),
        SessionEntry::Label(l) => Some(&l.timestamp),
        SessionEntry::Custom(c) => Some(&c.timestamp),
        SessionEntry::CustomMessage(cm) => Some(&cm.timestamp),
        SessionEntry::TtsrInjection(ti) => Some(&ti.timestamp),
        SessionEntry::SessionInit(si) => Some(&si.timestamp),
        SessionEntry::McpToolSelection(mts) => Some(&mts.timestamp),
        SessionEntry::ServiceTierChange(stc) => Some(&stc.timestamp),
        _ => None,
    };
    ts_str.and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.timestamp_millis())
}

fn part_descriptor(block: &ContentBlock) -> (&'static str, Option<String>, Option<serde_json::Value>, Option<bool>, &'static str) {
    match block {
        ContentBlock::Text { .. } => ("text", None, None, None, "part"),
        ContentBlock::Thinking { .. } => ("reasoning", None, None, None, "part"),
        ContentBlock::ToolCall { name, arguments, .. } => (
            "tool",
            Some(name.clone()),
            Some(serde_json::json!({"input": arguments})),
            None,
            "part",
        ),
        ContentBlock::ToolUse { name, input, .. } => (
            "tool",
            Some(name.clone()),
            Some(serde_json::json!({"input": input})),
            None,
            "part",
        ),
        ContentBlock::ToolResult { is_error, .. } => (
            "tool_result",
            None,
            None,
            Some(*is_error),
            "part",
        ),
        ContentBlock::Image { source } => (
            "image",
            None,
            Some(source.clone()),
            None,
            "part",
        ),
        ContentBlock::ImageUrl { image_url } => (
            "image",
            None,
            Some(image_url.clone()),
            None,
            "part",
        ),
    }
}

fn block_tool_use_id(block: &ContentBlock) -> Option<String> {
    match block {
        ContentBlock::ToolCall { id, .. } | ContentBlock::ToolUse { id, .. } => Some(id.clone()),
        _ => None,
    }
}

fn entry_raw_type(entry: &SessionEntry) -> &'static str {
    match entry {
        SessionEntry::Compaction(_) => "compaction",
        SessionEntry::BranchSummary(_) => "branch_summary",
        SessionEntry::ModelChange(_) => "model_change",
        SessionEntry::ThinkingLevelChange(_) => "thinking_level_change",
        SessionEntry::ModeChange(_) => "mode_change",
        SessionEntry::TitleChange(_) | SessionEntry::Title(_) => "title_change",
        SessionEntry::Label(_) => "label",
        SessionEntry::Custom(_) => "custom",
        SessionEntry::CustomMessage(_) => "custom_message",
        SessionEntry::TtsrInjection(_) => "ttsr_injection",
        SessionEntry::SessionInit(_) => "session_init",
        SessionEntry::McpToolSelection(_) => "mcp_tool_selection",
        SessionEntry::ServiceTierChange(_) => "service_tier_change",
        _ => "unknown",
    }
}

fn entry_raw_subtype(_entry: &SessionEntry) -> &'static str {
    ""
}
