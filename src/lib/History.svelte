<script>
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';

  let runs = [];
  let loading = true;
  let selected = null;
  let error = '';

  onMount(async () => {
    try {
      runs = await invoke('get_run_history');
      runs.sort((a, b) => new Date(b.ended_at) - new Date(a.ended_at));
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  });

  function formatDuration(secs) {
    if (secs < 60) return `${secs}s`;
    const m = Math.floor(secs / 60);
    const s = secs % 60;
    return `${m}m ${s}s`;
  }

  function formatDate(iso) {
    return new Date(iso).toLocaleString();
  }

  function selectRun(run) {
    selected = selected?.id === run.id ? null : run;
  }
</script>

<div class="page">
  <header class="page-header">
    <h1>History</h1>
    <p class="subtitle">{runs.length} run{runs.length !== 1 ? 's' : ''} recorded.</p>
  </header>

  {#if loading}
    <p class="empty-state">Loading history…</p>
  {:else if error}
    <p class="error-msg">{error}</p>
  {:else if runs.length === 0}
    <div class="empty-card">
      <p class="empty-state">No runs yet. Start a run from the Dashboard!</p>
    </div>
  {:else}
    <div class="history-layout">
      <!-- Run list -->
      <ul class="run-list">
        {#each runs as run}
          <li>
            <button
              class="run-row"
              class:selected={selected?.id === run.id}
              on:click={() => selectRun(run)}
            >
              <div class="run-row-main">
                <span class="run-class">{run.character_class}</span>
                <span class="run-depth">Floor {run.dungeon_level_reached}</span>
              </div>
              <div class="run-row-sub">
                <span class="run-date">{formatDate(run.ended_at)}</span>
                <span class="run-duration">{formatDuration(run.duration_secs)}</span>
              </div>
              <div class="run-death">{run.cause_of_death}</div>
            </button>
          </li>
        {/each}
      </ul>

      <!-- Run detail -->
      {#if selected}
        <section class="run-detail card">
          <h2 class="detail-title">
            {selected.character_class.toUpperCase()} — Floor {selected.dungeon_level_reached}
          </h2>

          <div class="detail-grid">
            <div class="detail-item">
              <span class="detail-label">Date</span>
              <span class="detail-value">{formatDate(selected.ended_at)}</span>
            </div>
            <div class="detail-item">
              <span class="detail-label">Duration</span>
              <span class="detail-value">{formatDuration(selected.duration_secs)}</span>
            </div>
            <div class="detail-item">
              <span class="detail-label">Character Level</span>
              <span class="detail-value">{selected.char_level}</span>
            </div>
            <div class="detail-item">
              <span class="detail-label">Turns</span>
              <span class="detail-value">{selected.turns}</span>
            </div>
            <div class="detail-item">
              <span class="detail-label">AI Provider</span>
              <span class="detail-value">{selected.provider} / {selected.model}</span>
            </div>
          </div>

          <div class="detail-section">
            <h3 class="section-label">Cause of Death</h3>
            <p class="death-text">{selected.cause_of_death}</p>
          </div>

          <div class="detail-section">
            <h3 class="section-label">AI Analysis</h3>
            <pre class="analysis-text">{selected.ai_analysis}</pre>
          </div>

          {#if selected.lua_patches_applied?.length}
            <div class="detail-section">
              <h3 class="section-label">Lua Patches Applied</h3>
              <ul class="patch-list">
                {#each selected.lua_patches_applied as patch}
                  <li class="patch-item">{patch}</li>
                {/each}
              </ul>
            </div>
          {/if}
        </section>
      {:else}
        <div class="no-selection card">
          <p class="empty-state">Select a run to view its analysis.</p>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .page { padding: 2rem; height: 100%; display: flex; flex-direction: column; }
  .page-header { margin-bottom: 1.5rem; }
  .page-header h1 { font-size: 1.6rem; color: #c9a84c; }
  .subtitle { color: #888; font-size: 0.9rem; margin-top: 0.25rem; }

  .history-layout {
    display: grid;
    grid-template-columns: 300px 1fr;
    gap: 1rem;
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  .run-list { list-style: none; overflow-y: auto; }
  .run-row {
    display: block; width: 100%; text-align: left;
    background: #1a1a1a; border: 1px solid #2a2a2a;
    border-radius: 7px; padding: 0.85rem 1rem;
    margin-bottom: 0.5rem; cursor: pointer;
    transition: border-color 0.15s, background 0.15s;
  }
  .run-row:hover    { background: #1f1f1f; border-color: #444; }
  .run-row.selected { border-color: #c9a84c; background: #1f1b0e; }

  .run-row-main {
    display: flex; justify-content: space-between;
    align-items: baseline; margin-bottom: 0.25rem;
  }
  .run-class  { font-weight: 600; color: #e0e0e0; text-transform: capitalize; }
  .run-depth  { font-size: 0.85rem; color: #c9a84c; }
  .run-row-sub {
    display: flex; justify-content: space-between;
    font-size: 0.75rem; color: #666; margin-bottom: 0.3rem;
  }
  .run-death  { font-size: 0.78rem; color: #f44336; font-style: italic; }

  /* Detail panel */
  .card {
    background: #1a1a1a; border: 1px solid #2a2a2a;
    border-radius: 8px; padding: 1.5rem; overflow-y: auto;
  }
  .no-selection { display: flex; align-items: center; justify-content: center; }

  .detail-title { font-size: 1.1rem; color: #c9a84c; margin-bottom: 1rem; }
  .detail-grid {
    display: grid; grid-template-columns: 1fr 1fr;
    gap: 0.5rem; margin-bottom: 1.25rem;
  }
  .detail-item { display: flex; flex-direction: column; gap: 0.15rem; }
  .detail-label { font-size: 0.72rem; color: #666; text-transform: uppercase; letter-spacing: 0.05em; }
  .detail-value { font-size: 0.88rem; color: #e0e0e0; }

  .detail-section { margin-top: 1.25rem; }
  .section-label  { font-size: 0.78rem; color: #888; text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 0.5rem; }
  .death-text     { color: #f44336; font-size: 0.88rem; }
  .analysis-text  {
    background: #111; border-radius: 5px; padding: 0.75rem;
    font-size: 0.8rem; color: #ccc; white-space: pre-wrap;
    word-break: break-word; font-family: 'JetBrains Mono', monospace;
    max-height: 300px; overflow-y: auto;
  }

  .patch-list { list-style: none; }
  .patch-item {
    display: inline-block; margin: 0.2rem 0.3rem 0.2rem 0;
    padding: 0.2rem 0.6rem; border-radius: 3px;
    background: #0d2b0d; color: #4caf50;
    border: 1px solid #1a5c1a; font-size: 0.78rem;
  }

  .empty-state { color: #555; font-style: italic; font-size: 0.9rem; }
  .error-msg   { color: #f44336; font-size: 0.88rem; }
  .empty-card  {
    background: #1a1a1a; border: 1px solid #2a2a2a;
    border-radius: 8px; padding: 3rem; text-align: center;
  }
</style>
