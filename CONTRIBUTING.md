# Contributing

Issues and pull requests are welcome.

Everyone taking part follows the [Code of Conduct](CODE_OF_CONDUCT.md). Report security problems privately, as
[SECURITY.md](SECURITY.md) describes, not in a public issue.

Before a pull request, run `bun run check` and the headless checks below, then open one of your builds
in the app and compare the sidebar against Path of Building itself.

Keep the numbers PoB's own. Do not reimplement calculations in Rust or TypeScript.

Add a code comment only where the code cannot show why, such as a PoB quirk, a workaround or an invariant
that is easy to break. Keep it to one short line.

## Build

You need:

- Rust stable. On Windows, the MSVC toolchain. The first build compiles LuaJIT.
- CMake and libclang. The first build of the app also compiles whisper.cpp for voice input. On
  Windows, the "C++ Clang tools" component of Visual Studio provides libclang.
- Bun 1.4 or newer. This is what CI uses.
- Tauri's platform prerequisites. Windows: WebView2. Linux: `libwebkit2gtk-4.1-dev`,
  `libappindicator3-dev`, `librsvg2-dev`, `patchelf`, `libssl-dev`, `libasound2-dev`,
  `libclang-dev`. macOS: the Command Line Tools, with the licence accepted
  (`sudo xcodebuild -license accept`); the system WKWebView needs nothing installed. See
  [MACOS.md](MACOS.md).
- Checkouts of [PathOfBuilding-PoE2](https://github.com/PathOfBuildingCommunity/PathOfBuilding-PoE2)
  and [PathOfBuilding](https://github.com/PathOfBuildingCommunity/PathOfBuilding) next to this repo.
  To sync one game from somewhere else, run `bun run sync:poe2` or `bun run sync:poe1` with
  `POB_SOURCE` set to that checkout.

```sh
bun install
bun run sync          # copy both games' PoB Lua and data into src-tauri/resources/pob and pob1. Run this first.
bun run sync:fast     # same, but keep the decoded tree art from the last sync
bun run tauri dev     # run the app with hot reload
bun run tauri build   # installers land in target/release/bundle/
bun run check         # type-check the frontend
```

CI type-checks and compiles on every push. Installers are built only when a `v*` tag is pushed; see
`.github/workflows/release.yml`.

If `NoDefaultCurrentDirectoryInExePath` is set in your shell, unset it before the first build. LuaJIT's
`msvcbuild.bat` needs cmd.exe to find `minilua` in the current directory.

### Build-time settings

These are read when the app is compiled. A build without them leaves the feature off rather than
talking to a server it does not own. CI reads them from repository variables of the same name.

| Variable | Effect |
|---|---|
| `POB_FORGE_CDN_URL` | The CDN (`infra/cdn`) for tree art, item art and timeless jewel data. `bun run sync` also needs it when tree art is missing locally |
| `POB_FORGE_GGG_CLIENT_ID` | pathofexile.com OAuth client registered for this app. Without it, account sign-in, character import and trade sign-in stay off |
| `POB_FORGE_CONTACT` | Contact GGG's API rules require in the OAuth user agent. Default: the repository's issues page |

## Releasing

1. Once: run `bun tauri signer generate -w ~/.tauri/pob-forge.key`. Keep the private key and its
   password out of the repository. In the GitHub repository, add the secrets `TAURI_SIGNING_PRIVATE_KEY`
   (the key file's contents) and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, and the variable
   `TAURI_UPDATER_PUBKEY` (the `.pub` file's contents).
2. Once: deploy the CDN and set `POB_FORGE_CDN_URL`; see [infra/cdn/README.md](infra/cdn/README.md).
3. `bun run release:tag patch` (or `minor`, `major`, `x.y.z`) bumps the version, commits, tags and
   pushes. Publish the draft release the workflow creates. Installed copies find it through `update/latest.json` on the CDN, or the
   release's own `latest.json`.

Builds made without the release workflow do not update themselves.

## Layout

```
pob-forge/
  crates/pob-engine/   headless PoB: LuaJIT host shim, JSON bridge, worker engine pool, `pobctl` CLI
  crates/pob-sync/     copies a PathOfBuilding-PoE2 checkout into src-tauri/resources/pob
  src-tauri/           Tauri app: engine thread, commands, `pob://` asset protocol, MCP server
  src/                 Svelte frontend
  pob-sync.toml        upstream checkout path and pinned commit
```

## Translate the interface

The interface is translated with [Paraglide JS](https://paraglidejs.com). English lives in
`messages/en.json`; each other language is a file beside it, named for its locale. A key a language
does not have compiles to the English text, so a partial translation is safe to merge.

```sh
bun run i18n          # compile messages into src/lib/paraglide (bun run check and the dev server do this too)
```

To translate, copy a key and its English text into your language's file and replace the text. Keep
every `{placeholder}` exactly as it appears in English; the compiler rejects a message that invents
one. Leave proper nouns alone: Path of Building, PoB Forge, Discord, Wraeclast and key names such as
`Ctrl+B`. Where a string names something from the game, prefer the wording the game itself uses in
that language.

In the app, `m.some_key()` returns the text for the current language and re-renders when it changes.
Build a table of labels inside `$derived`, never at module scope, or it keeps the language it was
first built with.

To add a language, put its tag in `locales` in `project.inlang/settings.json`, add
`messages/<tag>.json`, and add the language's own name to `LOCALE_LABEL` in
`src/lib/state/locale.svelte.ts`.

Some labels come from Path of Building: the stat labels in the sidebar and the Compare summary, the
item slot labels, and the labels on the Config and Calcs tabs. Their English text is generated into
`messages/pob/en.json`, so never edit that file by hand. To translate them, create
`messages/pob/<tag>.json` and add keys the same way as above. Each key is named after its English
text: `pob_stat_*`, `pob_slot_*`, `pob_config_*` or `pob_calc_*`.

- A numbered slot shares one key, such as `pob_slot_flask_n` for "Flask {n}".
- Some Config labels carry PoB colour codes, such as `^x7070FF` before a word and `^7` after it.
  Keep each code and move it with the word it colours.
- A label that ends with a colon has none in the file. The app adds it back.

After a PoB update, `bun run i18n:labels` regenerates the English file and `bun run check:labels`
fails if it is out of date.

Other game data, such as passive names, item mods and gem names, comes from Path of Building and
stays in English whatever the interface language is.

## Update the bundled PoB data

The installer bundles PoB's Lua and game data. The app never downloads data while it runs, and PoB's
built-in updater is stubbed out. To move to a newer PoB:

1. Pull the PathOfBuilding-PoE2 checkout.
2. Put the new commit hash in `pob-sync.toml`. `bun run sync` refuses to run if the checkout's HEAD
   differs from that pin. Pass `--allow-commit-mismatch` to override.
3. Run `bun run sync`. Use the full sync when the tree changed, because tree art is decoded only with
   `--tree-assets`.
4. Run the checks below, then open one of your builds and compare the sidebar against Path of
   Building. `lua/bridge.lua` drives PoB's internals, so an upstream refactor can break it.
5. Build and release. CI reads the pin from `pob-sync.toml` and checks out that exact commit, so the
   repo alone describes a release.

Users get new data with a new app release. The status bar shows the bundled PoB version and commit.
Do not edit files under `src-tauri/resources/pob/`, because the next sync overwrites them.

## Run checks without the UI

`pobctl` drives the engine from the command line.

```sh
bun run ctl -- --pob-root src-tauri/resources/pob methods            # list bridge methods
bun run ctl -- --pob-root src-tauri/resources/pob call get_build     # call one method
bun run ctl -- --pob-root src-tauri/resources/pob stats build.xml    # load a build, print its sidebar
bun run ctl -- --pob-root src-tauri/resources/pob bench              # time full recalculations

# parallel scoring against the sequential path; mismatches must be 0
bun run ctl -- --pob-root src-tauri/resources/pob power build.xml --stat Life --pool 8
bun run ctl -- --pob-root src-tauri/resources/pob gems build.xml --group 1 --pool 8
```

With the app running and its MCP server on, `bun run mcp` calls the same tools an AI client would, and
`bun run check:tools` guards the assistant's tool lists against the live registry.

## Where builds are stored

PoB Forge reads and writes the same folder as Path of Building:
`Documents/Path of Building (PoE2)/Builds`. It reads PoB's `Settings.xml` to find a custom build path,
and never writes that file.

| Variable | Effect |
|---|---|
| `POB_FORGE_USER_DIR` | Parent folder for `Path of Building (PoE2)/`. Default: Documents |
| `POB_FORGE_POB_ROOT` | Use this PoB data folder instead of the bundled one |
| `POB_FORGE_OPEN` | Build XML to open at launch. The first argument does the same |
| `POB_FORGE_POOL` | Worker engines for node power and gem scoring. They boot on the first scan and are released two minutes after the last. Default: half the cores, at most 8 |
| `POB_FORGE_MCP` | Start the MCP server on this port at launch, whatever the saved setting says |
