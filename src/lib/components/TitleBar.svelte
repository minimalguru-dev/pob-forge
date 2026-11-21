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
    { id: "notes", label: m.view_notes(), key: "7", icon: "note-pencil" },
    { id: "party", label: m.view_party(), key: "8", icon: "users-three" },
    { id: "optimise", label: m.view_optimise(), key: "9", icon: "chart-line-up" },
    { id: "compare", label: m.view_compare(), key: "", icon: "scales" },
    { id: "regex", label: m.view_regex(), key: "", icon: "magnifying-glass" },
  ]);

  /** The four views a build is made in sit in one switch; the rest are plain words beside it. */
  const PRIMARY: ViewId[] = ["items", "tree", "skills", "config"];
  const primary = $derived(PRIMARY.map((id) => tabs.find((t) => t.id === id)!));
  const secondary = $derived(tabs.filter((t) => !PRIMARY.includes(t.id) && t.id !== "compare"));
  const levelLine = $derived(
    build.info ? [`lvl ${build.info.level}`, build.info.ascendClassName ?? build.info.className].filter(Boolean).join(" · ") : "",
  );

  function tabTitle(t: { label: string; key: string }) {
    const shortcut = t.key ? `Ctrl+${t.key}` : "";
    if (!compact) return shortcut || undefined;
    return shortcut ? `${t.label} (${shortcut})` : t.label;
  }

  // the build name lives in the sidebar; the OS title carries it for the taskbar
  $effect(() => {
    const name = build.info?.name;
    win.setTitle(name ? `${name}${build.info?.unsaved ? " •" : ""} — PoB Forge` : "PoB Forge").catch(() => {});
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
    <span class="wordmark" title="PoB Forge" data-tauri-drag-region>forge<span class="slash">/</span></span>
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
      <span class="bname" title={build.info.file ?? m.sidebar_not_saved()}>{build.info.name}</span>
      <span class="blevel">{levelLine}</span>
    {/if}
    <div class="views" role="tablist" aria-label={m.titlebar_views()}>
      {#each primary as t (t.id)}
        <button
          role="tab"
          class="seg"
          class:active={build.view === t.id && !appOptions.open}
          aria-selected={build.view === t.id && !appOptions.open}
          disabled={!build.loaded}
          onclick={() => openView(t.id)}
          title={t.key ? `Ctrl+${t.key}` : undefined}
        >
          {t.label}{#if t.key}<span class="segkey">{t.key}</span>{/if}
        </button>
      {/each}
    </div>
  </div>

  <div class="tabs" class:compact role="tablist" bind:this={tabsEl}>
    {#each secondary as t (t.id)}
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
      {#each secondary as t (t.id)}<span class="tab">{t.label}</span>{/each}
    </div>
  </div>

  <div class="findcell">
    {#if build.loaded}
      <button class="find" title={m.palette_title()} onclick={() => (ui.paletteOpen = true)}>
        <Icon name="magnifying-glass" size={13} />
        <span class="findlabel">{m.palette_button()}</span>
        <Kbd keys="Mod+K" />
      </button>
      <button
        class="ghost"
        class:on={build.view === "compare" && !appOptions.open}
        aria-pressed={build.view === "compare" && !appOptions.open}
        onclick={() => openView("compare")}
      >
        {m.titlebar_compare_with()}
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
  .titlebar {
    height: var(--titlebar-h);
    display: grid;
    grid-template-columns: auto auto minmax(0, auto) minmax(0, 1fr) auto auto;
    align-items: stretch;
    background: linear-gradient(90deg, var(--glow), transparent 55%), var(--bg-1);
    border-bottom: 1px solid var(--line-0);
    -webkit-app-region: drag;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 10px 0 14px;
    border-right: 1px solid var(--line-0);
  }
  .sb {
    appearance: none;
    border: 0;
    background: transparent;
    color: var(--fg-3);
    margin-left: auto;
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
  .wordmark {
    font-family: var(--font-head);
    font-size: 15px;
    font-weight: 600;
    letter-spacing: 0.02em;
    color: var(--fg-0);
    white-space: nowrap;
    user-select: none;
  }
  .slash {
    color: var(--brand);
  }
  .lead {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
    padding: 0 14px 0 16px;
  }
  .lead .bname {
    flex: 0 1 auto;
    min-width: 0;
    max-width: 180px;
    font-weight: 600;
    color: var(--fg-0);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .blevel {
    flex: none;
    font-family: var(--font-mono);
    font-size: var(--fs-xs);
    color: var(--fg-3);
    white-space: nowrap;
  }
  .views {
    flex: none;
    display: flex;
    margin-left: 4px;
    border: 1px solid var(--line-1);
    border-radius: 6px;
    overflow: hidden;
    -webkit-app-region: no-drag;
  }
  .seg {
    display: flex;
    align-items: baseline;
    gap: 6px;
    height: 26px;
    padding: 0 13px;
    border: 0;
    background: transparent;
    color: var(--fg-2);
    font-size: var(--fs-sm);
    font-weight: 500;
    white-space: nowrap;
    line-height: 26px;
  }
  .seg + .seg {
    border-left: 1px solid var(--line-1);
  }
  .seg:hover:not(:disabled) {
    color: var(--fg-0);
  }
  .seg.active {
    background: var(--bg-hover);
    color: var(--fg-0);
  }
  .seg:disabled {
    color: var(--fg-4);
    cursor: var(--cursor-arrow, default);
  }
  .segkey {
    font-family: var(--font-mono);
    font-size: var(--fs-2xs);
    color: var(--fg-4);
  }
  .ghost {
    height: 26px;
    margin-right: 6px;
    padding: 0 11px;
    border: 1px solid var(--line-1);
    border-radius: 6px;
    background: transparent;
    color: var(--fg-1);
    font-size: var(--fs-sm);
    font-weight: 500;
    white-space: nowrap;
  }
  .ghost:hover,
  .ghost.on {
    background: var(--bg-hover);
    color: var(--fg-0);
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
  .tabs {
    display: flex;
    align-items: stretch;
    min-width: 0;
    overflow-x: auto;
    scrollbar-width: none;
    -webkit-app-region: no-drag;
  }
  .tabs::-webkit-scrollbar {
    display: none;
  }
  /* Tabs are words: the open one is lit and underlined, nothing is boxed. */
  .tab {
    flex: none;
    white-space: nowrap;
    appearance: none;
    border: 0;
    background: transparent;
    color: var(--fg-3);
    font-size: var(--fs-sm);
    padding: 0 10px;
    cursor: var(--cursor-pointer, pointer);
    position: relative;
    letter-spacing: 0.01em;
  }
  .tabs.compact .tab {
    display: grid;
    place-items: center;
    padding: 0 12px;
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
  }
  .tab.active::after {
    content: "";
    position: absolute;
    left: 10px;
    right: 10px;
    bottom: 8px;
    height: 1px;
    background: var(--brand);
  }
  .tabs.compact .tab.active::after {
    left: 8px;
    right: 8px;
    bottom: 5px;
  }
  .tab:disabled {
    color: var(--fg-4);
    cursor: var(--cursor-arrow, default);
  }
  .spacer {
    display: flex;
    align-items: center;
    justify-content: center;
    min-width: 0;
  }
  .controls {
    display: flex;
    -webkit-app-region: no-drag;
  }
  .findcell {
    display: flex;
    align-items: center;
    padding-right: 6px;
    -webkit-app-region: no-drag;
  }
  .save {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 26px;
    margin-right: 8px;
    padding: 0 14px;
    border: 0;
    border-radius: 6px;
    background: var(--save-bg);
    color: var(--save-fg);
    font: inherit;
    font-size: var(--fs-sm);
    font-weight: 600;
    white-space: nowrap;
  }
  .save:hover {
    background: var(--save-hover);
  }
  .save .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: currentColor;
  }
  .find {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 26px;
    margin-right: 6px;
    padding: 0 6px 0 10px;
    border: 1px solid var(--line-1);
    border-radius: 6px;
    background: transparent;
    color: var(--fg-3);
    font: inherit;
    font-size: var(--fs-xs);
    white-space: nowrap;
  }
  .find:hover {
    background: var(--bg-hover);
    color: var(--fg-1);
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
  }
  @media (max-width: 1360px) {
    .blevel {
      display: none;
    }
    .tab {
      padding: 0 11px;
    }
  }
</style>
