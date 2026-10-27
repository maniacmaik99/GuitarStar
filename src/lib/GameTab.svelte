<script lang="ts">
  import { onMount } from "svelte";
  import { api, type GameSettings, type MonitorSize, type Overview, type Runner } from "$lib/api";
  import { t } from "$lib/i18n.svelte";

  let { ov, run, busy }: { ov: Overview; run: Runner; busy: boolean } = $props();

  const COMMON = [
    [1280, 720],
    [1600, 900],
    [1920, 1080],
    [2560, 1080],
    [2560, 1440],
    [3440, 1440],
    [3840, 2160],
  ];

  let monitors = $state<MonitorSize[]>([]);
  let monitorsLoaded = $state(false);
  let form = $state<GameSettings | null>(null);
  /** Resolution the player wants to see on screen, in physical pixels. */
  let target = $state<[number, number]>([0, 0]);
  /** Write a smaller resolution that Windows' scaling enlarges to exactly `target`. */
  let adapt = $state(false);

  // Rocksmith ignores the HIGHDPIAWARE compatibility flag (measured: window stays
  // DPI-unaware), so instead the resolution is divided by the scale factor.
  const logical = (w: number, h: number, scale: number): [number, number] => [
    Math.round(w / scale),
    Math.round(h / scale),
  ];

  onMount(async () => {
    monitors = await api.monitorSizes().catch(() => []);
    monitorsLoaded = true;
  });

  let dpiFix = $derived(ov.components.find((c) => c.id === "dpi_fix"));
  /** With the fix the game is DPI aware and gets the real resolution, in every display mode. */
  let fixInstalled = $derived(!!dpiFix?.installed);
  let anyScaled = $derived(monitors.some((m) => m.scale_factor > 1));
  let fixOutdated = $derived(!!dpiFix?.installed && dpiFix.installed.version !== dpiFix.bundled_version);

  function toggleKeepVisible(enabled: boolean) {
    run(
      t("common.saving"),
      () => api.setKeepVisible(enabled),
      enabled ? t("game.keepVisibleOn") : t("game.keepVisibleOff"),
    );
  }

  $effect(() => {
    if (!monitorsLoaded) return;
    // Read only from `ov.settings`, never from `form`, so edits don't re-trigger this.
    const s = ov.settings;
    form = s ? { ...s } : null;
    if (!s?.screen_width || !s.screen_height) return;
    const w = s.screen_width;
    const h = s.screen_height;
    const adaptedFor = monitors.find((m) => {
      const [lw, lh] = logical(m.width, m.height, m.scale_factor);
      return m.scale_factor > 1 && lw === w && lh === h;
    });
    if (adaptedFor) {
      target = [adaptedFor.width, adaptedFor.height];
      adapt = !fixInstalled;
    } else {
      target = [w, h];
      adapt = !fixInstalled && scaleFor(w, h) > 1;
    }
  });

  /** Scale of the monitor with this physical size, else of the first (usually primary) monitor. */
  function scaleFor(w: number, h: number): number {
    const m = monitors.find((m) => m.width === w && m.height === h) ?? monitors[0];
    return m?.scale_factor ?? 1;
  }

  let scale = $derived(scaleFor(target[0], target[1]));
  let written = $derived<[number, number]>(
    adapt && !fixInstalled && scale > 1 ? logical(target[0], target[1], scale) : target,
  );

  let resolutions = $derived.by(() => {
    const all = [...monitors.map((m) => [m.width, m.height]), ...COMMON];
    if (target[0] && target[1]) all.unshift([...target]);
    const seen = new Set<string>();
    return all
      .filter(([w, h]) => {
        const k = `${w}x${h}`;
        return seen.has(k) ? false : (seen.add(k), true);
      })
      .sort((a, b) => a[0] * a[1] - b[0] * b[1]);
  });

  function nativeOf(w: number, h: number) {
    return monitors.some((m) => m.width === w && m.height === h);
  }

  const getResolution = () => `${target[0]}x${target[1]}`;
  function setResolution(v: string) {
    const [w, h] = v.split("x").map(Number);
    target = [w, h];
  }

  function disableDpiFlag() {
    run(t("game.dpiRemoving"), () => api.setDpiOverride(false), t("game.dpiRemoved"));
  }

  function save() {
    if (!form) return;
    const settings = { ...form, screen_width: written[0], screen_height: written[1] };
    run(t("game.savingSettings"), () => api.saveGameSettings(settings), t("game.settingsSaved"));
  }

  /** Installs the fix and, if the ini still holds a scaled-down resolution, writes the real one. */
  function installFix() {
    if (!dpiFix?.bundled_version) return;
    const version = dpiFix.bundled_version;
    const settings = form ? { ...form, screen_width: target[0], screen_height: target[1] } : null;
    run(
      t("game.installingFix"),
      async () => {
        await api.installComponent("dpi_fix", version);
        if (settings && (settings.screen_width !== form?.screen_width || settings.screen_height !== form?.screen_height)) {
          await api.saveGameSettings(settings);
        }
      },
      t("game.fixInstalledDone"),
    );
  }
</script>

{#if ov.dpi_override}
  <section class="card attention">
    <h2>{t("game.dpiTitle")}</h2>
    <p>{t("game.dpiText")}</p>
    <button disabled={busy} onclick={disableDpiFlag}>{t("common.remove")}</button>
  </section>
{/if}

{#if anyScaled && dpiFix}
  <section class="card" class:attention={!fixInstalled}>
    <h2>{t("game.scalingTitle")}</h2>
    {#if fixInstalled}
      <p><span class="badge ok">{t("game.fixActive")}</span></p>
      <p class="muted">{t("game.fixActiveText")}</p>
      {#if fixOutdated}
        <button class="primary" disabled={busy} onclick={installFix}>{t("game.updateFix", { version: dpiFix.bundled_version ?? "" })}</button>
      {/if}
    {:else}
      <p>{t("game.scaledText")}</p>
      {#if dpiFix.conflicts.length}
        <p class="muted small">{t("game.fixConflict", { files: dpiFix.conflicts.join(", ") })}</p>
      {/if}
      <button class="primary" disabled={busy} onclick={installFix}>{t("game.installFix")}</button>
    {/if}
  </section>
{/if}

{#if !form}
  <section class="card">
    <h2>{t("game.iniMissingTitle")}</h2>
    <p>{t("game.iniMissingText")}</p>
    <button disabled={busy} onclick={() => run(t("game.checking"), async () => {})}>{t("common.reload")}</button>
  </section>
{:else}
  <section class="card">
    <h2>{t("game.display")}</h2>
    <div class="field">
      <label for="res">{t("game.resolution")}</label>
      <select id="res" bind:value={getResolution, setResolution}>
        {#each resolutions as [w, h]}
          <option value={`${w}x${h}`}>{w} × {h}{nativeOf(w, h) ? t("game.monitorSuffix") : ""}</option>
        {/each}
      </select>
    </div>
    <div class="field">
      <label for="fs">{t("game.displayMode")}</label>
      <select id="fs" bind:value={form.fullscreen}>
        <option value={0}>{t("game.windowed")}</option>
        <option value={1}>{t("game.fullscreenWindow")}</option>
        <option value={2}>{t("game.exclusiveFullscreen")}</option>
      </select>
    </div>
    {#if fixInstalled && !fixOutdated}
      <div class="field">
        <span>{t("game.focusLoss")}</span>
        <label class="row">
          <input
            type="checkbox"
            checked={ov.state.keep_visible}
            disabled={busy}
            onchange={(e) => toggleKeepVisible(e.currentTarget.checked)}
          />
          {t("game.keepVisible")}
        </label>
      </div>
      {#if ov.state.keep_visible && form.fullscreen === 2}
        <div class="hint">{t("game.exclusiveHint")}</div>
      {/if}
    {/if}
    {#if scale > 1 && !fixInstalled}
      <div class="scaling">
        <label class="row">
          <input type="checkbox" bind:checked={adapt} />
          {t("game.adapt", { percent: Math.round(scale * 100) })}
        </label>
        <p class="muted small">
          {#if adapt}
            {t("game.adaptOn", { w: written[0], h: written[1], tw: target[0], th: target[1] })}
          {:else}
            {t("game.adaptOff", {
              w: target[0],
              h: target[1],
              sw: Math.round(target[0] * scale),
              sh: Math.round(target[1] * scale),
            })}
          {/if}
        </p>
      </div>
    {/if}
  </section>

  <section class="card">
    <h2>{t("game.audioTitle")}</h2>
    <p class="muted">{t("game.audioHint")}</p>
    <div class="field">
      <label for="excl">{t("game.exclusiveMode")}</label>
      <input id="excl" type="checkbox" bind:checked={form.exclusive_mode} />
    </div>
    <div class="field">
      <label for="ull">{t("game.ullMode")}</label>
      <input id="ull" type="checkbox" bind:checked={form.ultra_low_latency} />
    </div>
    <div class="field">
      <label for="lb">{t("game.latencyBuffer")}</label>
      <select id="lb" bind:value={form.latency_buffer}>
        <option value={1}>{t("game.bufferLowest")}</option>
        <option value={2}>2</option>
        <option value={3}>3</option>
        <option value={4}>{t("game.bufferStablest")}</option>
      </select>
    </div>
  </section>

  <div class="row">
    <button class="primary" disabled={busy} onclick={save}>{t("common.save")}</button>
    <span class="muted">{t("game.saveHint")}</span>
  </div>
{/if}

<style>
  .attention {
    border-color: var(--warn);
  }
  .scaling {
    border-top: 1px solid var(--border);
    margin-top: 12px;
    padding-top: 10px;
  }
  .small {
    font-size: 0.85rem;
  }
  .hint {
    padding: 8px 12px;
    border-radius: 6px;
    border: 1px solid var(--warn);
    margin: 8px 0;
  }
</style>
