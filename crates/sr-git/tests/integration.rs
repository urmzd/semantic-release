use std::process::Command;

use sr_core::git::GitRepository;
use sr_git::NativeGitRepository;
use tempfile::TempDir;

fn init_repo() -> (TempDir, NativeGitRepository) {
    let dir = TempDir::new().unwrap();
    let path = dir.path();

    let git = |args: &[&str]| {
        let out = Command::new("git")
            .arg("-C")
            .arg(path)
            .args(args)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&out.stderr)
        );
    };

    git(&["init"]);
    git(&["config", "user.email", "test@test.com"]);
    git(&["config", "user.name", "Test"]);
    git(&["commit", "--allow-empty", "-m", "feat: initial"]);

    let repo = NativeGitRepository::open(path).unwrap();
    (dir, repo)
}

fn git_in(dir: &TempDir, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir.path())
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

#[test]
fn open_valid_repo() {
    let (dir, _repo) = init_repo();
    assert!(NativeGitRepository::open(dir.path()).is_ok());
}

#[test]
fn open_non_repo() {
    let dir = TempDir::new().unwrap();
    assert!(NativeGitRepository::open(dir.path()).is_err());
}

#[test]
fn latest_tag_none() {
    let (_dir, repo) = init_repo();
    let tag = repo.latest_tag("v").unwrap();
    assert!(tag.is_none());
}

#[test]
fn latest_tag_finds_latest() {
    let (dir, repo) = init_repo();
    git_in(&dir, &["tag", "v1.0.0"]);
    git_in(&dir, &["commit", "--allow-empty", "-m", "feat: second"]);
    git_in(&dir, &["tag", "v1.1.0"]);

    let tag = repo.latest_tag("v").unwrap().unwrap();
    assert_eq!(tag.name, "v1.1.0");
    assert_eq!(tag.version, semver::Version::new(1, 1, 0));
}

#[test]
fn commits_since_all() {
    let (dir, repo) = init_repo();
    git_in(&dir, &["commit", "--allow-empty", "-m", "fix: second"]);
    git_in(&dir, &["commit", "--allow-empty", "-m", "feat: third"]);

    let commits = repo.commits_since(None).unwrap();
    assert_eq!(commits.len(), 3);
}

#[test]
fn commits_since_partial() {
    let (dir, repo) = init_repo();
    let first_sha = git_in(&dir, &["rev-parse", "HEAD"]);
    git_in(&dir, &["commit", "--allow-empty", "-m", "fix: second"]);
    git_in(&dir, &["commit", "--allow-empty", "-m", "feat: third"]);

    let commits = repo.commits_since(Some(&first_sha)).unwrap();
    assert_eq!(commits.len(), 2);
}

#[test]
fn create_tag_exists() {
    let (dir, repo) = init_repo();
    repo.create_tag("v1.0.0", "release v1.0.0").unwrap();

    let tags = git_in(&dir, &["tag", "-l"]);
    assert!(tags.contains("v1.0.0"));
}
