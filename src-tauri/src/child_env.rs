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

/// A `:`-separated path a child should see: the user's own entries, without the bundle's.
///
/// `None` means the variable should not be set at all. Entries the user set before launching
/// the app are kept — only the bundle's own directories are removed.
fn without_bundle(current: Option<&str>, root: Option<&str>) -> Option<String> {
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

/// A single directory a child should see, or `None` when it belongs to the bundle.
fn without_bundle_single(current: Option<&str>, root: Option<&str>) -> Option<String> {
    let current = current?;
    if belongs_to_bundle(current, root) {
        None
    } else {
        Some(current.to_string())
    }
}

/// The variables a child should get, and the ones it must lose. One list, so the synchronous
/// and the asynchronous command types cannot drift apart.
pub fn environment_for(
    library_path: Option<&str>,
    python_path: Option<&str>,
    python_home: Option<&str>,
    root: Option<&str>,
) -> Vec<(&'static str, Option<String>)> {
    let mut ops: Vec<(&'static str, Option<String>)> = Vec::new();
    ops.push(("LD_LIBRARY_PATH", without_bundle(library_path, root)));
    // The same disease in a different variable, and a nastier symptom: the AppImage sets
    // PYTHONHOME to its own `usr/`, which is not a Python installation. `python3 --version`
    // still works (it needs no `encodings`), but every real call dies with
    // "Fatal Python error: Failed to import encodings module" — so the companion has to
    // unset both by hand for every Python command it runs.
    ops.push(("PYTHONPATH", without_bundle(python_path, root)));
    ops.push(("PYTHONHOME", without_bundle_single(python_home, root)));
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
    let library_path = std::env::var("LD_LIBRARY_PATH").ok();
    let python_path = std::env::var("PYTHONPATH").ok();
    let python_home = std::env::var("PYTHONHOME").ok();
    environment_for(
        library_path.as_deref(),
        python_path.as_deref(),
        python_home.as_deref(),
        root.as_deref(),
    )
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
    without_bundle(current.as_deref(), root.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bundles_own_library_directories_are_dropped() {
        let root = "/tmp/.mount_abc";
        let current = format!("/tmp/.mount_abc/usr/lib:/opt/vendor/lib:/tmp/.mount_abc/usr/lib64:");

        let cleaned = without_bundle(Some(&current), Some(root)).unwrap();
        assert_eq!(cleaned, "/opt/vendor/lib:", "the user's own entry stays");

        // Only the bundle: the variable goes away entirely.
        let only_bundle = "/tmp/.mount_abc/usr/lib:/tmp/.mount_abc/lib";
        assert_eq!(without_bundle(Some(only_bundle), Some(root)), None);

        // No bundle at all: untouched.
        assert_eq!(
            without_bundle(Some("/opt/vendor/lib"), None).as_deref(),
            Some("/opt/vendor/lib")
        );

        // Empty and unset are both "nothing to set".
        assert_eq!(without_bundle(Some(""), Some(root)), None);
        assert_eq!(without_bundle(None, Some(root)), None);
    }

    /// The caller gets a complete instruction list, not a yes/no: which value to set and
    /// which variable to drop. Both command types apply exactly this list.
    #[test]
    fn the_environment_removes_the_bundle_markers() {
        let ops = environment_for(
            Some("/tmp/.mount_abc/usr/lib:/opt/vendor/lib"),
            None,
            None,
            Some("/tmp/.mount_abc"),
        );
        let find = |name: &str| ops.iter().find(|(key, _)| *key == name).map(|(_, value)| value.clone());

        assert_eq!(find("LD_LIBRARY_PATH"), Some(Some("/opt/vendor/lib".to_string())));
        assert_eq!(find("ARGV0"), Some(None), "grown programs read this as their name");
        assert_eq!(find("APPDIR"), Some(None));
        assert_eq!(find("APPIMAGE"), Some(None));

        // Outside an AppImage the variables are still cleared: they mean nothing then either.
        let plain = environment_for(None, None, None, None);
        assert_eq!(plain.len(), 7, "LD_LIBRARY_PATH, PYTHONPATH, PYTHONHOME + four markers");
    }

    /// Without `APPDIR` the mount convention still names the bundle: `/tmp/.mount_…` is never
    /// a user's library directory.
    #[test]
    fn a_mount_directory_counts_as_the_bundle() {
        let cleaned = without_bundle(
            Some("/tmp/.mount_abc123/usr/lib:/usr/local/lib"),
            None,
        );
        assert_eq!(cleaned.as_deref(), Some("/usr/local/lib"));
    }

    /// The AppImage sets PYTHONHOME to its own `usr/`, which is not a Python installation:
    /// `python3 --version` still answers, every real call dies with "Failed to import encodings
    /// module". A user's own Python setup is left alone.
    #[test]
    fn the_bundles_python_variables_are_dropped() {
        let root = "/tmp/.mount_abc";
        let ops = environment_for(
            None,
            Some("/tmp/.mount_abc/usr/share/pyshared/:/home/me/lib/python"),
            Some("/tmp/.mount_abc/usr/"),
            Some(root),
        );
        let find = |name: &str| {
            ops.iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| value.clone())
        };

        assert_eq!(
            find("PYTHONPATH"),
            Some(Some("/home/me/lib/python".to_string())),
            "the user's own path survives"
        );
        assert_eq!(
            find("PYTHONHOME"),
            Some(None),
            "a PYTHONHOME inside the bundle is removed, not passed on"
        );

        // Somebody with their own Python installation keeps it.
        let own = environment_for(None, None, Some("/usr"), Some(root));
        let home = own
            .iter()
            .find(|(key, _)| *key == "PYTHONHOME")
            .map(|(_, value)| value.clone());
        assert_eq!(home, Some(Some("/usr".to_string())));
    }

    /// The whole chain on a real child process: a shell that would print the variables must not
    /// see them. The unit test above checks the list; this checks that a child really gets it.
    #[tokio::test]
    async fn a_child_process_does_not_see_the_bundles_python_variables() {
        let mut command = tokio::process::Command::new("sh");
        command
            .arg("-c")
            .arg("echo \"home=${PYTHONHOME:-none} path=${PYTHONPATH:-none}\"");
        // What the bundle would have set.
        command.env("PYTHONHOME", "/tmp/.mount_abc/usr/");
        command.env("PYTHONPATH", "/tmp/.mount_abc/usr/share/pyshared/");
        for (name, value) in environment_for(
            None,
            Some("/tmp/.mount_abc/usr/share/pyshared/"),
            Some("/tmp/.mount_abc/usr/"),
            Some("/tmp/.mount_abc"),
        ) {
            match value {
                Some(value) => command.env(name, value),
                None => command.env_remove(name),
            };
        }

        let output = command.output().await.expect("a shell");
        let text = String::from_utf8_lossy(&output.stdout).to_string();
        assert!(text.contains("home=none"), "PYTHONHOME must be gone: {text}");
        assert!(text.contains("path=none"), "PYTHONPATH must be gone: {text}");
    }

    /// A path that merely shares a prefix with the bundle root is a different directory.
    /// (A `/.mount_…` path is always the bundle, whatever the root says — see below.)
    #[test]
    fn a_similar_prefix_is_not_the_bundle() {
        let cleaned = without_bundle(
            Some("/opt/app-other/lib:/opt/app/usr/lib"),
            Some("/opt/app"),
        );
        assert_eq!(cleaned.as_deref(), Some("/opt/app-other/lib"));
    }
}
