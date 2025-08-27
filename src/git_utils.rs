use crate::repository::Repository;
use git2::Repository as Git2Repository;
use tokio::fs;

pub async fn clone_repository(repository: &Repository) -> Result<Git2Repository, String> {
    let repo_url = repository.url.clone();
    let path = repository.path.clone();

    // Remove existing folder if it exists
    if repository.path.exists() {
        fs::remove_dir_all(&repository.path)
            .await
            .map_err(|e| format!("Failed to remove existing directory {:?}: {}", repository.path, e))?;
    }

    tokio::task::spawn_blocking(move || {
        Git2Repository::clone(&repo_url, &path)
            .map_err(|e| format!("Git clone error: {}", e))
    })
    .await
    .map_err(|e| format!("Blocking task join error: {}", e))?
}
