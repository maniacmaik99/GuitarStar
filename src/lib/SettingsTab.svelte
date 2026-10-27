<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { api, type Overview, type Runner } from "$lib/api";
  import { getLang, setLang, t, type Lang } from "$lib/i18n.svelte";

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
  <h2>{t("settings.language")}</h2>
  <select value={getLang()} onchange={(e) => setLang(e.currentTarget.value as Lang)}>
    <option value="de">Deutsch</option>
    <option value="en">English</option>
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
