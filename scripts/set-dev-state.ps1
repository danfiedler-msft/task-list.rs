<#
.SYNOPSIS
  Write the shared dev lifecycle state that the run-app skill reads before restarting the app.

.DESCRIPTION
  The coder calls this to signal edit/build status so on-demand restarts (the run-app skill) never
  restart the app into a half-written or broken state.

  Signal values:
    building - code is mid-change / building; run-app must NOT restart.
    ready    - coder is done (local build + format:fix + lint + unit tests pass); safe to restart.
    broken   - knowingly broken; run-app must NOT restart.

  The signal is authoritative and honored as written: while it is `building`/`broken` the watch
  (run-app.ps1) polls for `ready` and never restarts — there is no time-based coercion. A leftover
  `building`/`broken` from a prior, now-gone session is cleared to `ready` on the next fresh session
  boot by scripts/session-startup.ps1 (the lifecycle safety valve).

.EXAMPLE
  ./scripts/set-dev-state.ps1 building
  ./scripts/set-dev-state.ps1 ready  -Note "case-export refactor step 2/5"
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory)][ValidateSet('building', 'ready', 'broken')][string]$State,
    [string]$Note = ''
)

$ErrorActionPreference = 'Stop'
$stateFile = Join-Path $PSScriptRoot '.dev-state.json'

[pscustomobject]@{
    state     = $State
    note      = $Note
    updatedAt = (Get-Date).ToString('o')
    updatedBy = $env:USERNAME
} | ConvertTo-Json | Set-Content -Path $stateFile -Encoding utf8

Write-Host "dev-state -> $State $(if($Note){"($Note)"})"
