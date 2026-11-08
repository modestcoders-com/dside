# Flatpak build

```sh
./packaging/flatpak/build.sh
flatpak install --user -y target/flatpak/DS-IDE-<version>-x86_64.flatpak
flatpak run com.deepvibe.dside
```

## What is in the bundle, and why

| Module | Why |
| --- | --- |
| `shared-modules/libayatana-appindicator` | The GNOME runtime has no tray chain at all. Without it the app quits when the window closes, and reminders (which need a running process) never fire. Vendored from `flathub/shared-modules`, with one line added to each of its three CMake modules: `-DCMAKE_INSTALL_LIBDIR=lib`. The SDK's CMake defaults to `lib64`, which the runtime does not search, and `flatpak-builder` in Ubuntu does not correct it — the build then fails in `libayatana-indicator` or, worse, installs the tray library where it cannot be found. |
| `git` (from source) | Not in the runtime. Built with `NO_RUST=1` — 2.47 clashes on an `unreachable` macro with the newer glibc, and 2.51+ wants cargo for `libgitcore`. |
| `dside` | The release binary, desktop entry, AppStream metadata, icon. |

## Permissions that matter

- `--filesystem=home` — the workspace is whatever folder the user picks.
- `--filesystem=xdg-data/com.deepvibe.ds-ide:create` — the data directory the AppImage and the
  deb use. Without it the sandboxed build is a *second installation* with an empty conversation.
- `--share=network` — the model API.
- `--talk-name=org.freedesktop.secrets` — the API key lives in the OS keyring.
- `--talk-name=org.freedesktop.portal.Desktop` — screenshots go through the desktop portal,
  because no capture tool exists inside the runtime.

## What works inside the sandbox

Everything the app is for: files in the home directory, git over HTTPS, the terminal, the
companion's shell, screenshots (through the portal, verified as a real 3840×1080 PNG), the tray
icon, and closing the window keeps the app running in the background.

The one difference from the AppImage: the first screenshot may ask the desktop for permission.
That is the desktop's decision, not the app's — and it is the price of a sandbox that means
something.
