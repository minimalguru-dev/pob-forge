<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { onMount } from "svelte";
  import { build, type ViewId } from "$lib/state/build.svelte";
  import { ui } from "$lib/state/ui.svelte";
  import { appOptions } from "$lib/state/options.svelte";
  import { game, GAMES, GAME_SHORT, GAME_LABEL, type Game } from "$lib/state/game.svelte";
  import Icon, { type IconName } from "$lib/components/Icon.svelte";
  import Kbd from "$lib/components/Kbd.svelte";
  import { isTextField } from "$lib/keys";
  import { m } from "$lib/paraglide/messages";


  const win = getCurrentWindow();
  let maximized = $state(false);
  let tabsEl = $state<HTMLDivElement | null>(null);
  let spacerEl = $state<HTMLDivElement | null>(null);
  let measureEl = $state<HTMLDivElement | null>(null);
  let labelsWidth = $state(0);
  let room = $state(0);
  const compact = $derived(ui.tabIcons || labelsWidth > room);

  function openView(id: ViewId) {
    appOptions.open = false;
    build.view = id;
  }

  // Declining the unsaved-changes prompt leaves the game as it was, which the
  // select cannot know, so put it back.
  async function onGame(e: Event) {
    const el = e.currentTarget as HTMLSelectElement;
    if (!(await game.choose(el.value as Game))) el.value = game.current;
  }

  $effect(() => {
    void build.view;
    void appOptions.open;
    tabsEl?.querySelector(".tab.active")?.scrollIntoView({ block: "nearest", inline: "nearest" });
  });

  const tabs = $derived<{ id: ViewId; label: string; key: string; icon: IconName }[]>([
    { id: "import", label: m.view_builds(), key: "1", icon: "folder" },
    { id: "overview", label: m.view_overview(), key: "", icon: "squares-four" },
    { id: "tree", label: m.view_tree(), key: "2", icon: "graph" },
    { id: "skills", label: m.view_skills(), key: "3", icon: "diamonds-four" },
    { id: "items", label: m.view_items(), key: "4", icon: "sword" },
    { id: "calcs", label: m.view_calcs(), key: "5", icon: "calculator" },
    { id: "config", label: m.view_config(), key: "6", icon: "sliders-horizontal" },
    { id: "party", label: m.view_party(), key: "7", icon: "users-three" },
    { id: "optimise", label: m.view_optimise(), key: "8", icon: "chart-line-up" },
    { id: "compare", label: m.view_compare(), key: "", icon: "scales" },
    { id: "regex", label: m.view_regex(), key: "", icon: "magnifying-glass" },
  ]);

  const levelLine = $derived(
    build.info ? [build.info.ascendClassName ?? build.info.className, build.info.level].filter(Boolean).join(" ") : "",
  );

  function tabTitle(t: { label: string; key: string }) {
    const shortcut = t.key ? `Ctrl+${t.key}` : "";
    if (!compact) return shortcut || undefined;
    return shortcut ? `${t.label} (${shortcut})` : t.label;
  }

  // the build name lives in the sidebar; the OS title carries it for the taskbar
  $effect(() => {
    const name = build.info?.name;
    win.setTitle(name ? `${name}${build.info?.unsaved ? " •" : ""} — PoE Forge` : "PoE Forge").catch(() => {});
  });

  onMount(() => {
    win.isMaximized().then((v) => (maximized = v));
    const un = win.onResized(async () => (maximized = await win.isMaximized()));
    const onKey = (e: KeyboardEvent) => {
      if (e.ctrlKey && !e.altKey && e.key === "Tab") {
        const open = tabs.filter((t) => build.loaded || t.id === "import" || t.id === "regex");
        const at = Math.max(0, open.findIndex((t) => t.id === build.view));
        openView(open[(at + (e.shiftKey ? -1 : 1) + open.length) % open.length].id);
        e.preventDefault();
        return;
      }
      if (e.ctrlKey && e.shiftKey && !e.altKey && e.key.toLowerCase() === "s") {
        void build.saveAs();
        e.preventDefault();
        return;
      }
      if (e.ctrlKey && !e.shiftKey && !e.altKey) {
        if (e.key === "s") {
          void build.save();
          e.preventDefault();
        }
        const t = tabs.find((t) => t.key === e.key);
        if (t && build.loaded) {
          openView(t.id);
          e.preventDefault();
        }
        const treeUndo = build.view === "tree" && !appOptions.open && !isTextField(e.target);
        if (e.key === "z" && treeUndo) {
          build.undo();
          e.preventDefault();
        }
        if (e.key === "y" && treeUndo) {
          build.redo();
          e.preventDefault();
        }
        if (e.key === "b") {
          ui.toggleSidebar();
          e.preventDefault();
        }
        if (e.key === ",") {
          appOptions.open = !appOptions.open;
          e.preventDefault();
        }
        if (e.key === "=" || e.key === "+") {
          ui.stepScale(1);
          e.preventDefault();
        }
        if (e.key === "-") {
          ui.stepScale(-1);
          e.preventDefault();
        }
        if (e.key === "0") {
          ui.setScale(1);
          e.preventDefault();
        }
      }
    };
    window.addEventListener("keydown", onKey);
    const onWheel = (e: WheelEvent) => {
      if (!tabsEl || tabsEl.scrollWidth <= tabsEl.clientWidth || !e.deltaY) return;
      tabsEl.scrollLeft += e.deltaY;
      e.preventDefault();
    };
    tabsEl?.addEventListener("wheel", onWheel, { passive: false });
    const fit = new ResizeObserver(() => {
      labelsWidth = measureEl?.offsetWidth ?? 0;
      room = (tabsEl?.clientWidth ?? 0) + (spacerEl?.clientWidth ?? 0);
    });
    for (const el of [measureEl, tabsEl, spacerEl]) if (el) fit.observe(el);
    return () => {
      fit.disconnect();
      tabsEl?.removeEventListener("wheel", onWheel);
      un.then((f) => f());
      window.removeEventListener("keydown", onKey);
    };
  });
</script>

<header class="titlebar" data-tauri-drag-region>
  <div class="brand" data-tauri-drag-region>
    <svg class="hammer" width="20" height="17" viewBox="0 0 26 22" aria-hidden="true"
      ><path d="M2 6h15l5-3h2v6h-2l-4 2v2h-4v5h5v3H6v-3h5v-5H8C5 13 2 10 2 6z" fill="currentColor" /></svg
    >
    <span class="wordmark" data-tauri-drag-region>PoE Forge</span>
    <button
      class="sb"
      class:on={!ui.sidebarCollapsed}
      aria-label={ui.sidebarCollapsed ? m.titlebar_sidebar_show() : m.titlebar_sidebar_hide()}
      title={ui.sidebarCollapsed ? m.titlebar_sidebar_show_title() : m.titlebar_sidebar_hide_title()}
      onclick={() => ui.toggleSidebar()}
    >
      <svg width="14" height="12" viewBox="0 0 14 12" fill="none" stroke="currentColor" stroke-width="1">
        <rect x="0.5" y="0.5" width="13" height="11" rx="1.5" />
        <path d="M5 0.5v11" />
      </svg>
    </button>
    <div class="game">
      <select
        class="select gsel"
        aria-label={m.titlebar_game()}
        value={game.current}
        disabled={game.switching || build.busy > 0}
        title={GAME_LABEL[game.current]}
        onchange={onGame}
      >
        {#each GAMES as g (g)}
          <option value={g}>{GAME_SHORT[g]}</option>
        {/each}
      </select>
    </div>
  </div>


  <div class="lead" data-tauri-drag-region>
    {#if build.info}
      <span class="buildplate fg-well" title={build.info.file ?? m.sidebar_not_saved()}>
        <span class="bname">{build.info.name}</span>
        <span class="blevel">{levelLine}</span>
      </span>
    {/if}
  </div>

  <div class="tabs" class:compact role="tablist" aria-label={m.titlebar_views()} bind:this={tabsEl}>
    {#each tabs as t (t.id)}
      <button
        role="tab"
        class="tab"
        class:active={build.view === t.id && !appOptions.open}
        aria-selected={build.view === t.id && !appOptions.open}
        disabled={!build.loaded && t.id !== "import" && t.id !== "regex"}
        onclick={() => openView(t.id)}
        title={tabTitle(t)}
        aria-label={t.label}
      >
        {#if compact}<Icon name={t.icon} size={15} />{:else}{t.label}{/if}
      </button>
    {/each}
  </div>

  <div class="spacer" data-tauri-drag-region bind:this={spacerEl}></div>
  <div class="measure" aria-hidden="true">
    <div class="tabs" bind:this={measureEl}>
      {#each tabs as t (t.id)}<span class="tab">{t.label}</span>{/each}
    </div>
  </div>

  <div class="findcell">
    {#if build.loaded}
      <button class="find fg-well" title={m.palette_title()} onclick={() => (ui.paletteOpen = true)}>
        <Icon name="magnifying-glass" size={13} />
        <span class="findlabel">{m.palette_button()}</span>
        <Kbd keys="Mod+K" />
      </button>
      <button class="save" class:dirty={build.info?.unsaved} title={m.palette_save()} onclick={() => void build.save()}>
        {m.import_save()}{#if build.info?.unsaved}<span class="dot" aria-label={m.sidebar_unsaved()}></span>{/if}
      </button>
    {/if}
  </div>

  <div class="controls">
    <button
      class="wc opts"
      class:on={appOptions.open}
      aria-label={m.titlebar_settings()}
      aria-pressed={appOptions.open}
      title={m.titlebar_settings_title()}
      onclick={() => (appOptions.open = !appOptions.open)}
    >
      <Icon name="gear" size={15} />
    </button>
    <button class="wc" aria-label={m.titlebar_minimize()} onclick={() => win.minimize()}>
      <svg width="10" height="10" viewBox="0 0 10 10"><path d="M0 5.5h10" stroke="currentColor" stroke-width="1" /></svg>
    </button>
    <button class="wc" aria-label={m.titlebar_maximize()} onclick={() => win.toggleMaximize()}>
      {#if maximized}
        <svg width="10" height="10" viewBox="0 0 10 10"
          ><path d="M2.5 0.5h7v7M0.5 2.5h7v7h-7z" fill="none" stroke="currentColor" stroke-width="1" /></svg
        >
      {:else}
        <svg width="10" height="10" viewBox="0 0 10 10"><rect x="0.5" y="0.5" width="9" height="9" fill="none" stroke="currentColor" stroke-width="1" /></svg>
      {/if}
    </button>
    <button class="wc close" aria-label={m.titlebar_close()} onclick={() => win.close()}>
      <svg width="10" height="10" viewBox="0 0 10 10"><path d="M0.5 0.5l9 9M9.5 0.5l-9 9" stroke="currentColor" stroke-width="1.1" /></svg>
    </button>
  </div>
</header>

<style>
  /* A forged bar: plate iron with a bevel, a gold name plate, tabs pressed into it. */
  .titlebar {
    height: var(--titlebar-h);
    display: grid;
    grid-template-columns: auto auto minmax(0, auto) minmax(0, 1fr) auto auto;
    align-items: stretch;
    background: var(--bar-bg);
    box-shadow: var(--bar-shadow);
    border-bottom: 1px solid var(--plate-edge);
    -webkit-app-region: drag;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 0 10px 0 14px;
  }
  .hammer {
    flex: none;
    color: var(--brand);
    pointer-events: none;
  }
  .wordmark {
    font-family: var(--font-head);
    font-size: 16px;
    font-weight: 700;
    letter-spacing: 0.1em;
    color: var(--brand);
    white-space: nowrap;
    user-select: none;
  }
  .sb {
    appearance: none;
    border: 0;
    background: transparent;
    color: var(--fg-3);
    padding: 4px;
    border-radius: var(--r-1);
    display: grid;
    place-items: center;
    cursor: var(--cursor-pointer, pointer);
    -webkit-app-region: no-drag;
  }
  .sb:hover {
    color: var(--fg-0);
    background: var(--bg-hover);
  }
  .sb.on {
    color: var(--fg-2);
  }
  .game {
    display: flex;
    align-items: center;
    margin-left: -6px;
    -webkit-app-region: no-drag;
  }
  .gsel {
    height: 22px;
    /* Fixed, so a longer option cannot widen the closed control. */
    width: 68px;
    padding: 0 20px 0 7px;
    background-color: transparent;
    background-position: right 6px center;
    border-color: transparent;
    box-shadow: none;
    color: var(--fg-0);
    font-family: var(--font-mono);
    font-size: var(--fs-xs);
    letter-spacing: 0.06em;
  }
  .gsel:hover:not(:disabled) {
    background-color: var(--bg-hover);
  }
  .gsel:disabled {
    cursor: var(--cursor-arrow, default);
    opacity: 0.6;
  }
  .gsel option {
    background: var(--bg-1);
    color: var(--fg-0);
  }
  .lead {
    display: flex;
    align-items: center;
    min-width: 0;
    padding: 0 10px 0 4px;
  }
  .buildplate {
    display: flex;
    align-items: baseline;
    gap: 10px;
    min-width: 0;
    max-width: 300px;
    padding: 2px 9px;
  }
  .bname {
    min-width: 0;
    font-weight: 600;
    color: var(--fg-0);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .blevel {
    flex: none;
    font-family: var(--font-mono);
    font-size: var(--fs-2xs);
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--fg-3);
    white-space: nowrap;
  }
  .tabs {
    display: flex;
    align-items: center;
    gap: 2px;
    min-width: 0;
    overflow-x: auto;
    scrollbar-width: none;
    -webkit-app-region: no-drag;
  }
  .tabs::-webkit-scrollbar {
    display: none;
  }
  /* Tabs are stamped words; the open one is pressed into the bar and glows at its foot. */
  .tab {
    flex: none;
    height: 24px;
    white-space: nowrap;
    appearance: none;
    border: 1px solid transparent;
    background: transparent;
    color: var(--fg-3);
    font-size: var(--fs-2xs);
    font-weight: 600;
    letter-spacing: 0.15em;
    text-transform: uppercase;
    padding: 0 9px;
    cursor: var(--cursor-pointer, pointer);
  }
  .tabs.compact .tab {
    display: grid;
    place-items: center;
    padding: 0 10px;
  }
  .measure {
    position: absolute;
    width: 0;
    height: 0;
    overflow: hidden;
    visibility: hidden;
  }
  .measure .tabs {
    width: max-content;
    overflow: visible;
  }
  .tab:hover:not(:disabled) {
    color: var(--fg-0);
  }
  .tab.active {
    color: var(--fg-0);
    background: var(--well-bg);
    border-color: var(--well-edge);
    box-shadow: var(--well-shadow), inset 0 -2px 0 var(--heat);
  }
  .tab:disabled {
    color: var(--fg-4);
    cursor: var(--cursor-arrow, default);
  }
  .spacer {
    min-width: 0;
  }
  .controls {
    display: flex;
    -webkit-app-region: no-drag;
  }
  .findcell {
    display: flex;
    align-items: center;
    gap: 8px;
    padding-right: 8px;
    -webkit-app-region: no-drag;
  }
  .find {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 24px;
    min-width: 170px;
    padding: 0 6px 0 10px;
    color: var(--fg-3);
    font: inherit;
    font-size: var(--fs-xs);
    white-space: nowrap;
  }
  .findlabel {
    flex: 1;
    text-align: left;
  }
  .find:hover {
    color: var(--fg-1);
  }
  .save {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 24px;
    padding: 0 14px;
    border: 1px solid var(--primary-edge);
    border-radius: var(--r-1);
    background: var(--primary-bg);
    box-shadow: var(--primary-shadow);
    color: var(--primary-fg);
    font: inherit;
    font-size: var(--fs-2xs);
    font-weight: 700;
    letter-spacing: 0.16em;
    text-transform: uppercase;
    white-space: nowrap;
  }
  .save:hover {
    background: var(--primary-hover-bg);
  }
  .save .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: currentColor;
  }
  .wc {
    appearance: none;
    width: 46px;
    border: 0;
    background: transparent;
    color: var(--fg-2);
    cursor: var(--cursor-arrow, default);
    display: grid;
    place-items: center;
  }
  .wc:hover {
    background: var(--bg-hover);
    color: var(--fg-0);
  }
  .wc.opts.on {
    color: var(--fg-0);
    background: var(--bg-active);
  }
  .wc.opts {
    width: 40px;
    margin-right: 6px;
    border-right: 1px solid var(--line-0);
    cursor: var(--cursor-pointer, pointer);
  }
  .wc.close:hover {
    background: #c42b1c;
    color: #fff;
  }
  @media (max-width: 1500px) {
    .findlabel {
      display: none;
    }
    .find {
      min-width: 0;
    }
  }
  @media (max-width: 1360px) {
    .wordmark,
    .blevel {
      display: none;
    }
  }
</style>
