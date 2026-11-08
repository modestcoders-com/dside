//! Dev-only UI test bridge.
//!
//! The native path into the webview is closed on Linux: XTest does not deliver keys to the
//! WebKit window under Wayland, and the X11 frame of the window is a ghost. This bridge gives
//! a debug build a localhost channel into the main webview, so a test can click the app's own
//! handlers and read what the DOM shows.
//!
//! It is not part of a release: `serve` is debug-only and starts only with
//! `DS_IDE_TEST_BRIDGE=1`. What remains in a release is the `test_bridge_report` command,
//! which stores a string and cannot be reached from outside the webview.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

#[cfg(debug_assertions)]
use tauri::{AppHandle, Manager};
#[cfg(debug_assertions)]
use tokio::io::{AsyncReadExt, AsyncWriteExt};
#[cfg(debug_assertions)]
use tokio::net::TcpStream;

/// The last report and a generation counter. The counter lets a client tell an old report
/// from the answer to the eval it just sent.
#[derive(Default)]
pub struct TestBridgeState {
    // Only the debug bridge reads the report; a release build still stores one.
    #[cfg_attr(not(debug_assertions), allow(dead_code))]
    last: Mutex<Option<(u64, String)>>,
    next: AtomicU64,
}

impl TestBridgeState {
    pub fn record(&self, payload: String) {
        let seq = self.next.fetch_add(1, Ordering::SeqCst) + 1;
        if let Ok(mut slot) = self.last.lock() {
            *slot = Some((seq, payload));
        }
    }

    #[cfg(debug_assertions)]
    pub fn snapshot(&self) -> Option<(u64, String)> {
        self.last.lock().ok().and_then(|slot| slot.clone())
    }
}

#[tauri::command]
pub fn test_bridge_report(state: tauri::State<'_, TestBridgeState>, payload: String) {
    state.record(payload);
}

/// Listen on localhost and evaluate scripts in the main window until the app exits.
#[cfg(debug_assertions)]
pub async fn serve(app: AppHandle, port: u16) {
    let listener = match tokio::net::TcpListener::bind(("127.0.0.1", port)).await {
        Ok(listener) => listener,
        Err(error) => {
            crate::log_line!("test bridge: cannot bind 127.0.0.1:{port}: {error}");
            return;
        }
    };
    crate::log_line!("test bridge: listening on 127.0.0.1:{port}");
    loop {
        let (stream, _) = match listener.accept().await {
            Ok(pair) => pair,
            Err(error) => {
                crate::log_line!("test bridge: accept failed: {error}");
                continue;
            }
        };
        let app = app.clone();
        tokio::spawn(async move { handle(app, stream).await });
    }
}

#[cfg(debug_assertions)]
struct Request {
    method: String,
    path: String,
    body: String,
}

#[cfg(debug_assertions)]
const MAX_REQUEST: usize = 512 * 1024;

#[cfg(debug_assertions)]
async fn handle(app: AppHandle, mut stream: TcpStream) {
    let request = match read_request(&mut stream).await {
        Ok(request) => request,
        Err(error) => {
            let _ = respond(&mut stream, 400, "text/plain; charset=utf-8", &error).await;
            return;
        }
    };
    let (status, content_type, payload) = match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/health") => (200, "text/plain; charset=utf-8", "ok".to_string()),
        ("GET", "/report") => {
            let state = app.state::<TestBridgeState>();
            let value = match state.snapshot() {
                Some((seq, payload)) => {
                    let payload = serde_json::from_str::<serde_json::Value>(&payload)
                        .unwrap_or(serde_json::Value::String(payload));
                    serde_json::json!({ "seq": seq, "payload": payload })
                }
                None => serde_json::json!({ "seq": 0, "payload": null }),
            };
            (200, "application/json", value.to_string())
        }
        // Closing the way the titlebar button does: Tauri's own close(), so the registered
        // CloseRequested handler runs. xdotool destroys the X11 window instead, which bypasses
        // every handler and kills the process — fine for a real crash, useless as a test.
        // The tray's own quit path: `app.exit` tears the indicator down, which a signal does
        // not. Needed to check that a normal quit leaves nothing behind in the panel.
        ("POST", "/quit") => {
            app.exit(0);
            (200, "application/json", "{\"ok\":true}".to_string())
        }
        ("POST", "/close") => match app.get_webview_window("main") {
            Some(window) => match window.close() {
                Ok(()) => (200, "application/json", "{\"ok\":true}".to_string()),
                Err(error) => (
                    500,
                    "text/plain; charset=utf-8",
                    format!("close failed: {error}"),
                ),
            },
            None => (503, "text/plain; charset=utf-8", "no main window".to_string()),
        },
        ("POST", "/eval") => match app.get_webview_window("main") {
            Some(window) => match window.eval(wrap(&request.body)) {
                Ok(()) => (200, "application/json", "{\"ok\":true}".to_string()),
                Err(error) => (
                    500,
                    "text/plain; charset=utf-8",
                    format!("eval failed: {error}"),
                ),
            },
            None => (503, "text/plain; charset=utf-8", "no main window".to_string()),
        },
        _ => (404, "text/plain; charset=utf-8", "not found".to_string()),
    };
    let _ = respond(&mut stream, status, content_type, &payload).await;
}

#[cfg(debug_assertions)]
async fn read_request(stream: &mut TcpStream) -> Result<Request, String> {
    let mut data = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        let read = stream
            .read(&mut buffer)
            .await
            .map_err(|error| error.to_string())?;
        if read == 0 {
            return Err("connection closed before the request was complete".into());
        }
        data.extend_from_slice(&buffer[..read]);
        if let Some(end) = find_header_end(&data) {
            let head = String::from_utf8_lossy(&data[..end]).to_string();
            let body_start = end + 4;
            let length = head
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    if name.eq_ignore_ascii_case("content-length") {
                        value.trim().parse::<usize>().ok()
                    } else {
                        None
                    }
                })
                .unwrap_or(0);
            if data.len() >= body_start + length {
                let body =
                    String::from_utf8_lossy(&data[body_start..body_start + length]).to_string();
                let mut parts = head.lines().next().unwrap_or_default().split_whitespace();
                let method = parts.next().unwrap_or_default().to_string();
                let path = parts.next().unwrap_or_default().to_string();
                return Ok(Request { method, path, body });
            }
        }
        if data.len() > MAX_REQUEST {
            return Err("request too large".into());
        }
    }
}

#[cfg(debug_assertions)]
fn find_header_end(data: &[u8]) -> Option<usize> {
    data.windows(4).position(|window| window == b"\r\n\r\n")
}

#[cfg(debug_assertions)]
async fn respond(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &str,
) -> std::io::Result<()> {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        500 => "Internal Server Error",
        503 => "Service Unavailable",
        _ => "OK",
    };
    let head = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes()).await?;
    stream.write_all(body.as_bytes()).await?;
    stream.flush().await
}

/// Wraps the submitted script so a test can report a JSON value without knowing the IPC
/// details. The wrapper is async, so a script may await the app's own promises.
#[cfg(debug_assertions)]
fn wrap(code: &str) -> String {
    format!(
        r#"(async () => {{
  const report = (data) => {{
    try {{
      window.__TAURI_INTERNALS__.invoke("test_bridge_report", {{ payload: JSON.stringify(data) }});
    }} catch (_) {{}}
  }};
  try {{
    {code}
  }} catch (error) {{
    void report({{ error: String(error) }});
  }}
}})();"#
    )
}

#[cfg(all(test, debug_assertions))]
mod tests {
    use super::*;

    #[test]
    fn the_wrapper_exposes_report_and_keeps_the_script() {
        let script = wrap("report({ ok: true });");
        assert!(script.contains("test_bridge_report"));
        assert!(script.contains("report({ ok: true });"));
        assert!(script.starts_with("(async () =>"));
    }

    #[test]
    fn a_request_head_is_terminated_by_a_blank_line() {
        let data = b"POST /eval HTTP/1.1\r\nContent-Length: 0\r\n\r\n";
        let end = find_header_end(data).expect("header end");
        assert_eq!(&data[end..end + 4], b"\r\n\r\n");
    }
}
