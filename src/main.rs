use clap::{Parser};
use git2prompt::process_github_urls;

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
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    println!("Starting git2prompt...");
    println!("Repositories to process: {:?}", args.urls);
    println!("No file headers: {}", args.no_headers);
    println!("Merge into a single output file: {}", args.merge_files);

    // Call the library function to process the URLs
    match process_github_urls(args.urls, args.no_headers, args.merge_files).await {
        Ok(output_paths) => {
            println!("Processing complete. Output files created:");
            for path in output_paths {
                println!(" - {:?}", path);
            }
            Ok(())
        }
        Err(e) => {
            eprintln!("Error during processing: {}", e);
            Err(e.into())
        }
    }
}
