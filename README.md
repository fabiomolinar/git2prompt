# **git2prompt**

**git2prompt** is a command-line utility written in Rust that streamlines the process of preparing GitHub repository content for large language models (LLMs). It clones repositories, filters out unnecessary files, and concatenates the source code into a single, clean markdown file, ready to be used as context for AI tools.

## **Features**

- **Seamless Integration:** Simply provide one or more GitHub repository URLs to get started.  
- **Content Concatenation:** Merges all processed files into a single output file, or creates a separate file for each repository.  
- **Intelligent Filtering:** Automatically ignores common non-text files and the *.git* directory to ensure only relevant source code is included.  
- **Customizable Filtering:** Use a *.git2promptignore* file to specify additional files or directories to exclude from the output.  
- **Readability:** Automatically adds markdown headers and language-specific code fences to the output for enhanced readability by both humans and AI models.

## **How to Use It**

To get started, clone the repository and build the project with Cargo.

`cargo build --release`

After building, you can use the compiled binary directly.

The output files are stored within an `output` folder which is created where the binary is ran from.

### **Basic Usage**

To process a single repository and output a single file:

`git2prompt <owner/repo>`

For example:

`git2prompt rust-lang/rust-by-example`

### **Advanced Usage**

Process multiple repositories and merge their contents into a single file:

`git2prompt --merge-files rust-lang/rust rust-lang/book`

Use the `--no-headers` flag to remove the file path headers above each code block:

`git2prompt --no-headers rust-lang/rust-by-example`

Sometimes you only need a single folder from a repository (instead of downloading the entire repo and ignoring most files). Use the `--folder` flag to restrict processing to a single directory:

`git2prompt rust-lang/rust-by-example -f src`

You can also restrict processing to only the files impacted by a GitHub pull request.  

`git2prompt --pr 123 rust-lang/rust-by-example`

## **Filtering**

**git2prompt** automatically ignores certain common file types and directories to keep the output clean.

These are automatically ignored:

- The .git directory and its contents.  
- Binary file extensions: png, jpg, jpeg, gif, zip, tar, gz, bin, o, so, dll

To ignore additional files or directories, create a file named *.git2promptignore* in the same directory as the binary. The format is a simple list, with one file or folder per line.

For example, a `.git2promptignore` file might look like this:

```
assets/
docs/  
README.md
```

The paths are relative to the repository. In the example above, the `assets` and `docs` folders within the repository root folder and the `README.md` file would be excluded.

Alternatively, you can specify a custom ignore file using the `--ignore-file` flag:

`git2prompt --ignore-file my-custom-ignore.txt <owner/repo>`

## Rust reminders

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

Before pushing to *crates.io*, run the following:

1. `cargo fmt`
2. `cargo build`
3. `cargo test`
4. `cargo clippy`

If all good:

1. Update version on `Cargo.toml`.
2. Commit and push.
3. Run `cargo package` and then `cargo publish`.
