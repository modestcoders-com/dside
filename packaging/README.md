# Linux packaging for DS·IDE

## The order that matters

Build and **run** the Linux package locally *before* a tag exists. A tag starts the CI builds for
macOS and Windows, which cannot be tested here — so everything that can be checked on this machine
is checked first.

```sh
pnpm build                                   # frontend
cd src-tauri && cargo build --release --features tauri/custom-protocol   # the binary
pnpm tauri build --bundles deb,rpm,appimage  # the usual bundles
./packaging/flatpak/build.sh                 # Flatpak (see below)
```

## Flatpak

```sh
flatpak install --user -y target/flatpak/DS-IDE-<version>-x86_64.flatpak
flatpak run com.deepvibe.dside
```

`build.sh` compiles the program first and packages it second: the sandbox only copies the release
binary, `.desktop`, AppStream metadata and icon into place. What is packaged is what was run.

## What the sandbox changes — and what it does not

Flatpak *is* a sandbox; there is no version without one. The question is not "why a sandbox" but
which permissions the app needs, and which of its features can still work inside.

**It sees the same data as every other installation.** A sandboxed app normally gets its own
directory (`~/.var/app/<id>/data`), which means a second installation opens with an empty
conversation while the first one still has the history. The manifest mounts the usual data
directory (`--filesystem=xdg-data/com.deepvibe.ds-ide`) and the program uses it when it runs under
Flatpak (`data_dir` in `lib.rs`). One history, one set of memories, one settings record.
Only one of them should run at a time — both write the same SQLite file.

**Screenshots keep working, through the desktop portal.** The capture tools the AppImage calls
(`grim`, `spectacle`, `scrot`) are not in the runtime, so inside the sandbox the app asks the
desktop instead (`portal.rs`): the compositor takes the picture, the desktop decides, and the app
receives a file. The desktop may ask the user once for permission — that is its decision, not the
app's. Without this the companion would be blind in the packaged build, which would make the
Flatpak a version with half the senses.

Three things are decided by the manifest and are easy to get wrong later:

- **`libayatana-appindicator` is not in the GNOME runtime.** The tray crate *panics* (not returns)
  when the library is missing, and the release profile uses `panic = "abort"`, so the app checks
  for the library before it builds an icon (`tray_supported`). In the Flatpak there is no tray;
  closing the window quits, which is the honest behaviour without an icon.
- **`git` is not in the runtime either.** It is built into the app from source (`NO_RUST=1`;
  git ≥ 2.51 needs cargo for `libgitcore`), so the git panel, the terminal and the companion's
  shell all find it. Checked: `git ls-remote https://…` works inside the sandbox.
- **No tray icon.** With the icon absent, closing the window ends the app; reminders therefore
  only fire while a window is open. A tray in the sandbox would need the appindicator library
  inside the runtime, which GNOME's does not carry.
