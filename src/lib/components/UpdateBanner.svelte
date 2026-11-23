<script lang="ts">
  import { appUpdate } from "$lib/state/update.svelte";
  import Markdown from "$lib/components/Markdown.svelte";
  import { m } from "$lib/paraglide/messages";

  let notesOpen = $state(false);
</script>

{#if appUpdate.showBanner}
  <div class="banner" role="status">
    <div class="line">
      <span class="txt">{m.update_ready_banner({ version: appUpdate.version ?? "" })}</span>
      {#if appUpdate.notes}
        <button class="btn sm ghost" aria-expanded={notesOpen} onclick={() => (notesOpen = !notesOpen)}>{m.update_whats_new()}</button>
      {/if}
      <button class="btn sm primary" disabled={appUpdate.installing} onclick={() => appUpdate.restart()}>{m.update_restart()}</button>
      <button class="btn sm ghost" onclick={() => appUpdate.dismiss()}>{m.update_later()}</button>
    </div>
    {#if notesOpen && appUpdate.notes}
      <div class="notes"><Markdown text={appUpdate.notes} /></div>
    {/if}
  </div>
{/if}

<style>
  .banner {
    padding: 6px 12px;
    background: var(--bg-2);
    border-bottom: 1px solid var(--line-1);
    font-size: var(--fs-sm);
    color: var(--fg-1);
  }
  .line {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .txt {
    flex: 1;
  }
  .notes {
    max-height: 40vh;
    overflow: auto;
    margin-top: 6px;
    padding-top: 6px;
    border-top: 1px solid var(--line-0);
  }
</style>
