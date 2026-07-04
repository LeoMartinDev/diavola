use std::path::Path;

use serde::Serialize;

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitInfo {
    pub repo_name: Option<String>,
    pub branch: Option<String>,
    pub worktree: Option<String>,
    pub is_worktree: bool,
    pub display_path: Option<String>,
}

pub fn detect_git_info(base_dir: &Path) -> GitInfo {
    let display_path = home_relative_path(base_dir);

    let repo = match gix::discover(base_dir) {
        Ok(repo) => repo,
        Err(_) => {
            return GitInfo {
                display_path,
                ..Default::default()
            };
        }
    };

    let repo_name = repo
        .work_dir()
        .and_then(|work_dir| work_dir.file_name())
        .map(|name| name.to_string_lossy().into_owned());

    let branch = repo
        .head_name()
        .ok()
        .flatten()
        .map(|head| head.shorten().to_string());

    // Detect linked worktree: .git is a file containing "gitdir: <path>"
    let mut worktree: Option<String> = None;
    let mut is_worktree = false;
    let dot_git = base_dir.join(".git");
    if dot_git.is_file() {
        if let Ok(contents) = std::fs::read_to_string(&dot_git) {
            if let Some(gitdir_line) = contents.strip_prefix("gitdir: ") {
                let gitdir_path = gitdir_line.trim();
                // Extract worktree name: last component before /.git/worktrees/<name>
                // The gitdir path format is typically:
                //   <repo>/.git/worktrees/<name>
                let path = Path::new(gitdir_path);
                if let Some(worktrees_idx) = path
                    .components()
                    .position(|c| c.as_os_str() == "worktrees")
                {
                    if let Some(name) = path.components().nth(worktrees_idx + 1) {
                        worktree = Some(name.as_os_str().to_string_lossy().into_owned());
                        is_worktree = true;
                    }
                }
            }
        }
    }

    GitInfo {
        repo_name,
        branch,
        worktree,
        is_worktree,
        display_path,
    }
}

fn home_relative_path(absolute: &Path) -> Option<String> {
    if let Ok(home) = std::env::var("HOME") {
        let home_path = Path::new(&home);
        if let Ok(stripped) = absolute.strip_prefix(home_path) {
            return Some(stripped.to_string_lossy().into_owned());
        }
    }
    let mut comps = absolute.components().rev();
    let name = comps.next()?.as_os_str().to_string_lossy();
    let parent = comps.next().map(|c| c.as_os_str().to_string_lossy());
    match parent {
        Some(parent) => Some(format!("{}/{}", parent, name)),
        None => Some(name.into_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use tempfile::TempDir;

    fn setup_git_repo() -> (TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().to_path_buf();
        Command::new("git")
            .args(["init"])
            .current_dir(&path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(&path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(&path)
            .output()
            .unwrap();
        // Create an initial commit so HEAD resolves
        std::fs::write(path.join("readme.md"), "# test").unwrap();
        Command::new("git")
            .args(["add", "readme.md"])
            .current_dir(&path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "init"])
            .current_dir(&path)
            .output()
            .unwrap();
        (dir, path)
    }

    #[test]
    fn non_git_directory_returns_default() {
        let dir = tempfile::tempdir().unwrap();
        let info = detect_git_info(dir.path());
        assert!(info.repo_name.is_none());
        assert!(info.branch.is_none());
        assert!(info.worktree.is_none());
        assert!(!info.is_worktree);
    }

    #[test]
    fn normal_repo_detects_branch_and_name() {
        let (_dir, path) = setup_git_repo();
        let info = detect_git_info(&path);
        assert!(info.repo_name.is_some());
        assert!(info.branch.is_some());
        // Default branch may be "main" or "master"
        let branch = info.branch.unwrap();
        assert!(branch == "main" || branch == "master" || branch.starts_with("refs/heads/"));
        assert!(!info.is_worktree);
        assert!(info.worktree.is_none());
    }

    #[test]
    fn subdirectory_of_repo_detects_git() {
        let (_dir, path) = setup_git_repo();
        let subdir = path.join("subdir");
        std::fs::create_dir(&subdir).unwrap();
        let info = detect_git_info(&subdir);
        assert!(info.repo_name.is_some());
        assert!(info.branch.is_some());
    }

    #[test]
    fn worktree_is_detected() {
        let (_dir, path) = setup_git_repo();
        // Create a linked worktree
        let worktree_path = path.parent().unwrap().join("worktree_test");
        let _ = std::fs::remove_dir_all(&worktree_path);
        let output = Command::new("git")
            .args(["worktree", "add", worktree_path.to_str().unwrap()])
            .current_dir(&path)
            .output()
            .unwrap();
        assert!(output.status.success(), "git worktree add failed: {:?}", output);
        let info = detect_git_info(&worktree_path);
        assert!(info.is_worktree);
        assert!(info.worktree.is_some());
    }
}
