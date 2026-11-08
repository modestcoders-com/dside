use serde::{Deserialize, Serialize};

/// Configuration for one engine turn. Mirrors the parts of the Mavis
/// `IAgentConfig` / `IModelRef` contract that we actually need.
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineConfig {
    /// Never accepted from the webview: the key is resolved in Rust from the OS keyring.
    #[serde(skip, default)]
    pub api_key: String,
    #[serde(default = "default_base_url")]
    pub base_url: String,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default)]
    pub system_prompt: String,
    #[serde(default)]
    pub workspace: String,
    /// "read" = no writes/commands, "ask" = approve each write/edit/bash, "full" = full access.
    #[serde(default = "default_permission")]
    pub permission: String,
    /// "all" or "allowlist". The allowlist applies to the model's bash tool only.
    #[serde(default = "default_command_policy")]
    pub command_policy: String,
    #[serde(default)]
    pub allowed_commands: Vec<String>,
    /// Reject write/edit outside the workspace.
    #[serde(default)]
    pub workspace_only: bool,
    /// The user allowed the partner to read and fill forms in the browser windows they opened.
    #[serde(default)]
    pub browser_control: bool,
    #[serde(default = "default_max_turns")]
    pub max_turns: u32,
    /// How hard the model thinks: the provider's own scale (`none` … `max`). Empty means
    /// "whatever the provider does by default", which is the honest default for us too.
    #[serde(default)]
    pub reasoning_effort: Option<String>,
    /// The model's own context window, as the provider reports it. Compression needs it to
    /// know when a conversation is getting too long to send whole.
    #[serde(default)]
    pub context_window: Option<i64>,
    /// Tools the mode takes away. They are not offered to the model, and a call that arrives
    /// anyway is refused instead of quietly running.
    #[serde(default)]
    pub disabled_tools: Vec<String>,
    /// How long a question or an approval waits for the user. Five minutes is a guess about
    /// somebody's attention; the setting exists because only the person knows whether they
    /// stepped away or are still reading.
    #[serde(default = "default_ask_timeout")]
    pub ask_timeout_seconds: u64,
}

/// Five minutes: long enough to answer a question, short enough that a forgotten one does not
/// block a turn for the rest of the day.
fn default_ask_timeout() -> u64 {
    300
}

fn default_base_url() -> String {
    "https://api.deepseek.com".to_string()
}

fn default_model() -> String {
    "deepseek-flash".to_string()
}

fn default_max_turns() -> u32 {
    24
}

fn default_permission() -> String {
    "ask".to_string()
}

fn default_command_policy() -> String {
    "all".to_string()
}

impl EngineConfig {
    pub fn chat_completions_url(&self) -> String {
        format!("{}/chat/completions", self.base_url.trim_end_matches('/'))
    }
}

/// A conversation message in OpenAI chat-completions shape.
#[derive(Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Data URLs. Only the user sends these, and only when something was attached.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<OutgoingToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: Some(content.into()),
            image: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: Some(content.into()),
            image: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }


    pub fn tool(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: "tool".to_string(),
            content: Some(content.into()),
            image: None,
            tool_calls: None,
            tool_call_id: Some(tool_call_id.into()),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct OutgoingToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: OutgoingFunction,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct OutgoingFunction {
    pub name: String,
    pub arguments: String,
}

/// Event streamed from the engine to the frontend.
///
/// Maps onto the Mavis runtime contract (`RuntimeEventType`):
/// - `kind = "stream"`        -> `STREAM_RESP`
/// - `kind = "action_required"` -> `ACTION_REQUIRED`
/// - `kind = "status"`        -> `SESSION_STATUS`
/// - `kind = "turn_end"`      -> `TURN_TERMINAL`
/// - `kind = "tool_start"` / `"tool_result"` / `"error"` are presentation-level.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineEvent {
    pub session_id: String,
    pub seq: u64,
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<ToolEvent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<ActionEvent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<UsageEvent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl EngineEvent {
    pub fn new(session_id: &str, seq: u64, kind: &str) -> Self {
        Self {
            session_id: session_id.to_string(),
            seq,
            kind: kind.to_string(),
            text: None,
            tool: None,
            action: None,
            status: None,
            stop_reason: None,
            usage: None,
            error: None,
        }
    }
}

/// Something the turn is blocked on. Mirrors the Mavis `ACTION_REQUIRED` contract.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionEvent {
    pub id: String,
    /// "question" or "permission".
    pub kind: String,
    pub question: String,
    pub options: Vec<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolEvent {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    /// A data URL for tools that produce a picture (currently only `screenshot`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageEvent {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}
