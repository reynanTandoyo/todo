use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=.git/HEAD");
    if let Ok(head) = std::fs::read_to_string(".git/HEAD")
        && let Some(ref_path) = head.trim().strip_prefix("ref: ")
    {
        println!("cargo:rerun-if-changed=.git/{ref_path}");
    }

    let commit = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            std::env::var("GITHUB_SHA")
                .ok()
                .map(|s| s.chars().take(7).collect::<String>())
                .filter(|s| !s.is_empty())
        })
        .unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=TODO_GIT_COMMIT={commit}");
}
