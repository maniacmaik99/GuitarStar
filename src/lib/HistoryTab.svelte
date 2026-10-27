<script lang="ts">
  import { ask } from "@tauri-apps/plugin-dialog";
  import { api, formatDate, type Overview, type Runner, type SnapshotSummary } from "$lib/api";
  import { t } from "$lib/i18n.svelte";

  let { ov, run, busy }: { ov: Overview; run: Runner; busy: boolean } = $props();

  async function undoTo(s: SnapshotSummary, index: number) {
    const count = index + 1;
    const ok = await ask(
      count === 1
        ? t("history.undoOne", { label: s.label })
        : t("history.undoMany", { label: s.label, count: count - 1 }),
      { title: t("history.undoTitle"), kind: "warning" },
    );
    if (ok) run(t("history.undoing"), () => api.restoreSnapshot(s.id), t("history.undone"));
  }

  async function restoreOriginal() {
    const ok = await ask(
      t("history.restoreConfirm"),
      { title: t("history.originalTitle"), kind: "warning" },
    );
    if (ok) run(t("history.restoring"), () => api.restoreOriginal(), t("history.restored"));
  }
</script>

<section class="card">
  <h2>{t("history.title")}</h2>
  <p class="muted">{t("history.intro")}</p>
  {#if ov.snapshots.length === 0}
    <p>{t("history.empty")}</p>
  {:else}
    <ul>
      {#each ov.snapshots as s, i (s.id)}
        <li>
          <div>
            <strong>{s.label}</strong>
            <div class="muted small">
              {formatDate(s.created)}{s.file_count ? t("history.files", { count: s.file_count }) : t("history.windowsSetting")}
            </div>
          </div>
          <button disabled={busy} onclick={() => undoTo(s, i)}>{t("history.undo")}</button>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<section class="card">
  <h2>{t("history.originalTitle")}</h2>
  <p class="muted">{t("history.originalText")}</p>
  <button class="danger" disabled={busy || ov.snapshots.length === 0} onclick={restoreOriginal}>
    {t("history.restoreButton")}
  </button>
</section>

<style>
  ul {
    list-style: none;
    padding: 0;
    margin: 0;
  }
  li {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px 0;
    border-top: 1px solid var(--border);
  }
  .small {
    font-size: 0.85rem;
  }
</style>
