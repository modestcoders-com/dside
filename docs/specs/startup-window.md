# Spec: no window before the state it needs

## Goal

Remove the cause of the Windows start-up fault instead of softening it. On that machine the
webview asked before the backend had managed `Db` and `SecretStore`: commands without state
answered (working folder, editor list, window title), commands with state failed
(`settings_all`, `secret_status`, `sessions_list`). An empty answer looked like a fresh
installation, and the interface then wrote its defaults over the stored values.

0.6.5 made this survivable — a failed read is retried, a failure is not treated as "empty", every
boot step is isolated and logged. The race itself is still there.

## What exists today

- Both windows are declared in `tauri.conf.json` with `visible: false`; the app shows the splash,
  then the main window once `splash_ready` arrives.
- `setup` runs `data_dir` → `Db::open` → `app.manage(...)` → tray → scheduler → test bridge.
- The main webview is created by Tauri from the config, and its frontend starts booting
  immediately: `initSettings` and `initEngine` can both run before `setup` has finished.

## The fix, in two parts

**1. A ready flag the frontend can read without state.**

A `static READY: AtomicBool` in `lib.rs`, set as the **last** statement of `setup` after
`app.manage` calls have returned. A state-free command:

```rust
#[tauri::command]
fn app_ready() -> bool { READY.load(Ordering::SeqCst) }
```

It needs no `tauri::State`, so it answers at any moment — the one thing the frontend can ask early.
`initSettings` and `initEngine` await it (poll every 25 ms, give up after 10 s and log
`startup: app_ready never arrived`).

**2. Retry stays, so a missed flag is not fatal.**

The twelve attempts in `readStored` and the isolated `bootStep`s stay. Belt and braces here is
deliberate: the flag closes the race, the retry survives a bug in the flag.

## Why not create the window in `setup`

That is the textbook answer and it is the more thorough fix: build the main window inside `setup`
after `manage`, and nothing can run early. It is not the first version because it moves the splash
handover (`splash_ready`, the `visible: false` dance, window-state restore, the tray's
`show_main_window`) into one function — four paths that are currently known to work on three
platforms. Changing the start path and the splash in a round where nobody can test Windows again
is how a start-up fault is traded for a different start-up fault.

So: the flag first. If Windows still reports something odd after that, the window-in-`setup`
rework is the next step, and it will have a measured starting point.

## What "done" means

- `app_ready` is false until `setup` has managed everything, and the frontend does not call a
  stateful command before it is true.
- A simulated slow `setup` (an artificial sleep before `manage` completes) shows no failed
  `settings_all` in the log.
- The log keeps the two lines added in 0.6.5 (`data: …`, `settings: read …`), because they are how
  a report from an untestable machine becomes readable.
- Tests: `app_ready` is false before the flag is set and true after; a unit test on the frontend
  side is not possible here, so the check is the log in a packaged build.

## Out of scope

- Removing the retries. They are cheap and they cover the failure this spec cannot reproduce here.
- Changes to the splash artwork or timing.
