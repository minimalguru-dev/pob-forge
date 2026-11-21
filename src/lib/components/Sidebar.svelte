<script lang="ts">
  import { tick } from "svelte";
  import { flip } from "svelte/animate";
  import { sortDrag } from "$lib/sortable";
  import { statsOrder } from "$lib/state/saved-order.svelte";
  import CardHead from "./CardHead.svelte";
  import PobText from "./PobText.svelte";
  import BreakdownPanel from "./BreakdownPanel.svelte";
  import Icon from "./Icon.svelte";
  import MoreMenu, { type MenuItem } from "./MoreMenu.svelte";
  import SkillSwitcher from "./SkillSwitcher.svelte";
  import { engine, type BreakdownSection } from "$lib/engine.svelte";
  import { build } from "$lib/state/build.svelte";
  import { groupSidebar, type SidebarSection } from "$lib/sidebar-groups";
  import { statLabelText } from "$lib/pob-labels";
  import { parsePobText, stripPobText } from "$lib/pobtext";
  import { m } from "$lib/paraglide/messages";
  import { ledger, statValue, formatPct, DPS_STATS } from "$lib/state/ledger.svelte";

  const COLLAPSED_KEY = "pob-forge:stats-collapsed";
  let collapsed = $state<Set<string>>(new Set());
  try {
    collapsed = new Set(JSON.parse(localStorage.getItem(COLLAPSED_KEY) ?? "[]"));
  } catch {}
  function toggleGroup(key: string) {
    const next = new Set(collapsed);
    next.has(key) ? next.delete(key) : next.add(key);
    collapsed = next;
    try {
      localStorage.setItem(COLLAPSED_KEY, JSON.stringify([...next]));
    } catch {}
  }

  // What a collapsed group still shows in its header: the first of these stats PoB lists, or all of them when `all`.
  const HEADLINE: Record<string, { stats: string[]; all?: boolean }> = {
    offence: { stats: ["FullDPS", "CombinedDPS", "TotalDPS", "TotalDotDPS", "AverageDamage", "AverageHit"] },
    attributes: { stats: ["Str", "Dex", "Int"], all: true },
    resources: { stats: ["Life", "EnergyShield", "Mana"] },
    mitigation: { stats: ["TotalEHP"] },
    resistances: { stats: ["FireResist", "ColdResist", "LightningResist", "ChaosResist"], all: true },
    fulldps: { stats: ["FullDPS"] },
  };
  function headline(sec: SidebarSection): { text: string; color: string | null }[] {
    const h = HEADLINE[sec.key];
    if (!h) return [];
    const byStat = new Map(sec.items.filter((i) => i.row.stat && i.row.rhs).map((i) => [i.row.stat!, i.row]));
    const found = h.stats.flatMap((k) => (byStat.has(k) ? [byStat.get(k)!] : []));
    if (!h.all) return found.slice(0, 1).map((r) => ({ text: r.rhs!, color: null }));
    // Several values share one line: drop PoB's "(+40%)" overcap and take each label's colour.
    return found.map((r) => ({
      text: stripPobText(r.rhs).replace(/\s*\(.*\)\s*$/, ""),
      color: parsePobText(r.lhs).find((sp) => sp.text.trim())?.color ?? null,
    }));
  }

  // breakdown popup for hovered/pinned stat rows
  let bd = $state<{ sections: BreakdownSection[]; row: number; title: string; y: number; pinned: boolean; ready: boolean } | null>(null);
  let bdPop = $state<HTMLElement>();
  let bdTimer = 0;
  const bdCache = new Map<string, BreakdownSection[]>();

  async function placeBreakdown(row: number, clientY: number) {
    await tick();
    if (!bd || bd.row !== row || !bdPop) return;
    const height = bdPop.getBoundingClientRect().height;
    const y = Math.max(40, Math.min(clientY - 40, window.innerHeight - height - 12));
    bd = { ...bd, y, ready: true };
  }

  function rowBreakdown(clientY: number, rowIndex: number, title: string, pin: boolean) {
    clearTimeout(bdTimer);
    if (pin && bd?.pinned && bd.row === rowIndex) {
      bd = null;
      return;
    }
    const y = Math.max(40, clientY - 40);
    const key = `${rowIndex}:${build.rev}`;
    const apply = (sections: BreakdownSection[]) => {
      bd = { sections, row: rowIndex, title, y, pinned: pin || (bd?.pinned && bd.row === rowIndex) || false, ready: false };
      void placeBreakdown(rowIndex, clientY);
    };
    const cached = bdCache.get(key);
    if (cached) {
      apply(cached);
      return;
    }
    bdTimer = window.setTimeout(async () => {
      try {
        const r = await engine.sidebarBreakdown(rowIndex);
        bdCache.set(key, r.sections);
        apply(r.sections);
      } catch {
        /* row changed under us */
      }
    }, pin ? 0 : 140);
  }

  const rowTitle = (lhs: string | null) => statLabelText(lhs).replace(/:\s*$/, "") || m.sidebar_breakdown();

  function rowLeave() {
    clearTimeout(bdTimer);
    if (bd && !bd.pinned) bd = null;
  }

  $effect(() => {
    build.rev;
    bdCache.clear();
    bd = null;
  });

  // A pinned breakdown takes the mouse (to scroll it) and closes with Escape or a click elsewhere.
  $effect(() => {
    if (!bd?.pinned) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") bd = null;
    };
    const onDown = (e: PointerEvent) => {
      const at = e.target as Element | null;
      if (at && (bdPop?.contains(at) || at.closest(".srow"))) return;
      bd = null;
    };
    window.addEventListener("keydown", onKey);
    window.addEventListener("pointerdown", onDown, true);
    return () => {
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("pointerdown", onDown, true);
    };
  });

  const info = $derived(build.info);
  const side = $derived(build.sidebar);
  /** The stat sheet opens on the build's headline damage, large, with its change since the last save. */
  const headRow = $derived.by(() => {
    const rows = (side?.rows ?? []).filter((r) => r.stat && r.rhs && r.actor !== "minion");
    for (const s of DPS_STATS) {
      const r = rows.find((x) => x.stat === s);
      if (r) return r;
    }
    return null;
  });
  const headDelta = $derived(headRow ? formatPct(ledger.sinceSave(headRow.actor, headRow.stat!, statValue(headRow.rhs))) : "");

  /** A row's change since the last save, coloured by whether it helps the build. */
  function rowDelta(r: { stat: string | null; rhs: string | null; actor: "player" | "minion" | null }): { text: string; tone: string } | null {
    if (!r.stat) return null;
    const pct = ledger.sinceSave(r.actor, r.stat, statValue(r.rhs));
    const text = formatPct(pct);
    if (!text || pct === null) return null;
    const better = pct > 0 === ledger.higherIsBetter(r.stat);
    return { text, tone: better ? "gain" : "loss" };
  }
  const sections = $derived(side ? groupSidebar(side.rows) : []);
  // loadouts: PoB's named tree+items+skills+config combos
  let loadouts = $state<{ loadouts: string[]; active: string | null }>({ loadouts: [], active: null });
  let loEdit = $state<{ mode: "new" | "copy" | "rename"; draft: string } | null>(null);
  $effect(() => {
    build.rev;
    if (build.loaded) engine.getLoadouts().then((r) => (loadouts = r)).catch(() => {});
  });
  function loCommit() {
    const e = loEdit;
    loEdit = null;
    if (!e) return;
    const name = e.draft.trim();
    if (!name) return;
    if (e.mode === "new") build.run(() => engine.newLoadout(name));
    else if (e.mode === "copy" && loadouts.active) build.run(() => engine.copyLoadout(loadouts.active!, name));
    else if (e.mode === "rename" && loadouts.active && name !== loadouts.active) build.run(() => engine.renameLoadout(loadouts.active!, name));
  }

  const buildMenu = $derived<MenuItem[]>([
    ...(loadouts.loadouts.length
      ? [
          { label: m.sidebar_loadout_new(), onclick: () => (loEdit = { mode: "new", draft: "" }) },
          { label: m.sidebar_loadout_copy(), disabled: !loadouts.active, onclick: () => (loEdit = { mode: "copy", draft: m.sidebar_loadout_copy_suffix({ name: loadouts.active ?? "" }) }) },
          { label: m.sidebar_loadout_rename(), disabled: !loadouts.active, onclick: () => (loEdit = { mode: "rename", draft: loadouts.active ?? "" }) },
          {
            label: m.sidebar_loadout_delete(),
            danger: true,
            disabled: loadouts.loadouts.length <= 1 || !loadouts.active,
            onclick: () => loadouts.active && build.run(() => engine.deleteLoadout(loadouts.active!)),
          },
        ]
      : []),
    { label: m.sidebar_rename_build(), separator: loadouts.loadouts.length > 0, onclick: () => (nameEdit = info?.name ?? "") },
  ]);

  // Click the build name to rename it; Enter or blur commits, Escape cancels.
  let nameEdit = $state<string | null>(null);
  function commitName() {
    const name = nameEdit?.trim() ?? "";
    nameEdit = null;
    if (name && name !== info?.name) build.rename(name);
  }

  interface Card {
    key: string;
    label: string;
    kind: "stats" | "warnings" | "notcounted";
    open: boolean;
    toggle: string;
    sec?: SidebarSection;
    title?: string;
  }
  interface CardSum {
    parts: { text: string; color?: string | null }[];
    sep: string;
    num?: boolean;
  }

  const infoSections = $derived(sections.filter((s) => !s.label));
  const cards = $derived.by(() => {
    const list: Card[] = sections
      .filter((s) => s.label)
      .map((s) => ({ key: s.key, label: s.label!, kind: "stats", sec: s, open: !collapsed.has(s.key), toggle: s.key }));
    if (side?.warnings.length) list.push({ key: "warnings", label: m.sidebar_warnings(), kind: "warnings", open: !collapsed.has("warnings"), toggle: "warnings" });
    // Starts collapsed: the stored key marks it opened rather than closed.
    if (side?.notCounted?.count) {
      list.push({ key: "notcounted", label: m.sidebar_not_counted(), kind: "notcounted", open: collapsed.has("notcounted-open"), toggle: "notcounted-open", title: m.sidebar_not_counted_title() });
    }
    return statsOrder.sort(list, (c) => c.key);
  });

  function cardSum(card: Card): CardSum | null {
    if (card.kind === "warnings") return { parts: [{ text: String(side?.warnings.length ?? 0) }], sep: "", num: true };
    if (card.sec && !card.open) return { parts: headline(card.sec), sep: "/", num: true };
    return null;
  }

  let statsEl = $state<HTMLDivElement>();
  let sorting = $state<string | null>(null);

  function startSort(e: PointerEvent, key: string) {
    sortDrag(e, {
      id: key,
      zones: () => [{ column: "stats", el: statsEl }],
      scroller: () => statsEl,
      move: (_column, index) => statsOrder.move(key, cards.map((c) => c.key), index),
      onstart: () => {
        sorting = key;
        rowLeave();
      },
      onend: () => {
        sorting = null;
        statsOrder.save();
      },
    });
  }

  async function nudgeCard(e: KeyboardEvent, key: string) {
    if (!e.altKey || (e.key !== "ArrowUp" && e.key !== "ArrowDown")) return;
    e.preventDefault();
    const shown = cards.map((c) => c.key);
    if (!statsOrder.move(key, shown, shown.indexOf(key) + (e.key === "ArrowUp" ? -1 : 1))) return;
    statsOrder.save();
    await tick();
    statsEl?.querySelector<HTMLElement>(`[data-sort="${key}"] > .cardhead`)?.focus();
  }

  // PoB emits "label:" / "value" pairs plus header rows (only lhs) and spacer
  // rows (no text). Header detection: lhs without a trailing colon.
  function kind(r: { lhs: string | null; rhs: string | null; h: number; align: string | null }) {
    if (!r.lhs && !r.rhs) return "space";
    if (r.align === "CENTER_X") return "center";
    if (r.lhs && !r.rhs) return "head";
    return "row";
  }
</script>

<aside class="sidebar">
  {#if info}
    <section class="headline" aria-live="polite">
      {#if headRow}
        <span class="hlabel">{stripPobText(statLabelText(headRow.lhs)).replace(/:\s*$/, "")}</span>
        <span class="hvalue"><PobText text={headRow.rhs} defaultColor="var(--fg-0)" /></span>
        {#if headDelta}<span class="hdelta" class:gain={headDelta.startsWith("+")} class:loss={headDelta.startsWith("−")}>{m.sheet_since_save({ delta: headDelta })}</span>{/if}
      {/if}
      <!-- The name is in the header; renaming and loadouts stay in this menu. -->
      <span class="hmenu"><MoreMenu label={m.sidebar_build_menu()} align="end" items={buildMenu} /></span>
    </section>
    {#if nameEdit !== null || loadouts.loadouts.length}
    <section class="head">
      {#if nameEdit !== null}
        <!-- svelte-ignore a11y_autofocus -->
        <input
          class="input"
          bind:value={nameEdit}
          autofocus
          onblur={commitName}
          onkeydown={(e) => {
            if (e.key === "Enter") (e.target as HTMLInputElement).blur();
            if (e.key === "Escape") (nameEdit = null);
          }}
        />
      {/if}
      {#if loadouts.loadouts.length}
        {#if loEdit}
          <!-- svelte-ignore a11y_autofocus -->
          <input
            class="input"
            placeholder={loEdit.mode === "new" ? m.sidebar_loadout_new_placeholder() : loEdit.mode === "copy" ? m.sidebar_loadout_copy_placeholder() : m.sidebar_loadout_rename_placeholder()}
            bind:value={loEdit.draft}
            autofocus
            onblur={loCommit}
            onkeydown={(e) => {
              if (e.key === "Enter") (e.target as HTMLInputElement).blur();
              if (e.key === "Escape") (loEdit = null);
            }}
          />
        {:else}
          <select
            class="select loadsel"
            value={loadouts.active ?? ""}
            onchange={(e) => build.run(() => engine.selectLoadout((e.target as HTMLSelectElement).value))}
            disabled={build.busy > 0}
            aria-label={m.sidebar_loadout()}
            title={m.sidebar_loadout_title()}
          >
            {#if !loadouts.active}<option value="">—</option>{/if}
            {#each loadouts.loadouts as l}
              <option value={l}>{stripPobText(l)}</option>
            {/each}
          </select>
        {/if}
      {/if}
    </section>
    {/if}
    <div class="charwrap">
      <SkillSwitcher />
    </div>

    <div class="stats" class:busy={build.busy > 0} class:sorting={sorting !== null} bind:this={statsEl}>
      {#if side}
        {#each infoSections as sec (sec.key)}
          <section class="scard"><div class="cardbody">{@render statRows(sec)}</div></section>
        {/each}
        {#each cards as card (card.key)}
          <section class="scard" class:warncard={card.kind === "warnings"} class:dragging={sorting === card.key} data-sort={card.key} animate:flip={{ duration: sorting === card.key ? 0 : 200 }}>
            {@render cardHead(card.label, card.open, card.toggle, cardSum(card), card.key, card.title)}
            {#if card.open}
              {#if card.sec}
                <div class="cardbody">{@render statRows(card.sec)}</div>
              {:else if card.kind === "warnings"}
                <div class="cardbody warnings">
                  {#each side.warnings as w}
                    <div class="warn">{w}</div>
                  {/each}
                </div>
              {:else if side.notCounted}
                {@const nc = side.notCounted}
                <div class="cardbody notcounted">
                  {#each [...nc.items.map((x) => ({ key: x.slot, head: x.slot, sub: stripPobText(x.name), lines: x.lines })), ...nc.nodes.map((x) => ({ key: String(x.id), head: x.name ?? "", sub: "", lines: x.lines }))] as src (src.key)}
                    <div class="ncsrc">
                      <div class="nchead">{src.head}{#if src.sub}<span class="dim">{` · ${src.sub}`}</span>{/if}</div>
                      {#each src.lines as l}<div class="ncline">{l}</div>{/each}
                    </div>
                  {/each}
                </div>
              {/if}
            {/if}
          </section>
        {/each}
        {#if statsOrder.changed}<button class="reset" onclick={() => statsOrder.reset()}>{m.common_reset_order()}</button>{/if}
      {/if}
    </div>
  {:else}
    <div class="empty">
      <span class="label">{m.sidebar_no_build()}</span>
    </div>
  {/if}

  {#snippet cardHead(label: string, open: boolean, toggle: string, sum: CardSum | null, sortKey: string | null, title: string | undefined)}
    {#snippet summary()}
      {#if sum}<span class:num={sum.num}>{#each sum.parts as p, i}{#if i}<span class="sep">{sum.sep}</span>{/if}<PobText text={p.text} defaultColor={p.color ?? null} />{/each}</span>{/if}
    {/snippet}
    <CardHead
      {open}
      {title}
      sortable={sortKey !== null}
      onclick={() => toggleGroup(toggle)}
      onpointerdown={sortKey ? (e) => startSort(e, sortKey) : undefined}
      onkeydown={sortKey ? (e) => nudgeCard(e, sortKey) : undefined}
      extra={sum ? summary : undefined}
    >
      {#snippet name()}{label}{/snippet}
    </CardHead>
  {/snippet}

  {#snippet statRows(sec: SidebarSection)}
    {#each sec.items as { row: r, index: rowIndex } (rowIndex)}
      {@const k = kind(r)}
      {#if k === "space"}
        <div class="space"></div>
      {:else if k === "head"}
        <div class="shead"><PobText text={r.lhs} /></div>
      {:else if k === "center"}
        <div class="scenter"><PobText text={r.lhs} defaultColor="var(--fg-2)" /></div>
      {:else}
        {@const pinned = bd?.pinned && bd.row === rowIndex + 1}
        <div
          class="srow"
          class:hasbd={r.hasBreakdown}
          aria-pressed={pinned}
          role="button"
          tabindex={r.hasBreakdown ? 0 : -1}
          onmouseenter={(e) => r.hasBreakdown && rowBreakdown(e.clientY, rowIndex + 1, rowTitle(r.lhs), false)}
          onmouseleave={rowLeave}
          onclick={(e) => r.hasBreakdown && rowBreakdown(e.clientY, rowIndex + 1, rowTitle(r.lhs), true)}
          onkeydown={(e) => e.key === "Enter" && r.hasBreakdown && rowBreakdown(200, rowIndex + 1, rowTitle(r.lhs), true)}
        >
          <span class="k"><PobText text={statLabelText(r.lhs).replace(/:\s*$/, "")} defaultColor="var(--fg-1)" /></span>
          <span class="svalue">
            {#if rowDelta(r)}{@const d = rowDelta(r)!}<span class="delta {d.tone}">{d.text}</span>{/if}
            <span class="v num"><PobText text={r.rhs} /></span>
            {#if pinned}<span class="pinmark" title={m.sidebar_breakdown_pinned()}><Icon name="push-pin" size={11} /></span>{/if}
          </span>
        </div>
      {/if}
    {/each}
  {/snippet}

  {#if bd}
    <div bind:this={bdPop} class="bdpop" class:ready={bd.ready} class:pinned={bd.pinned} style:top={`${bd.y}px`}>
      <div class="bdhead">
        <span class="label"><PobText text={bd.title} /></span>
        {#if bd.pinned}<span class="bdpin" title={m.sidebar_breakdown_pinned()}><Icon name="push-pin" size={12} /></span>{/if}
      </div>
      <div class="bdscroll">
        <BreakdownPanel sections={bd.sections} />
      </div>
    </div>
  {/if}
</aside>

<style>
  .sidebar {
    width: var(--sidebar-w);
    display: flex;
    flex-direction: column;
    background: var(--bg-1);
    font-variant-numeric: tabular-nums;
    border-right: 1px solid var(--line-0);
    min-height: 0;
  }
  .headline {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-height: 44px;
    padding: 16px 16px 12px;
    border-bottom: 1px solid var(--line-0);
    font-variant-numeric: tabular-nums;
  }
  .hmenu {
    position: absolute;
    top: 10px;
    right: 8px;
  }
  .hlabel {
    font-size: var(--fs-2xs);
    font-weight: 500;
    letter-spacing: var(--label-track);
    text-transform: uppercase;
    color: var(--fg-3);
  }
  .hvalue {
    font-family: var(--font-mono);
    font-size: 26px;
    font-weight: 500;
    line-height: 1.2;
    color: var(--fg-0);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .hdelta {
    font-family: var(--font-mono);
    font-size: var(--fs-sm);
    color: var(--fg-3);
  }
  .gain {
    color: var(--gain);
  }
  .loss {
    color: var(--loss);
  }
  .delta {
    font-family: var(--font-mono);
    font-size: var(--fs-xs);
  }
  /* Group heads read as labels on a sheet, not as cards. */
  .stats :global(.cardhead) {
    min-height: 28px;
    background: transparent;
    border-bottom-color: transparent;
    color: var(--fg-3);
  }
  .stats :global(.cardhead:hover) {
    background: transparent;
    color: var(--fg-0);
  }
  .stats :global(.cardname) {
    font-size: var(--fs-2xs);
    font-weight: 500;
    letter-spacing: var(--label-track);
  }
  .stats :global(.cardsum) {
    font-family: var(--font-mono);
  }
  .head {
    padding: 8px 12px 8px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .loadsel {
    align-self: flex-start;
    max-width: 100%;
    height: 22px;
    padding-left: 0;
    border-color: transparent;
    background-color: transparent;
    color: var(--fg-2);
    font-size: var(--fs-xs);
  }
  .loadsel:hover {
    color: var(--fg-0);
  }
  .charwrap {
    padding: 8px 6px;
    border-bottom: 1px solid var(--line-0);
  }
  .stats {
    position: relative;
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 0;
    padding: 2px 4px 16px;
    background: var(--bg-1);
    scrollbar-width: none;
    transition: opacity 120ms;
  }
  .stats::-webkit-scrollbar {
    display: none;
  }
  .scard {
    flex: none;
    background: transparent;
    border: 0;
    border-top: 1px solid var(--line-0);
    border-radius: 0;
    overflow: clip;
  }
  .scard:first-child {
    border-top: 0;
  }
  .scard.dragging {
    position: relative;
    z-index: 20;
    opacity: 0.8;
    box-shadow: var(--shadow-pop);
    pointer-events: none;
    will-change: transform;
  }
  .sorting {
    cursor: grabbing;
  }
  .sorting .cardbody {
    pointer-events: none;
  }
  .reset {
    align-self: flex-end;
    appearance: none;
    margin-top: 2px;
    padding: 0 4px;
    border: 0;
    background: none;
    color: var(--fg-3);
    font: inherit;
    font-size: var(--fs-xs);
    cursor: var(--cursor-pointer, pointer);
  }
  .reset:hover {
    color: var(--fg-1);
    text-decoration: underline;
  }
  .sep {
    margin: 0 4px;
    color: var(--fg-3);
  }
  .warncard :global(.cardname) {
    color: var(--warn);
  }
  .cardbody {
    padding: 4px 8px 10px;
  }
  .stats.busy {
    opacity: 0.6;
  }
  .space {
    height: 7px;
  }
  .shead {
    padding: 8px 0 3px;
    font-size: var(--fs-xs);
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--fg-2);
  }
  .scenter {
    text-align: center;
    font-size: var(--fs-xs);
    padding: 1px 0;
  }
  .srow {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 10px;
    padding: 1px 0;
    font-size: var(--fs-sm);
    line-height: 18px;
    font-variant-numeric: tabular-nums;
    border-radius: 2px;
  }
  .srow .k {
    color: var(--fg-2);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .srow .v {
    white-space: nowrap;
    color: var(--fg-0);
    font-family: var(--font-mono);
    font-size: var(--fs-sm);
  }
  .svalue {
    flex: none;
    display: flex;
    align-items: center;
    gap: 5px;
  }
  .pinmark,
  .bdpin {
    display: grid;
    place-items: center;
    color: var(--focus);
  }
  .srow.hasbd {
    cursor: var(--cursor-arrow, default);
  }
  .srow.hasbd:hover {
    background: var(--bg-2);
    margin: 0 -6px;
    padding: 1px 6px;
  }
  .bdpop {
    position: fixed;
    left: calc(var(--sidebar-w) + 8px);
    width: min(720px, calc(100vw - var(--sidebar-w) - 24px));
    max-height: 60vh;
    display: flex;
    flex-direction: column;
    background: color-mix(in srgb, var(--bg-1) 96%, transparent);
    border: 1px solid var(--line-1);
    border-radius: var(--r-2);
    box-shadow: var(--shadow-tooltip);
    backdrop-filter: blur(8px);
    z-index: 20;
    pointer-events: none;
    visibility: hidden;
  }
  .bdpop.ready {
    visibility: visible;
  }
  /* Hovered it lets the mouse through; pinned it takes the wheel so its list scrolls. */
  .bdpop.pinned {
    pointer-events: auto;
  }
  .bdhead {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    padding: 8px 12px;
    border-bottom: 1px solid var(--line-0);
    border-radius: var(--r-2) var(--r-2) 0 0;
    background: var(--bg-3);
    color: var(--fg-0);
    font-weight: 700;
  }
  .bdscroll {
    min-height: 0;
    padding: 10px 12px;
    overflow-y: auto;
    overscroll-behavior: contain;
  }
  .warnings {
    display: flex;
    flex-direction: column;
    padding-top: 0;
    padding-bottom: 0;
  }
  .warn {
    padding: 6px 0;
    font-size: var(--fs-xs);
    color: var(--warn);
    line-height: 1.35;
  }
  .warn + .warn {
    border-top: 1px solid var(--line-1);
  }
  .notcounted {
    display: flex;
    flex-direction: column;
    padding-top: 0;
    padding-bottom: 0;
  }
  .ncsrc {
    padding: 6px 0;
    font-size: var(--fs-xs);
    line-height: 1.35;
  }
  .ncsrc + .ncsrc {
    border-top: 1px solid var(--line-1);
  }
  .nchead {
    color: var(--fg-1);
  }
  .ncline {
    color: var(--bad);
  }
  .empty {
    flex: 1;
    display: grid;
    place-items: center;
  }
</style>
