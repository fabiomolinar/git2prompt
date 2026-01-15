> **Note to AI agents:** Headers in this file have been modified (prepended with '##') to avoid conflict with the main document structure.

### **git2prompt**

**git2prompt** is a command-line utility written in Rust that streamlines the process of preparing GitHub repository content for large language models (LLMs). It clones repositories, filters out unnecessary files, and concatenates the source code into a single, clean markdown file, ready to be used as context for AI tools.

#### **Features**

- **Seamless Integration:** Simply provide one or more GitHub repository URLs to get started.  
- **Content Concatenation:** Merges all processed files into a single output file, or creates a separate file for each repository.  
- **Intelligent Filtering:** Automatically ignores common non-text files and the *.git* directory to ensure only relevant source code is included.  
- **Customizable Filtering:** Use a *.git2promptignore* file to specify additional files or directories to exclude from the output.
- **Advanced Ignore Logic:** Supports standard `.gitignore` syntax (glob patterns, negations, directory-specific rules) via the `ignore` crate.
- **Folder-based Splitting:** Automatically splits content from specific folders (e.g., `src`, `docs`) into separate output files for better context management.
- **Persistent Configuration:** Use a `.git2promptconfig` file (TOML) to save your preferences for ignore patterns, split folders, and more.
- **Readability:** Automatically adds markdown headers and language-specific code fences to the output for enhanced readability by both humans and AI models.
- **Smart Markdown Processing:** Automatically modifies headers in Markdown files (demoting them with `##`) to preserve the structural integrity of the final output. It also injects a warning note to inform the AI of these changes.
- **Context-Aware Naming:** When processing local directories, the tool automatically uses the actual folder name as the repository title in the output.

#### **How to Use It**

To get started, clone the repository and build the project with Cargo.

`cargo build --release`

After building, you can use the compiled binary directly.

The output files are stored within an `output` folder which is created where the binary is ran from.

##### **Basic Usage**

To process a single repository and output a single file:

`git2prompt <owner/repo>`

For example:

`git2prompt rust-lang/rust-by-example`

Or in case you have the repository on your local machine, then just run it with the `--local` flag.  For example:

`git2prompt --local .`

##### **Advanced Usage**

Process multiple repositories and merge their contents into a single file:

`git2prompt --merge-files rust-lang/rust rust-lang/book`

**Splitting Content by Folder:**

If you want to separate documentation or specific modules into their own files, use the `--split-folder` flag:

`git2prompt rust-lang/rust --split-folder src --split-folder docs`

This will generate files like `rust_processed.md` (default content), `rust_src_processed.md`, and `rust_docs_processed.md`.

**Custom Configuration:**

You can persist your preferences in a `.git2promptconfig` file (see below) or specify a custom config path:

`git2prompt --config my-config.toml rust-lang/rust`

Use the `--no-headers` flag to remove the file path headers above each code block:

`git2prompt --no-headers rust-lang/rust-by-example`

Sometimes you only need a single folder from a repository (instead of downloading the entire repo and ignoring most files). Use the `--folder` flag to restrict processing to a single directory:

`git2prompt rust-lang/rust-by-example -f src`

You can also restrict processing to only the files impacted by a GitHub pull request.  

`git2prompt --pr 123 rust-lang/rust-by-example`

#### **Filtering**

**git2prompt** automatically ignores certain common file types and directories to keep the output clean.

These are automatically ignored:

- The .git directory and its contents.  
- Binary file extensions: png, jpg, jpeg, gif, zip, tar, gz, bin, o, so, dll, exe, pdf, ico

To ignore additional files or directories, create a file named *.git2promptignore* in the same directory as the binary. The format supports standard **.gitignore** syntax (glob patterns, wildcards, negations).

For example:

```.git2promptignore
assets/
docs/*.pdf
!docs/important.txt
target/
```

Alternatively, you can specify a custom ignore file using the `--ignore-file` flag:

`git2prompt --ignore-file my-custom-ignore.txt <owner/repo>`

#### **Configuration File**

You can create a `.git2promptconfig` file in your working directory to save your preferences. This file uses TOML format.

Example `.git2promptconfig`:

```toml
# Default ignore patterns (supplementary to .git2promptignore)
ignore_patterns = ["tests/", "*.log"]

# Folders to always split into separate output files
split_folders = ["docs", "examples"]

# Default settings
no_headers = false
ignore_file = ".git2promptignore"
```

#### Rust reminders

As I am starting my journey with Rust, here it goes a few reminders so I don't have to Google them all the time:

- To create a new project, run `cargo new <project-name>`.
- To build the project, run `cargo build`.
- To run the project in dev mode, run `cargo run`.
- To run the project in release mode, run `cargo run --release`.
- To check the code without building the final library, run `cargo check`.
- To run tests, run `cargo test`.
- To run Rust built-in linters, run `cargo clippy` (run with `--fix` to automatically fix the issues).
- To run the tests with a specific test file, run `cargo test <test-file>`.
- To run the tests with a specific test function, run `cargo test <test-function>`.
- To install the crate locally from the source, run `cargo install --path .` from the root of the crate.

Before pushing to *crates.io*, run the following:

1. `cargo fmt`
2. `cargo build --release`
3. `cargo test`
4. `cargo clippy`

If all good:

1. Update version on `Cargo.toml`.
2. Commit and push.
3. Run `cargo package` and then `cargo publish`.

To update the CLI program binary from source, run `cargo install --path .`.
