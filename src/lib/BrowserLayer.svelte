<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { api, errorText, type DownloadReport } from "$lib/api";

  let { onclose }: { onclose: () => void } = $props();

  let area: HTMLDivElement;
  let status = $state<{ text: string; kind: "busy" | "ok" | "bad" } | null>(null);
  let error = $state<string | null>(null);

  // The CustomsForge page is a native webview placed over this element.
  function bounds() {
    const r = area.getBoundingClientRect();
    return { x: r.left, y: r.top, width: r.width, height: r.height };
  }

  function close() {
    api.hideCustomsforge();
    onclose();
  }

  onMount(() => {
    api.openCustomsforge(bounds()).catch((e) => (error = errorText(e)));
    const sync = () => api.customsforgeBounds(bounds());
    const observer = new ResizeObserver(sync);
    observer.observe(area);
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && close();
    window.addEventListener("keydown", onKey);
    const unlisteners = [
      listen<string>("cdlc-download-started", (e) => (status = { text: `Lädt ${e.payload} …`, kind: "busy" })),
      listen<DownloadReport>("cdlc-download", (e) => {
        const r = e.payload;
        if (r.error) status = { text: `${r.file_name}: ${r.error}`, kind: "bad" };
        else if (r.result?.imported.length) status = { text: `Installiert: ${r.result.imported.join(", ")}`, kind: "ok" };
        else status = { text: `${r.file_name}: ${r.result?.skipped[0]?.[1] ?? "kein Song gefunden"}`, kind: "bad" };
      }),
    ];
    return () => {
      observer.disconnect();
      window.removeEventListener("keydown", onKey);
      unlisteners.forEach((p) => p.then((off) => off()));
      api.hideCustomsforge();
    };
  });
</script>

<div class="layer">
  <div class="bar">
    <button title="Zurück" onclick={() => api.customsforgeNavigate("back")}>←</button>
    <button title="Vor" onclick={() => api.customsforgeNavigate("forward")}>→</button>
    <button title="Neu laden" onclick={() => api.customsforgeNavigate("reload")}>⟳</button>
    <button title="Startseite" onclick={() => api.customsforgeNavigate("home")}>CustomsForge</button>
    <div class="status" class:ok={status?.kind === "ok"} class:bad={status?.kind === "bad"}>
      {#if error}
        <span class="bad">{error}</span>
      {:else if status}
        {#if status.kind === "busy"}<span class="spinner"></span>{/if}{status.text}
      {:else}
        <span class="muted">
          Heruntergeladene Songs werden automatisch installiert. Werbung auf dieser Seite schaltet CustomsForge
          (ausblendbar unter Einstellungen).
        </span>
      {/if}
    </div>
    <button class="primary" onclick={close}>Fertig</button>
  </div>
  <div class="area" bind:this={area}></div>
</div>

<style>
  .layer {
    position: fixed;
    inset: 0;
    z-index: 100;
    display: flex;
    flex-direction: column;
    background: var(--bg);
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
    background: var(--panel);
  }
  .bar button {
    padding: 5px 10px;
  }
  .status {
    flex: 1;
    min-width: 0;
    margin: 0 10px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .status.ok {
    color: var(--ok);
  }
  .status.bad,
  .bad {
    color: var(--warn);
  }
  .area {
    flex: 1;
  }
  .spinner {
    display: inline-block;
    width: 11px;
    height: 11px;
    margin-right: 8px;
    border: 2px solid var(--muted);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
