<#
.SYNOPSIS
  Run the local end-to-end app. Backs the `run-app` skill (.github/skills/run-app.md).

.DESCRIPTION
  App runner for the local end-to-end app. Launches the app via dev.ps1 (cargo watch + Vite,
  detached) and can force a clean restart by killing the app process tree (freeing the API
  port) and relaunching. Use Restart when the watcher stays up but the app is down (e.g. after
  a failed build). The Watch action is a 30s liveness loop (started at session start by
  scripts/session-startup.ps1) that keeps the app alive automatically. Ensure/Restart/Watch
  respect the coder's lifecycle signal in scripts/.dev-state.json (the authoritative state,
  written by scripts/set-dev-state.ps1): when state is `building`/`broken` the coder owns the
  app, so a restart no-ops (Watch keeps polling for `ready`) unless -Force is passed. The signal
  is honored as written and the watch never overrides it on its own (no time-based coercion); a
  leftover `building`/`broken` from a prior, now-gone session is cleared to `ready` on the next
  fresh session boot by scripts/session-startup.ps1.

.PARAMETER Action
  Ensure  (default) - launch the app only if it isn't already healthy.
  Restart           - kill the running app tree and relaunch.
  Stop              - kill the running app tree, don't relaunch.
  Watch             - 30s liveness loop: restart when down and state is ready, else poll for ready.
                      Run detached via scripts/session-startup.ps1.

.PARAMETER Force
  Ignore the building/broken state signal and act anyway.

.EXAMPLE
  ./scripts/run-app.ps1            # ensure running
  ./scripts/run-app.ps1 Restart    # force a clean restart
#>
[CmdletBinding()]
param(
    [ValidateSet('Ensure', 'Restart', 'Stop', 'Watch')][string]$Action = 'Ensure',
    [switch]$Force,
    [string]$Url = 'http://localhost:8080/api/health',
    [int]$Port = 8080,
    [int]$IntervalSeconds = 30,
    [int]$GraceSeconds = 60,
    [int]$MaxRestarts = 3
)

$ErrorActionPreference = 'Stop'
$devScript = Join-Path $PSScriptRoot 'dev.ps1'
$stateFile = Join-Path $PSScriptRoot '.dev-state.json'
$secretsFile = Join-Path $PSScriptRoot 'dev-secrets.local.ps1'
$appPidFile = Join-Path $PSScriptRoot '.dev-app.pid'
$watchPidFile = Join-Path $PSScriptRoot '.liveness-watch.pid'
$watchLogFile = Join-Path $PSScriptRoot 'liveness-watch.log'

function Test-AppHealthy {
    try {
        $resp = Invoke-WebRequest -Uri $Url -TimeoutSec 8 -UseBasicParsing
        return $resp.StatusCode -ge 200 -and $resp.StatusCode -lt 400
    }
    catch { return $false }
}

function Get-DevState {
    if (-not (Test-Path $stateFile)) { return 'ready' }   # no signal yet = assume safe
    try {
        $json = Get-Content $stateFile -Raw | ConvertFrom-Json
        $state = $json.state
        if ($state -notin @('ready', 'building', 'broken')) { return 'broken' }
        # Honor the authoritative signal as written: `building`/`broken` means the coder owns the
        # app, so the watch must POLL for `ready` (never restart). A session that ends mid-build
        # must reset the signal (set-dev-state ready/broken) — the watch will not override it.
        return $state
    }
    catch { return 'broken' }
}

function Test-StateAllows([string]$What) {
    $state = Get-DevState
    if ($state -in @('building', 'broken') -and -not $Force) {
        Write-Host "dev-state is '$state' — the coder owns the app; skipping $What. Pass -Force to override."
        return $false
    }
    return $true
}

function Get-DescendantProcessId([int]$ParentId) {
    $ids = @()
    foreach ($child in (Get-CimInstance Win32_Process -Filter "ParentProcessId = $ParentId" -ErrorAction SilentlyContinue)) {
        $ids += [int]$child.ProcessId
        $ids += Get-DescendantProcessId ([int]$child.ProcessId)
    }
    return $ids
}

function Stop-Tree([int]$RootId) {
    if ($RootId -le 0) { return }
    $ids = @($RootId) + (Get-DescendantProcessId $RootId) | Sort-Object -Unique
    foreach ($id in $ids) { Stop-Process -Id $id -Force -ErrorAction SilentlyContinue }
}

# Walk up through the cargo/app chain only (tasklist-api <- cargo-watch/cargo), stopping before
# any shell, so freeing the port from an orphaned launch never climbs into pwsh/agent processes.
function Get-AppChainRoot([int]$ProcessId) {
    $rootId = $ProcessId
    $cursor = Get-CimInstance Win32_Process -Filter "ProcessId = $ProcessId" -ErrorAction SilentlyContinue
    while ($cursor -and $cursor.ParentProcessId -gt 0) {
        $parent = Get-CimInstance Win32_Process -Filter "ProcessId = $($cursor.ParentProcessId)" -ErrorAction SilentlyContinue
        if ($parent -and $parent.Name -match '^(cargo|cargo-watch|tasklist-api)') {
            $rootId = [int]$parent.ProcessId
            $cursor = $parent
        }
        else { break }
    }
    return $rootId
}

function Stop-App {
    # 1) Kill the recorded launch tree (pwsh -> dev.ps1 -> cargo watch/Vite -> app/node).
    if (Test-Path $appPidFile) {
        $recorded = (Get-Content $appPidFile -Raw).Trim()
        if ($recorded -match '^\d+$') { Stop-Tree ([int]$recorded) }
        Remove-Item $appPidFile -ErrorAction SilentlyContinue
    }
    # 2) Safety net: free the API port from any orphaned cargo/app tree (e.g. a prior session).
    foreach ($conn in (Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue)) {
        Stop-Tree (Get-AppChainRoot ([int]$conn.OwningProcess))
    }
}

function Start-App {
    if (-not (Test-Path $secretsFile)) {
        Write-Warning "scripts/dev-secrets.local.ps1 missing — copy dev-secrets.template.ps1 and fill it in. Launching anyway; the app may fail to start."
    }
    $proc = Start-Process pwsh -ArgumentList '-NoProfile', '-File', $devScript -WindowStyle Normal -PassThru
    Set-Content -Path $appPidFile -Value $proc.Id -Encoding ascii
    Write-Host "app launched (pid=$($proc.Id)); $Url may take ~20-60s to come up (first cargo build)."
}

function Restart-App {
    Write-Host 'stopping app tree...'
    Stop-App
    Start-Sleep -Seconds 2
    Start-App
}

function Test-RecordedScriptRunning([string]$PidPath, [string]$ScriptPath) {
    if (-not (Test-Path $PidPath)) { return $false }
    $existing = (Get-Content $PidPath -Raw).Trim()
    if (-not $existing -or $existing -notmatch '^\d+$') { return $false }
    $process = Get-CimInstance Win32_Process -Filter "ProcessId = $existing" -ErrorAction SilentlyContinue
    if (-not $process -or -not $process.CommandLine) { return $false }
    $leaf = Split-Path -Leaf $ScriptPath
    return $process.CommandLine.IndexOf($leaf, [StringComparison]::OrdinalIgnoreCase) -ge 0
}

switch ($Action) {
    'Ensure' {
        if (Test-AppHealthy) { Write-Host "app already healthy at $Url."; break }
        if (-not (Test-StateAllows 'launch')) { break }
        Start-App
    }
    'Restart' {
        if (-not (Test-StateAllows 'restart')) { break }
        Restart-App
    }
    'Stop' {
        Stop-App
        Write-Host 'app stopped.'
    }
    'Watch' {
        if (Test-Path $watchPidFile) {
            $existing = (Get-Content $watchPidFile -Raw).Trim()
            if ($existing -and (Test-RecordedScriptRunning $watchPidFile $PSCommandPath)) {
                Write-Host "liveness watch already running (pid=$existing); exiting."
                break
            }
        }
        Set-Content -Path $watchPidFile -Value $PID -Encoding ascii
        function Write-WatchLog([string]$Message) {
            $line = "[$((Get-Date).ToString('o'))] $Message"
            Add-Content -Path $watchLogFile -Value $line
            Write-Host $line
        }
        Write-WatchLog "liveness watch started (pid=$PID, interval=${IntervalSeconds}s, grace=${GraceSeconds}s, url=$Url)"
        $consecutiveRestarts = 0
        $flapPaused = $false
        while ($true) {
            if (Test-AppHealthy) {
                if ($flapPaused) { Write-WatchLog 'recovered: app healthy again; resuming auto-restart' }
                $consecutiveRestarts = 0
                $flapPaused = $false
                Write-WatchLog 'healthy'
                Start-Sleep -Seconds $IntervalSeconds
                continue
            }
            $state = Get-DevState
            if ($state -ne 'ready') {
                # Coder owns the app; not a flap. Reset so a fresh ready cycle gets full attempts.
                $consecutiveRestarts = 0
                $flapPaused = $false
                Write-WatchLog "DOWN + state=$state -> waiting (coder owns the app; polling for ready)"
                Start-Sleep -Seconds $IntervalSeconds
                continue
            }
            if ($consecutiveRestarts -ge $MaxRestarts) {
                if (-not $flapPaused) {
                    $flapPaused = $true
                    Write-WatchLog "INCIDENT: app still DOWN after $MaxRestarts restarts -> auto-restart PAUSED. Check secrets/build (cargo output); will resume when the app is healthy or dev-state leaves 'ready'."
                }
                Start-Sleep -Seconds $IntervalSeconds
                continue
            }
            $consecutiveRestarts++
            Write-WatchLog "DOWN + state=ready -> clean restart (attempt $consecutiveRestarts/$MaxRestarts)"
            try { Restart-App } catch { Write-WatchLog "restart error: $($_.Exception.Message)" }
            Write-WatchLog "restart done; grace ${GraceSeconds}s before re-probing"
            Start-Sleep -Seconds $GraceSeconds
        }
    }
}
