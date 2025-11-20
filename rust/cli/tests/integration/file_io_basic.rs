use crate::helpers::cli_runner::CliRunner;
use crate::helpers::temp_files::TempFileManager;

// C-series 5.1: input validation and error handling (Red)

#[test]
fn c1_replay_requires_input_arg() {
    let cli = CliRunner::new().expect("CliRunner init");
    let res = cli.run(&["replay"]); // missing --input
    assert_ne!(res.exit_code, 0);
    let err = res.stderr.to_lowercase();
    assert!(
        err.contains("required") || err.contains("usage"),
        "stderr should indicate missing required arg: {}",
        res.stderr
    );
}

#[test]
fn c2_replay_speed_validation() {
    let tfm = TempFileManager::new().expect("tfm");
    let path = tfm.create_file("in.jsonl", "").expect("file");
    let cli = CliRunner::new().expect("CliRunner init");
    let res = cli.run(&["replay", "--input", &path.to_string_lossy(), "--speed", "0"]);
    assert_ne!(res.exit_code, 0);
    assert!(
        res.stderr.to_lowercase().contains("speed"),
        "stderr should mention speed violation: {}",
        res.stderr
    );
}

#[test]
fn c3_play_vs_human_accepts_piped_stdin() {
    let cli = CliRunner::new().expect("CliRunner init");
    use std::ffi::OsString;
    struct EnvVarGuard {
        prev: Option<OsString>,
    }
    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            unsafe {
                match self.prev.take() {
                    Some(prev) => std::env::set_var("axiomind_NON_TTY", prev),
                    None => std::env::remove_var("axiomind_NON_TTY"),
                }
            }
        }
    }
    let _env_guard = EnvVarGuard {
        prev: std::env::var_os("axiomind_NON_TTY"),
    };
    unsafe {
        std::env::set_var("axiomind_NON_TTY", "1");
    }
    // Provide input via pipe to avoid hanging
    let res = cli.run_with_input(&["play", "--vs", "human", "--hands", "1"], "q\n");
    // Should complete successfully with piped stdin
    assert_eq!(
        res.exit_code, 0,
        "Expected success with piped stdin, stderr: {}",
        res.stderr
    );
    assert!(
        res.stdout.contains("completed"),
        "Expected successful completion, got: {}",
        res.stdout
    );
}
