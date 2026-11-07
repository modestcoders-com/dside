use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tauri::{AppHandle, Manager};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenCapture {
    pub name: String,
    pub mime: String,
    pub data_base64: String,
}

/// A capture that may be the window alone or, when the compositor would not say where the
/// window is, the whole desktop. `window` says which one arrived, so nobody has to guess.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowCapture {
    pub name: String,
    pub mime: String,
    pub data_base64: String,
    pub window: bool,
    pub note: String,
    /// Where the window was found, in desktop pixels, and how large the desktop is.
    pub rect: Option<[i64; 4]>,
    pub desktop: Option<[u32; 2]>,
}

/// Captures the whole virtual desktop and stores it next to the app data. The Composer puts
/// it into the normal attachment list, so the model sees exactly what the user sees.
#[tauri::command]
pub fn capture_screen(app: AppHandle) -> Result<ScreenCapture, String> {
    let bytes = capture_png(&app)?;
    Ok(ScreenCapture {
        name: capture_name(),
        mime: "image/png".to_string(),
        data_base64: crate::base64_encode(&bytes),
    })
}

/// The PNG bytes of the whole desktop; the temporary file belongs to the capture tool.
fn capture_png(app: &AppHandle) -> Result<Vec<u8>, String> {
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join("screenshots");
    // (`app_data_dir` on purpose: a screenshot is a transient of this installation and does not
    // belong in the shared directory the database lives in.)
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    let path = directory.join(format!("capture-{stamp}.png"));

    capture_to(&path)?;
    let bytes = fs::read(&path).map_err(|error| format!("screenshot: {error}"))?;
    // Temporary: the attachment is written by the frontend, the model gets the bytes.
    let _ = fs::remove_file(&path);
    if bytes.len() < 64 {
        return Err("screenshot: the capture is empty".to_string());
    }
    if bytes.len() > 30 * 1024 * 1024 {
        return Err("screenshot: the capture is larger than 30 MB".to_string());
    }
    Ok(bytes)
}

fn capture_name() -> String {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    format!("screenshot-{stamp}.png")
}

/// Captures one of our own windows. A desktop tool can only photograph the screen, so the
/// window is cut out of the desktop capture afterwards: its position comes from the window
/// manager. A compositor that does not publish positions (some Wayland sessions) leaves the
/// full desktop as the honest fallback, with a note saying so.
#[tauri::command]
pub fn capture_window(app: AppHandle, id: String) -> Result<WindowCapture, String> {
    let window = app
        .get_webview_window(&id)
        .ok_or_else(|| format!("screenshot: no window '{id}'"))?;
    let bytes = capture_png(&app)?;

    let position = window.outer_position();
    let size = window.outer_size();
    let (Ok(position), Ok(size)) = (position, size) else {
        return Ok(WindowCapture {
            name: capture_name(),
            mime: "image/png".to_string(),
            data_base64: crate::base64_encode(&bytes),
            window: false,
            note: "the compositor does not publish window positions; this is the whole desktop"
                .to_string(),
            rect: None,
            desktop: None,
        });
    };
    if size.width == 0 || size.height == 0 {
        return Ok(WindowCapture {
            name: capture_name(),
            mime: "image/png".to_string(),
            data_base64: crate::base64_encode(&bytes),
            window: false,
            note: "the window has no size yet; this is the whole desktop".to_string(),
            rect: None,
            desktop: None,
        });
    }

    match crop_png(&bytes, position.x, position.y, size.width, size.height) {
        Ok((cropped, desktop)) => Ok(WindowCapture {
            name: capture_name(),
            mime: "image/png".to_string(),
            data_base64: crate::base64_encode(&cropped),
            window: true,
            note: format!("cropped to the window at {},{} ({}×{})", position.x, position.y, size.width, size.height),
            rect: Some([
                position.x as i64,
                position.y as i64,
                size.width as i64,
                size.height as i64,
            ]),
            desktop: Some(desktop),
        }),
        Err(error) => Ok(WindowCapture {
            name: capture_name(),
            mime: "image/png".to_string(),
            data_base64: crate::base64_encode(&bytes),
            window: false,
            note: format!("could not crop the window ({error}); this is the whole desktop"),
            rect: Some([
                position.x as i64,
                position.y as i64,
                size.width as i64,
                size.height as i64,
            ]),
            desktop: None,
        }),
    }
}

/// Cuts `width`×`height` at `x`,`y` out of a PNG. Coordinates are physical pixels, which is
/// what both the capture and the window reports use; the rectangle is clamped to the image so
/// a window partly off-screen still yields what is visible.
fn crop_png(
    png: &[u8],
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> Result<(Vec<u8>, [u32; 2]), String> {
    let image = image::load_from_memory(png).map_err(|error| error.to_string())?;
    let desktop = [image.width(), image.height()];
    let left = x.max(0) as u32;
    let top = y.max(0) as u32;
    let right = (x + width as i32).clamp(0, desktop[0] as i32) as u32;
    let bottom = (y + height as i32).clamp(0, desktop[1] as i32) as u32;
    if right <= left || bottom <= top {
        return Err("the window is outside the captured screen".to_string());
    }
    let cropped = image.crop_imm(left, top, right - left, bottom - top);
    let mut out = Vec::new();
    cropped
        .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
        .map_err(|error| error.to_string())?;
    Ok((out, desktop))
}

#[cfg(windows)]
fn capture_to(path: &Path) -> Result<(), String> {
    use std::os::windows::process::CommandExt;

    let escaped = path.to_string_lossy().replace('\'', "''");
    let script = format!(
        "Add-Type -AssemblyName System.Windows.Forms; \
         Add-Type -AssemblyName System.Drawing; \
         $b=[System.Windows.Forms.SystemInformation]::VirtualScreen; \
         $bitmap=New-Object System.Drawing.Bitmap $b.Width,$b.Height; \
         $graphics=[System.Drawing.Graphics]::FromImage($bitmap); \
         $graphics.CopyFromScreen($b.Location,[System.Drawing.Point]::Empty,$b.Size); \
         $bitmap.Save('{escaped}',[System.Drawing.Imaging.ImageFormat]::Png); \
         $graphics.Dispose(); $bitmap.Dispose()"
    );

    let mut command = Command::new("powershell.exe");
    command.args([
        "-NoProfile",
        "-NonInteractive",
        "-WindowStyle",
        "Hidden",
        "-Command",
        &script,
    ]);
    command.creation_flags(0x0800_0000);
    let output = command
        .output()
        .map_err(|error| format!("screenshot: powershell: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "screenshot: powershell: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    if !path.is_file() {
        return Err("screenshot: no file was written".to_string());
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn capture_to(path: &Path) -> Result<(), String> {
    let output = Command::new("/usr/sbin/screencapture")
        .args(["-x", "-t", "png"])
        .arg(path)
        .output()
        .map_err(|error| format!("screenshot: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "screenshot: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    if !path.is_file() {
        return Err("screenshot: no file was written (screen recording permission?)".to_string());
    }
    Ok(())
}

/// Which capture tools are worth trying. On Wayland scrot and ImageMagick are left out on
/// purpose: they read the X11 root window, which under Wayland is a black void that they
/// capture "successfully". Offering them means a valid, black screenshot and no error.
#[cfg(target_os = "linux")]
fn capture_candidates(wayland: bool) -> Vec<(&'static str, &'static [&'static str])> {
    const GRIM: (&str, &[&str]) = ("grim", &[]);
    const GNOME: (&str, &[&str]) = ("gnome-screenshot", &["-f"]);
    const SPECTACLE: (&str, &[&str]) = ("spectacle", &["-b", "-n", "-o"]);
    if wayland {
        vec![GRIM, GNOME, SPECTACLE]
    } else {
        vec![
            GRIM,
            GNOME,
            SPECTACLE,
            ("scrot", &[]),
            ("import", &["-window", "root"]),
        ]
    }
}

/// Is this build running inside a sandbox? Flatpak sets `FLATPAK_ID`, Snap sets `SNAP`.
#[cfg(target_os = "linux")]
pub(crate) fn in_sandbox() -> bool {
    std::env::var_os("FLATPAK_ID").is_some() || std::env::var_os("SNAP").is_some()
}

/// Wayland is identified by either signal: a nested compositor may not set the session type,
/// and a session can carry XDG_SESSION_TYPE while still only having a Wayland socket.
#[cfg(target_os = "linux")]
fn is_wayland_session(has_wayland_socket: bool, session_type: Option<&str>) -> bool {
    has_wayland_socket
        || session_type
            .map(|value| value.eq_ignore_ascii_case("wayland"))
            .unwrap_or(false)
}

#[cfg(target_os = "linux")]
fn capture_to(path: &Path) -> Result<(), String> {
    use std::process::Stdio;

    /// What a tool did, so the final error can name the real reason instead of "not found".
    enum Attempt {
        Captured,
        Failed,
        Flat,
    }

    fn attempt(program: &str, args: &[&str], path: &Path) -> Attempt {
        // A capture tool that cannot work here (grim on a compositor without the screencopy
        // protocol, for example) is expected: the next one is tried. Its stderr would land in
        // the app's terminal and claim a failure that did not happen — the screenshot was
        // taken by the tool after it. Only the first successful tool is worth a log line.
        let _ = fs::remove_file(path);
        let mut command = Command::new(program);
        command
            .args(args)
            .arg(path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        // ImageMagick reads ARGV0 as its program name and a bundled LD_LIBRARY_PATH breaks it
        // with libraries that are not the system's. See `child_env`.
        crate::child_env::sanitize(&mut command);
        let Ok(status) = command.status() else {
            return Attempt::Failed;
        };
        let written = status.success()
            && path.is_file()
            && fs::metadata(path).map(|meta| meta.len() > 0).unwrap_or(false);
        if !written {
            return Attempt::Failed;
        }
        // Exit code 0 and a file are not proof of a picture. An X11 tool on a Wayland session
        // sees only the empty XWayland root: it succeeds and writes a perfectly valid,
        // perfectly black image. Accepting that means handing the model a black rectangle and
        // calling it a screenshot.
        if is_a_flat_image(path) {
            return Attempt::Flat;
        }
        Attempt::Captured
    }

    let wayland = is_wayland_session(
        std::env::var_os("WAYLAND_DISPLAY").is_some(),
        std::env::var("XDG_SESSION_TYPE").ok().as_deref(),
    );
    let candidates = capture_candidates(wayland);

    let mut flat: Vec<&str> = Vec::new();
    let mut failed: Vec<&str> = Vec::new();

    // A sandbox has no capture tool: the portal is the only way out, and it is the sanctioned
    // one — the compositor takes the picture and the desktop decides. Tried first there,
    // because a tool that does not exist is not worth four failed attempts.
    #[cfg(target_os = "linux")]
    if in_sandbox() {
        match crate::portal::screenshot(path) {
            Ok(()) => {
                crate::log_line!("screenshot: captured through the desktop portal");
                if !is_a_flat_image(path) {
                    return Ok(());
                }
                crate::log_line!("screenshot: the portal produced an empty image, ignored");
                flat.push("portal");
            }
            Err(error) => {
                crate::log_line!("screenshot: portal: {error}");
                failed.push("portal");
            }
        }
    }

    for (program, args) in &candidates {
        match attempt(program, args, path) {
            Attempt::Captured => {
                crate::log_line!("screenshot: captured with {program}");
                return Ok(());
            }
            Attempt::Flat => {
                // Visible in the Logs tab: a tool claiming success while writing a blank
                // picture is exactly the confusion this check exists to end.
                crate::log_line!("screenshot: {program} produced an empty image, ignored");
                flat.push(program);
            }
            Attempt::Failed => failed.push(program),
        }
    }

    if !flat.is_empty() {
        // Name the trap instead of returning an image that looks like a result.
        return Err(format!(
            "screenshot: {} produced an empty {} image{}. On a Wayland session only Wayland-capable \
             tools can see the screen; install grim or make sure the desktop portal allows capture.",
            flat.join(", "),
            if wayland { "black" } else { "blank" },
            if failed.is_empty() {
                String::new()
            } else {
                format!(" ({} did not run)", failed.join(", "))
            },
        ));
    }

    Err(if wayland {
        format!(
            "screenshot: no Wayland capture tool worked ({}). Install grim, or allow screen \
             capture for this app in the desktop portal.",
            failed.join(", ")
        )
    } else {
        format!(
            "screenshot: no capture tool worked ({}). Install grim, gnome-screenshot, spectacle, \
             scrot or ImageMagick.",
            failed.join(", ")
        )
    })
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    fn write_image(name: &str, paint: impl Fn(u32, u32) -> [u8; 3]) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(name);
        let mut image = image::RgbImage::new(400, 300);
        for (x, y, pixel) in image.enumerate_pixels_mut() {
            *pixel = image::Rgb(paint(x, y));
        }
        image.save(&path).unwrap();
        path
    }

    /// The bug this guards: scrot on a Wayland session exits 0 and writes a valid, uniformly
    /// black picture. It must not be handed on as a screenshot.
    #[test]
    fn a_uniform_capture_counts_as_flat() {
        let path = write_image("ds-ide-flat.png", |_, _| [0, 0, 0]);
        assert!(is_a_flat_image(&path));
        let _ = std::fs::remove_file(&path);

        // A blank but not black surface is just as useless.
        let path = write_image("ds-ide-white.png", |_, _| [250, 250, 250]);
        assert!(is_a_flat_image(&path));
        let _ = std::fs::remove_file(&path);
    }

    /// scrot and ImageMagick are X11-only. Offering them on Wayland is what produced the black
    /// screenshots, so the Wayland list must not contain them.
    #[test]
    fn wayland_never_offers_x11_only_tools() {
        let wayland = capture_candidates(true);
        let names: Vec<&str> = wayland.iter().map(|(name, _)| *name).collect();
        assert!(!names.contains(&"scrot"), "{names:?}");
        assert!(!names.contains(&"import"), "{names:?}");
        assert!(names.contains(&"grim"), "{names:?}");

        // On X11 they are still worth a try.
        let x11 = capture_candidates(false);
        let names: Vec<&str> = x11.iter().map(|(name, _)| *name).collect();
        assert!(names.contains(&"scrot"), "{names:?}");

        assert!(is_wayland_session(true, None));
        assert!(is_wayland_session(false, Some("wayland")));
        assert!(!is_wayland_session(false, Some("x11")));
    }

    /// A real desktop has text and borders; a single differing patch is enough.
    #[test]
    fn a_capture_with_content_is_not_flat() {
        let path = write_image("ds-ide-real.png", |x, y| {
            if (200..260).contains(&x) && (100..140).contains(&y) {
                [220, 220, 220]
            } else {
                [12, 12, 14]
            }
        });
        assert!(!is_a_flat_image(&path));
        let _ = std::fs::remove_file(&path);
    }
}

/// True when the picture carries no information: every sampled pixel is (nearly) the same
/// colour. A real desktop has text, borders and a taskbar, so a uniform grid means the capture
/// tool photographed an empty surface. Sampling keeps this cheap on a 4K screen.
#[cfg(target_os = "linux")]
fn is_a_flat_image(path: &Path) -> bool {
    let Ok(image) = image::open(path) else {
        // Undecodable is not "flat"; the caller's size check already guards empty files.
        return false;
    };
    let (width, height) = (image.width(), image.height());
    if width < 32 || height < 32 {
        return false;
    }
    let rgb = image.to_rgb8();
    let mut first: Option<[u8; 3]> = None;
    // A 12 × 12 grid across the screen: enough to catch a single window or a taskbar.
    for row in 0..12u32 {
        for column in 0..12u32 {
            let x = (width - 1) * column / 11;
            let y = (height - 1) * row / 11;
            let pixel = rgb.get_pixel(x, y);
            let value = [pixel[0], pixel[1], pixel[2]];
            match first {
                None => first = Some(value),
                Some(reference) => {
                    let differs = (0..3).any(|index| reference[index].abs_diff(value[index]) > 3);
                    if differs {
                        return false;
                    }
                }
            }
        }
    }
    true
}