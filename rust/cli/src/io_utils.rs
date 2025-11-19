//! File I/O utilities for reading JSONL, text files, and ensuring directories.
//!
//! This module provides helper functions for file operations used across CLI commands:
//! - Reading from stdin (interactive input)
//! - Reading text files with automatic .zst decompression
//! - Ensuring parent directories exist before file writes
//!
//! ## Error Handling
//!
//! Functions return `Result` types with appropriate error messages. I/O errors
//! are converted to `String` for easy integration with command error handling.
//!
//! ## Compressed File Support
//!
//! The `read_text_auto` function automatically detects and decompresses .zst
//! (Zstandard) compressed files based on the file extension.

use std::io::BufRead;

/// Reads a line of input from a buffered reader, blocking until available.
///
/// This function is used for interactive commands that need user input.
/// It trims whitespace from the input and returns `None` on EOF or read errors.
///
/// # Arguments
///
/// * `stdin` - Buffered reader to read from (typically stdin)
///
/// # Returns
///
/// * `Some(String)` - Trimmed input line (may be empty after trimming)
/// * `None` - EOF or read error occurred
///
/// # Example
///
/// ```rust,no_run
/// use std::io::{self, BufRead};
/// # use axiomind_cli::io_utils::read_stdin_line;
///
/// let stdin = io::stdin();
/// let mut handle = stdin.lock();
/// if let Some(line) = read_stdin_line(&mut handle) {
///     println!("You entered: {}", line);
/// }
/// ```
pub fn read_stdin_line(stdin: &mut dyn BufRead) -> Option<String> {
    let mut line = String::new();
    match stdin.read_line(&mut line) {
        Ok(0) => None, // EOF
        Ok(_) => {
            let trimmed = line.trim();
            Some(trimmed.to_string())
        }
        Err(_) => None, // Read error
    }
}

/// Read text file with automatic .zst decompression detection.
///
/// This function reads a text file from the specified path. If the path ends
/// with ".zst", the file is automatically decompressed using Zstandard compression.
/// UTF-8 BOM (Byte Order Mark) is automatically stripped if present.
///
/// # Arguments
///
/// * `path` - File path to read (supports .zst compressed files)
///
/// # Returns
///
/// * `Ok(String)` - File contents as UTF-8 string
/// * `Err(String)` - I/O error, decompression error, or UTF-8 conversion error
///
/// # Example
///
/// ```rust,no_run
/// # use axiomind_cli::io_utils::read_text_auto;
///
/// // Read plain text file
/// let content = read_text_auto("data.txt").unwrap();
///
/// // Read compressed file (automatic decompression)
/// let compressed = read_text_auto("data.jsonl.zst").unwrap();
/// ```
pub fn read_text_auto(path: &str) -> Result<String, String> {
    let mut content = if path.ends_with(".zst") {
        // Read entire compressed file then decompress; more portable across platforms
        let comp = std::fs::read(path).map_err(|e| e.to_string())?;
        // Use a conservative initial capacity; zstd will grow as needed
        let dec = zstd::bulk::decompress(&comp, 8 * 1024 * 1024).map_err(|e| e.to_string())?;
        String::from_utf8(dec).map_err(|e| e.to_string())?
    } else {
        std::fs::read_to_string(path).map_err(|e| e.to_string())?
    };
    strip_utf8_bom(&mut content);
    Ok(content)
}

/// Ensure parent directory exists for given path, creating if needed.
///
/// This function checks if the parent directory of the given path exists,
/// and creates it (including any missing intermediate directories) if needed.
/// This is useful before writing files to ensure the destination directory exists.
///
/// # Arguments
///
/// * `path` - File path whose parent directory should exist
///
/// # Returns
///
/// * `Ok(())` - Parent directory exists or was created successfully
/// * `Err(String)` - Failed to create directory with error message
///
/// # Example
///
/// ```rust,no_run
/// use std::path::Path;
/// # use axiomind_cli::io_utils::ensure_parent_dir;
///
/// let path = Path::new("output/data/file.jsonl");
/// ensure_parent_dir(path).unwrap();
/// // Now "output/data/" directory exists
/// ```
pub fn ensure_parent_dir(path: &std::path::Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory {}: {}", parent.display(), e))?;
        }
    }
    Ok(())
}

/// Strip UTF-8 BOM (Byte Order Mark) from the beginning of a string if present.
///
/// UTF-8 BOM is the character U+FEFF at the start of a file. Some text editors
/// add this marker, but it can cause issues when parsing JSON or other formats.
///
/// # Arguments
///
/// * `s` - Mutable string reference to strip BOM from
fn strip_utf8_bom(s: &mut String) {
    const UTF8_BOM: &str = "\u{feff}";
    if s.starts_with(UTF8_BOM) {
        s.drain(..UTF8_BOM.len());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_read_stdin_line_valid_input() {
        let input = b"hello world\n";
        let mut cursor = Cursor::new(input);
        let result = read_stdin_line(&mut cursor);
        assert_eq!(result, Some("hello world".to_string()));
    }

    #[test]
    fn test_read_stdin_line_with_whitespace() {
        let input = b"  spaces  \n";
        let mut cursor = Cursor::new(input);
        let result = read_stdin_line(&mut cursor);
        assert_eq!(result, Some("spaces".to_string()));
    }

    #[test]
    fn test_read_stdin_line_empty_after_trim() {
        let input = b"   \n";
        let mut cursor = Cursor::new(input);
        let result = read_stdin_line(&mut cursor);
        assert_eq!(result, Some("".to_string()));
    }

    #[test]
    fn test_read_stdin_line_eof() {
        let input = b"";
        let mut cursor = Cursor::new(input);
        let result = read_stdin_line(&mut cursor);
        assert_eq!(result, None);
    }

    #[test]
    fn test_read_stdin_line_bet_with_amount() {
        let input = b"bet 100\n";
        let mut cursor = Cursor::new(input);
        let result = read_stdin_line(&mut cursor);
        assert_eq!(result, Some("bet 100".to_string()));
    }

    #[test]
    fn test_strip_utf8_bom() {
        let mut s = "\u{feff}hello".to_string();
        strip_utf8_bom(&mut s);
        assert_eq!(s, "hello");
    }

    #[test]
    fn test_strip_utf8_bom_no_bom() {
        let mut s = "hello".to_string();
        strip_utf8_bom(&mut s);
        assert_eq!(s, "hello");
    }

    #[test]
    fn test_ensure_parent_dir_creates_directory() {
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let nested_path = temp_dir.path().join("subdir").join("file.txt");

        let result = ensure_parent_dir(&nested_path);
        assert!(result.is_ok());
        assert!(temp_dir.path().join("subdir").exists());
    }

    #[test]
    fn test_ensure_parent_dir_no_parent() {
        use std::path::Path;

        // Path with no parent (e.g., root or relative file)
        let path = Path::new("file.txt");
        let result = ensure_parent_dir(path);
        assert!(result.is_ok());
    }
}
