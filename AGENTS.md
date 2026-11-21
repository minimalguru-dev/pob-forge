# Fork development handoff

This fork focuses on the AI assistant and the in-game overlay. The Progression module was removed on 2026-11-17 (see `DEVELOPMENT_LOG.md`).

- Before changing this fork, read `DEVELOPMENT_LOG.md`, `mistakes.md`, `OVERLAY.md` and upstream `CONTRIBUTING.md`. Verify historical claims against the current branch and CI when practical.
- Keep `DEVELOPMENT_LOG.md` current after meaningful implementation, design, commit and validation milestones. Separate implemented, tested and still-unverified behavior; record the tested revision and CI/artifact links.
- Record confirmed mistakes in `mistakes.md`: symptom, evidence, cause, fix, prevention and validation. Update the existing entry when the same issue recurs rather than rediscovering it or creating contradictory entries.
- Preserve upstream calculation behavior. The assistant and overlay use the existing PoB Lua engine; do not invent DPS or replace it with TypeScript/Rust formulas.
- Evaluations (item and map checks, trade searches) must leave the open build, its undo history and Compare state unchanged.
- Do not add game-memory access, session-cookie collection or gameplay automation. Buying and whispering on trade stay manual.
- The only input the app may send to the game, only while the game is in front and one per key press of the user's (GGG's one-action-per-key-press rule): a user-configured macro typing one chat command starting with `/`, and the overlay price check's single Ctrl+Alt+C (the game's advanced copy, with mod tiers and roll ranges) right after the price shortcut (owner's decision, 2026-11-19; Ctrl+Alt+C instead of Ctrl+C, 2026-11-21). No sequences of game actions, no clicks, no timers.
- Run the checks appropriate to the change. Frontend fixture tests are not native-engine validation; a compiled installer is not proof of desktop interaction or gameplay correctness. Do not label skipped tests as passed.
- Work on the feature branch. Do not merge or publish a production release as a side effect of validation.
