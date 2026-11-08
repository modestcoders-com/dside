use futures_util::StreamExt;
use serde_json::{json, Value};

use super::types::{ChatMessage, EngineConfig, OutgoingFunction, OutgoingToolCall};

/// Which stream a delta belongs to. A model that thinks sends its
/// thinking on `reasoning_content` before the answer.
#[derive(Clone, Copy)]
pub enum DeltaKind {
    Text,
    Reasoning,
}

pub struct StreamOutcome {
    pub content: String,
    /// Collected thinking. The UI gets it live through `DeltaKind::Reasoning`; this is the
    /// full text for callers (and tests) that need it after the fact.
    pub reasoning: String,
    pub tool_calls: Vec<OutgoingToolCall>,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    /// Set when the model started repeating itself and the stream was cut. The repeated tail
    /// is already removed from `content`/`reasoning`; the flag is how the turn reports it.
    pub repeated: Option<Repetition>,
}

/// A detected loop: which stream repeated, what unit, and how often.
pub struct Repetition {
    /// "text" or "reasoning".
    pub stream: &'static str,
    pub block: String,
    pub repeats: usize,
}

/// How much repetition counts as a loop. Exact repetition of the same block for 320 bytes is
/// not prose and not a table: prose does not repeat a phrase 60 times in a row, and a wide
/// markdown table has cells between its separators. Setting it this low bounds the damage
/// (a few hundred wasted tokens) while staying far away from normal formatting.
const LOOP_MIN_CHARS: usize = 320;
const LOOP_MIN_REPEATS: usize = 24;
const LOOP_MAX_BLOCK: usize = 200;

/// The repeated unit at the end of `text`, if the tail is nothing but copies of one block.
/// Only the tail is examined, so a long correct answer with one repeated word does not trip.
fn repeated_tail(text: &str) -> Option<(String, usize)> {
    let bytes = text.as_bytes();
    if bytes.len() < LOOP_MIN_CHARS {
        return None;
    }
    for block_len in 1..=LOOP_MAX_BLOCK {
        let repeats = (LOOP_MIN_CHARS / block_len).max(LOOP_MIN_REPEATS);
        let span = block_len * repeats;
        if bytes.len() < span {
            continue;
        }
        let tail = &bytes[bytes.len() - span..];
        let block = &tail[..block_len];
        if block.iter().all(|byte| byte.is_ascii_whitespace()) {
            continue;
        }
        if (0..repeats).all(|index| &tail[index * block_len..(index + 1) * block_len] == block) {
            return Some((String::from_utf8_lossy(block).to_string(), repeats));
        }
    }
    None
}

/// Removes the detected repetition from the end. Cutting by bytes is safe here because the
/// removed span is a whole number of repeated blocks; the lossy conversion only guards the
/// case where a block boundary split a multi-byte character.
fn cut_repetition(text: &mut String, block: &str, repeats: usize) {
    let span = block.len() * repeats;
    if span == 0 || span > text.len() || block.is_empty() {
        return;
    }
    let cut = text.len() - span;
    *text = String::from_utf8_lossy(&text.as_bytes()[..cut]).to_string();
    // Copies that began before the detected span are stripped too, so the answer ends where
    // the loop started instead of keeping a dozen "The." that add nothing.
    while text.ends_with(block) {
        let shorter = text.len() - block.len();
        text.truncate(shorter);
    }
    while text.ends_with([' ', '\n']) {
        text.pop();
    }
}

fn truncate(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_string();
    }
    text.chars().take(limit).collect::<String>() + "…"
}

/// OpenAI-compatible providers take pictures as content parts, not as a top-level `image`
/// field: `content: [{type:"text",…}, {type:"image_url",image_url:{url:"data:…"}}]`.
/// `ChatMessage` keeps the images in their own field for the rest of the engine, so this is
/// the one place that translates them to the wire shape. Getting this wrong makes the
/// provider accept the request and ignore the picture — the model then answers as if it were
/// blind.
fn wire_messages(messages: &[ChatMessage]) -> Vec<Value> {
    fn wire_message(message: &ChatMessage) -> Value {
        let mut wire = json!({ "role": message.role.as_str() });

        let images = message
            .image
            .as_ref()
            .filter(|images| !images.is_empty());
        if let Some(images) = images {
            let mut content = Vec::with_capacity(images.len() + 1);
            if let Some(text) = message.content.as_deref() {
                if !text.is_empty() {
                    content.push(json!({ "type": "text", "text": text }));
                }
            }
            for url in images {
                content.push(json!({
                    "type": "image_url",
                    "image_url": { "url": url }
                }));
            }
            wire["content"] = Value::Array(content);
        } else if let Some(text) = message.content.as_deref() {
            wire["content"] = Value::String(text.to_string());
        }

        if let Some(calls) = &message.tool_calls {
            wire["tool_calls"] = json!(calls);
        }
        if let Some(id) = &message.tool_call_id {
            wire["tool_call_id"] = json!(id);
        }
        wire
    }

    let mut wire = Vec::with_capacity(messages.len());
    let mut index = 0;
    while index < messages.len() {
        let message = &messages[index];
        wire.push(wire_message(message));

        if let Some(calls) = &message.tool_calls {
            // The provider rejects a request when an assistant `tool_calls` message is not
            // followed by one tool message per call. A turn can be stopped (or the app can
            // restart) while a call waits for approval; the assistant row is already in the
            // database then. Pad the missing answers here instead of leaving every later turn
            // of that session poisoned with HTTP 400.
            let mut next = index + 1;
            let mut answered = std::collections::HashSet::new();
            while next < messages.len() && messages[next].role == "tool" {
                if let Some(id) = messages[next].tool_call_id.as_deref() {
                    answered.insert(id.to_string());
                }
                wire.push(wire_message(&messages[next]));
                next += 1;
            }
            for call in calls {
                if !answered.contains(&call.id) {
                    wire.push(json!({
                        "role": "tool",
                        "tool_call_id": call.id,
                        "content": "Cancelled before the tool ran.",
                    }));
                }
            }
            index = next;
            continue;
        }
        index += 1;
    }
    wire
}

/// Streams one assistant turn from an OpenAI-compatible chat-completions endpoint
/// (DeepSeek, `POST /chat/completions`). Text deltas are forwarded to `on_text`.
pub async fn stream_chat(
    config: &EngineConfig,
    messages: &[ChatMessage],
    tools: &[Value],
    on_delta: &mut (dyn FnMut(DeltaKind, &str) + Send),
) -> Result<StreamOutcome, String> {
    let client = reqwest::Client::builder()
        .build()
        .map_err(|error| error.to_string())?;

    let mut body = json!({
        "model": config.model,
        "stream": true,
        "stream_options": { "include_usage": true },
        "messages": wire_messages(messages),
    });
    if !tools.is_empty() {
        body["tools"] = Value::Array(tools.to_vec());
    }
    // How hard the model thinks before answering. The provider rejects an empty string, so an
    // unset value must stay out of the body entirely and leave its own default in place.
    if let Some(effort) = config.reasoning_effort.as_deref() {
        if !effort.trim().is_empty() {
            body["reasoning_effort"] = Value::String(effort.trim().to_string());
        }
    }

    let response = client
        .post(config.chat_completions_url())
        .bearer_auth(&config.api_key)
        .json(&body)
        .send()
        .await
        .map_err(|error| format!("LLM request failed: {error}"))?;

    let status = response.status();
    if !status.is_success() {
        let text = response.text().await.unwrap_or_default();
        return Err(format!("LLM HTTP {status}: {}", truncate(&text, 700)));
    }

    let mut stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut content = String::new();
    let mut reasoning = String::new();
    let mut partials: Vec<(String, String, String)> = Vec::new();
    let mut usage = (0u32, 0u32, 0u32);
    // A model that starts repeating itself does not stop on its own; the stream is cut as
    // soon as a tail is nothing but copies of one block. My own turn once produced "The. The."
    // for a hundred thousand characters because nothing looked at the output.
    let mut repeated: Option<Repetition> = None;

    'stream: while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|error| error.to_string())?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        loop {
            let Some(index) = buffer.find('\n') else { break };
            let line = buffer[..index].trim_end_matches('\r').to_string();
            buffer.drain(..=index);

            let Some(data) = line.strip_prefix("data:") else {
                continue;
            };
            let data = data.trim();
            if data.is_empty() || data == "[DONE]" {
                continue;
            }
            let Ok(parsed) = serde_json::from_str::<Value>(data) else {
                continue;
            };

            if let Some(usage_value) = parsed.get("usage") {
                if !usage_value.is_null() {
                    usage.0 = usage_value
                        .get("prompt_tokens")
                        .and_then(Value::as_u64)
                        .unwrap_or(0) as u32;
                    usage.1 = usage_value
                        .get("completion_tokens")
                        .and_then(Value::as_u64)
                        .unwrap_or(0) as u32;
                    usage.2 = usage_value
                        .get("total_tokens")
                        .and_then(Value::as_u64)
                        .unwrap_or(0) as u32;
                }
            }

            let Some(delta) = parsed
                .get("choices")
                .and_then(|choices| choices.get(0))
                .and_then(|choice| choice.get("delta"))
            else {
                continue;
            };

            if let Some(text) = delta.get("content").and_then(Value::as_str) {
                if !text.is_empty() {
                    content.push_str(text);
                    if let Some((block, repeats)) = repeated_tail(&content) {
                        cut_repetition(&mut content, &block, repeats);
                        repeated = Some(Repetition {
                            stream: "text",
                            block,
                            repeats,
                        });
                        break 'stream;
                    }
                    on_delta(DeltaKind::Text, text);
                }
            }

            if let Some(thinking) = delta.get("reasoning_content").and_then(Value::as_str) {
                if !thinking.is_empty() {
                    reasoning.push_str(thinking);
                    if let Some((block, repeats)) = repeated_tail(&reasoning) {
                        cut_repetition(&mut reasoning, &block, repeats);
                        repeated = Some(Repetition {
                            stream: "reasoning",
                            block,
                            repeats,
                        });
                        break 'stream;
                    }
                    on_delta(DeltaKind::Reasoning, thinking);
                }
            }

            if let Some(calls) = delta.get("tool_calls").and_then(Value::as_array) {
                for call in calls {
                    let index = call.get("index").and_then(Value::as_u64).unwrap_or(0) as usize;
                    while partials.len() <= index {
                        partials.push((String::new(), String::new(), String::new()));
                    }
                    let slot = &mut partials[index];
                    if let Some(id) = call.get("id").and_then(Value::as_str) {
                        if !id.is_empty() {
                            slot.0 = id.to_string();
                        }
                    }
                    if let Some(function) = call.get("function") {
                        if let Some(name) = function.get("name").and_then(Value::as_str) {
                            if !name.is_empty() {
                                slot.1.push_str(name);
                            }
                        }
                        if let Some(arguments) = function.get("arguments").and_then(Value::as_str) {
                            slot.2.push_str(arguments);
                        }
                    }
                }
            }
        }
    }

    let tool_calls = partials
        .into_iter()
        .filter(|(_, name, _)| !name.is_empty())
        .enumerate()
        .map(|(index, (id, name, arguments))| OutgoingToolCall {
            id: if id.is_empty() {
                format!("call_{index}")
            } else {
                id
            },
            call_type: "function".to_string(),
            function: OutgoingFunction { name, arguments },
        })
        .collect();

    Ok(StreamOutcome {
        content,
        reasoning,
        tool_calls,
        prompt_tokens: usage.0,
        completion_tokens: usage.1,
        total_tokens: usage.2,
        repeated,
    })
}

#[cfg(test)]
mod live {
    use crate::engine::types::EngineConfig;
    use super::*;

    fn live_config() -> Option<EngineConfig> {
        let api_key = std::env::var("DS_IDE_API_KEY").ok()?;
        if api_key.trim().is_empty() {
            return None;
        }
        Some(EngineConfig {
            api_key,
            base_url: std::env::var("DS_IDE_BASE_URL")
                .unwrap_or_else(|_| "https://api.deepseek.com".to_string()),
            model: std::env::var("DS_IDE_MODEL").unwrap_or_else(|_| "deepseek-flash".to_string()),
            system_prompt: "You are a terse test assistant. Follow instructions exactly."
                .to_string(),
            workspace: "/tmp".to_string(),
            permission: "full".to_string(),
            command_policy: "all".to_string(),
            allowed_commands: Vec::new(),
            workspace_only: false,
            browser_control: false,
            max_turns: 2,
            reasoning_effort: None,
            context_window: None,
            disabled_tools: Vec::new(),
            ask_timeout_seconds: 300,
        })
    }

    #[tokio::test]
    #[ignore = "requires DS_IDE_API_KEY"]
    async fn live_deepseek_streams_text() {
        let Some(config) = live_config() else {
            crate::log_line!("skipped: DS_IDE_API_KEY not set");
            return;
        };
        let messages = vec![ChatMessage::user("Antworte mit exakt einem Wort: PONG")];
        let mut streamed = String::new();
        let result = {
            let mut on_delta = |kind: DeltaKind, chunk: &str| {
                if matches!(kind, DeltaKind::Text) {
                    streamed.push_str(chunk);
                }
            };
            stream_chat(&config, &messages, &[], &mut on_delta).await
        };

        match result {
            Ok(outcome) => {
                println!("content: {:?}", outcome.content);
                println!("streamed chars: {}", streamed.len());
                println!(
                    "usage prompt/completion/total: {}/{}/{}",
                    outcome.prompt_tokens, outcome.completion_tokens, outcome.total_tokens
                );
                assert!(!outcome.content.trim().is_empty(), "empty completion");
            }
            Err(error) => panic!("live text call failed: {error}"),
        }
    }

    #[tokio::test]
    #[ignore = "requires DS_IDE_API_KEY"]
    async fn live_deepseek_emits_tool_calls() {
        let Some(config) = live_config() else {
            crate::log_line!("skipped: DS_IDE_API_KEY not set");
            return;
        };
        let messages = vec![ChatMessage::user(
            "Rufe jetzt das Tool glob mit pattern \"Cargo.toml\" auf. Keine Erklärung, nur der Tool-Aufruf.",
        )];
        let tools = crate::engine::tools::definitions();
        let mut on_delta = |_kind: DeltaKind, _chunk: &str| {};
        let result = stream_chat(&config, &messages, &tools, &mut on_delta).await;

        match result {
            Ok(outcome) => {
                for call in &outcome.tool_calls {
                    println!("tool call: {} args={}", call.function.name, call.function.arguments);
                }
                println!("text: {:?}", outcome.content);
                assert!(!outcome.tool_calls.is_empty(), "expected at least one tool call");
            }
            Err(error) => panic!("live tool call failed: {error}"),
        }
    }

    /// The compression call: a strict instruction plus a transcript, no tools. If the provider
    /// rejects this shape, compressing would fail silently in the background.
    #[tokio::test]
    #[ignore = "requires DS_IDE_API_KEY"]
    async fn live_deepseek_summarises_a_conversation() {
        let Some(config) = live_config() else {
            crate::log_line!("skipped: DS_IDE_API_KEY not set");
            return;
        };

        let messages = vec![
            ChatMessage::system(
                "You compress the earlier part of a conversation so it can be replaced by your \
                 summary. Keep what still matters: decisions, facts about the user, the state of \
                 the work, open tasks. Write notes, not prose. Never answer the conversation — \
                 only summarise it.",
            ),
            ChatMessage::user(
                "user: Wir bauen DS.IDE, eine lokale agentische IDE auf Tauri v2 mit Rust-Kern.\n\
                 assistant: Verstanden. Die Oberfläche ist Vue, die Daten liegen in SQLite.\n\
                 user: Bitte merke dir, dass ich kurze Antworten mag.\n\
                 assistant: Notiert.",
            ),
        ];

        let mut sink = |_kind: DeltaKind, _chunk: &str| {};
        let outcome = stream_chat(&config, &messages, &[], &mut sink)
            .await
            .expect("the compression request must be accepted");
        let summary = outcome.content.trim();
        println!("summary: {summary}");
        assert!(!summary.is_empty(), "compression must produce a summary");
    }

    /// Images go in an OpenAI `image_url` content part as a data URL. A top-level `image`
    /// field is accepted by the API but silently ignored by the model — the exact bug that
    /// made the eyes look blind.
    #[tokio::test]
    #[ignore = "requires DS_IDE_API_KEY"]
    async fn live_deepseek_takes_an_image() {
        let Some(config) = live_config() else {
            crate::log_line!("skipped: DS_IDE_API_KEY not set");
            return;
        };

        // A real 520x180 PNG with "HALLO 4711"; the model must read it back.
        let png = include_bytes!("../../tests/fixtures/vision.png");

        let mut message = ChatMessage::user("Antworte nur mit dem Text auf dem Bild.");
        message.image = Some(vec![format!(
            "data:image/png;base64,{}",
            crate::base64_encode(png)
        )]);
        let messages = vec![message];

        let mut sink = |_kind: DeltaKind, _chunk: &str| {};
        let outcome = stream_chat(&config, &messages, &[], &mut sink)
            .await
            .expect("sending an image must be accepted");
        println!("answer: {:?}", outcome.content);
        assert!(
            outcome.content.contains("HALLO 4711"),
            "the model did not read the image: {:?}",
            outcome.content
        );
    }

    #[test]
    fn images_travel_as_openai_content_parts() {
        let mut message = ChatMessage::user("Schau");
        message.image = Some(vec![
            "data:image/png;base64,AAAA".to_string(),
            "data:image/png;base64,BBBB".to_string(),
        ]);
        let wire = wire_messages(&[message]);
        assert!(wire[0].get("image").is_none());
        assert_eq!(wire[0]["content"][0]["type"], "text");
        assert_eq!(wire[0]["content"][1]["type"], "image_url");
        assert_eq!(
            wire[0]["content"][1]["image_url"]["url"],
            "data:image/png;base64,AAAA"
        );
        assert_eq!(
            wire[0]["content"][2]["image_url"]["url"],
            "data:image/png;base64,BBBB"
        );

        let plain = wire_messages(&[ChatMessage::user("hallo")]);
        assert_eq!(plain[0]["content"], "hallo");

        let tool = wire_messages(&[ChatMessage::tool("call-1", "done")]);
        assert_eq!(tool[0]["tool_call_id"], "call-1");
        assert_eq!(tool[0]["content"], "done");
    }

    /// Stopping a turn while a tool call waits for approval leaves the assistant row with a
    /// tool call and no result. Every later request would fail with HTTP 400; the wire
    /// sequence gets the missing answer padded instead.
    #[test]
    fn a_cancelled_tool_call_is_padded_for_the_provider() {
        let call = |id: &str, name: &str| OutgoingToolCall {
            id: id.to_string(),
            call_type: "function".to_string(),
            function: OutgoingFunction {
                name: name.to_string(),
                arguments: "{}".to_string(),
            },
        };
        let assistant = ChatMessage {
            role: "assistant".to_string(),
            content: Some(String::new()),
            image: None,
            tool_calls: Some(vec![call("call-a", "read"), call("call-b", "bash")]),
            tool_call_id: None,
        };

        let wire = wire_messages(&[
            ChatMessage::user("los"),
            assistant,
            ChatMessage::tool("call-a", "ok"),
            ChatMessage::user("weiter"),
        ]);

        assert_eq!(wire.len(), 5);
        assert_eq!(wire[1]["role"], "assistant");
        assert_eq!(wire[2]["tool_call_id"], "call-a");
        assert_eq!(wire[3]["tool_call_id"], "call-b");
        assert_eq!(wire[3]["content"], "Cancelled before the tool ran.");
        assert_eq!(wire[4]["role"], "user");
    }

    /// `reasoning_effort` is a real field with a fixed set of values; a typo in either the
    /// name or the value is a 422, and that is worth catching without a human in the loop.
    #[tokio::test]
    #[ignore = "requires DS_IDE_API_KEY"]
    async fn live_deepseek_accepts_reasoning_effort() {
        let Some(mut config) = live_config() else {
            crate::log_line!("skipped: DS_IDE_API_KEY not set");
            return;
        };
        config.model = "deepseek-v4-pro".to_string();
        config.reasoning_effort = Some("low".to_string());

        let messages = vec![ChatMessage::user("Antworte mit genau: OK")];
        let mut sink = |_kind: DeltaKind, _chunk: &str| {};
        let outcome = stream_chat(&config, &messages, &[], &mut sink)
            .await
            .expect("request with reasoning_effort must be accepted");
        assert_eq!(outcome.content.trim(), "OK");
    }

    #[tokio::test]
    #[ignore = "requires DS_IDE_API_KEY"]
    async fn live_deepseek_reasoner_streams_thinking() {
        let Some(mut config) = live_config() else {
            crate::log_line!("skipped: DS_IDE_API_KEY not set");
            return;
        };
        config.model = "deepseek-v4-pro".to_string();

        let messages = vec![ChatMessage::user("Was ist 17 * 23? Denke kurz nach.")];
        let mut text = String::new();
        let result = {
            let mut on_delta = |kind: DeltaKind, chunk: &str| {
                if matches!(kind, DeltaKind::Text) {
                    text.push_str(chunk);
                }
            };
            stream_chat(&config, &messages, &[], &mut on_delta).await
        };

        match result {
            Ok(outcome) => {
                println!("reasoning chars: {}", outcome.reasoning.len());
                println!("answer: {:?}", outcome.content);
                assert!(
                    !outcome.reasoning.is_empty(),
                    "expected reasoning_content from the provider's thinking model"
                );
                assert!(!outcome.content.is_empty(), "expected a final answer");
            }
            Err(error) => panic!("live reasoner call failed: {error}"),
        }
    }
}

#[cfg(test)]
mod guard {
    use super::*;

    #[test]
    fn a_repeated_phrase_is_a_loop() {
        let mut text = String::from("Here is the answer.\n");
        text.push_str(&"The. ".repeat(80));
        let (block, repeats) = repeated_tail(&text).expect("the loop must be detected");
        assert_eq!(block, "The. ");
        assert!(repeats >= 60);
    }

    #[test]
    fn a_repeated_sentence_is_a_loop() {
        let unit = "I will check the file first. ";
        let mut text = String::new();
        for _ in 0..30 {
            text.push_str(unit);
        }
        let (block, repeats) = repeated_tail(&text).expect("repeated sentences must be caught");
        assert_eq!(block, unit);
        assert!(repeats * block.len() >= LOOP_MIN_CHARS);
    }

    #[test]
    fn normal_prose_and_tables_stay_untouched() {
        let prose = "The function reads the file, checks the header and returns the rows. \
                     It reports an error when the header is missing, so the caller can decide \
                     what to do. Each row is validated before it is written to the database, \
                     and the transaction is rolled back when a row fails.";
        assert!(repeated_tail(prose).is_none());

        // A wide markdown table repeats its separator pattern but has cells in between.
        let mut table = String::new();
        for column in 0..40 {
            table.push_str(&format!("| column {column} "));
        }
        table.push_str("|\n");
        table.push_str(&"|---|".repeat(60));
        assert!(repeated_tail(&table).is_none(), "a separator row is not a loop");

        // An indented code block with blank lines is not a loop either.
        let code = "fn main() {\n\n    let x = 1;\n\n    println!(\"{x}\");\n\n}\n";
        assert!(repeated_tail(&code.repeat(4)).is_none());
    }

    #[test]
    fn cutting_removes_the_repetition_and_keeps_the_answer() {
        let mut text = String::from("Answer before the loop. ");
        let block = "The. ";
        text.push_str(&block.repeat(80));
        let (found, repeats) = repeated_tail(&text).unwrap();
        cut_repetition(&mut text, &found, repeats);
        assert_eq!(text, "Answer before the loop.");
    }

    /// The guard runs on the tail only; a repeated word in the middle of a long answer must
    /// not cut it off.
    #[test]
    fn repetition_earlier_in_the_answer_is_not_the_tail() {
        let mut text = String::new();
        text.push_str(&"The. ".repeat(100));
        text.push_str("And here is the actual answer, written in ordinary prose that continues \
                       for a while so the tail is clearly not a repetition of one block.");
        assert!(repeated_tail(&text).is_none());
    }
}
