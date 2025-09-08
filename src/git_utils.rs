use crate::repository::Repository;
use git2::Repository as Git2Repository;
use reqwest::Client;
use serde::Deserialize;
use std::path::Path;
use tokio::fs;

#[derive(Deserialize)]
struct GitHubPRFile {
    filename: String,
    patch: Option<String>, // not always present (binary files)
}

pub async fn clone_repository(repository: &Repository) -> Result<Git2Repository, String> {
    let repo_url = repository.url.clone();
    let path = repository.path.clone();

    // Remove existing folder if it exists
    if repository.path.exists() {
        fs::remove_dir_all(&repository.path).await.map_err(|e| {
            format!(
                "Failed to remove existing directory {:?}: {}",
                repository.path, e
            )
        })?;
    }

    tokio::task::spawn_blocking(move || {
        Git2Repository::clone(&repo_url, &path).map_err(|e| format!("Git clone error: {}", e))
    })
    .await
    .map_err(|e| format!("Blocking task join error: {}", e))?
}

pub async fn fetch_and_reconstruct_pr_files(
    repo: &str,
    pr_number: u32,
    base_path: &Path,
) -> Result<(), String> {
    let api_url = format!(
        "https://api.github.com/repos/{}/pulls/{}/files",
        repo, pr_number
    );

    let client = Client::new();
    let resp = client
        .get(&api_url)
        .header("User-Agent", "git2prompt")
        .send()
        .await
        .map_err(|e| format!("Failed to call GitHub API: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("GitHub API error: {}", resp.status()));
    }

    let files: Vec<GitHubPRFile> = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse GitHub API response: {}", e))?;

    for file in files {
        if let Some(patch) = file.patch {
            let file_path = base_path.join(&file.filename);
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent)
                    .await
                    .map_err(|e| e.to_string())?;
            }
            fs::write(&file_path, patch)
                .await
                .map_err(|e| e.to_string())?;
        } else {
            eprintln!("Skipping file {} (no patch, maybe binary)", file.filename);
        }
    }

    Ok(())
}
