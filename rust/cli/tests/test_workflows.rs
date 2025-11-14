use axm_cli::run;
use std::fs;
use std::path::PathBuf;

use std::env;

fn p(name: &str, ext: &str) -> PathBuf {
    let mut pb = PathBuf::from("target");
    pb.push(format!("{}_{}.{}", name, std::process::id(), ext));
    let _ = fs::create_dir_all(pb.parent().unwrap());
    pb
}

#[test]
fn e2e_sim_stats_replay_export_verify() {
    // 1) simulate
    let out_jsonl = p("wf_sim", "jsonl");
    let _ = fs::remove_file(&out_jsonl);
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = run(
        [
            "axm",
            "sim",
            "--hands",
            "3",
            "--seed",
            "4",
            "--output",
            out_jsonl.to_string_lossy().as_ref(),
        ],
        &mut out,
        &mut err,
    );
    assert_eq!(
        code,
        0,
        "sim should exit 0, stderr={}",
        String::from_utf8_lossy(&err)
    );
    let contents = fs::read_to_string(&out_jsonl).unwrap();
    assert_eq!(contents.lines().filter(|l| !l.trim().is_empty()).count(), 3);

    // 2) stats
    out.clear();
    err.clear();
    let code = run(
        [
            "axm",
            "stats",
            "--input",
            out_jsonl.to_string_lossy().as_ref(),
        ],
        &mut out,
        &mut err,
    );
    assert_eq!(code, 0);
    let s = String::from_utf8_lossy(&out);
    assert!(s.contains("\"hands\": 3"));

    // 3) replay
    out.clear();
    err.clear();
    let code = run(
        [
            "axm",
            "replay",
            "--input",
            out_jsonl.to_string_lossy().as_ref(),
        ],
        &mut out,
        &mut err,
    );
    assert_eq!(code, 0);
    let s = String::from_utf8_lossy(&out);
    assert!(s.contains("Counted: 3 hands in file"));

    // 4) export json
    let out_json = p("wf_exp", "json");
    let _ = fs::remove_file(&out_json);
    out.clear();
    err.clear();
    let code = run(
        [
            "axm",
            "export",
            "--input",
            out_jsonl.to_string_lossy().as_ref(),
            "--format",
            "json",
            "--output",
            out_json.to_string_lossy().as_ref(),
        ],
        &mut out,
        &mut err,
    );
    assert_eq!(code, 0);
    let arr: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&out_json).unwrap()).unwrap();
    assert_eq!(arr.as_array().unwrap().len(), 3);

    // 5) verify OK for completed boards
    out.clear();
    err.clear();
    let code = run(
        [
            "axm",
            "verify",
            "--input",
            out_jsonl.to_string_lossy().as_ref(),
        ],
        &mut out,
        &mut err,
    );
    assert_eq!(
        code,
        0,
        "verify should be OK: stderr={} out={}",
        String::from_utf8_lossy(&err),
        String::from_utf8_lossy(&out)
    );
}

#[test]
fn e2e_dataset_stream_normalizes_crlf() {
    // simulate a small dataset
    let out_jsonl = p("wf_stream", "jsonl");
    let _ = fs::remove_file(&out_jsonl);
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = run(
        [
            "axm",
            "sim",
            "--hands",
            "5",
            "--seed",
            "99",
            "--output",
            out_jsonl.to_string_lossy().as_ref(),
        ],
        &mut out,
        &mut err,
    );
    assert_eq!(
        code,
        0,
        "sim should exit 0, stderr={} out={}",
        String::from_utf8_lossy(&err),
        String::from_utf8_lossy(&out),
    );

    // rewrite file with CRLF endings to emulate Windows hand histories
    let lf_contents = fs::read_to_string(&out_jsonl).unwrap();
    let crlf_contents = lf_contents.replace('\n', "\r\n");
    fs::write(&out_jsonl, crlf_contents).unwrap();

    // force streaming splitter path to exercise BufRead processing
    let var_name = "AXM_DATASET_STREAM_THRESHOLD";
    let prev_threshold = env::var_os(var_name);
    env::set_var(var_name, "1");
    let out_dir = p("wf_dataset", "dir");
    let _ = fs::remove_dir_all(&out_dir);
    let out_dir_owned = out_dir.to_string_lossy().into_owned();

    out.clear();
    err.clear();
    let code = run(
        [
            "axm",
            "dataset",
            "--input",
            out_jsonl.to_string_lossy().as_ref(),
            "--outdir",
            &out_dir_owned,
            "--train",
            "0.5",
            "--val",
            "0.25",
            "--test",
            "0.25",
            "--seed",
            "123",
        ],
        &mut out,
        &mut err,
    );
    match prev_threshold {
        Some(val) => env::set_var(var_name, val),
        None => env::remove_var(var_name),
    }
    assert_eq!(
        code,
        0,
        "dataset should exit 0, stderr={} out={}",
        String::from_utf8_lossy(&err),
        String::from_utf8_lossy(&out),
    );

    let train = fs::read_to_string(out_dir.join("train.jsonl")).unwrap();
    assert!(
        !train.contains('\r'),
        "train split should use LF-only newlines",
    );
    let val = fs::read_to_string(out_dir.join("val.jsonl")).unwrap();
    assert!(!val.contains('\r'), "val split should use LF-only newlines",);
    let test = fs::read_to_string(out_dir.join("test.jsonl")).unwrap();
    assert!(
        !test.contains('\r'),
        "test split should use LF-only newlines",
    );
}
