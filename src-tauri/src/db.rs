use std::collections::HashMap;
use std::path::Path;
use std::sync::{Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS sessions (
  id          TEXT PRIMARY KEY,
  title       TEXT NOT NULL DEFAULT '',
  grp         TEXT NOT NULL DEFAULT 'project',
  workspace   TEXT NOT NULL DEFAULT '',
  archived    INTEGER NOT NULL DEFAULT 0,
  created_at  INTEGER NOT NULL,
  updated_at  INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS messages (
  -- parts holds the answer as a timeline (text, thinking, tool calls in the order they
  -- happened); content stays the plain text for search. See PROGRESS.md section 24.
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  session_id  TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
  role        TEXT NOT NULL,
  content     TEXT,
  tool_calls  TEXT,
  tool_call_id TEXT,
  created_at  INTEGER NOT NULL,
  parts       TEXT
);
CREATE INDEX IF NOT EXISTS idx_messages_session ON messages(session_id, id);

-- Compression: the summary that stands in for the older part of a conversation.
CREATE TABLE IF NOT EXISTS session_summaries (
  session_id         TEXT PRIMARY KEY,
  through_message_id INTEGER NOT NULL,
  summary            TEXT NOT NULL,
  created_at         INTEGER NOT NULL
);

-- The provider's own prompt size for the last request of a session, with how many messages
-- were in it. Compression uses the measurement plus the growth since then instead of a
-- character-count guess, which never saw the system prompt or the tool schemas.
CREATE TABLE IF NOT EXISTS session_usage (
  session_id    TEXT PRIMARY KEY,
  prompt_tokens INTEGER NOT NULL,
  message_count INTEGER NOT NULL,
  updated_at    INTEGER NOT NULL
);

-- Trust is earned over time, never by a single click: one row per awarded gesture, with a
-- daily cap applied when it is written. The name is revealed when the total crosses the
-- threshold (see TRUST_THRESHOLD).
CREATE TABLE IF NOT EXISTS trust_points (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  kind       TEXT NOT NULL,
  points     INTEGER NOT NULL,
  day        TEXT NOT NULL,
  note       TEXT,
  created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS tool_calls (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  session_id  TEXT NOT NULL,
  message_id  INTEGER,
  call_id     TEXT NOT NULL,
  name        TEXT NOT NULL,
  arguments   TEXT,
  output      TEXT,
  is_error    INTEGER NOT NULL DEFAULT 0,
  duration_ms INTEGER,
  created_at  INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_tool_calls_message ON tool_calls(message_id);

CREATE TABLE IF NOT EXISTS todos (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  session_id  TEXT NOT NULL,
  content     TEXT NOT NULL,
  status      TEXT,
  priority    TEXT,
  created_at  INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_todos_session ON todos(session_id);

-- Long-term memory. Filled deliberately, not by default. See PROGRESS.md.
CREATE TABLE IF NOT EXISTS memories (
  id           INTEGER PRIMARY KEY AUTOINCREMENT,
  kind         TEXT NOT NULL,
  scope        TEXT NOT NULL DEFAULT 'user',
  title        TEXT,
  text         TEXT NOT NULL,
  source       TEXT,
  importance   INTEGER NOT NULL DEFAULT 1,
  created_at   INTEGER NOT NULL,
  last_used_at INTEGER,
  origin       TEXT NOT NULL DEFAULT 'seeky',
  edited_at    INTEGER,
  retired_at   INTEGER,
  workspace    TEXT NOT NULL DEFAULT ''
);
CREATE INDEX IF NOT EXISTS idx_memories_scope ON memories(scope, kind);

CREATE TABLE IF NOT EXISTS settings (
  key    TEXT PRIMARY KEY,
  value  TEXT NOT NULL
);

-- Interval scheduler. Deliberately not full cron: an interval is what the UI offers,
-- and pretending otherwise would be dishonest.
CREATE TABLE IF NOT EXISTS schedules (
  id               INTEGER PRIMARY KEY AUTOINCREMENT,
  session_id       TEXT NOT NULL,
  prompt           TEXT NOT NULL,
  interval_seconds INTEGER NOT NULL,
  enabled          INTEGER NOT NULL DEFAULT 1,
  last_run_at      INTEGER,
  next_run_at      INTEGER NOT NULL,
  created_at       INTEGER NOT NULL,
  -- A one-shot reminder is stamped and switched off *before* its turn runs (at-most-once:
  -- a crash in between loses the reminder instead of firing it twice — see
  -- `run_due_schedules`). A repeating schedule keeps going. created_by separates what the
  -- user set up from what the companion set for itself, so the companion reminders can be
  -- capped.
  one_shot         INTEGER NOT NULL DEFAULT 0,
  created_by       TEXT NOT NULL DEFAULT 'user'
);
CREATE INDEX IF NOT EXISTS idx_schedules_due ON schedules(enabled, next_run_at);

-- Snapshots of a file before the partner changed it. `content` NULL means the file did
-- not exist before; reverting then removes it. Kept across sessions on purpose — the user
-- can undo a change after the conversation that caused it is gone.
CREATE TABLE IF NOT EXISTS file_versions (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  path        TEXT NOT NULL,
  content     TEXT,
  existed     INTEGER NOT NULL DEFAULT 1,
  session_id  TEXT,
  origin      TEXT NOT NULL,
  call_id     TEXT,
  turn_id     TEXT,
  created_at  INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_file_versions_path ON file_versions(path, id);

PRAGMA user_version = 1;
";

/// How many points the partner needs before the user is told its name. Deliberately a
/// marathon: a day of use, ten finished turns or a handful of memories are worth far less.
pub const TRUST_THRESHOLD: i64 = 100;

/// The local calendar day, for anything that runs at most once a day.
pub fn local_day() -> String {
    today()
}

/// Local calendar day, for the per-day caps. A year of UTC offsets is not worth a timezone
/// database; the day boundary matters only for "not twice in one day".
fn today() -> String {
    // `date('now','localtime')` is what the memories panel already trusts for dates.
    let stamp = now();
    let days = stamp.div_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}")
}

/// Howard Hinnant's days-from-civil inverse: valid for any date a Unix timestamp can hold.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m as u32, d as u32)
}

pub fn unix_now() -> i64 {
    now()
}

fn row_to_schedule(row: &rusqlite::Row<'_>) -> rusqlite::Result<ScheduleRow> {
    Ok(ScheduleRow {
        id: row.get(0)?,
        session_id: row.get(1)?,
        prompt: row.get(2)?,
        interval_seconds: row.get(3)?,
        enabled: row.get::<_, i32>(4)? != 0,
        last_run_at: row.get(5)?,
        next_run_at: row.get(6)?,
        one_shot: row.get::<_, i32>(7)? != 0,
        created_by: row.get(8)?,
    })
}

const SCHEDULE_COLUMNS: &str = "id, session_id, prompt, interval_seconds, enabled, last_run_at, \
     next_run_at, one_shot, created_by";

fn row_to_memory(row: &rusqlite::Row<'_>) -> rusqlite::Result<MemoryRow> {
    Ok(MemoryRow {
        id: row.get(0)?,
        kind: row.get(1)?,
        scope: row.get(2)?,
        title: row.get(3)?,
        text: row.get(4)?,
        importance: row.get(5)?,
        created_at: row.get(6)?,
        origin: row.get(7)?,
        edited_at: row.get(8)?,
        retired_at: row.get(9)?,
        workspace: row.get(10)?,
        basis: row.get(11)?,
    })
}

/// Reads the image parts of a stored message back into the data URLs the provider wants.
/// Missing files are skipped — a deleted picture must not break the conversation.
/// The summary row of a session, on a connection that is already locked. Taking the lock a
/// second time inside a method that holds it deadlocks the whole database.
fn summary_of(conn: &Connection, session_id: &str) -> Result<Option<(i64, String)>, String> {
    conn.query_row(
        "SELECT through_message_id, summary FROM session_summaries WHERE session_id = ?1",
        params![session_id],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
    )
    .optional()
    .map_err(|error| error.to_string())
}

fn images_from_parts(parts: Option<&str>) -> Option<Vec<String>> {
    let parsed: Vec<serde_json::Value> = serde_json::from_str(parts?).ok()?;
    let images: Vec<String> = parsed
        .iter()
        .filter(|part| part.get("kind").and_then(|kind| kind.as_str()) == Some("image"))
        .filter_map(|part| {
            let path = part.get("path")?.as_str()?;
            let mime = part
                .get("mime")
                .and_then(|value| value.as_str())
                .unwrap_or("image/png");
            let bytes = std::fs::read(path).ok()?;
            Some(format!(
                "data:{mime};base64,{}",
                crate::base64_encode(&bytes)
            ))
        })
        .collect();
    if images.is_empty() {
        None
    } else {
        Some(images)
    }
}

/// Adds a column to an existing table when an older database does not have it yet.
fn add_column(
    conn: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> Result<(), String> {
    let mut statement = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(|error| error.to_string())?;
    let columns: Vec<String> = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|error| error.to_string())?
        .filter_map(Result::ok)
        .collect();

    if columns.iter().any(|name| name == column) {
        return Ok(());
    }
    conn.execute(
        &format!("ALTER TABLE {table} ADD COLUMN {column} {definition}"),
        [],
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

/// The panel gives each memory one line for its heading. 120 characters fit a card without
/// turning the grid back into a wall of text.
const TITLE_MAX_CHARS: usize = 120;

fn clip(text: &str) -> String {
    if text.chars().count() <= TITLE_MAX_CHARS {
        return text.to_string();
    }
    let mut out: String = text.chars().take(TITLE_MAX_CHARS - 1).collect();
    out.push('…');
    out
}

fn clean_title(value: Option<&str>) -> Option<String> {
    let value = value?;
    let flat = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.is_empty() {
        return None;
    }
    Some(clip(&flat))
}

/// Older rows (and user entries without a heading) get their first line as the title.
fn derive_title(text: &str) -> String {
    let first = text
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("");
    let flat = first.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.is_empty() {
        "Memory".to_string()
    } else {
        clip(&flat)
    }
}

/// Cut a readable window around the first match, so hits make sense in a list.
fn snippet_around(text: &str, needle: &str, width: usize) -> String {
    let lowered = text.to_lowercase();
    let needle_lower = needle.to_lowercase();
    let chars: Vec<char> = text.chars().collect();

    let Some(position) = lowered.find(&needle_lower) else {
        return chars.into_iter().take(width).collect();
    };

    let prefix_chars = text[..position].chars().count();
    let start = prefix_chars.saturating_sub(width / 3);
    let end = (start + width).min(chars.len());

    let mut out = String::new();
    if start > 0 {
        out.push('…');
    }
    out.extend(chars[start..end].iter());
    if end < chars.len() {
        out.push('…');
    }
    out.replace('\n', " ")
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionRow {
    pub id: String,
    pub title: String,
    pub group: String,
    pub workspace: String,
    pub archived: bool,
    pub updated_at: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolRow {
    pub id: String,
    pub name: String,
    pub arguments: Option<String>,
    pub output: Option<String>,
    pub is_error: bool,
    pub duration_ms: Option<i64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryMessage {
    pub id: String,
    pub role: String,
    pub text: String,
    pub tools: Vec<ToolRow>,
    /// Raw JSON of the timeline; the frontend turns it into message parts. `None` for rows
    /// written before this existed, and for user messages.
    pub parts: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TodoRow {
    pub id: i64,
    pub content: String,
    pub status: Option<String>,
    pub priority: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleRow {
    pub id: i64,
    pub session_id: String,
    pub prompt: String,
    pub interval_seconds: i64,
    pub enabled: bool,
    pub last_run_at: Option<i64>,
    pub next_run_at: i64,
    pub one_shot: bool,
    /// `user` for one the user set up, `companion` for a self-reminder.
    pub created_by: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    pub kind: String,
    pub session_id: String,
    pub session_title: String,
    pub snippet: String,
    /// The session's workspace; empty for the global search dialog, named for recall.
    pub workspace: String,
}

/// One result of the companion's own recall: a stored note or a line from an earlier
/// conversation. Both fit in the same shape so the model does not have to parse two.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecallHit {
    /// "memory", "session" or "message".
    pub kind: String,
    pub title: String,
    pub text: String,
    /// "#12 [fact/user] seeky" for a note, the hit kind for a conversation line.
    pub source: String,
    pub session_id: String,
    pub workspace: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryReport {
    pub total: i64,
    /// Entries that repeat an older one (exact after normalising whitespace and case).
    pub duplicates: Vec<i64>,
    /// Never used and older than the cutoff — candidates for forgetting.
    pub stale: Vec<i64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryRow {
    pub id: i64,
    pub kind: String,
    pub scope: String,
    /// A short heading for the panel. `None` for rows written before titles existed.
    pub title: Option<String>,
    pub text: String,
    pub importance: i64,
    pub created_at: i64,
    /// `seeky` or `user` — whose words these are. The user marker is what the recall shows.
    pub origin: String,
    /// When the user last changed the entry, if they ever did.
    pub edited_at: Option<i64>,
    /// When the entry was set aside. Only the retired list has this set.
    pub retired_at: Option<i64>,
    /// The workspace this note belongs to; empty means it applies everywhere.
    pub workspace: String,
    /// `verified` when a tool result confirmed it, `assumed` when it was reasoned or inferred.
    /// The recall labels the second kind so a guess cannot quietly become a remembered fact.
    pub basis: String,
}

pub struct Db {
    conn: Mutex<Connection>,
}

/// Metadata for one snapshot; the content is loaded only when a diff is opened.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileVersionInfo {
    pub id: i64,
    pub path: String,
    pub origin: String,
    pub existed: bool,
    pub bytes: i64,
    pub created_at: i64,
}

pub struct FileVersionRow {
    pub info: FileVersionInfo,
    pub content: Option<String>,
}

/// The first snapshot of a path inside one turn. Its content is the state before the turn;
/// the turn review compares it with the file as it is now.
pub struct TurnVersionRow {
    pub id: i64,
    pub path: String,
    pub content: Option<String>,
    pub existed: bool,
    pub origin: String,
}

impl Db {
    pub fn open(path: &Path) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let conn = Connection::open(path).map_err(|error| error.to_string())?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
            .map_err(|error| error.to_string())?;
        conn.execute_batch(SCHEMA).map_err(|error| error.to_string())?;
        Self::migrate(&conn)?;

        // The database contains prompts, tool output and file content: keep it private.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
        }

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    fn lock(&self) -> Result<MutexGuard<'_, Connection>, String> {
        self.conn.lock().map_err(|error| error.to_string())
    }

    /// `CREATE TABLE IF NOT EXISTS` never alters an existing table, so columns added
    /// after the first release need an explicit step here.
    fn migrate(conn: &Connection) -> Result<(), String> {
        add_column(conn, "sessions", "archived", "INTEGER NOT NULL DEFAULT 0")?;
        // The companion rows: who owns a memory (Seeky or the user), when the user last
        // touched it, and when it was retired. Retired rows stay in the table — the memory
        // is set aside, not erased.
        add_column(conn, "memories", "origin", "TEXT NOT NULL DEFAULT 'seeky'")?;
        add_column(conn, "memories", "edited_at", "INTEGER")?;
        add_column(conn, "memories", "retired_at", "INTEGER")?;
        // A short heading for the memory panel. Older rows have none and the UI derives one.
        add_column(conn, "memories", "title", "TEXT")?;
        // Empty means the note is global; a path scopes it to one workspace.
        add_column(conn, "memories", "workspace", "TEXT NOT NULL DEFAULT ''")?;
        // Notes written before this existed were reasoned, not checked; calling them assumed is
        // the honest default, and a note the user wrote is marked as theirs by `origin` anyway.
        add_column(conn, "memories", "basis", "TEXT NOT NULL DEFAULT 'assumed'")?;
        // The ordered timeline of an assistant answer.
        add_column(conn, "messages", "parts", "TEXT")?;
        add_column(conn, "file_versions", "turn_id", "TEXT")?;
        // Every schedule that existed before this was set up by the user and repeats.
        add_column(conn, "schedules", "one_shot", "INTEGER NOT NULL DEFAULT 0")?;
        add_column(conn, "schedules", "created_by", "TEXT NOT NULL DEFAULT 'user'")?;
        // Round 81 added the reveal threshold. Anyone who already has a chosen name keeps
        // seeing it — the rule is for installations that start from nothing.
        let has_name: bool = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'partnerName'",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|error| error.to_string())?
            .map(|value| value.trim().trim_matches('"').len() > 0)
            .unwrap_or(false);
        let has_flag: bool = conn
            .query_row(
                "SELECT 1 FROM settings WHERE key = 'trustRevealed'",
                [],
                |_row| Ok(true),
            )
            .optional()
            .map_err(|error| error.to_string())?
            .unwrap_or(false);
        if has_name && !has_flag {
            conn.execute(
                "INSERT INTO settings (key, value) VALUES ('trustRevealed', 'true')
                 ON CONFLICT(key) DO UPDATE SET value = 'true'",
                [],
            )
            .map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    pub fn ensure_session(
        &self,
        id: &str,
        title: &str,
        group: &str,
        workspace: &str,
    ) -> Result<(), String> {
        let conn = self.lock()?;
        let stamp = now();
        conn.execute(
            "INSERT INTO sessions (id, title, grp, workspace, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?5)
             ON CONFLICT(id) DO UPDATE SET
               updated_at = ?5,
               workspace  = ?4,
               title      = CASE WHEN ?2 = '' THEN sessions.title ELSE ?2 END",
            params![id, title, group, workspace, stamp],
        )
        .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn touch_session(&self, id: &str) -> Result<(), String> {
        let conn = self.lock()?;
        conn.execute(
            "UPDATE sessions SET updated_at = ?2 WHERE id = ?1",
            params![id, now()],
        )
        .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn session_title(&self, id: &str) -> Result<Option<String>, String> {
        let conn = self.lock()?;
        conn.query_row(
            "SELECT title FROM sessions WHERE id = ?1",
            params![id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| error.to_string())
    }

    pub fn update_session(
        &self,
        id: &str,
        title: Option<&str>,
        group: Option<&str>,
    ) -> Result<(), String> {
        let conn = self.lock()?;
        let stamp = now();
        if let Some(title) = title {
            conn.execute(
                "UPDATE sessions SET title = ?2, updated_at = ?3 WHERE id = ?1",
                params![id, title, stamp],
            )
            .map_err(|error| error.to_string())?;
        }
        if let Some(group) = group {
            conn.execute(
                "UPDATE sessions SET grp = ?2, updated_at = ?3 WHERE id = ?1",
                params![id, group, stamp],
            )
            .map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    pub fn list_sessions(&self) -> Result<Vec<SessionRow>, String> {
        self.session_rows("WHERE archived = 0")
    }

    pub fn list_archived_sessions(&self) -> Result<Vec<SessionRow>, String> {
        self.session_rows("WHERE archived = 1")
    }

    fn session_rows(&self, filter: &str) -> Result<Vec<SessionRow>, String> {
        let conn = self.lock()?;
        let mut statement = conn
            .prepare(&format!(
                "SELECT id, title, grp, workspace, archived, updated_at
                 FROM sessions {filter} ORDER BY updated_at DESC"
            ))
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([], |row| {
                Ok(SessionRow {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    group: row.get(2)?,
                    workspace: row.get(3)?,
                    archived: row.get::<_, i32>(4)? != 0,
                    updated_at: row.get(5)?,
                })
            })
            .map_err(|error| error.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())
    }

    /// Archiving only takes a session out of the sidebar — nothing is deleted.
    pub fn set_session_archived(&self, id: &str, archived: bool) -> Result<(), String> {
        let conn = self.lock()?;
        conn.execute(
            "UPDATE sessions SET archived = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, archived as i32, now()],
        )
        .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn create_session(&self, id: &str, title: &str, workspace: &str) -> Result<(), String> {
        self.ensure_session(id, title, "project", workspace)
    }

    /// Moves a thread into a project after the fact. Threads written before projects existed
    /// carry an empty workspace; this is how they get one without editing the database.
    pub fn set_session_workspace(&self, id: &str, workspace: &str) -> Result<(), String> {
        let conn = self.lock()?;
        conn.execute(
            "UPDATE sessions SET workspace = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, workspace, now()],
        )
        .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn delete_session(&self, id: &str) -> Result<(), String> {
        let conn = self.lock()?;
        conn.execute("DELETE FROM messages WHERE session_id = ?1", params![id])
            .map_err(|error| error.to_string())?;
        conn.execute("DELETE FROM tool_calls WHERE session_id = ?1", params![id])
            .map_err(|error| error.to_string())?;
        conn.execute("DELETE FROM todos WHERE session_id = ?1", params![id])
            .map_err(|error| error.to_string())?;
        conn.execute("DELETE FROM session_usage WHERE session_id = ?1", params![id])
            .map_err(|error| error.to_string())?;
        conn.execute("DELETE FROM session_summaries WHERE session_id = ?1", params![id])
            .map_err(|error| error.to_string())?;
        conn.execute("DELETE FROM sessions WHERE id = ?1", params![id])
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    /// Removes the failed tail of a turn: everything after the last user message. The user
    /// message stays, so a retry continues the same request instead of duplicating it.
    pub fn rollback_failed_turn(&self, session_id: &str) -> Result<(), String> {
        let conn = self.lock()?;
        let last_user: Option<i64> = conn
            .query_row(
                "SELECT id FROM messages
                 WHERE session_id = ?1 AND role = 'user'
                 ORDER BY id DESC LIMIT 1",
                params![session_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| error.to_string())?;
        let Some(last_user) = last_user else {
            return Err("engine: nothing to retry".to_string());
        };
        conn.execute(
            "DELETE FROM tool_calls WHERE session_id = ?1 AND message_id IN
               (SELECT id FROM messages WHERE session_id = ?1 AND id > ?2)",
            params![session_id, last_user],
        )
        .map_err(|error| error.to_string())?;
        conn.execute(
            "DELETE FROM messages WHERE session_id = ?1 AND id > ?2",
            params![session_id, last_user],
        )
        .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn insert_message(
        &self,
        session_id: &str,
        role: &str,
        content: Option<&str>,
        tool_calls: Option<&str>,
        tool_call_id: Option<&str>,
    ) -> Result<i64, String> {
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO messages (session_id, role, content, tool_calls, tool_call_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![session_id, role, content, tool_calls, tool_call_id, now()],
        )
        .map_err(|error| error.to_string())?;
        Ok(conn.last_insert_rowid())
    }

    /// The assistant row is created before the first round and kept up to date, so one turn
    /// is one message in the log — the same thing the user sees while it runs. The old code
    /// wrote a row per round, which is why a reloaded conversation had more bubbles than the
    /// live one.
    pub fn update_message(
        &self,
        id: i64,
        content: Option<&str>,
        tool_calls: Option<&str>,
        parts: Option<&str>,
    ) -> Result<(), String> {
        let conn = self.lock()?;
        conn.execute(
            "UPDATE messages SET content = ?2, tool_calls = ?3, parts = ?4 WHERE id = ?1",
            params![id, content, tool_calls, parts],
        )
        .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn insert_tool_call(
        &self,
        session_id: &str,
        message_id: Option<i64>,
        call_id: &str,
        name: &str,
        arguments: Option<&str>,
        output: Option<&str>,
        is_error: bool,
        duration_ms: Option<i64>,
    ) -> Result<(), String> {
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO tool_calls
               (session_id, message_id, call_id, name, arguments, output, is_error, duration_ms, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                session_id,
                message_id,
                call_id,
                name,
                arguments,
                output,
                is_error as i32,
                duration_ms,
                now()
            ],
        )
        .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn replace_todos(&self, session_id: &str, todos: &[serde_json::Value]) -> Result<(), String> {
        let conn = self.lock()?;
        conn.execute("DELETE FROM todos WHERE session_id = ?1", params![session_id])
            .map_err(|error| error.to_string())?;
        let stamp = now();
        for todo in todos {
            let content = todo
                .get("content")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            if content.is_empty() {
                continue;
            }
            let status = todo.get("status").and_then(serde_json::Value::as_str);
            let priority = todo.get("priority").and_then(serde_json::Value::as_str);
            conn.execute(
                "INSERT INTO todos (session_id, content, status, priority, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![session_id, content, status, priority, stamp],
            )
            .map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    pub fn list_todos(&self, session_id: &str) -> Result<Vec<TodoRow>, String> {
        let conn = self.lock()?;
        let mut statement = conn
            .prepare(
                "SELECT id, content, status, priority FROM todos
                 WHERE session_id = ?1 ORDER BY id",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([session_id], |row| {
                Ok(TodoRow {
                    id: row.get(0)?,
                    content: row.get(1)?,
                    status: row.get(2)?,
                    priority: row.get(3)?,
                })
            })
            .map_err(|error| error.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())
    }

    fn tools_for(conn: &Connection, message_id: i64) -> Result<Vec<ToolRow>, String> {
        let mut statement = conn
            .prepare(
                "SELECT call_id, name, arguments, output, is_error, duration_ms
                 FROM tool_calls WHERE message_id = ?1 ORDER BY id",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([message_id], |row| {
                Ok(ToolRow {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    arguments: row.get(2)?,
                    output: row.get(3)?,
                    is_error: row.get::<_, i32>(4)? != 0,
                    duration_ms: row.get(5)?,
                })
            })
            .map_err(|error| error.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())
    }

    /// The whole conversation, for tests that compare a thread end to end. The UI reads pages
    /// through `load_history_page`, so this would otherwise be dead weight in a release build.
    #[cfg(test)]
    pub fn load_history(&self, session_id: &str) -> Result<Vec<HistoryMessage>, String> {
        self.load_history_page(session_id, None, 0)
    }

    /// One page of the conversation, oldest first. `before_id` continues from a message the
    /// caller already has; `limit` 0 means everything. A long thread is read in pages so the
    /// window opens at once instead of waiting for hundreds of rows and every tool call in
    /// them.
    pub fn load_history_page(
        &self,
        session_id: &str,
        before_id: Option<i64>,
        limit: usize,
    ) -> Result<Vec<HistoryMessage>, String> {
        let conn = self.lock()?;
        let mut sql = String::from(
            "SELECT id, role, content, parts FROM messages
             WHERE session_id = ?1 AND role IN ('user', 'assistant')",
        );
        let mut args: Vec<rusqlite::types::Value> = vec![session_id.to_string().into()];
        if let Some(before) = before_id {
            args.push(before.into());
            sql.push_str(&format!(" AND id < ?{}", args.len()));
        }
        if limit > 0 {
            // Newest first to make the LIMIT cheap, then turned around for reading.
            sql.push_str(&format!(" ORDER BY id DESC LIMIT {limit}"));
        } else {
            sql.push_str(" ORDER BY id");
        }
        let mut statement = conn.prepare(&sql).map_err(|error| error.to_string())?;
        let mut rows = statement
            .query_map(rusqlite::params_from_iter(args.iter()), |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            })
            .map_err(|error| error.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;
        if limit > 0 {
            rows.reverse();
        }

        let mut history = Vec::with_capacity(rows.len());
        for (id, role, content, parts) in rows {
            history.push(HistoryMessage {
                id: format!("m{id}"),
                role,
                text: content.unwrap_or_default(),
                tools: Self::tools_for(&conn, id)?,
                parts,
            });
        }
        Ok(history)
    }

    pub fn load_llm_messages(
        &self,
        session_id: &str,
    ) -> Result<Vec<crate::engine::types::ChatMessage>, String> {
        let conn = self.lock()?;
        // Everything the summary already covers is not sent again.
        let through = summary_of(&conn, session_id)?.map(|(id, _)| id).unwrap_or(0);
        let mut statement = conn
            .prepare(
                "SELECT role, content, tool_calls, tool_call_id, parts FROM messages
                 WHERE session_id = ?1 AND id > ?2 ORDER BY id",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map(params![session_id, through], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                ))
            })
            .map_err(|error| error.to_string())?;

        let mut messages = Vec::new();
        // Tool results are only valid immediately after the assistant message that asked for
        // them. Older databases can contain orphans (from the per-round bug), so anything
        // without a matching call is dropped instead of being sent and rejected.
        let mut pending_calls: std::collections::HashSet<String> = std::collections::HashSet::new();
        if let Some((_, summary)) = summary_of(&conn, session_id)? {
            messages.push(crate::engine::types::ChatMessage::system(format!(
                "Earlier in this conversation (compressed, keep it in mind):\n{summary}"
            )));
        }
        for row in rows {
            let (role, content, tool_calls, tool_call_id, parts) =
                row.map_err(|error| error.to_string())?;
            let parsed_calls = tool_calls
                .as_deref()
                .and_then(|raw| serde_json::from_str::<Vec<crate::engine::types::OutgoingToolCall>>(raw).ok());
            // A question that came with pictures sends them again: the earlier turn is still
            // part of the conversation the model is answering.
            let images = if role == "user" {
                images_from_parts(parts.as_deref())
            } else {
                None
            };
            // A turn that failed after its row was created leaves an assistant row with
            // nothing in it. The provider rejects an assistant message without content, and
            // sending it would poison every later turn of the conversation.
            let empty_assistant = role == "assistant"
                && content.as_deref().map(str::trim).unwrap_or("").is_empty()
                && parsed_calls.as_ref().map(Vec::is_empty).unwrap_or(true);
            if empty_assistant {
                continue;
            }

            if role == "assistant" {
                pending_calls = parsed_calls
                    .iter()
                    .flatten()
                    .map(|call| call.id.clone())
                    .collect();
            } else if role == "tool" {
                let answered = tool_call_id
                    .as_deref()
                    .map(|id| pending_calls.remove(id))
                    .unwrap_or(false);
                if !answered {
                    continue;
                }
            } else {
                pending_calls.clear();
            }
            messages.push(crate::engine::types::ChatMessage {
                role,
                content,
                image: images,
                tool_calls: parsed_calls,
                tool_call_id,
            });
        }
        Ok(messages)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), String> {
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = ?2",
            params![key, value],
        )
        .map_err(|error| error.to_string())?;
        Ok(())
    }

    /// Tools the user allowed permanently from a permission question. Stored in the settings
    /// table so it survives restarts; the user can only grow this list from the dialog.
    pub fn trusted_tools(&self) -> Result<Vec<String>, String> {
        let value = self
            .all_settings()?
            .get("trustedTools")
            .cloned()
            .unwrap_or_default();
        if value.trim().is_empty() {
            return Ok(Vec::new());
        }
        serde_json::from_str::<Vec<String>>(&value).map_err(|error| error.to_string())
    }

    pub fn add_trusted_tool(&self, name: &str) -> Result<(), String> {
        let mut tools = self.trusted_tools()?;
        if !tools.iter().any(|tool| tool == name) {
            tools.push(name.to_string());
        }
        let value = serde_json::to_string(&tools).map_err(|error| error.to_string())?;
        self.set_setting("trustedTools", &value)
    }

    /// Awards trust once per gesture, honouring a per-day cap. Returns whether something was
    /// written, so the caller can avoid celebrating a no-op. `kind` doubles as the cap key:
    /// "day" (first turn of a day), "turn" (a finished turn that did something), "memory"
    /// (a note the companion wrote on its own) and "correction" (the user edited a note).
    pub fn award_trust(
        &self,
        kind: &str,
        points: i64,
        cap_per_day: i64,
        note: Option<&str>,
    ) -> Result<bool, String> {
        if points <= 0 || cap_per_day <= 0 {
            return Ok(false);
        }
        let conn = self.lock()?;
        let day = today();
        let already: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(points), 0) FROM trust_points WHERE kind = ?1 AND day = ?2",
                params![kind, day],
                |row| row.get(0),
            )
            .map_err(|error| error.to_string())?;
        if already >= cap_per_day {
            return Ok(false);
        }
        let points = points.min(cap_per_day - already);
        conn.execute(
            "INSERT INTO trust_points (kind, points, day, note, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![kind, points, day, note, unix_now()],
        )
        .map_err(|error| error.to_string())?;
        Ok(true)
    }

    pub fn trust_total(&self) -> Result<i64, String> {
        let conn = self.lock()?;
        conn.query_row("SELECT COALESCE(SUM(points), 0) FROM trust_points", [], |row| {
            row.get(0)
        })
        .map_err(|error| error.to_string())
    }

    /// Points per kind, biggest first — the partner page shows where they came from.
    pub fn trust_breakdown(&self) -> Result<Vec<(String, i64)>, String> {
        let conn = self.lock()?;
        let mut statement = conn
            .prepare(
                "SELECT kind, SUM(points) AS total FROM trust_points
                 GROUP BY kind ORDER BY total DESC",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(|error| error.to_string())?;
        rows.collect::<Result<Vec<(String, i64)>, _>>()
            .map_err(|error| error.to_string())
    }

    pub fn trust_revealed(&self) -> Result<bool, String> {
        Ok(self
            .all_settings()?
            .get("trustRevealed")
            .map(|value| value.trim().trim_matches('"') == "true")
            .unwrap_or(false))
    }

    /// Remembers the reveal. Once seen, the name stays visible: trust is not a game show.
    pub fn reveal_trust(&self) -> Result<(), String> {
        self.set_setting("trustRevealed", "true")
    }

    /// Remembers the provider's prompt size for the last request of a session. `message_count`
    /// is how many messages that request carried (without the system prompt), so the next turn
    /// can add only what arrived since.
    pub fn set_session_usage(
        &self,
        session_id: &str,
        prompt_tokens: i64,
        message_count: i64,
    ) -> Result<(), String> {
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO session_usage (session_id, prompt_tokens, message_count, updated_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(session_id) DO UPDATE SET
               prompt_tokens = ?2, message_count = ?3, updated_at = ?4",
            params![session_id, prompt_tokens, message_count, unix_now()],
        )
        .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn session_usage(&self, session_id: &str) -> Result<Option<(i64, i64)>, String> {
        let conn = self.lock()?;
        conn.query_row(
            "SELECT prompt_tokens, message_count FROM session_usage WHERE session_id = ?1",
            params![session_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|error| error.to_string())
    }

    pub fn all_settings(&self) -> Result<HashMap<String, String>, String> {
        let conn = self.lock()?;
        let mut statement = conn
            .prepare("SELECT key, value FROM settings")
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|error| error.to_string())?;

        let mut map = HashMap::new();
        for row in rows {
            let (key, value) = row.map_err(|error| error.to_string())?;
            map.insert(key, value);
        }
        Ok(map)
    }

    // -----------------------------------------------------------------------
    // Interval scheduler
    // -----------------------------------------------------------------------

    pub fn list_schedules(&self) -> Result<Vec<ScheduleRow>, String> {
        let conn = self.lock()?;
        let mut statement = conn
            .prepare(&format!(
                "SELECT {SCHEDULE_COLUMNS} FROM schedules ORDER BY next_run_at"
            ))
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([], row_to_schedule)
            .map_err(|error| error.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())
    }

    pub fn create_schedule(
        &self,
        session_id: &str,
        prompt: &str,
        interval_seconds: i64,
    ) -> Result<i64, String> {
        let stamp = now();
        let interval = interval_seconds.max(60);
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO schedules
               (session_id, prompt, interval_seconds, enabled, next_run_at, created_at)
             VALUES (?1, ?2, ?3, 1, ?4, ?5)",
            params![session_id, prompt, interval, stamp + interval, stamp],
        )
        .map_err(|error| error.to_string())?;
        Ok(conn.last_insert_rowid())
    }

    pub fn delete_schedule(&self, id: i64) -> Result<(), String> {
        let conn = self.lock()?;
        conn.execute("DELETE FROM schedules WHERE id = ?1", params![id])
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn set_schedule_enabled(&self, id: i64, enabled: bool) -> Result<(), String> {
        let conn = self.lock()?;
        conn.execute(
            "UPDATE schedules SET enabled = ?2 WHERE id = ?1",
            params![id, enabled as i32],
        )
        .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn due_schedules(&self, at: i64) -> Result<Vec<ScheduleRow>, String> {
        let conn = self.lock()?;
        let mut statement = conn
            .prepare(&format!(
                "SELECT {SCHEDULE_COLUMNS} FROM schedules
                 WHERE enabled = 1 AND next_run_at <= ?1 ORDER BY next_run_at"
            ))
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([at], row_to_schedule)
            .map_err(|error| error.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())
    }

    /// A reminder the companion sets for itself. `delay_seconds` is when it first fires;
    /// `repeat_seconds` turns it into a repeating schedule instead of a one-shot.
    pub fn create_reminder(
        &self,
        session_id: &str,
        prompt: &str,
        delay_seconds: i64,
        repeat_seconds: Option<i64>,
        created_by: &str,
    ) -> Result<i64, String> {
        let stamp = now();
        let delay = delay_seconds.clamp(60, 7 * 24 * 3600);
        let (interval, one_shot) = match repeat_seconds {
            Some(repeat) => (repeat.clamp(300, 7 * 24 * 3600), 0),
            None => (delay, 1),
        };
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO schedules
               (session_id, prompt, interval_seconds, enabled, next_run_at, created_at,
                one_shot, created_by)
             VALUES (?1, ?2, ?3, 1, ?4, ?5, ?6, ?7)",
            params![
                session_id,
                prompt,
                interval,
                stamp + delay,
                stamp,
                one_shot,
                created_by
            ],
        )
        .map_err(|error| error.to_string())?;
        Ok(conn.last_insert_rowid())
    }

    /// How many active reminders the companion has set for this session. The cap keeps a
    /// helpful habit from becoming a token leak.
    pub fn companion_reminder_count(&self, session_id: &str) -> Result<i64, String> {
        let conn = self.lock()?;
        conn.query_row(
            "SELECT COUNT(*) FROM schedules
             WHERE session_id = ?1 AND created_by = 'companion' AND enabled = 1",
            params![session_id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())
    }

    /// Records that a one-shot reminder ran, without rescheduling it. The time stays in the
    /// database so the reminder can be read back later; it is simply not due again.
    pub fn stamp_schedule_run(&self, id: i64) -> Result<(), String> {
        let conn = self.lock()?;
        conn.execute(
            "UPDATE schedules SET last_run_at = ?2 WHERE id = ?1",
            params![id, now()],
        )
        .map_err(|error| error.to_string())?;
        Ok(())
    }

    /// Switching a one-shot off after it ran, so a reminder cannot become a loop. It is called
    /// *before* the turn runs: a crash in between loses the reminder instead of duplicating it,
    /// which is the trade-off a "check the CI" note wants.
    pub fn disable_schedule(&self, id: i64) -> Result<(), String> {
        let conn = self.lock()?;
        conn.execute("UPDATE schedules SET enabled = 0 WHERE id = ?1", params![id])
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn mark_schedule_run(&self, id: i64, next_run_at: i64) -> Result<(), String> {
        let conn = self.lock()?;
        conn.execute(
            "UPDATE schedules SET last_run_at = ?2, next_run_at = ?3 WHERE id = ?1",
            params![id, now(), next_run_at],
        )
        .map_err(|error| error.to_string())?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Search
    // -----------------------------------------------------------------------

    /// The search dialog searches everything. `recall` uses the same engine but scopes the
    /// results to the current project; an empty workspace means "no project filter".
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchHit>, String> {
        self.search_with_workspace(query, limit, "")
    }

    fn search_with_workspace(
        &self,
        query: &str,
        limit: usize,
        workspace: &str,
    ) -> Result<Vec<SearchHit>, String> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }
        let needle = format!("%{trimmed}%");
        let conn = self.lock()?;
        let mut hits: Vec<SearchHit> = Vec::new();

        let mut sessions = conn
            .prepare(
                "SELECT id, title, COALESCE(workspace, '') FROM sessions
                 WHERE title LIKE ?1
                   AND (?2 = '' OR COALESCE(workspace, '') = '' OR workspace = ?2)
                 ORDER BY updated_at DESC LIMIT ?3",
            )
            .map_err(|error| error.to_string())?;
        let rows = sessions
            .query_map(params![needle, workspace, limit as i64], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(|error| error.to_string())?;
        for row in rows {
            let (id, title, workspace) = row.map_err(|error| error.to_string())?;
            hits.push(SearchHit {
                kind: "session".to_string(),
                session_id: id,
                snippet: title.clone(),
                session_title: title,
                workspace,
            });
        }

        let mut messages = conn
            .prepare(
                "SELECT m.session_id, s.title, COALESCE(s.workspace, ''), m.content
                 FROM messages m JOIN sessions s ON s.id = m.session_id
                 WHERE m.content LIKE ?1 AND m.role IN ('user', 'assistant')
                   AND (?3 = '' OR COALESCE(s.workspace, '') = '' OR s.workspace = ?3)
                 ORDER BY m.id DESC LIMIT ?2",
            )
            .map_err(|error| error.to_string())?;
        let rows = messages
            .query_map(params![needle, limit as i64, workspace], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            })
            .map_err(|error| error.to_string())?;
        for row in rows {
            let (session_id, session_title, workspace, content) =
                row.map_err(|error| error.to_string())?;
            hits.push(SearchHit {
                kind: "message".to_string(),
                session_id,
                session_title,
                snippet: snippet_around(&content.unwrap_or_default(), trimmed, 140),
                workspace,
            });
        }

        hits.truncate(limit);
        Ok(hits)
    }

    /// Searches the notes and the conversation archive in one call. Memories are limited to
    /// the current project plus the global ones, like the system-prompt recall; the message
    /// search is the same one the search dialog uses.
    pub fn recall(
        &self,
        query: &str,
        limit: usize,
        workspace: &str,
    ) -> Result<Vec<RecallHit>, String> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }
        let needle = format!("%{trimmed}%");
        let mut hits: Vec<RecallHit> = Vec::new();

        {
            let conn = self.lock()?;
            let mut statement = conn
                .prepare(
                    "SELECT id, kind, scope, COALESCE(title, ''), text, origin, workspace
                     FROM memories
                     WHERE retired_at IS NULL AND (title LIKE ?1 OR text LIKE ?1)
                       AND (workspace = '' OR workspace = ?2)
                     ORDER BY importance DESC, created_at DESC LIMIT ?3",
                )
                .map_err(|error| error.to_string())?;
            let rows = statement
                .query_map(params![needle, workspace, limit as i64], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, String>(6)?,
                    ))
                })
                .map_err(|error| error.to_string())?;
            for row in rows {
                let (id, kind, scope, title, text, origin, workspace) =
                    row.map_err(|error| error.to_string())?;
                hits.push(RecallHit {
                    kind: "memory".to_string(),
                    title: if title.trim().is_empty() {
                        text.clone()
                    } else {
                        title
                    },
                    text,
                    source: format!("#{id} [{kind}/{scope}] {origin}"),
                    session_id: String::new(),
                    workspace,
                });
            }
        }

        for hit in self.search_with_workspace(trimmed, limit, workspace)? {
            hits.push(RecallHit {
                kind: hit.kind,
                title: hit.session_title.clone(),
                text: hit.snippet,
                source: "conversation".to_string(),
                session_id: hit.session_id,
                workspace: hit.workspace,
            });
        }

        hits.truncate(limit);
        Ok(hits)
    }

    // -----------------------------------------------------------------------
    // Long-term memory
    // -----------------------------------------------------------------------

    pub fn insert_memory(
        &self,
        kind: &str,
        scope: &str,
        text: &str,
        source: Option<&str>,
        origin: &str,
    ) -> Result<i64, String> {
        self.insert_memory_with_title(kind, scope, None, text, source, origin)
    }

    /// The panel shows a heading per entry. A row without one gets the first line of its
    /// text as a fallback, so every new memory is readable at a glance.
    pub fn insert_memory_with_title(
        &self,
        kind: &str,
        scope: &str,
        title: Option<&str>,
        text: &str,
        source: Option<&str>,
        origin: &str,
    ) -> Result<i64, String> {
        let title = clean_title(title).unwrap_or_else(|| derive_title(text));
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO memories
                (kind, scope, title, text, source, importance, created_at, origin)
             VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, ?7)",
            params![kind, scope, title, text, source, now(), origin],
        )
        .map_err(|error| error.to_string())?;
        Ok(conn.last_insert_rowid())
    }

    /// Empty means global; the recall includes global notes everywhere and project notes
    /// only inside their workspace.
    pub fn set_memory_workspace(&self, id: i64, workspace: &str) -> Result<(), String> {
        let conn = self.lock()?;
        conn.execute(
            "UPDATE memories SET workspace = ?2 WHERE id = ?1",
            params![id, workspace],
        )
        .map_err(|error| error.to_string())?;
        Ok(())
    }

    /// The local calendar date, for notes that should carry their "when" in the text itself
    /// (recalled memories are shown without their timestamps).
    pub fn today(&self) -> Result<String, String> {
        let conn = self.lock()?;
        conn.query_row("SELECT date('now', 'localtime')", [], |row| {
            row.get::<_, String>(0)
        })
        .map_err(|error| error.to_string())
    }

    /// Records whether a note is backed by a tool result or was inferred. Kept apart from
    /// `update_memory` so writing a note and judging its evidence stay separate acts.
    pub fn set_memory_basis(&self, id: i64, basis: &str) -> Result<(), String> {
        let basis = match basis {
            "verified" => "verified",
            _ => "assumed",
        };
        let conn = self.lock()?;
        conn.execute(
            "UPDATE memories SET basis = ?1 WHERE id = ?2",
            params![basis, id],
        )
        .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn list_memories(&self, limit: usize) -> Result<Vec<MemoryRow>, String> {
        self.select_memories(
            "SELECT id, kind, scope, title, text, importance, created_at, origin, edited_at, retired_at, workspace, basis
             FROM memories WHERE retired_at IS NULL
             ORDER BY created_at DESC, id DESC LIMIT ?1",
            limit,
        )
    }

    /// Set aside, not erased: these are the entries the user took out of Seeky's memory.
    pub fn list_retired_memories(&self, limit: usize) -> Result<Vec<MemoryRow>, String> {
        self.select_memories(
            "SELECT id, kind, scope, title, text, importance, created_at, origin, edited_at, retired_at, workspace, basis
             FROM memories WHERE retired_at IS NOT NULL
             ORDER BY retired_at DESC, id DESC LIMIT ?1",
            limit,
        )
    }

    fn select_memories(&self, sql: &str, limit: usize) -> Result<Vec<MemoryRow>, String> {
        let conn = self.lock()?;
        let mut statement = conn.prepare(sql).map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([limit as i64], row_to_memory)
            .map_err(|error| error.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())
    }

    /// Most valuable entries first: importance, then freshness. Used for recall.
    /// Retired entries are never recalled; project notes only inside their workspace.
    pub fn memory_context(&self, limit: usize, workspace: &str) -> Result<Vec<MemoryRow>, String> {
        let conn = self.lock()?;
        let mut statement = conn
            .prepare(
                "SELECT id, kind, scope, title, text, importance, created_at, origin, edited_at, retired_at, workspace, basis
                 FROM memories
                 WHERE retired_at IS NULL AND (workspace = '' OR workspace = ?1)
                 ORDER BY importance DESC, COALESCE(last_used_at, 0) DESC, created_at DESC
                 LIMIT ?2",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map(params![workspace, limit as i64], row_to_memory)
            .map_err(|error| error.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())
    }

    /// Rewrites the words and records who owns them now. A change the user makes in the UI
    /// sets `origin` to `user` and stamps `edited_at`; Seeky correcting its own note puts it
    /// back to `seeky` and drops the stamp. The recall reads exactly this pair.
    pub fn update_memory(
        &self,
        id: i64,
        title: Option<&str>,
        text: Option<&str>,
        kind: Option<&str>,
        importance: Option<i64>,
        origin: &str,
    ) -> Result<(), String> {
        let title = clean_title(title);
        let conn = self.lock()?;
        let changed = conn
            .execute(
                "UPDATE memories
                 SET title      = COALESCE(?7, title),
                     text       = COALESCE(?2, text),
                     kind       = COALESCE(?3, kind),
                     importance = COALESCE(?4, importance),
                     origin     = ?6,
                     edited_at  = CASE WHEN ?6 = 'user' THEN ?5 ELSE NULL END
                 WHERE id = ?1",
                params![id, text, kind, importance, now(), origin, title],
            )
            .map_err(|error| error.to_string())?;
        if changed == 0 {
            return Err(format!("no memory #{id}"));
        }
        Ok(())
    }

    pub fn retire_memory(&self, id: i64) -> Result<(), String> {
        let conn = self.lock()?;
        let changed = conn
            .execute(
                "UPDATE memories SET retired_at = ?2 WHERE id = ?1 AND retired_at IS NULL",
                params![id, now()],
            )
            .map_err(|error| error.to_string())?;
        if changed == 0 {
            return Err(format!("no memory #{id}"));
        }
        Ok(())
    }

    pub fn restore_memory(&self, id: i64) -> Result<(), String> {
        let conn = self.lock()?;
        let changed = conn
            .execute(
                "UPDATE memories SET retired_at = NULL WHERE id = ?1",
                params![id],
            )
            .map_err(|error| error.to_string())?;
        if changed == 0 {
            return Err(format!("no memory #{id}"));
        }
        Ok(())
    }

    pub fn touch_memories(&self, ids: &[i64]) -> Result<(), String> {
        if ids.is_empty() {
            return Ok(());
        }
        let conn = self.lock()?;
        let stamp = now();
        for id in ids {
            conn.execute(
                "UPDATE memories SET last_used_at = ?2 WHERE id = ?1",
                params![id, stamp],
            )
            .map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    /// Nothing here deletes a row any more — retiring keeps the history and still gets the
    /// entry out of the way.
    pub fn retire_memories(&self, ids: &[i64]) -> Result<usize, String> {
        let mut retired = 0;
        for id in ids {
            if self.retire_memory(*id).is_ok() {
                retired += 1;
            }
        }
        Ok(retired)
    }

    /// The summary that replaces everything up to `through_message_id`, if the conversation
    /// has been compressed before.
    pub fn session_summary(&self, session_id: &str) -> Result<Option<(i64, String)>, String> {
        let conn = self.lock()?;
        conn.query_row(
            "SELECT through_message_id, summary FROM session_summaries WHERE session_id = ?1",
            params![session_id],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(|error| error.to_string())
    }

    pub fn set_session_summary(
        &self,
        session_id: &str,
        through_message_id: i64,
        summary: &str,
    ) -> Result<(), String> {
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO session_summaries (session_id, through_message_id, summary, created_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(session_id) DO UPDATE SET
               through_message_id = excluded.through_message_id,
               summary = excluded.summary,
               created_at = excluded.created_at",
            params![session_id, through_message_id, summary, now()],
        )
        .map_err(|error| error.to_string())?;
        Ok(())
    }

    /// What compression would fold in: everything after the current summary, oldest first,
    /// with the ids so the summary can record how far it reaches.
    pub fn compactable_messages(
        &self,
        session_id: &str,
    ) -> Result<Vec<(i64, String, String)>, String> {
        let conn = self.lock()?;
        let through = summary_of(&conn, session_id)?.map(|(id, _)| id).unwrap_or(0);
        let mut statement = conn
            .prepare(
                "SELECT id, role, COALESCE(content, '') FROM messages
                 WHERE session_id = ?1 AND id > ?2 AND role IN ('user', 'assistant')
                 ORDER BY id",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map(params![session_id, through], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(|error| error.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())
    }

    /// The row just written for a session. Used when a message is created and then filled in
    /// (the timeline of an answer, or the images of a question).
    pub fn last_message_id(&self, session_id: &str) -> Result<Option<i64>, String> {
        let conn = self.lock()?;
        conn.query_row(
            "SELECT MAX(id) FROM messages WHERE session_id = ?1",
            params![session_id],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())
    }

    /// How many messages the user has written in total, across sessions. Used as the honest
    /// answer to "do you know this person well enough to take a permanent name".
    pub fn count_user_messages(&self) -> Result<i64, String> {
        let conn = self.lock()?;
        conn.query_row(
            "SELECT COUNT(*) FROM messages WHERE role = 'user'",
            [],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())
    }

    /// The partner's own name. `None` means it has not chosen one yet and is still going by
    /// the placeholder.
    pub fn partner_name(&self) -> Result<Option<String>, String> {
        let conn = self.lock()?;
        let raw = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'partnerName'",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|error| error.to_string())?;
        Ok(raw.and_then(|value| serde_json::from_str(&value).ok()))
    }

    /// Choosing the name is a one-time act — the partner decides it, and nothing changes it
    /// afterwards. That is why this refuses to overwrite an existing name.
    pub fn set_partner_name(&self, name: &str) -> Result<(), String> {
        let name = name.split_whitespace().collect::<Vec<_>>().join(" ");
        if name.chars().count() < 2 || name.chars().count() > 40 {
            return Err("a name has 2 to 40 characters".to_string());
        }
        if name.chars().any(char::is_control) {
            return Err("that is not a name".to_string());
        }
        if self.partner_name()?.is_some() {
            return Err("the name is already chosen and cannot be changed".to_string());
        }

        let value = serde_json::to_string(&name).map_err(|error| error.to_string())?;
        self.set_setting("partnerName", &value)
    }

    /// The mechanical half of curation: what is provably redundant, and what has gone
    /// unused for a long time. Judgement about *meaning* stays with the model or the user —
    /// this only proposes, it never touches a memory on its own. Retired entries are out of
    /// the picture already, so they are not counted here.
    pub fn memory_report(&self, stale_after_days: i64) -> Result<MemoryReport, String> {
        let conn = self.lock()?;
        let mut statement = conn
            .prepare(
                "SELECT id, text, COALESCE(last_used_at, 0), created_at
                 FROM memories WHERE retired_at IS NULL ORDER BY id",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            })
            .map_err(|error| error.to_string())?;

        let cutoff = now() - stale_after_days.max(1) * 86_400;
        let mut seen: HashMap<String, i64> = HashMap::new();
        let mut duplicates = Vec::new();
        let mut stale = Vec::new();
        let mut total = 0i64;

        for row in rows {
            let (id, text, last_used, created_at) = row.map_err(|error| error.to_string())?;
            total += 1;

            let key = text
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .to_lowercase();
            match seen.get(&key) {
                Some(_) => duplicates.push(id),
                None => {
                    seen.insert(key, id);
                }
            }

            if last_used == 0 && created_at < cutoff {
                stale.push(id);
            }
        }

        Ok(MemoryReport {
            total,
            duplicates,
            stale,
        })
    }

    /// The state a file had *before* the partner changed it. Called after a successful
    /// write/edit; a failed tool must not leave a history entry behind.
    pub fn insert_file_version(
        &self,
        path: &str,
        content: Option<&str>,
        existed: bool,
        session_id: &str,
        origin: &str,
        call_id: &str,
        turn_id: Option<&str>,
    ) -> Result<i64, String> {
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO file_versions
               (path, content, existed, session_id, origin, call_id, turn_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                path,
                content,
                existed as i32,
                session_id,
                origin,
                call_id,
                turn_id,
                now()
            ],
        )
        .map_err(|error| error.to_string())?;
        Ok(conn.last_insert_rowid())
    }

    /// Newest first. `limit` is capped by the caller; the content is deliberately not
    /// included here — the list should stay cheap even for a long history.
    pub fn file_versions(&self, path: &str, limit: i64) -> Result<Vec<FileVersionInfo>, String> {
        let conn = self.lock()?;
        let mut statement = conn
            .prepare(
                "SELECT id, path, origin, existed, COALESCE(length(content), 0), created_at
                 FROM file_versions WHERE path = ?1 ORDER BY id DESC LIMIT ?2",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map(params![path, limit.max(1)], |row| {
                Ok(FileVersionInfo {
                    id: row.get(0)?,
                    path: row.get(1)?,
                    origin: row.get(2)?,
                    existed: row.get::<_, i32>(3)? != 0,
                    bytes: row.get(4)?,
                    created_at: row.get(5)?,
                })
            })
            .map_err(|error| error.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())
    }

    pub fn file_version(&self, id: i64) -> Result<FileVersionRow, String> {
        let conn = self.lock()?;
        conn.query_row(
            "SELECT id, path, content, existed, origin, created_at
             FROM file_versions WHERE id = ?1",
            params![id],
            |row| {
                let content: Option<String> = row.get(2)?;
                let info = FileVersionInfo {
                    id: row.get(0)?,
                    path: row.get(1)?,
                    origin: row.get(4)?,
                    existed: row.get::<_, i32>(3)? != 0,
                    bytes: content.as_ref().map(|value| value.len() as i64).unwrap_or(0),
                    created_at: row.get(5)?,
                };
                Ok(FileVersionRow { info, content })
            },
        )
        .map_err(|error| error.to_string())
    }

    /// The most recent turn in this session that changed at least one file. A turn is the
    /// assistant message row the engine used while the tools ran.
    pub fn latest_turn_id(&self, session_id: &str) -> Result<Option<String>, String> {
        let conn = self.lock()?;
        conn.query_row(
            "SELECT turn_id FROM file_versions
             WHERE session_id = ?1 AND turn_id IS NOT NULL
             ORDER BY id DESC LIMIT 1",
            params![session_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| error.to_string())
    }

    /// The earliest snapshot per path in one turn; later writes to the same path within the
    /// turn are covered by the first snapshot and do not appear twice.
    pub fn turn_versions(
        &self,
        session_id: &str,
        turn_id: &str,
    ) -> Result<Vec<TurnVersionRow>, String> {
        let conn = self.lock()?;
        let mut statement = conn
            .prepare(
                "SELECT v.id, v.path, v.content, v.existed, v.origin
                 FROM file_versions v
                 JOIN (
                   SELECT path, MIN(id) AS first_id
                   FROM file_versions
                   WHERE session_id = ?1 AND turn_id = ?2
                   GROUP BY path
                 ) first ON v.id = first.first_id
                 ORDER BY v.path",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map(params![session_id, turn_id], |row| {
                Ok(TurnVersionRow {
                    id: row.get(0)?,
                    path: row.get(1)?,
                    content: row.get(2)?,
                    existed: row.get::<_, i32>(3)? != 0,
                    origin: row.get(4)?,
                })
            })
            .map_err(|error| error.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())
    }

    /// True when the file was changed after this turn (or outside a turn). Undoing an old
    /// turn would otherwise overwrite work that came later.
    pub fn has_later_version(
        &self,
        path: &str,
        id: i64,
        turn_id: &str,
    ) -> Result<bool, String> {
        let conn = self.lock()?;
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM file_versions
                 WHERE path = ?1 AND id > ?2 AND COALESCE(turn_id, '') <> ?3",
                params![path, id, turn_id],
                |row| row.get(0),
            )
            .map_err(|error| error.to_string())?;
        Ok(count > 0)
    }
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn sessions_list(db: tauri::State<'_, Db>) -> Result<Vec<SessionRow>, String> {
    db.list_sessions()
}

#[tauri::command]
pub fn sessions_archived(db: tauri::State<'_, Db>) -> Result<Vec<SessionRow>, String> {
    db.list_archived_sessions()
}

#[tauri::command]
pub fn session_archive(db: tauri::State<'_, Db>, id: String, archived: bool) -> Result<(), String> {
    db.set_session_archived(&id, archived)
}

#[tauri::command]
pub fn session_create(
    db: tauri::State<'_, Db>,
    id: String,
    title: String,
    workspace: String,
) -> Result<(), String> {
    db.create_session(&id, &title, &workspace)
}

#[tauri::command]
pub fn session_delete(db: tauri::State<'_, Db>, id: String) -> Result<(), String> {
    db.delete_session(&id)
}

#[tauri::command]
pub fn session_set_workspace(
    db: tauri::State<'_, Db>,
    id: String,
    workspace: String,
) -> Result<(), String> {
    db.set_session_workspace(&id, &workspace)
}

/// Deletes several sessions in one call. The archive uses it for "delete all"; deleting a
/// single row would otherwise cost one IPC round trip per archived task.
#[tauri::command]
pub fn session_delete_many(db: tauri::State<'_, Db>, ids: Vec<String>) -> Result<usize, String> {
    let mut deleted = 0;
    for id in ids {
        db.delete_session(&id)?;
        deleted += 1;
    }
    Ok(deleted)
}

#[tauri::command]
pub fn session_update(
    db: tauri::State<'_, Db>,
    id: String,
    title: Option<String>,
    group: Option<String>,
) -> Result<(), String> {
    db.update_session(&id, title.as_deref(), group.as_deref())
}

#[tauri::command]
pub fn session_history(
    db: tauri::State<'_, Db>,
    session_id: String,
    before_id: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<HistoryMessage>, String> {
    // The frontend identifies a message as "m{rowid}"; the row id is what the query needs.
    let before = before_id
        .as_deref()
        .and_then(|id| id.strip_prefix('m'))
        .and_then(|id| id.parse::<i64>().ok());
    db.load_history_page(&session_id, before, limit.unwrap_or(80).min(500))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionResume {
    pub summary: String,
    pub through_message_id: i64,
}

/// The compressed summary of a session, for the "where we were" card. `None` until the
/// conversation has been compressed at least once.
#[tauri::command]
pub fn session_resume(
    db: tauri::State<'_, Db>,
    session_id: String,
) -> Result<Option<SessionResume>, String> {
    Ok(db
        .session_summary(&session_id)?
        .map(|(through_message_id, summary)| SessionResume {
            summary,
            through_message_id,
        }))
}

#[tauri::command]
pub fn session_todos(db: tauri::State<'_, Db>, session_id: String) -> Result<Vec<TodoRow>, String> {
    db.list_todos(&session_id)
}

#[tauri::command]
pub fn settings_all(db: tauri::State<'_, Db>) -> Result<HashMap<String, String>, String> {
    db.all_settings()
}

#[tauri::command]
pub fn settings_set(db: tauri::State<'_, Db>, key: String, value: String) -> Result<(), String> {
    db.set_setting(&key, &value)
}

/// The name the partner chose for itself, if it has.
#[tauri::command]
pub fn partner_name(db: tauri::State<'_, Db>) -> Result<Option<String>, String> {
    db.partner_name()
}

#[tauri::command]
pub fn partner_name_set(db: tauri::State<'_, Db>, name: String) -> Result<(), String> {
    db.set_partner_name(&name)
}

/// The OS user's name, used until the user sets a display name of their own.
#[tauri::command]
pub fn default_display_name() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .or_else(|_| std::env::var("LOGNAME"))
        .ok()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| "User".to_string())
}

#[tauri::command]
pub fn memories_list(db: tauri::State<'_, Db>, limit: Option<usize>) -> Result<Vec<MemoryRow>, String> {
    db.list_memories(limit.unwrap_or(200))
}

/// A memory the user writes themselves. `origin` stays `user`, and because nothing was
/// rewritten, `edited_at` stays empty — the recall can tell the two apart.
#[tauri::command]
pub fn memory_add(
    db: tauri::State<'_, Db>,
    text: String,
    title: Option<String>,
    kind: Option<String>,
    scope: Option<String>,
) -> Result<i64, String> {
    let text = text.trim();
    if text.is_empty() {
        return Err("a memory needs text".to_string());
    }
    db.insert_memory_with_title(
        kind.as_deref().unwrap_or("fact"),
        scope.as_deref().unwrap_or("user"),
        title.as_deref(),
        text,
        None,
        "user",
    )
}

/// The memories that were set aside. They are still there — the panel can bring one back.
#[tauri::command]
pub fn memories_retired(db: tauri::State<'_, Db>, limit: Option<usize>) -> Result<Vec<MemoryRow>, String> {
    db.list_retired_memories(limit.unwrap_or(200))
}

#[tauri::command]
pub fn memory_update(
    app: tauri::AppHandle,
    db: tauri::State<'_, Db>,
    id: i64,
    title: Option<String>,
    text: Option<String>,
    kind: Option<String>,
    importance: Option<i64>,
) -> Result<(), String> {
    db.update_memory(
        id,
        title.as_deref(),
        text.as_deref(),
        kind.as_deref(),
        importance,
        "user",
    )?;
    // The user taking the time to correct a note is the clearest sign the work matters.
    crate::engine::award_trust(&app, "correction", 3, 3, Some("the user edited a memory"));
    Ok(())
}

#[tauri::command]
pub fn memory_retire(db: tauri::State<'_, Db>, id: i64) -> Result<(), String> {
    db.retire_memory(id)
}

#[tauri::command]
pub fn memory_restore(db: tauri::State<'_, Db>, id: i64) -> Result<(), String> {
    db.restore_memory(id)
}

#[tauri::command]
pub fn memory_report(db: tauri::State<'_, Db>) -> Result<MemoryReport, String> {
    db.memory_report(30)
}

/// The curation pass proposes; this sets the proposed entries aside. Nothing is erased.
#[tauri::command]
pub fn memory_prune(db: tauri::State<'_, Db>, ids: Vec<i64>) -> Result<usize, String> {
    db.retire_memories(&ids)
}

#[tauri::command]
pub fn search(
    db: tauri::State<'_, Db>,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<SearchHit>, String> {
    db.search(&query, limit.unwrap_or(40))
}

#[tauri::command]
pub fn schedules_list(db: tauri::State<'_, Db>) -> Result<Vec<ScheduleRow>, String> {
    db.list_schedules()
}

#[tauri::command]
pub fn schedule_create(
    db: tauri::State<'_, Db>,
    session_id: String,
    prompt: String,
    interval_seconds: i64,
) -> Result<i64, String> {
    db.create_schedule(&session_id, &prompt, interval_seconds)
}

#[tauri::command]
pub fn schedule_delete(db: tauri::State<'_, Db>, id: i64) -> Result<(), String> {
    db.delete_schedule(id)
}

#[tauri::command]
pub fn schedule_toggle(db: tauri::State<'_, Db>, id: i64, enabled: bool) -> Result<(), String> {
    db.set_schedule_enabled(id, enabled)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A note remembers how it is known. Default is `assumed` — a guess must not become a
    /// remembered fact — and only an explicit `verified` changes that. A user-edited note is
    /// theirs by `origin`, which the recall trusts regardless.
    #[test]
    fn a_memory_records_whether_it_was_verified() {
        let directory = std::env::temp_dir().join(format!("ds-ide-basis-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();
        let db = Db::open(&directory.join("basis.sqlite3")).unwrap();

        let assumed = db
            .insert_memory("fact", "user", "Wohnt vielleicht in Köln", None, "seeky")
            .unwrap();
        let verified = db
            .insert_memory("fact", "user", "Wohnt in Köln (aus der Adresse gelesen)", None, "seeky")
            .unwrap();
        db.set_memory_basis(verified, "verified").unwrap();
        // An unknown value falls back to the honest default instead of inventing a third state.
        db.set_memory_basis(verified, "probably").unwrap();

        let rows = db.list_memories(10).unwrap();
        assert_eq!(rows.iter().find(|row| row.id == assumed).unwrap().basis, "assumed");
        // `probably` was not a known value, so it stays assumed rather than being trusted.
        assert_eq!(rows.iter().find(|row| row.id == verified).unwrap().basis, "assumed");

        db.set_memory_basis(verified, "verified").unwrap();
        let stored = db.memory_context(10, "").unwrap();
        assert_eq!(stored.iter().find(|row| row.id == verified).unwrap().basis, "verified");
    }

    /// Trust is earned in small, capped steps: repeats the same day earn nothing, and the
    /// reveal flag starts unset for a fresh database.
    #[test]
    fn trust_is_capped_per_day_and_survives() {
        let directory = std::env::temp_dir().join(format!("ds-ide-trust-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();
        let db = Db::open(&directory.join("trust.sqlite3")).unwrap();

        assert_eq!(db.trust_total().unwrap(), 0);
        assert!(!db.trust_revealed().unwrap());

        assert!(db.award_trust("day", 2, 2, None).unwrap());
        assert!(!db.award_trust("day", 2, 2, None).unwrap(), "once per day");
        for _ in 0..12 {
            db.award_trust("turn", 1, 10, None).unwrap();
        }
        for _ in 0..8 {
            db.award_trust("memory", 1, 5, None).unwrap();
        }
        assert_eq!(db.trust_total().unwrap(), 2 + 10 + 5);

        let breakdown = db.trust_breakdown().unwrap();
        assert_eq!(breakdown[0], ("turn".to_string(), 10));
        assert_eq!(breakdown.len(), 3);

        db.reveal_trust().unwrap();
        assert!(db.trust_revealed().unwrap());
    }

    /// A database that already has a chosen name keeps showing it after the upgrade.
    #[test]
    fn an_existing_name_is_revealed_after_the_migration() {
        let directory = std::env::temp_dir().join(format!("ds-ide-trust-mig-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("old.sqlite3");
        {
            let db = Db::open(&path).unwrap();
            db.set_setting("partnerName", "\"Ziggy\"").unwrap();
        }
        // Reopening runs the migration on an installation that predates the threshold.
        let db = Db::open(&path).unwrap();
        assert!(db.trust_revealed().unwrap());
        assert_eq!(db.trust_total().unwrap(), 0);
    }

    /// A long thread is read newest-first in pages; the second page continues from the first
    /// message of the first without repeating or skipping one.
    #[test]
    fn history_pages_continue_where_the_previous_one_stopped() {
        let directory = std::env::temp_dir().join(format!("ds-ide-pages-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();
        let db = Db::open(&directory.join("pages.sqlite3")).unwrap();
        db.ensure_session("s1", "Paging", "project", "/tmp").unwrap();
        for index in 0..5 {
            db.insert_message("s1", if index % 2 == 0 { "user" } else { "assistant" },
                Some(&format!("m{index}")), None, None)
                .unwrap();
        }

        let newest = db.load_history_page("s1", None, 2).unwrap();
        assert_eq!(newest.len(), 2);
        assert_eq!(newest[1].text, "m4");

        let older = db
            .load_history_page("s1", Some(newest[0].id.trim_start_matches('m').parse().unwrap()), 2)
            .unwrap();
        assert_eq!(older.len(), 2);
        assert_eq!(older[0].text, "m1");
        assert_eq!(older[1].text, "m2");

        let rest = db
            .load_history_page("s1", Some(older[0].id.trim_start_matches('m').parse().unwrap()), 2)
            .unwrap();
        assert_eq!(rest.len(), 1);
        assert_eq!(rest[0].text, "m0");

        // The unpaged read is still the whole conversation, oldest first.
        let all = db.load_history("s1").unwrap();
        assert_eq!(all.len(), 5);
        assert_eq!(all[0].text, "m0");
    }

    /// A self-reminder fires once and switches itself off, a repeating one keeps its next run,
    /// and the active count is what the cap is enforced against.
    #[test]
    fn a_reminder_fires_once_and_a_schedule_repeats() {
        let directory = std::env::temp_dir().join(format!("ds-ide-remind-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();
        let db = Db::open(&directory.join("remind.sqlite3")).unwrap();
        db.ensure_session("s1", "Reminders", "project", "/tmp").unwrap();

        let now = unix_now();
        let once = db
            .create_reminder("s1", "check the CI", 60, None, "companion")
            .unwrap();
        let repeating = db
            .create_reminder("s1", "look at the log", 60, Some(3600), "companion")
            .unwrap();

        let rows = db.list_schedules().unwrap();
        let one = rows.iter().find(|row| row.id == once).unwrap();
        assert!(one.one_shot, "no repeat means one-shot");
        assert_eq!(one.created_by, "companion");
        assert!(one.next_run_at >= now + 60);

        let repeat = rows.iter().find(|row| row.id == repeating).unwrap();
        assert!(!repeat.one_shot);
        assert_eq!(repeat.interval_seconds, 3600);

        assert_eq!(db.companion_reminder_count("s1").unwrap(), 2);
        let due = db.due_schedules(now + 61).unwrap();
        assert_eq!(due.len(), 2, "both are due a minute later");

        // The runner disables a one-shot instead of rescheduling it.
        db.disable_schedule(once).unwrap();
        db.stamp_schedule_run(once).unwrap();
        assert_eq!(db.companion_reminder_count("s1").unwrap(), 1);
        let ran = db
            .list_schedules()
            .unwrap()
            .into_iter()
            .find(|row| row.id == once)
            .unwrap();
        assert!(ran.last_run_at.is_some(), "a fired reminder remembers when");
        let enabled: Vec<i64> = db
            .list_schedules()
            .unwrap()
            .into_iter()
            .filter(|row| row.enabled)
            .map(|row| row.id)
            .collect();
        assert_eq!(enabled, vec![repeating]);
    }

    #[test]
    fn round_trips_history_and_todos() {
        let directory = std::env::temp_dir().join(format!("ds-ide-db-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("test.sqlite3");

        let db = Db::open(&path).unwrap();
        db.ensure_session("s1", "Test", "project", "/tmp").unwrap();
        db.insert_message("s1", "user", Some("hallo"), None, None)
            .unwrap();
        let assistant_id = db
            .insert_message("s1", "assistant", Some("hi"), Some("[]"), None)
            .unwrap();
        db.insert_tool_call(
            "s1",
            Some(assistant_id),
            "call_1",
            "glob",
            Some("{\"pattern\":\"*\"}"),
            Some("ok"),
            false,
            Some(3),
        )
        .unwrap();
        db.replace_todos(
            "s1",
            &[serde_json::json!({ "content": "A", "status": "pending" })],
        )
        .unwrap();

        let history = db.load_history("s1").unwrap();
        assert_eq!(history.len(), 2, "user + assistant");
        assert_eq!(history[0].role, "user");
        assert_eq!(history[1].tools.len(), 1, "tool call attached to assistant");
        assert_eq!(history[1].tools[0].name, "glob");
        assert_eq!(db.list_todos("s1").unwrap().len(), 1);
        assert_eq!(db.list_todos("s1").unwrap()[0].content, "A");

        let llm = db.load_llm_messages("s1").unwrap();
        assert_eq!(llm.len(), 2);
        assert_eq!(llm[1].role, "assistant");

        let sessions = db.list_sessions().unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id, "s1");

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn file_versions_round_trip() {
        let directory =
            std::env::temp_dir().join(format!("ds-ide-versions-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();
        let db = Db::open(&directory.join("test.sqlite3")).unwrap();

        let older = db
            .insert_file_version("/tmp/a.txt", Some("alt"), true, "s1", "edit", "c1", None)
            .unwrap();
        let newer = db
            .insert_file_version("/tmp/a.txt", None, false, "s1", "write", "c2", None)
            .unwrap();

        let list = db.file_versions("/tmp/a.txt", 10).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].id, newer, "newest first");
        assert_eq!(list[0].bytes, 0);
        assert!(!list[0].existed);
        assert_eq!(list[1].id, older);
        assert!(list[1].existed);

        let row = db.file_version(older).unwrap();
        assert_eq!(row.info.path, "/tmp/a.txt");
        assert_eq!(row.content.as_deref(), Some("alt"));
        assert_eq!(row.info.origin, "edit");

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn turn_versions_take_the_first_snapshot_per_path() {
        let directory =
            std::env::temp_dir().join(format!("ds-ide-turn-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();
        let db = Db::open(&directory.join("test.sqlite3")).unwrap();

        let first = db
            .insert_file_version(
                "/tmp/a.txt",
                Some("alt"),
                true,
                "s1",
                "edit",
                "c1",
                Some("m1"),
            )
            .unwrap();
        db.insert_file_version(
            "/tmp/a.txt",
            Some("mittel"),
            true,
            "s1",
            "edit",
            "c2",
            Some("m1"),
        )
        .unwrap();
        db.insert_file_version("/tmp/b.txt", None, false, "s1", "write", "c3", Some("m1"))
            .unwrap();

        assert_eq!(db.latest_turn_id("s1").unwrap().as_deref(), Some("m1"));
        let versions = db.turn_versions("s1", "m1").unwrap();
        assert_eq!(versions.len(), 2, "one row per path");
        let a = versions.iter().find(|row| row.path == "/tmp/a.txt").unwrap();
        assert_eq!(a.id, first);
        assert_eq!(a.content.as_deref(), Some("alt"));
        assert!(!db.has_later_version("/tmp/a.txt", first, "m1").unwrap());

        db.insert_file_version(
            "/tmp/a.txt",
            Some("neu"),
            true,
            "s1",
            "edit",
            "c4",
            Some("m2"),
        )
        .unwrap();
        assert!(db.has_later_version("/tmp/a.txt", first, "m1").unwrap());
        assert_eq!(db.latest_turn_id("s1").unwrap().as_deref(), Some("m2"));

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn rollback_removes_the_failed_tail_but_keeps_the_user_message() {
        let directory =
            std::env::temp_dir().join(format!("ds-ide-rollback-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();
        let db = Db::open(&directory.join("test.sqlite3")).unwrap();
        db.ensure_session("s1", "Test", "project", "/tmp").unwrap();

        db.insert_message("s1", "user", Some("hallo"), None, None)
            .unwrap();
        let assistant = db
            .insert_message("s1", "assistant", Some(""), Some("[]"), None)
            .unwrap();
        db.insert_tool_call(
            "s1",
            Some(assistant),
            "call_1",
            "write",
            Some("{}"),
            Some("ok"),
            false,
            Some(1),
        )
        .unwrap();
        db.insert_message("s1", "tool", Some("ok"), None, Some("call_1"))
            .unwrap();

        db.rollback_failed_turn("s1").unwrap();
        let history = db.load_history("s1").unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].role, "user");
        assert!(history[0].tools.is_empty());

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn memories_round_trip() {
        let directory = std::env::temp_dir().join(format!("ds-ide-mem-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("m.sqlite3");

        let db = Db::open(&path).unwrap();
        let id = db
            .insert_memory("preference", "user", "Mag kurze Antworten", None, "seeky")
            .unwrap();
        assert!(id > 0);

        let list = db.list_memories(10).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].scope, "user");
        assert_eq!(list[0].kind, "preference");
        assert_eq!(list[0].origin, "seeky");
        assert!(list[0].edited_at.is_none());

        let context = db.memory_context(10, "").unwrap();
        assert_eq!(context.len(), 1);
        db.touch_memories(&[id]).unwrap();

        db.retire_memory(id).unwrap();
        assert!(db.list_memories(10).unwrap().is_empty());
        assert!(db.memory_context(10, "").unwrap().is_empty());

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn recall_finds_memories_and_past_messages() {
        let directory = std::env::temp_dir().join(format!("ds-ide-recall-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();
        let db = Db::open(&directory.join("r.sqlite3")).unwrap();

        db.create_session("s1", "Farben", "/tmp/project").unwrap();
        db.insert_message("s1", "user", Some("Die Testfarbe ist Blau."), None, None)
            .unwrap();
        db.create_session("s2", "Anderes Projekt", "/tmp/other").unwrap();
        db.insert_message(
            "s2",
            "user",
            Some("Blau gehoert zum anderen Projekt."),
            None,
            None,
        )
        .unwrap();
        let id = db
            .insert_memory_with_title(
                "fact",
                "user",
                Some("Testfarbe"),
                "Die Testfarbe ist Blau.",
                None,
                "seeky",
            )
            .unwrap();
        db.set_memory_workspace(id, "/tmp/project").unwrap();

        let hits = db.recall("Blau", 10, "/tmp/project").unwrap();
        assert!(
            hits.iter()
                .any(|hit| hit.kind == "memory" && hit.text.contains("Blau")),
            "the project note is recalled: {hits:?}"
        );
        assert!(
            hits.iter().any(|hit| {
                hit.kind == "message" && hit.session_id == "s1" && hit.workspace == "/tmp/project"
            }),
            "the earlier message is recalled with its workspace: {hits:?}"
        );
        assert!(
            !hits.iter().any(|hit| hit.session_id == "s2"),
            "the other project's conversation must stay out: {hits:?}"
        );

        let elsewhere = db.recall("Blau", 10, "/tmp/other").unwrap();
        assert!(
            !elsewhere
                .iter()
                .any(|hit| hit.kind == "memory" && hit.workspace == "/tmp/project"),
            "another project must not see this note"
        );
        assert!(
            elsewhere.iter().any(|hit| {
                hit.kind == "message" && hit.session_id == "s2" && hit.workspace == "/tmp/other"
            }),
            "the other project's own message is recalled there: {elsewhere:?}"
        );
        assert!(
            !elsewhere.iter().any(|hit| hit.session_id == "s1"),
            "the first project's conversation must stay out: {elsewhere:?}"
        );

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn memories_are_scoped_to_the_workspace() {
        let directory =
            std::env::temp_dir().join(format!("ds-ide-mem-scope-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();
        let db = Db::open(&directory.join("test.sqlite3")).unwrap();

        db.insert_memory("fact", "user", "global note", None, "seeky")
            .unwrap();
        let project = db
            .insert_memory("fact", "user", "project note", None, "seeky")
            .unwrap();
        db.set_memory_workspace(project, "/work/project").unwrap();

        let here = db.memory_context(10, "/work/project").unwrap();
        assert_eq!(here.len(), 2);
        assert!(here.iter().any(|row| row.text == "project note"));

        let elsewhere = db.memory_context(10, "/elsewhere").unwrap();
        assert_eq!(elsewhere.len(), 1);
        assert_eq!(elsewhere[0].text, "global note");

        let _ = std::fs::remove_dir_all(&directory);
    }

    /// Retiring is not deleting: the entry keeps its text and can come back.
    #[test]
    fn retiring_hides_a_memory_and_restoring_brings_it_back() {
        let directory = std::env::temp_dir().join(format!("ds-ide-mem-retire-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("m.sqlite3");

        let db = Db::open(&path).unwrap();
        let id = db
            .insert_memory("fact", "user", "Wohnt in Köln", None, "seeky")
            .unwrap();

        db.retire_memory(id).unwrap();
        assert!(db.list_memories(10).unwrap().is_empty());

        let retired = db.list_retired_memories(10).unwrap();
        assert_eq!(retired.len(), 1);
        assert_eq!(retired[0].text, "Wohnt in Köln");
        assert!(retired[0].retired_at.is_some());
        assert_eq!(db.memory_report(30).unwrap().total, 0, "retired is not counted");

        db.restore_memory(id).unwrap();
        assert_eq!(db.list_memories(10).unwrap().len(), 1);
        assert!(db.list_retired_memories(10).unwrap().is_empty());

        let _ = std::fs::remove_dir_all(&directory);
    }

    /// Who owns a memory is the point of the marker: the user's rewrite is marked and stays
    /// marked, until Seeky rewrites the entry itself.
    #[test]
    fn a_user_edit_marks_the_memory_as_theirs() {
        let directory = std::env::temp_dir().join(format!("ds-ide-mem-origin-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("m.sqlite3");

        let db = Db::open(&path).unwrap();
        let id = db
            .insert_memory("preference", "user", "Mag lange Antworten", None, "seeky")
            .unwrap();

        db.update_memory(id, None, Some("Mag kurze, direkte Antworten"), None, None, "user")
            .unwrap();
        let row = &db.list_memories(10).unwrap()[0];
        assert_eq!(row.text, "Mag kurze, direkte Antworten");
        assert_eq!(row.origin, "user");
        assert!(row.edited_at.is_some());
        assert_eq!(row.kind, "preference", "untouched fields survive the update");

        db.update_memory(id, None, Some("Notiz von Seeky"), None, None, "seeky")
            .unwrap();
        let row = &db.list_memories(10).unwrap()[0];
        assert_eq!(row.origin, "seeky");
        assert!(row.edited_at.is_none(), "the user marker goes with the text");

        assert!(db
            .update_memory(999, None, Some("x"), None, None, "user")
            .is_err());
        assert!(db.retire_memory(999).is_err());

        let _ = std::fs::remove_dir_all(&directory);
    }

    /// The companion columns have to appear in a database that predates them.
    #[test]
    fn migrate_adds_the_memory_columns() {
        let directory =
            std::env::temp_dir().join(format!("ds-ide-mem-migrate-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("old.sqlite3");

        {
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch(
                "CREATE TABLE memories (
                   id           INTEGER PRIMARY KEY AUTOINCREMENT,
                   kind         TEXT NOT NULL,
                   scope        TEXT NOT NULL DEFAULT 'user',
                   text         TEXT NOT NULL,
                   source       TEXT,
                   importance   INTEGER NOT NULL DEFAULT 1,
                   created_at   INTEGER NOT NULL,
                   last_used_at INTEGER
                 );
                 INSERT INTO memories (kind, scope, text, created_at)
                 VALUES ('fact', 'user', 'Altes Wissen', 1);",
            )
            .unwrap();
        }

        let db = Db::open(&path).unwrap();
        let rows = db.list_memories(10).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].text, "Altes Wissen");
        assert_eq!(rows[0].origin, "seeky", "old rows belong to Seeky");
        assert!(rows[0].title.is_none(), "old rows have no title yet");
        assert!(rows[0].edited_at.is_none());
        assert!(rows[0].retired_at.is_none());
        assert_eq!(
            rows[0].basis, "assumed",
            "a note written before the field existed was reasoned, not checked"
        );

        // New rows carry a title; without one, the first line becomes the heading.
        let id = db
            .insert_memory_with_title(
                "fact",
                "user",
                Some("  Ein Titel  "),
                "Ein Text",
                None,
                "seeky",
            )
            .unwrap();
        let titled = db
            .list_memories(10)
            .unwrap()
            .into_iter()
            .find(|row| row.id == id)
            .unwrap();
        assert_eq!(titled.title.as_deref(), Some("Ein Titel"));

        let derived = db
            .insert_memory("fact", "user", "Erste Zeile\nzweite Zeile", None, "seeky")
            .unwrap();
        let derived = db
            .list_memories(10)
            .unwrap()
            .into_iter()
            .find(|row| row.id == derived)
            .unwrap();
        assert_eq!(derived.title.as_deref(), Some("Erste Zeile"));

        let _ = std::fs::remove_dir_all(&directory);
    }

    /// Compression: the summary stands in for the messages it covers — in the request to the
    /// model, while the transcript itself stays complete.
    #[test]
    fn a_summary_replaces_the_messages_it_covers() {
        let directory = std::env::temp_dir().join(format!("ds-ide-compact-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();
        let db = Db::open(&directory.join("c.sqlite3")).unwrap();

        db.create_session("s1", "Test", "/tmp").unwrap();
        for text in ["erster", "zweiter", "dritter", "vierter"] {
            db.insert_message("s1", "user", Some(text), None, None).unwrap();
        }

        let all = db.compactable_messages("s1").unwrap();
        assert_eq!(all.len(), 4);
        let through = all[1].0;
        db.set_session_summary("s1", through, "Zusammenfassung der ersten beiden").unwrap();

        let sent = db.load_llm_messages("s1").unwrap();
        assert_eq!(sent.len(), 3, "summary plus the two messages it does not cover");
        assert!(sent[0]
            .content
            .as_deref()
            .unwrap_or_default()
            .contains("Zusammenfassung der ersten beiden"));
        assert_eq!(sent[1].content.as_deref(), Some("dritter"));
        assert_eq!(sent[2].content.as_deref(), Some("vierter"));

        assert_eq!(
            db.compactable_messages("s1").unwrap().len(),
            2,
            "only what the summary does not cover is folded again"
        );
        assert_eq!(
            db.load_history("s1").unwrap().len(),
            4,
            "the transcript stays complete"
        );

        let _ = std::fs::remove_dir_all(&directory);
    }

    /// One turn is one message: the row is created first and the timeline is filled in as the
    /// answer grows. Reading it back has to give the same order it was written in.
    #[test]
    fn a_turn_is_stored_with_its_timeline() {
        let directory = std::env::temp_dir().join(format!("ds-ide-parts-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();
        let db = Db::open(&directory.join("p.sqlite3")).unwrap();

        db.create_session("s1", "Test", "/tmp").unwrap();
        db.insert_message("s1", "user", Some("Was liegt hier?"), None, None)
            .unwrap();

        let id = db
            .insert_message("s1", "assistant", None, None, None)
            .unwrap();
        let timeline = r#"[{"kind":"tool","tool":{"id":"c1","name":"bash","output":"a.txt"}},
                          {"kind":"text","text":"Eine Datei."}]"#;
        db.update_message(id, Some("Eine Datei."), None, Some(timeline))
            .unwrap();

        let history = db.load_history("s1").unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].role, "user");
        assert!(history[0].parts.is_none(), "a user message has no timeline");

        let answer = &history[1];
        assert_eq!(answer.text, "Eine Datei.");
        let parts = answer.parts.as_deref().expect("timeline is stored");
        assert!(parts.starts_with(r#"[{"kind":"tool""#), "order is kept: {parts}");
        assert!(parts.contains(r#""kind":"text""#));

        // A message that predates the timeline still loads, just without parts.
        db.insert_message("s1", "assistant", Some("Alt"), None, None)
            .unwrap();
        let history = db.load_history("s1").unwrap();
        assert_eq!(history.len(), 3);
        assert_eq!(history[2].text, "Alt");
        assert!(history[2].parts.is_none());

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn search_finds_sessions_and_messages() {
        let directory =
            std::env::temp_dir().join(format!("ds-ide-search-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();
        let db = Db::open(&directory.join("s.sqlite3")).unwrap();

        db.ensure_session("s1", "Monaco Editor", "project", "/tmp")
            .unwrap();
        db.insert_message("s1", "user", Some("Wie baue ich Monaco ein?"), None, None)
            .unwrap();
        db.insert_message(
            "s1",
            "assistant",
            Some("Tauri und Vite moegen Worker sehr."),
            None,
            None,
        )
        .unwrap();

        let by_title = db.search("Monaco", 20).unwrap();
        assert!(
            by_title.iter().any(|hit| hit.kind == "session"),
            "title hit expected"
        );

        let by_body = db.search("Worker", 20).unwrap();
        assert!(
            by_body
                .iter()
                .any(|hit| hit.kind == "message" && hit.snippet.contains("Worker")),
            "message hit with snippet expected"
        );

        assert!(db.search("   ", 20).unwrap().is_empty(), "blank query");

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn schedules_cycle() {
        let directory =
            std::env::temp_dir().join(format!("ds-ide-sched-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();
        let db = Db::open(&directory.join("c.sqlite3")).unwrap();

        db.ensure_session("s1", "Test", "project", "/tmp").unwrap();
        let id = db.create_schedule("s1", "Status?", 60).unwrap();

        let first = db.list_schedules().unwrap();
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].interval_seconds, 60);

        // Not yet due …
        assert!(db
            .due_schedules(first[0].next_run_at - 1)
            .unwrap()
            .is_empty());
        // … but due later.
        let due = db.due_schedules(unix_now() + 3600).unwrap();
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].prompt, "Status?");

        db.mark_schedule_run(id, unix_now() + 99_999).unwrap();
        assert!(db.due_schedules(unix_now() + 3600).unwrap().is_empty());

        db.set_schedule_enabled(id, false).unwrap();
        assert!(!db.list_schedules().unwrap()[0].enabled);

        db.delete_schedule(id).unwrap();
        assert!(db.list_schedules().unwrap().is_empty());

        let _ = std::fs::remove_dir_all(&directory);
    }

    /// The name is the partner's own, it is chosen once, and the database is what enforces
    /// that — not the UI that happens not to offer an edit field.
    #[test]
    fn the_partner_name_is_chosen_once() {
        let directory = std::env::temp_dir().join(format!("ds-ide-name-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();
        let db = Db::open(&directory.join("n.sqlite3")).unwrap();

        assert!(db.partner_name().unwrap().is_none(), "unnamed at first");
        assert!(db.set_partner_name("x").is_err(), "one letter is not a name");
        assert!(db.partner_name().unwrap().is_none(), "a refused name is not stored");

        db.set_partner_name("  Njord  ").unwrap();
        assert_eq!(db.partner_name().unwrap().as_deref(), Some("Njord"));

        // The note that records the choice carries a plain calendar date.
        let day = db.today().unwrap();
        assert_eq!(day.len(), 10, "expected YYYY-MM-DD, got {day}");

        assert!(
            db.set_partner_name("Anders").is_err(),
            "a second choice is refused"
        );
        assert_eq!(db.partner_name().unwrap().as_deref(), Some("Njord"));

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn memory_report_flags_exact_duplicates() {
        let directory =
            std::env::temp_dir().join(format!("ds-ide-doctor-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();
        let db = Db::open(&directory.join("d.sqlite3")).unwrap();

        db.insert_memory("preference", "user", "Mag kurze Antworten", None, "seeky")
            .unwrap();
        db.insert_memory("preference", "user", "mag  kurze   antworten", None, "seeky")
            .unwrap();
        db.insert_memory("fact", "user", "Etwas anderes", None, "seeky")
            .unwrap();

        let report = db.memory_report(30).unwrap();
        assert_eq!(report.total, 3);
        assert_eq!(report.duplicates.len(), 1, "one repeat of the first entry");
        assert!(report.stale.is_empty(), "fresh entries are not stale");

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn sessions_can_be_renamed_and_pinned() {
        let directory =
            std::env::temp_dir().join(format!("ds-ide-session-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();
        let db = Db::open(&directory.join("s.sqlite3")).unwrap();

        db.ensure_session("s1", "", "project", "/tmp").unwrap();
        assert_eq!(db.session_title("s1").unwrap().as_deref(), Some(""));

        db.update_session("s1", Some("Monaco-Anbindung"), None)
            .unwrap();
        assert_eq!(
            db.session_title("s1").unwrap().as_deref(),
            Some("Monaco-Anbindung")
        );

        db.update_session("s1", None, Some("pinned")).unwrap();
        assert_eq!(db.list_sessions().unwrap()[0].group, "pinned");

        let _ = std::fs::remove_dir_all(&directory);
    }

    /// A thread written before projects existed has an empty workspace; assigning one later
    /// must work without touching the conversation, and must survive a reload.
    #[test]
    fn a_thread_can_be_moved_into_a_project() {
        let directory =
            std::env::temp_dir().join(format!("ds-ide-project-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();
        let db = Db::open(&directory.join("s.sqlite3")).unwrap();

        db.create_session("s1", "alt", "").unwrap();
        assert_eq!(db.list_sessions().unwrap()[0].workspace, "");

        db.set_session_workspace("s1", "/p/project").unwrap();
        let rows = db.list_sessions().unwrap();
        assert_eq!(rows[0].workspace, "/p/project");
        // Moving it does not rename it.
        assert_eq!(rows[0].title, "alt");

        // An unknown id is a no-op, not an error: the row may have been archived meanwhile.
        db.set_session_workspace("gone", "/p/x").unwrap();

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn archiving_hides_a_session_without_deleting_it() {
        let directory =
            std::env::temp_dir().join(format!("ds-ide-archive-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();
        let db = Db::open(&directory.join("a.sqlite3")).unwrap();

        db.ensure_session("s1", "Bleibt erhalten", "project", "/tmp")
            .unwrap();
        db.insert_message("s1", "user", Some("hallo"), None, None)
            .unwrap();

        db.set_session_archived("s1", true).unwrap();
        assert!(db.list_sessions().unwrap().is_empty(), "gone from the sidebar");
        let archived = db.list_archived_sessions().unwrap();
        assert_eq!(archived.len(), 1, "still in the archive");
        assert!(archived[0].archived);
        assert_eq!(
            db.load_history("s1").unwrap().len(),
            1,
            "the conversation is untouched"
        );

        db.set_session_archived("s1", false).unwrap();
        assert_eq!(db.list_sessions().unwrap().len(), 1, "reactivated");

        let _ = std::fs::remove_dir_all(&directory);
    }

    /// The existing installation has a `sessions` table without `archived`.
    #[test]
    fn migrate_adds_archived_to_an_old_database() {
        let directory =
            std::env::temp_dir().join(format!("ds-ide-migrate-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("old.sqlite3");

        {
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch(
                "CREATE TABLE sessions (
                   id         TEXT PRIMARY KEY,
                   title      TEXT NOT NULL DEFAULT '',
                   grp        TEXT NOT NULL DEFAULT 'project',
                   workspace  TEXT NOT NULL DEFAULT '',
                   created_at INTEGER NOT NULL,
                   updated_at INTEGER NOT NULL
                 );
                 INSERT INTO sessions (id, title, grp, workspace, created_at, updated_at)
                 VALUES ('s1', 'Alte Sitzung', 'project', '/tmp', 1, 1);",
            )
            .unwrap();
        }

        let db = Db::open(&path).unwrap();
        let sessions = db.list_sessions().unwrap();
        assert_eq!(sessions.len(), 1, "old row survived the migration");
        assert!(!sessions[0].archived);

        db.set_session_archived("s1", true).unwrap();
        assert!(db.list_sessions().unwrap().is_empty());
        assert_eq!(db.list_archived_sessions().unwrap().len(), 1);

        let _ = std::fs::remove_dir_all(&directory);
    }
}
