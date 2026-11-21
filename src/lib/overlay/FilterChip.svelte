<script lang="ts">
  /**
   * A filter as Awakened PoE Trade draws it: a word that turns the filter on or off, and for a
   * number a field beside it ("Item Level: 84"). Typing in the field turns it on; emptying it
   * turns it off. On is a lit border, off is the word dimmed. No checkbox.
   */
  let {
    label,
    on = $bindable(false),
    value = $bindable(""),
    numeric = false,
    tone = "",
    title = "",
    disabled = false,
    onclick,
  }: {
    label: string;
    on?: boolean;
    value?: string;
    numeric?: boolean;
    tone?: string;
    title?: string;
    disabled?: boolean;
    /** Replaces the plain on/off toggle, for a chip that flips between two meanings. */
    onclick?: () => void;
  } = $props();

  function toggle() {
    if (disabled) return;
    if (onclick) onclick();
    else on = !on;
  }
</script>

{#if numeric}
  <span class="chip {tone}" class:on class:disabled {title}>
    <button type="button" class="word" {disabled} onclick={toggle}>{label}</button>
    <input
      class="mono"
      bind:value
      {disabled}
      placeholder="—"
      aria-label={label}
      onfocus={(e) => {
        on = true;
        (e.currentTarget as HTMLInputElement).select();
      }}
      onblur={() => {
        if (!value.trim()) on = false;
      }}
    />
  </span>
{:else}
  <button type="button" class="chip word {tone}" class:on class:disabled {title} {disabled} onclick={toggle}>{label}</button>
{/if}

<style>
  .chip {
    display: inline-flex;
    align-items: center;
    height: 22px;
    padding: 0 7px;
    border: 1px solid transparent;
    border-radius: 4px;
    background: var(--bg-0);
    color: var(--fg-3);
    font: inherit;
    font-size: 12px;
    white-space: nowrap;
  }
  .chip.on {
    border-color: var(--fg-3);
    color: var(--fg-0);
  }
  .chip.disabled {
    opacity: 0.5;
  }
  .chip.bad.on {
    color: var(--bad);
  }
  .word {
    cursor: pointer;
  }
  span.chip {
    padding: 0 2px 0 0;
  }
  span.chip .word {
    height: 100%;
    padding: 0 3px 0 7px;
    border: 0;
    background: transparent;
    color: inherit;
    font: inherit;
  }
  input {
    width: 3.2em;
    height: 18px;
    padding: 0 2px;
    border: 1px solid transparent;
    border-radius: 3px;
    background: transparent;
    color: inherit;
    font-size: 12px;
    text-align: center;
    outline: none;
  }
  input:hover,
  input:focus {
    background: var(--bg-hover);
    border-color: var(--line-2);
  }
</style>
