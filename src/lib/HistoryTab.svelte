<script lang="ts">
  import { ask } from "@tauri-apps/plugin-dialog";
  import { api, formatDate, type Overview, type Runner, type SnapshotSummary } from "$lib/api";
  import { storedLabel, t } from "$lib/i18n.svelte";
  import SavesCard from "$lib/SavesCard.svelte";

  let { ov, run, busy }: { ov: Overview; run: Runner; busy: boolean } = $props();

  /** The entry's label in the current language; `component` holds a component id. */
  function label(s: SnapshotSummary): string {
    const args = { ...(s.label_args ?? {}) };
    if (args.component) args.component = ov.components.find((c) => c.id === args.component)?.name ?? args.component;
    return storedLabel(s.label_key, args, s.label);
  }

  async function undoTo(s: SnapshotSummary, index: number) {
    const count = index + 1;
    const ok = await ask(
      count === 1
        ? t("history.undoOne", { label: label(s) })
        : t("history.undoMany", { label: label(s), count: count - 1 }),
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
            <strong>{label(s)}</strong>
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

<SavesCard {run} {busy} />

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
