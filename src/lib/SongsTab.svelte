<script lang="ts">
  import { onMount } from "svelte";
  import { SvelteSet } from "svelte/reactivity";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { ask, open } from "@tauri-apps/plugin-dialog";
  import {
    api,
    errorText,
    type BulkAction,
    type BulkResult,
    type CdlcEntry,
    type CdlcOverview,
    type DownloadReport,
    type ImportResult,
    type Overview,
    type Runner,
    type SongSource,
    type CheckEvent,
    type CheckStart,
    type UpdateStatus,
  } from "$lib/api";
  import { formatNumber, locale, t, type MessageKey } from "$lib/i18n.svelte";

  let {
    run,
    busy,
    openBrowser,
  }: { ov: Overview; run: Runner; busy: boolean; openBrowser: (url?: string) => void } = $props();

  type SortKey = "artist" | "title" | "modified" | "size";
  type Sort = { key: SortKey; dir: 1 | -1 };
  const SORT_STORAGE_KEY = "guitarstar.songs.sort";
  const CF_SONG_URL = "https://ignition4.customsforge.com/cdlc/";

  let data = $state<CdlcOverview | null>(null);
  let loadError = $state<string | null>(null);
  let query = $state("");
  let sources = $state<Record<string, SongSource>>({});
  let sort = $state<Sort>(loadSort());
  let duplicatesOnly = $state(false);
  const selected = new SvelteSet<string>();
  /** Row of the last click on a selection box, for Shift-click ranges. */
  let anchor: string | null = null;
  let dragging = $state(false);
  /** Newest first; shows what happened to imports and browser downloads. */
  let activity = $state<{ text: string; ok: boolean }[]>([]);

  let updateStatus = $state<UpdateStatus | null>(null);
  /** Text about the song update check: running, result or why it did not run. */
  let checkMessage = $state<{ text: string; ok: boolean } | null>(null);

  async function loadUpdateStatus() {
    updateStatus = await api.cdlcUpdateStatus().catch(() => null);
  }

  const reasonText: Record<NonNullable<CheckStart["reason"]>, MessageKey> = {
    none: "songs.updatesNoIds",
    running: "songs.updatesRunning",
    auto_done: "songs.updatesAllChecked",
    manual_limit: "songs.updatesManualLimit",
    all_checked: "songs.updatesAllChecked",
  };

  async function checkUpdates() {
    try {
      const start = await api.cdlcCheckUpdates(true);
      if (start.started) checkMessage = { text: t("songs.updatesChecking", { done: 0, total: start.count }), ok: true };
      else if (start.reason) checkMessage = { text: t(reasonText[start.reason]), ok: start.reason !== "manual_limit" };
    } catch (e) {
      checkMessage = { text: errorText(e), ok: false };
    }
    await loadUpdateStatus();
  }

  function onCheckEvent(e: CheckEvent) {
    if (e.state === "progress") {
      checkMessage = { text: t("songs.updatesChecking", { done: e.done, total: e.total }), ok: true };
      return;
    }
    loadUpdateStatus().then(() => {
      const found = Object.keys(updateStatus?.updates ?? {}).length;
      if (e.state === "done") {
        checkMessage = found ? { text: t("songs.updatesFound", { count: found }), ok: true } : { text: t("songs.updatesNone"), ok: true };
      } else {
        const key = e.state === "login" ? "songs.updatesLogin" : e.state === "limited" ? "songs.updatesLimited" : "songs.updatesError";
        checkMessage = { text: t(key), ok: false };
      }
    });
  }

  async function reload() {
    loadUpdateStatus();
    try {
      data = await api.cdlcOverview();
      loadError = null;
      const present = new Set(data.songs.map((s) => s.file_name));
      for (const name of [...selected]) if (!present.has(name)) selected.delete(name);
    } catch (e) {
      loadError = errorText(e);
    }
    try {
      sources = await api.cdlcSources();
    } catch {
      // Only the links to CustomsForge are missing then.
    }
  }

  function loadSort(): Sort {
    try {
      const saved = JSON.parse(localStorage.getItem(SORT_STORAGE_KEY) ?? "null");
      if (["artist", "title", "modified", "size"].includes(saved?.key) && (saved.dir === 1 || saved.dir === -1)) return saved;
    } catch {
      // Unreadable or no storage: default order.
    }
    return { key: "artist", dir: 1 };
  }

  function sortBy(key: SortKey) {
    // Newest and largest first are the useful defaults for dates and sizes.
    sort = sort.key === key ? { key, dir: sort.dir === 1 ? -1 : 1 } : { key, dir: key === "modified" || key === "size" ? -1 : 1 };
    try {
      localStorage.setItem(SORT_STORAGE_KEY, JSON.stringify(sort));
    } catch {
      // Not persisted; the order still applies for this session.
    }
  }

  function select(name: string, on: boolean, shift: boolean) {
    const names = filtered.map((s) => s.file_name);
    const from = anchor ? names.indexOf(anchor) : -1;
    const to = names.indexOf(name);
    if (shift && from >= 0 && to >= 0) {
      for (const n of names.slice(Math.min(from, to), Math.max(from, to) + 1)) {
        if (on) selected.add(n);
        else selected.delete(n);
      }
    } else if (on) selected.add(name);
    else selected.delete(name);
    anchor = name;
  }

  function selectAll(on: boolean) {
    for (const s of filtered) {
      if (on) selected.add(s.file_name);
      else selected.delete(s.file_name);
    }
  }

  async function bulk(action: BulkAction) {
    const names = [...selected];
    if (!names.length) return;
    if (
      action === "delete" &&
      !(await ask(t("songs.bulkDeleteConfirm", { count: names.length }), { title: t("songs.bulkDeleteTitle"), kind: "warning" }))
    )
      return;
    let result: BulkResult | null = null;
    const label = action === "delete" ? t("songs.bulkDeleting") : t("songs.bulkWorking");
    await run(label, async () => {
      result = await api.cdlcBulk(names, action);
    });
    if (result) {
      const r: BulkResult = result;
      for (const name of r.done) selected.delete(name);
      for (const [name, reason] of r.failed) activity.unshift({ text: t("songs.bulkFailed", { name, reason }), ok: false });
      activity = activity.slice(0, 8);
    }
    await reload();
  }

  function openOnCustomsforge(source: SongSource) {
    openBrowser(CF_SONG_URL + encodeURIComponent(source.cf_id));
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

  /** Number of files per song key; more than one means several versions. */
  let keyCounts = $derived.by(() => {
    const counts = new Map<string, number>();
    for (const s of data?.songs ?? []) counts.set(s.song_key, (counts.get(s.song_key) ?? 0) + 1);
    return counts;
  });
  const isDuplicate = (s: CdlcEntry) => (keyCounts.get(s.song_key) ?? 0) > 1;
  let duplicateCount = $derived(data?.songs.filter(isDuplicate).length ?? 0);

  const text = (v: string | null) => (v ?? "").toLocaleLowerCase();
  function compare(a: CdlcEntry, b: CdlcEntry): number {
    switch (sort.key) {
      case "artist":
        return text(a.artist).localeCompare(text(b.artist)) || text(a.title).localeCompare(text(b.title));
      case "title":
        return text(a.title).localeCompare(text(b.title)) || text(a.artist).localeCompare(text(b.artist));
      case "modified":
        return a.modified - b.modified;
      case "size":
        return a.size - b.size;
    }
  }

  let filtered = $derived.by(() => {
    const q = query.trim().toLowerCase();
    let songs = data?.songs ?? [];
    if (q) songs = songs.filter((s) => `${s.artist ?? ""} ${s.title} ${s.file_name}`.toLowerCase().includes(q));
    if (duplicatesOnly) songs = songs.filter(isDuplicate);
    // Ties keep versions of one song together, oldest version first.
    return [...songs].sort((a, b) => sort.dir * compare(a, b) || a.file_name.localeCompare(b.file_name));
  });
  let allShownSelected = $derived(filtered.length > 0 && filtered.every((s) => selected.has(s.file_name)));
  let enabledCount = $derived(data?.songs.filter((s) => s.enabled).length ?? 0);

  const day = (unixSeconds: number) => new Date(unixSeconds * 1000).toLocaleDateString(locale(), { dateStyle: "medium" });
  const arrow = (key: SortKey) => (sort.key === key ? (sort.dir === 1 ? " ▲" : " ▼") : "");

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
      listen<CheckEvent>("cdlc-update-check", (e) => onCheckEvent(e.payload)),
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
      <button class="primary" onclick={() => openBrowser()}>{t("songs.openCustomsforge")}</button>
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
      {#if duplicateCount}
        <span class="badge warn dup" title={t("songs.duplicateHint")}>{t("songs.duplicateCount", { count: duplicateCount })}</span>
      {/if}
      <input class="search" placeholder={t("songs.search")} bind:value={query} />
    </div>
    <div class="updates">
      <div class="row">
        <button
          disabled={busy || !updateStatus || updateStatus.running || updateStatus.manual_left === 0}
          title={t("songs.updatesHint")}
          onclick={checkUpdates}
        >
          {t("songs.updatesCheck", { left: updateStatus?.manual_left ?? 3 })}
        </button>
        {#if checkMessage}
          <span class="small" class:bad-text={!checkMessage.ok}>{checkMessage.text}</span>
        {:else if updateStatus && Object.keys(updateStatus.updates).length}
          <span class="badge ok">{t("songs.updatesFound", { count: Object.keys(updateStatus.updates).length })}</span>
        {/if}
        {#if updateStatus?.last_check}
          <span class="muted small">{t("songs.updatesLast", { date: day(updateStatus.last_check) })}</span>
        {/if}
      </div>
      <p class="muted small">{t("songs.updatesHint")}</p>
    </div>
    {#if data.songs.length === 0}
      <p class="muted">{t("songs.empty")}</p>
    {:else}
      <div class="row tools">
        <span class="muted small">
          {t("songs.shownCount", { shown: filtered.length, total: data.songs.length })}
        </span>
        {#if duplicateCount}
          <label class="small" title={t("songs.duplicateHint")}>
            <input type="checkbox" bind:checked={duplicatesOnly} />
            {t("songs.duplicatesOnly")}
          </label>
        {/if}
        {#if selected.size}
          <span class="selection">
            <strong class="small">{t("songs.selectedCount", { count: selected.size })}</strong>
            <button disabled={busy} onclick={() => bulk("enable")}>{t("songs.bulkEnable")}</button>
            <button disabled={busy} onclick={() => bulk("disable")}>{t("songs.bulkDisable")}</button>
            <button class="danger" disabled={busy} onclick={() => bulk("delete")}>{t("songs.bulkDelete")}</button>
            <button class="link" onclick={() => selected.clear()}>{t("songs.clearSelection")}</button>
          </span>
        {/if}
      </div>
      <table>
        <thead>
          <tr>
            <th class="narrow">
              <input
                type="checkbox"
                title={t("songs.selectAll")}
                checked={allShownSelected}
                disabled={!filtered.length}
                onchange={(e) => selectAll(e.currentTarget.checked)}
              />
            </th>
            <th>{t("songs.colActive")}</th>
            <th><button class="sort" title={t("songs.sortTitle")} onclick={() => sortBy("artist")}>{t("songs.colArtist")}{arrow("artist")}</button></th>
            <th><button class="sort" title={t("songs.sortTitle")} onclick={() => sortBy("title")}>{t("songs.colTitle")}{arrow("title")}</button></th>
            <th>{t("songs.colVersion")}</th>
            <th>
              <button class="sort" title={t("songs.dateTitle")} onclick={() => sortBy("modified")}>{t("songs.colDate")}{arrow("modified")}</button>
            </th>
            <th><button class="sort" title={t("songs.sortTitle")} onclick={() => sortBy("size")}>{t("songs.colSize")}{arrow("size")}</button></th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          {#each filtered as s (s.file_name)}
            {@const source = sources[s.file_name]}
            <tr class:off={!s.enabled} class:picked={selected.has(s.file_name)}>
              <td class="narrow">
                <input
                  type="checkbox"
                  title={t("songs.colSelect")}
                  checked={selected.has(s.file_name)}
                  onclick={(e) => select(s.file_name, e.currentTarget.checked, e.shiftKey)}
                />
              </td>
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
              <td title={s.file_name}>
                {s.title}
                {#if isDuplicate(s)}
                  <span class="badge warn dup" title={t("songs.duplicateHint")}>{t("songs.duplicate")}</span>
                {/if}
                {#if source}
                  <button
                    class="link small"
                    title={t("songs.onCustomsforgeTitle", { date: day(source.downloaded_at) })}
                    onclick={() => openOnCustomsforge(source)}>{t("songs.onCustomsforge")}</button
                  >
                {/if}
              </td>
              <td class="muted">
                {s.version ?? ""}
                {#if updateStatus?.updates[s.file_name]}
                  {@const update = updateStatus.updates[s.file_name]}
                  <button
                    class="badge ok update"
                    title={t("songs.updateBadgeTitle")}
                    onclick={() => source && openOnCustomsforge(source)}
                  >
                    {update.version ? t("songs.updateBadge", { version: update.version }) : t("songs.updateBadgeNoVersion")}
                  </button>
                {/if}
              </td>
              <td class="muted nowrap">{s.modified ? day(s.modified) : ""}</td>
              <td class="muted nowrap">{mb(s.size)}</td>
              <td class="right">
                <button class="link" disabled={busy} onclick={() => remove(s.file_name, s.title)}>{t("songs.delete")}</button>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
      {#if !filtered.length}
        <p class="muted">{t("songs.noMatches")}</p>
      {/if}
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
  tr.picked td {
    background: color-mix(in srgb, var(--accent) 10%, transparent);
  }
  th.narrow,
  td.narrow {
    width: 1%;
  }
  .nowrap {
    white-space: nowrap;
  }
  button.sort {
    background: none;
    border: none;
    padding: 0;
    font: inherit;
    color: inherit;
    cursor: pointer;
  }
  button.sort:hover {
    color: var(--text);
  }
  .tools {
    margin-top: 10px;
    gap: 14px;
  }
  .selection {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    margin-left: auto;
  }
  .updates {
    margin: 10px 0 4px;
  }
  .updates p {
    margin: 6px 0 0;
  }
  .bad-text {
    color: var(--warn);
  }
  .badge.update {
    padding: 1px 8px;
    margin-left: 6px;
    cursor: pointer;
  }
  .badge.dup {
    cursor: help;
    margin-left: 6px;
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
