//! A read-only static server for the workspace.
//!
//! The companion can write HTML and CSS, but it cannot open a `file://` page in the browser
//! and it has no way to put a file under a server the browser knows. This server closes that
//! loop: bind to localhost, serve exactly the workspace, and the companion can look at its
//! own page in the app's browser.

use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Manager};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[derive(Default)]
pub struct PreviewState {
    server: Mutex<Option<PreviewServer>>,
}

struct PreviewServer {
    root: PathBuf,
    port: u16,
    stop: Arc<AtomicBool>,
}

/// Starts (or reuses) a preview server for `workspace` and returns its port. Only files
/// below the canonical workspace are served; a changed workspace replaces the server.
pub async fn ensure(app: &AppHandle, workspace: &Path) -> Result<u16, String> {
    let root = workspace
        .canonicalize()
        .map_err(|error| format!("workspace {workspace:?}: {error}"))?;
    if !root.is_dir() {
        return Err(format!("workspace {workspace:?} is not a directory"));
    }
    let state = app.state::<PreviewState>();
    if let Ok(guard) = state.server.lock() {
        if let Some(server) = guard.as_ref() {
            if server.root == root {
                return Ok(server.port);
            }
        }
    }

    let listener = TcpListener::bind(("127.0.0.1", 0))
        .await
        .map_err(|error| format!("bind: {error}"))?;
    let port = listener
        .local_addr()
        .map_err(|error| format!("address: {error}"))?
        .port();
    let stop = Arc::new(AtomicBool::new(false));
    let replaced = {
        let mut guard = state
            .server
            .lock()
            .map_err(|_| "state is poisoned".to_string())?;
        guard.replace(PreviewServer {
            root: root.clone(),
            port,
            stop: stop.clone(),
        })
    };
    if let Some(old) = replaced {
        old.stop.store(true, Ordering::SeqCst);
    }

    let task_root = root.clone();
    let task_stop = stop.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            if task_stop.load(Ordering::SeqCst) {
                break;
            }
            let (stream, _) = match listener.accept().await {
                Ok(pair) => pair,
                Err(error) => {
                    crate::log_line!("preview: accept: {error}");
                    continue;
                }
            };
            let root = task_root.clone();
            tokio::spawn(async move { serve(stream, root).await });
        }
    });
    crate::log_line!("preview: serving {} on 127.0.0.1:{port}", root.display());
    Ok(port)
}

/// The port of the server that currently serves the workspace, if one was started.
pub fn active_port(app: &AppHandle) -> Option<u16> {
    app.try_state::<PreviewState>()
        .and_then(|state| state.server.lock().ok().and_then(|guard| guard.as_ref().map(|server| server.port)))
}

/// True when `url` points at the running preview server. Used to tell the companion's own
/// page (clicking it is part of looking at it) from a page the user opened elsewhere.
pub fn is_preview_url(app: &AppHandle, url: &str) -> bool {
    let Some(port) = active_port(app) else {
        return false;
    };
    matches_preview_url(url, port)
}

/// A pure helper so the rule is testable without an app: plain http, loopback host, exact
/// port, any path.
fn matches_preview_url(url: &str, port: u16) -> bool {
    let trimmed = url.trim();
    let Some(rest) = trimmed.strip_prefix("http://") else {
        return false;
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    match authority.rsplit_once(':') {
        Some((host, value)) => {
            value == port.to_string() && matches!(host, "127.0.0.1" | "localhost" | "[::1]")
        }
        None => false,
    }
}

/// The Tauri command behind the work area/test path; the companion uses the `preview` tool.
#[tauri::command]
pub async fn preview_start(app: AppHandle, root: String) -> Result<String, String> {
    let port = ensure(&app, Path::new(&root)).await?;
    Ok(format!("http://127.0.0.1:{port}/"))
}

async fn serve(mut stream: TcpStream, root: PathBuf) {
    let Some((method, target)) = read_request(&mut stream).await else {
        let _ = respond(&mut stream, 400, "text/plain; charset=utf-8", b"bad request").await;
        return;
    };
    if method != "GET" && method != "HEAD" {
        let _ = respond(
            &mut stream,
            405,
            "text/plain; charset=utf-8",
            b"method not allowed",
        )
        .await;
        return;
    }

    let path = target.split(['?', '#']).next().unwrap_or("/");
    let decoded = percent_decode(path);
    let mut relative = PathBuf::new();
    for component in Path::new(decoded.trim_start_matches('/')).components() {
        match component {
            Component::Normal(part) => relative.push(part),
            // ParentDir, RootDir and a Windows prefix all mean the request left the root.
            _ => {
                let _ = respond(&mut stream, 404, "text/plain; charset=utf-8", b"not found").await;
                return;
            }
        }
    }

    let mut file = root.join(&relative);
    if file.is_dir() {
        file.push("index.html");
    }
    let canonical = match file.canonicalize() {
        Ok(path) if path.starts_with(&root) && path.is_file() => path,
        _ => {
            let _ = respond(&mut stream, 404, "text/plain; charset=utf-8", b"not found").await;
            return;
        }
    };
    let bytes = match std::fs::read(&canonical) {
        Ok(bytes) => bytes,
        Err(error) => {
            crate::log_line!("preview: read {}: {error}", canonical.display());
            let _ = respond(&mut stream, 404, "text/plain; charset=utf-8", b"not found").await;
            return;
        }
    };
    let mime = mime_for(&canonical);
    let head = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        bytes.len()
    );
    let _ = stream.write_all(head.as_bytes()).await;
    if method == "GET" {
        let _ = stream.write_all(&bytes).await;
    }
    let _ = stream.flush().await;
}

async fn read_request(stream: &mut TcpStream) -> Option<(String, String)> {
    let mut data = Vec::new();
    let mut buffer = [0u8; 4096];
    loop {
        let read = stream.read(&mut buffer).await.ok()?;
        if read == 0 {
            break;
        }
        data.extend_from_slice(&buffer[..read]);
        if data.windows(4).any(|window| window == b"\r\n\r\n") || data.len() > 16 * 1024 {
            break;
        }
    }
    let head = String::from_utf8_lossy(&data);
    let first = head.lines().next()?;
    let mut parts = first.split_whitespace();
    let method = parts.next()?.to_string();
    let target = parts.next()?.to_string();
    Some((method, target))
}

async fn respond(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &[u8],
) -> std::io::Result<()> {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        _ => "OK",
    };
    let head = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes()).await?;
    stream.write_all(body).await?;
    stream.flush().await
}

pub(crate) fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let (Some(high), Some(low)) = (hex_value(bytes[index + 1]), hex_value(bytes[index + 2]))
            {
                out.push(high * 16 + low);
                index += 3;
                continue;
            }
        }
        out.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn mime_for(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "txt" | "md" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_decoding_handles_spaces_and_utf8() {
        assert_eq!(percent_decode("a%20b/%C3%A4.html"), "a b/ä.html");
        assert_eq!(percent_decode("plain.css"), "plain.css");
    }

    #[test]
    fn the_mime_keeps_pages_and_styles_typed() {
        assert!(mime_for(Path::new("a.html")).starts_with("text/html"));
        assert!(mime_for(Path::new("a.css")).starts_with("text/css"));
        assert_eq!(mime_for(Path::new("a.bin")), "application/octet-stream");
    }

    #[test]
    fn only_the_running_preview_server_counts_as_local() {
        assert!(matches_preview_url("http://127.0.0.1:35177/index.html", 35177));
        assert!(matches_preview_url("http://localhost:35177/", 35177));
        assert!(!matches_preview_url("http://127.0.0.1:1420/", 35177));
        assert!(!matches_preview_url("https://127.0.0.1:35177/", 35177));
        assert!(!matches_preview_url("http://example.com:35177/", 35177));
        assert!(!matches_preview_url("http://127.0.0.1/", 35177));
    }
}
