use clap::Parser;
use git2prompt::process_github_urls;
use std::path::PathBuf;

/// A command-line tool to download GitHub repository contents and format them for AI tools.
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// GitHub repository URLs (e.g., "owner/repo"). Supports multiple URLs.
    #[clap(required = true)]
    urls: Vec<String>,

    /// Do not add file paths as headers above code blocks in the output file(s).
    #[clap(short, long, action)]
    no_headers: bool,

    /// Merge contents of all repositories into a single output file.
    /// If not set, a separate file will be created for each repository.
    #[clap(short, long, action)]
    merge_files: bool,

    /// Path to a file containing a list of files/folders to ignore.
    /// Each line in the file should be a path or pattern to be excluded.
    #[clap(
        long,
        value_name = "PATH TO IGNORE FILE",
        default_value = ".git2promptignore"
    )]
    ignore_file: PathBuf,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    println!("Starting git2prompt...");
    println!("Repositories to process: {:?}", args.urls);
    println!("No file headers: {}", args.no_headers);
    println!("Merge into a single output file: {}", args.merge_files);

    // Call the library function to process the URLs, wrapping the ignore file path in Some
    match process_github_urls(
        args.urls,
        args.no_headers,
        args.merge_files,
        Some(args.ignore_file),
    )
    .await
    {
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
