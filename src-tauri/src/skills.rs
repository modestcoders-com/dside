use std::path::{Path, PathBuf};

use serde::Serialize;

/// Where skills live. Deliberately outside the app bundle so the user can drop files in
/// without rebuilding: one folder per skill, with its notes in `<dir>/SKILL.md`.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillSummary {
    pub name: String,
    pub description: String,
    pub path: String,
}

pub fn skills_root() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".ds-ide").join("skills")
}

pub fn ensure_root() -> PathBuf {
    let root = skills_root();
    let _ = std::fs::create_dir_all(&root);
    root
}

/// Read a `key: value` pair from YAML-ish front matter. Unknown structures are ignored on
/// purpose: a malformed skill should not take the whole list down.
fn front_matter_field(raw: &str, key: &str) -> Option<String> {
    let mut inside = false;
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed == "---" {
            if inside {
                break;
            }
            inside = true;
            continue;
        }
        if !inside {
            continue;
        }
        if let Some(value) = trimmed.strip_prefix(&format!("{key}:")) {
            return Some(value.trim().trim_matches('"').trim().to_string());
        }
    }
    None
}

fn fallback_name(path: &Path) -> String {
    path.parent()
        .and_then(Path::file_name)
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default()
}

fn collect(paths: &mut Vec<PathBuf>, dir: &Path, depth: usize) {
    if depth > 3 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() {
            collect(paths, &path, depth + 1);
            continue;
        }
        if entry.file_name().to_string_lossy().eq_ignore_ascii_case("SKILL.md") {
            paths.push(path);
        }
    }
}

fn skill_paths() -> Vec<PathBuf> {
    let root = skills_root();
    let mut paths = Vec::new();
    collect(&mut paths, &root, 0);
    paths
}

fn summarise(path: &Path) -> Option<SkillSummary> {
    let raw = std::fs::read_to_string(path).ok()?;
    Some(SkillSummary {
        name: front_matter_field(&raw, "name").unwrap_or_else(|| fallback_name(path)),
        description: front_matter_field(&raw, "description").unwrap_or_default(),
        path: path.to_string_lossy().to_string(),
    })
}

pub fn list() -> Vec<SkillSummary> {
    ensure_root();
    let mut skills: Vec<SkillSummary> = skill_paths().iter().filter_map(|path| summarise(path)).collect();
    skills.sort_by(|a, b| a.name.cmp(&b.name));
    skills
}

pub fn find(name: &str) -> Option<PathBuf> {
    for path in skill_paths() {
        let matches = summarise(&path)
            .map(|summary| summary.name == name)
            .unwrap_or(false);
        if matches {
            return Some(path);
        }
    }
    None
}

#[tauri::command]
pub fn skills_list() -> Vec<SkillSummary> {
    list()
}

#[tauri::command]
pub fn skills_root_path() -> String {
    ensure_root().to_string_lossy().to_string()
}

/// A skill folder name the file system and the readers can live with. Anything else would
/// either escape the skills folder or produce a folder nobody can find again.
fn safe_slug(name: &str) -> Result<String, String> {
    let slug: String = name
        .trim()
        .to_lowercase()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect();
    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() {
        return Err("a skill needs a name with letters or digits".to_string());
    }
    Ok(slug)
}

/// A skill is one `SKILL.md` in its own folder. Creating and editing are the same operation:
/// writing the file. The front matter is what makes it findable.
pub fn write_skill(name: &str, description: &str, body: &str) -> Result<String, String> {
    write_skill_at(&ensure_root(), name, description, body)
}

/// The same thing with the folder passed in, so a test can write into a temporary one instead
/// of into the user's real skills.
fn write_skill_at(
    root: &Path,
    name: &str,
    description: &str,
    body: &str,
) -> Result<String, String> {
    let slug = safe_slug(name)?;
    let folder = root.join(&slug);
    std::fs::create_dir_all(&folder).map_err(|error| error.to_string())?;

    let description = description.trim();
    let content = format!(
        "---\nname: {slug}\ndescription: {description}\n---\n\n{}\n",
        body.trim()
    );
    let path = folder.join("SKILL.md");
    std::fs::write(&path, content).map_err(|error| error.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn skill_read(path: String) -> Result<String, String> {
    // Only inside the skills folder: the panel edits skills, not arbitrary files.
    let root = ensure_root();
    let candidate = PathBuf::from(&path);
    let absolute = candidate
        .canonicalize()
        .or_else(|_| Ok::<PathBuf, std::io::Error>(candidate.clone()))
        .map_err(|error| error.to_string())?;
    let root = root.canonicalize().unwrap_or(root);
    if !absolute.starts_with(&root) {
        return Err("that is not a skill file".to_string());
    }
    std::fs::read_to_string(&absolute).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn skill_write(
    name: String,
    description: Option<String>,
    content: Option<String>,
) -> Result<String, String> {
    write_skill(&name, description.as_deref().unwrap_or(""), content.as_deref().unwrap_or(""))
}

#[cfg(test)]
mod tests {
    /// The slug decides the folder name, and a name that could escape the skills folder is
    /// refused rather than sanitised into something surprising.
    #[test]
    fn a_written_skill_is_readable_again() {
        let root = std::env::temp_dir().join(format!("ds-ide-skills-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();

        let path = write_skill_at(
            &root,
            "Release Notes",
            "Beim Schreiben von Versionsnotizen",
            "# Schritte\n\n1. Diff lesen",
        )
        .unwrap();

        let written = std::fs::read_to_string(&path).unwrap();
        assert!(written.starts_with("---\nname: release-notes\n"), "{written}");
        assert!(written.contains("description: Beim Schreiben von Versionsnotizen"));
        assert!(written.contains("1. Diff lesen"));
        assert!(Path::new(&path).ends_with("release-notes/SKILL.md"));

        // Writing again is editing, and does not create a second folder.
        write_skill_at(&root, "Release Notes", "anders", "neu").unwrap();
        let entries: Vec<_> = std::fs::read_dir(&root).unwrap().filter_map(Result::ok).collect();
        assert_eq!(entries.len(), 1);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn slugs_stay_inside_the_folder() {
        assert_eq!(safe_slug("Release Notes").unwrap(), "release-notes");
        assert_eq!(safe_slug("../etc/passwd").unwrap(), "etc-passwd");
        assert!(safe_slug("...").is_err());
        assert!(safe_slug("   ").is_err());
    }

    use super::*;

    #[test]
    fn parses_front_matter_and_falls_back_to_directory_name() {
        let raw = "---\nname: landing-page-builder\ndescription: Build a landing page\n---\n\n# Body\n";
        assert_eq!(
            front_matter_field(raw, "name").as_deref(),
            Some("landing-page-builder")
        );
        assert_eq!(
            front_matter_field(raw, "description").as_deref(),
            Some("Build a landing page")
        );

        let without_name = "---\ndescription: no name here\n---\n";
        assert_eq!(front_matter_field(without_name, "name"), None);
    }

    /// Depends on the user's own ~/.ds-ide/skills, so it is opt-in.
    #[test]
    #[ignore = "depends on the user's ~/.ds-ide/skills"]
    fn list_local_skills_smoke() {
        let found = list();
        println!("{} skill(s) in {}", found.len(), skills_root().display());
        for skill in &found {
            println!("  {} — {}", skill.name, skill.description);
        }
    }
}
