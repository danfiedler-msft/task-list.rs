---
name: run-app
description: Run the local end-to-end app — ensure it is running, force a clean restart on demand (e.g. when the cargo watcher wedges), or run the 30s liveness watch that keeps it alive automatically.
---

The local app runner. Use it to launch the app, to restart it on demand (for example when `cargo
watch` stays up but the app is down after a failed build), and to run the 30s liveness watch that
keeps it alive. The watch is started at session start (`scripts/session-startup.ps1`).

The app runs the API (`cargo watch -x 'run -p tasklist-api'`, serving `/api/*` on
`http://localhost:8080`) alongside the SPA dev server (`npm --prefix web run dev`, Vite, which
proxies `/api/*` to the backend). Both are launched by `scripts/dev.ps1`.

## Operations

Run from the repository root.

### Ensure the app is running (idempotent)

Launches the app (detached `cargo watch` + Vite via `scripts/dev.ps1`) only if it isn't already
healthy. Run this at session start; it no-ops if the app is already serving.

    ./scripts/run-app.ps1 Ensure

### Restart the app now (on demand)

Kills the running app tree (`pwsh` -> `cargo watch`/Vite -> app, freeing port 8080) and relaunches.
Use this when the app is wedged or you want a clean restart.

    ./scripts/run-app.ps1 Restart

### Stop the app

    ./scripts/run-app.ps1 Stop

### Keep the app alive automatically (background)

Started at session start by `scripts/session-startup.ps1`; you rarely invoke this directly. Runs a
detached 30s loop that probes health and, when the app is down, restarts it only if the coder state is
`ready` (otherwise it polls for `ready`). A short grace pause after each restart avoids killing a
still-booting app.

    ./scripts/run-app.ps1 Watch

## Notes

- Health probe: `http://localhost:8080/api/health`.
- `Ensure`/`Restart` respect the coder's lifecycle signal in `scripts/.dev-state.json` (the
  authoritative state, written by `scripts/set-dev-state.ps1`): when state is `building` or
  `broken` the coder owns the app, so they no-op (the watch polls for `ready`). Pass `-Force` to
  override. The signal is honored as written (no time-based coercion); a leftover `building`/`broken`
  from a prior, now-gone session is cleared to `ready` on the next fresh session boot by
  `scripts/session-startup.ps1`.
- Secrets come from `scripts/dev-secrets.local.ps1` (git-ignored); copy it from
  `scripts/dev-secrets.template.ps1`. Only `TASKLIST_ENV` is consumed by the walking skeleton today.
- First launch takes ~20-60s while cargo does its initial build before `/api/health` responds.
