//! The tray icon: the app keeps its promises while no window is open.
//!
//! A reminder set with `schedule` can only fire while the process is alive, and a turn can only
//! be started by something that is running. So closing the window hides it instead of ending the
//! app — but only when the tray icon actually appeared. Without one the process would be
//! invisible and unreachable, which is worse than quitting honestly.
//!
//! The menu labels are UI text and belong to the six catalogs, so the frontend pushes them
//! (`tray_set_labels`) as soon as it has a locale. The words here are the ones the menu shows
//! for the moment before that.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

pub struct TrayState {
    running: AtomicBool,
    open_item: Mutex<Option<MenuItem<tauri::Wry>>>,
    quit_item: Mutex<Option<MenuItem<tauri::Wry>>>,
}

impl Default for TrayState {
    fn default() -> Self {
        Self {
            running: AtomicBool::new(false),
            open_item: Mutex::new(None),
            quit_item: Mutex::new(None),
        }
    }
}

impl TrayState {
    /// Whether the icon is on screen. Closing the window may only keep the process alive when
    /// there is something the user can click to get it back.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}

/// Builds the tray icon and menu. Returns false when the desktop cannot show one — a session
/// without a StatusNotifier host or without the appindicator library — and the caller keeps
/// "closing quits" then.
pub fn install(app: &AppHandle) -> bool {
    // A tray icon needs GTK's appindicator on Linux, and that library is not in every
    // environment: the GNOME runtime does not ship it, nor does a minimal desktop. The crate
    // behind it *panics* (not returns) when the library is missing, and the release profile is
    // built with `panic = "abort"`, so a catch would not help — the check has to come first.
    // Without an icon the window still opens, and closing quits, which is the honest behaviour.
    if !tray_supported() {
        crate::log_line!(
            "tray: not available — the appindicator library is missing in this environment"
        );
        return false;
    }

    let (menu, open, quit) = match tray_menu(app) {
        Ok(parts) => parts,
        Err(error) => {
            crate::log_line!("tray: menu failed: {error}");
            return false;
        }
    };

    let mut builder = TrayIconBuilder::with_id("dside-tray")
        .menu(&menu)
        .tooltip("DS·IDE")
        // A left click on the icon means "open" on Linux; macOS opens the menu by itself.
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => crate::show_main_window(app),
            "quit" => {
                crate::log_line!("tray: quit from the menu");
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                crate::show_main_window(tray.app_handle());
            }
        });

    // The tray crate writes the icon as a PNG and passes its **path** to the panel as the icon
    // name. In a sandbox that path is a private `/run/user/…/tray-icon/…` the desktop cannot
    // open, so the entry exists with a working menu and an empty slot — exactly what a Flatpak
    // build showed. The file therefore goes into the cache directory under the home, which the
    // manifest shares with the host.
    if let Some(dir) = tray_icon_dir() {
        builder = builder.temp_dir_path(dir);
    }

    // The icon is embedded, not taken from the window: `default_window_icon()` was empty in the
    // packaged build, and a tray entry without a picture is a blank slot in the panel.
    // The mark itself, transparent: a square app icon reads as a dark sticker in a panel.
    match tauri::image::Image::from_bytes(include_bytes!("../icons/tray.png")) {
        Ok(icon) => builder = builder.icon(icon),
        Err(error) => {
            crate::log_line!("tray: embedded icon unreadable: {error}");
            if let Some(icon) = app.default_window_icon().cloned() {
                builder = builder.icon(icon);
            }
        }
    }

    match builder.build(app) {
        Ok(_) => {
            let state = app.state::<TrayState>();
            if let Ok(mut slot) = state.open_item.lock() {
                *slot = Some(open);
            }
            if let Ok(mut slot) = state.quit_item.lock() {
                *slot = Some(quit);
            }
            state.running.store(true, Ordering::SeqCst);
            crate::log_line!(
                "tray: icon installed (embedded: {}, window icon: {})",
                true,
                app.default_window_icon().is_some()
            );
            true
        }
        Err(error) => {
            crate::log_line!("tray: not available: {error}");
            false
        }
    }
}

/// Where the tray icon file goes: inside the home, so the desktop's panel can read it. Outside a
/// sandbox the runtime directory would do, but there is no reason for two code paths.
fn tray_icon_dir() -> Option<std::path::PathBuf> {
    let home = std::env::var_os("HOME")?;
    let dir = std::path::PathBuf::from(home).join(".cache/ds-ide/tray");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

/// Can this environment show a tray icon at all?
#[cfg(target_os = "linux")]
fn tray_supported() -> bool {
    // The same four names the appindicator crate tries, in the same order.
    const NAMES: [&str; 4] = [
        "libayatana-appindicator3.so.1",
        "libappindicator3.so.1",
        "libayatana-appindicator3.so",
        "libappindicator3.so",
    ];
    NAMES.iter().any(|name| {
        let Ok(cname) = std::ffi::CString::new(*name) else {
            return false;
        };
        // SAFETY: a plain dlopen/dlclose pair on a constant name; the handle is closed again
        // and never dereferenced. RTLD_LAZY is enough to prove the library exists.
        unsafe {
            let handle = libc::dlopen(cname.as_ptr(), libc::RTLD_LAZY | libc::RTLD_LOCAL);
            if handle.is_null() {
                false
            } else {
                libc::dlclose(handle);
                true
            }
        }
    })
}

/// macOS and Windows always have a tray; nothing to check.
#[cfg(not(target_os = "linux"))]
fn tray_supported() -> bool {
    true
}

/// Rewrites the two menu entries. Called by the frontend once it knows the language, and again
/// whenever the user changes it.
#[tauri::command]
pub fn tray_set_labels(
    state: tauri::State<'_, TrayState>,
    open: String,
    quit: String,
) -> Result<(), String> {
    if !state.is_running() {
        return Ok(());
    }
    if let Ok(slot) = state.open_item.lock() {
        if let Some(item) = slot.as_ref() {
            item.set_text(open).map_err(|error| error.to_string())?;
        }
    }
    if let Ok(slot) = state.quit_item.lock() {
        if let Some(item) = slot.as_ref() {
            item.set_text(quit).map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

fn tray_menu(
    app: &AppHandle,
) -> tauri::Result<(Menu<tauri::Wry>, MenuItem<tauri::Wry>, MenuItem<tauri::Wry>)> {
    // Fallback wording; the frontend replaces it with the user's language right after mount.
    let open = MenuItem::with_id(app, "open", "Open window", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&open, &separator, &quit])?;
    Ok((menu, open, quit))
}

/// Whether closing the window should hide it. Two conditions, both required: the tray is there,
/// and the user has not turned the behaviour off.
pub fn hides_on_close(app: &AppHandle) -> bool {
    if !app.state::<TrayState>().is_running() {
        return false;
    }
    app.try_state::<crate::db::Db>()
        .and_then(|db| db.all_settings().ok())
        .and_then(|settings| settings.get("keepInBackground").cloned())
        .map(|value| value.trim().trim_matches('"') != "false")
        .unwrap_or(true)
}
