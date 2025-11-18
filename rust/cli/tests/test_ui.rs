#[test]
fn write_error_formats_consistently() {
    let mut buf: Vec<u8> = Vec::new();
    axiomind_cli::ui::write_error(&mut buf, "oops").unwrap();
    assert_eq!(String::from_utf8(buf).unwrap(), "Error: oops\n");
}
