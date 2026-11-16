//! Is there a newer release than the one running?
//!
//! Two ways to update exist and they are not the same thing:
//! - The AppImage updates itself through the `.zsync` control file that ships with each release
//!   (Shelly does that, verified end to end on 2026-11-15).
//! - `.deb`, `.rpm`, the Windows installer and the macOS disk image are replaced by installing a
//!   newer package. Nothing in the running program can do that safely by itself.
//!
//! So this check only answers "is there something newer", and the interface offers the release
//! page. Downloading and starting an installer from inside a running application is exactly the
//! kind of cleverness that breaks on somebody else's machine.
//!
//! It is a courtesy, never a critical path: no network, a rate limit or a malformed answer all
//! end in "no answer", and the app carries on without saying anything.

use serde::Serialize;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const REPO: &str = "modestcoders-com/dside";

/// GitHub answers sixty unauthenticated requests per hour and per address. One check per start
/// would be far below that, but the interface asks again whenever it remounts, so the answer is
/// kept for a while instead of asking eleven times in a minute.
const FRESH: Duration = Duration::from_secs(6 * 60 * 60);
const TIMEOUT: Duration = Duration::from_secs(8);

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    /// The version that is running, so the interface never has to ask twice.
    pub current: String,
    pub latest: String,
    pub url: String,
    pub available: bool,
}

static CACHE: Mutex<Option<(Instant, Option<UpdateInfo>)>> = Mutex::new(None);

/// Compares two version strings numerically. `v0.6.10` is newer than `v0.6.9`, which a plain
/// string comparison would get wrong. Anything unparsable counts as zero, so a tag like
/// `v0.7.0-beta.1` still compares as `0.7.0.1`.
pub(crate) fn is_newer(latest: &str, current: &str) -> bool {
    fn parts(version: &str) -> Vec<u64> {
        version
            .trim()
            .trim_start_matches(['v', 'V'])
            .split(['.', '-', '+'])
            .map(|part| {
                // `0.6.5-beta` and `0.6.5+build` must not become "0.6.5.0.0" by accident in a
                // way that outranks 0.6.5 itself; only numeric segments count.
                part.parse::<u64>().unwrap_or(0)
            })
            .collect()
    }
    let (newer, older) = (parts(latest), parts(current));
    for index in 0..newer.len().max(older.len()) {
        let left = newer.get(index).copied().unwrap_or(0);
        let right = older.get(index).copied().unwrap_or(0);
        if left != right {
            return left > right;
        }
    }
    false
}

async fn fetch() -> Option<UpdateInfo> {
    let client = reqwest::Client::builder()
        // GitHub rejects requests without a user agent.
        .user_agent(concat!("DS-IDE/", env!("CARGO_PKG_VERSION")))
        .timeout(TIMEOUT)
        .build()
        .ok()?;
    let response = client
        .get(format!("https://api.github.com/repos/{REPO}/releases/latest"))
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .ok()?;
    if !response.status().is_success() {
        return None;
    }
    let body: serde_json::Value = response.json().await.ok()?;
    let tag = body.get("tag_name")?.as_str()?.trim().to_string();
    let url = body.get("html_url")?.as_str()?.to_string();
    if tag.is_empty() || url.is_empty() {
        return None;
    }
    let current = env!("CARGO_PKG_VERSION").to_string();
    let available = is_newer(&tag, &current);
    Some(UpdateInfo {
        current,
        latest: tag.trim_start_matches(['v', 'V']).to_string(),
        url,
        available,
    })
}

/// `force` skips the cache — for a "check again" in the interface, not for a normal start.
#[tauri::command]
pub async fn update_check(force: Option<bool>) -> Result<Option<UpdateInfo>, String> {
    if !force.unwrap_or(false) {
        if let Ok(cache) = CACHE.lock() {
            if let Some((at, answer)) = cache.as_ref() {
                if at.elapsed() < FRESH {
                    return Ok(answer.clone());
                }
            }
        }
    }
    let answer = fetch().await;
    if let Ok(mut cache) = CACHE.lock() {
        *cache = Some((Instant::now(), answer.clone()));
    }
    match &answer {
        Some(info) if info.available => crate::log_line!(
            "update: {} is available, running {}",
            info.latest,
            info.current
        ),
        Some(info) => crate::log_line!("update: {} is the newest, nothing to do", info.latest),
        // Not an error: no network, a rate limit or an old release page. The app just does not
        // know, and saying nothing is better than a warning nobody can act on.
        None => crate::log_line!("update: could not be checked"),
    }
    Ok(answer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_newer_version_is_recognised() {
        assert!(is_newer("v0.6.6", "0.6.5"));
        assert!(is_newer("0.7.0", "0.6.9"));
        // The case a string comparison gets wrong.
        assert!(is_newer("v0.6.10", "0.6.9"));
        assert!(!is_newer("v0.6.5", "0.6.5"));
        assert!(!is_newer("v0.6.4", "0.6.5"));
    }

    #[test]
    fn tags_and_suffixes_do_not_break_the_comparison() {
        assert!(is_newer("v1.0.0", "0.9.9"));
        assert!(!is_newer("v0.6.5", "0.6.5-beta"));
        assert!(is_newer("v0.6.6-beta", "0.6.5"));
        assert!(!is_newer("", "0.6.5"));
        assert!(!is_newer("nonsense", "0.6.5"));
    }
}
