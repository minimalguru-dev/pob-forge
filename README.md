# PoE Forge

[![Latest release](https://img.shields.io/github/v/release/minimalguru-dev/pob-forge?style=flat-square&label=release)](../../releases/latest)
[![Downloads](https://img.shields.io/github/downloads/minimalguru-dev/pob-forge/total?style=flat-square&label=downloads)](../../releases)
[![License: MIT](https://img.shields.io/badge/license-MIT-2a2a30?style=flat-square)](LICENSE)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS-2a2a30?style=flat-square)

A desktop app for planning Path of Exile 1 and 2 builds. It runs Path of Building Community's own
calculation code, so every number matches Path of Building, and it opens the same build files.

## Download

Every installer is on the [Releases](../../releases/latest) page.

| System | File | Install |
|---|---|---|
| Windows 10 or 11 | `PoE.Forge_<version>_x64-setup.exe` | Run it. It installs for your user only and fetches WebView2 if Windows lacks it. |
| macOS, Apple Silicon | `PoE.Forge_<version>_aarch64.dmg` | Drag the app to Applications. See [macOS says the app cannot be opened](#macos-says-the-app-cannot-be-opened). |

There is no build for Linux or Intel Macs.

Windows releases are signed through SignPath Foundation's free programme for open source, once the project is
approved; until then they are unsigned. See the [code signing policy](CODE_SIGNING_POLICY.md). Free code signing
provided by [SignPath.io](https://about.signpath.io/), certificate by [SignPath Foundation](https://signpath.org/).

## Getting started

1. On first start, pick Path of Exile 1 or 2. The toggle in the title bar switches games later.
2. Open the **Builds** tab. Your saved builds are listed there, and **New build** starts an empty one.
3. To bring in someone else's build, paste it into the **Import** box and press **Import**.

Import takes:

- a Path of Building share code or build XML
- a link from pobb.in, Maxroll, Mobalytics, poe.ninja, poe2db, poedb, Pastebin or Rentry
- a Maxroll build guide link, when the guide's author attached a PoB code

**Open file…** opens a saved `.xml` build, or on Path of Exile 2 a GGG Build Planner file (`.build`).

A build from the other game switches the app to that game first.

Builds are saved in PoE Forge's own folder, and the Builds tab also lists Path of Building's folder under
"Path of Building", so builds saved there stay where you can open them:

- Path of Exile 1: `Documents/PoE Forge/Builds/PoE1` (and `Documents/Path of Building/Builds`)
- Path of Exile 2: `Documents/PoE Forge/Builds/PoE2` (and `Documents/Path of Building (PoE2)/Builds`)

**Browse** on the Builds tab picks another folder; **Reset** goes back to PoE Forge's.

## Updating

When a new version is out, a banner appears at the top of the window.

Press **Update**, then **Restart** if the app asks.

To check by hand, open Settings (the gear in the title bar, or Ctrl+,) and press **Check for updates**.

## Features

### Every Path of Building tab

Tree, skills, items, calcs, config, notes, party and builds are all here, with PoB's tooltips and
breakdowns. Calcs sorts PoB's sections into offence, defence and other, with DPS, EHP and the
largest hit you can take at the top. Your buffs and the curses on the enemy show as game icons.

Ctrl+K (Cmd+K on macOS) opens a search for commands, tabs, saved builds and everything in the open
build: notables and keystones, items, skills, config options and calcs rows. Pick one to go straight to it.

### Optimize

Reviews the build and lists what is wrong, worst first, with a fix for each. It can
design a rare for any slot from the real mod pool, and it ranks passive nodes by what each one adds
per point. Every figure comes from PoB's own calculation, and it needs no account or key. The search is best
effort and still being improved, so check each suggestion against your build before you keep it.

### Compare

Loads a second build next to yours and shows the differences in stats, tree, items,
skills and config.

### Assistant

An optional chat panel that answers questions about the open build and can change it. It works in
both games; Path of Exile 1 support is new and marked beta. Open it from the chat icon in the status bar or with Ctrl+L. It runs
through a coding agent on your computer, signed in to your own plan: Claude Code, Codex, GitHub Copilot, Cursor,
Grok, Kimi Code, OpenCode or Google Antigravity (which the app downloads for you). It can also use a model
you run with Ollama.

Sign in once with the agent's own command (for example `claude auth login` or `codex login`); Antigravity signs in
with Google from Settings > Assistant. The app never sees
your login: it starts the agent, which uses its own. Your plan's usage limits apply. The agent works in an empty folder, its shell and file-editing tools are turned off or
refused, and it is told to use only the build's tools.

Ask a question and it answers; ask for a change and it makes it. Each reply that changed the build ends with Keep
and Undo, so one click puts everything back. Turn on "Ask before each change" in Settings > Assistant to approve
every change first.

### In-game overlay

An optional panel over the game, off until you turn it on in Settings > Overlay. Ctrl+C on an item shows what it does
to the open build in every slot it fits; Ctrl+C on a map shows which of its mods are dangerous for the build.
Ctrl+D prices the item under the cursor from poe.ninja and the trade site, with mod filters you can change. It reads
only what you copy and the game's log file; the only keys it presses are your chat macros and the price check's
Ctrl+Alt+C. See [OVERLAY.md](OVERLAY.md).

### Search regexes

The Regex tab builds text for the game's search box: tick what you want highlighted (currency, divination cards,
fragments, omens, uniques…) and a minimum price, and it writes the shortest regex that lights up everything worth that
much on poe.ninja and nothing else poe.ninja lists, within the search box's character limit. Use it in the Ritual
window or a stash. Copy it and paste it into the game yourself; regexes you build or write yourself can be saved.

### MCP server

Lets an AI client such as Claude Code, Claude Desktop or Cursor read and edit the open build, in
either game (Path of Exile 1 is in beta). It is off until you turn it on in Settings, which then shows
the address, an access token and ready-made client settings. It listens on `127.0.0.1` only, stops
when the app closes, and refuses any request without the token.

## Troubleshooting

### macOS says the app cannot be opened

The app is not signed with an Apple Developer ID or notarized yet, so macOS blocks the first launch.
On macOS 15 or later, open System Settings → Privacy & Security and click **Open Anyway**. On macOS
14 and earlier, right-click the app and choose **Open**. [MACOS.md](MACOS.md) has more detail.

### An API key will not save on Windows

When Credential Manager refuses a key, the app stores it in a file encrypted for your Windows account
instead. If saving still fails, remove entries you no longer need under Credential Manager → Windows
Credentials → Generic Credentials and try again.

### A number differs from Path of Building

Both apps run the same calculation code, so a difference is a bug. [Open an issue](../../issues) and
include the build's share code along with the stat you checked and the value each app shows.

## Contributing

Issues and pull requests are welcome. You need Rust stable, Bun, and checkouts of
[PathOfBuilding-PoE2](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2) and
[PathOfBuilding](https://github.com/PathOfBuildingCommunity/PathOfBuilding) next to this repo.

```sh
bun install
bun run sync          # copy both games' PoB Lua and data into src-tauri/resources. Run this first.
bun run tauri dev     # run the app with hot reload
```

[CONTRIBUTING.md](CONTRIBUTING.md) has the rest: prerequisites, project layout, updating the bundled
PoB data, the `pobctl` command line and the environment variables. Keep the calculations in PoB's
Lua; do not reimplement them in Rust or TypeScript.

## Credits

Every number PoE Forge shows comes from
[Path of Building Community](https://github.com/PathOfBuildingCommunity). This app runs and bundles
the Lua code and game data of their
[Path of Exile 1](https://github.com/PathOfBuildingCommunity/PathOfBuilding) and
[Path of Exile 2](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2) projects.

Built with [LuaJIT](https://luajit.org) through [mlua](https://github.com/mlua-rs/mlua),
[Tauri](https://tauri.app), [Svelte](https://svelte.dev), [Rust](https://www.rust-lang.org),
[rmcp](https://github.com/modelcontextprotocol/rust-sdk) for the MCP server, and the
[AI SDK](https://ai-sdk.dev) for the assistant.

This product isn't affiliated with or endorsed by Grinding Gear Games in any way.

## Licence

MIT. See [LICENSE](LICENSE). Path of Building Community is MIT as well, and its licence file is
bundled with the app.
