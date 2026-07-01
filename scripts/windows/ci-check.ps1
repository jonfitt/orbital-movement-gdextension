# Local checks mirrored by CI (.github/workflows/rust.yml, .gitlab-ci.yml).
$ErrorActionPreference = "Stop"

$Root = git rev-parse --show-toplevel
if (-not $Root) {
    throw "Not inside a git repository."
}
Set-Location $Root

Write-Host "==> verify VERSION sync"
. "$Root/scripts/windows/version-lib.ps1"
Test-ProjectVersionSync -Root $Root

Write-Host "==> cargo fmt --check"
cargo fmt --all -- --check

Write-Host "==> cargo clippy"
cargo clippy --all-targets -- -D warnings

Write-Host "==> cargo build"
cargo build --verbose

Write-Host "==> cargo test"
cargo test --verbose

Write-Host "All checks passed."
