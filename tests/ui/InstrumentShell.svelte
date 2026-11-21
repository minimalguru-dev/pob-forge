<script lang="ts">
  import TitleBar from "../../src/lib/components/TitleBar.svelte";
  import Sidebar from "../../src/lib/components/Sidebar.svelte";
  import ChangeLedger from "../../src/lib/components/ChangeLedger.svelte";
</script>

<div class="app">
  <TitleBar />
  <div class="fg-heat" aria-hidden="true"></div>
  <div class="body">
    <Sidebar />
    <main class="view"></main>
  </div>
  <ChangeLedger />
</div>

<style>
  .app {
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--page);
  }
  .body {
    flex: 1;
    display: flex;
    min-height: 0;
  }
  .view {
    flex: 1;
  }
</style>
