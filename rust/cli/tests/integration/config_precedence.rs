use crate::helpers::cli_runner::CliRunner;
use crate::helpers::temp_files::TempFileManager;

use serde_json::Value;
use std::sync::{Mutex, OnceLock};

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[test]
fn i1_cfg_shows_defaults_for_adaptive_and_ai_version() {
    let _guard = env_lock().lock().unwrap();
    unsafe {
        std::env::remove_var("AXIOMIND_CONFIG");
    }
    unsafe {
        std::env::remove_var("AXIOMIND_ADAPTIVE");
    }
    unsafe {
        std::env::remove_var("AXIOMIND_AI_VERSION");
    }
    unsafe {
        std::env::remove_var("AXIOMIND_SEED");
    }
    unsafe {
        std::env::remove_var("AXIOMIND_LEVEL");
    }

    let cli = CliRunner::new().expect("init");
    let res = cli.run(&["cfg"]);
    assert_eq!(res.exit_code, 0);
    let json: Value = serde_json::from_str(&res.stdout).unwrap();

    let adaptive = &json["adaptive"];
    assert_eq!(adaptive["value"].as_bool(), Some(true));
    assert_eq!(adaptive["source"].as_str(), Some("default"));

    let ai_version = &json["ai_version"];
    assert_eq!(ai_version["value"].as_str(), Some("latest"));
    assert_eq!(ai_version["source"].as_str(), Some("default"));
}

#[test]
fn i2_precedence_cli_over_env_over_file_for_seed_and_ai() {
    let _guard = env_lock().lock().unwrap();
    unsafe {
        std::env::remove_var("AXIOMIND_CONFIG");
    }
    unsafe {
        std::env::remove_var("AXIOMIND_SEED");
    }
    unsafe {
        std::env::remove_var("AXIOMIND_AI_VERSION");
    }
    unsafe {
        std::env::remove_var("AXIOMIND_ADAPTIVE");
    }

    let tfm = TempFileManager::new().unwrap();
    let cfg_path = tfm
        .create_file(
            "axiomind.toml",
            "seed = 456\nai_version = \"v1\"\nadaptive = false\n",
        )
        .unwrap();
    unsafe {
        std::env::set_var("AXIOMIND_CONFIG", &cfg_path);
    }

    let cli = CliRunner::new().expect("init");
    let cfg1 = cli.run(&["cfg"]);
    assert_eq!(cfg1.exit_code, 0);
    let json1: Value = serde_json::from_str(&cfg1.stdout).unwrap();
    assert_eq!(json1["seed"]["value"].as_u64(), Some(456));
    assert_eq!(json1["seed"]["source"].as_str(), Some("file"));
    assert_eq!(json1["ai_version"]["value"].as_str(), Some("v1"));
    assert_eq!(json1["ai_version"]["source"].as_str(), Some("file"));
    assert_eq!(json1["adaptive"]["value"].as_bool(), Some(false));
    assert_eq!(json1["adaptive"]["source"].as_str(), Some("file"));

    unsafe {
        std::env::set_var("AXIOMIND_SEED", "123");
    }
    unsafe {
        std::env::set_var("AXIOMIND_AI_VERSION", "v2");
    }
    unsafe {
        std::env::set_var("AXIOMIND_ADAPTIVE", "on");
    }
    let cfg2 = cli.run(&["cfg"]);
    assert_eq!(cfg2.exit_code, 0);
    let json2: Value = serde_json::from_str(&cfg2.stdout).unwrap();
    assert_eq!(json2["seed"]["value"].as_u64(), Some(123));
    assert_eq!(json2["seed"]["source"].as_str(), Some("env"));
    assert_eq!(json2["ai_version"]["value"].as_str(), Some("v2"));
    assert_eq!(json2["ai_version"]["source"].as_str(), Some("env"));
    assert_eq!(json2["adaptive"]["value"].as_bool(), Some(true));
    assert_eq!(json2["adaptive"]["source"].as_str(), Some("env"));

    let r1 = cli.run(&["rng", "--seed", "42"]);
    let r2 = cli.run(&["rng", "--seed", "42"]);
    assert_eq!(
        r1.stdout, r2.stdout,
        "same seed should produce identical RNG output"
    );

    unsafe {
        std::env::remove_var("AXIOMIND_CONFIG");
    }
    unsafe {
        std::env::remove_var("AXIOMIND_SEED");
    }
    unsafe {
        std::env::remove_var("AXIOMIND_AI_VERSION");
    }
    unsafe {
        std::env::remove_var("AXIOMIND_ADAPTIVE");
    }
}

#[test]
fn i3_seed_default_is_non_deterministic() {
    let cli = CliRunner::new().expect("init");
    let a = cli.run(&["rng"]);
    let b = cli.run(&["rng"]);
    assert_ne!(
        a.stdout, b.stdout,
        "rng without --seed should be non-deterministic"
    );
}
