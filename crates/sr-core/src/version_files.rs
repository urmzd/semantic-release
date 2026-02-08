use std::fs;
use std::path::Path;

use regex::Regex;

use crate::error::ReleaseError;

/// Bump the `version` field in the given manifest file.
///
/// The file format is auto-detected from the filename:
/// - `Cargo.toml`          → TOML (`package.version` or `workspace.package.version`)
/// - `package.json`        → JSON (`.version`)
/// - `pyproject.toml`      → TOML (`project.version` or `tool.poetry.version`)
/// - `build.gradle`        → Gradle Groovy DSL (`version = '...'` or `version = "..."`)
/// - `build.gradle.kts`    → Gradle Kotlin DSL (`version = "..."`)
/// - `pom.xml`             → Maven (`<version>...</version>`, skipping `<parent>` block)
/// - `*.go`                → Go (`var/const Version = "..."`)
pub fn bump_version_file(path: &Path, new_version: &str) -> Result<(), ReleaseError> {
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();

    match filename {
        "Cargo.toml" => bump_cargo_toml(path, new_version),
        "package.json" => bump_package_json(path, new_version),
        "pyproject.toml" => bump_pyproject_toml(path, new_version),
        "pom.xml" => bump_pom_xml(path, new_version),
        "build.gradle" | "build.gradle.kts" => bump_gradle(path, new_version),
        _ if filename.ends_with(".go") => bump_go_version(path, new_version),
        other => Err(ReleaseError::VersionBump(format!(
            "unsupported version file: {other}"
        ))),
    }
}

fn bump_cargo_toml(path: &Path, new_version: &str) -> Result<(), ReleaseError> {
    let contents = read_file(path)?;
    let mut doc: toml_edit::DocumentMut = contents.parse().map_err(|e| {
        ReleaseError::VersionBump(format!("failed to parse {}: {e}", path.display()))
    })?;

    if doc.get("package").and_then(|p| p.get("version")).is_some() {
        doc["package"]["version"] = toml_edit::value(new_version);
    } else if doc
        .get("workspace")
        .and_then(|w| w.get("package"))
        .and_then(|p| p.get("version"))
        .is_some()
    {
        doc["workspace"]["package"]["version"] = toml_edit::value(new_version);
    } else {
        return Err(ReleaseError::VersionBump(format!(
            "no version field found in {}",
            path.display()
        )));
    }

    write_file(path, &doc.to_string())
}

fn bump_package_json(path: &Path, new_version: &str) -> Result<(), ReleaseError> {
    let contents = read_file(path)?;
    let mut value: serde_json::Value = serde_json::from_str(&contents).map_err(|e| {
        ReleaseError::VersionBump(format!("failed to parse {}: {e}", path.display()))
    })?;

    value
        .as_object_mut()
        .ok_or_else(|| ReleaseError::VersionBump("package.json is not an object".into()))?
        .insert(
            "version".into(),
            serde_json::Value::String(new_version.into()),
        );

    let output = serde_json::to_string_pretty(&value).map_err(|e| {
        ReleaseError::VersionBump(format!("failed to serialize {}: {e}", path.display()))
    })?;

    write_file(path, &format!("{output}\n"))
}

fn bump_pyproject_toml(path: &Path, new_version: &str) -> Result<(), ReleaseError> {
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

    write_file(path, &doc.to_string())
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
    let re =
        Regex::new(r#"((?:var|const)\s+Version\s*(?:string\s*)?=\s*")([^"]*)(")"#).unwrap();
    if !re.is_match(&contents) {
        return Err(ReleaseError::VersionBump(format!(
            "no Version variable found in {}",
            path.display()
        )));
    }
    let result = re.replacen(&contents, 1, format!("${{1}}{new_version}${{3}}"));
    write_file(path, &result)
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
}
