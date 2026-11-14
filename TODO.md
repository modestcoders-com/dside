# Open work

What is left, in the order I would do it. `PROGRESS.md` is the record of what happened;
this file is the list of what has not.

## Done in the 0.6.5 round (unreleased)

- **One instance.** A second start raises the existing window instead of adding a copy; seven of
  them sat in the task bar at once, each with its own tray icon on the same database.
- **A read failure is not an empty database.** Settings are read with twelve attempts (~4 s); a
  final failure shows a banner and writes nothing, instead of greeting an existing installation
  with the first-start dialog and then overwriting its values with defaults.
- **Onboarding and shutdown write immediately** (`flushSettings`, plus a flush when the window is
  hidden), and every failed `settings_set` lands in the log.
- **Boot steps are independent.** A failing keyring read no longer costs the sessions, the file
  tree and the history — each step logs `ui: init: <step> failed: …` and the rest continues.
- **A stored `.` no longer comes back.** `adoptSessionWorkspace` re-binds the session to the real
  folder instead of adopting the relative path; `activateProject` and `bindActiveThread` refuse
  relative paths. Proven by reproducing the Windows state on a copy.
- **Two start-up log lines**: `data: <path> (<os>)` and `settings: read <n> keys, firstRun=…`,
  which are what make a report from a machine I cannot see answerable.

## Ready, small, no design needed

*Nothing left here at the moment.*

## Done in the 0.6.3 round (unreleased, gathering)

- **Desktop categories.** `bundle.category = DeveloperTool` — without it both the AppImage and
  the `.deb` carried `Categories=` empty, and KDE put the app in no menu branch at all.
- **A dead `.desktop` file no longer counts as an installed editor.** The PATH branch checked
  executability, the desktop branch trusted the file; a launcher left behind by a removed
  program appeared in the picker and failed on every click.
- **The path refusal names the real problem.** An allowed program missing from PATH is refused
  when called by path — correctly — but the message used to talk about a rule that was not the
  issue. It now names the program, says it is allowed, and says whether the file exists.
- **An expired question ends the turn** (`stop_reason: no_answer`) instead of letting the model
  decide without the user.
- **The AppImage can update itself.** `LDAI_VERSION` and `LDAI_UPDATE_INFORMATION` go into the
  build (they reach linuxdeploy through the environment, so no repacking), and
  `scripts/appimage-zsync.sh` writes the `.zsync` control file next to the package; CI uploads it
  with the release. Measured on a local 0.6.2 build: `.upd_info` carries
  `gh-releases-zsync|modestcoders-com|dside|latest|DS.IDE_*_amd64.AppImage.zsync`, the embedded
  `.desktop` carries `X-AppImage-Version=0.6.2`, and the control file has `Filename`, `MTime`,
  `URL`, `SHA-1`. **Not proven:** that Shelly performs the update — that needs a release which
  ships the `.zsync`, so it is the first thing to watch after 0.6.3.

## Wanted, needs a decision (not a defect)

- **One instance with several windows** ("Window 1: <project>"), listed once in the task bar.
  Today several instances are allowed and share one database — verified with five at once, the
  history stayed intact. The title carries the project, which is what made them
  indistinguishable before. Whether to refuse a second start, or to grow into one process with
  several windows, is a product decision: the second is a multi-window architecture, the first
  would fight the per-project workflow.
- **Absolute paths and the allowlist.** Refusing `Path::new`-style calls keeps the policy
  meaningful, but it also blocks an allowed tool that simply is not on PATH. Accepting an
  absolute path whose file name is allowed *and* which lies outside the workspace would fix the
  everyday case without letting the model point at something it wrote. Not decided.

## Needs a spec first

- **Selectable plugins — decided: plugins may carry code.** Not just text instructions. That
  moves plugins into the same class as MCP servers: foreign code that acts on the user's machine,
  so every call has to pass the permission model, and the trust question ("who wrote this, what
  may it touch") has to be answerable in the panel. Open: where a plugin lives (app data vs.
  folder named in settings), how enabling works per project, and whether the built-in skills
  become plugins too — one mechanism instead of two. Needs a spec before code.
- **MCP client — decided: same rules as `bash`.** The first use in a session is confirmed, later
  calls in that session run without asking; in read-only mode no MCP tool is offered at all,
  because nobody knows what it does. stdio first, HTTP/SSE later. Server output is foreign text
  like a web page: it may inform, never instruct. Configuration is a JSON file the user writes —
  no settings form for four fields.
- **Verifier that may run `bash` in read-only mode — decided: yes.** A fixed list of read-only
  programs (`git status/log/diff/show`, `grep`, `rg`, `ls`, `cat`, `head`, `tail`, `wc`, `diff`,
  `stat`), no redirection, no writing flags; anything not classified does not run, and the
  verifier then says it could not check instead of claiming it did. The point is not a sandbox
  but a classification with a small, reviewable list.
- **An independent reviewer before anything leaves the house.** The demand behind all three: more
  room to decide, and a second look before a release. The honest shape is a *process* the
  companion cannot skip, not a faculty it has: a fresh context that sees only the change, the
  claim and the evidence (never the reasoning that produced them), which has to demand checkable
  statements ("test X runs", "the package contains the category") and may not wave an unsupported
  claim through. A verdict with an exit code in the release path — like `check-private.mjs` today.
  It must be able to stop a release, not add a comment, and "undecided" has to block as well;
  otherwise it becomes a rubber stamp. Deliberately **not** a second model: the same weights
  produce the same plausible story, and a genuinely different faculty would live in the runtime,
  not in an application. The independence here comes from *information* — what the reviewer is
  not told — not from a different brain. Open: how its objection is recorded in the chronicle,
  and whether it also gates destructive commands and deletions.

- **The context ring estimates instead of measuring when the loaded history is shorter than the
  stored count.** `prompt_size` only trusts the provider's report while the number of loaded
  messages matches it; orphaned tool results (dropped at load time) break that match. Comparing
  `session_summaries.created_at` with `session_usage.updated_at` would separate "compressed" from
  "a few rows skipped" and make the exact figure usable again. Cosmetic at a 1M context.

## Known limits, not planned work

- **A Windows test needs a person.** Nothing here runs Windows; the build is checked by CI only.
  When something is reported from that machine, the honest steps are a hypothesis from the code,
  a log line that can be read back, and the user's report as the measurement — not a claim that it
  is fixed.
- **Synchronous Tauri commands run on the UI thread.** `open_browser_window` blocked there and
  deadlocked on Windows (§125) — and with 0.6.4 the browser window was confirmed working on that
  machine. Other sync commands have not been audited for the same pattern; anything that waits for
  the event loop must be `async`.
- **The main window may exist before `setup` has managed `Db` and `SecretStore`.** On the older
  Windows machine the webview asked first: state-free commands answered, stateful ones failed
  (`settings_all`, `secret_status`, `sessions_list`), and the UI then wrote its defaults over the
  stored values. The 0.6.5 changes make the boot steps independent and stop a read failure from
  looking like an empty database — the race itself is still there. The honest fix is to create the
  main window inside `setup`, after `app.manage`, which means reworking the splash handover.
- Windows and macOS builds are unsigned. The Windows build has now run on real hardware (0.6.4:
  window, browser window, editor detection with zero editors); macOS still has not.
- Sandbox-only code paths are now unreachable: `FLATPAK_ID` in `lib.rs`, `portal.rs` and
  `in_sandbox` in `capture.rs` exist for a package that is no longer built. Harmless, but dead —
  remove them when the next change touches capture or the data directory.
- The AppImage can update itself only if the `.zsync` reaches the release page next to the
  package; a download by hand is never wrong, it just does not help.
- Flathub, Snap and WinGet declined the project; homebrew accepted it. That is a positioning
  question ("how much of this was written with AI assistance, and is it stated openly"), not a
  code defect — and the answer has to be the maintainer's.

## Outside this repository

- **peer²'s CI** (`github.com/modestcoders-com/peer2`). The Flatpak job is obsolete — there will be
  no Flatpak for peer² either. The `arch` job still fails with `ERR_PNPM_LOCKFILE_CONFIG_MISMATCH`:
  the `archlinux:base-devel` container installs pnpm from the Arch repositories, which is now 12
  and ignores `pnpm.overrides` (the lockfile was written with pnpm 10). One line: pin
  `npm i -g pnpm@10` in that job, as `AGENTS.md` in both projects says. The maintainer is passing
  this on; the repository is not ours to change.
