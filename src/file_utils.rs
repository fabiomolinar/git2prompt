use std::path::Path;

/// A helper function to map file extensions to a programming language alias.
/// The aliases are from the list of languages supported by Highlight.js.
/// Returns an empty string if no alias is found.
pub fn get_language_alias(path: &Path) -> &'static str {
    let extension = path.extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("");

    match extension.to_lowercase().as_str() {
        "sh" | "bash"                => "bash",
        "c"                          => "c",
        "cc" | "cxx" | "c++" | "cpp" => "cpp",
        "cs"                         => "csharp",
        "css"                        => "css",
        "go"                         => "go",
        "html" | "htm"               => "xml",
        "java"                       => "java",
        "js" | "cjs" | "mjs"         => "javascript",
        "json"                       => "json",
        "jsx"                        => "jsx",
        "kt" | "kts"                 => "kotlin",
        "md" | "markdown"            => "markdown",
        "php"                        => "php",
        "py"                         => "python",
        "rb"                         => "ruby",
        "rs"                         => "rust",
        "scss"                       => "scss",
        "sql"                        => "sql",
        "swift"                      => "swift",
        "toml"                       => "toml",
        "ts" | "cts" | "mts"         => "typescript",
        "tsx"                        => "tsx",
        "txt"                        => "",
        "yaml" | "yml"               => "yaml",
        _                            => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_get_language_alias() {
        assert_eq!(get_language_alias(&PathBuf::from("main.rs")), "rust");
        assert_eq!(get_language_alias(&PathBuf::from("index.js")), "javascript");
        assert_eq!(get_language_alias(&PathBuf::from("app.jsx")), "jsx");
        assert_eq!(get_language_alias(&PathBuf::from("component.tsx")), "tsx");
        assert_eq!(get_language_alias(&PathBuf::from("script.py")), "python");
        assert_eq!(get_language_alias(&PathBuf::from("Main.java")), "java");
        assert_eq!(get_language_alias(&PathBuf::from("file.c")), "c");
        assert_eq!(get_language_alias(&PathBuf::from("test.cpp")), "cpp");
        assert_eq!(get_language_alias(&PathBuf::from("Program.cs")), "csharp");
        assert_eq!(get_language_alias(&PathBuf::from("main.go")), "go");
        assert_eq!(get_language_alias(&PathBuf::from("utils.rb")), "ruby");
        assert_eq!(get_language_alias(&PathBuf::from("index.php")), "php");
        assert_eq!(get_language_alias(&PathBuf::from("my_file.swift")), "swift");
        assert_eq!(get_language_alias(&PathBuf::from("script.kt")), "kotlin");
        assert_eq!(get_language_alias(&PathBuf::from("run.sh")), "bash");
        assert_eq!(get_language_alias(&PathBuf::from("index.html")), "xml");
        assert_eq!(get_language_alias(&PathBuf::from("style.css")), "css");
        assert_eq!(get_language_alias(&PathBuf::from("variables.scss")), "scss");
        assert_eq!(get_language_alias(&PathBuf::from("data.json")), "json");
        assert_eq!(get_language_alias(&PathBuf::from("config.toml")), "toml");
        assert_eq!(get_language_alias(&PathBuf::from("config.yaml")), "yaml");
        assert_eq!(get_language_alias(&PathBuf::from("schema.sql")), "sql");
        assert_eq!(get_language_alias(&PathBuf::from("README.md")), "markdown");
        assert_eq!(get_language_alias(&PathBuf::from("log.txt")), "");
        assert_eq!(get_language_alias(&PathBuf::from("unrecognized.xyz")), "");
        assert_eq!(get_language_alias(&PathBuf::from("no_extension")), "");
    }
}