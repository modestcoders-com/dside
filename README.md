###### [DE](./README_de.md) | [EN](./README.md) | [ES](./README_es.md) | [FR](./README_fr.md) | [RU](./README_ru.md) | [ZH](./README_zh.md)

# DS·IDE

An agentic desktop IDE on **Tauri v2** (no Electron) with **DeepSeek** as the model — and a partner inside it, not just a tool. The partner chooses its own name, once; until it does, it goes by the placeholder **Seeky**. It is called **Ziggy** now.

> The authoritative status, every design decision and the open list live in
> [PROGRESS.md](./PROGRESS.md) — not in this file.

![DS·IDE in Chinese: chat, work area with the README, and the file tree](docs/dside_050.webp)

*The same window in Chinese — one of six languages. The chat runs next to the files; the
work area shows a preview, not an editor.*

## What it does

**The partner**

- Agentic chat with streaming and real tool execution — reading, writing, editing, shell, search, memory, sub-agents.
- **Three permission stages**: **Plan mode** (read-only), **Ask before changes** (approve every write/edit/bash) and **Full access**. New installations start in Ask before changes; Full access goes through a consent dialog.
- One answer is a **timeline**: thinking, text and tool calls in the order they happened.
  Tool rows open to show the command, its output, and the approval state.
- **Long-term memory** the user can read, search, correct and retire (nothing is deleted), with a visible marker for what the user wrote or rewrote, optional project scoping, and a marker on notes it wrote from reasoning rather than from something it checked.
- **Context ring** showing how full the model's window is, and **compression**: older turns are folded into a summary the model writes itself (automatically at 75 %, or on click). The automatic trigger uses the provider's own prompt size for the last request plus the growth since; only a session without a measurement falls back to a character estimate.
- **Image attachments** by paste or picker; they stay with the message and are sent along.
- **Follow mode**: files the partner reads or edits open in the work area (preview or code editor), pages it fetches open in the app's browser. The eye in the tab bar turns it off; the cursor stays in the composer.
- **Retry** after a failed turn: the failed tail is rolled back, the same user message is sent again, and the error bubble survives a visit to Settings.
- **File history**: every successful write/edit stores the previous content first; the file tab shows the versions, a read-only Monaco diff and Restore (with a backup of the current state).

**The work area**

- File tree, source view (Monaco, read-only), and previews without external tools:
  Markdown, PDF, images, Word (docx, doc, odt), Excel (all sheets, values **or formulas**),
  presentations (pptx, ppt, odp), RTF.
- **Terminal:** a real PTY in a tab — ConPTY on Windows, the Unix PTY elsewhere. It stays alive while its tab exists; closing the tab stops the shell.
- **Git:** status, stage/unstage, diff, commit, local branches and `pull --ff-only`. No push or merge UI; the terminal is there for that.
- **Screen capture:** the eye in the composer captures the virtual desktop and attaches a 2000px JPEG. The `screenshot` tool lets the companion ask for a capture itself; with a `window` id it captures that browser window alone, cut out of the desktop, and says whether the crop worked. The image is sent as an OpenAI `image_url` content part; `deepseek-flash` reads text and layout from it (tested). Very small images (for example 1×1) are rejected by the provider.
- **Logs:** an in-memory ring of the app's and engine's log lines, with a Logs tab and a read-only `logs` tool for the companion. Nothing is written to disk.
- **Integrated browser.** With **Browser control** on, the partner can open, read and fill pages itself; the setting is under Settings → General. A local preview of workspace files (for example the HTML/CSS it just wrote) works without that switch.
- **Preview:** `preview(path)` renders a workspace file (HTML/CSS with relative assets) in the app browser through a local, read-only server and returns the page's text and layout — viewport, element boxes, spacing, overflow and colours. Works without Browser control. Clicking and filling that page (`browser_click`, `browser_fill`) works without the switch too, so interactive states are testable. `width`/`height` lay the page out at a given CSS size, so a breakpoint can be measured instead of guessed. The answer carries `size.requested`, `size.effective` and `size.clamped`, so a measurement clamped by the window minimum (320×280) is not mistaken for the requested one.
- **Skills** live in `~/.ds-ide/skills` and can be written in the app.

**The app**

- Sessions, open tabs, the last file and the window geometry survive a restart.
- **Copy:** every answer has a copy button (icon only) that copies the raw Markdown source, code fences and tables included.
- **Runaway answers:** if the model starts repeating itself, the stream is cut after a few hundred repeated characters and the turn says so; everything before the repetition is kept.
- **Long threads** open with the newest 80 messages; older ones load a page at a time without moving the line you are reading.
- **Archive:** search and filter archived tasks by project, restore them, or delete one or all — with a confirmation.
- **Tray:** closing the window keeps DS·IDE in the tray so reminders and schedules still fire; the tray menu opens it again or quits. Settings → General turns it off.
- **Daily memory check:** once a day a deterministic pass counts duplicate and never-used notes and says so in the memory panel — no model call, nothing changed.
- **Verify:** the partner can hand a claim to a second, read-only reader that checks it against the files and answers `VERDICT: PASS` or `VERDICT: FAIL` — a failed check reaches it as a failed tool, so a confident report cannot pass for one. When a claim needs a build or a test run, the partner names the command, the app runs it under the normal permission rules, and the reader judges the real transcript.
- **Reminders:** after handing work off (a push whose CI has to be watched), it can set itself a one-shot reminder that starts a new turn later.
- **Trust:** the companion chooses its name early, but you learn it only after 100 points earned over days — a day of use, finished tasks, its own notes, your corrections. The partner page shows the progress; nothing is awarded for a single click.
- **First start** asks for mode, workspace and API key; **Settings → Diagnostics** checks the keyring, provider, database and detected editors.
- **Keyboard shortcuts** are listed, searchable and editable under Settings → Keyboard shortcuts.
- **Policies:** the companion's `bash` tool can be limited to an allowlist and writes can be kept inside the workspace. This is a policy, not an OS sandbox; the user's terminal stays free.
- **Permissions:** Ask before changes offers allow once, allow for this session and always allow. Permanent grants are listed under Settings → Usage & model and can be revoked there; they also silence the safety question for that tool.
- Settings → a group named after the partner (memory, name, the user's own instructions).
- **Recall:** the partner can search its long-term notes and earlier conversations by keyword; results are limited to the current project.
- **Everyday mode**: the partner gets no shell, code review or sub-agents, and code panels make way for documents and the web; the user's terminal and Git tabs remain.
- Six languages (de, en, es, fr, ru, zh-Hans), light and dark theme, adjustable reading size.
- API key in the OS keychain, never in the frontend. Everything local in SQLite.

## Requirements

- Node + pnpm (**pnpm 10** — another major gives `ERR_PNPM_UNEXPECTED_STORE`, the store layouts are incompatible)
- Rust (stable)
- Linux: WebKitGTK 4.1, libsoup3, GTK3

## Run it

```sh
pnpm install
pnpm tauri dev
```

First start opens the onboarding dialog: choose **Programming** or **Everyday**, pick the
workspace (or type a path) and enter the API key. It is stored in the OS keychain. The default permission stage is **Ask before changes**: `write`, `edit` and `bash` pause for approval. **Plan mode** denies them, **Full access** runs them without asking after the
consent dialog. The key can be changed later under **Settings → Account & model**.

## Build an installer

```sh
pnpm tauri build
```

Produces `src-tauri/target/release/bundle/`: `.deb`, `.rpm` and `.AppImage` on Linux
(`dmg` on macOS, `nsis` on Windows). GitHub Actions builds all three platforms on a tag;
see `.github/workflows/release.yml`.

## Test

```sh
pnpm build                                                   # vue-tsc + vite
cd src-tauri && cargo test                                   # unit tests
DS_IDE_API_KEY=... cargo test --lib live_deepseek -- --ignored --nocapture
```

The live tests talk to the real provider and cost tokens; they are ignored by default.

## Structure

```
src/                  Vue frontend (components, stores, i18n)
src/engine/           persona, name pool, mode policy
src/lib/              previews, office readers, persisted UI state
src-tauri/src/        Rust: engine/ (LLM loop + tools), pty.rs (terminal), git_panel.rs,
                      capture.rs (screen), logs.rs, db.rs (SQLite), secret.rs (keyring)
scripts/              small tools (e.g. the name-table importer)
```

## Known limitations

- The partner works **inside** the app: it opens files in the work area and pages in the integrated browser. It cannot drive an external editor or your system browser.
- The integrated browser carries native window chrome; there is no shared toolbar above several pages yet.
- The partner chooses its name once, and it cannot be changed afterwards.
- **Permission stages are a policy, not an OS sandbox.** The allowlist and workspace-only write path restrict the companion, but allowed interpreters (node/python) and your own terminal remain powerful. In Ask before changes, sub-agent writes are denied because the sub-agent cannot reach the question panel.
- **Screenshots are image parts:** the tested `deepseek-flash` reads text and layout from them. Very small images are rejected; fine visual judgement remains a model limitation. On macOS, Screen Recording permission is required; Linux needs one of grim/gnome-screenshot/spectacle/scrot/ImageMagick.
- The Git panel has no push or merge UI; it has branch switching and pull. The terminal has
  one shell per tab and no profile picker.
- The published macOS DMG is **aarch64** (Apple Silicon); Intel needs another CI target or a universal build.
- No signed installers, no notarization, no auto-updater. Windows SmartScreen and macOS Gatekeeper warn; the app is still installable.
- Windows is tested on Windows 11 and in CI. The macOS build is CI-only and has not been runtime-tested on a Mac.
- No frontend E2E suite yet; CI runs the build and the Rust unit tests.

## Origins and acknowledgements

The window arrangement — chat beside a work area beside a file tree — is the shape most agent IDEs share; MiniMax Code's screenshots were the first visual reference for it. **No code was copied from it**: the engine, the tool protocol and the partner's persona were written for this project, and the layout was rebuilt rather than taken. Typeface: **Ubuntu Font Licence 1.0** (`public/fonts/UFL.txt`), credited in the app under Settings → About.

The reference checkouts this project was built against (`minimax-code-main`,
`minimax-jarvis`, `kilocode-main`) are **not** part of this repository; where to find them is recorded in PROGRESS.md §30–§31.

## Licence

MIT — see [LICENSE](./LICENSE). The bundled Ubuntu font keeps its own licence (`public/fonts/UFL.txt`). What that means in practice: use it, change it, ship it, even commercially — just keep the copyright notice.
