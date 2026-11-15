<script lang="ts">
  import { onMount } from "svelte";
  import { open, save } from "@tauri-apps/plugin-dialog";
  import { call, fetchBuildCode, readTextFile, writeTextFile } from "$lib/engine.svelte";
  import { build } from "$lib/state/build.svelte";
  import { game } from "$lib/state/game.svelte";
  import { m } from "$lib/paraglide/messages";
  import { addSnapshot, assess, decodePlan, diffSnapshots, emptyPlan, numberStat, priorities, removeSnapshot, savePlan, STORAGE_KEY, type Boss, type Plan, type Snapshot } from "$lib/progression/model";

  let plan = $state<Plan>(emptyPlan());
  let ready = $state(false);
  let busy = $state(false);
  let error = $state("");
  let notice = $state("");
  let label = $state("");
  let input = $state("");
  let role = $state<"current" | "target">("current");
  let boss = $state<Boss>("Pinnacle");
  const current = $derived(plan.snapshots.find(s => s.id === plan.currentId));
  const target = $derived(plan.snapshots.find(s => s.id === plan.targetId));
  const diff = $derived(current && target ? diffSnapshots(current, target) : null);
  const checks = $derived(current && target ? assess(current, target) : []);
  const next = $derived(current && target ? priorities(current, target) : []);
  const blockers = $derived(checks.filter(c => c.status === "blocked").length);
  const statRows = [
    ["CombinedDPS", "Main skill combined DPS"], ["FullDPS", "Full DPS (configured skill counts)"],
    ["Speed", "Main skill rate / s"], ["Life", "Life"], ["EnergyShield", "Energy shield"],
    ["TotalEHP", "Effective hit pool"], ["PhysicalMaximumHitTaken", "Physical maximum hit"],
    ["FireMaximumHitTaken", "Fire maximum hit"], ["ColdMaximumHitTaken", "Cold maximum hit"],
    ["LightningMaximumHitTaken", "Lightning maximum hit"], ["ChaosMaximumHitTaken", "Chaos maximum hit"],
    ["FireResist", "Fire resistance %"], ["ColdResist", "Cold resistance %"], ["LightningResist", "Lightning resistance %"],
  ];
  const fmt = (s: Snapshot, key: string, profile: boolean) => {
    const value = numberStat(profile ? s.boss.stats : s.stats, key);
    return value === null ? "—" : value.toLocaleString(undefined, { maximumFractionDigits: 2 });
  };

  onMount(() => {
    try {
      const saved = localStorage.getItem(STORAGE_KEY);
      if (saved) plan = decodePlan(saved);
      ready = true;
    } catch (e) { error = String(e); }
  });

  function commit(candidate: Plan) {
    savePlan(localStorage, candidate);
    plan = candidate;
    ready = true;
  }

  async function run(action: () => void | Promise<void>) {
    if (busy) return;
    busy = true;
    error = "";
    notice = "";
    try { await action(); } catch (e) { error = String(e); } finally { busy = false; }
  }

  async function capture(live: boolean, fileText?: string) {
    await run(async () => {
      const text = (fileText ?? input).trim();
      if (!live && !text) throw new Error("Paste a PoB code, XML or supported build URL first.");
      let source: { code?: string; xml?: string } = {};
      if (!live) {
        if (/^https?:\/\//i.test(text)) source = { code: (await fetchBuildCode(text)).code };
        else if (text.startsWith("<")) source = { xml: text };
        else source = { code: text };
      }
      const snapshot = await call<Omit<Snapshot, "id" | "at">>("progression_capture", {
        ...source, label: label.trim() || (live ? build.info?.name : role === "current" ? "Current build" : "Target build"), boss,
      });
      if (!game.isPoe1) throw new Error("Game changed during capture. Return to PoE1 and capture again.");
      commit(addSnapshot(plan, { ...snapshot, id: crypto.randomUUID(), at: new Date().toISOString() }, role));
      notice = "Snapshot saved locally. The open build is unchanged.";
      input = "";
    });
  }

  async function importXml() {
    const path = await open({ multiple: false, filters: [{ name: "PoB build", extensions: ["xml"] }] });
    if (typeof path === "string") {
      try { await capture(false, await readTextFile(path)); } catch (e) { error = String(e); }
    }
  }

  async function exportHistory() {
    await run(async () => {
      const path = await save({ defaultPath: "progression-history.json", filters: [{ name: "Progression backup", extensions: ["json"] }] });
      if (path) {
        const text = ready ? JSON.stringify(plan) : localStorage.getItem(STORAGE_KEY);
        if (text) await writeTextFile(path, text);
      }
    });
  }

  async function importHistory() {
    await run(async () => {
      const path = await open({ multiple: false, filters: [{ name: "Progression backup", extensions: ["json"] }] });
      if (typeof path !== "string") return;
      const imported = decodePlan(await readTextFile(path));
      const snapshots = [...plan.snapshots, ...imported.snapshots.filter(s => !plan.snapshots.some(existing => existing.id === s.id))];
      commit({ schema: 1, snapshots, currentId: plan.currentId ?? imported.currentId, targetId: plan.targetId ?? imported.targetId });
      notice = "History imported. Stored calculations are historical; re-capture to verify them with this engine.";
    });
  }
</script>

<div class="page progression">
  <header>
    <div><div class="label">PATH OF EXILE 1</div><h1>{m.view_progression()}</h1><p class="dim">Plan a transition, preserve your history, and review the next changes.</p></div>
    <div class="actions"><button class="btn" disabled={busy} onclick={exportHistory}>Export history</button><button class="btn" disabled={busy} onclick={importHistory}>Import history</button></div>
  </header>
  {#if error}<p class="error" role="alert">{error}</p>{/if}
  {#if notice}<p role="status">{notice}</p>{/if}
  {#if !game.isPoe1}
    <section><h2>PoE1 only</h2><p>Choose Path of Exile 1 in the title bar to use Progression.</p></section>
  {:else}
    <section>
      <h2>Capture a build</h2>
      <p class="dim">For Shield Crush Slayer → Dual Strike of Ambidexterity Slayer, capture your current build and the intended target loadout. Choose the tree, item set and skills in PoB before exporting. No example DPS is supplied.</p>
      <div class="inputs">
        <label>Use as<select class="select" aria-label="Use as" bind:value={role} disabled={busy}><option value="current">Current</option><option value="target">Target</option></select></label>
        <label>Snapshot name<input class="input" bind:value={label} maxlength="160" placeholder="e.g. Shield Crush — early maps" disabled={busy} /></label>
        <label>Boss profile<select class="select" aria-label="Boss profile" bind:value={boss} disabled={busy}><option value="Boss">Standard boss</option><option value="Pinnacle">Guardian / Pinnacle</option><option value="Uber">Uber Pinnacle</option></select></label>
      </div>
      <label class="paste">PoB share code, XML or supported build URL<textarea bind:value={input} rows="3" disabled={busy} spellcheck="false" placeholder="Paste a build here. Import runs only when you press Capture."></textarea></label>
      <div class="actions">
        <button class="btn primary" disabled={busy || !ready || !input.trim()} onclick={() => capture(false)}>Capture pasted build</button>
        <button class="btn" disabled={busy || !ready || !build.loaded} onclick={() => capture(true)}>Capture open build</button>
        <button class="btn" disabled={busy || !ready} onclick={() => void importXml().catch(e => error = String(e))}>Open PoB XML…</button>
        {#if busy}<span role="status">Calculating with PoB…</span>{/if}
      </div>
    </section>
    <section>
      <h2>Local history <span class="dim">{plan.snapshots.length} / 20</span></h2>
      <p class="dim">Snapshots include the full build XML and are frozen in time. Capture again after changing a build. Export a backup before clearing application data.</p>
      {#if !plan.snapshots.length}<p>No snapshots yet. Capture a current and target build above.</p>{/if}
      {#each [...plan.snapshots].reverse() as snapshot (snapshot.id)}
        <div class="history">
          <div><strong>{snapshot.label}</strong><div class="dim small">{snapshot.mainSkill ?? "No main skill"} · {snapshot.ascendancy} {snapshot.level} · {new Date(snapshot.at).toLocaleString()} · PoB {snapshot.pobVersion}</div><div class="dim small">Tree: {snapshot.loadout.tree} · Item set: {snapshot.loadout.items} · Skill set: {snapshot.loadout.skills} · {snapshot.boss.kind}</div></div>
          <div class="actions">
            <button class="btn sm" class:on={plan.currentId === snapshot.id} disabled={busy} onclick={() => run(() => commit({ ...plan, currentId: snapshot.id }))}>Current</button>
            <button class="btn sm" class:on={plan.targetId === snapshot.id} disabled={busy} onclick={() => run(() => commit({ ...plan, targetId: snapshot.id }))}>Target</button>
            <button class="btn sm" disabled={busy || !ready} onclick={() => capture(false, snapshot.xml)}>Re-capture as {role}</button>
            <button class="btn sm ghost" disabled={busy} onclick={() => run(() => commit(removeSnapshot(plan, snapshot.id)))}>Remove</button>
          </div>
        </div>
      {/each}
    </section>
    {#if current && target && diff}
      <section>
        <div class="label">{current.label} → {target.label}</div>
        <h2>{blockers ? `${blockers} detected blocker(s)` : "No detected blockers — manual review required"}</h2>
        <p class="dim">A comparison of two complete builds, not a simulated partial gear swap. No readiness percentage or automatic switch approval.</p>
        <h3>Next priorities</h3>
        <p class="dim">Rules rank blockers before gear, gems and tree work. This is not a price or DPS-per-currency ranking. Use Optimize to calculate individual upgrades.</p>
        <ol>{#each next as check (check.id)}<li><strong>{check.title}</strong> <span class="badge" class:blocked={check.status === "blocked"}>{check.status}</span><p>{check.detail}</p></li>{/each}</ol>
        <div class="actions"><button class="btn" onclick={() => build.view = "optimise"}>Open Optimize</button><button class="btn" onclick={() => build.view = "compare"}>Open Compare</button><button class="btn" onclick={() => build.view = "config"}>Review Config</button></div>
        <details><summary>All readiness checks ({checks.length})</summary>{#each checks as check (check.id)}<p><strong>{check.title}</strong> · {check.status}<br /><span class="dim">{check.detail}</span></p>{/each}</details>
      </section>
      <section>
        <h2>Equipment changes</h2>
        {#if !diff.equipment.length}<p>Equipment matches, including item text and selected variants.</p>{/if}
        {#each diff.equipment as row (row.slot)}
          <details><summary><strong>{row.slot}</strong> · {row.current?.name ?? "Empty"} → {row.target?.name ?? "Empty"}</summary><div class="two"><pre>{row.current?.raw ?? "Empty"}</pre><pre>{row.target?.raw ?? "Empty"}</pre></div></details>
        {/each}
        <h3>Passive tree changes</h3>
        {#if !diff.compatibleTree}<p>Different tree versions or classes. Raw node IDs cannot be compared safely.</p>
        {:else}<div class="two"><div><h4>Add ({diff.gained.length})</h4>{#each diff.gained as node (node.id)}<div>{node.name} <span class="dim">#{node.id}</span></div>{/each}</div><div><h4>Remove ({diff.lost.length})</h4>{#each diff.lost as node (node.id)}<div>{node.name} <span class="dim">#{node.id}</span></div>{/each}</div></div>
          {#each diff.masteries as node (node.id)}<p>Mastery #{node.id}: {current.nodes.find(n => n.id === node.id)?.mastery ?? "None"} → {node.mastery ?? "None"}</p>{/each}
        {/if}
      </section>
      <section>
        <h2>Bossing profile</h2>
        <p>PoB calculates both profiles on temporary solo copies. The boss profile sets the selected boss category, disables flask activation, configured power/frenzy/endurance charges, configured Onslaught, Rampage and kill conditions.</p>
        <p class="dim">Other imported settings remain, including custom modifiers, enemy overrides, shock, rage, guard skills and skill counts. Automatic mechanics may still grant buffs. This is a scenario calculation, not measured combat DPS or a guarantee of sustain. Party buffs are not loaded by PoB CompareEntry.</p>
        {#if current.boss.kind !== target.boss.kind}<p class="error">Boss categories differ. Re-capture both with the same category.</p>{/if}
        <div class="table-wrap"><table><thead><tr><th>PoB statistic</th><th>Current configured</th><th>Target configured</th><th>Current {current.boss.kind}</th><th>Target {target.boss.kind}</th></tr></thead><tbody>
          {#each statRows as [key, title] (key)}<tr><th>{title}</th><td>{fmt(current, key, false)}</td><td>{fmt(target, key, false)}</td><td>{fmt(current, key, true)}</td><td>{fmt(target, key, true)}</td></tr>{/each}
        </tbody></table></div>
        <details><summary>Configuration differences ({diff.config.length})</summary>{#each diff.config as key (key)}<p><strong>{key}</strong>: {String(current.config[key] ?? "Default")} → {String(target.config[key] ?? "Default")}</p>{/each}</details>
        <details><summary>Exact boss profile overrides</summary><pre>{JSON.stringify(current.boss.overrides, null, 2)}</pre></details>
      </section>
    {/if}
    <footer class="dim small">Local build planning only. No game memory access, session cookies or gameplay automation. This product isn't affiliated with or endorsed by Grinding Gear Games in any way.</footer>
  {/if}
</div>

<style>
  .progression { overflow-y: auto; padding: 24px; display: flex; flex-direction: column; gap: 20px; }
  header, .history { display: flex; justify-content: space-between; align-items: center; gap: 16px; flex-wrap: wrap; }
  h1 { font-size: 28px; margin: 5px 0; } h2 { font-size: 19px; margin: 0 0 12px; } h3 { margin-top: 22px; }
  p { line-height: 1.55; } section { padding: 20px; background: var(--bg-1); border: 1px solid var(--line-1); border-radius: 10px; }
  .actions { display: flex; align-items: center; flex-wrap: wrap; gap: 8px; }
  .inputs { display: grid; grid-template-columns: 150px minmax(180px, 1fr) 200px; gap: 12px; }
  label { display: flex; flex-direction: column; gap: 7px; } .paste { margin: 16px 0 12px; }
  textarea { width: 100%; resize: vertical; padding: 10px; border: 1px solid var(--line-1); border-radius: 6px; background: var(--bg-0); color: var(--fg-1); font: inherit; }
  .history { padding: 14px 0; border-bottom: 1px solid var(--line-1); } .history .small { margin-top: 5px; }
  .error, .blocked { color: var(--bad); } .badge { font-size: 11px; text-transform: uppercase; padding-left: 8px; }
  li { padding: 6px 0; } li p { margin: 5px 0; color: var(--fg-3); }
  details { margin-top: 12px; } summary { cursor: pointer; padding: 8px 0; }
  .two { display: grid; grid-template-columns: 1fr 1fr; gap: 20px; }
  pre { white-space: pre-wrap; overflow-wrap: anywhere; font-size: 12px; user-select: text; }
  .table-wrap { overflow-x: auto; } table { border-collapse: collapse; width: 100%; font-size: 12px; }
  th, td { padding: 10px; border-bottom: 1px solid var(--line-1); text-align: right; } th:first-child { text-align: left; }
  @media (max-width: 900px) { .inputs { grid-template-columns: 1fr; } .two { grid-template-columns: 1fr; } }
</style>
