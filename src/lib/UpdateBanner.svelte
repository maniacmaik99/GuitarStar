<script lang="ts">
  import { onMount } from "svelte";
  import { check, type Update } from "@tauri-apps/plugin-updater";
  import { relaunch } from "@tauri-apps/plugin-process";
  import { errorText } from "$lib/api";

  let update = $state<Update | null>(null);
  let progress = $state<number | null>(null);
  let error = $state<string | null>(null);
  let dismissed = $state(false);

  onMount(async () => {
    try {
      update = await check();
    } catch {
      // Offline, or no release with update information yet: nothing to show.
    }
  });

  async function install() {
    if (!update) return;
    error = null;
    let total = 0;
    let done = 0;
    progress = 0;
    try {
      await update.downloadAndInstall((event) => {
        if (event.event === "Started") total = event.data.contentLength ?? 0;
        else if (event.event === "Progress") {
          done += event.data.chunkLength;
          progress = total ? Math.round((done / total) * 100) : null;
        }
      });
      await relaunch();
    } catch (e) {
      error = errorText(e);
      progress = null;
    }
  }
</script>

{#if update && !dismissed}
  <div class="update">
    <div>
      <strong>GuitarStar {update.version} ist da</strong>
      <span class="muted">(installiert: {update.currentVersion})</span>
      {#if error}<div class="error">Update fehlgeschlagen: {error}</div>{/if}
    </div>
    <div class="row">
      {#if progress !== null}
        <span class="muted">Wird geladen … {progress}%</span>
      {:else}
        <button class="primary" onclick={install}>Jetzt aktualisieren</button>
        <button class="link" onclick={() => (dismissed = true)}>später</button>
      {/if}
    </div>
  </div>
{/if}

<style>
  .update {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    padding: 10px 14px;
    margin-bottom: 12px;
    border: 1px solid var(--accent);
    border-radius: 8px;
    background: var(--panel);
  }
  .error {
    color: var(--danger);
    font-size: 0.9rem;
  }
</style>
