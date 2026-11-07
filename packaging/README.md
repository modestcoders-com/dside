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

Three things are decided by the manifest and are easy to get wrong later:

- **`libayatana-appindicator` is not in the GNOME runtime.** The tray crate *panics* (not returns)
  when the library is missing, and the release profile uses `panic = "abort"`, so the app checks
  for the library before it builds an icon (`tray_supported`). In the Flatpak there is no tray;
  closing the window quits, which is the honest behaviour without an icon.
- **`git` is not in the runtime either.** It is built into the app from source (`NO_RUST=1`;
  git ≥ 2.51 needs cargo for `libgitcore`), so the git panel, the terminal and the companion's
  shell all find it. Checked: `git ls-remote https://…` works inside the sandbox.
- **Screenshots do not work in the sandbox** — no `grim`/`spectacle`/`scrot` inside, and the
  portal is not wired up. The tool reports the failure; the rest of the app is unaffected.
