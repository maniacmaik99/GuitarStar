<script lang="ts">
  import { onMount } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { api, errorText, type Overview } from "$lib/api";
  import GameTab from "$lib/GameTab.svelte";
  import AudioTab from "$lib/AudioTab.svelte";
  import ComponentsTab from "$lib/ComponentsTab.svelte";
  import SongsTab from "$lib/SongsTab.svelte";
  import HistoryTab from "$lib/HistoryTab.svelte";

  type Tab = "game" | "audio" | "songs" | "components" | "history";
  const tabs: { id: Tab; label: string }[] = [
    { id: "game", label: "Spiel" },
    { id: "audio", label: "Audio" },
    { id: "songs", label: "Songs" },
    { id: "components", label: "Komponenten" },
    { id: "history", label: "Verlauf" },
  ];

  let ov = $state<Overview | null>(null);
  let tab = $state<Tab>("game");
  let busy = $state<string | null>(null);
  let error = $state<string | null>(null);
  let notice = $state<string | null>(null);
  let detected = $state<string[]>([]);

  async function refresh() {
    ov = await api.overview();
  }

  /** Runs an action with a busy indicator; returns false when it failed. */
  async function run(label: string, action: () => Promise<unknown>, done?: string): Promise<boolean> {
    busy = label;
    error = null;
    notice = null;
    try {
      await action();
      await refresh();
      if (done) notice = done;
      return true;
    } catch (e) {
      error = errorText(e);
      return false;
    } finally {
      busy = null;
    }
  }

  async function chooseFolder() {
    const picked = await open({ directory: true, title: "Rocksmith-2014-Ordner auswählen" });
    if (typeof picked === "string") await run("Ordner wird geprüft …", () => api.setGameDir(picked));
  }

  onMount(async () => {
    try {
      await refresh();
      if (!ov?.state.game_dir) detected = await api.detectGames();
    } catch (e) {
      error = errorText(e);
    }
  });
</script>

<div class="app">
  <header>
    <div class="brand">GuitarStar</div>
    {#if ov?.game}
      <div class="gamepath" title={ov.game.path}>
        <span class="dot ok"></span>{ov.game.path}
        <button class="link" onclick={chooseFolder}>ändern</button>
      </div>
    {/if}
  </header>

  {#if error}
    <div class="banner error" role="alert">
      {error}<button class="link" onclick={() => (error = null)}>schließen</button>
    </div>
  {/if}
  {#if notice}
    <div class="banner success">
      {notice}<button class="link" onclick={() => (notice = null)}>schließen</button>
    </div>
  {/if}
  {#if busy}
    <div class="banner info"><span class="spinner"></span>{busy}</div>
  {/if}

  {#if !ov}
    <p class="muted center">Lade …</p>
  {:else if !ov.game}
    <section class="card setup">
      <h1>Willkommen bei GuitarStar</h1>
      <p>Wähle zuerst deinen Rocksmith-2014-Ordner aus. GuitarStar ändert dort nur Dateien, die es vorher gesichert hat.</p>
      {#if detected.length}
        <p class="muted">Gefundene Installationen:</p>
        {#each detected as path}
          <button class="choice" disabled={!!busy} onclick={() => run("Ordner wird geprüft …", () => api.setGameDir(path))}>
            {path}
          </button>
        {/each}
      {:else}
        <p class="muted">Über Steam wurde keine Installation gefunden.</p>
      {/if}
      <button class="primary" disabled={!!busy} onclick={chooseFolder}>Ordner auswählen …</button>
    </section>
  {:else}
    {#if ov.game.problems.length}
      <div class="banner warn">{ov.game.problems.join(" ")}</div>
    {/if}
    <nav>
      {#each tabs as t}
        <button class:active={tab === t.id} onclick={() => (tab = t.id)}>{t.label}</button>
      {/each}
    </nav>
    <main>
      {#if tab === "game"}
        <GameTab {ov} {run} busy={!!busy} />
      {:else if tab === "audio"}
        <AudioTab {ov} {run} busy={!!busy} gotoComponents={() => (tab = "components")} />
      {:else if tab === "songs"}
        <SongsTab {ov} {run} busy={!!busy} />
      {:else if tab === "components"}
        <ComponentsTab {ov} {run} busy={!!busy} />
      {:else}
        <HistoryTab {ov} {run} busy={!!busy} />
      {/if}
    </main>
  {/if}
</div>

<style>
  :global(:root) {
    --bg: #14161a;
    --panel: #1d2026;
    --panel-2: #252931;
    --border: #323742;
    --text: #e7e9ee;
    --muted: #9aa1ad;
    --accent: #f2a03d;
    --accent-text: #1a1206;
    --ok: #4cc38a;
    --warn: #e5b53c;
    --danger: #ef6461;
    color-scheme: dark;
    font-family: "Segoe UI", system-ui, sans-serif;
    font-size: 15px;
  }
  :global(body) {
    margin: 0;
    background: var(--bg);
    color: var(--text);
  }
  :global(button) {
    font: inherit;
    color: inherit;
    background: var(--panel-2);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 7px 14px;
    cursor: pointer;
  }
  :global(button:hover:not(:disabled)) {
    border-color: var(--muted);
  }
  :global(button:disabled) {
    opacity: 0.5;
    cursor: default;
  }
  :global(button.primary) {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--accent-text);
    font-weight: 600;
  }
  :global(button.danger) {
    border-color: var(--danger);
    color: var(--danger);
  }
  :global(button.link) {
    background: none;
    border: none;
    padding: 0 0 0 10px;
    color: var(--accent);
    text-decoration: underline;
  }
  :global(input, select) {
    font: inherit;
    color: var(--text);
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 6px 8px;
  }
  :global(.card) {
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 18px 20px;
    margin-bottom: 16px;
  }
  :global(.card h2) {
    margin: 0 0 6px;
    font-size: 1.1rem;
  }
  :global(.muted) {
    color: var(--muted);
  }
  :global(.row) {
    display: flex;
    gap: 10px;
    align-items: center;
    flex-wrap: wrap;
  }
  :global(.field) {
    display: grid;
    grid-template-columns: 220px 1fr;
    gap: 12px;
    align-items: center;
    margin: 10px 0;
  }
  :global(.badge) {
    display: inline-block;
    font-size: 0.8rem;
    padding: 2px 8px;
    border-radius: 99px;
    background: var(--panel-2);
    border: 1px solid var(--border);
  }
  :global(.badge.ok) {
    color: var(--ok);
    border-color: var(--ok);
  }
  :global(.badge.warn) {
    color: var(--warn);
    border-color: var(--warn);
  }
  .app {
    max-width: 980px;
    margin: 0 auto;
    padding: 0 20px 30px;
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px 0;
    gap: 16px;
  }
  .brand {
    font-size: 1.4rem;
    font-weight: 700;
    color: var(--accent);
  }
  .gamepath {
    color: var(--muted);
    font-size: 0.9rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .dot {
    display: inline-block;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    margin-right: 8px;
  }
  .dot.ok {
    background: var(--ok);
  }
  nav {
    display: flex;
    gap: 4px;
    border-bottom: 1px solid var(--border);
    margin-bottom: 18px;
  }
  nav button {
    border: none;
    border-bottom: 2px solid transparent;
    border-radius: 0;
    background: none;
    padding: 10px 16px;
    color: var(--muted);
  }
  nav button.active {
    color: var(--text);
    border-bottom-color: var(--accent);
  }
  .banner {
    padding: 10px 14px;
    border-radius: 8px;
    margin-bottom: 12px;
    border: 1px solid var(--border);
    background: var(--panel);
  }
  .banner.error {
    border-color: var(--danger);
  }
  .banner.success {
    border-color: var(--ok);
  }
  .banner.warn {
    border-color: var(--warn);
  }
  .setup {
    max-width: 620px;
    margin: 40px auto;
  }
  .setup h1 {
    margin-top: 0;
  }
  .choice {
    display: block;
    width: 100%;
    text-align: left;
    margin-bottom: 8px;
  }
  .center {
    text-align: center;
  }
  .spinner {
    display: inline-block;
    width: 12px;
    height: 12px;
    margin-right: 10px;
    border: 2px solid var(--muted);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
