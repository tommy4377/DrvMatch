$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Step([string]$name, [scriptblock]$command) {
  Write-Host "`n==> $name" -ForegroundColor Cyan
  & $command
  if ($LASTEXITCODE -ne 0) { throw "$name failed with exit code $LASTEXITCODE" }
}

$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

Step 'Install locked frontend dependencies' { npm ci }
Step 'Frontend presentation tests' { npm test }
Step 'Svelte diagnostics' { npm run check }
Step 'Frontend production build' { npm run build }
Step 'Rust formatting' { cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check }
Step 'Rust clippy' { cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings }
Step 'Rust tests' { cargo test --manifest-path src-tauri/Cargo.toml --all-targets --locked }
Step 'Rust release build' { cargo build --manifest-path src-tauri/Cargo.toml --locked --release }
Step 'npm audit' { npm audit }

if (Get-Command cargo-audit -ErrorAction SilentlyContinue) {
  Step 'RustSec audit' { cargo audit --file src-tauri/Cargo.lock }
} else {
  Write-Warning 'cargo-audit is not installed; install it with `cargo install cargo-audit` before publishing.'
}

Step 'Tauri MSI/NSIS bundle' { npm run tauri build }

$bundleRoot = Join-Path $root 'src-tauri\target\release\bundle'
if (Test-Path $bundleRoot) {
  $artifacts = Get-ChildItem $bundleRoot -Recurse -File | Where-Object { $_.Extension -in '.msi', '.exe' }
  if ($artifacts.Count -eq 0) { throw 'Tauri build completed but no MSI/EXE bundle was found.' }
  Write-Host "`nRelease artifacts:" -ForegroundColor Green
  foreach ($artifact in $artifacts) {
    $hash = Get-FileHash $artifact.FullName -Algorithm SHA256
    Write-Host ("{0}  {1}" -f $hash.Hash, $artifact.FullName)
  }
} else {
  throw "Bundle directory was not created: $bundleRoot"
}

Write-Host "`nDrvMatch 1.0.0 automated release checks completed." -ForegroundColor Green
Write-Host 'Complete the manual Windows matrix in docs/RELEASE_VALIDATION.md before publishing.'
