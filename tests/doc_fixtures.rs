use std::fs;
use std::path::Path;

const FIXTURE_HEADER: &[u8] = b"<!-- TEST FIXTURE: invented data, not a real task list -->";
const UTF8_BOM: &[u8] = b"\xEF\xBB\xBF";

#[test]
fn every_fixture_starts_with_fixture_banner() {
    let dir = Path::new("tests/fixtures");
    let mut count = 0;
    for entry in fs::read_dir(dir).expect("read tests/fixtures") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        if path.file_name().and_then(|n| n.to_str()) == Some(".gitkeep") {
            continue;
        }
        count += 1;
        let bytes = fs::read(&path).expect("read fixture bytes");
        let after_bom = bytes.strip_prefix(UTF8_BOM).unwrap_or(&bytes);
        assert!(
            after_bom.starts_with(FIXTURE_HEADER),
            "fixture {} must start with the invented-fixture banner",
            path.display()
        );
    }
    assert!(count >= 10, "expected at least 10 fixtures, found {count}");
}

#[test]
fn messy_fixture_keeps_exact_bom_crlf_and_trailing_spaces() {
    let bytes = fs::read("tests/fixtures/messy_crlf_bom.md").expect("read messy fixture");
    assert!(
        bytes.starts_with(UTF8_BOM),
        "messy fixture must have UTF-8 BOM"
    );
    let text = std::str::from_utf8(&bytes[UTF8_BOM.len()..]).expect("valid UTF-8 after BOM");
    assert!(text.contains("\r\n"), "messy fixture must use CRLF");
    assert!(
        !text.replace("\r\n", "").contains('\n'),
        "messy fixture must have only CRLF"
    );
    assert!(
        text.contains("   \r\n"),
        "messy fixture must keep trailing spaces"
    );
}

#[test]
fn mixed_fixture_has_mixed_endings_and_no_final_newline() {
    let bytes = fs::read("tests/fixtures/no_final_newline_mixed.md").expect("read mixed fixture");
    assert!(!bytes.starts_with(UTF8_BOM));
    assert!(!bytes.ends_with(b"\n"));
    let text = std::str::from_utf8(&bytes).expect("valid UTF-8");
    assert!(text.contains("\r\n"), "must contain CRLF");
    assert!(
        text.replace("\r\n", "").contains('\n'),
        "must also contain LF"
    );
}

#[test]
fn bad_utf8_fixture_has_invalid_bytes() {
    let bytes = fs::read("tests/fixtures/bad_utf8.md").expect("read bad_utf8 fixture");
    assert!(std::str::from_utf8(&bytes).is_err());
}
