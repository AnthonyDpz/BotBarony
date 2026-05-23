<script>
  import { onMount, createEventDispatcher } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';

  const dispatch = createEventDispatcher();

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
  let testingProvider = false;
  let testResult = null;     // null | { ok: boolean, message: string }
  let errorMsg = '';
  let savedMsg = '';

  // Barony config
  let baronyPath = '';
  let characterClass = 'wanderer';
  let characterRace  = 'Human';

  // Launch state
  let launching     = false;
  let launchStatus  = null; // null | { ok: boolean, message: string }

  const CLASSES = [
    'barbarian','warrior','healer','rogue','wanderer','cleric','merchant',
    'wizard','arcanist','joker','sexton','ninja','monk','conjurer','accursed',
    'mesmer','brewer','mechanist','punisher','shaman','hunter',
    'bard','sapper','scion','hermit','paladin',
  ];
  const RACES = [
    'Human','Skeleton','Vampire','Succubus','Goatman','Automaton',
    'Incubus','Goblin','Insectoid','Gnome','Gremlin','Dryad','Myconid','Salamander',
  ];

  onMount(async () => {
    try {
      const saved = await invoke('load_provider_config');
      if (saved) {
        const p = PROVIDERS.find(p => p.id === saved.provider);
        if (p) {
          selectedProvider = p;
          baseUrl = saved.base_url ?? p.defaultUrl;
          apiKey = saved.api_key ?? '';
          selectedModel = saved.model ?? '';
          baronyPath      = saved.barony_path      ?? '';
        characterClass  = saved.character_class  ?? 'wanderer';
        characterRace   = saved.character_race   ?? 'Human';
        }
      }
    } catch (e) {
      // No saved config — first launch
    }
  });

  function onProviderChange(provider) {
    selectedProvider = provider;
    baseUrl = provider.defaultUrl;
    apiKey = '';
    models = [];
    selectedModel = '';
    healthStatus = null;
    testResult = null;
    errorMsg = '';
    savedMsg = '';
  }

  async function checkHealth() {
    loadingHealth = true;
    healthStatus = null;
    testResult = null;
    errorMsg = '';
    try {
      const ok = await invoke('check_provider_health', {
        provider: selectedProvider.id,
        baseUrl: baseUrl || null,
        apiKey: apiKey || null,
      });
      healthStatus = ok ? 'ok' : 'error';
      if (ok) await loadModels();
    } catch (e) {
      healthStatus = 'error';
      errorMsg = friendlyError(selectedProvider.id, String(e));
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
      if (models.length > 0 && !selectedModel) selectedModel = models[0];
    } catch (e) {
      errorMsg = String(e);
    } finally {
      loadingModels = false;
    }
  }

  async function testProvider() {
    if (!selectedModel) { errorMsg = 'Sélectionnez un modèle avant de tester.'; return; }
    testingProvider = true;
    testResult = null;
    errorMsg = '';
    try {
      const response = await invoke('test_provider', {
        provider: selectedProvider.id,
        baseUrl: baseUrl || null,
        apiKey: apiKey || null,
        model: selectedModel,
      });
      testResult = { ok: true, message: response.trim() };
    } catch (e) {
      testResult = { ok: false, message: String(e) };
    } finally {
      testingProvider = false;
    }
  }

  async function saveConfig() {
    if (!selectedModel) { errorMsg = 'Choisissez un modèle avant de sauvegarder.'; return; }
    errorMsg = '';
    savedMsg = '';
    try {
      await invoke('save_provider_config', {
        config: {
          provider: selectedProvider.id,
          base_url: baseUrl || null,
          api_key: apiKey || null,
          model: selectedModel,
          barony_path:     baronyPath     || null,
          character_class: characterClass,
          character_race:  characterRace,
        },
      });
      savedMsg = 'Configuration sauvegardée.';
      setTimeout(() => dispatch('saved'), 800);
    } catch (e) {
      errorMsg = String(e);
    }
  }

  function friendlyError(providerId, raw) {
    if (raw.includes('Connection refused') || raw.includes('connect')) {
      if (providerId === 'ollama')   return "Ollama n'est pas lancé. Démarrez-le avec `ollama serve`.";
      if (providerId === 'lmstudio') return "LM Studio n'est pas lancé. Ouvrez l'application et activez le serveur local.";
    }
    if (raw.includes('401') || raw.includes('Unauthorized')) return 'Clé API invalide ou expirée.';
    if (raw.includes('403')) return 'Accès refusé — vérifiez les permissions de votre clé API.';
    return raw;
  }

  async function launchGame() {
    launching    = true;
    launchStatus = null;
    try {
      const saved = await invoke('load_provider_config');
      await invoke('launch_game', {
        config: {
          provider:        (saved?.provider)        ?? selectedProvider.id,
          base_url:        (saved?.base_url)        ?? (baseUrl || null),
          api_key:         (saved?.api_key)         ?? (apiKey  || null),
          model:           (saved?.model)           ?? selectedModel,
          barony_path:     baronyPath             || null,
          character_class: characterClass,
          character_race:  characterRace,
        },
      });
      launchStatus = { ok: true, message: 'Partie lancée.' };
    } catch (e) {
      launchStatus = { ok: false, message: String(e) };
    } finally {
      launching = false;
    }
  }

  $: canSave = selectedModel.length > 0 && (healthStatus === 'ok');
</script>

<div class="page setup-page">
  <header class="page-header">
    <h1>Configuration</h1>
    <p class="subtitle">Choisissez votre provider IA et configurez Barony.</p>
  </header>

  <!-- ── Provider selection ─────────────────────────────────────────────────── -->
  <section class="card">
    <h2 class="card-title">Provider IA</h2>

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
        <span class="field-label">URL de base</span>
        <input class="field-input" type="text" bind:value={baseUrl}
          placeholder={selectedProvider.defaultUrl} />
      </label>

      {#if selectedProvider.requiresKey}
        <label class="form-field">
          <span class="field-label">Clé API</span>
          <input class="field-input" type="password" bind:value={apiKey}
            placeholder="sk-…" />
        </label>
      {/if}
    </div>

    <div class="actions-row">
      <button class="btn btn-secondary" on:click={checkHealth}
        disabled={loadingHealth || loadingModels}>
        {loadingHealth ? 'Vérification…' : 'Vérifier la connexion'}
      </button>

      {#if healthStatus === 'ok'}
        <span class="status-badge status-ok">● Connecté</span>
      {:else if healthStatus === 'error'}
        <span class="status-badge status-error">● Injoignable</span>
      {:else}
        <span class="status-badge status-neutral">○ Non vérifié</span>
      {/if}
    </div>

    {#if errorMsg}
      <p class="error-msg">{errorMsg}</p>
    {/if}

    {#if models.length > 0}
      <div class="model-row">
        <label class="form-field" style="flex:1">
          <span class="field-label">Modèle</span>
          <select class="field-input field-select" bind:value={selectedModel}>
            {#each models as model}
              <option value={model}>{model}</option>
            {/each}
          </select>
        </label>
        <button class="btn btn-secondary btn-test" on:click={testProvider}
          disabled={testingProvider || !selectedModel}>
          {testingProvider ? '…' : 'Tester'}
        </button>
      </div>

      {#if testResult}
        <p class="test-result" class:test-ok={testResult.ok} class:test-err={!testResult.ok}>
          {testResult.ok ? '✓' : '✗'} {testResult.message}
        </p>
      {/if}
    {:else if healthStatus === 'ok' && !loadingModels}
      <p class="info-msg">Aucun modèle trouvé sur ce provider.</p>
    {/if}
  </section>

  <!-- ── Barony configuration ────────────────────────────────────────────────── -->
  <section class="card">
    <h2 class="card-title">Paramètres Barony</h2>
    <div class="form-grid">
      <label class="form-field">
        <span class="field-label">Chemin vers l'exécutable</span>
        <input class="field-input" type="text" bind:value={baronyPath}
          placeholder="Auto-détecté via Steam" />
      </label>
    </div>
  </section>

  <!-- ── Bot configuration ──────────────────────────────────────────────────── -->
  <section class="card">
    <h2 class="card-title">Paramètres du bot</h2>
    <div class="form-grid form-grid-2col">
      <label class="form-field">
        <span class="field-label">Classe</span>
        <select class="field-input field-select" bind:value={characterClass}>
          {#each CLASSES as c}
            <option value={c}>{c.charAt(0).toUpperCase() + c.slice(1)}</option>
          {/each}
        </select>
      </label>
      <label class="form-field">
        <span class="field-label">Race</span>
        <select class="field-input field-select" bind:value={characterRace}>
          {#each RACES as r}
            <option value={r}>{r}</option>
          {/each}
        </select>
      </label>
    </div>

    <div class="launch-row">
      <button class="btn btn-launch" on:click={launchGame} disabled={launching}>
        {#if launching}
          <span class="spinner"></span> Lancement en cours…
        {:else}
          ▶ Lancer une partie
        {/if}
      </button>

      {#if launchStatus}
        <p class="launch-result" class:launch-ok={launchStatus.ok} class:launch-err={!launchStatus.ok}>
          {launchStatus.ok ? '✓' : '✗'} {launchStatus.message}
        </p>
      {/if}
    </div>
  </section>

  <!-- ── Save ───────────────────────────────────────────────────────────────── -->
  <div class="page-footer">
    {#if savedMsg}
      <span class="saved-msg">{savedMsg}</span>
    {/if}
    <button class="btn btn-primary" on:click={saveConfig} disabled={!canSave}>
      Sauvegarder et continuer
    </button>
  </div>
</div>

<style>
  .page { padding: 2rem; max-width: 820px; }
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

  .model-row { display: flex; align-items: flex-end; gap: 0.75rem; margin-top: 1rem; }
  .btn-test  { white-space: nowrap; padding: 0.45rem 0.85rem; align-self: flex-end; margin-bottom: 0; }

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
  .status-ok      { background: #0d2b0d; color: #4caf50; border: 1px solid #1a5c1a; }
  .status-error   { background: #2b0d0d; color: #f44336; border: 1px solid #5c1a1a; }
  .status-neutral { background: #1e1e1e; color: #666;    border: 1px solid #2a2a2a; }

  .error-msg { color: #f44336; font-size: 0.82rem; margin-top: 0.5rem; }
  .info-msg  { color: #888;    font-size: 0.82rem; margin-top: 0.75rem; }

  .test-result { font-size: 0.82rem; margin-top: 0.5rem; font-family: monospace; }
  .test-ok  { color: #4caf50; }
  .test-err { color: #f44336; }

  .page-footer { margin-top: 1.5rem; display: flex; align-items: center; gap: 1rem; }
  .saved-msg { color: #4caf50; font-size: 0.85rem; }

  .form-grid-2col { flex-direction: row; gap: 1rem; }
  .form-grid-2col .form-field { flex: 1; }

  .launch-row { margin-top: 1.25rem; display: flex; flex-direction: column; gap: 0.5rem; }
  .btn-launch {
    background: #1a3a1a;
    color: #4caf50;
    border: 1px solid #2a5c2a;
    padding: 0.55rem 1.4rem;
    border-radius: 5px;
    font-size: 0.9rem;
    font-weight: 600;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    transition: background 0.15s, border-color 0.15s;
    align-self: flex-start;
  }
  .btn-launch:hover:not(:disabled) { background: #243d24; border-color: #3a7a3a; }
  .btn-launch:disabled { opacity: 0.5; cursor: not-allowed; }

  .spinner {
    display: inline-block;
    width: 12px; height: 12px;
    border: 2px solid #4caf5055;
    border-top-color: #4caf50;
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }
  @keyframes spin { to { transform: rotate(360deg); } }

  .launch-result { font-size: 0.83rem; margin: 0; }
  .launch-ok  { color: #4caf50; }
  .launch-err { color: #f44336; }
</style>
