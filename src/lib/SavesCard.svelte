<script lang="ts">
  import { onMount } from "svelte";
  import { ask } from "@tauri-apps/plugin-dialog";
  import { errorText, formatDate, type Runner } from "$lib/api";
  import { formatNumber, storedLabel, t } from "$lib/i18n.svelte";
  import { saves, type SaveBackup, type SaveFolder } from "$lib/saves";

  let { run, busy }: { run: Runner; busy: boolean } = $props();

  let folders = $state<SaveFolder[] | null>(null);
  let backups = $state<SaveBackup[]>([]);
  let loadError = $state<string | null>(null);

  const multipleAccounts = $derived(
    new Set([...(folders ?? []).map((f) => f.account), ...backups.map((b) => b.account)]).size > 1,
  );

  const label = (b: SaveBackup) => storedLabel(b.label_key, b.label_args, b.label);

  function size(bytes: number): string {
    if (bytes < 1024 * 1024) return formatNumber(Math.max(1, Math.round(bytes / 1024))) + " KB";
    return formatNumber(bytes / 1024 / 1024, { maximumFractionDigits: 1 }) + " MB";
  }

  async function load() {
    try {
      [folders, backups] = await Promise.all([saves.folders(), saves.backups()]);
      loadError = null;
    } catch (e) {
      loadError = errorText(e);
    }
  }

  async function act(label: string, action: () => Promise<unknown>, done?: string) {
    await run(label, action, done);
    await load();
  }

  async function restore(b: SaveBackup) {
    const ok = await ask(t("saves.restoreConfirm", { date: formatDate(b.created), label: label(b) }), {
      title: t("saves.restoreTitle"),
      kind: "warning",
    });
    if (ok) await act(t("saves.restoring"), () => saves.restore(b.id), t("saves.restored"));
  }

  async function remove(b: SaveBackup) {
    const ok = await ask(t("saves.deleteConfirm", { date: formatDate(b.created) }), {
      title: t("saves.deleteTitle"),
      kind: "warning",
    });
    if (ok) await act(t("saves.deleting"), () => saves.remove(b.id));
  }

  onMount(load);
</script>

<section class="card">
  <h2>{t("saves.title")}</h2>
  <p class="muted">{t("saves.intro")}</p>

  {#if loadError}
    <p class="hint">{loadError}</p>
  {:else if folders === null}
    <p class="muted">{t("saves.loading")}</p>
  {:else if folders.length === 0}
    <p class="hint">{t("saves.noFolder")}</p>
  {:else}
    {#each folders as f (f.path)}
      <div class="row folder">
        <span class="badge ok">{t("saves.found")}</span>
        <span>{t("saves.account", { account: f.account })}</span>
        <span class="muted small">
          {t("saves.files", { count: f.file_count })}{f.modified
            ? t("saves.lastChange", { date: formatDate(f.modified) })
            : ""}
        </span>
        <button class="link" disabled={busy} onclick={() => run(t("saves.opening"), () => saves.openFolder(f.account))}>
          {t("saves.openFolder")}
        </button>
      </div>
    {/each}
  {/if}

  <div class="row actions">
    <button
      class="primary"
      disabled={busy || !folders?.length}
      onclick={() => act(t("saves.backingUp"), () => saves.backup(), t("saves.backedUp"))}
    >
      {t("saves.backupNow")}
    </button>
  </div>
  <p class="muted small">{t("saves.autoHint")}</p>

  <h3>{t("saves.backups")}</h3>
  {#if backups.length === 0}
    <p class="muted">{t("saves.noBackups")}</p>
  {:else}
    <ul>
      {#each backups as b (b.id)}
        <li>
          <div>
            <strong>{formatDate(b.created)}</strong>
            <span class="muted">· {label(b)}</span>
            <div class="muted small">
              {t("saves.files", { count: b.files.length })} · {size(b.size)}{multipleAccounts
                ? " · " + t("saves.account", { account: b.account })
                : ""}{b.automatic ? " · " + t("saves.automatic") : ""}
            </div>
          </div>
          <div class="row buttons">
            <button disabled={busy} onclick={() => restore(b)}>{t("saves.restore")}</button>
            <button class="danger" disabled={busy} onclick={() => remove(b)}>{t("saves.delete")}</button>
          </div>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  h3 {
    margin: 18px 0 6px;
    font-size: 1rem;
  }
  .folder {
    margin: 8px 0;
  }
  .actions {
    margin-top: 12px;
  }
  .buttons {
    flex-wrap: nowrap;
  }
  ul {
    list-style: none;
    padding: 0;
    margin: 0;
  }
  li {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    padding: 10px 0;
    border-top: 1px solid var(--border);
  }
  .hint {
    padding: 8px 12px;
    border-radius: 6px;
    border: 1px solid var(--warn);
    margin: 8px 0;
  }
  .small {
    font-size: 0.85rem;
  }
</style>
