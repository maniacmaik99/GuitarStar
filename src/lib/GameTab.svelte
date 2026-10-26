<script lang="ts">
  import { onMount } from "svelte";
  import { api, type GameSettings, type MonitorSize, type Overview, type Runner } from "$lib/api";

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
      "Wird gespeichert …",
      () => api.setKeepVisible(enabled),
      enabled ? "Rocksmith minimiert sich ab dem nächsten Start nicht mehr." : "Rocksmith verhält sich wieder wie vorher.",
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
    run("Einstellung wird entfernt …", () => api.setDpiOverride(false), "Die wirkungslose DPI-Einstellung wurde entfernt.");
  }

  function save() {
    if (!form) return;
    const settings = { ...form, screen_width: written[0], screen_height: written[1] };
    run("Einstellungen werden gespeichert …", () => api.saveGameSettings(settings), "Einstellungen gespeichert.");
  }

  /** Installs the fix and, if the ini still holds a scaled-down resolution, writes the real one. */
  function installFix() {
    if (!dpiFix?.bundled_version) return;
    const version = dpiFix.bundled_version;
    const settings = form ? { ...form, screen_width: target[0], screen_height: target[1] } : null;
    run(
      "Skalierungs-Fix wird installiert …",
      async () => {
        await api.installComponent("dpi_fix", version);
        if (settings && (settings.screen_width !== form?.screen_width || settings.screen_height !== form?.screen_height)) {
          await api.saveGameSettings(settings);
        }
      },
      "Skalierungs-Fix installiert. Starte Rocksmith neu.",
    );
  }
</script>

{#if ov.dpi_override}
  <section class="card attention">
    <h2>DPI-Einstellung entfernen</h2>
    <p>
      Die Windows-Einstellung „Skalierung durch: Anwendung“ ist für Rocksmith gesetzt, wirkt bei der Steam-Version aber
      nicht. Sie lässt sich gefahrlos entfernen; der Skalierungs-Fix unten ersetzt sie.
    </p>
    <button disabled={busy} onclick={disableDpiFlag}>Entfernen</button>
  </section>
{/if}

{#if anyScaled && dpiFix}
  <section class="card" class:attention={!fixInstalled}>
    <h2>Windows-Skalierung</h2>
    {#if fixInstalled}
      <p><span class="badge ok">Skalierungs-Fix aktiv</span></p>
      <p class="muted">
        Rocksmith bekommt die echte Auflösung deines Monitors – im Fenster, im Vollbild und im exklusiven Vollbild.
        Entfernen lässt sich der Fix unter „Komponenten“.
      </p>
      {#if fixOutdated}
        <button class="primary" disabled={busy} onclick={installFix}>Fix auf {dpiFix.bundled_version} aktualisieren</button>
      {/if}
    {:else}
      <p>
        Dein Monitor ist skaliert. Rocksmith kommt damit nicht klar: Windows vergrößert das Spiel, und es ragt über den
        Bildschirm. Der Skalierungs-Fix behebt das in allen Anzeigemodi.
      </p>
      {#if dpiFix.conflicts.length}
        <p class="muted small">
          Achtung: Im Spielordner liegt schon eine andere {dpiFix.conflicts.join(", ")}. Sie wird gesichert und ersetzt.
        </p>
      {/if}
      <button class="primary" disabled={busy} onclick={installFix}>Skalierungs-Fix installieren</button>
    {/if}
  </section>
{/if}

{#if !form}
  <section class="card">
    <h2>Rocksmith.ini fehlt noch</h2>
    <p>
      Das Spiel legt seine Einstellungsdatei erst beim ersten Start an. Starte Rocksmith einmal kurz über Steam, beende
      es wieder und lade diese Seite dann neu.
    </p>
    <button disabled={busy} onclick={() => run("Prüfe …", async () => {})}>Neu laden</button>
  </section>
{:else}
  <section class="card">
    <h2>Bild</h2>
    <div class="field">
      <label for="res">Auflösung</label>
      <select id="res" bind:value={getResolution, setResolution}>
        {#each resolutions as [w, h]}
          <option value={`${w}x${h}`}>{w} × {h}{nativeOf(w, h) ? " (Monitor)" : ""}</option>
        {/each}
      </select>
    </div>
    <div class="field">
      <label for="fs">Anzeigemodus</label>
      <select id="fs" bind:value={form.fullscreen}>
        <option value={0}>Fenster</option>
        <option value={1}>Vollbild (Fenster)</option>
        <option value={2}>Exklusives Vollbild</option>
      </select>
    </div>
    {#if fixInstalled && !fixOutdated}
      <div class="field">
        <span>Bei Fokusverlust</span>
        <label class="row">
          <input
            type="checkbox"
            checked={ov.state.keep_visible}
            disabled={busy}
            onchange={(e) => toggleKeepVisible(e.currentTarget.checked)}
          />
          nicht minimieren (z. B. beim Klick auf den zweiten Monitor)
        </label>
      </div>
      {#if ov.state.keep_visible && form.fullscreen === 2}
        <div class="hint">
          Im exklusiven Vollbild minimiert DirectX das Spiel beim Fokusverlust immer – das lässt sich nicht verhindern.
          Für zwei Monitore „Vollbild (Fenster)“ wählen; dank Skalierungs-Fix sieht es genauso aus.
        </div>
      {/if}
    {/if}
    {#if scale > 1 && !fixInstalled}
      <div class="scaling">
        <label class="row">
          <input type="checkbox" bind:checked={adapt} />
          Ohne Fix: Auflösung an Windows-Skalierung ({Math.round(scale * 100)} %) anpassen (nur im Fenstermodus)
        </label>
        <p class="muted small">
          {#if adapt}
            Rocksmith bekommt {written[0]} × {written[1]}; Windows vergrößert das Fenster dann genau auf {target[0]} ×
            {target[1]}. Das Bild ist dadurch minimal weicher.
          {:else}
            Ohne Anpassung vergrößert Windows ein {target[0]} × {target[1]}-Fenster auf
            {Math.round(target[0] * scale)} × {Math.round(target[1] * scale)}, und es ragt über den Bildschirm.
          {/if}
        </p>
      </div>
    {/if}
  </section>

  <section class="card">
    <h2>Audio im Spiel</h2>
    <p class="muted">Für RS_ASIO empfohlen: exklusiver Modus und Ultra-Low-Latency an, Puffer so klein wie ohne Knacken möglich.</p>
    <div class="field">
      <label for="excl">Exklusiver Modus</label>
      <input id="excl" type="checkbox" bind:checked={form.exclusive_mode} />
    </div>
    <div class="field">
      <label for="ull">Ultra-Low-Latency-Modus</label>
      <input id="ull" type="checkbox" bind:checked={form.ultra_low_latency} />
    </div>
    <div class="field">
      <label for="lb">Latenzpuffer</label>
      <select id="lb" bind:value={form.latency_buffer}>
        <option value={1}>1 (niedrigste Latenz)</option>
        <option value={2}>2</option>
        <option value={3}>3</option>
        <option value={4}>4 (stabilste)</option>
      </select>
    </div>
  </section>

  <div class="row">
    <button class="primary" disabled={busy} onclick={save}>Speichern</button>
    <span class="muted">Vor dem Speichern wird die bisherige Rocksmith.ini gesichert.</span>
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
