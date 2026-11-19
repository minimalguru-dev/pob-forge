<script lang="ts">
  let { class: cls = "" }: { class?: string } = $props();
</script>

<!-- A hammer over an ember: the head and handle take the text colour, the ember the brand colour. -->
<svg class={cls} viewBox="142 142 740 740" aria-hidden="true" focusable="false">
  <path fill="currentColor" d="M218 182H806A56 56 0 0 1 862 238V382H162V238A56 56 0 0 1 218 182Z" />
  <path fill="currentColor" d="M282 422H422V842H338A56 56 0 0 1 282 786Z" />
  <path fill="var(--brand)" d="M672 482L842 652L672 822L502 652Z" />
</svg>
