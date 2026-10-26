<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { api, type Overview, type Runner } from "$lib/api";

  let { ov, run, busy }: { ov: Overview; run: Runner; busy: boolean } = $props();

  let fixInstalled = $derived(!!ov.state.installed["dpi_fix"]);

  function setHideAds(enabled: boolean) {
    run(
      "Wird gespeichert …",
      () => api.setHideAds(enabled),
      enabled ? "Werbung wird beim nächsten Öffnen von CustomsForge ausgeblendet." : "Werbung wird wieder angezeigt.",
    );
  }
</script>

<section class="card">
  <h2>CustomsForge</h2>
  <div class="info">
    <strong>Die Werbung kommt von CustomsForge, nicht von GuitarStar.</strong>
    <p>
      GuitarStar zeigt die Seite von CustomsForge nur an und verdient daran nichts. Die Werbung schaltet CustomsForge
      selbst – zusammen mit Spenden bezahlt sie den Betrieb der Seite, über die die kostenlosen Songs (CDLC) der
      Community verteilt werden.
    </p>
    <p>
      Du kannst die Werbung hier ausblenden. Wenn du das tust, überleg dir, CustomsForge auf anderem Weg zu
      unterstützen.
    </p>
    <div class="row">
      <button onclick={() => openUrl("https://ignition4.customsforge.com/support")}>CustomsForge unterstützen</button>
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
    Werbung auf CustomsForge ausblenden
  </label>
  <p class="muted small">
    Gilt ab dem nächsten Öffnen von CustomsForge. Du bleibst dabei eingeloggt. Falls nach einer Änderung der Seite
    doch Werbung durchrutscht oder etwas fehlt, gib bitte auf GitHub Bescheid.
  </p>
</section>

<section class="card">
  <h2>Fehlersuche</h2>
  <label class="row toggle">
    <input
      type="checkbox"
      checked={ov.state.dll_log}
      disabled={busy || !fixInstalled}
      onchange={(e) =>
        run("Wird gespeichert …", () => api.setDllLog(e.currentTarget.checked), "Gilt ab dem nächsten Spielstart.")}
    />
    Diagnose-Protokolle des Skalierungs-Fix schreiben
  </label>
  <p class="muted small">
    Schreibt beim Spielen Fenster- und Video-Ereignisse in GuitarStar-window.log im Spielordner. Nur für die
    Fehlersuche nötig – sonst aus lassen.
    {#if !fixInstalled}Erst verfügbar, wenn der Skalierungs-Fix installiert ist.{/if}
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
