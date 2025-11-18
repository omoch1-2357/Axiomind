use axiomind_cli::run;
use std::fs;
use std::path::PathBuf;

fn out_path(name: &str) -> PathBuf {
    let mut p = PathBuf::from("target");
    p.push(format!("{}_{}.jsonl", name, std::process::id()));
    if let Some(parent) = p.parent() {
        let _ = fs::create_dir_all(parent);
    }
    p
}

#[test]
fn sim_runs_n_hands_and_writes_file() {
    let path = out_path("sim");
    // Remove any existing file to avoid data from previous runs
    let _ = fs::remove_file(&path);
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();
    let code = run(
        [
            "axiomind",
            "sim",
            "--hands",
            "5",
            "--seed",
            "1",
            "--output",
            path.to_string_lossy().as_ref(),
        ],
        &mut out,
        &mut err,
    );
    assert_eq!(code, 0);
    let stdout = String::from_utf8_lossy(&out);
    assert!(stdout.contains("Simulated: 5 hands"));
    let contents = fs::read_to_string(&path).unwrap();
    let lines = contents.lines().filter(|l| !l.trim().is_empty()).count();
    assert_eq!(lines, 5);
}
