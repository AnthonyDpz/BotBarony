<script>
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { onMount, onDestroy } from 'svelte';

  let runId = null;
  let isRunning = false;
  let logs = [];
  let gameState = {
    level: 0,
    hp: [0, 100],
    mp: [0, 50],
    xp: 0,
    char_level: 1,
    turn: 0,
    inventory: {},
    status_effects: [],
    recent_events: [],
  };

  let unlistenLog = null;
  let unlistenState = null;

  /** Scrollable log container ref */
  let logContainer;

  onMount(async () => {
    // Listen to real-time events emitted by the Rust backend.
    unlistenLog = await listen('bot:log', (event) => {
      logs = [...logs, { ts: new Date().toLocaleTimeString(), msg: event.payload }];
      // Auto-scroll to bottom.
      setTimeout(() => {
        if (logContainer) logContainer.scrollTop = logContainer.scrollHeight;
      }, 0);
    });

    unlistenState = await listen('bot:state', (event) => {
      gameState = event.payload;
    });
  });

  onDestroy(() => {
    unlistenLog?.();
    unlistenState?.();
  });

  async function startRun() {
    logs = [];
    try {
      // Config is normally sourced from a store; hardcoded here for skeleton.
      runId = await invoke('start_run', {
        config: {
          provider: 'ollama',
          base_url: null,
          api_key: null,
          model: 'llama3',
          character_class: 'human',
          barony_executable: null,
        },
      });
      isRunning = true;
    } catch (e) {
      logs = [...logs, { ts: new Date().toLocaleTimeString(), msg: `ERROR: ${e}` }];
    }
  }

  async function stopRun() {
    if (!runId) return;
    try {
      await invoke('stop_run', { runId });
    } catch (e) {
      console.error(e);
    } finally {
      isRunning = false;
      runId = null;
    }
  }

  function clearLogs() { logs = []; }

  $: hpPct = gameState.hp[1] > 0 ? (gameState.hp[0] / gameState.hp[1]) * 100 : 0;
  $: mpPct = gameState.mp[1] > 0 ? (gameState.mp[0] / gameState.mp[1]) * 100 : 0;
  $: inventoryEntries = Object.entries(gameState.inventory || {});
</script>

<div class="page">
  <header class="page-header">
    <div>
      <h1>Dashboard</h1>
      <p class="subtitle">Monitor and control the active bot run.</p>
    </div>
    <div class="run-controls">
      {#if isRunning}
        <span class="run-badge running">Running</span>
        <button class="btn btn-danger" on:click={stopRun}>Stop Run</button>
      {:else}
        <span class="run-badge idle">Idle</span>
        <button class="btn btn-primary" on:click={startRun}>Start Run</button>
      {/if}
    </div>
  </header>

  <div class="dashboard-grid">
    <!-- ── Character stats ───────────────────────────────────────────────────── -->
    <section class="card stats-card">
      <h2 class="card-title">Character</h2>

      <div class="stat-row">
        <span class="stat-label">Level</span>
        <span class="stat-value">{gameState.char_level}</span>
      </div>
      <div class="stat-row">
        <span class="stat-label">Dungeon</span>
        <span class="stat-value">Floor {gameState.level}</span>
      </div>
      <div class="stat-row">
        <span class="stat-label">Turn</span>
        <span class="stat-value">{gameState.turn}</span>
      </div>

      <div class="bar-group">
        <div class="bar-header">
          <span>HP</span>
          <span>{gameState.hp[0]} / {gameState.hp[1]}</span>
        </div>
        <div class="bar-track">
          <div class="bar-fill hp-fill" style="width:{hpPct}%"></div>
        </div>
      </div>

      <div class="bar-group">
        <div class="bar-header">
          <span>MP</span>
          <span>{gameState.mp[0]} / {gameState.mp[1]}</span>
        </div>
        <div class="bar-track">
          <div class="bar-fill mp-fill" style="width:{mpPct}%"></div>
        </div>
      </div>

      {#if gameState.status_effects?.length}
        <div class="effects">
          {#each gameState.status_effects as fx}
            <span class="fx-badge">{fx}</span>
          {/each}
        </div>
      {/if}
    </section>

    <!-- ── Inventory ─────────────────────────────────────────────────────────── -->
    <section class="card inventory-card">
      <h2 class="card-title">Inventory</h2>
      {#if inventoryEntries.length === 0}
        <p class="empty-state">No items yet.</p>
      {:else}
        <ul class="item-list">
          {#each inventoryEntries as [name, qty]}
            <li class="item-row">
              <span class="item-name">{name}</span>
              {#if qty > 1}
                <span class="item-qty">×{qty}</span>
              {/if}
            </li>
          {/each}
        </ul>
      {/if}
    </section>

    <!-- ── Live log ──────────────────────────────────────────────────────────── -->
    <section class="card log-card">
      <div class="log-header">
        <h2 class="card-title">Live Log</h2>
        <button class="btn btn-ghost" on:click={clearLogs}>Clear</button>
      </div>
      <div class="log-body" bind:this={logContainer}>
        {#if logs.length === 0}
          <p class="empty-state">Waiting for events…</p>
        {:else}
          {#each logs as entry}
            <div class="log-line">
              <span class="log-ts">{entry.ts}</span>
              <span class="log-msg">{entry.msg}</span>
            </div>
          {/each}
        {/if}
      </div>
    </section>
  </div>
</div>

<style>
  .page { padding: 2rem; height: 100%; display: flex; flex-direction: column; }
  .page-header {
    display: flex; justify-content: space-between; align-items: flex-start;
    margin-bottom: 1.5rem;
  }
  .page-header h1 { font-size: 1.6rem; color: #c9a84c; }
  .subtitle { color: #888; font-size: 0.9rem; margin-top: 0.25rem; }

  .run-controls { display: flex; align-items: center; gap: 0.75rem; }
  .run-badge {
    padding: 0.25rem 0.75rem; border-radius: 20px; font-size: 0.78rem; font-weight: 600;
  }
  .running { background: #0d2b0d; color: #4caf50; border: 1px solid #1a5c1a; }
  .idle    { background: #1a1a1a; color: #666;    border: 1px solid #333; }

  .dashboard-grid {
    display: grid;
    grid-template-columns: 220px 220px 1fr;
    grid-template-rows: auto;
    gap: 1rem;
    flex: 1;
    min-height: 0;
  }

  .card {
    background: #1a1a1a; border: 1px solid #2a2a2a;
    border-radius: 8px; padding: 1.25rem;
  }
  .card-title { font-size: 0.95rem; font-weight: 600; color: #e0e0e0; margin-bottom: 1rem; }

  /* Stats */
  .stat-row {
    display: flex; justify-content: space-between;
    padding: 0.35rem 0; border-bottom: 1px solid #222; font-size: 0.85rem;
  }
  .stat-label { color: #888; }
  .stat-value { color: #e0e0e0; font-weight: 500; }

  .bar-group { margin-top: 1rem; }
  .bar-header {
    display: flex; justify-content: space-between;
    font-size: 0.78rem; color: #888; margin-bottom: 0.3rem;
  }
  .bar-track { height: 8px; background: #111; border-radius: 4px; overflow: hidden; }
  .bar-fill { height: 100%; border-radius: 4px; transition: width 0.4s ease; }
  .hp-fill { background: #4caf50; }
  .mp-fill { background: #2196f3; }

  .effects { display: flex; flex-wrap: wrap; gap: 0.4rem; margin-top: 0.75rem; }
  .fx-badge {
    padding: 0.15rem 0.5rem; border-radius: 3px; font-size: 0.7rem;
    background: #2a1a1a; color: #f88; border: 1px solid #5c1a1a;
  }

  /* Inventory */
  .item-list { list-style: none; }
  .item-row {
    display: flex; justify-content: space-between; align-items: center;
    padding: 0.35rem 0; border-bottom: 1px solid #222; font-size: 0.85rem;
  }
  .item-name { color: #e0e0e0; }
  .item-qty  { color: #c9a84c; font-weight: 600; }
  .empty-state { color: #555; font-size: 0.85rem; font-style: italic; }

  /* Log */
  .log-card {
    grid-column: 3;
    grid-row: 1 / -1;
    display: flex; flex-direction: column;
  }
  .log-header {
    display: flex; justify-content: space-between; align-items: center;
    margin-bottom: 0.75rem;
  }
  .log-header .card-title { margin-bottom: 0; }
  .log-body {
    flex: 1; overflow-y: auto; font-family: 'JetBrains Mono', monospace;
    font-size: 0.78rem; background: #111; border-radius: 5px;
    padding: 0.75rem; min-height: 0;
  }
  .log-line { display: flex; gap: 0.75rem; padding: 0.15rem 0; line-height: 1.5; }
  .log-ts  { color: #555; white-space: nowrap; flex-shrink: 0; }
  .log-msg { color: #ccc; word-break: break-word; }

  .btn {
    padding: 0.45rem 1rem; border-radius: 5px; font-size: 0.85rem;
    cursor: pointer; border: none; transition: opacity 0.15s, background 0.15s;
  }
  .btn-primary { background: #c9a84c; color: #0d0d0d; font-weight: 600; }
  .btn-primary:hover { background: #dbbe6e; }
  .btn-danger  { background: #5c1a1a; color: #f44336; border: 1px solid #882020; }
  .btn-danger:hover  { background: #7a2020; }
  .btn-ghost   { background: transparent; color: #555; border: 1px solid #333; padding: 0.25rem 0.6rem; font-size: 0.78rem; }
  .btn-ghost:hover   { color: #999; border-color: #555; }
</style>
