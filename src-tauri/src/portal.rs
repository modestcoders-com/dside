//! Screenshots through the desktop portal.
//!
//! Inside a Flatpak the app cannot call a capture tool: `grim`, `spectacle` and `scrot` are not
//! part of the runtime, and the sandbox has no way to reach them. Without a fallback the
//! companion would be blind there — it could not look at what the user is looking at, which is
//! half of what a screenshot is for.
//!
//! The desktop portal is the sanctioned way out: the compositor takes the picture, writes a
//! file, and hands back its URI. The app asks, the user's desktop decides.
//!
//! Two details matter. The answer is a *directed* signal to the caller's own connection, so it
//! has to be received in-process — a `gdbus monitor` in a child process never sees it. And the
//! answer can arrive immediately, so the subscription has to exist before the request is sent;
//! `handle_token` makes the request path predictable, which is what allows that order.

use std::collections::HashMap;
use std::path::Path;

use zbus::blocking::{Connection, Proxy};
use zbus::zvariant::{OwnedObjectPath, OwnedValue, Value};

/// Takes a screenshot through `org.freedesktop.portal.Screenshot` and copies it to `target`.
pub fn screenshot(target: &Path) -> Result<(), String> {
    let connection = Connection::session().map_err(|error| format!("portal: {error}"))?;

    // The handle is built by the portal from our own bus name and the token we choose, so we
    // know the path before the call — and can listen before answering.
    let unique = connection
        .unique_name()
        .ok_or_else(|| "portal: the connection has no bus name".to_string())?
        .to_string();
    let sender = unique.trim_start_matches(':').replace('.', "_");
    let token = format!("dside{}", std::process::id());
    let handle = format!("/org/freedesktop/portal/desktop/request/{sender}/{token}");

    // The blocking signal iterator has no timeout, and the answer must be received on *our*
    // connection — a child process would never see it. So one worker thread owns the wait,
    // and a handshake makes sure the match rule exists before the request goes out.
    let (ready_tx, ready_rx) = std::sync::mpsc::channel::<Result<(), String>>();
    let (answer_tx, answer_rx) = std::sync::mpsc::channel::<zbus::Message>();
    let worker_connection = connection.clone();
    let worker_handle = handle.clone();
    std::thread::spawn(move || {
        let proxy = match Proxy::new(
            &worker_connection,
            "org.freedesktop.portal.Desktop",
            worker_handle.as_str(),
            "org.freedesktop.portal.Request",
        ) {
            Ok(proxy) => proxy,
            Err(error) => {
                let _ = ready_tx.send(Err(format!("portal: request proxy: {error}")));
                return;
            }
        };
        let mut signals = match proxy.receive_signal("Response") {
            Ok(signals) => signals,
            Err(error) => {
                let _ = ready_tx.send(Err(format!(
                    "portal: could not listen for the answer: {error}"
                )));
                return;
            }
        };
        let _ = ready_tx.send(Ok(()));
        if let Some(message) = signals.next() {
            let _ = answer_tx.send(message);
        }
    });

    match ready_rx.recv_timeout(std::time::Duration::from_secs(5)) {
        Ok(Ok(())) => {}
        Ok(Err(error)) => return Err(error),
        Err(_) => {
            return Err(
                "portal: could not listen for the answer — the session bus did not respond"
                    .to_string(),
            )
        }
    }

    let desktop = Proxy::new(
        &connection,
        "org.freedesktop.portal.Desktop",
        "/org/freedesktop/portal/desktop",
        "org.freedesktop.portal.Screenshot",
    )
    .map_err(|error| format!("portal: screenshot proxy: {error}"))?;

    let mut options: HashMap<&str, Value> = HashMap::new();
    options.insert("handle_token", Value::from(token.as_str()));
    // Not interactive: no region selection, no delay. The whole screen, now.
    options.insert("interactive", Value::from(false));

    let _: OwnedObjectPath = desktop
        .call("Screenshot", &("", options))
        .map_err(|error| format!("portal: the request was refused: {error}"))?;

    // The portal answers with a signal; give the compositor time to take the picture, but not
    // forever. Twenty seconds is generous for a screen grab and short enough to report back.
    let message = answer_rx
        .recv_timeout(std::time::Duration::from_secs(20))
        .map_err(|_| {
            "portal: no answer within 20 s — the desktop did not confirm the screenshot"
                .to_string()
        })?;

    let (code, results): (u32, HashMap<String, OwnedValue>) = message
        .body()
        .deserialize()
        .map_err(|error| format!("portal: unreadable answer: {error}"))?;
    if code != 0 {
        return Err(if code == 1 {
            "portal: the user cancelled the screenshot".to_string()
        } else {
            format!("portal: the screenshot failed (code {code})")
        });
    }

    let uri = results
        .get("uri")
        .and_then(|value| value.downcast_ref::<&str>().ok())
        .ok_or_else(|| "portal: the answer carried no readable file name".to_string())?
        .to_string();

    let source = file_uri_to_path(&uri)
        .ok_or_else(|| format!("portal: unexpected file name {uri}"))?;
    std::fs::copy(&source, target)
        .map_err(|error| format!("portal: could not read the picture: {error}"))?;
    // The picture in the user's folder is the portal's copy, not ours to keep.
    let _ = std::fs::remove_file(&source);
    Ok(())
}

/// `file:///home/…/Screenshot.png` → `/home/…/Screenshot.png`.
fn file_uri_to_path(uri: &str) -> Option<std::path::PathBuf> {
    let rest = uri.strip_prefix("file://")?;
    // `file:///home/…` carries an empty authority; `file://host/path` is not a local file.
    let path = rest.strip_prefix('/').map(|tail| format!("/{tail}"))?;
    Some(std::path::PathBuf::from(crate::preview::percent_decode(&path)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_uri_becomes_a_path() {
        assert_eq!(
            file_uri_to_path("file:///home/someone/Bilder/Screenshot_1.png").unwrap(),
            std::path::PathBuf::from("/home/someone/Bilder/Screenshot_1.png")
        );
        // Spaces and umlauts are percent-encoded by the portal.
        assert_eq!(
            file_uri_to_path("file:///home/someone/Meine%20Bilder/S%C3%BCd.png").unwrap(),
            std::path::PathBuf::from("/home/someone/Meine Bilder/Süd.png")
        );
        // Not a local file, not our business.
        assert!(file_uri_to_path("file://otherhost/x.png").is_none());
        assert!(file_uri_to_path("https://example.com/x.png").is_none());
    }
}
