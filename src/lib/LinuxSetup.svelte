<script lang="ts">
  import { onMount } from "svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import type { Overview } from "$lib/api";
  import { linuxSetupStatus, type LinuxSetupStatus } from "$lib/linux";

  // `ov` is only used to reload when the game folder or installed components change.
  let { ov }: { ov: Overview } = $props();

  const RS_ASIO_LINUX_DOCS = "https://github.com/mdias/rs_asio/tree/master/docs/linux";

  let status = $state<LinuxSetupStatus | null>(null);
  let loaded = $state(false);
  let error = $state<string | null>(null);
  let copied = $state(false);

  let launchOk = $derived(!!status && status.launch_options_checked && status.missing.length === 0);

  async function load() {
    try {
      status = await linuxSetupStatus();
      error = null;
    } catch (e) {
      error = String(e);
    }
    loaded = true;
  }

  onMount(load);

  // Installing or removing a mod changes the required overrides.
  let installedKey = $derived(Object.keys(ov.state.installed).sort().join(","));
  let lastKey = "";
  $effect(() => {
    const key = `${ov.state.game_dir}|${installedKey}`;
    if (lastKey && key !== lastKey) load();
    lastKey = key;
  });

  async function copy() {
    if (!status?.launch_options) return;
    try {
      await navigator.clipboard.writeText(status.launch_options);
      copied = true;
      setTimeout(() => (copied = false), 2000);
    } catch {
      error = "Kopieren nicht möglich – bitte die Zeile von Hand markieren und kopieren.";
    }
  }
</script>

{#if loaded && (status || error)}
  <section class="card">
    <div class="row head">
      <h2>Linux / Steam Deck (Proton)</h2>
      <button class="link" onclick={load}>neu prüfen</button>
    </div>
    {#if error}
      <div class="hint">{error}</div>
    {/if}
    {#if status}
      <div class="item">
        <div class="row">
          <strong>Proton-Prefix</strong>
          {#if status.proton_prefix}
            <span class="badge ok">gefunden</span>
          {:else}
            <span class="badge warn">fehlt</span>
          {/if}
        </div>
        {#if status.proton_prefix}
          <div class="muted path">{status.proton_prefix}</div>
        {:else}
          <p class="muted">
            Rocksmith einmal über Steam mit Proton starten, damit der Prefix (<code>compatdata/221680</code>) angelegt
            wird.
          </p>
        {/if}
      </div>

      <div class="item">
        <div class="row">
          <strong>wineasio (32-Bit)</strong>
          {#if status.wineasio_registered}
            <span class="badge ok">registriert</span>
          {:else}
            <span class="badge warn">nicht gefunden</span>
          {/if}
          {#if status.wineasio_dll_in_prefix}
            <span class="badge">DLL im Prefix</span>
          {/if}
        </div>
        {#if !status.wineasio_registered}
          <p class="muted">
            RS_ASIO spricht unter Linux über wineasio mit JACK/PipeWire. wineasio muss als 32-Bit-Treiber im
            Proton-Prefix von Rocksmith registriert sein, z. B. mit <code>protontricks 221680</code> bzw. nach der
            <button class="link inline" onclick={() => openUrl(RS_ASIO_LINUX_DOCS)}>RS_ASIO-Anleitung für Linux</button>.
            Danach diese Karte neu prüfen.
          </p>
        {/if}
        {#if status.asio_drivers.length}
          <div class="muted">
            ASIO-Treiber im Prefix: {status.asio_drivers
              .map((d) => d.name + (d.has_32bit ? "" : " (nur 64-Bit)"))
              .join(", ")}
          </div>
        {/if}
      </div>

      <div class="item">
        <div class="row">
          <strong>Steam-Startoptionen</strong>
          {#if !status.launch_options}
            <span class="badge">nicht nötig</span>
          {:else if launchOk}
            <span class="badge ok">gesetzt</span>
          {:else if status.launch_options_checked}
            <span class="badge warn">fehlen</span>
          {:else}
            <span class="badge">nicht prüfbar</span>
          {/if}
        </div>
        {#if status.launch_options}
          <p class="muted">
            Wine lädt die Mod-DLLs im Spielordner nur mit diesen Startoptionen. In Steam: Rocksmith 2014 →
            Eigenschaften → Allgemein → Startoptionen, Zeile einfügen.
          </p>
          <div class="row">
            <code class="line">{status.launch_options}</code>
            <button onclick={copy}>{copied ? "Kopiert" : "Kopieren"}</button>
          </div>
          {#if status.launch_options_checked && !launchOk}
            <p class="muted">
              {#if status.current_launch_options}
                Aktuell eingetragen: <code>{status.current_launch_options}</code> – es fehlt: {status.missing.join(", ")}.
              {:else}
                Für Rocksmith sind noch keine Startoptionen eingetragen.
              {/if}
              Steam speichert Änderungen erst beim Beenden; danach neu prüfen.
            </p>
          {/if}
        {:else}
          <p class="muted">Im Spielordner ist noch keine Mod installiert, die Startoptionen braucht.</p>
        {/if}
      </div>
    {/if}
  </section>
{/if}

<style>
  .head {
    justify-content: space-between;
  }
  .item {
    margin: 12px 0;
  }
  .item p {
    margin: 6px 0;
  }
  .path {
    font-size: 0.85rem;
    word-break: break-all;
  }
  .line {
    flex: 1;
    min-width: 0;
    padding: 6px 8px;
    border-radius: 6px;
    border: 1px solid var(--border);
    background: var(--bg);
    word-break: break-all;
  }
  .link.inline {
    padding: 0;
  }
  .hint {
    padding: 8px 12px;
    border-radius: 6px;
    border: 1px solid var(--warn);
    margin: 8px 0;
  }
</style>
