use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use std::sync::Mutex;

use crate::helpers::temp_files::TempDir;
use crate::helpers::{TestError, TestErrorKind};

#[allow(dead_code)]
pub static DOCTOR_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug)]
pub struct CliRunner {
    binary_path: PathBuf,
    temp_dir: TempDir,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct CliResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration: Duration,
}

struct EnvGuard {
    restores: Vec<(String, Option<String>)>,
}

impl EnvGuard {
    fn apply(pairs: &[(&str, &str)]) -> Self {
        let mut restores = Vec::new();
        for (key, value) in pairs {
            let key_owned = key.to_string();
            let previous = std::env::var(key).ok();
            std::env::set_var(key, value);
            restores.push((key_owned, previous));
        }
        EnvGuard { restores }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, previous) in self.restores.iter().rev() {
            match previous {
                Some(val) => std::env::set_var(key, val),
                None => std::env::remove_var(key),
            }
        }
    }
}

impl CliRunner {
    pub fn new() -> Result<Self, TestError> {
        let temp_dir = TempDir::new().map_err(|err| {
            TestError::with_source(
                TestErrorKind::FileOperationFailed,
                "failed to create temporary CLI workspace",
                err,
            )
        })?;

        let binary_path = Self::resolve_binary_path()?;

        Ok(Self {
            binary_path,
            temp_dir,
        })
    }

    pub fn run(&self, args: &[&str]) -> CliResult {
        self.run_inner(args, &[], None, None)
    }

    #[allow(dead_code)]
    pub fn run_with_env(&self, args: &[&str], env: &[(&str, &str)]) -> CliResult {
        self.run_inner(args, env, None, None)
    }

    pub fn run_with_input(&self, args: &[&str], input: &str) -> CliResult {
        self.run_inner(args, &[], Some(input), None)
    }

    pub fn run_with_timeout(&self, args: &[&str], timeout: Duration) -> CliResult {
        self.run_inner(args, &[], None, Some(timeout))
    }

    fn run_inner(
        &self,
        args: &[&str],
        env: &[(&str, &str)],
        input: Option<&str>,
        timeout: Option<Duration>,
    ) -> CliResult {
        if self.binary_path.is_file() {
            self.run_via_binary(args, env, input, timeout)
        } else {
            self.run_via_library(args, env)
        }
    }

    fn run_via_binary(
        &self,
        args: &[&str],
        env: &[(&str, &str)],
        input: Option<&str>,
        timeout: Option<Duration>,
    ) -> CliResult {
        let mut cmd = Command::new(&self.binary_path);
        cmd.args(args)
            .current_dir(self.temp_dir.path())
            .stdin(if input.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        for (key, value) in env.iter() {
            cmd.env(key, value);
        }

        let start = Instant::now();
        let mut child = cmd.spawn().expect("failed to spawn CLI binary");

        if let Some(payload) = input {
            use std::io::Write as _;
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(payload.as_bytes());
            }
        }

        let (status, stdout, stderr) = if let Some(limit) = timeout {
            loop {
                if let Some(_exit) = child.try_wait().expect("failed to poll child") {
                    let output = child.wait_with_output().expect("failed to read output");
                    break (output.status, output.stdout, output.stderr);
                }

                if start.elapsed() >= limit {
                    let _ = child.kill();
                    let output = child
                        .wait_with_output()
                        .expect("failed to collect output after kill");
                    break (output.status, output.stdout, output.stderr);
                }

                std::thread::sleep(Duration::from_millis(10));
            }
        } else {
            let output = child.wait_with_output().expect("failed to read output");
            (output.status, output.stdout, output.stderr)
        };

        let duration = start.elapsed();
        CliResult {
            exit_code: status.code().unwrap_or(1),
            stdout: String::from_utf8_lossy(&stdout).to_string(),
            stderr: String::from_utf8_lossy(&stderr).to_string(),
            duration,
        }
    }

    fn run_via_library(&self, args: &[&str], env: &[(&str, &str)]) -> CliResult {
        let _guard = EnvGuard::apply(env);
        let mut out = Vec::new();
        let mut err = Vec::new();
        let start = Instant::now();
        let argv: Vec<String> = std::iter::once("axiomind".to_string())
            .chain(args.iter().map(|s| s.to_string()))
            .collect();
        let code = axiomind_cli::run(argv, &mut out, &mut err);
        let duration = start.elapsed();
        CliResult {
            exit_code: code,
            stdout: String::from_utf8_lossy(&out).to_string(),
            stderr: String::from_utf8_lossy(&err).to_string(),
            duration,
        }
    }

    fn resolve_binary_path() -> Result<PathBuf, TestError> {
        if let Ok(explicit) = std::env::var("CARGO_BIN_EXE_axiomind") {
            let candidate = PathBuf::from(&explicit);
            if candidate.is_file() {
                return Ok(candidate);
            }

            return Err(TestError::new(
                TestErrorKind::BinaryNotFound,
                format!(
                    "CARGO_BIN_EXE_axiomind points to '{}', but the file does not exist",
                    explicit
                ),
            ));
        }

        let executable = if cfg!(windows) {
            "axiomind.exe"
        } else {
            "axiomind"
        };
        let mut search_roots = Vec::new();

        if let Ok(custom_target) = std::env::var("CARGO_TARGET_DIR") {
            search_roots.push(PathBuf::from(custom_target));
        }

        search_roots.push(PathBuf::from("target"));

        for root in &search_roots {
            for profile in ["debug", "release"] {
                let candidate = root.join(profile).join(executable);
                if candidate.is_file() {
                    return Ok(candidate);
                }
            }
        }

        let mut fallback = search_roots
            .into_iter()
            .next()
            .unwrap_or_else(|| PathBuf::from("target"));
        fallback = fallback.join("debug").join(executable);
        Ok(fallback)
    }
}

#[cfg(test)]
mod tests {
    use super::CliRunner;
    use std::time::Duration;

    #[test]
    fn run_with_input_accepts_empty_payload() {
        let cli = CliRunner::new().expect("CliRunner init");
        let result = cli.run_with_input(&["--help"], "");

        assert_eq!(result.exit_code, 0);
        assert!(result.stdout.contains("Usage"));
    }

    #[test]
    fn run_with_timeout_finishes_quickly() {
        let cli = CliRunner::new().expect("CliRunner init");
        let result = cli.run_with_timeout(&["--version"], Duration::from_secs(2));

        assert_eq!(result.exit_code, 0);
        assert!(result.duration <= Duration::from_secs(2));
    }

    #[test]
    fn run_executes_help_command() {
        let cli = CliRunner::new().expect("CliRunner init");
        let result = cli.run(&["--help"]);

        assert_eq!(result.exit_code, 0);
        assert!(result.stdout.contains("Usage"));
    }
}

// No extra platform helpers needed after refactor above
