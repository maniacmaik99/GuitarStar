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
  import { locale, t } from "$lib/i18n.svelte";

  let { ov, run, busy }: { ov: Overview; run: Runner; busy: boolean } = $props();

  let rsmods = $state<RsModsStatus | null>(null);
  const loadRsmods = async () => (rsmods = await api.rsmodsStatus().catch(() => null));
  onMount(loadRsmods);

  async function installRsmods() {
    const ok = await ask(
      t("components.rsmodsConfirm"),
      { title: t("components.rsmodsConfirmTitle"), kind: "info" },
    );
    if (ok) {
      await run(t("components.rsmodsLoading"), () => api.installRsmods(), t("common.installerStarted"));
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
    run(
      t("components.installing", { name: c.name, version }),
      () => api.installComponent(c.id, version),
      t("components.installedDone", { name: c.name, version }),
    );
  }

  function uninstall(c: ComponentStatus) {
    run(t("components.removing", { name: c.name }), () => api.uninstallComponent(c.id), t("components.removed", { name: c.name }));
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
        <span class="badge ok">{t("components.installedBadge", { version: c.installed.version })}</span>
        {#if latest && latest.version !== c.installed.version}
          <span class="badge warn">{t("components.updateBadge", { version: latest.version })}</span>
        {/if}
      {:else}
        <span class="badge">{t("common.notInstalled")}</span>
      {/if}
    </div>
    <p class="muted">{c.description}</p>
    <p class="muted small">
      {#if c.repo}
        {t("components.source")} <button class="link inline" onclick={() => openUrl(`https://github.com/${c.repo}`)}>github.com/{c.repo}</button>
      {:else}
        {t("components.bundled", { version: c.bundled_version ?? "" })}
      {/if}
    </p>

    {#if c.conflicts.length}
      <div class="hint">{t("components.conflict", { files: c.conflicts.join(", ") })}</div>
    {/if}

    {#if c.installed && !c.files_present}
      <div class="hint">{t("components.filesMissing")}</div>
    {/if}

    <div class="row">
      {#if c.bundled_version && c.installed?.version !== c.bundled_version}
        <button class="primary" disabled={busy} onclick={() => install(c, c.bundled_version!)}>
          {c.installed ? t("components.updateTo", { version: c.bundled_version }) : t("common.install")}
        </button>
      {/if}
      {#if c.repo && !releases[c.id]}
        <button disabled={busy} onclick={() => loadReleases(c)}>{t("components.searchVersions")}</button>
      {/if}
      {#if latest && latest.version !== c.installed?.version}
        <button class="primary" disabled={busy} onclick={() => install(c, latest.version)}>
          {c.installed
            ? t("components.updateTo", { version: latest.version })
            : t("components.installVersion", { version: latest.version })}
        </button>
      {/if}
      {#if c.installed && !c.files_present}
        <button class="primary" disabled={busy} onclick={() => install(c, c.installed!.version)}>{t("components.reinstall")}</button>
      {/if}
      {#if c.installed}
        <button class="danger" disabled={busy} onclick={() => uninstall(c)}>{t("common.remove")}</button>
      {/if}
    </div>
    {#if loadError[c.id]}
      <p class="error-text">{loadError[c.id]}</p>
    {/if}

    {#if c.repo && (releases[c.id] || c.cached.length)}
      {@const list = versions(c)}
      <table>
        <thead>
          <tr><th>{t("components.colVersion")}</th><th>{t("components.colPublished")}</th><th></th><th></th></tr>
        </thead>
        <tbody>
          {#each showAll[c.id] ? list : list.slice(0, 6) as v (v.version)}
            <tr class:current={v.version === c.installed?.version}>
              <td>{v.version}{v.prerelease ? t("components.prerelease") : ""}</td>
              <td class="muted">{v.published ? new Date(v.published).toLocaleDateString(locale()) : "–"}</td>
              <td>{#if isCached(c, v.version)}<span class="badge">{t("components.offline")}</span>{/if}</td>
              <td class="right">
                {#if v.version === c.installed?.version}
                  <span class="muted">{t("components.active")}</span>
                {:else}
                  <button disabled={busy} onclick={() => install(c, v.version)}>
                    {c.installed ? t("components.switchVersion") : t("common.install")}
                  </button>
                {/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
      {#if list.length > 6}
        <button class="link" onclick={() => (showAll[c.id] = !showAll[c.id])}>
          {showAll[c.id] ? t("components.showLess") : t("components.showAll", { count: list.length })}
        </button>
      {/if}
    {/if}
    {#if c.installed}
      <p class="muted small">{t("components.installedAt", { date: formatDate(c.installed.installed_at) })}</p>
    {/if}
  </section>
{/each}

<section class="card">
  <div class="row head">
    <h2>RSMods</h2>
    {#if rsmods?.installed}
      <span class="badge ok">{t("common.installed")}</span>
    {:else}
      <span class="badge">{t("common.notInstalled")}</span>
    {/if}
    {#if rsmods?.cache_modified}
      <span class="badge ok">{t("components.cacheModifiedBadge")}</span>
    {/if}
    <span class="badge">{t("components.optional")}</span>
  </div>
  {#if rsmods?.installed}
    <p class="small">
      {#if rsmods.cache_modified === true}
        {t("components.cacheModified")}
      {:else if rsmods.cache_modified === false}
        {t("components.cacheOriginal")}
      {:else}
        {t("components.cacheUnknown")}
      {/if}
    </p>
  {/if}
  <p class="muted">
    {t("components.rsmodsIntroBefore")} <strong>Fast Load</strong> {t("components.rsmodsIntroAfter")}
  </p>
  <p class="muted small">
    {t("components.source")}
    <button class="link inline" onclick={() => openUrl("https://github.com/Lovrom8/RSMods")}>github.com/Lovrom8/RSMods</button>
    {t("components.rsmodsSourceNote")}
  </p>
  <div class="hint">{t("components.rsmodsHint")}</div>
  <div class="row">
    <button class={rsmods?.installed ? "" : "primary"} disabled={busy} onclick={installRsmods}>
      {rsmods?.installed ? t("components.installLatest") : t("components.installRsmods")}
    </button>
    {#if rsmods?.gui}
      <button class="primary" disabled={busy} onclick={() => run(t("components.opening"), () => api.openRsmodsGui())}>
        {t("components.openRsmods")}
      </button>
    {/if}
    <button disabled={busy} onclick={loadRsmods}>{t("common.recheck")}</button>
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
