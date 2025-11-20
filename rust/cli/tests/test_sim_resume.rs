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
fn sim_gracefully_saves_partial_and_resumes() {
    let path = out_path("sim_resume");
    // Remove any existing file to avoid data from previous runs
    let _ = fs::remove_file(&path);

    // force interruption after 3
    unsafe {
        std::env::set_var("axiomind_SIM_BREAK_AFTER", "3");
    }
    unsafe {
        std::env::set_var("axiomind_SIM_FAST", "1");
    }
    let mut out1: Vec<u8> = Vec::new();
    let mut err1: Vec<u8> = Vec::new();
    let code1 = run(
        [
            "axiomind",
            "sim",
            "--hands",
            "5",
            "--seed",
            "3",
            "--output",
            path.to_string_lossy().as_ref(),
        ],
        &mut out1,
        &mut err1,
    );
    assert_ne!(code1, 0);
    let s1 = String::from_utf8_lossy(&out1);
    assert!(s1.contains("Interrupted: saved 3/5"));
    let lines = fs::read_to_string(&path).unwrap().lines().count();
    assert_eq!(lines, 3);

    // resume to complete 5
    unsafe {
        std::env::remove_var("axiomind_SIM_BREAK_AFTER");
    }
    unsafe {
        std::env::set_var("axiomind_SIM_FAST", "1");
    }
    let mut out2: Vec<u8> = Vec::new();
    let mut err2: Vec<u8> = Vec::new();
    let code2 = run(
        [
            "axiomind",
            "sim",
            "--hands",
            "5",
            "--seed",
            "3",
            "--resume",
            path.to_string_lossy().as_ref(),
        ],
        &mut out2,
        &mut err2,
    );
    assert_eq!(code2, 0);
    let s2 = String::from_utf8_lossy(&out2);
    assert!(s2.contains("Resumed from 3"));
    assert!(s2.contains("Simulated: 5 hands"));
    let lines2 = fs::read_to_string(&path).unwrap().lines().count();
    assert_eq!(lines2, 5);
}
