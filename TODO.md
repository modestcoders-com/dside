# Open work

What is left, in the order I would do it. `PROGRESS.md` is the record of what happened;
this file is the list of what has not.

## The scope, decided 2026-11-25

Written down because it is the question that kept producing new work. Between 0.6.3 and 0.6.7
every release added a capability, and the one thing the product is judged by — how well the
companion does the work — was never measured.

**In:** the chat with one companion that reads and writes files, runs allowed commands, browses,
renders previews, remembers and verifies; projects with threads; settings that keep what they
promise.

**Out until further notice:** a plugin store, an MCP client, a release reviewer, several windows in
one process. Not because they are bad ideas — because an empty mechanism is not a feature, and the
time belongs to the companion.

**Then:** freeze. 0.6.7 plus the companion round, checked properly, and then the package for
DeepSeek. Seven weeks without a handover is not a schedule, it is a pattern.

## Done in the companion round (2026-11-25)

- **The tool descriptions are the half of the prompt nobody looks at.** Eighteen were a single
  sentence; each now names what the tool does, when to use it instead of its sibling, an example
  call and the limit that exists in the code. A test refuses anything under 60 characters.
- **The persona is fifty prohibitions and almost no procedure.** A new section, "How you approach a
  task", adds the order — look, say what you understood, smallest change, work the plan, verify,
  ask once, stop when done. Every tone rule is untouched on purpose.
- **There was no way to tell whether any of it helped.** `docs/companion-checks.md` holds ten
  concrete tasks with what a good and a bad answer look like, scored before and after. It is still
  unmeasured: that needs ten real conversations and the user's judgement.

## What is left before the freeze

- **Run the ten checks.** The one measurement nobody else can do. Same model, same permission
  stage, fresh thread for each task.
- **Then decide: ship or stop.** If ship: 0.6.8 with the plugin round and the companion round, then
  the package for DeepSeek. If stop: a final README line and this file closed, tags untouched.

## Done: the ten technical checks (2026-11-25/26)

`docs/qa-log.md` holds the runs, one per hour, each in a debug build against a copy of the
database: archiving and restoring a thread, history search, memories, the git panel, the preview
servers's assets, the terminal, project switching, appearance and reading size, all six languages,
and the turn diff with its undo. Seven found nothing; three found something — a missing
`application/pdf` MIME type and a system theme the app did not follow were fixed, and one search
behaviour is recorded below as a decision, not a defect.

These check the machinery, not the companion. Whether it works *well* is still the open question,
and it needs a model call and a judgement — see `docs/companion-checks.md`.

## Found in daily use (2026-11-27)

- **The companion works bash-heavy.** In the last 400 messages of a real thread: 261 `bash` calls
  against 3 `grep`, 1 `glob` and 30 `edit`. Every shell call costs a round, where one `grep` could
  have read several files in one. That is the likeliest reason a large task runs into the round
  ceiling. Not fixed: it is a working pattern in the prompt and the tool descriptions, and changing
  it needs the ten checks of `docs/companion-checks.md` to show whether it helped.
- **The round ceiling now has a checkpoint** (two budgets of 24 rounds and a final round without
  tools, see PROGRESS §142). Raising the number itself is not needed any more: the ceiling grants
  twice the work and ends with a hand-over instead of a cut.

## Found by QA

Not fixed, because each one is a decision rather than a slip. See `docs/qa-log.md` for the runs.

- **A search hit in an archived thread opens without a title.** `sessions_list` returns only
  unarchived threads, and the title bar derives its text from that list; search does not filter by
  the archive flag, so a hit in an archived thread opens the conversation while the header stays
  empty and the rail shows no active entry. Verified against the same flow on an unarchived thread,
  where both are correct. The open question is what should happen: exclude archived threads from
  search (loses the main reason to archive instead of delete), show them read-only, or unarchive on
  open. Frontend-only, and this project has no frontend test runner — a Rust test would not cover it.
- **Two conventions for comparing paths.** `samePath` in `src/lib/projects.ts` compares without
  regard for case, and `groupByProject` groups by the raw path, while `plugins::path_key` on the
  Rust side lowercases on Windows. Two spellings of the same folder (`C:\Users\…\Proj` vs
  `c:\users\…\proj`) would therefore count as two projects in the rail and as one in the plugin
  lookup. Not measured — it cannot happen on Linux. Worth one shared helper the next time either
  side is touched; nothing observed is broken today.

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

## Done in 0.6.6 (published 2026-11-16)

- **"Sign out" says what it does.** The menu entry is called "Abmelden", but it deletes the API key;
  the confirming step now reads "API-Key löschen" and the entry carries a hint saying where to put
  the key back. Found on the first Windows run with 0.6.5 — as a joke, which was fair.

- **An update hint inside the app.** `update_check` compares the running version with the newest
  GitHub release (numerically — `0.6.10` beats `0.6.9`), the title bar shows a small green
  "Update x.y.z" chip when there is something newer, and a click opens the release page in the
  system browser. It never installs anything itself, never blocks the start, stays silent when
  the check fails, and caches its answer for six hours. Proven in a packaged build with a
  temporarily lowered version; the click itself was not triggered (it opens a browser).

- **Three findings from the IDE, all fixed.** `preview` now names the assets it could not serve
  (a bundle with absolute paths renders empty and says nothing), the size note no longer blames the
  minimum when the window simply had not taken the new size, and a screenshot's picture is attached
  *after* all tool results of its block — it used to be pushed between them, which the provider
  rejects, so the companion was told an image would follow and never saw one.

- **Six findings from daily use, all addressed.** The chat no longer shatters into word
  fragments with a "thinking" fold between each (a model that thinks in passes sends interleaved
  deltas; reasoning is now one block at the top, and stored conversations are merged on load).
  `PYTHONHOME`/`PYTHONPATH` pointing into a bundle are stripped from child processes like
  `LD_LIBRARY_PATH` — though our own AppImage never set them. `2>&1` is a file descriptor, not a
  file, so `cargo test 2>&1 | grep …` works now. `sed`, `awk`, `jq`, `test`, `cmp`, `xxd`,
  `base64`, `sha256sum`, `md5sum` joined the default allowlist — and an untouched old list migrates
  to it automatically, while a customised one is left alone. The `timeout_ms` ceiling went from ten
  to thirty minutes, and the refusal message from five lines to two. `rm` and `cp` stay out of the
  defaults on purpose: one deletes, the other overwrites; whoever needs them adds them in Policies.

## Wanted, needs a decision (not a defect)

- **Several windows in one process.** Since 0.6.5 a second start raises the existing window
  instead of adding a copy — that is settled and confirmed on Windows. What is still open is the
  other direction: *one* process that shows several windows, one per project, so the task bar
  lists a single application. That is a multi-window architecture (shared state, one tray icon,
  a window per project in the rail) and a product decision, not a defect.
## Done in 0.6.7 (published 2026-11-19)

- **Extra program folders (decision C).** The companion can be told where to look for a program
  that is allowed by name but lives outside the system path — nvm, pyenv, rustup, Homebrew, Scoop.
  One folder per line under Policies; the list is trimmed and de-duplicated as it is typed, because
  an empty `PATH` segment means "the current directory" to a shell. The allowlist still decides
  *what* may run; this only decides where a name is looked for.
- **A missing program says why.** An allowed program that cannot be found now names itself and
  points at that setting, instead of the shell's bare `command not found` — the rustup case:
  allowed, installed, invisible.
- **`app_ready`.** Set as the last statement of `setup`; the interface waits for it before touching
  the database or the keyring, which closes the Windows start-up race. The retries stay, so a bug
  in the flag cannot cost the configuration again.
- **The verifier may read.** A fixed list of read-only programs, `git` limited to its reading
  subcommands, a deny-list for flags that write (`-delete`, `-exec`, `--output`, `git branch -D` …).
  Anything unclassified does not run, and the verifier then reports that it could not check. It
  also runs in Plan mode now: reading is exactly what that mode permits.
- **Two things the packaged run caught.** The new default list named `cmp` twice — one chip per
  entry, so removing it would have removed both; now 79 entries, each once. And the list migration
  ran before the watcher existed, so it was only written at the next unrelated change; it now
  writes its result immediately, once.

## Done in the plugins round (in the working tree, 2026-11-25)

- **Plugins with code, and notes without any.** A folder in `plugins/` with a `plugin.json` is a
  plugin; an optional `entry` program speaks the same JSON-RPC over stdio the MCP client will use.
  Tools are exposed as `plugin__<id>__<tool>` — the prefix so a foreign tool is recognisable, the
  split done against known ids so an underscore in a tool name is not ambiguous. The manifest
  decides the surface: a server offering more than its manifest names is refused.
- **Per project, and immediate.** The switch in the new Settings → Plugins panel applies to the
  folder the thread runs in; off removes the tools from the next turn and leaves the files alone.
  The panel shows id, version, author, the exposed names, the path and a fingerprint of the
  program.
- **"Test" without a conversation.** A button starts the server once and lists offered vs.
  allowed vs. missing — the only way to check a plugin without spending a model call.
- **Notes reach the prompt.** `SERVER.md` of an enabled plugin is appended under a heading that
  says it came from the plugin's authors and cannot change the rules. A notes-only folder needs no
  program.
- **Same gate as `bash`.** First use in a session is confirmed, later calls are not; read mode
  offers none, and everyday mode loses them together with the shell.

132 tests, `vue-tsc`, i18n (555 keys) and `check:private` are green. A real model turn with a
plugin tool is still unproven — that needs a conversation.

## Specs written, waiting for the go-ahead

Each one is a page in `docs/specs/`. The decisions are taken; what remains is the order.

- **A store for plugins is out of scope for now, and the decision stands.** When there is a reason
  for one, the shape is: a single repository with a `marketplace.json` and a SHA-256 per entry,
  which is what ZCode does. Borrowing *their* store is not the same thing: their 58 entries are
  skill bundles (markdown plus scripts) with a manifest we do not read, some with MCP servers and
  a pre-write hook system DS.IDE does not have.

- **MCP client** — `docs/specs/mcp-client.md`. Tool names are `mcp__<server>__<tool>`: double
  underscore so a name with a single one cannot be misread, and the `mcp__` prefix so an external
  tool is distinguishable from a built-in one at a glance. Dashes and dots are normalised away —
  readable for people, fragile for the tool-name grammars models are held to. stdio only;
  configuration is a 0600 JSON file the user writes.
- **Read-only `bash` for the verifier** — `docs/specs/verifier-readonly.md`. **Built.** A fixed list of
  programs, no redirection, a deny-list of flags (`-delete`, `-exec`, `git branch -D`, `--output`
  …), workspace only. Anything unclassified does not run, and the verifier then reports that it
  could not check instead of claiming it did.
- **A reviewer before a release** — `docs/specs/release-reviewer.md`. Sees the diff, the claims and
  the evidence, never the reasoning that produced them. Three verdicts, and `UNDECIDED` blocks.
  Exit code in the release path. Our own model in a clean session, **not** a free-tier one: the
  task is judging whether evidence supports a claim, and a weak model agrees with confident prose —
  which is the failure being fixed.
- **No window before the state it needs** — `docs/specs/startup-window.md`. **First step built** (the flag); creating the window in `setup` is still open. An `app_ready` flag
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
