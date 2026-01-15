// tests/integration_tests.rs
use git2prompt::{io_utils, processing, repository::Repository};
use std::fs as stdfs;
use std::path::PathBuf;
use tokio::fs;

/// A helper struct that cleans up a file or directory when it goes out of scope.
struct TestCleanup {
    path: PathBuf,
}

impl TestCleanup {
    fn new(path: &PathBuf) -> Self {
        Self { path: path.clone() }
    }
}

impl Drop for TestCleanup {
    fn drop(&mut self) {
        if self.path.exists() {
            if self.path.is_dir() {
                if let Err(e) = stdfs::remove_dir_all(&self.path) {
                    eprintln!("Failed to clean up test directory {:?}: {}", self.path, e);
                }
            } else if self.path.is_file() {
                if let Err(e) = stdfs::remove_file(&self.path) {
                    eprintln!("Failed to clean up test file {:?}: {}", self.path, e);
                }
            }
        }
    }
}

/// Helper to create a dummy repository
async fn setup_dummy_repo(path: &PathBuf) -> std::io::Result<()> {
    fs::create_dir_all(path.join("src")).await?;
    fs::write(
        path.join("src/main.rs"),
        "fn main() { println!(\"Hello\"); }",
    )
    .await?;
    fs::write(path.join("README.md"), "# Test Repo").await?;
    Ok(())
}

#[tokio::test]
async fn test_process_repository_files() -> Result<(), Box<dyn std::error::Error>> {
    let test_repo_path = PathBuf::from("test_temp_repo");
    let _cleanup = TestCleanup::new(&test_repo_path);
    setup_dummy_repo(&test_repo_path).await?;

    use processing::process_repository_files;

    let src_main_path = PathBuf::from("src").join("main.rs");
    let readme_path = PathBuf::from("README.md");

    // Case 1: With headers
    let buckets_with_headers =
        process_repository_files(&test_repo_path, false, false, None, None, None)
            .await
            .unwrap();

    let content_with_headers = buckets_with_headers.get("default").expect("Default bucket missing");

    assert!(content_with_headers.contains(&format!("## File: {}", src_main_path.display())));
    assert!(content_with_headers.contains("fn main() { println!(\"Hello\"); }"));
    assert!(content_with_headers.contains(&format!("## File: {}", readme_path.display())));
    // Assert that Markdown headers were demoted (prepended with ##)
    assert!(content_with_headers.contains("### Test Repo"));
    // Assert that the warning note was added
    assert!(
        content_with_headers
            .contains("> **Note to AI agents:** Headers in this file have been modified")
    );

    // Case 2: No headers
    let buckets_no_headers =
        process_repository_files(&test_repo_path, true, false, None, None, None)
            .await
            .unwrap();
    
    let content_no_headers = buckets_no_headers.get("default").expect("Default bucket missing");

    assert!(!content_no_headers.contains(&format!("## File: {}", src_main_path.display())));
    assert!(content_no_headers.contains("fn main() { println!(\"Hello\"); }"));
    assert!(!content_no_headers.contains(&format!("## File: {}", readme_path.display())));
    // Check features in no-header mode as well
    assert!(content_no_headers.contains("### Test Repo"));
    assert!(
        content_no_headers
            .contains("> **Note to AI agents:** Headers in this file have been modified")
    );

    // Case 3: Merged files (Header check changes from ## to ###)
    let buckets_merged =
        process_repository_files(&test_repo_path, false, true, None, None, None)
            .await
            .unwrap();
    
    let content_merged = buckets_merged.get("default").expect("Default bucket missing");

    assert!(
        content_merged.contains(&format!("### File: {}", src_main_path.display()))
    );
    assert!(content_merged.contains("fn main() { println!(\"Hello\"); }"));
    assert!(content_merged.contains(&format!("### File: {}", readme_path.display())));
    assert!(content_merged.contains("### Test Repo"));
    assert!(
        content_merged
            .contains("> **Note to AI agents:** Headers in this file have been modified")
    );

    Ok(())
}

#[tokio::test]
async fn test_write_content_to_file() -> Result<(), Box<dyn std::error::Error>> {
    let test_dir = PathBuf::from("test_temp_dir");
    let _cleanup = TestCleanup::new(&test_dir);
    fs::create_dir_all(&test_dir).await?;

    let test_file = test_dir.join("test_output.txt");
    let test_content = "This is a test content.";
    io_utils::write_content_to_file(&test_file, test_content).await?;

    let read_content = fs::read_to_string(&test_file).await?;
    assert_eq!(read_content, test_content);

    Ok(())
}

#[tokio::test]
async fn test_ignore_patterns_file_based() -> Result<(), Box<dyn std::error::Error>> {
    // Tests the ignore crate integration using a real ignore file
    let test_repo_path = PathBuf::from("test_ignore_repo");
    let _cleanup = TestCleanup::new(&test_repo_path);

    fs::create_dir_all(test_repo_path.join("data")).await?;
    fs::create_dir_all(test_repo_path.join("src")).await?;
    fs::write(test_repo_path.join("data/secret.txt"), "This is a secret.").await?;
    fs::write(test_repo_path.join("README.md"), "# README").await?;
    fs::write(test_repo_path.join("src/main.rs"), "fn main() {}").await?;
    
    // Create a custom ignore file
    let ignore_file_name = ".customignore";
    let ignore_path = test_repo_path.join(ignore_file_name);
    
    // IMPORTANT:
    // 1. We must ignore the .customignore file itself, otherwise it appears in output.
    // 2. We use "*.rs" instead of "src/main.rs" to avoid Windows/Unix path separator mismatches in this specific test.
    let ignore_content = "data/\nREADME.md\n*.rs\n.customignore";
    fs::write(&ignore_path, ignore_content).await?;

    // Use current_dir to construct robust absolute paths
    let current_dir = std::env::current_dir()?;
    let abs_repo_path = current_dir.join(&test_repo_path);
    let abs_ignore_path = current_dir.join(&ignore_path);

    // Pass the ignore file path to the processor
    let buckets = processing::process_repository_files(
        &abs_repo_path, 
        true, 
        true, 
        Some(&abs_ignore_path), 
        None, 
        None
    ).await?;

    let content = buckets.get("default").unwrap();

    // Verify files were ignored using strict header check (safest) OR content check
    // "secret.txt" content is "This is a secret."
    assert!(!content.contains("This is a secret."), "Should NOT contain data/secret.txt content");
    
    // "README.md" content is "# README". 
    // We check for the header because the ignore file content itself might appear in debug logs or errors,
    // but here we ignored the ignore file too.
    assert!(!content.contains("## File: README.md"), "Should NOT contain README.md file header");
    assert!(!content.contains("# README"), "Should NOT contain README.md content");

    // "main.rs" content is "fn main() {}"
    assert!(!content.contains("fn main() {}"), "Should NOT contain main.rs content");

    Ok(())
}

#[tokio::test]
async fn test_git_ignore_advanced_syntax() -> Result<(), Box<dyn std::error::Error>> {
    // Tests glob patterns and negations supported by the ignore crate
    let test_repo_path = PathBuf::from("test_glob_repo");
    let _cleanup = TestCleanup::new(&test_repo_path);

    fs::create_dir_all(&test_repo_path).await?;
    
    // Create files
    fs::write(test_repo_path.join("keep.rs"), "keep").await?;
    fs::write(test_repo_path.join("ignore.log"), "log").await?;
    fs::write(test_repo_path.join("temp.swp"), "swp").await?;
    
    // Create a standard .git2promptignore file which is automatically picked up
    let ignore_content = "*.log\n*.swp";
    fs::write(test_repo_path.join(".git2promptignore"), ignore_content).await?;

    let buckets = processing::process_repository_files(
        &test_repo_path, 
        true, 
        false, 
        None, // No custom file, rely on .git2promptignore discovery
        None, 
        None
    ).await?;

    let content = buckets.get("default").unwrap();

    assert!(content.contains("keep"), "Should contain keep.rs");
    assert!(!content.contains("log"), "Should not contain .log file");
    assert!(!content.contains("swp"), "Should not contain .swp file");

    Ok(())
}

#[tokio::test]
async fn test_split_folders() -> Result<(), Box<dyn std::error::Error>> {
    let test_repo_path = PathBuf::from("test_split_repo");
    let _cleanup = TestCleanup::new(&test_repo_path);

    // Setup structure:
    // root/
    //   src/main.rs
    //   docs/info.md
    //   docs/internal/secret.md
    //   README.md
    
    fs::create_dir_all(test_repo_path.join("src")).await?;
    fs::create_dir_all(test_repo_path.join("docs/internal")).await?;

    fs::write(test_repo_path.join("src/main.rs"), "fn main() {}").await?;
    fs::write(test_repo_path.join("docs/info.md"), "Documentation").await?;
    fs::write(test_repo_path.join("docs/internal/deep.md"), "Deep Docs").await?;
    fs::write(test_repo_path.join("README.md"), "# Root").await?;

    let split_folders = vec!["docs".to_string()];

    let buckets = processing::process_repository_files(
        &test_repo_path, 
        true, 
        false, 
        None, 
        Some(&split_folders), 
        None
    ).await?;

    // Check "default" bucket
    let default_content = buckets.get("default").expect("Default bucket missing");
    assert!(default_content.contains("fn main() {}"));
    assert!(default_content.contains("# Root"));
    assert!(!default_content.contains("Documentation")); // Should be moved
    assert!(!default_content.contains("Deep Docs"));     // Should be moved

    // Check "docs" bucket
    let docs_content = buckets.get("docs").expect("Docs bucket missing");
    assert!(docs_content.contains("Documentation"));
    assert!(docs_content.contains("Deep Docs")); // Recursive split check
    assert!(!docs_content.contains("fn main() {}"));

    Ok(())
}

#[tokio::test]
async fn test_local_repo_name_resolution() -> Result<(), Box<dyn std::error::Error>> {
    // We need a directory with a specific name to test if canonicalize works
    let dir_name = "test_project_folder";
    let test_path = PathBuf::from(dir_name);
    let _cleanup = TestCleanup::new(&test_path);

    fs::create_dir_all(&test_path).await?;

    // Create the repository object from the local path
    let repo = Repository::from_local_path(&test_path);

    // The name should match the directory name, not "local-repo" or "."
    assert_eq!(repo.name, dir_name);
    assert_eq!(repo.url, "local");

    Ok(())
}