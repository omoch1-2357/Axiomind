use serde_json::Value;

#[allow(dead_code)]
pub trait PokerAssertions {
    fn assert_help_contains_commands(&self, help_text: &str);
    fn assert_jsonl_format(&self, content: &str);
    fn assert_required_fields(&self, content: &str, fields: &[&str]);
    fn assert_chip_conservation(&self, content: &str);
    fn assert_valid_hand_id(&self, hand_id: &str);
    fn assert_deterministic_output(&self, _seed: u64, out1: &str, out2: &str);
}

#[derive(Debug, Default, Copy, Clone)]
#[allow(dead_code)]
pub struct DefaultAsserter;

#[allow(dead_code)]
pub(crate) fn commands_list() -> &'static [&'static str] {
    &[
        "play", "replay", "sim", "eval", "stats", "verify", "deal", "bench", "rng", "cfg",
        "doctor", "export",
        "dataset",
        // Note: "serve" and "train" removed per Requirements 5 & 6 (not implemented)
    ]
}

impl PokerAssertions for DefaultAsserter {
    fn assert_help_contains_commands(&self, help_text: &str) {
        for c in commands_list() {
            assert!(
                help_text.contains(c),
                "help should contain command `{}`\n---help---\n{}\n----------",
                c,
                help_text
            );
        }
    }

    fn assert_jsonl_format(&self, content: &str) {
        for (i, line) in content.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            serde_json::from_str::<Value>(line)
                .unwrap_or_else(|e| panic!("invalid JSON at line {}: {}\n{}", i + 1, e, line));
        }
    }

    fn assert_required_fields(&self, content: &str, fields: &[&str]) {
        // Check first non-empty line only (schema spot-check)
        let first = content
            .lines()
            .find(|l| !l.trim().is_empty())
            .expect("no lines");
        let v: Value = serde_json::from_str(first).expect("first line must be JSON");
        let obj = v.as_object().expect("record must be object");
        for f in fields {
            assert!(
                obj.contains_key(*f),
                "missing required field `{}` in {}",
                f,
                first
            );
        }
    }

    fn assert_chip_conservation(&self, content: &str) {
        for (i, line) in content.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let v: Value = serde_json::from_str(line).expect("json");
            let net = v.get("net_result").expect("net_result");
            let map = net.as_object().expect("net_result must be object");
            let mut sum: i64 = 0;
            for (_k, val) in map.iter() {
                sum += val.as_i64().expect("net_result values must be integers");
            }
            assert_eq!(
                sum,
                0,
                "chip conservation violated at line {}: {} (sum={})",
                i + 1,
                line,
                sum
            );
        }
    }

    fn assert_valid_hand_id(&self, hand_id: &str) {
        let ok = hand_id.len() == 15
            && hand_id.chars().take(8).all(|c| c.is_ascii_digit())
            && &hand_id[8..9] == "-"
            && hand_id.chars().skip(9).all(|c| c.is_ascii_digit());
        assert!(
            ok,
            "invalid hand_id format (expected YYYYMMDD-NNNNNN): {}",
            hand_id
        );
    }

    fn assert_deterministic_output(&self, _seed: u64, out1: &str, out2: &str) {
        assert_eq!(out1, out2, "outputs differ for same seed");
    }
}

#[allow(dead_code)]
pub fn asserter() -> DefaultAsserter {
    DefaultAsserter
}
