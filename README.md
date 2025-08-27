# git2prompt

git2prompt is a command-line tool that takes a GitHub repository URL, downloads its contents, and generates a single text file optimized for use as input to AI tools.

## How to use it

Refer to `git2prompt --help` or `cargo run -- --help` for more information.

## Filtering

To remove certain files or folders from the output, you can add them to the `.git2promptignore` file. Alternatively, you can also pass a file path to the `--ignore-file` flag.


Certain files and folders are automatically ignored by git2prompt:

- `.git` internal files.
- Non-text files extensions: "png" | "jpg" | "jpeg" | "gif" | "zip" | "tar" | "gz" | "bin" | "o" | "so" | "dll"

## Rust reminders

As I am starting my journey with Rust, here it goes a few reminders:

- To create a new project, run `cargo new <project-name>`.
- To build the project, run `cargo build`.
- To run the project in dev mode, run `cargo run`.
- To run the project in release mode, run `cargo run --release`.
- To check the code without building the final library, run `cargo check`.
- To run tests, run `cargo test`.
- To run Rust built-in linters, run `cargo clippy` (run with `--fix` to automatically fix the issues).
- To run the tests with a specific test file, run `cargo test <test-file>`.
- To run the tests with a specific test function, run `cargo test <test-function>`.
