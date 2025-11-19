use std::{
    fmt,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug)]
pub struct Repository {
    pub url: String,
    pub name: String,
    pub path: PathBuf,
    pub content: Option<String>,
}

impl Repository {
    /// Creates a new Repository instance from a remote GitHub URL.
    pub fn new(base_download_dir: &Path, repo_url: &str) -> Self {
        let url = format!("https://github.com/{}.git", repo_url);
        let name = repo_url.replace("/", "-");
        let path = base_download_dir.join(&name);

        Self {
            url,
            name,
            path,
            content: None,
        }
    }

    /// Creates a new Repository instance from a local file system path.
    pub fn from_local_path(local_path: &Path) -> Self {
        // Canonicalize to resolve "." or relative paths to absolute paths
        // so we can extract the actual folder name.
        let abs_path = local_path.canonicalize().unwrap_or_else(|_| local_path.to_path_buf());
        
        let name = abs_path
            .file_name() // Get the final component of the path (the folder name)
            .and_then(|s| s.to_str()) // Convert it to a string slice
            .unwrap_or("local-repo") // Fallback if the name is not valid UTF-8
            .to_string();
        Self {
            url: "local".to_string(), // URL is not applicable
            name,
            path: local_path.to_path_buf(), // The path is the provided local path
            content: None,
        }
    }

    pub fn has_content(&self) -> bool {
        self.content.is_some()
    }
}

impl fmt::Display for Repository {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Repository(name: {}, url: {}, path: {:?}, has_content: {})",
            self.name,
            self.url,
            self.path,
            self.has_content()
        )
    }
}