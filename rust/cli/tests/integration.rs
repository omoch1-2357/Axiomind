// Deny specific lints instead of all warnings to avoid breakage on new Rust releases
#![deny(missing_debug_implementations, unused_must_use)]
#![warn(clippy::all)]
// Note: rust_2018_idioms and unreachable_pub are too strict for test code with helper modules
mod helpers;
mod integration {
    // groups files under tests/integration/
    mod assertions_basic; // rust/cli/tests/integration/assertions_basic.rs (2.3)
    mod cli_basic; // rust/cli/tests/integration/cli_basic.rs
    mod config_precedence; // rust/cli/tests/integration/config_precedence.rs (3.2)
    mod evaluation_basic; // rust/cli/tests/integration/evaluation_basic.rs (6.2)
    mod file_corruption_recovery; // rust/cli/tests/integration/file_corruption_recovery.rs (5.2)
    mod file_dir_processing; // rust/cli/tests/integration/file_dir_processing.rs (5.3)
    mod file_io_basic; // rust/cli/tests/integration/file_io_basic.rs (5.1)
    mod game_logic;
    mod helpers_temp_files; // rust/cli/tests/integration/helpers_temp_files.rs (2.2 Red)
    mod simulation_basic; // rust/cli/tests/integration/simulation_basic.rs (6.1)
                          // rust/cli/tests/integration/game_logic.rs (B 4.2)

    mod performance_stress {
        use crate::helpers::cli_runner::CliRunner;
        use crate::helpers::temp_files::TempFileManager;
        use serde_json::json;
        use std::fs;
        use std::time::Duration;

        fn generate_records(count: usize) -> String {
            let mut buf = String::new();
            for idx in 0..count {
                let record = json!({
                    "hand_id": format!("20250102-{idx:06}", idx = idx + 1),
                    "seed": idx as u64,
                    "actions": [],
                    "board": [],
                    "result": null,
                    "ts": null,
                    "meta": null,
                    "showdown": null
                });
                buf.push_str(&record.to_string());
                buf.push('\n');
            }
            buf
        }

        // Performance test: may be environment-dependent
        // Set AXM_SIM_FAST=1 to enable faster simulation for testing
        #[test]
        #[ignore] // Ignore by default due to environment dependency
        fn p1_sim_large_run_under_budget() {
            let cli = CliRunner::new().expect("cli runner");
            let tfm = TempFileManager::new().expect("temp dir");
            let sim_dir = tfm.create_directory("sim").expect("create sim dir");
            let out_path = sim_dir.join("perf.jsonl");
            let out_path_owned = out_path.to_string_lossy().into_owned();
            let args = [
                "sim",
                "--hands",
                "2000",
                "--seed",
                "42",
                "--output",
                out_path_owned.as_str(),
            ];
            let env = [("AXM_SIM_FAST", "1")];
            let res = cli.run_with_env(&args, &env);

            assert_eq!(
                res.exit_code, 0,
                "sim should succeed: stderr={}",
                res.stderr
            );
            let budget = Duration::from_millis(2500);
            assert!(
                res.duration < budget,
                "simulated 2000 hands too slow: {:?} >= {:?}",
                res.duration,
                budget
            );

            let contents = fs::read_to_string(&out_path).expect("read simulation output");
            let lines = contents.lines().filter(|l| !l.trim().is_empty()).count();
            assert_eq!(lines, 2000, "expected 2000 hand records");
        }

        #[test]
        fn p2_dataset_streaming_processes_large_file() {
            let cli = CliRunner::new().expect("cli runner");
            let tfm = TempFileManager::new().expect("temp dir");
            let dataset = generate_records(12_000);
            let input = tfm
                .create_file("large.jsonl", &dataset)
                .expect("write large dataset");
            let outdir = tfm.create_directory("splits").expect("create splits dir");
            let input_owned = input.to_string_lossy().into_owned();
            let outdir_owned = outdir.to_string_lossy().into_owned();
            let args = [
                "dataset",
                "--input",
                input_owned.as_str(),
                "--outdir",
                outdir_owned.as_str(),
                "--train",
                "0.5",
                "--val",
                "0.25",
                "--test",
                "0.25",
                "--seed",
                "99",
            ];
            let env = [
                ("AXM_DATASET_STREAM_THRESHOLD", "1000"),
                ("AXM_DATASET_STREAM_TRACE", "1"),
            ];
            let res = cli.run_with_env(&args, &env);

            assert_eq!(
                res.exit_code, 0,
                "dataset should succeed in streaming mode: stderr={}",
                res.stderr
            );
            assert!(
                res.stderr.contains("Streaming dataset input"),
                "expected streaming trace message, stderr={}",
                res.stderr
            );
            let budget = Duration::from_millis(3000);
            assert!(
                res.duration < budget,
                "dataset streaming too slow: {:?} >= {:?}",
                res.duration,
                budget
            );

            let train = fs::read_to_string(outdir.join("train.jsonl")).expect("train split");
            let val = fs::read_to_string(outdir.join("val.jsonl")).expect("val split");
            let test = fs::read_to_string(outdir.join("test.jsonl")).expect("test split");
            let total = train.lines().filter(|l| !l.trim().is_empty()).count()
                + val.lines().filter(|l| !l.trim().is_empty()).count()
                + test.lines().filter(|l| !l.trim().is_empty()).count();
            assert_eq!(total, 12_000, "expected all records to be assigned");
        }

        #[test]
        fn p3_sim_respects_timeout_limit() {
            let cli = CliRunner::new().expect("cli runner");
            let tfm = TempFileManager::new().expect("temp dir");
            let slow_dir = tfm.create_directory("slow").expect("create slow dir");
            let out_path = slow_dir.join("slow.jsonl");
            let out_path_owned = out_path.to_string_lossy().into_owned();
            let timeout = Duration::from_millis(250);

            std::env::set_var("AXM_SIM_FAST", "1");
            std::env::set_var("AXM_SIM_SLEEP_MICROS", "2000");
            let res = cli.run_with_timeout(
                &[
                    "sim",
                    "--hands",
                    "400",
                    "--seed",
                    "7",
                    "--output",
                    out_path_owned.as_str(),
                ],
                timeout,
            );
            std::env::remove_var("AXM_SIM_FAST");
            std::env::remove_var("AXM_SIM_SLEEP_MICROS");

            assert!(
                res.duration >= timeout,
                "expected duration >= timeout, got {:?} < {:?}",
                res.duration,
                timeout
            );

            if let Ok(contents) = fs::read_to_string(&out_path) {
                assert!(
                    !contents.trim().is_empty(),
                    "expected partial simulation output before timeout"
                );
            }
        }
    } // rust/cli/tests/integration/performance_stress.rs (14.2)
    mod data_format {
        use crate::helpers::cli_runner::CliRunner;
        use crate::helpers::temp_files::TempFileManager;

        #[test]
        fn k1_dataset_rejects_schema_mismatch() {
            let tfm = TempFileManager::new().expect("temp dir");
            let input = tfm
                .create_file(
                    "invalid.jsonl",
                    "{\"hand_id\":\"20250102-000001\",\"seed\":1,\"actions\":\"oops\",\"board\":[],\"result\":null,\"ts\":null,\"meta\":null}\n",
                )
                .expect("write invalid jsonl");
            let outdir = tfm.create_directory("out").expect("create output dir");
            let cli = CliRunner::new().expect("cli runner");
            let input_str = input.to_string_lossy().to_string();
            let outdir_str = outdir.to_string_lossy().to_string();
            let args = [
                "dataset",
                "--input",
                input_str.as_str(),
                "--outdir",
                outdir_str.as_str(),
            ];
            let res = cli.run(&args);
            assert_ne!(res.exit_code, 0, "dataset should fail for schema mismatch");
            assert!(
                res.stderr.contains("Invalid record"),
                "expected schema error, stderr={}",
                res.stderr
            );
        }

        #[test]
        fn k2_export_reports_lock_conflict() {
            use rusqlite::Connection;

            let tfm = TempFileManager::new().expect("temp dir");
            let input = tfm
                .create_file(
                    "valid.jsonl",
                    "{\"hand_id\":\"20250102-000001\",\"seed\":1,\"actions\":[],\"board\":[],\"result\":null,\"ts\":null,\"meta\":null}\n",
                )
                .expect("write input jsonl");
            let sqlite_dir = tfm.create_directory("sqlite").expect("create sqlite dir");
            let db_path = sqlite_dir.join("locked.sqlite");

            let conn = Connection::open(&db_path).expect("open sqlite");
            conn.execute("BEGIN IMMEDIATE", []).expect("lock sqlite");

            let cli = CliRunner::new().expect("cli runner");
            let res = cli.run(&[
                "export",
                "--input",
                input.to_string_lossy().as_ref(),
                "--format",
                "sqlite",
                "--output",
                db_path.to_string_lossy().as_ref(),
            ]);

            assert_eq!(
                res.exit_code, 2,
                "export should fail while database is locked"
            );
            assert!(
                res.stderr.contains("SQLite busy"),
                "expected lock retry message, stderr={}",
                res.stderr
            );

            let _ = conn.execute("ROLLBACK", []);
        }

        #[test]
        fn k3_dataset_streams_large_input() {
            let tfm = TempFileManager::new().expect("temp dir");
            let mut content = String::new();
            for i in 0..32 {
                content.push_str(&format!(
                    "{{\"hand_id\":\"20250102-{idx:06}\",\"seed\":1,\"actions\":[],\"board\":[],\"result\":null,\"ts\":null,\"meta\":null}}\n",
                    idx = i + 1
                ));
            }
            let input = tfm
                .create_file("bulk.jsonl", &content)
                .expect("write bulk input");
            let outdir = tfm.create_directory("dataset").expect("create dataset dir");
            let cli = CliRunner::new().expect("cli runner");
            let res = cli.run_with_env(
                &[
                    "dataset",
                    "--input",
                    input.to_string_lossy().as_ref(),
                    "--outdir",
                    outdir.to_string_lossy().as_ref(),
                ],
                &[
                    ("AXM_DATASET_STREAM_THRESHOLD", "5"),
                    ("AXM_DATASET_STREAM_TRACE", "1"),
                ],
            );

            assert_eq!(res.exit_code, 0, "dataset should succeed in streaming mode");
            assert!(
                res.stderr.contains("Streaming dataset input"),
                "expected streaming trace message, stderr={}",
                res.stderr
            );
        }
    }
}
