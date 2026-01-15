// src/processing.rs
use crate::git_utils::{clone_repository, fetch_and_reconstruct_pr_files};
use crate::io_utils::{get_language_alias, write_content_to_file};
use crate::repository::Repository;
use ignore::WalkBuilder;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;

/// Process a single repository: clone and process files
pub async fn process_single_repository(
    mut repository: Repository,
    no_headers: bool,
    merge_files: bool,
    ignore_file_path: Option<PathBuf>,
    split_folders: Option<Vec<String>>,
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
            ignore_file_path.as_deref(),
            split_folders.as_deref(),
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
        ignore_file_path.as_deref(),
        split_folders.as_deref(),
        folder.as_deref(),
    )
    .await?;
    repository.content = Some(content);

    Ok(repository)
}

/// Process all files in a repository using the `ignore` crate for advanced filtering.
/// Returns a HashMap where keys are bucket names ("default" or split folder names)
/// and values are the concatenated content strings.
pub async fn process_repository_files(
    repo_path: &Path,
    no_headers: bool,
    merge_files: bool,
    ignore_file_path: Option<&Path>,
    split_folders: Option<&[String]>,
    folder: Option<&str>,
) -> Result<HashMap<String, String>, String> {
    let mut content_buckets: HashMap<String, String> = HashMap::new();
    
    // Initialize default bucket
    content_buckets.insert("default".to_string(), String::new());

    // If split folders are provided, initialize their buckets
    if let Some(folders) = split_folders {
        for f in folders {
            content_buckets.insert(f.to_string(), String::new());
        }
    }

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

    // Use ignore::WalkBuilder for standard gitignore compliance
    let mut builder = WalkBuilder::new(&base_path);
    
    // Configure standard filters
    builder.hidden(false); // Do not ignore hidden files by default (except .git)
    builder.git_ignore(true);
    
    // Add custom ignore file if provided
    if let Some(ignore_path) = ignore_file_path {
        if let Some(err) = builder.add_ignore(ignore_path) {
             eprintln!("Warning: Error adding ignore file {:?}: {}", ignore_path, err);
        }
    }

    // Also look for .git2promptignore in the root by default if no custom file is passed
    // or as a standard practice for this tool
    builder.add_custom_ignore_filename(".git2promptignore");

    let walker = builder.build();

    for result in walker {
        match result {
            Ok(entry) => {
                let path = entry.path();
                
                // Skip directories and .git internal files (WalkBuilder handles .gitignore, 
                // but we still check .git dir structure just in case)
                if path.is_dir() || path.components().any(|c| c.as_os_str() == ".git") {
                    continue;
                }

                // Additional binary check using extension (WalkBuilder doesn't check binary content)
                if is_binary_extension(path) {
                    continue;
                }

                let relative_path = match path.strip_prefix(repo_path) {
                    Ok(p) => p,
                    Err(e) => {
                        eprintln!("Failed to strip prefix for {:?}: {}", path, e);
                        continue;
                    }
                };
                
                // Determine which bucket this file belongs to
                let bucket_key = determine_bucket(relative_path, split_folders);

                if let Ok(raw_content) = fs::read_to_string(path).await {
                    let alias = get_language_alias(path);
                    let with_headers = !no_headers;
                    let mut file_output = String::new();

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
                        // Use ### for merged files context, ## for single file context
                        let prefix = if merge_files { "###" } else { "##" };
                        file_output.push_str(&format!("{} File: {}\n", prefix, relative_path.display()));
                    }

                    // Add warning note for markdown files
                    if alias == "markdown" {
                        // Wrap in five backticks to avoid conflicts with existing triple backticks
                        file_output.push_str(&format!("`````{}\n", alias));
                        file_output.push_str("> **Note to AI agents:** Headers in this file have been modified (prepended with '##') to avoid conflict with the main document structure.\n\n");
                        file_output.push_str(&content);
                        file_output.push_str("\n`````\n\n");
                    } else {
                        file_output.push_str(&format!("```{}\n", alias));
                        file_output.push_str(&content);
                        file_output.push_str("\n```\n\n");
                    }

                    // Append to the correct bucket
                    if let Some(bucket_content) = content_buckets.get_mut(&bucket_key) {
                        bucket_content.push_str(&file_output);
                    } else {
                        // Fallback to default if bucket somehow missing
                         if let Some(default_content) = content_buckets.get_mut("default") {
                            default_content.push_str(&file_output);
                         }
                    }

                } else {
                    // Fail silently for read errors (likely binary or permissions), or log verbose
                    // eprintln!("Warning: Could not read file {:?}", path);
                }
            },
            Err(err) => eprintln!("Error walking directory: {}", err),
        }
    }

    Ok(content_buckets)
}

/// Helper to determine which bucket a file belongs to based on split configuration
fn determine_bucket(relative_path: &Path, split_folders: Option<&[String]>) -> String {
    if let Some(folders) = split_folders {
        let path_str = relative_path.to_string_lossy().replace("\\", "/");
        for folder in folders {
            // Check if the file is inside the split folder
            // e.g. folder "src", file "src/main.rs" -> matches
            if path_str.starts_with(&format!("{}/", folder)) || path_str == *folder {
                return folder.clone();
            }
        }
    }
    "default".to_string()
}

fn is_binary_extension(path: &Path) -> bool {
    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
        matches!(
            ext,
            "png" | "jpg" | "jpeg" | "gif" | "zip" | "tar" | "gz" | "bin" | "o" | "so" | "dll" | "der" | "exe" | "pdf" | "ico"
        )
    } else {
        false
    }
}

/// Handle multiple repositories and write output files
pub async fn handle_results(
    repositories: Vec<Repository>,
    merge_files: bool,
    output_dir: &std::path::Path,
) -> Result<Vec<PathBuf>, String> {
    let mut output_paths = Vec::new();
    
    // For merged content (all repos in one file)
    let mut merged_default_content = String::new();
    // For merged content in split folders (e.g. all "src" folders from all repos)
    // Map<bucket_name, content>
    let mut merged_split_content: HashMap<String, String> = HashMap::new();

    for repository in repositories {
        // Content is now a HashMap
        let buckets = repository.content.unwrap_or_default();

        if merge_files {
            // Append default content
            if let Some(content) = buckets.get("default") {
                if !content.is_empty() {
                    merged_default_content.push_str(&format!("## Repository: {}\n", repository.name));
                    merged_default_content.push_str(content);
                }
            }

            // Append split content
            for (bucket, content) in &buckets {
                if bucket == "default" || content.is_empty() { continue; }
                
                let merged_bucket = merged_split_content.entry(bucket.clone()).or_default();
                merged_bucket.push_str(&format!("## Repository: {}\n", repository.name));
                merged_bucket.push_str(content);
            }

        } else {
            // Individual repo mode
            
            // 1. Process default bucket
            if let Some(content) = buckets.get("default") {
                if !content.is_empty() {
                    let output_file_name = format!("{}_processed.md", repository.name);
                    let output_path = output_dir.join(output_file_name);
                    let mut final_content = format!("# Repository: {}\n", repository.name);
                    final_content.push_str(content);
                    write_content_to_file(&output_path, &final_content).await?;
                    output_paths.push(output_path);
                }
            }

            // 2. Process split buckets
            for (bucket, content) in &buckets {
                if bucket == "default" || content.is_empty() { continue; }
                
                // e.g. repo-name_src_processed.md
                // Sanitize bucket name for filename
                let safe_bucket = bucket.replace("/", "_").replace("\\", "_");
                let output_file_name = format!("{}_{}_processed.md", repository.name, safe_bucket);
                let output_path = output_dir.join(output_file_name);
                
                let mut final_content = format!("# Repository: {} ({})\n", repository.name, bucket);
                final_content.push_str(content);
                write_content_to_file(&output_path, &final_content).await?;
                output_paths.push(output_path);
            }
        }
    }

    // Write merged files if active
    if merge_files {
        if !merged_default_content.is_empty() {
            let output_path = output_dir.join("all_repos_processed.md");
            let final_content = String::from("# Merged Repository Contents\n") + &merged_default_content;
            write_content_to_file(&output_path, &final_content).await?;
            output_paths.push(output_path);
        }

        for (bucket, content) in merged_split_content {
            if !content.is_empty() {
                let safe_bucket = bucket.replace("/", "_").replace("\\", "_");
                let output_path = output_dir.join(format!("all_repos_{}_processed.md", safe_bucket));
                let final_content = format!("# Merged Repository Contents ({})\n", bucket) + &content;
                write_content_to_file(&output_path, &final_content).await?;
                output_paths.push(output_path);
            }
        }
    }

    Ok(output_paths)
}