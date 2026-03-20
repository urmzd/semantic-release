use std::fs;
use std::path::{Path, PathBuf};

use regex::Regex;

use crate::error::ReleaseError;

/// Bump the `version` field in the given manifest file.
///
/// Returns a list of additional files that were auto-discovered and bumped
/// (e.g. workspace member manifests). The caller should stage these files.
///
/// The file format is auto-detected from the filename:
/// - `Cargo.toml`          → TOML (`package.version` or `workspace.package.version`)
/// - `package.json`        → JSON (`.version`)
/// - `pyproject.toml`      → TOML (`project.version` or `tool.poetry.version`)
/// - `build.gradle`        → Gradle Groovy DSL (`version = '...'` or `version = "..."`)
/// - `build.gradle.kts`    → Gradle Kotlin DSL (`version = "..."`)
/// - `pom.xml`             → Maven (`<version>...</version>`, skipping `<parent>` block)
/// - `*.go`                → Go (`var/const Version = "..."`)
///
/// For workspace roots (Cargo, npm, uv), member manifests are auto-discovered
/// and bumped without needing to list them in `version_files`.
pub fn bump_version_file(path: &Path, new_version: &str) -> Result<Vec<PathBuf>, ReleaseError> {
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();

    match filename {
        "Cargo.toml" => bump_cargo_toml(path, new_version),
        "package.json" => bump_package_json(path, new_version),
        "pyproject.toml" => bump_pyproject_toml(path, new_version),
        "pom.xml" => bump_pom_xml(path, new_version).map(|()| vec![]),
        "build.gradle" | "build.gradle.kts" => bump_gradle(path, new_version).map(|()| vec![]),
        _ if filename.ends_with(".go") => bump_go_version(path, new_version).map(|()| vec![]),
        other => Err(ReleaseError::VersionBump(format!(
            "unsupported version file: {other}"
        ))),
    }
}

fn bump_cargo_toml(path: &Path, new_version: &str) -> Result<Vec<PathBuf>, ReleaseError> {
    let contents = read_file(path)?;
    let mut doc: toml_edit::DocumentMut = contents.parse().map_err(|e| {
        ReleaseError::VersionBump(format!("failed to parse {}: {e}", path.display()))
    })?;

    let is_workspace = doc
        .get("workspace")
        .and_then(|w| w.get("package"))
        .and_then(|p| p.get("version"))
        .is_some();

    if doc.get("package").and_then(|p| p.get("version")).is_some() {
        doc["package"]["version"] = toml_edit::value(new_version);
    } else if is_workspace {
        doc["workspace"]["package"]["version"] = toml_edit::value(new_version);

        // Also update [workspace.dependencies] entries that are internal path deps
        if let Some(deps) = doc
            .get_mut("workspace")
            .and_then(|w| w.get_mut("dependencies"))
            .and_then(|d| d.as_table_like_mut())
        {
            for (_, dep) in deps.iter_mut() {
                if let Some(tbl) = dep.as_table_like_mut()
                    && tbl.get("path").is_some()
                    && tbl.get("version").is_some()
                {
                    tbl.insert("version", toml_edit::value(new_version));
                }
            }
        }
    } else {
        return Err(ReleaseError::VersionBump(format!(
            "no version field found in {}",
            path.display()
        )));
    }

    write_file(path, &doc.to_string())?;

    // Auto-discover and bump workspace member Cargo.toml files
    let mut extra = Vec::new();
    if is_workspace {
        let members = extract_toml_string_array(&doc, &["workspace", "members"]);
        let root_dir = path.parent().unwrap_or(Path::new("."));
        for member_path in resolve_member_globs(root_dir, &members, "Cargo.toml") {
            if member_path.as_path() == path {
                continue;
            }
            match bump_cargo_member(&member_path, new_version) {
                Ok(true) => extra.push(member_path),
                Ok(false) => {}
                Err(e) => eprintln!("warning: {e}"),
            }
        }
    }

    Ok(extra)
}

/// Bump `package.version` in a workspace member Cargo.toml (skip if using `version.workspace = true`).
/// Returns `true` if the file was actually modified.
fn bump_cargo_member(path: &Path, new_version: &str) -> Result<bool, ReleaseError> {
    let contents = read_file(path)?;
    let mut doc: toml_edit::DocumentMut = contents.parse().map_err(|e| {
        ReleaseError::VersionBump(format!("failed to parse {}: {e}", path.display()))
    })?;

    // Skip members that inherit version from workspace
    let version_item = doc.get("package").and_then(|p| p.get("version"));
    match version_item {
        Some(item) if item.is_value() => {
            doc["package"]["version"] = toml_edit::value(new_version);
            write_file(path, &doc.to_string())?;
            Ok(true)
        }
        _ => Ok(false), // No version or uses workspace inheritance — skip
    }
}

fn bump_package_json(path: &Path, new_version: &str) -> Result<Vec<PathBuf>, ReleaseError> {
    let contents = read_file(path)?;
    let mut value: serde_json::Value = serde_json::from_str(&contents).map_err(|e| {
        ReleaseError::VersionBump(format!("failed to parse {}: {e}", path.display()))
    })?;

    let obj = value
        .as_object_mut()
        .ok_or_else(|| ReleaseError::VersionBump("package.json is not an object".into()))?;

    // Extract workspace patterns before mutating
    let workspace_patterns: Vec<String> = obj
        .get("workspaces")
        .and_then(|w| w.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    obj.insert(
        "version".into(),
        serde_json::Value::String(new_version.into()),
    );

    let output = serde_json::to_string_pretty(&value).map_err(|e| {
        ReleaseError::VersionBump(format!("failed to serialize {}: {e}", path.display()))
    })?;

    write_file(path, &format!("{output}\n"))?;

    // Auto-discover and bump workspace member package.json files
    let mut extra = Vec::new();
    if !workspace_patterns.is_empty() {
        let root_dir = path.parent().unwrap_or(Path::new("."));
        for member_path in resolve_member_globs(root_dir, &workspace_patterns, "package.json") {
            if member_path == path {
                continue;
            }
            match bump_json_version(&member_path, new_version) {
                Ok(true) => extra.push(member_path),
                Ok(false) => {}
                Err(e) => eprintln!("warning: {e}"),
            }
        }
    }

    Ok(extra)
}

/// Bump `version` in a member package.json (skip if no version field).
/// Returns `true` if the file was actually modified.
fn bump_json_version(path: &Path, new_version: &str) -> Result<bool, ReleaseError> {
    let contents = read_file(path)?;
    let mut value: serde_json::Value = serde_json::from_str(&contents).map_err(|e| {
        ReleaseError::VersionBump(format!("failed to parse {}: {e}", path.display()))
    })?;

    let obj = match value.as_object_mut() {
        Some(o) => o,
        None => return Ok(false),
    };

    if obj.get("version").is_none() {
        return Ok(false);
    }

    obj.insert(
        "version".into(),
        serde_json::Value::String(new_version.into()),
    );

    let output = serde_json::to_string_pretty(&value).map_err(|e| {
        ReleaseError::VersionBump(format!("failed to serialize {}: {e}", path.display()))
    })?;

    write_file(path, &format!("{output}\n"))?;
    Ok(true)
}

fn bump_pyproject_toml(path: &Path, new_version: &str) -> Result<Vec<PathBuf>, ReleaseError> {
    let contents = read_file(path)?;
    let mut doc: toml_edit::DocumentMut = contents.parse().map_err(|e| {
        ReleaseError::VersionBump(format!("failed to parse {}: {e}", path.display()))
    })?;

    if doc.get("project").and_then(|p| p.get("version")).is_some() {
        doc["project"]["version"] = toml_edit::value(new_version);
    } else if doc
        .get("tool")
        .and_then(|t| t.get("poetry"))
        .and_then(|p| p.get("version"))
        .is_some()
    {
        doc["tool"]["poetry"]["version"] = toml_edit::value(new_version);
    } else {
        return Err(ReleaseError::VersionBump(format!(
            "no version field found in {}",
            path.display()
        )));
    }

    write_file(path, &doc.to_string())?;

    // Auto-discover uv workspace members
    let members = extract_toml_string_array(&doc, &["tool", "uv", "workspace", "members"]);
    let mut extra = Vec::new();
    if !members.is_empty() {
        let root_dir = path.parent().unwrap_or(Path::new("."));
        for member_path in resolve_member_globs(root_dir, &members, "pyproject.toml") {
            if member_path.as_path() == path {
                continue;
            }
            match bump_pyproject_member(&member_path, new_version) {
                Ok(true) => extra.push(member_path),
                Ok(false) => {}
                Err(e) => eprintln!("warning: {e}"),
            }
        }
    }

    Ok(extra)
}

/// Bump version in a uv workspace member pyproject.toml (skip if no version field).
/// Returns `true` if the file was actually modified.
fn bump_pyproject_member(path: &Path, new_version: &str) -> Result<bool, ReleaseError> {
    let contents = read_file(path)?;
    let mut doc: toml_edit::DocumentMut = contents.parse().map_err(|e| {
        ReleaseError::VersionBump(format!("failed to parse {}: {e}", path.display()))
    })?;

    if doc.get("project").and_then(|p| p.get("version")).is_some() {
        doc["project"]["version"] = toml_edit::value(new_version);
    } else if doc
        .get("tool")
        .and_then(|t| t.get("poetry"))
        .and_then(|p| p.get("version"))
        .is_some()
    {
        doc["tool"]["poetry"]["version"] = toml_edit::value(new_version);
    } else {
        return Ok(false); // No version field — skip
    }

    write_file(path, &doc.to_string())?;
    Ok(true)
}

fn bump_gradle(path: &Path, new_version: &str) -> Result<(), ReleaseError> {
    let contents = read_file(path)?;
    let re = Regex::new(r#"(version\s*=\s*["'])([^"']*)(["'])"#).unwrap();
    if !re.is_match(&contents) {
        return Err(ReleaseError::VersionBump(format!(
            "no version assignment found in {}",
            path.display()
        )));
    }
    let result = re.replacen(&contents, 1, format!("${{1}}{new_version}${{3}}"));
    write_file(path, &result)
}

fn bump_pom_xml(path: &Path, new_version: &str) -> Result<(), ReleaseError> {
    let contents = read_file(path)?;

    // Determine search start: skip past </parent> if present, else after </modelVersion>
    let search_start = if let Some(pos) = contents.find("</parent>") {
        pos + "</parent>".len()
    } else if let Some(pos) = contents.find("</modelVersion>") {
        pos + "</modelVersion>".len()
    } else {
        0
    };

    let rest = &contents[search_start..];
    let re = Regex::new(r"<version>[^<]*</version>").unwrap();
    if let Some(m) = re.find(rest) {
        let replacement = format!("<version>{new_version}</version>");
        let mut result = String::with_capacity(contents.len());
        result.push_str(&contents[..search_start + m.start()]);
        result.push_str(&replacement);
        result.push_str(&contents[search_start + m.end()..]);
        write_file(path, &result)
    } else {
        Err(ReleaseError::VersionBump(format!(
            "no <version> element found in {}",
            path.display()
        )))
    }
}

fn bump_go_version(path: &Path, new_version: &str) -> Result<(), ReleaseError> {
    let contents = read_file(path)?;
    let re = Regex::new(r#"((?:var|const)\s+Version\s*(?:string\s*)?=\s*")([^"]*)(")"#).unwrap();
    if !re.is_match(&contents) {
        return Err(ReleaseError::VersionBump(format!(
            "no Version variable found in {}",
            path.display()
        )));
    }
    let result = re.replacen(&contents, 1, format!("${{1}}{new_version}${{3}}"));
    write_file(path, &result)
}

/// Extract a string array from a nested TOML path (e.g. `["workspace", "members"]`).
fn extract_toml_string_array(doc: &toml_edit::DocumentMut, keys: &[&str]) -> Vec<String> {
    let mut item: Option<&toml_edit::Item> = None;
    for key in keys {
        item = match item {
            None => doc.get(key),
            Some(parent) => parent.get(key),
        };
        if item.is_none() {
            return vec![];
        }
    }
    item.and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

/// Resolve workspace member glob patterns into manifest file paths.
/// Each glob is resolved relative to `root_dir`, and `manifest_name` is appended
/// to each matched directory (e.g. "Cargo.toml", "package.json", "pyproject.toml").
fn resolve_member_globs(root_dir: &Path, patterns: &[String], manifest_name: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for pattern in patterns {
        let full_pattern = root_dir.join(pattern).to_string_lossy().into_owned();
        let Ok(entries) = glob::glob(&full_pattern) else {
            continue;
        };
        for entry in entries.flatten() {
            let manifest = if entry.is_dir() {
                entry.join(manifest_name)
            } else {
                continue;
            };
            if manifest.exists() {
                paths.push(manifest);
            }
        }
    }
    paths
}

fn read_file(path: &Path) -> Result<String, ReleaseError> {
    fs::read_to_string(path)
        .map_err(|e| ReleaseError::VersionBump(format!("failed to read {}: {e}", path.display())))
}

fn write_file(path: &Path, contents: &str) -> Result<(), ReleaseError> {
    fs::write(path, contents)
        .map_err(|e| ReleaseError::VersionBump(format!("failed to write {}: {e}", path.display())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bump_cargo_toml_package_version() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Cargo.toml");
        fs::write(
            &path,
            r#"[package]
name = "my-crate"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = "1"
"#,
        )
        .unwrap();

        bump_version_file(&path, "1.2.3").unwrap();

        let contents = fs::read_to_string(&path).unwrap();
        assert!(contents.contains("version = \"1.2.3\""));
        assert!(contents.contains("name = \"my-crate\""));
        assert!(contents.contains("serde = \"1\""));
    }

    #[test]
    fn bump_cargo_toml_workspace_version() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Cargo.toml");
        fs::write(
            &path,
            r#"[workspace]
members = ["crates/*"]

[workspace.package]
version = "0.0.1"
edition = "2021"
"#,
        )
        .unwrap();

        bump_version_file(&path, "2.0.0").unwrap();

        let contents = fs::read_to_string(&path).unwrap();
        assert!(contents.contains("version = \"2.0.0\""));
        assert!(contents.contains("members = [\"crates/*\"]"));
    }

    #[test]
    fn bump_package_json_version() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("package.json");
        fs::write(
            &path,
            r#"{
  "name": "my-pkg",
  "version": "0.0.0",
  "description": "test"
}"#,
        )
        .unwrap();

        bump_version_file(&path, "3.1.0").unwrap();

        let contents = fs::read_to_string(&path).unwrap();
        let value: serde_json::Value = serde_json::from_str(&contents).unwrap();
        assert_eq!(value["version"], "3.1.0");
        assert_eq!(value["name"], "my-pkg");
        assert_eq!(value["description"], "test");
        assert!(contents.ends_with('\n'));
    }

    #[test]
    fn bump_pyproject_toml_project_version() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pyproject.toml");
        fs::write(
            &path,
            r#"[project]
name = "my-project"
version = "0.1.0"
description = "A test project"
"#,
        )
        .unwrap();

        bump_version_file(&path, "1.0.0").unwrap();

        let contents = fs::read_to_string(&path).unwrap();
        assert!(contents.contains("version = \"1.0.0\""));
        assert!(contents.contains("name = \"my-project\""));
    }

    #[test]
    fn bump_pyproject_toml_poetry_version() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pyproject.toml");
        fs::write(
            &path,
            r#"[tool.poetry]
name = "my-poetry-project"
version = "0.2.0"
description = "A poetry project"
"#,
        )
        .unwrap();

        bump_version_file(&path, "0.3.0").unwrap();

        let contents = fs::read_to_string(&path).unwrap();
        assert!(contents.contains("version = \"0.3.0\""));
        assert!(contents.contains("name = \"my-poetry-project\""));
    }

    #[test]
    fn bump_unknown_file_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("unknown.txt");
        fs::write(&path, "version = 1").unwrap();

        let err = bump_version_file(&path, "1.0.0").unwrap_err();
        assert!(matches!(err, ReleaseError::VersionBump(_)));
        assert!(err.to_string().contains("unsupported"));
    }

    #[test]
    fn bump_build_gradle_version() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("build.gradle");
        fs::write(
            &path,
            r#"plugins {
    id 'java'
}

group = 'com.example'
version = '1.0.0'

dependencies {
    implementation 'org.slf4j:slf4j-api:2.0.0'
}
"#,
        )
        .unwrap();

        bump_version_file(&path, "2.0.0").unwrap();

        let contents = fs::read_to_string(&path).unwrap();
        assert!(contents.contains("version = '2.0.0'"));
        assert!(contents.contains("group = 'com.example'"));
        // dependency version must not change
        assert!(contents.contains("slf4j-api:2.0.0"));
    }

    #[test]
    fn bump_build_gradle_kts_version() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("build.gradle.kts");
        fs::write(
            &path,
            r#"plugins {
    kotlin("jvm") version "1.9.0"
}

group = "com.example"
version = "1.0.0"

dependencies {
    implementation("org.slf4j:slf4j-api:2.0.0")
}
"#,
        )
        .unwrap();

        bump_version_file(&path, "3.0.0").unwrap();

        let contents = fs::read_to_string(&path).unwrap();
        assert!(contents.contains("version = \"3.0.0\""));
        assert!(contents.contains("group = \"com.example\""));
    }

    #[test]
    fn bump_pom_xml_version() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pom.xml");
        fs::write(
            &path,
            r#"<?xml version="1.0" encoding="UTF-8"?>
<project>
    <modelVersion>4.0.0</modelVersion>
    <groupId>com.example</groupId>
    <artifactId>my-app</artifactId>
    <version>1.0.0</version>
</project>
"#,
        )
        .unwrap();

        bump_version_file(&path, "2.0.0").unwrap();

        let contents = fs::read_to_string(&path).unwrap();
        assert!(contents.contains("<version>2.0.0</version>"));
        assert!(contents.contains("<groupId>com.example</groupId>"));
    }

    #[test]
    fn bump_pom_xml_with_parent_version() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pom.xml");
        fs::write(
            &path,
            r#"<?xml version="1.0" encoding="UTF-8"?>
<project>
    <modelVersion>4.0.0</modelVersion>
    <parent>
        <groupId>com.example</groupId>
        <artifactId>parent</artifactId>
        <version>5.0.0</version>
    </parent>
    <artifactId>my-app</artifactId>
    <version>1.0.0</version>
</project>
"#,
        )
        .unwrap();

        bump_version_file(&path, "2.0.0").unwrap();

        let contents = fs::read_to_string(&path).unwrap();
        // Parent version must NOT be changed
        assert!(contents.contains("<version>5.0.0</version>"));
        // Project version must be changed
        assert!(contents.contains("<version>2.0.0</version>"));
        // Verify there are exactly two <version> tags with expected values
        let version_count: Vec<&str> = contents.matches("<version>").collect();
        assert_eq!(version_count.len(), 2);
    }

    #[test]
    fn bump_cargo_toml_workspace_dependencies_with_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Cargo.toml");
        fs::write(
            &path,
            r#"[workspace]
members = ["crates/*"]

[workspace.package]
version = "0.1.0"
edition = "2021"

[workspace.dependencies]
# Internal crates
sr-core = { path = "crates/sr-core", version = "0.1.0" }
sr-git = { path = "crates/sr-git", version = "0.1.0" }
# External dep should not change
serde = { version = "1", features = ["derive"] }
"#,
        )
        .unwrap();

        bump_version_file(&path, "2.0.0").unwrap();

        let contents = fs::read_to_string(&path).unwrap();
        let doc: toml_edit::DocumentMut = contents.parse().unwrap();

        // workspace.package.version should be bumped
        assert_eq!(
            doc["workspace"]["package"]["version"].as_str().unwrap(),
            "2.0.0"
        );
        // Internal path deps should have their version bumped
        assert_eq!(
            doc["workspace"]["dependencies"]["sr-core"]["version"]
                .as_str()
                .unwrap(),
            "2.0.0"
        );
        assert_eq!(
            doc["workspace"]["dependencies"]["sr-git"]["version"]
                .as_str()
                .unwrap(),
            "2.0.0"
        );
        // External dep version must NOT change
        assert_eq!(
            doc["workspace"]["dependencies"]["serde"]["version"]
                .as_str()
                .unwrap(),
            "1"
        );
    }

    #[test]
    fn bump_go_version_var() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("version.go");
        fs::write(
            &path,
            r#"package main

var Version = "1.0.0"

func main() {}
"#,
        )
        .unwrap();

        bump_version_file(&path, "2.0.0").unwrap();

        let contents = fs::read_to_string(&path).unwrap();
        assert!(contents.contains(r#"var Version = "2.0.0""#));
    }

    #[test]
    fn bump_go_version_const() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("version.go");
        fs::write(
            &path,
            r#"package main

const Version string = "0.5.0"

func main() {}
"#,
        )
        .unwrap();

        bump_version_file(&path, "0.6.0").unwrap();

        let contents = fs::read_to_string(&path).unwrap();
        assert!(contents.contains(r#"const Version string = "0.6.0""#));
    }

    // --- workspace auto-discovery tests ---

    #[test]
    fn bump_cargo_workspace_discovers_members() {
        let dir = tempfile::tempdir().unwrap();

        // Create workspace root
        let root = dir.path().join("Cargo.toml");
        fs::write(
            &root,
            r#"[workspace]
members = ["crates/*"]

[workspace.package]
version = "1.0.0"
edition = "2021"

[workspace.dependencies]
my-core = { path = "crates/core", version = "1.0.0" }
"#,
        )
        .unwrap();

        // Create member with hardcoded version
        fs::create_dir_all(dir.path().join("crates/core")).unwrap();
        let member = dir.path().join("crates/core/Cargo.toml");
        fs::write(
            &member,
            r#"[package]
name = "my-core"
version = "1.0.0"
edition = "2021"
"#,
        )
        .unwrap();

        // Create member that uses workspace inheritance (should be skipped)
        fs::create_dir_all(dir.path().join("crates/cli")).unwrap();
        let inherited_member = dir.path().join("crates/cli/Cargo.toml");
        fs::write(
            &inherited_member,
            r#"[package]
name = "my-cli"
version.workspace = true
edition.workspace = true
"#,
        )
        .unwrap();

        let extra = bump_version_file(&root, "2.0.0").unwrap();

        // Root should be bumped
        let root_contents = fs::read_to_string(&root).unwrap();
        assert!(root_contents.contains("version = \"2.0.0\""));

        // Workspace dep should be bumped
        let doc: toml_edit::DocumentMut = root_contents.parse().unwrap();
        assert_eq!(
            doc["workspace"]["dependencies"]["my-core"]["version"]
                .as_str()
                .unwrap(),
            "2.0.0"
        );

        // Member with hardcoded version should be bumped
        let member_contents = fs::read_to_string(&member).unwrap();
        assert!(member_contents.contains("version = \"2.0.0\""));

        // Member with workspace inheritance should NOT be modified
        let inherited_contents = fs::read_to_string(&inherited_member).unwrap();
        assert!(inherited_contents.contains("version.workspace = true"));

        // Only the hardcoded member should be in extra
        assert_eq!(extra.len(), 1);
        assert_eq!(extra[0], member);
    }

    #[test]
    fn bump_npm_workspace_discovers_members() {
        let dir = tempfile::tempdir().unwrap();

        // Create root package.json with workspaces
        let root = dir.path().join("package.json");
        fs::write(
            &root,
            r#"{
  "name": "my-monorepo",
  "version": "1.0.0",
  "workspaces": ["packages/*"]
}"#,
        )
        .unwrap();

        // Create member
        fs::create_dir_all(dir.path().join("packages/core")).unwrap();
        let member = dir.path().join("packages/core/package.json");
        fs::write(
            &member,
            r#"{
  "name": "@my/core",
  "version": "1.0.0"
}"#,
        )
        .unwrap();

        // Create member without version (should be skipped)
        fs::create_dir_all(dir.path().join("packages/utils")).unwrap();
        let no_version_member = dir.path().join("packages/utils/package.json");
        fs::write(
            &no_version_member,
            r#"{
  "name": "@my/utils",
  "private": true
}"#,
        )
        .unwrap();

        let extra = bump_version_file(&root, "2.0.0").unwrap();

        // Root bumped
        let root_contents: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&root).unwrap()).unwrap();
        assert_eq!(root_contents["version"], "2.0.0");

        // Member with version bumped
        let member_contents: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&member).unwrap()).unwrap();
        assert_eq!(member_contents["version"], "2.0.0");

        // Member without version untouched
        let utils_contents: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&no_version_member).unwrap()).unwrap();
        assert!(utils_contents.get("version").is_none());

        assert_eq!(extra.len(), 1);
        assert_eq!(extra[0], member);
    }

    #[test]
    fn bump_uv_workspace_discovers_members() {
        let dir = tempfile::tempdir().unwrap();

        // Create root pyproject.toml with uv workspace
        let root = dir.path().join("pyproject.toml");
        fs::write(
            &root,
            r#"[project]
name = "my-monorepo"
version = "1.0.0"

[tool.uv.workspace]
members = ["packages/*"]
"#,
        )
        .unwrap();

        // Create member
        fs::create_dir_all(dir.path().join("packages/core")).unwrap();
        let member = dir.path().join("packages/core/pyproject.toml");
        fs::write(
            &member,
            r#"[project]
name = "my-core"
version = "1.0.0"
"#,
        )
        .unwrap();

        let extra = bump_version_file(&root, "2.0.0").unwrap();

        // Root bumped
        let root_contents = fs::read_to_string(&root).unwrap();
        assert!(root_contents.contains("version = \"2.0.0\""));

        // Member bumped
        let member_contents = fs::read_to_string(&member).unwrap();
        assert!(member_contents.contains("version = \"2.0.0\""));

        assert_eq!(extra.len(), 1);
        assert_eq!(extra[0], member);
    }

    #[test]
    fn bump_non_workspace_returns_empty_extra() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Cargo.toml");
        fs::write(
            &path,
            r#"[package]
name = "solo-crate"
version = "1.0.0"
"#,
        )
        .unwrap();

        let extra = bump_version_file(&path, "2.0.0").unwrap();
        assert!(extra.is_empty());
    }
}
