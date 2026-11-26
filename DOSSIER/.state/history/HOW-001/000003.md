<!-- dossier-format: 1 -->
# HOW-001 · Run the Linux checks locally in WSL before a push

- kind: howto
- status: draft
- created: 2026-11-26T19:51:12+01:00
- author: reyna
- via: reyna - claude fable 5.2
- software: WSL FedoraLinux-44
- topic: testing
- related: DEC-061, TOOL-002
- history: 2026-11-26T19:51:12+01:00 reyna via reyna - claude fable 5.2: created
- history: 2026-11-26T19:51:43+01:00 reyna via reyna - claude fable 5.2: changed — related DEC-061
- history: 2026-11-26T19:51:44+01:00 reyna via reyna - claude fable 5.2: changed — related TOOL-002

## Purpose

Runs the format check, Clippy and the full test suite of todo on Linux, on this Windows PC, before a push. Linux-only failures otherwise show up only in CI.

## Steps

1. Save the script below as a .sh file with LF line endings, under a folder that WSL sees through /mnt/c.
2. Run it from PowerShell with the branch to check, for example dev: wsl.exe -d FedoraLinux-44 -u reyna -- bash <path-under-/mnt/c> dev
3. Read the result: all three commands must pass. Linux runs 3 fewer tests than Windows, because some tests are Windows-only.
4. After the run, when ~/todo/target is larger than 3 GB, run cargo clean in ~/todo.

The script:

    ref="${1:-dev}"; source ~/.cargo/env
    [ -d ~/todo/.git ] || git clone -q /mnt/c/Project/todo ~/todo
    cd ~/todo && git fetch -q /mnt/c/Project/todo '+refs/heads/*:refs/remotes/win/*' && git checkout -q --force --detach "win/$ref"
    unset TODO_FILE TODO_AGENT TODO_REGISTER
    cargo fmt --all -- --check; cargo clippy --all-targets --all-features -q -- -D warnings
    cargo test --all-features --no-fail-fast

## Settings

The WSL distribution is FedoraLinux-44, with the default user reyna. gcc, git and gawk come from dnf; Rust comes from rustup in ~/.cargo, with clippy and rustfmt. The repository is cloned to ~/todo inside Fedora, and each run fetches the local branches of the Windows repository and checks one out detached.

## Traps

Do not build through /mnt/c: it is slow and the file permissions are wrong. Do not run as root: the read-only-file tests fail, because root may write anyway. The TODO variables must be unset, or the tests read the real task list. The WSL disk image never shrinks, so cap the build folder instead of expecting space back.

## Tested with

Fedora 44 in WSL 2 on Windows 11, stable Rust from rustup, todo 0.8 to 1.0 (first used 2026-11-21).
