use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct Repository {
    pub url: String,
    pub name: String,
    pub path: PathBuf,
    pub content: Option<String>,
}

impl Repository {
    pub fn new(base_download_dir: &Path, repo_url: &str) -> Self {
        let url = format!("https://github.com/{}.git", repo_url);
        let name = repo_url.replace("/", "-");
        let path = base_download_dir.join(&name);

        Self { url, name, path, content: None }
    }
}
