# In-game overlay (preview)

A small panel that sits over Path of Exile while you play. It shows what an item or a map you copy does to the
build open in PoB Redux. Every number comes from Path of Building's own calculation.

## Turn it on

1. Open Settings (Ctrl+,) → **Overlay** and turn on **Turn on the overlay**.
2. Play in **Windowed Fullscreen**. A window cannot sit over the game in exclusive fullscreen.
3. Open your build in PoB Redux. The overlay compares everything against the open build.

| Shortcut | Default | What it does |
|---|---|---|
| Show or hide | `Alt+Shift+O` | Shows or hides the overlay. |
| Move or click | `Alt+Shift+I` | Makes the overlay clickable and movable. Press again, or **Done**, to let clicks through to the game. The position is saved. |

Both shortcuts are global while the overlay is on, so pick combinations the game does not use. The command palette
(Ctrl+K) also has **Show or hide the in-game overlay**.

## Cards

A card opens beside the cursor, on the side of the screen with more room, about half a second after you copy.
It closes as soon as the mouse moves, unless you hold **Ctrl**: with Ctrl held it stays, and a Ctrl+click on
the card keeps it open and makes its buttons work. A kept card closes when you click back in the game or press
**Esc**. While Ctrl is held over a card, the card takes the click instead of the game. Ctrl's state is read with
Windows' `GetAsyncKeyState`, which only reports whether the key is down; nothing is sent to the game.

## Panels

**Item check.** Hover an item in the game and press Ctrl+C, or press **Copy** on a trade site listing. The overlay
shows, for each slot the item fits, the change to DPS, minion DPS, life, energy shield, EHP, resistances and movement
speed, with the best slot first. It also warns when the item needs a higher level or more attributes than the build
has. This is PoB's own "Equipping this item" calculation (`item_evaluate`); the build does not change. Jewels that
rebuild the passive tree (Timeless, Impossible Escape and similar) are not scored; compare them in the Items tab.

**Map check.** Ctrl+C on a map. On Path of Exile 1 the mods PoB knows are set in the Config tab's map mod options on a
temporary copy of the build, so the change to DPS, EHP, maximum hits and maximum resistances is PoB's own. A map tier
of 11+ counts as red, 6–10 as yellow, 1–5 as white, and rolled ranges take the top of the roll. Mods PoB does not
calculate are checked against the build: physical or elemental reflect against the main skill's damage split, no
regeneration, no leech, less recovery. Lines the overlay does not recognise are counted, not guessed. PoE2 waystones
get the text checks only, because PoB for PoE2 has no map mod options. PoB's map mod list is maintained by hand and
can lag behind a league; read the map as well.

**Game log.** The overlay shows no panel for it, but the assistant reads the zone, area level, act and character
level from the game's log (`Client.txt`) through `get_game_context`. The log is found in the standalone install and in Steam libraries. If yours is elsewhere, set **Game log file** in
Settings → Overlay. Only English game clients are understood. A long-played log is hundreds of megabytes; the overlay
reads the last half megabyte on start and then only what the game appends.

**Prices.** Uniques, currency, fragments, scarabs and divination cards get their [poe.ninja](https://poe.ninja)
price on the item card (Path of Exile 1): the cheapest variant, a range when variants differ, and the total for a
stack. The league is the current one unless you set **Price league**. The app downloads poe.ninja's whole price list
for a category and keeps it for half an hour, so poe.ninja never sees what you copied. Rares have no list price; use
the assistant's trade search for those.

**Ask the assistant.** In interact mode every card has an **Ask the assistant** button. It puts the copied text in the
assistant's message box and brings the chat forward; nothing is sent until you press Send. The assistant can also
read the last five things you copied by itself (`get_game_context`), each scored against the open build, so "is the
ring I just copied better?" works without pasting anything.

## What it reads, and what it never does

- While the overlay is showing, it reads the clipboard every 300 ms and acts only on item text: text that starts with
  `Item Class:`, which is what the game writes on Ctrl+C, or a `Rarity:` line followed by the game's `--------`
  section breaks, which is what a trade site Copy can leave. Other clipboard text is ignored and never logged or
  stored. When the overlay is hidden or off, the clipboard is not read.
- The last five items it acted on are kept in memory for the assistant and forgotten when the app closes.
- It reads the game's own log file, which the game writes for players.
- It never reads the game's memory or injects anything into the game, and it never clicks.
- The only keys it sends are **macros** (Settings → Overlay → Macros, F5 → `/hideout` by default): one key press types
  one chat command, as Enter, the command, Enter. Commands must start with `/` and be one line. The macro keys are
  taken only while the game window is in front, so they keep their usual meaning in every other program, and a macro
  waits for Ctrl, Shift and Alt to be released so the typing cannot turn into shortcuts. If the chat box is already
  open with text in it, the first Enter sends that text, as it would in Awakened PoE Trade.

GGG's [third-party policy](https://www.pathofexile.com/developer/docs#policy) remains authoritative. This product
isn't affiliated with or endorsed by Grinding Gear Games in any way.

## Limits

- macOS draws the overlay opaque; transparent windows there need a Tauri feature this build does not enable.
- The overlay does not know whether the game has focus, so it stays up when you alt-tab. Hide it with the shortcut.
- The item and map cards use the build open in PoB Redux, not your live character. Keep it in step.

## Development

| Area | Code |
|---|---|
| Window, shortcuts, clipboard watcher | `src-tauri/src/overlay.rs` |
| Game log reader | `src-tauri/src/gamelog.rs` |
| Settings (`settings.json`, `overlay`) | `OverlaySettings` in `src-tauri/src/game.rs` |
| Prices | `src-tauri/src/prices.rs` (`cargo test -p pob-redux --lib prices -- --ignored` talks to poe.ninja) |
| Assistant | `get_game_context` and `open_trade_search` in `src-tauri/src/tools.rs` |
| Bridge methods | `item_evaluate`, `map_evaluate` in `crates/pob-engine/lua/bridge.lua` |
| Window UI | `src/OverlayWindow.svelte` |

```sh
cargo test -p pob-redux --lib overlay gamelog
POB_ROOT=src-tauri/resources/pob1 cargo test -p pob-engine --test item_evaluate --test map_evaluate
```
