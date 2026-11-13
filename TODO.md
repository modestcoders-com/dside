# Open work

What is left, in the order I would do it. `PROGRESS.md` is the record of what happened;
this file is the list of what has not.

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

## Known limits, not planned work

- Windows and macOS builds are unsigned and have never run on real hardware. Only the CI builds
  them.
- Sandbox-only code paths are now unreachable: `FLATPAK_ID` in `lib.rs`, `portal.rs` and
  `in_sandbox` in `capture.rs` exist for a package that is no longer built. Harmless, but dead —
  remove them when the next change touches capture or the data directory.
- The AppImage can update itself only if the `.zsync` reaches the release page next to the
  package; a download by hand is never wrong, it just does not help.
- Flathub, Snap and WinGet declined the project; homebrew accepted it. That is a positioning
  question ("how much of this was written with AI assistance, and is it stated openly"), not a
  code defect — and the answer has to be the maintainer's.

## Outside this repository

- **peer²'s Flatpak and Arch builds fail in CI** (`github.com/modestcoders-com/peer2`). Two
  causes, both diagnosed and both a one-line fix: the Flatpak job runs on `ubuntu-22.04`, whose
  `flatpak-builder` calls `appstream-compose`, which the GNOME 50 SDK no longer ships (needs
  `ubuntu-24.04`); and the Arch container installs pnpm 12, which ignores the `pnpm.overrides`
  field in `package.json`, so `--frozen-lockfile` fails (needs `npm i -g pnpm@10`, as `AGENTS.md`
  in both projects already says).
