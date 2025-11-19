use crate::git_utils::{clone_repository, fetch_and_reconstruct_pr_files};
use crate::io_utils::{get_language_alias, write_content_to_file};
use crate::repository::Repository;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use walkdir::WalkDir;

/// Process a single repository: clone and process files
pub async fn process_single_repository(
    mut repository: Repository,
    no_headers: bool,
    merge_files: bool,
    ignore_patterns: Arc<Vec<String>>,
    folder: Option<String>,
    pr: Option<u32>,
) -> Result<Repository, String> {
    // Case 1: PR mode → don’t clone repo, reconstruct from API
    if let Some(pr_number) = pr {
        println!(
            "Processing repository {} in PR mode (PR #{})",
            repository.url, pr_number
        );

        // Extract "owner/repo" from "https://github.com/owner/repo.git"
        let repo_name = repository
            .url
            .trim_end_matches(".git")
            .trim_start_matches("https://github.com/")
            .to_string();

        let pr_temp_path = repository.path.join(format!("pr-{}", pr_number));

        fetch_and_reconstruct_pr_files(&repo_name, pr_number, &pr_temp_path).await?;

        let content = process_repository_files(
            &pr_temp_path,
            no_headers,
            merge_files,
            &ignore_patterns,
            None, // folder restriction not applied in PR mode
        )
        .await?;

        repository.content = Some(content);
        return Ok(repository);
    }

    // Case 2: Normal mode → clone repo
    println!(
        "Preparing to clone {} to {:?}",
        repository.url, repository.path
    );
    clone_repository(&repository).await?;
    println!(
        "Successfully cloned {} to {:?}",
        repository.name, repository.path
    );

    let content = process_repository_files(
        &repository.path,
        no_headers,
        merge_files,
        &ignore_patterns,
        folder.as_deref(),
    )
    .await?;
    repository.content = Some(content);

    Ok(repository)
}

/// Process all files in a repository
pub async fn process_repository_files(
    repo_path: &std::path::Path,
    no_headers: bool,
    merge_files: bool,
    ignore_patterns: &[String],
    folder: Option<&str>,
) -> Result<String, String> {
    let mut combined_content = String::new();

    let base_path = if let Some(folder) = folder {
        repo_path.join(folder)
    } else {
        repo_path.to_path_buf()
    };

    if !base_path.exists() {
        return Err(format!(
            "Specified folder {:?} not found in repo",
            base_path
        ));
    }

    for entry in WalkDir::new(&base_path).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();

        if is_valid_file(path, repo_path, ignore_patterns) {
            let with_headers = !no_headers;

            if let Ok(raw_content) = fs::read_to_string(path).await {
                let relative_path = path
                    .strip_prefix(repo_path)
                    .map_err(|e| format!("Failed to strip prefix: {}", e))?;
                let alias = get_language_alias(path);

                // Adjust content if it is markdown to avoid header conflicts
                let content = if alias == "markdown" {
                    raw_content
                        .lines()
                        .map(|line| {
                            if line.trim_start().starts_with('#') {
                                format!("##{}", line)
                            } else {
                                line.to_string()
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                } else {
                    raw_content
                };

                if with_headers {
                    if merge_files {
                        combined_content
                            .push_str(&format!("### File: {}\n", relative_path.display()));
                    } else {
                        combined_content
                            .push_str(&format!("## File: {}\n", relative_path.display()));
                    }
                }
                combined_content.push_str(&format!("```{}\n", alias));
                combined_content.push_str(&content);
                combined_content.push_str("\n```\n\n");
            } else {
                eprintln!("Warning: Could not read file {:?}", path);
            }
        }
    }

    Ok(combined_content)
}

/// Handle multiple repositories and write output files
pub async fn handle_results(
    repositories: Vec<Repository>,
    merge_files: bool,
    output_dir: &std::path::Path,
) -> Result<Vec<PathBuf>, String> {
    let mut output_paths = Vec::new();
    let mut all_processed_content = String::new();

    for repository in repositories {
        let content = repository.content.unwrap_or_default();
        if merge_files {
            all_processed_content.push_str(&format!("## Repository: {}\n", repository.name));
            all_processed_content.push_str(&content);
        } else {
            let output_file_name = format!("{}_processed.md", repository.name);
            let output_path = output_dir.join(output_file_name);
            let mut final_content = format!("# Repository: {}\n", repository.name);
            final_content.push_str(&content);
            write_content_to_file(&output_path, &final_content).await?;
            output_paths.push(output_path);
        }
    }

    if merge_files && !all_processed_content.is_empty() {
        let output_path = output_dir.join("all_repos_processed.md");
        let final_content = String::from("# Merged Repository Contents\n") + &all_processed_content;
        write_content_to_file(&output_path, &final_content).await?;
        output_paths.push(output_path);
    }

    Ok(output_paths)
}

/// Check if a file should be processed
fn is_valid_file(
    path: &std::path::Path,
    repo_path: &std::path::Path,
    ignore_patterns: &[String],
) -> bool {
    if path.components().any(|c| c.as_os_str() == ".git") {
        return false;
    }
    if ignore_due_to_pattern(path, repo_path, ignore_patterns) {
        return false;
    }
    if !path.is_file() {
        return false;
    }

    if let Some(ext) = path.extension().and_then(|s| s.to_str())
        && matches!(
            ext,
            "png"
                | "jpg"
                | "jpeg"
                | "gif"
                | "zip"
                | "tar"
                | "gz"
                | "bin"
                | "o"
                | "so"
                | "dll"
                | "der"
        )
    {
        return false;
    }

    true
}

fn ignore_due_to_pattern(
    path: &std::path::Path,
    repo_path: &std::path::Path,
    ignore_patterns: &[String],
) -> bool {
    let relative_path_str = match path.strip_prefix(repo_path) {
        Ok(p) => p.to_string_lossy().replace("\\", "/"),
        Err(_) => return false,
    };
    ignore_patterns
        .iter()
        .any(|pattern| relative_path_str.starts_with(&pattern.replace("\\", "/")))
}