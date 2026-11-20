//! `todo brief` command: the work order of one task, one text a worker reads before it starts
//! (T-154; R-DETAIL-3, R-MILE-4, R-LOCK-5, R-READ-4).

use clap::Args as ClapArgs;

use crate::cli::{CommandKind, CommandMeta, Context, load_read, write_milestone_chain};
use crate::doc::EntryKind;
use crate::error::{ErrorCode, Result, TodoError};
use crate::ops::brief::{BriefJson, build_brief};
use crate::ops::resolve_task_in_doc;
use crate::output::Output;

/// Metadata for the `brief` command.
pub const META: CommandMeta = CommandMeta {
    name: "brief",
    summary: "Print the work order of a task: goals above it, files, steps, check, finish commands",
    kind: CommandKind::Read,
    details: "Prints one work order for a task, so a worker reads one text and starts (R-DETAIL-3): the goals of the milestones above the task, from the top down (R-MILE-4); its ID, title and why; the files to read (`read`); the files it may change (`files`), with the sentence that no other file may be changed; the numbered steps; the `verify` command and whether `done` runs it; for each dependency that is done its done note and commit; and the commands to finish (commit only the listed files, then `todo done <ID> --commit HEAD`) and to give up (`todo stuck <ID> --note`), with the agent name from `--agent` or `TODO_AGENT`, or a placeholder. `next --claim --brief` and `claim --brief` print the same after the claim. Takes a task ID or part of its title; a milestone, question or idea is refused (`BAD_VALUE`). `--json` returns the same parts (`todo.brief.v1`). Reads only; takes no lock.",
    examples: &[
        "todo brief T-021 --agent worker-1",
        "todo brief T-021 --json",
    ],
};

/// Arguments for `todo brief`.
#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Task ID (e.g. `T-001`, `t1`, or `1`) or part of its title.
    #[arg(value_name = "TASK")]
    pub task: String,
}

/// Runs `todo brief <task>`.
pub fn run(args: Args, ctx: &Context, out: &mut Output) -> Result<()> {
    let (cfg, loaded) = load_read(ctx, out, false, true)?;
    let task = resolve_task_in_doc(&loaded.doc, &args.task)?;
    if task.kind() != EntryKind::Task {
        let id = cfg.id_config.format_id(task.id);
        return Err(TodoError::new(
            ErrorCode::BadValue,
            format!(
                "{id} is a {}, not a task: a work order is printed for a task",
                task.kind().as_str()
            ),
        )
        .with_task(id));
    }
    let brief = build_brief(
        &loaded.doc,
        loaded.archive.as_ref(),
        task,
        &cfg,
        ctx.agent.as_deref(),
    );
    // Read-only: the work order is built, so the documents are not needed any more (T-186).
    loaded.leak_for_exit();
    if out.is_json() {
        out.emit_json_ok(META.name, &brief);
    } else {
        write_brief(out, &brief);
    }
    Ok(())
}

/// Writes the work order as text, in the order of R-DETAIL-3. Also used by `next --claim --brief`
/// and `claim --brief`, so the text after a claim is the same as `todo brief` prints.
pub(crate) fn write_brief(out: &mut Output, brief: &BriefJson) {
    if !brief.milestones.is_empty() {
        write_milestone_chain(out, &brief.id, &brief.milestones);
        out.write_line("");
    }
    out.write_line(&format!("{} \u{00B7} {}", brief.id, brief.title));
    out.write_line(&format!("why: {}", brief.why.as_deref().unwrap_or("-")));
    out.write_line("");

    if brief.read.is_empty() {
        out.write_line("Files to read (read): none listed.");
    } else {
        out.write_line("Files to read (read):");
        for path in &brief.read {
            out.write_line(&format!("- {path}"));
        }
    }
    out.write_line("");

    if brief.files.is_empty() {
        out.write_line("Files you may change (files): none listed, so the task may change any file.");
    } else {
        out.write_line("Files you may change (files):");
        for path in &brief.files {
            out.write_line(&format!("- {path}"));
        }
        out.write_line("No other file may be changed.");
    }
    out.write_line("");

    if brief.steps.is_empty() {
        out.write_line("Steps: none written.");
    } else {
        out.write_line("Steps:");
        for (number, step) in brief.steps.iter().enumerate() {
            out.write_line(&format!("{}. {step}", number + 1));
        }
    }
    out.write_line("");

    match &brief.verify {
        None => out.write_line("Verify: none; nothing checks this task."),
        Some(verify) => {
            out.write_line(&format!("Verify: {}", verify.command));
            let when = match verify.runs {
                "wave" => "todo verify --wave runs it after the wave, not done",
                "never" => "todo never runs it (verify = false)",
                _ => "done runs it",
            };
            out.write_line(&format!("  {when}"));
        }
    }
    out.write_line("");

    if brief.finished_dependencies.is_empty() {
        out.write_line("Finished dependencies: none.");
    } else {
        out.write_line("Finished dependencies:");
        for dep in &brief.finished_dependencies {
            out.write_line(&format!("- {} \u{00B7} {}", dep.id, dep.title));
            out.write_line(&format!(
                "  done note: {}",
                dep.note.as_deref().unwrap_or("-")
            ));
            let commits = if dep.commit.is_empty() {
                "-".to_string()
            } else {
                dep.commit.join(", ")
            };
            out.write_line(&format!("  commit: {commits}"));
        }
    }
    out.write_line("");

    out.write_line("Finish:");
    for line in &brief.commands.finish {
        out.write_line(&format!("  {line}"));
    }
    out.write_line("Give up, if you cannot finish:");
    out.write_line(&format!("  {}", brief.commands.give_up));
}
