mod file_utils;

use file_utils::get_language_alias;
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
    ignore_file: Option<PathBuf>,
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

    // Read ignore patterns from the specified file if provided
    let ignore_patterns: Vec<String> = if let Some(path) = ignore_file {
        if path.exists() {
            let content = fs::read_to_string(&path)
                .await
                .map_err(|e| format!("Failed to read ignore file {:?}: {}", path, e))?;
            content.lines().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
        } else {
            eprintln!("Warning: Ignore file {:?} not found. Proceeding without ignore patterns.", path);
            Vec::new()
        }
    } else {
        Vec::new()
    };

    // Prepare tasks for concurrent downloading and processing
    let processing_tasks: Vec<_> = urls.iter().map(|url| {
        let repo_url = format!("https://github.com/{}.git", url);
        let repo_name = url.replace("/", "-");
        let clone_path = download_dir.join(&repo_name);
        let no_headers_clone = no_headers;
        let merge_files_clone = merge_files;
        let ignore_patterns_clone = ignore_patterns.clone();
        
        // Spawn a new task that handles both clone and file processing
        tokio::spawn(async move {
            println!("Preparing to clone {} to {:?}", repo_url, clone_path);
            
            // Clone the repository
            clone_repository(&repo_url, &clone_path).await?;
            println!("Successfully cloned {} to {:?}", repo_name, clone_path);
            
            // Process the files in the cloned repository
            let content = process_repository_files(&clone_path, no_headers_clone, merge_files_clone, ignore_patterns_clone).await?;
            
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
                    all_processed_content.push_str(&format!("## Repository: {}\n", repo_name));
                    all_processed_content.push_str(&content);
                } else {
                    let output_file_name = format!("{}_processed.md", repo_name);
                    let output_path = output_dir.join(output_file_name);
                    let mut final_content = String::from(format!("# Repository: {}\n", repo_name));
                    final_content.push_str(&content);
                    write_content_to_file(&output_path, &final_content).await?;
                    output_paths.push(output_path);
                }
            },
            Ok(Err(e)) => return Err(format!("Failed to process a repository: {}", e)),
            Err(e) => return Err(format!("Task failed unexpectedly: {}", e)), // Handle tokio::task::JoinError
        }
    }

    if merge_files && !all_processed_content.is_empty() {
        let output_path = output_dir.join("all_repos_processed.md");
        let final_content = String::from("# Merged Repository Contents\n") + &all_processed_content;
        write_content_to_file(&output_path, &final_content).await?;
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
async fn process_repository_files(repo_path: &Path, no_headers: bool, merge_files: bool, ignore_patterns: Vec<String>) -> Result<String, String> {
    let mut combined_content = String::new();

    for entry in WalkDir::new(repo_path).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        
        // Ignore .git directory and its contents by checking path components
        if path.components().any(|c| c.as_os_str() == ".git") {
            continue;
        }

        // Check if the file or its parent directory matches any ignore patterns
        let relative_path_str = path.strip_prefix(repo_path)
            .map_err(|e| format!("Failed to strip prefix: {}", e))?
            .to_string_lossy();
        
        if ignore_patterns.iter().any(|pattern| relative_path_str.contains(pattern)) {
            println!("Ignoring file/folder: {}", relative_path_str);
            continue;
        }

        if path.is_file() {
            // Basic filtering: ignore common non-source files
            if let Some(ext) = path.extension().and_then(|s| s.to_str())
                && matches!(ext, "png" | "jpg" | "jpeg" | "gif" | "zip" | "tar" | "gz" | "bin" | "o" | "so" | "dll") {
                    continue; // Skip binary or archive files
            }
            let with_headers = !no_headers;        

            if let Ok(content) = fs::read_to_string(path).await {
                let relative_path = path.strip_prefix(repo_path)
                    .map_err(|e| format!("Failed to strip prefix: {}", e))?;
                let alias = get_language_alias(path);
                if with_headers {
                    if merge_files {
                        combined_content.push_str(&format!("### File: {}\n", relative_path.display()));
                    } else {
                        combined_content.push_str(&format!("## File: {}\n", relative_path.display()));
                    }
                }
                combined_content.push_str(&format!("```{}\n", alias));
                combined_content.push_str(&content);
                combined_content.push_str("\n```\n\n"); // Add a newline after each file's content
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Result;
    use std::path::{Path, PathBuf};

    /// A helper struct that cleans up a file or directory when it goes out of scope.
    struct TestCleanup {
        path: PathBuf,
    }

    impl TestCleanup {
        fn new(path: &Path) -> Self {
            Self { path: path.to_path_buf() }
        }
    }

    /// Implement the Drop trait to ensure cleanup on success or failure.
    impl Drop for TestCleanup {
        fn drop(&mut self) {
            if self.path.exists() {
                if self.path.is_dir() {
                    if let Err(e) = fs::remove_dir_all(&self.path) {
                        eprintln!("Failed to clean up test directory {:?}: {}", self.path, e);
                    }
                } else if self.path.is_file() {
                    if let Err(e) = fs::remove_file(&self.path) {
                        eprintln!("Failed to clean up test file {:?}: {}", self.path, e);
                    }
                }
            }
        }
    }

    /// A helper function to create a dummy repository structure for testing.
    async fn setup_dummy_repo(path: &Path) -> Result<()> {
        fs::create_dir_all(path.join("src"))?;
        fs::write(path.join("src/main.rs"), "fn main() { println!(\"Hello\"); }")?;
        fs::write(path.join("README.md"), "# Test Repo")?;
        Ok(())
    }

    #[tokio::test]
    async fn test_process_repository_files() -> Result<()> {
        let test_repo_path = PathBuf::from("test_temp_repo");
        // Create a cleanup instance. It will automatically remove the directory when the function ends.
        let _cleanup = TestCleanup::new(&test_repo_path);
        setup_dummy_repo(&test_repo_path).await?;

        let src_main_path = PathBuf::from("src").join("main.rs");
        let readme_path = PathBuf::from("README.md");
        
        let content_with_headers = process_repository_files(&test_repo_path, false, false, Vec::new()).await.unwrap();
        assert!(content_with_headers.contains(&format!("## File: {}", src_main_path.display())));
        assert!(content_with_headers.contains("fn main() { println!(\"Hello\"); }"));
        assert!(content_with_headers.contains(&format!("## File: {}", readme_path.display())));
        assert!(content_with_headers.contains("# Test Repo"));

        let content_no_headers = process_repository_files(&test_repo_path, true, false, Vec::new()).await.unwrap();
        assert!(!content_no_headers.contains(&format!("## File: {}", src_main_path.display())));
        assert!(content_no_headers.contains("fn main() { println!(\"Hello\"); }"));
        assert!(!content_no_headers.contains(&format!("## File: {}", readme_path.display())));
        assert!(content_no_headers.contains("# Test Repo"));

        let content_with_headers_merged = process_repository_files(&test_repo_path, false, true, Vec::new()).await.unwrap();
        assert!(content_with_headers_merged.contains(&format!("### File: {}", src_main_path.display())));
        assert!(content_with_headers_merged.contains("fn main() { println!(\"Hello\"); }"));
        assert!(content_with_headers_merged.contains(&format!("### File: {}", readme_path.display())));
        assert!(content_with_headers_merged.contains("# Test Repo"));

        let content_no_headers_merged = process_repository_files(&test_repo_path, true, true, Vec::new()).await.unwrap();
        assert!(!content_no_headers_merged.contains(&format!("### File: {}", src_main_path.display())));
        assert!(content_no_headers_merged.contains("fn main() { println!(\"Hello\"); }"));
        assert!(!content_no_headers_merged.contains(&format!("### File: {}", readme_path.display())));
        assert!(content_no_headers_merged.contains("# Test Repo"));

        Ok(())
    }

    #[tokio::test]
    async fn test_write_content_to_file() -> Result<()> {
        let test_dir = PathBuf::from("test_temp_dir");
        let _cleanup = TestCleanup::new(&test_dir);
        fs::create_dir_all(&test_dir)?;
        
        let test_file = test_dir.join("test_output.txt");
        let test_content = "This is a test content.";
        write_content_to_file(&test_file, test_content).await.unwrap();

        let read_content = fs::read_to_string(&test_file)?;
        assert_eq!(read_content, test_content);

        Ok(())
    }
}