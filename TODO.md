# Open work

What is left, in the order I would do it. `PROGRESS.md` is the record of what happened;
this file is the list of what has not.

## Ready, small, no design needed

*Nothing left here at the moment.*

## Wanted, needs a decision (not a defect)

- **One instance with several windows** ("Window 1: <project>"), listed once in the task bar.
  Today several instances are allowed and share one database — verified with five at once, the
  history stayed intact. The title carries the project, which is what made them
  indistinguishable before. Whether to refuse a second start, or to grow into one process with
  several windows, is a product decision: the second is a multi-window architecture, the first
  would fight the per-project workflow.

## Needs a spec first

- **Selectable plugins.** Fixed set of bundled skills today; the ask is a plain store in the
  app's own look (no marketplace, no companion-generated plugins). Needs: where a plugin lives,
  what it may do, how it is enabled per project.
- **MCP client.** The user configures servers; nothing is shipped fixed. Needs: transport,
  permission model, tool namespace, how servers appear in the panel.
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
