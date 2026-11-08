# Open work

What is left, in the order I would do it. `PROGRESS.md` is the record of what happened;
this file is the list of what has not.

## Waiting for a decision

- **0.6.2 — patch release.** Everything below under "ready" is on `main`: the reversed
  `is_error` flag, `grep` on a single file, the command scanner, the shared data directory and
  the tray icon in the Flatpak, the portal screenshots. `0.6.1` carries two of these bugs in
  published builds. Order agreed with the maintainer: build deb/rpm/AppImage and the Flatpak
  **locally, start them**, and only then set the tag — a tag starts the macOS and Windows builds,
  which cannot be checked here.

## Ready, small, no design needed

- **The question window's deadline is hardcoded** (five minutes, `ASK_TIMEOUT` in
  `engine/mod.rs`). It is now visible as a countdown and the typed text survives expiry, but the
  length is a guess. Belongs in the settings.
- **Screenshot of the browser window only.** `screenshot` captures the whole desktop; a page the
  companion opened cannot be photographed on its own.
- **Single instance.** Two windows can run at once and share one database (WAL and the busy
  timeout keep it from breaking, but two views of one conversation invite confusion). Either
  refuse the second start or make the windows distinguishable — see the next point.
- **The `.flatpak` is not attached to a release** and CI does not build it. Building it locally
  and uploading it by hand is the agreed path for now.
- **`flatpak-builder` fetches the git source from the network.** A pinned, vendored source would
  make the build reproducible offline.

## Needs a spec first

- **Selectable plugins.** Fixed set of bundled skills today; the ask is a plain store in the
  app's own look (no marketplace, no companion-generated plugins). Needs: where a plugin lives,
  what it may do, how it is enabled per project.
- **MCP client.** The user configures servers; nothing is shipped fixed. Needs: transport,
  permission model, tool namespace, how servers appear in the panel.
- **Several windows in one instance**, listed as "Window 1: <project>" in the task bar. A
  multi-window architecture with state per webview, not a parameter.
- **Verifier that may run `bash` read-only.** Needs a classification of commands by read effect;
  the permission model alone cannot answer it.

## Known limits, not planned work

- Windows and macOS builds are unsigned and have never run on real hardware. Only the CI builds
  them.
- Screenshots inside the Flatpak go through the desktop portal; the desktop may ask for
  permission the first time. That is its decision, not the app's.
- The AppImage carries no update metadata, so no store can update it. The Flatpak is the channel
  that can.
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
