#![allow(clippy::expect_used)] // whole file is test code; helpers outside #[test] may expect()

mod common;

use common::fixture;

use std::fs;

use chrono::NaiveDate;
use tempfile::TempDir;
use todo::doc::{
    CANONICAL_FIELD_ORDER, Document, FieldLocation, IdConfig, LineEdit, ProblemKind,
    ProblemSeverity, SectionLookup, Status, TaskChangeIntent, TaskId, agent_names_equal,
    inspect_problems, is_valid_agent_name, strip_section_number, verify_task_edits,
};

#[test]
fn parses_clean_basic_fixture_completely() {
    let bytes = fixture("clean_basic.md");
    let doc = Document::parse_bytes(&bytes).expect("valid UTF-8");

    assert!(
        doc.problems.is_empty(),
        "unexpected problems: {:?}",
        doc.problems
    );
    assert!(!doc.blocks_all_writes());
    assert_eq!(doc.declared_format_version, Some(1));
    assert_eq!(doc.effective_format_version(), 1);
    assert_eq!(doc.format_line, Some(1));
    assert_eq!(doc.preamble_range, 0..6);
    assert_eq!(doc.sections.len(), 3);
    assert_eq!(doc.sections[0].name, "1. Now");
    assert_eq!(doc.sections[0].normalized_name(), "Now");
    assert_eq!(doc.sections[1].name, "2) Later");
    assert_eq!(doc.sections[1].normalized_name(), "Later");
    assert_eq!(doc.sections[2].name, "Questions");

    assert_eq!(doc.tasks.len(), 6);
    assert_eq!(doc.highest_id(), Some(6));

    // T-001
    let t1 = doc.task_by_id(TaskId(1)).expect("T-001");
    assert_eq!(t1.title, "Water the tomato plants");
    assert_eq!(t1.separator, " · ");
    assert_eq!(doc.task_section_name(t1), Some("1. Now"));
    assert_eq!(t1.status, Status::Claimed);
    assert_eq!(t1.owner.as_deref(), Some("gardener-bot"));
    assert_eq!(t1.claimed.as_deref(), Some("2026-11-15T09:30+01:00"));
    assert!(t1.parse_claimed_at().is_some());
    assert_eq!(t1.priority.as_deref(), Some("P0"));
    assert_eq!(t1.task_type.as_deref(), Some("chore"));
    assert_eq!(
        t1.task_type_loc,
        Some(FieldLocation::CombinedPriority(
            t1.priority_detail.as_ref().expect("prio").line
        ))
    );
    assert_eq!(t1.area.as_deref(), Some("greenhouse"));
    assert_eq!(
        t1.area_loc,
        Some(FieldLocation::CombinedPriority(
            t1.priority_detail.as_ref().expect("prio").line
        ))
    );
    assert_eq!(t1.files.as_deref(), Some("greenhouse/bed1.txt"));
    assert_eq!(
        t1.problem.as_deref(),
        Some("soil moisture dropped below 20%")
    );
    assert_eq!(t1.fix.as_deref(), Some("run drip line for 10 minutes"));
    assert_eq!(t1.accept.as_deref(), Some("moisture reads above 45%"));
    assert_eq!(t1.unknown_fields.len(), 1);
    assert_eq!(t1.unknown_fields[0].key, "custom_tag");
    assert_eq!(t1.unknown_fields[0].value, "morning-shift");
    assert_eq!(t1.field_value("CUSTOM_TAG"), Some("morning-shift"));
    assert_eq!(t1.notes.len(), 2);
    assert_eq!(
        t1.notes[0].date,
        NaiveDate::from_ymd_opt(2026, 11, 15).expect("date")
    );
    assert_eq!(t1.notes[0].agent, "coordinator");
    assert_eq!(t1.notes[0].text, "added");
    assert_eq!(t1.notes[1].agent, "gardener-bot");
    assert_eq!(t1.notes[1].text, "started watering");
    // Trailing blank lines excluded, but internal prose & indented sub-item included.
    assert_eq!(
        doc.line_doc.lines[t1.block_range.end - 1].text,
        "  - indented sub-item is not a field"
    );

    // T-002: standalone type/area, nobody owner `—`, depends on T-001 (which is claimed -> not satisfied)
    let t2 = doc.task_by_id(TaskId(2)).expect("T-002");
    assert_eq!(t2.status, Status::Open);
    assert_eq!(t2.owner, None);
    assert_eq!(t2.task_type.as_deref(), Some("maintenance"));
    assert!(matches!(t2.task_type_loc, Some(FieldLocation::OwnLine(_))));
    assert_eq!(t2.area.as_deref(), Some("fence"));
    assert!(matches!(t2.area_loc, Some(FieldLocation::OwnLine(_))));
    assert_eq!(t2.dependency_ids(), vec![TaskId(1)]);
    assert!(!doc.dependencies_satisfied(t2, None));

    // All nobody forms (`-`, `none`, empty) and remaining statuses (`blocked`, `done`, `dropped`)
    let t3 = doc.task_by_id(TaskId(3)).expect("T-003");
    assert_eq!(t3.status, Status::Blocked);
    assert_eq!(t3.owner, None);

    let t4 = doc.task_by_id(TaskId(4)).expect("T-004");
    assert_eq!(t4.status, Status::Done);
    assert_eq!(t4.owner.as_deref(), Some("helper-2"));
    assert_eq!(t4.commit, vec!["abc1234", "def5678"]);

    let t5 = doc.task_by_id(TaskId(5)).expect("T-005");
    assert_eq!(t5.status, Status::Dropped);
    assert_eq!(t5.owner, None);

    let t6 = doc.task_by_id(TaskId(6)).expect("T-006");
    assert_eq!(t6.status, Status::Open);
    assert_eq!(t6.owner, None);
    assert_eq!(t6.task_type.as_deref(), Some("question"));
}

#[test]
fn messy_fixture_ignores_fences_comments_and_warns_only_on_fake_tasks() {
    let bytes = fixture("messy_crlf_bom.md");
    let doc = Document::parse_bytes(&bytes).expect("valid UTF-8");

    assert!(!doc.blocks_all_writes());
    assert_eq!(doc.declared_format_version, Some(1));
    assert_eq!(doc.sections.len(), 1);
    assert_eq!(doc.sections[0].name, "10. Prep Work");
    assert_eq!(doc.tasks.len(), 2);
    assert_eq!(doc.tasks[0].id, TaskId(10));
    assert_eq!(doc.tasks[0].title, "Bake sourdough bread");
    assert_eq!(doc.tasks[0].unknown_fields.len(), 1);
    assert_eq!(doc.tasks[0].unknown_fields[0].key, "custom-field_1");
    assert_eq!(doc.tasks[0].unknown_fields[0].value, "hydration 75%");
    assert!(doc.tasks[0].notes.is_empty());
    assert_eq!(doc.tasks[1].id, TaskId(14));
    assert_eq!(doc.tasks[1].title, "Clean the mixer");

    // Only the 3 probable-mistake headings (`T11`, `T-12NoSpace`, `T-13 ·   `) produce warnings;
    // `### ADR-001` does not.
    assert_eq!(doc.problems.len(), 3);
    for p in &doc.problems {
        assert_eq!(p.severity, ProblemSeverity::Warning);
        assert_eq!(p.kind, ProblemKind::HeadingProbableMistake);
    }
    // T-010 block ends right before `### T11 Missing hyphen...`, excluding blank lines.
    assert_eq!(
        doc.line_doc.lines[doc.tasks[0].block_range.end - 1].text,
        "Some notes with `### T-996 · inline` text."
    );
}

#[test]
fn mixed_fixture_handles_omitted_format_lenient_ids_separators_and_presection_task() {
    let bytes = fixture("no_final_newline_mixed.md");
    let doc = Document::parse_bytes(&bytes).expect("valid UTF-8");

    assert!(
        doc.problems.is_empty(),
        "unexpected problems: {:?}",
        doc.problems
    );
    assert_eq!(doc.declared_format_version, None);
    assert_eq!(doc.effective_format_version(), 1);
    assert_eq!(doc.preamble_range, 0..1);
    assert_eq!(doc.sections.len(), 1);
    assert_eq!(doc.tasks.len(), 4);

    let t7 = &doc.tasks[0];
    assert_eq!(t7.id, TaskId(7));
    assert_eq!(t7.raw_id, "t-7");
    assert_eq!(t7.separator, " - ");
    assert_eq!(t7.title, "Task before any section — 日本語テスト");
    assert_eq!(t7.section, None);
    assert_eq!(t7.status, Status::Open); // missing status defaults to open
    assert_eq!(t7.dependency_ids(), vec![TaskId(12), TaskId(14)]);

    let t12 = &doc.tasks[1];
    assert_eq!(t12.id, TaskId(12));
    assert_eq!(t12.separator, ": ");
    assert_eq!(t12.section, Some(0));

    let t14 = &doc.tasks[2];
    assert_eq!(t14.id, TaskId(14));
    assert_eq!(t14.separator, " — ");

    let t15 = &doc.tasks[3];
    assert_eq!(t15.id, TaskId(15));
    assert_eq!(t15.separator, " – ");
    assert_eq!(t15.title, "En dash separator without final newline");
}

#[test]
fn detects_managed_block() {
    let bytes = fixture("managed_block.md");
    let doc = Document::parse_bytes(&bytes).expect("valid UTF-8");
    assert!(doc.problems.is_empty());
    let block = doc.managed_block.expect("managed block");
    assert_eq!(block.begin_line, 4);
    assert_eq!(block.end_line, 7);
}

#[test]
fn reports_all_file_level_refusals_from_format_section_13() {
    // 1. Invalid UTF-8
    let bad_utf8 = fixture("bad_utf8.md");
    let utf8_probs = inspect_problems(&bad_utf8, IdConfig::default());
    assert_eq!(utf8_probs.len(), 1);
    assert!(utf8_probs[0].blocks_all_writes());
    assert!(matches!(
        utf8_probs[0].kind,
        ProblemKind::InvalidUtf8 { .. }
    ));
    assert!(
        utf8_probs[0]
            .format_with_path("TODO.md")
            .starts_with("TODO.md:4: invalid UTF-8 at byte ")
    );

    // 2. Version 2 format line
    let v2 = fixture("bad_version_2.md");
    let v2_doc = Document::parse_bytes(&v2).expect("valid UTF-8");
    assert!(v2_doc.blocks_all_writes());
    assert_eq!(v2_doc.declared_format_version, Some(2));
    assert_eq!(v2_doc.tasks.len(), 1); // read commands still work
    assert!(matches!(
        v2_doc.problems[0].kind,
        ProblemKind::UnknownFormatVersion { ref raw } if raw == "2"
    ));
    assert!(
        v2_doc.problems[0]
            .format_with_path("TODO.md")
            .starts_with("TODO.md:2: ")
    );

    // 3. Conflict markers
    let conflict = fixture("bad_conflict_markers.md");
    let conflict_doc = Document::parse_bytes(&conflict).expect("valid UTF-8");
    assert!(conflict_doc.blocks_all_writes());
    let conflict_lines: Vec<usize> = conflict_doc
        .problems
        .iter()
        .filter(|p| p.kind == ProblemKind::ConflictMarker)
        .map(|p| p.line)
        .collect();
    assert_eq!(conflict_lines, vec![5, 7, 9]);

    // 4. Unclosed code fence
    let fence = fixture("bad_unclosed_fence.md");
    let fence_doc = Document::parse_bytes(&fence).expect("valid UTF-8");
    assert!(fence_doc.blocks_all_writes());
    assert_eq!(fence_doc.problems[0].kind, ProblemKind::UnclosedCodeFence);
    assert_eq!(fence_doc.problems[0].line, 6);

    // 5. Unclosed HTML comment
    let comment = fixture("bad_unclosed_comment.md");
    let comment_doc = Document::parse_bytes(&comment).expect("valid UTF-8");
    assert!(comment_doc.blocks_all_writes());
    assert_eq!(
        comment_doc.problems[0].kind,
        ProblemKind::UnclosedHtmlComment
    );
    assert_eq!(comment_doc.problems[0].line, 6);
}

#[test]
fn reports_task_level_refusals_and_cross_archive_duplicates() {
    let bytes = fixture("bad_task_problems.md");
    let doc = Document::parse_bytes(&bytes).expect("valid UTF-8");

    assert!(!doc.blocks_all_writes());
    assert!(doc.blocks_task_write(TaskId(1)));
    assert!(!doc.blocks_task_write(TaskId(2)));

    assert!(doc.problems.iter().any(|p| matches!(
        p.kind,
        ProblemKind::RepeatedField { id: TaskId(1), ref field, .. } if field == "status"
    )));
    assert!(doc.problems.iter().any(|p| matches!(
        p.kind,
        ProblemKind::DuplicateTaskId {
            id: TaskId(1),
            first_line: 4
        }
    )));
    assert!(doc.problems.iter().any(|p| matches!(
        p.kind,
        ProblemKind::UnknownStatus { id: TaskId(1), ref value } if value == "paused"
    )));
    assert!(doc.problems.iter().any(|p| matches!(
        p.kind,
        ProblemKind::RepeatedField { id: TaskId(1), ref field, .. } if field == "type"
    )));
    assert!(
        doc.problems
            .iter()
            .any(|p| p.kind == ProblemKind::HeadingProbableMistake
                && p.severity == ProblemSeverity::Warning)
    );

    // Archive duplicate check
    let main_doc = Document::parse_bytes(&fixture("clean_basic.md")).expect("clean");
    let archive_doc =
        Document::parse_bytes(b"## Archive\n### T-002 \xC2\xB7 Archived copy\n- status: done\n")
            .expect("archive");
    let archive_dups = main_doc.check_archive_duplicates(&archive_doc);
    assert_eq!(archive_dups.len(), 1);
    assert!(archive_dups[0].blocks_task(TaskId(2)));
    assert!(!archive_dups[0].blocks_task(TaskId(1)));
}

#[test]
fn id_config_parses_flexible_inputs_and_formats_canonically() {
    let cfg = IdConfig::default();
    for raw in ["12", "t12", "T12", "T-12", "t-12", "T-012", "t-0012"] {
        assert_eq!(cfg.parse_user_id(raw), Some(TaskId(12)), "failed for {raw}");
    }
    assert_eq!(cfg.parse_user_id("ADR-12"), None);
    assert_eq!(cfg.parse_user_id("T-"), None);
    assert_eq!(cfg.format_id(TaskId(7)), "T-007");
    assert_eq!(cfg.format_id(TaskId(1000)), "T-1000");

    let custom = IdConfig::new("bug", 4);
    assert_eq!(custom.parse_user_id("12"), Some(TaskId(12)));
    assert_eq!(custom.parse_user_id("bug12"), Some(TaskId(12)));
    assert_eq!(custom.parse_user_id("BUG-0012"), Some(TaskId(12)));
    assert_eq!(custom.parse_user_id("T-12"), None);
    assert_eq!(custom.format_id(TaskId(12)), "BUG-0012");
    assert_eq!(custom.format_id(TaskId(12345)), "BUG-12345");
}

#[test]
fn section_matching_ignores_leading_numbers_and_case() {
    let bytes = fixture("clean_basic.md");
    let doc = Document::parse_bytes(&bytes).expect("parse");

    assert_eq!(strip_section_number("12. Phase 2"), "Phase 2");
    assert_eq!(strip_section_number("3) Backlog"), "Backlog");
    assert_eq!(strip_section_number("2026 Roadmap"), "2026 Roadmap");

    assert_eq!(doc.find_section("now"), SectionLookup::Found(0));
    assert_eq!(doc.find_section("1. Now"), SectionLookup::Found(0));
    assert_eq!(doc.find_section("Later"), SectionLookup::Found(1));
    assert_eq!(doc.find_section("quest"), SectionLookup::Found(2));
    assert_eq!(doc.find_section("missing"), SectionLookup::NotFound);

    let amb_doc = Document::parse_bytes(b"## 1. API Core\n## 2. API CLI\n").expect("parse");
    assert_eq!(
        amb_doc.find_section("api"),
        SectionLookup::Ambiguous(vec![0, 1])
    );
}

#[test]
fn canonical_field_order_and_insertion_lines_follow_format_section_10() {
    assert_eq!(
        CANONICAL_FIELD_ORDER,
        &[
            "status", "owner", "claimed", "session", "priority", "type", "area", "why", "depends",
            "files", "problem", "fix", "accept", "commit"
        ]
    );

    let sample = b"## Tasks\n### T-001 \xC2\xB7 Example\n- status: open\n- priority: P1 \xC2\xB7 type: bug\n- accept: works\n- custom_a: 1\n";
    let doc = Document::parse_bytes(sample).expect("parse");
    let task = doc.task_by_id(TaskId(1)).expect("T-001");

    // `owner` goes right after `status` (line 2 -> insert at 3).
    assert_eq!(task.insert_line_for_field("owner"), 3);
    // `area` goes right after combined `priority`/`type` (line 3 -> insert at 4).
    assert_eq!(task.insert_line_for_field("area"), 4);
    // `why` comes after `area` and before `depends`/`problem`; with no line above it, it also
    // goes right after the combined `priority`/`type` line (insert at 4).
    assert_eq!(task.insert_line_for_field("why"), 4);
    // `commit` goes right after `accept` (line 4 -> insert at 5).
    assert_eq!(task.insert_line_for_field("commit"), 5);
    // Another unknown field goes after `custom_a` (line 5 -> insert at 6).
    assert_eq!(task.insert_line_for_field("custom_b"), 6);
    // Notes append at end of block.
    assert_eq!(task.note_insert_line(), 6);
}

#[test]
fn why_is_read_as_its_own_field_from_a_hand_written_line() {
    // A `why` line written by hand before v0.6 was an unknown field; it is now read as `why`
    // (R-WRITE-17), and an empty `- why:` means no value (line kept, value absent).
    let sample = b"## Tasks\n### T-001 \xC2\xB7 Example\n- status: open\n- priority: P1\n- why: the renderer has no shading stage yet\n- depends: T-002\n- problem: none\n- custom_a: 1\n### T-002 \xC2\xB7 Empty why\n- status: open\n- why:\n";
    let doc = Document::parse_bytes(sample).expect("parse");
    assert!(
        doc.problems.is_empty(),
        "unexpected problems: {:?}",
        doc.problems
    );

    let t1 = doc.task_by_id(TaskId(1)).expect("T-001");
    assert_eq!(
        t1.why.as_deref(),
        Some("the renderer has no shading stage yet")
    );
    assert_eq!(t1.why_line, Some(4));
    assert_eq!(
        t1.field_value("why"),
        Some("the renderer has no shading stage yet")
    );
    assert!(t1.unknown_fields.iter().all(|f| f.key != "why"));
    // Unrelated unknown fields are still kept as written.
    assert_eq!(t1.unknown_fields.len(), 1);
    assert_eq!(t1.unknown_fields[0].key, "custom_a");
    // A new `depends` line would go after `why`, which comes before `depends` in §10.
    assert_eq!(t1.insert_line_for_field("depends"), 5);

    let t2 = doc.task_by_id(TaskId(2)).expect("T-002");
    assert_eq!(t2.why, None);
    assert_eq!(t2.why_line, Some(10));
    assert_eq!(t2.field_value("why"), None);
}

#[test]
fn repeated_why_line_is_a_repeated_field_problem() {
    let doc = Document::parse_bytes(b"## Tasks\n### T-001 \xC2\xB7 Twice\n- why: a\n- why: b\n")
        .expect("parse");
    assert!(
        doc.problems.iter().any(|p| matches!(
            &p.kind,
            ProblemKind::RepeatedField { field, .. } if field == "why"
        )),
        "expected a repeated `why` problem, got {:?}",
        doc.problems
    );
}

#[test]
fn dependencies_satisfied_checks_current_and_archive_documents() {
    let bytes = fixture("clean_basic.md");
    let doc = Document::parse_bytes(&bytes).expect("parse");

    // T-002 depends on T-001 (claimed) -> not satisfied.
    let t2 = doc.task_by_id(TaskId(2)).expect("T-002");
    assert!(!doc.dependencies_satisfied(t2, None));

    let with_archive_dep = Document::parse_bytes(
        b"## Now\n### T-010 \xC2\xB7 Needs archived task\n- depends: T-004\n",
    )
    .expect("parse");
    let t10 = with_archive_dep.task_by_id(TaskId(10)).expect("T-010");
    // Without archive, T-004 is missing -> not satisfied.
    assert!(!with_archive_dep.dependencies_satisfied(t10, None));
    // With archive (`doc` has T-004 as `done`), satisfied!
    assert!(with_archive_dep.dependencies_satisfied(t10, Some(&doc)));
}

#[test]
fn task_level_verification_allows_intended_edit_and_rejects_unintended_changes() {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join("TODO.md");
    let orig_bytes = fixture("clean_basic.md");
    fs::write(&path, &orig_bytes).expect("write");

    let doc = Document::parse_bytes(&orig_bytes).expect("parse");
    let t1 = doc.task_by_id(TaskId(1)).expect("T-001");
    let status_line = t1.status_line.expect("status line");

    // 1. Intended single-task edit passes both byte and parsed-task verification.
    let ok_edit = [LineEdit::replace_line(status_line, "- status: done")];
    let applied = doc
        .apply_and_verify(&ok_edit, &TaskChangeIntent::modify_one(TaskId(1)))
        .expect("verified edit");
    assert!(applied.changed);
    assert_eq!(
        applied.doc.task_by_id(TaskId(1)).expect("T-001").status,
        Status::Done
    );

    // 2. Claiming intent is only T-002 when editing T-001 fails task verification.
    let wrong_intent = doc.apply_and_verify(&ok_edit, &TaskChangeIntent::modify_one(TaskId(2)));
    assert!(wrong_intent.is_err());

    // 3. Opening an unclosed fence inside T-001 (which swallows T-002..T-006) fails verification.
    let bad_fence_edit = [LineEdit::insert(
        t1.note_insert_line(),
        vec!["```".to_string()],
    )];
    let fence_err = doc.apply_and_verify(&bad_fence_edit, &TaskChangeIntent::modify_one(TaskId(1)));
    assert!(fence_err.is_err());

    // 4. Adding a task with matching add_one intent passes.
    let sec_end = doc.sections[0].content_end;
    let add_edit = [LineEdit::insert(
        sec_end,
        vec![
            String::new(),
            "### T-007 · Mulch the strawberry patch".to_string(),
            "- status: open".to_string(),
        ],
    )];
    let added = doc
        .apply_and_verify(&add_edit, &TaskChangeIntent::add_one(TaskId(7)))
        .expect("add T-007");
    assert_eq!(added.doc.tasks.len(), 7);

    // 5. Removing T-005 with remove_many intent passes; removing without intent fails.
    let t5 = doc.task_by_id(TaskId(5)).expect("T-005");
    let remove_edit = [LineEdit::remove(t5.block_range.clone())];
    assert!(
        doc.apply_and_verify(&remove_edit, &TaskChangeIntent::default())
            .is_err()
    );
    let removed = doc
        .apply_and_verify(&remove_edit, &TaskChangeIntent::remove_many([TaskId(5)]))
        .expect("remove T-005");
    assert!(removed.doc.task_by_id(TaskId(5)).is_none());

    // 6. Direct verify_task_edits catches reordering untouched tasks.
    let reordered = Document::parse_bytes(
        b"## Now\n### T-002 \xC2\xB7 Repair the wooden gate\n### T-001 \xC2\xB7 Water the tomato plants\n",
    )
    .expect("reordered");
    let simple_orig = Document::parse_bytes(
        b"## Now\n### T-001 \xC2\xB7 Water the tomato plants\n### T-002 \xC2\xB7 Repair the wooden gate\n",
    )
    .expect("orig");
    assert!(verify_task_edits(&simple_orig, &reordered, &TaskChangeIntent::default()).is_err());

    // Original temp file remained untouched throughout failed verifications.
    assert_eq!(fs::read(&path).expect("read"), orig_bytes);
}

#[test]
fn validates_agent_names_per_format_section_11() {
    assert!(is_valid_agent_name("agent-ui-1"));
    assert!(is_valid_agent_name("Claude_Code.v2"));
    assert!(is_valid_agent_name(&"a".repeat(64)));
    assert!(!is_valid_agent_name(""));
    assert!(!is_valid_agent_name(&"a".repeat(65)));
    assert!(!is_valid_agent_name("agent 1"));
    assert!(!is_valid_agent_name("agent/1"));
    assert!(agent_names_equal("Agent-1", "agent-1"));
}
