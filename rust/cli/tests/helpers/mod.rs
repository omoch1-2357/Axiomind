//! # テストヘルパー概要
//!
//! - `assertions` モジュール: `PokerAssertions` トレイトと `asserter()` を提供し、
//!   CLI 出力や JSONL の妥当性を共通化します。
//! - `cli_runner` モジュール: `CliRunner` が `Cargo` 生成バイナリ/ライブラリを呼び出し、
//!   標準出力・標準エラー・終了コード・実行時間を取得します。
//! - `temp_files` モジュール: `TempFileManager` が競合しない一時パスを作成し、Drop 時に掃除します。
//!
//! ```rust
//! use crate::helpers::{asserter, cli_runner::CliRunner, temp_files::TempFileManager};
//!
//! let cli = CliRunner::new().expect("cli runner");
//! let tmp = TempFileManager::new().expect("temp dir");
//! let out = tmp.create_file("hands.jsonl", "{}").expect("write");
//! let res = cli.run(&["sim", "--hands", "1", "--output", out.to_string_lossy().as_ref()]);
//! assert_eq!(res.exit_code, 0);
//! asserter().assert_jsonl_format(&std::fs::read_to_string(out).unwrap());
//! ```
//!
//! 上記スニペットを雛形として、新しい統合テストでも同じユーティリティを再利用してください。
pub mod error {
    use std::error::Error as StdError;
    use std::fmt;

    #[derive(Debug)]
    pub struct TestError {
        pub kind: TestErrorKind,
        pub message: String,
        pub source: Option<Box<dyn StdError + Send + Sync>>,
    }

    impl TestError {
        pub fn new(kind: TestErrorKind, message: impl Into<String>) -> Self {
            Self {
                kind,
                message: message.into(),
                source: None,
            }
        }

        pub fn with_source(
            kind: TestErrorKind,
            message: impl Into<String>,
            source: impl StdError + Send + Sync + 'static,
        ) -> Self {
            Self {
                kind,
                message: message.into(),
                source: Some(Box::new(source)),
            }
        }
    }

    impl fmt::Display for TestError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}: {}", self.kind, self.message)
        }
    }

    impl StdError for TestError {
        fn source(&self) -> Option<&(dyn StdError + 'static)> {
            self.source
                .as_deref()
                .map(|err| err as &(dyn StdError + 'static))
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum TestErrorKind {
        BinaryNotFound,
        ExecutionTimeout,
        UnexpectedExitCode,
        OutputMismatch,
        FileOperationFailed,
        AssertionFailed,
    }

    impl fmt::Display for TestErrorKind {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let name = match self {
                TestErrorKind::BinaryNotFound => "binary not found",
                TestErrorKind::ExecutionTimeout => "execution timeout",
                TestErrorKind::UnexpectedExitCode => "unexpected exit code",
                TestErrorKind::OutputMismatch => "output mismatch",
                TestErrorKind::FileOperationFailed => "file operation failed",
                TestErrorKind::AssertionFailed => "assertion failed",
            };
            f.write_str(name)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::{TestError, TestErrorKind};

        #[test]
        fn constructs_variants_and_display() {
            let kinds = [
                TestErrorKind::BinaryNotFound,
                TestErrorKind::ExecutionTimeout,
                TestErrorKind::UnexpectedExitCode,
                TestErrorKind::OutputMismatch,
                TestErrorKind::FileOperationFailed,
                TestErrorKind::AssertionFailed,
            ];

            for kind in kinds {
                let display = kind.to_string();
                assert!(!display.is_empty());
            }
        }

        #[test]
        fn with_source_keeps_kind_and_message() {
            let err = TestError::with_source(
                TestErrorKind::AssertionFailed,
                "context",
                std::io::Error::other("details"),
            );

            assert_eq!(err.kind, TestErrorKind::AssertionFailed);
            assert_eq!(err.message, "context");
            assert!(err.source.is_some());
        }
    }
}

pub use error::{TestError, TestErrorKind};
pub mod assertions;
pub mod cli_runner;
pub mod temp_files;
