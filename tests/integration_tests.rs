use git2prompt::{processing, io_utils};
use std::path::PathBuf;
use tokio::fs;
use std::fs as stdfs;

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
    fs::write(path.join("src/main.rs"), "fn main() { println!(\"Hello\"); }").await?;
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

	let content_with_headers = process_repository_files(&test_repo_path, false, false, &Vec::new()).await.unwrap();
	assert!(content_with_headers.contains(&format!("## File: {}", src_main_path.display())));
	assert!(content_with_headers.contains("fn main() { println!(\"Hello\"); }"));
	assert!(content_with_headers.contains(&format!("## File: {}", readme_path.display())));
	assert!(content_with_headers.contains("# Test Repo"));

	let content_no_headers = process_repository_files(&test_repo_path, true, false, &Vec::new()).await.unwrap();
	assert!(!content_no_headers.contains(&format!("## File: {}", src_main_path.display())));
	assert!(content_no_headers.contains("fn main() { println!(\"Hello\"); }"));
	assert!(!content_no_headers.contains(&format!("## File: {}", readme_path.display())));
	assert!(content_no_headers.contains("# Test Repo"));

	let content_with_headers_merged = process_repository_files(&test_repo_path, false, true, &Vec::new()).await.unwrap();
	assert!(content_with_headers_merged.contains(&format!("### File: {}", src_main_path.display())));
	assert!(content_with_headers_merged.contains("fn main() { println!(\"Hello\"); }"));
	assert!(content_with_headers_merged.contains(&format!("### File: {}", readme_path.display())));
	assert!(content_with_headers_merged.contains("# Test Repo"));

	let content_no_headers_merged = process_repository_files(&test_repo_path, true, true, &Vec::new()).await.unwrap();
	assert!(!content_no_headers_merged.contains(&format!("### File: {}", src_main_path.display())));
	assert!(content_no_headers_merged.contains("fn main() { println!(\"Hello\"); }"));
	assert!(!content_no_headers_merged.contains(&format!("### File: {}", readme_path.display())));
	assert!(content_no_headers_merged.contains("# Test Repo"));

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
async fn test_ignore_patterns_cross_platform() -> Result<(), Box<dyn std::error::Error>> {
    let test_repo_path = PathBuf::from("test_ignore_repo");
    let _cleanup = TestCleanup::new(&test_repo_path);

    fs::create_dir_all(test_repo_path.join("data")).await?;
    fs::create_dir_all(test_repo_path.join("src")).await?;
    fs::write(test_repo_path.join("data/secret.txt"), "This is a secret.").await?;
    fs::write(test_repo_path.join("README.md"), "# README").await?;
    fs::write(test_repo_path.join("src/main.rs"), "fn main() {}").await?;

    let ignore_patterns = vec![
        "data/".to_string(),
        "README.md".to_string(),
        "src\\main.rs".to_string()
    ];

    let content = processing::process_repository_files(&test_repo_path, true, true, &ignore_patterns).await?;

    // Ignored files should not be in the output
    assert!(!content.contains("secret.txt"));
    assert!(!content.contains("README.md"));
    assert!(!content.contains("main.rs"));

    // Remaining content is empty
    assert!(content.is_empty());

    Ok(())
}
