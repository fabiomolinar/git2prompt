// src/main.rs
use clap::Parser;
use git2prompt::{config::Config, process_github_urls, process_local_path};
use std::path::PathBuf;

/// A command-line tool to process repository contents and format them for AI tools.
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// GitHub repository URLs (e.g., "owner/repo") or a single local path with --local.
    #[clap(required = true)]
    sources: Vec<String>,

    /// Process the source as a local directory path instead of a GitHub URL.
    #[clap(short, long, action)]
    local: bool,

    /// Do not add file paths as headers above code blocks in the output file(s).
    #[clap(short, long, action)]
    no_headers: bool,

    /// Merge contents of all repositories into a single output file.
    /// Incompatible with --local.
    #[clap(short, long, action, conflicts_with = "local")]
    merge_files: bool,

    /// Path to a file containing a list of files/folders to ignore.
    /// Standard .gitignore syntax is supported.
    #[clap(long, value_name = "PATH", default_value = ".git2promptignore")]
    ignore_file: PathBuf,

    /// Configuration file path.
    #[clap(long, value_name = "CONFIG_PATH", default_value = ".git2promptconfig")]
    config: PathBuf,

    /// Specify folders to split into separate output files.
    /// Can be used multiple times (e.g., --split-folder src --split-folder docs)
    #[clap(long, value_name = "FOLDER")]
    split_folder: Vec<String>,

    /// Download and process only a specific folder within the repository.
    #[clap(short, long, value_name = "FOLDER PATH", conflicts_with = "pr")]
    folder: Option<String>,

    /// Process only the files changed in a specific pull request.
    /// Incompatible with --local.
    #[clap(long, value_name = "PULL REQUEST NUMBER", conflicts_with_all = ["folder", "local"])]
    pr: Option<u32>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    println!("Starting git2prompt...");

    // --- Smart Config Lookup Logic ---
    // 1. Start with the path provided by CLI or the default ".git2promptconfig"
    let mut config_path = args.config.clone();

    // 2. If that file doesn't exist, and we are in --local mode, check the target directory.
    if !config_path.exists() && args.local && args.sources.len() == 1 {
        let local_repo_config = PathBuf::from(&args.sources[0]).join(".git2promptconfig");
        if local_repo_config.exists() {
            println!(
                "Configuration file found in local repository: {:?}",
                local_repo_config
            );
            config_path = local_repo_config;
        }
    }

    // 3. Load configuration
    let config = Config::load_from_file(&config_path).await;

    // --- Merge Settings (CLI takes precedence) ---

    // Headers: CLI arg OR Config file OR default(false)
    let final_no_headers = if args.no_headers {
        true
    } else {
        config.no_headers.unwrap_or(false)
    };

    // Ignore file: CLI arg OR Config OR default
    // We check if the user provided a custom path or if we should fall back to config
    let final_ignore_file = if args.ignore_file.to_string_lossy() == ".git2promptignore"
        && config.ignore_file.is_some()
    {
        PathBuf::from(config.ignore_file.unwrap())
    } else {
        args.ignore_file
    };

    // Split folders: Merge CLI and Config
    let mut final_split_folders = config.split_folders.unwrap_or_default();
    final_split_folders.extend(args.split_folder);
    let final_split_folders_opt = if final_split_folders.is_empty() {
        None
    } else {
        Some(final_split_folders)
    };

    let result = if args.local {
        // --- LOCAL PATH MODE ---
        if args.sources.len() != 1 {
            return Err(Box::from(
                "Error: When using --local, exactly one directory path must be provided.",
            ));
        }
        let local_path = PathBuf::from(&args.sources[0]);
        let ignore_canonical = match final_ignore_file.canonicalize() {
            Ok(p) => Some(p),
            Err(_) => {
                // Warning only if it's a custom path that failed
                if final_ignore_file.to_string_lossy() != ".git2promptignore" {
                    eprintln!(
                        "Warning: Ignore file {:?} not found. Proceeding without it.",
                        final_ignore_file
                    );
                }
                None
            }
        };

        println!("Processing local repository at: {:?}", local_path);
        println!("No file headers: {}", final_no_headers);
        println!("Split folders: {:?}", final_split_folders_opt);
        println!("----------------------------------------");

        process_local_path(
            local_path,
            final_no_headers,
            ignore_canonical,
            final_split_folders_opt,
            args.folder,
        )
        .await
    } else {
        // --- GITHUB URL MODE (default) ---
        println!("Repositories to process: {:?}", args.sources);
        println!("No file headers: {}", final_no_headers);
        println!("Merge into a single output file: {}", args.merge_files);
        println!("Ignore file path: {:?}", final_ignore_file);
        println!("Split folders: {:?}", final_split_folders_opt);
        println!("Folder to process: {:?}", args.folder);
        println!("Pull request number: {:?}", args.pr);
        println!("----------------------------------------");

        process_github_urls(
            args.sources,
            final_no_headers,
            args.merge_files,
            Some(final_ignore_file), // Pass raw path, let logic handle existence
            final_split_folders_opt,
            args.folder,
            args.pr,
        )
        .await
    };

    match result {
        Ok(output_paths) => {
            println!("Processing complete. Output files created:");
            for path in output_paths {
                println!(" - {}", path.display());
            }
            Ok(())
        }
        Err(e) => {
            eprintln!("Error during processing: {}", e);
            Err(e.into())
        }
    }
}
