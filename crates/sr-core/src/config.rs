use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::commit::{CommitType, DEFAULT_COMMIT_PATTERN, default_commit_types};
use crate::error::ReleaseError;
use crate::version::BumpLevel;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ReleaseConfig {
    pub branches: Vec<String>,
    pub tag_prefix: String,
    pub commit_pattern: String,
    pub breaking_section: String,
    pub misc_section: String,
    pub types: Vec<CommitType>,
    pub changelog: ChangelogConfig,
    pub version_files: Vec<String>,
    pub version_files_strict: bool,
    pub artifacts: Vec<String>,
    pub floating_tags: bool,
}

impl Default for ReleaseConfig {
    fn default() -> Self {
        Self {
            branches: vec!["main".into(), "master".into()],
            tag_prefix: "v".into(),
            commit_pattern: DEFAULT_COMMIT_PATTERN.into(),
            breaking_section: "Breaking Changes".into(),
            misc_section: "Miscellaneous".into(),
            types: default_commit_types(),
            changelog: ChangelogConfig::default(),
            version_files: vec![],
            version_files_strict: false,
            artifacts: vec![],
            floating_tags: false,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ChangelogConfig {
    pub file: Option<String>,
    pub template: Option<String>,
}

impl ReleaseConfig {
    /// Load config from a YAML file, falling back to defaults if the file doesn't exist.
    pub fn load(path: &Path) -> Result<Self, ReleaseError> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let contents =
            std::fs::read_to_string(path).map_err(|e| ReleaseError::Config(e.to_string()))?;

        serde_yaml_ng::from_str(&contents).map_err(|e| ReleaseError::Config(e.to_string()))
    }
}

// Custom deserialization for BumpLevel so it can appear in YAML config.
impl<'de> Deserialize<'de> for BumpLevel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "major" => Ok(BumpLevel::Major),
            "minor" => Ok(BumpLevel::Minor),
            "patch" => Ok(BumpLevel::Patch),
            _ => Err(serde::de::Error::custom(format!("unknown bump level: {s}"))),
        }
    }
}

impl Serialize for BumpLevel {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let s = match self {
            BumpLevel::Major => "major",
            BumpLevel::Minor => "minor",
            BumpLevel::Patch => "patch",
        };
        serializer.serialize_str(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn default_values() {
        let config = ReleaseConfig::default();
        assert_eq!(config.branches, vec!["main", "master"]);
        assert_eq!(config.tag_prefix, "v");
        assert_eq!(config.commit_pattern, DEFAULT_COMMIT_PATTERN);
        assert_eq!(config.breaking_section, "Breaking Changes");
        assert_eq!(config.misc_section, "Miscellaneous");
        assert!(!config.types.is_empty());
        assert!(!config.version_files_strict);
        assert!(config.artifacts.is_empty());
        assert!(!config.floating_tags);
    }

    #[test]
    fn load_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nonexistent.yml");
        let config = ReleaseConfig::load(&path).unwrap();
        assert_eq!(config.tag_prefix, "v");
    }

    #[test]
    fn load_valid_yaml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.yml");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "branches:\n  - develop\ntag_prefix: release-").unwrap();

        let config = ReleaseConfig::load(&path).unwrap();
        assert_eq!(config.branches, vec!["develop"]);
        assert_eq!(config.tag_prefix, "release-");
    }

    #[test]
    fn load_partial_yaml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.yml");
        std::fs::write(&path, "tag_prefix: rel-\n").unwrap();

        let config = ReleaseConfig::load(&path).unwrap();
        assert_eq!(config.tag_prefix, "rel-");
        assert_eq!(config.branches, vec!["main", "master"]);
        // defaults should still apply for types/pattern/breaking_section
        assert_eq!(config.commit_pattern, DEFAULT_COMMIT_PATTERN);
        assert_eq!(config.breaking_section, "Breaking Changes");
        assert!(!config.types.is_empty());
    }

    #[test]
    fn load_yaml_with_artifacts() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.yml");
        std::fs::write(
            &path,
            "artifacts:\n  - \"dist/*.tar.gz\"\n  - \"build/output-*\"\n",
        )
        .unwrap();

        let config = ReleaseConfig::load(&path).unwrap();
        assert_eq!(config.artifacts, vec!["dist/*.tar.gz", "build/output-*"]);
        // defaults still apply
        assert_eq!(config.tag_prefix, "v");
    }

    #[test]
    fn load_yaml_with_floating_tags() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.yml");
        std::fs::write(&path, "floating_tags: true\n").unwrap();

        let config = ReleaseConfig::load(&path).unwrap();
        assert!(config.floating_tags);
        // defaults still apply
        assert_eq!(config.tag_prefix, "v");
    }

    #[test]
    fn bump_level_roundtrip() {
        for (level, expected) in [
            (BumpLevel::Major, "major"),
            (BumpLevel::Minor, "minor"),
            (BumpLevel::Patch, "patch"),
        ] {
            let yaml = serde_yaml_ng::to_string(&level).unwrap();
            assert!(yaml.contains(expected));
            let parsed: BumpLevel = serde_yaml_ng::from_str(&yaml).unwrap();
            assert_eq!(parsed, level);
        }
    }

    #[test]
    fn types_roundtrip() {
        let config = ReleaseConfig::default();
        let yaml = serde_yaml_ng::to_string(&config).unwrap();
        let parsed: ReleaseConfig = serde_yaml_ng::from_str(&yaml).unwrap();
        assert_eq!(parsed.types.len(), config.types.len());
        assert_eq!(parsed.types[0].name, "feat");
        assert_eq!(parsed.commit_pattern, config.commit_pattern);
        assert_eq!(parsed.breaking_section, config.breaking_section);
    }
}
