<script lang="ts">
  import { onMount } from "svelte";
  import {
    api,
    type AsioDriver,
    type AudioConfig,
    type DeviceProfile,
    type OutputDevice,
    type Overview,
    type Runner,
  } from "$lib/api";

  let { ov, run, busy, gotoComponents }: { ov: Overview; run: Runner; busy: boolean; gotoComponents: () => void } =
    $props();

  let drivers = $state<AsioDriver[]>([]);
  let windowsOutput = $state<OutputDevice | null>(null);
  let profiles = $state<DeviceProfile[]>([]);
  let loaded = $state(false);
  let form = $state<AudioConfig>(emptyConfig());
  let profileName = $state("");

  let rsAsioInstalled = $derived(!!ov.state.installed["rs_asio"]);
  let selectedDriver = $derived(drivers.find((d) => d.name.toLowerCase() === form.driver.toLowerCase()));
  let suitable = $derived(profiles.filter((p) => drivers.some((d) => d.name.toLowerCase() === p.driver.toLowerCase())));
  let selectedProfile = $derived(profiles.find((p) => p.id === form.profile_id));

  function emptyConfig(): AudioConfig {
    return {
      profile_id: null,
      driver: "",
      output_mode: "asio",
      output_only: false,
      output_base_channel: 0,
      alt_base_channel: null,
      input_channel: 0,
      second_input_channel: null,
      buffer_size_mode: "driver",
      custom_buffer_size: null,
      enable_ref_count_hack: null,
    };
  }

  function applyProfile(id: string) {
    const p = profiles.find((x) => x.id === id);
    if (!p) {
      form.profile_id = null;
      return;
    }
    form = {
      profile_id: p.id,
      driver: p.driver,
      output_mode: form.output_mode,
      output_only: p.output_only,
      output_base_channel: p.output_base_channel,
      alt_base_channel: p.alt_base_channel,
      input_channel: p.input_channel,
      second_input_channel: p.second_input_channel,
      buffer_size_mode: p.buffer_size_mode,
      custom_buffer_size: p.custom_buffer_size,
      enable_ref_count_hack: p.enable_ref_count_hack,
    };
  }

  onMount(async () => {
    [drivers, profiles] = await Promise.all([api.listAsioDrivers(), api.listProfiles()]);
    windowsOutput = await api.defaultOutputDevice().catch(() => null);
    if (ov.state.audio) form = { ...ov.state.audio };
    else if (suitable.length) applyProfile(suitable[0].id);
    else if (drivers.length) form.driver = drivers[0].name;
    loaded = true;
  });

  function save() {
    run("Audio wird eingerichtet …", () => api.applyAudioConfig($state.snapshot(form)), "Audio-Einstellungen gespeichert.");
  }

  async function saveAsProfile() {
    const name = profileName.trim();
    if (!name) return;
    const id = "user-" + name.toLowerCase().replace(/[^a-z0-9]+/g, "-");
    const profile: DeviceProfile = {
      ...$state.snapshot(form),
      id,
      name,
      notes: null,
      source: null,
      custom: true,
    };
    const ok = await run("Profil wird gespeichert …", () => api.saveProfile(profile), `Profil „${name}“ gespeichert.`);
    if (ok) {
      profiles = await api.listProfiles();
      form.profile_id = id;
      profileName = "";
    }
  }
</script>

{#if !rsAsioInstalled}
  <section class="card">
    <h2>RS_ASIO ist noch nicht installiert</h2>
    <p>Damit das Spiel dein Audio-Interface nutzen kann, installiere zuerst RS_ASIO.</p>
    <button class="primary" onclick={gotoComponents}>Zu den Komponenten</button>
  </section>
{:else if !loaded}
  <p class="muted">Suche Audio-Treiber …</p>
{:else}
  {#if drivers.length === 0}
    <section class="card">
      <h2>Kein ASIO-Treiber gefunden</h2>
      {#if ov.platform === "linux"}
        <p>Unter Linux läuft ASIO über wineasio im Proton-Prefix. Diese Einrichtung wird später unterstützt.</p>
      {:else}
        <p>
          Installiere den Treiber deines Audio-Interfaces (z. B. „Focusrite Control“ für Scarlett-Geräte), schließ das
          Gerät an und öffne diesen Tab erneut. Ohne Interface-Treiber hilft ASIO4ALL.
        </p>
      {/if}
    </section>
  {/if}

  <section class="card">
    <h2>Gerät</h2>
    <div class="field">
      <label for="profile">Geräteprofil</label>
      <select id="profile" value={form.profile_id ?? ""} onchange={(e) => applyProfile(e.currentTarget.value)}>
        <option value="">Eigene Einstellungen</option>
        {#if suitable.length}
          <optgroup label="Passend zu deinen Treibern">
            {#each suitable as p}<option value={p.id}>{p.name}</option>{/each}
          </optgroup>
        {/if}
        <optgroup label="Alle Profile">
          {#each profiles as p}<option value={p.id}>{p.name}{p.custom ? " (eigenes)" : ""}</option>{/each}
        </optgroup>
      </select>
    </div>
    {#if selectedProfile?.notes}
      <p class="muted">{selectedProfile.notes}</p>
    {/if}
    <div class="field">
      <label for="driver">ASIO-Treiber</label>
      <select id="driver" bind:value={form.driver}>
        {#if form.driver && !selectedDriver}
          <option value={form.driver}>{form.driver} (nicht installiert)</option>
        {/if}
        {#each drivers as d}
          <option value={d.name}>{d.name}{d.has_32bit ? "" : " (nur 64-Bit)"}</option>
        {/each}
      </select>
    </div>
    {#if selectedDriver && !selectedDriver.has_32bit}
      <div class="hint warn">
        Dieser Treiber ist nur als 64-Bit-Version installiert. Rocksmith ist ein 32-Bit-Spiel und braucht den
        32-Bit-Treiber.
      </div>
    {/if}
    <div class="field">
      <label for="inch">Eingangskanal Gitarre/Bass</label>
      <input id="inch" type="number" min="0" max="64" bind:value={form.input_channel} disabled={form.output_only} />
    </div>
    <div class="field">
      <label for="outch">Erster Ausgangskanal</label>
      <input id="outch" type="number" min="0" max="64" bind:value={form.output_base_channel} />
    </div>
    <div class="field">
      <label for="buf">Puffergröße</label>
      <div class="row">
        <select id="buf" bind:value={form.buffer_size_mode}>
          <option value="driver">vom Treiber</option>
          <option value="host">vom Spiel</option>
          <option value="custom">fest</option>
        </select>
        {#if form.buffer_size_mode === "custom"}
          <input type="number" min="16" max="2048" step="16" bind:value={form.custom_buffer_size} />
          <span class="muted">Samples; bei Knacken erhöhen (96, 192 …)</span>
        {/if}
      </div>
    </div>
  </section>

  <section class="card">
    <h2>Tonausgabe</h2>
    <label class="option">
      <input type="radio" bind:group={form.output_mode} value="asio" />
      <div>
        <strong>Über das Interface</strong>
        <div class="muted">
          Kopfhörer/Boxen am Interface. Niedrigste Latenz – empfohlen zum Spielen. Discord kann den Spielton dann
          nicht übertragen; Sprache aus Discord hörst du, wenn Discord ebenfalls das Interface als Ausgabe nutzt.
        </div>
      </div>
    </label>
    <label class="option">
      <input type="radio" bind:group={form.output_mode} value="windows" />
      <div>
        <strong>Über Windows (z. B. Bluetooth)</strong>
        <div class="muted">
          Spielton über das Windows-Standardgerät, Gitarre weiter über das Interface. Läuft parallel zu Discord und
          lässt sich streamen. Bluetooth verzögert den Ton um etwa 0,2–0,5 s – auch deine Gitarre hörst du so spät.
        </div>
      </div>
    </label>
    <label class="option">
      <input type="radio" bind:group={form.output_mode} value="ask" />
      <div>
        <strong>Bei jedem Spielstart fragen</strong>
        <div class="muted">RS_ASIO zeigt beim Start eine Auswahl zwischen Interface und Windows.</div>
      </div>
    </label>
    {#if form.output_mode !== "asio" && windowsOutput}
      <div class="device" class:warn={windowsOutput.bluetooth}>
        Windows-Ausgabe: <strong>{windowsOutput.name}</strong>{windowsOutput.device ? ` (${windowsOutput.device})` : ""}
        <button class="link" onclick={async () => (windowsOutput = await api.defaultOutputDevice())}>neu prüfen</button>
        {#if windowsOutput.bluetooth}
          <p>
            Das ist ein Bluetooth-Gerät. Spielton und deine Gitarre kommen damit 0,2–0,5 s verzögert an – zum Spielen
            merklich. Besser: Kopfhörer per Kabel an das Interface, oder Lautsprecher/Monitor als Windows-Ausgabe.
          </p>
        {/if}
      </div>
    {/if}
  </section>

  {#if ov.settings?.exclusive_mode && ov.state.audio && ov.state.audio.output_mode !== "asio"}
    <div class="hint">
      Rocksmith beansprucht das Windows-Ausgabegerät gerade exklusiv. Dann hörst du in Discord niemanden, und Discord
      kann den Spielton nicht übertragen. Klick auf „Übernehmen“, um das zu beheben.
    </div>
  {/if}

  <div class="row">
    <button class="primary" disabled={busy || !form.driver} onclick={save}>Übernehmen</button>
    <span class="muted">
      Schreibt RS_ASIO.ini und passt Rocksmith.ini an (exklusiver Modus nur bei Ausgabe über das Interface). Beides wird
      vorher gesichert.
    </span>
  </div>

  <section class="card" style="margin-top: 16px">
    <h2>Als eigenes Profil speichern</h2>
    <p class="muted">Für Geräte ohne fertiges Profil: Einstellungen oben testen und hier für später speichern.</p>
    <div class="row">
      <input placeholder="Name, z. B. Mein Interface" bind:value={profileName} />
      <button disabled={busy || !profileName.trim() || !form.driver} onclick={saveAsProfile}>Speichern</button>
    </div>
  </section>
{/if}

<style>
  .option {
    display: flex;
    gap: 12px;
    align-items: flex-start;
    padding: 10px 0;
    cursor: pointer;
  }
  .option input {
    margin-top: 4px;
  }
  .device {
    margin-top: 6px;
    padding: 8px 12px;
    border-radius: 6px;
    border: 1px solid var(--border);
    background: var(--panel-2);
  }
  .device.warn {
    border-color: var(--warn);
  }
  .device p {
    margin: 6px 0 0;
  }
  .hint {
    padding: 8px 12px;
    border-radius: 6px;
    border: 1px solid var(--warn);
    margin: 8px 0;
  }
</style>
