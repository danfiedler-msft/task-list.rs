<#
.SYNOPSIS
  Load local dev secrets and run the full app: the API watcher (cargo) + the SPA (Vite).

.DESCRIPTION
  Dot-sources scripts/dev-secrets.local.ps1 (git-ignored) so the process inherits the
  required environment variables, then starts both dev servers:
    - the API via `cargo watch -x 'run -p tasklist-api'` (auto-rebuild on change), and
    - the SPA via `npm --prefix web run dev` (Vite, proxies /api/* to the backend).
  Vite serves the SPA in dev; the backend serves `/api/*` on http://localhost:8080.

.EXAMPLE
  ./scripts/dev.ps1
#>
[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$secretsFile = Join-Path $PSScriptRoot 'dev-secrets.local.ps1'

if (-not (Test-Path $secretsFile)) {
    Write-Error "Missing $secretsFile. Copy scripts/dev-secrets.template.ps1 to scripts/dev-secrets.local.ps1 and fill in the values."
}

. $secretsFile

if (-not $env:TASKLIST_ENV) { $env:TASKLIST_ENV = 'local' }

# Prefer `cargo watch` for auto-rebuild; fall back to a plain run if it isn't installed.
$ErrorActionPreference = 'Continue'
cargo watch --version *> $null
$hasCargoWatch = ($LASTEXITCODE -eq 0)
$ErrorActionPreference = 'Stop'
if (-not $hasCargoWatch) {
    Write-Warning "cargo-watch not found (install with 'cargo install cargo-watch') — running the API without auto-rebuild."
}

Push-Location $root
try {
    # Start the Vite dev server as a child process so it shares this script's process tree
    # (run-app.ps1 tears the tree down on restart/stop). Launch via `npm.cmd` (not bare
    # `npm`): on Windows `npm` can resolve to the `npm.ps1` shim, which Start-Process cannot
    # exec ("%1 is not a valid Win32 application"), aborting this script before cargo runs.
    $web = Join-Path $root 'web'
    $vite = Start-Process npm.cmd -ArgumentList 'run', 'dev' -WorkingDirectory $web -NoNewWindow -PassThru

    try {
        # Run the API watcher in the foreground so this script stays alive with it.
        if ($hasCargoWatch) {
            cargo watch -x 'run -p tasklist-api'
        }
        else {
            cargo run -p tasklist-api
        }
    }
    finally {
        if ($vite -and -not $vite.HasExited) {
            # Best-effort teardown of the Vite tree (npm -> node); run-app.ps1 does the
            # authoritative recursive cleanup.
            foreach ($child in (Get-CimInstance Win32_Process -Filter "ParentProcessId = $($vite.Id)" -ErrorAction SilentlyContinue)) {
                Stop-Process -Id $child.ProcessId -Force -ErrorAction SilentlyContinue
            }
            Stop-Process -Id $vite.Id -Force -ErrorAction SilentlyContinue
        }
    }
}
finally {
    Pop-Location
}
