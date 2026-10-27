<script lang="ts">
  import { onMount } from "svelte";
  import { ask } from "@tauri-apps/plugin-dialog";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import {
    api,
    errorText,
    formatDate,
    type ComponentStatus,
    type Overview,
    type ReleaseInfo,
    type RsModsStatus,
    type Runner,
  } from "$lib/api";

  let { ov, run, busy }: { ov: Overview; run: Runner; busy: boolean } = $props();

  let rsmods = $state<RsModsStatus | null>(null);
  const loadRsmods = async () => (rsmods = await api.rsmodsStatus().catch(() => null));
  onMount(loadRsmods);

  async function installRsmods() {
    const ok = await ask(
      "GuitarStar lädt den offiziellen RSMods-Installer von GitHub (Lovrom8/RSMods) und startet ihn. Vorher werden xinput1_3.dll, RSMods.ini und cache.psarc gesichert. Weiter?",
      { title: "RSMods installieren", kind: "info" },
    );
    if (ok) {
      await run("RSMods-Installer wird geladen …", () => api.installRsmods(), "Installer gestartet. Danach hier „Neu prüfen“ klicken.");
    }
  }

  let releases = $state<Record<string, ReleaseInfo[]>>({});
  let loadError = $state<Record<string, string>>({});
  let showAll = $state<Record<string, boolean>>({});

  async function loadReleases(c: ComponentStatus) {
    delete loadError[c.id];
    try {
      releases[c.id] = await api.componentReleases(c.id);
    } catch (e) {
      loadError[c.id] = errorText(e);
    }
  }

  function isCached(c: ComponentStatus, version: string) {
    return c.cached.some((s) => s.version === version);
  }

  function install(c: ComponentStatus, version: string) {
    run(`${c.name} ${version} wird installiert …`, () => api.installComponent(c.id, version), `${c.name} ${version} ist installiert.`);
  }

  function uninstall(c: ComponentStatus) {
    run(`${c.name} wird entfernt …`, () => api.uninstallComponent(c.id), `${c.name} wurde entfernt.`);
  }

  /** Online releases plus cached versions that are no longer listed online. */
  function versions(c: ComponentStatus): { version: string; published: string | null; prerelease: boolean }[] {
    const online = releases[c.id] ?? [];
    const offline = c.cached
      .filter((s) => !online.some((r) => r.version === s.version))
      .map((s) => ({ version: s.version, published: null, prerelease: false }));
    return [...online, ...offline];
  }
</script>

{#each ov.components as c (c.id)}
  {@const latest = releases[c.id]?.find((r) => !r.prerelease)}
  <section class="card">
    <div class="row head">
      <h2>{c.name}</h2>
      {#if c.installed}
        <span class="badge ok">installiert: {c.installed.version}</span>
        {#if latest && latest.version !== c.installed.version}
          <span class="badge warn">Update: {latest.version}</span>
        {/if}
      {:else}
        <span class="badge">nicht installiert</span>
      {/if}
    </div>
    <p class="muted">{c.description}</p>
    <p class="muted small">
      {#if c.repo}
        Quelle: <button class="link inline" onclick={() => openUrl(`https://github.com/${c.repo}`)}>github.com/{c.repo}</button>
      {:else}
        In GuitarStar enthalten (Version {c.bundled_version})
      {/if}
    </p>

    {#if c.conflicts.length}
      <div class="hint">
        Im Spielordner liegt bereits eine andere {c.conflicts.join(", ")} (vermutlich von einem anderen Mod). Beim
        Installieren wird sie gesichert und ersetzt; über den Verlauf lässt sie sich zurückholen.
      </div>
    {/if}

    {#if c.installed && !c.files_present}
      <div class="hint">
        Dateien fehlen im Spielordner, vermutlich hat Steam die Spieldateien überprüft. Installiere die Version erneut.
      </div>
    {/if}

    <div class="row">
      {#if c.bundled_version && c.installed?.version !== c.bundled_version}
        <button class="primary" disabled={busy} onclick={() => install(c, c.bundled_version!)}>
          {c.installed ? `Auf ${c.bundled_version} aktualisieren` : "Installieren"}
        </button>
      {/if}
      {#if c.repo && !releases[c.id]}
        <button disabled={busy} onclick={() => loadReleases(c)}>Nach Versionen suchen</button>
      {/if}
      {#if latest && latest.version !== c.installed?.version}
        <button class="primary" disabled={busy} onclick={() => install(c, latest.version)}>
          {c.installed ? `Auf ${latest.version} aktualisieren` : `${latest.version} installieren`}
        </button>
      {/if}
      {#if c.installed && !c.files_present}
        <button class="primary" disabled={busy} onclick={() => install(c, c.installed!.version)}>Erneut installieren</button>
      {/if}
      {#if c.installed}
        <button class="danger" disabled={busy} onclick={() => uninstall(c)}>Entfernen</button>
      {/if}
    </div>
    {#if loadError[c.id]}
      <p class="error-text">{loadError[c.id]}</p>
    {/if}

    {#if c.repo && (releases[c.id] || c.cached.length)}
      {@const list = versions(c)}
      <table>
        <thead>
          <tr><th>Version</th><th>Veröffentlicht</th><th></th><th></th></tr>
        </thead>
        <tbody>
          {#each showAll[c.id] ? list : list.slice(0, 6) as v (v.version)}
            <tr class:current={v.version === c.installed?.version}>
              <td>{v.version}{v.prerelease ? " (Vorabversion)" : ""}</td>
              <td class="muted">{v.published ? new Date(v.published).toLocaleDateString("de-DE") : "–"}</td>
              <td>{#if isCached(c, v.version)}<span class="badge">offline verfügbar</span>{/if}</td>
              <td class="right">
                {#if v.version === c.installed?.version}
                  <span class="muted">aktiv</span>
                {:else}
                  <button disabled={busy} onclick={() => install(c, v.version)}>
                    {c.installed ? "Zu dieser Version wechseln" : "Installieren"}
                  </button>
                {/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
      {#if list.length > 6}
        <button class="link" onclick={() => (showAll[c.id] = !showAll[c.id])}>
          {showAll[c.id] ? "weniger anzeigen" : `alle ${list.length} Versionen anzeigen`}
        </button>
      {/if}
    {/if}
    {#if c.installed}
      <p class="muted small">Installiert am {formatDate(c.installed.installed_at)}</p>
    {/if}
  </section>
{/each}

<section class="card">
  <div class="row head">
    <h2>RSMods</h2>
    {#if rsmods?.installed}
      <span class="badge ok">installiert</span>
    {:else}
      <span class="badge">nicht installiert</span>
    {/if}
    {#if rsmods?.cache_modified}
      <span class="badge ok">Spieldatei-Mods aktiv (z. B. Fast Load)</span>
    {/if}
    <span class="badge">optional</span>
  </div>
  {#if rsmods?.installed}
    <p class="small">
      {#if rsmods.cache_modified === true}
        cache.psarc ist gegenüber dem gesicherten Original verändert – Fast Load oder ein anderer „Set And Forget“-Mod ist
        angewendet.
      {:else if rsmods.cache_modified === false}
        cache.psarc ist noch im Originalzustand: Fast Load ist nicht angewendet. In RSMods unter „Set And Forget Mods“
        einschalten.
      {:else}
        Ob Fast Load aktiv ist, lässt sich nicht prüfen: Es gibt keine von GuitarStar gesicherte Original-cache.psarc.
      {/if}
    </p>
  {/if}
  <p class="muted">
    Große Mod-Sammlung der Community: <strong>Fast Load</strong> (überspringt die Intro-Logos beim Start), eigene
    Songlisten, Songs ohne Neustart neu laden, Riff Repeater über 100 %, Lautstärke per Hotkey und mehr. Eingestellt
    wird alles in der RSMods-Oberfläche.
  </p>
  <p class="muted small">
    Quelle: <button class="link inline" onclick={() => openUrl("https://github.com/Lovrom8/RSMods")}>github.com/Lovrom8/RSMods</button>
    – wird mit dem offiziellen Installer eingerichtet, nicht von GuitarStar mitgeliefert.
  </p>
  <div class="hint">
    Fast Load und einige andere Funktionen verändern die Spieldatei cache.psarc. GuitarStar sichert sie vor der
    Installation; über „Verlauf“ lässt sich der Stand davor zurückholen. Fast Load gilt auf HDDs als instabil.
  </div>
  <div class="row">
    <button class={rsmods?.installed ? "" : "primary"} disabled={busy} onclick={installRsmods}>
      {rsmods?.installed ? "Neueste Version installieren …" : "RSMods installieren …"}
    </button>
    {#if rsmods?.gui}
      <button class="primary" disabled={busy} onclick={() => run("RSMods wird geöffnet …", () => api.openRsmodsGui())}>
        RSMods öffnen (Fast Load einschalten)
      </button>
    {/if}
    <button disabled={busy} onclick={loadRsmods}>Neu prüfen</button>
  </div>
</section>

<style>
  .head h2 {
    margin: 0;
  }
  .small {
    font-size: 0.85rem;
  }
  .link.inline {
    padding: 0;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    margin-top: 14px;
  }
  th {
    text-align: left;
    font-weight: 500;
    color: var(--muted);
    font-size: 0.85rem;
    padding: 4px 6px;
  }
  td {
    padding: 6px;
    border-top: 1px solid var(--border);
  }
  tr.current td {
    color: var(--accent);
  }
  .right {
    text-align: right;
  }
  .hint {
    padding: 8px 12px;
    border-radius: 6px;
    border: 1px solid var(--warn);
    margin: 8px 0;
  }
  .error-text {
    color: var(--danger);
  }
</style>
