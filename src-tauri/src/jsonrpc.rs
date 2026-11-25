//! A JSON-RPC 2.0 client over a child process's stdin and stdout.
//!
//! One transport for plugins and, in a later round, MCP servers. Both speak the same three
//! methods — `initialize`, `tools/list`, `tools/call` — because writing two clients for one
//! protocol would be two places to get the framing wrong.
//!
//! Framing is one JSON object per line. That is what the MCP stdio transport specifies and what
//! a shell pipeline can fake, which is why the test below can run against `sh` instead of a
//! bundled server.
//!
//! Failure is a value, not a panic: every error names the method and what went wrong, so a
//! broken plugin produces a sentence in the conversation instead of an empty turn.

use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

/// How a server is started. Deliberately plain data: the caller decides program, arguments,
/// environment and working directory, this module only runs them.
pub struct Launch {
    pub program: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub cwd: Option<PathBuf>,
    /// Used in log lines, so a message says which plugin misbehaved.
    pub label: String,
}

pub struct StdioServer {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
    label: String,
}

impl StdioServer {
    /// Starts the program with sanitised environment and pipes for the protocol.
    pub fn start(launch: Launch) -> Result<Self, String> {
        let label = launch.label.clone();
        let mut command = Command::new(&launch.program);
        command
            .args(&launch.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            // A server that writes its complaints to stderr must not fill a pipe we never
            // read; that would block it at 64 KB and look like a hang.
            .stderr(Stdio::null())
            .kill_on_drop(true);
        if let Some(cwd) = &launch.cwd {
            command.current_dir(cwd);
        }
        for (key, value) in &launch.env {
            command.env(key, value);
        }
        // The bundle's LD_LIBRARY_PATH and PYTHONHOME point into the AppImage mount; a server
        // that runs a python script would read them and fail in a way that looks like our bug.
        crate::child_env::sanitize_async(&mut command);

        let mut child = command
            .spawn()
            .map_err(|error| format!("could not start ({}): {error}", launch.program))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "no stdin for the server".to_string())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "no stdout for the server".to_string())?;
        Ok(Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
            next_id: 1,
            label,
        })
    }

    /// The full handshake both plugins and MCP servers use. Returns the tools the server offers,
    /// unfiltered — what the manifest allows is a separate question.
    pub async fn handshake(
        &mut self,
        client_name: &str,
        client_version: &str,
    ) -> Result<Vec<Value>, String> {
        let params = json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": client_name, "version": client_version }
        });
        self.request("initialize", params, Duration::from_secs(20))
            .await?;
        self.notify("notifications/initialized", json!({})).await?;
        let result = self
            .request("tools/list", json!({}), Duration::from_secs(20))
            .await?;
        let tools = result
            .get("tools")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        Ok(tools)
    }

    pub async fn call_tool(
        &mut self,
        name: &str,
        arguments: &Value,
        timeout: Duration,
    ) -> Result<Value, String> {
        let params = json!({ "name": name, "arguments": arguments });
        self.request("tools/call", params, timeout).await
    }

    pub async fn notify(&mut self, method: &str, params: Value) -> Result<(), String> {
        let line = json!({ "jsonrpc": "2.0", "method": method, "params": params }).to_string();
        self.write_line(&line).await
    }

    pub async fn request(
        &mut self,
        method: &str,
        params: Value,
        timeout: Duration,
    ) -> Result<Value, String> {
        let id = self.next_id;
        self.next_id += 1;
        let line = json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params })
            .to_string();
        self.write_line(&line).await?;

        let deadline = tokio::time::Instant::now() + timeout;
        let mut buffer = String::new();
        loop {
            buffer.clear();
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return Err(format!("{method}: no answer within {}s", timeout.as_secs()));
            }
            let read = tokio::time::timeout(remaining, self.stdout.read_line(&mut buffer))
                .await
                .map_err(|_| format!("{method}: no answer within {}s", timeout.as_secs()))?
                .map_err(|error| format!("{method}: reading failed: {error}"))?;
            if read == 0 {
                return Err(format!(
                    "{method}: the server closed the connection (exit code {})",
                    self.exit_code()
                ));
            }
            let trimmed = buffer.trim();
            if trimmed.is_empty() {
                continue;
            }
            let Ok(value) = serde_json::from_str::<Value>(trimmed) else {
                // Servers that print a banner on stdout are common enough to tolerate; a line
                // that is not JSON is not an answer to our request.
                crate::log_line!("jsonrpc: {}: ignoring a non-JSON line", self.label);
                continue;
            };
            if !answers_id(&value, id) {
                continue;
            }
            if let Some(error) = value.get("error") {
                return Err(format!("{method}: {}", describe_error(error)));
            }
            return Ok(value.get("result").cloned().unwrap_or(Value::Null));
        }
    }

    async fn write_line(&mut self, line: &str) -> Result<(), String> {
        self.stdin
            .write_all(line.as_bytes())
            .await
            .map_err(|error| format!("writing to the server failed: {error}"))?;
        self.stdin
            .write_all(b"\n")
            .await
            .map_err(|error| format!("writing to the server failed: {error}"))?;
        self.stdin
            .flush()
            .await
            .map_err(|error| format!("writing to the server failed: {error}"))
    }

    fn exit_code(&mut self) -> String {
        match self.child.try_wait() {
            Ok(Some(status)) => status
                .code()
                .map(|code| code.to_string())
                .unwrap_or_else(|| "terminated".to_string()),
            _ => "still running".to_string(),
        }
    }

    pub fn kill(&mut self) {
        let _ = self.child.start_kill();
    }
}

/// A reply carries our id; a notification carries none and a late reply carries another. Both
/// are skipped rather than treated as an answer.
fn answers_id(value: &Value, id: u64) -> bool {
    match value.get("id") {
        Some(Value::Number(number)) => number.as_u64() == Some(id),
        Some(Value::String(text)) => text.parse::<u64>().ok() == Some(id),
        _ => false,
    }
}

/// JSON-RPC errors may carry a code and a message, a bare string, or something odd. All of them
/// end up as one line the model can act on.
fn describe_error(error: &Value) -> String {
    if let Some(message) = error.get("message").and_then(Value::as_str) {
        let code = error.get("code").and_then(Value::as_i64);
        return match code {
            Some(code) => format!("{message} (code {code})"),
            None => message.to_string(),
        };
    }
    if let Some(text) = error.as_str() {
        return text.to_string();
    }
    format!("the server reported an error: {error}")
}

/// The text of a `tools/call` result. MCP returns a list of content blocks; plugins use the same
/// shape, so one reader serves both. Anything else is passed through as JSON rather than lost.
pub fn result_text(result: &Value) -> (String, bool) {
    let is_error = result
        .get("isError")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let content = result.get("content").and_then(Value::as_array);
    let Some(content) = content else {
        return (result.to_string(), is_error);
    };
    let mut parts = Vec::new();
    for block in content {
        if let Some(text) = block.get("text").and_then(Value::as_str) {
            parts.push(text.to_string());
        } else if block.get("type").and_then(Value::as_str) == Some("image") {
            parts.push("[an image the plugin returned; images from plugins are not shown]".to_string());
        } else {
            parts.push(block.to_string());
        }
    }
    if parts.is_empty() {
        return ("(the plugin returned no content)".to_string(), is_error);
    }
    (parts.join("\n"), is_error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reply_is_matched_by_its_id() {
        let number = json!({ "jsonrpc": "2.0", "id": 7, "result": {} });
        let text = json!({ "jsonrpc": "2.0", "id": "7", "result": {} });
        let other = json!({ "jsonrpc": "2.0", "id": 8, "result": {} });
        let notification = json!({ "jsonrpc": "2.0", "method": "log" });
        assert!(answers_id(&number, 7));
        assert!(answers_id(&text, 7));
        assert!(!answers_id(&other, 7));
        assert!(!answers_id(&notification, 7));
    }

    #[test]
    fn errors_become_one_readable_line() {
        assert_eq!(
            describe_error(&json!({ "code": -32601, "message": "Method not found" })),
            "Method not found (code -32601)"
        );
        assert_eq!(describe_error(&json!("nope")), "nope");
        assert!(describe_error(&json!({ "weird": true })).contains("weird"));
    }

    #[test]
    fn tool_results_are_read_block_by_block() {
        let (text, error) = result_text(&json!({
            "content": [
                { "type": "text", "text": "first" },
                { "type": "text", "text": "second" }
            ]
        }));
        assert_eq!(text, "first\nsecond");
        assert!(!error);

        let (text, error) = result_text(&json!({
            "content": [{ "type": "text", "text": "boom" }],
            "isError": true
        }));
        assert_eq!(text, "boom");
        assert!(error);

        let (text, _) = result_text(&json!({ "content": [{ "type": "image" }] }));
        assert!(text.contains("image"));

        let (text, _) = result_text(&json!({ "unexpected": 1 }));
        assert!(text.contains("unexpected"));
    }

    /// A real child that speaks the protocol, so the framing is proven rather than reasoned
    /// about. `sh` reads a line and answers it; there is no shell on Windows, where this test
    /// would have to be a different program.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_real_child_answers_the_handshake() {
        let script = r#"
while IFS= read -r line; do
  case "$line" in
    *'"method":"initialize"'*)
      printf '{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2024-11-05"}}\n' ;;
    *'"method":"tools/list"'*)
      printf '{"jsonrpc":"2.0","id":2,"result":{"tools":[{"name":"ping","description":"p","inputSchema":{"type":"object"}}]}}\n' ;;
    *) : ;;
  esac
done
"#;
        let launch = Launch {
            program: "sh".to_string(),
            args: vec!["-c".to_string(), script.to_string()],
            env: Vec::new(),
            cwd: None,
            label: "test".to_string(),
        };
        let mut server = StdioServer::start(launch).expect("sh starts");
        let tools = server.handshake("test", "0").await.expect("handshake");
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].get("name").and_then(Value::as_str), Some("ping"));
    }

    /// A server that never answers must not hang the turn.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_silent_child_times_out() {
        let launch = Launch {
            program: "sh".to_string(),
            args: vec!["-c".to_string(), "sleep 30".to_string()],
            env: Vec::new(),
            cwd: None,
            label: "test".to_string(),
        };
        let mut server = StdioServer::start(launch).expect("sh starts");
        let error = server
            .request("initialize", json!({}), Duration::from_millis(150))
            .await
            .expect_err("must not answer");
        assert!(error.contains("no answer"), "{error}");
    }
}
