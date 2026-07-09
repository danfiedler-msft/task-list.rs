---
name: build-and-test-full
description: Full build — unit tests, release build, and the OpenAPI client drift check.
---

## Commands

### Backend (Rust)

From the repository root. Zero warnings is a hard gate — clippy and the release build both treat
warnings as errors. Secrets are not required for the unit suite today (the walking skeleton uses the
in-memory adapters); dot-source them first for parity with later integration work.

    . ./scripts/dev-secrets.local.ps1
    cargo fmt --all -- --check
    cargo clippy --all-targets --all-features -- -D warnings
    cargo test
    cargo build --release

### Frontend (web)

From `web/`.

    npm ci
    npm run lint
    npm run typecheck
    npm run test:ci
    npm run build

### OpenAPI client drift check

The typed web client (`web/src/ApiClient.generated.ts`) is generated from the API's OpenAPI
document and committed. Regenerate both and confirm there is no diff — a diff means the API changed
without regenerating the client.

From the repository root:

    cargo run -p tasklist-api -- openapi > web/openapi.json
    npm --prefix web run gen:api
    git diff --exit-code web/openapi.json web/src/ApiClient.generated.ts
