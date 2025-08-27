use futures::future::join_all;
use std::path::{Path, PathBuf};
use tokio::fs::{self, File};
use tokio::io::AsyncWriteExt;
use git2::Repository;
use walkdir::WalkDir;

/// Processes a list of GitHub URLs concurrently, downloads and processes content,
/// and prepares it for AI tools.
/// Returns the paths to the generated output files, or an error.
pub async fn process_github_urls(
    urls: Vec<String>,
    no_headers: bool,
    merge_files: bool,
) -> Result<Vec<PathBuf>, String> {
    println!("Library received URLs: {:?}, no_headers: {}, merge_files: {}", urls, no_headers, merge_files);

    let download_dir = PathBuf::from("./temp_repos"); // Temporary directory for cloning
    let output_dir = PathBuf::from("./output"); // Directory for final output files

    // Create the temporary download directory if it doesn't exist
    fs::create_dir_all(&download_dir)
        .await
        .map_err(|e| format!("Failed to create download directory: {}", e))?;
    fs::create_dir_all(&output_dir)
        .await
        .map_err(|e| format!("Failed to create output directory: {}", e))?;

    // Prepare tasks for concurrent downloading and processing
    let processing_tasks: Vec<_> = urls.iter().map(|url| {
        let repo_url = format!("https://github.com/{}.git", url);
        let repo_name = url.replace("/", "-");
        let clone_path = download_dir.join(&repo_name);
        let no_headers_clone = no_headers;
        
        // Spawn a new task that handles both clone and file processing
        tokio::spawn(async move {
            println!("Preparing to clone {} to {:?}", repo_url, clone_path);
            
            // Clone the repository
            clone_repository(&repo_url, &clone_path).await?;
            println!("Successfully cloned {} to {:?}", repo_name, clone_path);
            
            // Process the files in the cloned repository
            let content = process_repository_files(&clone_path, no_headers_clone).await?;
            
            // Return the processed content and its associated info
            Ok::<(String, String), String>((repo_name, content))
        })
    }).collect();

    // Await all processing tasks concurrently
    let results = join_all(processing_tasks).await;

    let mut output_paths = Vec::new();
    let mut all_processed_content = String::new();

    for result in results {
        match result {
            Ok(Ok((repo_name, content))) => {
                if merge_files {
                    all_processed_content.push_str(&format!("\n\n--- Repository: {} ---\n\n", repo_name));
                    all_processed_content.push_str(&content);
                } else {
                    let output_file_name = format!("{}_processed.txt", repo_name);
                    let output_path = output_dir.join(output_file_name);
                    write_content_to_file(&output_path, &content).await?;
                    output_paths.push(output_path);
                }
            },
            Ok(Err(e)) => return Err(format!("Failed to process a repository: {}", e)),
            Err(e) => return Err(format!("Task failed unexpectedly: {}", e)), // Handle tokio::task::JoinError
        }
    }

    if merge_files && !all_processed_content.is_empty() {
        let output_path = output_dir.join("all_repos_processed.txt");
        write_content_to_file(&output_path, &all_processed_content).await?;
        output_paths.push(output_path);
    }

    // Clean up the temporary download directory
    fs::remove_dir_all(&download_dir)
        .await
        .map_err(|e| format!("Failed to remove temporary download directory: {}", e))?;

    Ok(output_paths)
}

// Clones a Git repository asynchronously
async fn clone_repository(repo_url: &str, path: &Path) -> Result<Repository, String> {
    let repo_url_owned = repo_url.to_string();
    let path_owned = path.to_path_buf();
    tokio::task::spawn_blocking(move || {
        Repository::clone(&repo_url_owned, &path_owned)
            .map_err(|e| format!("Git clone error: {}", e))
    })
    .await
    .map_err(|e| format!("Blocking task join error: {}", e))?
}

// Processes all files in a cloned repository, concatenating their content.
async fn process_repository_files(repo_path: &Path, no_headers: bool) -> Result<String, String> {
    let mut combined_content = String::new();

    for entry in WalkDir::new(repo_path).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() {
            // Basic filtering: ignore common non-source files and git artifacts
            if let Some(ext) = path.extension().and_then(|s| s.to_str())
                && matches!(ext, "png" | "jpg" | "jpeg" | "gif" | "zip" | "tar" | "gz" | "bin" | "o" | "so" | "dll") {
                    continue; // Skip binary or archive files
            }
            if path.to_string_lossy().contains(".git/") {
                continue; // Skip git internal files
            }

            if let Ok(content) = fs::read_to_string(path).await {
                let relative_path = path.strip_prefix(repo_path)
                    .map_err(|e| format!("Failed to strip prefix: {}", e))?;

                if !no_headers {
                    combined_content.push_str(&format!("\n\n--- File: {} ---\n\n", relative_path.display()));
                }
                combined_content.push_str(&content);
                combined_content.push('\n'); // Add a newline after each file's content
            } else {
                eprintln!("Warning: Could not read file {:?}", path);
            }
        }
    }
    Ok(combined_content)
}

// Writes content to a specified file.
async fn write_content_to_file(path: &Path, content: &str) -> Result<(), String> {
    let mut file = File::create(path)
        .await
        .map_err(|e| format!("Failed to create output file {:?}: {}", path, e))?;
    file.write_all(content.as_bytes())
        .await
        .map_err(|e| format!("Failed to write to output file {:?}: {}", path, e))?;
    Ok(())
}