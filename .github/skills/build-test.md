---
name: build-and-test
description: Builds the project and runs unit tests (no integration tests). Used for fast feedback.
---

## Commands

### Backend (Rust)

From the repository root. Zero warnings is a hard gate — clippy runs with `-D warnings`.

    cargo fmt --all -- --check
    cargo clippy --all-targets --all-features -- -D warnings
    cargo test

### Frontend (web)

From `web/`.

    npm run lint
    npm run typecheck
    npm run test:ci
    npm run build
