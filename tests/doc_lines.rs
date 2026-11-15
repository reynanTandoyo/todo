use std::fs;
use std::path::Path;

use proptest::prelude::*;
use tempfile::TempDir;
use todo::doc::{
    EditError, LineDoc, LineEdit, LineEnding, UTF8_BOM, sanitize_single_line,
    sanitize_trimmed_value, verify_byte_edits,
};

#[test]
fn all_valid_utf8_fixtures_roundtrip_byte_identical() {
    let dir = Path::new("tests/fixtures");
    let mut checked = 0;
    for entry in fs::read_dir(dir).expect("read tests/fixtures") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name == ".gitkeep" || name == "bad_utf8.md" {
            continue;
        }
        let original = fs::read(&path).expect("read fixture");
        let doc = LineDoc::from_bytes(&original)
            .unwrap_or_else(|e| panic!("fixture {name} failed UTF-8 decode: {e}"));
        let written = doc.to_bytes();
        assert_eq!(
            written, original,
            "fixture {name} did not round-trip byte-identical"
        );
        checked += 1;
    }
    assert!(checked >= 9);
}

#[test]
fn bad_utf8_reports_exact_byte_position_and_line() {
    let bytes = fs::read("tests/fixtures/bad_utf8.md").expect("read bad_utf8.md");
    let err = LineDoc::from_bytes(&bytes).expect_err("must reject invalid UTF-8");
    let expected_offset = bytes.iter().position(|&b| b == 0xFF).expect("0xFF marker");
    assert_eq!(err.byte_offset, expected_offset);
    assert_eq!(err.line, 4);
    assert!(err.to_string().contains(&expected_offset.to_string()));
}

#[test]
fn empty_and_bom_only_files_roundtrip() {
    let empty = LineDoc::from_bytes(b"").expect("empty");
    assert!(!empty.has_bom);
    assert!(empty.is_empty());
    assert_eq!(empty.default_ending(), LineEnding::Lf);
    assert_eq!(empty.to_bytes(), b"");

    let bom_only = LineDoc::from_bytes(UTF8_BOM).expect("bom only");
    assert!(bom_only.has_bom);
    assert!(bom_only.is_empty());
    assert_eq!(bom_only.default_ending(), LineEnding::Lf);
    assert_eq!(bom_only.to_bytes(), UTF8_BOM);
}

#[test]
fn default_ending_follows_majority_and_ties_to_lf() {
    let crlf_maj = LineDoc::from_bytes(b"a\r\nb\r\nc\n").expect("decode");
    assert_eq!(crlf_maj.default_ending(), LineEnding::Crlf);

    let lf_maj = LineDoc::from_bytes(b"a\r\nb\nc\n").expect("decode");
    assert_eq!(lf_maj.default_ending(), LineEnding::Lf);

    let tie = LineDoc::from_bytes(b"a\r\nb\n").expect("decode");
    assert_eq!(tie.default_ending(), LineEnding::Lf);

    let no_endings = LineDoc::from_bytes(b"single line").expect("decode");
    assert_eq!(no_endings.default_ending(), LineEnding::Lf);
}

#[test]
fn appending_after_unterminated_last_line_moves_no_ending_to_new_last_line() {
    let orig = b"first\r\nsecond\r\nthird";
    let doc = LineDoc::from_bytes(orig).expect("decode");
    let edits = [LineEdit::insert(
        doc.len(),
        vec!["fourth".to_string(), "fifth".to_string()],
    )];
    let applied = doc.apply_and_verify_bytes(&edits).expect("apply");
    assert!(applied.changed);
    assert_eq!(
        applied.bytes,
        b"first\r\nsecond\r\nthird\r\nfourth\r\nfifth"
    );
    assert_eq!(
        applied.doc.lines.last().map(|l| l.ending),
        Some(LineEnding::None)
    );
}

#[test]
fn multiple_edits_preserve_bom_mixed_endings_and_detect_noop() {
    let orig = b"\xEF\xBB\xBFline1\r\nline2\nline3\r\nline4";
    let doc = LineDoc::from_bytes(orig).expect("decode");

    let noop = doc
        .apply_edits(&[LineEdit::replace_line(1, "line2")])
        .expect("noop");
    assert!(!noop.changed);
    assert_eq!(noop.bytes, orig);

    let edits = [
        LineEdit::replace_line(0, "LINE1"),
        LineEdit::insert(2, vec!["inserted".to_string()]),
        LineEdit::remove(3..4),
    ];
    let applied = doc.apply_and_verify_bytes(&edits).expect("apply");
    assert!(applied.changed);
    // Majority ending in orig: 2 CRLF vs 1 LF -> CRLF for inserted line.
    // line1 keeps CRLF, line2 keeps LF, line4 removed.
    assert_eq!(
        applied.bytes,
        b"\xEF\xBB\xBFLINE1\r\nline2\ninserted\r\nline3\r\n"
    );
}

#[test]
fn sanitize_single_line_replaces_all_linebreak_kinds() {
    assert_eq!(sanitize_single_line("a\r\nb\nc\rd"), "a b c d");
    assert_eq!(sanitize_trimmed_value("  hello\r\nworld  "), "hello world");
}

#[test]
fn rejects_newlines_overlaps_and_out_of_bounds_edits() {
    let doc = LineDoc::from_bytes(b"a\nb\n").expect("decode");

    let err = doc
        .apply_edits(&[LineEdit::replace_line(0, "x\ny")])
        .expect_err("newline rejected");
    assert!(matches!(err, EditError::NewlineInLine { .. }));

    let err = doc
        .apply_edits(&[LineEdit::remove(0..2), LineEdit::replace_line(1, "z")])
        .expect_err("overlap rejected");
    assert!(matches!(err, EditError::OverlappingEdits { .. }));

    let err = doc
        .apply_edits(&[LineEdit::remove(1..5)])
        .expect_err("oob rejected");
    assert!(matches!(err, EditError::RangeOutOfBounds { .. }));

    let unterminated = LineDoc::from_bytes(b"only-line").expect("decode");
    let err = unterminated
        .apply_edits(&[LineEdit::replace_line(0, "")])
        .expect_err("empty unterminated last line rejected");
    assert!(matches!(err, EditError::EmptyUnterminatedLastLine));
}

#[test]
fn verification_catches_deliberately_wrong_edits_and_writes_nothing() {
    let dir = TempDir::new().expect("tempdir");
    let file_path = dir.path().join("TODO.md");
    let orig = b"\xEF\xBB\xBF# Header\r\n- status: open\r\n- owner: none\r\n";
    fs::write(&file_path, orig).expect("write original");

    let edits = [LineEdit::replace_line(1, "- status: claimed")];

    // Simulated buggy output 1: accidentally modified an untouched line too.
    let wrong_untouched = b"\xEF\xBB\xBF# HEADER\r\n- status: claimed\r\n- owner: none\r\n";
    let err = verify_byte_edits(orig, wrong_untouched, &edits).expect_err("must catch wrong line");
    assert!(matches!(err, EditError::VerificationFailed { .. }));

    // Simulated buggy output 2: dropped BOM.
    let wrong_bom = b"# Header\r\n- status: claimed\r\n- owner: none\r\n";
    assert!(verify_byte_edits(orig, wrong_bom, &edits).is_err());

    // Simulated buggy output 3: flipped ending on untouched line.
    let wrong_ending = b"\xEF\xBB\xBF# Header\n- status: claimed\r\n- owner: none\r\n";
    assert!(verify_byte_edits(orig, wrong_ending, &edits).is_err());

    // Simulated buggy output 4: extra line added unexpectedly.
    let wrong_extra =
        b"\xEF\xBB\xBF# Header\r\n- status: claimed\r\n- extra: bad\r\n- owner: none\r\n";
    assert!(verify_byte_edits(orig, wrong_extra, &edits).is_err());

    // Simulate guarded write: only write if verify_byte_edits returns Ok.
    let maybe_write = |candidate: &[u8]| -> Result<(), EditError> {
        verify_byte_edits(orig, candidate, &edits)?;
        fs::write(&file_path, candidate).expect("write");
        Ok(())
    };
    assert!(maybe_write(wrong_untouched).is_err());
    assert_eq!(fs::read(&file_path).expect("read back"), orig);
}

fn arb_line_text() -> impl Strategy<Value = String> {
    prop::string::string_regex("[A-Za-z0-9 _.:·—日本語-]{0,30}").expect("valid regex")
}

fn arb_nonempty_line_text() -> impl Strategy<Value = String> {
    prop::string::string_regex("[A-Za-z0-9 _.:·—日本語-]{1,30}").expect("valid regex")
}

fn arb_ending() -> impl Strategy<Value = LineEnding> {
    prop_oneof![Just(LineEnding::Lf), Just(LineEnding::Crlf)]
}

fn arb_doc_bytes(uniform_ending: bool) -> impl Strategy<Value = Vec<u8>> {
    (
        any::<bool>(),
        arb_ending(),
        prop::collection::vec((arb_line_text(), arb_ending()), 1..15),
        any::<bool>(),
    )
        .prop_map(move |(has_bom, default_end, raw_lines, trailing_nl)| {
            let mut out = Vec::new();
            if has_bom {
                out.extend_from_slice(UTF8_BOM);
            }
            let len = raw_lines.len();
            for (idx, (text, end)) in raw_lines.into_iter().enumerate() {
                // Ensure an unterminated final line is never empty (otherwise it represents 0 bytes).
                let final_text = if idx + 1 == len && !trailing_nl && text.is_empty() {
                    "x".to_string()
                } else {
                    text
                };
                out.extend_from_slice(final_text.as_bytes());
                if idx + 1 == len && !trailing_nl {
                    // Leave last line without an ending.
                } else if uniform_ending {
                    out.extend_from_slice(default_end.as_bytes());
                } else {
                    out.extend_from_slice(end.as_bytes());
                }
            }
            out
        })
}

proptest! {
    #[test]
    fn prop_random_edit_sequences_preserve_untouched_bytes(
        orig_bytes in arb_doc_bytes(false),
        steps in prop::collection::vec((any::<u64>(), prop::collection::vec(arb_nonempty_line_text(), 1..4)), 1..6),
    ) {
        let mut current = LineDoc::from_bytes(&orig_bytes).expect("generated UTF-8");
        for (seed, new_texts) in steps {
            if current.is_empty() {
                let edit = LineEdit::insert(0, new_texts);
                let applied = current.apply_and_verify_bytes(&[edit]).expect("insert into empty");
                current = applied.doc;
                continue;
            }

            let len = current.len();
            let idx = (seed as usize) % (len + 1);
            let op = ((seed >> 8) % 3) as u8;

            let edit = match op {
                0 => LineEdit::insert(idx, new_texts),
                1 if idx < len => LineEdit::replace_line(idx, new_texts[0].clone()),
                _ if idx < len => LineEdit::remove(idx..idx + 1),
                _ => LineEdit::insert(len, new_texts),
            };

            let step_orig_bytes = current.to_bytes();
            let applied = current
                .apply_and_verify_bytes(std::slice::from_ref(&edit))
                .expect("apply & verify");
            prop_assert_eq!(applied.doc.has_bom, current.has_bom);

            // Perturbing an untouched line (when one exists) must fail verification.
            if current.len() >= 2 && edit.range.start > 0 {
                let mut corrupted = applied.doc.clone();
                corrupted.lines[0].text.push('!');
                let corrupted_bytes = corrupted.to_bytes();
                prop_assert!(verify_byte_edits(&step_orig_bytes, &corrupted_bytes, &[edit]).is_err());
            }

            current = applied.doc;
        }
    }

    #[test]
    fn prop_edit_then_reverse_restores_original_bytes(
        orig_bytes in arb_doc_bytes(true),
        seed in any::<u64>(),
        insert_lines in prop::collection::vec(arb_nonempty_line_text(), 1..4),
    ) {
        let doc = LineDoc::from_bytes(&orig_bytes).expect("generated UTF-8");
        prop_assume!(!doc.is_empty());
        // Keep uniform endings matching default_ending so a LineEdit reverse restores exact bytes,
        // and avoid appending past an unterminated EOF which intentionally adds a newline to the old last line.
        let ends_with_nl = doc.lines.last().is_some_and(|l| l.ending != LineEnding::None);
        let default_end = doc.default_ending();
        prop_assume!(doc.lines.iter().all(|l| l.ending == default_end || l.ending == LineEnding::None));

        let len = doc.len();
        let op = (seed % 3) as u8;
        let idx = ((seed >> 4) as usize) % len;

        let (forward, reverse) = match op {
            0 => {
                // Insert before idx (or at len if ends_with_nl).
                let pos = if ends_with_nl && ((seed >> 12) & 1 == 1) { len } else { idx };
                let count = insert_lines.len();
                (
                    LineEdit::insert(pos, insert_lines),
                    LineEdit::remove(pos..pos + count),
                )
            }
            1 => {
                // Replace line idx and restore original text.
                let orig_text = doc.lines[idx].text.clone();
                (
                    LineEdit::replace_line(idx, insert_lines[0].clone()),
                    LineEdit::replace_line(idx, orig_text),
                )
            }
            _ => {
                // Remove a line that has `default_end`, leaving at least one other line with
                // `default_end` so the document's inferred default ending stays the same.
                prop_assume!(len >= 2);
                prop_assume!(doc.lines[idx].ending == default_end);
                prop_assume!(doc.lines.iter().enumerate().any(|(i, l)| i != idx && l.ending == default_end));
                let orig_text = doc.lines[idx].text.clone();
                (
                    LineEdit::remove(idx..idx + 1),
                    LineEdit::insert(idx, vec![orig_text]),
                )
            }
        };

        let after_forward = doc.apply_and_verify_bytes(&[forward]).expect("forward");
        let after_reverse = after_forward.doc.apply_and_verify_bytes(&[reverse]).expect("reverse");
        prop_assert_eq!(after_reverse.bytes, orig_bytes);
    }
}
