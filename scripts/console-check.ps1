<#
.SYNOPSIS
Prints sample `todo` output containing middot (`·`), em-dash (`—`), and Japanese text
for manual verification in PowerShell 5, PowerShell 7, cmd, and Windows Terminal (T-006 / R-OUT-6).
#>
param(
    [string]$TodoExe = (Join-Path $PSScriptRoot "..\target\release\todo.exe")
)

if (-not (Test-Path $TodoExe)) {
    $debugExe = Join-Path $PSScriptRoot "..\target\debug\todo.exe"
    if (Test-Path $debugExe) {
        $TodoExe = $debugExe
    } else {
        Write-Error "todo.exe not found. Build with 'cargo build --release' or 'cargo build' first."
        exit 1
    }
}

$tmpDir = Join-Path ([System.IO.Path]::GetTempPath()) ("todo-console-check-" + [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $tmpDir | Out-Null

try {
    $todoPath = Join-Path $tmpDir "TODO.md"
    $sample = @"
<!-- TEST FIXTURE: invented data, not a real task list -->
## Now

### T-001 · 日本語タスク — コンソール表示確認
- status: open
- owner: —
- priority: P1 · type: test · area: cli
- note (2026-11-15, reviewer): 文字化けしないことを確認 — UTF-8
"@
    [System.IO.File]::WriteAllText($todoPath, $sample, [System.Text.UTF8Encoding]::new($false))

    Write-Host "=== todo list ==="
    & $TodoExe --file $todoPath list --wide
    Write-Host ""
    Write-Host "=== todo show T-001 ==="
    & $TodoExe --file $todoPath show T-001
    Write-Host ""
    Write-Host "Check that '·', '—', and '日本語タスク' render cleanly above."
}
finally {
    Remove-Item -Path $tmpDir -Recurse -Force -ErrorAction SilentlyContinue
}
