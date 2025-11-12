#[test]
fn display_warning_formats_with_prefix() {
    let mut buf: Vec<u8> = Vec::new();
    axm_cli::ui::display_warning(&mut buf, "AI opponent is a placeholder").unwrap();
    let output = String::from_utf8(buf).unwrap();
    assert!(output.starts_with("WARNING:"));
    assert!(output.contains("AI opponent is a placeholder"));
}

#[test]
fn warn_parameter_unused_formats_correctly() {
    let mut buf: Vec<u8> = Vec::new();
    axm_cli::ui::warn_parameter_unused(&mut buf, "speed").unwrap();
    let output = String::from_utf8(buf).unwrap();
    assert!(output.starts_with("WARNING:"));
    assert!(output.contains("speed"));
    assert!(output.contains("not used"));
}

#[test]
fn tag_demo_output_appends_indicator() {
    let result = axm_cli::ui::tag_demo_output("ai: check");
    assert!(result.contains("ai: check"));
    assert!(result.contains("[DEMO MODE]"));
}

#[test]
fn tag_demo_output_preserves_original_content() {
    let original = "Player action: bet 100";
    let result = axm_cli::ui::tag_demo_output(original);
    assert!(result.contains(original));
}
