// Look-only fixture for the price check card: a fake backend answers with a sample ring, made-up
// listings and a held-back first search. Not trade-site or engine validation.
let searches = 0;
const info = {
  name: "Doom Loop, Two-Stone Ring", unique: false, rarity: "RARE", itemLevel: 84, corrupted: false, category: "Ring", baseName: "Two-Stone Ring",
  realms: ["PC"], listed: ["Instant Buyout", "Instant Buyout & In Person", "In Person (Online)", "Any"], defences: [],
  mods: [
    { lines: ["^x8888FF+14% to Fire and Cold Resistances"], type: "implicit", searchable: true, value: 14, ranged: true, kinds: [] },
    { lines: ["^x8888FF+62 to maximum Life"], type: "explicit", searchable: true, value: 62, ranged: true, kinds: [] },
    { lines: ["^x8888FF+46% to Fire Resistance"], type: "explicit", searchable: true, value: 46, ranged: true, kinds: [] },
    { lines: ["^xB8DAF1+22 to Strength"], type: "explicit", searchable: true, value: 22, ranged: true, kinds: ["crafted"] },
  ],
  quality: null, tradeQuality: null, weapon: null, foulborn: false,
};
const listing = (account: string, amount: number, currency: string) => ({ amount, currency, indexed: new Date(Date.now() - 3600e3).toISOString(), account, online: true, stack: null });
(window as unknown as { __TAURI_INTERNALS__: unknown }).__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "overlay" }, currentWebview: { windowLabel: "overlay", label: "overlay" } },
  transformCallback: () => 0,
  unregisterCallback: () => {},
  invoke: async (cmd: string, args: { method?: string }) => {
    if (cmd === "engine_call") {
      const result =
        args.method === "trade_leagues" ? { leagues: [{ id: "Covens", text: "Covens", realm: null }] }
        : args.method === "buy_similar_info" ? info
        : args.method === "buy_similar_url" ? { url: "https://www.pathofexile.com/trade", query: JSON.stringify({ query: { status: { option: "online" } } }) }
        : null;
      return { result, elapsed_ms: 1 };
    }
    if (cmd === "overlay_settings_get") return { statRange: 10, pricePrediction: true };
    if (cmd === "price_ninja") {
      const base = (args as { category?: string }).category === "BaseType";
      const lines = base
        ? [{ name: "Two-Stone Ring", baseType: null, variant: null, gemLevel: null, gemQuality: null, corrupted: false, links: null, mapTier: null,
            levelRequired: 84, frame: 2, value: 12, listings: 31, change: 18, trend: [0, 4, 3, 9, 12, 15, 18] }]
        : [];
      return { primary: "chaos", rates: { divine: 0.005 }, lines };
    }
    if (cmd === "price_predict") return { min: 18, max: 30, currency: "chaos", confidence: 71 };
    if (cmd === "price_trade") {
      searches++;
      if (searches === 1) throw "rate-limited:4";
      return {
        total: 87, url: "https://www.pathofexile.com/trade",
        listings: [listing("a", 15, "chaos"), listing("a", 15, "chaos"), listing("b", 18, "chaos"), listing("a", 20, "chaos"), listing("c", 0.1, "divine"), listing("d", 24, "chaos")],
      };
    }
    return null;
  },
};

const raw = `Item Class: Rings
Rarity: Rare
Doom Loop
Two-Stone Ring
--------
Item Level: 84
--------
{ Implicit Modifier — Elemental, Fire, Cold, Resistance }
+14(12-16)% to Fire and Cold Resistances (implicit)
--------
{ Prefix Modifier "Virile" (Tier: 4) — Life }
+62(55-69) to maximum Life
{ Suffix Modifier "of the Volcano" (Tier: 1) — Elemental, Fire, Resistance }
+46(46-48)% to Fire Resistance
{ Master Crafted Suffix Modifier "of Craft" (Rank: 2) — Attribute }
+22(19-22) to Strength (crafted)
`;

const { mount } = await import("svelte");
await import("../../src/lib/state/locale.svelte");
await import("@fontsource-variable/inter");
await import("@fontsource-variable/jetbrains-mono");
await import("../../src/app.css");
const { game } = await import("../../src/lib/state/game.svelte");
const { overlay } = await import("../../src/lib/state/overlay.svelte");
const { default: PriceCard } = await import("../../src/lib/overlay/PriceCard.svelte");
game.current = "poe1";
document.documentElement.dataset.theme = "dark";
await overlay.init();
const box = document.getElementById("app")!;
box.style.cssText = "width: 400px; padding: 14px; margin: 16px; background: var(--bg-1); border: 1px solid var(--line-1); border-radius: 10px;";
mount(PriceCard, { target: box, props: { raw, onclose: () => {} } });
(window as unknown as { searches: () => number }).searches = () => searches;
