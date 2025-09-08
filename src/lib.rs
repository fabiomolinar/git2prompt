pub mod git_utils;
pub mod io_utils;
pub mod processing;
pub mod repository;

use futures::future::join_all;
use io_utils::{ensure_directories, read_ignore_patterns};
use processing::process_single_repository;
use repository::Repository;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;

/// Processes a list of GitHub URLs concurrently, downloads and processes content,
/// and prepares it for AI tools.
pub async fn process_github_urls(
    urls: Vec<String>,
    no_headers: bool,
    merge_files: bool,
    ignore_file: Option<PathBuf>,
    folder: Option<String>,
    pr: Option<u32>,
) -> Result<Vec<PathBuf>, String> {
    println!(
        "Library received URLs: {:?}, no_headers: {}, merge_files: {}",
        urls, no_headers, merge_files
    );

    // Prepare directories
    let download_dir = PathBuf::from("./temp_repos");
    let output_dir = PathBuf::from("./output");
    ensure_directories(&download_dir, &output_dir).await?;

    // Read ignore patterns
    let ignore_patterns = Arc::new(read_ignore_patterns(ignore_file).await?);

    // Spawn processing tasks
    let tasks: Vec<_> = urls
        .iter()
        .map(|url| {
            let repository = Repository::new(&download_dir, url);
            let ignore_patterns = Arc::clone(&ignore_patterns);
            let folder = folder.clone();            
            tokio::spawn(async move {
                process_single_repository(repository, no_headers, merge_files, ignore_patterns, folder, pr)
                    .await
            })
        })
        .collect();

    let results = join_all(tasks).await;

    let mut repositories = Vec::new();
    for result in results {
        match result {
            Ok(Ok(repo)) => repositories.push(repo),
            Ok(Err(e)) => return Err(format!("Failed to process a repository: {}", e)),
            Err(e) => return Err(format!("Task failed unexpectedly: {}", e)),
        }
    }

    let output_paths = processing::handle_results(repositories, merge_files, &output_dir).await?;

    // Cleanup temporary directory
    fs::remove_dir_all(&download_dir)
        .await
        .map_err(|e| format!("Failed to remove temporary download directory: {}", e))?;

    Ok(output_paths)
}
