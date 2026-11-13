# Linux packaging for DS·IDE

## The order that matters

Build and **run** the Linux package locally *before* a tag exists. A tag starts the CI builds for
macOS and Windows, which cannot be tested here — so everything that can be checked on this machine
is checked first.

```sh
pnpm build                                   # frontend
cd src-tauri && cargo build --release --features tauri/custom-protocol   # the binary
pnpm tauri build --bundles deb,rpm,appimage  # the bundles
bash scripts/appimage-zsync.sh               # the update control file for the AppImage
```

## The AppImage is the one that matters

It is the only format that can be installed without a package manager, and on this machine no
distribution repository is open to the project. So it carries its own update path:

- `LDAI_VERSION` and `LDAI_UPDATE_INFORMATION` at build time give it a version to compare and a
  pointer to its `.zsync` control file (both end up in the ELF section `.upd_info` and in the
  embedded `.desktop`). The release workflow passes them.
- `scripts/appimage-zsync.sh` writes that control file next to the package, and CI uploads it with
  the release.

Without both halves an installed copy cannot tell whether it is current. Tauri has no step of its
own for this; `createUpdaterArtifacts` is a different mechanism (its own updater, needs a signing
key) and does not help.

## What is checked before a tag

- The `.desktop` inside the AppImage carries `Categories=Development;` — with an empty field, KDE
  puts the entry in no menu branch at all, so the app installs and appears nowhere.
- `.upd_info` is not empty, and the `.zsync` exists beside the package.
- `.deb` and `.rpm` are inspected (`dpkg-deb -c`, `rpm -qlp`) and the AppImage is started once.

## Flatpak — removed 2026-11-13

There was one, built and working: shared data directory, screenshots through the desktop portal,
the tray chain vendored into the bundle, a CI job. It was removed because it did not earn its
place — the maintainer does not use it, no one else does, and the channels that would have made it
worthwhile (Flathub) declined the project. Packaging that nobody runs is a claim, not a feature.

The sandbox-specific code paths are still in the program (`FLATPAK_ID` in `lib.rs`, `portal.rs`,
`in_sandbox` in `capture.rs`) because they are small and harmless. They no longer have a package
that reaches them; see `TODO.md`.
