//! `todo`: concurrency-safe editor for `TODO.md` task files.

fn main() -> std::process::ExitCode {
    todo::cli::run_cli().into()
}
