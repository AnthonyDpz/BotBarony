<script>
  import { onMount } from 'svelte';
  import Setup from './lib/Setup.svelte';
  import Dashboard from './lib/Dashboard.svelte';
  import History from './lib/History.svelte';

  /** @type {'setup' | 'dashboard' | 'history'} */
  let currentPage = 'setup';

  const navItems = [
    { id: 'setup',     label: 'Setup',     icon: '⚙️' },
    { id: 'dashboard', label: 'Dashboard', icon: '🎮' },
    { id: 'history',   label: 'History',   icon: '📜' },
  ];
</script>

<main class="app-shell">
  <!-- Sidebar navigation -->
  <nav class="sidebar">
    <div class="logo">
      <span class="logo-icon">⚔️</span>
      <span class="logo-text">BotBarony</span>
    </div>
    <ul class="nav-list">
      {#each navItems as item}
        <li>
          <button
            class="nav-item"
            class:active={currentPage === item.id}
            on:click={() => (currentPage = item.id)}
          >
            <span class="nav-icon">{item.icon}</span>
            <span class="nav-label">{item.label}</span>
          </button>
        </li>
      {/each}
    </ul>
    <div class="sidebar-footer">
      <span class="version">v0.1.0</span>
    </div>
  </nav>

  <!-- Main content area -->
  <section class="content">
    {#if currentPage === 'setup'}
      <Setup on:saved={() => (currentPage = 'dashboard')} />
    {:else if currentPage === 'dashboard'}
      <Dashboard />
    {:else if currentPage === 'history'}
      <History />
    {/if}
  </section>
</main>

<style>
  :global(*) {
    box-sizing: border-box;
    margin: 0;
    padding: 0;
  }

  :global(body) {
    font-family: 'Inter', system-ui, sans-serif;
    background: #0d0d0d;
    color: #e8e8e8;
    height: 100vh;
    overflow: hidden;
  }

  .app-shell {
    display: flex;
    height: 100vh;
  }

  /* ── Sidebar ─────────────────────────────────────────────────────────────── */
  .sidebar {
    width: 200px;
    min-width: 200px;
    background: #161616;
    border-right: 1px solid #2a2a2a;
    display: flex;
    flex-direction: column;
    padding: 1.25rem 0;
  }

  .logo {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0 1.25rem 1.5rem;
    border-bottom: 1px solid #2a2a2a;
  }

  .logo-icon { font-size: 1.4rem; }
  .logo-text  { font-size: 1rem; font-weight: 700; color: #c9a84c; }

  .nav-list {
    list-style: none;
    flex: 1;
    padding: 0.75rem 0;
  }

  .nav-item {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    width: 100%;
    padding: 0.65rem 1.25rem;
    border: none;
    background: transparent;
    color: #888;
    font-size: 0.9rem;
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
    text-align: left;
  }

  .nav-item:hover  { background: #1e1e1e; color: #e8e8e8; }
  .nav-item.active { background: #1e1e1e; color: #c9a84c; font-weight: 600; }

  .nav-icon  { font-size: 1rem; }
  .nav-label { font-size: 0.9rem; }

  .sidebar-footer {
    padding: 0 1.25rem;
    font-size: 0.7rem;
    color: #444;
  }

  /* ── Content ─────────────────────────────────────────────────────────────── */
  .content {
    flex: 1;
    overflow: auto;
  }
</style>
