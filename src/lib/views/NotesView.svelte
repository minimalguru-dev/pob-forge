<script lang="ts">
  import { tick } from "svelte";
  import { engine, openUrl } from "$lib/engine.svelte";
  import { build } from "$lib/state/build.svelte";
  import { parsePobText } from "$lib/pobtext";
  import { noteLines, outline } from "$lib/notes";
  import { m } from "$lib/paraglide/messages";

  let text = $state("");
  let loadedFor = -1;
  let timer = 0;
  let area = $state<HTMLTextAreaElement | null>(null);
  let reader = $state<HTMLDivElement | null>(null);
  let editing = $state(false);

  // PoB's own notes palette, from Data/Global.lua.
  const COLOURS: [string, string][] = [
    ["NORMAL", "^xC8C8C8"],
    ["MAGIC", "^x8888FF"],
    ["RARE", "^xFFFF77"],
    ["UNIQUE", "^xAF6025"],
    ["FIRE", "^xB97123"],
    ["COLD", "^x3F6DB3"],
    ["LIGHTNING", "^xADAA47"],
    ["CHAOS", "^xD02090"],
    ["STRENGTH", "^xE05030"],
    ["DEXTERITY", "^x70FF70"],
    ["INTELLIGENCE", "^x7070FF"],
    ["DEFAULT", "^7"],
  ];
  const swatch = (code: string) => parsePobText(`${code}x`)[0]?.color ?? "var(--fg-0)";

  const lines = $derived(noteLines(text));
  const contents = $derived(outline(lines));

  /** Drop a colour code at the cursor, the way PoB's buttons do. */
  function insert(code: string) {
    const el = area;
    if (!el) return;
    const at = el.selectionStart ?? text.length;
    const to = el.selectionEnd ?? at;
    text = text.slice(0, at) + code + text.slice(to);
    onInput();
    queueMicrotask(() => {
      el.focus();
      el.selectionStart = el.selectionEnd = at + code.length;
    });
  }

  $effect(() => {
    const key = build.info ? build.info.name + build.info.file : null;
    if (key === null) return;
    // The engine restarts rev on load, so a drop means another build was opened.
    const rev = build.rev;
    if (loadedFor === -1 || rev < loadedFor) {
      engine.getNotes().then((r) => {
        text = r.text;
        editing = !r.text.trim();
      });
    }
    loadedFor = rev;
  });

  async function toggleEdit() {
    editing = !editing;
    if (editing) {
      await tick();
      area?.focus();
    }
  }

  function onInput() {
    clearTimeout(timer);
    timer = window.setTimeout(() => build.run(() => engine.setNotes(text), { sync: false }), 400);
  }

  function jump(line: number) {
    reader?.querySelector(`[data-line="${line}"]`)?.scrollIntoView({ block: "start", behavior: "smooth" });
  }

  function follow(e: MouseEvent, href: string) {
    e.preventDefault();
    void openUrl(href);
  }
</script>

{#snippet rendered(withLines: boolean)}
  {#each lines as l, i}
    <div class="line" class:h={l.heading} data-line={withLines ? i : undefined}>
      {#each l.pieces as p}
        {#if p.href}<a href={p.href} style:color={p.color ?? undefined} onclick={(e) => follow(e, p.href!)}>{p.text}</a
          >{:else}<span style:color={p.color ?? undefined}>{p.text}</span>{/if}
      {/each}{#if l.pieces.length === 0}&nbsp;{/if}
    </div>
  {/each}
{/snippet}

<div class="page">
  <div class="toolbar">
    <button class="btn sm" class:primary={editing} aria-pressed={editing} title={m.notes_edit_title()} onclick={toggleEdit}>
      {editing ? m.notes_done() : m.notes_edit()}
    </button>
    {#if editing}
      <span class="vr"></span>
      <div class="swatches" role="group" aria-label={m.notes_colours()}>
        {#each COLOURS as [name, code] (name)}
          <button class="swatch" title={m.notes_insert({ name, code })} aria-label={m.notes_insert({ name, code })} onclick={() => insert(code)}>
            <span class="dot" style:background={swatch(code)}></span>
          </button>
        {/each}
      </div>
    {/if}
  </div>

  {#if editing}
    <div class="split">
      <textarea class="editor selectable" bind:this={area} bind:value={text} oninput={onInput} placeholder={m.notes_placeholder()}
      ></textarea>
      <div class="text preview selectable" aria-label={m.notes_preview()}>
        {#if text.trim()}{@render rendered(false)}{:else}<span class="dim">{m.notes_placeholder()}</span>{/if}
      </div>
    </div>
  {:else if !text.trim()}
    <div class="empty dim">{m.notes_empty()}</div>
  {:else}
    <div class="read" class:with-outline={contents.length > 0}>
      {#if contents.length}
        <nav class="outline" aria-label={m.notes_outline()}>
          <div class="label">{m.notes_outline()}</div>
          {#each contents as c (c.line)}
            <button class="entry" onclick={() => jump(c.line)}>{c.title}</button>
          {/each}
        </nav>
      {/if}
      <div class="text selectable" bind:this={reader}>{@render rendered(true)}</div>
    </div>
  {/if}
</div>

<style>
  .page {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  .toolbar {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px;
    padding: 6px 10px;
    border-bottom: 1px solid var(--line-0);
    background: var(--bg-1);
  }
  .swatches {
    display: flex;
    gap: 2px;
  }
  .swatch {
    appearance: none;
    background: none;
    border: 1px solid transparent;
    border-radius: var(--r-2);
    padding: 3px;
    cursor: var(--cursor-pointer, pointer);
    display: grid;
    place-items: center;
  }
  .swatch:hover {
    border-color: var(--line-1);
    background: var(--bg-hover);
  }
  .dot {
    width: 12px;
    height: 12px;
    border-radius: 2px;
    box-shadow: inset 0 0 0 1px color-mix(in oklab, var(--fg-0) 18%, transparent);
  }
  .split {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: 1fr 1fr;
  }
  .editor {
    border: 0;
    border-right: 1px solid var(--line-0);
    outline: none;
    resize: none;
    background: var(--bg-0);
    color: var(--fg-0);
    padding: 16px 20px;
    font-family: var(--font-mono);
    font-size: var(--fs-sm);
    line-height: 1.55;
    min-height: 0;
  }
  .read {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: 1fr;
  }
  .read.with-outline {
    grid-template-columns: minmax(140px, 220px) 1fr;
  }
  .outline {
    overflow: auto;
    border-right: 1px solid var(--line-0);
    padding: 12px 8px;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .outline .label {
    font-size: var(--fs-xs);
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--fg-3);
    padding: 0 8px 6px;
  }
  .entry {
    appearance: none;
    background: none;
    border: 0;
    border-radius: var(--r-2);
    text-align: left;
    color: var(--fg-1);
    font-size: var(--fs-sm);
    padding: 4px 8px;
    cursor: var(--cursor-pointer, pointer);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .entry:hover {
    background: var(--bg-hover);
    color: var(--fg-0);
  }
  .text {
    overflow: auto;
    min-height: 0;
    padding: 16px 24px;
    font-size: var(--fs-md, 14px);
    line-height: 1.6;
    color: var(--fg-0);
    background: var(--bg-0);
  }
  .line {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .line.h {
    font-weight: 600;
    margin-top: 0.8em;
    scroll-margin-top: 12px;
  }
  .line.h:first-child {
    margin-top: 0;
  }
  .text a {
    text-decoration: underline;
    text-underline-offset: 2px;
    cursor: var(--cursor-pointer, pointer);
  }
  .text a:not([style]) {
    color: var(--accent);
  }
  .empty {
    padding: 24px;
  }
  @media (max-width: 900px) {
    .split {
      grid-template-columns: 1fr;
      grid-template-rows: 1fr 1fr;
    }
    .editor {
      border-right: 0;
      border-bottom: 1px solid var(--line-0);
    }
    .read.with-outline {
      grid-template-columns: 1fr;
    }
    .outline {
      display: none;
    }
  }
</style>
