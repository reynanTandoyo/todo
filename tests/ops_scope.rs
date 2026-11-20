//! The overlap rules of file scopes (R-SCOPE-1, `docs/format.md` §19), checked on the public
//! `todo::ops::scope` API: paths, folders, patterns, `**`, separators, case, `:<line>` suffixes,
//! and the entries that `check` reports.

use todo::ops::scope::{outside_folder_reason, overlap_lanes, parse_files, scopes_overlap};

/// Whether the `files` value `a` and the `files` value `b` overlap (a task-level comparison).
fn overlaps(a: &str, b: &str) -> bool {
    scopes_overlap(&parse_files(a), &parse_files(b))
}

#[test]
fn equal_paths_overlap_and_different_paths_do_not() {
    assert!(overlaps("src/shader.c", "src/shader.c"));
    assert!(!overlaps("src/shader.c", "src/shader.h"));
    assert!(!overlaps("src/shader.c", "src/shader.c.bak"));
}

#[test]
fn backslashes_and_a_leading_dot_part_are_the_same_path() {
    assert!(overlaps("src\\shader.c", "src/shader.c"));
    assert!(overlaps("./src/shader.c", "src/shader.c"));
}

#[test]
fn case_is_ignored_on_windows_only() {
    assert_eq!(overlaps("SRC/Shader.C", "src/shader.c"), cfg!(windows));
}

#[test]
fn a_trailing_line_number_is_ignored() {
    assert!(overlaps("src/main.rs:42", "src/main.rs"));
    assert!(overlaps("src/main.rs:42", "src/main.rs:7"));
    assert!(!overlaps("src/main.rs:42", "src/lib.rs"));
}

#[test]
fn a_folder_covers_everything_below_it_but_not_a_similar_name() {
    assert!(overlaps("src/gpu/", "src/gpu/shader.c"));
    assert!(overlaps("src/gpu/shader/deep.c", "src/gpu/"));
    assert!(!overlaps("src/gpu/", "src/gpu2/shader.c"));
    // A file named `gpu` is not inside the folder `gpu/`.
    assert!(!overlaps("src/gpu/", "src/gpu"));
}

#[test]
fn folders_overlap_when_one_lies_inside_the_other() {
    assert!(overlaps("src/", "src/gpu/"));
    assert!(overlaps("src/gpu/", "src/"));
    assert!(!overlaps("src/gp/", "src/gpu/"));
}

#[test]
fn a_pattern_matches_files_in_its_own_folder_only() {
    assert!(overlaps("src/*.c", "src/a.c"));
    assert!(!overlaps("src/*.c", "src/a.h"));
    // `*` does not match a `/`, so a file in a folder below is not matched.
    assert!(!overlaps("src/*.c", "src/gpu/a.c"));
}

#[test]
fn a_double_star_part_matches_any_number_of_folders() {
    assert!(overlaps("tests/**/fixture_*.md", "tests/fixture_a.md"));
    assert!(overlaps("tests/**/fixture_*.md", "tests/x/y/fixture_b.md"));
    assert!(!overlaps("tests/**/fixture_*.md", "tests/x/other.md"));
    assert!(overlaps("**/x.c", "x.c"));
    assert!(overlaps("**/x.c", "a/b/x.c"));
}

#[test]
fn a_pattern_naming_folders_covers_what_lies_below_them() {
    assert!(overlaps("src/*/", "src/a/b/c.c"));
    assert!(!overlaps("src/*/", "lib/a/b/c.c"));
}

#[test]
fn two_patterns_overlap_unless_their_fixed_parts_differ() {
    assert!(overlaps("src/*.c", "src/*.h"));
    assert!(!overlaps("src/*.c", "lib/*.c"));
    // A pattern whose fixed part is empty may match anything, so it overlaps everything.
    assert!(overlaps("**/*.md", "docs/*.md"));
    // The fixed parts `src/` and `src/gpu/` do not differ, so these count as overlapping.
    assert!(overlaps("src/*.c", "src/gpu/*.c"));
}

#[test]
fn a_folder_and_a_pattern_overlap_when_the_pattern_can_reach_the_folder() {
    assert!(overlaps("src/", "src/*.c"));
    assert!(!overlaps("docs/", "src/*.c"));
}

#[test]
fn items_are_trimmed_and_empty_items_are_dropped() {
    assert!(parse_files(" , ,").is_empty());
    assert!(overlaps("src/a.c, tests/b.rs ,docs/", "docs/readme.md"));
    assert!(!overlaps("src/a.c, tests/b.rs ,docs/", "lib/x.rs"));
}

#[test]
fn paths_may_contain_spaces() {
    assert!(overlaps("my docs/a b.md", "my docs/a b.md"));
    assert!(!overlaps("my docs/a b.md", "my docs/a.md"));
}

#[test]
fn absolute_entries_and_entries_that_leave_the_folder_are_reported() {
    assert_eq!(outside_folder_reason("/etc/a.c"), Some("is absolute"));
    assert_eq!(outside_folder_reason("C:\\work\\a.c"), Some("is absolute"));
    assert_eq!(
        outside_folder_reason("..\\other\\a.c"),
        Some("leaves the folder of TODO.md")
    );
    assert_eq!(
        outside_folder_reason("src/../../x.c"),
        Some("leaves the folder of TODO.md")
    );
}

#[test]
fn ordinary_relative_entries_are_not_reported() {
    assert_eq!(outside_folder_reason("src/a.c"), None);
    assert_eq!(outside_folder_reason("src/a.c:12"), None);
    assert_eq!(outside_folder_reason("docs/"), None);
    assert_eq!(outside_folder_reason("src/*.c"), None);
}

/// The lanes that compare every pair of items: two items share a lane when their scopes overlap,
/// through any chain of items. Numbered from 0 in order of first appearance, as `overlap_lanes`.
fn pairwise_lanes(items: &[&str]) -> Vec<usize> {
    let scopes: Vec<_> = items.iter().map(|files| parse_files(files)).collect();
    let mut lane: Vec<usize> = (0..items.len()).collect();
    let mut changed = true;
    while changed {
        changed = false;
        for i in 0..items.len() {
            for j in 0..items.len() {
                if lane[i] != lane[j] && scopes_overlap(&scopes[i], &scopes[j]) {
                    let (from, to) = (lane[j], lane[i]);
                    for value in &mut lane {
                        if *value == from {
                            *value = to;
                        }
                    }
                    changed = true;
                }
            }
        }
    }
    let mut numbers: Vec<usize> = Vec::new();
    lane.iter()
        .map(|&value| {
            if let Some(pos) = numbers.iter().position(|&seen| seen == value) {
                pos
            } else {
                numbers.push(value);
                numbers.len() - 1
            }
        })
        .collect()
}

#[test]
fn lanes_join_items_whose_scopes_overlap_directly_or_through_another_item() {
    let items = [
        "src/x.rs, src/y.rs",
        "src/y.rs, src/z.rs",
        "src/z.rs",
        "",
        "src/w.rs",
    ];
    assert_eq!(overlap_lanes(&scopes_of(&items)), vec![0, 0, 0, 1, 2]);
    assert_eq!(overlap_lanes(&scopes_of(&items)), pairwise_lanes(&items));
}

#[test]
fn lanes_match_the_pairwise_rule_for_folders_patterns_and_case() {
    let items = [
        "docs/",
        "docs/a.md",
        "src/*.rs",
        "*/x.rs",
        "**/y.rs",
        "Src/X.RS",
        "src/x.rs:42",
        "./src/w.rs",
        "lib*",
        "lib/a.rs",
        "libs/b.rs",
        "li*/c.rs",
        "src/",
        "src/deep/e.rs",
        "",
        ".",
        "tests/**/fixture.txt",
        "tests/a/fixture.txt",
        "a b/c d.md",
        "a b/c d.md, lib/a.rs",
    ];
    assert_eq!(overlap_lanes(&scopes_of(&items)), pairwise_lanes(&items));
}

#[test]
fn an_item_without_files_is_a_lane_of_its_own() {
    let items = ["", ".", "src/a.rs", "src/a.rs"];
    assert_eq!(overlap_lanes(&scopes_of(&items)), vec![0, 1, 2, 2]);
}

/// The parsed `files` values of `items`, as the planner hands them to `overlap_lanes`.
fn scopes_of(items: &[&str]) -> Vec<Vec<todo::ops::scope::ScopeEntry>> {
    items.iter().map(|files| parse_files(files)).collect()
}
