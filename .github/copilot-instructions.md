# copilot-instructions.md — Agent playbook

This file is the source of truth for any AI agent working in this repository.

All agents must address me, the human, as Mr. Das, an alt of Iron Man. Or Sir or something similar.

## Golden rules (guardrails)

0. All agents:
   - Must be crisp and to the point in communication with the human.
     - Bring out high signal, salient points, and avoid verbosity. Avoid repeating the human's words back to them.
   - Don't assume. Don't hide confusion. Surface tradeoffs.
   - State your assumptions explicitly. If uncertain, ask.
   - If multiple interpretations exist, present them - don't pick silently.
   - If a simpler approach exists, say so. Push back when warranted.
   - If something is unclear, stop. Name what's confusing. Ask.
1. Always reload and understand the high level design and architecture of the system from `docs/design.md`.
2. Separation of duties (strict). Do not cross these lanes.
3. Never touch `main`. Work on a branch named `vibe/<feature_name>`.
4. Never deploy.
5. `web/openapi.json` and `web/src/ApiClient.generated.ts` are auto generated (utoipa OpenAPI document → `openapi-typescript`).
   - Never edit these files directly.
6. Stop and ask when a task needs a product/architecture decision. That call belongs to the human architect.
7. Human can invoke any agent on demand.
8. For frontend tests:
   - Definitely write unit tests for business logic.
   - Some unit tests for deep UI components / rendering are okay, but don't overdo.
   - Tests that cause timing issues should be avoided.
9. For backend tests:
   - Unit tests: Fine grained unit tests for all business logic and critical ones only for scaffolding.
   - Integration tests: Add for the critical paths, not every code path. Don't overdo.

