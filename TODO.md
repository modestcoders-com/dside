# Open work

What is left, in the order I would do it. `PROGRESS.md` is the record of what happened;
this file is the list of what has not.

## Done in 0.6.5 (published 2026-11-15)

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

## Done in 0.6.3 (published 2026-11-13)

- **Desktop categories.** `bundle.category = DeveloperTool` — without it both the AppImage and
  the `.deb` carried `Categories=` empty, and KDE put the app in no menu branch at all. Confirmed
  on the actual desktop: DS.IDE appears under "Entwicklung".
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
  `URL`, `SHA-1`. **Proven end to end 2026-11-15:** Shelly offered `v0.6.5` for an installed
  `v0.6.2`, performed the update, and the result appeared in the menu marked as a new installation
  under "Entwicklung".

## Done in the 0.6.6 round (unreleased)

- **"Sign out" says what it does.** The menu entry is called "Abmelden", but it deletes the API key;
  the confirming step now reads "API-Key löschen" and the entry carries a hint saying where to put
  the key back. Found on the first Windows run with 0.6.5 — as a joke, which was fair.

- **An update hint inside the app.** `update_check` compares the running version with the newest
  GitHub release (numerically — `0.6.10` beats `0.6.9`), the title bar shows a small green
  "Update x.y.z" chip when there is something newer, and a click opens the release page in the
  system browser. It never installs anything itself, never blocks the start, stays silent when
  the check fails, and caches its answer for six hours. Proven in a packaged build with a
  temporarily lowered version; the click itself was not triggered (it opens a browser).

## Wanted, needs a decision (not a defect)

- **Several windows in one process.** Since 0.6.5 a second start raises the existing window
  instead of adding a copy — that is settled and confirmed on Windows. What is still open is the
  other direction: *one* process that shows several windows, one per project, so the task bar
  lists a single application. That is a multi-window architecture (shared state, one tray icon,
  a window per project in the rail) and a product decision, not a defect.
- **Absolute paths and the allowlist.** Refusing `Path::new`-style calls keeps the policy
  meaningful, but it also blocks an allowed tool that simply is not on PATH. Accepting an
  absolute path whose file name is allowed *and* which lies outside the workspace would fix the
  everyday case without letting the model point at something it wrote. Not decided.

## Specs written, waiting for the go-ahead

Each one is a page in `docs/specs/`. The decisions are taken; what remains is the order.

- **Plugins that may carry code** — `docs/specs/plugins.md`. A plugin is a folder with a manifest,
  optional notes and an optional program that speaks JSON-RPC over stdio — the same transport the
  MCP client will use, so one permission gate covers both. Text-only plugins need no code path of
  their own. Built-in skills stay built-in (decided). Disabling is immediate and leaves files on
  disk. No store in the first version; when there is a reason, a single repository with a
  `marketplace.json` and a SHA-256 per entry, which is what ZCode does.
- **MCP client** — `docs/specs/mcp-client.md`. Tool names are `mcp__<server>__<tool>`: double
  underscore so a name with a single one cannot be misread, and the `mcp__` prefix so an external
  tool is distinguishable from a built-in one at a glance. Dashes and dots are normalised away —
  readable for people, fragile for the tool-name grammars models are held to. stdio only;
  configuration is a 0600 JSON file the user writes.
- **Read-only `bash` for the verifier** — `docs/specs/verifier-readonly.md`. A fixed list of
  programs, no redirection, a deny-list of flags (`-delete`, `-exec`, `git branch -D`, `--output`
  …), workspace only. Anything unclassified does not run, and the verifier then reports that it
  could not check instead of claiming it did.
- **A reviewer before a release** — `docs/specs/release-reviewer.md`. Sees the diff, the claims and
  the evidence, never the reasoning that produced them. Three verdicts, and `UNDECIDED` blocks.
  Exit code in the release path. Our own model in a clean session, **not** a free-tier one: the
  task is judging whether evidence supports a claim, and a weak model agrees with confident prose —
  which is the failure being fixed.
- **No window before the state it needs** — `docs/specs/startup-window.md`. An `app_ready` flag
  plus a frontend gate closes the Windows race; creating the main window inside `setup` is the
  thorough version and stays the second step, because it reworks four start-up paths at once.

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
