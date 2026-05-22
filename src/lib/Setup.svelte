<script>
  import { invoke } from '@tauri-apps/api/core';

  const PROVIDERS = [
    { id: 'ollama',   label: 'Ollama',      requiresKey: false, defaultUrl: 'http://localhost:11434' },
    { id: 'lmstudio', label: 'LM Studio',   requiresKey: false, defaultUrl: 'http://localhost:1234'  },
    { id: 'claude',   label: 'Claude API',  requiresKey: true,  defaultUrl: 'https://api.anthropic.com' },
    { id: 'openai',   label: 'OpenAI API',  requiresKey: true,  defaultUrl: 'https://api.openai.com' },
  ];

  let selectedProvider = PROVIDERS[0];
  let baseUrl = selectedProvider.defaultUrl;
  let apiKey = '';
  let selectedModel = '';
  let models = [];
  let healthStatus = null;   // null | 'ok' | 'error'
  let loadingModels = false;
  let loadingHealth = false;
  let errorMsg = '';

  // Barony config
  let baronyPath = '';
  let characterClass = 'human';
  const CLASSES = ['human', 'skeleton', 'automaton', 'goblin', 'incubus', 'succubus', 'vampire', 'goatman'];

  function onProviderChange(provider) {
    selectedProvider = provider;
    baseUrl = provider.defaultUrl;
    apiKey = '';
    models = [];
    selectedModel = '';
    healthStatus = null;
    errorMsg = '';
  }

  async function checkHealth() {
    loadingHealth = true;
    healthStatus = null;
    errorMsg = '';
    try {
      const ok = await invoke('check_provider_health', {
        provider: selectedProvider.id,
        baseUrl: baseUrl || null,
        apiKey: apiKey || null,
      });
      healthStatus = ok ? 'ok' : 'error';
    } catch (e) {
      healthStatus = 'error';
      errorMsg = String(e);
    } finally {
      loadingHealth = false;
    }
  }

  async function loadModels() {
    loadingModels = true;
    errorMsg = '';
    models = [];
    try {
      models = await invoke('list_models', {
        provider: selectedProvider.id,
        baseUrl: baseUrl || null,
        apiKey: apiKey || null,
      });
      if (models.length > 0) selectedModel = models[0];
    } catch (e) {
      errorMsg = String(e);
    } finally {
      loadingModels = false;
    }
  }

  function saveConfig() {
    // TODO: persist config to tauri store / app config
    alert('Configuration saved!');
  }
</script>

<div class="page setup-page">
  <header class="page-header">
    <h1>Setup</h1>
    <p class="subtitle">Configure your AI provider and Barony installation.</p>
  </header>

  <!-- ── Provider selection ─────────────────────────────────────────────────── -->
  <section class="card">
    <h2 class="card-title">AI Provider</h2>

    <div class="provider-tabs">
      {#each PROVIDERS as provider}
        <button
          class="provider-tab"
          class:active={selectedProvider.id === provider.id}
          on:click={() => onProviderChange(provider)}
        >
          {provider.label}
        </button>
      {/each}
    </div>

    <div class="form-grid">
      <label class="form-field">
        <span class="field-label">Base URL</span>
        <input class="field-input" type="text" bind:value={baseUrl} placeholder={selectedProvider.defaultUrl} />
      </label>

      {#if selectedProvider.requiresKey}
        <label class="form-field">
          <span class="field-label">API Key</span>
          <input class="field-input" type="password" bind:value={apiKey} placeholder="sk-..." />
        </label>
      {/if}
    </div>

    <div class="actions-row">
      <button class="btn btn-secondary" on:click={checkHealth} disabled={loadingHealth}>
        {loadingHealth ? 'Checking…' : 'Health Check'}
      </button>
      <button class="btn btn-secondary" on:click={loadModels} disabled={loadingModels}>
        {loadingModels ? 'Loading…' : 'Load Models'}
      </button>

      {#if healthStatus === 'ok'}
        <span class="status-badge status-ok">Connected</span>
      {:else if healthStatus === 'error'}
        <span class="status-badge status-error">Unreachable</span>
      {/if}
    </div>

    {#if errorMsg}
      <p class="error-msg">{errorMsg}</p>
    {/if}

    {#if models.length > 0}
      <label class="form-field" style="margin-top:1rem">
        <span class="field-label">Model</span>
        <select class="field-input field-select" bind:value={selectedModel}>
          {#each models as model}
            <option value={model}>{model}</option>
          {/each}
        </select>
      </label>
    {/if}
  </section>

  <!-- ── Barony configuration ────────────────────────────────────────────────── -->
  <section class="card">
    <h2 class="card-title">Barony Settings</h2>
    <div class="form-grid">
      <label class="form-field">
        <span class="field-label">Executable Path</span>
        <input class="field-input" type="text" bind:value={baronyPath}
          placeholder="/path/to/barony" />
      </label>
      <label class="form-field">
        <span class="field-label">Character Class</span>
        <select class="field-input field-select" bind:value={characterClass}>
          {#each CLASSES as cls}
            <option value={cls}>{cls.charAt(0).toUpperCase() + cls.slice(1)}</option>
          {/each}
        </select>
      </label>
    </div>
  </section>

  <div class="page-footer">
    <button class="btn btn-primary" on:click={saveConfig}>Save Configuration</button>
  </div>
</div>

<style>
  .page { padding: 2rem; max-width: 800px; }
  .page-header { margin-bottom: 2rem; }
  .page-header h1 { font-size: 1.6rem; color: #c9a84c; }
  .subtitle { color: #888; margin-top: 0.25rem; font-size: 0.9rem; }

  .card {
    background: #1a1a1a;
    border: 1px solid #2a2a2a;
    border-radius: 8px;
    padding: 1.5rem;
    margin-bottom: 1.25rem;
  }
  .card-title { font-size: 1rem; font-weight: 600; margin-bottom: 1rem; color: #e0e0e0; }

  .provider-tabs { display: flex; gap: 0.5rem; margin-bottom: 1.25rem; flex-wrap: wrap; }
  .provider-tab {
    padding: 0.4rem 0.9rem;
    border: 1px solid #333;
    background: transparent;
    color: #888;
    border-radius: 5px;
    cursor: pointer;
    font-size: 0.85rem;
    transition: all 0.15s;
  }
  .provider-tab:hover  { border-color: #555; color: #ccc; }
  .provider-tab.active { border-color: #c9a84c; color: #c9a84c; background: #1f1b0e; }

  .form-grid { display: flex; flex-direction: column; gap: 0.85rem; }
  .form-field { display: flex; flex-direction: column; gap: 0.3rem; }
  .field-label { font-size: 0.8rem; color: #888; text-transform: uppercase; letter-spacing: 0.05em; }
  .field-input {
    background: #111;
    border: 1px solid #333;
    border-radius: 5px;
    color: #e8e8e8;
    padding: 0.5rem 0.75rem;
    font-size: 0.9rem;
    outline: none;
    transition: border-color 0.15s;
  }
  .field-input:focus  { border-color: #c9a84c; }
  .field-select { cursor: pointer; }

  .actions-row { display: flex; align-items: center; gap: 0.75rem; margin-top: 1rem; flex-wrap: wrap; }

  .btn {
    padding: 0.45rem 1rem;
    border-radius: 5px;
    font-size: 0.85rem;
    cursor: pointer;
    border: none;
    transition: opacity 0.15s, background 0.15s;
  }
  .btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .btn-primary   { background: #c9a84c; color: #0d0d0d; font-weight: 600; }
  .btn-primary:hover:not(:disabled)   { background: #dbbe6e; }
  .btn-secondary { background: #2a2a2a; color: #ccc; border: 1px solid #333; }
  .btn-secondary:hover:not(:disabled) { background: #333; }

  .status-badge {
    padding: 0.25rem 0.65rem;
    border-radius: 20px;
    font-size: 0.78rem;
    font-weight: 600;
  }
  .status-ok    { background: #0d2b0d; color: #4caf50; border: 1px solid #1a5c1a; }
  .status-error { background: #2b0d0d; color: #f44336; border: 1px solid #5c1a1a; }

  .error-msg { color: #f44336; font-size: 0.82rem; margin-top: 0.5rem; }

  .page-footer { margin-top: 1.5rem; }
</style>
