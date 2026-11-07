use std::fs;
use std::path::Path;

use serde::Serialize;

/// A Git panel without a Git library: the CLI is already a dependency of the code-review
/// tool and behaves the same on Windows, macOS and Linux. Every call uses `--` before paths
/// and never goes through a shell.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitEntry {
    pub path: String,
    pub code: String,
    pub staged: bool,
    pub untracked: bool,
    pub conflicted: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitStatus {
    pub branch: String,
    pub head: String,
    pub entries: Vec<GitEntry>,
    /// How many entries git reported before the cap.
    pub total: usize,
    /// True when `entries` was cut to `MAX_ENTRIES`.
    pub truncated: bool,
}

/// No git call may hold the app hostage. A slow or wedged repository — a network mount, a huge
/// pack, a lock — has to end in a message, not in a spinning fan.
const GIT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(25);

/// Runs git asynchronously, off the worker that answers the webview. `kill_on_drop` plus the
/// timeout means a hung command is terminated rather than left behind. Returns the raw output,
/// including a non-zero exit, so callers can treat a plain "no" differently from a failure.
async fn run_git_output(
    workspace: &str,
    args: &[&str],
) -> Result<std::process::Output, String> {
    if !Path::new(workspace).is_dir() {
        return Err(format!("Arbeitsverzeichnis nicht gefunden: {workspace}"));
    }

    let mut command = tokio::process::Command::new("git");
    command
        .current_dir(workspace)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_PAGER", "cat")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);
    // A bundled library path makes the system git load libraries that do not match its own
    // libcurl — `git push` over HTTPS then dies on a missing symbol. See `child_env`.
    crate::child_env::sanitize_async(&mut command);

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // No console flash when a GUI app shells out to git.
        command.creation_flags(0x0800_0000);
    }

    match tokio::time::timeout(GIT_TIMEOUT, command.output()).await {
        Ok(Ok(output)) => Ok(output),
        Ok(Err(error)) => Err(format!("git konnte nicht gestartet werden: {error}")),
        Err(_) => Err(format!(
            "git hat nach {} s nicht geantwortet — abgebrochen. Ein großes Repository, ein \
             Netzwerkpfad oder eine Sperre kann die Ursache sein.",
            GIT_TIMEOUT.as_secs()
        )),
    }
}

async fn run_git(workspace: &str, args: &[&str]) -> Result<Vec<u8>, String> {
    let output = run_git_output(workspace, args).await?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        return Err(if stderr.is_empty() { stdout } else { stderr });
    }
    Ok(output.stdout)
}

async fn git_text(workspace: &str, args: &[&str]) -> Option<String> {
    run_git(workspace, args)
        .await
        .ok()
        .map(|bytes| String::from_utf8_lossy(&bytes).trim().to_string())
}

/// Parses `git status --porcelain=v1 -z`. A file with both staged and unstaged edits yields
/// two entries, so the panel can show both sides and diff exactly the side that was clicked.
fn parse_status(bytes: &[u8]) -> Vec<GitEntry> {
    let mut entries = Vec::new();
    let mut parts = bytes.split(|byte| *byte == 0);

    while let Some(raw) = parts.next() {
        if raw.is_empty() || raw.len() < 4 {
            continue;
        }
        let xy = &raw[..2];
        let path = String::from_utf8_lossy(&raw[3..]).to_string();
        let x = xy[0] as char;
        let y = xy[1] as char;
        let untracked = xy == b"??";
        let conflicted = x == 'U' || y == 'U' || xy == b"AA" || xy == b"DD";
        let staged = x != ' ' && x != '?';
        let unstaged = y != ' ' && y != '?';

        if staged {
            entries.push(GitEntry {
                path: path.clone(),
                code: x.to_string(),
                staged: true,
                untracked: false,
                conflicted,
            });
        }
        if unstaged {
            entries.push(GitEntry {
                path: path.clone(),
                code: y.to_string(),
                staged: false,
                untracked: false,
                conflicted,
            });
        }
        if untracked {
            entries.push(GitEntry {
                path: path.clone(),
                code: "??".to_string(),
                staged: false,
                untracked: true,
                conflicted: false,
            });
        }

        // Rename/copy entries carry the original path in the following NUL segment.
        if x == 'R' || x == 'C' || y == 'R' || y == 'C' {
            let _ = parts.next();
        }
    }

    entries
}

/// A working tree with more entries than this is not something a panel can show, and listing
/// them costs minutes on a real disk. The count is still reported so nothing is hidden.
const MAX_ENTRIES: usize = 2000;

/// The one answer the panel gives when this project has no repository of its own — whether git
/// found none at all or only a parent's. Which parent it was belongs in a bug report, not in a
/// panel; the front end turns this marker into a translated sentence.
const NO_REPOSITORY: &str = "no_git_repository";

/// Where git thinks the repository is, following parents. This is the check that keeps the
/// panel honest: a project directory without its own `.git` is often inside a larger repository
/// (a folder of projects, a dotfiles repo). `git status` there reports that whole repository —
/// on this machine one click produced 120 366 untracked entries and froze the window.
async fn repository_root(workspace: &str) -> Result<String, String> {
    let output =
        run_git_output(workspace, &["--no-pager", "rev-parse", "--show-toplevel"]).await?;
    // git writes this in the user's language, so the message is not the signal — the exit
    // status is. A rev-parse that names no top level has none for this folder.
    if !output.status.success() {
        return Err(NO_REPOSITORY.to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Cuts the list to what a panel can show and keeps the real count. Returned separately from
/// the command so the rule is testable without a repository on disk.
fn cap_entries(entries: Vec<GitEntry>) -> (Vec<GitEntry>, usize, bool) {
    let total = entries.len();
    let truncated = total > MAX_ENTRIES;
    let mut entries = entries;
    if truncated {
        entries.truncate(MAX_ENTRIES);
    }
    (entries, total, truncated)
}

/// Compares two paths without requiring them to exist twice.
fn same_directory(a: &str, b: &str) -> bool {
    let canon = |value: &str| {
        fs::canonicalize(value)
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_else(|_| value.trim_end_matches(['/', '\\']).to_string())
    };
    canon(a) == canon(b)
}

#[tauri::command]
pub async fn git_panel_status(workspace: String) -> Result<GitStatus, String> {
    let root = repository_root(&workspace).await?;
    if !same_directory(&root, &workspace) {
        return Err(NO_REPOSITORY.to_string());
    }

    // `--untracked-files=normal` collapses an untracked directory to one row. `all` would
    // enumerate every file inside it, which is what made a single click take nine minutes.
    let bytes = run_git(
        &workspace,
        &[
            "--no-pager",
            "status",
            "--porcelain=v1",
            "-z",
            "--untracked-files=normal",
        ],
    )
    .await?;
    let (entries, total, truncated) = cap_entries(parse_status(&bytes));
    let branch = git_text(&workspace, &["--no-pager", "branch", "--show-current"])
        .await
        .unwrap_or_default();
    let head = git_text(&workspace, &["--no-pager", "rev-parse", "--short", "HEAD"])
        .await
        .unwrap_or_default();
    Ok(GitStatus {
        branch,
        head,
        entries,
        total,
        truncated,
    })
}

#[tauri::command]
pub async fn git_panel_diff(
    workspace: String,
    path: String,
    staged: bool,
    untracked: bool,
) -> Result<String, String> {
    if untracked {
        let full = Path::new(&workspace).join(&path);
        // A multi-gigabyte artifact in an untracked folder would be read whole and then thrown
        // away by the diff limit. Refuse it by size instead — the panel says why.
        const MAX_UNTRACKED_BYTES: u64 = 2 * 1024 * 1024;
        let size = fs::metadata(&full)
            .map_err(|error| format!("Datei nicht lesbar: {error}"))?
            .len();
        if size > MAX_UNTRACKED_BYTES {
            return Ok(format!(
                "— Datei ist {:.1} MB groß; die Vorschau zeigt nur Dateien bis 2 MB.",
                size as f64 / (1024.0 * 1024.0)
            ));
        }
        let bytes = fs::read(&full).map_err(|error| format!("Datei nicht lesbar: {error}"))?;
        if bytes.contains(&0) {
            return Ok("Binärdatei — keine Textvorschau.".to_string());
        }
        let text = String::from_utf8_lossy(&bytes);
        let mut diff = format!("--- /dev/null\n+++ b/{path}\n");
        for line in text.lines() {
            diff.push('+');
            diff.push_str(line);
            diff.push('\n');
        }
        return Ok(limit_diff(diff));
    }

    let mut args = vec!["--no-pager", "diff", "--no-color"];
    if staged {
        args.push("--cached");
    }
    args.push("--");
    args.push(&path);
    let bytes = run_git(&workspace, &args).await?;
    Ok(limit_diff(String::from_utf8_lossy(&bytes).to_string()))
}

/// A huge generated file should not freeze the webview; the panel says what was cut.
fn limit_diff(mut diff: String) -> String {
    const LIMIT: usize = 400_000;
    if diff.len() > LIMIT {
        let mut cut = LIMIT;
        while cut > 0 && !diff.is_char_boundary(cut) {
            cut -= 1;
        }
        diff.truncate(cut);
        diff.push_str("\n… Diff gekürzt …\n");
    }
    diff
}

#[tauri::command]
pub async fn git_panel_stage(workspace: String, paths: Vec<String>) -> Result<(), String> {
    if paths.is_empty() {
        return Ok(());
    }
    let mut args = vec!["add", "--"];
    args.extend(paths.iter().map(String::as_str));
    run_git(&workspace, &args).await?;
    Ok(())
}

#[tauri::command]
pub async fn git_panel_unstage(workspace: String, paths: Vec<String>) -> Result<(), String> {
    if paths.is_empty() {
        return Ok(());
    }
    let mut args = vec!["--no-pager", "reset", "-q", "HEAD", "--"];
    args.extend(paths.iter().map(String::as_str));
    run_git(&workspace, &args).await?;
    Ok(())
}

#[tauri::command]
pub async fn git_panel_stage_all(workspace: String) -> Result<(), String> {
    run_git(&workspace, &["add", "-A"]).await?;
    Ok(())
}

#[tauri::command]
pub async fn git_panel_commit(workspace: String, message: String) -> Result<String, String> {
    let message = message.trim();
    if message.is_empty() {
        return Err("Die Commit-Nachricht ist leer.".to_string());
    }
    let bytes = run_git(&workspace, &["--no-pager", "commit", "-m", message]).await?;
    Ok(String::from_utf8_lossy(&bytes).trim().to_string())
}

#[tauri::command]
pub async fn git_panel_branches(workspace: String) -> Result<Vec<String>, String> {
    let bytes = run_git(
        &workspace,
        &[
            "--no-pager",
            "for-each-ref",
            "--format=%(refname:short)",
            "refs/heads",
        ],
    )
    .await?;
    Ok(String::from_utf8_lossy(&bytes)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect())
}

#[tauri::command]
pub async fn git_panel_checkout(workspace: String, branch: String) -> Result<(), String> {
    let branch = branch.trim();
    if branch.is_empty() || branch.starts_with('-') {
        return Err("Ungültiger Zweig.".to_string());
    }
    run_git(&workspace, &["--no-pager", "checkout", branch]).await?;
    Ok(())
}

#[tauri::command]
pub async fn git_panel_pull(workspace: String) -> Result<String, String> {
    let bytes = run_git(&workspace, &["--no-pager", "pull", "--ff-only"]).await?;
    let text = String::from_utf8_lossy(&bytes).trim().to_string();
    if text.is_empty() {
        Ok("Bereits aktuell.".to_string())
    } else {
        Ok(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    #[test]
    fn parses_staged_unstaged_untracked_renamed_and_conflicted() {
        let raw =
            b" M a.txt\0?? new.txt\0MM both.txt\0R  new-name.txt\0old-name.txt\0UU conflict.txt\0";
        let entries = parse_status(raw);

        assert_eq!(entries.len(), 7);
        assert_eq!(entries[0].path, "a.txt");
        assert!(!entries[0].staged);
        assert_eq!(entries[0].code, "M");

        assert_eq!(entries[1].path, "new.txt");
        assert!(entries[1].untracked);

        assert_eq!(entries[2].path, "both.txt");
        assert!(entries[2].staged);
        assert_eq!(entries[3].path, "both.txt");
        assert!(!entries[3].staged);

        assert_eq!(entries[4].path, "new-name.txt");
        assert!(entries[4].staged);
        assert_eq!(entries[4].code, "R");

        assert_eq!(entries[5].path, "conflict.txt");
        assert!(entries[5].conflicted);
        assert_eq!(entries[6].path, "conflict.txt");
        assert!(entries[6].conflicted);
        assert!(!entries[6].staged);
    }

    #[test]
    fn rejects_short_or_empty_entries() {
        assert!(parse_status(b"").is_empty());
        assert!(parse_status(b"abc\0").is_empty());
    }

    /// A repository with more entries than a panel can render is cut, and the real number is
    /// kept — the limit exists so one click cannot freeze the window.
    #[test]
    fn a_huge_list_is_cut_and_counted() {
        let entries: Vec<GitEntry> = (0..MAX_ENTRIES + 500)
            .map(|index| GitEntry {
                path: format!("file-{index}.txt"),
                code: "??".to_string(),
                staged: false,
                untracked: true,
                conflicted: false,
            })
            .collect();
        let (kept, total, truncated) = cap_entries(entries);
        assert_eq!(kept.len(), MAX_ENTRIES);
        assert_eq!(total, MAX_ENTRIES + 500);
        assert!(truncated);

        // An ordinary repository is untouched.
        let small: Vec<GitEntry> = Vec::new();
        let (kept, total, truncated) = cap_entries(small);
        assert!(kept.is_empty() && total == 0 && !truncated);
    }

    /// The guard that keeps the panel from listing a parent's tree: only the folder git names as
    /// the top level counts as this project's repository. Trailing separators are the same folder.
    #[test]
    fn the_repository_root_must_be_the_workspace_itself() {
        let base = std::env::temp_dir();
        let text = base.to_string_lossy().to_string();
        assert!(same_directory(&text, &format!("{text}/")));
        assert!(!same_directory(
            &text,
            &base.join("project").to_string_lossy()
        ));
    }

    /// The bug that froze the app: a project without its own `.git` inside a larger repository
    /// (here the parent) made git report that whole tree. The panel must refuse instead.
    #[tokio::test]
    #[ignore = "requires the git executable"]
    async fn a_foreign_repository_is_refused() {
        let parent = std::env::temp_dir().join(format!("ds-ide-git-parent-{}", std::process::id()));
        let child = parent.join("project");
        let _ = fs::remove_dir_all(&parent);
        fs::create_dir_all(&child).unwrap();

        let status = Command::new("git")
            .current_dir(&parent)
            .args(["init", "-q"])
            .status()
            .expect("run git");
        assert!(status.success());

        let error = match git_panel_status(child.to_string_lossy().to_string()).await {
            Ok(status) => panic!(
                "a project without its own repository must not list the parent's files — got {} \
                 entries",
                status.entries.len()
            ),
            Err(error) => error,
        };
        assert_eq!(error, NO_REPOSITORY);

        // Once the project has its own repository, the panel works on it.
        let status = Command::new("git")
            .current_dir(&child)
            .args(["init", "-q"])
            .status()
            .expect("run git");
        assert!(status.success());
        let ok = git_panel_status(child.to_string_lossy().to_string())
            .await
            .unwrap_or_else(|error| panic!("its own repository must work: {error}"));
        assert!(ok.entries.is_empty());
        assert_eq!(ok.total, 0);
        assert!(!ok.truncated);

        let _ = fs::remove_dir_all(&parent);
    }

    /// Exercises the real git binary against a throwaway repository: status, stage, diff and
    /// commit. Ignored by default because CI images and developer machines must have git.
    #[tokio::test]
    #[ignore = "requires the git executable"]
    async fn live_git_panel_round_trip() {
        let directory =
            std::env::temp_dir().join(format!("ds-ide-git-panel-{}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();

        let git = |args: &[&str]| {
            let status = Command::new("git")
                .current_dir(&directory)
                .args(args)
                .status()
                .expect("run git");
            assert!(status.success(), "git {:?} failed", args);
        };
        git(&["init", "-q"]);
        git(&["config", "user.email", "test@example.com"]);
        git(&["config", "user.name", "Test"]);

        fs::write(directory.join("a.txt"), "eins\n").unwrap();
        let workspace = directory.to_string_lossy().to_string();

        let status = git_panel_status(workspace.clone()).await.unwrap();
        assert!(status.entries.iter().any(|entry| entry.path == "a.txt" && entry.untracked));

        git_panel_stage(workspace.clone(), vec!["a.txt".to_string()]).await.unwrap();
        let status = git_panel_status(workspace.clone()).await.unwrap();
        assert!(status.entries.iter().any(|entry| entry.path == "a.txt" && entry.staged));

        let diff = git_panel_diff(workspace.clone(), "a.txt".to_string(), true, false).await.unwrap();
        assert!(diff.contains("+eins"));

        git_panel_commit(workspace.clone(), "test commit".to_string()).await.unwrap();
        let status = git_panel_status(workspace.clone()).await.unwrap();
        assert!(status.entries.is_empty(), "working tree should be clean");

        git(&["branch", "other"]);
        let branches = git_panel_branches(workspace.clone()).await.unwrap();
        assert!(branches.iter().any(|branch| branch == "other"));
        git_panel_checkout(workspace.clone(), "other".to_string()).await.unwrap();
        let status = git_panel_status(workspace.clone()).await.unwrap();
        assert_eq!(status.branch, "other");

        let _ = fs::remove_dir_all(&directory);
    }
}
