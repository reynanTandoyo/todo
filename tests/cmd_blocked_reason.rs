//! `summary` and `report` give the same reason for a blocked task (T-140; R-REPORT-1, R-READ-5).
//! The fixture is invented: three blocked tasks with several block notes each, and one note that
//! is not a block note after the reason.

mod common;

use std::collections::BTreeMap;
use std::fs;

use serde_json::Value;
use tempfile::TempDir;

use common::{parse_json, run_todo, setup_fixture_proj, stderr_text, stdout_text};

const NOW: &str = "2026-11-19T10:00:00+01:00";

/// What the report and the text summary show when no block note gives a reason.
const NO_REASON: &str = "no reason recorded";

/// The reason `summary --json` gives for each blocked task, by ID; `None` when it gives none.
fn summary_reasons(json: &Value) -> BTreeMap<String, Option<String>> {
    json["data"]["blocked"]
        .as_array()
        .expect("blocked list")
        .iter()
        .map(|task| {
            (
                task["id"].as_str().expect("id").to_string(),
                task["reason"].as_str().map(str::to_string),
            )
        })
        .collect()
}

/// The reason the report page shows for the blocked task `id`: the first `blocked:` reason after
/// the task's row.
fn report_reason(html: &str, id: &str) -> String {
    let row = html
        .find(&format!("id=\"e-{id}\""))
        .unwrap_or_else(|| panic!("{id} has no row in the report"));
    let rest = &html[row..];
    let marker = "<span class=\"reason\">blocked: ";
    let start = rest
        .find(marker)
        .unwrap_or_else(|| panic!("{id} shows no blocked reason"))
        + marker.len();
    let end = rest[start..].find("</span>").expect("end of reason span");
    rest[start..start + end].to_string()
}

#[test]
fn summary_and_report_give_the_same_reason_for_each_blocked_task() {
    let tmp = TempDir::new().unwrap();
    let (home, work, _todo) = setup_fixture_proj(&tmp, "blocked_reasons.md");

    let summary = run_todo(
        &home,
        &work,
        &["--json", "summary"],
        &[("TODO_TEST_NOW", NOW)],
    )
    .unwrap();
    assert_eq!(summary.status.code(), Some(0), "{}", stderr_text(&summary));
    let reasons = summary_reasons(&parse_json(&summary.stdout));

    // The latest reason: the text after the dash of the latest `blocked — <reason>` note, so the
    // unrelated note after it and the bare `blocked` note do not count.
    assert_eq!(reasons.len(), 3);
    assert_eq!(
        reasons["T-001"].as_deref(),
        Some("the vendor driver crashes on the test scene")
    );
    assert_eq!(
        reasons["T-002"].as_deref(),
        Some("waiting for the design review")
    );
    assert_eq!(reasons["T-003"], None);

    let report = run_todo(
        &home,
        &work,
        &["report", "--html"],
        &[("TODO_TEST_NOW", NOW)],
    )
    .unwrap();
    assert_eq!(report.status.code(), Some(0), "{}", stderr_text(&report));
    let html = fs::read_to_string(work.join(".todo").join("report.html")).unwrap();
    for (id, reason) in &reasons {
        assert_eq!(
            report_reason(&html, id),
            reason.as_deref().unwrap_or(NO_REASON),
            "report reason of {id}"
        );
    }

    // The text overview prints the same reason on each blocked task's line.
    let text = run_todo(&home, &work, &["summary"], &[("TODO_TEST_NOW", NOW)]).unwrap();
    let printed = stdout_text(&text);
    for (id, reason) in &reasons {
        let line = format!("{id} · ");
        let expected = format!("— {}", reason.as_deref().unwrap_or(NO_REASON));
        assert!(
            printed
                .lines()
                .any(|l| l.contains(&line) && l.ends_with(&expected)),
            "summary text line of {id} does not end with {expected:?}:\n{printed}"
        );
    }
}
