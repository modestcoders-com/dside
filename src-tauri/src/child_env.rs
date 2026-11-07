//! The environment a child process gets.
//!
//! An AppImage mount exists only for the app itself. It sets `LD_LIBRARY_PATH` to its own
//! `usr/lib` so its bundled libraries win — which is right for the app and wrong for every
//! program it starts. The system `git` then loads the bundled `libnghttp2` into the system
//! `libcurl`, and `git push` over HTTPS dies with a symbol lookup error:
//!
//! ```text
//! git-remote-https: symbol lookup error: /usr/lib/libcurl.so.4:
//! undefined symbol: nghttp2_option_set_no_rfc9113_leading_and_trailing_ws_validation
//! ```
//!
//! The same applies to `ARGV0`, which some programs (ImageMagick, misusing it as the program
//! name) take literally — the reason screenshots and `magick` failed in a packaged build.
//!
//! So: children get the environment of the *system*, not of the bundle.

use std::process::Command;

/// Paths that belong to the running AppImage (or a similar self-contained bundle).
fn bundle_root() -> Option<String> {
    std::env::var("APPDIR")
        .ok()
        .filter(|value| !value.trim().is_empty())
}

/// Is this `LD_LIBRARY_PATH` entry inside the bundle?
///
/// Two markers: `APPDIR`, which the AppImage sets itself, and the mount convention
/// `/tmp/.mount_<random>`. The second matters because a launcher that repacks the app (or a
/// test that starts the binary by hand) may pass the library path without `APPDIR`.
fn belongs_to_bundle(entry: &str, root: Option<&str>) -> bool {
    if entry.is_empty() {
        return false;
    }
    if entry.contains("/.mount_") {
        return true;
    }
    match root {
        Some(root) => entry == root || entry.starts_with(&format!("{root}/")),
        None => false,
    }
}

/// The library path a child should see: the user's own entries, without the bundle's.
///
/// `None` means the variable should not be set at all. Entries the user set before launching
/// the app are kept — only the bundle's own directories are removed.
fn child_library_path(current: Option<&str>, root: Option<&str>) -> Option<String> {
    let current = current?;
    let kept: Vec<&str> = current
        .split(':')
        .filter(|entry| !belongs_to_bundle(entry, root))
        .collect();
    let joined = kept.join(":");
    if joined.trim_matches(':').is_empty() {
        None
    } else {
        Some(joined)
    }
}

/// The variables a child should get, and the ones it must lose. One list, so the synchronous
/// and the asynchronous command types cannot drift apart.
pub fn environment_for(
    library_path: Option<&str>,
    root: Option<&str>,
) -> Vec<(&'static str, Option<String>)> {
    let mut ops: Vec<(&'static str, Option<String>)> = Vec::new();
    ops.push((
        "LD_LIBRARY_PATH",
        child_library_path(library_path, root),
    ));
    // Only the app is the AppImage. A child that reads ARGV0 as its program name — which is
    // what ImageMagick does — fails or reports nonsense otherwise.
    for name in ["ARGV0", "APPIMAGE", "APPDIR", "OWD"] {
        ops.push((name, None));
    }
    ops
}

/// The same, read from this process — what the command builders call.
pub fn environment() -> Vec<(&'static str, Option<String>)> {
    let root = bundle_root();
    let current = std::env::var("LD_LIBRARY_PATH").ok();
    environment_for(current.as_deref(), root.as_deref())
}

/// Cleans the environment of a `std::process::Command`.
pub fn sanitize(command: &mut Command) {
    for (name, value) in environment() {
        match value {
            Some(value) => command.env(name, value),
            None => command.env_remove(name),
        };
    }
}

/// Same for a `tokio::process::Command`.
pub fn sanitize_async(command: &mut tokio::process::Command) {
    for (name, value) in environment() {
        match value {
            Some(value) => command.env(name, value),
            None => command.env_remove(name),
        };
    }
}

/// The same rule for a shell command builder (`portable-pty`), which is not a `Command`.
pub fn sanitized_library_path() -> Option<String> {
    let root = bundle_root();
    let current = std::env::var("LD_LIBRARY_PATH").ok();
    child_library_path(current.as_deref(), root.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bundles_own_library_directories_are_dropped() {
        let root = "/tmp/.mount_abc";
        let current = format!("/tmp/.mount_abc/usr/lib:/opt/vendor/lib:/tmp/.mount_abc/usr/lib64:");

        let cleaned = child_library_path(Some(&current), Some(root)).unwrap();
        assert_eq!(cleaned, "/opt/vendor/lib:", "the user's own entry stays");

        // Only the bundle: the variable goes away entirely.
        let only_bundle = "/tmp/.mount_abc/usr/lib:/tmp/.mount_abc/lib";
        assert_eq!(child_library_path(Some(only_bundle), Some(root)), None);

        // No bundle at all: untouched.
        assert_eq!(
            child_library_path(Some("/opt/vendor/lib"), None).as_deref(),
            Some("/opt/vendor/lib")
        );

        // Empty and unset are both "nothing to set".
        assert_eq!(child_library_path(Some(""), Some(root)), None);
        assert_eq!(child_library_path(None, Some(root)), None);
    }

    /// The caller gets a complete instruction list, not a yes/no: which value to set and
    /// which variable to drop. Both command types apply exactly this list.
    #[test]
    fn the_environment_removes_the_bundle_markers() {
        let ops = environment_for(
            Some("/tmp/.mount_abc/usr/lib:/opt/vendor/lib"),
            Some("/tmp/.mount_abc"),
        );
        let find = |name: &str| ops.iter().find(|(key, _)| *key == name).map(|(_, value)| value.clone());

        assert_eq!(find("LD_LIBRARY_PATH"), Some(Some("/opt/vendor/lib".to_string())));
        assert_eq!(find("ARGV0"), Some(None), "grown programs read this as their name");
        assert_eq!(find("APPDIR"), Some(None));
        assert_eq!(find("APPIMAGE"), Some(None));

        // Outside an AppImage the variables are still cleared: they mean nothing then either.
        let plain = environment_for(None, None);
        assert_eq!(plain.len(), 5);
    }

    /// Without `APPDIR` the mount convention still names the bundle: `/tmp/.mount_…` is never
    /// a user's library directory.
    #[test]
    fn a_mount_directory_counts_as_the_bundle() {
        let cleaned = child_library_path(
            Some("/tmp/.mount_abc123/usr/lib:/usr/local/lib"),
            None,
        );
        assert_eq!(cleaned.as_deref(), Some("/usr/local/lib"));
    }

    /// A path that merely shares a prefix with the bundle root is a different directory.
    /// (A `/.mount_…` path is always the bundle, whatever the root says — see below.)
    #[test]
    fn a_similar_prefix_is_not_the_bundle() {
        let cleaned = child_library_path(
            Some("/opt/app-other/lib:/opt/app/usr/lib"),
            Some("/opt/app"),
        );
        assert_eq!(cleaned.as_deref(), Some("/opt/app-other/lib"));
    }
}
