<script lang="ts">
  import { ask } from "@tauri-apps/plugin-dialog";
  import { api, formatDate, type Overview, type Runner, type SnapshotSummary } from "$lib/api";

  let { ov, run, busy }: { ov: Overview; run: Runner; busy: boolean } = $props();

  async function undoTo(s: SnapshotSummary, index: number) {
    const count = index + 1;
    const ok = await ask(
      count === 1
        ? `„${s.label}“ rückgängig machen?`
        : `„${s.label}“ und die ${count - 1} späteren Änderungen rückgängig machen?`,
      { title: "Rückgängig machen", kind: "warning" },
    );
    if (ok) run("Wird rückgängig gemacht …", () => api.restoreSnapshot(s.id), "Rückgängig gemacht.");
  }

  async function restoreOriginal() {
    const ok = await ask(
      "Alle Änderungen von GuitarStar am Spielordner zurücknehmen? RS_ASIO und alle Einstellungen werden auf den Stand vor der ersten Änderung gesetzt.",
      { title: "Originalzustand", kind: "warning" },
    );
    if (ok) run("Originalzustand wird wiederhergestellt …", () => api.restoreOriginal(), "Originalzustand wiederhergestellt.");
  }
</script>

<section class="card">
  <h2>Verlauf der Änderungen</h2>
  <p class="muted">
    Vor jeder Änderung am Spielordner sichert GuitarStar die betroffenen Dateien. Du kannst jede Änderung samt allen
    späteren zurücknehmen.
  </p>
  {#if ov.snapshots.length === 0}
    <p>Noch keine Änderungen.</p>
  {:else}
    <ul>
      {#each ov.snapshots as s, i (s.id)}
        <li>
          <div>
            <strong>{s.label}</strong>
            <div class="muted small">
              {formatDate(s.created)}{s.file_count ? ` · ${s.file_count} Datei(en)` : " · Windows-Einstellung"}
            </div>
          </div>
          <button disabled={busy} onclick={() => undoTo(s, i)}>Rückgängig</button>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<section class="card">
  <h2>Originalzustand</h2>
  <p class="muted">Nimmt alle Änderungen von GuitarStar zurück, z. B. wenn das Spiel nicht mehr startet.</p>
  <button class="danger" disabled={busy || ov.snapshots.length === 0} onclick={restoreOriginal}>
    Originalzustand wiederherstellen
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
