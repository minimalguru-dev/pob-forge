# In-game overlay (preview)

A small panel that sits over Path of Exile while you play. It shows what an item or a map you copy does to the
build open in PoB Forge. Every number comes from Path of Building's own calculation.

## Turn it on

1. Open Settings (Ctrl+,) → **Overlay** and turn on **Turn on the overlay**.
2. Play in **Windowed Fullscreen**. A window cannot sit over the game in exclusive fullscreen.
3. Open your build in PoB Forge. The overlay compares everything against the open build.

| Shortcut | Default | What it does |
|---|---|---|
| Show or hide | `Alt+Shift+O` | Shows or hides the overlay. |
| Move or click | `Alt+Shift+I` | Makes the overlay clickable and movable. Press again, or **Done**, to let clicks through to the game. The position is saved. |

Both shortcuts are global while the overlay is on, so pick combinations the game does not use. The command palette
(Ctrl+K) also has **Show or hide the in-game overlay**.

## Cards

A card copied in the game always opens in the same place: just left of the inventory panel, centred on the screen's
height, so it covers neither the stash nor the inventory. One copied elsewhere (the trade site) opens beside the
cursor. The card is as tall as its content. It follows the Awakened PoE Trade rules:

- **Hold Ctrl to look.** Keep Ctrl down after Ctrl+C or Ctrl+D and the card stays; let go and it closes, unless the
  cursor is on the card.
- **A quick Ctrl+C** (Ctrl already let go) keeps the card until the mouse moves away.
- **Let go of Ctrl with the cursor on the card** and it stays, as if Ctrl+clicked.
- **Ctrl+click** the card to keep it open and make its buttons and fields work. A kept card closes when you click back
  in the game or press **Esc**. While Ctrl is held over a card, the card takes the click instead of the game.

Ctrl's state is read with Windows' `GetAsyncKeyState`, which only reports whether the key is down.

## Panels

**Item check.** Hover an item in the game and press Ctrl+C, or press **Copy** on a trade site listing. The overlay
shows, for each slot the item fits, the change to DPS, minion DPS, life, energy shield, EHP, resistances and movement
speed, with the best slot first. It also warns when the item needs a higher level or more attributes than the build
has. This is PoB's own "Equipping this item" calculation (`item_evaluate`); the build does not change. Jewels that
rebuild the passive tree (Timeless, Impossible Escape and similar) are not scored; compare them in the Items tab.

**Price check (Windows).** Hover an item in the game and press **Ctrl+D**. The app presses Ctrl+C once so the
game copies the item, then opens a price card just left of the inventory panel. Click the card to change its filters
(no Ctrl needed); Enter searches again, Esc or ✕ closes it and gives focus back to the game.

- **poe.ninja** prices currency, fragments, scarabs, essences, divination cards, gems (by level, quality and
  corruption) and uniques (six-links separately on PoE1), for the league you pick, with its seven-day trend. A stack
  shows its total.
- **Currency exchange.** Currency and divination cards also show the cheapest online offers on the trade site's bulk
  exchange, per unit, in chaos (PoE1) or exalted orbs (PoE2).
- **Trade site.** For equipment, PoB's own Buy Similar search (the same one as the Items tab) is built from the item:
  a unique by name, a rare by its item class, its resistances and life as the trade site's pseudo totals ("+#% total
  Elemental Resistance", "+# total maximum Life") and its other searchable mods, all at 90% of the item's values.
  Mods counted in a ticked total are not searched on their own. Item level, quality, links (PoE1), corruption, the
  listing kind, the currency the price is in, and a minimum of empty prefixes or suffixes are filters too. Same base
  type and item level are toggles, both off at first; item level starts at the copied item's own.
  **Quality** works as on the trade site, which counts armour, evasion, energy shield and weapon DPS as if the item
  had 20% quality: an item below 20% is priced at 20% (rows marked **Q20**), and weapons get physical and elemental
  DPS rows, the larger one ticked on a rare. The quality filter starts at the item's own quality on flasks,
  tinctures and charms, and on anything past 20%; gems search their own quality.
  **Foulborn** uniques are priced by their mutated mods: PoB knows which they are (the game's copy does not mark
  them), ticks them, and poe.ninja's line for that set of mutated mods is shown. Mods are tagged by what PoB says they
  are (mutated, crafted, fractured…); crafted mods start unticked. Influences, Searing Exarch and Eater of Worlds
  items, Fractured, Synthesised, Mirrored and Split (PoE2: Fractured, Mirrored, Sanctified) are ticked filters.
  Maps and waystones also offer their quantity, rarity and pack figures (unticked), and blighted or blight-ravaged maps
  search their own market.
  **Base only** turns every mod filter off and searches the base type alone. Untick mods or change the minimums and search again. The ten cheapest listings show with their age; **Open on the trade site** opens
  the full search on pathofexile.com in your browser. Under each mod are its tags, as in Awakened PoE Trade: pseudo,
  implicit, enchant, crafted, fractured, eldritch (Searing Exarch and Eater of Worlds) and what else the game marks.
- **Temples, gems, maps and the rest.** A Chronicle of Atzoatl is searched by its open rooms, with Locus of
  Corruption, Doryani's Institute and the Apex ticked; gems by level, quality and corruption; maps and waystones by
  tier. Anything else Buy Similar cannot read is searched by its name and base.
- Searches are anonymous: no sign-in and no cookies. pathofexile.com allows about five searches every ten seconds;
  when a limit is reached the card says how long to wait instead of sending more.
- **Wiki** and **poedb** open the item's pages in your browser.
- Ctrl+D is taken only while the game window has focus, so it keeps working in other programs. Change it or turn the
  price check off in Settings → Overlay.

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

- It reads the clipboard once per key press, as GGG's overlay rules ask. While the overlay shows, it watches only
  whether Ctrl+C is pressed (Windows' `GetAsyncKeyState`) and Windows' clipboard change counter, which says that
  something was copied without reading it. After your Ctrl+C, the clipboard is read once, as soon as the copy has
  landed (usually a few tens of milliseconds). Between presses it is not read at all, so nothing else you copy is
  seen. It acts only on item text: text that starts with `Item Class:`, or a `Rarity:` line followed by the game's
  `--------` section breaks. Other text is ignored and never logged or stored. A trade site's Copy button is a click,
  not a key press, so use Ctrl+C there instead. Ctrl+D reads the clipboard once, after its own Ctrl+C.
- The last five items it acted on are kept in memory for the assistant and forgotten when the app closes.
- It reads the game's own log file, which the game writes for players.
- It never reads the game's memory or injects anything into the game, and it never clicks.
- It sends keys in two cases only, each on one key press of yours while the game window is in front:
  - **Macros** (Settings → Overlay → Macros, F5 → `/hideout` by default): one key press types one chat command, as
    Enter, the command, Enter. Commands must start with `/` and be one line. A macro waits for Ctrl, Shift and Alt to
    be released so the typing cannot turn into shortcuts. If the chat box is already open with text in it, the first
    Enter sends that text, as it would in Awakened PoE Trade.
  - **Price check** (Ctrl+D): one Ctrl+C, so the game copies the item under the cursor. If you are still holding
    Ctrl, only C is sent.
  These keys are taken only while the game is in front, so they keep their usual meaning in every other program.
- A price check sends the item's search to pathofexile.com's trade site and asks poe.ninja for its price tables.
  Nothing else leaves your computer.

GGG's [third-party policy](https://www.pathofexile.com/developer/docs#policy) remains authoritative. This product
isn't affiliated with or endorsed by Grinding Gear Games in any way.

## Limits

- macOS draws the overlay opaque; transparent windows there need a Tauri feature this build does not enable.
- The overlay does not know whether the game has focus, so it stays up when you alt-tab. Hide it with the shortcut.
- The price check needs Windows: other systems have no way here to have the game copy an item.
- The trade search reads equipment, jewels and flasks through PoB's item parser. Currency, gems and cards are priced
  by poe.ninja only.
- The item and map cards use the build open in PoB Forge, not your live character. Keep it in step.

## Development

| Area | Code |
|---|---|
| Window, shortcuts, clipboard watcher | `src-tauri/src/overlay.rs` |
| Game log reader | `src-tauri/src/gamelog.rs` |
| Settings (`settings.json`, `overlay`) | `OverlaySettings` in `src-tauri/src/game.rs` |
| Prices | `src-tauri/src/prices.rs` (`cargo test -p pob-forge --lib prices -- --ignored` talks to poe.ninja) |
| Assistant | `get_game_context` and `open_trade_search` in `src-tauri/src/tools.rs` |
| Bridge methods | `item_evaluate`, `map_evaluate` in `crates/pob-engine/lua/bridge.lua` |
| Window UI | `src/OverlayWindow.svelte` |

```sh
cargo test -p pob-forge --lib overlay gamelog
POB_ROOT=src-tauri/resources/pob1 cargo test -p pob-engine --test item_evaluate --test map_evaluate
```
