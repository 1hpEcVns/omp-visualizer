use serde::{Deserialize, Serialize};

// ── OMP JSONL Entry Types ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionHeader {
    pub version: Option<u32>,
    pub id: String,
    pub timestamp: String,
    pub cwd: String,
    pub title: Option<String>,
    #[serde(rename = "titleSource")]
    pub title_source: Option<String>,
    #[serde(rename = "parentSession")]
    pub parent_session: Option<String>,
}

// Message content blocks - use untagged enum to avoid infinite recursion
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "thinking")]
    Thinking { thinking: String, #[serde(rename = "thinkingSignature")] thinking_signature: Option<String> },
    #[serde(rename = "toolCall")]
    ToolCall {
        id: String,
        name: String,
        arguments: serde_json::Value,
        #[serde(rename = "partialArgs", default)]
        partial_args: Option<String>,
        #[serde(rename = "streamIndex", default)]
        stream_index: Option<u32>,
    },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        #[serde(rename = "tool_use_id")]
        tool_use_id: String,
        content: Option<serde_json::Value>,
        #[serde(rename = "is_error", default)]
        is_error: bool,
    },
    #[serde(rename = "image")]
    Image {
        source: serde_json::Value,
    },
    #[serde(rename = "image_url")]
    ImageUrl {
        image_url: serde_json::Value,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub input: Option<u64>,
    pub output: Option<u64>,
    #[serde(rename = "cacheRead")]
    pub cache_read: Option<u64>,
    #[serde(rename = "cacheWrite")]
    pub cache_write: Option<u64>,
    pub cost: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub role: String,
    pub content: Vec<ContentBlock>,
    #[serde(default)]
    pub usage: Option<Usage>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub api: Option<String>,
    pub timestamp: Option<i64>,
    #[serde(rename = "toolCallId")]
    pub tool_call_id: Option<String>,
    #[serde(rename = "toolName")]
    pub tool_name: Option<String>,
    #[serde(rename = "isError")]
    pub is_error: Option<bool>,
}

// Raw omp entry types - use derive macro with skip for unknown types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SessionEntry {
    #[serde(rename = "session")]
    SessionHeader(SessionHeader),
    #[serde(rename = "title")]
    Title(TitleEntry),
    #[serde(rename = "message")]
    Message(MessageEntry),
    #[serde(rename = "thinking_level_change")]
    ThinkingLevelChange(ThinkingLevelChangeEntry),
    #[serde(rename = "model_change")]
    ModelChange(ModelChangeEntry),
    #[serde(rename = "service_tier_change")]
    ServiceTierChange(ServiceTierChangeEntry),
    #[serde(rename = "compaction")]
    Compaction(CompactionEntry),
    #[serde(rename = "branch_summary")]
    BranchSummary(BranchSummaryEntry),
    #[serde(rename = "custom")]
    Custom(CustomEntry),
    #[serde(rename = "custom_message")]
    CustomMessage(CustomMessageEntry),
    #[serde(rename = "label")]
    Label(LabelEntry),
    #[serde(rename = "ttsr_injection")]
    TtsrInjection(TtsrInjectionEntry),
    #[serde(rename = "session_init")]
    SessionInit(SessionInitEntry),
    #[serde(rename = "mode_change")]
    ModeChange(ModeChangeEntry),
    #[serde(rename = "mcp_tool_selection")]
    McpToolSelection(McpToolSelectionEntry),
    #[serde(rename = "title_change")]
    TitleChange(TitleChangeEntry),
}

// Concrete entry types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TitleEntry {
    pub v: Option<u32>,
    pub title: String,
    pub source: Option<String>,
    #[serde(rename = "updatedAt")]
    pub updated_at: Option<String>,
    pub pad: Option<String>,
    pub id: Option<String>,
    #[serde(rename = "parentId")]
    pub parent_id: Option<String>,
    pub timestamp: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageEntry {
    pub id: String,
    #[serde(rename = "parentId")]
    pub parent_id: Option<String>,
    pub timestamp: String,
    pub message: AgentMessage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkingLevelChangeEntry {
    pub id: String,
    #[serde(rename = "parentId")]
    pub parent_id: Option<String>,
    pub timestamp: String,
    #[serde(rename = "thinkingLevel")]
    pub thinking_level: String,
    pub configured: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelChangeEntry {
    pub id: String,
    #[serde(rename = "parentId")]
    pub parent_id: Option<String>,
    pub timestamp: String,
    pub model: String,
    pub role: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceTierChangeEntry {
    pub id: String,
    #[serde(rename = "parentId")]
    pub parent_id: Option<String>,
    pub timestamp: String,
    #[serde(rename = "serviceTier")]
    pub service_tier: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionEntry {
    pub id: String,
    #[serde(rename = "parentId")]
    pub parent_id: Option<String>,
    pub timestamp: String,
    pub summary: Option<String>,
    #[serde(rename = "shortSummary")]
    pub short_summary: Option<String>,
    #[serde(rename = "firstKeptEntryId")]
    pub first_kept_entry_id: Option<String>,
    #[serde(rename = "tokensBefore")]
    pub tokens_before: Option<u64>,
    pub details: Option<serde_json::Value>,
    #[serde(rename = "preserveData")]
    pub preserve_data: Option<serde_json::Value>,
    #[serde(rename = "fromExtension")]
    pub from_extension: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchSummaryEntry {
    pub id: String,
    #[serde(rename = "parentId")]
    pub parent_id: Option<String>,
    pub timestamp: String,
    #[serde(rename = "fromId")]
    pub from_id: String,
    pub summary: Option<String>,
    pub details: Option<serde_json::Value>,
    #[serde(rename = "fromExtension")]
    pub from_extension: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomEntry {
    pub id: String,
    #[serde(rename = "parentId")]
    pub parent_id: Option<String>,
    pub timestamp: String,
    #[serde(rename = "customType")]
    pub custom_type: String,
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomMessageEntry {
    pub id: String,
    #[serde(rename = "parentId")]
    pub parent_id: Option<String>,
    pub timestamp: String,
    #[serde(rename = "customType")]
    pub custom_type: String,
    pub content: Option<serde_json::Value>,
    pub display: Option<bool>,
    pub details: Option<serde_json::Value>,
    pub attribution: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelEntry {
    pub id: String,
    #[serde(rename = "parentId")]
    pub parent_id: Option<String>,
    pub timestamp: String,
    #[serde(rename = "targetId")]
    pub target_id: String,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsrInjectionEntry {
    pub id: String,
    #[serde(rename = "parentId")]
    pub parent_id: Option<String>,
    pub timestamp: String,
    #[serde(rename = "injectedRules")]
    pub injected_rules: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInitEntry {
    pub id: String,
    #[serde(rename = "parentId")]
    pub parent_id: Option<String>,
    pub timestamp: String,
    #[serde(rename = "systemPrompt")]
    pub system_prompt: Option<String>,
    pub task: Option<String>,
    pub tools: Option<Vec<String>>,
    #[serde(rename = "outputSchema")]
    pub output_schema: Option<serde_json::Value>,
    pub spawns: Option<String>,
    #[serde(rename = "readSummarize")]
    pub read_summarize: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeChangeEntry {
    pub id: String,
    #[serde(rename = "parentId")]
    pub parent_id: Option<String>,
    pub timestamp: String,
    pub mode: String,
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolSelectionEntry {
    pub id: String,
    #[serde(rename = "parentId")]
    pub parent_id: Option<String>,
    pub timestamp: String,
    #[serde(rename = "selectedToolNames")]
    pub selected_tool_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TitleChangeEntry {
    pub id: String,
    #[serde(rename = "parentId")]
    pub parent_id: Option<String>,
    pub timestamp: String,
    pub title: String,
    pub source: Option<String>,
}

// ── API Response Models (matching minelogue wire format) ───────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavAddress {
    #[serde(rename = "sessionId")]
    pub session_id: String,
    #[serde(rename = "jsonlFile")]
    pub jsonl_file: String,
    #[serde(rename = "lineNumber")]
    pub line_number: i64,
    #[serde(rename = "eventIndex")]
    pub event_index: i64,
    pub scope: String,
    #[serde(rename = "agentPath")]
    pub agent_path: String,
    #[serde(rename = "elementType")]
    pub element_type: String,
    pub view: String,
    #[serde(rename = "messageId")]
    pub message_id: Option<String>,
    #[serde(rename = "contentIndex")]
    pub content_index: Option<i64>,
    #[serde(rename = "toolUseId")]
    pub tool_use_id: Option<String>,
    #[serde(rename = "jsonPointer")]
    pub json_pointer: Option<String>,
    #[serde(rename = "problemId")]
    pub problem_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input: i64,
    pub output: i64,
    pub cache: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericPart {
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub part_type: String,
    pub text: Option<String>,
    pub tool: Option<String>,
    pub state: Option<serde_json::Value>,
    pub tokens: Option<TokenUsage>,
    pub time_created: Option<i64>,
    pub synthetic: Option<bool>,
    pub nav: Option<NavAddress>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    #[serde(rename = "providerID")]
    pub provider_id: Option<String>,
    #[serde(rename = "modelID")]
    pub model_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageSummary {
    pub title: Option<String>,
    pub diffs: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Option<String>,
    pub role: String,
    pub agent: Option<String>,
    pub model: Option<serde_json::Value>,
    #[serde(rename = "modelID")]
    pub model_id: Option<String>,
    pub time_created: Option<i64>,
    pub time_updated: Option<i64>,
    pub summary: Option<MessageSummary>,
    pub finish: Option<String>,
    pub parts: Vec<GenericPart>,
    pub nav: Option<NavAddress>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSummary {
    pub id: String,
    pub title: Option<String>,
    pub directory: Option<String>,
    #[serde(rename = "gitBranch")]
    pub git_branch: Option<String>,
    pub version: Option<String>,
    #[serde(rename = "projectID")]
    pub project_id: Option<String>,
    #[serde(rename = "parentId")]
    pub parent_id: Option<String>,
    pub time_created: Option<i64>,
    pub time_updated: Option<i64>,
    pub model: Option<String>,
    #[serde(rename = "messageCount")]
    pub message_count: i64,
    #[serde(rename = "subagentCount")]
    pub subagent_count: i64,
    #[serde(rename = "firstProblem")]
    pub first_problem: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawEvent {
    pub id: String,
    pub nav: NavAddress,
    pub raw: serde_json::Value,
    #[serde(rename = "parseError")]
    pub parse_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParserDiagnostic {
    pub id: String,
    pub severity: String,
    pub kind: String,
    pub message: String,
    pub nav: Option<NavAddress>,
    pub related: Vec<NavAddress>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblemFlag {
    pub id: String,
    pub severity: String,
    pub kind: String,
    pub reason: String,
    pub nav: NavAddress,
    #[serde(rename = "jsonPath")]
    pub json_path: Option<String>,
    pub related: Vec<NavAddress>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationExport {
    pub summary: ConversationSummary,
    pub messages: Vec<Message>,
    #[serde(rename = "subagentTranscripts")]
    pub subagent_transcripts: Vec<ConversationExport>,
    #[serde(rename = "taskPartId")]
    pub task_part_id: Option<String>,
    #[serde(rename = "taskMessageId")]
    pub task_message_id: Option<String>,
    #[serde(rename = "parentTaskNav")]
    pub parent_task_nav: Option<NavAddress>,
    #[serde(rename = "parentResultNav")]
    pub parent_result_nav: Option<NavAddress>,
    #[serde(rename = "previousSiblingNav")]
    pub previous_sibling_nav: Option<NavAddress>,
    #[serde(rename = "nextSiblingNav")]
    pub next_sibling_nav: Option<NavAddress>,
    #[serde(rename = "relationshipHint")]
    pub relationship_hint: Option<String>,
    #[serde(rename = "relationshipBasis")]
    pub relationship_basis: Option<String>,
    #[serde(rename = "agentType")]
    pub agent_type: Option<String>,
    #[serde(rename = "agentDescription")]
    pub agent_description: Option<String>,
    #[serde(rename = "rawEvents")]
    pub raw_events: Vec<RawEvent>,
    #[serde(rename = "parserDiagnostics")]
    pub parser_diagnostics: Vec<ParserDiagnostic>,
    #[serde(rename = "problemFlags")]
    pub problem_flags: Vec<ProblemFlag>,
    #[serde(rename = "navIndex")]
    pub nav_index: Vec<NavAddress>,
}
