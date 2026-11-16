# Progression development handoff

- Before changing this fork, read `DEVELOPMENT_LOG.md`, `mistakes.md`, `PROGRESSION.md` and upstream `CONTRIBUTING.md`. Verify historical claims against the current branch and CI when practical.
- Keep `DEVELOPMENT_LOG.md` current after meaningful implementation, design, commit and validation milestones. Separate implemented, tested and still-unverified behavior; record the tested revision and CI/artifact links.
- Record confirmed mistakes in `mistakes.md`: symptom, evidence, cause, fix, prevention and validation. Update the existing entry when the same issue recurs rather than rediscovering it or creating contradictory entries.
- Preserve upstream calculation behavior. Progression uses the existing PoB Lua engine; do not invent DPS or replace it with TypeScript/Rust formulas.
- Keep snapshot capture isolated from the live build and existing Compare state. Preserve backup/schema failure handling and the distinction between target-build comparison and a simulated partial equipment change.
- Do not add game-memory access, session-cookie collection or gameplay automation.
- Run the checks appropriate to the change. Frontend fixture tests are not native-engine validation; a compiled installer is not proof of desktop interaction or gameplay correctness. Do not label skipped tests as passed.
- Work on the feature branch. Do not merge or publish a production release as a side effect of validation.
