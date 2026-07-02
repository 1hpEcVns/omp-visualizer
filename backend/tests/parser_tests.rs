use omp_visualizer::session::parser;
use omp_visualizer::models::*;

#[test]
fn test_parse_session_header() {
    let line = r#"{"type":"session","version":3,"id":"test-1234","timestamp":"2026-01-01T00:00:00Z","cwd":"/test","title":"Test Session","titleSource":"auto"}"#;
    
    let entry = parser::parse_entry(line);
    assert!(entry.is_some());
    
    match entry.unwrap() {
        SessionEntry::SessionHeader(h) => {
            assert_eq!(h.id, "test-1234");
            assert_eq!(h.cwd, "/test");
            assert_eq!(h.title.unwrap(), "Test Session");
            assert_eq!(h.version, Some(3));
        }
        _ => panic!("Expected SessionHeader"),
    }
}

#[test]
fn test_parse_title_entry() {
    let line = r#"{"type":"title","v":1,"title":"My Title","source":"auto","updatedAt":"2026-01-01T00:00:00Z"}"#;
    
    let entry = parser::parse_entry(line);
    assert!(entry.is_some());
    
    match entry.unwrap() {
        SessionEntry::Title(t) => {
            assert_eq!(t.title, "My Title");
            assert_eq!(t.source.unwrap(), "auto");
        }
        _ => panic!("Expected Title"),
    }
}

#[test]
fn test_parse_message_entry() {
    let line = r#"{"type":"message","id":"msg001","parentId":null,"timestamp":"2026-01-01T00:00:00Z","message":{"role":"user","content":[{"type":"text","text":"Hello world"}]}}"#;
    
    let entry = parser::parse_entry(line);
    assert!(entry.is_some());
    
    match entry.unwrap() {
        SessionEntry::Message(m) => {
            assert_eq!(m.id, "msg001");
            assert_eq!(m.message.role, "user");
            assert_eq!(m.message.content.len(), 1);
        }
        _ => panic!("Expected Message"),
    }
}

#[test]
fn test_parse_thinking_level_change() {
    let line = r#"{"type":"thinking_level_change","id":"evt001","parentId":"msg001","timestamp":"2026-01-01T00:00:00Z","thinkingLevel":"high"}"#;
    
    let entry = parser::parse_entry(line);
    assert!(entry.is_some());
    
    match entry.unwrap() {
        SessionEntry::ThinkingLevelChange(t) => {
            assert_eq!(t.thinking_level, "high");
        }
        _ => panic!("Expected ThinkingLevelChange"),
    }
}

#[test]
fn test_parse_model_change() {
    let line = r#"{"type":"model_change","id":"evt002","parentId":"msg001","timestamp":"2026-01-01T00:00:00Z","model":"deepseek/deepseek-v4-pro","role":"default"}"#;
    
    let entry = parser::parse_entry(line);
    assert!(entry.is_some());
    
    match entry.unwrap() {
        SessionEntry::ModelChange(m) => {
            assert_eq!(m.model, "deepseek/deepseek-v4-pro");
            assert_eq!(m.role.unwrap(), "default");
        }
        _ => panic!("Expected ModelChange"),
    }
}

#[test]
fn test_parse_compaction() {
    let line = r#"{"type":"compaction","id":"cmp001","parentId":"msg001","timestamp":"2026-01-01T00:00:00Z","summary":"Conversation summary","shortSummary":"Short recap","firstKeptEntryId":"a1b2c3d4","tokensBefore":42000}"#;
    
    let entry = parser::parse_entry(line);
    assert!(entry.is_some());
    
    match entry.unwrap() {
        SessionEntry::Compaction(c) => {
            assert_eq!(c.summary.unwrap(), "Conversation summary");
            assert_eq!(c.short_summary.unwrap(), "Short recap");
            assert_eq!(c.tokens_before, Some(42000));
        }
        _ => panic!("Expected Compaction"),
    }
}

#[test]
fn test_parse_branch_summary() {
    let line = r#"{"type":"branch_summary","id":"br001","parentId":"a1b2c3d4","timestamp":"2026-01-01T00:00:00Z","fromId":"a1b2c3d4","summary":"Abandoned path"}"#;
    
    let entry = parser::parse_entry(line);
    assert!(entry.is_some());
    
    match entry.unwrap() {
        SessionEntry::BranchSummary(b) => {
            assert_eq!(b.summary.unwrap(), "Abandoned path");
            assert_eq!(b.from_id, "a1b2c3d4");
        }
        _ => panic!("Expected BranchSummary"),
    }
}

#[test]
fn test_parse_invalid_line() {
    let line = "not valid json at all";
    let entry = parser::parse_entry(line);
    assert!(entry.is_none());
}

#[test]
fn test_parse_unknown_type() {
    // Unknown types should fail to parse with derive macros
    let line = r#"{"type":"unknown_future_type","id":"x","parentId":null,"timestamp":"2026-01-01T00:00:00Z"}"#;
    let entry = parser::parse_entry(line);
    assert!(entry.is_none(), "Unknown entry types should be skipped");
}

#[test]
fn test_content_block_deserialization() {
    let json = r#"{"type":"text","text":"Hello"}"#;
    let block: Result<ContentBlock, _> = serde_json::from_str(json);
    assert!(block.is_ok());
    
    let json2 = r#"{"type":"thinking","thinking":"Let me think..."}"#;
    let block2: Result<ContentBlock, _> = serde_json::from_str(json2);
    assert!(block2.is_ok());
    
    let json3 = r#"{"type":"tool_use","id":"call_01","name":"read","input":{"path":"file.txt"}}"#;
    let block3: Result<ContentBlock, _> = serde_json::from_str(json3);
    assert!(block3.is_ok());
}

#[test]
fn test_extract_preview() {
    let line = r#"{"type":"message","id":"m1","parentId":null,"timestamp":"2026-01-01T00:00:00Z","message":{"role":"assistant","content":[{"type":"text","text":"The fix is to update the parser to handle unknown types gracefully."}]}}"#;
    let entry = parser::parse_entry(line).unwrap();
    let preview = parser::extract_preview(&entry);
    assert!(preview.contains("assistant:"));
    assert!(preview.contains("The fix is"));
}

#[test]
fn test_entry_kind() {
    let msg_line = r#"{"type":"message","id":"m1","parentId":null,"timestamp":"2026-01-01T00:00:00Z","message":{"role":"user","content":[{"type":"text","text":"hi"}]}}"#;
    let entry = parser::parse_entry(msg_line).unwrap();
    let kind = parser::entry_kind(&entry);
    assert_eq!(kind, "message.user");
    
    let model_line = r#"{"type":"model_change","id":"m2","parentId":null,"timestamp":"2026-01-01T00:00:00Z","model":"gpt-4"}"#;
    let entry2 = parser::parse_entry(model_line).unwrap();
    let kind2 = parser::entry_kind(&entry2);
    assert_eq!(kind2, "raw.model_change");
}

#[test]
fn test_build_capsule_seeds() {
    let entries = vec![
        parser::parse_entry(r#"{"type":"message","id":"m1","parentId":null,"timestamp":"2026-01-01T00:00:00Z","message":{"role":"user","content":[{"type":"text","text":"Hello"}]}}"#).unwrap(),
        parser::parse_entry(r#"{"type":"thinking_level_change","id":"t1","parentId":"m1","timestamp":"2026-01-01T00:00:01Z","thinkingLevel":"high"}"#).unwrap(),
    ];
    
    let seeds = parser::build_capsule_seeds(&entries, "session-1", "file.jsonl", "main", 0);
    assert_eq!(seeds.len(), 2);
    
    let first = &seeds[0];
    assert_eq!(first["k"], "m");
    assert_eq!(first["role"], "user");
    assert_eq!(first["mid"], "m1");
    assert!(first["parts"].as_array().unwrap().len() > 0);
    
    let second = &seeds[1];
    assert_eq!(second["k"], "r");
    assert_eq!(second["rt"], "thinking_level_change");
}

#[test]
fn test_tool_result_message() {
    let line = r#"{"type":"message","id":"m3","parentId":null,"timestamp":"2026-01-01T00:00:00Z","message":{"role":"toolResult","toolCallId":"call_01","toolName":"read","content":[{"type":"text","text":"file contents here"}],"isError":false}}"#;
    
    let entry = parser::parse_entry(line);
    assert!(entry.is_some());
    
    match entry.unwrap() {
        SessionEntry::Message(m) => {
            assert_eq!(m.message.role, "toolResult");
            assert_eq!(m.message.tool_call_id.as_deref(), Some("call_01"));
            assert_eq!(m.message.tool_name.as_deref(), Some("read"));
        }
        _ => panic!("Expected Message with toolResult"),
    }
}

#[test]
fn test_capsule_seed_tool_parts() {
    let entry = parser::parse_entry(r#"{"type":"message","id":"m4","parentId":null,"timestamp":"2026-01-01T00:00:00Z","message":{"role":"assistant","content":[{"type":"text","text":"Let me read the file"},{"type":"toolCall","id":"call_01","name":"read","arguments":{"path":"/tmp/test"}}]}}"#).unwrap();
    
    let seeds = parser::build_capsule_seeds(&[entry], "s1", "f.jsonl", "main", 0);
    assert_eq!(seeds.len(), 1);
    
    let parts = seeds[0]["parts"].as_array().unwrap();
    assert_eq!(parts.len(), 2);
    // First part: text
    assert_eq!(parts[0][0], "text");
    // Second part: tool
    assert_eq!(parts[1][0], "tool");
    assert_eq!(parts[1][1], "read");
}
