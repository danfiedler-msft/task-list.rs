# task-list.rs — design & hard rules

> **The single authoritative design + rules for this repo — read this first, every session.** It is both
> the terse project map (layout, dependency flow, hard rules) and the full narrative (request flow,
> persistence, patterns, offline, DevOps, risks). The feature plan lives in `docs/features/`. Concrete
> dependency and version choices live in `Cargo.toml` and `web/package.json` — this doc describes **roles
> and seams**, not crate/version pins.

**task-list.rs** is a **Rust (axum) backend + a React + TypeScript SPA**, laid out as a Clean-Architecture
**Cargo workspace** and deployed as a single container image on **Azure Container Apps** that hosts the API
and serves the SPA — a **mobile-first, offline-first, installable PWA**. It is **multi-user**: **Auth0
(OIDC, Google social connection)** authenticates via a **cookie-based server-side session (BFF)** and all
data is scoped per user. Persistence is **Azure Cosmos DB** (system of record) + **Azure Blob Storage**
(task images), reached in prod via **managed identity** (no keys); secrets live in **Azure Key Vault**.
Telemetry goes to **Azure Application Insights**; infra is **Bicep**; CI/CD is **GitHub Actions**, single
**Production**. It exists to be a full-featured clean-architecture starter that lights up the Azure stack
end-to-end.

## Stack

- **Backend:** Rust, **axum** HTTP, Clean-Architecture **Cargo workspace**.
- **Frontend:** **React + TypeScript + Vite** SPA (`web/`) — mobile-first, offline-first, installable PWA.
- **Persistence:** Azure Cosmos DB (behind the `TaskRepository` port) + Azure Blob Storage (behind `BlobStore`).
- **Auth:** Auth0 (Google social connection), cookie-session BFF — the server holds the session; the SPA stores no tokens.
- **Telemetry:** Azure Application Insights behind a single `init_telemetry()` seam.
- **Host / CI:** one container on Azure Container Apps; **GitHub Actions** CI/CD, single Production.
- **API contract:** the API derives an OpenAPI doc → a typed TypeScript client is generated for the SPA.

The core libraries are named above; **every other dependency is described by role only**. Concrete crates +
versions: see `Cargo.toml` and `web/package.json`.

## Directory layout & dependency flow

Cargo workspace; dependency flow is strict: **`domain ← application ← infrastructure / api`**.

```
task-list.rs/            Cargo workspace (Cargo.toml [workspace] — members + shared [workspace.dependencies])
├─ domain/               Entities, value-object newtypes, enums, domain errors. No inbound deps
│                        (only small leaf utility crates: ids, timestamps, typed errors). Invariants at construction.
├─ application/          Use-cases (async fns), PORT traits, DTOs, validation, app errors. Depends only on domain.
├─ infrastructure/       Port ADAPTERS (in-memory now; Cosmos / Blob / telemetry later) + config loader. Deps: application, domain.
├─ api/                  axum host: routers, handlers, auth middleware, OpenAPI, RFC7807 mapping, SPA serving,
│                        composition root (AppState). Depends on application + infrastructure + domain.
├─ web/                  React + TS + Vite SPA (installable PWA). src/{pages,components,contexts,api,hooks,offline}.
├─ .github/              workflows/ (ci.yml + deploy.yml), skills/*, agents/*, copilot-instructions.md.
├─ .arm/                 Bicep IaC (main.bicep + prod.main.bicepparam + bicepconfig.json).  [IaC dir ruled `.arm/`]
├─ scripts/              Local-dev PowerShell helpers (dev / run-app / session-startup / secrets / dev-state).
├─ docs/                 design.md (this), features/.
└─ Dockerfile            Multi-stage: web build + cargo build → runtime image serving API + SPA on one port.
```

Package names: `tasklist-{domain,application,infrastructure,api}` (`api` is a lib + binary). Each library
crate keeps unit tests in-crate (`#[cfg(test)]`); cross-component tests live in each crate's `tests/`.

## Hard rules

1. **Clean architecture, strict.** Respect the dependency flow above. `domain` has no framework/Azure deps.
   Adapters never appear outside `infrastructure`; wiring only in `api`'s composition root. The OpenAPI
   generator's `ToSchema` derives **are permitted on `application` DTOs** — passive compile-time schema
   metadata, no web-framework coupling; this does **not** violate `domain ← application ← api`.
2. **Zero-warning policy (non-negotiable).** `cargo fmt --check`, `cargo clippy --all-targets
   --all-features -- -D warnings`, `cargo test`, `cargo build --release` all clean; web runs `lint`,
   `typecheck`, `test:ci`, `build` clean. Enforced in CI (not `#![deny(warnings)]`).
3. **Generated files are never hand-edited:** `web/src/ApiClient.generated.ts` (and `web/openapi.json`).
   Regenerate from the OpenAPI doc; the CI drift check gates it.
4. **Parse, don't validate.** Domain newtypes (`TaskId` / `UserId` / `Title` …) enforce invariants at
   construction; invalid states are unrepresentable.
5. **Errors → RFC7807.** Library code returns typed errors; `api` maps them to `application/problem+json`
   (400/401/403/404/409/500). Never leak internal detail to clients.
6. **Config & secrets.** Non-secret config via env (`TASKLIST_ENV=local|cloud` seam). Secrets only via Key
   Vault in cloud / the git-ignored `scripts/dev-secrets.local.ps1` locally — **never committed**.
7. **Same-origin web.** The SPA calls `/api/*`; Vite dev-proxies to the backend. No cross-origin access in prod (same-origin SPA); the CORS layer is locked to same-origin.
8. **Health contract (load-bearing).** `GET /api/health` → `200 {status, time}`; `time` is sourced through
   the `Clock` port = the DIP proof on the health path. The liveness probe checks only the 2xx status code,
   so the body is an **evolvable superset**. (Reconciles the bootstrap.md `{status:"ok"}` shorthand.)
9. **Testing.** domain: exhaustive newtype/status unit tests. application: use-case tests vs in-memory
   fakes. api: HTTP tests via axum's test client. Integration (emulators) in later tasks. Tag unit vs
   integration so CI runs them separately.
10. **Lane discipline.** Work on `vibe/<feature>`; never touch `main`. Agents never commit/push/deploy.
11. **Local-dev contract is fixed.** `set-dev-state.ps1` + `.dev-state.json` and the `GET /api/health`
    probe are load-bearing for the liveness watch — do not change their shape.

## Backend design

### Crates & responsibilities (`domain ← application ← infrastructure / api`)

| Crate | Owns | Notes |
|-------|------|-------|
| **domain** | `Task` entity; newtypes (`TaskId`, `UserId`, `Title`, `Description`, `ImageRef`, `ETag`); `TaskStatus`; domain errors | No inbound deps; only small leaf utility crates. Invariants at construction (parse, don't validate). |
| **application** | Use-case fns (`tasks::{create,list,get,update,delete,set_image}`), **port traits** (`TaskRepository`, `BlobStore`, `UserContext`, `Clock`), request/response DTOs, DTO validation, app errors | Depends only on `domain`. No Azure types leak in — ports are pure Rust traits. The OpenAPI `ToSchema` derive is allowed here (rule 1). |
| **infrastructure** | `CosmosTaskRepository`, `AzureBlobStore`, telemetry init (`init_telemetry()`), config loader, `SystemClock`; in-memory fakes for the skeleton | The only crate that references Azure SDKs. Adapter swaps are contained here (Cosmos risk). |
| **api** | axum routers + handlers, HTTP middleware (trace / same-origin-locked CORS / session / auth / CSRF / rate-limit), OIDC login/callback, OpenAPI + `/api/health`, RFC7807 mapping, static-file + SPA fallback, composition root (`main.rs`) | Wires ports → adapters via constructor injection (`AppState`). Depends on application + infrastructure + domain. |

### Domain model

```rust
// domain/src/task.rs  (illustrative — invariants enforced by newtype constructors)
pub struct Task {
    pub id:          TaskId,             // time-ordered UUIDv7
    pub owner:       UserId,             // Auth0 subject → Cosmos partition key
    pub title:       Title,              // newtype: 1..=200 chars, non-blank
    pub description: Option<Description>,
    pub status:      TaskStatus,         // Todo | InProgress | Done
    pub due_date:    Option<Date>,
    pub image:       Option<ImageRef>,   // blob key; the bytes live in Blob Storage
    pub created_at:  Timestamp,
    pub updated_at:  Timestamp,
    pub version:     ETag,               // Cosmos _etag → optimistic concurrency
}

pub enum TaskStatus { Todo, InProgress, Done }
```

- **Time-ordered UUIDv7** ids are naturally sortable (good Cosmos item ids) and safe to mint client-side offline.
- The **`image`** field holds only a blob **reference** (`{ownerId}/{taskId}/{imageId}`); the bytes never
  touch Cosmos — the concrete demonstration of Blob Storage flowing through the layers.
- `version` mirrors the Cosmos server-managed `_etag`; the client only echoes it on update.

### Request flow

```
Browser SPA (SameSite session cookie, same-origin)
  → HTTPS → axum Router
  → trace → CORS → session load
  → auth middleware: require an authenticated user; build UserContext(owner) from session claims
  → CSRF check (mutations only)
  → handler: deserialize + validate the DTO
  → use-case (application::tasks::*) invoked with UserContext(owner)
  → ports: TaskRepository (Cosmos) | BlobStore (Blob)
  → CosmosTaskRepository → Azure Cosmos DB (db=tasklist, container=tasks, PK=/ownerId)
    AzureBlobStore       → Azure Blob Storage (container=task-images)
Errors → AppError → IntoResponse → RFC7807 application/problem+json
```

**Auth challenge flow (BFF).** `GET /auth/login` → OIDC authorization-code + PKCE redirect (scopes
`openid profile email`, Google connection) → the user authenticates at Auth0 → `GET /auth/callback`
validates the ID token, upserts the user, creates a server-side session, sets the cookie → SPA.
`POST /auth/logout` clears the session and redirects to Auth0 logout.

### Persistence model

**Cosmos DB (system of record).**
- One database `tasklist`, one container `tasks`, **partition key `/ownerId`** — the per-user partition
  seam. Every read/write is scoped to the caller's partition; cross-user reads are not first-class (by design).
- Point reads by `(ownerId, id)`; list = a single-partition query.
- **Optimistic concurrency:** the Cosmos `_etag` + `If-Match` on replace → 412 → surfaced as **409 Conflict**.
- **Serialization:** JSON docs (`id`, `ownerId`, fields, server-managed `_etag` / `_ts`); the repository
  adapter is explicit (no change tracking).
- **Auth to Cosmos:** managed identity in Azure (keyless, data-plane RBAC); the well-known emulator key locally.

**Blob Storage (task images).**
- Container `task-images`, blob key `{ownerId}/{taskId}/{imageId}` (owner-prefixed for isolation).
- `set_image` → `BlobStore.put(key, bytes, content_type)` → store the key in `task.image`; download streams
  `BlobStore.get(key)` (or a short-lived read URL). Public blob access **off**.

## Application-wide patterns

- **Use-cases, not a mediator.** Every operation is an `async fn` in `application::tasks::*` taking a
  `UserContext` + validated input and the ports it needs. Simple, explicit, testable — no mediator-style dispatch.
- **Ports & adapters.** `TaskRepository`, `BlobStore`, `UserContext`, `Clock` are traits in `application`;
  `infrastructure` supplies the Azure-backed impls; tests supply in-memory fakes. This seam contains the
  preview-SDK risk. `api` is the composition root: it builds `AppState { repo, clock, ... }` with
  `Arc<dyn Port>` and injects it; handlers depend on the traits, never on a concrete adapter.
- **Error handling.** Libraries return typed errors; `api` maps them to **RFC7807
  `application/problem+json`**: 400 (validation), 401 (unauthenticated), 403 (ownership), 404 (not found /
  not owned — don't leak existence), 409 (etag conflict), 500 (unexpected).
- **Validation, two layers.** Structural DTO validation at the edge; **newtype parsing** in `domain` so
  invariants (e.g. `Title` length) are unrepresentable-if-invalid.
- **Authorization = per-user ownership.** No roles (YAGNI). `UserContext` yields the caller's `UserId`;
  repository access is partition-scoped to that user, and a fetched task whose `owner` differs returns 404.
  Ownership is the entire authorization model.
- **Auth / session (locked in).** Cookie-based server-side session + Auth0 OIDC (auth-code + PKCE, **Google
  social connection**). On callback the app validates the ID token and upserts a `User` keyed by the Auth0
  `sub`. `SameSite=Lax` + `Secure` + `HttpOnly` cookie; a CSRF token for mutations. Default store =
  **encrypted cookie-stored** (stateless — survives Container Apps scale-to-zero / multi-replica via a Key
  Vault signing key); a Cosmos-backed store is the scale-out alternative. The session lifetime governs the
  offline window. A SPA-driven bearer-JWT-in-browser approach was **rejected** — the SPA stores no tokens.
- **Config & secrets.** Non-secret config via a layered config loader (defaults → file → Container Apps
  env). **Secrets live only in Azure Key Vault** — the Auth0 client secret and session signing key are Key
  Vault entries surfaced as **Container Apps secrets (Key Vault references)**, read at runtime under the
  app's **managed identity**; **no secrets in env or repo**. Cosmos + Blob data-plane access is **keyless**
  via managed identity in prod. Locally, only emulator credentials are used; `dev-secrets.local.ps1` is git-ignored.
- **Telemetry.** Spans/events → OTel → App Insights via one `init_telemetry()` in `infrastructure` that owns
  the exporter, so the sink is swappable (see risks).
- **Typed client generation.** The API derives the OpenAPI doc from handlers/DTOs; a build/CI step
  regenerates `web/src/ApiClient.generated.ts`. Generated files are never hand-edited; a **CI drift check**
  fails if the committed client differs from a fresh generation.
- **Zero-warning policy.** `cargo fmt --check` + `cargo clippy --all-targets --all-features -D warnings`
  gate CI; the web side runs lint/type-check clean.
- **Hardening.** Rate limiting, upload body-size limits, the CORS layer locked to same-origin in prod, CSRF for
  cookie-auth mutations, HTTPS-only.
- **Testing.** domain: exhaustive unit tests on newtypes + status transitions. application: use-case unit
  tests against in-memory port fakes. infrastructure: `tests/` integration against the Cosmos **emulator** +
  **Azurite**. api: `tests/` HTTP tests via axum's test client. Tag unit vs integration so CI runs them separately.

## Frontend design (`web/`)

A React + TypeScript + Vite SPA, same-origin with the API, delivered as a **mobile-first, offline-first,
installable PWA** — the stated product identity, so the offline / PWA layer is a first-class concern
(below). The rest of the UI stays deliberately lean; the starter's depth is in the backend / Azure stack +
the offline data layer.

| Folder / file | Responsibility |
|---------------|----------------|
| `pages/` | Routed screens (`Tasks`, `Task` detail/edit, `Login`, `AuthCallback`, `NotFound`). |
| `components/` | `feature/` (task UI), `layouts/` (chrome), `primitives/` (base UI). |
| `contexts/` | `AuthProvider` (current-user/session via `/api/me`), `Theme`. |
| `api/` | `ApiClient.generated.ts` (from OpenAPI — never hand-edited) + a hand-written base wrapper. |
| `hooks/` | `useHealthStatus`, data hooks over the generated client. |
| `offline/` | Owner-scoped offline engine: schema, mirror/outbox/blob primitives + teardown, health-driven sync (below). |
| `pwa.tsx` | Service-worker registration + update toast; **Reload gated on an empty outbox**. |
| `App.tsx`, `main.tsx`, `ErrorBoundary.tsx` | Root app, bootstrap, error boundary. |

**Auth pattern (locked in).** Cookie-based server-side session (BFF) with Auth0 + Google: the backend runs
the OIDC auth-code + PKCE dance and holds the session; the SPA is a same-origin cookie client and stores
**no tokens** in the browser. A SPA-driven bearer-JWT-in-browser approach was **rejected** (the SPA stores
no tokens).

## Offline-first & PWA

The stated product identity is a **mobile-first, offline-first, installable PWA**, making offline a
first-class architectural concern. A per-user (owner-partitioned) model is **well-suited** to an offline
mirror: each user's data is a single Cosmos partition (`/ownerId = me`), so the mirror is a 1:1 replica of
one partition (nothing shared), and the `_etag` is a ready-made optimistic-concurrency / conflict primitive.
Two bounded tensions remain (designed below): the offline **auth window** and offline **id minting**.

**Two independent layers (must not be conflated).**
1. **App-shell PWA (service worker).** Precaches the static built shell + a navigation-fallback to
   `index.html`. It **never caches `/api/*` or `/auth/*`**. Its only job: the app boots with no network.
2. **Offline data engine (app-managed, IndexedDB).** All offline *data* lives in an owner-scoped IndexedDB
   mirror + outbox that the app reads/writes/syncs explicitly. **No runtime caching of API responses**
   (which would give stale reads, cache auth responses, and bypass etag handling).

**Client mirror (IndexedDB), owner-scoped.** One DB, four stores — `tasks` (read mirror incl.
`owner`/`status`/`version`/`updatedAt`), `outbox` (pending writes, one latest per task, coalesced by PK,
carrying the verbatim command + `heldEtag`), `blobs` (cached images), `meta` (cached `/api/me` identity +
`syncedAt`). Every record carries `owner`; reads are owner-filtered; content is cleared on
**logout / owner-change** (never on same-user re-auth). Client owner-scoping is convenience / defense-in-depth
— the server still re-derives the owner from the session and enforces the per-user partition + 404-on-not-owned
on every sync call.

**Read / write flow.** Offline reads come from the mirror with pending `outbox` edits overlaid
(re-editable). Create mints a **client-side UUIDv7 `TaskId`** (used as the Cosmos doc id → idempotent
create) with no `heldEtag`. Update writes through + enqueues an `upsert` carrying the held `_etag`. Delete
removes from the mirror + enqueues a `delete` (404-on-replay = success). Images captured as blobs; embedded
as base64 into the command only at flush. Online, the same use-cases run immediately (no outbox).

**Sync on reconnect (health-driven).** Driven by the `useHealthStatus` signal, not naked
`navigator.onLine`.
- **Pull:** when healthy, run the single-partition list query → refresh the `tasks` mirror + download
  images → stamp `syncedAt` (idempotent).
- **Push:** drain the owner-scoped outbox, replaying each command **verbatim**: `upsert` → `PUT`/upsert with
  `If-Match: heldEtag` (create → none); `200/201` → delete the row + re-pull that task (fresh etag);
  `412` → **409** → conflict (below); `401` → session expired offline → prompt re-login on reconnect,
  **keep the row** (durable); other → mark `error`, keep the row. Flush-time guard: a command still
  referencing an evicted local blob is **not** sent (prevents a corrupt replay).
- **Triggers:** debounced flush on reconnect, an explicit **Sync** affordance, first-load pull. The SW
  update-toast's Reload is **gated on an empty outbox** so a new build never clobbers pending writes.

**Conflict resolution — last-writer-wins, owner-scoped.** The `_etag` / `If-Match` upsert **is** the
concurrency mechanism; because data is single-owner, the only reachable conflict is the same user editing
the same task from two devices. Replay carries `heldEtag`: match → applied + re-pull; mismatch → 412 → 409 →
mark `conflict`, re-pull for visibility, then an **explicit retry** re-stamps the fresh `_etag` and replays
(this device's edit wins). **No CRDT, no field-merge** (that needs a patch API we don't have — YAGNI); a
merge / choose-a-version UI is out of scope.

**Offline auth / session.** Reads and writes need no session (local mirror + outbox). Cached identity in
`meta` lets the shell render "signed in as X" offline and supplies the scoping `owner`; it is
**single-user-per-device** (a second user booting offline could see the prior cached identity until an
online login repopulates it — accepted, mitigated by clear-on-logout). The one real tension: the BFF session
cookie can **expire during a long offline window** — sync (not reads) needs a live cookie, so an expired one
401s the drain → prompt re-login on reconnect, then flush; because the outbox is durable, nothing is lost.
Offline windows are typically short, so a **rolling multi-day session** comfortably outlives them; an
extended session is an available lever. The security boundary stays server-side.

**Installable PWA (service worker + manifest).**
- Precache the built shell (`js,css,html,svg,png,ico,woff2`) + `navigateFallback: '/index.html'`,
  `cleanupOutdatedCaches: true`.
- **`navigateFallbackDenylist`: `/^\/api\//`, `/^\/auth\//`, `/^\/swagger/`** — critical: the SW must
  **never** serve cached `index.html` on the Auth0 OIDC callback (`/auth/callback`) or it hijacks the login
  and loops installed/returning users forever. This is a known **service-worker/OIDC-callback hijack
  landmine** — keep the denylist.
- **No runtime caching of `/api`** — offline data is the IndexedDB engine's job.
- **`registerType: 'prompt'`** → a non-disruptive "Update available — Reload" toast; **Reload gated on an
  empty outbox**.
- **Manifest** (single source of truth in `vite.config.ts`): complete
  `id`/`name`/`short_name`/`description`/`lang`/`dir`/`start_url`/`scope`/`display: standalone`/`theme_color`/
  `background_color`, and icons with **both `any` and `maskable`** purposes → installable on mobile + desktop.
- **Mobile-first UI:** responsive single-column list, large touch targets, no desktop-only chrome.

**Slice-ability.** Offline is naturally incremental — (1) installable app-shell PWA; (2) read mirror +
"Sync my tasks"; (3) offline write + outbox + flush + conflict handling. Each is an independently deployable slice.

## DevOps

### CI/CD pipeline (GitHub Actions)

```
ci.yml   (push / PR to main)
├─ backend : cargo fmt --check → clippy -D warnings → cargo test (unit) → cargo test (integration, emulator+Azurite) → cargo build --release
├─ web     : npm ci → lint → typecheck → test → vite build
├─ drift   : regenerate OpenAPI + the typed client → git diff --exit-code (fails on a stale generated client)
├─ iac     : bicep build/lint + az deployment group validate (prod params)
└─ image   : docker build (multi-stage: cargo build → runtime image with SPA assets)

deploy.yml (push to main, or manual)  ── Production only, GitHub Environment approval ──
└─ build image → push (ACR/GHCR) → az deployment group create (main.bicep + prod.main.bicepparam)
   → roll a new Container Apps revision → smoke-probe /api/health
```

Single **Production** stage. Integration tests run against emulator services in CI (no live Azure).

### Infrastructure (Bicep — `.arm/main.bicep`, single Production)

| Resource | Config highlights |
|----------|-------------------|
| **Cosmos DB** (Core/SQL) | Serverless, Session consistency; db `tasklist`, container `tasks` PK `/ownerId`; periodic backup; **data-plane RBAC** (Built-in Data Contributor) to the app's managed identity — no keys. |
| **Storage Account** | StandardV2, container `task-images`; public blob access **off**, soft-delete on; **Storage Blob Data Contributor** to the app identity. |
| **Azure Container Apps** | Hosts the single container (API + SPA); external HTTPS ingress, min-replicas 0 (**scale-to-zero**) → N; **system-assigned managed identity**; managed environment + Log Analytics. |
| **App Insights** (workspace-based) | Telemetry sink; connection string injected as an app setting. |
| **Azure Key Vault** | Auth0 client secret + session signing key, exposed as **Container Apps secrets via Key Vault references**, read under managed identity (RBAC: Key Vault Secrets User). |
| **Params** | Auth0 `domain`/`clientId`/`audience` (plain); `clientSecret` + signing key are **Key Vault references** (not `@secure()` deploy params); env / version / timestamp. |

The IaC directory is **`.arm/`** (ruled).

### Config & secrets
- Runtime config is set on the Container App by Bicep at deploy: `Auth0__*` (non-secret), the App Insights
  connection string, Cosmos + Storage **endpoints** (keyless data access via managed identity). The Auth0
  client secret + session signing key are **Key Vault references** surfaced as Container Apps secrets — never
  plaintext env or `@secure()` deploy params.
- CI/deploy secrets are **GitHub Actions secrets** (Azure OIDC federated login — no stored password).
- The generated OpenAPI spec + TS client are committed artifacts; the drift job keeps them honest.

### Local dev (a hard requirement)
The `scripts/` keep the same liveness / health contract so the agent loop is unchanged:
- **`session-startup.ps1`** — starts the detached liveness watch (and clears a stale `building`/`broken` →
  `ready` on a fresh session boot).
- **`run-app.ps1`** — Ensure / Restart / Stop / Watch lifecycle keyed off the `.dev-state.json`
  building/ready/broken signal; the health probe stays `GET /api/health`.
- **`dev.ps1`** — dot-source `dev-secrets.local.ps1`, then run the API + SPA watchers (`cargo watch` + Vite);
  Vite proxies `/api/*` to the backend so the SPA is same-origin in dev.
- **`dev-secrets.template.ps1`** — env vars (`AUTH0__*`, `COSMOS__ENDPOINT`, `STORAGE__CONNECTION`,
  `APPLICATIONINSIGHTS_CONNECTION_STRING`, `TASKLIST_ENV=local|cloud`). The Auth0 client secret is Key Vault
  in cloud; locally it lives only in the git-ignored `dev-secrets.local.ps1`.
- **Local Azure deps** — a `docker-compose.yml` brings up the **Cosmos DB emulator** + **Azurite** as the
  first-class local backing store. `TASKLIST_ENV=local` points adapters at the emulators (well-known creds
  only); `cloud` uses real Azure via managed identity + `az login`. App Insights may be left unconfigured
  locally (telemetry no-ops).

## Walking-skeleton endpoints

- `GET /api/health` → `200 {status, time}` — `time` via the `Clock` port = the DIP proof on the health path;
  the liveness probe checks only the 2xx status code, so the body is an **evolvable superset** (reconciles
  the bootstrap.md `{status:"ok"}` shorthand).
- `GET /api/tasks` → the owner's tasks via `tasks::list` through `TaskRepository` (the DIP proof — in-memory
  fake first, Cosmos later, no domain/application change).
- `GET /api/openapi.json` → the OpenAPI doc; Swagger UI at `/swagger`.
- Everything else → the SPA (`web/dist/index.html`, 200 fallback for client routes). `/api/*` misses → RFC7807 404.

## Risks / observations

1. **The Cosmos SDK is pre-GA.** The primary Cosmos adapter is a **pre-GA SDK**; the `TaskRepository` port is
   the swap-seam insurance (a REST adapter over the Cosmos data-plane, or another backing store, drops in
   without touching domain/application). An **emulator etag round-trip must be proven before building on it**
   (pinned version: see `Cargo.toml`). This is the design's highest-uncertainty dependency; expect breaking
   pre-1.0 bumps.
2. **The App Insights exporter is community/unofficial.** The direct Rust → App Insights exporter is not an
   official Microsoft path; the officially supported route is OTLP → OpenTelemetry Collector → Azure Monitor.
   *Mitigation:* emit through OTel behind the one `init_telemetry()` seam so the sink can switch if the
   exporter lags.
3. **Cosmos-without-an-ORM ergonomics.** No change tracking, no LINQ — the repository adapter does explicit
   JSON (de)serialization, manual `If-Match`/etag handling, and single-partition queries; more boilerplate
   than an ORM, contained entirely in `infrastructure`. Acceptable, worth acknowledging.
4. **Auth session store under scale-to-zero / multi-replica.** Default = an encrypted cookie-stored session
   (stateless; shared signing key from Key Vault; identity claims only). Scale-out alternative = a
   Cosmos-backed store. The chosen cookie lifetime bounds the offline window.
5. **Offline auth + id minting** (bounded, designed above): a session cookie can expire during a long offline
   window (durable outbox + re-login-before-flush); an offline-created task gets a client-side UUIDv7 id
   (owner-partitioned, collision-safe) → an idempotent first-sync create.

## Commands

- **Backend:** `cargo fmt`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test`,
  `cargo build --release`, `cargo run -p tasklist-api`.
- **Web (`web/`):** `npm ci`, `npm run lint|typecheck|test:ci|build`, `npm run gen:api`.
- **Local loop:** `./scripts/dev.ps1` (API watcher + Vite). Skills: `.github/skills/{build-test,build-test-full,run-app}.md`.
