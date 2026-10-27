<script lang="ts">
  import { onMount } from "svelte";
  import { t } from "$lib/i18n.svelte";
  import { checkForUpdate, installUpdate, loadVersion, updater } from "$lib/updater.svelte";

  onMount(() => {
    loadVersion();
    checkForUpdate(true);
  });
</script>

{#if updater.update && !updater.dismissed}
  <div class="update">
    <div>
      <strong>{t("update.available", { version: updater.update.version })}</strong>
      <span class="muted">{t("update.current", { version: updater.update.currentVersion })}</span>
      {#if updater.installError}<div class="error">{t("update.failed", { error: updater.installError })}</div>{/if}
    </div>
    <div class="row">
      {#if updater.progress !== null}
        <span class="muted">{t("update.downloading", { progress: Math.max(updater.progress, 0) })}</span>
      {:else}
        <button class="primary" onclick={installUpdate}>{t("update.now")}</button>
        <button class="link" onclick={() => (updater.dismissed = true)}>{t("update.later")}</button>
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
