use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

/// Tracks the state of the git branch lifecycle during a repair apply.
#[derive(Debug, Clone)]
pub struct GitBranchState {
    pub base_branch: String,
    pub head_branch: String,
    pub base_sha: String,
    pub remote: String,
}

/// Serializable receipt for git branch/commit/push operations.
#[derive(Debug, Clone, Serialize)]
pub struct GitMutationReceipt {
    pub status: String,
    pub base_branch: String,
    pub head_branch: String,
    pub base_sha: String,
    pub head_sha: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit_sha: Option<String>,
    pub commit_title: String,
    pub files_committed: Vec<String>,
    pub rollback_command: String,
    pub remote: String,
    pub pushed: bool,
}

/// Serializable receipt for GitHub draft PR creation.
#[derive(Debug, Clone, Serialize)]
pub struct GithubPrReceipt {
    pub status: String,
    pub draft: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    pub remote: String,
    pub base_branch: String,
    pub head_branch: String,
    pub command: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Validate that the repo is a git repository with a clean tracked worktree
/// and a named branch (not detached HEAD). Returns the branch state.
pub fn preflight_git_repo(
    repo: &Path,
    remote: &str,
    requested_base: &str,
) -> Result<GitBranchState> {
    let toplevel = git_output(repo, ["rev-parse", "--show-toplevel"])
        .context("real repair apply requires a git repository")?;
    let toplevel = toplevel.trim();
    let canonical_repo =
        fs::canonicalize(repo).with_context(|| format!("canonicalize {}", repo.display()))?;
    let canonical_top =
        fs::canonicalize(toplevel).with_context(|| format!("canonicalize git root {toplevel}"))?;
    if !canonical_repo.starts_with(&canonical_top) {
        bail!(
            "repair repo {} is not inside git root {}",
            canonical_repo.display(),
            canonical_top.display()
        );
    }

    let current_branch = git_output(repo, ["symbolic-ref", "--short", "HEAD"])
        .context("real repair apply requires a named branch, not detached HEAD")?;
    let current_branch = current_branch.trim().to_string();
    if current_branch.is_empty() {
        bail!("real repair apply requires a non-empty current branch");
    }

    let status = git_output(repo, ["status", "--porcelain=v1", "--untracked-files=no"])
        .context("read git worktree status")?;
    if !status.trim().is_empty() {
        bail!(
            "real repair apply requires a clean tracked git worktree before mutation; tracked changes:\n{}",
            status
        );
    }

    let base_branch = if requested_base.is_empty() {
        current_branch.clone()
    } else {
        requested_base.to_string()
    };

    let base_sha = git_output(repo, ["rev-parse", &base_branch])
        .context("resolve base branch SHA")?
        .trim()
        .to_string();

    if remote.trim().is_empty() {
        bail!("remote name must not be empty");
    }

    Ok(GitBranchState {
        base_branch,
        head_branch: String::new(),
        base_sha,
        remote: remote.to_string(),
    })
}

/// Create a repair branch from the current HEAD. The branch name must
/// start with `jankurai/repair/` and pass git ref-format validation.
pub fn create_repair_branch(
    repo: &Path,
    state: &GitBranchState,
    branch: &str,
) -> Result<GitBranchState> {
    validate_branch_name(repo, branch)?;

    let ref_name = format!("refs/heads/{branch}");
    if git_status(repo, ["show-ref", "--verify", "--quiet", &ref_name])?.success() {
        bail!("repair branch `{branch}` already exists");
    }

    git_output(repo, ["switch", "-c", branch])
        .with_context(|| format!("create repair branch `{branch}`"))?;

    let mut next = state.clone();
    next.head_branch = branch.to_string();
    Ok(next)
}

/// Stage files, verify there are staged changes, and commit.
pub fn commit_repair(
    repo: &Path,
    state: &GitBranchState,
    files: &[String],
    title: &str,
    body: &str,
) -> Result<GitMutationReceipt> {
    if files.is_empty() {
        bail!("cannot commit repair with no files");
    }

    let mut add = Command::new("git");
    add.arg("-C").arg(repo).arg("add").arg("--").args(files);
    command_output(&mut add, "git add repair files")?;

    if git_status(repo, ["diff", "--cached", "--quiet"])?.success() {
        bail!("repair apply produced no staged changes");
    }

    let mut commit = Command::new("git");
    commit
        .arg("-C")
        .arg(repo)
        .arg("commit")
        .arg("-m")
        .arg(title)
        .arg("-m")
        .arg(body);
    command_output(&mut commit, "git commit repair")?;

    let head_sha = git_output(repo, ["rev-parse", "HEAD"])?.trim().to_string();

    Ok(GitMutationReceipt {
        status: "committed".to_string(),
        base_branch: state.base_branch.clone(),
        head_branch: state.head_branch.clone(),
        base_sha: state.base_sha.clone(),
        head_sha: head_sha.clone(),
        commit_sha: Some(head_sha),
        commit_title: title.to_string(),
        files_committed: files.to_vec(),
        rollback_command: rollback_command(repo, state),
        remote: state.remote.clone(),
        pushed: false,
    })
}

/// Rollback a repair branch by resetting to base SHA and deleting the branch.
pub fn rollback_repair_branch(repo: &Path, state: &GitBranchState) -> Result<GitMutationReceipt> {
    let mut errors = Vec::new();

    if let Err(error) = git_output(repo, ["reset", "--hard", &state.base_sha]) {
        errors.push(format!("reset failed: {error}"));
    }
    if let Err(error) = git_output(repo, ["switch", &state.base_branch]) {
        errors.push(format!("switch failed: {error}"));
    }
    if !state.head_branch.is_empty() {
        if let Err(error) = git_output(repo, ["branch", "-D", &state.head_branch]) {
            errors.push(format!("delete branch failed: {error}"));
        }
    }

    if !errors.is_empty() {
        bail!("repair rollback encountered errors: {}", errors.join("; "));
    }

    Ok(GitMutationReceipt {
        status: "rolled-back".to_string(),
        base_branch: state.base_branch.clone(),
        head_branch: state.head_branch.clone(),
        base_sha: state.base_sha.clone(),
        head_sha: state.base_sha.clone(),
        commit_sha: None,
        commit_title: "rolled back repair before commit".to_string(),
        files_committed: Vec::new(),
        rollback_command: rollback_command(repo, state),
        remote: state.remote.clone(),
        pushed: false,
    })
}

/// Push a branch to its remote.
pub fn push_branch(repo: &Path, remote: &str, branch: &str) -> Result<()> {
    git_output(repo, ["push", "-u", remote, branch])
        .with_context(|| format!("push repair branch `{branch}` to `{remote}`"))?;
    Ok(())
}

/// Create a GitHub draft PR using the `gh` CLI. Returns a receipt regardless
/// of success or failure (never panics on gh errors).
pub fn create_github_draft_pr(
    repo: &Path,
    remote: &str,
    base_branch: &str,
    head_branch: &str,
    title: &str,
    body_file: &Path,
) -> GithubPrReceipt {
    let command = vec![
        "gh".to_string(),
        "pr".to_string(),
        "create".to_string(),
        "--draft".to_string(),
        "--base".to_string(),
        base_branch.to_string(),
        "--head".to_string(),
        head_branch.to_string(),
        "--title".to_string(),
        title.to_string(),
        "--body-file".to_string(),
        body_file.display().to_string(),
    ];

    let mut process = Command::new("gh");
    process
        .current_dir(repo)
        .arg("pr")
        .arg("create")
        .arg("--draft")
        .arg("--base")
        .arg(base_branch)
        .arg("--head")
        .arg(head_branch)
        .arg("--title")
        .arg(title)
        .arg("--body-file")
        .arg(body_file);

    match process.output() {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let url = stdout
                .lines()
                .rev()
                .find(|line| line.trim_start().starts_with("http"))
                .map(|line| line.trim().to_string())
                .or_else(|| (!stdout.is_empty()).then_some(stdout));
            GithubPrReceipt {
                status: "created".to_string(),
                draft: true,
                url,
                remote: remote.to_string(),
                base_branch: base_branch.to_string(),
                head_branch: head_branch.to_string(),
                command,
                error: None,
            }
        }
        Ok(output) => GithubPrReceipt {
            status: "failed".to_string(),
            draft: true,
            url: None,
            remote: remote.to_string(),
            base_branch: base_branch.to_string(),
            head_branch: head_branch.to_string(),
            command,
            error: Some(String::from_utf8_lossy(&output.stderr).trim().to_string()),
        },
        Err(error) => GithubPrReceipt {
            status: "failed".to_string(),
            draft: true,
            url: None,
            remote: remote.to_string(),
            base_branch: base_branch.to_string(),
            head_branch: head_branch.to_string(),
            command,
            error: Some(error.to_string()),
        },
    }
}

/// Write a PR body to a temporary file for use with `gh pr create --body-file`.
pub fn write_pr_body(repo: &Path, body: &str) -> Result<PathBuf> {
    let path = repo.join("target/jankurai/repair-pr-body.md");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, body)?;
    Ok(path)
}

/// Validate that a branch name is safe for repair use.
pub fn validate_branch_name(repo: &Path, branch: &str) -> Result<()> {
    if !branch.starts_with("jankurai/repair/") {
        bail!("repair branch `{branch}` must start with `jankurai/repair/`");
    }
    if branch.contains(char::is_whitespace)
        || branch.contains("..")
        || branch.contains("@{")
        || branch.ends_with('/')
        || branch.starts_with('-')
    {
        bail!("repair branch `{branch}` is not safe");
    }
    if !git_status(repo, ["check-ref-format", "--branch", branch])?.success() {
        bail!("repair branch `{branch}` is not a valid git branch name");
    }
    Ok(())
}

fn rollback_command(repo: &Path, state: &GitBranchState) -> String {
    format!(
        "git -C {} reset --hard {} && git -C {} switch {} && git -C {} branch -D {}",
        repo.display(),
        state.base_sha,
        repo.display(),
        state.base_branch,
        repo.display(),
        state.head_branch
    )
}

fn git_output<I, S>(repo: &Path, args: I) -> Result<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let args_vec = args
        .into_iter()
        .map(|arg| arg.as_ref().to_string_lossy().to_string())
        .collect::<Vec<_>>();
    let mut command = Command::new("git");
    command.arg("-C").arg(repo).args(&args_vec);
    command_output(&mut command, &format!("git {}", args_vec.join(" ")))
}

fn git_status<I, S>(repo: &Path, args: I) -> Result<ExitStatus>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let args_vec = args
        .into_iter()
        .map(|arg| arg.as_ref().to_string_lossy().to_string())
        .collect::<Vec<_>>();
    Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(&args_vec)
        .status()
        .with_context(|| format!("git {}", args_vec.join(" ")))
}

fn command_output(command: &mut Command, label: &str) -> Result<String> {
    let output = command.output().with_context(|| label.to_string())?;
    if !output.status.success() {
        bail!(
            "{} failed with status {}: {}",
            label,
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
