# Work Item: Bootstrap — Walking Skeleton (end-to-end pipeline light-up)
**Branch:** vibe/bootstrap
**Status:** Planning

> **Intent (Mr. Das's words):** *"Just sufficient placeholder code deliverables at this point to light
> up all aspects end-to-end, including GH CI, ARM/Bicep, Azure."* This feature is the **walking
> skeleton**: the thinnest vertical slice that nonetheless exercises **every** architectural concern the
> final app will use, with placeholder/stub logic where real task-management features come later. It
> proves the whole pipeline is wired — it does **not** deliver task features.
>
> **Source of truth:** `docs/design.md` (the merged authoritative design + rules, all rulings baked in).
> This spec does not restate the design; it sequences its realization. The two deliberate reconciliations
> are called out in **Notes & Decisions** (IaC dir, guardrail wording).

---

## Options Considered

### Option 1 — Walking skeleton: thinnest vertical slice, every concern stubbed  ✅ RECOMMENDED
- **Description:** Task 1 stands up the full 4-crate workspace + axum host + SPA shell + CI + local-dev
  loop end-to-end through `/api/health` (with an **in-memory** `TaskRepository` proving DIP). Each
  subsequent task lights up exactly one remaining concern (Cosmos, Blob, Auth, Telemetry, PWA/offline,
  IaC/Azure) as an independently deployable full-stack slice with placeholder logic. Final task is a
  repo-wide residue/contradiction sweep.
- **Pros:** Matches Mr. Das's brief verbatim. Proves the pipeline (CI → container → Azure) on day one;
  every task is e2e-verifiable and deployable on its own; **top risk (preview Cosmos SDK) is de-risked
  early** (Task 2). No throwaway scaffolding — each stub is the real seam, later filled.
- **Cons:** Task 1 is broad (workspace + CI + scripts + skeleton in one slice). Mitigated: it is still
  *thin* (one health path, in-memory repo, no Azure), and its breadth is the point of a skeleton.

### Option 2 — Breadth-first by layer (all backend, then all frontend, then infra)
- **Description:** Build the complete backend, then the complete SPA, then IaC/CI last.
- **Pros:** Each layer internally coherent; fewer context switches.
- **Cons:** **No end-to-end proof until the very end** — CI/Azure/PWA light up last, exactly inverting the
  brief. Violates the "each task is a full-stack, independently verifiable slice" rule. Integration risk
  (Cosmos preview, Auth0 callback, SW denylist) surfaces late. **Rejected.**

### Option 3 — Depth-first per concern (fully implement Cosmos, then fully implement Auth, …)
- **Description:** Complete each concern to production depth before starting the next.
- **Pros:** Each concern "done-done" once.
- **Cons:** Over-builds before the pipeline is proven; contradicts "placeholder code to light up all
  aspects." Delays the first green CI + first deployable image. Wastes effort hardening features whose
  end-to-end wiring hasn't been validated. **Rejected.**

## Selected Design

**Option 1 — walking skeleton, risk-ordered.** Realizes `docs/design.md` as follows:

- **Layering (§1, §2):** Cargo workspace, four crates `tasklist-{domain,application,infrastructure,api}`,
  dependency flow `domain ← application ← infrastructure/api`. Ports (`TaskRepository`, `BlobStore`,
  `UserContext`, `Clock`) are traits in `application`; adapters live only in `infrastructure`; `api` is
  the composition root (constructor injection via `AppState`).
- **Skeleton path:** `GET /api/health` (200) + a trivial owner-scoped read through the `TaskRepository`
  port — **in-memory fake first (Task 1)**, swapped to the real Cosmos adapter (Task 2) with no
  domain/application change. This is the concrete DIP demonstration.
- **Risk-first sequencing:** the preview `azure_data_cosmos` etag round-trip against the **Cosmos
  emulator** (design.md — Risks/observations: the Cosmos preview-SDK risk, the highest-uncertainty dependency) is proven in **Task 2**,
  immediately after the skeleton, before anything is built on it.
- **One concern per task thereafter:** Blob/Azurite → Auth0+Google cookie BFF → App Insights telemetry →
  installable PWA + Dexie stub → IaC (Bicep) + managed identity + Key Vault + single Production deploy.
- **Doc reset:** author fresh `docs/design.md` (terse project map + hard rules for the Rust stack) in
  Task 1; it evolves as tasks land. Rewrite `README.md`, `Cargo.toml`, `.gitignore`, `.editorconfig`,
  the `scripts/*`, and the `.github/skills/*` to the cargo + Vite toolchain.
- **Local dev is first-class:** keep the `set-dev-state.ps1` / `.dev-state.json` lifecycle mechanism and
  the `GET /api/health` contract unchanged (Dave guardrail #8 and the liveness watch depend on them);
  repurpose `dev.ps1`/`run-app.ps1`/`session-startup.ps1` to `cargo watch` + Vite + a docker-compose for
  the Cosmos emulator + Azurite, with a `TASKLIST_ENV=local|cloud` toggle.
- **Placeholder discipline:** every stub is the *real seam* (a trait impl, a real endpoint, a real CI
  job), carrying trivial logic — never scaffolding to be deleted.

---

## Task Breakdown

Each row is a full-stack, independently deployable/verifiable slice. Dave signals `building` → `ready`
via `set-dev-state.ps1` per task; Bhaskar validates build/test; JARVIS handles all git.

| #  | Task Description                                                                          | Status  | Commit |
|----|------------------------------------------------------------------------------------------|---------|--------|
| 1  | Workspace + doc reset + walking skeleton (`/api/health` e2e via port) + CI green          | Verified ✓ — awaiting commit | -      |
| 2  | Cosmos preview-crate spike: real etag round-trip via `TaskRepository` on the emulator      | Pending | -      |
| 3  | Blob stub upload/download through `BlobStore` against Azurite (image plumbing)             | Pending | -      |
| 4  | Auth0 + Google cookie BFF: OIDC login/callback/logout + session, protecting an endpoint    | Pending | -      |
| 5  | App Insights telemetry seam: `init_telemetry()` (tracing→OTel→App Insights)                | Pending | -      |
| 6  | Installable PWA shell (Workbox) + owner-scoped Dexie stub (offline scaffolding)            | Pending | -      |
| 7  | IaC (fresh Bicep): Container Apps + Cosmos + Blob + Key Vault + App Insights + MI; deploy   | Pending | -      |
| 8  | Repo-wide contradiction & residue sweep (reconciled repo)                                  | Pending | -      |

### Task 1 — Workspace + doc reset + walking skeleton + CI green
**Scope**
- Cargo workspace: `Cargo.toml [workspace]` with members `domain`, `application`, `infrastructure`,
  `api`, and shared `[workspace.dependencies]`; packages `tasklist-{domain,application,infrastructure,api}`
  (`api` = binary). Dependency flow enforced per §2.1.
- `domain`: minimal `Task` entity + newtypes (`TaskId` uuidv7, `UserId`, `Title`) + `TaskStatus` + domain
  error — just enough for the skeleton path (invariants at construction).
- `application`: `TaskRepository` + `Clock` port traits; one trivial use-case (e.g. `tasks::list` for the
  caller) + DTOs.
- `infrastructure`: `InMemoryTaskRepository` (impl `TaskRepository`) + `SystemClock`; config loader stub
  with `TASKLIST_ENV` seam.
- `api`: axum host; `GET /api/health` → 200 `{status,time}`; the trivial use-case wired through the port
  (proves DIP); `utoipa` OpenAPI doc + `/api/openapi.json` (+ swagger UI); static-file serving + SPA
  fallback (`index.html`); RFC7807 error mapping seam; composition root builds `AppState` with the
  in-memory repo. **No Azure crates yet.**
- `web/`: React 19 + TS + Vite SPA shell — calls `/api/health` via a generated typed client, renders a
  health indicator + placeholder auth-state area. `openapi-typescript` generates
  `web/src/ApiClient.generated.ts` from the OpenAPI doc (never hand-edited); base fetch wrapper; Vite dev
  proxy `/api/*` → backend (same-origin).
- **Doc reset:** author `docs/design.md` (terse source-of-truth project map + hard rules for this stack);
  rewrite `README.md` (Rust/React stack), `Cargo.toml`,
  `.gitignore` (Rust `target/`, Node `node_modules/`, `dist/`, keep the dev-secret/state ignores),
  `.editorconfig` (Rust/TS).
- **Scripts (repurpose, keep the contract):** `dev.ps1` → dot-source local secrets then run
  `cargo watch -x 'run -p tasklist-api'` **+** `npm --prefix web run dev`; drop the legacy HTTPS dev-cert
  block. `run-app.ps1` → Ensure/Restart/Stop/Watch lifecycle over the cargo/Vite process, health probe
  stays `GET /api/health`, still respects `.dev-state.json`. `session-startup.ps1` → unchanged role.
  `dev-secrets.template.ps1` → Rust env vars (`AUTH0__*`, `COSMOS__ENDPOINT`, `STORAGE__CONNECTION`,
  `APPLICATIONINSIGHTS_CONNECTION_STRING`, `TASKLIST_ENV`). **Keep `set-dev-state.ps1` + `.dev-state.json`
  untouched.**
- **Skills:** rewrite `.github/skills/{build-test,build-test-full,run-app}.md` for cargo + Vite (fmt,
  clippy, `cargo test`, `npm run lint/typecheck/test:ci/build`; integration variant dot-sources local
  secrets + emulators).
- **CI (`.github/workflows/ci.yml`, push/PR to `main`):** jobs — `backend` (`cargo fmt --check` →
  `clippy --all-targets --all-features -D warnings` → `cargo test` unit → `cargo build --release`);
  `web` (`npm ci` → lint → typecheck → vitest → `vite build`); `drift` (regenerate OpenAPI +
  `openapi-typescript` → `git diff --exit-code`); `image` (multi-stage **Dockerfile**: cargo build →
  runtime image bundling the built SPA assets). *(iac + integration jobs added in Tasks 2 & 7.)*

**Acceptance / verification**
- CI green on all four jobs. `./scripts/dev.ps1` serves the SPA; health indicator shows green from a live
  `/api/health`. `docker build` produces a runnable image that serves both API and SPA on one port.
  OpenAPI doc reachable; drift check passes against the committed generated client.

### Task 2 — Cosmos preview-crate spike (de-risk design.md — Risks: Cosmos preview SDK)
**Scope**
- `infrastructure`: `CosmosTaskRepository` impl of `TaskRepository` over **`azure_data_cosmos` 0.36
  (pinned exact)**; serde mapping of `Task` ↔ Cosmos doc (`id`, `ownerId`, fields, `_etag`); point read
  by `(ownerId, id)`; single-partition list; **replace with `If-Match` → 412 → surfaced as 409**.
- **Prove the real etag round-trip** (create → read → update-with-matching-etag → update-with-stale-etag→
  409) against the **Cosmos DB emulator**.
- `scripts/docker-compose.yml`: Cosmos emulator (Linux container) [+ Azurite placeholder for Task 3];
  config loader wires `TASKLIST_ENV=local` → emulator (well-known emulator key), `cloud` →
  `DefaultAzureCredential`. Composition root selects the Cosmos repo over the in-memory fake when
  configured.
- `infrastructure/tests/`: integration test (create/read/update/etag-conflict) against the emulator.
- CI: add an `integration` job that starts the emulator (+ Azurite) as service containers and runs
  `cargo test` integration; keep it separate from the unit job.

**Acceptance / verification**
- Emulator etag round-trip test passes locally and in CI. The Task-1 skeleton path now round-trips a real
  Cosmos doc under `TASKLIST_ENV=local` with no change to domain/application code (DIP proven).

### Task 3 — Blob stub upload/download through `BlobStore` (Azurite)
**Scope**
- `domain`: `ImageRef` newtype; `Task.image: Option<ImageRef>`. `application`: `BlobStore` port + stub
  `tasks::set_image` use-case. `infrastructure`: `AzureBlobStore` over **`azure_storage_blob` 1.0**;
  blob key `{ownerId}/{taskId}/{imageId}`, container `task-images`, public access off.
- `api`: stub endpoints to PUT (upload bytes → key stored in `task.image`) and GET (stream back) an image
  through the port; body-size limit.
- `web/`: minimal image upload/download UI stub wired to the generated client.
- `docker-compose`: activate **Azurite**; `TASKLIST_ENV=local` points the blob adapter at it.
- `infrastructure/tests/`: integration test (put → get round-trip) against Azurite; add to CI integration
  job.

**Acceptance / verification**
- Upload a byte payload and read it back end-to-end through the `BlobStore` port against Azurite, locally
  and in CI. Blob-ref plumbing (`Task.image`) demonstrably flows through all layers.

### Task 4 — Auth0 + Google cookie BFF (OIDC + session)
**Scope**
- `api`: `openidconnect` auth-code + PKCE against Auth0 (scopes `openid profile email`, **Google social
  connection**); `tower-sessions` **encrypted cookie-stored** session (signing key from config/local
  secret now, Key Vault in Task 7); `GET /auth/login`, `GET /auth/callback` (validate ID token, upsert
  user keyed by `sub`, set cookie), `POST /auth/logout`; `GET /api/me`; `UserContext` extractor →
  `UserId`; CSRF token for mutations; cookie `SameSite=Lax` + `Secure` + `HttpOnly`.
- **Protect** the Task-1 trivial endpoint behind the auth middleware (401 when unauthenticated).
- `web/`: `AuthProvider` (session state via `/api/me`), `Login` page, `AuthCallback` route; header shows
  real signed-in identity; no tokens stored in the browser.
- Config: `AUTH0__DOMAIN/CLIENTID/AUDIENCE` (non-secret); **client secret + signing key** live only in
  git-ignored `dev-secrets.local.ps1` locally (Key Vault in cloud).

**Acceptance / verification**
- Log in via Auth0 (Google) → cookie set → protected endpoint returns owner-scoped data → `/api/me`
  reflects identity → logout clears session and redirects to Auth0 logout. **Prereq:** Auth0 tenant has
  the Google social connection enabled (design.md — Risks: auth session strategy).

### Task 5 — App Insights telemetry seam
**Scope**
- `infrastructure`: single `init_telemetry()` owning the exporter — `tracing` → `opentelemetry` →
  **`opentelemetry-application-insights` 0.45**; the seam so the sink is swappable to OTLP→Collector
  (design.md — Risks: telemetry exporter maturity). `api`: `tower-http` `TraceLayer` wired; health + request spans emitted with
  owner/route context.
- Local: telemetry **no-ops** when `APPLICATIONINSIGHTS_CONNECTION_STRING` is unset; cloud exports to App
  Insights. `web/`: optional `@microsoft/applicationinsights-web` client stub behind the same env.

**Acceptance / verification**
- With the connection string set, request/health spans reach App Insights (or the console exporter in
  dev); with it unset, the app runs clean with telemetry disabled. Single seam, no exporter types leaking
  past `infrastructure`.

### Task 6 — Installable PWA shell + owner-scoped Dexie stub
**Scope**
- `web/`: **`vite-plugin-pwa`** (Workbox `generateSW`) — precache the built shell
  (`js,css,html,svg,png,ico,woff2`), `navigateFallback: '/index.html'`, `cleanupOutdatedCaches: true`,
  **`navigateFallbackDenylist`: `/^\/api\//`, `/^\/auth\//`, `/^\/swagger/`** (must not hijack the Auth0
  callback — the service-worker/OIDC-callback hijack landmine), `registerType: 'prompt'` + update toast **gated on an empty
  outbox**; complete manifest (`id`/`name`/`short_name`/`start_url`/`scope`/`display:standalone`/theme +
  icons with both `any` and `maskable`). **No runtime caching of `/api`.**
- `web/src/offline/`: minimal owner-scoped **Dexie** stub — `db.ts` schema (`tasks`/`outbox`/`blobs`/
  `meta`), each content record carries `owner`; `store.ts` with mirror/outbox primitives + **clear-on-
  logout/owner-change**. **No full sync** (pull/push/flush is a later feature) — just proves the
  offline-first scaffolding + teardown exist.

**Acceptance / verification**
- App is installable (Lighthouse PWA pass) and boots offline (shell only); SW never serves cached
  `index.html` for `/api/*`, `/auth/*`, `/swagger`; Dexie DB is created owner-scoped and cleared on
  logout. Confirms the two-layer split (Workbox shell vs app-managed data) from design.md — Offline-first & PWA.

### Task 7 — IaC (fresh Bicep) + managed identity + Key Vault + single Production deploy
**Scope**
- **`.arm/`** (directory decision — Mr. Das ruled `.arm/`, see Notes & Decisions): `main.bicep` +
  `prod.main.bicepparam` + `bicepconfig.json`.
- Resources (design.md — DevOps): **Azure Container Apps** (single container = API + SPA, external HTTPS
  ingress, min-replicas 0 scale-to-zero, **system-assigned managed identity**) + managed environment +
  **Log Analytics**; **Cosmos DB** (Core/SQL, **serverless**, Session consistency, db `tasklist`,
  container `tasks` PK `/ownerId`, data-plane RBAC Built-in Data Contributor to the app identity — no
  keys); **Storage** (StandardV2, container `task-images`, public access off, soft-delete, Blob Data
  Contributor to the app identity); **App Insights** (workspace-based); **Key Vault** (Auth0 client
  secret + session signing key surfaced as **Container Apps secrets via Key Vault references**, RBAC Key
  Vault Secrets User).
- **Secrets/identity wiring:** managed identity for Cosmos + Blob data-plane (`DefaultAzureCredential`),
  Key Vault references for the two secrets as Container Apps secrets — **no secrets in env or repo**.
  Non-secret runtime config (Auth0 domain/clientId/audience, endpoints, App Insights connection string)
  set by Bicep at deploy.
- **CI:** add `iac` job — `bicep build` + lint + `az deployment group validate` (prod params) [+
  `what-if`]. `.github/workflows/deploy.yml` (**Production only**, GitHub Environment approval, Azure OIDC
  federated login): build+push image (ACR/GHCR) → `az deployment group create` (main.bicep +
  prod.main.bicepparam) → roll a new Container Apps revision → smoke-probe `/api/health`.

**Acceptance / verification**
- `bicep build`/lint + `az deployment group validate` (and `what-if`) pass in CI. **The actual Production
  deploy is Mr. Das's to run** (agents never deploy — guardrail #4); `deploy.yml` is authored and
  validated but not triggered by an agent.

### Task 8 — Repo-wide contradiction & residue sweep (FINAL)  *(Mr. Das's required last task)*
**Goal:** a reconciled repo — zero stray Nucleus/.NET/C#-specific residue in *live* artifacts, and zero
internal contradictions across docs/code/scripts/CI. Output is the reconciled repo + a short reconciliation
report.

**(a) Residue markers to grep-hunt (case-insensitive) and eliminate from live files:**
- **.NET / C# toolchain:** `dotnet`, `.csproj`, `.sln`, `\.cs\b`, `MediatR`, `EntityFramework`/`EF Core`,
  `nswag`, `FluentValidation`, `AutoMapper`, `xUnit`, `dev-certs`, `Nucleus.sln`, `msbuild`,
  `TreatWarningsAsErrors`.
- **Nucleus domain residue:** `Nucleus`, `Dilligenz`/`Diligenz`, `\bSVP\b`, `valuation`, `MIS`,
  `Checklist`, `LocationId`, `site visit`/`site engineer`.
- **Bicep-Nucleus / infra residue:** any old `.arm/` resource names, Nucleus-specific param/resource
  identifiers, staging-stage references (we are single Production).
- **Trunk naming:** `\bmaster\b` (should be `main` everywhere).
- Known current hits (from the planning scan, for reference): `.editorconfig`, `.gitignore`, `dev.ps1`,
  `run-app.ps1`, `.github/skills/*`, `.github/agents/jarvis.md`, `.github/copilot-instructions.md`,
  `docs/design.md`. *(Tasks 1–7 rewrite most of these; the sweep verifies nothing was missed.)*
- **No historical-reference exception (Mr. Das override, 2026-07-08):** the former carve-out that kept a
  legacy-stack → Rust concept map and process-note prose as "comparative/historical" is **revoked** — the
  docs consolidation removed that prose, so the sweep now eliminates **every** live hit of these markers
  (none are exempt as "historical"). `scripts/dev-secrets.local.ps1` is git-ignored (not in the repo) — out of scope.

**(b) Internal-contradiction reconciliation:**
- **Guardrail file `.github/copilot-instructions.md` (sanctioned edits — confirm with Mr. Das/JARVIS as
  it is the playbook):** #3 `master` → **`main`** (trunk is `main`); #5 `*.generated.cs` + nswag →
  **utoipa OpenAPI → `openapi-typescript` generated TS client** (`web/src/ApiClient.generated.ts`; drop
  the `.cs`); confirm #1's `docs/design.md` exists (authored Task 1).
- **Agent files** (`.github/agents/*.md`): reconcile any `master` → `main`.
- **design.md ↔ this spec:** ✅ RESOLVED — IaC dir ruled `.arm/` (matches design.md — Directory layout / DevOps);
  no reconciliation needed. Verify Task 7 shipped Bicep under `.arm/` (not `infra/`).
- **Skills ↔ scripts ↔ CI:** the build/test/run commands in `.github/skills/*` match the rewritten
  `scripts/*` and `ci.yml` (cargo + Vite, not dotnet).
- **README ↔ design.md:** stack, feature list, and hosting all agree (Rust/axum, React
  19, Container Apps, Cosmos serverless, Auth0+Google cookie BFF, offline-first PWA).

**Acceptance / verification**
- Grep for each marker above returns **only** intentional-historical hits (documented in the report). The
  guardrail reconciliations are applied (with sign-off). `master`/`main`, generated-client, and toolchain
  references are consistent across every file. Reconciliation report lists every change + every
  deliberately-retained reference with justification.

## Global Refactoring Log

_(Dave appends cross-cutting refactors here as tasks land — e.g. port-signature changes, shared-dep
version bumps, config-loader evolution when the in-memory repo is swapped for Cosmos in Task 2.)_

## Review Log

### Task 1 — verification & design review (2026-07-07)
- **Bhaskar (verify): PASS.** Backend gate (fmt / clippy `-D warnings` / **19 tests**) ✓; full frontend CI
  (lint / typecheck / vitest 5 / build) ✓; OpenAPI drift byte-identical ✓; **`--no-cache` Linux `docker
  build` ✓ and the container runs API+SPA on one port** (health 200, DIP `/api/tasks` 200).
- **Dave (stabilize) — defects fixed during Task 1:** (a) `dev.ps1` launched Vite via a bare `npm` shim
  that aborted the launcher before cargo ran → `npm.cmd`; (b) `run-app.ps1` 20-min stale→ready coercion
  thrash-restarted the watch → removed, explicit `building`/`broken` now honored; (c) root manifest
  `cargo.toml` → **`Cargo.toml`** (was breaking Linux CI + docker build).
- **Anders (design review): APPROVE-WITH-SUGGESTIONS — nothing blocks.** Layering exact
  (`domain ← application ← infrastructure/api`), both DIP seams genuinely exercised (Clock on health,
  TaskRepository on `/api/tasks`, `owner` passed as a param so Cosmos/Auth swaps touch no `application`
  code), all seams real (RFC7807, utoipa→openapi-typescript+drift gate, `TASKLIST_ENV`, SPA fallback),
  **zero throwaway scaffolding**. Findings & pending decisions below.

**Anders findings (Task 1):**
- **F1 (MEDIUM, fix now):** `scripts/set-dev-state.ps1` (lines ~14–15) comment still documents the deleted
  20-min coercion — a live doc-vs-code contradiction. Align once decision (ii) is ruled.
- **F2 (LOW, Mr. Das's call):** `utoipa::ToSchema` derives live on `application` DTOs, while design.md's
  crate-responsibilities table assigns utoipa to `api`. Passive compile-time metadata, no framework
  coupling — accept + clarify the layering rule, or move `ToSchema` wrappers to `api` (more boilerplate).
- **F3 (LOW):** `[profile.release] debug = true` bloats the container image (works against scale-to-zero
  cold start); `strip`/split-debuginfo in the Docker runtime copy — keeps App Insights symbolication.
- **Later-task watch-items (no action now):** Task 2 error seam must grow `Conflict` (412→409) + `ETag`
  field; Task 4 swaps `AppState.demo_owner` → `UserContext` extractor (clean); **Task 6 PWA MUST set
  `navigateFallbackDenylist` for `/api/`,`/auth/`,`/swagger`** (the service-worker/OIDC-callback hijack landmine); config
  loader hand-rolled for now (revisit `figment`/`config` when Cosmos/Auth0 settings arrive).

**✅ Decisions RULED by Mr. Das (2026-07-08):**
1. **(i) Health body:** KEEP `{status,time}` (Clock-port DIP proof). The merged design doc states it as the
   authoritative health contract; reconcile the bootstrap.md `{status:"ok"}` shorthand.
2. **(ii) Liveness safety valve:** Anders' rec — **lifecycle reset**: `session-startup.ps1` clears a stale
   `building`/`broken` → `ready` on fresh session boot (no time-based coercion). F1 comment aligned to match.
3. **(F2) Layering:** KEEP idiomatic — `ToSchema` allowed on `application` DTOs; clarifying line added to the
   layering rule (design.md — Hard rules / crate responsibilities). (F3 strip-symbols: deferred to Task 5/7 where symbolication is set up.)

**📄 Docs consolidation directive (Mr. Das 2026-07-08):** merge the two design docs into ONE authoritative
`docs/design.md`; **drop ALL C#/.NET & Nucleus references from every doc** (this OVERRIDES the earlier
Task-8 "keep intentional historical concept map" exception — Mr. Das wants them gone); **drop library/version
references from the architecture content** (patterns/seams stay; concrete crate + version choices live in
Cargo.toml + the feature specs). Flow: Anders plans → Dave implements (incl. updating every inbound
reference) → Bhaskar verifies (no broken links / no residue) → Anders reviews.

### Docs consolidation + governance scrub (2026-07-08)
- **Dave:** merged `docs/architecture.md` → `docs/design.md` (deleted architecture.md); purged all C#/.NET/Nucleus;
  de-libraried (core stack Rust·axum·React·TS·Vite kept, versionless; all else role-described → Cargo.toml/package.json);
  inserted the two rulings; fixed every inbound ref; `master→main` in docs; added the README "agentic loop" section + personas.
- **JARVIS (sanctioned, Mr. Das-approved):** scrubbed governance files — `copilot-instructions.md` #3 `master→main`,
  #5 `*.generated.cs`/nswag → utoipa→openapi-typescript; `agents/jarvis.md` dotnet→cargo/Vite, ADO PAT→`gh` CLI,
  wrong `dilligenzvaluation/nucleus` pipeline URL → GitHub Actions, stage→prod → single Production, `master→main`;
  `agents/anders.md` `master→main`. (Trunk confirmed = `main`.)
- **Bhaskar: PASS** — zero dangling `architecture.md` refs, residue confined to sanctioned bootstrap.md marker/playbook strings,
  all links resolve, both rulings present, core stack versionless with no crate leakage, `cargo build` green.
- **Anders: APPROVE-WITH-SUGGESTIONS** — faithful/coherent merge; three critical risks retained their teeth; ToSchema wording
  **ruled accept-as-is**; P1–P4 optional polish only (P1: one-line CORS reconcile in rule #7/hardening — non-blocking).

## Notes & Decisions

**Settled rulings (Mr. Das 2026-07-06, now in `docs/design.md` — carried into this spec):**
- **Auth:** cookie-based **server-side session (BFF)** with **Auth0 OIDC + Google social connection**;
  SPA stores no tokens. Encrypted cookie-stored session (signing key from Key Vault) — survives Container
  Apps scale-to-zero / multi-replica; Cosmos-backed store is the scale-out alternative. (design.md — Application-wide patterns: auth/session; Risks)
- **Persistence:** **preview `azure_data_cosmos` 0.36 accepted as the primary adapter**, kept behind the
  `TaskRepository` port as the swap seam; emulator etag round-trip must be proven in Task 2 before
  building on it. (design.md — Risks: Cosmos preview SDK)
- **Hosting:** **Azure Container Apps** (scale-to-zero), single container serving API + SPA. (design.md — DevOps; Risks)
- **Secrets/identity:** **managed identity** for Cosmos + Blob data-plane (keyless, prod); **Key Vault**
  for the Auth0 client secret + session signing key, surfaced as Container Apps secrets via **Key Vault
  references**; **no secrets in env or repo**; emulator/Azurite well-known creds only locally. (design.md — Application-wide patterns: config/secrets)
- **Local dev:** Cosmos **emulator** + **Azurite** via docker-compose as the first-class local backing
  store; `TASKLIST_ENV=local|cloud` toggle; `/api/health` contract + `set-dev-state.ps1`/`.dev-state.json`
  lifecycle preserved (liveness watch + Dave guardrail #8 depend on them). (design.md — DevOps: local dev)
- **Offline-first PWA:** in scope, no fundamental clash with per-user Cosmos; two-layer split (Workbox
  app-shell vs app-managed Dexie data engine); SW **denylists** `/api/*`, `/auth/*`, `/swagger`; bootstrap
  ships the installable shell + Dexie stub only (full pull/push/flush is a later feature). (design.md — Offline-first & PWA)
- **Conflicts:** **last-writer-wins scoped to the owner's own devices** (etag/`If-Match` → 412→409 →
  re-pull → explicit retry re-stamps fresh etag); no CRDT/field-merge (YAGNI). Not exercised in bootstrap
  beyond the etag round-trip. (design.md — Offline-first & PWA: conflict resolution)
- **Client offline id minting:** offline-created tasks get a **client-side uuidv7 id** = Cosmos doc id →
  idempotent first-sync create. (design.md — Offline-first & PWA: offline id minting) — scaffolding present via the Dexie stub in Task 6.

**Decisions made in this spec (please confirm / veto):**
- **IaC directory = `.arm/`** — **RULED by Mr. Das 2026-07-07** (chose zero churn on the finalized
  `docs/design.md`, which specifies `.arm/` under Directory layout / DevOps). Bicep files live under `.arm/`.
  **Consequence:** no design.md reconciliation needed in Task 8 for this item; Task 7 authors Bicep
  under `.arm/`. *(The `infra/` alternative — azd-convention rename — was considered and declined.)*
- **Skeleton repo swap order:** Task 1 uses the **in-memory** `TaskRepository`; Task 2 swaps in Cosmos.
  This is deliberate so DIP is provable before the preview SDK is introduced.

**Open items for Mr. Das to confirm before Dave starts Task 1:**
1. **Auth0 tenant:** Google social connection enabled + a callback/app registration available (needed
   Task 4, but provision early).
2. **IaC directory:** ✅ RESOLVED — `.arm/` (Mr. Das, 2026-07-07). See decision above.
3. **Toolchain pins:** Rust ≥ 1.88 (required by `azure_data_cosmos` driver) and Node 20/22 LTS — confirm
   the CI/dev toolchain versions.
4. **Docker locally:** the Cosmos DB **Linux** emulator has known feature/stability caveats; confirm
   Docker Desktop is available and accept that Task 2 may need the emulator's documented limitations
   worked around (fallback: a dedicated dev Cosmos account).
5. **Guardrail edits in Task 8:** editing `.github/copilot-instructions.md` (#3 master→main, #5
   nswag→TS-client) touches the agent playbook — confirm the sweep may apply these, or route them through
   JARVIS.
6. **Image registry:** ACR vs GHCR for the pushed container image (Task 7 / deploy.yml).
