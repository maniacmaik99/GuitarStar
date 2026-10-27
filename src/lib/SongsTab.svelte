<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { ask, open } from "@tauri-apps/plugin-dialog";
  import {
    api,
    errorText,
    type CdlcOverview,
    type DownloadReport,
    type ImportResult,
    type Overview,
    type Runner,
  } from "$lib/api";
  import { formatNumber, t } from "$lib/i18n.svelte";

  let { run, busy, openBrowser }: { ov: Overview; run: Runner; busy: boolean; openBrowser: () => void } = $props();

  let data = $state<CdlcOverview | null>(null);
  let loadError = $state<string | null>(null);
  let query = $state("");
  let dragging = $state(false);
  /** Newest first; shows what happened to imports and browser downloads. */
  let activity = $state<{ text: string; ok: boolean }[]>([]);

  async function reload() {
    try {
      data = await api.cdlcOverview();
      loadError = null;
    } catch (e) {
      loadError = errorText(e);
    }
  }

  function report(r: ImportResult, source?: string) {
    for (const name of r.imported) activity.unshift({ text: t("songs.installed", { name }), ok: true });
    for (const [name, reason] of r.skipped) activity.unshift({ text: t("songs.skipped", { name, reason }), ok: false });
    if (!r.imported.length && !r.skipped.length && source) activity.unshift({ text: t("songs.nothingFound", { source }), ok: false });
    activity = activity.slice(0, 8);
  }

  async function importPaths(paths: string[]) {
    let result: ImportResult | null = null;
    await run(t("songs.installing"), async () => {
      result = await api.cdlcImport(paths);
    });
    if (result) report(result);
    await reload();
  }

  async function pickFiles() {
    const picked = await open({
      multiple: true,
      title: t("songs.pickTitle"),
      filters: [{ name: t("songs.filterName"), extensions: ["psarc", "zip"] }],
    });
    if (Array.isArray(picked) && picked.length) await importPaths(picked);
  }

  async function toggle(name: string, enabled: boolean) {
    await run(enabled ? t("songs.enabling") : t("songs.disabling"), () => api.cdlcSetEnabled(name, enabled));
    await reload();
  }

  async function remove(name: string, title: string) {
    if (!(await ask(t("songs.deleteConfirm", { title }), { title: t("songs.deleteTitle"), kind: "warning" }))) return;
    await run(t("songs.deleting"), () => api.cdlcDelete(name));
    await reload();
  }

  async function installEnabler() {
    const ok = await ask(
      t("songs.enablerConfirm"),
      { title: t("songs.enablerTitle"), kind: "info" },
    );
    if (ok) await run(t("common.installerLoading"), () => api.installCdlcEnabler(), t("common.installerStarted"));
  }

  let filtered = $derived.by(() => {
    const q = query.trim().toLowerCase();
    const songs = data?.songs ?? [];
    return q ? songs.filter((s) => `${s.artist ?? ""} ${s.title} ${s.file_name}`.toLowerCase().includes(q)) : songs;
  });
  let enabledCount = $derived(data?.songs.filter((s) => s.enabled).length ?? 0);

  const mb = (bytes: number) => formatNumber(bytes / 1024 / 1024, { minimumFractionDigits: 1, maximumFractionDigits: 1, useGrouping: false }) + " MB";

  onMount(() => {
    reload();
    const unlisteners = [
      getCurrentWebview().onDragDropEvent((e) => {
        if (e.payload.type === "over" || e.payload.type === "enter") dragging = true;
        else if (e.payload.type === "leave") dragging = false;
        else if (e.payload.type === "drop") {
          dragging = false;
          if (e.payload.paths.length) importPaths(e.payload.paths);
        }
      }),
      listen<DownloadReport>("cdlc-download", (e) => {
        const r = e.payload;
        if (r.result) report(r.result, r.file_name);
        else activity.unshift({ text: `${r.file_name}: ${r.error}`, ok: false });
        reload();
      }),
    ];
    return () => unlisteners.forEach((p) => p.then((off) => off()));
  });
</script>

{#if loadError}
  <p class="error-text">{loadError}</p>
{:else if data}
  {#if !data.enabler_installed}
    <section class="card attention">
      <h2>{t("songs.enablerMissing")}</h2>
      <p>{t("songs.enablerMissingText")}</p>
      {#if data.cherub_rock_installed === false}
        <p class="muted small">{t("songs.cherubHint")}</p>
      {/if}
      <div class="row">
        <button class="primary" disabled={busy} onclick={installEnabler}>{t("songs.installEnabler")}</button>
        <button disabled={busy} onclick={reload}>{t("common.recheck")}</button>
      </div>
    </section>
  {/if}

  <section class="card">
    <div class="row head">
      <h2>{t("songs.find")}</h2>
      <button class="primary" onclick={openBrowser}>{t("songs.openCustomsforge")}</button>
      <button disabled={busy} onclick={pickFiles}>{t("songs.pickFiles")}</button>
    </div>
    <p class="muted">{t("songs.findText")}</p>
    {#if activity.length}
      <ul class="activity">
        {#each activity as a}
          <li class:bad={!a.ok}>{a.text}</li>
        {/each}
      </ul>
    {/if}
  </section>

  <section class="card">
    <div class="row head">
      <h2>{t("songs.yours")}</h2>
      <span class="badge">{t("songs.enabledCount", { count: enabledCount })}</span>
      {#if data.songs.length - enabledCount}
        <span class="badge">{t("songs.disabledCount", { count: data.songs.length - enabledCount })}</span>
      {/if}
      <input class="search" placeholder={t("songs.search")} bind:value={query} />
    </div>
    {#if data.songs.length === 0}
      <p class="muted">{t("songs.empty")}</p>
    {:else}
      <table>
        <thead>
          <tr>
            <th>{t("songs.colActive")}</th>
            <th>{t("songs.colArtist")}</th>
            <th>{t("songs.colTitle")}</th>
            <th>{t("songs.colVersion")}</th>
            <th>{t("songs.colSize")}</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          {#each filtered as s (s.file_name)}
            <tr class:off={!s.enabled}>
              <td>
                <input
                  type="checkbox"
                  checked={s.enabled}
                  disabled={busy}
                  title={s.enabled ? t("songs.disable") : t("songs.enable")}
                  onchange={(e) => toggle(s.file_name, e.currentTarget.checked)}
                />
              </td>
              <td>{s.artist ?? "–"}</td>
              <td title={s.file_name}>{s.title}</td>
              <td class="muted">{s.version ?? ""}</td>
              <td class="muted">{mb(s.size)}</td>
              <td class="right">
                <button class="link" disabled={busy} onclick={() => remove(s.file_name, s.title)}>{t("songs.delete")}</button>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
      <p class="muted small">{t("songs.changesHint")}</p>
    {/if}
  </section>
{:else}
  <p class="muted">{t("songs.loading")}</p>
{/if}

{#if dragging}
  <div class="drop">{t("songs.drop")}</div>
{/if}

<style>
  .attention {
    border-color: var(--warn);
  }
  .head h2 {
    margin: 0;
    margin-right: auto;
  }
  .search {
    min-width: 200px;
  }
  .small {
    font-size: 0.85rem;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    margin-top: 12px;
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
  tr.off td {
    opacity: 0.55;
  }
  .right {
    text-align: right;
  }
  .activity {
    list-style: none;
    padding: 0;
    margin: 8px 0 0;
    font-size: 0.9rem;
  }
  .activity li {
    color: var(--ok);
    padding: 2px 0;
  }
  .activity li.bad {
    color: var(--warn);
  }
  .error-text {
    color: var(--danger);
  }
  .drop {
    position: fixed;
    inset: 12px;
    border: 3px dashed var(--accent);
    border-radius: 14px;
    background: color-mix(in srgb, var(--bg) 85%, transparent);
    display: grid;
    place-items: center;
    font-size: 1.4rem;
    color: var(--accent);
    pointer-events: none;
  }
</style>
