<script lang="ts">
  import { onMount } from "svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import type { Overview } from "$lib/api";
  import { t } from "$lib/i18n.svelte";
  import { linuxSetupStatus, type LinuxSetupStatus } from "$lib/linux";

  // `ov` is only used to reload when the game folder or installed components change.
  let { ov }: { ov: Overview } = $props();

  const RS_ASIO_LINUX_DOCS = "https://github.com/mdias/rs_asio/tree/master/docs/linux";

  let status = $state<LinuxSetupStatus | null>(null);
  let loaded = $state(false);
  let error = $state<string | null>(null);
  let copied = $state(false);

  let launchOk = $derived(!!status && status.launch_options_checked && status.missing.length === 0);

  async function load() {
    try {
      status = await linuxSetupStatus();
      error = null;
    } catch (e) {
      error = String(e);
    }
    loaded = true;
  }

  onMount(load);

  // Installing or removing a mod changes the required overrides.
  let installedKey = $derived(Object.keys(ov.state.installed).sort().join(","));
  let lastKey = "";
  $effect(() => {
    const key = `${ov.state.game_dir}|${installedKey}`;
    if (lastKey && key !== lastKey) load();
    lastKey = key;
  });

  async function copy() {
    if (!status?.launch_options) return;
    try {
      await navigator.clipboard.writeText(status.launch_options);
      copied = true;
      setTimeout(() => (copied = false), 2000);
    } catch {
      error = t("linux.copyFailed");
    }
  }
</script>

{#if loaded && (status || error)}
  <section class="card">
    <div class="row head">
      <h2>Linux / Steam Deck (Proton)</h2>
      <button class="link" onclick={load}>{t("linux.recheck")}</button>
    </div>
    {#if error}
      <div class="hint">{error}</div>
    {/if}
    {#if status}
      <div class="item">
        <div class="row">
          <strong>{t("linux.prefix")}</strong>
          {#if status.proton_prefix}
            <span class="badge ok">{t("linux.found")}</span>
          {:else}
            <span class="badge warn">{t("linux.missing")}</span>
          {/if}
        </div>
        {#if status.proton_prefix}
          <div class="muted path">{status.proton_prefix}</div>
        {:else}
          <p class="muted">{t("linux.prefixMissing")}</p>
        {/if}
      </div>

      <div class="item">
        <div class="row">
          <strong>wineasio (32-Bit)</strong>
          {#if status.wineasio_registered}
            <span class="badge ok">{t("linux.registered")}</span>
          {:else}
            <span class="badge warn">{t("linux.notFound")}</span>
          {/if}
          {#if status.wineasio_dll_in_prefix}
            <span class="badge">{t("linux.dllInPrefix")}</span>
          {/if}
        </div>
        {#if !status.wineasio_registered}
          <p class="muted">{t("linux.wineasioHelp")}</p>
          <button class="link inline" onclick={() => openUrl(RS_ASIO_LINUX_DOCS)}>{t("linux.docs")}</button>
        {/if}
        {#if status.asio_drivers.length}
          <div class="muted">
            {t("linux.driversInPrefix", {
              list: status.asio_drivers.map((d) => d.name + (d.has_32bit ? "" : t("linux.only64"))).join(", "),
            })}
          </div>
        {/if}
      </div>

      <div class="item">
        <div class="row">
          <strong>{t("linux.launchOptions")}</strong>
          {#if !status.launch_options}
            <span class="badge">{t("linux.notNeeded")}</span>
          {:else if launchOk}
            <span class="badge ok">{t("linux.set")}</span>
          {:else if status.launch_options_checked}
            <span class="badge warn">{t("linux.launchMissing")}</span>
          {:else}
            <span class="badge">{t("linux.notCheckable")}</span>
          {/if}
        </div>
        {#if status.launch_options}
          <p class="muted">{t("linux.launchHelp")}</p>
          <div class="row">
            <code class="line">{status.launch_options}</code>
            <button onclick={copy}>{copied ? t("linux.copied") : t("linux.copy")}</button>
          </div>
          {#if status.launch_options_checked && !launchOk}
            <p class="muted">
              {#if status.current_launch_options}
                {t("linux.current", { value: status.current_launch_options, missing: status.missing.join(", ") })}
              {:else}
                {t("linux.noneSet")}
              {/if}
              {t("linux.steamSavesOnExit")}
            </p>
          {/if}
        {:else}
          <p class="muted">{t("linux.noModNeeds")}</p>
        {/if}
      </div>
    {/if}
  </section>
{/if}

<style>
  .head {
    justify-content: space-between;
  }
  .item {
    margin: 12px 0;
  }
  .item p {
    margin: 6px 0;
  }
  .path {
    font-size: 0.85rem;
    word-break: break-all;
  }
  .line {
    flex: 1;
    min-width: 0;
    padding: 6px 8px;
    border-radius: 6px;
    border: 1px solid var(--border);
    background: var(--bg);
    word-break: break-all;
  }
  .link.inline {
    padding: 0;
  }
  .hint {
    padding: 8px 12px;
    border-radius: 6px;
    border: 1px solid var(--warn);
    margin: 8px 0;
  }
</style>
