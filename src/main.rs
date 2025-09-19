use clap::Parser;
use git2prompt::{process_github_urls, process_local_path};
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
    /// Each line in the file should be a path or pattern to be excluded.
    #[clap(
        long,
        value_name = "PATH TO IGNORE FILE",
        default_value = ".git2promptignore"
    )]
    ignore_file: PathBuf,

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

    let result = if args.local {
        // --- LOCAL PATH MODE ---
        if args.sources.len() != 1 {
            // Using Box::from to create a valid error type
            return Err(Box::from(
                "Error: When using --local, exactly one directory path must be provided.",
            ));
        }
        let local_path = PathBuf::from(&args.sources[0]);
        println!("Processing local repository at: {:?}", local_path);
        println!("No file headers: {}", args.no_headers);
        println!("Ignore file path: {:?}", args.ignore_file);
        println!("----------------------------------------");
        process_local_path(
            local_path,
            args.no_headers,
            Some(args.ignore_file),
            args.folder,
        )
        .await
    } else {
        // --- GITHUB URL MODE (default) ---
        println!("Repositories to process: {:?}", args.sources);
        println!("No file headers: {}", args.no_headers);
        println!("Merge into a single output file: {}", args.merge_files);
        println!("Ignore file path: {:?}", args.ignore_file);
        println!("Folder to process: {:?}", args.folder);
        println!("Pull request number: {:?}", args.pr);
        println!("----------------------------------------");
        process_github_urls(
            args.sources,
            args.no_headers,
            args.merge_files,
            Some(args.ignore_file),
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