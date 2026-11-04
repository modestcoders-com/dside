# DS·IDE — Handover

This is the short handover for taking the project over. `PROGRESS.md` is the authoritative
log (German, one section per round); `AGENTS.md` describes how to work in the repository.

**Repository:** https://github.com/modestcoders-com/dside (moved from `moc72/dside` on
2026-10-26; the old path redirects).

**Status:** `v0.6.0` is current. Since `v0.4.13` — the last release before the 0.5 line — the
project gained a tool-result discipline (nothing may be reported without a tool result behind
it), a verifier that answers fail-closed, self-reminders, paged history, the browser preview,
projects with threads bound to their folders, and the official whale mark as an animated SVG.

The five 0.5 releases were built and never published; `v0.6.0` carries their content.

## What it is

A local-first agentic desktop IDE on **Tauri v2** (no Electron):

- **Frontend:** Vue 3 + Vite. Chat timeline, work area (progress, file viewer, skills,
  schedules), Monaco (read-only viewer and diff), integrated previews.
- **Backend:** Rust. DeepSeek streaming loop and tools, SQLite, keyring, skills, schedules,
  native browser windows.
- **Model:** DeepSeek via an OpenAI-compatible chat-completions API. The API key never
  reaches the frontend; Rust reads it from the OS keyring per turn.

## Build and run

Requirements:

- Node 22+ and **pnpm 10** (other majors have incompatible store layouts; pnpm 12 also fails
  on `ERR_PNPM_IGNORED_BUILDS` for `he-tree-vue`).
- Rust stable.
- Linux: WebKitGTK 4.1, libsoup3, GTK3. Windows: WebView2. macOS: the system WebView.

```sh
pnpm install --frozen-lockfile
pnpm tauri dev                    # Vite on 1420 + debug window; cargo must be on PATH
```

Checks and release:

```sh
pnpm build                        # i18n check + vue-tsc + Vite
pnpm check:i18n                   # six catalogs, same keys, no German leftovers
cd src-tauri && cargo test --lib  # 63 passed, 13 ignored (live tests need DS_IDE_API_KEY)
pnpm tauri build                  # deb/rpm/AppImage, NSIS, dmg
```

On Windows, a direct `cargo build --release` needs
`--features tauri/custom-protocol`; without it the binary loads `http://localhost:1420`
instead of the embedded frontend. The Tauri CLI sets it itself.

CI: `.github/workflows/release.yml` runs verify (build + Rust tests) and builds Linux,
macOS and Windows on a version tag, creating a **draft** release. The tag is the version;
`scripts/set-version.mjs` keeps the manifests aligned. Signing and notarization are **not**
configured (see limitations).

## UI test bridge (dev only)

The native input path does not reach the WebKit window on Linux/Wayland (XTest is not
delivered; the X11 frame is a ghost). A debug build can therefore expose a small localhost
channel into the main webview:

```sh
DS_IDE_TEST_BRIDGE=1 pnpm tauri dev   # debug only; 127.0.0.1:9333, port via DS_IDE_TEST_BRIDGE_PORT
node scripts/ui-test.mjs eval 'report({ active: document.activeElement?.tagName })'
node scripts/ui-test.mjs shot /tmp/dside-ui.png
```

`POST /eval` runs JavaScript in the main window; the wrapper provides `report(data)`, which
stores a JSON payload with a sequence number through the `test_bridge_report` command.
`GET /report` returns it, `GET /health` is a liveness check. `scripts/ui-test.mjs` is the
client (`eval`, `report`, `wait`, `shot`); `shot` needs xdotool and ImageMagick on Linux,
while `eval`/`report` are platform-independent. A release build does not contain the server
(`#[cfg(debug_assertions)]`); without `DS_IDE_TEST_BRIDGE=1` nothing listens. This is a test
backdoor, not a product feature — use it against the isolated test data, not the real
workspace.

## First start

A fresh install has an empty `settings` table and shows the onboarding dialog:

- **Mode:** coding (shell, code review, sub-agents, code panels) or everyday (documents and
  web; no shell/code editor).
- **Workspace:** validated with `read_workspace_tree`; the partner reads and writes there.
- **API key:** stored in the OS keyring. The dialog shows whether a key is already present.

Existing installations never see the dialog (the `onboarded` flag is stored with the other
settings). The key can always be changed under **Settings → Usage & model**; **Settings →
Diagnostics** shows the keyring backend, database, workspace, editors and runs a provider
connection test.

## Where state lives

- Tauri app data dir (`com.deepvibe.ds-ide`): `ds-ide.sqlite3`, `attachments/`, the window
  state, and the `secrets.json` fallback (0600) if the keyring is unavailable.
- SQLite: sessions, messages and their timeline, tool calls, todos, memories, settings,
  schedules, session summaries, and `file_versions` (snapshots taken before successful
  `write`/`edit`).
- API key: OS keyring (`ds-ide.deepvibe` / `deepseek-api-key`). On Windows the Credential
  Manager, on macOS the Keychain, on Linux Secret Service. The platform features are
  required in `Cargo.toml`; without them `keyring` silently uses its mock store.
- Skills: `~/.ds-ide/skills/<name>/SKILL.md`.
- Project rules: `AGENTS.md` or `.dside/rules.md` in the workspace; loaded into the system
  prompt for the partner and its sub-agents.

## Git panel and long lists

- `git_panel_status` refuses a folder whose repository root is somewhere else (a project
  without its own `.git` inside a folder of projects). That case produced 120 366 untracked
  entries and froze the window. The refusal and a folder without any repository both answer
  `no_git_repository`, which the panel shows as one calm line ("Kein Git-Repository
  angelegt.") — the parent path belongs in a bug report, not in the panel. The signal is the
  exit status of `rev-parse`, never its text: git localizes its messages.
- Untracked entries are collapsed (`--untracked-files=normal`) and the list is capped at
  `MAX_ENTRIES` (2000) with `total`/`truncated` reported, so a huge repository cannot flood the
  DOM.
- Every git call goes through `run_git`, which is async with a 25 s timeout and `kill_on_drop`.
  Untracked diffs larger than 2 MB are refused by size instead of read whole.
- The work-area tab bar holds only tabs. "Open another panel" (git, logs, changes; terminal
  keeps its own icon) and the robot that follows the companion sit in the title-bar toolbar,
  between the terminal icon and the work-area toggle — that row never scrolls out of view.
  The menu entries are icon-only with the label on hover, like the rest of the bar.

## Features worth knowing

- **Projects:** the rail's top row is a project picker (folder, name, chevron, "+"). The
  base folder lives in Settings → General ("Projektbasis"). `settings.workspace` stays the one
  folder the tools work in; the base is only listed. A thread carries its project in
  `sessions.workspace`; opening a thread adopts it, switching the project switches to that
  project's newest thread (or a new one), and any explicit change re-binds the open thread
  (`bindActiveThread`). Helpers in `src/lib/projects.ts`, state in
  `src/composables/useProjects.ts`. The git panel watches the workspace; the terminal keeps
  running and shows its start folder.

## The whale and the About page

- `src/components/DeepSeekWhale.vue` draws the mark as three paths from
  `src/components/deepseek-whale.ts`: body, fluke, pectoral fin. They were traced from
  `src/assets/DS-icon-blue.png`, the official blue mark — do not redraw them by hand; retrace
  if the source changes. The fluke turns around the notch, the fin around its root.
- The moving pieces sit **on top of** the full silhouette in the same colour. That is what
  keeps a few degrees of rotation invisible inside the body; only what leaves the outline
  moves visibly. Their pivots are percentages of each path's `fill-box`.
- `still` (prop) and `prefers-reduced-motion` stop all movement.
- Composer drafts live in `src/lib/drafts.ts` (`localStorage`, keyed by session id, 40
  newest, 20k chars each). Text only — attachments are never resumed behind the user's back.
  The field restores on start, on a session switch and after a reload; sending clears it. Used in the sidebar (26 px),
  the first-run dialog (36 px) and the splash (110 px).
- The About page is a hero line (mark, name, version, authors), a one-line "built with" strip,
  two cards (facts / packages) and the addresses as chips. If a section grows, keep the
  two-column grid — the page is meant to fit one screen.

- **Verifier with a command:** `verify` takes an optional `command`. The caller names it, the
  engine runs it through `tools::run_shell_command` — the same function, allowlist and timeout as
  `bash` — and the transcript becomes evidence the reader may judge. Permissions match `bash`:
  refused in Plan mode, approved through the normal dialog in Ask mode (command first, claim
  below), immediate in full access. Without approval nothing is checked and the model is told
  not to report the claim as verified.
- **Verifier:** `verify` hands a claim to a second reader with a read-only allowlist (read,
  glob, grep, logs, settings, recall, web_fetch, browser_read, browser_windows, preview, skill).
  It answers in `Check / Method / Evidence / Result` blocks and ends with `VERDICT: PASS` or
  `VERDICT: FAIL`; a FAIL comes back as a **failed tool**, and a report without exactly one
  verdict counts as not verified. It is its own tool rather than a role of `task`, because
  `task` is off in everyday mode while a reader belongs in both.
- **Tray icon:** the app installs one when the desktop can show it (`src-tauri/src/tray.rs`).
  With it, closing the main window hides it instead of quitting, so the scheduler and reminders
  keep working; the menu has "Open window" and "Quit" (labels pushed from the frontend in the
  user's language), a left click opens the window, and Settings → General has
  `keepInBackground`. Without a tray the old behaviour stays: closing quits, because an
  invisible process with no way back is worse.
- **Daily memory check:** `src-tauri/src/memory_check.rs` runs once a day inside the scheduler
  loop, counts duplicates and never-used notes, stores the result in `settings.memoryCheck` and
  emits `memory://checked`. It changes nothing and costs no model call; the memory panel shows
  the last result, and setting entries aside stays a human decision.
- **Self-reminders:** `schedule` (add/list/cancel) sets one-shot or repeating reminders on the
  session; `schedules.one_shot` and `created_by` mark them, a fired one-shot disables itself
  before the turn and keeps `last_run_at`, and at most three active per session. Scheduled runs
  in Ask mode are read-only, since no one can approve from a timer.
- **Prompt layout:** the main persona lives in `src/engine/persona.ts` (frontend, follows the
  user's settings); sub-agent and verifier prompts live in `src-tauri/src/engine/prompts.rs`.
  Keep the shared discipline — nothing reported without a tool result — in both.
- **Evidence rule:** the persona states that no fact about the world (line numbers, file
  contents, command output, database rows, test results) may be reported unless a tool result in
  the same turn says it; a plan is not a result. Compression keeps that distinction — verified
  stays a fact, claimed becomes `unverified: …`. Memories carry `basis` (`verified`/`assumed`),
  and an assumed note is recalled with `[unverified]` and shown as "unbestätigt" in the panel.
- **Trust and the name:** the companion picks its name early (`choose_name`), but the UI shows
  the placeholder until 100 points are earned: 2 for the first turn of a day, 1 per finished
  turn that ran a tool successfully (max 10/day), 1 per memory it writes (max 5/day), 3 when
  the user corrects a memory (max 3/day). `trust_points` holds the rows; `TRUST_THRESHOLD` in
  `db.rs` is the line. Crossing it sets `trustRevealed`, emits `trust://revealed` and shows a
  banner; the partner page shows the progress and where the points came from. Installations
  that already had a name are marked revealed by the migration.
- **Window screenshots:** `screenshot` takes an optional `window` id. The desktop is captured
  and cropped to that window (`image` crate, PNG); the answer says whether the crop happened.
  A compositor that does not report window positions gets the whole desktop with a note.
- **Runaway answers:** the stream is watched for a repeated tail (one block of up to 200
  bytes, at least 320 bytes and 24 repeats). On a hit the stream is cut, the repetition is
  removed and the turn ends with `stop_reason: "loop"` plus a message; what came before is
  kept. Reasoning is stored and shown but never sent back to the provider, so a model cannot
  read its own loop back in.
- **Long threads:** the newest 80 messages open first (`HISTORY_PAGE`); older ones are read in
  pages with `session_history(beforeId, limit)` and prepended while the scroll position is
  kept. The engine still sends the whole history — this is a display concern only.
- **Side panels** stay mounted and slide over their grid column (320 ms, reduced-motion aware,
  transitions off while a divider is dragged). Hidden panels are `inert` and `aria-hidden`.
- **Archive:** search, project filter, restore, delete one and delete all — the deletions ask
  through the app's own dialog, and `session_delete_many` removes the whole archive in one
  call.
- **Timeline:** thinking, text and tool calls in the order they happened; tool rows open to
  show the command, output and approval state. Every assistant answer has a **copy markdown**
  action that copies the raw source (code fences and tables included).
- **File history:** every successful `write`/`edit` first stores the old content; the file
  tab's **History** shows the versions, a read-only Monaco diff and **Restore** (the current
  state is backed up before restoring; a snapshot from before the file existed removes it).
  The confirmation is the app's own dialog, and a removed file leaves a clear note in the tab.
- **Retry:** a failed turn shows the cause and a Retry button. The failed tail is rolled back,
  the same user message is sent again (no duplicate), and the error bubble survives a visit
  to Settings. The provider base URL is editable under **Settings → Account & model**.
- **Follow mode:** files the partner reads/writes open in the work area, `web_fetch` opens
  the page in the integrated browser; the eye in the tab bar turns it off. A read opens at
  once; a write/edit opens when the tool result confirms it, not while approval is pending.
  A followed file is revealed in the tree (parents expanded, row scrolled into view); the
  containing folders are re-read, so a file written after the tree was loaded still appears.
- **Browser control:** with Settings → General → Browser control on, the partner can open a
  page in a new window itself (`browser_read` with `url`), read its text, links and form
  fields, fill/submit forms (`browser_fill`) and click elements (`browser_click`). A read
  never navigates a page the user opened; the new window does not take the keyboard.
  `browser_read` also returns a layout snapshot (viewport plus visible elements' boxes,
  margins, padding and gaps).
- **Preview:** `preview(path)` serves the workspace read-only on a random localhost port,
  opens the file in the app browser and returns its text plus layout (viewport, boxes,
  spacing, overflow, colours) in the same call. It reuses one preview window, so iterating
  does not fill the desktop. Relative stylesheets, scripts and images work. It deliberately
  does **not** require Browser control: only workspace files are served, never remote pages.
  `browser_read`, `browser_click` and `browser_fill` on that same preview window are exempt
  from the switch and from approval too; every other page follows the browser rules.
  `width`/`height` lay the page out at a given CSS size (the window minimum is 320×280), so a
  breakpoint is measured rather than guessed; `size.requested`, `size.effective`,
  `size.clamped` and `size.kept` say what was actually measured. `screenshot` still captures
  the whole desktop.
- **Browser windows** are titled "DS·IDE — Browser" and then follow the page title after each
  read. `browser_windows` lists every window with its id, URL, title and which one a read
  without an id would use; `browser_read(url, id)` navigates that window deliberately, while
  `browser_read(url)` opens a new one so a page the user is reading is never replaced. A page
  that has no text, links or fields is reported with `loading: true` instead of as an empty
  result, and a failed `browser_click` names the window's current URL.
- **Settings tool:** read-only `settings` returns the configuration the current turn runs
  with (permission, browser control, policy, allowlist, model, effort) next to the stored
  values and the permanently trusted tools. A switch flipped mid-turn arrives with the next
  turn; the tool says so.
- **Permissions:** Ask before changes questions once per call, with "allow once", "allow for
  this session" and "always allow"; the last one is stored in `settings.trustedTools` and
  shown with a revoke button under Settings → Usage & model. A permanent grant also silences
  the danger floor for that tool.
- **Recall:** the partner can search its long-term notes and earlier conversations by
  keyword (`recall`). Notes and conversations are project-scoped; the workspace is filled
  into every hit.
- **Missing workspace:** `glob`/`grep` fail with the missing path instead of returning
  "(no matches)"; the composer status line and the file tree name the missing directory and
  link to Settings.
- **Memory:** long-term memory the user can read, search, correct and retire; notes can be
  project-scoped (recalled only inside that workspace); context compression folds older
  turns into a summary the model writes. The trigger counts text, tool arguments and
  images; the summary replaces the older turns.
- **Where we were:** a compressed conversation shows its summary as a sticky card above
  the timeline; the user can open it to re-enter the work.
- **Previews:** Markdown, PDF, images, Word, Excel (values or formulas), presentations, RTF
  without external tools.
- **Editor picker:** opens the workspace in an installed external editor; on Windows the
  `code` shim is launched through `code.cmd` without a console flash.
- **Skills and schedules:** skills on disk; an interval scheduler that runs only while the
  app is open.
- **Terminal:** a real PTY (portable-pty: ConPTY on Windows, the Unix PTY elsewhere) in the
  work area. The terminal stays alive while its tab exists; closing the tab stops the shell.
  Keys go to the shell while it has focus.
- **Git:** status (porcelain), stage/unstage/stage-all, diff, commit, local branch switching
  and `pull --ff-only` from the work area. No push or merge UI; the terminal is there for that.
- **Policies:** the companion's bash tool can be restricted to an editable allowlist of
  program names; chains with unknown programs, redirection, backticks and `$()` are rejected.
  `write`/`edit` can be kept inside the workspace (including no `..` and no symlink escape).
  The default is allowlist + workspace-only. Your own terminal is not restricted. This is a
  policy, not an OS sandbox; allowed interpreters such as node/python remain powerful.
- **Workspace picker:** a folder browser on the existing `read_dir_children` command (path
  field, up, folder list, choose) in the first-start dialog and under Settings → General.
  No native-dialog plugin or new dependency.
- **Screen capture:** the composer's eye button captures the virtual desktop and attaches a
  2000px JPEG. The `screenshot` tool lets the companion take a capture itself: Ask before
  changes asks first, Plan mode/Full access run directly, the picture appears in the tool row
  and is attached as a user image to the next model call. The image travels as an OpenAI
  `image_url` content part, not as a top-level `image` field; `deepseek-flash` reads text and
  layout from it (verified with a 520×180 fixture). Very small images are rejected. The full
  capture is temporary; Windows uses PowerShell, macOS `screencapture`, Linux one of
  grim/gnome-screenshot/spectacle/scrot/ImageMagick.
- **Logs:** an in-memory log ring (400 lines) receives the app's and engine's log lines. The
  Logs tab shows them; the `logs` tool lets the companion read them. Nothing is persisted.
- **Permission stages:** **Plan mode** (read-only), **Ask before changes** (approve every
  write/edit/bash), **Full access** (no prompts). The composer select and Settings use the
  same three stages; choosing Full access goes through a warning dialog with a required
  checkbox. New installations start in Ask before changes; the old full-authorization
  boolean is migrated. In Ask before changes the engine pauses with the concrete
  command/content before the tool runs and offers once/session/deny; a session grant lives
  only in memory and is cleared when the stage changes. Scheduled runs are read-only in Ask
  before changes (nobody can answer). Even in Full access, a small danger floor
  (`rm -rf /`, `sudo`, `mkfs`, `git reset --hard`, `curl | sh`, …) asks once.
- **Languages:** de, en, es, fr, ru, zh-Hans; light/dark theme and reading size.

## Trust and safety

- `write`, `edit` and `bash` follow the three permission stages. **Plan mode** denies them,
  **Ask before changes** pauses the turn and shows the concrete command or change for
  allow/deny, and **Full access** runs them without a prompt after the consent dialog. New
  installations start in Ask before changes; the old boolean is migrated.
- There is **no OS sandbox**. The policies are a policy: an allowlist of program names and
  a workspace-only write path. Allowed interpreters and the user's own terminal are not
  confined. File history is the undo net, not a security boundary. A small hard floor for
  destructive shell patterns asks even in Full access. In Ask before changes, sub-agent writes
  are still denied because the sub-agent cannot reach the question panel; use Full access for
  those. The terminal and the Git panel are user-initiated tools, not model tools.
- The **screen capture** button captures the whole virtual desktop; the `screenshot` tool does
  it on the companion's request (Ask before changes asks first). The image reaches the request
  as an OpenAI `image_url` content part; `deepseek-flash` reads it (tested). On macOS, Screen
  Recording permission is required; on Linux one of the capture tools must be installed.
- The API key never reaches the frontend; the database is local. Do not commit
  `secrets.json`; it is gitignored.

## Known limitations

- No signed installers, no notarization, no auto-updater. Windows SmartScreen and macOS
  Gatekeeper warn; the app is still installable.
- The published macOS DMG is **aarch64** (Apple Silicon). Intel needs another CI target
  (`macos-13`) or a universal build.
- The Git panel has no push or merge UI; it has branch switching and pull. The terminal has
  one shell per tab and no profile picker.
- Screenshots are real image inputs: the tested `deepseek-flash` reads text and layout from
  them. Very small images (for example 1×1) are rejected; fine visual judgement remains a
  model limitation.
- Full access is a per-action approval in Ask before changes, not a sandbox or capability
  layer. The Full access dialog is the consent for full access. The allowlist/workspace-only
  policies are a policy; a real OS sandbox/per-command capability layer is still open.
- The main frontend chunk is now ~224 KB; Monaco, PDF, Office, terminal, Git and logs are
  lazy chunks.
- No frontend E2E suite yet; CI runs the build and the Rust unit tests.

## Testing

```sh
pnpm build
cd src-tauri && cargo test --lib
DS_IDE_API_KEY=... cargo test --lib live_deepseek -- --ignored --nocapture
```

The live tests cost tokens and are ignored by default. A fresh-install smoke test was run on
Windows with an emptied `settings` table: the onboarding dialog appeared, coding mode was
chosen, the keyring key was recognized, the diagnostics connection test returned
`2 models · 27.58 USD`, and the original database was restored afterwards.

## Origins and licence

MIT (see `LICENSE`). No code was copied from another project. The window arrangement (chat,
work area, file tree) is the shape most agent IDEs share and MiniMax Code's screenshots were
the first visual reference — the engine, the tool protocol and the partner's persona were
written for this project and the layout was rebuilt. The bundled Ubuntu font is under the
Ubuntu Font Licence 1.0 (`public/fonts/UFL.txt`). The reference checkouts used during
development are not part of the repository and are gitignored.

## First checks for a new maintainer

1. Build with pnpm 10 and run the Rust tests.
2. Start from an empty app data dir: onboarding, keyring, workspace, provider.
3. Open Settings → Diagnostics and run the connection test.
4. Ask the partner to change a file; open the file tab's History, inspect the diff, restore.
5. Read `PROGRESS.md` §44–§68 for the recent fixes, then `AGENTS.md` for the working rules.
