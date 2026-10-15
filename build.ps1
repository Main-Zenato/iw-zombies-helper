# Builds everything: the interface pages (TypeScript), then both programs.
#   .\build.ps1
# Result: target\release\iw-helper-gui.exe (the interface, the one Steam starts) and
# target\release\iw-helper.exe (the command line). The game must be closed: Windows refuses to
# replace a running program.
$ErrorActionPreference = "Stop"
$root = $PSScriptRoot

function Step([string]$what, [scriptblock]$run) {
    Write-Host "== $what"
    & $run
    if ($LASTEXITCODE -ne 0) { throw "$what failed (exit code $LASTEXITCODE)" }
}

# The pages are built into iw-helper-gui.exe at compile time: they are built first.
Step "Page dependencies" { pnpm --dir "$root\gui" install --frozen-lockfile }
Step "Interface pages" { pnpm --dir "$root\gui" build }
Step "Programs" { cargo build --release --workspace --manifest-path "$root\Cargo.toml" }
Write-Host "Done: $root\target\release\iw-helper-gui.exe"
