<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { api, type Overview, type Runner } from "$lib/api";
  import { getLang, LANGUAGES, setLang, t, type Lang } from "$lib/i18n.svelte";
  import { checkForUpdate, installUpdate, updater } from "$lib/updater.svelte";

  let { ov, run, busy }: { ov: Overview; run: Runner; busy: boolean } = $props();

  let fixInstalled = $derived(!!ov.state.installed["dpi_fix"]);

  function setHideAds(enabled: boolean) {
    run(
      t("common.saving"),
      () => api.setHideAds(enabled),
      enabled ? t("settings.hideAdsOn") : t("settings.hideAdsOff"),
    );
  }
</script>

<section class="card">
  <h2>{t("update.title")}</h2>
  <p>{t("update.installedVersion", { version: updater.version ?? "?" })}</p>
  <div class="row">
    {#if updater.update}
      <button class="primary" disabled={updater.progress !== null} onclick={installUpdate}>
        {updater.progress !== null
          ? t("update.downloading", { progress: Math.max(updater.progress, 0) })
          : `${t("update.now")} (${updater.update.version})`}
      </button>
    {:else}
      <button disabled={updater.checking} onclick={() => checkForUpdate()}>
        {updater.checking ? t("update.checking") : t("update.check")}
      </button>
      {#if updater.upToDate && !updater.checking}
        <span class="badge ok">{t("update.upToDate")}</span>
      {/if}
    {/if}
  </div>
  {#if updater.checkError}
    <p class="error-text">{t("update.checkFailed", { error: updater.checkError })}</p>
  {/if}
  {#if updater.installError}
    <p class="error-text">{t("update.failed", { error: updater.installError })}</p>
  {/if}
  <p class="muted small">{t("update.autoHint")}</p>
</section>

<section class="card">
  <h2>{t("settings.language")}</h2>
  <select value={getLang()} onchange={(e) => setLang(e.currentTarget.value as Lang)}>
    {#each LANGUAGES as l (l.code)}
      <option value={l.code}>{l.name}</option>
    {/each}
  </select>
</section>

<section class="card">
  <h2>CustomsForge</h2>
  <div class="info">
    <strong>{t("settings.adsNotOurs")}</strong>
    <p>{t("settings.adsText")}</p>
    <p>{t("settings.adsSupportText")}</p>
    <div class="row">
      <button onclick={() => openUrl("https://ignition4.customsforge.com/support")}>{t("settings.support")}</button>
      <button onclick={() => openUrl("https://www.patreon.com/customsforge")}>Patreon</button>
    </div>
  </div>
  <label class="row toggle">
    <input
      type="checkbox"
      checked={ov.state.hide_ads}
      disabled={busy}
      onchange={(e) => setHideAds(e.currentTarget.checked)}
    />
    {t("settings.hideAds")}
  </label>
  <p class="muted small">{t("settings.hideAdsHint")}</p>
</section>

<section class="card">
  <h2>{t("settings.troubleshooting")}</h2>
  <label class="row toggle">
    <input
      type="checkbox"
      checked={ov.state.dll_log}
      disabled={busy || !fixInstalled}
      onchange={(e) =>
        run(t("common.saving"), () => api.setDllLog(e.currentTarget.checked), t("settings.nextLaunch"))}
    />
    {t("settings.dllLog")}
  </label>
  <p class="muted small">
    {t("settings.dllLogHint")}
    {#if !fixInstalled}{t("settings.dllLogUnavailable")}{/if}
  </p>
</section>

<style>
  .error-text {
    color: var(--danger);
  }
  .info {
    border: 1px solid var(--border);
    border-left: 3px solid var(--accent);
    border-radius: 6px;
    padding: 10px 14px;
    margin: 10px 0 14px;
    background: var(--panel-2);
  }
  .info p {
    margin: 8px 0;
  }
  .toggle {
    margin: 6px 0;
    cursor: pointer;
  }
  .small {
    font-size: 0.85rem;
  }
</style>
