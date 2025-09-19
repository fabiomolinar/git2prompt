pub mod git_utils;
pub mod io_utils;
pub mod processing;
pub mod repository;

use futures::future::join_all;
use io_utils::{ensure_directories, read_ignore_patterns};
use processing::{process_repository_files, process_single_repository}; // Import process_repository_files
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
                process_single_repository(
                    repository,
                    no_headers,
                    merge_files,
                    ignore_patterns,
                    folder,
                    pr,
                )
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

/// Processes a single local directory path, prepares content, and writes to output.
pub async fn process_local_path(
    path: PathBuf,
    no_headers: bool,
    ignore_file: Option<PathBuf>,
    folder: Option<String>,
) -> Result<Vec<PathBuf>, String> {
    if !path.is_dir() {
        return Err(format!("Local path {:?} is not a directory.", path));
    }

    // Prepare output directory
    let output_dir = PathBuf::from("./output");
    ensure_directories(&PathBuf::new(), &output_dir).await?; // No download dir needed

    // Read ignore patterns
    let ignore_patterns = read_ignore_patterns(ignore_file).await?;

    // Create a repository object from the local path
    let mut repository = Repository::from_local_path(&path);
    // Print full path for debugging
    println!("Processing local repository at path: {:?}", repository.path);

    // Process the files in the local directory
    let content = process_repository_files(
        &repository.path,
        no_headers,
        false, // merge_files is false since there is only one repo
        &ignore_patterns,
        folder.as_deref(),
    )
    .await?;
    repository.content = Some(content);

    // Use handle_results to generate the output file
    let repositories = vec![repository];
    let output_paths = processing::handle_results(repositories, false, &output_dir).await?;

    Ok(output_paths)
}
