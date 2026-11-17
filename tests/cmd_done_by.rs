//! CLI integration tests for the `by` field of `done` (R-WRITE-16, NFR-5): who finished a task,
//! mapped through `.mailmap`, plus `reopen`, `list --by` and the `check` warning.

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

use common::{read_text, run_todo, setup_proj, stderr_text, stdout_text};

const NOW: &str = "2026-11-15T14:30:00+01:00";

const BODY: &str = "<!-- TEST FIXTURE: invented data, not a real task list -->\n\
## Now\n\n\
### T-001 · First task\n\
- status: open\n\
- owner: —\n\
\n\
### T-002 · Second task\n\
- status: open\n\
- owner: —\n\
\n\
### T-003 · Third task\n\
- status: open\n\
- owner: —\n";

/// Two different identities that `.mailmap` maps to one name.
const MAILMAP: &str = "# invented identities\n\
Ada Lovelace <ada@example.org> <ada@laptop.invalid>\n\
Ada Lovelace <ada@example.org> Ada L <ada@work.invalid>\n";

/// Runs git in `dir` with the developer's own git configuration out of the way.
fn git_as(dir: &Path, name_email: Option<(&str, &str)>, args: &[&str]) -> String {
    let mut cmd = Command::new("git");
    cmd.current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", dir.join("no-such-gitconfig"))
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env_remove("GIT_AUTHOR_NAME")
        .env_remove("GIT_AUTHOR_EMAIL");
    if let Some((name, email)) = name_email {
        cmd.arg("-c")
            .arg(format!("user.name={name}"))
            .arg("-c")
            .arg(format!("user.email={email}"));
    }
    let out = cmd.args(args).output().expect("run git");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

struct Proj {
    _tmp: TempDir,
    home: PathBuf,
    work: PathBuf,
    todo: PathBuf,
}

fn proj() -> Proj {
    let tmp = TempDir::new().unwrap();
    let (home, work, todo) = setup_proj(&tmp, BODY);
    Proj {
        _tmp: tmp,
        home,
        work,
        todo,
    }
}

/// Makes `work` a repository with `.mailmap` and two commits by two identities; returns the hashes.
fn repo_with_two_authors(p: &Proj) -> (String, String) {
    git_as(&p.work, None, &["init", "-q"]);
    fs::write(p.work.join(".mailmap"), MAILMAP).unwrap();
    git_as(&p.work, None, &["add", "."]);
    let first = Some(("Ada L", "ada@work.invalid"));
    git_as(&p.work, first, &["commit", "-q", "-m", "one"]);
    let h1 = git_as(&p.work, None, &["rev-parse", "--short", "HEAD"]);
    fs::write(p.work.join("other.txt"), "x\n").unwrap();
    git_as(&p.work, None, &["add", "."]);
    let second = Some(("ada", "ada@laptop.invalid"));
    git_as(&p.work, second, &["commit", "-q", "-m", "two"]);
    let h2 = git_as(&p.work, None, &["rev-parse", "--short", "HEAD"]);
    (h1, h2)
}

fn todo(p: &Proj, args: &[&str]) -> std::process::Output {
    run_todo(&p.home, &p.work, args, &[("TODO_TEST_NOW", NOW)]).unwrap()
}

fn ok(out: &std::process::Output) {
    assert!(out.status.success(), "stderr: {}", stderr_text(out));
}

#[test]
fn done_with_commit_records_mailmap_name_of_either_identity() {
    let p = proj();
    let (h1, h2) = repo_with_two_authors(&p);

    ok(&todo(
        &p,
        &["done", "T-001", "--agent", "bot", "--commit", &h1],
    ));
    ok(&todo(
        &p,
        &["done", "T-002", "--agent", "bot", "--commit", &h2],
    ));

    let text = read_text(&p.todo);
    assert_eq!(text.matches("- by: Ada Lovelace\n").count(), 2, "{text}");
    assert!(!text.contains('@'), "no email in TODO.md: {text}");
    assert!(!text.contains("Ada L\n"), "{text}");
}

#[test]
fn done_without_commit_uses_configured_identity_through_mailmap() {
    let p = proj();
    git_as(&p.work, None, &["init", "-q"]);
    fs::write(p.work.join(".mailmap"), MAILMAP).unwrap();
    git_as(&p.work, None, &["config", "user.name", "Ada L"]);
    git_as(&p.work, None, &["config", "user.email", "ada@work.invalid"]);

    ok(&todo(&p, &["done", "T-001", "--agent", "bot"]));
    let text = read_text(&p.todo);
    assert!(text.contains("- by: Ada Lovelace\n"), "{text}");
    assert!(!text.contains('@'), "{text}");
}

#[test]
fn done_without_commit_and_unmapped_identity_uses_configured_name() {
    let p = proj();
    git_as(&p.work, None, &["init", "-q"]);
    git_as(&p.work, None, &["config", "user.name", "Grace H"]);
    git_as(
        &p.work,
        None,
        &["config", "user.email", "grace@example.org"],
    );

    ok(&todo(&p, &["done", "T-001", "--agent", "bot"]));
    assert!(read_text(&p.todo).contains("- by: Grace H\n"));
}

#[test]
fn done_without_repo_or_name_leaves_by_out_without_error() {
    // Not a repository: the identity in the isolated global config is not used.
    let p = proj();
    fs::write(
        p.home.join("gitconfig-none"),
        "[user]\n\tname = Global Person\n\temail = g@example.org\n",
    )
    .unwrap();
    ok(&todo(&p, &["done", "T-001", "--agent", "bot"]));
    assert!(!read_text(&p.todo).contains("- by:"));

    // A repository without `user.name`.
    git_as(&p.work, None, &["init", "-q"]);
    let _ = fs::remove_file(p.home.join("gitconfig-none"));
    ok(&todo(&p, &["done", "T-002", "--agent", "bot"]));
    assert!(!read_text(&p.todo).contains("- by:"));
}

#[test]
fn done_replaces_existing_by_and_reopen_removes_it() {
    let p = proj();
    git_as(&p.work, None, &["init", "-q"]);
    git_as(&p.work, None, &["config", "user.name", "Ada Lovelace"]);
    git_as(&p.work, None, &["config", "user.email", "ada@example.org"]);

    ok(&todo(
        &p,
        &[
            "set",
            "T-001",
            "--agent",
            "bot",
            "--field",
            "by=Someone Else",
        ],
    ));
    assert!(read_text(&p.todo).contains("- by: Someone Else\n"));

    ok(&todo(&p, &["done", "T-001", "--agent", "bot"]));
    let text = read_text(&p.todo);
    assert!(text.contains("- by: Ada Lovelace\n"), "{text}");
    assert_eq!(text.matches("- by:").count(), 1, "{text}");

    ok(&todo(
        &p,
        &["reopen", "T-001", "--agent", "bot", "--note", "again"],
    ));
    assert!(!read_text(&p.todo).contains("- by:"));
}

#[test]
fn apply_done_records_by_from_commit_and_from_config() {
    let p = proj();
    let (h1, _h2) = repo_with_two_authors(&p);
    git_as(&p.work, None, &["config", "user.name", "Ada L"]);
    git_as(&p.work, None, &["config", "user.email", "ada@work.invalid"]);

    let ops = format!(
        "{{\"op\":\"done\",\"task\":\"T-001\",\"commit\":\"{h1}\"}}\n{{\"op\":\"done\",\"task\":\"T-002\"}}\n"
    );
    let ops_file = p.work.join("ops.jsonl");
    fs::write(&ops_file, ops).unwrap();
    ok(&todo(
        &p,
        &["apply", ops_file.to_str().unwrap(), "--agent", "bot"],
    ));

    let text = read_text(&p.todo);
    assert_eq!(text.matches("- by: Ada Lovelace\n").count(), 2, "{text}");
    assert!(!text.contains('@'), "{text}");
}

#[test]
fn list_by_filters_ignoring_case_and_implies_status_all() {
    let p = proj();
    git_as(&p.work, None, &["init", "-q"]);
    git_as(&p.work, None, &["config", "user.name", "Ada Lovelace"]);
    git_as(&p.work, None, &["config", "user.email", "ada@example.org"]);
    ok(&todo(&p, &["done", "T-001", "--agent", "bot"]));
    git_as(&p.work, None, &["config", "user.name", "Grace H"]);
    ok(&todo(&p, &["done", "T-002", "--agent", "bot"]));

    let out = todo(&p, &["list", "--by", "ada LOVELACE"]);
    ok(&out);
    let text = stdout_text(&out);
    assert!(text.contains("T-001"), "{text}");
    assert!(!text.contains("T-002") && !text.contains("T-003"), "{text}");

    let out = todo(&p, &["list", "--by", "grace h", "--status", "open"]);
    ok(&out);
    assert!(!stdout_text(&out).contains("T-002"));
}

#[test]
fn check_warns_about_by_names_missing_from_mailmap_without_failing() {
    let p = proj();
    git_as(&p.work, None, &["init", "-q"]);
    git_as(&p.work, None, &["config", "user.name", "Grace H"]);
    git_as(
        &p.work,
        None,
        &["config", "user.email", "grace@example.org"],
    );
    ok(&todo(&p, &["done", "T-001", "--agent", "bot"]));
    git_as(&p.work, None, &["config", "user.name", "Ada Lovelace"]);
    ok(&todo(&p, &["done", "T-002", "--agent", "bot"]));

    // No .mailmap: nothing to check against.
    let out = todo(&p, &["check"]);
    let before = out.status.code();
    assert!(
        !stderr_text(&out).contains(".mailmap"),
        "{}",
        stderr_text(&out)
    );

    fs::write(p.work.join(".mailmap"), MAILMAP).unwrap();
    let out = todo(&p, &["check"]);
    let err = stderr_text(&out);
    assert!(err.contains("T-001") && err.contains("Grace H"), "{err}");
    assert!(err.contains(".mailmap"), "{err}");
    assert!(!err.contains("T-002"), "{err}");
    assert_eq!(out.status.code(), before, "warnings keep the exit code");
}
