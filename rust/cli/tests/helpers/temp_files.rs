use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use std::sync::atomic::{AtomicU64, Ordering};

use crate::helpers::{TestError, TestErrorKind};
use axiomind_engine::logger::HandRecord;
use serde_json;
use std::io::BufWriter;
static COUNTER: AtomicU64 = AtomicU64::new(0);

mod tempfile {
    use super::COUNTER;
    use std::env;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[derive(Debug)]
    pub struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        pub fn new() -> std::io::Result<Self> {
            Builder::new().tempdir()
        }

        pub fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[derive(Debug, Default)]
    pub struct Builder {
        prefix: Option<String>,
    }

    impl Builder {
        pub fn new() -> Self {
            Self { prefix: None }
        }

        pub fn prefix(mut self, value: &str) -> Self {
            self.prefix = Some(value.to_string());
            self
        }

        pub fn tempdir(self) -> std::io::Result<TempDir> {
            let ts = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis();
            let unique = COUNTER.fetch_add(1, super::Ordering::Relaxed);

            let mut dir = env::temp_dir();
            let prefix = self.prefix.unwrap_or_else(|| "axiomind-cli".to_string());
            dir.push(format!("{}-{}-{}-{}", prefix, process::id(), ts, unique));

            fs::create_dir_all(&dir)?;
            Ok(TempDir { path: dir })
        }
    }
}

use tempfile::Builder;
pub use tempfile::TempDir;

#[derive(Debug)]
#[allow(dead_code)]
pub struct TempFileManager {
    base_dir: TempDir,
}

impl TempFileManager {
    #[allow(dead_code)]
    pub fn new() -> Result<Self, TestError> {
        let base_dir = Builder::new()
            .prefix("axiomind-cli")
            .tempdir()
            .map_err(|err| {
                TestError::with_source(
                    TestErrorKind::FileOperationFailed,
                    "failed to create temporary directory",
                    err,
                )
            })?;

        Ok(Self { base_dir })
    }

    #[allow(dead_code)]
    pub fn create_directory(&self, name: &str) -> Result<PathBuf, TestError> {
        let path = self.base_dir.path().join(name);
        fs::create_dir_all(&path).map_err(|err| {
            TestError::with_source(
                TestErrorKind::FileOperationFailed,
                format!("failed to create directory '{}'", path.display()),
                err,
            )
        })?;
        Ok(path)
    }

    #[allow(dead_code)]
    pub fn create_file(&self, name: &str, content: &str) -> Result<PathBuf, TestError> {
        let path = self.base_dir.path().join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                TestError::with_source(
                    TestErrorKind::FileOperationFailed,
                    format!("failed to create parent directory '{}'", parent.display()),
                    err,
                )
            })?;
        }
        let mut file = File::create(&path).map_err(|err| {
            TestError::with_source(
                TestErrorKind::FileOperationFailed,
                format!("failed to create file '{}'", path.display()),
                err,
            )
        })?;
        file.write_all(content.as_bytes()).map_err(|err| {
            TestError::with_source(
                TestErrorKind::FileOperationFailed,
                format!("failed to write file '{}'", path.display()),
                err,
            )
        })?;
        Ok(path)
    }

    pub fn create_jsonl(&self, name: &str, records: &[HandRecord]) -> Result<PathBuf, TestError> {
        let path = self.base_dir.path().join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                TestError::with_source(
                    TestErrorKind::FileOperationFailed,
                    format!("failed to create parent directory '{}'", parent.display()),
                    err,
                )
            })?;
        }
        let file = File::create(&path).map_err(|err| {
            TestError::with_source(
                TestErrorKind::FileOperationFailed,
                format!("failed to create jsonl file '{}'", path.display()),
                err,
            )
        })?;
        let mut writer = BufWriter::new(file);
        for record in records {
            serde_json::to_writer(&mut writer, record).map_err(|err| {
                TestError::with_source(
                    TestErrorKind::FileOperationFailed,
                    format!("failed to serialize hand record for '{}'", path.display()),
                    err,
                )
            })?;
            writer.write_all(b"\n").map_err(|err| {
                TestError::with_source(
                    TestErrorKind::FileOperationFailed,
                    format!("failed to write newline for '{}'", path.display()),
                    err,
                )
            })?;
        }
        writer.flush().map_err(|err| {
            TestError::with_source(
                TestErrorKind::FileOperationFailed,
                format!("failed to flush jsonl writer '{}'", path.display()),
                err,
            )
        })?;
        Ok(path)
    }

    pub fn create_compressed(&self, name: &str, content: &str) -> Result<PathBuf, TestError> {
        let path = self.base_dir.path().join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                TestError::with_source(
                    TestErrorKind::FileOperationFailed,
                    format!("failed to create parent directory '{}'", parent.display()),
                    err,
                )
            })?;
        }
        let file = File::create(&path).map_err(|err| {
            TestError::with_source(
                TestErrorKind::FileOperationFailed,
                format!("failed to create compressed file '{}'", path.display()),
                err,
            )
        })?;
        let mut encoder = zstd::stream::write::Encoder::new(file, 0).map_err(|err| {
            TestError::with_source(
                TestErrorKind::FileOperationFailed,
                format!("failed to start zstd encoder for '{}'", path.display()),
                err,
            )
        })?;
        encoder.write_all(content.as_bytes()).map_err(|err| {
            TestError::with_source(
                TestErrorKind::FileOperationFailed,
                format!("failed to write compressed payload '{}'", path.display()),
                err,
            )
        })?;
        encoder.finish().map_err(|err| {
            TestError::with_source(
                TestErrorKind::FileOperationFailed,
                format!("failed to finish zstd encoder for '{}'", path.display()),
                err,
            )
        })?;
        Ok(path)
    }

    pub fn path(&self, name: &str) -> PathBuf {
        self.base_dir.path().join(name)
    }

    #[allow(dead_code)]
    pub fn root(&self) -> &Path {
        self.base_dir.path()
    }
}

impl Drop for TempFileManager {
    fn drop(&mut self) {
        // TempDir handles cleanup automatically.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axiomind_engine::logger::{ActionRecord, HandRecord, Street};
    use axiomind_engine::player::PlayerAction;
    use serde_json::from_str;
    use std::fs::File;
    use std::io::Read;

    fn sample_record(hand_id: &str) -> HandRecord {
        HandRecord {
            hand_id: hand_id.to_string(),
            seed: Some(42),
            actions: vec![ActionRecord {
                player_id: 0,
                street: Street::Preflop,
                action: PlayerAction::Fold,
            }],
            board: Vec::new(),
            result: Some("fold".to_string()),
            ts: None,
            meta: None,
            showdown: None,
        }
    }

    #[test]
    fn create_jsonl_writes_records_line_delimited() {
        let manager = TempFileManager::new().expect("create temp dir");
        let records = vec![sample_record("hand-1"), sample_record("hand-2")];

        let path = manager
            .create_jsonl("hands/test.jsonl", &records)
            .expect("create jsonl");
        let data = std::fs::read_to_string(&path).expect("read jsonl");
        let mut lines = data.lines();

        let first: HandRecord = from_str(lines.next().expect("first line")).expect("parse first");
        assert_eq!(first.hand_id, "hand-1");

        let second: HandRecord =
            from_str(lines.next().expect("second line")).expect("parse second");
        assert_eq!(second.hand_id, "hand-2");
        assert!(lines.next().is_none(), "no extra lines");
    }

    #[test]
    fn create_compressed_writes_zstd_content() {
        let manager = TempFileManager::new().expect("create temp dir");
        let expected = "compressed payload";

        let path = manager
            .create_compressed("logs/output.txt.zst", expected)
            .expect("create compressed");
        let file = File::open(&path).expect("open compressed");
        let mut decoder = zstd::stream::read::Decoder::new(file).expect("decoder");
        let mut actual = String::new();
        decoder.read_to_string(&mut actual).expect("decompress");

        assert_eq!(actual, expected);
    }

    #[test]
    fn path_joins_base_directory() {
        let manager = TempFileManager::new().expect("create temp dir");
        let nested = manager.path("nested/output.txt");

        assert!(nested.starts_with(manager.root()));
        assert_eq!(nested.file_name().unwrap().to_string_lossy(), "output.txt");
    }
}
