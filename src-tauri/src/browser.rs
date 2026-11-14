use serde::{Deserialize, Serialize};
use std::time::Duration;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use tokio::time::sleep;

const PREFIX: &str = "browser-";
const MAX_WINDOWS: u32 = 6;

fn parse(url: &str) -> Result<tauri::Url, String> {
    url.parse::<tauri::Url>()
        .map_err(|error| format!("browser: {error}"))
}

/// A bare host is a web address, not a relative path. Without this the companion's
/// `browser_read {url: "example.com"}` would fail at `Url::parse` and look like a broken tool.
fn normalize_url(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        trimmed.to_string()
    } else {
        format!("https://{trimmed}")
    }
}

fn raise(window: &WebviewWindow) {
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
}

fn open_windows(app: &AppHandle) -> Vec<(u32, WebviewWindow)> {
    (1..=MAX_WINDOWS)
        .filter_map(|index| {
            app.get_webview_window(&format!("{PREFIX}{index}"))
                .map(|window| (index, window))
        })
        .collect()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserWindowInfo {
    pub id: String,
    pub url: String,
    /// What the window calls itself. Without it "browser-1" is just a slot number.
    pub title: String,
    /// The newest window, which `browser_read` without an id would read.
    pub newest: bool,
}

/// Opens a URL in a window we own.
///
/// `new_window` decides between reusing the most recent window and opening another one —
/// web design work needs several pages side by side, everything else wants one. The cap
/// exists so a stray loop cannot fill the desktop.
///
/// Note: each window carries native chrome instead of a toolbar of ours. Putting our own
/// toolbar above a foreign page needs Tauri's multi-webview support, which is behind the
/// `unstable` feature flag — not something to ship in a product.
///
/// `async` is not decoration. Tauri runs a *synchronous* command on the thread that received the
/// IPC call — the UI thread — and building a window from there waits for the event loop that is
/// currently blocked. On GTK this got away with it; on Windows it deadlocked: the window appeared
/// white, its close button did nothing, and even "Quit" in the tray had no effect because the
/// loop never came back. An `async` command runs on the runtime instead, which is where window
/// creation belongs.
#[tauri::command]
pub async fn open_browser_window(
    app: AppHandle,
    url: String,
    new_window: Option<bool>,
    focus: Option<bool>,
) -> Result<String, String> {
    let target = parse(&url)?;
    let raise_after = focus.unwrap_or(true);
    let open = open_windows(&app);

    if !new_window.unwrap_or(false) {
        if let Some((_, window)) = open.last() {
            window
                .navigate(target)
                .map_err(|error| format!("browser: {error}"))?;
            if raise_after {
                raise(window);
            }
            return Ok(window.label().to_string());
        }
    }

    let used: Vec<u32> = open.iter().map(|(index, _)| *index).collect();
    let Some(slot) = (1..=MAX_WINDOWS).find(|index| !used.contains(index)) else {
        return Err(format!("browser: {MAX_WINDOWS} windows are already open"));
    };

    let label = format!("{PREFIX}{slot}");
    // One line per window: without it a white window and a report of "the browser does not work"
    // cannot be told apart from a window that never got created.
    crate::log_line!("browser: opening {label} for {target}");
    let window = WebviewWindowBuilder::new(&app, &label, WebviewUrl::External(target.clone()))
        .title("DS·IDE — Browser")
        .inner_size(1120.0, 840.0)
        // Low enough to measure a narrow breakpoint (the companion's preview can ask for
        // 400 px), high enough that the window still has usable chrome.
        .min_inner_size(320.0, 280.0)
        // Following the companion must not take the keyboard away from the composer.
        .focused(raise_after)
        .build()
        .map_err(|error| {
            crate::log_line!("browser: {label} failed: {error}");
            format!("browser: {error}")
        })?;
    crate::log_line!("browser: {label} built for {target}");
    if raise_after {
        raise(&window);
    }
    Ok(label)
}

#[tauri::command]
pub fn browser_windows(app: AppHandle) -> Vec<BrowserWindowInfo> {
    let mut windows = open_windows(&app);
    let newest = windows.last().map(|(_, window)| window.label().to_string());
    windows
        .drain(..)
        .map(|(_, window)| {
            let label = window.label().to_string();
            BrowserWindowInfo {
                title: window.title().unwrap_or_default(),
                newest: newest.as_deref() == Some(label.as_str()),
                url: without_marker(&window.url().map(|url| url.to_string()).unwrap_or_default()),
                id: label,
            }
        })
        .collect()
}

#[tauri::command]
pub fn browser_action(app: AppHandle, id: String, action: String) -> Result<(), String> {
    let Some(window) = app.get_webview_window(&id) else {
        return Err(format!("browser: no window '{id}'"));
    };

    if action == "focus" {
        raise(&window);
        return Ok(());
    }

    let script = match action.as_str() {
        "back" => "history.back()",
        "forward" => "history.forward()",
        "reload" => "location.reload()",
        other => return Err(format!("browser: unknown action '{other}'")),
    };
    window.eval(script).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn browser_close(app: AppHandle, id: String) -> Result<(), String> {
    let Some(window) = app.get_webview_window(&id) else {
        return Err(format!("browser: no window '{id}'"));
    };
    window.close().map_err(|error| error.to_string())
}


/// What the page extraction script writes into the URL hash; the host reads it back without
/// giving the remote page any IPC access.
const READ_SCRIPT: &str = r##"
(() => {
  try {
    const cap = (value, limit) => (value || "").slice(0, limit);
    const links = Array.from(document.querySelectorAll("a[href]"))
      .slice(0, 40)
      .map((a) => ({ text: cap((a.innerText || "").trim(), 120), href: a.href }));
    const fields = Array.from(document.querySelectorAll("input, textarea, select"))
      .slice(0, 60)
      .map((el, index) => {
        const type = (el.type || el.tagName).toLowerCase();
        el.setAttribute("data-dside-field", String(index));
        return {
          selector: '[data-dside-field="' + index + '"]',
          name: el.name || "",
          id: el.id || "",
          type,
          value: type === "password" ? "" : cap(el.value || "", 500),
          placeholder: cap(el.placeholder || "", 160),
          checked: !!el.checked,
          options: el.tagName === "SELECT"
            ? Array.from(el.options).slice(0, 20).map((o) => ({ value: o.value, text: cap((o.text || "").trim(), 80) }))
            : []
        };
      });
    const data = {
      url: location.href,
      title: document.title,
      text: cap(document.body ? document.body.innerText : "", 8000),
      links,
      fields,
      layout: (() => {
        const elements = Array.from(document.querySelectorAll("body *"))
          .map((el) => {
            const rect = el.getBoundingClientRect();
            const style = getComputedStyle(el);
            if (rect.width <= 0 || rect.height <= 0 || style.display === "none" || style.visibility === "hidden") return null;
            const leaf = el.children.length === 0;
            return {
              tag: el.tagName.toLowerCase(),
              id: el.id || "",
              classes: cap(typeof el.className === "string" ? el.className : "", 80),
              text: leaf ? cap((el.textContent || "").trim(), 80) : "",
              x: Math.round(rect.x),
              y: Math.round(rect.y),
              width: Math.round(rect.width),
              height: Math.round(rect.height),
              display: style.display,
              position: style.position,
              margin: [style.marginTop, style.marginRight, style.marginBottom, style.marginLeft].join(" "),
              padding: [style.paddingTop, style.paddingRight, style.paddingBottom, style.paddingLeft].join(" "),
              gap: style.gap && style.gap !== "normal" ? style.gap : "",
              align: style.textAlign !== "start" ? style.textAlign : "",
              // Overflow is what a too-long token looks like without seeing it.
              overflow: (el.scrollWidth > el.clientWidth + 1 ? "x" : "") + (el.scrollHeight > el.clientHeight + 1 ? "y" : "") || "",
              color: leaf ? style.color : "",
              background: leaf ? style.backgroundColor : "",
              fontSize: leaf ? style.fontSize : ""
            };
          })
          .filter(Boolean)
          .slice(0, 80);
        return {
          viewport: {
            width: window.innerWidth,
            height: window.innerHeight,
            scrollHeight: document.documentElement ? document.documentElement.scrollHeight : 0
          },
          elements
        };
      })()
    };
    const encoded = btoa(unescape(encodeURIComponent(JSON.stringify(data))));
    try {
      history.replaceState(null, "", location.pathname + location.search + "#__dside_read=" + encoded);
    } catch (_) {
      location.hash = "__dside_read=" + encoded;
    }
  } catch (error) {
    try {
      history.replaceState(null, "", location.pathname + location.search + "#__dside_read_error=1");
    } catch (_) {}
  }
})();
"##;

#[derive(Serialize, Deserialize)]
pub struct BrowserLink {
    pub text: String,
    pub href: String,
}

#[derive(Serialize, Deserialize)]
pub struct BrowserField {
    pub selector: String,
    pub name: String,
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub value: String,
    pub placeholder: String,
    pub checked: bool,
    pub options: Vec<BrowserOption>,
}

#[derive(Serialize, Deserialize)]
pub struct BrowserOption {
    pub value: String,
    pub text: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserPage {
    #[serde(default)]
    pub id: String,
    pub url: String,
    pub title: String,
    pub text: String,
    pub links: Vec<BrowserLink>,
    pub fields: Vec<BrowserField>,
    /// Element boxes and the viewport: the only way to judge alignment without pixels.
    #[serde(default)]
    pub layout: serde_json::Value,
    /// The document reported neither a title nor any text. It may genuinely be empty, but on
    /// a freshly opened page it usually means the page is still loading — say so instead of
    /// handing back an empty document that looks like a result.
    #[serde(default)]
    pub loading: bool,
}

impl BrowserPage {
    /// Nothing has arrived yet. A page can carry a title from the document head while the
    /// body is still empty; title alone is not evidence that there is something to read.
    fn looks_unloaded(&self) -> bool {
        self.text.trim().is_empty() && self.links.is_empty() && self.fields.is_empty()
    }
}

#[derive(Serialize, Deserialize)]
pub struct BrowserFillField {
    pub selector: String,
    pub value: String,
}

#[derive(Serialize)]
pub struct BrowserFillResult {
    pub filled: i64,
    pub submitted: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserClickResult {
    pub clicked: bool,
    pub tag: String,
    pub text: String,
    /// The click likely started a navigation; the marker vanished before it could be read.
    pub navigated: bool,
    /// Where the window is after the click. A selector from an older read may no longer
    /// match because the page changed; the URL is what tells those two cases apart.
    pub url: String,
}

/// The URL a window is on. Used to decide whether a page is the companion's own preview:
/// that one may be clicked without Browser control.
pub fn window_url(app: &AppHandle, id: Option<&str>) -> Result<String, String> {
    let window = pick_window(app, id)?;
    window
        .url()
        .map(|url| url.to_string())
        .map_err(|error| format!("browser: {error}"))
}

/// Resizes a browser window so the page is laid out at `width` CSS pixels. A layout measured
/// in a 1120 px window says nothing about the narrow panel it may end up in; this is how the
/// companion checks a breakpoint instead of guessing.
pub fn set_viewport(
    app: &AppHandle,
    id: Option<&str>,
    width: Option<u32>,
    height: Option<u32>,
) -> Result<(), String> {
    let window = pick_window(app, id)?;
    if width.is_none() && height.is_none() {
        return Ok(());
    }
    let scale = window.scale_factor().unwrap_or(1.0).max(0.5);
    let current = window
        .inner_size()
        .map_err(|error| format!("browser: {error}"))?;
    let logical_width = width.map(|value| value as f64).unwrap_or(current.width as f64 / scale);
    let logical_height = height.map(|value| value as f64).unwrap_or(current.height as f64 / scale);
    window
        .set_size(tauri::LogicalSize::new(
            logical_width.clamp(240.0, 4000.0),
            logical_height.clamp(200.0, 4000.0),
        ))
        .map_err(|error| format!("browser: {error}"))
}

fn pick_window(app: &AppHandle, id: Option<&str>) -> Result<WebviewWindow, String> {
    if let Some(id) = id {
        return app
            .get_webview_window(id)
            .ok_or_else(|| format!("browser: no window '{id}'"));
    }
    open_windows(app)
        .into_iter()
        .last()
        .map(|(_, window)| window)
        .ok_or_else(|| "browser: no browser window is open".to_string())
}

fn read_hash(url: &str) -> Option<String> {
    let fragment = url.split_once("#__dside_read=").map(|(_, value)| value)?;
    let bytes = crate::base64_decode(fragment)?;
    String::from_utf8(bytes).ok()
}

fn clear_hash(window: &WebviewWindow) {
    let _ = window.eval(
        "try { history.replaceState(null, '', location.pathname + location.search); } catch (_) {}",
    );
}

/// Reads the page the user has open in our browser window: title, URL, visible text, links and
/// form fields. The page itself never gets IPC access; the script hands the data back through
/// a temporary URL hash, which the host removes again.
#[tauri::command]
pub async fn browser_read(app: AppHandle, id: Option<String>) -> Result<BrowserPage, String> {
    let window = pick_window(&app, id.as_deref())?;
    let label = window.label().to_string();
    window
        .eval(READ_SCRIPT)
        .map_err(|error| format!("browser: {error}"))?;

    for _ in 0..20 {
        sleep(Duration::from_millis(50)).await;
        let url = window
            .url()
            .map(|url| url.to_string())
            .unwrap_or_default();
        if let Some(json) = read_hash(&url) {
            clear_hash(&window);
            let mut page: BrowserPage =
                serde_json::from_str(&json).map_err(|error| format!("browser: {error}"))?;
            page.id = label;
            page.loading = page.looks_unloaded();
            follow_title(&window, &page);
            return Ok(page);
        }
    }

    clear_hash(&window);
    Err("browser: the page did not answer (CSP or an unsupported page)".to_string())
}

/// Opens a page for the companion and reads it back. The window is deliberately not focused:
/// the user keeps the keyboard in the composer. A new window is used so a page the user has
/// open is not navigated away; `browser_read` is retried until the new window has left
/// `about:blank`, because a page can still be loading even when it is local.
pub async fn browser_open_read(app: AppHandle, value: String) -> Result<BrowserPage, String> {
    let url = normalize_url(&value);
    if url.is_empty() {
        return Err("browser: empty url".to_string());
    }
    let label = open_browser_window(app.clone(), url, Some(true), Some(false)).await?;
    read_when_loaded(app, label).await
}

/// Navigates a window the caller named and reads it back. `browser_read` without an id
/// deliberately never navigates, because that would replace a page the user is looking at;
/// an explicit window id is the caller taking that decision.
pub async fn browser_open_read_in(
    app: AppHandle,
    value: String,
    label: &str,
) -> Result<BrowserPage, String> {
    let url = normalize_url(&value);
    if url.is_empty() {
        return Err("browser: empty url".to_string());
    }
    let window = app
        .get_webview_window(label)
        .ok_or_else(|| format!("browser: no window '{label}'"))?;
    window
        .navigate(parse(&url)?)
        .map_err(|error| format!("browser: {error}"))?;
    read_when_loaded(app, label.to_string()).await
}

/// Shows a workspace page in the browser: an existing preview window is navigated (iterating
/// on a page should not fill the desktop with windows), otherwise a new one is opened.
pub async fn show_preview_read(app: AppHandle, url: String) -> Result<BrowserPage, String> {
    let target = parse(&url)?;
    let existing = open_windows(&app)
        .into_iter()
        .rev()
        .map(|(_, window)| window)
        .find(|window| {
            window
                .url()
                .map(|current| crate::preview::is_preview_url(&app, &current.to_string()))
                .unwrap_or(false)
        });
    let label = match existing {
        Some(window) => {
            window
                .navigate(target)
                .map_err(|error| format!("browser: {error}"))?;
            window.label().to_string()
        }
        None => open_browser_window(app.clone(), url, Some(true), Some(false)).await?,
    };
    read_when_loaded(app, label).await
}

async fn read_when_loaded(app: AppHandle, label: String) -> Result<BrowserPage, String> {
    let mut last: Option<BrowserPage> = None;
    let mut last_error = String::new();
    for _ in 0..6 {
        match browser_read(app.clone(), Some(label.clone())).await {
            // A page that has its URL but no title and no text is still painting. Retrying is
            // cheaper than handing the companion an empty document it would report as "leer".
            Ok(page) if !page.url.is_empty() && page.url != "about:blank" && !page.loading => {
                return Ok(page)
            }
            Ok(page) => last = Some(page),
            Err(error) => last_error = error,
        }
        sleep(Duration::from_millis(300)).await;
    }

    if let Some(page) = last {
        return Ok(page);
    }
    Err(if last_error.is_empty() {
        "browser: the page did not load".to_string()
    } else {
        last_error
    })
}

/// Names the window after the page, the way a browser does. A window called "Browser" tells
/// the user nothing when several are open; the page title does.
fn follow_title(window: &WebviewWindow, page: &BrowserPage) {
    let title = page.title.trim();
    if title.is_empty() || page.loading {
        return;
    }
    let mut short: String = title.chars().take(70).collect();
    if title.chars().count() > 70 {
        short.push('…');
    }
    let _ = window.set_title(&format!("{short} — DS·IDE"));
}

fn fill_script(fields: &[BrowserFillField], submit: bool, submit_selector: Option<&str>) -> String {
    let fields_json = serde_json::to_string(fields).unwrap_or_else(|_| "[]".to_string());
    let selector_json = serde_json::to_string(&submit_selector).unwrap_or_else(|_| "null".to_string());
    format!(
        r##"
(() => {{
  try {{
    const fields = {fields_json};
    let filled = 0;
    for (const field of fields) {{
      const el = document.querySelector(field.selector);
      if (!el) continue;
      const type = (el.type || "").toLowerCase();
      if (type === "checkbox" || type === "radio") {{
        el.checked = ["1", "true", "on", "yes", "checked"].includes(String(field.value).toLowerCase());
      }} else if (el.tagName === "SELECT") {{
        el.value = field.value;
        if (el.value !== field.value) {{
          const option = Array.from(el.options).find(
            (entry) => entry.value === field.value || (entry.text || "").trim() === field.value
          );
          if (option) el.value = option.value;
        }}
      }} else {{
        const proto = el.tagName === "TEXTAREA" ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype;
        const setter = Object.getOwnPropertyDescriptor(proto, "value")?.set;
        if (setter) setter.call(el, field.value);
        else el.value = field.value;
      }}
      el.dispatchEvent(new Event("input", {{ bubbles: true }}));
      el.dispatchEvent(new Event("change", {{ bubbles: true }}));
      filled += 1;
    }}
    let submitted = 0;
    if ({}) {{
      const target = {selector_json} ? document.querySelector({selector_json}) : null;
      const form = target && target.tagName === "FORM" ? target : target ? target.closest("form") : document.querySelector("form");
      if (target && target.tagName !== "FORM") {{
        target.click();
      }} else if (form) {{
        if (form.requestSubmit) form.requestSubmit();
        else form.submit();
      }}
      submitted = 1;
    }}
    try {{
      history.replaceState(null, "", location.pathname + location.search + "#__dside_fill=" + filled + ":" + submitted);
    }} catch (_) {{}}
  }} catch (error) {{
    try {{
      history.replaceState(null, "", location.pathname + location.search + "#__dside_fill_error=1");
    }} catch (_) {{}}
  }}
}})();
"##,
        if submit { "true" } else { "false" },
        selector_json = selector_json,
    )
}

/// Fills form controls in the newest browser window. The caller must have been allowed by the
/// user's setting and permission stage; the page itself is only touched by this host script.
#[tauri::command]
pub async fn browser_fill(
    app: AppHandle,
    id: Option<String>,
    fields: Vec<BrowserFillField>,
    submit: Option<bool>,
    submit_selector: Option<String>,
) -> Result<BrowserFillResult, String> {
    if fields.is_empty() && !submit.unwrap_or(false) {
        return Err("browser: nothing to fill".to_string());
    }
    let window = pick_window(&app, id.as_deref())?;
    let should_submit = submit.unwrap_or(false);
    let script = fill_script(&fields, should_submit, submit_selector.as_deref());
    window
        .eval(&script)
        .map_err(|error| format!("browser: {error}"))?;

    let mut filled = fields.len() as i64;
    let mut submitted = should_submit;
    for _ in 0..10 {
        sleep(Duration::from_millis(50)).await;
        let url = window
            .url()
            .map(|url| url.to_string())
            .unwrap_or_default();
        if let Some((count, did_submit)) = url
            .split_once("#__dside_fill=")
            .and_then(|(_, value)| value.split_once(':'))
        {
            filled = count.parse().unwrap_or(filled);
            submitted = did_submit.starts_with('1');
            clear_hash(&window);
            return Ok(BrowserFillResult { filled, submitted });
        }
        if url.contains("#__dside_fill_error=1") {
            clear_hash(&window);
            return Err("browser: the fill script failed on this page".to_string());
        }
    }

    // A submit may already have navigated away, taking the marker with it. The script was
    // accepted by the webview; report the intent rather than pretending it failed.
    if should_submit {
        return Ok(BrowserFillResult { filled, submitted });
    }
    clear_hash(&window);
    Err("browser: the page did not confirm the fill (CSP or an unsupported page)".to_string())
}

/// Clicks one element. The marker is written before the click, because a click that starts a
/// navigation takes any later marker with it: reporting first is what makes the difference
/// between "clicked, page changed" and "nothing happened" visible from the host.
fn click_script(selector: &str) -> String {
    let selector_json = serde_json::to_string(selector).unwrap_or_else(|_| "\"\"".to_string());
    format!(
        r##"
(() => {{
  try {{
    const selector = {selector_json};
    const el = selector ? document.querySelector(selector) : null;
    if (!el) {{
      history.replaceState(null, "", location.pathname + location.search + "#__dside_click=missing:0:0:0");
      return;
    }}
    el.scrollIntoView({{ block: "center", inline: "nearest" }});
    const tag = el.tagName.toLowerCase();
    const text = ((el.innerText || el.textContent || "").trim()).slice(0, 80);
    const href = el.getAttribute("href") || "";
    const marker =
      "#__dside_click=ok:" +
      (href ? "1" : "0") + ":" +
      encodeURIComponent(tag) + ":" +
      encodeURIComponent(text);
    try {{
      history.replaceState(null, "", location.pathname + location.search + marker);
    }} catch (_) {{
      location.hash = marker;
    }}
    el.click();
  }} catch (error) {{
    try {{
      history.replaceState(null, "", location.pathname + location.search + "#__dside_click=error");
    }} catch (_) {{}}
  }}
}})();
"##
    )
}

fn decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[index + 1..index + 3]).ok();
            if let Some(value) = hex.and_then(|value| u8::from_str_radix(value, 16).ok()) {
                out.push(value);
                index += 3;
                continue;
            }
        }
        out.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

/// The URL as a reader should see it: the host scripts hand data back through a fragment
/// (`#__dside_click=…`), and that bookkeeping is not part of the page's address.
fn without_marker(url: &str) -> String {
    match url.find("#__dside_") {
        Some(index) => url[..index].to_string(),
        None => url.to_string(),
    }
}

/// Clicks an element in a browser window. The caller decides whether the click is allowed
/// (the companion's own preview page is, a foreign page needs Browser control).
#[tauri::command]
pub async fn browser_click(
    app: AppHandle,
    id: Option<String>,
    selector: String,
) -> Result<BrowserClickResult, String> {
    let selector = selector.trim().to_string();
    if selector.is_empty() {
        return Err("browser: 'selector' is required".to_string());
    }
    if selector.len() > 400 {
        return Err("browser: the selector is too long".to_string());
    }
    let window = pick_window(&app, id.as_deref())?;
    window
        .eval(&click_script(&selector))
        .map_err(|error| format!("browser: {error}"))?;

    for _ in 0..10 {
        sleep(Duration::from_millis(50)).await;
        let url = window
            .url()
            .map(|url| url.to_string())
            .unwrap_or_default();
        let Some((_, marker)) = url.split_once("#__dside_click=") else {
            continue;
        };
        clear_hash(&window);
        let mut parts = marker.splitn(4, ':');
        let state = parts.next().unwrap_or("");
        let href = parts.next().unwrap_or("0");
        let tag = decode(parts.next().unwrap_or(""));
        // The text comes last so colons inside it cannot shift the other fields.
        let text = decode(parts.next().unwrap_or(""));
        let current = without_marker(
            &window
                .url()
                .map(|url| url.to_string())
                .unwrap_or_default(),
        );
        return match state {
            "ok" => Ok(BrowserClickResult {
                clicked: true,
                tag,
                text,
                navigated: href.starts_with('1'),
                url: current,
            }),
            // A selector from an older read and a page that has since navigated produce the
            // same "not found". Naming the current URL is what tells them apart.
            "missing" => Err(format!(
                "browser: no element matches {selector} — the window is now on {current}; \
                 the page may have changed since the read, call browser_read again"
            )),
            _ => Err("browser: the click script failed on this page".to_string()),
        };
    }

    // The marker was written before the click; if it is gone, the click almost certainly
    // started a navigation. Say that instead of reporting a failure that did not happen.
    Ok(BrowserClickResult {
        clicked: true,
        tag: String::new(),
        text: String::new(),
        navigated: true,
        url: without_marker(
            &window
                .url()
                .map(|url| url.to_string())
                .unwrap_or_default(),
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_page_hash() {
        let json = r#"{"url":"https://example.test","title":"Example","text":"hi","links":[],"fields":[]}"#;
        let encoded = crate::base64_encode(json.as_bytes());
        let url = format!("https://example.test/#__dside_read={encoded}");
        assert_eq!(read_hash(&url).as_deref(), Some(json));
        assert!(read_hash("https://example.test/").is_none());
    }

    #[test]
    fn the_reported_url_has_no_bookkeeping_fragment() {
        assert_eq!(
            without_marker("https://example.test/a?b=1#__dside_click=ok:0:button:Ja"),
            "https://example.test/a?b=1"
        );
        assert_eq!(
            without_marker("https://example.test/a#section"),
            "https://example.test/a#section"
        );
    }

    /// An empty document on a freshly opened page is usually a page that is still painting.
    /// Saying so is better than handing back "leer" as if it were the result. A title from the
    /// document head proves nothing: the body can still be empty.
    #[test]
    fn an_empty_document_looks_unloaded() {
        let mut page = BrowserPage {
            id: String::new(),
            url: "https://example.test/".to_string(),
            title: String::new(),
            text: String::new(),
            links: Vec::new(),
            fields: Vec::new(),
            layout: serde_json::json!({}),
            loading: false,
        };
        assert!(page.looks_unloaded());
        page.title = "Example".to_string();
        assert!(page.looks_unloaded(), "a title alone is not content");
        page.text = "hello".to_string();
        assert!(!page.looks_unloaded());
        page.text.clear();
        page.links.push(BrowserLink {
            text: "a".to_string(),
            href: "https://example.test/a".to_string(),
        });
        assert!(!page.looks_unloaded(), "links count as loaded");
    }

    #[test]
    fn a_bare_host_gets_https() {
        assert_eq!(normalize_url("example.com"), "https://example.com");
        assert_eq!(normalize_url(" http://127.0.0.1:8765/ "), "http://127.0.0.1:8765/");
        assert_eq!(normalize_url("https://a.test/x"), "https://a.test/x");
        assert_eq!(normalize_url(""), "");
    }
}
