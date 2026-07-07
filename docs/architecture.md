# Architecture Overview — task-list.rs (Rust Clean-Architecture starter)

> **Scope & relationship to other docs.** This is the alignment artifact for the Rust rewrite: a
> grounded, ~3-page tour of the target system. It **complements** `docs/design.md` (the terse
> source-of-truth project map + hard rules) with the concrete flow, persistence, patterns, and DevOps
> shape we intend to build. It mirrors the structure of the inherited Nucleus architecture doc but
> every element is **Rust-idiomatic** and reflects Mr. Das's rulings (Auth0/Google **cookie-session
> BFF**, multi-user, **preview** Cosmos DB behind a port, Blob images, App Insights, **Azure Container
> Apps** hosting, **managed identity + Key Vault** secrets, **offline-first installable PWA**, first-class
> local dev on emulators, single Production deploy).
>
> **Status:** design-time proposal. No application code exists yet. Library rows marked **(verified
> crates.io 2026-07-06)** were checked directly against the registry; rows marked **(assumption)** are
> conventional choices we should confirm during the bootstrap spike.

**task-list.rs** is a **Rust (axum) backend + React 19 (TypeScript) SPA**, laid out as a
Clean-Architecture **Cargo workspace** and deployed as a single container image on **Azure Container
Apps**, hosting the API and serving the SPA — the latter a **mobile-first, offline-first,
standards-compliant installable PWA** (§4). It is **multi-user**: **Auth0 (OIDC, Google social
connection)** authenticates users via a **cookie-based server-side session (BFF)** and all data is scoped
per user. Persistence is **Azure Cosmos DB** (system of record) + **Azure Blob Storage** (task images),
reached in prod via **managed identity** (no keys); secrets live in **Azure Key Vault** (no secrets in
env or repo). Telemetry is **Azure Application Insights**; infra is **Bicep**; CI/CD is **GitHub
Actions** with a single **Production** stage. It exists to be a full-featured clean-architecture starter
that lights up the Azure stack end-to-end.

---

## 0. Library maturity & verification (read this first)

The reinstated Azure features hinge on the **Azure SDK for Rust**, which is partly still in preview.
This is the single most important risk in the whole design, so it leads the document.

| Crate | Version | Status | Role | Note |
|---|---|---|---|---|
| `azure_storage_blob` | **1.0.0** | **GA** (verified crates.io 2026-07-06, published 2026-05-14) | Blob upload/download behind `BlobStore` port | Production-ready. |
| `azure_identity` | **1.0.0** | **GA** (verified 2026-07-06) | `DefaultAzureCredential` → managed identity for Cosmos/Blob | Production-ready. `azure_core` GA by extension. |
| `azure_data_cosmos` | **0.36.0** | ⚠ **Public preview / NOT GA** (verified 2026-07-06, published 2026-06-22 by Microsoft OSS Releases) | Cosmos client behind `TaskRepository` port | **Accepted as the primary adapter** (Mr. Das, 2026-07-06); the port is kept as insurance/swap seam. Pre-1.0 — pin the exact version, expect breaking 0.x bumps. Pulls `azure_data_cosmos_driver`, needs Rust ≥ 1.88. See risk #1. |
| `opentelemetry-application-insights` | **0.45.0** | **Community** (frigus02), not an official Microsoft crate (verified 2026-07-06) | `tracing`/OTel spans → App Insights | Widely used; unofficial. Official path is OTLP → OpenTelemetry Collector → Azure Monitor. See risk #2. |
| `openidconnect` | **4.0.1** | Mature (verified 2026-07-06) | Auth0 OIDC auth-code + PKCE | Backbone of auth. |
| `axum` | **0.8.9** | Mature (verified 2026-07-06) | HTTP routing/handlers (tower/hyper) | Backbone of the API. |
| `utoipa` | **5.5.0** | Mature (verified 2026-07-06) | OpenAPI generation → TS client | Replaces nswag. |
| `tower-sessions` | **0.15.0** | Mature 0.x (verified 2026-07-06) | Cookie-based server-side session (BFF auth) — **locked in** | Store = encrypted cookie-stored by default (survives Container Apps scale-to-zero / multi-replica via a Key Vault signing key); Cosmos-backed for scale-out. See risk #4. |
| `validator` | **0.20.0** | Mature 0.x (verified 2026-07-06) | DTO validation | Pairs with domain newtypes. |

**Bottom line:** Blob, Identity, axum, and the auth/OpenAPI stack are solid. **Cosmos is the weak
link** — the whole persistence layer sits behind a port precisely so the preview SDK can be swapped
for a REST adapter or another store without touching domain/application code. Mr. Das has **accepted the
preview Cosmos SDK** as the primary adapter (2026-07-06) with the port as the insurance policy; the
bootstrap spike must still prove an emulator etag round-trip before we build on it.

---

## 1. Directory structure

```
task-list.rs/            (Cargo workspace — cargo.toml [workspace])
├─ domain/               Entities, value-object newtypes, domain enums + errors; zero inbound deps
├─ application/          Use-cases (fns), port traits, DTOs, validation, app errors; deps: domain
├─ infrastructure/       Port adapters: Cosmos repo, Blob store, telemetry, config; deps: application, domain
├─ api/                  axum host: routers, handlers, auth middleware, OpenAPI, composition root; serves SPA
├─ web/                  React 19 + TS + Vite SPA (offline-first installable PWA) — src/{pages,components,contexts,api,hooks,offline}
├─ .github/workflows/    GitHub Actions: ci.yml (fmt/clippy/test/build/lint/drift) + deploy.yml (Production)
├─ .arm/                 Bicep IaC (main.bicep + prod.main.bicepparam + bicepconfig.json)
├─ scripts/              Local dev / secrets PowerShell helpers (session-startup, run-app, dev, secrets)
├─ docs/                 design.md, features/, this file
└─ cargo.toml            Workspace manifest (members + shared [workspace.dependencies])
```

Each library crate keeps unit tests in-crate (`#[cfg(test)]`); cross-component tests live in each
crate's `tests/` directory. Package names: `tasklist-domain`, `tasklist-application`,
`tasklist-infrastructure`, `tasklist-api` (binary).

---

## 2. Backend

### 2.1 Crates & responsibilities (dependency flow: `domain ← application ← infrastructure/api`)

| Crate | Owns | Notes |
|---|---|---|
| **domain** | `Task` entity; value-object newtypes (`TaskId`, `UserId`, `Title`, `Description`, `ImageRef`, `ETag`); `TaskStatus` enum; domain errors | No inbound deps. Only leaf crates (`uuid`, `time`, `thiserror`). Invariants enforced at construction (parse-don't-validate). |
| **application** | Use-case fns (`tasks::{create,list,get,update,delete,set_image}`), **port traits** (`TaskRepository`, `BlobStore`, `UserContext`, `Clock`), request/response DTOs, `validator` rules, application errors | Depends only on `domain`. No Azure types leak in — ports are pure Rust traits. |
| **infrastructure** | `CosmosTaskRepository` (impl `TaskRepository` over `azure_data_cosmos`), `AzureBlobStore` (impl `BlobStore` over `azure_storage_blob`), telemetry init (`tracing`+OTel→App Insights), config loader, `SystemClock` | Implements the application's ports; the only crate that references Azure SDKs. Swappable per risk #1. |
| **api** | axum routers + handlers, tower middleware (trace/CORS/session/auth/CSRF/rate-limit), `openidconnect` login/callback, `utoipa` OpenAPI + `/api/health`, RFC7807 error mapping, composition root (`main.rs`), static-file serving + SPA fallback | Wires ports→adapters via constructor injection (an `AppState`). Depends on application + infrastructure + domain. |

### 2.2 Concept map: .NET / Nucleus → Rust

| Nucleus (.NET) | task-list.rs (Rust) | Rationale |
|---|---|---|
| Solution + `.csproj` projects | Cargo workspace + member crates | Same Clean-Architecture layering, native to Rust. |
| MediatR CQRS + handlers | Plain `async fn` use-cases in `application::tasks::*` | No mediator indirection (YAGNI). Handlers call use-cases directly; optional `Command`/`Query` structs for shape. |
| Pipeline behaviours (exception/validation/authz) | tower middleware layers + explicit in-use-case checks | `TraceLayer` → CORS → session → auth extractor → CSRF; validation at the DTO boundary; ownership inside the use-case where it needs entity data. |
| FluentValidation | `validator` derive **+ domain newtypes** | Structural checks via `validator`; hard invariants (e.g. `Title` 1–200) enforced by the type so they cannot be bypassed. |
| AutoMapper | Explicit `From`/`Into` between DTO ↔ domain | Idiomatic, compile-checked, no reflection. |
| EF Core Cosmos (`IRepository`) | `azure_data_cosmos` behind **`TaskRepository`** port | ⚠ preview SDK; the port is the swap seam. See risk #1. |
| `Azure.Storage.Blobs` (`IBlobService`) | `azure_storage_blob` behind **`BlobStore`** port | GA 1.0. |
| ASP.NET OIDC + Cookie auth (Auth0) | `openidconnect` + `tower-sessions` (BFF, server-side session) | Auth-code + PKCE; **Google via an Auth0 social connection**. |
| `IUserContext` (reads claims) | **`UserContext`** extractor → current `UserId` | Injected into every use-case for per-user scoping. |
| `ICachedRepository` / HybridCache | *Omitted (YAGNI)*; optional `moka` behind a `Cache` port later | Not needed for the task-list domain. |
| App Insights server SDK | `tracing` → `opentelemetry` → `opentelemetry-application-insights` | Community exporter; OTLP→Collector fallback. See risk #2. |
| nswag (TS + C# clients) | `utoipa` (OpenAPI doc) → `openapi-typescript` (TS client) | C# client dropped (no server-side consumer). |
| xUnit + `[Trait("type", …)]` | in-crate `#[cfg(test)]` unit + `tests/` integration | Port doubles via `mockall` or hand-written fakes. |
| Bicep + Azure DevOps pipeline | Bicep + GitHub Actions | Single Production stage. |
| MSBuild `TreatWarningsAsErrors` | `cargo clippy -- -D warnings` + `cargo fmt --check` | Zero-warning policy enforced in CI. |

### 2.3 Domain model

```rust
// domain/src/task.rs  (illustrative — invariants enforced by newtype constructors)
pub struct Task {
    pub id:          TaskId,             // uuidv7 (time-ordered)
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

- **uuidv7** ids (`uuid` crate, `v7` feature) are time-ordered → naturally sortable, good Cosmos item ids.
- The **`image`** field holds only a blob **reference** (`{ownerId}/{taskId}/{imageId}`); the image
  bytes never touch Cosmos. This is the concrete demonstration of Blob Storage flowing through the layers.
- `version` mirrors the Cosmos server-managed `_etag`; the client never sets it, only echoes it on update.

### 2.4 Request flow

```
Browser SPA  (SameSite session cookie, same-origin)
  → HTTPS → axum Router
  → tower-http TraceLayer  →  CORS  →  tower-sessions (load session)
  → auth middleware: require authenticated user; build UserContext(owner) from session claims
  → CSRF check (mutations only)
  → handler (api/handlers/tasks.rs): deserialize + `validator` the DTO
  → use-case (application::tasks::create) invoked with UserContext(owner)
  → ports:  TaskRepository (Cosmos)   |   BlobStore (Blob)
  → CosmosTaskRepository → Azure Cosmos DB   (db=tasklist, container=tasks, PK=/ownerId)
     AzureBlobStore       → Azure Blob Storage (container=task-images)
Errors → AppError (thiserror) → IntoResponse → RFC7807 application/problem+json
```

**Auth challenge flow (BFF):** `GET /auth/login` → `openidconnect` builds an Auth0 authorization-code
+ PKCE redirect (scopes `openid profile email`, Google connection) → user authenticates at Auth0 →
`GET /auth/callback` validates the ID token, upserts the user, creates a server-side session, sets the
cookie → SPA. `POST /auth/logout` clears the session and redirects to the Auth0 logout endpoint.

### 2.5 Persistence model

**Cosmos DB (system of record).**
- **One database** `tasklist`, **one container** `tasks`, **partition key `/ownerId`** — the per-user
  multi-tenant seam (analogous to Nucleus's `LocationId`). Every read/write is scoped to the caller's
  partition; cross-user reads are not first-class (by design).
- **Point reads** by `(partitionKey = ownerId, id)`; list = single-partition query.
- **Optimistic concurrency**: Cosmos `_etag` + `If-Match` on replace → **412 Precondition Failed** on
  mismatch, surfaced to the client as **409 Conflict**.
- **Serialization**: `serde` structs map the domain to/from JSON docs (`id`, `ownerId`, fields, plus
  server-managed `_etag`, `_ts`). No EF-style change tracking — the repository adapter is explicit.
- **Auth to Cosmos**: `DefaultAzureCredential` (managed identity) in Azure; key/connection-string
  against the local emulator. Data-plane RBAC (Cosmos Built-in Data Contributor) in prod.

**Blob Storage (task images).**
- Container `task-images`, blob key `{ownerId}/{taskId}/{imageId}` (owner-prefixed for isolation).
- Upload flow: `set_image` use-case → `BlobStore.put(key, bytes, content_type)` → stores the returned
  key in `task.image`. Download flow: handler streams `BlobStore.get(key)` (or issues a short-lived
  read SAS/URL). Public blob access is **off**.

### 2.6 Top backend libraries

| Library | Version | Role |
|---|---|---|
| **axum** | 0.8.9 | HTTP routing/handlers over tower + hyper. |
| **tokio** | 1.x | Async runtime. |
| **openidconnect** | 4.0.1 | Auth0 OIDC (auth-code + PKCE), ID-token validation. |
| **tower-sessions** | 0.15 | Server-side session cookie (BFF). |
| **azure_data_cosmos** | 0.36 *(preview)* | Cosmos client behind `TaskRepository`. |
| **azure_storage_blob** | 1.0 *(GA)* | Blob client behind `BlobStore`. |
| **azure_identity** | 1.0 *(GA)* | `DefaultAzureCredential` (managed identity). |
| **serde / serde_json** | 1.x | (De)serialization of DTOs and Cosmos docs. |
| **validator** | 0.20 | DTO-level request validation. |
| **thiserror** | 2.x *(assumption)* | Typed domain/app errors → ProblemDetails. |
| **utoipa** | 5.5 | OpenAPI doc generation → TS client. |
| **tracing** + **opentelemetry** + **opentelemetry-application-insights** | 0.45 (exporter) | Telemetry → App Insights. |
| **uuid** *(v7)* / **time** | 1.x *(assumption)* | Ids and timestamps/dates. |

*Honorable mentions:* `tower-http` (CORS, TraceLayer, compression, static files), `tower_governor`
(rate limiting), `figment` or `config` (layered config), `mockall` (port test doubles), `anyhow`
(composition-root error glue only), `reqwest` (transitive — used by the Azure/OTel crates).

---

## 3. Frontend (`web/`)

React 19 + TypeScript + Vite SPA, same-origin with the API, delivered as a **mobile-first, offline-first,
standards-compliant installable PWA** — this is Mr. Das's stated product identity, so the offline/PWA
layer is a **first-class concern**, designed in its own right in **§4**. The rest of the UI stays
deliberately lean; the starter's depth is in the backend/Azure stack + the offline data layer, not in
elaborate screens.

### Folders & responsibilities (`web/src/`)

| Folder / file | Responsibility |
|---|---|
| `pages/` | Routed screens (`Tasks` list, `Task` detail/edit, `Login`, `AuthCallback`, `NotFound`). |
| `components/` | `feature/` (task UI: list, form, image upload), `layouts/` (chrome), `primitives/` (base UI). |
| `contexts/` | `AuthProvider` (current-user/session state via `/api/me`), `Theme`. |
| `api/` | `ApiClient.generated.ts` (from OpenAPI — never hand-edited) + hand-written base wrapper. |
| `hooks/` | `useHealthStatus`, data hooks over the generated client. |
| `offline/` | Owner-scoped offline engine: `db.ts` (Dexie schema), `types.ts`, `store.ts` (mirror/outbox/blob primitives + teardown), `sync.ts` (health-driven pull/push). See §4. |
| `pwa.tsx` | Service-worker registration (`virtual:pwa-register`) + update toast; **Reload gated on an empty outbox** (never clobber pending writes). |
| `App.tsx`, `main.tsx`, `ErrorBoundary.tsx` | Root app, bootstrap, error boundary. |

### Top libraries

| Library | Role |
|---|---|
| **react** 19 / **react-dom** | UI runtime. |
| **vite** 6 (+ `@vitejs/plugin-react-swc`) | Dev server + build. |
| **react-router-dom** 7 | Client-side routing. |
| **@tanstack/react-query** *(or SWR)* | Server-state fetching/caching over the generated client. |
| **tailwindcss** 4 + **daisyui** 5 | Styling / components. |
| **lucide-react** | Icons. |
| **@microsoft/applicationinsights-web** (+ react plugin) | Client telemetry → App Insights. |
| **dexie** | IndexedDB wrapper — the owner-scoped offline **mirror / outbox / blob cache** (§4). |
| **vite-plugin-pwa** (+ **Workbox**) | Installable PWA: injects the manifest + Workbox `generateSW` app-shell precache & SW registration (§4.8). |
| **openapi-typescript** (dev) | Generates the typed client from the OpenAPI doc. |
| **vitest** + **@testing-library/react** (+ `jsdom`) | Test stack. |

**Auth pattern (locked in — Mr. Das, 2026-07-06).** **Cookie-based server-side session (BFF)** with
**Auth0 + Google social connection**: the backend runs the OIDC auth-code + PKCE dance and holds the
session; the SPA is a same-origin cookie client and stores **no tokens** in the browser (most secure,
matches Nucleus). The SPA-driven `auth0-react` bearer-JWT alternative (tokens in the browser) was
weighed and **rejected**. Offline session behaviour — cached identity, cookie expiry, re-auth on
reconnect — is designed in **§4.7**.

---

## 4. Offline-first & PWA

The README's stated identity is **"mobile-first, offline-first, standards-compliant installable PWA."**
That makes offline a **first-class architectural concern**, not a bolt-on. This section answers Mr.
Das's question — *"what is the clash?"* — head-on, then designs the layer. It adapts the inherited
Nucleus `docs/offline-first-design.md` (axes A–G) to a **generic, owner-partitioned task list** (drop
the site-visit specifics; keep the owner-scoped mirror + outbox + etag-guarded replay).

### 4.1 Is there a clash with the per-user Cosmos model? — **No (claim retracted)**

My earlier risk note asserted offline-first "conflicts with the per-user Cosmos model." **That was
overstated; I withdraw it.** Working it through from first principles, the per-user model is if anything
**well-suited** to offline:

| Concern | Per-user (owner-partitioned) Cosmos reality | Verdict |
|---|---|---|
| **What to mirror** | Each user's entire dataset **is a single Cosmos partition** (`/ownerId = me`). The offline mirror is a 1:1 replica of *one partition* — no cross-user data, no shared docs, no multi-tenant fan-out. | **Aligned.** This is the *easiest* case for a mirror. |
| **Conflict detection** | Writes are whole-doc upserts guarded by `_etag` + `If-Match`. The held etag *is* a ready-made optimistic-concurrency / conflict-detection primitive. | **Aligned.** Etag gives detection for free. |
| **Concurrency surface** | Single-owner data ⇒ the *only* realistic concurrent writer is **the same user on another device**. No office-vs-field, no multi-writer merge. | **Narrowed**, not worsened. LWW handles it (§4.6). |
| **Write shape** | Whole-document replace (no field-level patch API) → outbox stores the latest command per task and **replays it verbatim** — server contract unchanged. | **Aligned** with Nucleus's outbox-replay model. |

**The two things that *are* genuinely different** (bounded, non-blocking — both designed below):

- **(a) Offline auth window** — the BFF **session cookie can expire during a long offline stretch**, so
  the outbox can't drain until the user re-authenticates. This is a *timing* tension, not a data-model
  clash. Mitigation: **durable outbox + re-login-before-flush** (lossless), optionally an extended
  session. §4.7.
- **(b) Offline id minting** — Nucleus deliberately never minted IDs offline (SVP/Checklist were
  server-provisioned). A generic task list lets a user **create** a task with no signal, so the client
  **mints the `TaskId` (uuidv7) itself**. That's safe: uuidv7 is collision-proof and owner-partitioned,
  and using the client id as the Cosmos doc id makes the first sync an **idempotent create**. §4.4/4.6.

**Conclusion:** design offline-first properly (below). It is core, not optional.

### 4.2 Two independent layers

Offline is delivered by **two layers that must not be conflated**:

1. **App-shell PWA (Workbox).** The service worker precaches the *static* built shell and does a
   navigation fallback to `index.html`. It **never caches `/api/*` or `/auth/*`** (§4.8). Its only job is
   "the app boots with no network."
2. **Offline data engine (app-managed, Dexie).** All offline *data* lives in an owner-scoped IndexedDB
   mirror + outbox that the app reads/writes/syncs explicitly. **We do not use Workbox runtime caching of
   API responses** — that would give stale reads, cache auth responses, and bypass etag handling. This is
   the same stance Nucleus took (SW `/api` denylist; data via Dexie).

Keeping these separate is the key architectural decision: the SW gives *installability + shell offline*;
the Dexie engine gives *data offline* with real sync + concurrency semantics.

### 4.3 Client mirror (Dexie / IndexedDB), owner-scoped

One Dexie DB, four stores. **Every content record carries an `owner` field**; all reads are
owner-filtered; content is cleared on **logout / owner-change** (never on same-user re-auth).

| Store | Key | Holds |
|---|---|---|
| `tasks` | `id` (uuidv7) | Read mirror of the user's tasks: full task VM incl. `owner`, `status`, `version` (`_etag`), `updatedAt`. |
| `outbox` | `taskId` | Pending writes, **one latest per task** (coalesced by PK): `{ taskId, owner, kind: 'upsert' \| 'delete', command, heldEtag, enqueuedAt, status }`. `command` is the **verbatim request body** an online client would send (new image as base64, existing image as its blob ref). |
| `blobs` | blob key | Cached task images as **`Blob`s** (converted to base64 only at flush — memory safety). |
| `meta` | (singleton) | Cached boot/identity context: `/api/me` result (`sub`, `email`, `name`), `syncedAt`. **Single-user-per-device** (accepted limitation — see §4.7). |

**Where owner/partition scoping lives client-side:** the cached `sub` **equals the Cosmos partition key
`/ownerId`**. The client filters every mirror read by `owner == cached sub`. This is a
convenience/isolation layer, **not** the security boundary — the server still re-derives the owner from
the session and enforces the per-user partition + 404-on-not-owned on **every** sync call (§2.5, §5).

### 4.4 Offline read & write flow

- **Read (offline):** the UI reads from the `tasks` mirror (owner-filtered) and images from `blobs`.
  Pending `outbox` edits are **overlaid** onto the mirror so a re-opened task shows the latest local
  state and is **re-editable** (Nucleus's write-through, axis D3).
- **Create (offline):** mint a **client-side uuidv7 `TaskId`** → write the new task into the mirror →
  enqueue an `upsert` with **no `heldEtag`** (it's a create). Because the client id becomes the Cosmos
  doc id, a replay is an idempotent create.
- **Update (offline):** write-through to the mirror → enqueue an `upsert` carrying the **held `_etag`**
  (coalesces with any pending edit for that task).
- **Delete (offline):** remove from the mirror → enqueue a `delete` (idempotent: a 404 on replay = success).
- **Images:** captured as a `Blob` in `blobs`; embedded as base64 into the command **only at flush**.

Online, the same use-cases run immediately (no outbox) — the offline path is a divert triggered by
`!navigator.onLine` / an `isOfflineFailure` classifier on a mid-write network drop.

### 4.5 Sync on reconnect (pull + push), health-driven

Reconnect is driven by the existing **`useHealthStatus`** halo (green/amber/red), not a naked
`navigator.onLine`.

- **Pull ("Sync my tasks"):** when green, run the **single-partition list query** for my tasks → refresh
  the `tasks` mirror + download referenced images into `blobs` → stamp `syncedAt`. Idempotent; safe to
  re-run. Best-effort `navigator.storage.persist()` here to reduce eviction.
- **Push ("flush"):** drain the owner-scoped `outbox`, replaying each `command` **verbatim** against the
  existing REST endpoints:
  - **upsert** → `PUT`/upsert with `If-Match: heldEtag` (create → no `If-Match`).
  - **`200/201`** → delete the outbox row **and re-pull that task** (fresh etag + image) so the next
    offline edit doesn't self-inflict a 409.
  - **`412` → surfaced as `409`** → conflict (see §4.6).
  - **`401`** → session expired offline → surface "re-login to sync," trigger the Auth0 redirect on
    reconnect; **keep the row** (durable). See §4.7.
  - **other** → mark `error`, keep the row.
  - **Flush-time guard:** if a replay command still references an **evicted local blob** (`offline:` id),
    do **not** POST — mark `error` and surface it (prevents a corrupt replay). (Nucleus's T4 guard.)
- **Triggers:** opportunistic **debounced flush on red→green**, an explicit manual **Sync** affordance,
  and a first-load pull. **Gate the SW update-toast's Reload on an empty outbox** so a new build never
  clobbers pending writes.

### 4.6 Conflict resolution vs `_etag` / version + 409

The whole-doc upsert guarded by `_etag`/`If-Match` **is** the concurrency mechanism (§2.5). Because data
is single-owner, the only reachable conflict is **the same user editing the same task from two devices**
(one offline). Default policy — **last-writer-wins, scoped to the owner's own devices**:

1. Replay carries the `heldEtag` → server compares against the live `_etag`.
2. **Match** → applied; re-pull refreshes the etag.
3. **Mismatch → 412 → 409** → mark `conflict`, **re-pull for visibility**, then an **explicit retry**
   re-stamps the command with the fresh `_etag` and replays (this device's edit wins).

**No CRDT, no field-merge** — that solves multi-writer concurrency and needs a patch API we don't have
(**YAGNI**). LWW-with-re-pull is the decided policy (Mr. Das, 2026-07-06); a merge/choose-a-version UI
is explicitly out of scope.

### 4.7 Offline auth / session behaviour

- **Reads/writes need no session** — they hit the local mirror + outbox, so a valid cookie is irrelevant
  offline.
- **Cached identity** in `meta` (from `/api/me` at last online) lets the shell render "signed in as X"
  offline and supplies the `owner` used for scoping. It is **single-user-per-device**: a *second* user
  booting **offline** on a shared device could see the prior user's cached identity until an online login
  repopulates it — an accepted limitation, mitigated by **clear-on-logout** (same caveat Nucleus accepted).
- **Cookie expiry during an offline window** is the one real tension (§4.1a). Sync (not reads) needs a
  live cookie; if it expired, the outbox drain **401s** → the app prompts **re-login on reconnect**, then
  flushes. Because the **outbox is durable**, nothing is lost. A generic task list isn't Nucleus's 12h
  field day, so a **rolling multi-day session** comfortably outlives typical offline windows; an
  **extended/role-scoped session** is an available lever if telemetry shows expiry hurting.
- **Security boundary stays server-side:** offline owner-scoping is convenience/defense-in-depth; every
  sync call still authenticates and re-derives the owner server-side (§4.3).

### 4.8 Installable PWA (service worker + manifest)

- **`vite-plugin-pwa`** (Workbox **`generateSW`**): precache the built shell
  (`js,css,html,svg,png,ico,woff2`) + `navigateFallback: '/index.html'`, `cleanupOutdatedCaches: true`.
- **`navigateFallbackDenylist`: `/^\/api\//`, `/^\/auth\//`, `/^\/swagger/`** — critical: the SW must
  **never** serve cached `index.html` on the **Auth0 OIDC callback** (`/auth/callback`) or it hijacks the
  login and causes an **infinite login loop** for installed/returning users. (This was the exact blocker
  the architect caught in Nucleus's `installable-pwa`.)
- **No runtime caching of `/api`** — offline data is the Dexie engine's job (§4.2), avoiding stale reads
  and auth-response caching.
- **`registerType: 'prompt'`** → non-disruptive "Update available — Reload" toast; **Reload gated on an
  empty outbox** (§4.5).
- **Manifest** (plugin-owned, single source of truth in `vite.config.ts`): complete `id` / `name` /
  `short_name` / `description` / `lang` / `dir` / `start_url` / `scope` / `display: standalone` /
  `theme_color` / `background_color`, and icons with **both `any` and `maskable`** purposes → installable
  on mobile + desktop.
- **Mobile-first UI:** responsive single-column task list, large touch targets, no desktop-only chrome —
  the app is designed for a phone first, scaling up.

### 4.9 Adaptation from Nucleus (what changes, what carries over)

| Nucleus (site engineers) | task-list.rs (generic) | Why |
|---|---|---|
| Offline **fenced to the SE role** + SVP surface; office stays online-first | **Whole-surface offline for every user** | All data is the user's own partition — no office/field split to fence. |
| IDs **server-provisioned**, SE never mints offline | **Client mints `TaskId` (uuidv7) offline** | Users create tasks with no signal; uuidv7 + idempotent create make it safe (§4.1b). |
| "Sync **my day**" (`VisitBy && Initiated`, geo, photos-per-visit) | "Sync **my tasks**" (single-partition list) | Drop site-visit specifics; keep the owner-scoped pull. |
| Server rule "office can't edit an assigned in-field SVP" to make 409 rare | **Not needed** — data is single-owner by construction | The partition *is* the ownership fence. |
| Dexie mirror/outbox/blob + `meta`; outbox whole-command replay under etag; health-driven flush; SW `/api` denylist; Reload gated on empty outbox | **Carried over unchanged** | Proven shape; directly applicable. |

**Slice-ability:** offline is naturally incremental (mirrors Nucleus's `installable-pwa` → `offline-1/2/3`):
(1) installable app-shell PWA; (2) read mirror + "Sync my tasks"; (3) offline write + outbox + flush +
conflict handling. Each is an independently deployable, end-to-end slice.

---

## 5. Application-wide patterns

- **Use-cases, not a mediator** — every operation is an `async fn` in `application::tasks::*` taking a
  `UserContext` + validated input and the ports it needs. Simple, explicit, testable; no MediatR-style
  dispatch.
- **Ports & adapters** — `TaskRepository`, `BlobStore`, `UserContext`, `Clock` are traits in
  `application`; `infrastructure` supplies the Azure-backed impls; tests supply in-memory fakes. This
  is the seam that contains the preview-SDK risk.
- **Error handling** — libraries return typed `thiserror` errors; `api` maps them to **RFC7807
  `application/problem+json`** via `IntoResponse`: 400 (validation), 401 (unauthenticated), 403
  (ownership), 404 (not found / not owned — don't leak existence), 409 (etag conflict), 500 (unexpected).
- **Validation** — two layers: `validator` on DTOs at the edge; **newtype parsing** in the domain so
  invariants (e.g. `Title` length) are unrepresentable-if-invalid.
- **Authorization = per-user ownership** — no roles (YAGNI). The `UserContext` yields the caller's
  `UserId`; repository access is partition-scoped to that user, and a fetched task whose `owner`
  differs returns 404. Ownership is the entire authorization model.
- **Auth / session (locked in)** — **Cookie-based server-side session** (`tower-sessions`) + **Auth0
  OIDC** (`openidconnect`, auth-code + PKCE, **Google social connection**). On callback the app validates
  the ID token and upserts a `User` keyed by the Auth0 `sub`. `SameSite=Lax` + `Secure` + `HttpOnly`
  cookie; CSRF token for mutations. Default store = **encrypted cookie-stored** (stateless — survives
  Container Apps scale-to-zero / multi-replica; signing key from Key Vault); Cosmos-backed store is the
  scale-out alternative. The session lifetime governs offline sync (§4.7).
- **Config & secrets (Mr. Das ruling, 2026-07-06)** — non-secret config via layered `figment`/`config`
  (defaults → file → Container Apps env). **Secrets live only in Azure Key Vault** — the Auth0 client
  secret and the session signing key are Key Vault entries surfaced as **Container Apps secrets (Key
  Vault references)** and read at runtime under the app's **managed identity**; **no secrets in env or
  repo**. Cosmos + Blob data-plane access is **keyless** via `DefaultAzureCredential` (managed identity)
  in prod. Locally, only **emulator** credentials (the well-known Cosmos-emulator key / Azurite dev key)
  are used — never a real secret; `scripts/dev-secrets.local.ps1` is git-ignored.
- **Telemetry** — `tracing` spans/events → `opentelemetry` → App Insights (`opentelemetry-application-insights`).
  A single `init_telemetry()` in `infrastructure` owns the exporter so the sink is swappable (risk #2).
- **Typed client generation** — `utoipa` derives the OpenAPI doc from handlers/DTOs; a build/CI step
  runs `openapi-typescript` to regenerate `web/src/ApiClient.generated.ts`. Generated files are never
  hand-edited; a **CI drift check** fails if the committed client differs from a fresh generation.
- **Zero-warning policy** — `cargo fmt --check` + `cargo clippy --all-targets --all-features -D
  warnings` gate CI; web side runs its lint/type-check clean. Enforced in CI rather than
  `#![deny(warnings)]` in source (keeps future toolchain bumps from breaking builds).
- **Hardening** — `tower_governor` rate limiting, body-size limits for uploads, CORS restricted
  (same-origin in prod), CSRF for cookie-auth mutations, HTTPS-only.
- **Testing** — domain: exhaustive unit tests on newtypes + status transitions. application: use-case
  unit tests against in-memory port fakes (`mockall`). infrastructure: `tests/` integration against
  the **Cosmos emulator** + **Azurite**. api: `tests/` HTTP tests via `axum`'s test client. Tag/organize
  unit vs integration so CI can run them in separate jobs (mirrors Nucleus's `[Trait]` split).

---

## 6. DevOps

### Pipeline (GitHub Actions)

```
ci.yml   (push / PR to main)
├─ backend   : cargo fmt --check → clippy -D warnings → cargo test (unit) → cargo test (integration, emulator+Azurite services) → cargo build --release
├─ web       : npm ci → lint → typecheck → vitest → vite build
├─ drift     : regenerate OpenAPI + openapi-typescript → git diff --exit-code (fails on stale generated client)
├─ iac       : bicep build/lint + az deployment group validate (prod params)
└─ image     : docker build (multi-stage: cargo build → runtime image with SPA assets)

deploy.yml  (push to main, or manual)  ── Production only ──
└─ build image → push (ACR/GHCR) → az deployment group create (main.bicep + prod.main.bicepparam) → roll a new **Container Apps revision** → smoke-probe /api/health
                                     (Production gated by a GitHub Environment approval)
```

Single **Production** stage (staging dropped per Mr. Das). Integration tests run against emulator
services in CI so they need no live Azure.

### Infrastructure (Bicep — `.arm/main.bicep`, single Production)

| Resource | Config highlights |
|---|---|
| **Cosmos DB** (Core/SQL) | **Serverless**, Session consistency; database `tasklist`, container `tasks` PK `/ownerId`; periodic backup; **data-plane RBAC** (Built-in Data Contributor) to the app's managed identity — no keys in prod. |
| **Storage Account** | StandardV2, blob container `task-images`; public blob access **off**, soft-delete on; **Storage Blob Data Contributor** role to the app identity. |
| **Azure Container Apps** (locked in) | Hosts the single container (API + SPA); external ingress, HTTPS-only, min-replicas 0 (**scale-to-zero**) → N; **system-assigned managed identity** for Key Vault, Cosmos & Blob. Managed environment + Log Analytics workspace. |
| **App Insights** (workspace-based) + **Log Analytics** | Telemetry sink; connection string injected as app setting. |
| **Azure Key Vault** (locked in) | Holds the Auth0 client secret + session signing key; exposed as **Container Apps secrets via Key Vault references**, read under managed identity (RBAC: **Key Vault Secrets User**). No secrets in env/repo. |
| **Params** | Auth0 `domain` / `clientId` / `audience` (plain); `clientSecret` + session signing key are **Key Vault secrets** (referenced, not passed as `@secure()` deploy params); env/version/timestamp. |

### Config & secrets

- Runtime config set on the Container App by Bicep at deploy: `Auth0__*` (non-secret), App Insights
  connection string, Cosmos + Storage **endpoints** (keyless data access via managed identity). The
  **Auth0 client secret and session signing key are Key Vault references** surfaced as Container Apps
  secrets — never plaintext env or `@secure()` deploy params.
- CI/deploy secrets are **GitHub Actions secrets** (Azure OIDC federated login — no stored SP
  password; Auth0 params; ACR/GHCR creds).
- Generated OpenAPI spec + TS client are committed artifacts; the drift job keeps them honest.

### Local dev experience (a hard requirement)

The existing `scripts/` are repurposed from the .NET toolchain to cargo + Vite, keeping the same
liveness/health contract so the agent loop is unchanged:

- **`scripts/session-startup.ps1`** — unchanged role: starts the detached 30s liveness watch.
- **`scripts/run-app.ps1`** — unchanged lifecycle (Ensure/Restart/Stop/Watch, `.dev-state.json`
  building/ready/broken signal); the health probe stays `GET /api/health`.
- **`scripts/dev.ps1`** — repurposed: dot-source `dev-secrets.local.ps1`, then run the API and SPA
  watchers: `cargo watch -x 'run -p tasklist-api'` (or `bacon`) **+** `npm --prefix web run dev`
  (Vite). Vite proxies `/api/*` to the backend so the SPA is same-origin in dev too.
- **`scripts/dev-secrets.template.ps1`** — repurposed env vars: `AUTH0__DOMAIN/CLIENTID/AUDIENCE`,
  `COSMOS__ENDPOINT`, `STORAGE__CONNECTION`, `APPLICATIONINSIGHTS_CONNECTION_STRING`, and a
  `TASKLIST_ENV=local|cloud` toggle. The **Auth0 client secret** is Key Vault in cloud; locally it lives
  only in the **git-ignored** `dev-secrets.local.ps1` (never committed, never a cloud env var).
- **Local Azure dependencies (first-class local dev — Mr. Das ruling)** — a `docker-compose.yml` brings
  up the **Cosmos DB emulator** (Linux container) + **Azurite** (Blob) as the first-class local backing
  store. `TASKLIST_ENV=local` points the adapters at the emulators (well-known emulator credentials
  only); `cloud` uses real Azure via `DefaultAzureCredential` + `az login`. App Insights can be left
  unconfigured locally (telemetry no-ops) or pointed at a dev resource.

> ⚠ **Validate early:** the **preview `azure_data_cosmos` + Cosmos emulator** combination is the
> riskiest local-dev assumption. The bootstrap spike must prove a round-trip (create/read/update with
> etag) against the emulator before we build on it.

---

## 7. Observations / open risks (please weigh in)

1. **Azure SDK for Rust — Cosmos is preview. DECIDED (2026-07-06): accepted.** Mr. Das accepted the
   preview `azure_data_cosmos` (**0.36.0, pre-1.0**) as the **primary adapter**; the `TaskRepository`
   port stays as insurance — we can drop in a **REST adapter** over the Cosmos data-plane (`azure_core`)
   or swap the backing store without touching domain/application. Residual (not a blocker): **pin the
   exact version**, expect breaking 0.x bumps, and **prove an emulator etag round-trip in the bootstrap
   spike** before building on it. Remains the design's highest-uncertainty dependency.

2. **App Insights exporter is community/unofficial.** `opentelemetry-application-insights` (frigus02)
   is the direct Rust→App Insights path but is **not a Microsoft crate**; the officially supported
   route is OTLP → OpenTelemetry Collector → Azure Monitor. *Mitigation:* emit through `tracing`+OTel
   behind one `init_telemetry()` seam so we can switch to the Collector path if the crate lags.

3. **Cosmos-without-EF ergonomics.** No change tracking, no LINQ — the repository adapter does explicit
   serde (de)serialization, manual `If-Match`/etag handling, and single-partition queries. This is
   more boilerplate than EF Core, contained entirely in `infrastructure`. Acceptable, but worth
   acknowledging as an ongoing cost.

4. **Auth session strategy + store. DECIDED (2026-07-06): cookie-based server-side session (BFF),
   Auth0 + Google.** Residual = the **store shape** under Container Apps (scale-to-zero + multi-replica):
   in-memory won't survive either. Default = **encrypted cookie-stored session** (stateless — survives
   scale-to-zero and works across replicas via a **shared signing key from Key Vault**; small — identity
   claims only, no Auth0 tokens needed since downstream access is via managed identity). Scale-out /
   opaque-session-id alternative = a **Cosmos-backed store**. Action: confirm the **Auth0 tenant has the
   Google social connection** enabled. The chosen cookie lifetime also bounds the offline window (§4.7).

5. **Hosting shape. DECIDED (2026-07-06): Azure Container Apps** (scale-to-zero, cheaper idle, good fit
   for a fork-and-run starter). Bicep + deploy target Container Apps; the session-store choice (risk #4)
   follows from scale-to-zero / multi-replica.

6. **Offline-first / PWA — RETRACTION + DECISION (2026-07-06). In scope; there is no fundamental clash.**
   My earlier claim that offline-first "conflicts with the per-user Cosmos model" was **overstated and is
   withdrawn** (full analysis in **§4.1**). A per-user (owner-partitioned) model is in fact
   **well-suited** to an offline mirror — each user mirrors *exactly their own partition*, nothing is
   shared, and Cosmos `_etag`/version optimistic concurrency is a **ready-made conflict-detection
   primitive**. Two bounded, non-blocking tensions remain (both designed in §4): **(6a) offline auth** —
   a session cookie can expire during a long offline window, forcing re-auth before the outbox can drain
   (mitigated by a **durable outbox + re-login-before-flush**, optionally an extended session);
   **(6b) offline id minting** — a task *created* offline gets a **client-side uuidv7 id**
   (owner-partitioned, collision-safe), making the first sync an **idempotent create**.

7. **Managed identity + Key Vault. DECIDED (2026-07-06).** **Managed identity** for Cosmos + Blob
   data-plane in prod (`azure_identity` GA — no stored keys); **Azure Key Vault** for all secrets (Auth0
   client secret, session signing key), surfaced as **Container Apps secrets via Key Vault references**
   and read under managed identity. **No secrets in env or repo.** Keys/connection-strings only for local
   emulators.

8. **Offline conflict-resolution policy — DECIDED (Mr. Das, 2026-07-06): last-writer-wins scoped
   to the owner's own devices** (409 → re-pull → explicit retry re-stamps the fresh `_etag`, this
   device's edit wins). Because data is single-owner, the only realistic conflict is the same user
   editing the same task from two devices; CRDT/field-merge is **YAGNI** (no multi-writer concurrency, no
   patch API). A merge/choose-a-version UI is out of scope.

> **Process note (not architecture):** guardrail #3 in `.github/copilot-instructions.md` says "Never
> touch `master`," but trunk here is **`main`**. Flagging for JARVIS to reconcile the wording.
