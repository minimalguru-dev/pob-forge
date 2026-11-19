# Progression (PoE1 preview)

Progression adds a local record of where a character is and the build they are aiming for. It compares complete snapshots and orders the work needed for a transition. It does not treat the target build's DPS as the result of putting one target item on the current character.

## Use

1. Select **Path of Exile 1**, then **Progression** (also in the command palette).
2. Choose **Current**, give the snapshot a name, and capture the open build, paste a PoB code/XML/supported URL, or open a PoB XML file. Importing here does not replace the open build.
3. Repeat with **Target**. For example, capture Shield Crush Slayer as current and a Dual Strike of Ambidexterity Slayer loadout as target. Supply your own builds; the preview contains no invented character stats or guide-specific item requirements.
4. Select the intended tree, item set, skill set and main skill in PoB before exporting. The snapshot records those active selections. The existing Compare tab remains available for experimenting with other loadouts.
5. Read detected blockers first, then the equipment, gem and tree preparation. Use **Optimize** and **Compare** for individual item experiments. Those buttons open the existing tabs for the live build; they do not load or change a Progression snapshot.
6. Capture again after a change. History is deliberately explicit, not an automatic character tracker. Select any two snapshots as Current and Target to inspect historical changes.
7. Export history for a portable JSON backup. Import merges by snapshot ID. Remove individual snapshots when no longer needed; a selected snapshot's removal clears that selection.

## What is calculated

`progression_capture` is one serialized, read-only engine call. It creates a temporary upstream `CompareEntry`, runs PoB's Lua calculations, extracts a snapshot, applies a boss scenario on that same temporary entry and recalculates. It never switches the live build, alters its XML, or changes the existing Compare list. The engine's shared calculation cache may be invalidated, as with upstream Compare.

Each snapshot contains:

- Original XML, capture time, game, PoB version and active tree version.
- Level, class, ascendancy, selected main skill and active loadout identifiers.
- Equipped item text and selected variants; matching item names alone are not considered equal.
- Allocated node IDs/names and mastery selections. Cross-version/class node diffs are withheld. Counts include allocated nodes and must not be interpreted as exact refund costs. Cluster jewels and tattoos still require review in the tree UI.
- Enabled gems, gem levels/quality, PoB gem errors, scalar config inputs and selected raw calculation outputs.
- The boss scenario's exact config overrides and raw PoB outputs. Missing outputs are shown as **—**, never synthesized as DPS.

Configured and boss figures are **solo CompareEntry calculations**. Upstream CompareEntry does not load party buffs. They can differ from the live sidebar for a party build. The regression test covers parity for a solo fixture, not every PoB feature.

## Readiness and priorities

Checks identify base-class incompatibility, PoB attribute requirement shortfalls, gem errors and resource-cost warnings. Level, ascendancy, target gear/gems, tree changes, resistance caps and configuration differences require review. Missing stats remain unknown. The final manual review is always present: this module cannot know ownership, currency, refund availability, socket colours, ailment protection, combat recovery or sustainable charge generation.

Priorities are transparent rules: detected blockers, comparability of the inputs, then preparation of equipment, gems and tree. Changed target slots are listed individually; weapons receive early review. This is **not** a market-price ranking, an optimizer search or an estimate of DPS gained per currency. It intentionally does not produce a readiness percentage or certify that a build is safe to switch.

## Boss scenario

Choose Standard Boss, Guardian/Pinnacle or Uber Pinnacle before capture. On the temporary copy only:

- Set PoB's `enemyIsBoss` to `Boss`, `Pinnacle` or `Uber`.
- Disable configured frenzy, power and endurance charges, configured Onslaught and Rampage.
- Disable kill conditions/counts defined by the pinned PoB config options.
- Disable activation of the five flask slots.

All other imported assumptions remain, including custom mods, explicit enemy damage/resistance overrides, boss skill, rage, shock, guard skills and Full DPS skill counts. Buffs granted automatically by PoB mechanics may remain. This is a reproducible conservative **scenario**, not a measured or guaranteed sustainable DPS model. Review Config and the displayed overrides. Different boss categories are flagged; re-capture both sides to compare the same scenario.

## Local data and boundaries

History is stored by the app in `progression-poe1.json` in its local data folder, so the Progression tab, the overlay and the assistant read the same plan. History saved by earlier previews under the local-storage key `pob-forge:progression:poe1:v1` is moved there once; the old copy is left in place. It stores up to 20 snapshots and a maximum serialized size of 4 MiB. There is no silent eviction: a full or oversized history is refused and the previous plan remains intact. Invalid or unsupported history is not overwritten; it is reported and capture stays disabled. Backup imports are untrusted historical data: re-capture their XML to verify numbers with the current engine.

History includes full build XML, which can contain character/build names and notes. It stays local unless the user exports it. Progression adds no telemetry or automatic network requests. Explicit URL imports use the app's existing supported fetchers. No new GGG API client, session-cookie collection, game-memory access, screen capture, game-file modification or input automation is added. The opt-in in-game overlay reads the clipboard only while it is showing, and only acts on item text the game copied; see [OVERLAY.md](OVERLAY.md). GGG's [third-party policy](https://www.pathofexile.com/developer/docs#policy) remains authoritative; this is not a claim of GGG endorsement.

The preview disables PoE account authorization until the fork has an appropriate OAuth registration. Code/XML/URL imports do not require that account. Application credentials use the actual application identifier. Codex uses a separate preview profile and requires the operating-system credential store; it does not inherit the user's OpenAI API-key environment. Sign in through the assistant settings for this profile. AI requests can send conversation and accessed build data to the selected provider. Other vendor CLI profiles are not covered by the Codex isolation change. See [SECURITY_REVIEW.md](SECURITY_REVIEW.md) for the review scope and remaining limitations.

The Import tab now shows clipboard and import failures inline. A click retries a temporarily unavailable clipboard at most three times; an empty clipboard gives an explicit message. Cache writes retain the operating-system error, and tree-art cleanup preserves in-progress downloads. Downloads are verified against pinned art manifests; these changes do not substitute artwork or alter the calculation engine.

The MCP server remains opt-in, token-protected and loopback-only as upstream implements it. In Path of Exile 1 the assistant and MCP clients get three Progression tools: `get_progression` reads the plan with its differences, checks and priorities; `capture_progression_snapshot` saves the open build as the current or target snapshot; `select_progression_snapshots` picks the pair. None of them changes the open build. Keep and Undo in the chat do not remove a snapshot; remove it in the Progression tab.

## Development and validation

Follow [CONTRIBUTING.md](CONTRIBUTING.md). The fork retains the upstream MIT licence and pinned PoB commits in `pob-sync.toml`. No vendored PoB data or calculation formula is edited.

```sh
bun install --frozen-lockfile
bun run check
bun test scripts
bun x playwright install chromium
bun run test:progression:ui
bun run build
bun run sync
cargo test -p pob-engine
cargo test -p pob-forge --lib
```

Run the real-engine test after syncing PoE1. In PowerShell:

```powershell
$env:POB_FORGE_POB_ROOT = 'src-tauri/resources/pob1'
cargo test -p pob-engine --test progression -- --ignored --nocapture
```

The integration test creates its own temporary build and user directory. It checks configured-stat parity, exact boss-scenario parity against direct PoB config changes, successful and failed capture non-mutation, preservation of Compare state and repeatable XML import. It does not use player data. Unit tests cover item rolls/variants, tree/mastery differences, unknown stats, ordering, history round trips, corruption and storage failures. The private build corpus is not bundled; the golden suites require the owner's corpus key.

`.github/workflows/progression-windows.yml` provides a Windows build with pinned PoB data, frontend checks, frontend unit tests, Rust regressions and the real-engine integration test. It uploads an **unsigned validation installer** as a GitHub Actions artifact; it does not publish a production release. A dedicated Tauri config gives it a separate application identity and URL scheme, and `VITE_PROGRESSION_VALIDATION=true` disables upstream auto-updates so they cannot replace the preview. PoB's normal build folder remains shared, so use test copies when editing builds through other tabs.

For a local validation installer with the prerequisites installed:

```powershell
Remove-Item Env:NoDefaultCurrentDirectoryInExePath -ErrorAction SilentlyContinue
$env:VITE_PROGRESSION_VALIDATION = 'true'
bun run tauri build --bundles nsis --config src-tauri/tauri.progression.conf.json
```

The Windows CI also removes that environment variable in each Cargo process: an empty value still prevents LuaJIT's build script from finding its local `minilua.exe`. Keep patch files as LF as prescribed by `.gitattributes`.

Preview code revision `90cd6bb` passed [Linux validation](https://github.com/minimalguru-dev/pob-forge/actions/runs/43718155297) and [Windows validation](https://github.com/minimalguru-dev/pob-forge/actions/runs/43718149574), including real-PoE1 parity/non-mutation tests on both platforms. The Windows run produced the [unsigned NSIS installer](https://github.com/minimalguru-dev/pob-forge/actions/runs/43718149574/artifacts/14164402860). This does not certify native desktop interaction or an actual player's build. See DEVELOPMENT_LOG.md for exact counts, hash and unavailable checks.

The Progression tab label participates in upstream localization; new preview explanations currently use English. Complete translation, per-item counterfactual simulation, budget/market integration and automated character synchronization are outside this minimal module.
